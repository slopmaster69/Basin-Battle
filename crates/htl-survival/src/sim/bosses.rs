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
//!   its place: a piece that moves has none).
//! - The 45% ring (`rocks_round`) brings a beam down over each of its falls, a ring of timbers
//!   round the boss.
//! - Everything that came down stays until the fight is lost or the dungeon resets: at most
//!   `RUBBLE_MOST` loose pieces (the oldest going first), and every face, whatever the count.
//!   They're the props the finished ruin is built of (`htl-server`'s `aftermath.rs`): the dark
//!   Burning Steppes rocks and a dig site's timber. The loose pieces have no hull.

use std::collections::VecDeque;

use super::mobs::MobState;
use super::*;
use crate::content::BurnDef;

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
pub(crate) const BEAM: u32 = 1508;
const ROCK_BOTTOM: f32 = 0.41;
const BOULDER_BOTTOM: f32 = -0.10;
/// A rock is sunk this much of its size below its lowest point, as the ruin's are, so its open
/// underside stays under the floor.
const EMBED: f32 = 0.4;
const BEAM_LEN: f32 = 23.87;
const BEAM_HALF: f32 = 1.2;
/// Sizes: a fall's rock (about 1.7 by 2.3 yd, a yard high; a heavy fall's a little more), the
/// wave's rocks (4 to 6 yd across, 2 to 3 high) and its boulders, and a beam (7 yd long).
const CHUNK_SCALE: f32 = 0.3;
const CHUNK_HEAVY: f32 = 0.36;
const ROW_SCALES: [f32; 4] = [0.8, 0.95, 0.7, 1.05];
const ROW_BOULDER: f32 = 0.27;
pub(crate) const BEAM_SCALE: f32 = 0.3;
/// Rock falls from this high over the floor (lower where a floor lies overhead, never under
/// `DROP_LOW`), in the last `ROCK_SECS` before its blow lands.
const DROP_HIGH: f32 = 5.0;
const DROP_LOW: f32 = 2.0;
pub(crate) const ROCK_SECS: f32 = 0.45;
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
const ROW_SECS: f32 = 0.6;
const ROW_RING: f32 = 2.2;
const ROW_RINGS: usize = 6;
pub(crate) const BEAM_STEP: f32 = 14.0;
/// A face of rubble, of the broken rock (5.7 by 7.7 yd and 3.2 high at full size) sized by the
/// roof (`FACE_ROOF` yards of roof to a rock of full size): its long side along the face, one
/// every `FACE_PITCH` of its size from wall to wall, its near side `FACE_GAP` behind the line
/// (`FACE_BACK` of its size to its middle), a layer every `FACE_RISE` of its size up to the
/// roof (`FACE_LAYERS` at most), each layer `FACE_LEAN` farther back, the first landing
/// `FACE_FIRST` after the wave reaches the line and each `FACE_LAYER_SECS` after the one below.
/// The piece that fell is taken away `FACE_SWAP` after its solid twin shows (the client fades a
/// new object in over two seconds).
const FACE_ROOF: f32 = 8.0;
const FACE_PITCH: f32 = 5.8;
const FACE_GAP: f32 = 0.25;
const FACE_BACK: f32 = 2.62;
const FACE_RISE: f32 = 2.0;
const FACE_LAYERS: usize = 7;
const FACE_LEAN: f32 = 0.5;
const FACE_FIRST: f32 = 0.5;
const FACE_LAYER_SECS: f32 = 0.3;
const FACE_SWAP: f32 = 2.2;
/// The band across a surge's line: bright (not a blow's colour: it's where to go), this wide,
/// drawn anew every `BAND_EVERY` for `BAND_SECS` (each a pulse from the route out to the walls).
const BAND_COLOUR: &str = "FFE27A";
const BAND_WIDTH: f32 = 2.0;
const BAND_EVERY: f32 = 0.7;
const BAND_SECS: f32 = 1.4;
/// Dust from the roof over a section going: this many spots each dust beat, out to this far to
/// each side of the route.
const ROOF_DUST: usize = 5;
const ROOF_DUST_SIDE: f32 = 9.0;
/// The most loose pieces left lying (the oldest, farthest behind, goes first), and in the air
/// at once (a face's rocks beside).
pub(crate) const RUBBLE_MOST: usize = 400;
const FALLING_MOST: usize = 48;
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

