# Mend log

What Claude measured, changed and still sees wrong, as it happens (newest last). The plan is
`docs/mend-plan.md`; the encounter review is `docs/deadmines-review.md`.

## Nov 7, night

- **Floor markers had been off** all of Nov 6 for Corey (the lab wrote `noMarks = 1` into his saved
  settings on Nov 4). The lab now runs its own game folder on E: (`E:\Basin-Battle-Lab\game`) and
  never touches his.
- **Bladestorm "did no damage"**: the rules land it (5 to 16 a strike, 126 over 6 s); its key only
  raised the reticle and a left click was needed to spin. Fixed: it spins on the key (#233).
- **Fireball, Meteor, the Priest's spells** all land what their cards say in the lab. Their
  problem is size against foes at 1.6 times health, not bugs (Fireball: a tap 17, a full charge
  36, 43 on the flare; Meteor 70, 84 on the flare).
- **A creature's strike**: filmed at 20 frames a second against the trace. The telegraph is exact;
  the strike was a 42 to 80 ms snap after a 0.45 s frozen pose, and the return crawled at half
  speed through the recovery (a second swing with no blow). Fixed in the client (patch 0060).
- **The sparks lag the hit** by 2 to 3 frames (about 0.1 s). Not yet looked into.
- **The meter** starts its Fight tab over after 5 s with no blow dealt or taken. Not yet changed.
- **Session recorder** installed on Corey's PC: every game session filmed on the GPU, with every
  fall marked by its time in the video.

## Nov 7, after midnight

- **Corey's sound is nearly off.** His game's `benilla-config\config.toml` has `MasterVolume =
  "0.1"` and `SoundVolume = "0.1"`: sound effects at a tenth of a tenth, while music is 0.15 and
  ambience 0.25. The shipped config sets no volume at all (full). That alone could be most of "no
  sound": the tells' woosh before a blow lands, the ping for a blow no guard stops, every hit.
  Not changed (his settings are his): to tell him.
- **The tells already exist**, with markers on or off: the striker's weapon glows amber or red
  for its wind-up, flashes bright a reaction before the window to answer, an unblockable blow
  pings, and the stroke wooshes 0.22 s before it lands (`ES.Tell`). What's missing is a sound
  *as the wind-up starts*, for a blow from behind.
- **A bolt has a ring**: from the moment it flies, where it will land (`release`, `mark_ground`).
  None during the cast, and nothing for a hero with markers off. The review's "no mark" was wrong
  on that point.
- **The reaction budget** (`tests_fair.rs`): every blow of every creature in the game against a
  0.3 s reaction, the sword's light swing (it commits 0.50 s before a dodge can cut it) and a
  dodge's first 0.04 s. Half of all first blows (170 of 340) came quicker than a hero mid-swing
  could answer; 137 melee blows were quicker than anyone could answer from standing still (most
  of them a string's later blows, which the held guard answers). Fixed by a rule, not 170 edits:
  `[ai] first_blow = 0.6`, the least a string's first blow (or a lone blow) up close winds up, at
  any phase's pace. Whitemane's unblockable second staff swing now comes 0.3 s after the first,
  so it can be dodged out of the first's dodge. Nine ranged multi-casts still come too close
  together; they go with the bolt work.
- **One blow at a time** (branch `claude/fair-blows`): with two on a hero, the second never winds
  up while the first does, nor for 0.35 s after its blow lands (`[ai] turn_beat`); it holds where
  it is, facing the hero. A creature that has just run up stands 0.35 s before its first blow up
  close (`[ai] arrive_beat`); a lunge or a charge from afar doesn't wait to settle but still waits
  its turn. Tested: two pirates on one hero for 8 s never wind up together.
- **The meter** (#236): a fight now lasts while any creature is still on the hero; a lull alone
  no longer starts the Fight tab over.
- **Foe health** (branch `claude/fight-length`): `foe_health` 1.6 to 1.0. A pirate in 7 starting
  sword hits, not 11 (15 swings with its shield). Cookie 650, Smite 1100: long, but bosses.
- **The bestiary lab** (branch `claude/bestiary-lab`): `blow:<string>` plays a creature's whole
  named string; a queue line `bestiary dm_*` films every string of every Deadmines creature.
- **Client patch 0060** (#237): the strike pacing, the whole series applied on the pinned client.
  My first test build was on an older base (it lacked 17 lines of the Mage's held-pose fix in the
  same file): rebuilt from main's exact series plus 0060 (`main-0060` in the client clone).
- **Cookie's pot rings were markers.** A thrown splash under 6 yd (`GROUND_LOOK_MIN`) is warned by
  the drawn ring (the floor markers' `TA` line), not by Heated Ground: Cookie's 3-yard pots were
  invisible on Nov 6 because the markers were off, not because the sand hid them. With markers off,
  a pot's only tell is Cookie's smoking hand and the pot's arc. A splash of 6 yd or more now gets
  the drawn ring as well as its spell's ground look (`claude/rings-read`), since a look's size
  isn't its blast's.
- **Who comes** (`claude/pull-report`, results in the review): the beach caster and patrol always
  fight together; the patrol turned 12 yd from Cookie's pot; the Juggernaut's evokers bring
  Captain Greenskin.
- **Encounter 1** (`claude/beach-lessons`): the first pirate stands in plain sight (no ambush), the
  beach Squallshaper isn't linked to the pirates and notices at 16 yd (24 took in the whole beach),
  the beach pirates stand instead of walking to Cookie's pot, and Cookie throws two pots at his
  half-way turn (three) and his boil-over has no pot after the ring.
- **One string at a time, with a breath**: turn-taking blow by blow still left one pirate always
  winding up (two alternating strings cover the clock). Now the next waits for the other's whole
  string, recovery included, and 0.8 s after (`turn_beat`); casters cast only while no blow up
  close winds up at their hero.
- **No free hits on a broken guard** (#238): Mr. Smite's four-blow flurry broke a buckler on its
  third blow (a block costs 6 stamina and 90% of the blow) and the fourth landed 0.07 s after the
  hero could move; a creature also took a reeling hero as an opening and began a new string.
  Now the breaker's string ends, and no creature begins a blow up close at a reeling or
  knocked-down hero. "While reeling" felled heroes seven times on Nov 6.
- **Wind-up voices** (#242): each Deadmines creature shouts as a string up close begins, from
  where it stands (the pirates' own attack shouts, goblins', a murloc's for Cookie, a tauren's
  for Smite, an ogre's for Rhahk'Zor). Found with the htl-map `soundfind` example.

### The pull requests so far

| PR | What | Lands on |
| --- | --- | --- |
| #232 | This review, the plan and this log | main, when Corey has read it |
| #233 | Bladestorm spins on its key | playtest |
| #236 | The meter's fight lasts while a creature is on you | playtest |
| #237 | Client patch 0060: the strike is seen to swing | playtest (film to attach) |
| #238 | Enemies you can answer (first-blow floor, string turns, casters, guard breaks) | playtest |
| #239 | Foe health 1.6 to 1.0 | playtest |
| #240 | The Deadmines' first encounter | playtest |
| #241 | The pull report | main (tooling) |
| #242 | Wind-up voices | playtest |
| #243 | Rings always drawn under ground looks | playtest |
| bestiary | `blow:<string>` and `bestiary dm_*` in the lab | main (tooling) |
