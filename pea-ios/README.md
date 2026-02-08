# PeaPod iOS

iOS protocol implementation for PeaPod (Network Extension, discovery, transport). Uses pea-core (Rust) via static lib or XCFramework when built.

## Environment

- **Xcode**: 15.0 or later (Swift 5.9, iOS 17 SDK). Install from the Mac App Store or [developer.apple.com](https://developer.apple.com/xcode/).
- **Rust targets** (from repo root):
  ```bash
  rustup target add aarch64-apple-ios x86_64-apple-ios   # device + simulator
  ```

## Building pea-core for iOS

From the **repo root**, build pea-core as a static library for device and/or simulator. You can run the helper script (on macOS) to build all Apple targets at once:

```bash
./scripts/build-pea-core-apple.sh
```

Or build manually:

```bash
# Device (arm64)
cargo build -p pea-core --target aarch64-apple-ios --release

# Simulator (x86_64 or aarch64-apple-ios-sim when available)
cargo build -p pea-core --target x86_64-apple-ios --release
```

Then either:

- **Static lib**: Use `target/<triple>/release/libpea_core.a` in your Xcode project and link from the app or extension target.
- **XCFramework**: Build a universal binary and wrap in an XCFramework so one artifact works for device and simulator; document the exact `lipo`/`xcodebuild -create-xcframework` steps in .tasks/05-ios when implementing.

## Scaffold

This directory is a Swift Package placeholder. Replace or add an Xcode project (`.xcodeproj`) for the app and Network Extension when implementing per [.tasks/05-ios.md](../.tasks/05-ios.md).

**Next steps (from .tasks §1):** (1) Create an Xcode project with an iOS app target (Swift, e.g. iOS 15+). (2) Add a Network Extension target (Packet Tunnel or App Proxy). (3) Configure app groups for shared state between app and extension. (4) Build pea-core for `aarch64-apple-ios` and `x86_64-apple-ios`; add the static lib and C header to the project; call from Swift. (5) Enable Network Extensions and Personal VPN capabilities; add local network usage description.

## Tasks

See [.tasks/05-ios.md](../.tasks/05-ios.md) for the full iOS implementation checklist.
