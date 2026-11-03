// VietFlow Windows TSF - Main Text Input Processor (TIP)
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "tip.h"
#include "edit_session.h"
#include "debug_log.h"

CVietFlowTip::CVietFlowTip()
    : m_cRef(1)
    , m_clientId(TF_CLIENTID_NULL)
    , m_pThreadMgr(nullptr)
    , m_pComposition(nullptr)
    , m_pEngine(vf_engine_create())
    , m_currentConfig({})
    , m_configInitialized(false)
    , m_dwThreadMgrEventSinkCookie(TF_INVALID_COOKIE)
    , m_dwThreadFocusSinkCookie(TF_INVALID_COOKIE)
    , m_dwActiveLanguageProfileNotifySinkCookie(TF_INVALID_COOKIE)
{
    InterlockedIncrement(&g_cDllRef);
    VF_LOG(L"CVietFlowTip::CVietFlowTip created (engine=%p)", m_pEngine);
}

CVietFlowTip::~CVietFlowTip() {
    VF_LOG(L"CVietFlowTip::~CVietFlowTip destroyed");
    if (m_pEngine) {
        vf_engine_destroy(m_pEngine);
        m_pEngine = nullptr;
    }
    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }
    if (m_pThreadMgr) {
        m_pThreadMgr->Release();
        m_pThreadMgr = nullptr;
    }
    InterlockedDecrement(&g_cDllRef);
}

STDMETHODIMP CVietFlowTip::QueryInterface(REFIID riid, void** ppvObj) {
    if (!ppvObj) return E_POINTER;
    if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_ITfTextInputProcessor)) {
        *ppvObj = static_cast<ITfTextInputProcessor*>(this);
    } else if (IsEqualIID(riid, IID_ITfTextInputProcessorEx)) {
        *ppvObj = static_cast<ITfTextInputProcessorEx*>(this);
    } else if (IsEqualIID(riid, IID_ITfThreadMgrEventSink)) {
        *ppvObj = static_cast<ITfThreadMgrEventSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfThreadFocusSink)) {
        *ppvObj = static_cast<ITfThreadFocusSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfActiveLanguageProfileNotifySink)) {
        *ppvObj = static_cast<ITfActiveLanguageProfileNotifySink*>(this);
    } else if (IsEqualIID(riid, IID_ITfKeyEventSink)) {
        *ppvObj = static_cast<ITfKeyEventSink*>(this);
    } else if (IsEqualIID(riid, IID_ITfCompositionSink)) {
        *ppvObj = static_cast<ITfCompositionSink*>(this);
    } else {
        *ppvObj = nullptr;
        return E_NOINTERFACE;
    }
    AddRef();
    return S_OK;
}

STDMETHODIMP_(ULONG) CVietFlowTip::AddRef() {
    return InterlockedIncrement(&m_cRef);
}

STDMETHODIMP_(ULONG) CVietFlowTip::Release() {
    ULONG c = InterlockedDecrement(&m_cRef);
    if (c == 0) {
        delete this;
    }
    return c;
}

STDMETHODIMP CVietFlowTip::Activate(ITfThreadMgr* ptim, TfClientId tid) {
    VF_LOG(L"CVietFlowTip::Activate: ptim=%p, tid=%u", ptim, tid);
    m_clientId = tid;
    m_configReader.UpdateActiveFocus(true);

    if (ptim) {
        m_pThreadMgr = ptim;
        m_pThreadMgr->AddRef();

        ITfSource* pSource = nullptr;
        if (SUCCEEDED(ptim->QueryInterface(IID_ITfSource, (void**)&pSource)) && pSource) {
            HRESULT hrSink = pSource->AdviseSink(IID_ITfThreadMgrEventSink, static_cast<ITfThreadMgrEventSink*>(this), &m_dwThreadMgrEventSinkCookie);
            VF_LOG(L"AdviseSink(ITfThreadMgrEventSink) hr=0x%08X, cookie=%u", hrSink, m_dwThreadMgrEventSinkCookie);

            hrSink = pSource->AdviseSink(IID_ITfThreadFocusSink, static_cast<ITfThreadFocusSink*>(this), &m_dwThreadFocusSinkCookie);
            VF_LOG(L"AdviseSink(ITfThreadFocusSink) hr=0x%08X, cookie=%u", hrSink, m_dwThreadFocusSinkCookie);

            hrSink = pSource->AdviseSink(IID_ITfActiveLanguageProfileNotifySink, static_cast<ITfActiveLanguageProfileNotifySink*>(this), &m_dwActiveLanguageProfileNotifySinkCookie);
            VF_LOG(L"AdviseSink(ITfActiveLanguageProfileNotifySink) hr=0x%08X, cookie=%u", hrSink, m_dwActiveLanguageProfileNotifySinkCookie);

            pSource->Release();
        } else {
            VF_LOG(L"Failed to query ITfSource from ptim");
        }

        ITfKeystrokeMgr* pKeystrokeMgr = nullptr;
        if (SUCCEEDED(ptim->QueryInterface(IID_ITfKeystrokeMgr, (void**)&pKeystrokeMgr)) && pKeystrokeMgr) {
            HRESULT hr = pKeystrokeMgr->AdviseKeyEventSink(tid, static_cast<ITfKeyEventSink*>(this), TRUE);
            (void)hr;
            VF_LOG(L"AdviseKeyEventSink result: 0x%08X", hr);
            pKeystrokeMgr->Release();
        } else {
            VF_LOG(L"Failed to query ITfKeystrokeMgr from ptim");
        }
    }
    SyncConfig();
    return S_OK;
}

