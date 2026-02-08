# 05 – iOS Protocol Implementation

Implementation of the PeaPod protocol for iOS: Swift app with Network Extension for traffic interception; Rust core via C ABI or XCFramework; discovery and local transport. Depends on 01-pea-core and 07.

## 1. iOS project scaffold

(Next steps summarized in [pea-ios/README.md](../pea-ios/README.md) “Next steps”.)

- [ ] **1.1** Create protocol implementation for iOS (Xcode project)
  - [ ] 1.1.1 Create `pea-ios/` with Xcode; iOS app target (Swift, minimum iOS version e.g. 15)
  - [ ] 1.1.2 Add Network Extension target (Packet Tunnel or App Proxy) to same project
  - [ ] 1.1.3 Configure app groups or shared container if extension and app need to share state
- [ ] **1.2** Rust core for iOS
  - [ ] 1.2.1 Build pea-core for `aarch64-apple-ios` (device) and `x86_64-apple-ios` (simulator); optional: `./scripts/build-pea-core-apple.sh` from repo root
  - [ ] 1.2.2 Create C header (or generate via cbindgen) exposing: init, on_request, on_peer_joined, on_peer_left, on_message_received, on_chunk_received, tick (Note: pea-core/cbindgen.toml; run `cbindgen pea-core -o pea_core.h`; CI verifies.)
  - [ ] 1.2.3 Produce static library (.a) or XCFramework containing the Rust lib for both architectures
  - [ ] 1.2.4 Add lib to Xcode: link binary with extension and app targets; add header search path
  - [ ] 1.2.5 Call C API from Swift (Bridging header or module map)
- [ ] **1.3** Capabilities and entitlements
  - [ ] 1.3.1 Enable Network Extensions capability (Packet Tunnel and/or App Proxy) in Apple Developer account and in Xcode
  - [ ] 1.3.2 Add Personal VPN entitlement
  - [ ] 1.3.3 Add local network usage description (Info.plist) for discovery
  - [ ] 1.3.4 Add App Sandbox and any required exceptions (e.g. outgoing network, local network)

## 2. Network Extension (traffic interception)

- [ ] **2.1** Choose extension type
  - [ ] 2.1.1 App Proxy: good for HTTP/HTTPS; can parse and accelerate range requests
  - [ ] 2.1.2 Packet Tunnel: full control; implement custom IP layer; more work
  - [ ] 2.1.3 Implement App Proxy first (or Packet Tunnel if scope requires it)
- [ ] **2.2** Extension lifecycle
  - [ ] 2.2.1 Implement NEPacketTunnelProvider or NEAppProxyProvider subclass
  - [ ] 2.2.2 In start(completionHandler): set up tunnel (IP settings for Packet Tunnel) or start handling flows (App Proxy)
  - [ ] 2.2.3 Handle system stop and reconnection
- [ ] **2.3** App Proxy: flow handling
  - [ ] 2.3.1 Implement handleNewFlow(_ flow: NEAppProxyFlow) -> Bool
  - [ ] 2.3.2 For TCP flows: read request (e.g. HTTP); parse URL and headers; check eligibility (HTTP, range)
  - [ ] 2.3.3 If eligible: pass metadata to Rust core; get chunk plan; fetch chunks (self + peers via local transport); reassemble; write response back to flow
  - [ ] 2.3.4 If ineligible: open connection to real destination and relay (full duplex) so app sees normal traffic
- [ ] **2.4** Packet Tunnel (if used)
  - [ ] 2.4.1 Create TUN interface; read packets, parse IP/TCP; dispatch to local proxy or core
  - [ ] 2.4.2 Write response packets back to TUN
  - [ ] 2.4.3 More complex; document as alternative to App Proxy

## 3. Discovery on iOS

- [ ] **3.1** LAN discovery
  - [ ] 3.1.1 Request local network permission (iOS 14+); show usage description
  - [ ] 3.1.2 Create UDP socket (same multicast group/port as 07); send beacon; receive beacons from other devices
  - [ ] 3.1.3 Parse beacon; maintain peer list; call Rust core on_peer_joined / on_peer_left
  - [ ] 3.1.4 Advertise own IP and port (extension may have different network context; document and test)
- [ ] **3.2** Optional: BLE or MultipeerConnectivity
  - [ ] 3.2.1 Use for "who's nearby" before WiFi; then use LAN for actual chunk transfer
  - [ ] 3.2.2 Document as optional enhancement

