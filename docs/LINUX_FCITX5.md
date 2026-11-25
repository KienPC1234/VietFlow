# VietFlow for Linux (Fcitx5 Addon)

VietFlow provides a native Fcitx5 input method addon (`vietflow.so`) backed by the same Rust engine (`vietflow-core`) used on Windows.

## Supported Environments

- **Display Servers**: X11, XWayland, and Wayland (`text-input-v3` / `input-method-v2` via Fcitx5).
- **Distributions**: Any Linux distribution with Fcitx5 (`apt`, `dnf`, `pacman`, `zypper`, or manual build).

## 1. Installation

### Option A: Build & Install from Source (Recommended)

Run the installer script from the repository root. It verifies root/sudo privileges upfront, installs Fcitx5 and build dependencies for your package manager, installs Rust 1.80+ via `rustup` if missing, compiles `vietflow-core` and `vietflow.so`, and installs the addon into `/usr`:

```bash
./scripts/install_fcitx5_linux.sh
```

Useful flags:
- `--no-deps`: Skip system package installation.
- `--no-rustup`: Do not auto-install Rust if missing or outdated.
- `--prefix <dir>`: Custom CMake install prefix (default: `/usr`).
- `--uninstall`: Remove `vietflow.so` and Fcitx5 `.conf` files.

### Option B: Prebuilt Archive (`x86_64`)

For `x86_64` systems with `glibc 2.39+` and `libFcitx5Core.so.7` (e.g. Ubuntu 24.04+, Linux Mint 22+):

```bash
tar -xzf vietflow-fcitx5-linux-x86_64.tar.gz
cd vietflow-fcitx5-linux-x86_64
./install.sh
```

## 2. Enabling Fcitx5 & VietFlow

### Debian / Ubuntu / Linux Mint (X11)

```bash
im-config -n fcitx5
```

Log out and log back in (or restart Fcitx5 with `fcitx5 -r -d`).

### Other Distros (Fedora, Arch, openSUSE)

Add the following to `~/.pam_environment` or `/etc/environment` (for X11 / XWayland apps):

```ini
GTK_IM_MODULE=fcitx
QT_IM_MODULE=fcitx
XMODIFIERS=@im=fcitx
```

### Wayland Notes

- **KDE Plasma / Sway / Hyprland**: Select **Fcitx 5** as the Virtual Keyboard / Input Method in your compositor or system settings instead of forcing `GTK_IM_MODULE=fcitx`.
- **Chromium / Google Chrome / Electron (VS Code, Discord, Obsidian)**: If running under native Wayland (`--ozone-platform=wayland`), pass `--enable-wayland-ime` (or add it to `~/.config/chrome-flags.conf` / `~/.config/electron-flags.conf`).

### Adding VietFlow to Fcitx5

1. Open **Fcitx 5 Configuration** (`fcitx5-configtool`).
2. Search for **VietFlow** in the right-hand Available Input Method list and move it to Current Input Method.
3. Click the configure icon next to **VietFlow** to customize:
   - Input Method (`Telex`, `VNI`, `VIQR`, `Simple Telex`, `Simple Telex 2`, `Telex + VNI`)
   - Output Charset (`Unicode (NFC)`, `Unicode Composite (NFD)`, `TCVN3 (ABC)`, `VNI Windows`, `VIQR`)
   - Spell Check, Modern Tone Placement (`oà`, `uý`), Auto-Restore English Words, Free Tone Marking, Fix Shift Lag
   - Macros (including `@date`, `@time`, `@datetime`, `@tab`, `@newline`, `@clipboard`)
   - Custom English Words, Removed English Words, Foreign Initials

Configuration is persisted at `~/.config/fcitx5/conf/vietflow.conf` and reloads immediately on save.

## 3. Macro `@clipboard` Dependency

To expand `@clipboard` inside macros:
- **Wayland**: install `wl-clipboard` (`wl-paste`).
- **X11**: install `xclip` or `xsel`.

## 4. Troubleshooting

- **VietFlow does not appear in `fcitx5-configtool`**:
  - Verify `vietflow.so` is in your Fcitx5 addon directory (`pkg-config --variable=addondir Fcitx5Core`, typically `/usr/lib/x86_64-linux-gnu/fcitx5` or `/usr/lib/fcitx5`).
  - Restart Fcitx5: `fcitx5 -r -d`.
- **Cannot type Vietnamese in a specific app**:
  - Run `fcitx5-diagnose` to check missing GTK/Qt frontend modules (`fcitx5-frontend-gtk3`, `fcitx5-frontend-gtk4`, `fcitx5-frontend-qt5`, `fcitx5-frontend-qt6`).
  - For snap/flatpak apps, ensure the portal/frontend integration for Fcitx5 is enabled.
