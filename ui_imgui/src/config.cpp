#include "config.h"
#include <nlohmann/json.hpp>
#include <fstream>
#include <filesystem>
#include <windows.h>
#include <shlobj.h>

namespace vietflow {

using json = nlohmann::json;

std::string Config::GetDefaultConfigPath() {
    wchar_t path[MAX_PATH];
    if (SUCCEEDED(SHGetFolderPathW(NULL, CSIDL_APPDATA, NULL, 0, path))) {
        std::wstring ws(path);
        std::filesystem::path p(ws);
        p /= "VietFlow";
        std::filesystem::create_directories(p);
        // Must match platforms/windows/src/config.rs::AppConfig::config_path().
        p /= "vietflow_config.json";
        return p.string();
    }
    return "vietflow_config.json";
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
        disable_on_non_us_layout = j.value("disable_on_non_us_layout", disable_on_non_us_layout);
        enable_macro = j.value("enable_macro", enable_macro);
        allow_macro_when_disabled = j.value("allow_macro_when_disabled", allow_macro_when_disabled);
        use_clipboard_for_unicode = j.value("use_clipboard_for_unicode", use_clipboard_for_unicode);
        start_with_windows = j.value("start_with_windows", start_with_windows);
        notification_sound = j.value("notification_sound", notification_sound);
        typing_timeout_ms = j.value("typing_timeout_ms", typing_timeout_ms);
        hotkey_mode = j.value("hotkey_mode", hotkey_mode);
        hotkey_settings = j.value("hotkey_settings", hotkey_settings);
        hotkey_converter = j.value("hotkey_converter", hotkey_converter);

        if (j.contains("english_keywords") && j["english_keywords"].is_array()) {
            english_keywords = j["english_keywords"].get<std::vector<std::string>>();
        }
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
        if (j.contains("macros") && j["macros"].is_object()) {
            macros = j["macros"].get<std::map<std::string, std::string>>();
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
        j["disable_on_non_us_layout"] = disable_on_non_us_layout;
        j["enable_macro"] = enable_macro;
        j["allow_macro_when_disabled"] = allow_macro_when_disabled;
        j["use_clipboard_for_unicode"] = use_clipboard_for_unicode;
        j["start_with_windows"] = start_with_windows;
        j["notification_sound"] = notification_sound;
        j["typing_timeout_ms"] = typing_timeout_ms;
        j["hotkey_mode"] = hotkey_mode;
        j["hotkey_settings"] = hotkey_settings;
        j["hotkey_converter"] = hotkey_converter;

        j["english_keywords"] = english_keywords;
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
        j["macros"] = macros;

        std::ofstream file(path);
        if (!file.is_open()) return false;
        file << j.dump(2);
        return true;
    } catch (...) {
        return false;
    }
}

} // namespace vietflow
