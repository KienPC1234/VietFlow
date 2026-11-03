// VietFlow Windows TSF - Edit Session for Direct Document Text Insertion
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "edit_session.h"
#include "debug_log.h"

CEditSession::CEditSession(ITfContext* pContext,
                           ITfCompositionSink* pSink,
                           ITfComposition** ppComposition,
                           const std::wstring& text,
                           EditAction action)
    : m_cRef(1)
    , m_pContext(pContext)
    , m_pSink(pSink)
    , m_ppComposition(ppComposition)
    , m_text(text)
    , m_action(action)
{
    if (m_pContext) m_pContext->AddRef();
    if (m_pSink) m_pSink->AddRef();
}

CEditSession::~CEditSession() {
    if (m_pContext) {
        m_pContext->Release();
        m_pContext = nullptr;
    }
    if (m_pSink) {
        m_pSink->Release();
        m_pSink = nullptr;
    }
}

STDMETHODIMP CEditSession::QueryInterface(REFIID riid, void** ppvObj) {
    if (!ppvObj) return E_POINTER;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfEditSession)) {
        *ppvObj = static_cast<ITfEditSession*>(this);
        AddRef();
        return S_OK;
    }
    *ppvObj = nullptr;
    return E_NOINTERFACE;
}

STDMETHODIMP_(ULONG) CEditSession::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

