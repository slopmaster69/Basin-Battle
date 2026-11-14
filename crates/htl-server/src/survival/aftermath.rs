//! The finished Deadmines, inspected independently of a boss fight. This is a temporary
//! presentation of the world: the rules, their clock, hero positions and saves stay intact.
//! Every visible ruin is a native, solid game object. The display-less marker asks client
//! patch 0087 to remove the ship's render groups AND colliders for this map only.
//!
//! The ruins are fitted to the installed map when the preview starts (`aftermath_piles.rs`): each
//! pile slumps against the rock it fell from, under the broken roof, and every piece rests on the
//! floor or on rock without passing through the mine's walls or its own doodads.

use std::collections::BTreeMap;

use htl_map::deadmines::aftermath::Survey;
use htl_map::solids::Solids;
use htl_survival::Land;

use super::*;
use crate::packets::{Create, Placement};
use shapes::{matrix, unturn, Obb, V3};

#[path = "aftermath_air.rs"]
mod air;
#[path = "aftermath_doodads.rs"]
pub(crate) mod doodads;
#[path = "aftermath_layout.rs"]
mod layout;
#[path = "aftermath_piles.rs"]
mod piles;
#[path = "aftermath_shapes.rs"]
mod shapes;
#[path = "aftermath_wreck.rs"]
mod wreck;
pub(crate) use wreck::StagedWreck;
#[cfg(test)]
#[path = "aftermath_tests.rs"]
mod tests;

pub(super) const MARKER_ENTRY: u32 = 500_100;
pub(super) const MARKER_GUID: u64 = 0xF110_07A184_FFFFFF;
const FIRST_ENTRY: u32 = MARKER_ENTRY + 1;
const INERT: u32 = 0x30;
const GENERIC: u32 = 5;
const BATCH: usize = 16;
/// A cameraman is brought to the hero it films once this far off (yd): sooner than in play, so
/// its ground is loaded round each spot the hero is set down at.
const WATCH_NEAR: f32 = 12.0;

pub(crate) struct Aftermath {
    props: Vec<Prop>,
    /// Inspectors move only here, never in the saved simulation.
    positions: BTreeMap<ConnId, ([f32; 3], f32)>,
    missing: Vec<&'static str>,
    /// Dust settling and timber creaking (`aftermath_air.rs`).
    air: air::Air,
    /// The copies of the mine's own doodads that client patch 0091 hides (`aftermath_doodads.rs`),
    /// sent only to a client of its round, after the ruins (their GUIDs number on from them).
    knocked: Vec<Prop>,
}

pub(super) struct Region {
    key: &'static str,
    name: &'static str,
    view: [f32; 3],
    facing: f32,
}

/// What each site's wreckage is made of besides rock and timber.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Dressing { Tunnel, Timber, Minecart, Machine, Cannon, Cargo, Shore }

/// How a site's rock came down.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum Fall {
    /// A cave-in slumped against the nearest wall, under the broken roof it fell from.
    Slump,
    /// A side passage or doorway filled to its lintel.
    Seal,
    /// The Foundry's back door: the lintel down on a pile against one jamb, the way through kept.
    Lintel,
    /// Loose rock and timber spread across the floor, the roof whole.
    Spill,
}

