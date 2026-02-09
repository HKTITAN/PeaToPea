# PeaPod Installer for Windows
# Usage: iwr -useb https://raw.githubusercontent.com/HKTITAN/PeaToPea/main/install.ps1 | iex
#
# This script:
#   1. Checks for prerequisites (Git, Rust)
#   2. Shows you exactly what will be installed
#   3. Asks for confirmation before proceeding
#   4. Builds and installs pea-windows from source
#   5. Adds PeaPod to your PATH
#   6. Optionally sets up auto-start
#
# Environment variables:
#   PEAPOD_PREFIX      - install directory (default: %LOCALAPPDATA%\PeaPod)
#   PEAPOD_NO_CONFIRM  - set to 1 to skip confirmation prompts

$ErrorActionPreference = "Stop"

# ── Colors ──────────────────────────────────────────────────────────

function Write-Info  { Write-Host "[info]  " -ForegroundColor Cyan -NoNewline; Write-Host $args }
function Write-Warn  { Write-Host "[warn]  " -ForegroundColor Yellow -NoNewline; Write-Host $args }
function Write-Err   { Write-Host "[error] " -ForegroundColor Red -NoNewline; Write-Host $args }
function Write-Ok    { Write-Host "[ok]    " -ForegroundColor Green -NoNewline; Write-Host $args }

# ── Banner ──────────────────────────────────────────────────────────

function Show-Banner {
    Write-Host ""
    Write-Host "    ____             ____            __" -ForegroundColor Green
    Write-Host "   / __ \___  ____ _/ __ \____  ____/ /" -ForegroundColor Green
    Write-Host "  / /_/ / _ \/ __ ``/ /_/ / __ \/ __  /" -ForegroundColor Green
    Write-Host " / ____/  __/ /_/ / ____/ /_/ / /_/ /" -ForegroundColor Green
    Write-Host "/_/    \___/\__,_/_/    \____/\__,_/" -ForegroundColor Green
    Write-Host ""
    Write-Host "  PeaPod Installer" -ForegroundColor White -NoNewline
    Write-Host " - Pool internet across your devices"
    Write-Host "  https://github.com/HKTITAN/PeaToPea"
    Write-Host ""
}

# ── Disclaimer ──────────────────────────────────────────────────────

function Show-Disclaimer {
    Write-Host "+-----------------------------------------------------------------+" -ForegroundColor Yellow
    Write-Host "|                     WHAT YOU'RE INSTALLING                       |" -ForegroundColor Yellow
    Write-Host "+-----------------------------------------------------------------+" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  PeaPod is a protocol that lets nearby devices form an           |" -ForegroundColor Yellow
    Write-Host "|  encrypted local mesh and pool their internet connections        |" -ForegroundColor Yellow
    Write-Host "|  for faster uploads and downloads.                               |" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  This installer will:                                            |" -ForegroundColor Yellow
    Write-Host "|    * Install the Rust toolchain (if not already installed)       |" -ForegroundColor Yellow
    Write-Host "|    * Clone and build the PeaPod Windows app from source          |" -ForegroundColor Yellow
    Write-Host "|    * Install the binary to %LOCALAPPDATA%\PeaPod                |" -ForegroundColor Yellow
    Write-Host "|    * Add PeaPod to your user PATH                               |" -ForegroundColor Yellow
    Write-Host "|    * Optionally configure PeaPod as your system proxy            |" -ForegroundColor Yellow
    Write-Host "|    * Optionally set PeaPod to start on login (system tray)       |" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  PeaPod is open source (MIT License).                            |" -ForegroundColor Yellow
    Write-Host "|  Source: https://github.com/HKTITAN/PeaToPea                    |" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  PeaPod does NOT:                                                |" -ForegroundColor Yellow
    Write-Host "|    * Collect any personal data or telemetry                      |" -ForegroundColor Yellow
    Write-Host "|    * Require an account or sign-up                               |" -ForegroundColor Yellow
    Write-Host "|    * Route traffic through external servers                      |" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  PeaPod DOES:                                                    |" -ForegroundColor Yellow
    Write-Host "|    * Listen on localhost:3128 (HTTP proxy)                       |" -ForegroundColor Yellow
    Write-Host "|    * Listen on UDP port 45678 (LAN discovery)                    |" -ForegroundColor Yellow
    Write-Host "|    * Listen on TCP port 45679 (local transport)                  |" -ForegroundColor Yellow
    Write-Host "|    * Set itself as the system proxy (can be toggled on/off)      |" -ForegroundColor Yellow
    Write-Host "|    * Show a tray icon for status and controls                    |" -ForegroundColor Yellow
    Write-Host "|    * Communicate with other PeaPod devices on your LAN           |" -ForegroundColor Yellow
    Write-Host "|                                                                  |" -ForegroundColor Yellow
    Write-Host "|  All peer-to-peer traffic is encrypted (X25519 + ChaCha20).     |" -ForegroundColor Yellow
    Write-Host "+-----------------------------------------------------------------+" -ForegroundColor Yellow
    Write-Host ""
}

