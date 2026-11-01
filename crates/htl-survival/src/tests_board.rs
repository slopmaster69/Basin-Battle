//! The board's new cards, one rule each: the weapon arts (Sundering Blow and its crushed guard,
//! the Unstoppable Blow no guard stops, Backstab from behind and on a Mark, Ambush), light that
//! heals a friend, holy ground, Onslaught, the Death Mark, Sanctuary, Cold Snap, Dread Step, the
//! shatter, an old save's keys put on the board's rows, the defense row (Lay on Hands, Shield Wall,
//! Dread Ward) and a full charge's flare and faster missile.

use std::collections::BTreeMap;

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

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

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3, facing: f32) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = facing;
    id
}

fn calm(s: &mut Survival, id: Id) {
    if let Some(m) = s.mobs.get_mut(&id) {
        m.state = MobState::Idle { wander_at: 1e9 };
        m.path.clear();
    }
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

/// Put a card on its row's key and use it (an aimed one: a tap of the click).
fn cast(s: &mut Survival, p: Id, slot: &str, key: &str, ex: bool) -> Vec<Event> {
    s.slot_ability(p, slot, key).unwrap();
    s.input(p, Input::Ability(key_of(slot), ex));
    let mut ev = Vec::new();
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(run(s, 0.05));
        s.input(p, Input::AttackUp);
    }
    ev
}

/// Run on with a creature held where it stands, facing one way (it would turn to the hero).
fn run_held(s: &mut Survival, k: Id, facing: f32, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        calm(s, k);
        if let Some(m) = s.mobs.get_mut(&k) {
            m.facing = facing;
        }
        out.extend(s.step(DT));
    }
    out
}

fn hits_on(events: &[Event], victim: Id) -> Vec<(u32, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, anim: false, .. } if *v == victim => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

fn flashed(events: &[Event], p: Id, text: &str) -> bool {
    events.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

/// The hero's weapon's damage roll, least and most.
fn weapon_roll(s: &Survival, p: Id) -> (f32, f32) {
    let w = s.weapon_of(&s.players[&p]);
    (w.damage[0] as f32, w.damage[1] as f32)
}

#[test]
fn sundering_blow_winds_up_hits_for_its_share_of_the_weapon_and_staggers() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.5, 0.0, 0.0], std::f32::consts::PI);
    let stamina = s.players[&p].stamina;
    let ev = cast(&mut s, p, "close", "sundering_blow", false);
    assert!(matches!(s.players[&p].action, Action::Swing { ref strike, .. } if strike.art.is_some()), "a swing of its own");
    assert!(s.players[&p].stamina < stamina, "its stamina");
    assert!(hits_on(&ev, k).is_empty(), "nothing before its wind-up");
    // (The brave held where it stands: it would come at the hero meanwhile.)
    let ev = run_held(&mut s, k, std::f32::consts::PI, 0.5);
    assert!(hits_on(&ev, k).is_empty(), "still winding up at 0.5 s");
    let ev = run_held(&mut s, k, std::f32::consts::PI, 0.5);
    let hit = hits_on(&ev, k);
    assert_eq!(hit.len(), 1, "{hit:?}");
    let (lo, hi) = weapon_roll(&s, p);
    assert!(hit[0].0 as f32 >= lo * 2.2 - 1.0 && hit[0].0 as f32 <= hi * 2.2 + 1.0, "220% of the weapon: {}", hit[0].0);
    assert!(s.cc_log_of(p).iter().any(|l| l.contains("stagger")), "it staggers");
    // Too winded for another.
    calm(&mut s, k);
    run(&mut s, 2.0);
    s.set_cooldowns(p, true);
    s.players.get_mut(&p).unwrap().stamina = 5.0;
    let ev = cast(&mut s, p, "close", "sundering_blow", false);
    assert!(ev.is_empty() && !matches!(s.players[&p].action, Action::Swing { .. }), "winded");
}

