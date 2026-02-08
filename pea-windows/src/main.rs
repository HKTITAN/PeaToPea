// PeaPod Windows: proxy, discovery, tray per .tasks/02-windows.md.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod proxy;

#[cfg(windows)]
mod system_proxy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::BOOL;
        let _ = BOOL(1);
    }
    let _ = pea_core::Config::default();

    let core = std::sync::Arc::new(tokio::sync::Mutex::new(pea_core::PeaPodCore::new()));
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
            let proxy = proxy::run_proxy(bind, core);
            let ctrl_c = tokio::signal::ctrl_c();
            tokio::select! {
                _ = proxy => {}
                _ = ctrl_c => {
                    let _ = system_proxy::restore_system_proxy();
                }
            }
        }
        #[cfg(not(windows))]
        {
            proxy::run_proxy(bind, core).await.ok();
        }
    })?;
    Ok(())
}
