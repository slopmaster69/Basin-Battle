//! The Juggernaut's starboard rail on the real deck; no game assets stored here.
use htl_server::survival::land::DeadminesLand;
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
    // The rail's line at x -64, and the deck's lip beyond it (dmprobe: the deck ends at -840.25).
    let (rail, lip) = (-839.45, -840.25);
    let mut y = start[1];
    while y > -845.0 {
        y -= 0.35;
        s.move_player(hero, [-64.0, y, 41.25], -std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Moving::default() });
        for _ in 0..2 { s.step(0.05); }
        let p = s.players[&hero].pos;
        assert!(p[1] > rail && p[1] > lip, "on the deck: {p:?}");
    }
}

/// Each piece stands on the deck and follows it: a straight chord of the rail's line, its ends and
/// middle on the deck, its base sunk into the deck and its top at least 1.73 yd over it at both
/// ends and the middle (a hero's 1 yd step up and a 0.23 yd hop back clear neither, by half a yard).
/// The one exception is the bow's slope, where the deck falls 3.9 yd in a piece's length and twists
/// as it falls: no straight piece lies flush there (every placement tried along its edge leaves
/// the deck up to 0.5 yd off its middle, above or below), so its piece stands on its ends and
/// clears the deck by up to 0.4 yd at its middle, its top 2.3 yd up there; nothing gets under that.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_piece_of_the_rail_stands_on_the_real_deck_and_tops_a_step_up() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    let s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let rails = s.content.dungeon.as_ref().unwrap().rails.clone();
    let pieces: Vec<_> = s.objs.keys().filter_map(|id| s.rail_piece(*id)).collect();
    assert!(pieces.len() >= 15, "{} pieces", pieces.len());
    for p in pieces {
        let r = &rails[p.rail];
        let (bottom, top) = (r.lift + r.height[0] * r.scale, r.lift + r.height[1] * r.scale);
        let mid = [(p.from[0] + p.to[0]) / 2.0, (p.from[1] + p.to[1]) / 2.0, (p.from[2] + p.to[2]) / 2.0];
        let slope = (p.to[2] - p.from[2]).abs() > 3.0;
        for at in [p.from, mid, p.to] {
            let clear = if slope && at == mid { 0.4 } else { 0.05 };
            let floor = s.land().height_near(at[0], at[1], at[2] + 0.5);
            println!("{} piece {:?} -> {:?} at ({:.1}, {:.1}): deck {floor:.2}, base {:.2}, top {:.2}", r.key, p.from, p.to, at[0], at[1], at[2] + bottom, at[2] + top);
            // The trench's invisible wall runs through a crate and a block on the rim: held only to
            // its base under whatever it crosses and its top over what a hero can stand on (the
            // crate; the block's top is 2 yd and more over the deck, out of reach).
            let trench = r.key == "juggernaut_trench";
            assert!(trench || (floor - at[2]).abs() < 0.6, "on the deck at ({}, {}): {floor} vs {}", at[0], at[1], at[2]);
            assert!(at[2] + bottom <= floor + clear, "its base reaches the deck at ({}, {})", at[0], at[1]);
            let reach = !trench || floor - at[2] < 1.5;
            assert!(!reach || at[2] + top - floor >= 1.73, "its top stands {:.2} over the deck at ({}, {})", at[2] + top - floor, at[0], at[1]);
        }
    }
}

