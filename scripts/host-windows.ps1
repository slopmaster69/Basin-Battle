# Host the games on a Windows PC: Basin Battle and Elwynn Survival, each on its own realm.
# "Host Basin Battle.bat" in the top folder runs this; you can also run it from PowerShell:
#   scripts\host-windows.ps1                          password "hold"
#   scripts\host-windows.ps1 --game basin             only Basin Battle (or --game survival)
#   scripts\host-windows.ps1 --password secret        pick the password everyone uses
#   scripts\host-windows.ps1 --public-host 1.2.3.4    your internet address (found automatically otherwise)
# The first run installs Rust if it's missing (a few minutes), asks for your WoW 1.12.1 folder
# once (remembered in settings-windows.txt) and builds the server (5-10 minutes); later runs
# rebuild only what changed. Every argument goes to the server as given.
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path)
Set-Location $Root
$Host.UI.RawUI.WindowTitle = "Basin Battle server (close this window to stop it)"
$Settings = Join-Path $Root "settings-windows.txt"

function Say($text) { Write-Host $text -ForegroundColor Cyan }

# --- Rust ----------------------------------------------------------------------------------------
# The GNU toolchain links with its own bundled linker, so no Visual Studio is needed.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Say "Installing Rust (once; a few minutes)..."
    $installer = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/x86_64" -OutFile $installer
    & $installer -y --profile minimal --default-host x86_64-pc-windows-gnu --default-toolchain stable
    if ($LASTEXITCODE -ne 0) { throw "The Rust installer failed (exit $LASTEXITCODE)." }
    $env:PATH = "$cargoBin;$env:PATH"
}

# --- the WoW 1.12.1 Data folder ------------------------------------------------------------------
# A folder counts if it is the Data folder itself or the WoW folder that holds it.
function DataFolderOf($dir) {
    if (-not $dir) { return $null }
    $dir = $dir.Trim().Trim('"')
    if (Test-Path (Join-Path $dir "dbc.MPQ")) { return (Resolve-Path $dir).Path }
    $inner = Join-Path $dir "Data"
    if (Test-Path (Join-Path $inner "dbc.MPQ")) { return (Resolve-Path $inner).Path }
    return $null
}

$data = $null
if ($env:WOW_DATA) { $data = DataFolderOf $env:WOW_DATA }
if (-not $data -and (Test-Path $Settings)) { $data = DataFolderOf (Get-Content $Settings -TotalCount 1) }
if (-not $data) {
    Add-Type -AssemblyName System.Windows.Forms
    while (-not $data) {
        Say "Pick your World of Warcraft 1.12.1 folder (the one with WoW.exe and a Data folder in it)."
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
        $dialog.Description = "Pick your World of Warcraft 1.12.1 folder (the one that contains the Data folder)"
        $dialog.ShowNewFolderButton = $false
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
            Say "No folder picked. Run Host Basin Battle again when you're ready."
            exit 1
        }
        $data = DataFolderOf $dialog.SelectedPath
        if (-not $data) {
            [System.Windows.Forms.MessageBox]::Show(
                "That folder has no Data\dbc.MPQ in it, so it isn't a WoW 1.12.1 install. Pick the folder that has WoW.exe in it.",
                "Basin Battle") | Out-Null
        }
    }
    Set-Content -Path $Settings -Value $data
}
Say "Game data: $data"

# --- build ---------------------------------------------------------------------------------------
Say "Building the server (only what changed since last time)..."
& cargo build --release -q -p htl-server
if ($LASTEXITCODE -ne 0) { throw "The server didn't build (exit $LASTEXITCODE): see the errors above." }
$exe = Join-Path $Root "target\release\htl-server.exe"

# --- addresses -----------------------------------------------------------------------------------
$serverArgs = @("--data", $data, "--content", "content/game.toml", "--saves", "saves/basin-battle.toml",
    "--survival-content", "content/survival.toml", "--survival-save", "saves/elwynn-survival.toml")
if (-not ($args -contains "--public-host")) {
    $public = $env:HTL_PUBLIC_HOST
    if (-not $public) {
        try { $public = (Invoke-RestMethod -Uri "https://api.ipify.org" -TimeoutSec 4).ToString().Trim() } catch { $public = $null }
    }
    if ($public -match '^[0-9a-fA-F.:]+$') {
        $serverArgs += @("--public-host", $public)
        Say "Friends on the internet join with: $public   (your router must forward TCP ports 3724, 8085 and 8086 to this PC)"
    }
}
try {
    $lan = Get-NetIPAddress -AddressFamily IPv4 -ErrorAction Stop |
        Where-Object { $_.IPAddress -notlike "127.*" -and $_.IPAddress -notlike "169.254.*" -and $_.InterfaceAlias -notlike "*Loopback*" } |
        Select-Object -First 1 -ExpandProperty IPAddress
    if ($lan) { Say "Friends on your own network join with: $lan" }
} catch { }
Say "If Windows asks whether htl-server may use the network, allow it (Private networks at least)."

# --- run -----------------------------------------------------------------------------------------
New-Item -ItemType Directory -Force -Path (Join-Path $Root "saves"), (Join-Path $Root "logs") | Out-Null
$log = Join-Path $Root ("logs\host-" + (Get-Date -Format "yyyy-MM-dd-HHmmss") + ".log")
Say "This session's log: $log"
# Through cmd so the server's own error stream reaches the log as plain lines.
$quoted = ($serverArgs + $args | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join " "
& cmd /c "`"$exe`" $quoted 2>&1" | Tee-Object -FilePath $log
