// VietFlow Windows - Low-Level Keyboard Hook (WH_KEYBOARD_LL) & App Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::Instant;
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE,
    VK_F1, VK_F5, VK_F9, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetForegroundWindow, GetWindowThreadProcessId, PostMessageW, SetWindowsHookExW,
    UnhookWindowsHookEx, HHOOK, KBDLLHOOKSTRUCT, LLKHF_INJECTED, MB_ICONASTERISK,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP, WM_SYSKEYDOWN, WM_SYSKEYUP,
};

pub type HWINEVENTHOOK = *mut std::ffi::c_void;
pub const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;

pub type WINEVENTPROC = Option<
    unsafe extern "system" fn(
        h_win_event_hook: HWINEVENTHOOK,
        event: u32,
        hwnd: HWND,
        id_object: i32,
        id_child: i32,
        id_event_thread: u32,
        dwms_event_time: u32,
    ),
>;

#[link(name = "user32")]
extern "system" {
    fn MessageBeep(uType: u32) -> i32;
    fn SetWinEventHook(
        eventMin: u32,
        eventMax: u32,
        hmodWinEventProc: HMODULE,
        pfnWinEventProc: WINEVENTPROC,
        idProcess: u32,
        idThread: u32,
        dwFlags: u32,
    ) -> HWINEVENTHOOK;
    fn UnhookWinEvent(hWinEventHook: HWINEVENTHOOK) -> i32;
}

use crate::config::AppConfig;
use crate::inject::{send_atomic_batch, VIETFLOW_MAGIC};
use crate::process::{get_process_name_for_window, is_app_excluded};
use crate::startup::set_start_with_windows;
use crate::tray::{WM_CONVERT, WM_SETTINGS, WM_TRAYICON};

pub struct HookState {
    pub engine: VietFlowEngine,
    pub config: AppConfig,
    pub user_enabled: bool,
    pub ctrl_pressed: bool,
    pub shift_pressed: bool,
    pub alt_pressed: bool,
    pub win_pressed: bool,
    pub last_foreground: HWND,
    pub is_foreground_excluded: bool,
    /// True when the foreground window's keyboard layout is not US-EN.
    pub non_us_layout: bool,
    pub tray_hwnd: HWND,
    pub hook_handle: HHOOK,
    pub win_event_hook: HWINEVENTHOOK,
    pub last_tick: Option<Instant>,
    pub display_shadow: Vec<char>,
}

unsafe impl Send for HookState {}
unsafe impl Sync for HookState {}

