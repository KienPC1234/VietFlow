# VietFlow P1 Windows acceptance matrix

This matrix distinguishes automated implementation checks from independent
Windows acceptance. Do not consider a passing unit build proof that a real
desktop text control or installed MSI works.

## Regression evidence

| Scope | Checks / evidence |
|---|---|
| F03 Unicode | Windows injection UTF-16 surrogate-pair unit test; multi-app E2E pending |
| F04 UTF-8 C ABI | Owned UTF-8 results replace fixed 256/512-byte TSF output buffers; caller frees Rust memory explicitly. Legacy API reports full required bytes and rejects insufficient capacity without partial copying. Rust tests include long macro/emoji and C++/Rust native ABI smoke passes; real TSF typing and sanitizer/architecture matrix still pending |
| F07 Architecture | MSI and ZIP packaging now share a canonical dist-only EXE/TSF resolver, checking PE architecture x64/x86/ARM64 with no stale build/target fallback. Offline positive/negative tests run on Windows CI. Cross-architecture real MSI installation and signing still pending |
| F08 Settings | Production load first reads a valid primary or last valid .bak (without destroying corrupt primary), atomic Rust/C++ staged saves, interprocess writer lock and corrupted-primary recovery regression |
| F09 Uninstall | MSI no longer forcibly terminates running IME, allows same-version major replacement, or unregisters TSF during an upgrade or invokes blanket shortcut cleanup under SYSTEM. Final uninstall uses a separate TSF COM-only command pointed at the installed sibling DLL, without rewriting user preferences; offline authoring tests detect forced termination, same-version replacement and missing upgrade guards. A native WiX 5.0.2 x64 MSI was compiled and all ICE validation checks returned zero errors/warnings; CI compiles/decompiles MSI x64 without installing it. Real MSI/VM and multiuser registry cleanup still pending |
| F10 Hook callback | Async worker prewarmed; bounded nonblocking 8-job queue, 750 ms pre-readiness expiry and exact foreground identity; new two-phase handoff requires worker readiness within 5 ms AND an explicit hook commit before suppressing original key. Queued-but-not-ready, expired, canceled, disconnected or wrong-target jobs fail open without later injection; 64 KiB text / 4096-backspace limits. Post-commit focus/SendInput failures and rapid input ordering cannot be guaranteed by this handshake, so mixed-app, 24h and latency p95/p99 E2E remain pending |
| F11 SendInput | Partial input stops without replaying accepted backspaces/code units, releases pressed injected keys; unit tests cover UTF-16, Backspace, Shift, repeated VK; independent host fault injection still pending |
| F12 TSF IPC | Rust writer clamps malformed enum/flag/version fields before publishing; C++ reader rejects malformed yet sequence-consistent snapshots and retains last valid config. Independent native smoke tests cover every field; AppContainer ACL and multi-process long-run tests pending |
| Q01 Release | Official publisher routes to one verified gate; default is offline. Preflight verifies PE/ZIP architecture, genuine read-only Windows Installer MSI database identity, 12 payload hashes, updater URLs, SHA256SUMS and 6 metadata assets. A shared Windows Installer COM verifier now reads actual MSI ProductVersion, ProductCode, UpgradeCode, architecture, scope and language in both manifest generation and publishing; minimal synthetic MSI databases and 20 positive/negative offline scenarios run in CI. Canonical-dist packaging regression prevents stale MSI/ZIP inputs. Explicit publish still requires clean source, matching origin/main and tag, plus remote asset digests; signing/MSI VM/public release remain NO-GO pending independent approval |
| Q02 Independent E2E | Matrix below must be run by QA and recorded for target release |

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
| E03 | TSF composition Return, Shift+Return, Tab, Esc; exactly one host action per physical key |
| E04 | Telex/VNI/VIQR plus supplementary-plane Unicode and mixed English; exact Unicode roundtrip |
| E05 | Short and very large macros plus Safe mode: ensure 5 ms readiness timeout passes physical key exactly once without late queued replacement; force congested worker and focus changes before/after commit; measure hook callback p50/p95/p99/maximum, CPU/memory and recovery after 24h |
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
