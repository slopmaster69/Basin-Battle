# Deadmines balance table

Measured Nov 16 2026 on `main` at f22fdce, for the overnight balance pass (Job 3, part one: measure
and report; nothing here was tuned). The harness is `crates/htl-survival/src/sim/balance_tests.rs`.

## What was measured

Every Deadmines boss and every pack on the route between them, fought by a scripted hero of each
class (warrior, mage, priest, hunter), two ways:

- **Plays it right**: answers every blow it sees (parries a blade it can block, rolls a heavy blow, a
  bolt or a gunner's locked aim, bashes a cast within reach), waits out a boss's string, keeps a
  roll's stamina back. 0.30 s reaction, 0.15 s for a string's follow-up (`tests_fair`).
- **Sloppy**: the same, but misses half of those answers (a blow it never answers) and swings when it
  shouldn't.

Both are the fighter `tests_fair_hero` already had for the warrior, grown by what Job 3 asked for:

- the class's own two starting cards (Q and E: the "ranged" and "close" rows), cast whenever they
  are ready and in reach (a full charge when nothing is within 6 yd, the least charge point blank,
  since a full one never finishes under blows);
- the real numbers a server runs: `Content::builtin()`, not `Content::dev()` (dev takes the foes'
  damage back to 1.0; the real `foe_damage` is 1.35);
- a hero with the real 200 health, 3 Draughts of 40% (drunk below half health, when nothing seen
  lands during the 1.4 s drink; the sloppy hero drinks regardless), the starting axe and buckler;
- an end: a fight is **won** when the boss (or the pack) is down, **fell** when the hero reaches 0
  health (the run ends there), **stalled** if still going at 420 s;
- damage taken is every point of health lost between steps (blows, burns, rubble, a shove), not
  only blows, so a heal or a Draught does not take it back.

Seeds per cell: 10 for a boss, 5 for a pack (seeds change the world's dice and the hero's misreads).
Each class is a different cell because only its two cards differ; the weapon and armour are the
same for all. Hunter is measured with its arrow and Call of the Wild card, its pet not modelled
(Hunter's numbers are not touched by anything here).

### Cell format

`mean seconds (mean seconds of the wins) · damage taken as a share of the hero's 200 health · Draughts
drunk · falls/runs · wins/runs`. Over 100% damage means Draughts or luck kept the hero up (3
Draughts are 120%).

### What is not measured

- **Gilnid's pole.** Neither engineer is struck down; the plank comes down on the furnace's
  demonstration or at the sixth stick. A hero who strikes the pole would be faster.
- **Rhahk'Zor's collapse.** His phases run as they are except the run through the falling mine and
  its slam (they need the real floor); his maul, his rocks and the last stand are measured. The
  collapse adds the gauntlet, so the real fight is longer than the number here.
- **VanCleef's Unbowed and Cookie's Gorged**: secret forms, not part of the plain fight.
- **Lamp rests between fights**, and a real player's mistakes beyond the half-missed answers. The
  scripted hero never misses a swing's uptime or walks the wrong way, so **a human's times will be
  longer than these**; read the table for what is far out and for the order, not for the second.
- A hero who falls ends the run: the table has no "fell, rested, came back" time.
- More than one hero, and the pulls where a pack brings another with it (`pulls.rs`).

## Bosses (route order)

Each cell: mean seconds (mean seconds of the wins) · damage taken as a share of the hero's health · Draughts drunk · falls/runs · wins/runs.

| fight | class | plays it right | sloppy |
| --- | --- | --- | --- |
| Rhahk'Zor | warrior | 105 s (wins 105) · 71% · 1.0 · 0/10 · 10/10 | 33 s (no win) · 189% · 2.2 · 10/10 · 0/10 |
|  | mage | 90 s (wins 90) · 34% · 0.1 · 0/10 · 10/10 | 42 s (no win) · 198% · 2.4 · 10/10 · 0/10 |
|  | priest | 152 s (wins 159) · 165% · 1.3 · 1/10 · 9/10 | 32 s (no win) · 206% · 2.4 · 10/10 · 0/10 |
|  | hunter | 98 s (wins 105) · 95% · 1.0 · 2/10 · 8/10 | 26 s (no win) · 146% · 1.2 · 10/10 · 0/10 |
| Sneed and the Shredder | warrior | 65 s (wins 65) · 59% · 1.0 · 0/10 · 10/10 | 58 s (wins 59) · 153% · 2.6 · 1/10 · 9/10 |
|  | mage | 69 s (wins 69) · 60% · 1.0 · 0/10 · 10/10 | 67 s (wins 72) · 170% · 2.7 · 3/10 · 7/10 |
|  | priest | 81 s (wins 81) · 91% · 0.7 · 0/10 · 10/10 | 68 s (wins 70) · 207% · 2.8 · 1/10 · 9/10 |
|  | hunter | 76 s (wins 76) · 84% · 1.3 · 0/10 · 10/10 | 67 s (wins 69) · 174% · 2.8 · 1/10 · 9/10 |
| Gilnid | warrior | 58 s (wins 58) · 6% · 0.0 · 0/10 · 10/10 | 56 s (wins 57) · 174% · 2.9 · 2/10 · 8/10 |
|  | mage | 72 s (wins 72) · 4% · 0.0 · 0/10 · 10/10 | 62 s (wins 66) · 183% · 2.8 · 3/10 · 7/10 |
|  | priest | 85 s (wins 85) · 7% · 0.0 · 0/10 · 10/10 | 68 s (wins 69) · 214% · 2.9 · 2/10 · 8/10 |
|  | hunter | 70 s (wins 70) · 10% · 0.0 · 0/10 · 10/10 | 61 s (wins 61) · 165% · 2.5 · 1/10 · 9/10 |
| Smite | warrior | 59 s (wins 59) · 0% · 0.0 · 0/10 · 10/10 | 38 s (wins 60) · 203% · 2.6 · 9/10 · 1/10 |
|  | mage | 71 s (wins 71) · 0% · 0.0 · 0/10 · 10/10 | 38 s (wins 70) · 201% · 2.6 · 9/10 · 1/10 |
|  | priest | 74 s (wins 74) · 0% · 0.0 · 0/10 · 10/10 | 45 s (wins 72) · 230% · 2.7 · 9/10 · 1/10 |
|  | hunter | 61 s (wins 61) · 0% · 0.0 · 0/10 · 10/10 | 42 s (wins 60) · 209% · 2.8 · 8/10 · 2/10 |
| Greenskin | warrior | 54 s (wins 54) · 41% · 0.2 · 0/10 · 10/10 | 48 s (wins 52) · 176% · 2.6 · 5/10 · 5/10 |
|  | mage | 65 s (wins 65) · 95% · 1.5 · 0/10 · 10/10 | 51 s (wins 62) · 215% · 3.0 · 9/10 · 1/10 |
|  | priest | 55 s (wins 55) · 71% · 0.0 · 0/10 · 10/10 | 51 s (wins 60) · 254% · 3.0 · 9/10 · 1/10 |
|  | hunter | 49 s (wins 49) · 26% · 0.2 · 0/10 · 10/10 | 48 s (wins 47) · 193% · 2.7 · 5/10 · 5/10 |
| VanCleef | warrior | 66 s (wins 66) · 0% · 0.0 · 0/10 · 10/10 | 54 s (wins 54) · 149% · 2.4 · 0/10 · 10/10 |
|  | mage | 88 s (wins 88) · 0% · 0.0 · 0/10 · 10/10 | 65 s (wins 69) · 196% · 2.9 · 6/10 · 4/10 |
|  | priest | 86 s (wins 86) · 0% · 0.0 · 0/10 · 10/10 | 73 s (wins 69) · 215% · 2.6 · 2/10 · 8/10 |
|  | hunter | 77 s (wins 77) · 0% · 0.0 · 0/10 · 10/10 | 60 s (wins 59) · 181% · 2.8 · 3/10 · 7/10 |
| Cookie | warrior | 25 s (wins 25) · 0% · 0.0 · 0/10 · 10/10 | 21 s (wins 21) · 48% · 0.6 · 0/10 · 10/10 |
|  | mage | 32 s (wins 32) · 0% · 0.0 · 0/10 · 10/10 | 30 s (wins 30) · 82% · 1.2 · 0/10 · 10/10 |
|  | priest | 35 s (wins 35) · 0% · 0.0 · 0/10 · 10/10 | 30 s (wins 30) · 87% · 0.9 · 0/10 · 10/10 |
|  | hunter | 31 s (wins 31) · 0% · 0.0 · 0/10 · 10/10 | 27 s (wins 27) · 72% · 1.0 · 0/10 · 10/10 |

## Packs between them (the same pack in several places is fought once; the numbers are the pack numbers in `deadmines.toml`)

Each cell: mean seconds (mean seconds of the wins) · damage taken as a share of the hero's health · Draughts drunk · falls/runs · wins/runs.

| fight | class | plays it right | sloppy |
| --- | --- | --- | --- |
| 1: Defias Pirate x1 | warrior | 4 s (wins 4) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 9% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 4% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 12% · 0.0 · 0/5 · 5/5 |
|  | priest | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 4 s (wins 4) · 10% · 0.0 · 0/5 · 5/5 |
|  | hunter | 5 s (wins 5) · 2% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 4% · 0.0 · 0/5 · 5/5 |
| 2, 4, 7, 10, 13, 48, 50, 51: Defias Pirate x2 | warrior | 9 s (wins 9) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 29% · 0.0 · 0/5 · 5/5 |
|  | mage | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 14% · 0.0 · 0/5 · 5/5 |
|  | priest | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 9 s (wins 8) · 52% · 0.0 · 1/5 · 4/5 |
|  | hunter | 10 s (wins 10) · 0% · 0.0 · 0/5 · 5/5 | 9 s (wins 9) · 31% · 0.0 · 0/5 · 5/5 |
| 3, 8: Defias Squallshaper x1 | warrior | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 7% · 0.0 · 0/5 · 5/5 |
|  | mage | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 8 s (wins 8) · 7% · 0.0 · 0/5 · 5/5 |
|  | priest | 9 s (wins 9) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 12% · 0.0 · 0/5 · 5/5 |
|  | hunter | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 9% · 0.0 · 0/5 · 5/5 |
| 6: Defias Crossbowman x1 | warrior | 4 s (wins 4) · 0% · 0.0 · 0/5 · 5/5 | 4 s (wins 4) · 6% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 |
|  | priest | 11 s (wins 11) · 0% · 0.0 · 0/5 · 5/5 | 8 s (wins 8) · 9% · 0.0 · 0/5 · 5/5 |
|  | hunter | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 |
| 14, 22: Goblin Woodcutter x2 | warrior | 7 s (wins 7) · 2% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 11% · 0.0 · 0/5 · 5/5 |
|  | mage | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 8 s (wins 8) · 13% · 0.0 · 0/5 · 5/5 |
|  | priest | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 14% · 0.0 · 0/5 · 5/5 |
|  | hunter | 10 s (wins 10) · 0% · 0.0 · 0/5 · 5/5 | 11 s (wins 11) · 20% · 0.0 · 0/5 · 5/5 |
| 15, 20, 21, 23: Goblin Engineer x1 | warrior | 4 s (wins 4) · 0% · 0.0 · 0/5 · 5/5 | 4 s (wins 4) · 0% · 0.0 · 0/5 · 5/5 |
|  | mage | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 |
|  | priest | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 |
|  | hunter | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 3% · 0.0 · 0/5 · 5/5 |
| 16, 18, 19, 27: Defias Blackguard x1 | warrior | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 8% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 9% · 0.0 · 0/5 · 5/5 |
|  | priest | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 11% · 0.0 · 0/5 · 5/5 |
|  | hunter | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 9% · 0.0 · 0/5 · 5/5 |
| 25, 29, 30, 33: Defias Miner x2 | warrior | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 3% · 0.0 · 0/5 · 5/5 |
|  | mage | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 7% · 0.0 · 0/5 · 5/5 |
|  | priest | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 8 s (wins 8) · 15% · 0.0 · 0/5 · 5/5 |
|  | hunter | 11 s (wins 11) · 0% · 0.0 · 0/5 · 5/5 | 11 s (wins 11) · 32% · 0.0 · 0/5 · 5/5 |
| 26, 31: Defias Dynamiter x1 | warrior | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 3% · 0.0 · 0/5 · 5/5 |
|  | priest | 14 s (wins 14) · 0% · 0.0 · 0/5 · 5/5 | 10 s (wins 10) · 18% · 0.0 · 0/5 · 5/5 |
|  | hunter | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 3% · 0.0 · 0/5 · 5/5 |
| 28, 32: Defias Overseer x1 | warrior | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 18% · 0.0 · 0/5 · 5/5 |
|  | mage | 9 s (wins 9) · 2% · 0.0 · 0/5 · 5/5 | 12 s (wins 10) · 58% · 0.0 · 2/5 · 3/5 |
|  | priest | 8 s (wins 8) · 2% · 0.0 · 0/5 · 5/5 | 13 s (wins 7) · 80% · 0.0 · 4/5 · 1/5 |
|  | hunter | 10 s (wins 10) · 3% · 0.0 · 0/5 · 5/5 | 9 s (wins 9) · 9% · 0.0 · 0/5 · 5/5 |
| 38, 43, 46: Defias Wizard x1 | warrior | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 10% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 5% · 0.0 · 0/5 · 5/5 |
|  | priest | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 7% · 0.0 · 0/5 · 5/5 |
|  | hunter | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 5% · 0.0 · 0/5 · 5/5 |
| 39: Defias Evoker x2 | warrior | 15 s (wins 15) · 5% · 0.0 · 0/5 · 5/5 | 10 s (wins 10) · 21% · 0.0 · 0/5 · 5/5 |
|  | mage | 7 s (wins 7) · 2% · 0.0 · 0/5 · 5/5 | 8 s (wins 8) · 16% · 0.0 · 0/5 · 5/5 |
|  | priest | 24 s (wins 24) · 7% · 0.0 · 0/5 · 5/5 | 12 s (wins 12) · 39% · 0.2 · 0/5 · 5/5 |
|  | hunter | 24 s (wins 24) · 14% · 0.0 · 0/5 · 5/5 | 13 s (wins 13) · 25% · 0.0 · 0/5 · 5/5 |
| 40: Goblin Craftsman x1 | warrior | 4 s (wins 4) · 0% · 0.0 · 0/5 · 5/5 | 4 s (wins 4) · 5% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 5% · 0.0 · 0/5 · 5/5 |
|  | priest | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 13% · 0.0 · 0/5 · 5/5 |
|  | hunter | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 |
| 41: Defias Taskmaster x1 | warrior | 12 s (wins 12) · 0% · 0.0 · 0/5 · 5/5 | 14 s (wins 14) · 24% · 0.0 · 0/5 · 5/5 |
|  | mage | 18 s (wins 18) · 0% · 0.0 · 0/5 · 5/5 | 18 s (wins 17) · 54% · 0.2 · 1/5 · 4/5 |
|  | priest | 21 s (wins 21) · 2% · 0.0 · 0/5 · 5/5 | 33 s (wins 33) · 41% · 0.0 · 0/5 · 5/5 |
|  | hunter | 12 s (wins 12) · 6% · 0.0 · 0/5 · 5/5 | 11 s (wins 11) · 13% · 0.0 · 0/5 · 5/5 |
| 42: Defias Strip Miner x3 | warrior | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 1% · 0.0 · 0/5 · 5/5 |
|  | mage | 7 s (wins 7) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 4% · 0.0 · 0/5 · 5/5 |
|  | priest | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 9% · 0.0 · 0/5 · 5/5 |
|  | hunter | 8 s (wins 8) · 0% · 0.0 · 0/5 · 5/5 | 9 s (wins 9) · 12% · 0.0 · 0/5 · 5/5 |
| 44: Defias Blackguard x3 | warrior | 13 s (wins 13) · 4% · 0.0 · 0/5 · 5/5 | 12 s (wins 13) · 60% · 0.0 · 2/5 · 3/5 |
|  | mage | 14 s (wins 14) · 25% · 0.0 · 0/5 · 5/5 | 12 s (wins 11) · 56% · 0.0 · 2/5 · 3/5 |
|  | priest | 13 s (wins 13) · 6% · 0.0 · 0/5 · 5/5 | 11 s (wins 12) · 66% · 0.0 · 2/5 · 3/5 |
|  | hunter | 13 s (wins 13) · 0% · 0.0 · 0/5 · 5/5 | 14 s (wins 15) · 69% · 0.2 · 2/5 · 3/5 |
| 45, 47: Defias Evoker x1 | warrior | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 7% · 0.0 · 0/5 · 5/5 |
|  | mage | 6 s (wins 6) · 2% · 0.0 · 0/5 · 5/5 | 6 s (wins 6) · 5% · 0.0 · 0/5 · 5/5 |
|  | priest | 5 s (wins 5) · 0% · 0.0 · 0/5 · 5/5 | 5 s (wins 5) · 5% · 0.0 · 0/5 · 5/5 |
|  | hunter | 6 s (wins 6) · 0% · 0.0 · 0/5 · 5/5 | 7 s (wins 7) · 2% · 0.0 · 0/5 · 5/5 |
| 49: Defias Squallshaper x2 | warrior | 12 s (wins 12) · 2% · 0.0 · 0/5 · 5/5 | 10 s (wins 10) · 32% · 0.2 · 0/5 · 5/5 |
|  | mage | 12 s (wins 12) · 0% · 0.0 · 0/5 · 5/5 | 11 s (wins 11) · 7% · 0.0 · 0/5 · 5/5 |
|  | priest | 25 s (wins 25) · 5% · 0.0 · 0/5 · 5/5 | 14 s (wins 14) · 35% · 0.2 · 0/5 · 5/5 |
|  | hunter | 18 s (wins 18) · 3% · 0.0 · 0/5 · 5/5 | 15 s (wins 15) · 23% · 0.0 · 0/5 · 5/5 |

## Against the targets

The owner's targets (Job 3, changeable): a hero who plays it right wins each boss in 1.5 to 3 minutes
and rarely falls; a sloppy hero falls more often than they win; no class is more than 1.5 times
slower than the fastest on any boss; no boss takes longer than VanCleef.

| Target | Result | Verdict |
| --- | --- | --- |
| Right play wins in 90 to 180 s | Rhahk'Zor 90 to 159 s (was 133 to 221 s, health 1900 to 1300 in `tune-rhahkzor`); every other boss 25 to 88 s (warrior's time: Shredder 65, Gilnid 58, Smite 59, Greenskin 54, VanCleef 66, Cookie 25) | **Miss**: six of seven bosses are under 90 s for every class, by 2 to 45% (Cookie by 65 to 70%). Rhahk'Zor's priest is now under 180 s |
| Right play rarely falls | Only Rhahk'Zor, now warrior 0/10, mage 0/10, priest 1/10 (165% of its health taken), hunter 2/10 (was mage 1, hunter 2, priest 3 of 10 at 1900 health). No other boss makes a right hero fall | **Miss** for Rhahk'Zor's hunter only (Hunter numbers are not touched) |
| Sloppy falls more than wins | Rhahk'Zor 10/10 falls (all four classes), Smite 8 or 9/10, Greenskin mage and priest 9/10, VanCleef mage 6/10. But Shredder 1 to 3/10, Gilnid 1 to 3/10, VanCleef warrior 0/10, priest 2/10, hunter 3/10, Cookie 0/10, Greenskin warrior and hunter 5/10 | **Miss** on Shredder, Gilnid, VanCleef, Cookie: a hero who misses half the answers still wins with 3 Draughts (Shredder 7 to 9/10, Gilnid 7 to 9/10, VanCleef 4 to 10/10, Cookie 10/10) |
| No class more than 1.5x the fastest | Rhahk'Zor **1.77x** by the wins (mage 90 s, priest 159 s; was 1.66x at 1900 health); Gilnid 1.47x (warrior 58, priest 85); Cookie 1.40x; Greenskin and VanCleef 1.33x; Shredder and Smite 1.25x | **Miss** only on Rhahk'Zor (priest): the spread is how the class plays him, not a number (a smaller health scales every class alike, and the mage gained most); Gilnid is at the edge |
| No boss takes longer than VanCleef | VanCleef 66 to 88 s. Rhahk'Zor is now 1.2 to 1.8x that (90 to 159 s; was 2 to 2.5x) | **Miss**, narrower: Rhahk'Zor still outlasts VanCleef (the real fight adds the collapse run);  Greenskin, Smite, Shredder and Gilnid are shorter than VanCleef, as wanted |

What else stands out:

- **Priest is the slowest class on Rhahk'Zor (159 s), the Shredder (81 s), Gilnid (85 s), Smite
  (74 s) and Cookie (35 s)**; the mage is on Greenskin (65 s) and VanCleef (88 s, the priest 86 s).
  The priest also takes by far the most on Rhahk'Zor (165% of its health). Its two starting cards (Penance and Holy Nova) do little damage next to the axe; nothing here tunes
  them (a class card is not a boss number).
- **Mage takes 95% of its health on Greenskin** (1.5 Draughts) against the warrior's 41% and the hunter's 26%: the 1.5 s
  Fireball charge under a gunner's locked aim.
- **The packs cost a right hero almost nothing.** Every pack of the route is a 4 to 25 s fight and
  under 15% of health, bar Blackguard x3 for the mage (25%); most are 0%. A sloppy hero falls in a
  pack only against the Overseer (mage 2/5, priest 4/5), Blackguard x3 (2/5 for every class), the
  Taskmaster (mage 1/5) and the Pirate pairs (priest 1/5 in one). Nothing in the packs is far from
  a target; whether a pack of 3 stacks blows with no answer is [deadmines-packs.md](deadmines-packs.md)'s (linked groups), not this table's; it found Wizard x2 + Evoker did, and pack 46 is now one wizard.
- **Cookie is by far the shortest boss**: 25 to 35 s, and a sloppy hero hardly takes a scratch
  (21 to 30 s, 48 to 87% of its health, never falls). His pot is the design, not the numbers; see
  the owner's call below.
- **Sneed and the Shredder** take 59 to 91% of a right hero's health and about one Draught (the
  priest 91%): with Rhahk'Zor and Greenskin, the fights where the damage taken, not the time, is the
  pressure.
