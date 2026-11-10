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
way, about 10 yd out; the mast (a plain wooden pole 48 yd long, Stormwind's flagpole without its
flag) stepped in the bow half and standing square to its broken deck, so aslant, to 45 yd, well over
the reared bow (at 31 yd, level with it, it read as one more of the hull's ribs), with a spar (its
yard) hanging off it into the water; cannons on the seabed and
three thrown up onto the first level ground of the beach under the landing; the Deadmines'
anchor standing in the shallows there (the anchor the third film saw in a passage on the walk is
the mine's own, at the Cannon Gallery's entrance); a field of floating wreckage (the shipwreck pool's
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
least a fifth more rock than the old layout's. That measure still passed a pile that read smaller
on film (the entrance's, one narrow cone where the dome rose to the beam and spread right to the
cart: rock behind or beside its camera counted). So each view's picture is measured too
(`picture_rock`): in each 4-degree column of it, out to 45 yd, the most any rock the eye can see
(past the mine's own posts and walls) stands over the floor under it. Where a column is shorter
than the old layout's was (`OLD_VIEW_ROCK`, measured with the old rocks' own meshes), large pieces
are set where that view sees them; the entrance's view is held to a third more than its dome.
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
timber lies or stands round their footing or in the first 9 yd of their picture (a pole stood at the lens at
Johnson's, boards and a post filled the workshop's picture), and the hero's own camera, up behind
them, is clear.

An inspector standing where the ruined mine has ground stays there when the preview starts (only
one who might have logged out on the old ship is sent to the landing), and a cameraman stays by
its hero: sent to the landing and back, its client took ten seconds to load the ground again and
filmed the void. The preview takes the cameraman (`.es watch`) where the hero it films inspects,
within 12 yd: the rules stand
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

What the server can't do: open a real hole in the roof (the WMO's ceiling is one group).

The mine's own doodads (`aftermath_doodads.rs`, client patch 0091): the cargo stacks, carts,
barrows and lanterns are the WMO's doodads, so the server can't move them. While the marker stands,
a client of round 30 (patch 0091, `.es hello <version> ac30`) hides those in
`htl_map::deadmines::aftermath::MINE_DOODADS` by their place in the WMO, as 0087 hides the ship's,
and puts out the mine's own light (MOLT) within 1.6 yd of each in `DARK_LANTERNS`. The server sets
its own copies where they stood, to that client alone: an older one keeps the mine's own doodads
and is sent none, so nothing stands twice. A client whose hello comes after the preview began gets
them with its hello. The copies are fitted to the mine without the hidden doodads (the open survey,
`load_aftermath_open_survey`, read once beside the full one), among the piles, which are still
fitted to the mine with them, as an older client shows it. Each passes the contact check, or hangs
askew by its own hook where the mine's lantern hung.

- The Mast Room (view 3): the neat cargo toppled, its kegs and barrels rolled, the netted cargo on
  its side, the large lumber pile spilled and the ore cart tipped.
- The eastern niche (view 9): the stacked lumber spilled, the bucket on it kicked over, the sack
  that leaned on it gone under the seal. Its lantern hangs askew and still burns: the review found
  the niche too dark to read already.
- The buried front door (view 15): the barrow upside down, the stacked lumber spilled, its bucket
  kicked over, and the lantern over the fall gone with the roof, its light out.
- The lanterns: 20 of the mine's 70 lit ones go out and are gone (the entrance's over the
  Foreman's pile, view 1, the front door's, and the dead ends': Johnson's gallery, the eastern side
  tunnel, the corner south of the entrance, the workshop's side passage and rear cell, the cannon
  approach's recess, the Foundry's far bays and the Shredder's loft). A put-out lantern leaves no
  copy: every lantern model in the data has the same glowing yellow glass, and the first film
  (10 Nov) showed the one laid on the front door's rock reading lit. None within 5 yd of the walk from the entrance to the cove goes out, nor
  any on the way in from the portal. Six hang askew and still burn (the entrance's second, view
  1, and the eastern niche's among them). The walls keep the warm light baked into them (MOCV) where a lantern
  went out: only its light on props and heroes goes.

Filmed on 10 Nov (client `doodadsA`): the originals gone with no doubles, the copies seated with
nothing floating or clipping, Johnson's and Marisa's dead ends dark, and the restore exact.

`.es aftermath status` counts the copies. The extra displays the copies need (the hanging lantern
60005, the ore cart 60006, the netted cargo 60007) are patch 0091's own. The ignored data tests
`native_the_mines_own_doodads_rest_or_hang_where_they_stood` (htl-server) and
`native_a_third_of_the_lit_lanterns_go_out_each_with_its_own_light` (htl-map) check them on the
installed map; `dmdoodads <Data> [word...]` (htl-map's examples) lists the mine's doodads by index
with their lights, for choosing more.

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
- `native_the_entrance_pile_fills_its_view_as_the_old_dome_did`: the entrance's picture, column by
  column, taller than the old dome wherever it stood, and a third bigger in all.

The installed-data check currently places 1938 solids with no rejected sites: 104 at the entrance,
389 in the galleries, 100 in the Mast Room, 157 in the workshop, 314 in the Foundry, 313 in the
Cannon Gallery, 375 in the cove and 186 at the landing and sea tunnel. Their rock against the old
layout's: entrance 10793 (4052), galleries 12459 (9487), Mast Room 7954 (5870), workshop 6987
(4514), Foundry 11288 (8146), Cannon Gallery 9139 (6597), cove 15840 (13675), landing 5859 (4869).
Fitting them takes about 0.65 s after a 1 s survey of the map. Floating wreckage uses the measured native water surface at
-0.023 yd. These counts establish coverage, not visual approval.
