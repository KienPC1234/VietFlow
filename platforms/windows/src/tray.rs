// VietFlow Windows - System Tray Manager
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::mem::zeroed;
use std::ptr::null_mut;
use windows_sys::Win32::Foundation::{HWND, POINT};
use windows_sys::Win32::UI::Shell::{
    Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NIM_MODIFY,
    NOTIFYICONDATAW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, DestroyIcon, DestroyMenu, GetCursorPos,
    PostMessageW, SetForegroundWindow, TrackPopupMenu, HICON,
    MF_CHECKED, MF_POPUP, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    WM_NULL,
};

pub use crate::constants::*;

pub struct TrayIcon {
    pub hwnd: HWND,
    nid: NOTIFYICONDATAW,
    icon_v: HICON,
    icon_e: HICON,
    icon_app: HICON,
    pub is_tsf_mode: bool,
}

impl TrayIcon {
    pub unsafe fn new(
        hwnd: HWND,
        icon_v: HICON,
        icon_e: HICON,
        icon_app: HICON,
        is_tsf_mode: bool,
    ) -> Self {
        let mut nid: NOTIFYICONDATAW = zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        nid.uCallbackMessage = WM_TRAYICON;
        nid.hIcon = if is_tsf_mode { icon_app } else { icon_v };

        let tip = if is_tsf_mode {
            "VietFlow - Cài đặt [IME]\0"
        } else {
            "VietFlow - Bộ gõ tiếng Việt\0"
        };
        let mut tip_buf = [0u16; 128];
        for (i, c) in tip.encode_utf16().take(127).enumerate() {
            tip_buf[i] = c;
        }
        std::ptr::copy_nonoverlapping(
            tip_buf.as_ptr(),
            std::ptr::addr_of_mut!(nid.szTip) as *mut u16,
            128,
        );

        Shell_NotifyIconW(NIM_ADD, &nid);

        Self {
            hwnd,
            nid,
            icon_v,
            icon_e,
            icon_app,
            is_tsf_mode,
        }
    }

    pub unsafe fn set_icons(&mut self, icon_v: HICON, icon_e: HICON, icon_app: HICON) {
        let old_v = self.icon_v;
        let old_e = self.icon_e;
        let old_app = self.icon_app;
        self.icon_v = icon_v;
        self.icon_e = icon_e;
        self.icon_app = icon_app;
        if !old_v.is_null() && old_v != icon_v {
            DestroyIcon(old_v);
        }
        if !old_e.is_null() && old_e != icon_e {
            DestroyIcon(old_e);
        }
        if !old_app.is_null() && old_app != icon_app {
            DestroyIcon(old_app);
        }
    }

    pub unsafe fn set_tsf_mode(&mut self, is_tsf: bool) {
        self.is_tsf_mode = is_tsf;
    }

    pub unsafe fn update_status(&mut self, is_vietnamese: bool) {
        if self.is_tsf_mode {
            self.nid.hIcon = self.icon_app;
            let tip_str = "VietFlow - Cài đặt [IME]\0";
            let mut tip_buf = [0u16; 128];
            for (i, c) in tip_str.encode_utf16().take(127).enumerate() {
                tip_buf[i] = c;
            }
            std::ptr::copy_nonoverlapping(
                tip_buf.as_ptr(),
                std::ptr::addr_of_mut!(self.nid.szTip) as *mut u16,
                128,
            );
        } else {
            self.nid.hIcon = if is_vietnamese { self.icon_v } else { self.icon_e };
            let tip_str = if is_vietnamese {
                "VietFlow - Tiếng Việt [V]\0"
            } else {
                "VietFlow - English [E]\0"
            };
            let mut tip_buf = [0u16; 128];
            for (i, c) in tip_str.encode_utf16().take(127).enumerate() {
                tip_buf[i] = c;
            }
            std::ptr::copy_nonoverlapping(
                tip_buf.as_ptr(),
                std::ptr::addr_of_mut!(self.nid.szTip) as *mut u16,
                128,
            );
        }
        Shell_NotifyIconW(NIM_MODIFY, &self.nid);
    }

    pub unsafe fn show_context_menu(
        &self,
        is_vietnamese: bool,
        method: u8,
        charset: u8,
        enable_macro: bool,
        spell_check: bool,
        start_with_windows: bool,
        hotkey_mode: &str,
        tsf_present: bool,
        use_tsf: bool,
        tsf_registered: bool,
    ) {
        let hmenu = CreatePopupMenu();
        let hmenu_method = CreatePopupMenu();
        let hmenu_charset = CreatePopupMenu();

        let check_flag = |cond: bool| if cond { MF_CHECKED } else { MF_UNCHECKED };

        let hotkey_label = match hotkey_mode {
            "CtrlShift" => "Ctrl + Shift",
            "AltZ" => "Alt + Z",
            "CtrlSpace" => "Ctrl + Space",
            "ShiftSpace" => "Shift + Space",
            "AltShift" => "Alt + Shift",
            "CapsLock" => "Caps Lock",
            "WinSpace" => "Win + Space",
            other => other,
        };

        // Toggle item
        let toggle_text = if is_vietnamese {
            to_wide(&format!("Chuyển sang English [{hotkey_label}]"))
        } else {
            to_wide(&format!("Chuyển sang Tiếng Việt [{hotkey_label}]"))
        };
        AppendMenuW(hmenu, MF_STRING, IDM_TOGGLE, toggle_text.as_ptr());
        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());

