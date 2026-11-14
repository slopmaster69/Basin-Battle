use super::*;
use crate::land::{FlatLand, Land};

pub(super) const DT: f32 = 0.05;

pub(super) fn content() -> Content {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == "sneed");
    d.packs[0].at = [0.0, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [0.0, -20.0, 0.0];
    c
}

pub(super) fn fight(heroes: usize, seed: u64) -> (Survival, Vec<Id>, Id) {
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

pub(super) fn live(s: &Survival, key: &str) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
}

pub(super) fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

/// The machine broken: its run back to its post and its crash there, Sneed inside the wreck.
pub(super) fn broken(s: &mut Survival, machine: Id, by: Id) -> Id {
    s.hurt_mob(machine, by, u32::MAX);
    for _ in 0..400 {
        if let Some(d) = live(s, DRIVER) {
            return d;
        }
        s.step(DT);
    }
    panic!("the broken machine never crashed");
}

/// Steps until Sneed has crawled out of the wreck, got up and spoken: on his feet in the fight.
pub(super) fn emerged(s: &mut Survival, driver: Id) {
    for _ in 0..200 {
        s.step(DT);
        if s.machinery.drivers.get(&driver).is_none_or(|d| d.emerge.is_none()) {
            return;
        }
    }
    panic!("Sneed never came out of his machine");
}

pub(super) fn working(s: &mut Survival, driver: Id) -> f32 {
    for _ in 0..800 {
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: WORK, ms } if *caster == driver && *ms == (REPAIR * 1000.0) as u32)) {
            return s.clock;
        }
    }
    panic!("Sneed did not reach his wreck and start working");
}

pub(super) fn yells(ev: &[(f32, Event)]) -> Vec<(f32, String)> {
    ev.iter().filter_map(|(t, e)| match e { Event::Yell { text, .. } => Some((*t, text.clone())), _ => None }).collect()
}

pub(super) fn timed(s: &mut Survival, secs: f32) -> Vec<(f32, Event)> {
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
        assert!(ev.iter().any(|e| matches!(e, Event::React { id: who, anim: SLUMP_CLIP, .. } if *who == id)), "its legs buckle");
        let held = STALL - SLUMP_AT - SLUMP_RISE;
        assert!(ev.iter().any(|e| matches!(e, Event::Freeze { id: who, after, secs } if *who == id && *after == SLUMP_AT && (*secs - held).abs() < 1e-4)), "slumped, held still for the stall");
        assert!(s.posed(id), "no flinch stands it up");
        assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock > 3.2));
        let ev = run(&mut s, 3.0);
        assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "stall is a real punish window");
        // Smoke pours and sparks fly the whole stall, and the engine sputters.
        assert!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == id)).count() >= 10, "smoke pours");
        assert!(ev.iter().filter(|e| matches!(e, Event::Visual { id: who, kit: combat::KIT_PARRY, .. } if *who == id)).count() >= 6, "sparks fly");
        assert!(ev.iter().filter(|e| matches!(e, Event::Sound { id: who, sound: SPUTTER_SOUND } if *who == id)).count() >= 4, "the engine sputters");
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
fn shredder_encounter_lines_are_never_closer_than_two_seconds() {
    let (mut s, band, machine) = fight(1, 23);
    let mut ev = timed(&mut s, 14.0);
    s.hurt_mob(machine, band[0], u32::MAX);
    ev.extend(timed(&mut s, 40.0));
    let lines = yells(&ev);
    assert!(lines.len() >= 5, "{lines:?}");
    assert!(lines.iter().any(|(_, t)| t == "No, no, not now!") && lines.iter().any(|(_, t)| t == "That was company property!"), "{lines:?}");
    for w in lines.windows(2) {
        assert!(w[1].0 - w[0].0 >= LINE_GAP - 0.06, "{lines:?}");
    }
}

#[test]
fn sneed_keeps_his_distance_and_flurries_when_cornered_then_is_open() {
    let (mut s, band, machine) = fight(1, 29);
    let driver = broken(&mut s, machine, band[0]);
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
fn shredder_debug_damage_does_not_stop_the_repair() {
    let (mut s, band, machine) = fight(1, 18);
    let driver = broken(&mut s, machine, band[0]);
    working(&mut s, driver);
    s.hurt_mob_as(driver, band[0], 10, "debug");
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }));
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
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    let away = [8.0, 0.0, 0.0];
    s.mobs.get_mut(&driver).unwrap().pos = away;
    s.mobs.get_mut(&driver).unwrap().path.clear();
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Seeking { at: [4.6, 0.0, 0.0], until: s.clock + SEEK_LIMIT, repath_at: s.clock };
    s.land = Box::new(NoRoute);
    let mut ev = run(&mut s, SEEK_LIMIT + 0.1);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    assert!(dist2d(s.mobs[&driver].pos, away) < 3.0, "no teleport to the wreck");
    assert!(dist2d(s.mobs[&driver].pos, [4.6, 0.0, 0.0]) > 0.5);
    ev.extend(run(&mut s, LINE_GAP));
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == driver && text == LINE_NO_WAY)), "never given up without a word");
    assert!(matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_infinite()), "the wreck stays, quiet");
    assert_eq!(s.machinery.drivers[&driver].quiet, Some(machine));
    assert!(live(&s, MACHINE).is_none());
}

