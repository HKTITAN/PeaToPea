//! Chunk manager: split transfer into chunks, track state, reassemble.

use std::collections::{HashMap, HashSet};

use crate::integrity;
use crate::protocol::Message;

/// Default chunk size in bytes (constant for now).
pub const DEFAULT_CHUNK_SIZE: u64 = 256 * 1024; // 256 KiB

/// Chunk identifier: transfer ID + range (start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkId {
    pub transfer_id: [u8; 16],
    pub start: u64,
    pub end: u64,
}

/// Split a transfer into chunks by fixed size. HTTP range semantics: each chunk = one range (start, end).
pub fn split_into_chunks(transfer_id: [u8; 16], total_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    let size = if chunk_size == 0 {
        DEFAULT_CHUNK_SIZE
    } else {
        chunk_size
    };
    let mut out = Vec::new();
    let mut start = 0u64;
    while start < total_len {
        let end = (start + size).min(total_len);
        out.push(ChunkId {
            transfer_id,
            start,
            end,
        });
        start = end;
    }
    out
}

/// Per-transfer state: which chunks are assigned, received, in flight; reassembly.
pub struct TransferState {
    pub transfer_id: [u8; 16],
    pub total_length: u64,
    chunk_ids: Vec<ChunkId>,
    /// Chunk payloads received and verified (ChunkId -> payload).
    received: HashMap<ChunkId, Vec<u8>>,
    /// Chunks currently in flight (requested but not yet received).
    in_flight: HashSet<ChunkId>,
}

impl TransferState {
    pub fn new(transfer_id: [u8; 16], total_length: u64, chunk_ids: Vec<ChunkId>) -> Self {
        Self {
            transfer_id,
            total_length,
            chunk_ids,
            received: HashMap::new(),
            in_flight: HashSet::new(),
        }
    }

    /// Mark chunk as in flight (requested).
    pub fn mark_in_flight(&mut self, chunk_id: ChunkId) {
        self.in_flight.insert(chunk_id);
    }

    /// Record that a chunk was received and verified. Returns true if transfer is now complete.
    pub fn mark_received(&mut self, chunk_id: ChunkId, payload: Vec<u8>) -> bool {
        self.in_flight.remove(&chunk_id);
        self.received.insert(chunk_id, payload);
        self.is_complete()
    }

    /// Mark a chunk as failed (remove from in-flight so it can be reassigned).
    pub fn mark_failed(&mut self, chunk_id: ChunkId) {
        self.in_flight.remove(&chunk_id);
    }

    pub fn is_complete(&self) -> bool {
        self.chunk_ids
            .iter()
            .all(|id| self.received.contains_key(id))
    }

    pub fn is_received(&self, chunk_id: &ChunkId) -> bool {
        self.received.contains_key(chunk_id)
    }

    /// Reassemble chunks in order into a single byte stream. Call only when `is_complete()`.
    pub fn reassemble_into_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.total_length as usize);
        for id in &self.chunk_ids {
            if let Some(payload) = self.received.get(id) {
                out.extend_from_slice(payload);
            }
        }
        out
    }

    pub fn chunk_ids(&self) -> &[ChunkId] {
        &self.chunk_ids
    }

    /// Get chunks that are in flight.
    pub fn in_flight(&self) -> &HashSet<ChunkId> {
        &self.in_flight
    }
}

/// Build a ChunkRequest message for the given chunk (to send to a peer).
pub fn chunk_request_message(chunk_id: ChunkId) -> Message {
    Message::ChunkRequest {
        transfer_id: chunk_id.transfer_id,
        start: chunk_id.start,
        end: chunk_id.end,
    }
}

/// Result of processing received ChunkData: verified and stored, or error.
pub enum ChunkReceiveResult {
    /// Chunk stored; transfer is now complete and reassembled bytes are ready.
    Complete(Vec<u8>),
    /// Chunk stored; transfer not yet complete.
    InProgress,
    /// Integrity check failed.
    IntegrityFailed,
}

