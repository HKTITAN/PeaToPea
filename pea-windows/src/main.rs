// PeaPod Windows: proxy, discovery, transport, tray per .tasks/02-windows.md.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod proxy;
mod discovery;
mod transport;

#[cfg(windows)]
mod system_proxy;
#[cfg(windows)]
mod tray;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::BOOL;
        let _ = BOOL(1);
    }
    let _ = pea_core::Config::default();

    let keypair = std::sync::Arc::new(pea_core::Keypair::generate());
    let core = std::sync::Arc::new(tokio::sync::Mutex::new(
        pea_core::PeaPodCore::with_keypair_arc(keypair.clone()),
    ));
    let bind: std::net::SocketAddr = proxy::DEFAULT_PROXY_ADDR.parse()?;

    #[cfg(windows)]
    {
        let (host, port) = ("127.0.0.1", 3128u16);
        system_proxy::set_system_proxy(host, port)?;
    }

    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        #[cfg(windows)]
        {
            let (connect_tx, connect_rx) = tokio::sync::mpsc::unbounded_channel();
            let peer_senders = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let transfer_waiters: transport::TransferWaiters =
                std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let (tray_tx, mut tray_rx) = tokio::sync::mpsc::unbounded_channel::<tray::TrayCommand>();
            let tray_tx_for_thread = tray_tx.clone();
            std::thread::spawn(move || {
                let _ = tray::run_tray(tray_tx_for_thread);
            });
            tokio::spawn(proxy::run_proxy(
                bind,
                core.clone(),
                peer_senders.clone(),
                transfer_waiters.clone(),
            ));
            let core_disc = core.clone();
            let keypair_disc = keypair.clone();
            tokio::spawn(async move {
                let _ =
                    discovery::run_discovery(core_disc, keypair_disc, discovery::LOCAL_TRANSPORT_PORT, connect_tx).await;
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
            let (host, port) = ("127.0.0.1", 3128u16);
            loop {
                tokio::select! {
                    Some(cmd) = tray_rx.recv() => {
                        match cmd {
                            tray::TrayCommand::Enable => {
                                let _ = system_proxy::set_system_proxy(host, port);
                            }
                            tray::TrayCommand::Disable => {
                                let _ = system_proxy::restore_system_proxy();
                            }
                            tray::TrayCommand::OpenSettings => {
                                // TODO: open settings window (§6.2)
                            }
                            tray::TrayCommand::Exit => break,
                        }
                    }
                    _ = tokio::signal::ctrl_c() => break,
                }
            }
            let _ = system_proxy::restore_system_proxy();
        }
        #[cfg(not(windows))]
        {
            let peer_senders = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let transfer_waiters: transport::TransferWaiters =
                std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            proxy::run_proxy(bind, core, peer_senders, transfer_waiters).await.ok();
        }
    })?;
    Ok(())
}
