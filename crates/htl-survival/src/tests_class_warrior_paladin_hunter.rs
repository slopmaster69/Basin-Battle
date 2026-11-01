//! The redesigned warrior, paladin and hunter cards (#40), each cast from the real content: the thing that makes it
//! play differently.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules (a dev build: every card owned), friendly fire on or off.
fn world_with(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(false)
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, facing the hero.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run on with these creatures held where they stand, idle (struck, they'd come at the hero).
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

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Put a card on its row's key (cooldowns off) and press the key (`ex`: with Ctrl). An aimed one
/// comes up and waits for the click.
fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

/// The key on that row pressed again, as it is (no slotting, no cooldowns touched).
fn press_again(s: &mut Survival, p: Id, slot: &str) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
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

/// The click held `secs` (the reticle kept on `aim`, facing and elevation, as the client keeps
/// saying it) and let go.
fn draw(s: &mut Survival, p: Id, secs: f32, aim: (f32, f32), mobs: &[Id]) -> Vec<Event> {
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        s.set_aim_line(p, aim.0, aim.1);
        ev.extend(run_pinned(s, DT, mobs));
    }
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::AttackUp);
    ev.extend(run_pinned(s, DT, mobs));
    ev
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
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

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn missiles(ev: &[Event], p: Id, spell: u32) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell: s, .. } if *caster == p && *s == spell)).count()
}

/// The elevation that looks at the ground this far ahead (from the eye, 1.9 yd up).
fn down_to(dist: f32) -> f32 {
    -(1.9f32).atan2(dist)
}

/// A hero's save from before the redesign: of this class, owning these old cards, each on its key
/// (slot, key). Loaded in a world without the dev build (so what's owned is what was saved).
fn load_old_save(class: &str, keys: &[(&str, &str)]) -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = false;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.class = class.to_string();
    save.owned = keys.iter().map(|(_, k)| k.to_string()).collect();
    save.loadout = keys.iter().map(|(slot, k)| (slot.to_string(), k.to_string())).collect();
    let q = s.join("ayla", "Ayla", Some(&save));
    (s, q)
}

/// The hero owns each new card, on its row's key, and nothing the rules don't know.
fn has_on_keys(s: &Survival, p: Id, cards: &[(&str, &str)]) {
    let h = &s.players[&p];
    for (slot, key) in cards {
        assert!(h.owned.contains(*key), "{key} owned: {:?}", h.owned);
        assert_eq!(h.loadout.get(*slot).map(String::as_str), Some(*key), "{key} on {slot}");
    }
    assert!(h.owned.iter().all(|k| s.content.ability(k).is_some()), "nothing unknown: {:?}", h.owned);
}

// --- Warrior: bulldoze ----------------------------------------------------------------------------

#[test]
fn hurled_axe_strikes_each_foe_going_out_and_again_coming_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = mob(&mut s, "brave", [6.0, 0.0, 0.0]);
    // (A little left of the line: the axe drifts left as it flies, from the right hand.)
    let far = mob(&mut s, "brave", [16.0, 0.4, 0.0]);
    let aside = mob(&mut s, "brave", [10.0, 6.0, 0.0]);
    let pins = [near, far, aside];
    press(&mut s, p, "hurled_axe", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "hurled_axe")));
    s.set_aim_line(p, 0.0, down_to(24.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 3.0, &pins));
    let turn = ev.iter().position(|e| matches!(e, Event::MissileFrom { caster, .. } if *caster == p)).expect("it turns and comes back");
    let (out, back) = ev.split_at(turn);
    for m in [near, far] {
        assert_eq!(hits(out, p, m).len(), 1, "struck going out, the far one too: it flies through");
        assert_eq!(hits(back, p, m).len(), 1, "and again coming back");
    }
    assert!(hits(&ev, p, aside).is_empty(), "not the one aside");
    assert!(s.projectiles.is_empty(), "caught");
}

