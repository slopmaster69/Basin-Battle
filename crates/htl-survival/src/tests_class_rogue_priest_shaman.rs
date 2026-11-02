//! The redesigned rogue, priest and shaman cards (#40), each cast from the real content: the thing that makes it
//! play differently.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules on flat land, no creatures but the test's (`pvp`: friendly fire on).
fn world(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    c.dev.full_tree = true;
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

/// A grunt standing idle, facing the hero at the origin.
fn grunt(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

/// Creatures held idle where they are (struck, they'd come for the hero).
fn pin(s: &mut Survival, pins: &[(Id, Vec3)]) {
    for (m, at) in pins {
        if let Some(x) = s.mobs.get_mut(m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = *at;
        }
    }
}

fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        pin(s, pins);
        out.extend(s.step(DT));
    }
    pin(s, pins);
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Own a card and put it on its key (cooldowns left on).
fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// Its key pressed (`ex`: with Ctrl).
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    s.step(DT)
}

/// A tap of the click (an aimed ability goes).
fn click(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

/// A card learned, on its key, and used (its EX version with `ex`, Valor enough for it); an
/// aimed one goes on a tap of the click.
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    learn(s, p, key);
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    if ex {
        s.set_valor(p, 100.0);
    }
    let mut ev = press(s, p, &slot, ex);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        ev.extend(click(s, p));
    }
    ev
}

/// The blows that landed on a unit from this hero.
fn hits(ev: &[Event], by: Id, on: Id) -> usize {
    ev.iter()
        .filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, anim: false, .. } if *attacker == by && *v == on && *damage > 0))
        .count()
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

// --- Rogue --------------------------------------------------------------------------------------

#[test]
fn fan_of_knives_lands_all_three_knives_point_blank_one_from_afar_and_none_past_its_reach() {
    // In their face: all three knives, and the poison.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = grunt(&mut s, [2.5, 0.0, 0.0]);
    let pins = [(near, [2.5, 0.0, 0.0])];
    let mut ev = cast(&mut s, p, "fan_of_knives", false);
    ev.extend(run_pinned(&mut s, 0.4, &pins));
    assert_eq!(hits(&ev, p, near), 3, "three knives in the one up close");
    assert!(s.has_cc(near, "burn"), "poisoned");
    assert!(s.auras_of(near).contains(&1978), "its poison shows");
    let spent = s.cooldown(p, idx(&s, "fan_of_knives"));
    assert_eq!(spent.charges, 1, "one of its two in hand spent");

    // From 12 yd the fan has opened past them: one knife.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let far = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(far, [12.0, 0.0, 0.0])];
    let mut ev = cast(&mut s, p, "fan_of_knives", false);
    ev.extend(run_pinned(&mut s, 0.8, &pins));
    assert_eq!(hits(&ev, p, far), 1, "only the middle knife reaches that far in line");

    // Past its 16 yd: nothing.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let out = grunt(&mut s, [18.0, 0.0, 0.0]);
    let pins = [(out, [18.0, 0.0, 0.0])];
    let before = hp(&s, out);
    let mut ev = cast(&mut s, p, "fan_of_knives", false);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(hits(&ev, p, out), 0);
    assert_eq!(hp(&s, out), before, "out of its reach");
    // Its EX version is still the long throw that goes through.
    assert_eq!(s.content.ex_of(idx(&s, "fan_of_knives")), Some(idx(&s, "crippling_throw")));
}

