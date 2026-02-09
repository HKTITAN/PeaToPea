//! PeaPod protocol reference implementation.
//! Host-driven: no I/O; host passes events and receives actions.
//!
//! Wire format and discovery are specified in **PROTOCOL.md** in the repo (`docs/PROTOCOL.md`).
//!
//! ## Host responsibilities
//!
//! - **I/O**: The host performs all actual I/O: sockets, discovery (e.g. UDP multicast),
//!   system proxy or VPN to intercept traffic. The core is pure logic and crypto; it never
//!   opens sockets or files.
//! - **Request metadata**: The host parses incoming requests (URL, range, method) and passes
//!   metadata to the core. The host executes WAN HTTP range requests and injects chunk data
//!   into the core via `on_chunk_received`; the core returns reassembled segments for the host
//!   to pass to the application.
//! - **Transport**: The host sends core-generated messages (e.g. `encode_frame(Message)`) to
//!   peers over the local transport (TCP or other); it receives bytes from peers, decodes
//!   frames, and passes decoded messages to the core via `on_message_received` (when implemented).

pub mod identity;
pub mod protocol;
pub mod wire;

/// C ABI for staticlib linking (Android NDK, etc.).
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub mod ffi;

pub use chunk::ChunkId;
pub use core::{
    Action, ChunkError, ChunkReceiveOutcome, Config, OnMessageError, OutboundAction, PeaPodCore,
    PeerMetrics,
};
pub use identity::{DeviceId, Keypair, PublicKey};
pub use protocol::{Message, PROTOCOL_VERSION};
pub use wire::{decode_frame, encode_frame, FrameDecodeError, FrameEncodeError};

// Stub modules for chunk manager, scheduler, integrity (full impl later).
pub mod chunk;
pub mod core;
pub mod integrity;
pub mod scheduler;
