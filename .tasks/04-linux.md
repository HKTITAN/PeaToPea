# 04 – Linux Protocol Implementation

Implementation of the PeaPod protocol for Linux: daemon that runs as system or user service; system proxy (or netfilter); discovery and local transport; optional tray. Depends on 01-pea-core and 07.

## 1. Project scaffold

- [ ] **1.1** Create protocol implementation for Linux (crate)
  - [ ] 1.1.1 Add `pea-linux/` as Rust binary (e.g. `cargo init --bin pea-linux`)
  - [ ] 1.1.2 Add dependency on `pea-core`
  - [ ] 1.1.3 Add Cargo.toml with Linux deps (e.g. for systemd, or minimal for proxy only)
- [ ] **1.2** Build
  - [ ] 1.2.1 `cargo build --release` for target `x86_64-unknown-linux-gnu` and optionally `aarch64-unknown-linux-gnu`
  - [ ] 1.2.2 Document run: `./pea-linux` or `pea-linux` from PATH

## 2. Traffic interception (v1: system proxy)

- [ ] **2.1** Proxy server
  - [ ] 2.1.1 Implement HTTP/HTTPS proxy (listen on 127.0.0.1:port, configurable)
  - [ ] 2.1.2 Handle CONNECT (HTTPS) and GET/POST with range; parse for eligibility
  - [ ] 2.1.3 Eligible requests: hand to core; execute chunk plan (self + peers); reassemble and return
  - [ ] 2.1.4 Ineligible: forward to upstream (direct or via existing system proxy)
- [ ] **2.2** System proxy configuration
  - [ ] 2.2.1 Read environment: HTTP_PROXY, HTTPS_PROXY (many GUI apps and terminals respect these)
  - [ ] 2.2.2 When enabling PeaPod: export HTTP_PROXY and HTTPS_PROXY to point to local proxy (for user session)
  - [ ] 2.2.3 Document: for global effect, user must set in shell profile or desktop environment (e.g. GNOME proxy settings)
  - [ ] 2.2.4 When disabling: unset or restore previous proxy
- [ ] **2.3** Optional: netfilter/iptables path (post-v1)
  - [ ] 2.3.1 Document iptables REDIRECT or DNAT to send selected traffic to local proxy port
  - [ ] 2.3.2 May require cap_net_admin or root; document and provide optional script
- [ ] **2.4** Optional: eBPF (future)
  - [ ] 2.4.1 Research eBPF for traffic redirect on modern kernels; document as future option

## 3. Discovery on Linux

- [ ] **3.1** LAN discovery
  - [ ] 3.1.1 UDP socket: bind to discovery port; join multicast group (same as 07) or use broadcast
  - [ ] 3.1.2 Send periodic beacon (device ID, public key, protocol version)
  - [ ] 3.1.3 Receive beacons; parse; maintain peer list; notify core on peer join/leave
  - [ ] 3.1.4 Advertise own IP and TCP port for local transport in beacon
- [ ] **3.2** Firewall
  - [ ] 3.2.1 Document: allow inbound UDP on discovery port and TCP on local transport port (e.g. ufw or firewalld)
  - [ ] 3.2.2 Optional: open ports automatically with policy kit or document manual step

## 4. Local transport

- [ ] **4.1** TCP server
  - [ ] 4.1.1 Listen on configurable port (e.g. 0 for ephemeral, or fixed port from config)
  - [ ] 4.1.2 Accept connections from peers; associate with peer_id; frame messages
  - [ ] 4.1.3 Pass received messages to core; send core output to peer sockets
- [ ] **4.2** TCP client
  - [ ] 4.2.1 Connect to each discovered peer's advertised address:port
  - [ ] 4.2.2 Handshake and encrypted message exchange per core
  - [ ] 4.2.3 Heartbeats and chunk traffic
- [ ] **4.3** Same wire format as 07; encryption via core

## 5. Integration with pea-core

