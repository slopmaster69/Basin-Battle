//! Gilnid's ledges, valve and fire on the real Foundry floor; no game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::land::clear_line;
use htl_survival::{dist2d, Content, Event, Id, Survival};

const DT: f32 = 0.05;
/// The gap by the Foundry's door, where a hero first sees into the pit.
const WAY_IN: [[f32; 2]; 2] = [[-186.0, -579.0], [-188.0, -580.0]];

fn gilnid(s: &Survival) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == "dm_gilnid").map(|(id, _)| *id)
}

fn step(s: &mut Survival, hero: Id) -> Vec<Event> {
    // Controlled staging, not an unassisted clear.
    let p = s.players.get_mut(&hero).unwrap();
    p.health = p.max_health;
    let ev = s.step(DT);
    for e in &ev {
        if let Event::Log(text) = e {
            assert!(!text.contains("put back") && !text.contains("fell through") && !text.contains("omitted"), "{text}");
        }
    }
    ev
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn gilnid_is_seen_from_the_way_in_his_crew_stands_on_planks_over_the_pit_and_the_fire_crosses_it() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.key == "gilnid");
    c.dungeon.as_mut().unwrap().doors.clear();
    let aggro = c.mobs[c.mob("dm_gilnid").unwrap()].aggro;
    let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let hero = s.join("ledgeproof", "Ledgeproof", None);
    s.players.get_mut(&hero).unwrap().loadout.clear();
    let way_in: Vec<_> = WAY_IN.iter().map(|p| s.land().footing([p[0], p[1], 21.0]).unwrap()).collect();
    s.players.get_mut(&hero).unwrap().last_safe = way_in[0];
    // Far enough out not to pull him yet.
    let outside = s.land().footing([-178.0, -579.0, 19.5]).unwrap();
    s.move_player(hero, outside, std::f32::consts::PI, Default::default());
    for _ in 0..20 { step(&mut s, hero); }
    let boss = gilnid(&s).expect("Gilnid at his post");
    let post = s.mobs[&boss].home;

    // His sight: from the gap by the door he's seen, close enough to come.
    for p in &way_in {
        assert!(clear_line(s.land(), *p, post), "seen from {p:?}");
        assert!(dist2d(*p, post) <= aggro, "{p:?} is {:.1} yd off", dist2d(*p, post));
    }

    let (ledges, valve) = s.perch_ledges(boss);
    assert_eq!(ledges.len(), 2, "both ledges placed");
    for l in &ledges {
        let high = l.stand[2] - l.base[2];
        println!("ledge: stand {:?} rail {:?} pole {:?} landing {:?} climb {:?}, {high:.1} yd up", l.stand, l.rail, l.base, l.landing, l.climb);
        assert!((7.0..=10.0).contains(&high), "7 to 10 yd over the pit: {high}");
        assert!((l.base[2] - post[2]).abs() < 0.5, "the pole stands on his floor");
        // The plank's back end on the ramp's rail: the ramp is right under it.
        let ramp = s.land().height_near(l.rail[0], l.rail[1], l.rail[2] + 0.5);
        assert!(ramp <= l.rail[2] + 0.05 && ramp >= l.rail[2] - 1.6, "rail {:?}: ramp at {ramp}", l.rail);
        // Open air under him and under the plank's far part, down to the pit.
        for p in [l.stand, l.base] {
            assert!(!s.land().slab_between(p[0], p[1], l.base[2] + 0.2, l.stand[2] - 0.5), "open air at {p:?}");
            assert!((s.land().height_near(p[0], p[1], l.stand[2] - 1.0) - post[2]).abs() < 0.5, "nothing under {p:?} but the pit");
        }
        assert!(s.land().walk_clear(post, l.base) || s.land().path(post, l.base).is_some(), "a hero can walk to the pole");
        assert!(s.land().walk_clear(post, l.landing) || s.land().path(post, l.landing).is_some());
        // Seen from his post: a hero beside him can shoot the man on the plank.
        assert!(clear_line(s.land(), post, l.stand), "the plank seen from his post");
        // The way back: onto the ramp behind the rail, and down it to the pit.
        assert!(l.climb[2] < l.rail[2] && l.climb[2] > l.rail[2] - 2.5, "climb onto the ramp walkway");
        assert!(s.land().path(l.climb, post).is_some(), "down the ramp to the pit");
    }
    let (wheel, stand) = valve.unwrap();
    assert!(dist2d(wheel, [-208.1, -583.1, 0.0]) < 0.1, "the melting pot's own valve");
    assert!(dist2d(stand, wheel) < 1.5 && s.land().walk_clear(post, stand) || s.land().path(post, stand).is_some());

    // Pull from the way in, then send the crew down at once: the fire's channel is marked
    // across the pit floor (the valve's own proof is `tests/gilnid_valve.rs`).
    s.move_player(hero, way_in[1], htl_survival::facing(way_in[1], post), Default::default());
    for _ in 0..10 { step(&mut s, hero); }
    assert!(s.mobs[&boss].state.foe().is_some(), "pulled from the way in");
    // Fight beside his post: the first stick, then the furnace brings the first man down.
    let beside = s.land().footing([post[0] + 2.0, post[1] + 2.0, post[2]]).unwrap();
    s.move_player(hero, beside, htl_survival::facing(beside, post), Default::default());
    let mut lesson = false;
    for _ in 0..400 {
        let ev = step(&mut s, hero);
        lesson |= ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Mind the equipment!"));
        if lesson && s.mobs.get(&ledges[0].worker).is_some_and(|m| m.pos == ledges[0].landing) { break; }
    }
    assert!(lesson, "the first stick thrown and blown, then the furnace's lesson");
    assert_eq!(s.mobs[&ledges[0].worker].pos, ledges[0].landing, "he fell to the pit floor");
    s.move_player(hero, way_in[1], htl_survival::facing(way_in[1], post), Default::default());
    for _ in &ledges { s.dev_hurt(hero, 0.0, "engineer").ok(); }
    for l in &ledges {
        if s.mobs.get(&l.worker).is_some_and(|m| m.health > 0) {
            let at = s.mobs[&l.worker].pos;
            s.move_player(hero, s.land().footing([at[0], at[1], post[2]]).unwrap_or(beside), 0.0, Default::default());
            s.dev_hurt(hero, 0.0, "engineer").unwrap();
        }
    }
    let mut stripe = Vec::new();
    for _ in 0..400 {
        let ev = step(&mut s, hero);
        stripe.extend(ev.iter().filter_map(|e| match e {
            Event::GroundBurst { shape: "line", at, reach, width, .. } => Some((*at, *reach, *width)), _ => None,
        }));
        if !stripe.is_empty() { break; }
    }
    let (at, reach, width) = *stripe.last().expect("the channel is marked");
    println!("channel: from {at:?}, {reach:.1} yd long, {width:.1} wide");
    assert!(reach >= 9.0, "it crosses the pit: {reach:.1} yd");
    assert!((at[2] - post[2]).abs() < 1.0, "on his floor");
}
