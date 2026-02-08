// PeaPod Linux: proxy, discovery, transport daemon per .tasks/04-linux.md.

mod proxy;
mod discovery;
mod transport;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = pea_core::Config::default();

    let keypair = std::sync::Arc::new(pea_core::Keypair::generate());
    let core = std::sync::Arc::new(tokio::sync::Mutex::new(
        pea_core::PeaPodCore::with_keypair_arc(keypair.clone()),
    ));

    let bind: std::net::SocketAddr = proxy::DEFAULT_PROXY_ADDR.parse()?;
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
        tokio::spawn(async move {
            let _ = discovery::run_discovery(
                core_disc,
                keypair_disc,
                discovery::LOCAL_TRANSPORT_PORT,
                connect_tx,
            )
            .await;
        });
        let core_trans = core.clone();
        let keypair_trans = keypair.clone();
        tokio::spawn(async move {
            let _ = transport::run_transport(
                core_trans,
                keypair_trans,
                connect_rx,
                peer_senders,
                transfer_waiters,
            )
            .await;
        });
        tokio::signal::ctrl_c().await?;
    })?;
    Ok(())
}
