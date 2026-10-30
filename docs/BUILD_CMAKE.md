# VietFlow Build & Packaging Guide

VietFlow uses a Rust workspace plus a C++20 Dear ImGui/DirectX 11 UI. The UI is
built as a static library and linked into the single `VietFlow.exe`; it is not a
Qt application and there is no runtime extraction step.

---

## I. Requirements

| Component | Minimum | Notes |
| :--- | :--- | :--- |
| **Rust** | 1.75+ (MSVC) | `x86_64-pc-windows-msvc`; add `i686-pc-windows-msvc` and `aarch64-pc-windows-msvc` for the other targets |
| **CMake** | 3.20+ | Visual Studio generator |
| **Visual Studio** | 2022+ with Desktop C++ | ARM64 builds need the ARM64 MSVC component |
| **Git** | Any recent | Submodules: `imgui`, `nlohmann_json` |
| **Velopack CLI** | Optional | `dotnet tool install -g vpk`; only for installers |

Fetch submodules after cloning:

```powershell
git submodule update --init --recursive
```

---

## II. Standard Build

```powershell
# One architecture
.\scripts\build.ps1 -Arch x64
.\scripts\build.ps1 -Arch x86
.\scripts\build.ps1 -Arch arm64

# All three
.\scripts\build.ps1 -Arch All

# With Velopack installers
.\scripts\build.ps1 -Arch All -Velopack

# Dynamic CRT (needs VC++ 2015-2022 Redistributable, smaller exe)
.\scripts\build.ps1 -DynamicCrt

# Skip tests when the target cannot run on this host (ARM64 cross-build)
.\scripts\build.ps1 -Arch arm64 -SkipTests
```

The default is static CRT: C++ `/MT` and Rust `+crt-static`. The portable exe does
not import `VCRUNTIME140` or `MSVCP140`.

Output:

- `dist\VietFlow.exe` / `dist\VietFlow-x64.exe` (x64)
- `dist\VietFlow-x86.exe` (x86)
- `dist\VietFlow-arm64.exe` (ARM64)
- `dist\latest.json`, `dist\version.txt`, `dist\CHANGELOG.md`
- `releases\VietFlow-win-<arch>-Setup.exe` with `-Velopack`

---

## III. Manual CMake Commands

The build script is the supported path. For a quick UI-only iteration:

```powershell
cmake -B ui_imgui/build -S ui_imgui -DCMAKE_BUILD_TYPE=Release -DVIETFLOW_STATIC_CRT=ON
cmake --build ui_imgui/build --config Release

# x86
cmake -B ui_imgui/build-x86 -S ui_imgui -A Win32 -DCMAKE_BUILD_TYPE=Release -DVIETFLOW_STATIC_CRT=ON
cmake --build ui_imgui/build-x86 --config Release

# ARM64
cmake -B ui_imgui/build-arm64 -S ui_imgui -A ARM64 -DCMAKE_BUILD_TYPE=Release -DVIETFLOW_STATIC_CRT=ON
cmake --build ui_imgui/build-arm64 --config Release
```

Then:

```powershell
cargo test --workspace
cargo build --release -p vietflow-windows
```

`platforms/windows/build.rs` resolves `ui_imgui/build/`, `build-x86/` or
`build-arm64/` from the Rust target architecture.

---

## IV. Tests and CI

```powershell
cargo test --workspace
```

The GitHub Actions workflow `.github/workflows/build.yml`:

1. runs workspace tests on x64 with the UI library built;
2. builds x64 and x86 on `windows-latest`;
3. builds ARM64 on `windows-11-vs2026-arm`;
4. uploads the portable executables as workflow artifacts.

ARM64 cannot be tested on an x64 host; the ARM64 job is the runtime validation
for that target.

---

## V. Architecture

1. **`core/` (`vietflow-core`)**:
   - Pure Rust `std`, no OS dependencies.
   - Stable C ABI in [`ffi/vietflow_core.h`](../ffi/vietflow_core.h).
2. **`ui_imgui/`**:
   - Dear ImGui + DirectX 11, C++20, statically linked into the host.
   - Configuration client only; it never handles keystrokes.
3. **`platforms/windows/`**:
   - Native `WH_KEYBOARD_LL` hook, tray host, config watcher, updater.
   - Spawns `VietFlow.exe --settings` / `--convert` for the UI, so the hook
     process never loads DX11/ImGui.
   - Future Linux/macOS hosts can reuse the core and C ABI without changing the
     engine.
