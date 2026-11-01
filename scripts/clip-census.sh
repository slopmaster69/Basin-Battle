#!/bin/bash
# Measure every character model's attack clips from your WoW 1.12.1 data: for each race and gender,
# every clip with an impact keyframe, each take: its length, when the blow lands, whether the body
# leaps, flips, spins or lunges, how the weapon hand strikes (overhead, chop, sweep, thrust,
# upswing) and what kind of blow it fits (flurry, cut, heavy, wide, gap closer, boss special).
#   scripts/clip-census.sh            the table, written to logs/clip-census.md
#   scripts/clip-census.sh --json     one JSON line a clip, to logs/clip-census.jsonl
# A model the Data folder lacks or can't be measured (the Goblin models may be missing from 1.12.1) is
# named on the terminal and skipped; the rest are measured.
# It's numbers about the clips, not the clips themselves; still, keep it out of the repository
# (logs/ is ignored) and share it where it's needed.
set -euo pipefail
. "$(dirname "$0")/common.sh"
find_wow_data
mkdir -p "$ROOT/logs"
out="$ROOT/logs/clip-census.md"
[ "${1:-}" = "--json" ] && out="$ROOT/logs/clip-census.jsonl"
cargo run -q --release --manifest-path "$ROOT/tools/clip-census/Cargo.toml" -- "$WOW_DATA" "$@" > "$out"
echo "Written to $out"