pub(super) struct Site {
    region: usize,
    name: &'static str,
    at: [f32; 3],
    yaw: f32,
    scale: f32,
    fall: Fall,
    dressing: Dressing,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Prop {
    pub(super) region: usize,
    pub(super) site: &'static str,
    pub(super) model: usize,
    pub(super) pos: [f32; 3],
    pub(super) scale: f32,
    pub(super) rotation: [f32; 4],
    pub(super) facing: f32,
}

/// Bounds measured from the installed native meshes (`godisp <Data> <display>`).
pub(super) struct Model {
    pub(super) display: u32,
    name: &'static str,
    pub(super) min: [f32; 3],
    pub(super) max: [f32; 3],
}

/// The one rock: Burning Steppes boulder 05, dark ash rock (the mine rubble's own texture) on an
/// open skirt. Every size in a pile is this mesh, turned and scaled; the tall 5737 boulder read
/// as a stalagmite and is gone (views 8, 10, 12).
pub(super) const ROCK: usize = 0;
/// The gravel a pile spreads across the floor: the same dark rock, a hand or two across. (The
/// grave and dirt mounds tried first read as green grass on film, and one glowed pale.)
pub(super) const SKIRT: usize = 1;
/// Scree: a little larger gravel.
pub(super) const SCREE: usize = 2;
/// The dwarven excavation pole, an iron-shod log: snapped beams and broken posts.
pub(super) const BEAM: usize = 3;
/// The excavation barrier plank: broken boards. (The Dark Iron planks and woodpile rendered
/// bright gold in the mine's light, views 3 and 5.)
pub(super) const BOARD: usize = 4;
/// The Deadmines' own small lumber pile, spilled.
pub(super) const LUMBER: usize = 5;
pub(super) const CART: usize = 6;
pub(super) const BARROW: usize = 7;
pub(super) const BARREL: usize = 8;
pub(super) const CRATE: usize = 9;
/// The Deadmines' cargo boxes (the model of its neat stacks), toppled.
pub(super) const CARGO: usize = 10;
pub(super) const MACHINE: usize = 11;
pub(super) const CANNON: usize = 12;
pub(super) const BOAT: usize = 13;
/// Not placed (its pale cloth glowed green by a lantern on film); kept so the models' entries
/// stay where clients cached them.
#[allow(dead_code)]
pub(super) const SACK: usize = 14;
/// The Deadmines' own anchor.
pub(super) const ANCHOR: usize = 15;
/// A barrel riding low in the water, as Azeroth's floating debris has it.
pub(super) const DRIFT_BARREL: usize = 16;
pub(super) const DRIFTWOOD: usize = 17;
/// The floating stores of a sunken ship (crates, barrels, planks), the shipwreck fishing pool's.
pub(super) const FLOTSAM: usize = 18;
/// A plain wooden pole (Stormwind's flagpole, no flag): the wrecked ship's mast and its yard.
pub(super) const MAST: usize = 19;
// The mine's own doodads, set down where the collapse left them (`aftermath_doodads.rs`).
/// The mine's hanging lantern (patch 0091's own display).
pub(super) const LANTERN: usize = 20;
/// The lantern the mine stands on its floors. Not placed (its glass read lit lying on the rock on
/// film); kept so the entries after it stay where clients cached them.
#[allow(dead_code)]
pub(super) const FLOOR_LANTERN: usize = 21;
/// The mine's ore cart (patch 0091's own display).
pub(super) const MINE_CART: usize = 22;
/// The mine's netted cargo (patch 0091's own display).
pub(super) const NET_CARGO: usize = 23;
pub(super) const KEG: usize = 24;
pub(super) const CASK: usize = 25;
pub(super) const BUCKET: usize = 26;
/// The mine's large lumber pile.
pub(super) const LUMBER_PILE: usize = 27;

pub(super) const MODELS: &[Model] = &[
    Model { display: 5741, name: "Fallen Rock", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 5741, name: "Gravel", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 5741, name: "Scree", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 1508, name: "Snapped Beam", min: [-1.20,-1.24,-5.77], max: [1.15,1.45,23.87] },
    Model { display: 4651, name: "Broken Board", min: [-0.28,-5.57,0.0], max: [0.22,5.82,2.29] },
    Model { display: 1108, name: "Spilled Lumber", min: [-1.68,-0.65,0.0], max: [1.68,0.66,0.56] },
    Model { display: 1887, name: "Wrecked Mine Cart", min: [-0.82,-1.06,-0.10], max: [1.25,1.05,1.60] },
    Model { display: 215, name: "Overturned Barrow", min: [-1.73,-0.79,-0.01], max: [1.16,0.80,1.23] },
    Model { display: 9, name: "Broken Barrel", min: [-1.23,-0.58,-0.04], max: [0.43,0.82,1.02] },
    Model { display: 3791, name: "Broken Crate", min: [-0.89,-1.05,-0.02], max: [1.25,0.85,1.31] },
    Model { display: 36, name: "Toppled Cargo", min: [-1.28,-1.14,-0.02], max: [1.29,1.12,1.76] },
    Model { display: 2029, name: "Toppled Machinery", min: [-1.30,-1.77,-0.02], max: [1.30,1.77,2.11] },
    Model { display: 245, name: "Overturned Cannon", min: [-4.72,-2.24,-0.01], max: [5.48,2.23,4.13] },
    Model { display: 223, name: "Shattered Boat", min: [-6.71,-1.77,-1.96], max: [2.79,2.24,2.32] },
    Model { display: 5511, name: "Split Sack", min: [-0.36,-0.43,0.0], max: [0.31,0.54,0.27] },
    Model { display: 4011, name: "Juggernaut's Anchor", min: [-0.11,-2.19,-0.05], max: [2.57,2.97,6.45] },
    Model { display: 6366, name: "Floating Barrel", min: [-0.55,-0.47,-0.63], max: [0.55,0.53,0.55] },
    Model { display: 627, name: "Driftwood", min: [-1.56,-1.78,-0.33], max: [1.26,2.24,1.18] },
    Model { display: 6434, name: "Floating Wreckage", min: [-2.50,-2.76,-3.08], max: [1.91,2.71,0.43] },
    Model { display: 625, name: "Broken Mast", min: [-0.55,-0.57,-0.08], max: [0.47,0.58,9.90] },
    Model { display: 60005, name: "Crooked Lantern", min: [-0.35,-0.35,0.0], max: [0.36,0.36,1.79] },
    Model { display: 6038, name: "Fallen Lantern", min: [-0.35,-0.35,0.0], max: [0.37,0.36,1.37] },
    Model { display: 60006, name: "Tipped Ore Cart", min: [-1.02,-0.77,-0.01], max: [1.03,0.75,1.48] },
    Model { display: 60007, name: "Toppled Netted Cargo", min: [-1.82,-1.99,-0.26], max: [1.92,2.03,1.48] },
    Model { display: 436, name: "Rolled Powder Keg", min: [-0.24,-0.25,-0.01], max: [0.24,0.31,0.59] },
    Model { display: 288, name: "Rolled Barrel", min: [-0.43,-0.47,0.0], max: [0.43,0.52,0.99] },
    Model { display: 2891, name: "Kicked Bucket", min: [-0.29,-0.39,0.02], max: [0.42,0.33,0.74] },
    Model { display: 1109, name: "Spilled Lumber Pile", min: [-1.68,-0.91,0.0], max: [1.70,0.86,1.67] },
];

/// Rock and soil: the masses a pile is made of. They may run into the mine's walls (a cave-in
/// enters the rock it came from) and into each other; wood and iron may not.
pub(super) fn rocky(model: usize) -> bool { matches!(model, ROCK | SKIRT | SCREE) }

/// The lowest point of a model's turned box, relative to its pivot (unscaled).
pub(super) fn bottom(model: usize, q: [f32; 4]) -> f32 {
    let m = &MODELS[model];
    let row = matrix(q)[2];
    (0..8).map(|i| (0..3).map(|a| row[a] * if i & (1 << a) == 0 { m.min[a] } else { m.max[a] }).sum::<f32>())
        .fold(f32::INFINITY, f32::min)
}

/// Where the fallen rock's open skirt meets the floor (its rim, model space), and the mounds':
/// sampled round each mesh's footprint when the installed meshes aren't at hand.
fn rim_samples(model: usize) -> Vec<V3> {
    let _ = model;
    let (cx, cy, rx, ry, z) = (-0.15, 0.2, 2.75, 3.85, 0.41);
    (0..12).map(|k| { let a = k as f32 * std::f32::consts::PI / 6.0; [cx + rx * 0.97 * a.cos(), cy + ry * 0.97 * a.sin(), z] }).collect()
}

/// The fallen rock's top over its own (x, y) without the installed mesh: a slumped dome on an open
/// skirt fitted to it (peak 3.6 near (-0.15, 0.2), the rim at 0.41 on an ellipse 2.75 by 3.85).
fn rock_top(x: f32, y: f32) -> Option<f32> {
    let (rx, ry) = ((x + 0.15) / 2.75, (y - 0.2) / 3.85);
    let r2 = rx*rx + ry*ry;
    (r2 < 1.0).then(|| 0.41 + 3.15 * (1.0 - r2).powf(0.4))
}

/// The highest point of a mesh straight over (x, y).
pub(super) fn top_of(tris: &[[V3; 3]], x: f32, y: f32) -> Option<f32> {
    let mut top: Option<f32> = None;
    for t in tris {
        let (a, b, c) = (t[0], t[1], t[2]);
        let det = (b[1]-c[1])*(a[0]-c[0]) + (c[0]-b[0])*(a[1]-c[1]);
        if det.abs() < 1e-9 { continue; }
        let l1 = ((b[1]-c[1])*(x-c[0]) + (c[0]-b[0])*(y-c[1])) / det;
        let l2 = ((c[1]-a[1])*(x-c[0]) + (a[0]-c[0])*(y-c[1])) / det;
        let l3 = 1.0 - l1 - l2;
        if l1 >= -1e-4 && l2 >= -1e-4 && l3 >= -1e-4 {
            let z = l1*a[2] + l2*b[2] + l3*c[2];
            top = Some(top.map_or(z, |t| t.max(z)));
        }
    }
    top
}

impl Prop {
    pub(super) fn obb(&self, shrink: f32) -> Obb {
        let m = &MODELS[self.model];
        Obb::new(m.min, m.max, self.pos, self.scale, self.rotation, shrink)
    }

