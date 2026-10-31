// VietFlow Windows - Manifest-based update checker (WinINet)
// Licensed under GNU General Public License v3.0 (GPLv3)
//
// The updater reads release metadata packaged with each release:
//   latest.json  -> { "version": "0.2.0", "notes": "...", "url": "..." }
//   version.txt  -> 0.2.0 (fallback)
// It falls back to the GitHub Releases API, then to the releases page.
// No background polling: this only runs when the user asks for it.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_ICONWARNING,
    MB_OK, MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO,
};

use crate::tray::to_wide;
use crate::version::{APP_AUTHORS, APP_LICENSE, APP_NAME, APP_VERSION, GITHUB_REPO_URL};

static IS_CHECKING: AtomicBool = AtomicBool::new(false);

const RELEASES_URL: &str = "https://github.com/KienPC1234/VietFlow/releases";
const MANIFEST_URL: &str =
    "https://github.com/KienPC1234/VietFlow/releases/latest/download/latest.json";
const VERSION_ASSET_URL: &str =
    "https://github.com/KienPC1234/VietFlow/releases/latest/download/version.txt";
const API_URL: &str = "https://api.github.com/repos/KienPC1234/VietFlow/releases/latest";

const MAX_BODY_BYTES: usize = 64 * 1024;
const NOTES_LIMIT: usize = 900;
const TIMEOUT_MS: u32 = 8_000;

const INTERNET_OPEN_TYPE_DIRECT: u32 = 1;
const INTERNET_OPTION_CONNECT_TIMEOUT: u32 = 2;
const INTERNET_OPTION_SEND_TIMEOUT: u32 = 5;
const INTERNET_OPTION_RECEIVE_TIMEOUT: u32 = 6;

const INTERNET_FLAG_RELOAD: u32 = 0x8000_0000;
const INTERNET_FLAG_NO_CACHE_WRITE: u32 = 0x0400_0000;
const INTERNET_FLAG_SECURE: u32 = 0x0080_0000;
const INTERNET_FLAG_NO_UI: u32 = 0x0000_0200;
const HTTP_QUERY_STATUS_CODE: u32 = 19;
const HTTP_QUERY_FLAG_NUMBER: u32 = 0x2000_0000;

const FETCH_FLAGS: u32 = INTERNET_FLAG_RELOAD
    | INTERNET_FLAG_NO_CACHE_WRITE
    | INTERNET_FLAG_SECURE
    | INTERNET_FLAG_NO_UI;

#[link(name = "wininet")]
extern "system" {
    fn InternetOpenW(
        lpszAgent: *const u16,
        dwAccessType: u32,
        lpszProxy: *const u16,
        lpszProxyBypass: *const u16,
        dwFlags: u32,
    ) -> *mut c_void;

    fn InternetOpenUrlW(
        hInternet: *mut c_void,
        lpszUrl: *const u16,
        lpszHeaders: *const u16,
        dwHeadersLength: u32,
        dwFlags: u32,
        dwContext: usize,
    ) -> *mut c_void;

    fn InternetReadFile(
        hFile: *mut c_void,
        lpBuffer: *mut u8,
        dwNumberOfBytesToRead: u32,
        lpdwNumberOfBytesRead: *mut u32,
    ) -> i32;

    fn InternetSetOptionW(
        hInternet: *mut c_void,
        dwOption: u32,
        lpBuffer: *mut c_void,
        dwBufferLength: u32,
    ) -> i32;

    fn HttpQueryInfoW(
        hRequest: *mut c_void,
        dwInfoLevel: u32,
        lpBuffer: *mut c_void,
        lpdwBufferLength: *mut u32,
        lpdwIndex: *mut u32,
    ) -> i32;

    fn InternetCloseHandle(hInternet: *mut c_void) -> i32;
}

/// Closes a WinINet handle when it goes out of scope.
struct InternetHandle(*mut c_void);

impl Drop for InternetHandle {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                InternetCloseHandle(self.0);
            }
        }
    }
}

#[derive(Debug)]
struct FetchError {
    message: String,
    status: Option<u32>,
}

impl FetchError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status: None,
        }
    }

    fn http(status: u32) -> Self {
        Self {
            message: format!("GitHub trả về HTTP {status}"),
            status: Some(status),
        }
    }

    fn is_not_found(&self) -> bool {
        self.status == Some(404)
    }
}

#[derive(Debug)]
pub(crate) struct ReleaseInfo {
    version: String,
    notes: String,
    url: String,
    download: Option<String>,
}

