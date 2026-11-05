// VietFlow Windows - Keystroke Injection (Atomic, Scan-code, Safe)
// Licensed under GNU General Public License v3.0 (GPLv3)

use std::cell::Cell;
use std::mem::size_of;
use std::sync::{mpsc, OnceLock};
use std::thread::sleep;
use std::time::{Duration, Instant};
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

// A slow target application must not accumulate an unbounded backlog of
// key events that could be delivered seconds later to the wrong text field.
const MAX_PENDING_INJECTION_JOBS: usize = 8;
// Queue is bounded by both count and payload size, otherwise eight very
// large user macros still consume unbounded memory in the keyboard callback.
pub(crate) const MAX_PENDING_TEXT_BYTES: usize = 64 * 1024;
const MAX_PENDING_BACKSPACES: usize = 4_096;
const MAX_PENDING_JOB_AGE: Duration = Duration::from_millis(750);
// A hook must not swallow the physical key merely because a queue accepted a
// job. Give a prewarmed worker a very short chance to confirm it is ready.
// On timeout the physical key passes through and the worker cannot execute it.
const MAX_WORKER_READY_WAIT: Duration = Duration::from_millis(5);
const ATOMIC_BACKSPACE_LIMIT: usize = 16;
const ATOMIC_CHAR_LIMIT: usize = 32;
/// Maximum UTF-8 bytes in a 32-scalar atomic macro replacement.
/// Use this BEFORE copying cached clipboard macros inside a keyboard hook.
pub const MAX_ATOMIC_WORD_BREAK_UTF8_BYTES: usize = ATOMIC_CHAR_LIMIT * 4;
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
    queued_at: Instant,
    // Two-phase handoff. Worker may inject only after the keyboard hook
    // explicitly commits to consuming the physical key.
    ready_sender: mpsc::SyncSender<()>,
    commit_receiver: mpsc::Receiver<()>,
}

fn eligible_queued_job(
    job: &InjectionJob,
    foreground: Option<InjectionTarget>,
    now: Instant,
) -> bool {
    matches_target(job.target, foreground)
        && now
            .checked_duration_since(job.queued_at)
            .is_some_and(|age| age <= MAX_PENDING_JOB_AGE)
}

fn try_enqueue(
    sender: &mpsc::SyncSender<InjectionJob>,
    job: InjectionJob,
) -> Result<(), mpsc::TrySendError<InjectionJob>> {
    // Crucially, this never waits for a slow worker on WH_KEYBOARD_LL.
    sender.try_send(job)
}

/// The worker may announce readiness only for a fresh, same-foreground job.
/// Until it receives a commit from the hook, it MUST NOT touch SendInput.
/// The hook always drops its commit sender on timeout, so this recv cannot
/// outlive the callback even if the callback exits due to an error.
fn wait_for_queued_job_commit(
    job: &InjectionJob,
    foreground: Option<InjectionTarget>,
    now: Instant,
) -> bool {
    eligible_queued_job(job, foreground, now)
        && job.ready_sender.send(()).is_ok()
        && job.commit_receiver.recv().is_ok()
}

/// The sole decision point that authorizes suppressing a native key.
/// This wait is bounded and occurs only on the asynchronous replacement path.
fn approve_prepared_job(
    ready_receiver: mpsc::Receiver<()>,
    commit_sender: mpsc::SyncSender<()>,
) -> bool {
    if ready_receiver.recv_timeout(MAX_WORKER_READY_WAIT).is_err() {
        // Drop commit_sender; the worker will either see disconnection or
        // never be able to notify readiness. In both cases it must not inject.
        return false;
    }
    commit_sender.send(()).is_ok()
}

static INJECTION_TX: OnceLock<Option<mpsc::SyncSender<InjectionJob>>> = OnceLock::new();

