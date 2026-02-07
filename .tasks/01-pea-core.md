# 01 – Pea-Core (Protocol Reference Implementation)

Reference implementation of the PeaPod protocol: shared protocol logic, crypto, chunking, scheduling, and integrity. No GUI; host-driven API. Must be finished before any protocol implementation (Windows, Android, etc.).

## 1. Crate bootstrap

- [x] **1.1** Create crate
  - [x] 1.1.1 Run `cargo init --lib` in `pea-core/` (or add as workspace member)
  - [x] 1.1.2 Set crate name in `Cargo.toml` (e.g. `pea-core` or `pea_protocol`)
  - [x] 1.1.3 Set edition = "2021" (or current stable)
- [x] **1.2** Dependencies
  - [x] 1.2.1 Add `serde` and `serde_json` or `bincode` for serialization
  - [x] 1.2.2 Add crypto: e.g. `x25519-dalek`, `chacha20poly1305` or `aes-gcm`
  - [x] 1.2.3 Add hashing: e.g. `sha2` or `blake2`
  - [x] 1.2.4 Add `thiserror` and/or `anyhow` for errors
  - [x] 1.2.5 Add `uuid` or similar for chunk/transfer IDs if needed
  - [x] 1.2.6 Add `rand` for key generation and nonces
  - [x] 1.2.7 Add `tracing` or `log` for diagnostics (optional, no I/O in core)

## 2. Identity and crypto module

- [x] **2.1** Device identity
  - [x] 2.1.1 Define type for device keypair (e.g. X25519)
  - [x] 2.1.2 Implement keypair generation (random)
  - [x] 2.1.3 Define device ID as deterministic hash of public key (e.g. SHA-256 truncated or BLAKE2)
  - [x] 2.1.4 Expose public key and device ID types (serializable for wire)
  - [x] 2.1.5 Implement serialization of public key and device ID for discovery/beacon
- [x] **2.2** Session keys for pod
  - [x] 2.2.1 Define session key type (symmetric: e.g. 256-bit)
  - [x] 2.2.2 Implement key exchange: two devices derive shared secret from keypairs
  - [x] 2.2.3 Derive session encryption key from shared secret (KDF: e.g. HKDF or simple hash)
  - [x] 2.2.4 Document or implement "pod session": N devices share same session key or pairwise (decide spec)
- [x] **2.3** Encryption of wire messages
  - [x] 2.3.1 Choose AEAD: ChaCha20-Poly1305 or AES-GCM
  - [x] 2.3.2 Implement encrypt(plaintext, key, nonce) -> ciphertext
  - [x] 2.3.3 Implement decrypt(ciphertext, key, nonce) -> Result<plaintext>
  - [x] 2.3.4 Define nonce format (e.g. 96-bit counter or random per message; document reuse rules)
- [x] **2.4** No plaintext inspection
  - [x] 2.4.1 Ensure core never requires plaintext of user data for eligibility; only metadata (e.g. URL, range) if at all
  - [x] 2.4.2 Chunk hashes are over plaintext before encrypting; verify after decrypting

## 3. Protocol and wire format

- [x] **3.1** Message types (define enum or struct per type)
  - [x] 3.1.1 Discovery beacon (advertise presence; include device ID, public key, protocol version)
  - [x] 3.1.2 Discovery response (ack; include own device ID, public key)
  - [x] 3.1.3 Join (request to join pod or confirm membership)
  - [x] 3.1.4 Leave (graceful leave)
  - [x] 3.1.5 Heartbeat (liveness; optional payload)
  - [x] 3.1.6 Chunk request (request a chunk by ID or range)
  - [x] 3.1.7 Chunk data (payload: chunk bytes or encrypted chunk + hash)
  - [x] 3.1.8 NACK / redistribute (chunk failed or peer left; trigger reassignment)
- [x] **3.2** Serialization
  - [x] 3.2.1 Implement Serialize/Deserialize for all message types (serde)
  - [x] 3.2.2 Choose wire encoding: bincode or custom binary; document in 07-protocol-and-interop
  - [x] 3.2.3 Add protocol version field to beacon and handshake messages
  - [x] 3.2.4 Ensure wire format is stable (no breaking changes without version bump)
- [x] **3.3** Framing
  - [x] 3.3.1 Define frame format if needed (length-prefix or delimiter)
  - [x] 3.3.2 Implement encode frame (message -> bytes)
  - [x] 3.3.3 Implement decode frame (bytes -> message) with error handling for partial reads

