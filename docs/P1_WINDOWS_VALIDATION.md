# VietFlow P1 Windows acceptance matrix

This matrix distinguishes automated implementation checks from independent
Windows acceptance. Do not consider a passing unit build proof that a real
desktop text control or installed MSI works.

## Regression evidence

| Scope | Checks / evidence |
|---|---|
| F02 Enter/Tab auto-switch | English-to-Vietnamese word-break rewriting now builds one checked SendInput batch: backspaces, UTF-16 replacement, then exactly one Enter/Tab; Space/punctuation is emitted once inside text. Safe/oversized requests keep native break and original English mode; scan-code uses one checked batch instead of accepting a delayed rewrite. Rejected edits restore speculative lookback and auto-switch state; four focused native Windows unit tests, real multi-app E2E pending |
| F03 Unicode | Windows UTF-16 surrogate-pair regression and Safe-mode scalar builder now emit matched key-down/up events for every UTF-16 unit; an unused single-INPUT helper that silently truncated non-BMP scalars was replaced. Actual mixed-app non-BMP text E2E remains pending |
| F04 UTF-8 C ABI | Owned UTF-8 results replace fixed 256/512-byte TSF output buffers; caller frees Rust memory explicitly. Legacy API reports full required bytes and rejects insufficient capacity without partial copying. Rust tests include long macro/emoji and C++/Rust native ABI smoke passes; real TSF typing and sanitizer/architecture matrix still pending |
| F05 Installer ACL | Offline WiX authoring regression now asserts installed binaries under Program Files have no explicit permission overrides, and mutation-tests a restored `Users:GenericAll` grant to ensure CI rejects it. ProgramData has a separate writable configuration subtree. These static safeguards do **not** prove that old installed ACL grants are removed during an upgrade; real admin/standard-user VM acceptance remains pending |
| F06 Update navigation | The Portable release manifest accepts only official VietFlow GitHub release/tag-page URLs; invalid/external/encoded redirect candidates fall back to the canonical releases page. Release versions and official API tags are validated; prerelease version ordering now compares dotted numeric SemVer identifiers (e.g. `rc.10` later than `rc.2`) and ignores build metadata. The MSI downloader now accepts only canonical repository release-asset URLs, never raw CDN or arbitrary repository paths, and requires exact GitHub API asset size and SHA-256 before launch. MSI auto-launch fails closed unless Windows WinVerifyTrust validates the package Authenticode chain, after size/SHA-256 verification. A read-only, share-read-only file handle prevents concurrent writes or replacement during verification and launch dispatch; it does not guard the file after updater exit. Unsigned or untrusted MSI packages require manual release-page navigation. This verifies Windows certificate trust but does not pin a VietFlow publisher certificate: exact publisher identity, real signed-MSI VM installation, and live redirect/network faults remain release gates |
| F07 Architecture | MSI and ZIP packaging now share a canonical dist-only EXE/TSF resolver, checking PE architecture x64/x86/ARM64 with no stale build/target fallback. Offline positive/negative tests run on Windows CI. Cross-architecture real MSI installation and signing still pending |
| F08 Settings | Production load first reads a valid primary or last valid .bak (without destroying corrupt primary), atomic Rust/C++ staged saves, interprocess writer lock and corrupted-primary recovery regression |
| F09 Uninstall | MSI no longer forcibly terminates running IME, allows same-version major replacement, or unregisters TSF during an upgrade or invokes blanket shortcut cleanup under SYSTEM. Final uninstall uses a separate TSF COM-only command pointed at the installed sibling DLL, without rewriting user preferences; offline authoring tests detect forced termination, same-version replacement and missing upgrade guards. A native WiX 5.0.2 x64 MSI was compiled and all ICE validation checks returned zero errors/warnings; CI compiles/decompiles MSI x64 without installing it. Real MSI/VM and multiuser registry cleanup still pending |
| F10 Hook callback | Injection worker and sound worker prewarmed before hook registration; notification chime uses one-slot nonblocking channel (bursts dropped, no thread creation inside hook). Injection worker also prewarmed; bounded nonblocking 8-job queue, 750 ms pre-readiness expiry and exact foreground identity; new two-phase handoff requires worker readiness within 5 ms AND an explicit hook commit before suppressing original key. Queued-but-not-ready, expired, canceled, disconnected or wrong-target jobs fail open without later injection; 64 KiB text / 4096-backspace limits. Macro-only word breaks also use synchronous atomic rewrite+delimiter in Unicode/scan-code modes instead of queuing a delayed expansion; Safe paced and oversized macros keep their original typed trigger and break key. Modified shortcuts (Ctrl/Alt/Win+Enter, Tab, Backspace, arrows; Shift+editing keys) bypass expansion, and Ctrl+Backspace invalidates the whole English AutoSwitch word rather than popping a character. Clipboard-backed macro-only expansion uses a bounded 32-scalar/128-byte preflight and nonblocking cache try-lock; normal Vietnamese-mode macro expansion also applies a 64-KiB nonblocking hook budget. Windows CF_UNICODETEXT reads are bounded to GlobalSize and require a NUL terminator plus valid UTF-16 within the allocation. Malformed/cleared/unavailable clipboard data invalidates stale cache; a missing clipboard preserves the raw trigger rather than silently expanding it to empty text. Large clipboard content or contention cannot stall the hook or trigger a late replacement. Global toggle and settings hotkeys now use an independent per-key atomic press latch: one action per physical key-down across auto-repeat, matching consumed key-up is suppressed, modifier hotkeys retain native key-up, and key-up re-arms even when STATE is contended. The latch is cleared on hook reinstallation. Manual V/E hotkey toggles no longer run config JSON serialization or filesystem writes inside WH_KEYBOARD_LL: they set an atomic pending flag, PostMessage the daemon window to persist the newest config outside the hook, retry on the 250-ms config watcher timer, and flush on window destruction. Independent tests verify coalesced/re-armed requests and nonblocking request under an already-held STATE mutex. Post-commit focus/SendInput failures and rapid typing order cannot be guaranteed by worker handoff, so mixed-app, 24h and latency p95/p99 E2E remain pending |
| F04/F11 Rust lint | Full production workspace passes strict Clippy (`cargo clippy --workspace --locked -- -D warnings`), enforced by the Windows Test CI job. Four owned-UTF8 C ABI calls document caller pointer lifetime, aliasing and free-before-reuse contracts. The previously outstanding test-only lint diagnostics are resolved; full `cargo clippy --workspace --all-targets --locked -- -D warnings` passes locally and is enforced by Windows CI |
| F11 SendInput | Immediate Unicode, scan-code, Auto and Safe dispatch now reports real success/failure instead of unconditionally consuming physical keys after a zero-event SendInput rejection. Ctrl/Alt/Win+Backspace in Vietnamese hook mode bypasses IME composition and retains native editor shortcut behavior; the tentative Vietnamese-to-English AutoSwitch is committed only after a complete synchronous SendInput replacement, never a later-cancelable worker admission. Atomic word-break dispatch checks the whole event count; partial input stops without replaying accepted destructive backspaces/code units and releases pressed keys. Synthetic zero/partial/full, UTF-16 and extended scan-code tests pass. Cleanup of partially inserted input now matches Unicode/scancode/extended-key flags as well as VK and scan bytes, so an E0-prefixed keypad key cannot be confused with a same-scan ordinary key. Independent real-host SendInput fault injection, destructive-event rollback behavior and editor E2E remain pending |
| F12 TSF IPC | Rust writer clamps malformed enum/flag/version fields before publishing; C++ reader rejects malformed yet sequence-consistent snapshots and retains last valid config. Independent native smoke tests cover every field; the C++ reader also checks the sampled sequence field itself, and a native two-process stress test covers 16,000 correlated field updates through named shared memory. The process test requires the final committed B profile even if CI schedules the writer after the transient reader loop. AppContainer ACL, low-integrity writes, multiple user sessions, TIP/editor E2E, and long-run tests remain pending |
| F13 Held-key handling | The hook already tracks 256 VK states independently for simultaneous WASD movement. The non-modifier auto-repeat path now bounds-checks VK before touching that table; malformed/out-of-range codes pass through instead of causing an indexing panic in the Windows callback. New tests verify simultaneous keys, independent releases, stale-state recovery and out-of-range safety. Real mixed-key game/editor E2E and contention remain pending |
| F14 Keyboard layouts | The hook's single-character Telex translation follows the foreground keyboard HKL and passes dead keys/multi-unit translations natively. Scan-code injection now uses the foreground window's HKL via `VkKeyScanExW` and `MapVirtualKeyExW`; it sets `KEYEVENTF_SCANCODE` (plus `KEYEVENTF_EXTENDEDKEY` for E0) with `wVk=0` for real physical scancodes. Ctrl/Alt/AltGr-dependent and unsupported physical mappings fall back to Unicode instead of emitting a misleading bare VK. Pure modifier regression tests pass. The in-process TSF TIP now uses `ToUnicodeEx` with state-preserving flag 0x04 on the active thread HKL and classifies actual ASCII characters (including translated VNI digits/Telex brackets/VIQR marks), rather than hard-coded US virtual keys. Dead keys, multi-unit, non-ASCII and AltGr shortcuts are left for Windows; if the TSF OnKeyDown callback runs with a live preedit, it finalizes that preedit before allowing the native key. A native isolated C++ smoke checks all already-loaded HKLs (five on the staging Windows host) without changing user keyboard settings. Windows 10 1607+ is required for the no-state-change flag. Real TSF callback ordering and editor/AltGr/dead-key E2E remain pending |
| Q01 Release | Official publisher routes to one verified gate; default is offline. Preflight verifies PE/ZIP architecture, genuine read-only Windows Installer MSI database identity, 12 payload hashes, updater URLs, SHA256SUMS and 6 metadata assets. A shared Windows Installer COM verifier now reads actual MSI ProductVersion, ProductCode, UpgradeCode, architecture, scope and language in both manifest generation and publishing; minimal synthetic MSI databases and 20 positive/negative offline scenarios run in CI. Canonical-dist packaging regression prevents stale MSI/ZIP inputs. Explicit publish still requires clean source, matching origin/main and tag, plus remote asset digests; signing/MSI VM/public release remain NO-GO pending independent approval |
| Q02 Independent E2E | Matrix below must be run by QA and recorded for target release |

