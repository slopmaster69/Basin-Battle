# The visual check: plays scenarios from tools/visual-check/ in the real game client against the
# local test server and keeps what the screen showed (a burst of stills and an MP4), so an
# ability's effects, animations and icons are seen working before its pull request is called
# done. docs/visual-check.md explains it.
#
# "Visual Check.bat" in the top folder runs this; from PowerShell:
#   scripts\visual-check-windows.ps1                       lists the scenarios and asks which
#   scripts\visual-check-windows.ps1 hunter-aimed-shot     one scenario (several: a space between)
#   scripts\visual-check-windows.ps1 hunter-*              every scenario the pattern matches
#   scripts\visual-check-windows.ps1 all                   every scenario in turn
#   scripts\visual-check-windows.ps1 setup                 make the test heroes (once)
#   scripts\visual-check-windows.ps1 -Watch                run what request files ask for, forever
#   scripts\visual-check-windows.ps1 -InstallWatcher       start the watcher with Windows, and now
#   scripts\visual-check-windows.ps1 -UninstallWatcher     stop it, and not with Windows any more
#   scripts\visual-check-windows.ps1 -Check                read every scenario, run nothing (CI)
# What it needs: the server running on this PC (Host Basin Battle.bat), benilla's dev build, and
# visual-check-settings.txt (made on the first run; never committed). For video, ffmpeg.exe in
# Documents\visual-check\ffmpeg\.
param(
    [switch]$Watch,
    [switch]$InstallWatcher,
    [switch]$UninstallWatcher,
    [switch]$Check,
    [string]$Folder = "",
    [Parameter(ValueFromRemainingArguments = $true)][string[]]$Names = @()
)
$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
$ScenarioDir = Join-Path $Root "tools\visual-check"
$SettingsFile = Join-Path $Root "visual-check-settings.txt"
$Inv = [System.Globalization.CultureInfo]::InvariantCulture
# The watcher's folder (requests\, out\, scenarios\, ffmpeg\, watcher.log).
if (-not $Folder) { $Folder = Join-Path ([Environment]::GetFolderPath("MyDocuments")) "visual-check" }

function Say($text) { Write-Host $text -ForegroundColor Cyan }
function Num($s) { [double]::Parse([string]$s, $Inv) }
function Fmt($x) { ([double]$x).ToString("0.###", $Inv) }
function Quote($a) { if ($a -match '[\s"]') { '"' + $a.Replace('"', '\"') + '"' } else { $a } }

# --- settings ------------------------------------------------------------------------------------
# key = value lines; anything not set falls back to these.
$Defaults = [ordered]@{
    client        = ""
    data          = ""
    host          = "127.0.0.1"
    port          = "3724"
    realm         = "Elwynn Survival"
    account       = "spelltest"
    password      = "hold"
    hunter        = "Vchunter"
    warrior       = "Vcwarrior"
    resolution    = "1280x720"
    format        = "jpg"
    width         = "1280"
    video         = "title"
    ffmpeg        = ""
    settle        = "6"
    login_timeout = "120"
}
$SettingsHelp = @(
    "# The visual check's settings (docs/visual-check.md). This file stays on this PC: it's gitignored.",
    "# client: benilla's dev build (benilla.exe, or its folder): the 'Basin Battle for Windows (visual check)'",
    "#   artifact of the Release workflow run with 'dev' ticked. Empty: visual-check-client\benilla.exe here.",
    "# data: the Data folder of your WoW 1.12.1 install. Empty: the one settings-windows.txt names.",
    "# host, port, realm, account, password: the local test server and its test account (test credentials",
    "#   only, on a local server; the password is the one the host prints, 'hold' unless you changed it).",
    "# hunter, warrior: the test heroes on that account the scenarios play ('setup' makes them).",
    "# resolution: the client's window. format: jpg (small) or png. width: stills wider are scaled down.",
    "# video: title (record the game window), desktop (the screen where the window is: use it if the",
    "#   title's video comes out black) or off. ffmpeg: ffmpeg.exe (empty: Documents\visual-check\ffmpeg).",
    "# settle: seconds the world gets to load in before a scenario starts. login_timeout: seconds to log in."
)

function Read-Settings {
    $s = [ordered]@{}
    foreach ($k in $Defaults.Keys) { $s[$k] = $Defaults[$k] }
    if (Test-Path $SettingsFile) {
        foreach ($line in Get-Content $SettingsFile) {
            if ($line -match '^\s*([A-Za-z_]+)\s*=\s*(.*?)\s*$') { $s[$Matches[1].ToLower()] = $Matches[2].Trim('"') }
        }
    }
    $s
}

function Save-Settings($s) {
    $lines = @($SettingsHelp) + @($s.Keys | ForEach-Object { "$_ = $($s[$_])" })
    Set-Content -Path $SettingsFile -Value $lines
}

function DataFolderOf($dir) {
    if (-not $dir) { return $null }
    $dir = $dir.Trim().Trim('"')
    if (Test-Path (Join-Path $dir "dbc.MPQ")) { return (Resolve-Path $dir).Path }
    $inner = Join-Path $dir "Data"
    if (Test-Path (Join-Path $inner "dbc.MPQ")) { return (Resolve-Path $inner).Path }
    return $null
}

function ExeOf($path, $name) {
    if (-not $path) { return $null }
    $path = $path.Trim().Trim('"')
    if (Test-Path $path -PathType Container) { $path = Join-Path $path $name }
    if (Test-Path $path -PathType Leaf) { return (Resolve-Path $path).Path }
    return $null
}

