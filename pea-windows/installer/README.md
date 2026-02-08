# PeaPod Windows Installer (Inno Setup)

Installer for the Windows protocol implementation (§7.1). Builds a single `.exe` setup that installs the PeaPod binary and Start Menu shortcut. Uninstall restores the system proxy if PeaPod was enabled.

## Prerequisites

- [Inno Setup](https://jrsoftware.org/isdownload.php) (e.g. 6.x) installed and `iscc` on PATH.
- Built release binary: from repo root run  
  `cargo build -p pea-windows --release`  
  (output: `target/release/pea_windows.exe`).

## Build the installer

From the **repo root**:

```batch
iscc /DSourceExe=target\release\pea_windows.exe pea-windows\installer\PeaPod.iss
```

Or from this directory (`pea-windows/installer`):

```batch
iscc PeaPod.iss
```
(This uses the default path `..\..\target\release\pea_windows.exe`.)

The setup executable is created in `pea-windows/installer/output/PeaPod-Setup-0.1.0.exe`.

## Behavior

- **Install:** Copies `pea_windows.exe` to e.g. `%LOCALAPPDATA%\Programs\PeaPod` and creates Start Menu shortcuts. Does **not** set the system proxy (§7.1.2); the user enables it via the app (tray → Enable).
- **Uninstall:** Runs `pea_windows.exe --restore-proxy` before removing files, then deletes the app files (§7.1.3). This restores the proxy state from `%APPDATA%\PeaPod\proxy_backup.json` if it exists.
