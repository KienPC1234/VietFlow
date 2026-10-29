// VietFlow Core - C-ABI Interface for FFI (Linux Fcitx5 / Windows Native / C++)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ffi::c_void;
use crate::engine::VietFlowEngine;
use crate::models::{CharsetId, InputMethod};

#[no_mangle]
pub extern "C" fn vf_engine_create() -> *mut c_void {
    let engine = Box::new(VietFlowEngine::default());
    Box::into_raw(engine) as *mut c_void
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_destroy(engine: *mut c_void) {
    if !engine.is_null() {
        drop(Box::from_raw(engine as *mut VietFlowEngine));
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_reset(engine: *mut c_void) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_method(engine: *mut c_void, method: u8) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.method = match method {
            0 => InputMethod::Telex,
            1 => InputMethod::SimpleTelex,
            2 => InputMethod::Vni,
            3 => InputMethod::Viqr,
            _ => InputMethod::Telex,
        };
        engine.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_charset(engine: *mut c_void, charset: u8) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.charset = match charset {
            0 => CharsetId::Unicode,
            1 => CharsetId::UnicodeComposite,
            2 => CharsetId::Tcvn3,
            3 => CharsetId::VniWindows,
            4 => CharsetId::Viqr,
            _ => CharsetId::Unicode,
        };
        engine.reset();
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_spell_check(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.spell_check = enabled;
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_modern_style(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.modern_style = enabled;
    }
}

#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_auto_restore(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.auto_non_vn_restore = enabled;
    }
}

/// Process a single character.
/// Output is UTF-8 encoded into `out_utf8`.
/// Returns true if a key was processed, writes output byte length to `out_len`
/// and number of backspaces to `out_backspaces`.
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
