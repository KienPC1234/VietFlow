#![windows_subsystem = "windows"]
// VietFlow Windows - Native Daemon Host
// Licensed under GNU General Public License v3.0 (GPLv3)

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
use std::sync::Mutex;
use std::time::SystemTime;
use vietflow_core::models::{CharsetId, InputMethod};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};

use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, ShowWindow,
    TranslateMessage, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MB_SETFOREGROUND,
    MB_TOPMOST, MSG, SW_RESTORE, SW_SHOWNORMAL, WNDCLASSW, WM_CLOSE, WM_COMMAND, WM_DESTROY,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_RBUTTONUP, WM_TIMER,
};

pub mod constants;

use crate::config::AppConfig;
use crate::constants::*;
use crate::hook::{apply_config, install_hook, uninstall_hook, STATE};
use crate::icon::{create_app_icon, create_e_icon, create_v_icon};
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

const TIMER_ID_CONFIG: usize = 1;
static CONFIG_MTIME: Mutex<Option<SystemTime>> = Mutex::new(None);

fn config_mtime() -> Option<SystemTime> {
    std::fs::metadata(AppConfig::config_path_ref())
        .ok()
        .and_then(|meta| meta.modified().ok())
}

static UI_CHILDREN: Mutex<Vec<std::process::Child>> = Mutex::new(Vec::new());

/// Drop handles for UI children that have already exited. Without this, the
/// daemon keeps one process handle after the settings window is closed until
/// the next time the GUI is opened.
fn reap_ui_children() {
    if let Ok(mut children) = UI_CHILDREN.lock() {
        children.retain_mut(|child| matches!(child.try_wait(), Ok(None)));
    }
}

