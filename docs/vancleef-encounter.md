# Edwin VanCleef: the false opening

Corey's revised design (Nov 8, "Deadmines bosses: review and redesign"). The rule a hero learns:
**hunched means hit, blades up means wait.** Every completed combination ends in a hunch
(StandWound held for its whole recovery; a kneel for the last one), and only a hunch is open.
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
   hunch. He looks different: black Blackwing blades (displays 31855 and 31856) and Shadowform's
   dark swirl at his feet (15473). His gap is the halved one from the start.

His lines never come closer than 2 sec together: one due sooner waits its turn. Both forms bypass
random string cuts and extensions, held beats, extra chains, tactical retreats and circling, so
the visible exchange stays honest. The ordinary lunge still closes distance without spending the
next Invitation. Reset, return, wipe, death and the handoff remove his poses and the Blackguards.

## The shared fixes, done here for VanCleef alone

Corey's table of shared fixes belongs to another session, which builds them engine-wide. These
are the VanCleef and Unbowed versions, for it to fold in:

| Shared fix | Here |
| --- | --- |
| Punish windows look like idling | The hunch (`HoldPose` StandWound 8) for every final recovery, and the kneel (EmoteKneel 75) for the last stand. `vancleef_string_landed`, `duel_open` |
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

`tools/visual-check/vancleef-*.txt` film each beat with the spectator cameraman (`camera duel
Edwin VanCleef`), markers off, from the hero's own keys once the video starts. Setup (the deck
cleared, the boss spawned, the hero healed) happens before the video. A clip that needs a health
change after that says "dev setup" in its first line and proves nothing about the fight.

## Not tested in the client

The hunch is the model's own StandWound and the kneel EmoteKneel, held by the add-on's `WP`
line; whether they read at duel distance, and the Blackwing blades and dark swirl, need the
real client. So do the red glow on a creature's weapon, the smoke puff's opacity, the native
deck's light at the duel spot, and the Blackguards' walk out of sight.
