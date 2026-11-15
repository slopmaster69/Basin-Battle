//! The redesigned mage, warlock and druid cards (#40), each cast from the real content: the thing that makes it
//! play differently.

use std::collections::BTreeMap;

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::abilities::Stage;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules, every card owned (friendly fire on with `pvp`).
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

/// A boar (it notices no one till struck), standing still.
fn boar(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("boar").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

/// Run on with these creatures held where they are (struck, they'd come for the hero).
fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, at) in pins {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *at;
            }
        }
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

/// Own a card and put it on its key.
fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// A key pressed (`ex`: with Ctrl).
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    s.step(DT)
}

/// The click held `secs` (a tap: 0) and let go, the creatures held where they are.
fn hold(s: &mut Survival, p: Id, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    ev.extend(run_pinned(s, secs, pins));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
}

fn missile(ev: &[Event], spell: u32) -> bool {
    ev.iter().any(|e| matches!(e, Event::Missile { spell: s, .. } if *s == spell))
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

/// The damage of each of `p`'s blows that landed on `m`.
fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, .. } if *attacker == p && *v == m => Some(*damage),
            _ => None,
        })
        .collect()
}

// --- Mage: freeze, then shatter ---------------------------------------------------------------------

#[test]
fn frost_nova_makes_the_next_fireball_a_free_pyroblast_that_shatters_the_frozen() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "fireball");
    learn(&mut s, p, "cone_of_cold");
    s.set_cooldowns(p, true);
    let k = boar(&mut s, [6.0, 0.0, 0.0]);
    let pins = [(k, [6.0, 0.0, 0.0])];
    // Frost Nova (Cone of Cold's EX, 25 Valor): the boar frozen, the Fireball key empowered 5 s.
    s.set_valor(p, 25.0);
    let ev = press(&mut s, p, "close", true);
    assert!(missile(&ev, 122), "the nova goes");
    assert!(s.has_cc(k, "petrify"), "the boar frozen");
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "one key empowered: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize, "the Fireball's");
    assert!((em[0].1 - (s.clock + 5.0)).abs() < 0.2, "for 5 s");
    let valor = s.players[&p].valor;
    // The plain Fireball key now aims a Pyroblast; held to its moment (all or nothing: rolled as
    // the charge begins) it goes, for no Valor.
    run_pinned(&mut s, 0.3, &pins);
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "pyroblast")), "the key's EX version, Pyroblast");
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    let flare = s.players[&p].check_flare;
    ev.extend(run_pinned(&mut s, flare - DT, &pins));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    assert!(missile(&ev, 11366), "the Pyroblast flies");
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert!(crate::sim::cues::seen::beat(&ev, p, crate::sim::cues::BEAT_SHATTER) && crate::sim::cues::seen::looked(&ev, k, crate::sim::cues::KIT_FROST_BURST), "the frozen boar shatters");
    // The same shatter as a Hunter's: the action lines' burst on the frozen one too, and the
    // server adds no fire of its own (Pyroblast's own impact is the client's, from its spell).
    assert!(s.content.shatter.burst_kit != 0 && crate::sim::cues::seen::looked(&ev, k, s.content.shatter.burst_kit), "the shatter's action lines");
    assert!(!flashed(&ev, p, "Shatter"), "no word");
    assert!(hits(&ev, p, k).iter().any(|d| *d >= 70), "half again its 48: {:?}", hits(&ev, p, k));
    assert!(s.players[&p].valor + 0.001 >= valor, "no Valor spent: {} then {}", valor, s.players[&p].valor);
    assert!(s.empowered_keys(p).is_empty(), "the combo spent");
    // Then the key is Fireball again.
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "fireball")));
}

// --- Warlock: rot ---------------------------------------------------------------------------------

