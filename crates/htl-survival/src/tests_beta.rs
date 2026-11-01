//! The board's beta page: generic cards any class can take for free and slot on its own row's key,
//! owned by all in a dev build, left out of Siege, kept by a save; and each card doing the one
//! thing it's there for (cast from the real content).

use crate::content::{AbilityDef, Content, BETA, BETA_ROW, SCHOOLS, SLOTS};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// A world of the built-in rules: a dev build (every card owned) or not, friendly fire or not.
fn world(dev: bool, pvp: bool) -> Survival {
    let mut c = Content::dev();
    c.dev.full_tree = dev;
    if !pvp {
        c.pvp = Default::default();
    }
    // The grunts stand and wait till struck (no noticing a hero close by and running at them).
    if let Some(k) = c.mob("grunt") {
        c.mobs[k].aggro = 0.0;
    }
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

/// A creature that stands and waits, facing the hero at the origin.
fn mob(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
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

fn key_of(slot: &str) -> u8 {
    SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Own a card, put it on its row's key, cooldowns off and Valor full.
fn ready(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.set_valor(p, 100.0);
}

/// Press a row's key (`ex`: with Ctrl) and, if it's aimed, click: it goes. The events of the step.
fn cast(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
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

/// The damage dealt to `victim` in these events.
fn dealt(events: &[Event], victim: Id) -> u32 {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome: Outcome::Hit, .. } if *v == victim => Some(*damage),
            _ => None,
        })
        .sum()
}

/// The blows that landed on `victim`, one by one.
fn blows(events: &[Event], victim: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome: Outcome::Hit, .. } if *v == victim && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

/// Whether a creature was sent to within `within` of `to` (a pull's end; not its chase after).
fn moved_near(events: &[Event], id: Id, to: Vec3, within: f32) -> bool {
    events.iter().any(|e| matches!(e, Event::MobMove { id: m, path, .. } if *m == id && path.last().is_some_and(|end| dist2d(*end, to) < within)))
}

// --- the page ---------------------------------------------------------------------------------------

#[test]
fn the_beta_page_is_four_cards_a_row_at_most_and_no_class_column() {
    let s = world(true, false);
    let c = &s.content;
    for row in SLOTS {
        let beta: Vec<&AbilityDef> = c.abilities.iter().filter(|a| a.on_board() && a.is_beta() && a.slot == row).collect();
        assert!(!beta.is_empty() && beta.len() <= BETA_ROW, "{row}: {} beta cards", beta.len());
    }
    for a in c.abilities.iter().filter(|a| a.on_board()) {
        assert_eq!(a.page(), if a.school == BETA { 2 } else { 1 }, "{}", a.key);
        assert_eq!(a.is_beta(), !SCHOOLS.contains(&a.school.as_str()), "{}: a class's or the beta page's", a.key);
    }
    // A fifth beta card on a row is refused as the rules load.
    let mut c = Content::dev();
    let n = c.abilities.iter().filter(|a| a.on_board() && a.is_beta() && a.slot == "ranged").count();
    let extra = |k: usize| -> AbilityDef {
        toml::from_str(&format!("key = \"t_beta_{k}\"\nname = \"Beta {k}\"\nschool = \"beta\"\nslot = \"ranged\"\nkind = \"projectile\"\nspeed = 20.0\nspell = 1\n")).unwrap()
    };
    for k in n..BETA_ROW {
        c.abilities.push(extra(k));
    }
    assert!(c.validate_abilities().is_ok(), "four to a row");
    c.abilities.push(extra(BETA_ROW));
    assert!(c.validate_abilities().is_err(), "a fifth");
}

#[test]
fn a_warrior_and_a_mage_both_take_and_slot_a_beta_card_for_free() {
    let mut s = world(false, false);
    let w = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let save = s.fresh_save("bren", "Bren", "mage", HeroLook::default());
    let m = s.join("bren", "Bren", Some(&save));
    assert_eq!((s.players[&w].class.as_str(), s.players[&m].class.as_str()), ("warrior", "mage"));
    for (p, key, row) in [(w, "ricochet", "ranged"), (m, "ricochet", "ranged"), (w, "bubble", "defense"), (m, "whirlwind", "close")] {
        assert!(s.slot_ability(p, row, key).is_err(), "not theirs yet");
        // Not learned anywhere, and no debug: taken straight from the board.
        assert!(!s.debug);
        assert!(s.reachable(&s.players[&p].owned, key), "{key} can be taken");
        s.take_node(p, key).unwrap();
        assert!(s.players[&p].owned.contains(key));
        // Only on its own row's key.
        let wrong = if row == "ranged" { "close" } else { "ranged" };
        assert!(s.slot_ability(p, wrong, key).is_err(), "{key} on {wrong}");
        s.slot_ability(p, row, key).unwrap();
        assert_eq!(s.players[&p].loadout.get(row).map(String::as_str), Some(key));
    }
    // A class card still isn't taken from the board (it's learned out in Elwynn).
    let class_card = s.content.abilities.iter().find(|a| a.on_board() && !a.is_beta() && !s.players[&w].owned.contains(&a.key)).map(|a| a.key.clone()).unwrap();
    assert!(s.take_node(w, &class_card).is_err(), "{class_card}");
    // And it goes off from the key like any card.
    s.set_cooldowns(m, true);
    cast(&mut s, m, "close", false);
    assert!(matches!(s.players[&m].action, Action::Channel { .. }), "the mage spins");
}

#[test]
fn a_dev_build_owns_the_beta_page_but_keeps_its_class_on_the_keys() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let beta: Vec<String> = s.content.abilities.iter().filter(|a| a.on_board() && a.is_beta()).map(|a| a.key.clone()).collect();
    assert!(beta.len() >= 12);
    for k in &beta {
        assert!(s.players[&p].owned.contains(k), "{k} owned");
        assert!(!s.players[&p].loadout.values().any(|v| v == k), "{k} not on a key by itself");
    }
    // `.es tree full` (dev) gives them too, from a fresh start.
    let mut s = world(false, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    assert!(!beta.iter().any(|k| s.players[&p].owned.contains(k)), "a hero starts with none");
    s.set_tree(p, true);
    assert!(beta.iter().all(|k| s.players[&p].owned.contains(k)));
}

#[test]
fn siege_leaves_the_beta_page_out() {
    // A dev build's hero arrives fresh: every class card, none of the beta page.
    let mut c = Content::dev();
    c.dev.full_tree = true;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    let p = s.join("ayla", "Ayla", None);
    s.siege_arrive(p);
    let owned = s.players[&p].owned.clone();
    assert!(owned.len() > 2, "a dev build's class cards: {owned:?}");
    for a in s.content.abilities.iter().filter(|a| a.on_board() && a.is_beta()) {
        assert!(!owned.contains(&a.key), "{} in a Siege kit", a.key);
        assert!(s.card_price(&a.key).is_none(), "{} has no price", a.key);
    }
    let why = s.take_node(p, "ricochet").unwrap_err();
    assert!(why.contains("Siege"), "{why}");
    // The Battle Trainer sells only the class's own.
    let mut c = Content::dev();
    c.dev.full_tree = false;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    let p = s.join("ayla", "Ayla", None);
    s.siege_arrive(p);
    let sold = s.siege_trainer(p);
    assert!(!sold.is_empty());
    for (k, ..) in &sold {
        assert!(!s.content.abilities[idx(&s, k)].is_beta(), "{k} at the trainer");
    }
}

#[test]
fn a_save_keeps_a_slotted_beta_card() {
    let mut s = world(false, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.take_node(p, "whirlwind").unwrap();
    s.slot_ability(p, "close", "whirlwind").unwrap();
    let save = s.leave(p).unwrap();
    assert!(save.owned.iter().any(|k| k == "whirlwind"));
    let q = s.join("ayla", "Ayla", Some(&save));
    assert!(s.players[&q].owned.contains("whirlwind"));
    assert_eq!(s.players[&q].loadout.get("close").map(String::as_str), Some("whirlwind"), "still on E");
}

// --- the cards ---------------------------------------------------------------------------------------

#[test]
fn ricochet_bounces_twice_on_and_hits_harder_each_time() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, [6.0, 0.0, 0.0]);
    let b = mob(&mut s, [12.0, 0.0, 0.0]);
    let c = mob(&mut s, [12.0, 6.0, 0.0]);
    ready(&mut s, p, "ricochet");
    let mut ev = cast(&mut s, p, "ranged", false);
    ev.extend(run(&mut s, 0.6));
    let (da, db, dc) = (dealt(&ev, a), dealt(&ev, b), dealt(&ev, c));
    assert!(da > 0 && da < db && db < dc, "harder each bounce: {da}, {db}, {dc}");
}

