# VietFlow Engineering Rules & Architecture Standards

Core rules for developing, maintaining, and shipping VietFlow across Windows and Linux.

---

## I. Architecture & Modular Boundaries

1. **`vietflow-core` (Platform-Agnostic Engine)**:
   - Must remain **100% platform-neutral** with zero OS dependencies (`std`-only).
   - Absolute prohibition against Win32, X11, Wayland, or GUI types in the core.
   - Hot path (`process_key`) must remain zero-allocation on steady-state typing.
   - Must export a stable C-ABI (`c_abi.rs`) for native integration (e.g., Linux Fcitx5 C++ addon).
   - Any modification to tone placement or charset mapping must be accompanied by comprehensive tests in `vietflow-core`.

2. **`vietflow-windows` (Native Windows Host)**:
   - Direct Win32 API integration via `windows-sys`. No heavy runtime or external DLL dependencies.
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` low-level keyboard hook with `SendInput` atomic injection.
   - **Do not add TSF / IME / TIP components.** VietFlow deliberately uses low-level keyboard hooks to maintain 100% anti-cheat and full-screen game compatibility.
   - Game Mode: Background process inspection via `GetWindowThreadProcessId` + `QueryFullProcessImageNameW`, bypassing hook interception for blacklisted game executables.

3. **`vietflow-gui` (Unified GUI Toolkit)**:
   - Powered by **Slint** in pure Rust. Zero external C++ or Qt SDK installation required.
   - Declarative QML-style UI with automatic layout, native responsiveness, and dark theme support.
   - Communicates with the daemon via `%APPDATA%\VietFlow\vietflow_config.json`.
   - Embeds all brand assets (PNG logos, vector icons) directly into the compiled artifact.

4. **Single Binary Distribution (`VietFlow.exe`)**:
   - The daemon and configuration GUI compile into a **single, unified executable** (`VietFlow.exe`).
   - Normal launch executes the background daemon and system tray runner (< 2 MB RAM).
   - Parameterized execution (`--gui`, `--convert`) launches the Slint configuration panel on demand from the same binary.

---

## II. Code Quality & Standards

1. **Safety & Concurrency**:
   - Limit `unsafe` strictly to necessary FFI/Win32 hook dispatch boundaries.
   - Guard shared state across thread boundaries using `std::sync::Mutex` or atomic primitives.

2. **Zero Incomplete Code**:
   - Prohibit `todo!()`, `unimplemented!()`, or dummy placeholder stubs in production branches.
   - Any untracked technical debt must be registered in `TODO.md`.

3. **Sanitization**:
   - Zero compiler warnings (`#[warn(unused)]` enforced).
   - Remove unused imports, debug macros (`println!`, `dbg!`), and transient scaffolding before commit.

---

## III. Versioning & Commit Conventions

- Current milestone: `1.0.4`.
- Commit messages follow Conventional Commits: `feat:`, `fix:`, `refactor:`, `perf:`, `docs:`, `test:`, `chore:`.
- Release validation checklist:
  1. `cargo test --workspace` passes 100%.
  2. `cargo build --workspace --release` compiles with zero warnings.
