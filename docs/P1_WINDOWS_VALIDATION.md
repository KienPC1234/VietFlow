# VietFlow P1 Windows acceptance matrix

This matrix distinguishes automated implementation checks from independent
Windows acceptance. Do not consider a passing unit build proof that a real
desktop text control or installed MSI works.

## Regression evidence

| Scope | Checks / evidence |
|---|---|
| F03 Unicode | Windows injection UTF-16 surrogate-pair unit test; multi-app E2E pending |
| F04 UTF-8 C ABI | Existing Rust UTF-8 boundary tests; C++ integration acceptance pending |
| F07 Architecture | Strict PE checks for x64/x86/ARM64 plus TSF core .lib paths; cross-architecture CI pending |
| F08 Settings | Atomic Rust/C++ staged write, last valid .bak, interprocess writer lock, corrupted primary test |
| F09 Uninstall | WiX authoring inspection, separate clean/upgrade MSI VM tests pending |
| F10 Hook callback | Long macro asynchronous worker exists on current main; real hook latency measurement pending |
| F11 SendInput | Partial input does not blindly replay already acknowledged events; fault injection pending |
| F12 TSF IPC | Rust secure mappings test and isolated C++ focus/config snapshot smoke |
| Q01 Release | Verification script checks architecture, ZIP contents, MSI presence and SHA-256 metadata; publication preflight |
| Q02 Independent E2E | Matrix below must be run by QA and recorded for target release |

## Manual / independent test IDs

| ID | Matrix and exact assertions |
|---|---|
| E01 | Windows 10/11 fresh user: Vietnamese typing in Word, Notepad, Chrome/Firefox, legacy editor and console; no duplicate or dropped keys |
| E02 | TSF active then fallback app: confirm hook still types, no double injection, no lost foreground focus |
| E03 | TSF composition Return, Shift+Return, Tab, Esc; exactly one host action per physical key |
| E04 | Telex/VNI/VIQR plus supplementary-plane Unicode and mixed English; exact Unicode roundtrip |
| E05 | Short and very large macros plus Safe mode: measure hook callback p50/p95/p99/maximum, CPU/memory and recovery after 24h |
| E06 | Force SendInput zero, partial and complete returns; no duplicate destructive backspaces or stranded key-down event |
| E07 | Concurrent Rust daemon + C++ settings saves; forced interruption before rename; corrupted JSON; keep a valid last backup and macros |
| E08 | Standard user, admin, low-integrity and AppContainer mapping access: verify config cannot be modified, focus signal still works; tamper attempts logged |
| E09 | Install/repair/upgrade/uninstall MSI x64, x86, ARM64 in VMs. Inspect ACL, shortcuts, TSF registration and preservation of user settings/sentinel files |
| E10 | Intentionally remove one binary or corrupt one ZIP; packaging and publisher must fail before any public release |
| E11 | Repeat cold starts and focus/layout changes 100 times; 24-hour mixed-app typing without hook unregistration |

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
- Existing async injection can race focus change; no production promotion
  until the independent host-input and latency tests pass.
- Exact official release metadata integrity does not replace vendor code signing.
