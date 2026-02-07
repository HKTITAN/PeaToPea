//! PeaPod protocol reference implementation.
//! Host-driven: no I/O; host passes events and receives actions.
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

pub use identity::{DeviceId, Keypair, PublicKey};
pub use protocol::{Message, PROTOCOL_VERSION};
pub use wire::{decode_frame, encode_frame, FrameDecodeError, FrameEncodeError};
pub use core::{Action, ActiveUpload, ChunkError, MessageError, OutboundAction, PeaPodCore,
    RequestMetadata, UploadAction, is_eligible, DEFAULT_CHUNK_TIMEOUT_TICKS};
pub use chunk::ChunkId;
pub use integrity::{PeerTrustTracker, DEFAULT_MAX_INTEGRITY_FAILURES};
pub use scheduler::{PeerMetrics, PeerMetricsTracker, SLOW_PEER_FAILURE_THRESHOLD};

// Stub modules for chunk manager, scheduler, integrity (full impl later).
pub mod chunk;
pub mod scheduler;
pub mod integrity;
pub mod core;
