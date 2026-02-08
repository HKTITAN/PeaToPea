# Changelog

All notable changes to the PeaPod project are documented here. Format: version (or date), then added/changed/fixed per component.

## [Unreleased]

- **Documentation:** QUALITY.md — how to measure metrics (throughput, pod formation, linear scaling, battery), optional coverage (cargo-tarpaulin), store listing/privacy policy guidance. RELEASE.md — full pod test step, .deb test step. INTEROP.md — two-process smoke script; optional CI job. iOS/macOS READMEs — next steps for Xcode project. CONTRIBUTING — optional interop script. .tasks README — remaining work summary.
- (Ongoing work: see [.tasks/](.tasks/README.md).)

## 0.1.0 (initial)

- **pea-core:** Protocol logic: identity, wire encoding, chunking, scheduler, integrity, host-driven API; C FFI for Android/iOS.
- **pea-windows:** Proxy, discovery, transport, system proxy, tray.
- **pea-android:** VPNService, JNI, discovery, transport, main screen, settings, first-run, battery handling.
- **pea-linux:** Daemon: proxy, discovery, transport; config file and env; systemd user unit; SIGTERM shutdown; docs.
- **Protocol:** Wire format and discovery specified in [docs/PROTOCOL.md](docs/PROTOCOL.md); version 1.
- **Documentation:** README, ARCHITECTURE.md, PROTOCOL.md, API.md, TROUBLESHOOTING.md, CONTRIBUTING.md.

---

When releasing new protocol or app versions: update this file and tag the release. When bumping protocol version: update PROTOCOL.md, pea-core `PROTOCOL_VERSION`, and all clients; document here.
