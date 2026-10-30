// VietFlow Windows - Configuration Storage
// Licensed under GNU General Public License v3.0 (GPLv3)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

/// Per-application overrides. All fields are optional: `None` means "inherit
/// the global setting". The map is keyed by executable name.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppProfile {
    #[serde(default)]
    pub method: Option<u8>,
    #[serde(default)]
    pub charset: Option<u8>,
    #[serde(default)]
    pub injection_mode: Option<u8>,
    #[serde(default)]
    pub allow_macro: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    pub method: u8, // 0: Telex, 1: VNI, 2: VIQR, 3: Simple Telex, 4: Simple Telex 2, 5: Telex + VNI
    #[serde(default)]
    pub charset: u8, // 0: Unicode, 1: Composite, 2: TCVN3, 3: VNI Windows
    #[serde(default = "default_true")]
    pub modern_style: bool,
    #[serde(default = "default_true")]
    pub free_marking: bool,
    #[serde(default = "default_true")]
    pub spell_check: bool,
    #[serde(default = "default_true")]
    pub auto_restore: bool,
    #[serde(default = "default_true")]
    pub fix_shift_lag: bool,
    #[serde(default)]
    pub consonant_shortcuts: bool,
    #[serde(default)]
    pub auto_capitalize: bool,
    #[serde(default = "default_auto_capitalize_chars")]
    pub auto_capitalize_chars: String,
    /// Disable Vietnamese input when the foreground window's keyboard layout
    /// is not US (EVKey: "Tắt TV khi Layout bàn phím khác US").
    #[serde(default)]
    pub disable_on_non_us_layout: bool,
    #[serde(default = "default_true")]
    pub enable_macro: bool,
    #[serde(default)]
    pub allow_macro_when_disabled: bool,
    /// 0: Auto, 1: Unicode SendInput, 2: scan-code, 3: clipboard, 4: safe paced.
    #[serde(default)]
    pub injection_mode: u8,
    #[serde(default)]
    pub use_clipboard_for_unicode: bool,
    #[serde(default)]
    pub start_with_windows: bool,
    #[serde(default)]
    pub notification_sound: bool,
    #[serde(default = "default_timeout")]
    pub typing_timeout_ms: u64,
    #[serde(default = "default_hotkey_mode")]
    pub hotkey_mode: String,
    #[serde(default = "default_hotkey_settings")]
    pub hotkey_settings: String,
    #[serde(default = "default_hotkey_converter")]
    pub hotkey_converter: String,

    #[serde(default)]
    pub english_keywords: Vec<String>,
    #[serde(default)]
    pub english_keywords_removed: Vec<String>,
    #[serde(default = "default_foreign_initials")]
    pub foreign_initials: String,
    #[serde(default = "default_tray_theme")]
    pub tray_icon_theme: String,
    #[serde(default)]
    pub custom_icon_v: String,
    #[serde(default)]
    pub custom_icon_e: String,

    #[serde(default = "default_true")]
    pub tray_icon_v_fill: bool,
    #[serde(default = "default_white")]
    pub tray_icon_v_fill_color: String,
    #[serde(default = "default_true")]
    pub tray_icon_v_border: bool,
    #[serde(default = "default_pink")]
    pub tray_icon_v_border_color: String,

    #[serde(default = "default_true")]
    pub tray_icon_e_fill: bool,
    #[serde(default = "default_white")]
    pub tray_icon_e_fill_color: String,
    #[serde(default = "default_true")]
    pub tray_icon_e_border: bool,
    #[serde(default = "default_blue")]
    pub tray_icon_e_border_color: String,

    #[serde(default = "default_excluded_apps")]
    pub excluded_apps: Vec<String>,
    /// Lock V/E, settings and converter hotkeys while an excluded app is in
    /// front. The tray/menu override remains available.
    #[serde(default = "default_true")]
    pub lock_hotkeys_in_excluded_apps: bool,
    #[serde(default)]
    pub app_profiles: HashMap<String, AppProfile>,
    #[serde(default = "default_macros")]
    pub macros: HashMap<String, String>,
    #[serde(default)]
    pub custom_keymap: CustomKeymapConfig,
}

