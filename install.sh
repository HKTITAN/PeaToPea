#!/usr/bin/env sh
# PeaPod Installer for Linux and macOS
# Usage: curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh
#
# This script:
#   1. Detects your OS and architecture
#   2. Asks for confirmation before installing
#   3. Installs the Rust toolchain (if not present)
#   4. Builds and installs pea-linux (Linux) or pea-macos (macOS)
#   5. Installs the systemd service (Linux) or launchd plist (macOS)
#
# Environment variables:
#   PEAPOD_PREFIX   - install prefix (default: /usr/local)
#   PEAPOD_NO_CONFIRM - set to 1 to skip confirmation prompts

set -e

# ── Colors ──────────────────────────────────────────────────────────

if [ -t 1 ] && command -v tput >/dev/null 2>&1; then
    BOLD=$(tput bold 2>/dev/null || true)
    GREEN=$(tput setaf 2 2>/dev/null || true)
    YELLOW=$(tput setaf 3 2>/dev/null || true)
    CYAN=$(tput setaf 6 2>/dev/null || true)
    RED=$(tput setaf 1 2>/dev/null || true)
    RESET=$(tput sgr0 2>/dev/null || true)
else
    BOLD="" GREEN="" YELLOW="" CYAN="" RED="" RESET=""
fi

info()  { printf "%s[info]%s  %s\n" "$CYAN"  "$RESET" "$1"; }
warn()  { printf "%s[warn]%s  %s\n" "$YELLOW" "$RESET" "$1"; }
error() { printf "%s[error]%s %s\n" "$RED"    "$RESET" "$1"; }
ok()    { printf "%s[ok]%s    %s\n" "$GREEN"  "$RESET" "$1"; }

# ── Banner ──────────────────────────────────────────────────────────

