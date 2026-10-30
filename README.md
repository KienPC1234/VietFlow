# VietFlow

A native Windows Vietnamese input method. The engine and keyboard hook are Rust; the settings UI is a separate Dear ImGui + DirectX 11 process.

Repository: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

## Why VietFlow

These are the parts that address common complaints about classic UniKey: accidental hotkeys in games, a fixed English list, 32-bit installs and low-DPI settings.

- VietFlow disables the global shortcuts in apps on the exclusion list, so Ctrl+Shift in a game cannot switch V/E. The tray icon and its menu still change the language by hand. The lock is a checkbox if you want shortcuts back.
- Each app can override the typing method, charset, key-injection mode and macro permission. Available modes are Unicode, scan-code, clipboard and a slower safe mode for games, Chromium apps and remote sessions.
- The default English list leaves out short words that are also Vietnamese keystrokes (`test`, `reset`, `me`). A searchable popup adds or removes any keyword, including the defaults.
- The converter uses the Rust engine for Unicode, TCVN3 and VNI text, including raw-byte clipboard input and output. Macros can be imported and exported as TSV, CSV or `trigger=value` lines.
- Builds for x64, x86 and ARM64 use the static MSVC runtime, so the target machine does not need the VC++ Redistributable.
- The UI scales per monitor from 100% to 200%; tabs, preset buttons, popups and dialogs fit at the minimum window width.
- Press a shortcut to record the settings or converter hotkey. You can also type it by hand.
- The update check runs only when you ask. It reads a release manifest with the changelog and separate x64/x86 installers.
- The engine supports Telex, VNI and VIQR, modern tone placement, free marking, macros with `@date`, `@time` and `@clipboard`, and custom tray icons.

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

- Rust 1.75+ (`x86_64-pc-windows-msvc`; add `i686-pc-windows-msvc` and `aarch64-pc-windows-msvc` for the other architectures)
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

# Default: x64, static MSVC runtime (~1.6 MB)
.\scripts\build.ps1

# x64, x86 and ARM64, with Velopack installers
.\scripts\build.ps1 -Arch All -Velopack

# One architecture
.\scripts\build.ps1 -Arch arm64

# Also create installers for the current architecture
.\scripts\build.ps1 -Velopack

# Smaller x64 build that requires the VC++ 2015-2022 Redistributable (~1.23 MB)
.\scripts\build.ps1 -DynamicCrt
```

Output: `dist\VietFlow.exe` (x64), `dist\VietFlow-x86.exe` (x86), `dist\VietFlow-arm64.exe` (ARM64), plus `dist\latest.json`, `dist\version.txt` and `dist\CHANGELOG.md`. With `-Velopack`, installers are written to `releases\` as `VietFlow-win-x64-Setup.exe`, `VietFlow-win-x86-Setup.exe` and `VietFlow-win-arm64-Setup.exe`. The default static build imports neither `VCRUNTIME140` nor `MSVCP140`.

## Release

Releases are published only for a feature milestone, a breaking change or a critical security fix. Small fixes wait for the next milestone.

`VERSION` and `CHANGELOG.md` are the single source of truth. Run `.\scripts\build.ps1 -Arch All -Velopack` to build the executables and installers. See [docs/RELEASING.md](docs/RELEASING.md) for the asset and updater checklist.

## License

Released under the GNU General Public License v3.0 (GPLv3).
The Vietnamese input algorithm is derived from UniKey (Pham Kim Long).
