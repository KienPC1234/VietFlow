// VietFlow Windows - Text Services Framework (TSF) Manager & Cross-Process IPC
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::config::AppConfig;
use crate::tray::to_wide;
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, FreeLibrary, GetLastError, ERROR_CANCELLED, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::Security::{
    InitializeSecurityDescriptor, SetSecurityDescriptorDacl, SECURITY_ATTRIBUTES,
    SECURITY_DESCRIPTOR,
};
const SECURITY_DESCRIPTOR_REVISION: u32 = 1;
use windows_sys::Win32::Foundation::HWND;
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
use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowThreadProcessId, SW_HIDE};

const SHARED_MEM_NAME: &str = "Local\\VietFlow_TSF_Config\0";

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
    pub last_active_tick: u64,
    pub last_active_pid: u32,
    pub is_composing: u32,
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
            version: 1,
            last_active_tick: 0,
            last_active_pid: 0,
            is_composing: 0,
        }
    }
}

pub struct SharedConfigWriter {
    mapping: HANDLE,
    view: *mut TsfSharedConfig,
}

unsafe impl Send for SharedConfigWriter {}
unsafe impl Sync for SharedConfigWriter {}

impl SharedConfigWriter {
    pub fn new() -> Option<Self> {
        unsafe {
            let name = to_wide(SHARED_MEM_NAME);
            let size = std::mem::size_of::<TsfSharedConfig>() as u32;

            // Security attributes with NULL DACL allows AppContainer (UWP / Windows Store)
            // and elevated processes on Windows 10/11 to access the shared memory configuration.
            let mut sd: SECURITY_DESCRIPTOR = std::mem::zeroed();
            InitializeSecurityDescriptor(
                &mut sd as *mut _ as *mut std::ffi::c_void,
                SECURITY_DESCRIPTOR_REVISION,
            );
            SetSecurityDescriptorDacl(
                &mut sd as *mut _ as *mut std::ffi::c_void,
                1,
                std::ptr::null_mut(),
                0,
            );
            let mut sa = SECURITY_ATTRIBUTES {
                nLength: std::mem::size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: &mut sd as *mut _ as *mut std::ffi::c_void,
                bInheritHandle: 0,
            };

            let mapping = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                &mut sa,
                PAGE_READWRITE,
                0,
                size,
                name.as_ptr(),
            );

            if mapping.is_null() {
                return None;
            }

            let mapped = MapViewOfFile(mapping, FILE_MAP_ALL_ACCESS, 0, 0, size as usize);
            if mapped.Value.is_null() {
                CloseHandle(mapping);
                return None;
            }
            let view = mapped.Value as *mut TsfSharedConfig;

            // Initialize default values if freshly created
            if (*view).version == 0 {
                std::ptr::write(view, TsfSharedConfig::default());
            }

            Some(Self { mapping, view })
        }
    }

    pub fn update(&mut self, config: &TsfSharedConfig) {
        if !self.view.is_null() {
            unsafe {
                std::ptr::write(self.view, *config);
            }
        }
    }
}

impl Drop for SharedConfigWriter {
    fn drop(&mut self) {
        unsafe {
            if !self.view.is_null() {
                UnmapViewOfFile(MEMORY_MAPPED_VIEW_ADDRESS {
                    Value: self.view as *mut std::ffi::c_void,
                });
            }
            if !self.mapping.is_null() {
                CloseHandle(self.mapping);
            }
        }
    }
}

static SHARED_WRITER: Mutex<Option<SharedConfigWriter>> = Mutex::new(None);

/// Whether VietFlow's TSF TIP is installed/registered, NOT whether a focused
/// application's text field is processing keys through it.
pub fn is_tsf_mode_active() -> bool {
    is_tsf_registered() && is_tsf_dll_present()
}

/// Decide whether the foreground hook must yield to the native TSF TIP.
/// The TIP calls UpdateActiveFocus when it receives/loses thread focus;
/// a global TIP registration never implies activation in an unrelated app.
#[inline]
fn should_yield_to_tsf(registered: bool, foreground_pid: u32, focused_pid: u32) -> bool {
    registered && foreground_pid != 0 && foreground_pid == focused_pid
}

/// Read the foreground process focus signal already published by the C++ TIP.
/// On shared-memory contention, yield for this one event to avoid double
/// insertion; never block a WH_KEYBOARD_LL callback on an IPC mutex.
pub fn is_tsf_handling_foreground(foreground: HWND) -> bool {
    if foreground.is_null() {
        return false;
    }
    let mut foreground_pid = 0u32;
    unsafe {
        GetWindowThreadProcessId(foreground, &mut foreground_pid);
    }
    if foreground_pid == 0 {
        return false;
    }
    match SHARED_WRITER.try_lock() {
        Ok(guard) => {
            let Some(writer) = guard.as_ref() else {
                // If there is no published focus state, the hook is the fallback.
                return false;
            };
            if writer.view.is_null() {
                return false;
            }
            let active_pid = unsafe {
                std::ptr::read_volatile(std::ptr::addr_of!((*writer.view).last_active_pid))
            };
            should_yield_to_tsf(true, foreground_pid, active_pid)
        }
        // Avoid double-injection if configuration and focus-state IPC are racing.
        Err(_) => true,
    }
}

#[cfg(test)]
mod focus_tests {
    use super::should_yield_to_tsf;

    #[test]
    fn only_active_foreground_process_owns_tsf_typing() {
        assert!(should_yield_to_tsf(true, 101, 101));
        assert!(!should_yield_to_tsf(true, 101, 202));
        assert!(!should_yield_to_tsf(true, 101, 0));
        assert!(!should_yield_to_tsf(true, 0, 101));
        assert!(!should_yield_to_tsf(false, 101, 101));
    }
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
        let (prev_tick, prev_pid, prev_comp) = if !writer.view.is_null() {
            unsafe {
                (
                    (*writer.view).last_active_tick,
                    (*writer.view).last_active_pid,
                    (*writer.view).is_composing,
                )
            }
        } else {
            (0, 0, 0)
        };
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
            version: 1,
            last_active_tick: prev_tick,
            last_active_pid: prev_pid,
            is_composing: prev_comp,
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
        assert_eq!(std::mem::size_of::<TsfSharedConfig>(), 56);
        let cfg = TsfSharedConfig::default();
        assert_eq!(cfg.enabled, 1);
        assert_eq!(cfg.method, 0);
        assert_eq!(cfg.auto_switch_by_typing, 1);
        assert_eq!(cfg.enable_macro, 1);
        assert_eq!(cfg.version, 1);
        assert_eq!(cfg.last_active_pid, 0);
        assert_eq!(cfg.is_composing, 0);
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
