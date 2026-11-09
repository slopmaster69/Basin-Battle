//! Native movement regression. Requires the user's installed 1.12.1 data; no map assets here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{Content, Event, Survival, dist2d};

fn point(route: &[[f32; 3]], mut distance: f32) -> [f32; 3] {
    for w in route.windows(2) {
        let len = (0..3).map(|k| (w[1][k]-w[0][k]).powi(2)).sum::<f32>().sqrt();
        if distance <= len {
            let t = distance / len;
            return std::array::from_fn(|k| w[0][k] + (w[1][k]-w[0][k])*t);
        }
        distance -= len;
    }
    *route.last().unwrap()
}

fn content() -> Content {
    Content::builtin()
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn rhahkzor_walks_the_native_collapse_route_without_resets() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to the installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut content = content();
    let def = content.mob("dm_rhahkzor").unwrap();
    let collapse = content.mobs[def].phases.iter().find_map(|p| p.collapse.as_ref()).expect("gauntlet configured");
    for (i, w) in collapse.route.windows(2).enumerate() {
        let path = htl_survival::land::walking_route(&land, w);
        assert!(path.is_some(), "native leg {i}: {:?} -> {:?}", w[0], w[1]);
    }
    let route = htl_survival::land::walking_route(&land, &collapse.route).expect("native route");
    let distance: f32 = route.windows(2).map(|w| (0..3).map(|k| (w[1][k]-w[0][k]).powi(2)).sum::<f32>().sqrt()).sum();
    println!("{} native corners; {distance:.2} yd; {:.2} seconds at {} yd/s", route.len(), distance/collapse.speed, collapse.speed);
    let speed = collapse.speed;
    let duration = distance / speed;
    let hold = collapse.hold;
    content.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_rhahkzor");
    let mut s = Survival::new_dungeon(content, Box::new(land), 7, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let original_home = s.mobs[&boss].home;
    let hero = s.join("routeproof", "Routeproof", None);
    // Controlled movement test: health is replenished and phase damage is injected. The boss
    // itself is never moved, reset or forced onto a corner by the test.
    s.move_player(hero, point(&route, 5.0), 0.0, Default::default());
    s.dev_hurt(hero, 0.68, "rhahkzor").unwrap();
    let mut began = false;
    for _ in 0..100 {
        let events = s.step(0.05);
        began |= events.iter().any(|e| matches!(e, Event::Log(t) if t.contains("brings the place down")));
        if began { break; }
    }
    assert!(began, "collapse phase begins before the lethal-hit check");
    s.dev_hurt(hero, 0.0, "rhahkzor").unwrap();
    let floor = (hold * s.mobs[&boss].max_health as f32).round() as u32;
    assert_eq!(s.mobs[&boss].health, floor, "early kill held at 26%");
    let mut previous = s.mobs[&boss].pos;
    let mut reached = false;
    let mut max_step = 0.0_f32;
    for tick in 0..((duration + 5.0) / 0.05).ceil() as usize {
        let time = tick as f32 * 0.05;
        let along = (5.0 + time * 7.0).min(time * speed + 30.0).min(distance);
        let at = point(&route, along);
        let at = s.land().footing(at).unwrap_or_else(|| panic!("hero's continuous native route at tick {tick}, {along} yd: {at:?}, floor {}, blocked {}", s.land().height_near(at[0],at[1],at[2]),s.land().blocked_near(at[0],at[1],at[2])));
        s.move_player(hero, at, 0.0, Default::default());
        let p = s.players.get_mut(&hero).unwrap();
        p.health = p.max_health;
        let move_limit = s.mobs[&boss].speed.max(12.0) * 0.05 + 0.05;
        let events = s.step(0.05);
        for event in &events {
            if let Event::Log(text) = event {
                assert!(!text.contains("put back") && !text.contains("fell through"), "tick {tick}: {text}");
                if text.contains("collapse reached its end") { reached = true; }
            }
        }
        let at = s.mobs[&boss].pos;
        let step = dist2d(previous, at);
        max_step = max_step.max(step);
        assert!(step <= move_limit, "teleport at tick {tick}: {previous:?} -> {at:?}");
        assert!(s.land().footing(at).is_some(), "invalid boss footing at tick {tick}: {at:?}");
        assert!(s.mobs[&boss].state.foe().is_some(), "fight lost at tick {tick}: {:?}", s.mobs[&boss].state);
        assert!(s.mobs[&boss].health >= floor);
        previous = at;
        if tick % 800 == 0 { println!("t={time:.1}: hero {along:.1} yd, boss {at:?}"); }
    }
    assert!(reached, "front arrived");
    assert!(dist2d(s.mobs[&boss].pos, *route.last().unwrap()) < 8.0, "boss arrived: {:?}", s.mobs[&boss].pos);
    s.dev_hurt(hero, 0.24, "rhahkzor").unwrap();
    s.dev_hurt(hero, 0.11, "rhahkzor").unwrap();
    s.dev_hurt(hero, 0.0, "rhahkzor").unwrap();
    assert_eq!(s.mobs[&boss].health, 0, "killable after front arrives");
    println!("arrived and killable; zero resets/falls; maximum stride {max_step:.3} yd; started {original_home:?}");
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_native_collapse_wipe_returns_whole_to_its_original_floor() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to the installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = content();
    let def = c.mob("dm_rhahkzor").unwrap();
    let collapse = c.mobs[def].phases.iter().find_map(|p| p.collapse.as_ref()).unwrap();
    let route = htl_survival::land::walking_route(&land, &collapse.route).unwrap();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_rhahkzor");
    let mut s = Survival::new_dungeon(c, Box::new(land), 7, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let home = s.mobs[&boss].home;
    let hero = s.join("wipeproof", "Wipeproof", None);
    s.move_player(hero, point(&route, 5.0), 0.0, Default::default());
    s.dev_hurt(hero, 0.68, "rhahkzor").unwrap();
    let mut began = false;
    for tick in 0..300 {
        s.move_player(hero, point(&route, 5.0 + tick as f32 * 0.3), 0.0, Default::default());
        let p = s.players.get_mut(&hero).unwrap();
        p.health = p.max_health;
        for event in s.step(0.05) {
            if let Event::Log(text) = event {
                assert!(!text.contains("put back") && !text.contains("fell through"), "{text}");
                began |= text.contains("brings the place down");
            }
        }
    }
    assert!(began);
    assert!(dist2d(home, s.mobs[&boss].pos) > 20.0, "walked away from home");
    s.fall_damage(hero, u32::MAX / 2);
    let mut resets = 0;
    for _ in 0..100 {
        for event in s.step(0.05) {
            if let Event::Log(text) = event {
                assert!(!text.contains("fell through"), "{text}");
                if text.contains("put back") { resets += 1; }
            }
        }
    }
    let m = &s.mobs[&boss];
    assert!(dist2d(m.pos, home) < 1.0 && (m.pos[2] - home[2]).abs() < 1.0);
    assert_eq!(m.health, m.max_health);
    assert_eq!(m.phase, 0);
    assert!((1..=2).contains(&resets), "bounded wipe reset: {resets}");
    println!("wipe: {resets} reset events, no falls, original floor restored whole");
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn smites_existing_short_rack_walk_still_reaches_its_floor() {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to the installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    let def = c.mob("dm_smite").unwrap();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_smite");
    c.dungeon.as_mut().unwrap().doors.clear();
    let mut s = Survival::new_dungeon(c, Box::new(land), 7, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let home = s.mobs[&boss].home;
    let rack = s.land().footing([home[0], home[1]-6.0, home[2]]).unwrap();
    let hero = s.join("rackproof", "Rackproof", None);
    // Initial test placement on the dock, below the entry lamp's floor.
    s.players.get_mut(&hero).unwrap().last_safe = home;
    s.move_player(hero, [home[0], home[1]+4.0, home[2]], 0.0, Default::default());
    s.dev_hurt(hero, 0.60, "smite").unwrap();
    let mut nearest = f32::MAX;
    for _ in 0..400 {
        let p = s.players.get_mut(&hero).unwrap();
        p.health = p.max_health;
        for event in s.step(0.05) {
            if let Event::Log(text) = event {
                assert!(!text.contains("put back") && !text.contains("fell through") && !text.contains("rack route timed out"), "{text}");
            }
        }
        let at = s.mobs[&boss].pos;
        assert!(s.land().footing(at).is_some(), "Smite grounded at {at:?}");
        nearest = nearest.min(dist2d(at, rack));
    }
    assert!(nearest <= 1.7, "within the arsenal's physical rack arrival distance: {nearest}");
    assert_eq!(s.mobs[&boss].phase, 1, "equipped axes after the walk");
    println!("Smite rack: {nearest:.3} yd nearest, equipped axes, no resets/falls");
}