## 4. Chunk manager

- [x] **4.1** Chunk model
  - [x] 4.1.1 Define chunk ID type (e.g. transfer_id + range or index)
  - [x] 4.1.2 Define chunk size policy: configurable constant or adaptive (start with constant)
  - [x] 4.1.3 Implement "split transfer into chunks": input (url, total length or range), output list of chunk IDs / ranges
  - [x] 4.1.4 Support HTTP range semantics: each chunk = one range request (start, end)
- [x] **4.2** Chunk state per transfer
  - [x] 4.2.1 Track which chunks are assigned to which peer
  - [x] 4.2.2 Track which chunks are received and verified
  - [x] 4.2.3 Track which chunks are in flight (requested but not yet received)
  - [x] 4.2.4 Detect when all chunks for a transfer are complete
- [x] **4.3** Request and receive
  - [x] 4.3.1 Implement "request chunk from peer" (output: message to send to peer)
  - [x] 4.3.2 Implement "on chunk data received": verify integrity, store, update state
  - [x] 4.3.3 Prevent duplicate downloads: do not assign same chunk to two peers (or allow and dedupe; decide)
  - [x] 4.3.4 Implement "reassemble chunks in order" -> single byte stream for host to feed to app
- [ ] **4.4** Upload path
  - [x] 4.4.1 Define "split outbound data into chunks" for uploads
  - [ ] 4.4.2 Assign upload chunks to peers; each peer uploads its portion via own WAN
  - [ ] 4.4.3 Track completion and integrity for upload chunks; coordinate server-side compatibility (e.g. multipart or range put if supported)

## 5. Distributed scheduler

- [x] **5.1** Inputs
  - [x] 5.1.1 Peer list: set of device IDs (or peer handles) currently in pod
  - [ ] 5.1.2 Per-peer metrics: bandwidth (optional), latency (optional), stability (e.g. recent failures)
  - [x] 5.1.3 Current transfer: list of chunks and current assignment
- [x] **5.2** Assignment logic
  - [x] 5.2.1 Implement assign chunks to peers (e.g. round-robin or by bandwidth weight)
  - [x] 5.2.2 Implement reassignment when a peer leaves: move its chunks to remaining peers
  - [ ] 5.2.3 Implement "reduce allocation to slow peer": decrease weight or exclude if repeated failure
  - [x] 5.2.4 Output: for each chunk, which peer should fetch/upload it
- [ ] **5.3** Eligibility and fallback
  - [ ] 5.3.1 Define "eligible flow": e.g. HTTP/HTTPS with range support; no DRM/encrypted streaming
  - [ ] 5.3.2 Core exposes "is_eligible(metadata)" or host decides and only sends eligible flows to core
  - [ ] 5.3.3 When no peers available or flow ineligible: core returns "fallback to normal path" (host handles)

## 6. Integrity verification

- [ ] **6.1** Per-chunk hash
  - [ ] 6.1.1 Choose hash: SHA-256 or BLAKE2; implement hash(chunk_bytes) -> digest
  - [ ] 6.1.2 Include hash in chunk request (requester may not have it yet; or hash comes with chunk data)
  - [ ] 6.1.3 On chunk data received: compute hash of payload, compare to expected; reject if mismatch
  - [ ] 6.1.4 On mismatch: mark chunk failed, trigger NACK/redistribute so scheduler reassigns
- [ ] **6.2** Malicious peer handling
  - [ ] 6.2.1 On integrity failure: record peer; optionally isolate (stop assigning to that peer) or retry once
  - [ ] 6.2.2 Document behavior: isolate after N failures (configurable)

## 7. Failure recovery

- [ ] **7.1** Timeouts
  - [ ] 7.1.1 Define timeout for chunk request (e.g. 30s); configurable
  - [ ] 7.1.2 On timeout: mark chunk as not received; reassign to another peer or retry
  - [x] 7.1.3 Define timeout for heartbeat (e.g. peer considered dead after 3 missed)
- [x] **7.2** Heartbeat
  - [x] 7.2.1 Core produces "send heartbeat" events at interval (host sends over transport)
  - [x] 7.2.2 Core consumes "heartbeat received" from host; update last-seen per peer
  - [x] 7.2.3 When peer exceeds heartbeat timeout: emit "peer left" internally; scheduler redistributes its chunks
