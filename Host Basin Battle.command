#!/bin/bash
# Double-click to host a game. Leave this window open while you play; close it to stop.
cd "$(dirname "$0")" || exit 1
bash scripts/host.sh "$@"
read -r -p "The server stopped. Press Return to close." _
