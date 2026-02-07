//! Host-driven API: PeaPodCore receives events from host, returns actions.

use std::collections::{HashMap, HashSet};

use crate::chunk::{self, ChunkId, TransferState, DEFAULT_CHUNK_SIZE};
use crate::identity::{DeviceId, Keypair, PublicKey};
use crate::integrity;
use crate::protocol::Message;
use crate::scheduler;
use crate::wire;

const HEARTBEAT_TIMEOUT_TICKS: u64 = 5;

/// Default timeout (in ticks) before a chunk request is considered timed out.
pub const DEFAULT_CHUNK_TIMEOUT_TICKS: u64 = 30;

/// Stub for upload path (split outbound into chunks; full impl later).
pub fn split_upload_chunks(transfer_id: [u8; 16], data_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    chunk::split_into_chunks(transfer_id, data_len, chunk_size)
}

/// Active transfer: state and assignment.
struct ActiveTransfer {
    state: TransferState,
    assignment: Vec<(ChunkId, DeviceId)>,
}

/// Active upload state for outbound data distribution.
pub struct ActiveUpload {
    pub transfer_id: [u8; 16],
    pub total_length: u64,
    pub assignment: Vec<(ChunkId, DeviceId)>,
    pub completed: HashSet<ChunkId>,
    pub chunk_hashes: HashMap<ChunkId, [u8; 32]>,
}

/// Result of starting an upload.
pub enum UploadAction {
    /// Distribute chunks to peers.
    Distribute {
        transfer_id: [u8; 16],
        assignment: Vec<(ChunkId, DeviceId)>,
        chunk_data: Vec<(ChunkId, Vec<u8>)>,
    },
    /// No peers available; host should handle the upload alone.
    Fallback,
}

/// Metadata about an incoming request for traffic eligibility checking.
pub struct RequestMetadata {
    pub url: String,
    pub method: String,
    pub content_length: Option<u64>,
    pub supports_range: bool,
    pub is_encrypted_stream: bool,
}

/// Check whether a request is eligible for peer-assisted transfer.
pub fn is_eligible(metadata: &RequestMetadata) -> bool {
    metadata.supports_range
        && !metadata.is_encrypted_stream
        && metadata.content_length.map(|l| l > 0).unwrap_or(false)
}

/// Main coordinator. Host passes events; core returns actions.
pub struct PeaPodCore {
    keypair: Keypair,
    peers: Vec<DeviceId>,
    peer_last_tick: HashMap<DeviceId, u64>,
    tick_count: u64,
    active_transfer: Option<ActiveTransfer>,
    active_upload: Option<ActiveUpload>,
    chunk_request_times: HashMap<ChunkId, u64>,
}

impl PeaPodCore {
    pub fn new() -> Self {
        Self {
            keypair: Keypair::generate(),
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            active_upload: None,
            chunk_request_times: HashMap::new(),
        }
    }