#[test]
fn a_seed_of_corruption_sticks_then_bursts_round_its_carrier_and_leaves_them_rotting() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "seed_of_corruption");
    let carrier = boar(&mut s, [8.0, 0.0, 0.0]);
    let beside = boar(&mut s, [10.0, 2.5, 0.0]);
    let away = boar(&mut s, [8.0, -9.0, 0.0]);
    let pins = [(carrier, [8.0, 0.0, 0.0]), (beside, [10.0, 2.5, 0.0]), (away, [8.0, -9.0, 0.0])];
    let (hc, hb, ha) = (s.mobs[&carrier].health, s.mobs[&beside].health, s.mobs[&away].health);
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "seed_of_corruption")));
    let mut ev = hold(&mut s, p, 0.0, &pins);
    ev.extend(run_pinned(&mut s, 0.8, &pins));
    // Planted: nothing yet, but it's in the boar and shows on it.
    assert!(hits(&ev, p, carrier).is_empty() && s.mobs[&carrier].health == hc, "nothing yet");
    assert!(s.projectiles.iter().any(|x| matches!(x.stage, Stage::Stuck { on: Some(u), .. } if u == carrier)), "stuck in it");
    assert!(s.auras_of(carrier).contains(&23072), "the corruption pulsing on it");
    // Two seconds on, it bursts round the boar: the one beside it struck too, both rotting.
    ev.extend(run_pinned(&mut s, 1.8, &pins));
    assert!(s.mobs[&carrier].health < hc && s.mobs[&beside].health < hb, "it went off round its carrier");
    assert_eq!(s.mobs[&away].health, ha, "not past its 5 yd");
    assert!(s.has_cc(carrier, "burn") && s.has_cc(beside, "burn"), "each left rotting");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { spell: 24050, target: Target::Point(at), .. } if dist2d(*at, [8.0, 0.0, 0.0]) < 1.0)),
        "its burst there"
    );
    assert!(s.projectiles.is_empty());
}

/// How long an effect of `kind` has left on `u`.
fn left(s: &Survival, u: Id, kind: &str) -> f32 {
    s.cc_of(u).iter().find(|c| c.kind == kind).map_or(0.0, |c| c.until - s.clock)
}

/// A rot of the Warlock's (Seed of Corruption's: 4 damage a second for 4 sec) set on a foe.
fn rot(s: &mut Survival, from: Id, on: Id) {
    assert!(s.apply_cc_look(on, from, "burn", 4.0, 4.0, 0.0, "Seed of Corruption", 172));
}

#[test]
fn soul_swap_copies_the_warlocks_rot_from_the_foe_at_the_reticle_onto_each_foe_within_10_yd() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "seed_of_corruption");
    s.set_cooldowns(p, true);
    // The one under the reticle; two within 10 yd of it, one of them a hero; one past it.
    let src = boar(&mut s, [14.0, 0.0, 0.0]);
    let near = boar(&mut s, [14.0, 8.0, 0.0]);
    let edge = boar(&mut s, [14.0, -9.5, 0.0]);
    let far = boar(&mut s, [14.0, 14.0, 0.0]);
    let foe = hero(&mut s, "Bren", [8.0, -4.0, 0.0], std::f32::consts::PI);
    let pins = [(src, [14.0, 0.0, 0.0]), (near, [14.0, 8.0, 0.0]), (edge, [14.0, -9.5, 0.0]), (far, [14.0, 14.0, 0.0])];
    let d = s.content.abilities[idx(&s, "soul_swap")].clone();
    assert_eq!((d.range[1], d.splash, d.valor, d.kind.as_str()), (30.0, 10.0, 25, "spread"));
    rot(&mut s, p, src);
    s.set_valor(p, 25.0);
    run_pinned(&mut s, 1.0, &pins);
    let was = left(&s, src, "burn");
    press(&mut s, p, "ranged", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "soul_swap")), "Ctrl: the EX version");
    let ev = hold(&mut s, p, 0.0, &pins);
    assert!(missile(&ev, 686), "its look flies to them");
    for m in [near, edge] {
        let c = s.cc_of(m).iter().find(|c| c.kind == "burn").expect("the rot is on it");
        assert_eq!((c.from, c.amount, c.source.as_str(), c.aura), (p, 4.0, "Seed of Corruption", 172));
        assert!((left(&s, m, "burn") - left(&s, src, "burn")).abs() < 0.2, "the time it had left: {} and {}", left(&s, m, "burn"), left(&s, src, "burn"));
    }
    assert!(left(&s, near, "burn") < 3.2 && left(&s, near, "burn") > 2.0, "what was left of 4 sec, not a new 4: {}", left(&s, near, "burn"));
    assert!(left(&s, src, "burn") <= was, "the foe it came from keeps its own, no longer");
    assert!(s.has_cc(src, "burn"));
    assert!(!s.has_cc(far, "burn"), "past 10 yd of it nothing");
    // The hero within 10 yd of it takes it too (shortened as any effect is on a hero, but with the
    // time the rot had left).
    assert!((left(&s, foe, "burn") - left(&s, src, "burn")).abs() < 0.2, "a hero: {} and {}", left(&s, foe, "burn"), left(&s, src, "burn"));
    assert_eq!(s.players[&p].valor, 0.0, "25 Valor");
    // It deals nothing itself: only the rot ticks.
    let ev: Vec<Event> = ev.into_iter().filter(|e| matches!(e, Event::Swing { style: Blow::Arrow(_) | Blow::Heavy | Blow::Light, .. })).collect();
    assert!(ev.is_empty(), "no blow of its own: {ev:?}");
}

