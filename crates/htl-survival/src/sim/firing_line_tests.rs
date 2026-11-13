use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

/// A hero in sight of the deck, out of his reach: the crew at its posts, nobody pulled yet.
fn posted(heroes: usize) -> (Survival, Vec<Id>, Id, Vec<Id>) {
    posted_with(heroes, 31)
}

fn posted_with(heroes: usize, seed: u64) -> (Survival, Vec<Id>, Id, Vec<Id>) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), seed, 0.0);
    let mut band = Vec::new();
    for n in 0..heroes {
        let h = s.join(&format!("boarder{n}"), &format!("Boarder{n}"), None);
        let p = s.players.get_mut(&h).unwrap();
        p.pos = [-24.0, n as f32 * 1.5, 0.0];
        p.health = 100_000;
        p.max_health = 100_000;
        band.push(h);
    }
    let boss = s.spawn_mob(s.content.mob("dm_greenskin").unwrap(), [0.0; 3], None, false);
    run(&mut s, DT);
    let crew = s.firing_line.crews[&boss].gunners.clone();
    assert_eq!(crew.len(), 2);
    (s, band, boss, crew)
}

/// Pulled, with the heroes 8 yd from him.
fn fight(heroes: usize) -> (Survival, Vec<Id>, Id, Vec<Id>) {
    fight_with(heroes, 31)
}

fn fight_with(heroes: usize, seed: u64) -> (Survival, Vec<Id>, Id, Vec<Id>) {
    let (mut s, band, boss, crew) = posted_with(heroes, seed);
    for (n, h) in band.iter().enumerate() {
        s.players.get_mut(h).unwrap().pos = [-8.0, n as f32 * 1.5, 0.0];
    }
    s.engage(boss, Foe::Hero(band[0]));
    (s, band, boss, crew)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

/// Both gunners down, his draw and its shot done: the captain alone.
fn alone() -> (Survival, Id, Id) {
    let (mut s, band, boss, crew) = fight(1);
    let h = band[0];
    for g in crew {
        s.hurt_mob(g, h, u32::MAX);
    }
    let until = s.clock + 10.0;
    while s.clock < until && !(s.firing_alone(boss) && s.firing_line.crews[&boss].gun_ready > 0.0 && !s.firing_line.aims.contains_key(&boss)) {
        run(&mut s, DT);
    }
    assert!(s.firing_alone(boss));
    (s, h, boss)
}

/// The hero stands beside him for `secs` (no harpoon, no gun at a kiting hero).
fn beside(s: &mut Survival, h: Id, boss: Id, secs: f32) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        let p = s.mobs[&boss].pos;
        s.move_player(h, [p[0] - 2.0, p[1], p[2]], 0.0, Moving::default());
        ev.extend(s.step(DT));
    }
    ev
}

fn shots(ev: &[Event], from: &[Id]) -> Vec<Id> {
    ev.iter().filter_map(|e| match e { Event::Missile { caster, spell: TRACER, .. } if from.contains(caster) => Some(*caster), _ => None }).collect()
}

// --- the setup and the pull ---------------------------------------------------------------------

#[test]
fn the_gunners_stand_at_their_posts_before_the_pull_and_come_with_him() {
    let (mut s, band, boss, crew) = posted(1);
    let home = s.mobs[&boss].home;
    let mut sides = Vec::new();
    for g in &crew {
        let p = s.mobs[g].pos;
        assert!((9.0..=12.0).contains(&dist2d(p, home)), "a post 9 to 12 yd out: {p:?}");
        assert_eq!(s.firing_line_gun(*g), Some(GUN), "guns in hand");
        sides.push(p[1] - home[1]);
    }
    assert!(sides[0] * sides[1] < 0.0, "one each side of him");
    let ev = run(&mut s, 6.0);
    assert!(crew.iter().all(|g| matches!(s.mobs[g].state, MobState::Idle { .. })), "at ease, nobody pulled");
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { .. } | Event::MobWindup { .. } | Event::Clip { .. })));
    assert_eq!(s.firing_line.crews[&boss].gunners, crew, "the same men all along, none faded in");
    s.engage(boss, Foe::Hero(band[0]));
    run(&mut s, DT);
    assert!(crew.iter().all(|g| s.mobs[g].state.target() == Some(band[0])), "pulled with him");
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
}

#[test]
fn striking_a_gunner_first_pulls_the_captain_and_the_whole_crew() {
    let (mut s, band, boss, crew) = posted(1);
    s.hurt_mob(crew[0], band[0], 10);
    run(&mut s, DT);
    assert_eq!(s.mobs[&boss].state.target(), Some(band[0]));
    assert_eq!(s.mobs[&crew[1]].state.target(), Some(band[0]));
    assert!(matches!(s.firing_line.crews[&boss].stage, Stage::Order { .. }));
}

#[test]
fn the_order_follows_his_pull_line_then_the_gunners_take_turns_never_on_one_beat() {
    for heroes in [1, 2] {
        let (mut s, _, boss, crew) = fight(heroes);
        let start = s.clock;
        let (mut lines, mut point, mut loads, mut fired) = (Vec::new(), None, Vec::new(), Vec::new());
        while s.clock - start < 17.0 {
            for e in s.step(DT) {
                let at = s.clock - start;
                match e {
                    Event::Yell { id, text } if id == boss => lines.push((text, at)),
                    Event::Emote { id, emote: POINT } if id == boss => { point.get_or_insert(at); }
                    Event::Clip { id, clip: CLIP_LOAD, .. } if crew.contains(&id) => loads.push(at),
                    Event::Missile { caster, spell: TRACER, .. } if crew.contains(&caster) => fired.push((caster, at)),
                    _ => {}
                }
            }
        }
        assert_eq!(lines[0].0, "Hold the quarterdeck!");
        assert_eq!(lines[1].0, "Steady... FIRE!");
        assert!(lines[1].1 - lines[0].1 >= 1.95, "the order after his pull line: {lines:?}");
        let point = point.expect("he points his cutlass at a gunner");
        assert!((0.9..1.6).contains(&point), "a second after the pull: {point}");
        assert!(loads[0] > point, "the first load after the order");
        assert!(fired.len() >= 4, "{fired:?}");
        for w in fired.windows(2) {
            assert_ne!(w[0].0, w[1].0, "one gunner at a time, in turn");
            assert!((w[1].1 - w[0].1 - SHOT_GAP).abs() < 0.11, "3.6 sec apart: {fired:?}");
        }
    }
}

#[test]
fn the_captain_never_idles_and_never_lands_a_blow_within_0_8_sec_of_a_shot() {
    for seed in [31, 7, 12] {
        let (mut s, band, boss, crew) = fight_with(1, seed);
        let h = band[0];
        let (mut lands, mut starts, mut fired) = (Vec::new(), Vec::new(), Vec::new());
        let start = s.clock;
        while s.clock - start < 40.0 {
            for e in s.step(DT) {
                match e {
                    Event::MobWindup { id, windup, .. } if id == boss => {
                        starts.push(s.clock);
                        lands.push(s.clock + windup);
                    }
                    Event::Missile { caster, spell: TRACER, .. } if crew.contains(&caster) => fired.push(s.clock),
                    _ => {}
                }
            }
            assert!(!matches!(s.mobs[&boss].state, MobState::Idle { .. } | MobState::Return));
            // The hero stands his ground beside him.
            let p = s.mobs[&boss].pos;
            s.move_player(h, [p[0] - 2.0, p[1], p[2]], 0.0, Moving::default());
        }
        assert!(fired.len() >= 9, "{fired:?}");
        assert!(lands.len() >= 8, "he fights all the while: {starts:?}");
        for l in &lands {
            for f in &fired {
                assert!((l - f).abs() >= CLASH - DT - 1e-3, "a blow at {l} and a shot at {f}");
            }
        }
        let first = starts[0];
        for w in starts.windows(2) {
            assert!(w[1] - w[0] < 6.0, "no long pause: {starts:?}");
        }
        assert!(first - start < 5.0, "his first blow comes soon: {first}");
    }
}

