# VietFlow Project Roadmap & Task Tracker

All active and completed work items across the VietFlow codebase.

The implementation plan for the UniKey / EVKey / OpenKey gaps is in
[`PLAN.md`](PLAN.md).

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
  - [x] Per-app profiles, injection modes and macro-only mode for excluded apps.
  - [x] Optional hotkey lock in excluded apps, with tray/manual override.
  - [x] Auto-start via the Windows `Run` registry key.
  - [x] Live configuration reload (file watcher timer, no daemon restart).
  - [x] x64, x86 and ARM64 build matrix with GitHub Actions.

- [x] **Dear ImGui UI (`ui_imgui/`)**:
  - [x] Native Dear ImGui + DirectX 11 frontend in C++20.
  - [x] Seven pages: Overview, Typing, Macros, Exclusions, Converter, Advanced, About.
  - [x] Editable English keyword list, foreign initials, per-app profiles and hotkey recorder.
  - [x] Rust Core accessed through the C ABI; the UI runs in a separate process from the hook.
  - [x] Compact dark theme, short user-facing copy, 980×660 window, per-monitor DPI.
  - [x] Clipboard and direct-text charset conversion, including raw TCVN3/VNI bytes.
  - [x] Macro import/export and configurable auto-capitalize characters.

---

## II. Active & Upcoming Tasks

- [x] **Packaging**:
  - [x] Single-file `VietFlow.exe`: Rust host + statically linked ImGui/DX11 UI.
  - [x] No Qt runtime, no first-launch extraction and no VC++ Redistributable by default.
  - [x] `scripts/build.ps1` produces x64, x86 and ARM64 portable builds.
  - [x] Static CRT keeps the portable exe around 1.6 MB.
  - [ ] Signed installer / Microsoft Store package (optional).

- [x] **Updater**: on-demand GitHub Releases check via WinINet; opens the release
      page when a newer tag exists. Velopack runtime removed to keep the binary
      small (installer packaging via `vpk` still available).

- [x] **Elevated / Administrator Privileges**:
  - [x] Optional "Chạy lại với quyền quản trị" from the tray menu.
  - [ ] Optional elevated scheduled task so VietFlow can type into games or tools
        running as Administrator without UAC prompts on boot.

- [ ] **Linux Integration**:
  - [ ] Fcitx5 addon connecting to `libvietflow_core.so` via the C ABI.
  - [ ] Verify X11 and Wayland input forwarding.
  - [ ] Package a native Linux UI.

- [ ] **macOS Integration**:
  - [ ] InputMethodKit adapter (Swift/Obj-C) using the Rust core.
  - [ ] Code signing and notarization for a native macOS UI bundle.

- [ ] **Benchmark & Performance**:
  - [ ] Continuous benchmark suite comparing keystroke latency against UniKey and EVKey.
