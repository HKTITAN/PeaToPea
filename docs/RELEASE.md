# Release checklist

Use this checklist when cutting a new release. See [.tasks/09-quality-and-metrics.md](../.tasks/09-quality-and-metrics.md) §7 for the full task list.

## Pre-release

- [ ] **Builds and regressions:** All implementations build and run; no known regressions.
  - `cargo build -p pea-core && cargo test -p pea-core`
  - `cargo build -p pea-windows` (Windows)
  - `cargo build -p pea-linux`
  - pea-android: Gradle assembleDebug (or release)
  - pea-ios / pea-macos: Swift build (when implemented)
- [ ] **Protocol version:** [docs/PROTOCOL.md](PROTOCOL.md) and pea-core `PROTOCOL_VERSION` are in sync. If you bumped the protocol, all clients are updated and documented in CHANGELOG.
- [ ] **CHANGELOG:** [CHANGELOG.md](../CHANGELOG.md) updated with version, date, and added/changed/fixed per component.
- [ ] **Metrics and edge cases:** Success metrics (09 §1) measured or documented; edge cases (09 §2) verified (manual or automated).
- [ ] **Privacy and risks:** Privacy/security (09 §5) and risk mitigations (09 §3) in place and documented (see [QUALITY.md](QUALITY.md)).

## Release

- [ ] **Tag and release:** Tag the version in git (e.g. `v0.2.0`). Create a GitHub (or other) release from the tag.
- [ ] **Artifacts:** Attach build artifacts as needed:
  - Windows: installer (e.g. from pea-windows/installer) or portable binary
  - Android: APK or AAB (debug or signed release)
  - Linux: binary and/or .deb (see [pea-linux README](../pea-linux/README.md)). **Binary:** CI job `pea-linux-release` produces a x86_64 artifact; or build locally with `cargo build -p pea-linux --release`. For aarch64: `cargo build -p pea-linux --release --target aarch64-unknown-linux-gnu`. **.deb:** `cargo install cargo-deb && cargo deb -p pea-linux` from repo root; install with `dpkg -i target/debian/pea-linux_*.deb`.
  - iOS: IPA or TestFlight (when implemented)
  - macOS: .app or DMG (when implemented)
- [ ] **Release notes:** In the release description, link to CHANGELOG and list supported platforms and known limitations.

## After release

- Bump version or add “Unreleased” section in CHANGELOG for the next cycle.
- If protocol version was bumped, ensure PROTOCOL.md and all implementations are updated and documented.