#[derive(Debug, Clone, Default)]
pub(crate) struct Bosses {
    /// Bosses on the run, by creature.
    runs: BTreeMap<Id, Run>,
    /// Where a boss that ran began its fight, to be put back there once the fight is lost.
    began: BTreeMap<Id, Vec3>,
    /// Bosses whose heavy blows burn, and how.
    burning: BTreeMap<Id, BurnDef>,
    /// Burning ground.
    fires: Vec<Fire>,
    /// The place coming down behind the heroes, if a boss has brought it down.
    collapse: Option<Collapse>,
    /// The faces of rubble standing (the rule: nobody passes one).
    faces: Vec<Face>,
    /// Foes running from a collapse, and when each is gone.
    fleeing: Vec<(Id, f32)>,
    /// What the falling mine looks like.
    pub(crate) looks: Looks,
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
    /// A face's rocks as they fell, each taken away once its solid twin has shown: (when, whose,
    /// the prop).
    swaps: Vec<(f32, Id, Id)>,
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
    obj: Id,
}

impl Survival {
    /// A boss's phase has begun (`enrage`): its beats beyond the phase's own.
    pub(crate) fn begin_boss_beats(&mut self, mid: Id, phase: usize) {
        let Some((def, home)) = self.mobs.get(&mid).map(|m| (m.def, m.home)) else { return };
        let Some(e) = self.content.mobs[def].phases.get(phase).cloned() else { return };
        if let Some(b) = e.burn {
            self.bosses.burning.insert(mid, b);
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
        self.step_collapse_fled();
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
        // It roars it down (unless a blow of its own is winding up: that keeps its look).
        if !matches!(m.state, MobState::Windup { .. }) {
            self.events.push(Event::Emote { id: boss, emote: mobs::ROAR_EMOTE });
        }
        self.events.push(Event::Log(format!(
            "{name}'s collapse: section {} of {} goes, {from:.1} to {line:.1} yd, its wave in {:.1} s",
            c.section + 1,
            c.sections.len(),
            c.warn
        )));
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
                // (Nobody behind it from this very step.)
                c.catch_at = now;
                let name = self.content.mobs[self.mobs[&boss].def].name.clone();
                let (k, n) = (c.section + 1, c.sections.len());
                self.events.push(Event::Log(format!("{name}'s collapse: section {k} of {n} is down, its face at {line:.1} yd")));
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
            }
            Stage::Rest { until } => {
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
            self.rocks_fall(boss, at, FALL_RADIUS, FALL_WARN, damage, false);
        }
        if now >= c.dust_at {
            c.dust_at = now + DUST_EVERY;
            let at = route_point(&c.route, &c.along, c.front);
            self.events.push(Event::Missile { caster: boss, spell: DUST, target: Target::Point(at) });
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
        for (h, was, facing, max) in behind {
            // (Side by side, not on a spot somebody stands on.)
            let to = [0.0, 1.4, -1.4, 2.8, -2.8]
                .iter()
                .filter_map(|off| self.land.footing([clear[0] - yaw.sin() * off, clear[1] + yaw.cos() * off, clear[2]]))
                .find(|p| !c.behind(*p, 0.0) && !self.players.iter().any(|(o, q)| *o != h && q.health > 0 && dist2d(q.pos, *p) < 1.0))
                .unwrap_or(clear);
            self.set_down(h, to, facing);
            if !c.caught.insert(h) {
                continue;
            }
            let damage = if c.catch > 0.0 { ((max as f32) * c.catch).round().max(1.0) as u32 } else { 0 };
            let name = self.players[&h].name.clone();
            self.events.push(Event::Log(format!(
                "the collapse catches {name} at ({:.1}, {:.1}, {:.1}): {damage} of {max} health gone, thrown clear to ({:.1}, {:.1}, {:.1})",
                was[0], was[1], was[2], to[0], to[1], to[2]
            )));
            if damage > 0 {
                self.hurt_player_by(h, damage, boss, "the falling mine");
            }
            if c.down > 0.0 && self.players.get(&h).is_some_and(|p| p.health > 0) {
                let share = self.content.cc.player_share.max(0.05);
                self.knock_down(boss, h, c.down / share, "the falling mine");
            }
        }
        if let Some(pos) = self.mobs.get(&boss).map(|m| m.pos) {
            if c.behind(pos, BOSS_SLACK) {
                let at = route_point(&c.route, &c.along, (line + BOSS_CLEAR).min(total));
                self.set_back(boss, at, "behind its own collapse");
            }
        }
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
    /// moved them): stopped a step short of it, at once. (A move too long to be one of those is
    /// left to the catch.)
    pub(crate) fn hold_at_face(&mut self, id: Id, from: Vec3, to: Vec3) -> bool {
        if self.bosses.faces.is_empty() || dist2d(from, to) > 12.0 {
            return false;
        }
        let Some(back) = self.face_crossing(from, to) else { return false };
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
        self.bosses.collapse = None;
        self.collapse_cleared(boss);
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
        self.bosses.looks = Looks { off, ..Default::default() };
    }

    /// A blow on a boss whose collapse is still coming down: it can't be brought below its hold
    /// before the last section is down (the last of the fight is fought at its face).
    pub(crate) fn boss_hold(&mut self, mid: Id, damage: u32) -> u32 {
        let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == mid && !c.done && c.hold > 0.0) else { return damage };
        let Some(m) = self.mobs.get(&mid) else { return damage };
        let floor = (c.hold * m.max_health as f32).round() as u32;
        let most = m.health.saturating_sub(floor);
        if damage <= most {
            return damage;
        }
        if !c.held {
            let line = c.held_text.clone();
            self.bosses.collapse.as_mut().unwrap().held = true;
            self.yell(mid, line);
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
                self.events.push(Event::Emote { id, emote: mobs::ROAR_EMOTE });
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
        self.bosses.burning.remove(&id);
        self.collapse_cleared(id);
        if let Some(at) = self.bosses.began.remove(&id) {
            if let Some(m) = self.mobs.get_mut(&id) {
                m.home = at;
            }
            self.set_back(id, at, "boss put back where its fight began");
        }
    }

    /// A boss is down: its beats end (the ground it set burning burns out). What its collapse
    /// brought down stays, its faces too, until the dungeon resets.
    pub(crate) fn boss_gone(&mut self, id: Id) {
        self.bosses.runs.remove(&id);
        self.bosses.burning.remove(&id);
        self.bosses.began.remove(&id);
        if self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id) {
            self.bosses.collapse = None;
        }
    }

    /// A boss's melee blow leaves burning ground where a heated weapon lands.
    pub(crate) fn boss_blow_lands(&mut self, id: Id, pos: Vec3, facing: f32, range: f32, arc: f32, heavy: bool) {
        let now = self.clock;
        if heavy {
            if let Some(b) = self.bosses.burning.get(&id).cloned() {
                let at = if arc >= 300.0 { pos } else { [pos[0] + facing.cos() * range * 0.7, pos[1] + facing.sin() * range * 0.7, pos[2]] };
                let at = self.ground_near(at[0], at[1], at[2]);
                if b.look != 0 {
                    self.events.push(Event::Missile { caster: id, spell: b.look, target: Target::Point(at) });
                }
                let obj = self.alloc();
                self.objs.insert(obj, Obj { kind: ObjKind::Fire { on: 0 }, pos: at, facing: 0.0 });
                self.events.push(Event::ObjSpawned { id: obj });
                self.bosses.fires.push(Fire { at, radius: b.radius, damage: b.damage, until: now + b.secs, bite_at: now + BURN_TICK * 0.5, owner: id, obj });
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
        // A short lodge (the ordinary slam) must not shorten a longer normal recovery either.
        if let Some(Mob { state: MobState::Recover { until, .. }, .. }) = self.mobs.get_mut(&id) {
            *until = until.max(recovery);
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
        for f in self.bosses.fires.iter_mut() {
            if now >= f.until {
                out.push(f.obj);
                continue;
            }
            if now >= f.bite_at {
                f.bite_at = now + BURN_TICK;
                bites.push((f.at, f.radius, f.damage, f.owner));
            }
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

    /// This share of the boss's own blow (as hard as its fight has grown).
    fn boss_blow(&self, id: Id, share: f32) -> u32 {
        let d = self.mobs.get(&id).map_or(0, |m| self.content.mobs[m.def].attack.damage);
        (d as f32 * share * self.foe_blow_share(id)).round().max(1.0) as u32
    }

    /// Rocks come down on `at`: dust there first (`warn` seconds), then the fall, which strikes
    /// whoever is in it (heavy: thrown down).
    fn rocks_fall(&mut self, owner: Id, at: Vec3, radius: f32, warn: f32, damage: u32, heavy: bool) {
        let now = self.clock;
        let at = self.ground_near(at[0], at[1], at[2]);
        self.events.push(Event::Missile { caster: owner, spell: DUST, target: Target::Point(at) });
        let mark = self.mark_ground(at, radius, warn, DUST);
        self.missiles.push(combat::Missile { owner, from: at, at, radius, damage, blockable: false, heavy, land_at: now + warn, mark, look: DUST });
        let near: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= radius + 2.0).map(|(h, _)| *h).collect();
        for v in near {
            self.events.push(Event::Danger { player: v, secs: warn, from: None });
        }
        // How it looks: its ring on the floor till it lands, and a rock dropping onto it.
        if !self.bosses.looks.off {
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: radius, arc: 360.0, width: 0.0, secs: warn, colour: mobs::HAZARD_COLOUR.into() });
            let scale = if heavy { CHUNK_HEAVY } else { CHUNK_SCALE };
            self.rock_drops(owner, at, ROCK, scale, now + warn, radius, 0);
        }
    }

    // --- how the falling mine looks --------------------------------------------------------------

    /// A piece of the falling mine (on its way down or lying where it fell), and its tilt: the
    /// server draws it at once with no hull, moved in place as it falls.
    pub fn collapse_piece_tilt(&self, id: Id) -> Option<f32> {
        self.bosses.looks.tilt.get(&id).copied()
    }

    /// The loose pieces lying where they fell (for tests and the log).
    pub fn collapse_rubble(&self) -> Vec<Id> {
        self.bosses.looks.rubble.iter().map(|(_, id)| *id).collect()
    }

    /// The rocks of the faces, landed and solid (for tests and the log).
    pub fn collapse_faces(&self) -> Vec<Id> {
        self.bosses.looks.face_pieces.iter().map(|(_, id)| *id).collect()
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
        let bottom = if display == BOULDER { BOULDER_BOTTOM } else { ROCK_BOTTOM };
        let to = [at[0], at[1], at[2] - (bottom + EMBED) * scale];
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
        let every = spots.len().div_ceil(ROW_RINGS).max(1);
        for (j, spot) in spots.into_iter().enumerate() {
            // (Nothing past the line: the face stands there.)
            if s.authored && s.side(spot) > -1.0 {
                continue;
            }
            let k = self.look_seq();
            let jit = [((k % 7) as f32 - 3.0) * 0.25, ((k % 5) as f32 - 2.0) * 0.3];
            let at = self.floor_beside(spot, [spot[0] + jit[0], spot[1] + jit[1]]);
            let land = now + ROW_SECS + (k % 4) as f32 * 0.08;
            if j % every == 0 {
                self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: ROW_RING, arc: 360.0, width: 0.0, secs: land - now, colour: mobs::HAZARD_COLOUR.into() });
            }
            let (display, scale) = if k % 5 == 4 { (BOULDER, ROW_BOULDER) } else { (ROCK, ROW_SCALES[k as usize % 4]) };
            let crash = if k % 3 == 0 { SOUND_CRASH } else { 0 };
            self.rock_drops(c.boss, at, display, scale, land, 0.0, crash);
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
                let to = [xy[0], xy[1], floor[2] + rise * layer as f32 - (ROCK_BOTTOM + EMBED) * scale];
                let from = [to[0], to[1], (s.at[2] + s.height + 1.0).max(to[2] + 3.0)];
                let yaw = facing + ((k % 5) as f32 - 2.0) * 0.06;
                let land = now + FACE_FIRST + FACE_LAYER_SECS * layer as f32 + (k % 3) as f32 * 0.05;
                let start = (land - ROCK_SECS).max(now);
                self.bosses.looks.falling.push(Falling {
                    owner,
                    id: None,
                    shows: start,
                    start,
                    land,
                    display: ROCK,
                    scale,
                    path: Path::Drop { from, to, yaw, tilt: (0.3, 0.0) },
                    at: floor,
                    ring: 0.0,
                    crack: 0,
                    crash: if k % 4 == 0 { SOUND_CRASH } else { 0 },
                    landed: false,
                    face: true,
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
        let (line, reach) = (c.sections[k].line, c.sections[k].reach.min(ROOF_DUST_SIDE));
        for _ in 0..ROOF_DUST {
            let n = self.look_seq();
            let d = c.front + (line - c.front) * ((n % 11) as f32 + 0.5) / 11.0;
            let off = ((n % 7) as f32 / 3.0 - 1.0) * reach;
            let (p, yaw) = route_pose(&c.route, &c.along, d);
            let at = self.floor_beside(p, [p[0] - yaw.sin() * off, p[1] + yaw.cos() * off]);
            self.point_look(boss, at, DUST);
        }
        let front = route_point(&c.route, &c.along, c.front);
        if c.stage == Stage::Wave {
            for ahead in [1.5, 4.5] {
                let at = route_point(&c.route, &c.along, (c.front + ahead).min(line));
                self.point_look(boss, at, DUST_ROLL);
            }
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
        let objs = &self.objs;
        let l = &mut self.bosses.looks;
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
        let mut landed = Vec::new();
        for k in 0..self.bosses.looks.falling.len() {
            let f = self.bosses.looks.falling[k].clone();
            if now < f.shows {
                continue;
            }
            let t = if f.land > f.start { ((now - f.start) / (f.land - f.start)).clamp(0.0, 1.0) } else { 1.0 };
            let (pos, facing, tilt) = f.path.pose(t);
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
                self.bosses.looks.falling[k].landed = true;
                landed.push(k);
            }
        }
        for k in landed {
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
            if f.face {
                // Solid from here: a twin stands where it landed, drawn with its hull (a piece
                // that moves has none), and the piece that fell goes once the twin has shown.
                let (pos, facing, tilt) = f.path.pose(1.0);
                let twin = self.alloc();
                self.objs.insert(twin, Obj { kind: ObjKind::Prop { display: f.display, scale: f.scale }, pos, facing });
                let l = &mut self.bosses.looks;
                l.tilt.insert(twin, tilt);
                l.solid.insert(twin);
                l.face_pieces.push((f.owner, twin));
                l.swaps.push((now + FACE_SWAP, f.owner, id));
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
        let mut gone: Vec<Id> = l.rubble.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id).collect();
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

impl Path {
    /// Where its prop stands `t` of the way down (0 to 1, falling faster as it goes): its
    /// position, facing and tilt (about the model's own Y after its facing, the drawing's).
    fn pose(&self, t: f32) -> (Vec3, f32, f32) {
        match *self {
            Path::Drop { from, to, yaw, tilt } => {
                let u = t * t;
                let pos = [from[0] + (to[0] - from[0]) * u, from[1] + (to[1] - from[1]) * u, from[2] + (to[2] - from[2]) * u];
                (pos, yaw + (1.0 - u) * 1.3, tilt.0 + (tilt.1 - tilt.0) * u)
            }
            Path::Beam { .. } => beam_pose(self, t),
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
