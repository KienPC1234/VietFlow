// Isolated Windows regression for C++ GUI configuration persistence.
// Opt-in via -DVIETFLOW_BUILD_CONFIG_TESTS=ON; never touches the live config.
#include "config.h"
#include <windows.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>
#include <vector>

namespace fs = std::filesystem;

static std::string bytes(const fs::path& path) {
    std::ifstream in(path, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

static bool check(bool condition, const char* message) {
    if (!condition) std::cerr << "FAIL: " << message << "\n";
    return condition;
}

static bool spawn_writer(const fs::path& path, int marker, PROCESS_INFORMATION& out) {
    wchar_t exe[MAX_PATH] = {};
    const DWORD length = GetModuleFileNameW(nullptr, exe, MAX_PATH);
    if (length == 0 || length >= MAX_PATH) return false;
    const std::wstring cmd = L"\"" + std::wstring(exe) +
        L"\" --stress-writer \"" + path.wstring() +
        L"\" " + std::to_wstring(marker);
    std::vector<wchar_t> args(cmd.begin(), cmd.end());
    args.push_back(L'\0');
    STARTUPINFOW startup = {};
    startup.cb = sizeof(startup);
    return CreateProcessW(
        exe, args.data(), nullptr, nullptr, FALSE, CREATE_NO_WINDOW,
        nullptr, nullptr, &startup, &out
    ) != FALSE;
}

int main(int argc, char** argv) {
    try {
        if (argc == 4 && std::string(argv[1]) == "--stress-writer") {
            const int marker = std::stoi(argv[3]);
            if (marker != 1 && marker != 2) return 80;
            for (int i = 0; i < 60; ++i) {
                vietflow::Config writer;
                writer.method = marker;
                writer.enabled = marker == 1;
                writer.macros["writer-marker"] = marker == 1 ? "writer-one" : "writer-two";
                if (!writer.Save(argv[2])) return 81;
            }
            return 0;
        }
        const auto directory = fs::temp_directory_path() /
            ("vietflow-ui-config-test-" + std::to_string(GetCurrentProcessId()) +
             "-" + std::to_string(GetTickCount64()));
        fs::create_directories(directory);
        struct Cleanup {
            fs::path root;
            ~Cleanup() { std::error_code ec; fs::remove_all(root, ec); }
        } cleanup{directory};

        const auto primary = directory / "settings.json";
        auto backup = primary;
        backup += L".bak";

        vietflow::Config original;
        original.enabled = true;
        original.macros["recovery-marker"] = "preserve-me";
        if (!check(original.Save(primary.string()), "initial save")) return 1;

        vietflow::Config updated;
        updated.enabled = false;
        updated.method = 2;
        if (!check(updated.Save(primary.string()), "second valid save")) return 2;

        vietflow::Config previous;
        if (!check(previous.Load(backup.string()), "load last valid backup") ||
            !check(previous.macros["recovery-marker"] == "preserve-me", "backup data")) return 3;
        const std::string backup_before = bytes(backup);

        // A JSON object may parse successfully but fail typed deserialization
        // after applying earlier fields. The saved backup should recover the
        // entire config, not leave a half-updated in-memory UI state.
        {
            std::ofstream malformed(primary, std::ios::binary | std::ios::trunc);
            malformed << R"({"enabled":false,"method":"not-a-number","macros":{}})";
        }
        vietflow::Config semantic_recovery;
        semantic_recovery.enabled = false;
        semantic_recovery.method = 6;
        semantic_recovery.macros.clear();
        semantic_recovery.macros["sentinel"] = "should-be-replaced-from-backup";
        if (!check(semantic_recovery.Load(primary.string()), "semantic invalid primary falls back to backup") ||
            !check(semantic_recovery.enabled && semantic_recovery.method == 0,
                   "fallback restores coherent backup fields") ||
            !check(semantic_recovery.macros.at("recovery-marker") == "preserve-me",
                   "fallback restores backup macros") ||
            !check(semantic_recovery.macros.count("sentinel") == 0, "no leaked partial values")) return 9;

        // If neither candidate is safe, Load must fail without changing ANY
        // values. Both files are valid JSON syntax but contain wrong field types.
        const auto invalid = directory / "invalid-settings.json";
        auto invalid_backup = invalid;
        invalid_backup += L".bak";
        {
            std::ofstream out(invalid, std::ios::binary | std::ios::trunc);
            out << R"({"enabled":false,"method":"not-a-number"})";
        }
        {
            std::ofstream out(invalid_backup, std::ios::binary | std::ios::trunc);
            out << R"({"enabled":false,"method":[]})";
        }
        vietflow::Config unchanged;
        unchanged.enabled = true;
        unchanged.method = 5;
        unchanged.macros["custom-sentinel"] = "must survive failure";
        if (!check(!unchanged.Load(invalid.string()), "both semantically invalid candidates rejected") ||
            !check(unchanged.enabled && unchanged.method == 5,
                   "failed Load preserves scalar fields") ||
            !check(unchanged.macros.at("custom-sentinel") == "must survive failure",
                   "failed Load preserves macros")) return 10;

        {
            std::ofstream broken(primary, std::ios::binary | std::ios::trunc);
            broken << "{corrupt";
        }
        if (!check(updated.Save(primary.string()), "recover after corrupted primary")) return 4;

        size_t snapshots = 0;
        for (const auto& entry : fs::directory_iterator(directory)) {
            const auto filename = entry.path().filename().string();
            if (filename.find("settings.json.corrupt.") == 0) {
                ++snapshots;
                if (!check(bytes(entry.path()) == "{corrupt", "quarantine preserves original bytes")) return 5;
            }
        }
        if (!check(snapshots == 1, "exactly one quarantine snapshot")) return 6;
        if (!check(bytes(backup) == backup_before, "good backup never overwritten by corruption")) return 7;

        vietflow::Config recovered;
        if (!check(recovered.Load(primary.string()), "load recovered primary") ||
            !check(!recovered.enabled && recovered.method == 2, "new config contents")) return 8;
        // A syntactically valid but semantically invalid primary must not be
        // promoted over a known-good .bak on the next user-initiated Save.
        const auto semantic_path = directory / "semantic-save.json";
        auto semantic_backup = semantic_path;
        semantic_backup += L".bak";
        vietflow::Config known;
        known.macros["golden-backup"] = "keep-this";
        if (!check(known.Save(semantic_path.string()), "seed valid primary")) return 11;
        vietflow::Config changed = known;
        changed.method = 2;
        if (!check(changed.Save(semantic_path.string()), "establish known-good backup")) return 12;
        const std::string valid_backup = bytes(semantic_backup);
        const std::string semantic_bad = R"({"enabled":false,"method":"wrong-type"})";
        {
            std::ofstream out(semantic_path, std::ios::binary | std::ios::trunc);
            out << semantic_bad;
        }
        if (!check(changed.Save(semantic_path.string()), "save with semantic-corrupt primary")) return 13;
        if (!check(bytes(semantic_backup) == valid_backup,
                   "semantically corrupt JSON never replaces a valid backup")) return 14;
        bool found_quarantine = false;
        for (const auto& entry : fs::directory_iterator(directory)) {
            const auto filename = entry.path().filename().string();
            if (filename.find("semantic-save.json.corrupt.") == 0) {
                found_quarantine |= bytes(entry.path()) == semantic_bad;
            }
        }
        if (!check(found_quarantine, "semantic-corrupt primary quarantined verbatim")) return 15;
        vietflow::Config after_semantic_save;
        if (!check(after_semantic_save.Load(semantic_path.string()),
                   "repaired semantic config loads") ||
            !check(after_semantic_save.method == 2,
                   "replacement did not lose the new settings")) return 16;

        // Verify cross-PROCESS writer exclusion (not merely two threads).
        // The parent and children only access this one ephemeral test path.
        const auto simultaneous = directory / "concurrent-settings.json";
        vietflow::Config seed;
        seed.method = 1;
        seed.enabled = true;
        seed.macros["writer-marker"] = "writer-one";
        if (!check(seed.Save(simultaneous.string()), "seed shared writer fixture")) return 17;

        PROCESS_INFORMATION one = {}, two = {};
        if (!check(spawn_writer(simultaneous, 1, one), "spawn writer one")) return 18;
        if (!spawn_writer(simultaneous, 2, two)) {
            TerminateProcess(one.hProcess, 99);
            WaitForSingleObject(one.hProcess, 5000);
            CloseHandle(one.hThread);
            CloseHandle(one.hProcess);
            return 19;
        }
        HANDLE handles[2] = {one.hProcess, two.hProcess};
        const DWORD waited = WaitForMultipleObjects(2, handles, TRUE, 60000);
        if (waited != WAIT_OBJECT_0) {
            TerminateProcess(one.hProcess, 98);
            TerminateProcess(two.hProcess, 98);
            WaitForMultipleObjects(2, handles, TRUE, 5000);
        }
        DWORD code_one = 0, code_two = 0;
        GetExitCodeProcess(one.hProcess, &code_one);
        GetExitCodeProcess(two.hProcess, &code_two);
        CloseHandle(one.hThread);
        CloseHandle(two.hThread);
        CloseHandle(one.hProcess);
        CloseHandle(two.hProcess);
        if (!check(waited == WAIT_OBJECT_0 && code_one == 0 && code_two == 0,
                   "both independent config writers must complete")) return 20;

        auto verify_consistent = [](const fs::path& config_path) {
            vietflow::Config saved;
            if (!saved.Load(config_path.string(), false)) return false;
            const auto it = saved.macros.find("writer-marker");
            if (it == saved.macros.end()) return false;
            return (saved.method == 1 && saved.enabled && it->second == "writer-one") ||
                   (saved.method == 2 && !saved.enabled && it->second == "writer-two");
        };
        auto simultaneous_backup = simultaneous;
        simultaneous_backup += L".bak";
        if (!check(verify_consistent(simultaneous), "final primary is coherent") ||
            !check(verify_consistent(simultaneous_backup), "final backup is coherent")) return 21;
        for (const auto& entry : fs::directory_iterator(directory)) {
            const auto filename = entry.path().filename().string();
            if (filename.find("concurrent-settings.json.tmp.") == 0 ||
                filename.find("concurrent-settings.json.bak.tmp.") == 0) {
                if (!check(false, "a concurrent Save left an incomplete staging file")) return 22;
            }
        }
        std::cout << "PASS: C++ config semantic validation and 2-process atomic save\n";
        return 0;
    } catch (const std::exception& ex) {
        std::cerr << "FAIL: exception: " << ex.what() << "\n";
        return 99;
    }
}
