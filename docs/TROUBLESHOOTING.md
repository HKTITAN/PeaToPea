# Troubleshooting and FAQ

## Common issues

### No peers discovered

- **Firewall:** Allow inbound UDP on the discovery port (e.g. 45678) and TCP on the local transport port (e.g. 45679). On Linux see [pea-linux/README.md](../pea-linux/README.md#firewall); on Windows ensure the app is allowed in Windows Defender Firewall; on Android/iOS grant local network permission.
- **Same subnet:** Peers must be on the same LAN (multicast TTL is 1). Different VLANs or guest networks usually cannot see each other.
- **Multicast/broadcast:** Some routers or corporate networks block multicast. Try link-local broadcast if your platform supports it.
- **Mobile:** On Android and iOS, enable "Local network" (or similar) when the app prompts; without it, discovery may not work.

### Transfer not accelerated

- **Eligibility:** Only HTTP GET with a **Range** header is accelerated in the current implementation. Full-file downloads without range, or non-HTTP traffic, fall back to normal forwarding.
- **No peers:** If the pod has no other devices (or they are unreachable), the core returns Fallback and the request is served normally.
- **DRM / special content:** Some streams or apps use non-range requests or encryption; they are not accelerated.

### App broken or not loading

- Ineligible flows should never be accelerated; the core returns **Fallback** and the host forwards the request to the origin. If something breaks, it may be a bug in the host (e.g. wrong eligibility check or forwarding). Report with platform, app name, and URL (if possible) in the project repo.

---

## FAQ

### Does PeaPod replace my ISP?

No. PeaPod uses your existing internet connections (WiFi, cellular, ethernet) and pools them. It does not provide internet by itself.

### Is my data sent to other devices?

Only what’s needed for acceleration: chunk metadata (e.g. range, transfer id) and **encrypted** chunk payloads between devices in your pod. There is no central server; discovery is local (multicast/broadcast on the LAN).

### What if I don’t trust a peer?

Each chunk is integrity-checked (hash). If a peer sends bad data, the chunk is rejected and can be reassigned. Repeated failures can isolate that peer. Future versions may add explicit “accept device” or trust lists.

### Where are the project goals and non-goals?

See the [root README](../README.md): PeaPod is a protocol that lets nearby devices form an encrypted swarm and pool bandwidth; it does not replace your ISP or change servers.
