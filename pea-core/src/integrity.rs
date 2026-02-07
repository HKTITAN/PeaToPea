//! Integrity: per-chunk hash (e.g. SHA-256), verify on receive, peer trust tracking.

use std::collections::HashMap;

use sha2::{Digest, Sha256};

use crate::identity::DeviceId;

/// Default maximum integrity failures before a peer is isolated.
pub const DEFAULT_MAX_INTEGRITY_FAILURES: u32 = 3;

/// Hash a chunk payload. Returns 32-byte digest.
pub fn hash_chunk(payload: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hasher.finalize().into()
}

/// Verify chunk payload against expected hash.
pub fn verify_chunk(payload: &[u8], expected_hash: &[u8; 32]) -> bool {
    hash_chunk(payload) == *expected_hash
}

/// Tracks integrity failures per peer for malicious peer detection.
pub struct PeerTrustTracker {
    failures: HashMap<DeviceId, u32>,
}

impl PeerTrustTracker {
    pub fn new() -> Self {
        Self {
            failures: HashMap::new(),
        }
    }

    /// Record an integrity failure for a peer.
    pub fn record_failure(&mut self, peer_id: DeviceId) {
        *self.failures.entry(peer_id).or_insert(0) += 1;
    }

    /// Check if a peer should be isolated (failures >= max_failures).
    pub fn is_isolated(&self, peer_id: &DeviceId, max_failures: u32) -> bool {
        self.failure_count(peer_id) >= max_failures
    }

    /// Get the failure count for a peer.
    pub fn failure_count(&self, peer_id: &DeviceId) -> u32 {
        self.failures.get(peer_id).copied().unwrap_or(0)
    }
}

impl Default for PeerTrustTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Keypair;

    #[test]
    fn hash_verify_roundtrip() {
        let payload = b"hello chunk";
        let hash = hash_chunk(payload);
        assert!(verify_chunk(payload, &hash));
    }

    #[test]
    fn verify_rejects_tampered() {
        let payload = b"hello chunk";
        let hash = hash_chunk(payload);
        assert!(!verify_chunk(b"tampered", &hash));
    }

    #[test]
    fn peer_not_isolated_before_threshold() {
        let mut tracker = PeerTrustTracker::new();
        let peer = Keypair::generate().device_id();
        assert!(!tracker.is_isolated(&peer, DEFAULT_MAX_INTEGRITY_FAILURES));
        tracker.record_failure(peer);
        tracker.record_failure(peer);
        assert_eq!(tracker.failure_count(&peer), 2);
        assert!(!tracker.is_isolated(&peer, DEFAULT_MAX_INTEGRITY_FAILURES));
    }

    #[test]
    fn peer_isolated_after_threshold() {
        let mut tracker = PeerTrustTracker::new();
        let peer = Keypair::generate().device_id();
        for _ in 0..DEFAULT_MAX_INTEGRITY_FAILURES {
            tracker.record_failure(peer);
        }
        assert_eq!(tracker.failure_count(&peer), DEFAULT_MAX_INTEGRITY_FAILURES);
        assert!(tracker.is_isolated(&peer, DEFAULT_MAX_INTEGRITY_FAILURES));
    }

    #[test]
    fn peer_isolated_custom_threshold() {
        let mut tracker = PeerTrustTracker::new();
        let peer = Keypair::generate().device_id();
        tracker.record_failure(peer);
        assert!(tracker.is_isolated(&peer, 1));
        assert!(!tracker.is_isolated(&peer, 2));
    }
}
