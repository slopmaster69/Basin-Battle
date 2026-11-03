//! The Warrior's second choices, the Warlord (Corey's blueprint, Nov 2): Tackle carries one enemy
//! and Crashes it into what's behind it; Intervene takes the blow meant for a friend; Rallying Cry
//! and Battle Cry give the group stamina and harder heavy attacks; Bladestorm ramps and is ended
//! into a dodge or a guard; Cleave cashes in an opened enemy; Thunder Clap slows a pack's swings;
//! Ravager holds a spot and is torn back through the pack; Heroic Throw Taunts.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world_on(pvp: bool, land: FlatLand) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    let mut s = Survival::new(c, Box::new(land), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_on(false, FlatLand::default())
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, facing the hero, that won't swing.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.ready_at = 1e9;
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
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// The ability's key (with Ctrl: its EX), cooldowns off.
fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[s.content.ability(key).unwrap()].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

/// The key, then a click along `yaw` (an aimed ability goes).
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool, yaw: f32, pitch: f32) -> Vec<Event> {
    let mut ev = press(s, p, key, ex);
    s.set_aim_line(p, yaw, pitch);
    s.input(p, Input::Attack);
    ev.extend(s.step(DT));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

fn struck(ev: &[Event], victim: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome: Outcome::Hit, .. } if *v == victim => Some(*damage),
            _ => None,
        })
        .collect()
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn walled(at: [f32; 2], r: f32) -> FlatLand {
    FlatLand { walls: vec![(at, r)], ..Default::default() }
}

// --- Tackle --------------------------------------------------------------------------------------------

#[test]
fn tackle_drives_an_enemy_into_a_wall_and_it_crashes_stunned() {
    let mut s = world_on(false, walled([6.5, 0.0], 1.0));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let before = hp(&s, k);
    let stamina = s.players[&p].stamina;
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    assert!(s.players[&p].stamina < stamina - 10.0, "it costs 12 stamina");
    assert!(s.unstaggerable(p), "nothing staggers the hero during the rush");
    ev.extend(run(&mut s, 0.8));
    assert!(flashed(&ev, p, "Crash!"), "told as a Crash");
    assert!(struck(&ev, k).contains(&20), "20 damage on the wall: {:?}", struck(&ev, k));
    assert!(hp(&s, k) < before);
    assert!(s.has_cc(k, "stun"), "stunned against the wall");
    let at = s.mobs[&k].pos;
    assert!(at[0] > 3.5 && at[0] < 5.6, "carried up to the wall's face: {at:?}");
    assert!(s.players[&p].pos[0] > 2.0, "the hero rushed in behind it: {:?}", s.players[&p].pos);
}

#[test]
fn tackle_into_another_enemy_knocks_both_down() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let b = mob(&mut s, "brave", [5.5, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.8));
    assert!(struck(&ev, a).contains(&14), "the carried one takes 14: {:?}", struck(&ev, a));
    assert!(struck(&ev, b).contains(&14), "the one it's driven into too: {:?}", struck(&ev, b));
    assert!(s.has_cc(a, "stun") && s.has_cc(b, "stun"), "both knocked down");
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim, .. } if *id == b && *anim == KNOCKDOWN_ANIM)), "thrown to the ground");
}

#[test]
fn tackle_with_nothing_behind_drops_the_enemy_at_your_feet() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [2.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.35));
    assert!(s.mobs[&k].pos[0] > 8.0, "carried the rest of the 8 yd: {:?}", s.mobs[&k].pos);
    ev.extend(run(&mut s, 0.3));
    assert_eq!(struck(&ev, k), vec![10], "dropped for 10");
    assert!(!s.has_cc(k, "stun"), "no Crash, no stun");
    assert!(s.players[&p].pos[0] > 7.0, "the hero went the whole rush: {:?}", s.players[&p].pos);
}

