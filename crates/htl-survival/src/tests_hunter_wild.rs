//! Call of the Wild after Corey's play-test (#110): one press goes at the foe the camera looks at,
//! a call has a short recharge and costs more, a parry or a perfect dodge empowers the next for a
//! few seconds, and the beasts stay to be seen, out of everyone's way.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::hunter::{BEASTS, WILD_PRIMED_SECS, WILD_STAMINA};
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
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

/// A creature standing still.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.health = 9999;
    m.max_health = 9999;
    id
}

/// Run on with these creatures held where they stand, idle.
fn run_pinned(s: &mut Survival, secs: f32, mobs: &[Id]) -> Vec<Event> {
    let at: Vec<(Id, Vec3)> = mobs.iter().filter_map(|m| Some((*m, s.mobs.get(m)?.pos))).collect();
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, pos) in &at {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *pos;
            }
        }
        out.extend(s.step(DT));
    }
    out
}

fn close_key() -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == "close").unwrap() as u8
}

/// Call of the Wild on the E key, cooldowns as they are (`off`: none).
fn slot_call(s: &mut Survival, p: Id, off: bool) {
    s.slot_ability(p, "close", "call_of_the_wild").unwrap();
    s.set_cooldowns(p, off);
}

/// The E key pressed, the camera's line coming with it as the client sends it (`None`: no line).
fn call(s: &mut Survival, p: Id, aim: Option<(f32, f32)>) -> Vec<Event> {
    if let Some((f, e)) = aim {
        s.set_aim_line(p, f, e);
    }
    s.input(p, Input::Ability(close_key(), false));
    s.step(DT)
}

/// The damage of each of `p`'s blows that landed on `m`.
fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, anim: false, .. } if *attacker == p && *v == m && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

fn spirits(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key.starts_with("spirit_")).map(|(id, _)| *id).collect()
}

#[test]
fn with_no_lock_one_press_sends_the_beast_at_the_foe_the_camera_looks_at() {
    let mut s = world();
    // The hero turned away; the camera looks the other way, along +x.
    let p = hero(&mut s, "Ayla", ORIGIN, std::f32::consts::PI);
    slot_call(&mut s, p, true);
    let ahead = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    // Nearer, and within the cone, but off the camera's line.
    let aside = mob(&mut s, "grunt", [6.0, 3.5, 0.0]);
    assert!(s.players[&p].lock.is_none());
    let mut ev = call(&mut s, p, Some((0.0, 0.0)));
    assert!(!matches!(s.players[&p].action, Action::Aim { .. }), "no aiming: it goes at once");
    ev.extend(run_pinned(&mut s, 0.8, &[ahead, aside]));
    assert!(!hits(&ev, p, ahead).is_empty(), "the beast strikes the foe ahead of the camera");
    assert!(hits(&ev, p, aside).is_empty(), "not the one off the line");
}

#[test]
fn a_press_kept_past_a_swing_still_goes_where_the_camera_was() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, std::f32::consts::PI);
    slot_call(&mut s, p, true);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.set_aim_line(p, 0.0, 0.0);
    run_pinned(&mut s, 1.0, &[k]);
    let mut ev = call(&mut s, p, None);
    ev.extend(run_pinned(&mut s, 0.8, &[k]));
    assert!(!hits(&ev, p, k).is_empty(), "the camera's line from a second ago, not the hero's back");
}

#[test]
fn the_lock_still_wins() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, true);
    let ahead = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    let locked = mob(&mut s, "grunt", [0.0, 10.0, 0.0]);
    s.players.get_mut(&p).unwrap().lock = Some(locked);
    let mut ev = call(&mut s, p, Some((0.0, 0.0)));
    ev.extend(run_pinned(&mut s, 0.8, &[ahead, locked]));
    assert!(!hits(&ev, p, locked).is_empty());
    assert!(hits(&ev, p, ahead).is_empty());
}

#[test]
fn a_call_costs_its_stage_and_waits_a_moment_for_the_next_and_a_miss_costs_nothing() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, false);
    let a = s.content.ability("call_of_the_wild").unwrap();
    // Nothing there: no beast, no stamina, no recharge.
    let ev = call(&mut s, p, Some((0.0, 0.0)));
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text == "No enemy in range")));
    assert!((s.players[&p].stamina - 100.0).abs() < 0.01);
    assert_eq!(s.cooldown(p, a).charges, 1, "a miss spends no recharge");
    run_pinned(&mut s, 1.0, &[]);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    call(&mut s, p, Some((0.0, 0.0)));
    let spent = 100.0 - s.players[&p].stamina;
    assert!((spent - WILD_STAMINA[0]).abs() < 0.5, "the first stage's stamina: {spent}");
    assert!(WILD_STAMINA.iter().sum::<f32>() >= 85.0, "a whole chain all but empties the bar");
    run_pinned(&mut s, 0.6, &[k]);
    assert_eq!(spirits(&s).len(), 1);
    // Again at once: not yet.
    let mut ev = call(&mut s, p, Some((0.0, 0.0)));
    ev.extend(run_pinned(&mut s, 0.2, &[k]));
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text.contains("ready in"))), "{ev:?}");
    assert_eq!(spirits(&s).len(), 1, "no second beast yet");
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    run_pinned(&mut s, 0.8, &[k]);
    call(&mut s, p, Some((0.0, 0.0)));
    run_pinned(&mut s, 0.1, &[k]);
    assert_eq!(spirits(&s).len(), 2, "recharged: the second beast, the first still there");
}

