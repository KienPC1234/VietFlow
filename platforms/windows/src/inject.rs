// VietFlow Windows - Keystroke Injection (Atomic & Paced SendInput)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::mem::size_of;
use std::thread::sleep;
use std::time::Duration;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, KEYEVENTF_UNICODE,
    VK_BACK,
};

pub const VIETFLOW_MAGIC: usize = 0x56494554; // 'VIET'

#[link(name = "user32")]
extern "system" {
    fn MapVirtualKeyW(uCode: u32, uMapType: u32) -> u32;
}

const ATOMIC_BACKSPACE_LIMIT: usize = 8;
const ATOMIC_CHAR_LIMIT: usize = 32;
const MAX_ATOMIC_INPUTS: usize = (ATOMIC_BACKSPACE_LIMIT + ATOMIC_CHAR_LIMIT) * 2;

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
pub fn create_unicode_input(c: char, key_up: bool) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: 0,
                wScan: c as u16,
                dwFlags: KEYEVENTF_UNICODE | (if key_up { KEYEVENTF_KEYUP } else { 0 }),
                time: 0,
                dwExtraInfo: VIETFLOW_MAGIC,
            },
        },
    }
}

/// Sends backspaces and replacement Unicode characters to the active window.
/// Normal keystroke adjustments (<= 8 backspaces, <= 32 chars) are sent in ONE
/// atomic SendInput call so the caret never visibly jumps or flickers.
/// Large macro restorations pace backspaces with a tiny delay to prevent apps from dropping them.
pub fn send_atomic_batch(backspaces: usize, text: &str) {
    if backspaces == 0 && text.is_empty() {
        return;
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

        for c in text.chars() {
            inputs[count] = create_unicode_input(c, false);
            inputs[count + 1] = create_unicode_input(c, true);
            count += 2;
        }

        unsafe {
            SendInput(
                count as u32,
                inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
        return;
    }

    // Large restorations or macro expansions
    if backspaces > 0 {
        let mut back_inputs = Vec::with_capacity(backspaces * 2);
        for _ in 0..backspaces {
            back_inputs.push(create_vk_input(VK_BACK, false));
            back_inputs.push(create_vk_input(VK_BACK, true));
        }
        unsafe {
            SendInput(
                back_inputs.len() as u32,
                back_inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
        sleep(Duration::from_millis(3));
    }

    if !text.is_empty() {
        let mut char_inputs = Vec::with_capacity(char_count * 2);
        for c in text.chars() {
            char_inputs.push(create_unicode_input(c, false));
            char_inputs.push(create_unicode_input(c, true));
        }
        unsafe {
            SendInput(
                char_inputs.len() as u32,
                char_inputs.as_ptr(),
                size_of::<INPUT>() as i32,
            );
        }
    }
}
