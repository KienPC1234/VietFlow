// VietFlow Windows - Foreground Process Detection & Exclusion
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::config::AppProfile;
use std::collections::HashMap;
use windows_sys::core::BOOL;
use windows_sys::Win32::Foundation::{CloseHandle, HWND, INVALID_HANDLE_VALUE, LPARAM};
use windows_sys::Win32::System::Diagnostics::ToolHelp::{
    CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
    TH32CS_SNAPPROCESS,
};
use windows_sys::Win32::System::Threading::{
    OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{EnumChildWindows, GetWindowThreadProcessId};

struct UwpChildSearch {
    parent_pid: u32,
    child_pid: Option<u32>,
}

unsafe extern "system" fn enum_child_uwp_proc(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let search = &mut *(lparam as *mut UwpChildSearch);
    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid != 0 && pid != search.parent_pid {
        search.child_pid = Some(pid);
        0
    } else {
        1
    }
}

unsafe fn find_uwp_child_pid(hwnd: HWND, parent_pid: u32) -> Option<u32> {
    let mut search = UwpChildSearch {
        parent_pid,
        child_pid: None,
    };
    EnumChildWindows(
        hwnd,
        Some(enum_child_uwp_proc),
        &mut search as *mut UwpChildSearch as LPARAM,
    );
    search.child_pid
}

unsafe fn get_process_name_by_pid(pid: u32) -> Option<String> {
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

pub unsafe fn get_process_name_for_window(hwnd: HWND) -> Option<String> {
    if hwnd.is_null() {
        return None;
    }

    let mut pid: u32 = 0;
    GetWindowThreadProcessId(hwnd, &mut pid);
    if pid == 0 {
        return None;
    }

    let mut name = get_process_name_by_pid(pid);

    // If the window belongs to ApplicationFrameHost.exe (UWP / Windows Store / Xbox GDK wrapper),
    // probe its child hierarchy (such as Windows.UI.Core.CoreWindow) to identify the real child process.
    if let Some(ref proc_name) = name {
        if proc_name.eq_ignore_ascii_case("applicationframehost.exe") {
            if let Some(real_pid) = find_uwp_child_pid(hwnd, pid) {
                if let Some(real_name) = get_process_name_by_pid(real_pid) {
                    name = Some(real_name);
                }
            }
        }
    }

    name
}

/// Match a process name against a pattern using case-insensitive wildcard (* and ?).
pub fn wildcard_match(pattern: &str, text: &str) -> bool {
    let p_bytes = pattern.as_bytes();
    let t_bytes = text.as_bytes();
    let p_len = p_bytes.len();
    let t_len = t_bytes.len();

    let mut p_idx = 0;
    let mut t_idx = 0;
    let mut star_p_idx = None;
    let mut star_t_idx = 0;

    while t_idx < t_len {
        if p_idx < p_len && (p_bytes[p_idx] == b'?' || p_bytes[p_idx] == t_bytes[t_idx]) {
            p_idx += 1;
            t_idx += 1;
        } else if p_idx < p_len && p_bytes[p_idx] == b'*' {
            star_p_idx = Some(p_idx);
            p_idx += 1;
            star_t_idx = t_idx;
        } else if let Some(sp) = star_p_idx {
            p_idx = sp + 1;
            star_t_idx += 1;
            t_idx = star_t_idx;
        } else {
            return false;
        }
    }

    while p_idx < p_len && p_bytes[p_idx] == b'*' {
        p_idx += 1;
    }

    p_idx == p_len
}

pub fn app_name_matches(current: &str, configured: &str, smart_prefix: bool) -> bool {
    let proc = current.trim().to_lowercase();
    let app = configured.trim().to_lowercase();
    if proc.is_empty() || app.is_empty() {
        return false;
    }
    let proc_stem = proc.strip_suffix(".exe").unwrap_or(&proc);
    let app_stem = app.strip_suffix(".exe").unwrap_or(&app);

    // 1. Exact match (full name or stem without .exe)
    if proc == app || proc_stem == app_stem {
        return true;
    }

    // 2. Wildcard pattern matching (* and ?)
    if app.contains('*') || app.contains('?') {
        if wildcard_match(&app, &proc) || wildcard_match(app_stem, proc_stem) {
            return true;
        }
    }

    // 3. Smart prefix matching (only when enabled by user option)
    if smart_prefix && app_stem.len() >= 4 && proc_stem.starts_with(app_stem) {
        return true;
    }

    false
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

pub fn is_app_excluded(
    current_process: Option<&str>,
    excluded_apps: &[String],
    smart_prefix: bool,
) -> bool {
    match current_process {
        Some(proc) => excluded_apps
            .iter()
            .any(|app| app_name_matches(proc, app, smart_prefix)),
        None => false,
    }
}

/// Known terminal, console, and shell processes automatically excluded when `auto_exclude_apps` is enabled.
pub fn is_auto_excluded_app(proc: &str) -> bool {
    let lower = proc.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "cmd.exe"
            | "powershell.exe"
            | "pwsh.exe"
            | "wt.exe"
            | "windowsterminal.exe"
            | "conhost.exe"
            | "bash.exe"
            | "wsl.exe"
            | "putty.exe"
            | "mobaxterm.exe"
            | "xshell.exe"
            | "mintty.exe"
    )
}

pub fn profile_for_app<'a>(
    current_process: Option<&str>,
    profiles: &'a HashMap<String, AppProfile>,
    smart_prefix: bool,
) -> Option<&'a AppProfile> {
    let proc = current_process?;
    profiles
        .iter()
        .find(|(app, _)| app_name_matches(proc, app, smart_prefix))
        .map(|(_, profile)| profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_match() {
        assert!(wildcard_match("roblox*", "robloxplayerbeta.exe"));
        assert!(wildcard_match("roblox*", "robloxstudiobeta.exe"));
        assert!(wildcard_match("roblox*", "roblox.exe"));
        assert!(!wildcard_match("roblox*", "notepad.exe"));
        assert!(wildcard_match("*terminal*", "windowsterminal.exe"));
        assert!(wildcard_match("cs?.exe", "cs2.exe"));
        assert!(!wildcard_match("cs?.exe", "csgo.exe"));
        assert!(wildcard_match("*.exe", "anything.exe"));
        assert!(!wildcard_match("*.exe", "noext"));
    }

    #[test]
    fn test_is_app_excluded() {
        let list = vec![
            "valorant.exe".to_string(),
            "cs2".to_string(),
            "League of Legends.exe".to_string(),
            "roblox*".to_string(),
            "code".to_string(),
        ];

        // Exact match with or without .exe
        assert!(is_app_excluded(Some("valorant.exe"), &list, false));
        assert!(is_app_excluded(Some("VALORANT.EXE"), &list, false));
        assert!(is_app_excluded(Some("valorant"), &list, false));
        assert!(is_app_excluded(Some("cs2.exe"), &list, false));
        assert!(is_app_excluded(Some("CS2"), &list, false));
        assert!(is_app_excluded(Some("League of Legends.exe"), &list, false));
        assert!(is_app_excluded(Some("code.exe"), &list, false));
        assert!(is_app_excluded(Some("code"), &list, false));

        // Wildcard roblox* matches variants even when smart_prefix is false
        assert!(is_app_excluded(Some("RobloxPlayerBeta.exe"), &list, false));
        assert!(is_app_excluded(Some("RobloxStudioBeta.exe"), &list, false));
        assert!(is_app_excluded(Some("roblox.exe"), &list, false));

        // False-positive avoidance: 'code' does NOT match 'codeblocks.exe' when smart_prefix is false!
        assert!(!is_app_excluded(Some("codeblocks.exe"), &list, false));
        assert!(!is_app_excluded(Some("codepath.exe"), &list, false));

        // When smart_prefix is true, 'code' CAN match 'codeblocks.exe'
        assert!(is_app_excluded(Some("codeblocks.exe"), &list, true));

        assert!(!is_app_excluded(Some("notepad.exe"), &list, false));
        assert!(!is_app_excluded(None, &list, false));
        assert!(!is_app_excluded(Some(""), &list, false));
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
            profile_for_app(Some("code"), &profiles, false).and_then(|p| p.injection_mode),
            Some(4)
        );
        assert_eq!(
            profile_for_app(Some("CS2.EXE"), &profiles, false).and_then(|p| p.allow_macro),
            Some(true)
        );
        assert!(profile_for_app(Some("notepad.exe"), &profiles, false).is_none());
        assert!(profile_for_app(None, &profiles, false).is_none());
    }
}
