# What Play.exe runs (with no console window): the launcher. It shows the project's shield and
# what it's doing, updates this folder if there's a new version, finds your WoW 1.12.1, then
# starts the game where you played last time (in 3 seconds, or press Change to choose again).
#   play.ps1 -Change      straight to the choices
#   play.ps1 -NoUpdate    don't look for a new version this time
# What it did is kept in launcher\play.log, for when something goes wrong.
param([switch]$Change, [switch]$NoUpdate, [string]$Game = "")
. (Join-Path $PSScriptRoot "common.ps1")
Add-Type -AssemblyName System.Windows.Forms, System.Drawing
[System.Windows.Forms.Application]::EnableVisualStyles()

$LogFile = Join-Path $Launcher "play.log"
try { Set-Content -Path $LogFile -Value ("{0:yyyy-MM-dd HH:mm:ss} Play, {1}" -f (Get-Date), (Get-InstalledVersion)) } catch { }
function Log($t) { try { Add-Content -Path $LogFile -Value ("{0:HH:mm:ss} {1}" -f (Get-Date), $t) } catch { } }

# --- the window --------------------------------------------------------------------------------------
function Rgb($r, $g, $b) { [System.Drawing.Color]::FromArgb($r, $g, $b) }
$Gold = Rgb 232 193 90
$Ink = Rgb 232 222 202
$Dim = Rgb 150 140 120
$Night = Rgb 24 21 17
$Wood = Rgb 46 39 30
function Pos($x, $y) { New-Object System.Drawing.Point($x, $y) }
function Size($w, $h) { New-Object System.Drawing.Size($w, $h) }
function Font($size, [switch]$Bold, $face = "Segoe UI") {
    $style = if ($Bold) { [System.Drawing.FontStyle]::Bold } else { [System.Drawing.FontStyle]::Regular }
    New-Object System.Drawing.Font($face, $size, $style)
}

$ui = @{ clicked = $null; closed = $false }
$form = New-Object System.Windows.Forms.Form
$form.Text = $Title
$form.ClientSize = Size 640 366
$form.FormBorderStyle = [System.Windows.Forms.FormBorderStyle]::FixedSingle
$form.MaximizeBox = $false
$form.StartPosition = [System.Windows.Forms.FormStartPosition]::CenterScreen
$form.BackColor = $Night
$form.ForeColor = $Ink
$icon = Join-Path $Launcher "icon.ico"
if (Test-Path $icon) { try { $form.Icon = New-Object System.Drawing.Icon($icon) } catch { } }
$form.Add_FormClosed({ $ui.closed = $true })

$shield = Join-Path $Launcher "shield.png"
if (Test-Path $shield) {
    $pic = New-Object System.Windows.Forms.PictureBox
    $pic.Image = [System.Drawing.Image]::FromFile($shield)
    $pic.SizeMode = [System.Windows.Forms.PictureBoxSizeMode]::Zoom
    $pic.Location = Pos 26 40
    $pic.Size = Size 220 260
    $form.Controls.Add($pic)
}

function Label($text, $x, $y, $w, $h, $font, $color) {
    $l = New-Object System.Windows.Forms.Label
    $l.Text = $text
    $l.Location = Pos $x $y
    $l.Size = Size $w $h
    $l.Font = $font
    $l.ForeColor = $color
    $l.BackColor = [System.Drawing.Color]::Transparent
    return $l
}
$main = New-Object System.Windows.Forms.Panel
$main.Location = Pos 262 24
$main.Size = Size 362 300
$form.Controls.Add($main)
$main.Controls.Add((Label $Title 0 8 360 44 (Font 22 -Bold "Palatino Linotype") $Gold))
$main.Controls.Add((Label ("version " + (Get-InstalledVersion)) 2 52 360 20 (Font 9) $Dim))
$status = Label "" 2 86 356 120 (Font 11) $Ink
$main.Controls.Add($status)
$bar = New-Object System.Windows.Forms.ProgressBar
$bar.Style = [System.Windows.Forms.ProgressBarStyle]::Marquee
$bar.MarqueeAnimationSpeed = 30
$bar.Location = Pos 4 210
$bar.Size = Size 352 6
$main.Controls.Add($bar)