#[test]
fn a_guard_that_blocks_a_sundering_blow_is_crushed_and_the_unstoppable_one_goes_through_it() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    s.input(b, Input::Guard);
    run(&mut s, 0.4);
    let before = s.players[&b].stamina;
    cast(&mut s, a, "close", "sundering_blow", false);
    let ev = run(&mut s, 1.0);
    let hit = hits_on(&ev, b);
    assert!(hit.iter().any(|h| matches!(h.1, Outcome::Blocked | Outcome::GuardBroken)), "{hit:?}");
    assert!(before - s.players[&b].stamina > 40.0 || matches!(s.players[&b].action, Action::Stagger { .. }), "crushed: {before} to {}", s.players[&b].stamina);
    // The Unstoppable Blow: Bren is warned (a red weapon), and his guard doesn't stop it.
    run(&mut s, 3.0);
    s.players.get_mut(&b).unwrap().stamina = 100.0;
    s.input(b, Input::GuardUp);
    s.input(b, Input::Guard);
    run(&mut s, 0.5);
    s.set_cooldowns(a, true);
    s.set_valor(a, 50.0);
    let health = s.players[&b].health;
    let mut ev = cast(&mut s, a, "close", "sundering_blow", true);
    ev.extend(run(&mut s, 1.0));
    assert!(ev.iter().any(|e| matches!(e, Event::Danger { player, from: Some(f), .. } if *player == b && *f == a)), "warned");
    assert!(hits_on(&ev, b).iter().any(|h| h.1 == Outcome::Hit), "no guard stops it: {:?}", hits_on(&ev, b));
    assert!(s.players[&b].health < health);
}

#[test]
fn backstab_does_three_times_from_behind_and_crits_on_a_mark_and_ambush_steps_behind() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_cooldowns(p, true);
    let (lo, hi) = weapon_roll(&s, p);
    // Facing away from her: from behind.
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0], 0.0);
    cast(&mut s, p, "close", "backstab", false);
    let ev = run_held(&mut s, k, 0.0, 0.6);
    let hit = hits_on(&ev, k);
    assert!(hit[0].0 as f32 >= lo * 1.2 * 3.0 - 1.0, "from behind: {}", hit[0].0);
    assert!(flashed(&ev, p, "Backstab"));
    // Face to face: its plain share.
    run_held(&mut s, k, std::f32::consts::PI, 0.6);
    cast(&mut s, p, "close", "backstab", false);
    let ev = run_held(&mut s, k, std::f32::consts::PI, 0.6);
    let hit = hits_on(&ev, k);
    assert!((hit[0].0 as f32) <= hi * 1.2 * 1.3 + 1.0, "from the front: {}", hit[0].0);
    // A Mark makes it a critical hit.
    run_held(&mut s, k, std::f32::consts::PI, 0.6);
    s.apply_cc(k, p, "mark", 8.0, 0.15, 0.0, "test");
    cast(&mut s, p, "close", "backstab", false);
    let ev = run_held(&mut s, k, std::f32::consts::PI, 0.6);
    assert!(flashed(&ev, p, "Critical"));
    // Ambush: a step behind the foe on the reticle first.
    calm(&mut s, k);
    let far = mob(&mut s, "brave", [9.0, 0.0, 0.0], std::f32::consts::PI);
    s.mobs.get_mut(&k).unwrap().pos = [0.0, 20.0, 0.0];
    run_held(&mut s, far, std::f32::consts::PI, 1.5);
    s.set_valor(p, 100.0);
    s.set_aim_line(p, 0.0, 0.0);
    cast(&mut s, p, "close", "backstab", true);
    let at = s.players[&p].pos;
    assert!(at[0] > 9.5, "behind it: {at:?}");
    let ev = run_held(&mut s, far, std::f32::consts::PI, 0.6);
    assert!(hits_on(&ev, far)[0].0 as f32 >= lo * 1.4 * 3.0 - 1.0, "always from behind");
}

#[test]
fn holy_shock_heals_the_friend_under_the_reticle_and_ends_what_holds_them_and_the_shield_strikes_a_foe() {
    let mut s = world_with(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [8.0, 0.0, 0.0], 0.0);
    s.set_cooldowns(a, true);
    let max = s.players[&b].max_health;
    s.players.get_mut(&b).unwrap().health = max / 2;
    // Avenger's Shield's EX, Holy Shock: on the friend under the reticle, at once.
    s.set_valor(a, 50.0);
    s.set_aim_line(a, 0.0, 0.0);
    cast(&mut s, a, "ranged", "avengers_shield", true);
    let healed = s.players[&b].health - max / 2;
    assert_eq!(healed, (max as f32 * 0.25).round() as u32, "a quarter of his health");
    // A foe takes the shield as a blow.
    let k = mob(&mut s, "grunt", [8.0, 6.0, 0.0], 0.0);
    s.players.get_mut(&b).unwrap().pos = [0.0, -20.0, 0.0];
    s.set_aim_line(a, (6.0f32).atan2(8.0), 0.0);
    let mut ev = cast(&mut s, a, "ranged", "avengers_shield", false);
    ev.extend(run(&mut s, 1.0));
    assert!(hits_on(&ev, k).iter().any(|h| h.0 > 0));
    // Holy Shock ends a root on a friend.
    calm(&mut s, k);
    s.mobs.get_mut(&k).unwrap().pos = [0.0, 30.0, 0.0];
    s.players.get_mut(&b).unwrap().pos = [8.0, 0.0, 0.0];
    s.apply_cc(b, k, "root", 4.0, 0.0, 0.0, "test");
    assert!(s.has_cc(b, "root"));
    s.set_valor(a, 50.0);
    s.set_aim_line(a, 0.0, 0.0);
    cast(&mut s, a, "ranged", "avengers_shield", true);
    run(&mut s, 0.2);
    assert!(!s.has_cc(b, "root"), "set free");
}