#[test]
fn tackle_stops_against_a_boss_and_shakes_its_stance() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [2.5, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "tackle", false, 0.0, 0.0);
    ev.extend(run(&mut s, 0.1));
    assert!(s.posture_of(w) >= 30, "35 posture: {}", s.posture_of(w));
    assert!(flashed(&ev, p, "Stopped hard"));
    assert!(!ev.iter().any(|e| matches!(e, Event::MobMove { id, speed, .. } if *id == w && *speed > 10.0)), "the boss isn't carried");
    assert!(s.players[&p].pos[0] < 2.0, "the hero stops against it");
}

// --- Intervene ----------------------------------------------------------------------------------------

#[test]
fn intervene_rushes_to_a_friend_and_takes_the_next_blow_meant_for_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [11.5, 0.5, 0.0]);
    s.set_valor(a, 25.0);
    let mut ev = cast(&mut s, a, "tackle", true, 0.0, 0.0);
    ev.extend(run(&mut s, 0.5));
    assert!(dist2d(s.players[&a].pos, s.players[&b].pos) < 2.0, "Ayla is at Bren's side: {:?}", s.players[&a].pos);
    assert!(s.has_cc(b, "guarded"), "Bren is covered");
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    let from = s.mobs[&k].pos;
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(DT);
    assert_eq!(s.players[&b].health, hb, "the blow meant for Bren");
    assert!(s.players[&a].health < ha, "strikes Ayla instead");
    assert!(flashed(&ev, a, "Intervened"));
    assert!(!s.has_cc(b, "guarded"), "only the first blow");
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    s.step(DT);
    assert!(s.players[&b].health < hb, "the next one lands on Bren");
}

#[test]
fn intervene_with_the_guard_up_blocks_the_blow() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let k = mob(&mut s, "brave", [10.0, 2.5, 0.0]);
    s.set_valor(a, 25.0);
    cast(&mut s, a, "tackle", true, 0.0, 0.0);
    run(&mut s, 0.5);
    s.input(a, Input::Guard);
    run(&mut s, 0.6);
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    let from = s.mobs[&k].pos;
    s.strike_fair(k, b, 20, from, true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(DT);
    assert_eq!((s.players[&a].health, s.players[&b].health), (ha, hb), "Ayla's guard takes it");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, .. } if *v == a)), "blocked on Ayla");
}

#[test]
fn intervene_with_no_friend_in_reach_spends_nothing() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.set_valor(a, 25.0);
    s.set_cooldowns(a, false);
    let slot = "utility";
    s.slot_ability(a, slot, "tackle").unwrap();
    s.input(a, Input::Ability(key_of(slot), true));
    s.step(DT);
    s.set_aim_line(a, 0.0, 0.0);
    s.input(a, Input::Attack);
    let ev = s.step(DT);
    assert!(flashed(&ev, a, "No ally in reach"));
    assert_eq!(s.players[&a].valor, 25.0, "the Valor back");
    let t = s.content.ability("tackle").unwrap();
    assert!(s.cooldown(a, t).charges > 0, "and the cooldown");
}

// --- Rallying Cry, Battle Cry -------------------------------------------------------------------------------

#[test]
fn rallying_cry_refills_the_stamina_of_everyone_near() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let c = hero(&mut s, "Cara", [25.0, 0.0, 0.0], 0.0);
    for h in [a, b, c] {
        s.players.get_mut(&h).unwrap().stamina = 10.0;
    }
    let ev = press(&mut s, a, "rallying_cry", false);
    assert!(s.players[&a].stamina >= 50.0, "Ayla's own: {}", s.players[&a].stamina);
    assert!(s.players[&b].stamina >= 50.0, "Bren, 10 yd off: {}", s.players[&b].stamina);
    assert!(s.players[&c].stamina < 20.0, "Cara, 25 yd off, out of reach: {}", s.players[&c].stamina);
    let shouted: Vec<Id> = ev.iter().filter_map(|e| match e { Event::Missile { spell: 6673, target: Target::Unit(u), .. } => Some(*u), _ => None }).collect();
    assert!(shouted.contains(&a) && shouted.contains(&b) && !shouted.contains(&c), "Battle Shout's look on each one reached: {shouted:?}");
}

