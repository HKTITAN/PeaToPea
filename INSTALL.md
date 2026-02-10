# Installing PeaPod

PeaPod can be installed in several ways depending on your platform and preference.

## Download pre-built binaries

The fastest way to get started — no build tools required.

### From GitHub Releases

Download the latest release for your platform from the [Releases page](https://github.com/HKTITAN/PeaToPea/releases):

| Platform | Asset | Notes |
|----------|-------|-------|
| Linux (x86_64) | `pea-linux-x86_64` | Standalone binary |
| Windows (x86_64) | `pea-windows-x86_64.exe` | Standalone binary |
| Android | `app-debug.apk` | Debug APK |

### Linux / macOS — one-line binary install

Downloads a pre-built binary from GitHub Releases (no Rust or compiler needed):

```bash
curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh -s -- --binary
```

This downloads the latest release binary, installs it to `/usr/local/bin`, and sets up the systemd service (Linux) or launch agent (macOS).

### Windows — one-line binary install

Downloads a pre-built binary from GitHub Releases (no Rust or compiler needed):

```powershell
iwr -useb https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.ps1 | iex -- --binary
```

### Manual binary install (Linux)

```bash
# Download
curl -L -o pea-linux https://github.com/HKTITAN/PeaToPea/releases/latest/download/pea-linux-x86_64

# Make executable
chmod +x pea-linux

# Move to PATH
sudo mv pea-linux /usr/local/bin/

# Run
pea-linux
```

### Manual binary install (Windows)

1. Download `pea-windows-x86_64.exe` from the [latest release](https://github.com/HKTITAN/PeaToPea/releases/latest).
2. Move it to a folder in your PATH (e.g. `%LOCALAPPDATA%\PeaPod\`).
3. Run `pea-windows.exe`.

## Build from source

If you prefer to build from source (requires [Rust](https://rustup.rs)):

### One-line install (build from source)

The install scripts handle everything automatically — Rust toolchain, build dependencies, building, and service setup:

```bash
# Linux / macOS
curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh
```

```powershell
# Windows (PowerShell)
iwr -useb https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.ps1 | iex
```

### Build from a local clone

```bash
git clone https://github.com/HKTITAN/PeaToPea.git
cd PeaToPea

# Linux / macOS
./install.sh --local --yes

# Windows (PowerShell)
.\install.ps1 --local --yes
```

### Build manually with Cargo

```bash
# Linux
cargo build -p pea-linux --release
# Binary: target/release/pea-linux

# Windows
cargo build -p pea-windows --release
# Binary: target/release/pea-windows.exe
```

## Linux packages

### .deb (Debian/Ubuntu)

```bash
cargo install cargo-deb
cargo deb -p pea-linux
sudo dpkg -i target/debian/pea-linux_*.deb
```

After install, enable the systemd user service:

```bash
systemctl --user daemon-reload
systemctl --user enable peapod
systemctl --user start peapod
```

## Windows installer (Inno Setup)

A graphical Windows installer can be built using the Inno Setup script in `pea-windows/installer/`. See [pea-windows/installer/README.md](pea-windows/installer/README.md) for instructions.

## Android

See [pea-android/README.md](pea-android/README.md) for building and installing the Android app. Debug APKs are also available from the [Releases page](https://github.com/HKTITAN/PeaToPea/releases).

## Manage your installation

```bash
# Update to latest version
install.sh --update        # Linux / macOS
install.ps1 --update       # Windows

# Change settings (auto-start, service, proxy)
install.sh --modify        # Linux / macOS
install.ps1 --modify       # Windows

# Uninstall
install.sh --uninstall     # Linux / macOS
install.ps1 --uninstall    # Windows
```

## Platform-specific READMEs

- [pea-linux/README.md](pea-linux/README.md) — Linux daemon, systemd, .deb, firewall, configuration
- [pea-windows/README.md](pea-windows/README.md) — Windows app, tray icon, proxy, installer
- [pea-android/README.md](pea-android/README.md) — Android app (Gradle/Kotlin)
- [pea-ios/README.md](pea-ios/README.md) — iOS (placeholder)
- [pea-macos/README.md](pea-macos/README.md) — macOS (placeholder)
