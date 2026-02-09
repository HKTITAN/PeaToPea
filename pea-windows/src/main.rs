// PeaPod Windows: proxy, discovery, transport, tray per .tasks/02-windows.md.
#![cfg_attr(windows, windows_subsystem = "windows")]

#[allow(dead_code)]
mod discovery;
mod proxy;
#[allow(dead_code)]
mod transport;

#[cfg(windows)]
mod autostart;
#[cfg(windows)]
mod system_proxy;
#[cfg(windows)]
mod tray;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(windows)]
    {
        // Uninstaller runs "pea-windows.exe --restore-proxy" to restore system proxy before removing files.
        if std::env::args().any(|a| a == "--restore-proxy") {
            let _ = system_proxy::restore_system_proxy();
            return Ok(());
        }
    }

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
            use std::sync::atomic::AtomicBool;
            use windows::Win32::Foundation::{LPARAM, WPARAM};
            use windows::Win32::UI::WindowsAndMessaging::PostMessageW;

            let (connect_tx, connect_rx) = tokio::sync::mpsc::unbounded_channel();
            let peer_senders: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<pea_core::DeviceId, tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>> = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let transfer_waiters: transport::TransferWaiters =
                std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let (tray_tx, mut tray_rx) = tokio::sync::mpsc::unbounded_channel::<tray::TrayCommand>();
            let (state_tx, state_rx) = tokio::sync::mpsc::unbounded_channel::<tray::TrayStateUpdate>();
            let (hwnd_tx, hwnd_rx) = tokio::sync::oneshot::channel();
            let proxy_enabled = std::sync::Arc::new(AtomicBool::new(true));

            std::thread::spawn(move || {
                let _ = tray::run_tray(tray_tx, state_rx, hwnd_tx);
            });
            let tray_hwnd = hwnd_rx.await.expect("tray failed to send hwnd");

            let state_tx_updater = state_tx.clone();
            let tray_hwnd_updater = tray_hwnd;
            let proxy_enabled_updater = proxy_enabled.clone();
            let peer_senders_updater = peer_senders.clone();
            tokio::spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    let enabled = proxy_enabled_updater.load(std::sync::atomic::Ordering::Relaxed);
                    let senders = peer_senders_updater.lock().await;
                    let peer_count = senders.len() as u32;
                    let peer_ids = senders.keys().map(|d| *d.as_bytes()).collect();
                    drop(senders);
                    let autostart_enabled = autostart::is_autostart_enabled().unwrap_or(false);
                    let _ = state_tx_updater.send(tray::TrayStateUpdate {
                        enabled,
                        peer_count,
                        peer_ids,
                        autostart_enabled,
                    });
                    let _ = PostMessageW(
                        tray_hwnd_updater,
                        tray::WM_TRAY_UPDATE_STATE,
                        WPARAM(0),
                        LPARAM(0),
                    );
                }
            });

            // Initial state so tooltip and settings have data before first 2s tick.
            let autostart_enabled = autostart::is_autostart_enabled().unwrap_or(false);
            let _ = state_tx.send(tray::TrayStateUpdate {
                enabled: true,
                peer_count: 0,
                peer_ids: vec![],
                autostart_enabled,
            });
            let _ = PostMessageW(
                tray_hwnd,
                tray::WM_TRAY_UPDATE_STATE,
                WPARAM(0),
                LPARAM(0),
            );

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
                                proxy_enabled.store(true, std::sync::atomic::Ordering::Relaxed);
                                let _ = system_proxy::set_system_proxy(host, port);
                            }
                            tray::TrayCommand::Disable => {
                                proxy_enabled.store(false, std::sync::atomic::Ordering::Relaxed);
                                let _ = system_proxy::restore_system_proxy();
                            }
                            tray::TrayCommand::SetAutostart(enable) => {
                                let _ = autostart::set_autostart(enable);
                            }
                            tray::TrayCommand::OpenSettings => {
                                let senders = peer_senders.lock().await;
                                let peer_ids = senders.keys().map(|d| *d.as_bytes()).collect();
                                let peer_count = peer_ids.len() as u32;
                                let enabled = proxy_enabled.load(std::sync::atomic::Ordering::Relaxed);
                                let autostart_enabled = autostart::is_autostart_enabled().unwrap_or(false);
                                drop(senders);
                                let _ = state_tx.send(tray::TrayStateUpdate {
                                    enabled,
                                    peer_count,
                                    peer_ids,
                                    autostart_enabled,
                                });
                                let _ = PostMessageW(
                                    tray_hwnd,
                                    tray::WM_TRAY_UPDATE_STATE,
                                    WPARAM(0),
                                    LPARAM(0),
                                );
                                let _ = PostMessageW(
                                    tray_hwnd,
                                    tray::WM_SHOW_SETTINGS,
                                    WPARAM(0),
                                    LPARAM(0),
                                );
                            }
                            tray::TrayCommand::Exit => break,
                        }
                        // Update tooltip immediately after Enable/Disable/SetAutostart
                        let enabled = proxy_enabled.load(std::sync::atomic::Ordering::Relaxed);
                        let senders = peer_senders.lock().await;
                        let peer_ids = senders.keys().map(|d| *d.as_bytes()).collect();
                        let peer_count = senders.len() as u32;
                        let autostart_enabled = autostart::is_autostart_enabled().unwrap_or(false);
                        drop(senders);
                        let _ = state_tx.send(tray::TrayStateUpdate {
                            enabled,
                            peer_count,
                            peer_ids,
                            autostart_enabled,
                        });
                        let _ = PostMessageW(
                            tray_hwnd,
                            tray::WM_TRAY_UPDATE_STATE,
                            WPARAM(0),
                            LPARAM(0),
                        );
                    }
                    _ = tokio::signal::ctrl_c() => break,
                }
            }
            proxy_enabled.store(false, std::sync::atomic::Ordering::Relaxed);
            let _ = system_proxy::restore_system_proxy();
        }
        #[cfg(not(windows))]
        {
            let peer_senders: std::sync::Arc<tokio::sync::Mutex<std::collections::HashMap<pea_core::DeviceId, tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>> = std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            let transfer_waiters: transport::TransferWaiters =
                std::sync::Arc::new(tokio::sync::Mutex::new(std::collections::HashMap::new()));
            proxy::run_proxy(bind, core, peer_senders, transfer_waiters).await.ok();
        }
    });
    Ok(())
}
