//! The Rogue's Blades tree (docs/rogue-roles.md, build plan entry 4): Flourish, its trait; Darting
//! Knife and its EX Pinning Knife; Blade Flurry and its EX Hemorrhage; Quickstep and its EX
//! Afterimage; Evasion and its EX Adrenaline Rush; Blade Dance; and the board's third choice in
//! the Rogue's column. The perfect dodge itself is the core's (`tests_rogue_core.rs`).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::rogue::BLADES;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const PI: f32 = std::f32::consts::PI;

fn world_with(full_tree: bool) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = full_tree;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(true)
}

/// A hero of this class, made as the creation screen makes one, at the origin facing +x.
fn hero_of(s: &mut Survival, name: &str, class: &str) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// Blades' five cards on these keys.
const BLADES_KEYS: [(&str, &str); 5] = [("ranged", "darting_knife"), ("close", "blade_flurry"), ("utility", "quickstep"), ("defense", "blade_evasion"), ("ultimate", "blade_dance")];

fn on_keys(s: &mut Survival, p: Id, keys: &[(&str, &str)]) {
    for (slot, k) in keys {
        s.slot_ability(p, slot, k).unwrap();
    }
}

/// A Rogue (two daggers) with all five of Blades' cards on the keys: its trait on.
fn rogue(s: &mut Survival) -> Id {
    let p = hero_of(s, "Ayla", "rogue");
    on_keys(s, p, &BLADES_KEYS);
    assert!(s.rogue_trait(&s.players[&p], BLADES));
    p
}

/// A creature at `at` facing `f`, at rest, that strikes nothing on its own.
fn mob(s: &mut Survival, key: &str, at: Vec3, f: f32) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, at, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = at;
    m.facing = f;
    m.ready_at = 1e9;
    m.health = 100_000;
    m.max_health = 100_000;
    id
}

/// A creature with no shield 2 yd in front of the Rogue, facing her.
fn foe(s: &mut Survival) -> Id {
    mob(s, "dm_woodcutter", [2.0, 0.0, 0.0], PI)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Its key pressed (`ex`: its EX version, with Valor for it), an aimed one let go on a tap of the
/// click. Stamina is filled first unless `keep` says to leave it.
fn press_as(s: &mut Survival, p: Id, slot: &str, ex: bool, keep: bool) -> Vec<Event> {
    s.set_valor(p, 100.0);
    if !keep {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
    }
    s.input(p, Input::Ability(key_of(slot), ex));
    let mut ev = s.step(DT);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(DT));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
    }
    ev
}

fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    press_as(s, p, slot, ex, false)
}

/// A dodge on the spot, `secs` into it.
fn dodge_for(s: &mut Survival, p: Id, secs: f32) {
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "dodging: {:?}", s.players[&p].action);
    run(s, secs);
}

/// A blow from `attacker` (2 yd in front) on hero `p`, at once.
fn blow(s: &mut Survival, attacker: Id, p: Id) -> Outcome {
    s.strike_player(attacker, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light)
}

fn cut_open(s: &Survival, p: Id) -> bool {
    s.players[&p].rogue.cut_until > s.clock
}

fn flourish(s: &Survival, p: Id) -> u32 {
    s.flourish_of(&s.players[&p])
}

/// A perfect dodge of a blow from `m`, the dodge then run out.
fn perfect(s: &mut Survival, p: Id, m: Id) {
    dodge_for(s, p, 0.1);
    assert_eq!(blow(s, m, p), Outcome::Dodged);
    run(s, 0.5);
}

fn strike(s: &Survival, p: Id) -> Option<Strike> {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => Some(strike.clone()),
        _ => None,
    }
}

/// The swings the hero began in these events (each blow's start).
fn swings(ev: &[Event], p: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: None, anim: true, .. } if *attacker == p)).count()
}

// --- Flourish ---------------------------------------------------------------------------------------

