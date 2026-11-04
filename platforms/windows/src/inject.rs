// VietFlow Windows - Keystroke Injection (Atomic, Scan-code, Safe)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::cell::Cell;
use std::mem::size_of;
use std::sync::{mpsc, OnceLock};
use std::thread::sleep;
use std::time::Duration;
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, VkKeyScanW, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
    KEYEVENTF_UNICODE, VK_BACK, VK_SHIFT,
};

use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct InjectionTarget {
    hwnd: usize,
    thread_id: u32,
    process_id: u32,
}

impl InjectionTarget {
    fn foreground() -> Option<Self> {
        unsafe {
            let hwnd = GetForegroundWindow();
            if hwnd.is_null() {
                return None;
            }
            let mut process_id = 0u32;
            let thread_id = GetWindowThreadProcessId(hwnd, &mut process_id);
            if thread_id == 0 || process_id == 0 {
                return None;
            }
            Some(Self {
                hwnd: hwnd as usize,
                thread_id,
                process_id,
            })
        }
    }
}

/// Only an identical window, foreground thread and process can receive
/// a delayed injection. An HWND value on its own can be recycled.
fn matches_target(target: InjectionTarget, actual: Option<InjectionTarget>) -> bool {
    actual == Some(target)
}

thread_local! {
    static ASYNC_TARGET: Cell<Option<InjectionTarget>> = const { Cell::new(None) };
}

fn async_target_still_active() -> bool {
    ASYNC_TARGET.with(|current| match current.get() {
        Some(target) => matches_target(target, InjectionTarget::foreground()),
        None => true, // The immediate hook path remains synchronous.
    })
}

struct InjectionJob {
    backspaces: usize,
    text: String,
    mode: u8,
    target: InjectionTarget,
}

static INJECTION_TX: OnceLock<mpsc::Sender<InjectionJob>> = OnceLock::new();

fn get_injection_sender() -> mpsc::Sender<InjectionJob> {
    INJECTION_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::channel::<InjectionJob>();
            std::thread::Builder::new()
                .name("vietflow-async-injector".into())
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        // Queued macro work must not type into the next app if
                        // the foreground changed before the worker was scheduled.
                        if !matches_target(job.target, InjectionTarget::foreground()) {
                            continue;
                        }
                        ASYNC_TARGET.with(|target| target.set(Some(job.target)));
                        execute_injection_job(job.backspaces, &job.text, job.mode);
                        ASYNC_TARGET.with(|target| target.set(None));
                    }
                })
                .expect("failed to spawn async injector thread");
            tx
        })
        .clone()
}

/// Initialize the worker before WH_KEYBOARD_LL is installed. Thread startup
/// and channel allocation must never happen on the keystroke callback path.
pub fn prewarm_async_injector() {
    let _ = get_injection_sender();
}

pub fn enqueue_async_injection(backspaces: usize, text: String, mode: u8) {
    let Some(target) = InjectionTarget::foreground() else {
        return;
    };
    let _ = get_injection_sender().send(InjectionJob {
        backspaces,
        text,
        mode,
        target,
    });
}

fn execute_injection_job(backspaces: usize, text: &str, mode: u8) {
    match mode {
        2 => send_scancode_batch(backspaces, text),
        3 | 4 => send_safe_batch(backspaces, text),
        1 => {
            let _ = send_unicode_batch_status(backspaces, text);
        }
        _ => {
            let (success, total_inserted) = send_unicode_batch_status(backspaces, text);
            if !success && total_inserted == 0 {
                send_safe_batch(backspaces, text);
            }
        }
    }
}

/// Sends backspaces and replacement Unicode characters to the active window.
/// Normal keystroke adjustments (<= 16 backspaces, <= 32 chars) are sent in ONE
/// atomic SendInput call so the caret never visibly jumps or flickers.
/// Large macro restorations pace backspaces with a tiny delay to prevent apps from dropping them.
/// Large macros and safe mode are offloaded to an asynchronous worker to prevent blocking
/// LowLevelKeyboardProc and exceeding Windows hook timeout.
pub fn send_atomic_batch_with_mode(backspaces: usize, text: &str, mode: u8) {
    let char_count = text.chars().count();
    let is_large_or_paced = char_count > ATOMIC_CHAR_LIMIT
        || backspaces > ATOMIC_BACKSPACE_LIMIT
        || mode == 3
        || mode == 4;

    if is_large_or_paced {
        enqueue_async_injection(backspaces, text.to_string(), mode);
        return;
    }

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

/// On partial insertion, release only the keys in the accepted prefix.
/// Never replay accepted backspaces, modifiers or Unicode code units.
fn pending_releases(prefix: &[INPUT]) -> Vec<INPUT> {
    let mut pressed: Vec<KEYBDINPUT> = Vec::new();
    for event in prefix {
        if event.r#type != INPUT_KEYBOARD {
            continue;
        }
        let key = unsafe { event.Anonymous.ki };
        if key.dwFlags & KEYEVENTF_KEYUP == 0 {
            pressed.push(key);
        } else if let Some(pos) = pressed.iter().rposition(|k| {
            k.wVk == key.wVk
                && k.wScan == key.wScan
                && (k.dwFlags & KEYEVENTF_UNICODE) == (key.dwFlags & KEYEVENTF_UNICODE)
        }) {
            pressed.remove(pos);
        }
    }
    pressed
        .into_iter()
        .rev()
        .map(|mut key| {
            key.dwFlags |= KEYEVENTF_KEYUP;
            INPUT {
                r#type: INPUT_KEYBOARD,
                Anonymous: INPUT_0 { ki: key },
            }
        })
        .collect()
}

