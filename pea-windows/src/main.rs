// PeaPod Windows: proxy, discovery, tray per .tasks/02-windows.md.
#![cfg_attr(windows, windows_subsystem = "windows")]

mod proxy;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::BOOL;
        let _ = BOOL(1);
    }
    let _ = pea_core::Config::default();

    let core = std::sync::Arc::new(tokio::sync::Mutex::new(pea_core::PeaPodCore::new()));
    let bind: std::net::SocketAddr = proxy::DEFAULT_PROXY_ADDR.parse()?;
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(proxy::run_proxy(bind, core))?;
    Ok(())
}
