//! VanCleef's native stern staging. Requires installed 1.12.1 data; no map assets here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{Content, Event, Id, Survival, dist2d};

const DT: f32 = 0.05;

fn fight() -> (Survival, Id, Id) {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    let def = c.mob("dm_vancleef").unwrap();
    // Isolate the actual pack; neither its authored home nor the native floor is changed.
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.mob == "dm_vancleef");
    c.dungeon.as_mut().unwrap().doors.clear();
    let mut s = Survival::new_dungeon(c, Box::new(land), 7, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let home = s.mobs[&boss].home;
    assert!(dist2d(home, [-84.0, -820.0, 39.5]) < 0.01);
    let hero = s.join("sternproof", "Sternproof", None);
    s.players.get_mut(&hero).unwrap().last_safe = home;
    s.move_player(hero, [-76.0, -820.0, 40.2], std::f32::consts::PI, Default::default());
    step(&mut s, hero);
    (s, hero, boss)
}

fn step(s: &mut Survival, hero: Id) -> Vec<Event> {
    // Controlled staging proof, not an unassisted clear: replenish the observing hero.
    let p = s.players.get_mut(&hero).unwrap();
    p.health = p.max_health;
    let events = s.step(DT);
    for event in &events {
        if let Event::Log(text) = event {
            assert!(!text.contains("put back") && !text.contains("fell through"), "{text}");
        }
    }
    events
}

fn observers(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "dm_blackguard_observer").map(|(id, _)| *id).collect()
}

