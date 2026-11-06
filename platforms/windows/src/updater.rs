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
use windows_sys::Win32::Security::WinTrust::{
    WinVerifyTrust, WINTRUST_ACTION_GENERIC_VERIFY_V2, WINTRUST_DATA, WINTRUST_DATA_0,
    WINTRUST_FILE_INFO, WTD_CHOICE_FILE, WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
    WTD_REVOKE_WHOLECHAIN, WTD_STATEACTION_IGNORE, WTD_UICONTEXT_INSTALL, WTD_UI_NONE,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    MessageBoxW, IDYES, MB_ICONINFORMATION, MB_ICONQUESTION, MB_ICONWARNING, MB_OK,
    MB_SETFOREGROUND, MB_TOPMOST, MB_YESNO,
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
const MIN_MSI_BYTES: usize = 500 * 1024; // At least 500 KB for a valid VietFlow MSI
const MAX_MSI_BYTES: usize = 50 * 1024 * 1024; // 50 MB
const MSI_MAGIC_HEADER: [u8; 8] = [0xD0, 0xCF, 0x11, 0xE0, 0xA1, 0xB1, 0x1A, 0xE1];
const NOTES_LIMIT: usize = 900;
const TIMEOUT_MS: u32 = 8_000;
const DOWNLOAD_TIMEOUT_MS: u32 = 30_000;

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
    /// Only official GitHub Releases API asset digests can authorize execution.
    sha256: Option<[u8; 32]>,
    size: Option<u64>,
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
    set_timeout_ms(handle, option, TIMEOUT_MS);
}

