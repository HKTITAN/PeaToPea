// PeaPod Windows: proxy, discovery, transport, tray per .tasks/02-windows.md.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod proxy;
mod discovery;
mod transport;

#[cfg(windows)]
mod system_proxy;

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
            tokio::spawn(proxy::run_proxy(bind, core.clone()));
            let core_disc = core.clone();
            let keypair_disc = keypair.clone();
            tokio::spawn(async move {
                let _ =
                    discovery::run_discovery(core_disc, keypair_disc, discovery::LOCAL_TRANSPORT_PORT, connect_tx).await;
            });
            let core_trans = core.clone();
            let keypair_trans = keypair.clone();
            tokio::spawn(async move {
                let _ = transport::run_transport(core_trans, keypair_trans, connect_rx).await;
            });
            tokio::signal::ctrl_c().await.ok();
            let _ = system_proxy::restore_system_proxy();
        }
        #[cfg(not(windows))]
        {
            proxy::run_proxy(bind, core).await.ok();
        }
    })?;
    Ok(())
}
