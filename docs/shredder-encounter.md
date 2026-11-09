# Sneed's Shredder, then Sneed: the machine and the coward

Corey's revised design (Nov 9). The machine's three moves each look and sound like themselves, its
grind is a plant, a rev and a drive, and Sneed climbs out, runs, and gets chased. One machine cycle
is about 12 to 15 sec. The code is `crates/htl-survival/src/sim/machinery.rs` (tests in
`machinery_tests.rs`); the numbers are `dm_shredder` and `dm_sneed` in `content/deadmines.toml`.

## The nine beats

1. **Pull.** At its post the machine cuts at a log with its back to the Mast Room door: a swing
   (AttackUnarmed, either take) every 3.2 sec, sparks off the blade, the saw's sound. It notices a
   hero as any creature does, turns to face them (0.8 sec), "Clear the floor!", and walks in at its
   Walk clip's own pace (2.5 yd a second) until it's in reach. Its first blow comes only then.
2. **Saw sweep.** The arm swings out (AttackUnarmed take 0, 1 sec), the saw revs with sparks every
   0.35 sec. A guard stops it, or get behind it or out of its 4.7 yd. One vent puff after.
3. **Stamp and rake.** A foot rises and lands (CreatureSpecial, 1.1 sec), dust where it lands, no
   guard stops it. Then a blockable rake (AttackUnarmed take 1, 0.7 sec). Now the vents pour steam
   (a puff every 0.5 sec) with a hiss.
4. **Grind,** at full heat. It plants and stops turning, its legs churning in place (its Run,
   held 1.4 sec, the engine racing and smoking): "That'll cut it!" Then it drives 6 yd straight in
   0.9 sec, its legs running, chips and sparks off the floor every 0.2 sec. Its 3 yd lane checks
   the swept path and hits each hero once; walls stop it. Step aside during the rev.
5. **Stall,** 3.5 sec. Slumped (StandWound, held), smoking and sparking, facing the way it drove.
   "C'mon, you heap of junk!" A posture break may lengthen it, never shorten it.
6. **Break.** "No, no, not now!" The machine collapses (its Death lands at 2.5 sec). Then Sneed is
   thrown out (0.6 sec in the air) past the wreck's edge farthest from his foe and lands in dust.
   Nothing reaches him before he lands. A second later: "That was company property!" His first
   blow comes no sooner than 2 sec after he lands. The wreck smokes while he can still mend it.
7. **Sneed on foot.** He backs off, facing his foe, to 8 yd at 4 yd a second (slower than a hero
   runs). A wrench every 6 sec, then a 4 yd scramble. Caught within 3.4 yd, he flurries three
   blows, then is open 1.4 sec, hunched (StandWound, held).
8. **The repair, once.** No sooner than 6 sec after he lands, at a safe moment: "Just...
   one... bolt..." and he runs to the reachable edge of the wreck farthest from his foe (6 sec to
   get there, else he gives up). 4 sec of hammering (Mining) and sparks. Any hit, or a stagger,
   stops it for good, and the wreck goes quiet: no more smoke, a body gone 6 sec later. Finished,
   the same wreck heaves back up (JumpEnd) in smoke and dust where it lay, facing the way it fell,
   at a third of its health. It says nothing and doesn't walk in; it waits 2 sec before its first
   blow.
9. **Second break.** He bails out the same way, with the health he had. No repair left.

A wipe at any point (Sneed inside, on foot, mid-repair) puts back one whole machine at its post;
a lamp rest does too. Only Sneed's death awards plunder and gear and clears the pack.

## Health and sizes

The machine has 950 base health and 320 posture; Sneed 340 and 160. The machine's legs stand
about 1.5 yd out from its middle at scale 1 (`girth`), 2.2 yd at its drawn size (1.1 times the
bosses' 1.35). A hero's blow reaches it from there, and it stops 4 yd from its hero (reach 4.3),
so the hero stands outside its legs. Between blows it turns where it stands rather than circling.

## What the shared fixes became here

The shared-fixes table belongs to the Chandrian session; these are the local versions for this
fight only.

| Shared fix | Here |
| --- | --- |
| Punish windows look like idling | The stall holds StandWound 3.5 sec; Sneed's open 1.4 sec holds GoblinMale's StandWound (`HoldPose` 8) |
| Debug damage counts as real | `machinery_debug`: debug damage builds no posture on either form and never stops the repair |
| Second forms are instant swaps | Sneed hidden in the wreck until it lands, thrown out, a line, 2 sec before his first blow; the rebuilt machine heaves up and waits 2 sec |
| Deaths lag | Either form sends `HoldPose` 0 as it falls, letting go of a held slump, rev or hunch |
| Yells collide | `machinery_say` keeps the encounter's lines at least 2 sec apart (the pull line included) |
| Bodies overlap | `girth` on the machine: hero reach and the machine's stop are measured from its legs |

The one-tint tell: the machine's wind-ups no longer tint its body. The server marks its
telegraphs (`TG` field 13, "1") and drops the creature from its unblockable warning (`WN`), and the
add-on skips the glow for it. The hero is still warned on their own screen (the flash and ping
before an unblockable blow, the floor mark with markers on).

## Native data used

GoblinShredder (display 1269, `Creature\Goblin\GoblinShredder.m2`) has Stand, Walk (2.5 yd a
second), Run (6.9), AttackUnarmed takes 0 and 1 (impact 1.00 and 0.83 sec), CreatureSpecial 130,
StandWound, Death ($DTH at 2.50 sec), Walkbackwards, Swim and the jumps. No stun or kneel clip, so
the stall is StandWound and the heave is JumpEnd. Sounds: GoblinShredderAttack 1977 (saw),
GoblinShredder 3700 (engine), GoblinShredderWoundCrit 1979 (stall), SteamGeyserVar1 4858 (vent
hiss). Smoke is Vanish's puff (spell 1856), sparks the parry kit (555), dust 5649. Sneed's wrench
is spell 13398 (`Spells\Missile_Wrench.mdx`, speed 20) on GoblinMale's AttackThrown 107.

## Filming

Film with the spectator cameraman ("Filming a boss" in `docs/visual-check.md`): `camera low
Sneed's Shredder` for the machine, `camera duel Sneed` for him on foot. Start from an unbeaten
Sneed pack and a hero out of its sight near the Mast Room door, so the log work, the turn and the
walk in are on camera. `tools/visual-check/shredder-pressure-repair.txt` is an older
dev-command demonstration (forced breaks, repeated heals): it shows the moves, not a fight.

## Tests

`cargo test -p htl-survival machinery` covers each beat on flat ground: the move order and each
move's look, the walk in and where it stops, the lane and the stall, the break and Sneed's
landing, the 2 sec lines, his footwork, the repair (far edge, 4 sec, the heave, health kept, no
second repair), interruptions, wipes and debug damage. `shredder_native` (ignored without
`WOW_DATA`) runs the repair on the real Mast Room floor:
`cargo test --release -p htl-server --test shredder_native -- --ignored --nocapture`.