/// Process ChunkData message: verify hash, store in state. Returns result for the transfer.
pub fn on_chunk_data_received(
    state: &mut TransferState,
    transfer_id: [u8; 16],
    start: u64,
    end: u64,
    hash: [u8; 32],
    payload: Vec<u8>,
) -> ChunkReceiveResult {
    if state.transfer_id != transfer_id {
        return ChunkReceiveResult::IntegrityFailed;
    }
    let chunk_id = ChunkId {
        transfer_id,
        start,
        end,
    };
    if !integrity::verify_chunk(&payload, &hash) {
        state.mark_failed(chunk_id);
        return ChunkReceiveResult::IntegrityFailed;
    }
    let complete = state.mark_received(chunk_id, payload);
    if complete {
        ChunkReceiveResult::Complete(state.reassemble_into_bytes())
    } else {
        ChunkReceiveResult::InProgress
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::integrity;

    #[test]
    fn split_chunks() {
        let id = [1u8; 16];
        let chunks = split_into_chunks(id, 100, 30);
        assert_eq!(chunks.len(), 4);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 30);
        assert_eq!(chunks[3].start, 90);
        assert_eq!(chunks[3].end, 100);
    }

    #[test]
    fn split_chunks_exact() {
        let id = [1u8; 16];
        let chunks = split_into_chunks(id, 90, 30);
        assert_eq!(chunks.len(), 3);
        assert_eq!(chunks[2].end, 90);
    }

    #[test]
    fn split_chunks_single() {
        let id = [1u8; 16];
        let chunks = split_into_chunks(id, 10, 100);
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].start, 0);
        assert_eq!(chunks[0].end, 10);
    }

    #[test]
    fn split_chunks_zero_length() {
        let id = [1u8; 16];
        let chunks = split_into_chunks(id, 0, 30);
        assert!(chunks.is_empty());
    }

    #[test]
    fn split_chunks_default_size() {
        let id = [1u8; 16];
        let chunks = split_into_chunks(id, DEFAULT_CHUNK_SIZE * 3, 0);
        assert_eq!(chunks.len(), 3);
    }

    #[test]
    fn transfer_state_reassemble() {
        let id = [2u8; 16];
        let chunks = split_into_chunks(id, 100, 30);
        let mut state = TransferState::new(id, 100, chunks.clone());
        assert!(!state.is_complete());
        for c in &chunks {
            let payload: Vec<u8> = (c.start..c.end).map(|i| i as u8).collect();
            let hash = integrity::hash_chunk(&payload);
            let r =
                on_chunk_data_received(&mut state, c.transfer_id, c.start, c.end, hash, payload);
            match r {
                ChunkReceiveResult::InProgress => {}
                ChunkReceiveResult::Complete(bytes) => {
                    assert_eq!(bytes.len(), 100);
                    for (i, &b) in bytes.iter().enumerate() {
                        assert_eq!(b, i as u8);
                    }
                }
                ChunkReceiveResult::IntegrityFailed => panic!("integrity failed"),
            }
        }
        assert!(state.is_complete());
    }

    #[test]
    fn duplicate_chunk_idempotent() {
        let id = [3u8; 16];
        let chunks = split_into_chunks(id, 60, 30);
        let mut state = TransferState::new(id, 60, chunks.clone());
        let payload: Vec<u8> = (0..30).collect();
        let hash = integrity::hash_chunk(&payload);
        // Receive same chunk twice.
        let r1 = on_chunk_data_received(&mut state, id, 0, 30, hash, payload.clone());
        assert!(matches!(r1, ChunkReceiveResult::InProgress));
        let r2 = on_chunk_data_received(&mut state, id, 0, 30, hash, payload);
        assert!(matches!(r2, ChunkReceiveResult::InProgress));
    }

    #[test]
    fn integrity_failure_rejects_tampered() {
        let id = [4u8; 16];
        let chunks = split_into_chunks(id, 30, 30);
        let mut state = TransferState::new(id, 30, chunks);
        let payload = vec![1u8; 30];
        let bad_hash = [0u8; 32];
        let r = on_chunk_data_received(&mut state, id, 0, 30, bad_hash, payload);
        assert!(matches!(r, ChunkReceiveResult::IntegrityFailed));
        assert!(!state.is_complete());
    }

    #[test]
    fn in_flight_tracking() {
        let id = [5u8; 16];
        let chunks = split_into_chunks(id, 60, 30);
        let mut state = TransferState::new(id, 60, chunks.clone());
        state.mark_in_flight(chunks[0]);
        assert!(state.in_flight().contains(&chunks[0]));
        let payload: Vec<u8> = (0..30).collect();
        let _hash = integrity::hash_chunk(&payload);
        state.mark_received(chunks[0], payload);
        assert!(!state.in_flight().contains(&chunks[0]));
    }
}
