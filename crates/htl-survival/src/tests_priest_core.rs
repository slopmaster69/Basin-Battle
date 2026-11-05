//! The Priest's core (claude/priest-kit-draft.md): staffs whose right click is an ability and
//! that never guard (the holy staff's Smite, Heal and Flash Heal; the shadow staff's Pain and
//! Drain), Power Word: Shield as the key 2 card that parries a blow for the friend it lands on
//! just in time, and Grace, which the Priest's perfects raise and the Heal spends.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const EAST: f32 = 0.0;
const WEST: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    // (No mending by itself: what's healed is the spells'.)
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
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

/// A hero holding `staff` (taken from the rack), back where they stood.
fn armed(s: &mut Survival, name: &str, pos: Vec3, facing: f32, staff: &str) -> Id {
    let p = hero(s, name, pos, facing);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, staff).unwrap();
    s.players.get_mut(&p).unwrap().pos = pos;
    p
}

/// A creature standing idle where it's put (a dummy stands still whatever strikes it).
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.ready_at = 1e9;
    m.pos = pos;
    id
}

/// A wolf `at`, its first bite on its way to `on`.
fn wolf_on(s: &mut Survival, on: Id, at: Vec3) -> Id {
    let def = s.content.mob("wolf").unwrap();
    let k = s.spawn_mob(def, at, None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.engage(k, Foe::Hero(on));
    k
}

/// Run until the wolf's bite is `before` seconds off landing.
fn until_bite(s: &mut Survival, k: Id, before: f32) -> Vec<Event> {
    let mut ev = Vec::new();
    for _ in 0..1000 {
        if let MobState::Windup { t, .. } = s.mobs[&k].state {
            if t + before >= s.attack_now(k).windup - 1.0e-4 {
                return ev;
            }
        }
        ev.extend(s.step(DT));
    }
    panic!("no bite");
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

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
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

fn perfect(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

fn outcomes(events: &[Event], on: Id) -> Vec<Outcome> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), outcome, .. } if *v == on => Some(*outcome),
            _ => None,
        })
        .collect()
}

fn hurt(s: &mut Survival, p: Id, health: u32) {
    s.players.get_mut(&p).unwrap().health = health;
}

fn health(s: &Survival, p: Id) -> u32 {
    s.players[&p].health
}

/// What a hero's shield has left to absorb (0: none).
fn shield_left(s: &Survival, p: Id) -> f32 {
    s.cc_of(p).iter().filter(|c| c.kind == "shield" && c.until > s.clock).map(|c| c.left).fold(0.0, f32::max)
}

/// Power Word: Shield on its key, cast now along the reticle.
fn shield(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Ability(key_of("defense"), false));
    let mut ev = s.step(0.0);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
        ev.extend(s.step(0.0));
    }
    ev
}

fn set_grace(s: &mut Survival, p: Id, level: u32) {
    let now = s.clock;
    let h = s.players.get_mut(&p).unwrap();
    h.priest.grace = level;
    h.priest.grace_at = now;
}

// --- the holy staff -------------------------------------------------------------------------

#[test]
fn the_holy_staffs_right_click_heals_an_ally_and_never_raises_a_guard() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    let w = s.weapon_of(&s.players[&p]).clone();
    assert!(w.has_alt() && w.alt == "acolyte_flash_heal", "its right click is Flash Heal");
    s.input(p, Input::Guard);
    let mut ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![6685], "a bolt of light leaves the staff");
    assert!(!matches!(s.players[&p].action, Action::Guard), "no guard");
    ev.extend(run(&mut s, 0.5));
    assert_eq!(health(&s, f), 107, "Bren healed 3.5% of 200");
    assert!(missiles(&ev, p).contains(&2061), "Flash Heal's look on him");
    s.input(p, Input::GuardUp);
    let _ = s.step(0.0);

    // A wolf's bite on the staff's holder, right mouse held as it lands: nothing stops it.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, WEST, "acolyte_staff");
    let k = wolf_on(&mut s, p, [2.0, 0.0, 0.0]);
    let full = health(&s, p);
    until_bite(&mut s, k, 0.05);
    s.input(p, Input::Guard);
    let ev = run(&mut s, 0.3);
    assert!(!matches!(s.players[&p].action, Action::Guard), "still no guard");
    let out = outcomes(&ev, p);
    assert!(out.contains(&Outcome::Hit), "bitten: {out:?}");
    assert!(!out.iter().any(|o| matches!(o, Outcome::Parried | Outcome::Blocked)), "not parried, not blocked: {out:?}");
    assert!(health(&s, p) < full);
}

