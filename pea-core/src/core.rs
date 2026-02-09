//! Host-driven API: PeaPodCore receives events from host, returns actions.

use std::collections::HashMap;
use std::sync::Arc;

use crate::chunk::{self, ChunkId, TransferState, DEFAULT_CHUNK_SIZE};
use crate::identity::{derive_session_key, DeviceId, Keypair, PublicKey};
use crate::protocol::{Message, PROTOCOL_VERSION};
use crate::scheduler;
use crate::wire;
use crate::wire::FrameDecodeError;

const HEARTBEAT_TIMEOUT_TICKS: u64 = 5;

/// Configuration for timeouts and peer trust (optional; use defaults when not set).
#[derive(Clone, Debug, Default)]
pub struct Config {}

/// Optional per-peer metrics for scheduler weighting.
#[derive(Clone, Debug, Default)]
pub struct PeerMetrics {
    /// Estimated bandwidth in bytes per second; higher gives more chunks.
    pub bandwidth_bytes_per_sec: Option<u64>,
    /// Latency in milliseconds (for future use).
    pub latency_ms: Option<u32>,
}

/// Stub for upload path (split outbound into chunks; full impl later).
pub fn split_upload_chunks(transfer_id: [u8; 16], data_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    chunk::split_into_chunks(transfer_id, data_len, chunk_size)
}

/// Active transfer: state and assignment.
struct ActiveTransfer {
    state: TransferState,
    assignment: Vec<(ChunkId, DeviceId)>,
}

/// Main coordinator. The host passes events (request metadata, peer join/leave, messages, chunk data);
/// the core returns actions (chunk assignment, messages to send). No I/O inside the core.
pub struct PeaPodCore {
    keypair: Arc<Keypair>,
    peers: Vec<DeviceId>,
    peer_last_tick: HashMap<DeviceId, u64>,
    tick_count: u64,
    active_transfer: Option<ActiveTransfer>,
    /// Optional metrics per peer (and self) for weighted chunk assignment.
    peer_metrics: HashMap<DeviceId, PeerMetrics>,
}

impl PeaPodCore {
    pub fn new() -> Self {
        Self {
            keypair: Arc::new(Keypair::generate()),
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            peer_metrics: HashMap::new(),
        }
    }

    pub fn with_keypair(keypair: Keypair) -> Self {
        Self {
            keypair: Arc::new(keypair),
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            peer_metrics: HashMap::new(),
        }
    }

    /// Same as with_keypair but takes Arc<Keypair> so the host can share the keypair (e.g. with discovery).
    pub fn with_keypair_arc(keypair: Arc<Keypair>) -> Self {
        Self {
            keypair,
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            peer_metrics: HashMap::new(),
        }
    }

    /// Set or update metrics for a peer (or self) for weighted chunk assignment.
    pub fn set_peer_metrics(&mut self, peer_id: DeviceId, metrics: PeerMetrics) {
        self.peer_metrics.insert(peer_id, metrics);
    }

    /// Build weights for the given workers (self first, then peers). Returns None only when
    /// every participant has default weight 1, so that weighted scheduling is used whenever
    /// any participant (including self) has a non-default bandwidth.
    fn worker_weights(&self, workers: &[DeviceId]) -> Option<Vec<u64>> {
        let weights: Vec<u64> = workers
            .iter()
            .map(|id| {
                self.peer_metrics
                    .get(id)
                    .and_then(|m| m.bandwidth_bytes_per_sec)
                    .unwrap_or(1)
            })
            .collect();
        if weights.iter().all(|&w| w == 1) {
            return None;
        }
        Some(weights)
    }

    /// This device's 16-byte ID (used in discovery and as "self" in assignments).
    pub fn device_id(&self) -> DeviceId {
        self.keypair.device_id()
    }

    /// Build discovery beacon frame (length-prefix + bincode Beacon) for the host to send via UDP. Same format as 07.
    pub fn beacon_frame(&self, listen_port: u16) -> Result<Vec<u8>, wire::FrameEncodeError> {
        let beacon = Message::Beacon {
            protocol_version: PROTOCOL_VERSION,
            device_id: self.keypair.device_id(),
            public_key: self.keypair.public_key().clone(),
            listen_port,
        };
        wire::encode_frame(&beacon)
    }