    /// A world point in the model's own space (unscaled).
    pub(super) fn local(&self, p: V3) -> V3 {
        shapes::mul(unturn(&matrix(self.rotation), shapes::sub(p, self.pos)), 1.0 / self.scale)
    }

    pub(super) fn world(&self, v: V3) -> V3 {
        shapes::add(self.pos, shapes::turn(&matrix(self.rotation), shapes::mul(v, self.scale)))
    }

    /// Turned over and hung from the roof.
    pub(super) fn hangs(&self) -> bool { matrix(self.rotation)[2][2] < -0.9 }
}

/// A model as the installed data has it: its triangles, its points and its skirt's rim.
struct Shape {
    tris: Vec<[V3; 3]>,
    points: Vec<V3>,
    rim: Vec<V3>,
}

/// What the ruins are fitted to: the collapsed map's floors and, from the installed data, the rock
/// over and beside them, the doodads the mine already holds and the ruins' own meshes. The authored
/// tests' flat world has floors and the roof they give it, and measures the models by their boxes.
pub(crate) struct Ground {
    pub(crate) land: Box<dyn Land>,
    solids: Option<Solids>,
    /// The mine's own doodads' boxes (cobwebs left out: a rock may cover a web).
    natives: Vec<(String, Obb)>,
    shapes: Vec<Shape>,
    /// The walk from the mine's entrance to the cove, every half yard: kept open.
    route: Vec<V3>,
    /// The mine's own doodads this ground lacks (client patch 0091's, an open survey's), where
    /// they stood.
    hidden: Vec<htl_map::deadmines::aftermath::Standing>,
    /// The same mine as a client of patch 0091's round has it, those doodads gone: what their
    /// copies are fitted to (the piles are fitted to this one, which older clients show).
    pub(crate) open: Option<Box<Ground>>,
}

impl Ground {
    #[cfg(test)]
    pub(crate) fn with_solids(land: Box<dyn Land>, solids: Solids) -> Self {
        Self { land, solids: Some(solids), natives: Vec::new(), shapes: Vec::new(), route: Vec::new(), hidden: Vec::new(), open: None }
    }

    /// The same, lacking these of the mine's own doodads (an open ground's stand-in).
    #[cfg(test)]
    pub(crate) fn lacking(mut self, hidden: Vec<htl_map::deadmines::aftermath::Standing>) -> Self {
        self.hidden = hidden;
        self
    }

    /// The surveyed map, and each of [`MODELS`]' triangles in its own space.
    pub(crate) fn surveyed(survey: Survey, meshes: Vec<Vec<[V3; 3]>>) -> Self {
        let natives = survey.standing.iter()
            .filter(|s| !s.model.to_ascii_lowercase().contains("web"))
            .filter_map(|s| s.bounds.map(|(lo, hi)| (s.model.clone(), Obb::native(s.origin, &s.axes, lo, hi, 0.1))))
            .collect();
        let shapes = meshes.into_iter().enumerate().map(|(model, tris)| {
            // Each corner once (a mesh repeats a corner for every face that meets there).
            let mut points: Vec<V3> = tris.iter().flatten().copied().collect();
            points.sort_by(|a, b| a.iter().zip(b).map(|(x, y)| x.total_cmp(y)).find(|o| o.is_ne()).unwrap_or(std::cmp::Ordering::Equal));
            points.dedup_by(|a, b| (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4));
            let rim = if rocky(model) {
                let low = points.iter().map(|p| p[2]).fold(f32::INFINITY, f32::min);
                points.iter().filter(|p| p[2] < low + 0.04).copied().collect()
            } else { Vec::new() };
            Shape { tris, points, rim }
        }).collect();
        let land = super::land::DeadminesLand::new(survey.loaded);
        let route = Self::walk(&land);
        Self { land: Box::new(land), solids: Some(survey.solids), natives, shapes, route, hidden: survey.hidden, open: None }
    }