// --- a shot -------------------------------------------------------------------------------------

#[test]
fn a_shot_loads_with_a_click_locks_at_the_shoulder_and_fires_along_its_line() {
    let (mut s, band, _, crew) = fight(1);
    let (h, g) = (band[0], crew[0]);
    let mut ev = Vec::new();
    while !matches!(s.mobs[&g].state, MobState::Windup { .. }) {
        ev = s.step(DT);
    }
    let began = s.clock;
    assert!(ev.iter().any(|e| matches!(e, Event::Clip { id, clip: CLIP_LOAD, .. } if *id == g)), "the load");
    assert!(ev.iter().any(|e| matches!(e, Event::Sound { id, sound: CLICK } if *id == g)), "its click");
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == g)), "no cast of its own over the clips");
    // The barrel follows the hero while he loads...
    let before = s.mobs[&g].facing;
    s.players.get_mut(&h).unwrap().pos[1] += 4.0;
    let mut ev = Vec::new();
    while s.clock - began < LOAD + DT / 2.0 {
        ev.extend(s.step(DT));
    }
    assert!(angle_between(before, s.mobs[&g].facing) > 4.0, "followed while loading");
    let aim = ev.iter().find_map(|e| match e { Event::Clip { id, clip: CLIP_AIM, secs, .. } if *id == g => Some(*secs), _ => None });
    assert!(aim.is_some_and(|secs| (secs - 1.6).abs() < 0.1), "at the shoulder, held 1.6 sec: {aim:?}");
    let locked = s.mobs[&g].facing;
    let at = s.firing_line.aims[&g].to;
    assert!(dist2d(at, s.players[&h].pos) < 1e-3);
    // ...and holds once it's at the shoulder: a sidestep now leaves the line.
    s.players.get_mut(&h).unwrap().pos[1] -= 3.0;
    let mut ev = Vec::new();
    while s.clock - began < 2.2 + DT / 2.0 {
        ev.extend(s.step(DT));
    }
    assert_eq!(s.mobs[&g].facing, locked);
    assert!(ev.iter().any(|e| matches!(e, Event::Clip { id, clip: CLIP_FIRE, .. } if *id == g)), "the shot");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: combat::KIT_FALL_DUST, .. } if *id == g)), "a puff at the muzzle");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 8682, .. } if *caster == g)), "its bang");
    let end = ev.iter().find_map(|e| match e { Event::Missile { caster, spell: TRACER, target: Target::Point(p) } if *caster == g => Some(*p), _ => None })
        .expect("a ball in flight");
    let from = s.mobs[&g].pos;
    assert!((dist2d(from, end) - 32.0).abs() < 0.1, "to its full range");
    assert!(angle_between(facing(from, end), locked) < 0.5, "along the barrel");
    assert!(ev.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", width, reach, .. } if (*width - 1.0).abs() < 1e-3 && (*reach - 16.0).abs() < 0.1)),
        "its 1 yd line drawn with the markers off");
    let fired = s.clock;
    while s.clock - fired < 32.0 / 20.0 + 0.1 {
        ev.extend(s.step(DT));
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: SPLINTERS, target: Target::Point(p) } if *caster == g && dist2d(*p, end) < 1e-3)),
        "splinters where it stops");
    assert!(!ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, .. } if *attacker == g && *v == h && *damage > 0)),
        "the sidestep left the line");
}

/// A gunner's ball laid along +x from the origin, 20 yd at 20 yd/s.
fn ball(s: &mut Survival, owner: Id) {
    let now = s.clock;
    s.line_shots.push(combat::LineShot { owner, from: [0.0, 0.0, 0.0], to: [20.0, 0.0, 0.0], radius: 0.5, speed: 20.0, start: now,
        reached: 0.0, damage: 17, blockable: true, pull_to: None, look: SPLINTERS, struck: Vec::new() });
}

#[test]
fn the_line_strikes_as_the_ball_reaches_a_hero_and_misses_a_sidestep_or_another_floor() {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let owner = s.spawn_mob(s.content.mob("dm_deck_gunner").unwrap(), [0.0, 0.0, 0.0], None, false);
    let at = |s: &mut Survival, name: &str, pos: Vec3| {
        let h = s.join(name, name, None);
        let p = s.players.get_mut(&h).unwrap();
        p.pos = pos;
        p.health = 1000;
        p.max_health = 1000;
        p.facing = 0.0;
        h
    };
    let on = at(&mut s, "on", [10.0, 0.6, 0.0]);
    let beside = at(&mut s, "beside", [10.0, 1.6, 0.0]);
    let below = at(&mut s, "below", [15.0, 0.0, -6.0]);
    let steps = at(&mut s, "steps", [6.0, 0.0, 0.0]);
    let behind = at(&mut s, "behind", [3.0, 2.0, 0.0]);
    ball(&mut s, owner);
    let hurt = |s: &Survival, h: Id| s.players[&h].health < 1000;
    let start = s.clock;
    let tick = |s: &mut Survival| {
        s.clock += DT;
        s.step_pending();
        s.step_missiles();
    };
    while s.clock - start < 0.2 {
        tick(&mut s);
    }
    // Off the line before the ball comes; onto it after it has passed.
    s.players.get_mut(&steps).unwrap().pos[1] = 2.0;
    while s.clock - start < 0.4 {
        tick(&mut s);
    }
    s.players.get_mut(&behind).unwrap().pos[1] = 0.0;
    assert!(!hurt(&s, on), "not before the ball reaches them");
    while s.clock - start < 1.2 {
        tick(&mut s);
    }
    assert!(hurt(&s, on), "struck as it reached them");
    assert_eq!(1000 - s.players[&on].health, 17, "once");
    for h in [beside, below, steps, behind] {
        assert!(!hurt(&s, h), "{}", s.players[&h].name);
    }
    assert!(s.line_shots.is_empty());
}

#[test]
fn a_raised_guard_facing_the_gunner_blocks_his_shot() {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let owner = s.spawn_mob(s.content.mob("dm_deck_gunner").unwrap(), [0.0, 0.0, 0.0], None, false);
    let h = s.join("guard", "Guard", None);
    s.move_player(h, [8.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    s.input(h, Input::Guard);
    let health = s.players[&h].health;
    ball(&mut s, owner);
    let mut ev = Vec::new();
    for _ in 0..30 {
        ev.extend(s.step(DT));
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, .. } if *v == h)));
    assert!(s.players[&h].health + 5 >= health);
}

// --- a gunner falls, then the other -------------------------------------------------------------