#[test]
fn hurled_axe_comes_back_to_where_you_stand_now_so_a_step_lays_its_way_back_across_a_foe() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Well to the side of the throw.
    let side = mob(&mut s, "brave", [11.0, 5.0, 0.0]);
    press(&mut s, p, "hurled_axe", false);
    s.set_aim_line(p, 0.0, 0.0);
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 0.6, &[side]));
    assert!(hits(&ev, p, side).is_empty(), "out along the reticle, past it");
    // She steps across before it turns: it comes back to her through the foe.
    s.players.get_mut(&p).unwrap().pos = [0.0, 10.0, 0.0];
    ev.extend(run_pinned(&mut s, 2.5, &[side]));
    assert_eq!(hits(&ev, p, side).len(), 1, "struck on its way back to her");
    assert!(s.projectiles.is_empty(), "caught");
}

#[test]
fn rending_axe_drags_what_it_strikes_toward_you_going_and_coming_and_staggers_it() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [12.0, 0.0, 0.0], std::f32::consts::PI);
    s.set_valor(a, 25.0);
    press(&mut s, a, "hurled_axe", true);
    assert_eq!(aiming(&s, a), Some(idx(&s, "rending_axe")), "Ctrl: Rending Axe");
    s.set_aim_line(a, 0.0, 0.0);
    let mut ev = click(&mut s, a);
    ev.extend(run(&mut s, 2.0));
    let hops: Vec<f32> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Hop { player, dir, .. } if *player == b => Some(*dir),
            _ => None,
        })
        .collect();
    assert_eq!(hops.len(), 2, "pulled going out and coming back: {hops:?}");
    assert!(hops.iter().all(|d| d.cos() < -0.95), "toward her (along -x): {hops:?}");
    assert!(s.cc_log_of(b).iter().any(|l| l.contains("Rending Axe: stagger")), "staggered: {:?}", s.cc_log_of(b));
}

#[test]
fn charge_bowls_through_every_foe_in_its_line_staggering_each() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let first = mob(&mut s, "brave", [5.0, 0.0, 0.0]);
    let second = mob(&mut s, "brave", [12.0, 0.0, 0.0]);
    let (h1, h2) = (s.mobs[&first].health, s.mobs[&second].health);
    press(&mut s, p, "charge", false);
    s.set_aim_line(p, 0.0, 0.0);
    let mut ev = click(&mut s, p);
    let len = ev
        .iter()
        .find_map(|e| match e {
            Event::Dash { player, speed, secs, .. } if *player == p => Some(speed * secs),
            _ => None,
        })
        .expect("she charges");
    assert!(len > 15.0, "on past both, not stopped at the first: {len}");
    ev.extend(run(&mut s, 1.0));
    assert!(s.mobs[&first].health < h1 && s.mobs[&second].health < h2, "both struck");
    assert_eq!(hits(&ev, p, first).len() + hits(&ev, p, second).len(), 2, "each once");
    let staggers = s.cc_log_of(p).iter().filter(|l| l.contains("Charge: stagger 0.6")).count();
    assert_eq!(staggers, 2, "each staggered: {:?}", s.cc_log_of(p));
}

#[test]
fn an_old_warriors_save_comes_back_with_the_new_cards() {
    let (s, p) = load_old_save("warrior", &[("ranged", "aimed_throw"), ("close", "sundering_blow"), ("utility", "charge"), ("defense", "retaliation"), ("ultimate", "onslaught")]);
    has_on_keys(&s, p, &[("ranged", "hurled_axe"), ("close", "sundering_blow"), ("utility", "charge"), ("defense", "retaliation"), ("ultimate", "onslaught")]);
    assert_eq!(crate::save::renamed("aimed_throw"), "hurled_axe");
}

// --- Paladin: shield your friends, punish who touches them ----------------------------------------