    pub fn with_keypair(keypair: Keypair) -> Self {
        Self {
            keypair,
            peers: Vec::new(),
            peer_last_tick: HashMap::new(),
            tick_count: 0,
            active_transfer: None,
            active_upload: None,
            chunk_request_times: HashMap::new(),
        }
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
        let assignment = scheduler::assign_chunks_to_peers(&chunk_ids, &workers);
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

    /// On incoming request with metadata: checks eligibility first, then proceeds.
    pub fn on_incoming_request_with_metadata(
        &mut self,
        metadata: &RequestMetadata,
    ) -> Action {
        if !is_eligible(metadata) {
            return Action::Fallback;
        }
        let range = metadata.content_length.map(|l| (0u64, l.saturating_sub(1)));
        self.on_incoming_request(&metadata.url, range)
    }

    /// Start an upload: split data into chunks, assign to peers, compute hashes.
    pub fn start_upload(&mut self, data: &[u8]) -> UploadAction {
        if self.peers.is_empty() {
            return UploadAction::Fallback;
        }
        let transfer_id: [u8; 16] = uuid::Uuid::new_v4().into_bytes();
        let chunk_ids = chunk::split_into_chunks(transfer_id, data.len() as u64, DEFAULT_CHUNK_SIZE);
        let assignment = scheduler::assign_chunks_to_peers(&chunk_ids, &self.peers);

        let mut chunk_hashes = HashMap::new();
        let mut chunk_data = Vec::new();
        for &cid in &chunk_ids {
            let payload = &data[cid.start as usize..cid.end as usize];
            let hash = integrity::hash_chunk(payload);
            chunk_hashes.insert(cid, hash);
            chunk_data.push((cid, payload.to_vec()));
        }

        self.active_upload = Some(ActiveUpload {
            transfer_id,
            total_length: data.len() as u64,
            assignment: assignment.clone(),
            completed: HashSet::new(),
            chunk_hashes,
        });

        UploadAction::Distribute {
            transfer_id,
            assignment,
            chunk_data,
        }
    }

    /// Mark an upload chunk as complete. Returns true if the entire upload is done.
    pub fn on_upload_chunk_complete(&mut self, chunk_id: ChunkId) -> bool {
        if let Some(ref mut upload) = self.active_upload {
            upload.completed.insert(chunk_id);
            let all_done = upload.assignment.iter().all(|(c, _)| upload.completed.contains(c));
            if all_done {
                self.active_upload = None;
                return true;
            }
        }
        false
    }

    /// Record when a chunk request was sent (for timeout tracking).
    pub fn mark_chunk_requested(&mut self, chunk_id: ChunkId) {
        self.chunk_request_times.insert(chunk_id, self.tick_count);
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
        let chunk_id = ChunkId { transfer_id, start, end };
        self.chunk_request_times.remove(&chunk_id);

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

    /// Process a received wire message from a peer.
    pub fn on_message_received(
        &mut self,
        peer_id: DeviceId,
        bytes: &[u8],
    ) -> Result<Vec<OutboundAction>, MessageError> {
        let (msg, _consumed) = wire::decode_frame(bytes).map_err(|_| MessageError::DecodeError)?;
        match msg {
            Message::Heartbeat { device_id } => {
                self.on_heartbeat_received(device_id);
                Ok(vec![])
            }
            Message::Join { device_id } => {
                // Use a placeholder public key derived from device_id bytes for join
                let placeholder = PublicKey::from_bytes({
                    let mut buf = [0u8; 32];
                    buf[..16].copy_from_slice(device_id.as_bytes());
                    buf
                });
                self.on_peer_joined(device_id, &placeholder);
                Ok(vec![])
            }
            Message::Leave { device_id } => {
                let actions = self.on_peer_left(device_id);
                Ok(actions)
            }
            Message::ChunkData {
                transfer_id,
                start,
                end,
                hash,
                payload,
            } => {
                match self.on_chunk_received(transfer_id, start, end, hash, payload) {
                    Ok(Some(bytes)) => Ok(vec![OutboundAction::TransferComplete(transfer_id, bytes)]),
                    Ok(None) => Ok(vec![]),
                    Err(_) => Ok(vec![]),
                }
            }
            Message::ChunkRequest {
                transfer_id,
                start,
                end,
            } => {
                let chunk_id = ChunkId { transfer_id, start, end };
                Ok(vec![OutboundAction::FetchChunk(chunk_id)])
            }
            Message::Nack {
                transfer_id,
                start,
                end,
            } => {
                let chunk_id = ChunkId { transfer_id, start, end };
                // Reassign this chunk to another peer if we have an active transfer
                if let Some(ref mut active) = self.active_transfer {
                    if active.state.transfer_id == transfer_id {
                        let remaining: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
                            .chain(self.peers.iter().copied().filter(|p| *p != peer_id))
                            .collect();
                        if !remaining.is_empty() {
                            let new_peer = remaining[0];
                            active.assignment.retain(|(c, _)| *c != chunk_id);
                            active.assignment.push((chunk_id, new_peer));
                            let msg = chunk::chunk_request_message(chunk_id);
                            if let Ok(bytes) = wire::encode_frame(&msg) {
                                return Ok(vec![OutboundAction::SendMessage(new_peer, bytes)]);
                            }
                        }
                    }
                }
                Ok(vec![])
            }
            _ => Err(MessageError::UnexpectedMessage),
        }
    }

    /// Periodic tick: check heartbeat timeouts, check chunk request timeouts, produce heartbeat messages.
    pub fn tick(&mut self) -> Vec<OutboundAction> {
        self.tick_count = self.tick_count.saturating_add(1);
        let mut actions = Vec::new();

        // Check heartbeat timeouts
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

        // Check chunk request timeouts and reassign
        let timed_out: Vec<ChunkId> = self
            .chunk_request_times
            .iter()
            .filter(|(_, &t)| self.tick_count.saturating_sub(t) > DEFAULT_CHUNK_TIMEOUT_TICKS)
            .map(|(&c, _)| c)
            .collect();
        for chunk_id in timed_out {
            self.chunk_request_times.remove(&chunk_id);
            if let Some(ref mut active) = self.active_transfer {
                if active.state.transfer_id == chunk_id.transfer_id {
                    // Reassign to first available worker
                    let workers: Vec<DeviceId> = std::iter::once(self.keypair.device_id())
                        .chain(self.peers.iter().copied())
                        .collect();
                    if !workers.is_empty() {
                        let new_peer = workers[0];
                        active.assignment.retain(|(c, _)| *c != chunk_id);
                        active.assignment.push((chunk_id, new_peer));
                        let msg = chunk::chunk_request_message(chunk_id);
                        if let Ok(bytes) = wire::encode_frame(&msg) {
                            actions.push(OutboundAction::SendMessage(new_peer, bytes));
                        }
                    }
                }
            }
        }

        // Send heartbeats
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

#[derive(Debug, thiserror::Error)]
pub enum MessageError {
    #[error("failed to decode frame")]
    DecodeError,
    #[error("unexpected message type")]
    UnexpectedMessage,
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
    FetchChunk(ChunkId),
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

    // 10.3.1: Split transfer with various sizes
    #[test]
    fn split_transfer_various_sizes() {
        let tid = [1u8; 16];
        // Small data, smaller than one chunk
        let chunks = split_into_chunks(tid, 50, 256 * 1024);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 50);

        // Data exactly one chunk
        let chunks = split_into_chunks(tid, 256 * 1024, 256 * 1024);
        assert_eq!(chunks.len(), 1);

        // Data slightly more than one chunk
        let chunks = split_into_chunks(tid, 256 * 1024 + 1, 256 * 1024);
        assert_eq!(chunks.len(), 2);
        assert_eq!(chunks[1].start, 256 * 1024);
        assert_eq!(chunks[1].end, 256 * 1024 + 1);

        // Large data, multiple chunks
        let chunks = split_into_chunks(tid, 1_000_000, 256 * 1024);
        assert_eq!(chunks.len(), 4); // ceil(1000000 / 262144) = 4
    }

    // 10.3.2: Reassembly completeness
    #[test]
    fn reassembly_completeness() {
        let tid = [3u8; 16];
        let data: Vec<u8> = (0..500u16).map(|i| (i % 256) as u8).collect();
        let chunks = split_into_chunks(tid, data.len() as u64, 100);
        let mut state = TransferState::new(tid, data.len() as u64, chunks.clone());

        for c in &chunks {
            let payload = data[c.start as usize..c.end as usize].to_vec();
            let hash = integrity::hash_chunk(&payload);
            chunk::on_chunk_data_received(&mut state, tid, c.start, c.end, hash, payload);
        }
        assert!(state.is_complete());
        let reassembled = state.reassemble_into_bytes();
        assert_eq!(reassembled, data);
    }

    // 10.3.3: Duplicate chunk handling
    #[test]
    fn duplicate_chunk_handling() {
        let tid = [4u8; 16];
        let data = vec![42u8; 200];
        let chunks = split_into_chunks(tid, data.len() as u64, 100);
        let mut state = TransferState::new(tid, data.len() as u64, chunks.clone());

        // Send first chunk twice
        let c = &chunks[0];
        let payload = data[c.start as usize..c.end as usize].to_vec();
        let hash = integrity::hash_chunk(&payload);
        let r1 = chunk::on_chunk_data_received(&mut state, tid, c.start, c.end, hash, payload.clone());
        assert!(matches!(r1, chunk::ChunkReceiveResult::InProgress));
        let r2 = chunk::on_chunk_data_received(&mut state, tid, c.start, c.end, hash, payload);
        assert!(matches!(r2, chunk::ChunkReceiveResult::InProgress));

        // Complete with second chunk
        let c2 = &chunks[1];
        let payload2 = data[c2.start as usize..c2.end as usize].to_vec();
        let hash2 = integrity::hash_chunk(&payload2);
        let r3 = chunk::on_chunk_data_received(&mut state, tid, c2.start, c2.end, hash2, payload2);
        assert!(matches!(r3, chunk::ChunkReceiveResult::Complete(_)));
    }

    // 10.4.1: Assignment with 1, 2, N peers
    #[test]
    fn assignment_with_varying_peers() {
        let mut core = PeaPodCore::new();
        let peer1 = Keypair::generate().device_id();
        core.on_peer_joined(peer1, &Keypair::generate().public_key().clone());

        // 1 peer + self = 2 workers
        let action = core.on_incoming_request("http://example.com/f", Some((0, 499)));
        match &action {
            Action::Accelerate { assignment, .. } => {
                assert!(!assignment.is_empty());
            }
            Action::Fallback => panic!("expected Accelerate with 1 peer"),
        }

        // 2 peers + self = 3 workers
        let mut core2 = PeaPodCore::new();
        let p1 = Keypair::generate().device_id();
        let p2 = Keypair::generate().device_id();
        core2.on_peer_joined(p1, &Keypair::generate().public_key().clone());
        core2.on_peer_joined(p2, &Keypair::generate().public_key().clone());
        let action2 = core2.on_incoming_request("http://example.com/f", Some((0, 999)));
        match &action2 {
            Action::Accelerate { assignment, .. } => {
                assert!(!assignment.is_empty());
            }
            Action::Fallback => panic!("expected Accelerate with 2 peers"),
        }
    }

    // 10.4.2: Reassignment when peer leaves
    #[test]
    fn reassignment_when_peer_leaves() {
        let mut core = PeaPodCore::new();
        let p1 = Keypair::generate().device_id();
        let p2 = Keypair::generate().device_id();
        core.on_peer_joined(p1, &Keypair::generate().public_key().clone());
        core.on_peer_joined(p2, &Keypair::generate().public_key().clone());

        let action = core.on_incoming_request("http://example.com/f", Some((0, 999)));
        assert!(matches!(action, Action::Accelerate { .. }));

        let actions = core.on_peer_left(p1);
        // Should produce SendMessage actions for reassigned chunks
        for a in &actions {
            match a {
                OutboundAction::SendMessage(peer, _) => {
                    assert_ne!(*peer, p1, "should not assign to left peer");
                }
                _ => {}
            }
        }
    }

    // 10.4.3: No assignment when zero peers
    #[test]
    fn no_assignment_zero_peers() {
        let mut core = PeaPodCore::new();
        let action = core.on_incoming_request("http://example.com/f", Some((0, 99)));
        assert!(matches!(action, Action::Fallback));
    }

    // 10.5.1: Valid chunk passes verification
    #[test]
    fn valid_chunk_passes_verification() {
        let payload = b"valid data for chunk";
        let hash = integrity::hash_chunk(payload);
        assert!(integrity::verify_chunk(payload, &hash));
    }

    // 10.5.2: Tampered chunk fails verification
    #[test]
    fn tampered_chunk_fails_verification() {
        let payload = b"original data";
        let hash = integrity::hash_chunk(payload);
        assert!(!integrity::verify_chunk(b"tampered data", &hash));
    }

    // 10.6.1: No peers -> Fallback
    #[test]
    fn mock_host_no_peers_fallback() {
        let mut core = PeaPodCore::new();
        let action = core.on_incoming_request("http://example.com/file.bin", Some((0, 1023)));
        assert!(matches!(action, Action::Fallback));
    }

    // 10.6.2: One peer, chunk data, reassembled output
    #[test]
    fn mock_host_one_peer_reassembly() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let data: Vec<u8> = (0..300u16).map(|i| (i % 256) as u8).collect();
        let action = core.on_incoming_request("http://example.com/f", Some((0, 299)));
        let transfer_id = match &action {
            Action::Accelerate { transfer_id, total_length, .. } => {
                assert_eq!(*total_length, 300);
                *transfer_id
            }
            Action::Fallback => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, 300, DEFAULT_CHUNK_SIZE);
        for &cid in &chunk_ids {
            let payload = data[cid.start as usize..cid.end as usize].to_vec();
            let hash = integrity::hash_chunk(&payload);
            let result = core.on_chunk_received(transfer_id, cid.start, cid.end, hash, payload);
            if let Ok(Some(reassembled)) = result {
                assert_eq!(reassembled, data);
                return;
            }
        }
        panic!("should have completed");
    }

    // 10.6.3: Peer leaves mid-transfer
    #[test]
    fn mock_host_peer_leaves_mid_transfer() {
        let mut core = PeaPodCore::new();
        let p1 = Keypair::generate().device_id();
        let p2 = Keypair::generate().device_id();
        core.on_peer_joined(p1, &Keypair::generate().public_key().clone());
        core.on_peer_joined(p2, &Keypair::generate().public_key().clone());

        let action = core.on_incoming_request("http://example.com/f", Some((0, 999)));
        assert!(matches!(action, Action::Accelerate { .. }));

        // Peer leaves mid-transfer
        let leave_actions = core.on_peer_left(p1);
        // Verify we got reassignment messages
        let assignment = core.current_assignment().unwrap();
        for (_, assigned_peer) in &assignment {
            assert_ne!(*assigned_peer, p1, "left peer should not be in assignment");
        }
        // Verify leave_actions are SendMessage to remaining peers
        for a in &leave_actions {
            match a {
                OutboundAction::SendMessage(peer, _) => {
                    assert_ne!(*peer, p1);
                }
                _ => {}
            }
        }
    }

    // 10.6.4: Heartbeat timeout
    #[test]
    fn mock_host_heartbeat_timeout() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());
        assert_eq!(core.peers.len(), 1);

        // Tick past the timeout threshold without heartbeat
        for _ in 0..=HEARTBEAT_TIMEOUT_TICKS + 1 {
            core.tick();
        }
        assert!(core.peers.is_empty(), "peer should be removed after heartbeat timeout");
    }

