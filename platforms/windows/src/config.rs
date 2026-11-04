// VietFlow Windows - Configuration Storage
// Licensed under GNU General Public License v3.0 (GPLv3)

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

/// The daemon checks this path every 250 ms. Building a PathBuf and calling
/// create_dir_all on every timer tick was unnecessary allocator and disk work.
static CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();
static CUSTOM_CONFIG_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn set_custom_config_path(path: PathBuf) {
    let _ = CUSTOM_CONFIG_PATH.set(path);
}

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
    #[serde(default = "default_true")]
    pub auto_switch_by_typing: bool,
    #[serde(default = "default_true")]
    pub auto_detect_game_mode: bool,
    #[serde(default = "default_true")]
    pub pause_auto_after_manual: bool,
    /// Disable Vietnamese input when the foreground window's keyboard layout
    /// is not US (EVKey: "Tắt TV khi Layout bàn phím khác US").
    #[serde(default)]
    pub disable_on_non_us_layout: bool,
    /// Disable Vietnamese input in ANSI-only windows (for example the VBA IDE),
    /// where Unicode characters cannot be displayed correctly.
    #[serde(default = "default_true")]
    pub disable_on_non_unicode_window: bool,
    /// Macros are opt-in: the default Vietnamese typing experience must never
    /// replace ordinary words such as `vn` or `vf` without explicit consent.
    #[serde(default)]
    pub enable_macro: bool,
    #[serde(default)]
    pub allow_macro_when_disabled: bool,
    /// 0: Auto, 1: Unicode SendInput, 2: scan-code, 4: safe paced.
    /// Legacy 3 (clipboard) is migrated to Auto.
    #[serde(default)]
    pub injection_mode: u8,
    #[serde(default)]
    pub use_tsf: bool,
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

    #[serde(default)]
    pub english_keywords: Vec<String>,
    #[serde(default)]
    pub english_keywords_removed: Vec<String>,
    #[serde(default = "default_foreign_initials")]
    pub foreign_initials: String,
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
    /// When enabled, configured patterns of 4+ characters can match processes starting with that prefix.
    /// Disabled by default to prevent false-positive matches (e.g. 'code' matching 'codeblocks.exe').
    /// Users can also use explicit wildcards like 'roblox*' or '*terminal*'.
    #[serde(default)]
    pub smart_prefix_matching: bool,
    /// Lock V/E, settings and converter hotkeys while an excluded app is in
    /// front. The tray/menu override remains available.
    #[serde(default = "default_true")]
    pub lock_hotkeys_in_excluded_apps: bool,
    /// Remember Vietnamese/English typing mode per application and switch dynamically.
    #[serde(default)]
    pub remember_per_app_mode: bool,
    /// Automatically exclude terminal and console applications from Vietnamese typing.
    #[serde(default)]
    pub auto_exclude_apps: bool,
    /// Dynamic per-application mode memory map (process_name.toLowerCase -> is_vietnamese).
    #[serde(default)]
    pub per_app_modes: HashMap<String, bool>,
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
fn default_timeout() -> u64 {
    0
}
fn default_hotkey_mode() -> String {
    "CtrlShift".to_string()
}
fn default_hotkey_settings() -> String {
    "Ctrl + Shift + F5".to_string()
}
fn default_foreign_initials() -> String {
    "fjwz".to_string()
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
            auto_switch_by_typing: true,
            auto_detect_game_mode: false,
            pause_auto_after_manual: true,
            disable_on_non_us_layout: false,
            disable_on_non_unicode_window: true,
            enable_macro: false,
            allow_macro_when_disabled: false,
            injection_mode: 0,
            use_tsf: false,
            start_with_windows: false,
            notification_sound: false,
            typing_timeout_ms: default_timeout(),
            hotkey_mode: default_hotkey_mode(),
            hotkey_settings: default_hotkey_settings(),
            english_keywords: Vec::new(),
            english_keywords_removed: Vec::new(),
            foreign_initials: default_foreign_initials(),
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
            smart_prefix_matching: false,
            lock_hotkeys_in_excluded_apps: true,
            remember_per_app_mode: false,
            auto_exclude_apps: false,
            per_app_modes: HashMap::new(),
            app_profiles: HashMap::new(),
            macros: default_macros(),
            custom_keymap: CustomKeymapConfig::default(),
        }
    }
}

