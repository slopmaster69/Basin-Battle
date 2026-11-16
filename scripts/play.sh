#!/bin/bash
# Play: starts the benilla client pointed at a Basin Battle server.
#   scripts/play.sh                 the game hosted on this computer
#   scripts/play.sh 192.168.1.20    a friend's game
#   scripts/play-survival.sh [...]  the same, into Elwynn Survival (HTL_GAME=survival picks it;
#                                   HTL_REALM names a realm outright)
# Log in with any name and the host's password (default: hold), then pick a class.
set -euo pipefail
. "$(dirname "$0")/common.sh"
cd "$ROOT"
find_wow_data
host="${1:-${HTL_HOST:-127.0.0.1}}"
[ $# -ge 1 ] && save_setting HTL_HOST "$host"
if [ ! -x benilla/target/release/benilla ]; then
    echo "The client isn't built yet; running setup first."
    "$ROOT/scripts/setup-mac.sh"
fi
# This folder's client changes (Elwynn Survival's mouse and aiming camera): rebuilt once after
# they change.
build_benilla_if_needed
# A first game starts from the settings the Windows download ships: the user agreements already
# read, since an install without tos.html shows an empty Terms of Use that can't be accepted.
if [ ! -f benilla/benilla-config/config.toml ]; then
    mkdir -p benilla/benilla-config
    cp dist/windows/benilla-config/config.toml benilla/benilla-config/config.toml
fi
# A settings file the client wrote itself (a game started before this one, or by hand) may lack
# the agreements, and the Terms of Use then stops the game at a screen whose Accept never lights.
# Only those two keys are set; the player's mouse speed and the rest stay.
cfg=benilla/benilla-config/config.toml
for key in readEULA readTOS; do
    if ! grep -Eq "^[[:space:]]*$key[[:space:]]*=[[:space:]]*\"1\"" "$cfg"; then
        if grep -Eq "^[[:space:]]*$key[[:space:]]*=" "$cfg"; then
            awk -v k="$key" '$0 ~ "^[[:space:]]*" k "[[:space:]]*=" { print k " = \"1\""; next } { print }' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
        elif grep -q '^\[cvars\]' "$cfg"; then
            awk -v k="$key" '{ print } /^\[cvars\]/ { print k " = \"1\"" }' "$cfg" > "$cfg.tmp" && mv "$cfg.tmp" "$cfg"
        else
            printf '[cvars]\n%s = "1"\n' "$key" >> "$cfg"
        fi
    fi
done
# The add-ons (Basin Battle's card picker and trackers; Elwynn Survival's action layout and
# windows), fresh from this folder every launch. Each sleeps on the other's realm.
mkdir -p benilla/benilla-config/AddOns
rm -rf benilla/benilla-config/AddOns/BasinBattle benilla/benilla-config/AddOns/ElwynnSurvival
cp -R "$ROOT/addon/BasinBattle" "$ROOT/addon/ElwynnSurvival" benilla/benilla-config/AddOns/
# The client's memory of item and creature details: cleared so it always asks this server afresh.
rm -rf benilla/benilla-config/cache
cd benilla
# The realm to enter, by the name project.toml gives its game.
if [ "${HTL_GAME:-basin}" = "survival" ]; then realm="$(project_name survival "Elwynn Survival")"; else realm="$(project_name basin "Basin Battle")"; fi
WOW_DATA="$WOW_DATA" WOW_HOST="$host" WOW_REALM="${HTL_REALM:-$realm}" exec target/release/benilla
