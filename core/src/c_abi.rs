// VietFlow Core - C-ABI Interface for FFI (Linux Fcitx5 / Windows Native / C++)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ffi::c_void;
use crate::english::default_english_keywords;
use crate::engine::VietFlowEngine;
use crate::models::{CharsetId, InputMethod};

/// Newline-separated default English keyword list (UTF-8). The settings UI
/// can fetch it for the keyword editor without duplicating the list in C++.
///
/// # Safety
/// Caller must ensure `out` (if non-null) points to at least `out_cap` writable bytes.
#[no_mangle]
pub unsafe extern "C" fn vf_english_keywords(out: *mut u8, out_cap: usize) -> usize {
    let mut text = default_english_keywords().join("\n");
    text.push('\n');
    let bytes = text.as_bytes();

    if !out.is_null() && out_cap > 0 {
        let to_write = bytes.len().min(out_cap);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, to_write);
    }

    bytes.len()
}

#[no_mangle]
pub extern "C" fn vf_engine_create() -> *mut c_void {
    let engine = Box::new(VietFlowEngine::default());
    Box::into_raw(engine) as *mut c_void
}

/// Destroy an engine instance created via `vf_engine_create`.
///
/// # Safety
/// `engine` must be a valid pointer returned by `vf_engine_create` or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_destroy(engine: *mut c_void) {
    if !engine.is_null() {
        drop(Box::from_raw(engine as *mut VietFlowEngine));
    }
}

/// Reset the engine word buffer and state.
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_reset(engine: *mut c_void) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.reset();
    }
}

/// Configure the input method (0: Telex, 1: VNI, 2: VIQR, 3: Simple, 4: Simple 2, 5: Hybrid, 6: Custom).
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_method(engine: *mut c_void, method: u8) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        let new_method = match method {
            0 => InputMethod::Telex,
            1 => InputMethod::Vni,
            2 => InputMethod::Viqr,
            3 => InputMethod::SimpleTelex,
            4 => InputMethod::TelexSimple2,
            5 => InputMethod::TelexVni,
            6 => InputMethod::UserCustom,
            _ => InputMethod::Telex,
        };
        if engine.options.method != new_method {
            engine.options.method = new_method;
            engine.reset();
        }
    }
}

/// Configure the output charset (0: Unicode, 1: Composite, 2: TCVN3, 3: VNI Windows, 4: VIQR).
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_charset(engine: *mut c_void, charset: u8) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        let new_charset = match charset {
            0 => CharsetId::Unicode,
            1 => CharsetId::UnicodeComposite,
            2 => CharsetId::Tcvn3,
            3 => CharsetId::VniWindows,
            4 => CharsetId::Viqr,
            _ => CharsetId::Unicode,
        };
        if engine.charset != new_charset {
            engine.charset = new_charset;
            engine.reset();
        }
    }
}

/// Enable or disable spell-checking rules.
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_spell_check(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        if engine.options.spell_check != enabled {
            engine.options.spell_check = enabled;
            engine.reset();
        }
    }
}

/// Enable or disable modern orthography tone placement.
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_modern_style(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        if engine.options.modern_style != enabled {
            engine.options.modern_style = enabled;
            engine.reset();
        }
    }
}

/// Enable or disable auto-restoration of non-Vietnamese syllables.
///
/// # Safety
/// `engine` must be a valid engine pointer or null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_auto_restore(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        if engine.options.auto_non_vn_restore != enabled {
            engine.options.auto_non_vn_restore = enabled;
            engine.reset();
        }
    }
}

/// Process a single character.
/// Output is UTF-8 encoded into `out_utf8`.
/// Returns true if a key was processed, writes output byte length to `out_len`
/// and number of backspaces to `out_backspaces`.
///
/// # Safety
/// `engine` must be a valid engine pointer. Output buffers and pointers must be valid if non-null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_process_key(
    engine: *mut c_void,
    ch: u32,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
    out_backspaces: *mut usize,
) -> bool {
    let engine = match (engine as *mut VietFlowEngine).as_mut() {
        Some(e) => e,
        None => return false,
    };

    let c = match char::from_u32(ch) {
        Some(c) => c,
        None => return false,
    };

    let (backspaces, text) = engine.process_key(c);

    if !out_backspaces.is_null() {
        *out_backspaces = backspaces;
    }

    let bytes = text.as_bytes();
    let to_write = bytes.len().min(max_out_len);

    if !out_utf8.is_null() && to_write > 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, to_write);
    }

    if !out_len.is_null() {
        *out_len = to_write;
    }

    true
}

/// Returns the current preedit text for in-line IME composition.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_get_preedit(
    engine: *mut c_void,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
) -> bool {
    let engine = match (engine as *mut VietFlowEngine).as_mut() {
        Some(e) => e,
        None => return false,
    };

    let text = engine.current_preedit_text();
    let bytes = text.as_bytes();
    let to_write = bytes.len().min(max_out_len);

    if !out_utf8.is_null() && to_write > 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, to_write);
    }
    if !out_len.is_null() {
        *out_len = to_write;
    }
    true
}

/// Finalizes composition on a word break delimiter (Space, punctuation, Enter).
#[no_mangle]
pub unsafe extern "C" fn vf_engine_finish_on_break(
    engine: *mut c_void,
    break_char: u32,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
) -> bool {
    let engine = match (engine as *mut VietFlowEngine).as_mut() {
        Some(e) => e,
        None => return false,
    };

    let c = char::from_u32(break_char).unwrap_or(' ');
    let text = engine.finish_composition_on_break(c);
    let bytes = text.as_bytes();
    let to_write = bytes.len().min(max_out_len);

    if !out_utf8.is_null() && to_write > 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, to_write);
    }
    if !out_len.is_null() {
        *out_len = to_write;
    }
    true
}

/// Process backspace in composition mode and return the updated preedit string.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_process_backspace(
    engine: *mut c_void,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
) -> bool {
    let engine = match (engine as *mut VietFlowEngine).as_mut() {
        Some(e) => e,
        None => return false,
    };

    let _ = engine.process_key('\x08');
    let text = engine.current_preedit_text();
    let bytes = text.as_bytes();
    let to_write = bytes.len().min(max_out_len);

    if !out_utf8.is_null() && to_write > 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, to_write);
    }
    if !out_len.is_null() {
        *out_len = to_write;
    }
    true
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_free_marking(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        if engine.options.free_marking != enabled {
            engine.options.free_marking = enabled;
            engine.reset();
        }
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_enable_macro(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.enable_macro = enabled;
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_auto_switch_by_typing(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.auto_switch_by_typing = enabled;
    }
}
