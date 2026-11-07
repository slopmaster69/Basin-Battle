# The Deadmines, one encounter at a time

After the Nov 6 play-test (Corey and David, two heroes, about an hour on main at v0.4.2: 24
falls, "almost completely unplayable") the work for the next days is the enemies, not the
classes. Nothing new goes in until the core loop is fun and stable: every fight below is looked
at in order, its creatures' numbers read against the hero's, its tells checked in the real client,
and what's wrong is fixed before the next one. This file is that review, an encounter a section,
with the decisions and what was changed. Numbers in it are the effective ones a solo hero meets
(`content/deadmines.toml` times `combat.foe_health` 1.6 and `combat.foe_damage` 1.35), not the
raw ones in the file.

## What the Nov 6 log says

The host log (`logs/host-2026-11-06-160533.log` and the three before it) names every fall, the
blow that landed it and what the hero was doing. The whole session was the Deadmines, played in
order to Rhahk'Zor (Cookie, Mr. Smite, Gilnid's foundry, Sneed, Rhahk'Zor all fell).

| Felled by | Falls | Caught |
| --- | --- | --- |
| Defias Pirate | 6 | mid-swing 3, mid-cast 1, reeling 1, unguarded 1 |
| Defias Squallshaper | 5 | mid-swing 3, mid-cast 1, reeling 1 |
| Cookie | 5 | unguarded 2, past the guard (unblockable) 2, mid-swing 1 |
| Mr. Smite | 3 | reeling, unguarded, mid-swing |
| Sneed and his Shredder | 3 | reeling 2, mid-cast 1 |
| Goblin Engineer, Defias Miner | 2 | mid-cast, reeling |

Three things stand out.

1. **Sixteen of the 24 falls were to the first two areas' creatures** (the Brotherhood's foot
   soldiers and Cookie). The run got easier as it went, not harder: the deep mines killed less
   than the beach.
