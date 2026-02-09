// PeaPod Linux: proxy, discovery, transport daemon per .tasks/04-linux.md.

mod config;
mod discovery;
mod proxy;
mod transport;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn print_help() {
    println!("pea-linux {} — PeaPod protocol daemon for Linux", VERSION);
    println!();
    println!("USAGE:");
    println!("    pea-linux [OPTIONS]");
    println!();
    println!("OPTIONS:");
    println!("    -h, --help       Print this help message and exit");
    println!("    -V, --version    Print version and exit");
    println!();
    println!("DESCRIPTION:");
    println!("    Starts the PeaPod daemon: local HTTP proxy, LAN peer discovery,");
    println!("    and encrypted transport. Nearby devices running PeaPod form a");
    println!("    mesh and pool their internet connections.");
    println!();
    println!("    Proxy       127.0.0.1:3128   (HTTP/HTTPS proxy)");
    println!("    Discovery   UDP 45678        (LAN multicast 239.255.60.60)");
    println!("    Transport   TCP 45679        (encrypted peer-to-peer)");
    println!();
    println!("    Stop with Ctrl+C or SIGTERM.");
    println!();
    println!("CONFIGURATION:");
    println!("    Config file (optional, first found wins):");
    println!("      ~/.config/peapod/config.toml");
    println!("      /etc/peapod/config.toml");
    println!();
    println!("    Example config.toml:");
    println!("      proxy_port = 3128");
    println!("      discovery_port = 45678");
    println!("      transport_port = 45679");
    println!();
    println!("ENVIRONMENT VARIABLES (override config file):");
    println!("    PEAPOD_PROXY_PORT       Proxy listen port (default: 3128)");
    println!("    PEAPOD_DISCOVERY_PORT   Discovery UDP port (default: 45678)");
    println!("    PEAPOD_TRANSPORT_PORT   Transport TCP port (default: 45679)");
    println!();
    println!("SYSTEMD:");
    println!("    systemctl --user enable peapod    Enable auto-start on login");
    println!("    systemctl --user start peapod     Start now");
    println!("    systemctl --user status peapod    Check status");
    println!("    systemctl --user stop peapod      Stop");
    println!();
    println!("MORE INFO:");
    println!("    https://github.com/HKTITAN/PeaToPea");
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if let Some(arg) = std::env::args().nth(1) {
        match arg.as_str() {
            "--version" | "-V" => {
                println!("pea-linux {}", VERSION);
                return Ok(());
            }
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => {
                eprintln!("pea-linux: unknown option '{}'\n", other);
                print_help();
                std::process::exit(1);
            }
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
    let peer_senders =
        std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
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
        shutdown_signal().await
    })?;
    Ok(())
}

/// Wait for Ctrl+C or SIGTERM (Unix). On shutdown, runtime and tasks exit; systemd may restart if configured.
async fn shutdown_signal() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{signal, SignalKind};
        let mut sigterm = signal(SignalKind::terminate()).map_err(std::io::Error::other)?;
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