#[test]
fn shadowstep_goes_out_of_a_root_and_its_key_again_steps_back() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "shadowstep");
    s.set_valor(p, 40.0);
    s.apply_cc(p, p, "root", 3.0, 0.0, 0.0, "test");
    press(&mut s, p, "utility", false);
    assert!(matches!(s.players[&p].action, Action::Aim { .. }), "aimed at the reticle");
    click(&mut s, p);
    let out = s.players[&p].pos;
    assert!(out[0] > 15.0, "stepped out along the reticle: {out:?}");
    assert!(!s.has_cc(p, "root"), "out of the root");
    let keys = s.recast_keys(p);
    assert_eq!(keys.len(), 1, "the key armed for the way back");
    assert_eq!(keys[0].1, idx(&s, "shadowstep_back"));
    let cd = s.cooldown(p, idx(&s, "shadowstep"));
    // The key again: straight back, no aim, for nothing.
    run_pinned(&mut s, 1.0, &[]);
    press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back where it began: {:?}", s.players[&p].pos);
    assert!(s.recast_keys(p).is_empty(), "the way back used");
    assert_eq!(s.cooldown(p, idx(&s, "shadowstep")), cd, "no more cooldown spent");
    assert!((s.players[&p].valor - 40.0).abs() < 0.01, "nor Valor");
}

#[test]
fn blinding_step_blinds_where_you_come_out_and_steps_back_too() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [10.0, 0.0, 0.0]);
    let pins = [(m, [10.0, 0.0, 0.0])];
    cast(&mut s, p, "shadowstep", true);
    assert!(s.players[&p].pos[0] > 6.0, "stepped in on it: {:?}", s.players[&p].pos);
    assert!(s.has_cc(m, "incapacitate"), "blinded where she came out");
    assert!((s.players[&p].valor - 75.0).abs() < 0.01, "a segment of Valor");
    run_pinned(&mut s, 0.5, &pins);
    press(&mut s, p, "utility", false);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "and back out: {:?}", s.players[&p].pos);
    assert!((s.players[&p].valor - 75.0).abs() < 0.01, "the way back free");
}

#[test]
fn smoke_bomb_hides_you_and_friends_in_it_slows_foes_and_hides_you_again_after_a_blow() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [2.0, 2.0, 0.0], 0.0);
    let hunter = grunt(&mut s, [4.0, 0.0, 0.0]);
    let far = grunt(&mut s, [12.0, 0.0, 0.0]);
    s.engage(hunter, Foe::Hero(p));
    assert_eq!(s.mobs[&hunter].state.foe(), Some(Foe::Hero(p)), "after her");
    let ev = cast(&mut s, p, "smoke_bomb", false);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 1856, .. })), "its puff of smoke");
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 7272, .. })), "its cloud on the ground");
    assert!(s.has_cc(p, "veil"), "hidden at once");
    assert_ne!(s.mobs[&hunter].state.foe(), Some(Foe::Hero(p)), "the creature after her lost her");
    assert!(s.has_cc(hunter, "snare"), "a foe in it slowed");
    assert!(!s.has_cc(far, "snare"), "not one outside it");
    run_pinned(&mut s, 1.1, &[(far, [12.0, 0.0, 0.0])]);
    assert!(s.has_cc(f, "veil"), "the friend in it hidden too");
    assert!(s.has_cc(hunter, "snare"), "still slowed while in it");
    // A blow shows her; the cloud hides her again a moment later.
    s.input(p, Input::Attack);
    s.step(DT);
    s.input(p, Input::AttackUp);
    s.step(DT);
    assert!(!s.has_cc(p, "veil"), "a blow brings her out");
    run_pinned(&mut s, 1.1, &[(far, [12.0, 0.0, 0.0])]);
    assert!(s.has_cc(p, "veil"), "hidden again in the smoke");
    // Once it clears, nothing more.
    run_pinned(&mut s, 4.0, &[(far, [12.0, 0.0, 0.0])]);
    assert!(!s.has_cc(p, "veil") && !s.has_cc(f, "veil"), "the smoke gone, so is the veil");
}

