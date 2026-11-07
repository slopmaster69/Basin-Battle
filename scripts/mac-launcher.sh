#!/bin/bash
# Play on a Mac: builds Play (dist/windows/buttons, the same launcher as the Windows download's
# Play.exe) and makes "<title>.app" (project.toml's title) in this folder, with the shield as its
# icon. Play Basin Battle.command runs this.
#   scripts/mac-launcher.sh            build Play and the app
#   scripts/mac-launcher.sh --quiet    the same, saying nothing unless it fails
# The app plays through scripts/play.sh, so the game itself still needs setting up once
# (Build Basin Battle.command); the folder is updated with git pull, not by Play.
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

name="$(project_name title "Elwynn Survival")"
app="$ROOT/$name.app"
# The app under the project's old name, from before the title changed.
[ "$name" != "Basin Battle" ] && [ -d "$ROOT/Basin Battle.app" ] && rm -rf "$ROOT/Basin Battle.app"
mkdir -p "$app/Contents/MacOS" "$app/Contents/Resources"
changed=0
# Each file goes in under a new name and is then moved over the old: the app runs this script
# while Play, its own program, is running.
put() { # from to
    cp "$1" "$2.new"
    chmod +x "$2.new"
    mv -f "$2.new" "$2"
    changed=1
}
# Play is the app's own program, so macOS asks for Documents, Desktop or Downloads on its behalf
# (an app whose program is a script is kept out of them without being asked). The app's copy
# is signed, so it never matches the build: the build last put in is kept beside it, to compare.
built=dist/windows/buttons/target/release/Play
cmp -s "$built" "$built.in-app" && [ -f "$app/Contents/MacOS/Play" ] || { put "$built" "$app/Contents/MacOS/Play"; cp "$built" "$built.in-app"; }
# The script that was the app's program before Play was.
[ -e "$app/Contents/MacOS/Basin Battle" ] && { rm -f "$app/Contents/MacOS/Basin Battle"; changed=1; }

tmp="$(mktemp)"
cat > "$tmp" <<PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key><string>$name</string>
    <key>CFBundleDisplayName</key><string>$name</string>
    <key>CFBundleIdentifier</key><string>io.github.slopmaster69.basinbattle.play</string>
    <key>CFBundleExecutable</key><string>Play</string>
    <key>CFBundleIconFile</key><string>shield</string>
    <key>CFBundlePackageType</key><string>APPL</string>
    <key>CFBundleShortVersionString</key><string>1.0</string>
    <key>LSMinimumSystemVersion</key><string>11.0</string>
    <key>NSHighResolutionCapable</key><true/>
    <key>NSDocumentsFolderUsageDescription</key><string>$name plays from its folder, and the folder is in Documents.</string>
    <key>NSDesktopFolderUsageDescription</key><string>$name plays from its folder, and the folder is on the Desktop.</string>
    <key>NSDownloadsFolderUsageDescription</key><string>$name plays from its folder, or reads World of Warcraft from it, in Downloads.</string>
</dict>
</plist>
PLIST
cmp -s "$tmp" "$app/Contents/Info.plist" || put "$tmp" "$app/Contents/Info.plist"
chmod 644 "$app/Contents/Info.plist"
rm -f "$tmp"

# The shield as the app's icon (sips and iconutil come with every Mac).
if [ ! -f "$app/Contents/Resources/shield.icns" ] && command -v iconutil >/dev/null 2>&1; then
    set_dir="$(mktemp -d)/shield.iconset"
    mkdir -p "$set_dir"
    for s in 16 32 128 256; do
        sips -z $s $s dist/windows/art/shield.png --out "$set_dir/icon_${s}x${s}.png" >/dev/null
        sips -z $((s * 2)) $((s * 2)) dist/windows/art/shield.png --out "$set_dir/icon_${s}x${s}@2x.png" >/dev/null
    done
    iconutil -c icns "$set_dir" -o "$app/Contents/Resources/shield.icns" && changed=1 || true
fi
# Signed (ad hoc: no developer account), so macOS can remember what it allowed the app. Only when
# something in it changed: a new Play is a new signature, and macOS may ask again after it.
if command -v codesign >/dev/null 2>&1; then
    if [ "$changed" -eq 1 ] || ! codesign --verify --strict "$app" >/dev/null 2>&1; then
        codesign --force --sign - "$app" >/dev/null 2>&1 || say "Couldn't sign $(basename "$app"); macOS may keep it out of this folder."
    fi
fi
# So Finder shows the new icon, and macOS opens the new program, at once.
touch "$app"
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
[ "$changed" -eq 1 ] && [ -x "$lsregister" ] && "$lsregister" -f "$app" >/dev/null 2>&1 || true
say "Made $(basename "$app"): double-click it to play."
