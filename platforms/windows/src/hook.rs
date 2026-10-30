// VietFlow Windows - Low-Level Keyboard Hook (WH_KEYBOARD_LL) & App Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use std::sync::Mutex;
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE,
    VK_F1, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN,
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

#[link(name = "winmm")]
extern "system" {
    fn PlaySoundW(pszSound: *const u16, hmod: HMODULE, fdwSound: u32) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn Beep(dwFreq: u32, dwDuration: u32) -> i32;
    fn GetTickCount64() -> u64;
}

/// Play the configured V/E toggle notification. PlaySoundW with a system alias
/// respects the user's sound scheme; MessageBeep/Beep are fallbacks for systems
/// where the alias is unavailable or muted.
pub fn play_notification_sound() {
    const SND_ASYNC: u32 = 0x0001;
    const SND_ALIAS: u32 = 0x0001_0000;
    let alias = to_wide("SystemNotification");
    let played = unsafe { PlaySoundW(alias.as_ptr(), null_mut(), SND_ASYNC | SND_ALIAS) };
    if played == 0 {
        let beeped = unsafe { MessageBeep(MB_ICONASTERISK) };
        if beeped == 0 {
            unsafe {
                Beep(880, 80);
            }
        }
    }
}

/// C ABI used by the ImGui UI so the settings preview uses the same sound path
/// as the actual V/E toggle.
#[no_mangle]
pub extern "C" fn vf_play_notification_sound() {
    play_notification_sound();
}

use crate::config::{AppConfig, AppProfile};
use crate::inject::{send_atomic_batch_with_mode, VIETFLOW_MAGIC};
use crate::process::{get_process_name_for_window, is_app_excluded, profile_for_app};
use crate::startup::set_start_with_windows;
use crate::tray::{to_wide, WM_CONVERT, WM_SETTINGS, WM_TRAYICON};

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
    /// True when the user manually toggled V/E from the tray while the
    /// foreground app is excluded. Reset when the foreground app changes.
    pub manual_override: bool,
    /// True when the foreground window's keyboard layout is not US-EN.
    pub non_us_layout: bool,
    /// Per-app overrides resolved for the current foreground executable.
    pub active_profile: Option<AppProfile>,
    /// 0: Auto, 1: Unicode, 2: scan-code, 3: clipboard, 4: safe paced.
    pub injection_mode: u8,
    /// Raw characters typed while Vietnamese is disabled but macros are on.
    pub macro_buffer: String,
    pub tray_hwnd: HWND,
    pub hook_handle: HHOOK,
    pub win_event_hook: HWINEVENTHOOK,
}

unsafe impl Send for HookState {}
unsafe impl Sync for HookState {}

impl HookState {
    #[inline]
    pub fn effective_enabled(&self) -> bool {
        self.user_enabled
            && (!self.is_foreground_excluded || self.manual_override)
            && !(self.config.disable_on_non_us_layout && self.non_us_layout)
    }

    /// Macros are allowed when they are globally enabled and either the app
    /// profile explicitly allows them, the active mode is Vietnamese, or the
    /// user opted into macros while typing is disabled.
    #[inline]
    pub fn macros_allowed(&self) -> bool {
        if !self.config.enable_macro {
            return false;
        }
        if let Some(profile) = &self.active_profile {
            if profile.allow_macro == Some(false) {
                return false;
            }
            if profile.allow_macro == Some(true) {
                return true;
            }
        }
        self.effective_enabled() || self.config.allow_macro_when_disabled
    }

    /// True when only macro expansion should run: Vietnamese typing is off but
    /// macros are explicitly available.
    #[inline]
    pub fn macro_only(&self) -> bool {
        !self.effective_enabled() && self.macros_allowed()
    }

    /// Global hotkeys are normally locked in excluded apps so a game cannot
    /// accidentally toggle Vietnamese. Users can opt out in the exclusions
    /// page; the tray/menu override stays available either way.
    #[inline]
    pub fn hotkeys_locked(&self) -> bool {
        self.is_foreground_excluded && self.config.lock_hotkeys_in_excluded_apps
    }

    /// Toggle V/E. In an excluded app this is a manual override, not a change
    /// to the global mode: the first action enables typing, the second returns
    /// to the normal excluded state.
    pub fn toggle_language(&mut self) {
        if self.is_foreground_excluded {
            self.manual_override = !self.manual_override;
            if self.manual_override {
                self.user_enabled = true;
            }
        } else {
            self.user_enabled = !self.user_enabled;
        }
        self.config.enabled = self.user_enabled;
        self.engine.reset();
        self.macro_buffer.clear();
    }

    #[inline]
    pub fn send_engine_output(&mut self, backspaces: usize, text: &str) {
        if backspaces > 0 || !text.is_empty() {
            send_atomic_batch_with_mode(backspaces, text, self.injection_mode);
        }
    }