#[test]
fn a_fallen_gunners_turns_pass_unshot_and_the_captain_fights_through_them() {
    let (mut s, band, boss, crew) = fight(1);
    let h = band[0];
    let ev = run(&mut s, 0.5);
    assert!(shots(&ev, &crew).is_empty());
    s.hurt_mob(crew[0], h, u32::MAX);
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Sound { id, sound: CLATTER } if *id == crew[0])), "his gun clatters");
    let (mut fired, mut blows) = (Vec::new(), Vec::new());
    let start = s.clock;
    while s.clock - start < 24.0 {
        for e in s.step(DT) {
            match e {
                Event::Missile { caster, spell: TRACER, .. } if crew.contains(&caster) => fired.push((caster, s.clock)),
                Event::MobWindup { id, .. } if id == boss => blows.push(s.clock),
                _ => {}
            }
        }
        let p = s.mobs[&boss].pos;
        s.move_player(h, [p[0] - 2.0, p[1], p[2]], 0.0, Moving::default());
    }
    assert!(fired.iter().all(|(g, _)| *g == crew[1]), "only the living gunner fires");
    assert!(fired.len() >= 2);
    for w in fired.windows(2) {
        assert!((w[1].1 - w[0].1 - 2.0 * SHOT_GAP).abs() < 0.11, "half the shots: {fired:?}");
    }
    // The dead man's turn is no pause: the captain swings through it.
    let (a, b) = (fired[0].1, fired[1].1);
    assert!(blows.iter().any(|t| *t > a && *t < b), "{blows:?} between {a} and {b}");
    assert_eq!(s.mobs[&boss].phase, 0, "one gunner down changes nothing else");
    assert!(!s.firing_pattern_allowed(boss, "pistol shot"));
}

#[test]
fn the_last_gunner_falling_brings_a_real_draw_a_load_and_a_shot_and_the_gun_stays_out() {
    let (mut s, band, boss, crew) = fight(1);
    let h = band[0];
    let hp = s.mobs[&boss].health;
    s.hurt_mob(boss, h, hp - 1);
    run(&mut s, 3.0);
    assert!(!s.firing_alone(boss), "health never escalates him");
    for g in &crew {
        s.hurt_mob(*g, h, u32::MAX);
    }
    let (mut said, mut drawn) = (false, false);
    while !drawn && s.clock < 40.0 {
        for e in s.step(DT) {
            said |= matches!(&e, Event::Yell { id, text } if *id == boss && text == "I'll do it meself.");
            drawn |= matches!(e, Event::Clip { id, clip: CLIP_SHEATH, .. } if id == boss);
        }
    }
    let drew = s.clock;
    assert!(drawn, "the draw");
    assert_eq!(s.firing_line_gun(boss), None, "not yet in hand");
    let mut beats = Vec::new();
    while s.clock - drew < 2.4 {
        for e in s.step(DT) {
            said |= matches!(&e, Event::Yell { id, text } if *id == boss && text == "I'll do it meself.");
            match e {
                Event::MobGear { id } if id == boss && s.firing_line_gun(boss).is_some() => beats.push(("gun", s.clock - drew)),
                Event::Clip { id, clip: CLIP_LOAD, .. } if id == boss => beats.push(("load", s.clock - drew)),
                Event::Sound { id, sound: CLICK } if id == boss => beats.push(("click", s.clock - drew)),
                Event::Clip { id, clip: CLIP_AIM, secs, .. } if id == boss => beats.push(("aim", secs)),
                Event::Missile { caster, spell: TRACER, .. } if caster == boss => beats.push(("shot", s.clock - drew)),
                _ => {}
            }
        }
    }
    let when = |k: &str| beats.iter().find(|b| b.0 == k).unwrap_or_else(|| panic!("{k}: {beats:?}")).1;
    assert!((when("gun") - DRAW_SWAP).abs() < 0.06, "{beats:?}");
    assert!((when("load") - DRAW).abs() < 0.06 && (when("click") - DRAW).abs() < 0.06);
    assert!((when("aim") - 1.2).abs() < 0.08, "a 1.2 sec aim: {beats:?}");
    assert!((when("shot") - 2.3).abs() < 0.06, "{beats:?}");
    assert_eq!(s.mobs[&boss].phase, 1);
    let ev = beside(&mut s, h, boss, 2.5);
    assert!(said || ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == "I'll do it meself.")), "his line");
    assert!(s.content.mobs[s.mobs[&boss].def].phases.is_empty());
    let shot = drew + when("shot");
    while s.clock - shot < GUN_OUT - 0.2 {
        beside(&mut s, h, boss, DT);
    }
    assert_eq!(s.firing_line_gun(boss), Some(GUN), "out about 6 sec");
    let ev = beside(&mut s, h, boss, 0.8);
    assert!(ev.iter().any(|e| matches!(e, Event::Clip { id, clip: CLIP_SHEATH, .. } if *id == boss)), "put away the same way");
    assert_eq!(s.firing_line_gun(boss), None);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew, "no replacement crew");
}

#[test]
fn alone_he_has_no_pause_and_shoots_only_a_kiting_hero_every_10_sec() {
    let (s, _, boss, _) = fight(1);
    assert!(s.attack_now(boss).cooldown > 0.0);
    let (mut s, h, boss) = alone();
    assert_eq!(s.attack_now(boss).cooldown, 0.0, "no pauses: the escalation");
    assert!(!s.firing_pattern_allowed(boss, "pistol shot"), "not again so soon");
    let shot = s.firing_line.crews[&boss].gun_ready - GUN_EVERY;
    while s.clock < shot + GUN_EVERY - 0.1 {
        let p = s.mobs[&boss].pos;
        s.move_player(h, [p[0] - 2.0, p[1], p[2]], 0.0, Moving::default());
        run(&mut s, DT);
        assert!(!s.firing_pattern_allowed(boss, "pistol shot"));
    }
    run(&mut s, 0.2);
    assert!(s.firing_pattern_allowed(boss, "pistol shot"));
    let d = &s.content.mobs[s.mobs[&boss].def];
    assert_eq!(d.patterns.iter().find(|p| p.name == "pistol shot").unwrap().min_range, 6.0, "only at a hero away from him");
}

// --- the harpoon --------------------------------------------------------------------------------

#[test]
fn the_harpoon_waits_for_a_hero_who_keeps_8_yd_away_for_2_sec() {
    let (mut s, h, boss) = alone();
    let kite = |s: &mut Survival, yards: f32| {
        let p = s.mobs[&boss].pos;
        s.move_player(h, [p[0] - yards, p[1], p[2]], 0.0, Moving::default());
        s.firing_line.crews.get_mut(&boss).unwrap().gun_ready = f32::MAX;
        run(s, DT);
    };
    kite(&mut s, 5.0);
    assert!(!s.firing_pattern_allowed(boss, "harpoon"), "not at a hero beside him");
    for _ in 0..30 {
        kite(&mut s, 10.0);
    }
    assert!(!s.firing_pattern_allowed(boss, "harpoon"), "1.5 sec is not long enough");
    kite(&mut s, 5.0);
    for _ in 0..30 {
        kite(&mut s, 10.0);
    }
    assert!(!s.firing_pattern_allowed(boss, "harpoon"), "coming close starts it again");
    for _ in 0..12 {
        kite(&mut s, 10.0);
    }
    assert!(s.firing_line.crews[&boss].far_since.is_some_and(|t| s.clock - t >= HARPOON_WAIT));
    assert_eq!(s.content.mobs[s.mobs[&boss].def].patterns.iter().find(|p| p.name == "harpoon").unwrap().min_range, 8.0);
}

/// The harpoon begun at a hero `yards` off, he held still for it.
fn harpoon(yards: f32) -> (Survival, Id, Id) {
    let (mut s, h, boss) = alone();
    let p = s.mobs[&boss].pos;
    s.move_player(h, [p[0] - yards, p[1], p[2]], 0.0, Moving::default());
    s.stagger_mob(boss, 0.01, false);
    s.mobs.get_mut(&boss).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    let k = s.content.mobs[s.mobs[&boss].def].patterns.iter().position(|p| p.name == "harpoon").unwrap();
    s.begin_pattern(boss, k, false);
    s.windup(boss, Foe::Hero(h), s.players[&h].pos);
    (s, h, boss)
}

