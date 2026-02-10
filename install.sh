#!/usr/bin/env sh
# PeaPod Installer for Linux and macOS
# Usage: curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh
#
# This script:
#   1. Detects your OS and architecture
#   2. Asks for confirmation before installing
#   3. Installs the Rust toolchain (if not present) — or downloads a pre-built binary (--binary)
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
│    • Or download a pre-built binary (with --binary flag)         │
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

# Install git and curl if not found (needed for cloning and downloading).
install_git_curl() {
    MISSING=""
    command -v git  >/dev/null 2>&1 || MISSING="$MISSING git"
    command -v curl >/dev/null 2>&1 || MISSING="$MISSING curl"
    if [ -z "$MISSING" ]; then
        return 0
    fi
    warn "Missing required tools:$MISSING"

    if [ "$OS" = "macos" ]; then
        # git and curl ship with Xcode Command Line Tools
        info "Installing Xcode Command Line Tools (provides git and curl)..."
        if confirm "Install Xcode Command Line Tools?"; then
            xcode-select --install 2>/dev/null || true
            info "If a dialog appeared, follow the prompts then re-run this installer."
            exit 0
        else
            error "git and curl are required. Please install them and re-run."
            exit 1
        fi
    fi

    if [ "$OS" = "linux" ]; then
        if command -v apt-get >/dev/null 2>&1; then
            info "Installing$MISSING via apt..."
            if confirm "Install$MISSING?"; then
                sudo apt-get update -qq && sudo apt-get install -y $MISSING
            fi
        elif command -v dnf >/dev/null 2>&1; then
            info "Installing$MISSING via dnf..."
            if confirm "Install$MISSING?"; then
                sudo dnf install -y $MISSING
            fi
        elif command -v yum >/dev/null 2>&1; then
            info "Installing$MISSING via yum..."
            if confirm "Install$MISSING?"; then
                sudo yum install -y $MISSING
            fi
        elif command -v pacman >/dev/null 2>&1; then
            info "Installing$MISSING via pacman..."
            if confirm "Install$MISSING?"; then
                sudo pacman -Sy --noconfirm $MISSING
            fi
        elif command -v zypper >/dev/null 2>&1; then
            info "Installing$MISSING via zypper..."
            if confirm "Install$MISSING?"; then
                sudo zypper install -y $MISSING
            fi
        elif command -v apk >/dev/null 2>&1; then
            info "Installing$MISSING via apk..."
            if confirm "Install$MISSING?"; then
                sudo apk add $MISSING
            fi
        fi
    fi

    # Verify
    command -v git  >/dev/null 2>&1 || { error "git is required but could not be installed. Please install it manually."; exit 1; }
    command -v curl >/dev/null 2>&1 || { error "curl is required but could not be installed. Please install it manually."; exit 1; }
    ok "git and curl are available."
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

install_build_deps() {
    # A C compiler/linker is needed by some Rust crates (e.g. ring, cc)
    if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then
        ok "C compiler found."
        return 0
    fi

    warn "No C compiler (cc/gcc) found. Some Rust crates require one."

    if [ "$OS" = "macos" ]; then
        info "Installing Xcode Command Line Tools (provides clang)..."
        if confirm "Install Xcode Command Line Tools?"; then
            xcode-select --install 2>/dev/null || true
            info "If a dialog appeared, follow the prompts then re-run this installer."
            exit 0
        else
            error "A C compiler is required to build PeaPod. Aborting."
            exit 1
        fi
    fi

    # Linux - try to auto-install via package manager
    if [ "$OS" = "linux" ]; then
        if command -v apt-get >/dev/null 2>&1; then
            info "Detected apt. Installing build-essential..."
            if confirm "Install build-essential (gcc, make, etc.)?"; then
                sudo apt-get update -qq && sudo apt-get install -y build-essential
            else
                error "A C compiler is required to build PeaPod. Install gcc or build-essential and re-run."
                exit 1
            fi
        elif command -v dnf >/dev/null 2>&1; then
            info "Detected dnf. Installing Development Tools..."
            if confirm "Install gcc and make?"; then
                sudo dnf install -y gcc make
            else
                error "A C compiler is required to build PeaPod. Install gcc and re-run."
                exit 1
            fi
        elif command -v yum >/dev/null 2>&1; then
            info "Detected yum. Installing Development Tools..."
            if confirm "Install gcc and make?"; then
                sudo yum install -y gcc make
            else
                error "A C compiler is required to build PeaPod. Install gcc and re-run."
                exit 1
            fi
        elif command -v pacman >/dev/null 2>&1; then
            info "Detected pacman. Installing base-devel..."
            if confirm "Install base-devel (gcc, make, etc.)?"; then
                sudo pacman -Sy --noconfirm base-devel
            else
                error "A C compiler is required to build PeaPod. Install gcc and re-run."
                exit 1
            fi
        elif command -v zypper >/dev/null 2>&1; then
            info "Detected zypper. Installing gcc and make..."
            if confirm "Install gcc and make?"; then
                sudo zypper install -y gcc make
            else
                error "A C compiler is required to build PeaPod. Install gcc and re-run."
                exit 1
            fi
        elif command -v apk >/dev/null 2>&1; then
            info "Detected apk. Installing build-base..."
            if confirm "Install build-base (gcc, make, musl-dev)?"; then
                sudo apk add build-base
            else
                error "A C compiler is required to build PeaPod. Install gcc and re-run."
                exit 1
            fi
        else
            error "No supported package manager found. Please install gcc manually and re-run."
            exit 1
        fi

        if command -v cc >/dev/null 2>&1 || command -v gcc >/dev/null 2>&1; then
            ok "C compiler installed."
        else
            error "Failed to install C compiler. Please install gcc manually and re-run."
            exit 1
        fi
    fi
}

fallback_to_source_build() {
    BINARY_INSTALL=0
    install_rust
    install_build_deps
    install_git_curl
    clone_repo
    trap cleanup EXIT
    build_binary
    install_binary
}

download_binary() {
    REPO="HKTITAN/PeaToPea"
    BIN_NAME="pea-linux"

    if [ "$ARCH" = "x86_64" ]; then
        ASSET_NAME="pea-linux-x86_64"
    elif [ "$ARCH" = "aarch64" ]; then
        ASSET_NAME="pea-linux-aarch64"
    else
        error "No pre-built binary available for architecture: $ARCH"
        error "Use the source install instead (without --binary)."
        exit 1
    fi

    # Check if a release exists via the GitHub API
    API_URL="https://api.github.com/repos/$REPO/releases/latest"
    DOWNLOAD_URL=""

    if API_RESPONSE=$(curl -fsSL "$API_URL" 2>/dev/null); then
        # Extract the browser_download_url for the matching asset
        DOWNLOAD_URL=$(printf '%s' "$API_RESPONSE" | grep -o "\"browser_download_url\"[[:space:]]*:[[:space:]]*\"[^\"]*${ASSET_NAME}\"" | head -1 | grep -o 'https://[^"]*')
        if [ -z "$DOWNLOAD_URL" ]; then
            TAG_NAME=$(printf '%s' "$API_RESPONSE" | grep -o '"tag_name"[[:space:]]*:[[:space:]]*"[^"]*"' | head -1 | grep -o '"[^"]*"$' | tr -d '"')
            error "Release '${TAG_NAME}' exists but does not contain '${ASSET_NAME}'."
        fi
    else
        error "No releases found for $REPO."
        error "The project has not published a release yet."
    fi

    if [ -z "$DOWNLOAD_URL" ]; then
        warn "Pre-built binary is not available."
        if confirm "Would you like to build from source instead?"; then
            fallback_to_source_build
            return
        else
            error "Installation cancelled. You can also try: install.sh (without --binary) to build from source."
            exit 1
        fi
    fi

    info "Downloading pre-built binary from GitHub Releases..."
    info "URL: $DOWNLOAD_URL"

    TMPDIR="${TMPDIR:-/tmp}"
    DOWNLOAD_PATH="$TMPDIR/$ASSET_NAME-$$"

    if ! curl -fSL -o "$DOWNLOAD_PATH" "$DOWNLOAD_URL"; then
        error "Failed to download binary from $DOWNLOAD_URL"
        warn "Pre-built binary download failed."
        if confirm "Would you like to build from source instead?"; then
            rm -f "$DOWNLOAD_PATH"
            fallback_to_source_build
            return
        else
            error "Installation cancelled. You can also try: install.sh (without --binary) to build from source."
            rm -f "$DOWNLOAD_PATH"
            exit 1
        fi
    fi

    chmod +x "$DOWNLOAD_PATH"
    ok "Binary downloaded: $DOWNLOAD_PATH"

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
        cp "$DOWNLOAD_PATH" "$INSTALL_PATH"
        chmod 755 "$INSTALL_PATH"
    else
        info "Need elevated permissions to install to $BIN_DIR"
        sudo cp "$DOWNLOAD_PATH" "$INSTALL_PATH"
        sudo chmod 755 "$INSTALL_PATH"
    fi

    rm -f "$DOWNLOAD_PATH"
    ok "Installed: $INSTALL_PATH"
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
    detect_os

    if ! confirm "Uninstall PeaPod?"; then
        info "Uninstall cancelled."
        exit 0
    fi

    info "Uninstalling PeaPod..."

    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    INSTALL_PATH="$PREFIX/bin/pea-linux"

    # Stop services
    if [ "$OS" = "linux" ]; then
        systemctl --user stop peapod.service 2>/dev/null || true
        systemctl --user disable peapod.service 2>/dev/null || true
        rm -f "$HOME/.config/systemd/user/peapod.service"
        systemctl --user daemon-reload 2>/dev/null || true
        ok "Systemd service removed."
    elif [ "$OS" = "macos" ]; then
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
    else
        warn "Binary not found at $INSTALL_PATH (already removed?)."
    fi

    info "Config at ~/.config/peapod/ was not removed (your settings are preserved)."
    ok "PeaPod has been uninstalled."
    exit 0
}

# ── Update ──────────────────────────────────────────────────────────

update() {
    banner
    detect_os
    info "Updating PeaPod to the latest version..."

    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    INSTALL_PATH="$PREFIX/bin/pea-linux"

    if [ ! -f "$INSTALL_PATH" ]; then
        error "PeaPod is not installed at $INSTALL_PATH. Run the installer first."
        exit 1
    fi

    # Show current version if binary supports --version
    CURRENT_VER=$("$INSTALL_PATH" --version 2>/dev/null || echo "unknown")
    info "Current version: $CURRENT_VER"

    if ! confirm "Download and build the latest version?"; then
        info "Update cancelled."
        exit 0
    fi

    # Ensure Rust is available
    # shellcheck source=/dev/null
    . "$HOME/.cargo/env" 2>/dev/null || true
    install_rust

    install_build_deps

    install_git_curl
    clone_repo
    trap cleanup EXIT

    build_binary

    # Stop running instance before replacing
    if [ "$OS" = "linux" ]; then
        systemctl --user stop peapod.service 2>/dev/null || true
    elif [ "$OS" = "macos" ]; then
        PLIST_FILE="$HOME/Library/LaunchAgents/com.peapod.daemon.plist"
        launchctl unload "$PLIST_FILE" 2>/dev/null || true
    fi

    install_binary

    # Restart service if it was running
    if [ "$OS" = "linux" ]; then
        if systemctl --user is-enabled peapod.service >/dev/null 2>&1; then
            systemctl --user start peapod.service 2>/dev/null || true
            ok "PeaPod service restarted."
        fi
    elif [ "$OS" = "macos" ]; then
        PLIST_FILE="$HOME/Library/LaunchAgents/com.peapod.daemon.plist"
        if [ -f "$PLIST_FILE" ]; then
            launchctl load "$PLIST_FILE" 2>/dev/null || true
            ok "PeaPod launch agent restarted."
        fi
    fi

    NEW_VER=$("$INSTALL_PATH" --version 2>/dev/null || echo "unknown")
    printf "\n"
    ok "PeaPod updated: $CURRENT_VER -> $NEW_VER"
    exit 0
}

# ── Modify ──────────────────────────────────────────────────────────

modify() {
    banner
    detect_os
    info "Modify PeaPod installation"

    PREFIX="${PEAPOD_PREFIX:-/usr/local}"
    INSTALL_PATH="$PREFIX/bin/pea-linux"

    if [ ! -f "$INSTALL_PATH" ]; then
        error "PeaPod is not installed at $INSTALL_PATH. Run the installer first."
        exit 1
    fi

    printf "\n"
    printf "  %sWhat would you like to change?%s\n" "$BOLD" "$RESET"
    printf "\n"
    printf "  1) Toggle auto-start on login\n"
    printf "  2) Reinstall / repair the systemd service or launchd agent\n"
    printf "  3) Remove config (reset to defaults)\n"
    printf "  4) Cancel\n"
    printf "\n"
    printf "  Choice [1-4]: "
    read -r choice

    case "$choice" in
        1)
            if [ "$OS" = "linux" ]; then
                if systemctl --user is-enabled peapod.service >/dev/null 2>&1; then
                    info "Auto-start is currently ENABLED."
                    if confirm "Disable auto-start?"; then
                        systemctl --user disable peapod.service 2>/dev/null || true
                        ok "Auto-start disabled. PeaPod will not start on login."
                    fi
                else
                    info "Auto-start is currently DISABLED."
                    if confirm "Enable auto-start?"; then
                        if [ ! -f "$HOME/.config/systemd/user/peapod.service" ]; then
                            info "Service file missing. Reinstalling..."
                            install_service_linux
                        else
                            systemctl --user enable peapod.service 2>/dev/null || true
                        fi
                        ok "Auto-start enabled. PeaPod will start on login."
                    fi
                fi
            elif [ "$OS" = "macos" ]; then
                PLIST_FILE="$HOME/Library/LaunchAgents/com.peapod.daemon.plist"
                if [ -f "$PLIST_FILE" ]; then
                    info "Launch agent is currently INSTALLED."
                    if confirm "Remove launch agent (disable auto-start)?"; then
                        launchctl unload "$PLIST_FILE" 2>/dev/null || true
                        rm -f "$PLIST_FILE"
                        ok "Auto-start disabled."
                    fi
                else
                    info "Launch agent is currently NOT installed."
                    if confirm "Install launch agent (enable auto-start)?"; then
                        install_service_macos
                    fi
                fi
            fi
            ;;
        2)
            if [ "$OS" = "linux" ]; then
                info "Reinstalling systemd service..."
                systemctl --user stop peapod.service 2>/dev/null || true
                install_service_linux
            elif [ "$OS" = "macos" ]; then
                info "Reinstalling launch agent..."
                PLIST_FILE="$HOME/Library/LaunchAgents/com.peapod.daemon.plist"
                launchctl unload "$PLIST_FILE" 2>/dev/null || true
                install_service_macos
            fi
            ;;
        3)
            CONFIG_DIR="$HOME/.config/peapod"
            if [ -d "$CONFIG_DIR" ]; then
                if confirm "Remove $CONFIG_DIR and reset to defaults?"; then
                    rm -rf "$CONFIG_DIR"
                    create_config_dir
                    ok "Config reset to defaults."
                fi
            else
                info "No config directory found. Creating defaults..."
                create_config_dir
            fi
            ;;
        4|*)
            info "No changes made."
            ;;
    esac
    exit 0
}

