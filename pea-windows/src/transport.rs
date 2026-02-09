//! Local transport: TCP server (incoming), TCP client (outbound to discovered peers), handshake + encrypted frames.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use pea_core::identity::{derive_session_key, PublicKey};
use pea_core::wire::{decode_frame, encode_frame};
use pea_core::{DeviceId, Keypair, Message, OutboundAction, PeaPodCore, PROTOCOL_VERSION};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

use crate::discovery;

const HANDSHAKE_SIZE: usize = 1 + 16 + 32; // version + device_id + public_key
const LEN_SIZE: usize = 4;
const MAX_FRAME_LEN: u32 = 16 * 1024 * 1024;

async fn fetch_range(url: &str, start: u64, end: u64) -> std::io::Result<Vec<u8>> {
    let end_inclusive = end.saturating_sub(1);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(std::io::Error::other)?;
    let range_header = format!("bytes={}-{}", start, end_inclusive);
    let resp = client
        .get(url)
        .header("Range", range_header)
        .send()
        .await
        .map_err(std::io::Error::other)?;
    let bytes = resp.bytes().await.map_err(std::io::Error::other)?;
    Ok(bytes.to_vec())
}

/// Shared: when a transfer completes (reassembled body ready), transport sends it here so the proxy can respond.
pub type TransferWaiters =
    Arc<Mutex<std::collections::HashMap<[u8; 16], tokio::sync::oneshot::Sender<Vec<u8>>>>>;

/// Run transport: listen for incoming TCP, accept connections; connect outbound when peer is pushed to `connect_rx`.
/// `peer_senders` is shared with the proxy so it can send ChunkRequests. `transfer_waiters`: proxy registers (transfer_id, tx); transport sends body on tx when transfer completes.
pub async fn run_transport(
    core: Arc<Mutex<PeaPodCore>>,
    keypair: Arc<Keypair>,
    mut connect_rx: mpsc::UnboundedReceiver<(DeviceId, SocketAddr)>,
    peer_senders: Arc<Mutex<HashMap<DeviceId, mpsc::UnboundedSender<Vec<u8>>>>>,
    transfer_waiters: TransferWaiters,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(("0.0.0.0", discovery::LOCAL_TRANSPORT_PORT)).await?;

    let tick_core = core.clone();
    let tick_senders = peer_senders.clone();
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            let actions = tick_core.lock().await.tick();
            let senders = tick_senders.lock().await;
            for action in actions {
                let OutboundAction::SendMessage(peer, bytes) = action;
                if let Some(tx) = senders.get(&peer) {
                    let _ = tx.send(bytes);
                }
            }
        }
    });

    let accept_core = core.clone();
    let accept_keypair = keypair.clone();
    let accept_senders = peer_senders.clone();
    let accept_waiters = transfer_waiters.clone();
    tokio::spawn(async move {
        while let Ok((mut stream, _)) = listener.accept().await {
            let core = accept_core.clone();
            let keypair = accept_keypair.clone();
            let senders = accept_senders.clone();
            let waiters = accept_waiters.clone();
            tokio::spawn(async move {
                if let Ok((peer_id, session_key)) =
                    handshake_accept(&mut stream, keypair.as_ref()).await
                {
                    run_connection(stream, peer_id, session_key, core, senders, waiters).await;
                }
            });
        }
    });

    while let Some((_peer_id, addr)) = connect_rx.recv().await {
        let core = core.clone();
        let keypair = keypair.clone();
        let senders = peer_senders.clone();
        let waiters = transfer_waiters.clone();
        tokio::spawn(async move {
            if let Ok(mut stream) = TcpStream::connect(addr).await {
                if let Ok((peer_id, session_key)) =
                    handshake_connect(&mut stream, keypair.as_ref()).await
                {
                    run_connection(stream, peer_id, session_key, core, senders, waiters).await;
                }
            }
        });
    }
    Ok(())
}

async fn handshake_accept(
    stream: &mut TcpStream,
    keypair: &Keypair,
) -> std::io::Result<(DeviceId, [u8; 32])> {
    let mut buf = [0u8; HANDSHAKE_SIZE];
    let (mut r, mut w) = stream.split();
    r.read_exact(&mut buf).await?;
    let version = buf[0];
    if version != PROTOCOL_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported protocol version",
        ));
    }
    let mut device_id = [0u8; 16];
    device_id.copy_from_slice(&buf[1..17]);
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&buf[17..49]);
    let peer_id = DeviceId::from_bytes(device_id);
    let peer_public = PublicKey::from_bytes(public_key);

    let secret = keypair.shared_secret(&peer_public);
    let session_key = derive_session_key(&secret);

    let out = handshake_bytes(keypair);
    w.write_all(&out).await?;
    w.flush().await?;
    Ok((peer_id, session_key))
}