#[test]
fn the_harpoon_is_held_back_in_his_hand_then_hauls_a_hit_hero_to_3_yd_into_his_clearing_cut() {
    let (mut s, h, boss) = harpoon(11.0);
    let ev = std::mem::take(&mut s.events);
    assert_eq!(s.firing_line_spear(boss), Some(SPEAR), "the harpoon in his hand");
    let held = ev.iter().find_map(|e| match e { Event::Clip { id, clip: CLIP_THROW_HOLD, secs, at, .. } if *id == boss => Some((*secs, *at)), _ => None });
    assert!(held.is_some_and(|(secs, at)| (secs - 1.7).abs() < 0.01 && (at - secs).abs() < 0.01), "held back the full 1.7 sec: {held:?}");
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { .. })));
    let mut ev = Vec::new();
    let start = s.clock;
    while s.clock - start < 1.75 {
        ev.extend(s.step(DT));
        s.move_player(h, s.players[&h].pos, 0.0, Moving::default());
    }
    assert!(ev.iter().any(|e| matches!(e, Event::Clip { id, clip: CLIP_THROW, .. } if *id == boss)), "the throw");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 6533, .. } if *caster == boss)), "the line in flight");
    assert_eq!(s.firing_line_spear(boss), None, "out of his hand");
    let (mut hop, mut cut) = (None, false);
    while s.clock - start < 3.0 && hop.is_none() {
        for e in s.step(DT) {
            match e {
                Event::Hop { player, dir, speed, .. } if player == h => hop = Some((dir, speed)),
                Event::MobWindup { id, .. } if id == boss => cut = s.firing_pattern_name(boss) == Some("clearing cut"),
                _ => {}
            }
        }
    }
    let (dir, _) = hop.expect("hauled in");
    let to = s.mobs[&boss].pos;
    assert!(angle_between(dir, facing(s.players[&h].pos, to)) < 5.0, "toward him");
    assert!(cut, "straight into his clearing cut");
    assert!(matches!(s.mobs[&boss].state, MobState::Windup { .. }));
}

#[test]
fn a_sidestep_or_a_guard_spares_the_harpoons_haul() {
    for guard in [false, true] {
        let (mut s, h, boss) = harpoon(11.0);
        s.players.get_mut(&h).unwrap().rtt = 0.25;
        if guard {
            let to = s.mobs[&boss].pos;
            s.move_player(h, s.players[&h].pos, facing(s.players[&h].pos, to), Moving::default());
            s.input(h, Input::Guard);
        }
        let start = s.clock;
        let mut ev = Vec::new();
        while s.clock - start < 1.75 {
            ev.extend(s.step(DT));
        }
        if !guard {
            s.players.get_mut(&h).unwrap().pos[1] += 3.0;
        }
        while s.clock - start < 3.5 {
            ev.extend(s.step(DT));
        }
        assert!(!ev.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == h)), "guard {guard}: no haul");
        if guard {
            assert!(ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, .. } if *v == h)));
        }
    }
}

// --- the fight's bounds -------------------------------------------------------------------------

#[test]
fn a_party_death_retargets_the_same_finite_crew() {
    let (mut s, band, boss, crew) = fight(2);
    let (first, second) = (band[0], band[1]);
    run(&mut s, 1.3);
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    run(&mut s, 0.1);
    assert_eq!(s.mobs[&boss].state.target(), Some(second));
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
    assert!(crew.iter().all(|g| s.mobs[g].state.target() == Some(second)));
    run(&mut s, 12.0);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
}

#[test]
fn a_wipe_puts_the_crew_back_at_their_posts_and_his_death_clears_every_shot() {
    let (mut s, band, boss, crew) = fight(1);
    let h = band[0];
    let posts: Vec<Vec3> = crew.iter().map(|g| s.mobs[g].pos).collect();
    run(&mut s, 2.5);
    s.hurt_mob(crew[0], h, u32::MAX);
    s.players.get_mut(&h).unwrap().health = 0;
    run(&mut s, 0.1);
    assert!(crew.iter().all(|g| !s.mobs.contains_key(g)), "the crew goes, the fallen too");
    assert!(s.firing_line.aims.is_empty() && s.line_shots.iter().all(|l| !crew.contains(&l.owner)));
    run(&mut s, 8.0);
    let back = s.firing_line.crews[&boss].gunners.clone();
    assert_eq!(back.len(), 2);
    assert!(back.iter().all(|g| !crew.contains(g)));
    for (g, post) in back.iter().zip(&posts) {
        assert!(dist2d(s.mobs[g].pos, *post) < 0.1, "back at his post");
        assert_eq!(s.mobs[g].health, s.mobs[g].max_health, "whole");
    }
    assert_eq!(s.firing_line.crews[&boss].stage, Stage::Posted);
    // Pulled again, one gunner felled, then the captain: the fallen lie, nothing outlives him.
    s.players.get_mut(&h).unwrap().health = 100_000;
    s.engage(boss, Foe::Hero(h));
    run(&mut s, 3.6);
    s.hurt_mob(back[0], h, u32::MAX);
    run(&mut s, 2.0);
    s.hurt_mob(boss, h, u32::MAX);
    assert!(matches!(s.mobs.get(&back[0]).map(|m| &m.state), Some(MobState::Dead { .. })), "the fallen lie where they fell");
    assert!(!s.mobs.contains_key(&back[1]), "the living go with him");
    assert!(s.firing_line.crews.is_empty() && s.firing_line.aims.is_empty() && s.firing_line.owners.is_empty());
    assert!(s.line_shots.iter().all(|l| l.owner != boss && !back.contains(&l.owner)));
    assert!(s.pending.iter().all(|p| p.attacker != boss && !back.contains(&p.attacker)));
    s.reset_firing_lines();
    assert!(s.firing_line.guns.is_empty() && s.firing_line.spears.is_empty());
}

#[test]
fn gunner_posts_reject_water_and_the_wrong_floor_before_spawning() {
    struct Ship;
    impl Land for Ship {
        fn height(&self, x: f32, y: f32) -> f32 { if y > 1.0 && x > 0.0 { -16.0 } else { 0.0 } }
        fn water(&self, x: f32, y: f32) -> bool { y > 1.0 && x < 0.0 }
    }
    let (mut s, _, boss, _) = posted(1);
    s.reset_firing_lines();
    s.land = Box::new(Ship);
    run(&mut s, DT);
    let c = &s.firing_line.crews[&boss];
    assert_eq!(c.gunners.len(), 2);
    for id in &c.gunners {
        let p = s.mobs[id].pos;
        assert_eq!(p[2], 0.0);
        assert!(!s.land.water_near(p[0], p[1], p[2]));
        assert!(s.land.walk_clear(s.mobs[&boss].home, p));
    }
}

#[test]
fn ordinary_sword_inputs_can_clear_the_crew_and_captain_solo_and_duo() {
    // Controlled pressure fixture: normal input, sword and health scaling, inflated hero HP.
    for heroes in [1, 2] {
        let (mut s, band, boss, crew) = fight(heroes);
        let start = s.clock;
        while s.clock - start < 150.0 && s.mobs[&boss].health > 0 {
            let target = crew.iter().copied().find(|g| s.mobs.get(g).is_some_and(|m| m.health > 0)).unwrap_or(boss);
            let at = s.mobs[&target].pos;
            for hero in &band {
                let p = &s.players[hero];
                let face = facing(p.pos, at);
                let pos = if dist2d(p.pos, at) > 1.8 {
                    [p.pos[0] + face.cos() * 0.3, p.pos[1] + face.sin() * 0.3, p.pos[2]]
                } else { p.pos };
                let ready = matches!(p.action, Action::Ready);
                s.move_player(*hero, pos, face, Moving::default());
                if ready { s.input(*hero, Input::Attack); }
            }
            run(&mut s, DT);
        }
        eprintln!("Greenskin sword fixture heroes={heroes} seconds={:.2}", s.clock - start);
        assert_eq!(s.mobs[&boss].health, 0);
        assert!(s.firing_line.crews.is_empty());
    }
}