# The settings filled in, asking only when someone is there to answer ($ask); otherwise an error
# that says what's missing.
function Resolve-Settings([bool]$ask) {
    $s = Read-Settings
    if (-not (Test-Path $SettingsFile)) { Save-Settings $s }
    $client = ExeOf $s.client "benilla.exe"
    if (-not $client) { $client = ExeOf (Join-Path $Root "visual-check-client") "benilla.exe" }
    while (-not $client) {
        if (-not $ask) { throw "No dev client: set client in $SettingsFile to benilla.exe of the visual-check build." }
        $client = ExeOf (Read-Host "Where is benilla.exe of the dev build (the 'Basin Battle for Windows (visual check)' folder)?") "benilla.exe"
        if ($client) { $s.client = $client; Save-Settings $s }
    }
    $data = DataFolderOf $s.data
    $hostSettings = Join-Path $Root "settings-windows.txt"
    if (-not $data -and (Test-Path $hostSettings)) { $data = DataFolderOf (Get-Content $hostSettings -TotalCount 1) }
    while (-not $data) {
        if (-not $ask) { throw "No WoW data: set data in $SettingsFile to the Data folder of WoW 1.12.1." }
        $data = DataFolderOf (Read-Host "Where is the Data folder of your WoW 1.12.1 install?")
        if ($data) { $s.data = $data; Save-Settings $s }
    }
    $s.client = $client
    $s.data = $data
    $s.ffmpeg = ExeOf $(if ($s.ffmpeg) { $s.ffmpeg } else { Join-Path $Folder "ffmpeg" }) "ffmpeg.exe"
    $s
}

# Only benilla's dev build has the scripted probes this drives; a player build would sit at login.
$DevChecked = @{}
function Assert-DevBuild($exe) {
    if ($DevChecked[$exe]) { return }
    $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
    if (-not $text.Contains("WOW_PROBE_LUA")) {
        throw "$exe is the player build, which has no scripted probes. Use the dev build: the Release workflow run with 'dev' ticked (docs/visual-check.md)."
    }
    $DevChecked[$exe] = $true
}

function Test-Server($s) {
    $tcp = New-Object System.Net.Sockets.TcpClient
    try {
        $wait = $tcp.BeginConnect($s.host, [int]$s.port, $null, $null)
        return ($wait.AsyncWaitHandle.WaitOne(2000) -and $tcp.Connected)
    } catch { return $false } finally { $tcp.Close() }
}

# A client or a recording of ours left running (a run cut short) goes first: the client would
# hold the account.
function Stop-Strays($s) {
    foreach ($exe in @($s.client, $s.ffmpeg)) {
        if (-not $exe) { continue }
        $name = [System.IO.Path]::GetFileNameWithoutExtension($exe)
        Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe } | ForEach-Object {
            Say "Closing a visual-check $name still running (pid $($_.Id))."
            Stop-Process -Id $_.Id -Force -ErrorAction SilentlyContinue
        }
    }
}

# --- scenarios -----------------------------------------------------------------------------------
function Find-Scenario($name) {
    foreach ($dir in @((Join-Path $Folder "scenarios"), $ScenarioDir)) {
        $f = Join-Path $dir "$name.txt"
        if (Test-Path $f) { return $f }
    }
    throw "No scenario called $name (looked in $Folder\scenarios and tools\visual-check)."
}

function List-Scenarios {
    $all = @{}
    foreach ($dir in @($ScenarioDir, (Join-Path $Folder "scenarios"))) {
        if (Test-Path $dir) {
            Get-ChildItem $dir -Filter *.txt | Where-Object { -not $_.Name.StartsWith("_") } | ForEach-Object { $all[$_.BaseName] = $true }
        }
    }
    @($all.Keys | Sort-Object)
}

# A scenario file (docs/visual-check.md, "Writing a scenario") as its steps, sorted by time
# with the file's order kept among equals.
function Read-Scenario($name, $depth = 0) {
    if ($depth -gt 4) { throw "$name includes itself." }
    $sc = @{ Name = $name; Hero = ""; Cam = ""; End = $null; Video = $null; Steps = New-Object System.Collections.ArrayList }
    $file = Find-Scenario $name
    $n = 0
    foreach ($raw in Get-Content $file) {
        $n++
        $line = ($raw -replace '(^|\s)#.*$', '').Trim()
        if (-not $line) { continue }
        $where = "$name.txt line $n"
        if ($line -match '^hero\s+(\S+)$') { $sc.Hero = $Matches[1].ToLower(); continue }
        if ($line -match '^cam\s+(\S+)\s+(\S+)\s+(\S+)$') {
            $sc.Cam = "$(Fmt (Num $Matches[1])),$(Fmt (Num $Matches[2])),$(Fmt (Num $Matches[3]))@0"
            continue
        }
        if ($line -match '^include\s+(\S+)$') {
            $inner = Read-Scenario $Matches[1] ($depth + 1)
            foreach ($st in $inner.Steps) { [void]$sc.Steps.Add($st) }
            if (-not $sc.Hero) { $sc.Hero = $inner.Hero }
            if (-not $sc.Cam) { $sc.Cam = $inner.Cam }
            continue
        }
        if ($line -notmatch '^([0-9]+(?:\.[0-9]+)?)\s+(.+)$') { throw "$($where): want '<seconds> <what>', got: $line" }
        $t = Num $Matches[1]
        $what = $Matches[2]
        if ($what -match '^shots\s+(\S+)\s+([0-9]+)\s+([0-9.]+)$') {
            [void]$sc.Steps.Add(@{ T = $t; Kind = "shots"; Text = $Matches[1]; Count = [int]$Matches[2]; Every = (Num $Matches[3]) })
        } elseif ($what -match '^lua\s+(.+)$') {
            [void]$sc.Steps.Add(@{ T = $t; Kind = "lua"; Text = $Matches[1] })
        } elseif ($what -eq "end") {
            $sc.End = $t
        } elseif ($what -eq "video") {
            $sc.Video = $t
        } elseif ($what.StartsWith("/")) {
            throw "$($where): a slash command isn't sent as chat here; use 'lua' with the function it calls."
        } else {
            [void]$sc.Steps.Add(@{ T = $t; Kind = "chat"; Text = $what })
        }
    }
    if ($depth -gt 0) { return $sc }
    $i = 0
    foreach ($st in $sc.Steps) { $st.Order = $i; $i++ }
    $sc.Steps = @($sc.Steps | Sort-Object -Property @{ Expression = { $_.T } }, @{ Expression = { $_.Order } })
    $last = 0.0
    $firstShot = $null
    foreach ($st in $sc.Steps) {
        $tEnd = $st.T
        if ($st.Kind -eq "shots") {
            $tEnd = $st.T + [Math]::Max(0, $st.Count - 1) * $st.Every
            if ($null -eq $firstShot -or $st.T -lt $firstShot) { $firstShot = $st.T }
        }
        if ($tEnd -gt $last) { $last = $tEnd }
    }
    if ($null -eq $sc.End) { $sc.End = $last + 3.0 }
    # The video starts a moment before the first shots (or where `<secs> video` says), to the end.
    # (0.0, not 0: with an int first, PowerShell picks Max(int, int) and rounds the time.)
    if ($null -eq $sc.Video) { $sc.Video = [Math]::Max(0.0, $(if ($null -ne $firstShot) { $firstShot } else { 0 }) - 1.5) }
    $sc
}

