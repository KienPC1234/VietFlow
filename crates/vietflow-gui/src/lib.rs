// VietFlow Unified Configuration GUI Library
// Licensed under GNU General Public License v3.0 (GPLv3)

slint::include_modules!();

use arboard::Clipboard;
use serde::{Deserialize, Serialize};
use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use vietflow_core::converter::convert_charset;
use vietflow_core::models::CharsetId;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub enabled: bool,
    pub method: u8,
    pub charset: u8,
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
    pub hotkey_mode: String,
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

pub fn get_config_path() -> PathBuf {
    if let Ok(app_data) = std::env::var("APPDATA") {
        let dir = PathBuf::from(app_data).join("VietFlow");
        let _ = fs::create_dir_all(&dir);
        dir.join("vietflow_config.json")
    } else {
        PathBuf::from("vietflow_config.json")
    }
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if let Ok(data) = fs::read_to_string(&path) {
        if let Ok(cfg) = serde_json::from_str(&data) {
            return cfg;
        }
    }
    AppConfig::default()
}

pub fn save_config_to_disk(cfg: &AppConfig) {
    let path = get_config_path();
    if let Ok(json) = serde_json::to_string_pretty(cfg) {
        let _ = fs::write(path, json);
    }
}

fn map_hotkey_to_index(mode: &str) -> i32 {
    match mode {
        "CtrlShift" => 0,
        "AltZ" => 1,
        "CtrlSpace" => 2,
        "AltShift" => 3,
        _ => 0,
    }
}

fn map_index_to_hotkey(idx: i32) -> String {
    match idx {
        0 => "CtrlShift".to_string(),
        1 => "AltZ".to_string(),
        2 => "CtrlSpace".to_string(),
        3 => "AltShift".to_string(),
        _ => "CtrlShift".to_string(),
    }
}

fn map_int_to_charset_id(idx: i32) -> CharsetId {
    match idx {
        0 => CharsetId::Unicode,
        1 => CharsetId::UnicodeComposite,
        2 => CharsetId::Tcvn3,
        3 => CharsetId::VniWindows,
        _ => CharsetId::Unicode,
    }
}

fn sync_to_ui(window: &AppWindow, cfg: &AppConfig) {
    window.set_is_enabled(cfg.enabled);
    window.set_method_index(cfg.method as i32);
    window.set_charset_index(cfg.charset as i32);
    window.set_hotkey_index(map_hotkey_to_index(&cfg.hotkey_mode));
    window.set_start_with_windows(cfg.start_with_windows);
    window.set_notification_sound(cfg.notification_sound);

    window.set_spell_check(cfg.spell_check);
    window.set_auto_restore(cfg.auto_restore);
    window.set_modern_style(cfg.modern_style);
    window.set_free_marking(cfg.free_marking);
    window.set_fix_shift_lag(cfg.fix_shift_lag);
    window.set_consonant_shortcuts(cfg.consonant_shortcuts);
    window.set_auto_capitalize(cfg.auto_capitalize);
    window.set_use_clipboard(cfg.use_clipboard_for_unicode);

    window.set_enable_macro(cfg.enable_macro);
    window.set_allow_macro_when_disabled(cfg.allow_macro_when_disabled);

    // Sync macros
    let mut entries = Vec::new();
    let mut sorted_keys: Vec<_> = cfg.macros.keys().collect();
    sorted_keys.sort();
    for k in sorted_keys {
        entries.push(MacroEntry {
            key: SharedString::from(k),
            value: SharedString::from(&cfg.macros[k]),
        });
    }
    window.set_macro_list(ModelRc::new(VecModel::from(entries)));

    // Sync excluded apps
    let apps: Vec<SharedString> = cfg
        .excluded_apps
        .iter()
        .map(|s| SharedString::from(s))
        .collect();
    window.set_excluded_apps(ModelRc::new(VecModel::from(apps)));
}

