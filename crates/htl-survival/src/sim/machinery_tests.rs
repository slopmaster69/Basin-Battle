use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

fn content() -> Content {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == "sneed");
    d.packs[0].at = [0.0, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [0.0, -20.0, 0.0];
    c
}

fn fight(heroes: usize, seed: u64) -> (Survival, Vec<Id>, Id) {
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), seed, 1_000_000.0);
    let machine = live(&s, MACHINE).unwrap();
    let mut band = Vec::new();
    for n in 0..heroes {
        let h = s.join(&format!("hero{n}"), &format!("Hero{n}"), None);
        s.move_player(h, [4.0, n as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
        let p = s.players.get_mut(&h).unwrap();
        p.max_health = 100_000;
        p.health = 100_000;
        band.push(h);
    }
    s.engage(machine, Foe::Hero(band[0]));
    (s, band, machine)
}

fn live(s: &Survival, key: &str) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

/// Steps until Sneed has been thrown out, landed and spoken: on his feet in the fight.
fn emerged(s: &mut Survival, driver: Id) {
    for _ in 0..200 {
        s.step(DT);
        if s.machinery.drivers.get(&driver).is_none_or(|d| d.emerge.is_none()) {
            return;
        }
    }
    panic!("Sneed never came out of his machine");
}

fn working(s: &mut Survival, driver: Id) -> f32 {
    for _ in 0..800 {
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: WORK, ms: 4000 } if *caster == driver)) {
            return s.clock;
        }
    }
    panic!("Sneed did not reach his wreck and start working");
}

fn yells(ev: &[(f32, Event)]) -> Vec<(f32, String)> {
    ev.iter().filter_map(|(t, e)| match e { Event::Yell { text, .. } => Some((*t, text.clone())), _ => None }).collect()
}

fn timed(s: &mut Survival, secs: f32) -> Vec<(f32, Event)> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        let ev = s.step(DT);
        let t = s.clock;
        out.extend(ev.into_iter().map(|e| (t, e)));
    }
    out
}

#[test]
fn shredder_executes_pressure_cycle_at_full_and_low_health_then_grinds_a_fixed_safe_lane() {
    for (heroes, seed, low) in [(1, 2, false), (2, 7, false), (2, 19, true)] {
        let (mut s, band, id) = fight(heroes, seed);
        if low { s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health / 4; }
        let mut moves = Vec::new();
        let mut origin = None;
        for _ in 0..800 {
            let ev = s.step(DT);
            if ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id) || matches!(e, Event::React { id: who, anim: RUN_CLIP, .. } if *who == id)) {
                let m = &s.mobs[&id];
                let (k, step) = m.pattern.unwrap();
                let name = s.content.mobs[m.def].patterns[k].name.clone();
                if step == 0 { moves.push(name.clone()); }
                if name == "grind" {
                    assert_eq!(s.machinery.pressure[&id], 2);
                    // Its own rev: its Run held in place for the whole wind-up, not a swing.
                    assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)));
                    assert!(ev.iter().any(|e| matches!(e, Event::React { id: who, anim: RUN_CLIP, hold, .. } if *who == id && (*hold - 1.4).abs() < 0.01)));
                    origin = Some((m.pos, m.facing));
                    break;
                }
            }
        }
        assert_eq!(moves, ["saws", "stamp and rake", "grind"]);
        let (at, dir) = origin.expect("the actual AI reached its grind");
        let point = |ahead: f32, side: f32| [at[0] + dir.cos() * ahead - dir.sin() * side, at[1] + dir.sin() * ahead + dir.cos() * side, at[2]];
        // The hero changes sides as the rev begins; the aim must not follow them.
        s.move_player(band[0], point(3.0, 4.0), 0.0, Moving::default());
        if heroes == 2 { s.move_player(band[1], point(4.0, 0.0), 0.0, Moving::default()); }
        let before = s.players[&band[0]].health;
        let ev = run(&mut s, 2.4);
        assert_eq!(s.players[&band[0]].health, before, "a basic sidestep is a safe answer");
        assert!(dist2d(s.mobs[&id].pos, point(DISTANCE, 0.0)) < 0.2, "a straight six-yard path");
        if heroes == 2 {
            assert_eq!(ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, anim: false, .. } if *attacker == id && *v == band[1] && *damage > 0)).count(), 1, "one hit as the saw passes, no repeat tick");
        }
        assert!(ev.iter().filter(|e| matches!(e, Event::Visual { id: who, .. } if *who == id)).count() >= 3, "chips and sparks fly as it drives");
        assert_eq!(s.machinery.pressure[&id], 0);
        assert!(ev.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text.contains("heap of junk"))));
        assert!(ev.iter().any(|e| matches!(e, Event::React { id: who, anim: SLUMP_CLIP, hold, .. } if *who == id && *hold >= STALL)), "slumped, held for the stall");
        assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock > 3.2));
        let ev = run(&mut s, 3.0);
        assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "stall is a real punish window");
        assert!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == id)).count() >= 2, "it smokes as it stalls");
        assert!(!s.mobs[&id].enraged && s.mobs[&id].phase == 0);
    }
}

