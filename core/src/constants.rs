// VietFlow Core - Shared Engine Constants
// Licensed under GNU General Public License v3.0 (GPLv3)

pub const METHOD_TELEX: u8 = 0;
pub const METHOD_VNI: u8 = 1;
pub const METHOD_VIQR: u8 = 2;
pub const METHOD_SIMPLE_TELEX: u8 = 3;
pub const METHOD_SIMPLE_TELEX2: u8 = 4;
pub const METHOD_TELEX_VNI: u8 = 5;
pub const METHOD_USER_CUSTOM: u8 = 6;

pub const CHARSET_UNICODE: u8 = 0;
pub const CHARSET_UNICODE_COMPOSITE: u8 = 1;
pub const CHARSET_TCVN3: u8 = 2;
pub const CHARSET_VNI_WINDOWS: u8 = 3;
pub const CHARSET_VIQR: u8 = 4;

pub const DEFAULT_TYPING_TIMEOUT_MS: u64 = 1500;
pub const DEFAULT_FOREIGN_INITIALS: &str = "fjwz";

pub const PRESET_TELEX: u8 = 0;
pub const PRESET_VNI: u8 = 1;
pub const PRESET_VIQR: u8 = 2;
pub const PRESET_CLEAR: u8 = 3;

