//! Basin Battle's boss beats, beyond what a phase does by itself (`[[mob.phase]]`):
//!
//! - **The run** (`run_to`): the boss breaks off and runs a route to another room, heeding nobody,
//!   shaking rocks loose behind it (`run_falls`). At the route's end it waits for the heroes
//!   (`wait_for`), then takes up the fight there (`arrive_text`, `arrive_kit`), its home from then.
//!   A fight lost there puts it back where its fight began.
//! - **Burning blows** (`burn`): from the phase on, each heavy blow leaves burning ground where
//!   it lands, for a while.
//! - **Debris** (`debris`): rocks brought down over set points as the phase begins, dust first.
//! - **A lodged weapon** (a step's `lodge`): its weapon sticks where the blow lands, and it hauls
//!   at it, open to every blow.
//! - **The collapse** (`collapse`): the place comes down in sections along a route, from its
//!   buried end the way the heroes are to go (Corey, Nov 13: "what happens if the player is on
//!   the other side of that rubble? Then he's cut off and cannot escape. The rubble should start
//!   from the entrance of deadmines and continue to fill, fall, and build towards the way you
//!   want the player to escape").
//!   - Each section goes in a surge: a warning (`warn`), then a wave of falling rock from its
//!     back edge to its front edge (`speed`), then the front holds still (`rest`) and the next
//!     section goes. The boss can't be brought below `hold` until the last one is down.
//!   - Where the wave stops it leaves a face of rubble across the section's front edge, wall to
//!     wall. Nobody passes a face: a hero's walk, roll, jump or throw ends at it
//!     (`face_crossing`), and its rocks are solid on the client.
//!   - Nobody is left behind the rubble. A hero behind the wave or behind a face, however far to
//!     the side, is caught: a share of their health gone (`catch`, once a surge), thrown onto
//!     clear floor past the surge's line (`throw`), down a moment (`down`).
//!   - The boss walks out ahead of each wave and fights at the face between. A hero who runs
//!     far ahead of the fight has it hold the line and hurl rock after them.
//!   - Every other foe on the stretch runs for it as a surge begins, and is gone.
//!   - A fight lost puts the place back: every rock and face gone, the boss whole at its post.
//!     The boss dead, the faces stand until the dungeon resets.
//! - **The slam** (`slam`): the phase begins on the boss's own body, not as a cast (Corey on the
//!   film of Rhahk'Zor's yell, Nov 14: "He shouldn't be casting, but roaring or body slamming the
//!   wall, which should shake the screen and multiple more rocks should come down"). From the
//!   yell it runs to the frame of the collapse's first door if it can be there in time, else to
//!   the nearest wall, swings (`clip`) and strikes it `secs` after the yell. On that one frame
//!   the wall throws dust, every hero in the place is shaken (hardest near it) and rock comes
//!   loose over `volley` seconds: small and middling rocks for show round it and at the door
//!   (no ring, no blow: they hurt nobody), and one rock marked on the floor under each hero near
//!   it, `DEBRIS_WARN` before it lands (step out, or be thrown down). It can be hit all the
//!   while; no stagger, break or hold stops it. With `secs` the collapse's `warn`, the blow is
//!   the moment the first wave starts: the collapse's own times are as they were. Each later
//!   section goes as it pounds the floor where it stands (the same clip, a smaller shake, no
//!   rock of its own), unless a blow of its own is winding up.
//!
//! Each danger shows before it lands (dust before rocks, the flames where they burn), with or
//! without the floor markers.
//!
//! **How the falling mine looks** (`Looks`, Corey on the footage, Nov 9: "NO falling rocks, no
//! debris, the map doesn't appear to be falling apart"; Nov 13: "way more interesting and
//! obvious"). None of it strikes, warns, stops or times anything: the rules above are the same
//! with it off.
//! - Every fall of rock (`rocks_fall`) shows a rock dropping from overhead onto the spot as its
//!   blow lands, its ring on the floor till then (markers or not), and dust thrown up and a
//!   shake for heroes beside it where it lands. The rock stays.
//! - A surge shows a bright band across the full width of its section's front edge from its
//!   first moment to its last (the line to get past), dust from the roof over the whole section,
//!   a rumble, and shaking for every hero in it.
//! - The wave brings rock down in rows across the section, wall to wall, back edge first
//!   (`ROW_STEP`), each rock's ring on the floor before it, and a timber across the tunnel now
//!   and then (`BEAM_STEP`). What falls stays: the section is left full.
//! - A face is built as the wave reaches the line: rocks side by side from wall to wall, layer
//!   on layer to the roof. Each is solid from the moment it lands (a twin with its hull takes
//!   its place: a piece that moves has none). As its first layer lands the heroes near it are
//!   shaken (`BEAT_FACE_NEAR`, `BEAT_FACE_FAR`): a section coming down has weight.
//! - A loose rock doesn't stop dead where it strikes: it hops once, rolls on a little and comes
//!   to rest (`settle`). A face's rocks don't: they stand where they land.
//! - The 45% ring (`rocks_round`) brings a beam down over each of its falls, a ring of timbers
//!   round the boss.
//! - Everything that came down stays until the fight is lost or the dungeon resets: at most
//!   `RUBBLE_MOST` loose pieces (the oldest going first), and every face, whatever the count.
//!   They're the props the finished ruin is built of (`htl-server`'s `aftermath.rs`): the dark
//!   Burning Steppes rocks and a dig site's timber. The loose pieces have no hull.

use std::collections::VecDeque;

use super::mobs::MobState;
use super::*;
use crate::content::{BurnDef, SlamDef};

#[cfg(test)]
#[path = "bosses_tests.rs"]
mod tests;

/// The dust over a spot where rocks are about to come down, and where they land.
const DUST: u32 = 7272;
/// How long the dust shows before the rocks of a set piece fall, how wide they come down, and
/// their blow as a share of the boss's own (heavy: they throw down).
const DEBRIS_WARN: f32 = 1.3;
const DEBRIS_RADIUS: f32 = 3.0;
const DEBRIS_SHARE: f32 = 0.6;
/// Rocks shaken loose behind it on the run: a lighter fall, and a shorter warning.
const FALL_WARN: f32 = 1.0;
const FALL_RADIUS: f32 = 2.5;
const FALL_SHARE: f32 = 0.25;
/// At the route's end it waits this near for a hero (yards), at most this long (seconds).
const WAIT_FOR: f32 = 22.0;
const WAIT_MOST: f32 = 30.0;
/// A leg it has made no way along for this long: it's set down at the leg's end.
const STUCK: f32 = 3.0;
/// It runs its route this much quicker than it chases: well ahead of the heroes, there first.
const RUN_HASTE: f32 = 1.6;
/// A leg taking this many times longer than a straight run along it is helped along.
const LEG_SLACK: f32 = 2.5;
/// Burning ground bites a hero standing in it this often.
const BURN_TICK: f32 = 1.0;
/// How burning ground shows: its edge drawn at its true size (fire's colour, drawn anew every
/// `FIRE_REDRAW` so it goes with the patch), a flame at its heart and `FIRE_FLAMES` round it at
/// `FIRE_FLAMES_AT` of its radius. A patch smaller than `GROUND_LOOK_MIN` gets no ground spell:
/// cast at a point those cover a whole room (filmed, November 13: `Heated Ground` for a 3 yd
/// patch lit the whole cave floor as lava, so no patch could be told from the floor).
const FIRE_COLOUR: &str = "FF7A1A";
const FIRE_REDRAW: f32 = 0.5;
const FIRE_FLAMES: usize = 3;
const FIRE_FLAMES_AT: f32 = 0.55;
/// A blow refused by a collapse's hold flashes on the boss at most this often.
const HELD_FLASH_GAP: f32 = 0.6;
/// The end of a collapse: dust thrown out along its last face, a cloud every this far.
const END_DUST_GAP: f32 = 2.5;
/// The collapse: how often it catches a hero behind the rubble, warns those in a section going,
/// drops a lighter fall ahead of its wave, and raises dust.
const CATCH_TICK: f32 = 0.25;
const DANGER_EVERY: f32 = 1.0;
const WARN_EVERY: f32 = 1.1;
const DUST_EVERY: f32 = 0.6;
/// A hero this far behind the wave is under it. The boss is put ahead of its own collapse only
/// when it's this much farther behind than that.
const WAVE_SLACK: f32 = 0.5;
const BOSS_SLACK: f32 = 1.5;
/// In a wave the boss walks out to this far past the section's line.
const BOSS_CLEAR: f32 = 3.0;
/// A later section's pound (a boss with a slam) is begun no nearer than this to the end the
/// section's wave starts at: at the wave's speed, the time it has to walk out after.
const POUND_AHEAD: f32 = 8.0;
/// ...faster than the wave by this much at the least, whatever slows it (nothing holds it in
/// its own falling mine).
const BOSS_OUTRUN: f32 = 1.3;
/// Walking the route, the leg it is on is looked for this far on from the last one only (the
/// far side of a hairpin is never taken for it).
const LEG_LOOK: f32 = 6.0;
/// A face of rubble stops a hero this far short of its line, reaches this far past its ends
/// (into the walls), and stands for a hero from this far under its floor to this far over its
/// roof.
const FACE_STOP: f32 = 0.4;
/// A creature on its own feet stops this far short of one (its back to the rubble, not in it).
const FACE_SHY: f32 = 1.2;
const FACE_PAD: f32 = 1.0;
const FACE_BELOW: f32 = 6.0;
const FACE_ABOVE: f32 = 4.0;
/// A foe on the stretch as a surge begins runs for it this long, then is gone.
const FLEE_SECS: f32 = 2.5;
/// A collapse with no sections of its own is one: its face this far short of the route's end,
/// this far to each side of the route.
const FACE_SHORT: f32 = 6.0;
const FACE_HALF: f32 = 8.0;
/// A hero farther than this off the route is not on it (another floor, a side cave).
const OFF_ROUTE: f32 = 20.0;
/// Its hero this far past the warning band has run off: the boss holds the line, and hurls rock at
/// them this often.
const LINE_SLACK: f32 = 6.0;
/// A hero stopped at a face is put on the floor there when it is at most this far above them.
const FACE_FLOOR: f32 = 3.0;
const HURL_EVERY: f32 = 4.0;
/// A hero farther than this from the collapse's whole route hasn't run off ahead of it: they've
/// left the fight (woken at a lamp elsewhere, carried off). Far past the lamp beyond the route's
/// end, so nobody running from it gets here on foot before it's over.
const FIGHT_LEFT: f32 = 150.0;

// --- how it looks (`Looks`): nothing below strikes, warns or times anything ----------------------

/// The props, by `GameObjectDisplayInfo` id, measured from the installed meshes (`godisp`), the
/// ones the finished ruin is built of: `BurningSteppesBoulders05.mdx` (a broken dark rock, z
/// 0.41..3.61), `BurningSteppesBoulders01.mdx` (a tall dark boulder, z -0.10..9.42), and the dig
/// site's `ExcavationBarrierPole.mdx` (a timber post, z -5.77..23.87 along its up, 2.4 thick).
pub(crate) const ROCK: u32 = 5741;
pub(crate) const BOULDER: u32 = 5737;
/// The collapse's own rock is pale, so a lamp's light shows it in a dark mine (the ruin's rock is
/// near black there): Westfall's sea boulders, `WestFallBoulder02.mdx` (a broad one, x
/// -9.12..8.60, y -10.41..8.60, z -0.98..7.00) and `WestFallBoulder01.mdx` (a tall one, x
/// -8.11..7.76, y -9.47..9.78, z -1.26..12.31). Both are made to stand with z 0 on the ground.
/// `PALE_SIZE` of the broad one is as big as the broken dark rock at full size.
pub(crate) const PALE: u32 = 4316;
pub(crate) const PALE_TALL: u32 = 4315;
const PALE_SIZE: f32 = 0.36;
pub(crate) const BEAM: u32 = 1508;
const ROCK_BOTTOM: f32 = 0.41;
const BOULDER_BOTTOM: f32 = -0.10;
/// A rock is sunk this much of its size below its lowest point, as the ruin's are, so its open
/// underside stays under the floor.
const EMBED: f32 = 0.4;
/// How far under its resting level a rock's own middle is put, for each yard of its size.
fn rock_sink(display: u32) -> f32 {
    match display {
        BOULDER => BOULDER_BOTTOM + EMBED,
        PALE | PALE_TALL => 0.3,
        _ => ROCK_BOTTOM + EMBED,
    }
}
const BEAM_LEN: f32 = 23.87;
const BEAM_HALF: f32 = 1.2;
/// Sizes: a fall's rock (about 1.7 by 2.3 yd, a yard high; a heavy fall's a little more), the
/// wave's rocks (4 to 6 yd across, 2 to 3 high) and its boulders, and a beam (7 yd long).
const CHUNK_SCALE: f32 = 0.3;
const CHUNK_HEAVY: f32 = 0.36;
const ROW_SCALES: [f32; 4] = [0.29, 0.34, 0.25, 0.38];
const ROW_BOULDER: f32 = 0.3;
pub(crate) const BEAM_SCALE: f32 = 0.3;
/// Rock falls from this high over the floor (lower where a floor lies overhead, never under
/// `DROP_LOW`), in the last `ROCK_SECS` before its blow lands (0.45 s read on film as a rock
/// appearing on the floor: a 5 yd fall from rest takes about 0.7 s).
const DROP_HIGH: f32 = 5.0;
const DROP_LOW: f32 = 2.0;
pub(crate) const ROCK_SECS: f32 = 0.7;
/// A loose rock that has struck the floor hops this high once and rolls on, at rest
/// `SETTLE_SECS` later (a fall's rock, a row's on the floor, a row's on the pile).
const SETTLE_SECS: f32 = 0.3;
const HOP_FALL: f32 = 0.35;
/// How far a settling rock rolls on from where it struck, for each yard of its hop, and how far
/// it turns as it does (radians).
const SETTLE_ROLL: f32 = 1.6;
const SETTLE_TURN: f32 = 0.7;
const HOP_ROW: f32 = 0.22;
const HOP_PILE: f32 = 0.1;
/// A beam shows cracked overhead `BEAM_HOLD`, then comes down over `BEAM_SECS`: its loose end
/// swings to the floor (the first `BEAM_SWING` of it), then the rest crashes down flat.
const BEAM_HOLD: f32 = 0.3;
pub(crate) const BEAM_SECS: f32 = 0.8;
const BEAM_SWING: f32 = 0.55;
/// A surge's wave: a row of rock across the section every `ROW_STEP` it comes (the first
/// `ROW_FIRST` in, none within `ROW_SHORT` of the line, where the face is built), a rock every
/// `ROW_GAP` along the row out to the walls (`ROW_SIDE` at most to each side, a gap of one
/// passed over: a pillar), each landing as the wave reaches it, `ROW_SECS` after its ring shows
/// (`ROW_RING` wide, on at most `ROW_RINGS` of a row: the client draws 16 marks at once). A beam
/// every `BEAM_STEP`.
pub(crate) const ROW_STEP: f32 = 4.0;
const ROW_FIRST: f32 = 1.5;
const ROW_SHORT: f32 = 4.5;
const ROW_GAP: f32 = 4.5;
const ROW_SIDE: usize = 9;
const ROW_SECS: f32 = 0.8;
/// A row is piled: further layers of rock on its first, each this far over the one below and
/// landing this long after it, a layer for every `ROW_FILLS` of roof (at most `ROW_LAYERS`). The
/// stretch behind the wave fills toward its roof as you watch.
pub(crate) const ROW_RISE: f32 = 2.3;
const ROW_LAYER_SECS: f32 = 0.35;
const ROW_FILLS: f32 = 3.0;
pub(crate) const ROW_LAYERS: usize = 3;
/// Seconds a row's rock is on its way down from the roof (it lands as the wave reaches it, as
/// before: it starts down sooner).
const ROW_FALL_SECS: f32 = 0.8;
pub(crate) const BEAM_STEP: f32 = 14.0;
/// A face of rubble, of the broken rock (5.7 by 7.7 yd and 3.2 high at full size) sized by the
/// roof (`FACE_ROOF` yards of roof to a rock of full size): its long side along the face, one
/// every `FACE_PITCH` of its size from wall to wall, its near side `FACE_GAP` behind the line
/// (`FACE_BACK` of its size to its middle), a layer every `FACE_RISE` of its size up to the
/// roof (`FACE_LAYERS` at most), each layer `FACE_LEAN` farther back, the first landing
/// `FACE_FIRST` after the wave reaches the line and each `FACE_LAYER_SECS` after the one below.
/// The pale rock that fell stays, as what's seen of it; its solid twin (the dark rock, with the
/// hull) shows behind it as it lands.
const FACE_ROOF: f32 = 8.0;
const FACE_PITCH: f32 = 5.8;
const FACE_GAP: f32 = 0.25;
const FACE_BACK: f32 = 2.62;
/// What's seen of a face is the pale rock: one over each of its dark rocks, `FACE_PALE` of the
/// size that would match it, its near side `FACE_PALE_GAP` behind the line (`FACE_PALE_BACK` of
/// its size to its middle): just proud of the dark rock, and short of where a hero is stopped.
/// It has no hull (the pale mesh's own let a hero who hopped into it through the floor).
const FACE_PALE: f32 = 1.15;
const FACE_PALE_BACK: f32 = 3.2;
const FACE_PALE_GAP: f32 = -0.1;
const FACE_RISE: f32 = 2.0;
const FACE_LAYERS: usize = 7;
const FACE_LEAN: f32 = 0.5;
const FACE_FIRST: f32 = 0.5;
const FACE_LAYER_SECS: f32 = 0.3;
/// The band across a surge's line: bright (not a blow's colour: it's where to go), this wide,
/// drawn anew every `BAND_EVERY` for `BAND_SECS` (each a pulse from the route out to the walls).
const BAND_COLOUR: &str = "FFE27A";
const BAND_WIDTH: f32 = 5.0;
const BAND_EVERY: f32 = 0.5;
const BAND_SECS: f32 = 1.1;
/// The lamps along a surge's line, on its clear side: tall torches (`FreestandingTorch04_HUGE`,
/// a 2 yd post under a flame as tall again) from wall to wall, this far apart and this far past
/// the line (none where the route crosses it: that floor is where the caught land). They stand from the surge's first moment, and light the face that comes to
/// stand behind them, and the fight held in front of it.
pub(crate) const LINE_LAMP: u32 = 2430;
const LINE_LAMP_SCALE: f32 = 1.2;
const LINE_LAMP_GAP: f32 = 4.5;
const LINE_LAMP_AHEAD: f32 = 3.0;
const LINE_LAMP_CLEAR: f32 = 2.0;
/// Dust from the roof over a section going: this many spots each dust beat, out to this far to
/// each side of the route.
const ROOF_DUST: usize = 18;
const ROOF_DUST_SIDE: f32 = 9.0;
/// The dust across the wave's front, wall to wall: a cloud every this far across.
const FRONT_DUST_GAP: f32 = 4.0;
/// A standing face keeps settling: dust at a spot along its foot this often, and half way up it.
const FACE_DUST_EVERY: f32 = 0.9;
/// The most loose pieces left lying (the oldest, farthest behind, goes first), and in the air
/// at once (a face's rocks beside).
pub(crate) const RUBBLE_MOST: usize = 400;
const FALLING_MOST: usize = 160;
/// `Plant Thornling`'s ground look (`DustCloud_Land.mdx`, Charge's dust, nothing but dust): the
/// dust rolling ahead of the front, and thrown up where a piece lands. From an unseen stand-in on
/// the spot (its own cast pose isn't anyone's). Not `Rocky Ground Impact` (26271): on film (take
/// 344, and Smite's slam) its boulders show pink and linger.
const DUST_ROLL: u32 = 22792;
const LAND_DUST: u32 = DUST_ROLL;
/// A landing's ring on the floor (drawn whatever the markers), dust brown.
const DUST_COLOUR: &str = "A88E6A";
/// `DeadmineDoor01Destroy` (wood smashing) as a beam cracks, `BlackRockDoorSingleDestroy` (stone
/// crashing) as it and the wave's rocks land, `EarthquakeCameraShake` (a deep rumble) from the
/// boss every `RUMBLE_EVERY` of a surge.
const SOUND_CRACK: u32 = 3079;
const SOUND_CRASH: u32 = 6533;
const SOUND_RUMBLE: u32 = 1485;
const RUMBLE_EVERY: f32 = 3.0;
/// The shaking: `BEAT_RUMBLE` for a hero in a section going or this near its wave, each dust
/// beat; `BEAT_ROCKFALL` for one this near a piece landing, at most once in `SHAKE_GAP` (the
/// add-on's `Cues.lua`).
pub(crate) const BEAT_RUMBLE: &str = "rumble";
pub(crate) const BEAT_ROCKFALL: &str = "rockfall";
const RUMBLE_NEAR: f32 = 20.0;
const SHAKE_NEAR: f32 = 6.0;
const SHAKE_GAP: f32 = 0.3;
/// A section's face landing shakes the heroes this near it (and less, this near): the wave has
/// come all the way down.
pub(crate) const BEAT_FACE_NEAR: &str = "face_near";
pub(crate) const BEAT_FACE_FAR: &str = "face_far";
const FACE_SHAKE_NEAR: f32 = 14.0;
const FACE_SHAKE_FAR: f32 = 32.0;
/// The slam's blow shakes every hero in the place, by how far from it they stand (the add-on's
/// `Cues.lua`: the hardest is past the hardest blow a hero takes). A later section's pound is
/// one size down.
pub(crate) const BEAT_SLAM_NEAR: &str = "slam_near";
pub(crate) const BEAT_SLAM_MID: &str = "slam_mid";
pub(crate) const BEAT_SLAM_FAR: &str = "slam_far";
const SLAM_NEAR: f32 = 12.0;
const SLAM_MID: f32 = 35.0;
/// (On open land, with no place round it: nobody farther than this is shaken.)
const SLAM_FAR: f32 = 120.0;
/// Running to its wall it goes no faster than this, and has arrived this near its spot; walls
/// are looked for this many ways round it, a step of `SLAM_STEP` at a time; the first door's
/// frame is looked for this far behind the route's start; a wall toward the door counts as this
/// much nearer than one away from it; the wall is struck this high over its foot.
const SLAM_RUN_MOST: f32 = 14.0;
const SLAM_ARRIVE: f32 = 0.4;
const SLAM_WAYS: usize = 16;
const SLAM_STEP: f32 = 0.5;
const SLAM_DOOR_LOOK: f32 = 8.0;
const SLAM_DOOR_LEAN: f32 = 2.0;
const SLAM_HIGH: f32 = 2.2;
/// The rock it shakes loose, for show: the collapse's pale rock in three sizes (display, scale,
/// its hop as it settles), about 1.2, 2 and 3 yd across. Each is `LOOSE_SECS` on its way down,
/// lands no nearer a hero than `LOOSE_CLEAR` (as they stood at the blow; `LOOSE_WIDER` more for
/// each size up) nor nearer the lit band than `LOOSE_BAND`, and none is marked: it has no blow.
const LOOSE: [(u32, f32, f32); 3] = [(PALE, 0.07, 0.5), (PALE_TALL, 0.1, 0.32), (PALE, 0.16, 0.16)];
const LOOSE_SECS: f32 = 0.75;
const LOOSE_CLEAR: f32 = 3.0;
const LOOSE_WIDER: f32 = 2.5;
const LOOSE_BAND: f32 = 3.5;
/// The marked rocks: one under each hero near it, at most this many.
const SLAM_MARKED: usize = 5;
/// The catch throws its hero clear through the air: at about this speed, in the air no less and
/// no longer than this, and only where the floor between is open to within `THROW_MISS` of the
/// landing (anywhere else they are set down there, as before). A throw shorter than
/// `THROW_LEAST` is a set-down too. It lands before the section's face has rock in it (no later
/// than the face's first moment after the wave reaches the line), so it never flies into a face.
/// `THROW_GRACE` after it should have landed, a hero still behind the rubble is set down clear
/// all the same: nobody is behind a face `FACE_FIRST + THROW_GRACE` after the wave stops.
const THROW_SPEED: f32 = 20.0;
const THROW_AIR: (f32, f32) = (0.5, 1.4);
const THROW_MISS: f32 = 1.5;
const THROW_LEAST: f32 = 2.0;
const THROW_LIFT: f32 = 2.5;
const THROW_GRACE: f32 = 0.4;
/// A fight lost: the rock stands this long in its dust, then goes over this long (the newest
/// first, back toward the door), dust where it goes.
const LEAVE_HOLD: f32 = 2.0;
const LEAVE_SECS: f32 = 1.5;

