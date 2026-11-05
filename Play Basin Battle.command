#!/bin/bash
# Double-click to play: opens the War Table launcher (Basin Battle.app in this folder, built the
# first time, which takes a few minutes). After that you can open Basin Battle.app itself.
# If the launcher can't be built, this plays the plain way instead: scripts/play.sh, the game
# hosted on this Mac.
cd "$(dirname "$0")" || exit 1
if bash scripts/mac-launcher.sh; then
    open "Basin Battle.app"
    exit 0
fi
echo
echo "The launcher couldn't be built (the reason is above), so playing the plain way instead."
bash scripts/play.sh
