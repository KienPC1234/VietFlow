#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
ADDON_DIR="${ROOT_DIR}/platforms/linux_fcitx5"
BUILD_DIR="${ADDON_DIR}/build"
INSTALL_PREFIX="/usr"
INSTALL_DEPS=1
AUTO_RUSTUP=1
DO_UNINSTALL=0
MIN_RUST_MINOR=80

log_info() { printf '[INFO] %s\n' "$*"; }
log_warn() { printf '[WARN] %s\n' "$*" >&2; }
log_err()  { printf '[ERROR] %s\n' "$*" >&2; }

usage() {
    cat <<EOF
Usage: $(basename "$0") [OPTIONS]

Build and install the VietFlow Fcitx5 addon on Linux.

Options:
  --no-deps          Skip system package dependency installation
  --no-rustup        Do not auto-install Rust via rustup if missing/outdated
  --prefix <dir>     CMake install prefix (default: /usr)
  --uninstall        Remove installed VietFlow Fcitx5 files
  -h, --help         Show this help message
EOF
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --no-deps)
            INSTALL_DEPS=0
            shift
            ;;
        --no-rustup)
            AUTO_RUSTUP=0
            shift
            ;;
        --prefix)
            if [[ $# -lt 2 || -z "$2" ]]; then
                log_err "--prefix requires a non-empty directory argument."
                exit 1
            fi
            INSTALL_PREFIX="$2"
            shift 2
            ;;
        --uninstall)
            DO_UNINSTALL=1
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            log_err "Unknown option: $1"
            usage >&2
            exit 1
            ;;
    esac
done

# Determine build user so cargo/cmake artifacts are not root-owned when invoked via sudo.
if [[ "${EUID}" -eq 0 && -n "${SUDO_USER:-}" && "${SUDO_USER}" != "root" ]]; then
    BUILD_USER="${SUDO_USER}"
    BUILD_HOME="$(getent passwd "${SUDO_USER}" | cut -d: -f6)"
else
    BUILD_USER=""
    BUILD_HOME="${HOME}"
fi

# Verify root/sudo privileges upfront and exit immediately if unavailable.
ensure_root_access() {
    if [[ "${EUID}" -eq 0 ]]; then
        return 0
    fi
    if ! command -v sudo >/dev/null 2>&1; then
        log_err "Root privileges are required and 'sudo' is not installed. Run as root."
        exit 1
    fi
    log_info "Checking sudo privileges..."
    if ! sudo -v; then
        log_err "Insufficient privileges (sudo authentication failed). Exiting."
        exit 1
    fi
}

run_as_root() {
    if [[ "${EUID}" -eq 0 ]]; then
        "$@"
    else
        sudo "$@"
    fi
}

run_as_build_user() {
    if [[ -n "${BUILD_USER}" ]]; then
        sudo -u "${BUILD_USER}" -H env PATH="${BUILD_HOME}/.cargo/bin:${PATH}" "$@"
    else
        "$@"
    fi
}

if [[ "${DO_UNINSTALL}" -eq 1 ]]; then
    ensure_root_access
    ADDON_LIBDIR="$(pkg-config --variable=addondir Fcitx5Core 2>/dev/null || echo "${INSTALL_PREFIX}/lib/x86_64-linux-gnu/fcitx5")"
    PKG_DATADIR="$(pkg-config --variable=pkgdatadir Fcitx5Core 2>/dev/null || echo "${INSTALL_PREFIX}/share/fcitx5")"
    log_info "Removing VietFlow Fcitx5 addon from ${ADDON_LIBDIR} and ${PKG_DATADIR}..."
    run_as_root rm -f \
        "${ADDON_LIBDIR}/vietflow.so" \
        "${PKG_DATADIR}/addon/vietflow.conf" \
        "${PKG_DATADIR}/inputmethod/vietflow.conf" \
        "${INSTALL_PREFIX}/share/icons/hicolor/"*"/apps/fcitx-vietflow.png"
    if command -v gtk-update-icon-cache >/dev/null 2>&1; then
        run_as_root gtk-update-icon-cache -f -t "${INSTALL_PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
    fi
    log_info "Uninstalled."
    exit 0
fi

ensure_root_access

install_system_deps() {
    if command -v apt-get >/dev/null 2>&1; then
        log_info "[1/5] Installing dependencies via apt-get..."
        run_as_root env DEBIAN_FRONTEND=noninteractive apt-get update

        local required_pkgs=(
            build-essential cmake pkg-config curl ca-certificates
            extra-cmake-modules gettext
            libfcitx5core-dev libfcitx5config-dev libfcitx5utils-dev
            fcitx5 fcitx5-config-qt fcitx5-frontend-gtk3 fcitx5-frontend-qt5
        )
        local optional_pkgs=(
            im-config fcitx5-frontend-gtk2 fcitx5-frontend-gtk4 fcitx5-frontend-qt6
        )
        local install_pkgs=("${required_pkgs[@]}")
        local pkg
        for pkg in "${optional_pkgs[@]}"; do
            if apt-cache show "$pkg" >/dev/null 2>&1; then
                install_pkgs+=("$pkg")
            fi
        done

        run_as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends "${install_pkgs[@]}"
        return 0
    fi

    if command -v dnf >/dev/null 2>&1; then
        log_info "[1/5] Installing dependencies via dnf..."
        run_as_root dnf install -y \
            gcc-c++ make cmake pkgconf-pkg-config curl ca-certificates \
            extra-cmake-modules gettext \
            fcitx5-devel fcitx5 fcitx5-configtool \
            fcitx5-gtk fcitx5-qt
        return 0
    fi

    if command -v pacman >/dev/null 2>&1; then
        log_info "[1/5] Installing dependencies via pacman..."
        run_as_root pacman -Sy --needed --noconfirm \
            base-devel cmake pkgconf curl ca-certificates \
            extra-cmake-modules gettext \
            fcitx5 fcitx5-configtool fcitx5-gtk fcitx5-qt
        return 0
    fi

    if command -v zypper >/dev/null 2>&1; then
        log_info "[1/5] Installing dependencies via zypper..."
        run_as_root zypper --non-interactive install \
            gcc-c++ make cmake pkg-config curl ca-certificates \
            extra-cmake-modules gettext-tools \
            fcitx5-devel fcitx5 fcitx5-configtool \
            fcitx5-gtk3 fcitx5-qt5
        return 0
    fi

    log_err "Unsupported package manager. Install Fcitx5 development packages manually and re-run with --no-deps."
    exit 1
}

if [[ "${INSTALL_DEPS}" -eq 1 ]]; then
    install_system_deps
fi

# Ensure cargo/rustc from rustup are on PATH.
export PATH="${BUILD_HOME}/.cargo/bin:${PATH}"

check_rust_version() {
    local ver major minor
    if ! command -v rustc >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then
        return 1
    fi
    ver="$(rustc --version | awk '{print $2}')"
    major="${ver%%.*}"
    minor="${ver#*.}"
    minor="${minor%%.*}"
    if [[ "${major}" -gt 1 ]] || [[ "${major}" -eq 1 && "${minor}" -ge "${MIN_RUST_MINOR}" ]]; then
        return 0
    fi
    log_warn "Found rustc ${ver}, but VietFlow requires Rust 1.${MIN_RUST_MINOR}+."
    return 1
}

log_info "[2/5] Checking Rust toolchain (1.${MIN_RUST_MINOR}+)..."
if ! check_rust_version; then
    if [[ "${AUTO_RUSTUP}" -ne 1 ]]; then
        log_err "Rust 1.${MIN_RUST_MINOR}+ is required. Install via https://rustup.rs or re-run without --no-rustup."
        exit 1
    fi
    if ! command -v curl >/dev/null 2>&1; then
        log_err "curl is required to install rustup. Re-run without --no-deps or install curl first."
        exit 1
    fi
    log_info "Installing Rust stable toolchain via rustup..."
    run_as_build_user bash -c "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --profile minimal"
    export PATH="${BUILD_HOME}/.cargo/bin:${PATH}"
    if ! check_rust_version; then
        log_err "Rust installation failed or version is still below 1.${MIN_RUST_MINOR}."
        exit 1
    fi
fi

log_info "[3/5] Building vietflow-core (release)..."
run_as_build_user cargo build --release -p vietflow-core --manifest-path "${ROOT_DIR}/Cargo.toml"

log_info "[4/5] Configuring and building Fcitx5 addon..."
if [[ -f "${BUILD_DIR}/CMakeCache.txt" ]]; then
    CACHED_PREFIX="$(grep -E '^CMAKE_INSTALL_PREFIX:PATH=' "${BUILD_DIR}/CMakeCache.txt" | cut -d= -f2- || true)"
    if [[ -n "${CACHED_PREFIX}" && "${CACHED_PREFIX}" != "${INSTALL_PREFIX}" ]]; then
        log_info "Prefix changed (${CACHED_PREFIX} -> ${INSTALL_PREFIX}), cleaning CMake cache..."
        rm -rf "${BUILD_DIR}"
    fi
fi

run_as_build_user cmake -S "${ADDON_DIR}" -B "${BUILD_DIR}" \
    -DCMAKE_BUILD_TYPE=Release \
    -DCMAKE_INSTALL_PREFIX="${INSTALL_PREFIX}"
run_as_build_user cmake --build "${BUILD_DIR}" -j"$(nproc)"

log_info "[5/5] Installing Fcitx5 addon to ${INSTALL_PREFIX}..."
run_as_root cmake --install "${BUILD_DIR}"

if command -v gtk-update-icon-cache >/dev/null 2>&1; then
    run_as_root gtk-update-icon-cache -f -t "${INSTALL_PREFIX}/share/icons/hicolor" >/dev/null 2>&1 || true
fi

ADDON_LIBDIR="$(pkg-config --variable=addondir Fcitx5Core 2>/dev/null || echo "${INSTALL_PREFIX}/lib/x86_64-linux-gnu/fcitx5")"
if [[ -f "${ADDON_LIBDIR}/vietflow.so" ]] && command -v ldd >/dev/null 2>&1; then
    if ldd -r "${ADDON_LIBDIR}/vietflow.so" 2>&1 | grep -q "not found"; then
        log_err "Shared library dependency check failed for ${ADDON_LIBDIR}/vietflow.so"
        ldd -r "${ADDON_LIBDIR}/vietflow.so" >&2
        exit 1
    fi
fi

cat <<EOF

VietFlow Fcitx5 addon installed successfully.

Next steps:
  1. Ensure Fcitx5 is your active input method framework:
       - On Debian/Ubuntu-based distros: im-config -n fcitx5
       - Otherwise set GTK_IM_MODULE=fcitx QT_IM_MODULE=fcitx XMODIFIERS=@im=fcitx (or enable Fcitx5 in your Wayland compositor settings).
  2. Log out and log back in (or run: fcitx5 -r -d).
  3. Open "Fcitx 5 Configuration" (fcitx5-configtool) and add "VietFlow".
EOF