/// Initialize the bounded worker before installing WH_KEYBOARD_LL.
/// A failure is surfaced without panic; no thread is started from the hook.
pub fn prewarm_async_injector() -> bool {
    INJECTION_TX
        .get_or_init(|| {
            let (tx, rx) = mpsc::sync_channel::<InjectionJob>(MAX_PENDING_INJECTION_JOBS);
            match std::thread::Builder::new()
                .name("vietflow-async-injector".into())
                .spawn(move || {
                    while let Ok(job) = rx.recv() {
                        // No original key may be consumed while this worker
                        // waits behind older jobs or rejects a stale target.
                        if !wait_for_queued_job_commit(
                            &job,
                            InjectionTarget::foreground(),
                            Instant::now(),
                        ) {
                            continue;
                        }
                        // Check again immediately before any OS input. A focus
                        // switch after consent cannot be sent to a new window.
                        if !matches_target(job.target, InjectionTarget::foreground()) {
                            continue;
                        }
                        ASYNC_TARGET.with(|target| target.set(Some(job.target)));
                        execute_injection_job(job.backspaces, &job.text, job.mode);
                        ASYNC_TARGET.with(|target| target.set(None));
                    }
                }) {
                Ok(_) => Some(tx),
                Err(err) => {
                    eprintln!("VietFlow async injector unavailable: {err}");
                    None
                }
            }
        })
        .is_some()
}

fn acceptable_queued_payload(backspaces: usize, text_bytes: usize) -> bool {
    backspaces <= MAX_PENDING_BACKSPACES && text_bytes <= MAX_PENDING_TEXT_BYTES
}

/// True means a fresh worker has confirmed readiness and received the
/// hook's commit. Only then may the hook suppress the physical key.
/// False means the job was rejected, the worker was not ready within 5 ms,
/// or its handoff failed; it cannot execute and the key MUST pass through.
pub fn enqueue_async_injection(backspaces: usize, text: String, mode: u8) -> bool {
    if !acceptable_queued_payload(backspaces, text.len()) {
        return false;
    }
    let Some(target) = InjectionTarget::foreground() else {
        return false;
    };
    // Do not implicitly spawn a worker from the keystroke callback.
    let Some(sender) = INJECTION_TX.get().and_then(Option::as_ref) else {
        return false;
    };
    let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
    let (commit_sender, commit_receiver) = mpsc::sync_channel(1);
    if try_enqueue(
        sender,
        InjectionJob {
            backspaces,
            text,
            mode,
            target,
            queued_at: Instant::now(),
            ready_sender,
            commit_receiver,
        },
    )
    .is_err()
    {
        return false;
    }
    approve_prepared_job(ready_receiver, commit_sender)
}

