# VietFlow Engineering Rules & Architecture Standards

Core rules for developing, maintaining, and shipping VietFlow across Windows, Linux and macOS.

---

## I. Architecture & Modular Boundaries

1. **`core/` (`vietflow-core`, Platform-Agnostic Engine)**:
   - Must remain **100% platform-neutral** with zero OS dependencies (`std`-only).
   - Absolute prohibition against Win32, X11, Wayland, or GUI types in the core.
   - The hot path (`process_key`) must avoid heap allocation for normal keystrokes
     (outputs are stored inline in `KeyText`).
   - Must export a stable C ABI (`ffi/vietflow_core.h`) for native integration
     (Qt UI, Linux Fcitx5 addon, macOS InputMethodKit).
   - Any modification to tone placement or charset mapping must be accompanied by
     comprehensive tests in `vietflow-core`.

2. **`platforms/windows/` (`vietflow-windows`, Native Windows Host)**:
   - Direct Win32 API integration via `windows-sys`. No heavy runtime dependencies.
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` hook with `SendInput` injection.
   - **Do not add TSF / IME / TIP components.** VietFlow deliberately uses low-level
     keyboard hooks to maintain anti-cheat and full-screen game compatibility.
   - **Qt must never run inside the keyboard-hook process.** The UI is a separate
     executable launched on demand.
   - Game Mode: process inspection via `GetWindowThreadProcessId` +
     `QueryFullProcessImageNameW`, bypassing interception for excluded executables.
   - Configuration changes are picked up at runtime by watching the JSON file
     (1-second timer), so saving the UI does not require a daemon restart.

3. **`ui/` (Qt 6 Configuration Frontend)**:
   - Qt Quick/QML for the interface and C++ for the backend
     (`SettingsBackend`, `EngineBridge`).
   - The primary settings UI for Windows/Linux/macOS. It is a configuration client
     only: keystrokes are never handled here.
   - Talks to the Rust core exclusively through the C ABI in `ffi/vietflow_core.h`.
   - Reads and writes the same `vietflow_config.json` schema as the native host.
   - Uses one shared codebase and a deterministic dark theme; no platform-native
     widget styling.

4. **Distribution**:
   - `VietFlow.exe`: a single portable file containing the native daemon, tray host
     and a compressed Qt UI runtime.
   - On first UI launch the runtime is extracted to
     `%LOCALAPPDATA%\VietFlow\ui-<version>-<size>` and reused afterwards.
   - Qt still runs as a separate process; it is never loaded in the hook process.
   - A development layout (`vietflow-ui.exe` + Qt DLLs next to the host) is also
     supported and preferred when present.
   - Users never install Qt, Rust or a C++ toolchain.

## II. Repository Layout

```
vietflow/
├── core/                  Rust engine (platform-neutral)
├── platforms/windows/     Win32 host, hook, tray, config watcher
├── ui/                    Qt 6 / QML frontend + C++ backend
├── ffi/                   Public C ABI header
├── assets/                Canonical brand assets
├── docs/                  Architecture, roadmap and active work claims
├── scripts/build.ps1      Release packaging
└── Cargo.toml             Rust workspace
```

## III. Code Quality & Standards

1. **Safety & Concurrency**:
   - Limit `unsafe` strictly to FFI/Win32 boundaries.
   - Guard shared state across threads with `Mutex` or atomics.

2. **Zero Incomplete Code**:
   - Prohibit `todo!()`, `unimplemented!()`, or placeholder stubs in production.
   - Register technical debt in `docs/ROADMAP.md`.

3. **Sanitization**:
   - Zero compiler warnings for Rust and Qt/C++ builds.
   - Remove unused imports, debug macros and scaffolding before commit.

4. **UI Copy**:
   - Keep user-facing text short and non-technical. Do not surface implementation
     details (hooks, ABI, packaging internals) in the app.

## IV. Versioning & Commit Conventions

- Current milestone: `0.1.0-alpha`.
- Conventional Commits: `feat:`, `fix:`, `refactor:`, `perf:`, `docs:`, `test:`, `chore:`.
- Release validation:
  1. `cargo test --workspace` passes 100%.
  2. `cargo build --workspace --release` compiles with zero warnings.
  3. `cmake --build ui/build --config Release` compiles with zero warnings.

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

