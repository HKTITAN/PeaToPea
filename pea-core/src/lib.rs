//! PeaPod protocol reference implementation.
//! Host-driven: no I/O; host passes events and receives actions.

pub mod identity;
pub mod protocol;
pub mod wire;

pub use identity::{DeviceId, Keypair, PublicKey};
pub use protocol::{Message, PROTOCOL_VERSION};
pub use wire::{decode_frame, encode_frame, FrameDecodeError, FrameEncodeError};
pub use core::{Action, OutboundAction, PeaPodCore};

// Stub modules for chunk manager, scheduler, integrity (full impl later).
pub mod chunk;
pub mod scheduler;
pub mod integrity;
pub mod core;
