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
through his post and the foreman's chamber to where the galleries open. Its last face stands
6.4 yd short of the route's end, and the route ends 31 yd short of the Galleries lamp, so a hero
who wakes there always walks up to the fight. The native route resolution, the ordered corner
cursor and the walking test above still carry it. What follows is what was built
(`sim::bosses`, `[mob.phase.collapse]` in `content/deadmines.toml`); none of its look has been
seen in the client yet.

Heroes come to him from the galleries; the buried door is behind him. So the rubble starts
behind the boss and pushes everyone the way they came.

| Section | Along the route | Its face (front edge) | What it takes |
| --- | --- | --- | --- |
| 1 | 0 to 30.9 yd | x -37, the tunnel's mouth at his post | the door pocket and the tunnel |
| 2 | 30.9 to 45.9 yd | x -51, both sides of the pillar | the chamber's near end |
| 3 | 45.9 to 74.7 yd | x -78, the corridor's mouth | the rest of the foreman's chamber and its side cave |
| 4 | 74.7 to 88.2 yd | x -91, the corridor's narrowest | the corridor |

The faces are at the narrowings the mine has, so the sections are 31, 15, 29 and 14 yd, not four
of equal length.

- **Surges.** At 70% he yells and the first section goes. A surge is 2 sec of warning, then a wave
  of rock from the section's door end to its front edge at 5 yd/sec (a hero runs 7). Then the
  front stands still for 12 sec while he fights at the face, and the next section goes. From the
  yell to the last face: 62 sec. `speed`, `warn` and `rest` in the content file.
- **What shows.** From the warning's first moment: a bright band across the full width of the
  floor at the section's front edge (the line to get past; ground bursts along the face's line,
  pulsed until the face stands), dust from the roof over the whole section, a rumble, and on the
  screen of each hero in the section the shake and the Danger warning. Then rock in rows across
  the full width, door end first, each fall with its ring. Behind the wave the floor stays full
  of rock; at the front edge a face of rubble is built to the roof, wall to wall.
- **The face is solid.** Its rocks have hulls on the client, and the server stops a hero at its
  line: a walk, a roll, a jump or a knockback ends a step short of it, from either side.
  Creatures stop 1.2 yd short of it. Faces stand until the fight is lost or the dungeon resets;
  the cap on loose rubble (400 pieces) never takes a face's rock.
- **Nobody is cut off.** A living hero behind the wave or behind a face, at any distance to the
  side (the door pocket, the chamber's side cave), is caught: 35% of maximum health gone once a
  surge ("the falling mine"), set down on clear floor 4 yd past that surge's line, down for
  1 sec. There is no crush any more. The throw is a set-down, not a flight through the air.
- **Rhahk'Zor** fights on through it. In a wave he walks the route out of the section going, to
  3 yd past its line, faster than the wave; he never backs through a standing face. He holds the
  line and hurls rock at a hero who runs far ahead (14 + 6 yd past the line, or 20 yd off the
  route), and can't be brought below 26% until the last face stands. At 45% his ring of rocks
  comes down round him wherever he is. With the last face up: his end line, and the last stand
  (burning maul at 25%, lodged pound at 12%) at the galleries' opening.
- **Other foes** within a section's reach of the route when it starts run for it and are gone.
- **A fight lost** puts all of it back at once: no rock, no face, no stop, and he is whole at his
  post. It is lost the moment the last hero in it is down (his own blow's recovery doesn't hold
  it open), so a hero who wakes at a lamp finds the mine standing. A hero farther than 150 yd
  from the whole route has left the fight: no rock is hurled after them.
- **He falls:** the faces and the rubble stay until the dungeon resets. They are not saved: a
  server restarted in between comes back with the tunnel clear.

The checks: `sim::bosses` tests (the sim's rules on a flat floor), `tests_below.rs` (the fight
lost) and, in the real mine with the game's data, `crates/htl-server/tests/rhahk_route.rs`: the
route and its sections, nobody cut off on any floor the collapse reaches (a flood of the walkable
floor after each face), the wipe, and a hero who does nothing on the door side.

To film (`tools/visual-check/`, written from server rehearsals and not yet run):
`rhahk-collapse-front` (a hero on the door side caught and thrown clear, the fight at the first
face, the hero stopped at the face), `rhahk-collapse-escape` (three surges outrun: the band, the
rows, each face building) and `rhahk-collapse-wipe` (the hero felled and the mine put back).
