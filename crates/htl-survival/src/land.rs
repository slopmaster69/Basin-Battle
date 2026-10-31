//! What the world is made of, as the rules need it: the ground's height, what can't be walked or
//! shot through, water and roads, and a way around for long walks. The server backs it with
//! Elwynn's real terrain, buildings and trees; tests use [`FlatLand`].

use htl_sim::geom::{dist2d, Vec3};

pub trait Land: Send + Sync {
    fn height(&self, x: f32, y: f32) -> f32;
    /// The ground's height at (x, y) for a walker coming from height `z`: on ground with floors
    /// above floors (a deck over a hold), the floor they'd be on. One floor everywhere by default.
    fn height_near(&self, x: f32, y: f32, _z: f32) -> f32 {
        self.height(x, y)
    }
    /// [`Land::blocked`] for a walker at height `z`.
    fn blocked_near(&self, x: f32, y: f32, _z: f32) -> bool {
        self.blocked(x, y)
    }
    fn water_near(&self, x: f32, y: f32, _z: f32) -> bool {
        self.water(x, y)
    }
    /// A floor (a deck, a ceiling) lies between heights `a` and `b` over (x, y).
    fn slab_between(&self, _x: f32, _y: f32, _a: f32, _b: f32) -> bool {
        false
    }
    /// A wall, a tree trunk or a building stands here.
    fn blocked(&self, _x: f32, _y: f32) -> bool {
        false
    }
    fn water(&self, _x: f32, _y: f32) -> bool {
        false
    }
    /// How much of the ground here is road, 0 to 1.
    fn road(&self, _x: f32, _y: f32) -> f32 {
        0.0
    }
    /// A walk from `from` to `to` around what's in the way, if the land knows one.
    fn path(&self, _from: Vec3, _to: Vec3) -> Option<Vec<Vec3>> {
        None
    }
    /// Whether [`Land::path`] searches: a land that does, finding no way, means there is none.
    fn plans(&self) -> bool {
        false
    }
    /// Tree trunks, for placing timber beside them.
    fn trees(&self) -> Vec<Vec3> {
        Vec::new()
    }
    /// Whether a walker can go straight from `from` to `to` on foot: nothing solid in the way and
    /// no ground too steep to climb (a creature's straight walk; the rest go by [`Land::path`]).
    /// By default, the line of sight's own test.
    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool {
        clear_line_on(self, from, to)
    }
    /// The land's extent (its south-east and north-west corners), for spreading what's laid out
    /// across all of it; `None` for a land with no edge.
    fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        None
    }
    /// Whether the ground here is the zone's own (not a neighbour's at its edge).
    fn in_zone(&self, _x: f32, _y: f32) -> bool {
        true
    }
}

/// Open level ground, with optional walls (circles) for tests.
#[derive(Default)]
pub struct FlatLand {
    pub z: f32,
    pub walls: Vec<([f32; 2], f32)>,
    pub roads: Vec<([f32; 2], f32)>,
    pub trees: Vec<Vec3>,
    /// An edge, for what's spread across the land (none by default).
    pub bounds: Option<([f32; 2], [f32; 2])>,
}

impl Land for FlatLand {
    fn height(&self, _x: f32, _y: f32) -> f32 {
        self.z
    }
    fn blocked(&self, x: f32, y: f32) -> bool {
        self.walls.iter().any(|(c, r)| dist2d([c[0], c[1], 0.0], [x, y, 0.0]) <= *r)
    }
    fn road(&self, x: f32, y: f32) -> f32 {
        if self.roads.iter().any(|(c, r)| dist2d([c[0], c[1], 0.0], [x, y, 0.0]) <= *r) {
            1.0
        } else {
            0.0
        }
    }
    fn trees(&self) -> Vec<Vec3> {
        self.trees.clone()
    }
    fn bounds(&self) -> Option<([f32; 2], [f32; 2])> {
        self.bounds
    }
}

/// Whether nothing solid stands between `a` and `b` (eye height above each): walls and trunks by
/// the land, and ground rising above the line.
pub fn clear_line(land: &dyn Land, a: Vec3, b: Vec3) -> bool {
    clear_line_on(land, a, b)
}

fn clear_line_on<L: Land + ?Sized>(land: &L, a: Vec3, b: Vec3) -> bool {
    let d = dist2d(a, b);
    let steps = (d / 0.5).ceil().max(1.0) as usize;
    let (za, zb) = (a[2] + 1.5, b[2] + 1.0);
    let mut last = za;
    // The very ends belong to the two bodies, not to what's between them.
    for k in 1..steps {
        let t = k as f32 / steps as f32;
        let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
        let along = t * d;
        if along < 0.6 || d - along < 0.6 {
            continue;
        }
        let z = za + (zb - za) * t;
        if land.blocked_near(x, y, z - 1.2) {
            return false;
        }
        if land.height_near(x, y, z - 1.2) > z || land.slab_between(x, y, last, z) {
            return false;
        }
        last = z;
    }
    true
}
