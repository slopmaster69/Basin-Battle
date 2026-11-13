//! The Juggernaut's bulwark on the real deck; no game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::dungeon::RailPiece;
use htl_survival::{Content, Moving, Survival};

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_walk_through_the_rail_on_the_real_deck_stops_at_it() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let hero = s.join("railproof", "Railproof", None);
    let start = s.land().footing([-64.0, -835.5, 41.5]).unwrap();
    s.players.get_mut(&hero).unwrap().last_safe = start;
    s.players.get_mut(&hero).unwrap().pos = start;
    // The rail's line at x -64 (along the deck's edge), and the deck's lip in the game's own
    // yard-wide ground (dmprobe: the deck ends at -840.25).
    let (rail, lip) = (-840.49, -840.25);
    let (mut y, mut held) = (start[1], false);
    while y > -845.0 {
        y -= 0.35;
        s.move_player(hero, [-64.0, y, 41.25], -std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Moving::default() });
        for _ in 0..2 { s.step(0.05); }
        let p = s.players[&hero].pos;
        // Never past the line (the client's own collision keeps a body 0.6 yd in from it, in from
        // the ground's lip; the server alone holds it at the line).
        assert!(p[1] > rail, "on the deck: {p:?}");
        held |= y < rail;
    }
    assert!(held && s.players[&hero].pos[1] < lip + 0.5, "it walked to the line and was held there");
}

/// The deck at a quarter of a yard (the game's own ground is a yard wide): every floor walkable from
/// the captain's home on the ship, its walls, and the lights standing on it.
struct Fine {
    g: htl_map::layers::Layers,
    seen: Vec<bool>,
    lights: Vec<[f32; 3]>,
}

fn fine_deck() -> Fine {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let d = htl_map::deadmines::load_with(std::path::Path::new(&data), 0.25, Some(([-112.0, -846.0], [-30.0, -778.0]))).unwrap();
    let seen = d.ground.reachable([-62.0, -815.0, 42.0], 1.2);
    let lights = d.props.iter()
        .filter(|(m, p, _)| p[2] > 36.0 && p[2] < 46.0 && ["torch", "lantern"].iter().any(|w| m.to_lowercase().contains(w)))
        .map(|(_, p, _)| *p).collect();
    Fine { g: d.ground, seen, lights }
}

impl Fine {
    /// Every floor of the ship's decks walkable from the captain's home: [x, y, z].
    fn deck(&self) -> Vec<[f32; 3]> {
        let (w, h) = self.g.size();
        let mut out = Vec::new();
        for j in 0..h {
            for i in 0..w {
                if let Some(z) = self.reach_at(i, j) {
                    let c = self.g.cell_xy(i, j);
                    out.push([c[0], c[1], z]);
                }
            }
        }
        out
    }

    fn reach_at(&self, i: usize, j: usize) -> Option<f32> {
        self.g.floor_range(i, j).filter(|f| self.seen[*f]).map(|f| self.g.floor(f).z).filter(|z| *z > 36.0 && *z < 46.0).reduce(f32::max)
    }

    /// The deck's open edges: a floor with a neighbour holding nothing to stand on near it, no wall
    /// and nothing over it (a drop).
    fn edges(&self) -> Vec<[f32; 3]> {
        let (w, h) = self.g.size();
        let mut out = Vec::new();
        for j in 0..h {
            for i in 0..w {
                let Some(z) = self.reach_at(i, j) else { continue };
                let drop = [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)].iter().any(|(di, dj)| {
                    let (ni, nj) = (i as i32 + di, j as i32 + dj);
                    if ni < 0 || nj < 0 || ni >= w as i32 || nj >= h as i32 {
                        return false;
                    }
                    let (ni, nj) = (ni as usize, nj as usize);
                    !self.reach_at(ni, nj).is_some_and(|nz| (nz - z).abs() < 2.0)
                        && !self.g.floor_range(ni, nj).any(|f| {
                            let fl = self.g.floor(f);
                            (fl.blocked && (fl.z - z).abs() < 2.5) || fl.z > z + 2.0
                        })
                });
                if drop {
                    let c = self.g.cell_xy(i, j);
                    out.push([c[0], c[1], z]);
                }
            }
        }
        out
    }

    /// The deck's height at p: of the floors walkable from the captain's home under it and up to
    /// `reach` in from it, the nearest p's own height (within a yard), if any.
    fn floor(&self, p: [f32; 3], inward: [f32; 2], reach: f32) -> Option<f32> {
        let mut best: Option<f32> = None;
        for k in 0..=((reach / 0.25) as usize) {
            let off = k as f32 * 0.25;
            let Some((i, j)) = self.g.cell_at(p[0] + inward[0] * off, p[1] + inward[1] * off) else { continue };
            for f in self.g.floor_range(i, j) {
                let fl = self.g.floor(f);
                if self.seen[f] && !fl.blocked && (fl.z - p[2]).abs() < 1.0 && best.map_or(true, |b| (fl.z - p[2]).abs() < (b - p[2]).abs()) {
                    best = Some(fl.z);
                }
            }
        }
        best
    }
}