    /// The mine's own doodads this ground lacks, where they stood.
    pub(super) fn hidden(&self) -> &[htl_map::deadmines::aftermath::Standing] { &self.hidden }

    /// The way from the entrance's view to the cove's, view to view, every half yard. (The landing
    /// is across the water now the ship is gone.)
    fn walk(land: &super::land::DeadminesLand) -> Vec<V3> {
        let mut out = Vec::new();
        for leg in layout::REGIONS[..7].windows(2) {
            let ends = [&leg[0], &leg[1]].map(|r| floor(land, r.view, false).map(|z| [r.view[0], r.view[1], z]));
            let [Some(a), Some(b)] = ends else { continue };
            for w in land.walk(a, b).unwrap_or_default().windows(2) {
                let steps = (shapes::len(shapes::sub(w[1], w[0])) / 0.5).ceil().max(1.0) as usize;
                out.extend((0..steps).map(|k| shapes::add(w[0], shapes::mul(shapes::sub(w[1], w[0]), k as f32 / steps as f32))));
            }
        }
        out
    }

    pub(super) fn route(&self) -> &[V3] { &self.route }

    /// A prop's points in the world: its mesh's, or its box's when the meshes aren't at hand.
    pub(super) fn points(&self, p: &Prop) -> Vec<V3> {
        match self.shape(p.model) {
            Some(s) => s.points.iter().map(|v| p.world(*v)).collect(),
            None => p.obb(0.0).samples(),
        }
    }

    fn shape(&self, model: usize) -> Option<&Shape> { self.shapes.get(model).filter(|s| !s.tris.is_empty()) }

    /// How far a line from `from` along `dir` (unit) runs before solid rock, within `max`.
    pub(super) fn ray(&self, from: V3, dir: V3, max: f32) -> Option<f32> {
        self.solids.as_ref()?.ray(from, dir, max)
    }

    /// The height of the roof over a floor point, if there is one within 40 yd.
    pub(super) fn roof(&self, at: V3) -> Option<f32> {
        self.ray([at[0], at[1], at[2] + 0.5], [0.0, 0.0, 1.0], 40.0).map(|d| at[2] + 0.5 + d)
    }

    pub(super) fn natives(&self) -> &[(String, Obb)] { &self.natives }

    /// The rim of a rock's or mound's open skirt, model space.
    pub(super) fn rim(&self, model: usize) -> Vec<V3> {
        self.shape(model).map_or_else(|| rim_samples(model), |s| s.rim.clone())
    }

    /// The lowest point of a model turned by `q`, relative to its pivot (unscaled).
    pub(super) fn low(&self, model: usize, q: [f32; 4]) -> f32 {
        let Some(s) = self.shape(model) else { return bottom(model, q) };
        let row = matrix(q)[2];
        s.points.iter().map(|p| shapes::dot(row, *p)).fold(f32::INFINITY, f32::min)
    }

    /// A rock's or mound's top over its own (x, y), model space.
    fn top_local(&self, model: usize, x: f32, y: f32) -> Option<f32> {
        match self.shape(model) {
            Some(s) => top_of(&s.tris, x, y),
            None => rock_top(x, y),
        }
    }

    /// Whether a world point lies inside a rock or mound (never, for wood and iron).
    pub(super) fn holds(&self, p: &Prop, at: V3) -> bool { self.holds_within(p, at, 0.0) }

    /// [`Ground::holds`], counting points within `margin` yards over its surface as inside.
    pub(super) fn holds_within(&self, p: &Prop, at: V3, margin: f32) -> bool {
        if !rocky(p.model) { return false; }
        let [x, y, z] = p.local(at);
        let floor = MODELS[p.model].min[2];
        let m = margin / p.scale;
        [(0.0, 0.0), (m, 0.0), (-m, 0.0), (0.0, m), (0.0, -m)].iter()
            .any(|(dx, dy)| self.top_local(p.model, x + dx, y + dy).is_some_and(|t| z >= floor - m && z <= t + m))
    }