    /// Build DiscoveryResponse frame (sent to beacon sender). Same wire shape, different variant.
    pub fn discovery_response_frame(
        &self,
        listen_port: u16,
    ) -> Result<Vec<u8>, wire::FrameEncodeError> {
        let resp = Message::DiscoveryResponse {
            protocol_version: PROTOCOL_VERSION,
            device_id: self.keypair.device_id(),
            public_key: self.keypair.public_key().clone(),
            listen_port,
        };
        wire::encode_frame(&resp)
    }

    /// Handshake bytes for local transport: 1 version + 16 device_id + 32 public_key.
    pub fn handshake_bytes(&self) -> [u8; 49] {
        let mut out = [0u8; 49];
        out[0] = PROTOCOL_VERSION;
        out[1..17].copy_from_slice(self.keypair.device_id().as_bytes());
        out[17..49].copy_from_slice(self.keypair.public_key().as_bytes());
        out
    }

    /// Session key for a peer (from shared secret with peer's public key).
    pub fn session_key(&self, peer_public: &PublicKey) -> [u8; 32] {
        derive_session_key(&self.keypair.shared_secret(peer_public))
    }

    /// Called when the host has an eligible request. Returns [`Action::Accelerate`] with chunk assignment
    /// (host then fetches self chunks and sends ChunkRequest to peers) or [`Action::Fallback`].
    pub fn on_incoming_request(&mut self, _url: &str, range: Option<(u64, u64)>) -> Action {
        let total_length = range
            .map(|(s, e)| e.saturating_sub(s).saturating_add(1))
            .unwrap_or(0);
        if total_length == 0 {
            return Action::Fallback;
        }
        if self.peers.is_empty() {
            return Action::Fallback;
        }
        let transfer_id: [u8; 16] = uuid::Uuid::new_v4().into_bytes();
        let chunk_ids = chunk::split_into_chunks(transfer_id, total_length, DEFAULT_CHUNK_SIZE);
        let workers: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
            .chain(self.peers.iter().copied())
            .collect();
        let weights = self.worker_weights(&workers);
        let assignment =
            scheduler::assign_chunks_to_peers_weighted(&chunk_ids, &workers, weights.as_deref());
        let state = TransferState::new(transfer_id, total_length, chunk_ids.clone());
        self.active_transfer = Some(ActiveTransfer {
            state,
            assignment: assignment.clone(),
        });
        Action::Accelerate {
            transfer_id,
            total_length,
            assignment,
        }
    }

    /// Process received chunk. Returns `Ok(Some(body))` when the transfer is complete and reassembled,
    /// `Ok(None)` when still in progress, or `Err(ChunkError)` on integrity failure or unknown transfer.
    pub fn on_chunk_received(
        &mut self,
        transfer_id: [u8; 16],
        start: u64,
        end: u64,
        hash: [u8; 32],
        payload: Vec<u8>,
    ) -> Result<Option<Vec<u8>>, ChunkError> {
        let active = match &mut self.active_transfer {
            Some(a) if a.state.transfer_id == transfer_id => a,
            _ => return Err(ChunkError::UnknownTransfer),
        };
        match chunk::on_chunk_data_received(
            &mut active.state,
            transfer_id,
            start,
            end,
            hash,
            payload,
        ) {
            chunk::ChunkReceiveResult::Complete(bytes) => {
                self.active_transfer = None;
                Ok(Some(bytes))
            }
            chunk::ChunkReceiveResult::InProgress => Ok(None),
            chunk::ChunkReceiveResult::IntegrityFailed => Err(ChunkError::IntegrityFailed),
        }
    }

    /// Notify that a peer joined (from discovery). Updates peer list for chunk assignment.
    pub fn on_peer_joined(&mut self, peer_id: DeviceId, _public_key: &PublicKey) {
        if !self.peers.contains(&peer_id) {
            self.peers.push(peer_id);
        }
        self.peer_last_tick.insert(peer_id, self.tick_count);
    }

    /// Notify that a peer left. Redistributes its chunks to remaining peers; returns actions to send ChunkRequests.
    pub fn on_peer_left(&mut self, peer_id: DeviceId) -> Vec<OutboundAction> {
        self.peers.retain(|p| *p != peer_id);
        self.peer_last_tick.remove(&peer_id);
        self.redistribute_peer_chunks(peer_id)
    }

