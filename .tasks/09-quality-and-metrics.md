# 09 – Quality and Metrics

PRD success metrics, edge-case handling, and risk mitigations. Verify during and after implementation.

## 1. PRD success metrics

- [ ] **1.1** Throughput improvement
  - [ ] 1.1.1 Define test: single large HTTP range download (e.g. 100 MB) with one peer; measure time with PeaPod on vs off
  - [x] 1.1.2 Target: measurable aggregate throughput improvement (e.g. document "up to N× with 2 devices" in README or report) — docs/QUALITY.md
  - [ ] 1.1.3 Optional: run in CI or release process; log result
- [ ] **1.2** Time-to-download
  - [ ] 1.2.1 Same as 1.1: reduced time-to-download for large files when pod has peers
  - [x] 1.2.2 Document expected range (e.g. "typically 1.5–2× faster with 2 devices on same LAN") — docs/QUALITY.md
- [ ] **1.3** Upload time
  - [ ] 1.3.1 Test: large upload with pod; measure time with PeaPod on vs off
  - [ ] 1.3.2 Target: reduced upload time for large transfers when server supports (e.g. multipart or range put)
  - [x] 1.3.3 Document server compatibility limits (upload acceleration may require server support) — TROUBLESHOOTING
- [ ] **1.4** Pod formation time
  - [ ] 1.4.1 Measure: time from "enable" on two devices to "both show 1 peer"
  - [ ] 1.4.2 Target: stable pod formation &lt; 5 seconds (per PRD)
  - [ ] 1.4.3 Test on at least Windows+Android and one other pair; document result
- [ ] **1.5** Zero application breakage
  - [ ] 1.5.1 Test: browse major sites, stream video (non-DRM), download files, use apps that use HTTP; ensure no breakage
  - [x] 1.5.2 Ineligible flows must fall back to normal path; no modification of response that could break app
  - [x] 1.5.3 Document "zero application breakage" as goal and list tested scenarios (docs/QUALITY.md)
- [ ] **1.6** Minimal idle battery consumption
  - [ ] 1.6.1 On Android and iOS: measure battery drain over 24 h idle (PeaPod on, no active transfer) vs PeaPod off
  - [ ] 1.6.2 Target: minimal idle impact; document threshold (e.g. &lt; 1% additional per hour) or qualitative "low"
  - [ ] 1.6.3 Implement low-power behavior (throttle beacon, reduce participation when low battery) per 03-android and 05-ios

## 2. Edge case handling (from PRD)

- [x] **2.1** Device leaves mid-transfer
  - [x] 2.1.1 Implement: heartbeat timeout or leave message → core marks peer left → scheduler redistributes its chunks to remaining peers
  - [ ] 2.1.2 Verify in 01-pea-core tests and in manual test: kill one peer during download; transfer completes
  - [x] 2.1.3 Document in 08: "Chunks are reassigned automatically when a device leaves"
- [x] **2.2** Slow peer
  - [x] 2.2.1 Scheduler takes per-peer metrics (bandwidth/latency); reduce allocation weight for slow peer
  - [x] 2.2.2 Implement: track response time per peer; assign fewer chunks to slow peer in next assignment
  - [ ] 2.2.3 Optional: timeout per chunk request; reassign if peer is too slow
- [x] **2.3** Malicious peer (integrity failure)
  - [x] 2.3.1 On chunk hash mismatch: mark chunk failed; do not use that chunk; request from another peer or self
  - [ ] 2.3.2 Optional: isolate peer after N integrity failures (stop assigning chunks); document in 08
  - [x] 2.3.3 No plaintext inspection; integrity is cryptographic (hash per chunk)
- [x] **2.4** No peers available
  - [x] 2.4.1 Core returns "fallback"; host forwards request normally (no acceleration)
  - [x] 2.4.2 UI shows "Pod: 0 devices" or "No peers nearby"; PeaPod remains idle
  - [ ] 2.4.3 Verify: enable PeaPod with no other device; browsing and downloads work as without PeaPod

## 3. Risk mitigations (from PRD)

- [x] **3.1** OS-level integration complexity
  - [x] 3.1.1 Use proxy on Windows/Linux and VPNService on Android for v1; avoid kernel drivers
  - [x] 3.1.2 Document WinDivert/WFP and netfilter as optional next steps for full transparency
- [x] **3.2** Encrypted streaming / DRM compatibility
  - [x] 3.2.1 Do not accelerate flows that cannot be range-requested or chunked; mark ineligible and fall back
  - [x] 3.2.2 Document: streaming DRM and some HTTPS flows may not be accelerated
- [x] **3.3** CDN anti-abuse throttling
  - [x] 3.3.1 Document: requests from multiple IPs (each device) may trigger CDN throttling; optional adaptive chunk size later
  - [x] 3.3.2 Do not spoof single IP; each device uses its own WAN IP