- A scripted hero that never misses a swing makes every time here **a lower bound**: the fights
  that are 20% under 90 s may well be at 90 s for a person. Read "far outside" below with that.

## Tuning proposals

All are health numbers in `content/deadmines.toml`, chosen so a warrior who plays right lands near
100 s and the slowest class stays under 180 s, scaled from time being close to proportional to
health (it is not exact: phases and openings do not scale). **Every one is the owner's call**:
they either move six of seven bosses by 40% to 3.4x (far outside the targets) or change what a
fight is for. They are first guesses to rerun the table on, not settled numbers.

| Boss | Line | From | To | Why |
| --- | --- | --- | --- | --- |
| Rhahk'Zor | 2090 `health` | 1900 | 1300 | **Done in `claude/tune-rhahkzor`** (rows above are the rerun). Was priest 221 s, 3/10 falls; mage 133 s. Now warrior 105 s, priest 152 s (159 s for the wins), falls 0, 0, 1, 2 of 10. The collapse (not measured) adds to the real fight |
| VanCleef | 807 `health` | 1500 | 2600 | Final boss, 66 to 88 s: shorter than the boss before it by half (target: nothing longer than him). Warrior to about 115 s, mage to about 153 s |
| Smite | 606 `health` | 1100 | 1700 | 59 to 74 s: below 90 s for all four classes. Warrior to about 91 s |
| Greenskin | 706 `health` | 1200 | 2000 | 49 to 65 s. Warrior to about 90 s. The gunners (85 health, line 774) die first and stay as they are |
| Gilnid | 1444 `health` | 1300 | 2100 | 58 to 85 s (and faster still when the pole is struck, which the table does not do). Warrior to about 94 s |
| The Shredder | 1530 `health` | 950 | 1500 | 65 to 81 s. Warrior to about 103 s. Sneed's own (line 1588, 340 health) goes with it, to 540, so the second half keeps its share |
| Cookie | 538 `health` | 650 | 2400 | 25 to 35 s. Needs 3.7x to reach 90 s: **far outside the target, and the design is a short fight about his pot; the owner decides whether it should be longer at all** |