- [x] **7.3** Redistribution
  - [x] 7.3.1 When peer leaves (leave message or heartbeat timeout): get list of chunks assigned to that peer
  - [x] 7.3.2 Reassign those chunks to remaining peers via scheduler
  - [x] 7.3.3 Emit chunk request messages for newly assigned chunks

## 8. Host-driven API

- [x] **8.1** Core API surface
  - [x] 8.1.1 Define main entry type (e.g. `PeaPodCore` or `Coordinator`) that holds state
  - [x] 8.1.2 Method: `on_incoming_request(metadata)` -> Action (e.g. StartTransfer { chunks, assignments } or Fallback)
  - [x] 8.1.3 Method: `on_peer_joined(peer_id, public_key)` -> optional session setup / welcome
  - [x] 8.1.4 Method: `on_peer_left(peer_id)` -> internal redistribution; output messages to send if any
  - [ ] 8.1.5 Method: `on_message_received(peer_id, bytes)` -> decrypt, parse, update state; return optional response messages and/or chunk requests for WAN
  - [x] 8.1.6 Method: `on_chunk_received(peer_id, chunk_id, data)` -> verify, store; return optional reassembled stream segment for host to pass to app
  - [x] 8.1.7 Method: `tick()` or `poll_events()` -> heartbeat timers, timeouts; return list of outbound messages and WAN chunk requests
- [ ] **8.2** Host responsibilities (document only)
  - [x] 8.2.1 Document: host performs actual I/O (sockets, discovery, proxy/VPN); core is pure logic + crypto
  - [x] 8.2.2 Document: host passes in parsed request metadata (URL, range, method); host executes WAN requests and injects chunk data into core
  - [x] 8.2.3 Document: host sends core-generated messages to peers over local transport

## 9. Traffic eligibility (logic in core)

- [ ] **9.1** Eligibility rules
  - [ ] 9.1.1 Support HTTP range-based downloads: core accepts range (start, end) and splits into chunks
  - [ ] 9.1.2 Support chunked uploads: core splits outbound body into chunks for peers
  - [ ] 9.1.3 Mark or reject flows that must not be accelerated (e.g. DRM, encrypted streaming); host can pass "ineligible" and core returns Fallback
  - [ ] 9.1.4 Document: host decides eligibility when possible; core may reject if it cannot handle (e.g. unknown protocol)

## 10. Unit and integration tests

- [x] **10.1** Identity and crypto
  - [x] 10.1.1 Test keypair generation and device ID derivation
  - [x] 10.1.2 Test key exchange: two keypairs produce same shared secret
  - [x] 10.1.3 Test encrypt/decrypt roundtrip
- [x] **10.2** Protocol
  - [x] 10.2.1 Test serialize/deserialize for each message type (roundtrip)
  - [x] 10.2.2 Test framing encode/decode with partial reads
- [ ] **10.3** Chunk manager
  - [ ] 10.3.1 Test split transfer into chunks (various sizes and chunk sizes)
  - [ ] 10.3.2 Test reassembly order and completeness
  - [ ] 10.3.3 Test duplicate chunk handling (idempotent or reject)
- [ ] **10.4** Scheduler
  - [ ] 10.4.1 Test assignment with 1, 2, N peers
  - [ ] 10.4.2 Test reassignment when one peer "leaves"
  - [ ] 10.4.3 Test no assignment when zero peers (fallback)
- [ ] **10.5** Integrity
  - [ ] 10.5.1 Test valid chunk passes verification
  - [ ] 10.5.2 Test tampered chunk fails verification and triggers reassign
- [ ] **10.6** Integration (mock host)
  - [ ] 10.6.1 Mock host: inject request, no peers -> Fallback
  - [ ] 10.6.2 Mock host: inject request, one peer -> chunk assignments and mock chunk data -> reassembled output
  - [ ] 10.6.3 Mock host: peer leaves mid-transfer -> redistribution and completion
  - [ ] 10.6.4 Mock host: heartbeat timeout -> peer marked dead, chunks redistributed
  - [x] 10.6.5 Integration test: request with range, receive chunks, verify reassembled output

## 11. No platform-specific I/O

- [ ] **11.1** Ensure crate has no std::net, no tokio/async, no file I/O unless behind feature flag for tests
  - [ ] 11.1.1 Use only types that can be passed in from host (bytes, slices)
  - [ ] 11.1.2 Optional: feature "test-utils" for mock time or mock RNG in tests only