#[test]
fn consecration_burns_foes_and_heals_friends_and_hallowed_ground_softens_their_blows() {
    let mut s = world_with(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    s.set_cooldowns(a, true);
    let k = mob(&mut s, "brave", [0.0, 4.0, 0.0], 0.0);
    let max = s.players[&a].max_health;
    s.players.get_mut(&a).unwrap().health = max / 2;
    s.players.get_mut(&b).unwrap().health = max / 2;
    cast(&mut s, a, "close", "consecration", false);
    let mob_hp = s.mobs[&k].health;
    let ev = run_held(&mut s, k, 0.0, 3.2);
    assert!(s.mobs[&k].health < mob_hp, "the foe burns");
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { .. })));
    assert!(s.players[&a].health > max / 2 && s.players[&b].health > max / 2, "friends heal");
    // Hallowed: a blow on a friend standing in it lands 30% lighter.
    run_held(&mut s, k, 0.0, 4.0);
    s.players.get_mut(&b).unwrap().health = max;
    s.set_valor(a, 50.0);
    cast(&mut s, a, "close", "consecration", true);
    run_held(&mut s, k, 0.0, 1.1);
    assert!(s.has_cc(b, "hallowed"));
    let before = s.players[&b].health;
    let landed = before - {
        s.strike_player(k, b, 100, [3.0, 4.0, 0.0], false, false, false, Blow::Light);
        s.players[&b].health
    };
    let armor = 1.0 - s.armor_reduce(&s.players[&b]);
    assert!((landed as f32 - 100.0 * 0.7 * armor).abs() <= 1.5, "{landed} of 100");
}

#[test]
fn onslaught_spends_no_stamina_and_nothing_staggers_it() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    s.set_valor(a, 100.0);
    cast(&mut s, a, "ultimate", "onslaught", false);
    assert_eq!(s.players[&a].valor, 0.0, "the whole bar");
    assert!(s.has_cc(a, "onslaught"));
    run(&mut s, 0.5);
    let stamina = s.players[&a].stamina;
    s.input(a, Input::Attack);
    s.input(a, Input::AttackUp);
    run(&mut s, 0.2);
    assert_eq!(s.players[&a].stamina, stamina, "no stamina spent");
    s.apply_cc(a, b, "stagger", 1.0, 0.0, 0.0, "test");
    assert!(!matches!(s.players[&a].action, Action::Stagger { .. }), "not staggered");
    // Its heavy attack goes through a guard.
    run(&mut s, 1.0);
    s.input(b, Input::Guard);
    run(&mut s, 0.4);
    s.input(a, Input::Heavy);
    let ev = run(&mut s, 1.5);
    assert!(hits_on(&ev, b).iter().any(|h| h.1 == Outcome::Hit), "{:?}", hits_on(&ev, b));
    // It runs out.
    run(&mut s, 5.0);
    assert!(!s.has_cc(a, "onslaught"));
}

#[test]
fn a_death_mark_makes_everything_land_harder_and_a_kill_under_it_readies_every_cooldown() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [10.0, 0.0, 0.0], 0.0);
    s.set_valor(p, 100.0);
    s.set_aim_line(p, 0.0, 0.0);
    cast(&mut s, p, "ultimate", "death_mark", false);
    run(&mut s, 0.6);
    assert!(s.has_cc(k, "deathmark"));
    assert!((s.taken_share(k) - 1.3).abs() < 1.0e-3);
    calm(&mut s, k);
    // A cooldown spent, then the kill.
    cast(&mut s, p, "ranged", "hurled_axe", false);
    assert_eq!(s.cooldown(p, s.content.ability("hurled_axe").unwrap()).charges, 0);
    s.mobs.get_mut(&k).unwrap().health = 1;
    let ev = run(&mut s, 1.0);
    assert!(!s.alive(k));
    assert!(flashed(&ev, p, "Death Mark"));
    assert!(s.players[&p].cooldowns.is_empty(), "every cooldown ready");
}

