// VietFlow Windows TSF - Edit Session for Direct Document Text Insertion
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <msctf.h>
#include <string>

enum class EditAction {
    StartOrUpdate,
    ReplaceAndEndComposition,
    ClearAndEndComposition,
    EndComposition
};

class CEditSession : public ITfEditSession {
public:
    CEditSession(ITfContext* pContext,
                 ITfCompositionSink* pSink,
                 ITfComposition** ppComposition,
                 const std::wstring& text,
                 EditAction action);

    virtual ~CEditSession();

    // IUnknown
    STDMETHODIMP QueryInterface(REFIID riid, void** ppvObj) override;
    STDMETHODIMP_(ULONG) AddRef() override;
    STDMETHODIMP_(ULONG) Release() override;

    // ITfEditSession
    STDMETHODIMP DoEditSession(TfEditCookie ec) override;

private:
    void SetCaretToEnd(TfEditCookie ec, ITfRange* pRange);

    LONG m_cRef;
    ITfContext* m_pContext;
    ITfCompositionSink* m_pSink;
    ITfComposition** m_ppComposition;
    std::wstring m_text;
    EditAction m_action;
};

