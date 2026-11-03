// VietFlow Windows - Keystroke Injection (Atomic, Scan-code, Safe)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VK_BACK, VK_SHIFT,
};

pub const VIETFLOW_MAGIC: usize = 0x56494554; // 'VIET'

#[link(name = "user32")]
extern "system" {
    fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
}

const ATOMIC_BACKSPACE_LIMIT: usize = 16;
const ATOMIC_CHAR_LIMIT: usize = 32;
const MAX_ATOMIC_INPUTS: usize = (ATOMIC_BACKSPACE_LIMIT + 2) * 2 + (ATOMIC_CHAR_LIMIT + 4) * 4;

#[inline]
pub fn create_vk_input(vk: u16, key_up: bool) -> INPUT {
    let scan = unsafe { MapVirtualKeyW(vk as u32, 0) as u16 };
    create_vk_input_scanned(vk, scan, key_up)
}

#[inline]
fn create_vk_input_scanned(vk: u16, scan: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: scan,
                dwFlags: if key_up { KEYEVENTF_KEYUP } else { 0 },
                time: 0,
                dwExtraInfo: VIETFLOW_MAGIC,
            },
        },
    }
}

#[inline]
pub fn create_utf16_input(code_unit: u16, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: code_unit,
                dwFlags: KEYEVENTF_UNICODE | (if key_up { KEYEVENTF_KEYUP } else { 0 }),
                time: 0,
                dwExtraInfo: VIETFLOW_MAGIC,
            },
        },
    }
}

#[allow(dead_code)]
#[inline]
pub fn create_unicode_input(c: char, key_up: bool) -> INPUT {
    let mut buf = [0u16; 2];
    let encoded = c.encode_utf16(&mut buf);
    create_utf16_input(encoded[0], key_up)
}

/// Sends backspaces and replacement Unicode characters to the active window.
/// Normal keystroke adjustments (<= 8 backspaces, <= 32 chars) are sent in ONE
/// atomic SendInput call so the caret never visibly jumps or flickers.
/// Large macro restorations pace backspaces with a tiny delay to prevent apps from dropping them.
/// Select the output strategy before every batch. 0/Auto and 1/Unicode use the
/// normal atomic Unicode path; 0/Auto retries paced Unicode if the atomic batch
/// was completely rejected (0 events inserted).
pub fn send_atomic_batch_with_mode(backspaces: usize, text: &str, mode: u8) {
    match mode {
        2 => send_scancode_batch(backspaces, text),
        3 | 4 => send_safe_batch(backspaces, text),
        1 => {
            let _ = send_unicode_batch_status(backspaces, text);
        }
        _ => {
            // Auto: Unicode first; if Windows rejected the batch completely (0 events),
            // fall back to paced safe batch. Never blindly retry if partial insertion occurred.
            let (success, total_inserted) = send_unicode_batch_status(backspaces, text);
            if !success && total_inserted == 0 {
                send_safe_batch(backspaces, text);
            }
        }
    }
}

#[allow(dead_code)]
pub fn send_unicode_batch(backspaces: usize, text: &str) -> bool {
    let (success, _) = send_unicode_batch_status(backspaces, text);
    success
}

