# Sneed's Shredder, then Sneed: the machine and the coward

Corey's revised design (Nov 9). The machine's three moves each look and sound like themselves, its
grind is a plant, a rev and a drive, and Sneed climbs out, runs, and gets chased. Since Nov 14 the
broken machine always ends at its log pile, Sneed crawls out of the wreck, and his repair is a
race to a log rigged over it (Corey's notes on the boss review). One machine cycle
is about 12 to 15 sec. The code is `crates/htl-survival/src/sim/machinery.rs` (tests in
`machinery_tests.rs`); the numbers are `dm_shredder` and `dm_sneed` in `content/deadmines.toml`.

## The beats

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
5. **Stall,** 3.5 sec, facing the way it drove: its legs buckle and it slumps (JumpEnd's landing
   crouch, caught 0.25 sec in with the body at half its height and tipped about 30°, and held
   still), smoke pouring (a puff every 0.25 sec) and sparks flying (every 0.4 sec) the whole time,
   the engine sputtering (its own clanks at uneven gaps). 0.45 sec before the end the engine
   catches and it heaves itself up. Hits don't flinch it out of the slump.
   "C'mon, you heap of junk!" A posture break may lengthen it, never shorten it.
6. **Break.** "No, no, not now!" Its legs churn where it stands 0.7 sec, smoke and sparks pouring,
   then it careers back to its post out of control (its Run, smoke and sparks every 0.2 sec) and
   crashes into the log pile it was cutting: a bang, dust, lumber thrown. The run is no attack: it
   hurts and shoves nobody. Its pace is set from the way's length (6 to 18 yd a second), so the
   beat is about as long wherever it broke; at its post already, it just crashes. The wreck
   always lies in the same place, 3.5 yd ahead of the post, and collapses there (its Death lands
   at 2.5 sec).
7. **Sneed crawls out.** Scrap and lumber burst off the wreck and he lies in them on the open
   floor beside it (GoblinMale's Sleep, 1 sec), gets up (SleepUp, 2 sec), and only then: "That
   was company property!" He is in view 3 sec before his line and 4 sec before his first blow.
   Nothing reaches him until he is up. The wreck smokes while he can still mend it.
8. **Sneed on foot.** He backs off, facing his foe, to 8 yd at 4 yd a second (slower than a hero
   runs). A wrench every 6 sec, then a 4 yd scramble. Caught within 3.4 yd, he flurries three
   blows, then is open 1.4 sec, hunched (StandWound, held).
9. **The repair, once: a race.** No sooner than 6 sec after he is up, at a safe moment: "Just...
   one... bolt..." and he runs (5 yd a second) to his one work place beside the wreck, under the
   log. 4.2 sec of hammering (Mining) under a cast bar; the wreck coughs at each quarter, "She's
   turning over!" at half, his sparks quicken (one every 0.8 sec down to 0.25) and the smoke
   thickens. A plain hit hurts him and stops nothing, and he steps back from no blow. Three things
   stop it for good: the log landing on him ("Gah! Who cut that loose?!", stunned and open 4
   sec), a broken stance ("Bah! Forget the heap. I'll do this by hand!"), or his death. Then he
   fights on foot and the wreck goes quiet where it lies: no more smoke or sparks. If he can't get
   to his work in 6 sec he says so ("Bah! Can't get at her. By hand, then!"): he never gives it up
   without a word. Finished, the same wreck heaves back up (JumpEnd) in smoke and dust where it
   lay, at a third of its health. It says nothing and doesn't walk in; it waits 2 sec before its
   first blow. Broken again it runs back and crashes in the same place, no second heap of lumber;
   he crawls out with the health he had and there is no second repair.

A wipe at any point (the machine on its run, Sneed inside, on foot, mid-repair, the log up, cut
or down) puts back one whole machine at its post and hangs the log again over a clean floor; a
lamp rest does too. Only Sneed's death awards plunder and gear and clears the pack; the wreck,
the lumber and the fallen log go with his body.

## The rigged log

From the pull a log hangs 6 yd over where the wreck will lie and Sneed will work (three of the
dig site's barrier poles slung together on their sides, 9.5 yd long), one rope running down to a waist-high stake on the
floor, the tie-off, 4.8 yd from his work place and 3.2 yd outside the drop. The drop is marked on
the floor from the pull: a quiet blue strip, 9.5 yd by 3.2, drawn by the client itself (not the
add-on's optional floor marks).

- **Cutting it.** A melee blow that reaches the stake cuts the rope: any hero's, light or heavy,
  at any time in the fight. One log, one use a fight. A blow meant for a foe doesn't cut it: if
  the blow strikes a foe nearer the hero's own line than the stake is, the rope holds (the first
  filmed look lost the log to a blow at Sneed beside the stake).
- **The fall.** The rope whips away, the log creaks 0.7 sec and comes down in 0.3; the drop's
  mark burns bright from the cut until it lands. A hero in the drop's middle who moves at the
  creak is out either side (1.6 yd).
- **Under it.** A hero is knocked down 2 sec and takes no damage. Sneed is stunned 4 sec, open,
  and a repair is over. The machine is thrown into its own stall (3.5 sec, the Grind's, not
  retuned).
- **After.** The log lies broken in the drop as lumber nobody collides with: it walls nobody in,
  pins nobody and blocks no path. A lost fight or a lamp rest hangs it again.

### The numbers, by measure (`shredder_rig.rs`, on the real Mast Room floor)

| What | Measure | So |
| --- | --- | --- |
| The run back | 1.3 sec broken at the post, 1.9 to 2.7 sec from eight other stands 4 to 16 yd out; 2.7 sec the longest way in the room (49 yd) | One beat from anywhere; it never needs the 6 sec limit |
| To the tie-off | A warrior on Sneed as he says his line is 3.9 yd from the stake: 0.55 sec of run, 0.45 of swing, 0.7 of creak, 0.3 of fall. The log lands 2.0 sec after they go | |
| The repair | His hands are on the wreck 0.6 sec after his line; the machine stands 4.8 sec after it | **4.2 sec**: a hero on him who goes within 2.5 sec of his line wins, one who waits 3 sec loses. A hero waiting 12 yd off wins up to 2 sec late; one 19 yd off must go at once |
| A stance break, the log spent | At his own stance of 160, blows begun as his hands reach the wreck never break it in 4.2 sec (light blows reach 97, heavy 67) | **A stance of 100 while he mends** (`MEND_POSTURE`): light blows without a gap break it 3.35 sec in, heavy ones 3.95 sec in. Possible, and hard |
| A hero under the log | Knocked down 2 sec in reach of a live machine that stands clear of the log, 8 fights on flat ground (`machinery_rig_tests.rs`): 14% of full health lost, 28% at worst. With the machine under it too, nothing: it is stalled | Not a sure death: the cost of a careless cut, or of a friend's |

The repair was 4 sec and any hit stopped it. It was expected to grow to 7 to 9 sec; measured, the
stake is so close to where a hero fights him that a longer repair is won at a stroll, so its
length stayed and what stops it changed.

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
| Punish windows look like idling | The stall's slump: JumpEnd caught with the legs buckled and held still (`freeze_open_pose`, the add-on's `FZ`), smoke, sparks and a sputter the whole 3.5 sec; Sneed's open 1.4 sec holds GoblinMale's StandWound (`hold_open_pose` 8). Neither is flinched out of by the hits that punish it |
| Debug damage counts as real | `machinery_debug`: debug damage builds no posture on either form and never stops the repair |
| Second forms are instant swaps | Sneed hidden in the wreck until it has collapsed, then seen lying in its scrap and getting up 3 sec before his line; the rebuilt machine heaves up and waits 2 sec |
| Deaths lag | Either form sends `HoldPose` 0 as it falls, letting go of a held slump, rev or hunch |
| Yells collide | `machinery_say` keeps the encounter's lines at least 2 sec apart (the pull line included). So a machine broken within 2 sec of another of his lines says "No, no, not now!" as it runs, not as it breaks. One line is not held back: "Gah! Who cut that loose?!" is said as the log lands on him, over whatever was waiting (filmed behind "She's turning over!" it came 2 sec after the log had flattened him) |
| Bodies overlap | `girth` on the machine: hero reach and the machine's stop are measured from its legs |

The one-tint tell: the machine's wind-ups no longer tint its body. The server marks its
telegraphs (`TG` field 13, "1") and drops the creature from its unblockable warning (`WN`), and the
add-on skips the glow for it. The hero is still warned on their own screen (the flash and ping
before an unblockable blow, the floor mark with markers on).

## Native data used

GoblinShredder (display 1269, `Creature\Goblin\GoblinShredder.m2`) has Stand, Walk (2.5 yd a
second), Run (6.9), AttackUnarmed takes 0 and 1 (impact 1.00 and 0.83 sec), CreatureSpecial 130,
StandWound, Death ($DTH at 2.50 sec), Walkbackwards, Swim and the jumps. No stun or kneel clip,
and StandWound keeps the body at full height (measured on its bones: head and body never under
0.95 of standing), so the stall is JumpEnd held at its landing crouch (0.18 to 0.34 sec in, the
body at 0.49 to 0.55 of its height) and the heave is JumpEnd played through. Sounds: GoblinShredderAttack 1977 (saw),
GoblinShredder 3700 (engine), GoblinShredderWoundCrit 1979 (stall), SteamGeyserVar1 4858 (vent
hiss). Smoke is Vanish's puff (spell 1856), sparks the parry kit (555), dust 5649. Sneed's wrench
is spell 13398 (`Spells\Missile_Wrench.mdx`, speed 14) on GoblinMale's AttackThrown 107,
drawn three times its size, tumbling end over end, with an orange streak behind it (its
`[[missile_look]]` in `content/survival.toml`): the bare model was a speck at 16 yd.

The crawl is GoblinMale's Sleep (held 1 sec) and SleepUp (101, 2 sec). The log is the dig site's
barrier pole (ExcavationBarrierPole, display 1508) on its side at a third of its size, three of
them slung together (one alone read as a stick on film); the
tie-off is the dig site's stake (ExcavationStake, 2270) at three times its size, waist high; the
ropes are drawn lines from overhead to the stake and to the log either side of its middle. The
crash's and the fallen log's lumber is DeadMineLumberPileSmall (1108) and loose planks (6481).
Sounds: wood struck 3720, a heavy thud 921, the creak 3360 (the mine's door); the dust is spell
22792's. The drop's mark on the floor is the client's own ground burst (patch 0026), so it shows
with the add-on's floor marks off.

## Filming

Film with the spectator cameraman ("Filming a boss" in `docs/visual-check.md`): `camera low
Sneed's Shredder` for the machine, `camera duel Sneed` for him on foot, `camera wide Sneed's
Shredder|Sneed` for a take that runs through the break. Start from an unbeaten Sneed pack and a
hero out of its sight near the Mast Room door, so the log work, the turn and the walk in are on
camera. The `sneed-shredder-*` scenarios (listed in `docs/visual-check.md`) film the break, the
crawl, the log and the repair; `tools/visual-check/shredder-pressure-repair.txt` is an older
dev-command demonstration (forced breaks, repeated heals): it shows the moves, not a fight.

## Tests

`cargo test -p htl-survival machinery` covers each beat on flat ground: the move order and each
move's look, the walk in and where it stops, the lane and the stall, the run back and the crash,
Sneed's crawl, the 2 sec lines, his footwork, the log (the cut, the creak, who it stuns, the
marks, one use, a hero under it beside a live machine, hung again after a wipe or a lamp rest), the repair (his one work place, 4.2
sec, what stops it and what doesn't, the heave, health kept, no second repair), wipes at every
stage and debug damage. `shredder_rig` (ignored without `WOW_DATA`) runs it on the real Mast Room
floor: where the wreck, his work place and the tie-off stand and that each is reached, the run
back from every yard of the room, the race for the log from four stands, the stance break with
the log spent, and the repair left alone:
`cargo test --release -j1 -p htl-server --test shredder_rig -- --ignored --nocapture --test-threads=1`.
