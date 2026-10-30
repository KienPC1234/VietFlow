# VietFlow Feature and Release Plan

This plan turns the UniKey / EVKey / OpenKey gap research into shippable work.
It is deliberately narrow: keep the low-level keyboard hook, do not add TSF/IME,
and prefer features that can be implemented and tested without a cloud service.

## Goals

1. Make the Windows host reliable in games, IDEs, terminals and remote sessions.
2. Give users explicit per-application control instead of one global mode.
3. Match the table-stakes features of OpenKey (converter, macro import/export,
   per-app charset) while keeping VietFlow's stronger exclusion and hotkey lock.
4. Ship reproducible x64, x86 and ARM64 builds from GitHub Actions.

## Non-goals

- TSF / IME / TIP integration.
- Cloud prediction, telemetry or silent background updates.
- Chasing individual anti-cheat engines. The hook is best-effort; exclusion and
  hotkey lock are the safe fallback.
- A macOS or Linux UI in this milestone. The core stays platform-neutral.

## Implementation status (2026-10-30)

- [x] x64 and x86 portable builds validated locally.
- [x] Per-app profiles, injection modes, macro-only mode, real converter,
      macro import/export, configurable auto-capitalize characters, hotkey-lock
      checkbox and admin relaunch are implemented.
- [x] Windows host tests and core tests pass.
- [x] ARM64 executable validated on the native CI runner (`Arm64`, PE32+).
- [ ] Installer packaging and code signing.

## Phase 0: Build and release foundation

- [x] `scripts/build.ps1` supports `x64`, `x86`, `arm64` and `All`.
- [x] CMake uses the matching Visual Studio architecture (`x64`, `Win32`, `ARM64`).
- [x] Rust uses `x86_64-pc-windows-msvc`, `i686-pc-windows-msvc` and
      `aarch64-pc-windows-msvc`.
- [x] `build.rs` resolves `build/`, `build-x86/` and `build-arm64/`.
- [x] Artifact names: `VietFlow.exe`, `VietFlow-x86.exe`, `VietFlow-arm64.exe`.
- [x] `.github/workflows/build.yml` runs tests on x64 and builds all three
      architectures, uploading portable executables.
- [x] ARM64 builds on the native `windows-11-vs2026-arm` runner; x64/x86 build on
      `windows-latest`.

Acceptance: x64 and x86 were validated locally. ARM64 was built by the
`windows-11-vs2026-arm` runner; the artifact was downloaded and its PE header
reports `Arm64` / PE32+.

## Phase 1: Per-app profiles and injection modes

### Config model

Add `injection_mode` globally and an `app_profiles` map keyed by executable name.
Each profile can override:

- input method;
- charset;
- injection mode;
- whether macros are allowed.

Use `-1` / `null` to mean "inherit global". Matching is case-insensitive and
accepts names with or without `.exe`, reusing the exclusion matching rules.

### Runtime

- Resolve the profile whenever the foreground window changes.
- Apply method and charset to the live engine without restarting the daemon.
- Resolve the injection mode before every output batch.
- Reset the engine and macro buffer on profile change.

### Injection modes

- `Auto`: Unicode `SendInput` (current behavior).
- `Unicode`: force Unicode `SendInput`.
- `ScanCode`: VK/scancode events for ASCII, Unicode fallback for characters that
  cannot be mapped.
- `Clipboard`: set clipboard, send Ctrl+V, restore the previous clipboard text.
- `Safe`: paced Unicode output with full backspaces, for apps that drop fast
  batches or mis-handle display shadowing.

Each mode keeps the existing atomic backspace + text behavior where possible.

## Phase 2: Macro correctness

- [x] Fix `allow_macro_when_disabled` so it means "macros only", not "turn
      Vietnamese typing back on".
- [x] Add a macro-only buffer used while Vietnamese is disabled or the app is
      excluded. Printable keys pass through; when a word break arrives the hook
      checks the trigger, backspaces the typed trigger and sends the replacement.
- [x] Keep global shortcuts locked in excluded apps; macros remain available
      only when the user explicitly allows them. The lock is configurable.
- [x] Fix tray manual override in an excluded app: the first tray click must
      enable typing through the override, the second must disable it.

## Phase 3: Converter and macro management

- [x] Wire the converter page to `vf_convert_charset` and `vf_strip_accents`.
- [x] Support Unicode, Unicode Composite, TCVN3 and VNI Windows for the target.
- [x] Add case conversion and accent stripping without corrupting Vietnamese.
- [x] Add macro import/export (TSV, CSV, `trigger=value`, `trigger||value`).
- [x] Make conversion status messages reflect success or failure.

## Phase 4: Low-risk quality features

- [x] Configurable auto-capitalize character set (default `.!?` and newlines).
- [x] Optional "restart as administrator" from the tray.
- [x] Macro editor supports large lists without growing the window.
- [x] Keep static CRT defaults and per-monitor DPI behavior.

## Next actions

1. [x] **Run the ARM64 workflow** on the native `windows-11-vs2026-arm` runner.
   x64, x86 and ARM64 jobs are green; the ARM64 artifact was verified as PE32+ Arm64.
2. **Manual runtime matrix** on a real desktop. Use
   [`VALIDATION.md`](VALIDATION.md) as the checklist:
   - hotkey lock checkbox on/off; V/E, settings and converter shortcuts;
   - tray override first/second click in an excluded app;
   - per-app profile for an IDE, browser and game;
   - Unicode, scan-code, clipboard and safe injection modes;
   - macro-only in English mode and in an excluded app;
   - TCVN3/VNI clipboard conversion in both directions;
   - macro import/export with Unicode replacements;
   - admin relaunch with UAC accepted and cancelled.
3. **Fix validation findings** before any release. Likely risk areas: clipboard
   mode blocking the hook, uppercase TCVN3 roundtrip, and apps that only expose
   Unicode clipboard data.
4. **Sync stale docs**: `ROADMAP.md` still describes the old Qt UI and single-file
   Qt runtime; `AGENTS.md` and `docs/BUILD_CMAKE.md` also mention Qt. Update them
   to the ImGui/DX11 layout before another agent works from them.
5. **Coordinate the working tree**: there is concurrent QR/donation work under
   `ui_imgui/third_party/qrcodegen/` and `ui_imgui/CMakeLists.txt`. Do not stage
   or commit the whole tree blindly.
6. **Release only after validation**: bump `VERSION`, update `CHANGELOG.md`, run
   `.\scripts\build.ps1 -Arch All -Velopack`, then use `docs/RELEASING.md`.
   Portable builds are usable now; installer/code signing is optional.

## Phase 5: Deferred

- Vietnamese word prediction: possible local-only experiment, not a release
  blocker.
- Light/system theme and English UI.
- macOS InputMethodKit and Linux Fcitx5 hosts.
- Code signing and notarization.

## Test matrix

| Area | Tests |
| --- | --- |
| Core tone rules, English restore, macro | `cargo test -p vietflow-core` |
| C ABI converter and strip accents | `cargo test -p vietflow-core` |
| App matching / profile resolution | `cargo test -p vietflow-windows` (x64) |
| Hotkey parser and override semantics | `cargo test -p vietflow-windows` (x64) |
| Build | x64, x86 and ARM64 Release with zero warnings |

## Release policy

Do not publish a GitHub release for these changes alone. The next release is a
minor milestone after the plan is implemented and validated; use the existing
`docs/RELEASING.md` checklist.
