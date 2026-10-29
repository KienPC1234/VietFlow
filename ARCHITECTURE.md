# VietFlow Architecture & Technical Specification

## I. Architectural Principles

VietFlow is architected with three primary layers to achieve minimal latency, zero memory leaks, and native platform integration:

1. **VietFlow Core (`vietflow-core`)**:
   - Written in 100% safe and robust Rust.
   - Zero OS dependencies (`std`-only).
   - State-machine driven input processor supporting Telex, Simple Telex, VNI, and VIQR.
   - Modern tone placement rule-set (dấu mới `oà, uỳ, oè` vs dấu cũ `òa, ùy, òe`).
   - Macro expansion supporting static words and dynamic template variables (`@date`, `@time`, `@datetime`, `@clipboard`, `@tab`, `@newline`).
   - Cross-charset conversion engine supporting Unicode NFC, Unicode NFD (Composite), TCVN3 (ABC), and VNI Windows.
   - Zero-overhead C-ABI interface for seamless cross-platform consumption.

2. **Windows Native Host (`vietflow-windows`)**:
   - Direct Win32 API integration via `windows-sys`.
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` low-level keyboard hook.
   - Dynamic tray icon rendering with real-time status indication (`[V]` / `[E]`).
   - Game Mode: Background process inspection via `GetWindowThreadProcessId` and `QueryFullProcessImageNameW`, automatically bypassing keyboard interception in games.
   - Startup integration via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

3. **Unified Configuration GUI (`vietflow-gui`)**:
   - Declarative, modern reactive user interface built on Slint.
   - Direct synchronization with `%APPDATA%\VietFlow\vietflow_config.json`.
   - Integrated text & clipboard conversion utility (`Ctrl+Shift+F9`).
   - Embedded SVG and high-resolution PNG brand assets.

4. **Linux Host Integration (Fcitx5 Addon)**:
   - Uses `vietflow_core` dynamic library or static link via the standard C-ABI (`vf_engine_*`).
   - Shares the same state machine and spelling rules as Windows.

## II. Data Flow

```
Raw Physical Keypress
         │
         ▼
[WH_KEYBOARD_LL Hook / Fcitx5 Handler]
         │
         ├── Game Mode active? ──Yes──► Pass through (raw key)
         │
         └── No
              │
              ▼
    [vietflow-core State Machine]
              │
         ┌────┴──────────────────────────┐
         ▼                               ▼
    Consumes Key?                   Ignored / Flush
         │                               │
    Generate Backspaces + Chars          Pass through
         │
         ▼
    [SendInput Key Injection]
```

## III. Build Instructions

Prerequisites:
- Rust 1.80+ (stable toolchain)
- Cargo

Commands:
```powershell
# Run all tests
cargo test --workspace

# Produce optimized release binaries
cargo build --workspace --release
```
