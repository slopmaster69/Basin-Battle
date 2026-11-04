//! The world's solid shapes in the round, for a line to meet as the player sees them: every
//! building's collision faces (walls, roofs, floors over floors) and every doodad's collision hull
//! (rocks, fences, stone walls, statues, carts, tree trunks). The walking grid flattens these to
//! cells a walker can't enter between ankle and head height; a grappling hook aimed at a roof, a
//! boulder or a fence needs the shapes themselves.

/// One triangle, in world space.
pub type Tri = [[f32; 3]; 3];

/// Triangles bucketed over a grid of square cells on the map, for a line to test only those
/// along its way.
#[derive(Default)]
pub struct Solids {
    min: [f32; 2],
    cell: f32,
    dims: (usize, usize),
    /// Each cell's triangles: `items[starts[k]..starts[k + 1]]`.
    starts: Vec<u32>,
    items: Vec<u32>,
    tris: Vec<Tri>,
}

impl Solids {
    /// Bucket `tris` over the box from `min` to `max` in cells `cell` yards a side (triangles
    /// wholly outside it are left out).
    pub fn build(min: [f32; 2], max: [f32; 2], cell: f32, tris: Vec<Tri>) -> Self {
        let cell = cell.max(0.5);
        let dims = (((max[0] - min[0]) / cell).ceil().max(1.0) as usize, ((max[1] - min[1]) / cell).ceil().max(1.0) as usize);
        let range = |t: &Tri| -> Option<(usize, usize, usize, usize)> {
            let lo = [t[0][0].min(t[1][0]).min(t[2][0]), t[0][1].min(t[1][1]).min(t[2][1])];
            let hi = [t[0][0].max(t[1][0]).max(t[2][0]), t[0][1].max(t[1][1]).max(t[2][1])];
            if hi[0] < min[0] || hi[1] < min[1] || lo[0] > max[0] || lo[1] > max[1] {
                return None;
            }
            let i = |v: f32, o: f32, n: usize| (((v - o) / cell).floor().max(0.0) as usize).min(n - 1);
            Some((i(lo[0], min[0], dims.0), i(hi[0], min[0], dims.0), i(lo[1], min[1], dims.1), i(hi[1], min[1], dims.1)))
        };
        let mut counts = vec![0u32; dims.0 * dims.1];
        let mut kept = Vec::with_capacity(tris.len());
        for t in tris {
            if let Some((i0, i1, j0, j1)) = range(&t) {
                for i in i0..=i1 {
                    for j in j0..=j1 {
                        counts[i * dims.1 + j] += 1;
                    }
                }
                kept.push(t);
            }
        }
        let mut starts = Vec::with_capacity(counts.len() + 1);
        let mut total = 0u32;
        for c in &counts {
            starts.push(total);
            total += c;
        }
        starts.push(total);
        let mut fill = starts.clone();
        let mut items = vec![0u32; total as usize];
        for (n, t) in kept.iter().enumerate() {
            let (i0, i1, j0, j1) = range(t).expect("kept");
            for i in i0..=i1 {
                for j in j0..=j1 {
                    let k = i * dims.1 + j;
                    items[fill[k] as usize] = n as u32;
                    fill[k] += 1;
                }
            }
        }
        Self { min, cell, dims, starts, items, tris: kept }
    }

    /// How many triangles it holds.
    pub fn len(&self) -> usize {
        self.tris.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tris.is_empty()
    }

    /// How far along the line from `from` (`dir` a unit vector) the first triangle lies, within
    /// `max`: the cells it crosses in order, nearest first, each cell's triangles tested.
    pub fn ray(&self, from: [f32; 3], dir: [f32; 3], max: f32) -> Option<f32> {
        if self.tris.is_empty() || max <= 0.0 {
            return None;
        }
        let (w, h) = (self.dims.0 as i64, self.dims.1 as i64);
        let gx = (from[0] - self.min[0]) / self.cell;
        let gy = (from[1] - self.min[1]) / self.cell;
        let (mut i, mut j) = (gx.floor() as i64, gy.floor() as i64);
        let step = |d: f32| if d > 0.0 { 1i64 } else { -1 };
        let (si, sj) = (step(dir[0]), step(dir[1]));
        // The distance along the line to the next cell boundary in x and in y, and between them.
        let next = |g: f32, d: f32| -> (f32, f32) {
            if d.abs() < 1.0e-6 {
                return (f32::INFINITY, f32::INFINITY);
            }
            let edge = if d > 0.0 { g.floor() + 1.0 } else { g.floor() };
            (((edge - g) * self.cell / d).abs(), (self.cell / d).abs())
        };
        let (mut tx, dx) = next(gx, dir[0]);
        let (mut ty, dy) = next(gy, dir[1]);
        let mut best: Option<f32> = None;
        let mut seen: Vec<u32> = Vec::new();
        loop {
            if i >= 0 && j >= 0 && i < w && j < h {
                let k = (i * h + j) as usize;
                for &n in &self.items[self.starts[k] as usize..self.starts[k + 1] as usize] {
                    if seen.contains(&n) {
                        continue;
                    }
                    seen.push(n);
                    if let Some(t) = hit(&self.tris[n as usize], from, dir) {
                        if t <= max && best.is_none_or(|b| t < b) {
                            best = Some(t);
                        }
                    }
                }
            }
            // Leaving this cell: a hit nearer than its far edge can't be beaten further on.
            let leave = tx.min(ty);
            if best.is_some_and(|b| b <= leave) || leave > max {
                return best;
            }
            if tx < ty {
                tx += dx;
                i += si;
            } else {
                ty += dy;
                j += sj;
            }
        }
    }
}