/// The open edges of the Juggernaut's decks, as surveyed on 12 Nov 2026 (every floor walkable from
/// the captain's home, each with a neighbour holding no floor within 2 yd of it, no wall and nothing
/// raised): one cell a yard, [x, y, z].
const OPEN_EDGES: &[[f32; 3]] = &[[-108.0, -825.0, 37.7], [-108.0, -824.5, 37.7], [-108.0, -823.0, 37.7], [-108.0, -822.5, 37.7], [-108.0, -821.0, 37.7], [-108.0, -820.5, 37.7], [-107.5, -826.5, 37.7], [-107.5, -819.0, 37.7], [-107.5, -818.5, 37.7], [-107.5, -817.0, 37.7], [-107.5, -816.5, 37.7], [-107.5, -815.0, 37.7], [-107.5, -814.5, 37.7], [-107.0, -826.5, 37.8], [-106.5, -827.0, 37.8], [-106.5, -812.0, 37.8], [-105.5, -827.5, 37.9], [-105.0, -828.0, 37.9], [-105.0, -811.0, 37.9], [-104.5, -811.0, 37.9], [-104.0, -828.5, 38.0], [-104.0, -803.5, 30.6], [-104.0, -803.0, 30.3], [-104.0, -802.5, 30.1], [-103.5, -810.5, 38.8], [-103.0, -829.0, 38.0], [-101.5, -830.0, 38.2], [-101.5, -809.0, 38.1], [-101.0, -830.0, 38.2], [-101.0, -809.0, 38.2], [-100.5, -830.5, 38.2], [-100.5, -808.5, 38.2], [-99.5, -831.0, 38.3], [-99.0, -831.0, 38.3], [-99.0, -808.0, 38.3], [-98.5, -807.5, 38.3], [-97.5, -832.0, 38.4], [-97.5, -799.0, 30.4], [-97.5, -798.5, 30.2], [-97.0, -832.5, 38.5], [-97.0, -814.5, 38.5], [-97.0, -807.0, 38.4], [-97.0, -799.5, 30.7], [-96.5, -832.5, 38.5], [-96.5, -814.5, 38.5], [-96.5, -807.0, 38.5], [-96.5, -800.0, 32.0], [-96.0, -833.0, 38.6], [-96.0, -806.5, 38.5], [-95.5, -815.0, 38.6], [-95.0, -833.5, 38.6], [-95.0, -806.0, 38.6], [-95.0, -799.5, 33.0], [-94.5, -806.0, 38.6], [-94.5, -799.0, 33.4], [-94.0, -834.0, 38.7], [-93.5, -798.5, 34.1], [-93.0, -834.0, 38.8], [-93.0, -798.5, 34.4], [-92.5, -834.5, 38.8], [-92.5, -798.0, 34.9], [-92.5, -796.5, 35.3], [-92.0, -797.0, 35.2], [-91.5, -835.0, 38.9], [-91.0, -835.0, 38.9], [-91.0, -795.5, 35.3], [-90.5, -835.0, 39.0], [-90.5, -795.5, 35.4], [-90.0, -835.5, 39.0], [-89.0, -836.0, 39.1], [-89.0, -795.0, 36.1], [-88.5, -795.0, 36.4], [-87.5, -836.5, 39.2], [-87.0, -836.5, 39.3], [-87.0, -795.0, 37.0], [-86.5, -837.0, 39.3], [-86.5, -795.0, 37.2], [-86.0, -794.5, 37.5], [-85.0, -837.5, 39.4], [-85.0, -794.5, 37.9], [-85.0, -791.5, 38.4], [-85.0, -791.0, 38.4], [-85.0, -790.5, 38.4], [-85.0, -787.5, 40.4], [-84.5, -794.5, 38.1], [-84.5, -793.0, 38.3], [-84.5, -792.5, 38.4], [-84.5, -787.0, 40.4], [-84.5, -786.5, 40.4], [-83.5, -838.0, 39.5], [-83.5, -790.0, 40.1], [-83.5, -788.5, 40.1], [-83.0, -838.0, 39.6], [-83.0, -788.0, 39.8], [-83.0, -786.0, 40.9], [-82.5, -788.5, 39.8], [-82.0, -787.0, 39.8], [-81.5, -838.5, 39.7], [-81.5, -786.0, 40.0], [-81.0, -838.5, 39.7], [-81.0, -786.5, 39.9], [-80.5, -838.5, 39.8], [-80.5, -785.0, 38.9], [-80.0, -839.0, 39.8], [-79.0, -839.0, 39.9], [-78.5, -839.5, 40.0], [-78.5, -785.0, 39.0], [-78.0, -784.5, 39.0], [-77.0, -839.5, 40.1], [-77.0, -784.5, 39.1], [-76.5, -840.0, 40.1], [-76.5, -784.5, 39.1], [-75.0, -840.0, 40.3], [-75.0, -784.5, 39.3], [-74.5, -840.0, 40.3], [-74.5, -784.5, 39.3], [-74.0, -804.0, 41.4], [-73.5, -803.0, 42.1], [-73.0, -840.0, 40.4], [-73.0, -784.0, 39.5], [-72.5, -840.0, 40.5], [-72.5, -784.0, 39.6], [-71.0, -840.0, 40.6], [-71.0, -784.0, 39.8], [-70.5, -840.5, 40.6], [-70.5, -784.0, 39.8], [-69.0, -840.5, 40.8], [-68.5, -840.5, 40.8], [-68.0, -784.0, 40.1], [-67.0, -840.5, 41.0], [-67.0, -784.0, 40.0], [-66.5, -840.5, 41.0], [-66.5, -784.0, 40.0], [-65.5, -801.5, 42.7], [-65.0, -840.5, 41.1], [-65.0, -802.5, 42.2], [-65.0, -784.0, 39.9], [-64.5, -840.0, 41.2], [-64.5, -803.0, 42.3], [-64.5, -802.5, 42.3], [-64.5, -784.0, 39.8], [-63.0, -840.0, 41.3], [-63.0, -784.5, 39.7], [-62.5, -840.0, 41.4], [-62.5, -784.5, 39.7], [-62.0, -802.5, 42.7], [-62.0, -799.0, 40.0], [-61.5, -805.0, 42.4], [-61.0, -840.0, 41.5], [-61.0, -805.5, 42.6], [-61.0, -799.5, 40.3], [-61.0, -799.0, 40.4], [-61.0, -784.5, 39.6], [-60.5, -840.0, 41.6], [-60.5, -805.0, 43.0], [-60.5, -798.0, 39.4], [-60.5, -797.0, 38.8], [-60.5, -796.5, 38.9], [-60.5, -784.5, 39.5], [-60.0, -795.0, 38.9], [-60.0, -794.5, 39.0], [-59.0, -840.0, 41.7], [-59.0, -793.5, 39.0], [-59.0, -784.5, 39.4], [-58.5, -839.5, 41.7], [-58.5, -784.5, 39.4], [-58.0, -800.5, 41.7], [-58.0, -785.0, 39.3], [-57.5, -801.0, 41.8], [-57.5, -794.0, 38.8], [-57.0, -839.5, 41.9], [-57.0, -801.0, 41.8], [-57.0, -794.0, 38.8], [-57.0, -785.0, 39.2], [-56.5, -839.0, 41.9], [-56.5, -801.0, 41.9], [-56.5, -794.0, 38.8], [-56.5, -785.0, 39.2], [-56.0, -801.5, 41.9], [-55.5, -785.5, 39.1], [-55.0, -838.5, 42.1], [-55.0, -801.5, 42.0], [-55.0, -794.5, 38.6], [-55.0, -785.5, 39.1], [-54.5, -838.5, 42.1], [-54.5, -802.0, 42.1], [-54.5, -794.5, 38.6], [-54.5, -785.5, 39.0], [-53.0, -838.0, 42.3], [-53.0, -802.0, 42.2], [-53.0, -794.5, 38.5], [-53.0, -786.0, 38.9], [-52.5, -838.0, 42.3], [-52.5, -802.5, 42.3], [-52.5, -795.0, 38.5], [-52.5, -786.0, 38.8], [-51.5, -803.0, 42.4], [-51.5, -795.5, 38.7], [-51.0, -837.5, 42.5], [-51.0, -795.5, 38.7], [-51.0, -786.5, 38.7], [-50.5, -837.0, 42.5], [-50.5, -796.0, 38.8], [-50.0, -786.5, 38.6], [-49.0, -836.5, 42.7], [-49.0, -796.5, 38.9], [-49.0, -787.0, 38.6], [-48.5, -836.5, 42.7], [-48.5, -796.5, 39.0], [-48.5, -787.0, 38.7], [-48.0, -797.0, 39.0], [-47.5, -801.5, 41.3], [-47.5, -787.5, 38.8], [-47.0, -836.0, 42.9], [-47.0, -801.0, 41.0], [-47.0, -800.5, 40.8], [-47.0, -797.0, 39.1], [-47.0, -787.5, 38.9], [-46.5, -835.5, 42.9], [-46.5, -800.0, 40.4], [-46.5, -788.5, 39.9], [-46.0, -835.0, 43.0], [-46.0, -799.0, 39.9], [-45.5, -834.5, 43.0], [-45.5, -798.5, 39.5], [-45.0, -834.5, 43.1], [-45.0, -788.5, 39.1], [-44.5, -834.0, 43.1], [-44.5, -788.5, 39.1], [-44.0, -829.5, 43.1], [-44.0, -811.5, 43.1], [-44.0, -789.0, 39.2], [-43.5, -833.0, 43.2], [-43.0, -832.5, 43.3], [-43.0, -829.0, 43.2], [-43.0, -789.0, 39.3], [-42.5, -832.0, 43.3], [-42.5, -828.5, 43.3], [-42.5, -808.5, 43.3], [-42.0, -809.0, 43.3], [-42.0, -807.0, 42.6], [-42.0, -806.5, 42.3], [-41.5, -809.5, 43.4], [-41.5, -790.0, 39.4], [-41.0, -831.0, 43.5], [-41.0, -805.0, 41.2], [-41.0, -804.5, 41.0], [-41.0, -790.0, 39.4], [-40.5, -830.5, 43.5], [-40.5, -804.0, 40.6], [-40.5, -790.5, 39.4], [-40.0, -803.0, 39.9], [-40.0, -791.0, 39.4], [-39.5, -829.0, 43.6], [-39.5, -802.5, 39.5], [-39.0, -828.5, 43.7], [-39.0, -812.0, 43.6], [-39.0, -801.5, 39.3], [-39.0, -791.5, 39.4], [-38.5, -813.0, 43.7], [-38.5, -812.5, 43.7], [-38.5, -801.0, 39.3], [-38.5, -800.5, 39.3], [-37.5, -799.0, 39.3], [-37.5, -792.5, 39.4], [-37.0, -798.5, 39.3], [-37.0, -793.0, 39.4], [-36.5, -798.0, 39.3], [-36.5, -793.0, 39.4], [-36.0, -797.0, 39.3], [-36.0, -793.5, 39.4], [-35.5, -796.5, 39.4], [-35.0, -795.5, 39.4], [-35.0, -794.0, 39.4], [-34.5, -795.0, 39.4]];

