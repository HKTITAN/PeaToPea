# PeaPod Architecture

## Overview

PeaPod is a cooperative bandwidth protocol. Nearby devices form an encrypted mesh ("pod") over the local network and split internet traffic across all members' connections. This document describes the architecture: layer placement, components, data flow, and how `pea-core` fits with platform implementations.

## Layer placement

PeaPod sits **above IP** and **below applications**:

```mermaid
block-beta
    columns 1
    block:app["Application (browser, app)"]
    end
    block:tcp["TCP / HTTP / TLS"]
    end
    block:peapod["PeaPod cooperative layer"]
        columns 3
        A["Intercept\n(proxy / VPN / NE)"]
        B["Schedule & chunk\n(pea-core)"]
        C["Transport\n(local TCP)"]
    end
    block:platform["Platform (proxy, VPN, Network Extension)"]
    end
    block:net["Network interfaces (WAN + LAN)"]
    end

    app --> tcp
    tcp --> peapod
    peapod --> platform
    platform --> net
```

Traffic is intercepted by the platform implementation (e.g. system proxy or VPNService). Eligible flows are handed to the cooperative layer, which uses pea-core for protocol logic. Chunks are fetched or sent via WAN and exchanged over the local link; the requesting device reassembles and delivers the result to the app.

## System architecture

```mermaid
graph TB
    subgraph DeviceA["Device A (requesting)"]
        AppA["App (browser)"]
        subgraph PlatformA["Platform Implementation"]
            ProxyA["Proxy / VPN /<br/>Network Extension"]
            DiscoveryA["Discovery<br/>(UDP multicast)"]
            TransportA["Transport<br/>(TCP)"]
            subgraph CoreA["pea-core"]
                SchedulerA["Scheduler"]
                ChunkMgrA["Chunk Manager"]
                IdentityA["Identity"]
                IntegrityA["Integrity"]
                WireA["Wire Codec"]
            end
        end
        WANA["WAN (Internet)"]
    end

    subgraph DeviceB["Device B (peer)"]
        subgraph PlatformB["Platform Implementation"]
            TransportB["Transport<br/>(TCP)"]
            CoreB["pea-core"]
        end
        WANB["WAN (Internet)"]
    end

    AppA --> ProxyA
    ProxyA <--> CoreA
    ProxyA --> DiscoveryA
    ProxyA --> TransportA
    TransportA --> WANA
    TransportA <-->|"Encrypted TCP"| TransportB
    TransportB --> WANB
    TransportB <--> CoreB
```

## Core components

| Component | Location | Responsibility |
|-----------|----------|----------------|
| **Discovery** | Platform | Advertise presence and discover peers via LAN multicast/broadcast (same wire format on all platforms) |
| **Identity & encryption** | pea-core + platform | Device keypairs (X25519), device ID derivation, session keys, ChaCha20-Poly1305 AEAD encryption |
| **Scheduler** | pea-core | Assign chunks to peers (round-robin, weighted, or single-peer) based on availability and metrics |
| **Chunk manager** | pea-core | Split transfers into byte-range chunks, track state per transfer, reassemble completed chunks |
| **Wire codec** | pea-core | Encode/decode protocol messages (bincode, 4-byte LE length framing) |
| **Integrity** | pea-core | SHA-256 hash per chunk; verify on receive; reject and reassign on failure |
| **Local transport** | Platform | TCP connections between peers; framed + encrypted messages (pea-core defines format, platform does I/O) |
| **Traffic interception** | Platform | System proxy (Linux/Windows), VPNService (Android), Network Extension (iOS/macOS) |

## pea-core: host-driven, no I/O

The shared library **pea-core** contains no sockets, no file I/O, and no platform APIs. The host (each platform implementation) is responsible for:

- Discovery (sending and receiving beacons).
- Accepting and parsing incoming requests (e.g. HTTP with range).
- Sending and receiving framed messages to peers over TCP.
- Executing WAN requests for chunks assigned to self.
- Calling into pea-core with events (request metadata, peer joined/left, message received, chunk data received) and acting on returned values (chunk assignments, messages to send, reassembled stream).

This keeps pea-core portable and testable with mock hosts.

```mermaid
graph LR
    subgraph Host["Host (Platform)"]
        IO["Sockets, HTTP,<br/>UDP, TCP, proxy,<br/>VPN, file I/O"]
    end

    subgraph Core["pea-core (pure logic)"]
        API["on_incoming_request → Action<br/>on_chunk_received → Option body<br/>on_peer_joined/left<br/>on_message_received → Actions<br/>tick → Actions"]
    end

    IO -->|"events"| Core
    Core -->|"actions"| IO
```

