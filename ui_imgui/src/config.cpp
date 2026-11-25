#include "config.h"
#include "constants.h"
#include <nlohmann/json.hpp>
#include <algorithm>
#include <cctype>
#include <fstream>
#include <filesystem>
#include <windows.h>
#include <shlobj.h>

namespace vietflow {

using json = nlohmann::json;
// Same per-session writer lock as Rust config.rs: never truncate a live config.
class ConfigWriteLock {
public:
    ConfigWriteLock() : mutex_(CreateMutexW(nullptr, FALSE, L"Local\\VietFlow_Config_Write")) {
        if (mutex_) {
            DWORD result = WaitForSingleObject(mutex_, 5000);
            held_ = result == WAIT_OBJECT_0 || result == WAIT_ABANDONED;
        }
    }
    ~ConfigWriteLock() {
        if (held_) ReleaseMutex(mutex_);
        if (mutex_) CloseHandle(mutex_);
    }
    bool acquired() const { return held_; }
private:
    HANDLE mutex_ = nullptr;
    bool held_ = false;
};

// Flush staged UTF-8 JSON to disk before atomically publishing it.
static bool FlushFileToDisk(const std::filesystem::path& path) {
    HANDLE file = CreateFileW(path.c_str(), GENERIC_WRITE,
                             FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE,
                             nullptr, OPEN_EXISTING, FILE_ATTRIBUTE_NORMAL, nullptr);
    if (file == INVALID_HANDLE_VALUE) return false;
    BOOL ok = FlushFileBuffers(file);
    CloseHandle(file);
    return ok != FALSE;
}

static bool ReadConfigJson(const std::filesystem::path& path, json& result) {
    try {
        std::ifstream f(path, std::ios::binary);
        if (!f) return false;
        json parsed = json::parse(f);
        if (!parsed.is_object()) return false;
        result = std::move(parsed);
        return true;
    } catch (...) {
        return false;
    }
}




static void LowerAsciiInPlace(std::string& value) {
    for (char& c : value) {
        c = (char)std::tolower((unsigned char)c);
    }
}

static int NormalizeInjectionMode(int mode) {
    // -1 means "inherit" for a per-app profile. Clipboard mode (3) was removed;
    // migrate it to Auto. Keep Safe (4).
    if (mode == -1) return -1;
    return (mode == 0 || mode == 1 || mode == 2 || mode == 4) ? mode : 0;
}

static void NormalizeWordList(std::vector<std::string>& words) {
    for (auto& word : words) {
        std::string normalized;
        normalized.reserve(word.size());
        for (unsigned char c : word) {
            if (!std::isspace(c)) normalized.push_back((char)std::tolower(c));
        }
        word = std::move(normalized);
    }
    words.erase(std::remove(words.begin(), words.end(), std::string()), words.end());
    std::sort(words.begin(), words.end());
    words.erase(std::unique(words.begin(), words.end()), words.end());
}

std::string Config::GetDefaultConfigPath() {
    wchar_t exe_path[MAX_PATH];
    bool is_installed = false;
    std::filesystem::path exe_dir;
    if (GetModuleFileNameW(NULL, exe_path, MAX_PATH) > 0) {
        std::filesystem::path p(exe_path);
        exe_dir = p.parent_path();
        if (std::filesystem::exists(exe_dir / ".installed") || std::filesystem::exists(exe_dir / "msi.tag")) {
            is_installed = true;
        }
    }

    if (is_installed) {
        // MSI installation mode: NEVER use Program Files.
        // 1. Common Application Data (%ProgramData%\VietFlow\vietflow_config.json)
        wchar_t pd_path[MAX_PATH];
        if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_COMMON_APPDATA, NULL, 0, pd_path))) {
            std::filesystem::path p(pd_path);
            p /= APP_DIR_NAME;
            std::error_code ec;
            std::filesystem::create_directories(p, ec);
            auto target = p / CONFIG_FILE_NAME;
            // Migration check: If target does not exist, check legacy Program Files
            if (!std::filesystem::exists(target) && !exe_dir.empty()) {
                auto legacy_pf = exe_dir / CONFIG_FILE_NAME;
                if (std::filesystem::exists(legacy_pf)) {
                    std::filesystem::copy_file(legacy_pf, target, std::filesystem::copy_options::skip_existing, ec);
                }
            }
            return target.string();
        }

        // 2. User Application Data (%APPDATA%\VietFlow\vietflow_config.json)
        wchar_t path[MAX_PATH];
        if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, path))) {
            std::filesystem::path p(path);
            p /= APP_DIR_NAME;
            std::error_code ec;
            std::filesystem::create_directories(p, ec);
            return (p / CONFIG_FILE_NAME).string();
        }
    } else {
        // Portable mode:
        if (!exe_dir.empty()) {
            auto local_cfg = exe_dir / CONFIG_FILE_NAME;
            if (std::filesystem::exists(local_cfg)) {
                return local_cfg.string();
            }
            // Check write permission
            auto test_file = exe_dir / ".w_test.tmp";
            std::ofstream tf(test_file);
            if (tf.is_open()) {
                tf.close();
                std::error_code ec;
                std::filesystem::remove(test_file, ec);
                return local_cfg.string();
            }
        }

        // Shared common fallback for portable mode
        wchar_t path[MAX_PATH];
        if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, path))) {
            std::filesystem::path p(path);
            p /= APP_DIR_NAME;
            std::error_code ec;
            std::filesystem::create_directories(p, ec);
            return (p / CONFIG_FILE_NAME).string();
        }
    }
    return CONFIG_FILE_NAME;
}

