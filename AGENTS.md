@docs/ACTIVE_WORK.md

# VietFlow Engineering Rules & Architecture Standards

> [!CAUTION]
> ### CRITICAL COMMIT & RELEASE RULES (MANDATORY FOR ALL AGENTS)
> 1. **STRICTLY PROHIBIT MICRO-COMMITS**:
>    - Never commit after small, iterative edits or individual file fixes. Doing so bloats the git commit graph.
>    - All changes MUST be accumulated and squashed into **large, cohesive milestone commits** (`feat(release): ...`, `fix(core): ...`) only after the entire feature, refactor, or fix is 100% complete, tested, and verified across the workspace.
>    - When still in progress or iterating with the user, keep all changes in the working tree uncommitted until explicitly approved or a major milestone is reached.
>    - Commit worthiness is measured by **cohesive scope, not diff size**: a small but 100% complete, tested fix is committable, while a large diff is not committable if it is incomplete or untested.
>    - **Anti-padding**: strictly prohibit inflating a diff to look "big enough" (gratuitous reformatting, unrelated drive-by edits, generated-file or lockfile noise bundled without need). Keep every commit's diff limited to what the milestone requires.
> 2. **EXHAUSTIVE GITHUB RELEASE ASSETS (NO MISSING FILES)**:
>    - When the user requests a GitHub release (`scripts/publish-release.ps1`), the published asset list MUST be 100% complete.
>    - **Pure Native Packaging**: Native WiX v5 MSIs (`VietFlow-win-*.msi`), standalone portable executables (`VietFlow-*.exe`), and updater metadata (`latest.json`, `version.txt`, `CHANGELOG.md`). No Velopack wrappers, NuGet packages, or delta updates.
>    - **Portable vs Installer distinction**: The updater serves MSI downloads exclusively to installed MSI instances, while portable instances are directed directly to the GitHub release page for manual download.
> 3. **NATIVE WIX V5 PACKAGING STANDARD (ZERO VELOPACK)**:
>    - The official packaging system is the native WiX Toolset v5 project located at `installer/wix/`.
>    - Absolute prohibition against Velopack (`vpk`), NuGet delta packages, or wrapper executables masquerading as installers.
>    - All MSI packages must maintain 100% Add/Remove Programs (ARP) metadata integrity (`ARPPRODUCTICON`, `ARPCOMMENTS`, `ARPURLINFOABOUT`, `ARPHELPLINK`, `ARPURLUPDATEINFO`).
>    - All uninstallation routines must be atomic: clean all shortcuts (per-user and per-machine) and invoke `SHChangeNotify(SHCNE_ASSOCCHANGED, ...)` to immediately flush stale shell icon caches.

Core rules for developing, maintaining and shipping VietFlow on Windows. The
engine and C ABI remain portable so Linux and macOS hosts can be added later.

---

## I. Architecture & Modular Boundaries

1. **`core/` (`vietflow-core`, platform-agnostic engine)**:
   - Must remain **100% platform-neutral** with zero OS dependencies (`std`-only).
   - Absolute prohibition against Win32, X11, Wayland or GUI types in the core.
   - The hot path (`process_key`) must avoid heap allocation for normal keystrokes
     (outputs are stored inline in `KeyText`).
   - Must export a stable C ABI (`ffi/vietflow_core.h`) for native integration
     (the ImGui UI, a Linux Fcitx5 addon, a macOS InputMethodKit host).
   - Any modification to tone placement or charset mapping must be accompanied by
     comprehensive tests in `vietflow-core`.

