//! LAN discovery: UDP multicast beacon, parse beacons/responses, maintain peer list.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use pea_core::wire::{decode_frame, encode_frame};
use pea_core::PublicKey;
use pea_core::{DeviceId, Keypair, Message, PeaPodCore, PROTOCOL_VERSION};
use tokio::net::UdpSocket;
use tokio::sync::Mutex;

const MULTICAST_GROUP: &str = "239.255.60.60";
const BEACON_INTERVAL: Duration = Duration::from_secs(4);
const PEER_TIMEOUT: Duration = Duration::from_secs(16);

struct PeerState {
    #[allow(dead_code)]
    public_key: PublicKey,
    #[allow(dead_code)]
    addr: SocketAddr,
    last_seen: Instant,
}

pub async fn run_discovery(
    core: Arc<Mutex<PeaPodCore>>,
    keypair: Arc<Keypair>,
    discovery_port: u16,
    transport_port: u16,
    connect_tx: tokio::sync::mpsc::UnboundedSender<(DeviceId, SocketAddr)>,
) -> std::io::Result<()> {
    let socket = make_multicast_socket(discovery_port).await?;
    let socket = Arc::new(socket);
    let peers: Arc<Mutex<HashMap<DeviceId, PeerState>>> = Arc::new(Mutex::new(HashMap::new()));

    let send_socket = socket.clone();
    let recv_socket = socket.clone();
    let peers_recv = peers.clone();
    let core_recv = core.clone();
    let keypair_recv = keypair.clone();
    let connect_tx_recv = connect_tx.clone();

    let beacon_task = tokio::spawn(async move {
        beacon_loop(send_socket, keypair, discovery_port, transport_port).await
    });
    let recv_task = tokio::spawn(async move {
        recv_loop(
            recv_socket,
            peers_recv,
            core_recv,
            keypair_recv,
            transport_port,
            connect_tx_recv,
        )
        .await
    });
    let timeout_task = tokio::spawn(async move { peer_timeout_loop(peers.clone(), core).await });

    let _ = tokio::try_join!(beacon_task, recv_task, timeout_task);
    Ok(())
}

async fn make_multicast_socket(discovery_port: u16) -> std::io::Result<UdpSocket> {
    let std_sock = std::net::UdpSocket::bind(("0.0.0.0", discovery_port))?;
    let multicast: std::net::Ipv4Addr =
        MULTICAST_GROUP
            .parse()
            .map_err(|e: std::net::AddrParseError| {
                std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
            })?;
    std_sock.join_multicast_v4(&multicast, &"0.0.0.0".parse().unwrap())?;
    std_sock.set_multicast_ttl_v4(1)?;
    tokio::net::UdpSocket::from_std(std_sock)
}

async fn beacon_loop(
    socket: Arc<UdpSocket>,
    keypair: Arc<Keypair>,
    discovery_port: u16,
    transport_port: u16,
) -> std::io::Result<()> {
    let device_id = keypair.device_id();
    let public_key = keypair.public_key().clone();
    let beacon = Message::Beacon {
        protocol_version: PROTOCOL_VERSION,
        device_id,
        public_key,
        listen_port: transport_port,
    };
    let frame = encode_frame(&beacon)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    let dest: SocketAddr = format!("{}:{}", MULTICAST_GROUP, discovery_port)
        .parse()
        .map_err(|e: std::net::AddrParseError| {
            std::io::Error::new(std::io::ErrorKind::InvalidInput, e)
        })?;
    loop {
        let _ = socket.send_to(&frame, dest).await;
        tokio::time::sleep(BEACON_INTERVAL).await;
    }
}

async fn recv_loop(
    socket: Arc<UdpSocket>,
    peers: Arc<Mutex<HashMap<DeviceId, PeerState>>>,
    core: Arc<Mutex<PeaPodCore>>,
    keypair: Arc<Keypair>,
    transport_port: u16,
    connect_tx: tokio::sync::mpsc::UnboundedSender<(DeviceId, SocketAddr)>,
) -> std::io::Result<()> {
    let mut buf = vec![0u8; 65536];
    let my_id = keypair.device_id();
    let my_public = keypair.public_key().clone();
    let response_frame = encode_frame(&Message::DiscoveryResponse {
        protocol_version: PROTOCOL_VERSION,
        device_id: my_id,
        public_key: my_public,
        listen_port: transport_port,
    })
    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    loop {
        match socket.recv_from(&mut buf).await {
            Ok((n, from)) => {
                let buf = &buf[..n];
                if let Ok((msg, _)) = decode_frame(buf) {
                    match &msg {
                        Message::Beacon {
                            protocol_version,
                            device_id,
                            public_key,
                            listen_port,
                        } => {
                            if *protocol_version != PROTOCOL_VERSION {
                                continue;
                            }
                            if *device_id == my_id {
                                continue;
                            }
                            let is_new = {
                                let mut p = peers.lock().await;
                                let is_new = !p.contains_key(device_id);
                                p.insert(
                                    *device_id,
                                    PeerState {
                                        public_key: public_key.clone(),
                                        addr: SocketAddr::new(from.ip(), *listen_port),
                                        last_seen: Instant::now(),
                                    },
                                );
                                is_new
                            };
                            if is_new {
                                let mut c = core.lock().await;
                                c.on_peer_joined(*device_id, public_key);
                                let addr = SocketAddr::new(from.ip(), *listen_port);
                                let _ = connect_tx.send((*device_id, addr));
                            }
                            let _ = socket.send_to(&response_frame, from).await;
                        }
                        Message::DiscoveryResponse {
                            protocol_version,
                            device_id,
                            public_key,
                            listen_port,
                        } => {
                            if *protocol_version != PROTOCOL_VERSION {
                                continue;
                            }
                            if *device_id == my_id {
                                continue;
                            }
                            let is_new = {
                                let mut p = peers.lock().await;
                                let is_new = !p.contains_key(device_id);
                                p.insert(
                                    *device_id,
                                    PeerState {
                                        public_key: public_key.clone(),
                                        addr: SocketAddr::new(from.ip(), *listen_port),
                                        last_seen: Instant::now(),
                                    },
                                );
                                is_new
                            };
                            if is_new {
                                let mut c = core.lock().await;
                                c.on_peer_joined(*device_id, public_key);
                                let addr = SocketAddr::new(from.ip(), *listen_port);
                                let _ = connect_tx.send((*device_id, addr));
                            }
                        }
                        _ => {}
                    }
                }
            }
            Err(e) => return Err(e),
        }
    }
}

async fn peer_timeout_loop(
    peers: Arc<Mutex<HashMap<DeviceId, PeerState>>>,
    core: Arc<Mutex<PeaPodCore>>,
) -> std::io::Result<()> {
    loop {
        tokio::time::sleep(Duration::from_secs(4)).await;
        let now = Instant::now();
        let timed_out: Vec<DeviceId> = {
            let mut p = peers.lock().await;
            let list: Vec<DeviceId> = p
                .iter()
                .filter(|(_, s)| now.duration_since(s.last_seen) >= PEER_TIMEOUT)
                .map(|(id, _)| *id)
                .collect();
            for id in &list {
                p.remove(id);
            }
            list
        };
        for peer_id in timed_out {
            let mut c = core.lock().await;
            c.on_peer_left(peer_id);
        }
    }
}
