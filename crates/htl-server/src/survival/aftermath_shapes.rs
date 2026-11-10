//! The ruins' geometry: rotations built from the way a piece should lie, model boxes turned into
//! the world, and the overlap tests that keep a board out of a wall or out of the mine's own carts.

pub(super) type V3 = [f32; 3];

pub(super) fn add(a: V3, b: V3) -> V3 { [a[0]+b[0], a[1]+b[1], a[2]+b[2]] }
pub(super) fn sub(a: V3, b: V3) -> V3 { [a[0]-b[0], a[1]-b[1], a[2]-b[2]] }
pub(super) fn mul(a: V3, k: f32) -> V3 { [a[0]*k, a[1]*k, a[2]*k] }
pub(super) fn dot(a: V3, b: V3) -> f32 { a[0]*b[0] + a[1]*b[1] + a[2]*b[2] }
pub(super) fn cross(a: V3, b: V3) -> V3 { [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]] }
pub(super) fn len(a: V3) -> f32 { dot(a, a).sqrt() }
pub(super) fn unit(a: V3) -> V3 { mul(a, 1.0 / len(a).max(1e-6)) }

/// Roll about the model's X, then pitch about Y, then yaw about Z (x, y, z, w).
pub(super) fn euler(roll: f32, pitch: f32, yaw: f32) -> [f32; 4] {
    let (sr, cr) = (roll * 0.5).sin_cos();
    let (sp, cp) = (pitch * 0.5).sin_cos();
    let (sy, cy) = (yaw * 0.5).sin_cos();
    [sr*cp*cy-cr*sp*sy, cr*sp*cy+sr*cp*sy, cr*cp*sy-sr*sp*cy, cr*cp*cy+sr*sp*sy]
}

/// The rotation that turns the model's +X onto `x` and +Y onto `y` (unit and square to each
/// other): a board's face and its length, a log's side and its run.
pub(super) fn frame(x: V3, y: V3) -> [f32; 4] {
    let z = cross(x, y);
    // Columns x, y, z; the standard trace-based conversion.
    let (m00, m11, m22) = (x[0], y[1], z[2]);
    let trace = m00 + m11 + m22;
    let q = if trace > 0.0 {
        let s = (trace + 1.0).sqrt() * 2.0;
        [(y[2]-z[1])/s, (z[0]-x[2])/s, (x[1]-y[0])/s, 0.25*s]
    } else if m00 > m11 && m00 > m22 {
        let s = (1.0 + m00 - m11 - m22).sqrt() * 2.0;
        [0.25*s, (y[0]+x[1])/s, (z[0]+x[2])/s, (y[2]-z[1])/s]
    } else if m11 > m22 {
        let s = (1.0 + m11 - m00 - m22).sqrt() * 2.0;
        [(y[0]+x[1])/s, 0.25*s, (z[1]+y[2])/s, (z[0]-x[2])/s]
    } else {
        let s = (1.0 + m22 - m00 - m11).sqrt() * 2.0;
        [(z[0]+x[2])/s, (z[1]+y[2])/s, 0.25*s, (x[1]-y[0])/s]
    };
    let n = q.iter().map(|v| v*v).sum::<f32>().sqrt();
    [q[0]/n, q[1]/n, q[2]/n, q[3]/n]
}

/// The rotation's matrix, rows.
pub(super) fn matrix(q: [f32; 4]) -> [V3; 3] {
    let [x, y, z, w] = q;
    [
        [1.0-2.0*(y*y+z*z), 2.0*(x*y-z*w), 2.0*(x*z+y*w)],
        [2.0*(x*y+z*w), 1.0-2.0*(x*x+z*z), 2.0*(y*z-x*w)],
        [2.0*(x*z-y*w), 2.0*(y*z+x*w), 1.0-2.0*(x*x+y*y)],
    ]
}

pub(super) fn turn(m: &[V3; 3], v: V3) -> V3 { [dot(m[0], v), dot(m[1], v), dot(m[2], v)] }
/// The inverse turn (the transpose).
pub(super) fn unturn(m: &[V3; 3], v: V3) -> V3 {
    [m[0][0]*v[0]+m[1][0]*v[1]+m[2][0]*v[2], m[0][1]*v[0]+m[1][1]*v[1]+m[2][1]*v[2], m[0][2]*v[0]+m[1][2]*v[1]+m[2][2]*v[2]]
}