unsafe fn set_timeout_ms(handle: *mut c_void, option: u32, timeout_ms: u32) {
    if handle.is_null() {
        return;
    }
    let mut timeout = timeout_ms;
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

        match query_http_status(request.0) {
            Some(200) => {}
            Some(status) => return Err(FetchError::http(status)),
            None => return Err(FetchError::new("Cannot verify metadata HTTP status")),
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
            if ok == 0 {
                return Err(FetchError::new("Metadata download connection interrupted"));
            }
            if read == 0 {
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

pub fn is_trusted_download_url(url: &str) -> bool {
    // Only the exact release-asset path authorized by the official API can
    // enter the download routine. WinINet follows GitHub's signed CDN
    // redirects internally; arbitrary CDN URLs must not be initial inputs.
    let Some(path) = url.strip_prefix("https://github.com/KienPC1234/VietFlow/releases/download/")
    else {
        return false;
    };
    let Some((tag, filename)) = path.split_once('/') else {
        return false;
    };
    safe_release_tag(tag)
        && matches!(
            filename,
            "VietFlow-win-x64.msi" | "VietFlow-win-x86.msi" | "VietFlow-win-arm64.msi"
        )
}

/// Download a file (e.g. MSI installer) directly to disk using streaming buffer.
/// Implements HTTPS origin verification, fail-closed read checks, minimum size verification,
/// OLE2 compound document magic header validation, and atomic staging.
fn download_file(url: &str, destination: &std::path::Path) -> Result<(), FetchError> {
    use std::io::{Read, Write};

    if !is_trusted_download_url(url) {
        return Err(FetchError::new(
            "Đường dẫn tải tệp không thuộc nguồn tin cậy (chỉ chấp nhận HTTPS GitHub chính thức)",
        ));
    }

    let temp_dest = destination.with_extension("tmp");
    // Never overwrite an existing file or follow a pre-created temporary link.
    let mut created_temp = false;

    let download_result = (|| -> Result<(), FetchError> {
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
                return Err(FetchError::new("Không thể khởi tạo kết nối WinINet"));
            }
            set_timeout_ms(
                session.0,
                INTERNET_OPTION_CONNECT_TIMEOUT,
                DOWNLOAD_TIMEOUT_MS,
            );
            set_timeout_ms(session.0, INTERNET_OPTION_SEND_TIMEOUT, DOWNLOAD_TIMEOUT_MS);
            set_timeout_ms(
                session.0,
                INTERNET_OPTION_RECEIVE_TIMEOUT,
                DOWNLOAD_TIMEOUT_MS,
            );

            let wide_url = to_wide(url);
            let headers =
                to_wide("User-Agent: VietFlow\r\nAccept: application/octet-stream, */*\r\n");
            let request = InternetHandle(InternetOpenUrlW(
                session.0,
                wide_url.as_ptr(),
                headers.as_ptr(),
                headers.len() as u32 - 1,
                FETCH_FLAGS,
                0,
            ));
            if request.0.is_null() {
                return Err(FetchError::new("Không thể mở đường dẫn tải tệp"));
            }
            set_timeout_ms(request.0, INTERNET_OPTION_SEND_TIMEOUT, DOWNLOAD_TIMEOUT_MS);
            set_timeout_ms(
                request.0,
                INTERNET_OPTION_RECEIVE_TIMEOUT,
                DOWNLOAD_TIMEOUT_MS,
            );

            match query_http_status(request.0) {
                Some(200) => {}
                Some(status) => return Err(FetchError::http(status)),
                None => return Err(FetchError::new("Cannot verify MSI HTTP status")),
            }

            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(&temp_dest)
                .map_err(|e| FetchError::new(format!("Không thể tạo tệp tạm: {e}")))?;

            created_temp = true;
            let mut buffer = [0u8; 64 * 1024];
            let mut total_bytes: usize = 0;
            loop {
                let mut read = 0u32;
                let ok = InternetReadFile(
                    request.0,
                    buffer.as_mut_ptr(),
                    buffer.len() as u32,
                    &mut read,
                );
                if ok == 0 {
                    let err = windows_sys::Win32::Foundation::GetLastError();
                    return Err(FetchError::new(format!(
                        "Lỗi mạng khi tải tệp (mã lỗi {err})"
                    )));
                }
                if read == 0 {
                    break;
                }
                total_bytes += read as usize;
                if total_bytes > MAX_MSI_BYTES {
                    return Err(FetchError::new("Tệp tải về vượt quá giới hạn cho phép"));
                }
                file.write_all(&buffer[..read as usize])
                    .map_err(|e| FetchError::new(format!("Lỗi ghi tệp cài đặt: {e}")))?;
            }
            file.flush()
                .map_err(|e| FetchError::new(format!("Lỗi ghi tệp cài đặt: {e}")))?;

            file.sync_all()
                .map_err(|e| FetchError::new(format!("Cannot sync MSI: {e}")))?;

            if total_bytes < MIN_MSI_BYTES {
                return Err(FetchError::new(
                    "Tệp tải về bị cắt ngắn hoặc dung lượng không hợp lệ",
                ));
            }

            // Verify OLE2 / MSI Compound File magic header
            drop(file);
            let mut header = [0u8; 8];
            let mut f = std::fs::File::open(&temp_dest)
                .map_err(|e| FetchError::new(format!("Không thể đọc lại tệp cài đặt: {e}")))?;
            if f.read_exact(&mut header).is_err() || header != MSI_MAGIC_HEADER {
                return Err(FetchError::new(
                    "Tệp tải về không phải định dạng MSI hợp lệ",
                ));
            }

            Ok(())
        }
    })();

    match download_result {
        Ok(()) => match std::fs::rename(&temp_dest, destination) {
            Ok(()) => Ok(()),
            Err(err) => {
                let _ = std::fs::remove_file(&temp_dest);
                Err(FetchError::new(format!(
                    "Cannot finalize staged MSI: {err}"
                )))
            }
        },
        Err(e) => {
            if created_temp {
                let _ = std::fs::remove_file(&temp_dest);
            }
            // destination remains untouched until a verified staging rename.
            Err(e)
        }
    }
}

fn normalize_version(value: &str) -> String {
    value.trim().trim_start_matches(['v', 'V']).to_string()
}

pub fn is_installed_via_msi() -> bool {
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let Some(dir) = exe.parent() else {
        return false;
    };
    // Explicit installation marker deployed by native MSI installer (.installed / msi.tag)
    dir.join(".installed").is_file() || dir.join("msi.tag").is_file()
}

/// Keep all release-page navigations inside the official VietFlow repository.
/// Release metadata is for display, never an authority to redirect users.
fn safe_release_tag(tag: &str) -> bool {
    !tag.is_empty()
        && tag.len() <= 128
        && !tag.contains("..")
        && tag
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
}

fn is_official_release_page(url: &str) -> bool {
    if url == RELEASES_URL {
        return true;
    }
    url.strip_prefix(RELEASES_URL)
        .and_then(|suffix| suffix.strip_prefix("/tag/"))
        .is_some_and(safe_release_tag)
}

pub(crate) fn parse_manifest_for_arch(
    body: &str,
    is_msi: bool,
    arch_key: &str,
) -> Option<ReleaseInfo> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let version = normalize_version(json.get("version")?.as_str()?);
    if version_key(&version).is_none() || !safe_release_tag(&format!("v{version}")) {
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
        .filter(|value| is_official_release_page(value))
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
        // Portable mode does not auto-download binary; directs user to release page
        None
    };

    Some(ReleaseInfo {
        version,
        notes,
        url,
        download,
        sha256: None,
        size: None,
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

fn runtime_arch() -> &'static str {
    if cfg!(target_arch = "x86") {
        "x86"
    } else if cfg!(target_arch = "aarch64") {
        "arm64"
    } else {
        "x64"
    }
}

fn parse_sha256_digest(value: &str) -> Option<[u8; 32]> {
    let hex = value.strip_prefix("sha256:")?;
    if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let mut digest = [0u8; 32];
    for (i, byte) in digest.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&hex[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(digest)
}

fn parse_api_release_for_arch(body: &str, is_msi: bool, arch: &str) -> Option<ReleaseInfo> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;
    let tag = json.get("tag_name")?.as_str()?;
    let version = normalize_version(tag);
    if version_key(&version).is_none() || !safe_release_tag(tag) {
        return None;
    }
    let notes = json
        .get("body")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let url = format!("{RELEASES_URL}/tag/{tag}");
    let mut download = None;
    let mut sha256 = None;
    let mut size = None;
    if is_msi && matches!(arch, "x64" | "x86" | "arm64") {
        let expected_name = format!("VietFlow-win-{arch}.msi");
        let expected_url = format!(
            "https://github.com/KienPC1234/VietFlow/releases/download/{tag}/{expected_name}"
        );
        if let Some(assets) = json.get("assets").and_then(|v| v.as_array()) {
            for asset in assets {
                if asset.get("name").and_then(|v| v.as_str()) != Some(expected_name.as_str())
                    || asset.get("browser_download_url").and_then(|v| v.as_str())
                        != Some(expected_url.as_str())
                {
                    continue;
                }
                let Some(asset_digest) = asset
                    .get("digest")
                    .and_then(|v| v.as_str())
                    .and_then(parse_sha256_digest)
                else {
                    continue;
                };
                let Some(asset_size) = asset
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .filter(|&n| n > 0 && n <= MAX_MSI_BYTES as u64)
                else {
                    continue;
                };
                download = Some(expected_url.clone());
                sha256 = Some(asset_digest);
                size = Some(asset_size);
                break;
            }
        }
    }
    Some(ReleaseInfo {
        version,
        notes,
        url,
        download,
        sha256,
        size,
    })
}

fn parse_api_release(body: &str) -> Option<ReleaseInfo> {
    parse_api_release_for_arch(body, is_installed_via_msi(), runtime_arch())
}

fn fetch_release_info() -> Result<ReleaseInfo, FetchError> {
    if is_installed_via_msi() {
        // Only official GitHub API asset digest/size can authorize an MSI.
        // Do not auto-run a URL supplied by latest.json.
        return fetch_url(API_URL).and_then(|body| {
            parse_api_release_for_arch(&body, true, runtime_arch())
                .ok_or_else(|| FetchError::new("Invalid official release metadata"))
        });
    }
    // Portable builds only display release links.
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
            if version_key(&version).is_some() && safe_release_tag(&format!("v{version}")) {
                return Ok(ReleaseInfo {
                    version: version.clone(),
                    notes: String::new(),
                    url: format!("{RELEASES_URL}/tag/v{version}"),
                    download: None,
                    sha256: None,
                    size: None,
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
            sha256: None,
            size: None,
        }),
        Err(error) => Err(error),
    }
}

fn version_key(value: &str) -> Option<([u32; 3], Option<String>)> {
    let value = normalize_version(value);
    // SemVer build metadata does not affect precedence. A prerelease like
    // rc.10 must be parsed as dotted identifiers, not compared as one string.
    let (without_build, build) = match value.split_once('+') {
        Some((core, build)) => (core, Some(build)),
        None => (value.as_str(), None),
    };
    if build.is_some_and(|b| b.is_empty() || !b.split('.').all(valid_semver_identifier)) {
        return None;
    }
    let (core, prerelease) = match without_build.split_once('-') {
        Some((core, pre)) => {
            if !pre.split('.').all(valid_semver_prerelease_identifier) {
                return None;
            }
            (core, Some(pre.to_string()))
        }
        None => (without_build, None),
    };
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() {
        return None;
    }
    Some(([major, minor, patch], prerelease))
}

fn valid_semver_identifier(identifier: &str) -> bool {
    !identifier.is_empty()
        && identifier
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

fn valid_semver_prerelease_identifier(identifier: &str) -> bool {
    valid_semver_identifier(identifier)
        && !(identifier.len() > 1
            && identifier.starts_with('0')
            && identifier.bytes().all(|b| b.is_ascii_digit()))
}

fn compare_prerelease_identifiers(a: &str, b: &str) -> std::cmp::Ordering {
    let a_numeric = a.bytes().all(|c| c.is_ascii_digit());
    let b_numeric = b.bytes().all(|c| c.is_ascii_digit());
    match (a_numeric, b_numeric) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        (false, false) => a.cmp(b),
        (true, true) => {
            // Avoid u32/u64 overflow on large numeric prerelease identifiers.
            let a = a.trim_start_matches('0');
            let b = b.trim_start_matches('0');
            a.len().cmp(&b.len()).then_with(|| a.cmp(b))
        }
    }
}

fn prerelease_is_newer(remote: &str, local: &str) -> bool {
    let mut r = remote.split('.');
    let mut l = local.split('.');
    loop {
        match (r.next(), l.next()) {
            (None, None) | (None, Some(_)) => return false,
            (Some(_), None) => return true,
            (Some(a), Some(b)) => match compare_prerelease_identifiers(a, b) {
                std::cmp::Ordering::Greater => return true,
                std::cmp::Ordering::Less => return false,
                std::cmp::Ordering::Equal => {}
            },
        }
    }
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
        (None, Some(_)) => true, // A final release is newer than any prerelease.
        (Some(_), None) => false,
        (Some(remote), Some(local)) => prerelease_is_newer(&remote, &local),
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

#[link(name = "bcrypt")]
extern "system" {
    fn BCryptOpenAlgorithmProvider(
        handle: *mut *mut c_void,
        algorithm: *const u16,
        implementation: *const u16,
        flags: u32,
    ) -> i32;
    fn BCryptCloseAlgorithmProvider(handle: *mut c_void, flags: u32) -> i32;
    fn BCryptCreateHash(
        algorithm: *mut c_void,
        hash: *mut *mut c_void,
        object: *mut u8,
        object_len: u32,
        secret: *mut u8,
        secret_len: u32,
        flags: u32,
    ) -> i32;
    fn BCryptHashData(hash: *mut c_void, data: *mut u8, len: u32, flags: u32) -> i32;
    fn BCryptFinishHash(hash: *mut c_void, output: *mut u8, len: u32, flags: u32) -> i32;
    fn BCryptDestroyHash(hash: *mut c_void) -> i32;
}

struct CngAlgorithm(*mut c_void);
impl Drop for CngAlgorithm {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                BCryptCloseAlgorithmProvider(self.0, 0);
            }
        }
    }
}

