// VietFlow Windows TSF - Main Text Input Processor (TIP)
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <msctf.h>
#include <string>
#include "ipc.h"
#include "debug_log.h"
#include "../../ffi/vietflow_core.h"

extern LONG g_cDllRef;

class CVietFlowTip : public ITfTextInputProcessorEx,
                     public ITfThreadMgrEventSink,
                     public ITfThreadFocusSink,
                     public ITfActiveLanguageProfileNotifySink,
                     public ITfKeyEventSink,
                     public ITfCompositionSink
{
public:
    CVietFlowTip();
    virtual ~CVietFlowTip();

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppvObj) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // ITfTextInputProcessor
    STDMETHODIMP Activate(ITfThreadMgr* ptim, TfClientId tid) override;
    STDMETHODIMP Deactivate() override;

    // ITfTextInputProcessorEx
    STDMETHODIMP ActivateEx(ITfThreadMgr* ptim, TfClientId tid, DWORD dwFlags) override;

    // ITfThreadMgrEventSink
    STDMETHODIMP OnInitDocumentMgr(ITfDocumentMgr* pdim) override;
    STDMETHODIMP OnUninitDocumentMgr(ITfDocumentMgr* pdim) override;
    STDMETHODIMP OnSetFocus(ITfDocumentMgr* pdimFocus, ITfDocumentMgr* pdimPrevFocus) override;
    STDMETHODIMP OnPushContext(ITfContext* pic) override;
    STDMETHODIMP OnPopContext(ITfContext* pic) override;

    // ITfThreadFocusSink
    STDMETHODIMP OnSetThreadFocus() override;
    STDMETHODIMP OnKillThreadFocus() override;

    // ITfActiveLanguageProfileNotifySink
    STDMETHODIMP OnActivated(REFCLSID clsid, REFGUID guidProfile, BOOL fActivated) override;

    // ITfKeyEventSink
    STDMETHODIMP OnSetFocus(BOOL fForeground) override;
    STDMETHODIMP OnTestKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnTestKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) override;
    STDMETHODIMP OnPreservedKey(ITfContext* pic, REFGUID rguid, BOOL* pfEaten) override;

    // ITfCompositionSink
    STDMETHODIMP OnCompositionTerminated(TfEditCookie ecWrite, ITfComposition* pComposition) override;

private:
    void RequestEditSession(ITfContext* pContext, ITfEditSession* pSession);
    void SyncConfig();
    void FinishCompositionWithBreak(ITfContext* pContext, wchar_t breakChar);
    void ClearAndEndComposition(ITfContext* pContext);
    void EndComposition(ITfContext* pContext);
    void ReplaceAndEndComposition(ITfContext* pContext, const std::wstring& text);

    static bool IsModifierPressed();
    static bool IsCompositionKey(UINT vk, bool shift, uint32_t method);
    static wchar_t VkToChar(UINT vk, bool shift, bool caps);
    static std::wstring Utf8ToWide(const std::string& str);

    LONG m_cRef;
    TfClientId m_clientId;
    ITfThreadMgr* m_pThreadMgr;
    ITfComposition* m_pComposition;
    VfEngine* m_pEngine;
    SharedConfigReader m_configReader;
    TsfSharedConfig m_currentConfig;
    bool m_configInitialized;

    DWORD m_dwThreadMgrEventSinkCookie;
    DWORD m_dwThreadFocusSinkCookie;
    DWORD m_dwActiveLanguageProfileNotifySinkCookie;
};