#[test]
fn a_perfect_dodge_gives_a_flourish_up_to_5_for_6_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let r = s.content.rogue.clone();
    assert_eq!((r.flourish_most, r.flourish_secs), (5, 6.0));
    perfect(&mut s, p, m);
    assert_eq!(flourish(&s, p), 1);
    assert!(s.auras_of(p).contains(&r.flourish_aura), "its look on the body");
    for _ in 0..6 {
        perfect(&mut s, p, m);
    }
    assert_eq!(flourish(&s, p), 5, "up to 5");
    // A plain dodge (late in it) gives none, and doesn't refresh.
    dodge_for(&mut s, p, 0.24);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert_eq!(flourish(&s, p), 5);
    // 6 sec from the latest perfect one, gone, its look with it and the count told 0.
    run(&mut s, 4.5);
    assert_eq!(flourish(&s, p), 5, "still within 6 sec of the latest");
    let ev = run(&mut s, 1.5);
    assert_eq!(flourish(&s, p), 0);
    assert!(!s.auras_of(p).contains(&r.flourish_aura));
    assert!(ev.iter().any(|e| matches!(e, Event::Stacks { player, kind: "flourish", n: 0, .. } if *player == p)));
}

#[test]
fn each_flourish_counts_to_the_rogues_own_screen() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    dodge_for(&mut s, p, 0.1);
    s.events.clear();
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    let ev = std::mem::take(&mut s.events);
    assert!(ev.iter().any(|e| matches!(e, Event::Stacks { player, kind: "flourish", n: 1, on: None } if *player == p)), "the count beside the reticle");
}

#[test]
fn the_blades_trait_is_on_with_3_blades_cards_and_never_another_classes() {
    let mut s = world();
    let p = hero_of(&mut s, "Ayla", "rogue");
    on_keys(&mut s, p, &BLADES_KEYS[..2]);
    let m = foe(&mut s);
    perfect(&mut s, p, m);
    assert_eq!(flourish(&s, p), 0, "2 of 5: no trait");
    on_keys(&mut s, p, &BLADES_KEYS[2..3]);
    perfect(&mut s, p, m);
    assert_eq!(flourish(&s, p), 1, "3 of 5: Flourish");
    // A Warrior with all five has no perfect dodge, so no Flourish.
    let w = hero_of(&mut s, "Bren", "warrior");
    on_keys(&mut s, w, &BLADES_KEYS);
    assert!(!s.rogue_trait(&s.players[&w], BLADES));
}

#[test]
fn each_flourish_makes_blows_6_percent_harder_and_4_percent_quicker() {
    let mut s = world();
    let p = rogue(&mut s);
    let plain = s.strike_at(p, StrikeKind::Light, 0);
    let off = s.strike_at(p, StrikeKind::Off, 0);
    let h = s.players.get_mut(&p).unwrap();
    h.rogue.flourish = 5;
    h.rogue.flourish_until = s.clock + 6.0;
    let five = s.strike_at(p, StrikeKind::Light, 0);
    let off5 = s.strike_at(p, StrikeKind::Off, 0);
    assert!((five.damage / plain.damage - 1.30).abs() < 1.0e-4, "30% harder at 5: {} {}", five.damage, plain.damage);
    assert!((off5.damage / off.damage - 1.30).abs() < 1.0e-4, "both hands");
    assert!((plain.windup / five.windup - 1.20).abs() < 1.0e-4, "20% quicker at 5");
    assert!((plain.recovery / five.recovery - 1.20).abs() < 1.0e-4);
}

#[test]
fn a_blow_landing_on_the_rogue_takes_every_flourish() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    for _ in 0..3 {
        perfect(&mut s, p, m);
    }
    assert_eq!(flourish(&s, p), 3);
    s.events.clear();
    assert_eq!(blow(&mut s, m, p), Outcome::Hit);
    assert_eq!(flourish(&s, p), 0, "one blow takes them all");
    let ev = std::mem::take(&mut s.events);
    assert!(ev.iter().any(|e| matches!(e, Event::Stacks { player, kind: "flourish", n: 0, .. } if *player == p)));
}

