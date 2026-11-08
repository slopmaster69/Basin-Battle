# Gilnid: trouble above the furnace

Status: implemented in PR #295; automated verification passed on November 8, 2026. Real-client
capture and unassisted fight validation are still required.

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

Small native valve-steam props warn overflow points even with optional floor circles disabled.
These use `DeadMineValveSteam02` (display 353) at scale 0.8 and expire at impact. The exact steam
footprint and readability still require client footage; the native Molten Metal impact, support
silhouette, dynamite arc/fuse and continuous falling motion have the same visual acceptance.

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

Not tested yet: the authored Foundry geometry and visuals in the real client, a full unassisted
fight with markers disabled, or solo-versus-duo balance. Cargo supports are static rather than
animated to sway. Keep `needs playtest` until these acceptance checks are recorded.
