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
            let quoted = format!("\"{}\"", exe_path.to_string_lossy());
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

/// Remove all application shortcuts from Start Menu, Desktop, and Startup registry,
/// then signal Windows Shell to purge stale icon caches and refresh UI immediately.
pub unsafe fn clean_all_shortcuts_and_cache() {
    // 1. Remove Startup Run entry
    let _ = set_start_with_windows(false);

    // 2. Collect shortcut file targets across user and machine profiles
    let mut candidate_paths: Vec<std::path::PathBuf> = Vec::new();

    if let Ok(appdata) = std::env::var("APPDATA") {
        let p = std::path::Path::new(&appdata);
        candidate_paths.push(p.join(r"Microsoft\Windows\Start Menu\Programs\VietFlow.lnk"));
        candidate_paths.push(p.join(r"Microsoft\Windows\Start Menu\Programs\VietFlow Team\VietFlow.lnk"));
    }
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        let p = std::path::Path::new(&userprofile);
        candidate_paths.push(p.join(r"Desktop\VietFlow.lnk"));
        candidate_paths.push(p.join(r"OneDrive\Desktop\VietFlow.lnk"));
    }
    if let Ok(public) = std::env::var("PUBLIC") {
        let p = std::path::Path::new(&public);
        candidate_paths.push(p.join(r"Desktop\VietFlow.lnk"));
    }
    if let Ok(programdata) = std::env::var("ProgramData") {
        let p = std::path::Path::new(&programdata);
        candidate_paths.push(p.join(r"Microsoft\Windows\Start Menu\Programs\VietFlow.lnk"));
        candidate_paths.push(p.join(r"Microsoft\Windows\Start Menu\Programs\VietFlow Team\VietFlow.lnk"));
    }

    for path in &candidate_paths {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    // 3. Remove orphaned Start Menu directories if empty
    if let Ok(appdata) = std::env::var("APPDATA") {
        let team_folder = std::path::Path::new(&appdata).join(r"Microsoft\Windows\Start Menu\Programs\VietFlow Team");
        let _ = std::fs::remove_dir(&team_folder);
    }
    if let Ok(programdata) = std::env::var("ProgramData") {
        let team_folder = std::path::Path::new(&programdata).join(r"Microsoft\Windows\Start Menu\Programs\VietFlow Team");
        let _ = std::fs::remove_dir(&team_folder);
    }

    // 4. Force Windows Shell icon cache refresh
    windows_sys::Win32::UI::Shell::SHChangeNotify(
        windows_sys::Win32::UI::Shell::SHCNE_ASSOCCHANGED as i32,
        windows_sys::Win32::UI::Shell::SHCNF_IDLIST,
        std::ptr::null(),
        std::ptr::null(),
    );
}
