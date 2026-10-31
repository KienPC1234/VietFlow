// VietFlow ImGui UI - System Constants and Identifiers
// Licensed under GNU General Public License v3.0 (GPLv3)

#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>

namespace vietflow {

// Windows Message IDs (matching Windows daemon)
inline constexpr UINT WM_VF_TRAYICON = WM_USER + 101;
inline constexpr UINT WM_VF_SETTINGS = WM_USER + 102;
inline constexpr UINT WM_VF_CONVERT = WM_USER + 103;
inline constexpr UINT WM_VF_RELOAD_CONFIG = WM_USER + 104;

// Command IDs
inline constexpr UINT IDM_VF_CHECK_UPDATE = 1070;
inline constexpr UINT IDM_VF_ABOUT = 1071;
inline constexpr UINT IDM_VF_RESTART_ADMIN = 1072;

// Window Class Names & IPC Identifiers
inline constexpr const wchar_t* WINDOW_CLASS_DAEMON = L"VietFlowMessageWindowClass";
inline constexpr const wchar_t* WINDOW_TITLE_DAEMON = L"VietFlowDaemon";
inline constexpr const wchar_t* WINDOW_CLASS_UI = L"VietFlowImGuiWindow";

// UI Window Titles
inline constexpr const wchar_t* WINDOW_TITLE_CONVERT = L"VietFlow - Chuy\u1EC3n m\u00E3 v\u0103n b\u1EA3n";
inline constexpr const wchar_t* WINDOW_TITLE_SETTINGS = L"VietFlow - B\u1EA3ng \u0111i\u1EC1u khi\u1EC3n b\u1ED9 g\u00F5 ti\u1EBFng Vi\u1EC7t";

// App and Config Path Identifiers
inline constexpr const char* APP_DIR_NAME = "VietFlow";
inline constexpr const char* CONFIG_FILE_NAME = "vietflow_config.json";

// Repository & Update URLs
inline constexpr const char* REPO_URL = "https://github.com/KienPC1234/VietFlow";
inline constexpr const char* RELEASES_URL = "https://github.com/KienPC1234/VietFlow/releases";

// Donation / Author Support Information
inline constexpr const char* DONATION_BANK_NAME = "Techcombank (Napas 247)";
inline constexpr const char* DONATION_BANK_BIN = "970407";
inline constexpr const char* DONATION_ACCOUNT_NO = "9978589220";
inline constexpr const char* DONATION_ACCOUNT_DISPLAY = "9978 5892 20";
inline constexpr const char* DONATION_ACCOUNT_HOLDER = "HA TRI KIEN";
inline constexpr const char* DONATION_MESSAGE = "Ung ho VietFlow";
inline constexpr const char* DONATION_VIETQR_PAYLOAD = "00020101021138540010A00000072701240006970407011099785892200208QRIBFTTA53037045802VN5911HA TRI KIEN6005HANOI62190815Ung ho VietFlow6304D712";
inline constexpr const char* DONATION_VIETQR_URL = "https://img.vietqr.io/image/970407-9978589220-compact2.png?amount=0&addInfo=Ung%20ho%20VietFlow&accountName=HA%20TRI%20KIEN";

// Input Method Identifiers
enum class InputMethodId : int {
    Telex = 0,
    Vni = 1,
    Viqr = 2,
    SimpleTelex = 3,
    TelexSimple2 = 4,
    TelexVni = 5,
    UserCustom = 6,
};

// Charset Identifiers
enum class CharsetId : int {
    Unicode = 0,
    UnicodeComposite = 1,
    Tcvn3 = 2,
    VniWindows = 3,
    Viqr = 4,
};

} // namespace vietflow