#[test]
fn shredder_moves_each_have_their_own_look_and_no_body_glow() {
    let (mut s, _, id) = fight(1, 3);
    assert!(s.tells_by_body(id));
    let mut seen = Vec::new();
    for _ in 0..600 {
        let ev = s.step(DT);
        for e in &ev {
            match e {
                Event::MobWindup { id: who, clip, take, .. } if *who == id => seen.push((*clip, *take)),
                Event::Danger { from: Some(m), .. } if *m == id => {
                    // (The server drops the body's glow for it; the hero is still warned.)
                }
                _ => {}
            }
        }
        if s.machinery.grinds.contains_key(&id) { break; }
    }
    // Saw (take 0), the stamp (its special), the rake (take 1); the grind sends no swing at all.
    assert_eq!(seen, [(16, 0), (130, 0), (16, 1)]);
}

#[test]
fn shredder_stamp_lands_in_dust_and_steam_pours_at_full_heat() {
    let (mut s, _, id) = fight(1, 5);
    let mut stamp_dust = false;
    for _ in 0..400 {
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Visual { id: who, kit, .. } if *who == id && *kit == combat::KIT_DUST)) && s.pattern_name(id) == Some("stamp and rake") {
            stamp_dust = true;
        }
        if s.machinery.pressure.get(&id) == Some(&2) { break; }
    }
    assert!(stamp_dust, "the stamp lands in dust");
    let ev = run(&mut s, 1.2);
    assert!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == id)).count() >= 2, "steam pours at full heat");
}

#[test]
fn shredder_works_its_log_then_turns_and_walks_in_to_stop_outside_its_legs() {
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), 9, 1_000_000.0);
    let id = live(&s, MACHINE).unwrap();
    let h = s.join("hero0", "Hero0", None);
    // Out of its sight first: it works.
    s.move_player(h, [0.0, -40.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 7.0);
    assert!(ev.iter().filter(|e| matches!(e, Event::Clip { id: who, clip: 16, .. } if *who == id)).count() >= 2, "it cuts at its log");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id: who, kit, .. } if *who == id && *kit == combat::KIT_PARRY)));
    assert!(dist2d(s.mobs[&id].pos, s.mobs[&id].home) < 0.1, "it stays at its post");
    s.move_player(h, [0.0, -14.0, 0.0], 0.0, Moving::default());
    let ev = timed(&mut s, 12.0);
    let pulled = ev.iter().find(|(_, e)| matches!(e, Event::Yell { text, .. } if text == "Clear the floor!")).map(|(t, _)| *t).expect("its pull line");
    let walk = ev.iter().find_map(|(t, e)| match e { Event::MobMove { id: who, speed, .. } if *who == id => Some((*t, *speed)), _ => None }).expect("it walks in");
    assert!(walk.0 - pulled >= TURN - 0.06, "it turns before it walks");
    assert!((walk.1 - WALK_IN).abs() < 0.05, "a walk, not a run");
    let first = ev.iter().find(|(_, e)| matches!(e, Event::MobWindup { id: who, .. } if *who == id)).map(|(t, _)| *t).expect("it reaches its hero");
    assert!(first - pulled > 3.0);
    let d = dist2d(s.mobs[&id].pos, s.players[&h].pos);
    let body = s.content.mobs[s.mobs[&id].def].body();
    assert!(body > 2.0, "its legs stand wide");
    assert!(d >= body + 1.0, "the hero stands outside its legs: {d} vs {body}");
    // And from there a hero's blow reaches it.
    let face = facing(s.players[&h].pos, s.mobs[&id].pos);
    s.move_player(h, s.players[&h].pos, face, Moving::default());
    assert!(s.foes_in_reach(h) > 0);
}