#[test]
fn soul_swap_takes_only_the_warlocks_own_rot_and_spends_nothing_when_there_is_none() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let other = hero(&mut s, "Bren", [-5.0, 0.0, 0.0], 0.0);
    learn(&mut s, p, "seed_of_corruption");
    s.set_cooldowns(p, true);
    let src = boar(&mut s, [14.0, 0.0, 0.0]);
    let near = boar(&mut s, [14.0, 6.0, 0.0]);
    let pins = [(src, [14.0, 0.0, 0.0]), (near, [14.0, 6.0, 0.0])];
    // No rot at all.
    s.set_valor(p, 25.0);
    press(&mut s, p, "ranged", true);
    let ev = hold(&mut s, p, 0.0, &pins);
    assert!(flashed(&ev, p, "Nothing to spread"), "said so");
    assert!(!s.has_cc(near, "burn"));
    assert_eq!(s.players[&p].valor, 25.0, "nothing paid");
    // Someone else's rot on it is not the Warlock's to spread.
    rot(&mut s, other, src);
    run_pinned(&mut s, 1.0, &pins);
    press(&mut s, p, "ranged", true);
    let ev = hold(&mut s, p, 0.0, &pins);
    assert!(flashed(&ev, p, "Nothing to spread"));
    assert!(!s.has_cc(near, "burn"), "not theirs");
    assert_eq!(s.players[&p].valor, 25.0);
    // With no other foe near the one with the rot, there is nothing to give it to.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "seed_of_corruption");
    s.set_cooldowns(p, true);
    let lone = boar(&mut s, [14.0, 0.0, 0.0]);
    let pins = [(lone, [14.0, 0.0, 0.0])];
    rot(&mut s, p, lone);
    s.set_valor(p, 25.0);
    press(&mut s, p, "ranged", true);
    let ev = hold(&mut s, p, 0.0, &pins);
    assert!(flashed(&ev, p, "No other enemies near"));
    assert_eq!(s.players[&p].valor, 25.0, "nothing paid");
}

#[test]
fn soul_swaps_words_say_what_the_data_does() {
    let s = world(false);
    let d = &s.content.abilities[idx(&s, "soul_swap")];
    assert!(d.text.contains(&format!("up to {} yd away", d.range[1])), "{}", d.text);
    assert!(d.text.contains(&format!("within {} yd of it", d.splash)), "{}", d.text);
    assert_eq!(s.content.ex_of(idx(&s, "seed_of_corruption")), Some(idx(&s, "soul_swap")));
}

#[test]
fn howl_of_terror_sends_every_foe_within_10_yd_fleeing_for_4_sec_a_hero_for_1_5() {
    let mut s = world(true);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "shadowfury");
    let d = s.content.abilities[idx(&s, "howl_of_terror")].clone();
    let e = d.effects[0].clone();
    assert_eq!((d.radius[1], d.valor, e.kind.as_str(), e.secs[1], e.hero_secs, e.break_after), (10.0, 25, "panic", 4.0, 1.5, 1.0));
    let near = boar(&mut s, [3.0, 0.0, 0.0]);
    let behind = boar(&mut s, [-9.0, 0.0, 0.0]);
    let past = boar(&mut s, [11.5, 0.0, 0.0]);
    let foe = hero(&mut s, "Bren", [0.0, 6.0, 0.0], 0.0);
    s.set_valor(p, 25.0);
    let ev = press(&mut s, p, "close", true);
    assert!(missile(&ev, 5484), "Howl of Terror's look");
    for m in [near, behind] {
        assert!((left(&s, m, "panic") - 4.0).abs() < 0.2, "4 sec: {}", left(&s, m, "panic"));
    }
    assert!((left(&s, foe, "panic") - 1.5).abs() < 0.2, "a hero 1.5 sec: {}", left(&s, foe, "panic"));
    assert!(!s.has_cc(past, "panic"), "nothing past 10 yd");
    assert_eq!(s.players[&p].valor, 0.0, "25 Valor");
    // They run: in a second and a half the one near is well away from where it stood.
    run_pinned(&mut s, 1.5, &[]);
    assert!(dist2d(s.mobs[&near].pos, ORIGIN) > 5.0, "it fled: {:?}", s.mobs[&near].pos);
    run_pinned(&mut s, 3.0, &[]);
    assert!(!s.has_cc(near, "panic"), "over after its 4 sec");
}

