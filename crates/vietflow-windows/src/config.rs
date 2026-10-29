// VietFlow Windows - Configuration Storage
// Licensed under GNU General Public License v3.0 (GPLv3)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub method: u8,       // 0: Telex, 1: SimpleTelex, 2: Vni, 3: Viqr
    pub charset: u8,      // 0: Unicode, 1: Composite, 2: Tcvn3, 3: VniWindows
    pub modern_style: bool,
    pub free_marking: bool,
    pub spell_check: bool,
    pub auto_restore: bool,
    pub fix_shift_lag: bool,
    pub consonant_shortcuts: bool,
    pub auto_capitalize: bool,
    pub enable_macro: bool,
    pub allow_macro_when_disabled: bool,
    pub use_clipboard_for_unicode: bool,
    pub start_with_windows: bool,
    pub notification_sound: bool,
    pub typing_timeout_ms: u64,
    pub hotkey_mode: String, // "CtrlShift", "AltZ", "CtrlSpace", "ShiftSpace", "AltShift", "CapsLock"
    pub excluded_apps: Vec<String>,
    pub macros: HashMap<String, String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        let mut macros = HashMap::new();
        macros.insert("vf".to_string(), "VietFlow".to_string());
        macros.insert("vn".to_string(), "Việt Nam".to_string());
        macros.insert(
            "chxhcnvn".to_string(),
            "Cộng hòa Xã hội Chủ nghĩa Việt Nam".to_string(),
        );

        Self {
            enabled: true,
            method: 0,
            charset: 0,
            modern_style: false,
            free_marking: true,
            spell_check: true,
            auto_restore: true,
            fix_shift_lag: true,
            consonant_shortcuts: false,
            auto_capitalize: false,
            enable_macro: false,
            allow_macro_when_disabled: false,
            use_clipboard_for_unicode: false,
            start_with_windows: false,
            notification_sound: true,
            typing_timeout_ms: 3000,
            hotkey_mode: "CtrlShift".to_string(),
            excluded_apps: vec![
                "valorant.exe".to_string(),
                "cs2.exe".to_string(),
                "League of Legends.exe".to_string(),
                "Overwatch.exe".to_string(),
                "dota2.exe".to_string(),
            ],
            macros,
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
}