#[test]
fn shredder_uses_another_edge_when_the_farthest_is_blocked_and_never_falls_back_to_center() {
    let (mut s, band, machine) = fight(1, 8);
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    let foe = Foe::Hero(band[0]);
    // A post with no dry place for its fixed work point (so no log hung) falls back on the
    // wreck's edges.
    for g in s.machinery.rigs.values_mut() { g.work = None; }
    // (Off to one side, clear of every edge.)
    let wreck = s.mobs[&machine].pos;
    s.mobs.get_mut(&driver).unwrap().pos = [wreck[0], wreck[1] + 9.0, 0.0];
    s.move_player(band[0], [wreck[0] + 4.0, wreck[1], 0.0], std::f32::consts::PI, Moving::default());
    let first = s.repair_work_point(driver, machine, foe).unwrap();
    let hero = s.players[&band[0]].pos;
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
    assert!(matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_infinite()), "the wreck stays, quiet");
    assert_eq!(s.machinery.drivers[&driver].quiet, Some(machine));
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
    let driver = broken(&mut s, machine, band[0]);
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
    let driver = broken(&mut s, machine, band[0]);
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
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Spent;
    let before = s.events.len();
    s.hurt_mob(driver, band[0], u32::MAX);
    assert!(s.events[before..].iter().any(|e| matches!(e, Event::HoldPose { id, secs, .. } if *id == driver && *secs == 0.0)));
}

/// A blow into the stall leaves its slump held (no flinch), and a break in the stall lets go of
/// the held pose so the machine's fall plays.
#[test]
fn shredder_stall_holds_through_hits_and_lets_go_as_it_breaks() {
    let (mut s, band, id) = fight(1, 21);
    let h = band[0];
    let foe = Foe::Hero(h);
    let now = s.clock;
    s.mobs.get_mut(&id).unwrap().state = MobState::Recover { foe, until: now + STALL };
    s.machinery.grinds.insert(id, Grind { last: s.mobs[&id].pos, facing: 0.0, until: now, damage: 0, hit: BTreeSet::new(), fx_at: now, fx: 0, crash: false });
    let ev = run(&mut s, 0.1);
    assert!(ev.iter().any(|e| matches!(e, Event::Freeze { id: who, .. } if *who == id)));
    assert!(s.posed(id));
    let pos = s.mobs[&id].pos;
    s.move_player(h, [pos[0] + 3.0, pos[1], pos[2]], std::f32::consts::PI, Moving::default());
    let hp = s.mobs[&id].health;
    s.input(h, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(s.mobs[&id].health < hp, "the blow landed");
    assert!(!ev.iter().any(|e| matches!(e, Event::React { id: who, hold, fit, .. } if *who == id && *hold == 0.0 && *fit == 0.0)), "no flinch in the stall");
    s.events.clear();
    s.hurt_mob(id, h, u32::MAX);
    assert!(s.events.iter().any(|e| matches!(e, Event::Freeze { id: who, secs, .. } if *who == id && *secs == 0.0)), "the held slump lets go as it breaks");
}

/// Sneed's wrench reads in flight (Corey, Nov 13: a speck at 16 yd): its look draws it three
/// times its size, tumbling, with a streak behind it, flown at the throw's own speed so the look
/// lands with the blow.
#[test]
fn sneeds_wrench_is_drawn_big_and_streaked_at_the_throws_speed() {
    let c = Content::dev();
    let l = c.missile_looks.iter().find(|l| l.spell == 13398).expect("the wrench's look");
    assert!(l.scale >= 3.0 && l.tumble && !l.trail.is_empty(), "{l:?}");
    let sneed = c.mobs.iter().find(|m| m.key == "dm_sneed").expect("Sneed");
    let throw = sneed.patterns.iter().flat_map(|p| &p.steps).find(|s| s.spell == Some(13398)).expect("his throw");
    assert_eq!(throw.speed, Some(l.speed), "the look flies with the throw");
}
