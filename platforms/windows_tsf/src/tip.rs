// VietFlow Windows TSF - In-Process Text Input Processor (TIP)
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::edit_session::{CompositionEditSession, EditAction};
use crate::ipc::SharedConfigReader;
use crate::DLL_REF_COUNT;
use std::cell::{Cell, RefCell};
use std::sync::atomic::Ordering;
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows::core::{implement, Interface, IUnknownImpl, Ref, Result, BOOL, GUID};
use windows::Win32::Foundation::{LPARAM, WPARAM};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_BACK, VK_CONTROL, VK_ESCAPE, VK_MENU, VK_RETURN, VK_SPACE, VK_TAB,
};
use windows::Win32::UI::TextServices::{
    ITfComposition, ITfCompositionSink, ITfCompositionSink_Impl, ITfContext,
    ITfEditSession, ITfKeyEventSink, ITfKeyEventSink_Impl,
    ITfKeystrokeMgr, ITfTextInputProcessor, ITfTextInputProcessorEx,
    ITfTextInputProcessorEx_Impl, ITfTextInputProcessor_Impl, ITfThreadMgr,
    TF_ES_READWRITE, TF_ES_SYNC,
};

#[implement(
    ITfTextInputProcessor,
    ITfTextInputProcessorEx,
    ITfKeyEventSink,
    ITfCompositionSink
)]
pub struct VietFlowTip {
    client_id: Cell<u32>,
    thread_mgr: RefCell<Option<ITfThreadMgr>>,
    engine: RefCell<VietFlowEngine>,
    config_reader: SharedConfigReader,
    active_composition: RefCell<Option<ITfComposition>>,
}

impl VietFlowTip {
    pub fn new() -> Self {
        DLL_REF_COUNT.fetch_add(1, Ordering::SeqCst);
        let engine = VietFlowEngine::new(
            EngineOptions {
                method: InputMethod::Telex,
                modern_style: true,
                free_marking: true,
                spell_check: true,
                auto_non_vn_restore: true,
                enable_macro: true,
                ..Default::default()
            },
            CharsetId::Unicode,
        );

        Self {
            client_id: Cell::new(0),
            thread_mgr: RefCell::new(None),
            engine: RefCell::new(engine),
            config_reader: SharedConfigReader::new(),
            active_composition: RefCell::new(None),
        }
    }

    fn sync_config(&self) {
        let cfg = self.config_reader.get();
        let mut engine = self.engine.borrow_mut();

        engine.options.method = match cfg.method {
            1 => InputMethod::Vni,
            2 => InputMethod::Viqr,
            3 => InputMethod::SimpleTelex,
            4 => InputMethod::TelexSimple2,
            5 => InputMethod::TelexVni,
            _ => InputMethod::Telex,
        };

        engine.charset = match cfg.charset {
            1 => CharsetId::UnicodeComposite,
            2 => CharsetId::Tcvn3,
            3 => CharsetId::VniWindows,
            _ => CharsetId::Unicode,
        };

        engine.options.modern_style = cfg.modern_style != 0;
        engine.options.spell_check = cfg.spell_check != 0;
        engine.options.free_marking = cfg.free_marking != 0;
        engine.options.auto_non_vn_restore = cfg.auto_restore != 0;
        engine.options.consonant_shortcuts = cfg.consonant_shortcuts != 0;
        engine.options.enable_macro = cfg.enable_macro != 0;
    }

    pub fn is_modifier_pressed() -> bool {
        unsafe {
            let ctrl = (GetKeyState(VK_CONTROL.0 as i32) as u16 & 0x8000) != 0;
            let alt = (GetKeyState(VK_MENU.0 as i32) as u16 & 0x8000) != 0;
            let win = (GetKeyState(0x5B) as u16 & 0x8000) != 0 || (GetKeyState(0x5C) as u16 & 0x8000) != 0;
            ctrl || alt || win
        }
    }
}

fn request_edit_session_rw(context: &ITfContext, client_id: u32, session: &ITfEditSession) {
    unsafe {
        let hr = context.RequestEditSession(client_id, session, TF_ES_READWRITE | TF_ES_SYNC);
        if hr.is_err() {
            let _ = context.RequestEditSession(client_id, session, TF_ES_READWRITE);
        }
    }
}

impl Drop for VietFlowTip {
    fn drop(&mut self) {
        DLL_REF_COUNT.fetch_sub(1, Ordering::SeqCst);
    }
}

impl VietFlowTip_Impl {
    fn end_active_composition(&self, context: &ITfContext) {
        let session: ITfEditSession = CompositionEditSession::new(
            context.clone(),
            self.to_object().to_interface(),
            self.active_composition.as_ptr(),
            "",
            EditAction::EndComposition,
        )
        .into();

        request_edit_session_rw(context, self.client_id.get(), &session);
        self.engine.borrow_mut().reset();
    }

