#!/bin/bash
# Play Elwynn Survival: the benilla client into its realm (named in project.toml).
#   scripts/play-survival.sh                 the game hosted on this computer
#   scripts/play-survival.sh 192.168.1.20    a friend's game
HTL_GAME=survival exec "$(dirname "$0")/play.sh" "$@"
