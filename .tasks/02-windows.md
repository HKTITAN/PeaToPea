# 02 – Windows Protocol Implementation

Implementation of the PeaPod protocol for Windows: background process to discover peers, intercept traffic (proxy or WinDivert), run core, tray and settings. Depends on 01-pea-core and 07 (protocol/discovery spec).

## 1. Project scaffold

- [x] **1.1** Create protocol implementation for Windows (crate or app)
  - [x] 1.1.1 Add `pea-windows/` as Rust binary (e.g. `cargo init --bin pea-windows`) or Tauri app
  - [x] 1.1.2 Add dependency on `pea-core` (path or workspace)
  - [x] 1.1.3 Add Cargo.toml with Windows-only deps (e.g. `winapi` or `windows` crate for tray/proxy)
- [x] **1.2** Build and run
  - [x] 1.2.1 `cargo build` and `cargo run` succeed on Windows
  - [x] 1.2.2 Document how to run from command line for development (pea-windows/README.md)

## 2. Traffic interception (v1: system proxy)

- [x] **2.1** Proxy server
  - [x] 2.1.1 Implement local HTTP/HTTPS proxy server (listen on localhost, e.g. 127.0.0.1:port)
  - [x] 2.1.2 Accept CONNECT for HTTPS; tunnel or parse where possible for range requests (v1: tunnel only)
  - [x] 2.1.3 Accept HTTP requests; parse URL and headers for range/eligibility
  - [x] 2.1.4 For eligible requests: hand off to core (chunking, scheduler); for ineligible: forward directly to target
  - [x] 2.1.5 Implement response path: receive chunks from core (or WAN), reassemble, send back to client (self-assigned chunks via reqwest; peer chunks fallback until §4)
- [ ] **2.2** System proxy configuration
  - [ ] 2.2.1 Read current system proxy (Windows registry or WinINet API)
  - [ ] 2.2.2 Set system proxy to localhost:port when user enables PeaPod
  - [ ] 2.2.3 Restore previous proxy (or clear) when user disables PeaPod
  - [ ] 2.2.4 Handle "no proxy" vs "custom proxy" so we don't overwrite user's choice when off
- [ ] **2.3** Optional: WinDivert path (post-v1)
  - [ ] 2.3.1 Document WinDivert install and license
  - [ ] 2.3.2 Implement packet capture/redirect for TCP (e.g. port 80/443) to local proxy
  - [ ] 2.3.3 Require admin or document admin requirement for global capture

## 3. Discovery on Windows

- [ ] **3.1** LAN discovery
  - [ ] 3.1.1 Implement UDP socket: bind to discovery port (same as in 07-protocol-and-interop)
  - [ ] 3.1.2 Send periodic beacon (multicast or broadcast) with device ID, public key, protocol version
  - [ ] 3.1.3 Listen for beacons from other devices; parse and validate
  - [ ] 3.1.4 Send response to discovered device (if required by protocol)
  - [ ] 3.1.5 Maintain list of discovered peers; call core `on_peer_joined` / `on_peer_left` on change
- [ ] **3.2** Optional: WiFi Direct
  - [ ] 3.2.1 Research Windows WiFi Direct API
  - [ ] 3.2.2 Add optional discovery via WiFi Direct if needed for same-subnet guarantee

## 4. Local transport

- [ ] **4.1** TCP server for incoming peer connections
  - [ ] 4.1.1 Listen on a local port (or use same as discovery with different message type)
  - [ ] 4.1.2 Accept TCP connections from peers; associate with peer_id (from discovery or handshake)
  - [ ] 4.1.3 Decode framed messages; pass to core `on_message_received`
  - [ ] 4.1.4 Send outbound messages from core to peers over corresponding sockets
- [ ] **4.2** TCP client to connect to peers
  - [ ] 4.2.1 When peer discovered, establish TCP connection to peer's advertised address/port
  - [ ] 4.2.2 Perform encrypted handshake if required (session key establishment)
  - [ ] 4.2.3 Exchange heartbeats and chunk messages per core
