// VietFlow Windows TSF - COM & TSF Profile Registration
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::clsid::{
    CLSID_VIETFLOW_TIP, GUID_VIETFLOW_PROFILE, LANGID_ENGLISH_US, LANGID_VIETNAMESE,
    TEXT_SERVICE_NAME, TEXT_SERVICE_PROFILE_NAME,
};
use windows::core::{HRESULT, PCWSTR};
use windows::Win32::Foundation::{HMODULE, S_OK};
use windows::Win32::System::Com::{
    CoCreateInstance, CLSCTX_INPROC_SERVER,
};
use windows::Win32::System::LibraryLoader::GetModuleFileNameW;
use windows::Win32::System::Registry::{
    RegCloseKey, RegCreateKeyExW, RegDeleteTreeW, RegSetValueExW, HKEY, HKEY_CLASSES_ROOT,
    HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_ALL_ACCESS, REG_OPTION_NON_VOLATILE, REG_SZ,
};
use windows::Win32::UI::Input::KeyboardAndMouse::HKL;
use windows::Win32::UI::TextServices::{
    CLSID_TF_CategoryMgr, CLSID_TF_InputProcessorProfiles, GUID_TFCAT_CATEGORY_OF_TIP,
    GUID_TFCAT_TIPCAP_COMLESS, GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
    GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, GUID_TFCAT_TIPCAP_SECUREMODE,
    GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
    GUID_TFCAT_TIP_KEYBOARD, ITfCategoryMgr, ITfInputProcessorProfileMgr,
    ITfInputProcessorProfiles,
};

unsafe fn register_clsid_in_root(
    root: HKEY,
    subkey_prefix: &str,
    clsid_guid: &windows::core::GUID,
    module_path: &[u16],
    len: u32,
) -> Result<(), ()> {
    let clsid_str = format!("{subkey_prefix}\\{{{clsid_guid:?}}}\0");
    let clsid_wide: Vec<u16> = clsid_str.encode_utf16().collect();

    let mut key: HKEY = HKEY::default();
    if RegCreateKeyExW(
        root,
        PCWSTR(clsid_wide.as_ptr()),
        None,
        None,
        REG_OPTION_NON_VOLATILE,
        KEY_ALL_ACCESS,
        None,
        &mut key,
        None,
    )
    .is_err()
    {
        return Err(());
    }

    let name_wide: Vec<u16> = format!("{TEXT_SERVICE_NAME}\0").encode_utf16().collect();
    let _ = RegSetValueExW(
        key,
        None,
        Some(0),
        REG_SZ,
        Some(std::slice::from_raw_parts(
            name_wide.as_ptr() as *const u8,
            name_wide.len() * 2,
        )),
    );

    let mut inproc_key: HKEY = HKEY::default();
    let inproc_wide: Vec<u16> = "InprocServer32\0".encode_utf16().collect();
    if RegCreateKeyExW(
        key,
        PCWSTR(inproc_wide.as_ptr()),
        None,
        None,
        REG_OPTION_NON_VOLATILE,
        KEY_ALL_ACCESS,
        None,
        &mut inproc_key,
        None,
    )
    .is_ok()
    {
        let _ = RegSetValueExW(
            inproc_key,
            None,
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(
                module_path.as_ptr() as *const u8,
                (len as usize + 1) * 2,
            )),
        );

        let threading_wide: Vec<u16> = "Apartment\0".encode_utf16().collect();
        let tm_wide: Vec<u16> = "ThreadingModel\0".encode_utf16().collect();
        let _ = RegSetValueExW(
            inproc_key,
            PCWSTR(tm_wide.as_ptr()),
            Some(0),
            REG_SZ,
            Some(std::slice::from_raw_parts(
                threading_wide.as_ptr() as *const u8,
                threading_wide.len() * 2,
            )),
        );
        let _ = RegCloseKey(inproc_key);
    }
    let _ = RegCloseKey(key);
    Ok(())
}

unsafe fn reg_create(root: HKEY, subkey: &str) -> Option<HKEY> {
    let subkey_null = format!("{subkey}\0");
    let wide: Vec<u16> = subkey_null.encode_utf16().collect();
    let mut key = HKEY::default();
    if RegCreateKeyExW(
        root,
        PCWSTR(wide.as_ptr()),
        None,
        None,
        REG_OPTION_NON_VOLATILE,
        KEY_ALL_ACCESS,
        None,
        &mut key,
        None,
    )
    .is_ok()
    {
        Some(key)
    } else {
        None
    }
}