/// Where people must pass, left open: a box (x from, x to, y from, y to) and why.
const PASSAGES: &[([f32; 4], &str)] = &[
    ([-100.0, -84.0, -802.6, -793.0], "the gangway's head, at the stern-port quarter, and the walkway's corner it comes up to"),
    ([-51.6, -45.0, -801.6, -796.0], "the steps between the main deck and the port walkway, at the trench's north end"),
    ([-49.6, -45.0, -804.2, -801.6], "the head of those steps, on the main deck"),
    ([-62.5, -45.0, -800.0, -792.5], "the port walkway's rim over the trench: the boarding route and the Blackguards' way off (#308)"),
];

/// Edges that are pockets in the deck beside a solid structure, not a way over the side: within
/// 1.6 yd of one of these.
const POCKETS: &[([f32; 2], &str)] = &[
    ([-96.75, -814.6], "against VanCleef's cabin, its north-west corner"),
    ([-96.9, -824.8], "against VanCleef's cabin, its south-west corner"),
    ([-43.25, -829.0], "against the forecastle's south face"),
    ([-42.5, -813.2], "against the forecastle's north face"),
];

fn passage(p: [f32; 3]) -> bool {
    PASSAGES.iter().any(|([x0, x1, y0, y1], _)| (*x0..=*x1).contains(&p[0]) && (*y0..=*y1).contains(&p[1]))
}

/// The plank's outer face stands this far out from its line (ExcavationBarrierPlank's hull, x -0.28
/// to 0.22 at 0.85).
const PLANK_OUT: f32 = 0.19;

/// Where p stands to the bulwark: of the pieces it lies beside (along one, within 3 yd of its line
/// and a floor's height), how far in from the nearest one's line it is (negative: out past it),
/// and whether it is in from any of them within 1.5 yd (a piece reaching back over the deck from a
/// corner has deck out past it that the piece before encloses).
fn beside(pieces: &[RailPiece], p: [f32; 3]) -> Option<(f32, bool)> {
    let mut nearest: Option<f32> = None;
    let mut held = false;
    for q in pieces {
        let (dx, dy) = (q.to[0] - q.from[0], q.to[1] - q.from[1]);
        let l2 = dx * dx + dy * dy;
        let t = ((p[0] - q.from[0]) * dx + (p[1] - q.from[1]) * dy) / l2;
        if !(-0.01..=1.01).contains(&t) || (p[2] - (q.from[2] + (q.to[2] - q.from[2]) * t)).abs() > 3.0 {
            continue;
        }
        let side = (dx * (p[1] - q.from[1]) - dy * (p[0] - q.from[0])) / l2.sqrt();
        if side.abs() < 3.0 && nearest.map_or(true, |n| side.abs() < n.abs()) {
            nearest = Some(side);
        }
        held |= (0.0..1.5).contains(&side);
    }
    nearest.map(|n| (n, held))
}

fn pieces(s: &Survival) -> Vec<RailPiece> {
    s.objs.keys().filter_map(|id| s.rail_piece(*id)).collect()
}

