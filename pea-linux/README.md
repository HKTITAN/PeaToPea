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

Stop with Ctrl+C. On Unix, SIGTERM (e.g. `systemctl --user stop peapod`) also triggers a graceful exit.

**CLI:** `pea-linux --version` or `pea-linux -V` prints the version and exits. Enable = run the binary; disable = stop it (Ctrl+C, SIGTERM, or stop the systemd service).

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

### System service (optional)

For a system-wide install (e.g. under `/usr/local` or `/opt`), run as a dedicated user for security:

1. Create a `peapod` user: `sudo useradd -r -s /bin/false peapod`
2. Install the binary to `/usr/local/bin/pea-linux` (or `/opt/peapod/bin/pea-linux`).
3. Copy the system unit: `sudo cp pea-linux/misc/peapod-system.service /etc/systemd/system/peapod.service`
4. Edit `ExecStart` if the binary is not in `/usr/local/bin`.
5. `sudo systemctl daemon-reload`, `sudo systemctl enable peapod`, `sudo systemctl start peapod`.

The unit file is `pea-linux/misc/peapod-system.service` (User=peapod, Group=peapod).

## Edge cases

- **No peers:** The proxy runs normally; traffic is forwarded to the origin without acceleration. No extra configuration needed.
- **Graceful shutdown:** On SIGTERM or Ctrl+C, the daemon exits; systemd will restart it if you have `Restart=on-failure` and the service is enabled.
- **Ports:** Default ports (3128, 45678, 45679) do not require root. To use port 80 for the proxy you would need setcap or run as root (not recommended); use a high port and point clients at it instead.

## Optional: netfilter and eBPF (future)

- **netfilter/iptables:** For transparent interception without setting HTTP_PROXY, you can redirect selected traffic to the local proxy port using iptables REDIRECT or DNAT. This typically requires `cap_net_admin` or root. Document the rules (e.g. redirect port 80/443 to 127.0.0.1:3128) and provide an optional script if desired; not required for v1.
- **eBPF:** Traffic redirect via eBPF on modern kernels is a possible future option; document as such.