#[test]
fn sanctuary_mends_and_steadies_friends_and_slows_foes() {
    let mut s = world_with(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [4.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "grunt", [0.0, 5.0, 0.0], 0.0);
    let max = s.players[&b].max_health;
    s.players.get_mut(&b).unwrap().health = max / 2;
    s.set_valor(a, 100.0);
    cast(&mut s, a, "ultimate", "sanctuary", false);
    run(&mut s, 2.1);
    calm(&mut s, k);
    assert!(s.players[&b].health >= max / 2 + (max as f32 * 0.04 * 2.0) as u32 - 1, "4% a second");
    assert!(s.has_cc(b, "steadfast"));
    s.apply_cc(b, k, "stagger", 1.0, 0.0, 0.0, "test");
    assert!(!matches!(s.players[&b].action, Action::Stagger { .. }), "can't be staggered");
    assert!(s.has_cc(k, "snare"), "the foe is slowed");
}

#[test]
fn cold_snap_freezes_what_is_near_as_the_ice_block_breaks_and_fire_shatters_it() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [5.0, 0.0, 0.0], 0.0);
    s.set_valor(p, 25.0);
    cast(&mut s, p, "defense", "ice_block", true);
    assert!(s.immune(p));
    run(&mut s, 2.2);
    assert!(!s.immune(p));
    assert!(s.has_cc(k, "petrify"), "frozen as it breaks");
    calm(&mut s, k);
    // Fire on the frozen foe: the shatter.
    s.set_aim_line(p, 0.0, 0.0);
    let mut ev = cast(&mut s, p, "ranged", "fireball", false);
    ev.extend(run(&mut s, 1.0));
    assert!(flashed(&ev, p, "Shatter"));
    assert!(!s.has_cc(k, "petrify"), "the ice is gone");
}

#[test]
fn dread_circle_leaves_a_scream_where_it_steps_from() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_valor(p, 25.0);
    // The circle (free), a walk away, then the step back: a scream where they stood.
    cast(&mut s, p, "utility", "demonic_circle", true);
    s.move_player(p, [10.0, 0.0, 0.0], 0.0, Moving::default());
    let k = mob(&mut s, "grunt", [11.0, 2.0, 0.0], 0.0);
    s.input(p, Input::Ability(key_of("utility"), true));
    assert!(s.players[&p].pos[0] < 1.0, "back at the circle: {:?}", s.players[&p].pos);
    assert!(s.has_cc(k, "panic"), "the scream");
}

#[test]
fn the_step_back_to_a_demonic_circle_sets_you_down_on_the_ground_not_under_it() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Reckoned a moment past their last report, up a slope: the server has them under the ground.
    s.players.get_mut(&p).unwrap().pos = [0.0, 0.0, -2.0];
    s.slot_ability(p, "utility", "demonic_circle").unwrap();
    s.input(p, Input::Ability(key_of("utility"), false));
    s.move_player(p, [10.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.1);
    s.input(p, Input::Ability(key_of("utility"), false));
    let z = s.players[&p].pos[2];
    assert!(s.players[&p].pos[0] < 1.0, "back at the circle");
    assert!(z > 0.0 && z < 1.0, "set down just above the ground: {z}");
}

#[test]
fn an_old_saves_keys_go_on_the_boards_rows() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut save = s.leave(p).unwrap();
    save.loadout = BTreeMap::from([
        ("spell1".to_string(), "fireball".to_string()),
        ("control".to_string(), "hammer_of_justice".to_string()),
        ("counter".to_string(), "retaliation".to_string()),
        ("capstone".to_string(), "meteor".to_string()),
    ]);
    let q = s.join("ayla", "Ayla", Some(&save));
    let l = &s.players[&q].loadout;
    assert_eq!(l.get("ranged").map(String::as_str), Some("fireball"));
    assert_eq!(l.get("utility").map(String::as_str), Some("divine_steed"), "the first card for the row (the old Hammer of Justice's place)");
    assert_eq!(l.get("ultimate").map(String::as_str), Some("meteor"));
    assert_eq!(l.get("defense").map(String::as_str), Some("retaliation"), "the old counter, on its own row now");
    assert_eq!(l.get("close").map(String::as_str), Some("sundering_blow"), "a row it had nothing for keeps the default");
    assert!(!l.contains_key("spell1"));
    // A card since moved to another row (Ice Block, utility to defense) goes with it.
    let mut save = s.leave(q).unwrap();
    save.loadout = BTreeMap::from([("utility".to_string(), "ice_block".to_string()), ("ultimate".to_string(), "pyroblast".to_string())]);
    let r = s.join("ayla", "Ayla", Some(&save));
    let l = &s.players[&r].loadout;
    assert_eq!(l.get("defense").map(String::as_str), Some("ice_block"));
    assert_eq!(l.get("utility").map(String::as_str), Some("charge"), "the row it left keeps the default");
}