STDMETHODIMP_(ULONG) CEditSession::Release() {
    ULONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

void CEditSession::SetCaretToEnd(TfEditCookie ec, ITfRange* pRange) {
    if (!m_pContext || !pRange) return;
    ITfRange* pSelRange = nullptr;
    if (SUCCEEDED(pRange->Clone(&pSelRange)) && pSelRange) {
        pSelRange->Collapse(ec, TF_ANCHOR_END);
        TF_SELECTION sel = {};
        sel.range = pSelRange;
        sel.style.ase = TF_AE_END;
        sel.style.fInterimChar = FALSE;
        m_pContext->SetSelection(ec, 1, &sel);
        pSelRange->Release();
    }
}

STDMETHODIMP CEditSession::DoEditSession(TfEditCookie ec) {
    VF_LOG(L"DoEditSession: action=%d, text='%s'", static_cast<int>(m_action), m_text.c_str());

    if (m_action == EditAction::EndComposition) {
        if (m_ppComposition && *m_ppComposition) {
            ITfRange* pRange = nullptr;
            if (SUCCEEDED((*m_ppComposition)->GetRange(&pRange)) && pRange) {
                (*m_ppComposition)->EndComposition(ec);
                SetCaretToEnd(ec, pRange);
                pRange->Release();
            } else {
                (*m_ppComposition)->EndComposition(ec);
            }
            (*m_ppComposition)->Release();
            *m_ppComposition = nullptr;
        }
        return S_OK;
    }

    if (m_action == EditAction::ClearAndEndComposition) {
        if (m_ppComposition && *m_ppComposition) {
            ITfRange* pRange = nullptr;
            if (SUCCEEDED((*m_ppComposition)->GetRange(&pRange)) && pRange) {
                pRange->SetText(ec, 0, nullptr, 0);
                (*m_ppComposition)->EndComposition(ec);
                SetCaretToEnd(ec, pRange);
                pRange->Release();
            } else {
                (*m_ppComposition)->EndComposition(ec);
            }
            (*m_ppComposition)->Release();
            *m_ppComposition = nullptr;
        }
        return S_OK;
    }

    if (m_action == EditAction::ReplaceAndEndComposition) {
        if (m_ppComposition && *m_ppComposition) {
            ITfRange* pRange = nullptr;
            if (SUCCEEDED((*m_ppComposition)->GetRange(&pRange)) && pRange) {
                if (!m_text.empty()) {
                    pRange->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                }
                (*m_ppComposition)->EndComposition(ec);
                SetCaretToEnd(ec, pRange);
                pRange->Release();
            } else {
                (*m_ppComposition)->EndComposition(ec);
            }
            (*m_ppComposition)->Release();
            *m_ppComposition = nullptr;
            return S_OK;
        }

        // Fallback if no active composition: insert directly at current selection
        if (!m_text.empty() && m_pContext) {
            TF_SELECTION sel = {};
            ULONG fetched = 0;
            if (SUCCEEDED(m_pContext->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &sel, &fetched)) && fetched > 0) {
                if (sel.range) {
                    sel.range->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                    SetCaretToEnd(ec, sel.range);
                    sel.range->Release();
                }
            }
        }
        return S_OK;
    }

    // StartOrUpdate
    if (m_ppComposition) {
        if (*m_ppComposition == nullptr) {
            // Start composition on current caret selection
            TF_SELECTION sel = {};
            ULONG fetched = 0;
            if (SUCCEEDED(m_pContext->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &sel, &fetched)) && fetched > 0) {
                if (sel.range) {
                    ITfContextComposition* pCompCtx = nullptr;
                    if (SUCCEEDED(m_pContext->QueryInterface(IID_ITfContextComposition, (void**)&pCompCtx)) && pCompCtx) {
                        ITfComposition* pComp = nullptr;
                        HRESULT hrComp = pCompCtx->StartComposition(ec, sel.range, m_pSink, &pComp);
                        VF_LOG(L"StartComposition hr=0x%08X, pComp=%p", hrComp, pComp);
                        if (SUCCEEDED(hrComp) && pComp) {
                            ITfRange* pCompRange = nullptr;
                            if (SUCCEEDED(pComp->GetRange(&pCompRange)) && pCompRange) {
                                pCompRange->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                                SetCaretToEnd(ec, pCompRange);
                                pCompRange->Release();
                            } else {
                                sel.range->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                                SetCaretToEnd(ec, sel.range);
                            }
                            *m_ppComposition = pComp; // Take ownership
                        } else {
                            // Direct insertion fallback if composition rejected
                            sel.range->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                            SetCaretToEnd(ec, sel.range);
                        }
                        pCompCtx->Release();
                    }
                    sel.range->Release();
                }
            } else {
                VF_LOG(L"GetSelection returned fetched=%u", fetched);
            }
        } else {
            // Update existing composition
            ITfRange* pRange = nullptr;
            if (SUCCEEDED((*m_ppComposition)->GetRange(&pRange)) && pRange) {
                pRange->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                SetCaretToEnd(ec, pRange);
                pRange->Release();
            } else {
                VF_LOG(L"Stale composition handle in update");
                (*m_ppComposition)->Release();
                *m_ppComposition = nullptr;

                TF_SELECTION sel = {};
                ULONG fetched = 0;
                if (SUCCEEDED(m_pContext->GetSelection(ec, TF_DEFAULT_SELECTION, 1, &sel, &fetched)) && fetched > 0) {
                    if (sel.range) {
                        ITfContextComposition* pCompCtx = nullptr;
                        if (SUCCEEDED(m_pContext->QueryInterface(IID_ITfContextComposition, (void**)&pCompCtx)) && pCompCtx) {
                            ITfComposition* pComp = nullptr;
                            if (SUCCEEDED(pCompCtx->StartComposition(ec, sel.range, m_pSink, &pComp)) && pComp) {
                                ITfRange* pCompRange = nullptr;
                                if (SUCCEEDED(pComp->GetRange(&pCompRange)) && pCompRange) {
                                    pCompRange->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                                    SetCaretToEnd(ec, pCompRange);
                                    pCompRange->Release();
                                } else {
                                    sel.range->SetText(ec, 0, m_text.c_str(), static_cast<LONG>(m_text.length()));
                                    SetCaretToEnd(ec, sel.range);
                                }
                                *m_ppComposition = pComp;
                            }
                            pCompCtx->Release();
                        }
                        sel.range->Release();
                    }
                }
            }
        }
    }

    return S_OK;
}

