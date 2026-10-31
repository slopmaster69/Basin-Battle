//! A height grid over the arena and a shortest-path search on it.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use htl_sim::{Ground, Vec3};

/// Terrain heights sampled on a square grid. Holes in the terrain (none in Arathi Basin's play
/// area, but possible) take the nearest sampled height.
pub struct HeightGrid {
    min: [f32; 2],
    cell: f32,
    w: usize,
    h: usize,
    z: Vec<f32>,
    /// A wall at walking height stands in this cell.
    blocked: Vec<bool>,
    /// Deep water covers this cell.
    water: Vec<bool>,
    /// Any water at all covers this cell (a shallow ford too).
    wet: Vec<bool>,
    /// How much of this cell is paved road, 0 to 1.
    road: Vec<f32>,
    /// A bridge deck over water.
    deck: Vec<bool>,
}

impl HeightGrid {
    pub fn sample(min: [f32; 2], max: [f32; 2], cell: f32, f: impl Fn(f32, f32) -> Option<f32>) -> Self {
        let w = ((max[0] - min[0]) / cell).ceil() as usize + 1;
        let h = ((max[1] - min[1]) / cell).ceil() as usize + 1;
        let mut z = vec![f32::NAN; w * h];
        for j in 0..h {
            for i in 0..w {
                let (x, y) = (min[0] + i as f32 * cell, min[1] + j as f32 * cell);
                if let Some(v) = f(x, y) {
                    z[j * w + i] = v;
                }
            }
        }
        // Fill holes from a neighbour, a few passes.
        for _ in 0..8 {
            let snapshot = z.clone();
            for j in 0..h {
                for i in 0..w {
                    if !snapshot[j * w + i].is_nan() {
                        continue;
                    }
                    let mut sum = 0.0;
                    let mut n = 0;
                    for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                        let (ni, nj) = (i as i32 + di, j as i32 + dj);
                        if ni >= 0 && nj >= 0 && (ni as usize) < w && (nj as usize) < h {
                            let v = snapshot[nj as usize * w + ni as usize];
                            if !v.is_nan() {
                                sum += v;
                                n += 1;
                            }
                        }
                    }
                    if n > 0 {
                        z[j * w + i] = sum / n as f32;
                    }
                }
            }
        }
        for v in &mut z {
            if v.is_nan() {
                *v = 0.0;
            }
        }
        let n = w * h;
        Self {
            min,
            cell,
            w,
            h,
            z,
            blocked: vec![false; n],
            water: vec![false; n],
            wet: vec![false; n],
            road: vec![0.0; n],
            deck: vec![false; n],
        }
    }

    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }
    pub fn cell(&self) -> f32 {
        self.cell
    }
    pub fn heights(&self) -> &[f32] {
        &self.z
    }
    pub fn set_layers(&mut self, floor: Vec<f32>, blocked: Vec<bool>, water: Vec<bool>, wet: Vec<bool>) {
        assert_eq!(floor.len(), self.z.len());
        self.z = floor;
        self.blocked = blocked;
        self.water = water;
        self.wet = wet;
    }
    pub fn cell_xy(&self, i: usize, j: usize) -> [f32; 2] {
        [self.min[0] + i as f32 * self.cell, self.min[1] + j as f32 * self.cell]
    }
    /// The cell containing (x, y), if it is on the grid.
    pub fn cell_at(&self, x: f32, y: f32) -> Option<(usize, usize)> {
        let fi = ((x - self.min[0]) / self.cell).round();
        let fj = ((y - self.min[1]) / self.cell).round();
        (fi >= 0.0 && fj >= 0.0 && (fi as usize) < self.w && (fj as usize) < self.h)
            .then(|| (fi as usize, fj as usize))
    }
    /// The cells a triangle's footprint can touch, clamped to the grid.
    pub fn cell_range(&self, t: &[[f32; 3]; 3]) -> ((usize, usize), (usize, usize)) {
        let xs = [t[0][0], t[1][0], t[2][0]];
        let ys = [t[0][1], t[1][1], t[2][1]];
        let fx = |v: f32| ((v - self.min[0]) / self.cell).clamp(0.0, (self.w - 1) as f32);
        let fy = |v: f32| ((v - self.min[1]) / self.cell).clamp(0.0, (self.h - 1) as f32);
        let lo = (
            fx(xs.iter().cloned().fold(f32::MAX, f32::min)).floor() as usize,
            fy(ys.iter().cloned().fold(f32::MAX, f32::min)).floor() as usize,
        );
        let hi = (
            fx(xs.iter().cloned().fold(f32::MIN, f32::max)).ceil() as usize,
            fy(ys.iter().cloned().fold(f32::MIN, f32::max)).ceil() as usize,
        );
        (lo, hi)
    }
    pub fn is_blocked(&self, x: f32, y: f32) -> bool {
        self.cell_at(x, y).is_some_and(|(i, j)| self.blocked[j * self.w + i])
    }
    pub fn is_water(&self, x: f32, y: f32) -> bool {
        self.cell_at(x, y).is_some_and(|(i, j)| self.water[j * self.w + i])
    }
    /// Paved road under each cell, 0 to 1.
    pub fn set_road(&mut self, road: Vec<f32>) {
        assert_eq!(road.len(), self.z.len());
        self.road = road;
    }
    pub fn set_deck(&mut self, deck: Vec<bool>) {
        assert_eq!(deck.len(), self.z.len());
        self.deck = deck;
    }
    /// A bridge deck over water stands here.
    pub fn is_deck(&self, x: f32, y: f32) -> bool {
        self.cell_at(x, y).is_some_and(|(i, j)| self.deck[j * self.w + i])
    }
    /// How much of this spot is road, 0 to 1.
    pub fn road(&self, x: f32, y: f32) -> f32 {
        self.cell_at(x, y).map_or(0.0, |(i, j)| self.road[j * self.w + i])
    }
    /// The cell index of (x, y), clamped to the grid.
    pub fn index_of(&self, x: f32, y: f32) -> usize {
        let (i, j) = self.cell_of(x, y);
        j * self.w + i
    }
    /// Any water over this spot, however shallow.
    pub fn is_wet(&self, x: f32, y: f32) -> bool {
        self.cell_at(x, y).is_some_and(|(i, j)| self.wet[j * self.w + i])
    }
    /// The nearest point to `near` with `clear` yards of open, dry, fairly level ground around it.
    pub fn open_spot(&self, near: [f32; 2], clear: f32, search: f32) -> Option<Vec3> {
        self.open_spot_where(near, clear, search, |_| true)
    }

    /// [`Self::open_spot`], also requiring `ok(cell index)` of the chosen cell.
    pub fn open_spot_where(
        &self,
        near: [f32; 2],
        clear: f32,
        search: f32,
        ok: impl Fn(usize) -> bool,
    ) -> Option<Vec3> {
        let r = (clear / self.cell).ceil() as i32;
        let steps = (search / self.cell).ceil() as i32;
        let mut best: Option<(f32, Vec3)> = None;
        let (ci, cj) = self.cell_at(near[0], near[1])?;
        for dj in -steps..=steps {
            for di in -steps..=steps {
                let (i, j) = (ci as i32 + di, cj as i32 + dj);
                if i - r < 0 || j - r < 0 || i + r >= self.w as i32 || j + r >= self.h as i32 {
                    continue;
                }
                let d = ((di * di + dj * dj) as f32).sqrt() * self.cell;
                if best.as_ref().is_some_and(|(bd, _)| *bd <= d) {
                    continue;
                }
                let z0 = self.at(i as usize, j as usize);
                if !ok(j as usize * self.w + i as usize) {
                    continue;
                }
                let clear_all = (-r..=r).all(|y| {
                    (-r..=r).all(|x| {
                        if x * x + y * y > r * r {
                            return true;
                        }
                        let k = (j + y) as usize * self.w + (i + x) as usize;
                        !self.blocked[k] && !self.water[k] && (self.z[k] - z0).abs() < 1.5
                    })
                });
                if clear_all {
                    let [x, y] = self.cell_xy(i as usize, j as usize);
                    best = Some((d, [x, y, z0]));
                }
            }
        }
        best.map(|(_, p)| p)
    }

    fn at(&self, i: usize, j: usize) -> f32 {
        self.z[j.min(self.h - 1) * self.w + i.min(self.w - 1)]
    }

    fn cell_of(&self, x: f32, y: f32) -> (usize, usize) {
        let i = ((x - self.min[0]) / self.cell).round().clamp(0.0, (self.w - 1) as f32) as usize;
        let j = ((y - self.min[1]) / self.cell).round().clamp(0.0, (self.h - 1) as f32) as usize;
        (i, j)
    }

    fn center(&self, i: usize, j: usize) -> Vec3 {
        let (x, y) = (self.min[0] + i as f32 * self.cell, self.min[1] + j as f32 * self.cell);
        [x, y, self.at(i, j)]
    }
}