    /// Recompute engine options and the output mode after a foreground,
    /// profile, or config change.
    pub fn apply_runtime_profile(&mut self) {
        self.engine.options =
            engine_options_from_config(&self.config, self.active_profile.as_ref());
        self.engine.charset = charset_from_config(&self.config, self.active_profile.as_ref());
        self.injection_mode = self
            .active_profile
            .as_ref()
            .and_then(|profile| profile.injection_mode)
            .unwrap_or(self.config.injection_mode);
        // Legacy setting: clipboard mode for apps that cannot accept Unicode.
        if self.config.use_clipboard_for_unicode && self.injection_mode == 0 {
            self.injection_mode = 3;
        }
        self.engine.reset();
        self.macro_buffer.clear();
    }

    pub fn load_macros(&mut self) {
        self.engine.macros.clear();
        for (trigger, replacement) in &self.config.macros {
            self.engine.macros.insert(trigger, replacement);
        }
    }
}

pub static STATE: Mutex<Option<HookState>> = Mutex::new(None);

fn engine_options_from_config(
    config: &AppConfig,
    profile: Option<&AppProfile>,
) -> EngineOptions {
    let configured_method = profile
        .and_then(|profile| profile.method)
        .unwrap_or(config.method);
    let method = match configured_method {
        1 => InputMethod::Vni,
        2 => InputMethod::Viqr,
        3 => InputMethod::SimpleTelex,
        4 => InputMethod::TelexSimple2,
        5 => InputMethod::TelexVni,
        6 => InputMethod::UserCustom,
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
        auto_capitalize_chars: config.auto_capitalize_chars.clone(),
        enable_macro: config.enable_macro
            && profile.and_then(|profile| profile.allow_macro).unwrap_or(true),
        allow_macro_when_disabled: config.allow_macro_when_disabled,
        use_clipboard_for_unicode: config.use_clipboard_for_unicode,
        typing_timeout_ms: config.typing_timeout_ms,
        foreign_initials: config.normalized_foreign_initials(),
        custom_english_keywords: config.normalized_english_keywords(),
        removed_english_keywords: config.normalized_removed_english_keywords(),
        custom_keymap: config.custom_keymap.to_core(),
    }
}

