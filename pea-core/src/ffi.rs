//! C ABI for linking pea-core as a static library from Android (NDK) or other C/C++ hosts.
//! JNI in pea-android calls these from C (pea_jni.c).

use std::ffi::c_void;
use std::os::raw::c_int;
use std::slice;

use crate::chunk::ChunkId;
use crate::identity::{decrypt_wire, encrypt_wire, DeviceId, PublicKey};
use crate::protocol::{Message, PROTOCOL_VERSION};
use crate::wire::decode_frame;
use crate::{Action, PeaPodCore};

/// Returns the current protocol version. Used so the staticlib exports a C symbol and is linkable.
#[no_mangle]
pub extern "C" fn pea_core_version() -> u8 {
    PROTOCOL_VERSION
}

/// Create a new core instance. Returns opaque handle or null on failure.
#[no_mangle]
pub extern "C" fn pea_core_create() -> *mut c_void {
    let core = PeaPodCore::new();
    Box::into_raw(Box::new(core)) as *mut c_void
}

/// Destroy core instance. No-op if h is null.
#[no_mangle]
pub extern "C" fn pea_core_destroy(h: *mut c_void) {
    if h.is_null() {
        return;
    }
    let _ = unsafe { Box::from_raw(h as *mut PeaPodCore) };
}

/// Get this device's ID (16 bytes). Returns 0 on success, -1 if h null or out_buf too small.
#[no_mangle]
pub extern "C" fn pea_core_device_id(h: *mut c_void, out_buf: *mut u8, out_len: usize) -> c_int {
    if h.is_null() || out_buf.is_null() || out_len < 16 {
        return -1;
    }
    let core = unsafe { &*(h as *const PeaPodCore) };
    let id = core.device_id();
    unsafe {
        out_buf.copy_from_nonoverlapping(id.as_bytes().as_ptr(), 16);
    }
    0
}

/// Build discovery beacon frame for host to send (UDP). Fills out_buf with length-prefix + bincode Beacon. Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_beacon_frame(
    h: *mut c_void,
    listen_port: u16,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || out_buf.is_null() {
        return -1;
    }
    let core = unsafe { &*(h as *const PeaPodCore) };
    let frame = match core.beacon_frame(listen_port) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    if frame.len() > out_buf_len {
        return -1;
    }
    unsafe {
        out_buf.copy_from_nonoverlapping(frame.as_ptr(), frame.len());
    }
    frame.len() as c_int
}

/// Build DiscoveryResponse frame (send to beacon sender). Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_discovery_response_frame(
    h: *mut c_void,
    listen_port: u16,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || out_buf.is_null() {
        return -1;
    }
    let core = unsafe { &*(h as *const PeaPodCore) };
    let frame = match core.discovery_response_frame(listen_port) {
        Ok(f) => f,
        Err(_) => return -1,
    };
    if frame.len() > out_buf_len {
        return -1;
    }
    unsafe {
        out_buf.copy_from_nonoverlapping(frame.as_ptr(), frame.len());
    }
    frame.len() as c_int
}

/// Decode a discovery frame (Beacon or DiscoveryResponse). Fills device_id (16), public_key (32), listen_port. Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_decode_discovery_frame(
    bytes: *const u8,
    len: usize,
    out_device_id_16: *mut u8,
    out_public_key_32: *mut u8,
    out_listen_port: *mut u16,
) -> c_int {
    if bytes.is_null() || out_device_id_16.is_null() || out_public_key_32.is_null() || out_listen_port.is_null() {
        return -1;
    }
    let slice = unsafe { slice::from_raw_parts(bytes, len) };
    let (msg, _) = match decode_frame(slice) {
        Ok(x) => x,
        Err(_) => return -1,
    };
    match &msg {
        Message::Beacon {
            protocol_version,
            device_id,
            public_key,
            listen_port,
        }
        | Message::DiscoveryResponse {
            protocol_version,
            device_id,
            public_key,
            listen_port,
        } => {
            if *protocol_version != PROTOCOL_VERSION {
                return -1;
            }
            unsafe {
                out_device_id_16.copy_from_nonoverlapping(device_id.as_bytes().as_ptr(), 16);
                out_public_key_32.copy_from_nonoverlapping(public_key.as_bytes().as_ptr(), 32);
                *out_listen_port = *listen_port;
            }
            0
        }
        _ => -1,
    }
}