### F06 Authenticode positive and tamper QA

The source includes an ignored, explicitly opt-in Windows test
`valid_windows_signature_passes_winverifytrust_when_fixture_is_supplied`.
Set `VIETFLOW_SIGNED_TEST_FILE` to a trusted, signed Windows binary and run
the filtered Rust test with `--ignored`. It checks that WinVerifyTrust accepts
the unchanged fixture and rejects a separate temporary copy with one signed
image byte modified. On the staging Windows host, the read-only Google-signed
Chrome executable was used and both checks passed. This **does not** establish
that a real VietFlow MSI is signed by the intended publisher or installs safely.

## Offline publisher gate (Q01)

The safe entry point is scripts/publish-release.ps1 without -Publish. It
checks local artifact provenance, and neither creates nor edits a release.
Run scripts/tests/test_release_preflight.ps1 to verify the synthetic fixtures
and rejection cases without contacting GitHub.

Both publisher entry points require an explicit -Publish for remote release
mutations. The legacy -Draft flag is preflight-only; -Prerelease is disabled
until a reviewed prerelease workflow exists.

A passing preflight is NOT approval to publish. Signed assets, Windows MSI
install/upgrade/uninstall acceptance and real TSF/hook input tests require
independent evidence before any production release.

MSI inspection uses Windows Installer COM read-only database APIs to verify
Package properties, architecture, ProductCode/UpgradeCode and installation
scope. It does not install the MSI, extract its CAB payloads or prove code
signing. A minimal fixture may pass identity checks without being installable;
independent WiX compilation/ICE validation and real VM install/repair/
upgrade/uninstall tests are separate release gates.