unsafe fn reg_set_sz(key: HKEY, name: Option<&str>, val: &str) {
    let name_wide: Option<Vec<u16>> = name.map(|n| format!("{n}\0").encode_utf16().collect());
    let val_wide: Vec<u16> = format!("{val}\0").encode_utf16().collect();
    let _ = RegSetValueExW(
        key,
        name_wide.as_ref().map(|w| PCWSTR(w.as_ptr())).unwrap_or(PCWSTR(std::ptr::null())),
        Some(0),
        REG_SZ,
        Some(std::slice::from_raw_parts(
            val_wide.as_ptr() as *const u8,
            val_wide.len() * 2,
        )),
    );
}

unsafe fn reg_set_dword(key: HKEY, name: &str, val: u32) {
    let name_wide: Vec<u16> = format!("{name}\0").encode_utf16().collect();
    let _ = RegSetValueExW(
        key,
        PCWSTR(name_wide.as_ptr()),
        Some(0),
        windows::Win32::System::Registry::REG_DWORD,
        Some(std::slice::from_raw_parts(
            &val as *const u32 as *const u8,
            4,
        )),
    );
}

pub unsafe fn register_server(hmodule: HMODULE) -> HRESULT {
    let mut module_path = [0u16; 1024];
    let len = GetModuleFileNameW(Some(hmodule), &mut module_path);
    if len == 0 {
        return windows::core::HRESULT(-1);
    }

    // 1. Register COM CLSID in HKCU, HKLM, and HKCR
    let _ = register_clsid_in_root(HKEY_CURRENT_USER, "Software\\Classes\\CLSID", &CLSID_VIETFLOW_TIP, &module_path, len);
    let _ = register_clsid_in_root(HKEY_LOCAL_MACHINE, "SOFTWARE\\Classes\\CLSID", &CLSID_VIETFLOW_TIP, &module_path, len);
    let _ = register_clsid_in_root(HKEY_CLASSES_ROOT, "CLSID", &CLSID_VIETFLOW_TIP, &module_path, len);

    // 2. Direct HKLM write for CTF TIP LanguageProfile and Category
    let tip_guid_str = format!("{{{CLSID_VIETFLOW_TIP:?}}}");
    let prof_guid_str = format!("{{{GUID_VIETFLOW_PROFILE:?}}}");
    let module_path_str = String::from_utf16_lossy(&module_path[..len as usize]);

    // Profile for English US (0x00000409)
    let path_0409 = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{tip_guid_str}\\LanguageProfile\\0x00000409\\{prof_guid_str}");
    if let Some(k) = reg_create(HKEY_LOCAL_MACHINE, &path_0409) {
        reg_set_sz(k, Some("Description"), TEXT_SERVICE_PROFILE_NAME);
        reg_set_sz(k, Some("Display Description"), TEXT_SERVICE_PROFILE_NAME);
        reg_set_dword(k, "Enable", 1);
        reg_set_dword(k, "SubstituteLayout", 0x04090409);
        reg_set_sz(k, Some("IconFile"), &module_path_str);
        reg_set_dword(k, "IconIndex", 0);
        reg_set_dword(k, "ProfileFlags", 2);
        let _ = RegCloseKey(k);
    }

    // Profile for Vietnamese (0x0000042a)
    let path_042a = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{tip_guid_str}\\LanguageProfile\\0x0000042a\\{prof_guid_str}");
    if let Some(k) = reg_create(HKEY_LOCAL_MACHINE, &path_042a) {
        reg_set_sz(k, Some("Description"), TEXT_SERVICE_PROFILE_NAME);
        reg_set_sz(k, Some("Display Description"), TEXT_SERVICE_PROFILE_NAME);
        reg_set_dword(k, "Enable", 1);
        reg_set_dword(k, "SubstituteLayout", 0x04090409);
        reg_set_sz(k, Some("IconFile"), &module_path_str);
        reg_set_dword(k, "IconIndex", 0);
        reg_set_dword(k, "ProfileFlags", 2);
        let _ = RegCloseKey(k);
    }

    // Category registrations (Category of TIP, Keyboard TIP, and capabilities)
    let categories: [(windows::core::GUID, windows::core::GUID); 8] = [
        (GUID_TFCAT_CATEGORY_OF_TIP, GUID_TFCAT_TIP_KEYBOARD),
        (GUID_TFCAT_TIP_KEYBOARD, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_UIELEMENTENABLED, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_SECUREMODE, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_COMLESS, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, CLSID_VIETFLOW_TIP),
    ];

    for (cat, item) in &categories {
        let cat_str = format!("{{{cat:?}}}");
        let item_str = format!("{{{item:?}}}");
        let cat_path = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{tip_guid_str}\\Category\\Category\\{cat_str}\\{item_str}");
        if let Some(k) = reg_create(HKEY_LOCAL_MACHINE, &cat_path) {
            let _ = RegCloseKey(k);
        }
        let item_path = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{tip_guid_str}\\Category\\Item\\{item_str}\\{cat_str}");
        if let Some(k) = reg_create(HKEY_LOCAL_MACHINE, &item_path) {
            let _ = RegCloseKey(k);
        }
    }

    // 3. Register TSF Categories via COM
    let cat_mgr: Result<ITfCategoryMgr, _> =
        CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER);

    if let Ok(mgr) = cat_mgr {
        for (cat, item) in &categories {
            let _ = mgr.RegisterCategory(&CLSID_VIETFLOW_TIP, cat, item);
        }
    }

    // 4. Register Modern TSF Input Processor Profile via ITfInputProcessorProfileMgr
    let profile_mgr_v2: Result<ITfInputProcessorProfileMgr, _> =
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER);

    let desc_wide: Vec<u16> = format!("{TEXT_SERVICE_PROFILE_NAME}\0").encode_utf16().collect();
    let desc_slice = &desc_wide[..desc_wide.len() - 1];
    let icon_slice = &module_path[..len as usize];
    let hkl_us = HKL(0x04090409 as _);

    if let Ok(mgr_v2) = profile_mgr_v2 {
        let _ = mgr_v2.RegisterProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_ENGLISH_US,
            &GUID_VIETFLOW_PROFILE,
            desc_slice,
            icon_slice,
            0,
            hkl_us,
            0,
            true,
            0,
        );

        let _ = mgr_v2.RegisterProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_VIETNAMESE,
            &GUID_VIETFLOW_PROFILE,
            desc_slice,
            icon_slice,
            0,
            hkl_us,
            0,
            true,
            0,
        );
    }

    // 5. Register Legacy TSF Input Processor Profile via ITfInputProcessorProfiles for backwards compatibility
    let profile_mgr: Result<ITfInputProcessorProfiles, _> =
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER);

    if let Ok(mgr) = profile_mgr {
        let _ = mgr.Register(&CLSID_VIETFLOW_TIP);

        // Register for English US (0x0409)
        let _ = mgr.AddLanguageProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_ENGLISH_US,
            &GUID_VIETFLOW_PROFILE,
            desc_slice,
            icon_slice,
            0,
        );
        let _ = mgr.EnableLanguageProfile(&CLSID_VIETFLOW_TIP, LANGID_ENGLISH_US, &GUID_VIETFLOW_PROFILE, true);

        // Register for Vietnamese (0x042A)
        let _ = mgr.AddLanguageProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_VIETNAMESE,
            &GUID_VIETFLOW_PROFILE,
            desc_slice,
            icon_slice,
            0,
        );
        let _ = mgr.EnableLanguageProfile(&CLSID_VIETFLOW_TIP, LANGID_VIETNAMESE, &GUID_VIETFLOW_PROFILE, true);
    }

    // 6. Register active layout in input.dll for current user
    call_install_layout_or_tip(
        "0409:{7C2B5E10-9A31-4B8F-B623-192E48270101}{7C2B5E11-9A31-4B8F-B623-192E48270102}",
        false,
    );
    call_install_layout_or_tip(
        "042A:{7C2B5E10-9A31-4B8F-B623-192E48270101}{7C2B5E11-9A31-4B8F-B623-192E48270102}",
        false,
    );

    S_OK
}

