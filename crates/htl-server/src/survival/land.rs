//! Elwynn as the survival rules see it: the host's own terrain around Goldshire (heights, walls,
//! water, the cobbled roads) with every tree trunk solid.

use htl_map::elwynn::{LoadedElwynn, Trunk};
use htl_map::grid::{find_path_with, HeightGrid};
use htl_sim::Ground;
use htl_survival::Land;

type Vec3 = [f32; 3];

/// How steep a walk may climb (rise over run).
const MAX_SLOPE: f32 = 1.2;
/// Walks longer than this are planned a leg at a time (the first leg this long at most).
const MAX_PLAN: f32 = 420.0;
/// A long walk's leg (a search this long takes a few milliseconds).
const LEG: f32 = 140.0;
/// Below, nobody walks far: a foe chases within its leash and walks home from there (a longer
/// plan over the whole mine's floors would stall a tick).
const BELOW_PLAN: f32 = 120.0;
/// And it looks at no more floors than this for one (a few milliseconds).
const BELOW_LOOK: usize = 15_000;
static PLAN_CALLS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PLAN_NANOS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
static PLAN_WORST: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
/// Walking through a trunk's cell costs this much more: only where there's no way round at all
/// (a walk that starts against one); a straight walk never goes through one.
const TRUNK_COST: f32 = 2000.0;

pub struct ElwynnLand {
    grid: HeightGrid,
    trunks: Vec<Trunk>,
    /// Cells a trunk stands in.
    trunk_cells: Vec<bool>,
    /// The path search's cost per cell: trunks dear, all else even.
    extra: Vec<f32>,
    /// Which ground is Elwynn's.
    zone: htl_map::elwynn::ZoneMask,
    /// The box it covers.
    bounds: ([f32; 2], [f32; 2]),
    /// The caves and mines, each its own ground with floors below floors over its box.
    caves: Vec<CaveLand>,
    /// Walks that found no way, and when.
    failed: std::sync::Mutex<std::collections::HashMap<(i32, i32, i32, i32), std::time::Instant>>,
}

/// A cave's ground and its box.
struct CaveLand {
    min: [f32; 2],
    max: [f32; 2],
    land: DeadminesLand,
}

/// Below the hill's slope by this much: in the cave, not on the hill over it.
const UNDERGROUND: f32 = 2.5;

impl ElwynnLand {
    pub fn new(loaded: LoadedElwynn) -> Self {
        let LoadedElwynn { min, max, grid, trunks, zone, caves } = loaded;
        let caves = caves.into_iter().map(|c| CaveLand { min: c.min, max: c.max, land: DeadminesLand::from_layers(c.ground) }).collect();
        let (w, h) = grid.size();
        let mut trunk_cells = vec![false; w * h];
        let cell = grid.cell();
        for t in &trunks {
            let r = t.radius;
            let steps = (r / cell).ceil() as i32;
            for dj in -steps..=steps {
                for di in -steps..=steps {
                    let (x, y) = (t.pos[0] + di as f32 * cell, t.pos[1] + dj as f32 * cell);
                    if (di as f32 * cell).hypot(dj as f32 * cell) > r + 0.25 {
                        continue;
                    }
                    if let Some((i, j)) = grid.cell_at(x, y) {
                        trunk_cells[j * w + i] = true;
                    }
                }
            }
        }
        let extra = trunk_cells.iter().map(|t| if *t { TRUNK_COST } else { 1.0 }).collect();
        Self { grid, trunks, trunk_cells, extra, zone, bounds: (min, max), caves, failed: Default::default() }
    }

    pub fn height_at(&self, x: f32, y: f32) -> f32 {
        self.grid.height(x, y)
    }

    /// A building's wall stands here (not counting trunks).
    pub fn is_wall(&self, x: f32, y: f32) -> bool {
        self.grid.is_blocked(x, y)
    }

    pub fn is_trunk(&self, x: f32, y: f32) -> bool {
        self.trunk(x, y)
    }

    /// The cave whose box (x, y) is in, if any.
    fn cave(&self, x: f32, y: f32) -> Option<&CaveLand> {
        self.caves.iter().find(|c| x >= c.min[0] && x <= c.max[0] && y >= c.min[1] && y <= c.max[1])
    }

    /// Down in a cave (well under the hill's slope over it), not on the land.
    fn underground(&self, p: Vec3) -> bool {
        self.cave(p[0], p[1]).is_some() && p[2] < self.grid.height(p[0], p[1]) - UNDERGROUND
    }

    /// How many caves the land knows, and their boxes (for the probe).
    pub fn caves(&self) -> Vec<([f32; 2], [f32; 2])> {
        self.caves.iter().map(|c| (c.min, c.max)).collect()
    }