# ── Helpers ─────────────────────────────────────────────────────────

function Confirm-Action {
    param([string]$Prompt)
    if ($env:PEAPOD_NO_CONFIRM -eq "1") { return $true }
    $answer = Read-Host "  $Prompt [y/N]"
    return ($answer -match "^[Yy]")
}

function Test-Command {
    param([string]$Name)
    return [bool](Get-Command $Name -ErrorAction SilentlyContinue)
}

# ── Install steps ───────────────────────────────────────────────────

function Install-Rust {
    if (Test-Command "rustc") {
        $ver = & rustc --version
        Write-Ok "Rust already installed: $ver"
        return
    }
    Write-Info "Rust is not installed."
    if (-not (Confirm-Action "Install the Rust toolchain?")) {
        Write-Err "Rust is required to build PeaPod. Aborting."
        exit 1
    }
    Write-Info "Downloading rustup-init.exe..."
    $rustupUrl = "https://win.rustup.rs/x86_64"
    $rustupPath = "$env:TEMP\rustup-init.exe"
    Invoke-WebRequest -Uri $rustupUrl -OutFile $rustupPath -UseBasicParsing
    Write-Info "Running rustup installer (this may take a minute)..."
    & $rustupPath -y --quiet
    # Refresh PATH
    $env:PATH = [System.Environment]::GetEnvironmentVariable("PATH", "User") + ";" + $env:PATH
    if (Test-Command "rustc") {
        Write-Ok "Rust installed: $(rustc --version)"
    } else {
        Write-Warn "Rust installed but not found in PATH. You may need to restart your terminal."
    }
}

function Install-Git {
    if (Test-Command "git") {
        Write-Ok "Git already installed."
        return
    }
    Write-Err "Git is not installed."
    Write-Info "Please install Git from: https://git-scm.com/download/win"
    Write-Info "Then re-run this installer."
    exit 1
}

function Clone-Repo {
    $script:BuildDir = Join-Path $env:TEMP "peapod-build-$PID"
    Write-Info "Cloning PeaPod repository..."
    & git clone --depth 1 https://github.com/HKTITAN/PeaToPea.git $script:BuildDir
    if (-not (Test-Path $script:BuildDir)) {
        Write-Err "Failed to clone repository. Check your internet connection."
        exit 1
    }
    Write-Ok "Repository cloned to $script:BuildDir"
}

function Build-Binary {
    Write-Info "Building PeaPod (release mode)... this may take a few minutes."
    Push-Location $script:BuildDir
    try {
        & cargo build -p pea-windows --release
        if ($LASTEXITCODE -ne 0) {
            Write-Err "Build failed. See output above for details."
            exit 1
        }
        $script:Binary = Join-Path $script:BuildDir "target\release\pea-windows.exe"
        if (-not (Test-Path $script:Binary)) {
            Write-Err "Build failed - binary not found."
            exit 1
        }
        Write-Ok "Build complete: $script:Binary"
    } finally {
        Pop-Location
    }
}

function Install-Binary {
    $installDir = if ($env:PEAPOD_PREFIX) { $env:PEAPOD_PREFIX } else { Join-Path $env:LOCALAPPDATA "PeaPod" }
    $script:InstallDir = $installDir
    $binPath = Join-Path $installDir "pea-windows.exe"

    if (-not (Test-Path $installDir)) {
        New-Item -ItemType Directory -Path $installDir -Force | Out-Null
    }

    Write-Info "Installing to $binPath"
    Copy-Item $script:Binary $binPath -Force
    Write-Ok "Installed: $binPath"

    # Add to PATH if not already there
    $userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -notlike "*$installDir*") {
        [System.Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
        $env:PATH = "$env:PATH;$installDir"
        Write-Ok "Added $installDir to your PATH."
    }
}

function Setup-Autostart {
    if (-not (Confirm-Action "Start PeaPod automatically on login (system tray)?")) {
        Write-Info "Skipping auto-start setup."
        return
    }

    $startupDir = [System.Environment]::GetFolderPath("Startup")
    $shortcutPath = Join-Path $startupDir "PeaPod.lnk"
    $binPath = Join-Path $script:InstallDir "pea-windows.exe"

    $WshShell = New-Object -ComObject WScript.Shell
    $shortcut = $WshShell.CreateShortcut($shortcutPath)
    $shortcut.TargetPath = $binPath
    $shortcut.Description = "PeaPod - Encrypted mesh for pooled bandwidth"
    $shortcut.Save()

    Write-Ok "Auto-start configured: $shortcutPath"
}