function LuaStr($s) { '"' + $s.Replace('\', '\\').Replace('"', '\"') + '"' }

# The driver the client runs once it's in the world (WOW_PROBE_LUA): from `settle` seconds later it
# says each line at its time, takes each burst of shots, logs what the server answers, and quits.
# With $cue it takes one shot at that time first, the video's cue (the script sees the file land).
function Build-Lua($sc, $settle, $cue) {
    $steps = @($sc.Steps)
    if ($null -ne $cue) {
        # In time order: the driver takes each step only once those before it are due.
        $steps = @(@(@{ T = $cue; Kind = "shots"; Text = "cue"; Count = 1; Every = 0; Order = -1 }) + $steps |
            Sort-Object -Property @{ Expression = { $_.T } }, @{ Expression = { $_.Order } })
    }
    $rows = foreach ($st in $steps) {
        if ($st.Kind -eq "shots") {
            "{$(Fmt $st.T),`"shots`",$(LuaStr $st.Text),$($st.Count),$(Fmt $st.Every)}"
        } else {
            "{$(Fmt $st.T),`"$($st.Kind)`",$(LuaStr $st.Text)}"
        }
    }
    $list = $rows -join ","
    @"
if VisualCheck then return end
local VC = { steps = { $list }, i = 1, bursts = {}, shot = 0, stop = $(Fmt $sc.End) }
VisualCheck = VC
local log = ProbeLog or function() end
VC.t0 = GetTime() + $(Fmt $settle)
log("visual-check: in the world; the scenario starts in $(Fmt $settle) s")
if ScreenshotStatus then
  ScreenshotStatus:UnregisterEvent("SCREENSHOT_SUCCEEDED")
  ScreenshotStatus:UnregisterEvent("SCREENSHOT_FAILED")
end
local function fill(text)
  local f, e = 0, 0
  if GetActionAim then
    local a, b = GetActionAim()
    if a then f = a; e = b or 0 end
  end
  text = string.gsub(text, "{aim}", string.format("%.4f %.4f", f, e))
  text = string.gsub(text, "{facing([^}]*)}", function(off)
    return string.format("%.4f", f + (tonumber(off) or 0))
  end)
  return text
end
local function run(st, now)
  local kind, text = st[2], st[3]
  local at = string.format("%.2f", now)
  if kind == "chat" then
    text = fill(text)
    log("visual-check: t=" .. at .. " say " .. text)
    if UnitIsDeadOrGhost and UnitIsDeadOrGhost("player") then
      SendChatMessage(text, "WHISPER", nil, UnitName("player"))
    else
      SendChatMessage(text, "SAY")
    end
  elseif kind == "lua" then
    log("visual-check: t=" .. at .. " lua " .. text)
    local f, err = loadstring(text)
    if not f then
      log("visual-check: lua error: " .. tostring(err))
    else
      local ok, e2 = pcall(f)
      if not ok then log("visual-check: lua error: " .. tostring(e2)) end
    end
  elseif kind == "shots" then
    table.insert(VC.bursts, { label = text, left = st[4], every = st[5], next = st[1] })
  end
end
local ev = CreateFrame("Frame")
VC.events = ev
ev:RegisterEvent("CHAT_MSG_SYSTEM")
ev:RegisterEvent("UI_ERROR_MESSAGE")
ev:RegisterEvent("CHAT_MSG_ADDON")
ev:SetScript("OnEvent", function()
  local text = tostring(arg1)
  if event == "CHAT_MSG_ADDON" then
    -- The add-on's own lines: only its words to the hero (T) and what flashes mid-screen (FL:
    -- "No enemy in range", "Too far"), not the HUD's.
    local _, _, said = string.find(tostring(arg2), "^T\t(.*)")
    if not said then
      local _, _, flashed = string.find(tostring(arg2), "^FL\t(.*)")
      if not flashed then return end
      said = "(flash) " .. flashed
    end
    text = said
  end
  log("visual-check: server: " .. text)
end)
local frame = CreateFrame("Frame")
VC.frame = frame
frame:SetScript("OnUpdate", function()
  -- No window over the scene, ever: the board .es spells and .es slot open, the week's letter
  -- at login, the ledger (every window Escape closes).
  if UISpecialFrames then
    for _, n in ipairs(UISpecialFrames) do
      local w = getglobal(n)
      if w and w:IsShown() then w:Hide() end
    end
  end
  local now = GetTime() - VC.t0
  if now < 0 then return end
  while VC.steps[VC.i] and VC.steps[VC.i][1] <= now do
    run(VC.steps[VC.i], now)
    VC.i = VC.i + 1
  end
  for _, b in ipairs(VC.bursts) do
    if b.left > 0 and now >= b.next then
      if ScreenshotStatus then ScreenshotStatus:Hide() end
      Screenshot()
      VC.shot = VC.shot + 1
      log("visual-check: shot " .. VC.shot .. " " .. b.label .. " t=" .. string.format("%.2f", now))
      b.left = b.left - 1
      b.next = b.next + b.every
      if b.next < now then b.next = now end
    end
  end
  if now >= VC.stop and not VC.done then
    VC.done = true
    log("visual-check: done, " .. VC.shot .. " shots asked for")
    ForceQuit()
  end
end)
"@
}

