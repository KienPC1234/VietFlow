// VietFlow Windows - Foreground Process Detection & Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows_sys::Win32::Foundation::CloseHandle;
use windows_sys::Win32::Foundation::HWND;
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

pub fn is_app_excluded(current_process: Option<&str>, excluded_apps: &[String]) -> bool {
    let proc = match current_process {
        Some(p) => p.trim().to_lowercase(),
        None => return false,
    };
    if proc.is_empty() {
        return false;
    }
    let proc_stem = proc.strip_suffix(".exe").unwrap_or(&proc);

    excluded_apps.iter().any(|app| {
        let app_trim = app.trim().to_lowercase();
        if app_trim.is_empty() {
            return false;
        }
        let app_stem = app_trim.strip_suffix(".exe").unwrap_or(&app_trim);
        app_trim == proc || app_stem == proc_stem
    })
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
}
