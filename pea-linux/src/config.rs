//! Load config from file and environment. See .tasks/04-linux.md §6.

use serde::Deserialize;
use std::path::PathBuf;

/// Daemon configuration. File: ~/.config/peapod/config.toml or /etc/peapod/config.toml.
/// Env overrides: PEAPOD_PROXY_PORT, PEAPOD_DISCOVERY_PORT, PEAPOD_TRANSPORT_PORT.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// Proxy listen port (default 3128).
    #[serde(default = "default_proxy_port")]
    pub proxy_port: u16,
    /// Discovery UDP port (default 45678).
    #[serde(default = "default_discovery_port")]
    pub discovery_port: u16,
    /// Local transport TCP port (default 45679).
    #[serde(default = "default_transport_port")]
    pub transport_port: u16,
}

fn default_proxy_port() -> u16 {
    3128
}
fn default_discovery_port() -> u16 {
    45678
}
fn default_transport_port() -> u16 {
    45679
}

impl Default for Config {
    fn default() -> Self {
        Self {
            proxy_port: default_proxy_port(),
            discovery_port: default_discovery_port(),
            transport_port: default_transport_port(),
        }
    }
}

/// Load config: merge default, then config file (if present), then env vars.
pub fn load() -> Config {
    let mut c = load_file().unwrap_or_else(Config::default);
    if let Ok(s) = std::env::var("PEAPOD_PROXY_PORT") {
        if let Ok(p) = s.parse::<u16>() {
            c.proxy_port = p;
        }
    }
    if let Ok(s) = std::env::var("PEAPOD_DISCOVERY_PORT") {
        if let Ok(p) = s.parse::<u16>() {
            c.discovery_port = p;
        }
    }
    if let Ok(s) = std::env::var("PEAPOD_TRANSPORT_PORT") {
        if let Ok(p) = s.parse::<u16>() {
            c.transport_port = p;
        }
    }
    c
}

fn config_paths() -> Vec<PathBuf> {
    let home = std::env::var_os("HOME").map(PathBuf::from);
    let mut out = Vec::new();
    if let Some(h) = home {
        out.push(h.join(".config/peapod/config.toml"));
    }
    out.push(PathBuf::from("/etc/peapod/config.toml"));
    out
}

fn load_file() -> Option<Config> {
    for p in config_paths() {
        if p.exists() {
            if let Ok(s) = std::fs::read_to_string(&p) {
                if let Ok(c) = toml::from_str::<Config>(&s) {
                    return Some(c);
                }
            }
            break;
        }
    }
    None
}
