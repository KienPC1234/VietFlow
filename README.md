# VietFlow

A fast Vietnamese input method for Windows. The engine and keyboard hook are written in Rust; the settings UI is a separate Dear ImGui + DirectX 11 process.

Repository: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

## Features

- Native `WH_KEYBOARD_LL` hook. The UI runs in a separate process and is never loaded into the hook.
- Per-app exclusion list: switches to English `[E]` for games, IDEs and graphics tools, then restores `[V]` when focus leaves.
- High-DPI tray icons with optional custom `.ico` / `.png` files.
- English word detection (226 keywords) to avoid unwanted Vietnamese marks.
- On-demand update check: reads `latest.json` from GitHub Releases, shows the changelog and opens the download page. No background polling.

## Layout

```
core/                  Vietnamese engine (Rust std, C ABI in ffi/vietflow_core.h)
platforms/windows/     Win32 hook daemon, tray host, config watcher, updater
ui_imgui/              Dear ImGui + DirectX 11 settings UI
ui_imgui/third_party/  Git submodules: imgui, nlohmann_json
assets/                Application icon and brand assets
scripts/build.ps1      Release build and packaging script
Cargo.toml             Rust workspace
```

## Build

Requirements:

- Rust 1.75+ (`x86_64-pc-windows-msvc`)
- CMake 3.20+ and Visual Studio Build Tools (Desktop development with C++)
- Git
- Optional: .NET SDK and Velopack CLI (`dotnet tool install -g vpk`) only for installer packaging

Fetch the submodules:

```powershell
git clone --recurse-submodules https://github.com/KienPC1234/VietFlow.git
# or, in an existing clone:
git submodule update --init --recursive
```

`third_party/imgui` is pinned to `2fb554b` (1.93.1 WIP); `third_party/nlohmann_json` is pinned to `v3.11.3`.

Build commands:

```powershell
cargo test --manifest-path Cargo.toml

# Default: x64, static MSVC runtime (~1.51 MB)
.\scripts\build.ps1

# Both x64 and x86, with Velopack installers
.\scripts\build.ps1 -Arch All -Velopack

# Also create installers for the current architecture
.\scripts\build.ps1 -Velopack

# Smaller x64 build that requires the VC++ 2015-2022 Redistributable (~1.23 MB)
.\scripts\build.ps1 -DynamicCrt
```

Output: `dist\VietFlow.exe` (x64), `dist\VietFlow-x86.exe` (x86), plus `dist\latest.json`, `dist\version.txt` and `dist\CHANGELOG.md`. With `-Velopack`, installers are written to `releases\` as `VietFlow-win-x64-Setup.exe` and `VietFlow-win-x86-Setup.exe`. The default static build imports neither `VCRUNTIME140` nor `MSVCP140`.

## Release

`VERSION` and `CHANGELOG.md` are the single source of truth. `build.ps1` syncs the `Cargo.toml` version and generates `dist\version.txt`, `dist\latest.json`, and a copy of `CHANGELOG.md`.

1. Bump `VERSION` and add a section for that version to `CHANGELOG.md`.
2. Run `.\scripts\build.ps1 -Arch All -Velopack`.
3. Create and publish a GitHub release tagged `v<VERSION>`.
4. Upload the x64/x86 executables and installers, plus `latest.json`, `version.txt` and `CHANGELOG.md`.

The updater tries `releases/latest/download/latest.json`, then `version.txt`, then the GitHub API. `latest.json.downloads.x64` / `downloads.x86` point at the matching installer, and `latest.json.notes` is the changelog shown in the update dialog.

## License

Released under the **GNU General Public License v3.0 (GPLv3)**.
The Vietnamese input algorithm is derived from UniKey (Pham Kim Long).