fn length(from: [f32; 3], path: &[[f32; 3]]) -> f32 {
    std::iter::once(from).chain(path.iter().copied()).collect::<Vec<_>>().windows(2).map(|p| dist2d(p[0], p[1])).sum()
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn observers_walk_the_native_stern_and_leave_once() {
    let (mut s, hero, boss) = fight();
    let watchers = observers(&s);
    assert_eq!(watchers.len(), 2, "actual canonical-home observers before the health gate");
    let starts: Vec<_> = watchers.iter().map(|w| s.mobs[w].pos).collect();
    // Dry posts and exits alone were insufficient. Both old northbound routes around
    // the cabin exceed the unchanged short-route cap; every individual segment passes.
    for from in &starts {
        let to = s.land().footing([from[0], -807.0, from[2]]).unwrap();
        let path = s.land().path(*from, to).unwrap();
        let mut last = *from;
        for p in &path {
            assert!((p[2] - from[2]).abs() <= 1.5 && s.land().walk_clear(last, *p));
            last = *p;
        }
        let distance = length(*from, &path);
        let cap = dist2d(*from, to) * 1.6 + 1.0;
        assert!(distance > cap);
        println!("old departure from {from:?}: {distance:.3} yd exceeds {cap:.3}; every segment passes");
    }
    for _ in 0..20 { step(&mut s, hero); }
    for (w, at) in watchers.iter().zip(&starts) {
        assert_eq!(s.mobs[w].pos, *at, "waiting until 30%");
        assert!(s.mobs[w].path.is_empty() && s.mobs[w].state.foe().is_none());
        assert!(s.mobs[w].pack.is_none() && s.mobs[w].summoned_by.is_none());
    }
    // Set only the health gate so departure cannot be confused with a posture break.
    s.mobs.get_mut(&boss).unwrap().health = s.mobs[&boss].max_health * 3 / 10;
    step(&mut s, hero);
    let began = s.clock;
    let exits: Vec<_> = watchers.iter().map(|w| *s.mobs[w].path.last().expect("departure started")).collect();
    for ((w, at), exit) in watchers.iter().zip(&starts).zip(&exits) {
        let distance = length(*at, &s.mobs[w].path);
        assert!(distance <= dist2d(*at, *exit) * 1.6 + 1.0);
        assert!(distance / 2.2 < 7.5, "finish before the 8 sec cleanup");
        println!("observer {w}: {at:?} -> {exit:?}, {distance:.3} yd / {:.3} sec", distance / 2.2);
    }
    // Lower-ID boss starts their walks before the observers' movement in this same tick.
    let mut previous: Vec<_> = watchers.iter().map(|w| s.mobs[w].pos).collect();
    assert!(starts.iter().zip(&previous).all(|(a, b)| dist2d(*a, *b) <= 2.2 * DT + 0.002));
    let mut arrived = [None, None];
    for _ in 0..200 {
        let events = step(&mut s, hero);
        for (i, w) in watchers.iter().enumerate() {
            assert!(!events.iter().any(|e| matches!(e, Event::MobWindup { id, .. } if id == w)), "spectator cannot attack");
            if let Some(m) = s.mobs.get(w) {
                assert!(m.friendly && m.health == 1 && m.state.foe().is_none());
                assert!(s.land().footing(m.pos).is_some(), "dry supported observer: {:?}", m.pos);
                assert!(s.land().walk_clear(previous[i], m.pos), "native stride of observer {i}: {:?} -> {:?}, remaining {:?}", previous[i], m.pos, m.path);
                assert!(dist2d(previous[i], m.pos) <= 2.2 * DT + 0.002, "continuous walk: {:?} -> {:?}", previous[i], m.pos);
                if dist2d(m.pos, exits[i]) < 0.1 { arrived[i].get_or_insert(s.clock - began); }
                previous[i] = m.pos;
            } else {
                assert!(s.clock - began >= 7.99 && arrived[i].is_some(), "only cleaned up after reaching exit");
            }
        }
        if s.clock - began >= 8.1 { assert!(observers(&s).is_empty(), "no replacements after departure"); }
    }
    assert!(arrived.iter().all(Option::is_some));
    assert!(observers(&s).is_empty());
    println!("arrivals {arrived:?}; both gone after 8 sec; no teleports, falls, attacks or replacements");
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn native_posture_break_walks_then_cues_the_advance() {
    // At the authored home and after the same 12 sec of combat as the capture: prove
    // actual smoke, movement and windup events, rather than just a possible floor query.
    for delay in [0.0, 12.0] {
        let (mut s, hero, boss) = fight();
        for _ in 0..(delay / DT) as usize { step(&mut s, hero); }
        let at = s.mobs[&boss].pos;
        s.dev_hurt(hero, 0.65, "vancleef").unwrap();
        assert!(s.stance_broken(boss));
        let broken_until = s.mobs[&boss].stagger_until;
        let began = s.clock;
        let mut smoke = None;
        let mut smoked_at = None;
        let mut cleared_at = None;
        let mut arrived_at = None;
        let mut advanced_at = None;
        let mut previous = at;
        for _ in 0..160 {
            let events = step(&mut s, hero);
            let pos = s.mobs[&boss].pos;
            assert!(s.land().footing(pos).is_some());
            assert!(dist2d(previous, pos) <= 4.0 * DT + 0.002, "retreat cannot teleport");
            if s.clock < broken_until { assert_eq!(pos, at, "whole posture reward before retreat"); }
            for e in &events {
                match e {
                    Event::ObjSpawned { id } if s.object(*id).is_some_and(|o| o.display == 45) => {
                        assert!(smoke.replace(*id).is_none(), "one smoke cloud per break");
                        smoked_at = Some(s.clock);
                        assert!(s.clock >= broken_until);
                    }
                    Event::ObjGone { id } if Some(*id) == smoke => { cleared_at = Some(s.clock); }
                    Event::Visual { id, kit: 379, .. } if *id == boss => {
                        arrived_at = Some(s.clock);
                        assert!((dist2d(at, pos) - 4.0).abs() < 0.1, "walked the entire short retreat");
                        assert!(smoke.is_some() && cleared_at.is_some());
                    }
                    Event::MobWindup { id, windup, .. } if *id == boss => {
                        let arrival = arrived_at.expect("arrival before advance");
                        assert!(s.clock - arrival >= 0.89);
                        assert!((*windup - 1.1).abs() < 0.01, "full advance windup");
                        let k = s.mobs[&boss].pattern.unwrap().0;
                        assert_eq!(s.content.mobs[s.mobs[&boss].def].patterns[k].name, "advance");
                        advanced_at = Some(s.clock);
                    }
                    _ => {}
                }
            }
            previous = pos;
            if advanced_at.is_some() { break; }
        }
        let cloud_duration = cleared_at.unwrap() - smoked_at.unwrap();
        assert!((0.64..=0.71).contains(&cloud_duration));
        assert!(advanced_at.is_some());
        assert!(smoke.is_some_and(|id| s.object(id).is_none()));
        println!("delay {delay}s, break at {at:?}: smoke +{:.2}, arrival +{:.2}, advance +{:.2}; 4 yd walked", smoked_at.unwrap()-began, arrived_at.unwrap()-began, advanced_at.unwrap()-began);
    }
}
