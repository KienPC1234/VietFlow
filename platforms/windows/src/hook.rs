// VietFlow Windows - Low-Level Keyboard Hook (WH_KEYBOARD_LL) & App Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use std::sync::{mpsc, Mutex, OnceLock};
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, VK_BACK, VK_CAPITAL, VK_CONTROL, VK_ESCAPE,
    VK_F1, VK_LCONTROL, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU, VK_RCONTROL, VK_RETURN,
    VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId,
    IsWindowUnicode, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
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

/// A small IMA ADPCM chime, independent of the Windows sound scheme or media
/// files. Kept as a separate aligned copy because PlaySound reads it as a
/// byte buffer through SND_MEMORY.
const NOTIFICATION_SOUND: &[u8] = include_bytes!("../../../assets/notification.wav");
static SOUND_DATA: OnceLock<Vec<u8>> = OnceLock::new();

/// One worker per process. It serializes sounds so rapid V/E toggles cannot
/// overlap or cancel each other, and coalesces a burst into one sound.
static SOUND_TX: OnceLock<mpsc::Sender<()>> = OnceLock::new();

fn start_sound_worker() -> mpsc::Sender<()> {
    let (tx, rx) = mpsc::channel::<()>();
    let _ = std::thread::Builder::new()
        .name("vf-notification-sound".to_string())
        .spawn(move || {
            while rx.recv().is_ok() {
                while rx.try_recv().is_ok() {}
                play_notification_sound_blocking();
                // Drop requests that arrived while the sound was playing so a
                // rapid V/E burst cannot produce a queue of overlapping tones.
                while rx.try_recv().is_ok() {}
            }
        });
    tx
}

/// Queue a notification sound without blocking the hook or UI thread.
pub fn play_notification_sound() {
    let sender = SOUND_TX.get_or_init(start_sound_worker);
    let _ = sender.send(());
}

fn play_notification_sound_blocking() {
    const SND_SYNC: u32 = 0x0000;
    const SND_NODEFAULT: u32 = 0x0002;
    const SND_MEMORY: u32 = 0x0004;
    const SND_FILENAME: u32 = 0x0002_0000;
    const SND_ALIAS: u32 = 0x0001_0000;
    const MB_ICONASTERISK: u32 = 0x0000_0040;

    // Primary path: the embedded chime. No media file, codec lookup through
    // the sound scheme, or extra process is involved.
    let sound = SOUND_DATA.get_or_init(|| NOTIFICATION_SOUND.to_vec());
    let res = unsafe {
        PlaySoundW(
            sound.as_ptr() as *const u16,
            null_mut(),
            SND_MEMORY | SND_SYNC | SND_NODEFAULT,
        )
    };
    if res != 0 {
        return;
    }

    // Fallback for a stripped-down Windows installation without the ADPCM
    // codec. Never assume C:\Windows: Windows can be installed on another drive.
    let media = std::env::var_os("SystemRoot")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(r"C:\Windows"))
        .join("Media");

    // Prefer the short, light Windows notification sounds; fall back to the
    // standard notification and legacy beeps. SND_NODEFAULT lets the next
    // candidate run if a file is damaged.
    let sound_candidates = [
        "notify.wav",
        "ding.wav",
        "Windows Notify System Generic.wav",
        "Windows Notify.wav",
        "Windows Ding.wav",
        "chimes.wav",
    ];

    for name in sound_candidates {
        let path = media.join(name);
        if !path.is_file() {
            continue;
        }
        let wide = to_wide(&path.to_string_lossy());
        let res = unsafe {
            PlaySoundW(
                wide.as_ptr(),
                null_mut(),
                SND_SYNC | SND_FILENAME | SND_NODEFAULT,
            )
        };
        if res != 0 {
            return;
        }
    }

    // Some installations omit the .wav files but keep the system alias.
    let alias = to_wide("SystemNotification");
    let res = unsafe {
        PlaySoundW(
            alias.as_ptr(),
            null_mut(),
            SND_SYNC | SND_ALIAS | SND_NODEFAULT,
        )
    };
    if res != 0 {
        return;
    }

    let beeped = unsafe { MessageBeep(MB_ICONASTERISK) };
    if beeped == 0 {
        unsafe {
            Beep(1200, 60);
        }
    }
}

