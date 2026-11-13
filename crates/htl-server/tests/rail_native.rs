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
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_piece_of_the_rail_stands_on_the_real_deck_and_tops_a_step_up() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    let s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let r = s.content.dungeon.as_ref().unwrap().rails.iter().find(|r| r.key == "juggernaut_starboard").unwrap().clone();
    let (bottom, top) = (r.lift + r.height[0] * r.scale, r.lift + r.height[1] * r.scale);
    let pieces: Vec<_> = s.objs.keys().filter_map(|id| s.rail_piece(*id)).collect();
    assert!(pieces.len() >= 5, "{} pieces", pieces.len());
    for p in pieces {
        let mid = [(p.from[0] + p.to[0]) / 2.0, (p.from[1] + p.to[1]) / 2.0, (p.from[2] + p.to[2]) / 2.0];
        for at in [p.from, mid, p.to] {
            let floor = s.land().height_near(at[0], at[1], at[2] + 0.5);
            println!("piece {:?} -> {:?} at ({:.1}, {:.1}): deck {floor:.2}, base {:.2}, top {:.2}", p.from, p.to, at[0], at[1], at[2] + bottom, at[2] + top);
            assert!((floor - at[2]).abs() < 0.6, "on the deck at ({}, {}): {floor} vs {}", at[0], at[1], at[2]);
            assert!(at[2] + bottom <= floor + 0.05, "its base reaches the deck at ({}, {})", at[0], at[1]);
            assert!(at[2] + top - floor >= 1.73, "its top stands {:.2} over the deck at ({}, {})", at[2] + top - floor, at[0], at[1]);
        }
    }
}
