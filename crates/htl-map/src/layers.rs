//! Ground with floors above floors: a dungeon's decks over water, catwalks over a foundry, ramps
//! between levels. Each grid cell keeps every surface a walker could stand on there, from the
//! terrain and the buildings' collision, each with its own walls and water; a walker is always on
//! one of them, picked by the height it comes from. Paths step from floor to floor between cells,
//! so they climb ramps and stairs and never pass from a deck to the hold below through the planks.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

/// Floors closer together than this in one cell are one floor (the higher).
const MERGE: f32 = 0.35;
/// A floor with something solid this close above it is too cramped to stand on.
const HEADROOM: f32 = 1.9;
/// A wall between these heights above a floor blocks walking.
const WALL_FROM: f32 = 0.6;
const WALL_TO: f32 = 2.2;
/// Water deeper than this over a floor is "deep": paths avoid it.
const DEEP_WATER: f32 = 1.2;
/// A walker coming from height z stands on the highest floor no more than this above it.
pub const STEP_UP: f32 = 1.3;

/// One walkable surface over a cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Floor {
    pub z: f32,
    /// A wall at body height stands on it, or it's too cramped to stand on.
    pub blocked: bool,
    /// Deep water over it.
    pub water: bool,
    /// Any water over it.
    pub wet: bool,
}

pub struct Layers {
    min: [f32; 2],
    cell: f32,
    w: usize,
    h: usize,
    /// Cell k's floors are `floors[start[k]..start[k + 1]]`, lowest first.
    start: Vec<u32>,
    floors: Vec<Floor>,
    /// Which walkable piece of ground each floor is in (at the steepest walk paths take):
    /// floors in different pieces have no walk between them, so no search is needed.
    piece: Vec<u32>,
}

/// The steepest climb the pieces of ground are worked out for (the same as walkers').
pub const PIECE_SLOPE: f32 = 1.2;

/// What the ground is built from: triangles in world space, and the terrain where there is any.
pub struct Sources<'a> {
    /// The terrain's height, if there is terrain (not a hole) at (x, y).
    pub terrain: &'a dyn Fn(f32, f32) -> Option<f32>,
    /// Every solid triangle: floors, walls and ceilings are told apart by their slope.
    pub solid: &'a [[[f32; 3]; 3]],
    /// Water surfaces' triangles.
    pub liquid: &'a [[[f32; 3]; 3]],
}

fn normal_z(t: &[[f32; 3]; 3]) -> f32 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
    n[2] / len
}