#[derive(Debug, Clone, Default)]
pub(crate) struct Bosses {
    /// Bosses on the run, by creature.
    runs: BTreeMap<Id, Run>,
    /// Where a boss that ran began its fight, to be put back there once the fight is lost.
    began: BTreeMap<Id, Vec3>,
    /// Bosses whose heavy blows burn, and how.
    burning: BTreeMap<Id, BurnDef>,
    /// Bosses hauling at a lodged weapon, and until when.
    lodged: BTreeMap<Id, f32>,
    /// Burning ground.
    fires: Vec<Fire>,
    /// The place coming down behind the heroes, if a boss has brought it down.
    collapse: Option<Collapse>,
    /// The faces of rubble standing (the rule: nobody passes one).
    faces: Vec<Face>,
    /// Foes running from a collapse, and when each is gone.
    fleeing: Vec<(Id, f32)>,
    /// Heroes a collapse's catch has thrown, in the air on their way clear.
    thrown: BTreeMap<Id, Thrown>,
    /// Bosses that began a phase with their own body (`slam`), from the yell on.
    slams: BTreeMap<Id, Slam>,
    /// What the falling mine looks like.
    pub(crate) looks: Looks,
}

/// A boss bringing its phase on with its own body (`[mob.phase.slam]`), or pounding the floor
/// where it stands as a later section goes.
#[derive(Debug, Clone)]
struct Slam {
    def: SlamDef,
    /// The spot it strikes (on the wall, or the floor before it) and where it stands to do it.
    wall: Vec3,
    stand: Vec3,
    what: Struck,
    /// When its blow lands, and until when it is at nothing else.
    strike_at: f32,
    until: f32,
    swung: bool,
    struck: bool,
    /// The yell's own: the whole place shakes and rock comes loose. A later section's pound
    /// shakes less and brings nothing down of its own.
    opening: bool,
}

/// What a slam strikes: the frame of the collapse's first door, a wall, or (no wall in reach,
/// or a later section's pound) the floor before it.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Struck {
    Door,
    Wall,
    Floor,
}

/// A hero thrown clear by a collapse's catch: when they land (and are thrown down there), and
/// until when the catch leaves them to their flight.
#[derive(Debug, Clone, Copy)]
struct Thrown {
    boss: Id,
    lands: f32,
    until: f32,
    down: f32,
    landed: bool,
}

/// The falling mine's pieces: on their way down, and lying where they fell.
#[derive(Debug, Clone, Default)]
pub(crate) struct Looks {
    falling: Vec<Falling>,
    /// What came down and stays, oldest first: (whose, the prop).
    rubble: VecDeque<(Id, Id)>,
    /// The rocks of the faces, solid where they landed: (whose, the prop). Never counted out.
    face_pieces: Vec<(Id, Id)>,
    /// The props with a hull (a face's rocks once landed): drawn as any object, not as a piece
    /// that moves.
    solid: BTreeSet<Id>,
    /// A face's pale rocks, lying where they fell over their solid twins (they have no hull):
    /// (when it goes: never while the face stands, whose, the prop).
    swaps: Vec<(f32, Id, Id)>,
    /// The lamps along the lines: (whose, the prop).
    lamps: Vec<(Id, Id)>,
    /// A lost fight's rock on its way out: (whose, since when, how many pieces it began with,
    /// whether the first of it has gone).
    leaving: Option<(Id, f32, usize, bool)>,
    /// Every piece's tilt (falling or lying), for the drawing.
    tilt: BTreeMap<Id, f32>,
    /// When each hero last shook from a landing.
    shook: BTreeMap<Id, f32>,
    /// A count for the pieces' own variety (never the rules' dice, whose draws stay as they were).
    seq: u32,
    /// (Tests) none of it shown, to prove the rules don't change with it.
    pub(crate) off: bool,
}

#[derive(Debug, Clone)]
struct Falling {
    owner: Id,
    /// Its prop, once it shows.
    id: Option<Id>,
    shows: f32,
    start: f32,
    land: f32,
    display: u32,
    scale: f32,
    path: Path,
    /// Where it lands (the floor), and how wide its ring and the shaking round it.
    at: Vec3,
    ring: f32,
    /// Its sound as it starts down (0: none) and as it lands.
    crack: u32,
    crash: u32,
    landed: bool,
    /// One of a face's rocks: solid once it lands, and never counted out.
    face: bool,
    /// A face's rock's hull, standing from when it lands: display, scale, where, facing.
    hull: Option<(u32, f32, Vec3, f32)>,
    /// Having struck the floor (`land`: its dust, crash and shaking are then), it hops `hop`
    /// high and rolls on for `settle` seconds before it lies still (0: it stays where it struck).
    settle: f32,
    hop: f32,
    struck: bool,
}

#[derive(Debug, Clone, Copy)]
enum Path {
    /// Straight down from `from` to `to` (its resting place), turning as it goes.
    Drop { from: Vec3, to: Vec3, yaw: f32, tilt: (f32, f32) },
    /// A beam hinged overhead at `hinge`, lying along `yaw`: its loose end swings to the floor
    /// (`floor`), then (`flat`) the rest comes down after it.
    Beam { hinge: Vec3, yaw: f32, floor: f32, flat: bool },
}

/// A collapse under way (`[mob.phase.collapse]`).
#[derive(Debug, Clone)]
struct Collapse {
    boss: Id,
    route: Vec<Vec3>,
    /// How far along the route each point is (yards).
    along: Vec<f32>,
    /// Its sections in the order they come down, and the one going (or the last one down).
    sections: Vec<Section>,
    section: usize,
    stage: Stage,
    /// How far along the route the rubble has come: the wave in a surge, the newest face between.
    front: f32,
    speed: f32,
    lead: f32,
    hold: f32,
    warn: f32,
    rest: f32,
    catch: f32,
    throw: f32,
    down: f32,
    end_text: String,
    held_text: String,
    /// What shows on the boss while it is held (from the first blow refused until the last
    /// section is down), on each blow refused, and from the end of the collapse on.
    held_aura: u32,
    held_kit: u32,
    end_aura: u32,
    end_kit: u32,
    end_grow: f32,
    /// When a refused blow next flashes on it.
    flash_at: f32,
    /// Its last section is down: the last of the fight.
    done: bool,
    /// The heroes this surge has caught (each pays once a surge).
    caught: BTreeSet<Id>,
    /// Its own beats: the next catch of whoever is behind the rubble, warning to the heroes in
    /// the section going, lighter fall ahead of the wave, dust, and band across the line.
    catch_at: f32,
    danger_at: f32,
    warn_at: f32,
    dust_at: f32,
    band_at: f32,
    /// The boss was kept from falling before the end (said once).
    held: bool,
    /// When it next hurls rock at a hero who ran off ahead.
    hurl_at: f32,
    /// Next walking corner while it walks the route. Kept in order through hairpins where
    /// the same position occurs twice; a nearest-point projection alone can go backwards.
    boss_leg: usize,
    holding: bool,
    /// How far along the route the wave's rows and beams have come down, and the next rumble.
    rowed: f32,
    beamed: f32,
    rumble_at: f32,
    /// The boss's slam, if its phase has one: each later section goes as it pounds the floor.
    slam: Option<SlamDef>,
    /// When the face just built lands its first layer (the heroes near it shaken then).
    thud_at: f32,
    /// A later section's pound still owed (the boss was winding up a blow of its own when the
    /// section's warning began): begun the moment it is free, if its blow can still land by
    /// the wave. Infinite: none owed.
    pound_by: f32,
}

/// One section of a collapse, laid on its route.
#[derive(Debug, Clone)]
struct Section {
    /// Where along the route it begins (the face before it, or the route's start) and where its
    /// own face crosses the route, and that crossing.
    from: f32,
    line: f32,
    at: Vec3,
    /// Its face, from one wall to the other, and the roof over the floor there.
    a: [f32; 2],
    b: [f32; 2],
    height: f32,
    /// How far to the side of the route its floor runs.
    reach: f32,
    /// Which side of the face lies ahead, the way the heroes go (1 or -1: `side`'s sign).
    ahead: f32,
    /// Its face is its own (`[[mob.phase.collapse.section]]`), not one laid square across the
    /// route for want of any.
    authored: bool,
    /// Its own stretch of the route, and how far along the route each point of it is.
    stretch: Vec<Vec3>,
    stretch_along: Vec<f32>,
}

impl Section {
    /// How far ahead of its face's line a spot is (negative: behind it, the buried side).
    fn side(&self, pos: Vec3) -> f32 {
        let u = [self.b[0] - self.a[0], self.b[1] - self.a[1]];
        let len = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1.0e-3);
        (u[0] * (pos[1] - self.a[1]) - u[1] * (pos[0] - self.a[0])) / len * self.ahead
    }
}

/// Where a section's surge stands: its warning, its wave coming down, or down (the front held
/// still at its face until the next section goes).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Stage {
    Warn { until: f32 },
    Wave,
    Rest { until: f32 },
}

/// A face of rubble standing across a tunnel, wall to wall: nobody passes it.
#[derive(Debug, Clone)]
struct Face {
    owner: Id,
    a: [f32; 2],
    b: [f32; 2],
    /// Its floor, and the roof over it.
    z: f32,
    height: f32,
}

impl Collapse {
    fn total(&self) -> f32 {
        *self.along.last().unwrap()
    }

    /// The newest section whose rubble counts: the one going once its wave has begun, else the
    /// last one down. None before the first wave.
    fn limit(&self) -> Option<usize> {
        match self.stage {
            Stage::Warn { .. } => self.section.checked_sub(1),
            _ => Some(self.section),
        }
    }

    /// Whether a spot is in section `k` (behind its face's line by more than `margin`, and
    /// within its reach of its stretch of the route), and how far along the route it is there.
    fn in_section(&self, k: usize, pos: Vec3, margin: f32) -> Option<f32> {
        let s = &self.sections[k];
        if s.authored {
            if s.side(pos) >= -margin {
                return None;
            }
        } else {
            let (at, _) = route_progress(&self.route, &self.along, pos)?;
            if at >= s.line - margin {
                return None;
            }
        }
        let (at, off) = route_progress(&s.stretch, &s.stretch_along, pos)?;
        (off <= s.reach).then_some(at)
    }

    /// Whether a spot is behind the rubble (by more than `margin`): behind a standing face, or
    /// behind the wave in the section going, however far to the side.
    fn behind(&self, pos: Vec3, margin: f32) -> bool {
        let Some(limit) = self.limit() else { return false };
        (0..=limit).any(|k| {
            let Some(at) = self.in_section(k, pos, margin) else { return false };
            k != self.section || self.stage != Stage::Wave || at < self.front - WAVE_SLACK - margin
        })
    }

    /// Whether a spot is right out of the fight: farther than `FIGHT_LEFT` from the whole route.
    fn left(&self, pos: Vec3) -> bool {
        route_progress(&self.route, &self.along, pos).is_none_or(|(_, off)| off > FIGHT_LEFT)
    }

    /// Whether a spot is on the stretch the collapse takes: within a section's reach of the
    /// route, or (past the last face) as near the route as a hero on it would be.
    fn on_stretch(&self, pos: Vec3) -> bool {
        let Some((at, off)) = route_progress(&self.route, &self.along, pos) else { return false };
        off <= self.sections.iter().find(|s| at <= s.line).map_or(OFF_ROUTE, |s| s.reach)
    }
}

#[derive(Debug, Clone)]
struct Run {
    /// The phase (its index) whose route it runs.
    phase: usize,
    /// The point it's making for.
    leg: usize,
    /// When it next shakes rocks loose.
    falls_at: f32,
    /// When it reached the route's end (waiting there for the heroes).
    arrived: Option<f32>,
    /// Where it last made headway, and when: a run that stands still this long is helped along.
    went_from: Vec3,
    went_at: f32,
    /// When it set off on this leg, and from where: a leg taking far longer than a run along it
    /// would is helped along too.
    leg_at: f32,
    leg_from: Vec3,
}

#[derive(Debug, Clone)]
struct Fire {
    at: Vec3,
    radius: f32,
    damage: u32,
    until: f32,
    bite_at: f32,
    owner: Id,
    /// Its flames, and when its edge is next drawn.
    objs: Vec<Id>,
    ring_at: f32,
}