/// Dispatches unicode batch returning (all_inserted, total_events_inserted)
fn send_unicode_batch_status(backspaces: usize, text: &str) -> (bool, u32) {
    if backspaces == 0 && text.is_empty() {
        return (true, 0);
    }

    let char_count = text.chars().count();

    if backspaces <= ATOMIC_BACKSPACE_LIMIT && char_count <= ATOMIC_CHAR_LIMIT {
        // Fixed-size stack buffer: a normal keystroke must not hit the allocator.
        let mut inputs: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
        let mut count = 0usize;

        let back_scan = unsafe { MapVirtualKeyW(VK_BACK as u32, 0) as u16 };

        for _ in 0..backspaces {
            inputs[count] = create_vk_input_scanned(VK_BACK, back_scan, false);
            inputs[count + 1] = create_vk_input_scanned(VK_BACK, back_scan, true);
            count += 2;
        }

        let mut utf16_buf = [0u16; 2];
        for c in text.chars() {
            for unit in c.encode_utf16(&mut utf16_buf).iter().copied() {
                inputs[count] = create_utf16_input(unit, false);
                inputs[count + 1] = create_utf16_input(unit, true);
                count += 2;
            }
        }

        let inserted =
            unsafe { SendInput(count as u32, inputs.as_ptr(), size_of::<INPUT>() as i32) };
        return (inserted == count as u32, inserted);
    }

    // Large restorations or macro expansions
    let mut total_inserted = 0u32;
    let mut ok = true;
    if backspaces > 0 {
        let mut back_inputs = Vec::with_capacity(backspaces * 2);
        for _ in 0..backspaces {
            back_inputs.push(create_vk_input(VK_BACK, false));
            back_inputs.push(create_vk_input(VK_BACK, true));
        }
        let inserted = unsafe {
            SendInput(
                back_inputs.len() as u32,
                back_inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        total_inserted += inserted;
        ok &= inserted == back_inputs.len() as u32;
        sleep(Duration::from_millis(3));
    }

    if !text.is_empty() {
        let mut char_inputs = Vec::with_capacity(char_count * 4);
        let mut utf16_buf = [0u16; 2];
        for c in text.chars() {
            for unit in c.encode_utf16(&mut utf16_buf).iter().copied() {
                char_inputs.push(create_utf16_input(unit, false));
                char_inputs.push(create_utf16_input(unit, true));
            }
        }
        let inserted = unsafe {
            SendInput(
                char_inputs.len() as u32,
                char_inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            )
        };
        total_inserted += inserted;
        ok &= inserted == char_inputs.len() as u32;
    }
    (ok, total_inserted)
}

/// Scan-code mode for DirectInput-style windows. ASCII characters use the
/// current keyboard layout's VK; characters that cannot be mapped fall back to
/// Unicode so Vietnamese text is never silently dropped.
fn send_scancode_batch(backspaces: usize, text: &str) {
    if backspaces == 0 && text.is_empty() {
        return;
    }

    let mut inputs = Vec::with_capacity((backspaces + text.chars().count() * 3) * 2);
    for _ in 0..backspaces {
        inputs.push(create_vk_input(VK_BACK, false));
        inputs.push(create_vk_input(VK_BACK, true));
    }

    let mut utf16_buf = [0u16; 2];
    for c in text.chars() {
        if c.is_ascii() {
            let mapped = unsafe { VkKeyScanW(c as u16) };
            if mapped != -1 {
                let vk = (mapped as u16) & 0xFF;
                let shift_state = ((mapped as u16) >> 8) & 0xFF;
                if shift_state & 1 != 0 {
                    inputs.push(create_vk_input(VK_SHIFT, false));
                }
                inputs.push(create_vk_input(vk, false));
                inputs.push(create_vk_input(vk, true));
                if shift_state & 1 != 0 {
                    inputs.push(create_vk_input(VK_SHIFT, true));
                }
                continue;
            }
        }
        for unit in c.encode_utf16(&mut utf16_buf).iter().copied() {
            inputs.push(create_utf16_input(unit, false));
            inputs.push(create_utf16_input(unit, true));
        }
    }

    if !inputs.is_empty() {
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
    }
}

/// Compatibility mode: full backspaces, then one Unicode character at a time
/// with a tiny pause. Slower, but survives apps that drop fast batches.
fn send_safe_batch(backspaces: usize, text: &str) {
    if backspaces > 0 {
        let mut inputs = Vec::with_capacity(backspaces * 2);
        for _ in 0..backspaces {
            inputs.push(create_vk_input(VK_BACK, false));
            inputs.push(create_vk_input(VK_BACK, true));
        }
        unsafe {
            SendInput(
                inputs.len() as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
        sleep(Duration::from_millis(4));
    }

    let mut utf16_buf = [0u16; 2];
    for c in text.chars() {
        for unit in c.encode_utf16(&mut utf16_buf).iter().copied() {
            let one = [
                create_utf16_input(unit, false),
                create_utf16_input(unit, true),
            ];
            unsafe {
                SendInput(2, one.as_ptr(), size_of::<INPUT>() as i32);
            }
        }
        sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_utf16_input_surrogate_pairs() {
        let crab = '🦀'; // U+1F980 => 0xD83E, 0xDD80
        let mut buf = [0u16; 2];
        let units = crab.encode_utf16(&mut buf);
        assert_eq!(units.len(), 2);
        assert_eq!(units[0], 0xD83E);
        assert_eq!(units[1], 0xDD80);

        let input_high = create_utf16_input(units[0], false);
        let input_low = create_utf16_input(units[1], false);

        unsafe {
            assert_eq!(input_high.Anonymous.ki.wScan, 0xD83E);
            assert_eq!(input_high.Anonymous.ki.dwFlags, KEYEVENTF_UNICODE);
            assert_eq!(input_low.Anonymous.ki.wScan, 0xDD80);
            assert_eq!(input_low.Anonymous.ki.dwFlags, KEYEVENTF_UNICODE);
        }
    }
}