#[test]
fn a_solo_vanish_keeps_the_encounter_and_its_existing_crew() {
    let (mut s, band, boss, crew) = fight(1);
    let h = band[0];
    s.apply_cc(h, h, "veil", 20.0, 0.0, 0.0, "test");
    assert!(s.has_cc(h, "veil"));
    run(&mut s, 9.0);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
    assert!(s.mobs[&boss].state.foe().is_some());
    assert!(crew.iter().all(|g| !s.unseen_by(*g, h)));
}

#[test]
fn normal_charged_projectile_inputs_can_kill_a_gunner() {
    let (mut s, band, boss, crew) = fight(1);
    let (h, g) = (band[0], crew[0]);
    s.slot_ability(h, "ranged", "frostbolt").unwrap();
    let key = crate::content::SLOTS.iter().position(|k| *k == "ranged").unwrap() as u8;
    for _ in 0..12 {
        if !s.mobs.get(&g).is_some_and(|m| m.health > 0) { break; }
        let at = s.mobs[&g].pos;
        // Reposition to a clear flank between casts. The captain is a real blocking body;
        // standing on one ray and firing through him tests interception, not gunner access.
        let flank = facing(s.mobs[&boss].pos, at) + std::f32::consts::FRAC_PI_2;
        let pos = [at[0] + flank.cos() * 24.0, at[1] + flank.sin() * 24.0, at[2]];
        s.move_player(h, pos, facing(pos, at), Moving::default());
        s.set_aim_line(h, facing(pos, at), -(0.9f32).atan2(24.0));
        s.input(h, Input::Ability(key, false));
        s.input(h, Input::Attack);
        run(&mut s, 1.2);
        s.set_aim_line(h, facing(pos, at), -(0.9f32).atan2(24.0));
        s.input(h, Input::AttackUp);
        run(&mut s, 2.6);
    }
    assert!(!s.mobs.get(&g).is_some_and(|m| m.health > 0), "normal ranged input must reach and kill the deck crew");
}

#[test]
fn a_gunner_keeps_normal_knockback_motion_while_his_ai_holds_the_post() {
    let (mut s, band, _, crew) = fight(1);
    let g = crew[0];
    let before = s.mobs[&g].pos;
    let effect = crate::content::AbilityEffect { kind: "knockback".into(), amount: 3.0, from_caster: true, ..Default::default() };
    s.displace(band[0], g, &effect);
    run(&mut s, 0.4);
    assert!(dist2d(s.mobs[&g].pos, before) > 2.0);
    assert!(s.firing_line.owners.contains_key(&g));
}

#[test]
fn killing_the_captain_first_clears_his_elite_pack_and_pays_once() {
    let mut s = Survival::new_dungeon(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let h = s.join("boarder", "Boarder", None);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_greenskin").unwrap().0;
    let home = s.mobs[&boss].home;
    s.players.get_mut(&h).unwrap().pos = home;
    s.clean_firing_lines();
    let crew = s.firing_line.crews[&boss].gunners.clone();
    assert_eq!(crew.len(), 2, "at their posts before the pull");
    s.engage(boss, Foe::Hero(h));
    s.clean_firing_lines();
    assert!(!s.slain("greenskin"));
    s.hurt_mob(boss, h, u32::MAX);
    assert!(s.slain("greenskin"));
    assert!(crew.iter().all(|g| !s.mobs.contains_key(g)));
    let bags = s.players[&h].bag.clone();
    let kills = s.camp.counts.get("kill:dm_greenskin").copied();
    s.hurt_mob(boss, h, u32::MAX);
    assert_eq!(s.players[&h].bag, bags);
    assert_eq!(s.camp.counts.get("kill:dm_greenskin").copied(), kills);
    assert_eq!(kills, Some(1));
}

// --- a hero who plays by what they see ---------------------------------------------------------

/// What a solo clear cost a hero playing by the fight's tells alone.
#[derive(Debug, Default)]
struct Cleared {
    cleared: bool,
    secs: f32,
    health_left: u32,
    gunners_felled: u32,
    shots: u32,
    shots_taken: u32,
    sidesteps: u32,
    blows: u32,
    blows_taken: u32,
    alone_at: Option<f32>,
}

/// A warrior with the starting sword and their own health, reading only what the client shows:
/// the gunners at their posts, a gun at the shoulder (AN clip 48: step off the line the barrel
/// points along as it fires), the harpoon held back (clip 108: the same), and the captain's
/// windups (raise the guard as they land). `crew_first`: they go for the nearest gunner first, the
/// captain chasing, then duel him; else they duel him from the start, the crew firing throughout.
fn clear_by_tells(seed: u64, crew_first: bool) -> Cleared {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), seed, 0.0);
    s.content.pvp = Default::default();
    let hero = s.join("hero", "Hero", None);
    s.move_player(hero, [-18.0, 0.0, 0.0], 0.0, Moving::default());
    let boss = s.spawn_mob(s.content.mob("dm_greenskin").unwrap(), [0.0; 3], None, false);
    run(&mut s, DT);
    let crew = s.firing_line.crews[&boss].gunners.clone();
    let reaction = crate::tests_fair::REACTION;
    let weapon = s.weapon_of(&s.players[&hero]).clone();
    let mut out = Cleared::default();
    let start = s.clock;
    // Seen: each creature's facing as the client was last told it.
    let mut faces: BTreeMap<Id, f32> = BTreeMap::new();
    // A line about to fire: the shooter, when it fires, when the hero saw it raised.
    let mut lines: Vec<(Id, f32, f32)> = Vec::new();
    // His blow seen coming: when it lands, when it was seen.
    let mut blow: Option<(f32, f32)> = None;
    // Stepping aside: which way, until when.
    let mut aside: Option<(f32, f32)> = None;
    while s.clock - start < 300.0 {
        let now = s.clock;
        let Some(m) = s.mobs.get(&boss) else { out.cleared = true; break };
        if m.health == 0 { out.cleared = true; break; }
        if s.players[&hero].health == 0 { break; }
        let me = s.players[&hero].pos;
        let bpos = m.pos;
        let target = crew.iter().copied().filter(|g| crew_first && s.mobs.get(g).is_some_and(|m| m.health > 0))
            .min_by(|a, b| dist2d(s.mobs[a].pos, me).total_cmp(&dist2d(s.mobs[b].pos, me))).unwrap_or(boss);
        let tpos = s.mobs[&target].pos;
        let toward = facing(me, tpos);
        let action_ready = matches!(s.players[&hero].action, Action::Ready);
        let guarding = matches!(s.players[&hero].action, Action::Guard);
        let mut acted = false;
        // A gun (or the harpoon) about to fire along a line through them: step off it, late.
        if aside.is_none() {
            if let Some(&(shooter, fires, _)) = lines.iter().find(|(_, fires, seen)| now >= *seen && *fires - now < 0.4) {
                if let (Some(sm), Some(f)) = (s.mobs.get(&shooter), faces.get(&shooter)) {
                    let (dx, dy) = (me[0] - sm.pos[0], me[1] - sm.pos[1]);
                    let side = -dx * f.sin() + dy * f.cos();
                    let ahead = dx * f.cos() + dy * f.sin();
                    if ahead > -1.0 && side.abs() < 1.8 {
                        let way = f + if side >= 0.0 { std::f32::consts::FRAC_PI_2 } else { -std::f32::consts::FRAC_PI_2 };
                        aside = Some((way, now + 0.4));
                        out.sidesteps += 1;
                    }
                }
                lines.retain(|l| l.0 != shooter);
            }
        }
        lines.retain(|l| l.1 > now - 0.5);
        if let Some((way, until)) = aside {
            let step = 7.0 * DT;
            s.move_player(hero, [me[0] + way.cos() * step, me[1] + way.sin() * step, me[2]], toward, Moving { left: true, ..Moving::default() });
            acted = true;
            if now >= until { aside = None; }
        }
        // His blow: the guard up, facing him, as it lands.
        if let Some((land, seen)) = blow {
            if now >= seen && land - now <= 0.12 && !guarding && dist2d(me, bpos) < 8.0 {
                s.move_player(hero, s.players[&hero].pos, facing(me, bpos), Moving::default());
                s.input(hero, Input::Guard);
                acted = true;
            }
            if now > land + 0.15 {
                blow = None;
                if guarding { s.input(hero, Input::GuardUp); }
            }
        } else if guarding {
            s.input(hero, Input::GuardUp);
        }
        let threat = blow.is_some_and(|(land, _)| land - now < 0.6);
        if !acted && action_ready && !threat {
            let reach = weapon.range + 0.4;
            if dist2d(me, tpos) <= reach {
                s.move_player(hero, me, toward, Moving::default());
                if s.players[&hero].stamina > 25.0 {
                    s.input(hero, Input::Attack);
                }
            } else {
                let step = 7.0 * DT;
                s.move_player(hero, [me[0] + toward.cos() * step, me[1] + toward.sin() * step, me[2]], toward, Moving { forward: true, ..Moving::default() });
            }
        }
        for e in s.step(DT) {
            match e {
                Event::MobStop { id, facing, .. } => { faces.insert(id, facing); }
                Event::Clip { id, clip: CLIP_AIM | CLIP_THROW_HOLD, secs, .. } => {
                    let f = s.mobs.get(&id).map_or(0.0, |m| m.facing);
                    faces.entry(id).or_insert(f);
                    lines.push((id, s.clock + secs, s.clock + reaction));
                }
                Event::MobWindup { id, windup, .. } if id == boss => blow = Some((s.clock + windup, s.clock + reaction)),
                Event::Missile { caster, spell: TRACER | 6533, .. } if crew.contains(&caster) || caster == boss => out.shots += 1,
                Event::Swing { attacker, victim: Some(v), damage, .. } if v == hero && damage > 0 => {
                    if attacker == boss && s.line_shots.iter().all(|l| l.owner != boss) { out.blows_taken += 1; } else { out.shots_taken += 1; }
                }
                Event::Swing { attacker, victim: Some(v), outcome: Outcome::Blocked, .. } if v == hero && attacker == boss => out.blows += 1,
                Event::Died { id } if crew.contains(&id) && s.mobs.get(&boss).is_some_and(|m| m.health > 0) => out.gunners_felled += 1,
                _ => {}
            }
        }
        if out.alone_at.is_none() && s.firing_alone(boss) { out.alone_at = Some(s.clock - start); }
    }
    out.secs = s.clock - start;
    out.health_left = s.players[&hero].health;
    out
}

