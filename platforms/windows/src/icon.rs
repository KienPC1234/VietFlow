// VietFlow Windows - High-DPI Tray Icon Loader
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateIconFromResourceEx, GetSystemMetrics, HICON, SM_CXSMICON, SM_CYSMICON,
};

static ICON_V_BYTES: &[u8] = include_bytes!("../../../assets/icon-v.png");
static ICON_E_BYTES: &[u8] = include_bytes!("../../../assets/icon-e.png");

pub fn create_v_icon() -> HICON {
    create_icon_from_png(ICON_V_BYTES)
}

pub fn create_e_icon() -> HICON {
    create_icon_from_png(ICON_E_BYTES)
}

fn create_icon_from_png(bytes: &[u8]) -> HICON {
    unsafe {
        let size_x = GetSystemMetrics(SM_CXSMICON);
        let size_y = GetSystemMetrics(SM_CYSMICON);
        let cx = if size_x > 0 { size_x } else { 16 };
        let cy = if size_y > 0 { size_y } else { 16 };

        let hicon = CreateIconFromResourceEx(
            bytes.as_ptr(),
            bytes.len() as u32,
            1,
            0x00030000,
            cx,
            cy,
            0,
        );

        if !hicon.is_null() {
            hicon
        } else {
            CreateIconFromResourceEx(
                bytes.as_ptr(),
                bytes.len() as u32,
                1,
                0x00030000,
                0,
                0,
                0,
            )
        }
    }
}