#[test]
fn smite_is_the_holy_staffs_click() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let d = mob(&mut s, "training_dummy", [12.0, 0.0, 0.0]);
    let hp = s.mobs[&d].health;
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![585], "Smite's look");
    run(&mut s, 1.0);
    assert_eq!(hp - s.mobs[&d].health, 9, "9 damage");
}

#[test]
fn heals_cast_heals_the_ally_under_the_reticle_slowly_and_a_heavy_blow_breaks_it() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    s.input(p, Input::Heavy);
    let _ = s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Charge { .. }), "casting");
    assert!(s.speed_for(p) < s.content.player.run_speed * 0.8, "slowed while casting: {}", s.speed_for(p));
    run(&mut s, 1.8);
    assert_eq!(health(&s, f), 100, "not before its 2 sec");
    let ev = run(&mut s, 0.3);
    assert_eq!(health(&s, f), 130, "healed 15% of 200");
    assert!(missiles(&ev, p).contains(&2060), "Greater Heal's landing");
    assert!(!matches!(s.players[&p].action, Action::Charge { .. }));
    s.input(p, Input::AttackUp);

    // Alone, it heals you.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    hurt(&mut s, p, 100);
    s.input(p, Input::Heavy);
    run(&mut s, 2.1);
    assert_eq!(health(&s, p), 130, "on herself");

    // A heavy blow while casting breaks it.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    let k = mob(&mut s, "grunt", [-2.0, 0.0, 0.0]);
    s.input(p, Input::Heavy);
    run(&mut s, 1.0);
    s.strike_player(k, p, 5, [-2.0, 0.0, 0.0], true, true, true, Blow::Heavy);
    assert!(!matches!(s.players[&p].action, Action::Charge { .. }), "broken");
    run(&mut s, 1.5);
    assert_eq!(health(&s, f), 100, "no heal");
}

#[test]
fn flash_heal_passes_through_foes_to_the_first_ally_and_goes_about_one_and_a_half_a_sec() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let d = mob(&mut s, "training_dummy", [5.0, 0.0, 0.0]);
    let f = hero(&mut s, "Bren", [12.0, 0.0, 0.0], WEST);
    let g = hero(&mut s, "Cara", [16.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 100);
    hurt(&mut s, g, 100);
    let hp = s.mobs[&d].health;
    s.input(p, Input::Guard);
    let _ = s.step(0.0);
    s.input(p, Input::GuardUp);
    // A second press at once: too soon.
    run(&mut s, 0.4);
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    assert!(missiles(&ev, p).is_empty(), "one at a time");
    s.input(p, Input::GuardUp);
    run(&mut s, 0.4);
    assert_eq!(s.mobs[&d].health, hp, "through the dummy, untouched");
    assert_eq!(health(&s, f), 107, "the first ally healed");
    assert_eq!(health(&s, g), 100, "and only him");
    s.input(p, Input::Guard);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![6685], "the next, 0.8 sec after the first");
}

// --- the shadow staff -------------------------------------------------------------------------

