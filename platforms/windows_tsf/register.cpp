// VietFlow Windows TSF - COM & TSF Profile Registration
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "register.h"
#include "clsid.h"
#include "debug_log.h"
#include <msctf.h>
#include <string>

typedef BOOL (WINAPI *FnInstallLayoutOrTip)(LPCWSTR psz, DWORD dwFlags);

static void CallInstallLayoutOrTip(const std::wstring& layout, bool uninstall) {
    HMODULE hInput = LoadLibraryW(L"input.dll");
    if (hInput) {
        FnInstallLayoutOrTip pfn = reinterpret_cast<FnInstallLayoutOrTip>(GetProcAddress(hInput, "InstallLayoutOrTip"));
        if (pfn) {
            pfn(layout.c_str(), uninstall ? 1 : 0);
        }
        FreeLibrary(hInput);
    }
}

static bool RegSetString(HKEY hKey, const wchar_t* subKey, const wchar_t* valName, const wchar_t* value) {
    HKEY hSub = nullptr;
    LSTATUS status = RegCreateKeyExW(hKey, subKey, 0, nullptr, REG_OPTION_NON_VOLATILE, KEY_WRITE, nullptr, &hSub, nullptr);
    if (status != ERROR_SUCCESS) {
        VF_LOG(L"RegSetString: RegCreateKeyExW failed for %s (hKey=%p), err=%u", subKey, hKey, status);
        return false;
    }
    DWORD bytes = static_cast<DWORD>((wcslen(value) + 1) * sizeof(wchar_t));
    status = RegSetValueExW(hSub, valName, 0, REG_SZ, reinterpret_cast<const BYTE*>(value), bytes);
    if (status != ERROR_SUCCESS) {
        VF_LOG(L"RegSetString: RegSetValueExW failed for %s / %s, err=%u", subKey, valName ? valName : L"(Default)", status);
    }
    RegCloseKey(hSub);
    return (status == ERROR_SUCCESS);
}

static bool RegSetDword(HKEY hKey, const wchar_t* subKey, const wchar_t* valName, DWORD value) {
    HKEY hSub = nullptr;
    LSTATUS status = RegCreateKeyExW(hKey, subKey, 0, nullptr, REG_OPTION_NON_VOLATILE, KEY_WRITE, nullptr, &hSub, nullptr);
    if (status != ERROR_SUCCESS) {
        VF_LOG(L"RegSetDword: RegCreateKeyExW failed for %s (hKey=%p), err=%u", subKey, hKey, status);
        return false;
    }
    status = RegSetValueExW(hSub, valName, 0, REG_DWORD, reinterpret_cast<const BYTE*>(&value), sizeof(DWORD));
    if (status != ERROR_SUCCESS) {
        VF_LOG(L"RegSetDword: RegSetValueExW failed for %s / %s, err=%u", subKey, valName ? valName : L"(Default)", status);
    }
    RegCloseKey(hSub);
    return (status == ERROR_SUCCESS);
}

static void RegDeleteSubTree(HKEY hRoot, const wchar_t* subKey) {
    RegDeleteTreeW(hRoot, subKey);
}

