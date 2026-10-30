#include "ui_manager.h"
#include "theme.h"
#include <imgui.h>
#include <windows.h>
#include <commdlg.h>
#include <shellapi.h>
#include <algorithm>
#include <filesystem>
#include <map>

#ifndef VIETFLOW_VERSION
#define VIETFLOW_VERSION "0.1.0"
#endif

namespace vietflow {

static std::vector<RunningApp> GetRunningApps() {
    std::vector<RunningApp> apps;
    std::map<std::string, std::string> seen;

    EnumWindows([](HWND hwnd, LPARAM lParam) -> BOOL {
        if (!IsWindowVisible(hwnd)) return TRUE;
        int len = GetWindowTextLengthW(hwnd);
        if (len == 0) return TRUE;

        LONG ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE);
        if (ex_style & WS_EX_TOOLWINDOW) return TRUE;

        wchar_t title[256] = { 0 };
        GetWindowTextW(hwnd, title, sizeof(title) / sizeof(title[0]) - 1);

        DWORD pid = 0;
        GetWindowThreadProcessId(hwnd, &pid);
        if (pid == 0) return TRUE;

        HANDLE hProc = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, FALSE, pid);
        if (hProc) {
            wchar_t full_path[MAX_PATH] = { 0 };
            DWORD size = MAX_PATH;
            if (QueryFullProcessImageNameW(hProc, 0, full_path, &size)) {
                std::filesystem::path fp(full_path);
                std::string exe = fp.filename().string();
                std::transform(exe.begin(), exe.end(), exe.begin(), ::tolower);

                if (exe != "explorer.exe" && exe != "vietflow.exe" && exe != "taskmgr.exe") {
                    auto* map_ptr = reinterpret_cast<std::map<std::string, std::string>*>(lParam);
                    if (map_ptr->find(exe) == map_ptr->end()) {
                        int u8len = WideCharToMultiByte(CP_UTF8, 0, title, -1, nullptr, 0, nullptr, nullptr);
                        std::string u8title;
                        if (u8len > 0) {
                            u8title.resize(u8len - 1);
                            WideCharToMultiByte(CP_UTF8, 0, title, -1, &u8title[0], u8len, nullptr, nullptr);
                        }
                        (*map_ptr)[exe] = u8title;
                    }
                }
            }
            CloseHandle(hProc);
        }
        return TRUE;
    }, reinterpret_cast<LPARAM>(&seen));

    for (const auto& [exe, title] : seen) {
        apps.push_back({ exe, title });
    }
    std::sort(apps.begin(), apps.end(), [](const RunningApp& a, const RunningApp& b) {
        return a.exe_name < b.exe_name;
    });
    return apps;
}

UiManager::UiManager(const std::string& config_path, bool convert_mode, void* hwnd)
    : config_path_(config_path.empty() ? Config::GetDefaultConfigPath() : config_path),
      hwnd_(hwnd) {
    config_.Load(config_path_);
    if (convert_mode) {
        initial_page_ = Page::Converter;
    }
    strncpy_s(foreign_initials_buf_, config_.foreign_initials.c_str(), sizeof(foreign_initials_buf_) - 1);
}

void UiManager::SaveConfig() {
    config_.foreign_initials = foreign_initials_buf_;
    config_.Save(config_path_);
    dirty_ = false;
    saved_toast_timer_ = 2.0f;
    if (on_saved_cb_) {
        on_saved_cb_();
    }
    // Notify running daemon
    HWND hwnd = FindWindowW(L"VietFlowMessageWindowClass", L"VietFlowDaemon");
    if (hwnd) {
        PostMessageW(hwnd, 0x0400 + 104, 0, 0); // WM_USER + 104 = WM_RELOAD_CONFIG
    }
}

static void BeginPageScroll() {
    // Keep the tab bar and the Save button fixed; only the page body scrolls.
    // Without this, long macro/exclusion lists grow the outer window and push
    // the header off-screen.
    ImGui::BeginChild("##PageScroll", ImVec2(0.0f, 0.0f), false);
}

static void EndPageScroll() {
    ImGui::EndChild();
}

void UiManager::Render() {
    ImGuiIO& io = ImGui::GetIO();
    if (saved_toast_timer_ > 0.0f) {
        saved_toast_timer_ -= io.DeltaTime;
    }

    ImGuiViewport* viewport = ImGui::GetMainViewport();
    ImGui::SetNextWindowPos(viewport->WorkPos);
    ImGui::SetNextWindowSize(viewport->WorkSize);

    ImGuiWindowFlags window_flags = ImGuiWindowFlags_NoDecoration | ImGuiWindowFlags_NoMove |
                                   ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoSavedSettings;

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(Px(16.0f), Px(12.0f)));
    ImGui::Begin("VietFlowMainWindow", nullptr, window_flags);
    ImGui::PopStyleVar();

    // Top Header: Title + Status + Save button
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted(NF_KEYBOARD " VietFlow");
    ImGui::SameLine();
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.60f, 0.70f, 1.0f));
    ImGui::TextUnformatted("- Bộ gõ tiếng Việt thế hệ mới");
    ImGui::PopStyleColor();

    const char* save_btn_label = NF_SAVE " Lưu & Áp dụng";
    float save_btn_w = ImGui::CalcTextSize(save_btn_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    float save_x = ImGui::GetWindowWidth() - save_btn_w - ImGui::GetStyle().WindowPadding.x;

    if (saved_toast_timer_ > 0.0f) {
        // Draw the toast to the left of the button so save stays clickable.
        const char* saved_label = NF_CHECK " Đã lưu cài đặt";
        float saved_w = ImGui::CalcTextSize(saved_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
        ImGui::SameLine(save_x - saved_w - Px(12.0f));
        ImGui::TextColored(ImVec4(0.30f, 0.85f, 0.45f, 1.00f), "%s", saved_label);
        ImGui::SameLine(save_x);
    } else {
        ImGui::SameLine(save_x);
    }

    if (ImGui::Button(save_btn_label, ImVec2(save_btn_w, Px(28.0f)))) {
        SaveConfig();
    }

    ImGui::Spacing();

    // Native ImGui TabBar Navigation (No forced sidebar)
    static bool first_run = true;
    ImGuiTabBarFlags tab_bar_flags = ImGuiTabBarFlags_None;
    if (ImGui::BeginTabBar("MainTabs", tab_bar_flags)) {
        if (ImGui::BeginTabItem(NF_COG " Tổng quan")) {
            BeginPageScroll();
            PageOverview();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_KEYBOARD " Gõ & Chính tả")) {
            BeginPageScroll();
            PageTyping();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_BOLT " Gõ tắt (Macro)")) {
            BeginPageScroll();
            PageMacros();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_GAMEPAD " Loại trừ")) {
            BeginPageScroll();
            PageExclusions();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        ImGuiTabItemFlags conv_flags = (first_run && initial_page_ == Page::Converter) ? ImGuiTabItemFlags_SetSelected : 0;
        if (ImGui::BeginTabItem(NF_CONVERT " Chuyển mã", nullptr, conv_flags)) {
            BeginPageScroll();
            PageConverter();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_PAINT " Giao diện")) {
            BeginPageScroll();
            PageAdvanced();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_INFO " Giới thiệu")) {
            BeginPageScroll();
            PageAbout();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        first_run = false;
        ImGui::EndTabBar();
    }

    ImGui::End();
}