#[test]
fn avengers_shield_ricochets_to_two_more_foes_each_blow_weaker_and_silences_them() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let first = mob(&mut s, "brave", [10.0, 0.0, 0.0]);
    let second = mob(&mut s, "brave", [14.0, 3.0, 0.0]);
    let third = mob(&mut s, "brave", [18.0, 6.0, 0.0]);
    let beyond = mob(&mut s, "brave", [30.0, -6.0, 0.0]);
    let pins = [first, second, third, beyond];
    press(&mut s, p, "avengers_shield", false);
    s.set_aim_line(p, 0.0, down_to(12.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert!(missiles(&ev, p, 24275) >= 1, "the shield thrown");
    let (a, b, c) = (hits(&ev, p, first), hits(&ev, p, second), hits(&ev, p, third));
    assert_eq!((a.len(), b.len(), c.len()), (1, 1, 1), "three struck, once each");
    assert!(a[0] > b[0] && b[0] > c[0], "each blow weaker: {a:?} {b:?} {c:?}");
    assert!(hits(&ev, p, beyond).is_empty(), "two leaps, no more");
    for m in [first, second, third] {
        assert!(s.has_cc(m, "silence"), "silenced");
    }
}

#[test]
fn holy_shock_alone_heals_and_frees_yourself() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let max = s.players[&p].max_health;
    s.players.get_mut(&p).unwrap().health = max / 2;
    s.apply_cc(p, p, "root", 4.0, 0.0, 0.0, "test");
    s.set_valor(p, 25.0);
    press(&mut s, p, "avengers_shield", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "holy_shock")), "Ctrl: Holy Shock");
    let ev = click(&mut s, p);
    let (got, want) = (s.players[&p].health - max / 2, (max as f32 * 0.25).round() as u32);
    assert!(got >= want && got <= want + 2, "a quarter of her health: {got}");
    assert!(!s.has_cc(p, "root"), "and free");
    assert_eq!(missiles(&ev, p, 24275), 0, "no shield thrown");
}

#[test]
fn divine_steed_rides_free_of_roots_and_its_key_again_throws_hammer_of_justice() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [10.0, 0.0, 0.0]);
    s.apply_cc(p, p, "root", 4.0, 0.0, 0.0, "test");
    s.apply_cc(p, p, "snare", 4.0, 0.5, 0.0, "test");
    let ev = press(&mut s, p, "divine_steed", false);
    assert!(missiles(&ev, p, 13819) >= 1, "its look");
    assert!(!s.has_cc(p, "root") && !s.has_cc(p, "snare"), "free of what held her");
    assert!(s.has_cc(p, "haste"), "and away at speed");
    let keys = s.recast_keys(p);
    assert_eq!(keys.len(), 1, "the key armed: {keys:?}");
    assert_eq!((keys[0].0, keys[0].1), (key_of("utility") as usize, idx(&s, "hammer_of_justice")));
    // The key again: the hammer comes up, and goes on the click.
    run(&mut s, 0.4);
    press_again(&mut s, p, "utility");
    assert_eq!(aiming(&s, p), Some(idx(&s, "hammer_of_justice")), "the hammer, aimed");
    s.set_aim_line(p, 0.0, down_to(12.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 0.8, &[k]));
    assert!(missiles(&ev, p, 20685) >= 1, "thrown");
    assert!(s.has_cc(k, "stun"), "and it stuns");
    assert!(s.recast_keys(p).is_empty(), "spent");
    // Left unused, the window closes and the key is the Steed again.
    run(&mut s, 2.0);
    press(&mut s, p, "divine_steed", false);
    run(&mut s, 3.2);
    assert!(s.recast_keys(p).is_empty(), "the window closed");
    let ev = press_again(&mut s, p, "utility");
    assert_eq!(aiming(&s, p), None, "no hammer");
    assert!(missiles(&ev, p, 13819) >= 1, "the Steed again");
}

#[test]
fn blessing_of_freedom_frees_a_friend_and_speeds_them_and_arms_no_hammer() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    s.apply_cc(b, b, "root", 4.0, 0.0, 0.0, "test");
    s.set_valor(a, 25.0);
    press(&mut s, a, "divine_steed", true);
    assert_eq!(aiming(&s, a), Some(idx(&s, "blessing_of_freedom")), "aimed at a friend");
    s.set_aim_line(a, 0.0, 0.0);
    let ev = click(&mut s, a);
    assert!(flashed(&ev, a, "Bren"), "on him");
    assert!(!s.has_cc(b, "root"), "free");
    assert!(s.has_cc(b, "haste") && s.has_cc(b, "steadfast"), "faster and steady");
    assert!(!s.has_cc(a, "haste"), "not her");
    assert!(s.recast_keys(a).is_empty(), "the EX arms no hammer");
}

