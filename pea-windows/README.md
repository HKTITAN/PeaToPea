# PeaPod Windows

Windows protocol implementation: system proxy (or WinDivert), discovery, local transport, system tray. Uses [pea-core](../pea-core) for protocol logic.

## Build and run

From the **repo root** (requires [Rust](https://rustup.rs) and a Windows host):

```bash
cargo build -p pea-windows
cargo run -p pea-windows
```

The proxy listens on `127.0.0.1:3128` by default. On Windows, running the app sets the system proxy to that address (registry: Internet Settings) and restores the previous proxy when you press Ctrl+C (or when the tray "Disable" is used, once implemented). Discovery and tray are not yet implemented (see [.tasks/02-windows.md](../.tasks/02-windows.md)).

## Tasks

See [.tasks/02-windows.md](../.tasks/02-windows.md) for the full Windows implementation checklist.