    /// The height of a rock's or mound's upper surface over a world (x, y), where it stands
    /// upright (a rock hanging from the roof holds nothing up).
    pub(super) fn top_at(&self, p: &Prop, x: f32, y: f32) -> Option<f32> {
        if !rocky(p.model) || p.hangs() || matrix(p.rotation)[2][2] < 0.9 { return None; }
        // Straight down the world's vertical: on a leaning rock that line crosses its own space
        // aslant, so the point under (x, y) is found again at the height the surface is met.
        let mut z = p.pos[2];
        let mut top = None;
        for _ in 0..4 {
            let [lx, ly, _] = p.local([x, y, z]);
            let t = self.top_local(p.model, lx, ly)?;
            let w = p.world([lx, ly, t]);
            top = Some(w[2]);
            if (w[2] - z).abs() < 0.01 { break; }
            z = w[2];
        }
        top
    }
}

pub(super) fn floor(land: &dyn Land, at: [f32; 3], wet: bool) -> Option<f32> {
    let z = land.height_near(at[0], at[1], at[2]);
    // Never jump to the outdoor roof (~300 yd) or another Foundry floor. Blocked cells are
    // permitted for a cave-in's edge to enter a wall, but water is reserved for the cove.
    (z.is_finite() && (z-at[2]).abs() <= 3.0 && (wet || !land.water_near(at[0], at[1], z))).then_some(z)
}

/// Whether a rock passes through one of the mine's own doodads: part of it inside the rock and
/// part out. A doodad wholly inside is buried by the fall and can't be seen, unless it gives light
/// (a buried lantern would still light the rock round it).
/// `margin` grows the doodad's box for the "wholly inside" half of that (the builder leaves room
/// for the meshes' own corners; the check measures the box as it is).
pub(super) fn rock_through(model: &str, b: &Obb, margin: f32, inside: impl Fn(V3) -> bool) -> bool {
    let lit = ["lantern", "torch", "candle", "brazier", "lamp"].iter().any(|w| model.to_ascii_lowercase().contains(w));
    if !b.samples().iter().any(|s| inside(*s)) { return false; }
    let grown = Obb { h: b.h.map(|h| h * margin), ..*b };
    lit || !grown.samples().iter().all(|s| inside(*s))
}

/// The floor under a point, or over it where the point is buried just under a floor (a skirt's
/// rim, a board's end under a step): never the next floor down.
pub(super) fn floor_under(land: &dyn Land, v: V3) -> f32 {
    let below = land.height_near(v[0], v[1], v[2] + 0.5);
    if v[2] - below > 1.5 {
        let over = land.height_near(v[0], v[1], v[2] + 2.5);
        if over > v[2] - 0.3 && over < v[2] + 3.0 { return over; }
    }
    below
}

/// Test a 0.9 yd standing capsule against the actual oriented model box. The first native
/// check showed that a model's enclosing sphere cleared enormous clean disks around views,
/// even for a low plank or a tall rock whose solid body never reached the inspector.
pub(super) fn covers_view(p: &Prop) -> bool {
    let views = layout::REGIONS.iter().map(|r| (r.view, r.facing))
        .chain(layout::CAMERAS.iter().map(|(at, deg)| (*at, deg.to_radians())));
    // The footing, and the first yards it looks across; for timber, nothing round the footing or in
    // the first 9 yd at any height (boards and a post filled the workshop's picture, a pole stood
    // at the lens at Johnson's, planks lay and stood in the entrances' foregrounds; the piles a
    // camera is there to see stand 7 yd off and more, filling a passage); and the hero's own
    // camera, up behind them.
    let timber = matches!(p.model, BEAM | BOARD | LUMBER);
    views.into_iter().any(|(at, f)| {
        let ahead = |d: f32| [at[0] + d * f.cos(), at[1] + d * f.sin(), at[2]];
        covers(p, at, 1.2) || covers(p, ahead(2.0), 1.0) || covers(p, ahead(3.8), 0.7)
            || timber && [(0.0, 3.0), (2.0, 2.5), (3.8, 2.5), (5.2, 1.5), (6.8, 1.5), (8.5, 1.5)].iter()
                .any(|(d, r)| covers_at(p, ahead(*d), *r, &[0.15, 0.95, 1.75, 2.6, 3.4]))
            || covers_at(p, ahead(-2.5), 1.2, &[1.75, 2.6, 3.4]) || covers_at(p, ahead(-4.5), 1.2, &[2.6, 3.4])
    })
}

/// Where a region's view is seen from in its pairs: about the hero's head, looking the way they
/// face (their own camera is a little behind and above).
pub(super) fn view_eye(r: &Region) -> V3 {
    [r.view[0] - 1.0 * r.facing.cos(), r.view[1] - 1.0 * r.facing.sin(), r.view[2] + 1.9]
}

/// The picture's columns: 4 degrees each across 64 (32 each side of the way it looks).
pub(super) const COLUMNS: usize = 16;

/// How tall rock stands in a camera's picture: in each column of it (out to 45 yd), the most any
/// rock seen from the eye stands over the floor under it, as an angle (degrees). Rock behind the
/// camera, outside its picture or behind the mine's own walls counts nothing (the volume measure
/// counted those, and the entrance's pile still read smaller than the dome it replaced). Column 0
/// is the picture's right edge.
pub(super) fn picture_rock<'a>(eye: V3, facing: f32, rocks: impl Iterator<Item = &'a [V3]>, land: &dyn Land, solids: Option<&Solids>, natives: &[Obb]) -> [f32; COLUMNS] {
    let mut bins = [0.0f32; COLUMNS];
    // (The mine's own posts and beams in front hide what's behind them: the entrance's pile stood
    // half behind its left post.)
    let near: Vec<&Obb> = natives.iter().filter(|n| (n.c[0] - eye[0]).hypot(n.c[1] - eye[1]) < 47.0 && !n.contains(eye)).collect();
    for points in rocks {
        for v in points.iter().step_by(2) {
            let (dx, dy) = (v[0] - eye[0], v[1] - eye[1]);
            let h = dx.hypot(dy);
            if !(3.0..=45.0).contains(&h) { continue; }
            let off = ((dy.atan2(dx) - facing + 3.0 * std::f32::consts::PI).rem_euclid(2.0 * std::f32::consts::PI) - std::f32::consts::PI).to_degrees();
            if off.abs() >= 32.0 { continue; }
            if near.iter().any(|n| !n.contains(*v) && n.cuts(eye, *v)) { continue; }
            // (Nor anything of the mine's own between: a post, a beam, a wall.)
            let d = shapes::sub(*v, eye);
            if solids.is_some_and(|s| s.ray(eye, shapes::unit(d), (shapes::len(d) - 0.4).max(0.0)).is_some()) { continue; }
            let fl = floor_under(land, *v);
            let rise = ((v[2] - eye[2]).atan2(h) - (fl - eye[2]).atan2(h)).to_degrees();
            let b = ((off + 32.0) / 4.0) as usize;
            bins[b] = bins[b].max(rise);
        }
    }
    bins
}

/// Whether a prop's box comes within `radius` of a body standing at `at` (feet to head).
pub(super) fn covers(p: &Prop, at: V3, radius: f32) -> bool {
    covers_at(p, at, radius, &[0.15, 0.95, 1.75])
}

