#!/bin/bash
# Double-click to restart the running server (after an update, say) without dropping anyone
# unwarned: everyone playing sees a countdown, the worlds are saved, and the host window builds
# the newest version and starts it again. Give it a number of minutes in Terminal
# ("Restart Basin Battle.command" 5), or "cancel" to call a restart off. Two minutes otherwise.
cd "$(dirname "$0")" || exit 1
what="${1:-2}"
echo "$what" > restart.request
if [ "$what" = "cancel" ]; then
    echo "Asked the server to call off its restart."
else
    echo "Asked the server to restart in $what minutes. Everyone playing gets a warning; the host window"
    echo "builds the newest version and starts it again by itself."
fi
echo "(If no host window is running, nothing happens: start one with Host Basin Battle.)"
sleep 4