/// C ABI used by the ImGui UI so the settings preview uses the same sound path
/// as the actual V/E toggle.
#[no_mangle]
pub extern "C" fn vf_play_notification_sound() {
    play_notification_sound();
}

#[no_mangle]
pub extern "C" fn vf_is_tsf_present() -> bool {
    crate::tsf::is_tsf_dll_present()
}

#[no_mangle]
pub extern "C" fn vf_is_tsf_registered() -> bool {
    crate::tsf::is_tsf_registered()
}

#[no_mangle]
pub extern "C" fn vf_register_tsf() -> bool {
    crate::tsf::register_tsf().is_ok()
}

#[no_mangle]
pub extern "C" fn vf_unregister_tsf() -> bool {
    crate::tsf::unregister_tsf().is_ok()
}

#[no_mangle]
pub extern "C" fn vf_reinstall_tsf() -> bool {
    crate::tsf::reinstall_tsf().is_ok()
}

use crate::config::{normalize_injection_mode, AppConfig, AppProfile};
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
    /// True when the user temporarily enabled Vietnamese for one excluded app.
    /// It survives focus changes to the tray/shell, but resets when a different
    /// normal app becomes active.
    pub manual_override: bool,
    /// Excluded process that the temporary override applies to.
    pub override_process: Option<String>,
    /// Most recent excluded process, remembered through a tray/shell focus
    /// change so clicking the tray icon can still target the game/app.
    pub last_game_process: Option<String>,
    /// Once the user explicitly enabled temporary mode for an excluded app,
    /// the V/E hotkey may toggle it on and off until they leave that app.
    pub override_unlocked: bool,
    /// True when the foreground window's keyboard layout is not US-EN.
    pub non_us_layout: bool,
    /// True when the foreground window is ANSI-only and cannot render Unicode
    /// characters (for example the VBA IDE).
    pub non_unicode_window: bool,
    /// Per-app overrides resolved for the current foreground executable.
    pub active_profile: Option<AppProfile>,
    /// 0: Auto, 1: Unicode, 2: scan-code, 3: clipboard, 4: safe paced.
    pub injection_mode: u8,
    /// Raw characters typed while Vietnamese is disabled but macros are on.
    pub macro_buffer: String,
    pub tray_hwnd: HWND,
    pub hook_handle: HHOOK,
    pub mouse_hook_handle: HHOOK,
    pub win_event_hook: HWINEVENTHOOK,
}

unsafe impl Send for HookState {}
unsafe impl Sync for HookState {}