## 4. Local transport (iOS)

- [ ] **4.1** TCP in extension
  - [ ] 4.1.1 Network Extension can create TCP connections to peers (same network)
  - [ ] 4.1.2 Run TCP server (listen) and client (connect to peers); frame protocol messages
  - [ ] 4.1.3 Encrypt/decrypt via Rust core; same wire format as other platforms
- [ ] **4.2** Sandbox and capabilities
  - [ ] 4.2.1 Ensure extension has "outgoing network" and "incoming network" or equivalent so it can bind and connect on LAN
  - [ ] 4.2.2 Test that extension can reach peers on same WiFi

## 5. Integration with Rust core (Swift ↔ C)

- [ ] **5.1** Data passing
  - [ ] 5.1.1 Pass URL as C string or byte pointer; pass range as (start, end); get back action enum (accelerate / fallback)
  - [ ] 5.1.2 For chunk plan: core returns list of (chunk_id, peer_id or self); Swift executes WAN request for self, sends chunk request to peers
  - [ ] 5.1.3 Chunk data received: pass to core; get reassembled segment; write to flow
  - [ ] 5.1.4 Peer events and tick: same as other platforms; marshal results (e.g. list of messages to send) back to Swift
- [ ] **5.2** Threading
  - [ ] 5.2.1 Call Rust from a single queue or main thread of extension to avoid races; or ensure core is thread-safe
  - [ ] 5.2.2 Avoid blocking main thread; use background queue for core calls if needed

## 6. Main app UI

- [ ] **6.1** Enable/disable
  - [ ] 6.1.1 Main screen: toggle "Enable PeaPod" (starts VPN/Network Extension via NEVPNManager)
  - [ ] 6.1.2 System VPN consent appears on first enable; user must allow
  - [ ] 6.1.3 Show status: "Connected" and "Pod: N devices" when extension is running and has peers
- [ ] **6.2** Pod status
  - [ ] 6.2.1 Display list of peers (device IDs or short hashes); update when extension reports (via app group or IPC)
  - [ ] 6.2.2 When no peers: "No peers nearby" or "Pod: 0 devices"
- [ ] **6.3** Settings
  - [ ] 6.3.1 Optional: battery saver (reduce participation when low power); optional widget
  - [ ] 6.3.2 Optional: Home screen widget showing pod size

## 7. App ↔ Extension communication

- [ ] **7.1** Shared state
  - [ ] 7.1.1 Use App Group container to share: enabled flag, peer list (extension writes, app reads for UI)
  - [ ] 7.1.2 Or use Darwin notifications / IPC to push "pod updated" from extension to app
- [ ] **7.2** VPN configuration
  - [ ] 7.2.1 Use NEVPNManager to install and enable the Network Extension; user sees "PeaPod" in Settings > VPN
  - [ ] 7.2.2 On disable: turn off VPN configuration so extension stops

## 8. Battery and performance (PRD)

- [ ] **8.1** Low power
  - [ ] 8.1.1 When device is low power mode: optionally throttle or pause participation (reduce chunk accepts, slower beacon)
  - [ ] 8.1.2 Minimize idle wakeups in extension
- [ ] **8.2** Idle
  - [ ] 8.2.1 When no transfer: extension should not use significant CPU; beacon interval reasonable

## 9. Build and distribution

- [ ] **9.1** Debug
  - [ ] 9.1.1 Run on simulator (x86_64) and device (arm64); test extension in debug
  - [ ] 9.1.2 Use Xcode scheme for app + extension together
- [ ] **9.2** Archive and App Store
  - [ ] 9.2.1 Archive app (includes extension); validate and upload to App Store Connect
  - [ ] 9.2.2 Fill store listing: description, privacy policy (no centralized logging), VPN disclosure
  - [ ] 9.2.3 Apple may review VPN/Network Extension; ensure compliance with App Store guidelines
- [ ] **9.3** TestFlight
  - [ ] 9.3.1 Use TestFlight for beta; test on multiple devices and iOS versions

## Notes

- Scaffold support: scripts/build-pea-core-apple.sh builds pea-core for iOS/macOS; pea-core/cbindgen.toml + `cbindgen pea-core -o pea_core.h` for C header (CI verifies); pea-ios/README.md “Next steps” for Xcode project creation.
