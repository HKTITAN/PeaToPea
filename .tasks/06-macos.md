# 06 – macOS Protocol Implementation

Implementation of the PeaPod protocol for macOS: Swift app with Network Extension; menu bar; discovery and local transport. Same protocol as iOS so Mac and iPhone can share a pod. Depends on 01-pea-core and 07.

## 1. macOS project scaffold

(Next steps summarized in [pea-macos/README.md](../pea-macos/README.md) “Next steps”.)

- [ ] **1.1** Create protocol implementation for macOS (Xcode project)
  - [ ] 1.1.1 Create `pea-macos/` with Xcode; macOS app target (Swift, AppKit or SwiftUI; minimum macOS e.g. 12)
  - [ ] 1.1.2 Add Network Extension target (Packet Tunnel or App Proxy) for macOS
  - [ ] 1.1.3 Configure app groups if extension and app share state
- [ ] **1.2** Rust core for macOS
  - [ ] 1.2.1 Build pea-core for `aarch64-apple-darwin` (Apple Silicon) and `x86_64-apple-darwin` (Intel); optional: `./scripts/build-pea-core-apple.sh` from repo root
  - [ ] 1.2.2 Same C API as iOS (or shared XCFramework with iOS + macOS slices); C header: `cbindgen pea-core -o pea_core.h` (pea-core has cbindgen.toml)
  - [ ] 1.2.3 Add lib to Xcode; link with app and extension targets
  - [ ] 1.2.4 Call from Swift via bridging header or module
- [ ] **1.3** Entitlements and capabilities
  - [ ] 1.3.1 Enable Network Extensions (Packet Tunnel / App Proxy) in developer account and Xcode
  - [ ] 1.3.2 Add Personal VPN entitlement
  - [ ] 1.3.3 Sandbox: enable outgoing and incoming network; local network if needed for discovery
  - [ ] 1.3.4 Hardened Runtime and notarization for distribution outside App Store

## 2. Network Extension (macOS)

- [ ] **2.1** Extension type
  - [ ] 2.1.1 Use App Proxy for HTTP/HTTPS acceleration (same as iOS) or Packet Tunnel for full transparency
  - [ ] 2.1.2 Implement NEAppProxyProvider (or NEPacketTunnelProvider) subclass for macOS
- [ ] **2.2** Flow handling
  - [ ] 2.2.1 handleNewFlow: parse HTTP request; check eligibility; pass to core; execute chunk plan; reassemble and return; or relay ineligible flows
  - [ ] 2.2.2 Same logic as iOS extension; shared code or duplicate as needed
- [ ] **2.3** System extension (if required)
  - [ ] 2.3.1 On macOS 10.15+, Network Extension runs as system extension; ensure deployment target and signing support it
  - [ ] 2.3.2 User may need to approve extension in System Preferences > Security & Privacy

## 3. Discovery on macOS

- [ ] **3.1** LAN discovery
  - [ ] 3.1.1 UDP multicast/broadcast (same group/port as 07); send beacon; receive beacons
  - [ ] 3.1.2 Parse and maintain peer list; call core on_peer_joined / on_peer_left
  - [ ] 3.1.3 Request local network permission if required (macOS 11+); add usage description
  - [ ] 3.1.4 Advertise own IP and TCP port for local transport
- [ ] **3.2** Firewall
  - [ ] 3.2.1 Document: allow PeaPod in Firewall (System Settings > Network > Firewall) for incoming connections from local network
  - [ ] 3.2.2 Or use multicast/broadcast only and outbound TCP (no inbound) if protocol allows

## 4. Local transport (macOS)

- [ ] **4.1** TCP server and client
  - [ ] 4.1.1 Extension or app creates TCP listen socket and connects to peers (same as other platforms)
  - [ ] 4.1.2 Frame protocol messages; encrypt/decrypt via core; same wire format
- [ ] **4.2** Run in extension vs app
  - [ ] 4.2.1 Prefer running discovery and local transport inside Network Extension so they work when VPN is active; app can show status from shared state
  - [ ] 4.2.2 Ensure extension has network access to LAN (not only tunnel)

## 5. Integration with Rust core

- [ ] **5.1** Same as iOS
  - [ ] 5.1.1 Pass request metadata to core; get chunk plan; execute WAN + peer requests; pass chunk data to core; stream reassembled response
  - [ ] 5.1.2 Peer lifecycle and tick; marshal C structs / byte arrays between Swift and Rust
- [ ] **5.2** Shared core with iOS
  - [ ] 5.2.1 Ideally same XCFramework or static lib used by pea-ios and pea-macos; one protocol, one wire format
  - [ ] 5.2.2 Test: Mac and iPhone on same LAN join same pod and exchange chunks

## 6. Menu bar and UI

- [ ] **6.1** Menu bar app
  - [ ] 6.1.1 Create menu bar (status bar) icon; show "PeaPod" and status
  - [ ] 6.1.2 Menu: Enable / Disable, "Pod: N devices", Settings, Quit
  - [ ] 6.1.3 On Enable: install and activate VPN (NEVPNManager); extension starts
  - [ ] 6.1.4 On Disable: deactivate VPN; extension stops
- [ ] **6.2** Settings window
  - [ ] 6.2.1 Optional settings window: battery saver, optional "Launch at login"
  - [ ] 6.2.2 Show list of peers (device IDs) and link to documentation
- [ ] **6.3** System Settings integration
  - [ ] 6.3.1 PeaPod appears under System Settings > Network > VPN (when enabled)
  - [ ] 6.3.2 Optional: provide a pane or link from System Settings to open PeaPod app (e.g. URL scheme or open application)

## 7. VPN configuration

- [ ] **7.1** NEVPNManager
  - [ ] 7.1.1 Create and install VPN configuration with Network Extension
  - [ ] 7.1.2 User enables in app; system may prompt for permission
  - [ ] 7.1.3 On first launch: guide user to enable in menu bar
- [ ] **7.2** Persistence
  - [ ] 7.2.1 Save "enabled" preference; on next launch, re-enable VPN if user had it on (optional "launch at login")

## 8. Build and distribution

- [ ] **8.1** Debug and run
  - [ ] 8.1.1 Build and run on Apple Silicon and Intel; test extension
  - [ ] 8.1.2 Sign with development certificate for local run
- [ ] **8.2** Release
  - [ ] 8.2.1 Archive app; sign with distribution certificate; notarize for Gatekeeper
  - [ ] 8.2.2 Distribute: .app in DMG or .pkg; or submit to Mac App Store (same VPN/extension review as iOS)
- [ ] **8.3** Auto-update (optional)
  - [ ] 8.3.1 Use Sparkle or similar for outside-App-Store updates; or rely on Mac App Store updates

## 9. Edge cases

- [ ] **9.1** No peers
  - [ ] 9.1.1 Extension runs; traffic passes through without acceleration; show "Pod: 0 devices"
- [ ] **9.2** Sleep / wake
  - [ ] 9.2.1 On wake, rediscover peers and re-establish TCP connections if needed; core may need to redistribute
  - [ ] 9.2.2 Document or handle "disconnect on sleep" for VPN
- [ ] **9.3** Multiple network interfaces
  - [ ] 9.3.1 Prefer same interface for discovery and local transport (e.g. Wi-Fi); document multi-interface behavior