#[test]
fn shredder_debug_damage_builds_no_posture() {
    let (mut s, band, id) = fight(1, 11);
    s.hurt_mob_as(id, band[0], 200, "debug");
    assert_eq!(s.posture_of(id), 0);
    s.hurt_mob_as(id, band[0], 50, "Melee");
    assert!(s.posture_of(id) > 0);
}

#[test]
fn shredder_break_throws_sneed_out_the_far_side_and_his_lines_wait_their_turn() {
    for seed in [1, 2] {
        let (mut s, band, machine) = fight(1, seed);
        run(&mut s, 2.0);
        let wreck_at = s.mobs[&machine].pos;
        let start = s.clock;
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        assert!(s.mobs[&driver].hidden, "inside the machine as it collapses");
        s.hurt_mob(driver, band[0], 50);
        assert_eq!(s.mobs[&driver].health, s.mobs[&driver].max_health, "nothing reaches him in it");
        let mut ev = timed(&mut s, COLLAPSE + THROWN + 0.2);
        let out = ev.iter().find(|(_, e)| matches!(e, Event::MobSpawned { id } if *id == driver)).map(|(t, _)| *t).expect("thrown out");
        assert!(out - start >= COLLAPSE - 0.06, "once the machine has landed");
        let hero = s.players[&band[0]].pos;
        let landed = s.mobs[&driver].pos;
        ev.extend(timed(&mut s, 6.0));
        assert!(dist2d(landed, hero) > dist2d(wreck_at, hero) + 2.0, "out the far side");
        assert!(dist2d(landed, wreck_at) > 5.0, "clear of the wreck");
        let lines = yells(&ev);
        let words: Vec<&str> = lines.iter().map(|(_, t)| t.as_str()).collect();
        assert!(words.contains(&"That was company property!"));
        let said = lines.iter().find(|(_, t)| t == "That was company property!").unwrap().0;
        assert!(said - out >= THROWN + TALK_AFTER - 0.06);
        let first = ev.iter().find(|(_, e)| matches!(e, Event::MobWindup { id, .. } | Event::Cast { caster: id, .. } if *id == driver)).map(|(t, _)| *t);
        if let Some(t) = first {
            assert!(t - (out + THROWN) >= FIRST_BLOW - 0.06, "2 sec before his first blow");
        }
        assert!(ev.iter().any(|(_, e)| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)), "the wreck smokes");
    }
}

#[test]
fn shredder_encounter_lines_are_never_closer_than_two_seconds() {
    let (mut s, band, machine) = fight(1, 23);
    let mut ev = timed(&mut s, 14.0);
    s.hurt_mob(machine, band[0], u32::MAX);
    ev.extend(timed(&mut s, 30.0));
    let lines = yells(&ev);
    assert!(lines.len() >= 4, "{lines:?}");
    for w in lines.windows(2) {
        assert!(w[1].0 - w[0].0 >= LINE_GAP - 0.06, "{lines:?}");
    }
}

