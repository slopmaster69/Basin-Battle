# Mr. Smite: the arsenal

Status: implemented in PR #298; required automated checks passed on November 8, 2026.
Real-client acceptance remains pending.

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
probe found a 7-step walk from (-2, -768, 9.5) to the rack point (-2, -774, 9.550336). On Nov 9
the rack moved to (-2, -777, 9.6): 9 yd to his side of the way in and 19 walking steps from the
dock home, so his run to it crosses the duel camera. The runtime
also rejects water, a different floor, or an unreachable authored point. Rack 130 measures
1.29 by 1.06 yd with a 2.91 yd model height. Native clips 17, 87, 57, 58 and 181 cover main-hand,
off-hand, special cuts, the hammer and the stomp. The exact grips, impact poses and marker-free readability still require client review.
No extracted game assets are committed. The controlled scenario is `tools/visual-check/smite-arsenal.txt`.

The accompanying Gilnid comments explain the already-probed lower Foundry floor offsets and
why the hammer strings leave extra recovery for a solo hero. They do not change Gilnid mechanics.


## Verification and remaining acceptance

Combined source `c350e64` includes main `04e6182` (the Shredder) with both encounters' shared
hooks preserved. `cargo build --release -p htl-server --offline -j2` passed, and `cargo test
--release --offline -j2` passed 1,081 tests with 19 ignored and no failures. The survival crate
passed 912 tests with 4 ignored. These used the Windows GNU setup in `scripts/windows-rust.ps1`.
Repository rules passed across all 13 changed files, with no failures or warnings.

Ten focused simulation tests exercise live AI progression, distinct stage attacks, committed
attack boundaries, ordinary damage and posture delays, the 14 sec bound, a blocked route,
solo/duo target handoff and wipe/repull/death cleanup. Two server field tests check live versus
late-observer gear, removal of the off-hand axe, the hammer's sheath, and ordinary definition
handoffs clearing old shield/bow slots. The existing guard-break fairness test now uses the
axe crosscut and still ensures a broken guard does not expose a reeling hero to the next blow.

The final existing careful-hero controller (seed 7, starting sword, flat terrain) cleared in
75.76 sec, taking 0 damage, with 15 dodges and 10 parries. Its trace includes the sword, axe and
hammer stages. This fixture gives the hero inflated health, uses reaction-based inputs and
runs without the real dungeon geometry or ordinary human mistakes; it is not a client playtest.

A separate nonstop melee-input pressure fixture also uses inflated hero health and flat
terrain with ordinary encounter group scaling. Solo: 51.60 sec, 432 damage taken, axes at
34.75 sec. Duo: 24.10 sec, 310 combined damage taken, axes at 23.45 sec. Both can kill Smite
during his second transition before the hammer is fitted. This is intentional normal lethality,
not proof of normal-health survival or dungeon duo balance. No invulnerability or health
padding forces a cinematic stage.

Native sound entries 5778/5779 resolve to Smite's Combat01/Combat02; 698/700 are the metal
sheath/unsheath cues. Exact spoken playback and all animation silhouettes remain unreviewed.
The controlled recording scenario demonstrates all stages separately and labels its debug
health changes/healing. Still required: real-client appearance and audible cues, normal-health
solo/duo dungeon clears, and a full fight with optional floor markers disabled.