#[test]
fn blessing_of_protection_halves_what_a_friend_takes_and_alone_it_is_yours() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    press(&mut s, a, "blessing_of_protection", false);
    assert_eq!(aiming(&s, a), Some(idx(&s, "blessing_of_protection")), "aimed at a friend, not a stance");
    s.set_aim_line(a, 0.0, 0.0);
    let ev = click(&mut s, a);
    assert!(missiles(&ev, a, 1022) >= 1);
    assert!((s.taken_share(b) - 0.5).abs() < 1.0e-3, "half of every blow on him");
    assert!(s.unstaggerable(b), "and nothing staggers him");
    assert!((s.taken_share(a) - 1.0).abs() < 1.0e-3, "not on her");
    run(&mut s, 3.2);
    assert!((s.taken_share(b) - 1.0).abs() < 1.0e-3, "for 3 s");
    // Alone, it's hers.
    let _ = s.leave(b);
    press(&mut s, a, "blessing_of_protection", false);
    click(&mut s, a);
    assert!((s.taken_share(a) - 0.5).abs() < 1.0e-3, "on herself");
}

#[test]
fn lay_on_hands_on_a_friend_heals_two_fifths_and_ends_what_is_on_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let max = s.players[&b].max_health;
    s.players.get_mut(&b).unwrap().health = max / 4;
    s.apply_cc(b, b, "snare", 5.0, 0.5, 0.0, "test");
    s.set_valor(a, 25.0);
    press(&mut s, a, "blessing_of_protection", true);
    assert_eq!(aiming(&s, a), Some(idx(&s, "lay_on_hands")));
    s.set_aim_line(a, 0.0, 0.0);
    let ev = click(&mut s, a);
    let (got, want) = (s.players[&b].health - max / 4, (max as f32 * 0.4).round() as u32);
    assert!(got >= want && got <= want + 2, "two fifths of his health: {got}");
    assert!(!s.has_cc(b, "snare"), "cleansed");
    assert!(flashed(&ev, a, "Healed Bren"));
}

#[test]
fn an_old_paladins_save_comes_back_with_the_new_cards() {
    let (s, p) = load_old_save("paladin", &[("ranged", "holy_shock"), ("close", "consecration"), ("utility", "hammer_of_justice"), ("defense", "divine_protection"), ("ultimate", "sanctuary")]);
    has_on_keys(&s, p, &[("ranged", "avengers_shield"), ("close", "consecration"), ("utility", "divine_steed"), ("defense", "blessing_of_protection"), ("ultimate", "sanctuary")]);
    // The old keys that still name something (an EX version, a recast) never come back as cards.
    let h = &s.players[&p];
    assert!(!h.owned.contains("holy_shock") && !h.owned.contains("hammer_of_justice"), "{:?}", h.owned);
}

// --- Hunter: set up, then snipe -------------------------------------------------------------------