impl HookState {
    #[inline]
    pub fn effective_enabled(&self) -> bool {
        (self.user_enabled || self.manual_override)
            && (!self.is_foreground_excluded || self.manual_override)
            && !(self.config.disable_on_non_us_layout && self.non_us_layout)
            && !(self.config.disable_on_non_unicode_window && self.non_unicode_window && self.config.charset == 0)
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

    /// Keep or clear the temporary override when focus moves. Tray/shell
    /// windows are ignored so clicking the tray icon does not lose the target
    /// game; a different real app ends the override.
    fn update_override_for_foreground(
        &mut self,
        proc_name: Option<&str>,
        excluded: bool,
        shell: bool,
    ) {
        let same_override = excluded
            && self.override_process.as_deref().is_some()
            && self.override_process.as_deref() == proc_name;

        if excluded {
            let same_game = self.last_game_process.as_deref() == proc_name;
            if !same_game {
                self.override_unlocked = false;
            }
            self.last_game_process = proc_name.map(str::to_string);
            if self.manual_override && self.override_process.is_none() {
                self.override_process = proc_name.map(str::to_string);
            } else if self.manual_override && !same_override {
                self.manual_override = false;
                self.override_process = None;
                self.override_unlocked = false;
            }
        } else if !shell {
            // A real non-excluded app means the temporary game override is over.
            self.manual_override = false;
            self.override_process = None;
            self.last_game_process = None;
            self.override_unlocked = false;
        }
    }

    /// Toggle the temporary override for the last excluded app. The first
    /// toggle enables Vietnamese, the second returns to auto-disable.
    pub fn toggle_temporary_override(&mut self) -> bool {
        if self.manual_override {
            self.manual_override = false;
            self.override_process = None;
        } else if let Some(game) = self.last_game_process.clone() {
            self.manual_override = true;
            self.override_process = Some(game);
            self.override_unlocked = true;
        } else {
            return false;
        }
        self.engine.reset();
        self.macro_buffer.clear();
        true
    }

    /// Toggle the normal global V/E mode.
    pub fn toggle_language(&mut self) {
        self.user_enabled = !self.user_enabled;
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
        self.injection_mode = normalize_injection_mode(
            self.active_profile
                .as_ref()
                .and_then(|profile| profile.injection_mode)
                .unwrap_or(self.config.injection_mode),
        );
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
        // Legacy core field; clipboard injection has been removed.
        use_clipboard_for_unicode: false,
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
unsafe fn is_shell_window(hwnd: HWND) -> bool {
    let mut buffer = [0u16; 64];
    let len = GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32);
    if len <= 0 {
        return false;
    }
    let class = String::from_utf16_lossy(&buffer[..len as usize]);
    matches!(
        class.as_str(),
        "Shell_TrayWnd"
            | "Shell_SecondaryTrayWnd"
            | "NotifyIconOverflowWindow"
            | "TaskListThumbnailWnd"
            | "DV2ControlHost"
            | "TopLevelWindowForOverflowXamlIsland"
            | "XamlExplorerHostIslandWindow"
            | "VietFlowMessageWindowClass"
    )
}

unsafe fn is_non_unicode_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    IsWindowUnicode(hwnd) == 0
}

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
        // The keyboard layout or window Unicode mode can change without a
        // foreground event. Only query the guards the user enabled.
        let non_us = state.config.disable_on_non_us_layout && is_non_us_layout(foreground);
        let non_unicode =
            state.config.disable_on_non_unicode_window && is_non_unicode_window(foreground);
        if non_us != state.non_us_layout || non_unicode != state.non_unicode_window {
            state.non_us_layout = non_us;
            state.non_unicode_window = non_unicode;
            state.apply_runtime_profile();
            let tray_hwnd = state.tray_hwnd;
            let effective = state.effective_enabled();
            drop(lock);
            PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
        }
        return;
    }

    let old_excluded = state.is_foreground_excluded;
    let old_non_us = state.non_us_layout;
    let old_non_unicode = state.non_unicode_window;
    let old_effective = state.effective_enabled();

    let proc_name = get_process_name_for_window(foreground);
    let excluded = is_app_excluded(proc_name.as_deref(), &state.config.excluded_apps);
    let shell = is_shell_window(foreground);

    state.last_foreground = foreground;
    state.macro_buffer.clear();
    state.update_override_for_foreground(proc_name.as_deref(), excluded, shell);

    state.is_foreground_excluded = excluded;
    state.non_us_layout = state.config.disable_on_non_us_layout && is_non_us_layout(foreground);
    state.non_unicode_window =
        state.config.disable_on_non_unicode_window && is_non_unicode_window(foreground);
    state.active_profile =
        profile_for_app(proc_name.as_deref(), &state.config.app_profiles).cloned();
    state.apply_runtime_profile();

    let effective = state.effective_enabled();
    if old_excluded != state.is_foreground_excluded
        || old_non_us != state.non_us_layout
        || old_non_unicode != state.non_unicode_window
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
    let non_unicode = config.disable_on_non_unicode_window
        && !foreground.is_null()
        && is_non_unicode_window(foreground);
    let excluded = is_app_excluded(proc_name.as_deref(), &config.excluded_apps);
    let shell = !foreground.is_null() && is_shell_window(foreground);
    let start_with_windows = config.start_with_windows;
    let startup_changed = state.config.start_with_windows != start_with_windows;

    state.update_override_for_foreground(proc_name.as_deref(), excluded, shell);

    state.user_enabled = config.enabled;
    state.is_foreground_excluded = excluded;
    state.non_us_layout = non_us;
    state.non_unicode_window = non_unicode;
    state.active_profile = active_profile;
    state.config = config;
    state.apply_runtime_profile();
    state.load_macros();

    let tray_hwnd = state.tray_hwnd;
    let effective = state.effective_enabled();
    crate::tsf::sync_tsf_config(&state.config, effective);
    let use_tsf = state.config.use_tsf && crate::tsf::is_tsf_registered();
    if use_tsf {
        crate::tsf::set_tsf_profile_active(true);
        // In TSF mode: release all global low-level hooks so the daemon runs at minimum footprint
        // without intercepting or delaying any typing keystrokes.
        if !state.hook_handle.is_null() {
            UnhookWindowsHookEx(state.hook_handle);
            state.hook_handle = null_mut();
        }
        if !state.mouse_hook_handle.is_null() {
            UnhookWindowsHookEx(state.mouse_hook_handle);
            state.mouse_hook_handle = null_mut();
        }
        if !state.win_event_hook.is_null() {
            UnhookWinEvent(state.win_event_hook);
            state.win_event_hook = null_mut();
        }
    } else {
        crate::tsf::set_tsf_profile_active(false);
        // Non-TSF hook mode: restore low-level hooks if not currently active
        if state.hook_handle.is_null() {
            state.hook_handle = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(low_level_keyboard_proc),
                0 as HMODULE,
                0,
            );
        }
        if state.mouse_hook_handle.is_null() {
            state.mouse_hook_handle = SetWindowsHookExW(
                WH_MOUSE_LL,
                Some(low_level_mouse_proc),
                0 as HMODULE,
                0,
            );
        }
        if state.win_event_hook.is_null() {
            state.win_event_hook = SetWinEventHook(
                EVENT_SYSTEM_FOREGROUND,
                EVENT_SYSTEM_FOREGROUND,
                null_mut(),
                Some(win_event_proc),
                0,
                0,
                WINEVENT_OUTOFCONTEXT,
            );
        }
    }
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
    let initial_non_unicode =
        config.disable_on_non_unicode_window && is_non_unicode_window(initial_foreground);
    let initial_profile = profile_for_app(initial_proc.as_deref(), &config.app_profiles).cloned();
    let options = engine_options_from_config(&config, initial_profile.as_ref());
    let charset = charset_from_config(&config, initial_profile.as_ref());
    let injection_mode = normalize_injection_mode(
        initial_profile
            .as_ref()
            .and_then(|profile| profile.injection_mode)
            .unwrap_or(config.injection_mode),
    );

    let mut engine = VietFlowEngine::new(options, charset);
    engine.macros.clear();
    for (k, v) in &config.macros {
        engine.macros.insert(k, v);
    }

    let use_tsf = config.use_tsf && crate::tsf::is_tsf_registered();

    // In TSF mode: run daemon at minimum footprint with no keyboard/mouse hooks installed.
    let (hook_handle, mouse_hook_handle, win_event_hook) = if use_tsf {
        (null_mut(), null_mut(), null_mut())
    } else {
        let hk = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(low_level_keyboard_proc),
            0 as HMODULE,
            0,
        );
        let hm = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(low_level_mouse_proc),
            0 as HMODULE,
            0,
        );
        let hw = SetWinEventHook(
            EVENT_SYSTEM_FOREGROUND,
            EVENT_SYSTEM_FOREGROUND,
            null_mut(),
            Some(win_event_proc),
            0,
            0,
            WINEVENT_OUTOFCONTEXT,
        );
        (hk, hm, hw)
    };

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
        override_process: None,
        last_game_process: if initial_excluded { initial_proc.clone() } else { None },
        override_unlocked: false,
        non_us_layout: initial_non_us,
        non_unicode_window: initial_non_unicode,
        active_profile: initial_profile,
        injection_mode,
        macro_buffer: String::new(),
        tray_hwnd,
        hook_handle,
        mouse_hook_handle,
        win_event_hook,
    });
    drop(lock);

    if use_tsf && crate::tsf::is_tsf_registered() {
        crate::tsf::set_tsf_profile_active(true);
    }
}

