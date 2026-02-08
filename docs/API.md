# pea-core API (for platform authors)

Summary of main types and entry points for Windows, Android, Linux, iOS, and macOS. Wire format: [PROTOCOL.md](PROTOCOL.md).

## Main types (Rust)

- **PeaPodCore** — Coordinator. Create with `new()` or `with_keypair_arc(Arc<Keypair>)`.
- **Config** — Optional config; `Config::default()`.
- **Keypair**, **DeviceId**, **PublicKey** — Identity.
- **Action** — From `on_incoming_request`: `Fallback` or `Accelerate { transfer_id, total_length, assignment }`.
- **ChunkId**, **Message** — Chunk id and wire messages; use `encode_frame` / `decode_frame`.
- **OutboundAction** — e.g. `SendMessage(peer, bytes)` from `on_message_received` or `tick`.

## Main methods

- **on_incoming_request(url, range)** → **Action**. Host then fetches self chunks via WAN and sends ChunkRequest to peers.
- **on_chunk_received(transfer_id, start, end, hash, payload)** → **Result<Option<Vec<u8>>, ChunkError>**. `Ok(Some(body))` when complete.
- **on_peer_joined(peer_id, public_key)** / **on_peer_left(peer_id)** → peer list and optional **Vec<OutboundAction>**.
- **on_message_received(peer_id, bytes)** → **Result<(Vec<OutboundAction>, Option<(tid, body)>), OnMessageError>**.
- **tick()** → **Vec<OutboundAction>** (e.g. heartbeats). Call periodically.

Helpers: **beacon_frame(listen_port)**, **discovery_response_frame(listen_port)**, **handshake_bytes()**, **session_key(peer_public)**, **device_id()**.

## C FFI (pea-core/src/ffi.rs)

**pea_core_create** / **pea_core_destroy**; **pea_core_device_id**; **pea_core_beacon_frame**, **pea_core_discovery_response_frame**; **pea_core_on_incoming_request**, **pea_core_on_chunk_received**, **pea_core_on_peer_joined**, **pea_core_on_peer_left**, **pea_core_on_message_received**, **pea_core_tick**. Host provides buffers; core fills or returns length. Use from one thread or serialize access.

## JNI (Android)

Android calls the C ABI via pea_jni.c; Kotlin wrapper (PeaCore.kt) exposes the same logical API. See pea-android for signatures.

## Rust docs

`cargo doc -p pea-core --no-deps --open`
