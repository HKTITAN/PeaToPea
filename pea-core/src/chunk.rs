//! Chunk manager: split transfer into chunks, track state, reassemble.
//! Full implementation in later work.

/// Chunk identifier: transfer ID + range (start, end).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChunkId {
    pub transfer_id: [u8; 16],
    pub start: u64,
    pub end: u64,
}

/// Stub: split a transfer into chunks by fixed size. Full logic later.
pub fn split_into_chunks(transfer_id: [u8; 16], total_len: u64, chunk_size: u64) -> Vec<ChunkId> {
    let mut out = Vec::new();
    let mut start = 0u64;
    while start < total_len {
        let end = (start + chunk_size).min(total_len);
        out.push(ChunkId {
            transfer_id,
            start,
            end,
        });
        start = end;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
