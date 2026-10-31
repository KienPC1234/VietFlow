#include "ui_manager.h"
#include "constants.h"
#include "theme.h"
#include <imgui.h>
#include <windows.h>
#include <commdlg.h>
#include <shellapi.h>
#include <algorithm>
#include <cctype>
#include <cfloat>
#include <cstring>
#include <filesystem>
#include <fstream>
#include <map>

#ifndef VIETFLOW_VERSION
#define VIETFLOW_VERSION "0.1.0"
#endif

extern "C" void vf_play_notification_sound();
extern "C" bool vf_is_tsf_present();
extern "C" bool vf_is_tsf_registered();
extern "C" bool vf_register_tsf();
extern "C" bool vf_unregister_tsf();
extern "C" size_t vf_english_keywords(uint8_t* out, size_t out_cap);
extern "C" size_t vf_convert_charset(const unsigned char* input, size_t input_len,
                                     unsigned char source, unsigned char target,
                                     unsigned char* out, size_t out_cap);
extern "C" size_t vf_convert_charset_bytes(const unsigned char* input, size_t input_len,
                                           unsigned char source, unsigned char target,
                                           unsigned char* out, size_t out_cap);
extern "C" size_t vf_strip_accents(const unsigned char* input, size_t input_len,
                                   unsigned char* out, size_t out_cap);

namespace vietflow {

static std::vector<std::string> LoadDefaultEnglishKeywords() {
    const size_t needed = vf_english_keywords(nullptr, 0);
    if (needed == 0) return {};

    std::string buffer(needed, '\0');
    vf_english_keywords(reinterpret_cast<uint8_t*>(&buffer[0]), buffer.size());

    std::vector<std::string> words;
    size_t start = 0;
    while (start < buffer.size()) {
        size_t end = buffer.find('\n', start);
        if (end == std::string::npos) end = buffer.size();
        if (end > start) words.push_back(buffer.substr(start, end - start));
        start = end + 1;
    }
    std::sort(words.begin(), words.end());
    words.erase(std::unique(words.begin(), words.end()), words.end());
    return words;
}

static std::string TrimLower(const char* text) {
    if (!text) return {};
    std::string out;
    out.reserve(strlen(text));
    for (const char* p = text; *p; ++p) {
        unsigned char c = (unsigned char)*p;
        if (std::isspace(c)) continue;
        out.push_back((char)std::tolower(c));
    }
    return out;
}

static bool ContainsWord(const std::vector<std::string>& words, const std::string& word) {
    return std::find(words.begin(), words.end(), word) != words.end();
}

static void SortUnique(std::vector<std::string>& words) {
    std::sort(words.begin(), words.end());
    words.erase(std::unique(words.begin(), words.end()), words.end());
}

static std::string ConvertText(const std::string& input, int source, int target, bool strip) {
    if (input.empty()) return {};
    const auto* bytes = reinterpret_cast<const unsigned char*>(input.data());
    if (strip) {
        const size_t needed = vf_strip_accents(bytes, input.size(), nullptr, 0);
        if (needed == 0) return {};
        std::string out(needed, '\0');
        vf_strip_accents(bytes, input.size(), reinterpret_cast<unsigned char*>(&out[0]), out.size());
        return out;
    }
    const size_t needed = vf_convert_charset(
        bytes, input.size(), static_cast<unsigned char>(source),
        static_cast<unsigned char>(target), nullptr, 0);
    if (needed == 0) return {};
    std::string out(needed, '\0');
    vf_convert_charset(
        bytes, input.size(), static_cast<unsigned char>(source),
        static_cast<unsigned char>(target), reinterpret_cast<unsigned char*>(&out[0]), out.size());
    return out;
}

static std::string ConvertRaw(const std::string& input, int source, int target) {
    if (input.empty()) return {};
    const auto* bytes = reinterpret_cast<const unsigned char*>(input.data());
    const size_t needed = vf_convert_charset_bytes(
        bytes, input.size(), static_cast<unsigned char>(source),
        static_cast<unsigned char>(target), nullptr, 0);
    if (needed == 0) return {};
    std::string out(needed, '\0');
    vf_convert_charset_bytes(
        bytes, input.size(), static_cast<unsigned char>(source),
        static_cast<unsigned char>(target), reinterpret_cast<unsigned char*>(&out[0]), out.size());
    return out;
}

static std::string ReadClipboardAnsi() {
    if (!OpenClipboard(nullptr)) return {};
    std::string out;
    HANDLE handle = GetClipboardData(CF_TEXT);
    if (handle) {
        const char* ptr = static_cast<const char*>(GlobalLock(handle));
        if (ptr) {
            size_t len = 0;
            while (len < 1'048'576 && ptr[len] != '\0') ++len;
            out.assign(ptr, len);
            GlobalUnlock(handle);
        }
    }
    CloseClipboard();
    return out;
}

static bool WriteClipboardRaw(const std::string& raw_ansi, const std::string& unicode_preview) {
    if (!OpenClipboard(nullptr)) return false;
    EmptyClipboard();

    auto set_ansi = [&]() -> bool {
        HGLOBAL mem = GlobalAlloc(GMEM_MOVEABLE, raw_ansi.size() + 1);
        if (!mem) return false;
        char* ptr = static_cast<char*>(GlobalLock(mem));
        if (!ptr) { GlobalFree(mem); return false; }
        memcpy(ptr, raw_ansi.c_str(), raw_ansi.size() + 1);
        GlobalUnlock(mem);
        if (!SetClipboardData(CF_TEXT, mem)) { GlobalFree(mem); return false; }
        return true;
    };

    auto set_unicode = [&]() -> bool {
        int wlen = MultiByteToWideChar(CP_UTF8, 0, unicode_preview.c_str(), (int)unicode_preview.size(), nullptr, 0);
        if (wlen <= 0) return false;
        HGLOBAL mem = GlobalAlloc(GMEM_MOVEABLE, (wlen + 1) * sizeof(wchar_t));
        if (!mem) return false;
        wchar_t* ptr = static_cast<wchar_t*>(GlobalLock(mem));
        if (!ptr) { GlobalFree(mem); return false; }
        MultiByteToWideChar(CP_UTF8, 0, unicode_preview.c_str(), (int)unicode_preview.size(), ptr, wlen);
        ptr[wlen] = L'\0';
        GlobalUnlock(mem);
        if (!SetClipboardData(CF_UNICODETEXT, mem)) { GlobalFree(mem); return false; }
        return true;
    };

    bool ok = set_ansi();
    if (!unicode_preview.empty()) ok = set_unicode() || ok;
    CloseClipboard();
    return ok;
}

static bool CopyConvertedToClipboard(const std::string& utf8_input, int source, int target) {
    if (target >= 2) {
        std::string raw = ConvertRaw(utf8_input, source, target);
        if (raw.empty()) return false;
        std::string preview = ConvertRaw(raw, target, 0);
        return WriteClipboardRaw(raw, preview);
    }
    std::string result = ConvertText(utf8_input, source, target, false);
    if (result.empty()) return false;
    ImGui::SetClipboardText(result.c_str());
    return true;
}

static std::string ApplyCase(const std::string& text, bool upper, bool lower) {
    if (text.empty() || (!upper && !lower)) return text;
    int wlen = MultiByteToWideChar(CP_UTF8, 0, text.c_str(), (int)text.size(), nullptr, 0);
    if (wlen <= 0) return text;
    std::wstring wide((size_t)wlen, L'\0');
    MultiByteToWideChar(CP_UTF8, 0, text.c_str(), (int)text.size(), &wide[0], wlen);
    if (upper) CharUpperBuffW(&wide[0], (DWORD)wide.size());
    if (lower) CharLowerBuffW(&wide[0], (DWORD)wide.size());
    int len = WideCharToMultiByte(CP_UTF8, 0, wide.c_str(), (int)wide.size(), nullptr, 0, nullptr, nullptr);
    if (len <= 0) return text;
    std::string out((size_t)len, '\0');
    WideCharToMultiByte(CP_UTF8, 0, wide.c_str(), (int)wide.size(), &out[0], len, nullptr, nullptr);
    return out;
}

static std::string TrimString(const std::string& text) {
    size_t start = 0;
    while (start < text.size() && std::isspace((unsigned char)text[start])) ++start;
    size_t end = text.size();
    while (end > start && std::isspace((unsigned char)text[end - 1])) --end;
    return text.substr(start, end - start);
}

static bool ParseMacroLine(const std::string& raw, std::string& key, std::string& value) {
    std::string line = TrimString(raw);
    if (line.empty() || line[0] == '#') return false;
    size_t split = line.find("||");
    if (split == std::string::npos) split = line.find('\t');
    if (split == std::string::npos) split = line.find('=');
    if (split == std::string::npos) split = line.find(':');
    if (split == std::string::npos) split = line.find(',');
    if (split == std::string::npos) return false;
    key = TrimString(line.substr(0, split));
    value = TrimString(line.substr(split + (line.compare(split, 2, "||") == 0 ? 2 : 1)));
    return !key.empty() && !value.empty();
}

static std::string PickMacroFile(bool save) {
    OPENFILENAMEW ofn = { sizeof(ofn) };
    wchar_t file[MAX_PATH] = { 0 };
    if (save) wcscpy_s(file, L"vietflow-macros.tsv");
    ofn.lpstrFile = file;
    ofn.nMaxFile = MAX_PATH;
    ofn.lpstrFilter = L"Danh sách gõ tắt (*.tsv;*.txt;*.csv)\0*.tsv;*.txt;*.csv\0Tất cả tập tin (*.*)\0*.*\0";
    ofn.nFilterIndex = 1;
    ofn.Flags = OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR |
                (save ? OFN_OVERWRITEPROMPT : OFN_FILEMUSTEXIST);
    const BOOL ok = save ? GetSaveFileNameW(&ofn) : GetOpenFileNameW(&ofn);
    if (!ok) return {};
    int len = WideCharToMultiByte(CP_UTF8, 0, file, -1, nullptr, 0, nullptr, nullptr);
    if (len <= 1) return {};
    std::string path((size_t)len - 1, '\0');
    WideCharToMultiByte(CP_UTF8, 0, file, -1, &path[0], len, nullptr, nullptr);
    return path;
}

static size_t ImportMacros(const std::string& path, std::map<std::string, std::string>& macros) {
    std::ifstream file(path, std::ios::binary);
    if (!file.is_open()) return 0;
    size_t imported = 0;
    std::string line;
    while (std::getline(file, line)) {
        std::string key, value;
        if (!ParseMacroLine(line, key, value)) continue;
        macros[key] = value;
        ++imported;
    }
    return imported;
}

static size_t ExportMacros(const std::string& path, const std::map<std::string, std::string>& macros) {
    std::ofstream file(path, std::ios::binary);
    if (!file.is_open()) return 0;
    for (const auto& [key, value] : macros) {
        file << key << '\t' << value << '\n';
    }
    return macros.size();
}

// Returns 0 while waiting, 1 when a combination was captured, 2 on Esc.
static int PollHotkey(std::string& out) {
    if (GetAsyncKeyState(VK_ESCAPE) & 0x8000) return 2;

    const bool ctrl = (GetAsyncKeyState(VK_CONTROL) & 0x8000) != 0;
    const bool alt = (GetAsyncKeyState(VK_MENU) & 0x8000) != 0;
    const bool shift = (GetAsyncKeyState(VK_SHIFT) & 0x8000) != 0;
    const bool win = ((GetAsyncKeyState(VK_LWIN) & 0x8000) != 0) ||
                     ((GetAsyncKeyState(VK_RWIN) & 0x8000) != 0);

    struct KeyDef { int vk; const char* name; };
    static const KeyDef keys[] = {
        { VK_SPACE, "Space" }, { VK_CAPITAL, "CapsLock" }, { VK_TAB, "Tab" },
        { 0xC0, "~" },
        { VK_F1, "F1" }, { VK_F2, "F2" }, { VK_F3, "F3" }, { VK_F4, "F4" },
        { VK_F5, "F5" }, { VK_F6, "F6" }, { VK_F7, "F7" }, { VK_F8, "F8" },
        { VK_F9, "F9" }, { VK_F10, "F10" }, { VK_F11, "F11" }, { VK_F12, "F12" },
    };

    std::string key;
    for (const auto& def : keys) {
        if ((GetAsyncKeyState(def.vk) & 0x8000) != 0) {
            key = def.name;
            break;
        }
    }
    if (key.empty()) {
        for (int k = 'A'; k <= 'Z'; ++k) {
            if ((GetAsyncKeyState(k) & 0x8000) != 0) {
                key = std::string(1, (char)k);
                break;
            }
        }
    }
    if (key.empty()) {
        for (int k = '0'; k <= '9'; ++k) {
            if ((GetAsyncKeyState(k) & 0x8000) != 0) {
                key = std::string(1, (char)k);
                break;
            }
        }
    }
    if (key.empty()) return 0;

    out.clear();
    if (ctrl) out += "Ctrl + ";
    if (alt) out += "Alt + ";
    if (shift) out += "Shift + ";
    if (win) out += "Win + ";
    out += key;
    return 1;
}

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