# --- video ---------------------------------------------------------------------------------------
Add-Type @"
using System;
using System.Runtime.InteropServices;
public static class VisualCheckWin {
    [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
}
"@
[void][VisualCheckWin]::SetProcessDPIAware()

# ffmpeg recording the client's window ("benilla", the title benilla's lib.rs gives it) for
# $v.Duration seconds into $v.Out; `desktop` records the screen's rectangle where the window is.
function Start-Video($v, $client, [string]$how) {
    $grab = @("-f", "gdigrab", "-framerate", "30")
    if ($how -eq "desktop") {
        $client.Refresh()
        $r = New-Object VisualCheckWin+RECT
        if (-not [VisualCheckWin]::GetWindowRect($client.MainWindowHandle, [ref]$r)) { return $null }
        $w = ($r.Right - $r.Left) - (($r.Right - $r.Left) % 2)
        $h = ($r.Bottom - $r.Top) - (($r.Bottom - $r.Top) % 2)
        $grab += @("-offset_x", "$($r.Left)", "-offset_y", "$($r.Top)", "-video_size", "$($w)x$($h)", "-i", "desktop")
    } else {
        $grab += @("-i", "title=benilla")
    }
    $all = @("-hide_banner", "-nostdin", "-loglevel", "warning", "-y") + $grab + @(
        "-t", (Fmt $v.Duration), "-vf", "scale=trunc(iw/2)*2:trunc(ih/2)*2",
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p", $v.Out)
    $v.How = $how
    $v.Started = Get-Date
    $rec = Start-Process -FilePath $v.Ffmpeg -ArgumentList (($all | ForEach-Object { Quote $_ }) -join " ") `
        -NoNewWindow -PassThru -RedirectStandardError "$($v.Out).log"
    $null = $rec.Handle  # held now, or Windows PowerShell never learns its exit code
    $rec
}

# --- running the client --------------------------------------------------------------------------
# Runs the client with these variables until it quits or the time is up (then it's closed), its
# output in $log. With $v (a video), recording starts when the cue shot lands in the screenshots
# folder. Returns whether the client quit by itself.
function Invoke-Client($s, [hashtable]$vars, [int]$timeout, $chome, $log, $v = $null) {
    Stop-Strays $s
    $common = @{
        WOW_DATA = $s.data; WOW_HOST = $s.host; WOW_REALM = $s.realm
        WOW_USER = $s.account; WOW_PASS = $s.password
        BENILLA_HOME = $chome
        # Nobody at the keyboard: a refused login ends the run instead of waiting on a dialog;
        # WOW_BG=0 keeps the full-size window a run with only WOW_PROBE variables would shrink.
        WOW_UNATTENDED = "1"; WOW_BG = "0"
        WOW_PROBE_EXIT_AT = [string]($timeout - 5)
    }
    foreach ($k in $vars.Keys) { $common[$k] = $vars[$k] }
    $saved = @{}
    foreach ($k in $common.Keys) {
        $saved[$k] = [Environment]::GetEnvironmentVariable($k, "Process")
        [Environment]::SetEnvironmentVariable($k, [string]$common[$k], "Process")
    }
    $p = $null
    $rec = $null
    $shotsDir = Join-Path $chome "Screenshots"
    try {
        $p = Start-Process -FilePath $s.client -WorkingDirectory $chome -PassThru `
            -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
        $deadline = (Get-Date).AddSeconds($timeout)
        while (-not $p.HasExited -and (Get-Date) -lt $deadline) {
            if ($v -and -not $rec -and (Test-Path $shotsDir) -and (Get-ChildItem $shotsDir -Filter *.png)) {
                $rec = Start-Video $v $p $s.video
                if (-not $rec) { $v.Status = "no video: the game window wasn't found" }
            }
            # The window's title didn't match: once more, from the screen where the window is.
            if ($rec -and $rec.HasExited -and $null -ne $rec.ExitCode -and $rec.ExitCode -ne 0 -and $v.How -eq "title" -and ((Get-Date) - $v.Started).TotalSeconds -lt 5) {
                $v.Duration = [Math]::Max(1.0, $v.Duration - ((Get-Date) - $v.Started).TotalSeconds)
                $rec = Start-Video $v $p "desktop"
            }
            Start-Sleep -Milliseconds 100
        }
        return $p.HasExited
    } finally {
        if ($p -and -not $p.HasExited) {
            Say "  the client ran past $timeout s: closing it"
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            $p.WaitForExit(10000) | Out-Null
        }
        if ($rec) {
            if (-not $rec.WaitForExit(15000)) { Stop-Process -Id $rec.Id -Force -ErrorAction SilentlyContinue }
            if ((Test-Path $v.Out) -and (Get-Item $v.Out).Length -gt 0) {
                $v.Status = "video: $($v.Out) ($($v.How))"
                Remove-Item "$($v.Out).log" -ErrorAction SilentlyContinue
            } else {
                $v.Status = "no video: ffmpeg wrote nothing (see $($v.Out).log)"
            }
        } elseif ($v -and -not $v.Status) {
            $v.Status = "no video: the scenario's first shot never came"
        }
        foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k], "Process") }
        $parts = @("$log.out", "$log.err") | Where-Object { Test-Path $_ }
        if ($parts) { Get-Content $parts -ErrorAction SilentlyContinue | Set-Content $log }
        Remove-Item $parts -ErrorAction SilentlyContinue
    }
}