unsafe fn call_install_layout_or_tip(layout: &str, uninstall: bool) {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::Foundation::FreeLibrary;
    use windows::core::BOOL;

    let input_name: Vec<u16> = "input.dll\0".encode_utf16().collect();
    if let Ok(mod_handle) = LoadLibraryW(PCWSTR(input_name.as_ptr())) {
        let fn_name = std::ffi::CString::new("InstallLayoutOrTip").unwrap();
        if let Some(proc) = GetProcAddress(mod_handle, windows::core::PCSTR(fn_name.as_ptr() as *const u8)) {
            type FnType = unsafe extern "system" fn(*const u16, u32) -> BOOL;
            let func: FnType = std::mem::transmute(proc);
            let wide: Vec<u16> = format!("{layout}\0").encode_utf16().collect();
            let flags = if uninstall { 1 } else { 0 };
            let _ = func(wide.as_ptr(), flags);
        }
        let _ = FreeLibrary(mod_handle);
    }
}

pub unsafe fn unregister_server() -> HRESULT {
    // 0. Remove active layout in input.dll
    call_install_layout_or_tip(
        "0409:{7C2B5E10-9A31-4B8F-B623-192E48270101}{7C2B5E11-9A31-4B8F-B623-192E48270102}",
        true,
    );
    call_install_layout_or_tip(
        "042A:{7C2B5E10-9A31-4B8F-B623-192E48270101}{7C2B5E11-9A31-4B8F-B623-192E48270102}",
        true,
    );

    // 1. Unregister TSF Categories via COM
    let cat_mgr: Result<ITfCategoryMgr, _> =
        CoCreateInstance(&CLSID_TF_CategoryMgr, None, CLSCTX_INPROC_SERVER);

    let categories: [(windows::core::GUID, windows::core::GUID); 8] = [
        (GUID_TFCAT_CATEGORY_OF_TIP, GUID_TFCAT_TIP_KEYBOARD),
        (GUID_TFCAT_TIP_KEYBOARD, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_UIELEMENTENABLED, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_SECUREMODE, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_COMLESS, CLSID_VIETFLOW_TIP),
        (GUID_TFCAT_TIPCAP_INPUTMODECOMPARTMENT, CLSID_VIETFLOW_TIP),
    ];

    if let Ok(mgr) = cat_mgr {
        for (cat, item) in &categories {
            let _ = mgr.UnregisterCategory(&CLSID_VIETFLOW_TIP, cat, item);
        }
    }

    // 2. Unregister Modern TSF Profiles via ITfInputProcessorProfileMgr
    let profile_mgr_v2: Result<ITfInputProcessorProfileMgr, _> =
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER);

    if let Ok(mgr_v2) = profile_mgr_v2 {
        let _ = mgr_v2.UnregisterProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_ENGLISH_US,
            &GUID_VIETFLOW_PROFILE,
            0,
        );
        let _ = mgr_v2.UnregisterProfile(
            &CLSID_VIETFLOW_TIP,
            LANGID_VIETNAMESE,
            &GUID_VIETFLOW_PROFILE,
            0,
        );
    }

    // 3. Unregister Legacy TSF Profiles via ITfInputProcessorProfiles
    let profile_mgr: Result<ITfInputProcessorProfiles, _> =
        CoCreateInstance(&CLSID_TF_InputProcessorProfiles, None, CLSCTX_INPROC_SERVER);

    if let Ok(mgr) = profile_mgr {
        let _ = mgr.Unregister(&CLSID_VIETFLOW_TIP);
    }

    // 3. Delete registry keys from HKLM CTF TIP tree
    let tip_guid_str = format!("{{{CLSID_VIETFLOW_TIP:?}}}");
    let ctf_tip_path = format!("SOFTWARE\\Microsoft\\CTF\\TIP\\{tip_guid_str}\0");
    let ctf_wide: Vec<u16> = ctf_tip_path.encode_utf16().collect();
    let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, PCWSTR(ctf_wide.as_ptr()));

    // 4. Delete COM CLSID registrations
    let clsid_hkcu = format!("Software\\Classes\\CLSID\\{{{CLSID_VIETFLOW_TIP:?}}}\0");
    let clsid_hkcu_wide: Vec<u16> = clsid_hkcu.encode_utf16().collect();
    let _ = RegDeleteTreeW(HKEY_CURRENT_USER, PCWSTR(clsid_hkcu_wide.as_ptr()));

    let clsid_hklm = format!("SOFTWARE\\Classes\\CLSID\\{{{CLSID_VIETFLOW_TIP:?}}}\0");
    let clsid_hklm_wide: Vec<u16> = clsid_hklm.encode_utf16().collect();
    let _ = RegDeleteTreeW(HKEY_LOCAL_MACHINE, PCWSTR(clsid_hklm_wide.as_ptr()));

    let clsid_hkcr = format!("CLSID\\{{{CLSID_VIETFLOW_TIP:?}}}\0");
    let clsid_hkcr_wide: Vec<u16> = clsid_hkcr.encode_utf16().collect();
    let _ = RegDeleteTreeW(HKEY_CLASSES_ROOT, PCWSTR(clsid_hkcr_wide.as_ptr()));

    S_OK
}
