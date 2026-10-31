#!/bin/bash
# Host the games on this computer: Basin Battle and Elwynn Survival, each on its own realm.
# Friends connect to it with your address and pick a game in their launcher.
#   scripts/host.sh                         password "hold"
#   scripts/host.sh --game basin            only Basin Battle (or --game survival)
#   scripts/host.sh --password secret       pick the password everyone uses
#   scripts/host.sh --public-host 1.2.3.4   your internet address (found automatically otherwise)
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
# Everything the server says also goes to a log file, for looking back at a session.
mkdir -p logs
log="logs/host-$(date +%Y-%m-%d-%H%M%S).log"
echo "This session's log: $log"
target/release/htl-server "${args[@]}" "$@" 2>&1 | tee "$log"