#[test]
fn a_parry_empowers_the_next_call_for_a_few_seconds() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Without Call of the Wild on a key, nothing to empower.
    s.players.get_mut(&p).unwrap().loadout.clear();
    s.prime_wild(p);
    assert!(!s.wild_now(p).primed);
    slot_call(&mut s, p, true);
    // Before the first call too.
    s.prime_wild(p);
    assert!(s.wild_now(p).primed);
    assert!((s.wild_primed_left(p) - WILD_PRIMED_SECS).abs() < 0.01);
    let a = s.content.ability("call_of_the_wild").unwrap();
    assert!((s.hunt_stamina(p, a) - WILD_STAMINA[0] * 0.5).abs() < 0.01, "half");
    run_pinned(&mut s, WILD_PRIMED_SECS + 0.2, &[]);
    assert!(!s.wild_now(p).primed, "gone after its seconds");
    assert_eq!(s.wild_primed_left(p), 0.0);
    assert!((s.hunt_stamina(p, a) - WILD_STAMINA[0]).abs() < 0.01);
    // Spent by the call it empowers.
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.prime_wild(p);
    call(&mut s, p, Some((0.0, 0.0)));
    run_pinned(&mut s, 0.8, &[k]);
    assert!(!s.wild_now(p).primed);
}

#[test]
fn a_blow_landed_no_longer_empowers_the_call() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, true);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    call(&mut s, p, Some((0.0, 0.0)));
    let ev = run_pinned(&mut s, 0.8, &[k]);
    // (Its own beast's strike landed, and the next call is a full one.)
    assert!(!hits(&ev, p, k).is_empty());
    assert!(!s.wild_now(p).primed);
}

#[test]
fn the_beast_strikes_with_its_own_look_roars_lingers_and_goes_in_a_flash() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, true);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    let mut ev = call(&mut s, p, Some((0.0, 0.0)));
    let body = spirits(&s)[0];
    let beast = BEASTS.iter().find(|b| s.content.mob(b.mob) == Some(s.mobs[&body].def)).unwrap();
    assert!(s.spirit_body(body));
    ev.extend(run_pinned(&mut s, 0.8, &[k]));
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell, target: Target::Unit(t) } if *caster == body && *spell == beast.strike && *t == k)),
        "{}'s own strike, from it onto the foe",
        beast.name
    );
    ev.extend(run_pinned(&mut s, 0.9, &[k]));
    assert!(ev.iter().any(|e| matches!(e, Event::Emote { id, emote: 15 } if *id == body)), "it roars");
    assert!(s.mobs.contains_key(&body), "still there, a second after its strike");
    let ev = run_pinned(&mut s, 1.0, &[k]);
    assert!(!s.mobs.contains_key(&body), "gone after");
    assert!(ev.iter().any(|e| matches!(e, Event::MobGone { id } if *id == body)));
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { target: Target::Point(_), .. })), "in a flash");
}

#[test]
fn a_beast_is_never_locked_on_nor_in_the_way() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, true);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    call(&mut s, p, Some((0.0, 0.0)));
    run_pinned(&mut s, 0.8, &[k]);
    let body = spirits(&s)[0];
    // The foe gone, only the beast is left in front of the hero: Tab finds nothing.
    s.mobs.remove(&k);
    s.input(p, Input::Lock);
    s.step(DT);
    assert_ne!(s.players[&p].lock, Some(body));
    assert!(s.players[&p].lock.is_none());
}

#[test]
fn every_beast_has_a_body_and_an_icon_of_its_own_animal() {
    let s = world();
    let mut icons = std::collections::BTreeSet::new();
    for b in &BEASTS {
        assert!(s.content.mob(b.mob).is_some(), "{} has a body", b.name);
        assert!(b.icon.starts_with("Ability_Hunter_Pet_") || b.icon.starts_with("Ability_Mount_"), "{}: {}", b.name, b.icon);
        assert!(icons.insert(b.icon), "{}'s icon is its own", b.name);
        assert!(b.strike != 0);
    }
}

#[test]
fn a_beast_appears_by_the_foe_and_strikes_from_its_flank_clear_of_the_hunters_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    slot_call(&mut s, p, true);
    let k = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    let ev = call(&mut s, p, Some((0.0, 0.0)));
    let body = spirits(&s)[0];
    // It appears by the foe, off to one side, not out of the hunter, and charges in.
    let start = ev
        .iter()
        .find_map(|e| match e {
            Event::MobMove { id, path, .. } if *id == body => path.first().copied(),
            _ => None,
        })
        .expect("it charges");
    assert!(dist2d(start, [6.0, 0.0, 0.0]) < dist2d(start, ORIGIN), "nearer the foe than the hunter: {start:?}");
    assert!(start[1].abs() > 1.5, "off to the side, in the camera's view: {start:?}");
    run_pinned(&mut s, 0.8, &[k]);
    // The action camera looks down the hunter's back at the foe: a beast on that line (between
    // them) is hidden behind her. Beside the foe, well off the line, it's seen.
    let at = s.mobs[&body].pos;
    assert!(at[1].abs() > 1.2, "off the line to one side: {at:?}");
    assert!(at[0] > 4.5, "up by the foe, not at the hunter: {at:?}");
}