        // Method submenu
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 0),
            IDM_METHOD_TELEX,
            to_wide("Telex").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 1),
            IDM_METHOD_VNI,
            to_wide("VNI").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 2),
            IDM_METHOD_VIQR,
            to_wide("VIQR").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 3),
            IDM_METHOD_SIMPLE_TELEX,
            to_wide("Telex đơn giản").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 4),
            IDM_METHOD_SIMPLE_TELEX2,
            to_wide("Telex đơn giản 2").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 5),
            IDM_METHOD_TELEX_VNI,
            to_wide("Telex + VNI").as_ptr(),
        );
        AppendMenuW(
            hmenu_method,
            MF_STRING | check_flag(method == 6),
            IDM_METHOD_CUSTOM,
            to_wide("Tự định nghĩa").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_POPUP,
            hmenu_method as usize,
            to_wide("Kiểu gõ").as_ptr(),
        );

        // Charset submenu
        AppendMenuW(
            hmenu_charset,
            MF_STRING | check_flag(charset == 0),
            IDM_CHARSET_UNICODE,
            to_wide("Unicode dựng sẵn").as_ptr(),
        );
        AppendMenuW(
            hmenu_charset,
            MF_STRING | check_flag(charset == 1),
            IDM_CHARSET_COMPOSITE,
            to_wide("Unicode tổ hợp").as_ptr(),
        );
        AppendMenuW(
            hmenu_charset,
            MF_STRING | check_flag(charset == 2),
            IDM_CHARSET_TCVN3,
            to_wide("TCVN3 (ABC)").as_ptr(),
        );
        AppendMenuW(
            hmenu_charset,
            MF_STRING | check_flag(charset == 3),
            IDM_CHARSET_VNI,
            to_wide("VNI Windows").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_POPUP,
            hmenu_charset as usize,
            to_wide("Bảng mã").as_ptr(),
        );

        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());

        // Quick feature toggles
        AppendMenuW(
            hmenu,
            MF_STRING | check_flag(enable_macro),
            IDM_TOGGLE_MACRO,
            to_wide("Bật gõ tắt (Macro)").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING | check_flag(spell_check),
            IDM_TOGGLE_SPELL,
            to_wide("Kiểm tra chính tả").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING | check_flag(start_with_windows),
            IDM_TOGGLE_STARTUP,
            to_wide("Khởi động cùng Windows").as_ptr(),
        );

        if tsf_present {
            let hmenu_tsf = CreatePopupMenu();
            AppendMenuW(
                hmenu_tsf,
                MF_STRING | check_flag(use_tsf),
                IDM_TSF_ENABLE,
                to_wide("Bật chế độ TSF").as_ptr(),
            );
            let reg_label = if tsf_registered {
                "Hủy đăng ký TSF TIP (Unregister)..."
            } else {
                "Cài đặt / Đăng ký TSF TIP (Register)..."
            };
            AppendMenuW(
                hmenu_tsf,
                MF_STRING,
                IDM_TSF_REGISTER,
                to_wide(reg_label).as_ptr(),
            );
            AppendMenuW(
                hmenu,
                MF_POPUP,
                hmenu_tsf as usize,
                to_wide("Chế độ TSF IME").as_ptr(),
            );
        }

        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_CONVERT_TOOL,
            to_wide("Công cụ chuyển mã... [Ctrl+Shift+F9]").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_SETTINGS,
            to_wide("Bảng điều khiển... [Ctrl+Shift+F5]").as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_RESTART_ADMIN,
            to_wide("Chạy lại với quyền quản trị...").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_CHECK_UPDATE,
            to_wide("Kiểm tra cập nhật...").as_ptr(),
        );
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_ABOUT,
            to_wide("Thông tin (About)...").as_ptr(),
        );
        AppendMenuW(hmenu, MF_SEPARATOR, 0, null_mut());
        AppendMenuW(
            hmenu,
            MF_STRING,
            IDM_EXIT,
            to_wide("Kết thúc (Thoát)").as_ptr(),
        );

        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        SetForegroundWindow(self.hwnd);
        TrackPopupMenu(
            hmenu,
            TPM_LEFTALIGN | TPM_BOTTOMALIGN,
            pt.x,
            pt.y,
            0,
            self.hwnd,
            null_mut(),
        );
        PostMessageW(self.hwnd, WM_NULL, 0, 0);
        DestroyMenu(hmenu);
    }
}

impl Drop for TrayIcon {
    fn drop(&mut self) {
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.nid);
            if !self.icon_v.is_null() {
                DestroyIcon(self.icon_v);
            }
            if !self.icon_e.is_null() {
                DestroyIcon(self.icon_e);
            }
            if !self.icon_app.is_null() {
                DestroyIcon(self.icon_app);
            }
        }
    }
}

pub fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}
