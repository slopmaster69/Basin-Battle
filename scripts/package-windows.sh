#!/bin/bash
# Build "<title> for Windows.zip" (project.toml's title), laid out as a release's, from a
# cross-compiled benilla.exe and htl-server.exe (Linux, with mingw-w64):
#   scripts/package-windows.sh path/to/benilla.exe path/to/htl-server.exe
#   cargo build --release -p htl-server --target x86_64-pc-windows-gnu      (the server)
# Its version is "local build", so it never updates itself over to a release.
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
. "$ROOT/scripts/common.sh"
EXE="$1"
SERVER="${2:?the server too: scripts/package-windows.sh benilla.exe htl-server.exe}"
if LC_ALL=C grep -qaE "crates/benilla-formats([^/]|$)" "$EXE"; then
    echo "$EXE is a dev build (it has its build folder baked in); build it with --no-default-features." >&2
    exit 1
fi
PACKAGE="$(project_name title "Basin Battle") for Windows"
OUT="$ROOT/target/windows/$PACKAGE"
rm -rf "$OUT"
mkdir -p "$OUT/benilla-config" "$OUT/launcher" "$OUT/server"
x86_64-w64-mingw32-strip -o "$OUT/benilla.exe" "$EXE"
cp "$ROOT/dist/windows/Play.bat" "$ROOT/dist/windows/Host.bat" "$ROOT/dist/windows/README.txt" "$OUT/"
cp "$ROOT"/dist/windows/launcher/*.ps1 "$ROOT/project.toml" "$ROOT/dist/windows/LICENSE-MIT" "$ROOT/dist/windows/LICENSE-APACHE" "$OUT/launcher/"
echo "local build" > "$OUT/launcher/version.txt"
x86_64-w64-mingw32-strip -o "$OUT/server/htl-server.exe" "$SERVER"
cp -R "$ROOT/content" "$OUT/server/content"
# As config.default.toml: Play copies it to config.toml only on a first run, so an update keeps
# the player's own settings (the mouse speed among them).
cp "$ROOT/dist/windows/benilla-config/config.toml" "$OUT/benilla-config/config.default.toml"
mkdir -p "$OUT/benilla-config/AddOns"
cp -R "$ROOT/addon/BasinBattle" "$ROOT/addon/ElwynnSurvival" "$OUT/benilla-config/AddOns/"
cd "$ROOT/target/windows"
rm -f "$PACKAGE.zip"
python3 -c "
import shutil, sys; shutil.make_archive(sys.argv[1], 'zip', '.', sys.argv[1])
" "$PACKAGE"
ls -la "$ROOT/target/windows/$PACKAGE.zip"
