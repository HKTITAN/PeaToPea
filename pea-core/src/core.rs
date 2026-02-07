//! Host-driven API: PeaPodCore receives events from host, returns actions.

use std::collections::HashMap;

use crate::chunk::{self, ChunkId, TransferState, DEFAULT_CHUNK_SIZE};
use crate::identity::{self, DeviceId, Keypair, PublicKey};
use crate::protocol::Message;
use crate::scheduler::{self, PeerMetrics};
use crate::wire;

const HEARTBEAT_TIMEOUT_TICKS: u64 = 5;

/// Default timeout for chunk requests in ticks.
pub const DEFAULT_CHUNK_TIMEOUT_TICKS: u64 = 30;

/// Default max integrity failures before isolating a peer.
pub const DEFAULT_MAX_INTEGRITY_FAILURES: u64 = 3;

/// Stub for upload path (split outbound into chunks; full impl later).
pub fn split_upload_chunks(transfer_id: [u8; 16], data_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    chunk::split_into_chunks(transfer_id, data_len, chunk_size)
}

/// Active transfer: state and assignment.
struct ActiveTransfer {
    state: TransferState,
    assignment: Vec<(ChunkId, DeviceId)>,
    /// Tick when each in-flight chunk was requested (for timeout).
    request_ticks: HashMap<ChunkId, u64>,
    /// Whether this is an upload transfer.
    #[allow(dead_code)]
    is_upload: bool,
}

/// Main coordinator. Host passes events; core returns actions.
pub struct PeaPodCore {
    keypair: Keypair,
    peers: Vec<DeviceId>,
    peer_keys: HashMap<DeviceId, PublicKey>,
    peer_last_tick: HashMap<DeviceId, u64>,
    peer_metrics: HashMap<DeviceId, PeerMetrics>,
    tick_count: u64,
    active_transfer: Option<ActiveTransfer>,
    chunk_timeout_ticks: u64,
    max_integrity_failures: u64,
}

