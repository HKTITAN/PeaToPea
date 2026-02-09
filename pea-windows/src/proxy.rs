//! Local HTTP/HTTPS proxy: listen on localhost, parse requests, hand eligible GETs to core; forward rest.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use pea_core::chunk::chunk_request_message;
use pea_core::wire::encode_frame;
use pea_core::{Action, ChunkId, PeaPodCore};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{mpsc, Mutex};

/// Default proxy bind address (localhost).
pub const DEFAULT_PROXY_ADDR: &str = "127.0.0.1:3128";

/// Run the proxy: accept connections and handle each with the shared core.
/// peer_senders: send ChunkRequest frames to peers. transfer_waiters: register (transfer_id, tx) and wait for body.
pub async fn run_proxy(
    bind: SocketAddr,
    core: Arc<Mutex<PeaPodCore>>,
    peer_senders: Arc<Mutex<HashMap<pea_core::DeviceId, mpsc::UnboundedSender<Vec<u8>>>>>,
    transfer_waiters: crate::transport::TransferWaiters,
) -> std::io::Result<()> {
    let listener = TcpListener::bind(bind).await?;
    loop {
        let (stream, _) = listener.accept().await?;
        let core = core.clone();
        let peer_senders = peer_senders.clone();
        let transfer_waiters = transfer_waiters.clone();
        tokio::spawn(async move {
            let _ = handle_client(stream, core, peer_senders, transfer_waiters).await;
        });
    }
}

/// Check if this request is eligible for acceleration: GET with optional Range.
fn is_eligible(method: &[u8], _path: &[u8]) -> bool {
    method.eq_ignore_ascii_case(b"GET")
}

/// Parse the first line and headers; return (method, path, host, range).
fn parse_request(buf: &[u8]) -> Option<(Vec<u8>, Vec<u8>, Option<String>, Option<(u64, u64)>)> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    let status = req.parse(buf).ok()?;
    if !status.is_complete() {
        return None;
    }
    let method = req.method?.as_bytes().to_vec();
    let path = req.path?.as_bytes().to_vec();
    let mut host = None;
    let mut range = None;
    for h in req.headers.iter() {
        if h.name.eq_ignore_ascii_case("Host") {
            host = Some(String::from_utf8_lossy(h.value).trim().to_string());
        }
        if h.name.eq_ignore_ascii_case("Range") {
            let v = std::str::from_utf8(h.value).ok()?;
            range = parse_range_header(v);
        }
    }
    Some((method, path, host, range))
}

/// Parse "bytes=start-end" or "bytes=start-".
fn parse_range_header(s: &str) -> Option<(u64, u64)> {
    let s = s.trim().strip_prefix("bytes=")?;
    let (a, b) = s.split_once('-')?;
    let start: u64 = a.trim().parse().ok()?;
    let end = b.trim();
    let end = if end.is_empty() {
        None
    } else {
        Some(end.parse::<u64>().ok()?)
    };
    let end = match end {
        Some(e) => e,
        None => return None, // bytes=0- open-ended; we don't know length, fallback
    };
    if end < start {
        return None;
    }
    // HTTP Range end is inclusive (e.g. bytes=0-99 means 100 bytes).
    Some((start, end))
}

async fn handle_client(
    mut client: TcpStream,
    core: Arc<Mutex<PeaPodCore>>,
    peer_senders: Arc<Mutex<HashMap<pea_core::DeviceId, mpsc::UnboundedSender<Vec<u8>>>>>,
    transfer_waiters: crate::transport::TransferWaiters,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 65536];
    let n = client.read(&mut buf).await?;
    if n == 0 {
        return Ok(());
    }
    let buf = &buf[..n];

    // CONNECT: tunnel (no parsing of HTTPS body in v1)
    if buf.starts_with(b"CONNECT ") {
        return tunnel_connect(&mut client, buf).await;
    }

    // HTTP: parse and decide
    let (method, path, host, range) = match parse_request(buf) {
        Some(t) => t,
        None => return forward_raw(&mut client, buf).await,
    };

    let host = match host {
        Some(h) => h,
        None => return forward_raw(&mut client, buf).await,
    };

    if !is_eligible(&method, &path) {
        return forward_raw(&mut client, buf).await;
    }

    let path_str = String::from_utf8_lossy(&path);
    let url = if path_str.starts_with("http://") || path_str.starts_with("https://") {
        path_str.to_string()
    } else {
        format!("http://{}{}", host, path_str)
    };

    let range_opt = range;
    let action = {
        let mut c = core.lock().await;
        c.on_incoming_request(&url, range_opt)
    };

    match action {
        Action::Fallback => forward_raw(&mut client, buf).await,
        Action::Accelerate {
            transfer_id,
            total_length,
            assignment,
        } => {
            accelerate_response(
                &mut client,
                core,
                transfer_id,
                total_length,
                assignment,
                &url,
                peer_senders,
                transfer_waiters,
            )
            .await
        }
    }
}

