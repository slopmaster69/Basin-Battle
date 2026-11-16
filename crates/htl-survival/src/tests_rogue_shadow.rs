//! The Rogue's Shadow tree (docs/rogue-roles.md, build plan entry 2): Distract and its EX Sap,
//! Shadowstep to a back, Cloak and Dagger, and the trait, Shrouded Step.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::rogue::SHADOW;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const PI: f32 = std::f32::consts::PI;

fn world_with(pvp: bool, full_tree: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    c.dev.full_tree = full_tree;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(false, true)
}

/// A hero of this class, made as the creation screen makes one, at `at` facing `f`. In a dev world
/// a Rogue's keys hold its column's first choices: all five Shadow's.
fn hero_of(s: &mut Survival, name: &str, class: &str, at: Vec3, f: f32) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, at, f, Moving::default());
    let _ = s.step(0.0);
    id
}

fn rogue(s: &mut Survival) -> Id {
    hero_of(s, "Ayla", "rogue", ORIGIN, 0.0)
}

/// A creature at `at` facing `f`, at rest, that strikes nothing on its own.
fn mob(s: &mut Survival, key: &str, at: Vec3, f: f32) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, at, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = at;
    m.facing = f;
    m.ready_at = 1e9;
    m.health = 100_000;
    m.max_health = 100_000;
    id
}

fn grunt(s: &mut Survival, at: Vec3, f: f32) -> Id {
    mob(s, "grunt", at, f)
}

/// A grunt that stands as it was put (`.es target ... still`): it never turns nor walks of its own
/// will, so where a step lands can be judged against it.
fn still(s: &mut Survival, at: Vec3, f: f32) -> Id {
    let m = grunt(s, at, f);
    s.targets.insert(m, true);
    m
}

/// A boss (the Rogue's own, Morgan the Collector), as `mob` puts one.
fn boss(s: &mut Survival, at: Vec3, f: f32) -> Id {
    let m = mob(s, "morgan", at, f);
    assert!(s.content.mobs[s.mobs[&m].def].boss);
    m
}

/// In a fight with hero `p`, where it stands.
fn fighting(s: &mut Survival, m: Id, p: Id) {
    let x = s.mobs.get_mut(&m).unwrap();
    x.state = MobState::Chase { foe: Foe::Hero(p) };
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Its key pressed (`ex`: its EX version, with Valor for it), an aimed one let go on a tap of
/// the click.
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.set_valor(p, 100.0);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Ability(key_of(slot), ex));
    let mut ev = s.step(DT);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(DT));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
    }
    ev
}

/// A Distract's pouch landing at `at`, on this one (as the lob's landing gives it).
fn distract_at(s: &mut Survival, p: Id, m: Id, at: Vec3) {
    s.fx_from = Some(at);
    s.distract(p, m, 2.5, "Distract");
    s.fx_from = None;
}

/// The reticle on the ground `d` yd straight ahead of a hero at the origin facing +x.
fn aim_ahead(s: &mut Survival, p: Id, d: f32) {
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(d));
}

fn faces(s: &Survival, m: Id, at: Vec3) -> bool {
    let x = &s.mobs[&m];
    angle_between(x.facing, facing(x.pos, at)) < 2.0
}

fn winding(s: &Survival, m: Id) -> bool {
    matches!(s.mobs[&m].state, MobState::Windup { .. })
}

fn beat(ev: &[Event], p: Id, b: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == p && beat == b))
}

/// Run until the hero's blow is late in its recovery, where the next press goes.
fn to_late_recovery(s: &mut Survival, p: Id) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..80 {
        ev.extend(s.step(DT));
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) || matches!(s.players[&p].action, Action::Ready) {
            return ev;
        }
    }
    ev
}

// --- Distract --------------------------------------------------------------------------------------