fn spawn_gui(convert_mode: bool) {
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
        if convert_mode {
            command.arg("--convert");
        } else {
            command.arg("--settings");
        }
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
        if convert_mode {
            ShowConverterGui(std::ptr::null());
        } else {
            ShowSettingsGui(std::ptr::null(), None);
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
    let args: Vec<String> = std::env::args().collect();

    // Fast-path for Velopack installer / updater lifecycle hooks
    // (--veloapp-install, --veloapp-updated, --veloapp-obsolete, --veloapp-uninstall)
    if args.iter().any(|a| a.starts_with("--veloapp-")) {
        return;
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
                println!("Đã đăng ký VietFlow TSF TIP thành công.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Lỗi đăng ký VietFlow TSF TIP: {e}");
                std::process::exit(1);
            }
        }
    }
    if args.iter().any(|a| a == "--unregister-tsf") {
        match crate::tsf::unregister_tsf() {
            Ok(_) => {
                println!("Đã hủy đăng ký VietFlow TSF TIP thành công.");
                std::process::exit(0);
            }
            Err(e) => {
                eprintln!("Lỗi hủy đăng ký VietFlow TSF TIP: {e}");
                std::process::exit(1);
            }
        }
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

        let icon_v = create_v_icon(&config);
        let icon_e = create_e_icon(&config);
        let icon_app = create_app_icon();
        let is_tsf_mode = config.use_tsf && crate::tsf::is_tsf_dll_present() && crate::tsf::is_tsf_registered();
        let mut tray = TrayIcon::new(hwnd, icon_v, icon_e, icon_app, is_tsf_mode);
        tray.update_status(config.enabled);

        TRAY_PTR = &mut tray;

        install_hook(hwnd, config);

        *CONFIG_MTIME.lock().unwrap() = config_mtime();
        SetTimer(hwnd, TIMER_ID_CONFIG, 250, None);

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
                spawn_gui(false);
            } else if event == WM_LBUTTONUP {
                let is_tsf = !TRAY_PTR.is_null() && (*TRAY_PTR).is_tsf_mode;
                if is_tsf {
                    spawn_gui(false);
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
                                crate::tsf::is_tsf_dll_present(),
                                state.config.use_tsf,
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
            spawn_gui(false);
            0
        }
        WM_TIMER => {
            reap_ui_children();
            if w_param == TIMER_ID_CONFIG {
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
                    reload_runtime_config();
                }
            }
            0
        }
        WM_RELOAD_CONFIG => {
            reload_runtime_config();
            0
        }
        WM_CONVERT => {
            spawn_gui(true);
            0
        }
        WM_COMMAND => {
            let id = w_param as usize;
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
                            crate::tsf::sync_tsf_config(&state.config, state.effective_enabled());
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
                            crate::tsf::sync_tsf_config(&state.config, state.effective_enabled());
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_MACRO => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.enable_macro = !state.config.enable_macro;
                            state.engine.options.enable_macro = state.config.enable_macro;
                            crate::tsf::sync_tsf_config(&state.config, state.effective_enabled());
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_SPELL => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.spell_check = !state.config.spell_check;
                            state.engine.options.spell_check = state.config.spell_check;
                            crate::tsf::sync_tsf_config(&state.config, state.effective_enabled());
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
                IDM_CONVERT_TOOL => {
                    spawn_gui(true);
                }
                IDM_SETTINGS => {
                    spawn_gui(false);
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
                IDM_TSF_ENABLE => {
                    if crate::tsf::is_tsf_dll_present() {
                        if let Ok(mut lock) = STATE.lock() {
                            if let Some(state) = lock.as_mut() {
                                if !state.config.use_tsf && !crate::tsf::is_tsf_registered() {
                                    let _ = crate::tsf::register_tsf();
                                }
                                state.config.use_tsf = !state.config.use_tsf;
                                crate::tsf::sync_tsf_config(&state.config, state.effective_enabled());
                                if state.config.use_tsf && crate::tsf::is_tsf_registered() {
                                    crate::tsf::set_tsf_profile_active(true);
                                } else {
                                    crate::tsf::set_tsf_profile_active(false);
                                }
                                state.config.save();
                            }
                        }
                    }
                }
                IDM_TSF_REGISTER => {
                    if crate::tsf::is_tsf_dll_present() {
                        let is_reg = crate::tsf::is_tsf_registered();
                        if is_reg {
                            match crate::tsf::unregister_tsf() {
                                Ok(_) => show_message(
                                    "Đã hủy đăng ký VietFlow TSF IME thành công.",
                                    MB_OK | MB_ICONINFORMATION,
                                ),
                                Err(e) => show_error(&format!("Lỗi hủy đăng ký TSF: {e}")),
                            }
                        } else {
                            match crate::tsf::register_tsf() {
                                Ok(_) => {
                                    if let Ok(lock) = STATE.lock() {
                                        if let Some(state) = lock.as_ref() {
                                            if state.config.use_tsf {
                                                crate::tsf::set_tsf_profile_active(true);
                                            }
                                        }
                                    }
                                    show_message(
                                        "Đã cài đặt và đăng ký VietFlow TSF IME thành công.",
                                        MB_OK | MB_ICONINFORMATION,
                                    );
                                }
                                Err(e) => show_error(&format!("Lỗi đăng ký TSF: {e}")),
                            }
                        }
                    }
                }
                IDM_EXIT => {
                    terminate_all_ui();
                    uninstall_hook();
                    if !TRAY_PTR.is_null() {
                        let tray = TRAY_PTR;
                        TRAY_PTR = null_mut();
                        std::ptr::drop_in_place(tray);
                    }
                    std::process::exit(0);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            terminate_all_ui();
            uninstall_hook();
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

unsafe fn reload_runtime_config() {
    let config = AppConfig::load();
    if !TRAY_PTR.is_null() {
        let is_tsf_mode = config.use_tsf && crate::tsf::is_tsf_dll_present() && crate::tsf::is_tsf_registered();
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
            (*TRAY_PTR).set_icons(icon_v, icon_e, icon_app);
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
