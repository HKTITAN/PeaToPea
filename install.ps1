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

function Test-MSVCAvailable {
    # Check for cl.exe or link.exe from Visual Studio Build Tools
    if (Test-Command "cl") { return $true }
    if (Test-Command "link") {
        # Distinguish MSVC link.exe from Cygwin/other link.exe
        $linkOut = & link 2>&1 | Out-String
        if ($linkOut -match "Microsoft") { return $true }
    }
    # Check via vswhere (Visual Studio locator)
    $vswhere = "${env:ProgramFiles(x86)}\Microsoft Visual Studio\Installer\vswhere.exe"
    if (Test-Path $vswhere) {
        $vsPath = & $vswhere -latest -products * -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath 2>$null
        if ($vsPath) { return $true }
    }
    return $false
}

function Install-Rust {
    $hasMSVC = Test-MSVCAvailable

    if (Test-Command "rustc") {
        $ver = & rustc --version
        Write-Ok "Rust already installed: $ver"

        # If Rust is installed with MSVC target but no MSVC build tools, add the GNU target
        if (-not $hasMSVC) {
            $defaultHost = & rustc -vV 2>$null | Select-String "host:" | ForEach-Object { $_.Line -replace "host:\s*", "" }
            if ($defaultHost -and $defaultHost -match "msvc") {
                Write-Warn "Rust is using the MSVC toolchain but Visual Studio Build Tools are not installed."
                Write-Info "Adding the GNU target (x86_64-pc-windows-gnu) so PeaPod can build without Visual Studio..."
                & rustup target add x86_64-pc-windows-gnu
                & rustup toolchain install stable-x86_64-pc-windows-gnu
                $script:RustTarget = "x86_64-pc-windows-gnu"
                Write-Ok "GNU toolchain added. Builds will use x86_64-pc-windows-gnu."
            }
        }
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

    if ($hasMSVC) {
        Write-Info "Visual Studio Build Tools detected. Installing Rust with MSVC toolchain..."
        & $rustupPath -y --quiet
    } else {
        Write-Info "Visual Studio Build Tools not found. Installing Rust with GNU toolchain (no Visual Studio required)..."
        & $rustupPath -y --quiet --default-host x86_64-pc-windows-gnu
        $script:RustTarget = "x86_64-pc-windows-gnu"
    }

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
        if ($script:RustTarget) {
            Write-Info "Building for target: $($script:RustTarget)"
            & cargo build -p pea-windows --release --target $script:RustTarget
            if ($LASTEXITCODE -ne 0) {
                Write-Err "Build failed. See output above for details."
                exit 1
            }
            $script:Binary = Join-Path $script:BuildDir "target\$($script:RustTarget)\release\pea-windows.exe"
        } else {
            & cargo build -p pea-windows --release
            if ($LASTEXITCODE -ne 0) {
                Write-Err "Build failed. See output above for details."
                exit 1
            }
            $script:Binary = Join-Path $script:BuildDir "target\release\pea-windows.exe"
        }
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
    if (-not (Confirm-Action "Uninstall PeaPod?")) {
        Write-Info "Uninstall cancelled."
        exit 0
    }
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
    } else {
        Write-Warn "Binary not found at $binPath (already removed?)."
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

# ── Update ──────────────────────────────────────────────────────────

function Invoke-Update {
    Show-Banner
    Write-Info "Updating PeaPod to the latest version..."

    $installDir = if ($env:PEAPOD_PREFIX) { $env:PEAPOD_PREFIX } else { Join-Path $env:LOCALAPPDATA "PeaPod" }
    $binPath = Join-Path $installDir "pea-windows.exe"

    if (-not (Test-Path $binPath)) {
        Write-Err "PeaPod is not installed at $binPath. Run the installer first."
        exit 1
    }

    # Show current version
    $currentVer = try { & $binPath --version 2>$null } catch { "unknown" }
    Write-Info "Current version: $currentVer"

    if (-not (Confirm-Action "Download and build the latest version?")) {
        Write-Info "Update cancelled."
        exit 0
    }

    Install-Rust
    Install-Git
    Clone-Repo

    try {
        Build-Binary

        # Stop running instance before replacing
        Get-Process -Name "pea-windows" -ErrorAction SilentlyContinue | Stop-Process -Force -ErrorAction SilentlyContinue

        # Replace binary
        Copy-Item $script:Binary $binPath -Force
        Write-Ok "Binary updated: $binPath"
    } finally {
        Cleanup
    }

    $newVer = try { & $binPath --version 2>$null } catch { "unknown" }
    Write-Host ""
    Write-Ok "PeaPod updated: $currentVer -> $newVer"

    if (Confirm-Action "Start PeaPod now?") {
        Start-Process $binPath
        Write-Ok "PeaPod is running in the system tray."
    }
    exit 0
}

# ── Modify ──────────────────────────────────────────────────────────

function Invoke-Modify {
    Show-Banner
    Write-Info "Modify PeaPod installation"

    $installDir = if ($env:PEAPOD_PREFIX) { $env:PEAPOD_PREFIX } else { Join-Path $env:LOCALAPPDATA "PeaPod" }
    $binPath = Join-Path $installDir "pea-windows.exe"

    if (-not (Test-Path $binPath)) {
        Write-Err "PeaPod is not installed at $binPath. Run the installer first."
        exit 1
    }

    Write-Host ""
    Write-Host "  What would you like to change?" -ForegroundColor White
    Write-Host ""
    Write-Host "  1) Toggle auto-start on login"
    Write-Host "  2) Toggle system proxy"
    Write-Host "  3) Repair installation (re-add to PATH)"
    Write-Host "  4) Cancel"
    Write-Host ""
    $choice = Read-Host "  Choice [1-4]"

    switch ($choice) {
        "1" {
            $startupDir = [System.Environment]::GetFolderPath("Startup")
            $shortcutPath = Join-Path $startupDir "PeaPod.lnk"

            if (Test-Path $shortcutPath) {
                Write-Info "Auto-start is currently ENABLED."
                if (Confirm-Action "Disable auto-start?") {
                    Remove-Item $shortcutPath -Force
                    Write-Ok "Auto-start disabled. PeaPod will not start on login."
                }
            } else {
                Write-Info "Auto-start is currently DISABLED."
                if (Confirm-Action "Enable auto-start?") {
                    $script:InstallDir = $installDir
                    Setup-Autostart
                }
            }
        }
        "2" {
            $regPath = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Internet Settings"
            $proxyEnabled = (Get-ItemProperty -Path $regPath -Name ProxyEnable -ErrorAction SilentlyContinue).ProxyEnable

            if ($proxyEnabled -eq 1) {
                Write-Info "System proxy is currently ENABLED."
                if (Confirm-Action "Disable system proxy?") {
                    Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 0
                    Write-Ok "System proxy disabled."
                }
            } else {
                Write-Info "System proxy is currently DISABLED."
                if (Confirm-Action "Enable system proxy (127.0.0.1:3128)?") {
                    Set-ItemProperty -Path $regPath -Name ProxyEnable -Value 1
                    Set-ItemProperty -Path $regPath -Name ProxyServer -Value "127.0.0.1:3128"
                    Write-Ok "System proxy enabled (127.0.0.1:3128)."
                }
            }
        }
        "3" {
            $userPath = [System.Environment]::GetEnvironmentVariable("PATH", "User")
            if ($userPath -notlike "*$installDir*") {
                [System.Environment]::SetEnvironmentVariable("PATH", "$userPath;$installDir", "User")
                $env:PATH = "$env:PATH;$installDir"
                Write-Ok "Added $installDir to PATH."
            } else {
                Write-Ok "$installDir is already in PATH."
            }

            if (-not (Test-Path $binPath)) {
                Write-Warn "Binary missing at $binPath. Run the installer to rebuild."
            } else {
                Write-Ok "Installation looks good: $binPath"
            }
        }
        default {
            Write-Info "No changes made."
        }
    }
    exit 0
}

# ── Main ────────────────────────────────────────────────────────────

$script:LocalBuild = $false
$script:RustTarget = $null

function Main {
    # Handle flags
    foreach ($arg in $args) {
        switch ($arg) {
            "--uninstall" { Invoke-Uninstall }
            "--update"    { Invoke-Update }
            "--modify"    { Invoke-Modify }
            "--yes"       { $env:PEAPOD_NO_CONFIRM = "1" }
            "-y"          { $env:PEAPOD_NO_CONFIRM = "1" }
            "--local"     { $script:LocalBuild = $true }
            "--help"      {
                Write-Host "Usage: install.ps1 [--yes] [--local] [--uninstall] [--update] [--modify] [--help]"
                Write-Host "  --yes         Skip confirmation prompts"
                Write-Host "  --local       Build from the current directory (skip git clone)"
                Write-Host "  --uninstall   Remove PeaPod from this system"
                Write-Host "  --update      Update PeaPod to the latest version"
                Write-Host "  --modify      Change PeaPod settings (auto-start, proxy, PATH)"
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

    Install-Rust

    if ($script:LocalBuild) {
        if (-not (Test-Path "Cargo.toml")) {
            Write-Err "Not in the PeaPod repo root (Cargo.toml not found). Run from the repo root or remove --local."
            exit 1
        }
        $script:BuildDir = (Get-Location).Path
        Write-Ok "Building from local checkout: $script:BuildDir"
    } else {
        Install-Git
        Clone-Repo
    }

    try {
        Build-Binary
        Install-Binary
        Setup-Autostart
    } finally {
        if (-not $script:LocalBuild) {
            Cleanup
        }
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
    Write-Host "  |  Manage:                                                 |" -ForegroundColor Green
    Write-Host "  |    install.ps1 --update     Update to latest version     |" -ForegroundColor Green
    Write-Host "  |    install.ps1 --modify     Change settings              |" -ForegroundColor Green
    Write-Host "  |    install.ps1 --uninstall  Remove PeaPod                |" -ForegroundColor Green
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