# ── Main ────────────────────────────────────────────────────────────

LOCAL_BUILD=0
BINARY_INSTALL=0

main() {
    # Handle flags
    for arg in "$@"; do
        case "$arg" in
            --uninstall) uninstall ;;
            --update)    update ;;
            --modify)    modify ;;
            --yes|-y)    PEAPOD_NO_CONFIRM=1 ;;
            --local)     LOCAL_BUILD=1 ;;
            --binary)    BINARY_INSTALL=1 ;;
            --help|-h)
                printf "Usage: install.sh [--yes] [--local] [--binary] [--uninstall] [--update] [--modify] [--help]\n"
                printf "  --yes         Skip confirmation prompts\n"
                printf "  --local       Build from the current directory (skip git clone)\n"
                printf "  --binary      Download a pre-built binary from GitHub Releases (no Rust needed)\n"
                printf "  --uninstall   Remove PeaPod from this system\n"
                printf "  --update      Update PeaPod to the latest version\n"
                printf "  --modify      Change PeaPod settings (auto-start, service, config)\n"
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

    if [ "$BINARY_INSTALL" = "1" ]; then
        need_cmd curl
        download_binary
    elif [ "$LOCAL_BUILD" = "1" ]; then
        install_rust
        install_build_deps
        if [ ! -f "Cargo.toml" ]; then
            error "Not in the PeaPod repo root (Cargo.toml not found). Run from the repo root or remove --local."
            exit 1
        fi
        BUILD_DIR="$(pwd)"
        ok "Building from local checkout: $BUILD_DIR"
        build_binary
        install_binary
    else
        install_rust
        install_build_deps
        install_git_curl
        clone_repo
        trap cleanup EXIT
        build_binary
        install_binary
    fi

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
    printf "  %s│  Manage:                                             │%s\n" "$GREEN" "$RESET"
    printf "  %s│    install.sh --update    Update to latest version   │%s\n" "$GREEN" "$RESET"
    printf "  %s│    install.sh --modify    Change settings            │%s\n" "$GREEN" "$RESET"
    printf "  %s│    install.sh --uninstall Remove PeaPod              │%s\n" "$GREEN" "$RESET"
    printf "  %s│                                                      │%s\n" "$GREEN" "$RESET"
    printf "  %s└──────────────────────────────────────────────────────┘%s\n" "$GREEN" "$RESET"
    printf "\n"
}

main "$@"