/// Attempt every original event at most once. When Windows returns a partial
/// prefix, best-effort release injected keys without replaying destructive input.
fn send_checked_using<F>(inputs: &[INPUT], mut send: F) -> (bool, u32)
where
    F: FnMut(&[INPUT]) -> u32,
{
    if inputs.is_empty() {
        return (true, 0);
    }
    if inputs.len() > u32::MAX as usize {
        return (false, 0);
    }
    let expected = inputs.len() as u32;
    let inserted = send(inputs).min(expected);
    if inserted > 0 && inserted < expected {
        let releases = pending_releases(&inputs[..inserted as usize]);
        if !releases.is_empty() {
            // A partial follow-up release is not safe to retry either.
            let _ = send(&releases);
        }
    }
    (inserted == expected, inserted)
}

fn send_checked(inputs: &[INPUT]) -> (bool, u32) {
    if !async_target_still_active() {
        return (false, 0);
    }
    send_checked_using(inputs, |batch| unsafe {
        SendInput(
            batch.len() as u32,
            batch.as_ptr(),
            size_of::<INPUT>() as i32,
        )
    })
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

        return send_checked(&inputs[..count]);
    }

    // Large macro restorations run on the worker. If backspaces are only
    // partially accepted, do not insert replacement characters afterward.
    let mut total_inserted = 0u32;
    if backspaces > 0 {
        let mut back_inputs = Vec::with_capacity(backspaces * 2);
        for _ in 0..backspaces {
            back_inputs.push(create_vk_input(VK_BACK, false));
            back_inputs.push(create_vk_input(VK_BACK, true));
        }
        let (complete, inserted) = send_checked(&back_inputs);
        total_inserted = total_inserted.saturating_add(inserted);
        if !complete {
            return (false, total_inserted);
        }
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
        let (complete, inserted) = send_checked(&char_inputs);
        return (complete, total_inserted.saturating_add(inserted));
    }
    (true, total_inserted)
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
        let _ = send_checked(&inputs);
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
        let (complete, _) = send_checked(&inputs);
        if !complete {
            return;
        }
        sleep(Duration::from_millis(4));
    }
    let mut utf16_buf = [0u16; 2];
    for c in text.chars() {
        // Do not split a supplementary-plane scalar over separate SendInput calls.
        let mut inputs: [INPUT; 4] = unsafe { std::mem::zeroed() };
        let mut count = 0;
        for unit in c.encode_utf16(&mut utf16_buf).iter().copied() {
            inputs[count] = create_utf16_input(unit, false);
            inputs[count + 1] = create_utf16_input(unit, true);
            count += 2;
        }
        let (complete, _) = send_checked(&inputs[..count]);
        if !complete {
            return;
        }
        sleep(Duration::from_millis(1));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_unicode_prefix_releases_only_unmatched_key() {
        let events = [
            create_utf16_input(0xD83E, false),
            create_utf16_input(0xD83E, true),
            create_utf16_input(0xDD80, false),
            create_utf16_input(0xDD80, true),
        ];
        assert!(pending_releases(&events[..0]).is_empty());
        assert!(pending_releases(&events[..2]).is_empty());
        assert!(pending_releases(&events[..4]).is_empty());
        for (count, unit) in [(1usize, 0xD83E), (3usize, 0xDD80)] {
            let releases = pending_releases(&events[..count]);
            assert_eq!(releases.len(), 1);
            let key = unsafe { releases[0].Anonymous.ki };
            assert_eq!(key.wScan, unit);
            assert_ne!(key.dwFlags & KEYEVENTF_KEYUP, 0);
            assert_ne!(key.dwFlags & KEYEVENTF_UNICODE, 0);
        }
    }

    #[test]
    fn partial_scancode_prefix_cleans_shift_without_replaying_vk() {
        let events = [
            create_vk_input_scanned(VK_SHIFT, 42, false),
            create_vk_input_scanned(b'A' as u16, 30, false),
            create_vk_input_scanned(b'A' as u16, 30, true),
            create_vk_input_scanned(VK_SHIFT, 42, true),
        ];
        assert_eq!(
            unsafe { pending_releases(&events[..1])[0].Anonymous.ki.wVk },
            VK_SHIFT
        );
        let releases = pending_releases(&events[..2]);
        assert_eq!(releases.len(), 2);
        assert_eq!(unsafe { releases[0].Anonymous.ki.wVk }, b'A' as u16);
        assert_eq!(unsafe { releases[1].Anonymous.ki.wVk }, VK_SHIFT);
        assert_eq!(
            unsafe { pending_releases(&events[..3])[0].Anonymous.ki.wVk },
            VK_SHIFT
        );
        assert!(pending_releases(&events).is_empty());
    }

    #[test]
    fn partial_backspace_prefix_only_releases_pressed_key() {
        let events = [
            create_vk_input_scanned(VK_BACK, 14, false),
            create_vk_input_scanned(VK_BACK, 14, true),
        ];
        let releases = pending_releases(&events[..1]);
        assert_eq!(releases.len(), 1);
        assert_eq!(unsafe { releases[0].Anonymous.ki.wVk }, VK_BACK);
        assert_ne!(
            unsafe { releases[0].Anonymous.ki.dwFlags } & KEYEVENTF_KEYUP,
            0
        );
        assert!(pending_releases(&events).is_empty());
    }

    #[test]
    fn repeated_vk_events_pair_with_latest_keydown() {
        let events = [
            create_vk_input_scanned(VK_BACK, 14, false),
            create_vk_input_scanned(VK_BACK, 14, true),
            create_vk_input_scanned(VK_BACK, 14, false),
            create_vk_input_scanned(VK_BACK, 14, true),
        ];
        assert!(pending_releases(&events[..2]).is_empty());
        assert_eq!(pending_releases(&events[..3]).len(), 1);
        assert!(pending_releases(&events).is_empty());
    }

    #[test]
    fn mocked_partial_sendinput_never_replays_accepted_destructive_events() {
        let original = [
            create_vk_input_scanned(VK_BACK, 14, false),
            create_vk_input_scanned(VK_BACK, 14, true),
            create_utf16_input('x' as u16, false),
            create_utf16_input('x' as u16, true),
        ];
        let mut calls: Vec<Vec<(u16, u16, u32)>> = Vec::new();
        let (ok, count) = send_checked_using(&original, |batch| {
            let keys: Vec<(u16, u16, u32)> = batch
                .iter()
                .map(|event| {
                    let key = unsafe { event.Anonymous.ki };
                    (key.wVk, key.wScan, key.dwFlags)
                })
                .collect();
            calls.push(keys);
            if calls.len() == 1 {
                1
            } else {
                batch.len() as u32
            }
        });
        assert!(!ok);
        assert_eq!(count, 1);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].len(), original.len());
        assert_eq!(calls[1].len(), 1);
        assert_eq!(calls[1][0].0, VK_BACK);
        assert_ne!(calls[1][0].2 & KEYEVENTF_KEYUP, 0);
    }

    #[test]
    fn mocked_rejection_or_full_acceptance_never_retries_batch() {
        let inputs = [
            create_utf16_input('a' as u16, false),
            create_utf16_input('a' as u16, true),
        ];
        for accepted in [0u32, 2u32] {
            let mut calls = 0;
            let (ok, count) = send_checked_using(&inputs, |_batch| {
                calls += 1;
                accepted
            });
            assert_eq!(calls, 1);
            assert_eq!(count, accepted);
            assert_eq!(ok, accepted == 2);
        }
    }

    #[test]
    fn async_target_requires_same_window_process_and_thread() {
        let expected = InjectionTarget {
            hwnd: 0x1042,
            thread_id: 313,
            process_id: 718,
        };
        assert!(matches_target(expected, Some(expected)));
        assert!(!matches_target(expected, None));
        assert!(!matches_target(
            expected,
            Some(InjectionTarget {
                hwnd: 0x1043,
                ..expected
            })
        ));
        assert!(!matches_target(
            expected,
            Some(InjectionTarget {
                thread_id: 314,
                ..expected
            })
        ));
        assert!(!matches_target(
            expected,
            Some(InjectionTarget {
                process_id: 719,
                ..expected
            })
        ));
    }

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
