# What Play.bat runs: updates this folder if there's a new version, finds your WoW 1.12.1, then
# starts the game where you played last time (press C in the first 3 seconds to choose again).
#   play.ps1 -Change      straight to the choices
#   play.ps1 -NoUpdate    don't look for a new version this time
param([switch]$Change, [switch]$NoUpdate, [string]$Game = "")
. (Join-Path $PSScriptRoot "common.ps1")
$Host.UI.RawUI.WindowTitle = "$Title (this window closes once the game starts)"
function Stop-Here($text) { Warn $text; Read-Host "Press Enter to close"; exit 1 }

if (-not (Test-Path $Exe)) { Stop-Here "benilla.exe is missing from $Root. Your anti-virus may have taken it: restore it, or download the game again." }

# --- 1. the newest version -------------------------------------------------------------------------
if (-not $NoUpdate -and (Get-InstalledVersion)) {
    Say "$Title $(Get-InstalledVersion). Checking for a new version..."
    & (Join-Path $Launcher "update.ps1")
    if ($LASTEXITCODE -eq 0) {
        # The new version's own Play takes it from here.
        $again = @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Join-Path $Launcher "play.ps1"), "-NoUpdate")
        if ($Change) { $again += "-Change" }
        if ($Game) { $again += @("-Game", $Game) }
        & powershell.exe @again
        exit $LASTEXITCODE
    }
}

if (Get-OurGame) { Stop-Here "The game is already open (its window is called benilla). Only one at a time: close it first." }

# --- 2. your WoW 1.12.1 ----------------------------------------------------------------------------
$settings = Read-Settings
$data = Get-WowData $settings
if (-not $data) { Stop-Here "No WoW 1.12.1 folder picked. Run Play again when you have one." }

# --- 3. which game, and where ----------------------------------------------------------------------
if ($Game -match 'basin') { $settings["game"] = "basin" } elseif ($Game) { $settings["game"] = "survival" }
if (-not $settings["game"]) { $settings["game"] = "survival" }
if ($settings["place"] -eq "group" -and -not $GroupServer) { $settings["place"] = "" }

function Where-Words($s) {
    switch ($s["place"]) {
        "group" { "the group's server ($GroupServer)" }
        "own" { "your own server on this PC" }
        "address" { "$($s["address"])" }
        default { "" }
    }
}

function Choose($s) {
    Write-Host ""
    Say "Which game?"
    Say "  1  $SurvivalName"
    Say "  2  $BasinName"
    $now = if ($s["game"] -eq "basin") { "2" } else { "1" }
    $pick = (Read-Host "Type 1 or 2 (Enter keeps $now)").Trim()
    if ($pick -eq "2") { $s["game"] = "basin" } elseif ($pick -eq "1") { $s["game"] = "survival" }

    Write-Host ""
    Say "Where?"
    $options = @()
    if ($GroupServer) { $options += @{ key = "group"; text = "The group's server ($GroupServer)" } }
    $options += @{ key = "own"; text = "My own server on this PC (alone, or friends join me)" }
    $theirs = "Someone else's server (type their address)"
    if ($s["address"]) { $theirs = "Someone else's server ($($s["address"]), or type another)" }
    $options += @{ key = "address"; text = $theirs }
    for ($i = 0; $i -lt $options.Count; $i++) { Say "  $($i + 1)  $($options[$i].text)" }
    Say "  W  Pick a different WoW 1.12.1 folder (now: $data)"
    $now = 1
    for ($i = 0; $i -lt $options.Count; $i++) { if ($options[$i].key -eq $s["place"]) { $now = $i + 1 } }
    while ($true) {
        $pick = (Read-Host "Type a number (Enter keeps $now)").Trim()
        if ($pick -eq "w" -or $pick -eq "W") {
            $picked = Get-WowData $s -Pick
            if ($picked) { $script:data = $picked; Say "Using $picked" }
            continue
        }
        if (-not $pick) { $pick = "$now" }
        $n = 0
        if ([int]::TryParse($pick, [ref]$n) -and $n -ge 1 -and $n -le $options.Count) { break }
    }
    $s["place"] = $options[$n - 1].key
    if ($s["place"] -eq "address") {
        $prompt = "Their address (they see it in their Host window)"
        if ($s["address"]) { $prompt += "; Enter keeps $($s["address"])" }
        $typed = (Read-Host $prompt).Trim()
        if ($typed) { $s["address"] = $typed }
        while (-not $s["address"]) { $s["address"] = (Read-Host "Their address").Trim() }
    }
}

$where = Where-Words $settings
if ($Change -or -not $where) {
    Choose $settings
} else {
    # Last time's choice, unless C is pressed in the next 3 seconds.
    Write-Host ""
    Say "Playing $(GameName $settings["game"]) on $where."
    Say "Press C to change that (game, server or WoW folder)..."
    $until = (Get-Date).AddSeconds(3)
    while ((Get-Date) -lt $until) {
        try { $key = if ([Console]::KeyAvailable) { [Console]::ReadKey($true) } else { $null } } catch { $key = $null }
        if ($key -and $key.Key -eq "C") { Choose $settings; break }
        Start-Sleep -Milliseconds 100
    }
}
Save-Settings $settings
$gameName = GameName $settings["game"]
$port = WorldPort $settings["game"]

# --- 4. the server ---------------------------------------------------------------------------------
switch ($settings["place"]) {
    "group" { $address = $GroupServer }
    "address" { $address = $settings["address"] }
    default {
        $address = "127.0.0.1"
        if (-not ((PortOpen $address 3724) -and (PortOpen $address $port))) {
            if (-not (Test-Path $ServerExe)) { Stop-Here "This download has no server in it (server\htl-server.exe). Download the newest version, or play on someone else's server." }
            Say "Your server isn't running, so it's starting in its own window. Leave that window open while you play."
            Start-Process -FilePath (Join-Path $Root "Host.bat") -WorkingDirectory $Root
            $waited = 0
            while (-not ((PortOpen $address 3724) -and (PortOpen $address $port))) {
                Start-Sleep -Seconds 2
                $waited += 2
                if ($waited % 20 -eq 0) { Say "Still waiting for the server to load the world ($waited s)..." }
                if ($waited -ge 600) { Stop-Here "The server didn't start in 10 minutes. Look at its window for what went wrong." }
            }
            Say "Your server is up."
        }
    }
}

# --- 5. play -----------------------------------------------------------------------------------------
Say "Joining $gameName at $address. Log in with any name you like and the password (hold, unless the host changed it)."
$env:WOW_DATA = $data
$env:WOW_HOST = $address
$env:WOW_REALM = $gameName
# The game's memory of item and creature details: cleared so it always asks the server afresh.
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $Root "benilla-config\cache")
# The bundled settings only for a first run: an update keeps your mouse speed and the rest.
$config = Join-Path $Root "benilla-config\config.toml"
$defaults = Join-Path $Root "benilla-config\config.default.toml"
if (-not (Test-Path $config) -and (Test-Path $defaults)) { Copy-Item $defaults $config }
Start-Process -FilePath $Exe -WorkingDirectory $Root
Say "The game is opening in its own window. This one closes now."
Start-Sleep -Seconds 2
