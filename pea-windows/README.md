# PeaPod Windows

Windows protocol implementation: system proxy (or WinDivert), discovery, local transport, system tray. Uses [pea-core](../pea-core) for protocol logic.

## Build and run

From the **repo root** (requires [Rust](https://rustup.rs) and a Windows host):

```bash
cargo build -p pea-windows
cargo run -p pea-windows
```

The proxy listens on `127.0.0.1:3128` by default. On Windows, running the app sets the system proxy to that address (registry: Internet Settings) and restores the previous proxy when you press Ctrl+C. **Discovery** runs over UDP multicast (239.255.60.60:45678); **local transport** (TCP 45679, handshake + encrypted frames) connects to discovered peers. A **system tray** icon (right-click: Enable / Disable / Open settings / Exit) controls the system proxy and exits the app. The tooltip shows enabled/disabled and "Pod: N devices". **Open settings** opens a small Win32 window: PeaPod enabled checkbox, proxy address (127.0.0.1:3128), and list of pod members (anonymized device IDs).

## Tasks

See [.tasks/02-windows.md](../.tasks/02-windows.md) for the full Windows implementation checklist.
