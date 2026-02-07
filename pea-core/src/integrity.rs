//! Integrity: per-chunk hash (e.g. SHA-256), verify on receive.

use sha2::{Digest, Sha256};

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

#[cfg(test)]
mod tests {
    use super::*;

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
}