const HANDSHAKE_SIZE: usize = 1 + 16 + 32;

/// Fill out_buf with handshake bytes (49: version + device_id + public_key). Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_handshake_bytes(h: *mut c_void, out_buf: *mut u8, out_buf_len: usize) -> c_int {
    if h.is_null() || out_buf.is_null() || out_buf_len < HANDSHAKE_SIZE {
        return -1;
    }
    let core = unsafe { &*(h as *const PeaPodCore) };
    let bytes = core.handshake_bytes();
    unsafe {
        out_buf.copy_from_nonoverlapping(bytes.as_ptr(), HANDSHAKE_SIZE);
    }
    0
}

/// Derive session key for a peer. Fills out_session_key_32 (32 bytes). Returns 0 on success, -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_session_key(
    h: *mut c_void,
    peer_public_key_32: *const u8,
    out_session_key_32: *mut u8,
) -> c_int {
    if h.is_null() || peer_public_key_32.is_null() || out_session_key_32.is_null() {
        return -1;
    }
    let core = unsafe { &*(h as *const PeaPodCore) };
    let pk = unsafe { slice::from_raw_parts(peer_public_key_32, 32) };
    let mut arr = [0u8; 32];
    arr.copy_from_slice(pk);
    let peer_public = PublicKey(arr);
    let key = core.session_key(&peer_public);
    unsafe {
        out_session_key_32.copy_from_nonoverlapping(key.as_ptr(), 32);
    }
    0
}

/// Encrypt plaintext for wire. Output is ciphertext (plain_len + 16 for tag). Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_encrypt_wire(
    session_key_32: *const u8,
    nonce: u64,
    plain: *const u8,
    plain_len: usize,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if session_key_32.is_null() || plain.is_null() || out_buf.is_null() {
        return -1;
    }
    let key = unsafe { slice::from_raw_parts(session_key_32, 32) };
    if key.len() != 32 {
        return -1;
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key);
    let plain_slice = unsafe { slice::from_raw_parts(plain, plain_len) };
    let cipher = match encrypt_wire(&key_arr, nonce, plain_slice) {
        Ok(c) => c,
        Err(_) => return -1,
    };
    if cipher.len() > out_buf_len {
        return -1;
    }
    unsafe {
        out_buf.copy_from_nonoverlapping(cipher.as_ptr(), cipher.len());
    }
    cipher.len() as c_int
}

/// Decrypt ciphertext from wire. Output is plaintext (cipher_len - 16). Returns bytes written, or -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_decrypt_wire(
    session_key_32: *const u8,
    nonce: u64,
    cipher: *const u8,
    cipher_len: usize,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if session_key_32.is_null() || cipher.is_null() || out_buf.is_null() {
        return -1;
    }
    let key = unsafe { slice::from_raw_parts(session_key_32, 32) };
    if key.len() != 32 {
        return -1;
    }
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(key);
    let cipher_slice = unsafe { slice::from_raw_parts(cipher, cipher_len) };
    let plain = match decrypt_wire(&key_arr, nonce, cipher_slice) {
        Ok(p) => p,
        Err(_) => return -1,
    };
    if plain.len() > out_buf_len {
        return -1;
    }
    unsafe {
        out_buf.copy_from_nonoverlapping(plain.as_ptr(), plain.len());
    }
    plain.len() as c_int
}