static std::string FormatHotkeyLabel(const std::string& mode) {
    if (mode == "CtrlShift") return "Ctrl + Shift";
    if (mode == "AltZ") return "Alt + Z";
    if (mode == "CtrlSpace") return "Ctrl + Space";
    if (mode == "ShiftSpace") return "Shift + Space";
    if (mode == "AltShift") return "Alt + Shift";
    if (mode == "CapsLock") return "Caps Lock";
    if (mode == "WinSpace") return "Win + Space";

    std::string result;
    for (size_t i = 0; i < mode.size(); ++i) {
        if (mode[i] == '+') {
            if (i > 0 && mode[i - 1] != ' ') result += " ";
            result += "+";
            if (i + 1 < mode.size() && mode[i + 1] != ' ') result += " ";
        } else {
            result += mode[i];
        }
    }
    return result;
}

void UiManager::PageOverview() {
    SectionHeader("Bộ gõ & Bảng mã");

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Trạng thái hiện tại:");
    ImGui::SameLine(0.0f, Px(16.0f));

    const char* status_text = config_.enabled ? "[V] Đang bật tiếng Việt" : "[E] Tạm dừng tiếng Việt";
    ImVec4 status_col = config_.enabled ? ImVec4(0.18f, 0.55f, 0.28f, 1.0f) : ImVec4(0.35f, 0.38f, 0.45f, 1.0f);
    ImGui::PushStyleColor(ImGuiCol_Button, status_col);
    if (ImGui::Button(status_text, ImVec2(0.0f, Px(28.0f)))) {
        config_.enabled = !config_.enabled;
        SaveConfig();
    }
    ImGui::PopStyleColor();

    ImGui::Spacing();

    if (ImGui::BeginTable("OverviewSelectors", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("Kiểu gõ", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("Bảng mã", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TextUnformatted("Kiểu gõ chính:");
        ImGui::SetNextItemWidth(-Px(10.0f));
        const char* methods[] = {
            "Telex (Chuẩn)", "VNI", "VIQR", "Simple Telex", "Simple Telex 2", "Telex + VNI (Hỗn hợp)"
        };
        if (ImGui::Combo("##MethodCombo", &config_.method, methods, IM_ARRAYSIZE(methods))) {
            SaveConfig();
        }

        ImGui::TableSetColumnIndex(1);
        ImGui::TextUnformatted("Bảng mã xuất:");
        ImGui::SetNextItemWidth(-Px(10.0f));
        const char* charsets[] = {
            "Unicode (Dựng sẵn)", "Unicode Tổ hợp", "TCVN3 (ABC)", "VNI Windows"
        };
        if (ImGui::Combo("##CharsetCombo", &config_.charset, charsets, IM_ARRAYSIZE(charsets))) {
            SaveConfig();
        }

        ImGui::EndTable();
    }

    SectionHeader("Phím tắt chuyển đổi [V] / [E]");

    // Current Hotkey status badge
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Phím tắt đang dùng:");
    ImGui::SameLine(0.0f, Px(10.0f));
    HotkeyBadge(FormatHotkeyLabel(config_.hotkey_mode).c_str());

    ImGui::Spacing();
    ImGui::TextDisabled("1. Chọn nhanh từ các phím tắt phổ biến:");
    ImGui::Spacing();

    struct HotkeyPreset { const char* mode; const char* label; };
    HotkeyPreset presets[] = {
        {"CtrlShift", "Ctrl + Shift"},
        {"AltZ", "Alt + Z"},
        {"CtrlSpace", "Ctrl + Space"},
        {"ShiftSpace", "Shift + Space"},
        {"AltShift", "Alt + Shift"},
        {"CapsLock", "Caps Lock"},
        {"WinSpace", "Win + Space"}
    };

    float avail_width = ImGui::GetContentRegionAvail().x;
    for (int i = 0; i < IM_ARRAYSIZE(presets); ++i) {
        bool is_active = (config_.hotkey_mode == presets[i].mode || FormatHotkeyLabel(config_.hotkey_mode) == presets[i].label);
        if (is_active) {
            ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.20f, 0.45f, 0.75f, 1.00f));
            ImGui::PushStyleColor(ImGuiCol_Border, ImVec4(0.40f, 0.75f, 1.00f, 1.00f));
        }

        float btn_w = ImGui::CalcTextSize(presets[i].label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(16.0f);
        if (ImGui::Button(presets[i].label, ImVec2(btn_w, Px(28.0f)))) {
            config_.hotkey_mode = presets[i].mode;
            recording_hotkey_ = false;
            SaveConfig();
        }

        if (is_active) {
            ImGui::PopStyleColor(2);
        }

        if (i < IM_ARRAYSIZE(presets) - 1) {
            float next_btn_w = ImGui::CalcTextSize(presets[i + 1].label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(16.0f);
            if (ImGui::GetCursorPosX() + next_btn_w + Px(8.0f) < avail_width) {
                ImGui::SameLine(0.0f, Px(8.0f));
            }
        }
    }

    ImGui::Spacing();
    ImGui::Spacing();
    ImGui::TextDisabled("2. Tự gán phím tắt linh hoạt:");
    ImGui::Spacing();

    // Section 2A: Interactive Key Recorder
    if (recording_hotkey_) {
        bool is_ctrl  = (GetAsyncKeyState(VK_CONTROL) & 0x8000) != 0;
        bool is_alt   = (GetAsyncKeyState(VK_MENU) & 0x8000) != 0;
        bool is_shift = (GetAsyncKeyState(VK_SHIFT) & 0x8000) != 0;
        bool is_win   = ((GetAsyncKeyState(VK_LWIN) & 0x8000) != 0) || ((GetAsyncKeyState(VK_RWIN) & 0x8000) != 0);

        if ((GetAsyncKeyState(VK_ESCAPE) & 0x8000) != 0) {
            recording_hotkey_ = false;
        }

        std::string live_held;
        if (is_ctrl) live_held += "Ctrl + ";
        if (is_alt) live_held += "Alt + ";
        if (is_shift) live_held += "Shift + ";
        if (is_win) live_held += "Win + ";

        struct KeyDef { int vk; const char* name; };
        static const KeyDef s_keys[] = {
            { VK_SPACE, "Space" }, { VK_CAPITAL, "CapsLock" }, { VK_TAB, "Tab" },
            { 0xC0, "~" },
            { VK_F1, "F1" }, { VK_F2, "F2" }, { VK_F3, "F3" }, { VK_F4, "F4" },
            { VK_F5, "F5" }, { VK_F6, "F6" }, { VK_F7, "F7" }, { VK_F8, "F8" },
            { VK_F9, "F9" }, { VK_F10, "F10" }, { VK_F11, "F11" }, { VK_F12, "F12" },
        };

        std::string triggered_key;
        for (const auto& kd : s_keys) {
            if ((GetAsyncKeyState(kd.vk) & 0x8000) != 0) {
                triggered_key = kd.name;
                break;
            }
        }
        if (triggered_key.empty()) {
            for (int k = 'A'; k <= 'Z'; ++k) {
                if ((GetAsyncKeyState(k) & 0x8000) != 0) {
                    triggered_key = std::string(1, (char)k);
                    break;
                }
            }
        }
        if (triggered_key.empty()) {
            for (int k = '0'; k <= '9'; ++k) {
                if ((GetAsyncKeyState(k) & 0x8000) != 0) {
                    triggered_key = std::string(1, (char)k);
                    break;
                }
            }
        }

        if (!triggered_key.empty()) {
            std::string final_hotkey;
            if (is_ctrl) final_hotkey += "Ctrl + ";
            if (is_alt) final_hotkey += "Alt + ";
            if (is_shift) final_hotkey += "Shift + ";
            if (is_win) final_hotkey += "Win + ";
            final_hotkey += triggered_key;

            config_.hotkey_mode = final_hotkey;
            recording_hotkey_ = false;
            SaveConfig();
        }

        std::string record_btn_label = std::string("[ ") + NF_BOLT + " Đang nghe: " + (live_held.empty() ? std::string("Bấm tổ hợp phím...") : live_held) + " (Esc: Hủy) ]";
        float btn_w = ImGui::CalcTextSize(record_btn_label.c_str()).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.70f, 0.20f, 0.20f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.80f, 0.25f, 0.25f, 1.0f));
        if (ImGui::Button(record_btn_label.c_str(), ImVec2(btn_w, Px(28.0f)))) {
            recording_hotkey_ = false;
        }
        ImGui::PopStyleColor(2);
    } else {
        const char* start_record_label = NF_COG "  Bấm để gán phím tự động";
        float btn_w = ImGui::CalcTextSize(start_record_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
        if (ImGui::Button(start_record_label, ImVec2(btn_w, Px(28.0f)))) {
            recording_hotkey_ = true;
        }
    }

    ImGui::Spacing();

    // Section 2B: Manual Custom Hotkey Builder
    static bool custom_ctrl = true;
    static bool custom_alt = false;
    static bool custom_shift = false;
    static bool custom_win = false;
    static int custom_key_idx = 0;
    const char* custom_keys[] = {
        "Space", "Z", "CapsLock", "~", "Tab",
        "F1", "F2", "F3", "F4", "F5", "F6", "F7", "F8", "F9", "F10", "F11", "F12",
        "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P", "Q", "R", "S", "T", "U", "V", "W", "X", "Y"
    };

    ImGui::AlignTextToFramePadding();
    ImGui::TextDisabled("Hoặc tự phối:");
    ImGui::SameLine(0.0f, Px(8.0f));
    ImGui::Checkbox("Ctrl", &custom_ctrl);
    ImGui::SameLine(0.0f, Px(8.0f));
    ImGui::Checkbox("Alt", &custom_alt);
    ImGui::SameLine(0.0f, Px(8.0f));
    ImGui::Checkbox("Shift", &custom_shift);
    ImGui::SameLine(0.0f, Px(8.0f));
    ImGui::Checkbox("Win", &custom_win);

    ImGui::SameLine(0.0f, Px(12.0f));
    ImGui::TextUnformatted("+");
    ImGui::SameLine(0.0f, Px(12.0f));
    ImGui::SetNextItemWidth(Px(110.0f));
    ImGui::Combo("##CustomKeySelect", &custom_key_idx, custom_keys, IM_ARRAYSIZE(custom_keys));

    ImGui::SameLine(0.0f, Px(10.0f));
    const char* assign_label = NF_SAVE " Gán tổ hợp này";
    float assign_w = ImGui::CalcTextSize(assign_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(16.0f);
    if (ImGui::Button(assign_label, ImVec2(assign_w, Px(26.0f)))) {
        std::string res;
        if (custom_ctrl) res += "Ctrl";
        if (custom_alt) { if (!res.empty()) res += " + "; res += "Alt"; }
        if (custom_shift) { if (!res.empty()) res += " + "; res += "Shift"; }
        if (custom_win) { if (!res.empty()) res += " + "; res += "Win"; }
        if (!res.empty()) res += " + ";
        res += custom_keys[custom_key_idx];

        config_.hotkey_mode = res;
        recording_hotkey_ = false;
        SaveConfig();
    }

    SectionHeader("Hệ thống & Khởi động");
    if (ImGui::Checkbox("Khởi động cùng Windows khi bật máy", &config_.start_with_windows)) SaveConfig();
    ImGui::Spacing();
    if (ImGui::Checkbox("Phát âm thanh thông báo nhẹ khi chuyển chế độ V/E", &config_.notification_sound)) SaveConfig();
}

void UiManager::PageTyping() {
    SectionHeader("Quy tắc bỏ dấu tiếng Việt");
    if (ImGui::Checkbox("Bỏ dấu tự do (có thể bỏ dấu bất kỳ vị trí nào trong từ)", &config_.free_marking)) SaveConfig();
    if (ImGui::Checkbox("Đặt dấu thanh kiểu mới (oà, uỳ, oè thay vì kiểu cũ òa, ùy, òe)", &config_.modern_style)) SaveConfig();
    if (ImGui::Checkbox("Kiểm tra chính tả tiếng Việt", &config_.spell_check)) SaveConfig();
    if (ImGui::Checkbox("Tự động khôi phục phím gốc với các từ gõ sai hoặc từ tiếng Anh", &config_.auto_restore)) SaveConfig();

    SectionHeader("Tối ưu hóa bàn phím");
    if (ImGui::Checkbox("Chống dính phím Shift khi gõ hoa", &config_.fix_shift_lag)) SaveConfig();
    if (ImGui::Checkbox("Cho phép gõ tắt phụ âm đầu (f->ph, j->gi, w->qu)", &config_.consonant_shortcuts)) SaveConfig();
    if (ImGui::Checkbox("Tự động viết hoa chữ cái đầu câu", &config_.auto_capitalize)) SaveConfig();
    if (ImGui::Checkbox("Tắt tiếng Việt khi bàn phím không phải US", &config_.disable_on_non_us_layout)) SaveConfig();
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Tránh gõ sai khi bạn chuyển sang bố cục bàn phím khác (ví dụ tiếng Pháp, Đức).");
    }

    SectionHeader("Ký tự ngoại lai & Từ tiếng Anh");
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Phụ âm ngoại lai bỏ qua dấu:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(120.0f));
    if (ImGui::InputText("##ForeignInitials", foreign_initials_buf_, sizeof(foreign_initials_buf_))) {
        SaveConfig();
    }
    ImGui::SameLine();
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.58f, 0.67f, 1.00f));
    ImGui::TextUnformatted("(Mặc định: fjwz. Bỏ qua dấu khi gặp các chữ cái này ở đầu từ)");
    ImGui::PopStyleColor();

    ImGui::Spacing();
    ImGui::TextUnformatted("Danh sách từ khóa tiếng Anh không bỏ dấu:");
    ImGui::SetNextItemWidth(Px(260.0f));
    ImGui::InputTextWithHint("##NewEngKw", "case, api, github...", new_english_kw_, sizeof(new_english_kw_));
    ImGui::SameLine();

    if (ImGui::Button(NF_PLUS "  Thêm từ", ImVec2(0.0f, Px(26.0f)))) {
        if (strlen(new_english_kw_) > 0) {
            config_.english_keywords.push_back(new_english_kw_);
            new_english_kw_[0] = '\0';
            SaveConfig();
        }
    }

    ImGui::Spacing();
    int to_remove = -1;
    for (size_t i = 0; i < config_.english_keywords.size(); ++i) {
        ImGui::Text("  • %s", config_.english_keywords[i].c_str());
        ImGui::SameLine(ImGui::GetContentRegionAvail().x - Px(70.0f));
        ImGui::PushID((int)i);
        if (ImGui::SmallButton(NF_TRASH "  Xóa")) {
            to_remove = (int)i;
        }
        ImGui::PopID();
    }
    if (to_remove >= 0) {
        config_.english_keywords.erase(config_.english_keywords.begin() + to_remove);
        SaveConfig();
    }
}

