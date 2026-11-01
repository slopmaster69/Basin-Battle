#!/bin/bash
# Host the games on this computer: Basin Battle and Elwynn Survival, each on its own realm.
# Friends connect to it with your address and pick a game in their launcher.
#   scripts/host.sh                         password "hold"
#   scripts/host.sh --game basin            only Basin Battle (or --game survival)
#   scripts/host.sh --password secret       pick the password everyone uses
#   scripts/host.sh --public-host 1.2.3.4   your internet address (found automatically otherwise)
#   scripts/host.sh --board 1.2.3.4:3724    send wagers to a friend's Bookmaker's Board (or set
#                                           HTL_BOARD in settings.sh); without it, you keep it
set -euo pipefail
. "$(dirname "$0")/common.sh"
cd "$ROOT"
find_wow_data
# Rebuilds only what changed since last time: a few seconds, or a minute or two after an update.
cargo build --release -q -p htl-server
args=(--data "$WOW_DATA" --content content/game.toml --saves saves/basin-battle.toml --survival-content content/survival.toml --survival-save saves/elwynn-survival.toml)
if [[ " $* " != *" --public-host "* ]]; then
    # Friends on the internet are sent to your public address; find it unless it's given.
    public="${HTL_PUBLIC_HOST:-$(curl -s --max-time 4 https://api.ipify.org || true)}"
    if [[ "$public" =~ ^[0-9a-fA-F.:]+$ ]]; then
        args+=(--public-host "$public")
        lan="$(ipconfig getifaddr en0 2>/dev/null || hostname -I 2>/dev/null | awk '{print $1}' || true)"
        echo "Friends on the internet join with: $public   (your router must forward TCP ports 3724, 8085 and 8086 to this computer)"
        [ -n "$lan" ] && echo "Friends on your own network join with: $lan"
        tscli="$(command -v tailscale || true)"
        [ -z "$tscli" ] && [ -x /Applications/Tailscale.app/Contents/MacOS/Tailscale ] && tscli=/Applications/Tailscale.app/Contents/MacOS/Tailscale
        ts="$([ -n "$tscli" ] && "$tscli" ip -4 2>/dev/null | head -1 || true)"
        [ -n "$ts" ] && echo "Friends on your Tailscale network join with: $ts"
    fi
fi
# The Bookmaker's Board: this server keeps it for everyone unless HTL_BOARD (in settings.sh)
# names the friend's server that does ("1.2.3.4:3724"); HTL_BOARD_NAME is this camp's name on it.
[ -n "${HTL_BOARD:-}" ] && args+=(--board "$HTL_BOARD")
[ -n "${HTL_BOARD_NAME:-}" ] && args+=(--board-name "$HTL_BOARD_NAME")
# Everything the server says also goes to a log file, for looking back at a session. A warned
# restart (Restart Basin Battle.command) ends the server with code 75: then it's built again from
# what's in the folder now and started again, with a new log.
mkdir -p logs
while true; do
    log="logs/host-$(date +%Y-%m-%d-%H%M%S).log"
    echo "This session's log: $log"
    set +e
    target/release/htl-server "${args[@]}" "$@" 2>&1 | tee -a "$log"
    code=${PIPESTATUS[0]}
    set -e
    [ "$code" -eq 75 ] || exit "$code"
    echo "Restarting on the newest build..."
    cargo build --release -q -p htl-server || echo "The build failed: starting the last version that built."
done