#[test]
fn distract_turns_every_enemy_near_its_landing_to_face_it_for_2_5_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    // Past a grunt's notice, so none walks in: two within 6 yd of where it lands, one 9 yd off.
    let spot = [18.0, 0.0, 0.0];
    let side = grunt(&mut s, [18.0, 4.0, 0.0], PI);
    let far = grunt(&mut s, [22.0, 0.0, 0.0], PI);
    let out = grunt(&mut s, [18.0, 9.0, 0.0], PI);
    aim_ahead(&mut s, p, 18.0);
    let mut ev = press(&mut s, p, "ranged", false);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 1725, target: Target::Point(at) } if *caster == p && dist2d(*at, spot) < 1.5)), "the pouch thrown there");
    // No ring and no warning: the pouch hurts nobody.
    assert!(!ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(t), .. } if t.at.is_some())), "no ring where it lands");
    let mut t0 = None;
    for _ in 0..100 {
        ev.extend(s.step(DT));
        if s.has_cc(side, "distract") {
            t0 = Some(s.clock);
            break;
        }
    }
    assert!(t0.is_some(), "it landed and they turned");
    let at = s.mobs[&side].rogue.distract_at.unwrap();
    assert!(dist2d(at, spot) < 1.5, "turned to where it landed: {at:?}");
    assert!(faces(&s, side, at) && faces(&s, far, at), "both face it at once");
    assert!(s.has_cc(far, "distract"));
    assert!(!s.has_cc(out, "distract") && (s.mobs[&out].facing - PI).abs() < 1.0e-3, "9 yd off: untouched");
    // 2.5 sec of it.
    run(&mut s, 2.4);
    assert!(s.has_cc(side, "distract") && faces(&s, side, at), "still looking at 2.4 sec");
    run(&mut s, 0.2);
    assert!(!s.has_cc(side, "distract"), "over at 2.5 sec");
    assert!(matches!(s.mobs[&side].state, MobState::Idle { .. }), "and never drawn into a fight by it");
}

#[test]
fn a_distracted_enemy_starts_no_blow_and_one_from_its_front_ends_it() {
    let mut s = world();
    let p = rogue(&mut s);
    // In a fight with her, 2 yd in front, ready to strike: Distracted toward its own left.
    let m = grunt(&mut s, [2.0, 0.0, 0.0], PI);
    fighting(&mut s, m, p);
    s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
    distract_at(&mut s, p, m, [2.0, 6.0, 0.0]);
    assert!(faces(&s, m, [2.0, 6.0, 0.0]), "turned to the pouch");
    for _ in 0..(2.0 / DT) as usize {
        let _ = s.step(DT);
        assert!(!winding(&s, m), "no blow while distracted");
    }
    assert_eq!(s.mobs[&m].state.foe(), Some(Foe::Hero(p)), "it kept its foe");
    // She strikes it from where she stands: its side, not its back.
    assert!(!s.from_behind(m, s.players[&p].pos));
    s.input(p, Input::Attack);
    let _ = s.step(DT);
    s.input(p, Input::AttackUp);
    run(&mut s, 0.4);
    assert!(!s.has_cc(m, "distract"), "a blow from its front ends it");
    let mut wound = false;
    for _ in 0..(3.0 / DT) as usize {
        let _ = s.step(DT);
        wound |= winding(&s, m);
    }
    assert!(wound, "and it fights again");
}

#[test]
fn a_backstab_does_not_end_a_distraction() {
    let mut s = world();
    // She stands at its back: it is Distracted away from her.
    let p = hero_of(&mut s, "Ayla", "rogue", [2.0, -2.0, 0.0], PI / 2.0);
    let m = grunt(&mut s, [2.0, 0.0, 0.0], -PI / 2.0);
    fighting(&mut s, m, p);
    distract_at(&mut s, p, m, [2.0, 6.0, 0.0]);
    assert!(s.from_behind(m, s.players[&p].pos), "her spot is its back");
    let hp = s.mobs[&m].health;
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 0.4));
    assert!(s.mobs[&m].health < hp, "struck");
    assert!(beat(&ev, p, crate::sim::cues::BEAT_BACKSTAB), "a backstab");
    assert!(s.has_cc(m, "distract"), "still looking away");
}

