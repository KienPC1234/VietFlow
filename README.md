# VietFlow

Lightweight, memory-safe native Vietnamese input method for Windows.
Rust core engine and hook daemon with an isolated Dear ImGui + DirectX 11 settings UI.

Repository: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

## Key Features

- **Hybrid Input**: Native Text Services Framework (TSF TIP) for zero-backspace typing in modern apps, plus a 0ms low-level hook with hardware key repeat (WASD) for games.
- **Game Safe**: Automatic global hotkey locking (Ctrl+Shift / Alt+Z) and smart UWP/Xbox container detection (Roblox, CS2, Valorant).
- **Per-App Overrides**: Custom typing method, charset, key-injection mode, and macro access per executable.
- **Resource Footprint**: Always-on daemon uses ~2.9 MB private bytes and 0% idle CPU. Settings UI runs out-of-process and frees 100% RAM/GPU when closed.
- **Smart English Habit**: Phonotactic restoration prevents mangling common technical words (`test`, `reset`, `setup`).
- **Native Multi-Arch**: Static MSVC builds for x64, x86, and ARM64 (no VC++ Redistributable required). High-DPI vector UI (100%–200%).

## Feature Comparison

| Feature / Capability | VietFlow | EVKey | OpenKey | UniKey 4.3 |
| :--- | :--- | :--- | :--- | :--- |
| **Language & Architecture** | Rust std (Memory-safe) | C++ Hook + C#/.NET UI | C++ Win32 | C++ MFC / Win32 |
| **Input Delivery Modes** | Hybrid: TSF TIP + Low-Level Hook | Low-Level Hook only | Low-Level Hook only | Low-Level Hook only |
| **Daemon Memory Footprint** | ~2.9 MB Private Bytes | ~15–25 MB (.NET runtime) | ~5–8 MB | ~2–4 MB |
| **UI Isolation** | Out-of-process DirectX 11 + ImGui | In-process Form | Monolithic Win32 Dialog | Monolithic MFC Dialog |
| **Game Movement (WASD)** | 0ms Native Passthrough + Auto-Repeat | Heuristic passthrough | vDoNothing passthrough | Frequent key locks |
| **In-Game Hotkey Lock** | Automatic lock in excluded apps | Manual switch only | None | None |
| **Per-App Profile Overrides** | Yes (Engine, Charset, Macro, Mode) | Basic E/V switch list | Registry E/V toggle list | None |
| **Browser URL Bar Conflict** | Native in TSF / Minimal diffing | Temporary char hack | Narrow space hack (0x202F) | Freezes / suggestion collision |
| **Windows on ARM (ARM64)** | Native ARM64, x64, x86 | Emulated x86 | Emulated x86/x64 | Emulated x86 |
| **High-DPI Scaling** | Native vector rendering (100%–200%) | Fair (WinForms DPI issues) | Blurry GDI controls | Blurry bitmap icons |
| **Dynamic Macro Tokens** | `@date`, `@time`, `@clipboard`, CSV/TSV | Static text only | Static text only | Static text only |
| **License & Auditing** | Open Source (GPLv3, audited) | Closed Source | Open Source (GPLv3) | Closed Source (v4.3) |

## Layout

```
core/                  Vietnamese engine (Rust std, C ABI in ffi/vietflow_core.h)
platforms/windows/     Win32 hook daemon, tray host, config watcher, updater
platforms/windows_tsf/ Text Services Framework (TSF) TIP COM server
ui_imgui/              Dear ImGui + DirectX 11 settings UI
assets/                Application and tray icons
scripts/               Build, packaging, and release scripts
```

## Build

Requirements: Rust 1.75+, CMake 3.20+, and Visual Studio Desktop C++ Build Tools.

```powershell
# Clone with submodules
git clone --recurse-submodules https://github.com/KienPC1234/VietFlow.git

# Test workspace
cargo test

# Build release (x64, static CRT -> dist\VietFlow.exe)
.\scripts\build.ps1

# Build all architectures with Velopack installers
.\scripts\build.ps1 -Arch All -Velopack
```

## Donation

If VietFlow saves you time, you can support the project via VietQR (Techcombank `9978 5892 20` - HA TRI KIEN):

<p align="center">
  <img src="https://img.vietqr.io/image/970407-9978589220-compact2.png?amount=0&addInfo=Ung%20ho%20VietFlow&accountName=HA%20TRI%20KIEN" alt="VietQR code for supporting VietFlow" width="220" />
</p>

## License

GPLv3 License. Core Vietnamese input algorithm is derived from UniKey (Pham Kim Long).
