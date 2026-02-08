# pea-linux

PeaPod protocol implementation for Linux: daemon (proxy, discovery, local transport). Same wire format and discovery as pea-windows and 07-protocol-and-interop.

## Build

From the repo root:

```bash
cargo build -p pea-linux
cargo build -p pea-linux --release
```

Optional targets: `x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu` (e.g. for cross-compile).

## Run

- **From repo:** `./target/release/pea-linux` or `./target/debug/pea-linux`
- **From PATH:** Install the binary (e.g. to `~/.local/bin` or `/usr/local/bin`) and run `pea-linux`

The daemon listens on:

- **Proxy:** `127.0.0.1:3128` (HTTP/HTTPS proxy)
- **Discovery:** UDP port 45678 (multicast 239.255.60.60)
- **Local transport:** TCP port 45679

To use the proxy, set `HTTP_PROXY` and `HTTPS_PROXY` for your session (e.g. `export HTTP_PROXY=http://127.0.0.1:3128 HTTPS_PROXY=http://127.0.0.1:3128`). For global effect, configure your desktop environment or shell profile. Stop with Ctrl+C.

## Firewall

Allow inbound UDP on port 45678 and TCP on port 45679 (e.g. `ufw allow 45678/udp` and `ufw allow 45679/tcp`, or equivalent in firewalld).
