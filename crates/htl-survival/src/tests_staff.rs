//! Staffs that cast: with an Ember, Rime or Arcane staff in hand the left click is a bolt from
//! the staff along the reticle, the heavy attack a heavier one, and the rest of the core (the
//! shove with the guard up, the Apprentice's Staff's swing, every other weapon) is as it was.

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

/// A hero holding `staff`, back where they stood.
fn armed(s: &mut Survival, name: &str, pos: Vec3, staff: &str) -> Id {
    let p = hero(s, name, pos, 0.0);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, staff).unwrap();
    s.players.get_mut(&p).unwrap().pos = pos;
    p
}

fn missiles(events: &[Event], caster: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Missile { caster: c, spell, .. } if *c == caster => Some(*spell),
            _ => None,
        })
        .collect()
}

fn swings(events: &[Event], attacker: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::Swing { attacker: a, anim: true, .. } if *a == attacker)).count()
}

#[test]
fn an_ember_staffs_click_casts_a_burning_bolt_along_the_reticle_not_a_swing() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "ember_staff");
    let w = s.weapon_of(&s.players[&p]).clone();
    assert!(w.casts() && w.bolt == "ember_bolt", "the staff casts");
    assert_eq!(s.guard_of(&s.players[&p]).key, "arcane_ward", "and guards with the ward");
    let k = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let (hp, st) = (s.mobs[&k].health, s.players[&p].stamina);
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![133], "Fireball's missile leaves the staff");
    assert_eq!(swings(&ev, p), 0, "no swing");
    assert_eq!(s.players[&p].action.name(), "cast", "the cast's recovery");
    let bolt = s.content.ability("ember_bolt").unwrap();
    assert!((st - s.players[&p].stamina - s.content.abilities[bolt].stamina).abs() < 0.01, "a bolt's stamina");
    let ev = run(&mut s, 1.0);
    assert_eq!(swings(&ev, p), 0);
    let took = hp - s.mobs[&k].health;
    assert!(took >= 18, "the bolt lands for the staff's damage: {took}");
    assert!(s.has_cc(k, "burn"), "and it burns");
    assert_eq!(s.players[&p].action, Action::Ready, "free again after the recovery");
}

#[test]
fn the_heavy_attack_casts_the_heavy_bolt_and_each_staff_has_its_own_school() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "ember_staff");
    let k = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let hp = s.mobs[&k].health;
    s.input(p, Input::Heavy);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![23024], "the heavy bolt's own look");
    run(&mut s, 1.5);
    assert!(hp - s.mobs[&k].health >= 34, "and its weight: {}", hp - s.mobs[&k].health);
    // Rime: a shard that chills; its heavy goes through everything in its line.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "rime_staff");
    // (Dummies: they stand where they're put, so the line holds.)
    let near = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let far = mob(&mut s, "training_dummy", [16.0, 0.0, 0.0]);
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![116]);
    run(&mut s, 1.0);
    assert!(s.has_cc(near, "snare"), "chilled");
    s.input(p, Input::Heavy);
    let ev = run(&mut s, 1.5);
    assert_eq!(missiles(&ev, p), vec![23412]);
    let struck = |v: Id| ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(x), damage, .. } if *x == v && *damage > 0));
    assert!(struck(near) && struck(far), "the lance goes through the first to the second: {} {}", struck(near), struck(far));
    // Arcane: quick, plain bolts.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "arcane_staff");
    let k = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![7268]);
    run(&mut s, 1.0);
    assert!(s.mobs[&k].health < 1000 && s.cc_of(k).is_empty(), "lands, with nothing on the foe");
}

#[test]
fn a_click_in_a_bolts_recovery_waits_for_its_end_then_goes() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "arcane_staff");
    let _k = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    s.input(p, Input::Attack);
    run(&mut s, 0.2);
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert!(missiles(&ev, p).is_empty(), "not yet: the first cast is still recovering");
    let ev = run(&mut s, 0.4);
    assert_eq!(missiles(&ev, p), vec![7268], "the next bolt goes as the recovery ends");
}

#[test]
fn with_the_guard_up_the_click_still_shoves_and_a_swinging_staff_still_swings() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "ember_staff");
    let _k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    run(&mut s, 0.2);
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert!(missiles(&ev, p).is_empty(), "no bolt with the guard up");
    assert!(matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Kick), "a kick, as with any two-handed weapon: {:?}", s.players[&p].action.name());
    // The Apprentice's Staff has no bolt: it swings.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "apprentice_staff");
    let _k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.3);
    assert!(missiles(&ev, p).is_empty());
    assert_eq!(swings(&ev, p), 1, "a swing");
}

#[test]
fn a_winded_hero_casts_no_bolt_and_the_rack_offers_the_staffs() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "ember_staff");
    s.players.get_mut(&p).unwrap().stamina = 0.0;
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert!(missiles(&ev, p).is_empty());
    assert!(ev.iter().any(|e| matches!(e, Event::Winded { player } if *player == p)), "too winded");
    let rack: Vec<String> = s.rack_view(p).into_iter().map(|(k, _, _, _, _)| k).collect();
    for staff in ["apprentice_staff", "ember_staff", "rime_staff", "arcane_staff"] {
        assert!(rack.contains(&staff.to_string()), "{staff} on the rack");
    }
}
