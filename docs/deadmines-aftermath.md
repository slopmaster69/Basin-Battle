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
`Spill` (loose rock and timber under a whole roof).

The ship is wrecked, not just gone (change F). Its own WMO groups stay hidden by patch 0087 (the
server can't tilt them), so the wreck is native props across its old berth (the intact map's deck
ran from x -130 to -2 and y -780 to -860, over a flat seabed at -16): the bow half of the hull
(the wrecked rowboat at 8.5 times its size, 81 yd long) along the west of the berth, its broken
end on the seabed and its bow reared 32 yd out of the water, rolled with its broken open side
toward the landing; the stern half (6.5 times) along the east of the berth, turned the other
way, about 10 yd out; the mast (a plain wooden pole 34 yd long, Stormwind's flagpole without its
flag) stepped in the bow half and standing square to its broken deck, so aslant, to 31 yd, with a
spar (its yard) hanging off it into the water; cannons on the seabed and
three thrown up onto the first level ground of the beach under the landing; the Deadmines'
anchor standing in the shallows there; a field of floating wreckage (the shipwreck pool's
floating stores, floating barrels, driftwood, boards, lumber and crates) in two drifts from the
hull to Mr. Smite's dock, each riding the water; and the dock's planks snapped at its edge and
hanging into the water. Every big piece keeps 40 yd from the dock's cameras (the first take's
stern half filled the dock's view black) and off the walk, and the hull halves, the mast, the
beached cannons and the anchor are each in the landing cameras' picture with nothing of the mine
between. Every piece of the ship has its pivot over the floor or seabed under it, never in it:
the second film never showed the mast footed on the seabed nor the anchor seated under the beach
(the client finds a body's room straight down from its pivot). There is no smokestack model among the game objects (the ship's own stack is one of its
hidden groups), so there is no smokestack, and no mast or yard model either.

`aftermath_piles.rs` fits every pile to the installed map when the preview starts (the survey
from `htl_map::deadmines::load_aftermath_survey`: the floors, every solid triangle, and the box of
every native doodad still standing). A pile slumps against the wall nearest it, or the one that
leaves it farthest from the walk from the entrance to the cove. It is made of one rock mesh
(Burning Steppes boulder 05, the dark ash rock of the mine's own rubble) in three sizes: a large
mass with its flat cut face (model yaw -37 degrees) turned into the wall, as large as the roof
lets it stand (a collapse must read bigger than the rocks set down before it), a second large mass
beside it, pieces of a middle size (a man's height or less) as a shoulder and in front, small
stones across the floor, and a gravel skirt of the same dark rock toward the path. Over it the
roof is broken: a beam (the iron-shod excavation pole) snapped out of the roof down onto the pile
(never a stub left hanging from the roof, which read as a pole floating in the air), and a broken
post leaning on the pile, never steeper than 45 degrees. Boards
(the excavation plank) lie at its foot at angles of their own, one pinned under the mass and one
leaning on it. Carts, barrows, machinery, cannons and the mine's own cargo boxes are knocked
over and seated by their turned meshes. The Foundry's back door is stacked up toward its lintel. (No split sacks: their pale cloth glowed
green by the workshop rear cell's own lantern on film.)

Then rock is heaped on every pile until it holds more than the old layout had there
(`aftermath_layout::OLD_ROCK`, measured as the review measures a pile: each rock's footprint times
how far it stands over the floor, on the installed map). The second film found the entrance's
rebuilt pile one smooth cone smaller than the dome it replaced. Several large pieces are jumbled
over and against each pile's first mass, then smaller ones, each seated on whatever is under its
rim and kept under the roof; where a pile can't take its share (the walk runs by it, the shore
drops into the water), the other piles of its region take more, until every region holds at
least a fifth more rock than the old layout's.
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
review's other cameras (`CAMERAS`) keep their footing and the yards in front of them clear, no
timber stands in the first 9 yd of their picture at eye height (a pole stood at the lens at
Johnson's, boards and a post filled the workshop's picture), and the hero's own camera, up behind
them, is clear.

The preview takes the cameraman (`.es watch`) where the hero it films inspects: the rules stand
still in the preview, so it was never moved, stayed at the landing and filmed the cove's water and
the void past its loaded ground wherever the hero went (no dust, cyan and black frames on the
walk). And it starts after a finished fight: what an ended fight left drawn (a cast whose creature
is gone, a missile's stand-in) no longer refuses it; only a lab scene or danger still marked on
the ground does.

The cove stores tent (Codex's "pantry", view 13) is the medium tent on the cove's west beach at
(-159, -907.6): the camera stood on the beach at (-146, -906), and the old pile swallowed the tent
(its pale peak poked through the mound's top; the "rope fence" was the tent's guy ropes). The
spill now lies beside it. The two pale green circles on the workshop floor (view 4) are the
mine's own: they show in the same place in Codex's first palette trial, with no ruin there and
no native doodad within 8 yd, so they belong to the floor itself.

The air is native too (`aftermath_air.rs`): while the preview lasts, dust rises now and then at
each pile's foot (Charge's landing dust, spell 22792 `DustCloud_Land`, cast as a ground look from
an unseen stand-in on the spot, as Rhahk'Zor's falling mine shows its dust), about a third of the
puffs with the clatter of grit landing (sound kit 1211), and the snapped beams and posts creak
(the Deadmines' wooden door, 3078, and the haunted windmill's timber, 1650). Each source fires
every 5 to 9 s (dust) or 8 to 24 s (creaks), fixed per preview. Every pile has two dust spots
(the first take's cap of 90 left the cove and the landing with none), each cast 0.3 yd over the
floor; `.es aftermath status` counts the spots and the clouds raised so far, and the server logs
the spots when the preview starts. (A spell visual kit played on a game object showed nothing on
film. The second film showed no clouds either: its cameraman never left the landing, which the
piles' branch now fixes.) There is no lasting haze model in the data that reads as
dust (the ground fogs are orange or green); the repeated settling clouds stand in for it.

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
- `native_every_region_has_at_least_the_old_layouts_rock`: each region's rock, by the review's
  measure, at least the old layout's (main at e6eb2081, less the drowned ship's basin).

The installed-data check currently places 1933 solids with no rejected sites: 94 at the entrance,
385 in the galleries, 101 in the Mast Room, 158 in the workshop, 315 in the Foundry, 312 in the
Cannon Gallery, 378 in the cove and 190 at the landing and sea tunnel. Their rock against the old
layout's: entrance 5024 (4052), galleries 11603 (9487), Mast Room 7730 (5870), workshop 6724
(4514), Foundry 11050 (8146), Cannon Gallery 8841 (6597), cove 15840 (13675), landing 5859 (4869).
Fitting them takes about 0.4 s after a 1 s survey of the map. Floating wreckage uses the measured native water surface at
-0.023 yd. These counts establish coverage, not visual approval.
