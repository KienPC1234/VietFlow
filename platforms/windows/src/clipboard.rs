// VietFlow Windows - Clipboard cache for macro expansion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::borrow::Cow;
use std::mem::size_of;
use std::ptr::null_mut;
use std::sync::Mutex;
use vietflow_core::macro_table::CLIPBOARD_PLACEHOLDER;
use windows_sys::Win32::Foundation::HGLOBAL;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalSize, GlobalUnlock};

const CF_UNICODETEXT: u32 = 13;
const MAX_CLIPBOARD_CHARS: usize = 1_048_576;

static CLIPBOARD_TEXT: Mutex<Option<String>> = Mutex::new(None);

/// Only decode bytes owned by the Windows clipboard allocation. CF_UNICODETEXT
/// requires a NUL terminator within that allocation and within our cap.
/// A malformed/missing terminator must not read beyond GlobalSize or silently
/// cache a truncated clipboard value.
fn decode_clipboard_units(units: &[u16], max_chars: usize) -> Option<String> {
    let limit = units.len().min(max_chars.saturating_add(1));
    let length = units[..limit].iter().position(|&unit| unit == 0)?;
    if length > max_chars {
        return None;
    }
    String::from_utf16(&units[..length]).ok()
}

/// Convert the size of the owned clipboard allocation into a safe scan
/// length. This accounts for the required UTF-16 terminator and rejects
/// allocations with an odd number of bytes.
fn clipboard_scan_units(allocation_bytes: usize) -> Option<usize> {
    if allocation_bytes < size_of::<u16>() || !allocation_bytes.is_multiple_of(size_of::<u16>()) {
        return None;
    }
    Some((allocation_bytes / size_of::<u16>()).min(MAX_CLIPBOARD_CHARS + 1))
}

/// A failed/malformed/cleared clipboard read must not leave an older
/// (possibly sensitive) clipboard value active for later macro expansion.
fn update_cached_clipboard(cache: &mut Option<String>, newest: Option<String>) {
    *cache = newest;
}

fn read_clipboard_text() -> Option<String> {
    unsafe {
        if OpenClipboard(null_mut()) == 0 {
            return None;
        }

        let result = (|| {
            let handle = GetClipboardData(CF_UNICODETEXT);
            if handle.is_null() {
                return None;
            }
            // GetClipboardData does not promise a well-formed NUL-terminated
            // buffer. GlobalSize is the only supported bound for this handle.
            let max_units = clipboard_scan_units(GlobalSize(handle as HGLOBAL))?;
            let ptr = GlobalLock(handle as HGLOBAL) as *const u16;
            if ptr.is_null() {
                return None;
            }
            let text = {
                let units = std::slice::from_raw_parts(ptr, max_units);
                decode_clipboard_units(units, MAX_CLIPBOARD_CHARS)
            };
            GlobalUnlock(handle as HGLOBAL);
            text
        })();

        CloseClipboard();
        result
    }
}

/// Refresh the cache from the Windows clipboard. This is called from the
/// daemon message loop, never from the low-level keyboard hook.
pub fn refresh() {
    let newest = read_clipboard_text();
    if let Ok(mut cache) = CLIPBOARD_TEXT.lock() {
        update_cached_clipboard(&mut cache, newest);
    }
}

/// Check the actual expanded byte count before allocating a replacement.
/// This helper is pure so huge/repeated placeholders can be tested without
/// reading or mutating process clipboard state.
fn expand_cached_bounded<'a>(
    text: &'a str,
    cached: Option<&str>,
    max_bytes: usize,
) -> Option<Cow<'a, str>> {
    let count = text.matches(CLIPBOARD_PLACEHOLDER).count();
    if count == 0 {
        return (text.len() <= max_bytes).then_some(Cow::Borrowed(text));
    }
    // A clipboard format change/read failure must preserve the typed macro
    // trigger, not silently replace the placeholder with empty content.
    let cached = cached?;
    let literal_bytes = text
        .len()
        .checked_sub(count.checked_mul(CLIPBOARD_PLACEHOLDER.len_utf8())?)?;
    let expanded_bytes = literal_bytes.checked_add(count.checked_mul(cached.len())?)?;
    if expanded_bytes > max_bytes {
        return None;
    }
    Some(Cow::Owned(text.replace(CLIPBOARD_PLACEHOLDER, cached)))
}

