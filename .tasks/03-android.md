# 03 – Android Protocol Implementation

Implementation of the PeaPod protocol for Android: Kotlin app with VPNService to intercept traffic; JNI to pea-core; discovery and local transport. Depends on 01-pea-core and 07 (protocol/discovery).

## 1. Android project scaffold

- [x] **1.1** Create protocol implementation for Android (project)
  - [x] 1.1.1 Create `pea-android/` with Gradle (Kotlin DSL or Groovy)
  - [x] 1.1.2 Set minSdk (e.g. 24) and targetSdk (e.g. 34)
  - [x] 1.1.3 Add main Activity (or single-Activity app) and Application class if needed
  - [x] 1.1.4 Add dependency on AndroidX and Material (or minimal UI libs)
- [x] **1.2** Rust core integration
  - [x] 1.2.1 Create `pea-android/rust/` or `pea-core-android/` for Rust code that builds for Android (pea-core built for Android targets; libs in rust-out/<abi>)
  - [x] 1.2.2 Add NDK build: compile pea-core (or thin JNI wrapper) for armeabi-v7a, arm64-v8a, x86_64 (for emulator) (CMake links libpea_core.a; CI builds aarch64/x86_64)
  - [x] 1.2.3 Expose JNI functions: init core, on_request, on_peer_joined, on_peer_left, on_message_received, tick, etc. (pea-core ffi.rs C API + pea_jni.c JNI wrappers + PeaCore.kt)
  - [x] 1.2.4 Load native lib in Kotlin (System.loadLibrary) and call from Kotlin
- [x] **1.3** Permissions and manifest
  - [x] 1.3.1 Add INTERNET permission
  - [x] 1.3.2 Add FOREGROUND_SERVICE and FOREGROUND_SERVICE_SPECIAL_USE (or appropriate type) for VPN
  - [x] 1.3.3 Add local network / nearby devices permission (Android 12+) for discovery (ACCESS_NETWORK_STATE, CHANGE_WIFI_MULTICAST_STATE, POST_NOTIFICATIONS)
  - [x] 1.3.4 Declare VPN service in manifest; add BIND_VPN_SERVICE permission (PeaPodVpnService with specialUse/vpn type)

## 2. VPNService and traffic interception

- [x] **2.1** VPN setup
  - [x] 2.1.1 Create class extending VPNService (PeaPodVpnService)
  - [x] 2.1.2 Build VPN tunnel: establish with VPNService.Builder; set address and routes (e.g. 10.0.0.2/32, route 0.0.0.0/0) (addAddress 10.0.0.2/32, addRoute 0.0.0.0/0, addDnsServer 8.8.8.8)
  - [x] 2.1.3 Start VPN from Activity when user taps "Enable"; show system VPN consent dialog (MainActivity: VpnService.prepare, launcher, startVpn)
  - [x] 2.1.4 ParcelFileDescriptor from Builder.establish(); use for reading/writing packets or socket-based approach (PFD stored in tunnelFd; packet read loop deferred to §2.2)
- [ ] **2.2** Redirect traffic to local handler
  - [ ] 2.2.1 Option A: Use VPN to redirect to local proxy (localhost server in app); parse HTTP/HTTPS
  - [ ] 2.2.2 Option B: Parse packets from tunnel and dispatch to in-app TCP stack or proxy
  - [ ] 2.2.3 Implement local proxy (in app) that receives connections from VPN tunnel; parse request URL and headers
  - [ ] 2.2.4 For each request: determine eligibility (HTTP, range-supported); if eligible, pass to core via JNI
- [ ] **2.3** Response path
  - [ ] 2.3.1 Core returns chunk assignments; app requests chunks (self via WAN, peers via local transport)
  - [ ] 2.3.2 When chunks received: pass to core; get reassembled stream
  - [ ] 2.3.3 Write reassembled response back through VPN to app (so original app receives response)
  - [ ] 2.3.4 For ineligible: forward request to real network and forward response back (transparent pass-through)
- [x] **2.4** Foreground service and notification
  - [x] 2.4.1 When VPN is active, run as foreground service with persistent notification (startForeground in onStartCommand)
  - [x] 2.4.2 Notification content: "PeaPod active" and "Pod: N devices" (update when pod changes) (buildNotification(peerCount); updateNotification() for later discovery)
  - [x] 2.4.3 User can tap notification to open app; optional "Disconnect" action in notification (contentIntent MainActivity; action Disconnect)

## 3. Discovery on Android

- [ ] **3.1** LAN discovery
  - [ ] 3.1.1 Request local network permission (Android 12+); handle denied case
  - [ ] 3.1.2 Create UDP socket; join multicast group or use broadcast (same group/port as 07)
  - [ ] 3.1.3 Send periodic beacon (device ID, public key, protocol version) from Kotlin or via JNI (core can produce payload; Kotlin sends)
  - [ ] 3.1.4 Receive beacons; parse; maintain peer list; call into core on_peer_joined / on_peer_left
  - [ ] 3.1.5 Advertise own IP and port for local transport (TCP) in beacon or separate message
