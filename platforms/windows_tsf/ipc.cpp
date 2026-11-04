// VietFlow TSF: separate immutable configuration and foreground focus mappings.
// Licensed under GPLv3.
#include "ipc.h"
#include <cstring>

static constexpr wchar_t CONFIG_MAP_NAME[] = L"Local\\VietFlow_TSF_Config_v2";
static constexpr wchar_t FOCUS_MAP_NAME[] = L"Local\\VietFlow_TSF_Focus_v2";

static TsfSharedConfig DisabledFallback() {
    TsfSharedConfig c = {};
    c.enabled = 0;
    c.method = 0;
    c.charset = 0;
    c.modern_style = 1;
    c.spell_check = 1;
    c.free_marking = 1;
    c.auto_restore = 1;
    c.auto_switch_by_typing = 1;
    c.enable_macro = 1;
    c.version = 2;
    return c;
}

// Do not trust a mapping merely because its version and sequence match.
// Bad enum values otherwise wrap in the C ABI's uint8_t and malformed
// booleans get silently interpreted as enabled in the text processor.
static bool IsValidTsfConfig(const TsfSharedConfig& c) {
    auto validFlag = [](uint32_t value) { return value <= 1; };
    return c.version == 2
        && c.method <= 6     // core InputMethod (Telex .. UserCustom)
        && c.charset <= 4    // core CharsetId (Unicode .. VIQR)
        && c.reserved == 0
        && validFlag(c.enabled)
        && validFlag(c.modern_style)
        && validFlag(c.spell_check)
        && validFlag(c.free_marking)
        && validFlag(c.auto_restore)
        && validFlag(c.auto_switch_by_typing)
        && validFlag(c.enable_macro);
}

SharedConfigReader::SharedConfigReader(const wchar_t* configName, const wchar_t* focusName)
    : m_cached(DisabledFallback()),
      m_configName(configName ? configName : CONFIG_MAP_NAME),
      m_focusName(focusName ? focusName : FOCUS_MAP_NAME) {
    EnsureMaps();
}

void SharedConfigReader::EnsureMaps() {
    if (!m_pView) {
        if (!m_hMapping)
            m_hMapping = OpenFileMappingW(FILE_MAP_READ, FALSE, m_configName);
        if (m_hMapping) {
            m_pView = static_cast<const TsfSharedConfig*>(
                MapViewOfFile(m_hMapping, FILE_MAP_READ, 0, 0, sizeof(TsfSharedConfig)));
            if (!m_pView) { CloseHandle(m_hMapping); m_hMapping = nullptr; }
        }
    }
    if (!m_pFocus) {
        if (!m_hFocus)
            m_hFocus = OpenFileMappingW(FILE_MAP_READ | FILE_MAP_WRITE, FALSE, m_focusName);
        if (m_hFocus) {
            m_pFocus = static_cast<TsfFocusState*>(
                MapViewOfFile(m_hFocus, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(TsfFocusState)));
            if (!m_pFocus) { CloseHandle(m_hFocus); m_hFocus = nullptr; }
            if (m_pFocus && m_focusRequested) {
                InterlockedExchange(reinterpret_cast<volatile LONG*>(&m_pFocus->last_active_pid),
                                    static_cast<LONG>(GetCurrentProcessId()));
                InterlockedExchange64(reinterpret_cast<volatile LONG64*>(&m_pFocus->last_active_tick),
                                      static_cast<LONG64>(GetTickCount64()));
            }
        }
    }
}

SharedConfigReader::~SharedConfigReader() {
    if (m_pView) { UnmapViewOfFile(m_pView); m_pView = nullptr; }
    if (m_pFocus) { UnmapViewOfFile(m_pFocus); m_pFocus = nullptr; }
    if (m_hMapping) { CloseHandle(m_hMapping); m_hMapping = nullptr; }
    if (m_hFocus) { CloseHandle(m_hFocus); m_hFocus = nullptr; }
}

TsfSharedConfig SharedConfigReader::Get() {
    EnsureMaps();
    // Without both a stable config and focus channel, the hook must handle text.
    if (!m_pView || !m_pFocus || !m_focusRequested)
        return DisabledFallback();

    const volatile LONG* seq =
        reinterpret_cast<const volatile LONG*>(&m_pView->sequence);
    for (int attempt = 0; attempt < 8; ++attempt) {
        LONG first = *seq;
        if (first & 1) continue;
        MemoryBarrier();
        TsfSharedConfig snapshot;
        std::memcpy(&snapshot, m_pView, sizeof(snapshot));
        MemoryBarrier();
        LONG last = *seq;
        if (first == last && !(last & 1) && IsValidTsfConfig(snapshot)) {
            m_cached = snapshot;
            return snapshot;
        }
    }
    return m_cached;
}

void SharedConfigReader::UpdateActiveFocus(bool focused) {
    m_focusRequested = focused;
    EnsureMaps();
    if (!m_pFocus) return;
    const LONG pid = static_cast<LONG>(GetCurrentProcessId());
    volatile LONG* active =
        reinterpret_cast<volatile LONG*>(&m_pFocus->last_active_pid);
    if (focused) {
        InterlockedExchange(active, pid);
        InterlockedExchange64(
            reinterpret_cast<volatile LONG64*>(&m_pFocus->last_active_tick),
            static_cast<LONG64>(GetTickCount64()));
    } else if (InterlockedCompareExchange(active, 0, pid) == pid) {
        InterlockedExchange(
            reinterpret_cast<volatile LONG*>(&m_pFocus->is_composing), 0);
    }
}

void SharedConfigReader::UpdateActiveKeystroke(bool isComposing) {
    EnsureMaps();
    if (!m_pFocus || !m_focusRequested) return;
    const LONG pid = static_cast<LONG>(GetCurrentProcessId());
    volatile LONG* active =
        reinterpret_cast<volatile LONG*>(&m_pFocus->last_active_pid);
    if (InterlockedCompareExchange(active, pid, pid) != pid) return;
    InterlockedExchange(
        reinterpret_cast<volatile LONG*>(&m_pFocus->is_composing),
        isComposing ? 1 : 0);
    InterlockedExchange64(
        reinterpret_cast<volatile LONG64*>(&m_pFocus->last_active_tick),
        static_cast<LONG64>(GetTickCount64()));
}