/// Removed Clipboard mode (3) becomes Auto. Safe remains 4.
pub fn normalize_injection_mode(mode: u8) -> u8 {
    match mode {
        0..=2 | 4 => mode,
        _ => 0,
    }
}

fn is_dir_writable(dir: &std::path::Path) -> bool {
    let test_file = dir.join(format!(".w_test_{}.tmp", std::process::id()));
    match fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(&test_file)
    {
        Ok(f) => {
            drop(f);
            let _ = fs::remove_file(&test_file);
            true
        }
        Err(_) => false,
    }
}

fn get_program_data_dir() -> Option<PathBuf> {
    std::env::var("ProgramData")
        .ok()
        .map(|pd| PathBuf::from(pd).join(crate::constants::APP_DIR_NAME))
}

fn get_app_data_dir() -> Option<PathBuf> {
    std::env::var("APPDATA")
        .ok()
        .map(|ad| PathBuf::from(ad).join(crate::constants::APP_DIR_NAME))
}

fn get_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|p| p.to_path_buf()))
}

fn shared_common_config_path() -> Option<PathBuf> {
    // 1. Common Application Data (%ProgramData%\VietFlow\vietflow_config.json) - persistent across MSI upgrades
    if let Some(p) = get_program_data_dir() {
        if (p.is_dir() || fs::create_dir_all(&p).is_ok()) && is_dir_writable(&p) {
            return Some(p.join(crate::constants::CONFIG_FILE_NAME));
        }
    }
    // 2. Per-user Application Data (%APPDATA%\VietFlow\vietflow_config.json)
    if let Some(p) = get_app_data_dir() {
        if p.is_dir() || fs::create_dir_all(&p).is_ok() {
            return Some(p.join(crate::constants::CONFIG_FILE_NAME));
        }
    }
    None
}

impl AppConfig {
    pub fn normalize(&mut self) {
        self.injection_mode = normalize_injection_mode(self.injection_mode);
        for profile in self.app_profiles.values_mut() {
            if let Some(mode) = profile.injection_mode {
                profile.injection_mode = Some(normalize_injection_mode(mode));
            }
        }
    }

    fn build_config_path() -> PathBuf {
        if let Some(custom) = CUSTOM_CONFIG_PATH.get() {
            return custom.clone();
        }

        // If installed via MSI:
        if crate::updater::is_installed_via_msi() {
            // NEVER store active mutable config in Program Files.
            // Canonical persistent location is %ProgramData% or %APPDATA%.
            let target_path = if let Some(shared) = shared_common_config_path() {
                shared
            } else if let Some(appdata) = get_app_data_dir() {
                let _ = fs::create_dir_all(&appdata);
                appdata.join(crate::constants::CONFIG_FILE_NAME)
            } else {
                PathBuf::from(crate::constants::CONFIG_FILE_NAME)
            };

            // Migration: If target does not exist or is empty, look for legacy configs to preserve
            let target_exists = target_path.is_file()
                && fs::metadata(&target_path).map(|m| m.len()).unwrap_or(0) > 0;
            if !target_exists {
                let mut candidates = Vec::new();
                // 1. Check legacy Program Files location
                if let Some(exe_dir) = get_exe_dir() {
                    candidates.push(exe_dir.join(crate::constants::CONFIG_FILE_NAME));
                }
                // 2. Check %APPDATA% (if target was %ProgramData%)
                if let Some(ad) = get_app_data_dir() {
                    candidates.push(ad.join(crate::constants::CONFIG_FILE_NAME));
                }
                // 3. Check %ProgramData% (if target was %APPDATA%)
                if let Some(pd) = get_program_data_dir() {
                    candidates.push(pd.join(crate::constants::CONFIG_FILE_NAME));
                }
                // 4. Check backup
                candidates.push(target_path.with_extension("json.bak"));

                for candidate in candidates {
                    if candidate != target_path && candidate.is_file() {
                        if let Ok(content) = fs::read_to_string(&candidate) {
                            if serde_json::from_str::<AppConfig>(&content).is_ok() {
                                let _ = Self::save_atomic(&target_path, &content);
                                break;
                            }
                        }
                    }
                }
            }

            return target_path;
        }

        // Portable mode: if local config.json exists beside exe and directory is writable, prioritize it
        if let Some(exe_dir) = get_exe_dir() {
            let local_cfg = exe_dir.join(crate::constants::CONFIG_FILE_NAME);
            if local_cfg.is_file() || is_dir_writable(&exe_dir) {
                return local_cfg;
            }
        }

        // Shared common fallback for portable mode
        if let Some(shared) = shared_common_config_path() {
            return shared;
        }

        PathBuf::from(crate::constants::CONFIG_FILE_NAME)
    }

