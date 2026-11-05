//! The stop check, rolling: Frozen Orb rolls out along the reticle hurting and slowing what it
//! passes; the key again stops it where it is and it bursts; stopped on a foe it's a perfect, the
//! bigger burst and a freeze on everything in it; stopped at the caster's feet they freeze;
//! rolled the whole way it fades.

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
    s.players.get_mut(&id).unwrap().valor = s.content.valor.max;
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

/// Frozen Orb on the ultimate key, cast east.
fn roll(s: &mut Survival, p: Id) {
    s.slot_ability(p, "ultimate", "frozen_orb").unwrap();
    s.input(p, Input::Ability(key_of("ultimate"), false));
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
    }
    assert_eq!(s.orbs.len(), 1, "an orb out");
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

fn perfect_in(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

#[test]
fn the_orb_rolls_out_along_the_reticle_hurting_and_slowing_what_it_passes_then_fades() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("frozen_orb").unwrap()].clone();
    let near = dummy(&mut s, [8.0, 0.5, 0.0]);
    let aside = dummy(&mut s, [8.0, 5.0, 0.0]);
    roll(&mut s, p);
    assert_eq!(s.players[&p].valor, 0.0, "the whole bar");
    // Past the near dummy: slowed as it passes.
    let speed = d.range[0] / d.linger;
    let mut ev = run(&mut s, (8.0 + 2.5) / speed);
    assert!(s.has_cc(near, "snare") && !s.has_cc(near, "petrify"), "slowed as it passes, not frozen");
    ev.extend(run(&mut s, d.linger));
    assert!(s.orbs.is_empty(), "rolled the whole way, it's gone");
    let ticks = blows_on(&ev, near);
    assert!(ticks.len() >= 2 && ticks.iter().all(|t| *t == (d.linger_damage * d.tick).round() as u32), "ticks as it rolls past: {ticks:?}");
    assert!(!s.has_cc(near, "petrify"));
    assert!(blows_on(&ev, aside).is_empty(), "5 yd aside: untouched");
    assert!(!perfect_in(&ev, p));
    // Its look: with a rolling model, one roll from where it starts to its range out, and gone at
    // the end; without one, a missile to where it is, again and again.
    if d.orb_model.is_empty() {
        assert!(ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == d.tick_spell)).count() >= 10, "its look rolls with it");
        let far = ev.iter().filter_map(|e| match e {
            Event::Missile { caster, spell, target: Target::Point(at) } if *caster == p && *spell == d.tick_spell => Some(at[0]),
            _ => None,
        }).fold(0.0, f32::max);
        assert!(far > d.range[0] * 0.8, "rolled out to {far:.1} yd");
    } else {
        let roll = ev.iter().find_map(|e| match e {
            Event::OrbRoll { look, to, secs, model, .. } => Some((*look, to[0], *secs, model.clone())),
            _ => None,
        });
        let (look, far, secs, model) = roll.expect("the orb's roll goes to the clients");
        assert_eq!(model, d.orb_model);
        assert!(far > d.range[0] * 0.8, "rolls out to {far:.1} yd");
        assert!(secs > 0.0);
        assert!(ev.iter().any(|e| matches!(e, Event::OrbGone { look: l } if *l == look)), "and is gone when spent");
        assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == d.tick_spell)), "no streak with it");
    }
}

#[test]
fn the_key_again_stops_the_orb_and_it_bursts_on_a_foe_it_is_a_perfect_that_freezes_the_burst() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("frozen_orb").unwrap()].clone();
    let speed = d.range[0] / d.linger;
    let on = dummy(&mut s, [10.0, 0.0, 0.0]);
    let beside = dummy(&mut s, [13.0, 3.0, 0.0]);
    let far = dummy(&mut s, [20.0, 0.0, 0.0]);
    roll(&mut s, p);
    // Rolled to the foe: from 1.5 yd ahead at its speed.
    run(&mut s, (10.0 - 1.5) / speed);
    let valor = s.players[&p].valor;
    s.input(p, Input::Ability(key_of("ultimate"), false));
    let ev = s.step(0.0);
    assert!(s.orbs.is_empty(), "stopped");
    assert!(perfect_in(&ev, p), "on the foe: a perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor, "a perfect's Valor on top of the hits': {} from {valor}", s.players[&p].valor);
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell, .. } if *spell == d.spell)), "the burst's look");
    let burst = |v: Id| blows_on(&ev, v).into_iter().max().unwrap_or(0);
    assert_eq!(burst(on), d.damage[1] as u32, "the big burst on the foe under it");
    assert_eq!(burst(beside), d.damage[1] as u32, "and on the one 4 yd off, inside the 6 yd burst");
    assert_eq!(burst(far), 0, "not 10 yd off");
    assert!(s.has_cc(on, "petrify") && s.has_cc(beside, "petrify"), "everything in the burst frozen");
    assert!(!s.has_cc(far, "petrify"));
    assert!(!s.has_cc(p, "petrify"), "no backfire 10 yd from the caster");
}

#[test]
fn stopped_short_of_the_foe_it_bursts_smaller_with_no_freeze_and_at_the_casters_feet_they_freeze() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("frozen_orb").unwrap()].clone();
    let speed = d.range[0] / d.linger;
    let k = dummy(&mut s, [12.0, 0.0, 0.0]);
    roll(&mut s, p);
    // Stopped 4 yd short: in the burst, not under the orb.
    run(&mut s, (8.0 - 1.5) / speed);
    s.input(p, Input::Ability(key_of("ultimate"), false));
    let ev = s.step(0.0);
    assert!(!perfect_in(&ev, p), "short of it: no perfect");
    assert_eq!(blows_on(&ev, k).into_iter().max().unwrap_or(0), d.damage[0] as u32, "the plain burst reaches it");
    assert!(!s.has_cc(k, "petrify"), "no freeze without the perfect");
    // Again, stopped at once: at the caster's own feet.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
    roll(&mut s, p);
    run(&mut s, 0.1);
    s.input(p, Input::Ability(key_of("ultimate"), false));
    let ev = s.step(0.0);
    assert!(!perfect_in(&ev, p));
    assert!(s.has_cc(p, "petrify"), "the backfire: the caster freezes");
}

#[test]
fn frozen_orb_is_the_mages_second_ultimate_taught_by_the_unbound_flame() {
    let s = world();
    let c = &s.content;
    let k = c.ability("frozen_orb").unwrap();
    let d = &c.abilities[k];
    assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice), ("mage", "ultimate", 2));
    assert!(d.on_board(), "on the board");
    let flame = c.mob("unbound_flame").unwrap();
    assert!(c.mobs[flame].teaches.contains(&"frozen_orb".to_string()) && c.mobs[flame].teaches.contains(&"meteor".to_string()), "both ultimates from the trial: {:?}", c.mobs[flame].teaches);
}
