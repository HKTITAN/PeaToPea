# PeaPod: Project PeaToPea

**PeaPod** is a **protocol**—like Bluetooth, WiFi, or Hotspot—that lets nearby devices form an encrypted local swarm and pool their internet connections to speed up uploads and downloads.

## What we're building

- **A protocol, not an app.** Operating systems and devices *implement* PeaPod. When enabled (e.g. in device settings next to WiFi and Hotspot), a device advertises that it speaks PeaPod, discovers other PeaPod-capable devices on the same LAN, and forms a **pod** with them.

- **Cooperative bandwidth.** When any device in the pod sends or receives data, the work is split into chunks. Each device uses its own internet connection to fetch or send its assigned chunks; chunks are then exchanged over the fast local link and reassembled on the device that requested the data. The result: faster downloads and uploads without changing ISPs, servers, or apps.

- **Transparent and safe.** Apps keep working as usual. PeaPod runs below the application layer and only accelerates traffic that supports it (e.g. HTTP range requests). Everything between devices in a pod is encrypted; devices are identified by cryptographic IDs, with no central server or logging.

- **Reference implementation and native clients.** This repo contains:
  - **pea-core** — shared protocol logic (identity, crypto, chunking, scheduling, integrity) in Rust; no I/O, so any platform can plug in its own discovery and traffic interception.
  - **Protocol implementations** per OS: Windows, Android, Linux, iOS, macOS—each using the right mechanism (system proxy, VPNService, Network Extension, etc.) so that any mix of devices can join the same pod.

PeaPod enhances the internet; it does not replace it.

Licensed under the [MIT License](LICENSE). For Rust dependency licenses: `cargo install cargo-license && cargo license`.

## Repo layout

- [.tasks/](.tasks/README.md) — Task breakdown and checklists for the full project.
- `pea-core/` — PeaPod protocol reference implementation (Rust library).
- `pea-windows/`, `pea-linux/` — Stub binaries for Windows and Linux implementations (in progress).
- `pea-android/`, `pea-ios/`, `pea-macos/` — Placeholders for mobile and macOS implementations.

## Build and test

From the repo root (requires [Rust](https://rustup.rs)):

```bash
cargo build -p pea-core
cargo test -p pea-core
```

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
- **Contributing**: [CONTRIBUTING.md](CONTRIBUTING.md) — Branching, commits, and how to push to the PeaToPea repo.
- **Cursor**: Rules, skills, and subagents in [.cursor/](.cursor/) for consistent AI-assisted development (rules in `.cursor/rules/`, skills in `.cursor/skills/`, agents in `.cursor/agents/`).
