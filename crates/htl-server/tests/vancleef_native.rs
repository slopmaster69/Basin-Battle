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
fn he_steps_out_onto_the_open_deck_and_his_blackguards_ring_it() {
    let (mut s, hero, boss) = fight();
    let home = s.mobs[&boss].home;
    assert!(covered(&s, home), "he waits under his cabin's roof");
    let watchers = observers(&s);
    assert_eq!(watchers.len(), 8, "all eight Blackguards on the ring");
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
        println!("Blackguard {w} stands at {:?}", m.pos);
    }
    println!("first blow from {at:?}, {:.1} yd out of the doorway", dist2d(at, home));
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_blackguards_walk_off_the_ship_on_native_floors_and_leave_once() {
    let (mut s, hero, boss) = fight();
    let watchers = observers(&s);
    assert_eq!(watchers.len(), 8);
    for _ in 0..120 { step(&mut s, hero); }
    // Set only the health gate so departure cannot be confused with a posture break.
    s.mobs.get_mut(&boss).unwrap().health = s.mobs[&boss].max_health * 3 / 10;
    let mut walked_at = None;
    let mut previous: Vec<_> = watchers.iter().map(|w| s.mobs[w].pos).collect();
    let mut gone_at = [None; 8];
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
        if cowards.is_none() && events.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == "Cowards! All of you!")) {
            cowards = Some(s.clock);
        }
        if gone_at.iter().all(Option::is_some) { break; }
    }
    assert!(gone_at.iter().all(Option::is_some), "all walked off: {gone_at:?}");
    assert!(cowards.is_some_and(|c| walked_at.is_some_and(|w| c <= w + 0.1)), "he calls after them as they set off");
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

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_hero_carried_over_either_rail_mid_duel_is_set_back_on_the_deck() {
    // West (toward the Blackguards' rail) and east, across the beam from the lit deck.
    for side in [1.0_f32, -1.0] {
        let (mut s, hero, boss) = fight();
        for _ in 0..120 {
            step(&mut s, hero);
        }
        assert!(s.mobs[&boss].state.foe().is_some(), "he's dueling");
        let deck = s.players[&hero].pos[2];
        let mut at = s.players[&hero].pos;
        let mut held = None;
        for k in 0..80 {
            // A roll's reports: half a yard at a time at the deck's height, over the side too.
            let next = [at[0], at[1] + side * 0.5, deck];
            s.move_player(hero, next, std::f32::consts::PI, Default::default());
            let now = s.players[&hero].pos;
            if dist2d(now, next) > 0.01 {
                held = Some((k, next, now));
                break;
            }
            step(&mut s, hero);
            at = next;
        }
        let (k, past, back) = held.unwrap_or_else(|| panic!("side {side}: never held at the rail"));
        let floor = s.land().height_near(past[0], past[1], deck + 1.0);
        eprintln!("side {side}: held {k} steps out at {past:?} (floor {floor}), back at {back:?}, deck {deck}");
        // Held over the side, or at the bulwark that now runs along both rails (#343).
        assert!(s.land().water_near(past[0], past[1], deck + 1.0) || floor < deck - 4.0 || s.through_rail(at, past), "side {side}: held only over the side or at the bulwark: {past:?} floor {floor}");
        assert!((back[2] - deck).abs() < 1.5 && dist2d(back, past) < 3.0, "side {side}: back on the deck by the rail: {back:?} (deck {deck}, {k} steps out)");
        for _ in 0..40 {
            step(&mut s, hero);
        }
        assert!(s.mobs[&boss].state.foe().is_some(), "side {side}: the duel goes on");
    }
}

