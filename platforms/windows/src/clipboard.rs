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
    fn leaves_plain_text_alone() {
        assert_eq!(expand_clipboard("plain"), "plain");
    }
}