Not proposed, because they are not a number: Rhahk'Zor's priest and hunter falling (the priest's
cards), the sloppy hero who still wins at Shredder, Gilnid, VanCleef and Cookie (Draughts: 3 at 40%
is 120% of a 200-health hero; fewer or smaller Draughts, or a lamp's refill rule, would make misses
cost, which is a design change), and the class spread at Rhahk'Zor (the priest's damage).

## The owner's call

Not tuned, and not for a tuning pull request:

- **Raising the other bosses' health** (the table above), which would make VanCleef the longest boss.
- **Cookie's length** (25 to 35 s): his pot is the design.
- **Sloppy heroes still winning most fights** at Shredder, Gilnid, VanCleef and Cookie.
- **Draughts covering 120% of health** (3 at 40% on a 200-health hero).
- **Rhahk'Zor's hunter still falling 2 in 10** and the class spread there (priest 159 s, mage 90 s): the
  hunter's and the priest's own damage, not a boss number.

## Rerun it

The long form, under a minute in a release build (the cores are used); the output is these tables:

```sh
cargo test --release -p htl-survival balance_table -- --ignored --nocapture
```

Narrow it: `BALANCE_ONLY=Smite` keeps the fights whose name has that text, `BALANCE_CLASSES=warrior,mage`
the classes, `BALANCE_SEEDS=20` the seeds per cell. One fight told moment by moment (`BALANCE_FOES` is
a boss's creature key, or comma-separated creature keys for a pack; `BALANCE_SLOPPY=1` for the
sloppy hero; `BALANCE_TRACE=1` prints every wind-up and where everyone stands every 5 s):

```sh
BALANCE_FOES=dm_smite BALANCE_CLASS=mage BALANCE_SEED=3 BALANCE_TRACE=1 \
  cargo test --release -p htl-survival balance_one -- --ignored --nocapture
```

The table is an `#[ignore]`d test because it plays about 1,300 fights; a quick one, `a_scripted_hero_beats_one_pirate_and_a_pack_is_fought_to_its_end`,
runs with the ordinary tests so the harness cannot rot. After a tuning pull request, rerun the table
and put the changed rows in its description beside these.