    // Test on_message_received: heartbeat
    #[test]
    fn on_message_received_heartbeat() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let msg = Message::Heartbeat { device_id: peer };
        let frame = wire::encode_frame(&msg).unwrap();
        let result = core.on_message_received(peer, &frame);
        assert!(result.is_ok());
        assert!(result.unwrap().is_empty());
    }

    // Test on_message_received: join
    #[test]
    fn on_message_received_join() {
        let mut core = PeaPodCore::new();
        let new_peer = Keypair::generate().device_id();

        let msg = Message::Join { device_id: new_peer };
        let frame = wire::encode_frame(&msg).unwrap();
        let result = core.on_message_received(new_peer, &frame);
        assert!(result.is_ok());
        assert!(core.peers.contains(&new_peer));
    }

    // Test on_message_received: leave
    #[test]
    fn on_message_received_leave() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let msg = Message::Leave { device_id: peer };
        let frame = wire::encode_frame(&msg).unwrap();
        let result = core.on_message_received(peer, &frame);
        assert!(result.is_ok());
        assert!(!core.peers.contains(&peer));
    }

    // Test on_message_received: chunk request
    #[test]
    fn on_message_received_chunk_request() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();

        let msg = Message::ChunkRequest {
            transfer_id: [5u8; 16],
            start: 0,
            end: 100,
        };
        let frame = wire::encode_frame(&msg).unwrap();
        let result = core.on_message_received(peer, &frame).unwrap();
        assert_eq!(result.len(), 1);
        match &result[0] {
            OutboundAction::FetchChunk(cid) => {
                assert_eq!(cid.transfer_id, [5u8; 16]);
                assert_eq!(cid.start, 0);
                assert_eq!(cid.end, 100);
            }
            _ => panic!("expected FetchChunk"),
        }
    }

    // Test on_message_received: chunk data completing a transfer
    #[test]
    fn on_message_received_chunk_data_completes() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let action = core.on_incoming_request("http://example.com/f", Some((0, 49)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            Action::Fallback => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, 50, DEFAULT_CHUNK_SIZE);
        for &cid in &chunk_ids {
            let payload: Vec<u8> = (cid.start..cid.end).map(|j| j as u8).collect();
            let hash = integrity::hash_chunk(&payload);
            let msg = Message::ChunkData {
                transfer_id,
                start: cid.start,
                end: cid.end,
                hash,
                payload,
            };
            let frame = wire::encode_frame(&msg).unwrap();
            let result = core.on_message_received(peer, &frame).unwrap();
            if !result.is_empty() {
                match &result[0] {
                    OutboundAction::TransferComplete(tid, data) => {
                        assert_eq!(*tid, transfer_id);
                        assert_eq!(data.len(), 50);
                        return;
                    }
                    _ => {}
                }
            }
        }
        panic!("expected TransferComplete");
    }

    // Test on_message_received: decode error
    #[test]
    fn on_message_received_decode_error() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        let result = core.on_message_received(peer, &[0xFF, 0xFF]);
        assert!(matches!(result, Err(MessageError::DecodeError)));
    }

    // Test eligibility logic
    #[test]
    fn eligibility_supports_range_not_encrypted() {
        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(1000),
            supports_range: true,
            is_encrypted_stream: false,
        };
        assert!(is_eligible(&meta));
    }

    #[test]
    fn eligibility_no_range_support() {
        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(1000),
            supports_range: false,
            is_encrypted_stream: false,
        };
        assert!(!is_eligible(&meta));
    }

    #[test]
    fn eligibility_encrypted_stream() {
        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(1000),
            supports_range: true,
            is_encrypted_stream: true,
        };
        assert!(!is_eligible(&meta));
    }

    #[test]
    fn eligibility_zero_content_length() {
        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(0),
            supports_range: true,
            is_encrypted_stream: false,
        };
        assert!(!is_eligible(&meta));
    }

    #[test]
    fn eligibility_no_content_length() {
        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: None,
            supports_range: true,
            is_encrypted_stream: false,
        };
        assert!(!is_eligible(&meta));
    }

    // Test on_incoming_request_with_metadata
    #[test]
    fn request_with_metadata_eligible() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(500),
            supports_range: true,
            is_encrypted_stream: false,
        };
        let action = core.on_incoming_request_with_metadata(&meta);
        assert!(matches!(action, Action::Accelerate { .. }));
    }

    #[test]
    fn request_with_metadata_not_eligible() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let meta = RequestMetadata {
            url: "http://example.com/file".to_string(),
            method: "GET".to_string(),
            content_length: Some(500),
            supports_range: false,
            is_encrypted_stream: false,
        };
        let action = core.on_incoming_request_with_metadata(&meta);
        assert!(matches!(action, Action::Fallback));
    }

    // Test upload path
    #[test]
    fn upload_no_peers_fallback() {
        let mut core = PeaPodCore::new();
        let result = core.start_upload(&[1, 2, 3, 4, 5]);
        assert!(matches!(result, UploadAction::Fallback));
    }

    #[test]
    fn upload_with_peers_distributes() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let data = vec![0u8; 100];
        let result = core.start_upload(&data);
        match result {
            UploadAction::Distribute { assignment, chunk_data, .. } => {
                assert!(!assignment.is_empty());
                assert!(!chunk_data.is_empty());
                // All chunk data should sum to original data length
                let total: usize = chunk_data.iter().map(|(_, d)| d.len()).sum();
                assert_eq!(total, 100);
            }
            UploadAction::Fallback => panic!("expected Distribute"),
        }
    }

    #[test]
    fn upload_chunk_complete_tracking() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let data = vec![0u8; 100];
        let result = core.start_upload(&data);
        let chunks: Vec<ChunkId> = match result {
            UploadAction::Distribute { chunk_data, .. } => {
                chunk_data.iter().map(|(c, _)| *c).collect()
            }
            UploadAction::Fallback => panic!("expected Distribute"),
        };

        for (i, &cid) in chunks.iter().enumerate() {
            let done = core.on_upload_chunk_complete(cid);
            if i < chunks.len() - 1 {
                assert!(!done);
            } else {
                assert!(done);
            }
        }
        // Upload should be cleared
        assert!(core.active_upload.is_none());
    }

    // Test chunk request timeout
    #[test]
    fn chunk_request_timeout_reassigns() {
        let mut core = PeaPodCore::new();
        let peer = Keypair::generate().device_id();
        core.on_peer_joined(peer, &Keypair::generate().public_key().clone());

        let action = core.on_incoming_request("http://example.com/f", Some((0, 99)));
        let transfer_id = match action {
            Action::Accelerate { transfer_id, .. } => transfer_id,
            Action::Fallback => panic!("expected Accelerate"),
        };

        let chunk_ids = split_into_chunks(transfer_id, 100, DEFAULT_CHUNK_SIZE);
        // Mark a chunk as requested at tick 0
        core.mark_chunk_requested(chunk_ids[0]);

        // Tick past the timeout
        for _ in 0..=DEFAULT_CHUNK_TIMEOUT_TICKS + 1 {
            core.tick();
            // Keep peer alive by updating heartbeat
            core.on_heartbeat_received(peer);
        }

        // The timed-out chunk should have been reassigned (removed from chunk_request_times)
        assert!(!core.chunk_request_times.contains_key(&chunk_ids[0]));
    }
}
