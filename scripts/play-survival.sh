#!/bin/bash
# Play Elwynn Survival: the benilla client into the "Elwynn Survival" realm.
#   scripts/play-survival.sh                 the game hosted on this computer
#   scripts/play-survival.sh 192.168.1.20    a friend's game
HTL_REALM="Elwynn Survival" exec "$(dirname "$0")/play.sh" "$@"