#[test]
fn a_lone_sword_hero_playing_by_the_tells_clears_the_crew_then_the_captain() {
    for seed in [3, 5, 11] {
        let out = clear_by_tells(seed, true);
        eprintln!("Greenskin crew first, seed {seed}: {out:?}");
        assert!(out.cleared, "{out:?}");
        assert!(out.health_left > 0);
        assert_eq!(out.gunners_felled, 2, "both gunners by hand: {out:?}");
        assert!(out.alone_at.is_some());
        assert!(out.shots >= 1 && out.shots_taken * 2 <= out.shots, "the lines can be read and left: {out:?}");
    }
}

#[test]
fn a_lone_sword_hero_can_instead_duel_the_captain_stepping_out_of_the_lines() {
    for seed in [3, 5, 11] {
        let out = clear_by_tells(seed, false);
        eprintln!("Greenskin duel, seed {seed}: {out:?}");
        assert!(out.cleared, "{out:?}");
        assert!(out.health_left > 0);
        assert_eq!(out.gunners_felled, 0, "the crew goes with him: {out:?}");
        assert!(out.shots >= 4 && out.sidesteps >= 3, "the crew fired all through: {out:?}");
        assert!(out.shots_taken * 2 <= out.shots, "most lines left: {out:?}");
        // Breaking the line first meets two or three shots and takes none (the test above); a
        // duel through the volleys meets 16 or more and costs several.
        assert!(out.shots >= 12 && out.shots_taken >= 3, "the duel is the harder way: {out:?}");
    }
}


// --- the starboard rail ------------------------------------------------------------------------

const FENCE: u32 = 6151;