struct CngHash(*mut c_void);
impl Drop for CngHash {
    fn drop(&mut self) {
        if !self.0.is_null() {
            unsafe {
                BCryptDestroyHash(self.0);
            }
        }
    }
}

/// Incremental SHA-256 via Windows CNG. Unlike BCryptHash (Windows 10+),
/// BCryptCreateHash/HashData/FinishHash support Windows 7 and bound memory.
fn file_sha256(path: &std::path::Path) -> Result<[u8; 32], FetchError> {
    use std::io::Read;
    let mut file =
        std::fs::File::open(path).map_err(|e| FetchError::new(format!("Cannot read MSI: {e}")))?;
    unsafe {
        let mut algorithm = CngAlgorithm(std::ptr::null_mut());
        if BCryptOpenAlgorithmProvider(
            &mut algorithm.0,
            to_wide("SHA256").as_ptr(),
            std::ptr::null(),
            0,
        ) < 0
        {
            return Err(FetchError::new("SHA-256 provider unavailable"));
        }
        let mut hash = CngHash(std::ptr::null_mut());
        // Windows 7+ CNG may allocate the hash-object memory internally.
        if BCryptCreateHash(
            algorithm.0,
            &mut hash.0,
            std::ptr::null_mut(),
            0,
            std::ptr::null_mut(),
            0,
            0,
        ) < 0
        {
            return Err(FetchError::new("Cannot initialize MSI SHA-256"));
        }
        let mut buf = [0u8; 64 * 1024];
        loop {
            let count = file
                .read(&mut buf)
                .map_err(|e| FetchError::new(format!("Cannot read MSI bytes: {e}")))?;
            if count == 0 {
                break;
            }
            if BCryptHashData(hash.0, buf.as_mut_ptr(), count as u32, 0) < 0 {
                return Err(FetchError::new("Cannot hash MSI bytes"));
            }
        }
        let mut digest = [0u8; 32];
        if BCryptFinishHash(hash.0, digest.as_mut_ptr(), digest.len() as u32, 0) < 0 {
            return Err(FetchError::new("Cannot finalize MSI SHA-256"));
        }
        Ok(digest)
    }
}