impl Ground for HeightGrid {
    fn height(&self, x: f32, y: f32) -> f32 {
        let fx = ((x - self.min[0]) / self.cell).clamp(0.0, (self.w - 1) as f32);
        let fy = ((y - self.min[1]) / self.cell).clamp(0.0, (self.h - 1) as f32);
        let (i, j) = (fx.floor() as usize, fy.floor() as usize);
        let (tx, ty) = (fx - i as f32, fy - j as f32);
        let a = self.at(i, j) * (1.0 - tx) + self.at(i + 1, j) * tx;
        let b = self.at(i, j + 1) * (1.0 - tx) + self.at(i + 1, j + 1) * tx;
        a * (1.0 - ty) + b * ty
    }
}

#[derive(PartialEq)]
struct Node {
    f: f32,
    idx: usize,
}
impl Eq for Node {}
impl Ord for Node {
    fn cmp(&self, other: &Self) -> Ordering {
        other.f.total_cmp(&self.f).then_with(|| other.idx.cmp(&self.idx))
    }
}
impl PartialOrd for Node {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Walking off the road costs this many times more (on bare ground).
const OFF_ROAD: f32 = 3.5;
/// A grid past this many cells searches a window round a walk's ends, this many yards past them
/// at the least.
const BIG_GRID: usize = 1_000_000;
const WINDOW_MARGIN: f32 = 60.0;
/// Wading a shallow river costs this many times more than the road.
const WADE: f32 = 150.0;
/// Deep water: a lane swims only when there is no other way at all.
const SWIM: f32 = 1500.0;

/// The shortest walk from `from` to `to` over the grid that never climbs or drops steeper than
/// `max_slope` (rise over run). Roads are cheapest, open ground costs more, rivers far more (a
/// bridge is dry), and steep ground costs extra, so paths keep to the roads and bridges.
pub fn find_path(grid: &HeightGrid, from: Vec3, to: Vec3, max_slope: f32) -> Option<Vec<Vec3>> {
    find_path_with(grid, from, to, max_slope, None)
}

/// A cell's extra cost above which a diagonal step won't cut its corner.
const CORNER_DEAR: f32 = 10.0;

/// [`find_path`] with an extra cost factor per cell (1.0 for none).
pub fn find_path_with(grid: &HeightGrid, from: Vec3, to: Vec3, max_slope: f32, extra: Option<&[f32]>) -> Option<Vec<Vec3>> {
    let (w, h) = (grid.w, grid.h);
    let start = grid.cell_of(from[0], from[1]);
    let goal = grid.cell_of(to[0], to[1]);
    // On a big land the search keeps to a window round the two ends (room for a detour), so a
    // walk costs its own neighbourhood, not the whole land.
    let (i0, j0, i1, j1) = if w * h > BIG_GRID {
        let d = ((start.0 as f32 - goal.0 as f32).hypot(start.1 as f32 - goal.1 as f32) * 0.75).max(WINDOW_MARGIN / grid.cell) as usize;
        (start.0.min(goal.0).saturating_sub(d), start.1.min(goal.1).saturating_sub(d), (start.0.max(goal.0) + d).min(w - 1), (start.1.max(goal.1) + d).min(h - 1))
    } else {
        (0, 0, w - 1, h - 1)
    };
    let lw = i1 - i0 + 1;
    let lh = j1 - j0 + 1;
    // Cells by their place in the window; the grid's own index is got back from it.
    let idx = |(i, j): (usize, usize)| (j - j0) * lw + (i - i0);
    let global = |k: usize| (k % lw + i0, k / lw + j0);
    let mut cost = vec![f32::INFINITY; lw * lh];
    let mut came = vec![usize::MAX; lw * lh];
    let mut open = BinaryHeap::new();
    let heur = |i: usize, j: usize| {
        let (dx, dy) = (i as f32 - goal.0 as f32, j as f32 - goal.1 as f32);
        (dx * dx + dy * dy).sqrt() * grid.cell
    };
    cost[idx(start)] = 0.0;
    open.push(Node {
        f: heur(start.0, start.1),
        idx: idx(start),
    });
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
    while let Some(Node { idx: cur, .. }) = open.pop() {
        if cur == idx(goal) {
            break;
        }
        let (ci, cj) = global(cur);
        let cz = grid.at(ci, cj);
        for (di, dj, len) in STEPS {
            let (ni, nj) = (ci as i32 + di, cj as i32 + dj);
            if ni < i0 as i32 || nj < j0 as i32 || ni as usize > i1 || nj as usize > j1 {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            let run = len * grid.cell;
            let slope = (grid.at(ni, nj) - cz).abs() / run;
            let next = idx((ni, nj));
            let g = nj * w + ni;
            if slope > max_slope || grid.blocked[g] {
                continue;
            }
            // No cutting a corner: a diagonal step only where both cells beside it are open
            // (and neither is one dear to cross, a trunk's).
            if di != 0 && dj != 0 {
                let side = |i: usize, j: usize| {
                    let k = j * w + i;
                    grid.blocked[k] || extra.is_some_and(|e| e[k] > CORNER_DEAR)
                };
                if side(ni, cj) || side(ci, nj) {
                    continue;
                }
            }
            let ground = if grid.water[g] {
                SWIM
            } else if grid.wet[g] {
                WADE
            } else {
                1.0 + (OFF_ROAD - 1.0) * (1.0 - grid.road[g])
            };
            let more = extra.map_or(1.0, |e| e[g]);
            let step = run * (1.0 + 2.0 * slope * slope) * ground * more;
            let c = cost[cur] + step;
            if c < cost[next] {
                cost[next] = c;
                came[next] = cur;
                open.push(Node {
                    f: c + heur(ni, nj),
                    idx: next,
                });
            }
        }
    }
    if cost[idx(goal)].is_infinite() {
        return None;
    }
    let mut cells = vec![idx(goal)];
    while let Some(&last) = cells.last() {
        let prev = came[last];
        if prev == usize::MAX {
            break;
        }
        cells.push(prev);
    }
    cells.reverse();
    let mut path: Vec<Vec3> = cells.iter().map(|&c| {
        let (i, j) = global(c);
        grid.center(i, j)
    }).collect();
    path[0] = [from[0], from[1], grid.height(from[0], from[1])];
    if let Some(last) = path.last_mut() {
        *last = [to[0], to[1], grid.height(to[0], to[1])];
    }
    Some(path)
}

/// Every cell a walker can reach from `from`, under the same rules as [`find_path`].
pub fn reachable(grid: &HeightGrid, from: Vec3, max_slope: f32) -> Vec<bool> {
    let (w, h) = (grid.w, grid.h);
    let mut seen = vec![false; w * h];
    let Some(start) = grid.cell_at(from[0], from[1]) else {
        return seen;
    };
    let mut stack = vec![start];
    seen[start.1 * w + start.0] = true;
    while let Some((ci, cj)) = stack.pop() {
        let cz = grid.at(ci, cj);
        for (di, dj, len) in [(1i32, 0i32, 1.0f32), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0)] {
            let (ni, nj) = (ci as i32 + di, cj as i32 + dj);
            if ni < 0 || nj < 0 || ni as usize >= w || nj as usize >= h {
                continue;
            }
            let (ni, nj) = (ni as usize, nj as usize);
            let k = nj * w + ni;
            if seen[k] || grid.blocked[k] {
                continue;
            }
            if (grid.at(ni, nj) - cz).abs() / (len * grid.cell) > max_slope {
                continue;
            }
            seen[k] = true;
            stack.push((ni, nj));
        }
    }
    seen
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_goes_round_a_cliff_by_the_ramp() {
        // A plateau 15 yards up for x > 20, reached only by a ramp at y in 40..50 that climbs
        // from x = 20 to x = 40.
        let grid = HeightGrid::sample([0.0, 0.0], [60.0, 60.0], 1.0, |x, y| {
            Some(if x <= 20.0 {
                0.0
            } else if (40.0..=50.0).contains(&y) {
                ((x - 20.0) * 0.75).min(15.0)
            } else {
                15.0
            })
        });
        let path = find_path(&grid, [5.0, 5.0, 0.0], [58.0, 5.0, 15.0], 0.9).expect("a path");
        // It must pass through the ramp.
        assert!(path.iter().any(|p| p[0] > 21.0 && p[0] < 39.0 && (40.0..=50.0).contains(&p[1])));
        let lane = crate::finish_lane(&grid, &path);
        assert!(lane.windows(2).all(|w| htl_sim::geom::dist2d(w[0], w[1]) <= 4.01));
    }

    #[test]
    fn no_path_up_a_sheer_wall() {
        let grid = HeightGrid::sample([0.0, 0.0], [30.0, 30.0], 1.0, |x, _| Some(if x > 15.0 { 50.0 } else { 0.0 }));
        assert!(find_path(&grid, [2.0, 2.0, 0.0], [28.0, 2.0, 50.0], 0.9).is_none());
    }
}
