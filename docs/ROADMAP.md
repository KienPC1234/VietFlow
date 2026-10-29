# VietFlow Project Roadmap & Task Tracker

All active and completed work items across the VietFlow codebase.

---

## I. Completed Milestones (Rust Architecture)

- [x] **100% Rust Core Rewrite (`vietflow-core`)**:
  - [x] High-performance Telex, Simple Telex, VNI, and VIQR input state machine.
  - [x] Modern tone placement rules (dấu mới `oà, uỳ, oè` vs dấu cũ `òa, ùy, òe`).
  - [x] Auto-restore key sequence on English words.
  - [x] Smart consonant shortcuts (`f` -> `ph`, `j` -> `gi`, `w` -> `qu`).
  - [x] Shift lag automatic fix (sửa dính Shift chữ thứ hai).
  - [x] Smart macro engine with dynamic variables (`@date`, `@time`, `@datetime`, `@clipboard`, `@tab`, `@newline`).
  - [x] Cross-charset converter (Unicode dựng sẵn, Tổ hợp, TCVN3 ABC, VNI Windows).
  - [x] Stable C-ABI exports (`vf_engine_*`) for external consumers.
  - [x] 100% test suite passing with UniKey algorithmic parity.

- [x] **Native Windows Host (`vietflow-windows`)**:
  - [x] `WH_KEYBOARD_LL` low-level keyboard hook with `SendInput` atomic injection.
  - [x] Thread-safe modifier key state tracking (`Ctrl`, `Shift`, `Alt`, `CapsLock`).
  - [x] Global mode switching hotkey (`Ctrl+Shift`, `Alt+Z`, etc.) with beep audio feedback.
  - [x] Windows system tray icon with live `[V]` / `[E]` visual status badge.
  - [x] Context menu for instant method, charset, macro, and startup toggling.
  - [x] Automated Game Mode: Excluded process detection via Win32 process query.
  - [x] System auto-start integration via Windows Registry `Run` key.

- [x] **Unified GUI (`vietflow-gui`)**:
  - [x] Modern Slint declarative interface (0 C++ / 0 Qt SDK dependency).
  - [x] 5 complete tabs: Tổng quan, Bộ gõ & Chính tả, Gõ tắt & Biến số, Game & Loại trừ, Công cụ & Giới thiệu.
  - [x] Text & Clipboard converter utility with hotkey trigger (`Ctrl+Shift+F9`).
  - [x] Fully integrated dark theme matching Windows 11 Fluent style.
  - [x] Embedded brand assets (`app-logo.png`, `VietFlow.ico`).

---

## II. Active & Upcoming Tasks

- [ ] **Single Binary Packaging**:
  - [ ] Consolidate daemon host and Slint GUI into a single unified `VietFlow.exe` executable.
  - [ ] Support `--gui` and `--convert` command-line flags from the same binary.
  - [ ] Automated release export script copying production binaries to `dist/`.

- [ ] **Elevated / Administrator Privileges**:
  - [ ] Option to create an elevated scheduled task (`TaskScheduler`) allowing VietFlow to type into games or developer tools running as Administrator without UAC prompts on boot.

- [ ] **Linux Integration**:
  - [ ] Build and package the Fcitx5 C++ addon connecting to `libvietflow_core.so` via C-ABI.
  - [ ] Verify Linux X11 and Wayland input forwarding.

- [ ] **Benchmark & Performance**:
  - [ ] Continuous benchmarking suite comparing keystroke latency against UniKey and EVKey.