async fn handshake_connect(
    stream: &mut TcpStream,
    keypair: &Keypair,
) -> std::io::Result<(DeviceId, [u8; 32])> {
    let (mut r, mut w) = stream.split();
    let out = handshake_bytes(keypair);
    w.write_all(&out).await?;
    w.flush().await?;
    let mut buf = [0u8; HANDSHAKE_SIZE];
    r.read_exact(&mut buf).await?;
    if buf[0] != PROTOCOL_VERSION {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unsupported protocol version",
        ));
    }
    let mut device_id = [0u8; 16];
    device_id.copy_from_slice(&buf[1..17]);
    let mut public_key = [0u8; 32];
    public_key.copy_from_slice(&buf[17..49]);
    let peer_id = DeviceId::from_bytes(device_id);
    let peer_public = PublicKey::from_bytes(public_key);
    let secret = keypair.shared_secret(&peer_public);
    let session_key = derive_session_key(&secret);
    Ok((peer_id, session_key))
}

fn handshake_bytes(keypair: &Keypair) -> [u8; HANDSHAKE_SIZE] {
    let mut out = [0u8; HANDSHAKE_SIZE];
    out[0] = PROTOCOL_VERSION;
    out[1..17].copy_from_slice(keypair.device_id().as_bytes());
    out[17..49].copy_from_slice(keypair.public_key().as_bytes());
    out
}

async fn run_connection(
    stream: TcpStream,
    peer_id: DeviceId,
    session_key: [u8; 32],
    core: Arc<Mutex<PeaPodCore>>,
    peer_senders: Arc<Mutex<HashMap<DeviceId, mpsc::UnboundedSender<Vec<u8>>>>>,
    transfer_waiters: TransferWaiters,
) {
    let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();
    {
        let mut senders = peer_senders.lock().await;
        senders.insert(peer_id, tx);
    }
    let (mut reader, mut writer) = stream.into_split();
    let writer_key = session_key;
    let writer_senders = peer_senders.clone();
    tokio::spawn(async move {
        let mut write_nonce: u64 = 0;
        while let Some(plain) = rx.recv().await {
            if let Ok(cipher) = pea_core::identity::encrypt_wire(&writer_key, write_nonce, &plain) {
                write_nonce = write_nonce.saturating_add(1);
                let len = cipher.len() as u32;
                let _ = writer.write_all(&len.to_le_bytes()).await;
                let _ = writer.write_all(&cipher).await;
                let _ = writer.flush().await;
            }
        }
    });
    let mut read_nonce: u64 = 0;
    loop {
        let mut len_buf = [0u8; LEN_SIZE];
        if reader.read_exact(&mut len_buf).await.is_err() {
            break;
        }
        let len = u32::from_le_bytes(len_buf) as usize;
        if len > MAX_FRAME_LEN as usize {
            break;
        }
        let mut cipher = vec![0u8; len];
        if reader.read_exact(&mut cipher).await.is_err() {
            break;
        }
        let plain = match pea_core::identity::decrypt_wire(&session_key, read_nonce, &cipher) {
            Ok(p) => p,
            Err(_) => break,
        };
        read_nonce = read_nonce.saturating_add(1);
        if let Ok((
            Message::ChunkRequest {
                transfer_id,
                start,
                end,
                url: Some(ref url),
            },
            _,
        )) = decode_frame(&plain)
        {
            if let Ok(body) = fetch_range(url, start, end).await {
                let hash = pea_core::integrity::hash_chunk(&body);
                let chunk_data = Message::ChunkData {
                    transfer_id,
                    start,
                    end,
                    hash,
                    payload: body,
                };
                if let Ok(frame) = encode_frame(&chunk_data) {
                    let senders = writer_senders.lock().await;
                    if let Some(tx) = senders.get(&peer_id) {
                        let _ = tx.send(frame);
                    }
                }
            }
            continue;
        }
        let mut c = core.lock().await;
        if let Ok((actions, completed)) = c.on_message_received(peer_id, &plain) {
            for action in actions {
                let OutboundAction::SendMessage(to_peer, bytes) = action;
                let senders = writer_senders.lock().await;
                if let Some(tx) = senders.get(&to_peer) {
                    let _ = tx.send(bytes);
                }
            }
            if let Some((tid, body)) = completed {
                let mut w = transfer_waiters.lock().await;
                if let Some(tx) = w.remove(&tid) {
                    let _ = tx.send(body);
                }
            }
        }
    }
    let mut senders = peer_senders.lock().await;
    senders.remove(&peer_id);
    drop(senders);
    let mut c = core.lock().await;
    c.on_peer_left(peer_id);
}
