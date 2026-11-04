// VietFlow Windows TSF - Cross-Process Shared Configuration
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <cstdint>

#pragma pack(push, 1)
struct TsfSharedConfig {
    uint32_t enabled;
    uint32_t method;
    uint32_t charset;
    uint32_t modern_style;
    uint32_t spell_check;
    uint32_t free_marking;
    uint32_t auto_restore;
    uint32_t auto_switch_by_typing;
    uint32_t enable_macro;
    uint32_t version;
    uint32_t sequence;
    uint32_t reserved;
};
struct TsfFocusState {
    uint64_t last_active_tick;
    uint32_t last_active_pid;
    uint32_t is_composing;
};
#pragma pack(pop)
static_assert(sizeof(TsfSharedConfig) == 48, "Rust/C++ config ABI mismatch");
static_assert(sizeof(TsfFocusState) == 16, "Rust/C++ focus ABI mismatch");

class SharedConfigReader {
public:
    explicit SharedConfigReader(const wchar_t* configName = nullptr,
                                const wchar_t* focusName = nullptr);
    ~SharedConfigReader();

    TsfSharedConfig Get();
    void UpdateActiveFocus(bool focused);
    void UpdateActiveKeystroke(bool isComposing);

private:
    void EnsureMaps();
    HANDLE m_hMapping = nullptr;
    HANDLE m_hFocus = nullptr;
    const TsfSharedConfig* m_pView = nullptr;
    TsfFocusState* m_pFocus = nullptr;
    TsfSharedConfig m_cached = {};
    bool m_focusRequested = false;
    const wchar_t* m_configName = nullptr;
    const wchar_t* m_focusName = nullptr;
};