#[test]
fn gravity_lance_drags_the_crowd_onto_the_one_it_strikes() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let struck = mob(&mut s, [10.0, 0.0, 0.0]);
    let near = mob(&mut s, [10.0, 4.0, 0.0]);
    let far = mob(&mut s, [10.0, 12.0, 0.0]);
    ready(&mut s, p, "gravity_lance");
    let mut ev = cast(&mut s, p, "ranged", false);
    ev.extend(run(&mut s, 0.4));
    assert!(dealt(&ev, struck) > 0 && dealt(&ev, near) > 0, "both take it");
    assert!(moved_near(&ev, near, [10.0, 0.0, 0.0], 3.0), "the one nearby is dragged into the one it struck");
    assert_eq!(dealt(&ev, far), 0, "out of reach");
}

#[test]
fn whirlwind_carves_round_you_as_you_move() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let front = mob(&mut s, [3.0, 0.0, 0.0]);
    let side = mob(&mut s, [0.5, 3.0, 0.0]);
    let behind = mob(&mut s, [-3.0, 0.0, 0.0]);
    ready(&mut s, p, "whirlwind");
    let mut ev = cast(&mut s, p, "close", false);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "spinning");
    // Faster than an ordinary channel's walk.
    assert!(s.has_cc(p, "haste"));
    assert!(s.speed_for(p) > s.content.player.aim_speed * 1.4, "near a run");
    ev.extend(run(&mut s, 1.1));
    assert!(dealt(&ev, front) >= 10 && dealt(&ev, side) >= 10, "a tick each half second: {} {}", dealt(&ev, front), dealt(&ev, side));
    assert_eq!(dealt(&ev, behind), 0, "not what's well behind");
}