impl Survival {
    /// A boss's phase has begun (`enrage`): its beats beyond the phase's own.
    pub(crate) fn begin_boss_beats(&mut self, mid: Id, phase: usize) {
        let Some((def, home)) = self.mobs.get(&mid).map(|m| (m.def, m.home)) else { return };
        let Some(e) = self.content.mobs[def].phases.get(phase).cloned() else { return };
        if let Some(b) = e.burn {
            self.bosses.burning.insert(mid, b);
            // Its weapon's heat shows on it from now.
            self.events.push(Event::Auras { id: mid });
        }
        if !e.debris.is_empty() {
            let damage = self.boss_blow(mid, DEBRIS_SHARE);
            for p in &e.debris {
                self.rocks_fall(mid, *p, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
            }
        }
        if e.rocks_round > 0.0 {
            let at = self.mobs[&mid].pos;
            let damage = self.boss_blow(mid, DEBRIS_SHARE);
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0 + 0.4;
                let p = [at[0] + a.cos() * e.rocks_round, at[1] + a.sin() * e.rocks_round, at[2]];
                self.rocks_fall(mid, p, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
                // The beam over each comes down with its rock, lying round the ring's edge.
                let p = self.ground_near(p[0], p[1], p[2]);
                let yaw = a + std::f32::consts::FRAC_PI_2;
                let hinge = [p[0] - yaw.cos() * 3.0, p[1] - yaw.sin() * 3.0, p[2]];
                let land = self.clock + DEBRIS_WARN;
                self.beam_falls(mid, hinge, yaw, land, true);
            }
        }
        // Its slam begins before its collapse does: the first surge is brought on by its body,
        // not roared down.
        if let Some(sl) = &e.slam {
            let door = e.collapse.as_ref().filter(|c| c.route.len() >= 2).map(|c| (c.route[0], c.route[1]));
            self.slam_begins(mid, sl, door, true);
        }
        if let Some(c) = e.collapse.filter(|c| c.route.len() >= 2) {
            let name = self.content.mobs[def].name.clone();
            let Some(route) = crate::land::walking_route(self.land.as_ref(), &c.route) else {
                self.events.push(Event::Log(format!("{name}: collapse route has no safe walk; phase stopped")));
                return;
            };
            let now = self.clock;
            self.bosses.began.entry(mid).or_insert(home);
            let mut along = vec![0.0];
            for w in route.windows(2) {
                let d = dist3(w[0], w[1]);
                along.push(along.last().unwrap() + d);
            }
            let (sections, missed) = collapse_sections(&route, &along, &c.sections);
            for k in missed {
                self.events.push(Event::Log(format!("{name}: collapse section {} has no face of its own across the route; left out", k + 1)));
            }
            let speed = c.speed.max(0.5);
            let faces = sections.iter().map(|s| format!("{:.1}", s.line)).collect::<Vec<_>>().join(", ");
            self.events.push(Event::Log(format!(
                "{name} brings the place down: a collapse along {:.1} yd in {} sections (faces at {faces} yd), its wave {speed:.1} yd/s",
                along.last().unwrap(),
                sections.len()
            )));
            let mut col = Collapse {
                boss: mid,
                route,
                along,
                sections,
                section: 0,
                stage: Stage::Wave,
                front: 0.0,
                speed,
                lead: c.lead.max(0.0),
                hold: c.hold.clamp(0.0, 1.0),
                warn: c.warn.max(0.0),
                rest: c.rest.max(0.0),
                catch: c.catch.clamp(0.0, 1.0),
                throw: c.throw.max(0.0),
                down: c.down.max(0.0),
                end_text: c.end_text.clone(),
                held_text: c.held_text.clone(),
                held_aura: c.held_aura,
                held_kit: c.held_kit,
                end_aura: c.end_aura,
                end_kit: c.end_kit,
                end_grow: c.end_grow,
                flash_at: now,
                done: false,
                caught: BTreeSet::new(),
                catch_at: now + CATCH_TICK,
                danger_at: now,
                warn_at: now,
                dust_at: now,
                band_at: now,
                held: false,
                hurl_at: now,
                boss_leg: 1,
                holding: false,
                rowed: 0.0,
                beamed: 0.0,
                rumble_at: now,
                slam: e.slam.clone(),
                thud_at: f32::INFINITY,
                pound_by: f32::INFINITY,
            };
            self.surge_begins(&mut col);
            self.bosses.collapse = Some(col);
        }
        if !e.run_to.is_empty() {
            let now = self.clock;
            self.bosses.began.entry(mid).or_insert(home);
            self.bosses.runs.insert(
                mid,
                Run {
                    phase,
                    leg: 0,
                    falls_at: now + e.run_falls.max(0.5),
                    arrived: None,
                    went_from: home,
                    went_at: now,
                    leg_at: now,
                    leg_from: home,
                },
            );
            // Whatever it was winding up is let go: it runs.
            if let MobState::Windup { mark: Some(mk), .. } = self.mobs[&mid].state {
                self.unmark(mk);
            }
            let m = self.mobs.get_mut(&mid).unwrap();
            m.pattern = None;
            m.special = false;
            m.stagger_until = 0.0;
            if let Some(foe) = m.state.foe() {
                m.state = MobState::Chase { foe };
            }
            self.halt(mid);
        }
    }

    /// One step of a collapse: its section's surge (the warning, the wave, the face, the rest),
    /// whoever is behind the rubble caught and thrown clear, the boss fighting on through it,
    /// leashed to nowhere. A fight lost puts everything back: the boss where its fight began.
    pub(crate) fn step_boss_collapse(&mut self, dt: f32) {
        self.step_falling_looks();
        self.step_thrown();
        self.step_collapse_fled();
        self.step_boss_slams();
        let Some(mut c) = self.bosses.collapse.clone() else { return };
        let now = self.clock;
        let boss = c.boss;
        let Some(m) = self.mobs.get(&boss) else {
            self.bosses.collapse = None;
            self.collapse_cleared(boss);
            return;
        };
        // The boss down: its faces stand until the dungeon resets.
        if m.health == 0 {
            self.bosses.collapse = None;
            return;
        }
        // Its foe felled (by its own blow: it stands in that blow's recovery yet) or gone out of
        // the fight altogether (woken at a lamp elsewhere): on to whoever is still in its fight.
        let left = match m.state.foe() {
            Some(mobs::Foe::Hero(h)) => !self.players.get(&h).is_some_and(|p| p.health > 0 && !c.left(p.pos)),
            _ => false,
        };
        let next = if left { self.next_in_fight(boss) } else { None };
        // The fight lost (every hero down or gone): the collapse ends, and the boss goes back to
        // where its fight began, to be whole again there. It ends the moment the last hero is
        // down, not when the boss next looks for them: a hero who wakes at a lamp a moment later
        // must find the mine standing, not a fight still on a long way off.
        if (left && next.is_none()) || matches!(m.state, MobState::Return | MobState::Idle { .. }) {
            self.collapse_lost(boss);
            return;
        }
        if let (Some(h), MobState::Chase { .. }) = (next, &m.state) {
            let m = self.mobs.get_mut(&boss).unwrap();
            m.state = MobState::Chase { foe: mobs::Foe::Hero(h) };
            m.lost_at = None;
        }
        let Some(m) = self.mobs.get(&boss) else { return };
        if !c.done {
            // Its leash runs from wherever it is while the place comes down round it.
            if let Some(pos) = self.land.footing(m.pos) {
                self.mobs.get_mut(&boss).unwrap().home = pos;
            }
            self.collapse_stage(&mut c, dt);
        }
        if !c.done && !matches!(c.stage, Stage::Rest { .. }) {
            self.collapse_beats(&mut c);
        }
        if now >= c.thud_at {
            c.thud_at = f32::INFINITY;
            self.face_thuds(&c);
        }
        if c.pound_by.is_finite() {
            let wave = match c.stage {
                Stage::Warn { until } if now <= c.pound_by => Some(until),
                _ => None,
            };
            let free = self.mobs.get(&boss).is_some_and(|m| !matches!(m.state, MobState::Windup { .. })) && !self.disabled(boss) && !self.boss_slamming(boss);
            // Only from where the wave leaves it time to walk out after: past the section's
            // line already, or `POUND_AHEAD` and more from the end the wave starts at.
            let from = if c.section == 0 { 0.0 } else { c.sections[c.section - 1].line };
            let line = c.sections[c.section.min(c.sections.len() - 1)].line;
            let ahead = self.mobs.get(&boss).and_then(|m| route_progress(&c.route, &c.along, m.pos)).is_some_and(|(at, _)| at >= line + BOSS_CLEAR || at - from >= POUND_AHEAD);
            match (wave, c.slam.clone()) {
                (Some(wave), Some(sl)) if free && ahead => {
                    c.pound_by = f32::INFINITY;
                    self.slam_begins(boss, &sl, None, false);
                    // (Off again as the wave starts: it may be standing in the section going.)
                    if let Some(e) = self.bosses.slams.get_mut(&boss) {
                        e.until = e.until.min(wave).max(e.strike_at);
                    }
                }
                (Some(_), Some(_)) => {}
                _ => c.pound_by = f32::INFINITY,
            }
        }
        // Nobody is left behind the rubble (the last face too, for as long as the fight lasts).
        if now >= c.catch_at {
            c.catch_at = now + CATCH_TICK;
            self.collapse_catch(&mut c);
        }
        self.bosses.collapse = Some(c);
    }

    /// A section's surge begins, with its warning: whoever the last one caught can be caught
    /// again, and every other foe on the stretch runs for it.
    fn surge_begins(&mut self, c: &mut Collapse) {
        let now = self.clock;
        let (from, line) = (c.sections[c.section].from, c.sections[c.section].line);
        c.stage = Stage::Warn { until: now + c.warn };
        c.caught.clear();
        c.front = from;
        c.rowed = from - ROW_STEP + ROW_FIRST;
        c.danger_at = now;
        c.dust_at = now;
        c.band_at = now;
        c.rumble_at = now;
        c.warn_at = now + c.warn + 0.4;
        let boss = c.boss;
        let Some(m) = self.mobs.get(&boss) else { return };
        let name = self.content.mobs[m.def].name.clone();
        // It roars it down (unless a blow of its own is winding up: that keeps its look). One
        // with a slam brings it down with its body: the yell's own slam is under way for the
        // first section, and it pounds the floor where it stands for each one after (the moment
        // it is free of a blow of its own, while the pound can still land by the wave).
        let winding = matches!(m.state, MobState::Windup { .. });
        match &c.slam {
            None if !winding => self.events.push(Event::Emote { id: boss, emote: mobs::ROAR_EMOTE }),
            Some(sl) if !self.boss_slamming(boss) => c.pound_by = now + (c.warn - sl.swing).max(0.0),
            _ => {}
        }
        self.events.push(Event::Log(format!(
            "{name}'s collapse: section {} of {} goes, {from:.1} to {line:.1} yd, its wave in {:.1} s",
            c.section + 1,
            c.sections.len(),
            c.warn
        )));
        let s = c.sections[c.section].clone();
        self.line_lamps(boss, &s);
        self.collapse_foes_flee(c);
    }

    /// The surge moves on: the warning over, the wave comes from the section's back edge to its
    /// line; there its face stands at once, and the front holds until the next section goes. The
    /// last face up, the collapse is done.
    fn collapse_stage(&mut self, c: &mut Collapse, dt: f32) {
        let now = self.clock;
        let boss = c.boss;
        match c.stage {
            Stage::Warn { until } => {
                if now >= until {
                    c.stage = Stage::Wave;
                }
            }
            Stage::Wave => {
                let line = c.sections[c.section].line;
                c.front = (c.front + c.speed * dt).min(line);
                self.wave_comes_down(c);
                if c.front < line {
                    return;
                }
                let s = c.sections[c.section].clone();
                self.bosses.faces.push(Face { owner: boss, a: s.a, b: s.b, z: s.at[2], height: s.height });
                self.face_builds(boss, &s);
                c.thud_at = now + FACE_FIRST;
                // (Nobody behind it from this very step.)
                c.catch_at = now;
                let name = self.content.mobs[self.mobs[&boss].def].name.clone();
                let (k, n) = (c.section + 1, c.sections.len());
                self.events.push(Event::Log(format!("{name}'s collapse: section {k} of {n} is down, its face at {line:.1} yd")));
                let l = &self.bosses.looks;
                self.events.push(Event::Log(format!(
                    "{name}'s collapse looks: {} loose pieces lying, {} in the air, {} face rocks and {} pale over them, {} lamps",
                    l.rubble.len(),
                    l.falling.len(),
                    l.face_pieces.len(),
                    l.swaps.len(),
                    l.lamps.len()
                )));
                if k < n {
                    c.stage = Stage::Rest { until: now + c.rest };
                    return;
                }
                c.done = true;
                c.stage = Stage::Rest { until: f32::INFINITY };
                self.events.push(Event::Log(format!("{name}'s collapse reached its end")));
                self.yell(boss, c.end_text.clone());
                let pos = self.mobs[&boss].pos;
                self.mobs.get_mut(&boss).unwrap().home = pos;
                self.collapse_ends(c, &s);
            }
            Stage::Rest { until } => {
                if now >= c.dust_at {
                    c.dust_at = now + FACE_DUST_EVERY;
                    self.face_dust(c);
                }
                if now >= until {
                    c.section += 1;
                    self.surge_begins(c);
                }
            }
        }
    }

    /// A surge's beats, from its warning to its wave's end: the heroes in the section warned, a
    /// lighter fall ahead of the wave (keep moving), dust, and the band across its line.
    fn collapse_beats(&mut self, c: &mut Collapse) {
        let now = self.clock;
        let boss = c.boss;
        let k = c.section;
        if now >= c.danger_at {
            c.danger_at = now + DANGER_EVERY;
            let inside: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && c.in_section(k, p.pos, 0.0).is_some()).map(|(h, _)| *h).collect();
            for h in inside {
                self.events.push(Event::Danger { player: h, secs: DANGER_EVERY + 0.2, from: None });
            }
        }
        let line = c.sections[k].line;
        if c.stage == Stage::Wave && now >= c.warn_at && c.lead > 0.0 {
            c.warn_at = now + WARN_EVERY;
            // (Never past the line: the floor ahead of it is the way out.)
            let d = (c.front + self.rng.gen::<f32>() * c.lead).min(line - FALL_RADIUS).max(c.front);
            let at = route_point(&c.route, &c.along, d);
            let damage = self.boss_blow(boss, FALL_SHARE);
            self.rocks_fall_as(boss, at, FALL_RADIUS, FALL_WARN, damage, false, LAND_DUST);
        }
        if now >= c.dust_at {
            c.dust_at = now + DUST_EVERY;
            self.surge_dust(c);
        }
        if now >= c.band_at {
            c.band_at = now + BAND_EVERY;
            self.surge_band(c);
        }
    }

    /// Nobody is left behind the rubble: each living hero behind the wave or behind a face,
    /// however far to the side, is set down on clear floor past the newest line. The first time
    /// in a surge it costs them a share of their health and throws them down. The boss, were it
    /// ever behind its own collapse, is put ahead of it too.
    fn collapse_catch(&mut self, c: &mut Collapse) {
        let Some(limit) = c.limit() else { return };
        let boss = c.boss;
        let total = c.total();
        let line = c.sections[limit].line;
        let behind: Vec<(Id, Vec3, f32, u32)> =
            self.players.iter().filter(|(_, p)| p.health > 0 && c.behind(p.pos, 0.0)).map(|(h, p)| (*h, p.pos, p.facing, p.max_health)).collect();
        let (clear, yaw) = route_pose(&c.route, &c.along, (line + c.throw).min(total));
        let clear = self.land.footing(clear).unwrap_or(clear);
        let now = self.clock;
        for (h, was, facing, max) in behind {
            // (Thrown and in the air: on their way clear.)
            if self.bosses.thrown.get(&h).is_some_and(|t| now < t.until) {
                continue;
            }
            // (Side by side, not on a spot somebody stands on.)
            let to = [0.0, 1.4, -1.4, 2.8, -2.8]
                .iter()
                .filter_map(|off| self.land.footing([clear[0] - yaw.sin() * off, clear[1] + yaw.cos() * off, clear[2]]))
                .find(|p| !c.behind(*p, 0.0) && !self.players.iter().any(|(o, q)| *o != h && q.health > 0 && dist2d(q.pos, *p) < 1.0))
                .unwrap_or(clear);
            if !c.caught.insert(h) {
                self.set_down(h, to, facing);
                continue;
            }
            let damage = if c.catch > 0.0 { ((max as f32) * c.catch).round().max(1.0) as u32 } else { 0 };
            let name = self.players[&h].name.clone();
            if damage > 0 {
                self.hurt_player_by(h, damage, boss, "the falling mine");
            }
            let alive = self.players.get(&h).is_some_and(|p| p.health > 0);
            let share = self.content.cc.player_share.max(0.05);
            let down = if c.down > 0.0 { c.down / share } else { 0.0 };
            // Thrown clear through the air where the floor between is open; set down there
            // where it isn't (and a hero the blow felled: their body is laid clear).
            // The flight is over before the face has rock in it: no longer than the wave's
            // way to the line and the face's first moment (and none once the wave is there:
            // the face is building, or stands).
            let most = if c.stage == Stage::Wave { (line - c.front).max(0.0) / c.speed.max(0.1) + FACE_FIRST } else { 0.0 };
            let flight = if alive && most >= THROW_AIR.0 { self.throw_clear(was, to, most) } else { None };
            self.events.push(Event::Log(format!(
                "the collapse catches {name} at ({:.1}, {:.1}, {:.1}): {damage} of {max} health gone, {} clear to ({:.1}, {:.1}, {:.1})",
                was[0],
                was[1],
                was[2],
                match flight {
                    Some((_, _, _, air)) => format!("thrown {:.1} yd through {air:.2} s", dist2d(was, to)),
                    None => "set down".into(),
                },
                to[0],
                to[1],
                to[2]
            )));
            match flight {
                Some((dir, speed, lift, air)) => {
                    self.interrupt_hero(h);
                    self.set_action(h, Action::Stagger { until: now + air });
                    self.events.push(Event::Hop { player: h, dir, speed, lift, roll: false, style: "" });
                    self.bosses.thrown.insert(h, Thrown { boss, lands: now + air, until: now + air + THROW_GRACE, down, landed: false });
                }
                None => {
                    self.set_down(h, to, facing);
                    if alive && down > 0.0 {
                        self.knock_down(boss, h, down, "the falling mine");
                    }
                }
            }
        }
        if let Some(pos) = self.mobs.get(&boss).map(|m| m.pos) {
            if c.behind(pos, BOSS_SLACK) {
                let at = route_point(&c.route, &c.along, (line + BOSS_CLEAR).min(total));
                self.set_back(boss, at, "behind its own collapse");
            }
        }
    }

    /// The flight that throws a caught hero from `was` clear to `to`: its heading, speed, lift
    /// and time in the air. None where the floor between isn't open all the way (a wall, a drop,
    /// a climb), or it's no distance at all: there they are set down.
    fn throw_clear(&self, was: Vec3, to: Vec3, most: f32) -> Option<(f32, f32, f32, f32)> {
        let dist = dist2d(was, to);
        if dist < THROW_LEAST {
            return None;
        }
        let dir = facing(was, to);
        let end = self.slide_end(was, dir, dist);
        if dist2d(end, to) > THROW_MISS {
            return None;
        }
        let air = (dist / THROW_SPEED).clamp(THROW_AIR.0, THROW_AIR.1.min(most).max(THROW_AIR.0));
        // Up enough to come down on the landing's own floor after `air` seconds, the floor
        // between rising or falling evenly.
        let lift = ((to[2] - was[2] + 0.5 * super::moves::GRAVITY * air * air) / air).max(THROW_LIFT);
        Some((dir, dist / air, lift, air))
    }

    /// The thrown land: thrown down where they come to the floor; and a moment after, the catch
    /// has them again (still behind the rubble then, they are set down clear).
    fn step_thrown(&mut self) {
        if self.bosses.thrown.is_empty() {
            return;
        }
        let now = self.clock;
        let due: Vec<(Id, Thrown)> = self.bosses.thrown.iter().filter(|(_, t)| !t.landed && now >= t.lands).map(|(h, t)| (*h, *t)).collect();
        for (h, t) in due {
            if let Some(e) = self.bosses.thrown.get_mut(&h) {
                e.landed = true;
            }
            if t.down > 0.0 && self.players.get(&h).is_some_and(|p| p.health > 0) {
                self.knock_down(t.boss, h, t.down, "the falling mine");
            }
        }
        self.bosses.thrown.retain(|_, t| now < t.until);
    }

    /// A surge begins: every other foe on the stretch (no boss, nothing of the heroes') lets go
    /// of its fight and runs for the route's end, to be gone a moment later. The collapse takes
    /// no pack into the boss's fight.
    fn collapse_foes_flee(&mut self, c: &Collapse) {
        let now = self.clock;
        let end = *c.route.last().unwrap();
        let foes: Vec<(Id, usize)> = self
            .mobs
            .iter()
            .filter(|(id, m)| **id != c.boss && m.health > 0 && !m.friendly)
            .filter(|(_, m)| {
                let d = &self.content.mobs[m.def];
                !d.boss && !d.inert()
            })
            .filter(|(id, m)| !self.bosses.fleeing.iter().any(|(f, _)| f == *id) && c.on_stretch(m.pos))
            .map(|(id, m)| (*id, m.def))
            .collect();
        for (id, def) in foes {
            self.collapse_lets_go(id);
            let (name, speed) = (self.content.mobs[def].name.clone(), self.content.mobs[def].run_speed);
            self.go(id, end, speed);
            self.bosses.fleeing.push((id, now + FLEE_SECS));
            self.events.push(Event::Log(format!("{name} runs from the collapse")));
        }
    }

