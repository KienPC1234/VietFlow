# VietFlow Architecture & Technical Specification

## I. Architectural Principles

VietFlow is organized in clear layers to achieve low latency, predictable behavior, zero DLL dependencies, and native platform integration:

1. **VietFlow Core (`vietflow-core`)**:
   - Written in 100% safe and robust Rust.
   - Zero OS dependencies (`std`-only).
   - State-machine driven input processor supporting Telex, Simple Telex, Simple Telex 2, VNI, VIQR, and Telex+VNI hybrid.
   - Modern tone placement rule-set (dấu mới `oà, uỳ, oè` vs dấu cũ `òa, ùy, òe`).
   - Macro expansion supporting static words and dynamic template variables (`@date`, `@time`, `@datetime`, `@clipboard`, `@tab`, `@newline`).
   - Cross-charset conversion engine supporting Unicode NFC, Unicode NFD (Composite), TCVN3 (ABC), and VNI Windows.
   - Zero-overhead C-ABI interface for seamless cross-platform consumption.

2. **Windows Single-Binary Host & GUI (`vietflow-windows`)**:
   - Direct Win32 API integration via `windows-sys`.
   - Thread-safe, non-blocking `WH_KEYBOARD_LL` low-level keyboard hook daemon (~3 MB RAM).
   - Built-in immediate mode GUI powered by `eframe` / `egui` in pure Rust (~6 MB total standalone executable, 0 runtime DLLs, 0 file extraction).
   - CLI mode separation:
     - Default execution: Runs lightweight background hook and system tray daemon.
     - `--settings` or `--gui`: Spawns the native `egui` configuration dashboard as an independent UI process, freeing 100% of UI RAM upon window close.
     - `--convert`: Launches directly into the Text & Charset Converter tool.
   - Dynamic tray icon rendering with real-time status indication (`[V]` / `[E]`).
   - Game Mode: Background process inspection via `GetWindowThreadProcessId` and `QueryFullProcessImageNameW`, automatically bypassing keyboard interception in games and designated applications.
   - Startup integration via `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`.

3. **Linux Host Integration (Fcitx5 Addon)**:
   - Uses the `vietflow_core` dynamic library via the standard C ABI (`vf_engine_*`).
   - Shares the same state machine and spelling rules as Windows.

4. **Shared Configuration & 0ms Live Reload**:
   - `%APPDATA%\VietFlow\vietflow_config.json` (Windows) is the single source of truth.
   - IPC notification: The `egui` UI sends `WM_RELOAD_CONFIG` (`WM_USER + 104`) to `VietFlowDaemon` message window upon saving, achieving 0ms instant reload without restarting.
   - File modification watcher serves as a 250ms polling fallback.

5. **Single-File Standalone Architecture**:
   - Zero external DLL dependencies (MSVC runtime statically linked or system-provided).
   - Zero disk extraction at runtime (no temporary folder extraction).
   - Standalone binary output: `dist/VietFlow.exe` (~6 MB).

## II. Repository Layout

```
vietflow/
├── core/                  Rust engine (platform-neutral)
├── platforms/windows/     Unified single binary (Win32 hook daemon + egui GUI)
├── ffi/                   Public C ABI header
├── assets/                Canonical brand assets & icons
├── docs/                  Architecture and roadmap
└── scripts/build.ps1      Release packaging script
```

## III. Data Flow

```
Raw Physical Keypress
         │
         ▼
[WH_KEYBOARD_LL Hook / Fcitx5 Handler]
         │
         ├── Game Mode / Exclusion active? ──Yes──► Pass through (raw key)
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

## IV. Build Instructions

Prerequisites:
- Rust 1.80+ (stable toolchain)

Commands:
```powershell
# Run core test suite
cargo test --manifest-path vietflow/Cargo.toml

# Build single standalone binary
cargo build --release -p vietflow-windows

# Package release artifact to dist/VietFlow.exe
pwsh vietflow/scripts/build.ps1
```