/// The Deadmines as built, with a hero on the quarterdeck by its open side.
fn deck(at: Vec3) -> (Survival, Id) {
    let mut s = Survival::new_dungeon(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let h = s.join("boarder", "Boarder", None);
    let p = s.players.get_mut(&h).unwrap();
    // (Its arrival's firm ground is the entrance, 40 yd up on the real map: here, the deck.)
    (p.pos, p.last_safe, p.stamina) = (at, at, 100.0);
    (s, h)
}

/// How far south (toward the open side, -y) the rail's line is at `x`.
fn rail_y(s: &Survival, x: f32) -> f32 {
    let d = s.dungeon_def().unwrap();
    let r = d.rails.iter().find(|r| r.key == "juggernaut_starboard").unwrap();
    let w = r.points.windows(2).find(|w| (w[0][0]..=w[1][0]).contains(&x)).unwrap();
    let t = (x - w[0][0]) / (w[1][0] - w[0][0]);
    w[0][1] + (w[1][1] - w[0][1]) * t
}

fn hop_yards(ev: &[Event], h: Id) -> Option<f32> {
    ev.iter().find_map(|e| match e {
        Event::Hop { player, speed, lift, .. } if *player == h => Some(speed * super::super::moves::airtime(*lift)),
        _ => None,
    })
}

/// How far `p` is from a rail's line, on the ground.
fn off_line(r: &crate::content::RailDef, p: Vec3) -> f32 {
    r.points.windows(2).map(|w| combat::along_segment(w[0], w[1], p).1).fold(f32::MAX, f32::min)
}

#[test]
fn the_bulwark_runs_along_every_open_side_in_pieces_that_meet() {
    let (s, _) = deck([-62.0, -830.0, 0.0]);
    let d = s.dungeon_def().unwrap();
    let keys: Vec<&str> = d.rails.iter().map(|r| r.key.as_str()).collect();
    assert_eq!(keys, ["juggernaut_starboard", "juggernaut_port", "juggernaut_trench"]);
    for (k, r) in d.rails.iter().enumerate() {
        let len = (r.run[1] - r.run[0]) * r.scale;
        // In the order they were laid (ids rise), from the line's start to its end.
        let mut pieces: Vec<(Id, dungeon::RailPiece)> = s.objs.iter()
            .filter(|(_, o)| matches!(o.kind, ObjKind::Prop { display, .. } if display == r.display))
            .filter_map(|(id, _)| s.rail_piece(*id).map(|p| (*id, p))).filter(|(_, p)| p.rail == k).collect();
        pieces.sort_by_key(|(id, _)| *id);
        assert!(!pieces.is_empty(), "{}", r.key);
        let (first, last) = (r.points[0], *r.points.last().unwrap());
        assert!(dist2d(pieces[0].1.from, first) < 0.01 && dist2d(pieces.last().unwrap().1.to, last) < 0.01, "{}: from end to end", r.key);
        for w in pieces.windows(2) {
            // End to end, or (the last) starting further back along the one before: never a gap.
            let joined = dist2d(w[0].1.to, w[1].1.from) < 0.01 || off_line(r, w[1].1.from) < 0.05 && dist2d(w[1].1.to, last) < 0.01;
            assert!(joined, "{}: a gap between {:?} and {:?}", r.key, w[0].1, w[1].1);
        }
        for (id, p) in &pieces {
            assert!((dist2d(p.from, p.to) - len).abs() < 0.05, "{}: a whole piece's chord: {p:?}", r.key);
            for end in [p.from, p.to] {
                assert!(off_line(r, end) < 0.05, "{}: {end:?} on the line", r.key);
                let top = end[2] + r.lift + r.height[1] * r.scale;
                assert!(top - end[2] >= 1.73, "{}: its top {:.2} over the deck: clear of a 1 yd step up and a hop by half a yard", r.key, top - end[2]);
                assert!(end[2] + r.lift + r.height[0] * r.scale <= end[2], "{}: seated on the deck", r.key);
            }
            // A solid piece: the clients keep its hull (it isn't one of the passable props).
            assert!(!s.is_trap_obj(*id) && !s.is_perch_visual_prop(*id) && !s.is_kitchen_prop(*id) && s.collapse_piece_tilt(*id).is_none());
        }
    }
}

#[test]
fn every_rail_has_the_deck_on_its_left() {
    // The captain's home, on the main deck: inside every rail's line.
    let (s, _) = deck([-62.0, -830.0, 0.0]);
    let home = [-62.0, -815.0];
    for r in &s.dungeon_def().unwrap().rails {
        for w in r.points.windows(2) {
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let side = dx * (home[1] - w[0][1]) - dy * (home[0] - w[0][0]);
            assert!(side > 0.0, "{}: the deck is on the left of {:?} -> {:?}", r.key, w[0], w[1]);
        }
    }
}

#[test]
fn the_server_stop_holds_at_every_section_of_every_rail() {
    let (s0, _) = deck([-62.0, -830.0, 0.0]);
    for r in s0.dungeon_def().unwrap().rails.clone() {
        for w in r.points.windows(2) {
            let mid = [(w[0][0] + w[1][0]) / 2.0, (w[0][1] + w[1][1]) / 2.0];
            let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
            let l = (dx * dx + dy * dy).sqrt();
            // The deck's side is the line's left.
            let (nx, ny) = (-dy / l, dx / l);
            let inside = [mid[0] + nx * 0.6, mid[1] + ny * 0.6, 0.0];
            let outside = [mid[0] - nx * 1.5, mid[1] - ny * 1.5, 0.0];
            let (mut s, h) = deck(inside);
            s.move_player(h, outside, 0.0, Moving::default());
            let at = s.players[&h].pos;
            assert!(dist2d(at, inside) < 0.7 && (at[0] - mid[0]) * nx + (at[1] - mid[1]) * ny > 0.0,
                "{}: held on the deck at {:?} -> {:?}: {at:?}", r.key, w[0], w[1]);
            // And back aboard over it is free.
            let (mut s, h) = deck(outside);
            s.move_player(h, inside, 0.0, Moving::default());
            assert_eq!(s.players[&h].pos, inside, "{}", r.key);
        }
    }
}

/// A rotation (a game object's quaternion) applied to a vector.
fn turn(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let u = [x, y, z];
    let cross = |a: [f32; 3], b: [f32; 3]| [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]];
    let t = cross(u, v).map(|c| 2.0 * c);
    let ut = cross(u, t);
    [v[0] + w * t[0] + ut[0], v[1] + w * t[1] + ut[1], v[2] + w * t[2] + ut[2]]
}

#[test]
fn each_rail_piece_lies_along_its_chord_tipped_to_follow_the_deck() {
    // A deck rising toward +x, as the Juggernaut's toward its bow.
    struct Rising;
    impl Land for Rising {
        fn height(&self, x: f32, _: f32) -> f32 { 0.09 * (x + 100.0) }
    }
    let s = Survival::new_dungeon(Content::dev(), Box::new(Rising), 31, 0.0);
    let r = s.dungeon_def().unwrap().rails[0].clone();
    let (mut n, mut tipped) = (0, false);
    for (id, o) in s.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Prop { display, .. } if display == r.display)) {
        let Some(p) = s.rail_piece(*id).filter(|p| p.rail == 0) else { continue };
        n += 1;
        let run = if r.along_y { [0.0, 1.0, 0.0] } else { [1.0, 0.0, 0.0] };
        let along = turn(p.rotation, run);
        let chord = [p.to[0] - p.from[0], p.to[1] - p.from[1], p.to[2] - p.from[2]];
        let l = (chord[0].powi(2) + chord[1].powi(2) + chord[2].powi(2)).sqrt();
        let dot = (along[0] * chord[0] + along[1] * chord[1] + along[2] * chord[2]) / l;
        assert!(dot > 0.999, "its run along its chord, rise and all: {along:?} vs {chord:?}");
        // Its ends sit on the deck, so it rises or falls with it.
        let rise = 0.09 * (p.to[0] - p.from[0]);
        assert!((p.to[2] - p.from[2] - rise).abs() < 0.01, "the deck rises {rise} under it, the piece {}", p.to[2] - p.from[2]);
        tipped |= rise.abs() > 0.5;
        // Upright: its own up stays up but for the tip along its run.
        assert!(turn(p.rotation, [0.0, 0.0, 1.0])[2] > 0.99);
        // Its run's start, from its origin, is the chord's start (at `lift` over it).
        let start = [o.pos[0] + along[0] * r.run[0] * r.scale, o.pos[1] + along[1] * r.run[0] * r.scale, o.pos[2] + along[2] * r.run[0] * r.scale];
        assert!(dist2d(start, p.from) < 0.05 && (start[2] - (p.from[2] + r.lift)).abs() < 0.05, "{start:?} vs {:?}", p.from);
    }
    assert!(n >= 5 && tipped, "{n} pieces");
}

#[test]
fn a_piece_over_a_crown_in_the_deck_rises_over_it_by_up_to_its_sink() {
    // A deck crowned along x (as the port walkway's), 0.19 yd over a piece's chord at its middle.
    struct Crowned;
    impl Land for Crowned {
        fn height(&self, x: f32, _: f32) -> f32 { 40.0 - 0.008 * (x + 70.0).powi(2) }
    }
    let s = Survival::new_dungeon(Content::dev(), Box::new(Crowned), 31, 0.0);
    let r = s.dungeon_def().unwrap().rails[0].clone();
    let deck = |x: f32| 40.0 - 0.008 * (x + 70.0).powi(2);
    let (mut n, mut raised) = (0, false);
    for id in s.objs.keys() {
        let Some(p) = s.rail_piece(*id).filter(|p| p.rail == 0) else { continue };
        n += 1;
        for end in [p.from, p.to] {
            let over = end[2] - deck(end[0]);
            assert!((-1e-3..=-r.lift + 1e-3).contains(&over), "its ends meet the deck: {over}");
            raised |= over > 0.1;
        }
        let mid = [(p.from[0] + p.to[0]) / 2.0, (p.from[2] + p.to[2]) / 2.0];
        let top = mid[1] + r.lift + r.height[1] * r.scale - deck(mid[0]);
        assert!(top >= 1.73, "its top stands {top} over the crown");
    }
    assert!(n >= 5 && raised, "{n} pieces");
}