STDMETHODIMP CVietFlowTip::Deactivate() {
    VF_LOG(L"CVietFlowTip::Deactivate: tid=%u", m_clientId);
    m_configReader.UpdateActiveFocus(false);

    if (m_pThreadMgr) {
        ITfKeystrokeMgr* pKeystrokeMgr = nullptr;
        if (SUCCEEDED(m_pThreadMgr->QueryInterface(IID_ITfKeystrokeMgr, (void**)&pKeystrokeMgr)) && pKeystrokeMgr) {
            pKeystrokeMgr->UnadviseKeyEventSink(m_clientId);
            pKeystrokeMgr->Release();
        }

        ITfSource* pSource = nullptr;
        if (SUCCEEDED(m_pThreadMgr->QueryInterface(IID_ITfSource, (void**)&pSource)) && pSource) {
            if (m_dwThreadMgrEventSinkCookie != TF_INVALID_COOKIE) {
                pSource->UnadviseSink(m_dwThreadMgrEventSinkCookie);
                m_dwThreadMgrEventSinkCookie = TF_INVALID_COOKIE;
            }
            if (m_dwThreadFocusSinkCookie != TF_INVALID_COOKIE) {
                pSource->UnadviseSink(m_dwThreadFocusSinkCookie);
                m_dwThreadFocusSinkCookie = TF_INVALID_COOKIE;
            }
            if (m_dwActiveLanguageProfileNotifySinkCookie != TF_INVALID_COOKIE) {
                pSource->UnadviseSink(m_dwActiveLanguageProfileNotifySinkCookie);
                m_dwActiveLanguageProfileNotifySinkCookie = TF_INVALID_COOKIE;
            }
            pSource->Release();
        }

        m_pThreadMgr->Release();
        m_pThreadMgr = nullptr;
    }

    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }

    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }

    m_clientId = TF_CLIENTID_NULL;
    return S_OK;
}

STDMETHODIMP CVietFlowTip::ActivateEx(ITfThreadMgr* ptim, TfClientId tid, DWORD dwFlags) {
    (void)dwFlags;
    VF_LOG(L"CVietFlowTip::ActivateEx: dwFlags=0x%08X", dwFlags);
    return Activate(ptim, tid);
}