fn show_msg(text: &str, title: &str, flags: u32) -> i32 {
    let wide_text = to_wide(text);
    let wide_title = to_wide(title);
    // Use NULL owner on worker thread so MessageBox does not hijack foreign active windows
    let flags = flags | MB_SETFOREGROUND | MB_TOPMOST;
    unsafe { MessageBoxW(0 as _, wide_text.as_ptr(), wide_title.as_ptr(), flags) }
}

/// Show About dialog with app version and repository link
pub fn show_about_dialog() {
    let msg = format!(
        "{} - Bộ gõ tiếng Việt hiện đại\n\
         Phiên bản: v{}\n\
         Bản quyền (c) 2026 {}\n\
         Giấy phép: {}\n\n\
         Trang chủ & Mã nguồn:\n{}",
        APP_NAME, APP_VERSION, APP_AUTHORS, APP_LICENSE, GITHUB_REPO_URL
    );
    show_msg(&msg, "Thông tin VietFlow", MB_OK | MB_ICONINFORMATION);
}

struct ScopeGuard<F: FnOnce()>(Option<F>);
impl<F: FnOnce()> Drop for ScopeGuard<F> {
    fn drop(&mut self) {
        if let Some(f) = self.0.take() {
            f();
        }
    }
}

fn open_browser(url: &str) {
    let wide_url = to_wide(url);
    unsafe {
        windows_sys::Win32::UI::Shell::ShellExecuteW(
            0 as _,
            to_wide("open").as_ptr(),
            wide_url.as_ptr(),
            std::ptr::null(),
            std::ptr::null(),
            1, // SW_SHOWNORMAL
        );
    }
}

unsafe fn set_timeout(handle: *mut c_void, option: u32) {
    if handle.is_null() {
        return;
    }
    let mut timeout = TIMEOUT_MS;
    InternetSetOptionW(
        handle,
        option,
        &mut timeout as *mut u32 as *mut c_void,
        std::mem::size_of::<u32>() as u32,
    );
}

unsafe fn query_http_status(handle: *mut c_void) -> Option<u32> {
    if handle.is_null() {
        return None;
    }
    let mut status = 0u32;
    let mut length = std::mem::size_of::<u32>() as u32;
    let ok = HttpQueryInfoW(
        handle,
        HTTP_QUERY_STATUS_CODE | HTTP_QUERY_FLAG_NUMBER,
        &mut status as *mut u32 as *mut c_void,
        &mut length,
        std::ptr::null_mut(),
    );
    if ok != 0 {
        Some(status)
    } else {
        None
    }
}

/// Fetch a small UTF-8 document. Enforces an 8s timeout and a 64 KiB cap so a
/// broken server can never hang or balloon the updater.
fn fetch_url(url: &str) -> Result<String, FetchError> {
    unsafe {
        let agent = to_wide("VietFlow-Updater/1");
        let session = InternetHandle(InternetOpenW(
            agent.as_ptr(),
            INTERNET_OPEN_TYPE_DIRECT,
            std::ptr::null(),
            std::ptr::null(),
            0,
        ));
        if session.0.is_null() {
            return Err(FetchError::new("Không thể khởi tạo WinINet"));
        }
        set_timeout(session.0, INTERNET_OPTION_CONNECT_TIMEOUT);
        set_timeout(session.0, INTERNET_OPTION_SEND_TIMEOUT);
        set_timeout(session.0, INTERNET_OPTION_RECEIVE_TIMEOUT);

        let wide_url = to_wide(url);
        let headers = to_wide("User-Agent: VietFlow\r\nAccept: application/json, text/plain\r\n");
        let request = InternetHandle(InternetOpenUrlW(
            session.0,
            wide_url.as_ptr(),
            headers.as_ptr(),
            headers.len() as u32 - 1,
            FETCH_FLAGS,
            0,
        ));
        if request.0.is_null() {
            return Err(FetchError::new("Không thể kết nối đến máy chủ cập nhật"));
        }
        set_timeout(request.0, INTERNET_OPTION_SEND_TIMEOUT);
        set_timeout(request.0, INTERNET_OPTION_RECEIVE_TIMEOUT);

        if let Some(status) = query_http_status(request.0) {
            if status != 200 {
                return Err(FetchError::http(status));
            }
        }

        let mut body = Vec::with_capacity(2048);
        let mut buffer = [0u8; 4096];
        loop {
            let mut read = 0u32;
            let ok = InternetReadFile(
                request.0,
                buffer.as_mut_ptr(),
                buffer.len() as u32,
                &mut read,
            );
            if ok == 0 || read == 0 {
                break;
            }
            if body.len() + read as usize > MAX_BODY_BYTES {
                return Err(FetchError::new("Phản hồi từ máy chủ quá lớn"));
            }
            body.extend_from_slice(&buffer[..read as usize]);
        }

        String::from_utf8(body).map_err(|_| FetchError::new("Phản hồi không phải UTF-8"))
    }
}