#[test]
fn a_boss_glances_for_0_8_sec_once_every_15_sec_never_mid_string() {
    let mut s = world();
    let p = rogue(&mut s);
    let b = boss(&mut s, [3.0, 0.0, 0.0], PI);
    fighting(&mut s, b, p);
    let r = s.content.rogue.clone();
    assert_eq!((r.glance_secs, r.glance_every), (0.8, 15.0));
    // Mid-string: no glance, and none spent.
    s.mobs.get_mut(&b).unwrap().pattern = Some((0, 0));
    distract_at(&mut s, p, b, [3.0, 6.0, 0.0]);
    assert!(!s.has_cc(b, "distract"), "never mid-string");
    s.mobs.get_mut(&b).unwrap().pattern = None;
    distract_at(&mut s, p, b, [3.0, 6.0, 0.0]);
    assert!(s.has_cc(b, "distract") && faces(&s, b, [3.0, 6.0, 0.0]), "a glance");
    let ends = s.cc_of(b).iter().find(|c| c.kind == "distract").unwrap().until - s.clock;
    assert!((ends - 0.8).abs() < 1.0e-3, "for 0.8 sec, not 2.5: {ends}");
    run(&mut s, 0.85);
    assert!(!s.has_cc(b, "distract"));
    // Again within 15 sec: nothing.
    s.mobs.get_mut(&b).unwrap().pattern = None;
    distract_at(&mut s, p, b, [3.0, -6.0, 0.0]);
    assert!(!s.has_cc(b, "distract"), "once every 15 sec");
    s.clock += 15.0;
    s.mobs.get_mut(&b).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    s.mobs.get_mut(&b).unwrap().pattern = None;
    distract_at(&mut s, p, b, [3.0, -6.0, 0.0]);
    assert!(s.has_cc(b, "distract"), "after 15 sec, another");
    assert_eq!(s.mobs[&b].state.foe(), Some(Foe::Hero(p)), "and its fight is never reset");
}

#[test]
fn distract_does_nothing_to_a_hero() {
    let mut s = world_with(true, true);
    let p = rogue(&mut s);
    let h = hero_of(&mut s, "Bren", "warrior", [18.0, 2.0, 0.0], PI);
    aim_ahead(&mut s, p, 18.0);
    let mut ev = press(&mut s, p, "ranged", false);
    ev.extend(run(&mut s, 1.5));
    assert!(!s.has_cc(h, "distract"), "a hero's facing is their own");
    assert!((s.players[&h].facing - PI).abs() < 1.0e-3);
    assert!(!ev.iter().any(|e| matches!(e, Event::Danger { player, .. } if *player == h)), "and no warning for a pouch");
    // Nor when it is given straight.
    distract_at(&mut s, p, h, [18.0, 8.0, 0.0]);
    assert!(!s.has_cc(h, "distract"));
}

// --- Sap -------------------------------------------------------------------------------------------

#[test]
fn sap_from_behind_incapacitates_6_sec_and_from_the_front_is_a_cut() {
    let mut s = world();
    let p = rogue(&mut s);
    // At rest and Distracted away from her (else it notices her this close), its back to her.
    let m = grunt(&mut s, [1.8, 0.0, 0.0], 0.0);
    distract_at(&mut s, p, m, [6.0, 0.0, 0.0]);
    let hp = s.mobs[&m].health;
    press(&mut s, p, "ranged", true);
    run(&mut s, 0.4);
    assert!(s.mobs[&m].health < hp, "struck");
    assert!(s.has_cc(m, "incapacitate"), "sapped");
    let left = s.cc_of(m).iter().find(|c| c.kind == "incapacitate").unwrap().until - s.clock;
    assert!(left > 5.5 && left <= 6.0, "for 6 sec: {left}");
    run(&mut s, 3.0);
    assert!(s.has_cc(m, "incapacitate") && !winding(&s, m), "out cold");
    // Any damage ends it.
    s.hurt_mob(m, p, 1);
    assert!(!s.has_cc(m, "incapacitate"), "a blow wakes it");
    // From the front: only the cut.
    s.mobs.clear();
    s.set_cooldowns(p, true);
    let f = grunt(&mut s, [1.8, 0.0, 0.0], PI);
    let hp = s.mobs[&f].health;
    press(&mut s, p, "ranged", true);
    run(&mut s, 0.4);
    assert!(s.mobs[&f].health < hp, "the cut lands");
    assert!(!s.has_cc(f, "incapacitate"), "nothing more from the front");
}

#[test]
fn sap_never_takes_a_boss_or_a_creature_already_fighting() {
    let mut s = world();
    let p = rogue(&mut s);
    s.set_cooldowns(p, true);
    // In a fight with her, held looking away (a Distract), its back to her.
    let m = grunt(&mut s, [1.8, 0.0, 0.0], PI);
    fighting(&mut s, m, p);
    distract_at(&mut s, p, m, [6.0, 0.0, 0.0]);
    assert!(s.from_behind(m, s.players[&p].pos));
    let hp = s.mobs[&m].health;
    let mut ev = press(&mut s, p, "ranged", true);
    ev.extend(run(&mut s, 0.4));
    assert!(s.mobs[&m].health < hp && beat(&ev, p, crate::sim::cues::BEAT_BACKSTAB), "a cut in its back");
    assert!(!s.has_cc(m, "incapacitate"), "but it was already fighting");
    // A boss at rest, its back to her: never.
    s.mobs.clear();
    let b = boss(&mut s, [1.8, 0.0, 0.0], 0.0);
    press(&mut s, p, "ranged", true);
    run(&mut s, 0.4);
    assert!(!s.has_cc(b, "incapacitate"), "never a boss");
}