/// The z of triangle `t` above (x, y), if (x, y) is inside its footprint.
fn z_at(t: &[[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let (a, b, c) = (t[0], t[1], t[2]);
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if det.abs() < 1e-9 {
        return None;
    }
    let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / det;
    let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / det;
    let l3 = 1.0 - l1 - l2;
    let eps = -1e-4;
    (l1 >= eps && l2 >= eps && l3 >= eps).then(|| l1 * a[2] + l2 * b[2] + l3 * c[2])
}

fn edge_max(t: &[[f32; 3]; 3]) -> f32 {
    let d = |a: [f32; 3], b: [f32; 3]| {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    };
    d(t[0], t[1]).max(d(t[1], t[2])).max(d(t[2], t[0]))
}

impl Layers {
    /// Build the ground over the box `min`..`max`.
    pub fn build(min: [f32; 2], max: [f32; 2], cell: f32, src: &Sources) -> Self {
        let w = ((max[0] - min[0]) / cell).ceil() as usize + 1;
        let h = ((max[1] - min[1]) / cell).ceil() as usize + 1;
        let n = w * h;
        let mut floors: Vec<Vec<f32>> = vec![Vec::new(); n];
        let mut over: Vec<Vec<f32>> = vec![Vec::new(); n];
        let mut walls: Vec<Vec<f32>> = vec![Vec::new(); n];
        let mut wet: Vec<Vec<f32>> = vec![Vec::new(); n];
        let cell_at = |x: f32, y: f32| -> Option<usize> {
            let i = ((x - min[0]) / cell).round();
            let j = ((y - min[1]) / cell).round();
            (i >= 0.0 && j >= 0.0 && (i as usize) < w && (j as usize) < h)
                .then(|| j as usize * w + i as usize)
        };
        let range = |t: &[[f32; 3]; 3]| {
            let lo_x = t.iter().map(|p| p[0]).fold(f32::MAX, f32::min);
            let hi_x = t.iter().map(|p| p[0]).fold(f32::MIN, f32::max);
            let lo_y = t.iter().map(|p| p[1]).fold(f32::MAX, f32::min);
            let hi_y = t.iter().map(|p| p[1]).fold(f32::MIN, f32::max);
            let f = |v: f32, o: f32, m: usize| ((v - o) / cell).clamp(0.0, (m - 1) as f32);
            (
                (
                    f(lo_x, min[0], w).floor() as usize,
                    f(lo_y, min[1], h).floor() as usize,
                ),
                (
                    f(hi_x, min[0], w).ceil() as usize,
                    f(hi_y, min[1], h).ceil() as usize,
                ),
            )
        };
        // A surface over the cells its footprint covers (and its middle, for a small one).
        let paint = |t: &[[f32; 3]; 3], out: &mut Vec<Vec<f32>>| {
            let ((i0, j0), (i1, j1)) = range(t);
            let mut any = false;
            for j in j0..=j1 {
                for i in i0..=i1 {
                    let (x, y) = (min[0] + i as f32 * cell, min[1] + j as f32 * cell);
                    if let Some(z) = z_at(t, x, y) {
                        out[j * w + i].push(z);
                        any = true;
                    }
                }
            }
            if !any {
                let c = [
                    (t[0][0] + t[1][0] + t[2][0]) / 3.0,
                    (t[0][1] + t[1][1] + t[2][1]) / 3.0,
                    (t[0][2] + t[1][2] + t[2][2]) / 3.0,
                ];
                if let Some(k) = cell_at(c[0], c[1]) {
                    out[k].push(c[2]);
                }
            }
        };
        for t in src.solid {
            let nz = normal_z(t);
            if nz > 0.6 {
                paint(t, &mut floors);
            } else if nz < -0.6 {
                paint(t, &mut over);
            } else {
                // A wall: sample its surface and mark the cells it stands in.
                let m = (edge_max(t) / (cell * 0.5)).ceil().max(1.0) as usize;
                for a in 0..=m {
                    for b in 0..=(m - a) {
                        let (fa, fb) = (a as f32 / m as f32, b as f32 / m as f32);
                        let fc = 1.0 - fa - fb;
                        let p = [
                            t[0][0] * fa + t[1][0] * fb + t[2][0] * fc,
                            t[0][1] * fa + t[1][1] * fb + t[2][1] * fc,
                            t[0][2] * fa + t[1][2] * fb + t[2][2] * fc,
                        ];
                        if let Some(k) = cell_at(p[0], p[1]) {
                            walls[k].push(p[2]);
                        }
                    }
                }
            }
        }
        for t in src.liquid {
            paint(t, &mut wet);
        }
        let mut start = Vec::with_capacity(n + 1);
        let mut out = Vec::new();
        for j in 0..h {
            for i in 0..w {
                let k = j * w + i;
                start.push(out.len() as u32);
                let (x, y) = (min[0] + i as f32 * cell, min[1] + j as f32 * cell);
                let mut zs = std::mem::take(&mut floors[k]);
                if let Some(z) = (src.terrain)(x, y) {
                    zs.push(z);
                }
                zs.retain(|z| z.is_finite());
                zs.sort_by(|a, b| a.total_cmp(b));
                // One floor for each that's apart from the next (the higher of a close pair).
                let mut merged: Vec<f32> = Vec::new();
                for z in zs {
                    match merged.last_mut() {
                        Some(last) if z - *last < MERGE => *last = z,
                        _ => merged.push(z),
                    }
                }
                for (fi, &z) in merged.iter().enumerate() {
                    let next = merged.get(fi + 1).copied().unwrap_or(f32::INFINITY);
                    let cramped = next - z < HEADROOM
                        || over[k].iter().any(|&c| c > z + 0.15 && c < z + HEADROOM);
                    let walled = walls[k]
                        .iter()
                        .any(|&v| v > z + WALL_FROM && v < z + WALL_TO);
                    // The water over this floor: a surface above it and below the next floor up.
                    let surface = wet[k]
                        .iter()
                        .copied()
                        .filter(|&s| s >= z - 0.2 && s < next)
                        .fold(f32::NAN, f32::max);
                    let (is_wet, deep) = if surface.is_finite() {
                        (surface > z + 0.05, surface > z + DEEP_WATER)
                    } else {
                        (false, false)
                    };
                    out.push(Floor {
                        z,
                        blocked: cramped || walled,
                        water: deep,
                        wet: is_wet,
                    });
                }
            }
        }
        start.push(out.len() as u32);
        let mut g = Self { min, cell, w, h, start, floors: out, piece: Vec::new() };
        g.piece = g.pieces(PIECE_SLOPE);
        g
    }

    /// Each floor's piece of walkable ground.
    fn pieces(&self, max_slope: f32) -> Vec<u32> {
        let mut piece = vec![u32::MAX; self.floors.len()];
        let mut next = 0u32;
        let mut stack = Vec::new();
        for k in 0..self.w * self.h {
            for f in self.range_of(k) {
                // (A blocked floor is no piece's: nothing steps onto it.)
                if piece[f] != u32::MAX || self.floors[f].blocked {
                    continue;
                }
                piece[f] = next;
                stack.push((k, f));
                while let Some((ck, cf)) = stack.pop() {
                    let mut more = Vec::new();
                    self.steps(ck, cf, max_slope, |nk, g, _| more.push((nk, g)));
                    for (nk, g) in more {
                        if piece[g] == u32::MAX {
                            piece[g] = next;
                            stack.push((nk, g));
                        }
                    }
                }
                next += 1;
            }
        }
        piece
    }

    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    pub fn cell(&self) -> f32 {
        self.cell
    }
    pub fn origin(&self) -> [f32; 2] {
        self.min
    }
    pub fn floor_count(&self) -> usize {
        self.floors.len()
    }
    pub fn cell_xy(&self, i: usize, j: usize) -> [f32; 2] {
        [
            self.min[0] + i as f32 * self.cell,
            self.min[1] + j as f32 * self.cell,
        ]
    }
    /// The cell containing (x, y), if it is on the grid.
    pub fn cell_at(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let fi = ((x - self.min[0]) / self.cell).round();
        let fj = ((y - self.min[1]) / self.cell).round();
        (fi >= 0.0 && fj >= 0.0 && (fi as usize) < self.w && (fj as usize) < self.h)
            .then(|| (fi as usize, fj as usize))
    }
    /// The floors over cell (i, j), lowest first.
    pub fn floors_at(&self, i: usize, j: usize) -> &[Floor] {
        let k = j * self.w + i;
        &self.floors[self.start[k] as usize..self.start[k + 1] as usize]
    }
    fn range_of(&self, k: usize) -> std::ops::Range<usize> {
        self.start[k] as usize..self.start[k + 1] as usize
    }
    /// The floor a walker coming from height `z` stands on in cell k: the highest no more than
    /// [`STEP_UP`] above it, else the lowest.
    fn pick(&self, k: usize, z: f32) -> Option<usize> {
        let r = self.range_of(k);
        if r.is_empty() {
            return None;
        }
        let below = r.clone().rev().find(|&f| self.floors[f].z <= z + STEP_UP);
        Some(below.unwrap_or(r.start))
    }
    /// The floor a walker at (x, y, z) stands on, if there's any ground there.
    pub fn floor_near(&self, x: f32, y: f32, z: f32) -> Option<Floor> {
        let (i, j) = self.cell_at(x, y)?;
        self.pick(j * self.w + i, z).map(|f| self.floors[f])
    }
    /// The ground's height under a walker at (x, y) coming from height `z`, smoothed between
    /// cells on the same floor.
    pub fn height_near(&self, x: f32, y: f32, z: f32) -> f32 {
        let fx = ((x - self.min[0]) / self.cell).clamp(0.0, (self.w - 1) as f32);
        let fy = ((y - self.min[1]) / self.cell).clamp(0.0, (self.h - 1) as f32);
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let near = {
            let (ci, cj) = (fx.round() as usize, fy.round() as usize);
            match self.pick(cj * self.w + ci, z) {
                Some(f) => self.floors[f].z,
                None => return z,
            }
        };
        // Each corner's floor, as the walker would find it from the nearest cell's floor; a corner
        // that's a different level counts as the nearest's.
        let corner = |a: usize, b: usize| {
            let (a, b) = (a.min(self.w - 1), b.min(self.h - 1));
            match self.pick(b * self.w + a, near) {
                Some(f) if (self.floors[f].z - near).abs() < 1.5 => self.floors[f].z,
                _ => near,
            }
        };
        let top = corner(i, j) * (1.0 - tx) + corner(i + 1, j) * tx;
        let bottom = corner(i, j + 1) * (1.0 - tx) + corner(i + 1, j + 1) * tx;
        top * (1.0 - ty) + bottom * ty
    }
    /// Nothing to stand on here, or a wall, for a walker coming from height `z`.
    pub fn blocked_near(&self, x: f32, y: f32, z: f32) -> bool {
        self.floor_near(x, y, z).is_none_or(|f| f.blocked)
    }
    pub fn water_near(&self, x: f32, y: f32, z: f32) -> bool {
        self.floor_near(x, y, z).is_some_and(|f| f.water)
    }
    /// Whether a floor lies between heights `a` and `b` (either way round) over (x, y): a deck or
    /// a ceiling a line between them would pass through.
    pub fn slab_between(&self, x: f32, y: f32, a: f32, b: f32) -> bool {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        let Some((i, j)) = self.cell_at(x, y) else {
            return false;
        };
        self.floors_at(i, j)
            .iter()
            .any(|f| f.z > lo + 0.05 && f.z < hi - 0.05)
    }

    fn center(&self, f: usize) -> [f32; 3] {
        let k = self.start.partition_point(|&s| s as usize <= f) - 1;
        let [x, y] = self.cell_xy(k % self.w, k / self.w);
        [x, y, self.floors[f].z]
    }

    /// The neighbours of floor `f` (in cell k) a walker can step to: one floor in each of the
    /// eight cells around, within the slope, open and not deep water.
    fn steps(&self, k: usize, f: usize, max_slope: f32, mut each: impl FnMut(usize, usize, f32)) {
        const STEPS: [(i32, i32, f32); 8] = [
            (1, 0, 1.0),
            (-1, 0, 1.0),
            (0, 1, 1.0),
            (0, -1, 1.0),
            (1, 1, std::f32::consts::SQRT_2),
            (1, -1, std::f32::consts::SQRT_2),
            (-1, 1, std::f32::consts::SQRT_2),
            (-1, -1, std::f32::consts::SQRT_2),
        ];
        let (ci, cj) = ((k % self.w) as i32, (k / self.w) as i32);
        let z = self.floors[f].z;
        for (di, dj, len) in STEPS {
            let (ni, nj) = (ci + di, cj + dj);
            if ni < 0 || nj < 0 || ni as usize >= self.w || nj as usize >= self.h {
                continue;
            }
            let nk = nj as usize * self.w + ni as usize;
            let run = len * self.cell;
            let reach = max_slope * run + 0.1;
            let best = self
                .range_of(nk)
                .filter(|&g| (self.floors[g].z - z).abs() <= reach && !self.floors[g].blocked)
                .min_by(|&a, &b| {
                    (self.floors[a].z - z)
                        .abs()
                        .total_cmp(&(self.floors[b].z - z).abs())
                });
            if let Some(g) = best {
                {
                    // A diagonal step doesn't squeeze between two walls.
                    if di != 0 && dj != 0 {
                        let side = |a: i32, b: i32| {
                            let sk = b as usize * self.w + a as usize;
                            self.range_of(sk)
                                .filter(|&s| (self.floors[s].z - z).abs() <= reach)
                                .all(|s| self.floors[s].blocked)
                        };
                        if side(ci + di, cj) && side(ci, cj + dj) {
                            continue;
                        }
                    }
                    each(nk, g, run);
                }
            }
        }
    }

    /// The shortest walk from `from` to `to`, floor to floor, never steeper than `max_slope`;
    /// `extra` (by cell) makes some cells dearer. Wading costs more and deep water far more.
    pub fn find_path(
        &self,
        from: [f32; 3],
        to: [f32; 3],
        max_slope: f32,
        extra: Option<&[f32]>,
    ) -> Option<Vec<[f32; 3]>> {
        self.search(from, to, max_slope, extra, usize::MAX)
    }

    /// [`Self::find_path`], giving up (None) after looking at `limit` floors: a short walk found
    /// fast, and no stall on one that winds round half the map (or has no way at all).
    pub fn find_path_within(
        &self,
        from: [f32; 3],
        to: [f32; 3],
        max_slope: f32,
        extra: Option<&[f32]>,
        limit: usize,
    ) -> Option<Vec<[f32; 3]>> {
        self.search(from, to, max_slope, extra, limit)
    }

    fn search(
        &self,
        from: [f32; 3],
        to: [f32; 3],
        max_slope: f32,
        extra: Option<&[f32]>,
        limit: usize,
    ) -> Option<Vec<[f32; 3]>> {
        let (si, sj) = self.cell_at(from[0], from[1])?;
        let (gi, gj) = self.cell_at(to[0], to[1])?;
        let start = self.pick(sj * self.w + si, from[2])?;
        let goal = self.pick(gj * self.w + gi, to[2])?;
        // Ground with no walk between it: no search.
        let (a, b) = (self.piece[start], self.piece[goal]);
        if (max_slope - PIECE_SLOPE).abs() < 1e-3 && a != u32::MAX && b != u32::MAX && a != b {
            return None;
        }
        // The cost so far and the floor it came from, of every floor looked at (a map: a short
        // search touches few of the mine's floors).
        let mut seen: std::collections::HashMap<usize, (f32, usize)> = std::collections::HashMap::new();
        let mut open = BinaryHeap::new();
        let heur = |k: usize| {
            let (dx, dy) = (
                (k % self.w) as f32 - gi as f32,
                (k / self.w) as f32 - gj as f32,
            );
            (dx * dx + dy * dy).sqrt() * self.cell
        };
        let start_k = sj * self.w + si;
        seen.insert(start, (0.0, usize::MAX));
        open.push(Node {
            f: heur(start_k),
            floor: start,
            cell: start_k,
        });
        let mut looked = 0usize;
        let mut found = false;
        while let Some(Node {
            f,
            floor: cur,
            cell: k,
        }) = open.pop()
        {
            if cur == goal {
                found = true;
                break;
            }
            let base = seen[&cur].0;
            // A stale entry (a cheaper way here was found since).
            if f > base + heur(k) + 1e-3 {
                continue;
            }
            looked += 1;
            if looked > limit {
                return None;
            }
            let cz = self.floors[cur].z;
            self.steps(k, cur, max_slope, |nk, g, run| {
                let fl = self.floors[g];
                let slope = (fl.z - cz).abs() / run;
                let ground = if fl.water {
                    1500.0
                } else if fl.wet {
                    4.0
                } else {
                    1.0
                };
                let more = extra.map_or(1.0, |e| e[nk]);
                let c = base + run * (1.0 + 2.0 * slope * slope) * ground * more;
                if seen.get(&g).is_none_or(|(old, _)| c < *old) {
                    seen.insert(g, (c, cur));
                    open.push(Node {
                        f: c + heur(nk),
                        floor: g,
                        cell: nk,
                    });
                }
            });
        }
        if !found {
            return None;
        }
        let mut chain = vec![goal];
        while let Some(&last) = chain.last() {
            let prev = seen[&last].1;
            if prev == usize::MAX {
                break;
            }
            chain.push(prev);
        }
        chain.reverse();
        let mut path: Vec<[f32; 3]> = chain.iter().map(|&f| self.center(f)).collect();
        path[0] = [from[0], from[1], self.floors[start].z];
        if let Some(last) = path.last_mut() {
            *last = [to[0], to[1], self.floors[goal].z];
        }
        Some(path)
    }

    /// Every floor a walker can reach from `from`, by floor index.
    pub fn reachable(&self, from: [f32; 3], max_slope: f32) -> Vec<bool> {
        let mut seen = vec![false; self.floors.len()];
        let Some((i, j)) = self.cell_at(from[0], from[1]) else {
            return seen;
        };
        let k0 = j * self.w + i;
        let Some(f0) = self.pick(k0, from[2]) else {
            return seen;
        };
        seen[f0] = true;
        let mut stack = vec![(k0, f0)];
        while let Some((k, f)) = stack.pop() {
            let mut next = Vec::new();
            self.steps(k, f, max_slope, |nk, g, _| next.push((nk, g)));
            for (nk, g) in next {
                if !seen[g] {
                    seen[g] = true;
                    stack.push((nk, g));
                }
            }
        }
        seen
    }

    /// How many steps (cell to cell) a walker takes from `from` to each floor, by floor index
    /// (`u32::MAX` for those it can't reach): a breadth-first [`Self::reachable`].
    pub fn reach_steps(&self, from: [f32; 3], max_slope: f32) -> Vec<u32> {
        let mut dist = vec![u32::MAX; self.floors.len()];
        let Some((i, j)) = self.cell_at(from[0], from[1]) else {
            return dist;
        };
        let k0 = j * self.w + i;
        let Some(f0) = self.pick(k0, from[2]) else {
            return dist;
        };
        dist[f0] = 0;
        let mut queue = std::collections::VecDeque::from([(k0, f0)]);
        while let Some((k, f)) = queue.pop_front() {
            let d = dist[f];
            let mut next = Vec::new();
            self.steps(k, f, max_slope, |nk, g, _| next.push((nk, g)));
            for (nk, g) in next {
                if dist[g] == u32::MAX {
                    dist[g] = d + 1;
                    queue.push_back((nk, g));
                }
            }
        }
        dist
    }

    /// The floor indices over cell (i, j), lowest first (for [`Self::reachable`]'s answer).
    pub fn floor_range(&self, i: usize, j: usize) -> std::ops::Range<usize> {
        self.range_of(j * self.w + i)
    }
    pub fn floor(&self, f: usize) -> Floor {
        self.floors[f]
    }
}

#[derive(PartialEq)]
struct Node {
    f: f32,
    floor: usize,
    cell: usize,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .f
            .total_cmp(&self.f)
            .then_with(|| other.floor.cmp(&self.floor))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A square of two triangles at height z over x0..x1, y0..y1, facing up (or down).
    fn quad(x0: f32, x1: f32, y0: f32, y1: f32, z: f32, up: bool) -> Vec<[[f32; 3]; 3]> {
        let (a, b, c, d) = ([x0, y0, z], [x1, y0, z], [x1, y1, z], [x0, y1, z]);
        if up {
            vec![[a, b, c], [a, c, d]]
        } else {
            vec![[a, c, b], [a, d, c]]
        }
    }

    /// A deck at z 10 over 10..30 in x, over ground at 0; a ramp up to it from x 0..10 at y 0..4.
    fn deck_world() -> Layers {
        let mut solid = Vec::new();
        solid.extend(quad(10.0, 30.0, -10.0, 10.0, 10.0, true));
        solid.extend(quad(10.0, 30.0, -10.0, 10.0, 9.7, false));
        // The ramp: 0 at x 0 to 10 at x 10.
        let r = |x: f32, y: f32| [x, y, x];
        solid.push([r(0.0, 0.0), r(10.0, 0.0), r(10.0, 4.0)]);
        solid.push([r(0.0, 0.0), r(10.0, 4.0), r(0.0, 4.0)]);
        let terrain = |_x: f32, _y: f32| Some(0.0);
        Layers::build(
            [-10.0, -20.0],
            [40.0, 20.0],
            1.0,
            &Sources {
                terrain: &terrain,
                solid: &solid,
                liquid: &[],
            },
        )
    }

    #[test]
    fn a_walker_stays_on_its_own_floor() {
        let g = deck_world();
        // Under the deck: the ground.
        assert!((g.height_near(20.0, 0.0, 0.5) - 0.0).abs() < 0.01);
        // On the deck: the deck.
        assert!((g.height_near(20.0, 0.0, 10.2) - 10.0).abs() < 0.01);
        // The ground under the deck is open (the deck is high enough over it); the deck is open.
        assert!(!g.blocked_near(20.0, 0.0, 0.0));
        assert!(!g.blocked_near(20.0, 0.0, 10.0));
        assert!(g.slab_between(20.0, 0.0, 1.0, 11.0));
    }

    #[test]
    fn a_path_onto_the_deck_climbs_the_ramp() {
        let g = deck_world();
        let path = g
            .find_path([-5.0, 2.0, 0.0], [25.0, 0.0, 10.0], 1.2, None)
            .expect("a way up");
        assert!(path
            .iter()
            .any(|p| p[0] > 2.0 && p[0] < 8.0 && (0.0..=4.0).contains(&p[1]) && p[2] > 1.0));
        assert!((path.last().unwrap()[2] - 10.0).abs() < 0.01);
        // And under the deck, the walk stays on the ground.
        let low = g
            .find_path([5.0, -8.0, 0.0], [25.0, -8.0, 0.0], 1.2, None)
            .expect("a way under");
        assert!(low.iter().all(|p| p[2] < 0.5));
    }

    #[test]
    fn a_low_ceiling_is_no_floor() {
        let mut solid = quad(0.0, 10.0, 0.0, 10.0, 1.2, false);
        solid.extend(quad(0.0, 10.0, 0.0, 10.0, 1.5, true));
        let terrain = |_x: f32, _y: f32| Some(0.0);
        let g = Layers::build(
            [-5.0, -5.0],
            [15.0, 15.0],
            1.0,
            &Sources {
                terrain: &terrain,
                solid: &solid,
                liquid: &[],
            },
        );
        // The ground under a slab 1 yard up: too cramped.
        assert!(g.blocked_near(5.0, 5.0, 0.0));
        assert!(!g.blocked_near(-3.0, 5.0, 0.0));
    }
}
