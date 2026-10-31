#!/bin/bash
# Play: starts the benilla client pointed at a Basin Battle server.
#   scripts/play.sh                 the game hosted on this computer
#   scripts/play.sh 192.168.1.20    a friend's game
#   scripts/play-survival.sh [...]  the same, into Elwynn Survival (HTL_REALM picks the realm)
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
# The add-ons (Basin Battle's card picker and trackers; Elwynn Survival's action layout and
# windows), fresh from this folder every launch. Each sleeps on the other's realm.
mkdir -p benilla/benilla-config/AddOns
rm -rf benilla/benilla-config/AddOns/BasinBattle benilla/benilla-config/AddOns/ElwynnSurvival
cp -R "$ROOT/addon/BasinBattle" "$ROOT/addon/ElwynnSurvival" benilla/benilla-config/AddOns/
# The client's memory of item and creature details: cleared so it always asks this server afresh.
rm -rf benilla/benilla-config/cache
cd benilla
WOW_DATA="$WOW_DATA" WOW_HOST="$host" WOW_REALM="${HTL_REALM:-Basin Battle}" exec target/release/benilla
