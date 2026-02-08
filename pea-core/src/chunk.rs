//! Chunk manager: split transfer into chunks, track state, reassemble.

use std::collections::HashMap;

use crate::identity::DeviceId;
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
    let size = if chunk_size == 0 { DEFAULT_CHUNK_SIZE } else { chunk_size };
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
}

impl TransferState {
    pub fn new(transfer_id: [u8; 16], total_length: u64, chunk_ids: Vec<ChunkId>) -> Self {
        Self {
            transfer_id,
            total_length,
            chunk_ids,
            received: HashMap::new(),
        }
    }

    /// Record that a chunk was received and verified. Returns true if transfer is now complete.
    pub fn mark_received(&mut self, chunk_id: ChunkId, payload: Vec<u8>) -> bool {
        self.received.insert(chunk_id, payload);
        self.is_complete()
    }

    pub fn is_complete(&self) -> bool {
        self.chunk_ids.iter().all(|id| self.received.contains_key(id))
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

    /// Whether the chunk has been received and verified.
    pub fn is_chunk_received(&self, chunk_id: ChunkId) -> bool {
        self.received.contains_key(&chunk_id)
    }
}

/// Build a ChunkRequest message for the given chunk (to send to a peer).
/// Pass url so the responder can fetch from WAN when serving the request.
pub fn chunk_request_message(chunk_id: ChunkId, url: Option<String>) -> Message {
    Message::ChunkRequest {
        transfer_id: chunk_id.transfer_id,
        start: chunk_id.start,
        end: chunk_id.end,
        url,
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
    fn split_various_sizes() {
        let id = [0u8; 16];
        assert_eq!(split_into_chunks(id, 0, 100).len(), 0);
        assert_eq!(split_into_chunks(id, 50, 100).len(), 1);
        assert_eq!(split_into_chunks(id, 1000, 100).len(), 10);
        assert_eq!(split_into_chunks(id, 1001, 100).len(), 11);
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
            let r = on_chunk_data_received(&mut state, c.transfer_id, c.start, c.end, hash, payload);
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
        let c = &chunks[0];
        let payload: Vec<u8> = (c.start..c.end).map(|i| i as u8).collect();
        let hash = integrity::hash_chunk(&payload);
        let _ = on_chunk_data_received(&mut state, c.transfer_id, c.start, c.end, hash, payload.clone());
        let r2 = on_chunk_data_received(&mut state, c.transfer_id, c.start, c.end, hash, payload);
        assert!(matches!(r2, ChunkReceiveResult::InProgress));
    }
}
