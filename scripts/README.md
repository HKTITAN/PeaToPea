# Scripts

Helper scripts for building and testing PeaPod. Run from the **repo root**.

| Script | Platform | Description |
|--------|----------|-------------|
| [interop-two-linux.sh](interop-two-linux.sh) | Linux | Optional smoke test: starts two pea-linux instances and runs one HTTP request through the first proxy. See [docs/INTEROP.md](../docs/INTEROP.md). |
| [build-pea-core-apple.sh](build-pea-core-apple.sh) | macOS | Builds pea-core for iOS (device + simulator) and macOS (arm64 + x64). Use the resulting static libs in Xcode. See [pea-ios/README.md](../pea-ios/README.md) and [pea-macos/README.md](../pea-macos/README.md). |

Requirements: Rust (and for the Linux script, a built `pea-linux` binary). The Apple script must be run on macOS.