function Cleanup {
    if ($script:BuildDir -and (Test-Path $script:BuildDir)) {
        Remove-Item -Recurse -Force $script:BuildDir -ErrorAction SilentlyContinue
    }
}

# ── Uninstall ───────────────────────────────────────────────────────

function Invoke-Uninstall {
    Show-Banner
    Write-Info "Uninstalling PeaPod..."

    $installDir = if ($env:PEAPOD_PREFIX) { $env:PEAPOD_PREFIX } else { Join-Path $env:LOCALAPPDATA "PeaPod" }
    $binPath = Join-Path $installDir "pea-windows.exe"

    # Stop running instance
    Get-Process -Name "pea-windows" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue
    Write-Ok "Stopped PeaPod process."

    # Remove auto-start shortcut
    $startupDir = [System.Environment]::GetFolderPath("Startup")
    $shortcutPath = Join-Path $startupDir "PeaPod.lnk"
    if (Test-Path $shortcutPath) {
        Remove-Item $shortcutPath -Force
        Write-Ok "Auto-start removed."
    }

    # Restore proxy settings
    try {
        $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
        Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0 -ErrorAction SilentlyContinue
        Write-Ok "System proxy restored."
    } catch {}

    # Remove binary
    if (Test-Path $binPath) {
        Remove-Item $binPath -Force
        Write-Ok "Binary removed: $binPath"
    }

    # Remove from PATH
    $userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
    if ($userPath -like "*$installDir*") {
        $newPath = ($userPath -split ";" | Where-Object { $_ -ne $installDir }) -join ";"
        [System.Environment]::SetEnvironmentVariable("PATH", $newPath, "User")
        Write-Ok "Removed from PATH."
    }

    # Remove install directory if empty
    if ((Test-Path $installDir) -and -not (Get-ChildItem $installDir)) {
        Remove-Item $installDir -Force
    }

    Write-Info "Config at %APPDATA%\peapod was not removed (your settings are preserved)."
    Write-Ok "PeaPod has been uninstalled."
    exit 0
}

# ── Main ────────────────────────────────────────────────────────────

function Main {
    # Handle --uninstall flag
    foreach ($arg in $args) {
        switch ($arg) {
            "--uninstall" { Invoke-Uninstall }
            "--yes"       { $env:PEAPOD_NO_CONFIRM = "1" }
            "-y"          { $env:PEAPOD_NO_CONFIRM = "1" }
            "--help"      {
                Write-Host "Usage: install.ps1 [--yes] [--uninstall] [--help]"
                Write-Host "  --yes         Skip confirmation prompts"
                Write-Host "  --uninstall   Remove PeaPod from this system"
                Write-Host "  --help        Show this help"
                exit 0
            }
        }
    }

    Show-Banner
    Show-Disclaimer

    if (-not (Confirm-Action "Do you want to install PeaPod?")) {
        Write-Info "Installation cancelled."
        exit 0
    }

    Write-Host ""
    Write-Info "Detected: Windows $([System.Environment]::OSVersion.Version)"

    Install-Git
    Install-Rust
    Clone-Repo

    try {
        Build-Binary
        Install-Binary
        Setup-Autostart
    } finally {
        Cleanup
    }

    Write-Host ""
    Write-Host "  +---------------------------------------------------------+" -ForegroundColor Green
    Write-Host "  |              PeaPod installed successfully!              |" -ForegroundColor Green
    Write-Host "  +---------------------------------------------------------+" -ForegroundColor Green
    Write-Host "  |                                                          |" -ForegroundColor Green
    Write-Host "  |  Binary:  $($script:InstallDir)\pea-windows.exe" -ForegroundColor Green
    Write-Host "  |                                                          |" -ForegroundColor Green
    Write-Host "  |  Commands:                                               |" -ForegroundColor Green
    Write-Host "  |    pea-windows              Start (tray icon)            |" -ForegroundColor Green
    Write-Host "  |    pea-windows --version    Show version                 |" -ForegroundColor Green
    Write-Host "  |                                                          |" -ForegroundColor Green
    Write-Host "  |  Uninstall:                                              |" -ForegroundColor Green
    Write-Host "  |    iwr -useb <url>/install.ps1 | iex -- --uninstall     |" -ForegroundColor Green
    Write-Host "  |                                                          |" -ForegroundColor Green
    Write-Host "  +---------------------------------------------------------+" -ForegroundColor Green
    Write-Host ""

    if (Confirm-Action "Start PeaPod now?") {
        $binPath = Join-Path $script:InstallDir "pea-windows.exe"
        Start-Process $binPath
        Write-Ok "PeaPod is running in the system tray."
    }
}

Main @args