#[test]
fn launcher_throws_them_up_and_its_recast_slams_down_on_them() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, [3.0, 0.0, 0.0]);
    ready(&mut s, p, "launcher");
    let mut ev = cast(&mut s, p, "close", false);
    assert!(s.has_cc(m, "stun"), "up in the air");
    assert!(s.recast_keys(p).iter().any(|(n, r, _)| *n == 1 && *r == idx(&s, "launcher_slam")), "E armed for the Slam");
    // Its beat over, E again within the window: aimed, and away.
    ev.extend(run(&mut s, 0.4));
    ev.extend(cast(&mut s, p, "close", false));
    ev.extend(run(&mut s, 1.2));
    let hits = blows(&ev, m);
    assert!(hits.contains(&10) && hits.contains(&14), "the uppercut, then the Slam: {hits:?}");
    assert!(s.recast_keys(p).is_empty(), "the window spent");
}

#[test]
fn siphon_blade_heals_you_a_share_of_all_it_deals() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, [2.5, 0.0, 0.0]);
    let b = mob(&mut s, [2.5, 1.2, 0.0]);
    ready(&mut s, p, "siphon_blade");
    s.players.get_mut(&p).unwrap().health = 50;
    let ev = cast(&mut s, p, "close", false);
    assert_eq!((dealt(&ev, a), dealt(&ev, b)), (12, 12));
    // 60% of each 12 (7 a foe).
    let hp = s.players[&p].health;
    assert!((64..70).contains(&hp), "healed 14: {hp}");
}

#[test]
fn blink_dagger_blinks_and_blinks_again() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    ready(&mut s, p, "blink_dagger");
    cast(&mut s, p, "utility", false);
    let x = s.players[&p].pos[0];
    assert!((10.0..=12.5).contains(&x), "12 yd: {x}");
    assert!(!s.recast_keys(p).is_empty(), "armed for the second");
    run(&mut s, 0.4);
    cast(&mut s, p, "utility", false);
    let x2 = s.players[&p].pos[0];
    assert!(x2 > x + 8.0, "and again: {x2}");
    assert!(s.recast_keys(p).is_empty(), "two blinks, no third");
}