#[test]
fn howl_of_terror_breaks_on_enough_damage_but_not_in_its_first_second() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "shadowfury");
    let a = boar(&mut s, [3.0, 0.0, 0.0]);
    let b = boar(&mut s, [-3.0, 0.0, 0.0]);
    s.set_valor(p, 25.0);
    press(&mut s, p, "close", true);
    let max = s.mobs[&a].max_health;
    let share = (max as f32 * s.content.cc.panic_break).ceil() as u32 + 1;
    // In the first second even a blow that would break any fear doesn't.
    assert_eq!(s.cc_on_damage(a, share), share, "it takes the blow");
    assert!(s.has_cc(a, "panic"), "still fleeing: the grace");
    // After it, the same blow ends it.
    run_pinned(&mut s, 1.1, &[]);
    s.cc_on_damage(b, share);
    assert!(!s.has_cc(b, "panic"), "enough damage ends it early");
    // A small blow doesn't, whenever it comes.
    let c = boar(&mut s, [0.0, 3.0, 0.0]);
    s.apply_cc(c, p, "panic", 4.0, 0.0, 1.0, "test");
    run_pinned(&mut s, 1.1, &[]);
    s.cc_on_damage(c, 1);
    assert!(s.has_cc(c, "panic"), "a scratch doesn't");
}

#[test]
fn howl_of_terror_does_not_take_on_a_boss() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "shadowfury");
    let boss = {
        let def = s.content.mob("warlord").unwrap();
        let id = s.spawn_mob(def, [4.0, 0.0, 0.0], None, false);
        s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
        id
    };
    let k = boar(&mut s, [-4.0, 0.0, 0.0]);
    s.set_valor(p, 25.0);
    press(&mut s, p, "close", true);
    assert!(s.has_cc(k, "panic"));
    assert!(!s.has_cc(boss, "panic"), "a boss holds its ground, as against any control");
}

#[test]
fn howl_of_terrors_words_say_what_the_data_does() {
    let s = world(false);
    let d = &s.content.abilities[idx(&s, "howl_of_terror")];
    let e = &d.effects[0];
    assert!(d.text.contains(&format!("within {} yd", d.radius[1])), "{}", d.text);
    assert!(d.text.contains(&format!("for {} sec ({} sec against heroes)", e.secs[1], e.hero_secs)), "{}", d.text);
    assert!(d.text.contains(&format!("not in the first {} sec", e.break_after)), "{}", d.text);
    assert_eq!(s.content.ex_of(idx(&s, "shadowfury")), Some(idx(&s, "howl_of_terror")));
}

#[test]
fn drain_life_on_the_defense_key_heals_the_warlock_all_it_drains() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "drain_life");
    s.players.get_mut(&p).unwrap().health = 100;
    let k = boar(&mut s, [8.0, 0.0, 0.0]);
    let pins = [(k, [8.0, 0.0, 0.0])];
    press(&mut s, p, "defense", false);
    let mut ev = hold(&mut s, p, 0.0, &pins);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "tethered");
    ev.extend(run_pinned(&mut s, 3.5, &pins));
    let drained: u32 = hits(&ev, p, k).iter().sum();
    assert!(drained >= 24, "about 12 a second for 3 s: {drained}");
    assert_eq!(s.players[&p].health, 100 + drained, "every point of it back");
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "over after 3 s");
}

#[test]
fn death_coil_sends_its_target_fleeing_and_heals_the_warlock() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "drain_life");
    s.players.get_mut(&p).unwrap().health = 100;
    s.set_valor(p, 25.0);
    let k = boar(&mut s, [8.0, 0.0, 0.0]);
    press(&mut s, p, "defense", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "death_coil")), "Ctrl: the EX version");
    let mut ev = hold(&mut s, p, 0.0, &[]);
    ev.extend(run_pinned(&mut s, 0.6, &[]));
    assert!(missile(&ev, 6789), "the coil flies");
    assert!(s.has_cc(k, "panic"), "the boar flees");
    let max = s.players[&p].max_health;
    assert_eq!(s.players[&p].health, 100 + (max as f32 * 0.15).round() as u32, "15% of the warlock's health back");
}

#[test]
fn an_old_save_with_shadow_bolt_and_shadow_ward_loads_with_their_successors() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.class = "warlock".into();
    save.owned = ["shadow_bolt", "shadowfury", "demonic_circle", "shadow_ward"].map(String::from).to_vec();
    save.loadout = BTreeMap::from([("ranged".to_string(), "shadow_bolt".to_string()), ("defense".to_string(), "shadow_ward".to_string())]);
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.contains("seed_of_corruption") && h.owned.contains("drain_life"), "{:?}", h.owned);
    assert!(!h.owned.contains("shadow_bolt") && !h.owned.contains("shadow_ward"));
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("seed_of_corruption"));
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("drain_life"));
}

