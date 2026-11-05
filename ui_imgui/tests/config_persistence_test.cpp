// Isolated Windows regression for C++ GUI configuration persistence.
// Opt-in via -DVIETFLOW_BUILD_CONFIG_TESTS=ON; never touches the live config.
#include "config.h"
#include <windows.h>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <iterator>
#include <string>

namespace fs = std::filesystem;

static std::string bytes(const fs::path& path) {
    std::ifstream in(path, std::ios::binary);
    return {std::istreambuf_iterator<char>(in), std::istreambuf_iterator<char>()};
}

static bool check(bool condition, const char* message) {
    if (!condition) std::cerr << "FAIL: " << message << "\n";
    return condition;
}

int main() {
    try {
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
        std::cout << "PASS: C++ config backup, quarantine and atomic recovery\n";
        return 0;
    } catch (const std::exception& ex) {
        std::cerr << "FAIL: exception: " << ex.what() << "\n";
        return 99;
    }
}
