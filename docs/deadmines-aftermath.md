# Deadmines after the collapse

Codex claims the finished destroyed state of the entire dungeon, at Corey's request.
This is the environment after the heroes and Rhahk'Zor have already passed through it.
The boss fight and a new cinematic spectator system are outside this first change.

## Intended result

- Every tunnel and major room has a composed aftermath: large cave-ins, persistent rubble,
  broken supports and wrecked machinery or cargo appropriate to that room.
- Destruction changes the silhouette of the space. Dust is atmosphere around real solids,
  not the sole representation of a collapse.
- VanCleef's ship is gone from the final chamber. Rendering and collision must agree; hiding
  it behind a pile of props while leaving the intact ship underneath is not acceptance.
- The state is deterministic and inspectable without replaying a boss fight. A scoped preview
  control can select the final state and restore the intact map without touching player saves.
- Native installed assets only; no WoW asset files or extracted game data enter the repository.

## Work and validation

First survey the complete instance, usable rock/timber/wreckage models, and the ship's static
geometry. Author distinct destruction compositions for tunnels, workshop/mast room, foundry,
cannon approaches and ship cavern, including branches and side rooms. Large pieces establish
the damaged shape; smaller debris and restrained residual dust fill it out.

Check actual asset bounds and floors, visual density, visibility range, performance and cleanup.
Inspect the result in the native client from clean static views and camera moves. A debug
command succeeding, a count of spawned objects or a dust effect alone is not visual acceptance.

Client patch work, if required to remove built-in ship geometry, will be coordinated with
Claude's existing patch claims before implementation. Full spectator filming will be a
separate task after this finished environment is established.

## Inspecting the authored state

Use a client built with the ship-removal patch 0087 and a matching server. The preview is a
temporary state of the Deadmines world shared by its inspectors. It is deliberately independent
of boss progression. Enter the Deadmines, then type:

```text
.es debug quiet
.es aftermath on
.es aftermath list
.es aftermath visit foundry
.es aftermath off
```

The eight named views are `mouth`, `galleries`, `mast`, `workshop`, `foundry`, `cannon`, `cove`
and `landing`. `.es aftermath status` reports the object count and any authored sites rejected
for missing floors. `.es aftermath point <x> <y> <z> [facing]` moves an inspector to another dry
floor; it checks the actual collapsed map, including the absence of the ship decks. `.es where`
reports the inspection position. A 0.9 yd standing capsule at each named view stays clear of the
oriented debris bounds, with low rubble and timber aprons beginning nearby.

The first activation reads and caches the ship-free ground from the server's existing `--data`
or `WOW_DATA` folder. No game files are changed. The server freezes the original simulation and
its clock, hides its creatures and game objects, and sends the fixed ruin composition in batches
of 16 solid objects. Each late observer gets the same geometry marker and composition. All
inspectors begin at the retained Smugglers' Landing so a saved position on the former ship cannot
drop an arriving observer into the water.

Movement during inspection belongs to the preview alone. Combat, inventory, progression and
other gameplay inputs are blocked, and the world does not write its save. Turning the preview
off removes its objects and ship marker, redraws the intact world and returns each hero to their
original position. Ordinary movement resumes after the client acknowledges that return, so
inspection packets already in flight cannot overwrite it. Disconnecting keeps the original
hero position, too. The last inspector's
departure restores the normal realm after finishing without a save write. A server restart starts
with the intact world; there is no saved aftermath flag and no edit to a player's saved file.

## Composition and protocol

`aftermath_layout.rs` authors the mine entrance, eastern and western galleries, Johnson's branch,
the eastern niche, timber yard and Mast Room, workshop and its rear cells, both Foundry floors
and their ramp, Marisa's side room, the Cannon Gallery and cove passages, the entire cove rim,
pantry, beach, landing and sea tunnel. The ship's former quarters and deck are replaced by a
seabed-backed collapse basin, not props balanced on the old deck floor.

Large native dark Burning Steppes boulders establish the collapsed space, with shallow leans
and buried skirts so their open undersides stay beneath the floor. Smaller dark fragments
and scattered boards form distinct foregrounds for each
region. Fallen supports, lumber and timber piles connect the falls to the mine's
structure. Wrecked mine carts, broken crates and barrels, tipped machinery and overturned cannons
give each working area its own debris. Wrecked rowboats dress the cove edges. Bounds and pivots
come from the installed meshes, including the rotated bottom of tipped objects. Native appearance
and collision still require the visual check; source bounds alone do not establish acceptance.

The Foreman's chamber uses a broad dark rock seated into the native floor, with smaller stones
and broken supports around its base. All ruin objects use static native props; their geometry
does not depend on a held animation pose.

The ship marker is generic game-object entry `500100`, display `0`, GUID
`0xF11007A184FFFFFF`. It is sent before the ruin objects and destroyed on reset. Patch 0087
recognizes it on map 36. Ruins use entries `500101` onward, with native display IDs and ordinary
solid flags `0x30`. They never use the client flag that removes collision. The paired map loader
and client filter remove the same ship groups, doodads and supplemental bridges.

The server tests cover deterministic coverage, clear inspection views, tilted pivots, native
packet parsing and batch sizes, late observers, marker cleanup, unchanged simulation and saves,
disconnects and repeat activation. The ignored `native_aftermath_floors_support_every_region_and_no_ship_deck`
test reads the user's installed map through `WOW_DATA`; it verifies authored floor coverage,
retained inspection footing and the actual absence of the old ship floors.

The installed-data check currently places 575 solids with no rejected sites: 32 at the entrance,
99 in the galleries, 33 in the Mast Room, 50 in the workshop, 98 in the Foundry, 97 in the Cannon
Gallery, 116 in the cove and 50 at the landing and sea tunnel. Floating wreckage uses the measured
native water surface at -0.023 yd. These counts establish coverage, not visual approval.