void UiManager::PageMacros() {
    SectionHeader("Thiết lập gõ tắt");
    if (ImGui::Checkbox("Bật tính năng gõ tắt", &config_.enable_macro)) SaveConfig();
    if (ImGui::Checkbox("Cho phép gõ tắt ngay cả khi ở chế độ [E]", &config_.allow_macro_when_disabled)) SaveConfig();

    SectionHeader("Thêm từ gõ tắt mới");

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Từ tắt:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(100.0f));
    ImGui::InputTextWithHint("##MacroKey", "vn", new_macro_key_, sizeof(new_macro_key_));

    ImGui::SameLine();
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Thay thế:");
    ImGui::SameLine();

    float avail = ImGui::GetContentRegionAvail().x;
    float btn_width = Px(110.0f);
    ImGui::SetNextItemWidth(avail - btn_width - Px(10.0f));
    ImGui::InputTextWithHint("##MacroVal", "Cụm từ đầy đủ...", new_macro_val_, sizeof(new_macro_val_));

    ImGui::SameLine();
    if (ImGui::Button(NF_SAVE "  Lưu", ImVec2(btn_width, Px(26.0f)))) {
        if (strlen(new_macro_key_) > 0 && strlen(new_macro_val_) > 0) {
            config_.macros[new_macro_key_] = new_macro_val_;
            new_macro_key_[0] = '\0';
            new_macro_val_[0] = '\0';
            SaveConfig();
        }
    }

    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.60f, 0.70f, 1.00f));
    ImGui::TextUnformatted("Hỗ trợ biến động: @date, @time, @datetime, @clipboard, @tab, @newline");
    ImGui::PopStyleColor();

    SectionHeader("Danh sách từ gõ tắt");
    std::string remove_key;
    if (ImGui::BeginTable("MacroTable", 3, ImGuiTableFlags_RowBg | ImGuiTableFlags_BordersInnerV)) {
        ImGui::TableSetupColumn("Từ tắt", ImGuiTableColumnFlags_WidthFixed, Px(120.0f));
        ImGui::TableSetupColumn("Cụm từ thay thế", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Thao tác", ImGuiTableColumnFlags_WidthFixed, Px(70.0f));
        ImGui::TableHeadersRow();

        for (const auto& [k, v] : config_.macros) {
            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::TextUnformatted(k.c_str());

            ImGui::TableSetColumnIndex(1);
            ImGui::TextUnformatted(v.c_str());

            ImGui::TableSetColumnIndex(2);
            ImGui::PushID(k.c_str());
            if (ImGui::SmallButton(NF_TRASH "  Xóa")) {
                remove_key = k;
            }
            ImGui::PopID();
        }
        ImGui::EndTable();
    }
    if (!remove_key.empty()) {
        config_.macros.erase(remove_key);
        SaveConfig();
    }
}

