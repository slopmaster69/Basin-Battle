//! Native movement regression. Requires the user's installed 1.12.1 data; no map assets here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{Content, Event, Id, Survival, dist2d};
use std::collections::BTreeMap;

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

fn real_land() -> DeadminesLand {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to the installed Data directory");
    DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap())
}

/// The mine with Rhahk'Zor alone in it, a hero who fights him from the Galleries side (a few
/// yards past his post along the collapse's route), and the collapse begun.
fn begun(seed: u64) -> (Survival, Id, Id, f32) {
    let land = real_land();
    let mut c = content();
    let def = c.mob("dm_rhahkzor").unwrap();
    let collapse = c.mobs[def].phases.iter().find_map(|p| p.collapse.as_ref()).expect("his collapse").clone();
    let route = htl_survival::land::walking_route(&land, &collapse.route).expect("native route");
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_rhahkzor");
    let mut s = Survival::new_dungeon(c, Box::new(land), seed, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let hero = s.join("routeproof", "Routeproof", None);
    let at = s.land().footing(point(&route, 36.0)).expect("floor past his post");
    s.move_player(hero, at, 0.0, Default::default());
    s.dev_hurt(hero, 0.68, "rhahkzor").unwrap();
    let mut began = false;
    for _ in 0..100 {
        let events = s.step(0.05);
        began |= events.iter().any(|e| matches!(e, Event::Log(t) if t.contains("brings the place down")));
        if began { break; }
    }
    assert!(began, "the collapse begins");
    (s, boss, hero, 36.0)
}

/// One tick of the fight: the hero walks the route (7 yd/s at most, never back) to three yards
/// past the line of the section going, whole again each tick; the boss is never moved by the
/// test. Its checks: no reset, no fall, no teleport, the fight never lost.
fn tick(s: &mut Survival, boss: Id, hero: Id, along: &mut f32) -> Vec<Event> {
    let (k, ..) = s.collapse_state().expect("the collapse on");
    let (lines, total) = s.collapse_lines().unwrap();
    *along = along.max((*along + 7.0 * 0.05).min(lines[k] + 3.0)).min(total);
    let at = s.collapse_route_point(*along).unwrap();
    let at = s.land().footing(at).unwrap_or_else(|| panic!("the hero's route at {along} yd: {at:?}"));
    s.move_player(hero, at, 0.0, Default::default());
    let heroes: Vec<Id> = s.players.keys().copied().collect();
    for h in heroes {
        let p = s.players.get_mut(&h).unwrap();
        p.health = p.max_health;
    }
    let previous = s.mobs[&boss].pos;
    let move_limit = s.mobs[&boss].speed.max(12.0) * 0.05 + 0.05;
    let events = s.step(0.05);
    for event in &events {
        if let Event::Log(text) = event {
            assert!(!text.contains("put back") && !text.contains("fell through"), "{:.2} s: {text}", s.clock);
        }
    }
    let at = s.mobs[&boss].pos;
    assert!(dist2d(previous, at) <= move_limit, "teleport at {:.2} s: {previous:?} -> {at:?}", s.clock);
    assert!(s.land().footing(at).is_some(), "invalid boss footing at {:.2} s: {at:?}", s.clock);
    assert!(s.mobs[&boss].state.foe().is_some(), "fight lost at {:.2} s: {:?}", s.clock, s.mobs[&boss].state);
    events
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn rhahkzor_fights_through_the_collapse_from_the_door_inward_without_resets() {
    let land = real_land();
    let c = content();
    let def = c.mob("dm_rhahkzor").unwrap();
    let collapse = c.mobs[def].phases.iter().find_map(|p| p.collapse.as_ref()).expect("his collapse").clone();
    for (i, w) in collapse.route.windows(2).enumerate() {
        let path = htl_survival::land::walking_route(&land, w);
        assert!(path.is_some(), "native leg {i}: {:?} -> {:?}", w[0], w[1]);
    }
    let route = htl_survival::land::walking_route(&land, &collapse.route).expect("native route");
    let distance: f32 = route.windows(2).map(|w| (0..3).map(|k| (w[1][k]-w[0][k]).powi(2)).sum::<f32>().sqrt()).sum();
    // The Galleries lamp (where a hero who died wakes) is on past the route's end.
    let lamp = c.dungeon.as_ref().unwrap().lamps.iter().map(|l| l.at).min_by(|a, b| dist2d(*a, [-112.5, -405.0, 0.0]).total_cmp(&dist2d(*b, [-112.5, -405.0, 0.0]))).unwrap();
    let short = dist2d(*route.last().unwrap(), lamp);
    println!("{} native corners; {distance:.2} yd from {:?} to {:?}; the Galleries lamp {lamp:?} is {short:.1} yd past its end", route.len(), route[0], route.last().unwrap());
    assert!(dist2d(lamp, [-112.5, -405.0, 0.0]) < 3.0 && short >= 12.0, "the lamp at least 12 yd past the end: {short}");
    assert!((85.0..=110.0).contains(&distance), "about a hundred yards: {distance}");
    drop(land);

    let (mut s, boss, hero, mut along) = begun(7);
    let home = s.mobs[&boss].home;
    let (lines, total) = s.collapse_lines().unwrap();
    assert_eq!(lines.len(), 4, "every authored face crosses the walked route: {lines:?}");
    assert!((total - distance).abs() < 0.1);
    let mut from = 0.0;
    for (k, line) in lines.iter().enumerate() {
        let at = s.collapse_route_point(*line).unwrap();
        println!("section {}: {from:.1} to {line:.1} yd ({:.1} yd), its face across ({:.1}, {:.1}, {:.1})", k + 1, line - from, at[0], at[1], at[2]);
        assert!((10.0..=35.0).contains(&(line - from)), "a section of a size to run: {}", line - from);
        from = *line;
    }
    println!("past the last face: {:.1} yd of floor to the route's end", total - from);
    assert!(total - from >= collapse.throw + 1.0, "floor past the last face to be thrown to");
    // It starts at the door, behind him: his post is past the first line.
    assert_eq!(s.collapse_state().map(|c| (c.0, c.1, c.2)), Some((0, "warn", 0.0)), "from the door");
    assert!(!s.collapse_behind(home) && s.collapse_behind(s.collapse_route_point(lines[0] - 2.0).unwrap()) == false, "nothing down in the warning");
    let floor = (collapse.hold * s.mobs[&boss].max_health as f32).round() as u32;
    s.dev_hurt(hero, 0.0, "rhahkzor").unwrap();
    assert_eq!(s.mobs[&boss].health, floor, "early kill held at 26%");
    let started = s.clock;
    let mut reached = false;
    let mut last = (0, "warn");
    let mut max_step = 0.0_f32;
    while !reached && s.clock - started < 120.0 {
        let previous = s.mobs[&boss].pos;
        let events = tick(&mut s, boss, hero, &mut along);
        reached |= events.iter().any(|e| matches!(e, Event::Log(t) if t.contains("collapse reached its end")));
        max_step = max_step.max(dist2d(previous, s.mobs[&boss].pos));
        assert!(s.mobs[&boss].health >= floor);
        let (k, stage, front, _) = s.collapse_state().unwrap();
        if (k, stage) != last {
            let b = s.mobs[&boss].pos;
            println!("t={:.1}: section {} {stage}, front {front:.1} yd, hero {along:.1} yd, boss ({:.1}, {:.1}, {:.1})", s.clock - started, k + 1, b[0], b[1], b[2]);
            last = (k, stage);
        }
        assert!(!s.collapse_behind(s.mobs[&boss].pos) || stage == "wave", "he stands ahead of every face: {:?}", s.mobs[&boss].pos);
    }
    assert!(reached, "the last section came down");
    let took = s.clock - started;
    let want = 4.0 * collapse.warn + lines[3] / collapse.speed + 3.0 * collapse.rest;
    println!("from the yell to the last face: {took:.1} s (reckoned {want:.1} s)");
    assert!((took - want).abs() < 1.5, "{took} vs {want}");
    assert_eq!(s.collapse_face_lines().len(), 4);
    let at = s.mobs[&boss].pos;
    assert!(!s.collapse_behind(at) && dist2d(at, s.collapse_route_point(total).unwrap()) < 12.0, "the last stand at the end: {at:?}");
    assert!(dist2d(at, lamp) > 12.0);
    s.dev_hurt(hero, 0.24, "rhahkzor").unwrap();
    s.dev_hurt(hero, 0.11, "rhahkzor").unwrap();
    s.dev_hurt(hero, 0.0, "rhahkzor").unwrap();
    assert_eq!(s.mobs[&boss].health, 0, "killable once the mine is down");
    println!("arrived and killable; zero resets/falls; maximum stride {max_step:.3} yd; started {home:?}");
}

/// Every spot of floor a hero could get to `to` from within the mine's mouth, on a half yard's
/// grid (off the whole yards: the faces lie on them), keyed by cell: on foot, or with a hop down
/// off a ledge (the chamber's east floor ends in a bank a yard and a half over its west floor).
/// With `faces`, no step through a face of rubble.
fn flood(s: &Survival, to: [f32; 3], faces: bool) -> BTreeMap<(i32, i32), [f32; 3]> {
    let spot = |i: i32, j: i32, z: f32| s.land().footing([i as f32 * 0.5 + 0.13, j as f32 * 0.5 + 0.13, z]);
    let start = ((to[0] * 2.0).floor() as i32, (to[1] * 2.0).floor() as i32);
    let first = spot(start.0, start.1, to[2]).expect("floor to start from");
    let mut seen = BTreeMap::from([(start, first)]);
    let mut queue = std::collections::VecDeque::from([start]);
    while let Some((i, j)) = queue.pop_front() {
        let p = seen[&(i, j)];
        // (A step to the next cell; a hop as far as a yard and a half.)
        for (di, dj) in (-3..=3).flat_map(|di| (-3..=3).map(move |dj| (di, dj))) {
            let n = (i + di, j + dj);
            let step = di.abs() <= 1 && dj.abs() <= 1;
            if seen.contains_key(&n) || !(-250..=0).contains(&n.0) || !(-860..=-660).contains(&n.1) {
                continue;
            }
            // From there (`q`) to here (`p`): a walk; or a hop down off a ledge, nothing in the way.
            let walk = spot(n.0, n.1, p[2]).filter(|q| step && (q[2] - p[2]).abs() <= 1.5 && s.land().walk_clear(*q, p));
            let hop = || [2.0, 4.0].iter().filter_map(|d| spot(n.0, n.1, p[2] + d)).find(|q| {
                let (dx, dy) = (p[0] - q[0], p[1] - q[1]);
                let d = dx.hypot(dy);
                (0.5..=6.0).contains(&(q[2] - p[2])) && s.land().ray([q[0], q[1], q[2] + 1.0], [dx / d, dy / d, 0.0], d + 0.4).is_none()
            });
            let Some(q) = walk.or_else(hop) else { continue };
            if faces && s.face_crossing(q, p).is_some() {
                continue;
            }
            seen.insert(n, q);
            queue.push_back(n);
        }
    }
    seen
}

/// Corey's question, on the real mine: "what happens if the player is on the other side of that
/// rubble?" Nobody can be. Once a section's face stands, every spot of floor that no longer
/// walks to the Galleries lamp is a spot the collapse catches a hero on (and throws them clear
/// from), in the door pocket, the side cave, any corner; and no spot that still walks to the
/// lamp is one. Three heroes left in the worst corners end the collapse ahead of its last face.
#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn nobody_is_cut_off_anywhere_in_the_real_mine() {
    let (mut s, boss, hero, mut along) = begun(11);
    let lamp = [-112.5, -405.0, 59.9];
    let lamp = s.land().footing(lamp).expect("the Galleries lamp's floor");
    let whole = flood(&s, lamp, false);
    let (lines, _) = s.collapse_lines().unwrap();
    let door = s.collapse_route_point(1.0).unwrap();
    assert!(whole.values().any(|p| dist2d(*p, door) < 1.5), "the flood reaches the door: {} cells", whole.len());
    // The worst corners: the door's end, the side cave's end, the chamber's south corner.
    let worst = |score: &dyn Fn(&[f32; 3]) -> f32| *whole.values().filter(|p| p[0] > -77.0).max_by(|a, b| score(a).total_cmp(&score(b))).unwrap();
    let corners = [("Doorend", worst(&|p| p[0])), ("Cavend", worst(&|p| p[1])), ("Southend", worst(&|p| -p[1]))];
    let mut left = Vec::new();
    for (name, at) in corners {
        let h = s.join(&name.to_lowercase(), name, None);
        s.move_player(h, at, 0.0, Default::default());
        assert!(dist2d(s.players[&h].pos, at) < 0.1);
        println!("{name} left at ({:.1}, {:.1}, {:.1})", at[0], at[1], at[2]);
        left.push((name, h, at));
    }
    let mut caught: BTreeMap<String, usize> = BTreeMap::new();
    let mut checked = 0;
    let mut last = (0, "warn");
    let started = s.clock;
    loop {
        assert!(s.clock - started < 120.0, "the collapse never done");
        let events = tick(&mut s, boss, hero, &mut along);
        for e in &events {
            if let Event::Log(t) = e {
                if let Some(rest) = t.strip_prefix("the collapse catches ") {
                    println!("t={:.1}: caught {rest}", s.clock - started);
                    *caught.entry(rest.split(' ').next().unwrap().to_string()).or_default() += 1;
                }
            }
        }
        let (k, stage, _, done) = s.collapse_state().unwrap();
        if stage == "warn" && k == 0 {
            assert!(whole.values().all(|p| !s.collapse_behind(*p)), "nothing is behind anything before the first rock");
        }
        if stage == "rest" {
            for (name, h, _) in &left {
                let p = s.players[h].pos;
                assert!(!s.collapse_behind(p), "{name} behind a standing face at {p:?}");
            }
        }
        if (k, stage) != last && stage == "rest" {
            // The face just up: what still walks to the lamp, and what is caught.
            let open = flood(&s, lamp, true);
            let (mut cut, mut wrong_cut, mut wrong_open) = (0, Vec::new(), Vec::new());
            for (cell, p) in &whole {
                match (open.contains_key(cell), s.collapse_behind(*p)) {
                    (true, false) => {}
                    (false, true) => cut += 1,
                    (false, false) => wrong_cut.push(*p),
                    (true, true) => wrong_open.push(*p),
                }
            }
            println!("section {} down, its face at {:.1} yd: {} of {} sq yd of floor walk to the lamp, {} behind the rubble (caught)", k + 1, lines[k], open.len() / 4, whole.len() / 4, cut / 4);
            assert!(wrong_cut.is_empty(), "floor cut off from the lamp where nobody would be caught: {} spots, {:?}", wrong_cut.len(), &wrong_cut[..wrong_cut.len().min(12)]);
            assert!(wrong_open.is_empty(), "floor that walks to the lamp where a hero would be caught: {} spots, {:?}", wrong_open.len(), &wrong_open[..wrong_open.len().min(12)]);
            assert!(cut > 400 * (k + 1), "a real stretch of the mine buried: {cut}");
            assert!(open.values().any(|p| dist2d(*p, lamp) < 1.5));
            for (name, h, _) in &left {
                let p = s.players[h].pos;
                assert!(open.values().any(|q| dist2d(*q, p) < 1.5), "{name} at {p:?} walks to the lamp");
            }
            checked += 1;
        }
        last = (k, stage);
        if done {
            break;
        }
    }
    assert_eq!(checked, 4, "checked at every face");
    for (name, h, at) in &left {
        let p = s.players[h].pos;
        assert!(caught.get(*name).copied().unwrap_or(0) >= 1, "{name} caught");
        assert!(dist2d(p, *at) > 20.0 && !s.collapse_behind(p) && s.players[h].health > 0, "{name} thrown clear, alive: {p:?}");
        println!("{name} ends at ({:.1}, {:.1}, {:.1}), caught {} times", p[0], p[1], p[2], caught[*name]);
    }
    assert!(!caught.contains_key("Routeproof"), "the hero who kept ahead never caught");
    // No walking back through the last face either.
    let (lines, _) = s.collapse_lines().unwrap();
    let ahead = s.land().footing(s.collapse_route_point(lines[3] + 2.0).unwrap()).unwrap();
    let back = s.land().footing(s.collapse_route_point(lines[3] - 3.0).unwrap()).unwrap();
    s.move_player(hero, ahead, 0.0, Default::default());
    s.move_player(hero, back, 0.0, Default::default());
    assert!(!s.collapse_behind(s.players[&hero].pos) && dist2d(s.players[&hero].pos, back) > 2.0, "stopped at the face: {:?}", s.players[&hero].pos);
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_native_collapse_wipe_returns_whole_to_its_original_floor() {
    let (mut s, boss, hero, mut along) = begun(7);
    let home = s.mobs[&boss].home;
    // Through two sections and into the third: he has walked out of his chamber with the wave.
    let started = s.clock;
    while s.collapse_state().map(|c| (c.0, c.1)) != Some((2, "rest")) {
        assert!(s.clock - started < 90.0, "never at the third face: {:?}", s.collapse_state());
        tick(&mut s, boss, hero, &mut along);
    }
    for _ in 0..80 {
        tick(&mut s, boss, hero, &mut along);
    }
    assert!(dist2d(home, s.mobs[&boss].pos) > 20.0, "walked away from home: {:?}", s.mobs[&boss].pos);
    let (rubble, faces) = (s.collapse_rubble().len(), s.collapse_faces().len());
    assert!(rubble > 60 && faces >= 3 * 8, "the mine down behind him: {rubble} rocks, {faces} in its faces");
    assert_eq!(s.collapse_face_lines().len(), 3);
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
    // The mine stands again: no rock, no face, nothing to stop the next hero.
    assert_eq!(s.collapse_front(), None);
    assert!(s.collapse_rubble().is_empty() && s.collapse_faces().is_empty() && s.collapse_face_lines().is_empty(), "the mine put back");
    println!("wipe: {resets} reset events, no falls, original floor restored whole; {rubble} rocks and {faces} face rocks taken away");
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