- [ ] **5.1** Request path
  - [ ] 5.1.1 For each eligible request: core returns chunk plan; daemon fetches chunks (self + peers)
  - [ ] 5.1.2 Pass chunk data to core; stream reassembled response to client
- [ ] **5.2** Peer lifecycle and tick
  - [ ] 5.2.1 On peer join/leave: call core; send/receive heartbeats; call core tick()
- [ ] **5.3** WAN
  - [ ] 5.3.1 Use HTTP client (e.g. reqwest or ureq) for range requests assigned to self

## 6. Configuration

- [ ] **6.1** Config file
  - [ ] 6.1.1 Support config file (e.g. `~/.config/peapod/config.toml` or `/etc/peapod/config.toml`)
  - [ ] 6.1.2 Options: proxy listen port, discovery port, transport port, optional proxy upstream
  - [ ] 6.1.3 Optional: enable/disable from config (or from CLI/tray only)
- [ ] **6.2** Environment
  - [ ] 6.2.1 Override config with env vars if desired (e.g. PEAPOD_PROXY_PORT)
  - [ ] 6.2.2 Document env vars in 08-documentation

## 7. systemd integration

- [ ] **7.1** User service (recommended)
  - [ ] 7.1.1 Write systemd user unit file (e.g. `~/.config/systemd/user/peapod.service`)
  - [ ] 7.1.2 ExecStart = path to pea-linux binary and args (e.g. --proxy-port 3128)
  - [ ] 7.1.3 Restart=on-failure; document how to enable: `systemctl --user enable peapod`
- [ ] **7.2** System service (optional)
  - [ ] 7.2.1 Write system unit for installation under /usr/local or /opt
  - [ ] 7.2.2 Run as dedicated user (e.g. peapod) for security
- [ ] **7.3** Installer or package
  - [ ] 7.3.1 Place unit file in package so package install can enable user service (or document manual copy)

## 8. Optional tray and UI

- [ ] **8.1** Tray icon
  - [ ] 8.1.1 Optional: build with GTK or Tauri for system tray (Linux tray spec)
  - [ ] 8.1.2 Show status: enabled/disabled, "Pod: N devices"; menu: Enable/Disable, Settings, Quit
  - [ ] 8.1.3 When Enable: start daemon (or communicate with already-running daemon via socket)
- [ ] **8.2** Headless mode
  - [ ] 8.2.1 Daemon must run without GUI (e.g. on server or SSH session); config and CLI only for enable/disable
  - [ ] 8.2.2 CLI flags: e.g. `pea-linux --enable`, `pea-linux --disable`, `pea-linux --status`

## 9. Packaging and distribution

- [ ] **9.1** .deb (Debian/Ubuntu)
  - [ ] 9.1.1 Create debian/ directory or use cargo-deb: control file, install binary to /usr/bin
  - [ ] 9.1.2 Install systemd user unit to /usr/lib/systemd/user/ or document manual install
  - [ ] 9.1.3 Build .deb and test install/uninstall
- [ ] **9.2** Other formats
  - [ ] 9.2.1 Optional: .rpm for Fedora/RHEL
  - [ ] 9.2.2 Optional: Snap or Flatpak for distribution (include confinement and proxy access)
- [ ] **9.3** Binary release
  - [ ] 9.3.1 Provide static or dynamic binary for x86_64 and aarch64 on GitHub Releases or website
  - [ ] 9.3.2 Document install steps: download, chmod +x, optional move to PATH and systemd enable

## 10. Edge cases

- [ ] **10.1** No peers
  - [ ] 10.1.1 Proxy runs; all traffic forwarded without acceleration; log or status "Pod: 0 devices"
- [ ] **10.2** Graceful shutdown
  - [ ] 10.2.1 On SIGTERM: send Leave to peers, close sockets, exit; systemd restarts if configured
  - [ ] 10.2.2 Clear proxy env or restore if daemon was setting it
- [ ] **10.3** Privileged ports
  - [ ] 10.3.1 Do not require root for default ports; use high port (e.g. 3128) or document setcap if user wants 80
