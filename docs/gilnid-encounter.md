# Gilnid: trouble above the furnace

Status: mechanics merged in PR #295 and checked in controlled real-client footage. PR #299
follows up the visible tells; an unassisted fight and solo-versus-duo validation remain open.

Codex owns this encounter, following the independent Codex and Claude design comparison.
Claude's monster yells and Rhahk'Zor engine are already on main.

The fight uses low, visible perches over the Foundry pit. Engineers throw dynamite with clear
fuses and arcs. A ranged hit brings an engineer down through a continuous fall, followed by
a stunned landing and melee combat. A reachable support offers a melee alternative, and a
bounded automatic descent prevents loadout restrictions. The first fall is demonstrated by
the Foundry's overflow. Workers are finite; they are not a health-threshold summon wave.

Once the workers are dealt with, Gilnid operates the furnace and commits to a visible molten
discharge with a dry escape route. He then finishes the fight directly, without a generic
low-health damage multiplier.

Implementation scope: new encounter code with minimal hooks into existing simulation,
content definitions, focused lifecycle and combat tests, and a real-client recording scenario.
Preserve ordinary ledge safety, the existing ogre encounter, enemy variety, and the HumanMale
VanCleef animation correction. No client animation patches are claimed here.

Acceptance includes solo and party targeting, one hit triggering one fall, melee support and
automatic fallbacks, safe landing, no duplicate workers on reset, and a full fight with optional
floor markers disabled. The showcase must identify any dev-controlled demonstration footage.

## Implemented behavior

`sim/perches.rs` owns two engineers, their cargo supports and platforms, the valve, and temporary
steam warnings. Platforms sit 2.85 yd above checked floor points. The landing is 1.8 yd towards
the room; a different floor, wall, water, overhead slab or unreachable point omits that platform
instead of stranding a worker. The remaining encounter can still complete.

Only one engineer commits dynamite at a time. The 1.8 sec fuse chooses its landing before the
throw. A damaging direct projectile contact drops that engineer once; nearby splash and damage
over time do not drop living engineers. A lethal hit still brings the body down. An ordinary
melee swing can break a support from the same floor, within reach and facing it. Falls take
0.85 sec and leave a living engineer stunned for 2.4 sec, then fighting with a wrench.

The first automatic overflow is eligible after 8 sec, between committed throws. After 26 sec,
remaining perched engineers descend in sequence when no throw or fall is in progress. This is a
fallback, not a requirement to bring a ranged ability. Killing all workers sends Gilnid to the
valve regardless of his health. His 2.2 sec opening can be delayed by a stagger; the discharge
then travels through three separated floor points, leaving dry space to either side. The valve
sequence runs once. Boss death or reset removes workers, props, warning steam and pending throws.

Five native valve-steam props now cover each overflow's width, even with optional floor circles
disabled. `DeadMineValveSteam02` (display 353) at scale 1.4 uses anchors inside the existing damage
disk and expires at impact. Only encounter-owned steam (353), low debris (1108), and the valve
wheel (755) reuse client patch 0017's immediate, noncolliding-object flag. Cargo (36) and planks
(6481) keep their collision hulls and ordinary fade: workers rely on those surfaces for client
floor projection. Ordinary 2 sec fades had hidden most of the 1.4 sec warning. Damage,
the three 2.2 yd discharge radii, and the global minimum ground-spell radius are unchanged.

Perched engineers hold native red dynamite (item display 18575), with Heavy Dynamite's ready
pose, fuse effect and arc (spell 4062), plus the native fire-lighting sound 6155. On landing,
`MobGear` from PR #298 switches the same body to wrench 1610. Takeoff dust is removed; landing
dust remains. Low native Deadmines lumber (1108) appears immediately when the support is
removed, with a wooden impact sound; the structural support keeps its ordinary client fade.
This is a broken-state swap, not a simulated tipping support.

