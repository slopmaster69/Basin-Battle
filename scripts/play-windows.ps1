# Basin Battle for Windows players: sets up the benilla client and connects to a host's game.
# Run in PowerShell from the Basin Battle folder:
#   powershell -ExecutionPolicy Bypass -File scripts\play-windows.ps1 -Server 1.2.3.4
# The first run installs what's missing and builds the client (10-30 minutes).
param(
    [Parameter(Mandatory = $true)][string]$Server,
    [string]$WowData = "",
    [string]$Game = "Basin Battle"
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$Rev = "c23d0163d5efa1ae39e4748877fe4ba831202848"
$SettingsFile = Join-Path $Root "settings-windows.txt"

function Need($cmd, $wingetId, $what) {
    if (-not (Get-Command $cmd -ErrorAction SilentlyContinue)) {
        Write-Host "Installing $what..."
        winget install --id $wingetId -e --accept-source-agreements --accept-package-agreements
        $env:Path = [System.Environment]::GetEnvironmentVariable("Path", "Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path", "User")
    }
}
Need git "Git.Git" "Git"
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Installing the C++ build tools Rust needs (this can take a while)..."
    winget install --id Microsoft.VisualStudio.2022.BuildTools -e --accept-source-agreements --accept-package-agreements --override "--quiet --wait --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
    Need rustup "Rustlang.Rustup" "Rust"
    $env:Path += ";$env:USERPROFILE\.cargo\bin"
}

if (-not $WowData -and (Test-Path $SettingsFile)) { $WowData = Get-Content $SettingsFile -First 1 }
while (-not $WowData -or -not (Test-Path (Join-Path $WowData "dbc.MPQ"))) {
    $WowData = (Read-Host "Path to the Data folder of your WoW 1.12.1 install").Trim('"')
}
Set-Content $SettingsFile $WowData

$Benilla = Join-Path $Root "benilla"
if (-not (Test-Path (Join-Path $Benilla ".git"))) { git clone https://github.com/samwhosung/benilla $Benilla }
git -C $Benilla fetch -q origin
# The pinned client with this folder's own changes on top (Elwynn Survival's mouse and aiming
# camera), rebuilt when they change.
$Patches = Get-ChildItem (Join-Path $Root "patches\benilla") -Filter *.patch -ErrorAction SilentlyContinue | Sort-Object Name
$Stamp = ($Patches | ForEach-Object { (Get-FileHash $_.FullName -Algorithm SHA1).Hash }) -join ","
$StampFile = Join-Path $Benilla ".htl-patches"
$Exe = Join-Path $Benilla "target\release\benilla.exe"
$Built = if (Test-Path $StampFile) { Get-Content $StampFile -First 1 } else { "" }
if (-not (Test-Path $Exe) -or $Built -ne $Stamp) {
    git -C $Benilla checkout -q -- crates
    git -C $Benilla clean -q -f -- crates
    git -C $Benilla checkout -q $Rev
    foreach ($p in $Patches) { git -C $Benilla apply --whitespace=nowarn $p.FullName }
    Push-Location $Benilla
    cargo build --release -p benilla
    Pop-Location
    Set-Content $StampFile $Stamp
}
$env:WOW_DATA = $WowData
$env:WOW_HOST = $Server
$env:WOW_REALM = $Game
Push-Location $Benilla
& $Exe
Pop-Location