#[test]
fn the_shadow_staffs_bolt_hangs_pain_and_its_held_right_click_drains_the_foe_into_you() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    let hp = s.mobs[&d].health;
    s.input(p, Input::Attack);
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p), vec![686], "Shadow Bolt's look");
    run(&mut s, 0.5);
    let struck = hp - s.mobs[&d].health;
    assert!(struck >= 7, "the bolt: {struck}");
    assert!(s.auras_of(d).contains(&589), "Pain on it: {:?}", s.auras_of(d));
    run(&mut s, 3.0);
    let ticked = hp - s.mobs[&d].health - struck;
    assert!((4..=8).contains(&ticked), "Pain ticks 2 a sec: {ticked}");
    run(&mut s, 2.0);
    assert!(!s.auras_of(d).contains(&589), "gone after 4 sec");

    // Drain, held.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let d = mob(&mut s, "training_dummy", [10.0, 0.0, 0.0]);
    hurt(&mut s, p, 100);
    let hp = s.mobs[&d].health;
    s.input(p, Input::Guard);
    let _ = s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "the cord, held");
    run(&mut s, 2.05);
    let drawn = hp - s.mobs[&d].health;
    assert!((7..=9).contains(&drawn), "4 a sec: {drawn}");
    assert_eq!(health(&s, p), 100 + drawn, "healing her as much");
    s.input(p, Input::GuardUp);
    let _ = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "let go, it ends");
    let after = s.mobs[&d].health;
    run(&mut s, 1.0);
    assert_eq!(s.mobs[&d].health, after, "nothing more");
    // Out of its 15 yd: nothing to drain.
    let far = mob(&mut s, "training_dummy", [0.0, 0.0, 0.0]);
    s.mobs.get_mut(&d).unwrap().pos = [30.0, 0.0, 0.0];
    s.mobs.get_mut(&far).unwrap().pos = [-30.0, 0.0, 0.0];
    s.input(p, Input::Guard);
    let _ = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "too far");
}

#[test]
fn the_priests_staffs_are_on_the_rack() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let rack: Vec<String> = s.rack_view(p).into_iter().map(|(k, _, _, _, _)| k).collect();
    for staff in ["acolyte_staff", "shadowcaller_staff"] {
        assert!(rack.contains(&staff.to_string()), "{staff} on the rack");
    }
}

// --- Power Word: Shield ------------------------------------------------------------------------

/// A Priest with the Shield on key 2, 8 yd west of Bren (both facing each other along the
/// line), and a wolf 2 yd east of Bren, its bite on its way to him.
fn shield_set_up(s: &mut Survival) -> (Id, Id, Id) {
    let p = hero(s, "Ayla", [-8.0, 0.0, 0.0], EAST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let f = hero(s, "Bren", ORIGIN, EAST);
    let k = wolf_on(s, f, [2.0, 0.0, 0.0]);
    (p, f, k)
}

#[test]
fn power_word_shield_is_key_2_and_absorbs_for_the_ally_under_the_reticle() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    let sh = idx(&s, "power_word_shield");
    assert_eq!(s.content.ex_of(sh), Some(idx(&s, "power_word_barrier")), "the Barrier is its EX");
    let ev = shield(&mut s, p);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 17, target: Target::Unit(t) } if *caster == p && *t == f)),
        "its bubble on him"
    );
    assert!((shield_left(&s, f) - 30.0).abs() < 0.01, "30 to absorb: {}", shield_left(&s, f));
    assert_eq!(shield_left(&s, p), 0.0, "not her");
    assert_eq!(s.cooldown(p, sh).charges, 1, "one of its 2 charges left");
    run(&mut s, 0.5);
    let ev = shield(&mut s, p);
    assert!(missiles(&ev, p).contains(&17), "the second at once");
    assert_eq!(s.cooldown(p, sh).charges, 0);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    let full = health(&s, f);
    s.strike_player(k, f, 20, [10.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(health(&s, f), full, "absorbed");
    assert!((shield_left(&s, f) - 10.0).abs() < 1.01, "10 left: {}", shield_left(&s, f));
    // A blow long after, unshielded and with nothing to time: it lands.
    run(&mut s, 6.5);
    s.strike_player(k, f, 20, [10.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert!(health(&s, f) < full, "6 sec on, gone");
    // Its EX, the Barrier: the dome.
    s.set_valor(p, 100.0);
    s.players.get_mut(&p).unwrap().cooldowns.clear();
    run(&mut s, 0.5);
    s.input(p, Input::Ability(key_of("defense"), true));
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 20116, .. })), "the dome's light");
}

#[test]
fn a_shield_on_an_ally_as_the_wolfs_bite_lands_parries_it_for_him_and_pays_the_priest() {
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.05);
    let valor = s.players[&p].valor;
    ev.extend(shield(&mut s, p));
    assert!(shield_left(&s, f) > 29.0, "shielded");
    ev.extend(run(&mut s, 0.2));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried for him: {:?}", outcomes(&ev, f));
    assert_eq!(health(&s, f), full, "nothing landed");
    assert!(shield_left(&s, f) > 29.0, "the shield stays: {}", shield_left(&s, f));
    assert!(s.mobs[&k].stagger_until > s.clock, "the wolf reels");
    assert!(s.mobs[&k].riposte, "open to his riposte");
    assert!(perfect(&ev, p), "the Priest's perfect");
    assert!(!perfect(&ev, f), "not his");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor: {} from {valor}", s.players[&p].valor);
    let kit = s.content.checks.perfect_kit;
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: x, .. } if *id == f && *x == kit)), "the burst on him, for everyone");
    assert_eq!(s.grace_of(p), 1, "Grace");
}

