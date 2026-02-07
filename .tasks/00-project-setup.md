# 00 – Project Setup

Repo structure, tooling, and CI. Complete before building protocol implementations.

## 1. Monorepo structure

- [x] **1.1** Create root directory layout
  - [x] 1.1.1 Add `pea-core/` directory (Rust crate: protocol reference implementation)
  - [x] 1.1.2 Add `pea-windows/` directory (PeaPod protocol implementation for Windows)
  - [x] 1.1.3 Add `pea-android/` directory (PeaPod protocol implementation for Android)
  - [x] 1.1.4 Add `pea-linux/` directory (PeaPod protocol implementation for Linux)
  - [x] 1.1.5 Add `pea-ios/` directory (PeaPod protocol implementation for iOS)
  - [x] 1.1.6 Add `pea-macos/` directory (PeaPod protocol implementation for macOS)
  - [x] 1.1.7 Add root `Cargo.toml` workspace if using Rust workspace
- [x] **1.2** Root config files
  - [x] 1.2.1 Add `.gitignore` (target/, build/, .idea/, *.iml, etc.)
  - [x] 1.2.2 Add root `README.md` with project name (PeaPod: Project PeaToPea) and link to .tasks
  - [x] 1.2.3 Add `.editorconfig` or format config if desired

## 2. Version control and branching

- [x] **2.1** Initialize Git (if not already)
  - [x] 2.1.1 `git init`
  - [x] 2.1.2 Create initial commit with README and .tasks
- [x] **2.2** Branching strategy
  - [x] 2.2.1 Document main/develop (or trunk) in README or CONTRIBUTING
  - [x] 2.2.2 Define feature branch naming (e.g. `feature/pea-core-identity`)

## 3. Rust toolchain (for pea-core and desktop clients)

- [x] **3.1** Rust setup
  - [x] 3.1.1 Ensure `rustup` and stable toolchain available
  - [x] 3.1.2 Add `rust-toolchain.toml` or document required version in README
  - [x] 3.1.3 Add targets if needed: `aarch64-apple-ios`, `x86_64-apple-ios`, `aarch64-linux-android`, etc.
- [x] **3.2** Cargo workspace (optional)
  - [x] 3.2.1 Create root `Cargo.toml` with `[workspace]` and members: `pea-core`, `pea-windows`, `pea-linux`
  - [x] 3.2.2 Ensure pea-core can be built alone and as dependency

## 4. Android toolchain

- [ ] **4.1** Android environment
  - [ ] 4.1.1 Install Android Studio or SDK + NDK
  - [ ] 4.1.2 Document minimum SDK version (e.g. 24) and target SDK
  - [ ] 4.1.3 Add `pea-android/` as Android project (Gradle/Kotlin)
- [ ] **4.2** NDK for Rust
  - [ ] 4.2.1 Install NDK and document path
  - [ ] 4.2.2 Add build config to link Rust static lib into Android app

## 5. iOS/macOS toolchain

- [ ] **5.1** Apple development
  - [ ] 5.1.1 Document Xcode version requirement
  - [ ] 5.1.2 Create placeholder or scaffold for `pea-ios/` (Xcode project or Swift Package)
  - [ ] 5.1.3 Create placeholder or scaffold for `pea-macos/` (Xcode project)
- [ ] **5.2** Rust for Apple
  - [ ] 5.2.1 Add Rust targets: `aarch64-apple-ios`, `x86_64-apple-ios`, `aarch64-apple-darwin`, `x86_64-apple-darwin`
  - [ ] 5.2.2 Document how core is built as static lib or XCFramework

## 6. CI (continuous integration)

- [x] **6.1** Core and Windows/Linux CI
  - [x] 6.1.1 Add CI config (e.g. GitHub Actions) to build `pea-core` on push/PR
  - [x] 6.1.2 Run `cargo test` for pea-core
  - [ ] 6.1.3 Build pea-windows on Windows runner (optional)
  - [ ] 6.1.4 Build pea-linux on Linux runner (optional)
- [ ] **6.2** Android CI
  - [ ] 6.2.1 Add job to build pea-android (debug APK)
  - [ ] 6.2.2 Build Rust core for Android targets in CI
- [ ] **6.3** iOS/macOS CI
  - [ ] 6.3.1 Add job to build pea-ios (simulator) if macOS runner available
  - [ ] 6.3.2 Add job to build pea-macos if macOS runner available
- [x] **6.4** Linting and format
  - [x] 6.4.1 Run `cargo fmt -- --check` for Rust
  - [x] 6.4.2 Run `cargo clippy` for pea-core (and other Rust crates)

## 7. Protocol versioning (placeholder)

- [ ] **7.1** Define version scheme
  - [ ] 7.1.1 Document protocol version number (e.g. 1) in repo or pea-core
  - [ ] 7.1.2 Reserve field in wire format for version (implementation in 01-pea-core / 07)

## 8. Dependency and license audit

- [ ] **8.1** Licenses
  - [ ] 8.1.1 Choose license for project (e.g. MIT, Apache-2.0) and add LICENSE file
  - [ ] 8.1.2 Document third-party licenses for Rust crates (cargo license or similar)
- [ ] **8.2** Security
  - [ ] 8.2.1 Add `cargo audit` to CI (optional) for Rust
  - [ ] 8.2.2 Document process for dependency updates
