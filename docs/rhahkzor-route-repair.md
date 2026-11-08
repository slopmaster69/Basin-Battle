# Rhahk'Zor: the native route repair

PR #306, `codex/rhahkzor-native-route`, follows the rollback in #305 (`670e04b`).
The gauntlet is restored after the native walking test passes, with its original tuning.

The take298 capture confirmed that coarse route interpolation placed Rhahk'Zor over
missing upper floors and inside a roof. The walker's fall recovery could then restore
the same airborne start on every step. The active collapse already moves its home;
an original-home leash is not the cause.

The repair:

- Resolves authored collapse legs through the native land, preserving its corners and
  rejecting blocked, wet or wrong-floor endpoints.
- Samples the actual floor between corners; height chords can otherwise cross a hill or deck.
- Follows an ordered corner cursor through hairpins, including a point visited twice.
- Removes the collapse's teleport-to-front fallback. Invalid footing recovers to the last
  verified floor; a second failure without two yards of grounded progress ends the fight.
- Validates native path endpoints and retains their grid-cell centers before joining exact
  coordinates. Checks every crossed cell, including slivers missed by half-yard samples.
- Distinguishes destination floors in the failed-search cache.

The agreed 70% trigger, 4 yd/s front, 14 yd warning, 20% maximum health per second
crush, 26% health floor, 25% burning maul, 12% final pound and smugglers' exit are unchanged.
A movement test is not an unassisted clear;
real client footage follows separately on the coordinated recording rig.

The installed-data test resolves the 35 authored points to 2,038 floor samples over
1,018.16 yd: 254.54 seconds at the unchanged 4 yd/s. It advances the hero along that route
and lets the creature's actual walking code move Rhahk'Zor every 50 ms. Health assistance
isolates movement. It asserts valid footing throughout, no resets/falls, bounded strides,
the 26% hold before arrival and a kill after arrival. A separate native wipe test checks
the original floor, full health and bounded reset events. The shared navigation change
also has a native Smite rack walk check.

Run the installed-data checks with `WOW_DATA` pointing to the 1.12.1 `Data` directory:

```sh
cargo test --release -j1 -p htl-server --test rhahk_route -- --ignored --nocapture
```

On Windows, dot-source `scripts/windows-rust.ps1` before Cargo. Deterministic tests cover
bent routes, repeated hairpin points, curved floors, stacked floors, wet targets, invalid
endpoints, thin blocked-cell crossings and bounded recovery from an already airborne start.

No other boss content, client patch, add-on or game-derived asset belongs in this repair.