#[test]
fn battle_cry_makes_the_groups_heavy_attacks_break_a_stance_faster() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [0.0, 6.0, 0.0], 0.0);
    // Two bosses: the first struck before the cry, the second after.
    let w1 = mob(&mut s, "warlord", [1.6, 6.0, 0.0]);
    let w2 = mob(&mut s, "warlord", [1.6, 20.0, 0.0]);
    let heavy_on = |s: &mut Survival, w: Id| -> (u32, u32) {
        let (h0, p0) = (s.mobs[&w].health, s.posture_of(w));
        s.input(b, Input::Heavy);
        for _ in 0..60 {
            s.step(DT);
            if s.mobs[&w].health < h0 {
                break;
            }
        }
        (h0 - s.mobs[&w].health, s.posture_of(w) - p0)
    };
    let (d1, p1) = heavy_on(&mut s, w1);
    assert!(d1 > 0 && p1 > 0, "the plain heavy lands: {d1} damage, {p1} posture");
    s.set_valor(a, 25.0);
    run(&mut s, 1.0);
    press(&mut s, a, "rallying_cry", true);
    assert!(s.has_cc(b, "battle_cry") && s.has_cc(a, "battle_cry"), "the whole group within 15 yd");
    // Over to the second boss.
    s.move_player(b, [0.0, 20.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.6);
    let (d2, p2) = heavy_on(&mut s, w2);
    let (r1, r2) = (p1 as f32 / d1 as f32, p2 as f32 / d2 as f32);
    assert!(r2 > r1 * 1.35, "50% more posture a point of damage: {r1:.2} then {r2:.2}");
}

// --- Bladestorm ------------------------------------------------------------------------------------------

fn spin(s: &mut Survival, p: Id) -> Vec<Event> {
    s.set_valor(p, 100.0);
    cast(s, p, "warrior_bladestorm", false, 0.0, 0.0)
}

#[test]
fn bladestorm_ramps_a_point_a_strike_up_to_15() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "warlord", [3.5, 0.0, 0.0]);
    let mut ev = spin(&mut s, p);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "spinning");
    ev.extend(run(&mut s, 6.5));
    let hits = struck(&ev, k);
    assert_eq!(hits.len(), 12, "twice a second for 6 sec: {hits:?}");
    assert_eq!(hits, (4..=15).collect::<Vec<u32>>(), "4, then 1 more a strike, up to 15");
    assert_eq!(hits.iter().sum::<u32>(), 114);
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "done after 6 sec");
}

#[test]
fn bladestorm_walks_at_70_percent_of_a_run() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    let run_speed = s.content.player.run_speed;
    assert!((s.speed_for(p) - run_speed * 0.7).abs() < 0.01, "{} of {run_speed}", s.speed_for(p));
}

#[test]
fn bladestorm_ends_into_a_guard_on_that_press() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    run(&mut s, 1.0);
    s.input(p, Input::Guard);
    assert!(matches!(s.players[&p].action, Action::Guard), "the guard up at once: {:?}", s.players[&p].action);
    assert!((s.players[&p].guard_at - s.clock).abs() < 1e-3, "a fresh guard: its parry window open");
}

#[test]
fn bladestorm_ends_into_a_dodge_on_that_press() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    run(&mut s, 1.0);
    s.input(p, Input::Dodge);
    assert!(matches!(s.players[&p].action, Action::Dodge { .. }), "the dodge at once: {:?}", s.players[&p].action);
}

#[test]
fn bladestorm_takes_no_other_ability_and_a_stagger_ends_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    spin(&mut s, p);
    s.slot_ability(p, "utility", "tackle").unwrap();
    s.input(p, Input::Ability(key_of("utility"), false));
    s.step(DT);
    assert!(matches!(s.players[&p].action, Action::Channel { .. }), "no other ability while it spins");
    let k = mob(&mut s, "brave", [1.5, 0.0, 0.0]);
    assert!(s.apply_cc(p, k, "stagger", 1.0, 0.0, 0.0, "test"), "it can be staggered");
    assert!(!matches!(s.players[&p].action, Action::Channel { .. }), "and the stagger ends it");
}
