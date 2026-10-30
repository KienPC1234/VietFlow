#pragma once

#include <string>
#include <vector>
#include <map>

namespace vietflow {

struct Config {
    bool enabled = true;
    int method = 0;       // 0: Telex, 1: VNI, 2: VIQR, 3: Simple Telex, 4: Simple Telex 2, 5: Telex + VNI
    int charset = 0;      // 0: Unicode, 1: Composite, 2: TCVN3, 3: VNI Windows
    bool modern_style = true;
    bool free_marking = true;
    bool spell_check = true;
    bool auto_restore = true;
    bool fix_shift_lag = true;
    bool consonant_shortcuts = false;
    bool auto_capitalize = false;
    bool disable_on_non_us_layout = false;
    bool enable_macro = true;
    bool allow_macro_when_disabled = false;
    bool use_clipboard_for_unicode = false;
    bool start_with_windows = false;
    bool notification_sound = false;
    uint64_t typing_timeout_ms = 1500;
    std::string hotkey_mode = "CtrlShift";
    std::string hotkey_settings = "Ctrl + Shift + F5";
    std::string hotkey_converter = "Ctrl + Shift + F9";

    std::vector<std::string> english_keywords;
    std::string foreign_initials = "fjwz";
    std::string tray_icon_theme = "modern";
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

    std::vector<std::string> excluded_apps;
    std::map<std::string, std::string> macros;

    static std::string GetDefaultConfigPath();
    bool Load(const std::string& path = "");
    bool Save(const std::string& path = "") const;
};

} // namespace vietflow
