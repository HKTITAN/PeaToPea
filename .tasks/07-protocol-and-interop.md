# 07 – Protocol and Interop

Single wire format, discovery spec, and versioning so every implementation can join the same pod. Implement in pea-core and reference from all protocol implementation tasks.

## 1. Wire format specification

- [ ] **1.1** Document wire format
  - [ ] 1.1.1 Choose encoding: bincode (binary, compact) or custom binary; write short spec (field order, types, endianness)
  - [ ] 1.1.2 List all message types and their fields (beacon, response, join, leave, heartbeat, chunk request, chunk data, NACK)
  - [ ] 1.1.3 Define framing: length-prefix (4 bytes LE) + payload, or delimiter; document in 08-documentation
  - [ ] 1.1.4 Ensure same spec is implementable in Rust (pea-core), Kotlin (Android), Swift (iOS/macOS), and any other language
- [ ] **1.2** Version field
  - [ ] 1.2.1 Reserve protocol_version (e.g. u8 or u16) in beacon and in connection handshake
  - [ ] 1.2.2 Set current version to 1; document that peers with different major version may reject or downgrade
  - [ ] 1.2.3 Define compatibility rule: e.g. same major version required; minor may differ with best-effort support

## 2. Discovery protocol

- [ ] **2.1** Transport and address
  - [ ] 2.1.1 Choose: UDP multicast group (e.g. 239.255.60.60) + port (e.g. 45678), or link-local broadcast + port
  - [ ] 2.1.2 Document in 08: same group and port used by Windows, Android, Linux, iOS, macOS
  - [ ] 2.1.3 Define TTL/hop count for multicast (e.g. 1 for same subnet only)
- [ ] **2.2** Beacon format
  - [ ] 2.2.1 Beacon payload: protocol_version, device_id (bytes), public_key (bytes), optional capabilities, optional listen address:port for TCP
  - [ ] 2.2.2 Serialize with same wire encoding as 1.1
  - [ ] 2.2.3 Beacon interval: e.g. every 3–5 seconds; document so all platforms use similar interval
- [ ] **2.3** Response (if required)
  - [ ] 2.3.1 Define response message: sent to beacon sender's address; includes own device_id, public_key, listen address:port
  - [ ] 2.3.2 Or: no explicit response; receivers consider "beacon received" as discovery; document chosen behavior
- [ ] **2.4** Local transport address
  - [ ] 2.4.1 Each device advertises IP (or hostname) and port for TCP local transport
  - [ ] 2.4.2 Document: how to get "my LAN IP" per platform (e.g. primary interface, or from multicast send socket)
  - [ ] 2.4.3 Port: fixed (e.g. 45679) or ephemeral; if ephemeral, must be in beacon/response

## 3. Connection handshake (local transport)

- [ ] **3.1** First message on TCP
  - [ ] 3.1.1 Define handshake: e.g. send protocol_version + device_id + public_key; receive same from peer
  - [ ] 3.1.2 Derive session key (from two keypairs); switch to encrypted messages for all subsequent frames
  - [ ] 3.1.3 Reject if protocol_version incompatible
- [ ] **3.2** Encryption of subsequent messages
  - [ ] 3.2.1 All messages after handshake: AEAD (e.g. ChaCha20-Poly1305) with session key; include nonce (e.g. counter per direction)
  - [ ] 3.2.2 Frame: [nonce][ciphertext][tag] or [length][nonce][ciphertext]; document in 08
  - [ ] 3.2.3 Integrity: AEAD provides integrity; no separate hash for control messages if AEAD is used
- [ ] **3.3** Chunk data messages
  - [ ] 3.3.1 Chunk payload may be large; define chunk message: chunk_id, offset, length, hash, encrypted_payload (or plain payload + hash, then encrypt whole message)
  - [ ] 3.3.2 Receiver verifies hash after decrypting; NACK on failure

## 4. Versioning and compatibility

- [ ] **4.1** Backward compatibility
  - [ ] 4.1.1 Document policy: new minor version may add optional fields or new message types; old peers ignore unknown
  - [ ] 4.1.2 New major version: may break wire format; document upgrade path (e.g. support two major versions in core for transition)
- [ ] **4.2** Reject and downgrade
  - [ ] 4.2.1 When receiving beacon with unsupported major version: do not add to peer list (or add as "incompatible")
  - [ ] 4.2.2 When handshake fails due to version: close connection and log; do not crash
  - [ ] 4.2.3 Document in 08 how to handle version mismatch in UI ("Peer is using a different PeaPod version")

## 5. Cross-platform interop tests

Each implementation (per OS) must speak the same protocol; the tests below verify that.

- [ ] **5.1** Test matrix (manual or automated)
  - [ ] 5.1.1 Windows + Android: same LAN; both discover each other; form pod; run one download with chunk from each; verify reassembly
  - [ ] 5.1.2 Windows + Linux: same; verify discovery and chunk exchange
  - [ ] 5.1.3 Android + iOS: same; verify discovery and chunk exchange
  - [ ] 5.1.4 macOS + iOS: same; verify discovery and chunk exchange (same protocol)
  - [ ] 5.1.5 Linux + macOS: same
  - [ ] 5.1.6 Full pod: one device of each type (Windows, Android, Linux, iOS, macOS) in same pod; one transfer uses chunks from multiple device types; verify no breakage
- [ ] **5.2** Automated interop (optional)
  - [ ] 5.2.1 If possible: run two processes (e.g. Windows + Linux in CI) with mock or real discovery; run one transfer; assert success
  - [ ] 5.2.2 Or: unit test wire format: Rust encodes, Kotlin/Swift decodes (and vice versa) for each message type
- [ ] **5.3** Document results
  - [ ] 5.3.1 In 08-documentation: list tested platform pairs and outcome; update as new platforms added

## 6. Reference implementation (pea-core)

- [ ] **6.1** Single source of truth
  - [ ] 6.1.1 All message types and wire encoding implemented in pea-core (see 01-pea-core)
  - [ ] 6.1.2 Other platforms either use pea-core (Rust) or reimplement from spec; spec in 08 must match pea-core behavior
  - [ ] 6.1.3 Add tests in 01 that encode/decode each message and assert byte-equality or roundtrip
- [ ] **6.2** Spec document
  - [ ] 6.2.1 Write PROTOCOL.md or section in 08: wire format, discovery, handshake, versioning (reference from this task file)
  - [ ] 6.2.2 Link from README and from each platform's docs so implementers can stay in sync