// --- Darting Knife and Pinning Knife ----------------------------------------------------------------

#[test]
fn a_darting_knife_that_strikes_counts_as_a_hand() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = mob(&mut s, "dm_woodcutter", [6.0, 0.0, 0.0], PI);
    // Two knives: the run is at 2, either hand next.
    for n in 1..=2u8 {
        press(&mut s, p, "ranged", false);
        let mut struck = false;
        for _ in 0..60 {
            let ev = s.step(DT);
            if ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::Hit, .. } if *attacker == p && *v == m)) {
                struck = true;
                break;
            }
        }
        assert!(struck, "knife {n} strikes");
        assert_eq!(s.players[&p].dual_run, n, "knife {n} is a hand");
        run(&mut s, 0.2);
    }
    // Step in, two blows (main, then off): the second is the finisher with both blades.
    s.move_player(p, [4.5, 0.0, 0.0], 0.0, Moving::default());
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    for _ in 0..30 {
        if strike(&s, p).is_some() {
            break;
        }
        let _ = s.step(DT);
    }
    assert!(strike(&s, p).is_some_and(|st| !st.finish), "the 3rd blow: {:?}", s.players[&p].action);
    for _ in 0..80 {
        let _ = s.step(DT);
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
            break;
        }
    }
    s.input(p, Input::Guard);
    for _ in 0..30 {
        let _ = s.step(DT);
        if strike(&s, p).is_some_and(|st| st.kind == StrikeKind::Off) {
            break;
        }
    }
    assert!(strike(&s, p).is_some_and(|st| st.finish), "the 4th: both blades, two blows after two knives");
    // A knife never is the finisher itself.
    let mut s = world();
    let p = rogue(&mut s);
    s.players.get_mut(&p).unwrap().dual_run = DUAL_FINISH_AT - 1;
    s.players.get_mut(&p).unwrap().combo_until = s.clock;
    s.knife_hand(p);
    assert_eq!(s.players[&p].dual_run, DUAL_FINISH_AT - 1);
}

#[test]
fn pinning_knife_stops_an_enemy_turning_for_3_sec_and_a_boss_turns_at_half_speed() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    press(&mut s, p, "ranged", true);
    run(&mut s, 0.3);
    assert!(s.has_cc(m, "pinned"), "the knife pins it");
    let e = &s.content.abilities[idx(&s, "pinning_knife")].effects[0];
    assert_eq!((e.kind.as_str(), e.secs[1]), ("pinned", 3.0));
    // Its foe goes round to its back: it doesn't turn after her.
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let _ = s.step(DT);
    let (f0, at) = (s.mobs[&m].facing, s.mobs[&m].pos);
    s.move_player(p, [3.5, 0.0, 0.0], PI, Moving::default());
    run(&mut s, 1.0);
    let f = s.mobs[&m].facing;
    assert!(angle_between(f, f0) < 0.01, "still facing where it did: {f} {f0}");
    assert!(dist2d(s.mobs[&m].pos, at) < 0.05, "and stands");
    // Its wind-up keeps no turn either.
    assert_eq!(s.attack_now(m).turn_rate, 0.0);
    // 3 sec on, it turns to her.
    run(&mut s, 2.5);
    assert!(!s.has_cc(m, "pinned"));
    run(&mut s, 0.5);
    assert!(angle_between(s.mobs[&m].facing, f0) > 1.0, "free, it turns: {}", s.mobs[&m].facing);
    // A boss turns at half its speed.
    let mut s = world();
    let p = rogue(&mut s);
    let boss = s.content.mobs.iter().position(|d| d.boss && d.attack.turn_rate > 0.0).unwrap();
    let key = s.content.mobs[boss].key.clone();
    let b = mob(&mut s, &key, [2.0, 0.0, 0.0], PI);
    let base = s.attack_now(b).turn_rate;
    let pin = s.content.abilities[idx(&s, "pinning_knife")].effects.clone();
    s.apply_effects(p, b, &pin, 1.0, "Pinning Knife");
    assert!(s.has_cc(b, "pinned"));
    assert!((s.attack_now(b).turn_rate - base * s.content.rogue.pinned_boss_turn).abs() < 1.0e-4);
    assert_eq!(s.content.rogue.pinned_boss_turn, 0.5);
}