#[test]
fn aimed_shot_drawn_longer_flies_farther_and_harder_and_a_full_draw_goes_through_the_line() {
    // Farther: a boar 33 yd off is out of a short draw's reach, not a full one's.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let far = mob(&mut s, "boar", [33.0, 0.0, 0.0]);
    let aim = (0.0, down_to(40.0));
    press(&mut s, p, "aimed_arrow", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "aimed_arrow")));
    let mut ev = draw(&mut s, p, 0.2, aim, &[far]);
    ev.extend(run_pinned(&mut s, 1.5, &[far]));
    assert!(hits(&ev, p, far).is_empty(), "a short draw falls short");
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 1.5, aim, &[far]);
    ev.extend(run_pinned(&mut s, 1.5, &[far]));
    assert_eq!(hits(&ev, p, far).len(), 1, "a full draw reaches it");

    // Through the line: a short draw stops at the first boar, a full one goes on into the second,
    // and hits harder.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = mob(&mut s, "boar", [8.0, 0.0, 0.0]);
    let behind = mob(&mut s, "boar", [13.0, 0.0, 0.0]);
    let pins = [near, behind];
    let aim = (0.0, down_to(20.0));
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 0.2, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    let short = hits(&ev, p, near);
    assert_eq!(short.len(), 1, "the first struck");
    assert!(hits(&ev, p, behind).is_empty(), "and it stops there");
    press(&mut s, p, "aimed_arrow", false);
    let mut ev = draw(&mut s, p, 1.5, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 3044), 1, "fully drawn: the glowing arrow");
    let full = hits(&ev, p, near);
    assert_eq!(full.len(), 1);
    assert!(full[0] > short[0], "harder: {short:?} then {full:?}");
    assert_eq!(hits(&ev, p, behind).len(), 1, "and on through into the one behind");
}

#[test]
fn a_heavy_blow_spoils_the_aimed_shot_draw() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    press(&mut s, a, "aimed_arrow", false);
    s.input(a, Input::Attack);
    run(&mut s, 0.5);
    assert!(matches!(s.players[&a].action, Action::Charge { .. }), "drawing");
    s.strike_player(b, a, 20, [2.0, 0.0, 0.0], true, true, true, Blow::Heavy);
    assert!(!matches!(s.players[&a].action, Action::Charge { .. }), "the draw is spoiled");
    let ev = run(&mut s, 1.0);
    assert_eq!(missiles(&ev, a, 75) + missiles(&ev, a, 3044), 0, "no arrow");
}

#[test]
fn multi_shot_looses_five_arrows_in_a_fan() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // One on each arrow's line, 7 yd out, 10° apart; one well outside the fan.
    let fan: Vec<Id> = [-20.0f32, -10.0, 0.0, 10.0, 20.0].iter().map(|deg| mob(&mut s, "grunt", [7.0 * deg.to_radians().cos(), 7.0 * deg.to_radians().sin(), 0.0])).collect();
    let wide = mob(&mut s, "grunt", [7.0 * 50f32.to_radians().cos(), 7.0 * 50f32.to_radians().sin(), 0.0]);
    let mut pins = fan.clone();
    pins.push(wide);
    s.set_valor(p, 25.0);
    press(&mut s, p, "aimed_arrow", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "multi_shot")), "Ctrl: the Multi-Shot");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 75), 5, "five arrows, no draw");
    for m in &fan {
        assert_eq!(hits(&ev, p, *m).len(), 1, "each arrow its own foe");
    }
    assert!(hits(&ev, p, wide).is_empty(), "nothing outside the fan");
}

#[test]
fn freezing_trap_readies_a_free_multi_shot_and_the_frozen_shatter() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    s.slot_ability(p, "ranged", "aimed_arrow").unwrap();
    s.set_valor(p, 50.0);
    // The Freezing Trap at its feet.
    press(&mut s, p, "explosive_trap", true);
    assert_eq!(aiming(&s, p), Some(idx(&s, "freezing_trap")));
    s.set_aim_line(p, 0.0, down_to(6.0));
    click(&mut s, p);
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "Aimed Shot's key readied: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize);
    assert!((em[0].1 - (s.clock + 8.0)).abs() < 0.3, "for 8 s");
    run_pinned(&mut s, 1.2, &[k]);
    assert!(s.has_cc(k, "petrify"), "sprung: frozen");
    // Aimed Shot's key: the Multi-Shot, free, and the frozen one shatters.
    let valor = s.players[&p].valor;
    press_again(&mut s, p, "ranged");
    assert_eq!(aiming(&s, p), Some(idx(&s, "multi_shot")), "the Multi-Shot comes up");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    ev.extend(run_pinned(&mut s, 1.0, &[k]));
    assert_eq!(missiles(&ev, p, 75), 5, "five arrows");
    assert!(s.players[&p].valor + 0.01 >= valor, "no Valor spent: {valor} then {}", s.players[&p].valor);
    assert!(flashed(&ev, p, "Shatter!"), "the ice shatters");
    assert!(!s.has_cc(k, "petrify"));
    assert!(s.empowered_keys(p).is_empty(), "spent");
    // The next press is the plain draw again.
    run(&mut s, 0.5);
    press_again(&mut s, p, "ranged");
    assert_eq!(aiming(&s, p), Some(idx(&s, "aimed_arrow")));
}

