// VietFlow Windows - Low-Level Keyboard Hook (WH_KEYBOARD_LL) & App Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::collections::HashMap;
use std::ptr::null_mut;
use std::sync::{mpsc, Mutex, OnceLock};
use vietflow_core::models::{CharsetId, EngineOptions, InputMethod};
use vietflow_core::VietFlowEngine;
use windows_sys::Win32::Foundation::{HMODULE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    GetAsyncKeyState, GetKeyState, GetKeyboardLayout, MapVirtualKeyExW, SendInput, ToUnicodeEx, INPUT, VK_BACK,
    VK_CAPITAL, VK_CONTROL, VK_DOWN, VK_ESCAPE, VK_F1, VK_LCONTROL, VK_LEFT, VK_LMENU, VK_LSHIFT, VK_LWIN, VK_MENU,
    VK_OEM_1, VK_OEM_2, VK_OEM_3, VK_OEM_4, VK_OEM_5, VK_OEM_6, VK_OEM_7, VK_OEM_COMMA, VK_OEM_MINUS,
    VK_OEM_PERIOD, VK_OEM_PLUS, VK_RCONTROL, VK_RETURN, VK_RIGHT, VK_RMENU, VK_RSHIFT, VK_RWIN, VK_SHIFT,
    VK_SPACE, VK_TAB, VK_UP,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, GetClassNameW, GetForegroundWindow, GetWindowThreadProcessId,
    IsWindowUnicode, PostMessageW, SetWindowsHookExW, UnhookWindowsHookEx, HHOOK,
    KBDLLHOOKSTRUCT, LLKHF_INJECTED, WH_KEYBOARD_LL, WM_KEYDOWN, WM_KEYUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

#[allow(clippy::upper_case_acronyms)]
pub type HWINEVENTHOOK = *mut std::ffi::c_void;
pub const EVENT_SYSTEM_FOREGROUND: u32 = 0x0003;
pub const WINEVENT_OUTOFCONTEXT: u32 = 0x0000;

#[allow(clippy::upper_case_acronyms)]
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
    fn GetTickCount64() -> u64;
}

#[allow(dead_code)]
pub fn get_tick_count_64() -> u64 {
    unsafe { GetTickCount64() }
}

