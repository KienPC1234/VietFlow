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
    bool IsDirty() const { return dirty_; }
    void SaveConfig();
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
    bool dirty_ = false;
    float saved_toast_timer_ = 0.0f;
    std::function<void()> on_saved_cb_;

    // Macro page temp buffers
    char new_macro_key_[64] = {0};
    char new_macro_val_[256] = {0};

    // Exclusion page temp buffer & running apps picker
    char new_excluded_app_[128] = {0};
    char app_search_filter_[128] = {0};
    std::vector<RunningApp> running_apps_list_;

    // Typing page temp buffer
    char new_english_kw_[128] = {0};
    char foreign_initials_buf_[64] = {0};

    // Converter page state
    int conv_src_cs_ = 2; // TCVN3
    int conv_dst_cs_ = 0; // Unicode
    bool conv_uppercase_ = false;
    bool conv_lowercase_ = false;
    bool conv_strip_accents_ = false;
    std::string conv_status_msg_;
    bool conv_status_ok_ = true;

    // Hotkey recording state
    bool recording_hotkey_ = false;
};

} // namespace vietflow
