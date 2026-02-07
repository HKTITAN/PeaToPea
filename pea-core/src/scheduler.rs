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
    if peers.is_empty() {
        return vec![];
    }
    chunk_ids
        .iter()
        .enumerate()
        .map(|(i, &id)| (id, peers[i % peers.len()]))
        .collect()
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
    fn assign_empty_peers_returns_empty() {
        let chunks = vec![ChunkId {
            transfer_id: [0; 16],
            start: 0,
            end: 100,
        }];
        let out = assign_chunks_to_peers(&chunks, &[]);
        assert!(out.is_empty());
    }

    #[test]
    fn assign_empty_chunks_returns_empty() {
        let a = Keypair::generate();
        let out = assign_chunks_to_peers(&[], &[a.device_id()]);
        assert!(out.is_empty());
    }

    #[test]
    fn assign_many_peers() {
        let peers: Vec<_> = (0..5).map(|_| Keypair::generate().device_id()).collect();
        let chunks: Vec<_> = (0..10)
            .map(|i| ChunkId {
                transfer_id: [0; 16],
                start: i * 100,
                end: (i + 1) * 100,
            })
            .collect();
        let out = assign_chunks_to_peers(&chunks, &peers);
        assert_eq!(out.len(), 10);
        // Each peer should get 2 chunks
        for peer in &peers {
            let count = out.iter().filter(|(_, p)| p == peer).count();
            assert_eq!(count, 2);
        }
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

    #[test]
    fn reassign_when_no_remaining_peers() {
        let a = Keypair::generate();
        let chunks = vec![ChunkId {
            transfer_id: [0; 16],
            start: 0,
            end: 100,
        }];
        let peers = vec![a.device_id()];
        let assignment = assign_chunks_to_peers(&chunks, &peers);
        let new_assignments = reassign_after_peer_left(&assignment, a.device_id(), &[]);
        // Should still return the chunk (assigned back to the departed peer)
        assert_eq!(new_assignments.len(), 1);
    }

    #[test]
    fn assignment_map_lookup() {
        let a = Keypair::generate();
        let chunk = ChunkId {
            transfer_id: [0; 16],
            start: 0,
            end: 100,
        };
        let assignment = vec![(chunk, a.device_id())];
        let map = assignment_map(&assignment);
        assert_eq!(map.get(&chunk), Some(&a.device_id()));
    }
}