pub unsafe fn uninstall_hook() {
    crate::tsf::set_tsf_profile_active(false);
    let mut lock = STATE.lock().unwrap();
    if let Some(state) = lock.take() {
        if !state.win_event_hook.is_null() {
            UnhookWinEvent(state.win_event_hook);
        }
        if !state.hook_handle.is_null() {
            UnhookWindowsHookEx(state.hook_handle);
        }
        if !state.mouse_hook_handle.is_null() {
            UnhookWindowsHookEx(state.mouse_hook_handle);
        }
    }
}

pub const WH_MOUSE_LL: i32 = 14;
pub const WM_LBUTTONDOWN: u32 = 0x0201;
pub const WM_RBUTTONDOWN: u32 = 0x0204;
pub const WM_MBUTTONDOWN: u32 = 0x0207;

pub unsafe extern "system" fn low_level_mouse_proc(
    n_code: i32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    if n_code >= 0 {
        let msg = w_param as u32;
        if msg == WM_LBUTTONDOWN || msg == WM_RBUTTONDOWN || msg == WM_MBUTTONDOWN {
            if let Ok(mut lock) = STATE.try_lock() {
                if let Some(state) = lock.as_mut() {
                    state.engine.reset();
                    state.macro_buffer.clear();
                }
            }
        }
    }
    CallNextHookEx(null_mut(), n_code, w_param, l_param)
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

    // Fast-path: In TSF mode, immediately pass all input through with zero delay
    if crate::tsf::is_tsf_registered() {
        if let Ok(lock) = STATE.try_lock() {
            if let Some(state) = lock.as_ref() {
                if state.config.use_tsf {
                    return CallNextHookEx(null_mut(), n_code, w_param, l_param);
                }
            }
        }
    }

    let foreground = GetForegroundWindow();

    let mut lock = match STATE.try_lock() {
        Ok(l) => l,
        Err(_) => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

    let state = match lock.as_mut() {
        Some(s) => s,
        None => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
    };

    // Only inspect foreground app if the window focus actually shifted
    if !foreground.is_null() && foreground != state.last_foreground {
        drop(lock);
        check_foreground_change(foreground);
        lock = match STATE.try_lock() {
            Ok(l) => l,
            Err(_) => return CallNextHookEx(null_mut(), n_code, w_param, l_param),
        };
        if lock.is_none() {
            return CallNextHookEx(null_mut(), n_code, w_param, l_param);
        }
    }

    let state = lock.as_mut().unwrap();

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

    let hotkeys_locked = state.hotkeys_locked();
    let language_allowed =
        language_hotkey_allowed(hotkeys_locked, state.is_foreground_excluded, state.override_unlocked);
    if is_key_down && language_allowed {
        let ctrl = state.ctrl_pressed || is_vk_down(VK_CONTROL);
        let alt = state.alt_pressed || is_vk_down(VK_MENU);
        let shift = state.shift_pressed || is_vk_down(VK_SHIFT);
        let win = state.win_pressed || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);

        if !hotkeys_locked {
            // Configurable global hotkeys (defaults: Ctrl+Shift+F5 / F9).
            if check_custom_hotkey(&state.config.hotkey_settings, ctrl, alt, shift, win, vk) {
                PostMessageW(state.tray_hwnd, WM_SETTINGS, 0, 0);
                return 1;
            }
            if check_custom_hotkey(&state.config.hotkey_converter, ctrl, alt, shift, win, vk) {
                PostMessageW(state.tray_hwnd, WM_CONVERT, 0, 0);
                return 1;
            }
        }

        // Language toggle check
        let is_toggle = match state.config.hotkey_mode.as_str() {
            "CtrlShift" => {
                (ctrl && (vk == VK_LSHIFT || vk == VK_RSHIFT))
                    || (shift && (vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL))
            }
            "AltZ" => alt && (vk == b'Z' as u16),
            "CtrlSpace" => ctrl && vk == VK_SPACE,
            "ShiftSpace" => shift && vk == VK_SPACE,
            "AltShift" => {
                (alt && (vk == VK_LSHIFT || vk == VK_RSHIFT))
                    || (shift && (vk == VK_LMENU || vk == VK_RMENU || vk == VK_MENU))
            }
            "CapsLock" => vk == VK_CAPITAL,
            "WinSpace" => win && vk == VK_SPACE,
            other => check_custom_hotkey(other, ctrl, alt, shift, win, vk),
        };

        if is_toggle {
            if state.is_foreground_excluded {
                state.toggle_temporary_override();
            } else {
                state.toggle_language();
                state.config.save();
            }

            if state.config.notification_sound {
                play_notification_sound();
            }

            let tray_hwnd = state.tray_hwnd;
            let effective = state.effective_enabled();
            crate::tsf::sync_tsf_config(&state.config, effective);
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

    // In TSF mode: if TSF is enabled in config and registered in Windows,
    // yield typing input to the in-process TSF TIP (which composes directly without backspaces).
    // Hotkeys (Ctrl+Shift, Alt+Z, Win+Space, etc.) were already intercepted above.
    if state.config.use_tsf && crate::tsf::is_tsf_registered() {
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
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

    // Reset keys: Esc, Enter, Tab, Navigation (PageUp, PageDown, End, Home, Arrows), Delete
    if is_key_down && matches!(vk, VK_ESCAPE | VK_RETURN | VK_TAB | 0x21..=0x28 | 0x2E) {
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
    if let Some(replacement) = state.engine.macros.get(&trigger) {
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

fn language_hotkey_allowed(locked: bool, excluded: bool, unlocked: bool) -> bool {
    !locked || (excluded && unlocked)
}

fn compact_eq_ignore_ascii_case(input: &str, expected: &str) -> bool {
    let mut chars = input.chars().filter(|c| !c.is_whitespace());
    for expected_char in expected.chars() {
        match chars.next() {
            Some(c) if c.eq_ignore_ascii_case(&expected_char) => {}
            _ => return false,
        }
    }
    chars.next().is_none()
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

    // Direct normalized preset checks without allocating a compacted String on
    // every keydown (this function runs in the low-level hook).
    if compact_eq_ignore_ascii_case(s, "CtrlShift") {
        return (ctrl || is_vk_down(VK_CONTROL))
            && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT);
    }
    if compact_eq_ignore_ascii_case(s, "AltZ") {
        return (alt || is_vk_down(VK_MENU)) && (vk == b'Z' as u16);
    }
    if compact_eq_ignore_ascii_case(s, "CtrlSpace") {
        return (ctrl || is_vk_down(VK_CONTROL)) && vk == VK_SPACE;
    }
    if compact_eq_ignore_ascii_case(s, "ShiftSpace") {
        return (shift || is_vk_down(VK_SHIFT)) && vk == VK_SPACE;
    }
    if compact_eq_ignore_ascii_case(s, "AltShift") {
        return (alt || is_vk_down(VK_MENU))
            && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT);
    }
    if compact_eq_ignore_ascii_case(s, "CapsLock") || compact_eq_ignore_ascii_case(s, "Caps") {
        return vk == VK_CAPITAL;
    }
    if compact_eq_ignore_ascii_case(s, "WinSpace") {
        return (win || is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN)) && vk == VK_SPACE;
    }

    let mut req_ctrl = false;
    let mut req_alt = false;
    let mut req_shift = false;
    let mut req_win = false;
    let mut target_key = "";

    for part in s.split('+').map(|p| p.trim()) {
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

    #[test]
    fn embedded_notification_is_compressed_wav() {
        assert_eq!(&NOTIFICATION_SOUND[0..4], b"RIFF");
        assert_eq!(&NOTIFICATION_SOUND[8..12], b"WAVE");
        let format = u16::from_le_bytes([NOTIFICATION_SOUND[20], NOTIFICATION_SOUND[21]]);
        assert_eq!(format, 0x0011, "expected IMA ADPCM");
        assert!(
            NOTIFICATION_SOUND.len() < 3000,
            "keep the notification chime short"
        );
    }

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
            override_process: None,
            last_game_process: None,
            override_unlocked: false,
            non_us_layout: false,
            non_unicode_window: false,
            active_profile: None,
            injection_mode: 0,
            macro_buffer: String::new(),
            tray_hwnd: std::ptr::null_mut(),
            hook_handle: std::ptr::null_mut(),
            mouse_hook_handle: std::ptr::null_mut(),
            win_event_hook: std::ptr::null_mut(),
        }
    }

    #[test]
    fn manual_override_allows_typing_in_excluded_app() {
        assert!(!hook_state(true, false, true).effective_enabled());
        assert!(hook_state(true, true, true).effective_enabled());
        assert!(hook_state(true, true, false).effective_enabled());
        assert!(hook_state(false, false, true).effective_enabled());
        assert!(!hook_state(false, false, false).effective_enabled());
    }

    #[test]
    fn non_unicode_window_can_disable_typing() {
        let mut state = hook_state(false, false, true);
        state.non_unicode_window = true;
        assert!(!state.effective_enabled());
        state.config.disable_on_non_unicode_window = false;
        assert!(state.effective_enabled());
    }

    #[test]
    fn legacy_clipboard_mode_migrates_to_auto() {
        let mut state = hook_state(false, false, true);
        state.config.injection_mode = 3;
        state.apply_runtime_profile();
        assert_eq!(state.injection_mode, 0);

        state.config.injection_mode = 4;
        state.apply_runtime_profile();
        assert_eq!(state.injection_mode, 4);
    }

    #[test]
    fn language_hotkey_needs_unlock_inside_excluded_app() {
        assert!(!language_hotkey_allowed(true, true, false));
        assert!(language_hotkey_allowed(true, true, true));
        assert!(language_hotkey_allowed(false, true, false));
        assert!(!language_hotkey_allowed(true, false, true));
    }

    #[test]
    fn temporary_override_survives_shell_focus_but_not_another_app() {
        let mut state = hook_state(true, false, false);
        state.last_game_process = Some("game.exe".to_string());
        assert!(state.toggle_temporary_override());
        assert!(state.effective_enabled());

        state.update_override_for_foreground(Some("explorer.exe"), false, true);
        assert!(state.manual_override);
        assert!(state.last_game_process.is_some());

        state.update_override_for_foreground(Some("game.exe"), true, false);
        assert!(state.manual_override);

        state.update_override_for_foreground(Some("chrome.exe"), false, false);
        assert!(!state.manual_override);
        assert!(state.override_process.is_none());
        assert!(state.last_game_process.is_none());
        assert!(!state.override_unlocked);
    }

    #[test]
    fn temporary_override_works_for_last_excluded_app() {
        let mut state = hook_state(true, false, true);
        state.last_game_process = Some("game.exe".to_string());
        assert!(state.toggle_temporary_override());
        assert!(state.manual_override);
        assert!(state.override_unlocked);
        assert_eq!(state.override_process.as_deref(), Some("game.exe"));
        assert!(state.effective_enabled());

        assert!(state.toggle_temporary_override());
        assert!(!state.manual_override);
        assert!(state.override_unlocked);
        assert!(!state.effective_enabled());

        // Once unlocked, V/E can turn the temporary mode back on.
        assert!(state.toggle_temporary_override());
        assert!(state.manual_override);
        assert!(state.effective_enabled());

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
        state.config.enable_macro = true;
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
        state.config.enable_macro = true;
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
