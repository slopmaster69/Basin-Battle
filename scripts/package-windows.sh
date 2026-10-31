#!/bin/bash
# Build "Basin Battle for Windows.zip" from a cross-compiled benilla.exe (Linux, with mingw-w64):
#   scripts/package-windows.sh path/to/benilla.exe
# The exe must be the player build (`--no-default-features`): a dev build looks for its settings
# and add-ons in the build machine's source folder, so on a friend's PC no add-on would load.
#   cargo build --release -p benilla --bin benilla --target x86_64-pc-windows-gnu --no-default-features
# Where mingw-w64's msvcrt import library lacks _get_timezone, _get_tzname and _get_dstbias (the
# libc crate calls them; ucrt has them), link scripts/mingw-tz-shim.c in, which reads msvcrt's data:
#   x86_64-w64-mingw32-gcc -O2 -w -c scripts/mingw-tz-shim.c -o /tmp/tz.o
#   cargo rustc --release -p benilla --bin benilla --target x86_64-pc-windows-gnu --no-default-features \
#     -- -C link-arg=/tmp/tz.o -C link-arg=-lmsvcrt
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
EXE="$1"
if LC_ALL=C grep -qaE "crates/benilla-formats([^/]|$)" "$EXE"; then
    echo "$EXE is a dev build (it has its build folder baked in); build it with --no-default-features." >&2
    exit 1
fi
OUT="$ROOT/target/windows/Basin Battle for Windows"
rm -rf "$OUT"
mkdir -p "$OUT/benilla-config"
x86_64-w64-mingw32-strip -o "$OUT/benilla.exe" "$EXE"
cp "$ROOT/dist/windows/Play Basin Battle.bat" "$ROOT/dist/windows/Play Elwynn Survival.bat" "$ROOT/dist/windows/launcher.ps1" "$ROOT/dist/windows/README.txt" "$OUT/"
cp "$ROOT/dist/windows/LICENSE-MIT" "$ROOT/dist/windows/LICENSE-APACHE" "$OUT/"
# As config.default.toml: the launcher copies it to config.toml only on a first run, so unzipping
# an update over an old folder keeps the player's own settings (the mouse speed among them).
cp "$ROOT/dist/windows/benilla-config/config.toml" "$OUT/benilla-config/config.default.toml"
mkdir -p "$OUT/benilla-config/AddOns"
cp -R "$ROOT/addon/BasinBattle" "$ROOT/addon/ElwynnSurvival" "$OUT/benilla-config/AddOns/"
cd "$ROOT/target/windows"
rm -f "Basin Battle for Windows.zip"
python3 -c "
import shutil; shutil.make_archive('Basin Battle for Windows', 'zip', '.', 'Basin Battle for Windows')
"
ls -la "$ROOT/target/windows/Basin Battle for Windows.zip"