                if (exe != "explorer.exe" && exe != "taskmgr.exe") {
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

static bool CheckIsProcessElevated() {
    BOOL elevated = FALSE;
    HANDLE hToken = NULL;
    if (OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &hToken)) {
        TOKEN_ELEVATION elevation;
        DWORD cbSize = sizeof(TOKEN_ELEVATION);
        if (GetTokenInformation(hToken, TokenElevation, &elevation, sizeof(elevation), &cbSize)) {
            elevated = (elevation.TokenIsElevated != 0);
        }
        CloseHandle(hToken);
    }
    return elevated != FALSE;
}

UiManager::UiManager(const std::string& config_path, bool convert_mode, void* hwnd)
    : config_path_(config_path.empty() ? Config::GetDefaultConfigPath() : config_path),
      hwnd_(hwnd),
      is_running_as_admin_(CheckIsProcessElevated()) {
    config_.Load(config_path_);
    if (convert_mode) {
        initial_page_ = Page::Converter;
    }
    strncpy_s(foreign_initials_buf_, config_.foreign_initials.c_str(), sizeof(foreign_initials_buf_) - 1);
    strncpy_s(hotkey_settings_buf_, config_.hotkey_settings.c_str(), sizeof(hotkey_settings_buf_) - 1);
    strncpy_s(hotkey_converter_buf_, config_.hotkey_converter.c_str(), sizeof(hotkey_converter_buf_) - 1);
    strncpy_s(auto_capitalize_buf_, config_.auto_capitalize_chars.c_str(), sizeof(auto_capitalize_buf_) - 1);
    english_defaults_ = LoadDefaultEnglishKeywords();
    SyncCustomKeymapBuffersFromConfig();
}

void UiManager::SyncCustomKeymapBuffersFromConfig() {
    auto copy_str = [](char* dst, size_t dst_size, const std::string& src) {
        strncpy_s(dst, dst_size, src.c_str(), dst_size - 1);
    };
    copy_str(custom_acute_, sizeof(custom_acute_), config_.custom_keymap.acute);
    copy_str(custom_grave_, sizeof(custom_grave_), config_.custom_keymap.grave);
    copy_str(custom_hook_, sizeof(custom_hook_), config_.custom_keymap.hook);
    copy_str(custom_tilde_, sizeof(custom_tilde_), config_.custom_keymap.tilde);
    copy_str(custom_dot_, sizeof(custom_dot_), config_.custom_keymap.dot);
    copy_str(custom_remove_tone_, sizeof(custom_remove_tone_), config_.custom_keymap.remove_tone);

    copy_str(custom_circumflex_, sizeof(custom_circumflex_), config_.custom_keymap.circumflex);
    copy_str(custom_horn_, sizeof(custom_horn_), config_.custom_keymap.horn);
    copy_str(custom_breve_, sizeof(custom_breve_), config_.custom_keymap.breve);
    copy_str(custom_d_mark_, sizeof(custom_d_mark_), config_.custom_keymap.d_mark);
    copy_str(custom_hook_all_, sizeof(custom_hook_all_), config_.custom_keymap.hook_all);

    copy_str(custom_roof_a_, sizeof(custom_roof_a_), config_.custom_keymap.roof_a);
    copy_str(custom_roof_e_, sizeof(custom_roof_e_), config_.custom_keymap.roof_e);
    copy_str(custom_roof_o_, sizeof(custom_roof_o_), config_.custom_keymap.roof_o);
    copy_str(custom_horn_o_, sizeof(custom_horn_o_), config_.custom_keymap.horn_o);
    copy_str(custom_horn_u_, sizeof(custom_horn_u_), config_.custom_keymap.horn_u);
    copy_str(custom_breve_a_, sizeof(custom_breve_a_), config_.custom_keymap.breve_a);
    copy_str(custom_d_lower_, sizeof(custom_d_lower_), config_.custom_keymap.d_lower);
}

void UiManager::SyncCustomKeymapBuffersToConfig() {
    config_.custom_keymap.acute = custom_acute_;
    config_.custom_keymap.grave = custom_grave_;
    config_.custom_keymap.hook = custom_hook_;
    config_.custom_keymap.tilde = custom_tilde_;
    config_.custom_keymap.dot = custom_dot_;
    config_.custom_keymap.remove_tone = custom_remove_tone_;

    config_.custom_keymap.circumflex = custom_circumflex_;
    config_.custom_keymap.horn = custom_horn_;
    config_.custom_keymap.breve = custom_breve_;
    config_.custom_keymap.d_mark = custom_d_mark_;
    config_.custom_keymap.hook_all = custom_hook_all_;

    config_.custom_keymap.roof_a = custom_roof_a_;
    config_.custom_keymap.roof_e = custom_roof_e_;
    config_.custom_keymap.roof_o = custom_roof_o_;
    config_.custom_keymap.horn_o = custom_horn_o_;
    config_.custom_keymap.horn_u = custom_horn_u_;
    config_.custom_keymap.breve_a = custom_breve_a_;
    config_.custom_keymap.d_lower = custom_d_lower_;
}

void UiManager::LoadCustomKeymapPreset(int preset_id) {
    if (preset_id == 0) { // Telex
        config_.custom_keymap.acute = "s";
        config_.custom_keymap.grave = "f";
        config_.custom_keymap.hook = "r";
        config_.custom_keymap.tilde = "x";
        config_.custom_keymap.dot = "j";
        config_.custom_keymap.remove_tone = "z";
        config_.custom_keymap.circumflex = "";
        config_.custom_keymap.horn = "";
        config_.custom_keymap.breve = "";
        config_.custom_keymap.d_mark = "";
        config_.custom_keymap.hook_all = "w";
        config_.custom_keymap.roof_a = "";
        config_.custom_keymap.roof_e = "";
        config_.custom_keymap.roof_o = "";
        config_.custom_keymap.horn_o = "";
        config_.custom_keymap.horn_u = "";
        config_.custom_keymap.breve_a = "";
        config_.custom_keymap.d_lower = "";
        config_.custom_keymap.telex_double_keys = true;
        config_.custom_keymap.lone_w = true;
    } else if (preset_id == 1) { // VNI
        config_.custom_keymap.acute = "1";
        config_.custom_keymap.grave = "2";
        config_.custom_keymap.hook = "3";
        config_.custom_keymap.tilde = "4";
        config_.custom_keymap.dot = "5";
        config_.custom_keymap.remove_tone = "0";
        config_.custom_keymap.circumflex = "6";
        config_.custom_keymap.horn = "7";
        config_.custom_keymap.breve = "8";
        config_.custom_keymap.d_mark = "9";
        config_.custom_keymap.hook_all = "";
        config_.custom_keymap.roof_a = "";
        config_.custom_keymap.roof_e = "";
        config_.custom_keymap.roof_o = "";
        config_.custom_keymap.horn_o = "";
        config_.custom_keymap.horn_u = "";
        config_.custom_keymap.breve_a = "";
        config_.custom_keymap.d_lower = "";
        config_.custom_keymap.telex_double_keys = false;
        config_.custom_keymap.lone_w = false;
    } else if (preset_id == 2) { // VIQR
        config_.custom_keymap.acute = "'";
        config_.custom_keymap.grave = "`";
        config_.custom_keymap.hook = "?";
        config_.custom_keymap.tilde = "~";
        config_.custom_keymap.dot = ".";
        config_.custom_keymap.remove_tone = "0";
        config_.custom_keymap.circumflex = "^";
        config_.custom_keymap.horn = "+*";
        config_.custom_keymap.breve = "(";
        config_.custom_keymap.d_mark = "d";
        config_.custom_keymap.hook_all = "";
        config_.custom_keymap.roof_a = "";
        config_.custom_keymap.roof_e = "";
        config_.custom_keymap.roof_o = "";
        config_.custom_keymap.horn_o = "";
        config_.custom_keymap.horn_u = "";
        config_.custom_keymap.breve_a = "";
        config_.custom_keymap.d_lower = "";
        config_.custom_keymap.telex_double_keys = false;
        config_.custom_keymap.lone_w = false;
    } else { // Clear all
        config_.custom_keymap.acute = "";
        config_.custom_keymap.grave = "";
        config_.custom_keymap.hook = "";
        config_.custom_keymap.tilde = "";
        config_.custom_keymap.dot = "";
        config_.custom_keymap.remove_tone = "";
        config_.custom_keymap.circumflex = "";
        config_.custom_keymap.horn = "";
        config_.custom_keymap.breve = "";
        config_.custom_keymap.d_mark = "";
        config_.custom_keymap.hook_all = "";
        config_.custom_keymap.roof_a = "";
        config_.custom_keymap.roof_e = "";
        config_.custom_keymap.roof_o = "";
        config_.custom_keymap.horn_o = "";
        config_.custom_keymap.horn_u = "";
        config_.custom_keymap.breve_a = "";
        config_.custom_keymap.d_lower = "";
        config_.custom_keymap.telex_double_keys = false;
        config_.custom_keymap.lone_w = false;
    }
    SyncCustomKeymapBuffersFromConfig();
}

void UiManager::SaveConfig() {
    SyncCustomKeymapBuffersToConfig();
    config_.foreign_initials = foreign_initials_buf_;
    config_.auto_capitalize_chars = auto_capitalize_buf_;
    config_.hotkey_settings = hotkey_settings_buf_;
    config_.hotkey_converter = hotkey_converter_buf_;
    config_.Save(config_path_);
    dirty_ = false;
    saved_toast_timer_ = 2.0f;
    if (on_saved_cb_) {
        on_saved_cb_();
    }
    // Notify running daemon
    HWND hwnd = FindWindowW(WINDOW_CLASS_DAEMON, WINDOW_TITLE_DAEMON);
    if (hwnd) {
        PostMessageW(hwnd, WM_VF_RELOAD_CONFIG, 0, 0);
    }
}

void UiManager::ResetToDefaults() {
    config_ = Config();
    strncpy_s(foreign_initials_buf_, config_.foreign_initials.c_str(), sizeof(foreign_initials_buf_) - 1);
    strncpy_s(hotkey_settings_buf_, config_.hotkey_settings.c_str(), sizeof(hotkey_settings_buf_) - 1);
    strncpy_s(hotkey_converter_buf_, config_.hotkey_converter.c_str(), sizeof(hotkey_converter_buf_) - 1);
    strncpy_s(auto_capitalize_buf_, config_.auto_capitalize_chars.c_str(), sizeof(auto_capitalize_buf_) - 1);
    SyncCustomKeymapBuffersFromConfig();
    SaveConfig();
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
                                   ImGuiWindowFlags_NoResize | ImGuiWindowFlags_NoSavedSettings |
                                   ImGuiWindowFlags_NoScrollbar | ImGuiWindowFlags_NoScrollWithMouse;

    ImGui::PushStyleVar(ImGuiStyleVar_WindowPadding, ImVec2(Px(16.0f), Px(12.0f)));
    ImGui::Begin("VietFlowMainWindow", nullptr, window_flags);
    ImGui::PopStyleVar();

    // Top Header: Title + Status + Action buttons
    // Global ESC key to save & dismiss window (when not actively capturing hotkeys or inside modal popups)
    if (ImGui::IsKeyPressed(ImGuiKey_Escape)) {
        if (!recording_hotkey_ && recording_hotkey_target_ == 0 && !ImGui::IsPopupOpen(nullptr, ImGuiPopupFlags_AnyPopupId)) {
            SaveConfig();
            if (hwnd_) {
                PostMessageW((HWND)hwnd_, WM_CLOSE, 0, 0);
            }
        }
    }

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted(NF_KEYBOARD " VietFlow");
    ImGui::SameLine();
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.60f, 0.70f, 1.0f));
    ImGui::TextUnformatted("- Bộ gõ tiếng Việt thế hệ mới");
    ImGui::PopStyleColor();

    const char* save_btn_label = NF_SAVE " Lưu & Đóng (Esc)";
    float save_btn_w = ImGui::CalcTextSize(save_btn_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(16.0f);

    const char* reset_btn_label = NF_CONVERT " Khôi phục cài đặt gốc";
    float reset_btn_w = ImGui::CalcTextSize(reset_btn_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(16.0f);

    float spacing = Px(8.0f);
    float total_btns_w = save_btn_w + spacing + reset_btn_w;
    float start_x = ImGui::GetWindowWidth() - total_btns_w - ImGui::GetStyle().WindowPadding.x;

    if (saved_toast_timer_ > 0.0f) {
        const char* saved_label = NF_CHECK " Đã lưu cài đặt";
        float saved_w = ImGui::CalcTextSize(saved_label).x + Px(12.0f);
        ImGui::SameLine(start_x - saved_w - Px(12.0f));
        ImGui::TextColored(ImVec4(0.30f, 0.85f, 0.45f, 1.00f), "%s", saved_label);
        ImGui::SameLine(start_x);
    } else {
        ImGui::SameLine(start_x);
    }

    // Reset button on the LEFT of Save & Apply
    ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.18f, 0.20f, 0.25f, 1.0f));
    ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.28f, 0.32f, 0.40f, 1.0f));
    if (ImGui::Button(reset_btn_label, ImVec2(reset_btn_w, Px(28.0f)))) {
        ResetToDefaults();
    }
    ImGui::PopStyleColor(2);

    ImGui::SameLine(0.0f, spacing);

    // Save & Apply button: saves configuration and dismisses UI window automatically
    if (ImGui::Button(save_btn_label, ImVec2(save_btn_w, Px(28.0f)))) {
        SaveConfig();
        if (hwnd_) {
            PostMessageW((HWND)hwnd_, WM_CLOSE, 0, 0);
        }
    }

    ImGui::Spacing();

    // Native ImGui TabBar Navigation (No forced sidebar)
    static bool first_run = true;
    ImGuiTabBarFlags tab_bar_flags = ImGuiTabBarFlags_FittingPolicyScroll;
    if (ImGui::BeginTabBar("MainTabs", tab_bar_flags)) {
        if (ImGui::BeginTabItem(NF_COG " Tổng quan")) {
            BeginPageScroll();
            PageOverview();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_KEYBOARD " Gõ")) {
            BeginPageScroll();
            PageTyping();
            EndPageScroll();
            ImGui::EndTabItem();
        }

        if (ImGui::BeginTabItem(NF_BOLT " Gõ tắt")) {
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
        if (config_.notification_sound) {
            vf_play_notification_sound();
        }
    }
    ImGui::PopStyleColor();

    // Administrator elevation status badge and 1-click relaunch button
    ImGui::SameLine(0.0f, Px(16.0f));
    if (is_running_as_admin_) {
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.15f, 0.42f, 0.25f, 1.0f));
        ImGui::Button("[Quyền Administrator]", ImVec2(0.0f, Px(28.0f)));
        ImGui::PopStyleColor();
        if (ImGui::IsItemHovered()) {
            ImGui::SetTooltip("Đang chạy với quyền Admin; gõ được trong cửa sổ admin và game.");
        }
    } else {
        ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.50f, 0.32f, 0.10f, 1.0f));
        ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.68f, 0.42f, 0.12f, 1.0f));
        if (ImGui::Button(NF_BOLT " Chạy quyền Administrator", ImVec2(0.0f, Px(28.0f)))) {
            HWND hwndDaemon = FindWindowW(WINDOW_CLASS_DAEMON, WINDOW_TITLE_DAEMON);
            if (hwndDaemon) {
                PostMessageW(hwndDaemon, WM_COMMAND, IDM_VF_RESTART_ADMIN, 0);
                if (hwnd_) {
                    PostMessageW((HWND)hwnd_, WM_CLOSE, 0, 0);
                }
            }
        }
        ImGui::PopStyleColor(2);
        if (ImGui::IsItemHovered()) {
            ImGui::SetTooltip("Chạy lại với quyền Admin để gõ trong app admin và game.");
        }
    }

    ImGui::Spacing();

    if (ImGui::BeginTable("OverviewSelectors", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("Kiểu gõ", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("Bảng mã", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TextUnformatted("Kiểu gõ chính:");
        ImGui::SetNextItemWidth(-Px(10.0f));
        const char* methods[] = {
            "Telex (Chuẩn)", "VNI", "VIQR", "Simple Telex", "Simple Telex 2", "Telex + VNI (Hỗn hợp)", "Tự định nghĩa"
        };
        if (ImGui::Combo("##MethodCombo", &config_.method, methods, IM_ARRAYSIZE(methods))) {
            SaveConfig();
        }
        if (config_.method == 6) {
            ImGui::Spacing();
            ImGui::TextColored(ImVec4(0.35f, 0.70f, 1.0f, 1.0f), "Đang dùng kiểu gõ Tự định nghĩa.");
            ImGui::TextDisabled("Vào tab [Gõ] để tùy chỉnh chi tiết từng phím gõ.");
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

    SectionHeader("Phím tắt V/E");

    // Current Hotkey status badge
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Phím tắt đang dùng:");
    ImGui::SameLine(0.0f, Px(10.0f));
    HotkeyBadge(FormatHotkeyLabel(config_.hotkey_mode).c_str());

    ImGui::Spacing();
    ImGui::TextDisabled("Chọn nhanh:");
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
            float next_right = (ImGui::GetItemRectMax().x - ImGui::GetWindowPos().x)
                             + Px(8.0f) + next_btn_w;
            if (next_right <= ImGui::GetWindowContentRegionMax().x) {
                ImGui::SameLine(0.0f, Px(8.0f));
            }
        }
    }

    ImGui::Spacing();
    ImGui::Spacing();
    ImGui::TextDisabled("Tự gán:");
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

    SectionHeader("Phím tắt khác");
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Mở bảng điều khiển:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(210.0f));
    if (ImGui::InputText("##HotkeySettings", hotkey_settings_buf_, sizeof(hotkey_settings_buf_))) {
        SaveConfig();
    }
    ImGui::SameLine();
    if (ImGui::Button(recording_hotkey_target_ == 1 ? NF_BOLT " Đang nghe..." : NF_COG " Ghi",
                      ImVec2(Px(110.0f), Px(26.0f)))) {
        recording_hotkey_target_ = 1;
    }

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Mở công cụ chuyển mã:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(210.0f));
    if (ImGui::InputText("##HotkeyConverter", hotkey_converter_buf_, sizeof(hotkey_converter_buf_))) {
        SaveConfig();
    }
    ImGui::SameLine();
    if (ImGui::Button(recording_hotkey_target_ == 2 ? NF_BOLT " Đang nghe..." : NF_COG " Ghi",
                      ImVec2(Px(110.0f), Px(26.0f)))) {
        recording_hotkey_target_ = 2;
    }
    ImGui::TextDisabled("Bấm Ghi, nhấn tổ hợp (Esc hủy) hoặc gõ tay.");

    if (recording_hotkey_target_ != 0) {
        std::string captured;
        const int result = PollHotkey(captured);
        if (result == 2) {
            recording_hotkey_target_ = 0;
        } else if (result == 1) {
            if (recording_hotkey_target_ == 1) {
                config_.hotkey_settings = captured;
                strncpy_s(hotkey_settings_buf_, captured.c_str(), sizeof(hotkey_settings_buf_) - 1);
            } else {
                config_.hotkey_converter = captured;
                strncpy_s(hotkey_converter_buf_, captured.c_str(), sizeof(hotkey_converter_buf_) - 1);
            }
            recording_hotkey_target_ = 0;
            SaveConfig();
        } else {
            ImGui::TextColored(ImVec4(0.95f, 0.55f, 0.30f, 1.0f),
                               "Đang nghe... nhấn tổ hợp phím (Esc để hủy)");
        }
    }

    SectionHeader("Hệ thống");
    if (ImGui::Checkbox("Khởi động cùng Windows khi bật máy", &config_.start_with_windows)) SaveConfig();
    ImGui::Spacing();
    if (ImGui::Checkbox("Phát âm thanh thông báo nhẹ khi chuyển chế độ V/E", &config_.notification_sound)) {
        SaveConfig();
        if (config_.notification_sound) {
            vf_play_notification_sound();
        }
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Phát âm thanh khi chuyển V/E bằng phím tắt hoặc biểu tượng khay. Mặc định tắt.");
    }
}

