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

- Throughput improvement and time-to-download (e.g. 1.5–2× with 2 devices).
- Pod formation time (target &lt; 5 s).
- Zero application breakage (ineligible → fallback).
- Minimal idle battery (mobile); low-power behavior implemented on Android.

Tests and CI: pea-core unit and integration tests; per-platform build in CI; interop and manual pod tests before release.