# A settings folder of the run's own: the bundled config at the chosen window size and this
# checkout's add-ons, so the add-on under test is the branch's and no player setting is touched.
function New-ClientHome($s, $dir) {
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force -Path (Join-Path $dir "AddOns") | Out-Null
    $config = @(Get-Content (Join-Path $Root "dist\windows\benilla-config\config.toml"))
    $config += "gxWindow = `"1`""
    $config += "gxResolution = `"$($s.resolution)`""
    Set-Content (Join-Path $dir "config.toml") $config
    Copy-Item -Recurse (Join-Path $Root "addon\ElwynnSurvival"), (Join-Path $Root "addon\BasinBattle") (Join-Path $dir "AddOns")
}

function Get-LogTail($log, $n = 40) {
    if (Test-Path $log) { (Get-Content $log -Tail $n) -join "`r`n" } else { "(no client log)" }
}

# The stills in the order they were taken, renamed <label>-NN, as JPG or PNG no wider than the
# setting; the video's cue is dropped.
function Save-Shots($s, $from, $to, $log, [bool]$cue) {
    $files = @()
    if (Test-Path $from) {
        $files = @(Get-ChildItem $from -Filter "WoWScrnShot_*.png" | Sort-Object -Property @{ Expression = {
            if ($_.Name -match '_(\d\d)(\d\d)(\d\d)_(\d{6})(?:_(\d+))?\.png$') {
                $Matches[3] + $Matches[1] + $Matches[2] + $Matches[4] + ([int]("0" + $Matches[5])).ToString("000")
            } else { $_.Name } } })
    }
    $labels = @()
    if (Test-Path $log) {
        $labels = @(Select-String -Path $log -Pattern 'visual-check: shot \d+ (\S+)' | ForEach-Object { $_.Matches[0].Groups[1].Value })
    }
    if ($labels.Count -ne $files.Count) {
        if ($files.Count -gt 0) { Say "  $($files.Count) stills written, $($labels.Count) asked for: named in order, not by burst" }
        $labels = @(for ($k = 0; $k -lt $files.Count; $k++) { if ($cue -and $k -eq 0) { "cue" } else { "shot" } })
    }
    Add-Type -AssemblyName System.Drawing
    $jpg = [System.Drawing.Imaging.ImageCodecInfo]::GetImageEncoders() | Where-Object { $_.MimeType -eq "image/jpeg" }
    $params = New-Object System.Drawing.Imaging.EncoderParameters 1
    $params.Param[0] = New-Object System.Drawing.Imaging.EncoderParameter ([System.Drawing.Imaging.Encoder]::Quality, [long]85)
    $max = [int]$s.width
    $count = @{}
    $saved = 0
    for ($k = 0; $k -lt $files.Count; $k++) {
        $label = $labels[$k]
        if ($label -eq "cue") { continue }
        $count[$label] = 1 + [int]$count[$label]
        $stem = Join-Path $to ("{0}-{1:D2}" -f $label, ($count[$label] - 1))
        $img = [System.Drawing.Image]::FromFile($files[$k].FullName)
        try {
            $w = $img.Width; $h = $img.Height
            if ($max -gt 0 -and $w -gt $max) { $h = [int]($h * $max / $w); $w = $max }
            if ($s.format -eq "png" -and $w -eq $img.Width) {
                $img.Dispose(); $img = $null
                Move-Item $files[$k].FullName "$stem.png"
            } else {
                $bmp = New-Object System.Drawing.Bitmap $w, $h
                $g = [System.Drawing.Graphics]::FromImage($bmp)
                $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
                $g.DrawImage($img, 0, 0, $w, $h)
                $g.Dispose()
                if ($s.format -eq "png") { $bmp.Save("$stem.png", [System.Drawing.Imaging.ImageFormat]::Png) }
                else { $bmp.Save("$stem.jpg", $jpg, $params) }
                $bmp.Dispose()
            }
        } finally { if ($img) { $img.Dispose() } }
        $saved++
    }
    $saved
}