#[test]
fn flare_lobbed_over_blinds_the_foes_where_it_lands_until_they_are_struck() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let there = mob(&mut s, "grunt", [14.0, 0.0, 0.0]);
    let beside = mob(&mut s, "grunt", [14.0, 2.5, 0.0]);
    let away = mob(&mut s, "grunt", [14.0, 8.0, 0.0]);
    let pins = [there, beside, away];
    press(&mut s, p, "flare", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "flare")), "aimed, not a stance any more");
    s.set_aim_line(p, 0.0, down_to(14.0));
    let mut ev = click(&mut s, p);
    assert!(missiles(&ev, p, 2812) >= 1, "the flare flies");
    ev.extend(run_pinned(&mut s, 0.3, &pins));
    assert!(!s.has_cc(there, "incapacitate"), "still in the air");
    ev.extend(run_pinned(&mut s, 0.7, &pins));
    assert!(s.has_cc(there, "incapacitate") && s.has_cc(beside, "incapacitate"), "blinded where it landed");
    assert!(!s.has_cc(away, "incapacitate"), "not one well away");
    assert!(hits(&ev, p, there).is_empty(), "it does no harm");
    // A blow ends it.
    s.hurt_mob(there, p, 1);
    assert!(!s.has_cc(there, "incapacitate"), "struck: it sees again");
    assert!(s.has_cc(beside, "incapacitate"));
}

#[test]
fn kill_shot_wants_its_whole_draw_goes_through_the_line_and_finishes_the_weak() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let first = mob(&mut s, "brave", [6.0, 0.0, 0.0]);
    let weak = mob(&mut s, "brave", [11.0, 0.0, 0.0]);
    let last = mob(&mut s, "grunt", [16.0, 0.0, 0.0]);
    let pins = [first, weak, last];
    let max = s.mobs[&weak].max_health;
    s.mobs.get_mut(&weak).unwrap().health = (max as f32 * 0.24) as u32;
    let aim = (0.0, down_to(20.0));
    s.set_valor(p, 100.0);
    // Let go before its draw: lost, and nothing spent.
    press(&mut s, p, "kill_shot", false);
    let ev = draw(&mut s, p, 0.5, aim, &pins);
    assert_eq!(missiles(&ev, p, 3044), 0, "let go too soon");
    assert_eq!(s.players[&p].action, Action::Ready);
    assert_eq!(s.players[&p].valor, 100.0, "nothing spent");
    // The whole draw.
    press(&mut s, p, "kill_shot", false);
    let mut ev = draw(&mut s, p, 1.1, aim, &pins);
    ev.extend(run_pinned(&mut s, 1.0, &pins));
    assert_eq!(missiles(&ev, p, 3044), 1, "loosed");
    assert_eq!(hits(&ev, p, first).len(), 1, "the first struck");
    assert!(s.alive(first), "a hale one lives");
    assert!(!s.alive(weak), "the weak one finished");
    assert!(flashed(&ev, p, "Execute!"));
    assert_eq!(hits(&ev, p, last).len(), 1, "and on through to the last");
}

#[test]
fn an_old_hunters_save_comes_back_with_the_new_cards() {
    let (s, p) = load_old_save("hunter", &[("ranged", "arcane_shot"), ("close", "explosive_trap"), ("utility", "disengage"), ("defense", "deterrence"), ("ultimate", "volley")]);
    has_on_keys(&s, p, &[("ranged", "aimed_arrow"), ("close", "explosive_trap"), ("utility", "disengage"), ("defense", "flare"), ("ultimate", "kill_shot")]);
    assert_eq!(crate::save::renamed("volley"), "kill_shot");
}
