// VietFlow Windows - Low-Level Keyboard Hook (WH_KEYBOARD_LL)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::Instant;
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetKeyState, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE, VK_F5, VK_F9, VK_LCONTROL,
    VK_LMENU, VK_LSHIFT, VK_MENU, VK_RCONTROL, VK_RETURN, VK_RMENU, VK_RSHIFT, VK_SHIFT,
    VK_SPACE, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MB_ICONASTERISK,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[link(name = "user32")]
extern "system" {
    fn MessageBeep(uType: u32) -> i32;
}

use crate::config::AppConfig;
use crate::inject::{send_atomic_batch, VIETFLOW_MAGIC};
use crate::process::{get_process_name_for_window, is_app_excluded};
use crate::tray::{WM_CONVERT, WM_SETTINGS, WM_TRAYICON};

pub struct HookState {
    pub engine: VietFlowEngine,
    pub config: AppConfig,
    pub ctrl_pressed: bool,
    pub shift_pressed: bool,
    pub alt_pressed: bool,
    pub last_foreground: HWND,
    pub is_foreground_excluded: bool,
    pub tray_hwnd: HWND,
    pub hook_handle: HHOOK,
    pub last_tick: Option<Instant>,
    pub display_shadow: Vec<char>,
}

unsafe impl Send for HookState {}
unsafe impl Sync for HookState {}

impl HookState {
    pub fn track_display_char(&mut self, c: char) {
        if !c.is_alphabetic() {
            self.display_shadow.clear();
            return;
        }
        if self.display_shadow.len() < 128 {
            self.display_shadow.push(c);
        } else {
            self.display_shadow.clear();
        }
    }

    pub fn remove_last_display_char(&mut self) {
        self.display_shadow.pop();
    }

    pub fn reset_display_shadow(&mut self) {
        self.display_shadow.clear();
    }

    /// Minimizes caret movement and screen jitter by computing the longest common prefix
    /// between the current on-screen buffer and the newly desired replacement string.
    /// The prefix comparison works on the existing shadow buffer, so a normal
    /// keystroke does not allocate.
    pub fn send_engine_output(&mut self, backspaces: usize, text: &str) {
        if backspaces == 0 && text.is_empty() {
            return;
        }

        let contains_break = text
            .chars()
            .any(|c| c.is_whitespace() || c.is_ascii_punctuation() || c.is_ascii_digit());

        let text_chars = text.chars().count();
        let can_diff = backspaces > 0
            && backspaces <= self.display_shadow.len()
            && self.display_shadow.len() <= 128
            && text_chars <= 256
            && !contains_break;

        if !can_diff {
            send_atomic_batch(backspaces, text);
            if contains_break {
                self.reset_display_shadow();
            } else {
                if backspaces > self.display_shadow.len() {
                    self.reset_display_shadow();
                } else {
                    let new_len = self.display_shadow.len() - backspaces;
                    self.display_shadow.truncate(new_len);
                }
                for c in text.chars() {
                    if self.display_shadow.len() < 128 {
                        self.display_shadow.push(c);
                    } else {
                        self.reset_display_shadow();
                        break;
                    }
                }
            }
            return;
        }

        // Desired content is the kept prefix followed by `text`. The kept prefix
        // is known to match; extend the common prefix into the replaced region.
        let prefix_len = self.display_shadow.len() - backspaces;
        let shadow_len = self.display_shadow.len();
        let mut common = prefix_len;
        let mut suffix_offset = 0;

        for (offset, c) in text.char_indices() {
            if common < shadow_len && self.display_shadow[common] == c {
                common += 1;
                suffix_offset = offset + c.len_utf8();
            } else {
                suffix_offset = offset;
                break;
            }
        }

        send_atomic_batch(shadow_len - common, &text[suffix_offset..]);

        self.display_shadow.truncate(prefix_len);
        self.display_shadow.extend(text.chars());
        if self.display_shadow.len() > 128 {
            self.reset_display_shadow();
        }
    }
}

pub static STATE: Mutex<Option<HookState>> = Mutex::new(None);

fn engine_options_from_config(config: &AppConfig) -> EngineOptions {
    let method = match config.method {
        1 => InputMethod::SimpleTelex,
        2 => InputMethod::Vni,
        3 => InputMethod::Viqr,
        _ => InputMethod::Telex,
    };

    EngineOptions {
        method,
        spell_check: config.spell_check,
        modern_style: config.modern_style,
        free_marking: config.free_marking,
        auto_non_vn_restore: config.auto_restore,
        fix_shift_lag: config.fix_shift_lag,
        consonant_shortcuts: config.consonant_shortcuts,
        auto_capitalize: config.auto_capitalize,
        enable_macro: config.enable_macro,
        allow_macro_when_disabled: config.allow_macro_when_disabled,
        use_clipboard_for_unicode: config.use_clipboard_for_unicode,
        foreign_initials: config.normalized_foreign_initials(),
        custom_english_keywords: config.normalized_english_keywords(),
    }
}