// --- Shadowstep ------------------------------------------------------------------------------------

#[test]
fn shadowstep_lands_behind_the_enemy_under_the_reticle_else_at_the_point() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = grunt(&mut s, [12.0, 0.0, 0.0], PI);
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(12.0));
    let ev = press(&mut s, p, "utility", false);
    let at = s.players[&p].pos;
    assert!(dist2d(at, [13.3, 0.0, 0.0]) < 0.6, "just behind it: {at:?}");
    assert!(s.from_behind(m, at), "in its back");
    assert!(angle_between(s.players[&p].facing, PI) < 5.0, "facing its back");
    let puffs = ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: 1856, .. } if *caster == p)).count();
    assert!(puffs >= 2, "a puff where she left and where she landed");
    // No enemy under the reticle: to its point, as before (the way back spent first).
    run(&mut s, 3.1);
    s.mobs.clear();
    s.set_cooldowns(p, true);
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    s.set_aim_line(p, 0.0, 0.0);
    press(&mut s, p, "utility", false);
    assert!(s.players[&p].pos[0] > 15.0, "out along the reticle: {:?}", s.players[&p].pos);
}

#[test]
fn shadowstep_still_steps_back_within_3_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    grunt(&mut s, [12.0, 0.0, 0.0], PI);
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(12.0));
    press(&mut s, p, "utility", false);
    assert!(s.players[&p].pos[0] > 12.5, "behind it");
    let keys = s.recast_keys(p);
    assert_eq!(keys.len(), 1, "the key armed for the way back");
    assert_eq!(keys[0].1, idx(&s, "shadowstep_back"));
    run(&mut s, 1.0);
    press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back where it began: {:?}", s.players[&p].pos);
}

// --- Cloak and Dagger ------------------------------------------------------------------------------

#[test]
fn cloak_and_dagger_steps_behind_the_nearest_enemy_before_each_blow_for_6_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let near = still(&mut s, [4.0, 1.0, 0.0], PI);
    let far = still(&mut s, [30.0, 0.0, 0.0], PI);
    press(&mut s, p, "ultimate", false);
    run(&mut s, 0.5);
    assert!(s.has_cc(p, "cloak"), "on");
    assert_eq!(s.players[&p].rogue.cloak_steps, 10, "10 steps");
    let hp = s.mobs[&near].health;
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    let at = s.players[&p].pos;
    assert!(s.from_behind(near, at) && dist2d(at, s.mobs[&near].pos) < 1.6, "stepped behind the nearest: {at:?}");
    ev.extend(to_late_recovery(&mut s, p));
    assert!(s.mobs[&near].health < hp && beat(&ev, p, crate::sim::cues::BEAT_BACKSTAB), "and struck its back");
    assert_eq!(s.players[&p].rogue.cloak_steps, 9);
    assert!(dist2d(s.players[&p].pos, s.mobs[&far].pos) > 8.0, "30 yd off is out of its 8 yd");
    // The steps run out.
    s.players.get_mut(&p).unwrap().rogue.cloak_steps = 0;
    run(&mut s, 0.6);
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    s.input(p, Input::Attack);
    let _ = s.step(DT);
    s.input(p, Input::AttackUp);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "no steps left: a plain blow");
    // And so does the time.
    s.players.get_mut(&p).unwrap().rogue.cloak_steps = 5;
    run(&mut s, 6.0);
    assert!(!s.has_cc(p, "cloak"), "6 sec");
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    s.input(p, Input::Attack);
    let _ = s.step(DT);
    s.input(p, Input::AttackUp);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "over: a plain blow");
}

