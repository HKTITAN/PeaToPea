# PeaPod: Project PeaToPea

**PeaPod** is a protocol—like Bluetooth, WiFi, or Hotspot—that lets nearby devices form an encrypted local swarm and pool their internet connections to speed up uploads and downloads.

![PeaPod: Until now vs mesh](PeaToPea.png)

## What PeaPod is

Today, each of your devices connects to the internet on its own. Your phone uses cellular or WiFi; your laptop uses WiFi or ethernet; your tablet does the same. They don’t help each other. If one device has a slow link, it stays slow. If another has a fast link, that speed isn’t shared. PeaPod changes that by turning your nearby devices into a **pod**: a small mesh that talks to the internet as a coordinated group.

In that mesh, each device is a **pea**. Peas discover each other on the same local network (e.g. your home WiFi), authenticate using cryptographic identities, and form an encrypted **pea pod**. Inside the pod, devices communicate directly with each other over the local link. To the outside world, the pod can use whichever internet connection is available on any of its members. So instead of each pea connecting alone to “WiFi” or “Network A,” the pod collectively uses those connections and shares the work.

## How it works

- **Protocol.** PeaPod is a protocol. Operating systems and devices implement it. When you enable it (e.g. in device settings, alongside WiFi and Hotspot), your device advertises that it speaks PeaPod, discovers other PeaPod-capable devices on the same LAN, and joins or forms a pod with them.

- **Cooperative bandwidth.** When any device in the pod sends or receives data, the work is split into chunks. Each device uses its own internet connection to fetch or send its assigned chunks; chunks are then exchanged over the fast local link and reassembled on the device that requested the data. The result is faster downloads and uploads without changing ISPs, servers, or apps.

- **Transparent and safe.** Apps keep working as usual. PeaPod runs below the application layer and only accelerates traffic that supports it (e.g. HTTP range requests). Everything between devices in a pod is encrypted; devices are identified by cryptographic IDs, with no central server or logging.

- **Reference implementation and native clients.** This repo contains **pea-core** (shared protocol logic: identity, crypto, chunking, scheduling, integrity) in Rust—with no I/O so any platform can plug in its own discovery and traffic interception—and protocol implementations per OS: Windows, Android, Linux, iOS, macOS. Each implementation uses the right mechanism for that platform (system proxy, VPNService, Network Extension, etc.) so that any mix of devices can join the same pod.

PeaPod enhances the internet; it does not replace it.

Licensed under the [MIT License](LICENSE). For Rust dependency licenses: `cargo install cargo-license && cargo license`.

## Repo layout

- [.tasks/](.tasks/README.md) — Task breakdown and checklists for the full project.
- `pea-core/` — PeaPod protocol reference implementation (Rust library). Build/test: see [pea-core/README.md](pea-core/README.md). Wire format and discovery are specified in [docs/PROTOCOL.md](docs/PROTOCOL.md).
- **Implementations (per OS):**
  - [pea-windows/](pea-windows/README.md) — Windows: proxy, discovery, transport, tray. Build/run: see [pea-windows/README.md](pea-windows/README.md).
  - [pea-android/](pea-android/README.md) — Android app (Gradle/Kotlin, VPNService). Build/run: see [pea-android/README.md](pea-android/README.md).
  - [pea-linux/](pea-linux/README.md) — Linux daemon: proxy, discovery, transport, systemd. Build/run: see [pea-linux/README.md](pea-linux/README.md).
  - [pea-ios/](pea-ios/README.md) — iOS (Swift, Network Extension). Placeholder; see [pea-ios/README.md](pea-ios/README.md).
  - [pea-macos/](pea-macos/README.md) — macOS (Swift, menu bar, Network Extension). Placeholder; see [pea-macos/README.md](pea-macos/README.md).

## Install

**One-line install** — interactive, with disclaimers, tells you what it installs:

```bash
# Linux / macOS
curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh
```

```powershell
# Windows (PowerShell)
iwr -useb https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.ps1 | iex
```

The installers handle everything automatically:
- Show you exactly what PeaPod is and what it does
- Ask for confirmation before each step (skip with `--yes`)
- Install the Rust toolchain if needed (on Windows, automatically uses the GNU toolchain if Visual Studio Build Tools are not installed)
- Install build dependencies if needed (gcc/build-essential on Linux, Xcode CLI tools on macOS)
- Build from source and install the binary
- Set up the system service (systemd on Linux, launch agent on macOS, startup shortcut on Windows)