bool Config::Load(const std::string& custom_path, bool use_backup) {
    const std::string path = custom_path.empty() ? GetDefaultConfigPath() : custom_path;
    const std::filesystem::path primary(path);
    auto backup = primary;
    backup += L".bak";
    const Config previous = *this;

    // A syntactically valid JSON object may still contain invalid types.
    // Try the complete conversion before accepting it; on any failure
    // restore the original in-memory config and try the last good backup.
    // This also prevents partially changed macros/options escaping from
    // an unsuccessful Load call.
    auto try_load = [&](const std::filesystem::path& candidate) -> bool {
        json j;
        if (!ReadConfigJson(candidate, j)) return false;
        try {
        enabled = j.value("enabled", enabled);
        method = j.value("method", method);
        charset = j.value("charset", charset);
        modern_style = j.value("modern_style", modern_style);
        free_marking = j.value("free_marking", free_marking);
        spell_check = j.value("spell_check", spell_check);
        auto_restore = j.value("auto_restore", auto_restore);
        auto_switch_by_typing = j.value("auto_switch_by_typing", auto_switch_by_typing);
        auto_detect_game_mode = j.value("auto_detect_game_mode", auto_detect_game_mode);
        pause_auto_after_manual = j.value("pause_auto_after_manual", pause_auto_after_manual);
        fix_shift_lag = j.value("fix_shift_lag", fix_shift_lag);
        disable_on_non_us_layout = j.value("disable_on_non_us_layout", disable_on_non_us_layout);
        disable_on_non_unicode_window = j.value("disable_on_non_unicode_window", disable_on_non_unicode_window);
        enable_macro = j.value("enable_macro", enable_macro);
        allow_macro_when_disabled = j.value("allow_macro_when_disabled", allow_macro_when_disabled);
        injection_mode = NormalizeInjectionMode(j.value("injection_mode", injection_mode));
        start_with_windows = j.value("start_with_windows", start_with_windows);
        notification_sound = j.value("notification_sound", notification_sound);
        use_tsf = j.value("use_tsf", use_tsf);
        typing_timeout_ms = j.value("typing_timeout_ms", typing_timeout_ms);
        hotkey_mode = j.value("hotkey_mode", hotkey_mode);
        hotkey_settings = j.value("hotkey_settings", hotkey_settings);

        if (j.contains("english_keywords") && j["english_keywords"].is_array()) {
            english_keywords = j["english_keywords"].get<std::vector<std::string>>();
        }
        if (j.contains("english_keywords_removed") && j["english_keywords_removed"].is_array()) {
            english_keywords_removed = j["english_keywords_removed"].get<std::vector<std::string>>();
        }
        NormalizeWordList(english_keywords);
        NormalizeWordList(english_keywords_removed);
        foreign_initials = j.value("foreign_initials", foreign_initials);
        custom_icon_v = j.value("custom_icon_v", custom_icon_v);
        custom_icon_e = j.value("custom_icon_e", custom_icon_e);

        tray_icon_v_fill = j.value("tray_icon_v_fill", tray_icon_v_fill);
        tray_icon_v_fill_color = j.value("tray_icon_v_fill_color", tray_icon_v_fill_color);
        tray_icon_v_border = j.value("tray_icon_v_border", tray_icon_v_border);
        tray_icon_v_border_color = j.value("tray_icon_v_border_color", tray_icon_v_border_color);

        tray_icon_e_fill = j.value("tray_icon_e_fill", tray_icon_e_fill);
        tray_icon_e_fill_color = j.value("tray_icon_e_fill_color", tray_icon_e_fill_color);
        tray_icon_e_border = j.value("tray_icon_e_border", tray_icon_e_border);
        tray_icon_e_border_color = j.value("tray_icon_e_border_color", tray_icon_e_border_color);

        if (j.contains("excluded_apps") && j["excluded_apps"].is_array()) {
            excluded_apps = j["excluded_apps"].get<std::vector<std::string>>();
        }
        smart_prefix_matching = j.value("smart_prefix_matching", smart_prefix_matching);
        lock_hotkeys_in_excluded_apps = j.value("lock_hotkeys_in_excluded_apps", lock_hotkeys_in_excluded_apps);
        remember_per_app_mode = j.value("remember_per_app_mode", remember_per_app_mode);
        auto_exclude_apps = j.value("auto_exclude_apps", auto_exclude_apps);
        if (j.contains("per_app_modes") && j["per_app_modes"].is_object()) {
            per_app_modes.clear();
            for (const auto& [k, v] : j["per_app_modes"].items()) {
                if (v.is_boolean()) {
                    per_app_modes[k] = v.get<bool>();
                }
            }
        }
        if (j.contains("app_profiles") && j["app_profiles"].is_object()) {
            app_profiles.clear();
            for (const auto& [name, value] : j["app_profiles"].items()) {
                if (!value.is_object()) continue;
                AppProfile profile;
                if (value.contains("method") && !value["method"].is_null()) profile.method = value["method"];
                if (value.contains("charset") && !value["charset"].is_null()) profile.charset = value["charset"];
                if (value.contains("injection_mode") && !value["injection_mode"].is_null()) {
                    profile.injection_mode = NormalizeInjectionMode(value["injection_mode"].get<int>());
                }
                if (value.contains("allow_macro") && !value["allow_macro"].is_null()) profile.allow_macro = value["allow_macro"].get<bool>() ? 1 : 0;
                app_profiles[name] = profile;
            }
        }
        if (j.contains("macros") && j["macros"].is_object()) {
            macros.clear();
            for (const auto& [key, value] : j["macros"].items()) {
                if (!value.is_string()) continue;
                std::string normalized = key;
                LowerAsciiInPlace(normalized);
                macros[normalized] = value.get<std::string>();
            }
        }
        // Clamp values read from hand-edited/corrupt JSON. ImGui combo boxes
        // index these values directly and would read past their arrays.
        // Removed UserCustom (6) migrates cleanly to Telex (0).
        method = (method >= 0 && method <= 5) ? method : 0;
        charset = std::clamp(charset, 0, 3);
        typing_timeout_ms = std::clamp<uint64_t>(typing_timeout_ms, 0, 10000);
        for (auto& entry : app_profiles) {
            AppProfile& profile = entry.second;
            if (profile.method >= 0) {
                profile.method = (profile.method <= 5) ? profile.method : 0;
            }
            if (profile.charset >= 0) profile.charset = std::clamp(profile.charset, 0, 3);
            if (profile.allow_macro >= 0) profile.allow_macro = std::clamp(profile.allow_macro, 0, 1);
        }

        return true;
        } catch (...) {
            *this = previous;
            return false;
        }
    };
    return try_load(primary) || (use_backup && try_load(backup));
}