function Button($x, $w, $primary) {
    $b = New-Object System.Windows.Forms.Button
    $b.Location = Pos $x 236
    $b.Size = Size $w 40
    $b.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
    $b.FlatAppearance.BorderColor = $Gold
    $b.Font = if ($primary) { Font 11 -Bold } else { Font 10 }
    if ($primary) { $b.BackColor = $Gold; $b.ForeColor = $Night } else { $b.BackColor = $Wood; $b.ForeColor = $Ink }
    $b.Visible = $false
    return $b
}
$btnA = Button 2 150 $true
$btnB = Button 160 110 $false
$btnC = Button 278 78 $false
$btnA.Add_Click({ $ui.clicked = "a" })
$btnB.Add_Click({ $ui.clicked = "b" })
$btnC.Add_Click({ $ui.clicked = "c" })
$main.Controls.AddRange(@($btnA, $btnB, $btnC))

# --- the choices (shown by Change) ---------------------------------------------------------------------
$choices = New-Object System.Windows.Forms.Panel
$choices.Location = Pos 262 16
$choices.Size = Size 366 346
$choices.Visible = $false
$form.Controls.Add($choices)
$choices.Controls.Add((Label "Game" 0 0 300 22 (Font 11 -Bold) $Gold))
$gameBox = New-Object System.Windows.Forms.Panel
$gameBox.Location = Pos 8 22
$gameBox.Size = Size 350 50
$choices.Controls.Add($gameBox)
function Radio($text, $y) {
    $r = New-Object System.Windows.Forms.RadioButton
    $r.Text = $text
    $r.Location = Pos 0 $y
    $r.Size = Size 340 22
    $r.Font = Font 10
    $r.ForeColor = $Ink
    return $r
}
$rSurvival = Radio $SurvivalName 2
$rBasin = Radio $BasinName 24
$gameBox.Controls.AddRange(@($rSurvival, $rBasin))
$choices.Controls.Add((Label "Server" 0 78 300 22 (Font 11 -Bold) $Gold))
$placeBox = New-Object System.Windows.Forms.Panel
$placeBox.Location = Pos 8 100
$placeBox.Size = Size 350 120
$choices.Controls.Add($placeBox)
$y = 2
# One choice for each of the group's servers (project.toml), then your own, then any other.
$serverRadios = @{}
foreach ($srv in $Servers) {
    $r = Radio "$($srv.name)  ($($srv.address))" $y
    $placeBox.Controls.Add($r)
    $serverRadios[$srv.key] = $r
    $y += 23
}
$rOwn = Radio "Single player (your own server on this PC)" $y
$y += 23
$rOther = Radio "Someone else's:" $y
$rOther.Size = Size 130 22
$addressBox = New-Object System.Windows.Forms.TextBox
$addressBox.Location = Pos 134 ($y - 1)
$addressBox.Size = Size 200 24
$addressBox.Font = Font 10
$addressBox.Add_TextChanged({ if ($addressBox.Focused -and $addressBox.Text) { $rOther.Checked = $true } })
$placeBox.Controls.AddRange(@($rOwn, $rOther, $addressBox))
$choices.Controls.Add((Label "World of Warcraft 1.12.1" 0 224 300 22 (Font 11 -Bold) $Gold))
$wowLabel = Label "" 8 246 270 34 (Font 9) $Dim
$choices.Controls.Add($wowLabel)
$wowButton = New-Object System.Windows.Forms.Button
$wowButton.Text = "Change..."
$wowButton.Location = Pos 282 244
$wowButton.Size = Size 76 26
$wowButton.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
$wowButton.FlatAppearance.BorderColor = $Gold
$wowButton.BackColor = $Wood
$wowButton.ForeColor = $Ink
$wowButton.Add_Click({
    $picked = Pick-WowData
    if ($picked) { $ui.data = $picked; $wowLabel.Text = $picked }
})
$choices.Controls.Add($wowButton)
$okButton = New-Object System.Windows.Forms.Button
$okButton.Text = "Play"
$okButton.Location = Pos 2 290
$okButton.Size = Size 150 40
$okButton.FlatStyle = [System.Windows.Forms.FlatStyle]::Flat
$okButton.BackColor = $Gold
$okButton.ForeColor = $Night
$okButton.Font = Font 11 -Bold
$okButton.Add_Click({ $ui.clicked = "ok" })
$choices.Controls.Add($okButton)

