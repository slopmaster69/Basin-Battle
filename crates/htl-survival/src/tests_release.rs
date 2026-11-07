//! The release check: a charged spell with `perfect = true` flares as its window opens, and a
//! release within the window of the charge's top is a perfect cast (stronger, a parry's Valor, a
//! burst everyone sees). Earlier it goes at its charge; later it's full and no more. A missed
//! check is still a working spell.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

fn dummy(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("training_dummy").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Press the key, hold the click `secs`, let go; the events meanwhile and at the release.
fn charge_and_release(s: &mut Survival, p: Id, slot: &str, secs: f32) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
    assert!(matches!(s.players[&p].action, Action::Aim { .. }), "aiming");
    s.input(p, Input::Attack);
    let mut ev = run(s, secs);
    s.input(p, Input::AttackUp);
    ev.extend(s.step(0.0));
    ev
}

/// Press the key and hold the click to this cast's flare (rolled as the charge began, and sent to
/// the caster in its `CheckStart`), plus `off` seconds, then let go.
fn release_at_flare(s: &mut Survival, p: Id, slot: &str, off: f32) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
    s.input(p, Input::Attack);
    let mut ev = s.step(0.0);
    let flare = s.players[&p].check_flare;
    let sent = ev.iter().find_map(|e| match e {
        Event::CheckStart { player, kind: "release", a, .. } if *player == p => Some(*a),
        _ => None,
    });
    assert_eq!(sent, Some(flare), "the caster is told this cast's flare as the charge begins");
    ev.extend(run(s, flare + off));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(0.0));
    ev
}

fn grade_in(events: &[Event], p: Id) -> Option<&'static str> {
    events.iter().find_map(|e| match e {
        Event::CheckResult { player, grade, .. } if *player == p => Some(*grade),
        _ => None,
    })
}

/// The blow itself (the biggest single hit: a burn's ticks come after, smaller).
fn damage_on(events: &[Event], victim: Id) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim => Some(*damage),
            _ => None,
        })
        .max()
        .unwrap_or(0)
}

fn perfect_in(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

#[test]
fn fireball_released_on_its_flare_is_a_perfect_cast_stronger_with_valor_and_a_burst() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let k_ = s.content.checks.clone();
    let fb = s.content.ability("fireball").unwrap();
    let (top, full_dmg) = (s.content.abilities[fb].charge[1], s.content.abilities[fb].damage[1]);
    let valor = s.players[&p].valor;
    let _ = top;
    // Let go on this cast's flare (rolled, from 72% of the top to the top): it's perfect.
    let ev = release_at_flare(&mut s, p, "ranged", 0.0);
    assert!(matches!(grade_in(&ev, p), Some("good" | "perfect")), "graded: {:?}", grade_in(&ev, p));
    let flare_kit = crate::content_abilities::full_kit_of("mage");
    assert!(!ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == flare_kit)), "no flare on the caster's body: the ring marks the moment");
    assert!(perfect_in(&ev, p), "a perfect");
    let burst = { let a = s.content.abilities[fb].perfect_kit; if a != 0 { a } else { k_.perfect_kit } };
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == burst)), "and its burst");
    assert_eq!(s.players[&p].valor, valor + k_.perfect_valor, "a parry's Valor");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == s.content.abilities[fb].full_spell)), "the full missile");
    let landed = run(&mut s, 1.5);
    let dmg = damage_on(&landed, k);
    assert_eq!(dmg, (full_dmg * k_.perfect_damage).round() as u32, "full and then some: {dmg}");
}