/// Where people must pass, left open: a box (x from, x to, y from, y to) and why.
const PASSAGES: &[([f32; 4], &str)] = &[
    ([-108.5, -79.5, -806.5, -778.0], "the boarding ramp up the stern-port quarter"),
    ([-51.6, -45.0, -801.6, -796.0], "the steps between the main deck and the port walkway, at the trench's north end"),
    ([-62.5, -45.0, -800.0, -792.5], "the port walkway's rim over the trench: the boarding route and the Blackguards' way off (#308)"),
];

/// Edge cells that are pockets in the land's floor beside a solid structure (a mast, cannons, the
/// forecastle: the bow's works, its top 8 to 10 yd up and its sides walls), not a way over the
/// side: within 1.6 yd of one of these.
const POCKETS: &[([f32; 2], &str)] = &[
    ([-60.75, -805.0], "beside the block on the trench's rim"),
    ([-73.75, -803.5], "among the port cannons"),
    ([-96.25, -814.75], "behind VanCleef's cabin"),
    ([-43.75, -811.5], "against the forecastle's north side"),
    ([-38.75, -812.5], "against the forecastle's north side"),
    ([-43.25, -829.0], "against the forecastle's south face"),
];

fn dist_to(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = (dx * dx + dy * dy).max(1e-6);
    let t = (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0);
    ((a[0] + dx * t - p[0]).powi(2) + (a[1] + dy * t - p[1]).powi(2)).sqrt()
}