void UiManager::PageTyping() {
    SectionHeader("Bỏ dấu tiếng Việt");
    if (ImGui::Checkbox("Bỏ dấu tự do", &config_.free_marking)) SaveConfig();
    if (ImGui::Checkbox("Dấu thanh kiểu mới (oà, uỳ)", &config_.modern_style)) SaveConfig();
    if (ImGui::Checkbox("Kiểm tra chính tả", &config_.spell_check)) SaveConfig();
    if (ImGui::Checkbox("Tự khôi phục từ gõ sai / tiếng Anh", &config_.auto_restore)) SaveConfig();

    SectionHeader("Bàn phím");
    if (ImGui::Checkbox("Chống dính Shift", &config_.fix_shift_lag)) SaveConfig();
    if (ImGui::Checkbox("Gõ tắt phụ âm đầu", &config_.consonant_shortcuts)) SaveConfig();
    if (ImGui::Checkbox("Viết hoa đầu câu", &config_.auto_capitalize)) SaveConfig();
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(150.0f));
    if (ImGui::InputTextWithHint("##AutoCapChars", ".!?", auto_capitalize_buf_, sizeof(auto_capitalize_buf_))) {
        config_.auto_capitalize_chars = auto_capitalize_buf_;
        SaveConfig();
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Ký tự kết thúc câu (mặc định . ! ? xuống dòng).");
    }
    if (ImGui::Checkbox("Tắt khi bàn phím không phải US", &config_.disable_on_non_us_layout)) SaveConfig();
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Tránh gõ sai khi dùng layout khác.");
    }
    if (ImGui::Checkbox("Tắt trong cửa sổ không hỗ trợ Unicode", &config_.disable_on_non_unicode_window)) {
        SaveConfig();
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Ví dụ VBA IDE. Bỏ chọn nếu app vẫn nhận tiếng Việt qua clipboard.");
    }

    SectionHeader("Gửi phím");
    static const char* injection_modes[] = {
        "Tự động (khuyến nghị)", "Unicode", "Scan-code (game DirectInput)",
        "An toàn (chậm, chống mất ký tự)"
    };
    static const int injection_values[] = {0, 1, 2, 4};
    int injection_index = 0;
    for (int i = 0; i < IM_ARRAYSIZE(injection_values); ++i) {
        if (injection_values[i] == config_.injection_mode) {
            injection_index = i;
            break;
        }
    }
    ImGui::BeginDisabled(config_.use_tsf);
    ImGui::SetNextItemWidth(Px(340.0f));
    if (ImGui::Combo("##InjectionMode", &injection_index, injection_modes, IM_ARRAYSIZE(injection_modes))) {
        config_.injection_mode = injection_values[injection_index];
        SaveConfig();
    }
    if (config_.use_tsf) {
        ImGui::TextColored(ImVec4(0.35f, 0.70f, 1.0f, 1.0f), "Chế độ TSF IME đang bật: Soạn thảo văn bản trực tiếp in-process (không dùng cơ chế gửi phím SendInput).");
    } else {
        ImGui::TextDisabled("Tự động: Unicode (SendInput nhanh), tự động fallback gửi chậm nếu ứng dụng phản hồi trễ.");
    }
    ImGui::EndDisabled();

    SectionHeader("Chế độ TSF IME (Text Services Framework)");
    if (vf_is_tsf_present()) {
        bool tsf_registered = vf_is_tsf_registered();
        if (ImGui::Checkbox("Sử dụng chế độ gõ TSF", &config_.use_tsf)) {
            if (config_.use_tsf && !tsf_registered) {
                if (vf_register_tsf()) {
                    tsf_registered = true;
                }
            }
            SaveConfig();
        }
        if (ImGui::IsItemHovered()) {
            ImGui::SetTooltip("Xử lý gõ trực tiếp qua Text Services Framework in-process bên trong ứng dụng đích (Word, trình duyệt, Notepad), không cần gửi phím xóa backspace.");
        }

        ImGui::SameLine(0.0f, Px(16.0f));
        const char* reg_btn_text = tsf_registered ? "Hủy đăng ký TSF TIP" : "Cài đặt / Đăng ký TSF TIP";
        if (ImGui::Button(reg_btn_text, ImVec2(0.0f, Px(26.0f)))) {
            if (tsf_registered) {
                vf_unregister_tsf();
            } else {
                vf_register_tsf();
            }
        }
        if (ImGui::IsItemHovered()) {
            ImGui::SetTooltip("Đăng ký hoặc gỡ bỏ VietFlow TIP COM Server vào hệ thống Windows.");
        }

        if (tsf_registered) {
            ImGui::TextColored(ImVec4(0.25f, 0.78f, 0.35f, 1.0f), "Trạng thái: Đã đăng ký TSF TIP trong Windows");
        } else {
            ImGui::TextColored(ImVec4(0.85f, 0.60f, 0.20f, 1.0f), "Trạng thái: Chưa đăng ký TSF TIP trong Windows (Bấm nút trên để cài đặt)");
        }
    } else {
        ImGui::TextDisabled("Không tìm thấy vietflow_tip.dll (Chế độ TSF không khả dụng).");
    }

    SectionHeader("Ngoại lai & tiếng Anh");
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Phụ âm ngoại lai:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(120.0f));
    if (ImGui::InputText("##ForeignInitials", foreign_initials_buf_, sizeof(foreign_initials_buf_))) {
        SaveConfig();
    }
    ImGui::TextDisabled("Mặc định fjwz; bỏ dấu khi gặp ở đầu từ.");

    ImGui::Spacing();
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Từ tiếng Anh không bỏ dấu:");
    ImGui::SameLine();
    if (ImGui::Button(NF_COG "  Chỉnh sửa danh sách...", ImVec2(0.0f, Px(26.0f)))) {
        ImGui::OpenPopup("Từ tiếng Anh");
    }
    ImGui::TextDisabled("Mặc định + từ bạn thêm; popup để thêm/xóa.");

    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(Px(640.0f), Px(500.0f)), ImGuiCond_Appearing);
    if (ImGui::BeginPopupModal("Từ tiếng Anh", nullptr, ImGuiWindowFlags_NoResize)) {
        ImGui::TextDisabled("Giữ nguyên, không bỏ dấu.");
        ImGui::SetNextItemWidth(-Px(10.0f));
        ImGui::InputTextWithHint("##EngFilter", "Tìm từ...", english_keyword_filter_, sizeof(english_keyword_filter_));
        ImGui::Spacing();

        std::vector<std::string> words = english_defaults_;
        for (const auto& raw : config_.english_keywords) {
            std::string word = TrimLower(raw.c_str());
            if (!word.empty() && !ContainsWord(words, word)) words.push_back(word);
        }
        words.erase(std::remove_if(words.begin(), words.end(), [&](const std::string& word) {
            return ContainsWord(config_.english_keywords_removed, word);
        }), words.end());
        SortUnique(words);

        const std::string filter = TrimLower(english_keyword_filter_);
        if (ImGui::BeginChild("##EngList", ImVec2(0.0f, Px(300.0f)), true)) {
            int shown = 0;
            for (const auto& word : words) {
                if (!filter.empty() && word.find(filter) == std::string::npos) continue;
                ++shown;
                ImGui::PushID(word.c_str());
                ImGui::TextUnformatted(word.c_str());
                ImGui::SameLine(ImGui::GetContentRegionAvail().x - Px(60.0f));
                if (ImGui::SmallButton(NF_TRASH " Xóa")) {
                    config_.english_keywords.erase(
                        std::remove(config_.english_keywords.begin(), config_.english_keywords.end(), word),
                        config_.english_keywords.end());
                    if (ContainsWord(english_defaults_, word) &&
                        !ContainsWord(config_.english_keywords_removed, word)) {
                        config_.english_keywords_removed.push_back(word);
                        SortUnique(config_.english_keywords_removed);
                    }
                    SaveConfig();
                }
                ImGui::PopID();
            }
            if (shown == 0) {
                ImGui::TextDisabled("Không có từ phù hợp.");
            }
        }
        ImGui::EndChild();

        ImGui::SetNextItemWidth(-Px(100.0f));
        ImGui::InputTextWithHint("##EngNew", "Thêm từ tiếng Anh...", new_english_kw_, sizeof(new_english_kw_));
        ImGui::SameLine();
        if (ImGui::Button(NF_PLUS "  Thêm", ImVec2(Px(90.0f), Px(26.0f)))) {
            std::string word = TrimLower(new_english_kw_);
            if (!word.empty()) {
                config_.english_keywords_removed.erase(
                    std::remove(config_.english_keywords_removed.begin(), config_.english_keywords_removed.end(), word),
                    config_.english_keywords_removed.end());
                if (!ContainsWord(english_defaults_, word) &&
                    !ContainsWord(config_.english_keywords, word)) {
                    config_.english_keywords.push_back(word);
                    SortUnique(config_.english_keywords);
                }
                new_english_kw_[0] = '\0';
                SaveConfig();
            }
        }

        if (ImGui::Button("Khôi phục từ mặc định", ImVec2(0.0f, Px(26.0f)))) {
            config_.english_keywords_removed.clear();
            SaveConfig();
        }
        ImGui::SameLine();
        if (ImGui::Button("Đóng", ImVec2(Px(90.0f), Px(26.0f)))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    bool is_custom_active = (config_.method == 6);
    std::string header_title = std::string(NF_KEYBOARD " Kiểu gõ tự định nghĩa (Custom Keymap) ") +
        (is_custom_active ? "[Đang kích hoạt]" : "[Bấm để mở rộng tùy biến]");

    ImGui::Spacing();
    ImGui::AlignTextToFramePadding();
    ImGuiTreeNodeFlags custom_header_flags =
        ImGuiTreeNodeFlags_Framed | ImGuiTreeNodeFlags_SpanAvailWidth;
    if (is_custom_active) custom_header_flags |= ImGuiTreeNodeFlags_DefaultOpen;
    ImGui::PushStyleVar(ImGuiStyleVar_FramePadding,
                        ImVec2(Px(3.0f), ImGui::GetStyle().FramePadding.y));
    const bool custom_header_open =
        ImGui::CollapsingHeader(header_title.c_str(), custom_header_flags);
    ImGui::PopStyleVar();
    if (custom_header_open) {
        ImGui::TextDisabled("Gán phím dấu thanh/mũ/móc theo thói quen. Có thể gán nhiều phím (vd 's1').");
        ImGui::Spacing();

        // Determine current matching preset for visual feedback
        bool is_telex_match = (strcmp(custom_acute_, "s") == 0 && strcmp(custom_hook_all_, "w") == 0 && config_.custom_keymap.telex_double_keys);
        bool is_vni_match = (strcmp(custom_acute_, "1") == 0 && strcmp(custom_circumflex_, "6") == 0);
        bool is_viqr_match = (strcmp(custom_acute_, "'") == 0 && strcmp(custom_circumflex_, "^") == 0);

        ImGui::AlignTextToFramePadding();
        ImGui::TextUnformatted("Nạp nhanh mẫu chuẩn:");
        ImGui::SameLine();

        auto PresetButton = [this](const char* label, int preset_id, bool is_active) {
            if (is_active) {
                ImGui::PushStyleColor(ImGuiCol_Button, ImVec4(0.20f, 0.45f, 0.75f, 1.0f));
                ImGui::PushStyleColor(ImGuiCol_ButtonHovered, ImVec4(0.25f, 0.52f, 0.85f, 1.0f));
            }
            if (ImGui::Button(label, ImVec2(Px(95.0f), Px(24.0f)))) {
                LoadCustomKeymapPreset(preset_id);
                SaveConfig();
            }
            if (is_active) {
                ImGui::PopStyleColor(2);
            }
        };

        PresetButton("Mẫu Telex", 0, is_telex_match);
        ImGui::SameLine();
        PresetButton("Mẫu VNI", 1, is_vni_match);
        ImGui::SameLine();
        PresetButton("Mẫu VIQR", 2, is_viqr_match);
        ImGui::SameLine();
        if (ImGui::Button("Xóa trắng", ImVec2(Px(80.0f), Px(24.0f)))) {
            LoadCustomKeymapPreset(3);
            SaveConfig();
        }

        ImGui::Spacing();
        auto RenderKeyInput = [this](const char* label, const char* example, char* buf, size_t buf_size, const char* hint) {
            ImGui::AlignTextToFramePadding();
            ImGui::TextUnformatted(label);
            if (example && example[0]) {
                ImGui::SameLine();
                ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.50f, 0.55f, 0.65f, 1.0f));
                ImGui::TextUnformatted(example);
                ImGui::PopStyleColor();
            }
            ImGui::SameLine(Px(160.0f));
            ImGui::SetNextItemWidth(Px(80.0f));
            std::string id = std::string("##CustomKey_") + label;
            if (ImGui::InputTextWithHint(id.c_str(), hint, buf, buf_size)) {
                SaveConfig();
            }
        };

        if (ImGui::BeginTable("CustomKeymapTable", 2, ImGuiTableFlags_SizingStretchProp)) {
            ImGui::TableSetupColumn("Dấu thanh", ImGuiTableColumnFlags_WidthStretch, 0.5f);
            ImGui::TableSetupColumn("Dấu mũ & móc", ImGuiTableColumnFlags_WidthStretch, 0.5f);

            ImGui::TableNextRow();
            ImGui::TableSetColumnIndex(0);
            ImGui::TextDisabled("Phím gán dấu thanh:");
            RenderKeyInput("Dấu sắc", "(á, é, ó)", custom_acute_, sizeof(custom_acute_), "s, 1...");
            RenderKeyInput("Dấu huyền", "(à, è, ò)", custom_grave_, sizeof(custom_grave_), "f, 2...");
            RenderKeyInput("Dấu hỏi", "(ả, ẻ, ỏ)", custom_hook_, sizeof(custom_hook_), "r, 3...");
            RenderKeyInput("Dấu ngã", "(ã, ẽ, õ)", custom_tilde_, sizeof(custom_tilde_), "x, 4...");
            RenderKeyInput("Dấu nặng", "(ạ, ẹ, ọ)", custom_dot_, sizeof(custom_dot_), "j, 5...");
            RenderKeyInput("Xóa dấu thanh", "", custom_remove_tone_, sizeof(custom_remove_tone_), "z, 0...");

            ImGui::TableSetColumnIndex(1);
            ImGui::TextDisabled("Phím gán dấu mũ & móc:");
            RenderKeyInput("Mũ chung", "(â, ê, ô)", custom_circumflex_, sizeof(custom_circumflex_), "6, ^...");
            RenderKeyInput("Móc chung", "(ơ, ư)", custom_horn_, sizeof(custom_horn_), "7, +...");
            RenderKeyInput("Trăng", "(ă)", custom_breve_, sizeof(custom_breve_), "8, (...");
            RenderKeyInput("Chữ Đ", "(đ, Đ)", custom_d_mark_, sizeof(custom_d_mark_), "9, d...");
            RenderKeyInput("Móc & trăng", "(w)", custom_hook_all_, sizeof(custom_hook_all_), "w...");

            ImGui::EndTable();
        }

        ImGui::Spacing();
        if (ImGui::TreeNode("Gán phím riêng cho từng chữ cái (Tùy chọn nâng cao)")) {
            ImGui::TextDisabled("Chỉ dùng khi muốn phím riêng cho từng chữ cái:");
            if (ImGui::BeginTable("CustomIndividualTable", 2, ImGuiTableFlags_SizingStretchProp)) {
                ImGui::TableSetupColumn("Chữ Â, Ê, Ô", ImGuiTableColumnFlags_WidthStretch, 0.5f);
                ImGui::TableSetupColumn("Chữ Ơ, Ư, Ă, Đ", ImGuiTableColumnFlags_WidthStretch, 0.5f);

                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                RenderKeyInput("Chữ Â riêng", "(â)", custom_roof_a_, sizeof(custom_roof_a_), "");
                RenderKeyInput("Chữ Ê riêng", "(ê)", custom_roof_e_, sizeof(custom_roof_e_), "");
                RenderKeyInput("Chữ Ô riêng", "(ô)", custom_roof_o_, sizeof(custom_roof_o_), "");

                ImGui::TableSetColumnIndex(1);
                RenderKeyInput("Chữ Ơ riêng", "(ơ)", custom_horn_o_, sizeof(custom_horn_o_), "[...");
                RenderKeyInput("Chữ Ư riêng", "(ư)", custom_horn_u_, sizeof(custom_horn_u_), "]...");
                RenderKeyInput("Chữ Ă riêng", "(ă)", custom_breve_a_, sizeof(custom_breve_a_), "");
                RenderKeyInput("Chữ Đ riêng", "(đ)", custom_d_lower_, sizeof(custom_d_lower_), "");

                ImGui::EndTable();
            }
            ImGui::TreePop();
        }

        ImGui::Spacing();
        if (ImGui::Checkbox("Cho phép gõ lặp mũ kiểu Telex (aa -> â, ee -> ê, oo -> ô, dd -> đ)", &config_.custom_keymap.telex_double_keys)) SaveConfig();
        if (ImGui::Checkbox("Gõ phím móc đứng độc lập thành chữ 'ư'", &config_.custom_keymap.lone_w)) SaveConfig();

        ImGui::Spacing();
        ImGui::AlignTextToFramePadding();
        ImGui::TextUnformatted("Khung gõ thử nghiệm:");
        ImGui::SameLine();
        ImGui::SetNextItemWidth(Px(280.0f));
        ImGui::InputTextWithHint("##TestCustomTyping", "Gõ thử tiếng Việt tại đây...", custom_test_buf_, sizeof(custom_test_buf_));
        ImGui::SameLine();
        if (ImGui::Button("Xóa chữ", ImVec2(Px(70.0f), Px(24.0f)))) {
            custom_test_buf_[0] = '\0';
        }

        ImGui::Spacing();
        if (config_.method != 6) {
            if (ImGui::Button(NF_BOLT "  Kích hoạt sử dụng Kiểu gõ tự định nghĩa này làm kiểu gõ chính", ImVec2(0.0f, Px(28.0f)))) {
                config_.method = 6;
                SaveConfig();
            }
        } else {
            ImGui::TextColored(ImVec4(0.25f, 0.85f, 0.40f, 1.0f), "Kiểu gõ tự định nghĩa đang được kích hoạt làm kiểu gõ chính.");
        }
    }
}