fn dry_open_deck(s: &Survival, at: [f32; 3], deck: f32) -> bool {
    s.land().footing(at).is_some() && !s.land().water_near(at[0], at[1], at[2]) && !covered(s, at) && (at[2] - deck).abs() < 1.5
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_ring_sits_on_the_open_deck_and_every_shove_on_it_lands_on_dry_boards_inside_the_line() {
    let (mut s, _hero, boss) = fight();
    let (c, r) = s.test_ring(boss).expect("his duel is set up");
    // His lit spot's height: the deck is cambered, 40.3 at his door to 41.9 at the east rail.
    let deck = 40.62;
    let spot = [-70.0, -819.5, deck];
    println!("ring: middle {c:?}, radius {r}, his spot {:.1} yd from the middle", dist2d(spot, c));
    // The guards stand on it, dry, open to the sky, on the deck's own level.
    let guards = observers(&s);
    assert_eq!(guards.len(), 8, "all eight placed");
    for g in &guards {
        let at = s.mobs[g].pos;
        assert!(dry_open_deck(&s, at, deck), "a guard on the deck: {at:?}");
        assert!((dist2d(at, c) - r).abs() < 0.6, "on the ring: {at:?}");
    }
    // The whole line, and the boards inside it up to the line (a step of half a yard): dry, open,
    // level. Anything that is not is counted, so a mast or a crate in the ring shows up here.
    let (mut bad_line, mut bad_inside, mut inside) = (0, 0, 0);
    for k in 0..144 {
        let a = k as f32 * std::f32::consts::TAU / 144.0;
        let at = floor(&s, c[0] + a.cos() * r, c[1] + a.sin() * r, deck);
        if !dry_open_deck(&s, at, deck) { bad_line += 1; println!("line not deck at {at:?}"); }
    }
    let mut y = -r;
    while y <= r {
        let mut x = -r;
        while x <= r {
            if (x * x + y * y).sqrt() <= r - 1.0 {
                inside += 1;
                let at = floor(&s, c[0] + x, c[1] + y, deck);
                if !dry_open_deck(&s, at, deck) { bad_inside += 1; }
            }
            x += 0.5;
        }
        y += 0.5;
    }
    println!("line not deck: {bad_line} of 144; inside not deck: {bad_inside} of {inside}");
    assert_eq!(bad_line, 0, "the guards' line is open deck all round");
    assert!(bad_inside * 100 <= inside, "the ring holds open boards: {bad_inside} of {inside}");
    // Every shove from the line, with him close beside them, lands on dry open boards inside it.
    let mut worst = 0.0f32;
    for k in 0..72 {
        let a = k as f32 * std::f32::consts::TAU / 72.0;
        let at = floor(&s, c[0] + a.cos() * (r - 0.4), c[1] + a.sin() * (r - 0.4), deck);
        for aside in [0.0f32, 0.9, -0.9] {
            let b = a + std::f32::consts::PI + aside;
            let near = floor(&s, at[0] + b.cos() * 2.5, at[1] + b.sin() * 2.5, deck);
            s.mobs.get_mut(&boss).unwrap().pos = near;
            let to = s.test_shove_to(boss, at).unwrap_or_else(|| panic!("no shove from {at:?} with him at {near:?}"));
            assert!(dry_open_deck(&s, to, deck), "landing on dry open boards: {to:?}");
            assert!(dist2d(to, c) <= r - 1.0 && dist2d(to, near) >= 1.8);
            worst = worst.max(dist2d(at, to));
        }
    }
    println!("longest shove {worst:.1} yd");
}

fn floor(s: &Survival, x: f32, y: f32, deck: f32) -> [f32; 3] {
    [x, y, s.land().height_near(x, y, deck + 1.0)]
}

/// Unbowed on the real deck, from where the first form's lit spot is: he is the pack, so no rise,
/// and the secret is not needed to see his turned Invitation (docs/vancleef-encounter.md).
fn unbowed_fight(hero_at: [f32; 3]) -> (Survival, Id, Id) {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    let def = c.mob("dm_vancleef_unbowed").unwrap();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.mob == "dm_vancleef");
    d.doors.clear();
    d.packs[0].mob = "dm_vancleef_unbowed".into();
    d.packs[0].at = [-70.0, -819.5];
    d.packs[0].z = Some(40.62);
    let mut s = Survival::new_dungeon(c, Box::new(land), 7, 0.0);
    let boss = *s.mobs.iter().find(|(_, m)| m.def == def).unwrap().0;
    let hero = s.join("sternproof", "Sternproof", None);
    s.players.get_mut(&hero).unwrap().last_safe = s.mobs[&boss].home;
    s.move_player(hero, hero_at, std::f32::consts::PI, Default::default());
    step(&mut s, hero);
    (s, hero, boss)
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn unbowed_turned_invitation_thrust_stays_on_the_open_deck_wherever_the_hero_stands() {
    // Near him, at the east rail (the deck's far edge from his door), and at the north rail.
    for at in [[-66.0, -819.5, 41.0], [-56.4, -819.0, 41.9], [-64.0, -809.0, 41.4]] {
        let (mut s, hero, boss) = unbowed_fight(at);
        let deck = 40.62;
        let (mut thrusts, mut invitations) = (0, 0);
        for k in 0..1200 {
            // The hero stands where they were put (a hero who walks off is the unit tests' case).
            if k % 10 == 0 {
                let p = s.players[&hero].pos;
                if dist2d(p, at) > 1.0 { s.move_player(hero, at, std::f32::consts::PI, Default::default()); }
            }
            for e in step(&mut s, hero) {
                match e {
                    Event::MobWindup { id, clip: 85, blockable: false, .. } if id == boss => thrusts += 1,
                    Event::MobGuard { id, up: true } if id == boss => invitations += 1,
                    _ => {}
                }
            }
            let pos = s.mobs[&boss].pos;
            assert!(dry_open_deck(&s, pos, deck), "he stays on the open deck: {pos:?} (hero at {at:?})");
        }
        println!("hero at {at:?}: {invitations} Invitations, {thrusts} thrusts in 60 sec, he ended at {:?}", s.mobs[&boss].pos);
        assert!(invitations >= 2 && thrusts >= 2, "the turned Invitation comes round, and the thrust with it: {invitations}, {thrusts}");
    }
}
