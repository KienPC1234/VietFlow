// VietFlow Windows - Startup Registry Management
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ptr::null_mut;
use windows_sys::Win32::Foundation::ERROR_SUCCESS;
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegDeleteValueW, RegOpenKeyExW, RegSetValueExW, HKEY, HKEY_CURRENT_USER,
    KEY_SET_VALUE, REG_SZ,
};

use crate::tray::to_wide;

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const VALUE_NAME: &str = "VietFlow";

pub unsafe fn set_start_with_windows(enable: bool) -> bool {
    let subkey = to_wide(RUN_KEY);
    let mut hkey: HKEY = null_mut();

    let res = RegOpenKeyExW(
        HKEY_CURRENT_USER,
        subkey.as_ptr(),
        0,
        KEY_SET_VALUE,
        &mut hkey,
    );

    if res != ERROR_SUCCESS {
        return false;
    }

    let val_name = to_wide(VALUE_NAME);

    let success = if enable {
        if let Ok(exe_path) = std::env::current_exe() {
            // When installed via Velopack or MSI, the app executable is in `current\VietFlow.exe`.
            // Pointing the Run registry entry to the root launcher stub `..\VietFlow.exe` ensures
            // startup executes the latest version through updates.
            let target_exe = if exe_path
                .parent()
                .and_then(|p| p.file_name())
                .map(|f| f.to_string_lossy().eq_ignore_ascii_case("current"))
                .unwrap_or(false)
            {
                if let Some(grandparent) = exe_path.parent().and_then(|p| p.parent()) {
                    let root_stub = grandparent.join("VietFlow.exe");
                    if root_stub.exists() {
                        root_stub
                    } else {
                        exe_path
                    }
                } else {
                    exe_path
                }
            } else {
                exe_path
            };
            let quoted = format!("\"{}\"", target_exe.to_string_lossy());
            let wide_val = to_wide(&quoted);
            let byte_len = (wide_val.len() * std::mem::size_of::<u16>()) as u32;

            RegSetValueExW(
                hkey,
                val_name.as_ptr(),
                0,
                REG_SZ,
                wide_val.as_ptr() as *const u8,
                byte_len,
            ) == ERROR_SUCCESS
        } else {
            false
        }
    } else {
        RegDeleteValueW(hkey, val_name.as_ptr()) == ERROR_SUCCESS
    };

    RegCloseKey(hkey);
    success
}
