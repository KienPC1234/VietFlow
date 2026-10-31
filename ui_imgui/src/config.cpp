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
    wchar_t path[MAX_PATH];
    if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, path))) {
        std::wstring ws(path);
        std::filesystem::path p(ws);
        p /= APP_DIR_NAME;
        std::filesystem::create_directories(p);
        // Must match platforms/windows/src/config.rs::AppConfig::config_path().
        p /= CONFIG_FILE_NAME;
        return p.string();
    }
    return CONFIG_FILE_NAME;
}

bool Config::Load(const std::string& custom_path) {
    std::string path = custom_path.empty() ? GetDefaultConfigPath() : custom_path;
    std::ifstream file(path);
    if (!file.is_open()) return false;

    try {
        json j;
        file >> j;

        enabled = j.value("enabled", enabled);
        method = j.value("method", method);
        charset = j.value("charset", charset);
        modern_style = j.value("modern_style", modern_style);
        free_marking = j.value("free_marking", free_marking);
        spell_check = j.value("spell_check", spell_check);
        auto_restore = j.value("auto_restore", auto_restore);
        fix_shift_lag = j.value("fix_shift_lag", fix_shift_lag);
        consonant_shortcuts = j.value("consonant_shortcuts", consonant_shortcuts);
        auto_capitalize = j.value("auto_capitalize", auto_capitalize);
        auto_capitalize_chars = j.value("auto_capitalize_chars", auto_capitalize_chars);
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
        hotkey_converter = j.value("hotkey_converter", hotkey_converter);

        if (j.contains("english_keywords") && j["english_keywords"].is_array()) {
            english_keywords = j["english_keywords"].get<std::vector<std::string>>();
        }
        if (j.contains("english_keywords_removed") && j["english_keywords_removed"].is_array()) {
            english_keywords_removed = j["english_keywords_removed"].get<std::vector<std::string>>();
        }
        NormalizeWordList(english_keywords);
        NormalizeWordList(english_keywords_removed);
        foreign_initials = j.value("foreign_initials", foreign_initials);
        tray_icon_theme = j.value("tray_icon_theme", tray_icon_theme);
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
        lock_hotkeys_in_excluded_apps = j.value("lock_hotkeys_in_excluded_apps", lock_hotkeys_in_excluded_apps);
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
            macros = j["macros"].get<std::map<std::string, std::string>>();
        }
        if (j.contains("custom_keymap") && j["custom_keymap"].is_object()) {
            const auto& ck = j["custom_keymap"];
            custom_keymap.acute = ck.value("acute", custom_keymap.acute);
            custom_keymap.grave = ck.value("grave", custom_keymap.grave);
            custom_keymap.hook = ck.value("hook", custom_keymap.hook);
            custom_keymap.tilde = ck.value("tilde", custom_keymap.tilde);
            custom_keymap.dot = ck.value("dot", custom_keymap.dot);
            custom_keymap.remove_tone = ck.value("remove_tone", custom_keymap.remove_tone);

            custom_keymap.circumflex = ck.value("circumflex", custom_keymap.circumflex);
            custom_keymap.horn = ck.value("horn", custom_keymap.horn);
            custom_keymap.breve = ck.value("breve", custom_keymap.breve);
            custom_keymap.d_mark = ck.value("d_mark", custom_keymap.d_mark);
            custom_keymap.hook_all = ck.value("hook_all", custom_keymap.hook_all);

            custom_keymap.roof_a = ck.value("roof_a", custom_keymap.roof_a);
            custom_keymap.roof_e = ck.value("roof_e", custom_keymap.roof_e);
            custom_keymap.roof_o = ck.value("roof_o", custom_keymap.roof_o);
            custom_keymap.horn_o = ck.value("horn_o", custom_keymap.horn_o);
            custom_keymap.horn_u = ck.value("horn_u", custom_keymap.horn_u);
            custom_keymap.breve_a = ck.value("breve_a", custom_keymap.breve_a);
            custom_keymap.d_lower = ck.value("d_lower", custom_keymap.d_lower);

            custom_keymap.telex_double_keys = ck.value("telex_double_keys", custom_keymap.telex_double_keys);
            custom_keymap.lone_w = ck.value("lone_w", custom_keymap.lone_w);
        }
        return true;
    } catch (...) {
        return false;
    }
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
        j["fix_shift_lag"] = fix_shift_lag;
        j["consonant_shortcuts"] = consonant_shortcuts;
        j["auto_capitalize"] = auto_capitalize;
        j["auto_capitalize_chars"] = auto_capitalize_chars;
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
        j["hotkey_converter"] = hotkey_converter;

        j["english_keywords"] = english_keywords;
        j["english_keywords_removed"] = english_keywords_removed;
        j["foreign_initials"] = foreign_initials;
        j["tray_icon_theme"] = tray_icon_theme;
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
        j["lock_hotkeys_in_excluded_apps"] = lock_hotkeys_in_excluded_apps;

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

        json ck;
        ck["acute"] = custom_keymap.acute;
        ck["grave"] = custom_keymap.grave;
        ck["hook"] = custom_keymap.hook;
        ck["tilde"] = custom_keymap.tilde;
        ck["dot"] = custom_keymap.dot;
        ck["remove_tone"] = custom_keymap.remove_tone;

        ck["circumflex"] = custom_keymap.circumflex;
        ck["horn"] = custom_keymap.horn;
        ck["breve"] = custom_keymap.breve;
        ck["d_mark"] = custom_keymap.d_mark;
        ck["hook_all"] = custom_keymap.hook_all;

        ck["roof_a"] = custom_keymap.roof_a;
        ck["roof_e"] = custom_keymap.roof_e;
        ck["roof_o"] = custom_keymap.roof_o;
        ck["horn_o"] = custom_keymap.horn_o;
        ck["horn_u"] = custom_keymap.horn_u;
        ck["breve_a"] = custom_keymap.breve_a;
        ck["d_lower"] = custom_keymap.d_lower;

        ck["telex_double_keys"] = custom_keymap.telex_double_keys;
        ck["lone_w"] = custom_keymap.lone_w;
        j["custom_keymap"] = ck;

        std::ofstream file(path);
        if (!file.is_open()) return false;
        file << j.dump(2);
        return true;
    } catch (...) {
        return false;
    }
}

} // namespace vietflow
