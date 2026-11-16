# The Deadmines packs, in route order (audit, Nov 16)

Job 7 of the overnight list: for every pack between the bosses, what each creature does, how it reads
with the floor markers off, what answers it, and whether three to five of them together stack blows
past every answer. Read with [enemy-temperaments.md](enemy-temperaments.md) (who is what) and
[deadmines-balance.md](deadmines-balance.md) (the table; its pack rows fight one kind alone).

## The answers a hero has

- **Guard / parry**: stops a blade or a bolt marked `blockable` from the front. Costs stamina per block.
- **Roll** (15 stamina, 0.45 s, safe from 0.04 to 0.32 s): one roll answers every blow that lands inside
  its 0.28 s; a second blow 0.3 to 0.5 s later lands before a second roll can begin.
- **Dash** (4 yd in 0.25 s): clears a lobbed splash of up to 4 yd if started inside its flight.
- **Bash** a cast in reach, close the gap, or break the line of sight.
- Turns: one string at a time up close (`turn_beat`), no blade with a bolt in the air, nothing at a
  reeling hero (`waits_turn`). **Casters are throttled only by melee windups** (`caster_waits`): they
  cast in the gaps between blows up close, and take no turns among themselves.

## What each creature does

| Creature | What it does | How it reads with markers off | Its answer |
| --- | --- | --- | --- |
| Pirate (brawler) | two slashes (0.5, 0.4 s), lunge from 3.5 to 8 yd (0.6 s), feint (1.05 s, glowing cutlass) | the cutlass raised, the swing's trail, the cutlass glowing on the feint | parry the slashes, roll the feint and the lunge |
| Squallshaper (caster) | water bolt 1.2 s, two bolts, a squall all round (0.7 s, 4.2 yd, throws you down) when crowded | the staff gathering; the squall's own stamp and spell look | roll or block the bolt; the squall is a roll; step off the 4.2 yd |
| Crossbowman | one bolt, 1.8 s aim, 22 damage; a kick close in | the bow drawn back for 1.8 s | roll the bolt, rush it in the reload |
| Blackguard (backstabber) | flurry of 3 blows at 0.33 to 0.38 s and a 0.5 s finisher; lunge; strikes from behind | the dagger flashes; its voice is heard from behind | face it and hold the guard; the flank is read by turning |
| Woodcutter (bruiser) | two chops, a felling chop (1.0 s, 22, heavy) | the axe raised high | parry the chops, roll the felling chop |
| Engineer | a stick, 1.3 s, 2.6 yd splash, unblockable; two sticks; a wrench close in | the lit stick in the hand, then the stick in the air (0.7 s at least) | step or roll out of the ring |
| Miner / Strip Miner (coward) | two strikes; a pick held high (1.05 s, 22, heavy) | the pick overhead | parry, roll the heavy |
| Overseer (commander) | three cuts (the last 0.75 s, heavy), shield bash, lunge; 45% block | the shield up, the heavy cut's long raise | parry, roll the third cut; hit the back |
| Dynamiter | stick 1.2 s, 3 yd splash unblockable; three sticks; kick | lit sticks in the air | step out of each ring |
| Wizard | fireball 1.3 s (blockable bolt); frost ring (0.9 s, 4.5 yd, unblockable) when crowded | the staff, the bolt in flight; the ring's stamp | block or roll the bolt; step out of the ring |
| Evoker | shadow bolt 1.2 s; void burst 1.5 s, 3.5 yd, unblockable, a sound from behind | shadow gathered high, then the lob | block/roll the bolt; step or dash out of the burst |
| Craftsman | bomb 1.1 s, 3.5 yd, unblockable; wrench | the bomb in the air | step out |
| Taskmaster (commander) | lash (4.5 yd), crack (1.0 s, 24, heavy, 5 yd); at 50% two Strip Miners join | the whip raised | roll the crack; kill it first or fight a faster pack |

## The lobs and the dash