    /// A foe running from a collapse heeds nothing else (in place of its thinking).
    pub(crate) fn step_collapse_flee(&self, id: Id) -> bool {
        self.bosses.fleeing.iter().any(|(f, _)| *f == id)
    }

    /// The foes that ran from a collapse are gone once their time is up.
    fn step_collapse_fled(&mut self) {
        if self.bosses.fleeing.is_empty() {
            return;
        }
        let now = self.clock;
        let due: Vec<Id> = self.bosses.fleeing.iter().filter(|(_, t)| now >= *t).map(|(id, _)| *id).collect();
        self.bosses.fleeing.retain(|(_, t)| now < *t);
        for id in due {
            if self.mobs.remove(&id).is_some() {
                self.events.push(Event::MobGone { id });
            }
        }
    }

    /// Whatever a creature was winding up is let go: it has somewhere to be.
    fn collapse_lets_go(&mut self, id: Id) {
        let Some(m) = self.mobs.get(&id) else { return };
        if let MobState::Windup { mark: Some(mk), .. } = m.state {
            self.unmark(mk);
        }
        let m = self.mobs.get_mut(&id).unwrap();
        if let Some(foe) = m.state.foe() {
            m.pattern = None;
            m.special = false;
            m.state = MobState::Chase { foe };
        }
    }

    /// A face of rubble in a hero's way from `from` to `to` (a walk, a roll, a jump, a throw):
    /// where they stop, a step short of it, at `from`'s height. Either way: nobody passes one.
    /// A hero standing on its very line steps off it freely.
    pub fn face_crossing(&self, from: Vec3, to: Vec3) -> Option<Vec3> {
        let mut first: Option<f32> = None;
        for f in &self.bosses.faces {
            if from[2] < f.z - FACE_BELOW || from[2] > f.z + f.height + FACE_ABOVE {
                continue;
            }
            let u = [f.b[0] - f.a[0], f.b[1] - f.a[1]];
            let len = (u[0] * u[0] + u[1] * u[1]).sqrt();
            if len < 1.0e-3 {
                continue;
            }
            let side = |p: Vec3| (u[0] * (p[1] - f.a[1]) - u[1] * (p[0] - f.a[0])) / len;
            let (sa, sb) = (side(from), side(to));
            if sa == 0.0 || sa * sb > 0.0 {
                continue;
            }
            let t = sa / (sa - sb);
            let hit = [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t];
            // (Into the walls a little past each end.)
            let w = ((hit[0] - f.a[0]) * u[0] + (hit[1] - f.a[1]) * u[1]) / len;
            if w < -FACE_PAD || w > len + FACE_PAD {
                continue;
            }
            if first.is_none_or(|b| t < b) {
                first = Some(t);
            }
        }
        let t = first?;
        let way = dist2d(from, to).max(1.0e-4);
        let k = ((t * way - FACE_STOP) / way).clamp(0.0, 1.0);
        Some([from[0] + (to[0] - from[0]) * k, from[1] + (to[1] - from[1]) * k, from[2]])
    }

    /// A creature's way with faces of rubble standing: cut off a stride short of the first face
    /// it would go through (circling its foe, backing off to make space, a way found round a
    /// pillar). False when that leaves it nowhere to go: it stays where it is.
    pub(crate) fn short_of_faces(&self, from: Vec3, path: &mut Vec<Vec3>) -> bool {
        if self.bosses.faces.is_empty() {
            return true;
        }
        let mut at = from;
        for i in 0..path.len() {
            if let Some(stop) = self.face_crossing(at, path[i]) {
                let d = dist2d(at, stop);
                path.truncate(i);
                let back = FACE_SHY - FACE_STOP;
                if d > back + 0.3 {
                    let k = (d - back) / d;
                    let p = [at[0] + (stop[0] - at[0]) * k, at[1] + (stop[1] - at[1]) * k, at[2]];
                    path.push(self.land.footing(p).unwrap_or(p));
                }
                break;
            }
            at = path[i];
        }
        !path.is_empty()
    }

    /// A face of rubble in the way of a hero's own move (a walk, a roll, a jump as their client
    /// moved them): stopped a step short of it, at once, on the floor there if that's above them
    /// (by no more than `FACE_FLOOR`: another floor is not theirs). (A move too long to be one
    /// of those is left to the catch.)
    pub(crate) fn hold_at_face(&mut self, id: Id, from: Vec3, to: Vec3) -> bool {
        if self.bosses.faces.is_empty() || dist2d(from, to) > 12.0 {
            return false;
        }
        // (Thrown clear by the catch: its flight is over the line, not stopped at it.)
        if self.bosses.thrown.contains_key(&id) {
            return false;
        }
        let Some(mut back) = self.face_crossing(from, to) else { return false };
        // Never under the floor. The stop is at the height they came from, and the way to a face
        // may climb (the first one's does, 0.4 yd a yard): a step on at that height is under the
        // floor there, and their client falls through the world (filmed: z 46.6 under a floor
        // at 57.3). A hero in the air keeps their height.
        let floor = self.land.height_near(back[0], back[1], back[2]);
        if floor.is_finite() && floor > back[2] && floor - back[2] <= FACE_FLOOR {
            back[2] = floor;
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.pos = back;
            let facing = p.facing;
            self.events.push(Event::Teleport { player: id, pos: back, facing });
        }
        true
    }

    /// Everything a boss's collapse left is gone: its rock, and its faces with what they stop.
    fn collapse_cleared(&mut self, owner: Id) {
        self.clear_looks(owner);
        self.bosses.faces.retain(|f| f.owner != owner);
    }

    /// A collapse's fight is lost: the mine stands again for the next try (what came down is
    /// gone, its faces and their stops with it), and the boss is back where its fight began, on
    /// its way to being whole there (home, a creature heals and its phases start over).
    fn collapse_lost(&mut self, boss: Id) {
        // What its last stand put on it comes off: its own size again, no look of the fight.
        let grown = self.bosses.collapse.as_ref().is_some_and(|c| c.boss == boss && c.done && c.end_grow > 0.0);
        self.bosses.collapse = None;
        self.bosses.slams.remove(&boss);
        if let Some(def) = self.mobs.get(&boss).map(|m| m.def) {
            if grown {
                self.events.push(Event::Grow { id: boss, scale: self.content.mobs[def].scale });
            }
            self.events.push(Event::Auras { id: boss });
        }
        // The faces' stops go at once, with every rule of it. What it looked like goes a moment
        // after, in its dust (well before anyone wakes at a lamp).
        self.bosses.faces.retain(|f| f.owner != boss);
        self.bosses.thrown.clear();
        self.looks_leave(boss);
        let Some(at) = self.bosses.began.get(&boss).copied() else { return };
        self.collapse_lets_go(boss);
        let Some(m) = self.mobs.get_mut(&boss) else { return };
        m.home = at;
        if !matches!(m.state, MobState::Idle { .. }) {
            m.state = MobState::Return;
        }
        self.set_back(boss, at, "the collapse lost: back where the fight began");
    }

    /// Every hero below is down and the foes stir: a collapse still standing is lost with them.
    pub(crate) fn collapse_wiped(&mut self) {
        let Some(boss) = self.bosses.collapse.as_ref().map(|c| c.boss) else { return };
        if self.mobs.get(&boss).is_some_and(|m| m.health > 0) {
            self.collapse_lost(boss);
        }
    }

    /// The dungeon resets: no collapse, no face, no rubble, nobody running from one.
    pub(crate) fn collapse_reset(&mut self) {
        let off = self.bosses.looks.off;
        self.bosses.collapse = None;
        self.bosses.faces.clear();
        self.bosses.fleeing.clear();
        self.bosses.thrown.clear();
        self.bosses.slams.clear();
        self.bosses.looks = Looks { off, ..Default::default() };
    }