impl HookState {
    #[inline]
    pub fn effective_enabled(&self) -> bool {
        self.user_enabled
            && !self.is_foreground_excluded
            && !(self.config.disable_on_non_us_layout && self.non_us_layout)
    }

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
        1 => InputMethod::Vni,
        2 => InputMethod::Viqr,
        3 => InputMethod::SimpleTelex,
        4 => InputMethod::TelexSimple2,
        5 => InputMethod::TelexVni,
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

/// Check and react when the foreground active window changes.
/// If entering an excluded app: automatically flip to English [E].
/// When leaving back to normal app: automatically restore the user's selected mode.
unsafe fn is_non_us_layout(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let thread_id = GetWindowThreadProcessId(hwnd, null_mut());
    if thread_id == 0 {
        return false;
    }
    let layout = GetKeyboardLayout(thread_id);
    if layout.is_null() {
        return false;
    }
    // Low word of an HKL is the input language identifier. 0x0409 = en-US.
    (((layout as usize) & 0xFFFF) as u16) != 0x0409
}

pub unsafe fn check_foreground_change(foreground: HWND) {
    if foreground.is_null() {
        return;
    }

    let mut lock = match STATE.try_lock() {
        Ok(l) => l,
        Err(_) => return,
    };
    let state = match lock.as_mut() {
        Some(s) => s,
        None => return,
    };

    if foreground == state.last_foreground {
        // The keyboard layout can change without a foreground event. Only
        // query it when the user enabled the non-US layout guard.
        if state.config.disable_on_non_us_layout {
            let non_us = is_non_us_layout(foreground);
            if non_us != state.non_us_layout {
                state.non_us_layout = non_us;
                let tray_hwnd = state.tray_hwnd;
                let effective = state.effective_enabled();
                drop(lock);
                PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
            }
        }
        return;
    }

    state.last_foreground = foreground;
    state.engine.reset();
    state.reset_display_shadow();

    let proc_name = get_process_name_for_window(foreground);
    let excluded = is_app_excluded(proc_name.as_deref(), &state.config.excluded_apps);
    let non_us = state.config.disable_on_non_us_layout && is_non_us_layout(foreground);

    if excluded != state.is_foreground_excluded || non_us != state.non_us_layout {
        state.is_foreground_excluded = excluded;
        state.non_us_layout = non_us;
        let tray_hwnd = state.tray_hwnd;
        let effective = state.effective_enabled();
        drop(lock);

        PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
    }
}

unsafe extern "system" fn win_event_proc(
    _h_win_event_hook: HWINEVENTHOOK,
    event: u32,
    hwnd: HWND,
    _id_object: i32,
    _id_child: i32,
    _id_event_thread: u32,
    _dwms_event_time: u32,
) {
    if event == EVENT_SYSTEM_FOREGROUND {
        check_foreground_change(hwnd);
    }
}

/// Apply a freshly loaded configuration to the running hook without restarting daemon.
pub unsafe fn apply_config(config: AppConfig) {
    let mut lock = match STATE.lock() {
        Ok(lock) => lock,
        Err(_) => return,
    };
    let state = match lock.as_mut() {
        Some(state) => state,
        None => return,
    };

    state.user_enabled = config.enabled;
    state.engine.options = engine_options_from_config(&config);
    state.engine.charset = charset_from_config(&config);
    state.engine.macros.clear();
    for (trigger, replacement) in &config.macros {
        state.engine.macros.insert(trigger, replacement);
    }

    let foreground = state.last_foreground;
    let non_us = config.disable_on_non_us_layout
        && !foreground.is_null()
        && is_non_us_layout(foreground);
    let start_with_windows = config.start_with_windows;
    let startup_changed = state.config.start_with_windows != start_with_windows;

    if !foreground.is_null() {
        let proc_name = get_process_name_for_window(foreground);
        state.is_foreground_excluded =
            is_app_excluded(proc_name.as_deref(), &config.excluded_apps);
    }

    state.non_us_layout = non_us;
    state.config = config;
    state.engine.reset();
    state.reset_display_shadow();

    let tray_hwnd = state.tray_hwnd;
    let effective = state.effective_enabled();
    drop(lock);

    // The UI only writes the JSON file; apply the registry side-effect here so
    // "Khởi động cùng Windows" takes effect without restarting the daemon.
    if startup_changed {
        set_start_with_windows(start_with_windows);
    }

    PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
}

pub unsafe fn install_hook(tray_hwnd: HWND, config: AppConfig) {
    let user_enabled = config.enabled;
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

    let win_event_hook = SetWinEventHook(
        EVENT_SYSTEM_FOREGROUND,
        EVENT_SYSTEM_FOREGROUND,
        null_mut(),
        Some(win_event_proc),
        0,
        0,
        WINEVENT_OUTOFCONTEXT,
    );

    let initial_foreground = GetForegroundWindow();
    let initial_proc = get_process_name_for_window(initial_foreground);
    let initial_excluded = is_app_excluded(initial_proc.as_deref(), &config.excluded_apps);
    let initial_non_us = config.disable_on_non_us_layout && is_non_us_layout(initial_foreground);

    let mut lock = STATE.lock().unwrap();
    *lock = Some(HookState {
        engine,
        config,
        user_enabled,
        ctrl_pressed: false,
        shift_pressed: false,
        alt_pressed: false,
        win_pressed: false,
        last_foreground: initial_foreground,
        is_foreground_excluded: initial_excluded,
        non_us_layout: initial_non_us,
        tray_hwnd,
        hook_handle,
        win_event_hook,
        last_tick: None,
        display_shadow: Vec::with_capacity(128),
    });
}

pub unsafe fn uninstall_hook() {
    let mut lock = STATE.lock().unwrap();
    if let Some(state) = lock.take() {
        if !state.win_event_hook.is_null() {
            UnhookWinEvent(state.win_event_hook);
        }
        if !state.hook_handle.is_null() {
            UnhookWindowsHookEx(state.hook_handle);
        }
    }
}

#[inline]
unsafe fn is_vk_down(vk: u16) -> bool {
    (GetAsyncKeyState(vk as i32) as i16) < 0
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

    let foreground = GetForegroundWindow();
    check_foreground_change(foreground);

    let mut lock = match STATE.try_lock() {
        Ok(l) => l,
        Err(_) => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

    let state = match lock.as_mut() {
        Some(s) => s,
        None => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

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
        VK_LWIN | VK_RWIN => {
            state.win_pressed = is_key_down;
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
            "CtrlShift" => (state.ctrl_pressed || is_vk_down(VK_CONTROL)) && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "AltZ" => (state.alt_pressed || is_vk_down(VK_MENU)) && (vk == b'Z' as u16),
            "CtrlSpace" => (state.ctrl_pressed || is_vk_down(VK_CONTROL)) && vk == VK_SPACE,
            "ShiftSpace" => (state.shift_pressed || is_vk_down(VK_SHIFT)) && vk == VK_SPACE,
            "AltShift" => (state.alt_pressed || is_vk_down(VK_MENU)) && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "CapsLock" => vk == VK_CAPITAL,
            "WinSpace" => (state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN)) && vk == VK_SPACE,
            other => check_custom_hotkey(
                other,
                state.ctrl_pressed || is_vk_down(VK_CONTROL),
                state.alt_pressed || is_vk_down(VK_MENU),
                state.shift_pressed || is_vk_down(VK_SHIFT),
                state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN),
                vk,
            ),
        };

        if is_toggle {
            state.user_enabled = !state.user_enabled;
            state.config.enabled = state.user_enabled;
            state.engine.reset();
            state.config.save();

            if state.config.notification_sound {
                MessageBeep(MB_ICONASTERISK);
            }

            let tray_hwnd = state.tray_hwnd;
            let effective = state.effective_enabled();
            drop(lock);

            // Notify tray window
            PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });

