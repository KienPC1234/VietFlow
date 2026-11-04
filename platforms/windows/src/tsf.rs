// VietFlow Windows - Text Services Framework (TSF) Manager & Cross-Process IPC
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::config::AppConfig;
use crate::tray::to_wide;
use std::ffi::c_void;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, FreeLibrary, GetLastError, ERROR_CANCELLED, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;

#[link(name = "advapi32")]
extern "system" {
    fn OpenProcessToken(process: HANDLE, desired: u32, token: *mut HANDLE) -> i32;
    fn GetTokenInformation(
        token: HANDLE,
        info_class: i32,
        buffer: *mut c_void,
        size: u32,
        needed: *mut u32,
    ) -> i32;
    fn ConvertSidToStringSidW(sid: *mut c_void, sid_string: *mut *mut u16) -> i32;
    fn ConvertStringSecurityDescriptorToSecurityDescriptorW(
        string_sd: *const u16,
        revision: u32,
        security_descriptor: *mut *mut c_void,
        length: *mut u32,
    ) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetCurrentProcess() -> HANDLE;
    fn LocalFree(memory: *mut c_void) -> *mut c_void;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct SidAndAttributes {
    sid: *mut c_void,
    attributes: u32,
}
fn current_user_sid() -> Option<String> {
    unsafe {
        let mut token: HANDLE = std::ptr::null_mut();
        if OpenProcessToken(GetCurrentProcess(), 0x0008, &mut token) == 0 {
            return None;
        }
        let result = (|| {
            let mut required = 0u32;
            GetTokenInformation(token, 1, std::ptr::null_mut(), 0, &mut required);
            if required == 0 || required > 64 * 1024 {
                return None;
            }
            let mut data = vec![0u8; required as usize];
            if GetTokenInformation(token, 1, data.as_mut_ptr().cast(), required, &mut required) == 0
            {
                return None;
            }
            let entry = std::ptr::read_unaligned(data.as_ptr().cast::<SidAndAttributes>());
            let mut text: *mut u16 = std::ptr::null_mut();
            if entry.sid.is_null() || ConvertSidToStringSidW(entry.sid, &mut text) == 0 {
                return None;
            }
            let mut len = 0;
            while *text.add(len) != 0 && len < 256 {
                len += 1;
            }
            let sid = String::from_utf16_lossy(std::slice::from_raw_parts(text, len));
            LocalFree(text.cast());
            if sid.starts_with("S-1-") {
                Some(sid)
            } else {
                None
            }
        })();
        CloseHandle(token);
        result
    }
}

use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, HKEY, HKEY_CLASSES_ROOT, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE,
    KEY_READ,
};
use windows_sys::Win32::System::Threading::{GetExitCodeProcess, WaitForSingleObject};
use windows_sys::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