fn normalize_version(value: &str) -> String {
    value
        .trim()
        .trim_start_matches(|c: char| c == 'v' || c == 'V')
        .to_string()
}

pub fn is_installed_via_msi() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    // 1. Velopack MSI drops `.msi-installed` into the root install directory.
    if let Some(parent) = exe.parent() {
        if parent.join(".msi-installed").exists() {
            return true;
        }
        if let Some(grandparent) = parent.parent() {
            if grandparent.join(".msi-installed").exists() {
                return true;
            }
        }
    }
    // 2. Machine-wide installation under Program Files
    let exe_str = exe.to_string_lossy();
    for var in &["ProgramFiles", "ProgramFiles(x86)", "ProgramW6432"] {
        if let Ok(dir) = std::env::var(var) {
            if !dir.is_empty() && exe_str.starts_with(&dir) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn parse_manifest_for_arch(
    body: &str,
    is_msi: bool,
    arch_key: &str,
) -> Option<ReleaseInfo> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let version = normalize_version(json.get("version")?.as_str()?);
    if version.is_empty() {
        return None;
    }
    let notes = json
        .get("notes")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let url = json
        .get("url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(RELEASES_URL)
        .to_string();

    let download = if is_msi {
        let msi_key = format!("msi_{arch_key}");
        json.get("downloads")
            .and_then(|downloads| downloads.get(&msi_key))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    } else {
        json.get("downloads")
            .and_then(|downloads| downloads.get(arch_key))
            .and_then(|value| value.as_str())
            .filter(|value| !value.is_empty())
            .map(|value| value.to_string())
    };

    Some(ReleaseInfo {
        version,
        notes,
        url,
        download,
    })
}

fn parse_manifest(body: &str) -> Option<ReleaseInfo> {
    let arch_key = if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    };
    parse_manifest_for_arch(body, is_installed_via_msi(), arch_key)
}

fn parse_api_release(body: &str) -> Option<ReleaseInfo> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let version = normalize_version(json.get("tag_name")?.as_str()?);
    if version.is_empty() {
        return None;
    }
    let notes = json
        .get("body")
        .and_then(|value| value.as_str())
        .unwrap_or("")
        .to_string();
    let url = json
        .get("html_url")
        .and_then(|value| value.as_str())
        .filter(|value| !value.is_empty())
        .unwrap_or(RELEASES_URL)
        .to_string();
    Some(ReleaseInfo {
        version,
        notes,
        url,
        download: None,
    })
}

fn fetch_release_info() -> Result<ReleaseInfo, FetchError> {
    // 1) Metadata shipped with the release: no API rate limit, includes notes.
    match fetch_url(MANIFEST_URL) {
        Ok(body) => {
            if let Some(info) = parse_manifest(&body) {
                return Ok(info);
            }
        }
        Err(error) if !error.is_not_found() => return Err(error),
        Err(_) => {}
    }

    // 2) Plain version asset (small, always cacheable).
    match fetch_url(VERSION_ASSET_URL) {
        Ok(body) => {
            let version = normalize_version(&body);
            if !version.is_empty() && version.lines().count() <= 1 {
                return Ok(ReleaseInfo {
                    version: version.clone(),
                    notes: String::new(),
                    url: format!("{RELEASES_URL}/tag/v{version}"),
                    download: None,
                });
            }
        }
        Err(error) if !error.is_not_found() => return Err(error),
        Err(_) => {}
    }

    // 3) Last resort: GitHub API (may be rate limited or disabled).
    match fetch_url(API_URL) {
        Ok(body) => parse_api_release(&body)
            .ok_or_else(|| FetchError::new("Không đọc được thông tin bản phát hành từ GitHub")),
        // A 404 means the repository has no public release yet: report the
        // current version instead of a scary network error.
        Err(error) if error.is_not_found() => Ok(ReleaseInfo {
            version: APP_VERSION.to_string(),
            notes: String::new(),
            url: RELEASES_URL.to_string(),
            download: None,
        }),
        Err(error) => Err(error),
    }
}