/// On incoming request. url_len is byte length of url (UTF-8). range_end > range_start for a valid range; else treated as no range.
/// out_buf when Accelerate: 16 transfer_id, 8 total_length (LE), 4 num (LE), then num*(16 device_id, 8 start LE, 8 end LE).
/// Returns: 0 = Fallback, 1 = Accelerate (out_buf filled), -1 = error (e.g. out_buf too small).
#[no_mangle]
pub extern "C" fn pea_core_on_request(
    h: *mut c_void,
    url: *const u8,
    url_len: usize,
    range_start: u64,
    range_end: u64,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || url.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let url_slice = unsafe { slice::from_raw_parts(url, url_len) };
    let url_str = match std::str::from_utf8(url_slice) {
        Ok(s) => s,
        Err(_) => return -1,
    };
    let range = if range_end > range_start {
        Some((range_start, range_end))
    } else {
        None
    };
    let action = core.on_incoming_request(url_str, range);
    match action {
        Action::Fallback => 0,
        Action::Accelerate {
            transfer_id,
            total_length,
            assignment,
        } => {
            let need = 16 + 8 + 4 + assignment.len() * (16 + 8 + 8);
            if out_buf.is_null() || out_buf_len < need {
                return -1;
            }
            let buf = unsafe { slice::from_raw_parts_mut(out_buf, out_buf_len) };
            buf[0..16].copy_from_slice(&transfer_id);
            buf[16..24].copy_from_slice(&total_length.to_le_bytes());
            let n = assignment.len() as u32;
            buf[24..28].copy_from_slice(&n.to_le_bytes());
            for (i, (chunk_id, device_id)) in assignment.iter().enumerate() {
                let base = 28 + i * 32;
                buf[base..base + 16].copy_from_slice(device_id.as_bytes());
                buf[base + 16..base + 24].copy_from_slice(&chunk_id.start.to_le_bytes());
                buf[base + 24..base + 32].copy_from_slice(&chunk_id.end.to_le_bytes());
            }
            1
        }
    }
}

/// Peer joined. device_id_16 and public_key_32 must be non-null and at least 16 and 32 bytes.
#[no_mangle]
pub extern "C" fn pea_core_peer_joined(
    h: *mut c_void,
    device_id_16: *const u8,
    public_key_32: *const u8,
) -> c_int {
    if h.is_null() || device_id_16.is_null() || public_key_32.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let mut id = [0u8; 16];
    let mut pk = [0u8; 32];
    unsafe {
        id.copy_from_slice(slice::from_raw_parts(device_id_16, 16));
        pk.copy_from_slice(slice::from_raw_parts(public_key_32, 32));
    }
    let peer_id = DeviceId(id);
    let public_key = PublicKey(pk);
    core.on_peer_joined(peer_id, &public_key);
    0
}

/// Peer left. Optionally writes outbound actions (e.g. ChunkRequests) to out_buf. Returns bytes written to out_buf, or 0 if none/null.
#[no_mangle]
pub extern "C" fn pea_core_peer_left(
    h: *mut c_void,
    device_id_16: *const u8,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || device_id_16.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let mut id = [0u8; 16];
    unsafe {
        id.copy_from_slice(slice::from_raw_parts(device_id_16, 16));
    }
    let actions = core.on_peer_left(DeviceId(id));
    if actions.is_empty() || out_buf.is_null() {
        return 0;
    }
    write_outbound_actions(&actions, out_buf, out_buf_len)
}

/// Serialize outbound actions to out_buf: 4 bytes count (LE), then each (16 peer_id, 4 len LE, payload).
/// Returns number of bytes written, or -1 on error.
fn write_outbound_actions(actions: &[crate::OutboundAction], out_buf: *mut u8, out_buf_len: usize) -> c_int {
    if out_buf.is_null() {
        return -1;
    }
    let mut need = 4;
    for a in actions {
        if let crate::OutboundAction::SendMessage(_, ref bytes) = a {
            need += 16 + 4 + bytes.len();
        }
    }
    if out_buf_len < need {
        return -1;
    }
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, out_buf_len) };
    buf[0..4].copy_from_slice(&(actions.len() as u32).to_le_bytes());
    let mut off = 4;
    for a in actions {
        if let crate::OutboundAction::SendMessage(peer_id, bytes) = a {
            buf[off..off + 16].copy_from_slice(peer_id.as_bytes());
            off += 16;
            let len = bytes.len() as u32;
            buf[off..off + 4].copy_from_slice(&len.to_le_bytes());
            off += 4;
            buf[off..off + bytes.len()].copy_from_slice(bytes);
            off += bytes.len();
        }
    }
    off as c_int
}