/// Each piece stands on the deck and follows it, measured on the deck at a quarter of a yard at
/// every eighth of its length: its top 1.8 yd or more over the deck (to a hundredth; a hero's 1 yd step up and a
/// 0.23 yd hop back clear neither, by half a yard), and no more than 0.2 yd of open space between
/// the deck and its foot,
/// or the foot of the board under it where the deck falls away (the bow's slope). Where a piece
/// spans no deck (over the gangway, from a corner or into the forecastle), there is nothing to
/// stand on to step over it, nor any deck to show through under it.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_piece_of_the_bulwark_stands_on_the_real_deck_and_tops_a_step_up() {
    let s = real_deck();
    let fine = fine_deck();
    let rails = s.content.dungeon.as_ref().unwrap().rails.clone();
    let pieces = pieces(&s);
    assert!(pieces.len() >= 30, "{} pieces", pieces.len());
    let boards: Vec<_> = pieces.iter().filter(|p| p.under).collect();
    assert!(!boards.is_empty(), "a board under the bow's slope");
    let (mut measured, mut worst_top, mut worst_gap) = (0, f32::MAX, f32::MIN);
    for p in pieces.iter().filter(|p| !p.under) {
        let r = &rails[p.rail];
        let (bottom, top) = (r.lift + r.height[0] * r.scale, r.lift + r.height[1] * r.scale);
        let span = ((p.to[0] - p.from[0]).powi(2) + (p.to[1] - p.from[1]).powi(2)).sqrt();
        let inward = [-(p.to[1] - p.from[1]) / span, (p.to[0] - p.from[0]) / span];
        // The board under it, if any: the same chord, lowered.
        let board = boards.iter().find(|q| {
            ((q.from[0] - p.from[0]).abs() + (q.from[1] - p.from[1]).abs() + (q.to[0] - p.to[0]).abs() + (q.to[1] - p.to[1]).abs()) < 0.01
        });
        let drop = board.map_or(0.0, |q| p.from[2] - q.from[2]);
        for n in 0..=8 {
            let t = n as f32 / 8.0;
            let at = [p.from[0] + (p.to[0] - p.from[0]) * t, p.from[1] + (p.to[1] - p.from[1]) * t, p.from[2] + (p.to[2] - p.from[2]) * t];
            // The deck under it or, along the deck's edge, just in from it.
            let Some(floor) = fine.floor(at, inward, 0.5) else { continue };
            measured += 1;
            let (foot, head) = (at[2] + bottom - drop, at[2] + top);
            println!("{} piece {:?} -> {:?} at {t}: deck {floor:.2}, foot {foot:.2}, top {head:.2}", r.key, p.from, p.to);
            worst_top = worst_top.min(head - floor);
            worst_gap = worst_gap.max(foot - floor);
            assert!(foot <= floor + 0.2, "{}: {:.2} yd open under it at ({}, {})", r.key, foot - floor, at[0], at[1]);
            assert!(head - floor >= 1.79, "{}: its top stands {:.2} over the deck at ({}, {})", r.key, head - floor, at[0], at[1]);
        }
    }
    println!("lowest top over the deck {worst_top:.2}, widest gap under a foot {worst_gap:.2}");
    let uppers = pieces.len() - boards.len();
    assert!(measured >= uppers * 5, "{measured} measured of {uppers} pieces");
}