            // If toggle key is a non-modifier key (e.g. Z, Space, F-key), consume it to prevent typing
            let is_mod = matches!(
                vk,
                VK_LCONTROL | VK_RCONTROL | VK_CONTROL |
                VK_LSHIFT | VK_RSHIFT | VK_SHIFT |
                VK_LMENU | VK_RMENU | VK_MENU |
                VK_LWIN | VK_RWIN
            );
            if !is_mod {
                return 1;
            }
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }
    }

    // If active foreground app is in excluded list, completely bypass all typing processing
    if state.is_foreground_excluded {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Reset keys: Esc, Enter, Tab, Arrow keys
    if is_key_down && matches!(vk, VK_ESCAPE | VK_RETURN | VK_TAB | 0x25..=0x28) {
        state.engine.reset();
        state.reset_display_shadow();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Backspace handling
    if is_key_down && vk == VK_BACK && (state.effective_enabled() || state.config.allow_macro_when_disabled) {
        let (backspaces, text) = state.engine.process_key('\x08');
        if backspaces > 0 || !text.is_empty() {
            state.send_engine_output(backspaces, &text);
            return 1;
        }
        state.remove_last_display_char();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    let win_down = state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);
    let ctrl_down = state.ctrl_pressed || is_vk_down(VK_CONTROL);
    let alt_down = state.alt_pressed || is_vk_down(VK_MENU);

    // If Ctrl, Alt or Win is pressed, pass key through without processing Vietnamese
    if ctrl_down || alt_down || win_down {
        state.engine.reset();
        state.reset_display_shadow();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // If effective typing is disabled and macro is not allowed when disabled, pass key through
    if !state.effective_enabled() && !state.config.allow_macro_when_disabled {
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

            if backspaces > 0 || !text.is_empty() {
                state.send_engine_output(backspaces, &text);
                return 1;
            }

            state.track_display_char(input_char);
        }
    }

    CallNextHookEx(null_mut(), n_code, w_param, l_param)
}

fn vk_to_char(vk: u16, shift: bool) -> char {
    let caps_lock = unsafe { (GetKeyState(VK_CAPITAL as i32) & 0x0001) != 0 };
    let is_upper = shift ^ caps_lock;

    match vk {
        0x41..=0x5A => {
            let base = if is_upper { b'A' } else { b'a' };
            (base + (vk as u8 - 0x41)) as char
        }
        0x30..=0x39 => {
            if shift {
                match vk {
                    0x31 => '!',
                    0x32 => '@',
                    0x33 => '#',
                    0x34 => '$',
                    0x35 => '%',
                    0x36 => '^',
                    0x37 => '&',
                    0x38 => '*',
                    0x39 => '(',
                    0x30 => ')',
                    _ => '\0',
                }
            } else {
                (b'0' + (vk as u8 - 0x30)) as char
            }
        }
        0xBA => { if shift { ':' } else { ';' } }
        0xBB => { if shift { '+' } else { '=' } }
        0xBC => { if shift { '<' } else { ',' } }
        0xBD => { if shift { '_' } else { '-' } }
        0xBE => { if shift { '>' } else { '.' } }
        0xBF => { if shift { '?' } else { '/' } }
        0xC0 => { if shift { '~' } else { '`' } }
        0xDB => { if shift { '{' } else { '[' } }
        0xDC => { if shift { '|' } else { '\\' } }
        0xDD => { if shift { '}' } else { ']' } }
        0xDE => { if shift { '"' } else { '\'' } }
        VK_SPACE => ' ',
        _ => '\0',
    }
}

unsafe fn check_custom_hotkey(
    mode: &str,
    ctrl: bool,
    alt: bool,
    shift: bool,
    win: bool,
    vk: u16,
) -> bool {
    let s = mode.strip_prefix("Custom:").unwrap_or(mode).trim();
    if s.is_empty() {
        return false;
    }

    // Direct normalized preset checks (e.g. "Ctrl + Shift", "ctrl+shift", "CtrlShift")
    match s.to_ascii_uppercase().replace(" ", "").as_str() {
        "CTRLSHIFT" => return (ctrl || is_vk_down(VK_CONTROL)) && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT),
        "ALTZ" => return (alt || is_vk_down(VK_MENU)) && (vk == b'Z' as u16),
        "CTRLSPACE" => return (ctrl || is_vk_down(VK_CONTROL)) && vk == VK_SPACE,
        "SHIFTSPACE" => return (shift || is_vk_down(VK_SHIFT)) && vk == VK_SPACE,
        "ALTSHIFT" => return (alt || is_vk_down(VK_MENU)) && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT),
        "CAPSLOCK" | "CAPS" => return vk == VK_CAPITAL,
        "WINSPACE" => return (win || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN)) && vk == VK_SPACE,
        _ => {}
    }

    let parts: Vec<&str> = s.split('+').map(|p| p.trim()).collect();
    let mut req_ctrl = false;
    let mut req_alt = false;
    let mut req_shift = false;
    let mut req_win = false;
    let mut target_key = "";

    for &part in &parts {
        if part.eq_ignore_ascii_case("Ctrl") || part.eq_ignore_ascii_case("Control") {
            req_ctrl = true;
        } else if part.eq_ignore_ascii_case("Alt") {
            req_alt = true;
        } else if part.eq_ignore_ascii_case("Shift") {
            req_shift = true;
        } else if part.eq_ignore_ascii_case("Win") || part.eq_ignore_ascii_case("Windows") {
            req_win = true;
        } else {
            target_key = part;
        }
    }

    if req_ctrl != ctrl || req_alt != alt || req_shift != shift || req_win != win {
        return false;
    }

    if target_key.is_empty() {
        // Pure modifier combo: trigger only when one of the required modifiers is pressed
        if req_ctrl && (vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL) {
            return true;
        }
        if req_shift && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT) {
            return true;
        }
        if req_alt && (vk == VK_LMENU || vk == VK_RMENU || vk == VK_MENU) {
            return true;
        }
        if req_win && (vk == VK_LWIN || vk == VK_RWIN) {
            return true;
        }
        return false;
    }

    match target_key.to_ascii_uppercase().as_str() {
        "SPACE" => vk == VK_SPACE,
        "CAPSLOCK" | "CAPS" => vk == VK_CAPITAL,
        "TAB" => vk == VK_TAB,
        "TILDE" | "`" | "~" => vk == 0xC0,
        "ESCAPE" | "ESC" => vk == VK_ESCAPE,
        f if f.starts_with('F') && f.len() >= 2 => {
            if let Ok(num) = f[1..].parse::<u16>() {
                if (1..=12).contains(&num) {
                    vk == (VK_F1 + num - 1)
                } else {
                    false
                }
            } else {
                false
            }
        }
        k if k.len() == 1 => {
            let ch = k.as_bytes()[0];
            if ch.is_ascii_alphanumeric() {
                vk == ch.to_ascii_uppercase() as u16
            } else {
                false
            }
        }
        _ => false,
    }
}
