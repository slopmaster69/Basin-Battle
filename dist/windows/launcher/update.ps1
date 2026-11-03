# Brings this whole folder (the game, the server, the add-ons and these scripts) up to the newest
# release, so everything in it always comes from the same version. Play and Host run it for you;
# you never need to.
#   update.ps1            ask GitHub, update if there's something newer
#   update.ps1 -Quiet     the same without asking anything (Host's restarts use this)
# Kept: settings.txt, the game's own settings (benilla-config\config.toml, your key and add-on
# settings), and the server's worlds and logs (server\saves, backed up first, and server\logs).
# Exit code: 0 updated, 2 already up to date or can't tell, 3 not now (the game or the server is
# running), 1 failed (nothing changed).
param([switch]$Quiet)
. (Join-Path $PSScriptRoot "common.ps1")

$release = Get-LatestRelease
if (-not $release) { if (-not $Quiet) { Warn "Couldn't ask GitHub for a new version (offline?). Carrying on." }; exit 2 }
$tag = $release.tag_name
if (-not (Is-Newer $tag)) { exit 2 }
$asset = $release.assets | Where-Object { $_.name -like "*for Windows*.zip" } | Select-Object -First 1
if (-not $asset) { $asset = $release.assets | Where-Object { $_.name -like "*.zip" } | Select-Object -First 1 }
if (-not $asset) {
    # A release's download is built after it's published (about 40 minutes): try again next time.
    if (-not $Quiet) { Say "$tag is out, but its download isn't ready yet. Carrying on with $(Get-InstalledVersion)." }
    exit 2
}

Say "A new version is out: $tag (you have $(Get-InstalledVersion)). Updating..."
if (Get-OurServer) {
    Warn "Your server is running from this folder, so it can't be updated now. Host updates it at its next restart."
    exit 3
}
if (Get-OurGame) {
    if ($Quiet) {
        # A host who plays on this PC too: the restart has just disconnected the game.
        Warn "Your game is open on this PC. Close it to finish the update (waiting up to 10 minutes)..."
        $deadline = (Get-Date).AddMinutes(10)
        while ((Get-OurGame) -and (Get-Date) -lt $deadline) { Start-Sleep -Seconds 3 }
    } else {
        Warn "The game is open. Close it (its window is called benilla), then press Enter."
        Read-Host | Out-Null
    }
    if (Get-OurGame) { Warn "Still open: not updating now."; exit 3 }
}

$script:copying = $false
$tmp = Join-Path $env:TEMP ("update-" + [guid]::NewGuid().ToString("N").Substring(0, 8))
try {
    New-Item -ItemType Directory -Force -Path $tmp | Out-Null
    $zip = Join-Path $tmp "download.zip"
    Say "Downloading $($asset.name) ($([math]::Round($asset.size / 1MB)) MB)..."
    Invoke-WebRequest -UseBasicParsing -Uri $asset.browser_download_url -OutFile $zip -Headers @{ "User-Agent" = "$Title launcher" }
    if ((Get-Item $zip).Length -ne $asset.size) { throw "the download is the wrong size" }
    Expand-Archive -Path $zip -DestinationPath (Join-Path $tmp "new") -Force
    $exe = Get-ChildItem (Join-Path $tmp "new") -Recurse -Filter "benilla.exe" | Select-Object -First 1
    if (-not $exe) { throw "the download has no benilla.exe in it" }
    $new = $exe.DirectoryName
    if (-not (Test-Path (Join-Path $new "launcher\common.ps1"))) { throw "the download isn't laid out as this folder is" }

    # Your worlds first, in case the new server has a problem with them.
    $saves = Join-Path $ServerDir "saves"
    if (Test-Path $saves) {
        $backups = Join-Path $ServerDir "saves-backups"
        $backup = Join-Path $backups ("before-" + $tag + "-" + (Get-Date -Format "yyyy-MM-dd-HHmm"))
        New-Item -ItemType Directory -Force -Path $backup | Out-Null
        Copy-Item (Join-Path $saves "*") $backup -Recurse -Force
        # The last 10 kept.
        Get-ChildItem $backups -Directory | Sort-Object LastWriteTime -Descending | Select-Object -Skip 10 |
            Remove-Item -Recurse -Force -ErrorAction SilentlyContinue
    }

    # The new version number is written last: an update cut short is tried again next time.
    # Until then the folder keeps saying the old one (the mirror below would otherwise leave none).
    $pending = Join-Path $tmp "version.txt"
    Move-Item (Join-Path $new "launcher\version.txt") $pending
    Set-Content -Path (Join-Path $new "launcher\version.txt") -Value (Get-InstalledVersion)
    $script:copying = $true
    function Copy-Tree($from, $to, [switch]$Mirror) {
        $how = if ($Mirror) { "/MIR" } else { "/E" }
        & robocopy $from $to $how /R:2 /W:1 /NFL /NDL /NJH /NJS /NP | Out-Null
        if ($LASTEXITCODE -ge 8) { throw "copying into $to failed (robocopy $LASTEXITCODE)" }
    }
    # Everything over the old, which keeps your files (the download has none of them)...
    Copy-Tree $new $Root
    # ...then what the release owns outright made exactly the new one, so nothing stale is left
    # in it: the two add-ons, the server's content and these scripts.
    foreach ($owned in @("benilla-config\AddOns\BasinBattle", "benilla-config\AddOns\ElwynnSurvival", "server\content", "launcher")) {
        if (Test-Path (Join-Path $new $owned)) { Copy-Tree (Join-Path $new $owned) (Join-Path $Root $owned) -Mirror }
    }
    # Files an older download had that this one doesn't.
    $retired = @("launcher.ps1", "Play Basin Battle.bat", "Play Elwynn Survival.bat", "LICENSE-MIT", "LICENSE-APACHE",
        "Start on this PC.bat", "Start Elwynn Survival on this PC.bat", "start-on-this-pc.ps1", "start-elwynn-on-this-pc.ps1")
    foreach ($f in $retired) { Remove-Item (Join-Path $Root $f) -Force -ErrorAction SilentlyContinue }
    # The game's memory of item and creature details: asked afresh of the new server.
    Remove-Item (Join-Path $Root "benilla-config\cache") -Recurse -Force -ErrorAction SilentlyContinue
    Copy-Item $pending $VersionFile -Force

    Say "Updated to $tag."
    exit 0
} catch {
    if ($script:copying) {
        Warn "The update stopped part-way ($($_.Exception.Message)). Close everything and run Play again: it finishes the update."
    } else {
        Warn "The update didn't work ($($_.Exception.Message)). Nothing was changed; carrying on with $(Get-InstalledVersion)."
    }
    exit 1
} finally {
    Remove-Item $tmp -Recurse -Force -ErrorAction SilentlyContinue
}
