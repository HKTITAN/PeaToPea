//! Distributed scheduler: assign chunks to peers; reassign when peer leaves; per-peer metrics.

use std::collections::HashMap;

use crate::chunk::ChunkId;
use crate::identity::DeviceId;

/// Default failure threshold before reducing allocation to a slow peer.
pub const SLOW_PEER_FAILURE_THRESHOLD: u32 = 3;

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

/// Per-peer metrics for tracking performance and failures.
#[derive(Debug, Clone, Default)]
pub struct PeerMetrics {
    pub failures: u32,
    pub chunks_completed: u32,
    pub last_failure_tick: Option<u64>,
}

/// Tracks per-peer metrics across a session.
pub struct PeerMetricsTracker {
    metrics: HashMap<DeviceId, PeerMetrics>,
}

impl PeerMetricsTracker {
    pub fn new() -> Self {
        Self {
            metrics: HashMap::new(),
        }
    }

    /// Record a successful chunk completion for a peer.
    pub fn record_success(&mut self, peer_id: DeviceId) {
        self.metrics.entry(peer_id).or_default().chunks_completed += 1;
    }

    /// Record a failure for a peer at the given tick.
    pub fn record_failure(&mut self, peer_id: DeviceId, tick: u64) {
        let m = self.metrics.entry(peer_id).or_default();
        m.failures += 1;
        m.last_failure_tick = Some(tick);
    }

    /// Returns true if the peer's failures >= threshold, indicating allocation should be reduced.
    pub fn should_reduce_allocation(&self, peer_id: &DeviceId, threshold: u32) -> bool {
        self.metrics
            .get(peer_id)
            .map(|m| m.failures >= threshold)
            .unwrap_or(false)
    }

    /// Get the metrics for a peer.
    pub fn get_metrics(&self, peer_id: &DeviceId) -> Option<&PeerMetrics> {
        self.metrics.get(peer_id)
    }
}

impl Default for PeerMetricsTracker {
    fn default() -> Self {
        Self::new()
    }
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
    fn peer_metrics_record_success() {
        let mut tracker = PeerMetricsTracker::new();
        let peer = Keypair::generate().device_id();
        tracker.record_success(peer);
        tracker.record_success(peer);
        let m = tracker.get_metrics(&peer).unwrap();
        assert_eq!(m.chunks_completed, 2);
        assert_eq!(m.failures, 0);
        assert!(m.last_failure_tick.is_none());
    }

    #[test]
    fn peer_metrics_record_failure() {
        let mut tracker = PeerMetricsTracker::new();
        let peer = Keypair::generate().device_id();
        tracker.record_failure(peer, 10);
        tracker.record_failure(peer, 20);
        let m = tracker.get_metrics(&peer).unwrap();
        assert_eq!(m.failures, 2);
        assert_eq!(m.last_failure_tick, Some(20));
    }

    #[test]
    fn should_reduce_allocation_threshold() {
        let mut tracker = PeerMetricsTracker::new();
        let peer = Keypair::generate().device_id();
        assert!(!tracker.should_reduce_allocation(&peer, SLOW_PEER_FAILURE_THRESHOLD));
        tracker.record_failure(peer, 1);
        tracker.record_failure(peer, 2);
        assert!(!tracker.should_reduce_allocation(&peer, SLOW_PEER_FAILURE_THRESHOLD));
        tracker.record_failure(peer, 3);
        assert!(tracker.should_reduce_allocation(&peer, SLOW_PEER_FAILURE_THRESHOLD));
    }

    #[test]
    fn unknown_peer_metrics() {
        let tracker = PeerMetricsTracker::new();
        let peer = Keypair::generate().device_id();
        assert!(tracker.get_metrics(&peer).is_none());
        assert!(!tracker.should_reduce_allocation(&peer, 3));
    }
}