fn version_key(value: &str) -> Option<([u32; 3], Option<String>)> {
    let value = normalize_version(value);
    let (core, prerelease) = match value.split_once('-') {
        Some((core, prerelease)) => (core, Some(prerelease.to_string())),
        None => (value.as_str(), None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts
        .next()
        .unwrap_or("0")
        .split('+')
        .next()?
        .parse()
        .ok()?;
    Some(([major, minor, patch], prerelease))
}

fn is_newer_version(remote: &str, local: &str) -> bool {
    let (Some((remote_core, remote_pre)), Some((local_core, local_pre))) =
        (version_key(remote), version_key(local))
    else {
        return false;
    };
    if remote_core != local_core {
        return remote_core > local_core;
    }
    match (remote_pre, local_pre) {
        (None, None) => false,
        (None, Some(_)) => true,  // 1.0.0 is newer than 1.0.0-beta
        (Some(_), None) => false,
        (Some(remote), Some(local)) => remote > local,
    }
}

fn format_notes(notes: &str) -> String {
    notes
        .lines()
        .map(|line| {
            let line = line.trim();
            if let Some(rest) = line.strip_prefix("### ") {
                rest.to_string()
            } else if let Some(rest) = line.strip_prefix("## ") {
                rest.to_string()
            } else if let Some(rest) = line.strip_prefix("- ") {
                format!("• {rest}")
            } else {
                line.to_string()
            }
        })
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn short_notes(notes: &str) -> String {
    let notes = format_notes(notes);
    if notes.is_empty() {
        return String::new();
    }
    let mut result: String = notes.chars().take(NOTES_LIMIT).collect();
    if notes.chars().count() > NOTES_LIMIT {
        result.push('…');
    }
    result
}

fn prompt_update(info: &ReleaseInfo) {
    let notes = short_notes(&info.notes);
    let mut message = format!(
        "Đã có phiên bản mới: v{}\n(Phiên bản bạn đang dùng: v{})\n",
        info.version, APP_VERSION
    );
    if !notes.is_empty() {
        message.push_str("\nCó gì mới:\n");
        message.push_str(&notes);
        message.push_str("\n\n");
    }
    if is_installed_via_msi() {
        message.push_str("Bạn đang sử dụng bản cài đặt MSI. Bạn có muốn tải bản MSI mới nhất để nâng cấp không?");
    } else {
        message.push_str("Bạn có muốn mở trang tải bản cập nhật trên GitHub không?");
    }

    if show_msg(&message, "Cập nhật VietFlow", MB_YESNO | MB_ICONQUESTION) == IDYES {
        let target = info.download.as_deref().unwrap_or(&info.url);
        open_browser(target);
    }
}

fn prompt_up_to_date() {
    let message = format!("VietFlow đã là phiên bản mới nhất (v{}).", APP_VERSION);
    show_msg(&message, "Cập nhật VietFlow", MB_OK | MB_ICONINFORMATION);
}

fn prompt_error(error: &FetchError) {
    let message = format!(
        "Không thể kiểm tra bản cập nhật: {}\n\n\
         Bạn có muốn mở trang phát hành GitHub để kiểm tra thủ công không?",
        error.message
    );
    if show_msg(&message, "Cập nhật VietFlow", MB_YESNO | MB_ICONWARNING) == IDYES {
        open_browser(RELEASES_URL);
    }
}

/// Trigger an on-demand update check in a separate background thread.
/// Updates are never checked automatically or in a background polling loop.
pub fn check_for_updates_interactive() {
    if IS_CHECKING.swap(true, Ordering::SeqCst) {
        show_msg(
            "Hệ thống đang kiểm tra bản cập nhật. Vui lòng đợi trong giây lát...",
            "Cập nhật VietFlow",
            MB_OK | MB_ICONINFORMATION,
        );
        return;
    }

    thread::spawn(|| {
        let _guard = ScopeGuard(Some(|| {
            IS_CHECKING.store(false, Ordering::SeqCst);
        }));

        match fetch_release_info() {
            Ok(info) if is_newer_version(&info.version, APP_VERSION) => prompt_update(&info),
            Ok(_) => prompt_up_to_date(),
            Err(error) => prompt_error(&error),
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compares_versions() {
        assert!(is_newer_version("0.1.1", "0.1.0"));
        assert!(is_newer_version("0.2.0", "0.1.9"));
        assert!(is_newer_version("1.0.0", "0.9.9"));
        assert!(is_newer_version("v0.1.1", "0.1.0"));
        assert!(is_newer_version("0.1.0", "0.1.0-alpha"));
        assert!(!is_newer_version("0.1.0", "0.1.0"));
        assert!(!is_newer_version("0.1.0-alpha", "0.1.0"));
        assert!(!is_newer_version("garbage", "0.1.0"));
    }

    #[test]
    fn parses_manifest() {
        let info = parse_manifest(
            r#"{"version":"v0.2.0","notes":"hello","url":"https://example.test","downloads":{"x64":"https://example.test/x64.exe","x86":"https://example.test/x86.exe"}}"#,
        )
        .expect("manifest should parse");
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.notes, "hello");
        assert_eq!(info.url, "https://example.test");
        assert!(parse_manifest("{}").is_none());
        assert!(parse_manifest("not json").is_none());
    }

    #[test]
    fn parses_manifest_normal_and_msi() {
        let manifest_json = r#"{
            "version": "v0.2.0",
            "notes": "release notes",
            "url": "https://example.test/release",
            "downloads": {
                "x64": "https://example.test/VietFlow-win-x64-Setup.exe",
                "msi_x64": "https://example.test/VietFlow-win-x64.msi",
                "x86": "https://example.test/VietFlow-win-x86-Setup.exe",
                "msi_x86": "https://example.test/VietFlow-win-x86.msi",
                "arm64": "https://example.test/VietFlow-win-arm64-Setup.exe",
                "msi_arm64": "https://example.test/VietFlow-win-arm64.msi"
            }
        }"#;

        // Non-MSI (Setup.exe) resolution
        let setup_x64 = parse_manifest_for_arch(manifest_json, false, "x64").unwrap();
        assert_eq!(
            setup_x64.download.as_deref(),
            Some("https://example.test/VietFlow-win-x64-Setup.exe")
        );

        let setup_x86 = parse_manifest_for_arch(manifest_json, false, "x86").unwrap();
        assert_eq!(
            setup_x86.download.as_deref(),
            Some("https://example.test/VietFlow-win-x86-Setup.exe")
        );

        let setup_arm64 = parse_manifest_for_arch(manifest_json, false, "arm64").unwrap();
        assert_eq!(
            setup_arm64.download.as_deref(),
            Some("https://example.test/VietFlow-win-arm64-Setup.exe")
        );

        // MSI package resolution
        let msi_x64 = parse_manifest_for_arch(manifest_json, true, "x64").unwrap();
        assert_eq!(
            msi_x64.download.as_deref(),
            Some("https://example.test/VietFlow-win-x64.msi")
        );

        let msi_x86 = parse_manifest_for_arch(manifest_json, true, "x86").unwrap();
        assert_eq!(
            msi_x86.download.as_deref(),
            Some("https://example.test/VietFlow-win-x86.msi")
        );

        let msi_arm64 = parse_manifest_for_arch(manifest_json, true, "arm64").unwrap();
        assert_eq!(
            msi_arm64.download.as_deref(),
            Some("https://example.test/VietFlow-win-arm64.msi")
        );
    }

    #[test]
    fn parses_manifest_msi_fallback_to_url() {
        // When manifest does not provide MSI downloads, MSI install must fallback to
        // download = None so it directs the user to the GitHub release page instead of Setup.exe.
        let manifest_no_msi = r#"{
            "version": "v0.2.0",
            "url": "https://example.test/release",
            "downloads": {
                "x64": "https://example.test/VietFlow-win-x64-Setup.exe"
            }
        }"#;

        let info = parse_manifest_for_arch(manifest_no_msi, true, "x64").unwrap();
        assert_eq!(info.download, None);
        assert_eq!(info.url, "https://example.test/release");
    }

    #[test]
    fn parses_api_release() {
        let info = parse_api_release(
            r#"{"tag_name":"v0.3.0","body":"notes","html_url":"https://example.test"}"#,
        )
        .expect("release should parse");
        assert_eq!(info.version, "0.3.0");
        assert_eq!(info.notes, "notes");
        assert_eq!(info.url, "https://example.test");
    }

    #[test]
    fn formats_notes() {
        let formatted = format_notes("### Added\n- One\n\n- Two");
        assert_eq!(formatted, "Added\n• One\n• Two");
    }

    #[test]
    fn truncates_long_notes() {
        let short = short_notes("  hello  ");
        assert_eq!(short, "hello");
        let long = "a".repeat(NOTES_LIMIT + 10);
        let result = short_notes(&long);
        assert_eq!(result.chars().count(), NOTES_LIMIT + 1);
        assert!(result.ends_with('…'));
    }

    #[test]
    #[ignore = "network: requires a published GitHub release"]
    fn live_release_manifest() {
        let info = fetch_release_info().expect("live release manifest");
        assert!(!info.version.is_empty());
        let expected = if cfg!(target_arch = "x86") {
            "win-x86"
        } else {
            "win-x64"
        };
        assert!(
            info.download.as_deref().unwrap_or("").contains(expected),
            "manifest did not provide the {expected} download"
        );
    }
}
