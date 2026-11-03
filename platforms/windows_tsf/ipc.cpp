// VietFlow Windows TSF - Cross-Process Shared Configuration
// Licensed under GNU General Public License v3.0 (GPLv3)

#include "ipc.h"

static const wchar_t* SHARED_MEM_NAME = L"Local\\VietFlow_TSF_Config";

SharedConfigReader::SharedConfigReader() {
    DWORD size = sizeof(TsfSharedConfig);
    m_hMapping = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE, 0, size, SHARED_MEM_NAME);
    if (m_hMapping) {
        m_pView = static_cast<TsfSharedConfig*>(MapViewOfFile(m_hMapping, FILE_MAP_ALL_ACCESS, 0, 0, size));
        if (m_pView) {
            m_writable = true;
            if (m_pView->version == 0) {
                m_pView->enabled = 1;
                m_pView->method = 0;
                m_pView->charset = 0;
                m_pView->modern_style = 1;
                m_pView->spell_check = 1;
                m_pView->free_marking = 1;
                m_pView->auto_restore = 1;
                m_pView->auto_switch_by_typing = 1;
                m_pView->enable_macro = 1;
                m_pView->version = 1;
                m_pView->last_active_tick = 0;
                m_pView->last_active_pid = 0;
                m_pView->is_composing = 0;
            }
        } else {
            m_pView = static_cast<TsfSharedConfig*>(MapViewOfFile(m_hMapping, FILE_MAP_READ, 0, 0, size));
        }
    }
}

SharedConfigReader::~SharedConfigReader() {
    if (m_pView) {
        UnmapViewOfFile(m_pView);
        m_pView = nullptr;
    }
    if (m_hMapping) {
        CloseHandle(m_hMapping);
        m_hMapping = nullptr;
    }
}

TsfSharedConfig SharedConfigReader::Get() {
    if (m_pView) {
        return *m_pView;
    }
    TsfSharedConfig def = {};
    def.enabled = 1;
    def.method = 0;
    def.charset = 0;
    def.modern_style = 1;
    def.spell_check = 1;
    def.free_marking = 1;
    def.auto_restore = 1;
    def.auto_switch_by_typing = 1;
    def.enable_macro = 1;
    def.version = 1;
    return def;
}

void SharedConfigReader::UpdateActiveFocus(bool focused) {
    if (m_pView && m_writable) {
        if (focused) {
            m_pView->last_active_pid = GetCurrentProcessId();
            m_pView->last_active_tick = GetTickCount64();
        } else if (m_pView->last_active_pid == GetCurrentProcessId()) {
            m_pView->last_active_pid = 0;
            m_pView->is_composing = 0;
        }
    }
}

void SharedConfigReader::UpdateActiveKeystroke(bool isComposing) {
    if (m_pView && m_writable) {
        m_pView->is_composing = isComposing ? 1 : 0;
        m_pView->last_active_tick = GetTickCount64();
    }
}

