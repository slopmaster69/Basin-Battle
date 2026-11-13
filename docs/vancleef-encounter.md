# Edwin VanCleef: the false opening

Corey's revised design (Nov 8, "Deadmines bosses: review and redesign"). The rule a hero learns:
**hunched means hit, blades up means wait.** Every completed combination ends in a hunch
(StealthStand held for its whole recovery: bent well forward, blades down, breathing; a kneel
for the last one), and only a hunch is open. Every wind-up rises instead, blades up and back over
the head, so the lowest he ever stands is open (see "How it reads" below).
The code is `crates/htl-survival/src/sim/vancleef.rs`; the numbers are in `content/deadmines.toml`.

## The nine beats

1. **Pull.** He waits in his cabin's doorway (home (-77, -820), under the roof). Pulled, he walks
   7 yd north onto the open main deck at (-70, -819.5) before his first blow, under the lantern
   over the door. Two friendly Blackguards watch from the west rail, at (-64, -808) and
   (-68, -807.5). They have no pack, aggro, attack, loot or reward. The Juggernaut's main deck is
   also Captain Greenskin's: the duel assumes he has fallen first, as the way to the cabin passes him.
2. **Honest string.** Two cuts (0.75 and 0.65 sec windups), then a 1.8 sec hunch. Hits in a hunch
   count 2 times more toward his posture (3 times with the recovery bonus every boss has).
3. **The Invitation.** Blades up (the native held guard, both swords out), feet planted, the
   steel's scrape (sound 700), held 2 sec. "Go on, then." on the first, and again only after
   20 sec. Waited out, he drops into a 1.2 sec hunch: punish it.
