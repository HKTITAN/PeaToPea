//! Distributed scheduler: assign chunks to peers; reassign when peer leaves.
//! Supports per-peer metrics and slow peer reduction.

use std::collections::HashMap;

use crate::chunk::ChunkId;
use crate::identity::DeviceId;

/// Per-peer metrics: bandwidth, latency, stability.
#[derive(Debug, Clone, Default)]
pub struct PeerMetrics {
    /// Number of successful chunk deliveries.
    pub successes: u64,
    /// Number of failures (integrity, timeout).
    pub failures: u64,
}

impl PeerMetrics {
    /// Record a successful chunk delivery.
    pub fn record_success(&mut self) {
        self.successes += 1;
    }

    /// Record a failure.
    pub fn record_failure(&mut self) {
        self.failures += 1;
    }

    /// Failure rate (0.0 = no failures, 1.0 = all failures).
    pub fn failure_rate(&self) -> f64 {
        let total = self.successes + self.failures;
        if total == 0 {
            0.0
        } else {
            self.failures as f64 / total as f64
        }
    }
}

/// Default failure threshold to exclude a peer from assignment.
pub const DEFAULT_MAX_FAILURES: u64 = 3;

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

/// Assign chunks to peers, excluding peers that have exceeded the failure threshold.
/// Falls back to all peers if all are excluded.
pub fn assign_chunks_with_metrics(
    chunk_ids: &[ChunkId],
    peers: &[DeviceId],
    metrics: &HashMap<DeviceId, PeerMetrics>,
    max_failures: u64,
) -> Vec<(ChunkId, DeviceId)> {
    let eligible: Vec<DeviceId> = peers
        .iter()
        .filter(|p| {
            metrics
                .get(p)
                .is_none_or(|m| m.failures < max_failures)
        })
        .copied()
        .collect();
    let effective = if eligible.is_empty() { peers } else { &eligible };
    assign_chunks_to_peers(chunk_ids, effective)
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
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
            ChunkId { transfer_id: [0; 16], start: 100, end: 200 },
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
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
            ChunkId { transfer_id: [0; 16], start: 100, end: 200 },
            ChunkId { transfer_id: [0; 16], start: 200, end: 300 },
        ];
        let peers = vec![a.device_id(), b.device_id()];
        let out = assign_chunks_to_peers(&chunks, &peers);
        assert_eq!(out[0].1, a.device_id());
        assert_eq!(out[1].1, b.device_id());
        assert_eq!(out[2].1, a.device_id());
    }

    #[test]
    fn assign_no_peers_returns_empty() {
        let chunks = vec![
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
        ];
        let out = assign_chunks_to_peers(&chunks, &[]);
        assert!(out.is_empty());
    }

    #[test]
    fn reassign_after_leave() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let chunks = vec![
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
            ChunkId { transfer_id: [0; 16], start: 100, end: 200 },
        ];
        let peers = vec![a.device_id(), b.device_id()];
        let assignment = assign_chunks_to_peers(&chunks, &peers);
        let remaining = vec![b.device_id()];
        let new_assignments = reassign_after_peer_left(&assignment, a.device_id(), &remaining);
        assert_eq!(new_assignments.len(), 1);
        assert_eq!(new_assignments[0].1, b.device_id());
    }

    #[test]
    fn assign_with_metrics_excludes_failing_peer() {
        let a = Keypair::generate();
        let b = Keypair::generate();
        let chunks = vec![
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
            ChunkId { transfer_id: [0; 16], start: 100, end: 200 },
            ChunkId { transfer_id: [0; 16], start: 200, end: 300 },
        ];
        let peers = vec![a.device_id(), b.device_id()];

        let mut metrics = HashMap::new();
        let mut bad_metrics = PeerMetrics::default();
        for _ in 0..DEFAULT_MAX_FAILURES {
            bad_metrics.record_failure();
        }
        metrics.insert(a.device_id(), bad_metrics);

        let out = assign_chunks_with_metrics(&chunks, &peers, &metrics, DEFAULT_MAX_FAILURES);
        // All chunks should be assigned to peer b since a has too many failures.
        for (_chunk, peer) in &out {
            assert_eq!(*peer, b.device_id());
        }
    }

    #[test]
    fn assign_with_metrics_fallback_all_excluded() {
        let a = Keypair::generate();
        let chunks = vec![
            ChunkId { transfer_id: [0; 16], start: 0, end: 100 },
        ];
        let peers = vec![a.device_id()];

        let mut metrics = HashMap::new();
        let mut bad_metrics = PeerMetrics::default();
        for _ in 0..DEFAULT_MAX_FAILURES {
            bad_metrics.record_failure();
        }
        metrics.insert(a.device_id(), bad_metrics);

        // All peers excluded, should fall back to all peers.
        let out = assign_chunks_with_metrics(&chunks, &peers, &metrics, DEFAULT_MAX_FAILURES);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].1, a.device_id());
    }

    #[test]
    fn peer_metrics_failure_rate() {
        let mut m = PeerMetrics::default();
        assert_eq!(m.failure_rate(), 0.0);
        m.record_success();
        m.record_success();
        m.record_failure();
        assert!((m.failure_rate() - 1.0 / 3.0).abs() < 0.01);
    }
}