// --- five rows: the defense row, and full charge ---------------------------------------------------

#[test]
fn the_board_has_five_rows_of_nine() {
    let s = world_with(false);
    for row in crate::content::SLOTS {
        let cards: Vec<_> = s.content.abilities.iter().filter(|a| a.on_board() && a.slot == row).collect();
        assert_eq!(cards.len(), 9, "{row}");
        for col in crate::content::SCHOOLS {
            assert_eq!(cards.iter().filter(|a| a.school == col).count(), 1, "{row} {col}");
        }
    }
}

#[test]
fn lay_on_hands_alone_heals_two_fifths_and_ends_what_is_on_you() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().health = 50;
    s.apply_cc(p, p, "snare", 5.0, 0.5, 0.0, "test");
    assert!(s.has_cc(p, "snare"));
    s.set_valor(p, 25.0);
    cast(&mut s, p, "defense", "blessing_of_protection", true);
    let p_ = &s.players[&p];
    assert_eq!(p_.health, 50 + (p_.max_health as f32 * 0.4).round() as u32);
    assert!(!s.has_cc(p, "snare"), "cleansed");
}

#[test]
fn shield_wall_blocks_bolts_and_dread_ward_catches_every_blow() {
    let mut s = world_with(true);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [12.0, 0.0, 0.0], std::f32::consts::PI);
    // Retaliation lets a bolt by; its Shield Wall doesn't.
    s.set_valor(b, 25.0);
    cast(&mut s, b, "defense", "retaliation", true);
    let mut ev = cast(&mut s, a, "ranged", "hurled_axe", false);
    ev.extend(run(&mut s, 1.0));
    let hits: Vec<_> = ev.iter().filter_map(|e| match e { Event::Swing { victim: Some(v), outcome, anim: false, .. } if *v == b => Some(*outcome), _ => None }).collect();
    assert!(!hits.is_empty() && hits.iter().all(|o| *o == Outcome::Blocked), "the wall takes the axe: {hits:?}");
    run(&mut s, 3.0);
    // Dread Ward: two blows, both swallowed, and the striker panics (the second time the grace
    // that follows a hard crowd control stops it).
    s.set_cooldowns(b, true);
    s.set_valor(b, 25.0);
    s.move_player(a, [10.5, 0.0, 0.0], 0.0, Moving::default());
    cast(&mut s, b, "defense", "shadow_ward", true);
    let mut caught = 0;
    for _ in 0..2 {
        s.input(a, Input::Attack);
        s.input(a, Input::AttackUp);
        let ev = run(&mut s, 0.9);
        caught += ev.iter().filter(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, anim: false, .. } if *v == b)).count();
        s.cleanse(a, &["panic"]);
        s.move_player(a, [10.5, 0.0, 0.0], 0.0, Moving::default());
        s.set_action(a, Action::Ready);
    }
    assert_eq!(caught, 2, "every blow, not just the first");
    assert!(s.cc_log_of(a).iter().any(|l| l.contains("Dread Ward: panic") && !l.contains("grace")), "the striker panicked");
}

#[test]
fn a_full_charge_flares_and_the_arrow_flies_half_again_as_fast() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.slot_ability(p, "ranged", "aimed_arrow").unwrap();
    let shoot = |s: &mut Survival, secs: f32| {
        s.input(p, Input::Ability(0, false));
        s.input(p, Input::Attack);
        let mut ev = run(s, secs);
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
        ev
    };
    let ev = shoot(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 75, .. })), "a short draw: the plain arrow");
    assert!(!ev.iter().any(|e| matches!(e, Event::Visual { kit: 353, .. })));
    assert!((s.projectiles[0].speed - 40.0).abs() < 0.01);
    run(&mut s, 2.0);
    s.set_cooldowns(p, true);
    let ev = shoot(&mut s, 1.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit: 353, .. } if *id == p)), "the flare as it tops out");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 3044, .. })), "full: the glowing arrow");
    assert!((s.projectiles[0].speed - 60.0).abs() < 0.01);
}
