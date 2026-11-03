// VietFlow Windows TSF - Class Factory
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "class_factory.h"
#include "tip.h"
#include "debug_log.h"

CVietFlowClassFactory::CVietFlowClassFactory()
    : m_cRef(1)
{
    InterlockedIncrement(&g_cDllRef);
}

CVietFlowClassFactory::~CVietFlowClassFactory() {
    InterlockedDecrement(&g_cDllRef);
}

STDMETHODIMP CVietFlowClassFactory::QueryInterface(REFIID riid, void** ppvObj) {
    if (!ppvObj) return E_POINTER;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IClassFactory)) {
        *ppvObj = static_cast<IClassFactory*>(this);
        AddRef();
        return S_OK;
    }
    *ppvObj = nullptr;
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) CVietFlowClassFactory::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

STDMETHODIMP_(ULONG) CVietFlowClassFactory::Release() {
    ULONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

STDMETHODIMP CVietFlowClassFactory::CreateInstance(IUnknown* pUnkOuter, REFIID riid, void** ppvObj) {
    if (!ppvObj) return E_POINTER;
    *ppvObj = nullptr;

    if (pUnkOuter != nullptr) {
        VF_LOG(L"CreateInstance: aggregation not supported");
        return CLASS_E_NOAGGREGATION;
    }

    VF_LOG(L"CreateInstance: creating CVietFlowTip");
    CVietFlowTip* pTip = new CVietFlowTip();
    if (!pTip) return E_OUTOFMEMORY;

    HRESULT hr = pTip->QueryInterface(riid, ppvObj);
    pTip->Release();
    VF_LOG(L"CreateInstance: QueryInterface result 0x%08X", hr);
    return hr;
}

STDMETHODIMP CVietFlowClassFactory::LockServer(BOOL fLock) {
    if (fLock) {
        InterlockedIncrement(&g_cDllRef);
    } else {
        InterlockedDecrement(&g_cDllRef);
    }
    return S_OK;
}