Gilnid faces the valve before using the goblin's native standing interaction (Opening 21651,
animation 123), with the Deadmines steam whistle (3431). The opening remains 2.2 sec and can be
staggered; it grants no invulnerability. There is no wheel animation or guaranteed hand contact.

The Windows GNU timezone compatibility shim now lives once in the server library so examples,
the executable and integration tests link it consistently. The standalone protocol bot, which
does not link the server library, includes its own copy. This fixes the pre-existing example
link failures exposed by the required full test sweep.

## Verification and remaining acceptance

After integrating the gauntlet from main `56c4b6a`, `cargo test --release --offline -j2` passed:
1,060 workspace tests passed, 19 ignored, no failures. The survival crate passed 893 tests with
4 ignored, including all 11 focused Gilnid tests. `cargo build --release -p htl-server --offline
-j2` also passed. Both commands used `scripts/windows-rust.ps1` on Windows GNU. Repository rules
(`scripts/check-pr.sh`) passed across all 17 changed files, with no failures or warnings.

Focused simulation tests cover direct contact versus splash/damage over time, repeated and lethal
hits, support reach/facing/floor, warned throws and automatic descents, wipe/repull, the valve's
health-independent trigger and stagger delay, unreachable platforms, native-warning lifetime and
owner-death cleanup, and a surviving party member taking over the fight.

The `gilnid-perches` and `gilnid-perch-shot` recording scenarios parse successfully. Both disable
optional floor markers and explicitly describe controlled demonstration footage. They use debug
healing and selected debug kills; neither is evidence of an unassisted full-fight clear.

Controlled recordings on `8dc26de` with client `skyB` showed real projectile contact dropping a
worker while Gilnid stayed at full health, both automatic descents, grounded melee, the valve
trigger and cleanup. They also exposed tiny warning steam, weak dynamite preparation and dust
obscuring the fall. These recordings are video-only and cannot prove sound audibility.

PR #299 passed 1,084 release tests but failed in the real client: its broad immediate-prop
predicate removed the cargo and plank hulls, hiding both perched bodies. PR #302 reverted that
merge first (main `22d65ad`). Exact revert `b97f8d9` restored both visible bodies in a same-camera comparison on
client `skyB` (watcher request `gilnid-revert-b97f8d9-platforms`).

PR #304 restores the cues with a visual-only predicate. The simulation regression checks owned
visuals, structural props, unrelated objects, and cleanup. The server regression runs the real
crew through warning and drop, asserting that cargo/plank renderer flags never remove hulls
while steam, valve, and debris render immediately. Existing tests retain direct-hit versus
splash, finite crew, landing gear, warning bounds, safe dry side, and valve interruption checks.

`gilnid-tells-valve` keeps the canonical camera yaw 0 before spawning; camera yaw also affects
the hero's facing and therefore the boss's home. It scripts crew deaths, then moves along the
dry side during the actual discharge. `gilnid-tells-direct-hit` isolates one real Fireball on a
stationary debug boss's worker, with normal crew/controller behavior. These scenarios hide
floor markers and remain controlled QA, not unassisted fight or balance acceptance.

The restored production source was filmed at `299ac99` on `skyB` in watcher request
`gilnid-safe-299ac99-comparison`. Both perched bodies remain visible. The isolated Fireball
reduces one engineer from 80 to 59 health while Gilnid stays at 1,300; the worker visibly
descends and remains on the floor. The valve scene shows the correct-home boss working the
valve, then a dry-side traversal out and back during warning/discharge with health held at
200/200. The last debug heal is at scenario t=30; movement runs t=33-36.8 with no further heal.
The MP4s are 25.5 and 11 sec, 1,942 by 1,136, video only. Local evidence is retained under
`outputs/gilnid/safe-299ac99-*`, with detailed limits in `recovery-review.json`.

Still unverified: an unassisted clear, solo-versus-duo balance, audio audibility, normal-player
fuse readability, support rocking or tipping, and valve hand contact. Native impact effects
still obscure parts of the brief fall. Keep `needs playtest` for those remaining acceptance gaps.
