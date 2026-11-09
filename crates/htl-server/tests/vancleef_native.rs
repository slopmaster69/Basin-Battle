//! VanCleef's native staging on the Juggernaut's main deck. Requires installed 1.12.1 data; no map
//! assets here. Controlled staging proof, not an unassisted clear (that's the filmed one).
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
    assert!(dist2d(home, [-77.0, -820.0, 40.0]) < 0.01, "his cabin's doorway: {home:?}");
    let hero = s.join("sternproof", "Sternproof", None);
    s.players.get_mut(&hero).unwrap().last_safe = home;
    s.move_player(hero, [-66.0, -819.5, 41.0], std::f32::consts::PI, Default::default());
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

fn covered(s: &Survival, at: [f32; 3]) -> bool {
    s.land().slab_between(at[0], at[1], at[2] + 2.5, at[2] + 40.0)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn he_steps_out_onto_the_open_deck_and_his_blackguards_watch_from_the_rail() {
    let (mut s, hero, boss) = fight();
    let home = s.mobs[&boss].home;
    assert!(covered(&s, home), "he waits under his cabin's roof");
    let watchers = observers(&s);
    assert_eq!(watchers.len(), 2, "both Blackguards at their posts");
    let mut first_blow = None;
    let mut previous = s.mobs[&boss].pos;
    for _ in 0..160 {
        let events = step(&mut s, hero);
        let pos = s.mobs[&boss].pos;
        assert!(s.land().footing(pos).is_some());
        assert!(dist2d(previous, pos) <= 2.6 * DT + 0.01, "he walks out: {previous:?} -> {pos:?}");
        previous = pos;
        if events.iter().any(|e| matches!(e, Event::MobWindup { id, .. } if *id == boss)) {
            first_blow = Some(pos);
            break;
        }
    }
    let at = first_blow.expect("he fights once out");
    assert!(dist2d(at, home) > 5.0, "out of the doorway: {at:?}");
    assert!(!covered(&s, at), "on the open deck: {at:?}");
    for w in &watchers {
        let m = &s.mobs[w];
        assert!(!covered(&s, m.pos) && m.path.is_empty() && m.state.foe().is_none());
        assert!(m.pack.is_none() && m.summoned_by.is_none());
        println!("Blackguard {w} watches from {:?}", m.pos);
    }
    println!("first blow from {at:?}, {:.1} yd out of the doorway", dist2d(at, home));
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_blackguards_walk_off_the_ship_on_native_floors_and_leave_once() {
    let (mut s, hero, boss) = fight();
    let watchers = observers(&s);
    assert_eq!(watchers.len(), 2);
    for _ in 0..120 { step(&mut s, hero); }
    // Set only the health gate so departure cannot be confused with a posture break.
    s.mobs.get_mut(&boss).unwrap().health = s.mobs[&boss].max_health * 3 / 10;
    let mut walked_at = None;
    let mut previous: Vec<_> = watchers.iter().map(|w| s.mobs[w].pos).collect();
    let mut gone_at = [None, None];
    let mut cowards = None;
    for _ in 0..800 {
        let events = step(&mut s, hero);
        for (i, w) in watchers.iter().enumerate() {
            assert!(!events.iter().any(|e| matches!(e, Event::MobWindup { id, .. } if id == w)), "spectator cannot attack");
            if let Some(m) = s.mobs.get(w) {
                assert!(m.friendly && m.health == 1 && m.state.foe().is_none());
                assert!(s.land().footing(m.pos).is_some(), "dry supported observer: {:?}", m.pos);
                assert!(dist2d(previous[i], m.pos) <= 3.0 * DT + 0.01, "continuous walk: {:?} -> {:?}", previous[i], m.pos);
                if walked_at.is_none() && !m.path.is_empty() { walked_at = Some(s.clock); }
                previous[i] = m.pos;
            } else if gone_at[i].is_none() {
                gone_at[i] = Some(s.clock);
                println!("Blackguard {w} left the ship at {:?}", previous[i]);
                assert!(dist2d(previous[i], s.mobs[&boss].pos) > 18.0, "out of the duel, off the ship");
            }
        }
        if events.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == "Cowards! All of you!")) {
            cowards = Some(s.clock);
            break;
        }
    }
    assert!(gone_at.iter().all(Option::is_some), "both walked off: {gone_at:?}");
    assert!(cowards.is_some(), "then he says so");
    assert!(observers(&s).is_empty());
    println!("walked at {walked_at:?}, gone {gone_at:?}, line at {cowards:?}");
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_native_posture_break_backs_him_away_on_the_open_deck_then_cues_the_advance() {
    for delay in [3.0, 15.0] {
        let (mut s, hero, boss) = fight();
        for _ in 0..(delay / DT) as usize { step(&mut s, hero); }
        let at = s.mobs[&boss].pos;
        // A real break's reward, not a debug health change (which never counts toward posture).
        assert!(s.test_break_stance(boss, hero));
        let broken_until = s.mobs[&boss].stagger_until;
        let began = s.clock;
        let mut arrived_at = None;
        let mut advanced_at = None;
        let mut previous = at;
        for _ in 0..200 {
            let events = step(&mut s, hero);
            let pos = s.mobs[&boss].pos;
            assert!(s.land().footing(pos).is_some());
            assert!(dist2d(previous, pos) <= 2.2 * DT + 0.01, "retreat cannot snap: {previous:?} -> {pos:?}");
            if s.clock < broken_until { assert_eq!(pos, at, "whole posture reward before retreat"); }
            for e in &events {
                match e {
                    Event::Sound { id, sound: 649 } if *id == boss => {
                        arrived_at = Some(s.clock);
                        assert!(!covered(&s, pos), "out of the smoke on the open deck, not in the cabin: {pos:?}");
                    }
                    Event::MobWindup { id, windup, .. } if *id == boss => {
                        let arrival = arrived_at.expect("arrival before advance");
                        assert!(s.clock - arrival >= 0.89);
                        assert!((*windup - 1.1).abs() < 0.01, "full advance windup");
                        advanced_at = Some(s.clock);
                    }
                    _ => {}
                }
            }
            previous = pos;
            if advanced_at.is_some() { break; }
        }
        assert!(advanced_at.is_some());
        println!("delay {delay}s, break at {at:?}: arrival +{:.2} at {:.1} yd, advance +{:.2}", arrived_at.unwrap() - began, dist2d(at, previous), advanced_at.unwrap() - began);
    }
}