    /// Call when host receives a heartbeat from peer (so we don't mark peer as left).
    pub fn on_heartbeat_received(&mut self, peer_id: DeviceId) {
        self.peer_last_tick.insert(peer_id, self.tick_count);
    }

    /// Periodic tick: check heartbeat timeouts (treat overdue peers as left), produce heartbeat messages.
    /// Periodic tick (e.g. every 1 s). Returns outbound actions (e.g. heartbeats); host sends them to peers.
    pub fn tick(&mut self) -> Vec<OutboundAction> {
        self.tick_count = self.tick_count.saturating_add(1);
        let mut actions = Vec::new();
        let overdue: Vec<DeviceId> = self
            .peer_last_tick
            .iter()
            .filter(|(_, &t)| self.tick_count.saturating_sub(t) > HEARTBEAT_TIMEOUT_TICKS)
            .map(|(&p, _)| p)
            .collect();
        for peer_id in overdue {
            self.peers.retain(|p| *p != peer_id);
            self.peer_last_tick.remove(&peer_id);
            actions.extend(self.redistribute_peer_chunks(peer_id));
        }
        let self_id = self.keypair.device_id();
        for &peer in &self.peers {
            let msg = Message::Heartbeat { device_id: self_id };
            if let Ok(bytes) = wire::encode_frame(&msg) {
                actions.push(OutboundAction::SendMessage(peer, bytes));
            }
        }
        actions
    }

    fn redistribute_peer_chunks(&mut self, peer_left: DeviceId) -> Vec<OutboundAction> {
        let active = match &mut self.active_transfer {
            Some(a) => a,
            None => return vec![],
        };
        let remaining: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
            .chain(self.peers.iter().copied())
            .collect();
        let new_assignments =
            scheduler::reassign_after_peer_left(&active.assignment, peer_left, &remaining);
        active.assignment.retain(|(_, p)| *p != peer_left);
        let mut actions = Vec::new();
        for (chunk_id, new_peer) in new_assignments {
            active.assignment.push((chunk_id, new_peer));
            let msg = chunk::chunk_request_message(chunk_id, None);
            if let Ok(bytes) = wire::encode_frame(&msg) {
                actions.push(OutboundAction::SendMessage(new_peer, bytes));
            }
        }
        actions
    }

    /// Get current assignment for the active transfer (for host to issue ChunkRequests). Returns (chunk_id, peer_id) list.
    pub fn current_assignment(&self) -> Option<Vec<(ChunkId, DeviceId)>> {
        self.active_transfer.as_ref().map(|a| a.assignment.clone())
    }

    /// Process a received message (host decrypts and passes frame bytes).
    /// Returns (outbound actions, optional completed transfer body when ChunkData completes the transfer).
    #[allow(clippy::type_complexity)]
    pub fn on_message_received(
        &mut self,
        peer_id: DeviceId,
        frame_bytes: &[u8],
    ) -> Result<(Vec<OutboundAction>, Option<([u8; 16], Vec<u8>)>), OnMessageError> {
        let (msg, _) = wire::decode_frame(frame_bytes).map_err(OnMessageError::Decode)?;
        let mut actions = Vec::new();
        let mut completed = None;
        match msg {
            Message::Heartbeat { .. } => {
                self.on_heartbeat_received(peer_id);
            }
            Message::Leave { device_id } => {
                if device_id == peer_id {
                    actions.extend(self.on_peer_left(peer_id));
                }
            }
            Message::ChunkData {
                transfer_id,
                start,
                end,
                hash,
                payload,
            } => match self.on_chunk_received(transfer_id, start, end, hash, payload) {
                Ok(Some(body)) => completed = Some((transfer_id, body)),
                Ok(None) => {}
                Err(ChunkError::IntegrityFailed) => {
                    let chunk_id = ChunkId {
                        transfer_id,
                        start,
                        end,
                    };
                    actions.extend(self.reassign_single_chunk(chunk_id));
                }
                Err(ChunkError::UnknownTransfer) => {}
            },
            Message::Nack {
                transfer_id,
                start,
                end,
            } => {
                let chunk_id = ChunkId {
                    transfer_id,
                    start,
                    end,
                };
                actions.extend(self.reassign_single_chunk(chunk_id));
            }
            Message::Beacon { .. }
            | Message::DiscoveryResponse { .. }
            | Message::Join { .. }
            | Message::ChunkRequest { .. } => {}
        }
        Ok((actions, completed))
    }