#[test]
fn a_release_outside_the_window_is_an_ordinary_cast_early_or_late() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let fb = s.content.ability("fireball").unwrap();
    let (top, full_dmg) = (s.content.abilities[fb].charge[1], s.content.abilities[fb].damage[1]);
    let valor = s.players[&p].valor;
    // Held well past the top: full, no more, no perfect.
    let ev = charge_and_release(&mut s, p, "ranged", top + 0.4);
    assert!(!perfect_in(&ev, p), "late is just full");
    let landed = run(&mut s, 1.5);
    assert_eq!(damage_on(&landed, k), full_dmg as u32);
    assert_eq!(s.players[&p].valor, valor + s.content.valor.ability, "a hit's Valor only");
    // Let go early: the charge it had.
    run(&mut s, 4.0);
    let ev = charge_and_release(&mut s, p, "ranged", 0.8);
    assert!(!perfect_in(&ev, p));
    let landed = run(&mut s, 1.5);
    let dmg = damage_on(&landed, k);
    assert!(dmg > 16 && dmg < full_dmg as u32, "between the least and the full: {dmg}");
}

#[test]
fn a_spell_without_the_check_has_no_perfect_however_well_timed() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "frostbolt").unwrap();
    let _k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let fr = s.content.ability("frostbolt").unwrap();
    assert!(!s.content.abilities[fr].perfect);
    let top = s.content.abilities[fr].charge[1];
    let ev = charge_and_release(&mut s, p, "ranged", top);
    assert!(!perfect_in(&ev, p));
}

#[test]
fn a_heavy_bolt_from_a_staff_is_held_with_no_skill_check_and_its_mark_fills_instead() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "ember_staff").unwrap();
    s.players.get_mut(&p).unwrap().pos = ORIGIN;
    let k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let hb = s.content.ability("ember_bolt_heavy").unwrap();
    let (top, full_dmg) = (s.content.abilities[hb].charge[1], s.content.abilities[hb].damage[1]);
    s.input(p, Input::Heavy);
    let ev = s.step(0.0);
    assert_eq!(s.players[&p].action.name(), "charge");
    assert!(!ev.iter().any(|e| matches!(e, Event::CheckStart { .. })), "no ring: the spells keep the skill checks");
    assert!(ev.iter().any(|e| matches!(e, Event::StaffHeavy { player, hi, .. } if *player == p && (*hi - top).abs() < 1e-3)), "the staff's mark fills");
    let held = run(&mut s, top + 0.1);
    let flare_kit = crate::content_abilities::full_kit_of("mage");
    assert!(!held.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == flare_kit)), "no flare on the body");
    s.input(p, Input::AttackUp);
    let ev = s.step(0.0);
    assert!(!perfect_in(&ev, p));
    let landed = run(&mut s, 1.5);
    assert_eq!(damage_on(&landed, k), full_dmg.round() as u32, "held to the top: the whole of it");
    // The right button calls a held bolt off, nothing spent.
    run(&mut s, 1.0);
    let st = s.players[&p].stamina;
    s.input(p, Input::Heavy);
    run(&mut s, 0.4);
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == p)), "called off");
    assert!((s.players[&p].stamina - st).abs() < 1.0, "nothing spent on a bolt called off");
}

#[test]
fn meteor_is_held_and_let_go_early_it_is_half_the_meteor() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ultimate", "meteor").unwrap();
    s.players.get_mut(&p).unwrap().valor = s.content.valor.max;
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let mt = s.content.ability("meteor").unwrap();
    let (lo, top) = (s.content.abilities[mt].charge[0], s.content.abilities[mt].charge[1]);
    let dmg = s.content.abilities[mt].damage;
    // Let go as soon as it can: the small meteor.
    let ev = charge_and_release(&mut s, p, "ultimate", lo + 0.05);
    assert!(!perfect_in(&ev, p));
    let landed = run(&mut s, 2.5);
    let took = damage_on(&landed, k);
    assert!(took >= dmg[0] as u32 && took < dmg[1] as u32, "a small meteor: {took}");
    // Again, on the flare: the whole of it and more.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ultimate", "meteor").unwrap();
    s.players.get_mut(&p).unwrap().valor = s.content.valor.max;
    let k = dummy(&mut s, [10.0, 0.0, 0.0]);
    let _ = top;
    let ev = release_at_flare(&mut s, p, "ultimate", 0.0);
    assert!(perfect_in(&ev, p));
    let landed = run(&mut s, 2.5);
    assert_eq!(damage_on(&landed, k), (dmg[1] * s.content.checks.perfect_damage).round() as u32);
}

