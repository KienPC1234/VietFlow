// VietFlow Windows - System Constants and Identifiers
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows_sys::Win32::UI::WindowsAndMessaging::WM_USER;

// Windows Message IDs
pub const WM_TRAYICON: u32 = WM_USER + 101;
pub const WM_SETTINGS: u32 = WM_USER + 102;
pub const WM_CONVERT: u32 = WM_USER + 103;
pub const WM_RELOAD_CONFIG: u32 = WM_USER + 104;

// Tray Context Menu Action IDs
pub const IDM_TOGGLE: usize = 1001;

pub const IDM_METHOD_TELEX: usize = 1010;
pub const IDM_METHOD_VNI: usize = 1011;
pub const IDM_METHOD_VIQR: usize = 1012;
pub const IDM_METHOD_SIMPLE_TELEX: usize = 1013;
pub const IDM_METHOD_SIMPLE_TELEX2: usize = 1014;
pub const IDM_METHOD_TELEX_VNI: usize = 1015;
pub const IDM_METHOD_CUSTOM: usize = 1016;

pub const IDM_CHARSET_UNICODE: usize = 1020;
pub const IDM_CHARSET_COMPOSITE: usize = 1021;
pub const IDM_CHARSET_TCVN3: usize = 1022;
pub const IDM_CHARSET_VNI: usize = 1023;

pub const IDM_SETTINGS: usize = 1030;
pub const IDM_EXIT: usize = 1040;

pub const IDM_TOGGLE_MACRO: usize = 1050;
pub const IDM_TOGGLE_SPELL: usize = 1051;
pub const IDM_TOGGLE_STARTUP: usize = 1052;
pub const IDM_CONVERT_TOOL: usize = 1060;

pub const IDM_CHECK_UPDATE: usize = 1070;
pub const IDM_ABOUT: usize = 1071;
pub const IDM_RESTART_ADMIN: usize = 1072;

// Windows Class Names & IPC Identifiers
pub const WINDOW_CLASS_DAEMON: &str = "VietFlowMessageWindowClass";
pub const WINDOW_TITLE_DAEMON: &str = "VietFlowDaemon";
pub const MUTEX_SINGLE_INSTANCE: &str = "Global\\VietFlowSingleInstanceMutex";

// App and Config Path Identifiers
pub const APP_DIR_NAME: &str = "VietFlow";
pub const CONFIG_FILE_NAME: &str = "vietflow_config.json";

