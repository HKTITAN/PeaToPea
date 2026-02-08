// PeaPod Linux: proxy, discovery, transport daemon per .tasks/04-linux.md.

mod config;
mod proxy;
mod discovery;
mod transport;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() -> Result<(), Box<dyn std::error::Error>> {
    for arg in std::env::args().skip(1) {
        if arg == "--version" || arg == "-V" {
            println!("pea-linux {}", VERSION);
            return Ok(());
        }
    }

    let _ = pea_core::Config::default();
    let cfg = config::load();

    let keypair = std::sync::Arc::new(pea_core::Keypair::generate());
    let core = std::sync::Arc::new(tokio::sync::Mutex::new(
        pea_core::PeaPodCore::with_keypair_arc(keypair.clone()),
    ));

    let bind: std::net::SocketAddr = format!("127.0.0.1:{}", cfg.proxy_port).parse()?;
    let (connect_tx, connect_rx) = tokio::sync::mpsc::unbounded_channel();
    let peer_senders = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
    let transfer_waiters: transport::TransferWaiters =
        std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        tokio::spawn(proxy::run_proxy(
            bind,
            core.clone(),
            peer_senders.clone(),
            transfer_waiters.clone(),
        ));
        let core_disc = core.clone();
        let keypair_disc = keypair.clone();
        let disc_port = cfg.discovery_port;
        let transport_port = cfg.transport_port;
        tokio::spawn(async move {
            let _ = discovery::run_discovery(
                core_disc,
                keypair_disc,
                disc_port,
                transport_port,
                connect_tx,
            )
            .await;
        });
        let core_trans = core.clone();
        let keypair_trans = keypair.clone();
        let transport_port = cfg.transport_port;
        tokio::spawn(async move {
            let _ = transport::run_transport(
                core_trans,
                keypair_trans,
                transport_port,
                connect_rx,
                peer_senders,
                transfer_waiters,
            )
            .await;
        });
        shutdown_signal().await?;
    })?;
    Ok(())
}

/// Wait for Ctrl+C or SIGTERM (Unix). On shutdown, runtime and tasks exit; systemd may restart if configured.
async fn shutdown_signal() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate())
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e))?;
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {}
            _ = sigterm.recv() => {}
        }
    }
    #[cfg(not(unix))]
    {
        tokio::signal::ctrl_c().await?;
    }
    Ok(())
}