#[test]
fn every_cast_rolls_its_own_flare_and_the_middle_of_its_window_is_a_critical() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let _k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let fb = s.content.ability("fireball").unwrap();
    let (lo, top) = (s.content.abilities[fb].charge[0], s.content.abilities[fb].charge[1]);
    let mut seen = Vec::new();
    for _ in 0..6 {
        let ev = release_at_flare(&mut s, p, "ranged", 0.0);
        let flare = ev.iter().find_map(|e| match e { Event::CheckStart { a, .. } => Some(*a), _ => None }).unwrap();
        assert!(flare > lo && flare <= top + 1.0e-4, "inside the charge: {flare}");
        assert_eq!(grade_in(&ev, p), Some("perfect"), "dead on the flare is the middle of the window");
        seen.push(flare);
        run(&mut s, 4.0);
    }
    seen.sort_by(|a, b| a.total_cmp(b));
    assert!(seen[5] - seen[0] > 0.05, "no two casts quite the same: {seen:?}");
    // Just inside the window's edge: a perfect, not a critical. Well early: weak.
    let w = s.content.checks.window;
    let ev = release_at_flare(&mut s, p, "ranged", w * 0.8);
    assert_eq!(grade_in(&ev, p), Some("good"));
    run(&mut s, 4.0);
    let ev = release_at_flare(&mut s, p, "ranged", -0.35);
    assert_eq!(grade_in(&ev, p), Some("early"));
}

#[test]
fn an_all_or_nothing_spell_fizzles_off_its_moment_and_spends_its_valor() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let _k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let fb = s.content.ability("fireball").unwrap();
    s.content.abilities[fb].stakes = "all".into();
    let valor = s.players[&p].valor;
    let ev = release_at_flare(&mut s, p, "ranged", 0.3);
    assert_eq!(grade_in(&ev, p), Some("fizzle"));
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == p)), "nothing goes");
    assert!(s.players[&p].valor <= valor, "no Valor earned");
    assert_eq!(s.players[&p].action, Action::Ready);
}

#[test]
fn a_fixed_charges_window_never_opens_before_it_can_go_so_it_never_fizzles_inside_it() {
    // The review on #199: Pyroblast's moment could roll so its window opened before its least,
    // and a release there fizzled while the ring showed it in the window.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let pyro = s.content.ability("pyroblast").unwrap();
    let lo = s.content.abilities[pyro].charge[0];
    let w = s.content.checks.window;
    for _ in 0..40 {
        s.players.get_mut(&p).unwrap().valor = s.content.valor.max;
        s.players.get_mut(&p).unwrap().cd_off = true;
        s.input(p, Input::Ability(key_of("ranged"), true));
        s.input(p, Input::Attack);
        s.step(0.0);
        assert!(matches!(s.players[&p].action, Action::Charge { .. }), "charging Pyroblast");
        let flare = s.players[&p].check_flare;
        assert!(flare - w >= lo - 1e-4, "the window opens at {} before its least {lo}", flare - w);
        s.input(p, Input::Guard);
        s.input(p, Input::GuardUp);
        run(&mut s, 0.2);
    }
}

#[test]
fn a_charge_called_off_calls_its_check_off_so_its_ring_goes() {
    // The review on #199: a called-off charge left its ring up, and a later let-go judged it.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let _k = dummy(&mut s, [14.0, 0.0, 0.0]);
    s.input(p, Input::Ability(key_of("ranged"), false));
    s.input(p, Input::Attack);
    run(&mut s, 0.4);
    s.input(p, Input::Guard);
    let ev = run(&mut s, 0.1);
    assert_eq!(grade_in(&ev, p), Some("cancel"), "called off for the caster's screen");
    s.input(p, Input::GuardUp);
    s.input(p, Input::AttackUp);
    let ev = run(&mut s, 0.5);
    assert_eq!(grade_in(&ev, p), None, "nothing judged after");
}