# One scenario: its stills, <name>.mp4, run.txt (what was said and answered) and client.log in
# $out. Returns @{ Name; Ok; Shots; Message; Tail; Out }.
function Invoke-Scenario($s, $name, $out) {
    $sc = Read-Scenario $name
    $heroKey = if ($sc.Hero) { $sc.Hero } else { "hunter" }
    $hero = $s[$heroKey]
    if (-not $hero) { throw "$name plays the $heroKey hero, and the settings name none (set $heroKey = <name>)." }
    $cam = if ($sc.Cam) { $sc.Cam } else { "0,-10,9@0" }
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $chome = Join-Path $out "client"
    New-ClientHome $s $chome
    $settle = Num $s.settle
    $v = $null
    $videoNote = ""
    if ($s.video -eq "off") { $videoNote = "no video: video = off in the settings" }
    elseif (-not $s.ffmpeg) { $videoNote = "no video: no ffmpeg.exe in $(Join-Path $Folder 'ffmpeg') (or the ffmpeg setting)" }
    else { $v = @{ Ffmpeg = $s.ffmpeg; Out = (Join-Path $out "$name.mp4"); Duration = [Math]::Max(1.0, $sc.End - $sc.Video); Status = "" } }
    $lua = Build-Lua $sc $settle $(if ($v) { $sc.Video } else { $null })
    if ($lua.Length -gt 30000) { throw "$name is too long to hand the client ($($lua.Length) characters of Lua; 30000 at most)." }
    $asked = 0
    foreach ($st in $sc.Steps) { if ($st.Kind -eq "shots") { $asked += $st.Count } }
    $timeout = [int]((Num $s.login_timeout) + $settle + $sc.End + 30)
    Say "$name : $hero, $([int]$sc.End) s, $asked stills (closed after $timeout s at most)"
    $log = Join-Path $out "client.log"
    $quit = Invoke-Client $s @{
        WOW_CHAR = $hero; WOW_PROBE_LUA = $lua; WOW_PROBE_LUA_AT = "1"; WOW_PROBE_CAM = $cam
    } $timeout $chome $log $v
    $shots = Save-Shots $s (Join-Path $chome "Screenshots") $out $log ($null -ne $v)
    $runLog = @()
    if (Test-Path $log) {
        $runLog = @(Select-String -Path $log -Pattern 'visual-check: (.*)$' | ForEach-Object { $_.Matches[0].Groups[1].Value })
    }
    if ($v) { $videoNote = $v.Status }
    $runLog += $videoNote
    Set-Content (Join-Path $out "run.txt") $runLog
    Remove-Item -Recurse -Force $chome -ErrorAction SilentlyContinue
    $done = (Test-Path $log) -and (Select-String -Path $log -Pattern 'visual-check: done' -Quiet)
    $msg = if ($shots -eq 0) {
        if (-not (Test-Path $log) -or -not (Select-String -Path $log -Pattern 'visual-check: in the world' -Quiet)) {
            "no stills: the client never got into the world (see client.log: the login, the realm or the hero $hero)"
        } else { "no stills were written (see client.log)" }
    } elseif (-not $done) {
        "$shots of $asked stills, but the scenario didn't finish$(if (-not $quit) { ' (the client was closed at the time limit)' })"
    } elseif ($shots -lt $asked) { "$shots of $asked stills written" } else { "$shots stills" }
    $msg = "$msg; $videoNote"
    $ok = $shots -gt 0 -and $done
    Say "  $msg -> $out"
    @{ Name = $name; Ok = $ok; Shots = $shots; Message = $msg; Tail = $(if ($ok) { "" } else { Get-LogTail $log }); Out = $out }
}

