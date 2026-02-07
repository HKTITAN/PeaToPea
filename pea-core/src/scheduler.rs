//! Distributed scheduler: assign chunks to peers. Full implementation later.

use crate::identity::DeviceId;

/// Stub: assign chunks to a single peer (self). Full logic with peer list and weights later.
pub fn assign_chunks_to_peers(
    _chunk_ids: &[crate::chunk::ChunkId],
    peers: &[DeviceId],
) -> Vec<(crate::chunk::ChunkId, Option<DeviceId>)> {
    if peers.is_empty() {
        return vec![];
    }
    _chunk_ids
        .iter()
        .map(|id| (*id, Some(peers[0])))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chunk::ChunkId;
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
        assert_eq!(out[0].1, Some(kp.device_id()));
    }
}
