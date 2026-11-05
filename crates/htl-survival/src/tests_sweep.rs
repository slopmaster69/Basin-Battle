//! The stop check: Cone of Cold is a jet that sweeps across the hero's front, striking each foe
//! it passes once and slowing them; the key again stops it where it is, and what's in the jet
//! freezes. Stopped with a foe at its centre it's a perfect. Not stopped, it finishes and they're
//! only slowed.

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

/// A dummy `yards` out, `deg` to the left of the hero's facing (east).
fn dummy_at(s: &mut Survival, yards: f32, deg: f32) -> Id {
    let a = deg.to_radians();
    let def = s.content.mob("training_dummy").unwrap();
    let id = s.spawn_mob(def, [yards * a.cos(), yards * a.sin(), 0.0], None, false);
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

fn blows_on(events: &[Event], victim: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

fn missiles(events: &[Event], caster: Id, spell: u32) -> usize {
    events.iter().filter(|e| matches!(e, Event::Missile { caster: c, spell: sp, .. } if *c == caster && *sp == spell)).count()
}

fn perfect_in(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

/// Cone of Cold on the close key: its key, then the click (it's aimed like any cast): the sweep
/// begins.
fn sweep(s: &mut Survival, p: Id) {
    s.slot_ability(p, "close", "cone_of_cold").unwrap();
    s.input(p, Input::Ability(key_of("close"), false));
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
    }
    assert_eq!(s.players[&p].action.name(), "sweep", "{:?}", s.players[&p].action.name());
}

#[test]
fn the_jet_sweeps_across_the_front_striking_each_foe_once_and_slowing_them() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("cone_of_cold").unwrap()].clone();
    assert_eq!(d.kind, "sweep");
    let left = dummy_at(&mut s, 6.0, 45.0);
    let mid = dummy_at(&mut s, 6.0, 0.0);
    let right = dummy_at(&mut s, 6.0, -45.0);
    let far = dummy_at(&mut s, 14.0, 0.0);
    sweep(&mut s, p);
    let ev = run(&mut s, d.secs + 0.2);
    for (who, name) in [(left, "left"), (mid, "middle"), (right, "right")] {
        assert_eq!(blows_on(&ev, who), vec![d.damage[0] as u32], "{name}: struck once as the jet passes");
        assert!(s.has_cc(who, "snare") && !s.has_cc(who, "petrify"), "{name}: slowed, not frozen");
    }
    assert!(blows_on(&ev, far).is_empty(), "out of the jet's reach");
    assert!(missiles(&ev, p, d.tick_spell) >= 8, "the jet streams to its reach: {}", missiles(&ev, p, d.tick_spell));
    assert_eq!(s.players[&p].action, Action::Ready, "over");
    assert!(!perfect_in(&ev, p));
}

#[test]
fn the_key_again_stops_the_jet_and_freezes_what_is_in_it_a_centred_foe_is_a_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("cone_of_cold").unwrap()].clone();
    let k = dummy_at(&mut s, 6.0, 0.0);
    sweep(&mut s, p);
    // Halfway through, the jet's centre is dead ahead: stop it there.
    run(&mut s, d.secs / 2.0);
    let valor = s.players[&p].valor;
    s.input(p, Input::Ability(key_of("close"), false));
    let ev = s.step(0.0);
    assert!(missiles(&ev, p, d.spell) == 1, "the cone's look where it stopped");
    assert!(s.has_cc(k, "petrify"), "frozen in the held jet");
    assert!(perfect_in(&ev, p), "at the centre: a perfect");
    assert_eq!(s.players[&p].valor, valor + s.content.checks.perfect_valor);
    assert!(s.players[&p].facing.abs() < 0.05, "the hero faces where the jet holds: {}", s.players[&p].facing);
    assert_eq!(s.players[&p].action.name(), "sweep", "held a moment");
    run(&mut s, d.stop_secs + 0.1);
    assert_eq!(s.players[&p].action, Action::Ready);
    // A second press while it holds does nothing more.
}

#[test]
fn stopped_with_the_foe_off_centre_it_freezes_without_a_perfect_and_off_the_jet_it_only_slows() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("cone_of_cold").unwrap()].clone();
    let k = dummy_at(&mut s, 6.0, 0.0);
    sweep(&mut s, p);
    // A little early: the centre is 12° to the left of the foe, inside the 30° jet but past the
    // 10° of a perfect.
    run(&mut s, d.secs * 0.4);
    s.input(p, Input::Ability(key_of("close"), false));
    let ev = s.step(0.0);
    assert!(s.has_cc(k, "petrify"), "in the jet: frozen");
    assert!(!perfect_in(&ev, p), "not centred: no perfect");
    // Far too early: the jet is nowhere near, and the foe (passed later or not at all) is only
    // slowed, if that.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = dummy_at(&mut s, 6.0, 0.0);
    sweep(&mut s, p);
    run(&mut s, 0.1);
    s.input(p, Input::Ability(key_of("close"), false));
    let ev = s.step(0.0);
    assert!(!s.has_cc(k, "petrify") && !perfect_in(&ev, p), "the jet held well to the left of it");
}

#[test]
fn the_guard_does_not_interrupt_a_sweep_and_the_cooldown_is_spent_once() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("cone_of_cold").unwrap()].clone();
    let _k = dummy_at(&mut s, 6.0, 0.0);
    sweep(&mut s, p);
    s.input(p, Input::Guard);
    assert_eq!(s.players[&p].action.name(), "sweep");
    run(&mut s, d.secs / 2.0);
    s.input(p, Input::Ability(key_of("close"), false));
    run(&mut s, d.stop_secs + 0.1);
    assert_eq!(s.players[&p].action, Action::Guard, "the held guard comes up after");
    s.input(p, Input::GuardUp);
    s.input(p, Input::Ability(key_of("close"), false));
    assert_eq!(s.players[&p].action, Action::Ready, "on cooldown: no second sweep");
}