// --- Druid: seeds for friends, claws for foes -------------------------------------------------------

#[test]
fn a_living_seed_strikes_a_foe_and_settles_on_a_friend_to_mend_them_friendly_fire_or_not() {
    // On a co-op friend, with friendly fire on: mended over 4 s, never struck.
    let mut s = world(true);
    assert!(s.content.pvp.friendly_fire);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    learn(&mut s, a, "living_seed");
    s.players.get_mut(&b).unwrap().health = 100;
    press(&mut s, a, "ranged", false);
    assert_eq!(aiming(&s, a), Some(idx(&s, "living_seed")));
    let mut ev = hold(&mut s, a, 0.0, &[]);
    ev.extend(run_pinned(&mut s, 0.8, &[]));
    assert!(hits(&ev, a, b).is_empty(), "not struck");
    assert!(s.has_cc(b, "regen"), "the seed settled on him");
    ev.extend(run_pinned(&mut s, 3.5, &[]));
    let max = s.players[&b].max_health as f32;
    assert!(s.players[&b].health as f32 >= 100.0 + max * 0.04 * 3.0, "about 4% a second: {}", s.players[&b].health);
    assert!(!s.has_cc(a, "regen"), "not on her");
    // On a foe: struck for 16, and none of the mending.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "living_seed");
    let k = boar(&mut s, [8.0, 0.0, 0.0]);
    let pins = [(k, [8.0, 0.0, 0.0])];
    press(&mut s, p, "ranged", false);
    let mut ev = hold(&mut s, p, 0.0, &pins);
    ev.extend(run_pinned(&mut s, 0.8, &pins));
    assert_eq!(hits(&ev, p, k), vec![16], "struck for 16");
    assert!(!s.has_cc(k, "regen"), "a foe isn't mended");
    // Two at hand: a second at once, then the key waits.
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "living_seed")), "the second seed");
    hold(&mut s, p, 0.0, &pins);
    run_pinned(&mut s, 0.5, &pins);
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), None, "both spent");
}

#[test]
fn ironbark_wraps_the_friend_under_the_reticle_or_else_the_druid() {
    // A friend ahead: his, not hers.
    let mut s = world(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    learn(&mut s, a, "barkskin");
    s.set_valor(a, 25.0);
    press(&mut s, a, "defense", true);
    assert_eq!(aiming(&s, a), Some(idx(&s, "ironbark")), "Ctrl: the EX version, aimed at a friend");
    let ev = hold(&mut s, a, 0.0, &[]);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 22812, target: Target::Unit(t) } if *caster == a && *t == b)), "its look on him");
    assert!(s.has_cc(b, "hallowed") && s.has_cc(b, "regen"), "his bark and mending");
    assert!(!s.has_cc(a, "hallowed") && !s.has_cc(a, "regen"), "not hers");
    // Alone: her own.
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "barkskin");
    s.set_valor(p, 25.0);
    press(&mut s, p, "defense", true);
    hold(&mut s, p, 0.0, &[]);
    assert!(s.has_cc(p, "hallowed") && s.has_cc(p, "regen"), "solo, on herself");
}

#[test]
fn an_old_save_with_wrath_loads_with_living_seed() {
    let mut s = world(false);
    s.content.dev.full_tree = false;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.class = "druid".into();
    save.owned = ["wrath", "swipe", "feral_leap"].map(String::from).to_vec();
    save.loadout = BTreeMap::from([("ranged".to_string(), "wrath".to_string()), ("utility".to_string(), "feral_leap".to_string())]);
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert!(h.owned.contains("living_seed") && !h.owned.contains("wrath"), "{:?}", h.owned);
    assert_eq!(h.loadout.get("ranged").map(String::as_str), Some("living_seed"));
    assert_eq!(h.loadout.get("utility").map(String::as_str), Some("feral_leap"));
}

#[test]
fn typhoon_throws_a_creature_back_and_its_slow_takes_hold_after() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    learn(&mut s, p, "swipe");
    let b = boar(&mut s, [4.0, 0.0, 0.0]);
    s.set_valor(p, 25.0);
    let mut ev = press(&mut s, p, "close", true);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    ev.extend(s.step(DT));
    s.input(p, Input::AttackUp);
    for _ in 0..(1.2 / DT).round() as usize {
        ev.extend(s.step(DT));
    }
    let x = s.mobs[&b].pos[0];
    assert!(x > 11.0, "thrown back about 9 yd from 4: at {x:.1}");
    assert!(s.has_cc(b, "snare"), "and slowed");
    let _ = ev;
}
