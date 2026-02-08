# PeaPod macOS

macOS protocol implementation for PeaPod (Network Extension, discovery, menu bar). Uses pea-core (Rust) via static lib or XCFramework when built.

## Environment

- **Xcode**: 15.0 or later (Swift 5.9, macOS 14 SDK). Install from the Mac App Store or [developer.apple.com](https://developer.apple.com/xcode/).
- **Rust targets** (from repo root):
  ```bash
  rustup target add aarch64-apple-darwin x86_64-apple-darwin   # Apple Silicon + Intel Mac
  ```

## Building pea-core for macOS

From the **repo root**:

```bash
# Apple Silicon
cargo build -p pea-core --target aarch64-apple-darwin --release

# Intel Mac
cargo build -p pea-core --target x86_64-apple-darwin --release
```

Use `target/<triple>/release/libpea_core.a` in your Xcode project, or build an XCFramework for a universal binary (document steps in .tasks/06-macos when implementing).

## Scaffold

This directory is a Swift Package placeholder. Replace or add an Xcode project (`.xcodeproj`) for the menu bar app and Network Extension when implementing per [.tasks/06-macos.md](../.tasks/06-macos.md).

## Tasks

See [.tasks/06-macos.md](../.tasks/06-macos.md) for the full macOS implementation checklist.