## Data flow: accelerated download

```mermaid
sequenceDiagram
    participant App
    participant Host as Host (Platform)
    participant Core as pea-core
    participant PeerB as Peer B

    App->>Host: HTTP GET /file
    Host->>Core: on_incoming_request(url, range)
    Core-->>Host: Action::Accelerate<br/>{chunks: [(0-50K, self), (50K-100K, peer_B)]}

    par Self-fetch via WAN
        Host->>Host: GET Range: bytes=0-50K (WAN)
    and Peer-fetch via transport
        Host->>PeerB: ChunkRequest(50K-100K)
        PeerB->>PeerB: GET Range: bytes=50K-100K (WAN)
        PeerB-->>Host: ChunkData(50K-100K, hash, payload)
    end

    Host->>Core: on_chunk_received(0-50K)
    Host->>Core: on_chunk_received(50K-100K)
    Core-->>Host: Ok(Some(full_body))

    Host-->>App: HTTP 200 OK (full body)
```

## Discovery and connection sequence

```mermaid
sequenceDiagram
    participant A as Device A
    participant LAN as LAN (multicast 239.255.60.60:45678)
    participant B as Device B

    A->>LAN: Beacon {ver, device_id_A, pubkey_A, port}
    LAN->>B: Beacon
    B->>LAN: Beacon {ver, device_id_B, pubkey_B, port}
    LAN->>A: Beacon
    B-->>A: DiscoveryResponse

    Note over A,B: TCP connection to B's listen_port

    A->>B: Handshake [ver | device_id | public_key]
    B->>A: Handshake [ver | device_id | public_key]

    Note over A,B: Both compute:<br/>shared_secret = X25519(my_secret, peer_pubkey)<br/>session_key = SHA-256(shared_secret)

    A<-->B: Encrypted chunk/control traffic (ChaCha20-Poly1305)
```

## Platform implementation matrix

| Platform | Traffic interception | Discovery | Transport | UI |
|----------|---------------------|-----------|-----------|-----|
| **Linux** | HTTP proxy (localhost:3128) | UDP multicast | TCP | CLI / systemd |
| **Windows** | System proxy (registry) | UDP multicast | TCP | System tray (Win32) |
| **Android** | VPNService | UDP multicast | TCP | Activity + Service |
| **iOS** | Network Extension (planned) | UDP multicast | TCP | SwiftUI (planned) |
| **macOS** | Network Extension (planned) | UDP multicast | TCP | Menu bar (planned) |

## Security model

```mermaid
graph TD
    KG["Key Generation<br/>(X25519 keypair at first run)"] --> DID["Device ID<br/>(SHA-256 of pubkey, first 16 bytes)"]
    KG --> KE["Key Exchange<br/>(X25519 Diffie-Hellman on TCP connect)"]
    KE --> SK["Session Key<br/>(SHA-256 of shared secret)"]
    SK --> AEAD["Session Encryption<br/>(ChaCha20-Poly1305 AEAD,<br/>counter nonce per direction)"]
    AEAD --> CI["Chunk Integrity<br/>(SHA-256 per chunk,<br/>Nack + reassign on mismatch)"]
```

- **Identity**: Each device generates an X25519 keypair at first run. The `DeviceId` is derived from the public key (SHA-256, first 16 bytes).
- **Key exchange**: On TCP connection, both peers exchange public keys and derive a shared secret via X25519 Diffie-Hellman.
- **Session encryption**: All frames after handshake are encrypted with ChaCha20-Poly1305 AEAD using the shared session key. Per-message nonce (counter) prevents replay.
- **Chunk integrity**: Each chunk carries a SHA-256 hash. The receiver verifies before accepting; mismatches trigger Nack and reassignment.
- **No central server**: Discovery is LAN-only (multicast TTL=1). No data leaves the local network except normal WAN traffic through each device's own internet connection.

## Cross-references

- **Protocol wire format**: [docs/PROTOCOL.md](PROTOCOL.md) — Message types, encoding, discovery, handshake
- **API reference**: [docs/API.md](API.md) — pea-core types and methods for platform authors
- **pea-core README**: [pea-core/README.md](../pea-core/README.md) — Build, test, C FFI, cross-compilation
- **Task breakdown**: [.tasks/README.md](../.tasks/README.md) — Implementation checklists