#[test]
fn phase_dash_runs_through_foes_and_leaves_them_burning() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, [3.0, 0.0, 0.0]);
    let b = mob(&mut s, [6.0, 0.3, 0.0]);
    ready(&mut s, p, "phase_dash");
    let mut ev = cast(&mut s, p, "utility", false);
    // (The run itself is the hero's own client's: its length is what the sim sends it.)
    let len = ev.iter().find_map(|e| match e {
        Event::Dash { player, speed, secs, .. } if *player == p => Some(speed * secs),
        _ => None,
    });
    assert!(len.is_some_and(|l| l > 8.0), "through them both, on to its range: {len:?}");
    ev.extend(run(&mut s, 0.6));
    for m in [a, b] {
        assert!(dealt(&ev, m) >= 4, "struck in passing");
        assert!(s.has_cc(m, "burn"), "burning");
    }
}

#[test]
fn riposte_turns_the_blow_aside_and_cuts_the_attacker() {
    let mut s = world(true, true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [5.0, 0.0, 0.0], std::f32::consts::PI);
    ready(&mut s, a, "riposte");
    ready(&mut s, b, "ricochet");
    s.set_valor(a, 0.0);
    let hp = s.players[&a].health;
    cast(&mut s, a, "defense", false);
    assert!(matches!(s.players[&a].action, Action::Stance { .. }), "on guard");
    let mut ev = cast(&mut s, b, "ranged", false);
    ev.extend(run(&mut s, 0.3));
    assert_eq!(s.players[&a].health, hp, "turned aside");
    assert!(s.players[&a].valor >= 15.0, "a counter's Valor");
    assert!(!matches!(s.players[&a].action, Action::Stance { .. }), "it ends on the catch");
    assert!(s.has_cc(b, "burn"), "the attacker bleeds");
    ev.extend(run(&mut s, 1.1));
    assert!(dealt(&ev, b) >= 7, "the cut bleeds: {}", dealt(&ev, b));
}

#[test]
fn bubble_wraps_the_friend_you_aim_at_or_yourself() {
    let mut s = world(true, true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [8.0, 0.0, 0.0], std::f32::consts::PI);
    ready(&mut s, a, "bubble");
    cast(&mut s, a, "defense", false);
    assert!(s.has_cc(b, "shield"), "the friend under the reticle");
    assert!(!s.has_cc(a, "shield"));
    // Alone: on yourself.
    s.leave(b);
    run(&mut s, 0.4);
    cast(&mut s, a, "defense", false);
    assert!(s.has_cc(a, "shield"));
}

#[test]
fn black_hole_drags_foes_to_its_middle() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let aim = mob(&mut s, [12.0, 0.0, 0.0]);
    let near = mob(&mut s, [12.0, 5.0, 0.0]);
    ready(&mut s, p, "black_hole");
    let mut ev = cast(&mut s, p, "ultimate", false);
    assert_eq!(s.players[&p].valor, 0.0, "the whole bar");
    // Its flight (12 yd at 15 a second), then a tick.
    ev.extend(run(&mut s, 2.0));
    assert!(dealt(&ev, near) > 0, "it burns in it");
    assert!(moved_near(&ev, near, [12.0, 0.0, 0.0], 3.0), "dragged to its middle");
    assert!(s.mobs.contains_key(&aim));
}

#[test]
fn earthsplitter_breaks_open_along_your_aim_a_moment_later() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = mob(&mut s, [10.0, 0.0, 0.0]);
    let far = mob(&mut s, [25.0, 0.5, 0.0]);
    let aside = mob(&mut s, [10.0, 4.0, 0.0]);
    ready(&mut s, p, "earthsplitter");
    let mut ev = cast(&mut s, p, "ultimate", false);
    ev.extend(run(&mut s, 0.4));
    assert_eq!(dealt(&ev, near), 0, "not yet: the line is drawn first");
    ev.extend(run(&mut s, 0.6));
    for m in [near, far] {
        assert_eq!(dealt(&ev, m), 30);
        assert!(s.has_cc(m, "stun"));
    }
    assert_eq!(dealt(&ev, aside), 0, "off the line");
}

#[test]
fn avatar_makes_you_unstoppable_faster_and_harder_to_hurt() {
    let mut s = world(true, false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    ready(&mut s, p, "avatar");
    cast(&mut s, p, "ultimate", false);
    for k in ["onslaught", "haste", "hallowed"] {
        assert!(s.has_cc(p, k), "{k}");
    }
    assert!(s.unstaggerable(p));
    run(&mut s, 8.2);
    assert!(!s.has_cc(p, "onslaught"), "for 8 s");
}