/// Verify the exact downloaded size and digest before an installer launch.
fn verify_msi_file(
    path: &std::path::Path,
    expected_size: u64,
    expected_hash: &[u8; 32],
) -> Result<(), FetchError> {
    let downloaded = std::fs::metadata(path)
        .map_err(|e| FetchError::new(format!("Cannot inspect MSI size: {e}")))?
        .len();
    if downloaded != expected_size {
        return Err(FetchError::new("MSI size mismatch or truncated download"));
    }
    if &file_sha256(path)? != expected_hash {
        return Err(FetchError::new(
            "MSI SHA-256 does not match GitHub Releases",
        ));
    }
    Ok(())
}
/// Keep the downloaded path read-only while checking its digest, signature
/// and dispatching the installer. FILE_SHARE_READ denies concurrent writes,
/// deletes and replacement by other handles during this critical section.
/// This is only a dispatch-time guard, not a guarantee after the updater exits.
fn lock_msi_for_verification(path: &std::path::Path) -> Result<std::fs::File, FetchError> {
    use std::os::windows::fs::OpenOptionsExt;

    const FILE_SHARE_READ: u32 = 0x0000_0001;
    std::fs::OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ)
        .open(path)
        .map_err(|e| FetchError::new(format!("Cannot lock MSI against modifications: {e}")))
}