// Separate owner-controlled read-mostly configuration from the untrusted
// per-foreground focus signal. AppContainers can READ config, but can WRITE
// only the focus map. Versioned names avoid old 56-byte v1 mappings.
const CONFIG_MAP_NAME: &str = "Local\\VietFlow_TSF_Config_v2";
const FOCUS_MAP_NAME: &str = "Local\\VietFlow_TSF_Focus_v2";

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct TsfSharedConfig {
    pub enabled: u32,
    pub method: u32,
    pub charset: u32,
    pub modern_style: u32,
    pub spell_check: u32,
    pub free_marking: u32,
    pub auto_restore: u32,
    pub auto_switch_by_typing: u32,
    pub enable_macro: u32,
    pub version: u32,
    pub sequence: u32, // even=stable, odd=writer is publishing
    pub reserved: u32,
}
impl Default for TsfSharedConfig {
    fn default() -> Self {
        Self {
            enabled: 1,
            method: 0,
            charset: 0,
            modern_style: 1,
            spell_check: 1,
            free_marking: 1,
            auto_restore: 1,
            auto_switch_by_typing: 1,
            enable_macro: 1,
            version: 2,
            sequence: 0,
            reserved: 0,
        }
    }
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct TsfFocusState {
    pub last_active_tick: u64,
    pub last_active_pid: u32,
    pub is_composing: u32,
}

pub struct SharedConfigWriter {
    mapping: HANDLE,
    view: *mut TsfSharedConfig,
    focus_mapping: HANDLE,
    focus_view: *mut TsfFocusState,
}
unsafe impl Send for SharedConfigWriter {}
unsafe impl Sync for SharedConfigWriter {}

/// Create a mapping whose DACL grants the exact current user, SYSTEM and
/// Administrators full access. AppContainers get config READ only, or focus
/// READ/WRITE so the TIP may report thread focus without changing settings.
unsafe fn create_secure_mapping(
    name: &str,
    size: u32,
    appcontainer_can_write: bool,
) -> Option<(HANDLE, *mut c_void)> {
    let user_sid = current_user_sid()?;
    let package_rights = if appcontainer_can_write { "GRGW" } else { "GR" };
    // Focus reports from UWP/AppContainer run at low integrity. Only this
    // small advisory map gets a low mandatory label; the config remains
    // a medium-integrity, owner-writable mapping.
    let label = if appcontainer_can_write {
        "S:(ML;;NW;;;LW)"
    } else {
        ""
    };
    let sddl =
        format!("D:P(A;;GA;;;SY)(A;;GA;;;BA)(A;;GA;;;{user_sid})(A;;{package_rights};;;AC){label}");
    let mut sd: *mut c_void = std::ptr::null_mut();
    let wide_sd = to_wide(&sddl);
    if ConvertStringSecurityDescriptorToSecurityDescriptorW(
        wide_sd.as_ptr(),
        1,
        &mut sd,
        std::ptr::null_mut(),
    ) == 0
        || sd.is_null()
    {
        return None;
    }
    let attrs = SECURITY_ATTRIBUTES {
        nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
        lpSecurityDescriptor: sd,
        bInheritHandle: 0,
    };
    let wide_name = to_wide(name);
    let mapping = CreateFileMappingW(
        INVALID_HANDLE_VALUE,
        &attrs,
        PAGE_READWRITE,
        0,
        size,
        wide_name.as_ptr(),
    );
    LocalFree(sd);
    if mapping.is_null() {
        return None;
    }
    let mapped = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, size as usize);
    if mapped.Value.is_null() {
        CloseHandle(mapping);
        return None;
    }
    Some((mapping, mapped.Value))
}

impl SharedConfigWriter {
    pub fn new() -> Option<Self> {
        unsafe {
            let (mapping, ptr) = create_secure_mapping(
                CONFIG_MAP_NAME,
                std::mem::size_of::<TsfSharedConfig>() as u32,
                false,
            )?;
            let (focus_mapping, focus_ptr) = match create_secure_mapping(
                FOCUS_MAP_NAME,
                std::mem::size_of::<TsfFocusState>() as u32,
                true,
            ) {
                Some(v) => v,
                None => {
                    UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS { Value: ptr });
                    CloseHandle(mapping);
                    return None;
                }
            };
            let view = ptr.cast::<TsfSharedConfig>();
            if (*view).version != 2 {
                std::ptr::write(view, TsfSharedConfig::default());
            }
            let focus_view = focus_ptr.cast::<TsfFocusState>();
            // Focus is owned and published by the C++ TIP, not the daemon.
            Some(Self {
                mapping,
                view,
                focus_mapping,
                focus_view,
            })
        }
    }
    pub fn update(&mut self, cfg: &TsfSharedConfig) {
        if self.view.is_null() {
            return;
        }
        unsafe {
            let ptr = self.view;
            let sequence = &*std::ptr::addr_of!((*ptr).sequence).cast::<AtomicU32>();
            let old = sequence.load(Ordering::Acquire);
            if old & 1 != 0 {
                return;
            }
            sequence.store(old.wrapping_add(1), Ordering::Release);
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*ptr).enabled), cfg.enabled);
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*ptr).method), cfg.method);
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*ptr).charset), cfg.charset);
            std::ptr::write_volatile(
                std::ptr::addr_of_mut!((*ptr).modern_style),
                cfg.modern_style,
            );
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*ptr).spell_check), cfg.spell_check);
            std::ptr::write_volatile(
                std::ptr::addr_of_mut!((*ptr).free_marking),
                cfg.free_marking,
            );
            std::ptr::write_volatile(
                std::ptr::addr_of_mut!((*ptr).auto_restore),
                cfg.auto_restore,
            );
            std::ptr::write_volatile(
                std::ptr::addr_of_mut!((*ptr).auto_switch_by_typing),
                cfg.auto_switch_by_typing,
            );
            std::ptr::write_volatile(
                std::ptr::addr_of_mut!((*ptr).enable_macro),
                cfg.enable_macro,
            );
            std::ptr::write_volatile(std::ptr::addr_of_mut!((*ptr).version), 2);
            sequence.store(old.wrapping_add(2), Ordering::Release);
        }
    }
}
impl Drop for SharedConfigWriter {
    fn drop(&mut self) {
        unsafe {
            if !self.view.is_null() {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view.cast(),
                });
            }
            if !self.focus_view.is_null() {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.focus_view.cast(),
                });
            }
            if !self.mapping.is_null() {
                CloseHandle(self.mapping);
            }
            if !self.focus_mapping.is_null() {
                CloseHandle(self.focus_mapping);
            }
        }
    }
}