/// Whether a prop's box comes within `radius` of any of `heights` over `at`.
fn covers_at(p: &Prop, at: V3, radius: f32, heights: &[f32]) -> bool {
    let m = &MODELS[p.model];
    heights.iter().any(|height| {
        let local = p.local([at[0], at[1], at[2] + height]);
        let d2 = (0..3).map(|a| (local[a]-local[a].clamp(m.min[a], m.max[a])).powi(2)).sum::<f32>();
        d2 < (radius/p.scale).powi(2)
    })
}

/// The whole ruin fitted to the ground: the piles at every site, the close debris around each
/// inspection view and the wrecked ship.
pub(super) fn compose(ground: &Ground) -> (Vec<Prop>, Vec<&'static str>) {
    piles::compose(ground)
}

fn prop_guid(index: usize, model: usize) -> u64 {
    (0xF110_u64 << 48) | (u64::from(FIRST_ENTRY + model as u32) << 24) | (0x80_0000 + index as u64)
}

fn go_fields(guid: u64, entry: u32, display: u32, pos: [f32; 3], scale: f32, q: [f32; 4], facing: f32) -> Fields {
    let mut f = Fields::new();
    f.set_u64(idx::GUID, guid).set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT).set(idx::ENTRY, entry)
        .set_f32(idx::SCALE_X, scale).set(idx::GAMEOBJECT_DISPLAYID, display)
        .set(idx::GAMEOBJECT_FLAGS, INERT).set(idx::GAMEOBJECT_TYPE_ID, GENERIC)
        .set(idx::GAMEOBJECT_STATE, 1).set(idx::GAMEOBJECT_FACTION, 35).set(idx::GAMEOBJECT_ANIMPROGRESS, 100)
        .set_f32(idx::GAMEOBJECT_FACING, facing);
    for (i, value) in pos.iter().enumerate() { f.set_f32(idx::GAMEOBJECT_POS_X + i as u16, *value); }
    for (i, value) in q.iter().enumerate() { f.set_f32(idx::GAMEOBJECT_ROTATION + i as u16, *value); }
    f
}

impl SurvivalGame {
    pub(crate) fn aftermath_command(&mut self, conn: ConnId, arg: &str, target: &str) -> Result<(), String> {
        match arg {
            "on" => self.aftermath_start(conn),
            "off" => { self.aftermath_stop(); Ok(()) }
            "visit" => {
                if self.aftermath.is_none() { return Err("Start the preview with .es aftermath on.".into()); }
                let region = layout::REGIONS.iter().find(|r| r.key == target).ok_or("Choose a region with .es aftermath list.")?;
                let ground = self.aftermath_land.as_deref().ok_or("The aftermath ground is unavailable.")?;
                let z = floor(&*ground.land, region.view, false).ok_or("This view has no floor after the collapse.")?;
                self.aftermath_teleport(conn, [region.view[0], region.view[1], z+0.2], region.facing);
                self.say_to(conn, region.name);
                Ok(())
            }
            "point" => {
                if self.aftermath.is_none() { return Err("Start the preview with .es aftermath on.".into()); }
                let values: Vec<f32> = target.split_whitespace().map(str::parse::<f32>).collect::<Result<_, _>>()
                    .map_err(|_| "Use .es aftermath point <x> <y> <z> [facing].")?;
                if !(3..=4).contains(&values.len()) || values.iter().any(|v| !v.is_finite()) {
                    return Err("Use .es aftermath point <x> <y> <z> [facing].".into());
                }
                let at = [values[0], values[1], values[2]];
                let z = floor(&*self.aftermath_land.as_deref().unwrap().land, at, false)
                    .ok_or("That point has no dry floor after the collapse.")?;
                self.aftermath_teleport(conn, [at[0], at[1], z+0.2], values.get(3).copied().unwrap_or(0.0));
                Ok(())
            }
            "list" => {
                self.say_to(conn, &format!("Inspect a region with .es aftermath visit <name>. {}.", layout::REGIONS.iter().map(|r| r.key).collect::<Vec<_>>().join(", ")));
                Ok(())
            }
            "" | "status" => {
                let text = self.aftermath.as_ref().map_or_else(|| "Aftermath preview is off.".into(), |a|
                    format!("Aftermath preview is on. {} solid objects across {} regions. {} sites lack a matching floor. {} of the mine's own doodads knocked over or askew. {} dust spots, {} clouds raised so far. Use .es aftermath off to restore the dungeon.", a.props.len(), layout::REGIONS.len(), a.missing.len(), a.knocked.len(), a.air.dust.len(), a.air.puffs));
                self.say_to(conn, &text);
                Ok(())
            }
            _ => Err("Use .es aftermath on, off, list, status, visit <name> or point <x> <y> <z> [facing].".into()),
        }
    }