#[test]
fn a_boss_sees_through_smoke_and_vanish_and_keeps_fighting() {
    // A veil drops an ordinary creature's attention, but a champion in its fight keeps it: hiding
    // never resets a boss (nor empties its fight).
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let def = s.content.mobs.iter().position(|m| m.boss && !m.inert()).expect("a boss");
    let boss = s.spawn_mob(def, [5.0, 0.0, 0.0], None, false);
    let hunter = grunt(&mut s, [0.0, 5.0, 0.0]);
    s.engage(boss, Foe::Hero(p));
    s.engage(hunter, Foe::Hero(p));
    cast(&mut s, p, "smoke_bomb", false);
    assert!(s.has_cc(p, "veil"), "hidden");
    assert_ne!(s.mobs[&hunter].state.foe(), Some(Foe::Hero(p)), "the grunt lost her");
    assert_eq!(s.mobs[&boss].state.foe(), Some(Foe::Hero(p)), "the boss did not");
    // Nor does it lose her when it looks for whom to fight.
    for _ in 0..20 {
        s.step(DT);
    }
    assert!(s.mobs[&boss].state.foe() == Some(Foe::Hero(p)), "still after her: {:?}", s.mobs[&boss].state);
    assert!(!matches!(s.mobs[&boss].state, MobState::Idle { .. } | MobState::Return), "no reset");
}

#[test]
fn an_old_save_with_the_rogues_old_cards_loads_with_their_successors() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.owned = ["deadly_throw", "backstab", "shadowstep", "evasion"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "deadly_throw"), ("close", "backstab"), ("utility", "shadowstep"), ("defense", "evasion")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    for k in ["fan_of_knives", "backstab", "shadowstep", "smoke_bomb"] {
        assert!(h.owned.contains(k), "owns {k}: {:?}", h.owned);
    }
    assert!(!h.owned.contains("deadly_throw") && !h.owned.contains("evasion"));
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("fan_of_knives"));
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("smoke_bomb"));
    assert_eq!(h.loadout.get("utility").map(String::as_str), Some("shadowstep"));
}

// --- Priest -------------------------------------------------------------------------------------

fn channelling(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Channel { .. })
}

fn halve(s: &mut Survival, p: Id) -> u32 {
    let h = s.players.get_mut(&p).unwrap();
    h.health = h.max_health / 2;
    h.health
}

#[test]
fn penance_burns_the_foes_in_its_lance_heals_the_friend_it_crosses_and_sweeps_as_she_turns() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [6.0, -0.5, 0.0], 0.0);
    let near = grunt(&mut s, [10.0, 0.0, 0.0]);
    let far = grunt(&mut s, [20.0, 0.5, 0.0]);
    let beyond = grunt(&mut s, [26.0, 0.0, 0.0]);
    let north = grunt(&mut s, [0.0, 10.0, 0.0]);
    let pins: Vec<(Id, Vec3)> = [near, far, beyond, north].iter().map(|m| (*m, s.mobs[m].pos)).collect();
    let hf = halve(&mut s, f);
    let mut ev = cast(&mut s, p, "penance", false);
    assert!(channelling(&s, p), "held");
    assert!(s.auras_of(p).contains(&10060), "its glow on her while she holds it");
    ev.extend(run_pinned(&mut s, 0.9, &pins));
    assert!(hits(&ev, p, near) >= 3, "the near one burns each tick: {}", hits(&ev, p, near));
    assert!(hits(&ev, p, far) >= 3, "and the far one in the same lance: {}", hits(&ev, p, far));
    assert_eq!(hits(&ev, p, beyond), 0, "not past its 22 yd");
    assert_eq!(hits(&ev, p, north), 0, "nor off to the side, yet");
    assert!(s.players[&f].health > hf, "the friend it crosses mended");
    assert_eq!(hits(&ev, p, f), 0, "and never burned");
    assert!(
        ev.iter()
            .any(|e| matches!(e, Event::Missile { caster, spell: 6685, target: Target::Unit(t) } if *caster == p && *t == near)),
        "its light streams to the first foe in it"
    );
    // She turns north: the lance sweeps with her.
    s.move_player(p, ORIGIN, std::f32::consts::FRAC_PI_2, Moving::default());
    let ev = run_pinned(&mut s, 0.5, &pins);
    assert!(hits(&ev, p, north) >= 1, "now the north one");
    assert_eq!(hits(&ev, p, near), 0, "no longer the ones it left");
    run_pinned(&mut s, 0.5, &pins);
    assert!(!channelling(&s, p), "over after 1.5 s");
}

