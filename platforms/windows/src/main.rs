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

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::null_mut;
use std::sync::Mutex;
use std::time::SystemTime;
use vietflow_core::models::{CharsetId, InputMethod};
use windows_sys::Win32::Foundation::{
    CloseHandle, GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM,
};
use windows_sys::Win32::System::Threading::{OpenProcess, TerminateProcess, PROCESS_TERMINATE};
use windows_sys::Win32::UI::Shell::ShellExecuteW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, MessageBoxW,
    PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, ShowWindow,
    TranslateMessage, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONWARNING, MB_OK, MSG, SW_RESTORE,
    SW_SHOWNORMAL, WNDCLASSW, WM_CLOSE, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK, WM_LBUTTONUP,
    WM_RBUTTONUP, WM_TIMER,
};

pub mod constants;

use crate::config::AppConfig;
use crate::constants::*;
use crate::hook::{apply_config, install_hook, uninstall_hook, STATE};
use crate::icon::{create_e_icon, create_v_icon};
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
    std::fs::metadata(AppConfig::config_path())
        .ok()
        .and_then(|meta| meta.modified().ok())
}

static UI_CHILD_PIDS: Mutex<Vec<u32>> = Mutex::new(Vec::new());

fn spawn_gui(convert_mode: bool) {
    if let Ok(exe) = std::env::current_exe() {
        let mut command = std::process::Command::new(exe);
        if convert_mode {
            command.arg("--convert");
        } else {
            command.arg("--settings");
        }
        if let Ok(child) = command.spawn() {
            if let Ok(mut pids) = UI_CHILD_PIDS.lock() {
                pids.push(child.id());
            }
        }
    }
}

fn terminate_all_ui() {
    if let Ok(pids) = UI_CHILD_PIDS.lock() {
        for &pid in pids.iter() {
            unsafe {
                let h = OpenProcess(PROCESS_TERMINATE, 0, pid);
                if !h.is_null() {
                    TerminateProcess(h, 0);
                    CloseHandle(h);
                }
            }
        }
    }

    unsafe {
        let gui_title = to_wide("VietFlow - Bảng điều khiển bộ gõ tiếng Việt");
        let gui_hwnd = FindWindowW(null_mut(), gui_title.as_ptr());
        if !gui_hwnd.is_null() {
            PostMessageW(gui_hwnd, WM_CLOSE, 0, 0);
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

/// A second launch should open the settings window of the running daemon.
/// If the first instance is still starting up, wait briefly for its message
/// window instead of exiting silently.
unsafe fn open_existing_settings() -> bool {
    let class_name = to_wide(WINDOW_CLASS_DAEMON);
    let window_title = to_wide(WINDOW_TITLE_DAEMON);

    let gui_title = to_wide("VietFlow - Bảng điều khiển bộ gõ tiếng Việt");
    let gui_hwnd = FindWindowW(null_mut(), gui_title.as_ptr());
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
    if args.iter().any(|a| a == "--gui" || a == "-g" || a == "--settings") {
        run_native_gui(false);
        return;
    }
    if args.iter().any(|a| a == "--convert" || a == "-c") {
        run_native_gui(true);
        return;
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
        let mut tray = TrayIcon::new(hwnd, icon_v, icon_e);
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
                toggle_mode(hwnd);
                spawn_gui(false);
            } else if event == WM_LBUTTONUP {
                toggle_mode(hwnd);
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
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_MACRO => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.enable_macro = !state.config.enable_macro;
                            state.engine.options.enable_macro = state.config.enable_macro;
                            state.config.save();
                        }
                    }
                }
                IDM_TOGGLE_SPELL => {
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.spell_check = !state.config.spell_check;
                            state.engine.options.spell_check = state.config.spell_check;
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
    let icon_v = create_v_icon(&config);
    let icon_e = create_e_icon(&config);
    if !TRAY_PTR.is_null() {
        (*TRAY_PTR).set_icons(icon_v, icon_e);
    }
    apply_config(config);
}

unsafe fn toggle_mode(_hwnd: HWND) {
    if let Ok(mut lock) = STATE.lock() {
        if let Some(state) = lock.as_mut() {
            // Tray action is the manual override path in excluded apps.
            state.toggle_language();
            state.config.save();
            if state.config.notification_sound {
                crate::hook::play_notification_sound();
            }
            let effective = state.effective_enabled();
            if !TRAY_PTR.is_null() {
                (*TRAY_PTR).update_status(effective);
            }
        }
    }
}
