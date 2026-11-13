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

## Since replaced: the collapse from the front door inward

Corey, Nov 13, on the footage of the 1,018 yd gauntlet: "Right now you just have a line of rocks
falling down. But what happens if the player is on the other side of that rubble? Then he's cut
off and cannot escape. The rubble should start from the entrance of deadmines and continue to
fill, fall, and build towards the way you want the player to escape."

The gauntlet back to the ship is gone. The collapse now runs 94.6 yd, from the buried front door
through his post and the foreman's chamber to where the galleries open, and stops 31 yd short of
the Galleries lamp, so a hero who wakes there always walks up to the fight. The native route
resolution, the ordered corner cursor and the walking test above still carry it.

The rules (`sim::bosses`, `[mob.phase.collapse]` in `content/deadmines.toml`):

- **Sections.** Four, each ending at a narrowing: the door pocket and tunnel to his post (x -37),
  the chamber's near end to the pillar (x -50), the rest of the chamber to the corridor's mouth
  (x -78), the corridor to its narrowest (x -91).
- **Surges.** At 70% he yells and the first section goes. A surge is 2 sec of warning, then a wave
  of rock from the section's door end to its front edge at 5 yd/sec. Then the front stands still
  for 12 sec while he fights at the face, and the next section goes. About 61 sec in all.
- **What shows.** From the warning on: a bright band across the full width at the section's front
  edge (the line to get past), dust from the roof over the whole section, the rumble and shake.
  Then rock in rows across the full width, door end first. Behind the wave the floor stays full of
  rock; at the front edge a face of rubble is built to the roof, wall to wall.
- **The face is solid.** Its rocks have hulls on the client, and the server stops a hero at its
  line: a walk, a roll, a jump or a knockback ends a step short of it, from either side. Faces
  stand until the fight is lost or the dungeon resets, whatever the cap on loose rubble.
- **Nobody is cut off.** A living hero behind the wave or behind a face, at any distance to the
  side (the door pocket, the chamber's side cave), is caught: 35% of maximum health gone once a
  surge, set down 4 yd past that surge's line, down for 1 sec. There is no crush any more.
- **Rhahk'Zor** fights on through it, walks out ahead of the wave, holds the line and hurls rock
  at a hero who runs far ahead, and can't be brought below 26% until the last face stands. Then
  the end line, and the last stand (burning maul at 25%, lodged pound at 12%) where it stopped.
- **Other foes** on the stretch when it starts run for it and are gone.
- **A fight lost** puts all of it back: no rock, no face, no stop, and he is whole at his post.