- [ ] **4.3** Encryption and integrity
  - [ ] 4.3.1 All chunk and control traffic over TCP encrypted by core (host passes bytes to core for encrypt/decrypt or core returns plain bytes and host sends encrypted via core)
  - [ ] 4.3.2 Use same wire format as 07; ensure compatibility with Android/Linux/iOS/macOS

## 5. Integration with pea-core

- [ ] **5.1** Wire core into request path
  - [ ] 5.1.1 For each eligible request: call core with metadata (URL, range); get chunk assignments
  - [ ] 5.1.2 Request chunks: self (WAN) + peers (send chunk request over local transport)
  - [ ] 5.1.3 When chunk data received (from self or peer): pass to core; get reassembled segments
  - [ ] 5.1.4 Stream reassembled response back to client app
- [ ] **5.2** Peer lifecycle
  - [ ] 5.2.1 On new peer discovered and connected: call core `on_peer_joined`
  - [ ] 5.2.2 On peer disconnect or heartbeat timeout: call core `on_peer_left`
  - [ ] 5.2.3 Periodically call core `tick()` and send heartbeat messages to each peer
- [ ] **5.3** WAN requests
  - [ ] 5.3.1 Execute HTTP range requests (for chunks assigned to self) using system or library HTTP client
  - [ ] 5.3.2 Pass response bytes to core as chunk data; core verifies and reassembles

## 6. System tray and UI

- [ ] **6.1** Tray icon
  - [ ] 6.1.1 Create system tray icon (Windows API or Tauri)
  - [ ] 6.1.2 Show "PeaPod" and state: enabled/disabled, "Pod: N devices"
  - [ ] 6.1.3 Menu: Enable / Disable, Open settings, Exit
  - [ ] 6.1.4 On Enable: start proxy, set system proxy, start discovery and local transport
  - [ ] 6.1.5 On Disable: clear system proxy, stop discovery and transport, stop proxy
- [ ] **6.2** Settings window
  - [ ] 6.2.1 Simple settings UI: toggle PeaPod, display pod members (device IDs or anonymized), optional port/config
  - [ ] 6.2.2 Implement via Tauri window or C#/WinUI if separate UI project
- [ ] **6.3** Settings entry in Windows
  - [ ] 6.3.1 Document or implement "PeaPod" entry: link from Windows Settings to app (e.g. URI or open app settings page)
  - [ ] 6.3.2 Optional: add uninstall entry in Settings > Apps

## 7. Installer and distribution

- [ ] **7.1** Installer
  - [ ] 7.1.1 Create installer (e.g. Inno Setup, MSI, or Electron-builder/Tauri bundle) that installs binary and optional shortcut
  - [ ] 7.1.2 Installer does not set proxy by default; user enables in app
  - [ ] 7.1.3 Uninstaller restores proxy to previous state if PeaPod was enabled
- [ ] **7.2** Auto-start (optional)
  - [ ] 7.2.1 Option in settings: "Start PeaPod when I sign in"; set/clear registry or shortcut in Startup folder
  - [ ] 7.2.2 Default: do not auto-start unless user opts in

## 8. Edge cases and robustness

- [ ] **8.1** No peers
  - [ ] 8.1.1 When no peers in pod: proxy still runs; forward all traffic normally (no acceleration)
  - [ ] 8.1.2 UI shows "Pod: 0 devices" or "No peers nearby"
- [ ] **8.2** Graceful shutdown
  - [ ] 8.2.1 On exit: send Leave to peers, clear system proxy, close sockets
  - [ ] 8.2.2 Do not leave system proxy pointing to closed port
- [ ] **8.3** Ineligible traffic
  - [ ] 8.3.1 Detect and pass through non-HTTP or non-range requests without breaking
  - [ ] 8.3.2 Do not accelerate HTTPS where range cannot be used (e.g. streaming DRM); tunnel only
