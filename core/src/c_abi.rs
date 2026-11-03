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
/// Copies a UTF-8 string into a destination buffer without splitting multi-byte characters.
///
/// # Contract
/// - If `out_utf8` is null or `max_out_len == 0`: No bytes are written. If `out_len` is non-null,
///   it receives the full required length (`bytes.len()`) in bytes. Returns `true`.
/// - If `out_utf8` is non-null and `max_out_len > 0`: Copies up to `max_out_len` bytes, truncating
///   at the nearest preceding UTF-8 character boundary so that the written prefix is always
///   valid UTF-8. If `out_len` is non-null, it receives the exact number of bytes written.
///   Returns `true` if the entire string was written, or `false` if truncated due to small buffer.
unsafe fn copy_utf8_to_buffer(
    text: &str,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
) -> bool {
    let bytes = text.as_bytes();
    let full_len = bytes.len();

    if out_utf8.is_null() || max_out_len == 0 {
        if !out_len.is_null() {
            *out_len = full_len;
        }
        return true;
    }

    let mut safe_len = full_len.min(max_out_len);
    while safe_len > 0 && !text.is_char_boundary(safe_len) {
        safe_len -= 1;
    }

    if safe_len > 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, safe_len);
    }

    if !out_len.is_null() {
        *out_len = safe_len;
    }

    safe_len == full_len
}

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

    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len);
    true
}

/// Returns the current preedit text for in-line IME composition.
///
/// # Safety
/// `engine` must be a valid engine pointer. `out_utf8` and `out_len` must be valid pointers if non-null.
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len);
    true
}

/// Finalizes composition on a word break delimiter (Space, punctuation, Enter).
///
/// # Safety
/// `engine` must be a valid engine pointer. `out_utf8` and `out_len` must be valid pointers if non-null.
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len);
    true
}

/// Process backspace in composition mode and return the updated preedit string.
///
/// # Safety
/// `engine` must be a valid engine pointer. `out_utf8` and `out_len` must be valid pointers if non-null.
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len);
    true
}

/// Sets whether free marking is enabled in the engine.
///
/// # Safety
/// `engine` must be a valid pointer to `VietFlowEngine` if non-null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_free_marking(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        if engine.options.free_marking != enabled {
            engine.options.free_marking = enabled;
            engine.reset();
        }
    }
}

/// Sets whether macro expansion is enabled in the engine.
///
/// # Safety
/// `engine` must be a valid pointer to `VietFlowEngine` if non-null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_enable_macro(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.enable_macro = enabled;
    }
}

/// Sets whether automatic English/Vietnamese switching by typing is enabled.
///
/// # Safety
/// `engine` must be a valid pointer to `VietFlowEngine` if non-null.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_set_auto_switch_by_typing(engine: *mut c_void, enabled: bool) {
    if let Some(engine) = (engine as *mut VietFlowEngine).as_mut() {
        engine.options.auto_switch_by_typing = enabled;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_utf8_to_buffer_boundaries() {
        // 'ệ' is 3 bytes (0xE1, 0xBB, 0x87), '🦀' is 4 bytes (0xF0, 0x9F, 0xA6, 0x80)
        let sample = "việt🦀";
        let mut buf = [0u8; 32];
        let mut out_len = 0usize;

        // 1. Buffer length 0: queries length, writes 0 bytes
        unsafe {
            let ok = copy_utf8_to_buffer(sample, buf.as_mut_ptr(), 0, &mut out_len);
            assert!(ok);
            assert_eq!(out_len, sample.len());
        }

        // 2. Buffer length truncated inside 'ệ' (starts at index 2, length 3, ends at 5)
        // If max_out_len is 3 (contains 'v', 'i', and first byte of 'ệ'):
        // Safe cut must not split 'ệ', so it should write "vi" (len 2)
        unsafe {
            let ok = copy_utf8_to_buffer(sample, buf.as_mut_ptr(), 3, &mut out_len);
            assert!(!ok); // truncated
            assert_eq!(out_len, 2);
            assert_eq!(std::str::from_utf8(&buf[..out_len]).unwrap(), "vi");
        }

        // If max_out_len is 4:
        unsafe {
            let ok = copy_utf8_to_buffer(sample, buf.as_mut_ptr(), 4, &mut out_len);
            assert!(!ok);
            assert_eq!(out_len, 2);
            assert_eq!(std::str::from_utf8(&buf[..out_len]).unwrap(), "vi");
        }

        // If max_out_len is 5 (contains "việt" exactly without the 4-byte crab):
        // "việt" = 1 + 1 + 3 + 1 = 6 bytes. Index of 't' is 5..6.
        // Let's test every slice length from 0 to sample.len() + 2:
        for limit in 0..=(sample.len() + 2) {
            let mut test_buf = [0u8; 32];
            let mut written = 0usize;
            unsafe {
                let _ = copy_utf8_to_buffer(sample, test_buf.as_mut_ptr(), limit, &mut written);
            }
            if limit == 0 {
                assert_eq!(written, sample.len());
            } else {
                assert!(written <= limit);
                // Valid UTF-8 must be guaranteed
                let s = std::str::from_utf8(&test_buf[..written]).expect("must be valid utf8");
                assert!(sample.starts_with(s));
            }
        }
    }
}
