//! Host-driven API: PeaPodCore receives events from host, returns actions.

use std::collections::HashMap;

use crate::chunk::{self, ChunkId, TransferState, DEFAULT_CHUNK_SIZE};
use crate::identity::{DeviceId, Keypair, PublicKey};
use crate::protocol::Message;
use crate::scheduler;
use crate::wire;

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

/// Main coordinator. Host passes events; core returns actions.
pub struct PeaPodCore {
    keypair: Keypair,
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
            keypair: Keypair::generate(),
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            peer_metrics: HashMap::new(),
        }
    }

    pub fn with_keypair(keypair: Keypair) -> Self {
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

    pub fn device_id(&self) -> DeviceId {
        self.keypair.device_id()
    }

    /// On incoming request (URL, optional range). Returns Accelerate with plan or Fallback.
    pub fn on_incoming_request(
        &mut self,
        _url: &str,
        range: Option<(u64, u64)>,
    ) -> Action {
        let total_length = range.map(|(s, e)| e.saturating_sub(s).saturating_add(1)).unwrap_or(0);
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
        let assignment = scheduler::assign_chunks_to_peers_weighted(&chunk_ids, &workers, weights.as_deref());
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

    /// Process received ChunkData. Returns Ok(Some(reassembled_bytes)) when transfer complete, Ok(None) when in progress, Err on integrity failure.
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
        match chunk::on_chunk_data_received(&mut active.state, transfer_id, start, end, hash, payload) {
            chunk::ChunkReceiveResult::Complete(bytes) => {
                self.active_transfer = None;
                Ok(Some(bytes))
            }
            chunk::ChunkReceiveResult::InProgress => Ok(None),
            chunk::ChunkReceiveResult::IntegrityFailed => Err(ChunkError::IntegrityFailed),
        }
    }

    /// Peer joined. Update peer list and last-seen.
    pub fn on_peer_joined(&mut self, peer_id: DeviceId, _public_key: &PublicKey) {
        if !self.peers.contains(&peer_id) {
            self.peers.push(peer_id);
        }
        self.peer_last_tick.insert(peer_id, self.tick_count);
    }

    /// Peer left. Redistribute its chunks and return outbound actions (ChunkRequests to new peers).
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
        let new_assignments = scheduler::reassign_after_peer_left(
            &active.assignment,
            peer_left,
            &remaining,
        );
        active.assignment.retain(|(_, p)| *p != peer_left);
        let mut actions = Vec::new();
        for (chunk_id, new_peer) in new_assignments {
            active.assignment.push((chunk_id, new_peer));
            let msg = chunk::chunk_request_message(chunk_id);
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
}

impl Default for PeaPodCore {
    fn default() -> Self {
        Self::new()
    }
}

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

/// Action after host passes request metadata.
pub enum Action {
    Accelerate {
        transfer_id: [u8; 16],
        total_length: u64,
        assignment: Vec<(ChunkId, DeviceId)>,
    },
    Fallback,
}

/// Action for host to perform.
pub enum OutboundAction {
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
            let r = core.on_chunk_received(
                transfer_id,
                chunk_id.start,
                chunk_id.end,
                hash,
                payload,
            );
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