**Prerequisites** (installed automatically by the scripts if missing):

| Platform | Requirement | Notes |
|----------|-------------|-------|
| Linux | Rust, gcc, git, curl | Rust and gcc auto-installed; git/curl must be pre-installed |
| macOS | Rust, Xcode CLI Tools | Rust auto-installed; Xcode CLT prompted; git/curl included with macOS |
| Windows | Rust, Git | Rust auto-installed (uses GNU toolchain if no Visual Studio); Git must be pre-installed |

**Install from a local clone** (skip `git clone`, build from the repo you already have):

```bash
# Linux / macOS — from the repo root
./install.sh --local --yes
```

```powershell
# Windows (PowerShell) — from the repo root
.\install.ps1 --local --yes
```

**Android / iOS / macOS native:** See the platform-specific READMEs: [Android](pea-android/README.md), [iOS](pea-ios/README.md), [macOS](pea-macos/README.md).

**Uninstall:**

```bash
# Linux / macOS
curl -sSf https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.sh | sh -s -- --uninstall
```

```powershell
# Windows
iwr -useb https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.ps1 | iex -- --uninstall
```

## Build and test

From the repo root (requires [Rust](https://rustup.rs)):

```bash
cargo build -p pea-core
cargo test -p pea-core

# Or use make:
make dev      # Build, test, and lint (quick verification)
make build    # Build all crates
make test     # Run all tests
make lint     # Run fmt + clippy
make run      # Build and run pea-linux (debug)
make install  # Build release + install to /usr/local/bin
make help     # Show all commands
```

**Build and run per platform:** See each implementation’s README for prerequisites and steps: [Windows](pea-windows/README.md), [Android](pea-android/README.md), [Linux](pea-linux/README.md), [iOS](pea-ios/README.md), [macOS](pea-macos/README.md). On Linux, an optional [interop smoke script](docs/INTEROP.md#automated-interop-optional) runs two pea-linux instances and one proxy request (`./scripts/interop-two-linux.sh`).

Optional targets for platform implementations (add when working on that platform):

```bash
# Android (pea-android)
rustup target add aarch64-linux-android armv7-linux-androideabi i686-linux-android x86_64-linux-android

# iOS (pea-ios)
rustup target add aarch64-apple-ios x86_64-apple-ios

# macOS (pea-macos)
rustup target add aarch64-apple-darwin x86_64-apple-darwin
```

## Documentation

- **Project and task breakdown**: [.tasks/](.tasks/README.md) — Checklists and recommended order for building PeaPod.
- **Architecture**: [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) — Layer placement, components, and data flow.
- **Protocol**: [docs/PROTOCOL.md](docs/PROTOCOL.md) — Wire format, discovery, handshake, and versioning (reference: pea-core).
- **pea-core API**: [docs/API.md](docs/API.md) — Main types and methods for platform authors; C FFI and JNI notes.
- **Troubleshooting and FAQ**: [docs/TROUBLESHOOTING.md](docs/TROUBLESHOOTING.md) — Common issues and frequently asked questions.
- **Quality and metrics**: [docs/QUALITY.md](docs/QUALITY.md) — Edge cases, risk mitigations, and PRD success metrics.
- **Interop test matrix**: [docs/INTEROP.md](docs/INTEROP.md) — Cross-platform test pairs and results.
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md) — Branching, commits, and how to push to the PeaToPea repo.
- **Changelog**: [CHANGELOG.md](CHANGELOG.md) — Version history and protocol changes.
- **Release checklist**: [docs/RELEASE.md](docs/RELEASE.md) — Pre-release and release steps.
- **Scripts**: [scripts/](scripts/) — `interop-two-linux.sh` (Linux smoke test), `build-pea-core-apple.sh` (build pea-core for iOS/macOS), C header via `cbindgen pea-core -o pea_core.h`. See [scripts/README.md](scripts/README.md).
- **Cursor**: Rules, skills, and subagents in [.cursor/](.cursor/) for consistent AI-assisted development (rules in `.cursor/rules/`, skills in `.cursor/skills/`, agents in `.cursor/agents/`).