/// A short, clean uncompressed 16-bit 48kHz PCM notification chime.
/// Plays instantaneously with zero audio distortion, aliasing, or clicks
/// across all Windows versions and Bluetooth LE / A2DP audio endpoints.
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
    const MB_ICONASTERISK: u32 = 0x0000_0040;

    // Primary path: the embedded 16-bit 48kHz PCM chime.
    // Uncompressed PCM requires no external ACM codecs and maps 1:1
    // to Windows WASAPI and Bluetooth LE audio sample rates.
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

    // Gentle fallback: standard system asterisk beep if audio device is unavailable.
    // Never invoke raw hardware buzzer.
    unsafe {
        MessageBeep(MB_ICONASTERISK);
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
use crate::process::{get_process_name_for_window, is_app_excluded, is_auto_excluded_app, profile_for_app};
use crate::startup::set_start_with_windows;
use crate::tray::{WM_SETTINGS, WM_TRAYICON};

#[derive(Debug, Clone, Copy)]
pub struct GameDetectionTracker {
    pub recent_keys: [u16; 8],
    pub recent_timestamps: [u64; 8],
    pub index: usize,
    pub count: usize,
    pub key_down_time: [u64; 256],
}

impl GameDetectionTracker {
    pub const fn new() -> Self {
        Self {
            recent_keys: [0; 8],
            recent_timestamps: [0; 8],
            index: 0,
            count: 0,
            key_down_time: [0; 256],
        }
    }

    pub fn record_key(
        &mut self,
        vk: u16,
        now_ms: u64,
        held_vks: &[bool; 256],
    ) -> bool {
        let movement_vks = [0x57u16, 0x41, 0x53, 0x44]; // W, A, S, D
        let is_current_movement = movement_vks.contains(&vk);
        let is_current_aux = vk == VK_SPACE || vk == VK_SHIFT || vk == VK_LSHIFT || vk == VK_RSHIFT;

        // Signal 1: Sustained Movement Chording (Holding a movement key continuously for >= 500ms
        // while pressing Jump/Sprint or another movement key).
        // In typing, keys are tapped and released within 30-100ms. A continuous 500ms hold is definitive gaming.
        if is_current_movement || is_current_aux {
            for &m_vk in &movement_vks {
                if m_vk != vk && held_vks[m_vk as usize] {
                    let down_t = self.key_down_time[m_vk as usize];
                    if down_t > 0 && now_ms.saturating_sub(down_t) >= 500 {
                        self.reset();
                        return true;
                    }
                }
            }
        }

        // Record to sequence ring buffer
        self.recent_keys[self.index] = vk;
        self.recent_timestamps[self.index] = now_ms;
        self.index = (self.index + 1) % 8;
        if self.count < 8 {
            self.count += 1;
        }

        // Signal 2: Rapid Arrow key sequence (>= 3 arrow keys within 1200ms)
        // Arrow keys are NEVER typing letters in Vietnamese Telex/VNI.
        let is_arrow = matches!(vk, VK_LEFT | VK_UP | VK_RIGHT | VK_DOWN);
        if is_arrow && self.count >= 3 {
            let mut arrow_count = 0;
            for i in 0..self.count.min(5) {
                let idx = (self.index + 8 - 1 - i) % 8;
                let k = self.recent_keys[idx];
                let ts = self.recent_timestamps[idx];
                if now_ms.saturating_sub(ts) > 1200 {
                    break;
                }
                if matches!(k, VK_LEFT | VK_UP | VK_RIGHT | VK_DOWN) {
                    arrow_count += 1;
                }
            }
            if arrow_count >= 3 {
                self.reset();
                return true;
            }
        }

        false
    }

    pub fn reset(&mut self) {
        self.count = 0;
        self.index = 0;
    }
}

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
    /// Currently focused process name for dynamic per-app mode tracking.
    pub current_process: Option<String>,
    /// Memory map of per-app mode settings (lowercase process name -> is_vietnamese).
    pub per_app_modes: HashMap<String, bool>,
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
    /// Keystrokes buffered in English mode for automatic smart language switching.
    pub english_word_buffer: String,
    /// Whether the current English word started after a valid boundary (whitespace, quotes, opening bracket).
    pub english_word_valid: bool,
    /// Timestamp (GetTickCount64) until which AutoSwitch is paused after manual hotkey toggle.
    pub auto_pause_until_ms: u64,
    pub tray_hwnd: HWND,
    pub hook_handle: HHOOK,
    pub mouse_hook_handle: HHOOK,
    pub win_event_hook: HWINEVENTHOOK,
    pub tsf_registered: bool,
    pub held_vks: [bool; 256],
    pub foreground_pid: u32,
    pub game_detector: GameDetectionTracker,
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
        self.engine.reset_word();
        self.english_word_buffer.clear();
        self.english_word_valid = true;
        self.macro_buffer.clear();
        true
    }

    /// Toggle the normal global V/E mode.
    pub fn toggle_language(&mut self) {
        self.user_enabled = !self.user_enabled;
        self.config.enabled = self.user_enabled;
        if self.config.remember_per_app_mode && !self.is_foreground_excluded {
            if let Some(proc) = &self.current_process {
                self.per_app_modes.insert(proc.to_lowercase(), self.user_enabled);
                self.config.per_app_modes = self.per_app_modes.clone();
            }
        }
        self.engine.reset_word();
        self.english_word_buffer.clear();
        self.english_word_valid = true;
        self.macro_buffer.clear();
    }

    #[inline]
    pub fn send_engine_output(&mut self, backspaces: usize, text: &str) {
        if backspaces > 0 || !text.is_empty() {
            let text = crate::clipboard::expand_clipboard(text);
            send_atomic_batch_with_mode(backspaces, text.as_ref(), self.injection_mode);
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
        self.engine.reset_word();
        self.english_word_buffer.clear();
        self.english_word_valid = true;
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
        auto_switch_by_typing: config.auto_switch_by_typing,
        fix_shift_lag: config.fix_shift_lag,
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
    let mut excluded = is_app_excluded(
        proc_name.as_deref(),
        &state.config.excluded_apps,
        state.config.smart_prefix_matching,
    );
    if !excluded && state.config.auto_exclude_apps {
        if let Some(proc) = proc_name.as_deref() {
            if is_auto_excluded_app(proc) {
                excluded = true;
            }
        }
    }
    let shell = is_shell_window(foreground);

    let mut current_pid = 0u32;
    GetWindowThreadProcessId(foreground, &mut current_pid);
    state.foreground_pid = current_pid;

    state.last_foreground = foreground;
    state.engine.reset();
    state.macro_buffer.clear();
    state.english_word_buffer.clear();
    state.english_word_valid = true;
    state.update_override_for_foreground(proc_name.as_deref(), excluded, shell);

    state.is_foreground_excluded = excluded;
    if !shell && proc_name.is_some() {
        state.current_process = proc_name.clone();
        if state.config.remember_per_app_mode && !excluded {
            if let Some(proc) = &proc_name {
                let key = proc.to_lowercase();
                if let Some(&saved_mode) = state.per_app_modes.get(&key) {
                    state.user_enabled = saved_mode;
                } else {
                    state.per_app_modes.insert(key, state.user_enabled);
                }
            }
        }
    }
    state.non_us_layout = state.config.disable_on_non_us_layout && is_non_us_layout(foreground);
    state.non_unicode_window =
        state.config.disable_on_non_unicode_window && is_non_unicode_window(foreground);
    state.active_profile = profile_for_app(
        proc_name.as_deref(),
        &state.config.app_profiles,
        state.config.smart_prefix_matching,
    )
    .cloned();
    state.apply_runtime_profile();

    let effective = state.effective_enabled();
    crate::tsf::sync_tsf_config(&state.config, effective);
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
    let active_profile =
        profile_for_app(proc_name.as_deref(), &config.app_profiles, config.smart_prefix_matching).cloned();
    let non_us = config.disable_on_non_us_layout
        && !foreground.is_null()
        && is_non_us_layout(foreground);
    let non_unicode = config.disable_on_non_unicode_window
        && !foreground.is_null()
        && is_non_unicode_window(foreground);
    let excluded = is_app_excluded(
        proc_name.as_deref(),
        &config.excluded_apps,
        config.smart_prefix_matching,
    );
    let shell = !foreground.is_null() && is_shell_window(foreground);
    let start_with_windows = config.start_with_windows;
    let startup_changed = state.config.start_with_windows != start_with_windows;

    state.update_override_for_foreground(proc_name.as_deref(), excluded, shell);

    state.user_enabled = config.enabled;
    state.is_foreground_excluded = excluded;
    state.non_us_layout = non_us;
    state.non_unicode_window = non_unicode;
    state.active_profile = active_profile;
    for (k, v) in &config.per_app_modes {
        state.per_app_modes.entry(k.clone()).or_insert(*v);
    }
    state.config = config;
    state.apply_runtime_profile();
    state.load_macros();

    let tray_hwnd = state.tray_hwnd;
    let effective = state.effective_enabled();
    state.tsf_registered = crate::tsf::is_tsf_mode_active();
    crate::tsf::sync_tsf_config(&state.config, effective);
    drop(lock);

    // The UI only writes the JSON file; apply the registry side-effect here so
    // "Khởi động cùng Windows" takes effect without restarting the daemon.
    if startup_changed {
        set_start_with_windows(start_with_windows);
    }

    PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
}

pub unsafe fn install_hook(tray_hwnd: HWND, config: AppConfig) {
    let initial_foreground = GetForegroundWindow();
    let initial_proc = get_process_name_for_window(initial_foreground);
    let initial_excluded = is_app_excluded(
        initial_proc.as_deref(),
        &config.excluded_apps,
        config.smart_prefix_matching,
    );
    let mut user_enabled = config.enabled;
    let mut per_app_modes = config.per_app_modes.clone();
    if config.remember_per_app_mode {
        if let Some(proc) = &initial_proc {
            let key = proc.to_lowercase();
            if let Some(&saved_mode) = per_app_modes.get(&key) {
                user_enabled = saved_mode;
            } else if !initial_excluded {
                per_app_modes.insert(key, user_enabled);
            }
        }
    }
    let initial_non_us = config.disable_on_non_us_layout && is_non_us_layout(initial_foreground);
    let initial_non_unicode =
        config.disable_on_non_unicode_window && is_non_unicode_window(initial_foreground);
    let initial_profile = profile_for_app(
        initial_proc.as_deref(),
        &config.app_profiles,
        config.smart_prefix_matching,
    )
    .cloned();
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

    // Keep the low-level hooks installed in TSF mode too: they are still needed
    // for V/E hotkeys, foreground exclusion/override, and settings/converter
    // hotkeys. The hook callback simply yields typing to the TSF TIP.
    let hook_handle = SetWindowsHookExW(
        WH_KEYBOARD_LL,
        Some(low_level_keyboard_proc),
        0 as HMODULE,
        0,
    );
    let mouse_hook_handle = SetWindowsHookExW(
        WH_MOUSE_LL,
        Some(low_level_mouse_proc),
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

    let mut initial_pid = 0u32;
    if !initial_foreground.is_null() {
        GetWindowThreadProcessId(initial_foreground, &mut initial_pid);
    }

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
        current_process: initial_proc.clone(),
        per_app_modes,
        override_unlocked: false,
        non_us_layout: initial_non_us,
        non_unicode_window: initial_non_unicode,
        active_profile: initial_profile,
        injection_mode,
        macro_buffer: String::new(),
        english_word_buffer: String::new(),
        english_word_valid: true,
        auto_pause_until_ms: 0,
        tray_hwnd,
        hook_handle,
        mouse_hook_handle,
        win_event_hook,
        tsf_registered: crate::tsf::is_tsf_mode_active(),
        held_vks: [false; 256],
        foreground_pid: initial_pid,
        game_detector: GameDetectionTracker::new(),
    });
    drop(lock);
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
                    state.english_word_buffer.clear();
                    state.english_word_valid = true;
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

#[inline]
pub fn is_whitespace_key(vk: u16) -> bool {
    matches!(vk, VK_SPACE | VK_RETURN | VK_TAB)
}

#[inline]
pub fn is_word_start_key(vk: u16, shift: bool) -> bool {
    is_whitespace_key(vk)
        || (shift && vk == 0x39) // '(' (Shift + 9)
        || (!shift && vk == VK_OEM_4) // '['
        || (shift && vk == VK_OEM_4) // '{'
        || vk == VK_OEM_7 // '\'' or '"'
        || (!shift && vk == VK_OEM_3) // '`'
        || (shift && vk == VK_OEM_COMMA) // '<'
}

#[inline]
pub fn is_word_break_key(vk: u16, shift: bool) -> bool {
    matches!(vk, VK_SPACE | VK_RETURN | VK_TAB)
        || (!shift && vk == VK_OEM_COMMA) // ','
        || (!shift && vk == VK_OEM_PERIOD) // '.'
        || (!shift && vk == VK_OEM_1) // ';'
        || (shift && vk == VK_OEM_1) // ':'
        || (shift && vk == VK_OEM_2) // '?'
        || (shift && vk == 0x31) // '!'
        || vk == VK_OEM_7 // '\'' or '"'
        || (shift && vk == 0x30) // ')'
        || (!shift && vk == VK_OEM_6) // ']'
        || (shift && vk == VK_OEM_6) // '}'
        || (shift && vk == VK_OEM_PERIOD) // '>'
}

#[inline]
pub fn is_identifier_separator(vk: u16, shift: bool) -> bool {
    (shift && vk == VK_OEM_MINUS) // '_'
        || (!shift && vk == VK_OEM_MINUS) // '-'
        || (!shift && vk == VK_OEM_2) // '/'
        || vk == VK_OEM_5 // '\' or '|'
        || (shift && vk == 0x32) // '@'
        || (shift && matches!(vk, 0x33..=0x38)) // '#', '$', '%', '^', '&', '*'
        || vk == VK_OEM_PLUS // '=' or '+'
        || (shift && vk == VK_OEM_3) // '~'
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
    let is_key_up = msg == WM_KEYUP || msg == WM_SYSKEYUP;

    let vk = kbd.vkCode as u16;

    if is_key_up && vk < 256 {
        state.held_vks[vk as usize] = false;
        state.game_detector.key_down_time[vk as usize] = 0;
    }
    if is_key_down && vk < 256 && !state.held_vks[vk as usize] {
        state.game_detector.key_down_time[vk as usize] = get_tick_count_64();
    }

    // Synchronize modifiers with physical keyboard state via GetAsyncKeyState.
    // This permanently eliminates sticky modifier flags caused by missed WM_KEYUP
    // events during UAC elevation, Win+L lock screen, or rapid Alt-Tab switches.
    state.ctrl_pressed = is_vk_down(VK_CONTROL) || is_vk_down(VK_LCONTROL) || is_vk_down(VK_RCONTROL);
    state.shift_pressed = is_vk_down(VK_SHIFT) || is_vk_down(VK_LSHIFT) || is_vk_down(VK_RSHIFT);
    state.alt_pressed = is_vk_down(VK_MENU) || is_vk_down(VK_LMENU) || is_vk_down(VK_RMENU);
    state.win_pressed = is_vk_down(VK_LWIN) || is_vk_down(VK_RWIN);

    let hotkeys_locked = state.hotkeys_locked();
    let language_allowed =
        language_hotkey_allowed(hotkeys_locked, state.is_foreground_excluded, state.override_unlocked);
    if is_key_down && language_allowed {
        let ctrl = state.ctrl_pressed;
        let alt = state.alt_pressed;
        let shift = state.shift_pressed;
        let win = state.win_pressed;

        if !hotkeys_locked {
            // Configurable global settings hotkey (default: Ctrl+Shift+F5).
            if check_custom_hotkey(&state.config.hotkey_settings, ctrl, alt, shift, win, vk) {
                PostMessageW(state.tray_hwnd, WM_SETTINGS, 0, 0);
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
            if state.tsf_registered {
                // In TSF mode, V/E toggling is locked to Windows language manager
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }

            if state.config.pause_auto_after_manual {
                let now_tick = get_tick_count_64();
                state.auto_pause_until_ms = now_tick + 5000;
            }

            if state.is_foreground_excluded {
                state.toggle_temporary_override();
            } else {
                state.toggle_language();
                state.config.save();
            }
            state.game_detector.reset();

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

    // When Vietnamese typing is off, handle English keystroke buffering for smart
    // AutoSwitch detection and macro expansion. This runs regardless of whether TSF
    // is registered because standard TSF TIP is dormant in English mode.
    if !state.effective_enabled() {
        if is_key_down {
            if vk == VK_BACK {
                state.english_word_buffer.pop();
                if state.english_word_buffer.is_empty() {
                    state.english_word_valid = true;
                }
                if state.macro_only() {
                    return handle_macro_only(state, vk, n_code, w_param, l_param);
                }
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }

            let ctrl_down = state.ctrl_pressed;
            let alt_down = state.alt_pressed;
            let win_down = state.win_pressed;

            if ctrl_down || alt_down || win_down || matches!(vk, VK_ESCAPE | 0x21..=0x28 | 0x2E) {
                state.english_word_buffer.clear();
                state.english_word_valid = false;
                if state.macro_only() {
                    return handle_macro_only(state, vk, n_code, w_param, l_param);
                }
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }

            // Identifier separator guard (D6): '.', '_', '-', '/', '\', '@', etc. invalidate word buffer
            if is_identifier_separator(vk, state.shift_pressed) {
                state.english_word_buffer.clear();
                state.english_word_valid = false;
                state.engine.push_english_word_lookback("");
                if state.macro_only() {
                    return handle_macro_only(state, vk, n_code, w_param, l_param);
                }
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }

            let input_char = vk_to_char_with_layout(vk, state.shift_pressed);
            let is_vni_digit = state.config.method == 1 && (0x30..=0x39).contains(&vk) && !state.shift_pressed;

            if !is_vni_digit && (0x30..=0x39).contains(&vk) && !state.shift_pressed {
                // Digits after a space begin a new word (e.g. 5trawm -> 5trăm, D33); glued to letters, an identifier
                state.english_word_valid = state.english_word_buffer.is_empty() && state.english_word_valid;
                state.english_word_buffer.clear();
                if state.macro_only() {
                    return handle_macro_only(state, vk, n_code, w_param, l_param);
                }
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }

            if input_char.is_ascii_alphabetic() || is_vni_digit {
                if state.english_word_valid {
                    if state.english_word_buffer.len() < 32 {
                        state.english_word_buffer.push(input_char);
                    } else {
                        state.english_word_buffer.clear();
                        state.english_word_valid = false;
                    }
                }
            } else if is_word_break_key(vk, state.shift_pressed) {
                let is_enter = vk == VK_RETURN;
                let break_char = if is_enter || vk == VK_TAB {
                    '\0'
                } else if input_char != '\0' {
                    input_char
                } else {
                    ' '
                };
                let now_tick = get_tick_count_64();
                let auto_switch_allowed = state.config.auto_switch_by_typing
                    && !state.is_foreground_excluded
                    && now_tick >= state.auto_pause_until_ms
                    && state.english_word_valid
                    && state.english_word_buffer.len() >= 2;

                if auto_switch_allowed {
                    let word = state.english_word_buffer.clone();
                    if let Some((bs, text)) = state.engine.check_english_word_at_break(&word, break_char) {
                        state.send_engine_output(bs, text.as_str());
                        if is_enter {
                            let mut inputs = [
                                crate::inject::create_vk_input(VK_RETURN, false),
                                crate::inject::create_vk_input(VK_RETURN, true),
                            ];
                            unsafe {
                                SendInput(2, inputs.as_mut_ptr(), std::mem::size_of::<INPUT>() as i32);
                            }
                        }
                        state.toggle_language();
                        let effective = state.effective_enabled();
                        crate::tsf::sync_tsf_config(&state.config, effective);
                        let tray_hwnd = state.tray_hwnd;
                        let play_sound = state.config.notification_sound;
                        state.english_word_buffer.clear();
                        state.english_word_valid = is_word_start_key(vk, state.shift_pressed);
                        drop(lock);
                        PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
                        if play_sound {
                            play_notification_sound();
                        }
                        return 1;
                    }
                }

                if vk == VK_SPACE && state.english_word_valid && !state.english_word_buffer.is_empty() {
                    let word = state.english_word_buffer.clone();
                    state.engine.push_english_word_lookback(&word);
                } else {
                    state.engine.push_english_word_lookback("");
                }
                state.english_word_buffer.clear();
                state.english_word_valid = is_word_start_key(vk, state.shift_pressed);
            } else {
                state.english_word_buffer.clear();
                state.english_word_valid = is_word_start_key(vk, state.shift_pressed);
            }

            if state.macro_only() {
                return handle_macro_only(state, vk, n_code, w_param, l_param);
            }
        }
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // If the foreground application is actively handled by the in-process TSF TIP,
    // yield typing keystrokes so the low-level hook never races with TSF.
    // If the application is NOT handled by TSF (e.g. console, games, non-TSF apps),
    // let the hook handle it seamlessly as fallback.
    let foreground_pid = state.foreground_pid;
    if state.tsf_registered && crate::tsf::is_tsf_active_for_pid(foreground_pid) {
        state.engine.reset();
        state.macro_buffer.clear();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Automatic Game Mode Detection: when repeated movement patterns (Arrow keys or held WASD)
    // are detected, switch mode to English [E] automatically to prevent gaming disruption.
    // CRITICAL: NEVER run game detection when the engine is actively composing a syllable!
    if is_key_down && state.config.auto_detect_game_mode && state.user_enabled {
        if state.engine.is_empty() {
            let now_ms = get_tick_count_64();
            if state.game_detector.record_key(vk, now_ms, &state.held_vks) {
                state.user_enabled = false;
                state.config.enabled = false;
                if let Some(proc) = &state.current_process {
                    state.per_app_modes.insert(proc.clone(), false);
                }
                state.engine.reset_word();
                state.macro_buffer.clear();
                state.english_word_buffer.clear();

                let effective = state.effective_enabled();
                crate::tsf::sync_tsf_config(&state.config, effective);

                let tray_hwnd = state.tray_hwnd;
                if !tray_hwnd.is_null() {
                    unsafe {
                        PostMessageW(tray_hwnd, WM_TRAYICON, 0, 0);
                    }
                }
                if state.config.notification_sound {
                    play_notification_sound();
                }
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }
        }
    }

    // Reset keys: Esc, Enter, Tab, Navigation (PageUp, PageDown, End, Home, Arrows), Delete
    if is_key_down && matches!(vk, VK_ESCAPE | VK_RETURN | VK_TAB | 0x21..=0x28 | 0x2E) {
        state.engine.reset();
        state.macro_buffer.clear();
        if vk == VK_RETURN {
            state.game_detector.reset();
        }
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

    let win_down = state.win_pressed;
    let ctrl_down = state.ctrl_pressed;
    let alt_down = state.alt_pressed;

    // If Ctrl, Alt or Win is pressed, pass key through without processing Vietnamese
    if ctrl_down || alt_down || win_down {
        state.engine.reset();
        state.macro_buffer.clear();
        return CallNextHookEx(null_mut(), n_code, w_param, l_param);
    }

    // Key auto-repeat: if this non-modifier key is already held down, pass repeated
    // WM_KEYDOWN events natively to Windows. This allows smooth continuous
    // movement in games (e.g. holding WASD to walk) and prevents repetitive accent loops.
    if is_key_down && vk != VK_BACK {
        let is_mod = matches!(
            vk,
            VK_LCONTROL | VK_RCONTROL | VK_CONTROL |
            VK_LSHIFT | VK_RSHIFT | VK_SHIFT |
            VK_LMENU | VK_RMENU | VK_MENU |
            VK_LWIN | VK_RWIN
        );
        if !is_mod {
            let is_physically_down = is_vk_down(vk);
            if !is_physically_down {
                state.held_vks[vk as usize] = false;
            } else if vk < 256 && state.held_vks[vk as usize] {
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
            }
            if vk < 256 {
                state.held_vks[vk as usize] = true;
            }
        }
    }

    // Process alphanumeric/symbol key
    if is_key_down {
        let input_char = vk_to_char_with_layout(vk, state.shift_pressed);
        if input_char != '\0' {
            let now_ms = unsafe { GetTickCount64() };
            let (backspaces, text) = state.engine.process_key_with_time(input_char, now_ms);

            let auto_switch_req = state.engine.take_auto_switch_request();

            let mut char_buf = [0u8; 4];
            let char_str = input_char.encode_utf8(&mut char_buf);
            let is_plain_passthrough = backspaces == 0 && text.as_str() == char_str;

            // Always handle language switch when requested by engine
            if let Some(target_lang) = auto_switch_req {
                if !target_lang && state.config.auto_switch_by_typing {
                    // Switch from Vietnamese to English
                    state.toggle_language();
                    let effective = state.effective_enabled();
                    crate::tsf::sync_tsf_config(&state.config, effective);
                    let tray_hwnd = state.tray_hwnd;
                    let play_sound = state.config.notification_sound;
                    PostMessageW(tray_hwnd, WM_TRAYICON, 0, if effective { 1 } else { 0 });
                    if play_sound {
                        play_notification_sound();
                    }
                }
            }

            if !is_plain_passthrough && (backspaces > 0 || !text.is_empty()) {
                state.send_engine_output(backspaces, &text);
                drop(lock);
                return 1;
            }

            if auto_switch_req.is_some() {
                drop(lock);
                return CallNextHookEx(null_mut(), n_code, w_param, l_param);
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

    let c = vk_to_char_with_layout(vk, state.shift_pressed);
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

fn vk_to_char_with_layout(vk: u16, shift: bool) -> char {
    unsafe {
        let hwnd = GetForegroundWindow();
        if !hwnd.is_null() {
            let thread_id = GetWindowThreadProcessId(hwnd, null_mut());
            let hkl = GetKeyboardLayout(thread_id);
            if !hkl.is_null() {
                let mut key_state = [0u8; 256];
                if shift {
                    key_state[VK_SHIFT as usize] = 0x80;
                }
                let caps_lock = (GetKeyState(VK_CAPITAL as i32) & 0x0001) != 0;
                if caps_lock {
                    key_state[VK_CAPITAL as usize] = 0x01;
                }

                let scancode = MapVirtualKeyExW(vk as u32, 0, hkl);
                let mut out_chars = [0u16; 4];
                let res = ToUnicodeEx(
                    vk as u32,
                    scancode,
                    key_state.as_ptr(),
                    out_chars.as_mut_ptr(),
                    out_chars.len() as i32,
                    4, // TM_FLAG_DONT_UPDATE_STATE
                    hkl,
                );
                if res > 0 {
                    if let Some(ch) = char::from_u32(out_chars[0] as u32) {
                        if !ch.is_control() {
                            return ch;
                        }
                    }
                }
            }
        }
    }

    vk_to_char(vk, shift)
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
        return ((ctrl || is_vk_down(VK_CONTROL))
            && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT))
            || ((shift || is_vk_down(VK_SHIFT))
            && (vk == VK_LCONTROL || vk == VK_RCONTROL || vk == VK_CONTROL));
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
        return ((alt || is_vk_down(VK_MENU))
            && (vk == VK_LSHIFT || vk == VK_RSHIFT || vk == VK_SHIFT))
            || ((shift || is_vk_down(VK_SHIFT))
            && (vk == VK_LMENU || vk == VK_RMENU || vk == VK_MENU));
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
    fn embedded_notification_is_clean_pcm_wav() {
        assert_eq!(&NOTIFICATION_SOUND[0..4], b"RIFF");
        assert_eq!(&NOTIFICATION_SOUND[8..12], b"WAVE");
        let format = u16::from_le_bytes([NOTIFICATION_SOUND[20], NOTIFICATION_SOUND[21]]);
        assert_eq!(format, 0x0001, "expected uncompressed 16-bit PCM");
        let sample_rate = u32::from_le_bytes([
            NOTIFICATION_SOUND[24],
            NOTIFICATION_SOUND[25],
            NOTIFICATION_SOUND[26],
            NOTIFICATION_SOUND[27],
        ]);
        assert_eq!(sample_rate, 48000, "expected 48kHz native mixer sample rate");
        assert!(
            NOTIFICATION_SOUND.len() < 16000,
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
            current_process: None,
            per_app_modes: HashMap::new(),
            override_unlocked: false,
            non_us_layout: false,
            non_unicode_window: false,
            active_profile: None,
            injection_mode: 0,
            macro_buffer: String::new(),
            english_word_buffer: String::new(),
            english_word_valid: true,
            auto_pause_until_ms: 0,
            tray_hwnd: std::ptr::null_mut(),
            hook_handle: std::ptr::null_mut(),
            mouse_hook_handle: std::ptr::null_mut(),
            win_event_hook: std::ptr::null_mut(),
            tsf_registered: false,
            held_vks: [false; 256],
            foreground_pid: 0,
            game_detector: GameDetectionTracker::new(),
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
    fn test_remember_per_app_mode() {
        let mut state = hook_state(false, false, true);
        state.config.remember_per_app_mode = true;

        // Start in code.exe
        state.current_process = Some("Code.exe".to_string());
        state.per_app_modes.insert("code.exe".to_string(), true);
        assert!(state.user_enabled);

        // Toggle to English in code.exe
        state.toggle_language();
        assert!(!state.user_enabled);
        assert_eq!(state.per_app_modes.get("code.exe"), Some(&false));

        // Switch to chrome.exe (user enables Vietnamese)
        state.current_process = Some("chrome.exe".to_string());
        state.per_app_modes.insert("chrome.exe".to_string(), true);
        state.user_enabled = true;

        // Switch back to code.exe -> restored to English
        let code_key = "code.exe";
        if let Some(&saved) = state.per_app_modes.get(code_key) {
            state.user_enabled = saved;
        }
        assert!(!state.user_enabled);

        // Switch back to chrome.exe -> restored to Vietnamese
        let chrome_key = "chrome.exe";
        if let Some(&saved) = state.per_app_modes.get(chrome_key) {
            state.user_enabled = saved;
        }
        assert!(state.user_enabled);
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

    #[test]
    fn test_unmodified_keystroke_is_plain_passthrough() {
        let mut engine = VietFlowEngine::default();
        for &c in &['a', 's', 'd', 't', 'o', 'n'] {
            let (backspaces, text) = engine.process_key_with_time(c, 100);
            let mut char_buf = [0u8; 4];
            let char_str = c.encode_utf8(&mut char_buf);
            assert_eq!(backspaces, 0);
            assert_eq!(text.as_str(), char_str, "Keystroke '{c}' must match itself for native passthrough");
            engine.reset();
        }

        // When a transformation occurs (e.g. 'a' + 'a' -> 'â'), it is not a plain passthrough
        let _ = engine.process_key_with_time('a', 100);
        let (backspaces, text) = engine.process_key_with_time('a', 120);
        assert_eq!(backspaces, 1);
        assert_eq!(text.as_str(), "â");
    }

    #[test]
    fn test_typing_word_then_space() {
        let mut engine = vietflow_core::VietFlowEngine::default();
        for c in "tieng".chars() {
            let (bk, out) = engine.process_key_with_time(c, 0);
            println!("{}: bk={}, out='{}'", c, bk, out.as_str());
        }
        let (bk, out) = engine.process_key_with_time('s', 0);
        println!("s: bk={}, out='{}'", bk, out.as_str());
        let (bk, out) = engine.process_key_with_time(' ', 0);
        println!("Space: bk={}, out='{}'", bk, out.as_str());
        assert_eq!(bk, 0);
        assert_eq!(out.as_str(), " ");
    }

    #[test]
    fn test_backspace_after_space_does_not_explode_word() {
        let mut engine = vietflow_core::VietFlowEngine::default();
        for c in "tiengs".chars() {
            engine.process_key_with_time(c, 0);
        }
        let (bk, out) = engine.process_key_with_time(' ', 0);
        assert_eq!(bk, 0);
        assert_eq!(out.as_str(), " ");

        // Engine word buffer is empty after space delimiter.
        // Backspace should not produce backspaces or inject raw keystrokes.
        let (bk_bs, out_bs) = engine.process_key_with_time('\x08', 0);
        assert_eq!(bk_bs, 0);
        assert_eq!(out_bs.as_str(), "");
    }

    #[test]
    fn test_held_key_repeat_detection() {
        let mut state = hook_state(false, false, true);
        assert!(!state.held_vks[0x41]);

        // Key down sets held_vks for 'A' and 'W' simultaneously
        state.held_vks[0x41] = true;
        state.held_vks[0x57] = true;
        assert!(state.held_vks[0x41]);
        assert!(state.held_vks[0x57]);

        // Releasing 'A' clears 'A' but retains 'W'
        state.held_vks[0x41] = false;
        assert!(!state.held_vks[0x41]);
        assert!(state.held_vks[0x57]);
    }

    #[test]
    fn test_game_detection_tracker() {
        let mut tracker = GameDetectionTracker::new();
        let mut held = [false; 256];

        // 1. Normal typing: 's', 'a', 'o'
        assert!(!tracker.record_key(0x53, 100, &held)); // 'S'
        assert!(!tracker.record_key(0x41, 200, &held)); // 'A'
        assert!(!tracker.record_key(0x4F, 300, &held)); // 'O'
        tracker.reset();

        // 2. Typing "đây" (d, d, a, a, y) must NEVER trigger game mode!
        assert!(!tracker.record_key(0x44, 1000, &held)); // 'D'
        assert!(!tracker.record_key(0x44, 1080, &held)); // 'D'
        assert!(!tracker.record_key(0x41, 1160, &held)); // 'A'
        assert!(!tracker.record_key(0x41, 1240, &held)); // 'A' -> "ddaa" must NOT trigger!
        assert!(!tracker.record_key(0x59, 1320, &held)); // 'Y'
        tracker.reset();

        // 3. Common Vietnamese syllables with WASD keys (daws, saws, ada, dad) must NEVER trigger
        assert!(!tracker.record_key(0x44, 2000, &held)); // 'D'
        assert!(!tracker.record_key(0x41, 2100, &held)); // 'A'
        assert!(!tracker.record_key(0x44, 2200, &held)); // 'D' -> "dad" must NOT trigger!
        tracker.reset();

        assert!(!tracker.record_key(0x53, 3000, &held)); // 'S'
        assert!(!tracker.record_key(0x57, 3100, &held)); // 'W'
        assert!(!tracker.record_key(0x53, 3200, &held)); // 'S' -> "sws" must NOT trigger!
        tracker.reset();

        // 4. Arrow keys pattern: Left -> Up -> Right
        assert!(!tracker.record_key(0x25, 4000, &held));
        assert!(!tracker.record_key(0x26, 4200, &held));
        assert!(tracker.record_key(0x27, 4400, &held));  // triggers arrow sequence!
        tracker.reset();

        // 5. Movement Chording: holding 'W' for >= 500ms while pressing 'A' or Space
        held[0x57] = true; // W is held
        tracker.key_down_time[0x57] = 5000;
        // Pressing 'A' while 'W' has been held down for 550ms
        assert!(tracker.record_key(0x41, 5550, &held)); // triggers chording detection!
        tracker.reset();

        // Pressing 'Space' while 'W' has been held down for 550ms
        held[0x57] = true;
        tracker.key_down_time[0x57] = 6000;
        assert!(tracker.record_key(VK_SPACE, 6550, &held)); // triggers chording jump!
    }
}
