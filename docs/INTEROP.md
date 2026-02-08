# Cross-platform interop

All implementations (Windows, Android, Linux, iOS, macOS) use the same wire format and discovery ([PROTOCOL.md](PROTOCOL.md)). This page defines the interop test matrix and records results. See [.tasks/07-protocol-and-interop.md](../.tasks/07-protocol-and-interop.md) §5.

## Test matrix

Each pair (or full pod) should be tested on the **same LAN**: enable PeaPod on both devices, confirm they discover each other (e.g. "Pod: 1 device"), then run at least one HTTP range download that uses chunks from both (or from multiple devices in a full pod). Verify reassembly and no breakage.

| Pair / pod | Description | Status |
|------------|-------------|--------|
| **Windows + Android** | Same LAN; discover; form pod; one download with chunk from each; verify reassembly | _To be tested_ |
| **Windows + Linux** | Same | _To be tested_ |
| **Android + Linux** | Same | _To be tested_ |
| **Android + iOS** | Same (when iOS implemented) | _To be tested_ |
| **macOS + iOS** | Same (when both implemented) | _To be tested_ |
| **Linux + macOS** | Same (when macOS implemented) | _To be tested_ |
| **Full pod** | One device of each type (Win, Android, Linux, iOS, macOS) in same pod; one transfer uses chunks from multiple device types; verify no breakage | _To be tested_ |

Update the **Status** column when a pair is tested: e.g. "OK (2025-02)" or "Failed: …" with a short note.

## How to run

1. Put two (or more) devices on the same subnet (e.g. same WiFi).
2. Enable PeaPod on each (system proxy on Windows/Linux; VPN in app on Android; etc.).
3. Confirm discovery: each UI shows at least one peer.
4. On one device, trigger a large HTTP range download (e.g. a file that supports Range). Ensure the request goes through the proxy/VPN.
5. Verify the download completes and the content is correct (e.g. compare hash or size).
6. Optionally: disconnect one peer during a download and confirm the transfer still completes (chunk reassignment).

## Automated interop (optional)

- **Wire roundtrip:** pea-core unit tests (01-pea-core) cover encode/decode for all message types; no cross-language test yet.
- **Two-process smoke (Linux):** From repo root on Linux, run `./scripts/interop-two-linux.sh` after `cargo build -p pea-linux --release`. It starts two pea-linux instances (different proxy/transport ports, same discovery port) and runs one HTTP request through the first proxy. Verifies both start and proxy works; discovery between two processes on the same host may or may not work (multicast loopback). Optional CI job can run this script on the Linux runner.

When more automated tests exist (e.g. cross-platform or full transfer), link them here or from [QUALITY.md](QUALITY.md).
