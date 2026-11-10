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

/// Each piece stands on the deck and follows it: its posts reach the floor under both of its ends
/// (none floats where the deck falls away), and its top stands at least 1.6 yd over that floor at
/// both (a hero's 1 yd step up and a 0.23 yd hop back clear neither).
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn every_piece_of_the_rail_stands_on_the_real_deck_and_tops_a_step_up() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.clear();
    let s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let r = s.content.dungeon.as_ref().unwrap().rails.iter().find(|r| r.key == "juggernaut_starboard").unwrap().clone();
    // ElwynnWoodFence01's collision hull, from its origin.
    let (bottom, top) = (-0.56 * r.scale, 1.80 * r.scale);
    let len = r.length * r.scale;
    let pieces: Vec<_> = s.objs.values().filter(|o| matches!(o.kind, htl_survival::ObjKind::Prop { display, .. } if display == r.display)).collect();
    assert!(pieces.len() >= 14, "{} pieces", pieces.len());
    for o in pieces {
        let back = [o.pos[0] - o.facing.cos() * len, o.pos[1] - o.facing.sin() * len];
        for (x, y) in [(o.pos[0], o.pos[1]), (back[0], back[1])] {
            let floor = s.land().height_near(x, y, o.pos[2] + 0.5);
            println!("piece {:?} end ({x:.1}, {y:.1}): floor {floor:.2}, posts to {:.2}, top {:.2}", o.pos, o.pos[2] + bottom, o.pos[2] + top);
            assert!(o.pos[2] + bottom <= floor + 0.05, "its posts reach the deck at ({x}, {y})");
            assert!(o.pos[2] + top - floor >= 1.6, "its top stands {:.2} over the deck at ({x}, {y})", o.pos[2] + top - floor);
        }
    }
}