    fn trunk(&self, x: f32, y: f32) -> bool {
        let (w, _) = self.grid.size();
        self.grid.cell_at(x, y).is_some_and(|(i, j)| self.trunk_cells[j * w + i])
    }

    /// Whether a walker can go straight from `a` to `b`: nothing solid, no cliff, no deep water.
    fn walkable(&self, a: Vec3, b: Vec3) -> bool {
        let d = (b[0] - a[0]).hypot(b[1] - a[1]);
        let steps = (d / 0.5).ceil().max(1.0) as usize;
        let mut last = self.grid.height(a[0], a[1]);
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
            if self.blocked(x, y) || self.grid.is_water(x, y) {
                return false;
            }
            let z = self.grid.height(x, y);
            if (z - last).abs() > MAX_SLOPE * (d / steps as f32) + 0.05 {
                return false;
            }
            last = z;
        }
        true
    }
}

impl Land for ElwynnLand {
    fn height(&self, x: f32, y: f32) -> f32 {
        self.grid.height(x, y)
    }

    fn blocked(&self, x: f32, y: f32) -> bool {
        // Off the edge of the world we read is as good as a wall.
        self.grid.cell_at(x, y).is_none() || self.grid.is_blocked(x, y) || self.trunk(x, y)
    }

    fn water(&self, x: f32, y: f32) -> bool {
        self.grid.is_water(x, y)
    }

    fn road(&self, x: f32, y: f32) -> f32 {
        self.grid.road(x, y)
    }

    fn plans(&self) -> bool {
        true
    }

    // In a cave's box, a walker's ground is the cave's (its tunnels as well as the hill), the
    // trees standing only on the land's own surface.
    fn height_near(&self, x: f32, y: f32, z: f32) -> f32 {
        match self.cave(x, y) {
            Some(c) => c.land.height_near(x, y, z),
            None => self.grid.height(x, y),
        }
    }

    fn blocked_near(&self, x: f32, y: f32, z: f32) -> bool {
        match self.cave(x, y) {
            Some(c) => c.land.blocked_near(x, y, z) || (z > self.grid.height(x, y) - UNDERGROUND && self.trunk(x, y)),
            None => self.blocked(x, y),
        }
    }

    fn water_near(&self, x: f32, y: f32, z: f32) -> bool {
        match self.cave(x, y) {
            Some(c) => c.land.water_near(x, y, z),
            None => self.grid.is_water(x, y),
        }
    }

    fn slab_between(&self, x: f32, y: f32, a: f32, b: f32) -> bool {
        self.cave(x, y).is_some_and(|c| c.land.slab_between(x, y, a, b))
    }

    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool {
        match (self.cave(from[0], from[1]), self.cave(to[0], to[1])) {
            (Some(a), Some(b)) if std::ptr::eq(a, b) => a.land.walk_clear(from, to),
            // Into or out of a cave's box: on the land's surface only.
            _ if self.underground(from) || self.underground(to) => false,
            _ => self.walkable(from, to),
        }
    }

    fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let t0 = std::time::Instant::now();
        let out = self.path_inner(from, to);
        let ns = t0.elapsed().as_nanos() as u64;
        if let Ok(mut w) = SLOWEST.lock() {
            if w.as_ref().is_none_or(|(n, ..)| ns > *n) {
                *w = Some((ns, from, to, out.is_some()));
            }
        }
        out
    }

    fn trees(&self) -> Vec<Vec3> {
        self.trunks.iter().map(|t| t.pos).collect()
    }

    fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        Some(self.bounds)
    }

    fn in_zone(&self, x: f32, y: f32) -> bool {
        self.zone.contains(x, y)
    }
}

/// The slowest walk planned on Elwynn since last asked: its time (ns), its ends, whether found.
static SLOWEST: std::sync::Mutex<Option<(u64, Vec3, Vec3, bool)>> = std::sync::Mutex::new(None);

impl ElwynnLand {
    /// The slowest walk planned since the last ask (for the probe).
    pub fn slowest_walk() -> Option<(u64, Vec3, Vec3, bool)> {
        SLOWEST.lock().ok().and_then(|mut w| w.take())
    }

