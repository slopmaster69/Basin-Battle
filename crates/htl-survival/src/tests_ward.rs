//! The ward: a staff's guard, with a bar of its own (`[ward]`). What a caster's right click does
//! in place of a shield's block: blows drain the bar, a perfect ward costs nothing and reels the
//! attacker, a blow too heavy for what's left shatters it, and a shattered ward stays down until
//! its bar is whole again.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    // It doesn't swing on its own: the tests strike for it.
    s.mobs.get_mut(&id).unwrap().ready_at = 1e9;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// A hero with the Apprentice's Staff in hand, standing where they were.
fn staff_hero(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let p = hero(s, name, pos, 0.0);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "apprentice_staff").unwrap();
    s.players.get_mut(&p).unwrap().pos = pos;
    p
}

/// A light blow from the creature in front, with the guard held long enough that it's no parry.
fn held_blow(s: &mut Survival, k: Id, p: Id, damage: u32) -> Outcome {
    s.strike_player(k, p, damage, [2.0, 0.0, 0.0], true, false, true, Blow::Light)
}

#[test]
fn a_staff_guards_with_a_ward_whose_bar_pays_for_blocks_not_stamina() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    let guard = s.guard_of(&s.players[&p]).clone();
    assert!(guard.ward && guard.key == "arcane_ward", "the staff guards with the ward");
    let bar = s.content.ward.bar;
    assert_eq!(s.players[&p].ward, bar, "a hero starts with a whole bar");
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    let stamina = s.players[&p].stamina;
    assert_eq!(held_blow(&mut s, k, p, 20), Outcome::Blocked);
    let cost = guard.cost + 20.0 * guard.cost_per_damage;
    assert!((s.players[&p].ward - (bar - cost)).abs() < 0.01, "the bar pays {cost}: {}", s.players[&p].ward);
    assert_eq!(s.players[&p].stamina, stamina, "stamina doesn't");
    assert_eq!(s.players[&p].health, s.content.player.health, "nothing lands");
    // The bar is told to the hero as it changes.
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Ward { player } if *player == p)), "the bar's line");
    // A heavy blow costs twice as much.
    let before = s.players[&p].ward;
    assert_eq!(s.strike_player(k, p, 20, [2.0, 0.0, 0.0], true, true, true, Blow::Light), Outcome::Blocked);
    assert!((before - s.players[&p].ward - 2.0 * cost).abs() < 0.01);
}

#[test]
fn the_ward_refills_only_while_it_is_down() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    let w = s.content.ward.clone();
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    held_blow(&mut s, k, p, 20);
    let drained = s.players[&p].ward;
    assert!(drained < w.bar);
    // Held up: nothing comes back.
    run(&mut s, 2.0);
    assert_eq!(s.players[&p].ward, drained, "no refill while the ward is up");
    // Let down: it waits a moment, then refills at its rate.
    s.input(p, Input::GuardUp);
    run(&mut s, w.refill_delay * 0.5);
    assert_eq!(s.players[&p].ward, drained, "the delay first");
    run(&mut s, w.refill_delay * 0.5 + 0.5);
    let back = s.players[&p].ward - drained;
    assert!(back > w.refill * 0.4 && back <= w.refill * 0.6, "about half a second's refill: {back}");
    run(&mut s, 3.0);
    assert_eq!(s.players[&p].ward, w.bar, "whole again");
}

#[test]
fn a_perfect_ward_costs_nothing_gives_some_bar_back_bursts_and_reels_the_attacker() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    let w = s.content.ward.clone();
    // Spend some bar first, so the perfect's refill shows.
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    held_blow(&mut s, k, p, 30);
    s.input(p, Input::GuardUp);
    run(&mut s, 1.0);
    let (bar, valor) = (s.players[&p].ward, s.players[&p].valor);
    // The guard raised just in time.
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    assert_eq!(held_blow(&mut s, k, p, 30), Outcome::Parried, "a perfect ward");
    let p1 = &s.players[&p];
    assert!((p1.ward - (bar + w.perfect_refill).min(w.bar)).abs() < 0.01, "nothing spent and some back: {} from {bar}", p1.ward);
    assert_eq!(p1.health, s.content.player.health);
    assert_eq!(p1.valor, valor + s.content.valor.parry, "a parry's Valor");
    assert!(p1.opening_until > s.clock, "and a parry's opening");
    let m = &s.mobs[&k];
    assert!(m.stagger_until > s.clock + 1.0 && m.riposte, "the attacker reels, open to a riposte");
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == w.perfect_kit)), "the burst");
}