bool Config::Save(const std::string& custom_path) const {
    std::string path = custom_path.empty() ? GetDefaultConfigPath() : custom_path;
    try {
        std::filesystem::path p(path);
        if (p.has_parent_path()) {
            std::filesystem::create_directories(p.parent_path());
        }

        json j;
        j["enabled"] = enabled;
        j["method"] = method;
        j["charset"] = charset;
        j["modern_style"] = modern_style;
        j["free_marking"] = free_marking;
        j["spell_check"] = spell_check;
        j["auto_restore"] = auto_restore;
        j["auto_switch_by_typing"] = auto_switch_by_typing;
        j["auto_detect_game_mode"] = auto_detect_game_mode;
        j["pause_auto_after_manual"] = pause_auto_after_manual;
        j["fix_shift_lag"] = fix_shift_lag;
        j["disable_on_non_us_layout"] = disable_on_non_us_layout;
        j["disable_on_non_unicode_window"] = disable_on_non_unicode_window;
        j["enable_macro"] = enable_macro;
        j["allow_macro_when_disabled"] = allow_macro_when_disabled;
        j["injection_mode"] = injection_mode;
        j["start_with_windows"] = start_with_windows;
        j["notification_sound"] = notification_sound;
        j["use_tsf"] = use_tsf;
        j["typing_timeout_ms"] = typing_timeout_ms;
        j["hotkey_mode"] = hotkey_mode;
        j["hotkey_settings"] = hotkey_settings;

        j["english_keywords"] = english_keywords;
        j["english_keywords_removed"] = english_keywords_removed;
        j["foreign_initials"] = foreign_initials;
        j["custom_icon_v"] = custom_icon_v;
        j["custom_icon_e"] = custom_icon_e;

        j["tray_icon_v_fill"] = tray_icon_v_fill;
        j["tray_icon_v_fill_color"] = tray_icon_v_fill_color;
        j["tray_icon_v_border"] = tray_icon_v_border;
        j["tray_icon_v_border_color"] = tray_icon_v_border_color;

        j["tray_icon_e_fill"] = tray_icon_e_fill;
        j["tray_icon_e_fill_color"] = tray_icon_e_fill_color;
        j["tray_icon_e_border"] = tray_icon_e_border;
        j["tray_icon_e_border_color"] = tray_icon_e_border_color;

        j["excluded_apps"] = excluded_apps;
        j["smart_prefix_matching"] = smart_prefix_matching;
        j["lock_hotkeys_in_excluded_apps"] = lock_hotkeys_in_excluded_apps;
        j["remember_per_app_mode"] = remember_per_app_mode;
        j["auto_exclude_apps"] = auto_exclude_apps;
        j["per_app_modes"] = per_app_modes;

        json profiles = json::object();
        for (const auto& [name, profile] : app_profiles) {
            json p = json::object();
            if (profile.method >= 0) p["method"] = profile.method;
            if (profile.charset >= 0) p["charset"] = profile.charset;
            if (profile.injection_mode >= 0) p["injection_mode"] = profile.injection_mode;
            if (profile.allow_macro >= 0) p["allow_macro"] = profile.allow_macro != 0;
            profiles[name] = p;
        }
        j["app_profiles"] = profiles;
        j["macros"] = macros;

        ConfigWriteLock lock;
        if (!lock.acquired()) return false;
        const auto now = std::to_wstring(GetTickCount64());
        const auto nonce = std::to_wstring(GetCurrentProcessId()) + L"." + now;
        const auto temp = std::filesystem::path(path).wstring() + L".tmp." + nonce;
        const std::filesystem::path staged(temp);
        const std::filesystem::path target(path);
        auto backup = target; backup += L".bak";
        auto backupTemp = backup; backupTemp += L".tmp." + nonce;

        {
            std::ofstream file(staged, std::ios::binary | std::ios::trunc);
            if (!file) return false;
            file << j.dump(2);
            file.flush();
            if (!file.good()) {
                std::error_code ec;
                std::filesystem::remove(staged, ec);
                return false;
            }
        }
        if (!FlushFileToDisk(staged)) {
            std::error_code ec; std::filesystem::remove(staged, ec);
            return false;
        }

        // A parseable object with wrong typed fields is NOT a valid backup.
        Config previous;
        if (previous.Load(path, false)) {
            // Backup only a valid previous config; never poison the last good copy.
            if (!CopyFileW(target.c_str(), backupTemp.c_str(), FALSE) ||
                !FlushFileToDisk(backupTemp) ||
                !MoveFileExW(backupTemp.c_str(), backup.c_str(),
                             MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
                std::error_code ec;
                std::filesystem::remove(staged, ec);
                std::filesystem::remove(backupTemp, ec);
                return false;
            }
        } else {
            std::error_code ec;
            const bool exists = std::filesystem::exists(target, ec);
            if (ec || (exists && !std::filesystem::is_regular_file(target, ec)) || ec) {
                std::filesystem::remove(staged, ec);
                return false;
            }
            if (exists) {
                // Keep malformed JSON verbatim for recovery, without
                // destroying an earlier good .bak or an existing snapshot.
                auto quarantine = target;
                quarantine += L".corrupt." + nonce;
                if (!CopyFileW(target.c_str(), quarantine.c_str(), TRUE)) {
                    std::filesystem::remove(staged, ec);
                    return false;
                }
                if (!FlushFileToDisk(quarantine)) {
                    std::filesystem::remove(staged, ec);
                    std::filesystem::remove(quarantine, ec);
                    return false;
                }
            }
        }
        if (!MoveFileExW(staged.c_str(), target.c_str(),
                         MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH)) {
            std::error_code ec;
            std::filesystem::remove(staged, ec);
            return false;
        }
        return true;
    } catch (...) {
        return false;
    }
}

} // namespace vietflow