/// A box in the world: its middle, its three unit axes and half its size along each.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Obb {
    pub c: V3,
    pub a: [V3; 3],
    pub h: V3,
}

impl Obb {
    /// A model box `min..max` at `pos`, scaled and turned, shrunk by `shrink` of its size.
    pub fn new(min: V3, max: V3, pos: V3, scale: f32, q: [f32; 4], shrink: f32) -> Self {
        let m = matrix(q);
        let mid = mul(add(min, max), 0.5 * scale);
        let a = [[m[0][0], m[1][0], m[2][0]], [m[0][1], m[1][1], m[2][1]], [m[0][2], m[1][2], m[2][2]]];
        let h = [0, 1, 2].map(|i| (max[i]-min[i]) * 0.5 * scale * (1.0 - shrink));
        Self { c: add(pos, turn(&m, mid)), a, h }
    }

    /// A native doodad's box (`world = origin + axes · v`, scale folded into the axes).
    pub fn native(origin: V3, axes: &[V3; 3], min: V3, max: V3, shrink: f32) -> Self {
        let col = |c: usize| [axes[0][c], axes[1][c], axes[2][c]];
        let cols = [col(0), col(1), col(2)];
        let mid = mul(add(min, max), 0.5);
        let c = add(origin, add(add(mul(cols[0], mid[0]), mul(cols[1], mid[1])), mul(cols[2], mid[2])));
        let a = cols.map(unit);
        let h = [0, 1, 2].map(|i| (max[i]-min[i]) * 0.5 * len(cols[i]) * (1.0 - shrink));
        Self { c, a, h }
    }

    /// Its corners, the middles of its faces and edges, and its middle: where a solid test looks.
    pub fn samples(&self) -> Vec<V3> {
        let mut out = Vec::with_capacity(27);
        for i in [-1.0, 0.0, 1.0] {
            for j in [-1.0, 0.0, 1.0] {
                for k in [-1.0f32, 0.0, 1.0] {
                    out.push(add(self.c, add(add(mul(self.a[0], i*self.h[0]), mul(self.a[1], j*self.h[1])), mul(self.a[2], k*self.h[2]))));
                }
            }
        }
        out
    }

    pub fn contains(&self, p: V3) -> bool {
        let d = sub(p, self.c);
        (0..3).all(|i| dot(d, self.a[i]).abs() <= self.h[i])
    }

    /// Separating-axis test of two boxes.
    pub fn overlaps(&self, o: &Obb) -> bool {
        let t = sub(o.c, self.c);
        let mut axes: Vec<V3> = Vec::with_capacity(15);
        axes.extend(self.a);
        axes.extend(o.a);
        for a in self.a {
            for b in o.a {
                let c = cross(a, b);
                if len(c) > 1e-4 { axes.push(unit(c)); }
            }
        }
        axes.iter().all(|&l| {
            let ra: f32 = (0..3).map(|i| self.h[i] * dot(self.a[i], l).abs()).sum();
            let rb: f32 = (0..3).map(|i| o.h[i] * dot(o.a[i], l).abs()).sum();
            dot(t, l).abs() <= ra + rb
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_frame_turns_the_model_axes_where_asked_and_boxes_meet_only_when_they_touch() {
        let x = unit([0.0, 0.3, 1.0]);
        let y = unit(cross([1.0, 0.0, 0.0], x));
        let m = matrix(frame(x, y));
        for (got, want) in [(turn(&m, [1.0, 0.0, 0.0]), x), (turn(&m, [0.0, 1.0, 0.0]), y)] {
            assert!(len(sub(got, want)) < 1e-4, "{got:?} {want:?}");
        }
        let a = Obb::new([-1.0; 3], [1.0; 3], [0.0; 3], 1.0, euler(0.0, 0.0, 0.0), 0.0);
        let near = Obb::new([-1.0; 3], [1.0; 3], [1.9, 0.0, 0.0], 1.0, euler(0.0, 0.0, 0.7), 0.0);
        let far = Obb::new([-1.0; 3], [1.0; 3], [3.5, 0.0, 0.0], 1.0, euler(0.0, 0.0, 0.0), 0.0);
        assert!(a.overlaps(&near) && !a.overlaps(&far));
        assert!(a.contains([0.9, -0.9, 0.9]) && !a.contains([1.1, 0.0, 0.0]));
    }
}
