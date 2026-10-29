# VietFlow Architecture & Technical Specification

## I. Architectural Principles

VietFlow is organized in clear layers to achieve low latency, predictable behavior and native platform integration:

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

3. **Qt 6 Configuration UI (`ui/`)**:
   - Qt Quick/QML interface with a C++ backend (`SettingsBackend`, `EngineBridge`).
   - Separate executable (`vietflow-ui.exe`). Qt is never loaded in the hook process.
   - Uses the C ABI in `ffi/vietflow_core.h` for charset conversion and other
     core services; configuration is read/written as JSON.
   - Seven pages: Overview, Typing & Spelling, Macros, Game Mode, Converter,
     Advanced, About.

4. **Linux Host Integration (Fcitx5 Addon)**:
   - Uses the `vietflow_core` dynamic library via the standard C ABI (`vf_engine_*`).
   - Shares the same state machine and spelling rules as Windows.

5. **Shared Configuration & Live Reload**:
   - `%APPDATA%\VietFlow\vietflow_config.json` (Windows) is the single source of truth.
   - The Win32 host watches the file with a 1-second timer and reapplies settings to
     the running engine, so saving in the Qt UI takes effect without a restart.

6. **Single-File Packaging**:
   - The release script embeds a compressed Qt UI runtime inside `VietFlow.exe`.
   - On first UI launch the runtime is extracted to
     `%LOCALAPPDATA%\VietFlow\ui-<version>-<size>`; later launches reuse it.
   - The UI remains a separate process, so Qt is not loaded by the keyboard hook.
   - A separate-folder layout is available for development (`-Separate`).

## II. Repository Layout

```
vietflow/
├── core/                  Rust engine (platform-neutral)
├── platforms/windows/     Win32 hook, tray, Game Mode, config watcher
├── ui/                    Qt 6 / QML frontend + C++ backend
├── ffi/                   Public C ABI header
├── assets/                Canonical brand assets
├── docs/                  Architecture and roadmap
└── scripts/build.ps1      Release packaging
```

## III. Data Flow

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

## IV. Build Instructions

Prerequisites:
- Rust 1.80+ (stable toolchain)
- Qt 6.5+ with Qt Quick Controls 2 (only for the UI)
- CMake 3.21+

Commands:
```powershell
# Rust engine + Windows host
cargo test --workspace
cargo build --workspace --release

# Qt 6 configuration UI
cmake -S ui -B ui/build -DCMAKE_PREFIX_PATH="C:\Qt\6.8.3\msvc2022_64"
cmake --build ui/build --config Release

# Single-file release (host + embedded Qt runtime) -> dist/VietFlow.exe
pwsh scripts/build.ps1
```
