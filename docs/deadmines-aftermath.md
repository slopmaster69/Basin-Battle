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
the cove's stores tent, beach, landing and sea tunnel. A site says only where, how big and how
the rock came down: `Slump` (a cave-in against a wall), `Seal` (a side passage filled to its
lintel), `Lintel` (the Foundry's back door: the lintel down on a pile against one jamb) or
`Spill` (loose rock and timber under a whole roof). The ship's former quarters and deck are a
seabed-backed collapse basin, not props balanced on the old deck floor.

`aftermath_piles.rs` fits every pile to the installed map when the preview starts (the survey
from `htl_map::deadmines::load_aftermath_survey`: the floors, every solid triangle, and the box of
every native doodad still standing). A pile slumps against the wall nearest it, or the one that
leaves it farthest from the walk from the entrance to the cove. It is made of one rock mesh
(Burning Steppes boulder 05, the dark ash rock of the mine's own rubble) in three sizes: a large
mass with its flat cut face (model yaw -37 degrees) turned into the wall, as large as the roof
lets it stand (a collapse must read bigger than the rocks set down before it), a second large mass
beside it, pieces of a middle size (a man's height or less) as a shoulder and in front, small
stones across the floor, and a gravel skirt of the same dark rock toward the path. Over it the
roof is broken: a beam (the iron-shod excavation pole) snapped out of the roof onto the pile or
hanging from it, and a broken post leaning on the pile, never steeper than 45 degrees. Boards
(the excavation plank) lie at its foot at angles of their own, one pinned under the mass and one
leaning on it. Carts, barrows, machinery, cannons and the mine's own cargo boxes are knocked
over and seated by their turned meshes. The Foundry's back door is stacked up toward its lintel.
The first film (10 Nov) dropped three looks that read badly: the Tirisfal and Duskwood mounds read
as green grass, the rock hung from the roof as a smooth lump floating under it, and the tall 5737
boulder (a stalagmite) and the Dark Iron planks and woodpile (gold) were gone already.

Every piece rests and passes through nothing, by construction: rocks are seated so their rim
is under the floor or the rock below (or inside the wall), wood and iron touch the floor or rock
at both ends of their length (or both sides of a box), are lifted out of a stepped floor, never
reach a wall, the roof, another piece of wood or a native doodad, and are never swallowed by
rock. A rock may bury a native doodad whole, unless it gives light. Nothing knee-high stands on
the walk from the entrance's view to the cove's. A piece that can't meet all of that is left
out, so a few piles lack one part: the ignored native test lists them. The regions' views and the
review's other cameras (`CAMERAS`) keep their footing and the yards in front of them clear.

The cove stores tent (Codex's "pantry", view 13) is the medium tent on the cove's west beach at
(-159, -907.6): the camera stood on the beach at (-146, -906), and the old pile swallowed the tent
(its pale peak poked through the mound's top; the "rope fence" was the tent's guy ropes). The
spill now lies beside it. The two pale green circles on the workshop floor (view 4) are the
mine's own: they show in the same place in Codex's first palette trial, with no ruin there and
no native doodad within 8 yd, so they belong to the floor itself.

What the server can't do: open a real hole in the roof (the WMO's ceiling is one group), topple or tip the mine's own cargo stacks, carts, barrows and
lanterns (they are the WMO's doodads, views 3, 9 and 15), or put out or tilt a lantern. Each needs
a client patch beside 0087 that hides those doodads by index while the marker stands.

The ship marker is generic game-object entry `500100`, display `0`, GUID
`0xF11007A184FFFFFF`. It is sent before the ruin objects and destroyed on reset. Patch 0087
recognizes it on map 36. Ruins use entries `500101` onward, with native display IDs and ordinary
solid flags `0x30`. They never use the client flag that removes collision. The paired map loader
and client filter remove the same ship groups, doodads and supplemental bridges.

The server tests cover deterministic coverage, clear inspection views, the piles' parts, tipped
wreckage, native packet parsing and batch sizes, late observers, marker cleanup, unchanged
simulation and saves, disconnects and repeat activation. Three ignored tests read the installed
map through `WOW_DATA` (`cargo test --release -p htl-server --tests -- --ignored native_`):

- `native_ruins_rest_on_floor_or_rock_and_pass_through_nothing`: the contact check. It measures
  every ruin with its own installed mesh (not the builder's shortcuts) against the map's
  triangles and the native doodads' boxes. Run over the layout on main before this change, it
  failed 294 of 575 props, among them the floating timber of view 2, the floating machinery and
  crossed timber of view 4, the timber of view 6, the rock through the dredge and lantern of view 11
  and the crates in the wall of view 14.
- `native_aftermath_floors_support_every_region_and_no_ship_deck`: floors under every site and
  view, every large pile's main mass, change A and B's parts on every pile (a handful may lack one,
  listed), the ship's floors gone.
- `native_way_from_the_entrance_to_the_cove_stays_open`: the walk between the views, mouth to cove.

The installed-data check currently places 1387 solids with no rejected sites: 71 at the entrance,
267 in the galleries, 84 in the Mast Room, 125 in the workshop, 240 in the Foundry, 265 in the
Cannon Gallery, 210 in the cove and 125 at the landing and sea tunnel. Fitting them takes about
0.25 s after a 1 s survey of the map. Floating wreckage uses the measured native water surface at
-0.023 yd. These counts establish coverage, not visual approval.