## Manual / independent test IDs

| ID | Matrix and exact assertions |
|---|---|
| E01 | Windows 10/11 fresh user: Vietnamese typing in Word, Notepad, Chrome/Firefox, legacy editor and console; no duplicate or dropped keys |
| E02 | TSF active then fallback app: confirm hook still types, no double injection, no lost foreground focus |
| E03 | TSF composition Return, Shift+Return, Tab, Esc; plus Hook English auto-switch Enter/Tab/Space with Auto, Unicode, Safe and scan-code modes, including SendInput rejection. Exactly one host break action per physical key, correct word-before-break ordering, no duplicate spaces |
| E04 | Telex/VNI/VIQR plus supplementary-plane Unicode and mixed English; exact Unicode roundtrip |
| E05 | Short and very large macros plus Safe/scancode modes: ensure 5 ms readiness timeout passes original key exactly once; in macro-only and normal Vietnamese modes confirm Enter/Tab/space/punctuation order, Ctrl+Enter, Alt+Tab, Shift+Tab/Arrow, Ctrl+Backspace editor deletion, and huge/contended/malformed/cleared clipboard-placeholder fallback without stalled hook, overread or erased trigger. Force congested worker and focus changes before/after commit; measure hook p50/p95/p99/max, memory/CPU and 24h recovery |
| E06 | Force SendInput zero, partial and complete returns; verify Vietnamese Ctrl/Alt/Win+Backspace is not rewritten, failed Vietnamese-to-English AutoSwitch preserves language and native key, and Safe/oversized AutoSwitch never schedules a delayed language flip. Check no duplicate destructive backspaces or stranded key-down event |
| E07 | Concurrent Rust daemon + C++ settings saves; forced interruption before rename; corrupted JSON; keep a valid last backup and macros |
| E08 | Standard user, admin, low-integrity and AppContainer mapping access: verify config cannot be modified, focus signal still works; tamper attempts logged |
| E09 | Install/repair/upgrade/uninstall MSI x64, x86, ARM64 in VMs. Inspect ACL, shortcuts, TSF registration and preservation of user settings/sentinel files |
| E10 | Intentionally remove one binary or corrupt one ZIP; packaging and publisher must fail before any public release |
| E11 | Repeat cold starts and focus/layout changes 100 times; hold Ctrl+Space, Alt+Z, CapsLock and custom settings hotkeys through OS key-repeat and release, verify exactly one toggle/action per press and no stray key-up in an editor. Force STATE contention during key-up and verify next press is re-armed; delay the config JSON write and verify keyboard hook latency plus latest V/E mode persistence after restart; 24-hour mixed-app typing without hook unregistration |

