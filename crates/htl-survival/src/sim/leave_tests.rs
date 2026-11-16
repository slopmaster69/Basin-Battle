//! What a hero leaves behind when they leave (`Survival::leave` -> `drop_hero_owned`): their
//! totems, traps, wind-ups, anchor, orbs, bombs, flay and lockouts go with them, other heroes'
//! things stay, and a hero who rejoins is a new id that inherits none of it.

use super::*;
use crate::land::FlatLand;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, [0.0; 3], 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// Everything owned by `id`, one of each list, with the marks they hold; returns the marks.
fn litter(s: &mut Survival, id: Id) -> Vec<Id> {
    let now = s.clock;
    let ab = 0;
    let target = s.spawn_mob(s.content.mob("training_dummy").unwrap(), [4.0, 0.0, 0.0], None, false);
    let totem = s.spawn_mob(s.content.mob("training_dummy").unwrap(), [2.0, 2.0, 0.0], None, false);
    s.totems.push(abilities::Totem { mob: totem, owner: id, ability: ab, until: now + 99.0, next: now });
    let trap_mark = s.mark([1.0, 1.0, 0.0], 1.0, 99.0);
    s.traps.push(moves::Trap { owner: id, ability: ab, at: [1.0, 1.0, 0.0], share: 1.0, armed_at: now, until: now + 99.0, mark: trap_mark, obj: None });
    let wind_mark = s.mark([2.0, 1.0, 0.0], 1.0, 99.0);
    s.windups.push(moves::Windup { owner: id, ability: ab, at: [2.0, 1.0, 0.0], yaw: 0.0, share: 1.0, go_at: now + 5.0, mark: Some(wind_mark), fixed: true });
    let anchor_mark = s.mark([0.0, 3.0, 0.0], 1.0, 99.0);
    s.anchors.insert(id, moves::Anchor { ability: ab, at: [0.0, 3.0, 0.0], until: now + 99.0, mark: anchor_mark, since: now, health: 100, pulsed: 0 });
    s.orbs.push(orb::Orb { owner: id, ability: ab, pos: [1.0; 3], dir: [1.0, 0.0], speed: 1.0, until: now + 99.0, next_tick: now + 99.0, next_look: now + 99.0, look: 9_999 });
    s.bombs.push(bomb::Bomb { owner: id, ability: ab, on: target, since: now, flared: 0, at: [4.0, 0.0, 0.0] });
    s.flays.insert(id, flay::Flay { taut_since: None, last_taut: 0.0, rang: false, shown: 0.0, shown_at: 0.0 });
    s.lockouts.insert((id, ab), now + 99.0);
    s.lockouts.insert((target, ab), now + 99.0);
    vec![trap_mark, wind_mark, anchor_mark]
}

fn owned_by(s: &Survival, id: Id) -> usize {
    s.totems.iter().filter(|t| t.owner == id).count()
        + s.traps.iter().filter(|t| t.owner == id).count()
        + s.windups.iter().filter(|w| w.owner == id).count()
        + usize::from(s.anchors.contains_key(&id))
        + s.orbs.iter().filter(|o| o.owner == id).count()
        + s.bombs.iter().filter(|b| b.owner == id).count()
        + usize::from(s.flays.contains_key(&id))
        + s.lockouts.keys().filter(|(h, _)| *h == id).count()
}

#[test]
fn leaving_clears_everything_the_hero_owned_at_once() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    let marks = litter(&mut s, a);
    assert_eq!(owned_by(&s, a), 8);
    let ev = s.leave(a);
    assert!(ev.is_some());
    assert_eq!(owned_by(&s, a), 0, "nothing of theirs left");
    assert!(s.totems.is_empty());
    for m in marks {
        assert_eq!(s.mark_left(m), 0.0, "their marks are gone with them");
    }
    // The totem's body went too, not only its entry.
    assert!(s.mobs.values().all(|m| m.def == s.content.mob("training_dummy").unwrap()) && s.mobs.len() == 1, "the totem's body is gone");
    // Nothing springs, lands or rolls afterwards, and no tick trips over it.
    for _ in 0..200 {
        s.step(0.05);
    }
    assert!(s.windups.is_empty() && s.traps.is_empty() && s.orbs.is_empty() && s.bombs.is_empty());
}

