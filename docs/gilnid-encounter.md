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
  1.3 yd off, on his post's side. The pot's mouth is the furnace that belches and glows.

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
6. **The valve.** He runs to the valve (8 sec at most) and works it 2 sec (the goblin's standing
   use). A stagger stops his hands; he goes back and starts the 2 sec again. Then the furnace
   glows, a whistle (3431), steam vents along the channel for 1.2 sec, and a stripe of fire floor
   advances across the pit from his feet: up to 10 sections 1.6 yd apart, 3 yd wide, one every
   0.25 sec, each burning (30, then 15 once more 1.2 sec later). It runs at his foe where the pit
   floor carries it that way, else straight out from the furnace; it stops at a wall, a step or a
   drop. Both sides are dry. It's drawn as a line on the floor whatever the floor markers. Step
   aside, or stagger him first.
7. **Finish.** "Then we'll melt the lot!" From now his only string is a flurry: three blows, each
   stepping in (lunge), the last a heavy one that over-reaches into the floor. The spanner sticks
   there and he's open 2.5 sec, hunched over it (StandWound held), dust at his feet. It repeats
   until he falls: "Who's... going to pay for this..."

His lines never come closer than 2 sec together: one due sooner waits its turn.

## The shared fixes, done here for Gilnid alone

Corey's table of shared fixes belongs to another session, which builds them engine-wide. These
are Gilnid's versions, for it to fold in:

| Shared fix | Gilnid's version |
| --- | --- |
| Debug damage counts as real | `perch_posture_share`: "debug" damage builds no posture on Gilnid or his crew (hook in `hurt_mob_as`) |
| Effects play at the start of a windup | The blast plays when the stick blows (after its sit); the spill and each stripe section burst when they arrive; the flames come where the fire has reached, not before |
| Small hazards draw nothing with markers off | The stick's blast is a ring on the floor while it sits; the spill and the stripe are lines their true width (`GroundBurst`, drawn whatever the markers); steam and flames are real props |
| Projectiles too fast to see | The stick is in the air at least 0.7 sec, in an arc of two flights |
| Long windups freeze | The 3 sec fuse is Heavy Dynamite's own ready pose with the lit stick; the throw frame plays on release |
| Things outlive their boss | Death, reset or a wipe takes the crew, planks, poles, sticks, flames, steam, the fire still to come and every mark |
| Deaths lag | Death lets go of the stuck pose (`HoldPose` 0) and the valve's cast |
| Punish windows look like idling | The stuck spanner: StandWound held 2.5 sec; a landed engineer sits, then gets up, instead of the stagger's reel |
| Yells collide | `perch_say` keeps his lines 2 sec apart, the pull line included |

## Tests

`perches_tests.rs` (21 sim tests): the planks, poles and heights; the fuse, the arc's two legs,
the stick's sit and blast and the heavy-blow hold; the demonstration (belch, spill to the pole,
fire, line, fall outward, sit, get up, fight); one fall per man from a projectile, a pole blow
and a blow from the ramp; splash, burns and zero damage don't count; a corpse still falls; the
second man's three throws and climb; no demonstration once a hero has dropped a man; the valve's
2 sec, its stagger, the stripe advancing on one line with a dry side; his lines 2 sec apart; the
finish's flurry and stuck pose; debug posture; wipe, death and party handoff cleanup; a ledge
without floor is left out and the fight goes on.

`gilnid_native.rs` (installed data, `--ignored`): his post seen from the way in; both ledges and
their pieces on the real ramp and pit; the first stick and the furnace's lesson, the fall onto
the pit floor; the valve and a 17 yd stripe on his floor.

## Not yet seen in the client

Nothing here has been filmed. `tools/visual-check/gilnid-ledges.txt` and `gilnid-valve.txt` are
the takes to film (dev setup: debug heals, and debug kills for the valve take). To look for: the
plank's hull holding each engineer in the client, the pole's look, the arc's two flights reading
as one throw, the stick and its ring on the floor, the spill and the stripe, SitGround and
SitGroundUp on the goblin, the stuck pose, and whether a hero walking the ramp bumps a plank's
back end.