static void CleanupLegacyRegistrations() {
    std::wstring oldLayout0409 = std::wstring(L"0409:") + c_szLegacyClsidStr + c_szLegacyProfileStr;
    std::wstring oldLayout042a = std::wstring(L"042A:") + c_szLegacyClsidStr + c_szLegacyProfileStr;
    CallInstallLayoutOrTip(oldLayout0409, true);
    CallInstallLayoutOrTip(oldLayout042a, true);

    std::wstring oldTipCtfHklm = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\TIP\\") + c_szLegacyClsidStr;
    std::wstring oldTipCtfHkcu = std::wstring(L"Software\\Microsoft\\CTF\\TIP\\") + c_szLegacyClsidStr;
    std::wstring oldSortHkcu = std::wstring(L"Software\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\") + c_szLegacyClsidStr;
    std::wstring oldSortHklm = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\") + c_szLegacyClsidStr;

    std::wstring oldClsidHklm = std::wstring(L"SOFTWARE\\Classes\\CLSID\\") + c_szLegacyClsidStr;
    std::wstring oldClsidHkcu = std::wstring(L"Software\\Classes\\CLSID\\") + c_szLegacyClsidStr;
    std::wstring oldClsidHkcr = std::wstring(L"CLSID\\") + c_szLegacyClsidStr;

    RegDeleteSubTree(HKEY_LOCAL_MACHINE, oldTipCtfHklm.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, oldTipCtfHkcu.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, oldSortHkcu.c_str());
    RegDeleteSubTree(HKEY_LOCAL_MACHINE, oldSortHklm.c_str());
    RegDeleteSubTree(HKEY_LOCAL_MACHINE, oldClsidHklm.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, oldClsidHkcu.c_str());
    RegDeleteSubTree(HKEY_CLASSES_ROOT, oldClsidHkcr.c_str());
}

HRESULT RegisterServer(HMODULE hModule) {
    wchar_t modPath[MAX_PATH] = {};
    if (GetModuleFileNameW(hModule, modPath, MAX_PATH) == 0) {
        return E_FAIL;
    }

    CleanupLegacyRegistrations();

    // 1. Register COM InprocServer32 in HKLM, HKCU, and HKCR
    std::wstring clsidSubkeyHklm = std::wstring(L"SOFTWARE\\Classes\\CLSID\\") + c_szClsidStr;
    std::wstring inprocSubkeyHklm = clsidSubkeyHklm + L"\\InprocServer32";
    RegSetString(HKEY_LOCAL_MACHINE, clsidSubkeyHklm.c_str(), nullptr, c_szTextServiceName);
    RegSetString(HKEY_LOCAL_MACHINE, inprocSubkeyHklm.c_str(), nullptr, modPath);
    RegSetString(HKEY_LOCAL_MACHINE, inprocSubkeyHklm.c_str(), L"ThreadingModel", L"Apartment");

    std::wstring clsidSubkeyHkcu = std::wstring(L"Software\\Classes\\CLSID\\") + c_szClsidStr;
    std::wstring inprocSubkeyHkcu = clsidSubkeyHkcu + L"\\InprocServer32";
    RegSetString(HKEY_CURRENT_USER, clsidSubkeyHkcu.c_str(), nullptr, c_szTextServiceName);
    RegSetString(HKEY_CURRENT_USER, inprocSubkeyHkcu.c_str(), nullptr, modPath);
    RegSetString(HKEY_CURRENT_USER, inprocSubkeyHkcu.c_str(), L"ThreadingModel", L"Apartment");

    std::wstring clsidSubkeyHkcr = std::wstring(L"CLSID\\") + c_szClsidStr;
    std::wstring inprocSubkeyHkcr = clsidSubkeyHkcr + L"\\InprocServer32";
    RegSetString(HKEY_CLASSES_ROOT, clsidSubkeyHkcr.c_str(), nullptr, c_szTextServiceName);
    RegSetString(HKEY_CLASSES_ROOT, inprocSubkeyHkcr.c_str(), nullptr, modPath);
    RegSetString(HKEY_CLASSES_ROOT, inprocSubkeyHkcr.c_str(), L"ThreadingModel", L"Apartment");

    // 2. Direct HKLM & HKCU CTF TIP LanguageProfile writes (without icon and without SubstituteLayout)
    std::wstring profileKeyHklm = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\TIP\\") + c_szClsidStr +
                                  L"\\LanguageProfile\\0x00000409\\" + c_szProfileStr;
    RegSetString(HKEY_LOCAL_MACHINE, profileKeyHklm.c_str(), L"Description", c_szTextServiceProfileName);
    RegSetString(HKEY_LOCAL_MACHINE, profileKeyHklm.c_str(), L"Display Description", c_szTextServiceProfileName);
    RegSetDword(HKEY_LOCAL_MACHINE, profileKeyHklm.c_str(), L"Enable", 1);
    RegSetDword(HKEY_LOCAL_MACHINE, profileKeyHklm.c_str(), L"ProfileFlags", 0);

    std::wstring profileKeyHkcu = std::wstring(L"Software\\Microsoft\\CTF\\TIP\\") + c_szClsidStr +
                                  L"\\LanguageProfile\\0x00000409\\" + c_szProfileStr;
    RegSetString(HKEY_CURRENT_USER, profileKeyHkcu.c_str(), L"Description", c_szTextServiceProfileName);
    RegSetString(HKEY_CURRENT_USER, profileKeyHkcu.c_str(), L"Display Description", c_szTextServiceProfileName);
    RegSetDword(HKEY_CURRENT_USER, profileKeyHkcu.c_str(), L"Enable", 1);

    // 3. Register TSF Categories via COM (matching Microsoft SampleIME)
    ITfCategoryMgr* pCatMgr = nullptr;
    HRESULT hrCat = CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr, (void**)&pCatMgr);
    VF_LOG(L"RegisterServer: CoCreateInstance(CLSID_TF_CategoryMgr) hr=0x%08X", hrCat);
    if (SUCCEEDED(hrCat) && pCatMgr) {
        const GUID categories[] = {
            GUID_TFCAT_TIP_KEYBOARD,
            GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
            GUID_TFCAT_TIPCAP_SECUREMODE,
            GUID_TFCAT_TIPCAP_COMLESS,
            GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
            GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT
        };
        for (const auto& cat : categories) {
            pCatMgr->RegisterCategory(CLSID_VietFlowTip, cat, CLSID_VietFlowTip);
        }
        pCatMgr->Release();
    }

    // 4. Register Modern TSF Profile via ITfInputProcessorProfileMgr (hklSubstitute = NULL)
    ITfInputProcessorProfileMgr* pProfMgrV2 = nullptr;
    HRESULT hrProfV2 = CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfileMgr, (void**)&pProfMgrV2);
    VF_LOG(L"RegisterServer: CoCreateInstance(ITfInputProcessorProfileMgr) hr=0x%08X", hrProfV2);
    if (SUCCEEDED(hrProfV2) && pProfMgrV2) {
        HRESULT hrReg = pProfMgrV2->RegisterProfile(
            CLSID_VietFlowTip,
            c_langidEnglishUS,
            GUID_VietFlowProfile,
            c_szTextServiceProfileName,
            static_cast<ULONG>(wcslen(c_szTextServiceProfileName)),
            modPath,
            static_cast<ULONG>(wcslen(modPath)),
            0,
            NULL,
            0,
            TRUE,
            0
        );
        (void)hrReg;
        VF_LOG(L"RegisterServer: ITfInputProcessorProfileMgr::RegisterProfile hr=0x%08X", hrReg);
        pProfMgrV2->Release();
    }

    // 5. Install active keyboard layout via input.dll (English US only)
    std::wstring layout0409 = std::wstring(L"0409:") + c_szClsidStr + c_szProfileStr;
    std::wstring layout042a = std::wstring(L"042A:") + c_szClsidStr + c_szProfileStr;
    CallInstallLayoutOrTip(layout0409, false);
    CallInstallLayoutOrTip(layout042a, true);
    VF_LOG(L"RegisterServer: CallInstallLayoutOrTip completed for %s", layout0409.c_str());

    return S_OK;
}