#[test]
fn mind_blast_is_penances_ex_a_dark_star_that_lands_late_and_terrifies() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = grunt(&mut s, [12.0, 0.0, 0.0]);
    let pins = [(m, [12.0, 0.0, 0.0])];
    let before = hp(&s, m);
    let ev = cast(&mut s, p, "penance", true);
    assert!(!channelling(&s, p), "not the beam");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Mark { radius, tele: Some(_), .. } if (*radius - 3.5).abs() < 0.01)),
        "its ring, for everyone"
    );
    assert!((s.players[&p].valor - 75.0).abs() < 0.01, "a segment of Valor");
    run_pinned(&mut s, 0.3, &pins);
    assert_eq!(hp(&s, m), before, "not yet");
    run_pinned(&mut s, 0.5, &pins);
    assert!(hp(&s, m) < before, "it landed");
    assert!(s.has_cc(m, "panic"), "and terrifies");
}

#[test]
fn power_word_barrier_wards_and_steadies_whoever_stands_under_it() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let away = hero(&mut s, "Cara", [12.0, 0.0, 0.0], 0.0);
    let m = grunt(&mut s, [4.0, 2.0, 0.0]);
    let pins = [(m, [4.0, 2.0, 0.0])];
    let ev = cast(&mut s, p, "power_word_barrier", false);
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 20116, .. })), "its light on the ground");
    assert!(s.has_cc(p, "hallowed") && s.unstaggerable(p), "her own at once");
    assert!((s.taken_share(p) - 0.75).abs() < 0.01, "a quarter less: {}", s.taken_share(p));
    run_pinned(&mut s, 1.1, &pins);
    assert!(s.has_cc(f, "hallowed") && s.has_cc(f, "steadfast"), "the friend under it, both");
    assert!((s.taken_share(f) - 0.75).abs() < 0.01);
    assert!(!s.has_cc(away, "hallowed") && !s.has_cc(away, "steadfast"), "not one outside it");
    assert!(!s.has_cc(m, "steadfast") && !s.has_cc(m, "hallowed"), "never a foe in it");
    // Out from under it, it wears off.
    s.move_player(f, [12.0, 2.0, 0.0], 0.0, Moving::default());
    run_pinned(&mut s, 1.6, &pins);
    assert!(!s.has_cc(f, "hallowed") && !s.has_cc(f, "steadfast"), "gone once he left it");
    assert!(s.has_cc(p, "hallowed"), "still on her inside it");
    run_pinned(&mut s, 4.0, &pins);
    assert!(!s.has_cc(p, "hallowed"), "the dome gone, so is the ward");
}

#[test]
fn power_word_shield_is_the_barriers_ex_and_goes_on_the_friend_under_the_reticle_else_on_you() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], 0.0);
    let ev = cast(&mut s, p, "power_word_barrier", true);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 17, target: Target::Unit(t) } if *caster == p && *t == f)),
        "its look on him"
    );
    assert!(s.has_cc(f, "shield") && s.has_cc(f, "haste"), "shielded and quick");
    assert!(!s.has_cc(p, "shield") && !s.has_cc(p, "haste"), "not her");
    let v = s.players[&p].valor;
    assert!(v > 74.9 && v < 90.0, "a segment of Valor spent (a little back for helping): {v}");
    assert!(s.grounds.is_empty() && s.burnings.is_empty(), "no dome with it");

    // Alone, it's her own.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    cast(&mut s, p, "power_word_barrier", true);
    assert!(s.has_cc(p, "shield") && s.has_cc(p, "haste"), "on herself");
}

