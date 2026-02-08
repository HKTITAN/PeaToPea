//! "Start PeaPod when I sign in" via HKCU Run key (§7.2). Default: off.

#![cfg(windows)]

const RUN_KEY_PATH: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const VALUE_NAME: &str = "PeaPod";

/// Returns true if PeaPod is registered to run at user sign-in.
pub fn is_autostart_enabled() -> std::io::Result<bool> {
    let exe = std::env::current_exe()?.to_string_lossy().to_string();
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let run = hkcu.open_subkey(RUN_KEY_PATH)?;
    let current: String = run.get_value(VALUE_NAME).unwrap_or_default();
    Ok(!current.is_empty() && current.eq_ignore_ascii_case(&exe))
}

/// Enable or disable run at sign-in. Uses current executable path.
pub fn set_autostart(enabled: bool) -> std::io::Result<()> {
    let hkcu = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER);
    let (run, _) = hkcu.create_subkey(RUN_KEY_PATH)?;
    if enabled {
        let exe = std::env::current_exe()?.to_string_lossy().to_string();
        run.set_value(VALUE_NAME, &exe)?;
    } else {
        let _ = run.delete_value(VALUE_NAME); // ignore if value was not present
    }
    Ok(())
}
