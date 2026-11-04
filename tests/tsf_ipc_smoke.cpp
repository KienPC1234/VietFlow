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
