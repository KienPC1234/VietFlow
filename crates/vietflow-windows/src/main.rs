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

use std::ffi::c_void;
use std::mem::zeroed;
use std::ptr::null_mut;
use vietflow_core::models::{CharsetId, InputMethod};
use windows_sys::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, FindWindowW, GetMessageW, PostMessageW,
    PostQuitMessage, RegisterClassW, TranslateMessage, MSG, WNDCLASSW, WM_COMMAND, WM_DESTROY,
    WM_LBUTTONUP, WM_RBUTTONUP,
};

use crate::config::AppConfig;
use crate::hook::{install_hook, uninstall_hook, STATE};
use crate::icon::{create_e_icon, create_v_icon};
use crate::startup::set_start_with_windows;
use crate::tray::{
    to_wide, TrayIcon, IDM_CHARSET_COMPOSITE, IDM_CHARSET_TCVN3, IDM_CHARSET_UNICODE,
    IDM_CHARSET_VNI, IDM_CONVERT_TOOL, IDM_EXIT, IDM_METHOD_SIMPLE_TELEX, IDM_METHOD_TELEX,
    IDM_METHOD_VNI, IDM_SETTINGS, IDM_TOGGLE, IDM_TOGGLE_MACRO, IDM_TOGGLE_SPELL,
    IDM_TOGGLE_STARTUP, WM_CONVERT, WM_SETTINGS, WM_TRAYICON,
};

extern "system" {
    fn CreateMutexW(
        lpMutexAttributes: *mut c_void,
        bInitialOwner: i32,
        lpName: *const u16,
    ) -> HANDLE;
}

static mut TRAY_PTR: *mut TrayIcon = null_mut();

fn spawn_gui(convert_mode: bool) {
    if let Ok(exe) = std::env::current_exe() {
        let mut cmd = std::process::Command::new(exe);
        if convert_mode {
            cmd.arg("--convert");
        } else {
            cmd.arg("--gui");
        }
        let _ = cmd.spawn();
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--gui" || a == "-g") {
        let _ = vietflow_gui::run_gui(false);
        return;
    }
    if args.iter().any(|a| a == "--convert" || a == "-c") {
        let _ = vietflow_gui::run_gui(true);
        return;
    }

    unsafe {
        // Enforce single instance for daemon
        let mutex_name = to_wide("Global\\VietFlowSingleInstanceMutex");
        let mutex = CreateMutexW(null_mut(), 1, mutex_name.as_ptr());
        if mutex.is_null() || GetLastError() == ERROR_ALREADY_EXISTS {
            let class_name = to_wide("VietFlowMessageWindowClass");
            let window_title = to_wide("VietFlowDaemon");
            let hwnd = FindWindowW(class_name.as_ptr(), window_title.as_ptr());
            if !hwnd.is_null() {
                PostMessageW(hwnd, WM_SETTINGS, 0, 0);
            }
            return;
        }

        let config = AppConfig::load();

        let class_name = to_wide("VietFlowMessageWindowClass");
        let mut wc: WNDCLASSW = zeroed();
        wc.lpfnWndProc = Some(wnd_proc);
        wc.lpszClassName = class_name.as_ptr();
        RegisterClassW(&wc);

        let window_title = to_wide("VietFlowDaemon");
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

        let icon_v = create_v_icon();
        let icon_e = create_e_icon();
        let mut tray = TrayIcon::new(hwnd, icon_v, icon_e);
        tray.update_status(config.enabled);

        TRAY_PTR = &mut tray;

        install_hook(hwnd, config);

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
            if event == WM_LBUTTONUP {
                toggle_mode(hwnd);
            } else if event == WM_RBUTTONUP {
                if let Ok(lock) = STATE.lock() {
                    if let Some(state) = lock.as_ref() {
                        if !TRAY_PTR.is_null() {
                            (*TRAY_PTR).show_context_menu(
                                state.config.enabled,
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
                IDM_METHOD_TELEX | IDM_METHOD_SIMPLE_TELEX | IDM_METHOD_VNI => {
                    let m = match id {
                        IDM_METHOD_TELEX => 0,
                        IDM_METHOD_SIMPLE_TELEX => 1,
                        IDM_METHOD_VNI => 2,
                        _ => 0,
                    };
                    if let Ok(mut lock) = STATE.lock() {
                        if let Some(state) = lock.as_mut() {
                            state.config.method = m;
                            state.engine.options.method = match m {
                                0 => InputMethod::Telex,
                                1 => InputMethod::SimpleTelex,
                                2 => InputMethod::Vni,
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
                IDM_EXIT => {
                    PostQuitMessage(0);
                }
                _ => {}
            }
            0
        }
        WM_DESTROY => {
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, w_param, l_param),
    }
}

unsafe fn toggle_mode(_hwnd: HWND) {
    if let Ok(mut lock) = STATE.lock() {
        if let Some(state) = lock.as_mut() {
            state.config.enabled = !state.config.enabled;
            state.engine.reset();
            state.config.save();
            let enabled = state.config.enabled;
            if !TRAY_PTR.is_null() {
                (*TRAY_PTR).update_status(enabled);
            }
        }
    }
}