// --- Blade Flurry and Hemorrhage --------------------------------------------------------------------

#[test]
fn blade_flurry_cuts_4_times_0_3_sec_apart_and_ends_in_the_finisher() {
    let mut s = world();
    let p = rogue(&mut s);
    let _m = foe(&mut s);
    let d = s.content.abilities[idx(&s, "blade_flurry")].clone();
    assert_eq!((d.cuts, d.stamina, d.cooldown, d.needs_two_blades), (4, 20.0, 8.0, true));
    let gap = s.content.rogue.flurry_gap;
    assert_eq!(gap, 0.3);
    let mut starts: Vec<(f32, StrikeKind, bool, f32)> = Vec::new();
    s.set_valor(p, 0.0);
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Ability(key_of("close"), false));
    assert!((s.players[&p].stamina - (100.0 - d.stamina)).abs() < 1.0e-3, "20 stamina, once");
    let mut ev = s.step(DT);
    for _ in 0..100 {
        if swings(&ev, p) > 0 {
            if let Some(st) = strike(&s, p) {
                starts.push((s.clock, st.kind, st.finish, st.lunge));
            }
        }
        ev = s.step(DT);
    }
    assert_eq!(starts.len(), 4, "4 cuts: {starts:?}");
    for w in starts.windows(2) {
        assert!((w[1].0 - w[0].0 - gap).abs() <= DT + 1.0e-3, "0.3 sec apart: {starts:?}");
        assert_ne!(w[0].1, w[1].1, "the hands alternate");
    }
    assert_eq!(starts[0].1, StrikeKind::Light, "main hand first");
    assert!(starts.iter().all(|c| c.3 == s.content.rogue.flurry_step), "each steps 1 yd in");
    assert_eq!(starts.iter().map(|c| c.2).collect::<Vec<_>>(), [false, false, false, true], "the 4th is the finisher");
    assert!(s.players[&p].stamina >= 100.0 - d.stamina - 1.0e-3, "the cuts cost nothing more: {}", s.players[&p].stamina);
    // With one blade it refuses.
    let mut s = world();
    let p = rogue(&mut s);
    s.players.get_mut(&p).unwrap().off_weapon = None;
    s.players.get_mut(&p).unwrap().off_empty = true;
    let ev = press(&mut s, p, "close", false);
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == p && text.contains("requires a weapon in each hand"))));
}

#[test]
fn a_dodge_cancels_blade_flurry_between_cuts() {
    let mut s = world();
    let p = rogue(&mut s);
    let _m = foe(&mut s);
    press(&mut s, p, "close", false);
    // The first cut struck: a dodge goes, and no more cuts come.
    for _ in 0..40 {
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active) {
            break;
        }
        let _ = s.step(DT);
    }
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "the dodge cuts it: {:?}", s.players[&p].action);
    let ev = run(&mut s, 1.5);
    assert_eq!(swings(&ev, p), 0, "no cut after the dodge");
    assert_eq!(s.players[&p].rogue.flurry_left, 0);
}

#[test]
fn hemorrhage_stops_backstabs_on_it_losing_bonus_for_8_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    press(&mut s, p, "close", true);
    run(&mut s, 0.5);
    let e = s.content.abilities[idx(&s, "hemorrhage")].effects[0].clone();
    assert_eq!((e.kind.as_str(), e.secs[1]), ("hemorrhage", 8.0));
    assert!(s.has_cc(m, "hemorrhage"), "the cut opens it");
    for _ in 0..3 {
        s.backstabbed(m, p);
    }
    assert_eq!(s.backstab_bonus_for_test(m), 2.0, "every backstab keeps the whole bonus");
    // After it, backstabs count again (those under it weren't remembered).
    run(&mut s, 8.0);
    assert!(!s.has_cc(m, "hemorrhage"));
    assert_eq!(s.backstab_bonus_for_test(m), 2.0, "none remembered from under it");
    for _ in 0..3 {
        s.backstabbed(m, p);
    }
    assert!((s.backstab_bonus_for_test(m) - (1.0 + 0.75f32.powi(3))).abs() < 1.0e-4);
}