/// Tunnel CONNECT: connect to host:port, 200 to client, then bidirectional copy.
async fn tunnel_connect(client: &mut TcpStream, buf: &[u8]) -> std::io::Result<()> {
    let mut headers = [httparse::EMPTY_HEADER; 8];
    let mut req = httparse::Request::new(&mut headers);
    let _ = req.parse(buf).ok();
    let path = req.path.unwrap_or("");
    let (host, port) = match path.split_once(':') {
        Some((h, p)) => (h, p.parse::<u16>().unwrap_or(443)),
        None => return Ok(()),
    };
    let upstream = match TcpStream::connect((host, port)).await {
        Ok(s) => s,
        Err(_) => {
            let _ = client
                .write_all(b"HTTP/1.1 502 Bad Gateway\r\nConnection: close\r\n\r\n")
                .await;
            return Ok(());
        }
    };
    let _ = client
        .write_all(b"HTTP/1.1 200 Connection Established\r\nConnection: close\r\n\r\n")
        .await;
    let (mut cr, mut cw) = client.split();
    let (mut ur, mut uw) = upstream.into_split();
    let _ = tokio::join!(
        tokio::io::copy(&mut ur, &mut cw),
        tokio::io::copy(&mut cr, &mut uw)
    );
    Ok(())
}

/// Forward raw request to origin (Host header gives target); stream response back.
async fn forward_raw(client: &mut TcpStream, request: &[u8]) -> std::io::Result<()> {
    let mut headers = [httparse::EMPTY_HEADER; 32];
    let mut req = httparse::Request::new(&mut headers);
    req.parse(request)
        .map_err(|_| std::io::ErrorKind::InvalidData)?;
    let host = req
        .headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("Host"))
        .and_then(|h| std::str::from_utf8(h.value).ok())
        .map(|s| s.trim().to_string());
    let (host, port) = match host.as_deref() {
        Some(h) if h.contains(':') => {
            let (a, b) = h.split_once(':').unwrap();
            (a, b.parse::<u16>().unwrap_or(80))
        }
        Some(h) => (h, 80u16),
        None => return Ok(()),
    };
    let mut upstream = TcpStream::connect((host, port)).await?;
    upstream.write_all(request).await?;
    upstream.flush().await?;
    let (mut cr, mut cw) = client.split();
    let (mut ur, mut uw) = upstream.into_split();
    let _ = tokio::join!(
        tokio::io::copy(&mut ur, &mut cw),
        tokio::io::copy(&mut cr, &mut uw)
    );
    Ok(())
}

/// Execute accelerate path: fetch self chunks via HTTP, request peer chunks over transport; wait for reassembled body and send response.
async fn accelerate_response(
    stream: &mut TcpStream,
    core: Arc<Mutex<PeaPodCore>>,
    transfer_id: [u8; 16],
    _total_length: u64,
    assignment: Vec<(ChunkId, pea_core::DeviceId)>,
    url: &str,
    peer_senders: Arc<Mutex<HashMap<pea_core::DeviceId, mpsc::UnboundedSender<Vec<u8>>>>>,
    transfer_waiters: crate::transport::TransferWaiters,
) -> std::io::Result<()> {
    let self_id = core.lock().await.device_id();
    let (tx, rx) = tokio::sync::oneshot::channel();
    {
        let mut w = transfer_waiters.lock().await;
        w.insert(transfer_id, tx);
    }

    let http_client = reqwest::Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;

    for (chunk_id, peer_id) in &assignment {
        if *peer_id == self_id {
            let end_inclusive = chunk_id.end.saturating_sub(1);
            let range_header = format!("bytes={}-{}", chunk_id.start, end_inclusive);
            let resp = http_client
                .get(url)
                .header("Range", range_header)
                .send()
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let bytes = resp
                .bytes()
                .await
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
            let payload = bytes.to_vec();
            let hash = pea_core::integrity::hash_chunk(&payload);
            let mut c = core.lock().await;
            if let Ok(Some(full_body)) =
                c.on_chunk_received(transfer_id, chunk_id.start, chunk_id.end, hash, payload)
            {
                let _ = transfer_waiters.lock().await.remove(&transfer_id);
                let len = full_body.len();
                let status = "HTTP/1.1 200 OK\r\n";
                let headers = format!("Content-Length: {}\r\nConnection: close\r\n\r\n", len);
                stream.write_all(status.as_bytes()).await?;
                stream.write_all(headers.as_bytes()).await?;
                stream.write_all(&full_body).await?;
                stream.flush().await?;
                return Ok(());
            }
        } else {
            let msg = chunk_request_message(*chunk_id, Some(url.to_string()));
            if let Ok(frame) = encode_frame(&msg) {
                let senders = peer_senders.lock().await;
                if let Some(tx) = senders.get(peer_id) {
                    let _ = tx.send(frame);
                }
            }
        }
    }

    match tokio::time::timeout(Duration::from_secs(30), rx).await {
        Ok(Ok(full_body)) => {
            let _ = transfer_waiters.lock().await.remove(&transfer_id);
            let len = full_body.len();
            let status = "HTTP/1.1 200 OK\r\n";
            let headers = format!("Content-Length: {}\r\nConnection: close\r\n\r\n", len);
            stream.write_all(status.as_bytes()).await?;
            stream.write_all(headers.as_bytes()).await?;
            stream.write_all(&full_body).await?;
            stream.flush().await?;
            Ok(())
        }
        _ => {
            let _ = transfer_waiters.lock().await.remove(&transfer_id);
            Ok(())
        }
    }
}