/// Ticks of a burn (or any damage over time) on `victim`.
fn ticks_on(events: &[Event], victim: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::Swing { victim: Some(v), style: Blow::Dot, .. } if *v == victim)).count()
}

#[test]
fn fireball_explodes_on_the_foe_it_hits_and_catches_the_one_beside_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let fb = s.content.ability("fireball").unwrap();
    let (top, full_dmg, splash) = (s.content.abilities[fb].charge[1], s.content.abilities[fb].damage[1], s.content.abilities[fb].splash);
    assert!(splash >= 2.0, "it explodes: {splash}");
    let k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let beside = dummy(&mut s, [14.0, 2.0, 0.0]);
    let far = dummy(&mut s, [14.0, 7.0, 0.0]);
    charge_and_release(&mut s, p, "ranged", top + 0.4);
    let landed = run(&mut s, 1.5);
    assert_eq!(damage_on(&landed, k), full_dmg as u32, "the one it hit");
    assert_eq!(damage_on(&landed, beside), full_dmg as u32, "the one beside it, as hard");
    assert_eq!(damage_on(&landed, far), 0, "not one well outside it");
    // Its explosion shows where it struck; no second fireball flies there from the caster.
    let (spell, look) = (s.content.abilities[fb].spell, s.content.abilities[fb].burst_spell);
    let missiles = |sp: u32| landed.iter().filter(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == sp)).count();
    assert_eq!(missiles(spell), 0, "no second fireball");
    assert!(look != 0 && missiles(look) == 1, "its burst, once");
}

#[test]
fn a_perfect_fireball_still_burns_the_one_it_hit_longer_though_it_explodes() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "fireball").unwrap();
    let fb = s.content.ability("fireball").unwrap();
    assert!(!s.content.abilities[fb].crit_effects.is_empty());
    let k = dummy(&mut s, [14.0, 0.0, 0.0]);
    let beside = dummy(&mut s, [14.0, 2.0, 0.0]);
    let ev = release_at_flare(&mut s, p, "ranged", 0.0);
    assert!(perfect_in(&ev, p));
    run(&mut s, 1.5);
    // Past the plain burn's 4 sec, only the perfect's longer one still ticks.
    run(&mut s, 4.5);
    let late = run(&mut s, 3.0);
    assert!(ticks_on(&late, k) > 0, "the one it hit still burns");
    assert_eq!(ticks_on(&late, beside), 0, "the one beside it had the plain burn");
}

#[test]
fn pyroblast_stays_one_target_and_outhits_a_perfect_fireball() {
    let s = world();
    let (fb, pyro) = (s.content.ability("fireball").unwrap(), s.content.ability("pyroblast").unwrap());
    let (fb, pyro) = (&s.content.abilities[fb], &s.content.abilities[pyro]);
    assert_eq!(pyro.splash, 0.0, "the one big ball, for one foe");
    assert!(pyro.damage[0] > fb.damage[1] * s.content.checks.perfect_damage * 1.3, "well over a perfect Fireball");
}

#[test]
fn meteor_leaves_what_it_knocks_back_inside_its_crater() {
    let s = world();
    let mt = &s.content.abilities[s.content.ability("meteor").unwrap()];
    let knock = mt.effects.iter().find(|e| e.kind == "knockback").map_or(0.0, |e| e.amount);
    // A foe a yard from the middle, thrown back, still stands in the smallest crater.
    assert!(1.0 + knock < mt.radius[0], "knocked {knock} yd out of a {} yd crater", mt.radius[0]);
}