/// On message received from peer. Serializes outbound actions (and optional completed body) to out_buf.
/// Layout: 4 bytes completed_body_len (LE), 0 or body_len bytes of body, then same as write_outbound_actions.
/// If completed_body_len > 0, the transfer is complete and body follows. Returns total bytes written, -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_on_message_received(
    h: *mut c_void,
    peer_id_16: *const u8,
    msg: *const u8,
    msg_len: usize,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || peer_id_16.is_null() || msg.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let mut id = [0u8; 16];
    unsafe {
        id.copy_from_slice(slice::from_raw_parts(peer_id_16, 16));
    }
    let peer_id = DeviceId(id);
    let frame = unsafe { slice::from_raw_parts(msg, msg_len) };
    let (actions, completed) = match core.on_message_received(peer_id, frame) {
        Ok(x) => x,
        Err(_) => return -1,
    };
    let body_len = completed.as_ref().map(|(_, b)| b.len()).unwrap_or(0);
    let mut need = 4 + body_len;
    for a in &actions {
        if let crate::OutboundAction::SendMessage(_, ref bytes) = a {
            need += 16 + 4 + bytes.len();
        }
    }
    if out_buf.is_null() || out_buf_len < need {
        return -1;
    }
    let buf = unsafe { slice::from_raw_parts_mut(out_buf, out_buf_len) };
    buf[0..4].copy_from_slice(&(body_len as u32).to_le_bytes());
    let mut off = 4;
    if let Some((_, body)) = completed {
        buf[off..off + body.len()].copy_from_slice(&body);
        off += body.len();
    }
    let n = write_outbound_actions(&actions, buf[off..].as_mut_ptr(), out_buf_len - off);
    if n < 0 {
        return -1;
    }
    (off as c_int) + n
}

/// On chunk received. Returns 0 = in progress, 1 = complete (reassembled body in out_buf), -1 = error.
#[no_mangle]
pub extern "C" fn pea_core_on_chunk_received(
    h: *mut c_void,
    transfer_id_16: *const u8,
    start: u64,
    end: u64,
    hash_32: *const u8,
    payload: *const u8,
    payload_len: usize,
    out_buf: *mut u8,
    out_buf_len: usize,
) -> c_int {
    if h.is_null() || transfer_id_16.is_null() || hash_32.is_null() || payload.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let mut tid = [0u8; 16];
    let mut hash = [0u8; 32];
    unsafe {
        tid.copy_from_slice(slice::from_raw_parts(transfer_id_16, 16));
        hash.copy_from_slice(slice::from_raw_parts(hash_32, 32));
    }
    let payload_vec = unsafe { slice::from_raw_parts(payload, payload_len).to_vec() };
    match core.on_chunk_received(tid, start, end, hash, payload_vec) {
        Ok(None) => 0,
        Ok(Some(body)) => {
            if out_buf.is_null() || out_buf_len < body.len() {
                return -1;
            }
            unsafe {
                out_buf.copy_from_nonoverlapping(body.as_ptr(), body.len());
            }
            1
        }
        Err(_) => -1,
    }
}

/// Tick. Writes serialized outbound actions to out_buf. Returns bytes written, 0 if none, -1 on error.
#[no_mangle]
pub extern "C" fn pea_core_tick(h: *mut c_void, out_buf: *mut u8, out_buf_len: usize) -> c_int {
    if h.is_null() {
        return -1;
    }
    let core = unsafe { &mut *(h as *mut PeaPodCore) };
    let actions = core.tick();
    if actions.is_empty() {
        return 0;
    }
    write_outbound_actions(&actions, out_buf, out_buf_len)
}
