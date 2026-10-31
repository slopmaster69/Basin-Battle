#!/bin/bash
# Run the add-ons against real server lines in benilla's UI script host and print every Lua error.
#   scripts/check-addon.sh
# 1. A server test records every add-on line a client sees (picker, shop, fronts, map...; for
#    Elwynn Survival the HUD, windows and placing).
# 2. tools/addon-harness loads each add-on with stand-ins for the stock interface, replays those
#    lines and its script (tools/addon-harness/script.txt, survival.txt: more lines, Lua to run,
#    @EVENTS to fire), ticks its frames for ten seconds, and lists the errors and what the add-on
#    said to the server.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
LINES="$(mktemp)"
echo "=== Basin Battle ==="
ADDON_DUMP="$LINES" cargo test --release -q -p htl-server --test benilla_client the_addon_gets >/dev/null
cargo run --release -q --manifest-path "$ROOT/tools/addon-harness/Cargo.toml" -- \
  "$ROOT/addon/BasinBattle/BasinBattle.lua" "$LINES" "$ROOT/tools/addon-harness/script.txt"
echo "=== Elwynn Survival ==="
ADDON_DUMP="$LINES" cargo test --release -q -p htl-server --test survival_client a_hero_fights >/dev/null
cargo run --release -q --manifest-path "$ROOT/tools/addon-harness/Cargo.toml" -- \
  "$ROOT/addon/ElwynnSurvival/ElwynnSurvival.lua" "$LINES" "$ROOT/tools/addon-harness/survival.txt" ES
rm -f "$LINES"