/// The gangway's walking path, from its foot on the lower deck to the head where it meets the
/// deck, stays clear of every piece's collision hull (the hull the clients collide with, read from
/// the game's own model, placed, turned and scaled as the clients place it) by a body's width
/// (0.8 yd) at a body's height (2 yd) over every floor of it (take 381: a hero climbing it was
/// stopped short of its head).
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_gangway_stays_clear_of_every_piece_of_the_bulwark_by_a_body() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let s = real_deck();
    let mut chain = benilla_formats::Chain::open(std::path::Path::new(&data)).unwrap();
    let catalog = benilla_formats::load_gameobject_catalog(&mut chain).unwrap();
    // Each piece's hull as a box in its own frame, scaled, with its place and turn.
    let mut boxes = Vec::new();
    for (id, o) in &s.objs {
        let Some(p) = s.rail_piece(*id) else { continue };
        let htl_survival::ObjKind::Prop { display, scale } = o.kind else { continue };
        let path = catalog.model_path(display).expect("a model").to_string();
        let hull = benilla_formats::load_m2_collision_hull(&mut chain, &path).unwrap();
        assert!(!hull.is_empty(), "{display} has a hull");
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &hull.positions {
            for k in 0..3 {
                lo[k] = lo[k].min(v[k] * scale);
                hi[k] = hi[k].max(v[k] * scale);
            }
        }
        boxes.push((p.rotation, o.pos, lo, hi));
    }
    let turn = |q: [f32; 4], v: [f32; 3]| {
        let u = [q[0], q[1], q[2]];
        let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
        let t = cross(u, v).map(|c| 2.0 * c);
        let ut = cross(u, t);
        [v[0] + q[3] * t[0] + ut[0], v[1] + q[3] * t[1] + ut[1], v[2] + q[3] * t[2] + ut[2]]
    };
    // How near a point comes to a hull (0 inside it).
    let near = |w: [f32; 3]| {
        boxes.iter().map(|(q, pos, lo, hi)| {
            let l = turn([-q[0], -q[1], -q[2], q[3]], [w[0] - pos[0], w[1] - pos[1], w[2] - pos[2]]);
            (0..3).map(|k| (lo[k] - l[k]).max(l[k] - hi[k]).max(0.0).powi(2)).sum::<f32>().sqrt()
        }).fold(f32::MAX, f32::min)
    };
    // The gangway at a quarter of a yard: every floor walkable from its foot in the stern-port
    // quarter's box, from its foot (-98, -804) up to its head and the walkway beyond it.
    let d = htl_map::deadmines::load_with(std::path::Path::new(&data), 0.25, Some(([-102.0, -808.0], [-80.0, -792.0]))).unwrap();
    let g = &d.ground;
    let seen = g.reachable([-98.0, -804.0, 31.0], 1.2);
    let (w, h) = g.size();
    let (mut cells, mut worst) = (0, (f32::MAX, [0.0f32; 3]));
    for j in 0..h {
        for i in 0..w {
            for f in g.floor_range(i, j).filter(|f| seen[*f]) {
                let z = g.floor(f).z;
                // The gangway itself: below the deck it climbs to.
                if !(29.0..38.2).contains(&z) {
                    continue;
                }
                let c = g.cell_xy(i, j);
                cells += 1;
                for dz in [0.1f32, 0.7, 1.3, 2.0] {
                    let n = near([c[0], c[1], z + dz]);
                    if n < worst.0 {
                        worst = (n, [c[0], c[1], z]);
                    }
                }
            }
        }
    }
    println!("{cells} quarter-yards of gangway; the nearest a piece comes is {:.2} yd, at {:?}", worst.0, worst.1);
    assert!(cells > 400, "{cells} quarter-yards of gangway");
    assert!(worst.0 >= 0.8, "a piece {:.2} yd from a body on the gangway at {:?}", worst.0, worst.1);
}

/// No walkable deck lies outside the bulwark (out past its outer face, beside a piece) but at the
/// passages; every light on the deck stands inside it, half a yard in; and the bow's point is
/// inside: the two pieces meeting there have its tip, and the torch on it, in from them.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn no_deck_lies_outside_the_bulwark_but_at_the_passages() {
    let s = real_deck();
    let fine = fine_deck();
    let pieces = pieces(&s);
    let mut out = Vec::new();
    let deck = fine.deck();
    assert!(deck.len() > 20_000, "{} deck cells", deck.len());
    for p in &deck {
        if passage(*p) {
            continue;
        }
        if let Some((side, held)) = beside(&pieces, *p) {
            if side < -PLANK_OUT - 0.01 && !held {
                out.push(*p);
            }
        }
    }
    assert!(out.is_empty(), "{} quarter-yards of deck outside the bulwark: {:?}", out.len(), &out[..out.len().min(40)]);
    assert!(fine.lights.len() >= 15, "{} lights", fine.lights.len());
    for l in &fine.lights {
        let b = beside(&pieces, *l);
        assert!(b.map_or(true, |(side, held)| side >= 0.45 || held && side > -PLANK_OUT), "a light out at the bulwark: {l:?} {b:?}");
    }
    // The bow's point: its tip (the deck's last quarter-yard there) and the torch on it, in from
    // both pieces that meet at the point.
    let tip = [-34.25, -794.75, 39.4];
    let torch = *fine.lights.iter().find(|l| (l[0] + 34.5).abs() < 0.1 && (l[1] + 794.4).abs() < 0.1).expect("the bow's torch");
    let point = [-33.33, -794.64];
    let at_point: Vec<_> = pieces.iter().filter(|q| [q.from, q.to].iter().any(|e| ((e[0] - point[0]).powi(2) + (e[1] - point[1]).powi(2)).sqrt() < 0.05)).collect();
    assert_eq!(at_point.len(), 2, "two pieces meet at the bow's point");
    for p in [tip, torch] {
        for q in &at_point {
            let (dx, dy) = (q.to[0] - q.from[0], q.to[1] - q.from[1]);
            let side = (dx * (p[1] - q.from[1]) - dy * (p[0] - q.from[0])) / (dx * dx + dy * dy).sqrt();
            assert!(side > 0.2, "{p:?} in from the piece {:?} -> {:?}: {side}", q.from, q.to);
        }
    }
}