    fn aftermath_start(&mut self, conn: ConnId) -> Result<(), String> {
        if !self.sim.debug { return Err("Turn debug on first with .es debug quiet.".into()); }
        if self.realm != Realm::BELOW { return Err("Enter the Deadmines before starting its aftermath preview.".into()); }
        if self.aftermath.is_some() { return self.aftermath_command(conn, "status", ""); }
        // (What an ended fight left drawn doesn't count: a dead crossbowman's draw, a missile's
        // stand-in. Only a lab scene, or danger still marked on the ground, holds it.)
        if self.lab.is_some() || self.clear_fight_leftovers() > 0 {
            return Err("Finish the current fight or lab scene before starting the aftermath preview.".into());
        }
        if self.aftermath_land.is_none() {
            let data = super::lab::data_dir().ok_or("Start the server with --data or WOW_DATA to inspect the aftermath.")?;
            let survey = htl_map::deadmines::load_aftermath_survey(&data).map_err(|e| format!("Couldn't load the collapsed Deadmines. {e}"))?;
            let open = htl_map::deadmines::load_aftermath_open_survey(&data).map_err(|e| format!("Couldn't load the collapsed Deadmines. {e}"))?;
            let displays: Vec<u32> = MODELS.iter().map(|m| m.display).collect();
            let meshes = htl_map::deadmines::aftermath::display_shapes_with(&data, &displays, doodads::EXTRA).map_err(|e| format!("Couldn't load the ruins' models. {e}"))?;
            let mut ground = Ground::surveyed(survey, meshes.clone());
            ground.open = Some(Box::new(Ground::surveyed(open, meshes)));
            self.aftermath_land = Some(Box::new(ground));
        }
        let started = Instant::now();
        let (props, missing) = compose(self.aftermath_land.as_deref().unwrap());
        if layout::REGIONS.iter().enumerate().any(|(r, _)| !props.iter().any(|p| p.region == r)) {
            return Err("The aftermath has an empty region. Check the installed map data before previewing it.".into());
        }
        let knocked = self.aftermath_land.as_deref().and_then(|g| g.open.as_deref()).map_or_else(Vec::new, |open| piles::knock_over(open, &props).into_iter().map(|(_, p)| p).collect());
        println!("[aftermath] {} solids in {:?}; {} sites unavailable: {}", props.len(), started.elapsed(), missing.len(), missing.join(", "));
        println!("[aftermath] {} of the mine's own doodads knocked over or hung askew, for clients of round {}", knocked.len(), doodads::LEVEL);
        let air = air::Air::new(&props, self.aftermath_land.as_deref().unwrap(), Instant::now());
        println!("[aftermath] {} dust spots (unseen stand-ins casting {}), {} creaking beams", air.dust.len(), air::SPELL_DUST, props.iter().filter(|p| p.model == BEAM).count());
        self.aftermath = Some(Aftermath { props, positions: BTreeMap::new(), missing, air, knocked });
        let hidden: Vec<u64> = self.sim.mobs.keys().chain(self.sim.objs.keys()).filter_map(|id| self.guid_of(*id)).collect();
        for guid in hidden { self.broadcast(&packets::destroy_object(guid)); }
        let viewers: Vec<ConnId> = self.in_world().map(|(c, _, _)| *c).collect();
        for c in viewers {
            if let Some(w) = self.world_mut(c) { w.aftermath_return = None; }
            self.send_aftermath_to(c);
            self.aftermath_arrival(c);
        }
        self.aftermath_command(conn, "status", "")
    }

    pub(crate) fn send_aftermath_to(&self, conn: ConnId) {
        let Some(scene) = self.aftermath.as_ref() else { return };
        self.send(conn, packets::gameobject_query_response(MARKER_ENTRY, GENERIC, 0, ""));
        let f = go_fields(MARKER_GUID, MARKER_ENTRY, 0, [0.0; 3], 1.0, [0.0, 0.0, 0.0, 1.0], 0.0);
        self.send(conn, packets::update_object(&[Create { guid: MARKER_GUID, object_type: packets::TYPE_GAMEOBJECT,
            is_self: false, placement: Placement::At { pos: [0.0; 3], facing: 0.0 }, fields: &f }], &[]));
        for (i, m) in MODELS.iter().enumerate() {
            self.send(conn, packets::gameobject_query_response(FIRST_ENTRY+i as u32, GENERIC, m.display, m.name));
        }
        self.send_props_to(conn, &scene.props, 0);
        for p in scene.air.stand_ins() { self.send(conn, p); }
        self.send_knocked_to(conn);
    }

    /// The ruins `props`, numbered from `first`, in batches.
    fn send_props_to(&self, conn: ConnId, props: &[Prop], first: usize) {
        for (chunk_index, chunk) in props.chunks(BATCH).enumerate() {
            let at = first + chunk_index * BATCH;
            let fields: Vec<Fields> = chunk.iter().enumerate().map(|(i, p)| go_fields(prop_guid(at+i, p.model),
                FIRST_ENTRY+p.model as u32, MODELS[p.model].display, p.pos, p.scale, p.rotation, p.facing)).collect();
            let creates: Vec<Create> = chunk.iter().enumerate().map(|(i, p)| Create {
                guid: prop_guid(at+i, p.model), object_type: packets::TYPE_GAMEOBJECT, is_self: false,
                placement: Placement::At { pos: p.pos, facing: p.facing }, fields: &fields[i],
            }).collect();
            self.send(conn, packets::update_object(&creates, &[]));
        }
    }

    /// The copies of the mine's own doodads, to a client that hides the mine's (patch 0091's
    /// round): an older one keeps the mine's own and gets none, so nothing stands twice.
    pub(crate) fn send_knocked_to(&self, conn: ConnId) {
        let Some(scene) = self.aftermath.as_ref() else { return };
        if self.world_of(conn).is_none_or(|w| w.level < doodads::LEVEL) { return; }
        self.send_props_to(conn, &scene.knocked, scene.props.len());
    }

    /// The cameraman goes where the hero it films is inspecting. The rules stand still in the
    /// preview (the hero's place in them never changes, and `keep_watchers_near` doesn't run), so
    /// without this its client stayed at the landing: its world loaded there, it filmed the cove's
    /// water and the void past the loaded ground wherever the hero went.
    pub(crate) fn aftermath_keep_watchers_near(&mut self) {
        let Some(a) = self.aftermath.as_ref() else { return };
        let far: Vec<(ConnId, [f32; 3], f32)> = self.sessions.iter().filter_map(|(c, s)| {
            let name = s.watching.as_deref()?;
            if s.world.as_ref()?.porting { return None; }
            let hero = self.in_world()
                .find(|(_, hs, hw)| hs.watching.is_none() && !hw.porting && hw.name.eq_ignore_ascii_case(name))
                .map(|(hc, _, _)| *hc)?;
            let (at, facing) = *a.positions.get(&hero)?;
            let far = a.positions.get(c).is_none_or(|(p, _)| (0..3).map(|i| (p[i] - at[i]).powi(2)).sum::<f32>().sqrt() > WATCH_NEAR);
            far.then_some((*c, at, facing))
        }).collect();
        for (conn, at, facing) in far {
            if let Some(a) = self.aftermath.as_mut() { a.positions.insert(conn, (at, facing)); }
            let Some(w) = self.world_mut(conn) else { continue };
            w.counter = w.counter.wrapping_add(1);
            w.ground_z = at[2];
            w.fall_top = None;
            let (guid, counter) = (w.guid, w.counter);
            // (Only to the watcher: no one else is told of it.)
            self.send(conn, packets::teleport(guid, counter, at, facing));
        }
    }

