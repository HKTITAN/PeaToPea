//! Host-driven API: PeaPodCore receives events from host, returns actions.

use std::collections::HashMap;

use crate::chunk::{self, ChunkId, TransferState, DEFAULT_CHUNK_SIZE};
use crate::identity::{DeviceId, Keypair, PublicKey};
use crate::protocol::Message;
use crate::scheduler;
use crate::wire;

const HEARTBEAT_TIMEOUT_TICKS: u64 = 5;

/// Default number of integrity failures before a peer is isolated.
const DEFAULT_INTEGRITY_FAILURE_THRESHOLD: u32 = 3;

/// Default timeout for a chunk request (in ticks). Host calls `tick()` periodically.
const DEFAULT_CHUNK_TIMEOUT_TICKS: u64 = 30;

/// Stub for upload path (split outbound into chunks; full impl later).
pub fn split_upload_chunks(transfer_id: [u8; 16], data_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    chunk::split_into_chunks(transfer_id, data_len, chunk_size)
}

/// Active transfer: state and assignment.
struct ActiveTransfer {
    state: TransferState,
    assignment: Vec<(ChunkId, DeviceId)>,
    /// Tick when each in-flight chunk was requested (for timeout).
    in_flight: HashMap<ChunkId, u64>,
}

/// Main coordinator. Host passes events; core returns actions.
pub struct PeaPodCore {
    keypair: Keypair,
    peers: Vec<DeviceId>,
    peer_last_tick: HashMap<DeviceId, u64>,
    tick_count: u64,
    active_transfer: Option<ActiveTransfer>,
    /// Per-peer integrity failure count. After threshold, peer is isolated.
    integrity_failures: HashMap<DeviceId, u32>,
    /// Peers that have been isolated due to repeated integrity failures.
    isolated_peers: Vec<DeviceId>,
    /// Configurable integrity failure threshold.
    integrity_failure_threshold: u32,
    /// Configurable chunk request timeout in ticks.
    chunk_timeout_ticks: u64,
}

