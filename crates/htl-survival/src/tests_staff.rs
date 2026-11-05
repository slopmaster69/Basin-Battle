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
    // The heavy is held, then let go.
    s.input(p, Input::Heavy);
    let ev = s.step(0.0);
    assert!(missiles(&ev, p).is_empty() && s.players[&p].action.name() == "charge", "a heavy bolt charges: {}", s.players[&p].action.name());
    run(&mut s, 1.3);
    s.input(p, Input::AttackUp);
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
    let shard = s.content.abilities[s.content.ability("rime_bolt").unwrap()].spell;
    assert_eq!(missiles(&ev, p), vec![shard], "the shard's own look, whatever it is tuned to");
    run(&mut s, 1.0);
    assert!(s.has_cc(near, "snare"), "chilled");
    s.input(p, Input::Heavy);
    run(&mut s, 1.3);
    s.input(p, Input::AttackUp);
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
    let bolt = s.content.abilities[s.content.ability("arcane_bolt").unwrap()].spell;
    assert_eq!(missiles(&ev, p), vec![bolt]);
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
    let bolt = s.content.abilities[s.content.ability("arcane_bolt").unwrap()].spell;
    assert_eq!(missiles(&ev, p), vec![bolt], "the next bolt goes as the recovery ends");
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

/// A sturdy foe at `pos` that stands still and never swings.
fn sturdy(s: &mut Survival, pos: Vec3) -> Id {
    let k = mob(s, "grunt", pos);
    let m = s.mobs.get_mut(&k).unwrap();
    m.health = 100_000;
    m.max_health = 100_000;
    k
}

/// One plain bolt at the foe ahead (put back 12 yd in front, standing), landed.
fn bolt(s: &mut Survival, p: Id, k: Id) -> Vec<Event> {
    let m = s.mobs.get_mut(&k).unwrap();
    m.pos = [12.0, 0.0, 0.0];
    m.state = MobState::Idle { wander_at: 1e9 };
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    let mut ev = s.step(0.0);
    s.input(p, Input::AttackUp);
    ev.extend(run(s, 1.0));
    ev
}

fn stacks(events: &[Event], who: Id, kind: &str) -> Vec<u32> {
    events.iter().filter_map(|e| match e {
        Event::Stacks { player, kind: k, n, .. } if *player == who && *k == kind => Some(*n),
        _ => None,
    }).collect()
}

#[test]
fn an_ember_staffs_bolts_heat_the_foe_and_the_third_ignites_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "ember_staff");
    let k = sturdy(&mut s, [12.0, 0.0, 0.0]);
    let ignite = s.content.staffs.ignite_spell;
    let mut ev = bolt(&mut s, p, k);
    ev.extend(bolt(&mut s, p, k));
    assert_eq!(stacks(&ev, p, "heat"), vec![1, 2], "each hit heats it");
    assert!(!ev.iter().any(|e| matches!(e, Event::AreaLook { spell, .. } if *spell == ignite)));
    let ev = bolt(&mut s, p, k);
    assert_eq!(stacks(&ev, p, "heat"), vec![3], "the third");
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell, .. } if *spell == ignite)), "ignites: the burst on the foe");
    assert!(s.staff_marks.iter().all(|m| m.on != k), "and the heat is spent");
}

#[test]
fn a_rime_staffs_shards_chill_the_foe_slower_each_time_and_the_third_freezes_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "rime_staff");
    let k = sturdy(&mut s, [12.0, 0.0, 0.0]);
    let mut ev = bolt(&mut s, p, k);
    ev.extend(bolt(&mut s, p, k));
    assert_eq!(stacks(&ev, p, "chill"), vec![1, 2]);
    assert!(s.has_cc(k, "snare"), "chilled: slowed");
    assert!(!s.has_cc(k, "petrify"));
    bolt(&mut s, p, k);
    assert!(s.has_cc(k, "petrify"), "the third chill freezes it");
}

#[test]
fn an_arcane_staffs_bolts_gather_charges_and_the_heavy_orb_spends_them_harder_for_each() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, "arcane_staff");
    let k = sturdy(&mut s, [12.0, 0.0, 0.0]);
    let hb = s.content.ability("arcane_bolt_heavy").unwrap();
    let top = s.content.abilities[hb].charge[1];
    let heavy = |s: &mut Survival| -> u32 {
        let m = s.mobs.get_mut(&k).unwrap();
        m.pos = [12.0, 0.0, 0.0];
        m.state = MobState::Idle { wander_at: 1e9 };
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Heavy);
        run(s, top + 0.05);
        s.input(p, Input::AttackUp);
        let ev = run(s, 1.5);
        ev.iter().filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == k => Some(*damage),
            _ => None,
        }).max().unwrap_or(0)
    };
    let plain = heavy(&mut s);
    assert!(plain > 0);
    run(&mut s, 1.0);
    bolt(&mut s, p, k);
    let ev = bolt(&mut s, p, k);
    assert_eq!(stacks(&ev, p, "charge"), vec![2]);
    assert_eq!(s.players[&p].staff_charges, 2);
    let spent = heavy(&mut s);
    let want = (plain as f32 * (1.0 + 2.0 * s.content.staffs.spend_share)).round() as u32;
    assert!(spent.abs_diff(want) <= 1, "two charges: {spent}, about {want}");
    assert_eq!(s.players[&p].staff_charges, 0, "spent");
    // Charges fade a while after the last bolt.
    bolt(&mut s, p, k);
    assert_eq!(s.players[&p].staff_charges, 1);
    let keep = s.content.staffs.keep;
    run(&mut s, keep + 0.2);
    assert_eq!(s.players[&p].staff_charges, 0, "faded");
}
