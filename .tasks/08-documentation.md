# 08 – Documentation

Architecture, PeaPod protocol spec, and per-platform build/run instructions for each implementation. Keep in sync with implementation.

## 1. Root and overview

- [x] **1.1** Root README
  - [x] 1.1.1 Project name (PeaPod) and one-line description
  - [x] 1.1.2 Link to root README and to .tasks/README.md
  - [x] 1.1.3 High-level architecture: shared protocol core + implementations per OS; link to architecture doc
  - [x] 1.1.4 List implementations: Windows, Android, Linux, iOS, macOS; link to each implementation's build/run section
  - [x] 1.1.5 License and contribution (optional)
- [x] **1.2** CONTRIBUTING (optional)
  - [x] 1.2.1 How to build pea-core and run tests
  - [x] 1.2.2 Branching and PR process (if applicable)
  - [x] 1.2.3 Link to .tasks for task breakdown

## 2. Architecture document

- [x] **2.1** Architecture overview
  - [x] 2.1.1 Diagram: App → PeaPod layer (intercept, scheduler, chunk manager, local transport, integrity) → platform (Win/Android/Linux/iOS/macOS)
  - [x] 2.1.2 Explain: "above IP, below apps"; traffic intercepted and accelerated when eligible; no server changes
  - [x] 2.1.3 List core components: Discovery, Identity & Encryption, Distributed Scheduler, Chunk Manager, Local Transport, Integrity Verification, Failure Recovery (per PRD)
- [x] **2.2** Data flow
  - [x] 2.2.1 Download path: app request → intercept → core (chunk plan) → WAN (self) + peers (local) → chunks → core (reassemble) → app
  - [x] 2.2.2 Upload path: app upload → intercept → core (split) → assign to self + peers → peers upload via WAN → core (verify) → complete
  - [x] 2.2.3 Discovery: beacon → peer list → core; local transport: TCP between peers, encrypted
- [x] **2.3** Host-driven core
  - [x] 2.3.1 Explain: core has no I/O; host (each implementation) does sockets, discovery, proxy/VPN; host calls core with events and receives actions
  - [x] 2.3.2 Link to pea-core API (or list main entry points) — docs/API.md

## 3. Protocol specification

- [x] **3.1** Create PROTOCOL.md (or docs/PROTOCOL.md)
  - [x] 3.1.1 Wire encoding: bincode or custom; endianness; version field
  - [x] 3.1.2 Message types and fields (beacon, response, join, leave, heartbeat, chunk request, chunk data, NACK)
  - [x] 3.1.3 Framing: length-prefix or delimiter; max message size if any
  - [x] 3.1.4 Discovery: multicast group and port (or broadcast); beacon format and interval
  - [x] 3.1.5 Local transport: TCP; handshake (version, device_id, public_key); session key derivation; AEAD for all subsequent messages
  - [x] 3.1.6 Chunk message format: chunk_id, range, hash, payload
  - [x] 3.1.7 Versioning: major/minor; compatibility and rejection rules
- [x] **3.2** Reference from 07 and 01
  - [x] 3.2.1 In 07-protocol-and-interop: "See PROTOCOL.md"
  - [x] 3.2.2 In pea-core README: "Wire format is specified in PROTOCOL.md"

## 4. Build and run per platform

- [x] **4.1** Windows
  - [x] 4.1.1 Prerequisites: Rust, Windows 10/11; optional WinDivert if using that path
  - [x] 4.1.2 Build: `cargo build --release` in pea-windows (or from root workspace)
  - [x] 4.1.3 Run: `pea-windows.exe` or via installer; enable from tray
  - [x] 4.1.4 Config: system proxy set when enabled; optional config file path if added
- [x] **4.2** Android
  - [x] 4.2.1 Prerequisites: Android Studio, NDK, Rust targets for Android
  - [x] 4.2.2 Build: open pea-android in Android Studio; build project (Rust lib built via gradle NDK or external script)
  - [x] 4.2.3 Run: install debug APK on device/emulator; enable PeaPod in app; grant VPN and local network
  - [x] 4.2.4 Permissions: list required permissions and what they're for