void UiManager::PageMacros() {
    SectionHeader("Gõ tắt");
    if (ImGui::Checkbox("Bật tính năng gõ tắt", &config_.enable_macro)) SaveConfig();
    ImGui::BeginDisabled(!config_.enable_macro);
    if (ImGui::Checkbox("Cho phép gõ tắt ngay cả khi ở chế độ [E] / app loại trừ", &config_.allow_macro_when_disabled)) SaveConfig();
    ImGui::EndDisabled();

    if (ImGui::Button(NF_FOLDER "  Nhập từ file...", ImVec2(0.0f, Px(26.0f)))) {
        std::string path = PickMacroFile(false);
        if (!path.empty()) {
            size_t count = ImportMacros(path, config_.macros);
            if (count > 0) {
                SaveConfig();
                macro_status_msg_ = "Đã nhập " + std::to_string(count) + " từ gõ tắt.";
                macro_status_ok_ = true;
            } else {
                macro_status_msg_ = "Không tìm thấy dòng gõ tắt hợp lệ trong file.";
                macro_status_ok_ = false;
            }
        }
    }
    ImGui::SameLine();
    if (ImGui::Button(NF_SAVE "  Xuất ra file...", ImVec2(0.0f, Px(26.0f)))) {
        std::string path = PickMacroFile(true);
        if (!path.empty()) {
            size_t count = ExportMacros(path, config_.macros);
            macro_status_msg_ = "Đã xuất " + std::to_string(count) + " từ gõ tắt.";
            macro_status_ok_ = count > 0;
        }
    }
    if (!macro_status_msg_.empty()) {
        ImGui::TextColored(
            macro_status_ok_ ? ImVec4(0.30f, 0.85f, 0.45f, 1.0f) : ImVec4(0.95f, 0.45f, 0.35f, 1.0f),
            "%s", macro_status_msg_.c_str());
    }
    ImGui::TextDisabled("Mỗi dòng: khóa<TAB>thay thế, khóa=thay thế hoặc khóa||thay thế.");

    SectionHeader("Thêm từ");
    ImGui::TextDisabled("Ví dụ 'vn' -> 'Việt Nam'; gõ 'vn' + Space/Enter.");
    ImGui::Spacing();

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Từ tắt:");
    ImGui::SameLine();
    ImGui::SetNextItemWidth(Px(120.0f));
    ImGui::InputTextWithHint("##MacroKey", "vn", new_macro_key_, sizeof(new_macro_key_));

    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("Thay thế:");
    ImGui::SameLine();

    float avail = ImGui::GetContentRegionAvail().x;
    float btn_width = Px(110.0f);
    ImGui::SetNextItemWidth(avail - btn_width - Px(10.0f));
    bool enter_pressed = ImGui::InputTextWithHint("##MacroVal", "Cụm từ đầy đủ (Nhấn Enter để thêm)...", new_macro_val_, sizeof(new_macro_val_), ImGuiInputTextFlags_EnterReturnsTrue);

    ImGui::SameLine();
    bool btn_clicked = ImGui::Button(NF_SAVE "  Lưu", ImVec2(btn_width, Px(26.0f)));
    if (enter_pressed || btn_clicked) {
        if (strlen(new_macro_key_) > 0 && strlen(new_macro_val_) > 0) {
            config_.macros[new_macro_key_] = new_macro_val_;
            new_macro_key_[0] = '\0';
            new_macro_val_[0] = '\0';
            SaveConfig();
            ImGui::SetKeyboardFocusHere(-2);
        }
    }

    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.60f, 0.70f, 1.00f));
    ImGui::TextUnformatted("Hỗ trợ biến động: @date, @time, @datetime, @clipboard, @tab, @newline");
    ImGui::PopStyleColor();

    SectionHeader("Danh sách");
    ImGui::AlignTextToFramePadding();
    ImGui::TextDisabled("Tổng cộng: %zu từ gõ tắt", config_.macros.size());
    ImGui::SameLine(0.0f, Px(16.0f));
    ImGui::SetNextItemWidth(Px(240.0f));
    ImGui::InputTextWithHint("##MacroFilter", "Tìm từ tắt hoặc nội dung...", macro_filter_, sizeof(macro_filter_));

    if (!config_.macros.empty()) {
        ImGui::SameLine(0.0f, Px(16.0f));
        if (ImGui::SmallButton(NF_TRASH "  Xóa toàn bộ")) {
            ImGui::OpenPopup("Xác nhận xóa toàn bộ gõ tắt");
        }
    }

    if (ImGui::BeginPopupModal("Xác nhận xóa toàn bộ gõ tắt", nullptr, ImGuiWindowFlags_AlwaysAutoResize)) {
        ImGui::TextUnformatted("Bạn có chắc chắn muốn xóa toàn bộ danh sách từ gõ tắt không?");
        ImGui::Spacing();
        if (ImGui::Button("Xác nhận xóa", ImVec2(Px(110.0f), Px(26.0f)))) {
            config_.macros.clear();
            SaveConfig();
            ImGui::CloseCurrentPopup();
        }
        ImGui::SameLine();
        if (ImGui::Button("Hủy bỏ", ImVec2(Px(80.0f), Px(26.0f)))) {
            ImGui::CloseCurrentPopup();
        }
        ImGui::EndPopup();
    }

    std::string remove_key;
    if (ImGui::BeginTable("MacroTable", 3, ImGuiTableFlags_RowBg | ImGuiTableFlags_BordersInnerV)) {
        ImGui::TableSetupColumn("Từ tắt", ImGuiTableColumnFlags_WidthFixed, Px(120.0f));
        ImGui::TableSetupColumn("Cụm từ thay thế", ImGuiTableColumnFlags_WidthStretch);
        ImGui::TableSetupColumn("Thao tác", ImGuiTableColumnFlags_WidthFixed, Px(70.0f));
        ImGui::TableHeadersRow();

        std::string filter_str = macro_filter_;
        std::transform(filter_str.begin(), filter_str.end(), filter_str.begin(), ::tolower);
        int match_count = 0;

        for (const auto& [k, v] : config_.macros) {
            if (!filter_str.empty()) {
                std::string lk = k; std::transform(lk.begin(), lk.end(), lk.begin(), ::tolower);
                std::string lv = v; std::transform(lv.begin(), lv.end(), lv.begin(), ::tolower);
                if (lk.find(filter_str) == std::string::npos && lv.find(filter_str) == std::string::npos) {
                    continue;
                }
            }
            ++match_count;

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

        if (match_count == 0 && !filter_str.empty()) {
            ImGui::Spacing();
            ImGui::TextDisabled("Không tìm thấy từ gõ tắt phù hợp với bộ lọc.");
        }
    }
    if (!remove_key.empty()) {
        config_.macros.erase(remove_key);
        SaveConfig();
    }
}