The small lobs (Engineer 2.6, Dynamiter 3.0, Evoker 3.5, Craftsman 3.5 yd; the Noble's 4.0 and Miner
Johnson's 3.5 are rares) are told by the thrown thing: it is let go at the spot marked in the wind-up
and spends at least 0.7 s in the air (`LOB_MIN_FLY`, pinned in `shared_fixes_tests.rs`). A hero who
reacts in 0.3 s has 0.4 s left; a 4 yd dash (0.25 s) clears 3.5 yd with half a yard and 0.15 s to
spare, and a 4.0 yd ring only just. **Judged fair, not changed**: a roll answers any radius (its safe
moment is on the hero, not on a distance), the dash is only the second answer, and the only 4.0 yd lob
is a rare's. Shrinking a ring or lengthening the flight would not move any pack's cost.

## Linked groups (the pulls as the route meets them)

The table's pack rows fight one kind at a time; a pull brings its neighbours (`link`). These are the
real groups, five seeds a cell (30 for the worst), a scripted hero that rolls or blocks every blow it
sees ("right") and one that misses half ("sloppy"). Rerun: `cargo test --release -p htl-survival
balance_groups -- --ignored --nocapture` (`BALANCE_ONLY=text`, `BALANCE_SEEDS=n`). "Peak stack" is the
most blows from different creatures due within 0.4 s of each other; "dry" is steps in which a roll was
wanted with too little stamina to start one.

Ranked, worst first:

| # | Group | Right play | Sloppy | Stack |
| --- | --- | --- | --- | --- |
| 1 | **Wizard x2 + Evoker** (the deck's third stretch) | warrior 24 s, 31%, **3 falls in 30**; hunter 54 s, 72%, **4 in 30**; mage 1 in 30 | warrior/hunter 0 to 2 in 30, 32 to 80% | 3 casters, no blade: peak 3, **dry 69 to 83 a run set** |
| 2 | Pirate x2 + Evoker x2 (the deck's foot) | hunter 37 s, 18%; priest 28 s | hunter 2 in 5, 84% | 3 within 0.4 s, dry 22 |
| 3 | Pirate x2 + Squallshaper + Wizard | hunter 41 s, 18%; priest 35 s, 12% | priest 140% (2.2 Draughts) | 3, dry 10 |
| 4 | Pirate x2 + Squallshaper x2 | priest 41 s, 19% | 76 to 99%, mage 1.6 Draughts | 2 to 3, dry 6 |
| 5 | Overseer + Miner x2 (+ Wizard) | 0 to 6% | **falls 3 in 5 (9 in 30 with the wizard)** | 2 |
| 6 | Blackguard x3 + Evoker | 0 to 3% | warrior 2 in 5, 58% | 3, dry 0 |
| 7 | Taskmaster + Strip Miner x3 | 2% | falls 1 to 2 in 5 | 1 to 2 |
| 8 | Engineer x2 | 0% | 5 to 16% | 2, answered |
| 9 | Pirate x2 + Squallshaper; Passage: Miner x2 + Dynamiter; Woodcutter x2 + Engineer; Pirate x2 + Woodcutter x2 | 0 to 2% | 17 to 69% | 1 to 2 |

Reading it:

- **Group 1 is the only one past its answers with the markers off and a right hero.** Three casters on
  three clocks, nothing up close to throttle them (`caster_waits` only waits for a *blade* winding up
  at the hero): a roll is 15 stamina and the third bolt finds it spent, so the bolts land on a hero who
  stands "Ready" with nothing left. Two casters (the Evoker pair, 0 falls, 14%) are fine; three are not.
  The survey's own note in `deadmines.toml` promises "guards to get through to it": the Blackguard
  pack has them (group 6, 0 to 3% right), this one had none.
- Groups 2 to 4 are long (30 to 40 s for a hero that must chase a keep-away caster) but answerable.
- Group 5 is a sloppy-play failure: a shield, the third cut's long raise and a wizard's frost ring. Each
  blow has an answer; nothing stacks past one.
- Group 6: the flurry (0.33 to 0.38 s a blow) is answered by holding the guard, not by rolls; the
  turn rule keeps one string at a time on a hero; the Evoker's burst has its lob tell. Fine.
- The Blackguard's flank is the one read that takes the whole body: its voice is heard from behind
  (`sim/mobs.rs`, "a blow from behind is heard").

## Tried and dropped

- **Casters keep out of each other's dead zone** (hold a cast landing 0.3 to 0.6 s from another's) and
  **a flat 0.8 s between casters' bolts**: both made group 1 worse (hunter 13 in 30 falls; warrior 4 in
  5): spacing the bolts leaves a hero who rolls each one no breath to swing in, and the fight runs
  longer. Not shipped.

## Fixed: Wizard x2 + Evoker

`content/deadmines.toml`: the wizard pack at (-132.4, -786.1) is one wizard, not two. With the evoker
that is two casters, as the Evoker pair is. Rerun, 30 seeds: right play, every class 0 falls in 30 and
1 to 5% of health (warrior 10 s, mage 8 s, priest 18 s, hunter 23 s; was 31% and 3 falls, 7% and 1,
26% and 0, 72% and 4); sloppy 14 to 35%, 0 falls (was 2 falls, hunter 80%). Tests in
`sim/balance_tests.rs`: `no_pull_is_three_casters_with_nobody_up_close` (fails on the old pack) and
`the_caster_pair_at_the_deck_is_won_by_a_right_playing_hero_of_every_class`.

The table's pack rows are rerun and unchanged but for one: Wizard x2 is gone, so "38, 43: Defias
Wizard x1" is now "38, 43, 46".

## Owner's call

- Whether casters should take turns among themselves in a pack (a design rule, not tuned here: both
  tries at it made the fight worse for the scripted hero).
- The scripted hero never raises a guard against a bolt (it rolls them); a real player can block the
  blockable ones, so group 1 was easier than the table says. The cut still stands: three unscreened
  casters is the outlier either way.