fn read_from_ui(window: &AppWindow, cfg: &mut AppConfig) {
    cfg.enabled = window.get_is_enabled();
    cfg.method = window.get_method_index() as u8;
    cfg.charset = window.get_charset_index() as u8;
    cfg.hotkey_mode = map_index_to_hotkey(window.get_hotkey_index());
    cfg.start_with_windows = window.get_start_with_windows();
    cfg.notification_sound = window.get_notification_sound();

    cfg.spell_check = window.get_spell_check();
    cfg.auto_restore = window.get_auto_restore();
    cfg.modern_style = window.get_modern_style();
    cfg.free_marking = window.get_free_marking();
    cfg.fix_shift_lag = window.get_fix_shift_lag();
    cfg.consonant_shortcuts = window.get_consonant_shortcuts();
    cfg.auto_capitalize = window.get_auto_capitalize();
    cfg.use_clipboard_for_unicode = window.get_use_clipboard();

    cfg.enable_macro = window.get_enable_macro();
    cfg.allow_macro_when_disabled = window.get_allow_macro_when_disabled();
}

pub fn run_gui(_convert_mode: bool) -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    let config_state = Rc::new(std::cell::RefCell::new(load_config()));

    sync_to_ui(&window, &config_state.borrow());

    // Save config callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_save_config(move || {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            read_from_ui(&w, &mut cfg);
            save_config_to_disk(&cfg);
        }
    });

    // Reset defaults callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_reset_defaults(move || {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            *cfg = AppConfig::default();
            save_config_to_disk(&cfg);
            sync_to_ui(&w, &cfg);
        }
    });

    // Toggle live mode callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_toggle_mode(move || {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            cfg.enabled = !cfg.enabled;
            w.set_is_enabled(cfg.enabled);
            save_config_to_disk(&cfg);
        }
    });

    // Add macro callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_add_macro(move |key, value| {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            cfg.macros.insert(key.to_string(), value.to_string());
            save_config_to_disk(&cfg);
            sync_to_ui(&w, &cfg);
        }
    });

    // Delete macro callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_delete_macro(move |idx| {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            let mut sorted_keys: Vec<_> = cfg.macros.keys().cloned().collect();
            sorted_keys.sort();
            if idx >= 0 && (idx as usize) < sorted_keys.len() {
                let key = &sorted_keys[idx as usize];
                cfg.macros.remove(key);
                save_config_to_disk(&cfg);
                sync_to_ui(&w, &cfg);
            }
        }
    });

    // Add excluded app callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_add_excluded_app(move |app_name| {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            let name = app_name.to_string();
            if !cfg.excluded_apps.contains(&name) {
                cfg.excluded_apps.push(name);
                save_config_to_disk(&cfg);
                sync_to_ui(&w, &cfg);
            }
        }
    });

    // Delete excluded app callback
    let w_weak = window.as_weak();
    let cfg_clone = config_state.clone();
    window.on_delete_excluded_app(move |idx| {
        if let Some(w) = w_weak.upgrade() {
            let mut cfg = cfg_clone.borrow_mut();
            if idx >= 0 && (idx as usize) < cfg.excluded_apps.len() {
                cfg.excluded_apps.remove(idx as usize);
                save_config_to_disk(&cfg);
                sync_to_ui(&w, &cfg);
            }
        }
    });

    // Convert text callback
    window.on_convert_text(move |input, src_idx, dst_idx| {
        let src = map_int_to_charset_id(src_idx);
        let dst = map_int_to_charset_id(dst_idx);
        let converted = convert_charset(&input, src, dst);
        SharedString::from(converted)
    });

    // Convert clipboard callback
    let w_weak = window.as_weak();
    window.on_convert_clipboard(move |src_idx, dst_idx| {
        if let Some(w) = w_weak.upgrade() {
            if let Ok(mut cb) = Clipboard::new() {
                if let Ok(text) = cb.get_text() {
                    let src = map_int_to_charset_id(src_idx);
                    let dst = map_int_to_charset_id(dst_idx);
                    let converted = convert_charset(&text, src, dst);
                    if cb.set_text(converted).is_ok() {
                        w.set_conv_status(SharedString::from("Đã chuyển mã Clipboard thành công!"));
                        return;
                    }
                }
            }
            w.set_conv_status(SharedString::from("Lỗi: Không thể truy cập Clipboard!"));
        }
    });

    window.run()
}

