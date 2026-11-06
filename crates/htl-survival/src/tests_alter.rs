//! Alter Time: a mark that pulses once a second; the key again steps back to it with half the
//! health lost since, and on a pulse it's a perfect that throws back what's near. Spell Mirror,
//! its EX: only what reaches the Mage within the window of raising it flies back; later, it's
//! only swallowed.

use crate::content::{AbilityDef, Content};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// A rival's bolt, off the board, for the mirror.
const BOLT: &str = r#"
[[ability]]
key = "t_bolt"
name = "Bolt"
school = "warlock"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.4, 0.4]
damage = [12.0, 12.0]
spell = 686
"#;

fn world(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    #[derive(serde::Deserialize)]
    struct Extra {
        ability: Vec<AbilityDef>,
    }
    let e: Extra = toml::from_str(BOLT).expect("test abilities");
    for mut a in e.ability {
        a.bench = true;
        c.abilities.push(a);
    }
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

/// Run on until the clock reads `t`.
fn run_until(s: &mut Survival, t: f32) -> Vec<Event> {
    let left = t - s.clock;
    if left > 0.0 { run(s, left) } else { Vec::new() }
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn press(s: &mut Survival, p: Id, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of("defense"), ex));
    s.step(0.0)
}

fn perfect(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

/// Alter Time on the defense key, marked where the Mage stands; when it was set.
fn mark(s: &mut Survival, p: Id) -> f32 {
    s.slot_ability(p, "defense", "alter_time").unwrap();
    press(s, p, false);
    assert!(s.anchors.contains_key(&p), "marked");
    s.anchors[&p].since
}

#[test]
fn alter_time_steps_back_to_the_mark_with_half_the_health_lost() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let since = mark(&mut s, p);
    // Away from it, and hurt.
    s.move_player(p, [8.0, 0.0, 0.0], 0.0, Moving::default());
    let full = s.players[&p].health;
    s.players.get_mut(&p).unwrap().health = full - 40;
    // Off any pulse (half a beat in).
    run_until(&mut s, since + 1.5);
    // (Some of it may have come back by itself meanwhile: half of what's still lost comes back.)
    let before = s.players[&p].health;
    let ev = press(&mut s, p, false);
    assert!(s.players[&p].pos[0].abs() < 0.5, "back at the mark: {:?}", s.players[&p].pos);
    assert_eq!(s.players[&p].health, before + ((full - before) as f32 * 0.5).round() as u32, "half of what's lost comes back");
    assert!(s.players[&p].health >= full - 20);
    assert!(!perfect(&ev, p), "off the pulse: no perfect");
    assert!(!s.anchors.contains_key(&p), "the mark spent");
    assert!(!s.ready(p, s.content.ability("alter_time").unwrap()), "and the cooldown started");
}

#[test]
fn alter_time_on_a_pulse_is_a_perfect_that_throws_back_what_is_near() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = dummy(&mut s, [3.0, 0.0, 0.0]);
    let since = mark(&mut s, p);
    s.move_player(p, [10.0, 0.0, 0.0], 0.0, Moving::default());
    let valor = s.players[&p].valor;
    // The pulses: shown at the mark, one a second.
    let spell = s.content.abilities[s.content.ability("alter_time").unwrap()].tick_spell;
    let ev = run_until(&mut s, since + 2.0);
    let pulses = ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: k, target: Target::Point(_) } if *caster == p && *k == spell)).count();
    assert_eq!(pulses, 2, "two pulses by two seconds");
    let ev = press(&mut s, p, false);
    assert!(perfect(&ev, p), "on the pulse: a perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.perfect_valor, "its Valor");
    assert!(s.cc_log_of(p).iter().any(|l| l.contains("knockback on")), "the dummy by the mark thrown back: {:?}", s.cc_log_of(p));
    let _ = d;
}

#[test]
fn alter_time_judges_the_pulse_on_the_mages_own_screen() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_rtt(p, 0.18);
    let since = mark(&mut s, p);
    s.move_player(p, [10.0, 0.0, 0.0], 0.0, Moving::default());
    // Pressed on the pulse at 3 sec on Chain's screen, arriving a round trip later.
    run_until(&mut s, since + 3.0 + 0.18);
    let ev = press(&mut s, p, false);
    assert!(perfect(&ev, p), "still a perfect");
}

#[test]
fn alter_times_mark_fades_after_5_sec_with_four_pulses() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let since = mark(&mut s, p);
    let spell = s.content.abilities[s.content.ability("alter_time").unwrap()].tick_spell;
    let ev = run_until(&mut s, since + 5.2);
    let pulses = ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: k, target: Target::Point(_) } if *caster == p && *k == spell)).count();
    assert_eq!(pulses, 4, "a pulse each second it lasts");
    assert!(!s.anchors.contains_key(&p), "gone");
    // No circle on the ground for it, and no words.
    assert!(!ev.iter().any(|e| matches!(e, Event::Mark { .. })), "no circle drawn");
    s.move_player(p, [6.0, 0.0, 0.0], 0.0, Moving::default());
    press(&mut s, p, false);
    assert!(s.anchors.contains_key(&p), "the key again marks anew");
    assert!((s.players[&p].pos[0] - 6.0).abs() < 0.5, "and doesn't move them");
}

/// Ayla raises Spell Mirror and Bren's bolt flies at her from 10 yd. Raised `lead` seconds before
/// the bolt is about to reach her (it's checked every step: under 2 yd off, it reaches her next).
fn mirror_duel(raise_early: bool) -> (Survival, Id, Id, Vec<Event>) {
    let mut s = world(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], std::f32::consts::PI);
    s.slot_ability(a, "defense", "alter_time").unwrap();
    s.players.get_mut(&a).unwrap().valor = s.content.valor.max;
    let mut ev = Vec::new();
    if raise_early {
        ev.extend(press(&mut s, a, true));
    }
    let bolt = s.content.ability("t_bolt").unwrap();
    s.fire_with(b, bolt, 1.0, false);
    for _ in 0..200 {
        if !raise_early && s.projectiles.iter().any(|p| p.owner == b && dist2d(p.pos, ORIGIN) < 2.5) {
            ev.extend(press(&mut s, a, true));
            break;
        }
        ev.extend(s.step(DT));
    }
    ev.extend(run(&mut s, 1.0));
    (s, a, b, ev)
}

#[test]
fn spell_mirror_raised_as_the_bolt_arrives_sends_it_back() {
    let (s, a, b, ev) = mirror_duel(false);
    let full = s.players[&a].max_health;
    assert_eq!(s.players[&a].health, full, "Ayla took nothing");
    assert!(s.players[&b].health < s.players[&b].max_health, "the bolt came back at Bren");
    assert!(perfect(&ev, a), "a perfect");
}

#[test]
fn spell_mirror_raised_too_soon_only_swallows_it() {
    let (s, a, b, ev) = mirror_duel(true);
    assert_eq!(s.players[&a].health, s.players[&a].max_health, "Ayla took nothing");
    assert_eq!(s.players[&b].health, s.players[&b].max_health, "and nothing came back");
    assert!(!perfect(&ev, a), "no perfect");
}
