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
    // Let go right at the top of the charge: the flare came as the window opened, and it's perfect.
    let ev = charge_and_release(&mut s, p, "ranged", top);
    let flare_kit = crate::content_abilities::full_kit_of("mage");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == flare_kit)), "the flare on the caster");
    assert!(perfect_in(&ev, p), "a perfect");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == k_.perfect_kit)), "and its burst");
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
fn a_heavy_bolt_from_a_staff_is_held_and_let_go_on_the_flare_for_a_perfect() {
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
    assert_eq!(s.players[&p].action.name(), "charge");
    run(&mut s, top);
    s.input(p, Input::AttackUp);
    let ev = s.step(0.0);
    assert!(perfect_in(&ev, p), "on the flare");
    let landed = run(&mut s, 1.5);
    assert_eq!(damage_on(&landed, k), (full_dmg * s.content.checks.perfect_damage).round() as u32);
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
    let ev = charge_and_release(&mut s, p, "ultimate", top);
    assert!(perfect_in(&ev, p));
    let landed = run(&mut s, 2.5);
    assert_eq!(damage_on(&landed, k), (dmg[1] * s.content.checks.perfect_damage).round() as u32);
}
