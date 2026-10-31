#!/bin/bash
# Draw Elwynn Survival's HUD as the harness lays it out (tools/addon-harness/snap.txt) to PNGs:
#   scripts/snap-addon.sh [out folder]
# A layout check, not the game's own drawing: the add-on's art as it is, the game's icons as
# labelled tiles, text in a near font.
set -euo pipefail
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT="${1:-$ROOT/target/snap}"
mkdir -p "$OUT"
sed "s#{OUT}#$OUT#g" "$ROOT/tools/addon-harness/snap.txt" > "$OUT/script.txt"
: > "$OUT/lines.txt"
HARNESS_SCREEN=1365,768 cargo run --release -q --manifest-path "$ROOT/tools/addon-harness/Cargo.toml" -- \
  "$ROOT/addon/ElwynnSurvival/ElwynnSurvival.lua" "$OUT/lines.txt" "$OUT/script.txt" ES | grep -i "error" || true
for j in "$OUT"/*.json; do
  python3 "$ROOT/tools/addon-harness/snap.py" "$j" "${j%.json}.png"
done
ls "$OUT"/*.png