    /// The end of a collapse is a turn anyone can see, with no line read: the last face throws
    /// its dust out across the floor with a crash, and the boss turns on the heroes: it roars,
    /// what held it breaks off it (`end_kit`), and from then until it falls it wears `end_aura`
    /// and stands `end_grow` times its size.
    fn collapse_ends(&mut self, c: &Collapse, last: &Section) {
        let boss = c.boss;
        if !self.bosses.looks.off {
            let u = [last.b[0] - last.a[0], last.b[1] - last.a[1]];
            let wide = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1.0e-3);
            let ahead = [-u[1] / wide * last.ahead, u[0] / wide * last.ahead];
            let n = (wide / END_DUST_GAP).ceil().max(1.0) as usize;
            // Two rows of it, at the face's foot and rolling out from it, low and at head height.
            for row in [1.0_f32, 3.5] {
                for k in 0..=n {
                    let t = k as f32 / n as f32;
                    let xy = [last.a[0] + u[0] * t + ahead[0] * row, last.a[1] + u[1] * t + ahead[1] * row];
                    let at = self.floor_beside(last.at, xy);
                    self.point_look(boss, at, LAND_DUST);
                    self.point_look(boss, [at[0], at[1], at[2] + last.height * 0.5], LAND_DUST);
                }
            }
            self.events.push(Event::GroundBurst {
                at: last.at,
                yaw: ahead[1].atan2(ahead[0]),
                shape: "line",
                reach: 9.0,
                arc: 0.0,
                width: wide,
                secs: 1.2,
                colour: DUST_COLOUR.into(),
            });
            let near: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, last.at) <= 40.0).map(|(h, _)| *h).collect();
            for h in near {
                self.events.push(Event::Beat { player: h, beat: BEAT_ROCKFALL.into(), on: None });
            }
        }
        self.events.push(Event::Sound { id: boss, sound: SOUND_CRASH });
        self.events.push(Event::Emote { id: boss, emote: mobs::ROAR_EMOTE });
        if c.end_kit != 0 {
            self.events.push(Event::Visual { id: boss, kit: c.end_kit, by: None });
        }
        if c.end_grow > 0.0 {
            let own = self.content.mobs[self.mobs[&boss].def].scale;
            self.events.push(Event::Grow { id: boss, scale: own * c.end_grow });
        }
        // (The collapse is put back with `done` set after this step: its looks are read from it
        // when this event is played, after the step.)
        self.events.push(Event::Auras { id: boss });
    }

    /// What shows on a boss for where its fight stands (spells whose state the clients play):
    /// held by its collapse, in its last stand, its weapon burning.
    pub(crate) fn boss_looks(&self, id: Id) -> Vec<u32> {
        let mut out = Vec::new();
        if let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == id) {
            if c.done {
                out.push(c.end_aura);
            } else if c.held {
                out.push(c.held_aura);
            }
        }
        if let Some(b) = self.bosses.burning.get(&id) {
            out.push(b.aura);
        }
        // Its weapon lodged, the fire goes out of it: the look of its last stand and of its
        // burning weapon are off it for as long as it hauls, and back as the weapon comes free.
        // Filmed on Rhahk'Zor, November 13: the stun pose alone, the maul still in his hand,
        // did not say "now", and none of 9 stuns' stars showed on an ogre. A hold stays on:
        // a lodged boss that can't be brought lower must not look open.
        if self.bosses.lodged.get(&id).is_some_and(|until| self.clock < *until) {
            let held = self.bosses.collapse.as_ref().filter(|c| c.boss == id && !c.done && c.held).map(|c| c.held_aura);
            out.retain(|a| Some(*a) == held);
        }
        out.retain(|a| *a != 0);
        out
    }

    /// A blow on a boss whose collapse is still coming down: it can't be brought below its hold
    /// before the last section is down (the last of the fight is fought at its face). The first
    /// blow refused is said (`held_text`) and shown: `held_aura` on it from then until the hold
    /// lifts, and `held_kit` flashed on it for each blow refused.
    pub(crate) fn boss_hold(&mut self, mid: Id, by: Id, damage: u32) -> u32 {
        let now = self.clock;
        let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == mid && !c.done && c.hold > 0.0) else { return damage };
        let Some(m) = self.mobs.get(&mid) else { return damage };
        let floor = (c.hold * m.max_health as f32).round() as u32;
        let most = m.health.saturating_sub(floor);
        if damage <= most {
            return damage;
        }
        let (first, kit, flash) = (!c.held, c.held_kit, now >= c.flash_at);
        if first {
            let line = c.held_text.clone();
            self.bosses.collapse.as_mut().unwrap().held = true;
            self.yell(mid, line);
            self.events.push(Event::Auras { id: mid });
        }
        if kit != 0 && flash {
            self.bosses.collapse.as_mut().unwrap().flash_at = now + HELD_FLASH_GAP;
            self.events.push(Event::Visual { id: mid, kit, by: None });
        }
        // The blow's own number says what it took, and a blow that took nothing glanced off
        // (filmed, November 13: a hero at the hold saw 29 and 21 float off a health bar that
        // never moved). Whoever strikes pushes the blow's `Swing` just before it hurts, so the
        // number is the last `Swing` from `by` on this boss, and only if that is a hit of this
        // size: a hurt with no `Swing` of its own (a bleed, a burn) rewrites nobody's blow.
        if let Some(Event::Swing { damage: d, outcome, .. }) =
            self.events.iter_mut().rev().find(|e| matches!(e, Event::Swing { attacker, victim: Some(v), .. } if *attacker == by && *v == mid))
        {
            if *outcome == Outcome::Hit && *d == damage {
                *d = most;
                if most == 0 {
                    *outcome = Outcome::Blocked;
                }
            }
        }
        most
    }

    /// A boss whose collapse is coming down, in place of its thinking:
    /// - **In a wave** it walks the route out of the section going, to a little past its line
    ///   (faster than the wave comes), and fights on from there.
    /// - **Its hero run off ahead of the fight** (far past the line, or off the route), it holds
    ///   the line a little past the rubble and hurls rock down the tunnel after them. The
    ///   collapse and the boss set where the fight is, never the heroes.
    ///
    /// False when neither holds (it fights as ever, at the rubble's face) or for any other
    /// creature.
    pub(crate) fn step_boss_holdline(&mut self, id: Id) -> bool {
        let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == id && !c.done) else { return false };
        let (lead, total) = (c.lead, c.total());
        let line = c.sections[c.section].line;
        let wave = c.stage == Stage::Wave;
        let wave_speed = c.speed;
        let (mut leg, holding) = (c.boss_leg, c.holding);
        let Some(m) = self.mobs.get(&id) else { return false };
        let now = self.clock;
        let (route, along) = (c.route.clone(), c.along.clone());
        // Where it is on the route. While it walks the route its next corner is kept in order
        // (through a hairpin a nearest point alone can go backwards); from its own fight, or off
        // the route, it's found afresh.
        let Some((at, off)) = route_progress(&route, &along, m.pos) else { return false };
        if !holding || off > 1.0 {
            leg = along.iter().position(|a| *a > at + 0.01).unwrap_or(route.len() - 1).max(1);
        }
        // (It walks several corners in a step: the leg it has come to, of those just ahead.)
        let (first, mut near) = (leg, f32::MAX);
        for j in first..route.len() {
            if along[j - 1] > along[first] + LEG_LOOK {
                break;
            }
            let off = route_progress(&route[j - 1..=j], &along[j - 1..=j], m.pos).map_or(f32::MAX, |p| p.1);
            if off < near - 1.0e-3 {
                (leg, near) = (j, off);
            }
        }
        while leg + 1 < route.len() && dist3(m.pos, route[leg]) < 0.08 {
            leg += 1;
        }
        let (boss_at, boss_off) = route_progress(&route[leg - 1..=leg], &along[leg - 1..=leg], m.pos).unwrap();
        let hero = match m.state.foe() {
            Some(mobs::Foe::Hero(h)) => self.players.get(&h).filter(|p| p.health > 0 && route_progress(&route, &along, p.pos).is_some_and(|(_, off)| off <= FIGHT_LEFT)).map(|p| p.pos),
            _ => None,
        };
        let lost = m.lost_at.is_some_and(|t| now - t >= STUCK);
        let ran = hero.is_some_and(|hero| {
            route_progress(&route, &along, hero).is_some_and(|(at, off)| at > (line + lead + LINE_SLACK).min(total) || off > OFF_ROUTE || lost)
        });
        let out = wave && boss_at < line + BOSS_CLEAR;
        if !ran && !out {
            self.bosses.collapse.as_mut().unwrap().holding = false;
            return false;
        }
        let def = m.def;
        // Walking out of a wave, whatever it was winding up is let go.
        if out {
            self.collapse_lets_go(id);
        }
        // Follow the route corner by corner, never the chord from the creature to a distant spot.
        // A hero on another floor doesn't pull it out of the route either.
        let want = (line + if ran { (lead * 0.5).max(BOSS_CLEAR) } else { BOSS_CLEAR }).min(total);
        let on_route = boss_off <= 1.0;
        let to = if on_route {
            route_point(&route, &along, want.max(boss_at))
        } else {
            // (Off the route: back onto it, on ahead if the wave is coming.)
            route_point(&route, &along, if out { (boss_at + boss_off).min(want.max(boss_at)) } else { boss_at })
        };
        let c = self.bosses.collapse.as_mut().unwrap();
        c.boss_leg = leg;
        c.holding = true;
        if let Some(to) = self.land.footing(to).filter(|_| boss_at < want || !on_route) {
            let run = self.content.mobs[def].run_speed;
            let end = self.mobs[&id].path.last().copied();
            if end.is_none_or(|end| dist3(end, to) > 0.5) {
                if on_route {
                    // (All its corners at once: a corner a step would lose the race with the wave.)
                    let mut path: Vec<Vec3> = (leg..route.len()).take_while(|j| along[*j] < want - 0.05).map(|j| route[j]).collect();
                    path.push(to);
                    let speed = if out { run.max(wave_speed * BOSS_OUTRUN) } else { run * self.speed_share(id) };
                    self.go_along(id, path, speed);
                } else {
                    self.go(id, to, run);
                }
            }
        } else if !self.mobs[&id].path.is_empty() {
            self.halt(id);
        }
        // Rock hurled down the tunnel at the hero who ran.
        if let (true, Some(hero)) = (ran, hero) {
            let c = self.bosses.collapse.as_mut().unwrap();
            if now >= c.hurl_at {
                c.hurl_at = now + HURL_EVERY;
                let damage = self.boss_blow(id, DEBRIS_SHARE);
                // (One with a slam swings for it; any other roars.)
                match self.bosses.collapse.as_ref().and_then(|c| c.slam.clone()) {
                    Some(sl) => self.events.push(Event::Clip { id, clip: sl.clip, take: sl.take, from: None, secs: sl.swing + sl.recover, at: sl.swing, body: ClipBody::Full }),
                    None => self.events.push(Event::Emote { id, emote: mobs::ROAR_EMOTE }),
                }
                self.rocks_fall(id, hero, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
            }
        }
        true
    }

    /// Whether a boss's collapse is still coming down: it chases its heroes however far they run.
    pub(crate) fn boss_unleashed(&self, id: Id) -> bool {
        self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id && !c.done)
    }

    pub(crate) fn boss_fight_home(&self, id: Id) -> Option<Vec3> {
        self.bosses.began.get(&id).copied()
    }

    /// The collapse's front, as far along its route as its rubble has come (the wave in a surge,
    /// the newest face between), and whether its last section is down (for tests and the log).
    pub fn collapse_front(&self) -> Option<(f32, bool)> {
        self.bosses.collapse.as_ref().map(|c| (c.front, c.done))
    }

    /// The collapse's section going (or the last one down, counted from 0), where its surge
    /// stands (`warn`, `wave` or `rest`), its front, and whether it's done (for tests and the log).
    pub fn collapse_state(&self) -> Option<(usize, &'static str, f32, bool)> {
        self.bosses.collapse.as_ref().map(|c| {
            let stage = match c.stage {
                Stage::Warn { .. } => "warn",
                Stage::Wave => "wave",
                Stage::Rest { .. } => "rest",
            };
            (c.section, stage, c.front, c.done)
        })
    }

    /// Where along its route each of the collapse's faces stands, and the route's length (for
    /// tests and the log).
    pub fn collapse_lines(&self) -> Option<(Vec<f32>, f32)> {
        self.bosses.collapse.as_ref().map(|c| (c.sections.iter().map(|s| s.line).collect(), c.total()))
    }

    /// The point this far along the collapse's route (for tests).
    pub fn collapse_route_point(&self, d: f32) -> Option<Vec3> {
        self.bosses.collapse.as_ref().map(|c| route_point(&c.route, &c.along, d))
    }

    /// Whether a spot is behind the collapse's rubble: where a hero is caught (for tests).
    pub fn collapse_behind(&self, pos: Vec3) -> bool {
        self.bosses.collapse.as_ref().is_some_and(|c| c.behind(pos, 0.0))
    }

    /// The faces of rubble standing, each from wall to wall (the rule; for tests and the log).
    pub fn collapse_face_lines(&self) -> Vec<([f32; 2], [f32; 2])> {
        self.bosses.faces.iter().map(|f| (f.a, f.b)).collect()
    }

    /// Whether a boss is on its run (its phases wait until it has taken up the fight again).
    pub(crate) fn boss_running(&self, mid: Id) -> bool {
        self.bosses.runs.contains_key(&mid)
    }

    /// One step of a boss on its run, in place of its thinking: along its route, rocks behind it,
    /// then waiting at the end for a hero. False once it fights again (or for any other creature).
    pub(crate) fn step_boss_run(&mut self, id: Id) -> bool {
        let Some(run) = self.bosses.runs.get(&id).cloned() else { return false };
        let Some(m) = self.mobs.get(&id) else {
            self.bosses.runs.remove(&id);
            return false;
        };
        // Its fight lost on the way (every hero down or gone): the run ends, and it heads home
        // to be whole again where its fight began.
        if matches!(m.state, MobState::Return | MobState::Idle { .. }) {
            self.bosses.runs.remove(&id);
            return false;
        }
        let now = self.clock;
        let def = m.def;
        let Some(e) = self.content.mobs[def].phases.get(run.phase).cloned() else {
            self.bosses.runs.remove(&id);
            return false;
        };
        let pos = m.pos;
        let last = *e.run_to.last().unwrap();
        if run.arrived.is_none() {
            let to = e.run_to[run.leg.min(e.run_to.len() - 1)];
            // Headway: where it last moved a yard, and when.
            if dist2d(pos, run.went_from) >= 1.0 {
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.went_from = pos;
                r.went_at = now;
            }
            if dist2d(pos, to) < 1.5 && (pos[2] - to[2]).abs() < 4.0 {
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.leg += 1;
                let (leg, of) = (r.leg, e.run_to.len());
                let name = self.content.mobs[def].name.clone();
                self.events.push(Event::Log(format!("{name} on its run: point {leg} of {of} at {:.0} {:.0} {:.0}, {:.1} s in", pos[0], pos[1], pos[2], now - run.leg_at)));
                if leg >= of {
                    self.bosses.runs.get_mut(&id).unwrap().arrived = Some(now);
                    self.halt(id);
                    return true;
                }
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.leg_at = now;
                r.leg_from = pos;
                let next = e.run_to[leg];
                let speed = self.content.mobs[def].run_speed * RUN_HASTE;
                self.go(id, next, speed);
            } else if self.mobs[&id].lost_at.is_some_and(|t| now - t >= STUCK)
                || (now - run.went_at >= STUCK && dist2d(pos, run.went_from) < 1.0)
                || now - run.leg_at > dist2d(run.leg_from, to) / (self.content.mobs[def].run_speed * RUN_HASTE).max(1.0) * LEG_SLACK + STUCK
            {
                // No way along this leg, no headway on it, or far longer on it than a run along
                // it takes: set down at its end rather than standing in the wall or working along
                // a ledge (the log says where, to be looked into).
                self.set_back(id, to, "boss run, no headway on the leg");
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.went_from = to;
                r.went_at = now;
                r.leg_at = now;
                r.leg_from = to;
            } else if self.mobs[&id].path.is_empty() {
                let speed = self.content.mobs[def].run_speed * RUN_HASTE;
                self.go(id, to, speed);
            }
            // Rocks shaken loose where it has just been.
            if e.run_falls > 0.0 && now >= run.falls_at {
                self.bosses.runs.get_mut(&id).unwrap().falls_at = now + e.run_falls;
                let damage = self.boss_blow(id, FALL_SHARE);
                self.rocks_fall(id, pos, FALL_RADIUS, FALL_WARN, damage, false);
            }
            return true;
        }
        // At the route's end: it waits for a hero to come (not a lone one ambushed the moment they
        // step in, nor the party left to find it standing in a doorway forever).
        let near = if e.wait_for > 0.0 { e.wait_for } else { WAIT_FOR };
        let come = self
            .players
            .iter()
            .filter(|(_, p)| p.health > 0 && dist2d(p.pos, last) <= near && (p.pos[2] - last[2]).abs() < 8.0)
            .min_by(|a, b| dist2d(a.1.pos, last).total_cmp(&dist2d(b.1.pos, last)))
            .map(|(h, _)| *h);
        let waited = run.arrived.is_some_and(|t| now - t >= WAIT_MOST);
        if come.is_none() && !waited {
            return true;
        }
        self.bosses.runs.remove(&id);
        let m = self.mobs.get_mut(&id).unwrap();
        m.home = last;
        m.think = 0.0;
        if let Some(h) = come {
            m.state = MobState::Chase { foe: mobs::Foe::Hero(h) };
        }
        if e.arrive_kit != 0 {
            self.events.push(Event::Visual { id, kit: e.arrive_kit, by: None });
        }
        self.yell(id, e.arrive_text.clone());
        false
    }

    /// A boss whose fight was lost is home again (`MobState::Return`): one that ran is put back
    /// where its fight began, whole, and its blows burn no more.
    pub(crate) fn boss_home(&mut self, id: Id) {
        self.bosses.runs.remove(&id);
        let lodged = self.bosses.lodged.remove(&id).is_some();
        if self.bosses.burning.remove(&id).is_some() || lodged {
            self.events.push(Event::Auras { id });
        }
        self.fires_out(id);
        // (A lost collapse's rock is on its way out in its dust: `step_looks_leaving` takes it.)
        if self.bosses.looks.leaving.is_some_and(|l| l.0 == id) {
            self.bosses.faces.retain(|f| f.owner != id);
        } else {
            self.collapse_cleared(id);
        }
        if let Some(at) = self.bosses.began.remove(&id) {
            if let Some(m) = self.mobs.get_mut(&id) {
                m.home = at;
            }
            self.set_back(id, at, "boss put back where its fight began");
        }
    }

    /// A boss is down: its beats end, and the ground it set burning goes out with it (filmed,
    /// November 13: the floor burned on for 7 sec round his body and his drops). What its
    /// collapse brought down stays, its faces too, until the dungeon resets.
    pub(crate) fn boss_gone(&mut self, id: Id) {
        self.bosses.runs.remove(&id);
        self.bosses.burning.remove(&id);
        self.bosses.lodged.remove(&id);
        self.bosses.began.remove(&id);
        if self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id) {
            self.bosses.collapse = None;
        }
        self.fires_out(id);
    }

    /// The ground a boss set burning is out: its flames gone, nothing more bitten.
    fn fires_out(&mut self, owner: Id) {
        let gone: Vec<Id> = self.bosses.fires.iter().filter(|f| f.owner == owner).flat_map(|f| f.objs.clone()).collect();
        self.bosses.fires.retain(|f| f.owner != owner);
        for obj in gone {
            self.objs.remove(&obj);
            self.events.push(Event::ObjGone { id: obj });
        }
    }

    /// A boss's melee blow leaves burning ground where a heated weapon lands.
    pub(crate) fn boss_blow_lands(&mut self, id: Id, pos: Vec3, facing: f32, range: f32, arc: f32, heavy: bool) {
        let now = self.clock;
        if heavy {
            if let Some(b) = self.bosses.burning.get(&id).cloned() {
                let at = if arc >= 300.0 { pos } else { [pos[0] + facing.cos() * range * 0.7, pos[1] + facing.sin() * range * 0.7, pos[2]] };
                let at = self.ground_near(at[0], at[1], at[2]);
                if b.look != 0 && b.radius >= GROUND_LOOK_MIN {
                    self.events.push(Event::Missile { caster: id, spell: b.look, target: Target::Point(at) });
                }
                // Flames across the patch: one at its heart, the rest round it.
                let turn = (self.look_seq() % 6) as f32;
                let mut objs = Vec::new();
                for k in 0..=FIRE_FLAMES {
                    let p = if k == 0 {
                        at
                    } else {
                        let a = turn + k as f32 * std::f32::consts::TAU / FIRE_FLAMES as f32;
                        let g = self.ground_near(at[0] + a.cos() * b.radius * FIRE_FLAMES_AT, at[1] + a.sin() * b.radius * FIRE_FLAMES_AT, at[2]);
                        if !g[2].is_finite() || (g[2] - at[2]).abs() > 1.5 {
                            continue;
                        }
                        g
                    };
                    let obj = self.alloc();
                    self.objs.insert(obj, Obj { kind: ObjKind::Fire { on: 0 }, pos: p, facing: 0.0 });
                    self.events.push(Event::ObjSpawned { id: obj });
                    objs.push(obj);
                }
                self.bosses.fires.push(Fire { at, radius: b.radius, damage: b.damage, until: now + b.secs, bite_at: now + BURN_TICK * 0.5, owner: id, objs, ring_at: now });
            }
        }
    }

    /// The released step's weapon lodges after its ordinary recovery has been recorded.
    pub(crate) fn boss_weapon_lodges(&mut self, id: Id, secs: f32) {
        let recovery = match self.mobs.get(&id).map(|m| &m.state) {
            Some(MobState::Recover { until, .. }) => *until,
            _ => self.clock,
        };
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.stagger_mob(id, secs, false);
        // Seen for as long as it lasts (`boss_looks`).
        let until = self.bosses.lodged.get(&id).copied().unwrap_or(0.0).max(self.clock + secs);
        self.bosses.lodged.insert(id, until);
        self.events.push(Event::Auras { id });
        // A short lodge (the ordinary slam) must not shorten a longer normal recovery either.
        if let Some(Mob { state: MobState::Recover { until, .. }, .. }) = self.mobs.get_mut(&id) {
            *until = until.max(recovery);
        }
    }

    /// A lodged weapon comes free: the look of it goes.
    pub(crate) fn step_boss_lodges(&mut self) {
        if self.bosses.lodged.is_empty() {
            return;
        }
        let now = self.clock;
        let free: Vec<Id> = self.bosses.lodged.iter().filter(|(_, until)| now >= **until).map(|(id, _)| *id).collect();
        for id in free {
            self.bosses.lodged.remove(&id);
            self.events.push(Event::Auras { id });
        }
    }

    /// Burning ground: it bites whoever stands in it, then burns out.
    pub(crate) fn step_boss_fires(&mut self) {
        if self.bosses.fires.is_empty() {
            return;
        }
        let now = self.clock;
        let mut out = Vec::new();
        let mut bites = Vec::new();
        let mut rings = Vec::new();
        for f in self.bosses.fires.iter_mut() {
            if now >= f.until {
                out.extend(f.objs.iter().copied());
                continue;
            }
            if now >= f.ring_at {
                f.ring_at = now + FIRE_REDRAW;
                rings.push((f.at, f.radius, (FIRE_REDRAW + 0.05).min(f.until - now)));
            }
            if now >= f.bite_at {
                f.bite_at = now + BURN_TICK;
                bites.push((f.at, f.radius, f.damage, f.owner));
            }
        }
        // Its edge, where it bites and no wider.
        for (at, reach, secs) in rings {
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach, arc: 360.0, width: 0.0, secs, colour: FIRE_COLOUR.into() });
        }
        self.bosses.fires.retain(|f| now < f.until);
        for obj in out {
            self.objs.remove(&obj);
            self.events.push(Event::ObjGone { id: obj });
        }
        for (at, radius, damage, owner) in bites {
            let caught: Vec<Id> = self
                .players
                .iter()
                .filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= radius && (p.pos[2] - at[2]).abs() < 3.0)
                .map(|(h, _)| *h)
                .collect();
            for h in caught {
                self.hurt_player_by(h, damage, owner, "burning iron");
            }
        }
    }

    /// (Debug) The way below opened without its bosses beaten: every gate and shortcut (`all`),
    /// or one door by its key. For filming a fight that crosses rooms.
    pub fn unbar(&mut self, key: &str) -> Result<String, String> {
        let d = self.dungeon_def().cloned().ok_or("Not below.")?;
        let doors: Vec<String> = d
            .doors
            .iter()
            .filter(|dr| matches!(dr.kind.as_str(), "gate" | "shortcut") && (key == "all" || key.is_empty() || dr.key == key))
            .map(|dr| dr.key.clone())
            .collect();
        if doors.is_empty() {
            return Err(format!("No gate or shortcut called {key} here."));
        }
        for k in &doors {
            self.below.as_mut().unwrap().open.insert(k.clone());
            self.door_changed(k);
        }
        Ok(format!("Opened {}.", doors.join(", ")))
    }

    /// (Debug) The nearest foe within 40 yards (whose name or key holds `named`, if given) brought down to `share` of its health by this hero's
    /// hand, as a blow would (its phases come as they would): for filming a fight's turns.
    pub fn dev_hurt(&mut self, me: Id, share: f32, named: &str) -> Result<String, String> {
        let named = named.to_lowercase();
        let at = self.players.get(&me).map(|p| p.pos).ok_or("No hero.")?;
        let (mid, health, max) = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0 && !m.friendly && dist2d(m.pos, at) <= 40.0)
            .filter(|(_, m)| named.is_empty() || self.content.mobs[m.def].key.to_lowercase().contains(&named) || self.content.mobs[m.def].name.to_lowercase().contains(&named))
            .min_by(|a, b| dist2d(a.1.pos, at).total_cmp(&dist2d(b.1.pos, at)))
            .map(|(id, m)| (*id, m.health, m.max_health))
            .ok_or("No foe within 40 yards.")?;
        let to = (share.clamp(0.0, 1.0) * max as f32).round() as u32;
        if to >= health {
            return Err("It's already that low.".into());
        }
        let name = self.content.mobs[self.mobs[&mid].def].name.clone();
        if self.mobs[&mid].state.foe().is_none() {
            self.engage(mid, mobs::Foe::Hero(me));
        }
        self.hurt_mob_as(mid, me, health - to, "debug");
        Ok(format!("{name} is down to {}%.", (share * 100.0).round()))
    }

    /// (Debug) Every creature below taken away but those whose name or key holds `keep`, for
    /// filming one fight with nothing else in it. They're back at the next rest or reset.
    pub fn dev_clear_foes(&mut self, keep: &str) -> Result<String, String> {
        let keep = keep.to_lowercase();
        let gone: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| !m.friendly && !self.content.mobs[m.def].inert())
            .filter(|(_, m)| {
                let d = &self.content.mobs[m.def];
                keep.is_empty() || !(d.key.to_lowercase().contains(&keep) || d.name.to_lowercase().contains(&keep))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in &gone {
            self.mobs.remove(id);
            self.events.push(Event::MobGone { id: *id });
        }
        Ok(format!("Took away {} creatures.", gone.len()))
    }

    /// Whether this boss is at its slam (running to its wall, swinging, or only just struck): it
    /// is at nothing else, and nothing stops it.
    pub(crate) fn boss_slamming(&self, id: Id) -> bool {
        self.bosses.slams.get(&id).is_some_and(|s| self.clock < s.until)
    }

    /// The floor's edge from `from` the way `dir` points: the last spot with footing before a
    /// wall (no floor on, or none a walker could step to) and how far off it is. `None` with no
    /// wall within `most`, or where the floor only drops away (a ledge is no wall).
    fn slam_march(&self, from: Vec3, dir: [f32; 2], most: f32) -> Option<(Vec3, f32)> {
        let mut last = self.land.footing(from)?;
        let mut d = SLAM_STEP;
        while d <= most {
            let p = [from[0] + dir[0] * d, from[1] + dir[1] * d, last[2]];
            match self.land.footing(p) {
                Some(g) if self.land.walk_clear(last, g) => last = g,
                // (Floor there, but nothing walks onto it: a rock face, a pillar.)
                Some(_) => return Some((last, d - SLAM_STEP)),
                None => {
                    // (Floor a little below it: a drop, not a wall.)
                    let drop = (1..=4).any(|k| self.land.footing([p[0], p[1], last[2] - 1.5 * k as f32]).is_some());
                    return (!drop).then_some((last, d - SLAM_STEP));
                }
            }
            d += SLAM_STEP;
        }
        None
    }

    /// Where a slam at the wall `d` yards from `from` the way `dir` points (its floor's edge at
    /// `last`) is struck, and where the boss stands to strike it: `stand` short of it.
    fn slam_spot(&self, from: Vec3, dir: [f32; 2], last: Vec3, d: f32, stand: f32) -> (Vec3, Vec3) {
        let sd = (d + SLAM_STEP * 0.5 - stand).max(0.0);
        let xy = [from[0] + dir[0] * sd, from[1] + dir[1] * sd];
        let at = self.land.footing([xy[0], xy[1], last[2]]).or_else(|| self.land.footing([xy[0], xy[1], from[2]])).unwrap_or(last);
        ([last[0] + dir[0] * SLAM_STEP, last[1] + dir[1] * SLAM_STEP, last[2] + SLAM_HIGH], at)
    }

    /// The frame of a collapse's first door (its route from `a` on toward `b`): the wall behind
    /// the route's start or to either side of it, whichever the boss at `pos` is nearest, if it
    /// can be there in the slam's `door_secs` at its own run.
    fn slam_door(&self, pos: Vec3, a: Vec3, b: Vec3, def: &SlamDef, run: f32) -> Option<(Vec3, Vec3, Struck)> {
        let a = self.land.footing(a)?;
        let v = [b[0] - a[0], b[1] - a[1]];
        let len = (v[0] * v[0] + v[1] * v[1]).sqrt();
        if len < 1.0e-3 {
            return None;
        }
        let on = [v[0] / len, v[1] / len];
        let mut best: Option<(f32, Vec3, Vec3)> = None;
        for dir in [[-on[0], -on[1]], [-on[1], on[0]], [on[1], -on[0]]] {
            let Some((last, d)) = self.slam_march(a, dir, SLAM_DOOR_LOOK) else { continue };
            let (wall, stand) = self.slam_spot(a, dir, last, d, def.stand);
            let far = dist3(pos, stand);
            if far <= run * def.door_secs && best.as_ref().is_none_or(|b| far < b.0) {
                best = Some((far, wall, stand));
            }
        }
        best.map(|(_, wall, stand)| (wall, stand, Struck::Door))
    }

    /// The nearest wall to the boss at `pos` within the slam's reach, one toward `door` counted
    /// a little nearer than it is (the collapse comes from there).
    fn slam_wall(&self, pos: Vec3, door: Option<Vec3>, def: &SlamDef) -> Option<(Vec3, Vec3, Struck)> {
        let toward = door.map(|d| {
            let v = [d[0] - pos[0], d[1] - pos[1]];
            let len = (v[0] * v[0] + v[1] * v[1]).sqrt().max(1.0e-3);
            [v[0] / len, v[1] / len]
        });
        let mut best: Option<(f32, Vec3, Vec3)> = None;
        for k in 0..SLAM_WAYS {
            let a = k as f32 / SLAM_WAYS as f32 * std::f32::consts::TAU;
            let dir = [a.cos(), a.sin()];
            let Some((last, d)) = self.slam_march(pos, dir, def.reach + def.stand) else { continue };
            let score = d - toward.map_or(0.0, |t| SLAM_DOOR_LEAN * (dir[0] * t[0] + dir[1] * t[1]));
            if best.as_ref().is_none_or(|b| score < b.0) {
                let (wall, stand) = self.slam_spot(pos, dir, last, d, def.stand);
                best = Some((score, wall, stand));
            }
        }
        best.map(|(_, wall, stand)| (wall, stand, Struck::Wall))
    }

    /// The boss begins a slam. `opening`: the yell's own, at the first door's frame (`door`: the
    /// collapse's route, its first two points) if it can be there in time, else at the nearest
    /// wall, else on the floor before it; its blow lands `secs` after the yell. Otherwise a later
    /// section's pound: the floor where it stands, at once.
    fn slam_begins(&mut self, boss: Id, def: &SlamDef, door: Option<(Vec3, Vec3)>, opening: bool) {
        let Some(m) = self.mobs.get(&boss) else { return };
        let now = self.clock;
        let (pos, facing) = (m.pos, m.facing);
        let run = self.content.mobs[m.def].run_speed;
        let name = self.content.mobs[m.def].name.clone();
        let before = self.floor_beside(pos, [pos[0] + facing.cos() * def.stand, pos[1] + facing.sin() * def.stand]);
        let floor = (before, pos, Struck::Floor);
        let (wall, stand, what) = if opening {
            door.and_then(|(a, b)| self.slam_door(pos, a, b, def, run)).or_else(|| self.slam_wall(pos, door.map(|d| d.0), def)).unwrap_or(floor)
        } else {
            floor
        };
        let strike_at = now + if opening { def.secs.max(def.swing) } else { def.swing };
        // (At the door the wave starts under it: it is off the sooner.)
        let recover = if what == Struck::Door { def.recover.min(0.35) } else { def.recover };
        self.bosses.slams.insert(boss, Slam { def: def.clone(), wall, stand, what, strike_at, until: strike_at + recover, swung: false, struck: false, opening });
        if opening {
            let what = match what {
                Struck::Door => "the door's frame",
                Struck::Wall => "a wall",
                Struck::Floor => "the floor (no wall in reach)",
            };
            self.events.push(Event::Log(format!("{name} goes to slam {what} at ({:.1}, {:.1}), {:.1} yd off, in {:.1} s", wall[0], wall[1], dist3(pos, stand), strike_at - now)));
        }
    }

    /// The slams, each step: the run to the wall, the swing, the blow. Over, one stays known
    /// until the fight is: lost, what it shook loose goes.
    fn step_boss_slams(&mut self) {
        if self.bosses.slams.is_empty() {
            return;
        }
        let now = self.clock;
        let ids: Vec<Id> = self.bosses.slams.keys().copied().collect();
        for id in ids {
            let s = self.bosses.slams[&id].clone();
            let Some(m) = self.mobs.get(&id).filter(|m| m.health > 0) else {
                self.bosses.slams.remove(&id);
                continue;
            };
            if matches!(m.state, MobState::Return | MobState::Idle { .. }) {
                self.bosses.slams.remove(&id);
                // (A collapse's own loss takes its looks with it.)
                if !self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id) {
                    self.looks_leave(id);
                }
                continue;
            }
            if s.struck {
                continue;
            }
            let (pos, run) = (m.pos, self.content.mobs[m.def].run_speed);
            let swing_at = s.strike_at - s.def.swing;
            if !s.swung && now < swing_at {
                // On its way: as fast as gets it there for the swing.
                let far = dist2d(pos, s.stand);
                if far > SLAM_ARRIVE {
                    let heading = m.path.last().is_some_and(|end| dist3(*end, s.stand) <= 0.5);
                    if !heading {
                        let speed = (far / (swing_at - now - 0.1).max(0.05)).clamp(run, SLAM_RUN_MOST.max(run));
                        self.go(id, s.stand, speed);
                    }
                } else {
                    self.hold_facing(id, s.wall);
                }
                continue;
            }
            if !s.swung {
                // The swing: where it stands, at its wall (short of it, held or slowed on the
                // way: at the floor before it).
                self.halt(id);
                let (mut wall, mut what) = (s.wall, s.what);
                if what != Struck::Floor && dist2d(pos, s.stand) > 2.0 {
                    let face = facing(pos, s.wall);
                    wall = self.floor_beside(pos, [pos[0] + face.cos() * s.def.stand, pos[1] + face.sin() * s.def.stand]);
                    what = Struck::Floor;
                }
                let face = facing(pos, wall);
                if let Some(m) = self.mobs.get_mut(&id) {
                    m.facing = face;
                }
                self.events.push(Event::MobStop { id, pos, facing: face });
                self.events.push(Event::Clip { id, clip: s.def.clip, take: s.def.take, from: None, secs: s.def.swing + s.def.recover, at: s.def.swing, body: ClipBody::Full });
                let e = self.bosses.slams.get_mut(&id).unwrap();
                (e.wall, e.what, e.swung) = (wall, what, true);
            }
            if now >= s.strike_at {
                let e = self.bosses.slams.get_mut(&id).unwrap();
                e.struck = true;
                let s = e.clone();
                self.slam_strikes(id, &s);
            }
        }
    }

    /// The slam's blow lands: the one frame of it. The wall throws dust, every hero in the place
    /// is shaken by how near they stand, and (the yell's own) rock comes loose.
    fn slam_strikes(&mut self, id: Id, s: &Slam) {
        let Some(m) = self.mobs.get(&id) else { return };
        let now = self.clock;
        let pos = m.pos;
        let name = self.content.mobs[m.def].name.clone();
        let below = self.is_below();
        let floor = s.what == Struck::Floor;
        let foot = if floor { s.wall } else { [s.wall[0], s.wall[1], s.wall[2] - SLAM_HIGH] };
        self.events.push(Event::Sound { id, sound: SOUND_CRASH });
        // Everyone in the place feels it, hardest beside it. (A later section's pound: one size
        // down, and only those within sight of it.)
        let heroes: Vec<(Id, Vec3)> = self.players.iter().filter(|(_, p)| p.health > 0).map(|(h, p)| (*h, p.pos)).collect();
        let (near, mid) = if s.opening { (BEAT_SLAM_NEAR, BEAT_SLAM_MID) } else { (BEAT_SLAM_MID, BEAT_SLAM_FAR) };
        for (h, at) in &heroes {
            let far = dist3(pos, *at);
            let beat = if far <= SLAM_NEAR {
                near
            } else if far <= SLAM_MID {
                mid
            } else if s.opening && (below || far <= SLAM_FAR) {
                BEAT_SLAM_FAR
            } else {
                continue;
            };
            self.events.push(Event::Beat { player: *h, beat: beat.into(), on: None });
        }
        if !self.bosses.looks.off {
            // Dust off the wall where it struck, from its foot up, and across the floor.
            let ups: &[f32] = if floor { &[0.0] } else { &[0.0, SLAM_HIGH, SLAM_HIGH * 2.0] };
            for up in ups {
                self.point_look(id, [foot[0], foot[1], foot[2] + up], LAND_DUST);
            }
            let reach = if s.opening { 4.5 } else { 3.0 };
            self.events.push(Event::GroundBurst { at: foot, yaw: 0.0, shape: "ring", reach, arc: 360.0, width: 0.0, secs: 0.6, colour: DUST_COLOUR.into() });
        }
        if !s.opening {
            return;
        }
        // The lit band of the section it brings down: nothing lands on it, or hides it.
        let band = self.bosses.collapse.as_ref().filter(|c| c.boss == id).map(|c| {
            let s = &c.sections[c.section.min(c.sections.len() - 1)];
            (s.a, s.b, c.route.first().copied())
        });
        let off_band = |at: Vec3, by: f32| band.as_ref().is_none_or(|(a, b, _)| seg_far([at[0], at[1]], *a, *b) >= by);
        // A rock marked on the floor under each hero near it (the nearest few): out of its ring
        // before it lands, or struck by it (hurt, not thrown down: the run is still to come).
        let mut under: Vec<(f32, Vec3)> = heroes.iter().map(|(_, at)| (dist3(pos, *at), *at)).filter(|(far, at)| *far <= s.def.near && off_band(*at, LOOSE_BAND)).collect();
        under.sort_by(|a, b| a.0.total_cmp(&b.0));
        under.truncate(SLAM_MARKED);
        let damage = self.boss_blow(id, s.def.share);
        for (_, at) in &under {
            self.rocks_fall_as(id, *at, DEBRIS_RADIUS, DEBRIS_WARN, damage, false, DUST);
        }
        // The rock for show: off the wall itself first, then round it and at the door, small
        // ones and bigger, one after another over `volley` seconds. No ring and no blow.
        let mut loose = 0;
        if !self.bosses.looks.off {
            if !floor {
                let toward = facing(foot, pos);
                for j in 0..3 {
                    let side = toward + (j as f32 - 1.0) * 0.9;
                    let at = self.floor_beside(pos, [foot[0] + side.cos() * (1.0 + 0.5 * j as f32), foot[1] + side.sin() * (1.0 + 0.5 * j as f32)]);
                    if !off_band(at, LOOSE_BAND) {
                        continue;
                    }
                    let from = [foot[0], foot[1], foot[2] + SLAM_HIGH + 0.8 * j as f32];
                    self.rock_comes_loose(id, at, Some(from), 0, now + 0.4 + 0.1 * j as f32, 0.4);
                    loose += 1;
                }
            }
            // (The bigger the rock, the farther from every hero: none comes down between a hero
            // and their own view of the fight.)
            let clear = |at: Vec3, kind: usize| heroes.iter().all(|(_, h)| dist3(*h, at) >= LOOSE_CLEAR + LOOSE_WIDER * kind as f32) && off_band(at, LOOSE_BAND + kind as f32);
            let span = (s.def.volley - LOOSE_SECS).max(0.0);
            // Round where it stands: spread by the golden angle, 4 to 12 yd out.
            let (want, mut got, mut tries) = (s.def.rocks, 0, 0);
            while got < want && tries < want * 4 {
                let (angle, out) = (tries as f32 * 2.399 + 0.6, 4.0 + (tries * 5 % 9) as f32);
                tries += 1;
                // (The biggest fall first, the small ones last.)
                let kind = (LOOSE.len() - 1).saturating_sub(got * LOOSE.len() / want.max(1));
                let Some(at) = self.land.footing([pos[0] + angle.cos() * out, pos[1] + angle.sin() * out, pos[2]]) else { continue };
                if !self.land.walk_clear(pos, at) || !clear(at, kind) {
                    continue;
                }
                let land = now + LOOSE_SECS + span * got as f32 / want.max(1) as f32;
                self.rock_comes_loose(id, at, None, kind, land, LOOSE_SECS);
                got += 1;
            }
            loose += got;
            // At the door, to either side of the way in: the collapse starts there.
            if let Some(door) = band.as_ref().and_then(|b| b.2).and_then(|d| self.land.footing(d)) {
                let (want, mut got, mut tries) = (s.def.door_rocks, 0, 0);
                while got < want && tries < want * 4 {
                    let (angle, out) = (tries as f32 * 2.399 + 1.9, 2.0 + (tries * 3 % 5) as f32);
                    tries += 1;
                    let kind = got % LOOSE.len();
                    let Some(at) = self.land.footing([door[0] + angle.cos() * out, door[1] + angle.sin() * out, door[2]]) else { continue };
                    if !self.land.walk_clear(door, at) || !clear(at, kind) {
                        continue;
                    }
                    let land = now + LOOSE_SECS + span * (got as f32 + 0.5) / want.max(1) as f32;
                    self.rock_comes_loose(id, at, None, kind, land, LOOSE_SECS);
                    got += 1;
                }
                loose += got;
            }
        }
        let what = match s.what {
            Struck::Door => "the door's frame",
            Struck::Wall => "a wall",
            Struck::Floor => "the floor",
        };
        self.events.push(Event::Log(format!("{name} slams {what}: {loose} rocks come loose for show, {} marked under heroes", under.len())));
    }

    /// A rock shaken loose for show comes down on `at` (one of `LOOSE`'s sizes), landing at
    /// `land` after `secs` on its way: from the roof over it, or from `from` (off a wall). It is
    /// seen falling, strikes, hops and rolls to rest; it has no ring and no blow.
    fn rock_comes_loose(&mut self, owner: Id, at: Vec3, from: Option<Vec3>, kind: usize, land: f32, secs: f32) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let (display, scale, hop) = LOOSE[kind % LOOSE.len()];
        let k = self.look_seq();
        let yaw = (k as f32 * 2.399) % std::f32::consts::TAU;
        let to = [at[0], at[1], at[2] - rock_sink(display) * scale];
        let from = from.unwrap_or_else(|| [to[0] + yaw.cos() * 0.9, to[1] + yaw.sin() * 0.9, at[2] + self.drop_height(at)]);
        let tilt = (0.5 + (k % 5) as f32 * 0.15, (k % 3) as f32 * 0.04);
        let start = (land - secs).max(self.clock);
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: start,
            start,
            land,
            display,
            scale,
            path: Path::Drop { from, to, yaw, tilt },
            at,
            ring: 0.0,
            crack: 0,
            crash: if k % 3 == 0 { SOUND_CRASH } else { 0 },
            landed: false,
            face: false,
            hull: None,
            settle: SETTLE_SECS + 0.1,
            hop,
            struck: false,
        });
    }

    /// A section's face lands its first layer: the heroes near it are shaken, by how near.
    fn face_thuds(&mut self, c: &Collapse) {
        let s = &c.sections[c.section.min(c.sections.len() - 1)];
        let z = s.at[2];
        let shaken: Vec<(Id, &str)> = self
            .players
            .iter()
            .filter(|(_, p)| p.health > 0 && (p.pos[2] - z).abs() <= FACE_SHAKE_NEAR)
            .filter_map(|(h, p)| {
                let far = seg_far([p.pos[0], p.pos[1]], s.a, s.b);
                (far <= FACE_SHAKE_FAR).then(|| (*h, if far <= FACE_SHAKE_NEAR { BEAT_FACE_NEAR } else { BEAT_FACE_FAR }))
            })
            .collect();
        for (h, beat) in shaken {
            self.events.push(Event::Beat { player: h, beat: beat.into(), on: None });
        }
    }

    /// This share of the boss's own blow (as hard as its fight has grown).
    fn boss_blow(&self, id: Id, share: f32) -> u32 {
        let d = self.mobs.get(&id).map_or(0, |m| self.content.mobs[m.def].attack.damage);
        (d as f32 * share * self.foe_blow_share(id)).round().max(1.0) as u32
    }

    /// Rocks come down on `at`: dust there first (`warn` seconds), then the fall, which strikes
    /// whoever is in it (heavy: thrown down).
    fn rocks_fall(&mut self, owner: Id, at: Vec3, radius: f32, warn: f32, damage: u32, heavy: bool) {
        self.rocks_fall_as(owner, at, radius, warn, damage, heavy, DUST);
    }

    /// `rocks_fall` with the dust named (`look`: a collapse's own falls are in its soft dust,
    /// not a burst of their own).
    #[allow(clippy::too_many_arguments)]
    fn rocks_fall_as(&mut self, owner: Id, at: Vec3, radius: f32, warn: f32, damage: u32, heavy: bool, look: u32) {
        let now = self.clock;
        let at = self.ground_near(at[0], at[1], at[2]);
        if look == DUST {
            self.events.push(Event::Missile { caster: owner, spell: look, target: Target::Point(at) });
        } else if !self.bosses.looks.off {
            self.point_look(owner, at, look);
        }
        let mark = self.mark_ground(at, radius, warn, look);
        self.missiles.push(combat::Missile { owner, from: at, at, radius, damage, blockable: false, heavy, land_at: now + warn, mark, look });
        let near: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= radius + 2.0).map(|(h, _)| *h).collect();
        for v in near {
            self.events.push(Event::Danger { player: v, secs: warn, from: None });
        }
        // How it looks: its ring on the floor till it lands, and a rock dropping onto it.
        if !self.bosses.looks.off {
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: radius, arc: 360.0, width: 0.0, secs: warn, colour: mobs::HAZARD_COLOUR.into() });
            let scale = if heavy { CHUNK_HEAVY } else { CHUNK_SCALE };
            // (A collapse's own fall is of its pale rock.)
            let (display, scale) = if look == DUST { (ROCK, scale) } else { (PALE, scale * PALE_SIZE * 1.6) };
            self.rock_drops(owner, at, display, scale, now + warn, radius, 0);
        }
    }

    // --- how the falling mine looks --------------------------------------------------------------

    /// A piece of the falling mine (on its way down or lying where it fell), or of the Shredder's
    /// rigged log and its crash (`sim::machinery`), and its tilt: the server draws it at once
    /// with no hull, moved in place as it falls.
    pub fn collapse_piece_tilt(&self, id: Id) -> Option<f32> {
        self.bosses.looks.tilt.get(&id).copied().or_else(|| self.rig_piece_tilt(id))
    }

    /// The loose pieces lying where they fell (for tests and the log).
    pub fn collapse_rubble(&self) -> Vec<Id> {
        self.bosses.looks.rubble.iter().map(|(_, id)| *id).collect()
    }

    /// The rocks of the faces, landed and solid (for tests and the log).
    pub fn collapse_faces(&self) -> Vec<Id> {
        self.bosses.looks.face_pieces.iter().map(|(_, id)| *id).collect()
    }

    /// The pale rocks lying over the faces' solid ones: what's seen of a face (for tests).
    pub fn collapse_face_skin(&self) -> Vec<Id> {
        self.bosses.looks.swaps.iter().map(|(_, _, id)| *id).collect()
    }

    /// Whether a piece is one of a face's rocks where it landed: drawn with its hull, as any
    /// object, not as a piece that moves.
    pub fn collapse_piece_solid(&self, id: Id) -> bool {
        self.bosses.looks.solid.contains(&id)
    }

    /// The next of the looks' own count (their variety, never the rules' dice).
    fn look_seq(&mut self) -> u32 {
        let l = &mut self.bosses.looks;
        l.seq = l.seq.wrapping_add(1);
        l.seq
    }

    /// How high over the floor at `at` a piece starts down: the ceiling's height in a tunnel, or
    /// lower where a floor lies overhead (the Foundry's ring over its pit).
    fn drop_height(&self, at: Vec3) -> f32 {
        let mut h = DROP_HIGH;
        while h > DROP_LOW && self.land.slab_between(at[0], at[1], at[2] + 0.3, at[2] + h + 0.5) {
            h -= 0.5;
        }
        h
    }

    /// A rock (`display` at `scale`) drops onto `at`, landing at `land` (shaking the heroes beside
    /// it, `ring` its ring's reach), and stays there.
    #[allow(clippy::too_many_arguments)]
    fn rock_drops(&mut self, owner: Id, at: Vec3, display: u32, scale: f32, land: f32, ring: f32, crash: u32) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let k = self.look_seq();
        let yaw = (k as f32 * 2.399) % std::f32::consts::TAU;
        let to = [at[0], at[1], at[2] - rock_sink(display) * scale];
        let h = self.drop_height(at);
        // In from a little to one side, as rock coming away from a wall.
        let from = [to[0] + yaw.cos() * 0.7, to[1] + yaw.sin() * 0.7, at[2] + h];
        let tilt = (0.5 + (k % 5) as f32 * 0.15, (k % 3) as f32 * 0.04);
        let start = (land - ROCK_SECS).max(self.clock);
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: start,
            start,
            land,
            display,
            scale,
            path: Path::Drop { from, to, yaw, tilt },
            at,
            ring,
            crack: 0,
            crash,
            landed: false,
            face: false,
            hull: None,
            settle: SETTLE_SECS,
            hop: HOP_FALL,
            struck: false,
        });
    }

    /// A beam cracks overhead at `hinge` (its floor beneath), lying along `yaw`, and comes down to
    /// land at `land`: its loose end to the floor, then (`flat`) the rest after it. It stays.
    fn beam_falls(&mut self, owner: Id, hinge: Vec3, yaw: f32, land: f32, flat: bool) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let floor = hinge[2];
        let h = self.drop_height(hinge);
        let start = (land - BEAM_SECS).max(self.clock);
        let path = Path::Beam { hinge: [hinge[0], hinge[1], floor + h], yaw, floor, flat };
        let (foot, _, _) = beam_pose(&path, 1.0);
        let len = BEAM_LEN * BEAM_SCALE;
        // Where it comes down: about the middle of where it lies (or leans).
        let at = [foot[0] + yaw.cos() * len * 0.4, foot[1] + yaw.sin() * len * 0.4, floor];
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: (start - BEAM_HOLD).max(self.clock),
            start,
            land,
            display: BEAM,
            scale: BEAM_SCALE,
            path,
            at,
            ring: len * 0.5,
            crack: SOUND_CRACK,
            crash: SOUND_CRASH,
            landed: false,
            face: false,
            hull: None,
            settle: 0.0,
            hop: 0.0,
            struck: false,
        });
    }

    /// The wave's own rock (a row across the section every `ROW_STEP` it comes, each row's rings
    /// on the floor `ROW_SECS` before the wave reaches it) and timbers (one across the tunnel
    /// every `BEAM_STEP`): what fills the section behind it.
    fn wave_comes_down(&mut self, c: &mut Collapse) {
        if self.bosses.looks.off {
            return;
        }
        let now = self.clock;
        let boss = c.boss;
        let (from, line) = (c.sections[c.section].from, c.sections[c.section].line);
        let shown = c.front + c.speed * ROW_SECS;
        while c.rowed + ROW_STEP <= shown {
            c.rowed += ROW_STEP;
            if c.rowed <= line - ROW_SHORT {
                let d = c.rowed;
                self.row_falls(c, d);
            }
        }
        while c.beamed + BEAM_STEP <= c.front {
            c.beamed += BEAM_STEP;
            if c.beamed < from + 1.0 || c.beamed > line - ROW_SHORT {
                continue;
            }
            let k = self.look_seq();
            let (p, yaw) = route_pose(&c.route, &c.along, c.beamed);
            // Hinged up by one wall, across the tunnel to the other.
            let hand = if k % 2 == 0 { 1.0 } else { -1.0 };
            let across = yaw + hand * std::f32::consts::FRAC_PI_2;
            let hinge = self.floor_beside(p, [p[0] - across.cos() * 2.6, p[1] - across.sin() * 2.6]);
            // Every other one stays leaning where its end struck the floor; the rest come down flat.
            self.beam_falls(boss, hinge, across, now + BEAM_HOLD + BEAM_SECS, k % 4 < 2);
        }
    }

    /// One row of the wave, `d` along the route: a rock every `ROW_GAP` across the section's
    /// floor from wall to wall, its ring on the floor until it lands.
    fn row_falls(&mut self, c: &Collapse, d: f32) {
        let now = self.clock;
        let s = &c.sections[c.section];
        let (p, yaw) = route_pose(&c.route, &c.along, d);
        let side = [-yaw.sin(), yaw.cos()];
        // The floor out to each wall: on as long as there's footing, one gap (a pillar) passed.
        let mut spots = vec![p];
        for hand in [1.0f32, -1.0] {
            let (mut z, mut missed) = (p[2], 0);
            for j in 1..=ROW_SIDE {
                let off = j as f32 * ROW_GAP * hand;
                if off.abs() > s.reach {
                    break;
                }
                let xy = [p[0] + side[0] * off, p[1] + side[1] * off];
                match self.land.footing([xy[0], xy[1], z]) {
                    Some(g) => {
                        z = g[2];
                        missed = 0;
                        spots.push(g);
                    }
                    None => {
                        missed += 1;
                        if missed > 1 {
                            break;
                        }
                    }
                }
            }
        }
        let layers = ((s.height / ROW_FILLS).round() as usize).clamp(1, ROW_LAYERS);
        for spot in spots {
            // (Nothing past the line: the face stands there.)
            if s.authored && s.side(spot) > -1.0 {
                continue;
            }
            let k = self.look_seq();
            let jit = [((k % 7) as f32 - 3.0) * 0.25, ((k % 5) as f32 - 2.0) * 0.3];
            let at = self.floor_beside(spot, [spot[0] + jit[0], spot[1] + jit[1]]);
            let land = now + ROW_SECS + (k % 4) as f32 * 0.08;
            // (No ring: the wave is no blow of its own. What it does to whoever it reaches is
            // the catch's.)
            for layer in 0..layers {
                let n = k as usize + layer * 3;
                let (display, scale) = if n % 5 == 4 { (PALE_TALL, ROW_BOULDER) } else { (PALE, ROW_SCALES[n % 4]) };
                let crash = if n % 3 == 0 { SOUND_CRASH } else { 0 };
                self.row_rock(c.boss, at, s.height, ROW_RISE * layer as f32, display, scale, land + ROW_LAYER_SECS * layer as f32, crash);
            }
        }
    }

    /// One of a row's rocks: out of the roof over `at` (`height` up) and down onto the pile
    /// there, `rise` over the floor, landing at `land`. It stays.
    #[allow(clippy::too_many_arguments)]
    fn row_rock(&mut self, owner: Id, at: Vec3, height: f32, rise: f32, display: u32, scale: f32, land: f32, crash: u32) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let k = self.look_seq();
        let yaw = (k as f32 * 2.399) % std::f32::consts::TAU;
        let to = [at[0], at[1], at[2] + rise - rock_sink(display) * scale];
        let from = [to[0] + yaw.cos() * 0.7, to[1] + yaw.sin() * 0.7, (at[2] + height).max(to[2] + 2.5)];
        // (The pile's upper rocks lie as they fell, at more of a slant than the floor's.)
        let lean = if rise > 0.0 { ((k % 5) as f32 - 2.0) * 0.14 } else { (k % 3) as f32 * 0.04 };
        let tilt = (0.5 + (k % 5) as f32 * 0.15, lean);
        let start = (land - ROW_FALL_SECS).max(self.clock);
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: start,
            start,
            land,
            display,
            scale,
            path: Path::Drop { from, to, yaw, tilt },
            at: [at[0], at[1], at[2] + rise],
            ring: 0.0,
            crack: 0,
            crash,
            landed: false,
            face: false,
            hull: None,
            settle: SETTLE_SECS,
            hop: if rise > 0.0 { HOP_PILE } else { HOP_ROW },
            struck: false,
        });
    }

    /// The lamps along a section's line, lit as its surge begins: wall to wall on the clear side.
    fn line_lamps(&mut self, owner: Id, s: &Section) {
        if self.bosses.looks.off {
            return;
        }
        let u = [s.b[0] - s.a[0], s.b[1] - s.a[1]];
        let wide = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1.0e-3);
        let u = [u[0] / wide, u[1] / wide];
        // (Past the line: the clear side.)
        let ahead = [-u[1] * s.ahead, u[0] * s.ahead];
        let n = ((wide / LINE_LAMP_GAP).round() as usize).max(2);
        for j in 0..n {
            let across = (j as f32 + 0.5) * wide / n as f32;
            let xy = [s.a[0] + u[0] * across + ahead[0] * LINE_LAMP_AHEAD, s.a[1] + u[1] * across + ahead[1] * LINE_LAMP_AHEAD];
            // (Not on the floor the caught are thrown to.)
            if n > 2 && dist2d([xy[0], xy[1], 0.0], s.at) < LINE_LAMP_CLEAR {
                continue;
            }
            let pos = self.floor_beside(s.at, xy);
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Prop { display: LINE_LAMP, scale: LINE_LAMP_SCALE }, pos, facing: j as f32 * 1.3 });
            self.bosses.looks.tilt.insert(id, 0.0);
            self.bosses.looks.lamps.push((owner, id));
            self.events.push(Event::ObjSpawned { id });
        }
    }

    /// A section's face is built as its wave reaches the line: rocks side by side from wall to
    /// wall, their long sides along it, layer on layer to the roof, each dropped from over the
    /// roof onto its place. Each is solid from the moment it lands (`step_falling_looks`).
    fn face_builds(&mut self, owner: Id, s: &Section) {
        if self.bosses.looks.off {
            return;
        }
        let now = self.clock;
        let u = [s.b[0] - s.a[0], s.b[1] - s.a[1]];
        let wide = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1.0e-3);
        let u = [u[0] / wide, u[1] / wide];
        // (Behind the line: the buried side.)
        let back = [u[1] * s.ahead, -u[0] * s.ahead];
        let scale = (s.height / FACE_ROOF).clamp(0.8, 1.4);
        let rise = FACE_RISE * scale;
        let layers = ((s.height / rise).ceil() as usize).clamp(1, FACE_LAYERS);
        let n = ((wide / (FACE_PITCH * scale)).ceil() as usize).max(1);
        let facing = u[1].atan2(u[0]) - std::f32::consts::FRAC_PI_2;
        for layer in 0..layers {
            // (Each layer's rocks over the joints of the one below.)
            let count = if layer % 2 == 0 { n } else { n + 1 };
            for j in 0..count {
                let across = if layer % 2 == 0 { (j as f32 + 0.5) * wide / n as f32 } else { j as f32 * wide / n as f32 };
                let k = self.look_seq();
                let d = FACE_BACK * scale + FACE_GAP + FACE_LEAN * layer as f32;
                let xy = [s.a[0] + u[0] * across + back[0] * d, s.a[1] + u[1] * across + back[1] * d];
                let floor = self.floor_beside(s.at, xy);
                let yaw = facing + ((k % 5) as f32 - 2.0) * 0.06;
                let hull = [xy[0], xy[1], floor[2] + rise * layer as f32 - rock_sink(ROCK) * scale];
                // The pale rock over it, a little proud of it.
                let pale = scale * PALE_SIZE * FACE_PALE;
                let dp = FACE_PALE_BACK * FACE_PALE * scale + FACE_PALE_GAP + FACE_LEAN * layer as f32;
                let to = [s.a[0] + u[0] * across + back[0] * dp, s.a[1] + u[1] * across + back[1] * dp, floor[2] + rise * layer as f32 - rock_sink(PALE) * pale];
                let from = [to[0], to[1], (s.at[2] + s.height + 1.0).max(to[2] + 3.0)];
                let land = now + FACE_FIRST + FACE_LAYER_SECS * layer as f32 + (k % 3) as f32 * 0.05;
                let start = (land - ROCK_SECS).max(now);
                self.bosses.looks.falling.push(Falling {
                    owner,
                    id: None,
                    shows: start,
                    start,
                    land,
                    display: PALE,
                    scale: pale,
                    path: Path::Drop { from, to, yaw, tilt: (0.3, 0.0) },
                    at: floor,
                    ring: 0.0,
                    crack: 0,
                    crash: if k % 4 == 0 { SOUND_CRASH } else { 0 },
                    landed: false,
                    face: true,
                    hull: Some((ROCK, scale, hull, yaw)),
                    settle: 0.0,
                    hop: 0.0,
                    struck: false,
                });
            }
        }
    }

    /// The band across a surge's line, from its first moment to its last: the line to get past,
    /// from where the route crosses it out to each wall.
    fn surge_band(&mut self, c: &Collapse) {
        if self.bosses.looks.off {
            return;
        }
        let s = &c.sections[c.section];
        for end in [s.a, s.b] {
            let d = [end[0] - s.at[0], end[1] - s.at[1]];
            let reach = (d[0] * d[0] + d[1] * d[1]).sqrt();
            if reach < 0.5 {
                continue;
            }
            self.events.push(Event::GroundBurst { at: s.at, yaw: d[1].atan2(d[0]), shape: "line", reach, arc: 0.0, width: BAND_WIDTH, secs: BAND_SECS, colour: BAND_COLOUR.into() });
        }
    }

    /// Each dust beat of a surge: dust from the roof over what's still to come down, dust rolling
    /// on ahead of the wave, the rumble, and the shaking for every hero in the section or near
    /// its wave.
    fn surge_dust(&mut self, c: &mut Collapse) {
        if self.bosses.looks.off {
            return;
        }
        let now = self.clock;
        let boss = c.boss;
        let k = c.section;
        let (line, reach, height) = (c.sections[k].line, c.sections[k].reach.min(ROOF_DUST_SIDE), c.sections[k].height);
        // Dust sifting out of the roof over all that's still to come down: soft clouds under
        // it, half way to the floor and at head height, where a hero down the tunnel sees them
        // (no burst: nothing has landed).
        for j in 0..ROOF_DUST {
            let n = self.look_seq();
            let d = c.front + (line - c.front) * ((n % 11) as f32 + 0.5) / 11.0;
            let off = ((n % 7) as f32 / 3.0 - 1.0) * reach;
            let (p, yaw) = route_pose(&c.route, &c.along, d);
            let at = self.floor_beside(p, [p[0] - yaw.sin() * off, p[1] + yaw.cos() * off]);
            let up = match j % 3 {
                0 => height - 1.5,
                1 => height * 0.5,
                _ => 1.5,
            };
            self.point_look(boss, [at[0], at[1], at[2] + up.max(1.0)], LAND_DUST);
        }
        let front = route_point(&c.route, &c.along, c.front);
        if c.stage == Stage::Wave {
            // The front itself: a wall of dust from wall to wall, and rolling on ahead of it.
            let wide = c.sections[k].reach.min(ROW_SIDE as f32 * ROW_GAP);
            let (p, yaw) = route_pose(&c.route, &c.along, (c.front + 1.5).min(line));
            let side = [-yaw.sin(), yaw.cos()];
            let mut off = -wide;
            while off <= wide {
                let xy = [p[0] + side[0] * off, p[1] + side[1] * off];
                if off.abs() < 0.1 || self.land.footing([xy[0], xy[1], p[2]]).is_some() {
                    let at = self.floor_beside(p, xy);
                    self.point_look(boss, at, DUST_ROLL);
                }
                off += FRONT_DUST_GAP;
            }
            let at = route_point(&c.route, &c.along, (c.front + 4.5).min(line));
            self.point_look(boss, at, DUST_ROLL);
        }
        if now >= c.rumble_at {
            c.rumble_at = now + RUMBLE_EVERY;
            // (From the newest rock down, where there is one: the sound comes from the rubble.)
            let from = self.bosses.looks.rubble.back().map_or(boss, |&(_, piece)| piece);
            self.events.push(Event::Sound { id: from, sound: SOUND_RUMBLE });
        }
        let near: Vec<Id> = self
            .players
            .iter()
            .filter(|(_, h)| h.health > 0 && (dist3(h.pos, front) <= RUMBLE_NEAR || c.in_section(k, h.pos, 0.0).is_some()))
            .map(|(h, _)| *h)
            .collect();
        for h in near {
            self.events.push(Event::Beat { player: h, beat: BEAT_RUMBLE.into(), on: None });
        }
    }

    /// Dust at the foot of the face the collapse stands at (and half way up it): the rubble is
    /// still settling, and the dust in the lamps' light shows it for the wall it is.
    fn face_dust(&mut self, c: &Collapse) {
        if self.bosses.looks.off {
            return;
        }
        let s = &c.sections[c.section];
        let n = self.look_seq();
        let t = ((n % 7) as f32 + 0.5) / 7.0;
        let ahead = {
            let u = [s.b[0] - s.a[0], s.b[1] - s.a[1]];
            let wide = (u[0] * u[0] + u[1] * u[1]).sqrt().max(1.0e-3);
            [-u[1] / wide * s.ahead, u[0] / wide * s.ahead]
        };
        let xy = [s.a[0] + (s.b[0] - s.a[0]) * t + ahead[0] * 0.8, s.a[1] + (s.b[1] - s.a[1]) * t + ahead[1] * 0.8];
        let at = self.floor_beside(s.at, xy);
        let up = if n % 2 == 0 { 0.0 } else { s.height * 0.5 };
        self.point_look(c.boss, [at[0], at[1], at[2] + up], LAND_DUST);
    }

    /// A spot beside the route's point `p`, on its floor (the route's own height if the spot is
    /// off any floor near it).
    fn floor_beside(&self, p: Vec3, xy: [f32; 2]) -> Vec3 {
        let g = self.ground_near(xy[0], xy[1], p[2]);
        if g[2].is_finite() && (g[2] - p[2]).abs() < 2.0 { g } else { [xy[0], xy[1], p[2]] }
    }

    /// A ground look at `at` from an unseen stand-in on the spot (no caster's pose, and seen
    /// however far its owner is).
    fn point_look(&mut self, owner: Id, at: Vec3, spell: u32) {
        let anchor = self.alloc();
        self.events.push(Event::MissileFrom { anchor, caster: owner, spell, from: at, target: Target::Point(at) });
        self.events.push(Event::ObjGone { id: anchor });
    }

    /// The pieces on their way down move; those landing burst, shake whoever is beside them and
    /// stay (a face's rock solid from then: its twin, with a hull, in its place). What a reset
    /// took away is let go.
    fn step_falling_looks(&mut self) {
        let l = &self.bosses.looks;
        if l.falling.is_empty() && l.rubble.is_empty() && l.tilt.is_empty() {
            return;
        }
        self.step_looks_leaving();
        let objs = &self.objs;
        let l = &mut self.bosses.looks;
        l.lamps.retain(|(_, id)| objs.contains_key(id));
        l.rubble.retain(|(_, id)| objs.contains_key(id));
        l.face_pieces.retain(|(_, id)| objs.contains_key(id));
        l.swaps.retain(|(_, _, id)| objs.contains_key(id));
        l.solid.retain(|id| objs.contains_key(id));
        l.falling.retain(|f| f.id.is_none_or(|id| objs.contains_key(&id)));
        l.tilt.retain(|id, _| objs.contains_key(id));
        let now = self.clock;
        // A face's rocks as they fell go, their solid twins shown by now.
        if l.swaps.iter().any(|(t, ..)| now >= *t) {
            let due: Vec<Id> = l.swaps.iter().filter(|(t, ..)| now >= *t).map(|(_, _, id)| *id).collect();
            l.swaps.retain(|(t, ..)| now < *t);
            for id in due {
                self.bosses.looks.tilt.remove(&id);
                if self.objs.remove(&id).is_some() {
                    self.events.push(Event::ObjGone { id });
                }
            }
        }
        let (mut struck, mut landed) = (Vec::new(), Vec::new());
        for k in 0..self.bosses.looks.falling.len() {
            let f = self.bosses.looks.falling[k].clone();
            if now < f.shows {
                continue;
            }
            let t = if f.land > f.start { ((now - f.start) / (f.land - f.start)).clamp(0.0, 1.0) } else { 1.0 };
            // (Down: it hops and rolls to rest.)
            let settling = t >= 1.0 && f.settle > 0.0;
            let hop = if f.settle > 0.0 { f.hop } else { 0.0 };
            let (pos, facing, tilt) = if settling { f.path.settle(((now - f.land) / f.settle).clamp(0.0, 1.0), hop) } else { f.path.pose(t, hop) };
            match f.id {
                None => {
                    let id = self.alloc();
                    self.objs.insert(id, Obj { kind: ObjKind::Prop { display: f.display, scale: f.scale }, pos, facing });
                    self.bosses.looks.tilt.insert(id, tilt);
                    self.events.push(Event::ObjSpawned { id });
                    if f.crack != 0 {
                        self.events.push(Event::Sound { id, sound: f.crack });
                    }
                    self.bosses.looks.falling[k].id = Some(id);
                }
                Some(id) if now >= f.start => {
                    if let Some(o) = self.objs.get_mut(&id) {
                        o.pos = pos;
                        o.facing = facing;
                    }
                    self.bosses.looks.tilt.insert(id, tilt);
                    self.events.push(Event::ObjChanged { id });
                }
                Some(_) => {}
            }
            if t >= 1.0 && now >= f.start {
                if !f.struck {
                    self.bosses.looks.falling[k].struck = true;
                    struck.push(k);
                }
                if now >= f.land + f.settle {
                    self.bosses.looks.falling[k].landed = true;
                    landed.push(k);
                }
            }
        }
        for k in struck {
            let f = self.bosses.looks.falling[k].clone();
            let Some(id) = f.id else { continue };
            // Dust thrown up where it struck, its ring on the floor (a fall's own), its crash.
            self.point_look(f.owner, f.at, LAND_DUST);
            if f.ring > 0.0 {
                self.events.push(Event::GroundBurst { at: f.at, yaw: 0.0, shape: "ring", reach: f.ring, arc: 360.0, width: 0.0, secs: 0.5, colour: DUST_COLOUR.into() });
            }
            if f.crash != 0 {
                self.events.push(Event::Sound { id, sound: f.crash });
            }
            let shaken: Vec<Id> = self
                .players
                .iter()
                .filter(|(h, p)| p.health > 0 && dist3(p.pos, f.at) <= SHAKE_NEAR.max(f.ring + 2.0)
                    && self.bosses.looks.shook.get(*h).is_none_or(|t| now - t >= SHAKE_GAP))
                .map(|(h, _)| *h)
                .collect();
            for h in shaken {
                self.bosses.looks.shook.insert(h, now);
                self.events.push(Event::Beat { player: h, beat: BEAT_ROCKFALL.into(), on: None });
            }
        }
        for k in landed {
            let f = self.bosses.looks.falling[k].clone();
            let Some(id) = f.id else { continue };
            if f.face {
                // Solid from here: its hull stands behind it, drawn with one (a piece that moves
                // has none). The pale rock that fell stays over it.
                let (pos, facing, tilt) = f.path.pose(1.0, 0.0);
                let (display, scale, pos, facing) = f.hull.unwrap_or((f.display, f.scale, pos, facing));
                let twin = self.alloc();
                self.objs.insert(twin, Obj { kind: ObjKind::Prop { display, scale }, pos, facing });
                let l = &mut self.bosses.looks;
                l.tilt.insert(twin, tilt);
                l.solid.insert(twin);
                l.face_pieces.push((f.owner, twin));
                l.swaps.push((f32::INFINITY, f.owner, id));
                self.events.push(Event::ObjSpawned { id: twin });
                continue;
            }
            let l = &mut self.bosses.looks;
            l.rubble.push_back((f.owner, id));
            // (The loose pieces alone are counted out: a face stands whatever the count.)
            while l.rubble.len() > RUBBLE_MOST {
                let (_, old) = l.rubble.pop_front().unwrap();
                l.tilt.remove(&old);
                if self.objs.remove(&old).is_some() {
                    self.events.push(Event::ObjGone { id: old });
                }
            }
        }
        self.bosses.looks.falling.retain(|f| !f.landed);
        self.bosses.looks.shook.retain(|_, t| now - *t < SHAKE_GAP);
    }

    /// Everything of `owner`'s that came down (or is coming down) gone, its faces' rocks too: its
    /// fight lost, the mine stands again.
    fn clear_looks(&mut self, owner: Id) {
        let l = &mut self.bosses.looks;
        if l.leaving.is_some_and(|(o, ..)| o == owner) {
            l.leaving = None;
        }
        let mut gone: Vec<Id> = l.rubble.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id).collect();
        gone.extend(l.lamps.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id));
        l.lamps.retain(|(o, _)| *o != owner);
        gone.extend(l.falling.iter().filter(|f| f.owner == owner).filter_map(|f| f.id));
        gone.extend(l.face_pieces.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id));
        gone.extend(l.swaps.iter().filter(|(_, o, _)| *o == owner).map(|(_, _, id)| *id));
        l.rubble.retain(|(o, _)| *o != owner);
        l.falling.retain(|f| f.owner != owner);
        l.face_pieces.retain(|(o, _)| *o != owner);
        l.swaps.retain(|(_, o, _)| *o != owner);
        for id in gone {
            self.bosses.looks.tilt.remove(&id);
            self.bosses.looks.solid.remove(&id);
            if self.objs.remove(&id).is_some() {
                self.events.push(Event::ObjGone { id });
            }
        }
    }
}