    /// Reassign one chunk (e.g. after Nack or integrity failure). Returns ChunkRequest(s) to new peer(s).
    fn reassign_single_chunk(&mut self, chunk_id: ChunkId) -> Vec<OutboundAction> {
        let mut actions = Vec::new();
        let active = match &mut self.active_transfer {
            Some(a) => a,
            None => return actions,
        };
        let old_peer = active
            .assignment
            .iter()
            .find(|(c, _)| *c == chunk_id)
            .map(|(_, p)| *p);
        let Some(peer_left) = old_peer else {
            return actions;
        };
        let remaining: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
            .chain(self.peers.iter().copied())
            .filter(|&p| p != peer_left)
            .collect();
        if remaining.is_empty() {
            return actions;
        }
        let to_reassign = [chunk_id];
        let new_assignments = scheduler::assign_chunks_to_peers(&to_reassign, &remaining);
        active.assignment.retain(|(c, _)| *c != chunk_id);
        for (c, new_peer) in new_assignments {
            active.assignment.push((c, new_peer));
            let msg = chunk::chunk_request_message(c, None);
            if let Ok(bytes) = wire::encode_frame(&msg) {
                actions.push(OutboundAction::SendMessage(new_peer, bytes));
            }
        }
        actions
    }
}

/// Error when processing a received message (e.g. frame decode failure).
#[derive(Debug, thiserror::Error)]
pub enum OnMessageError {
    #[error("decode: {0}")]
    Decode(#[from] FrameDecodeError),
}

impl Default for PeaPodCore {
    fn default() -> Self {
        Self::new()
    }
}

/// Error from `on_chunk_received`: unknown transfer or integrity check failed.
#[derive(Debug, thiserror::Error)]
pub enum ChunkError {
    #[error("unknown transfer")]
    UnknownTransfer,
    #[error("integrity check failed")]
    IntegrityFailed,
}

/// Outcome of processing a received chunk: result and any outbound actions (e.g. reassign on failure).
#[derive(Debug)]
pub struct ChunkReceiveOutcome {
    pub result: Result<Option<Vec<u8>>, ChunkError>,
    pub actions: Vec<OutboundAction>,
}

/// Result of `on_incoming_request`: accelerate (with chunk assignment) or fall back to normal path.
pub enum Action {
    /// Core produced a chunk plan; host fetches self chunks via WAN and sends ChunkRequest to peers.
    Accelerate {
        transfer_id: [u8; 16],
        total_length: u64,
        assignment: Vec<(ChunkId, DeviceId)>,
    },
    /// Do not accelerate; host forwards the request normally.
    Fallback,
}

/// Instruction for the host: send a message to a peer (e.g. ChunkRequest, Heartbeat, Leave).
#[derive(Debug)]
pub enum OutboundAction {
    /// Send the given bytes to the peer over the local transport (host encrypts if required).
    SendMessage(DeviceId, Vec<u8>),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::split_into_chunks;
    use crate::integrity;

    #[test]
    fn integration_request_then_receive_chunks() {
        let kp = Keypair::generate();
        let mut core = PeaPodCore::with_keypair(kp);
        let peer_id = Keypair::generate().device_id();
        core.on_peer_joined(peer_id, &Keypair::generate().public_key().clone());

        let range = (0u64, 99u64);
        let total = 100u64;
        let action = core.on_incoming_request("http://example.com/file", Some(range));
        let transfer_id = match &action {
            Action::Accelerate {
                transfer_id,
                total_length,
                assignment: _,
            } => {
                assert_eq!(*total_length, total);
                *transfer_id
            }
            Action::Fallback => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, total, crate::chunk::DEFAULT_CHUNK_SIZE);
        for &chunk_id in &chunk_ids {
            let payload: Vec<u8> = (chunk_id.start..chunk_id.end).map(|j| j as u8).collect();
            let hash = integrity::hash_chunk(&payload);
            let r =
                core.on_chunk_received(transfer_id, chunk_id.start, chunk_id.end, hash, payload);
            if let Ok(Some(bytes)) = r {
                assert_eq!(bytes.len(), 100);
                for (j, &b) in bytes.iter().enumerate() {
                    assert_eq!(b, j as u8);
                }
                return;
            }
        }
        panic!("transfer should complete after receiving all chunks");
    }
}