#[test]
fn sneed_keeps_his_distance_and_flurries_when_cornered_then_is_open() {
    let (mut s, band, machine) = fight(1, 29);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    emerged(&mut s, driver);
    // Put off his repair: this is his footwork.
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Spent;
    let at = s.mobs[&driver].pos;
    s.move_player(band[0], [at[0] + 5.0, at[1], at[2]], std::f32::consts::PI, Moving::default());
    let ev = run(&mut s, 0.6);
    assert!(ev.iter().any(|e| matches!(e, Event::MobMove { id, face: Some(_), .. } if *id == driver)), "he backs off, facing his foe");
    assert!(dist2d(s.mobs[&driver].pos, s.players[&band[0]].pos) > 5.0);
    // Caught: he flurries, then holds his open pose.
    let mut open = false;
    let mut flurry = false;
    for _ in 0..300 {
        let p = s.mobs[&driver].pos;
        s.move_player(band[0], [p[0] + 1.6, p[1], p[2]], std::f32::consts::PI, Moving::default());
        let ev = s.step(DT);
        flurry |= s.pattern_name(driver) == Some("flurry");
        if ev.iter().any(|e| matches!(e, Event::HoldPose { id, clip: OPEN_CLIP, secs, .. } if *id == driver && *secs >= 1.4)) {
            open = true;
            break;
        }
    }
    assert!(flurry && open, "cornered: three blows, then open");
}

#[test]
fn shredder_repair_runs_to_the_far_edge_takes_four_seconds_preserves_driver_health_and_cannot_repeat() {
    for heroes in [1, 2] {
        let (mut s, band, machine) = fight(heroes, 13);
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        emerged(&mut s, driver);
        assert!(!s.slain("sneed"));
        s.hurt_mob(driver, band[0], 20);
        let driver_hp = s.mobs[&driver].health;
        let hero = s.players[&band[0]].pos;
        let wreck = s.mobs[&machine].pos;
        let began = working(&mut s, driver);
        assert!(dist2d(s.mobs[&driver].pos, hero) > dist2d(wreck, hero), "the wreck's far edge from his foe");
        assert!(s.mobs.contains_key(&machine), "wreck stays for the repair");
        run(&mut s, 3.85);
        assert!(live(&s, MACHINE).is_none(), "no early rebuild");
        assert!(!s.slain("sneed"));
        let ev = run(&mut s, 0.25);
        let back = live(&s, MACHINE).expect("repair succeeds");
        assert!(s.clock - began >= REPAIR);
        assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health / 3);
        assert!(dist2d(s.mobs[&back].pos, wreck) < 0.1, "where the wreck lay");
        assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim: HEAVE_CLIP, .. } if *id == back)), "it heaves up");
        assert!(!ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Clear the floor!")), "not a fresh machine's pull");
        assert!(matches!(s.mobs[&back].state, MobState::Recover { until, .. } if until - s.clock >= REBUILT_WAIT - 0.3));
        assert!(!s.mobs.contains_key(&driver) && !s.mobs.contains_key(&machine));
        assert!(!s.slain("sneed"));
        s.hurt_mob(back, band[0], u32::MAX);
        let final_driver = live(&s, DRIVER).unwrap();
        assert_eq!(s.mobs[&final_driver].health, driver_hp);
        let ev = run(&mut s, 20.0);
        assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == final_driver)), "he bails out the same way");
        assert!(live(&s, MACHINE).is_none());
        s.hurt_mob(final_driver, band[0], u32::MAX);
        assert!(s.slain("sneed"));
        assert_eq!(s.camp.counts.get("kill:dm_sneed"), Some(&1));
        run(&mut s, 7.0);
        assert!(!s.mobs.values().any(|m| matches!(s.content.mobs[m.def].key.as_str(), MACHINE | DRIVER)), "no ghost body");
        assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty());
    }
}

