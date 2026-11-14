# Gilnid: trouble above the furnace

Corey's revised design (Nov 9, "Deadmines bosses: review and redesign"). The lesson: **men on
ledges above the drop can be brought down, and the furnace is Gilnid's last weapon.** The code is
`crates/htl-survival/src/sim/perches.rs`; the numbers are in `content/deadmines.toml`.

The fight was first built by Codex (#295, #299, #302, #304) with low crate-stack perches beside
him. This rebuild replaces those with ledges on the Foundry's own spiral ramp and follows Corey's
seven beats; the finite crew, the one-fall-per-man rule, the cleanup on death or reset and the
fallback when a ledge can't be placed are kept.

## The arena

- **His post** is (-200, -587) on the pit floor (z 21). He stood at (-205, -592), behind the
  cargo, the cannonball stacks and the rock the ramp's east side rests on: from the gap by the door
  a hero couldn't see or lock him. From the gap (-186, -579) and (-188, -580) he's in sight and in
  his 18 yd. The spot the review filmed from, (-183, -585), is under the ramp beside that rock;
  he's still out of sight from there, but nobody has to stand there to come in.
- **The ledges** are on the spiral ramp's south side, where its walkway is 7 to 10 yd over the
  pit. Each is a plank (DarkIronWoodPlanks07, display 6481, scale 1.8) laid with its back end on
  the ramp's rail and its far end out over the drop, and a timber pole with an iron foot
  (ExcavationBarrierPole, 1508) from the pit floor up to the plank's underside. The engineer
  stands 2.4 yd out from the rail. The first stands 8.6 yd over the pit (rail top z 29.2), the
  second 9.7 (z 30.3); their planks splay apart. No crate stacks.
- Nothing new is added to the map: the ramp, rail and pit are the Deadmines' own floors, and the
  plank and pole are game objects with their own hulls (the client stands each engineer on his
  plank). `crates/htl-server/tests/gilnid_native.rs` checks every piece on the installed map:
  the rail under each plank's back end, open air under the man and the pole, the pole and the
  landing on the pit floor and walkable from his post, the plank in sight from his post, the
  walkway behind the rail and a way down the ramp from it.
- **The valve** is the melting pot's own, on its south face at (-208.1, -583.1); he works it from
  1.3 yd off, on his post's side, and nowhere else. The pot's mouth is the furnace that belches
  and glows.
- **The channel** his fire takes is one place in the pit, always the same: from the pot's foot by
  the valve 17.4 yd out across the floor, 3 yd wide, 30 degrees east of straight out (`channel` in
  `content/deadmines.toml`), 10 sections 1.6 yd apart. Each section burns 1.5 yd around its
  middle, and the mark reaches as far as the end sections bite, so no floor that burns lies off
  it. It lies whole on the pit floor, with no pole, no landing and no raised floor over it, and
  dry floor 2.3 yd from its middle on both sides (at its first section, against the pot, on one
  side).

## The seven beats

1. **Pull.** "Back to work! All of you!" The first engineer stands on his plank over the drop;
   the second waits his turn on his.
2. **First throw.** 1.5 sec after the pull the first man lights a stick: Heavy Dynamite's cast
   (4062), a 3 sec fuse with the lit stick in his hand (its precast kit and ReadyThrown pose) and
   the fire-lighting sound. Its landing is chosen as the fuse is lit, at his foe. On release
   (AttackThrown) it flies up to the top of an arc 1.8 yd over the line from his hand, then down
   from there to where it lands (the client flies a missile straight, so the arc is two flights,
   the second from an unseen stand-in a tick later; at least 0.7 sec in the air). It lands and
   sits there a second, a stick on the floor (GunShopDynamite, 423), hissing (1424), its 2.5 yd
   blast drawn as a ring on the floor whatever the floor markers. Then it blows. **While a stick is
   lit, in the air or on the floor, Gilnid holds his heavy blows** (the overhead, the kick).
3. **The demonstration**, after that first stick blows, if no one has been brought down yet. The
   furnace belches (Molten Metal at its mouth) and 0.6 sec later a spill of molten iron runs from
   its foot to the first man's pole, a section every 1.5 yd, every 0.2 sec, each a small splash
   (8) drawn as a line on the floor. Where it reaches, the pole takes light (flames, 397) and
   Gilnid shouts "Mind the equipment!". 1.4 sec later the plank goes: the pole and plank fall as
   lumber and a plank lying on the floor, and he falls outward, away from the ramp, 2.6 yd past
   where he stood, in plain view (0.9 sec, his Fall pose). He lands sitting (SitGround, 1.3 sec),
   gets up (SitGroundUp, 1.2 sec) and fights with his wrench.
4. **The second engineer** throws in turn, a stick each time the last one has blown and 1.5 sec
   passed. He comes down to one direct ranged hit (a projectile's own contact, not its splash, a
   burn or zero damage), a melee blow on his pole from the pit floor (it cracks: the same fall),
   or a blow on him from the ramp behind his plank. If none of those, after his third stick he
   steps back off his plank onto the ramp (1 sec) and walks down it to fight.
5. **Workers on foot** fight with wrenches. When the last is down: "Useless. Every one of you."
6. **The valve: a race.** At "Useless. Every one of you." he starts for the valve, and from that
   moment the channel is marked. The line, the mark and his run start together: if his last line
   is less than 2 sec old he waits for it where he stands, so the mark never shows before the
   words. It has three looks, and a fourth as it goes out:
   - **It will burn if he finishes** (quiet): a pale blue band on the floor its true size, and a
     thin wisp of steam over each section. Nothing hurts.
   - **It burns in 1.2 sec**: the furnace glows, a whistle (3431), the band turns bright and full
     steam stands on every section.
   - **It is burning**: flames, a section every 0.25 sec from the valve's end out, the band amber
     where they burn. Each section bites as it lights (30) and once more 1.2 sec later (15); on
     a lone warrior that's 41 and 20 of his 200. Sections overlap, so a hero between two
     middles, which is most of the channel, is bitten by both.
   - **It's out**: one grey band drawn back from the far end to the valve, the wisps going with it.

   He runs to the valve (0.95 to 1.75 sec from anywhere in his pit) and puts his hands on it for 2 sec
   (the goblin's standing use, with a cast bar). What a hero can do:
   - **Stop it for good**: stagger him while his hands are on the valve. One heavy blow that lands
     on him there does it whatever it deals; so does 120 posture built on him from the moment his
     hands go on (what was built before doesn't count, and a stance broken on the way doesn't
     shield him). No fire. The mark goes out, he shouts "No! You've jammed the valve!" and reels
     4 sec, open. Then "Then I'll break you myself!" and his finish. (As a hit ends Sneed's repair
     for good.) Anything else that takes his hands off it stops it the same way: a stun,
     incapacitate, petrify or panic that takes on him. He is a boss, so only an effect marked
     `vs_bosses` does; the rest he shrugs off and the valve runs its course.
   - **Only delay it**: a stagger while he runs costs him the time he reels, nothing more.
   - **Let it come and step aside**: both sides of the channel are dry, 0.33 sec at a run from
     its middle. After the fire he shouts "Then we'll melt the lot!".

   What can't happen:
   - He never works the valve from anywhere but the valve. Kept from it (3 sec without getting
     nearer; time spent reeling doesn't count) he gives it up: the mark goes out, no fire, "Then
     I'll break you myself!". (The first build let him work it where he stood after 8 sec. No
     reason for that cap was recorded; it's gone.)
   - No worked valve without fire, and no fire off the mark: the fire's sections are the marked
     channel's and nothing else. On a floor with no room for 4 sections, or nowhere to stand at
     the valve, he never starts for it: no mark, no line about melting, straight to his finish.
   - A hero up on a plank or the ramp isn't burned by the fire on the floor below, and the valve
     still runs its course while Gilnid waits for them.
   - His death, a lost fight or a reset takes the mark, the steam and the fire still to come at
     once. A hero who falls while a friend fights on ends nothing.
7. **Finish.** From now his only string is a flurry: three blows, each
   stepping in (lunge), the last a heavy two-handed chop (Attack2H's second take) that over-reaches
   into the floor. The spanner sticks there and he's open 2.5 sec, bent double over it with both
   hands on it (the chop caught 0.2 sec after it lands, his head at half his height and both hands
   by the floor, and held still the whole window), dust at his feet. Hits don't flinch him out of
   it. It repeats until he falls: "Who's... going to pay for this..."

### The race, measured

On the installed map (`crates/htl-server/tests/gilnid_valve.rs`), a warrior alone:

| Measure | Result |
| --- | --- |
| Gilnid to the valve, from 6 places in his pit (6.8 to 11.9 yd) | 0.95 to 1.75 sec |
| From his line to the whistle, from the second landing (8.1 yd) | 3.15 sec (hands on at 1.15) |
| A heavy blow, from its start to landing | 0.50 sec |
| A hero 8.8 yd from the valve who goes at once | stops him 1.70 sec after the line |
| The same hero 0.5, 1.0, 1.5 sec late | stops him at 1.90, 2.35, 2.80 sec |
| The same hero 2.0 sec late or more | too late: the whistle |
| Standing on a section's middle to the end | 61 of 200 health |
| Standing between two sections' middles to the end | 122 of 200 health |
| Stepping out 0.5 sec after the whistle, either side, sections 1, 5 and 9 | 0 |

So the race is won by a hero who answers the line within about 1.5 sec and lost by one who
doesn't; the 2 sec of work stay as they were (no retune). Light blows alone build the 120 in
about 2.7 sec, longer than the work: a lone warrior needs the heavy blow. Other classes and
parties weren't measured (with 2 heroes the 120 becomes 204; the heavy blow still does it at
once).

His lines never come closer than 2 sec together: one due sooner waits its turn.

## The shared fixes, done here for Gilnid alone

Corey's table of shared fixes belongs to another session, which builds them engine-wide. These
are Gilnid's versions, for it to fold in:

| Shared fix | Gilnid's version |
| --- | --- |
| Debug damage counts as real | `perch_posture_share`: "debug" damage builds no posture on Gilnid or his crew (hook in `hurt_mob_as`) |
| Effects play at the start of a windup | The blast plays when the stick blows (after its sit); the spill bursts where it arrives; the channel's mark stands from his line, the steam from the whistle, the flames only where the fire has come |
| Small hazards draw nothing with markers off | The stick's blast is a ring on the floor while it sits; the spill and the channel are bands their true width (`GroundBurst`, drawn whatever the markers); steam and flames are real props |
| Projectiles too fast to see | The stick is in the air at least 0.7 sec, in an arc of two flights |
| Long windups freeze | The 3 sec fuse is Heavy Dynamite's own ready pose with the lit stick; the throw frame plays on release |
| Things outlive their boss | Death, reset or a wipe takes the crew, planks, poles, sticks, flames, steam, the fire still to come and every mark |
| Deaths lag | Death lets go of the stuck pose (`HoldPose` 0) and the valve's cast |
| Punish windows look like idling | The stuck spanner: the chop into the floor caught bent double and held 2.5 sec (`freeze_open_pose`, the add-on's `FZ`), no flinch meanwhile; a landed engineer sits, then gets up, instead of the stagger's reel |
| Yells collide | `perch_say` keeps his lines 2 sec apart, the pull line included |

## Tests

`perches_tests.rs` (35 sim tests): the planks, poles and heights; the fuse, the arc's two legs,
the stick's sit and blast and the heavy-blow hold; the demonstration (belch, spill to the pole,
fire, line, fall outward, sit, get up, fight); one fall per man from a projectile, a pole blow
and a blow from the ramp; splash, burns and zero damage don't count; a corpse still falls; the
second man's three throws and climb; no demonstration once a hero has dropped a man; his lines
2 sec apart; the finish's flurry and stuck pose; debug posture; wipe, death and party handoff
cleanup; a ledge without floor is left out and the fight goes on. The valve: the channel fixed in
the world and marked from his line, never before it; the mark as long as the fire bites;
whistle, steam, then fire only on the mark with dry ground
either side; a hero in it at the whistle reaches dry floor either side; a stagger on the way only
delays; a stagger at the valve stops it for good and leaves him open; one heavy blow does it
whatever it deals, light blows take longer; kept from the valve he gives it up; no channel or no
stand, no valve; no fire from a dead man; a lost fight takes the mark, steam and fire; a hero who
falls while a friend fights on ends nothing; two heroes, one at the valve and one in the channel;
a hero on a plank isn't burned.

`gilnid_valve.rs` (installed data, `--ignored`, 5 tests): the channel whole on the pit floor with
dry floor a step to either side; he reaches the valve from all over his pit, works it only there,
and the fire keeps to the channel; the race won by a hero who goes at once and lost by one who
waits (the table above); a hero in the channel at the whistle walks out dry either side, and one
who stays pays; a hero on a plank or the ramp is out of the fire and the valve runs its course.

`gilnid_native.rs` (installed data, `--ignored`): his post seen from the way in; both ledges and
their pieces on the real ramp and pit; the first stick and the furnace's lesson, the fall onto
the pit floor; the valve and the channel on his floor.

## Seen in the client

The valve was filmed on Nov 13 with the scenarios `gilnid-valve-*` (`docs/visual-check.md`), from
above and on the hero's own screen, muted. What the footage shows:

- **The quiet mark**: from his line on, a pale blue band from the pot's foot, its true size, with
  a small red vent and a wisp on each section. It's there while he runs and works.
- **The whistle**: the band turns bright and the vents stand up as red boxes in steam. It's told
  apart from the quiet mark at a glance.
- **The fire**: flames a section at a time from the valve's end, dry floor on both sides. A hero
  who steps out 0.6 sec after the whistle takes 0. One who stays between 2 sections loses 122 of
  200 and lives.
- **The stop**: the heavy blow lands on him at the valve 2.05 sec after his line from 8 yd (dev
  setup), 2.2 sec from 10 yd (a real pull, no debug help) and 2.9 sec from 15 yd (a real pull,
  healed). Steam bursts and the band is gone within 0.6 sec.
- **A real pull** by the scripted warrior with no help: the crew down in 42 sec, the valve
  stopped, then the hero fell at 13 health to the finish's first blow. Healed throughout: pull
  to kill in 105 sec.

What reads weak, or wasn't seen:

- The mark going out is a fade under the steam. The grey band isn't seen as grey.
- His 4 sec open after the stop look like an ordinary exchange: he stands bent by the pot and
  nothing marks him as open.
- The valve has no part that moves. He stands at the pot's corner with his hands up.
- He stands in the channel's first section while he works, and the fire starts around him. It
  doesn't hurt him.
- On the hero's own screen, fighting him against the pot puts the camera inside both bodies.
- A `GroundBurst` can't be recalled, so the mark's last pulse stays up to 1 sec after a stop,
  his death or a lost fight. A client line that clears bursts would end it at once; none is
  written.
- Not filmed, tests only: the give-up when he's kept from the valve, a stagger on the way, 2
  heroes, his death during the work or the steam, a hero on a plank while the fire runs.
- The whistle wasn't heard: the takes are muted.