fn charset_from_config(config: &AppConfig) -> CharsetId {
    match config.charset {
        1 => CharsetId::UnicodeComposite,
        2 => CharsetId::Tcvn3,
        3 => CharsetId::VniWindows,
        _ => CharsetId::Unicode,
    }
}

/// Apply a freshly loaded configuration to the running hook without restarting
/// the daemon. The engine state is reset so the next keystroke starts a word
/// with the new options.
pub unsafe fn apply_config(config: AppConfig) {
    let mut lock = match STATE.lock() {
        Ok(lock) => lock,
        Err(_) => return,
    };
    let state = match lock.as_mut() {
        Some(state) => state,
        None => return,
    };

    state.engine.options = engine_options_from_config(&config);
    state.engine.charset = charset_from_config(&config);
    state.engine.macros.clear();
    for (trigger, replacement) in &config.macros {
        state.engine.macros.insert(trigger, replacement);
    }
    state.config = config;
    state.engine.reset();
    state.reset_display_shadow();
}

pub unsafe fn install_hook(tray_hwnd: HWND, config: AppConfig) {
    let options = engine_options_from_config(&config);
    let charset = charset_from_config(&config);

    let mut engine = VietFlowEngine::new(options, charset);
    engine.macros.clear();
    for (k, v) in &config.macros {
        engine.macros.insert(k, v);
    }

    let hook_handle = SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(low_level_keyboard_proc),
        0 as HMODULE,
        0,
    );

    let mut lock = STATE.lock().unwrap();
    *lock = Some(HookState {
        engine,
        config,
        ctrl_pressed: false,
        shift_pressed: false,
        alt_pressed: false,
        last_foreground: null_mut(),
        is_foreground_excluded: false,
        tray_hwnd,
        hook_handle,
        last_tick: None,
        display_shadow: Vec::with_capacity(128),
    });
}

pub unsafe fn uninstall_hook() {
    let mut lock = STATE.lock().unwrap();
    if let Some(state) = lock.take() {
        if !state.hook_handle.is_null() {
            UnhookWindowsHookEx(state.hook_handle);
        }
    }
}

