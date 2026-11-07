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
