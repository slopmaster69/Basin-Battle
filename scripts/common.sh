# Shared by the Mac/Linux scripts: where things live, and the saved settings.
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# The benilla client we play with, pinned so everyone runs the same build.
BENILLA_REPO="https://github.com/samwhosung/benilla"
BENILLA_REV="c23d0163d5efa1ae39e4748877fe4ba831202848"
SETTINGS="$ROOT/settings.sh"
# This folder's own changes to the client (the action-combat mouse and the aiming camera for
# Elwynn Survival), applied on top of the pinned revision.
BENILLA_PATCHES="$ROOT/patches/benilla"
# Only its NAME=value lines: a line of anything else (Play once wrote "key = value" lines here)
# would stop every script that runs with set -e.
[ -f "$SETTINGS" ] && eval "$(tr -d '\r' < "$SETTINGS" | grep -E '^[A-Za-z_][A-Za-z0-9_]*=' || true)"
# A name from project.toml (the one place the project's player-facing names live), or the default.
project_name() { # key default
    local v
    v="$(sed -n "s/^[[:space:]]*$1[[:space:]]*=[[:space:]]*\"\([^\"]*\)\".*/\1/p" "$ROOT/project.toml" 2>/dev/null | head -1)"
    printf '%s' "${v:-$2}"
}
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"

save_setting() { # name value
    touch "$SETTINGS"
    grep -v "^$1=" "$SETTINGS" > "$SETTINGS.tmp" || true
    printf '%s=%q\n' "$1" "$2" >> "$SETTINGS.tmp"
    mv "$SETTINGS.tmp" "$SETTINGS"
}

find_wow_data() {
    if [ -n "${WOW_DATA:-}" ] && [ -f "$WOW_DATA/dbc.MPQ" ]; then return 0; fi
    for c in "$ROOT/../WoW Classic/WoW-1.12.1/Data" "$ROOT/WoW/Data" "$HOME/WoW-1.12.1/Data" "/Applications/World of Warcraft/Data" \
        "$HOME/Games/WoW-1.12.1/Data" "$HOME/Games/World of Warcraft/Data" "/opt/wow/Data" "/srv/wow/Data"; do
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
# (anything an earlier patch changed or added in them is put back first). That's every tracked
# file, not only crates/: patches edit docs/ too, and a change left there from the last build
# made the next one's patch fail to apply. What the player keeps (benilla-config/, target/) is
# ignored by the client's git, so neither step touches it.
apply_benilla_patches() {
    local dir="$ROOT/benilla" tops
    git -C "$dir" checkout -q -- .
    # Files an earlier patch added, in the folders the patches reach.
    tops="$(cat "$BENILLA_PATCHES"/*.patch 2>/dev/null | sed -n 's|^+++ b/\([^/]*\)/.*|\1|p' | sort -u)"
    # shellcheck disable=SC2086 # one word a folder
    [ -z "$tops" ] || git -C "$dir" clean -q -f -- $tops
    git -C "$dir" checkout -q "$BENILLA_REV"
    local p
    for p in "$BENILLA_PATCHES"/*.patch; do
        [ -f "$p" ] || continue
        git -C "$dir" apply --whitespace=nowarn "$p"
    done
}

# Build the client if it's missing, its patches changed since it was built, or it was built in
# another folder.
build_benilla_if_needed() {
    local stamp built_in
    stamp="$(patches_stamp)"
    built_in="$(cat "$ROOT/benilla/.htl-root" 2>/dev/null || true)"
    if [ -x "$ROOT/benilla/target/release/benilla" ] && [ "$(cat "$ROOT/benilla/.htl-patches" 2>/dev/null)" = "$stamp" ] \
        && [ "$built_in" = "$ROOT" ]; then
        return 0
    fi
    echo "Building the benilla client with this folder's changes (a few minutes after an update)..."
    apply_benilla_patches
    # The client remembers the folder it was built in, and keeps its settings and add-ons there
    # (benilla/benilla-config). Cargo doesn't notice a moved or copied folder, so a client
    # built elsewhere would keep using the old folder's: its package is cleaned and rebuilt here.
    if [ "$built_in" != "$ROOT" ]; then
        (cd "$ROOT/benilla" && cargo clean --release -q -p benilla) || true
    fi
    (cd "$ROOT/benilla" && cargo build --release -p benilla)
    echo "$stamp" > "$ROOT/benilla/.htl-patches"
    printf '%s\n' "$ROOT" > "$ROOT/benilla/.htl-root"
}
