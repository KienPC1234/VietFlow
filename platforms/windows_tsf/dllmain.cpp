// VietFlow Windows TSF - In-process COM DLL Entry Points
// Licensed under GNU General Public License v3.0 (GPLv3)

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <msctf.h>
#include "clsid.h"
#include "class_factory.h"
#include "register.h"
#include "debug_log.h"

HINSTANCE g_hInst = nullptr;
LONG g_cDllRef = 0;

BOOL WINAPI DllMain(HINSTANCE hInstance, DWORD dwReason, LPVOID lpReserved) {
    (void)lpReserved;
    switch (dwReason) {
        case DLL_PROCESS_ATTACH:
            g_hInst = hInstance;
            DisableThreadLibraryCalls(hInstance);
            VF_LOG(L"--- DLL_PROCESS_ATTACH ---");
            break;
        case DLL_PROCESS_DETACH:
            VF_LOG(L"--- DLL_PROCESS_DETACH ---");
            break;
    }
    return TRUE;
}

STDAPI DllGetClassObject(REFCLSID rclsid, REFIID riid, void** ppv) {
    if (!ppv) return E_POINTER;
    *ppv = nullptr;

    if (!IsEqualCLSID(rclsid, CLSID_VietFlowTip)) {
        VF_LOG(L"DllGetClassObject: requested unknown CLSID");
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    VF_LOG(L"DllGetClassObject: creating CVietFlowClassFactory");
    CVietFlowClassFactory* pFactory = new CVietFlowClassFactory();
    if (!pFactory) return E_OUTOFMEMORY;

    HRESULT hr = pFactory->QueryInterface(riid, ppv);
    pFactory->Release();
    VF_LOG(L"DllGetClassObject: QueryInterface result 0x%08X", hr);
    return hr;
}

STDAPI DllCanUnloadNow() {
    return (g_cDllRef == 0) ? S_OK : S_FALSE;
}

STDAPI DllRegisterServer() {
    VF_LOG(L"DllRegisterServer called");
    HRESULT hr = RegisterServer(g_hInst);
    VF_LOG(L"DllRegisterServer returned 0x%08X", hr);
    return hr;
}

STDAPI DllUnregisterServer() {
    VF_LOG(L"DllUnregisterServer called");
    HRESULT hr = UnregisterServer();
    VF_LOG(L"DllUnregisterServer returned 0x%08X", hr);
    return hr;
}