#[test]
fn a_shield_too_early_or_too_late_only_absorbs() {
    // Early: 0.3 sec before the bite.
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let mut ev = until_bite(&mut s, k, 0.3);
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.5));
    let out = outcomes(&ev, f);
    assert!(!out.contains(&Outcome::Parried), "not parried: {out:?}");
    assert!(!perfect(&ev, p), "no perfect");
    assert!(shield_left(&s, f) < 29.0 && shield_left(&s, f) > 0.0, "the bite absorbed: {}", shield_left(&s, f));
    assert_eq!(s.grace_of(p), 0);

    // Late: the bite landed 0.12 sec before.
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.0);
    ev.extend(run(&mut s, 0.12));
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.2));
    assert!(health(&s, f) < full, "bitten");
    assert!(!outcomes(&ev, f).contains(&Outcome::Parried));
    assert!(!perfect(&ev, p), "no perfect");
}

#[test]
fn a_shield_on_yourself_never_parries() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, WEST);
    s.slot_ability(p, "defense", "power_word_shield").unwrap();
    let k = wolf_on(&mut s, p, [2.0, 0.0, 0.0]);
    let full = health(&s, p);
    let mut ev = until_bite(&mut s, k, 0.05);
    ev.extend(shield(&mut s, p));
    assert!(shield_left(&s, p) > 29.0, "on herself");
    ev.extend(run(&mut s, 0.3));
    assert!(!outcomes(&ev, p).contains(&Outcome::Parried), "not parried");
    assert!(!perfect(&ev, p), "no perfect");
    assert_eq!(health(&s, p), full, "only absorbed");
    assert!(shield_left(&s, p) < 29.0);
}

#[test]
fn the_shield_is_judged_fairly_with_a_round_trip() {
    // The Priest is 0.2 sec away: their press arriving 0.25 sec before the bite lands was made
    // as it landed on their screen (a lag-free Priest that early only absorbs: see above).
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    s.set_rtt(p, 0.2);
    let mut ev = until_bite(&mut s, k, 0.25);
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.4));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried: {:?}", outcomes(&ev, f));
    assert!(perfect(&ev, p));

    // Bren is 0.18 sec away: the bite waits on his round trip, and a shield arriving meanwhile
    // (0.1 sec after it landed on the server) is in time.
    let mut s = world();
    let (p, f, k) = shield_set_up(&mut s);
    s.set_rtt(f, 0.18);
    let full = health(&s, f);
    let mut ev = until_bite(&mut s, k, 0.0);
    ev.extend(s.step(DT));
    ev.extend(run(&mut s, 0.1));
    assert_eq!(health(&s, f), full, "still waiting on him");
    ev.extend(shield(&mut s, p));
    ev.extend(run(&mut s, 0.3));
    assert!(outcomes(&ev, f).contains(&Outcome::Parried), "parried: {:?}", outcomes(&ev, f));
    assert!(perfect(&ev, p));
    assert_eq!(health(&s, f), full, "nothing landed");
}

// --- Grace -------------------------------------------------------------------------------------

