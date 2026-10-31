// VietFlow Windows TSF - In-Process Text Input Processor (TIP)
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::edit_session::{CompositionEditSession, EditAction};
use crate::ipc::SharedConfigReader;
use crate::DLL_REF_COUNT;
use std::cell::{Cell, RefCell};
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant, SystemTime};
use vietflow_core::macro_table::CLIPBOARD_PLACEHOLDER;
use vietflow_core::models::{CharsetId, CustomKeymap, EngineOptions, InputMethod};
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
use windows_sys::Win32::Foundation::{CloseHandle, HGLOBAL};
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetForegroundWindow, GetWindowThreadProcessId,
};

const CF_UNICODETEXT: u32 = 13;
const MAX_CLIPBOARD_CHARS: usize = 1_048_576;

#[derive(Clone, Copy)]
struct ActiveProfile {
    method: Option<u8>,
    charset: Option<u8>,
    allow_macro: Option<bool>,
}

fn read_clipboard_text() -> String {
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return String::new();
        }
        let result = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                return String::new();
            }
            let ptr = GlobalLock(handle as HGLOBAL) as *const u16;
            if ptr.is_null() {
                return String::new();
            }
            let mut len = 0usize;
            while len < MAX_CLIPBOARD_CHARS && *ptr.add(len) != 0 {
                len += 1;
            }
            let text = String::from_utf16_lossy(std::slice::from_raw_parts(ptr, len));
            GlobalUnlock(handle as HGLOBAL);
            text
        })();
        CloseClipboard();
        result
    }
}

/// Core leaves a private placeholder for the platform host. TSF runs in the
/// target process, so it can resolve the clipboard at composition time.
fn expand_clipboard(text: String) -> String {
    if !text.contains(CLIPBOARD_PLACEHOLDER) {
        return text;
    }
    text.replace(CLIPBOARD_PLACEHOLDER, &read_clipboard_text())
}

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
    last_config_check: Cell<Instant>,
    last_profile_check: Cell<Instant>,
    config_mtime: Cell<Option<SystemTime>>,
    user_json: RefCell<Option<serde_json::Value>>,
    active_profile: RefCell<Option<ActiveProfile>>,
}

fn user_config_path() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("VietFlow").join("vietflow_config.json"))
}

fn foreground_process_name() -> Option<String> {
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd.is_null() {
            return None;
        }
        let mut pid = 0u32;
        GetWindowThreadProcessId(hwnd, &mut pid);
        if pid == 0 {
            return None;
        }
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let mut buffer = [0u16; 1024];
        let mut len = buffer.len() as u32;
        let ok = QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut len);
        CloseHandle(process);
        if ok == 0 || len == 0 {
            return None;
        }
        let path = String::from_utf16_lossy(&buffer[..len as usize]);
        std::path::Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
    }
}

fn normalized_words(value: Option<&serde_json::Value>) -> Vec<String> {
    let mut words: Vec<String> = value
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect()
        })
        .unwrap_or_default();
    words.sort();
    words.dedup();
    words
}

fn normalized_initials(text: &str) -> Vec<char> {
    let mut initials = Vec::new();
    for c in text.chars().filter(|c| c.is_alphabetic()) {
        let lower = c.to_ascii_lowercase();
        if !initials.contains(&lower) {
            initials.push(lower);
            if initials.len() == 26 {
                break;
            }
        }
    }
    if initials.is_empty() {
        vec!['f', 'j', 'w', 'z']
    } else {
        initials
    }
}