fn default_true() -> bool {
    true
}
fn default_auto_capitalize_chars() -> String {
    ".!?\r\n".to_string()
}
fn default_timeout() -> u64 {
    1500
}
fn default_hotkey_mode() -> String {
    "CtrlShift".to_string()
}
fn default_hotkey_settings() -> String {
    "Ctrl + Shift + F5".to_string()
}
fn default_hotkey_converter() -> String {
    "Ctrl + Shift + F9".to_string()
}
fn default_foreign_initials() -> String {
    "fjwz".to_string()
}
fn default_tray_theme() -> String {
    "modern".to_string()
}
fn default_white() -> String {
    "#FFFFFF".to_string()
}
fn default_pink() -> String {
    "#FF007F".to_string()
}
fn default_blue() -> String {
    "#0078D7".to_string()
}

fn default_acute() -> String { "s".to_string() }
fn default_grave() -> String { "f".to_string() }
fn default_hook() -> String { "r".to_string() }
fn default_tilde() -> String { "x".to_string() }
fn default_dot() -> String { "j".to_string() }
fn default_remove_tone() -> String { "z".to_string() }
fn default_hook_all() -> String { "w".to_string() }
fn default_empty() -> String { String::new() }

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CustomKeymapConfig {
    #[serde(default = "default_acute")]
    pub acute: String,
    #[serde(default = "default_grave")]
    pub grave: String,
    #[serde(default = "default_hook")]
    pub hook: String,
    #[serde(default = "default_tilde")]
    pub tilde: String,
    #[serde(default = "default_dot")]
    pub dot: String,
    #[serde(default = "default_remove_tone")]
    pub remove_tone: String,

    #[serde(default = "default_empty")]
    pub circumflex: String,
    #[serde(default = "default_empty")]
    pub horn: String,
    #[serde(default = "default_empty")]
    pub breve: String,
    #[serde(default = "default_empty")]
    pub d_mark: String,
    #[serde(default = "default_hook_all")]
    pub hook_all: String,

    #[serde(default)]
    pub roof_a: String,
    #[serde(default)]
    pub roof_e: String,
    #[serde(default)]
    pub roof_o: String,
    #[serde(default)]
    pub horn_o: String,
    #[serde(default)]
    pub horn_u: String,
    #[serde(default)]
    pub breve_a: String,
    #[serde(default)]
    pub d_lower: String,

    #[serde(default = "default_true")]
    pub telex_double_keys: bool,
    #[serde(default = "default_true")]
    pub lone_w: bool,
}

impl Default for CustomKeymapConfig {
    fn default() -> Self {
        Self {
            acute: default_acute(),
            grave: default_grave(),
            hook: default_hook(),
            tilde: default_tilde(),
            dot: default_dot(),
            remove_tone: default_remove_tone(),
            circumflex: String::new(),
            horn: String::new(),
            breve: String::new(),
            d_mark: String::new(),
            hook_all: default_hook_all(),
            roof_a: String::new(),
            roof_e: String::new(),
            roof_o: String::new(),
            horn_o: String::new(),
            horn_u: String::new(),
            breve_a: String::new(),
            d_lower: String::new(),
            telex_double_keys: true,
            lone_w: true,
        }
    }
}

impl CustomKeymapConfig {
    pub fn to_core(&self) -> vietflow_core::CustomKeymap {
        vietflow_core::CustomKeymap {
            acute: self.acute.clone(),
            grave: self.grave.clone(),
            hook: self.hook.clone(),
            tilde: self.tilde.clone(),
            dot: self.dot.clone(),
            remove_tone: self.remove_tone.clone(),
            circumflex: self.circumflex.clone(),
            horn: self.horn.clone(),
            breve: self.breve.clone(),
            d_mark: self.d_mark.clone(),
            hook_all: self.hook_all.clone(),
            roof_a: self.roof_a.clone(),
            roof_e: self.roof_e.clone(),
            roof_o: self.roof_o.clone(),
            horn_o: self.horn_o.clone(),
            horn_u: self.horn_u.clone(),
            breve_a: self.breve_a.clone(),
            d_lower: self.d_lower.clone(),
            telex_double_keys: self.telex_double_keys,
            lone_w: self.lone_w,
        }
    }
}

fn default_excluded_apps() -> Vec<String> {
    vec![
        "valorant.exe".to_string(),
        "cs2.exe".to_string(),
        "league of legends.exe".to_string(),
        "overwatch.exe".to_string(),
        "dota2.exe".to_string(),
    ]
}