HRESULT UnregisterServer() {
    VF_LOG(L"UnregisterServer: starting");
    // 0. Uninstall layouts via input.dll
    std::wstring layout0409 = std::wstring(L"0409:") + c_szClsidStr + c_szProfileStr;
    std::wstring layout042a = std::wstring(L"042A:") + c_szClsidStr + c_szProfileStr;
    CallInstallLayoutOrTip(layout0409, true);
    CallInstallLayoutOrTip(layout042a, true);

    CleanupLegacyRegistrations();

    // 1. Unregister Categories
    ITfCategoryMgr* pCatMgr = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_CategoryMgr, nullptr, CLSCTX_INPROC_SERVER, IID_ITfCategoryMgr, (void**)&pCatMgr)) && pCatMgr) {
        const GUID categories[] = {
            GUID_TFCAT_TIP_KEYBOARD,
            GUID_TFCAT_TIPCAP_UIELEMENTENABLED,
            GUID_TFCAT_TIPCAP_SECUREMODE,
            GUID_TFCAT_TIPCAP_COMLESS,
            GUID_TFCAT_TIPCAP_IMMERSIVESUPPORT,
            GUID_TFCAT_TIPCAP_SYSTRAYSUPPORT
        };
        for (const auto& cat : categories) {
            pCatMgr->UnregisterCategory(CLSID_VietFlowTip, cat, CLSID_VietFlowTip);
        }
        pCatMgr->Release();
    }

    // 2. Unregister Modern Profile
    ITfInputProcessorProfileMgr* pProfMgrV2 = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfileMgr, (void**)&pProfMgrV2)) && pProfMgrV2) {
        pProfMgrV2->UnregisterProfile(CLSID_VietFlowTip, c_langidEnglishUS, GUID_VietFlowProfile, 0);
        pProfMgrV2->UnregisterProfile(CLSID_VietFlowTip, c_langidVietnamese, GUID_VietFlowProfile, 0);
        pProfMgrV2->Release();
    }

    // 3. Unregister Legacy Profile
    ITfInputProcessorProfiles* pProfMgr = nullptr;
    if (SUCCEEDED(CoCreateInstance(CLSID_TF_InputProcessorProfiles, nullptr, CLSCTX_INPROC_SERVER, IID_ITfInputProcessorProfiles, (void**)&pProfMgr)) && pProfMgr) {
        pProfMgr->Unregister(CLSID_VietFlowTip);
        pProfMgr->Release();
    }

    // 4. Delete registry trees from HKLM, HKCU, and HKCR
    std::wstring tipCtfHklm = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\TIP\\") + c_szClsidStr;
    std::wstring tipCtfHkcu = std::wstring(L"Software\\Microsoft\\CTF\\TIP\\") + c_szClsidStr;
    std::wstring sortHkcu = std::wstring(L"Software\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\") + c_szClsidStr;
    std::wstring sortHklm = std::wstring(L"SOFTWARE\\Microsoft\\CTF\\SortOrder\\Language\\00000409\\") + c_szClsidStr;

    std::wstring clsidHklm = std::wstring(L"SOFTWARE\\Classes\\CLSID\\") + c_szClsidStr;
    std::wstring clsidHkcu = std::wstring(L"Software\\Classes\\CLSID\\") + c_szClsidStr;
    std::wstring clsidHkcr = std::wstring(L"CLSID\\") + c_szClsidStr;

    RegDeleteSubTree(HKEY_LOCAL_MACHINE, tipCtfHklm.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, tipCtfHkcu.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, sortHkcu.c_str());
    RegDeleteSubTree(HKEY_LOCAL_MACHINE, sortHklm.c_str());
    RegDeleteSubTree(HKEY_LOCAL_MACHINE, clsidHklm.c_str());
    RegDeleteSubTree(HKEY_CURRENT_USER, clsidHkcu.c_str());
    RegDeleteSubTree(HKEY_CLASSES_ROOT, clsidHkcr.c_str());

    return S_OK;
}
