// VietFlow Core - C-ABI Interface for FFI (Linux Fcitx5 / Windows Native / C++)
// Licensed under GNU General Public License v3.0 (GPLv3)

use crate::engine::VietFlowEngine;
use crate::english::default_english_keywords;
use crate::models::{CharsetId, InputMethod};
use std::ffi::c_void;

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

    if !out.is_null() && out_cap >= bytes.len() {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out, bytes.len());
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

/// Legacy borrowed output: report full required byte length through out_len.
/// A null pointer or zero capacity is a size query; mutating operations are
/// NOT retry-safe. Insufficient capacity returns false without writing ANY
/// bytes; success writes exactly required bytes without a NUL terminator.
/// Prefer owned-output variants for mutating methods with variable-size text.
unsafe fn copy_utf8_to_buffer(
    text: &str,
    out_utf8: *mut u8,
    max_out_len: usize,
    out_len: *mut usize,
) -> bool {
    let bytes = text.as_bytes();
    let required = bytes.len();
    if !out_len.is_null() {
        *out_len = required;
    }
    if out_utf8.is_null() || max_out_len == 0 {
        return true;
    }
    if max_out_len < required {
        return false;
    }
    if required != 0 {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out_utf8, required);
    }
    true
}

/// Owned UTF-8 result. Caller must release the exact pointer+length by calling
/// vf_owned_utf8_free exactly once. Empty output is a null pointer, len 0.
#[repr(C)]
pub struct VfOwnedUtf8 {
    pub data: *mut u8,
    pub len: usize,
}

unsafe fn write_owned_utf8(text: String, result: *mut VfOwnedUtf8) -> bool {
    if result.is_null() {
        return false;
    }
    if text.is_empty() {
        *result = VfOwnedUtf8 {
            data: std::ptr::null_mut(),
            len: 0,
        };
        return true;
    }
    let boxed = text.into_bytes().into_boxed_slice();
    let len = boxed.len();
    *result = VfOwnedUtf8 {
        data: Box::into_raw(boxed) as *mut u8,
        len,
    };
    true
}

/// # Safety
/// The value must come unchanged from a successful owned-result call and must
/// never be freed twice.
#[no_mangle]
pub unsafe extern "C" fn vf_owned_utf8_free(value: VfOwnedUtf8) {
    if !value.data.is_null() {
        let slice = std::ptr::slice_from_raw_parts_mut(value.data, value.len);
        drop(Box::from_raw(slice));
    }
}

