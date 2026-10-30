// VietFlow Windows - Configuration Storage
// Licensed under GNU General Public License v3.0 (GPLv3)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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
    /// Disable Vietnamese input when the foreground window's keyboard layout
    /// is not US (EVKey: "Tắt TV khi Layout bàn phím khác US").
    #[serde(default)]
    pub disable_on_non_us_layout: bool,
    #[serde(default = "default_true")]
    pub enable_macro: bool,
    #[serde(default)]
    pub allow_macro_when_disabled: bool,
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

    #[serde(default)]
    pub english_keywords: Vec<String>,
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
    #[serde(default = "default_macros")]
    pub macros: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u64 {
    1500
}
fn default_hotkey_mode() -> String {
    "CtrlShift".to_string()
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
            disable_on_non_us_layout: false,
            enable_macro: true,
            allow_macro_when_disabled: false,
            use_clipboard_for_unicode: false,
            start_with_windows: false,
            notification_sound: false,
            typing_timeout_ms: 1500,
            hotkey_mode: default_hotkey_mode(),
            english_keywords: Vec::new(),
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
            macros: default_macros(),
        }
    }
}

impl AppConfig {
    pub fn config_path() -> PathBuf {
        if let Ok(appdata) = std::env::var("APPDATA") {
            let mut p = PathBuf::from(appdata);
            p.push("VietFlow");
            let _ = fs::create_dir_all(&p);
            p.push("vietflow_config.json");
            return p;
        }
        PathBuf::from("vietflow_config.json")
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
