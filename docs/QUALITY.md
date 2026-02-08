# Quality, edge cases, and risk mitigations

This page summarizes PRD success metrics, edge-case handling, and risk mitigations. See [.tasks/09-quality-and-metrics.md](../.tasks/09-quality-and-metrics.md) for the full checklist.

## Edge case handling

- **Device leaves mid-transfer:** Heartbeat timeout or leave triggers `on_peer_left`; core redistributes that peer’s chunks to remaining peers (or self). Transfer completes without crash. Documented in [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
- **Slow peer:** Scheduler supports per-peer metrics (bandwidth); fewer chunks can be assigned to slower peers. Optional: per-chunk timeout and reassign (future).
- **Malicious peer (integrity failure):** Chunk hash mismatch → chunk rejected and reassigned. No plaintext inspection; integrity is per-chunk cryptographic hash (pea-core). Optional: isolate peer after N failures (future).
- **No peers:** Core returns Fallback; host forwards normally. UI shows “Pod: 0 devices”. Zero application breakage.

## Risk mitigations

- **OS integration (v1):** Proxy on Windows/Linux, VPNService on Android. No kernel drivers. WinDivert/netfilter documented as optional next steps (e.g. [pea-linux/README.md](../pea-linux/README.md)).
- **DRM / encrypted streaming:** Ineligible flows (no range, or unsupported) fall back; no modification of response. Documented in [TROUBLESHOOTING.md](TROUBLESHOOTING.md).
- **CDN throttling:** Multiple devices = multiple IPs; some CDNs may throttle. We do not spoof IPs. Documented in TROUBLESHOOTING.
- **Upload:** Server may need to support multipart or range PUT for upload acceleration; fall back when uncertain. Documented in TROUBLESHOOTING.
- **Battery (mobile):** Android implements low-battery throttling (beacon interval, reduce participation); see 03-android. Idle battery target: minimal; measure and document per 09.
- **Security:** No real-world identity (device ID = derived from public key). E2E encryption between peers; chunk-level hashing. No central server for data. Optional future: “accept new device” in UI.

## Privacy and security (PRD)

- **E2E encryption:** All chunk and control traffic between peers is encrypted (pea-core; all platforms use same wire format).
- **Chunk-level hashing:** Every chunk has a hash; verified on receive; reject and reassign on failure (pea-core).
- **Local-only:** Chunks only between local peers (same LAN); no central server for data.
- **No centralized logging:** No logs or telemetry sent to a central server; local logging only.

## Success metrics (PRD)

Defined in 09; to be measured and documented for release:

- **Throughput / time-to-download:** Target: measurable improvement with 2+ devices (e.g. 1.5–2× with 2 devices on same LAN). Test: large HTTP range download with PeaPod on vs off.
- **Pod formation time:** Target &lt; 5 s from enable on two devices to both showing 1 peer. To be measured on Windows+Android and other pairs.
- **Zero application breakage:** Ineligible flows must fall back to the normal path; no modification of response that could break apps. Goal: browse major sites, stream video (non-DRM), download files—no breakage. Implementations only accelerate eligible (e.g. HTTP GET with Range) and forward the rest.
- **Minimal idle battery (mobile):** Low-power behavior implemented on Android (03); measure and document idle drain. iOS when implemented (05).

### How to measure (09 §1)

- **Throughput (1.1) / time-to-download (1.2):** On two devices on the same LAN, set system proxy to the PeaPod proxy on each; enable PeaPod on both. Download a large file (e.g. 100 MB) via HTTP range (e.g. `curl -O` or a test URL). Note time with PeaPod on. Disable PeaPod on one device (or both) and repeat; compare times. Optional: run in CI using a local HTTP server and scripted curl.
- **Pod formation (1.4):** On two devices (e.g. Windows + Android), enable PeaPod on device A, then on device B; note the time until both UIs show “1 peer” (or equivalent). Target &lt; 5 s. Repeat for other pairs (e.g. Linux + Windows) and document.
- **Zero breakage (1.5):** Manual test: with PeaPod enabled and proxy set, browse major sites, stream non-DRM video, download files, use apps that use HTTP; confirm no breakage. Document scenarios tested before release.
- **Idle battery (1.6):** On Android (and iOS when available): leave PeaPod on, no active transfer, for 24 h; compare battery drain vs PeaPod off. Document threshold or “low impact” in release notes.
- **Linear scaling (4.1):** Same large HTTP range download with 2, then 3, then 4 devices in the pod (same LAN). Throughput should scale roughly linearly with device count, within LAN limits. Document that total throughput is capped by the slowest link (WAN or LAN).

## Testing and CI

- **pea-core:** Unit tests in identity, wire, protocol, chunk, scheduler, integrity, core (run: `cargo test -p pea-core`). CI runs build, test, fmt, clippy, audit on every push/PR.
- **Per-platform builds (CI):** pea-core (Linux); pea-windows (Windows); pea-linux (Linux); pea-android (Android NDK + Gradle assembleDebug); pea-ios and pea-macos (Swift build on macOS runner).
- **Interop / manual:** Cross-platform pod tests and full multi-device runs are manual or run in release process; document results before release (09 §6.3, §7). Test matrix and results: [INTEROP.md](INTEROP.md).
- **Optional coverage (09 §6.1.3):** To generate a coverage report for pea-core, install [cargo-tarpaulin](https://github.com/codecov/cargo-tarpaulin) and run e.g. `cargo tarpaulin -p pea-core --out Html` from the repo root. Optionally add a CI job or enforce a minimum threshold; not required for release.

**Release process:** See [RELEASE.md](RELEASE.md) for the pre-release and release checklist.