#[test]
fn a_line_cut_into_chords_covers_it_end_to_end() {
    let line = [[0.0, 0.0, 0.0], [10.0, 0.0, 1.0], [14.0, 4.0, 2.0], [14.0, 12.0, 3.0]];
    let chords = dungeon::rail_chords(&line, 6.0);
    assert_eq!(dist2d(chords[0].0, line[0]), 0.0);
    assert!(dist2d(chords.last().unwrap().1, line[3]) < 1e-4);
    for (a, b) in &chords {
        assert!((dist2d(*a, *b) - 6.0).abs() < 1e-3, "{a:?} {b:?}");
    }
    let n = chords.len();
    for k in 1..n - 1 {
        assert!(dist2d(chords[k - 1].1, chords[k].0) < 1e-3, "end to end");
    }
}

#[test]
fn a_walk_a_roll_or_a_jump_over_the_rail_is_put_back_on_the_deck() {
    for (from, to) in [
        ([-62.0, -839.0, 0.0], [-62.0, -840.5, 0.0]),  // a walk
        ([-62.0, -836.5, 0.0], [-62.0, -842.0, 0.0]),  // a roll's dash
        ([-70.0, -839.2, 0.0], [-70.0, -841.0, 1.6]),  // a jump over it
        ([-50.0, -836.0, 0.0], [-50.0, -838.0, -2.0]), // a step off, falling
    ] {
        let (mut s, h) = deck(from);
        s.events.clear();
        s.move_player(h, to, 0.0, Moving::default());
        // Stopped where they reached it, a step short of its line on the deck, not sent back.
        let (at, line) = (s.players[&h].pos, rail_y(&s, from[0]));
        assert!(at[1] > line && at[1] - line <= 0.45, "{from:?} -> {to:?}: stopped at the rail, on the deck: {at:?}");
        assert_eq!((at[0], at[2]), (from[0], from[2]));
        assert!(s.events.iter().any(|e| matches!(e, Event::Teleport { player, pos, .. } if *player == h && *pos == at)));
    }
    // Along the deck, and up to it, nothing holds them.
    let (mut s, h) = deck([-62.0, -838.5, 0.0]);
    s.move_player(h, [-60.0, -838.8, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&h].pos, [-60.0, -838.8, 0.0]);
    // Off the ship nothing is held.
    let (mut s, h) = deck([-130.0, -830.0, 0.0]);
    s.move_player(h, [-130.0, -834.0, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&h].pos, [-130.0, -834.0, 0.0]);
}

#[test]
fn a_hop_back_toward_the_edge_stops_at_the_rail() {
    let at = [-62.0, -838.0, 0.0];
    let (mut s, h) = deck(at);
    s.set_local_moves(h, false);
    // Facing in, toward the deck: the hop back goes toward the open side.
    s.move_player(h, at, std::f32::consts::FRAC_PI_2, Moving::default());
    s.events.clear();
    s.input(h, Input::Dodge);
    let yards = hop_yards(&s.events, h).expect("a hop back");
    assert!(at[1] - yards >= rail_y(&s, at[0]) - 0.05, "{yards:.2} yd: no farther than the rail");
}

#[test]
fn a_knockback_toward_the_edge_ends_on_the_deck_for_a_hero_and_a_creature() {
    let at = [-62.0, -837.5, 0.0];
    let (mut s, h) = deck(at);
    let g = s.spawn_mob(s.content.mob("dm_deck_gunner").unwrap(), [-62.0, -834.5, 0.0], None, false);
    let shove = crate::content::AbilityEffect { kind: "knockback".into(), amount: 8.0, from_caster: true, ..Default::default() };
    s.events.clear();
    s.displace(g, h, &shove);
    let yards = hop_yards(&s.events, h).expect("thrown");
    assert!(at[1] - yards >= rail_y(&s, at[0]) - 0.05, "the hero: {yards:.2} yd");
    // A gunner thrown at the side by a hero stops at it too.
    s.mobs.get_mut(&g).unwrap().pos = at;
    s.players.get_mut(&h).unwrap().pos = [-62.0, -834.5, 0.0];
    s.displace(h, g, &shove);
    for _ in 0..40 {
        s.step(DT);
    }
    assert!(s.mobs[&g].pos[1] >= rail_y(&s, at[0]) - 0.05, "the gunner: {:?}", s.mobs[&g].pos);
}

#[test]
fn a_pull_toward_the_edge_ends_on_the_deck() {
    // His harpoon hauls toward him; even a puller beyond the rail can't haul anyone over it.
    let at = [-62.0, -834.0, 0.0];
    let (mut s, h) = deck(at);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_greenskin").unwrap().0;
    s.mobs.get_mut(&boss).unwrap().pos = [-62.0, -846.0, 0.0];
    let haul = crate::content::AbilityEffect { kind: "pull".into(), amount: 20.0, from_caster: true, ..Default::default() };
    s.events.clear();
    s.displace(boss, h, &haul);
    let yards = hop_yards(&s.events, h).unwrap_or(0.0);
    assert!(at[1] - yards >= rail_y(&s, at[0]) - 0.05, "{yards:.2} yd");
    // The harpoon itself, from him on deck by the rail: the hero ends 3 yd from him, on deck.
    s.mobs.get_mut(&boss).unwrap().pos = [-62.0, -838.5, 0.0];
    s.players.get_mut(&h).unwrap().pos = [-62.0, -826.0, 0.0];
    s.events.clear();
    s.strike_fair_as(boss, h, 1, s.mobs[&boss].pos, true, false, false, Blow::Light, None, combat::Follow::Pull { yards: 9.0 }, HitKind::Projectile);
    let yards = hop_yards(&s.events, h).expect("hauled");
    assert!(-826.0 - yards >= rail_y(&s, -62.0), "{yards:.2} yd");
}

#[test]
fn the_server_never_carries_a_running_hero_over_the_rail_between_reports() {
    // Take 370: carried over it between his client's reports, his next report (behind where he
    // was carried) read as a crossing back, and he was held outside it.
    let at = [-62.0, -837.0, 0.0];
    let (mut s, h) = deck(at);
    s.move_player(h, at, -std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Moving::default() });
    for _ in 0..30 {
        s.step(DT);
        assert!(s.players[&h].pos[1] > rail_y(&s, at[0]), "carried no farther than the rail: {:?}", s.players[&h].pos);
    }
    assert!(s.players[&h].last_safe[1] > rail_y(&s, at[0]), "its firm ground stays on the deck");
    // Then his report from where his client got to, past it: stopped at the rail.
    s.move_player(h, [-62.0, -842.0, 0.0], -std::f32::consts::FRAC_PI_2, Moving { forward: true, ..Moving::default() });
    let y = s.players[&h].pos[1];
    assert!(y > rail_y(&s, at[0]) && y - rail_y(&s, at[0]) <= 0.45, "{y}");
}

#[test]
fn coming_back_aboard_over_the_rail_is_never_held() {
    let (mut s, h) = deck([-62.0, -842.0, 0.0]);
    s.events.clear();
    s.move_player(h, [-62.0, -838.0, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&h].pos, [-62.0, -838.0, 0.0]);
    assert!(!s.events.iter().any(|e| matches!(e, Event::Teleport { .. })));
}
