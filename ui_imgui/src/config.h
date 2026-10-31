#pragma once

#include <string>
#include <vector>
#include <map>

namespace vietflow {

struct CustomKeymap {
    std::string acute = "s";
    std::string grave = "f";
    std::string hook = "r";
    std::string tilde = "x";
    std::string dot = "j";
    std::string remove_tone = "z";

    std::string circumflex = "";
    std::string horn = "";
    std::string breve = "";
    std::string d_mark = "";
    std::string hook_all = "w";

    std::string roof_a = "";
    std::string roof_e = "";
    std::string roof_o = "";
    std::string horn_o = "";
    std::string horn_u = "";
    std::string breve_a = "";
    std::string d_lower = "";

    bool telex_double_keys = true;
    bool lone_w = true;
};

struct AppProfile {
    int method = -1;          // -1 = inherit, 0..6
    int charset = -1;         // -1 = inherit, 0..3
    int injection_mode = -1;  // -1 = inherit, 0..4
    int allow_macro = -1;     // -1 = inherit, 0 = off, 1 = on
};

struct Config {
    bool enabled = true;
    int method = 0;       // 0: Telex, 1: VNI, 2: VIQR, 3: Simple Telex, 4: Simple Telex 2, 5: Telex + VNI, 6: User Custom
    int charset = 0;      // 0: Unicode, 1: Composite, 2: TCVN3, 3: VNI Windows
    bool modern_style = true;
    bool free_marking = true;
    bool spell_check = true;
    bool auto_restore = true;
    bool fix_shift_lag = true;
    bool consonant_shortcuts = false;
    bool auto_capitalize = false;
    std::string auto_capitalize_chars = ".!?\r\n";
    bool disable_on_non_us_layout = false;
    bool disable_on_non_unicode_window = true;
    bool enable_macro = false;
    bool allow_macro_when_disabled = false;
    int injection_mode = 0; // 0 Auto, 1 Unicode, 2 scan-code, 4 safe; 3 is legacy
    bool start_with_windows = false;
    bool notification_sound = false;
    bool use_tsf = false;
    uint64_t typing_timeout_ms = 1500;
    std::string hotkey_mode = "CtrlShift";
    std::string hotkey_settings = "Ctrl + Shift + F5";
    std::string hotkey_converter = "Ctrl + Shift + F9";

    std::vector<std::string> english_keywords;
    std::vector<std::string> english_keywords_removed;
    std::string foreign_initials = "fjwz";
    std::string custom_icon_v;
    std::string custom_icon_e;

    bool tray_icon_v_fill = true;
    std::string tray_icon_v_fill_color = "#FFFFFF";
    bool tray_icon_v_border = true;
    std::string tray_icon_v_border_color = "#FF007F";

    bool tray_icon_e_fill = true;
    std::string tray_icon_e_fill_color = "#FFFFFF";
    bool tray_icon_e_border = true;
    std::string tray_icon_e_border_color = "#0078D7";

    // Keep these defaults in sync with platforms/windows/src/config.rs.
    // They also make the UI safe to launch (and reset) before the daemon has
    // created the first config file.
    std::vector<std::string> excluded_apps = {
        "valorant.exe", "cs2.exe", "league of legends.exe", "overwatch.exe", "dota2.exe"
    };
    bool lock_hotkeys_in_excluded_apps = true;
    std::map<std::string, AppProfile> app_profiles;
    std::map<std::string, std::string> macros = {
        {"vf", "VietFlow"},
        {"vn", "Việt Nam"},
        {"chxhcnvn", "Cộng hòa Xã hội Chủ nghĩa Việt Nam"}
    };
    CustomKeymap custom_keymap;

    static std::string GetDefaultConfigPath();
    bool Load(const std::string& path = "");
    bool Save(const std::string& path = "") const;
};

} // namespace vietflow
