#![windows_subsystem = "windows"]
// VietFlow Windows - Native Daemon Host
// Licensed under GNU General Public License v3.0 (GPLv3)

mod clipboard;
mod config;
mod hook;
mod icon;
mod inject;
mod process;
mod startup;
mod tray;
mod updater;
mod version;
pub mod tsf;

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::SystemTime;
use vietflow_core::models::{CharsetId, InputMethod};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};

use windows_sys::Win32::System::DataExchange::{
    AddClipboardFormatListener, RemoveClipboardFormatListener,
};
use windows_sys::Win32::System::Threading::{GetCurrentProcess, SetProcessWorkingSetSize};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW, GetMessageW, KillTimer, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, RegisterWindowMessageW, SetForegroundWindow, SetTimer, ShowWindow,
    TranslateMessage, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND,
    MB_TOPMOST, MSG, SW_RESTORE, SW_SHOWNORMAL, WNDCLASSW, WM_CLOSE, WM_CLIPBOARDUPDATE,
    WM_COMMAND, WM_DESTROY, WM_ENDSESSION, WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_QUERYENDSESSION,
    WM_RBUTTONUP, WM_TIMER,
};

pub mod constants;

use crate::config::AppConfig;
use crate::constants::*;
use crate::hook::{apply_config, install_hook, uninstall_hook, STATE};
use crate::icon::{create_app_icon, create_e_icon, create_tsf_icon, create_v_icon, ICON_FONT};
use crate::startup::set_start_with_windows;
use crate::tray::{to_wide, TrayIcon};

extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *mut c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> HANDLE;
}

static mut TRAY_PTR: *mut TrayIcon = null_mut();
static mut WM_TASKBAR_CREATED: u32 = 0;

const TIMER_ID_CONFIG: usize = 1;
const TIMER_ID_RETRY_TRAY: usize = 2;
static mut TRAY_RETRY_COUNT: u32 = 0;
static CONFIG_MTIME: Mutex<Option<SystemTime>> = Mutex::new(None);

/// A toggle is already reflected in STATE; the only work deferred is the
/// potentially slow JSON serialization + disk write. This flag coalesces
/// repeated toggles, and TIMER_ID_CONFIG retries when PostMessage fails.
static HOTKEY_MODE_SAVE_PENDING: AtomicBool = AtomicBool::new(false);

/// Return true only for the first request of a dirty burst. A later toggle
/// during an in-progress save gets a fresh notification after the flag clears.
fn mark_mode_save_pending(flag: &AtomicBool) -> bool {
    !flag.swap(true, Ordering::AcqRel)
}

fn take_mode_save_pending(flag: &AtomicBool) -> bool {
    flag.swap(false, Ordering::AcqRel)
}

/// Safe for the low-level keyboard callback: no STATE lock and no filesystem.
pub(crate) fn request_hotkey_mode_persist(hwnd: HWND) {
    if mark_mode_save_pending(&HOTKEY_MODE_SAVE_PENDING) && !hwnd.is_null() {
        unsafe {
            let _ = PostMessageW(hwnd, WM_PERSIST_HOTKEY_MODE, 0, 0);
        }
    }
}

/// Runs only on the normal window message loop / shutdown path, NEVER in a
/// keyboard hook. Snapshot the newest config before writing it; a hotkey
/// that arrives during save sets the flag again for the next message/timer.
fn flush_hotkey_mode_persist() {
    if !take_mode_save_pending(&HOTKEY_MODE_SAVE_PENDING) {
        return;
    }
    let snapshot = STATE
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|state| state.config.clone()));
    if let Some(config) = snapshot {
        config.save();
        // No redundant reload of our own just-persisted configuration.
        if let Ok(mut mtime) = CONFIG_MTIME.lock() {
            *mtime = config_mtime();
        }
    } else {
        // A transient missing/poisoned state cannot erase a requested save.
        mark_mode_save_pending(&HOTKEY_MODE_SAVE_PENDING);
    }
}