void UiManager::PageExclusions() {
    SectionHeader("Phím tắt trong app loại trừ");
    if (ImGui::Checkbox("Khóa phím tắt khi mới vào app loại trừ", &config_.lock_hotkeys_in_excluded_apps)) {
        SaveConfig();
    }
    if (ImGui::IsItemHovered()) {
        ImGui::SetTooltip("Sau khi bật tạm bằng icon khay, V/E đổi qua lại được trong app đó.");
    }

    SectionHeader("Cấu hình theo app");
    ImGui::TextDisabled("Ghi đè kiểu gõ, bảng mã, gửi phím, gõ tắt. Để trống = dùng chung.");
    ImGui::SetNextItemWidth(Px(240.0f));
    ImGui::InputTextWithHint("##NewProfileApp", "vd: Code.exe, chrome.exe...", new_profile_app_, sizeof(new_profile_app_));
    ImGui::SameLine();
    if (ImGui::Button(NF_PLUS "  Thêm profile", ImVec2(Px(130.0f), Px(26.0f)))) {
        std::string app = new_profile_app_;
        std::transform(app.begin(), app.end(), app.begin(), ::tolower);
        if (!app.empty() && config_.app_profiles.find(app) == config_.app_profiles.end()) {
            config_.app_profiles[app] = AppProfile{};
            SaveConfig();
        }
        new_profile_app_[0] = '\0';
    }
    if (!profile_status_.empty()) {
        ImGui::TextColored(ImVec4(0.55f, 0.70f, 0.90f, 1.0f), "%s", profile_status_.c_str());
    }

    static const char* profile_methods[] = {
        "Theo mặc định", "Telex", "VNI", "VIQR", "Telex đơn giản", "Telex đơn giản 2", "Telex + VNI", "Tự định nghĩa"
    };
    static const char* profile_charsets[] = {
        "Theo mặc định", "Unicode", "Unicode tổ hợp", "TCVN3", "VNI Windows"
    };
    static const char* profile_injections[] = {
        "Theo mặc định", "Tự động", "Unicode", "Scan-code", "An toàn"
    };
    static const int profile_injection_values[] = {-1, 0, 1, 2, 4};
    static const char* profile_macros[] = {
        "Theo mặc định", "Tắt gõ tắt", "Bật gõ tắt"
    };

    std::string remove_profile;
    if (!config_.app_profiles.empty()) {
        if (ImGui::BeginTable("AppProfiles", 6, ImGuiTableFlags_RowBg | ImGuiTableFlags_BordersInnerV | ImGuiTableFlags_SizingStretchProp)) {
            ImGui::TableSetupColumn("Ứng dụng", ImGuiTableColumnFlags_WidthFixed, Px(150.0f));
            ImGui::TableSetupColumn("Kiểu gõ", ImGuiTableColumnFlags_WidthFixed, Px(130.0f));
            ImGui::TableSetupColumn("Bảng mã", ImGuiTableColumnFlags_WidthFixed, Px(130.0f));
            ImGui::TableSetupColumn("Gửi phím", ImGuiTableColumnFlags_WidthFixed, Px(150.0f));
            ImGui::TableSetupColumn("Gõ tắt", ImGuiTableColumnFlags_WidthFixed, Px(120.0f));
            ImGui::TableSetupColumn("", ImGuiTableColumnFlags_WidthFixed, Px(55.0f));
            ImGui::TableHeadersRow();

            for (auto& [name, profile] : config_.app_profiles) {
                ImGui::TableNextRow();
                ImGui::TableSetColumnIndex(0);
                ImGui::TextUnformatted(name.c_str());

                int method = profile.method + 1;
                ImGui::TableSetColumnIndex(1);
                ImGui::SetNextItemWidth(-FLT_MIN);
                if (ImGui::Combo("##PMethod", &method, profile_methods, IM_ARRAYSIZE(profile_methods))) {
                    profile.method = method - 1;
                    SaveConfig();
                }

                int charset = profile.charset + 1;
                ImGui::TableSetColumnIndex(2);
                ImGui::SetNextItemWidth(-FLT_MIN);
                if (ImGui::Combo("##PCharset", &charset, profile_charsets, IM_ARRAYSIZE(profile_charsets))) {
                    profile.charset = charset - 1;
                    SaveConfig();
                }

                int injection = 0;
                for (int i = 0; i < IM_ARRAYSIZE(profile_injection_values); ++i) {
                    if (profile_injection_values[i] == profile.injection_mode) {
                        injection = i;
                        break;
                    }
                }
                ImGui::TableSetColumnIndex(3);
                ImGui::SetNextItemWidth(-FLT_MIN);
                if (ImGui::Combo("##PInject", &injection, profile_injections, IM_ARRAYSIZE(profile_injections))) {
                    profile.injection_mode = profile_injection_values[injection];
                    SaveConfig();
                }

                int macro = profile.allow_macro + 1;
                ImGui::TableSetColumnIndex(4);
                ImGui::SetNextItemWidth(-FLT_MIN);
                if (ImGui::Combo("##PMacro", &macro, profile_macros, IM_ARRAYSIZE(profile_macros))) {
                    profile.allow_macro = macro - 1;
                    SaveConfig();
                }

                ImGui::TableSetColumnIndex(5);
                ImGui::PushID(name.c_str());
                if (ImGui::SmallButton(NF_TRASH)) remove_profile = name;
                ImGui::PopID();
            }
            ImGui::EndTable();
        }
    }
    if (!remove_profile.empty()) {
        config_.app_profiles.erase(remove_profile);
        SaveConfig();
    }

    SectionHeader("Thêm app loại trừ");
    ImGui::TextDisabled("App trong danh sách tắt tiếng Việt; icon khay đổi tay.");
    ImGui::Spacing();
    ImGui::AlignTextToFramePadding();
    ImGui::TextUnformatted("App:");
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

    ImGui::Spacing();
    ImGui::TextDisabled("Thêm nhanh:");
    ImGui::Spacing();

    struct QuickExcl { const char* label; const char* exe; const char* alt_exe; };
    static const QuickExcl s_quick_excls[] = {
        { "Counter-Strike 2", "cs2.exe", nullptr },
        { "Valorant", "valorant.exe", nullptr },
        { "Liên Minh (LoL)", "league of legends.exe", "leagueclient.exe" },
        { "Dota 2", "dota2.exe", nullptr },
        { "Genshin Impact", "genshinimpact.exe", nullptr },
        { "Windows Terminal", "windowsterminal.exe", nullptr },
        { "Command Prompt", "cmd.exe", nullptr },
        { "PowerShell", "powershell.exe", nullptr },
    };

    for (int i = 0; i < IM_ARRAYSIZE(s_quick_excls); ++i) {
        bool already = (std::find(config_.excluded_apps.begin(), config_.excluded_apps.end(), s_quick_excls[i].exe) != config_.excluded_apps.end());
        if (already) {
            ImGui::BeginDisabled();
            ImGui::Button((std::string(NF_CHECK " ") + s_quick_excls[i].label).c_str(), ImVec2(0.0f, Px(24.0f)));
            ImGui::EndDisabled();
        } else {
            if (ImGui::Button((std::string(NF_PLUS " ") + s_quick_excls[i].label).c_str(), ImVec2(0.0f, Px(24.0f)))) {
                config_.excluded_apps.push_back(s_quick_excls[i].exe);
                if (s_quick_excls[i].alt_exe && std::find(config_.excluded_apps.begin(), config_.excluded_apps.end(), s_quick_excls[i].alt_exe) == config_.excluded_apps.end()) {
                    config_.excluded_apps.push_back(s_quick_excls[i].alt_exe);
                }
                SaveConfig();
            }
        }
        if (i < IM_ARRAYSIZE(s_quick_excls) - 1) {
            float next_w = ImGui::CalcTextSize(s_quick_excls[i + 1].label).x + Px(36.0f);
            float next_right = (ImGui::GetItemRectMax().x - ImGui::GetWindowPos().x) + Px(6.0f) + next_w;
            if (next_right <= ImGui::GetWindowContentRegionMax().x) {
                ImGui::SameLine(0.0f, Px(6.0f));
            }
        }
    }

    // Modal popup to select running applications
    ImVec2 center = ImGui::GetMainViewport()->GetCenter();
    ImGui::SetNextWindowPos(center, ImGuiCond_Appearing, ImVec2(0.5f, 0.5f));
    ImGui::SetNextWindowSize(ImVec2(Px(580.0f), Px(380.0f)), ImGuiCond_Appearing);

    if (ImGui::BeginPopupModal("Chọn ứng dụng đang chạy", nullptr, ImGuiWindowFlags_NoResize)) {
        ImGui::TextUnformatted("Chọn app đang mở:");
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

    SectionHeader("App không dùng tiếng Việt");
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
    SectionHeader("Bảng mã");
    ImGui::AlignTextToFramePadding();
    ImGui::TextColored(ImVec4(0.35f, 0.70f, 1.0f, 1.0f), NF_BOLT " Phím tắt chuyển nhanh Clipboard toàn hệ thống: ");
    ImGui::SameLine(0.0f, Px(8.0f));
    HotkeyBadge(config_.hotkey_converter.empty() ? "Ctrl + Shift + F9" : config_.hotkey_converter.c_str());
    ImGui::Spacing();

    ImGui::TextDisabled("Dán văn bản, chọn bảng mã, bấm Chuyển mã.");
    ImGui::TextDisabled("TCVN3/VNI: Clipboard đọc/ghi byte gốc; hộp text là bản xem trước.");
    ImGui::Spacing();

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

    ImGui::Spacing();
    if (ImGui::Button(NF_CONVERT "  Chuyển mã Clipboard", ImVec2(0.0f, Px(26.0f)))) {
        std::string input;
        bool legacy_missing = false;
        if (conv_src_cs_ >= 2) {
            input = ReadClipboardAnsi();
            legacy_missing = input.empty();
        } else {
            const char* clip = ImGui::GetClipboardText();
            if (clip) input = clip;
        }
        if (legacy_missing) {
            conv_status_msg_ = "Clipboard không có dữ liệu TCVN3/VNI (CF_TEXT). Hãy sao chép từ ứng dụng cũ.";
            conv_status_ok_ = false;
        } else if (input.empty()) {
            conv_status_msg_ = "Clipboard đang trống.";
            conv_status_ok_ = false;
        } else {
            bool ok = false;
            if (conv_strip_accents_) {
                std::string unicode = conv_src_cs_ >= 2
                    ? ConvertRaw(input, conv_src_cs_, 0)
                    : input;
                std::string result = ApplyCase(ConvertText(unicode, 0, 0, true), conv_uppercase_, conv_lowercase_);
                if (!result.empty()) {
                    ImGui::SetClipboardText(result.c_str());
                    ok = true;
                }
            } else if (conv_dst_cs_ >= 2) {
                std::string raw = ConvertRaw(input, conv_src_cs_, conv_dst_cs_);
                if (!raw.empty()) {
                    std::string preview = ConvertRaw(raw, conv_dst_cs_, 0);
                    if (conv_uppercase_ || conv_lowercase_) {
                        preview = ApplyCase(preview, conv_uppercase_, conv_lowercase_);
                        raw = ConvertRaw(preview, 0, conv_dst_cs_);
                    }
                    ok = WriteClipboardRaw(raw, preview);
                }
            } else {
                std::string result = ApplyCase(
                    ConvertText(input, conv_src_cs_, conv_dst_cs_, false),
                    conv_uppercase_, conv_lowercase_);
                if (!result.empty()) {
                    ImGui::SetClipboardText(result.c_str());
                    ok = true;
                }
            }
            conv_status_msg_ = ok
                ? "Đã chuyển mã và lưu vào Clipboard."
                : "Không chuyển được. Kiểm tra bảng mã nguồn.";
            conv_status_ok_ = ok;
        }
    }

    SectionHeader("Văn bản");
    static char src_buf[4096] = {0};
    static char dst_buf[4096] = {0};

    if (ImGui::BeginTable("DirectConvBoxes", 2, ImGuiTableFlags_SizingStretchProp)) {
        ImGui::TableSetupColumn("Nguồn", ImGuiTableColumnFlags_WidthStretch, 0.5f);
        ImGui::TableSetupColumn("Kết quả", ImGuiTableColumnFlags_WidthStretch, 0.5f);

        ImGui::TableNextRow();
        ImGui::TableSetColumnIndex(0);
        ImGui::TextUnformatted("Văn bản nguồn:");
        ImGui::SameLine();
        if (ImGui::SmallButton(NF_COPY " Dán từ Clipboard")) {
            const char* clip = ImGui::GetClipboardText();
            if (clip) {
                strncpy_s(src_buf, sizeof(src_buf), clip, _TRUNCATE);
            }
        }
        ImGui::InputTextMultiline("##ConvSrcText", src_buf, sizeof(src_buf), ImVec2(-Px(10.0f), Px(130.0f)));

        ImGui::TableSetColumnIndex(1);
        ImGui::TextUnformatted("Kết quả chuyển đổi:");
        ImGui::InputTextMultiline("##ConvDstText", dst_buf, sizeof(dst_buf), ImVec2(-Px(10.0f), Px(130.0f)));

        ImGui::EndTable();
    }

    ImGui::Spacing();
    if (ImGui::Button(NF_CONVERT "  Chuyển mã văn bản", ImVec2(0.0f, Px(28.0f)))) {
        std::string result = ApplyCase(
            ConvertText(src_buf, conv_src_cs_, conv_dst_cs_, conv_strip_accents_),
            conv_uppercase_, conv_lowercase_);
        if (result.empty() && src_buf[0] != '\0') {
            conv_status_msg_ = "Không chuyển được. Kiểm tra bảng mã nguồn.";
            conv_status_ok_ = false;
        } else {
            strncpy_s(dst_buf, sizeof(dst_buf), result.c_str(), _TRUNCATE);
            conv_status_msg_ = "Đã chuyển mã văn bản.";
            conv_status_ok_ = true;
        }
    }

    ImGui::SameLine();
    if (ImGui::Button(NF_COPY "  Sao chép kết quả", ImVec2(0.0f, Px(28.0f)))) {
        bool ok = true;
        if (conv_dst_cs_ >= 2) {
            ok = CopyConvertedToClipboard(dst_buf, 0, conv_dst_cs_);
        } else {
            ImGui::SetClipboardText(dst_buf);
        }
        conv_status_msg_ = ok ? "Đã sao chép kết quả." : "Không sao chép được kết quả.";
        conv_status_ok_ = ok;
    }

    ImGui::SameLine();
    if (ImGui::Button(NF_EXCHANGE "  Chuyển kết quả sang nguồn", ImVec2(0.0f, Px(28.0f)))) {
        strncpy_s(src_buf, sizeof(src_buf), dst_buf, _TRUNCATE);
        dst_buf[0] = '\0';
    }

    ImGui::SameLine();
    if (ImGui::Button(NF_TRASH "  Xóa trắng", ImVec2(0.0f, Px(28.0f)))) {
        src_buf[0] = '\0';
        dst_buf[0] = '\0';
        conv_status_msg_.clear();
    }

    if (!conv_status_msg_.empty()) {
        ImGui::TextColored(
            conv_status_ok_ ? ImVec4(0.30f, 0.85f, 0.45f, 1.0f) : ImVec4(0.95f, 0.45f, 0.35f, 1.0f),
            "%s", conv_status_msg_.c_str());
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
    SectionHeader("Biểu tượng khay");

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

    SectionHeader("Thời gian chốt từ");
    int timeout = (int)config_.typing_timeout_ms;
    ImGui::SetNextItemWidth(Px(260.0f));
    if (ImGui::SliderInt("##TimeoutSlider", &timeout, 500, 5000, "%d ms")) {
        config_.typing_timeout_ms = (uint64_t)timeout;
        SaveConfig();
    }
    ImGui::PushStyleColor(ImGuiCol_Text, ImVec4(0.55f, 0.58f, 0.67f, 1.00f));
    ImGui::TextUnformatted("Chờ giữa 2 phím để chốt từ (mặc định 1500 ms).");
    ImGui::PopStyleColor();

    ImGui::Dummy(ImVec2(0.0f, Px(20.0f)));
}

void UiManager::PageAbout() {
    SectionHeader("VietFlow");
    ImGui::TextUnformatted("Bộ gõ tiếng Việt nhẹ, hiện đại");
    ImGui::TextUnformatted("Phiên bản: v" VIETFLOW_VERSION);
    ImGui::TextUnformatted("Bản quyền: Giấy phép mã nguồn mở GPLv3");

    ImGui::Spacing();
    ImGui::TextUnformatted("Trang chủ dự án:");
    ImGui::SameLine();
    if (ImGui::SmallButton("github.com/KienPC1234/VietFlow")) {
        ShellExecuteA(NULL, "open", REPO_URL, NULL, NULL, SW_SHOWNORMAL);
    }

    ImGui::Spacing();
    const char* chk_label = NF_CONVERT " Kiểm tra bản cập nhật";
    float chk_w = ImGui::CalcTextSize(chk_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(chk_label, ImVec2(chk_w, Px(28.0f)))) {
        HWND hwndDaemon = FindWindowW(WINDOW_CLASS_DAEMON, WINDOW_TITLE_DAEMON);
        if (hwndDaemon) {
            PostMessageW(hwndDaemon, WM_COMMAND, IDM_VF_CHECK_UPDATE, 0);
        } else {
            ShellExecuteA(NULL, "open", RELEASES_URL, NULL, NULL, SW_SHOWNORMAL);
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

    ImGui::Spacing();
    ImGui::Separator();
    ImGui::Spacing();

    SectionHeader("Ủng hộ tác giả");
    ImGui::TextUnformatted("Nếu thấy VietFlow hữu ích, bạn có thể ủng hộ tác giả:");

    ImGui::Spacing();
    if (ImGui::BeginTable("DonationTable", 2, ImGuiTableFlags_BordersInnerH | ImGuiTableFlags_RowBg, ImVec2(Px(500.0f), 0.0f))) {
        ImGui::TableSetupColumn("Key", ImGuiTableColumnFlags_WidthFixed, Px(130.0f));
        ImGui::TableSetupColumn("Value", ImGuiTableColumnFlags_WidthStretch);

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        ImGui::TextUnformatted("Ngân hàng:");
        ImGui::TableNextColumn();
        ImGui::TextUnformatted(DONATION_BANK_NAME);

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        ImGui::TextUnformatted("Số tài khoản:");
        ImGui::TableNextColumn();
        ImGui::TextUnformatted(DONATION_ACCOUNT_DISPLAY);
        ImGui::SameLine();
        if (ImGui::SmallButton("Sao chép STK")) {
            ImGui::SetClipboardText(DONATION_ACCOUNT_NO);
        }

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        ImGui::TextUnformatted("Chủ tài khoản:");
        ImGui::TableNextColumn();
        ImGui::TextUnformatted(DONATION_ACCOUNT_HOLDER);

        ImGui::TableNextRow();
        ImGui::TableNextColumn();
        ImGui::TextUnformatted("Nội dung:");
        ImGui::TableNextColumn();
        ImGui::TextUnformatted(DONATION_MESSAGE);
        ImGui::SameLine();
        if (ImGui::SmallButton("Sao chép nội dung")) {
            ImGui::SetClipboardText(DONATION_MESSAGE);
        }

        ImGui::EndTable();
    }

    ImGui::Spacing();
    const char* qr_label = NF_CONVERT " Mở mã QR chuyển khoản (VietQR)";
    float qr_w = ImGui::CalcTextSize(qr_label).x + ImGui::GetStyle().FramePadding.x * 2.0f + Px(20.0f);
    if (ImGui::Button(qr_label, ImVec2(qr_w, Px(28.0f)))) {
        ShellExecuteA(NULL, "open", DONATION_VIETQR_URL, NULL, NULL, SW_SHOWNORMAL);
    }

    ImGui::Dummy(ImVec2(0.0f, Px(20.0f)));
}

} // namespace vietflow
