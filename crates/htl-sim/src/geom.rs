//! Positions in WoW world space: x and y on the ground, z up, yards.

pub type Vec3 = [f32; 3];

pub fn dist2d(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}

pub fn facing(from: Vec3, to: Vec3) -> f32 {
    let a = (to[1] - from[1]).atan2(to[0] - from[0]);
    if a < 0.0 {
        a + std::f32::consts::TAU
    } else {
        a
    }
}

/// The point `d` yards from `from` towards `to` on the ground plane (z copied from `from`).
pub fn toward(from: Vec3, to: Vec3, d: f32) -> Vec3 {
    let len = dist2d(from, to);
    if len < 1e-4 {
        return from;
    }
    let t = d / len;
    [
        from[0] + (to[0] - from[0]) * t,
        from[1] + (to[1] - from[1]) * t,
        from[2],
    ]
}

/// Length of a path on the ground plane.
pub fn path_len(path: &[Vec3]) -> f32 {
    path.windows(2).map(|w| dist2d(w[0], w[1])).sum()
}

/// The ground height under a point. The game server backs this with the arena's real terrain;
/// tests use [`Flat`].
pub trait Ground: Send + Sync {
    fn height(&self, x: f32, y: f32) -> f32;
}

/// Level ground at one height.
pub struct Flat(pub f32);

impl Ground for Flat {
    fn height(&self, _x: f32, _y: f32) -> f32 {
        self.0
    }
}

/// A straight walk from `from` to `to`, with a point every `step` yards set on the ground, so a
/// mover follows the terrain instead of cutting through hills.
pub fn ground_path(ground: &dyn Ground, from: Vec3, to: Vec3, step: f32) -> Vec<Vec3> {
    let len = dist2d(from, to);
    let n = (len / step).ceil().max(1.0) as usize;
    let mut out = Vec::with_capacity(n + 1);
    out.push(from);
    for i in 1..=n {
        let t = i as f32 / n as f32;
        let x = from[0] + (to[0] - from[0]) * t;
        let y = from[1] + (to[1] - from[1]) * t;
        out.push([x, y, ground.height(x, y)]);
    }
    out
}