banner() {
    printf "\n"
    printf "%s" "$GREEN"
    cat <<'EOF'
    ____             ____            __
   / __ \___  ____ _/ __ \____  ____/ /
  / /_/ / _ \/ __ `/ /_/ / __ \/ __  /
 / ____/  __/ /_/ / ____/ /_/ / /_/ /
/_/    \___/\__,_/_/    \____/\__,_/
EOF
    printf "%s" "$RESET"
    printf "\n"
    printf "  %sPeaPod Installer%s — Pool internet across your devices\n" "$BOLD" "$RESET"
    printf "  https://github.com/HKTITAN/PeaToPea\n"
    printf "\n"
}

# ── Disclaimer ──────────────────────────────────────────────────────

disclaimer() {
    printf "%s" "$YELLOW"
    cat <<'EOF'
┌──────────────────────────────────────────────────────────────────┐
│                        WHAT YOU'RE INSTALLING                    │
├──────────────────────────────────────────────────────────────────┤
│                                                                  │
│  PeaPod is a protocol that lets nearby devices form an           │
│  encrypted local mesh and pool their internet connections        │
│  for faster uploads and downloads.                               │
│                                                                  │
│  This installer will:                                            │
│    • Install the Rust toolchain (if not already installed)       │
│    • Clone and build the PeaPod daemon from source               │
│    • Install the binary to /usr/local/bin/pea-linux              │
│    • Set up a systemd user service (Linux) so PeaPod starts     │
│      automatically when you log in                               │
│                                                                  │
│  PeaPod is open source (MIT License).                            │
│  Source: https://github.com/HKTITAN/PeaToPea                    │
│                                                                  │
│  PeaPod does NOT:                                                │
│    • Collect any personal data or telemetry                      │
│    • Require an account or sign-up                               │
│    • Route traffic through external servers                      │
│    • Modify your system proxy settings (Linux)                   │
│                                                                  │
│  PeaPod DOES:                                                    │
│    • Listen on localhost:3128 (HTTP proxy)                       │
│    • Listen on UDP port 45678 (LAN discovery)                    │
│    • Listen on TCP port 45679 (local transport)                  │
│    • Communicate with other PeaPod devices on your LAN           │
│                                                                  │
│  All peer-to-peer traffic is encrypted (X25519 + ChaCha20).     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
EOF
    printf "%s" "$RESET"
    printf "\n"
}

# ── Helpers ─────────────────────────────────────────────────────────

need_cmd() {
    if ! command -v "$1" >/dev/null 2>&1; then
        error "Required command not found: $1"
        exit 1
    fi
}

confirm() {
    if [ "${PEAPOD_NO_CONFIRM:-0}" = "1" ]; then
        return 0
    fi
    printf "  %s%s%s [y/N] " "$BOLD" "$1" "$RESET"
    read -r answer
    case "$answer" in
        [Yy]*) return 0 ;;
        *)     return 1 ;;
    esac
}

detect_os() {
    OS="$(uname -s)"
    ARCH="$(uname -m)"
    case "$OS" in
        Linux)  OS="linux" ;;
        Darwin) OS="macos" ;;
        *)      error "Unsupported OS: $OS"; exit 1 ;;
    esac
    case "$ARCH" in
        x86_64|amd64)   ARCH="x86_64" ;;
        aarch64|arm64)  ARCH="aarch64" ;;
        *)              error "Unsupported architecture: $ARCH"; exit 1 ;;
    esac
    info "Detected: $OS ($ARCH)"
}

# ── Install steps ───────────────────────────────────────────────────

install_rust() {
    if command -v rustc >/dev/null 2>&1 && command -v cargo >/dev/null 2>&1; then
        RUST_VER=$(rustc --version)
        ok "Rust already installed: $RUST_VER"
        return 0
    fi
    info "Rust is not installed. Installing via rustup..."
    if ! confirm "Install the Rust toolchain?"; then
        error "Rust is required to build PeaPod. Aborting."
        exit 1
    fi
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --quiet
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env" 2>/dev/null || true
    ok "Rust installed: $(rustc --version)"
}

clone_repo() {
    TMPDIR="${TMPDIR:-/tmp}"
    BUILD_DIR="$TMPDIR/peapod-build-$$"
    info "Cloning PeaPod repository..."
    need_cmd git
    if ! git clone --depth 1 https://github.com/HKTITAN/PeaToPea.git "$BUILD_DIR"; then
        error "Failed to clone repository. Check your internet connection."
        exit 1
    fi
    ok "Repository cloned to $BUILD_DIR"
}

build_binary() {
    info "Building PeaPod (release mode)... this may take a few minutes."
    cd "$BUILD_DIR"
    if [ "$OS" = "linux" ]; then
        if ! cargo build -p pea-linux --release; then
            error "Build failed. See output above for details."
            exit 1
        fi
        BINARY="target/release/pea-linux"
        BIN_NAME="pea-linux"
    else
        # macOS — build pea-linux for now (pea-macos is placeholder)
        if ! cargo build -p pea-linux --release; then
            error "Build failed. See output above for details."
            exit 1
        fi
        BINARY="target/release/pea-linux"
        BIN_NAME="pea-linux"
    fi
    if [ ! -f "$BINARY" ]; then
        error "Build failed — binary not found at $BINARY"
        exit 1
    fi
    ok "Build complete: $BINARY"
}

install_binary() {
    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    BIN_DIR="$PREFIX/bin"
    INSTALL_PATH="$BIN_DIR/$BIN_NAME"

    info "Installing $BIN_NAME to $INSTALL_PATH"

    if [ ! -d "$BIN_DIR" ]; then
        if mkdir -p "$BIN_DIR" 2>/dev/null; then
            true
        else
            info "Need elevated permissions to create $BIN_DIR"
            sudo mkdir -p "$BIN_DIR"
        fi
    fi

    if [ -w "$BIN_DIR" ]; then
        cp "$BUILD_DIR/$BINARY" "$INSTALL_PATH"
        chmod 755 "$INSTALL_PATH"
    else
        info "Need elevated permissions to install to $BIN_DIR"
        sudo cp "$BUILD_DIR/$BINARY" "$INSTALL_PATH"
        sudo chmod 755 "$INSTALL_PATH"
    fi
    ok "Installed: $INSTALL_PATH"
}

install_service_linux() {
    SERVICE_DIR="$HOME/.config/systemd/user"
    SERVICE_FILE="$SERVICE_DIR/peapod.service"

    if [ -f "$SERVICE_FILE" ]; then
        warn "Service file already exists: $SERVICE_FILE"
        if ! confirm "Overwrite existing service file?"; then
            info "Skipping service installation."
            return 0
        fi
    fi

    mkdir -p "$SERVICE_DIR"
    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    cat > "$SERVICE_FILE" <<SVCEOF
[Unit]
Description=PeaPod — encrypted local mesh for pooled bandwidth
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
ExecStart=$PREFIX/bin/pea-linux
Restart=on-failure
RestartSec=5

[Install]
WantedBy=default.target
SVCEOF

    ok "Service installed: $SERVICE_FILE"

    if command -v systemctl >/dev/null 2>&1; then
        systemctl --user daemon-reload 2>/dev/null || true
        if confirm "Enable PeaPod to start on login?"; then
            systemctl --user enable peapod.service 2>/dev/null || true
            ok "PeaPod will start automatically on login."
        fi
        if confirm "Start PeaPod now?"; then
            systemctl --user start peapod.service 2>/dev/null || true
            ok "PeaPod is running."
        fi
    fi
}

install_service_macos() {
    PLIST_DIR="$HOME/Library/LaunchAgents"
    PLIST_FILE="$PLIST_DIR/com.peapod.daemon.plist"

    if [ -f "$PLIST_FILE" ]; then
        warn "Launch agent already exists: $PLIST_FILE"
        if ! confirm "Overwrite existing launch agent?"; then
            info "Skipping service installation."
            return 0
        fi
        launchctl unload "$PLIST_FILE" 2>/dev/null || true
    fi

    mkdir -p "$PLIST_DIR"
    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    cat > "$PLIST_FILE" <<PLISTEOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>com.peapod.daemon</string>
    <key>ProgramArguments</key>
    <array>
        <string>$PREFIX/bin/pea-linux</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>NetworkState</key>
        <true/>
    </dict>
    <key>StandardErrorPath</key>
    <string>/tmp/peapod.err</string>
    <key>StandardOutPath</key>
    <string>/tmp/peapod.out</string>
</dict>
</plist>
PLISTEOF

    ok "Launch agent installed: $PLIST_FILE"
    if confirm "Start PeaPod now?"; then
        launchctl load "$PLIST_FILE" 2>/dev/null || true
        ok "PeaPod is running."
    fi
}

create_config_dir() {
    CONFIG_DIR="$HOME/.config/peapod"
    if [ ! -d "$CONFIG_DIR" ]; then
        mkdir -p "$CONFIG_DIR"
        cat > "$CONFIG_DIR/config.toml" <<CFGEOF
# PeaPod configuration
# See: https://github.com/HKTITAN/PeaToPea

# proxy_port = 3128
# discovery_port = 45678
# transport_port = 45679
CFGEOF
        ok "Config directory created: $CONFIG_DIR"
    fi
}

cleanup() {
    if [ "${LOCAL_BUILD:-0}" = "1" ]; then
        return
    fi
    if [ -n "$BUILD_DIR" ] && [ -d "$BUILD_DIR" ]; then
        rm -rf "$BUILD_DIR"
    fi
}

# ── Uninstall ───────────────────────────────────────────────────────

uninstall() {
    banner
    info "Uninstalling PeaPod..."

    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    INSTALL_PATH="$PREFIX/bin/pea-linux"

    # Stop services
    if [ "$(uname -s)" = "Linux" ]; then
        systemctl --user stop peapod.service 2>/dev/null || true
        systemctl --user disable peapod.service 2>/dev/null || true
        rm -f "$HOME/.config/systemd/user/peapod.service"
        systemctl --user daemon-reload 2>/dev/null || true
        ok "Systemd service removed."
    elif [ "$(uname -s)" = "Darwin" ]; then
        PLIST_FILE="$HOME/Library/LaunchAgents/com.peapod.daemon.plist"
        launchctl unload "$PLIST_FILE" 2>/dev/null || true
        rm -f "$PLIST_FILE"
        ok "Launch agent removed."
    fi

    # Remove binary
    if [ -f "$INSTALL_PATH" ]; then
        if [ -w "$INSTALL_PATH" ]; then
            rm -f "$INSTALL_PATH"
        else
            sudo rm -f "$INSTALL_PATH"
        fi
        ok "Binary removed: $INSTALL_PATH"
    fi

    info "Config at ~/.config/peapod/ was not removed (your settings are preserved)."
    ok "PeaPod has been uninstalled."
    exit 0
}

# ── Main ────────────────────────────────────────────────────────────

LOCAL_BUILD=0

main() {
    # Handle flags
    for arg in "$@"; do
        case "$arg" in
            --uninstall) uninstall ;;
            --yes|-y)    PEAPOD_NO_CONFIRM=1 ;;
            --local)     LOCAL_BUILD=1 ;;
            --help|-h)
                printf "Usage: install.sh [--yes] [--local] [--uninstall] [--help]\n"
                printf "  --yes         Skip confirmation prompts\n"
                printf "  --local       Build from the current directory (skip git clone)\n"
                printf "  --uninstall   Remove PeaPod from this system\n"
                printf "  --help        Show this help\n"
                exit 0
                ;;
        esac
    done

    banner
    disclaimer

    if ! confirm "Do you want to install PeaPod?"; then
        info "Installation cancelled."
        exit 0
    fi

    printf "\n"
    detect_os

    install_rust

    if [ "$LOCAL_BUILD" = "1" ]; then
        if [ ! -f "Cargo.toml" ]; then
            error "Not in the PeaPod repo root (Cargo.toml not found). Run from the repo root or remove --local."
            exit 1
        fi
        BUILD_DIR="$(pwd)"
        ok "Building from local checkout: $BUILD_DIR"
    else
        need_cmd curl
        need_cmd git
        clone_repo
        trap cleanup EXIT
    fi

    build_binary
    install_binary
    create_config_dir

    if [ "$OS" = "linux" ]; then
        install_service_linux
    elif [ "$OS" = "macos" ]; then
        install_service_macos
    fi

    printf "\n"
    printf "  %s┌──────────────────────────────────────────────────────┐%s\n" "$GREEN" "$RESET"
    printf "  %s│              PeaPod installed successfully!          │%s\n" "$GREEN" "$RESET"
    printf "  %s├──────────────────────────────────────────────────────┤%s\n" "$GREEN" "$RESET"
    printf "  %s│                                                      │%s\n" "$GREEN" "$RESET"
    printf "  %s│  Binary:  %s/bin/pea-linux%s│%s\n" "$GREEN" "${PEAPOD_PREFIX:-/usr/local}" "                  " "$RESET"
    printf "  %s│  Config:  ~/.config/peapod/config.toml               │%s\n" "$GREEN" "$RESET"
    printf "  %s│                                                      │%s\n" "$GREEN" "$RESET"
    printf "  %s│  Commands:                                           │%s\n" "$GREEN" "$RESET"
    printf "  %s│    pea-linux              Start manually             │%s\n" "$GREEN" "$RESET"
    printf "  %s│    pea-linux --version    Show version               │%s\n" "$GREEN" "$RESET"
    if [ "$OS" = "linux" ]; then
    printf "  %s│    systemctl --user status peapod   Check status     │%s\n" "$GREEN" "$RESET"
    fi
    printf "  %s│                                                      │%s\n" "$GREEN" "$RESET"
    printf "  %s│  Uninstall:                                          │%s\n" "$GREEN" "$RESET"
    printf "  %s│    curl -sSf <install-url> | sh -s -- --uninstall    │%s\n" "$GREEN" "$RESET"
    printf "  %s│                                                      │%s\n" "$GREEN" "$RESET"
    printf "  %s└──────────────────────────────────────────────────────┘%s\n" "$GREEN" "$RESET"
    printf "\n"
}

main "$@"