#[test]
fn leaving_keeps_what_other_heroes_own() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    let b = hero(&mut s, "Bren");
    litter(&mut s, a);
    litter(&mut s, b);
    s.leave(a);
    assert_eq!(owned_by(&s, a), 0);
    // Bren's lockout on the dummy (keyed by the dummy) is theirs-by-target, not Ayla's: Ayla's
    // went, the dummy's stayed (one entry), and all of Bren's are untouched.
    assert_eq!(owned_by(&s, b), 8, "Bren's own are all still there");
}

#[test]
fn a_rejoining_hero_is_a_new_id_with_none_of_the_old_state() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    litter(&mut s, a);
    s.lockouts.insert((a, 1), s.clock + 99.0);
    let save = s.leave(a).unwrap();
    let b = s.join("ayla", "Ayla", Some(&save));
    assert_ne!(a, b, "ids are never reused");
    assert_eq!(owned_by(&s, a), 0);
    assert_eq!(owned_by(&s, b), 0, "the rejoined hero starts with no traps, orbs, anchor or lockouts");
    assert!(!s.lockouts.keys().any(|(h, _)| *h == a || *h == b));
}

#[test]
fn leaving_mid_flay_does_not_leak_the_cord() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    s.flays.insert(a, flay::Flay { taut_since: Some(0.0), last_taut: 0.0, rang: false, shown: 0.5, shown_at: 0.0 });
    s.leave(a);
    assert!(s.flays.is_empty());
}

#[test]
fn a_departed_heros_windup_never_lands() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    let d = s.spawn_mob(s.content.mob("training_dummy").unwrap(), [1.0, 0.0, 0.0], None, false);
    let hp = s.mobs[&d].health;
    // A fixed wind-up that would hit the dummy standing next to the spot.
    let ab = s.content.abilities.iter().position(|x| x.kind == "nova" && x.damage[1] > 0.0).expect("a nova card");
    s.windups.push(moves::Windup { owner: a, ability: ab, at: [0.0; 3], yaw: 0.0, share: 1.0, go_at: s.clock + 0.2, mark: None, fixed: true });
    s.leave(a);
    for _ in 0..40 {
        s.step(0.05);
    }
    assert_eq!(s.mobs[&d].health, hp, "nothing landed for the one who left");
}

/// Found by the soak: a leaver's spoken-refusal clock and crowd-control log were never dropped, a
/// pair of entries left behind for every hero who ever left (ids are never reused).
#[test]
fn a_leaver_takes_their_refusal_clock_and_cc_log_with_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    s.refuse(a, "Out of range.", cues::Vocal::OutOfRange);
    s.cc_note(a, "stunned".into());
    s.set_error_speech(a, false);
    assert!(s.told_vocal.contains_key(&a) || s.no_error_speech.contains(&a), "set up: something was kept for them");
    assert!(s.cc_log.contains_key(&a));
    s.leave(a);
    assert!(!s.told_vocal.contains_key(&a), "refusal clock");
    assert!(!s.cc_log.contains_key(&a), "cc log");
    assert!(!s.no_error_speech.contains(&a), "error speech switch");
}

/// Found by the soak: the Hunter's Call of the Wild chain and the Priest's per-hero notes were
/// kept by hero id for good, a few entries for every Hunter or Priest who ever left.
#[test]
fn a_leaver_takes_their_class_notes_with_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla");
    s.hunt.wild.insert(a, hunter::Wild::default());
    s.light.deaths.insert(a, true);
    s.light.recoiled.insert(a, 1.0);
    s.light.last_bolt.insert(a, 1.0);
    s.light.empower_shown.insert(a);
    s.leave(a);
    assert!(s.hunt.wild.is_empty(), "the wild chain");
    assert!(s.light.deaths.is_empty() && s.light.recoiled.is_empty() && s.light.last_bolt.is_empty() && s.light.empower_shown.is_empty(), "the priest's notes");
}
