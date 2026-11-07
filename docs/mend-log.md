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

## Nov 7, before dawn

- **The repository's rule changed** (#231, merged on the owner account): a pull request never
  waits for someone to play it; once its checks pass it goes ready with auto-merge, `main` is
  where it's played, and what nobody has seen in the client carries `needs playtest`. Bladestorm
  (#233) merged that way. My gameplay PRs are ready and labelled; `claude/playtest` and the
  playtest folder are only a convenience until they're on `main`.
- **Filmed on the playtest build** (patch 0060 in the client): a pirate's cutlass now travels from
  raised to through the hero over about 100 ms (three frames at 20 a second), where it stood
  still and then was there; Cookie's cleaver the same. After the blow the arm goes straight
  back and the last pose holds through the recovery: no slow second swing. The sparks still come
  about 0.1 s after the health drops. The pirate's shout plays with its wind-up (`Sound 1295`).
  Bladestorm spins on the key alone, 33 strikes on three pirates.
- **Crowded casters' rings flew as bolts** (#245): 25 strings in 9 dungeons were missing
  `ranged = false` and fired an unblockable point-blank missile instead of a ring at the
  caster's feet. A content check now fails on any.
- **Encounter 2** (`claude/scaffold-lessons`): the scaffold Wizard stands with the Squallshaper,
  not beside the first pirates; the Crossbowman notices at 24 yd and its bolt is 30, not 40.
- **Merging**: `gh pr merge` from here is denied by the permission settings, so ready PRs wait for
  the owner account's merge.
- **The bestiary** (#244): every Deadmines creature's every string filmed the same way on the
  playtest build, 95 runs. Every blow up close landed about its wind-up after it began, and every
  creature shouted. One string was wrong: Rhahk'Zor's pound knocked the hero down and its second
  blow knocked them down again where they lay (67 damage with no way to act). Now any blow that
  leaves the hero reeling or down ends the creature's string, not only one that breaks a guard
  (#238). Mr. Smite's flurry and VanCleef's thousand cuts read on film: each blow is its own wide
  swing, about 0.4 s apart, and each finisher winds up longer (0.75 and 0.7 s) with its own ring.
  Held block is the answer to both; a dodge answers the finisher.
- **Who comes, checked again** after the scaffold change: the scaffold's first pull is the two
  pirates alone, met first on the walkway; the Squallshaper brings the Wizard. A hero who walks
  past the pirates to the Wizard still brings all four, so the walkway order matters.
- **CI is green on all eleven open PRs** (#236 to #246). None has auto-merge on: `gh pr merge` is
  denied here, so they wait for the owner account.

## Nov 7, small hours

- **Encounter 3, the Juggernaut's deck** (#247): walking onto it brought two pirates, two Evokers
  and Captain Greenskin at once (plus his two summoned pirates at half). The Evokers stood
  between the two packs and the links passed the fight along. Now the deck hands link only to
  each other, Greenskin has no link and notices at 12 yd, the Evokers stand together on the
  raised fore deck 17 yd from both (walkable, probed), and an Evoker notices at 13 yd. The pull
  report: four fights, each alone. VanCleef's fight is unchanged; his strings read on film.
- **Encounter 4, the Foundry**: nothing changed. One oddity in the Nov 6 log: a ring Engineer
  "fell through the floor" at Gilnid's pit and was put back 200 yd up, then home. A scratch test
  chasing a hero round the pit from both ring Engineers didn't reproduce it (they stay on the
  ring and throw down). Once in all the logs; the game's own recovery handled it.
- **Two patches numbered 0060**: `main` took 0060 for the heal over a friend (#217) while the
  strike patch (#237) was open. The strike patch is now 0061 (a merge of `main` and a rename,
  not a rebase), and the whole series through 0061 is built on the pinned client before it goes
  ready.
- **Every ability, heard and measured** (lab, the client's sound probe on): 194 abilities cast
  against three Deadmines pirates, the 18 charged or timed ones again played properly. All but six
  start at least one sound in the client through their spell's look. The six that started nothing
  (Arcane Bolt, Frozen Orb, Intervene, Jump Pad, Launch Rig, Void Tether) now have a sound of their
  own (#248). So "almost no ability has a sound" on Nov 6 is mostly Corey's volume: Master 10%
  and Sound effects 10% in his saved settings, 1% together. A first read said 29 were silent; it
  had matched sounds to a fixed time after each run began, and runs that set the stage pressed up
  to 5 s later. Matching by the run each sound fell in fixed it.
- **Damage per cast against a pirate (176 health)**, played well, from the same sweep:
  - Mage: Fireball 47 on one (1.2 s charge, 3 s cooldown), Frostbolt 32, Pyroblast 58 for 25
    Valor; Meteor 141 over three (47 each). That Meteor was a short tap; held full it is 70 on
    each, 84 on the flare (a misreading of mine, corrected Nov 7).
  - Shaman: Lava Burst 96 over three (32 each) on a 4 s cooldown; Earthquake, Chain Lightning
    and Thunderstorm all hit all three.
  - Warrior: Shockwave 84 and Cleave 72 over three, every 8 s.
  The Mage's main spells hit one target for about what the Shaman's every-4-seconds spell does to
  each of three. That is Corey's "did almost no damage, single target". Not tuned: Corey put
  class work after the enemies. The numbers are for him. (Channels and damage over time are
  under-counted: a run records about 3 s.)
- **A stale build**: worktrees sharing one cargo target dir reuse each other's builds. The
  playtest server built at 01:00 was the fair-blows branch's rules. Rebuilt in its own target;
  the playtest folder now also has the client with the whole series through 0061.
- **Every Deadmines string, heard** (#249): the bestiary again with the sound probe on. All but
  three strings start a creature sound (a shout, a precast, a missile). The Crossbowman's shot
  and the Evoker's and Noble's shadow pillar (spell 27812, silent) now sound as they wind up.
  The pillar is the pillar's only tell with markers off: seen for its last 0.75 s.
- **Bolt chains elsewhere** (#238): nine unblockable follow-up bolts in Ragefire, the Wailing
  Caverns, Razorfen Kraul and the Graveyard came too soon to dodge after dodging the last. Wind-ups
  raised 0.05 to 0.15 s; a test now holds every string to it. The fair-blows report flags nothing.
- **The beach's rhythm, simulated** (#238): two pirates and a Squallshaper on a passive hero for
  30 s. A pirate could still start its swing during the caster's 1.1 s cast, so blade and bolt
  landed within 0.3 s four times. Blades now also wait for a bolt gathering at, or flying to,
  their hero. That made the pirate already in reach take every turn (10 strings to 0), so the
  turn goes to the one that has waited longest. Now 5 strings each, nothing together, something
  winding up at the hero 59% of the time (68% before), the longest quiet spell 1.5 s.

## Nov 7, morning

- **The Mage, tuned** (Corey: "tune the mage, then we focus on enemies all day"). Fireball now
  explodes on what it hits: its damage and burn on every enemy within 2.5 yd, Lava Burst's burst
  of fire where it struck. Pyroblast stays one target, 48 to 64, the pick for one big foe. Meteor
  45 to 90 (108 on the flare), knocked back 3 yd instead of 6, which threw them out of its own
  burning crater. Two bugs found on the way: a bursting projectile that struck a foe cast its
  spell again at the spot, flying a second missile there from the caster (Gravity Lance and
  Collapse did it too), and it dropped its critical's effects (a perfect Fireball's 8 sec burn).
- **The sparks' 0.1 s**: the server sends a creature's blow and the health drop in one tick, and
  the client spawns the blood on the frame it hears the blow. The likely lag is the effect's own
  emitter, which births its first particle only once its rate has added up to one. Not yet
  changed: a client patch that primes a one-shot effect's emitters.
- **Corey's run, 06:15 to 06:30** (the Playtest folder, a Warrior, filmed by the session
  recorder): three falls, to Mr. Smite (unguarded), Marisa (a heavy blow, mid-swing) and Gilnid
  (a heavy blow through the guard). Read from the film's health bar, he met each boss already
  worn down: 32 of 200 ten seconds before Smite, 27 before Marisa's ring, 6 at Gilnid. No fall was
  one unfair blow; each was the first big blow after a long drain.
- **A careful player, simulated** (#251, `tests_fair_hero.rs`): 0.3 s reaction, no markers;
  parries what it can block, rolls heavy blows and bolts, bashes casts in reach, keeps a roll's
  stamina, waits out a boss's string. Every pack 5 times. No blow anywhere is too quick to
  answer (one Evoker pair's, once in 5), and almost every ordinary pack costs it nothing. Bosses
  cost some: VanCleef 49% of health on average (96% worst), Rhahk'Zor 56%, the Shredder 35%,
  Smite 7%. No boss falls to 90 s of the starting sword.
- **So "too hard" is not unanswerable blows now; it's what a human misses adding up.** Each
  missed parry on a pirate is 12 to 18, a heavy one 30 to 60, and a lamp gives 200 health and 3
  Draughts of 80. The film shows the drain: 200 to 27 over the minute before Marisa. The levers
  are the health between lamps, the heavy blows' cost when misread (a guard against a heavy one
  breaks: two of the three falls), and how long a boss takes to kill.
- **A learning player, simulated** (#251): 0.45 s reaction, a quarter of blows misread, swings
  into strings. On the playtest build, mean of 5 runs, share of a 200-health bar: ordinary packs
  2 to 30% each; Greenskin 36%; the Noble, Cookie, Marisa and Johnson about 150%; Sneed, Gilnid,
  VanCleef 260 to 270%; Rhahk'Zor 313%; Mr. Smite 449%. The careful player takes 0 to 56% from the
  same bosses. So the bosses are fair (nothing too quick) but long and punishing: most outlast
  90 s of the starting sword, and every misread blow is 28 to 66. Corey's run agrees: Smite never
  fell (his last word heard was at a third of his health).
- **The sparks' 0.1 s, measured** (the client's own trace, `WOW_MOVE_TRACE`, tags fct and fx):
  the blood kit spawns 1 to 13 ms after the blow arrives (59 ms the first time, loading it). The
  lag is inside `BloodSpurt.mdl`'s emitter, as the stock client draws it. A client patch could
  prime a one-shot kit's emitters; not done.
- **Fireball and Meteor filmed** (#250, lab on the playtest build): one fireball, one burst of
  fire, 43 over each of three pirates and all three burning; Meteor 108 on each of three.