4. **The riposte.** A direct frontal hit (melee, off hand, or a projectile's direct contact) in the
   Invitation spends it once. He thrusts (HumanMale Pierce, 85, not the cuts' raised arm), his
   weapon glowing red for the 0.9 sec windup, the only blow that glows. His lunge carries him from
   where he stood, along the line fixed at contact, 1.5 yd past where the hero stood. It strikes
   along that whole line; no guard stops it. A hero who sidestepped is behind him for a backstab
   through his 2.4 sec hunch. One who stood still takes it, but it never takes their last point
   of health. Rear hits, splash, periodic damage, beams, pets and zero damage never provoke; a
   second hero's hit can't restart it; control and a posture break come first.
5. **The posture break, earned.** Posture 300, draining over 7 sec. A lone sword hero who parries
   his cuts (a parry fills a quarter) and punishes his hunches breaks it 2 to 6 times a fight. A
   debug health change never counts toward it. "You've learned something." After the whole 2.2
   sec reel: a smoke puff at his feet (Vanish's, 1856), and he walks 4 yd backward out of it,
   facing the hero, at 2.2 yd/sec, never under a roof (so never back into his cabin). A scuff on
   the boards (649) and a glint (kit 379), 0.9 sec, then the advance: 3 blows (block or
   sidestep), then a 2.5 sec hunch. Blocked or roofed routes leave him in place with the same cue.
6. **About 30%.** The Blackguards wait for a quiet moment (no blow in his hands), turn toward the
   bow, one shakes his head (emote 274), and after 1.5 sec both walk off the ship along the deck
   and down onto the scaffold at (-54, -793), 33 to 37 yd, and are gone when they get there (about
   12 sec). Then, alone: "Cowards! All of you!"
7. **Alone**, his gap between combinations (planted, squared up, after each hunch) halves from 1
   to 0.5 sec. His windups don't change.
8. **12%.** At the end of whatever string or recovery he's in: "This ends here." Four blows (0.65
   sec windups at least), then he kneels 3 sec: the last opening. It happens once, and never
   holds his health above zero. When he dies, any held pose stops at once, so death plays.
9. **Unbowed.** Through the quarters secret only. The body lies 4 sec, then he rises where it
   lay: kneeling 1 sec, then a roar as he stands, then 2.5 sec before his first attack. He's
   dropped the Invitation and the riposte. His strings are 4 blows (the last stand 5), and only
   the final hunch is open (2.8, 3 and 3.4 sec); the blows between have 0.2 to 0.3 sec and no
   hunch. His one single blow is a lunge (1.1 sec windup), open 2.2 sec after. He looks different: black Blackwing blades (displays 31855 and 31856) and Shadowform's
   dark swirl at his feet (15473). His gap is the halved one from the start.

His lines never come closer than 2 sec together: one due sooner waits its turn. Both forms bypass
random string cuts and extensions, held beats, extra chains, tactical retreats and circling, so
the visible exchange stays honest. The ordinary lunge still closes distance without spending the
next Invitation. Reset, return, wipe, death and the handoff remove his poses and the Blackguards.

## How it reads

The Nov 9 filming (sword hero) found "hunched means hit" back to front on screen: the hunch was
StandWound, nearly upright, while his cut wind-ups (the human man's Attack1H and spinning
Special1H, held at their coil) were a deep crouch. Measured on the models' bones at the pose a
wind-up holds (its coil, about half way to the blow) and over the hunch's loop:

| Pose | Head (share of standing) | Back forward | Weapon hand over the head |
| --- | --- | --- | --- |
| The hunch now, StealthStand 120 | 0.77 | 35° | -0.33 (down) |
| The hunch before, StandWound 8 | 0.96 | 11° | -0.43 |
| Old cut wind-ups, HumanMale 17 takes 0 / 1, 57 | 0.90 / 0.84 / 0.78 | 8° / 31° / 31° | about level |
| New cut wind-ups, ScourgeMale 17 takes 2 / 0, 18 | 0.89 / 0.97 / 0.91 * | leaning back * | +0.27 / +0.21 / +0.26 (up) |
| The riposte's thrust, HumanMale 85 (kept; its red glow is its tell) | 0.92 | 16° | -0.35 |

\* On the Undead man's own bones, which stand at 0.85 and 31° forward: each new wind-up rises
and leans back from his stance. The 12% kneel turns him to face the hero he was fighting.

The hunch also used to last only until the first blow into it: on the clients a hit's flinch
replaces any held pose, so punishing him stood him back up. The hunch and the kneel are now held
with `hold_open_pose`, which leaves out the flinch until the pose ends.

## The shared fixes, done here for VanCleef alone

Corey's table of shared fixes belongs to another session, which builds them engine-wide. These
are the VanCleef and Unbowed versions, for it to fold in:

| Shared fix | Here |
| --- | --- |
| Punish windows look like idling | The hunch (`HoldPose` StealthStand 120) for every final recovery, and the kneel (EmoteKneel 75) for the last stand. `vancleef_string_landed`, `duel_open` |
| Debug damage counts as real | `vancleef_posture_share` gives "debug" damage no posture (`hurt_mob_as`) |
| Second forms are instant swaps | Unbowed kneels, roars, then waits 2.5 sec (`vancleef_risen`, called from `rise`; the generic roar there skips him) |
| Things outlive their boss | The Blackguards leave on their own at 30% and are removed with the duel on death or reset |
| Deaths lag | `remove_duel` stops any held pose (`HoldPose` 0) as he falls |
| Yells collide | `duel_say` keeps his lines 2 sec apart |

The riposte's lunge and its line of contact are VanCleef's own hooks in `mobs.rs`
(`vancleef_lunge`, `vancleef_release`).

## Verification

`crates/htl-survival/src/sim/vancleef_tests.rs` covers each beat: the pull's walk-out, two cuts
and their hunch, hunch hits and debug damage on posture, waiting out the Invitation, its scrape
and line, the provoked riposte (once, glowing, a thrust, past the spot, sidestepped, never
lethal), what never provokes, control and break precedence, the backward retreat out of smoke
into the scuff and three blows, the Blackguards' quiet-moment departure and "Cowards!", line
spacing, the last stand's kneel and death stopping it, cleanup, the secret handoff with
Unbowed's kneel, roar and wait, and Unbowed's strings.

`a_lone_sword_hero_playing_by_the_tells_clears_both_forms` plays both forms with a warrior on the
starting sword and their own 200 health, reading only what the client shows (the hunch and kneel,
the raised blades, each windup, the red glow) with a 0.3 sec reaction: parry the cuts, sidestep
the glow, hit only hunches. Patient, it clears VanCleef in about 2.5 min with 2 breaks; hitting
into Invitations to draw ripostes, in about 2.2 min with 6. Unbowed takes about 1.8 min. It never
uses a debug command. That's the simulation, not the client.

`crates/htl-server/tests/vancleef_native.rs` (installed data, `--ignored`) walks the real deck:
the walk-out from under the cabin roof to the open deck, the Blackguards' posts in the open,
their walk off the ship (every stride a walker's, no falls), and the posture retreat staying on
the open deck, from the first exchange and after 15 sec of fighting.

## Filming

`tools/visual-check/vancleef-clear.txt` is an unassisted solo clear, filmed by the spectator
cameraman (`camera duel Edwin VanCleef`) with markers off. Setup before the video: the deck
cleared, the quarters secret found, the warrior healed 14 yd from the cabin door. From the
video on, only the warrior's own keys play (`.es in`), pressed by a small player in the
interface that reads only what the client shows: the hunch or kneel held (`WP` clip 120 or 75: hit,
0.3 sec after it shows), a windup (`AN`: guard the first blow as it lands, roll from a string's
follow-up), the red glow (`WG`: roll aside). It hits into every other Invitation to draw the
riposte and waits out the rest. `vancleef-late.txt` is the same from 33% (dev setup: a `.es hurt`
before the video), for the late beats alone.

On Nov 9 the clear succeeded on the third version of that player, with the Footman's Axe: VanCleef
in about 2 min (2 posture breaks, 4 ripostes rolled aside, the Blackguards gone, the 12% kneel),
Unbowed rising 4 sec later and falling about 1.8 min after, the warrior ending on 50 of 200
health. Earlier takes found what the player had to learn: hold the camera at no fixed yaw so the
lock-on turns the hero to him, guard only a string's first blow and roll the follow-up, and wait
a human's reaction before hitting a hunch (the blow that ends in it settles a round trip later).

## Not tested in the client

Seen in the Nov 9 footage, not yet judged by a person: the walk out of the doorway, the smoke puff
(a clear white ring, seen through), Unbowed's dark swirl and black blades, the body lying 4 sec and
the kneel before he rises. Hard to see: the hunch itself, because the Footman's Axe's bleed sets
VanCleef alight for most of the fight (a sword hero should be filmed), and the Blackguards'
walk off the ship, small at the back of the duel shot. The red riposte glow and the scrape and
scuff sounds need someone to watch and listen. The duel camera sometimes pulls in close where the
deck's masts, cabin and rails crowd it.

Not yet filmed (Nov 10): the StealthStand hunch, the Undead man's rising cuts and overhead on
VanCleef's body, and the 12% kneel turned to the hero. The pose numbers above are from the
models' bones, not the screen.
