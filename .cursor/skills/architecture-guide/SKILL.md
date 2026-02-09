---
name: architecture-guide
description: Use when making architectural decisions, adding components, or understanding how PeaPod's pieces fit together. Provides context on the protocol design, component boundaries, and data flow to help agents make correct design choices.
---

# PeaPod Architecture Guide

Use this skill when you need to understand or modify PeaPod's architecture.

## Key architectural decisions

### 1. pea-core is host-driven with NO I/O
- pea-core is a pure-logic library. It never opens sockets, reads files, or makes HTTP calls.
- The host (platform implementation) calls pea-core with events and executes the returned actions.
- This makes pea-core portable across all platforms and testable without mocking network I/O.
- **If you're about to add `tokio`, `std::net`, `reqwest`, or `std::fs` to pea-core — STOP. That belongs in the platform implementation.**

### 2. All platforms share the same wire format
- Protocol messages are defined in `pea-core/src/protocol.rs` and encoded with bincode + 4-byte LE length framing.
- Every platform (Linux, Windows, Android, iOS, macOS) must produce and consume identical bytes on the wire.
- **Never** create platform-specific message types or encodings.

### 3. Discovery is LAN-only
- UDP multicast on 239.255.60.60:45678 with TTL=1.
- Beacons include protocol_version, device_id, public_key, and listen_port.
- No central server, no cloud discovery, no WAN discovery.

### 4. Security is not optional
- X25519 key exchange on every TCP connection.
- ChaCha20-Poly1305 AEAD encryption for all post-handshake traffic.
- SHA-256 integrity verification for every chunk.
- Counter-based nonces (never reuse).

## Component boundaries

| Component | Crate | What it does | What it does NOT do |
|-----------|-------|-------------|---------------------|
| Protocol logic | pea-core | Chunk splitting, scheduling, integrity, wire codec | Any I/O |
| Identity/crypto | pea-core | Key generation, key exchange, encrypt/decrypt helpers | Key storage (host responsibility) |
| Discovery | pea-linux/windows/etc | UDP multicast send/receive, peer tracking | Protocol message parsing (uses pea-core wire codec) |
| Transport | pea-linux/windows/etc | TCP connections, handshake, encrypted framing | Chunk logic (calls pea-core on_message_received) |
| Proxy/VPN | pea-linux/windows/etc | Traffic interception, HTTP parsing | Scheduling (calls pea-core on_incoming_request) |

## Data flow reference

See [docs/ARCHITECTURE.md](../../docs/ARCHITECTURE.md) for Mermaid diagrams of:
- Layer placement
- System architecture (two-device view)
- Host ↔ pea-core interaction
- Accelerated download sequence
- Discovery and connection handshake

## When to update architecture docs

Update `docs/ARCHITECTURE.md` when:
- Adding a new component or crate
- Changing how components interact
- Adding a new message type to the protocol
- Changing the security model

Update `docs/PROTOCOL.md` when:
- Adding or changing message types in `protocol.rs`
- Changing the wire format or encoding
- Changing discovery or handshake behavior
