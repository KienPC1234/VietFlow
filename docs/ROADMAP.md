# VietFlow Project Roadmap & Task Tracker

All active and completed work items across the VietFlow codebase.

---

## I. Completed Milestones

- [x] **100% Rust Core (`core/`)**:
  - [x] High-performance Telex, Simple Telex, VNI and VIQR state machine.
  - [x] Modern tone placement rules (dấu mới `oà, uỳ, oè` vs dấu cũ `òa, ùy, òe`).
  - [x] Auto-restore key sequence on English words.
  - [x] Smart consonant shortcuts (`f` → `ph`, `j` → `gi`, `w` → `qu`).
  - [x] Shift-lag auto fix (sửa dính Shift chữ thứ hai).
  - [x] Macro engine with dynamic variables (`@date`, `@time`, `@datetime`, `@clipboard`, `@tab`, `@newline`).
  - [x] Cross-charset converter (Unicode NFC, Unicode NFD, TCVN3, VNI Windows).
  - [x] Allocation-free steady-state key output (`KeyText`) and stack-based tone lookup.
  - [x] Stable C ABI (`ffi/vietflow_core.h`) with charset conversion helpers.
  - [x] Full unit test suite passing.

- [x] **Windows Host (`platforms/windows/`)**:
  - [x] `WH_KEYBOARD_LL` hook with atomic `SendInput` batches and display shadow diffing.
  - [x] Modifier tracking and global hotkeys.
  - [x] System tray with live `[V]` / `[E]` status.
  - [x] Context menu for method, charset, macro, spellcheck and startup.
  - [x] Game Mode process exclusion.
  - [x] Auto-start via the Windows `Run` registry key.
  - [x] Live configuration reload (file watcher timer, no daemon restart).

- [x] **Qt 6 Configuration UI (`ui/`)**:
  - [x] Qt Quick/QML interface with C++ backend (`SettingsBackend`, `EngineBridge`).
  - [x] Seven pages: Overview, Typing & Spelling, Macros, Game Mode, Converter, Advanced, About.
  - [x] Advanced page restored from C# parity: English/foreign keyword list, foreign initials, config folder shortcut.
  - [x] Rust Core accessed through the C ABI; no Qt in the hook process.
  - [x] Compact dark theme, short user-facing copy, 900×600 window.
  - [x] Clipboard and direct-text charset conversion.
  - [x] Removed the old Slint UI and the legacy Qt Widgets prototype.

---

## II. Active & Upcoming Tasks

- [x] **Packaging**:
  - [x] Single-file `VietFlow.exe`: Rust host + embedded Qt UI runtime.
  - [x] First-launch extraction to `%LOCALAPPDATA%` and reuse afterwards.
  - [x] `scripts/build.ps1` produces the single-file release (and `-Separate` for dev).
  - [x] Trimmed embedded runtime (unused Qt plugins/libraries): ~19 MB single file.
  - [ ] Signed installer / Microsoft Store package (optional).

- [x] **Updater**: on-demand GitHub Releases check via WinINet; opens the release
      page when a newer tag exists. Velopack runtime removed to keep the binary
      small (installer packaging via `vpk` still available).

- [ ] **Elevated / Administrator Privileges**:
  - [ ] Optional elevated scheduled task so VietFlow can type into games or tools
        running as Administrator without UAC prompts on boot.

- [ ] **Linux Integration**:
  - [ ] Fcitx5 addon connecting to `libvietflow_core.so` via the C ABI.
  - [ ] Verify X11 and Wayland input forwarding.
  - [ ] Package the Qt UI for Linux distributions.

- [ ] **macOS Integration**:
  - [ ] InputMethodKit adapter (Swift/Obj-C) using the Rust core.
  - [ ] Code signing and notarization for the Qt UI bundle.

- [ ] **Benchmark & Performance**:
  - [ ] Continuous benchmark suite comparing keystroke latency against UniKey and EVKey.
