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
# (Not positional: a scenario's name given alone is a name, never the -Folder.)
[CmdletBinding(PositionalBinding = $false)]
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
    cameraman     = "Vccamera"
    resolution    = "1280x720"
    hero_resolution = "560x315"
    monitor       = ""
    mute          = "1"
    format        = "jpg"
    width         = "1280"
    video         = "screen"
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
    "# cameraman: the hero whose client films a scenario with a 'camera' line ('setup' makes it too).",
    "# resolution: the client's window. format: jpg (small) or png. width: stills wider are scaled down.",
    "# hero_resolution: with a cameraman, the hero's own window, beside the cameraman's on the same screen.",
    "# monitor: which screen the game window goes on, and the video films. Empty or 0: wherever Windows opens",
    "#   it. 1 is the main screen (the one with the taskbar by default), 2 the next, and so on, the others in",
    "#   the order Windows lists them; or a name like DISPLAY2 (Settings > System > Display > Identify).",
    "# mute: 1 (the default) mutes the game's sound in Windows' mixer from the moment it starts; 0 leaves it.",
    "# video: screen (the game window's picture, by Desktop Duplication: the one that sees a Vulkan",
    "#   window's frames), title or desktop (GDI: they see only a few stale frames of benilla's window), or off.",
    "#   ffmpeg: ffmpeg.exe (empty: Documents\visual-check\ffmpeg).",
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
function Stop-Strays($s, $keep = 0) {
    foreach ($exe in @($s.client, $s.ffmpeg)) {
        if (-not $exe) { continue }
        $name = [System.IO.Path]::GetFileNameWithoutExtension($exe)
        Get-Process $name -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $exe -and $_.Id -ne $keep } | ForEach-Object {
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
    $sc = @{ Name = $name; Hero = ""; Cam = ""; Camera = $null; End = $null; Video = $null; Steps = New-Object System.Collections.ArrayList }
    $file = Find-Scenario $name
    $n = 0
    foreach ($raw in Get-Content $file) {
        $n++
        $line = ($raw -replace '(^|\s)#.*$', '').Trim()
        if (-not $line) { continue }
        $where = "$name.txt line $n"
        if ($line -match '^hero\s+(\S+)$') { $sc.Hero = $Matches[1].ToLower(); continue }
        # `cam off`: the camera isn't held (a scenario that turns it itself, as the god-ray circles do).
        if ($line -match '^cam\s+off$') { $sc.Cam = "off"; continue }
        # `camera <duel|low|high> [<subject's name>]`: a second client, the cameraman, films it
        # (docs/visual-check.md, "Filming a boss").
        if ($line -match '^camera\s+(\S+)(?:\s+(.+))?$') {
            $shot = $Matches[1].ToLower()
            if ($shot -notin @("duel", "low", "high")) { throw "$($where): the camera's shot is duel, low or high, not $shot" }
            $subject = "$($Matches[2])".Trim()
            if ($subject.Contains(",")) { throw "$($where): a name with a comma can't be filmed" }
            $sc.Camera = @{ Shot = $shot; Subject = $subject }
            continue
        }
        if ($line -match '^cam\s+(\S+)\s+(\S+)\s+(\S+)$') {
            $sc.Cam = "$(Fmt (Num $Matches[1])),$(Fmt (Num $Matches[2])),$(Fmt (Num $Matches[3]))@0"
            continue
        }
        if ($line -match '^include\s+(\S+)$') {
            $inner = Read-Scenario $Matches[1] ($depth + 1)
            foreach ($st in $inner.Steps) { [void]$sc.Steps.Add($st) }
            if (-not $sc.Hero) { $sc.Hero = $inner.Hero }
            if (-not $sc.Cam) { $sc.Cam = $inner.Cam }
            if (-not $sc.Camera) { $sc.Camera = $inner.Camera }
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
    [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X, Y; }
    [DllImport("user32.dll")] static extern bool GetClientRect(IntPtr h, out RECT r);
    [DllImport("user32.dll")] static extern bool ClientToScreen(IntPtr h, ref POINT p);
    // The window's picture alone (not its title bar, borders or what shows past them), on the
    // desktop.
    public static bool GetPictureRect(IntPtr h, out RECT r) {
        r = new RECT(); RECT c; POINT p = new POINT();
        if (!GetClientRect(h, out c) || !ClientToScreen(h, ref p)) return false;
        r.Left = p.X; r.Top = p.Y; r.Right = p.X + c.Right; r.Bottom = p.Y + c.Bottom;
        return c.Right > 0 && c.Bottom > 0;
    }
    [DllImport("user32.dll")] public static extern bool SetProcessDPIAware();
    [DllImport("user32.dll")] public static extern int GetSystemMetrics(int i);
    [DllImport("user32.dll")] public static extern bool SetWindowPos(IntPtr h, IntPtr after, int x, int y, int cx, int cy, uint flags);
    public delegate bool EnumProc(IntPtr h, IntPtr l);
    [DllImport("user32.dll")] static extern bool EnumWindows(EnumProc f, IntPtr l);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr h, out uint pid);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr h);
    [DllImport("user32.dll")] static extern bool IsIconic(IntPtr h);
    // Every visible window of the process (not a minimized one, nor a tiny helper) to (x, y), its
    // size kept; how many there are. By the windows themselves, not Process.MainWindowHandle. For a
    // client that doesn't read WOW_WINDOW_AT: its window shows where Windows puts it, and moves
    // once the client answers (about a second and a half in).
    public static int MoveAll(int pid, int x, int y) {
        int n = 0;
        EnumWindows(delegate (IntPtr h, IntPtr l) {
            uint wp; GetWindowThreadProcessId(h, out wp);
            RECT r;
            if (wp != (uint)pid || !IsWindowVisible(h) || IsIconic(h) || !GetWindowRect(h, out r) || r.Right - r.Left < 100) return true;
            n++;
            // SWP_NOSIZE 0x1 | SWP_NOZORDER 0x4 | SWP_NOACTIVATE 0x10
            if (r.Left != x || r.Top != y) SetWindowPos(h, IntPtr.Zero, x, y, 0, 0, 0x15);
            return true;
        }, IntPtr.Zero);
        return n;
    }
}

// The game's sound muted in Windows' mixer (its audio sessions on every playback device), as a
// player would from the volume mixer: benilla has no setting of its own for it.
public static class VisualCheckAudio {
    [ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")] class MMDeviceEnumerator {}
    [ComImport, Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDeviceEnumerator { int EnumAudioEndpoints(int flow, int state, out IMMDeviceCollection c); }
    [ComImport, Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDeviceCollection { int GetCount(out int n); int Item(int i, out IMMDevice d); }
    [ComImport, Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IMMDevice { int Activate(ref Guid iid, int ctx, IntPtr p, [MarshalAs(UnmanagedType.IUnknown)] out object o); }
    [ComImport, Guid("77AA99A0-1BD6-484F-8BC7-2C654C9A9B6F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionManager2 {
        int GetAudioSessionControl(IntPtr a, int b, out IntPtr c);
        int GetSimpleAudioVolume(IntPtr a, int b, out IntPtr c);
        int GetSessionEnumerator(out IAudioSessionEnumerator e);
    }
    [ComImport, Guid("E2F5BB11-0570-40CA-ACDD-3AA01277DEE8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionEnumerator { int GetCount(out int n); int GetSession(int i, out IAudioSessionControl2 s); }
    [ComImport, Guid("bfb7ff88-7239-4fc9-8fa2-07c950be9c6d"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface IAudioSessionControl2 {
        int GetState(out int s); int GetDisplayName(out IntPtr p); int SetDisplayName(IntPtr a, IntPtr b);
        int GetIconPath(out IntPtr p); int SetIconPath(IntPtr a, IntPtr b); int GetGroupingParam(out Guid g);
        int SetGroupingParam(IntPtr a, IntPtr b); int RegisterAudioSessionNotification(IntPtr a);
        int UnregisterAudioSessionNotification(IntPtr a);
        int GetSessionIdentifier(out IntPtr p); int GetSessionInstanceIdentifier(out IntPtr p); int GetProcessId(out uint pid);
    }
    [ComImport, Guid("87CE5498-68D6-44E5-9215-6DA47EF883D8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
    interface ISimpleAudioVolume {
        int SetMasterVolume(float v, ref Guid ctx); int GetMasterVolume(out float v);
        int SetMute([MarshalAs(UnmanagedType.Bool)] bool m, ref Guid ctx); int GetMute([MarshalAs(UnmanagedType.Bool)] out bool m);
    }
    // The process's sessions found (each muted now if it wasn't).
    public static int Mute(int pid) { int muted; return Sessions(pid, true, out muted); }
    // The process's sessions, and how many of them are muted (muting them first if `mute`).
    public static int Sessions(int pid, bool mute, out int muted) {
        int n = 0; muted = 0;
        IMMDeviceCollection devs;
        if (((IMMDeviceEnumerator)new MMDeviceEnumerator()).EnumAudioEndpoints(0, 1, out devs) != 0) return 0;
        int count; devs.GetCount(out count);
        Guid iid = typeof(IAudioSessionManager2).GUID;
        for (int i = 0; i < count; i++) {
            IMMDevice d; object o; IAudioSessionEnumerator se; int sc;
            if (devs.Item(i, out d) != 0 || d.Activate(ref iid, 23, IntPtr.Zero, out o) != 0) continue;
            if (((IAudioSessionManager2)o).GetSessionEnumerator(out se) != 0) continue;
            se.GetCount(out sc);
            for (int k = 0; k < sc; k++) {
                IAudioSessionControl2 c; uint sp;
                if (se.GetSession(k, out c) != 0 || c.GetProcessId(out sp) != 0 || sp != (uint)pid) continue;
                var vol = (ISimpleAudioVolume)c;
                bool m; vol.GetMute(out m);
                Guid g = Guid.Empty;
                if (!m && mute) { vol.SetMute(true, ref g); vol.GetMute(out m); }
                if (m) muted++;
                n++;
            }
        }
        return n;
    }
}
"@
[void][VisualCheckWin]::SetProcessDPIAware()
Add-Type -AssemblyName System.Windows.Forms  # after DPI-aware, so Screen's bounds are real pixels

# The screens in the order the `monitor` setting counts them: the primary is 1, then the others as
# Windows lists them.
function Get-Monitors {
    $all = @([System.Windows.Forms.Screen]::AllScreens)
    @($all | Where-Object { $_.Primary }) + @($all | Where-Object { -not $_.Primary })
}

# The screen the `monitor` setting names (a number, or DISPLAYn), or $null for "wherever Windows
# opens it" (empty, 0) and, with a word of warning, for one this PC doesn't have.
function Get-WantedMonitor($setting) {
    $setting = ([string]$setting).Trim()
    if (-not $setting -or $setting -eq "0") { return $null }
    $screens = @(Get-Monitors)
    if ($setting -match '^\d+$') {
        $n = [int]$setting
        if ($n -ge 1 -and $n -le $screens.Count) { return $screens[$n - 1] }
    } else {
        $name = $setting.TrimStart([char]92, '.').ToUpper()
        $hit = @($screens | Where-Object { $_.DeviceName.TrimStart([char]92, '.').ToUpper() -eq $name })
        if ($hit.Count) { return $hit[0] }
    }
    Say "  monitor = $($setting): this PC has $($screens.Count) screen(s) ($(($screens | ForEach-Object { $_.DeviceName.TrimStart([char]92, '.') }) -join ', ')); leaving the window where Windows opens it."
    return $null
}

# ddagrab's output_idx for a screen: DXGI lists the outputs in an order of its own, so each index is
# opened once for its size, and the one the size matches is it (equal sizes: Windows' order among
# them). Cached for the run; $null when no index matches.
$OutputSizes = $null
function Get-OutputIdx([string]$ffmpeg, $screen) {
    $ErrorActionPreference = "Continue"  # ffmpeg's report is on stderr
    if (-not $script:OutputSizes) {
        $script:OutputSizes = @()
        for ($i = 0; $i -lt 8; $i++) {
            $out = & $ffmpeg -hide_banner -nostdin -loglevel info -f lavfi -i "ddagrab=output_idx=$($i):framerate=5" -frames:v 1 -f null - 2>&1 | Out-String
            if ($out -match 'Video: wrapped_avframe, d3d11, (\d+)x(\d+)') { $script:OutputSizes += ,@([int]$Matches[1], [int]$Matches[2]) } else { break }
        }
    }
    $same = @([System.Windows.Forms.Screen]::AllScreens | Where-Object { $_.Bounds.Width -eq $screen.Bounds.Width -and $_.Bounds.Height -eq $screen.Bounds.Height })
    $nth = [Array]::IndexOf(@($same | ForEach-Object { $_.DeviceName }), $screen.DeviceName)
    $seen = 0
    for ($i = 0; $i -lt $script:OutputSizes.Count; $i++) {
        if ($script:OutputSizes[$i][0] -eq $screen.Bounds.Width -and $script:OutputSizes[$i][1] -eq $screen.Bounds.Height) {
            if ($seen -eq [Math]::Max(0, $nth)) { return $i }
            $seen++
        }
    }
    return $null
}

# ffmpeg recording the client's window ("benilla", the title benilla's lib.rs gives it) for
# $v.Duration seconds into $v.Out. `screen` (the default) takes the window's picture (inside its
# title bar and borders) by Desktop Duplication (ffmpeg's ddagrab), which sees every frame a Vulkan window presents; GDI's grabs
# (`title`, the window; `desktop`, the screen's rectangle where it is) see only a few stale ones of
# benilla's window (the first run on Corey's laptop: 3 different frames in 289).
function Start-Video($v, $client, [string]$how) {
    $grab = @("-f", "gdigrab", "-framerate", "30")
    $vf = "scale=trunc(iw/2)*2:trunc(ih/2)*2"
    if ($how -eq "screen") {
        $client.Refresh()
        $r = New-Object VisualCheckWin+RECT
        if (-not [VisualCheckWin]::GetPictureRect($client.MainWindowHandle, [ref]$r)) { return $null }
        # The screen the window is on: its rectangle is cut to that screen and measured from that
        # screen's own top-left, which is what ddagrab's offsets count from.
        $scr = [System.Windows.Forms.Screen]::FromHandle($client.MainWindowHandle)
        $b = $scr.Bounds
        $idx = Get-OutputIdx $v.Ffmpeg $scr
        if ($null -eq $idx) { return $null }
        $l = [Math]::Max($b.Left, $r.Left); $t = [Math]::Max($b.Top, $r.Top)
        $w = [Math]::Min($b.Right, $r.Right) - $l; $h = [Math]::Min($b.Bottom, $r.Bottom) - $t
        $w -= $w % 2; $h -= $h % 2
        if ($w -lt 64 -or $h -lt 64) { return $null }
        $l -= $b.Left; $t -= $b.Top
        $grab = @("-f", "lavfi", "-i", "ddagrab=output_idx=$($idx):framerate=30:draw_mouse=0:offset_x=$($l):offset_y=$($t):video_size=$($w)x$($h)")
        $vf = "hwdownload,format=bgra," + $vf
    } elseif ($how -eq "desktop") {
        $client.Refresh()
        $r = New-Object VisualCheckWin+RECT
        if (-not [VisualCheckWin]::GetPictureRect($client.MainWindowHandle, [ref]$r)) { return $null }
        $w = ($r.Right - $r.Left) - (($r.Right - $r.Left) % 2)
        $h = ($r.Bottom - $r.Top) - (($r.Bottom - $r.Top) % 2)
        $grab += @("-offset_x", "$($r.Left)", "-offset_y", "$($r.Top)", "-video_size", "$($w)x$($h)", "-i", "desktop")
    } else {
        $grab += @("-i", "title=benilla")
    }
    $all = @("-hide_banner", "-nostdin", "-loglevel", "warning", "-y") + $grab + @(
        "-t", (Fmt $v.Duration), "-vf", $vf,
        "-c:v", "libx264", "-preset", "veryfast", "-crf", "23", "-pix_fmt", "yuv420p", $v.Out)
    $v.How = $how
    $v.Started = Get-Date
    $rec = Start-Process -FilePath $v.Ffmpeg -ArgumentList (($all | ForEach-Object { Quote $_ }) -join " ") `
        -NoNewWindow -PassThru -RedirectStandardError "$($v.Out).log"
    $null = $rec.Handle  # held now, or Windows PowerShell never learns its exit code
    $rec
}

# --- running the client --------------------------------------------------------------------------
# Starts the client with these variables (and the run's own: the server, the account, the settings
# folder $chome, the time limit), its output in $log.out and $log.err. Its window is made at $at
# (a "x,y" desktop position, for a client that reads WOW_WINDOW_AT), else at the top-left of the
# `monitor` setting's screen.
function Start-Client($s, [hashtable]$vars, [int]$timeout, $chome, $log, $at = $null) {
    $common = @{
        WOW_DATA = $s.data; WOW_HOST = $(if ($s.port -and [string]$s.port -ne "3724") { "$($s.host):$($s.port)" } else { $s.host }); WOW_REALM = $s.realm
        WOW_USER = $s.account; WOW_PASS = $s.password
        BENILLA_HOME = $chome
        # Nobody at the keyboard: a refused login ends the run instead of waiting on a dialog;
        # WOW_BG=0 keeps the full-size window a run with only WOW_PROBE variables would shrink.
        WOW_UNATTENDED = "1"; WOW_BG = "0"
        WOW_PROBE_EXIT_AT = [string]($timeout - 5)
    }
    # The window made on its monitor from the start (a client that reads WOW_WINDOW_AT).
    $screen = Get-WantedMonitor $s.monitor
    if ($at) { $common["WOW_WINDOW_AT"] = $at }
    elseif ($screen) { $common["WOW_WINDOW_AT"] = "$($screen.WorkingArea.X),$($screen.WorkingArea.Y)" }
    foreach ($k in $vars.Keys) { $common[$k] = $vars[$k] }
    $saved = @{}
    foreach ($k in $common.Keys) {
        $saved[$k] = [Environment]::GetEnvironmentVariable($k, "Process")
        [Environment]::SetEnvironmentVariable($k, [string]$common[$k], "Process")
    }
    try {
        Start-Process -FilePath $s.client -WorkingDirectory $chome -PassThru `
            -RedirectStandardOutput "$log.out" -RedirectStandardError "$log.err"
    } finally {
        foreach ($k in $saved.Keys) { [Environment]::SetEnvironmentVariable($k, $saved[$k], "Process") }
    }
}

# Where a client is kept until its video starts: $at (a "x,y" string), else the top-left of the
# `monitor` setting's screen; $null to leave it where Windows puts it.
function Get-WindowSpot($s, $at) {
    if ($at) { $xy = $at -split ','; return @([int]$xy[0], [int]$xy[1]) }
    $screen = Get-WantedMonitor $s.monitor
    if ($screen) { return @($screen.WorkingArea.X, $screen.WorkingArea.Y) }
    $null
}

# A client's output, its two halves, into $log.
function Close-ClientLog($log) {
    $parts = @("$log.out", "$log.err") | Where-Object { Test-Path $_ }
    if ($parts) { Get-Content $parts -ErrorAction SilentlyContinue | Set-Content $log }
    Remove-Item $parts -ErrorAction SilentlyContinue
}

# Runs the client with these variables until it quits or the time is up (then it's closed), its
# output in $log. With $v (a video), recording starts when the cue shot lands in the screenshots
# folder. Returns whether the client quit by itself. With $film (the cameraman's client, already
# running) the video records that client's window instead, which is kept on its screen and muted
# meanwhile too; $at is where this client's window goes then (beside it).
function Invoke-Client($s, [hashtable]$vars, [int]$timeout, $chome, $log, $v = $null, $at = $null, $film = $null) {
    Stop-Strays $s $(if ($film) { $film.Id } else { 0 })
    $spot = Get-WindowSpot $s $at
    $filmSpot = Get-WindowSpot $s $null
    $p = $null
    $rec = $null
    $shotsDir = Join-Path $chome "Screenshots"
    $muteNext = Get-Date
    # The outputs ddagrab sees are looked at now, not when the video is due.
    if ($v -and $s.video -eq "screen" -and $s.ffmpeg) { $null = Get-OutputIdx $s.ffmpeg ([System.Windows.Forms.Screen]::PrimaryScreen) }
    try {
        $p = Start-Client $s $vars $timeout $chome $log $at
        $deadline = (Get-Date).AddSeconds($timeout)
        while (-not $p.HasExited -and (Get-Date) -lt $deadline) {
            # The window goes to its spot the moment it exists, and is put back if the client
            # moves it, until the video starts (its offsets are taken then).
            if ($spot -and -not $rec) {
                [void][VisualCheckWin]::MoveAll($p.Id, $spot[0], $spot[1])
                if ($film -and $filmSpot) { [void][VisualCheckWin]::MoveAll($film.Id, $filmSpot[0], $filmSpot[1]) }
            }
            # Its sound muted as soon as it has any, and kept muted (a new device, a new session).
            if ($s.mute -ne "0" -and ((Get-Date) -ge $muteNext)) {
                try {
                    $n = [VisualCheckAudio]::Mute($p.Id)
                    if ($film) { $n += [VisualCheckAudio]::Mute($film.Id) }
                    if ($n -gt 0) { $muteNext = (Get-Date).AddSeconds(1) }
                } catch { $muteNext = (Get-Date).AddSeconds(1) }
            }
            if ($v -and -not $rec -and (Test-Path $shotsDir)) {
                $cue = Get-ChildItem $shotsDir -Filter *.png | Sort-Object CreationTime | Select-Object -First 1
                if ($cue) {
                    $v.CueAt = $cue.CreationTime
                    $rec = Start-Video $v $(if ($film) { $film } else { $p }) $s.video
                    if (-not $rec) { $v.Status = "no video: the game window wasn't found" }
                }
            }
            # Desktop Duplication refused (an ffmpeg without it, a locked screen), or the window's
            # title didn't match: once more, the next way down.
            if ($rec -and $rec.HasExited -and $null -ne $rec.ExitCode -and $rec.ExitCode -ne 0 -and $v.How -ne "desktop" -and ((Get-Date) - $v.Started).TotalSeconds -lt 5) {
                $v.Duration = [Math]::Max(1.0, $v.Duration - ((Get-Date) - $v.Started).TotalSeconds)
                $rec = Start-Video $v $(if ($film) { $film } else { $p }) $(if ($v.How -eq "screen") { "title" } else { "desktop" })
            }
            # Quick while the window is still to be caught.
            Start-Sleep -Milliseconds $(if ($rec -or -not $spot) { 100 } else { 15 })
        }
        return $p.HasExited
    } finally {
        if ($p -and -not $p.HasExited) {
            Say "  the client ran past $timeout s: closing it"
            Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
            $p.WaitForExit(10000) | Out-Null
        }
        if ($rec) {
            if ($rec.WaitForExit(15000)) {
                # ffmpeg stops once it has the video's length: its first frame was that long
                # before it quit (it starts some tenths of a second after it's asked to).
                $v.First = $rec.ExitTime.AddSeconds(-$v.Duration)
            } else { Stop-Process -Id $rec.Id -Force -ErrorAction SilentlyContinue }
            if ((Test-Path $v.Out) -and (Get-Item $v.Out).Length -gt 0) {
                $v.Status = "video: $($v.Out) ($($v.How))"
                Remove-Item "$($v.Out).log" -ErrorAction SilentlyContinue
            } else {
                $v.Status = "no video: ffmpeg wrote nothing (see $($v.Out).log)"
            }
        } elseif ($v -and -not $v.Status) {
            $v.Status = "no video: the scenario's first shot never came"
        }
        Close-ClientLog $log
    }
}

# --- the cameraman -------------------------------------------------------------------------------
# A scenario with a `camera` line is filmed by a second client, the cameraman (docs/visual-check.md,
# "Filming a boss"): its hero says `.es watch <hero>` (the server keeps it out of every rule,
# unseen, with the hero wherever they go) and benilla's WOW_SPECTATE holds its camera on the
# subject and the hero. Its window is the one the video records; its picture is clean (no
# interface, names or floor marks); its sound is its own mix (WOW_MIX_TAP), written before its
# output, which is silent while its window isn't focused and muted in Windows' mixer anyway. The
# hero's own client is beside it, smaller, and opens no sound device at all.

# The cameraman's driver: the picture cleared every frame, then `.es watch` the hero.
function Build-CameraLua($hero) {
    @"
if VisualCheck then return end
local VC = { i = 1, steps = { { 0.5, ".es debug quiet" }, { 1.0, ".es watch $hero" } } }
VisualCheck = VC
local log = ProbeLog or function() end
VC.t0 = GetTime()
log("visual-check: the cameraman is in the world")
SetCVar("UnitNameNPC", "0")
SetCVar("UnitNamePlayer", "0")
SetCVar("UnitNameOwn", "0")
ElwynnSurvivalSettings = ElwynnSurvivalSettings or {}
ElwynnSurvivalSettings.noMarks = 1
local ev = CreateFrame("Frame")
ev:RegisterEvent("CHAT_MSG_SYSTEM")
ev:RegisterEvent("UI_ERROR_MESSAGE")
ev:SetScript("OnEvent", function() log("visual-check: server: " .. tostring(arg1)) end)
local frame = CreateFrame("Frame")
frame:SetScript("OnUpdate", function()
  if UIParent:IsShown() then UIParent:Hide() end
  if ElwynnSurvival and ElwynnSurvival.ClearMarks then ElwynnSurvival.ClearMarks() end
  local now = GetTime() - VC.t0
  while VC.steps[VC.i] and VC.steps[VC.i][1] <= now do
    log("visual-check: t=" .. string.format("%.2f", now) .. " say " .. VC.steps[VC.i][2])
    SendChatMessage(VC.steps[VC.i][2], "SAY")
    VC.i = VC.i + 1
  end
end)
"@
}

# The cameraman's client, started and in the world watching $hero (or a throw that says why not):
# @{ P = its process; Log; Wav; At = where the hero's window goes, beside it }.
function Start-Cameraman($s, $sc, $hero, $out, [int]$timeout) {
    $name = $s.cameraman
    if (-not $name) { throw "$($sc.Name) has a camera, and the settings name no cameraman (set cameraman = <name>; 'setup' makes it)." }
    $chome = Join-Path $out "camera"
    New-ClientHome $s $chome $s.resolution
    $log = Join-Path $out "camera.log"
    $wav = Join-Path $out "sound.wav"
    $vars = @{
        WOW_CHAR = $name; WOW_PROBE_LUA = (Build-CameraLua $hero); WOW_PROBE_LUA_AT = "1"
        WOW_SPECTATE = "$($sc.Camera.Subject),$hero,$($sc.Camera.Shot)"; WOW_MIX_TAP = $wav
    }
    Say "  the cameraman $name films $(if ($sc.Camera.Subject) { "$($sc.Camera.Subject) and " })$hero ($($sc.Camera.Shot))"
    $p = Start-Client $s $vars $timeout $chome $log
    $null = $p.Handle  # held now, or Windows PowerShell never learns its exit code
    $spot = Get-WindowSpot $s $null
    $deadline = (Get-Date).AddSeconds([int](Num $s.login_timeout))
    $watching = $false
    while (-not $p.HasExited -and -not $watching -and (Get-Date) -lt $deadline) {
        if ($spot) { [void][VisualCheckWin]::MoveAll($p.Id, $spot[0], $spot[1]) }
        if ($s.mute -ne "0") { try { [void][VisualCheckAudio]::Mute($p.Id) } catch { } }
        $said = @("$log.out", "$log.err") | Where-Object { Test-Path $_ }
        $watching = $said -and (Select-String -Path $said -Pattern "You're watching" -SimpleMatch -Quiet)
        Start-Sleep -Milliseconds 15
    }
    $p.Refresh()
    $fits = $false
    if ($watching) {
        # The hero's window beside the cameraman's, on the same screen.
        $r = New-Object VisualCheckWin+RECT
        [void][VisualCheckWin]::GetWindowRect($p.MainWindowHandle, [ref]$r)
        $area = [System.Windows.Forms.Screen]::FromHandle($p.MainWindowHandle).WorkingArea
        $w = [int](($s.hero_resolution -split 'x')[0])
        $border = ($r.Right - $r.Left) - [int](($s.resolution -split 'x')[0])
        $x = $r.Right + 8
        $fits = $x + $w + $border -le $area.Right
    }
    if (-not $fits) {
        Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue
        $p.WaitForExit(10000) | Out-Null
        Close-ClientLog $log
        if (-not $watching) { throw "the cameraman $name never got to watching $hero (see camera.log: the login, or the server's answer to .es watch)" }
        throw "the hero's window ($($s.hero_resolution)) doesn't fit beside the cameraman's on its screen: make resolution or hero_resolution smaller"
    }
    @{ P = $p; Log = $log; Wav = $wav; At = "$x,$($r.Top)" }
}

# When the video's first frame was taken: from when ffmpeg quit, or (killed) when it was started.
function Get-VideoStart($v) {
    if ($v.First -and $v.First -ge $v.Started) { return $v.First }
    $v.Started
}

# The cameraman's sound under the video: the mix it wrote from its start, cut to where the video
# starts (to about a tenth of a second). What it says goes in run.txt.
function Add-Sound($s, $v, $wav) {
    if (-not (Test-Path $v.Out)) { return "no sound: no video" }
    if (-not (Test-Path $wav) -or (Get-Item $wav).Length -le 44) { return "no sound: the cameraman's mix wasn't written" }
    $skip = [Math]::Max(0.0, ((Get-VideoStart $v) - (Get-Item $wav).CreationTime).TotalSeconds)
    $tmp = "$($v.Out).sound.mp4"
    $ErrorActionPreference = "Continue"
    & $s.ffmpeg -hide_banner -nostdin -loglevel error -y -i $v.Out -ss (Fmt $skip) -i $wav -map 0:v -map 1:a -c:v copy -c:a aac -b:a 192k -shortest $tmp 2>&1 | Out-Null
    if ((Test-Path $tmp) -and (Get-Item $tmp).Length -gt 0) {
        Move-Item -Force $tmp $v.Out
        Remove-Item $wav -ErrorAction SilentlyContinue
        return "sound: the cameraman's own mix"
    }
    "no sound: ffmpeg couldn't put $wav under the video"
}

# Stills from the cameraman's video at the scenario's bursts' times, named as the client's would
# be (<label>-NN): the video's first frame is the cue still's time plus what passed between them.
function Save-FilmedShots($s, $sc, $v, $to) {
    if (-not (Test-Path $v.Out) -or -not $v.CueAt) { return 0 }
    $zero = $sc.Video + ((Get-VideoStart $v) - $v.CueAt).TotalSeconds
    $ext = if ($s.format -eq "png") { "png" } else { "jpg" }
    $saved = 0
    $ErrorActionPreference = "Continue"
    foreach ($st in @($sc.Steps | Where-Object { $_.Kind -eq "shots" })) {
        for ($k = 0; $k -lt $st.Count; $k++) {
            $at = $st.T + $k * $st.Every - $zero
            if ($at -lt 0) { continue }
            $file = Join-Path $to ("{0}-{1:D2}.{2}" -f $st.Text, $k, $ext)
            & $s.ffmpeg -hide_banner -nostdin -loglevel error -y -ss (Fmt $at) -i $v.Out -frames:v 1 -vf "scale='min($([int]$s.width),iw)':-2" -q:v 3 $file 2>&1 | Out-Null
            if (Test-Path $file) { $saved++ }
        }
    }
    $saved
}

# A settings folder of the run's own: the bundled config at the chosen window size and this
# checkout's add-ons, so the add-on under test is the branch's and no player setting is touched.
function New-ClientHome($s, $dir, $resolution = $null) {
    if (Test-Path $dir) { Remove-Item -Recurse -Force $dir }
    New-Item -ItemType Directory -Force -Path (Join-Path $dir "AddOns") | Out-Null
    $config = @(Get-Content (Join-Path $Root "dist\windows\benilla-config\config.toml"))
    $config += "gxWindow = `"1`""
    $config += "gxResolution = `"$(if ($resolution) { $resolution } else { $s.resolution })`""
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
    New-ClientHome $s $chome $(if ($sc.Camera) { $s.hero_resolution } else { $s.resolution })
    $settle = Num $s.settle
    $v = $null
    $videoNote = ""
    if ($s.video -eq "off") { $videoNote = "no video: video = off in the settings" }
    elseif (-not $s.ffmpeg) { $videoNote = "no video: no ffmpeg.exe in $(Join-Path $Folder 'ffmpeg') (or the ffmpeg setting)" }
    else { $v = @{ Ffmpeg = $s.ffmpeg; Out = (Join-Path $out "$name.mp4"); Duration = [Math]::Max(1.0, $sc.End - $sc.Video); Status = "" } }
    # With a cameraman its video gives the stills: the hero's client takes only the video's cue.
    $played = $sc
    if ($sc.Camera) { $played = $sc.Clone(); $played.Steps = @($sc.Steps | Where-Object { $_.Kind -ne "shots" }) }
    $lua = Build-Lua $played $settle $(if ($v) { $sc.Video } else { $null })
    if ($lua.Length -gt 30000) { throw "$name is too long to hand the client ($($lua.Length) characters of Lua; 30000 at most)." }
    $asked = 0
    foreach ($st in $sc.Steps) { if ($st.Kind -eq "shots") { $asked += $st.Count } }
    $timeout = [int]((Num $s.login_timeout) + $settle + $sc.End + 30)
    Say "$name : $hero, $([int]$sc.End) s, $asked stills (closed after $timeout s at most)"
    $log = Join-Path $out "client.log"
    $vars = @{ WOW_CHAR = $hero; WOW_PROBE_LUA = $lua; WOW_PROBE_LUA_AT = "1" }
    if ($cam -ne "off") { $vars.WOW_PROBE_CAM = $cam }
    # A cost run (a step that turns fpsJournal on) has the client write its frame-time journal
    # straight into the run's folder: take 280 found it in the client's own folder, out of reach.
    # It then covers the whole run; the stills' times tell the steps apart.
    if (@($sc.Steps | Where-Object { $_.Kind -eq "lua" -and $_.Text -match 'fpsJournal' }).Count -gt 0) {
        $vars.WOW_FPS_JOURNAL = Join-Path $out "fps-journal.csv"
    }
    $film = $null
    if ($sc.Camera) {
        Stop-Strays $s
        $film = Start-Cameraman $s $sc $hero $out ($timeout + [int](Num $s.login_timeout))
        # The cameraman's client is the one heard; the hero's opens no sound device.
        $vars.WOW_NOSOUND = "1"
    }
    try {
        $quit = Invoke-Client $s $vars $timeout $chome $log $v $(if ($film) { $film.At }) $(if ($film) { $film.P })
    } finally {
        if ($film) {
            if (-not $film.P.HasExited) {
                Stop-Process -Id $film.P.Id -Force -ErrorAction SilentlyContinue
                $film.P.WaitForExit(10000) | Out-Null
            }
            Close-ClientLog $film.Log
            Remove-Item -Recurse -Force (Join-Path $out "camera") -ErrorAction SilentlyContinue
        }
    }
    if ($film -and $v) { $v.Status = "$($v.Status); $(Add-Sound $s $v $film.Wav)" }
    $shots = if ($film) { if ($v) { Save-FilmedShots $s $sc $v $out } else { 0 } } else { Save-Shots $s (Join-Path $chome "Screenshots") $out $log ($null -ne $v) }
    $runLog = @()
    if (Test-Path $log) {
        $runLog = @(Select-String -Path $log -Pattern 'visual-check: (.*)$' | ForEach-Object { $_.Matches[0].Groups[1].Value })
    }
    # What the cameraman did: its own lines, and where its camera went.
    if ($film -and (Test-Path $film.Log)) {
        $runLog += @(Select-String -Path $film.Log -Pattern '(?:probe-log: visual-check: |\bspectate: )(.*)$' | ForEach-Object { "camera: $($_.Matches[0].Groups[1].Value)" })
    }
    if ($v) { $videoNote = $v.Status }
    $runLog += $videoNote
    Set-Content (Join-Path $out "run.txt") $runLog
    # A journal written into the run's own client home instead (an older client, with no
    # WOW_FPS_JOURNAL), which goes next: keep it beside the stills, unless one is already there.
    foreach ($journal in @((Join-Path $chome "Diagnostics\fps-journal.csv"), (Join-Path $chome "benilla-config\Diagnostics\fps-journal.csv"))) {
        if ((Test-Path $journal) -and -not (Test-Path (Join-Path $out "fps-journal.csv"))) { Copy-Item $journal (Join-Path $out "fps-journal.csv") -ErrorAction SilentlyContinue }
    }
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
$HeroBodies = @{ hunter = "3,3,0"; warrior = "1,1,0"; cameraman = "1,1,0" }  # race, class, gender: a dwarf hunter, two human warriors
function Invoke-Setup($s, $out) {
    if (Test-Path $out) { Remove-Item -Recurse -Force $out }
    New-Item -ItemType Directory -Force -Path $out | Out-Null
    $results = @()
    foreach ($role in @("hunter", "warrior", "cameraman")) {
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
    # `client <name>`: this request runs on clients\<name>\ (Invoke-ClientDeploy), not the default.
    $use = $names | Where-Object { $_ -match '^client\s+\S+$' } | Select-Object -First 1
    $names = @($names | Where-Object { $_ -notmatch '^client\s+\S+$' })
    Write-WatchLog "request $id : $($names -join ' ')$(if ($use) { " ($use)" })"
    $report = @()
    $failed = $false
    try {
        $s = Resolve-Settings $false
        if ($use) {
            $cname = ($use -split '\s+')[1]
            $exe = Get-ChildItem (Join-Path $Folder "clients\$cname") -Filter benilla.exe -File -Recurse -ErrorAction SilentlyContinue | Select-Object -First 1
            if (-not $exe) { throw "No client $cname in $(Join-Path $Folder 'clients')." }
            $s.client = $exe.FullName
        }
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

# Data questions, with nobody at the PC: a file in queries\ of lines `<tool> <args...>`, each run
# as one of the repository's read-only data tools against the WoW data (`cargo run -p htl-map
# --example <tool> -- <Data> <args...>`: spellfx, creaturedisp, displays, kitanims...) to find a
# look to borrow. The answers go to out\<id>.queries.txt (400 lines a question at most). Nothing
# in the game, the data or the repository changes.
function Invoke-Queries($file) {
    $id = [IO.Path]::GetFileNameWithoutExtension($file.Name)
    $outFile = Join-Path $Folder "out\$id.queries.txt"
    Write-WatchLog "queries $id"
    $done = Join-Path $Folder "queries\done"
    New-Item -ItemType Directory -Force -Path $done | Out-Null
    $lines = @(Get-Content $file.FullName)
    Move-Item $file.FullName (Join-Path $done $file.Name) -Force
    "" | Set-Content $outFile
    try {
        $s = Resolve-Settings $false
        Push-Location $Root
        . (Join-Path $Root "scripts\windows-rust.ps1")
        foreach ($line in $lines) {
            $line = $line.Trim()
            if ($line -eq "" -or $line.StartsWith("#")) { continue }
            $parts = $line -split '\s+'
            if ($parts[0] -notmatch '^[a-z0-9_]+$') { "== $line`r`n(not a tool name)" | Add-Content $outFile; continue }
            $rest = @(); if ($parts.Length -gt 1) { $rest = $parts[1..($parts.Length - 1)] }
            "== $line" | Add-Content $outFile
            & cargo run --release -q -p htl-map --example $parts[0] -- $s.data @rest 2>&1 | Select-Object -First 400 | ForEach-Object { "$_" } | Add-Content $outFile
        }
        "== done" | Add-Content $outFile
        Write-WatchLog "queries $id done"
    } catch {
        "== failed: $($_.Exception.Message)" | Add-Content $outFile
        Write-WatchLog "queries $id failed: $($_.Exception.Message)"
    } finally {
        Pop-Location -ErrorAction SilentlyContinue
    }
}

# Another dev client to check with (a branch's client patches), with nobody at the PC: its zip in
# parts in deploy\clients\<name>\ (part-0, part-1, ...), then <name>.sha256 last, holding the
# whole zip's SHA-256. The parts are joined, checked and unpacked into clients\<name>\; a request
# that says `client <name>` runs on it. The default client (settings) is left as it is.
function Invoke-ClientDeploy($dir) {
    $name = $dir.Name
    $hashFile = Join-Path $dir.FullName "$name.sha256"
    $want = (Get-Content $hashFile -Raw).Trim().ToLower()
    $parts = @(Get-ChildItem $dir.FullName -Filter "part-*" -File | Sort-Object { [int]($_.Name -replace '^part-', '') })
    $zip = Join-Path $dir.FullName "$name.zip"
    $out = [System.IO.File]::Create($zip)
    try {
        foreach ($p in $parts) { $b = [System.IO.File]::ReadAllBytes($p.FullName); $out.Write($b, 0, $b.Length) }
    } finally { $out.Close() }
    $got = (Get-FileHash $zip -Algorithm SHA256).Hash.ToLower()
    if ($got -ne $want) {
        Write-WatchLog "client $name : refused, the parts make $got, not $want"
        Rename-Item $hashFile "$name.sha256.refused"
        Remove-Item $zip -Force
        return
    }
    $dest = Join-Path $Folder "clients\$name"
    if (Test-Path $dest) { Remove-Item $dest -Recurse -Force }
    Expand-Archive -Path $zip -DestinationPath $dest -Force
    Remove-Item $dir.FullName -Recurse -Force
    $exe = Get-ChildItem $dest -Filter benilla.exe -File -Recurse | Select-Object -First 1
    Write-WatchLog "client $name : unpacked into $dest$(if ($exe) { " ($($exe.FullName))" } else { ' (no benilla.exe in it)' })"
}

function Start-Watcher {
    $requests = Join-Path $Folder "requests"
    foreach ($d in @($requests, (Join-Path $Folder "out"), (Join-Path $Folder "scenarios"), (Join-Path $Folder "deploy"), (Join-Path $Folder "queries"))) {
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
            # A client to check with, once its last file (the hash) has sat for 5 s.
            $cl = Get-ChildItem (Join-Path $Folder "deploy\clients") -Directory -ErrorAction SilentlyContinue |
                Where-Object { $h = Get-Item (Join-Path $_.FullName "$($_.Name).sha256") -ErrorAction SilentlyContinue; $h -and $h.LastWriteTime -lt (Get-Date).AddSeconds(-5) } |
                Select-Object -First 1
            if ($cl) {
                try { Invoke-ClientDeploy $cl } catch { Write-WatchLog "client $($cl.Name): $($_.Exception.Message)" }
                continue
            }
            # Data questions next, then scenarios.
            $q = Get-ChildItem (Join-Path $Folder "queries") -Filter *.txt -File -ErrorAction SilentlyContinue |
                Where-Object { $_.LastWriteTime -lt (Get-Date).AddSeconds(-2) } | Sort-Object LastWriteTime | Select-Object -First 1
            if ($q) {
                try { Invoke-Queries $q } catch { Write-WatchLog "queries $($q.Name): $($_.Exception.Message)" }
                continue
            }
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