void UiManager::PageExclusions() {
    SectionHeader("Thêm ứng dụng loại trừ (Game, đồ họa, phần mềm chuyên dụng)");
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Tên ứng dụng:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(210.0f));
    ImGui::InputTextWithHint("##NewExclApp", "vd: cs2.exe...", new_excluded_app_, sizeof(new_excluded_app_));
    ImGui::SameLine();

    if (ImGui::Button(NF_PLUS " Thêm", ImVec2(Px(75.0f), Px(26.0f)))) {
        if (strlen(new_excluded_app_) > 0) {
            std::string app = new_excluded_app_;
            std::transform(app.begin(), app.end(), app.begin(), ::tolower);
            if (std::find(config_.excluded_apps.begin(), config_.excluded_apps.end(), app) == config_.excluded_apps.end()) {
                config_.excluded_apps.push_back(app);
                SaveConfig();
            }
            new_excluded_app_[0] = '\0';
        }
    }

    ImGui::Spacing();
    const char* file_btn_label = NF_FOLDER " Chọn file (.exe)...";
    float file_btn_w = ImGui::CalcTextSize(file_btn_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(file_btn_label, ImVec2(file_btn_w, Px(28.0f)))) {
        OPENFILENAMEW ofn = { sizeof(ofn) };
        wchar_t szFile[MAX_PATH] = { 0 };
        ofn.hwndOwner = (HWND)hwnd_;
        ofn.lpstrFile = szFile;
        ofn.nMaxFile = MAX_PATH;
        ofn.lpstrFilter = L"File thực thi (*.exe)\0*.exe\0Tất cả tập tin (*.*)\0*.*\0";
        ofn.nFilterIndex = 1;
        ofn.Flags = OFN_PATHMUSTEXIST | OFN_FILEMUSTEXIST | OFN_NOCHANGEDIR;

        if (GetOpenFileNameW(&ofn)) {
            std::filesystem::path fp(szFile);
            std::string exe = fp.filename().string();
            std::transform(exe.begin(), exe.end(), exe.begin(), ::tolower);
            if (!exe.empty() && std::find(config_.excluded_apps.begin(), config_.excluded_apps.end(), exe) == config_.excluded_apps.end()) {
                config_.excluded_apps.push_back(exe);
                SaveConfig();
            }
        }
    }

    ImGui::SameLine(0.0f, Px(12.0f));
    const char* app_btn_label = NF_GAMEPAD " Chọn từ app đang chạy...";
    float app_btn_w = ImGui::CalcTextSize(app_btn_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(app_btn_label, ImVec2(app_btn_w, Px(28.0f)))) {
        running_apps_list_ = GetRunningApps();
        app_search_filter_[0] = '\0';
        ImGui::OpenPopup("Chọn ứng dụng đang chạy");
    }

    // Modal popup to select running applications
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(Px(580.0f), Px(380.0f)), ImGuiCond_Appearing);

    if (ImGui::BeginPopupModal("Chọn ứng dụng đang chạy", nullptr, ImGuiWindowFlags_NoResize)) {
        ImGui::TextUnformatted("Chọn các ứng dụng hoặc trò chơi đang mở trên máy để loại trừ:");
        ImGui::Spacing();

        ImGui::SetNextItemWidth(-1.0f);
        ImGui::InputTextWithHint("##AppSearchFilter", "Tìm kiếm ứng dụng...", app_search_filter_, sizeof(app_search_filter_));
        ImGui::Spacing();

        std::string filter_str = app_search_filter_;
        std::transform(filter_str.begin(), filter_str.end(), filter_str.begin(), ::tolower);

        if (ImGui::BeginChild("RunningAppsList", ImVec2(0.0f, Px(230.0f)), true)) {
            if (running_apps_list_.empty()) {
                ImGui::TextDisabled("Không tìm thấy ứng dụng nào đang mở có cửa sổ.");
            } else {
                for (const auto& app : running_apps_list_) {
                    if (!filter_str.empty()) {
                        std::string lower_title = app.window_title;
                        std::transform(lower_title.begin(), lower_title.end(), lower_title.begin(), ::tolower);
                        if (app.exe_name.find(filter_str) == std::string::npos &&
                            lower_title.find(filter_str) == std::string::npos) {
                            continue;
                        }
                    }

                    bool already_added = (std::find(config_.excluded_apps.begin(), config_.excluded_apps.end(), app.exe_name) != config_.excluded_apps.end());

                    ImGui::PushID(app.exe_name.c_str());
                    if (already_added) {
                        ImGui::BeginDisabled();
                        ImGui::Button("Đã có", ImVec2(Px(75.0f), Px(22.0f)));
                        ImGui::EndDisabled();
                    } else {
                        if (ImGui::Button(NF_PLUS "  Thêm", ImVec2(Px(75.0f), Px(22.0f)))) {
                            config_.excluded_apps.push_back(app.exe_name);
                            SaveConfig();
                        }
                    }
                    ImGui::SameLine();
                    ImGui::Text(" %s", app.exe_name.c_str());
                    if (!app.window_title.empty()) {
                        ImGui::SameLine();
                        ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.60f, 0.70f, 1.0f));
                        ImGui::TextUnformatted(("(" + app.window_title + ")").c_str());
                        ImGui::PopStyleColor();
                    }
                    ImGui::PopID();
                }
            }
        }
        ImGui::EndChild();

        ImGui::Spacing();
        if (ImGui::Button("Đóng", ImVec2(Px(100.0f), Px(26.0f)))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    SectionHeader("Danh sách ứng dụng không dùng tiếng Việt");
    int remove_idx = -1;
    for (size_t i = 0; i < config_.excluded_apps.size(); ++i) {
        ImGui::Text("  • %s", config_.excluded_apps[i].c_str());
        ImGui::SameLine(ImGui::GetContentRegionAvail().x - Px(70.0f));
        ImGui::PushID((int)i);
        if (ImGui::SmallButton(NF_TRASH "  Xóa")) {
            remove_idx = (int)i;
        }
        ImGui::PopID();
    }
    if (remove_idx >= 0) {
        config_.excluded_apps.erase(config_.excluded_apps.begin() + remove_idx);
        SaveConfig();
    }
}