/// Whether p is outboard of the line from a to b (the deck is on its left) and within 3 yd of it,
/// the bulwark between it and the deck: the rim outside the port bitts, which the line passes
/// inboard of, as a straight piece can't bend round them.
fn behind(a: [f32; 3], b: [f32; 3], p: [f32; 3]) -> bool {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = (dx * dx + dy * dy).max(1e-6);
    let t = ((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2;
    let side = dx * (p[1] - a[1]) - dy * (p[0] - a[0]);
    (0.0..=1.0).contains(&t) && side < 0.0 && -side / l2.sqrt() <= 3.0
}

fn real_deck() -> Survival {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    Survival::new_dungeon(c, Box::new(land), 17, 0.0)
}

/// Every open edge of the ship's decks has the bulwark along it (within 1.6 yd of its line, or
/// behind it), or is a passage people must use, or a pocket beside a solid structure.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_open_edge_of_the_ship_is_railed_or_a_passage() {
    let s = real_deck();
    let rails = s.content.dungeon.as_ref().unwrap().rails.clone();
    let mut open = Vec::new();
    for e in OPEN_EDGES {
        let covered = rails.iter().any(|r| r.points.windows(2).any(|w| dist_to(w[0], w[1], *e) <= 1.6 || behind(w[0], w[1], *e)));
        let passage = PASSAGES.iter().any(|([x0, x1, y0, y1], _)| (*x0..=*x1).contains(&e[0]) && (*y0..=*y1).contains(&e[1]));
        let pocket = POCKETS.iter().any(|(p, _)| ((p[0] - e[0]).powi(2) + (p[1] - e[1]).powi(2)).sqrt() <= 1.6);
        if !(covered || passage || pocket) {
            open.push(*e);
        }
    }
    assert!(open.is_empty(), "{} open edges with no bulwark: {open:?}", open.len());
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
    for r in &rails {
        for w in r.points.windows(2) {
            let mid = [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0, (w[0][2] + w[1][2]) / 2.0];
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let l = (dx * dx + dy * dy).sqrt();
            let (nx, ny) = (-dy / l, dx / l);
            let Some(inside) = base.land().footing([mid[0] + nx * 0.6, mid[1] + ny * 0.6, mid[2] + 0.5]) else { continue };
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
}