static SHARED_WRITER: Mutex<Option<SharedConfigWriter>> = Mutex::new(None);

/// A nonblocking read of TSF focus from the shared config mapping.
/// This is used by the low-level keyboard hook: it must not wait for mutex
/// contention or do registry/disk probes on the keystroke hot path.
fn try_get_tsf_active_pid() -> Option<u32> {
    let lock = SHARED_WRITER.try_lock().ok()?;
    let writer = match lock.as_ref() {
        Some(writer) if !writer.focus_view.is_null() => writer,
        _ => return Some(0),
    };
    Some(unsafe {
        std::ptr::read_volatile(std::ptr::addr_of!((*writer.focus_view).last_active_pid))
    })
}

/// Returns the active TSF process ID, or 0 if unavailable.
pub fn get_tsf_active_pid() -> u32 {
    try_get_tsf_active_pid().unwrap_or(0)
}

#[inline]
fn tip_owns_foreground(foreground_pid: u32, active_pid: u32) -> bool {
    foreground_pid != 0 && foreground_pid == active_pid
}

/// The hook already caches whether the TIP is registered; check only its
/// published focus PID here. On a concurrent configuration update, yield the
/// current event instead of risking a double insertion into a TSF text field.
pub fn is_tsf_active_for_pid(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    try_get_tsf_active_pid()
        .map(|active_pid| tip_owns_foreground(pid, active_pid))
        .unwrap_or(true)
}

#[cfg(test)]
mod focus_tests {
    use super::tip_owns_foreground;

    #[test]
    fn only_matching_nonzero_foreground_pid_is_owned_by_tsf() {
        assert!(tip_owns_foreground(101, 101));
        assert!(!tip_owns_foreground(101, 102));
        assert!(!tip_owns_foreground(101, 0));
        assert!(!tip_owns_foreground(0, 101));
    }
}

/// TSF mode is defined solely by the Windows registration of the TIP. Once
/// registered, the in-process TIP owns typing and Windows language switching
/// (Win+Space) controls V/E state.
pub fn is_tsf_mode_active() -> bool {
    is_tsf_registered() && is_tsf_dll_present()
}

/// Publishes engine options to the in-process TIP instances.
///
/// `enabled` is written as 1 only while TSF mode is active and Vietnamese input
/// is currently enabled. When the user switches to English or an excluded app is
/// focused, `enabled` is 0 so the TIP passes all keystrokes untouched.
pub fn sync_tsf_config(config: &AppConfig, enabled: bool) {
    let tip_enabled = is_tsf_mode_active() && enabled;
    let mut lock = match SHARED_WRITER.lock() {
        Ok(l) => l,
        Err(_) => return,
    };

    if lock.is_none() {
        *lock = SharedConfigWriter::new();
    }

    if let Some(writer) = lock.as_mut() {
        let tsf_cfg = TsfSharedConfig {
            enabled: if tip_enabled { 1 } else { 0 },
            method: config.method as u32,
            charset: config.charset as u32,
            modern_style: if config.modern_style { 1 } else { 0 },
            spell_check: if config.spell_check { 1 } else { 0 },
            free_marking: if config.free_marking { 1 } else { 0 },
            auto_restore: if config.auto_restore { 1 } else { 0 },
            auto_switch_by_typing: if config.auto_switch_by_typing { 1 } else { 0 },
            enable_macro: if config.enable_macro { 1 } else { 0 },
            version: 2,
            sequence: 0,
            reserved: 0,
        };
        writer.update(&tsf_cfg);
    }
}

