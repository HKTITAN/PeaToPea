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

The daemon listens on (default ports; see Configuration):

- **Proxy:** `127.0.0.1:3128` (HTTP/HTTPS proxy)
- **Discovery:** UDP port 45678 (multicast 239.255.60.60)
- **Local transport:** TCP port 45679

Stop with Ctrl+C.

## System proxy (using the daemon)

- **Enabling:** Point your apps at the local proxy. Set `HTTP_PROXY` and `HTTPS_PROXY` in the session where you run browsers/terminals (e.g. `export HTTP_PROXY=http://127.0.0.1:3128 HTTPS_PROXY=http://127.0.0.1:3128`). The daemon does not set these for you.
- **Global effect:** Configure your desktop (e.g. GNOME Settings → Network → Proxy) or shell profile so all apps use the proxy.
- **Disabling:** Stop the daemon and unset the variables (e.g. `unset HTTP_PROXY HTTPS_PROXY`) or change desktop proxy back to Off.

If you use an upstream proxy (e.g. corporate), the daemon forwards ineligible traffic directly to the origin host; future support for forwarding via `HTTP_PROXY`/`HTTPS_PROXY` may be added.

## Firewall

Allow inbound UDP on the discovery port and TCP on the transport port so other devices can discover and connect:

- **ufw:** `sudo ufw allow 45678/udp` and `sudo ufw allow 45679/tcp` (or your configured ports)
- **firewalld:** `sudo firewall-cmd --add-port=45678/udp --permanent` and `sudo firewall-cmd --add-port=45679/tcp --permanent`, then `sudo firewall-cmd --reload`

If you change ports via config or env, open those instead.

## Configuration

Config file (optional): `~/.config/peapod/config.toml` or `/etc/peapod/config.toml`. First existing file wins.

Example `config.toml`:

```toml
proxy_port = 3128
discovery_port = 45678
transport_port = 45679
```

Environment overrides (no config file required):

- `PEAPOD_PROXY_PORT` — proxy listen port
- `PEAPOD_DISCOVERY_PORT` — discovery UDP port
- `PEAPOD_TRANSPORT_PORT` — local transport TCP port

## systemd (user service)

To run pea-linux as a user service (starts on login, restarts on failure):

1. Copy the unit file to your user systemd directory:
   ```bash
   mkdir -p ~/.config/systemd/user
   cp pea-linux/misc/peapod.service ~/.config/systemd/user/
   ```
2. Edit `ExecStart` in `~/.config/systemd/user/peapod.service` so the path points to your `pea-linux` binary (e.g. `/home/you/.local/bin/pea-linux` or ` /path/to/target/release/pea-linux`).
3. Enable and start:
   ```bash
   systemctl --user daemon-reload
   systemctl --user enable peapod
   systemctl --user start peapod
   ```
4. To stop: `systemctl --user stop peapod`. To disable: `systemctl --user disable peapod`.

The unit file is in `pea-linux/misc/peapod.service` in the repo.
