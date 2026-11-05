#!/bin/bash
# The War Table launcher on a Mac: builds Play (dist/windows/buttons, the same launcher as the
# Windows download's Play.exe) and makes "Basin Battle.app" in this folder, with the shield as its
# icon. Play Basin Battle.command runs this; so does the app when it opens, to rebuild Play
# after an update changed it (seconds when nothing did).
#   scripts/mac-launcher.sh            build Play and the app
#   scripts/mac-launcher.sh --quiet    the same, saying nothing unless it fails
# The app plays through scripts/play.sh and hosts with Host Basin Battle.command, so the game
# itself still needs setting up once (Build Basin Battle.command).
set -euo pipefail
. "$(dirname "$0")/common.sh"
cd "$ROOT"
quiet=0
[ "${1:-}" = "--quiet" ] && quiet=1
say() { [ "$quiet" -eq 1 ] || echo "$@"; }

if ! command -v cargo >/dev/null 2>&1; then
    echo "Rust isn't installed yet: double-click Build Basin Battle.command first."
    exit 1
fi
say "Building Play (the launcher; a few minutes the first time)..."
if [ "$quiet" -eq 1 ]; then
    cargo build --release -q --manifest-path dist/windows/buttons/Cargo.toml --bin Play 2>/dev/null
else
    cargo build --release --manifest-path dist/windows/buttons/Cargo.toml --bin Play
fi

app="$ROOT/Basin Battle.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
# Each file goes in under a new name and is then moved over the old: the app runs this script
# while its own script is running, and bash reads a script as it goes.
put() { # from to
    cp "$1" "$2.new"
    chmod +x "$2.new"
    mv -f "$2.new" "$2"
}
cmp -s dist/windows/buttons/target/release/Play "$app/Contents/MacOS/Play" || put dist/windows/buttons/target/release/Play "$app/Contents/MacOS/Play"

# What the app runs when it's opened: Play rebuilt if its code changed, the dispatches written from
# this folder's history, then Play itself.
tmp="$(mktemp)"
cat > "$tmp" <<'APP'
#!/bin/bash
here="$(cd "$(dirname "$0")" && pwd)"
root="$(cd "$here/../../.." && pwd)"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
cd "$root" || exit 1
bash scripts/mac-launcher.sh --quiet >/dev/null 2>&1 || true
# The dispatches: python3 comes with Apple's command line tools (without them it would ask to install).
if xcode-select -p >/dev/null 2>&1; then python3 scripts/launcher-news.py dist/windows/launcher/ui/news.json >/dev/null 2>&1 || true; fi
exec "$here/Play" "$@"
APP
cmp -s "$tmp" "$app/Contents/MacOS/Basin Battle" || put "$tmp" "$app/Contents/MacOS/Basin Battle"
rm -f "$tmp"

name="$(project_name title "Basin Battle")"
cat > "$app/Contents/Info.plist" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$name</string>
    <key>CFBundleDisplayName</key><string>$name</string>
    <key>CFBundleIdentifier</key><string>io.github.slopmaster69.basinbattle.play</string>
    <key>CFBundleExecutable</key><string>Basin Battle</string>
    <key>CFBundleIconFile</key><string>shield</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
</dict>
</plist>
PLIST

# The shield as the app's icon (sips and iconutil come with every Mac).
if [ ! -f "$app/Contents/Resources/shield.icns" ] && command -v iconutil >/dev/null 2>&1; then
    set_dir="$(mktemp -d)/shield.iconset"
    mkdir -p "$set_dir"
    for s in 16 32 128 256; do
        sips -z $s $s dist/windows/art/shield.png --out "$set_dir/icon_${s}x${s}.png" >/dev/null
        sips -z $((s * 2)) $((s * 2)) dist/windows/art/shield.png --out "$set_dir/icon_${s}x${s}@2x.png" >/dev/null
    done
    iconutil -c icns "$set_dir" -o "$app/Contents/Resources/shield.icns" || true
fi
# So Finder shows the new icon at once.
touch "$app"
say "Made $(basename "$app"): double-click it to play."