#[test]
fn an_old_save_with_the_priests_old_cards_loads_with_their_successors() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.owned = ["mind_blast", "holy_nova", "leap_of_faith", "power_word_shield"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "mind_blast"), ("close", "holy_nova"), ("defense", "power_word_shield")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    for k in ["penance", "holy_nova", "leap_of_faith", "power_word_barrier"] {
        assert!(h.owned.contains(k), "owns {k}: {:?}", h.owned);
    }
    assert!(
        !h.owned.contains("mind_blast") && !h.owned.contains("power_word_shield"),
        "not the EX versions: they come with their cards"
    );
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("penance"));
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("power_word_barrier"));
}

// --- Shaman -------------------------------------------------------------------------------------

#[test]
fn lava_burst_is_lobbed_lands_late_where_its_ring_shows_and_leaves_the_ground_burning() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // (Those it's aimed at stand past a grunt's 14 yd notice, so none walks in before it's thrown.)
    let by_path = grunt(&mut s, [8.0, 1.0, 0.0]);
    let there = grunt(&mut s, [18.0, 0.0, 0.0]);
    let beside = grunt(&mut s, [18.0, 2.0, 0.0]);
    let outside = grunt(&mut s, [18.0, 5.0, 0.0]);
    let pins: Vec<(Id, Vec3)> = [by_path, there, beside, outside].iter().map(|m| (*m, s.mobs[m].pos)).collect();
    let (hp_path, hp_there, hp_beside, hp_out) = (hp(&s, by_path), hp(&s, there), hp(&s, beside), hp(&s, outside));
    // Aimed at the ground 18 yd ahead: about 0.6 s in the air.
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(18.0));
    let mut ev = cast(&mut s, p, "lava_burst", false);
    let ring = ev
        .iter()
        .find_map(|e| match e {
            Event::Mark { pos, radius, tele: Some(t), .. } if t.at.is_some() && (*radius - 3.0).abs() < 0.01 => Some(*pos),
            _ => None,
        })
        .expect("where it will land, ringed for everyone");
    assert!(dist2d(ring, [18.0, 0.0, 0.0]) < 1.5, "at the spot aimed at: {ring:?}");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 9483, target: Target::Point(_) } if *caster == p)),
        "the rock hurled there"
    );
    ev.extend(run_pinned(&mut s, 0.2, &pins));
    assert_eq!(hp(&s, there), hp_there, "still in the air");
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert!(hp(&s, there) < hp_there && hp(&s, beside) < hp_beside, "it burst where it landed, 3 yd wide");
    assert_eq!(hp(&s, outside), hp_out, "not 5 yd off");
    assert_eq!(hp(&s, by_path), hp_path, "nor the one it flew past");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { spell: 5162, target: Target::Point(at), .. } if dist2d(*at, ring) < 0.5)),
        "its fire bursting there"
    );
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 13812, .. })), "the ground glowing hot");
    // The ground burns on for 3 s.
    let landed = hp(&s, there);
    run_pinned(&mut s, 1.1, &pins);
    assert!(hp(&s, there) < landed, "burning in it");
    run_pinned(&mut s, 2.5, &pins);
    let cooled = hp(&s, there);
    run_pinned(&mut s, 1.5, &pins);
    assert_eq!(hp(&s, there), cooled, "cooled after 3 s");
}

#[test]
fn an_old_save_with_the_shamans_lightning_bolt_loads_with_lava_burst() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.owned = ["lightning_bolt", "thunderstorm"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "lightning_bolt"), ("close", "thunderstorm")]
        .into_iter()
        .map(|(a, b)| (a.to_string(), b.to_string()))
        .collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.contains("lava_burst") && h.owned.contains("thunderstorm"), "{:?}", h.owned);
    assert!(!h.owned.contains("lightning_bolt"));
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("lava_burst"));
    assert_eq!(h.loadout.get("close").map(String::as_str), Some("thunderstorm"));
}
