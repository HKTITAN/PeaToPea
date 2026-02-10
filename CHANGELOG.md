# Changelog

All notable changes to the PeaPod project are documented here. Format: version (or date), then added/changed/fixed per component.

## [Unreleased]

### Added
- **Install scripts:** Interactive one-line installers for Linux/macOS (`install.sh`) and Windows (`install.ps1`) with disclaimers, confirmation prompts, service setup, and `--uninstall` support.
- **Makefile:** Standard build/test/lint/install/uninstall targets (`make help` for the full list).
- **pea-core README:** Added `pea-core/README.md` — API overview, build/test instructions, C FFI, cross-compilation.

### Fixed
- **pea-core:** Fixed compilation errors — added `Debug` derives, fixed ChaCha20 nonce types, added `from_bytes()` constructors for `PublicKey`/`DeviceId`, fixed missing function arguments.
- **pea-linux:** Fixed all 16 clippy warnings — `io_other_error`, `while_let_loop`, `collapsible_match`, `single_match`, `question_mark`, `type_complexity`, `unwrap_or_default`, `too_many_arguments`, `dead_code`.
- **pea-windows:** Fixed all 33 clippy warnings — same categories as pea-linux plus `async fn` syntax simplification.
- **CI:** Fixed `dtolnay/rust-action@stable` → `dtolnay/rust-toolchain@stable` (correct action name).

### Changed
- **Documentation:** Updated README with install section and Makefile usage.
- **pea-core:** cbindgen.toml for C header generation (iOS/macOS); CI step generates and verifies `pea_core.h`.
- **Documentation:** QUALITY.md, RELEASE.md, INTEROP.md, iOS/macOS READMEs, CONTRIBUTING, scripts/README.md — see previous entries.

(Ongoing work: see [.tasks/](.tasks/README.md).)

## 0.1.0 (2026-02-10)

- **pea-core:** Protocol logic: identity, wire encoding, chunking, scheduler, integrity, host-driven API; C FFI for Android/iOS.
- **pea-windows:** Proxy, discovery, transport, system proxy, tray.
- **pea-android:** VPNService, JNI, discovery, transport, main screen, settings, first-run, battery handling.
- **pea-linux:** Daemon: proxy, discovery, transport; config file and env; systemd user unit; SIGTERM shutdown; docs.
- **Protocol:** Wire format and discovery specified in [docs/PROTOCOL.md](docs/PROTOCOL.md); version 1.
- **Documentation:** README, ARCHITECTURE.md, PROTOCOL.md, API.md, TROUBLESHOOTING.md, CONTRIBUTING.md.

---

When releasing new protocol or app versions: update this file and tag the release. When bumping protocol version: update PROTOCOL.md, pea-core `PROTOCOL_VERSION`, and all clients; document here.
