//! System proxy configuration on Windows (registry: Internet Settings).
//! Read current proxy, set to PeaPod localhost:port when enabling, restore when disabling.

#![cfg(windows)]

use std::path::PathBuf;

use winreg::RegKey;

const INTERNET_SETTINGS_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Internet Settings";
const PROXY_ENABLE: &str = "ProxyEnable";
const PROXY_SERVER: &str = "ProxyServer";
const PROXY_OVERRIDE: &str = "ProxyOverride";

/// Saved proxy state to restore when PeaPod is disabled.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SavedProxyState {
    pub enabled: bool,
    pub server: String,
    pub proxy_override: String,
}

/// Current system proxy state (from registry).
#[derive(Clone, Debug, Default)]
pub struct SystemProxyState {
    pub enabled: bool,
    pub server: String,
    pub proxy_override: String,
}

fn app_data_dir() -> std::io::Result<PathBuf> {
    std::env::var_os("APPDATA")
        .map(PathBuf::from)
        .map(|p| p.join("PeaPod"))
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "APPDATA not set"))
}

fn backup_path() -> std::io::Result<PathBuf> {
    Ok(app_data_dir()?.join("proxy_backup.json"))
}

fn open_internet_settings_key() -> std::io::Result<RegKey> {
    let hkcu = RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (key, _) = hkcu.create_subkey(INTERNET_SETTINGS_PATH)?;
    Ok(key)
}

/// Read current system proxy from registry.
pub fn get_system_proxy() -> std::io::Result<SystemProxyState> {
    let key = open_internet_settings_key()?;
    let enabled = key
        .get_value::<u32>(PROXY_ENABLE)
        .map(|v| v != 0)
        .unwrap_or(false);
    let server = key.get_value::<String>(PROXY_SERVER).unwrap_or_default();
    let proxy_override = key.get_value::<String>(PROXY_OVERRIDE).unwrap_or_default();
    Ok(SystemProxyState {
        enabled,
        server,
        proxy_override,
    })
}

/// Save current proxy state to backup file (call before setting our proxy).
fn save_backup(state: &SystemProxyState) -> std::io::Result<()> {
    let path = backup_path()?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let saved = SavedProxyState {
        enabled: state.enabled,
        server: state.server.clone(),
        proxy_override: state.proxy_override.clone(),
    };
    let json = serde_json::to_string_pretty(&saved)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// Load saved proxy state from backup file.
fn load_backup() -> std::io::Result<Option<SavedProxyState>> {
    let path = backup_path()?;
    if !path.exists() {
        return Ok(None);
    }
    let json = std::fs::read_to_string(&path)?;
    let saved: SavedProxyState = serde_json::from_str(&json)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    Ok(Some(saved))
}

/// Remove backup file after successful restore.
fn remove_backup() -> std::io::Result<()> {
    let path = backup_path()?;
    if path.exists() {
        std::fs::remove_file(&path)?;
    }
    Ok(())
}

/// Set system proxy to the given host:port (e.g. 127.0.0.1:3128).
/// Saves current proxy state to backup so it can be restored when disabling.
pub fn set_system_proxy(host: &str, port: u16) -> std::io::Result<()> {
    let current = get_system_proxy()?;
    save_backup(&current)?;
    let key = open_internet_settings_key()?;
    key.set_value(PROXY_ENABLE, &1u32)?;
    let server = format!("{}:{}", host, port);
    key.set_value(PROXY_SERVER, &server)?;
    Ok(())
}

/// Restore system proxy to the previously saved state (when user disables PeaPod).
/// If no backup exists (e.g. first run or backup cleared), disables proxy (ProxyEnable=0).
pub fn restore_system_proxy() -> std::io::Result<()> {
    let key = open_internet_settings_key()?;
    match load_backup()? {
        Some(saved) => {
            key.set_value(PROXY_ENABLE, &(if saved.enabled { 1u32 } else { 0u32 }))?;
            key.set_value(PROXY_SERVER, &saved.server)?;
            key.set_value(PROXY_OVERRIDE, &saved.proxy_override)?;
        }
        None => {
            key.set_value(PROXY_ENABLE, &0u32)?;
        }
    }
    remove_backup()?;
    Ok(())
}

/// Returns true if the current system proxy is set to our address (PeaPod is "on").
pub fn is_proxy_ours(host: &str, port: u16) -> std::io::Result<bool> {
    let state = get_system_proxy()?;
    let ours = format!("{}:{}", host, port);
    Ok(state.enabled && state.server.trim().eq_ignore_ascii_case(&ours))
}
