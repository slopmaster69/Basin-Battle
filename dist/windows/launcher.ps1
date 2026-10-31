# The launcher for Windows. "Play Basin Battle.bat" and "Play Elwynn Survival.bat" run this; you
# don't need to open it. It remembers your game folder and the host's address in settings.txt.
param([string]$Game = "Basin Battle")
$ErrorActionPreference = "Stop"
$Host.UI.RawUI.WindowTitle = "$Game launcher (closes once the game starts)"
$Here = Split-Path -Parent $MyInvocation.MyCommand.Path
$Settings = Join-Path $Here "settings.txt"
$Exe = Join-Path $Here "benilla.exe"
Add-Type -AssemblyName System.Windows.Forms

function Say($text) { Write-Host $text -ForegroundColor Cyan }

# settings.txt: line 1 the Data folder, line 2 the host's address.
$saved = @("", "")
if (Test-Path $Settings) { $saved = @(Get-Content $Settings) + @("", "") }
$data = $saved[0].Trim()
$hostAddr = $saved[1].Trim()

# A folder counts if it is the Data folder itself or the WoW folder that holds it.
function DataFolderOf($dir) {
    if (-not $dir) { return $null }
    $dir = $dir.Trim().Trim('"')
    if (Test-Path (Join-Path $dir "dbc.MPQ")) { return (Resolve-Path $dir).Path }
    $inner = Join-Path $dir "Data"
    if (Test-Path (Join-Path $inner "dbc.MPQ")) { return (Resolve-Path $inner).Path }
    return $null
}

$data = DataFolderOf $data
while (-not $data) {
    Say "Pick your World of Warcraft 1.12.1 folder (the one with WoW.exe and a Data folder in it)."
    $dialog = New-Object System.Windows.Forms.FolderBrowserDialog
    $dialog.Description = "Pick your World of Warcraft 1.12.1 folder (the one that contains the Data folder)"
    $dialog.ShowNewFolderButton = $false
    if ($dialog.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) {
        Say "No folder picked. Run Play Basin Battle again when you're ready."
        Read-Host "Press Enter to close"
        exit 1
    }
    $data = DataFolderOf $dialog.SelectedPath
    if (-not $data) {
        [System.Windows.Forms.MessageBox]::Show(
            "That folder has no Data\dbc.MPQ in it, so it isn't a WoW 1.12.1 install. Pick the folder that has WoW.exe in it.",
            "Basin Battle") | Out-Null
    }
}

$prompt = "Address of the game to join (ask whoever is hosting)"
if ($hostAddr) { $prompt += " [Enter keeps $hostAddr]" }
$answer = (Read-Host $prompt).Trim()
if ($answer) { $hostAddr = $answer }
while (-not $hostAddr) { $hostAddr = (Read-Host "Address of the game to join").Trim() }

Set-Content -Path $Settings -Value @($data, $hostAddr)

Say "Game files: $data"
Say "Joining:    $hostAddr ($Game)"
Say "Log in with any name you like and the host's password (default: hold)."
$env:WOW_DATA = $data
$env:WOW_HOST = $hostAddr
$env:WOW_REALM = $Game
# The game's memory of item and creature details: cleared so it always asks the server afresh.
Remove-Item -Recurse -Force -ErrorAction SilentlyContinue (Join-Path $Here "benilla-config\cache")
# The bundled settings only for a first run: an update keeps the mouse speed and the rest.
$Config = Join-Path $Here "benilla-config\config.toml"
$Defaults = Join-Path $Here "benilla-config\config.default.toml"
if (-not (Test-Path $Config) -and (Test-Path $Defaults)) { Copy-Item $Defaults $Config }
Start-Process -FilePath $Exe -WorkingDirectory $Here
Say "The game is starting in its own window (titled benilla). This launcher window closes now."
Start-Sleep -Seconds 2