    fn replace_and_end_active_composition(&self, context: &ITfContext, replacement: &str) {
        let session: ITfEditSession = CompositionEditSession::new(
            context.clone(),
            self.to_object().to_interface(),
            self.active_composition.as_ptr(),
            replacement,
            EditAction::ReplaceAndEndComposition,
        )
        .into();

        request_edit_session_rw(context, self.client_id.get(), &session);
        self.engine.borrow_mut().reset();
    }

    fn finish_composition(&self, context: &ITfContext, break_char: char) {
        let replacement = self.engine.borrow_mut().finish_composition_on_break(break_char);
        if let Some(rep) = replacement {
            self.replace_and_end_active_composition(context, &rep);
        } else {
            self.end_active_composition(context);
        }
    }

    fn clear_and_end_active_composition(&self, context: &ITfContext) {
        let session: ITfEditSession = CompositionEditSession::new(
            context.clone(),
            self.to_object().to_interface(),
            self.active_composition.as_ptr(),
            "",
            EditAction::ClearAndEndComposition,
        )
        .into();

        request_edit_session_rw(context, self.client_id.get(), &session);
        self.engine.borrow_mut().reset();
    }
}

impl ITfTextInputProcessor_Impl for VietFlowTip_Impl {
    fn Activate(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32) -> Result<()> {
        self.client_id.set(tid);

        if let Some(mgr) = ptim.as_ref() {
            self.thread_mgr.replace(Some(mgr.clone()));

            unsafe {
                let keystroke_mgr: ITfKeystrokeMgr = mgr.cast()?;
                let sink: ITfKeyEventSink = self.to_object().to_interface();
                keystroke_mgr.AdviseKeyEventSink(tid, &sink, true)?;
            }
        }

        Ok(())
    }

    fn Deactivate(&self) -> Result<()> {
        if let Some(mgr) = self.thread_mgr.borrow().as_ref() {
            unsafe {
                let keystroke_mgr: Result<ITfKeystrokeMgr> = mgr.cast();
                if let Ok(kmgr) = keystroke_mgr {
                    let _ = kmgr.UnadviseKeyEventSink(self.client_id.get());
                }
            }
        }

        self.active_composition.replace(None);
        self.thread_mgr.replace(None);
        self.engine.borrow_mut().reset();

        Ok(())
    }
}

impl ITfTextInputProcessorEx_Impl for VietFlowTip_Impl {
    fn ActivateEx(&self, ptim: Ref<'_, ITfThreadMgr>, tid: u32, _dwflags: u32) -> Result<()> {
        self.Activate(ptim, tid)
    }
}

impl ITfKeyEventSink_Impl for VietFlowTip_Impl {
    fn OnSetFocus(&self, _fforeground: BOOL) -> Result<()> {
        self.engine.borrow_mut().reset();
        self.active_composition.replace(None);
        Ok(())
    }

    fn OnTestKeyDown(
        &self,
        _pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        let cfg = self.config_reader.get();
        if cfg.enabled == 0 {
            return Ok(BOOL(0));
        }

        // Modifiers (Ctrl, Alt, Win) always take precedence so system and app shortcuts pass immediately
        if VietFlowTip::is_modifier_pressed() {
            return Ok(BOOL(0));
        }

        // When a composition is active, intercept all keys to either update or finalize
        if self.active_composition.borrow().is_some() {
            return Ok(BOOL(1));
        }

        let vk = wparam.0 as u16;

        // Test if key is an alphabetic, VNI number, bracket, or VIQR mark
        let is_letter = (0x41..=0x5A).contains(&vk);
        let is_digit = (0x30..=0x39).contains(&vk) && (cfg.method == 1 || cfg.method == 5);
        let is_bracket = (vk == 0xDB || vk == 0xDD) && (cfg.method == 0 || cfg.method == 3 || cfg.method == 4);
        let is_viqr_mark = cfg.method == 2
            && (vk == 0xDE || vk == 0xC0 || vk == 0xBF || vk == 0xBE || vk == 0xBB || vk == 0x36 || vk == 0x39);

        if is_letter || is_digit || is_bracket || is_viqr_mark {
            Ok(BOOL(1))
        } else {
            Ok(BOOL(0))
        }
    }