- [x] **4.3** Linux
  - [x] 4.3.1 Prerequisites: Rust, systemd (user or system)
  - [x] 4.3.2 Build: `cargo build --release` in pea-linux
  - [x] 4.3.3 Run: `./pea-linux` or `systemctl --user start peapod`; set HTTP_PROXY/HTTPS_PROXY if using proxy
  - [x] 4.3.4 Config: path to config file (e.g. ~/.config/peapod/config.toml); ports and options
  - [x] 4.3.5 Packaging: link to .deb or Flatpak if available (pea-linux README: cargo-deb .deb)
- [ ] **4.4** iOS (when pea-ios implementation exists: document in pea-ios/README.md)
  - [ ] 4.4.1 Prerequisites: Xcode, Apple Developer account, Rust toolchain for iOS
  - [ ] 4.4.2 Build: open pea-ios in Xcode; select target device/simulator; build (Rust built via script or Xcode build phase)
  - [ ] 4.4.3 Run: run on device/simulator; enable VPN in app; allow VPN and local network
  - [ ] 4.4.4 Distribution: TestFlight or App Store; note VPN/extension review
- [ ] **4.5** macOS (when pea-macos implementation exists: document in pea-macos/README.md)
  - [ ] 4.5.1 Prerequisites: Xcode, Rust for macOS (arm64/x86_64)
  - [ ] 4.5.2 Build: open pea-macos in Xcode; build
  - [ ] 4.5.3 Run: run app; enable from menu bar; allow extension in System Preferences if prompted
  - [ ] 4.5.4 Distribution: DMG/pkg or Mac App Store; notarization for outside store

## 5. pea-core API (for platform authors)

- [x] **5.1** API overview
  - [x] 5.1.1 List main types: e.g. PeaPodCore, Config
  - [x] 5.1.2 List main methods: init, on_incoming_request, on_peer_joined, on_peer_left, on_message_received, on_chunk_received, tick
  - [x] 5.1.3 Inputs and outputs: what host passes in, what core returns (actions, messages to send, WAN chunk requests)
- [x] **5.2** Rust doc comments
  - [x] 5.2.1 Add rustdoc to public functions and types in pea-core
  - [x] 5.2.2 Generate and publish docs (e.g. `cargo doc --no-deps`); optional link from root README (documented in API.md)
- [x] **5.3** C API (for iOS/macOS)
  - [x] 5.3.1 Document C header (or list functions) for Swift callers: init, feed request, feed peer events, feed message, feed chunk, tick
  - [x] 5.3.2 Data types: how to pass byte arrays, strings; who allocates/frees
  - [x] 5.3.3 Thread safety: single-threaded or allowed threads
- [x] **5.4** JNI API (for Android)
  - [x] 5.4.1 Document JNI function names and signatures (or Kotlin wrapper)
  - [x] 5.4.2 Same logical API as Rust; document parameter and return types (jbyteArray, jstring, etc.)

## 6. Troubleshooting and FAQ

- [x] **6.1** Common issues
  - [x] 6.1.1 "No peers discovered": check firewall, multicast/broadcast, same subnet, local network permission (mobile)
  - [x] 6.1.2 "Transfer not accelerated": eligibility (HTTP range, no DRM); or no peers in pod
  - [x] 6.1.3 "App broken / not loading": ineligible flow was accelerated; fallback should prevent; document how to report
- [x] **6.2** FAQ
  - [x] 6.2.1 Does PeaPod replace my ISP? No.
  - [x] 6.2.2 Is my data sent to other devices? Only chunk metadata and encrypted chunks for acceleration; no central server.
  - [x] 6.2.3 What if I don't trust a peer? Integrity check fails; peer isolated after failures; optional "accept device" in future.
  - [x] 6.2.4 Link to root README for goals and non-goals

## 7. Changelog and versioning

- [x] **7.1** Changelog
  - [x] 7.1.1 Add CHANGELOG.md (or keep in releases); format: version, date, added/changed/fixed per component
  - [x] 7.1.2 Update when releasing new protocol or app versions
- [x] **7.2** Protocol version
  - [x] 7.2.1 Document current protocol version in PROTOCOL.md and in pea-core (constant or config)
  - [x] 7.2.2 When bumping: update PROTOCOL.md, pea-core, and all clients; document in CHANGELOG
