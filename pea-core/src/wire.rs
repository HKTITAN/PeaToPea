//! Framing: length-prefix (4 bytes LE) + bincode payload.

use crate::protocol::Message;

const LEN_SIZE: usize = 4;
const MAX_FRAME_LEN: u32 = 16 * 1024 * 1024; // 16 MiB

/// Encode a message into a single frame: 4 bytes LE length + bincode payload.
pub fn encode_frame(msg: &Message) -> Result<Vec<u8>, FrameEncodeError> {
    let payload = bincode::serialize(msg).map_err(FrameEncodeError::Encode)?;
    let len = payload.len() as u32;
    if len > MAX_FRAME_LEN {
        return Err(FrameEncodeError::TooLarge);
    }
    let mut out = Vec::with_capacity(LEN_SIZE + payload.len());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&payload);
    Ok(out)
}

/// Error encoding a message into a frame (bincode or size limit).
#[derive(Debug, thiserror::Error)]
pub enum FrameEncodeError {
    #[error("encode error: {0}")]
    Encode(#[from] bincode::Error),
    #[error("frame too large")]
    TooLarge,
}

/// Decode one frame from the front of `bytes`. Returns the message and the number of bytes consumed.
/// Call with partial buffer; returns error if not enough bytes (caller should try again after more data).
pub fn decode_frame(bytes: &[u8]) -> Result<(Message, usize), FrameDecodeError> {
    if bytes.len() < LEN_SIZE {
        return Err(FrameDecodeError::NeedMore);
    }
    let len = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    if len > MAX_FRAME_LEN as usize {
        return Err(FrameDecodeError::TooLarge);
    }
    if bytes.len() < LEN_SIZE + len {
        return Err(FrameDecodeError::NeedMore);
    }
    let msg: Message =
        bincode::deserialize(&bytes[LEN_SIZE..LEN_SIZE + len]).map_err(FrameDecodeError::Decode)?;
    Ok((msg, LEN_SIZE + len))
}

/// Error decoding a frame (need more bytes, too large, or bincode failure).
#[derive(Debug, thiserror::Error)]
pub enum FrameDecodeError {
    #[error("need more bytes")]
    NeedMore,
    #[error("frame too large")]
    TooLarge,
    #[error("decode error: {0}")]
    Decode(#[from] bincode::Error),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::Keypair;
    use crate::protocol::PROTOCOL_VERSION;

    fn sample_beacon() -> Message {
        let kp = Keypair::generate();
        Message::Beacon {
            protocol_version: PROTOCOL_VERSION,
            device_id: kp.device_id(),
            public_key: kp.public_key().clone(),
            listen_port: 45678,
        }
    }

    #[test]
    fn roundtrip_beacon() {
        let msg = sample_beacon();
        let frame = encode_frame(&msg).unwrap();
        let (decoded, n) = decode_frame(&frame).unwrap();
        assert_eq!(n, frame.len());
        match (&msg, &decoded) {
            (
                Message::Beacon {
                    protocol_version: v1,
                    device_id: d1,
                    ..
                },
                Message::Beacon {
                    protocol_version: v2,
                    device_id: d2,
                    ..
                },
            ) => {
                assert_eq!(v1, v2);
                assert_eq!(d1, d2);
            }
            _ => panic!("expected Beacon"),
        }
    }

    #[test]
    fn partial_read_need_more() {
        let msg = sample_beacon();
        let frame = encode_frame(&msg).unwrap();
        assert!(matches!(
            decode_frame(&frame[..2]),
            Err(FrameDecodeError::NeedMore)
        ));
        assert!(matches!(
            decode_frame(&frame[..super::LEN_SIZE]),
            Err(FrameDecodeError::NeedMore)
        ));
    }

    #[test]
    fn multiple_messages() {
        let a = sample_beacon();
        let b = Message::Heartbeat {
            device_id: Keypair::generate().device_id(),
        };
        let fa = encode_frame(&a).unwrap();
        let fb = encode_frame(&b).unwrap();
        let mut buf = Vec::new();
        buf.extend_from_slice(&fa);
        buf.extend_from_slice(&fb);
        let (m1, n1) = decode_frame(&buf).unwrap();
        assert_eq!(n1, fa.len());
        let (m2, n2) = decode_frame(&buf[n1..]).unwrap();
        assert_eq!(n2, fb.len());
        assert!(matches!(m1, Message::Beacon { .. }));
        assert!(matches!(m2, Message::Heartbeat { .. }));
    }
}