#[test]
fn a_blow_too_heavy_for_the_bar_shatters_the_ward_which_stays_down_until_the_bar_is_whole() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    let w = s.content.ward.clone();
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    s.players.get_mut(&p).unwrap().ward = 10.0;
    let hp = s.players[&p].health;
    assert_eq!(held_blow(&mut s, k, p, 30), Outcome::GuardBroken, "the ward shatters");
    let p1 = &s.players[&p];
    assert_eq!(p1.ward, 0.0);
    assert!(p1.ward_broken);
    let took = hp - p1.health;
    assert!(took > 0 && took < 30, "part of the blow lands: {took}");
    assert_eq!(p1.action.name(), "stagger", "and the hero reels");
    assert!(p1.stamina > 0.0, "stamina is untouched");
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == w.break_kit)), "the shatter is seen");
    assert!(!ev.iter().any(|e| matches!(e, Event::Tell { player, .. } if *player == p)), "and not read");
    // Up from the stagger with the guard still held: the ward doesn't come back.
    run(&mut s, w.break_secs + 0.2);
    assert_eq!(s.players[&p].action, Action::Ready, "no ward while it's broken: {:?}", s.players[&p].action);
    // Pressed again: refused, the bar says so, and the blow lands whole.
    s.input(p, Input::GuardUp);
    s.input(p, Input::Guard);
    assert_eq!(s.players[&p].action, Action::Ready);
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Ward { player } if *player == p)));
    assert_eq!(held_blow(&mut s, k, p, 10), Outcome::Hit);
    // Whole again once the bar has refilled: the ward goes up on the next press.
    s.input(p, Input::GuardUp);
    run(&mut s, w.refill_delay + w.bar / w.refill + 0.2);
    assert!(!s.players[&p].ward_broken && s.players[&p].ward == w.bar, "whole: {}", s.players[&p].ward);
    s.input(p, Input::Guard);
    assert_eq!(s.players[&p].action, Action::Guard);
}

#[test]
fn a_heavy_blow_through_the_ward_puts_the_hero_down() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let k = mob(&mut s, "overseer", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    s.players.get_mut(&p).unwrap().ward = 5.0;
    assert_eq!(s.strike_player(k, p, 42, [2.0, 0.0, 0.0], true, true, false, Blow::Light), Outcome::GuardBroken);
    assert_eq!(s.players[&p].action.name(), "stagger");
    let secs = s.content.ward.break_secs + 0.1;
    run(&mut s, secs);
    assert_eq!(s.players[&p].action.name(), "stagger", "a heavy creature's blow keeps them down longer");
}

#[test]
fn a_perfect_ward_against_a_hero_is_a_parry_the_striker_reels_open_to_a_riposte() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let r = hero(&mut s, "Rook", [2.0, 0.0, 0.0], std::f32::consts::PI);
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    assert_eq!(s.strike_player(r, p, 20, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Parried);
    let rook = &s.players[&r];
    assert_eq!(rook.action.name(), "stagger", "the striking hero reels");
    assert!(rook.riposte_until > s.clock, "open to a riposte");
    // Held a while against a hero: a block, off the bar.
    run(&mut s, 1.5);
    let bar = s.players[&p].ward;
    assert_eq!(s.strike_player(r, p, 20, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Blocked);
    assert!(s.players[&p].ward < bar);
}

#[test]
fn the_wards_look_is_on_the_hero_while_it_is_up_and_a_shield_hero_has_no_bar() {
    let mut s = world();
    let p = staff_hero(&mut s, "Ayla", ORIGIN);
    let aura = s.content.ward.aura;
    assert!(!s.auras_of(p).contains(&aura));
    s.input(p, Input::Guard);
    let ev = run(&mut s, 0.1);
    assert!(s.auras_of(p).contains(&aura), "the barrier shows while the ward is up");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == p)), "and the clients are told");
    s.input(p, Input::GuardUp);
    run(&mut s, 0.1);
    assert!(!s.auras_of(p).contains(&aura), "and goes with it");
    // A hero with a shield guards as ever: stamina, no ward, no look.
    let q = hero(&mut s, "Bram", [0.0, 5.0, 0.0], 0.0);
    assert!(!s.guard_of(&s.players[&q]).ward);
    let k = mob(&mut s, "grunt", [2.0, 5.0, 0.0]);
    s.input(q, Input::Guard);
    run(&mut s, 1.0);
    let (st, bar) = (s.players[&q].stamina, s.players[&q].ward);
    assert_eq!(s.strike_player(k, q, 20, [2.0, 5.0, 0.0], true, false, true, Blow::Light), Outcome::Blocked);
    assert!(s.players[&q].stamina < st && s.players[&q].ward == bar, "stamina pays, the bar is untouched");
    assert!(!s.auras_of(q).contains(&aura));
}
