#!/bin/bash
# Double-click to play: builds Play (the launcher, a few minutes the first time) and opens the app
# it makes in this folder ("<title>.app", project.toml's title). After that you can open the app
# itself. If Play can't be built, this plays the plain way instead: scripts/play.sh, the game
# hosted on this Mac.
cd "$(dirname "$0")" || exit 1
. scripts/common.sh
if bash scripts/mac-launcher.sh; then
    open "$(project_name title "Elwynn Survival").app"
    exit 0
fi
echo
echo "Play couldn't be built (the reason is above), so playing the plain way instead."
bash scripts/play.sh