/// Validate the MSI's Authenticode chain with the installed Windows trust
/// policy. SHA-256 from the official release API alone is insufficient to
/// prove that a publisher signed the executable installer.
///
/// WinVerifyTrust is intentionally performed on the staged on-disk MSI before
/// ShellExecute. Unsigned, expired, revoked, or untrusted signatures fail
/// closed; portable/manual release browsing remains available instead.
///
/// This verifies the Windows trust chain, NOT a specific VietFlow publisher
/// certificate. Publisher-identity pinning is a separate release gate.
fn verify_msi_authenticode(path: &std::path::Path) -> Result<(), FetchError> {
    use std::os::windows::ffi::OsStrExt;

    let wide_path: Vec<u16> = path
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let mut file = WINTRUST_FILE_INFO {
        cbStruct: std::mem::size_of::<WINTRUST_FILE_INFO>() as u32,
        pcwszFilePath: wide_path.as_ptr(),
        hFile: std::ptr::null_mut(),
        pgKnownSubject: std::ptr::null_mut(),
    };
    let mut trust = WINTRUST_DATA {
        cbStruct: std::mem::size_of::<WINTRUST_DATA>() as u32,
        dwUIChoice: WTD_UI_NONE,
        fdwRevocationChecks: WTD_REVOKE_WHOLECHAIN,
        dwUnionChoice: WTD_CHOICE_FILE,
        Anonymous: WINTRUST_DATA_0 { pFile: &mut file },
        dwStateAction: WTD_STATEACTION_IGNORE,
        dwProvFlags: WTD_REVOCATION_CHECK_CHAIN_EXCLUDE_ROOT,
        dwUIContext: WTD_UICONTEXT_INSTALL,
        ..Default::default()
    };
    let mut action = WINTRUST_ACTION_GENERIC_VERIFY_V2;
    let status = unsafe {
        WinVerifyTrust(
            std::ptr::null_mut(),
            &mut action,
            (&mut trust as *mut WINTRUST_DATA).cast(),
        )
    };
    if status != 0 {
        return Err(FetchError::new(format!(
            "MSI Authenticode signature or trust chain invalid (0x{:08X})",
            status as u32
        )));
    }
    Ok(())
}

