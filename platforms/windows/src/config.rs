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
    #[serde(default)]
    pub english_keywords: Vec<String>,
    #[serde(default = "default_foreign_initials")]
    pub foreign_initials: String,
    pub excluded_apps: Vec<String>,
    pub macros: HashMap<String, String>,
}

fn default_foreign_initials() -> String {
    "fjwz".to_string()
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
            english_keywords: Vec::new(),
            foreign_initials: "fjwz".to_string(),
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

    /// Lowercase, de-duplicated, sorted custom English/foreign keywords.
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

    /// Letters that suppress Vietnamese marks on foreign-looking words.
    /// Falls back to the engine default when the user clears the field.
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
    fn default_lists_fall_back_to_engine_defaults() {
        let config = AppConfig::default();
        assert_eq!(config.normalized_foreign_initials(), vec!['f', 'j', 'w', 'z']);
        assert!(config.normalized_english_keywords().is_empty());
    }

    #[test]
    fn custom_lists_are_normalized_and_deduplicated() {
        let mut config = AppConfig::default();
        config.foreign_initials = "B, f b".to_string();
        config.english_keywords = vec![
            " Case ".to_string(),
            String::new(),
            "case".to_string(),
            "API".to_string(),
        ];

        assert_eq!(config.normalized_foreign_initials(), vec!['b', 'f']);
        assert_eq!(
            config.normalized_english_keywords(),
            vec!["api".to_string(), "case".to_string()]
        );
    }
}
