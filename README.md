# VietFlow

High-performance, modern Vietnamese input method written in 100% Rust.
Source: [https://github.com/KienPC1234/VietFlow](https://github.com/KienPC1234/VietFlow)

## Architecture Overview

```
                        +----------------------------+
                        |       vietflow-core        |
                        | (100% Pure Rust Engine)   |
                        | Telex, VNI, Tone, Macro    |
                        +--------------+-------------+
                                       |
                 +---------------------+---------------------+
                 |                                           |
                 v                                           v
      +--------------------+                      +--------------------+
      |  vietflow-windows  |                      |    Fcitx5 Addon    |
      | Windows Native Host|                      |    (Linux Host)    |
      | Low-level Hook     |                      | C-ABI Integration  |
      +---------+----------+                      +--------------------+
                |
                v
      +--------------------+
      |    vietflow-gui    |
      |  Modern Slint UI   |
      | Unified Config/Tool|
      +--------------------+
```

## Workspace Crates

* `crates/vietflow-core`: Pure Rust cross-platform Vietnamese engine with full Telex/VNI support, tone placement algorithms, macro expansion with dynamic variables (`@date`, `@time`, `@clipboard`, etc.), spellcheck, and C-ABI export.
* `crates/vietflow-windows`: High-performance, lightweight Windows daemon with `WH_KEYBOARD_LL` hook, system tray icon, Game Mode blacklist, auto-startup, and global hotkeys (`Ctrl+Shift`, `Ctrl+Shift+F5`, `Ctrl+Shift+F9`). Binary size < 400 KB, memory footprint < 2 MB.
* `crates/vietflow-gui`: Modern, declarative configuration panel and clipboard charset converter powered by Slint.

## Build & Test

```bash
# Run unit tests across workspace
cargo test --workspace

# Build optimized release binaries
cargo build --workspace --release
```

Release binaries are produced in `target/release/`:
* `vietflow-windows.exe`: Background daemon and hook runner.
* `vietflow-gui.exe`: Configuration dashboard and charset converter.
* `vietflow_core.dll` / `libvietflow_core.rlib`: Core library and C-ABI exports.

## License

GNU General Public License v3.0 (GPLv3).
Core input algorithms derived and evolved from UniKey by Pham Kim Long.