fn charset_from_config(config: &AppConfig, profile: Option<&AppProfile>) -> CharsetId {
    let configured_charset = profile
        .and_then(|profile| profile.charset)
        .unwrap_or(config.charset);
    match configured_charset {
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
                state.apply_runtime_profile();
                let tray_hwnd = state.tray_hwnd;
                let effective = state.effective_enabled();
                drop(lock);
                PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
            }
        }
        return;
    }

    let old_excluded = state.is_foreground_excluded;
    let old_non_us = state.non_us_layout;
    let old_effective = state.effective_enabled();

    state.last_foreground = foreground;
    state.manual_override = false;
    state.macro_buffer.clear();

    let proc_name = get_process_name_for_window(foreground);
    state.is_foreground_excluded = is_app_excluded(proc_name.as_deref(), &state.config.excluded_apps);
    state.non_us_layout = state.config.disable_on_non_us_layout && is_non_us_layout(foreground);
    state.active_profile =
        profile_for_app(proc_name.as_deref(), &state.config.app_profiles).cloned();
    state.apply_runtime_profile();

    let effective = state.effective_enabled();
    if old_excluded != state.is_foreground_excluded
        || old_non_us != state.non_us_layout
        || old_effective != effective
    {
        let tray_hwnd = state.tray_hwnd;
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

    let foreground = state.last_foreground;
    let proc_name = if foreground.is_null() {
        None
    } else {
        get_process_name_for_window(foreground)
    };
    let active_profile = profile_for_app(proc_name.as_deref(), &config.app_profiles).cloned();
    let non_us = config.disable_on_non_us_layout
        && !foreground.is_null()
        && is_non_us_layout(foreground);
    let excluded = is_app_excluded(proc_name.as_deref(), &config.excluded_apps);
    let start_with_windows = config.start_with_windows;
    let startup_changed = state.config.start_with_windows != start_with_windows;

    state.user_enabled = config.enabled;
    state.is_foreground_excluded = excluded;
    state.non_us_layout = non_us;
    state.active_profile = active_profile;
    state.config = config;
    state.apply_runtime_profile();
    state.load_macros();

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

    let initial_foreground = GetForegroundWindow();
    let initial_proc = get_process_name_for_window(initial_foreground);
    let initial_excluded = is_app_excluded(initial_proc.as_deref(), &config.excluded_apps);
    let initial_non_us = config.disable_on_non_us_layout && is_non_us_layout(initial_foreground);
    let initial_profile = profile_for_app(initial_proc.as_deref(), &config.app_profiles).cloned();
    let options = engine_options_from_config(&config, initial_profile.as_ref());
    let charset = charset_from_config(&config, initial_profile.as_ref());
    let mut injection_mode = initial_profile
        .as_ref()
        .and_then(|profile| profile.injection_mode)
        .unwrap_or(config.injection_mode);
    if config.use_clipboard_for_unicode && injection_mode == 0 {
        injection_mode = 3;
    }

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

    // Keep the Run key in sync even when the daemon starts after the UI saved
    // the setting while it was not running.
    set_start_with_windows(config.start_with_windows);

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
        manual_override: false,
        non_us_layout: initial_non_us,
        active_profile: initial_profile,
        injection_mode,
        macro_buffer: String::new(),
        tray_hwnd,
        hook_handle,
        win_event_hook,
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

    if is_key_down && !state.hotkeys_locked() {
        let ctrl = state.ctrl_pressed || is_vk_down(VK_CONTROL);
        let alt = state.alt_pressed || is_vk_down(VK_MENU);
        let shift = state.shift_pressed || is_vk_down(VK_SHIFT);
        let win = state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);

        // Configurable global hotkeys (defaults: Ctrl+Shift+F5 / Ctrl+Shift+F9).
        if check_custom_hotkey(&state.config.hotkey_settings, ctrl, alt, shift, win, vk) {
            PostMessageW(state.tray_hwnd, WM_SETTINGS, 0, 0);
            return 1;
        }
        if check_custom_hotkey(&state.config.hotkey_converter, ctrl, alt, shift, win, vk) {
            PostMessageW(state.tray_hwnd, WM_CONVERT, 0, 0);
            return 1;
        }

        // Language toggle check
        let is_toggle = match state.config.hotkey_mode.as_str() {
            "CtrlShift" => ctrl && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "AltZ" => alt && (vk == b'Z' as u16),
            "CtrlSpace" => ctrl && vk == VK_SPACE,
            "ShiftSpace" => shift && vk == VK_SPACE,
            "AltShift" => alt && (vk == VK_LSHIFT || vk == VK_RSHIFT),
            "CapsLock" => vk == VK_CAPITAL,
            "WinSpace" => win && vk == VK_SPACE,
            other => check_custom_hotkey(other, ctrl, alt, shift, win, vk),
        };

        if is_toggle {
            state.toggle_language();
            state.config.save();

            if state.config.notification_sound {
                play_notification_sound();
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

    // When Vietnamese typing is off, never run the Vietnamese engine. If the
    // user allowed macros while disabled, run only the small macro tracker;
    // otherwise this is a true pass-through (games, terminals, remote apps).
    if !state.effective_enabled() {
        if is_key_down && state.macro_only() {
            return handle_macro_only(state, vk, n_code, w_param, l_param);
        }
        state.macro_buffer.clear();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Reset keys: Esc, Enter, Tab, Arrow keys
    if is_key_down && matches!(vk, VK_ESCAPE | VK_RETURN | VK_TAB | 0x25..=0x28) {
        state.engine.reset();
        state.macro_buffer.clear();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Backspace handling
    if is_key_down && vk == VK_BACK {
        let (backspaces, text) = state.engine.process_key('\x08');
        if backspaces > 0 || !text.is_empty() {
            state.send_engine_output(backspaces, &text);
            return 1;
        }
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    let win_down = state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);
    let ctrl_down = state.ctrl_pressed || is_vk_down(VK_CONTROL);
    let alt_down = state.alt_pressed || is_vk_down(VK_MENU);

    // If Ctrl, Alt or Win is pressed, pass key through without processing Vietnamese
    if ctrl_down || alt_down || win_down {
        state.engine.reset();
        state.macro_buffer.clear();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Process alphanumeric/symbol key
    if is_key_down {
        let input_char = vk_to_char(vk, state.shift_pressed);
        if input_char != '\0' {
            let now_ms = unsafe { GetTickCount64() };
            let (backspaces, text) = state.engine.process_key_with_time(input_char, now_ms);

            if backspaces > 0 || !text.is_empty() {
                state.send_engine_output(backspaces, &text);
                return 1;
            }
        }
    }

    CallNextHookEx(null_mut(), n_code, w_param, l_param)
}

/// Macro-only path used when Vietnamese typing is disabled but the user asked
/// for macros to keep working (English mode or an excluded app). Printable
/// keys are allowed to reach the target app; only the trigger is remembered and
/// replaced when a word break arrives.
unsafe fn handle_macro_only(
    state: &mut HookState,
    vk: u16,
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    let pass = || CallNextHookEx(null_mut(), n_code, w_param, l_param);

    if vk == VK_BACK {
        state.macro_buffer.pop();
        return pass();
    }

    if matches!(vk, VK_RETURN | VK_TAB | 0x25..=0x28) {
        expand_macro_only(state);
        return pass();
    }
    if vk == VK_ESCAPE {
        state.macro_buffer.clear();
        return pass();
    }

    let ctrl = state.ctrl_pressed || is_vk_down(VK_CONTROL);
    let alt = state.alt_pressed || is_vk_down(VK_MENU);
    let win = state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);
    if ctrl || alt || win {
        state.macro_buffer.clear();
        return pass();
    }

    let c = vk_to_char(vk, state.shift_pressed);
    if c == '\0' {
        state.macro_buffer.clear();
        return pass();
    }

    let is_word_break = c.is_whitespace() || (!c.is_alphanumeric() && c != '_');
    if is_word_break {
        expand_macro_only(state);
    } else if state.macro_buffer.chars().count() < 64 {
        state.macro_buffer.push(c);
    } else {
        state.macro_buffer.clear();
    }

    pass()
}

fn expand_macro_only(state: &mut HookState) {
    if state.macro_buffer.is_empty() {
        return;
    }
    let trigger = state.macro_buffer.to_lowercase();
    if let Some(replacement) = state.config.macros.get(&trigger).cloned() {
        let backspaces = state.macro_buffer.chars().count();
        state.send_engine_output(backspaces, &replacement);
    }
    state.macro_buffer.clear();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn hook_state(excluded: bool, manual_override: bool, user_enabled: bool) -> HookState {
        HookState {
            engine: VietFlowEngine::default(),
            config: AppConfig::default(),
            user_enabled,
            ctrl_pressed: false,
            shift_pressed: false,
            alt_pressed: false,
            win_pressed: false,
            last_foreground: std::ptr::null_mut(),
            is_foreground_excluded: excluded,
            manual_override,
            non_us_layout: false,
            active_profile: None,
            injection_mode: 0,
            macro_buffer: String::new(),
            tray_hwnd: std::ptr::null_mut(),
            hook_handle: std::ptr::null_mut(),
            win_event_hook: std::ptr::null_mut(),
        }
    }

    #[test]
    fn manual_override_allows_typing_in_excluded_app() {
        assert!(!hook_state(true, false, true).effective_enabled());
        assert!(hook_state(true, true, true).effective_enabled());
        assert!(!hook_state(true, true, false).effective_enabled());
        assert!(hook_state(false, false, true).effective_enabled());
        assert!(!hook_state(false, false, false).effective_enabled());
    }

    #[test]
    fn toggle_language_uses_manual_override_in_excluded_apps() {
        let mut state = hook_state(true, false, true);
        state.toggle_language();
        assert!(state.manual_override);
        assert!(state.user_enabled);
        assert!(state.effective_enabled());

        state.toggle_language();
        assert!(!state.manual_override);
        assert!(!state.effective_enabled());

        let mut normal = hook_state(false, false, true);
        normal.toggle_language();
        assert!(!normal.user_enabled);
        normal.toggle_language();
        assert!(normal.user_enabled);
    }

    #[test]
    fn hotkey_lock_can_be_disabled_per_configuration() {
        let mut state = hook_state(true, false, true);
        assert!(state.hotkeys_locked());
        state.config.lock_hotkeys_in_excluded_apps = false;
        assert!(!state.hotkeys_locked());
        assert!(!hook_state(false, false, true).hotkeys_locked());
    }

    #[test]
    fn macros_are_macro_only_when_typing_is_disabled() {
        let mut state = hook_state(true, false, true);
        assert!(!state.macros_allowed());
        assert!(!state.macro_only());

        state.config.allow_macro_when_disabled = true;
        assert!(state.macros_allowed());
        assert!(state.macro_only());

        state.active_profile = Some(AppProfile {
            allow_macro: Some(false),
            ..Default::default()
        });
        assert!(!state.macros_allowed());
        assert!(!state.macro_only());
    }

    #[test]
    fn profile_can_force_macros_on_outside_excluded_apps() {
        let mut state = hook_state(false, false, true);
        state.active_profile = Some(AppProfile {
            allow_macro: Some(true),
            ..Default::default()
        });
        assert!(state.macros_allowed());
        assert!(!state.macro_only());
    }

    #[test]
    fn matches_configurable_hotkeys() {
        unsafe {
            assert!(check_custom_hotkey(
                "Ctrl + Shift + F5",
                true,
                false,
                true,
                false,
                0x74
            ));
            assert!(!check_custom_hotkey(
                "Ctrl + Shift + F5",
                true,
                false,
                false,
                false,
                0x74
            ));
            assert!(check_custom_hotkey(
                "Ctrl + Shift + F9",
                true,
                false,
                true,
                false,
                0x78
            ));
            assert!(check_custom_hotkey(
                "Alt + Z",
                false,
                true,
                false,
                false,
                0x5A
            ));
        }
    }
}