/// Process one key exactly once, returning complete UTF-8 without truncation.
///
/// # Safety
/// `engine` must point to a live VietFlow engine owned by the caller and must
/// not be concurrently accessed or destroyed. `result` must be a valid writable
/// `VfOwnedUtf8` slot, disjoint from the engine and all other outputs; free any
/// previously owned `result.data` before reusing it. If non-null,
/// `out_backspaces` must point to a separate writable `usize`.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_process_key_owned(
    engine: *mut c_void,
    ch: u32,
    result: *mut VfOwnedUtf8,
    out_backspaces: *mut usize,
) -> bool {
    if result.is_null() {
        return false;
    }
    *result = VfOwnedUtf8 {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let Some(engine) = (engine as *mut VietFlowEngine).as_mut() else {
        return false;
    };
    let Some(c) = char::from_u32(ch) else {
        return false;
    };
    let (backspaces, text) = engine.process_key(c);
    if !out_backspaces.is_null() {
        *out_backspaces = backspaces;
    }
    write_owned_utf8(text.as_str().to_owned(), result)
}

/// Read the complete UTF-8 preedit without truncation.
///
/// # Safety
/// `engine` must refer to a live VietFlow engine and must not be concurrently
/// mutated or destroyed. `result` must be a valid writable `VfOwnedUtf8` slot,
/// disjoint from the engine; release any prior owned result before reuse.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_get_preedit_owned(
    engine: *mut c_void,
    result: *mut VfOwnedUtf8,
) -> bool {
    if result.is_null() {
        return false;
    }
    *result = VfOwnedUtf8 {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let Some(engine) = (engine as *mut VietFlowEngine).as_ref() else {
        return false;
    };
    write_owned_utf8(engine.current_preedit_text(), result)
}

/// Finalize composition once and return its complete UTF-8 output.
///
/// # Safety
/// `engine` must refer to a live VietFlow engine with exclusive mutable access
/// for this call. `result` must point to a writable `VfOwnedUtf8` slot that
/// does not alias the engine; release any prior owned result before reuse.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_finish_on_break_owned(
    engine: *mut c_void,
    break_char: u32,
    result: *mut VfOwnedUtf8,
) -> bool {
    if result.is_null() {
        return false;
    }
    *result = VfOwnedUtf8 {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let Some(engine) = (engine as *mut VietFlowEngine).as_mut() else {
        return false;
    };
    let c = char::from_u32(break_char).unwrap_or(' ');
    write_owned_utf8(engine.finish_composition_on_break(c), result)
}

/// Process one Backspace and return the complete updated UTF-8 preedit.
///
/// # Safety
/// `engine` must refer to a live VietFlow engine with exclusive mutable access
/// for this call. `result` must point to a writable `VfOwnedUtf8` slot that
/// does not alias the engine; release any prior owned result before reuse.
#[no_mangle]
pub unsafe extern "C" fn vf_engine_process_backspace_owned(
    engine: *mut c_void,
    result: *mut VfOwnedUtf8,
) -> bool {
    if result.is_null() {
        return false;
    }
    *result = VfOwnedUtf8 {
        data: std::ptr::null_mut(),
        len: 0,
    };
    let Some(engine) = (engine as *mut VietFlowEngine).as_mut() else {
        return false;
    };
    let _ = engine.process_key('\x08');
    write_owned_utf8(engine.current_preedit_text(), result)
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

    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len)
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len)
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len)
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
    copy_utf8_to_buffer(&text, out_utf8, max_out_len, out_len)
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
        let sample = "vi?t??";
        let required = sample.len();
        for cap in 0..=(required + 2) {
            let mut buf = [0xa5u8; 64];
            let mut out_len = usize::MAX;
            let ok = unsafe { copy_utf8_to_buffer(sample, buf.as_mut_ptr(), cap, &mut out_len) };
            assert_eq!(out_len, required);
            if cap == 0 {
                assert!(ok, "zero capacity is a size query");
                assert!(buf.iter().all(|b| *b == 0xa5));
            } else if cap < required {
                assert!(!ok, "insufficient capacity must fail");
                assert!(buf.iter().all(|b| *b == 0xa5), "no partial bytes");
            } else {
                assert!(ok);
                assert_eq!(&buf[..required], sample.as_bytes());
                assert!(buf[required..].iter().all(|b| *b == 0xa5));
            }
        }
        let mut query_size = 0usize;
        assert!(unsafe { copy_utf8_to_buffer(sample, std::ptr::null_mut(), 8, &mut query_size) });
        assert_eq!(query_size, required);
    }

    #[test]
    fn test_owned_utf8_large_supplementary_payload_and_empty() {
        let text = "Ti?ng Vi?t ????".repeat(2048);
        let mut owned = VfOwnedUtf8 {
            data: std::ptr::null_mut(),
            len: 0,
        };
        assert!(unsafe { write_owned_utf8(text.clone(), &mut owned) });
        assert_eq!(owned.len, text.len());
        assert!(!owned.data.is_null());
        let restored = unsafe {
            std::str::from_utf8(std::slice::from_raw_parts(owned.data, owned.len)).unwrap()
        };
        assert_eq!(restored, text);
        unsafe { vf_owned_utf8_free(owned) };

        let mut empty = VfOwnedUtf8 {
            data: std::ptr::null_mut(),
            len: 0,
        };
        assert!(unsafe { write_owned_utf8(String::new(), &mut empty) });
        assert!(empty.data.is_null());
        assert_eq!(empty.len, 0);
        unsafe { vf_owned_utf8_free(empty) };
    }

    #[test]
    fn test_owned_mutating_api_single_pass_and_fail_closed() {
        unsafe {
            let engine = vf_engine_create();
            assert!(!engine.is_null());
            let mut data = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            let mut backspaces = usize::MAX;
            assert!(vf_engine_process_key_owned(
                engine,
                'a' as u32,
                &mut data,
                &mut backspaces
            ));
            if data.len > 0 {
                assert!(!data.data.is_null());
                assert!(
                    std::str::from_utf8(std::slice::from_raw_parts(data.data, data.len)).is_ok()
                );
            } else {
                assert!(data.data.is_null());
            }
            vf_owned_utf8_free(data);

            let mut preedit = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(vf_engine_get_preedit_owned(engine, &mut preedit));
            assert_eq!(
                std::str::from_utf8(std::slice::from_raw_parts(preedit.data, preedit.len)).unwrap(),
                "a"
            );
            vf_owned_utf8_free(preedit);

            let mut rejected = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(!vf_engine_process_key_owned(
                engine,
                0x110000,
                &mut rejected,
                &mut backspaces
            ));
            assert!(rejected.data.is_null());

            let mut after_rejection = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(vf_engine_get_preedit_owned(engine, &mut after_rejection));
            assert_eq!(
                std::str::from_utf8(std::slice::from_raw_parts(
                    after_rejection.data,
                    after_rejection.len
                ))
                .unwrap(),
                "a"
            );
            vf_owned_utf8_free(after_rejection);

            let mut after_backspace = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(vf_engine_process_backspace_owned(
                engine,
                &mut after_backspace
            ));
            vf_owned_utf8_free(after_backspace);

            let mut finalized = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(vf_engine_finish_on_break_owned(
                engine,
                ' ' as u32,
                &mut finalized
            ));
            vf_owned_utf8_free(finalized);
            vf_engine_destroy(engine);
        }
    }

    #[test]
    fn test_owned_ffi_preserves_long_macro_expansion_after_single_finalize() {
        unsafe {
            let engine = vf_engine_create();
            assert!(!engine.is_null());
            let expansion = "Vi?t Nam ???? ".repeat(600);
            let typed = &mut *(engine as *mut VietFlowEngine);
            typed.options.enable_macro = true;
            typed.macros.clear();
            typed.macros.insert("vf", &expansion);

            for c in ['v', 'f'] {
                let mut output = VfOwnedUtf8 {
                    data: std::ptr::null_mut(),
                    len: 0,
                };
                let mut backspaces = 0;
                assert!(vf_engine_process_key_owned(
                    engine,
                    c as u32,
                    &mut output,
                    &mut backspaces
                ));
                vf_owned_utf8_free(output);
            }

            let mut output = VfOwnedUtf8 {
                data: std::ptr::null_mut(),
                len: 0,
            };
            assert!(vf_engine_finish_on_break_owned(
                engine,
                ' ' as u32,
                &mut output
            ));
            assert_eq!(output.len, expansion.len());
            assert!(!output.data.is_null());
            let actual =
                std::str::from_utf8(std::slice::from_raw_parts(output.data, output.len)).unwrap();
            assert_eq!(actual, expansion);
            vf_owned_utf8_free(output);
            vf_engine_destroy(engine);
        }
    }

    #[test]
    fn test_keyword_list_never_returns_truncated_multibyte_text() {
        let required = unsafe { vf_english_keywords(std::ptr::null_mut(), 0) };
        assert!(required > 10);
        let mut small = [0xa5u8; 8];
        assert_eq!(
            unsafe { vf_english_keywords(small.as_mut_ptr(), small.len()) },
            required
        );
        assert_eq!(small, [0xa5u8; 8]);
        let mut buffer = vec![0u8; required];
        assert_eq!(
            unsafe { vf_english_keywords(buffer.as_mut_ptr(), buffer.len()) },
            required
        );
        assert!(std::str::from_utf8(&buffer).is_ok());
    }
}