/// Limit clipboard-backed macro replacement in the low-level keyboard hook.
/// In contrast to generic expansion, use try_lock: a concurrent clipboard
/// refresh must never stall the keyboard callback, and oversized cached text
/// must never be cloned just to discover that SendInput cannot accept it.
pub fn expand_clipboard_bounded(text: &str, max_bytes: usize) -> Option<Cow<'_, str>> {
    if !text.contains(CLIPBOARD_PLACEHOLDER) {
        return (text.len() <= max_bytes).then_some(Cow::Borrowed(text));
    }
    let cache = CLIPBOARD_TEXT.try_lock().ok()?;
    expand_cached_bounded(text, cache.as_deref(), max_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_cached_clipboard_placeholder() {
        // Pure fixture rather than changing global clipboard state: parallel
        // Windows unit tests must not race the contended-cache regression.
        assert_eq!(
            expand_cached_bounded(
                &format!("a{}b", CLIPBOARD_PLACEHOLDER),
                Some("clip text"),
                128,
            )
            .unwrap(),
            "aclip textb"
        );
    }

    #[test]
    fn clipboard_allocation_size_rejects_odd_and_unbounded_buffers() {
        assert_eq!(clipboard_scan_units(0), None);
        assert_eq!(clipboard_scan_units(1), None);
        assert_eq!(clipboard_scan_units(3), None);
        assert_eq!(clipboard_scan_units(2), Some(1));
        assert_eq!(clipboard_scan_units(10), Some(5));
        assert_eq!(
            clipboard_scan_units(4 * MAX_CLIPBOARD_CHARS),
            Some(MAX_CLIPBOARD_CHARS + 1)
        );
        assert_eq!(clipboard_scan_units(usize::MAX), None);
    }

    #[test]
    fn clipboard_read_never_scans_beyond_allocation_or_user_cap() {
        assert_eq!(
            decode_clipboard_units(&[0x0041, 0x0000], 8),
            Some("A".into())
        );
        assert_eq!(decode_clipboard_units(&[0], 0), Some(String::new()));
        assert_eq!(decode_clipboard_units(&[0x0041], 8), None);
        assert_eq!(decode_clipboard_units(&[0x0041, 0x0042], 8), None);
        assert_eq!(
            decode_clipboard_units(&[0x0041, 0x0042, 0], 2),
            Some("AB".into())
        );
        assert_eq!(decode_clipboard_units(&[0x0041, 0x0042, 0], 1), None);
        assert_eq!(decode_clipboard_units(&[0xD800, 0], 8), None);
        assert_eq!(
            decode_clipboard_units(&[0xD83E, 0xDD80, 0], 8),
            Some("\u{1F980}".into())
        );
        assert_eq!(decode_clipboard_units(&[0x0041, 0], 0), None);
    }

    #[test]
    fn failed_or_cleared_clipboard_update_invalidates_old_cached_text() {
        let mut value = Some("secret from previous clipboard".to_string());
        update_cached_clipboard(&mut value, None);
        assert!(value.is_none());
        update_cached_clipboard(&mut value, Some("new".to_string()));
        assert_eq!(value.as_deref(), Some("new"));
        update_cached_clipboard(&mut value, Some(String::new()));
        assert_eq!(value.as_deref(), Some(""));
    }

    #[test]
    fn bounded_macro_expansion_preflights_large_cached_content() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        let long_clipboard = "x".repeat(1_048_576);
        assert!(expand_cached_bounded(&format!("a{token}b"), Some(&long_clipboard), 128).is_none());
        assert!(
            expand_cached_bounded(&format!("{token}{token}"), Some(&"x".repeat(65)), 128).is_none()
        );
        assert_eq!(
            expand_cached_bounded(&format!("a{token}b"), Some("v"), 128).unwrap(),
            "avb"
        );
        assert_eq!(
            expand_cached_bounded("plain", Some(&long_clipboard), 5).unwrap(),
            "plain"
        );
        assert!(expand_cached_bounded("plain!", Some(""), 5).is_none());
    }

    #[test]
    fn unavailable_clipboard_never_erases_a_typed_macro_trigger() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        assert!(expand_cached_bounded(&token, None, 128).is_none());
        assert!(expand_cached_bounded(&format!("x{token}y"), None, 128).is_none());
        assert_eq!(expand_cached_bounded("plain", None, 128).unwrap(), "plain");
        // A valid but empty clipboard is distinct from an unavailable one.
        assert_eq!(expand_cached_bounded(&token, Some(""), 128).unwrap(), "");
    }

    #[test]
    fn normal_vietnamese_hook_budget_rejects_oversized_clipboard_payload() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        let max = crate::inject::MAX_PENDING_TEXT_BYTES;
        let valid = "z".repeat(max);
        let oversized = "z".repeat(max + 1);
        assert_eq!(
            expand_cached_bounded(&token, Some(&valid), max).unwrap().len(),
            max
        );
        assert!(expand_cached_bounded(&token, Some(&oversized), max).is_none());
        assert!(expand_cached_bounded(&format!("{token}!"), Some(&valid), max).is_none());
    }

    #[test]
    fn bounded_macro_expansion_preserves_unicode_byte_limits() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        let emoji = "\u{1F980}".repeat(32);
        assert_eq!(
            expand_cached_bounded(&token, Some(&emoji), 128).unwrap(),
            emoji
        );
        assert!(expand_cached_bounded(&format!("{token}!"), Some(&emoji), 128).is_none());
        // A placeholder replacing with empty text may fit a smaller budget.
        assert_eq!(expand_cached_bounded(&token, Some(""), 0).unwrap(), "");
    }

    #[test]
    fn bounded_expansion_fails_open_when_cache_lock_is_held() {
        let guard = CLIPBOARD_TEXT.lock().unwrap();
        assert!(expand_clipboard_bounded(&CLIPBOARD_PLACEHOLDER.to_string(), 128).is_none());
        drop(guard);
    }

    #[test]
    fn leaves_plain_text_alone() {
        assert_eq!(expand_clipboard_bounded("plain", 128).unwrap(), "plain");
    }
}