    pub(crate) fn aftermath_arrival(&mut self, conn: ConnId) {
        // An inspector standing where the ruined mine has ground stays there; one who might have
        // logged out on the old ship (no ground there now) starts at the retained smugglers'
        // landing. The saved position is still used on exit. A cameraman stays by its hero and
        // follows them (`aftermath_keep_watchers_near`): sent off to the landing and back, its
        // client took ten seconds to load the ground again and filmed the void meanwhile.
        let id = self.world_of(conn).map(|w| w.id);
        let at = match self.sessions.get(&conn).is_some_and(|s| s.watching.is_some()) {
            true => id.and_then(|id| self.watchers.get(&id).or_else(|| self.sim.players.get(&id))).map(|b| (b.pos, b.facing)),
            false => id.and_then(|id| self.sim.players.get(&id)).map(|p| (p.pos, p.facing)),
        };
        let ground = |pos: [f32; 3]| self.aftermath_land.as_deref().is_some_and(|g| floor(&*g.land, pos, false).is_some());
        match at.filter(|(pos, _)| ground(*pos) || self.sessions.get(&conn).is_some_and(|s| s.watching.is_some())) {
            Some(at) => { if let Some(a) = self.aftermath.as_mut() { a.positions.insert(conn, at); } }
            None => { let _ = self.aftermath_command(conn, "visit", "landing"); }
        }
        let watchers: Vec<ConnId> = self.sessions.iter().filter(|(_, s)| s.watching.is_some()).map(|(c, _)| *c).collect();
        if let Some(a) = self.aftermath.as_ref() {
            for (other, (pos, facing)) in &a.positions {
                // (No one is told of a cameraman.)
                if *other != conn && !watchers.contains(other) {
                    if let Some(w) = self.world_of(*other) { self.send(conn, packets::heartbeat(w.guid, *pos, *facing)); }
                }
            }
        }
    }

    pub(crate) fn aftermath_stop(&mut self) {
        let Some(scene) = self.aftermath.take() else { return };
        for (i, p) in scene.props.iter().chain(&scene.knocked).enumerate() { self.broadcast(&packets::destroy_object(prop_guid(i, p.model))); }
        self.broadcast(&packets::destroy_object(MARKER_GUID));
        for s in &scene.air.dust { self.broadcast(&packets::destroy_object(s.guid)); }
        let viewers: Vec<(ConnId, Id)> = self.in_world().map(|(c, _, w)| (*c, w.id)).collect();
        for (conn, id) in viewers {
            if let Some(p) = self.sim.players.get(&id) { self.aftermath_teleport(conn, p.pos, p.facing); }
            self.send_world_to(conn, id);
            self.say_to(conn, "Restored the dungeon and your original position.");
        }
    }

    fn aftermath_teleport(&mut self, conn: ConnId, pos: [f32; 3], facing: f32) {
        let returning = self.aftermath.is_none();
        let Some(w) = self.world_mut(conn) else { return };
        w.counter = w.counter.wrapping_add(1);
        w.aftermath_return = returning.then_some(w.counter);
        w.fall_top = None;
        w.ground_z = pos[2];
        let (guid, counter) = (w.guid, w.counter);
        if let Some(a) = self.aftermath.as_mut() { a.positions.insert(conn, (pos, facing)); }
        self.send(conn, packets::teleport(guid, counter, pos, facing));
        self.broadcast_except(conn, &packets::heartbeat(guid, pos, facing));
    }

    pub(crate) fn aftermath_return_ack(&mut self, conn: ConnId, r: &mut Reader) -> std::io::Result<()> {
        if self.world_of(conn).and_then(|w| w.aftermath_return).is_none() { return Ok(()); }
        // The native client's reply is unpacked GUID, counter, client time (16 bytes).
        let guid = r.u64()?;
        let counter = r.u32()?;
        let _time = r.u32()?;
        if let Some(w) = self.world_mut(conn) {
            if w.guid == guid && w.aftermath_return == Some(counter) { w.aftermath_return = None; }
        }
        Ok(())
    }

    pub(crate) fn aftermath_movement(&mut self, conn: ConnId, pos: [f32; 3], facing: f32) -> bool {
        let Some(a) = self.aftermath.as_mut() else { return false };
        if pos.iter().all(|v| v.is_finite()) && facing.is_finite() { a.positions.insert(conn, (pos, facing)); }
        true
    }

    pub(crate) fn aftermath_where(&self, conn: ConnId) -> Option<([f32; 3], f32)> {
        self.aftermath.as_ref()?.positions.get(&conn).copied()
    }

    pub(crate) fn aftermath_query(&self, conn: ConnId, entry: u32) -> bool {
        if entry == MARKER_ENTRY {
            self.send(conn, packets::gameobject_query_response(entry, GENERIC, 0, ""));
            return true;
        }
        if let Some(m) = entry.checked_sub(FIRST_ENTRY).and_then(|i| MODELS.get(i as usize)) {
            self.send(conn, packets::gameobject_query_response(entry, GENERIC, m.display, m.name));
            return true;
        }
        false
    }
}