/// Load the fields that are not part of the small shared-memory struct. The
/// file is read at most once per 750 ms and only reparsed when it changes, so
/// normal keystrokes still stay on the in-memory path.
fn apply_user_config(engine: &mut VietFlowEngine, json: &serde_json::Value) {
    if let Some(macros) = json.get("macros").and_then(serde_json::Value::as_object) {
        engine.macros.clear();
        for (trigger, value) in macros {
            if let Some(value) = value.as_str() {
                engine.macros.insert(trigger, value);
            }
        }
    }

    if let Some(ck) = json.get("custom_keymap").and_then(serde_json::Value::as_object) {
        let mut custom = CustomKeymap::default();
        let read = |key: &str| ck.get(key).and_then(serde_json::Value::as_str);
        if let Some(v) = read("acute") { custom.acute = v.to_string(); }
        if let Some(v) = read("grave") { custom.grave = v.to_string(); }
        if let Some(v) = read("hook") { custom.hook = v.to_string(); }
        if let Some(v) = read("tilde") { custom.tilde = v.to_string(); }
        if let Some(v) = read("dot") { custom.dot = v.to_string(); }
        if let Some(v) = read("remove_tone") { custom.remove_tone = v.to_string(); }
        if let Some(v) = read("circumflex") { custom.circumflex = v.to_string(); }
        if let Some(v) = read("horn") { custom.horn = v.to_string(); }
        if let Some(v) = read("breve") { custom.breve = v.to_string(); }
        if let Some(v) = read("d_mark") { custom.d_mark = v.to_string(); }
        if let Some(v) = read("hook_all") { custom.hook_all = v.to_string(); }
        if let Some(v) = read("roof_a") { custom.roof_a = v.to_string(); }
        if let Some(v) = read("roof_e") { custom.roof_e = v.to_string(); }
        if let Some(v) = read("roof_o") { custom.roof_o = v.to_string(); }
        if let Some(v) = read("horn_o") { custom.horn_o = v.to_string(); }
        if let Some(v) = read("horn_u") { custom.horn_u = v.to_string(); }
        if let Some(v) = read("breve_a") { custom.breve_a = v.to_string(); }
        if let Some(v) = read("d_lower") { custom.d_lower = v.to_string(); }
        if let Some(v) = ck.get("telex_double_keys").and_then(serde_json::Value::as_bool) {
            custom.telex_double_keys = v;
        }
        if let Some(v) = ck.get("lone_w").and_then(serde_json::Value::as_bool) {
            custom.lone_w = v;
        }
        engine.options.custom_keymap = custom;
    }

    if let Some(v) = json.get("foreign_initials").and_then(serde_json::Value::as_str) {
        engine.options.foreign_initials = normalized_initials(v);
    }
    if json.get("english_keywords").is_some() {
        engine.options.custom_english_keywords = normalized_words(json.get("english_keywords"));
    }
    if json.get("english_keywords_removed").is_some() {
        engine.options.removed_english_keywords =
            normalized_words(json.get("english_keywords_removed"));
    }
    if let Some(v) = json.get("typing_timeout_ms").and_then(serde_json::Value::as_u64) {
        engine.options.typing_timeout_ms = v.clamp(500, 5000);
    }
    if let Some(v) = json.get("auto_capitalize").and_then(serde_json::Value::as_bool) {
        engine.options.auto_capitalize = v;
    }
    if let Some(v) = json.get("auto_capitalize_chars").and_then(serde_json::Value::as_str) {
        engine.options.auto_capitalize_chars = v.to_string();
    }
    if let Some(v) = json.get("fix_shift_lag").and_then(serde_json::Value::as_bool) {
        engine.options.fix_shift_lag = v;
    }
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

        let tip = Self {
            client_id: Cell::new(0),
            thread_mgr: RefCell::new(None),
            engine: RefCell::new(engine),
            config_reader: SharedConfigReader::new(),
            active_composition: RefCell::new(None),
            last_config_check: Cell::new(Instant::now() - Duration::from_secs(2)),
            last_profile_check: Cell::new(Instant::now() - Duration::from_secs(2)),
            config_mtime: Cell::new(None),
            user_json: RefCell::new(None),
            active_profile: RefCell::new(None),
        };
        tip.load_user_config(&mut tip.engine.borrow_mut());
        tip
    }

    fn load_user_config(&self, engine: &mut VietFlowEngine) {
        let Some(path) = user_config_path() else { return; };
        let mtime = std::fs::metadata(&path).ok().and_then(|m| m.modified().ok());
        if mtime.is_some() && mtime == self.config_mtime.get() {
            return;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) {
                apply_user_config(engine, &json);
                *self.user_json.borrow_mut() = Some(json);
                self.last_profile_check
                    .set(Instant::now() - Duration::from_secs(1));
            }
        }
        self.config_mtime.set(mtime);
    }

    fn refresh_active_profile(&self) {
        let json = self.user_json.borrow();
        let profile = json.as_ref().and_then(|json| {
            let profiles = json.get("app_profiles")?.as_object()?;
            let process = foreground_process_name()?;
            let process_lower = process.to_ascii_lowercase();
            let process_stem = process_lower.strip_suffix(".exe").unwrap_or(&process_lower);
            profiles.iter().find_map(|(name, value)| {
                let name = name.to_ascii_lowercase();
                let name_stem = name.strip_suffix(".exe").unwrap_or(&name);
                if name == process_lower || name_stem == process_stem {
                    Some(value)
                } else {
                    None
                }
            })
        });

        let active = profile.map(|profile| ActiveProfile {
            method: profile
                .get("method")
                .and_then(serde_json::Value::as_u64)
                .map(|m| m.min(6) as u8),
            charset: profile
                .get("charset")
                .and_then(serde_json::Value::as_u64)
                .map(|c| c.min(3) as u8),
            allow_macro: profile
                .get("allow_macro")
                .and_then(serde_json::Value::as_bool),
        });
        *self.active_profile.borrow_mut() = active;
    }

    fn apply_active_profile(&self, engine: &mut VietFlowEngine, global_macro_enabled: bool) {
        let profile = *self.active_profile.borrow();
        let Some(profile) = profile else { return; };

        if let Some(method) = profile.method {
            engine.options.method = match method {
                1 => InputMethod::Vni,
                2 => InputMethod::Viqr,
                3 => InputMethod::SimpleTelex,
                4 => InputMethod::TelexSimple2,
                5 => InputMethod::TelexVni,
                6 => InputMethod::UserCustom,
                _ => InputMethod::Telex,
            };
        }
        if let Some(charset) = profile.charset {
            engine.charset = match charset {
                1 => CharsetId::UnicodeComposite,
                2 => CharsetId::Tcvn3,
                3 => CharsetId::VniWindows,
                _ => CharsetId::Unicode,
            };
        }
        if let Some(allow_macro) = profile.allow_macro {
            engine.options.enable_macro = global_macro_enabled && allow_macro;
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

        if self.last_config_check.get().elapsed() >= Duration::from_millis(750) {
            self.last_config_check.set(Instant::now());
            self.load_user_config(&mut engine);
        }
        if self.last_profile_check.get().elapsed() >= Duration::from_millis(750) {
            self.last_profile_check.set(Instant::now());
            self.refresh_active_profile();
        }
        self.apply_active_profile(&mut engine, cfg.enable_macro != 0);
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
            let rep = expand_clipboard(rep);
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
        // Re-evaluate the active app profile on the next key without waiting
        // for the normal refresh interval.
        self.last_config_check
            .set(Instant::now() - Duration::from_secs(1));
        self.last_profile_check
            .set(Instant::now() - Duration::from_secs(1));
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
        let shift = unsafe { (GetKeyState(0x10) as u16 & 0x8000) != 0 };

        // Test if key is an alphabetic, VNI number, bracket, or VIQR mark
        let is_letter = (0x41..=0x5A).contains(&vk);
        let is_digit = (0x30..=0x39).contains(&vk)
            && !shift
            && (cfg.method == 1 || cfg.method == 5 || cfg.method == 6);
        let is_bracket = (vk == 0xDB || vk == 0xDD)
            && (cfg.method == 0 || cfg.method == 3 || cfg.method == 4 || cfg.method == 6);
        let is_viqr_mark = (cfg.method == 2 || cfg.method == 6)
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
                let word = expand_clipboard(self.engine.borrow().current_preedit_text());

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

        let is_viqr_mark = (cfg.method == 2 || cfg.method == 6)
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

        if is_digit && cfg.method != 1 && cfg.method != 5 && cfg.method != 6 {
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
        let word = expand_clipboard(self.engine.borrow().current_preedit_text());

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_user_macros_and_custom_keymap() {
        let mut engine = VietFlowEngine::new(EngineOptions::default(), CharsetId::Unicode);
        let json = serde_json::json!({
            "macros": { "vn": "Việt Nam" },
            "custom_keymap": { "acute": "1" },
            "typing_timeout_ms": 800
        });
        apply_user_config(&mut engine, &json);
        assert_eq!(engine.macros.get("vn").as_deref(), Some("Việt Nam"));
        assert_eq!(engine.options.custom_keymap.acute, "1");
        assert_eq!(engine.options.typing_timeout_ms, 800);
    }
}