fn execute_injection_job(backspaces: usize, text: &str, mode: u8) {
    match mode {
        2 => {
            let _ = send_scancode_batch(backspaces, text);
        }
        3 | 4 => {
            let _ = send_safe_batch(backspaces, text);
        }
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
/// Returns whether replacement injection was authorized. In async modes,
/// readiness timeout/cancellation must leave the native key available.
pub fn send_atomic_batch_with_mode(backspaces: usize, text: &str, mode: u8) -> bool {
    // Validate before traversing and allocating a potentially untrusted macro.
    if !acceptable_queued_payload(backspaces, text.len()) {
        return false;
    }
    let char_count = text.chars().count();
    let is_large_or_paced = char_count > ATOMIC_CHAR_LIMIT
        || backspaces > ATOMIC_BACKSPACE_LIMIT
        || mode == 3
        || mode == 4;

    if is_large_or_paced {
        return enqueue_async_injection(backspaces, text.to_string(), mode);
    }

    match mode {
        2 => send_scancode_batch(backspaces, text),
        3 | 4 => send_safe_batch(backspaces, text),
        1 => send_unicode_batch_status(backspaces, text).0,
        _ => {
            // When zero original events were inserted, a safe fallback may
            // still work. Never retry a destructive partially accepted batch.
            let (success, inserted) = send_unicode_batch_status(backspaces, text);
            if !success && inserted == 0 {
                send_safe_batch(backspaces, text)
            } else {
                success
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

/// Fill the complete replacement and optional physical word-break key into
/// one SendInput buffer. This is the only safe ordering for Enter/Tab after an
/// English->Vietnamese conversion; queuing the conversion first can cause the
/// native key to arrive before the replacement.
fn fill_atomic_unicode_inputs(
    backspaces: usize,
    text: &str,
    break_vk: Option<u16>,
    inputs: &mut [INPUT; MAX_ATOMIC_INPUTS],
) -> Option<usize> {
    if backspaces > ATOMIC_BACKSPACE_LIMIT || text.chars().count() > ATOMIC_CHAR_LIMIT {
        return None;
    }
    if break_vk.is_some_and(|vk| vk != 0x0D && vk != 0x09) {
        return None;
    }
    let mut count = 0usize;
    let scan = unsafe { MapVirtualKeyW(VK_BACK as u32, 0) as u16 };
    for _ in 0..backspaces {
        inputs[count] = create_vk_input_scanned(VK_BACK, scan, false);
        inputs[count + 1] = create_vk_input_scanned(VK_BACK, scan, true);
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
    if let Some(vk) = break_vk {
        inputs[count] = create_vk_input(vk, false);
        inputs[count + 1] = create_vk_input(vk, true);
        count += 2;
    }
    Some(count)
}

/// For word-break replacement, never queue a rewrite behind the native
/// key. Unicode and scan-code modes can use a single checked SendInput batch;
/// paced Safe mode must preserve the original typed text rather than reorder.
fn can_send_atomic_break(mode: u8, backspaces: usize, text: &str) -> bool {
    (mode == 0 || mode == 1 || mode == 2)
        && acceptable_queued_payload(backspaces, text.len())
        && backspaces <= ATOMIC_BACKSPACE_LIMIT
        && text.chars().count() <= ATOMIC_CHAR_LIMIT
}

/// Atomically insert replacement + optional Enter/Tab in the same OS input
/// transaction, reporting exact SendInput acceptance. On rejection, the
/// original physical key must pass through and language must not be toggled.
pub fn send_atomic_word_break(
    backspaces: usize,
    text: &str,
    break_vk: Option<u16>,
    mode: u8,
) -> bool {
    if !can_send_atomic_break(mode, backspaces, text) {
        return false;
    }
    if mode == 2 {
        // A single scan-code transaction is required even when a macro
        // includes Unicode fallback characters.
        let Some(inputs) = build_scancode_inputs(backspaces, text, break_vk) else {
            return false;
        };
        return send_checked(&inputs).0;
    }
    let mut inputs: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
    let Some(count) = fill_atomic_unicode_inputs(backspaces, text, break_vk, &mut inputs) else {
        return false;
    };
    send_checked(&inputs[..count]).0
}

/// Dispatches unicode batch returning (all_inserted, total_events_inserted)
fn send_unicode_batch_status(backspaces: usize, text: &str) -> (bool, u32) {
    if backspaces == 0 && text.is_empty() {
        return (true, 0);
    }

    let char_count = text.chars().count();

    if backspaces <= ATOMIC_BACKSPACE_LIMIT && char_count <= ATOMIC_CHAR_LIMIT {
        let mut inputs: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
        // Same checked, stack-allocated input builder as word-break ordering.
        let count = fill_atomic_unicode_inputs(backspaces, text, None, &mut inputs)
            .expect("validated atomic injection limits");
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

/// Construct a bounded, ordered scan-code transaction. ASCII uses the
/// active layout's VK and non-mappable characters use UTF-16 fallback;
/// the optional Enter/Tab always comes last.
fn build_scancode_inputs(
    backspaces: usize,
    text: &str,
    break_vk: Option<u16>,
) -> Option<Vec<INPUT>> {
    if break_vk.is_some_and(|vk| vk != 0x0D && vk != 0x09) {
        return None;
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

    if let Some(vk) = break_vk {
        inputs.push(create_vk_input(vk, false));
        inputs.push(create_vk_input(vk, true));
    }
    Some(inputs)
}

/// Scan-code mode for DirectInput-style windows. Unmappable characters
/// fall back to Unicode rather than silently losing Vietnamese text.
fn send_scancode_batch(backspaces: usize, text: &str) -> bool {
    let Some(inputs) = build_scancode_inputs(backspaces, text, None) else {
        return false;
    };
    if !inputs.is_empty() {
        return send_checked(&inputs).0;
    }
    true
}

/// Compatibility mode: full backspaces, then one Unicode character at a time
/// with a tiny pause. Slower, but survives apps that drop fast batches.
fn send_safe_batch(backspaces: usize, text: &str) -> bool {
    if backspaces > 0 {
        let mut inputs = Vec::with_capacity(backspaces * 2);
        for _ in 0..backspaces {
            inputs.push(create_vk_input(VK_BACK, false));
            inputs.push(create_vk_input(VK_BACK, true));
        }
        let (complete, _) = send_checked(&inputs);
        if !complete {
            return false;
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
            return false;
        }
        sleep(Duration::from_millis(1));
    }
    true
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
    fn word_break_atomic_batch_orders_backspaces_utf16_then_enter_tab() {
        for vk in [0x0Du16, 0x09] {
            let mut events: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
            let text = "Vi\u{1EC7}t\u{1F980}";
            let utf16_len = text.encode_utf16().count();
            let len = fill_atomic_unicode_inputs(2, text, Some(vk), &mut events).unwrap();
            assert_eq!(len, 4 + 2 * utf16_len + 2);
            for i in 0..4 {
                let key = unsafe { events[i].Anonymous.ki };
                assert_eq!(key.wVk, VK_BACK);
                assert_eq!(key.dwFlags & KEYEVENTF_UNICODE, 0);
            }
            for event in &events[4..len - 2] {
                let key = unsafe { event.Anonymous.ki };
                assert_eq!(key.wVk, 0);
                assert_ne!(key.dwFlags & KEYEVENTF_UNICODE, 0);
            }
            let down = unsafe { events[len - 2].Anonymous.ki };
            let up = unsafe { events[len - 1].Anonymous.ki };
            assert_eq!(down.wVk, vk);
            assert_eq!(down.dwFlags & KEYEVENTF_KEYUP, 0);
            assert_eq!(up.wVk, vk);
            assert_ne!(up.dwFlags & KEYEVENTF_KEYUP, 0);
            assert_eq!(up.dwFlags & KEYEVENTF_UNICODE, 0);
        }
    }

    #[test]
    fn word_break_space_is_emitted_exactly_once_without_virtual_break() {
        let mut events: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
        let len = fill_atomic_unicode_inputs(0, "Vi\u{1EC7}t ", None, &mut events).unwrap();
        assert_eq!(len, 2 * "Vi\u{1EC7}t ".encode_utf16().count());
        let space_down = unsafe { events[len - 2].Anonymous.ki };
        let space_up = unsafe { events[len - 1].Anonymous.ki };
        assert_eq!(space_down.wVk, 0);
        assert_eq!(space_down.wScan, ' ' as u16);
        assert_eq!(space_up.wScan, ' ' as u16);
        assert_ne!(space_up.dwFlags & KEYEVENTF_KEYUP, 0);
    }

    #[test]
    fn break_switch_refuses_paced_oversized_or_unsupported_input() {
        for mode in [0, 1, 2] {
            assert!(can_send_atomic_break(mode, 3, "Vi\u{1EC7}t"));
        }
        for mode in [3, 4, 255] {
            assert!(!can_send_atomic_break(mode, 3, "Vi\u{1EC7}t"));
        }
        assert!(!can_send_atomic_break(0, ATOMIC_BACKSPACE_LIMIT + 1, "a"));
        assert!(!can_send_atomic_break(
            0,
            0,
            &"a".repeat(ATOMIC_CHAR_LIMIT + 1)
        ));
        let mut events: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
        assert!(fill_atomic_unicode_inputs(0, "vi", Some(VK_BACK), &mut events).is_none());
        assert!(
            fill_atomic_unicode_inputs(ATOMIC_BACKSPACE_LIMIT + 1, "a", None, &mut events)
                .is_none()
        );
        assert!(fill_atomic_unicode_inputs(
            0,
            &"a".repeat(ATOMIC_CHAR_LIMIT + 1),
            None,
            &mut events
        )
        .is_none());
    }

    #[test]
    fn scancode_word_break_keeps_enter_after_ascii_and_unicode_fallback() {
        let events = build_scancode_inputs(2, "hi\u{1EC7}n", Some(0x0D)).unwrap();
        assert!(events.len() >= 12);
        for event in &events[..4] {
            let ki = unsafe { event.Anonymous.ki };
            assert_eq!(ki.wVk, VK_BACK);
        }
        let tail_down = unsafe { events[events.len() - 2].Anonymous.ki };
        let tail_up = unsafe { events[events.len() - 1].Anonymous.ki };
        assert_eq!(tail_down.wVk, 0x0D);
        assert_eq!(tail_up.wVk, 0x0D);
        assert_eq!(tail_down.dwFlags & KEYEVENTF_KEYUP, 0);
        assert_ne!(tail_up.dwFlags & KEYEVENTF_KEYUP, 0);
        assert!(events[4..events.len() - 2].iter().any(|event| {
            let ki = unsafe { event.Anonymous.ki };
            ki.wVk == 0 && ki.wScan == 0x1EC7 && (ki.dwFlags & KEYEVENTF_UNICODE) != 0
        }));
        assert!(build_scancode_inputs(2, "abc", Some(VK_BACK)).is_none());
    }

    #[test]
    fn macro_only_atomic_delimiters_are_last_and_not_duplicated() {
        for delimiter in [' ', '.', '!'] {
            let output = format!("V\u{1EC7}n{delimiter}");
            let mut events: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
            let count = fill_atomic_unicode_inputs(2, &output, None, &mut events).unwrap();
            assert_eq!(count, 4 + 2 * output.encode_utf16().count());
            let tail = unsafe { events[count - 2].Anonymous.ki };
            assert_eq!(tail.wVk, 0);
            assert_eq!(tail.wScan, delimiter as u16);
            assert_ne!(tail.dwFlags & KEYEVENTF_UNICODE, 0);
            assert_eq!(
                unsafe { events[count - 1].Anonymous.ki }.wScan,
                delimiter as u16
            );
        }
    }

    #[test]
    fn rejected_atomic_word_break_never_delivers_enter_separately() {
        let mut events: [INPUT; MAX_ATOMIC_INPUTS] = unsafe { std::mem::zeroed() };
        let count = fill_atomic_unicode_inputs(1, "V\u{1EC7}", Some(0x0D), &mut events).unwrap();
        let mut calls = 0;
        let (ok, inserted) = send_checked_using(&events[..count], |batch| {
            calls += 1;
            assert_eq!(batch.len(), count);
            0 // Windows rejected ALL events; the physical Enter must pass.
        });
        assert!(!ok);
        assert_eq!(inserted, 0);
        assert_eq!(calls, 1);
        // A partial even-numbered prefix has no pressed keys to release,
        // and never retries the original batch or sends trailing Enter.
        let mut prefixes = Vec::<usize>::new();
        let (ok, inserted) = send_checked_using(&events[..count], |batch| {
            prefixes.push(batch.len());
            2 // Only the backspace key pair was accepted.
        });
        assert!(!ok);
        assert_eq!(inserted, 2);
        assert_eq!(prefixes, vec![count]);
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

    fn queued_test_job(target: InjectionTarget, queued_at: Instant) -> InjectionJob {
        let (ready_sender, _ready_receiver) = mpsc::sync_channel(1);
        let (_commit_sender, commit_receiver) = mpsc::sync_channel(1);
        InjectionJob {
            backspaces: 2,
            text: "?i?n".into(),
            mode: 3,
            target,
            queued_at,
            ready_sender,
            commit_receiver,
        }
    }

    fn test_handoff(
        target: InjectionTarget,
        queued_at: Instant,
    ) -> (InjectionJob, mpsc::Receiver<()>, mpsc::SyncSender<()>) {
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let (commit_sender, commit_receiver) = mpsc::sync_channel(1);
        (
            InjectionJob {
                backspaces: 1,
                text: "?".into(),
                mode: 4,
                target,
                queued_at,
                ready_sender,
                commit_receiver,
            },
            ready_receiver,
            commit_sender,
        )
    }

    #[test]
    fn bounded_injection_queue_never_blocks_when_full() {
        let (sender, receiver) = mpsc::sync_channel::<InjectionJob>(1);
        let target = InjectionTarget {
            hwnd: 0x1234,
            process_id: 43,
            thread_id: 91,
        };
        assert!(try_enqueue(&sender, queued_test_job(target, Instant::now())).is_ok());
        assert!(matches!(
            try_enqueue(&sender, queued_test_job(target, Instant::now())),
            Err(mpsc::TrySendError::Full(_)),
        ));
        let accepted = receiver
            .try_recv()
            .expect("the first queued job remains intact");
        assert_eq!(accepted.text, "?i?n");
        assert!(try_enqueue(&sender, queued_test_job(target, Instant::now())).is_ok());
        drop(receiver);
        assert!(matches!(
            try_enqueue(&sender, queued_test_job(target, Instant::now())),
            Err(mpsc::TrySendError::Disconnected(_)),
        ));
    }

    #[test]
    fn ready_worker_requires_explicit_hook_commit_before_injecting() {
        let target = InjectionTarget {
            hwnd: 0x2345,
            process_id: 45,
            thread_id: 9,
        };
        let (job, ready, commit) = test_handoff(target, Instant::now());
        let worker = std::thread::spawn(move || {
            wait_for_queued_job_commit(&job, Some(target), Instant::now())
        });
        assert!(ready.recv_timeout(Duration::from_millis(100)).is_ok());
        assert!(commit.send(()).is_ok());
        assert!(
            worker.join().unwrap(),
            "only an acknowledged job may inject"
        );
    }

    #[test]
    fn prepared_job_authorization_signal_is_bounded_and_explicit() {
        let (ready_sender, ready_receiver) = mpsc::sync_channel(1);
        let (commit_sender, commit_receiver) = mpsc::sync_channel(1);
        ready_sender.send(()).unwrap();
        assert!(approve_prepared_job(ready_receiver, commit_sender));
        assert!(commit_receiver
            .recv_timeout(Duration::from_millis(100))
            .is_ok());
    }

    #[test]
    fn worker_does_not_inject_after_hook_cancels_ready_job() {
        let target = InjectionTarget {
            hwnd: 0x2346,
            process_id: 45,
            thread_id: 9,
        };
        let (job, ready, commit) = test_handoff(target, Instant::now());
        let worker = std::thread::spawn(move || {
            wait_for_queued_job_commit(&job, Some(target), Instant::now())
        });
        assert!(ready.recv_timeout(Duration::from_millis(100)).is_ok());
        drop(commit); // Hook passed native key rather than authorizing injection.
        assert!(!worker.join().unwrap());
    }

    #[test]
    fn delayed_or_wrong_target_worker_never_overrides_native_key() {
        let target = InjectionTarget {
            hwnd: 0x2347,
            process_id: 45,
            thread_id: 9,
        };
        let (job, ready, commit) = test_handoff(target, Instant::now());
        assert!(
            !approve_prepared_job(ready, commit),
            "timeout must pass native key without allowing later injection"
        );
        assert!(
            !wait_for_queued_job_commit(&job, Some(target), Instant::now()),
            "late worker cannot inject after hook dropped consent"
        );

        let (job, ready, commit) = test_handoff(target, Instant::now());
        assert!(
            !wait_for_queued_job_commit(&job, None, Instant::now()),
            "worker must reject a changed foreground before readiness"
        );
        assert!(!approve_prepared_job(ready, commit));

        let (job, ready, commit) = test_handoff(
            target,
            Instant::now() - MAX_PENDING_JOB_AGE - Duration::from_millis(1),
        );
        assert!(!wait_for_queued_job_commit(
            &job,
            Some(target),
            Instant::now()
        ));
        assert!(!approve_prepared_job(ready, commit));
    }

    #[test]
    fn queued_job_must_be_fresh_and_foreground_owned() {
        let target = InjectionTarget {
            hwnd: 0x2048,
            process_id: 77,
            thread_id: 218,
        };
        let now = Instant::now();
        let current = queued_test_job(target, now);
        assert!(eligible_queued_job(&current, Some(target), now));
        let expired = queued_test_job(target, now - MAX_PENDING_JOB_AGE - Duration::from_millis(1));
        assert!(!eligible_queued_job(&expired, Some(target), now));
        let boundary = queued_test_job(target, now - MAX_PENDING_JOB_AGE);
        assert!(eligible_queued_job(&boundary, Some(target), now));
        assert!(!eligible_queued_job(&current, None, now));
        assert!(!eligible_queued_job(
            &current,
            Some(InjectionTarget {
                hwnd: 0x2049,
                ..target
            }),
            now
        ));
    }

    #[test]
    fn rejects_oversized_or_unavailable_queue_without_consuming_original_key() {
        assert!(acceptable_queued_payload(
            MAX_PENDING_BACKSPACES,
            MAX_PENDING_TEXT_BYTES
        ));
        assert!(!acceptable_queued_payload(MAX_PENDING_BACKSPACES + 1, 0));
        assert!(!acceptable_queued_payload(0, MAX_PENDING_TEXT_BYTES + 1));
        // Oversize rejection is checked before any access to a real foreground
        // or worker; the false result instructs the hook to pass through.
        assert!(!enqueue_async_injection(
            0,
            "x".repeat(MAX_PENDING_TEXT_BYTES + 1),
            4
        ));
        assert!(!send_atomic_batch_with_mode(
            0,
            &"x".repeat(MAX_PENDING_TEXT_BYTES + 1),
            3,
        ));
        assert!(!send_atomic_batch_with_mode(
            MAX_PENDING_BACKSPACES + 1,
            "",
            0
        ));
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
