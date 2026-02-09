//! Distributed scheduler: assign chunks to peers; reassign when peer leaves.

use std::collections::HashMap;

use crate::chunk::ChunkId;
use crate::identity::DeviceId;

/// Assign each chunk to a peer (round-robin over peers). Returns (ChunkId, DeviceId) for each chunk.
/// If peers is empty, returns empty. Does not include "self" in assignment; host treats missing peer as self.
pub fn assign_chunks_to_peers(
    chunk_ids: &[ChunkId],
    peers: &[DeviceId],
) -> Vec<(ChunkId, DeviceId)> {
    assign_chunks_to_peers_weighted(chunk_ids, peers, None)
}

/// Like assign_chunks_to_peers but with optional per-peer weights (same order as peers).
/// Weight 0 excludes a peer from assignment; chunks are distributed in proportion to weight.
pub fn assign_chunks_to_peers_weighted(
    chunk_ids: &[ChunkId],
    peers: &[DeviceId],
    weights: Option<&[u64]>,
) -> Vec<(ChunkId, DeviceId)> {
    if peers.is_empty() {
        return vec![];
    }
    let Some(w) = weights else {
        return chunk_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, peers[i % peers.len()]))
            .collect();
    };
    if w.len() != peers.len() {
        return chunk_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, peers[i % peers.len()]))
            .collect();
    }
    let total: u64 = chunk_ids.len() as u64;
    let sum_w: u64 = w.iter().sum();
    if sum_w == 0 {
        return chunk_ids
            .iter()
            .enumerate()
            .map(|(i, &id)| (id, peers[i % peers.len()]))
            .collect();
    }
    let mut counts: Vec<u64> = w.iter().map(|&x| (x * total) / sum_w).collect();
    let mut assigned: u64 = counts.iter().sum();
    let mut i = 0usize;
    while assigned < total && i < counts.len() {
        counts[i] += 1;
        assigned += 1;
        i += 1;
    }
    let mut out = Vec::with_capacity(chunk_ids.len());
    let mut idx = 0usize;
    for (pi, &count) in counts.iter().enumerate() {
        for _ in 0..count {
            if idx < chunk_ids.len() {
                out.push((chunk_ids[idx], peers[pi]));
                idx += 1;
            }
        }
    }
    out
}

/// Reassign chunks that were assigned to `peer_left` to the remaining peers.
/// Returns only the new assignments for chunks that were previously assigned to peer_left.
pub fn reassign_after_peer_left(
    current_assignment: &[(ChunkId, DeviceId)],
    peer_left: DeviceId,
    remaining_peers: &[DeviceId],
) -> Vec<(ChunkId, DeviceId)> {
    if remaining_peers.is_empty() {
        return current_assignment
            .iter()
            .filter(|(_, p)| *p == peer_left)
            .map(|(c, _)| (*c, peer_left)) // keep same peer (will fail; host can retry)
            .collect();
    }
    let to_reassign: Vec<ChunkId> = current_assignment
        .iter()
        .filter(|(_, p)| *p == peer_left)
        .map(|(c, _)| *c)
        .collect();
    assign_chunks_to_peers(&to_reassign, remaining_peers)
}

/// Build assignment map: ChunkId -> DeviceId for quick lookup (e.g. which peer to ask for a chunk).
pub fn assignment_map(assignment: &[(ChunkId, DeviceId)]) -> HashMap<ChunkId, DeviceId> {
    assignment.iter().map(|(c, p)| (*c, *p)).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Keypair;

    #[test]
    fn assign_zero_peers_empty() {
        let chunks = vec![ChunkId {
            transfer_id: [0; 16],
            start: 0,
            end: 100,
        }];
        let peers: Vec<DeviceId> = vec![];
        let out = assign_chunks_to_peers(&chunks, &peers);
        assert_eq!(out.len(), 0);
    }

    #[test]
    fn assign_to_single_peer() {
        let kp = Keypair::generate();
        let chunks = vec![
            ChunkId {
                transfer_id: [0; 16],
                start: 0,
                end: 100,
            },
            ChunkId {
                transfer_id: [0; 16],
                start: 100,
                end: 200,
            },
        ];
        let peers = vec![kp.device_id()];
        let out = assign_chunks_to_peers(&chunks, &peers);
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].1, kp.device_id());
    }

    #[test]
    fn assign_round_robin() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let chunks = vec![
            ChunkId {
                transfer_id: [0; 16],
                start: 0,
                end: 100,
            },
            ChunkId {
                transfer_id: [0; 16],
                start: 100,
                end: 200,
            },
            ChunkId {
                transfer_id: [0; 16],
                start: 200,
                end: 300,
            },
        ];
        let peers = vec![a.device_id(), b.device_id()];
        let out = assign_chunks_to_peers(&chunks, &peers);
        assert_eq!(out[0].1, a.device_id());
        assert_eq!(out[1].1, b.device_id());
        assert_eq!(out[2].1, a.device_id());
    }

    #[test]
    fn assign_weighted() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let chunks: Vec<ChunkId> = (0..10)
            .map(|i| ChunkId {
                transfer_id: [0; 16],
                start: i * 100,
                end: (i + 1) * 100,
            })
            .collect();
        let peers = vec![a.device_id(), b.device_id()];
        let weights = vec![1, 3]; // b gets 3x more chunks
        let out = assign_chunks_to_peers_weighted(&chunks, &peers, Some(&weights));
        assert_eq!(out.len(), 10);
        let a_count = out.iter().filter(|(_, p)| *p == a.device_id()).count();
        let b_count = out.iter().filter(|(_, p)| *p == b.device_id()).count();
        assert!(b_count > a_count, "weighted: b should get more chunks");
    }

    #[test]
    fn reassign_after_leave() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let chunks = vec![
            ChunkId {
                transfer_id: [0; 16],
                start: 0,
                end: 100,
            },
            ChunkId {
                transfer_id: [0; 16],
                start: 100,
                end: 200,
            },
        ];
        let peers = vec![a.device_id(), b.device_id()];
        let assignment = assign_chunks_to_peers(&chunks, &peers);
        let remaining = vec![b.device_id()];
        let new_assignments = reassign_after_peer_left(&assignment, a.device_id(), &remaining);
        assert_eq!(new_assignments.len(), 1);
        assert_eq!(new_assignments[0].1, b.device_id());
    }
}