fn default_macros() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert("vf".to_string(), "VietFlow".to_string());
    m.insert("vn".to_string(), "Việt Nam".to_string());
    m.insert(
        "chxhcnvn".to_string(),
        "Cộng hòa Xã hội Chủ nghĩa Việt Nam".to_string(),
    );
    m
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            method: 0,
            charset: 0,
            modern_style: true,
            free_marking: true,
            spell_check: true,
            auto_restore: true,
            fix_shift_lag: true,
            consonant_shortcuts: false,
            auto_capitalize: false,
            auto_capitalize_chars: default_auto_capitalize_chars(),
            disable_on_non_us_layout: false,
            enable_macro: true,
            allow_macro_when_disabled: false,
            injection_mode: 0,
            use_clipboard_for_unicode: false,
            start_with_windows: false,
            notification_sound: false,
            typing_timeout_ms: 1500,
            hotkey_mode: default_hotkey_mode(),
            hotkey_settings: default_hotkey_settings(),
            hotkey_converter: default_hotkey_converter(),
            english_keywords: Vec::new(),
            english_keywords_removed: Vec::new(),
            foreign_initials: default_foreign_initials(),
            tray_icon_theme: default_tray_theme(),
            custom_icon_v: String::new(),
            custom_icon_e: String::new(),
            tray_icon_v_fill: true,
            tray_icon_v_fill_color: default_white(),
            tray_icon_v_border: true,
            tray_icon_v_border_color: default_pink(),
            tray_icon_e_fill: true,
            tray_icon_e_fill_color: default_white(),
            tray_icon_e_border: true,
            tray_icon_e_border_color: default_blue(),
            excluded_apps: default_excluded_apps(),
            lock_hotkeys_in_excluded_apps: true,
            app_profiles: HashMap::new(),
            macros: default_macros(),
            custom_keymap: CustomKeymapConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut p = PathBuf::from(appdata);
            p.push(crate::constants::APP_DIR_NAME);
            let _ = fs::create_dir_all(&p);
            p.push(crate::constants::CONFIG_FILE_NAME);
            return p;
        }
        PathBuf::from(crate::constants::CONFIG_FILE_NAME)
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(cfg) = serde_json::from_str(&content) {
                return cfg;
            }
        }
        let def = Self::default();
        def.save();
        def
    }

    pub fn save(&self) {
        let path = Self::config_path();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            let _ = fs::write(path, json);
        }
    }

    pub fn normalized_english_keywords(&self) -> Vec<String> {
        let mut words: Vec<String> = self
            .english_keywords
            .iter()
            .map(|word| word.trim().to_ascii_lowercase())
            .filter(|word| !word.is_empty())
            .collect();
        words.sort();
        words.dedup();
        words
    }

    pub fn normalized_removed_english_keywords(&self) -> Vec<String> {
        let mut words: Vec<String> = self
            .english_keywords_removed
            .iter()
            .map(|word| word.trim().to_ascii_lowercase())
            .filter(|word| !word.is_empty())
            .collect();
        words.sort();
        words.dedup();
        words
    }

    pub fn normalized_foreign_initials(&self) -> Vec<char> {
        let mut initials = Vec::new();
        for c in self.foreign_initials.chars().filter(|c| c.is_alphabetic()) {
            let lower = c.to_ascii_lowercase();
            if !initials.contains(&lower) {
                initials.push(lower);
                if initials.len() == 26 {
                    break;
                }
            }
        }
        if initials.is_empty() {
            vec!['f', 'j', 'w', 'z']
        } else {
            initials
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_profile_overrides_and_hotkey_lock() {
        let config: AppConfig = serde_json::from_str(
            r#"{
                "lock_hotkeys_in_excluded_apps": false,
                "injection_mode": 3,
                "auto_capitalize_chars": "。\n",
                "app_profiles": {
                    "Code.exe": { "injection_mode": 3, "allow_macro": true },
                    "cs2": { "method": 1, "charset": 2 }
                }
            }"#,
        )
        .expect("config must parse");

        assert!(!config.lock_hotkeys_in_excluded_apps);
        assert_eq!(config.injection_mode, 3);
        assert_eq!(config.auto_capitalize_chars, "。\n");
        let code = config.app_profiles.get("Code.exe").unwrap();
        assert_eq!(code.injection_mode, Some(3));
        assert_eq!(code.allow_macro, Some(true));
        assert_eq!(code.method, None);
        let cs2 = config.app_profiles.get("cs2").unwrap();
        assert_eq!(cs2.method, Some(1));
        assert_eq!(cs2.charset, Some(2));
    }
}
