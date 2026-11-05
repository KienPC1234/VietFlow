// Cross-process TSF sequence-lock stress on isolated names.
// No production mappings, keyboard hooks, COM registration or installer actions.
#include "ipc.h"
#include <windows.h>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <string>
#include <vector>

static int Fail(const char* message) {
    std::cerr << "TSF IPC process stress FAIL: " << message << '\n';
    return 1;
}

static bool Coherent(const TsfSharedConfig& cfg) {
    return cfg.version == 2 && cfg.reserved == 0 &&
           (cfg.sequence & 1u) == 0 &&
           ((cfg.method == 3 && cfg.charset == 2 &&
             cfg.enabled == 1 && cfg.auto_restore == 1) ||
            (cfg.method == 5 && cfg.charset == 4 &&
             cfg.enabled == 0 && cfg.auto_restore == 0));
}

static volatile LONG* Field(uint32_t* p) {
    return reinterpret_cast<volatile LONG*>(p);
}

static int WriteChild(const std::wstring& configName) {
    HANDLE mapping = OpenFileMappingW(FILE_MAP_WRITE, FALSE, configName.c_str());
    if (!mapping) return Fail("child cannot open mapping");
    auto* cfg = static_cast<TsfSharedConfig*>(
        MapViewOfFile(mapping, FILE_MAP_WRITE, 0, 0, sizeof(TsfSharedConfig)));
    if (!cfg) {
        CloseHandle(mapping);
        return Fail("child cannot map writable view");
    }
    for (LONG i = 0; i < 16000; ++i) {
        const bool profileB = (i & 1) != 0;
        const LONG odd = 2 * i + 1;
        InterlockedExchange(Field(&cfg->sequence), odd);
        InterlockedExchange(Field(&cfg->method), profileB ? 5 : 3);
        InterlockedExchange(Field(&cfg->charset), profileB ? 4 : 2);
        InterlockedExchange(Field(&cfg->enabled), profileB ? 0 : 1);
        InterlockedExchange(Field(&cfg->auto_restore), profileB ? 0 : 1);
        InterlockedExchange(Field(&cfg->version), 2);
        InterlockedExchange(Field(&cfg->reserved), 0);
        InterlockedExchange(Field(&cfg->sequence), odd + 1);
        if ((i & 31) == 0) Sleep(1);
    }
    UnmapViewOfFile(cfg);
    CloseHandle(mapping);
    return 0;
}

int main(int argc, char** argv) {
    if (argc == 3 && std::strcmp(argv[1], "--writer") == 0) {
        const std::string narrow(argv[2]);
        // Map names are ASCII-only, unique to this test instance.
        return WriteChild(std::wstring(narrow.begin(), narrow.end()));
    }

    const std::wstring suffix = std::to_wstring(GetCurrentProcessId()) +
                                L"_" + std::to_wstring(GetTickCount64());
    const std::wstring configName = L"Local\\VietFlow_Config_Stress_" + suffix;
    const std::wstring focusName = L"Local\\VietFlow_Focus_Stress_" + suffix;
    HANDLE configMapping = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr,
        PAGE_READWRITE, 0, sizeof(TsfSharedConfig), configName.c_str());
    HANDLE focusMapping = CreateFileMappingW(INVALID_HANDLE_VALUE, nullptr,
        PAGE_READWRITE, 0, sizeof(TsfFocusState), focusName.c_str());
    if (!configMapping || !focusMapping) return Fail("parent mapping creation");
    auto* cfg = static_cast<TsfSharedConfig*>(
        MapViewOfFile(configMapping, FILE_MAP_WRITE, 0, 0, sizeof(TsfSharedConfig)));
    auto* focus = static_cast<TsfFocusState*>(
        MapViewOfFile(focusMapping, FILE_MAP_WRITE, 0, 0, sizeof(TsfFocusState)));
    if (!cfg || !focus) return Fail("parent mapping views");
    *cfg = {};
    cfg->version = 2;
    cfg->method = 3;
    cfg->charset = 2;
    cfg->enabled = 1;
    cfg->auto_restore = 1;
    *focus = {};

    int result = 0;
    {
        SharedConfigReader reader(configName.c_str(), focusName.c_str());
        reader.UpdateActiveFocus(true);
        if (!Coherent(reader.Get())) return Fail("initial reader state");

        wchar_t module[MAX_PATH] = {};
        DWORD count = GetModuleFileNameW(nullptr, module, MAX_PATH);
        if (count == 0 || count >= MAX_PATH) return Fail("module path");
        std::wstring command = L"\"" + std::wstring(module) +
                               L"\" --writer \"" + configName + L"\"";
        std::vector<wchar_t> mutableCommand(command.begin(), command.end());
        mutableCommand.push_back(0);
        STARTUPINFOW start = {};
        start.cb = sizeof(start);
        PROCESS_INFORMATION proc = {};
        if (!CreateProcessW(nullptr, mutableCommand.data(), nullptr, nullptr,
                            FALSE, CREATE_NO_WINDOW, nullptr, nullptr, &start, &proc)) {
            return Fail("child process creation");
        }
        CloseHandle(proc.hThread);
        bool sawProfileB = false;
        for (int i = 0; i < 500000; ++i) {
            const TsfSharedConfig snapshot = reader.Get();
            if (!Coherent(snapshot)) {
                result = Fail("torn or invalid shared snapshot accepted");
                break;
            }
            sawProfileB = sawProfileB || (snapshot.method == 5);
            if (WaitForSingleObject(proc.hProcess, 0) == WAIT_OBJECT_0) break;
            if ((i & 255) == 0) Sleep(0);
        }
        if (WaitForSingleObject(proc.hProcess, 30000) != WAIT_OBJECT_0)
            result = Fail("child writer timeout");
        DWORD exitCode = 0;
        if (!GetExitCodeProcess(proc.hProcess, &exitCode) || exitCode != 0)
            result = Fail("child writer failed");
        CloseHandle(proc.hProcess);
        if (!sawProfileB) result = Fail("reader never observed changing writer state");
        if (!Coherent(reader.Get())) result = Fail("final stable snapshot");
        reader.UpdateActiveFocus(false);
    }
    UnmapViewOfFile(cfg);
    UnmapViewOfFile(focus);
    CloseHandle(configMapping);
    CloseHandle(focusMapping);
    if (result == 0)
        std::cout << "TSF IPC process stress PASS: 16000 cross-process updates, coherent reads\n";
    return result;
}