/// Fail-closed MSI launcher; only official GitHub API digests, byte lengths
/// and valid Windows Authenticode chains authorize unattended execution.
fn launch_verified_msi(info: &ReleaseInfo) -> Result<(), FetchError> {
    let url = info
        .download
        .as_deref()
        .ok_or_else(|| FetchError::new("No verified MSI URL available"))?;
    let expected_hash = info
        .sha256
        .ok_or_else(|| FetchError::new("No verified SHA-256 available"))?;
    let expected_size = info
        .size
        .ok_or_else(|| FetchError::new("No verified MSI size available"))?;
    if !url.starts_with("https://github.com/KienPC1234/VietFlow/releases/download/")
        || !url.ends_with(&format!("/VietFlow-win-{}.msi", runtime_arch()))
    {
        return Err(FetchError::new("Untrusted MSI URL"));
    }
    let destination = std::env::temp_dir().join(format!(
        "VietFlow-update-{}-{}.msi",
        std::process::id(),
        crate::hook::get_tick_count_64()
    ));
    let result = (|| {
        download_file(url, &destination)?;
        let _verified_file_guard = lock_msi_for_verification(&destination)?;
        verify_msi_file(&destination, expected_size, &expected_hash)?;
        verify_msi_authenticode(&destination)?;
        let path = to_wide(&destination.to_string_lossy());
        let result = unsafe {
            windows_sys::Win32::UI::Shell::ShellExecuteW(
                0 as _,
                to_wide("open").as_ptr(),
                path.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if (result as isize) <= 32 {
            return Err(FetchError::new("Windows refused to launch verified MSI"));
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&destination);
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
        message.push_str("Bạn đang sử dụng bản cài đặt (MSI).\nBạn có muốn tự động tải bản MSI mới nhất và tiến hành nâng cấp ngay không?");
        if show_msg(&message, "Cập nhật VietFlow", MB_YESNO | MB_ICONQUESTION) == IDYES {
            if info.download.is_some() && info.sha256.is_some() && info.size.is_some() {
                show_msg(
                    "Downloading MSI and verifying GitHub SHA-256 and Windows Authenticode signature...",
                    "VietFlow update",
                    MB_OK | MB_ICONINFORMATION,
                );
                match launch_verified_msi(info) {
                    Ok(()) => std::process::exit(0),
                    Err(err) => {
                        let msg = format!(
                            "Unable to install MSI safely: {}\n\nOpen GitHub release page manually?",
                            err.message
                        );
                        if show_msg(&msg, "VietFlow update error", MB_YESNO | MB_ICONWARNING)
                            == IDYES
                        {
                            open_browser(&info.url);
                        }
                    }
                }
            } else {
                open_browser(&info.url);
            }
        }
    } else {
        message.push_str("Bạn đang sử dụng bản Portable.\nBạn có muốn mở trang phát hành trên GitHub để tải bản mới về không?");
        if show_msg(&message, "Cập nhật VietFlow", MB_YESNO | MB_ICONQUESTION) == IDYES {
            open_browser(&info.url);
        }
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
        assert!(is_newer_version("1.0.0-rc.10", "1.0.0-rc.2"));
        assert!(!is_newer_version("1.0.0-rc.2", "1.0.0-rc.10"));
        assert!(is_newer_version("1.0.0-alpha.1", "1.0.0-alpha"));
        assert!(is_newer_version("1.0.0-beta", "1.0.0-99"));
        assert!(is_newer_version(
            "1.0.0-alpha.100000000000000000000",
            "1.0.0-alpha.9"
        ));
        assert!(is_newer_version("1.0.0-rc.1.2", "1.0.0-rc.1"));
        assert!(!is_newer_version("1.0.0-rc.1", "1.0.0-rc.1.2"));
        assert!(!is_newer_version("1.0.0+build.5", "1.0.0+build.4"));
        assert!(is_newer_version("1.0.1+build.1", "1.0.0+build.99"));
        assert!(!is_newer_version("1.0.0.2", "1.0.0"));
        assert!(!is_newer_version("1.0.0-rc..1", "1.0.0"));
        assert!(!is_newer_version("1.0.0-rc.01", "1.0.0-rc.1"));
        assert!(!is_newer_version("1.0.0-01", "1.0.0-1"));
        assert!(!is_newer_version(
            "1.0.0-rc.1+build.12",
            "1.0.0-rc.1+build.1"
        ));
    }

    #[test]
    fn parses_manifest() {
        let info = parse_manifest(
            r#"{"version":"v0.2.0","notes":"hello","url":"https://example.test","downloads":{"x64":"https://example.test/x64.exe","x86":"https://example.test/x86.exe"}}"#,
        )
        .expect("manifest should parse");
        assert_eq!(info.version, "0.2.0");
        assert_eq!(info.notes, "hello");
        assert_eq!(info.url, RELEASES_URL);
        assert_eq!(info.download, None);
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
                "x64": "https://example.test/VietFlow-x64.exe",
                "msi_x64": "https://example.test/VietFlow-win-x64.msi",
                "x86": "https://example.test/VietFlow-x86.exe",
                "msi_x86": "https://example.test/VietFlow-win-x86.msi",
                "arm64": "https://example.test/VietFlow-arm64.exe",
                "msi_arm64": "https://example.test/VietFlow-win-arm64.msi"
            }
        }"#;

        // Portable mode: download is None (directs user to url)
        let portable_x64 = parse_manifest_for_arch(manifest_json, false, "x64").unwrap();
        assert_eq!(portable_x64.download, None);
        assert_eq!(portable_x64.url, RELEASES_URL);

        let portable_x86 = parse_manifest_for_arch(manifest_json, false, "x86").unwrap();
        assert_eq!(portable_x86.download, None);

        let portable_arm64 = parse_manifest_for_arch(manifest_json, false, "arm64").unwrap();
        assert_eq!(portable_arm64.download, None);

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
        assert_eq!(info.url, RELEASES_URL);
    }

    #[test]
    fn untrusted_release_manifest_cannot_redirect_portable_users() {
        let official = format!("{RELEASES_URL}/tag/v0.2.0");
        assert!(is_official_release_page(RELEASES_URL));
        assert!(is_official_release_page(&official));

        for untrusted in [
            "https://evil.example/installer",
            "http://github.com/KienPC1234/VietFlow/releases",
            "https://github.com/KienPC1234/VietFlow/releases/tag/v0.2.0/../../other",
            "https://github.com/KienPC1234/VietFlow/releases/tag/v0.2.0?next=evil",
            "https://github.com/KienPC1234/VietFlow/releases/tag/v0.2.0%2fother",
            "https://github.com/KienPC1234/VietFlow/releases/tag/..",
            "https://github.com/KienPC1234/VietFlow.evil/releases",
        ] {
            assert!(
                !is_official_release_page(untrusted),
                "accepted: {untrusted}"
            );
            let manifest = serde_json::json!({
                "version": "0.2.0", "url": untrusted, "notes": "notes"
            });
            let release = parse_manifest_for_arch(&manifest.to_string(), false, "x64").unwrap();
            assert_eq!(release.url, RELEASES_URL);
            assert!(release.download.is_none());
        }

        let official_manifest = serde_json::json!({
            "version": "0.2.0", "url": official,
        });
        assert_eq!(
            parse_manifest_for_arch(&official_manifest.to_string(), false, "x64")
                .unwrap()
                .url,
            official
        );

        for version in ["", "bad", "1.2.3/other", "1.2.3?redirect=evil"] {
            let invalid = serde_json::json!({"version":version});
            assert!(parse_manifest_for_arch(&invalid.to_string(), false, "x64").is_none());
        }
    }

    #[test]
    fn parses_api_release() {
        let info = parse_api_release(
            r#"{"tag_name":"v0.3.0","body":"notes","html_url":"https://example.test"}"#,
        )
        .expect("release should parse");
        assert_eq!(info.version, "0.3.0");
        assert_eq!(info.notes, "notes");
        assert_eq!(
            info.url,
            "https://github.com/KienPC1234/VietFlow/releases/tag/v0.3.0"
        );
    }

    #[test]
    fn github_msi_requires_exact_asset_digest_arch_and_origin() {
        let digest = "a".repeat(64);
        let asset = serde_json::json!({
            "name": "VietFlow-win-x64.msi",
            "browser_download_url":
                "https://github.com/KienPC1234/VietFlow/releases/download/v0.5.0/VietFlow-win-x64.msi",
            "size": 2048,
            "digest": format!("sha256:{digest}")
        });
        let release = serde_json::json!({
            "tag_name": "v0.5.0", "body": "notes", "assets": [asset]
        });
        let s = release.to_string();
        let checked = parse_api_release_for_arch(&s, true, "x64").unwrap();
        assert!(checked.download.is_some());
        assert_eq!(checked.size, Some(2048));
        assert_eq!(checked.sha256, Some([0xaa; 32]));
        assert!(parse_api_release_for_arch(&s, true, "x86")
            .unwrap()
            .download
            .is_none());
        assert!(parse_api_release_for_arch(&s, false, "x64")
            .unwrap()
            .download
            .is_none());

        for key in ["digest", "size", "browser_download_url", "name"] {
            let mut invalid = release.clone();
            invalid["assets"][0].as_object_mut().unwrap().remove(key);
            assert!(
                parse_api_release_for_arch(&invalid.to_string(), true, "x64")
                    .unwrap()
                    .download
                    .is_none()
            );
        }
        let mut unsafe_url = release.clone();
        unsafe_url["assets"][0]["browser_download_url"] =
            "https://not-github.example/install.msi".into();
        assert!(
            parse_api_release_for_arch(&unsafe_url.to_string(), true, "x64")
                .unwrap()
                .download
                .is_none()
        );
        let mut wrong_digest = release.clone();
        wrong_digest["assets"][0]["digest"] = "sha256:bad".into();
        assert!(
            parse_api_release_for_arch(&wrong_digest.to_string(), true, "x64")
                .unwrap()
                .download
                .is_none()
        );
    }

    #[test]
    fn streaming_cng_sha256_matches_known_vector() {
        let path = std::env::temp_dir().join(format!(
            "VietFlow-test-hash-{}-{}.bin",
            std::process::id(),
            crate::hook::get_tick_count_64()
        ));
        std::fs::write(&path, b"abc").unwrap();
        let actual = file_sha256(&path).expect("Windows CNG SHA-256");
        let _ = std::fs::remove_file(path);
        assert_eq!(
            actual,
            parse_sha256_digest(concat!(
                "sha256:ba7816bf8f01cfea414140de5dae2223",
                "b00361a396177a9cb410ff61f20015ad"
            ))
            .unwrap()
        );
    }

    #[test]
    fn verification_rejects_wrong_length_and_tampered_bytes() {
        let path = std::env::temp_dir().join(format!(
            "VietFlow-verifier-test-{}-{}.bin",
            std::process::id(),
            crate::hook::get_tick_count_64()
        ));
        std::fs::write(&path, b"abc").unwrap();
        let expected = parse_sha256_digest(concat!(
            "sha256:ba7816bf8f01cfea414140de5dae2223",
            "b00361a396177a9cb410ff61f20015ad"
        ))
        .unwrap();
        assert!(verify_msi_file(&path, 3, &expected).is_ok());
        assert!(verify_msi_file(&path, 2, &expected).is_err());
        std::fs::write(&path, b"abd").unwrap();
        assert!(verify_msi_file(&path, 3, &expected).is_err());
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn unsigned_installer_is_never_trusted_for_automatic_launch() {
        let path = std::env::temp_dir().join(format!(
            "VietFlow-test-unsigned-{}-{}.msi",
            std::process::id(),
            crate::hook::get_tick_count_64()
        ));
        // A valid OLE magic header alone cannot establish Authenticode trust.
        let mut body = Vec::from(MSI_MAGIC_HEADER);
        body.resize(1024, 0);
        std::fs::write(&path, body).unwrap();
        let verified = verify_msi_authenticode(&path);
        let _ = std::fs::remove_file(path);
        assert!(verified.is_err(), "an unsigned MSI-shaped file was trusted");
    }

    #[test]
    fn staged_installer_verification_guard_rejects_concurrent_writes() {
        let path = std::env::temp_dir().join(format!(
            "VietFlow-test-file-lock-{}-{}.msi",
            std::process::id(),
            crate::hook::get_tick_count_64()
        ));
        std::fs::write(&path, b"unsigned fixture").unwrap();
        let read_guard = lock_msi_for_verification(&path).unwrap();
        assert!(
            std::fs::OpenOptions::new().write(true).open(&path).is_err(),
            "a concurrent writer should not modify a verified MSI"
        );
        drop(read_guard);
        assert!(std::fs::OpenOptions::new().write(true).open(&path).is_ok());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    #[ignore = "requires an external, authenticode-signed, read-only Windows fixture"]
    fn valid_windows_signature_passes_winverifytrust_when_fixture_is_supplied() {
        use std::io::{Read, Seek, SeekFrom, Write};

        let path = std::env::var_os("VIETFLOW_SIGNED_TEST_FILE")
            .expect("set VIETFLOW_SIGNED_TEST_FILE to a trusted signed binary");
        let path = std::path::Path::new(&path);
        assert!(path.is_file(), "signed test fixture is unavailable");
        verify_msi_authenticode(path)
            .unwrap_or_else(|error| panic!("Windows rejected trusted fixture: {}", error.message));

        // A byte changed in the signed image must not remain trusted.
        // Never modify the source fixture: test only an isolated temp copy.
        assert!(std::fs::metadata(path).unwrap().len() > 8193);
        let tampered = std::env::temp_dir().join(format!(
            "VietFlow-signature-tamper-{}-{}.exe",
            std::process::id(),
            crate::hook::get_tick_count_64()
        ));
        std::fs::copy(path, &tampered).unwrap();
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(&tampered)
            .unwrap();
        file.seek(SeekFrom::Start(8192)).unwrap();
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).unwrap();
        file.seek(SeekFrom::Start(8192)).unwrap();
        file.write_all(&[byte[0] ^ 1]).unwrap();
        file.sync_all().unwrap();
        drop(file);
        let invalid = verify_msi_authenticode(&tampered).is_err();
        std::fs::remove_file(tampered).unwrap();
        assert!(invalid, "tampered signed binary must fail WinVerifyTrust");
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
    fn parses_real_release_manifest_file() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("releases")
            .join("latest.json");
        if path.is_file() {
            let content = std::fs::read_to_string(&path).expect("read latest.json");
            let msi_x64 = parse_manifest_for_arch(&content, true, "x64").unwrap();
            assert!(!msi_x64.version.is_empty());
            // Single-arch builds (e.g. `-Arch x86 -Msi`) rewrite latest.json
            // without x64 downloads; nothing to assert until a full rebuild.
            let Some(msi_url) = msi_x64.download.as_deref() else {
                return;
            };
            assert!(msi_url.contains("VietFlow-win-x64.msi"));
            assert!(!msi_x64.notes.is_empty());

            let port_x64 = parse_manifest_for_arch(&content, false, "x64").unwrap();
            assert_eq!(port_x64.version, msi_x64.version);
            assert_eq!(port_x64.download, None);
            assert!(!port_x64.notes.is_empty());
        }
    }

    #[test]
    #[ignore = "network: requires a published GitHub release"]
    fn live_release_manifest() {
        let info = fetch_release_info().expect("live release manifest");
        assert!(!info.version.is_empty());
        if is_installed_via_msi() {
            let expected = if cfg!(target_arch = "x86") {
                "win-x86"
            } else {
                "win-x64"
            };
            assert!(
                info.download.as_deref().unwrap_or("").contains(expected),
                "manifest did not provide the {expected} download"
            );
        } else {
            assert!(info.url.contains("releases"));
        }
    }

    #[test]
    fn test_trusted_download_url_validation() {
        assert!(is_trusted_download_url(
            "https://github.com/KienPC1234/VietFlow/releases/download/v0.1.4/VietFlow-win-x64.msi"
        ));
        for arch in ["x86", "arm64"] {
            assert!(is_trusted_download_url(&format!(
                "https://github.com/KienPC1234/VietFlow/releases/download/v0.2.0/VietFlow-win-{arch}.msi"
            )));
        }
        // A redirect target is never an authorized initial installer URL.
        // GitHub's CDN hops are followed by WinINet only after an official URL
        // was selected; byte length and official API SHA-256 still gate launch.
        for rejected in [
            "https://objects.githubusercontent.com/github-production-release-asset-2e65be/12345/VietFlow.msi",
            "https://github-releases.githubusercontent.com/12345/VietFlow.msi",
            "https://github.com/KienPC1234/VietFlow/README.md",
            "https://github.com/KienPC1234/VietFlow/releases/download/v0.2.0/other.msi",
            "https://github.com/KienPC1234/VietFlow/releases/download/../VietFlow-win-x64.msi",
            "https://github.com/KienPC1234/VietFlow/releases/download/v0.2.0/VietFlow-win-x64.msi?mirror=evil",
            "https://github.com/KienPC1234/VietFlow/releases/download/v0.2.0/extra/VietFlow-win-x64.msi",
            "https://github.com/KienPC1234/VietFlow/releases/download/v0.2.0%2fother/VietFlow-win-x64.msi",
        ] {
            assert!(!is_trusted_download_url(rejected), "accepted: {rejected}");
        }

        // Reject non-https, untrusted hosts, or fake domains
        assert!(!is_trusted_download_url(
            "http://github.com/KienPC1234/VietFlow/releases/download/v0.1.4/VietFlow.msi"
        ));
        assert!(!is_trusted_download_url("https://evil.com/VietFlow.msi"));
        assert!(!is_trusted_download_url(
            "https://github.com.evil.com/VietFlow.msi"
        ));
        assert!(!is_trusted_download_url(
            "https://github.com/OtherUser/VietFlow/releases/download/v0.1.4/VietFlow.msi"
        ));
    }
}
