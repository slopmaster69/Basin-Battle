# Shared by play.ps1, host.ps1 and update.ps1 (dot-sourced): where things are, the project's names,
# finding the WoW 1.12.1 install, the saved settings, and asking GitHub for the newest release.
# Nothing here is run by itself.
$ErrorActionPreference = "Stop"
[Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
$ProgressPreference = "SilentlyContinue"

$Launcher = $PSScriptRoot
$Root = Split-Path -Parent $Launcher
$ServerDir = Join-Path $Root "server"
$Exe = Join-Path $Root "benilla.exe"
$ServerExe = Join-Path $ServerDir "htl-server.exe"
$SettingsFile = Join-Path $Root "settings.txt"
$VersionFile = Join-Path $Launcher "version.txt"

function Say($text) { Write-Host $text -ForegroundColor Cyan }
function Warn($text) { Write-Host $text -ForegroundColor Yellow }

# --- the names players see (launcher\project.toml, copied from the repository's project.toml) --
function ProjectName($key, $default) {
    $file = Join-Path $Launcher "project.toml"
    if (Test-Path $file) {
        foreach ($line in Get-Content $file) {
            if ($line -match ('^\s*' + $key + '\s*=\s*"([^"]*)"') -and $Matches[1]) { return $Matches[1] }
        }
    }
    return $default
}
$Title = ProjectName "title" "Basin Battle"
$SurvivalName = ProjectName "survival" "Elwynn Survival"
$BasinName = ProjectName "basin" "Basin Battle"
$Repository = ProjectName "repository" "slopmaster69/Basin-Battle"
$GroupServer = ProjectName "server" ""

# The world port of each game, as the server runs them.
function WorldPort($game) { if ($game -eq "basin") { 8085 } else { 8086 } }
function GameName($game) { if ($game -eq "basin") { $BasinName } else { $SurvivalName } }

# --- settings.txt: one "key = value" a line, kept across updates (the zip carries none) ---------
# An older launcher's settings.txt held the Data folder on line 1 and the address on line 2: read
# as such once, and written back in this form.
function Read-Settings {
    $s = @{}
    if (-not (Test-Path $SettingsFile)) { return $s }
    $lines = @(Get-Content $SettingsFile)
    if ($lines.Count -gt 0 -and $lines[0] -notmatch '^\s*(#|\w+\s*=)') {
        if ($lines[0].Trim()) { $s["wow"] = $lines[0].Trim() }
        if ($lines.Count -gt 1 -and $lines[1].Trim()) { $s["address"] = $lines[1].Trim() }
        return $s
    }
    foreach ($line in $lines) {
        if ($line -match '^\s*(\w+)\s*=\s*(.*?)\s*$') { $s[$Matches[1]] = $Matches[2] }
    }
    return $s
}
function Save-Settings($s) {
    $out = @("# Your choices, kept across updates. Delete this file to be asked again.")
    foreach ($k in ($s.Keys | Sort-Object)) { if ($s[$k]) { $out += "$k = $($s[$k])" } }
    Set-Content -Path $SettingsFile -Value $out
}

# --- the WoW 1.12.1 install -----------------------------------------------------------------------
# A folder counts if it is the Data folder itself or the WoW folder that holds it, with dbc.MPQ in
# it (modern WoW Classic has no MPQs, so it never matches).
function DataFolderOf($dir) {
    if (-not $dir) { return $null }
    $dir = "$dir".Trim().Trim('"')
    try {
        if (Test-Path -LiteralPath (Join-Path $dir "dbc.MPQ")) { return (Resolve-Path -LiteralPath $dir).Path }
        $inner = Join-Path $dir "Data"
        if (Test-Path -LiteralPath (Join-Path $inner "dbc.MPQ")) { return (Resolve-Path -LiteralPath $inner).Path }
    } catch { }
    return $null
}

# Looks where people keep it: the usual folder names on every local drive, Program Files, the
# desktop, Documents, Downloads and Games folders, one level into any folder whose name says WoW.
function Find-WowData {
    $places = New-Object System.Collections.Generic.List[string]
    $drives = @(Get-PSDrive -PSProvider FileSystem -ErrorAction SilentlyContinue |
        Where-Object { $_.Root -match '^[A-Z]:\\$' } | ForEach-Object { $_.Root })
    $homes = @($env:USERPROFILE, [Environment]::GetFolderPath("Desktop"), [Environment]::GetFolderPath("MyDocuments"),
        (Join-Path $env:USERPROFILE "Downloads"), (Join-Path $env:USERPROFILE "Games"),
        $env:ProgramFiles, ${env:ProgramFiles(x86)}) | Where-Object { $_ }
    foreach ($d in $drives) { $places.Add($d); $places.Add((Join-Path $d "Games")) }
    foreach ($h in $homes) { $places.Add($h) }
    foreach ($place in $places) {
        if (-not (Test-Path -LiteralPath $place)) { continue }
        $folders = @(Get-ChildItem -LiteralPath $place -Directory -ErrorAction SilentlyContinue |
            Where-Object { $_.Name -match 'wow|warcraft|1\.12|vanilla|azeroth' })
        foreach ($f in $folders) {
            $found = DataFolderOf $f.FullName
            if ($found) { return $found }
            foreach ($inner in @(Get-ChildItem -LiteralPath $f.FullName -Directory -ErrorAction SilentlyContinue)) {
                $found = DataFolderOf $inner.FullName
                if ($found) { return $found }
            }
        }
    }
    return $null
}

function Pick-WowData {
    Add-Type -AssemblyName System.Windows.Forms
    while ($true) {
        Say "Pick your World of Warcraft 1.12.1 folder (the one with WoW.exe and a Data folder in it)."
        $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
        $dialog.Description = "Pick your World of Warcraft 1.12.1 folder (the one that contains the Data folder)"
        $dialog.ShowNewFolderButton = $false
        if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { return $null }
        $data = DataFolderOf $dialog.SelectedPath
        if ($data) { return $data }
        [System.Windows.Forms.MessageBox]::Show(
            "That folder has no Data\dbc.MPQ in it, so it isn't a WoW 1.12.1 install. Pick the folder that has WoW.exe in it. (Modern WoW Classic won't work: it has to be 1.12.1.)",
            $Title) | Out-Null
    }
}

# The saved folder, else one found on this PC, else the picker. Saved for next time.
function Get-WowData($settings, [switch]$Pick) {
    $data = if ($Pick) { $null } else { DataFolderOf $settings["wow"] }
    if (-not $data -and -not $Pick) {
        Say "Looking for your World of Warcraft 1.12.1 install..."
        $data = Find-WowData
        if ($data) { Say "Found it: $data" }
    }
    if (-not $data) { $data = Pick-WowData }
    if ($data) { $settings["wow"] = $data; Save-Settings $settings }
    return $data
}

# --- this PC's own server ---------------------------------------------------------------------------
function PortOpen($address, $port, $ms = 500) {
    $c = New-Object Net.Sockets.TcpClient
    try { return ($c.ConnectAsync($address, $port).Wait($ms) -and $c.Connected) } catch { return $false } finally { $c.Close() }
}

# --- Tailscale (how the group plays together for now) ------------------------------------------------
$TailscaleExe = Join-Path $env:ProgramFiles "Tailscale\tailscale.exe"
# An address on a Tailscale network: 100.64.0.0/10, or a MagicDNS name (*.ts.net).
function Is-TailscaleAddress($address) {
    if ($address -match '\.ts\.net$') { return $true }
    if ($address -match '^100\.(\d+)\.\d+\.\d+$') { $b = [int]$Matches[1]; return ($b -ge 64 -and $b -le 127) }
    return $false
}
# Why a server can't be reached, in words a player can act on.
function Why-Unreachable($address) {
    if (Is-TailscaleAddress $address) {
        if (-not (Test-Path $TailscaleExe)) {
            return "This server is on the group's Tailscale network, and Tailscale isn't on this PC yet. Install it from the page that just opened, sign in with the account the host invited, then run Play again."
        }
        # (Its complaints go to stderr, which under "Stop" would end the script: not here.)
        $ErrorActionPreference = "Continue"
        $up = $false
        try { $null = & $TailscaleExe status 2>&1; $up = ($LASTEXITCODE -eq 0) } catch { }
        if (-not $up) {
            return "Tailscale is off or signed out. Open it (its icon is by the clock), log in and connect, then try again."
        }
        return "Tailscale is on, but the server at $address doesn't answer. Is the host's server running (their Host window open)? If you were only just invited, the host may need to share their PC with you in Tailscale."
    }
    return "The server at $address doesn't answer. Is the host's server running (their Host window open), and is the address right?"
}
function Get-OurServer {
    Get-Process htl-server -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($ServerDir + "\", [StringComparison]::OrdinalIgnoreCase) }
}
function Get-OurGame {
    Get-Process benilla -ErrorAction SilentlyContinue |
        Where-Object { $_.Path -and $_.Path.StartsWith($Root + "\", [StringComparison]::OrdinalIgnoreCase) }
}

# --- versions -----------------------------------------------------------------------------------
function Get-InstalledVersion {
    if (Test-Path $VersionFile) { return (Get-Content $VersionFile -TotalCount 1).Trim() }
    return ""
}

# The newest release on GitHub, or $null when it can't be asked (offline, say).
function Get-LatestRelease {
    $headers = @{ "User-Agent" = "$Title launcher"; "Accept" = "application/vnd.github+json" }
    try {
        return Invoke-RestMethod -Uri "https://api.github.com/repos/$Repository/releases/latest" -Headers $headers -TimeoutSec 8
    } catch { return $null }
}

# Whether $tag is a newer release than this folder's. A folder without a version (built by hand, or
# a branch's test build) is never updated over.
function Is-Newer($tag) {
    $mine = Get-InstalledVersion
    if (-not $mine -or $mine -notmatch '^v\d') { return $false }
    if (-not $tag -or $tag -eq $mine) { return $false }
    try {
        $a = [version]($tag.TrimStart('v') -replace '[^\d.].*$', '')
        $b = [version]($mine.TrimStart('v') -replace '[^\d.].*$', '')
        return $a -gt $b
    } catch { return $tag -ne $mine }
}