# The test heroes: each made at character select (benilla's WOW_PROBE_CHARCREATE), kept. A hero
# already there stays as it is (the server answers that the name is in use).
$HeroBodies = @{ hunter = "3,3,0"; warrior = "1,1,0" }  # race, class, gender: a dwarf hunter, a human warrior
function Invoke-Setup($s, $out) {
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $results = @()
    foreach ($role in @("hunter", "warrior")) {
        $hero = $s[$role]
        if (-not $hero) { continue }
        $chome = Join-Path $out "client-$role"
        New-ClientHome $s $chome
        $log = Join-Path $out "setup-$role.log"
        Say "setup: the $role hero $hero on $($s.account)"
        Invoke-Client $s @{ WOW_PROBE_CHARCREATE = "$hero,$($HeroBodies[$role])"; WOW_PROBE_CHARCREATE_KEEP = "1" } `
            ([int](Num $s.login_timeout)) $chome $log | Out-Null
        Remove-Item -Recurse -Force $chome -ErrorAction SilentlyContinue
        $line = $null
        if (Test-Path $log) { $line = Select-String -Path $log -Pattern 'probe-charcreate: create result = (\S+) \((.*)\)' | Select-Object -Last 1 }
        $label = if ($line) { $line.Matches[0].Groups[2].Value } else { "" }
        $made = $label -match 'SUCCESS'
        $taken = $label -match 'IN_USE'
        $msg = if ($made) { "$hero made" } elseif ($taken) { "$hero is there already (or the name is another account's)" }
               elseif ($line) { "$hero not made: $label" } else { "$hero not made: the client never reached character select (see setup-$role.log)" }
        Say "  $msg"
        $results += @{ Name = "setup-$role"; Ok = ($made -or $taken); Shots = 0; Message = $msg; Tail = $(if ($made -or $taken) { "" } else { Get-LogTail $log }); Out = $out }
    }
    $results
}

# Runs the named scenarios ("all", patterns, "setup") into $outRoot\<name>, one run at a time on
# this PC. Returns the results.
function Invoke-Runs($s, [string[]]$names, $outRoot) {
    $want = @()
    foreach ($n in $names) {
        foreach ($part in ($n -split '[\s,;]+')) {
            if (-not $part) { continue }
            if ($part -eq "all") { $want += List-Scenarios }
            elseif ($part -eq "setup") { $want += "setup" }
            elseif ($part.Contains("*")) { $want += @(List-Scenarios | Where-Object { $_ -like $part }) }
            else { $want += $part }
        }
    }
    if ($want.Count -eq 0) { throw "No scenario named." }
    Assert-DevBuild $s.client
    if (-not (Test-Server $s)) { throw "The test server isn't answering at $($s.host):$($s.port). Start Host Basin Battle.bat first." }
    $lock = New-Object System.Threading.Mutex($false, "Local\BasinBattleVisualCheck")
    $held = $false
    try {
        try { $held = $lock.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $held = $true }
        if (-not $held) { throw "Another visual check is running on this PC; one at a time." }
        $results = @()
        foreach ($name in $want) {
            $out = Join-Path $outRoot $name
            try {
                if ($name -eq "setup") { $results += Invoke-Setup $s $out }
                else { $results += Invoke-Scenario $s $name $out }
            } catch {
                Say "  $name failed: $($_.Exception.Message)"
                $results += @{ Name = $name; Ok = $false; Shots = 0; Message = $_.Exception.Message; Tail = ""; Out = $out }
            } finally {
                Stop-Strays $s
            }
        }
        $results
    } finally {
        if ($held) { $lock.ReleaseMutex() }
        $lock.Dispose()
    }
}

# --- the watcher ---------------------------------------------------------------------------------
# <Folder>\requests\<id>.txt names the scenarios (or all, or setup), a space or a line between.
# They run into <Folder>\out\<id>\<scenario>\; <Folder>\out\<id>.done (or .failed, with the errors
# and the client's last lines) says how it went, and the request moves to requests\done\.
function Write-WatchLog($text) {
    $line = "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') $text"
    Write-Host $line
    Add-Content -Path (Join-Path $Folder "watcher.log") -Value $line
}

function Invoke-Request($file) {
    $id = $file.BaseName -replace '[^A-Za-z0-9._-]', '_'
    $outRoot = Join-Path $Folder "out"
    $out = Join-Path $outRoot $id
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    Remove-Item (Join-Path $outRoot "$id.done"), (Join-Path $outRoot "$id.failed") -ErrorAction SilentlyContinue
    Set-Content (Join-Path $out "status.txt") "running since $(Get-Date -Format s)"
    $names = @(Get-Content $file.FullName | ForEach-Object { ($_ -replace '#.*$', '').Trim() } | Where-Object { $_ })
    Write-WatchLog "request $id : $($names -join ' ')"
    $report = @()
    $failed = $false
    try {
        $s = Resolve-Settings $false
        $results = Invoke-Runs $s $names $out
        foreach ($r in $results) {
            $word = if ($r.Ok) { "ok    " } else { "FAILED" }
            Write-WatchLog "  $word $($r.Name): $($r.Message)"
            $report += "$word $($r.Name): $($r.Message)  [$($r.Out)]"
            if (-not $r.Ok) {
                $failed = $true
                if ($r.Tail) { $report += "  the client's last lines:"; $report += ($r.Tail -split "`r?`n" | ForEach-Object { "    $_" }) }
            }
        }
    } catch {
        $failed = $true
        $report += "FAILED: $($_.Exception.Message)"
        Write-WatchLog "  FAILED: $($_.Exception.Message)"
    }
    $status = if ($failed) { "failed" } else { "done" }
    Set-Content (Join-Path $outRoot "$id.$status") $report
    Set-Content (Join-Path $out "status.txt") "$status $(Get-Date -Format s)"
    $doneDir = Join-Path $Folder "requests\done"
    New-Item -ItemType Directory -Force -Path $doneDir | Out-Null
    Move-Item -Force $file.FullName (Join-Path $doneDir $file.Name)
    Write-WatchLog "request $id $status"
}

# A new build to check, with nobody at the PC: a branch's source zip (`git archive`) dropped in
# deploy\ as <name>.zip. Only where this copy of the game came from such a zip (no .git here: a
# checkout is never replaced): a handover script stops the server and this watcher, replaces
# everything but the build, saves, logs, settings and visual-check files, starts the server on the
# new source (it builds first) and a new watcher from it. The steps go to watcher.log.
$Handover = @'
param([string]$Zip, [string]$Root, [string]$Folder, [int]$Watcher)
$ErrorActionPreference = "Stop"
$log = Join-Path $Folder "watcher.log"
function L($t) { Add-Content -Path $log -Value "$(Get-Date -Format 'yyyy-MM-dd HH:mm:ss') deploy: $t" }
try {
    Wait-Process -Id $Watcher -Timeout 60 -ErrorAction SilentlyContinue
    Get-Process htl-server -ErrorAction SilentlyContinue | Stop-Process -Force
    Start-Sleep -Seconds 2
    Get-ChildItem $Root -Force -Exclude target, saves, logs, settings-windows.txt, settings.sh, "visual-check-*" | Remove-Item -Recurse -Force
    Expand-Archive -Path $Zip -DestinationPath $Root -Force
    Remove-Item $Zip -Force
    # A zip's files keep the commit's time, which can be older than the last build: cargo would
    # call the old build fresh. Now, so it rebuilds what's there.
    $now = Get-Date
    Get-ChildItem (Join-Path $Root "crates"), (Join-Path $Root "content") -Recurse -File -ErrorAction SilentlyContinue | ForEach-Object { $_.LastWriteTime = $now }
    L "unpacked into $Root; starting the server (it builds first)"
    Start-Process -FilePath (Join-Path $Root "Host Basin Battle.bat") -WorkingDirectory $Root -WindowStyle Minimized
    $until = (Get-Date).AddMinutes(40); $up = $false
    while (-not $up -and (Get-Date) -lt $until) {
        try { $t = New-Object System.Net.Sockets.TcpClient; $t.Connect("127.0.0.1", 3724); $up = $t.Connected; $t.Close() } catch { Start-Sleep -Seconds 10 }
    }
    if ($up) { L "the server answers" } else { L "the server never answered on 3724 (a build error? see its window)" }
} catch {
    L "failed: $($_.Exception.Message)"
} finally {
    $me = '"' + (Join-Path $Root "scripts\visual-check-windows.ps1") + '"'
    Start-Process powershell -WindowStyle Minimized -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", $me, "-Watch")
    L "a new watcher is started"
}
'@

function Invoke-Deploy($zip) {
    if (Test-Path (Join-Path $Root ".git")) {
        Write-WatchLog "deploy $($zip.Name): refused, $Root is a git checkout (deploys only replace a test copy made from a zip)"
        Move-Item $zip.FullName ($zip.FullName + ".refused") -Force
        return $false
    }
    $dir = Join-Path $Folder "deploy\in-progress"
    New-Item -ItemType Directory -Force -Path $dir | Out-Null
    $staged = Join-Path $dir "deploying.zip"
    Move-Item $zip.FullName $staged -Force
    $script = Join-Path $dir "handover.ps1"
    Set-Content -Path $script -Value $Handover
    Write-WatchLog "deploy $($zip.Name): handing over (this watcher stops; a new one starts once the server is up)"
    Start-Process powershell -WindowStyle Minimized -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", (Quote $script),
        "-Zip", (Quote $staged), "-Root", (Quote $Root), "-Folder", (Quote $Folder), "-Watcher", $PID)
    return $true
}

function Start-Watcher {
    $requests = Join-Path $Folder "requests"
    foreach ($d in @($requests, (Join-Path $Folder "out"), (Join-Path $Folder "scenarios"), (Join-Path $Folder "deploy"))) {
        New-Item -ItemType Directory -Force -Path $d | Out-Null
    }
    $one = New-Object System.Threading.Mutex($false, "Local\BasinBattleVisualCheckWatcher")
    $mine = $false
    try { $mine = $one.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $mine = $true }
    if (-not $mine) { Write-Host "A visual-check watcher is already running on this PC."; return }
    $Host.UI.RawUI.WindowTitle = "Visual check watcher (close this window to stop it)"
    Write-WatchLog "watching $requests (from $Root)"
    try {
        while ($true) {
            # A new build first (between requests, never during one).
            $zip = Get-ChildItem (Join-Path $Folder "deploy") -Filter *.zip -File -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -lt (Get-Date).AddSeconds(-5) } | Sort-Object LastWriteTime | Select-Object -First 1
            if ($zip -and (Invoke-Deploy $zip)) { break }
            # Oldest first, and only a file nobody has written to for 2 s (not still being copied in).
            $next = Get-ChildItem $requests -Filter *.txt -File -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -lt (Get-Date).AddSeconds(-2) } |
                Sort-Object LastWriteTime | Select-Object -First 1
            if ($next) {
                try { Invoke-Request $next } catch { Write-WatchLog "request $($next.Name): the watcher's own step broke: $($_.Exception.Message)" }
            } else {
                Start-Sleep -Seconds 3
            }
        }
    } finally {
        $one.ReleaseMutex()
        $one.Dispose()
    }
}

# The watcher in the Startup folder: a small .bat there starts it minimized at every sign-in.
$StartupBat = Join-Path ([Environment]::GetFolderPath("Startup")) "Basin Battle visual check watcher.bat"
function Install-Watcher {
    Set-Content -Path $StartupBat -Encoding ASCII -Value @(
        "@echo off",
        "rem Starts the Basin Battle visual check watcher (docs/visual-check.md), minimized.",
        "start `"Visual check watcher`" /min powershell -NoProfile -ExecutionPolicy Bypass -File `"$PSCommandPath`" -Watch"
    )
    Say "The watcher starts with Windows now ($StartupBat)."
    Start-Process -FilePath $StartupBat -WindowStyle Minimized
    Say "It's started too: it watches $(Join-Path $Folder 'requests') and logs to $(Join-Path $Folder 'watcher.log')."
}