impl Survival {
    /// How many of `owner`'s rocks lie or stand.
    fn looks_left(&self, owner: Id) -> usize {
        let l = &self.bosses.looks;
        l.rubble.iter().filter(|(o, _)| *o == owner).count()
            + l.face_pieces.iter().filter(|(o, _)| *o == owner).count()
            + l.swaps.iter().filter(|(_, o, _)| *o == owner).count()
    }

    /// A lost fight's rock begins to go: what was in the air is gone at once, and dust rises
    /// over everything that lies or stands (`step_looks_leaving` takes it away).
    fn looks_leave(&mut self, owner: Id) {
        let l = &mut self.bosses.looks;
        let air: Vec<Id> = l.falling.iter().filter(|f| f.owner == owner).filter_map(|f| f.id).collect();
        l.falling.retain(|f| f.owner != owner);
        for id in air {
            self.bosses.looks.tilt.remove(&id);
            if self.objs.remove(&id).is_some() {
                self.events.push(Event::ObjGone { id });
            }
        }
        let n = self.looks_left(owner);
        if n == 0 {
            self.clear_looks(owner);
            return;
        }
        self.bosses.looks.leaving = Some((owner, self.clock, n, false));
        self.leaving_dust(owner, 4);
    }

    /// Dust over a lost fight's rock, and its rumble: at every `every`th face rock and (thinner)
    /// loose piece.
    fn leaving_dust(&mut self, owner: Id, every: usize) {
        let l = &self.bosses.looks;
        let faces: Vec<Id> = l.face_pieces.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id).collect();
        let loose: Vec<Id> = l.rubble.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id).collect();
        let spots: Vec<Vec3> = faces
            .iter()
            .step_by(every.max(1))
            .chain(loose.iter().step_by(every.max(1) * 3))
            .filter_map(|id| self.objs.get(id).map(|o| o.pos))
            .collect();
        for at in spots {
            self.point_look(owner, at, LAND_DUST);
        }
        if let Some(from) = faces.last().or(loose.last()) {
            self.events.push(Event::Sound { id: *from, sound: SOUND_RUMBLE });
        }
    }

    /// A lost fight's rock goes: it stands `LEAVE_HOLD` in its dust, then goes over `LEAVE_SECS`,
    /// the newest first (the faces before the loose rock behind them), dust where it goes.
    fn step_looks_leaving(&mut self) {
        let Some((owner, since, n, begun)) = self.bosses.looks.leaving else { return };
        let now = self.clock;
        if now < since + LEAVE_HOLD {
            return;
        }
        let left = self.looks_left(owner);
        if !begun {
            self.leaving_dust(owner, 3);
            self.bosses.looks.leaving = Some((owner, since, n, true));
        }
        let share = ((now - since - LEAVE_HOLD) / LEAVE_SECS).clamp(0.0, 1.0);
        let keep = ((1.0 - share) * n as f32).floor() as usize;
        if share >= 1.0 || keep == 0 {
            self.clear_looks(owner);
            return;
        }
        let mut k = 0usize;
        while left.saturating_sub(k) > keep {
            let l = &mut self.bosses.looks;
            let id = if let Some(p) = l.swaps.iter().rposition(|(_, o, _)| *o == owner) {
                l.swaps.remove(p).2
            } else if let Some(p) = l.face_pieces.iter().rposition(|(o, _)| *o == owner) {
                l.face_pieces.remove(p).1
            } else if let Some(p) = l.rubble.iter().rposition(|(o, _)| *o == owner) {
                l.rubble.remove(p).map(|(_, id)| id).unwrap()
            } else {
                break;
            };
            l.tilt.remove(&id);
            l.solid.remove(&id);
            if let Some(o) = self.objs.remove(&id) {
                self.events.push(Event::ObjGone { id });
                if k % 6 == 0 {
                    self.point_look(owner, o.pos, LAND_DUST);
                }
            }
            k += 1;
        }
    }
}

