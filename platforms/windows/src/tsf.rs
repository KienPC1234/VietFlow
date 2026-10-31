// VietFlow Windows - Text Services Framework (TSF) Manager & Cross-Process IPC
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::config::AppConfig;
use crate::tray::to_wide;
use std::path::PathBuf;
use std::sync::Mutex;
use windows_sys::Win32::Foundation::{
    CloseHandle, FreeLibrary, GetLastError, ERROR_CANCELLED, HANDLE, INVALID_HANDLE_VALUE,
    WAIT_OBJECT_0,
};
use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
use windows_sys::Win32::System::Memory::{
    CreateFileMappingW, MapViewOfFile, UnmapViewOfFile, FILE_MAP_ALL_ACCESS,
    MEMORY_MAPPED_VIEW_ADDRESS, PAGE_READWRITE,
};
use windows_sys::Win32::System::Registry::{
    RegCloseKey, RegOpenKeyExW, HKEY, HKEY_LOCAL_MACHINE, KEY_READ,
};
use windows_sys::Win32::System::Threading::{
    GetExitCodeProcess, WaitForSingleObject,
};
use windows_sys::Win32::UI::Shell::{
    ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::SW_HIDE;

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
    pub consonant_shortcuts: u32,
    pub version: u32,
    pub last_active_tick: u64,
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
            consonant_shortcuts: 0,
            version: 1,
            last_active_tick: 0,
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

            let mapping = CreateFileMappingW(
                INVALID_HANDLE_VALUE,
                std::ptr::null_mut(),
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

pub fn sync_tsf_config(config: &AppConfig, enabled: bool) {
    let mut lock = match SHARED_WRITER.lock() {
        Ok(l) => l,
        Err(_) => return,
    };

    if lock.is_none() {
        *lock = SharedConfigWriter::new();
    }

    if let Some(writer) = lock.as_mut() {
        let prev_tick = if !writer.view.is_null() {
            unsafe { (*writer.view).last_active_tick }
        } else {
            0
        };
        let tsf_cfg = TsfSharedConfig {
            enabled: if enabled { 1 } else { 0 },
            method: config.method as u32,
            charset: config.charset as u32,
            modern_style: if config.modern_style { 1 } else { 0 },
            spell_check: if config.spell_check { 1 } else { 0 },
            free_marking: if config.free_marking { 1 } else { 0 },
            auto_restore: if config.auto_restore { 1 } else { 0 },
            consonant_shortcuts: if config.consonant_shortcuts { 1 } else { 0 },
            version: 1,
            last_active_tick: prev_tick,
        };
        writer.update(&tsf_cfg);
    }
}

/// Locates the `vietflow_tip.dll` file.
/// First checks the same directory as the executable, then working directory and target dirs.
pub fn tsf_dll_path() -> Option<PathBuf> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let direct = parent.join("vietflow_tip.dll");
            if direct.is_file() {
                return Some(direct);
            }
            if let Some(grandparent) = parent.parent() {
                let in_profile = grandparent.join("vietflow_tip.dll");
                if in_profile.is_file() {
                    return Some(in_profile);
                }
                if let Some(great) = grandparent.parent() {
                    let in_release = great.join("release").join("vietflow_tip.dll");
                    if in_release.is_file() {
                        return Some(in_release);
                    }
                    let in_debug = great.join("debug").join("vietflow_tip.dll");
                    if in_debug.is_file() {
                        return Some(in_debug);
                    }
                }
            }
        }
    }
    if let Ok(mut dir) = std::env::current_dir() {
        for _ in 0..4 {
            let direct = dir.join("vietflow_tip.dll");
            if direct.is_file() {
                return Some(direct);
            }
            let target_rel = dir.join("target").join("release").join("vietflow_tip.dll");
            if target_rel.is_file() {
                return Some(target_rel);
            }
            let dist_rel = dir.join("dist").join("vietflow_tip.dll");
            if dist_rel.is_file() {
                return Some(dist_rel);
            }
            if !dir.pop() {
                break;
            }
        }
    }
    None
}

/// Returns true if `vietflow_tip.dll` is present on the system.
/// If false, any TSF IME options must be completely hidden from user.
pub fn is_tsf_dll_present() -> bool {
    tsf_dll_path().is_some()
}

/// Checks whether the VietFlow TIP COM CLSID and LanguageProfile are registered in HKLM CTF.
pub fn is_tsf_registered() -> bool {
    unsafe {
        let subkey = to_wide("SOFTWARE\\Microsoft\\CTF\\TIP\\{7C2B5E10-9A31-4B8F-B623-192E48270101}\\LanguageProfile\\0x00000409\0");
        let mut hkey: HKEY = std::ptr::null_mut();
        let res = RegOpenKeyExW(
            HKEY_LOCAL_MACHINE,
            subkey.as_ptr(),
            0,
            KEY_READ,
            &mut hkey,
        );
        if res == 0 {
            RegCloseKey(hkey);
            true
        } else {
            false
        }
    }
}

