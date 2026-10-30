// VietFlow Core - C-ABI Interface for FFI (Linux Fcitx5 / Windows Native / C++)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::ffi::c_void;
use crate::converter::{convert_charset, strip_accents};
use crate::engine::VietFlowEngine;
use crate::models::{CharsetId, InputMethod};

fn charset_from_id(id: u8) -> CharsetId {
    match id {
        1 => CharsetId::UnicodeComposite,
        2 => CharsetId::Tcvn3,
        3 => CharsetId::VniWindows,
        4 => CharsetId::Viqr,
        _ => CharsetId::Unicode,
    }
}

/// Convert a UTF-8 string between charsets.
///
/// Returns the number of bytes required for the converted output. When `out` is
/// null (or `out_cap` is smaller than the output) nothing beyond `out_cap` bytes
/// is written, allowing callers to query the size first and then allocate.
#[no_mangle]
pub unsafe extern "C" fn vf_convert_charset(
    input: *const u8,
    input_len: usize,
    source: u8,
    target: u8,
    out: *mut u8,
    out_cap: usize,
) -> usize {
    if input.is_null() {
        return 0;
    }
    let bytes = std::slice::from_raw_parts(input, input_len);
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return 0,
    };

    let converted = convert_charset(text, charset_from_id(source), charset_from_id(target));
    let converted = converted.as_bytes();

    if !out.is_null() && out_cap > 0 {
        let to_write = converted.len().min(out_cap);
        std::ptr::copy_nonoverlapping(converted.as_ptr(), out, to_write);
    }

    converted.len()
}

/// Strip Vietnamese diacritics from a UTF-8 string (same size-query contract as
/// `vf_convert_charset`).
#[no_mangle]
pub unsafe extern "C" fn vf_strip_accents(
    input: *const u8,
    input_len: usize,
    out: *mut u8,
    out_cap: usize,
) -> usize {
    if input.is_null() {
        return 0;
    }
    let bytes = std::slice::from_raw_parts(input, input_len);
    let text = match std::str::from_utf8(bytes) {
        Ok(text) => text,
        Err(_) => return 0,
    };

    let stripped = strip_accents(text);
    let stripped = stripped.as_bytes();

    if !out.is_null() && out_cap > 0 {
        let to_write = stripped.len().min(out_cap);
        std::ptr::copy_nonoverlapping(stripped.as_ptr(), out, to_write);
    }

    stripped.len()
}

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
            1 => InputMethod::Vni,
            2 => InputMethod::Viqr,
            3 => InputMethod::SimpleTelex,
            4 => InputMethod::TelexSimple2,
            5 => InputMethod::TelexVni,
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

#[cfg(test)]
mod tests {
    use super::*;

    unsafe fn convert_via_abi(text: &str, source: u8, target: u8) -> String {
        let input = text.as_bytes();
        let needed = vf_convert_charset(
            input.as_ptr(),
            input.len(),
            source,
            target,
            std::ptr::null_mut(),
            0,
        );
        assert!(needed > 0);

        let mut output = vec![0u8; needed];
        let written = vf_convert_charset(
            input.as_ptr(),
            input.len(),
            source,
            target,
            output.as_mut_ptr(),
            output.len(),
        );
        assert_eq!(written, needed);
        String::from_utf8(output).expect("converter must emit UTF-8")
    }

    #[test]
    fn convert_charset_abi_matches_core() {
        let text = "Tiếng Việt";
        unsafe {
            let via_abi = convert_via_abi(text, 0, 2);
            let expected = convert_charset(text, CharsetId::Unicode, CharsetId::Tcvn3);
            assert_eq!(via_abi, expected);

            let roundtrip = convert_via_abi(&via_abi, 2, 0);
            assert_eq!(roundtrip, text);
        }
    }

    #[test]
    fn strip_accents_abi() {
        let text = "Đường hoa Nguyễn Huệ";
        let input = text.as_bytes();
        unsafe {
            let needed = vf_strip_accents(input.as_ptr(), input.len(), std::ptr::null_mut(), 0);
            assert!(needed > 0);
            let mut output = vec![0u8; needed];
            let written = vf_strip_accents(
                input.as_ptr(),
                input.len(),
                output.as_mut_ptr(),
                output.len(),
            );
            assert_eq!(written, needed);
            assert_eq!(String::from_utf8(output).unwrap(), "Duong hoa Nguyen Hue");
        }
    }

    #[test]
    fn size_query_never_overflows_buffer() {
        let text = "Việt Nam";
        let input = text.as_bytes();
        unsafe {
            let mut small = [0u8; 2];
            let needed = vf_convert_charset(
                input.as_ptr(),
                input.len(),
                0,
                0,
                small.as_mut_ptr(),
                small.len(),
            );
            assert_eq!(needed, text.len());
            assert_eq!(&small, &input[..2]);
        }
    }
}