// --- Quickstep and Afterimage -----------------------------------------------------------------------

#[test]
fn quickstep_has_2_charges_costs_no_stamina_and_evades_like_a_dodge() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    let d = s.content.abilities[idx(&s, "quickstep")].clone();
    assert_eq!((d.charges, d.cooldown, d.quickstep), (2, 10.0, true));
    s.players.get_mut(&p).unwrap().stamina = 40.0;
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "a dodge: {:?}", s.players[&p].action);
    assert_eq!(s.players[&p].stamina, 40.0, "for no stamina");
    // A dash along the movement keys at its own pace: 5 yd in the dodge's 0.25 sec.
    s.players.get_mut(&p).unwrap().action = Action::Dodge { t: 0.0, dir: 0.0, dash: true };
    assert_eq!(s.speed_for(p), s.content.rogue.quick_speed);
    assert!((s.content.rogue.quick_speed * s.content.dodge.dash_secs - 5.0).abs() < 1.0e-4);
    // It evades, and a perfect one pays.
    run(&mut s, 0.1);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(cut_open(&s, p), "perfect: the Shadow Cut");
    assert_eq!(flourish(&s, p), 1, "and a Flourish");
    // The second charge from inside the dodge: a chain.
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(matches!(s.players[&p].action, Action::Dodge { t, .. } if t < 0.05), "a fresh dodge");
    run(&mut s, 0.6);
    // No third.
    let _ = press_as(&mut s, p, "utility", false, true);
    assert!(!matches!(s.players[&p].action, Action::Dodge { .. }), "2 charges");
    // A plain dodge right after still costs what a dodge does.
    s.players.get_mut(&p).unwrap().stamina = 40.0;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].stamina, 40.0 - s.content.dodge.stamina);
    assert!(!s.players[&p].rogue.quick, "at the dodge's own pace");
}

#[test]
fn quickstep_cancels_your_own_wind_up_and_gives_its_stamina_back() {
    let mut s = world();
    let p = rogue(&mut s);
    let _m = foe(&mut s);
    s.players.get_mut(&p).unwrap().stamina = 60.0;
    s.input(p, Input::Attack);
    let paid = 60.0 - s.players[&p].stamina;
    assert!(paid > 0.0 && matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t < strike.windup));
    s.set_valor(p, 0.0);
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "out of the wind-up: {:?}", s.players[&p].action);
    assert!((s.players[&p].stamina - 60.0).abs() < 1.0e-3, "its stamina back: {}", s.players[&p].stamina);
    // Late in a blow too (after it struck: nothing back, the step goes).
    let mut s = world();
    let p = rogue(&mut s);
    let _m = foe(&mut s);
    s.players.get_mut(&p).unwrap().stamina = 60.0;
    s.input(p, Input::Attack);
    for _ in 0..40 {
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active) {
            break;
        }
        let _ = s.step(DT);
    }
    let before = s.players[&p].stamina;
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "out of the recovery at once");
    assert!(s.players[&p].stamina <= before + 1.0e-3, "nothing back once it struck");
}

