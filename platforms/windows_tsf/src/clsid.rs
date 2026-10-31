// VietFlow Windows TSF - GUIDs & Constants
// Licensed under GNU General Public License v3.0 (GPLv3)

use windows::core::GUID;

// {7c2b5e10-9a31-4b8f-b623-192e48270101}
pub const CLSID_VIETFLOW_TIP: GUID = GUID::from_u128(0x7c2b5e10_9a31_4b8f_b623_192e48270101);

// {7c2b5e11-9a31-4b8f-b623-192e48270102}
pub const GUID_VIETFLOW_PROFILE: GUID = GUID::from_u128(0x7c2b5e11_9a31_4b8f_b623_192e48270102);

// {7c2b5e12-9a31-4b8f-b623-192e48270103}
pub const GUID_VIETFLOW_DISPLAY_ATTR: GUID = GUID::from_u128(0x7c2b5e12_9a31_4b8f_b623_192e48270103);

pub const TEXT_SERVICE_NAME: &str = "VietFlow Vietnamese Keyboard";
pub const TEXT_SERVICE_PROFILE_NAME: &str = "VietFlow";
pub const LANGID_VIETNAMESE: u16 = 0x042A; // Vietnamese (vi-VN)
pub const LANGID_ENGLISH_US: u16 = 0x0409; // English (United States)
