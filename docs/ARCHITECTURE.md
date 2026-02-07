# PeaPod Architecture

## Layer placement

PeaPod sits **above IP** and **below applications**:

```
Application (browser, app)
        ↓
    TCP/HTTP
        ↓
PeaPod cooperative layer (intercept, schedule, chunk, transport, integrity)
        ↓
Platform implementation (proxy, VPN, Network Extension)
        ↓
Network interfaces (WAN + LAN)
```

Traffic is intercepted by the platform implementation (e.g. system proxy or VPNService). Eligible flows are handed to the cooperative layer, which uses pea-core for protocol logic. Chunks are fetched or sent via WAN and exchanged over the local link; the requesting device reassembles and delivers the result to the app.

## Core components

- **Discovery** — Advertise presence and discover peers (LAN multicast/broadcast; same wire format on all platforms).
- **Identity & encryption** — Device keypairs, device ID, session keys; encrypt all control and chunk traffic between peers.
- **Distributed scheduler** — Assign chunks to peers (and self) based on availability and metrics; redistribute when a peer leaves.
- **Chunk manager** — Split transfers into chunks, track state, request and receive chunks, reassemble.
- **Local transport** — TCP between peers on the LAN; framed, encrypted messages (pea-core defines format; platform does I/O).
- **Integrity verification** — Per-chunk hash; verify on receive; reject and reassign on failure.
- **Failure recovery** — Timeouts, heartbeat, redistribution when peers leave or stall.

## pea-core: host-driven, no I/O

The shared library **pea-core** contains no sockets, no file I/O, and no platform APIs. The host (each protocol implementation) is responsible for:

- Discovery (sending and receiving beacons).
- Accepting and parsing incoming requests (e.g. HTTP with range).
- Sending and receiving framed messages to peers over TCP.
- Executing WAN requests for chunks assigned to self.
- Calling into pea-core with events (request metadata, peer joined/left, message received, chunk data received) and acting on returned values (chunk assignments, messages to send, reassembled stream).

This keeps pea-core portable and testable with mock hosts.

## Data flow (download)

1. App issues request → platform intercepts → host parses (URL, range).
2. Host calls pea-core with request metadata → core returns chunk plan (which chunk from which peer or self).
3. Host: for each chunk, either fetch via WAN (self) or send chunk request to peer and wait for chunk data.
4. Host feeds chunk data to pea-core → core verifies and reassembles → returns stream segment to host.
5. Host delivers reassembled response to app.

Upload flow is analogous: host gives outbound data to core; core splits and assigns; peers upload their portions via their WAN; core verifies and signals completion.
