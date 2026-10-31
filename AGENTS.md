@docs/ACTIVE_WORK.md

# VietFlow Engineering Rules & Architecture Standards

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
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` hook with `SendInput` injection.
   - **Do not add TSF / IME / TIP components.** VietFlow deliberately uses low-level
     keyboard hooks to maintain game compatibility.
   - **The ImGui UI must never run inside the keyboard-hook process.** The daemon
     spawns `VietFlow.exe --settings` or `VietFlow.exe --convert`; that child process
     opens the UI and returns without installing a hook.
   - Game Mode: process inspection via `GetWindowThreadProcessId` +
     `QueryFullProcessImageNameW`, bypassing interception for excluded executables.
   - Per-app profiles may override method, charset, injection mode and macro
     permission. Foreground changes must resolve the profile without a restart.
   - Configuration changes are picked up at runtime by watching the JSON file
     (250 ms timer), so saving the UI does not require a daemon restart.
   - **Experimental exception**: `experiments/tsf_tip` may prototype a TSF text
     service to evaluate replacing clipboard injection. It must stay isolated,
     must not be linked into `VietFlow.exe`, and must not ship until validated.
     Production remains low-level-hook based; TSF/IME/TIP is allowed only under
     `experiments/` for evaluation.

3. **`ui_imgui/` (Dear ImGui + DirectX 11 settings frontend)**:
   - Native C++20 UI linked into `VietFlow.exe` as a static library.
   - It is a configuration client only: keystrokes are never handled here.
   - Talks to the Rust core through the C ABI in `ffi/vietflow_core.h`.
   - Reads and writes the same `vietflow_config.json` schema as the native host.
   - Uses one shared codebase and a deterministic dark theme; no platform-native
     widget styling.
   - Build with CMake. `ui_imgui/third_party/` contains the `imgui` and
     `nlohmann_json` submodules.

4. **Distribution**:
   - `VietFlow.exe`: one portable file containing the native daemon, tray host and
     the statically linked ImGui/DX11 UI. No Qt runtime, extraction step or VC++
     Redistributable is required by default.
   - The UI is still a separate process (`--settings` / `--convert`); it is never
     loaded into the hook process.
   - `scripts/build.ps1` produces x64, x86 and ARM64 binaries. Default CRT is static
     (`/MT` plus Rust `+crt-static`).
   - Users never install Rust, CMake, Visual Studio or a C++ toolchain.

## II. Repository Layout

```
vietflow/
├── core/                  Rust engine (platform-neutral)
├── platforms/windows/     Win32 host, hook, tray, config watcher, updater
├── ui_imgui/              Dear ImGui + DirectX 11 UI (C++20)
├── ui_imgui/third_party/  Git submodules: imgui, nlohmann_json
├── ffi/                   Public C ABI header
├── assets/                Canonical brand assets
├── docs/                  Architecture, roadmap, plan and release checklist
├── scripts/build.ps1      Release packaging for x64/x86/ARM64
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