pub unsafe extern "system" fn low_level_keyboard_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code < 0 {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    let kbd = *(l_param as *const KBDLLHOOKSTRUCT);

    // Skip all injected keys (prevents recursive loops when SendInput emits keys)
    if (kbd.flags & LLKHF_INJECTED) != 0 || kbd.dwExtraInfo == VIETFLOW_MAGIC {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    let mut lock = match STATE.try_lock() {
        Ok(l) => l,
        Err(_) => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

    let state = match lock.as_mut() {
        Some(s) => s,
        None => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

    let foreground = GetForegroundWindow();
    if foreground != state.last_foreground {
        state.last_foreground = foreground;
        state.engine.reset();
        state.reset_display_shadow();

        // Check if new foreground app is in excluded list (Game Mode)
        let proc_name = get_process_name_for_window(foreground);
        state.is_foreground_excluded = is_app_excluded(proc_name.as_deref(), &state.config.excluded_apps);
    }

    if state.is_foreground_excluded {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    let msg = w_param as u32;
    let is_key_down = msg == WM_KEYDOWN || msg == WM_SYSKEYDOWN;
    let _is_key_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    let vk = kbd.vkCode as u16;

    // Track modifiers
    match vk {
        VK_LCONTROL | VK_RCONTROL | VK_CONTROL => {
            state.ctrl_pressed = is_key_down;
        }
        VK_LSHIFT | VK_RSHIFT | VK_SHIFT => {
            state.shift_pressed = is_key_down;
        }
        VK_LMENU | VK_RMENU | VK_MENU => {
            state.alt_pressed = is_key_down;
        }
        _ => {}
    }

    if is_key_down {
        // Global Hotkey: Ctrl + Shift + F5 (Open Settings)
        if state.ctrl_pressed && state.shift_pressed && vk == VK_F5 {
            PostMessageW(state.tray_hwnd, WM_SETTINGS, 0, 0);
            return 1;
        }

        // Global Hotkey: Ctrl + Shift + F9 (Open Converter Toolkit)
        if state.ctrl_pressed && state.shift_pressed && vk == VK_F9 {
            PostMessageW(state.tray_hwnd, WM_CONVERT, 0, 0);
            return 1;
        }

        // Language toggle check
        let is_toggle = match state.config.hotkey_mode.as_str() {
            "CtrlShift" => state.ctrl_pressed && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "AltZ" => state.alt_pressed && (vk == b'Z' as u16),
            "CtrlSpace" => state.ctrl_pressed && vk == VK_SPACE,
            "ShiftSpace" => state.shift_pressed && vk == VK_SPACE,
            "AltShift" => state.alt_pressed && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "CapsLock" => vk == VK_CAPITAL,
            _ => state.ctrl_pressed && (vk == VK_LSHIFT || vk == VK_RSHIFT),
        };

        if is_toggle {
            state.config.enabled = !state.config.enabled;
            state.engine.reset();
            state.config.save();

            if state.config.notification_sound {
                MessageBeep(MB_ICONASTERISK);
            }

            let tray_hwnd = state.tray_hwnd;
            let enabled = state.config.enabled;
            drop(lock);

            // Notify tray window
            PostMessageW(tray_hwnd, WM_TRAYICON, 0, if enabled { 1 } else { 0 });
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }
    }

    // Reset keys: Esc, Enter, Tab, Arrow keys
    if is_key_down && matches!(vk, VK_ESCAPE | VK_RETURN | VK_TAB | 0x25..=0x28) {
        state.engine.reset();
        state.reset_display_shadow();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Backspace handling
    if is_key_down && vk == VK_BACK && (state.config.enabled || state.config.allow_macro_when_disabled) {
        let (backspaces, text) = state.engine.process_key('\x08');
        if backspaces > 0 || !text.is_empty() {
            state.send_engine_output(backspaces, &text);
            return 1;
        }
        state.remove_last_display_char();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // If Ctrl, Alt or Win is pressed, pass key through without processing Vietnamese
    if state.ctrl_pressed || state.alt_pressed {
        state.engine.reset();
        state.reset_display_shadow();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // If Vietnamese is disabled and macro is not allowed when disabled, pass key through
    if !state.config.enabled && !state.config.allow_macro_when_disabled {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Process alphanumeric/symbol key
    if is_key_down {
        let input_char = vk_to_char(vk, state.shift_pressed);
        if input_char != '\0' {
            let now = Instant::now();
            let timeout_ms = state.config.typing_timeout_ms.clamp(300, 10000);
            if let Some(prev) = state.last_tick {
                if now.duration_since(prev).as_millis() as u64 > timeout_ms {
                    state.engine.reset();
                    state.reset_display_shadow();
                }
            }
            state.last_tick = Some(now);

            let (backspaces, text) = state.engine.process_key(input_char);

            // If the engine transformed the text (applied tone, circumflex, restored chars, or macro expanded)
            let mut utf8 = [0u8; 4];
            let unchanged = text.as_bytes() == input_char.encode_utf8(&mut utf8).as_bytes();
            if backspaces > 0 || !unchanged {
                state.send_engine_output(backspaces, &text);
                return 1; // Consume key
            }

            state.track_display_char(input_char);
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }
    }

    CallNextHookEx(null_mut(), n_code, w_param, l_param)
}

fn vk_to_char(vk: u16, shift: bool) -> char {
    let caps = unsafe { (GetKeyState(VK_CAPITAL as i32) & 1) != 0 };
    let is_upper = shift ^ caps;

    if (b'A' as u16..=b'Z' as u16).contains(&vk) {
        let base = vk as u8;
        return if is_upper {
            base as char
        } else {
            (base + 32) as char
        };
    }

    if (b'0' as u16..=b'9' as u16).contains(&vk) {
        if !shift {
            return vk as u8 as char;
        }
        return match vk as u8 {
            b'1' => '!',
            b'2' => '@',
            b'3' => '#',
            b'4' => '$',
            b'5' => '%',
            b'6' => '^',
            b'7' => '&',
            b'8' => '*',
            b'9' => '(',
            b'0' => ')',
            _ => '\0',
        };
    }

    if vk == VK_SPACE {
        return ' ';
    }

    match vk {
        0xBA => if shift { ':' } else { ';' },
        0xBB => if shift { '+' } else { '=' },
        0xBC => if shift { '<' } else { ',' },
        0xBD => if shift { '_' } else { '-' },
        0xBE => if shift { '>' } else { '.' },
        0xBF => if shift { '?' } else { '/' },
        0xC0 => if shift { '~' } else { '`' },
        0xDB => if shift { '{' } else { '[' },
        0xDC => if shift { '|' } else { '\\' },
        0xDD => if shift { '}' } else { ']' },
        0xDE => if shift { '"' } else { '\'' },
        _ => '\0',
    }
}
