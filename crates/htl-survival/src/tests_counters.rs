//! The Mage's counter checks on Frost's defenses. Blink just as a blow lands and it misses, Blink
//! coming back twice as fast; Ice Block broken out of just as a blow lands on it freezes whoever
//! struck. Early or late, each is the plain spell.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const WEST: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// A Mage facing west, a wolf 2 yd east of them, its first bite on its way.
fn set_up(s: &mut Survival, slot: &str, ability: &str) -> (Id, Id) {
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, ORIGIN, WEST, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&p).unwrap().cd_off = false;
    s.slot_ability(p, slot, ability).unwrap();
    let def = s.content.mob("wolf").unwrap();
    let k = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.engage(k, Foe::Hero(p));
    (p, k)
}

/// Run until the wolf's bite is `before` seconds off landing.
fn until_bite(s: &mut Survival, k: Id, before: f32) {
    for _ in 0..1000 {
        if let MobState::Windup { t, .. } = s.mobs[&k].state {
            if t + before >= s.attack_now(k).windup - 1.0e-4 {
                return;
            }
        }
        s.step(DT);
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

/// Blink to the reticle (west, away from the wolf).
fn blink(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Ability(key_of("utility"), false));
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0)
}

fn perfect(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

/// Seconds till Blink is back.
fn blink_back_in(s: &Survival, p: Id) -> f32 {
    let b = s.content.ability("blink").unwrap();
    (s.cooldown(p, b).next_at - s.clock).max(0.0)
}

#[test]
fn blink_just_before_the_bite_slips_it_and_comes_back_twice_as_fast() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "utility", "blink");
    until_bite(&mut s, k, 0.05);
    let valor = s.players[&p].valor;
    let mut ev = blink(&mut s, p);
    assert!(s.players[&p].pos[0] < -10.0, "blinked west: {:?}", s.players[&p].pos);
    ev.extend(run(&mut s, 0.2));
    assert!(perfect(&ev, p), "slipped: a perfect");
    assert!(s.players[&p].valor >= valor + s.content.valor.counter, "a counter's Valor");
    let cd = s.content.abilities[s.content.ability("blink").unwrap()].cooldown;
    let left = blink_back_in(&s, p);
    assert!(left <= cd * 0.5 && left > cd * 0.5 - 0.5, "half the cooldown: {left}");
    let _ = k;
}

#[test]
fn blink_long_before_the_bite_is_a_plain_blink() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "utility", "blink");
    let windup = s.attack_now(k).windup;
    assert!(windup > 0.3, "the wolf's bite winds up long enough: {windup}");
    until_bite(&mut s, k, 0.3);
    let mut ev = blink(&mut s, p);
    ev.extend(run(&mut s, 0.5));
    assert!(!perfect(&ev, p), "no perfect");
    let cd = s.content.abilities[s.content.ability("blink").unwrap()].cooldown;
    assert!(blink_back_in(&s, p) > cd - 1.0, "the whole cooldown");
}

#[test]
fn blink_judges_the_bite_on_the_mages_own_screen() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "utility", "blink");
    s.set_rtt(p, 0.18);
    // The bite lands on the server; on Chain's screen it lands a round trip on, and he blinks
    // then. The bite waits on his round trip (as a guard's does), so the blink arriving in it slips.
    until_bite(&mut s, k, 0.0);
    let mut ev = s.step(DT);
    ev.extend(run(&mut s, 0.12));
    let health = s.players[&p].health;
    ev.extend(blink(&mut s, p));
    ev.extend(run(&mut s, 0.3));
    assert!(perfect(&ev, p), "slipped");
    assert_eq!(s.players[&p].health, health, "and nothing landed");
}

/// Ice Block up with the wolf on it; run until its bite lands on the ice. The events.
fn bite_the_ice(s: &mut Survival, p: Id, k: Id) -> Vec<Event> {
    until_bite(s, k, 0.3);
    s.input(p, Input::Ability(key_of("defense"), false));
    assert!(matches!(s.players[&p].action, Action::Stance { .. }), "iced");
    let mut ev = Vec::new();
    for _ in 0..100 {
        ev.extend(s.step(DT));
        if s.stances.get(&p).is_some_and(|st| st.struck.is_some()) {
            return ev;
        }
    }
    panic!("no bite on the ice");
}

fn frozen(s: &Survival, k: Id) -> bool {
    s.cc_of(k).iter().any(|c| c.kind == "petrify" && c.until > s.clock)
}

#[test]
fn ice_block_broken_out_as_the_bite_lands_freezes_the_wolf() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "defense", "ice_block");
    let full = s.players[&p].health;
    bite_the_ice(&mut s, p, k);
    assert_eq!(s.players[&p].health, full, "the ice took it");
    s.input(p, Input::Ability(key_of("defense"), false));
    let ev = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Stance { .. }), "out");
    assert!(perfect(&ev, p), "a perfect");
    assert!(frozen(&s, k), "the wolf frozen");
}

#[test]
fn ice_block_broken_out_late_freezes_nothing() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "defense", "ice_block");
    bite_the_ice(&mut s, p, k);
    run(&mut s, 0.3);
    s.input(p, Input::Ability(key_of("defense"), false));
    let ev = s.step(0.0);
    assert!(!matches!(s.players[&p].action, Action::Stance { .. }), "out");
    assert!(!perfect(&ev, p), "no perfect");
    assert!(!frozen(&s, k), "nothing frozen");
}

#[test]
fn blink_after_the_bite_landed_is_too_late() {
    let mut s = world();
    let (p, k) = set_up(&mut s, "utility", "blink");
    until_bite(&mut s, k, 0.0);
    let full = s.players[&p].health;
    let mut ev = s.step(DT);
    ev.extend(run(&mut s, 0.12));
    ev.extend(blink(&mut s, p));
    ev.extend(run(&mut s, 0.3));
    assert!(s.players[&p].health < full, "bitten");
    assert!(!perfect(&ev, p), "no perfect");
}