- [ ] **3.2** Optional: WiFi Direct
  - [ ] 3.2.1 Add Wi-Fi P2pManager for discovery if needed
  - [ ] 3.2.2 Use for local transfer or discovery; document as optional

## 4. Local transport (in-app)

- [ ] **4.1** TCP server
  - [ ] 4.1.1 Bind TCP server socket to local port (or use one chosen at runtime)
  - [ ] 4.1.2 Accept connections from peers; associate with peer_id (from discovery)
  - [ ] 4.1.3 Read framed messages; pass bytes to core (JNI); send back responses from core to socket
- [ ] **4.2** TCP client
  - [ ] 4.2.1 When peer discovered, connect to peer's advertised IP:port
  - [ ] 4.2.2 Perform handshake if needed; then exchange messages per protocol
  - [ ] 4.2.3 Handle disconnect; call core on_peer_left
- [ ] **4.3** Encryption
  - [ ] 4.3.1 Use core for encrypt/decrypt of wire messages (JNI); send only encrypted bytes over TCP
  - [ ] 4.3.2 Same wire format as other platforms

## 5. Integration with pea-core (JNI)

- [ ] **5.1** JNI API design
  - [ ] 5.1.1 Init: create core instance; return handle (long or jobject)
  - [ ] 5.1.2 Feed request: pass URL, range, method; get back action (accelerate with chunk list or fallback)
  - [ ] 5.1.3 Feed peer events: peer_joined(peer_id, public_key_bytes), peer_left(peer_id)
  - [ ] 5.1.4 Feed incoming message: message_received(peer_id, bytes); get back optional response bytes and/or WAN chunk requests
  - [ ] 5.1.5 Feed chunk data: chunk_received(peer_id, chunk_id, data); get back reassembled segment for app
  - [ ] 5.1.6 Tick: tick(); get back list of messages to send to each peer and heartbeat
- [ ] **5.2** Data types across JNI
  - [ ] 5.2.1 Pass byte arrays (byte[]) for keys, messages, chunk data
  - [ ] 5.2.2 Pass strings (jstring) for URL; pass primitive int/long for chunk IDs or use byte[] for serialized IDs
  - [ ] 5.2.3 Return serialized result (e.g. JSON or bincode) for chunk assignments and reassembled segments if needed
- [ ] **5.3** Thread safety
  - [ ] 5.3.1 Ensure core is called from single thread or core is internally synchronized
  - [ ] 5.3.2 Document which thread (e.g. background executor) calls JNI

## 6. UI and settings

- [ ] **6.1** Main screen
  - [ ] 6.1.1 Single main screen: large toggle "Enable PeaPod" (starts VPN and discovery)
  - [ ] 6.1.2 Display "Pod: N devices" and list of peer device IDs (anonymized or short hash)
  - [ ] 6.1.3 When disabled: show "PeaPod is off" and optional "No peers nearby when enabled"
- [ ] **6.2** Settings
  - [ ] 6.2.1 Settings screen or fragment: battery saver option (reduce participation when low battery), optional "Start on boot"
  - [ ] 6.2.2 Link from Android Settings: add optional Settings panel or "Open in PeaPod" from notification
- [ ] **6.3** First-run and permissions
  - [ ] 6.3.1 On first launch: explain PeaPod and request VPN permission (system dialog when user enables)
  - [ ] 6.3.2 Request local network permission (Android 12+) before starting discovery
  - [ ] 6.3.3 Handle "don't ask again" and guide user to app settings if permission denied

## 7. Battery and performance (PRD)

- [ ] **7.1** Low battery
  - [ ] 7.1.1 Listen to battery level / low-battery broadcast; when low, throttle or pause participation (e.g. stop accepting chunk requests from peers, or reduce beacon rate)
  - [ ] 7.1.2 Optional setting: "Pause when battery below X%"
- [ ] **7.2** Idle overhead
  - [ ] 7.2.1 When no active transfer: minimal CPU (beacon interval reasonable, e.g. every 5–10s)
  - [ ] 7.2.2 Release wake locks when not actively transferring
- [ ] **7.3** Minimal battery impact
  - [ ] 7.3.1 Use efficient discovery (UDP only); avoid constant scanning
  - [ ] 7.3.2 Document and test idle battery consumption in 09-quality-and-metrics

## 8. Build and distribution

- [ ] **8.1** Debug build
  - [ ] 8.1.1 Assemble debug APK with Rust libs for all ABIs (or limit to arm64-v8a for faster build)
  - [ ] 8.1.2 Test on emulator (x86_64) and real device (arm64)
- [ ] **8.2** Release build
  - [ ] 8.2.1 Signing config for release; minify/ProGuard if desired
  - [ ] 8.2.2 Build release AAB/APK for Play Store or sideload
- [ ] **8.3** Store listing (optional)
  - [ ] 8.3.1 Prepare store listing: short description, privacy policy if needed (no centralized logging per PRD)
  - [ ] 8.3.2 Declare VPN and network permissions in store console