void UiManager::PageConverter() {
    SectionHeader("Bảng mã nguồn và đích");

    const char* charsets[] = {
        "Unicode (Dựng sẵn)", "Unicode Tổ hợp", "TCVN3 (ABC)", "VNI Windows"
    };

    if (ImGui::BeginTable("ConverterSelectors", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("Nguồn", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("Đích", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TextUnformatted("Bảng mã nguồn:");
        ImGui::SetNextItemWidth(-Px(10.0f));
        ImGui::Combo("##ConvSrcCs", &conv_src_cs_, charsets, IM_ARRAYSIZE(charsets));

        ImGui::TableSetColumnIndex(1);
        ImGui::TextUnformatted("Bảng mã đích:");
        ImGui::SetNextItemWidth(-Px(10.0f));
        ImGui::Combo("##ConvDstCs", &conv_dst_cs_, charsets, IM_ARRAYSIZE(charsets));

        // Swap control on its own row so the source/target combos stay on the
        // same baseline (previously the button pushed the target combo one row
        // lower than the source combo).
        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        if (ImGui::Button(NF_EXCHANGE "  Đảo chiều", ImVec2(0.0f, Px(22.0f)))) {
            std::swap(conv_src_cs_, conv_dst_cs_);
        }

        ImGui::EndTable();
    }

    ImGui::Spacing();
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Tùy chọn:");
    ImGui::SameLine();
    if (ImGui::Checkbox("Chữ hoa", &conv_uppercase_) && conv_uppercase_) conv_lowercase_ = false;
    ImGui::SameLine();
    if (ImGui::Checkbox("Chữ thường", &conv_lowercase_) && conv_lowercase_) conv_uppercase_ = false;
    ImGui::SameLine();
    ImGui::Checkbox("Bỏ dấu tiếng Việt", &conv_strip_accents_);

    ImGui::SameLine(0.0f, Px(20.0f));
    if (ImGui::Button(NF_CONVERT "  Chuyển mã Clipboard", ImVec2(0.0f, Px(26.0f)))) {
        conv_status_msg_ = "Đã chuyển mã và lưu vào Clipboard thành công!";
        conv_status_ok_ = true;
    }

    SectionHeader("Hộp văn bản trực tiếp");
    static char src_buf[4096] = {0};
    static char dst_buf[4096] = {0};

    if (ImGui::BeginTable("DirectConvBoxes", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("Nguồn", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("Kết quả", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TextUnformatted("Văn bản nguồn:");
        ImGui::InputTextMultiline("##ConvSrcText", src_buf, sizeof(src_buf), ImVec2(-Px(10.0f), Px(130.0f)));

        ImGui::TableSetColumnIndex(1);
        ImGui::TextUnformatted("Kết quả chuyển đổi:");
        ImGui::InputTextMultiline("##ConvDstText", dst_buf, sizeof(dst_buf), ImVec2(-Px(10.0f), Px(130.0f)));

        ImGui::EndTable();
    }

    ImGui::Spacing();
    if (ImGui::Button(NF_CONVERT "  Chuyển mã văn bản", ImVec2(0.0f, Px(28.0f)))) {
        strncpy_s(dst_buf, src_buf, sizeof(dst_buf) - 1);
        conv_status_msg_ = "Đã chuyển mã văn bản thành công.";
    }

    ImGui::SameLine();
    if (ImGui::Button(NF_COPY "  Sao chép kết quả", ImVec2(0.0f, Px(28.0f)))) {
        ImGui::SetClipboardText(dst_buf);
    }

    ImGui::SameLine();
    if (ImGui::Button(NF_TRASH "  Xóa trắng", ImVec2(0.0f, Px(28.0f)))) {
        src_buf[0] = '\0';
        dst_buf[0] = '\0';
    }
}

static ImVec4 HexToImVec4(const std::string& hex, const ImVec4& def_col) {
    if (hex.size() < 7 || hex[0] != '#') return def_col;
    unsigned int r = 255, g = 255, b = 255;
    if (sscanf_s(hex.c_str() + 1, "%02x%02x%02x", &r, &g, &b) == 3) {
        return ImVec4(r / 255.0f, g / 255.0f, b / 255.0f, 1.0f);
    }
    return def_col;
}

static std::string ImVec4ToHex(const ImVec4& col) {
    char buf[16];
    int r = (int)(col.x * 255.0f + 0.5f);
    int g = (int)(col.y * 255.0f + 0.5f);
    int b = (int)(col.z * 255.0f + 0.5f);
    r = std::clamp(r, 0, 255);
    g = std::clamp(g, 0, 255);
    b = std::clamp(b, 0, 255);
    snprintf(buf, sizeof(buf), "#%02X%02X%02X", r, g, b);
    return std::string(buf);
}

static void DrawTrayIconPreview(const char* letter, bool fill, const ImVec4& fill_col, bool border, const ImVec4& border_col) {
    ImVec2 p0 = ImGui::GetCursorScreenPos();
    float size = Px(68.0f);
    ImVec2 p1(p0.x + size, p0.y + size);
    ImDrawList* draw_list = ImGui::GetWindowDrawList();

    // Background gray box mimicking Windows taskbar
    draw_list->AddRectFilled(p0, p1, IM_COL32(48, 50, 56, 255), 6.0f);
    draw_list->AddRect(p0, p1, IM_COL32(80, 84, 94, 255), 6.0f, 0, 1.0f);

    // Large centered letter 'E' or 'V'
    ImFont* font = ImGui::GetFont();
    float font_size = Px(42.0f);
    ImVec2 text_size = font->CalcTextSizeA(font_size, FLT_MAX, 0.0f, letter);
    ImVec2 text_pos(p0.x + (size - text_size.x) * 0.5f, p0.y + (size - text_size.y) * 0.5f);

    ImU32 col_border = border ? ImGui::ColorConvertFloat4ToU32(border_col) : IM_COL32(0, 0, 0, 0);
    ImU32 col_fill = fill ? ImGui::ColorConvertFloat4ToU32(fill_col) : (border ? IM_COL32(48, 50, 56, 255) : IM_COL32(200, 200, 200, 255));

    if (border) {
        const int radius = (int)(Px(2.0f) + 0.5f);
        for (int dx = -radius; dx <= radius; ++dx) {
            for (int dy = -radius; dy <= radius; ++dy) {
                if (dx != 0 || dy != 0) {
                    draw_list->AddText(font, font_size, ImVec2(text_pos.x + dx, text_pos.y + dy), col_border, letter);
                }
            }
        }
    }

    draw_list->AddText(font, font_size, text_pos, col_fill, letter);
    ImGui::Dummy(ImVec2(size, size));
}

void UiManager::PageAdvanced() {
    SectionHeader("Tùy biến biểu tượng khay hệ thống (EVKey Style)");

    static ImVec4 e_fill_col = HexToImVec4(config_.tray_icon_e_fill_color, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
    static ImVec4 e_border_col = HexToImVec4(config_.tray_icon_e_border_color, ImVec4(0.0f, 0.47f, 0.84f, 1.0f));
    static ImVec4 v_fill_col = HexToImVec4(config_.tray_icon_v_fill_color, ImVec4(1.0f, 1.0f, 1.0f, 1.0f));
    static ImVec4 v_border_col = HexToImVec4(config_.tray_icon_v_border_color, ImVec4(1.0f, 0.0f, 0.5f, 1.0f));

    if (ImGui::BeginTable("EVKeyTrayGrid", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("ColE", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("ColV", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();

        // ----------------- Group E -----------------
        ImGui::TableSetColumnIndex(0);
        if (ImGui::BeginChild("GroupE", ImVec2(0.0f, Px(155.0f)), true, ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse)) {
            ImGui::TextUnformatted("Chế độ tiếng Anh [E]");
            ImGui::Separator();
            ImGui::Spacing();

            ImGui::Columns(2, "ECols", false);
            ImGui::SetColumnWidth(0, Px(180.0f));

            if (ImGui::Checkbox("Màu tô##E", &config_.tray_icon_e_fill)) SaveConfig();
            ImGui::SameLine(0.0f, Px(10.0f));
            if (ImGui::ColorEdit3("##EFillCol", (float*)&e_fill_col, ImGuiColorEditFlags_NoInputs | ImGuiColorEditFlags_NoLabel)) {
                config_.tray_icon_e_fill_color = ImVec4ToHex(e_fill_col);
                config_.tray_icon_theme = "custom";
                SaveConfig();
            }

            ImGui::Spacing();
            if (ImGui::Checkbox("Màu viền##E", &config_.tray_icon_e_border)) SaveConfig();
            ImGui::SameLine(0.0f, Px(10.0f));
            if (ImGui::ColorEdit3("##EBorderCol", (float*)&e_border_col, ImGuiColorEditFlags_NoInputs | ImGuiColorEditFlags_NoLabel)) {
                config_.tray_icon_e_border_color = ImVec4ToHex(e_border_col);
                config_.tray_icon_theme = "custom";
                SaveConfig();
            }

            ImGui::NextColumn();
            DrawTrayIconPreview("E", config_.tray_icon_e_fill, e_fill_col, config_.tray_icon_e_border, e_border_col);

            ImGui::Columns(1);
        }
        ImGui::EndChild();

        // ----------------- Group V -----------------
        ImGui::TableSetColumnIndex(1);
        if (ImGui::BeginChild("GroupV", ImVec2(0.0f, Px(155.0f)), true, ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse)) {
            ImGui::TextUnformatted("Chế độ tiếng Việt [V]");
            ImGui::Separator();
            ImGui::Spacing();

            ImGui::Columns(2, "VCols", false);
            ImGui::SetColumnWidth(0, Px(180.0f));

            if (ImGui::Checkbox("Màu tô##V", &config_.tray_icon_v_fill)) SaveConfig();
            ImGui::SameLine(0.0f, Px(10.0f));
            if (ImGui::ColorEdit3("##VFillCol", (float*)&v_fill_col, ImGuiColorEditFlags_NoInputs | ImGuiColorEditFlags_NoLabel)) {
                config_.tray_icon_v_fill_color = ImVec4ToHex(v_fill_col);
                config_.tray_icon_theme = "custom";
                SaveConfig();
            }

            ImGui::Spacing();
            if (ImGui::Checkbox("Màu viền##V", &config_.tray_icon_v_border)) SaveConfig();
            ImGui::SameLine(0.0f, Px(10.0f));
            if (ImGui::ColorEdit3("##VBorderCol", (float*)&v_border_col, ImGuiColorEditFlags_NoInputs | ImGuiColorEditFlags_NoLabel)) {
                config_.tray_icon_v_border_color = ImVec4ToHex(v_border_col);
                config_.tray_icon_theme = "custom";
                SaveConfig();
            }

            ImGui::NextColumn();
            DrawTrayIconPreview("V", config_.tray_icon_v_fill, v_fill_col, config_.tray_icon_v_border, v_border_col);

            ImGui::Columns(1);
        }
        ImGui::EndChild();

        ImGui::EndTable();
    }

    ImGui::Spacing();
    if (ImGui::Button(NF_SAVE " Lưu & Áp dụng icon khay", ImVec2(0.0f, Px(28.0f)))) {
        config_.tray_icon_theme = "custom";
        config_.tray_icon_e_fill_color = ImVec4ToHex(e_fill_col);
        config_.tray_icon_e_border_color = ImVec4ToHex(e_border_col);
        config_.tray_icon_v_fill_color = ImVec4ToHex(v_fill_col);
        config_.tray_icon_v_border_color = ImVec4ToHex(v_border_col);
        SaveConfig();
    }
    ImGui::SameLine(0.0f, Px(10.0f));
    if (ImGui::Button("Khôi phục mặc định", ImVec2(0.0f, Px(28.0f)))) {
        config_.tray_icon_theme = "modern";
        config_.tray_icon_e_fill = true;
        config_.tray_icon_e_fill_color = "#FFFFFF";
        config_.tray_icon_e_border = true;
        config_.tray_icon_e_border_color = "#0078D7";
        config_.tray_icon_v_fill = true;
        config_.tray_icon_v_fill_color = "#FFFFFF";
        config_.tray_icon_v_border = true;
        config_.tray_icon_v_border_color = "#FF007F";
        e_fill_col = ImVec4(1.0f, 1.0f, 1.0f, 1.0f);
        e_border_col = ImVec4(0.0f, 0.47f, 0.84f, 1.0f);
        v_fill_col = ImVec4(1.0f, 1.0f, 1.0f, 1.0f);
        v_border_col = ImVec4(1.0f, 0.0f, 0.5f, 1.0f);
        SaveConfig();
    }

    SectionHeader("Thời gian hoàn tất từ khi ngừng gõ");
    int timeout = (int)config_.typing_timeout_ms;
    ImGui::SetNextItemWidth(Px(260.0f));
    if (ImGui::SliderInt("##TimeoutSlider", &timeout, 500, 5000, "%d ms")) {
        config_.typing_timeout_ms = (uint64_t)timeout;
        SaveConfig();
    }
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.58f, 0.67f, 1.00f));
    ImGui::TextUnformatted("Thời gian chờ giữa 2 lần gõ phím liên tiếp để nhận diện hoàn tất một từ (mặc định: 1500 ms).");
    ImGui::PopStyleColor();

    ImGui::Dummy(ImVec2(0.0f, Px(20.0f)));
}

void UiManager::PageAbout() {
    SectionHeader("VietFlow");
    ImGui::TextUnformatted("Bộ gõ tiếng Việt thế hệ mới - Siêu nhẹ, Hiện đại");
    ImGui::TextUnformatted("Phiên bản: v" VIETFLOW_VERSION);
    ImGui::TextUnformatted("Bản quyền: Giấy phép mã nguồn mở GPLv3");

    ImGui::Spacing();
    ImGui::TextUnformatted("Trang chủ dự án:");
    ImGui::SameLine();
    if (ImGui::SmallButton("github.com/KienPC1234/VietFlow")) {
        ShellExecuteA(NULL, "open", "https://github.com/KienPC1234/VietFlow", NULL, NULL, SW_SHOWNORMAL);
    }

    ImGui::Spacing();
    const char* chk_label = NF_CONVERT " Kiểm tra bản cập nhật";
    float chk_w = ImGui::CalcTextSize(chk_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(chk_label, ImVec2(chk_w, Px(28.0f)))) {
        HWND hwndDaemon = FindWindowW(L"VietFlowMessageWindowClass", L"VietFlowDaemon");
        if (hwndDaemon) {
            PostMessageW(hwndDaemon, WM_COMMAND, 1070 /* IDM_CHECK_UPDATE */, 0);
        } else {
            ShellExecuteA(NULL, "open", "https://github.com/KienPC1234/VietFlow/releases", NULL, NULL, SW_SHOWNORMAL);
        }
    }

    ImGui::SameLine(0.0f, Px(12.0f));
    const char* dir_label = NF_FOLDER " Mở thư mục cấu hình";
    float dir_w = ImGui::CalcTextSize(dir_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(dir_label, ImVec2(dir_w, Px(28.0f)))) {
        std::string p = Config::GetDefaultConfigPath();
        std::filesystem::path fp(p);
        std::wstring parent_dir = fp.parent_path().wstring();
        ShellExecuteW(NULL, L"open", parent_dir.c_str(), NULL, NULL, SW_SHOWNORMAL);
    }

    ImGui::Dummy(ImVec2(0.0f, Px(20.0f)));
}

} // namespace vietflow
