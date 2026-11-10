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
    let (rail, lip) = (-839.75, -840.25);
    let mut y = start[1];
    while y > -845.0 {
        y -= 0.35;
        s.move_player(hero, [-64.0, y, 41.25], -std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Moving::default() });
        for _ in 0..2 { s.step(0.05); }
        let p = s.players[&hero].pos;
        assert!(p[1] > rail && p[1] > lip, "on the deck: {p:?}");
    }
}
