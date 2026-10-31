# Shared by the Mac/Linux scripts: where things live, and the saved settings.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The benilla client we play with, pinned so everyone runs the same build.
BENILLA_REPO="https://github.com/samwhosung/benilla"
BENILLA_REV="c23d0163d5efa1ae39e4748877fe4ba831202848"
SETTINGS="$ROOT/settings.sh"
# This folder's own changes to the client (the action-combat mouse and the aiming camera for
# Elwynn Survival), applied on top of the pinned revision.
BENILLA_PATCHES="$ROOT/patches/benilla"
[ -f "$SETTINGS" ] && . "$SETTINGS"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

save_setting() { # name value
    touch "$SETTINGS"
    grep -v "^$1=" "$SETTINGS" > "$SETTINGS.tmp" || true
    printf '%s=%q\n' "$1" "$2" >> "$SETTINGS.tmp"
    mv "$SETTINGS.tmp" "$SETTINGS"
}

find_wow_data() {
    if [ -n "${WOW_DATA:-}" ] && [ -f "$WOW_DATA/dbc.MPQ" ]; then return 0; fi
    for c in "$ROOT/../WoW Classic/WoW-1.12.1/Data" "$ROOT/WoW/Data" "$HOME/WoW-1.12.1/Data" "/Applications/World of Warcraft/Data"; do
        if [ -f "$c/dbc.MPQ" ]; then WOW_DATA="$(cd "$c" && pwd)"; save_setting WOW_DATA "$WOW_DATA"; return 0; fi
    done
    echo "Where is the Data folder of your WoW 1.12.1 install? (drag the folder in here and press Return)"
    read -r answer
    answer="${answer%/}"; answer="${answer//\\ / }"; answer="${answer%\'}"; answer="${answer#\'}"
    if [ ! -f "$answer/dbc.MPQ" ]; then echo "That folder has no dbc.MPQ in it; it isn't a 1.12.1 Data folder."; exit 1; fi
    WOW_DATA="$answer"; save_setting WOW_DATA "$WOW_DATA"
}

# A fingerprint of the client patches: the client is rebuilt when it changes.
patches_stamp() {
    cat "$BENILLA_PATCHES"/*.patch 2>/dev/null | cksum | awk '{print $1}'
}

# The pinned client with this folder's patches on top, from a clean checkout of its sources
# (anything an earlier patch changed or added in them is put back first).
apply_benilla_patches() {
    local dir="$ROOT/benilla"
    git -C "$dir" checkout -q -- crates
    git -C "$dir" clean -q -f -- crates
    git -C "$dir" checkout -q "$BENILLA_REV"
    local p
    for p in "$BENILLA_PATCHES"/*.patch; do
        [ -f "$p" ] || continue
        git -C "$dir" apply --whitespace=nowarn "$p"
    done
}

# Build the client if it's missing or its patches changed since it was built.
build_benilla_if_needed() {
    local stamp
    stamp="$(patches_stamp)"
    if [ -x "$ROOT/benilla/target/release/benilla" ] && [ "$(cat "$ROOT/benilla/.htl-patches" 2>/dev/null)" = "$stamp" ]; then
        return 0
    fi
    echo "Building the benilla client with this folder's changes (a few minutes after an update)..."
    apply_benilla_patches
    (cd "$ROOT/benilla" && cargo build --release -p benilla)
    echo "$stamp" > "$ROOT/benilla/.htl-patches"
}