pub const CLSID_STR: &str = "{02B8B9EC-2D3E-42B6-97C0-26043A29BFCB}";
pub const PROFILE_STR: &str = "{FBA81B7C-5EAD-454F-9F81-413780E9A0BA}";

pub const LEGACY_CLSID_STR: &str = "{7C2B5E10-9A31-4B8F-B623-192E48270101}";
pub const LEGACY_PROFILE_STR: &str = "{7C2B5E11-9A31-4B8F-B623-192E48270102}";

/// Locates the `vietflow_tsf.dll` (or legacy `vietflow_tip.dll`) file.
/// First checks the same directory as the executable, then working directory and target dirs.
pub fn tsf_dll_path() -> Option<PathBuf> {
    let dll_names = ["vietflow_tsf.dll", "vietflow_tip.dll"];
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            for name in &dll_names {
                let direct = parent.join(name);
                if direct.is_file() {
                    return Some(direct);
                }
            }
            if let Some(grandparent) = parent.parent() {
                for name in &dll_names {
                    let in_profile = grandparent.join(name);
                    if in_profile.is_file() {
                        return Some(in_profile);
                    }
                }
                if let Some(great) = grandparent.parent() {
                    for name in &dll_names {
                        let in_release = great.join("release").join(name);
                        if in_release.is_file() {
                            return Some(in_release);
                        }
                        let in_debug = great.join("debug").join(name);
                        if in_debug.is_file() {
                            return Some(in_debug);
                        }
                    }
                }
            }
        }
    }
    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..4 {
            for name in &dll_names {
                let direct = dir.join(name);
                if direct.is_file() {
                    return Some(direct);
                }
                let target_rel = dir.join("target").join("release").join(name);
                if target_rel.is_file() {
                    return Some(target_rel);
                }
                let dist_rel = dir.join("dist").join(name);
                if dist_rel.is_file() {
                    return Some(dist_rel);
                }
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

/// Returns true if `vietflow_tsf.dll` (or `vietflow_tip.dll`) is present on the system.
/// If false, any TSF IME options must be completely hidden from user.
pub fn is_tsf_dll_present() -> bool {
    tsf_dll_path().is_some()
}

/// Checks whether the VietFlow TIP COM CLSID and LanguageProfile are genuinely registered.
/// Validates both the COM InprocServer32 registration and HKLM CTF profile, and checks
/// HKCU to ensure the profile is not disabled (Enable == 0).
pub fn is_tsf_registered() -> bool {
    unsafe {
        // 1. Verify COM InprocServer32 exists. Without COM registration, Windows cannot load the TIP.
        let clsid_subkey = to_wide(&format!(
            "SOFTWARE\\Classes\\CLSID\\{CLSID_STR}\\InprocServer32\0"
        ));
        let mut hkey: HKEY = std::ptr::null_mut();
        let mut com_registered = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            clsid_subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        ) == 0;
        if com_registered {
            RegCloseKey(hkey);
        } else {
            let clsid_hkcr = to_wide(&format!("CLSID\\{CLSID_STR}\\InprocServer32\0"));
            if RegOpenKeyExW(
                HKEY_CLASSES_ROOT,
                clsid_hkcr.as_ptr(),
                0,
                KEY_READ,
                &mut hkey,
            ) == 0
            {
                com_registered = true;
                RegCloseKey(hkey);
            }
        }
        if !com_registered {
            return false;
        }

        // 2. Verify CTF TIP LanguageProfile exists in HKLM or HKCU
        let tip_subkey_hklm = to_wide(&format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{CLSID_STR}\\LanguageProfile\\0x00000409\\{PROFILE_STR}\0"));
        let mut hkey_tip: HKEY = std::ptr::null_mut();
        let hklm_profile_ok = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            tip_subkey_hklm.as_ptr(),
            0,
            KEY_READ,
            &mut hkey_tip,
        ) == 0;
        if hklm_profile_ok {
            RegCloseKey(hkey_tip);
        }

        let tip_subkey_hkcu = to_wide(&format!("Software\\Microsoft\\CTF\\TIP\\{CLSID_STR}\\LanguageProfile\\0x00000409\\{PROFILE_STR}\0"));
        let mut hkey_hkcu_prof: HKEY = std::ptr::null_mut();
        let hkcu_profile_ok = RegOpenKeyExW(
            HKEY_CURRENT_USER,
            tip_subkey_hkcu.as_ptr(),
            0,
            KEY_READ,
            &mut hkey_hkcu_prof,
        ) == 0;
        if hkcu_profile_ok {
            RegCloseKey(hkey_hkcu_prof);
        }

        if !hklm_profile_ok && !hkcu_profile_ok {
            return false;
        }

        // 3. If present in HKCU, make sure Enable != 0
        let hkcu_subkey = to_wide(&format!("Software\\Microsoft\\CTF\\TIP\\{CLSID_STR}\\LanguageProfile\\0x00000409\\{PROFILE_STR}\0"));
        let mut hkey_hkcu: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(
            HKEY_CURRENT_USER,
            hkcu_subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey_hkcu,
        ) == 0
        {
            let mut enable_val: u32 = 0;
            let mut val_type: u32 = 0;
            let mut val_size: u32 = std::mem::size_of::<u32>() as u32;
            let enable_name = to_wide("Enable\0");
            let query_res = windows_sys::Win32::System::Registry::RegQueryValueExW(
                hkey_hkcu,
                enable_name.as_ptr(),
                std::ptr::null_mut(),
                &mut val_type,
                &mut enable_val as *mut u32 as *mut u8,
                &mut val_size,
            );
            RegCloseKey(hkey_hkcu);
            if query_res == 0 && enable_val == 0 {
                return false;
            }
        }

        true
    }
}

/// Note: SetDefaultLayoutOrTip is deliberately not called at runtime. It makes
/// Explorer synchronously tear down and reload every input stack, which hangs
/// the shell. Runtime state reaches the TIP through shared memory instead.
pub fn call_set_default_layout_or_tip(layout_str: &str) -> bool {
    unsafe {
        let input_mod = LoadLibraryW(to_wide("input.dll\0").as_ptr());
        if input_mod.is_null() {
            return false;
        }
        let proc = GetProcAddress(input_mod, c"SetDefaultLayoutOrTip".as_ptr() as *const _);
        let mut ok = false;
        if let Some(func) = proc {
            type FnSetDefaultLayoutOrTip =
                unsafe extern "system" fn(*const u16, u32) -> windows_sys::core::BOOL;
            let func: FnSetDefaultLayoutOrTip = std::mem::transmute(func);
            let wide = to_wide(&format!("{layout_str}\0"));
            ok = func(wide.as_ptr(), 0) != 0;
        }
        FreeLibrary(input_mod);
        ok
    }
}

const ILOT_UNINSTALL: u32 = 0x00000001;
const ILOT_DEFPROFILE: u32 = 0x00000002;

pub fn call_install_layout_or_tip(layout_str: &str, uninstall: bool) -> bool {
    unsafe {
        let input_mod = LoadLibraryW(to_wide("input.dll\0").as_ptr());
        if input_mod.is_null() {
            return false;
        }
        let proc = GetProcAddress(input_mod, c"InstallLayoutOrTip".as_ptr() as *const _);
        let mut ok = false;
        if let Some(func) = proc {
            type FnInstallLayoutOrTip =
                unsafe extern "system" fn(*const u16, u32) -> windows_sys::core::BOOL;
            let func: FnInstallLayoutOrTip = std::mem::transmute(func);
            let wide = to_wide(&format!("{layout_str}\0"));
            let flags = if uninstall {
                ILOT_UNINSTALL
            } else {
                ILOT_DEFPROFILE
            };
            ok = func(wide.as_ptr(), flags) != 0;
        }
        FreeLibrary(input_mod);
        ok
    }
}

fn run_elevated_wait(file: &str, params: &str) -> Result<(), String> {
    unsafe {
        #[link(name = "shell32")]
        extern "system" {
            fn IsUserAnAdmin() -> windows_sys::core::BOOL;
        }

        // If the current process is already elevated (e.g. running under Administrator or SYSTEM during MSI setup),
        // spawn directly using Command to avoid UAC prompt failure in non-interactive/SYSTEM contexts.
        if IsUserAnAdmin() != 0 {
            let mut cmd = std::process::Command::new(file);
            cmd.raw_arg(params);
            match cmd.status() {
                Ok(status) if status.success() => return Ok(()),
                Ok(status) => {
                    return Err(format!(
                        "Tiến trình hoàn tất với mã lỗi: {:?}",
                        status.code()
                    ))
                }
                Err(e) => return Err(format!("Không thể khởi chạy {file}: {e}")),
            }
        }

        let file_wide = to_wide(&format!("{file}\0"));
        let params_wide = to_wide(&format!("{params}\0"));
        let verb_wide = to_wide("runas\0");

        let mut sei: SHELLEXECUTEINFOW = std::mem::zeroed();
        sei.cbSize = std::mem::size_of::<SHELLEXECUTEINFOW>() as u32;
        sei.fMask = SEE_MASK_NOCLOSEPROCESS;
        sei.lpVerb = verb_wide.as_ptr();
        sei.lpFile = file_wide.as_ptr();
        sei.lpParameters = params_wide.as_ptr();
        sei.nShow = SW_HIDE;

        let ret = ShellExecuteExW(&mut sei);
        if ret == 0 {
            let err = GetLastError();
            if err == ERROR_CANCELLED {
                return Err(
                    "Thao tác bị hủy bởi người dùng (yêu cầu quyền Administrator).".to_string(),
                );
            }
            return Err(format!("ShellExecuteExW thất bại (mã lỗi: {err})"));
        }

        if !sei.hProcess.is_null() && sei.hProcess != INVALID_HANDLE_VALUE {
            let wait_res = WaitForSingleObject(sei.hProcess, 30_000);
            let mut exit_code: u32 = 0;
            GetExitCodeProcess(sei.hProcess, &mut exit_code);
            CloseHandle(sei.hProcess);

            if wait_res == WAIT_OBJECT_0 && exit_code == 0 {
                Ok(())
            } else {
                Err(format!("Tiến trình hoàn tất với mã lỗi: {exit_code}"))
            }
        } else {
            Ok(())
        }
    }
}

unsafe fn delete_hkcu_tree(subkey: &str) {
    let wide = to_wide(&format!("{subkey}\0"));
    #[link(name = "advapi32")]
    extern "system" {
        fn RegDeleteTreeW(hKey: HKEY, lpSubKey: *const u16) -> i32;
    }
    let _ = RegDeleteTreeW(HKEY_CURRENT_USER, wide.as_ptr());
}

/// Registers the VietFlow TSF in-process COM DLL via regsvr32 with elevation.
pub fn register_tsf() -> Result<(), String> {
    let dll = tsf_dll_path().ok_or_else(|| "vietflow_tsf.dll không tồn tại.".to_string())?;

    // 0. Remove legacy layouts and purge legacy HKCU entries
    call_install_layout_or_tip(
        &format!("0409:{LEGACY_CLSID_STR}{LEGACY_PROFILE_STR}"),
        true,
    );
    call_install_layout_or_tip(
        &format!("042a:{LEGACY_CLSID_STR}{LEGACY_PROFILE_STR}"),
        true,
    );
    unsafe {
        delete_hkcu_tree(&format!(
            "Software\\Microsoft\\CTF\\TIP\\{LEGACY_CLSID_STR}"
        ));
        delete_hkcu_tree(&format!(
            "Software\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\{LEGACY_CLSID_STR}"
        ));
        delete_hkcu_tree(&format!("Software\\Classes\\CLSID\\{LEGACY_CLSID_STR}"));
    }

    // 1. Run elevated regsvr32 to register COM server and HKLM CTF TIP entries
    let params = format!("/s \"{}\"", dll.to_string_lossy());
    run_elevated_wait("regsvr32.exe", &params)?;

    // 2. Add layout to active user keyboard layout list via input.dll (English US only)
    call_install_layout_or_tip(&format!("0409:{CLSID_STR}{PROFILE_STR}"), false);
    call_install_layout_or_tip(&format!("042a:{CLSID_STR}{PROFILE_STR}"), true);

    Ok(())
}

/// Unregisters the VietFlow TSF in-process COM DLL via regsvr32 with elevation.
pub fn unregister_tsf() -> Result<(), String> {
    // 1. Remove layout from active user keyboard layout list so Windows immediately deactivates it
    call_install_layout_or_tip(&format!("0409:{CLSID_STR}{PROFILE_STR}"), true);
    call_install_layout_or_tip(&format!("042a:{CLSID_STR}{PROFILE_STR}"), true);
    call_install_layout_or_tip(
        &format!("0409:{LEGACY_CLSID_STR}{LEGACY_PROFILE_STR}"),
        true,
    );
    call_install_layout_or_tip(
        &format!("042a:{LEGACY_CLSID_STR}{LEGACY_PROFILE_STR}"),
        true,
    );

    // 2. Wipe HKCU entries directly in current user hive so is_tsf_registered() does not report false positive
    unsafe {
        delete_hkcu_tree(&format!("Software\\Microsoft\\CTF\\TIP\\{CLSID_STR}"));
        delete_hkcu_tree(&format!(
            "Software\\Microsoft\\CTF\\TIP\\{LEGACY_CLSID_STR}"
        ));
        delete_hkcu_tree(&format!(
            "Software\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\{CLSID_STR}"
        ));
        delete_hkcu_tree(&format!(
            "Software\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\{LEGACY_CLSID_STR}"
        ));
        delete_hkcu_tree(&format!("Software\\Classes\\CLSID\\{CLSID_STR}"));
        delete_hkcu_tree(&format!("Software\\Classes\\CLSID\\{LEGACY_CLSID_STR}"));
    }

    // 3. Run elevated regsvr32 /u to wipe COM and HKLM CTF TIP registrations
    if let Some(dll) = tsf_dll_path() {
        let params = format!("/u /s \"{}\"", dll.to_string_lossy());
        let _ = run_elevated_wait("regsvr32.exe", &params);
    }

    Ok(())
}

/// Reinstalls (unregisters and re-registers) the VietFlow TSF COM TIP DLL.
pub fn reinstall_tsf() -> Result<(), String> {
    let _ = unregister_tsf();
    std::thread::sleep(std::time::Duration::from_millis(300));
    register_tsf()?;
    let config = AppConfig::load();
    sync_tsf_config(&config, config.enabled);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsf_shared_config_layout() {
        assert_eq!(std::mem::size_of::<TsfSharedConfig>(), 48);
        assert_eq!(std::mem::size_of::<TsfFocusState>(), 16);
        let cfg = TsfSharedConfig::default();
        assert_eq!(cfg.enabled, 1);
        assert_eq!(cfg.method, 0);
        assert_eq!(cfg.auto_switch_by_typing, 1);
        assert_eq!(cfg.enable_macro, 1);
        assert_eq!(cfg.version, 2);
        assert_eq!(cfg.sequence, 0);
        assert_eq!(TsfFocusState::default().last_active_pid, 0);
    }

    #[test]
    fn secure_config_and_focus_mappings_are_independent() {
        let suffix = format!(
            "{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        );
        let config_name = format!("Local\\VietFlow_Config_QA_{suffix}");
        let focus_name = format!("Local\\VietFlow_Focus_QA_{suffix}");
        let (mapping, cfg_ptr) = unsafe {
            create_secure_mapping(
                &config_name,
                std::mem::size_of::<TsfSharedConfig>() as u32,
                false,
            )
        }
        .expect("current-user config DACL and mapping");
        let (focus_mapping, focus_ptr) = unsafe {
            create_secure_mapping(
                &focus_name,
                std::mem::size_of::<TsfFocusState>() as u32,
                true,
            )
        }
        .expect("appcontainer-writable focus DACL and mapping");
        let mut writer = SharedConfigWriter {
            mapping,
            view: cfg_ptr.cast(),
            focus_mapping,
            focus_view: focus_ptr.cast(),
        };
        unsafe {
            std::ptr::write(writer.view, TsfSharedConfig::default());
            std::ptr::write(writer.focus_view, TsfFocusState::default());
            (*writer.focus_view).last_active_pid = 8527;
        }
        let mut next = TsfSharedConfig::default();
        next.enabled = 0;
        next.method = 2;
        writer.update(&next);
        unsafe {
            assert_eq!((*writer.view).enabled, 0);
            assert_eq!((*writer.view).method, 2);
            assert_eq!((*writer.view).version, 2);
            assert_eq!((*writer.view).sequence & 1, 0);
            assert_eq!((*writer.focus_view).last_active_pid, 8527);
        }
    }

    #[test]
    fn test_tsf_dll_path_search() {
        let _ = tsf_dll_path();
    }

    #[test]
    fn test_tsf_registration_lifecycle() {
        if is_tsf_registered() {
            let _ = unregister_tsf();
        }
        assert!(!is_tsf_registered());
    }
}