STDMETHODIMP CVietFlowTip::OnInitDocumentMgr(ITfDocumentMgr* pdim) {
    (void)pdim;
    VF_LOG(L"CVietFlowTip::OnInitDocumentMgr: pdim=%p", pdim);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnUninitDocumentMgr(ITfDocumentMgr* pdim) {
    (void)pdim;
    VF_LOG(L"CVietFlowTip::OnUninitDocumentMgr: pdim=%p", pdim);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnSetFocus(ITfDocumentMgr* pdimFocus, ITfDocumentMgr* pdimPrevFocus) {
    (void)pdimFocus;
    (void)pdimPrevFocus;
    VF_LOG(L"CVietFlowTip::OnSetFocus(ITfDocumentMgr): focus=%p, prev=%p", pdimFocus, pdimPrevFocus);
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnPushContext(ITfContext* pic) {
    (void)pic;
    VF_LOG(L"CVietFlowTip::OnPushContext: pic=%p", pic);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnPopContext(ITfContext* pic) {
    (void)pic;
    VF_LOG(L"CVietFlowTip::OnPopContext: pic=%p", pic);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnSetThreadFocus() {
    VF_LOG(L"CVietFlowTip::OnSetThreadFocus");
    m_configReader.UpdateActiveFocus(true);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnKillThreadFocus() {
    VF_LOG(L"CVietFlowTip::OnKillThreadFocus");
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }
    m_configReader.UpdateActiveFocus(false);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnActivated(REFCLSID clsid, REFGUID guidProfile, BOOL fActivated) {
    (void)clsid;
    (void)guidProfile;
    (void)fActivated;
    VF_LOG(L"CVietFlowTip::OnActivated: fActivated=%d", fActivated);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnSetFocus(BOOL fForeground) {
    VF_LOG(L"CVietFlowTip::OnSetFocus: fForeground=%d", fForeground);
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }
    m_configReader.UpdateActiveFocus(fForeground != FALSE);
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnCompositionTerminated(TfEditCookie ecWrite, ITfComposition* pComposition) {
    (void)ecWrite;
    (void)pComposition;
    VF_LOG(L"CVietFlowTip::OnCompositionTerminated");
    if (m_pComposition) {
        m_pComposition->Release();
        m_pComposition = nullptr;
    }
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
    m_configReader.UpdateActiveKeystroke(false);
    return S_OK;
}

void CVietFlowTip::RequestEditSession(ITfContext* pContext, ITfEditSession* pSession) {
    if (!pContext || !pSession) return;
    HRESULT hrSession = S_OK;
    HRESULT hr = pContext->RequestEditSession(m_clientId, pSession, TF_ES_READWRITE | TF_ES_SYNC, &hrSession);
    if (hr == TF_E_SYNCHRONOUS || FAILED(hr)) {
        HRESULT hrAsync = pContext->RequestEditSession(m_clientId, pSession, TF_ES_READWRITE | TF_ES_ASYNCDONTCARE, &hrSession);
        (void)hrAsync;
        VF_LOG(L"RequestEditSession async fallback: 0x%08X (session hr: 0x%08X)", hrAsync, hrSession);
    }
}

void CVietFlowTip::SyncConfig() {
    TsfSharedConfig cfg = m_configReader.Get();
    if (m_pEngine && cfg.version > 0) {
        if (!m_configInitialized || cfg.method != m_currentConfig.method) {
            vf_engine_set_method(m_pEngine, static_cast<uint8_t>(cfg.method));
        }
        if (!m_configInitialized || cfg.charset != m_currentConfig.charset) {
            vf_engine_set_charset(m_pEngine, static_cast<uint8_t>(cfg.charset));
        }
        if (!m_configInitialized || cfg.modern_style != m_currentConfig.modern_style) {
            vf_engine_set_modern_style(m_pEngine, cfg.modern_style != 0);
        }
        if (!m_configInitialized || cfg.spell_check != m_currentConfig.spell_check) {
            vf_engine_set_spell_check(m_pEngine, cfg.spell_check != 0);
        }
        if (!m_configInitialized || cfg.free_marking != m_currentConfig.free_marking) {
            vf_engine_set_free_marking(m_pEngine, cfg.free_marking != 0);
        }
        if (!m_configInitialized || cfg.auto_restore != m_currentConfig.auto_restore) {
            vf_engine_set_auto_restore(m_pEngine, cfg.auto_restore != 0);
        }
        if (!m_configInitialized || cfg.auto_switch_by_typing != m_currentConfig.auto_switch_by_typing) {
            vf_engine_set_auto_switch_by_typing(m_pEngine, cfg.auto_switch_by_typing != 0);
        }
        if (!m_configInitialized || cfg.enable_macro != m_currentConfig.enable_macro) {
            vf_engine_set_enable_macro(m_pEngine, cfg.enable_macro != 0);
        }
        m_currentConfig = cfg;
        m_configInitialized = true;
    }
}

void CVietFlowTip::ClearAndEndComposition(ITfContext* pContext) {
    VF_LOG(L"ClearAndEndComposition");
    CEditSession* pSession = new CEditSession(pContext, this, &m_pComposition, L"", EditAction::ClearAndEndComposition);
    RequestEditSession(pContext, pSession);
    pSession->Release();
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
}

void CVietFlowTip::EndComposition(ITfContext* pContext) {
    VF_LOG(L"EndComposition");
    CEditSession* pSession = new CEditSession(pContext, this, &m_pComposition, L"", EditAction::EndComposition);
    RequestEditSession(pContext, pSession);
    pSession->Release();
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
}

void CVietFlowTip::ReplaceAndEndComposition(ITfContext* pContext, const std::wstring& text) {
    VF_LOG(L"ReplaceAndEndComposition text='%s'", text.c_str());
    CEditSession* pSession = new CEditSession(pContext, this, &m_pComposition, text, EditAction::ReplaceAndEndComposition);
    RequestEditSession(pContext, pSession);
    pSession->Release();
    if (m_pEngine) {
        vf_engine_reset(m_pEngine);
    }
}

void CVietFlowTip::FinishCompositionWithBreak(ITfContext* pContext, wchar_t breakChar) {
    uint8_t outBuf[512] = {};
    size_t outLen = 0;
    vf_engine_finish_on_break(m_pEngine, breakChar ? static_cast<uint32_t>(breakChar) : ' ', outBuf, sizeof(outBuf) - 1, &outLen);
    std::string utf8Word(reinterpret_cast<char*>(outBuf), outLen);
    std::wstring text = Utf8ToWide(utf8Word);

    if (breakChar != 0) {
        text.push_back(breakChar);
    }

    VF_LOG(L"FinishCompositionWithBreak break='%c', finalized text='%s'", breakChar ? breakChar : L' ', text.c_str());
    if (!text.empty()) {
        ReplaceAndEndComposition(pContext, text);
    } else {
        EndComposition(pContext);
    }
}

bool CVietFlowTip::IsModifierPressed() {
    bool ctrl = (GetKeyState(VK_CONTROL) & 0x8000) != 0;
    bool alt = (GetKeyState(VK_MENU) & 0x8000) != 0;
    bool win = (GetKeyState(VK_LWIN) & 0x8000) != 0 || (GetKeyState(VK_RWIN) & 0x8000) != 0;
    return ctrl || alt || win;
}

bool CVietFlowTip::IsCompositionKey(UINT vk, bool shift, uint32_t method) {
    if (vk >= 'A' && vk <= 'Z') return true;
    if (vk >= '0' && vk <= '9' && !shift && (method == 1 || method == 5 || method == 6)) {
        return true;
    }
    if ((vk == 0xDB || vk == 0xDD) && !shift && (method == 0 || method == 6)) {
        return true; // [ and ] for Telex
    }
    if (method == 2 || method == 6) { // VIQR
        if (vk == 0xDE || vk == 0xC0 || vk == 0xBF || vk == 0xBE || vk == 0xBB) return true;
        if ((vk == '6' && shift) || (vk == '9' && shift)) return true;
    }
    return false;
}

wchar_t CVietFlowTip::VkToChar(UINT vk, bool shift, bool caps) {
    if (vk >= 'A' && vk <= 'Z') {
        bool upper = shift ^ caps;
        return upper ? static_cast<wchar_t>(L'A' + (vk - 'A')) : static_cast<wchar_t>(L'a' + (vk - 'A'));
    }
    if (vk >= '0' && vk <= '9') {
        if (!shift) {
            return static_cast<wchar_t>(L'0' + (vk - '0'));
        }
        switch (vk) {
            case '0': return L')';
            case '1': return L'!';
            case '2': return L'@';
            case '3': return L'#';
            case '4': return L'$';
            case '5': return L'%';
            case '6': return L'^';
            case '7': return L'&';
            case '8': return L'*';
            case '9': return L'(';
            default: return 0;
        }
    }
    switch (vk) {
        case VK_OEM_1: return shift ? L':' : L';';
        case VK_OEM_PLUS: return shift ? L'+' : L'=';
        case VK_OEM_COMMA: return shift ? L'<' : L',';
        case VK_OEM_MINUS: return shift ? L'_' : L'-';
        case VK_OEM_PERIOD: return shift ? L'>' : L'.';
        case VK_OEM_2: return shift ? L'?' : L'/';
        case VK_OEM_3: return shift ? L'~' : L'`';
        case VK_OEM_4: return shift ? L'{' : L'[';
        case VK_OEM_5: return shift ? L'|' : L'\\';
        case VK_OEM_6: return shift ? L'}' : L']';
        case VK_OEM_7: return shift ? L'"' : L'\'';
        default: return 0;
    }
}

std::wstring CVietFlowTip::Utf8ToWide(const std::string& str) {
    if (str.empty()) return std::wstring();
    int needed = MultiByteToWideChar(CP_UTF8, 0, str.c_str(), static_cast<int>(str.length()), nullptr, 0);
    if (needed <= 0) return std::wstring();
    std::wstring result(needed, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, str.c_str(), static_cast<int>(str.length()), &result[0], needed);
    return result;
}

STDMETHODIMP CVietFlowTip::OnTestKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) {
    (void)pic;
    (void)lParam;
    if (!pfEaten) return E_INVALIDARG;
    *pfEaten = FALSE;

    TsfSharedConfig cfg = m_configReader.Get();
    if (cfg.enabled == 0) return S_OK;

    if (IsModifierPressed()) return S_OK;

    UINT vk = static_cast<UINT>(wParam);
    bool shift = (GetKeyState(VK_SHIFT) & 0x8000) != 0;
    bool caps = (GetKeyState(VK_CAPITAL) & 1) != 0;

    // Navigation keys pass through natively
    if ((vk >= VK_PRIOR && vk <= VK_DOWN) || vk == VK_DELETE) {
        return S_OK;
    }

    bool isComp = IsCompositionKey(vk, shift, cfg.method);

    if (m_pComposition != nullptr) {
        if (vk == VK_TAB) {
            return S_OK; // Commit composition and pass through for focus navigation
        }
        if (vk == VK_BACK || vk == VK_ESCAPE || vk == VK_SPACE ||
            vk == VK_RETURN || isComp || VkToChar(vk, shift, caps) != 0) {
            *pfEaten = TRUE;
            VF_LOG(L"OnTestKeyDown: vk=0x%02X (%c), inside comp -> pfEaten=TRUE", vk, (vk >= 32 && vk < 127) ? vk : '?');
            return S_OK;
        }
        return S_OK;
    }

    if (isComp) {
        *pfEaten = TRUE;
        VF_LOG(L"OnTestKeyDown: vk=0x%02X (%c), start comp -> pfEaten=TRUE", vk, (vk >= 32 && vk < 127) ? vk : '?');
    }
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnKeyDown(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) {
    (void)lParam;
    if (!pfEaten) return E_INVALIDARG;
    *pfEaten = FALSE;

    TsfSharedConfig cfg = m_configReader.Get();
    if (cfg.enabled == 0) {
        if (m_pComposition && pic) {
            EndComposition(pic);
        }
        return S_OK;
    }

    if (!pic) return S_OK;

    if (IsModifierPressed()) {
        if (m_pComposition) {
            EndComposition(pic);
        }
        return S_OK;
    }

    SyncConfig();

    UINT vk = static_cast<UINT>(wParam);
    bool shift = (GetKeyState(VK_SHIFT) & 0x8000) != 0;
    bool caps = (GetKeyState(VK_CAPITAL) & 1) != 0;

    // Navigation keys (Arrows, Home, End, PgUp, PgDn, Delete) commit active composition cleanly and pass through
    if ((vk >= VK_PRIOR && vk <= VK_DOWN) || vk == VK_DELETE) {
        if (m_pComposition) {
            VF_LOG(L"OnKeyDown NAVIGATION key (0x%02X) while composing, committing", vk);
            FinishCompositionWithBreak(pic, 0);
            m_configReader.UpdateActiveKeystroke(false);
        }
        return S_OK;
    }

    // 1. Backspace in active composition
    if (vk == VK_BACK) {
        if (m_pComposition) {
            uint8_t buf[256] = {};
            size_t outLen = 0;
            vf_engine_process_backspace(m_pEngine, buf, sizeof(buf) - 1, &outLen);
            std::string utf8Str(reinterpret_cast<char*>(buf), outLen);
            std::wstring word = Utf8ToWide(utf8Str);
            VF_LOG(L"OnKeyDown BACKSPACE: remaining preedit='%s'", word.c_str());

            if (word.empty()) {
                ClearAndEndComposition(pic);
                m_configReader.UpdateActiveKeystroke(false);
            } else {
                CEditSession* pSession = new CEditSession(pic, this, &m_pComposition, word, EditAction::StartOrUpdate);
                RequestEditSession(pic, pSession);
                pSession->Release();
                m_configReader.UpdateActiveKeystroke(true);
            }
            *pfEaten = TRUE;
            return S_OK;
        }
        return S_OK;
    }

    // 2. Escape cancels composition
    if (vk == VK_ESCAPE) {
        if (m_pComposition) {
            VF_LOG(L"OnKeyDown ESCAPE");
            EndComposition(pic);
            m_configReader.UpdateActiveKeystroke(false);
            *pfEaten = TRUE;
            return S_OK;
        }
        return S_OK;
    }

    // 3. Space commits with space and eats key so host control cannot wipe selection
    if (vk == VK_SPACE) {
        if (m_pComposition) {
            VF_LOG(L"OnKeyDown SPACE");
            FinishCompositionWithBreak(pic, L' ');
            m_configReader.UpdateActiveKeystroke(false);
            *pfEaten = TRUE;
            return S_OK;
        }
        return S_OK;
    }

    // 4. Return commits cleanly without break char
    if (vk == VK_RETURN) {
        if (m_pComposition) {
            VF_LOG(L"OnKeyDown RETURN");
            FinishCompositionWithBreak(pic, 0);
            m_configReader.UpdateActiveKeystroke(false);
            *pfEaten = TRUE;
            return S_OK;
        }
        return S_OK;
    }

    // 5. Tab commits and passes through (*pfEaten = FALSE) for control navigation
    if (vk == VK_TAB) {
        if (m_pComposition) {
            VF_LOG(L"OnKeyDown TAB");
            FinishCompositionWithBreak(pic, 0);
            m_configReader.UpdateActiveKeystroke(false);
            *pfEaten = FALSE;
            return S_OK;
        }
        return S_OK;
    }

    bool isComp = IsCompositionKey(vk, shift, cfg.method);

    // 6. Composition key (letter, VNI number, bracket, VIQR mark)
    if (isComp) {
        wchar_t ch = VkToChar(vk, shift, caps);
        if (ch == 0) return S_OK;

        size_t outBackspaces = 0;
        size_t outLen = 0;
        uint8_t outUtf8[256] = {};
        vf_engine_process_key(m_pEngine, static_cast<uint32_t>(ch), outUtf8, sizeof(outUtf8) - 1, &outLen, &outBackspaces);

        uint8_t preeditBuf[256] = {};
        size_t preeditLen = 0;
        vf_engine_get_preedit(m_pEngine, preeditBuf, sizeof(preeditBuf) - 1, &preeditLen);
        std::string utf8Word(reinterpret_cast<char*>(preeditBuf), preeditLen);
        std::wstring word = Utf8ToWide(utf8Word);

        VF_LOG(L"OnKeyDown COMP: ch='%c', preedit='%s'", ch, word.c_str());

        if (word.empty()) {
            ClearAndEndComposition(pic);
            m_configReader.UpdateActiveKeystroke(false);
        } else {
            CEditSession* pSession = new CEditSession(pic, this, &m_pComposition, word, EditAction::StartOrUpdate);
            RequestEditSession(pic, pSession);
            pSession->Release();
            m_configReader.UpdateActiveKeystroke(true);
        }
        *pfEaten = TRUE;
        return S_OK;
    }

    // 7. Non-composition printable character while composing (punctuation, etc.)
    if (m_pComposition) {
        wchar_t ch = VkToChar(vk, shift, caps);
        if (ch != 0) {
            VF_LOG(L"OnKeyDown PUNCTUATION while composing: ch='%c'", ch);
            FinishCompositionWithBreak(pic, ch);
            m_configReader.UpdateActiveKeystroke(false);
            *pfEaten = TRUE;
            return S_OK;
        }
    }

    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnTestKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) {
    (void)pic;
    (void)wParam;
    (void)lParam;
    if (pfEaten) *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnKeyUp(ITfContext* pic, WPARAM wParam, LPARAM lParam, BOOL* pfEaten) {
    (void)pic;
    (void)wParam;
    (void)lParam;
    if (pfEaten) *pfEaten = FALSE;
    return S_OK;
}

STDMETHODIMP CVietFlowTip::OnPreservedKey(ITfContext* pic, REFGUID rguid, BOOL* pfEaten) {
    (void)pic;
    (void)rguid;
    if (pfEaten) *pfEaten = FALSE;
    return S_OK;
}