/// Where the line from `o` along `d` meets the triangle, as a distance (either face; none behind
/// the start).
fn hit(t: &Tri, o: [f32; 3], d: [f32; 3]) -> Option<f32> {
    let sub = |a: [f32; 3], b: [f32; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let dot = |a: [f32; 3], b: [f32; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let (e1, e2) = (sub(t[1], t[0]), sub(t[2], t[0]));
    let p = cross(d, e2);
    let det = dot(e1, p);
    if det.abs() < 1.0e-9 {
        return None;
    }
    let inv = 1.0 / det;
    let s = sub(o, t[0]);
    let u = dot(s, p) * inv;
    if !(-1.0e-5..=1.0 + 1.0e-5).contains(&u) {
        return None;
    }
    let q = cross(s, e1);
    let v = dot(d, q) * inv;
    if v < -1.0e-5 || u + v > 1.0 + 1.0e-5 {
        return None;
    }
    let t = dot(e2, q) * inv;
    (t >= 0.0).then_some(t)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A wall: a 4 yd square standing upright at x = `x`, from y -2 to 2, z 0 to 4.
    fn wall(x: f32) -> Vec<Tri> {
        vec![[[x, -2.0, 0.0], [x, 2.0, 0.0], [x, 2.0, 4.0]], [[x, -2.0, 0.0], [x, 2.0, 4.0], [x, -2.0, 4.0]]]
    }

    #[test]
    fn a_line_meets_the_nearest_shape_along_it_and_nothing_off_it() {
        let mut tris = wall(10.0);
        tris.extend(wall(20.0));
        let s = Solids::build([-50.0, -50.0], [50.0, 50.0], 4.0, tris);
        assert_eq!(s.len(), 4);
        let d = s.ray([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 30.0).expect("the near wall");
        assert!((d - 10.0).abs() < 1.0e-3, "{d}");
        // Over its top, the far wall is missed too (it's no taller); short of it, nothing.
        let up: [f32; 3] = [0.9, 0.0, 0.4359];
        assert!(s.ray([0.0, 0.0, 1.0], up, 40.0).is_none());
        assert!(s.ray([0.0, 0.0, 1.0], [1.0, 0.0, 0.0], 8.0).is_none(), "out of reach");
        assert!(s.ray([0.0, 5.0, 1.0], [1.0, 0.0, 0.0], 30.0).is_none(), "beside it");
        // Backward, diagonal and from the far side all find it.
        let back = s.ray([15.0, 0.0, 1.0], [-1.0, 0.0, 0.0], 30.0).expect("from behind");
        assert!((back - 5.0).abs() < 1.0e-3, "{back}");
        let k = std::f32::consts::FRAC_1_SQRT_2;
        let diag = s.ray([7.0, -3.0, 1.0], [k, k, 0.0], 30.0).expect("at a slant");
        assert!((diag - 3.0 / k).abs() < 1.0e-3, "{diag}");
    }

    #[test]
    fn a_shape_spanning_many_cells_is_met_once_and_off_the_box_is_left_out() {
        let roof: Vec<Tri> = vec![[[0.0, 0.0, 5.0], [30.0, 0.0, 5.0], [0.0, 30.0, 5.0]], [[500.0, 500.0, 0.0], [501.0, 500.0, 0.0], [500.0, 501.0, 1.0]]];
        let s = Solids::build([-10.0, -10.0], [40.0, 40.0], 2.0, roof);
        assert_eq!(s.len(), 1, "the far one is off the box");
        let d = s.ray([5.0, 5.0, 1.0], [0.0, 0.0, 1.0], 10.0).expect("straight up into it");
        assert!((d - 4.0).abs() < 1.0e-3, "{d}");
    }
}