- [x] **3.4** Upload server validation
  - [x] 3.4.1 Document: upload acceleration may require server support (multipart upload, range PUT, or similar)
  - [x] 3.4.2 Fall back to single-device upload when server does not support or when uncertain
- [x] **3.5** Battery on mobile
  - [x] 3.5.1 Implement: low-battery mode reduces or pauses participation (03-android, 05-ios)
  - [ ] 3.5.2 Measure idle battery (1.6) and document
- [x] **3.6** Security of open local pods
  - [x] 3.6.1 No real-world identity; device ID = hash of public key only
  - [ ] 3.6.2 Optional future: "accept new device" prompt in UI; document in 08 as future consideration
  - [x] 3.6.3 End-to-end encryption within pod; chunk-level hashing (PRD)

## 4. Performance and robustness

- [ ] **4.1** Linear scaling (PRD)
  - [ ] 4.1.1 Test: 2, 3, 4 devices in pod; same large download; throughput should scale roughly linearly (within LAN limits)
  - [ ] 4.1.2 Document or note LAN bandwidth as limit (e.g. total throughput capped by slowest link)
- [x] **4.2** Graceful degradation
  - [x] 4.2.1 When peers disconnect: complete in-flight chunks from remaining peers or self; no crash
  - [x] 4.2.2 When all peers leave: fall back to normal path for remaining chunks
  - [ ] 4.2.3 Verify in integration test (01) and manual multi-device test
- [x] **4.3** Low overhead when idle
  - [x] 4.3.1 When no active transfer: minimal CPU (beacon interval, heartbeat only)
  - [x] 4.3.2 No busy-wait; use timers or event-driven loop
  - [ ] 4.3.3 Optional: measure CPU % idle on each platform; document

## 5. Privacy and security (PRD)

- [x] **5.1** End-to-end encryption
  - [x] 5.1.1 All chunk and control traffic between peers encrypted (core implements; all platforms use)
  - [x] 5.1.2 No plaintext chunk data on wire; verify in protocol spec (08) and implementation
- [x] **5.2** Chunk-level hashing
  - [x] 5.2.1 Every chunk has hash; verified on receive; reject and reassign on failure
  - [x] 5.2.2 Implemented in 01-pea-core; used by all platforms
- [x] **5.3** Local-only data sharing
  - [x] 5.3.1 Chunks only between local peers (same LAN); no central server for data
  - [x] 5.3.2 Document in README and privacy policy (if any): "Data stays on your local network"
- [x] **5.4** No centralized logging
  - [x] 5.4.1 Do not send logs or telemetry to central server (per PRD)
  - [x] 5.4.2 Optional: local logging only; document in 08
  - [ ] 5.4.3 Store listing / privacy policy: state no collection of user data

## 6. Test coverage and CI

- [x] **6.1** pea-core
  - [x] 6.1.1 Unit tests for identity, protocol, chunk manager, scheduler, integrity (01-pea-core)
  - [x] 6.1.2 Integration tests with mock host (01); run in CI on every PR
  - [ ] 6.1.3 Optional: coverage report (e.g. cargo tarpaulin); set minimum threshold
- [x] **6.2** Per-implementation smoke tests
  - [x] 6.2.1 Windows: build and run; enable; verify proxy and discovery (optional in CI) — CI builds
  - [x] 6.2.2 Android: build APK; install on emulator; enable VPN (optional in CI) — CI builds APK
  - [x] 6.2.3 Linux: build and run (CI)
  - [x] 6.2.4 iOS/macOS: build (CI if macOS runner available)
- [ ] **6.3** Interop
  - [ ] 6.3.1 Cross-platform interop tests (07); document manual test results and, if any, automated
  - [ ] 6.3.2 Full pod test (five device types) before release; document in 08

## 7. Release checklist

- [ ] **7.1** Pre-release
  - [ ] 7.1.1 All implementations build and run; no known regressions
  - [ ] 7.1.2 Protocol version and PROTOCOL.md up to date
  - [ ] 7.1.3 CHANGELOG updated
  - [ ] 7.1.4 Success metrics (1.1–1.6) measured or documented; edge cases (2.x) verified
  - [ ] 7.1.5 Privacy/security (5.x) and risk mitigations (3.x) in place and documented
- [ ] **7.2** Release
  - [ ] 7.2.1 Tag version in git; create GitHub (or other) release
  - [ ] 7.2.2 Attach artifacts: Windows installer, Android APK/AAB, Linux binary/.deb, iOS IPA (or TestFlight), macOS .app/DMG
  - [ ] 7.2.3 Release notes: link to CHANGELOG; list platforms and known limitations
