#!/usr/bin/env bash
# Build pea-core for iOS and macOS (static libs for Xcode).
# Run from repo root on macOS: ./scripts/build-pea-core-apple.sh
# Requires: Rust (rustup), and for iOS simulator aarch64-apple-ios-sim if needed.

set -e
cd "$(dirname "$0")/.."

echo "Adding Rust targets (if needed)..."
rustup target add aarch64-apple-ios x86_64-apple-ios aarch64-apple-darwin x86_64-apple-darwin 2>/dev/null || true

echo "Building pea-core for iOS (device)..."
cargo build -p pea-core --target aarch64-apple-ios --release
echo "Building pea-core for iOS (simulator)..."
cargo build -p pea-core --target x86_64-apple-ios --release
echo "Building pea-core for macOS (Apple Silicon)..."
cargo build -p pea-core --target aarch64-apple-darwin --release
echo "Building pea-core for macOS (Intel)..."
cargo build -p pea-core --target x86_64-apple-darwin --release

echo ""
echo "Static libraries (use in Xcode; link and set header search path to pea-core/src or generate C header with cbindgen):"
echo "  iOS device:    target/aarch64-apple-ios/release/libpea_core.a"
echo "  iOS simulator: target/x86_64-apple-ios/release/libpea_core.a"
echo "  macOS (arm64): target/aarch64-apple-darwin/release/libpea_core.a"
echo "  macOS (x64):   target/x86_64-apple-darwin/release/libpea_core.a"
echo "C FFI: pea-core/src/ffi.rs (generate .h with cbindgen or use from Swift via Bridging header)."
