# Mr. Smite: the arsenal

Status: implementation complete; automated verification in progress. Real-client acceptance pending.

Codex owns a separate arsenal encounter module, minimal simulation and equipment-update hooks,
the exact Mr. Smite content block, focused tests and a labeled visual-check scenario. This follows
the agreed Codex/Claude design. Preserve Gilnid, RhahkZor, the concurrent Shredder work and all
client animation claims.

Smite starts with a sword, runs to a visible reachable rack at 66% for two axes, and returns at
33% for a great hammer. The held equipment changes with the available moves and their rhythm.
Ordinary attacks and posture can delay a change, within a bounded transition; no arena stun,
loadout requirement, generic health-based speed or damage multiplier, or teleport through walls.
Safe attack boundaries and ordered changes preserve the axe stage if damage crosses both lines.

Acceptance: real AI progression, meaningful distinct movesets and equipment, chase interruption,
bounded path failure, solo/duo behavior, death/wipe/reset cleanup and a labeled controlled scenario.
Rack floor/path and native assets must be verified or explicitly called unverified. Real-client
full fights and marker-free footage remain separate acceptance; unfilmed behavior is not claimed
as visually proven.

## Implemented behavior and asset evidence

Health phases request ordered equipment changes at safe attack boundaries. Smite backs off, points, runs
by a checked walking path to the rack, and spends 2 sec fitting the new loadout. Ordinary damage
pauses the run for 0.45 sec; posture breaks delay his hands. The same transition has a 14 sec
ceiling if Smite survives the transition. If a route becomes blocked, he fits carried spares at his current position and logs the
fallback. He remains damageable and never teleports. No transition attacks or stuns the heroes.
An axe showing of at least 7 sec prevents burst damage from immediately skipping to the hammer.

Sword sweeps and delayed overheads give way to short axe pairs and a delayed crosscut, then
unblockable hammer slams and close stomps with long recoveries. Stage filtering, fixed authored
strings and no random recovery skipping protect those rhythms. Native virtual equipment fields
update the existing creature; newly arriving observers read the same equipped stage. Reset,
wipe and death remove the rack, and a new pull starts with the sword.

Read-only native-data queries on November 8 verified display 2026 as TaurenMale, weapon rack
130 as GeneralWeaponrack01, sword 679, hatchet 2061 and two-handed hammer 2056. The actual map
probe found a 7-step walk from (-2, -768, 9.5) to the rack point (-2, -774, 9.550336). The runtime
also rejects water, a different floor, or an unreachable authored point. Rack 130 measures
1.29 by 1.06 yd with a 2.91 yd model height. Native clips 17, 87, 57, 58 and 181 cover main-hand,
off-hand, special cuts, the hammer and the stomp. The exact grips and impact poses and marker-free readability still require client review. No extracted game
assets are committed. The controlled scenario is `tools/visual-check/smite-arsenal.txt`.

The accompanying Gilnid comments explain the already-probed lower Foundry floor offsets and
why the hammer strings leave extra recovery for a solo hero. They do not change Gilnid mechanics.