    fn OnKeyDown(
        &self,
        pic: Ref<'_, ITfContext>,
        wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        let cfg = self.config_reader.get();
        if cfg.enabled == 0 {
            if self.active_composition.borrow().is_some() {
                if let Some(context) = pic.as_ref() {
                    self.end_active_composition(context);
                }
            }
            return Ok(BOOL(0));
        }

        let context = match pic.as_ref() {
            Some(c) => c,
            None => return Ok(BOOL(0)),
        };

        if VietFlowTip::is_modifier_pressed() {
            if self.active_composition.borrow().is_some() {
                self.end_active_composition(context);
            }
            return Ok(BOOL(0));
        }

        self.sync_config();

        let vk = wparam.0 as u16;

        // 1. Backspace handling in active composition
        if vk == VK_BACK.0 {
            if self.active_composition.borrow().is_some() {
                let (_, _) = self.engine.borrow_mut().process_key('\x08');
                let word = self.engine.borrow().current_preedit_text();

                if word.is_empty() {
                    self.clear_and_end_active_composition(context);
                } else {
                    let session: ITfEditSession = CompositionEditSession::new(
                        context.clone(),
                        self.to_object().to_interface(),
                        self.active_composition.as_ptr(),
                        &word,
                        EditAction::StartOrUpdate,
                    )
                    .into();
                    request_edit_session_rw(context, self.client_id.get(), &session);
                }
                return Ok(BOOL(1));
            }
            return Ok(BOOL(0));
        }

        // 2. Word break keys commit composition and pass through to app
        if vk == VK_SPACE.0 || vk == VK_RETURN.0 || vk == VK_TAB.0 {
            if self.active_composition.borrow().is_some() {
                let break_char = if vk == VK_SPACE.0 {
                    ' '
                } else if vk == VK_RETURN.0 {
                    '\n'
                } else {
                    '\t'
                };
                self.finish_composition(context, break_char);
            }
            return Ok(BOOL(0)); // Allow Space/Enter/Tab to pass to app
        }

        // 3. Escape commits/cancels composition and is eaten
        if vk == VK_ESCAPE.0 {
            if self.active_composition.borrow().is_some() {
                self.end_active_composition(context);
                return Ok(BOOL(1));
            }
            return Ok(BOOL(0));
        }

        // 4. Navigation keys (arrows, home, end, pgup, pgdn, delete) commit composition
        if (0x21..=0x28).contains(&vk) || vk == 0x2E {
            if self.active_composition.borrow().is_some() {
                self.finish_composition(context, ' ');
            }
            return Ok(BOOL(0));
        }

        // 5. Printable letters, VNI numbers, brackets, and VIQR marks
        let is_letter = (0x41..=0x5A).contains(&vk);
        let is_digit = (0x30..=0x39).contains(&vk);
        let is_bracket = vk == 0xDB || vk == 0xDD;
        let shift = unsafe { (GetKeyState(0x10) as u16 & 0x8000) != 0 };
        let caps = unsafe { (GetKeyState(0x14) & 1) != 0 };
        let upper = shift ^ caps;

        let is_viqr_mark = cfg.method == 2
            && (vk == 0xDE || vk == 0xC0 || vk == 0xBF || vk == 0xBE || vk == 0xBB
                || (vk == 0x36 && shift) || (vk == 0x39 && shift));

        if !is_letter && !is_digit && !is_bracket && !is_viqr_mark {
            if self.active_composition.borrow().is_some() {
                self.finish_composition(context, ' ');
            }
            return Ok(BOOL(0));
        }

        if is_digit && shift {
            // Shift + number is a symbol (!@#$%), not a VNI tone mark
            if self.active_composition.borrow().is_some() {
                self.finish_composition(context, ' ');
            }
            return Ok(BOOL(0));
        }

        if is_digit && cfg.method != 1 && cfg.method != 5 {
            // Numbers in Telex/VIQR are not tone marks
            if self.active_composition.borrow().is_some() {
                self.finish_composition(context, ' ');
            }
            return Ok(BOOL(0));
        }

        let ch = if is_letter {
            let mut c = (vk as u8) as char;
            if !upper {
                c = c.to_ascii_lowercase();
            }
            c
        } else if is_bracket {
            if vk == 0xDB {
                if shift { '{' } else { '[' }
            } else {
                if shift { '}' } else { ']' }
            }
        } else if is_viqr_mark {
            match vk {
                0xDE => if shift { '"' } else { '\'' },
                0xC0 => if shift { '~' } else { '`' },
                0xBF => if shift { '?' } else { '/' },
                0xBE => '.',
                0xBB => if shift { '+' } else { '=' },
                0x36 => '^',
                0x39 => '(',
                _ => (vk as u8) as char,
            }
        } else {
            (vk as u8) as char
        };

        // Process through VietFlow core engine
        let _ = self.engine.borrow_mut().process_key(ch);
        let word = self.engine.borrow().current_preedit_text();

        if word.is_empty() {
            self.clear_and_end_active_composition(context);
        } else {
            // Start or update TSF composition range with current word
            let session: ITfEditSession = CompositionEditSession::new(
                context.clone(),
                self.to_object().to_interface(),
                self.active_composition.as_ptr(),
                &word,
                EditAction::StartOrUpdate,
            )
            .into();
            request_edit_session_rw(context, self.client_id.get(), &session);
        }

        Ok(BOOL(1))
    }

    fn OnTestKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnKeyUp(
        &self,
        _pic: Ref<'_, ITfContext>,
        _wparam: WPARAM,
        _lparam: LPARAM,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }

    fn OnPreservedKey(
        &self,
        _pic: Ref<'_, ITfContext>,
        _rguid: *const GUID,
    ) -> Result<BOOL> {
        Ok(BOOL(0))
    }
}

impl ITfCompositionSink_Impl for VietFlowTip_Impl {
    fn OnCompositionTerminated(
        &self,
        _ecwrite: u32,
        _pcomposition: Ref<'_, ITfComposition>,
    ) -> Result<()> {
        self.active_composition.replace(None);
        self.engine.borrow_mut().reset();
        Ok(())
    }
}