#[test]
fn cloak_and_dagger_prefers_an_enemy_not_yet_struck() {
    let mut s = world();
    let p = rogue(&mut s);
    let a = still(&mut s, [3.0, 0.0, 0.0], PI);
    let b = still(&mut s, [3.0, 5.0, 0.0], PI);
    press(&mut s, p, "ultimate", false);
    run(&mut s, 0.5);
    let step = |s: &mut Survival| {
        s.input(p, Input::Attack);
        let _ = s.step(DT);
        s.input(p, Input::AttackUp);
        let at = s.players[&p].pos;
        to_late_recovery(s, p);
        at
    };
    let first = step(&mut s);
    assert!(dist2d(first, s.mobs[&a].pos) < 1.6, "the nearest first");
    // Right behind the first now, the other is farther: still the one not yet struck.
    let second = step(&mut s);
    assert!(dist2d(second, s.mobs[&b].pos) < 1.6 && s.from_behind(b, second), "the one not yet struck: {second:?}");
    // Both struck: the nearest again.
    let third = step(&mut s);
    assert!(dist2d(third, s.mobs[&b].pos) < 1.6 || dist2d(third, s.mobs[&a].pos) < 1.6, "the nearest of them: {third:?}");
    assert_eq!(s.players[&p].rogue.cloak_steps, 7);
}

// --- Shrouded Step and the trait -------------------------------------------------------------------

#[test]
fn shrouded_step_hides_the_rogue_1_sec_after_a_step_behind_and_a_boss_sees_through_it() {
    let mut s = world();
    let p = rogue(&mut s);
    assert!(s.rogue_trait(&s.players[&p], SHADOW), "a pure Shadow Rogue");
    let m = grunt(&mut s, [12.0, 0.0, 0.0], PI);
    fighting(&mut s, m, p);
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(12.0));
    let ev = press(&mut s, p, "utility", false);
    assert!(s.shrouded(p), "shrouded as she lands");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == p)));
    assert!(s.auras_of(p).contains(&s.content.rogue.shroud_aura), "the Stealth look on her");
    // It means to strike her at once, but starts no blow for the second.
    s.mobs.get_mut(&m).unwrap().ready_at = 0.0;
    let until = s.players[&p].rogue.shroud_until;
    assert!((until - s.clock - 1.0).abs() < 0.05, "1 sec");
    while s.clock < until - DT {
        let _ = s.step(DT);
        assert!(!winding(&s, m), "no blow at her while shrouded");
    }
    assert_eq!(s.mobs[&m].state.foe(), Some(Foe::Hero(p)), "nor did it lose its fight");
    let mut wound = false;
    for _ in 0..(3.0 / DT) as usize {
        let _ = s.step(DT);
        wound |= winding(&s, m);
    }
    assert!(!s.shrouded(p) && wound, "after it, the blow comes");
    assert!(!s.auras_of(p).contains(&s.content.rogue.shroud_aura), "the look gone");
    // A boss sees through it: a creature in a fight with her stands, a boss keeps on.
    let b = boss(&mut s, [3.0, 3.0, 0.0], PI);
    fighting(&mut s, b, p);
    fighting(&mut s, m, p);
    s.players.get_mut(&p).unwrap().rogue.shroud_until = s.clock + 1.0;
    assert!(s.step_shadowed(m, DT), "a creature stands");
    assert!(!s.step_shadowed(b, DT), "a boss keeps on");
}

#[test]
fn the_shadow_trait_is_on_with_3_shadow_cards_on_the_keys_and_off_with_2() {
    let mut s = world();
    let p = rogue(&mut s);
    let keys: Vec<String> = s.players[&p].loadout.values().cloned().collect();
    assert_eq!(keys.len(), 5);
    assert!(s.rogue_trait(&s.players[&p], SHADOW), "5 of 5");
    s.slot_ability(p, "ranged", "fan_of_knives").unwrap();
    s.slot_ability(p, "ultimate", "death_mark").unwrap();
    assert!(s.rogue_trait(&s.players[&p], SHADOW), "3 of 5");
    assert!(!s.rogue_trait(&s.players[&p], 2), "Venom's 2 is no trait");
    s.slot_ability(p, "utility", "blink").unwrap();
    assert!(!s.rogue_trait(&s.players[&p], SHADOW), "2 of 5");
    // Off, a step behind hides nobody.
    s.shrouded_step(p);
    assert!(!s.shrouded(p));
}