    fn path_inner(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        // A walk in a cave (or from its mouth in): its own ground's search.
        match (self.cave(from[0], from[1]), self.cave(to[0], to[1])) {
            (Some(a), Some(b)) if std::ptr::eq(a, b) => return a.land.path(from, to),
            _ if self.underground(from) || self.underground(to) => return None,
            _ => {}
        }
        let d = (to[0] - from[0]).hypot(to[1] - from[1]);
        if d > MAX_PLAN {
            // A long walk (a raid's march, a long way home): its first leg, to open dry ground
            // about that far along the way; the walker plans the next from there.
            let t = LEG / d;
            let (x, y) = (from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t);
            let leg = (0..48).find_map(|k| {
                let (r, a) = ((k / 8) as f32 * 6.0, (k % 8) as f32 * std::f32::consts::FRAC_PI_4);
                let (px, py) = (x + r * a.cos(), y + r * a.sin());
                (!self.blocked(px, py) && !self.grid.is_water(px, py)).then(|| [px, py, self.grid.height(px, py)])
            })?;
            return self.path(from, leg);
        }
        if self.grid.is_blocked(to[0], to[1]) || self.grid.is_blocked(from[0], from[1]) {
            return None;
        }
        // A walk that found no way recently isn't searched again for a while (a failed search
        // reads the whole land).
        let key = (
            (from[0] / 6.0).floor() as i32,
            (from[1] / 6.0).floor() as i32,
            (to[0] / 6.0).floor() as i32,
            (to[1] / 6.0).floor() as i32,
        );
        let now = std::time::Instant::now();
        if let Ok(f) = self.failed.lock() {
            if f.get(&key).is_some_and(|t| now.duration_since(*t).as_secs_f32() < 20.0) {
                return None;
            }
        }
        let Some(cells) = find_path_with(&self.grid, from, to, MAX_SLOPE, Some(&self.extra)) else {
            if let Ok(mut f) = self.failed.lock() {
                if f.len() > 2000 {
                    f.clear();
                }
                f.insert(key, now);
            }
            return None;
        };
        // Pull the string: from each corner, the farthest point still in a straight walk.
        let mut out = Vec::new();
        let mut at = 0;
        while at + 1 < cells.len() {
            let mut next = at + 1;
            // Forward while the straight walk holds (a few cells at a time).
            let mut k = at + 4;
            while k < cells.len() && self.walkable(cells[at], cells[k]) {
                next = k;
                k += 4;
            }
            if k >= cells.len() && self.walkable(cells[at], cells[cells.len() - 1]) {
                next = cells.len() - 1;
            }
            out.push(cells[next]);
            at = next;
        }
        Some(out)
    }
}

/// The Deadmines as the rules see it: ground with floors above floors (decks over the cove,
/// catwalks over the foundry), each walker on its own.
pub struct DeadminesLand {
    ground: htl_map::layers::Layers,
    failed:
        std::sync::Mutex<std::collections::HashMap<(i32, i32, i32, i32, i32), std::time::Instant>>,
}

/// Asked with no height: from above everything (the highest floor).
const FROM_ABOVE: f32 = 1.0e4;

impl DeadminesLand {
    pub fn new(loaded: htl_map::deadmines::LoadedDeadmines) -> Self {
        Self::from_layers(loaded.ground)
    }

    /// Ground with floors above floors (the Deadmines, or one of Elwynn's caves).
    pub fn from_layers(ground: htl_map::layers::Layers) -> Self {
        Self {
            ground,
            failed: Default::default(),
        }
    }

    pub fn ground(&self) -> &htl_map::layers::Layers {
        &self.ground
    }

    /// A straight walk from a to b, floor to floor, with nothing solid and no drop.
    fn walkable(&self, a: Vec3, b: Vec3) -> bool {
        let d = (b[0] - a[0]).hypot(b[1] - a[1]);
        let steps = (d / 0.5).ceil().max(1.0) as usize;
        let mut z = a[2];
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
            let Some(f) = self.ground.floor_near(x, y, z) else {
                return false;
            };
            if f.blocked || f.water || (f.z - z).abs() > MAX_SLOPE * (d / steps as f32) + 0.05 {
                return false;
            }
            z = f.z;
        }
        (z - b[2]).abs() < 1.0
    }
}

impl Land for DeadminesLand {
    fn height(&self, x: f32, y: f32) -> f32 {
        self.ground.height_near(x, y, FROM_ABOVE)
    }
    fn height_near(&self, x: f32, y: f32, z: f32) -> f32 {
        self.ground.height_near(x, y, z)
    }
    fn blocked(&self, x: f32, y: f32) -> bool {
        self.ground.blocked_near(x, y, FROM_ABOVE)
    }
    fn blocked_near(&self, x: f32, y: f32, z: f32) -> bool {
        self.ground.blocked_near(x, y, z)
    }
    fn water(&self, x: f32, y: f32) -> bool {
        self.ground.water_near(x, y, FROM_ABOVE)
    }
    fn water_near(&self, x: f32, y: f32, z: f32) -> bool {
        self.ground.water_near(x, y, z)
    }
    fn slab_between(&self, x: f32, y: f32, a: f32, b: f32) -> bool {
        self.ground.slab_between(x, y, a, b)
    }
    fn plans(&self) -> bool {
        true
    }
    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool {
        self.walkable(from, to)
    }
    fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        if (to[0] - from[0]).hypot(to[1] - from[1]) > BELOW_PLAN {
            return None;
        }
        self.plan(from, to, BELOW_LOOK)
    }
}