impl PeaPodCore {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::generate(),
            peers: Vec::new(),
            peer_keys: HashMap::new(),
            peer_last_tick: HashMap::new(),
            peer_metrics: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            chunk_timeout_ticks: DEFAULT_CHUNK_TIMEOUT_TICKS,
            max_integrity_failures: DEFAULT_MAX_INTEGRITY_FAILURES,
        }
    }

    pub fn with_keypair(keypair: Keypair) -> Self {
        Self {
            keypair,
            peers: Vec::new(),
            peer_keys: HashMap::new(),
            peer_last_tick: HashMap::new(),
            peer_metrics: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            chunk_timeout_ticks: DEFAULT_CHUNK_TIMEOUT_TICKS,
            max_integrity_failures: DEFAULT_MAX_INTEGRITY_FAILURES,
        }
    }

    /// Set custom chunk request timeout in ticks.
    pub fn set_chunk_timeout(&mut self, ticks: u64) {
        self.chunk_timeout_ticks = ticks;
    }

    /// Set max integrity failures before isolating a peer.
    pub fn set_max_integrity_failures(&mut self, n: u64) {
        self.max_integrity_failures = n;
    }

    pub fn device_id(&self) -> DeviceId {
        self.keypair.device_id()
    }

    /// Check whether a request is eligible for acceleration.
    /// Eligible: HTTP/HTTPS with range support (range must be provided and non-zero).
    /// Ineligible: no range, unknown protocol, or explicitly ineligible.
    pub fn is_eligible(url: &str, range: Option<(u64, u64)>, ineligible: bool) -> bool {
        if ineligible {
            return false;
        }
        // Must have a non-zero range to split into chunks.
        let total = range
            .map(|(s, e)| e.saturating_sub(s).saturating_add(1))
            .unwrap_or(0);
        if total == 0 {
            return false;
        }
        // Only HTTP/HTTPS URLs are eligible.
        let lower = url.to_ascii_lowercase();
        lower.starts_with("http://") || lower.starts_with("https://")
    }

    /// On incoming request (URL, optional range). Returns Accelerate with plan or Fallback.
    pub fn on_incoming_request(&mut self, url: &str, range: Option<(u64, u64)>) -> Action {
        if !Self::is_eligible(url, range, false) {
            return Action::Fallback;
        }
        let total_length = range
            .map(|(s, e)| e.saturating_sub(s).saturating_add(1))
            .unwrap_or(0);
        if self.peers.is_empty() {
            return Action::Fallback;
        }
        let transfer_id: [u8; 16] = uuid::Uuid::new_v4().into_bytes();
        let chunk_ids = chunk::split_into_chunks(transfer_id, total_length, DEFAULT_CHUNK_SIZE);
        let workers: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
            .chain(self.peers.iter().copied())
            .collect();
        let assignment = scheduler::assign_chunks_with_metrics(
            &chunk_ids,
            &workers,
            &self.peer_metrics,
            self.max_integrity_failures,
        );
        let state = TransferState::new(transfer_id, total_length, chunk_ids.clone());
        self.active_transfer = Some(ActiveTransfer {
            state,
            assignment: assignment.clone(),
            request_ticks: HashMap::new(),
            is_upload: false,
        });
        Action::Accelerate {
            transfer_id,
            total_length,
            assignment,
        }
    }

    /// On incoming upload request: split outbound data into chunks for peers.
    pub fn on_upload_request(&mut self, url: &str, data_len: u64) -> Action {
        if !Self::is_eligible(url, Some((0, data_len.saturating_sub(1))), false) {
            return Action::Fallback;
        }
        if self.peers.is_empty() {
            return Action::Fallback;
        }
        let transfer_id: [u8; 16] = uuid::Uuid::new_v4().into_bytes();
        let chunk_ids = chunk::split_into_chunks(transfer_id, data_len, DEFAULT_CHUNK_SIZE);
        let workers: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
            .chain(self.peers.iter().copied())
            .collect();
        let assignment = scheduler::assign_chunks_with_metrics(
            &chunk_ids,
            &workers,
            &self.peer_metrics,
            self.max_integrity_failures,
        );
        let state = TransferState::new(transfer_id, data_len, chunk_ids.clone());
        self.active_transfer = Some(ActiveTransfer {
            state,
            assignment: assignment.clone(),
            request_ticks: HashMap::new(),
            is_upload: true,
        });
        Action::Accelerate {
            transfer_id,
            total_length: data_len,
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
        // Find which peer sent this chunk (for metrics).
        let chunk_id = ChunkId {
            transfer_id,
            start,
            end,
        };
        let sender = self.active_transfer.as_ref().and_then(|a| {
            a.assignment
                .iter()
                .find(|(c, _)| *c == chunk_id)
                .map(|(_, p)| *p)
        });

        let active = match &mut self.active_transfer {
            Some(a) if a.state.transfer_id == transfer_id => a,
            _ => return Err(ChunkError::UnknownTransfer),
        };
        active.request_ticks.remove(&chunk_id);
        match chunk::on_chunk_data_received(
            &mut active.state,
            transfer_id,
            start,
            end,
            hash,
            payload,
        ) {
            chunk::ChunkReceiveResult::Complete(bytes) => {
                if let Some(peer) = sender {
                    self.peer_metrics.entry(peer).or_default().record_success();
                }
                self.active_transfer = None;
                Ok(Some(bytes))
            }
            chunk::ChunkReceiveResult::InProgress => {
                if let Some(peer) = sender {
                    self.peer_metrics.entry(peer).or_default().record_success();
                }
                Ok(None)
            }
            chunk::ChunkReceiveResult::IntegrityFailed => {
                if let Some(peer) = sender {
                    self.peer_metrics.entry(peer).or_default().record_failure();
                }
                Err(ChunkError::IntegrityFailed)
            }
        }
    }

    /// Peer joined. Update peer list and last-seen.
    pub fn on_peer_joined(&mut self, peer_id: DeviceId, public_key: &PublicKey) {
        if !self.peers.contains(&peer_id) {
            self.peers.push(peer_id);
        }
        self.peer_keys.insert(peer_id, public_key.clone());
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

    /// Process a received wire message: decrypt (if session key available), parse, update state,
    /// return optional response messages and/or chunk requests.
    pub fn on_message_received(
        &mut self,
        peer_id: DeviceId,
        bytes: &[u8],
    ) -> Result<Vec<OutboundAction>, MessageError> {
        // Try decryption if we have a session key for this peer.
        let plaintext = if let Some(peer_pub) = self.peer_keys.get(&peer_id) {
            let shared = self.keypair.shared_secret(peer_pub);
            let session_key = identity::derive_session_key(&shared);
            match identity::decrypt_wire(&session_key, 0, bytes) {
                Ok(plain) => plain,
                Err(_) => bytes.to_vec(), // Fall back to unencrypted parse.
            }
        } else {
            bytes.to_vec()
        };

        let (msg, _consumed) =
            wire::decode_frame(&plaintext).map_err(|_| MessageError::DecodeFailed)?;

        let mut actions = Vec::new();

        match msg {
            Message::Beacon {
                device_id,
                public_key,
                ..
            } => {
                self.on_peer_joined(device_id, &public_key);
                let resp = Message::DiscoveryResponse {
                    protocol_version: crate::protocol::PROTOCOL_VERSION,
                    device_id: self.keypair.device_id(),
                    public_key: self.keypair.public_key().clone(),
                    listen_port: 0,
                };
                if let Ok(frame) = wire::encode_frame(&resp) {
                    actions.push(OutboundAction::SendMessage(peer_id, frame));
                }
            }
            Message::DiscoveryResponse {
                device_id,
                public_key,
                ..
            } => {
                self.on_peer_joined(device_id, &public_key);
            }
            Message::Join { device_id } => {
                if !self.peers.contains(&device_id) {
                    self.peers.push(device_id);
                    self.peer_last_tick.insert(device_id, self.tick_count);
                }
            }
            Message::Leave { device_id } => {
                actions.extend(self.on_peer_left(device_id));
            }
            Message::Heartbeat { device_id } => {
                self.on_heartbeat_received(device_id);
            }
            Message::ChunkRequest {
                transfer_id,
                start,
                end,
            } => {
                // Peer is requesting a chunk from us. If we have the data in our active
                // transfer state, send ChunkData back.
                if let Some(active) = &self.active_transfer {
                    let chunk_id = ChunkId {
                        transfer_id,
                        start,
                        end,
                    };
                    if active.state.is_received(&chunk_id) {
                        // We have this chunk; the data was already verified and stored.
                        // In a full impl, we'd read the payload from state. For now, respond with NACK
                        // since we don't store raw payloads for re-sending.
                    }
                }
            }
            Message::ChunkData {
                transfer_id,
                start,
                end,
                hash,
                payload,
            } => {
                match self.on_chunk_received(transfer_id, start, end, hash, payload) {
                    Ok(Some(bytes)) => {
                        actions.push(OutboundAction::TransferComplete(transfer_id, bytes));
                    }
                    Ok(None) => {} // Still in progress.
                    Err(ChunkError::IntegrityFailed) => {
                        let nack = Message::Nack {
                            transfer_id,
                            start,
                            end,
                        };
                        if let Ok(frame) = wire::encode_frame(&nack) {
                            actions.push(OutboundAction::SendMessage(peer_id, frame));
                        }
                    }
                    Err(_) => {}
                }
            }
            Message::Nack {
                transfer_id,
                start,
                end,
            } => {
                // Peer reported failure for a chunk; reassign.
                let chunk_id = ChunkId {
                    transfer_id,
                    start,
                    end,
                };
                if let Some(active) = &mut self.active_transfer {
                    active.state.mark_failed(chunk_id);
                    active.request_ticks.remove(&chunk_id);
                }
                // Record failure for the peer.
                self.peer_metrics
                    .entry(peer_id)
                    .or_default()
                    .record_failure();
            }
        }

        Ok(actions)
    }

    /// Periodic tick: check heartbeat timeouts, chunk request timeouts, produce heartbeat messages.
    pub fn tick(&mut self) -> Vec<OutboundAction> {
        self.tick_count = self.tick_count.saturating_add(1);
        let mut actions = Vec::new();

        // Check heartbeat timeouts.
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

        // Check chunk request timeouts.
        if let Some(active) = &mut self.active_transfer {
            let timed_out: Vec<(ChunkId, DeviceId)> = active
                .request_ticks
                .iter()
                .filter(|(_, &t)| self.tick_count.saturating_sub(t) > self.chunk_timeout_ticks)
                .filter_map(|(&chunk_id, _)| {
                    active
                        .assignment
                        .iter()
                        .find(|(c, _)| *c == chunk_id)
                        .map(|(_, peer)| (chunk_id, *peer))
                })
                .collect();
            for (chunk_id, peer) in timed_out {
                active.state.mark_failed(chunk_id);
                active.request_ticks.remove(&chunk_id);
                self.peer_metrics.entry(peer).or_default().record_failure();
            }
        }

        // Send heartbeat to all peers.
        let self_id = self.keypair.device_id();
        for &peer in &self.peers {
            let msg = Message::Heartbeat { device_id: self_id };
            if let Ok(bytes) = wire::encode_frame(&msg) {
                actions.push(OutboundAction::SendMessage(peer, bytes));
            }
        }
        actions
    }

    /// Mark a chunk request as sent (start timeout tracking).
    pub fn mark_chunk_requested(&mut self, chunk_id: ChunkId) {
        if let Some(active) = &mut self.active_transfer {
            active.state.mark_in_flight(chunk_id);
            active.request_ticks.insert(chunk_id, self.tick_count);
        }
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

    /// Get per-peer metrics.
    pub fn peer_metrics(&self) -> &HashMap<DeviceId, PeerMetrics> {
        &self.peer_metrics
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

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("failed to decode message")]
    DecodeFailed,
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
    TransferComplete([u8; 16], Vec<u8>),
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

    #[test]
    fn fallback_no_peers() {
        let mut core = PeaPodCore::new();
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn fallback_no_range() {
        let mut core = PeaPodCore::new();
        let peer_id = Keypair::generate().device_id();
        core.on_peer_joined(peer_id, &Keypair::generate().public_key().clone());
        let action = core.on_incoming_request("http://example.com/file", None);
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn fallback_ineligible_url() {
        let mut core = PeaPodCore::new();
        let peer_id = Keypair::generate().device_id();
        core.on_peer_joined(peer_id, &Keypair::generate().public_key().clone());
        let action = core.on_incoming_request("ftp://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn eligibility_checks() {
        assert!(PeaPodCore::is_eligible(
            "http://example.com/file",
            Some((0, 99)),
            false
        ));
        assert!(PeaPodCore::is_eligible(
            "https://example.com/file",
            Some((0, 99)),
            false
        ));
        assert!(!PeaPodCore::is_eligible(
            "ftp://example.com/file",
            Some((0, 99)),
            false
        ));
        assert!(!PeaPodCore::is_eligible(
            "http://example.com/file",
            None,
            false
        ));
        assert!(!PeaPodCore::is_eligible(
            "http://example.com/file",
            Some((0, 99)),
            true
        ));
    }

    #[test]
    fn peer_leave_redistributes() {
        let mut core = PeaPodCore::new();
        let peer_a = Keypair::generate();
        let peer_b = Keypair::generate();
        core.on_peer_joined(peer_a.device_id(), peer_a.public_key());
        core.on_peer_joined(peer_b.device_id(), peer_b.public_key());

        // Use enough data to produce multiple chunks so peer_a is guaranteed at least one.
        let total = crate::chunk::DEFAULT_CHUNK_SIZE * 6;
        let action = core.on_incoming_request("http://example.com/file", Some((0, total - 1)));
        assert!(matches!(action, Action::Accelerate { .. }));

        let actions = core.on_peer_left(peer_a.device_id());
        // With 6 chunks across 3 workers (round-robin), peer_a gets chunks 1, 4 (indices 1, 4).
        // After leaving, those chunks are reassigned, producing outbound messages.
        assert!(!actions.is_empty());
    }

    #[test]
    fn heartbeat_timeout_removes_peer() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        // Tick enough times past the heartbeat timeout.
        for _ in 0..7 {
            core.tick();
        }

        // Peer should be removed; new request should fallback.
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn heartbeat_received_prevents_timeout() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        for _ in 0..4 {
            core.tick();
            core.on_heartbeat_received(peer.device_id());
        }

        // Peer should still be alive.
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Accelerate { .. }));
    }

    #[test]
    fn integrity_failure_records_metric() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            _ => panic!("expected Accelerate"),
        };

        // Send chunk with bad hash.
        let result = core.on_chunk_received(transfer_id, 0, 100, [0u8; 32], vec![1u8; 100]);
        assert!(matches!(result, Err(ChunkError::IntegrityFailed)));

        // Check metrics recorded the failure.
        let has_failure = core.peer_metrics().values().any(|m| m.failures > 0);
        assert!(has_failure);
    }

    #[test]
    fn upload_request() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        let action = core.on_upload_request("http://example.com/upload", 1000);
        assert!(matches!(action, Action::Accelerate { .. }));
    }

    #[test]
    fn upload_request_fallback_no_peers() {
        let mut core = PeaPodCore::new();
        let action = core.on_upload_request("http://example.com/upload", 1000);
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn on_message_received_beacon() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        let beacon = Message::Beacon {
            protocol_version: crate::protocol::PROTOCOL_VERSION,
            device_id: peer.device_id(),
            public_key: peer.public_key().clone(),
            listen_port: 1234,
        };
        let frame = wire::encode_frame(&beacon).unwrap();
        let actions = core.on_message_received(peer.device_id(), &frame).unwrap();
        // Should send a DiscoveryResponse.
        assert!(!actions.is_empty());
    }

    #[test]
    fn on_message_received_heartbeat() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());
        let hb = Message::Heartbeat {
            device_id: peer.device_id(),
        };
        let frame = wire::encode_frame(&hb).unwrap();
        let actions = core.on_message_received(peer.device_id(), &frame).unwrap();
        assert!(actions.is_empty());
    }

    #[test]
    fn on_message_received_leave() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());
        let leave = Message::Leave {
            device_id: peer.device_id(),
        };
        let frame = wire::encode_frame(&leave).unwrap();
        let _actions = core.on_message_received(peer.device_id(), &frame).unwrap();
        // Peer should be removed; new request should fallback.
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn chunk_timeout_marks_failed() {
        let mut core = PeaPodCore::new();
        core.set_chunk_timeout(2);
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            _ => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, 100, crate::chunk::DEFAULT_CHUNK_SIZE);
        core.mark_chunk_requested(chunk_ids[0]);

        // Tick past the timeout.
        for _ in 0..4 {
            core.tick();
        }

        // The chunk timeout should have recorded a failure for one of the workers.
        let total_failures: u64 = core.peer_metrics().values().map(|m| m.failures).sum();
        assert!(
            total_failures > 0,
            "Expected at least one failure after timeout"
        );
    }

    #[test]
    fn on_message_received_chunk_data_completes_transfer() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            _ => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, 100, crate::chunk::DEFAULT_CHUNK_SIZE);
        for &chunk_id in &chunk_ids {
            let payload: Vec<u8> = (chunk_id.start..chunk_id.end).map(|j| j as u8).collect();
            let hash = integrity::hash_chunk(&payload);
            let msg = Message::ChunkData {
                transfer_id,
                start: chunk_id.start,
                end: chunk_id.end,
                hash,
                payload,
            };
            let frame = wire::encode_frame(&msg).unwrap();
            let actions = core.on_message_received(peer.device_id(), &frame).unwrap();
            for action in &actions {
                if let OutboundAction::TransferComplete(tid, bytes) = action {
                    assert_eq!(*tid, transfer_id);
                    assert_eq!(bytes.len(), 100);
                    return;
                }
            }
        }
        panic!("transfer should complete after receiving all chunks via on_message_received");
    }

    #[test]
    fn on_message_received_nack_sends_integrity_failure() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate();
        core.on_peer_joined(peer.device_id(), peer.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            _ => panic!("expected Accelerate"),
        };

        // Send bad ChunkData via on_message_received.
        let msg = Message::ChunkData {
            transfer_id,
            start: 0,
            end: 100,
            hash: [0u8; 32],
            payload: vec![1u8; 100],
        };
        let frame = wire::encode_frame(&msg).unwrap();
        let actions = core.on_message_received(peer.device_id(), &frame).unwrap();
        // Should contain a NACK response.
        assert!(actions
            .iter()
            .any(|a| matches!(a, OutboundAction::SendMessage(_, _))));
    }
}
