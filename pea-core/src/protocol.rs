//! PeaPod wire protocol: message types and version.

use serde::{Deserialize, Serialize};

use crate::identity::{DeviceId, PublicKey};

/// Current protocol version. Used in beacon and handshake.
pub const PROTOCOL_VERSION: u8 = 1;

/// All wire message types. Encoding is bincode; framing is length-prefix (see wire module).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Message {
    /// Discovery: advertise presence. Include device ID, public key, protocol version, optional listen address.
    Beacon {
        protocol_version: u8,
        device_id: DeviceId,
        public_key: PublicKey,
        listen_port: u16,
    },
    /// Response to beacon: ack and advertise self.
    DiscoveryResponse {
        protocol_version: u8,
        device_id: DeviceId,
        public_key: PublicKey,
        listen_port: u16,
    },
    /// Request to join pod or confirm membership.
    Join {
        device_id: DeviceId,
    },
    /// Graceful leave.
    Leave {
        device_id: DeviceId,
    },
    /// Liveness heartbeat.
    Heartbeat {
        device_id: DeviceId,
    },
    /// Request a chunk by transfer ID and range.
    ChunkRequest {
        transfer_id: [u8; 16],
        start: u64,
        end: u64,
    },
    /// Chunk payload: transfer ID, range, hash, data (or encrypted).
    ChunkData {
        transfer_id: [u8; 16],
        start: u64,
        end: u64,
        hash: [u8; 32],
        payload: Vec<u8>,
    },
    /// Chunk failed or peer left; trigger reassignment.
    Nack {
        transfer_id: [u8; 16],
        start: u64,
        end: u64,
    },
}