#[test]
fn shredder_repair_is_interrupted_by_an_ordinary_melee_swing_without_a_loadout() {
    let (mut s, band, machine) = fight(1, 17);
    let h = band[0];
    s.players.get_mut(&h).unwrap().loadout.clear();
    s.hurt_mob(machine, h, u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    working(&mut s, driver);
    let pos = s.mobs[&driver].pos;
    s.move_player(h, [pos[0] + 1.2, pos[1], pos[2]], std::f32::consts::PI, Moving::default());
    let hp = s.mobs[&driver].health;
    s.input(h, Input::Attack);
    let ev = run(&mut s, 1.4);
    assert!(s.mobs[&driver].health < hp, "the ordinary swing landed");
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    assert!(s.machinery.drivers[&driver].wreck.is_none());
    // The wreck goes quiet: no more smoke, a body for a while.
    let quiet = ev.iter().rposition(|e| matches!(e, Event::CastEnd { caster } if *caster == driver)).unwrap();
    assert!(!ev[quiet..].iter().any(|e| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)));
    let ev = run(&mut s, 18.0);
    assert!(!s.mobs.contains_key(&machine));
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
    assert!(live(&s, MACHINE).is_none() && !s.slain("sneed"));
}

#[test]
fn shredder_debug_damage_does_not_stop_the_repair() {
    let (mut s, band, machine) = fight(1, 18);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    working(&mut s, driver);
    s.hurt_mob_as(driver, band[0], 10, "debug");
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }));
}

#[test]
fn shredder_repair_stagger_aborts_and_wipe_restores_exactly_one_whole_machine() {
    let (mut s, band, machine) = fight(1, 6);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    working(&mut s, driver);
    s.stagger_mob(driver, 1.0, false);
    s.step(DT);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    s.hurt_player_by(band[0], u32::MAX, driver, "test wipe");
    s.step(DT);
    assert!(live(&s, DRIVER).is_none());
    let back = live(&s, MACHINE).unwrap();
    assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health);
    assert_eq!(s.mobs.values().filter(|m| s.content.mobs[m.def].key == MACHINE).count(), 1);
    assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty());
    assert!(!s.slain("sneed"));
}

#[test]
fn shredder_wipe_while_sneed_is_still_inside_restores_one_machine() {
    let (mut s, band, machine) = fight(1, 7);
    s.hurt_mob(machine, band[0], u32::MAX);
    s.step(DT);
    s.hurt_player_by(band[0], u32::MAX, machine, "test wipe");
    run(&mut s, 0.2);
    assert!(live(&s, DRIVER).is_none());
    assert_eq!(s.mobs.values().filter(|m| s.content.mobs[m.def].key == MACHINE).count(), 1);
    assert!(live(&s, MACHINE).is_some() && !s.slain("sneed"));
}

struct NoRoute;
impl Land for NoRoute {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn plans(&self) -> bool { true }
    fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
}

#[test]
fn shredder_unreachable_wreck_has_a_bounded_attempt_and_no_teleport() {
    let (mut s, band, machine) = fight(1, 8);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    emerged(&mut s, driver);
    let away = [8.0, 0.0, 0.0];
    s.mobs.get_mut(&driver).unwrap().pos = away;
    s.mobs.get_mut(&driver).unwrap().path.clear();
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Seeking { at: [4.6, 0.0, 0.0], until: s.clock + SEEK_LIMIT, repath_at: s.clock };
    s.land = Box::new(NoRoute);
    run(&mut s, SEEK_LIMIT + 0.1);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    assert!(dist2d(s.mobs[&driver].pos, away) < 0.1);
    assert!(s.mobs.get(&machine).is_none_or(|m| matches!(m.state, MobState::Dead { gone_at } if gone_at.is_finite())));
    assert!(live(&s, MACHINE).is_none());
}

#[test]
fn shredder_uses_another_edge_when_the_farthest_is_blocked_and_never_falls_back_to_center() {
    let (mut s, band, machine) = fight(1, 8);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    emerged(&mut s, driver);
    let foe = Foe::Hero(band[0]);
    // (Off to one side, clear of every edge.)
    s.mobs.get_mut(&driver).unwrap().pos = [0.0, 9.0, 0.0];
    let first = s.repair_work_point(driver, machine, foe).unwrap();
    let hero = s.players[&band[0]].pos;
    let wreck = s.mobs[&machine].pos;
    assert!(dist2d(first, hero) > dist2d(wreck, hero), "the far edge first");
    s.land = Box::new(FlatLand { walls: vec![([first[0], first[1]], 0.8)], ..Default::default() });
    let other = s.repair_work_point(driver, machine, foe).expect("another reachable edge");
    assert!(dist2d(first, other) > 4.0);
    assert!(s.land.footing(other).is_some());
    assert!(dist2d(other, s.mobs[&machine].pos) > 4.0);
    s.land = Box::new(NoRoute);
    assert!(s.repair_work_point(driver, machine, foe).is_none());
    s.mobs.get_mut(&driver).unwrap().state = MobState::Chase { foe };
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Fighting { after: s.clock };
    let before = s.mobs[&driver].pos;
    assert!(s.step_machinery(driver));
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    assert_eq!(s.mobs[&driver].pos, before);
    assert!(matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_finite()), "the wreck is a body now");
}

