// VietFlow Windows - Foreground Process Detection & Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::config::AppProfile;
use std::collections::HashMap;
use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId;

pub unsafe fn get_process_name_for_window(hwnd: HWND) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 {
        return None;
    }

    let hprocess = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if hprocess.is_null() {
        return None;
    }

    let mut buffer = [0u16; 1024];
    let mut len = buffer.len() as u32;

    let ok = QueryFullProcessImageNameW(hprocess, 0, buffer.as_mut_ptr(), &mut len);
    CloseHandle(hprocess);

    if ok == 0 || len == 0 {
        return None;
    }

    let full_path = String::from_utf16_lossy(&buffer[..len as usize]);
    let file_name = std::path::Path::new(&full_path)
        .file_name()
        .map(|f| f.to_string_lossy().to_string());

    file_name
}

pub fn app_name_matches(current: &str, configured: &str) -> bool {
    let proc = current.trim().to_lowercase();
    let app = configured.trim().to_lowercase();
    if proc.is_empty() || app.is_empty() {
        return false;
    }
    let proc_stem = proc.strip_suffix(".exe").unwrap_or(&proc);
    let app_stem = app.strip_suffix(".exe").unwrap_or(&app);
    app == proc || app_stem == proc_stem
}

fn is_known_ime_name(name: &str) -> bool {
    matches!(
        name,
        "unikey.exe"
            | "unikeynt.exe"
            | "unikey64.exe"
            | "evkey.exe"
            | "evkey64.exe"
            | "openkey.exe"
            | "openkey64.exe"
            | "vietkey.exe"
            | "vietkey64.exe"
            | "gotiengviet.exe"
            | "viettype.exe"
            | "viettype64.exe"
            | "vietpad.exe"
            | "winvnkey.exe"
            | "vpskeys.exe"
            | "vietuni.exe"
            | "avavietkey.exe"
    )
}

/// Known Vietnamese IME processes that are running right now. The current
/// VietFlow process is excluded so startup does not warn about itself.
pub fn known_ime_conflicts() -> Vec<String> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
    if snapshot == INVALID_HANDLE_VALUE {
        return Vec::new();
    }

    let mut entry: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
    let mut found = Vec::new();

    unsafe {
        if Process32FirstW(snapshot, &mut entry) != 0 {
            loop {
                let name = String::from_utf16_lossy(&entry.szExeFile)
                    .trim_end_matches('\0')
                    .to_ascii_lowercase();
                if name != "vietflow.exe"
                    && is_known_ime_name(&name)
                    && !found.iter().any(|item| item == &name)
                {
                    found.push(name);
                }
                if Process32NextW(snapshot, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snapshot);
    }
    found
}

pub fn is_app_excluded(current_process: Option<&str>, excluded_apps: &[String]) -> bool {
    match current_process {
        Some(proc) => excluded_apps.iter().any(|app| app_name_matches(proc, app)),
        None => false,
    }
}

pub fn profile_for_app<'a>(
    current_process: Option<&str>,
    profiles: &'a HashMap<String, AppProfile>,
) -> Option<&'a AppProfile> {
    let proc = current_process?;
    profiles
        .iter()
        .find(|(app, _)| app_name_matches(proc, app))
        .map(|(_, profile)| profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_app_excluded() {
        let list = vec![
            "valorant.exe".to_string(),
            "cs2".to_string(),
            "League of Legends.exe".to_string(),
        ];

        assert!(is_app_excluded(Some("valorant.exe"), &list));
        assert!(is_app_excluded(Some("VALORANT.EXE"), &list));
        assert!(is_app_excluded(Some("valorant"), &list));
        assert!(is_app_excluded(Some("cs2.exe"), &list));
        assert!(is_app_excluded(Some("CS2"), &list));
        assert!(is_app_excluded(Some("League of Legends.exe"), &list));
        assert!(!is_app_excluded(Some("notepad.exe"), &list));
        assert!(!is_app_excluded(None, &list));
        assert!(!is_app_excluded(Some(""), &list));
    }

    #[test]
    fn test_known_ime_names() {
        assert!(is_known_ime_name("evkey64.exe"));
        assert!(is_known_ime_name("openkey.exe"));
        assert!(is_known_ime_name("unikey.exe"));
        assert!(!is_known_ime_name("vietflow.exe"));
        assert!(!is_known_ime_name("notepad.exe"));
    }

    #[test]
    fn test_profile_for_app_uses_same_matching_rules() {
        let mut profiles = HashMap::new();
        profiles.insert(
            "Code.exe".to_string(),
            AppProfile {
                injection_mode: Some(4),
                ..Default::default()
            },
        );
        profiles.insert(
            "cs2".to_string(),
            AppProfile {
                allow_macro: Some(true),
                ..Default::default()
            },
        );

        assert_eq!(
            profile_for_app(Some("code"), &profiles).and_then(|p| p.injection_mode),
            Some(4)
        );
        assert_eq!(
            profile_for_app(Some("CS2.EXE"), &profiles).and_then(|p| p.allow_macro),
            Some(true)
        );
        assert!(profile_for_app(Some("notepad.exe"), &profiles).is_none());
        assert!(profile_for_app(None, &profiles).is_none());
    }
}
