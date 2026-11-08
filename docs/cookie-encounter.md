# Cookie: dinner interrupted

Cookie alternates a cleaver pair, a broad ladle, and Serve It Hot. The pour locks its direction
at the first pose and lands 3 yd ahead in one 2 yd patch. It warns for 1.8 sec, throws a native flying mug to the committed floor point, and starts heat
after its short flight (at least 0.15 sec). It deals 20 damage on the first tick and 4 on each of two later ticks, then leaves 1.5 sec of harmless thinning
steam. Five steam anchors reduce to one when the heat ends. No second pour or scoop can begin
until the old patch disappears. Invalid, wet, blocked, or wrong-floor patch points are rejected.

Only a fully executed string counts as an exchange. An interrupted wind-up or the first cut of
the pair does not count. After 3 exchanges and the preceding recovery/cooling, Cookie walks to
his cauldron and scoops for 4 sec. A blocked route gives up after 6 sec without a teleport or heal.

Other crowd control delays the gulp; only a posture break spills it. His posture threshold falls from 150 to 60 while scooping, with ordinary party scaling. Plain
light attacks can break it. A break spills the taste and leaves 4 sec of cough recovery. Otherwise
he regains 4% maximum health and begins a separate 1.6 sec warning for a fixed 5 yd, 65-degree
steam cone. The cone deals 24 damage and leaves the same 4 sec recovery. The pot grants at most
2 heals in a pull, including tastes taken at full health: at most 8% total regeneration. His
second successful taste says the pot is empty; later scoops find scraps and still offer the same
interrupt/recovery choice without adding healing or a new damage stage.

Gorged alternates 2 slaps with a 2.2 sec preparation for a fixed 7 yd, 70-degree belch. The belch
deals 34 damage, then gives 4 sec of cough recovery. There are no health phases, murloc summons,
low-health speed bonuses, or hidden loadout requirements. The secret ingredient and delayed
second-form handoff stay in the existing dungeon system.

Native assets checked against local 1.12.1 data:

- Displays 1305 and 1773 both use Murloc. Their native multipliers are 1.3 and 2.0. Gorged's
  authored scale is now 1.15, so the second form visibly grows instead of matching Cookie's size.
- Murloc has Attack1H 17, Special1H 57, AttackThrown 107, LoadThrown 112, SpellCast 32 and
  CombatWound 9. It does not have clip 54 or dedicated eating, working, coughing or inhale clips.
  The scoop uses LoadThrown and the recovery uses CombatWound as stand-ins. No attached vessel,
  mouth belch animation, or physical belly inflation is claimed.
- Throw Dark Iron Ale 14814 has a native mug missile at 30 yd/sec. `Event::Missile` sends it to
  the fixed spill point; the heat and splash sound wait for its flight. The mug is a released
  projectile, not an attached vessel. Geyser 10987 uses ice hand/chest effects and is not used for
  the pour. Its pre-existing secret-form rise effect is unchanged.
- Cauldron display 216 exists and measures about 1.8 by 2.0 by 1.5 yd. Steam02 display 431 is a
  particle-only emitter. Unlike Deadmines valve display 353, it has no metal pipe mesh to leave
  in the stew. Owned particle cues use the existing immediate-display flag, without changing global
  ground-look thresholds or client patches. The solid cauldron keeps its collision hull. Particle
  radius/opacity still require client review.
- MurlocAlert 1109 precedes Gorged's belch, MurlocWound 418 accompanies recovery, the one-shot
  SteamGeyserVar1 4858 marks a breath release, and medium splash 1096 marks a pour or spill.
  These IDs exist; playback and whether the alert reads as an inhale are not yet verified.
- The real Deadmines map probe finds Cookie's post near (-63, -900) on floor 14.9 and the pot
  offset (-63, -898.2) on floor 14.31, connected by a 3-step path. Three nearby side positions
  are connected by 4-step paths. Runtime floor checks remain authoritative.

The kitchen owns every pot, warning, spill, and breath prop. Death, return, wipe, reset, and the
secret-form handoff remove them; repeated cleanup does not emit duplicate object removals. The
normal AI cannot skip a final recovery, randomly extend a string, or chain another attack over
this encounter's authored beats. Other boss behavior is unchanged. Separate comments beside
Smite's 3 loadouts explain existing tuning in response to the PR #298 review; their numbers do
not change.

Validation is in progress. The focused tests exercise actual AI/exchange counting, interruption
with plain light attacks, success/heal limits, a fixed pour and dry side, solo/duo completion while
waiting through every scoop, 4-hero progression, Gorged, route timeout, and cleanup. The controller
uses inflated hero health and flat terrain; it is evidence of progress, not a normal-health clear.

`tools/visual-check/cookie-kitchen.txt` and `cookie-gorged.txt` are isolated markers-off controlled
scenarios. They clear existing foes and summon fresh actors, so old dungeon completion cannot
hide the encounter. The kitchen scenario lets the first scoop succeed and schedules a DEBUG
posture-breaking damage hit after the second scoop yell. Both scenarios use periodic healing and
a debug finish, and must be labelled as demonstrations. The capture operator must check the
actual poses, sound, steam footprint, thinning, dry-side answer and cleanup before claiming them
as footage proof. No real-client playtest or normal-health solo/duo dungeon clear is claimed yet.