#[test]
fn shredder_stalls_at_a_wall_and_a_posture_break_cannot_shorten_the_opening() {
    let (mut s, band, id) = fight(1, 22);
    s.land = Box::new(FlatLand { walls: vec![([4.0, 0.0], 0.7)], ..Default::default() });
    let k = s.content.mobs[s.mobs[&id].def].patterns.iter().position(|p| p.name == "grind").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(band[0]), [8.0, 0.0, 0.0]);
    s.move_player(band[0], [4.8, 0.0, 0.0], 0.0, Moving::default());
    let hp = s.players[&band[0]].health;
    run(&mut s, 2.4);
    assert!(s.mobs[&id].pos[0] <= 3.3, "saw stops before the wall");
    assert_eq!(s.players[&band[0]].health, hp, "no hit through the wall");
    let until = s.machinery.stalls[&id].until;
    s.stagger_mob(id, 0.6, false);
    run(&mut s, 0.8);
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until: end, .. } if end >= until));
}

#[test]
fn shredder_zero_damage_does_not_cancel_work_but_a_departed_party_resets_it() {
    let (mut s, band, machine) = fight(1, 21);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    working(&mut s, driver);
    s.hurt_mob(driver, band[0], 0);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }));
    s.move_player(band[0], [500.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.1);
    assert!(!s.mobs.contains_key(&driver) && !s.mobs.contains_key(&machine));
    let back = live(&s, MACHINE).unwrap();
    assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health);
    assert!(s.machinery.drivers.is_empty() && !s.slain("sneed"));
}

#[test]
fn shredder_driver_commits_his_wrench_before_retreating() {
    let (mut s, band, machine) = fight(1, 31);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    emerged(&mut s, driver);
    let p = s.mobs[&driver].pos;
    let stand = [p[0] + 8.0, p[1], p[2]];
    s.move_player(band[0], stand, 0.0, Moving::default());
    let mut wound = false;
    for _ in 0..200 {
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: 13398, .. } if *caster == driver)) {
            wound = true;
            break;
        }
    }
    assert!(wound, "the AI chose its throw at range");
    let at = s.mobs[&driver].pos;
    let aim = s.players[&band[0]].pos;
    s.move_player(band[0], [aim[0], aim[1] + 3.0, aim[2]], 0.0, Moving::default());
    let hp = s.players[&band[0]].health;
    let ev = run(&mut s, 2.0);
    assert_eq!(s.players[&band[0]].health, hp, "a sidestep during the tell evades the wrench");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 13398, target: Target::Point(p) } if *caster == driver && dist2d(*p, aim) < 0.1)));
    assert!(dist2d(s.mobs[&driver].pos, at) > 3.0, "the throw covers his retreat");
    // And the next is 6 sec off.
    assert!(s.machinery.drivers[&driver].wrench_at - s.clock > WRENCH_EVERY - 2.5);
}

#[test]
fn shredder_death_lets_go_of_any_held_pose() {
    let (mut s, band, machine) = fight(1, 12);
    s.hurt_mob(machine, band[0], u32::MAX);
    let driver = live(&s, DRIVER).unwrap();
    emerged(&mut s, driver);
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Spent;
    let before = s.events.len();
    s.hurt_mob(driver, band[0], u32::MAX);
    assert!(s.events[before..].iter().any(|e| matches!(e, Event::HoldPose { id, secs, .. } if *id == driver && *secs == 0.0)));
}
