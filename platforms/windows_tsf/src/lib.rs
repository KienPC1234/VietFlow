// VietFlow Windows TSF - In-Process COM DLL Entry Points
// Licensed under GNU General Public License v3.0 (GPLv3)

mod class_factory;
pub mod clsid;
mod edit_session;
pub mod ipc;
mod registry;
mod tip;

use clsid::CLSID_VIETFLOW_TIP;
use std::ffi::c_void;
use std::sync::atomic::{AtomicUsize, Ordering};
use windows::core::{BOOL, GUID, HRESULT, Interface};
use windows::Win32::Foundation::{
    CLASS_E_CLASSNOTAVAILABLE, E_INVALIDARG, HINSTANCE, HMODULE, S_FALSE, S_OK,
};
use windows::Win32::System::SystemServices::DLL_PROCESS_ATTACH;

static mut MODULE_HANDLE: HMODULE = HMODULE(std::ptr::null_mut());
pub static DLL_REF_COUNT: AtomicUsize = AtomicUsize::new(0);

/// Standard in-process COM DLL entry point.
///
/// # Safety
/// Called by Windows OS loader when loading or unloading this library module.
#[no_mangle]
pub unsafe extern "system" fn DllMain(
    hinstance: HINSTANCE,
    dwreason: u32,
    _reserved: *mut c_void,
) -> BOOL {
    if dwreason == DLL_PROCESS_ATTACH {
        MODULE_HANDLE = HMODULE(hinstance.0);
    }
    BOOL(1)
}

/// Retrieves the class object from a DLL object handler or object application.
///
/// # Safety
/// Caller must pass valid pointers for `rclsid`, `riid`, and an allocated destination `ppv`.
#[no_mangle]
pub unsafe extern "system" fn DllGetClassObject(
    rclsid: *const GUID,
    riid: *const GUID,
    ppv: *mut *mut c_void,
) -> HRESULT {
    if rclsid.is_null() || riid.is_null() || ppv.is_null() {
        return E_INVALIDARG;
    }
    if *rclsid != CLSID_VIETFLOW_TIP {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    let factory = class_factory::VietFlowClassFactory;
    let unk: windows::core::IUnknown = factory.into();
    unk.query(riid, ppv)
}

/// Determines whether the DLL is in use and can be unloaded.
///
/// # Safety
/// Standard COM export called by OLE/COM subsystem.
#[no_mangle]
pub unsafe extern "system" fn DllCanUnloadNow() -> HRESULT {
    if DLL_REF_COUNT.load(Ordering::SeqCst) == 0 {
        S_OK
    } else {
        S_FALSE
    }
}

/// Instructs an in-process server to create its registry entries.
///
/// # Safety
/// Modifies HKLM/HKCR registry settings; requires administrative privileges.
#[no_mangle]
pub unsafe extern "system" fn DllRegisterServer() -> HRESULT {
    registry::register_server(MODULE_HANDLE)
}

/// Instructs an in-process server to remove only those entries created through `DllRegisterServer`.
///
/// # Safety
/// Modifies HKLM/HKCR registry settings; requires administrative privileges.
#[no_mangle]
pub unsafe extern "system" fn DllUnregisterServer() -> HRESULT {
    registry::unregister_server()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_clsid_constants() {
        assert_eq!(clsid::LANGID_ENGLISH_US, 0x0409);
        assert_eq!(clsid::LANGID_VIETNAMESE, 0x042A);
        assert_eq!(clsid::TEXT_SERVICE_PROFILE_NAME, "VietFlow");
    }
}