impl Path {
    /// Where its prop stands `t` of the way down (0 to 1, falling faster as it goes): its
    /// position, facing and tilt (about the model's own Y after its facing, the drawing's). A
    /// rock that settles (`hop` over 0) strikes the floor short of its resting place by its roll.
    fn pose(&self, t: f32, hop: f32) -> (Vec3, f32, f32) {
        match *self {
            Path::Drop { from, to, yaw, tilt } => {
                let u = t * t;
                let short = self.roll(hop);
                let to = [to[0] - short[0], to[1] - short[1], to[2]];
                let pos = [from[0] + (to[0] - from[0]) * u, from[1] + (to[1] - from[1]) * u, from[2] + (to[2] - from[2]) * u];
                let turn = if hop > 0.0 { SETTLE_TURN } else { 0.0 };
                (pos, yaw + turn + (1.0 - u) * 1.3, tilt.0 + (tilt.1 - tilt.0) * u)
            }
            Path::Beam { .. } => beam_pose(self, t),
        }
    }

    /// How far a dropped rock rolls as it settles, on the way it came: `hop` times `SETTLE_ROLL`.
    fn roll(&self, hop: f32) -> [f32; 2] {
        let Path::Drop { from, to, .. } = *self else { return [0.0; 2] };
        let d = [to[0] - from[0], to[1] - from[1]];
        let len = (d[0] * d[0] + d[1] * d[1]).sqrt();
        if len < 1.0e-3 || hop <= 0.0 {
            return [0.0; 2];
        }
        [d[0] / len * hop * SETTLE_ROLL, d[1] / len * hop * SETTLE_ROLL]
    }

