# PeaPod macOS

macOS protocol implementation for PeaPod (Network Extension, discovery, menu bar). Uses pea-core (Rust) via static lib or XCFramework when built.

## Environment

- **Xcode**: 15.0 or later (Swift 5.9, macOS 14 SDK). Install from the Mac App Store or [developer.apple.com](https://developer.apple.com/xcode/).
- **Rust targets** (from repo root):
  ```bash
  rustup target add aarch64-apple-darwin x86_64-apple-darwin   # Apple Silicon + Intel Mac
  ```

## Building pea-core for macOS

From the **repo root**. You can run the helper script (on macOS) to build all Apple targets (iOS + macOS) at once:

```bash
./scripts/build-pea-core-apple.sh
```

Or build for macOS only:

```bash
# Apple Silicon
cargo build -p pea-core --target aarch64-apple-darwin --release

# Intel Mac
cargo build -p pea-core --target x86_64-apple-darwin --release
```

Use `target/<triple>/release/libpea_core.a` in your Xcode project, or build an XCFramework for a universal binary (document steps in .tasks/06-macos when implementing). Generate a C header from the repo root: `cbindgen pea-core -o pea_core.h` (see [docs/API.md](../docs/API.md)).

## Scaffold

This directory is a Swift Package placeholder. Replace or add an Xcode project (`.xcodeproj`) for the menu bar app and Network Extension when implementing per [.tasks/06-macos.md](../.tasks/06-macos.md).

**Next steps (from .tasks §1):** (1) Create an Xcode project with a macOS app target (Swift, AppKit or SwiftUI, e.g. macOS 12+). (2) Add a Network Extension target (Packet Tunnel or App Proxy). (3) Configure app groups if extension and app share state. (4) Build pea-core for `aarch64-apple-darwin` and `x86_64-apple-darwin`; add the static lib and C API to the project; call from Swift. (5) Enable Network Extensions and Personal VPN; set Sandbox and Hardened Runtime for distribution.

## Tasks

See [.tasks/06-macos.md](../.tasks/06-macos.md) for the full macOS implementation checklist.
