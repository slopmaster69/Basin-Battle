# Rust for building the server on Windows, with the GNU toolchain: it links with its own bundled
# linker, so no Visual Studio is needed. Dot-sourced by host-windows.ps1, and by CI so it builds
# the way a player's PC does:  . scripts\windows-rust.ps1; cargo build --release -p htl-server
#
# Installs Rust if it's missing. Then, on the GNU toolchain with no MinGW dlltool on the PATH,
# adds rustup's llvm-tools and points rustc at its llvm-dlltool: windows-sys 0.61 links Windows'
# DLLs through raw-dylib, and on windows-gnu rustc needs a dlltool for that
# (https://github.com/rust-lang/rust/issues/140704), which a PC without MinGW doesn't have.
$cargoBin = Join-Path $env:USERPROFILE ".cargo\bin"
if (Test-Path $cargoBin) { $env:PATH = "$cargoBin;$env:PATH" }
if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rust (once; a few minutes)..." -ForegroundColor Cyan
    $installer = Join-Path $env:TEMP "rustup-init.exe"
    Invoke-WebRequest -UseBasicParsing -Uri "https://win.rustup.rs/x86_64" -OutFile $installer
    & $installer -y --profile minimal --default-host x86_64-pc-windows-gnu --default-toolchain stable
    if ($LASTEXITCODE -ne 0) { throw "The Rust installer failed (exit $LASTEXITCODE)." }
    $env:PATH = "$cargoBin;$env:PATH"
}

$rustHost = (& rustc -vV | Select-String -Pattern '^host: (\S+)').Matches[0].Groups[1].Value
if ($rustHost -like "*-windows-gnu*" -and -not (Get-Command dlltool -ErrorAction SilentlyContinue)) {
    $rustlib = Join-Path (& rustc --print sysroot).Trim() "lib\rustlib"
    function FindLlvmTool($name) {
        Get-ChildItem -Path $rustlib -Recurse -File -Filter $name -ErrorAction SilentlyContinue |
            Select-Object -First 1 -ExpandProperty FullName
    }
    if (-not ((FindLlvmTool "llvm-dlltool.exe") -or (FindLlvmTool "llvm-ar.exe"))) {
        Write-Host "Adding LLVM's tools to Rust (once; a minute)..." -ForegroundColor Cyan
        & rustup component add llvm-tools
        if ($LASTEXITCODE -ne 0) { throw "Adding Rust's llvm-tools failed (exit $LASTEXITCODE)." }
    }
    $dlltool = FindLlvmTool "llvm-dlltool.exe"
    if (-not $dlltool) {
        # llvm-ar is LLVM's dlltool too, when its file is named for it.
        $ar = FindLlvmTool "llvm-ar.exe"
        if (-not $ar) {
            $have = Get-ChildItem -Path $rustlib -Recurse -File -Filter "llvm-*.exe" -ErrorAction SilentlyContinue |
                ForEach-Object { $_.FullName }
            throw "Rust's llvm-tools has neither llvm-dlltool.exe nor llvm-ar.exe under $rustlib. It has: $($have -join ', ')"
        }
        $dlltool = Join-Path (Split-Path -Parent $PSScriptRoot) "target\llvm-dlltool\dlltool.exe"
        if (-not (Test-Path $dlltool) -or (Get-Item $ar).LastWriteTime -gt (Get-Item $dlltool).LastWriteTime) {
            New-Item -ItemType Directory -Force -Path (Split-Path -Parent $dlltool) | Out-Null
            Copy-Item -Force $ar $dlltool
        }
    }
    Write-Host "Using $dlltool as dlltool." -ForegroundColor Cyan
    # Encoded (one flag per 0x1F-separated field) so a user folder with a space in it survives.
    $env:CARGO_ENCODED_RUSTFLAGS = "-Cdlltool=$dlltool"
}