    /// Where a dropped rock is `u` of the way through settling after it struck the floor (0 to
    /// 1): one hop `hop` high, rolling on the way it came and turning as it goes, slower and
    /// slower, to rest on its resting place (the path's end).
    fn settle(&self, u: f32, hop: f32) -> (Vec3, f32, f32) {
        match *self {
            Path::Drop { to, yaw, tilt, .. } => {
                let short = self.roll(hop);
                let left = (1.0 - u) * (1.0 - u);
                let up = hop * 4.0 * u * (1.0 - u);
                let pos = [to[0] - short[0] * left, to[1] - short[1] * left, to[2] + up];
                (pos, yaw + SETTLE_TURN * left, tilt.1 + 0.6 * up)
            }
            Path::Beam { .. } => beam_pose(self, 1.0),
        }
    }
}

/// A beam's pose `t` of the way down: the post's foot (its origin) at the hinge, the post along
/// `yaw` tipped from lying level (a tilt of a quarter turn from upright) until its far end strikes
/// the floor; then, coming down flat, its foot drops after it, the far end where it struck.
fn beam_pose(path: &Path, t: f32) -> (Vec3, f32, f32) {
    let Path::Beam { hinge, yaw, floor, flat } = *path else { return ([0.0; 3], 0.0, 0.0) };
    use std::f32::consts::FRAC_PI_2;
    let len = BEAM_LEN * BEAM_SCALE;
    let rest = floor + BEAM_HALF * BEAM_SCALE;
    let d = [yaw.cos(), yaw.sin()];
    // The tilt that puts the far end on the floor, swung from the hinge.
    let struck = (-((hinge[2] - rest) / len).clamp(-1.0, 1.0)).acos();
    let swing = if flat { BEAM_SWING } else { 1.0 };
    if t <= swing {
        let u = (t / swing).powi(2);
        return (hinge, yaw, FRAC_PI_2 + (struck - FRAC_PI_2) * u);
    }
    let end = [hinge[0] + d[0] * len * struck.sin(), hinge[1] + d[1] * len * struck.sin(), rest];
    let u = ((t - swing) / (1.0 - swing)).powi(2);
    let z = hinge[2] + (rest - hinge[2]) * u;
    let reach = (len * len - (z - end[2]).powi(2)).max(0.0).sqrt();
    let foot = [end[0] - d[0] * reach, end[1] - d[1] * reach, z];
    let tilt = ((end[2] - z) / len).clamp(-1.0, 1.0).acos();
    (foot, yaw, tilt)
}

/// The point `d` yards along a route and the way the route runs there.
fn route_pose(route: &[Vec3], along: &[f32], d: f32) -> (Vec3, f32) {
    let a = route_point(route, along, (d - 0.5).max(0.0));
    let b = route_point(route, along, d + 0.5);
    let yaw = (b[1] - a[1]).atan2(b[0] - a[0]);
    (route_point(route, along, d), yaw)
}

/// How far `p` is from the stretch `a` to `b`, on the flat.
fn seg_far(p: [f32; 2], a: [f32; 2], b: [f32; 2]) -> f32 {
    let (u, v) = ([b[0] - a[0], b[1] - a[1]], [p[0] - a[0], p[1] - a[1]]);
    let t = ((u[0] * v[0] + u[1] * v[1]) / (u[0] * u[0] + u[1] * u[1]).max(1.0e-6)).clamp(0.0, 1.0);
    let d = [v[0] - u[0] * t, v[1] - u[1] * t];
    (d[0] * d[0] + d[1] * d[1]).sqrt()
}

fn dist3(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The point `d` yards along a route (its points, and how far along each is).
fn route_point(route: &[Vec3], along: &[f32], d: f32) -> Vec3 {
    for i in 0..route.len().saturating_sub(1) {
        if d <= along[i + 1] || i + 2 == route.len() {
            let len = (along[i + 1] - along[i]).max(1.0e-3);
            let t = ((d - along[i]) / len).clamp(0.0, 1.0);
            let (a, b) = (route[i], route[i + 1]);
            return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        }
    }
    route[0]
}

/// How far along a route a spot is (its nearest point on the route, by height too, so a floor
/// above isn't taken for one below), and how far off the route it stands.
fn route_progress(route: &[Vec3], along: &[f32], pos: Vec3) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    for i in 0..route.len().saturating_sub(1) {
        let (a, b) = (route[i], route[i + 1]);
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ap = [pos[0] - a[0], pos[1] - a[1], pos[2] - a[2]];
        let len2 = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]).max(1.0e-6);
        let t = ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / len2).clamp(0.0, 1.0);
        let q = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
        let off = dist3(pos, q);
        let at = along[i] + t * (along[i + 1] - along[i]);
        if best.is_none_or(|(_, o)| off < o) {
            best = Some((at, off));
        }
    }
    best
}

/// Where the stretch from `p` to `q` crosses the stretch from `a` to `b` on the map: how far
/// along each (0 to 1).
fn seg_cross(p: [f32; 2], q: [f32; 2], a: [f32; 2], b: [f32; 2]) -> Option<(f32, f32)> {
    let r = [q[0] - p[0], q[1] - p[1]];
    let s = [b[0] - a[0], b[1] - a[1]];
    let den = r[0] * s[1] - r[1] * s[0];
    if den.abs() < 1.0e-6 {
        return None;
    }
    let ap = [a[0] - p[0], a[1] - p[1]];
    let t = (ap[0] * s[1] - ap[1] * s[0]) / den;
    let u = (ap[0] * r[1] - ap[1] * r[0]) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some((t, u))
}

/// Where a route first crosses the line from `a` to `b`: how far along it, and the spot.
fn route_crossing(route: &[Vec3], along: &[f32], a: [f32; 2], b: [f32; 2]) -> Option<(f32, Vec3)> {
    for i in 0..route.len().saturating_sub(1) {
        let (p, q) = (route[i], route[i + 1]);
        if let Some((t, _)) = seg_cross([p[0], p[1]], [q[0], q[1]], a, b) {
            let at = [p[0] + (q[0] - p[0]) * t, p[1] + (q[1] - p[1]) * t, p[2] + (q[2] - p[2]) * t];
            return Some((along[i] + t * (along[i + 1] - along[i]), at));
        }
    }
    None
}

/// A collapse's sections laid on its route, in the order the route meets their faces, and the
/// ones left out (by their place in the content: a face the route doesn't cross, or one within a
/// yard of the route's start or of another's). With none, the whole route is one section, its
/// face square across the route a little short of its end.
fn collapse_sections(route: &[Vec3], along: &[f32], defs: &[crate::content::CollapseSectionDef]) -> (Vec<Section>, Vec<usize>) {
    let total = *along.last().unwrap();
    let mut found: Vec<(f32, Vec3, [f32; 2], [f32; 2], f32, f32, bool)> = Vec::new();
    let mut missed = Vec::new();
    for (k, d) in defs.iter().enumerate() {
        match route_crossing(route, along, d.face[0], d.face[1]) {
            Some((line, at)) if line >= 1.0 && !found.iter().any(|f| (f.0 - line).abs() < 1.0) => {
                found.push((line, at, d.face[0], d.face[1], d.height.max(1.0), d.reach.max(1.0), true));
            }
            _ => missed.push(k),
        }
    }
    if found.is_empty() {
        let line = (total - FACE_SHORT).max(total * 0.5);
        let (at, yaw) = route_pose(route, along, line);
        let side = [-yaw.sin() * FACE_HALF, yaw.cos() * FACE_HALF];
        found.push((line, at, [at[0] + side[0], at[1] + side[1]], [at[0] - side[0], at[1] - side[1]], crate::content::collapse_height(), crate::content::collapse_reach(), false));
    }
    found.sort_by(|x, y| x.0.total_cmp(&y.0));
    let mut sections = Vec::new();
    let mut from = 0.0;
    for (line, at, a, b, height, reach, authored) in found {
        // Which side of the face the route goes on to.
        let (_, yaw) = route_pose(route, along, line);
        let u = [b[0] - a[0], b[1] - a[1]];
        let ahead = if u[0] * yaw.sin() - u[1] * yaw.cos() >= 0.0 { 1.0 } else { -1.0 };
        let mut stretch = vec![route_point(route, along, from)];
        let mut stretch_along = vec![from];
        for (p, d) in route.iter().zip(along) {
            if *d > from + 0.01 && *d < line - 0.01 {
                stretch.push(*p);
                stretch_along.push(*d);
            }
        }
        stretch.push(at);
        stretch_along.push(line);
        sections.push(Section { from, line, at, a, b, height, reach, ahead, authored, stretch, stretch_along });
        from = line;
    }
    (sections, missed)
}