2. **The killing blows were small.** The damage of the blow that felled (in the log's brackets):
   1, 6, 7, 8, 8, 12, 13, 13, 16, 16, 17, 18, 19, 20, 21, 21, 26, 26, 28, 33, 35, 42. Median 18
   on a 200-health hero. Nobody was one-shot; everyone was worn down by blows they didn't see
   coming, then finished by whatever landed next.
3. **"Mid-swing" is the commonest way a foot soldier's blow caught a hero** (6 of 11). The hero
   was in a swing of their own when the blow landed: the pirate's wind-up started after the
   hero's swing did, and ended before it. That is a wind-up shorter than the hero's commitment,
   which no floor marker makes readable (see below).

Floor markers were off for this whole session (the lab had turned them off in Corey's saved
settings on Nov 4; Corey kept them off on purpose to see how the game read without them). So
the tally above is the game with its tells as the creatures' bodies alone show them: the test
the enemies have to pass anyway, since a marker is a crutch for a tell, not the tell.

## How a creature's blow is told now

For every blow a creature winds up (`sim/mobs.rs`, `windup`):

- Its body plays the swing's clip over the wind-up (`MobWindup`), the strike landing as the blow
  does; a string's step can name another model's clip (`from`).
- A step with a `kit` flashes it (the glowing cutlass of the pirate's feint, Cookie's smoking
  hand).
- On the ground, a **telegraph** (client patch 0008): a wedge of the blow's arc out to its reach,
  or a ring for a blow all round, turning with the creature, with a fill growing out to the rim
  as the blow lands. Orange for a blow a guard stops (rim gold through the parry window), red for
  one no guard stops. This is what the Floor markers setting turns off.
- A **lobbed throw** (a missile with a splash of 2.5 yards or more) marks where it will land from
  the wind-up's start, through the flight: the spell's own ground look (Heated Ground for Cookie's
  stew) or a ring.
- A blow **no guard stops** sends a `Danger` cue to its target (the add-on's warning).
- A plain **bolt** (a ranged attack with a small splash: the Squallshaper's water bolt, the
  Crossbowman's bolt) gets nothing on the ground, no cue, no marker. Its only tell is the
  caster's cast animation, often 13 to 18 yards away and behind the hero.

What's missing from that list: a sound for the wind-up (a grunt, a weapon-swish, a cast's own
sound), and anything at all for a bolt.

## Encounter 1: the landing, the east beach and Cookie

In by the smugglers' tunnel to the landing lamp, up the tunnel to the beach, Cookie at his stew
pot by the pier. `content/deadmines.toml` lines 94 to 150 and 380 to 625.

### The packs, in the order a hero meets them

| # | Pack | Where | Notes |
| --- | --- | --- | --- |
| 1 | 1 Defias Pirate, **hidden** | the tunnel's mouth | asleep until a hero is 5 yards off: an ambush |
| 2 | 2 Defias Pirates | by the tunnel's strongbox, where it drops away | linked 12 yards |
| 3 | 1 Defias Squallshaper | the slope down to the pier | linked 10 yards, keeps 13 yards off |
| 4 | 2 Defias Pirates, patrolling | the sand | linked 14 yards, walk to the pier and back |
| 5 | **Cookie** (boss) | the stew pot by the pier | 3 phases, 7 strings |

With two heroes below every pack of two or more grows by 35% (rounded: 2 pirates become 3) with
70% more health over the pack, and every blow lands 10% harder (`hero_health`, `hero_foes`,
`combat.hero_damage`).

### Defias Pirate (176 health; 1-handed cutlass and a shield)

| String | Wind-up (s) | Damage | Notes |
| --- | --- | --- | --- |
| usual blow | 0.55 | 18 | reach 2.7, arc 100 |
| two slashes | 0.5 then **0.4** | 16, 16 | recovery 0.2 between |
| lunge (from 3.5 to 8 yards) | 0.6 | 20 | steps 4 yards in |
| feint | 1.05 | 22 | cutlass glows (kit 379): catches a panicked dodge |

Blocks 30% of light blows from the front behind its shield; steps back out of 15% of heavy
wind-ups. Its cooldown between strings is 1.1 s, give or take 25%. Up to two of them press one
hero at once; the third circles at 3.5 yards and works round to the back.

The hero's side: a starting sword lands 14 to 19, an axe 18 to 24; a light swing commits for
0.69 s (sword: 0.26 wind-up, 0.11 active, 0.32 recovery). **A pirate takes 11 sword hits**
(about 15 swings with its blocks), **9 axe hits**. Its slash kills a 200-health hero in 11
blows, which sounds fair until two of them press and a squallshaper bolts from the slope.

### Defias Squallshaper (136 health; a staff, keeps 13 yards off)

| String | Wind-up (s) | Damage | Notes |
| --- | --- | --- | --- |
| water bolt | 1.2 cast | 24 | lands where the hero **stood when the cast ended**; no mark, no cue |
| two bolts | 1.1 then 0.6 | 24, 24 | |
| squall (crowded) | 0.7 | 16, **heavy: knocks down 1.5 s** | ring 4.2 yards, frost flash |

A bolt from 13 yards flies half a second after a 1.2-second cast: there is time to step aside if
you know it's coming, and nothing tells you it is while your back is to the slope and a pirate
is in your face. Three of its five kills were mid-swing.

### Cookie (1040 health, posture 150; "an early lesson in reading a wind-up and a ring")

| String | Wind-up (s) | Damage | Notes |
| --- | --- | --- | --- |
| cleaver | 0.65 then 0.5 | 24, 27 | |
| ladle | 1.1 | 35 heavy | arc 170, ladle glows |
| pot (6 to 22 yards) | 1.2 + flight | 30, **unblockable** | 3-yard ring of Heated Ground where it lands |
| ladle spin (below 75%) | 0.9 | 30 heavy | ring 4.2 all round |
| pots (below 50%) | 1.0, 0.85 | 30, 30 unblockable | two rings |
| three pots (at the 50% turn) | 1.1, 0.85, 0.85 | 30 x3 unblockable | three rings |
| boil over (below 20%) | 1.1 then 0.9 | **39** unblockable ring 5 yards, then a 36 pot | and he shoves everyone 5 yards at the turn |

Three turns (75%, 50%, 20%), each a line said and a pattern at once, his blows 10% then 20%
harder. **1040 health is 63 sword hits or 50 axe hits**; he kills in 6 to 8. The hero's heal is
3 Draughts of 80 health each. Five falls on Nov 6: two guarding against a pot (the recap says
"That attack can't be blocked. Dodge it."), two unguarded, one mid-swing.

### What's wrong here

1. **Wind-ups shorter than the hero's own swing.** The pirate's second slash winds up in 0.4 s,
   the first in 0.5, the lunge in 0.6; a hero's light swing commits them for 0.69 s, and a dodge
   or guard can cut its recovery only from 45% in. A blow whose wind-up starts after your swing
   began and lands before it ends cannot be answered, marker or no marker; it can only be
   avoided by never swinging while a pirate is upright, which is why the log says "mid-swing"
   six times. The souls games keep a foot soldier's first blow at 0.7 to 1.0 s, and only the
   later blows of a string quick (the first blow told you a string was coming).
2. **Bolts have no tell.** The Squallshaper's bolt (and later the Crossbowman's) is the one
   attack with nothing on the ground and no cue, and it comes from behind the fight. Five falls.
3. **Time to kill is too long for the first foot soldier.** 11 to 15 swings for a pirate, with a
   caster chipping 24 a bolt the while. `combat.foe_health` 1.6 was set because "the classes'
   cards hit hard"; the cards Corey tried did not (Fireball, Meteor, Bladestorm: see Mend first),
   so the whole dungeon ran at a health budget the heroes' real damage never met.
4. **The first thing in the dungeon is an ambush.** Pack 1 is a hidden pirate 5 yards off in a
   dark tunnel mouth: the first creature a new hero meets below teaches nothing, it just hits.
   An ambush is a good trick once the pirate's slashes are known.
5. **The first open fight is three at once.** Pack 2 (two pirates) sits by the strongbox at the
   tunnel's drop; pack 3 (the squallshaper) and pack 4 (two patrolling pirates) are linked 10
   and 14 yards and within a few strides of each other on the beach. A solo hero's first fight in
   the open is two pirates pressing with a caster behind them. Nothing before it taught one
   pirate alone.
6. **Cookie is three bosses in one for a lesson.** Seven strings, three turns, two kinds of
   unblockable, a shove, 63 hits of health. The lesson he is meant to teach (one wind-up, one
   ring) is the first 25% of him; the rest is a real boss fight before the hero has fought a
   real boss. The pot's ring (Heated Ground, 3 yards) has to be checked in the client on the
   dark beach: if it doesn't read there, that's two of the five falls.
7. **No sound on a wind-up.** Every tell is visual and in front of the hero's camera. A grunt or
   a swish as a creature winds up, a cast's own sound as a bolt is cast, is what the souls games
   use to make a blow from behind fair.

### Proposed changes (for Corey's call)

Tuning, in `content/deadmines.toml` unless said; each its own small pull request, filmed in the
lab before and after, then played.

- **A floor for wind-ups.** In the Deadmines no string's first blow winds up in under 0.7 s; a
  later blow in a string may be quick, but not under 0.4 s and not for more than 10% of a hero's
  health. Pirate: two slashes 0.75 then 0.45; usual blow 0.7; lunge 0.8. (Cookie's cleaver 0.75
  then 0.5.) Checked by a test over every mob in the file so it can't drift back.
- **A tell for bolts** (`sim/mobs.rs`): a ranged blow with a small splash gets what a lob has: a
  mark on the ground where it will land, for the cast's last 0.6 s and the flight; and its
  target gets the `Danger` cue when the caster is behind them (more than 90 degrees off their
  facing). Alongside: a sound on every wind-up (the creature's weapon swish or the cast's sound),
  which the client plays with the clip.
- **Time to kill.** Pirate health 110 to 75 (120 effective: 7 sword hits, 5 axe hits);
  Squallshaper 85 to 60 (96). The global 1.6 stays until the classes' damage is verified (below);
  if their cards turn out to do what they say, the global comes down instead of the mobs.
- **Order of lessons.** Pack 1 loses `hidden` (a pirate standing in the tunnel's mouth, in the
  light of the landing's lamp: one pirate, alone, the first lesson). The hidden pirate moves to
  the tunnel's drop beside the strongbox (pack 2 becomes one pirate standing and one hidden).
  Pack 3's link to the beach pirates goes (the squallshaper fights alone on its slope unless
  you're in its 24-yard aggro) and pack 4's patrol route is pulled back so the two fights don't
  join.
- **Cookie, the lesson.** Health 650 to 450 (720: 44 sword hits), the 50% turn's three pots
  become two, and the boil-over's pot after the ring is dropped (one unblockable at a time).
  His three turns stay: they are the best-told part of him. Film the pot's ring on the beach
  first; if it doesn't read, the ring gets the drawn red circle under the Heated Ground
  (`mark_as` as well as `mark_ground`).

### Corey's answers (Nov 7)

- **No fall was a one-shot.** "A shitshow of enemies rushing you, both doing the same animation
  in different frames. They rush you as fast as possible and begin swinging while the caster is
  free-casting in the back." The right play is the caster first, but the two pirates swing so
  quick and so together that there's no gap to go for it.
- **Cookie's pot rings could not be seen on the sand.** His animations were the easiest in the
  dungeon to read for a parry or a dodge, and still hard; and **his hits don't line up with his
  swings**, "that's most of them": the blow lands on a different frame than the blade.
- **No wind-up could be heard**, and "almost NONE of the spells you made originally have ANY
  sound at all, makes for a very lame experience."
- **Markers off is the goal**: a game telegraphed well enough that you dodge, block and parry
  without them.

What that adds to the proposals above:

- **The approach reads.** Pirates run at 7.5 (the hero at 7.0) from 14 yards and swing on
  arrival. A foot soldier should come in at a jog, stop at reach for a beat (the souls games'
  "arrive, then wind up"), and only then start its first string; the lunge is the exception, and
  it has the 0.8 s tell.
- **Two never swing on the same beat.** With two pressing one hero (`MAX_ATTACKERS`), the second
  waits until the first's string lands before it starts its own (`sim/mobs.rs`, the turn rule),
  so the hero answers one blow at a time and the "same animation in different frames" goes.
- **The blow lands on the blade's frame.** Measured in the lab: a pirate's slash and Cookie's
  cleaver filmed with `blow`, the frame the weapon connects read against the trace's `Swing`
  time. Any gap over 0.1 s is a client bug (patch 0006's coil-and-snap) or a wrong clip, and is
  fixed before any tuning is judged, since every parry is learned against it.
- **The pot ring is a bug.** Heated Ground does not read on the beach: the drawn red ring under
  every lob (`mark_as` as well as `mark_ground`), always.
- **Sound is its own track.** Every creature wind-up gets a sound (the weapon's swish, a grunt,
  the cast's own), played with the clip, and every ability a cast and an impact sound; there are
  none today (`content/survival.toml` has no ability sound fields at all). Souls telegraphs a blow
  from behind by its sound first.

## Mend first

Broken or unverified things from the Nov 6 session that are fixes, not features, and come before
the next encounter's tuning is judged (a tuning judged against a broken hero is wrong twice):

- **Bladestorm did no damage** (3 casts in the log). Reproduce in the lab against a pack.
- **The combat meter kept resetting and lost damage.** A likely cause is by design: the Fight
  tab shows "the fight you're in or were last in", and a fight ends after a lull of
  `player.combat_secs` (5 s) with no blow dealt or taken (`sim/meter.rs`). Five seconds of
  dodging pots, or walking to the caster, starts a new fight and the tab starts over. Confirm
  with two heroes, then: a fight lasts while any foe is still on you, and the lull is longer.
- **No ability has a sound** (no sound fields exist on `[[ability]]`), and no creature wind-up
  does: see encounter 1's sound track.
- **Fireball and Meteor did almost nothing** (58 and 7 casts): read their damage against a
  176-health pirate in the lab and against Corey's account of it.
- **"Priest just didn't work."** 23 Mind Blasts, 7 Fades, 3 Shields, 2 Penances, 1 Salvation,
  1 Holy Nova in the log and no refusals: find what each did on screen.
- **The lab shares Corey's saved settings** (`benilla-config/saved/ElwynnSurvival.lua`): it set
  `noMarks = 1` on Nov 4 and nothing restored it. The lab gets its own config, or restores the
  file when it finishes.

## Encounters still to review

2. The scaffold round the Juggernaut and Mr. Smite (crossbowman, pirates, squallshaper; Smite's
   three weapons).
3. The Juggernaut's deck (off the way): Captain Greenskin and Edwin VanCleef.
4. The Ironclad Door, the Cannon Gallery and the Foundry: goblin engineers and woodcutters,
   Gilnid.
5. Sneed and his Shredder.
6. The mine galleries and Rhahk'Zor: miners, overseers, dynamiters; the hazards.