#[test]
fn a_non_rogue_with_3_shadow_cards_has_no_trait() {
    let mut s = world();
    let w = hero_of(&mut s, "Bren", "warrior", ORIGIN, 0.0);
    for (slot, k) in [("ranged", "distract"), ("close", "backstab"), ("utility", "shadowstep"), ("defense", "smoke_bomb"), ("ultimate", "cloak_and_dagger")] {
        s.slot_ability(w, slot, k).unwrap();
    }
    assert!(!s.rogue_trait(&s.players[&w], SHADOW), "the trait is the Rogue class's");
    let m = grunt(&mut s, [12.0, 0.0, 0.0], PI);
    s.set_aim_line(w, 0.0, -(1.9f32).atan2(12.0));
    press(&mut s, w, "utility", false);
    // (Its back was to where she landed; seeing her there, it turns, as nothing hides her.)
    let at = s.players[&w].pos;
    assert!(dist2d(at, [13.3, 0.0, 0.0]) < 0.6, "the card still steps behind: {at:?}");
    assert!(!s.shrouded(w), "but no shroud");
}

// --- the board and the words -----------------------------------------------------------------------

#[test]
fn shadow_is_the_rogues_first_choice_on_every_key_and_venoms_cards_the_second() {
    let s = world();
    let card = |k: &str| &s.content.abilities[idx(&s, k)];
    for (slot, k) in [("ranged", "distract"), ("close", "backstab"), ("utility", "shadowstep"), ("defense", "smoke_bomb"), ("ultimate", "cloak_and_dagger")] {
        let d = card(k);
        assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice), ("rogue", slot, SHADOW), "{k}");
        assert!(d.on_board() && !s.learned_where(k).is_empty(), "{k} is on the board and learned somewhere");
    }
    for k in ["fan_of_knives", "death_mark"] {
        assert_eq!(card(k).choice, 2, "{k} is Venom's");
        assert!(!s.learned_where(k).is_empty(), "{k} is still learned somewhere");
    }
    assert_eq!(card("sap").ex_of, "distract");
    // A new Rogue (no dev tree) starts with Distract and Backstab on Q and E, and still knows Fan of
    // Knives; the Dead Drop teaches all three.
    let mut s = world_with(false, false);
    let p = rogue(&mut s);
    let h = &s.players[&p];
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("distract"));
    assert_eq!(h.loadout.get("close").map(String::as_str), Some("backstab"));
    assert!(h.owned.contains("fan_of_knives"));
    let shrine = s.content.chests.iter().find(|c| c.key == "shrine_rogue").unwrap();
    // (And Venom's Eviscerate, the tree's E: entry 3; Blades' Q and E: entry 4.)
    assert_eq!(shrine.teaches, ["distract", "backstab", "fan_of_knives", "eviscerate", "darting_knife", "blade_flurry"]);
}

#[test]
fn distract_and_sap_and_cloak_and_daggers_words_say_what_the_data_does() {
    let s = world();
    let r = &s.content.rogue;
    let share = s.content.cc.player_share;
    let d = &s.content.abilities[idx(&s, "distract")];
    let e = &d.effects[0];
    assert_eq!(e.kind, "distract");
    for words in [format!("up to {} yd", d.range[1]), format!("within {} yd of where it lands", d.splash), format!("for {} sec", e.secs[1]), format!("for {} sec, at most once every {} sec", r.glance_secs, r.glance_every)] {
        assert!(d.text.contains(&words), "{words:?} in {}", d.text);
    }
    assert!(d.lob && d.damage[1] == 0.0 && d.cooldown == 10.0);
    let d = &s.content.abilities[idx(&s, "sap")];
    let e = &d.behind_effects[0];
    assert_eq!((e.kind.as_str(), d.valor, d.calm_only), ("incapacitate", 25, true));
    for words in [format!("{} sec windup", d.windup), format!("for {}% weapon damage", (d.weapon * 100.0).round()), format!("for {} sec ({} sec against heroes)", e.secs[1], e.hero_secs)] {
        assert!(d.text.contains(&words), "{words:?} in {}", d.text);
    }
    assert!(e.hero_secs <= 1.5 && e.hero_secs != e.secs[1] * share, "a hero's 1.5 sec is its own number");
    let d = &s.content.abilities[idx(&s, "cloak_and_dagger")];
    let e = &d.effects[0];
    for words in [format!("For {} sec", e.secs[1]), format!("within {} yd", r.cloak_reach), format!("Up to {} steps", e.amount)] {
        assert!(d.text.contains(&words), "{words:?} in {}", d.text);
    }
    assert_eq!((d.valor, d.cooldown), (100, 40.0));
    let d = &s.content.abilities[idx(&s, "shadowstep")];
    assert!(d.behind_foe && d.text.contains("behind the enemy under the reticle"), "{}", d.text);
    assert!(s.content.abilities[idx(&s, "blinding_step")].behind_foe);
}