#[test]
fn afterimage_leaves_a_shade_and_the_enemies_on_you_turn_to_it() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    s.mobs.get_mut(&m).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let d = s.content.abilities[idx(&s, "afterimage")].clone();
    assert_eq!((d.shade, d.shade_turn, d.valor), (2.0, 1.5, 25));
    s.set_valor(p, 25.0);
    s.input(p, Input::Ability(key_of("utility"), true));
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "a Quickstep");
    let shade_def = s.content.mob(&s.content.rogue.shade).unwrap();
    let shade = *s.mobs.iter().find(|(_, x)| x.def == shade_def).map(|(k, _)| k).expect("a shade");
    assert!(dist2d(s.mobs[&shade].pos, ORIGIN) < 0.1, "where she stood");
    assert!(s.auras_of(shade).contains(&5215), "see-through");
    assert!(s.has_cc(m, "distract"), "the enemy on her turns to it");
    let left = s.cc_of(m).iter().find(|c| c.kind == "distract").map(|c| c.until - s.clock).unwrap();
    assert!((left - 1.5).abs() < 0.05, "for 1.5 sec: {left}");
    // She steps away; it keeps facing where the shade is.
    s.move_player(p, [0.0, 3.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.5);
    let to_shade = facing(s.mobs[&m].pos, ORIGIN);
    assert!(angle_between(s.mobs[&m].facing, to_shade) < 0.1, "it faces the shade, not her");
    run(&mut s, 1.6);
    assert!(!s.mobs.contains_key(&shade), "gone after 2 sec");
}

// --- Evasion and Adrenaline Rush --------------------------------------------------------------------

#[test]
fn evasion_widens_the_perfect_window_to_0_28_sec_and_a_dodge_costs_8() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    press(&mut s, p, "defense", false);
    run(&mut s, 0.4);
    let e = s.content.abilities[idx(&s, "blade_evasion")].effects[0].clone();
    assert_eq!((e.kind.as_str(), e.secs[1], e.amount), ("evasion", 5.0, 0.28));
    assert!(e.amount < s.content.dodge.evade_to, "inside the dodge's evade window");
    s.players.get_mut(&p).unwrap().stamina = 50.0;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].stamina, 50.0 - s.content.rogue.evasion_cost);
    assert_eq!(s.content.rogue.evasion_cost, 8.0);
    run(&mut s, 0.24);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(cut_open(&s, p), "0.24 sec in is perfect under Evasion");
    // 5 sec on, the window is the core's again.
    run(&mut s, 5.0);
    dodge_for(&mut s, p, 0.24);
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(!cut_open(&s, p));
    // Under it, a dodge with 10 stamina goes (8 needed, not 18).
    let mut s = world();
    let p = rogue(&mut s);
    press(&mut s, p, "defense", false);
    run(&mut s, 0.4);
    s.players.get_mut(&p).unwrap().stamina = 10.0;
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }));
}

#[test]
fn adrenaline_rush_makes_blows_and_dodges_free_for_6_sec() {
    let mut s = world();
    let p = rogue(&mut s);
    let _m = foe(&mut s);
    press(&mut s, p, "defense", true);
    run(&mut s, 0.4);
    let d = s.content.abilities[idx(&s, "adrenaline_rush")].clone();
    assert_eq!((d.valor, d.effects[0].kind.as_str(), d.effects[0].secs[1]), (50, "adrenaline", 6.0));
    s.players.get_mut(&p).unwrap().stamina = 30.0;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].stamina, 30.0, "a free dodge");
    run(&mut s, 0.6);
    s.players.get_mut(&p).unwrap().stamina = 30.0;
    s.input(p, Input::Attack);
    assert!(matches!(s.players[&p].action, Action::Swing { .. }));
    assert_eq!(s.players[&p].stamina, 30.0, "a free blow");
    run(&mut s, 6.0);
    s.players.get_mut(&p).unwrap().stamina = 30.0;
    s.input(p, Input::Dodge);
    assert!(s.players[&p].stamina < 30.0, "6 sec on, a dodge costs again");
}

// --- Blade Dance ------------------------------------------------------------------------------------

