# VietFlow UI — Qt 6 + Qt Quick (C++ backend)

Configuration frontend for VietFlow. The Rust core owns the input engine; this
process only edits configuration and calls the core through the C ABI in
`../ffi/vietflow_core.h` when the charset converter is used.

The native host (`VietFlow.exe`) keeps handling keystrokes. Qt is never loaded
inside the keyboard-hook process.

## Layout

```
ui/
├── CMakeLists.txt
├── src/
│   ├── main.cpp              Application entry, QML bootstrap
│   ├── settingsbackend.*     JSON config + QAbstractListModel for UI lists
│   ├── enginebridge.*        Rust C ABI wrapper (charset conversion)
│   └── theme.h               Design tokens exposed to QML
├── qml/
│   ├── Main.qml              Sidebar / header / pages / footer shell
│   ├── components/           Reusable themed controls
│   └── pages/                Seven settings pages
└── assets/                   Logo + colorized nav icons
```

## Build (Windows)

```powershell
# 1. Build the Rust core (produces vietflow_core.dll + import lib)
cargo build -p vietflow-core --release

# 2. Configure and build the Qt frontend
cmake -S ui -B ui/build -G "Visual Studio 17 2022" -A x64 `
      -DCMAKE_PREFIX_PATH="C:\Qt\6.8.3\msvc2022_64"
cmake --build ui/build --config Release

# 3. Run the UI from the build tree (development layout)
C:\Qt\6.8.3\msvc2022_64\bin\windeployqt.exe ui\build\Release\vietflow-ui.exe
```

For the single-file release (host + embedded Qt runtime) run:

```powershell
pwsh scripts/build.ps1
# -> dist\VietFlow.exe (one file; UI runtime extracted on first launch)
```

## Build (Linux / macOS)

```bash
cargo build -p vietflow-core --release
cmake -S ui -B ui/build -DCMAKE_BUILD_TYPE=Release
cmake --build ui/build -j
```

The UI reads the same configuration file as the native host:

- Windows: `%APPDATA%\VietFlow\vietflow_config.json`
- Linux: `~/.config/VietFlow/vietflow_config.json`
- macOS: `~/Library/Application Support/VietFlow/vietflow_config.json`

## Command line

- `vietflow-ui` — open the overview page.
- `vietflow-ui --convert` — open the converter page.
- `vietflow-ui --page N` — open page `N` (0–5), useful for QA screenshots.