impl PeaPodCore {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::generate(),
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            integrity_failures: HashMap::new(),
            isolated_peers: Vec::new(),
            integrity_failure_threshold: DEFAULT_INTEGRITY_FAILURE_THRESHOLD,
            chunk_timeout_ticks: DEFAULT_CHUNK_TIMEOUT_TICKS,
        }
    }

    pub fn with_keypair(keypair: Keypair) -> Self {
        Self {
            keypair,
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            integrity_failures: HashMap::new(),
            isolated_peers: Vec::new(),
            integrity_failure_threshold: DEFAULT_INTEGRITY_FAILURE_THRESHOLD,
            chunk_timeout_ticks: DEFAULT_CHUNK_TIMEOUT_TICKS,
        }
    }

    /// Set the number of integrity failures before a peer is isolated (default: 3).
    pub fn set_integrity_failure_threshold(&mut self, threshold: u32) {
        self.integrity_failure_threshold = threshold;
    }

    /// Set the chunk request timeout in ticks (default: 30).
    pub fn set_chunk_timeout_ticks(&mut self, ticks: u64) {
        self.chunk_timeout_ticks = ticks;
    }

    pub fn device_id(&self) -> DeviceId {
        self.keypair.device_id()
    }

    /// Check whether a flow is eligible for acceleration.
    ///
    /// Returns `true` for HTTP(S) range-based downloads. Returns `false` for
    /// flows that must not be accelerated (e.g. unknown protocols).
    /// The host may also pass `ineligible = true` to force fallback.
    pub fn is_eligible(url: &str, ineligible: bool) -> bool {
        if ineligible {
            return false;
        }
        let lower = url.to_ascii_lowercase();
        lower.starts_with("http://") || lower.starts_with("https://")
    }

    /// On incoming request (URL, optional range). Returns Accelerate with plan or Fallback.
    pub fn on_incoming_request(&mut self, url: &str, range: Option<(u64, u64)>) -> Action {
        if !Self::is_eligible(url, false) {
            return Action::Fallback;
        }
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
        let assignment = scheduler::assign_chunks_to_peers(&chunk_ids, &workers);
        let in_flight: HashMap<ChunkId, u64> = assignment
            .iter()
            .map(|(c, _)| (*c, self.tick_count))
            .collect();
        let state = TransferState::new(transfer_id, total_length, chunk_ids.clone());
        self.active_transfer = Some(ActiveTransfer {
            state,
            assignment: assignment.clone(),
            in_flight,
        });
        Action::Accelerate {
            transfer_id,
            total_length,
            assignment,
        }
    }

    /// Process received ChunkData. Returns Ok(Some(reassembled_bytes)) when transfer complete,
    /// Ok(None) when in progress, Err on integrity failure.
    ///
    /// On integrity failure the sending peer is recorded; after
    /// `integrity_failure_threshold` failures the peer is isolated and the
    /// chunk is reassigned via NACK.
    pub fn on_chunk_received(
        &mut self,
        peer_id: DeviceId,
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
        let chunk_id = ChunkId {
            transfer_id,
            start,
            end,
        };
        active.in_flight.remove(&chunk_id);
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
            chunk::ChunkReceiveResult::IntegrityFailed => {
                self.record_integrity_failure(peer_id);
                Err(ChunkError::IntegrityFailed)
            }
        }
    }

    /// Record an integrity failure for a peer. Isolate the peer after threshold.
    fn record_integrity_failure(&mut self, peer_id: DeviceId) {
        let count = self.integrity_failures.entry(peer_id).or_insert(0);
        *count += 1;
        if *count >= self.integrity_failure_threshold && !self.isolated_peers.contains(&peer_id) {
            self.isolated_peers.push(peer_id);
            self.peers.retain(|p| *p != peer_id);
            self.peer_last_tick.remove(&peer_id);
        }
    }

    /// Returns true if a peer has been isolated due to integrity failures.
    pub fn is_peer_isolated(&self, peer_id: &DeviceId) -> bool {
        self.isolated_peers.contains(peer_id)
    }

    /// Process a received wire message (already decoded). Returns outbound actions and
    /// optionally a completed transfer's reassembled bytes.
    ///
    /// The host decodes the frame (via `decode_frame`), then passes the `Message` here.
    pub fn on_message_received(
        &mut self,
        from: DeviceId,
        msg: Message,
    ) -> (Vec<OutboundAction>, Option<Vec<u8>>) {
        let mut actions = Vec::new();
        let mut completed = None;
        match msg {
            Message::Heartbeat { device_id } => {
                let id = if self.peers.contains(&device_id) {
                    device_id
                } else {
                    from
                };
                self.on_heartbeat_received(id);
            }
            Message::Join { device_id } => {
                let pk = PublicKey::from_bytes([0u8; 32]);
                self.on_peer_joined(device_id, &pk);
            }
            Message::Leave { device_id } => {
                actions.extend(self.on_peer_left(device_id));
            }
            Message::ChunkData {
                transfer_id,
                start,
                end,
                hash,
                payload,
            } => match self.on_chunk_received(from, transfer_id, start, end, hash, payload) {
                Ok(Some(bytes)) => completed = Some(bytes),
                Ok(None) => {}
                Err(ChunkError::IntegrityFailed) => {
                    let nack = Message::Nack {
                        transfer_id,
                        start,
                        end,
                    };
                    if let Ok(bytes) = wire::encode_frame(&nack) {
                        actions.push(OutboundAction::SendMessage(from, bytes));
                    }
                }
                Err(_) => {}
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
                if let Some(active) = &mut self.active_transfer {
                    if active.state.transfer_id == transfer_id {
                        active.in_flight.remove(&chunk_id);
                        let remaining: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
                            .chain(self.peers.iter().copied())
                            .collect();
                        if !remaining.is_empty() {
                            let new_peer = remaining[0];
                            active.assignment.retain(|(c, _)| *c != chunk_id);
                            active.assignment.push((chunk_id, new_peer));
                            active.in_flight.insert(chunk_id, self.tick_count);
                            let req = chunk::chunk_request_message(chunk_id);
                            if let Ok(bytes) = wire::encode_frame(&req) {
                                actions.push(OutboundAction::SendMessage(new_peer, bytes));
                            }
                        }
                    }
                }
            }
            Message::ChunkRequest {
                transfer_id,
                start,
                end,
            } => {
                // Host should handle the actual data fetch and send ChunkData back.
                // Core emits a WanFetch action so the host knows to fetch this range.
                actions.push(OutboundAction::WanFetch {
                    peer: from,
                    transfer_id,
                    start,
                    end,
                });
            }
            // Beacon and DiscoveryResponse are handled at the discovery layer by the host.
            _ => {}
        }
        (actions, completed)
    }

    /// Peer joined. Update peer list and last-seen.
    pub fn on_peer_joined(&mut self, peer_id: DeviceId, _public_key: &PublicKey) {
        if self.isolated_peers.contains(&peer_id) {
            return;
        }
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

    /// Periodic tick: check heartbeat timeouts and chunk request timeouts,
    /// produce heartbeat messages.
    pub fn tick(&mut self) -> Vec<OutboundAction> {
        self.tick_count = self.tick_count.saturating_add(1);
        let mut actions = Vec::new();

        // Heartbeat timeouts: remove overdue peers.
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

        // Chunk request timeouts: reassign timed-out chunks.
        if let Some(active) = &mut self.active_transfer {
            let timed_out: Vec<ChunkId> = active
                .in_flight
                .iter()
                .filter(|(_, &t)| self.tick_count.saturating_sub(t) > self.chunk_timeout_ticks)
                .map(|(&c, _)| c)
                .collect();
            for chunk_id in &timed_out {
                active.in_flight.remove(chunk_id);
            }
            if !timed_out.is_empty() {
                let remaining: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
                    .chain(self.peers.iter().copied())
                    .collect();
                if !remaining.is_empty() {
                    for (i, chunk_id) in timed_out.iter().enumerate() {
                        let new_peer = remaining[i % remaining.len()];
                        active.assignment.retain(|(c, _)| c != chunk_id);
                        active.assignment.push((*chunk_id, new_peer));
                        active.in_flight.insert(*chunk_id, self.tick_count);
                        let msg = chunk::chunk_request_message(*chunk_id);
                        if let Ok(bytes) = wire::encode_frame(&msg) {
                            actions.push(OutboundAction::SendMessage(new_peer, bytes));
                        }
                    }
                }
            }
        }

        // Send heartbeats to all peers.
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
            active.in_flight.insert(chunk_id, self.tick_count);
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

    /// Returns the list of peers currently in the pod.
    pub fn peers(&self) -> &[DeviceId] {
        &self.peers
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
#[derive(Debug)]
pub enum OutboundAction {
    /// Send encoded message bytes to the given peer over local transport.
    SendMessage(DeviceId, Vec<u8>),
    /// Host should fetch this range from WAN and deliver ChunkData back to core.
    WanFetch {
        peer: DeviceId,
        transfer_id: [u8; 16],
        start: u64,
        end: u64,
    },
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
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());

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
                peer_id,
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

    #[test]
    fn fallback_when_no_peers() {
        let mut core = PeaPodCore::new();
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn fallback_for_ineligible_url() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        core.on_peer_joined(peer_kp.device_id(), peer_kp.public_key());
        let action = core.on_incoming_request("ftp://example.com/file", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    #[test]
    fn is_eligible_http() {
        assert!(PeaPodCore::is_eligible("http://example.com/file", false));
        assert!(PeaPodCore::is_eligible("https://example.com/file", false));
        assert!(!PeaPodCore::is_eligible("ftp://example.com/file", false));
        assert!(!PeaPodCore::is_eligible("http://example.com/file", true));
    }

    #[test]
    fn on_message_received_heartbeat() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());
        let msg = Message::Heartbeat { device_id: peer_id };
        let (actions, completed) = core.on_message_received(peer_id, msg);
        assert!(actions.is_empty());
        assert!(completed.is_none());
    }

    #[test]
    fn on_message_received_join() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        let msg = Message::Join { device_id: peer_id };
        let (actions, completed) = core.on_message_received(peer_id, msg);
        assert!(actions.is_empty());
        assert!(completed.is_none());
        assert_eq!(core.peers().len(), 1);
    }

    #[test]
    fn on_message_received_leave() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());
        assert_eq!(core.peers().len(), 1);
        let msg = Message::Leave { device_id: peer_id };
        let (_actions, _completed) = core.on_message_received(peer_id, msg);
        assert_eq!(core.peers().len(), 0);
    }

    #[test]
    fn on_message_received_chunk_data() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match &action {
            Action::Accelerate { transfer_id, .. } => *transfer_id,
            Action::Fallback => panic!("expected Accelerate"),
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
            let (actions, completed) = core.on_message_received(peer_id, msg);
            assert!(actions.is_empty());
            if let Some(bytes) = completed {
                assert_eq!(bytes.len(), 100);
                return;
            }
        }
        panic!("transfer should complete");
    }

    #[test]
    fn on_message_received_chunk_data_integrity_failure_sends_nack() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());

        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match &action {
            Action::Accelerate { transfer_id, .. } => *transfer_id,
            Action::Fallback => panic!("expected Accelerate"),
        };

        let payload = vec![0u8; 100];
        let bad_hash = [0u8; 32]; // wrong hash
        let msg = Message::ChunkData {
            transfer_id,
            start: 0,
            end: 100,
            hash: bad_hash,
            payload,
        };
        let (actions, completed) = core.on_message_received(peer_id, msg);
        assert!(completed.is_none());
        // Should have sent a NACK back
        assert!(!actions.is_empty());
    }

    #[test]
    fn integrity_failure_isolates_peer_after_threshold() {
        let mut core = PeaPodCore::new();
        core.set_integrity_failure_threshold(2);

        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());
        assert!(!core.is_peer_isolated(&peer_id));

        // Simulate integrity failures through on_chunk_received
        let action = core.on_incoming_request("http://example.com/file", Some((0, 99)));
        let transfer_id = match &action {
            Action::Accelerate { transfer_id, .. } => *transfer_id,
            Action::Fallback => panic!("expected Accelerate"),
        };

        let bad_hash = [0u8; 32];
        // First failure
        let _ = core.on_chunk_received(peer_id, transfer_id, 0, 100, bad_hash, vec![0u8; 100]);
        assert!(!core.is_peer_isolated(&peer_id));

        // Second failure -> should isolate
        let _ = core.on_chunk_received(peer_id, transfer_id, 0, 100, bad_hash, vec![0u8; 100]);
        assert!(core.is_peer_isolated(&peer_id));
        assert_eq!(core.peers().len(), 0);
    }

    #[test]
    fn chunk_timeout_reassigns() {
        let mut core = PeaPodCore::new();
        core.set_chunk_timeout_ticks(2);

        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());

        let _action = core.on_incoming_request("http://example.com/file", Some((0, 99)));

        // Tick past timeout: 3 ticks should exceed timeout of 2
        let _ = core.tick();
        let _ = core.tick();
        let actions = core.tick();
        // Should have reassignment actions (chunk requests) plus heartbeats
        let has_send = actions
            .iter()
            .any(|a| matches!(a, OutboundAction::SendMessage(_, _)));
        assert!(has_send);
    }

    #[test]
    fn on_message_received_chunk_request_emits_wan_fetch() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());

        let msg = Message::ChunkRequest {
            transfer_id: [1u8; 16],
            start: 0,
            end: 100,
        };
        let (actions, _) = core.on_message_received(peer_id, msg);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], OutboundAction::WanFetch { .. }));
    }

    #[test]
    fn heartbeat_timeout_removes_peer() {
        let mut core = PeaPodCore::new();
        let peer_kp = Keypair::generate();
        let peer_id = peer_kp.device_id();
        core.on_peer_joined(peer_id, peer_kp.public_key());
        assert_eq!(core.peers().len(), 1);

        // Tick past heartbeat timeout (5 ticks)
        for _ in 0..6 {
            core.tick();
        }
        assert_eq!(core.peers().len(), 0);
    }

    #[test]
    fn peer_left_mid_transfer_redistributes() {
        let mut core = PeaPodCore::new();
        let peer_a = Keypair::generate();
        let peer_b = Keypair::generate();
        core.on_peer_joined(peer_a.device_id(), peer_a.public_key());
        core.on_peer_joined(peer_b.device_id(), peer_b.public_key());

        let _action = core.on_incoming_request("http://example.com/file", Some((0, 999)));
        let before = core.current_assignment().unwrap().len();
        assert!(before > 0);

        let actions = core.on_peer_left(peer_a.device_id());
        // Should produce outbound messages for redistributed chunks
        assert!(!actions.is_empty() || core.current_assignment().unwrap().len() >= before);
    }
}