#[test]
fn blade_dance_counts_every_evading_dodge_as_perfect_and_cuts_the_striker() {
    let mut s = world();
    let p = rogue(&mut s);
    let m = foe(&mut s);
    press(&mut s, p, "ultimate", false);
    run(&mut s, 0.4);
    let e = s.content.abilities[idx(&s, "blade_dance")].effects[0].clone();
    assert_eq!((e.kind.as_str(), e.secs[1], e.amount), ("blade_dance", 8.0, 0.6));
    assert!(s.auras_of(p).contains(&5215), "the body flickers through it");
    s.players.get_mut(&p).unwrap().stamina = 40.0;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].stamina, 40.0, "free");
    // 0.28 sec in: only an evade by the core's rule, perfect in the dance.
    run(&mut s, 0.28);
    let hp = s.mobs[&m].health;
    s.events.clear();
    assert_eq!(blow(&mut s, m, p), Outcome::Dodged);
    assert!(cut_open(&s, p), "perfect");
    assert_eq!(flourish(&s, p), 1);
    let ev = std::mem::take(&mut s.events);
    assert!(s.mobs[&m].health < hp, "the striker is cut");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == m && *kit == s.content.backstab.kit)), "from behind: the backstab's burst");
    // A blow that lands still lands, and a dodge outside the evade misses nothing.
    run(&mut s, 0.5);
    assert_eq!(blow(&mut s, m, p), Outcome::Hit);
}

// --- the board ---------------------------------------------------------------------------------------

#[test]
fn the_board_takes_a_third_rogue_choice_and_no_other_class_a_third() {
    let s = world_with(false);
    let card = |k: &str| &s.content.abilities[idx(&s, k)];
    for (slot, k) in BLADES_KEYS {
        let d = card(k);
        assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice), ("rogue", slot, BLADES), "{k}");
        assert!(d.on_board() && !s.learned_where(k).is_empty(), "{k} is on the board and learned somewhere");
    }
    assert!(s.content.abilities.iter().filter(|a| a.choice == 3).all(|a| a.school == "rogue"), "only the Rogue has a third");
    // The check refuses a third to any other class.
    let mut c = s.content.clone();
    let k = c.ability("charge").unwrap();
    c.abilities[k].choice = 3;
    assert!(c.validate_abilities().is_err(), "a Warrior's third choice");
    // A new Rogue knows Darting Knife and Blade Flurry (a class's Q and E rows) and learns the rest
    // out in Elwynn.
    let mut s = world_with(false);
    let p = hero_of(&mut s, "Ayla", "rogue");
    let h = &s.players[&p];
    assert!(h.owned.contains("darting_knife") && h.owned.contains("blade_flurry"));
    assert!(!h.owned.contains("blade_dance"));
    for (k, chest) in [("quickstep", "relic_rogue"), ("blade_evasion", "relic_rogue"), ("darting_knife", "shrine_rogue"), ("blade_flurry", "shrine_rogue")] {
        assert!(s.content.chests.iter().any(|c| c.key == chest && c.teaches.iter().any(|t| t == k)), "{chest} teaches {k}");
    }
    assert!(s.content.mobs.iter().any(|m| m.teaches.iter().any(|t| t == "blade_dance")), "Morgan teaches Blade Dance");
}

#[test]
fn an_old_save_with_evasion_still_loads_as_smoke_bomb() {
    let mut s = world_with(false);
    let p = hero_of(&mut s, "Ayla", "rogue");
    let mut save = s.leave(p).unwrap();
    save.owned.push("evasion".into());
    save.loadout.insert("defense".into(), "evasion".into());
    let q = s.join("ayla", "Ayla", Some(&save));
    let h = &s.players[&q];
    assert_eq!(h.loadout.get("defense").map(String::as_str), Some("smoke_bomb"), "the old Evasion is Smoke Bomb");
    assert!(!h.owned.contains("blade_evasion"), "not the new one");
    assert!(s.content.ability("evasion").is_none(), "the new Evasion has its own key");
}

// --- words ------------------------------------------------------------------------------------------