fn config_mtime() -> Option<SystemTime> {
    std::fs::metadata(AppConfig::config_path_ref())
        .ok()
        .and_then(|meta| meta.modified().ok())
}

static UI_CHILDREN: Mutex<Vec<std::process::Child>> = Mutex::new(Vec::new());

#[inline]
fn trim_working_set() {
    unsafe {
        SetProcessWorkingSetSize(GetCurrentProcess(), usize::MAX, usize::MAX);
    }
}

/// Drop handles for UI children that have already exited. Without this, the
/// daemon keeps one process handle after the settings window is closed until
/// the next time the GUI is opened.
fn reap_ui_children() {
    if let Ok(mut children) = UI_CHILDREN.lock() {
        let prev_len = children.len();
        children.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
        if prev_len > 0 && children.is_empty() {
            trim_working_set();
        }
    }
}

fn spawn_gui() {
    unsafe {
        let class_name = to_wide(WINDOW_CLASS_UI);
        let existing = FindWindowW(class_name.as_ptr(), null_mut());
        if !existing.is_null() {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
            return;
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut command = std::process::Command::new(exe);
        let config_path = crate::config::AppConfig::config_path();
        command.arg("--config").arg(config_path);
        command.arg("--settings");
        if let Ok(mut children) = UI_CHILDREN.lock() {
            // Drop already-exited children before adding a new one.
            children.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
            if !children.is_empty() {
                return;
            }
            if let Ok(child) = command.spawn() {
                children.push(child);
            }
        }
    }
}

fn terminate_all_ui() {
    if let Ok(mut children) = UI_CHILDREN.lock() {
        for child in children.iter_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        children.clear();
    }

    unsafe {
        let class_name = to_wide(WINDOW_CLASS_UI);
        let mut gui_hwnd = FindWindowW(class_name.as_ptr(), null_mut());
        while !gui_hwnd.is_null() {
            PostMessageW(gui_hwnd, WM_CLOSE, 0, 0);
            gui_hwnd = FindWindowW(class_name.as_ptr(), null_mut());
            if !gui_hwnd.is_null() {
                // If it doesn't close immediately, break to avoid infinite loop
                break;
            }
        }
    }
}

fn show_error(message: &str) {
    show_message(message, MB_OK | MB_ICONERROR);
}

#[allow(dead_code)]
fn show_warning(message: &str) {
    show_message(message, MB_OK | MB_ICONWARNING);
}

fn show_message(message: &str, flags: u32) {
    let message = to_wide(message);
    let title = to_wide("VietFlow");
    unsafe {
        MessageBoxW(null_mut(), message.as_ptr(), title.as_ptr(), flags);
    }
}

fn warn_ime_conflicts() {
    let conflicts = crate::process::known_ime_conflicts();
    if conflicts.is_empty() {
        return;
    }
    let message = format!(
        "Phát hiện bộ gõ tiếng Việt khác đang chạy:\n{}\n\n\
         Hãy tắt bộ gõ này khi dùng VietFlow để tránh gõ đúp hoặc mất dấu.",
        conflicts.join(", ")
    );
    show_message(
        &message,
        MB_OK | MB_ICONWARNING | MB_SETFOREGROUND | MB_TOPMOST,
    );
}

/// A second launch should open the settings window of the running daemon.
/// If the first instance is still starting up, wait briefly for its message
/// window instead of exiting silently.
unsafe fn open_existing_settings() -> bool {
    let class_name = to_wide(WINDOW_CLASS_DAEMON);
    let window_title = to_wide(WINDOW_TITLE_DAEMON);

    let gui_class = to_wide(WINDOW_CLASS_UI);
    let gui_hwnd = FindWindowW(gui_class.as_ptr(), null_mut());
    if !gui_hwnd.is_null() {
        ShowWindow(gui_hwnd, SW_RESTORE);
        SetForegroundWindow(gui_hwnd);
    }

    for _ in 0..30 {
        let hwnd = FindWindowW(class_name.as_ptr(), window_title.as_ptr());
        if !hwnd.is_null() {
            PostMessageW(hwnd, WM_SETTINGS, 0, 0);
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    false
}

const EMBEDDED_FONT: &[u8] = include_bytes!("../../../ui_imgui/assets/JetBrainsMonoNerdFont-Regular.ttf");

extern "C" {
    fn SetEmbeddedFontData(data: *const u8, size: usize);
    fn SetTrayFontData(data: *const u8, size: usize);
    fn ShowSettingsGui(config_path: *const std::ffi::c_char, on_saved: Option<extern "C" fn()>) -> i32;
    fn ShowConverterGui(config_path: *const std::ffi::c_char) -> i32;
}

fn run_native_gui(convert_mode: bool) {
    unsafe {
        let class_name = to_wide(WINDOW_CLASS_UI);
        let existing = FindWindowW(class_name.as_ptr(), null_mut());
        if !existing.is_null() {
            ShowWindow(existing, SW_RESTORE);
            SetForegroundWindow(existing);
            return;
        }
        SetEmbeddedFontData(EMBEDDED_FONT.as_ptr(), EMBEDDED_FONT.len());
        SetTrayFontData(ICON_FONT.as_ptr(), ICON_FONT.len());

        let config_path = crate::config::AppConfig::config_path();
        let path_str = config_path.to_string_lossy();
        let c_path = std::ffi::CString::new(path_str.as_bytes()).unwrap_or_default();

        if convert_mode {
            ShowConverterGui(c_path.as_ptr());
        } else {
            ShowSettingsGui(c_path.as_ptr(), None);
        }
    }
}

fn relaunch_as_admin() -> bool {
    let exe = match std::env::current_exe() {
        Ok(path) => path,
        Err(_) => return false,
    };
    let verb = to_wide("runas");
    let file = to_wide(&exe.to_string_lossy());
    let params = to_wide("--elevated");
    let result = unsafe {
        ShellExecuteW(
            null_mut(),
            verb.as_ptr(),
            file.as_ptr(),
            params.as_ptr(),
            null_mut(),
            SW_SHOWNORMAL,
        )
    };
    result as isize > 32
}

fn main() {
    #[link(name = "kernel32")]
    extern "system" {
        fn AttachConsole(dwProcessId: u32) -> windows_sys::core::BOOL;
    }
    const ATTACH_PARENT_PROCESS: u32 = 0xFFFFFFFF;
    unsafe {
        AttachConsole(ATTACH_PARENT_PROCESS);
    }

    let args: Vec<String> = std::env::args().collect();

    if let Some(pos) = args.iter().position(|a| a == "--config") {
        if let Some(path) = args.get(pos + 1) {
            crate::config::set_custom_config_path(std::path::PathBuf::from(path));
        }
    }

    if args.iter().any(|a| a == "--uninstall-cleanup") {
        unsafe {
            crate::startup::clean_all_shortcuts_and_cache(true);
        }
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--clean-shortcuts") {
        unsafe {
            crate::startup::clean_all_shortcuts_and_cache(false);
        }
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--status-tsf") {
        let is_reg = crate::tsf::is_tsf_registered();
        let is_active = crate::tsf::is_tsf_mode_active();
        let dll_path = crate::tsf::tsf_dll_path();
        let temp_dir = std::env::var("TEMP").unwrap_or_default();
        let log_path = format!("{temp_dir}\\vietflow_tsf.log");

        println!("=== VietFlow TSF Diagnostic Status ===");
        println!("  Registered:   {is_reg}");
        println!("  Active Mode:  {is_active}");
        println!("  DLL Path:     {:?}", dll_path.as_ref().map(|p| p.display().to_string()));
        println!("  Log File:     {log_path}");
        println!("  Log Exists:   {}", std::path::Path::new(&log_path).exists());
        println!("======================================");
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--log-tsf" || a == "--tail-tsf") {
        let temp_dir = std::env::var("TEMP").unwrap_or_default();
        let log_path = format!("{temp_dir}\\vietflow_tsf.log");
        if let Ok(content) = std::fs::read_to_string(&log_path) {
            let lines: Vec<&str> = content.lines().collect();
            let tail_count = lines.len().saturating_sub(60);
            println!("=== Last {} log lines from {} ===", lines.len() - tail_count, log_path);
            for line in &lines[tail_count..] {
                println!("{line}");
            }
        } else {
            println!("Chưa có file log tại {log_path} hoặc file đang trống.");
        }
        std::process::exit(0);
    }

    if args.iter().any(|a| a == "--gui" || a == "-g" || a == "--settings") {
        run_native_gui(false);
        return;
    }
    if args.iter().any(|a| a == "--convert" || a == "-c") {
        run_native_gui(true);
        return;
    }
    if args.iter().any(|a| a == "--register-tsf") {
        match crate::tsf::register_tsf() {
            Ok(_) => {
                let mut config = AppConfig::load();
                config.use_tsf = true;
                config.save();
                crate::tsf::sync_tsf_config(&config, config.enabled);
                println!("Đã đăng ký VietFlow TSF TIP thành công.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Lỗi đăng ký VietFlow TSF TIP: {e}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--msi-final-unregister-tsf") {
        // MSI runs as the installer service, not as the signed-in user.
        // Preserve user config, macros, keyboard layouts and shortcuts.
        match crate::tsf::unregister_tsf_from_msi_final_uninstall() {
            Ok(()) => std::process::exit(0),
            Err(e) => {
                eprintln!("MSI final TSF unregister failed: {e}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--unregister-tsf") {
        match crate::tsf::unregister_tsf() {
            Ok(_) => {
                let mut config = AppConfig::load();
                config.use_tsf = false;
                config.save();
                println!("Đã hủy đăng ký VietFlow TSF TIP thành công.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Lỗi hủy đăng ký VietFlow TSF TIP: {e}");
                std::process::exit(1);
            }
        }
    }

    // When TSF mode is active, synchronize initial configuration to cross-process shared memory
    // while keeping daemon active for tray management, hotkeys, and fallback hook for non-TSF apps.
    if crate::tsf::is_tsf_mode_active() {
        let config = AppConfig::load();
        crate::tsf::sync_tsf_config(&config, config.enabled);
    }

    unsafe {
        // Enforce single instance for daemon. An elevated relaunch is started
        // by the old process, so give the old mutex a few seconds to disappear.
        let mutex_name = to_wide(MUTEX_SINGLE_INSTANCE);
        let elevated = args.iter().any(|a| a == "--elevated");
        let mut mutex = CreateMutexW(null_mut(), 1, mutex_name.as_ptr());
        let mut last_error = GetLastError();
        if elevated && last_error == ERROR_ALREADY_EXISTS {
            if !mutex.is_null() {
                CloseHandle(mutex);
            }
            for _ in 0..40 {
                std::thread::sleep(std::time::Duration::from_millis(100));
                mutex = CreateMutexW(null_mut(), 1, mutex_name.as_ptr());
                last_error = GetLastError();
                if last_error != ERROR_ALREADY_EXISTS {
                    break;
                }
                if !mutex.is_null() {
                    CloseHandle(mutex);
                }
            }
        }
        if mutex.is_null() || last_error == ERROR_ALREADY_EXISTS {
            if last_error == ERROR_ALREADY_EXISTS {
                if !open_existing_settings() {
                    let message = to_wide(
                        "VietFlow đang chạy trên hệ thống!\n\nNhấn vào biểu tượng [V] / [E] ở góc phải dưới thanh tác vụ để mở bảng điều khiển."
                    );
                    let title = to_wide("VietFlow - Đang hoạt động");
                    MessageBoxW(
                        null_mut(),
                        message.as_ptr(),
                        title.as_ptr(),
                        MB_OK | MB_ICONINFORMATION | 0x00040000 | 0x00010000,
                    );
                }
            } else {
                show_error("Không thể khởi động VietFlow.");
            }
            return;
        }

        let config = AppConfig::load();
        warn_ime_conflicts();

        let class_name = to_wide(WINDOW_CLASS_DAEMON);
        let mut wc: WNDCLASSW = zeroed();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

        let window_title = to_wide(WINDOW_TITLE_DAEMON);
        let hwnd = CreateWindowExW(
            0,
            class_name.as_ptr(),
            window_title.as_ptr(),
            0,
            0,
            0,
            0,
            0,
            null_mut(),
            null_mut(),
            null_mut(),
            null_mut(),
        );

        WM_TASKBAR_CREATED = RegisterWindowMessageW(to_wide("TaskbarCreated\0").as_ptr());

        // Synchronize TSF configuration to cross-process shared memory immediately on daemon launch
        crate::tsf::sync_tsf_config(&config, config.enabled);

        let icon_v = create_v_icon(&config);
        let icon_e = create_e_icon(&config);
        let icon_app = create_app_icon();
        let icon_tsf = create_tsf_icon();
        let is_tsf_mode = crate::tsf::is_tsf_mode_active();
        if is_tsf_mode {
            crate::tsf::call_install_layout_or_tip(
                &format!("0409:{}{}", crate::tsf::CLSID_STR, crate::tsf::PROFILE_STR),
                false,
            );
            crate::tsf::call_install_layout_or_tip(
                &format!("042a:{}{}", crate::tsf::CLSID_STR, crate::tsf::PROFILE_STR),
                true,
            );
        }
        let mut tray = TrayIcon::new(hwnd, icon_v, icon_e, icon_app, icon_tsf, is_tsf_mode);
        tray.update_status(config.enabled);

        TRAY_PTR = &mut tray;

        AddClipboardFormatListener(hwnd);
        crate::clipboard::refresh();
        install_hook(hwnd, config);

        *CONFIG_MTIME.lock().unwrap() = config_mtime();
        SetTimer(hwnd, TIMER_ID_CONFIG, 250, None);

        trim_working_set();

        let mut msg: MSG = zeroed();
        while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        uninstall_hook();
        TRAY_PTR = null_mut();
    }
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    w_param: WPARAM,
    l_param: LPARAM,
) -> LRESULT {
    match msg {
        WM_TRAYICON => {
            let event = l_param as u32;
            if event == WM_LBUTTONDBLCLK {
                spawn_gui();
            } else if event == WM_LBUTTONUP {
                if !TRAY_PTR.is_null() && (*TRAY_PTR).is_tsf_mode {
                    spawn_gui();
                } else {
                    toggle_mode(hwnd);
                }
            } else if event == WM_RBUTTONUP {
                if let Ok(lock) = STATE.lock() {
                    if let Some(state) = lock.as_ref() {
                        if !TRAY_PTR.is_null() {
                            (*TRAY_PTR).show_context_menu(
                                state.effective_enabled(),
                                state.config.method,
                                state.config.charset,
                                state.config.enable_macro,
                                state.config.spell_check,
                                state.config.start_with_windows,
                                &state.config.hotkey_mode,
                                &state.config.hotkey_settings,
                                crate::tsf::is_tsf_dll_present(),
                                crate::tsf::is_tsf_registered(),
                            );
                        }
                    }
                }
            } else if w_param == 0 {
                // Internal status update from hook
                let is_vietnamese = l_param == 1;
                if !TRAY_PTR.is_null() {
                    (*TRAY_PTR).update_status(is_vietnamese);
                }
            }
            0
        }
        WM_SETTINGS => {
            spawn_gui();
            0
        }
        WM_PERSIST_HOTKEY_MODE => {
            flush_hotkey_mode_persist();
            0
        }
        WM_TIMER => {
            reap_ui_children();
            if w_param == TIMER_ID_RETRY_TRAY {
                let ok = if !TRAY_PTR.is_null() {
                    (*TRAY_PTR).re_add()
                } else {
                    false
                };
                TRAY_RETRY_COUNT += 1;
                if ok || TRAY_RETRY_COUNT >= 6 {
                    KillTimer(hwnd, TIMER_ID_RETRY_TRAY);
                }
                return 0;
            }
            if w_param == TIMER_ID_CONFIG {
                // Retry when hotkey's PostMessage was lost, while avoiding
                // disk writes inside the global keyboard hook callback.
                flush_hotkey_mode_persist();
                let current = config_mtime();
                let changed = {
                    let mut last = match CONFIG_MTIME.lock() {
                        Ok(lock) => lock,
                        Err(_) => return 0,
                    };
                    if current.is_some() && *last != current {
                        *last = current;
                        true
                    } else {
                        false
                    }
                };

                if changed {
                    reload_runtime_config(hwnd);
                }
            }
            0
        }
        WM_RELOAD_CONFIG => {
            reload_runtime_config(hwnd);
            0
        }
        WM_CLIPBOARDUPDATE => {
            crate::clipboard::refresh();
            0
        }
        WM_COMMAND => {
            let id = w_param;
            match id {
                IDM_TOGGLE => {
                    toggle_mode(hwnd);
                }
                IDM_METHOD_TELEX
                | IDM_METHOD_VNI
                | IDM_METHOD_VIQR
                | IDM_METHOD_SIMPLE_TELEX
                | IDM_METHOD_SIMPLE_TELEX2
                | IDM_METHOD_TELEX_VNI
                | IDM_METHOD_CUSTOM => {
                    let m = match id {
                        IDM_METHOD_TELEX => 0,
                        IDM_METHOD_VNI => 1,
                        IDM_METHOD_VIQR => 2,
                        IDM_METHOD_SIMPLE_TELEX => 3,
                        IDM_METHOD_SIMPLE_TELEX2 => 4,
                        IDM_METHOD_TELEX_VNI => 5,
                        IDM_METHOD_CUSTOM => 6,
                        _ => 0,
                    };
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.method = m;
                            state.engine.options.method = match m {
                                0 => InputMethod::Telex,
                                1 => InputMethod::Vni,
                                2 => InputMethod::Viqr,
                                3 => InputMethod::SimpleTelex,
                                4 => InputMethod::TelexSimple2,
                                5 => InputMethod::TelexVni,
                                6 => InputMethod::UserCustom,
                                _ => InputMethod::Telex,
                            };
                            state.engine.reset();
                            let effective = state.effective_enabled();
                            crate::tsf::sync_tsf_config(&state.config, effective);
                            state.config.save();
                        }
                    }
                }
                IDM_CHARSET_UNICODE | IDM_CHARSET_COMPOSITE | IDM_CHARSET_TCVN3 | IDM_CHARSET_VNI => {
                    let c = match id {
                        IDM_CHARSET_UNICODE => 0,
                        IDM_CHARSET_COMPOSITE => 1,
                        IDM_CHARSET_TCVN3 => 2,
                        IDM_CHARSET_VNI => 3,
                        _ => 0,
                    };
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.charset = c;
                            state.engine.charset = match c {
                                0 => CharsetId::Unicode,
                                1 => CharsetId::UnicodeComposite,
                                2 => CharsetId::Tcvn3,
                                3 => CharsetId::VniWindows,
                                _ => CharsetId::Unicode,
                            };
                            state.engine.reset();
                            let effective = state.effective_enabled();
                            crate::tsf::sync_tsf_config(&state.config, effective);
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_MACRO => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.enable_macro = !state.config.enable_macro;
                            state.engine.options.enable_macro = state.config.enable_macro;
                            let effective = state.effective_enabled();
                            crate::tsf::sync_tsf_config(&state.config, effective);
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_SPELL => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.spell_check = !state.config.spell_check;
                            state.engine.options.spell_check = state.config.spell_check;
                            let effective = state.effective_enabled();
                            crate::tsf::sync_tsf_config(&state.config, effective);
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_STARTUP => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.start_with_windows = !state.config.start_with_windows;
                            set_start_with_windows(state.config.start_with_windows);
                            state.config.save();
                        }
                    }
                }
                IDM_SETTINGS => {
                    spawn_gui();
                }
                IDM_RESTART_ADMIN => {
                    if relaunch_as_admin() {
                        terminate_all_ui();
                        PostQuitMessage(0);
                    } else {
                        show_error("Không thể yêu cầu quyền quản trị. Bạn đã hủy hộp thoại UAC?");
                    }
                }
                IDM_CHECK_UPDATE => {
                    crate::updater::check_for_updates_interactive();
                }
                IDM_ABOUT => {
                    crate::updater::show_about_dialog();
                }
                IDM_TSF_REGISTER => {
                    if crate::tsf::is_tsf_dll_present() {
                        let is_reg = crate::tsf::is_tsf_registered();
                        let result = if is_reg {
                            crate::tsf::unregister_tsf()
                        } else {
                            crate::tsf::register_tsf()
                        };
                        match result {
                            Ok(_) => {
                                if let Ok(mut lock) = STATE.lock() {
                                    if let Some(state) = lock.as_mut() {
                                        // use_tsf mirrors the registration for
                                        // config compatibility; it is not an opt-in.
                                        state.config.use_tsf = !is_reg;
                                        state.config.save();
                                    }
                                }
                                if !is_reg {
                                    show_message(
                                        "Đã cài đặt và đăng ký VietFlow TSF IME thành công.\n\nChế độ TSF xử lý trực tiếp trong từng ứng dụng. Tiến trình chạy nền sẽ tự đóng.\nBạn có thể mở lại VietFlow bất cứ lúc nào để chỉnh bảng điều khiển.",
                                        MB_OK | MB_ICONINFORMATION,
                                    );
                                    DestroyWindow(hwnd);
                                    return 0;
                                } else {
                                    show_message(
                                        "Đã hủy đăng ký VietFlow TSF IME thành công.\nVietFlow tiếp tục hoạt động ở chế độ Hook truyền thống.",
                                        MB_OK | MB_ICONINFORMATION,
                                    );
                                    reload_runtime_config(hwnd);
                                }
                            }
                            Err(e) => {
                                let action = if is_reg { "hủy đăng ký" } else { "đăng ký" };
                                show_error(&format!("Lỗi {action} TSF: {e}"));
                                reload_runtime_config(hwnd);
                            }
                        }
                    }
                }
                IDM_EXIT => {
                    DestroyWindow(hwnd);
                }
                _ => {}
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_QUERYENDSESSION => {
            1
        }
        WM_ENDSESSION => {
            if w_param != 0 {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_DESTROY => {
            RemoveClipboardFormatListener(hwnd);
            terminate_all_ui();
            // Stop new keyboard callbacks BEFORE saving the final mode.
            // Otherwise a hotkey could set the dirty flag after the last flush.
            uninstall_hook();
            flush_hotkey_mode_persist();
            PostQuitMessage(0);
            0
        }
        _ => {
            if WM_TASKBAR_CREATED != 0 && msg == WM_TASKBAR_CREATED {
                let ok = if !TRAY_PTR.is_null() {
                    (*TRAY_PTR).re_add()
                } else {
                    false
                };
                if !ok {
                    TRAY_RETRY_COUNT = 0;
                    SetTimer(hwnd, TIMER_ID_RETRY_TRAY, 400, None);
                }
                0
            } else {
                DefWindowProcW(hwnd, msg, w_param, l_param)
            }
        }
    }
}

unsafe fn reload_runtime_config(_hwnd: HWND) {
    let config = AppConfig::load();
    if !TRAY_PTR.is_null() {
        let is_tsf_mode = crate::tsf::is_tsf_mode_active();
        (*TRAY_PTR).set_tsf_mode(is_tsf_mode);
        let rebuild_icons = STATE
            .lock()
            .ok()
            .and_then(|lock| {
                lock.as_ref()
                    .map(|state| state.config.icon_settings_differ(&config))
            })
            .unwrap_or(true);
        if rebuild_icons {
            let icon_v = create_v_icon(&config);
            let icon_e = create_e_icon(&config);
            let icon_app = create_app_icon();
            let icon_tsf = create_tsf_icon();
            (*TRAY_PTR).set_icons(icon_v, icon_e, icon_app, icon_tsf);
        }
        let effective = STATE
            .lock()
            .ok()
            .and_then(|lock| lock.as_ref().map(|s| s.effective_enabled()))
            .unwrap_or(config.enabled);
        (*TRAY_PTR).update_status(effective);
    }
    apply_config(config);
}

unsafe fn toggle_mode(_hwnd: HWND) {
    // V/E is owned by the Windows input switcher while the TIP is registered.
    if !TRAY_PTR.is_null() && (*TRAY_PTR).is_tsf_mode {
        return;
    }
    if let Ok(mut lock) = STATE.lock() {
        if let Some(state) = lock.as_mut() {
            // A tray click can move focus to the shell. If we came from an
            // excluded app, toggle that app's temporary override instead of
            // changing the global V/E mode.
            if state.last_game_process.is_some() {
                state.toggle_temporary_override();
            } else {
                state.toggle_language();
                state.config.save();
            }
            if state.config.notification_sound {
                crate::hook::play_notification_sound();
            }
            let effective = state.effective_enabled();
            crate::tsf::sync_tsf_config(&state.config, effective);
            if !TRAY_PTR.is_null() {
                (*TRAY_PTR).update_status(effective);
            }
        }
    }
}

#[cfg(test)]
mod persistence_tests {
    use super::*;

    #[test]
    fn mode_saves_are_coalesced_and_rearmed_after_snapshot() {
        let dirty = AtomicBool::new(false);
        assert!(!take_mode_save_pending(&dirty));
        assert!(mark_mode_save_pending(&dirty));
        assert!(!mark_mode_save_pending(&dirty),
                "repeated toggle must not flood the UI message queue");
        assert!(take_mode_save_pending(&dirty));
        assert!(!take_mode_save_pending(&dirty));
        // A newer toggle while an older snapshot is being saved must not
        // be lost, even if the earlier message has already consumed its flag.
        mark_mode_save_pending(&dirty);
        assert!(take_mode_save_pending(&dirty));
    }

    #[test]
    fn keyboard_hook_save_request_never_acquires_state_mutex() {
        let _state_lock = STATE.lock().unwrap();
        // The hook may already own STATE when requesting persistence.
        // The request must never try to acquire it or write config to disk.
        request_hotkey_mode_persist(std::ptr::null_mut());
        assert!(take_mode_save_pending(&HOTKEY_MODE_SAVE_PENDING));
    }

    #[test]
    fn saving_after_missing_state_keeps_retry_request() {
        let dirty = AtomicBool::new(false);
        mark_mode_save_pending(&dirty);
        assert!(take_mode_save_pending(&dirty));
        // When STATE is temporarily unavailable, the production handler
        // restores the flag so WM_TIMER can try again.
        mark_mode_save_pending(&dirty);
        assert!(take_mode_save_pending(&dirty));
    }
}
