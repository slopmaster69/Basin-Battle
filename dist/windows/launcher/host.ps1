# What Host.bat runs: this PC's server for both games, the one that came in this download (nothing
# to build). Leave its window open while anyone plays; close it to stop the server.
#   Host.bat --password secret       the password everyone logs in with (default: hold)
#   Host.bat --game survival         only one of the games (or --game basin)
# Any option goes to the server as given (Host.bat --help lists them).
# Every 15 minutes it asks GitHub for a new version. When there is one, everyone playing is
# warned 5 minutes ahead, the worlds are saved, and the server comes back on the new version by
# itself, so the host and the friends who update by playing always match.
# The options for the server, kept before the shared part is loaded (loading it can reset $args).
$extra = @($args)
. (Join-Path $PSScriptRoot "common.ps1")
$Host.UI.RawUI.WindowTitle = "$Title server (close this window to stop it)"
function Stop-Here($text) { Warn $text; Read-Host "Press Enter to close"; exit 1 }

if (-not (Test-Path $ServerExe)) { Stop-Here "server\htl-server.exe is missing. Your anti-virus may have taken it: restore it, or download the game again." }
if (Get-OurServer) { Stop-Here "Your server is already running (another window is hosting). Only one at a time." }
foreach ($port in @(3724, 8085, 8086)) {
    if (PortOpen "127.0.0.1" $port) { Stop-Here "Port $port is taken: another server (another copy of the game, or a test build) is running on this PC. Close it first." }
}

# --- the newest version first ------------------------------------------------------------------------
if (Get-InstalledVersion) {
    Say "$Title $(Get-InstalledVersion). Checking for a new version..."
    & (Join-Path $Launcher "update.ps1")
    if ($LASTEXITCODE -eq 0) {
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Launcher "host.ps1") @extra
        exit $LASTEXITCODE
    }
}

$settings = Read-Settings
$data = Get-WowData $settings
if (-not $data) { Stop-Here "No WoW 1.12.1 folder picked: the server builds its worlds from your game files. Run Host again when you have one." }

# --- what friends type ---------------------------------------------------------------------------------
$serverArgs = @("--realm", $BasinName, "--survival-realm", $SurvivalName, "--data", $data,
    "--content", "content\game.toml", "--saves", "saves\basin-battle.toml",
    "--survival-content", "content\survival.toml", "--survival-save", "saves\elwynn-survival.toml")
Write-Host ""
Say "Give your friends one of these addresses (they choose 'Someone else's server' in Play):"
try {
    # The adapter with a default gateway is the one on the home network (WSL's and Hyper-V's
    # virtual adapters have none).
    $lan = Get-NetIPConfiguration -ErrorAction Stop |
        Where-Object { $_.IPv4DefaultGateway -and $_.NetAdapter.Status -eq "Up" } |
        ForEach-Object { $_.IPv4Address.IPAddress } | Select-Object -First 1
    if ($lan) { Say "  Same house or Wi-Fi:      $lan" }
} catch { }
$tailscale = Join-Path $env:ProgramFiles "Tailscale\tailscale.exe"
if (Test-Path $tailscale) {
    $ts = (& $tailscale ip -4 2>$null | Select-Object -First 1)
    if ($ts) { Say "  Over Tailscale:           $ts" }
}
if (-not ($extra -contains "--public-host")) {
    $public = $env:HTL_PUBLIC_HOST
    if (-not $public) {
        try { $public = (Invoke-RestMethod -Uri "https://api.ipify.org" -TimeoutSec 4).ToString().Trim() } catch { $public = $null }
    }
    if ($public -match '^[0-9a-fA-F.:]+$') {
        $serverArgs += @("--public-host", $public)
        Say "  Over the internet:        $public   (only once your router forwards TCP 3724, 8085 and 8086 to this PC)"
    }
}
Say "Password: hold, unless you started Host with --password."
Say "If Windows asks whether htl-server may use the network, click Allow."
Write-Host ""

# --- run, and come back after a warned restart --------------------------------------------------------------
Set-Location $ServerDir
New-Item -ItemType Directory -Force -Path (Join-Path $ServerDir "saves"), (Join-Path $ServerDir "logs") | Out-Null
$quoted = ($serverArgs + $extra | ForEach-Object { '"' + ($_ -replace '"', '\"') + '"' }) -join " "
[Console]::OutputEncoding = [Text.Encoding]::UTF8
while ($true) {
    # The watch for a new version: when one is out (and its download is ready), it asks the server
    # for a restart in 5 minutes, which warns everyone, saves the worlds and stops the server.
    $watch = $null
    if (Get-InstalledVersion) {
        $watch = Start-Job -ArgumentList $Repository, $Title, (Get-InstalledVersion), (Join-Path $ServerDir "restart.request") -ScriptBlock {
            param($repo, $title, $mine, $request)
            [Net.ServicePointManager]::SecurityProtocol = [Net.ServicePointManager]::SecurityProtocol -bor [Net.SecurityProtocolType]::Tls12
            while ($true) {
                Start-Sleep -Seconds 900
                try {
                    $r = Invoke-RestMethod -Uri "https://api.github.com/repos/$repo/releases/latest" -TimeoutSec 15 `
                        -Headers @{ "User-Agent" = "$title launcher"; "Accept" = "application/vnd.github+json" }
                    $ready = $r.assets | Where-Object { $_.name -like "*.zip" }
                    $newer = $false
                    try { $newer = [version]($r.tag_name.TrimStart('v') -replace '[^\d.].*$', '') -gt [version]($mine.TrimStart('v') -replace '[^\d.].*$', '') } catch { }
                    if ($newer -and $ready) { Set-Content -Path $request -Value "5"; return }
                } catch { }
            }
        }
    }
    $log = Join-Path $ServerDir ("logs\host-" + (Get-Date -Format "yyyy-MM-dd-HHmmss") + ".log")
    Say "This session's log: $log"
    $writer = New-Object System.IO.StreamWriter($log, $false, (New-Object System.Text.UTF8Encoding($false)))
    $writer.AutoFlush = $true
    try {
        & cmd /c "`"$ServerExe`" $quoted 2>&1" | ForEach-Object { Write-Host $_; $writer.WriteLine($_) }
        $code = $LASTEXITCODE
    } finally {
        $writer.Close()
        if ($watch) { Stop-Job $watch -ErrorAction SilentlyContinue; Remove-Job $watch -Force -ErrorAction SilentlyContinue }
    }
    # 75: a warned restart (a new version, or asked for by hand). Anything else: the server stopped.
    if ($code -ne 75) {
        Warn "The server stopped (code $code)."
        Read-Host "Press Enter to close"
        exit $code
    }
    Say "Restarting..."
    Set-Location $Root
    & (Join-Path $Launcher "update.ps1") -Quiet
    if ($LASTEXITCODE -eq 0) {
        # The new version's own Host takes it from here.
        & powershell.exe -NoProfile -ExecutionPolicy Bypass -File (Join-Path $Launcher "host.ps1") @extra
        exit $LASTEXITCODE
    }
    Set-Location $ServerDir
}
