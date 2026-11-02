#!/bin/bash
# Measure every character model's attack clips from your WoW 1.12.1 data: for each race and gender,
# every clip with an impact keyframe, each take: its length, when the blow lands, whether the body
# leaps, flips, spins or lunges, how the weapon hand strikes (overhead, chop, sweep, thrust,
# upswing) and what kind of blow it fits (flurry, cut, heavy, wide, gap closer, boss special).
#   scripts/clip-census.sh            the table, written to logs/clip-census.md
#   scripts/clip-census.sh --json     one JSON line a clip, to logs/clip-census.jsonl
# A model the Data folder lacks or can't be measured (the Goblin models may be missing from 1.12.1) is
# named on the terminal and skipped; the rest are measured.
# Creatures too: for each creature display id in a file (one a line, or separated by spaces or
# commas; # starts a comment), its model, scale and every attack-like clip: when it lands, how far
# the head moves into the blow, whether the body lunges, leaps, spins or rears first, and how big a
# wind-up a player sees before the blow (a "small" one is hard to read, as the snakes' are).
#   python3 tools/clip-census/displays.py > logs/displays.txt     every mob's display in content/
#   scripts/clip-census.sh --displays logs/displays.txt           to logs/creature-census.md
#   scripts/clip-census.sh --displays logs/displays.txt --json    to logs/creature-census.jsonl
#   scripts/clip-census.sh --find raptor    every display whose model path holds "raptor", on the
#                                           terminal (its output works as a --displays file too)
# It's numbers about the clips, not the clips themselves; still, keep it out of the repository
# (logs/ is ignored) and share it where it's needed.
set -euo pipefail
. "$(dirname "$0")/common.sh"
find_wow_data
mkdir -p "$ROOT/logs"
census() { cargo run -q --release --manifest-path "$ROOT/tools/clip-census/Cargo.toml" -- "$WOW_DATA" "$@"; }
json=
for a in "$@"; do [ "$a" = "--json" ] && json=1; done
case "${1:-}" in
    --find)
        census "$@"
        exit ;;
    --displays)
        [ -n "${2:-}" ] || { echo "usage: $0 --displays <file of display ids> [--json]" >&2; exit 2; }
        out="$ROOT/logs/creature-census.md"
        [ -n "$json" ] && out="$ROOT/logs/creature-census.jsonl" ;;
    *)
        out="$ROOT/logs/clip-census.md"
        [ -n "$json" ] && out="$ROOT/logs/clip-census.jsonl" ;;
esac
census "$@" > "$out"
echo "Written to $out"