/// Activates or deactivates the VietFlow TSF profile in the system.
pub fn set_tsf_profile_active(active: bool) {
    if active {
        call_set_default_layout_or_tip(
            "0409:{7c2b5e10-9a31-4b8f-b623-192e48270101}{7c2b5e11-9a31-4b8f-b623-192e48270102}",
        );
    }
}

pub fn call_set_default_layout_or_tip(layout_str: &str) -> bool {
    unsafe {
        let input_mod = LoadLibraryW(to_wide("input.dll\0").as_ptr());
        if input_mod.is_null() {
            return false;
        }
        let proc = GetProcAddress(input_mod, b"SetDefaultLayoutOrTip\0".as_ptr() as *const _);
        let mut ok = false;
        if let Some(func) = proc {
            type FnSetDefaultLayoutOrTip = unsafe extern "system" fn(*const u16, u32) -> windows_sys::core::BOOL;
            let func: FnSetDefaultLayoutOrTip = std::mem::transmute(func);
            let wide = to_wide(&format!("{layout_str}\0"));
            ok = func(wide.as_ptr(), 0) != 0;
        }
        FreeLibrary(input_mod);
        ok
    }
}

const ILOT_UNINSTALL: u32 = 0x00000001;

pub fn call_install_layout_or_tip(layout_str: &str, uninstall: bool) -> bool {
    unsafe {
        let input_mod = LoadLibraryW(to_wide("input.dll\0").as_ptr());
        if input_mod.is_null() {
            return false;
        }
        let proc = GetProcAddress(input_mod, b"InstallLayoutOrTip\0".as_ptr() as *const _);
        let mut ok = false;
        if let Some(func) = proc {
            type FnInstallLayoutOrTip = unsafe extern "system" fn(*const u16, u32) -> windows_sys::core::BOOL;
            let func: FnInstallLayoutOrTip = std::mem::transmute(func);
            let wide = to_wide(&format!("{layout_str}\0"));
            let flags = if uninstall { ILOT_UNINSTALL } else { 0 };
            ok = func(wide.as_ptr(), flags) != 0;
        }
        FreeLibrary(input_mod);
        ok
    }
}

fn run_elevated_wait(file: &str, params: &str) -> Result<(), String> {
    unsafe {
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
                return Err("Thao tác bị hủy bởi người dùng (yêu cầu quyền Administrator).".to_string());
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

/// Registers the VietFlow TSF in-process COM DLL via regsvr32 with elevation.
pub fn register_tsf() -> Result<(), String> {
    let dll = tsf_dll_path().ok_or_else(|| "vietflow_tip.dll không tồn tại.".to_string())?;

    // 1. Run elevated regsvr32 to register COM server and HKLM CTF TIP entries
    let params = format!("/s \"{}\"", dll.to_string_lossy());
    run_elevated_wait("regsvr32.exe", &params)?;

    // 2. Add layout to active user keyboard layout list via input.dll (English US only)
    call_install_layout_or_tip(
        "0409:{7c2b5e10-9a31-4b8f-b623-192e48270101}{7c2b5e11-9a31-4b8f-b623-192e48270102}",
        false,
    );
    // Explicitly uninstall 042a in case an older build registered it, preventing Windows from pulling in Vietnamese
    call_install_layout_or_tip(
        "042a:{7c2b5e10-9a31-4b8f-b623-192e48270101}{7c2b5e11-9a31-4b8f-b623-192e48270102}",
        true,
    );

    Ok(())
}

/// Unregisters the VietFlow TSF in-process COM DLL via regsvr32 with elevation.
pub fn unregister_tsf() -> Result<(), String> {
    // 1. Remove layout from active user keyboard layout list so Windows immediately deactivates it
    call_install_layout_or_tip(
        "0409:{7c2b5e10-9a31-4b8f-b623-192e48270101}{7c2b5e11-9a31-4b8f-b623-192e48270102}",
        true,
    );
    call_install_layout_or_tip(
        "042a:{7c2b5e10-9a31-4b8f-b623-192e48270101}{7c2b5e11-9a31-4b8f-b623-192e48270102}",
        true,
    );

    // 2. Run elevated regsvr32 /u to wipe COM and HKLM CTF TIP registrations
    let dll = tsf_dll_path().ok_or_else(|| "vietflow_tip.dll không tồn tại.".to_string())?;
    let params = format!("/u /s \"{}\"", dll.to_string_lossy());
    run_elevated_wait("regsvr32.exe", &params)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tsf_shared_config_layout() {
        assert_eq!(std::mem::size_of::<TsfSharedConfig>(), 48);
        let cfg = TsfSharedConfig::default();
        assert_eq!(cfg.enabled, 1);
        assert_eq!(cfg.method, 0);
        assert_eq!(cfg.consonant_shortcuts, 0);
        assert_eq!(cfg.version, 1);
    }

    #[test]
    fn test_tsf_dll_path_search() {
        let _ = tsf_dll_path();
    }

    #[test]
    fn test_tsf_registration_lifecycle() {
        let _ = is_tsf_registered();
    }
}