#[test]
fn grace_rises_on_a_priests_perfect_falls_on_a_blow_and_fades_after_10_sec() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    // Any Priest spell's perfect raises it, wherever it was pushed (another branch's Penance).
    s.events.push(Event::Perfect { player: p, ability: idx(&s, "penance") });
    let _ = s.step(DT);
    assert_eq!(s.grace_of(p), 1);
    let looks = s.content.grace.auras.clone();
    assert!(s.auras_of(p).contains(&looks[0]) && !s.auras_of(p).contains(&looks[1]), "one look for I");
    s.events.push(Event::Perfect { player: p, ability: idx(&s, "power_word_shield") });
    s.events.push(Event::Perfect { player: p, ability: idx(&s, "power_word_barrier") });
    let _ = s.step(DT);
    assert_eq!(s.grace_of(p), 3, "an EX version's counts as its card's");
    assert!(looks.iter().all(|a| s.auras_of(p).contains(a)), "all three looks at III");
    s.events.push(Event::Perfect { player: p, ability: idx(&s, "penance") });
    let _ = s.step(DT);
    assert_eq!(s.grace_of(p), 3, "III at most");
    // Not another class's perfect.
    s.events.push(Event::Perfect { player: p, ability: idx(&s, "fireball") });
    let _ = s.step(DT);
    assert_eq!(s.grace_of(p), 3);
    // A blow that hurts: a level lost.
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.strike_player(k, p, 10, [2.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(s.grace_of(p), 2, "hurt, II");
    // A blow a shield takes whole hurts nothing.
    s.apply_cc(p, p, "shield", 6.0, 30.0, 0.0, "test");
    s.strike_player(k, p, 10, [2.0, 0.0, 0.0], true, false, true, Blow::Light);
    assert_eq!(s.grace_of(p), 2, "absorbed, still II");
    // Fades 10 sec after the last rise.
    set_grace(&mut s, p, 2);
    run(&mut s, 9.8);
    assert_eq!(s.grace_of(p), 2);
    run(&mut s, 0.3);
    assert_eq!(s.grace_of(p), 0, "faded");
    assert!(!looks.iter().any(|a| s.auras_of(p).contains(a)), "and its looks");
}

#[test]
fn the_heal_spends_grace_faster_with_each_level_and_from_ii_overheals_into_a_shield() {
    // I: 30% quicker (1.4 sec), no overhealth.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 190);
    set_grace(&mut s, p, 1);
    s.input(p, Input::Heavy);
    let _ = s.step(0.0);
    assert_eq!(s.grace_of(p), 0, "spent as it begins");
    run(&mut s, 1.3);
    assert_eq!(health(&s, f), 190, "not yet");
    run(&mut s, 0.15);
    assert_eq!(health(&s, f), 200, "healed at 1.4 sec");
    assert_eq!(shield_left(&s, f), 0.0, "no overhealth at I");

    // II: 60% quicker (0.8 sec), healing past full stays on as a shield.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 190);
    set_grace(&mut s, p, 2);
    s.input(p, Input::Heavy);
    run(&mut s, 0.7);
    assert_eq!(health(&s, f), 190, "not yet");
    run(&mut s, 0.15);
    assert_eq!(health(&s, f), 200, "healed at 0.8 sec");
    assert!((shield_left(&s, f) - 20.0).abs() < 0.01, "the 20 past full, a shield: {}", shield_left(&s, f));
    run(&mut s, 8.1);
    assert_eq!(shield_left(&s, f), 0.0, "for 8 sec");

    // III: at once, and twice the overhealth, with its column of light.
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    hurt(&mut s, f, 190);
    set_grace(&mut s, p, 3);
    s.input(p, Input::Heavy);
    let ev = s.step(0.0);
    assert_eq!(health(&s, f), 200, "instant");
    assert!((shield_left(&s, f) - 40.0).abs() < 0.01, "twice the 20: {}", shield_left(&s, f));
    assert!(missiles(&ev, p).contains(&s.content.grace.top_spell), "the column of light");
    s.input(p, Input::AttackUp);
    let _ = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Charge { .. }));
}

// --- saves -------------------------------------------------------------------------------------

#[test]
fn an_old_save_with_the_barrier_on_key_2_loads_with_the_shield_there_and_the_barrier_its_ex() {
    let mut s = world();
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, EAST);
    let mut save = s.leave(p).unwrap();
    save.owned = ["penance", "holy_nova", "power_word_barrier"].into_iter().map(String::from).collect();
    save.loadout = [("ranged", "penance"), ("defense", "power_word_barrier")].into_iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.contains("power_word_shield"), "owns the Shield: {:?}", h.owned);
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("power_word_shield"));
    // The key works: the Shield, and with Ctrl the Barrier.
    s.move_player(q, ORIGIN, EAST, Moving::default());
    let ev = shield(&mut s, q);
    assert!(missiles(&ev, q).contains(&17), "the Shield");
    s.set_valor(q, 100.0);
    run(&mut s, 0.5);
    s.input(q, Input::Ability(key_of("defense"), true));
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 20116, .. })), "the Barrier");
}
