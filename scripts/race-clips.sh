#!/bin/bash
# List every character model's attack clips from your WoW 1.12.1 data: for each race and gender,
# each clip's takes, how long each runs and when its blow lands (its impact keyframe). This is the
# menu a move's or a creature's `from` picks from (content/*.toml); try one in game with
# `/esclip <clip> <take> <Model>` before naming it.
#   scripts/race-clips.sh                 every attack clip, every model
#   scripts/race-clips.sh ScourgeFemale   one model
#   scripts/race-clips.sh "" 17 57        only these clip ids
# Needs the benilla checkout (scripts/play.sh or setup makes it) and your Data folder.
set -euo pipefail
. "$(dirname "$0")/common.sh"
find_wow_data
if [ ! -d "$ROOT/benilla/.git" ]; then
    git clone -q "$BENILLA_REPO" "$ROOT/benilla"
fi
apply_benilla_patches
(cd "$ROOT/benilla" && cargo build -q --release -p benilla-formats --bin benilla-extract)
EXTRACT="$ROOT/benilla/target/release/benilla-extract"

ONLY="${1:-}"
shift || true
# The clips worth reading as blows, with their AnimationData names.
declare -A NAME=(
    [16]=AttackUnarmed [17]=Attack1H [18]=Attack2H [19]=Attack2HL [30]=Dodge [37]=JumpStart
    [38]=Jump [39]=JumpEnd [55]=BattleRoar [57]=Special1H [58]=Special2H [59]=ShieldBash
    [85]=Pierce [118]=SpecialUnarmed [143]=Sprint
)
IDS=("$@")
[ ${#IDS[@]} -eq 0 ] && IDS=(16 17 18 19 57 58 59 85 88 118 55 30 37 38 39 143)

printf '| Model | Clip | Take | Length (s) | Blow lands (s) |\n| --- | --- | --- | --- | --- |\n'
for race in Human Orc Dwarf NightElf Scourge Tauren Gnome Troll Goblin; do
    for gender in Male Female; do
        model="$race$gender"
        [ -n "$ONLY" ] && [ "$ONLY" != "$model" ] && continue
        path="Character\\$race\\$gender\\$model.m2"
        # One line a sequence with events: its index, anim id, length and keyframes. A sequence
        # without events has no impact: m2seq lists every take, so read the takes from it.
        "$EXTRACT" "$WOW_DATA" m2seq "$path" 2>/dev/null | awk -v model="$model" -v ids="${IDS[*]}" '
            BEGIN { n = split(ids, want, " "); for (i = 1; i <= n; i++) keep[want[i]] = 1 }
            NR > 1 && ($2 in keep) { take[$2]++; print model, $1, $2, take[$2] - 1, $4 }
        ' > /tmp/race-clips.$$ || true
        "$EXTRACT" "$WOW_DATA" m2events "$path" 2>/dev/null > /tmp/race-clips-ev.$$ || true
        while read -r m idx id tk dur; do
            impact=$(awk -v idx="$idx" '$1 == idx { for (i = 1; i <= NF; i++) if ($i ~ /^\$AH[0-3]$|^\$CAH$/) { sub(/s$/, "", $(i-1)); print $(i-1); exit } }' /tmp/race-clips-ev.$$)
            printf '| %s | %s %s | %s | %s | %s |\n' "$m" "$id" "${NAME[$id]:-}" "$tk" "$dur" "${impact:--}"
        done < /tmp/race-clips.$$
        rm -f /tmp/race-clips.$$ /tmp/race-clips-ev.$$
    done
done