/// Every open edge of the ship's decks has a piece beside it (the edge no further in than a yard,
/// nor out past the plank's face), or is a passage people must use, or a pocket beside a solid
/// structure.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_open_edge_of_the_ship_is_railed_or_a_passage() {
    let s = real_deck();
    let fine = fine_deck();
    let pieces = pieces(&s);
    let edges = fine.edges();
    assert!(edges.len() > 500, "{} edge cells", edges.len());
    let mut open = Vec::new();
    for e in &edges {
        let pocket = POCKETS.iter().any(|(p, _)| ((p[0] - e[0]).powi(2) + (p[1] - e[1]).powi(2)).sqrt() <= 1.6);
        let railed = beside(&pieces, *e).is_some_and(|(side, _)| (-0.25..=1.0).contains(&side));
        if !(railed || passage(*e) || pocket) {
            open.push(*e);
        }
    }
    assert!(open.is_empty(), "{} open edges with no bulwark: {:?}", open.len(), &open[..open.len().min(40)]);
}

fn real_deck() -> Survival {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    Survival::new_dungeon(c, Box::new(land), 17, 0.0)
}

/// The Blackguards' walk off the ship (VanCleef's, #308) and a hero's way aboard up the boarding
/// ramp, and back down it, cross no rail.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_blackguards_walk_off_and_the_way_aboard_cross_no_rail() {
    let s = real_deck();
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    // The same ground, for walks longer than a mob plans.
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let at = |x: f32, y: f32, z: f32| s.land().footing([x, y, z]).unwrap();
    // VanCleef's home (-77, -820): his Blackguards' posts and their way off (vancleef.rs).
    let exit = at(-54.0, -793.0, 39.5);
    // Halfway up the boarding ramp, 5 yd under the deck (the rails are all at deck height).
    let (landing, deck) = (at(-94.4, -800.1, 33.1), at(-62.0, -815.0, 42.0));
    let walks = [(at(-64.0, -808.0, 42.0), exit), (at(-68.0, -807.5, 42.0), exit), (landing, deck), (deck, landing)];
    for (from, to) in walks {
        let path = land.walk(from, to).unwrap_or_else(|| panic!("a way from {from:?} to {to:?}"));
        assert!(!path.is_empty());
        let mut last = from;
        for p in path {
            assert!(!s.through_rail(last, p), "{from:?} -> {to:?}: over a rail between {last:?} and {p:?}");
            last = p;
        }
    }
}

/// On the real deck, a hero stepping over any section of any rail is held on the deck.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_server_stop_holds_at_every_section_on_the_real_deck() {
    let base = real_deck();
    let rails = base.content.dungeon.as_ref().unwrap().rails.clone();
    let (mut tried, sections) = (0, rails.iter().map(|r| r.points.len() - 1).sum::<usize>());
    for r in &rails {
        for w in r.points.windows(2) {
            let mid = [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0, (w[0][2] + w[1][2]) / 2.0];
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let l = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = (-dy / l, dx / l);
            // (A yard in: the line runs along the deck's very edge.)
            let Some(inside) = base.land().footing([mid[0] + nx * 1.0, mid[1] + ny * 1.0, mid[2] + 0.5]).filter(|f| (f[2] - mid[2]).abs() < 1.5) else { continue };
            tried += 1;
            let outside = [mid[0] - nx * 2.0, mid[1] - ny * 2.0, inside[2]];
            let mut s = real_deck();
            let h = s.join("railproof", "Railproof", None);
            s.players.get_mut(&h).unwrap().last_safe = inside;
            s.players.get_mut(&h).unwrap().pos = inside;
            s.move_player(h, outside, 0.0, Moving::default());
            let p = s.players[&h].pos;
            assert!((p[0] - mid[0]) * nx + (p[1] - mid[1]) * ny > 0.0, "{}: held on the deck at {:?} -> {:?}: {p:?}", r.key, w[0], w[1]);
        }
    }
    assert!(tried * 4 >= sections * 3, "{tried} of {sections} sections tried");
}