    /// Cached path for the hot 250 ms config-mtime check.
    pub fn config_path_ref() -> &'static PathBuf {
        CONFIG_PATH.get_or_init(Self::build_config_path)
    }

    pub fn config_path() -> PathBuf {
        Self::config_path_ref().clone()
    }

    pub fn save_atomic(target_path: &std::path::Path, json: &str) -> bool {
        let parent = match target_path.parent() {
            Some(p) => p,
            None => return false,
        };
        if !parent.is_dir() && fs::create_dir_all(parent).is_err() {
            return false;
        }

        let pid = std::process::id();
        let tmp_path = parent.join(format!("{}.tmp.{}", crate::constants::CONFIG_FILE_NAME, pid));
        let bak_path = target_path.with_extension("json.bak");

        use std::io::Write;
        let write_ok = (|| -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(&tmp_path)?;
            file.write_all(json.as_bytes())?;
            file.sync_all()?;
            Ok(())
        })();

        if write_ok.is_err() {
            let _ = fs::remove_file(&tmp_path);
            return false;
        }

        // Backup existing valid file before replacing
        if target_path.is_file() {
            let _ = fs::copy(target_path, &bak_path);
        }

        // Replace target atomically
        if fs::rename(&tmp_path, target_path).is_err() {
            if fs::copy(&tmp_path, target_path).is_ok() {
                let _ = fs::remove_file(&tmp_path);
                return true;
            }
            let _ = fs::remove_file(&tmp_path);
            return false;
        }

        true
    }

    pub fn load() -> Self {
        let path = Self::config_path_ref();

        // 1. Try reading the active config file
        if path.is_file() {
            if let Ok(content) = fs::read_to_string(path) {
                if !content.trim().is_empty() {
                    match serde_json::from_str::<AppConfig>(&content) {
                        Ok(mut cfg) => {
                            cfg.normalize();
                            let bak = path.with_extension("json.bak");
                            if !bak.is_file() {
                                let _ = fs::copy(path, &bak);
                            }
                            return cfg;
                        }
                        Err(_) => {
                            // Quarantine corrupted file to avoid permanent data loss
                            let ts = std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            let corrupted_path =
                                path.with_extension(format!("json.corrupted.{}", ts));
                            let _ = fs::copy(path, &corrupted_path);

                            // Attempt recovery from backup
                            let bak = path.with_extension("json.bak");
                            if bak.is_file() {
                                if let Ok(bak_content) = fs::read_to_string(&bak) {
                                    if let Ok(mut cfg) =
                                        serde_json::from_str::<AppConfig>(&bak_content)
                                    {
                                        cfg.normalize();
                                        let _ = fs::copy(&bak, path);
                                        return cfg;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // 2. If active path is missing or empty, search fallback candidate locations
        let mut candidates = Vec::new();
        if let Some(p) = shared_common_config_path() {
            if &p != path {
                candidates.push(p);
            }
        }
        if let Some(ad) = get_app_data_dir() {
            let p = ad.join(crate::constants::CONFIG_FILE_NAME);
            if &p != path {
                candidates.push(p);
            }
        }
        if let Some(exe_dir) = get_exe_dir() {
            let p = exe_dir.join(crate::constants::CONFIG_FILE_NAME);
            if &p != path {
                candidates.push(p);
            }
        }
        let bak = path.with_extension("json.bak");
        candidates.push(bak);

        for candidate in candidates {
            if candidate.is_file() {
                if let Ok(content) = fs::read_to_string(&candidate) {
                    if let Ok(mut cfg) = serde_json::from_str::<AppConfig>(&content) {
                        cfg.normalize();
                        let _ = Self::save_atomic(path, &content);
                        return cfg;
                    }
                }
            }
        }

        // 3. Defaults if no valid config could be located
        let def = Self::default();
        def.save();
        def
    }

    pub fn save(&self) {
        let path = Self::config_path_ref();
        if let Ok(json) = serde_json::to_string_pretty(self) {
            if !Self::save_atomic(path, &json) {
                if let Some(ad) = get_app_data_dir() {
                    let alt_path = ad.join(crate::constants::CONFIG_FILE_NAME);
                    if &alt_path != path {
                        let _ = Self::save_atomic(&alt_path, &json);
                    }
                }
            }
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

    /// True when a config reload needs to rebuild the tray HICONs. Keeping the
    /// existing icons avoids two glyph renders on every save.
    pub fn icon_settings_differ(&self, other: &Self) -> bool {
        self.custom_icon_v != other.custom_icon_v
            || self.custom_icon_e != other.custom_icon_e
            || self.tray_icon_v_fill != other.tray_icon_v_fill
            || self.tray_icon_v_fill_color != other.tray_icon_v_fill_color
            || self.tray_icon_v_border != other.tray_icon_v_border
            || self.tray_icon_v_border_color != other.tray_icon_v_border_color
            || self.tray_icon_e_fill != other.tray_icon_e_fill
            || self.tray_icon_e_fill_color != other.tray_icon_e_fill_color
            || self.tray_icon_e_border != other.tray_icon_e_border
            || self.tray_icon_e_border_color != other.tray_icon_e_border_color
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
        let mut config: AppConfig = serde_json::from_str(
            r#"{
                "lock_hotkeys_in_excluded_apps": false,
                "injection_mode": 3,
                "auto_switch_by_typing": true,
                "app_profiles": {
                    "Code.exe": { "injection_mode": 3, "allow_macro": true },
                    "cs2": { "method": 1, "charset": 2 }
                }
            }"#,
        )
        .expect("config must parse");
        config.normalize();

        assert!(!config.lock_hotkeys_in_excluded_apps);
        assert_eq!(config.injection_mode, 0);
        assert!(config.auto_switch_by_typing);
        let code = config.app_profiles.get("Code.exe").unwrap();
        assert_eq!(code.injection_mode, Some(0));
        assert_eq!(code.allow_macro, Some(true));
        assert_eq!(code.method, None);
        let cs2 = config.app_profiles.get("cs2").unwrap();
        assert_eq!(cs2.method, Some(1));
        assert_eq!(cs2.charset, Some(2));
    }

    #[test]
    fn macros_are_opt_in() {
        assert!(!AppConfig::default().enable_macro);
        let missing: AppConfig = serde_json::from_str("{}").expect("empty config");
        assert!(!missing.enable_macro);

        let enabled: AppConfig = serde_json::from_str(r#"{"enable_macro":true}"#)
            .expect("explicit macro config");
        assert!(enabled.enable_macro);
    }

    #[test]
    fn detects_only_icon_related_changes() {
        let mut first = AppConfig::default();
        let mut second = AppConfig::default();
        assert!(!first.icon_settings_differ(&second));

        second.notification_sound = true;
        assert!(!first.icon_settings_differ(&second));

        second.tray_icon_v_fill_color = "#123456".to_string();
        assert!(first.icon_settings_differ(&second));

        first.tray_icon_v_fill_color = "#123456".to_string();
        assert!(!first.icon_settings_differ(&second));
    }

    #[test]
    fn atomic_save_and_backup_creation() {
        let temp_dir = std::env::temp_dir().join(format!("vietflow_cfg_test_{}", std::process::id()));
        let _ = fs::create_dir_all(&temp_dir);
        let cfg_path = temp_dir.join("vietflow_config.json");
        let bak_path = temp_dir.join("vietflow_config.json.bak");

        let mut config = AppConfig::default();
        config.method = 1;
        let json1 = serde_json::to_string_pretty(&config).unwrap();
        assert!(AppConfig::save_atomic(&cfg_path, &json1));
        assert!(cfg_path.is_file());
        assert!(!bak_path.is_file());

        config.method = 2;
        let json2 = serde_json::to_string_pretty(&config).unwrap();
        assert!(AppConfig::save_atomic(&cfg_path, &json2));
        assert!(cfg_path.is_file());
        assert!(bak_path.is_file());

        let bak_content = fs::read_to_string(&bak_path).unwrap();
        let bak_cfg: AppConfig = serde_json::from_str(&bak_content).unwrap();
        assert_eq!(bak_cfg.method, 1);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