### Requirements for every E2E receipt

Capture source commit SHA, Windows build and architecture, app versions,
test actions and expected/observed output, timestamp, tester, pass/fail,
and sanitized logs or video hashes where permitted. If hardware, VM,
installer or UI automation is unavailable, report BLOCKED, never PASS.

### Known boundaries

- A TSF focus channel writable by AppContainers is an advisory signal; same-user
  applications may spoof their own focus. The settings configuration mapping
  must never grant AppContainer write access.
- Source changes to WiX permissions do not automatically remove ACL grants
  left by a previously installed older version. Test old-to-new upgrades.
- Bounded queue rejects oversize, unavailable or saturated work before the hook
  eats the physical key; the hook resets composition and allows native passthrough.
  Once a job is already admitted, subsequent window changes/expiry may still
  cancel an output; this cannot be treated as lossless input without more
  extensive text-control synchronization. Verify with real apps before release.
- Final MSI uninstall can remove machine COM/TSF registration under SYSTEM, but
  per-user TIP registrations of other Windows profiles may remain. Never claim
  multi-user cleanup until a tested user-context deactivation route exists.
- Same-version major upgrade replacement is disabled to satisfy ICE61. MSI
  compares only the first three version fields; repackaging a different
  ProductCode under the same version can still result in side-by-side products.
  Bump the first three version fields and preserve the release-tag immutability
  gate for every production update.
- MSI-owned shortcuts are removed by Windows Installer. Legacy user-created
  shortcuts and per-user startup entries are not forcibly deleted by SYSTEM.
- Exact official release metadata integrity does not replace vendor code signing.
