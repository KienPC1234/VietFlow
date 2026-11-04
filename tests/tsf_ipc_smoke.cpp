// Isolated TSF config/focus protocol smoke test. No COM registration, real
// keyboard hook, production mapping names, or installer actions.
#include "ipc.h"
#include <windows.h>
#include <iostream>
#include <string>

static int Fail(const char* message) {
    std::cerr << "TSF IPC smoke FAIL: " << message << '\n';
    return 1;
}

int main() {
    const std::wstring id = std::to_wstring(GetCurrentProcessId()) +
                            L"_" + std::to_wstring(GetTickCount64());
    const std::wstring configName = L"Local\\VietFlow_Config_Smoke_" + id;
    const std::wstring focusName = L"Local\\VietFlow_Focus_Smoke_" + id;
    HANDLE configHandle = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE,
                                              0, sizeof(TsfSharedConfig), configName.c_str());
    HANDLE focusHandle = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr, PAGE_READWRITE,
                                             0, sizeof(TsfFocusState), focusName.c_str());
    if (!configHandle || !focusHandle) return Fail("mapping creation");
    auto* config = static_cast<TsfSharedConfig*>(
        MapViewOfFile(configHandle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(TsfSharedConfig)));
    auto* focus = static_cast<TsfFocusState*>(
        MapViewOfFile(focusHandle, FILE_MAP_READ | FILE_MAP_WRITE, 0, 0, sizeof(TsfFocusState)));
    if (!config || !focus) return Fail("mapping view");
    *config = {};
    *focus = {};
    config->version = 2;
    config->enabled = 1;
    config->method = 3;
    config->sequence = 0;

    int result = 0;
    {
        SharedConfigReader reader(configName.c_str(), focusName.c_str());
        reader.UpdateActiveFocus(true);
        const TsfSharedConfig initial = reader.Get();
        if (initial.method != 3 || initial.version != 2)
            result = Fail("initial coherent read");
        if (focus->last_active_pid != GetCurrentProcessId())
            result = Fail("published process focus");
        // Odd sequence blocks inconsistent snapshots: cached stable config wins.
        config->sequence = 1;
        config->method = 5;
        if (reader.Get().method != 3)
            result = Fail("torn config reader");
        config->sequence = 2;
        if (reader.Get().method != 5)
            result = Fail("committed config reader");

        // The native reader must not accept tampered or malformed IPC fields,
        // even if the shared sequence/version look coherent.
        const TsfSharedConfig valid = *config;
        auto rejectMalformed = [&](TsfSharedConfig candidate, const char* reason) {
            // Use method=6 as a valid changed sentinel; an incorrectly
            // accepted snapshot would switch from the last trusted method=5.
            candidate.method = 6;
            candidate.sequence = config->sequence + 2;
            *config = candidate;
            if (reader.Get().method != 5) result = Fail(reason);
            *config = valid;
            config->sequence = candidate.sequence + 2;
            if (reader.Get().method != 5) result = Fail("valid config recovery");
        };
        TsfSharedConfig malformed = valid;
        malformed.method = 7;
        malformed.sequence = config->sequence + 2;
        *config = malformed;
        if (reader.Get().method != 5) result = Fail("invalid method");
        *config = valid;
        config->sequence = malformed.sequence + 2;
        reader.Get();
        malformed = valid; malformed.charset = 5;
        rejectMalformed(malformed, "invalid charset");
        malformed = valid; malformed.enabled = 2;
        rejectMalformed(malformed, "invalid enabled");
        malformed = valid; malformed.modern_style = 3;
        rejectMalformed(malformed, "invalid modern_style");
        malformed = valid; malformed.spell_check = 3;
        rejectMalformed(malformed, "invalid spell_check");
        malformed = valid; malformed.free_marking = 3;
        rejectMalformed(malformed, "invalid free_marking");
        malformed = valid; malformed.auto_restore = 3;
        rejectMalformed(malformed, "invalid auto_restore");
        malformed = valid; malformed.auto_switch_by_typing = 3;
        rejectMalformed(malformed, "invalid auto_switch_by_typing");
        malformed = valid; malformed.enable_macro = 3;
        rejectMalformed(malformed, "invalid enable_macro");
        malformed = valid; malformed.version = 3;
        rejectMalformed(malformed, "invalid version");
        malformed = valid; malformed.reserved = 0xFFFFFFFF;
        rejectMalformed(malformed, "invalid reserved");

        reader.UpdateActiveKeystroke(true);
        if (focus->is_composing != 1)
            result = Fail("composition signal");
        reader.UpdateActiveFocus(false);
        if (focus->last_active_pid != 0 || focus->is_composing != 0)
            result = Fail("focus release");
        if (reader.Get().enabled != 0)
            result = Fail("fallback must disable TIP without focus");
    }
    UnmapViewOfFile(config);
    UnmapViewOfFile(focus);
    CloseHandle(configHandle);
    CloseHandle(focusHandle);
    if (!result) std::cout << "TSF IPC smoke PASS: coherent snapshot and isolated focus\n";
    return result;
}
