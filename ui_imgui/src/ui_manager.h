#pragma once

#include "config.h"
#include <functional>
#include <string>

namespace vietflow {

enum class Page {
    Overview,
    Typing,
    Macros,
    Exclusions,
    Converter,
    Advanced,
    About
};

struct RunningApp {
    std::string exe_name;
    std::string window_title;
};

class UiManager {
public:
    UiManager(const std::string& config_path, bool convert_mode = false, void* hwnd = nullptr);

    void Render();
    void SaveConfig();
    void ResetToDefaults();
    void SetOnSavedCallback(std::function<void()> cb) { on_saved_cb_ = cb; }

private:
    void PageOverview();
    void PageTyping();
    void PageMacros();
    void PageExclusions();
    void PageConverter();
    void PageAdvanced();
    void PageAbout();

    Config config_;
    std::string config_path_;
    void* hwnd_ = nullptr;
    Page initial_page_ = Page::Overview;
    long long last_config_mtime_ = 0;
    float saved_toast_timer_ = 0.0f;
    std::function<void()> on_saved_cb_;

    // Macro page temp buffers
    char new_macro_key_[64] = {0};
    char new_macro_val_[256] = {0};
    char macro_filter_[128] = {0};

    // System state
    bool is_running_as_admin_ = false;

    // Exclusion page temp buffer & running apps picker
    char new_excluded_app_[128] = {0};
    char new_profile_app_[128] = {0};
    char app_search_filter_[128] = {0};
    std::vector<RunningApp> running_apps_list_;

    // English keyword editor
    char english_keyword_filter_[128] = {0};
    std::vector<std::string> english_defaults_;

    // Hotkey recorder target: 1 = settings, 2 = converter
    int recording_hotkey_target_ = 0;

    // Typing page temp buffer
    char new_english_kw_[128] = {0};
    char foreign_initials_buf_[64] = {0};
    char auto_capitalize_buf_[64] = {0};

    // Other global hotkeys (settings / converter)
    char hotkey_settings_buf_[64] = {0};
    char hotkey_converter_buf_[64] = {0};

    // Converter page state
    int conv_src_cs_ = 2; // TCVN3
    int conv_dst_cs_ = 0; // Unicode
    bool conv_uppercase_ = false;
    bool conv_lowercase_ = false;
    bool conv_strip_accents_ = false;
    std::string conv_status_msg_;
    bool conv_status_ok_ = true;
    std::string macro_status_msg_;
    bool macro_status_ok_ = true;

    // Custom keymap buffers & helpers
    char custom_acute_[16] = {0};
    char custom_grave_[16] = {0};
    char custom_hook_[16] = {0};
    char custom_tilde_[16] = {0};
    char custom_dot_[16] = {0};
    char custom_remove_tone_[16] = {0};
    char custom_circumflex_[16] = {0};
    char custom_horn_[16] = {0};
    char custom_breve_[16] = {0};
    char custom_d_mark_[16] = {0};
    char custom_hook_all_[16] = {0};
    char custom_roof_a_[16] = {0};
    char custom_roof_e_[16] = {0};
    char custom_roof_o_[16] = {0};
    char custom_horn_o_[16] = {0};
    char custom_horn_u_[16] = {0};
    char custom_breve_a_[16] = {0};
    char custom_d_lower_[16] = {0};
    char custom_test_buf_[256] = {0};

    void SyncCustomKeymapBuffersFromConfig();
    void SyncCustomKeymapBuffersToConfig();
    void SyncTextBuffersFromConfig();
    void LoadCustomKeymapPreset(int preset_id);

    // Hotkey recording state
    bool recording_hotkey_ = false;
};

} // namespace vietflow