function Uninstall-Watcher {
    if (Test-Path $StartupBat) { Remove-Item $StartupBat; Say "The watcher no longer starts with Windows." }
    else { Say "The watcher wasn't set to start with Windows." }
    $me = $PID
    Get-CimInstance Win32_Process -Filter "Name = 'powershell.exe' OR Name = 'pwsh.exe'" |
        Where-Object { $_.ProcessId -ne $me -and $_.CommandLine -like "*visual-check-windows.ps1*-Watch*" } |
        ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue; Say "Stopped the running watcher (pid $($_.ProcessId))." }
    try { Stop-Strays (Resolve-Settings $false) } catch { }
}

# --- main ----------------------------------------------------------------------------------------
if ($Check) {
    # Every scenario read and turned into the client's Lua, nothing run (CI does this).
    $bad = 0
    foreach ($name in List-Scenarios) {
        try {
            $sc = Read-Scenario $name
            $lua = Build-Lua $sc 6 $sc.Video
            $shots = 0
            foreach ($st in $sc.Steps) { if ($st.Kind -eq "shots") { $shots += $st.Count } }
            if ($lua.Length -gt 30000) { throw "$($lua.Length) characters of Lua; 30000 at most" }
            Write-Host "ok     $name : $($sc.Steps.Count) steps, $shots stills, $(Fmt $sc.End) s, video from $(Fmt $sc.Video) s"
        } catch {
            $bad++
            Write-Host "FAILED $name : $($_.Exception.Message)"
        }
    }
    if ($bad -gt 0) { exit 1 }
    exit 0
}
if ($InstallWatcher) { Install-Watcher; exit 0 }
if ($UninstallWatcher) { Uninstall-Watcher; exit 0 }
if ($Watch) { Start-Watcher; exit 0 }

$s = Resolve-Settings $true
if ($Names.Count -eq 0) {
    Say "Scenarios (tools\visual-check):"
    List-Scenarios | ForEach-Object { Write-Host "  $_" }
    Write-Host "  all     every one of them in turn"
    Write-Host "  setup   make the test heroes ($($s.hunter), $($s.warrior)) on $($s.account), once"
    $answer = (Read-Host "Which (a name, a pattern like hunter-*, or several with a space between)?").Trim()
    if (-not $answer) { exit 0 }
    $Names = @($answer)
}
$outRoot = Join-Path $Root "visual-check-out"
$results = Invoke-Runs $s $Names $outRoot
$bad = @($results | Where-Object { -not $_.Ok })
Say ""
foreach ($r in $results) { Write-Host "$(if ($r.Ok) { 'ok    ' } else { 'FAILED' }) $($r.Name): $($r.Message)" }
Say "Stills and videos: $outRoot"
if ($bad.Count -gt 0) { exit 1 }
