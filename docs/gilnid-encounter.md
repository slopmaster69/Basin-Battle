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
disk and expires at impact. The encounter's props reuse client patch 0017's immediate,
noncolliding-object flag: ordinary 2 sec fades had hidden most of the 1.4 sec warning. Damage,
the three 2.2 yd discharge radii, and the global minimum ground-spell radius are unchanged.

Perched engineers hold native red dynamite (item display 18575), with Heavy Dynamite's ready
pose, fuse effect and arc (spell 4062), plus the native fire-lighting sound 6155. On landing,
`MobGear` from PR #298 switches the same body to wrench 1610. Takeoff dust is removed; landing
dust remains. The support disappears immediately into low native Deadmines lumber (1108),
with a wooden impact sound. This is a broken-state swap, not a simulated tipping support.

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

PR #299 adds checks for bounded steam placement and the safe dry side, low debris and dust only
at landing, the gear handoff, and valve facing/gesture cancellation. `gilnid-tells-fuse` and
`gilnid-tells-valve` are short controlled presentation scenarios; ambient foes are cleared and
worker kills are scripted for the valve check. Both explicitly hide optional floor markers.

On the final combined source, all 14 focused Gilnid tests passed. The workspace release suite
passed 1,084 tests with 19 ignored (915 survival tests passed). One initial full run missed the
timing-sensitive client parry check; its isolated rerun passed, and the subsequent complete
workspace rerun passed without failures. Merging main `b32e942` changed only Smite's documentation;
the tested encounter source was unchanged. Native floor queries also confirmed the dry exit
from (-205, -595) to (-205, -591.4) at floor 21; unsafe outer steam anchors are omitted.

Still unverified: the revised tells in the client, an unassisted clear, and solo-versus-duo
balance. Supports do not rock or tip, and valve hand contact is not established. Keep
`needs playtest` until the remaining acceptance is recorded.

## Presentation follow-up claim

Codex is following up the recorded PR #295 encounter in `codex/gilnid-tells`. The direct-hit
fall and finite-workforce mechanics are already verified. This branch improves the bounded
native discharge warning with a visible dry route, dynamite preparation, unobscured descent,
and valve-facing/action feedback. It preserves damage, projectile scheduling, safe-floor
projection and all other encounters. Native support sway and precise hand-to-valve contact may
remain limited by the existing client; any remaining limitations will be stated explicitly.

The current controlled footage confirms that the small steam outlets do not explain the
hazard footprint, the fuse is weak, and takeoff dust masks the body. The claim was published as
draft PR #299 before source changes began. Targeted real-client checks will name the exact
commit and retain the older evidence; silent recordings will not be presented as audio proof.