#[test]
fn blades_cards_words_say_what_the_data_does() {
    let s = world();
    let r = &s.content.rogue;
    let said = |k: &str, words: &[String]| {
        let d = &s.content.abilities[idx(&s, k)];
        for w in words {
            assert!(d.text.contains(w.as_str()), "{k}: {w:?} in {}", d.text);
        }
    };
    let pct = |x: f32| format!("{}%", (x * 100.0).round());
    let flourish = [
        format!("With {} Blades abilities on your keys, each perfect dodge grants 1 Flourish for {} sec, up to {}", r.trait_keys, r.flourish_secs, r.flourish_most),
        format!("deal {} more damage and swing {} faster", pct(r.flourish_damage), pct(r.flourish_haste)),
        "A hit on you removes them all".into(),
    ];
    let d = &s.content.abilities[idx(&s, "darting_knife")];
    assert!(d.hand && d.charges == 3 && d.cooldown == 5.0);
    let mut w = vec![format!("for {} damage, up to {} yd", d.damage[1], d.range[1]), "counts as a strike in your two-weapon chain".into(), format!("Holds {} charges", d.charges)];
    w.extend(flourish.iter().cloned());
    said("darting_knife", &w);
    let d = &s.content.abilities[idx(&s, "pinning_knife")];
    said("pinning_knife", &[format!("for {} damage, up to {} yd", d.damage[1], d.range[1]), format!("For {} sec, the enemy can't turn", d.effects[0].secs[1]), "Bosses turn at half speed instead".into(), "Heroes ignore it".into()]);
    assert_eq!(r.pinned_boss_turn, 0.5);
    let d = &s.content.abilities[idx(&s, "blade_flurry")];
    let mut w = vec![format!("Strikes {} times, {} sec apart", d.cuts, r.flurry_gap), format!("stepping {} yd forward", r.flurry_step), format!("The {}th strike is with both blades", d.cuts), "A dodge between strikes ends it".into(), "Requires a weapon in each hand".into()];
    w.extend(flourish.iter().cloned());
    said("blade_flurry", &w);
    assert_eq!(d.cuts as u8, DUAL_FINISH_AT, "the last cut is the run's finisher");
    let d = &s.content.abilities[idx(&s, "hemorrhage")];
    said("hemorrhage", &[format!("{} sec windup", d.windup), format!("for {} weapon damage", pct(d.weapon)), format!("For {} sec, attacks from behind on it by any hero keep their full bonus", d.effects[0].secs[1])]);
    let d = &s.content.abilities[idx(&s, "quickstep")];
    said("quickstep", &[format!("Dashes {} yd", r.quick_speed * s.content.dodge.dash_secs), "costs no stamina".into(), "Cancels your attack and gives back its stamina during its windup".into(), format!("Holds {} charges", d.charges)]);
    let d = &s.content.abilities[idx(&s, "afterimage")];
    said("afterimage", &[format!("leaves a shade where you stood for {} sec", d.shade), format!("turns to face the shade for {} sec", d.shade_turn), "A boss only glances at it".into()]);
    let d = &s.content.abilities[idx(&s, "blade_evasion")];
    said("blade_evasion", &[format!("For {} sec, a dodge is perfect up to {} sec after you press it, not {} sec", d.effects[0].secs[1], d.effects[0].amount, s.content.dodge.perfect_to), format!("Dodging costs {} stamina", r.evasion_cost)]);
    assert_eq!(d.cooldown, 20.0);
    let d = &s.content.abilities[idx(&s, "adrenaline_rush")];
    said("adrenaline_rush", &[format!("For {} sec, attacks and dodges cost no stamina", d.effects[0].secs[1])]);
    let d = &s.content.abilities[idx(&s, "blade_dance")];
    assert_eq!((d.valor, d.cooldown), (100, 40.0));
    said("blade_dance", &[format!("For {} sec, every dodge that evades an attack is a perfect dodge and costs no stamina", d.effects[0].secs[1]), format!("cuts the attacker from behind for {} weapon damage", pct(d.effects[0].amount))]);
}