impl DeadminesLand {
    /// A walk however far (for probing the layout).
    pub fn walk(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        self.plan(from, to, usize::MAX)
    }

    /// A walk looking at no more than `limit` floors, the string pulled; failures remembered a
    /// while.
    fn plan(&self, from: Vec3, to: Vec3, limit: usize) -> Option<Vec<Vec3>> {
        let t0 = std::time::Instant::now();
        let out = self.plan_inner(from, to, limit);
        let ns = t0.elapsed().as_nanos() as u64;
        PLAN_CALLS.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        PLAN_NANOS.fetch_add(ns, std::sync::atomic::Ordering::Relaxed);
        PLAN_WORST.fetch_max(ns, std::sync::atomic::Ordering::Relaxed);
        out
    }

    /// Walks planned below since the last ask: how many, their time in all, the longest (ns).
    pub fn plan_stats() -> (u64, u64, u64) {
        use std::sync::atomic::Ordering::Relaxed;
        (PLAN_CALLS.swap(0, Relaxed), PLAN_NANOS.swap(0, Relaxed), PLAN_WORST.swap(0, Relaxed))
    }

    fn plan_inner(&self, from: Vec3, to: Vec3, limit: usize) -> Option<Vec<Vec3>> {
        let key = (
            (from[0] / 6.0).floor() as i32,
            (from[1] / 6.0).floor() as i32,
            (to[0] / 6.0).floor() as i32,
            (to[1] / 6.0).floor() as i32,
            (from[2] / 4.0).floor() as i32,
        );
        let now = std::time::Instant::now();
        if let Ok(f) = self.failed.lock() {
            if f.get(&key)
                .is_some_and(|t| now.duration_since(*t).as_secs_f32() < 20.0)
            {
                return None;
            }
        }
        let Some(cells) = self.ground.find_path_within(from, to, MAX_SLOPE, None, limit) else {
            if let Ok(mut f) = self.failed.lock() {
                if f.len() > 2000 {
                    f.clear();
                }
                f.insert(key, now);
            }
            return None;
        };
        // Pull the string, as on Elwynn's ground.
        let mut out = Vec::new();
        let mut at = 0;
        while at + 1 < cells.len() {
            let mut next = at + 1;
            let mut k = at + 4;
            while k < cells.len() && self.walkable(cells[at], cells[k]) {
                next = k;
                k += 4;
            }
            if k >= cells.len() && self.walkable(cells[at], cells[cells.len() - 1]) {
                next = cells.len() - 1;
            }
            out.push(cells[next]);
            at = next;
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use htl_survival::Land;

    /// A small land: level ground, a cliff 20 yards high from x = 40 on, and a tree at (20, 0).
    fn land() -> ElwynnLand {
        let grid = HeightGrid::sample([0.0, -40.0], [80.0, 40.0], 1.0, |x, _| Some(if x >= 40.0 { 20.0 } else { 0.0 }));
        let trunks = vec![Trunk { pos: [20.0, 0.0, 0.0], radius: 1.5 }];
        ElwynnLand::new(LoadedElwynn { min: htl_map::elwynn::MIN, max: htl_map::elwynn::MAX, grid, trunks, zone: Default::default(), caves: Vec::new() })
    }

    #[test]
    fn a_straight_walk_goes_neither_through_a_trunk_nor_up_a_cliff() {
        let l = land();
        assert!(l.walk_clear([10.0, 5.0, 0.0], [30.0, 5.0, 0.0]), "past the tree");
        assert!(!l.walk_clear([10.0, 0.0, 0.0], [30.0, 0.0, 0.0]), "through the trunk");
        assert!(!l.walk_clear([30.0, 10.0, 0.0], [50.0, 10.0, 20.0]), "up the cliff");
        // The line of sight sees past the cliff's foot (why a chase used to walk up it).
        assert!(htl_survival::land::clear_line(&l, [30.0, 10.0, 0.0], [38.0, 10.0, 0.0]));
    }

    #[test]
    fn a_walk_goes_round_the_trunk_and_finds_no_way_up_the_cliff() {
        let l = land();
        let path = l.path([10.0, 0.0, 0.0], [30.0, 0.0, 0.0]).expect("round the tree");
        let mut at = [10.0f32, 0.0, 0.0];
        for p in &path {
            assert!(l.walk_clear(at, *p), "each leg walkable: {at:?} to {p:?} in {path:?}");
            at = *p;
        }
        assert!(l.path([30.0, 10.0, 0.0], [50.0, 10.0, 20.0]).is_none(), "the cliff has no way up");
    }
}
