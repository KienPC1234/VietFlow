// VietFlow Windows - Foreground Process Detection & Exclusion (Game Mode)
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
        Some(p) => p,
        None => return false,
    };

    let proc_lower = proc.to_lowercase();
    excluded_apps
        .iter()
        .any(|app| app.to_lowercase() == proc_lower)
}