2. **`platforms/windows/` (`vietflow-windows`, native Windows host)**:
   - Direct Win32 API integration via `windows-sys`. No heavy runtime dependencies.
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` hook with `SendInput` injection and 0ms passthrough.
   - Dual-mode architecture:
     - **Hook Mode (Default)**: Low-level keyboard hook for system-wide and gaming compatibility (Roblox, CS2, Valorant).
     - **TSF Mode**: Complements Hook mode by routing composition through `vietflow_tip.dll` for flicker-free typing without backspaces in Word, Excel, and Chromium.
   - **The ImGui UI must never run inside the keyboard-hook process.** The daemon
     spawns `VietFlow.exe --settings` or `VietFlow.exe --convert`; that child process
     opens the UI and returns without installing a hook.
   - Process inspection via `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`,
     supporting prefix matching and container resolution (`ApplicationFrameHost.exe`).
   - Per-app profiles may override method, charset, injection mode and macro permission.
   - Configuration changes are picked up at runtime by watching the JSON file (250 ms timer).

3. **`platforms/windows_tsf/` (`vietflow-tsf`, native TSF TIP COM server)**:
   - Native Text Services Framework Text Input Processor (`vietflow_tip.dll`).
   - Implements `ITfTextInputProcessor`, `ITfKeyEventSink`, `ITfCompositionSink`.
   - Direct inline text editing with Linux IME-inspired preedit buffers, eliminating backspace cursor flicker.
   - Registered and unregistered cleanly via `VietFlow.exe --register-tsf` and `VietFlow.exe --unregister-tsf`.

4. **`installer/wix/` (WiX Toolset v5 native MSI project)**:
   - Pure Windows Installer authoring (`VietFlow.wxs`, `build_msi.ps1`).
   - Standard per-machine installation into `Program Files\VietFlow`.
   - Embedded branding bitmaps (`assets/wix_banner.bmp`, `assets/wix_dialog.bmp`).
   - Process closing via `<util:CloseApplication>` to prevent locked file errors during setup.

5. **`ui_imgui/` (Dear ImGui + DirectX 11 settings frontend)**:
   - Native C++20 UI linked into `VietFlow.exe` as a static library.
   - It is a configuration client only: keystrokes are never handled here.
   - Talks to the Rust core through the C ABI in `ffi/vietflow_core.h`.
   - Reads and writes the same `vietflow_config.json` schema as the native host.
   - Uses one shared codebase and a deterministic dark theme; no platform-native widget styling.

6. **Distribution**:
   - `VietFlow.exe`: one portable file containing the native daemon, tray host and
     the statically linked ImGui/DX11 UI. No Qt runtime, extraction step or VC++
     Redistributable is required by default (`/MT` + `+crt-static`).
   - `VietFlow-win-<arch>.msi`: enterprise-ready native Windows Installer package.
   - `scripts/build.ps1 -Msi` builds release binaries and the native MSI for x64, x86 and ARM64.

## II. Repository Layout

```
vietflow/
├── core/                  Rust engine (platform-neutral)
├── platforms/windows/     Win32 host, hook, tray, config watcher, updater
├── platforms/windows_tsf/ Native TSF (Text Services Framework) TIP DLL
├── installer/wix/         WiX Toolset v5 native MSI project & builder
├── ui_imgui/              Dear ImGui + DirectX 11 UI (C++20)
├── ui_imgui/third_party/  Git submodules: imgui, nlohmann_json
├── ffi/                   Public C ABI header
├── assets/                Canonical brand assets (icons, bitmaps, sounds)
├── docs/                  Architecture, roadmap, plan and release checklist
├── scripts/build.ps1      Release packaging for x64/x86/ARM64 and native MSI
├── scripts/publish-release.ps1 GitHub release publisher
└── Cargo.toml             Rust workspace
```

## III. Code Quality & Standards

1. **Safety & Concurrency**:
   - Limit `unsafe` strictly to FFI/Win32 boundaries.
   - Guard shared state across threads with `Mutex` or atomics.
   - Do not block the low-level keyboard hook on clipboard or network work.

2. **Zero Incomplete Code**:
   - Prohibit `todo!()`, `unimplemented!()` or placeholder stubs in production.
   - Register technical debt in `docs/ROADMAP.md`.

3. **Sanitization**:
   - Zero compiler warnings for Rust and C++ builds.
   - Remove unused imports, debug macros and scaffolding before commit.

4. **UI Copy**:
   - Keep user-facing text short and non-technical. Do not surface implementation
     details (hooks, ABI, packaging internals) in the app.

## IV. Versioning & Commit Conventions

- Current milestone: `0.1.x`.
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `perf:`, `docs:`, `test:`,
  `chore:`.
- **Release policy**: publish a GitHub release only for a major/minor feature
  milestone, a breaking change or a critical security fix. Batch small bug fixes,
  copy changes and refactors into the next milestone; do not cut patch releases for
  them.
- Release validation:
  1. `cargo test --workspace` passes 100%.
  2. `cargo build --workspace --release` compiles with zero warnings.
  3. `cmake --build ui_imgui/build --config Release` compiles with zero warnings.
  4. `cmake --build ui_imgui/build-x86 --config Release` compiles with zero warnings.
  5. ARM64 is built by the `windows-11-vs2026-arm` CI runner; local builds need the
     ARM64 MSVC toolset.
  6. `.\installer\wix\build_msi.ps1 -Arch x64` compiles the native WiX v5 MSI package cleanly with zero errors.

## V. Multi-Agent File Coordination

1. **Single claim table**: `docs/ACTIVE_WORK.md` is the source of truth for which
   files are currently being edited. Read it before editing anything.
2. **Claim before edit**: every agent (main or subagent) must add a row with its
   task, claimed files/folders/globs, start time (UTC) and status before touching
   files. Claims must be as narrow as possible.
3. **No overlapping edits**: an agent must not edit a path claimed by another
   active row. The main agent serializes overlapping work or reassigns the claim.
4. **Subagent prompts**: the main agent must include the claim instruction in every
   subagent prompt and check the table before dispatching. Subagents may update
   only their own row.
5. **Release the claim**: mark the row `done` or remove it immediately when the
   task finishes. Do not hold claims while idle.
6. **Stale claims**: a claim with no progress for more than 2 hours may be cleared
   by the main agent after verifying that no agent/process still needs the files.
7. **Release hygiene**: remove completed rows before a release commit; the table
   should only show active work.
