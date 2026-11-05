// VietFlow Windows - Clipboard cache for macro expansion
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::borrow::Cow;
use std::ptr::null_mut;
use std::sync::Mutex;
use vietflow_core::macro_table::CLIPBOARD_PLACEHOLDER;
use windows_sys::Win32::Foundation::HGLOBAL;
use windows_sys::Win32::System::DataExchange::{CloseClipboard, GetClipboardData, OpenClipboard};
use windows_sys::Win32::System::Memory::{GlobalLock, GlobalUnlock};

const CF_UNICODETEXT: u32 = 13;
const MAX_CLIPBOARD_CHARS: usize = 1_048_576;

static CLIPBOARD_TEXT: Mutex<Option<String>> = Mutex::new(None);

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
            let ptr = GlobalLock(handle as HGLOBAL) as *const u16;
            if ptr.is_null() {
                return None;
            }

            let mut len = 0usize;
            while len < MAX_CLIPBOARD_CHARS && *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            let text = String::from_utf16_lossy(slice);
            GlobalUnlock(handle as HGLOBAL);
            Some(text)
        })();

        CloseClipboard();
        result
    }
}

/// Refresh the cache from the Windows clipboard. This is called from the
/// daemon message loop, never from the low-level keyboard hook.
pub fn refresh() {
    if let Some(text) = read_clipboard_text() {
        if let Ok(mut cache) = CLIPBOARD_TEXT.lock() {
            *cache = Some(text);
        }
    }
}

/// Replace the core placeholder with the last cached clipboard text. Doing
/// this from the hook only touches memory, so clipboard contention cannot block
/// a keystroke.
pub fn expand_clipboard(text: &str) -> Cow<'_, str> {
    if !text.contains(CLIPBOARD_PLACEHOLDER) {
        return Cow::Borrowed(text);
    }

    let clipboard = CLIPBOARD_TEXT
        .lock()
        .ok()
        .and_then(|cache| cache.clone())
        .unwrap_or_default();
    Cow::Owned(text.replace(CLIPBOARD_PLACEHOLDER, &clipboard))
}

/// Check the actual expanded byte count before allocating a replacement.
/// This helper is pure so huge/repeated placeholders can be tested without
/// reading or mutating process clipboard state.
fn expand_cached_bounded<'a>(
    text: &'a str,
    cached: &str,
    max_bytes: usize,
) -> Option<Cow<'a, str>> {
    let count = text.matches(CLIPBOARD_PLACEHOLDER).count();
    if count == 0 {
        return (text.len() <= max_bytes).then_some(Cow::Borrowed(text));
    }
    let literal_bytes = text.len().checked_sub(
        count.checked_mul(CLIPBOARD_PLACEHOLDER.len_utf8())?
    )?;
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
    expand_cached_bounded(text, cache.as_deref().unwrap_or_default(), max_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expands_cached_clipboard_placeholder() {
        if let Ok(mut cache) = CLIPBOARD_TEXT.lock() {
            *cache = Some("clip text".to_string());
        }
        assert_eq!(
            expand_clipboard(&format!("a{}b", CLIPBOARD_PLACEHOLDER)),
            "aclip textb"
        );
    }

    #[test]
    fn bounded_macro_expansion_preflights_large_cached_content() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        let long_clipboard = "x".repeat(1_048_576);
        assert!(expand_cached_bounded(&format!("a{token}b"), &long_clipboard, 128).is_none());
        assert!(expand_cached_bounded(&format!("{token}{token}"), &"x".repeat(65), 128).is_none());
        assert_eq!(
            expand_cached_bounded(&format!("a{token}b"), "v", 128).unwrap(),
            "avb"
        );
        assert_eq!(
            expand_cached_bounded("plain", &long_clipboard, 5).unwrap(),
            "plain"
        );
        assert!(expand_cached_bounded("plain!", "", 5).is_none());
    }

    #[test]
    fn bounded_macro_expansion_preserves_unicode_byte_limits() {
        let token = CLIPBOARD_PLACEHOLDER.to_string();
        let emoji = "\u{1F980}".repeat(32);
        assert_eq!(
            expand_cached_bounded(&token, &emoji, 128).unwrap(),
            emoji
        );
        assert!(expand_cached_bounded(&format!("{token}!"), &emoji, 128).is_none());
        // A placeholder replacing with empty text may fit a smaller budget.
        assert_eq!(expand_cached_bounded(&token, "", 0).unwrap(), "");
    }

    #[test]
    fn bounded_expansion_fails_open_when_cache_lock_is_held() {
        let guard = CLIPBOARD_TEXT.lock().unwrap();
        assert!(expand_clipboard_bounded(&CLIPBOARD_PLACEHOLDER.to_string(), 128).is_none());
        drop(guard);
    }

    #[test]
    fn leaves_plain_text_alone() {
        assert_eq!(expand_clipboard("plain"), "plain");
    }
}