# --- helpers -----------------------------------------------------------------------------------------
function Pump { [System.Windows.Forms.Application]::DoEvents() }
function Leave { try { $form.Close() } catch { }; exit 0 }
function Wait-Pumping($ms) {
    $end = (Get-Date).AddMilliseconds($ms)
    while ((Get-Date) -lt $end) { Pump; if ($ui.closed) { exit 0 }; Start-Sleep -Milliseconds 40 }
}
function Set-Status($text, [switch]$Busy) {
    $status.Text = $text
    $bar.Visible = [bool]$Busy
    Log $text
    Pump
}
# The shared part (finding WoW, the update) reports through these: into the window.
function Say($text) { Set-Status $text -Busy }
function Warn($text) { Set-Status $text }
function Buttons($a, $b, $c) {
    foreach ($pair in @(@($btnA, $a), @($btnB, $b), @($btnC, $c))) {
        $pair[0].Text = "$($pair[1])"
        $pair[0].Visible = [bool]$pair[1]
    }
    Pump
}
# Waits for one of the three buttons (or $ms to pass: then $null).
function Wait-Click($ms = 0) {
    $ui.clicked = $null
    $end = if ($ms) { (Get-Date).AddMilliseconds($ms) } else { [datetime]::MaxValue }
    while (-not $ui.clicked -and (Get-Date) -lt $end) {
        Pump
        if ($ui.closed) { exit 0 }
        Start-Sleep -Milliseconds 40
    }
    return $ui.clicked
}
function Fail($text) {
    Set-Status $text
    Buttons "Close" "" ""
    Wait-Click | Out-Null
    Leave
}
# A connection that keeps the window answering while it's tried.
function Reach($address, $port, $ms) {
    $c = New-Object Net.Sockets.TcpClient
    try {
        $t = $c.ConnectAsync($address, $port)
        $end = (Get-Date).AddMilliseconds($ms)
        while (-not $t.IsCompleted -and (Get-Date) -lt $end) { Pump; Start-Sleep -Milliseconds 40 }
        return ($t.IsCompleted -and -not $t.IsFaulted -and $c.Connected)
    } catch { return $false } finally { $c.Close() }
}
function Where-Words($s) {
    switch ($s["place"]) {
        "own" { "single player, on this PC" }
        "address" { "$($s["address"])" }
        default { $srv = Server-Of $s["place"]; if ($srv) { $srv.name } else { "" } }
    }
}
# This PC's own server (its ports: Own-Ports), started in its own window when it isn't running.
# Its address for the game.
function Start-Own($worldPort) {
    if (-not ((Reach "127.0.0.1" $own.login 500) -and (Reach "127.0.0.1" $worldPort 500))) {
        if (-not (Test-Path $ServerExe)) { Fail "This download has no server in it (server\htl-server.exe). Download the newest version, or play on someone else's server." }
        Set-Status "Starting your server. It opens in its own window: leave that open while you play." -Busy
        $hostExe = Join-Path $Root "Host.exe"
        if (Test-Path $hostExe) { Start-Process -FilePath $hostExe -WorkingDirectory $Root }
        else { Start-Process powershell.exe -ArgumentList @("-NoProfile", "-ExecutionPolicy", "Bypass", "-File", "`"$(Join-Path $Launcher "host.ps1")`"") -WorkingDirectory $Root }
        $waited = 0
        while (-not ((Reach "127.0.0.1" $own.login 500) -and (Reach "127.0.0.1" $worldPort 500))) {
            Wait-Pumping 1000
            $waited += 2
            if ($waited % 10 -eq 0) { Set-Status "Your server is loading the world ($waited s). Its window shows how it's going." -Busy }
            if ($waited -ge 600) { Fail "Your server didn't start in 10 minutes. Its window says what went wrong." }
        }
    }
    if ($own.login -ne 3724) { return "127.0.0.1:$($own.login)" }
    return "127.0.0.1"
}
# The choices, filled from $s; back into $s when Play is pressed there.
function Choose($s) {
    if ($s["game"] -eq "basin") { $rBasin.Checked = $true } else { $rSurvival.Checked = $true }
    $addressBox.Text = "$($s["address"])"
    $which = $rOwn
    if ($s["place"] -eq "address") { $which = $rOther }
    elseif ($serverRadios.ContainsKey("$($s["place"])")) { $which = $serverRadios[$s["place"]] }
    elseif (-not $s["place"] -and $Servers.Count) { $which = $serverRadios[$Servers[0].key] }
    $which.Checked = $true
    $wowLabel.Text = "$($ui.data)"
    $main.Visible = $false
    $choices.Visible = $true
    while ($true) {
        $ui.clicked = $null
        while ($ui.clicked -ne "ok") { Pump; if ($ui.closed) { exit 0 }; Start-Sleep -Milliseconds 40 }
        if ($rOther.Checked -and -not $addressBox.Text.Trim()) {
            [System.Windows.Forms.MessageBox]::Show("Type the address the host gave you (they see it in their Host window).", $Title) | Out-Null
            continue
        }
        break
    }
    $s["game"] = if ($rBasin.Checked) { "basin" } else { "survival" }
    $picked = $null
    foreach ($k in $serverRadios.Keys) { if ($serverRadios[$k].Checked) { $picked = $k } }
    if ($picked) { $s["place"] = $picked }
    elseif ($rOther.Checked) { $s["place"] = "address"; $s["address"] = $addressBox.Text.Trim() }
    else { $s["place"] = "own" }
    if ($ui.data) { $s["wow"] = $ui.data }
    Save-Settings $s
    $choices.Visible = $false
    $main.Visible = $true
    Pump
}

# --- the steps -----------------------------------------------------------------------------------------
$form.Show()
Pump
try {
    if (-not (Test-Path $Exe)) { Fail "benilla.exe is missing from this folder. Your anti-virus may have taken it: restore it, or download the game again." }
    while (Get-OurGame) {
        Set-Status "The game is already open (its window is called benilla). Only one at a time: close it first."
        Buttons "Try again" "" "Quit"
        if ((Wait-Click) -eq "c") { Leave }
    }

    # 1. The newest version, asked in the background so the window keeps answering.
    if (-not $NoUpdate -and (Get-InstalledVersion)) {
        Set-Status "Checking for a new version..." -Busy
        $job = Start-Job -ArgumentList (Join-Path $Launcher "update.ps1") -ScriptBlock {
            param($u)
            # A job is a new PowerShell: let it run the script, as Play.exe's own does.
            Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
            & $u -Quiet
            "EXITCODE=$LASTEXITCODE"
        }
        while ($job.State -eq "Running") {
            $info = $job.ChildJobs[0].Information
            if ($info.Count) {
                $line = "$($info[$info.Count - 1].MessageData)"
                if ($line -and $line -ne $status.Text) { Set-Status $line -Busy }
            }
            Pump
            if ($ui.closed) { Stop-Job $job; exit 0 }
            Start-Sleep -Milliseconds 80
        }
        $out = @(Receive-Job $job -ErrorAction SilentlyContinue)
        Remove-Job $job -Force -ErrorAction SilentlyContinue
        $code = "$($out | Where-Object { "$_" -match '^EXITCODE=' } | Select-Object -Last 1)" -replace '^EXITCODE=', ''
        Log "update: $code"
        if ($code -eq "0") {
            # The new version's own launcher takes it from here.
            Set-Status "Updated to $(Get-InstalledVersion). Starting it..." -Busy
            $again = @("-NoUpdate")
            if ($Change) { $again += "-Change" }
            if ($Game) { $again += @("-Game", $Game) }
            $playExe = Join-Path $Root "Play.exe"
            if (Test-Path $playExe) { Start-Process -FilePath $playExe -ArgumentList $again -WorkingDirectory $Root }
            else { Start-Process powershell.exe -ArgumentList (@("-NoProfile", "-ExecutionPolicy", "Bypass", "-STA", "-WindowStyle", "Hidden", "-File", "`"$(Join-Path $Launcher "play.ps1")`"") + $again) }
            Wait-Pumping 800
            Leave
        }
        if ($code -eq "1") { Wait-Pumping 2500 }
    }

    # 2. Your WoW 1.12.1.
    $settings = Read-Settings
    $ui.data = DataFolderOf $settings["wow"]
    if (-not $ui.data) {
        Set-Status "Looking for your World of Warcraft 1.12.1..." -Busy
        $ui.data = Find-WowData
        if ($ui.data) { Set-Status "Found it: $($ui.data)"; $settings["wow"] = $ui.data; Save-Settings $settings }
    }
    while (-not $ui.data) {
        Set-Status "I couldn't find World of Warcraft 1.12.1 on this PC. Show me its folder: the one with WoW.exe and a Data folder in it. (Modern WoW Classic won't work: it has to be 1.12.1.)"
        Buttons "Choose folder..." "" "Quit"
        if ((Wait-Click) -eq "c") { Leave }
        $ui.data = Pick-WowData
        if ($ui.data) { $settings["wow"] = $ui.data; Save-Settings $settings }
    }

    # 3. Which game, and where.
    if ($Game -match 'basin') { $settings["game"] = "basin" } elseif ($Game) { $settings["game"] = "survival" }
    if (-not $settings["game"]) { $settings["game"] = "survival" }
    # An older launcher's "group" is the first server; a server project.toml no longer names is forgotten.
    if ($settings["place"] -eq "group") { $settings["place"] = "server" }
    if ($settings["place"] -and @("own", "address") -notcontains $settings["place"] -and -not (Server-Of $settings["place"])) { $settings["place"] = "" }
    # With the group's servers named in project.toml, a first run goes straight to the first: one click to join.
    if (-not $settings["place"] -and $Servers.Count) { $settings["place"] = $Servers[0].key }
    $own = Own-Ports $settings
    # Single player for this time only (its button), whatever the saved choice is.
    $solo = $false
    if ($Change -or -not (Where-Words $settings)) { Choose $settings }

    while ($true) {
        $gameName = GameName $settings["game"]
        $port = WorldPort $settings["game"] $own
        # Last time's choice, unless Change is pressed in the next 3 seconds.
        $place = $settings["place"]
        if ($solo) { $place = "own" }
        $words = if ($solo) { "single player, on this PC" } else { Where-Words $settings }
        Set-Status "Playing $gameName on $words."
        $go = $null
        $offerSolo = ($place -ne "own")
        for ($left = 3; $left -ge 1 -and -not $go; $left--) {
            if ($offerSolo) { Buttons "Play ($left)" "Single player" "Change..." } else { Buttons "Play ($left)" "Change..." "" }
            $go = Wait-Click 1000
        }
        if ($offerSolo -and $go -eq "b") { $solo = $true; continue }
        if (($offerSolo -and $go -eq "c") -or (-not $offerSolo -and $go -eq "b")) { $solo = $false; Choose $settings; continue }
        Buttons "" "" ""

        # 4. The server.
        $address = $null
        $where = $words
        if ($place -eq "own") { $address = Start-Own $port; break }
        if ($place -eq "address") {
            $candidates = @(@{ key = "address"; address = $settings["address"]; name = $settings["address"]; pc = "" })
        } else {
            # The chosen one of the group's servers, then each after it: the first that answers.
            $candidates = @()
            $on = $false
            foreach ($srv in $Servers) { if ($srv.key -eq $place) { $on = $true }; if ($on) { $candidates += $srv } }
        }
        $tried = @()
        foreach ($srv in $candidates) {
            # A Tailscale address only answers with Tailscale on this PC: skipped then, unless it was picked.
            if ((Is-TailscaleAddress $srv.address) -and -not (Test-Path $TailscaleExe) -and $srv.key -ne $place) { continue }
            if ($srv.pc -and $srv.pc -ieq $env:COMPUTERNAME) {
                # This PC hosts that one: its own server, started if it isn't running.
                $address = Start-Own $port
                $where = "$($srv.name) (this PC)"
                break
            }
            Set-Status "Looking for $($srv.name)..." -Busy
            $at = Split-Address $srv.address
            if (Reach $at.host $at.port 4000) { $address = $srv.address; $where = $srv.name; break }
            Log "no answer from $($srv.name) at $($srv.address)"
            $tried += $srv
        }
        if ($address) { break }

        # None answered: say why, and offer single player.
        if ($tried.Count -eq 1) {
            if ((Is-TailscaleAddress $tried[0].address) -and -not (Test-Path $TailscaleExe)) { Start-Process "https://tailscale.com/download/windows" }
            Set-Status (Why-Unreachable $tried[0].address)
        } else {
            $names = ($tried | ForEach-Object { $_.name }) -join ", "
            Set-Status "None of the group's servers is up right now ($names). Someone has to be hosting for you to join: try again in a bit, or play single player on this PC."
        }
        Buttons "Try again" "Single player" "Change..."
        $answer = Wait-Click
        if ($answer -eq "b") { $solo = $true }
        if ($answer -eq "c") { $solo = $false; Choose $settings }
    }

    # 5. Play.
    $env:WOW_DATA = $ui.data
    $env:WOW_HOST = $address
    $env:WOW_REALM = $gameName
    # The game's memory of item and creature details: cleared so it always asks the server afresh.
    Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $Root "benilla-config\cache")
    # The bundled settings only for a first run: an update keeps your mouse speed and the rest.
    $config = Join-Path $Root "benilla-config\config.toml"
    $defaults = Join-Path $Root "benilla-config\config.default.toml"
    if (-not (Test-Path $config) -and (Test-Path $defaults)) { Copy-Item $defaults $config }
    # benilla is a console program: started like this it opens no console window behind the game.
    $start = New-Object System.Diagnostics.ProcessStartInfo
    $start.FileName = $Exe
    $start.WorkingDirectory = $Root
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    [System.Diagnostics.Process]::Start($start) | Out-Null
    Log "joining $gameName at $address ($where)"
    Set-Status "Starting $gameName on $where...`n`nLog in with any name you like (it becomes your hero) and the password: hold, unless the host changed it." -Busy
    Wait-Pumping 5000
    Leave
} catch {
    Log "error: $($_.Exception.Message) $($_.ScriptStackTrace)"
    Fail "Something went wrong: $($_.Exception.Message)`n`nWhat happened is in launcher\play.log."
}
