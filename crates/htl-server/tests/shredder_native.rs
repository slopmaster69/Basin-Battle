//! Real Mast Room footing and the retained native Shredder pose; no game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{Content, Event, Id, Input, Survival, dist2d};

const DT: f32 = 0.05;

fn live(s: &Survival, key: &str) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
}

fn step(s: &mut Survival, hero: Id) -> Vec<Event> {
    // Controlled staging, not an unassisted clear.
    let p = s.players.get_mut(&hero).unwrap();
    p.health = p.max_health;
    let ev = s.step(DT);
    for e in &ev {
        if let Event::Log(text) = e {
            assert!(!text.contains("put back") && !text.contains("fell through"), "{text}");
        }
    }
    ev
}

fn clearance(s: &Survival, driver: Id, origin: [f32; 3], face: f32, scale: f32) -> f32 {
    let p = s.mobs[&driver].pos;
    let (dx, dy) = (p[0] - origin[0], p[1] - origin[1]);
    let (x, y) = (face.cos() * dx + face.sin() * dy, -face.sin() * dx + face.cos() * dy);
    // Independently measured native Death-sequence bounds (not the small static hull).
    [x - 3.2587168 * scale, -4.287402 * scale - x,
        y - 4.791185 * scale, -4.0007997 * scale - y].into_iter().fold(f32::NEG_INFINITY, f32::max)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn sneed_works_outside_the_native_wreck_and_can_be_hit_or_rebuild_at_its_center() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    // Authored post, and the two native camera setups which previously hid the work.
    for (case, at, face, approach) in [
        ("post", [-280.0, -505.0, 49.5], std::f32::consts::PI, [-280.0, -505.0, 49.5]),
        ("north", [-288.0, -497.0, 50.4], 4.712389, [-288.0, -499.0, 50.2]),
        ("east", [-280.0, -505.0, 49.5], std::f32::consts::PI, [-277.0, -505.0, 49.5]),
    ] {
        let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
        let mut c = Content::builtin();
        c.dungeon.as_mut().unwrap().packs.retain(|p| case == "post" && p.key == "sneed");
        c.dungeon.as_mut().unwrap().doors.clear();
        let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
        let hero = s.join("repairproof", "Repairproof", None);
        s.players.get_mut(&hero).unwrap().last_safe = at;
        s.players.get_mut(&hero).unwrap().loadout.clear();
        s.move_player(hero, at, face, Default::default());
        if case != "post" { s.summon(hero, "dm_shredder").unwrap(); }
        for _ in 0..100 { step(&mut s, hero); }
        let machine = live(&s, "dm_shredder").unwrap();
        let m = &s.mobs[&machine];
        let (wreck, direction, scale) = (m.pos, m.facing, s.content.mobs[m.def].scale);
        s.dev_hurt(hero, 0.0, "dm_shredder").unwrap();
        s.move_player(hero, approach, face, Default::default());
        let driver = live(&s, "dm_sneed").unwrap();
        let mut began = None;
        let mut previous = s.mobs[&driver].pos;
        for _ in 0..500 {
            // Inside the collapsing machine, then thrown out of it: a flight, not a walk.
            let flying = s.mobs[&driver].hidden || s.mobs[&driver].thrown_until > s.clock - DT;
            let ev = step(&mut s, hero);
            let pos = s.mobs[&driver].pos;
            assert!(s.land().footing(pos).is_some(), "dry supported driver: {pos:?}");
            if !flying {
                assert!(dist2d(previous, pos) <= 7.5 * DT + 0.002, "no teleport: {previous:?} -> {pos:?}");
                assert!(s.land().walk_clear(previous, pos));
            }
            previous = pos;
            if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: 2575, ms: 4000 } if *caster == driver)) {
                let body = 0.5 * s.content.mobs[s.mobs[&driver].def].scale;
                let gap = clearance(&s, driver, wreck, direction, scale);
                assert!(gap >= body + 0.1, "actual Mining start outside pose, including body: {gap}");
                assert!(ev.iter().any(|e| matches!(e, Event::MobStop { id, facing, .. } if *id == driver && (*facing - s.mobs[&driver].facing).abs() < 0.001)), "client receives the work facing");
                println!("{case}: wreck {wreck:?} facing {direction:.6} scale {scale}; work {pos:?} facing {:.6}; clearance {gap:.3}; began {:.2}", s.mobs[&driver].facing, s.clock);
                began = Some(s.clock);
                break;
            }
        }
        let began = began.expect("native driver reaches edge and starts Mining");
        let work = s.mobs[&driver].pos;
        let hp = s.mobs[&driver].health;
        if case == "post" {
            for _ in 0..77 { step(&mut s, hero); }
            assert!(live(&s, "dm_shredder").is_none(), "no repair before four seconds");
            assert_eq!(s.mobs[&driver].pos, work);
            for _ in 0..5 { step(&mut s, hero); }
            assert!(s.clock - began >= 4.0);
            let back = live(&s, "dm_shredder").unwrap();
            assert_eq!(s.mobs[&back].pos, wreck, "restart at wreck, not the work point");
            assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health / 3);
            assert!(!s.mobs.contains_key(&machine) && !s.mobs.contains_key(&driver));
            s.dev_hurt(hero, 0.0, "dm_shredder").unwrap();
            let spent = live(&s, "dm_sneed").unwrap();
            assert_eq!(s.mobs[&spent].health, hp);
            for _ in 0..240 {
                assert!(!step(&mut s, hero).iter().any(|e| matches!(e, Event::Cast { spell: 2575, .. })));
            }
        } else {
            // Approach from outside the body, then use an ordinary empty-loadout swing.
            let outward = htl_survival::facing(wreck, work);
            let hit_from = s.land().footing([work[0] + outward.cos() * 1.2, work[1] + outward.sin() * 1.2, work[2]]).unwrap();
            assert!(s.land().walk_clear(hit_from, work), "an unobstructed melee approach");
            s.move_player(hero, hit_from, htl_survival::facing(hit_from, work), Default::default());
            s.input(hero, Input::Attack);
            for _ in 0..28 { step(&mut s, hero); }
            assert!(s.mobs[&driver].health < hp, "ordinary attack lands beside wreck");
            assert!(s.mobs.get(&machine).is_some_and(|m| m.health == 0 && m.pos == wreck && matches!(m.state, htl_survival::mobs::MobState::Dead { gone_at } if gone_at.is_infinite())), "interruption leaves the wreck where it lay, quiet");
            for _ in 0..200 {
                assert!(!step(&mut s, hero).iter().any(|e| matches!(e, Event::Cast { spell: 2575, .. })));
            }
            assert!(live(&s, "dm_shredder").is_none() && !s.slain("sneed"));
            println!("{case}: melee approach {hit_from:?}, facing {:.6}; ordinary interruption, no rebuild", htl_survival::facing(hit_from, work));
        }
    }
}
