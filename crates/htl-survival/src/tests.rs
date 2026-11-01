//! Elwynn Survival's rules against the build brief's acceptance checks.

use std::collections::BTreeMap;

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;

fn world_with(land: FlatLand) -> Survival {
    // The co-operative rules: heroes don't strike each other, and a fall leaves a sack.
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(land), 1, 1_000_000.0);
    // Quiet: no creatures unless a test brings them.
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn world() -> Survival {
    world_with(FlatLand::default())
}

/// A hero at `pos` facing `facing`.
fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    // Calm until a test wants it to fight.
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

fn hits_on(events: &[Event], victim: Id) -> Vec<(u32, Outcome)> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, anim: false, .. } if *v == victim => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

// --- melee -----------------------------------------------------------------------------------------

#[test]
fn a_swing_at_nothing_misses_but_costs_its_time_and_stamina() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let full = s.players[&p].stamina;
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.3);
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, anim: true, .. } if *attacker == p)), "the swing plays");
    assert!(s.players[&p].stamina <= full - 13.9, "stamina spent");
    assert_eq!(s.players[&p].action.name(), "swing", "still committed");
    run(&mut s, 0.8);
    assert_eq!(s.players[&p].action, Action::Ready, "the swing ran its course");
}

#[test]
fn the_guard_raised_while_a_blow_winds_up_calls_it_off_and_gives_its_stamina_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let full = s.players[&p].stamina;
    s.input(p, Input::Attack);
    s.step(DT);
    assert!(s.players[&p].stamina < full - 1.0, "spent on the swing");
    s.input(p, Input::Guard);
    let ev = s.step(DT);
    assert_eq!(s.players[&p].action, Action::Guard, "the guard up in its place");
    assert!(ev.iter().any(|e| matches!(e, Event::ClipStop { id } if *id == p)), "the swing's clip let go");
    assert!((s.players[&p].stamina - full).abs() < 1.0, "the stamina back: {} of {full}", s.players[&p].stamina);
    s.input(p, Input::GuardUp);
    // Once the blow has come, the guard no longer takes it back.
    s.input(p, Input::Attack);
    let windup = match &s.players[&p].action {
        Action::Swing { strike, .. } => strike.windup,
        a => panic!("a swing: {a:?}"),
    };
    run(&mut s, windup + 0.05);
    let spent = s.players[&p].stamina;
    s.input(p, Input::Guard);
    s.step(DT);
    assert!(s.players[&p].stamina <= spent + 1.0, "nothing back after the blow");
}

#[test]
fn a_forward_swing_spares_what_is_behind_or_beyond_a_wall() {
    // A wall between the hero and an enemy right in front, at swinging distance.
    let land = FlatLand { walls: vec![([1.4, 6.0], 0.4)], ..Default::default() };
    let mut s = world_with(land);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ahead = mob(&mut s, "grunt", [2.6, 0.0, 0.0]);
    let behind = mob(&mut s, "grunt", [-2.0, 0.0, 0.0]);
    let walled = mob(&mut s, "grunt", [2.6, 6.0, 0.0]);
    // Walls alone: no soft lock drawing the blow to whoever has come round the wall.
    s.content.lock.melee = 0.0;
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert_eq!(hits_on(&ev, ahead).len(), 1, "the one in front is struck once");
    assert!(hits_on(&ev, behind).is_empty(), "nothing behind is struck");
    // Facing the walled one: the wall takes it.
    s.move_player(p, [0.0, 3.5, 0.0], facing([0.0, 3.5, 0.0], [2.6, 6.0, 0.0]), Moving::default());
    run(&mut s, 0.5);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert!(hits_on(&ev, walled).is_empty(), "not through a wall");
}

#[test]
fn clicking_and_switching_targets_cannot_add_hits() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "grunt", [2.0, 0.6, 0.0]);
    let b = mob(&mut s, "grunt", [2.0, -0.6, 0.0]);
    let mut ev = Vec::new();
    // Twenty clicks in the first half second, turning between the two.
    for k in 0..20 {
        let f = if k % 2 == 0 { 0.25 } else { -0.25 };
        s.move_player(p, ORIGIN, f, Moving::default());
        s.input(p, Input::Attack);
        ev.extend(s.step(0.025));
    }
    ev.extend(run(&mut s, 0.5));
    let total = hits_on(&ev, a).len() + hits_on(&ev, b).len();
    // One swing, each enemy at most once: spam doesn't queue more swings this early.
    assert!(total <= 2, "{total} hits from one swing's worth of clicks");
    let swings = ev.iter().filter(|e| matches!(e, Event::Swing { attacker, anim: true, .. } if *attacker == p)).count();
    assert_eq!(swings, 1, "one swing");
}

#[test]
fn an_input_near_the_end_of_a_swing_waits_for_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.input(p, Input::Attack);
    // Late in the recovery: kept.
    run(&mut s, 0.7);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.3);
    let swings = ev.iter().filter(|e| matches!(e, Event::Swing { attacker, anim: true, .. } if *attacker == p)).count();
    assert_eq!(swings, 1, "the buffered swing follows the first");
    // Early in the wind-up: dropped.
    run(&mut s, 1.5);
    s.input(p, Input::Attack);
    let mut ev = run(&mut s, 0.1);
    s.input(p, Input::Attack);
    ev.extend(run(&mut s, 1.5));
    let swings = ev.iter().filter(|e| matches!(e, Event::Swing { attacker, anim: true, .. } if *attacker == p)).count();
    assert_eq!(swings, 1, "an early second press is dropped");
}

#[test]
fn an_attack_with_the_guard_up_bashes_and_the_shield_is_down_meanwhile() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    s.input(p, Input::Attack);
    assert_eq!(s.players[&p].action.name(), "bash");
    // A blow lands mid-swing: no block.
    let out = s.strike_player(k, p, 10, [2.0, 0.0, 0.0], true, false, false, Blow::Light);
    assert_eq!(out, Outcome::Hit);
    // The guard comes back up after the swing, as it's still held.
    run(&mut s, 1.0);
    assert_eq!(s.players[&p].action, Action::Guard);
}

// --- block and dodge -------------------------------------------------------------------------------

#[test]
fn a_raised_shield_blocks_in_front_but_not_behind() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    let before = s.players[&p].stamina;
    let hp = s.players[&p].health;
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, false, Blow::Light), Outcome::Blocked);
    assert_eq!(s.players[&p].health, hp, "nothing gets through");
    assert!(s.players[&p].stamina < before, "it costs stamina");
    assert_eq!(s.strike_player(k, p, 16, [-2.0, 0.0, 0.0], true, false, false, Blow::Light), Outcome::Hit, "from behind, it lands");
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], false, false, false, Blow::Light), Outcome::Hit, "an unblockable blow lands");
    // A heavy blow costs more to block.
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, false, Blow::Light);
    let light = 100.0 - s.players[&p].stamina;
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, true, false, Blow::Light);
    let heavy = 100.0 - s.players[&p].stamina;
    assert!(heavy > light * 1.5, "heavy {heavy} vs light {light}");
}

#[test]
fn a_blow_too_heavy_for_the_stamina_left_breaks_the_guard() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "overseer", [2.0, 0.0, 0.0]);
    // It doesn't swing again meanwhile.
    s.mobs.get_mut(&k).unwrap().ready_at = 1e9;
    s.input(p, Input::Guard);
    s.players.get_mut(&p).unwrap().stamina = 10.0;
    let hp = s.players[&p].health;
    assert_eq!(s.strike_player(k, p, 42, [2.0, 0.0, 0.0], true, true, false, Blow::Light), Outcome::GuardBroken);
    let took = hp - s.players[&p].health;
    assert!(took > 0 && took < 42, "part of it lands: {took}");
    assert_eq!(s.players[&p].action.name(), "stagger");
    // Staggered: the next blow lands whole, and nothing can be done for a moment.
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action.name(), "stagger", "no dodging out of a stagger");
    // A heavy creature's blow through the guard puts the hero down for a while.
    run(&mut s, 1.0);
    assert_eq!(s.players[&p].action.name(), "stagger", "still down");
    run(&mut s, 0.6);
    assert_eq!(s.players[&p].action, Action::Guard, "the guard comes back up after: {:?}", s.mobs[&k].state);
}

#[test]
fn a_dodge_evades_inside_its_window_only_and_hops_where_the_keys_point() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // (A grunt standing by that doesn't join in.)
    let g = s.content.mob("grunt").unwrap();
    s.content.mobs[g].aggro = 0.0;
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.move_player(p, ORIGIN, 0.0, Moving { left: true, ..Default::default() });
    s.input(p, Input::Dodge);
    let ev = s.step(0.1);
    let hop = ev.iter().find_map(|e| match e {
        Event::Hop { player, dir, .. } if *player == p => Some(*dir),
        _ => None,
    });
    assert!(hop.is_some_and(|d| (d - std::f32::consts::FRAC_PI_2).abs() < 0.01), "strafing left hops left: {hop:?}");
    // With a direction held the dash runs along the ground: no throw, the run speeds raised, and
    // put back once the dash is spent.
    let dash = s.content.dodge.speed;
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, speed, roll: true, .. } if *player == p && *speed == 0.0)), "no throw");
    assert!(ev.iter().any(|e| matches!(e, Event::Speed { player, speed, back } if *player == p && *speed == dash && *back == dash)), "dashing");

    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, false, Blow::Light), Outcome::Dodged);
    // Pressing again doesn't refresh the window.
    s.input(p, Input::Dodge);
    run(&mut s, 0.3);
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, false, Blow::Light), Outcome::Hit, "after the window it lands");
    let h = &s.players[&p];
    assert!(h.sent_speed < dash && h.sent_back == BACK_SPEED, "the dash spent, the speeds are back");
    // No keys: a step back.
    run(&mut s, 2.0);
    s.move_player(p, ORIGIN, 1.0, Moving::default());
    s.input(p, Input::Dodge);
    let ev = s.step(0.05);
    let hop = ev.iter().find_map(|e| match e {
        Event::Hop { dir, .. } => Some(*dir),
        _ => None,
    });
    assert!(hop.is_some_and(|d| angle_between(d, 1.0 + std::f32::consts::PI) < 1.0), "a backstep: {hop:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { speed, lift, .. } if *speed > 0.0 && *lift > 0.0)), "a hop back");
    assert!(!ev.iter().any(|e| matches!(e, Event::Speed { .. })), "no dash on the spot");
}

#[test]
fn stamina_stops_coming_back_while_guarding_and_running_out_leaves_you_open() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().stamina = 30.0;
    s.input(p, Input::Guard);
    run(&mut s, 2.0);
    assert!((s.players[&p].stamina - 30.0).abs() < 0.01, "no regen behind the shield");
    s.input(p, Input::GuardUp);
    run(&mut s, 2.0);
    assert!(s.players[&p].stamina > 60.0, "it comes back once lowered");
    // Spent out: a dodge is refused (a short spell of vulnerability).
    s.players.get_mut(&p).unwrap().stamina = 0.0;
    s.players.get_mut(&p).unwrap().spent_at = s.clock;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action, Action::Ready);
    // More than a roll costs but less than it takes to start one: still no.
    s.players.get_mut(&p).unwrap().stamina = (s.content.dodge.stamina + s.content.dodge.min_stamina) / 2.0;
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action, Action::Ready, "not with a sliver");
    s.players.get_mut(&p).unwrap().stamina = 0.0;
    s.players.get_mut(&p).unwrap().spent_at = s.clock;
    run(&mut s, 1.8);
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action.name(), "dodge", "after a moment it's back");
}

// --- the bow: Aimed Shot ---------------------------------------------------------------------------

/// The Aimed Shot's key (Q, the bow taken off the bench onto it).
const SHOT: Input = Input::Ability(0, false);

/// Draw the bow on its key, hold the click `secs`, and let go.
fn draw_and_loose(s: &mut Survival, p: Id, secs: f32) -> Vec<Event> {
    s.players.get_mut(&p).unwrap().cd_off = true;
    let _ = s.slot_ability(p, "ranged", "aimed_shot");
    s.input(p, SHOT);
    s.input(p, Input::Attack);
    let mut ev = run(s, secs);
    s.input(p, Input::AttackUp);
    ev.extend(run(s, 1.2));
    ev
}

#[test]
fn an_aimed_arrow_strikes_on_the_line_and_misses_off_it_or_behind_cover() {
    let land = FlatLand { walls: vec![([10.0, 8.0], 0.6)], ..Default::default() };
    let mut s = world_with(land);
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    let on = mob(&mut s, "grunt", [20.0, 0.2, 0.0]);
    let off = mob(&mut s, "grunt", [20.0, 4.0, 0.0]);
    let covered = mob(&mut s, "grunt", [20.0, 16.0, 0.0]);
    let ev = draw_and_loose(&mut s, p, 0.8);
    assert_eq!(hits_on(&ev, on).len(), 1, "struck on the line");
    assert!(hits_on(&ev, off).is_empty(), "nothing off the line");
    assert_eq!(s.players[&p].kit, Kit::Melee, "the bow is put away after the shot");
    // Turned to the one behind the wall: the wall takes it.
    s.move_player(p, ORIGIN, facing(ORIGIN, [20.0, 16.0, 0.0]), Moving::default());
    let ev = draw_and_loose(&mut s, p, 0.8);
    assert!(hits_on(&ev, covered).is_empty(), "the wall stops it");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { target: Target::Point(_), .. })), "the arrow's look flies to a point, never after a unit");
}

#[test]
fn right_click_calls_off_the_draw_and_nothing_is_spent() {
    let mut s = world();
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    mob(&mut s, "grunt", [15.0, 0.0, 0.0]);
    let _ = s.slot_ability(p, "ranged", "aimed_shot");
    s.input(p, SHOT);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.6);
    assert!(ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == p)), "the bow is drawn");
    s.input(p, Input::Guard);
    let ev = run(&mut s, 1.0);
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { .. })), "nothing fired");
    assert!(ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == p)), "the draw is let down");
    assert_eq!(s.players[&p].action, Action::Guard, "and the guard is up");
    let a = s.content.ability("aimed_shot").unwrap();
    assert!(s.cooldown(p, a).charges > 0, "no cooldown for a shot not taken");
}

#[test]
fn nothing_turns_a_shot_toward_a_foe_off_the_line() {
    let mut s = world();
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    // A little off the facing: the old quick shot found it; nothing does now.
    let near = mob(&mut s, "grunt", [15.0, 1.8, 0.0]);
    let ev = draw_and_loose(&mut s, p, 1.0);
    assert!(hits_on(&ev, near).is_empty(), "no auto-aim");
}

#[test]
fn a_longer_draw_hits_harder() {
    let mut s = world();
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    let k = mob(&mut s, "boar", [12.0, 0.0, 0.0]);
    let short = hits_on(&draw_and_loose(&mut s, p, 0.45), k);
    let long = hits_on(&draw_and_loose(&mut s, p, 1.6), k);
    assert!(short[0].0 < long[0].0, "{short:?} then {long:?}");
    assert!(matches!(s.content.abilities[s.content.ability("aimed_shot").unwrap()].kind.as_str(), "shot"));
}

#[test]
fn an_aimed_shot_from_the_shoulder_camera_flies_along_its_line() {
    let mut s = world();
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().shoulder_cam = true;
    // Straight ahead of the hero's body: off the camera's line, which runs beside them.
    let body_line = mob(&mut s, "grunt", [20.0, 1.2, 0.0]);
    let cam_line = mob(&mut s, "grunt", [20.0, -0.75, 0.0]);
    let ev = draw_and_loose(&mut s, p, 0.8);
    assert_eq!(hits_on(&ev, cam_line).len(), 1, "where the crosshair is");
    assert!(hits_on(&ev, body_line).is_empty());
}

#[test]
fn a_heavy_attack_is_slower_harder_and_throws_a_hero_back() {
    let mut s = pvp_world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    let w = s.weapon_of(&s.players[&a]).clone();
    let stamina = s.players[&a].stamina;
    s.input(a, Input::Heavy);
    assert_eq!(s.players[&a].action.name(), "heavy");
    let spent = stamina - s.players[&a].stamina;
    assert!(spent > w.stamina * 1.2, "it costs more: {spent}");
    // Not landed yet when a light swing would have.
    // A light swing lands as its wind-up ends; the heavy one is still winding.
    let ev = run(&mut s, w.windup + 0.1);
    assert!(hits_on(&ev, b).is_empty(), "slower to come");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, anim: true, style: Blow::Heavy, .. } if *attacker == a)) || s.players[&a].action.name() == "heavy");
    let ev = run(&mut s, 1.5);
    let hit = hits_on(&ev, b);
    assert_eq!(hit.len(), 1);
    assert!(hit[0].0 as f32 > w.damage[1] as f32, "harder than any light blow: {}", hit[0].0);
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == b)), "thrown back");
}

#[test]
fn a_swing_a_little_wide_still_finds_its_foe() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = s.weapon_of(&s.players[&p]).clone();
    // Just outside the arc, within the soft lock.
    let a = (w.arc / 2.0 + s.content.lock.melee * 0.6).to_radians();
    let foe = mob(&mut s, "grunt", [2.4 * a.cos(), 2.4 * a.sin(), 0.0]);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert_eq!(hits_on(&ev, foe).len(), 1);
    // Well outside it: missed.
    s.content.lock.melee = 0.0;
    run(&mut s, 0.5);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert!(hits_on(&ev, foe).is_empty());
}

#[test]
fn being_too_winded_says_so_without_a_word_in_chat() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().stamina = 0.0;
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.1);
    assert!(ev.iter().any(|e| matches!(e, Event::Winded { player } if *player == p)));
    assert!(!ev.iter().any(|e| matches!(e, Event::Tell { .. })));
}

// --- creatures -------------------------------------------------------------------------------------

#[test]
fn a_creature_telegraphs_commits_and_a_step_aside_late_still_dodges_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Its plain blow (not a string of them).
    let g = s.content.mob("grunt").unwrap();
    s.content.mobs[g].patterns.clear();
    s.content.mobs[g].special = None;
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    let mut ev = Vec::new();
    let mut marked = false;
    // Wait for the wind-up to begin.
    for _ in 0..40 {
        let e = s.step(DT);
        marked |= e.iter().any(|x| matches!(x, Event::Mark { .. }));
        ev.extend(e);
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            break;
        }
    }
    assert!(marked, "its blow is marked on the ground");
    // In the last moment (after it stops turning), step well round behind it.
    let a = s.content.mobs[s.mobs[&k].def].attack.clone();
    run(&mut s, a.windup - a.lock + 0.05);
    s.move_player(p, [4.5, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.5);
    assert!(hits_on(&ev, p).iter().all(|(d, _)| *d == 0), "it doesn't swing round onto you: {:?}", hits_on(&ev, p));
    assert!(matches!(s.mobs[&k].state, MobState::Recover { .. } | MobState::Chase { .. }), "then recovers");
}

#[test]
fn creatures_call_their_neighbours_and_give_up_past_their_leash() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let def = s.content.mob("grunt").unwrap();
    let pack = s.content.packs.iter().position(|k| k.mob == "grunt").unwrap();
    let a = s.spawn_mob(def, [20.0, 0.0, 0.0], Some(pack), false);
    let b = s.spawn_mob(def, [26.0, 0.0, 0.0], Some(pack), false);
    let far = s.spawn_mob(def, [60.0, 0.0, 0.0], Some(pack), false);
    s.hurt_mob(a, p, 1);
    assert!(matches!(s.mobs[&b].state, MobState::Chase { .. }), "the neighbour joins in");
    assert!(matches!(s.mobs[&far].state, MobState::Idle { .. }), "one far off doesn't");
    // Run off: past its leash it turns back.
    for k in 0..200 {
        s.move_player(p, [-(k as f32) * 0.5, 0.0, 0.0], 0.0, Moving::default());
        let _ = s.step(DT);
    }
    assert!(matches!(s.mobs[&a].state, MobState::Return | MobState::Idle { .. }), "{:?}", s.mobs[&a].state);
}

// --- gathering, the chest, building ----------------------------------------------------------------

fn node_of(s: &Survival, item: &str) -> Id {
    *s.objs.iter().find(|(_, o)| matches!(&o.kind, ObjKind::Node { def, ready: true, .. } if s.content.nodes[*def].item == item)).unwrap().0
}

#[test]
fn the_last_gather_from_a_node_goes_to_one_hero_only() {
    let mut s = world();
    let n = node_of(&s, "wood");
    let at = s.objs[&n].pos;
    let a = hero(&mut s, "Ayla", [at[0] + 1.5, at[1], 0.0], std::f32::consts::PI);
    let b = hero(&mut s, "Bren", [at[0] - 1.5, at[1], 0.0], 0.0);
    // Down to its last use.
    if let ObjKind::Node { uses, .. } = &mut s.objs.get_mut(&n).unwrap().kind {
        *uses = 1;
    }
    s.interact(a, Some(n));
    s.interact(b, Some(n));
    run(&mut s, 3.0);
    let wood = s.content.item("wood").unwrap();
    let got = |s: &Survival, p: Id| s.players[&p].bag.get(&wood).copied().unwrap_or(0);
    assert!((got(&s, a) > 0) != (got(&s, b) > 0), "one of them: {} and {}", got(&s, a), got(&s, b));
    assert!(s.object(n).is_none(), "the node is spent");
    // It comes back by real time.
    s.wall_start += 10_000.0;
    let ev = s.step(DT);
    assert!(ev.contains(&Event::ObjSpawned { id: n }));
}

fn give(s: &mut Survival, p: Id, key: &str, n: u32) {
    let i = s.content.item(key).unwrap();
    *s.players.get_mut(&p).unwrap().bag.entry(i).or_default() += n;
}

fn build_spot(s: &Survival) -> (Vec3, f32) {
    // Stand west of the building ground's middle, facing east into it.
    let c = s.content.world.build_center;
    ([c[0], c[1] + BUILD_DISTANCE, 0.0], -std::f32::consts::FRAC_PI_2)
}

#[test]
fn placing_a_station_costs_once_and_only_where_it_may_stand() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    assert!(s.placement(p).unwrap().problem.is_some_and(|w| w.contains("Needs")), "{:?}", s.placement(p));
    assert!(s.build_confirm(p).is_err(), "can't pay: refused");
    give(&mut s, p, "wood", 10);
    let _ = s.step(DT);
    assert_eq!(s.placement(p).unwrap().problem, None);
    s.build_confirm(p).unwrap();
    let wood = s.content.item("wood").unwrap();
    assert_eq!(s.players[&p].bag[&wood], 4, "six timber spent");
    // A repeated confirm (a retried request) does nothing more.
    assert!(s.build_confirm(p).is_err());
    assert_eq!(s.players[&p].bag[&wood], 4);
    // A second fire on the same spot: too close.
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    assert!(s.placement(p).unwrap().problem.is_some_and(|w| w.contains("close")), "{:?}", s.placement(p));
    s.cancel_preview(p);
    // Outside the camp's ground.
    s.move_player(p, [at[0] + 100.0, at[1], 0.0], f, Moving::default());
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    assert!(s.placement(p).unwrap().problem.is_some_and(|w| w.contains("Outside")));
    s.cancel_preview(p);
    // The chest pays too: banked timber counts.
    s.storage.insert(wood, 20);
    s.players.get_mut(&p).unwrap().bag.clear();
    s.move_player(p, [at[0] + 8.0, at[1], 0.0], f, Moving::default());
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    assert_eq!(s.placement(p).unwrap().problem, None, "{:?}", s.placement(p));
    s.build_confirm(p).unwrap();
    assert_eq!(s.storage[&wood], 14);
}

#[test]
fn the_road_and_the_camps_own_spots_stay_clear() {
    let mut s = world();
    let c = s.content.world.build_center;
    s.land = Box::new(FlatLand { roads: vec![([c[0], c[1]], 3.0)], ..Default::default() });
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    give(&mut s, p, "wood", 10);
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    assert!(s.placement(p).unwrap().problem.is_some_and(|w| w.contains("road")), "{:?}", s.placement(p));
}

#[test]
fn cooking_turns_meat_into_food_that_heals_unless_a_hit_spoils_the_meal() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    give(&mut s, p, "wood", 6);
    give(&mut s, p, "meat", 4);
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    let fire = *s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).unwrap().0;
    let roast = s.content.item("roast").unwrap();
    let before = s.players[&p].bag.get(&roast).copied().unwrap_or(0);
    s.craft(p, fire, "roast").unwrap();
    assert_eq!(s.players[&p].bag[&roast], before + 1);
    // Eat it hurt: it heals; struck mid-meal, the bite isn't eaten.
    s.players.get_mut(&p).unwrap().health = 50;
    s.input(p, Input::Use);
    run(&mut s, 0.5);
    let k = mob(&mut s, "grunt", [at[0] + 2.0, at[1], 0.0]);
    s.strike_player(k, p, 5, [at[0] + 2.0, at[1], 0.0], false, false, false, Blow::Light);
    s.mobs.remove(&k);
    assert_eq!(s.players[&p].action, Action::Ready, "the meal is spoiled");
    assert_eq!(s.players[&p].bag[&roast], before + 1, "and nothing eaten");
    s.input(p, Input::Use);
    run(&mut s, 2.0);
    assert!(s.players[&p].health > 100, "a full meal heals: {}", s.players[&p].health);
    assert_eq!(s.players[&p].bag.get(&roast).copied().unwrap_or(0), before);
}

#[test]
fn a_smithy_improves_gear_you_can_feel() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    give(&mut s, p, "wood", 20);
    give(&mut s, p, "ore", 12);
    s.build_start(p, "smithy").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    let anvil = *s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).unwrap().0;
    let k = mob(&mut s, "grunt", [at[0] - 2.0, at[1], 0.0]);
    let block_cost = |s: &mut Survival| {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.players.get_mut(&p).unwrap().action = Action::Guard;
        let pos = s.players[&p].pos;
        let from = [pos[0] + f.cos() * 2.0, pos[1] + f.sin() * 2.0, 0.0];
        assert_eq!(s.strike_player(k, p, 20, from, true, false, false, Blow::Light), Outcome::Blocked);
        100.0 - s.players[&p].stamina
    };
    let old = block_cost(&mut s);
    s.players.get_mut(&p).unwrap().action = Action::Ready;
    s.craft(p, anvil, "reinforced_shield").unwrap();
    let new = block_cost(&mut s);
    assert!(new < old * 0.7, "the reinforced shield blocks for less: {old} -> {new}");
    let buckler = s.content.item("buckler").unwrap();
    assert_eq!(s.storage.get(&buckler), Some(&1), "the old shield is kept in the chest");
}

#[test]
fn banking_moves_resources_but_keeps_food_and_the_chest_hands_them_back() {
    let mut s = world();
    let heart = s.content.world.heart;
    let p = hero(&mut s, "Ayla", [heart[0] + 1.5, heart[1], 0.0], std::f32::consts::PI);
    give(&mut s, p, "wood", 7);
    give(&mut s, p, "ore", 3);
    let chest = s.storage_id();
    s.interact(p, Some(chest));
    let wood = s.content.item("wood").unwrap();
    let roast = s.content.item("roast").unwrap();
    assert_eq!(s.storage[&wood], 7);
    assert!(!s.players[&p].bag.contains_key(&wood));
    assert!(s.players[&p].bag.contains_key(&roast), "food stays with you");
    s.withdraw(p, "wood", 5).unwrap();
    assert_eq!((s.storage[&wood], s.players[&p].bag[&wood]), (2, 5));
}

// --- falling and what's left behind ----------------------------------------------------------------

#[test]
fn a_fall_drops_carried_supplies_in_a_cache_that_comes_back_whole_once() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", [50.0, 50.0, 0.0], 0.0);
    give(&mut s, p, "ore", 4);
    let k = mob(&mut s, "grunt", [52.0, 50.0, 0.0]);
    s.strike_player(k, p, 10_000, [52.0, 50.0, 0.0], false, false, false, Blow::Light);
    assert_eq!(s.players[&p].health, 0);
    let ore = s.content.item("ore").unwrap();
    let roast = s.content.item("roast").unwrap();
    assert!(!s.players[&p].bag.contains_key(&ore));
    assert!(s.players[&p].bag.contains_key(&roast), "food and gear go with you");
    let caches = s.caches_of("ayla");
    assert_eq!(caches.len(), 1);
    let (cache, at) = caches[0];
    assert!(dist2d(at, [50.0, 50.0, 0.0]) < 0.1, "where they fell");
    // Up at the camp.
    run(&mut s, 7.0);
    assert!(s.players[&p].health > 0);
    let spawn = s.content.world.spawn;
    assert!(dist2d(s.players[&p].pos, [spawn[0], spawn[1], 0.0]) < 0.5);
    // Fall again with something else: a second cache, the first still there.
    give(&mut s, p, "wood", 2);
    s.strike_player(k, p, 10_000, [52.0, 50.0, 0.0], false, false, false, Blow::Light);
    assert_eq!(s.caches_of("ayla").len(), 2, "an earlier cache isn't lost");
    run(&mut s, 7.0);
    // Back for the first: claimed whole, and gone.
    s.move_player(p, [at[0] + 1.0, at[1], 0.0], std::f32::consts::PI, Moving::default());
    s.interact(p, Some(cache));
    assert_eq!(s.players[&p].bag.get(&ore), Some(&4));
    assert!(!s.objs.contains_key(&cache));
    s.interact(p, Some(cache));
    assert_eq!(s.players[&p].bag.get(&ore), Some(&4), "only once");
    // Someone else's isn't yours.
    let (other, oat) = s.caches_of("ayla")[0];
    let b = hero(&mut s, "Bren", [oat[0] + 1.0, oat[1], 0.0], std::f32::consts::PI);
    s.interact(b, Some(other));
    assert!(s.objs.contains_key(&other));
}

#[test]
fn a_fall_somewhere_unreachable_leaves_the_cache_on_the_last_firm_ground() {
    let land = FlatLand { walls: vec![([80.0, 80.0], 2.0)], ..Default::default() };
    let mut s = world_with(land);
    let p = hero(&mut s, "Ayla", [70.0, 80.0, 0.0], 0.0);
    run(&mut s, 0.2);
    give(&mut s, p, "ore", 2);
    // Inside the wall (a glitch, a ledge): not a place to leave things.
    s.move_player(p, [80.0, 80.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.1);
    s.hurt_player(p, 10_000);
    let (_, at) = s.caches_of("ayla")[0];
    assert!(dist2d(at, [70.0, 80.0, 0.0]) < 0.5, "{at:?}");
}

// --- keeping the world ------------------------------------------------------------------------------

#[test]
fn the_world_comes_back_from_its_save_as_it_was() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    give(&mut s, p, "wood", 6);
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    give(&mut s, p, "ore", 3);
    s.players.get_mut(&p).unwrap().health = 77;
    let wood = s.content.item("wood").unwrap();
    s.storage.insert(wood, 9);
    // A spent node, and a cache.
    let n = node_of(&s, "wood");
    if let ObjKind::Node { uses, ready, back_at, .. } = &mut s.objs.get_mut(&n).unwrap().kind {
        *uses = 0;
        *ready = false;
        *back_at = s.wall_start + 500.0;
    }
    let b = hero(&mut s, "Bren", [10.0, 10.0, 0.0], 0.0);
    give(&mut s, b, "ore", 2);
    s.hurt_player(b, 10_000);
    let left = s.leave(b).unwrap();
    let mut offline = BTreeMap::new();
    offline.insert("bren".to_string(), left);
    let save = s.save(&offline);
    let text = crate::save::to_toml(&save).unwrap();
    let back = crate::save::from_toml(&text).unwrap();
    assert_eq!(back, save);
    let mut s2 = Survival::load(Content::dev(), Box::new(FlatLand::default()), 2, 1_000_100.0, &back);
    let stations = s2.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { .. })).count();
    assert_eq!(stations, 1);
    assert_eq!(s2.storage[&wood], 9);
    assert_eq!(s2.caches_of("bren").len(), 1);
    let spent = s2.objs.values().filter(|o| matches!(o.kind, ObjKind::Node { ready: false, .. })).count();
    assert_eq!(spent, 1, "the spent node is still spent (no refresh on restart)");
    // Heroes come back where they were, as hurt as they were.
    let p2 = s2.join("ayla", "Ayla", back.players.get("ayla"));
    assert_eq!(s2.players[&p2].health, 77);
    assert!(dist2d(s2.players[&p2].pos, at) < 0.1);
    let ore = s2.content.item("ore").unwrap();
    assert_eq!(s2.players[&p2].bag[&ore], 3);
    // One who left while down gets up at the camp.
    let b2 = s2.join("bren", "Bren", back.players.get("bren"));
    assert!(matches!(s2.players[&b2].action, Action::Dead { .. }));
}

// --- the settlement attack --------------------------------------------------------------------------

fn ready_camp(s: &mut Survival, p: Id) {
    give(s, p, "wood", 30);
    give(s, p, "ore", 10);
    let (at, f) = build_spot(s);
    s.move_player(p, at, f, Moving::default());
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    s.move_player(p, [at[0] + 8.0, at[1], 0.0], f, Moving::default());
    s.build_start(p, "smithy").unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
}

#[test]
fn the_bell_calls_a_warned_raid_only_when_asked_and_the_camp_is_ready() {
    let mut s = world();
    let alarm = s.content.world.alarm;
    let p = hero(&mut s, "Ayla", [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI);
    let bell = *s.objs.iter().find(|(_, o)| o.kind == ObjKind::Alarm).unwrap().0;
    s.interact(p, Some(bell));
    assert!(s.raid_start(p).is_err(), "not without the stations");
    ready_camp(&mut s, p);
    s.move_player(p, [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI, Moving::default());
    let ev = {
        s.interact(p, Some(bell));
        s.step(DT)
    };
    assert!(ev.iter().any(|e| matches!(e, Event::Confirm { .. })), "it asks first");
    s.raid_start(p).unwrap();
    let ev = s.step(DT);
    assert!(s.raid.active());
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { .. })) || !s.raid.marks.is_empty(), "the approach is marked");
    // Nothing comes during the warning.
    let warning = s.content.raid.warning;
    run(&mut s, warning - 1.0);
    assert!(s.mobs.values().all(|m| !m.raider));
    run(&mut s, 2.0);
    assert!(s.mobs.values().any(|m| m.raider), "then they come");
}

#[test]
fn beating_the_raid_fills_the_chest_and_losing_it_wrecks_stations_you_can_mend() {
    let mut s = world();
    let alarm = s.content.world.alarm;
    let p = hero(&mut s, "Ayla", [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI);
    ready_camp(&mut s, p);
    s.move_player(p, [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI, Moving::default());
    let bell = *s.objs.iter().find(|(_, o)| o.kind == ObjKind::Alarm).unwrap().0;
    s.interact(p, Some(bell));
    s.raid_start(p).unwrap();
    s.raid.timer = 0.0;
    // Kill every raider as it comes.
    for _ in 0..2000 {
        let _ = s.step(DT);
        let raiders: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.raider && m.health > 0).map(|(id, _)| *id).collect();
        for r in raiders {
            s.hurt_mob(r, p, 10_000);
        }
        if !s.raid.active() {
            break;
        }
    }
    assert!(!s.raid.active(), "the raid is beaten");
    let ore = s.content.item("ore").unwrap();
    assert!(s.storage.get(&ore).copied().unwrap_or(0) >= 6, "spoils in the chest");
    assert_eq!(s.camp.raids_won, 1);
    // Again (the first raid, as if not yet beaten), but nobody fights back: a station is wrecked
    // and the raid ends.
    s.camp.raids_won = 0;
    s.interact(p, Some(bell));
    s.raid_start(p).unwrap();
    s.raid.timer = 0.0;
    s.hurt_player(p, 10_000);
    let fire = *s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).unwrap().0;
    s.hurt_station(fire, 100_000);
    for _ in 0..2000 {
        let _ = s.step(DT);
        if let Some(pl) = s.players.get_mut(&p) {
            if pl.health > 0 {
                pl.health = 0;
                pl.action = Action::Dead { until: 1e9 };
            }
        }
        if !s.raid.active() {
            break;
        }
    }
    assert!(!s.raid.active(), "with every hero down, the raiders leave");
    assert!(matches!(s.objs[&fire].kind, ObjKind::Station { damaged: true, .. }));
    // Mending it at the station itself (it doesn't need to work).
    s.players.get_mut(&p).unwrap().action = Action::Ready;
    s.players.get_mut(&p).unwrap().health = 100;
    let at = s.objs[&fire].pos;
    s.move_player(p, [at[0] + 2.0, at[1], 0.0], std::f32::consts::PI, Moving::default());
    give(&mut s, p, "wood", 5);
    s.interact(p, Some(fire));
    assert!(matches!(s.objs[&fire].kind, ObjKind::Station { damaged: false, .. }), "mended");
    assert!(!s.objs.values().any(|o| matches!(o.kind, ObjKind::Fire { .. })), "the fire is out");
}

#[test]
fn a_raid_waits_while_nobody_is_online_and_warns_on_return() {
    let mut s = world();
    let alarm = s.content.world.alarm;
    let p = hero(&mut s, "Ayla", [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI);
    ready_camp(&mut s, p);
    s.move_player(p, [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI, Moving::default());
    let bell = *s.objs.iter().find(|(_, o)| o.kind == ObjKind::Alarm).unwrap().0;
    s.interact(p, Some(bell));
    s.raid_start(p).unwrap();
    run(&mut s, 5.0);
    let left = s.raid.timer;
    let saved = s.leave(p).unwrap();
    run(&mut s, 60.0);
    assert!(s.raid.suspended);
    assert!((s.raid.timer - left).abs() < 0.2, "its clock stopped");
    assert!(s.mobs.values().all(|m| !m.raider));
    // Restart in the middle of it: still waiting.
    let mut off = BTreeMap::new();
    off.insert("ayla".to_string(), saved);
    let save = s.save(&off);
    let mut s2 = Survival::load(Content::dev(), Box::new(FlatLand::default()), 3, 1_000_000.0, &save);
    s2.mobs.clear();
    assert!(s2.raid.active() && s2.raid.suspended);
    let p2 = s2.join("ayla", "Ayla", save.players.get("ayla"));
    let _ = p2;
    let ev = s2.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("Resumes"))), "a warning on return");
    assert!(s2.raid.timer >= 9.9);
}

// --- parries, staggers and the weapon rack -----------------------------------------------------------

/// The hero takes up `key` from the rack (standing at it for the moment).
fn take_up(s: &mut Survival, p: Id, key: &str) {
    let back = s.players[&p].pos;
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, key).unwrap();
    s.players.get_mut(&p).unwrap().pos = back;
}

#[test]
fn a_guard_raised_just_in_time_parries_and_the_attacker_reels() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    let (hp, st) = (s.players[&p].health, s.players[&p].stamina);
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Parried);
    assert_eq!((s.players[&p].health, s.players[&p].stamina), (hp, st), "nothing lands and it costs nothing");
    let m = &s.mobs[&k];
    assert!(m.stagger_until > s.clock + 1.0 && m.riposte, "the kobold reels");
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, .. } if *player == p)));
    // A riposte while it reels bites harder than any ordinary blow.
    s.input(p, Input::GuardUp);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.6);
    let (dmg, _) = hits_on(&ev, k)[0];
    assert!(dmg as f32 > 24.0, "riposte {dmg}");
    // A guard held a while only blocks.
    s.input(p, Input::Guard);
    run(&mut s, 1.0);
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Blocked);
}

#[test]
fn twitching_the_guard_parries_nothing_and_missiles_only_block() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    run(&mut s, 0.1);
    s.input(p, Input::GuardUp);
    s.input(p, Input::Guard);
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Blocked, "too soon after the last raise");
    run(&mut s, 1.0);
    s.input(p, Input::GuardUp);
    s.input(p, Input::Guard);
    assert_eq!(s.strike_player(k, p, 16, [2.0, 0.0, 0.0], true, false, false, Blow::Light), Outcome::Blocked, "a missile isn't parried");
}

#[test]
fn a_creatures_own_blow_is_parried_through_its_wind_up() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "wolf", [2.0, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    // Wait for the wind-up, then raise the guard just before the bite.
    let mut ev = Vec::new();
    for _ in 0..100 {
        ev.extend(s.step(DT));
        if let MobState::Windup { t, .. } = s.mobs[&k].state {
            if t >= s.attack_now(k).windup - 0.15 {
                break;
            }
        }
    }
    s.input(p, Input::Guard);
    let ev = run(&mut s, 0.4);
    assert!(hits_on(&ev, p).iter().any(|(_, o)| *o == Outcome::Parried), "{:?}", hits_on(&ev, p));
    assert!(ev.iter().any(|e| matches!(e, Event::Emote { id, emote: mobs::STUN_EMOTE } if *id == k)), "it reels");
}

#[test]
fn a_heavy_weapon_knocks_a_wind_up_aside() {
    let mut s = world();
    // (The old rules: any heavy blow knocked a wind-up aside, again and again.)
    s.content.combat.heavy_knocks_aside = true;
    s.content.combat.knock_aside_wary = 0.0;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    take_up(&mut s, p, "maul");
    // (The brute's own poise shrugs off a light blow: that's the next test.)
    let b = s.content.mob("overseer").unwrap();
    s.content.mobs[b].attack.poise = Some(false);
    let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    for _ in 0..40 {
        s.step(DT);
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            break;
        }
    }
    assert!(matches!(s.mobs[&k].state, MobState::Windup { .. }));
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.9);
    assert!(!hits_on(&ev, k).is_empty(), "the maul lands");
    assert!(hits_on(&ev, p).is_empty(), "and the brute's blow never comes: {:?}", hits_on(&ev, p));
    assert!(s.mobs[&k].stagger_until > s.clock, "it reels");
}

/// A creature `k` fighting `p`, run on until it's in the state `want` names.
fn until_state(s: &mut Survival, k: Id, want: &str) {
    for _ in 0..200 {
        let st = &s.mobs[&k].state;
        let now = match st {
            MobState::Windup { .. } => "windup",
            MobState::Recover { .. } => "recover",
            _ => "",
        };
        if now == want {
            return;
        }
        s.step(DT);
    }
    panic!("never got to {want}: {:?}", s.mobs[&k].state);
}

#[test]
fn a_wind_up_goes_through_a_heavy_blow() {
    let mut s = world();
    assert!(!s.content.combat.heavy_knocks_aside);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // A sword: nothing of its own that knocks a wind-up aside.
    take_up(&mut s, p, "sword");
    assert_eq!(s.content.weapon("sword").unwrap().stagger, 0.0);
    let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    until_state(&mut s, k, "windup");
    // Hitting first doesn't stop it, a heavy blow neither: the maul lands, and so does the
    // brute's blow.
    s.input(p, Input::Heavy);
    let ev = run(&mut s, 2.0);
    assert!(!hits_on(&ev, k).is_empty(), "the maul lands");
    assert!(!hits_on(&ev, p).is_empty(), "and the brute's blow comes all the same");
    assert!(!ev.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == k && *emote == crate::sim::mobs::STUN_EMOTE)), "it never reels");
}

#[test]
fn a_mace_knocks_a_wind_up_aside_but_not_twice_running_and_never_a_boss() {
    for boss in [false, true] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        take_up(&mut s, p, "maul");
        let b = s.content.mob("overseer").unwrap();
        s.content.mobs[b].attack.poise = Some(false);
        s.content.mobs[b].attack.windup = 3.0;
        s.content.mobs[b].patterns.clear();
        s.content.mobs[b].boss = boss;
        s.content.mobs[b].posture = 0;
        let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
        s.mobs.get_mut(&k).unwrap().health = 5000;
        let stuns = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::Emote { id, emote } if *id == k && *emote == crate::sim::mobs::STUN_EMOTE)).count();
        let knock = |s: &mut Survival| {
            s.engage(k, Foe::Hero(p));
            s.step(DT);
            s.mobs.get_mut(&k).unwrap().state = MobState::Windup { foe: Foe::Hero(p), t: 0.0, mark: None, shown: true };
            s.mobs.get_mut(&k).unwrap().stagger_until = 0.0;
            s.players.get_mut(&p).unwrap().stamina = 100.0;
            s.input(p, Input::Attack);
            run(s, 1.5)
        };
        let ev = knock(&mut s);
        assert_eq!(stuns(&ev), usize::from(!boss), "the first (boss {boss})");
        s.players.get_mut(&p).unwrap().health = s.players[&p].max_health;
        let ev = knock(&mut s);
        assert_eq!(stuns(&ev), 0, "wary of it a moment later (boss {boss})");
    }
}

#[test]
fn a_heavy_blow_just_after_its_blow_staggers_it_but_never_a_boss() {
    for boss in [false, true] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        take_up(&mut s, p, "maul");
        let b = s.content.mob("overseer").unwrap();
        s.content.mobs[b].boss = boss;
        // (No stance to break: only the window counts here.)
        s.content.mobs[b].posture = 0;
        let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
        s.mobs.get_mut(&k).unwrap().health = 5000;
        // Just after its blow, a heavy blow in the window.
        s.players.get_mut(&p).unwrap().health = 5000;
        s.players.get_mut(&p).unwrap().max_health = 5000;
        s.engage(k, Foe::Hero(p));
        s.step(DT);
        s.mobs.get_mut(&k).unwrap().state = MobState::Recover { foe: Foe::Hero(p), until: s.clock + 5.0 };
        s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 5.0;
        s.input(p, Input::Heavy);
        let ev = run(&mut s, 2.0);
        assert!(!hits_on(&ev, k).is_empty(), "the maul lands");
        let reeled = ev.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == k && *emote == crate::sim::mobs::STUN_EMOTE));
        assert_eq!(reeled, !boss, "boss {boss}");
    }
}

#[test]
fn a_kick_stops_a_boss_only_in_a_cast() {
    for boss in [false, true] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        let b = s.content.mob("overseer").unwrap();
        s.content.mobs[b].boss = boss;
        s.content.mobs[b].attack.windup = 3.0;
        let k = mob(&mut s, "overseer", [2.0, 0.0, 0.0]);
        s.mobs.get_mut(&k).unwrap().health = 5000;
        s.engage(k, Foe::Hero(p));
        s.step(DT);
        s.mobs.get_mut(&k).unwrap().state = MobState::Windup { foe: Foe::Hero(p), t: 0.0, mark: None, shown: true };
        s.input(p, Input::Kick);
        let ev = run(&mut s, 0.6);
        let reeled = ev.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == k && *emote == crate::sim::mobs::STUN_EMOTE));
        assert_eq!(reeled, !boss, "a boss's blow goes on through a kick (boss {boss})");
    }
}

#[test]
fn foes_hit_harder_and_grow_with_the_heroes_in_their_fight() {
    let mut s = world();
    s.content.combat.foe_damage = 1.5;
    s.content.combat.hero_health = 0.7;
    s.content.combat.hero_damage = 0.1;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "boar", [2.0, 0.0, 0.0]);
    let base = s.mobs[&k].max_health;
    s.engage(k, Foe::Hero(p));
    s.step(DT);
    assert_eq!(s.mobs[&k].max_health, base, "one hero: as it is");
    assert!((s.foe_blow_share(k) - 1.5).abs() < 1e-4);
    // Hurt a little, then a friend comes: more health, what it took stays taken.
    s.mobs.get_mut(&k).unwrap().health = base - 10;
    let q = hero(&mut s, "Bren", [0.0, 3.0, 0.0], 0.0);
    s.step(DT);
    let grown = (base as f32 * 1.7).round() as u32;
    assert_eq!(s.mobs[&k].max_health, grown);
    assert_eq!(s.mobs[&k].health, grown - 10);
    assert!((s.foe_blow_share(k) - 1.65).abs() < 1e-4);
    // The friend walking off doesn't shrink it mid-fight.
    s.move_player(q, [200.0, 0.0, 0.0], 0.0, Moving::default());
    s.step(DT);
    assert_eq!(s.mobs[&k].max_health, grown);
    // Home again, it's itself.
    let home = s.mobs[&k].home;
    s.mobs.get_mut(&k).unwrap().pos = home;
    s.mobs.get_mut(&k).unwrap().state = MobState::Return;
    s.step(DT);
    assert_eq!(s.mobs[&k].max_health, base);
    assert_eq!(s.mobs[&k].fighters, 1);
    // A creature's blow lands as hard as its share says.
    let k2 = mob(&mut s, "boar", [2.0, 0.0, 0.0]);
    s.strike_fair(k2, p, 20, [2.0, 0.0, 0.0], false, false, false, Blow::Light, None, Follow::None);
    let ev = run(&mut s, 0.3);
    assert!(hits_on(&ev, p).iter().any(|(d, _)| *d == 30), "{:?}", hits_on(&ev, p));
}

#[test]
fn two_handed_weapons_guard_with_themselves_and_sweep_wider() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    assert_eq!(s.guard_of(&s.players[&p]).key, "buckler");
    take_up(&mut s, p, "greatsword");
    assert_eq!(s.guard_of(&s.players[&p]).key, "blade_guard", "no shield in a two-handed grip");
    // Three wolves in front: the claymore takes them all, a sword only one.
    let ws: Vec<Id> = (0..3).map(|i| mob(&mut s, "wolf", [2.2, -1.0 + i as f32, 0.0])).collect();
    assert_eq!(s.foes_in_reach(p), 3);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.3);
    assert_eq!(ws.iter().filter(|w| !hits_on(&ev, **w).is_empty()).count(), 3);
    take_up(&mut s, p, "sword");
    for w in &ws {
        s.mobs.get_mut(w).unwrap().health = 60;
    }
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert_eq!(ws.iter().filter(|w| !hits_on(&ev, **w).is_empty()).count(), 1);
}

#[test]
fn a_crossbow_shoots_at_once_and_reloads_long() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // (Not from the rack any more: bows are off it.)
    let crossbow = s.content.item("crossbow").unwrap();
    s.players.get_mut(&p).unwrap().bow = crossbow;
    let k = mob(&mut s, "boar", [12.0, 0.0, 0.0]);
    let _ = s.slot_ability(p, "ranged", "aimed_shot");
    s.input(p, SHOT);
    s.input(p, Input::Attack);
    assert_eq!(s.players[&p].action.name(), "cast", "no draw");
    let ev = run(&mut s, 0.5);
    assert_eq!(hits_on(&ev, k), vec![(34, Outcome::Hit)]);
    assert!(s.commit_left(p).is_some_and(|l| l > 0.8), "still reloading");
}

#[test]
fn the_rack_arms_heroes_only_at_the_rack() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", [-9400.0, 0.0, 0.0], 0.0);
    assert!(s.arm(p, "halberd").is_err(), "far from the rack");
    take_up(&mut s, p, "halberd");
    let c = &s.content;
    assert_eq!(c.items[s.players[&p].weapon].key, "halberd");
    let view = s.rack_view(p);
    assert!(view.iter().any(|(k, _, kind, held, _)| k == "halberd" && *kind == "two-handed" && *held));
    assert!(view.iter().any(|(_, _, kind, _, _)| *kind == "shield"), "shields are there");
    // No bows or crossbows: ranged fighting is off the board.
    let ranged = |k: &str| c.item(k).is_some_and(|i| c.items[i].kind == crate::content::ItemKind::Bow);
    assert!(!view.iter().any(|(k, ..)| ranged(k)), "no bows on the rack: {:?}", view.iter().map(|v| &v.0).collect::<Vec<_>>());
    assert!(!view.iter().any(|(_, _, kind, _, _)| *kind == "ranged"));
    assert!(s.arm(p, "crossbow").is_err(), "nor taken from it by name");
    assert!(s.arm(p, "bow").is_err());
}

// --- heroes against heroes, sprinting -----------------------------------------------------------------

fn pvp_world() -> Survival {
    let mut s = world();
    s.content.pvp = Content::dev().pvp;
    assert!(s.content.pvp.friendly_fire);
    s
}

#[test]
fn with_friendly_fire_heroes_strike_block_parry_and_fell_each_other() {
    let mut s = pvp_world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [2.0, 0.0, 0.0], std::f32::consts::PI);
    assert_eq!(s.foes_in_reach(a), 1);
    // A plain swing lands.
    s.input(a, Input::Attack);
    let ev = run(&mut s, 0.9);
    assert!(hits_on(&ev, b).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0));
    // A guard raised long before only blocks.
    s.input(b, Input::Guard);
    run(&mut s, 1.0);
    s.input(a, Input::Attack);
    let ev = run(&mut s, 0.9);
    assert_eq!(hits_on(&ev, b).last().map(|h| h.1), Some(Outcome::Blocked));
    // Raised just as the blow comes: parried, and Ayla reels, open to a riposte.
    s.input(b, Input::GuardUp);
    run(&mut s, 0.8);
    s.input(a, Input::Attack);
    run(&mut s, 0.2);
    s.input(b, Input::Guard);
    let ev = run(&mut s, 0.3);
    assert_eq!(hits_on(&ev, b).last().map(|h| h.1), Some(Outcome::Parried));
    assert_eq!(s.players[&a].action.name(), "stagger");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, .. } if *player == a)));
    s.input(b, Input::GuardUp);
    s.input(b, Input::Attack);
    let ev = run(&mut s, 0.6);
    let (dmg, _) = hits_on(&ev, a)[0];
    assert!(dmg > 24, "the riposte bites deeper: {dmg}");
    // Down to the last blow: felled, announced, and up again at once, whole, carrying all.
    s.players.get_mut(&a).unwrap().health = 5;
    s.players.get_mut(&a).unwrap().bag.insert(s.content.item("wood").unwrap(), 4);
    s.players.get_mut(&b).unwrap().stamina = 100.0;
    run(&mut s, 0.5);
    s.input(b, Input::Attack);
    let ev = run(&mut s, 0.6);
    assert!(
        ev.iter().any(|e| matches!(e, Event::Kill { killer: Some(k), victim } if k == "Bren" && victim == "Ayla")),
        "{ev:?}"
    );
    assert!(s.caches_of("ayla").is_empty(), "no sack");
    run(&mut s, 1.2);
    let p = &s.players[&a];
    assert_eq!(p.health, p.max_health, "up again, whole");
    assert_eq!(p.bag.get(&s.content.item("wood").unwrap()), Some(&4));
}

#[test]
fn an_arrow_finds_another_hero_and_a_shield_turns_it() {
    let mut s = pvp_world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [15.0, 0.0, 0.0], std::f32::consts::PI);
    let ev = draw_and_loose(&mut s, a, 0.8);
    assert_eq!(hits_on(&ev, b).len(), 1);
    assert_eq!(hits_on(&ev, b)[0].1, Outcome::Hit);
    s.input(b, Input::Guard);
    run(&mut s, 0.8);
    let ev = draw_and_loose(&mut s, a, 0.8);
    assert_eq!(hits_on(&ev, b)[0].1, Outcome::Blocked, "a shield turns it (no parrying arrows)");
}

#[test]
fn sprinting_is_faster_drains_stamina_and_stops_for_anything_else() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Sprint);
    let ev = run(&mut s, 1.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Speed { player, speed, .. } if *player == p && *speed > 9.0)), "faster");
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { spell: SPRINT_SPELL, .. })), "no look or sound of its own");
    assert!(s.players[&p].stamina < 99.0 && s.players[&p].stamina > 90.0, "it costs a little stamina");
    // A swing ends the sprint; standing still too.
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.1);
    assert!(ev.iter().any(|e| matches!(e, Event::Speed { speed, .. } if *speed < 4.0)));
    run(&mut s, 1.0);
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    run(&mut s, 0.2);
    assert!(!s.players[&p].sprinting);
    // Run dry: it stops, and doesn't flicker back on a sliver of stamina.
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    run(&mut s, 0.1);
    assert!(s.players[&p].sprinting, "going again while held");
    s.players.get_mut(&p).unwrap().stamina = 0.5;
    run(&mut s, 0.1);
    assert!(!s.players[&p].sprinting);
    s.players.get_mut(&p).unwrap().stamina = 8.0;
    run(&mut s, 0.1);
    assert!(!s.players[&p].sprinting, "not on a sliver");
    s.players.get_mut(&p).unwrap().stamina = 50.0;
    let ev = run(&mut s, 0.1);
    assert!(s.players[&p].sprinting, "with enough back");
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { spell: SPRINT_SPELL, .. })), "nor when it starts again");
    s.input(p, Input::SprintUp);
    run(&mut s, 0.1);
    assert!(!s.players[&p].sprinting, "let go");
}

#[test]
fn walking_is_the_clients_own_pace_and_back_to_running_runs() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let run_speed = s.content.player.run_speed;
    let speeds = |ev: &[Event]| -> Vec<f32> {
        ev.iter().filter_map(|e| if let Event::Speed { player, speed, .. } = e { (*player == p).then_some(*speed) } else { None }).collect()
    };
    // Walking (X), the guard up and down: the run speed the client is told comes back to a run,
    // never a walk's pace (which it would keep after X back to running).
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, walk: true, ..Default::default() });
    s.input(p, Input::Guard);
    let mut ev = run(&mut s, 0.3);
    s.input(p, Input::GuardUp);
    ev.extend(run(&mut s, 0.3));
    let told = speeds(&ev);
    assert!(!told.is_empty(), "the guard changes the pace");
    assert!(told.iter().all(|v| *v > WALK_SPEED + 0.1), "never a walk's pace as the run speed: {told:?}");
    assert!((told.last().unwrap() - run_speed).abs() < 0.01, "back to a run: {told:?}");
    // Shift while walking: no sprint, and no stamina spent on one.
    let before = s.players[&p].stamina;
    s.input(p, Input::Sprint);
    let ev = run(&mut s, 1.0);
    assert!(!s.players[&p].sprinting, "no sprint at a walk");
    assert!(s.players[&p].stamina >= before, "nothing spent");
    assert!(speeds(&ev).is_empty(), "nor a sprint's pace sent");
    // X back to running, Shift still held: now it sprints.
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    run(&mut s, 0.1);
    assert!(s.players[&p].sprinting, "running again, it sprints");
}

#[test]
fn the_server_reckons_a_walking_hero_at_a_walk() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, walk: true, ..Default::default() });
    let from = s.players[&p].pos;
    run(&mut s, 1.0);
    let to = s.players[&p].pos;
    let went = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
    // (Reckoned for RECKON_FOR, 0.6 s: about 1.5 yards at a walk, 4.2 at a run.)
    assert!(went > 0.5 && went <= WALK_SPEED + 0.3, "a walk's pace, not a run's: {went}");
}

#[test]
fn a_roll_made_while_walking_keeps_its_burst() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, walk: true, ..Default::default() });
    let from = s.players[&p].pos;
    s.input(p, Input::Dodge);
    // Within the dash: the roll's pace (16 yd/s), not a walk's (2.5). The client runs it at full
    // speed, so the server's hero mustn't trail it while blows are judged.
    run(&mut s, 0.2);
    let to = s.players[&p].pos;
    let went = ((to[0] - from[0]).powi(2) + (to[1] - from[1]).powi(2)).sqrt();
    assert!(went > WALK_SPEED, "the dash's pace while walking: {went}");
}


// --- move sets, cancels, shoves, the lock-on and lag --------------------------------------------

fn strike_of(s: &Survival, p: Id) -> Option<Strike> {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => Some(strike.clone()),
        _ => None,
    }
}

#[test]
fn light_attacks_chain_through_the_set_late_in_each_recovery() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.input(p, Input::Attack);
    let first = strike_of(&s, p).unwrap();
    assert_eq!((first.step, first.clip, first.take), (0, 17, 0));
    // Pressed again early: it waits (the buffer), not chained at once.
    run(&mut s, 0.2);
    s.input(p, Input::Attack);
    assert_eq!(strike_of(&s, p).unwrap().step, 0);
    // Late in the recovery: the second blow comes at once, the next take.
    let late = first.windup + first.active + first.recovery * 0.5;
    run(&mut s, late - 0.2);
    s.input(p, Input::Attack);
    let second = strike_of(&s, p).unwrap();
    assert_eq!((second.step, second.clip, second.take), (1, 17, 1), "chained");
    // And the third, the set's finisher: slower, harder.
    run(&mut s, second.windup + second.active + second.recovery * 0.5);
    s.input(p, Input::Attack);
    let third = strike_of(&s, p).unwrap();
    assert_eq!((third.step, third.clip), (2, 57));
    assert!(third.windup > first.windup && third.damage > first.damage);
    // After a pause the chain starts over.
    run(&mut s, 3.0);
    s.input(p, Input::Attack);
    assert_eq!(strike_of(&s, p).unwrap().step, 0, "a fresh chain");
}

#[test]
fn a_dodge_cuts_a_blow_short_only_late_in_its_recovery() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.input(p, Input::Attack);
    let st = strike_of(&s, p).unwrap();
    // In the wind-up: committed (it waits for the end).
    run(&mut s, 0.1);
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action.name(), "swing");
    // Late in the recovery: out at once.
    run(&mut s, st.windup + st.active + st.recovery * 0.6 - 0.1);
    s.input(p, Input::Dodge);
    assert_eq!(s.players[&p].action.name(), "dodge");
}

#[test]
fn a_heavy_attack_and_a_running_attack_are_their_own_blows() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.input(p, Input::Heavy);
    let heavy = strike_of(&s, p).unwrap();
    assert_eq!((heavy.kind, heavy.clip), (StrikeKind::Heavy, 57));
    run(&mut s, 3.0);
    // Sprinting forward, an attack is the running one, with a long lunge.
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Sprint);
    run(&mut s, 0.2);
    s.input(p, Input::Attack);
    let running = strike_of(&s, p).unwrap();
    assert_eq!(running.kind, StrikeKind::Running);
    assert!(running.lunge > 2.0 && running.damage > 1.0);
}

#[test]
fn a_shield_bash_knocks_a_creature_reeling_and_a_kick_breaks_a_heros_guard() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.input(p, Input::Guard);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.5);
    assert!(s.mobs[&k].stagger_until > s.clock, "reeling");
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim, .. } if *id == k && *anim == STUN_ANIM)));
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == k && *kit == KIT_BASH)));

    // Heroes, with friendly fire on: a kick through a raised guard tears its stamina, and breaks
    // one with little left.
    let mut c = Content::dev();
    c.pvp.friendly_fire = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bern", [1.8, 0.0, 0.0], std::f32::consts::PI);
    s.input(b, Input::Guard);
    s.players.get_mut(&b).unwrap().stamina = 30.0;
    s.input(a, Input::Guard);
    s.input(a, Input::Heavy);
    assert_eq!(s.players[&a].action.name(), "kick");
    let ev = run(&mut s, 0.5);
    assert!(hits_on(&ev, b).iter().any(|(_, o)| *o == Outcome::GuardBroken), "{:?}", hits_on(&ev, b));
    assert_eq!(s.players[&b].action.name(), "stagger");
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, speed, .. } if *player == b && *speed > 0.0)), "shoved");
}

#[test]
fn the_lock_on_takes_the_foe_in_front_and_lets_go() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ahead = mob(&mut s, "grunt", [8.0, 1.0, 0.0]);
    let _behind = mob(&mut s, "grunt", [-4.0, 0.0, 0.0]);
    s.input(p, Input::Lock);
    let ev = s.step(DT);
    assert_eq!(s.players[&p].lock, Some(ahead));
    assert!(ev.iter().any(|e| matches!(e, Event::Lock { player, target: Some(t) } if *player == p && *t == ahead)));
    // Tab again with no other foe in view: the lock stays; held, it lets go.
    s.input(p, Input::Lock);
    assert_eq!(s.players[&p].lock, Some(ahead));
    s.input(p, Input::Unlock);
    assert_eq!(s.players[&p].lock, None);
}

#[test]
fn a_jump_costs_stamina() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.jumped(p);
    assert!((s.players[&p].stamina - (100.0 - s.content.player.jump_stamina)).abs() < 0.01);
}

#[test]
fn tab_again_switches_round_the_foes_in_view() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "grunt", [5.0, 0.0, 0.0]);
    let b = mob(&mut s, "grunt", [8.0, 3.0, 0.0]);
    let c = mob(&mut s, "grunt", [9.0, -4.0, 0.0]);
    s.input(p, Input::Lock);
    assert_eq!(s.players[&p].lock, Some(a));
    let mut seen = vec![a];
    for _ in 0..2 {
        s.input(p, Input::Lock);
        let now = s.players[&p].lock.unwrap();
        assert!(!seen.contains(&now), "a foe not yet switched through");
        seen.push(now);
    }
    assert!(seen.contains(&b) && seen.contains(&c));
    // Round again.
    s.input(p, Input::Lock);
    assert!(s.players[&p].lock.is_some());
    assert_ne!(s.players[&p].lock, Some(seen[2]));
}

#[test]
fn a_dodge_pressed_as_the_blow_was_seen_still_counts_a_round_trip_later() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [1.5, 0.0, 0.0]);
    s.set_rtt(p, 0.15);
    // The blow lands on the server...
    s.strike_fair(k, p, 20, [1.5, 0.0, 0.0], true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(0.05);
    assert!(hits_on(&ev, p).is_empty(), "held for their round trip");
    // ...and the dodge they pressed when they saw it arrives just after.
    s.input(p, Input::Dodge);
    let ev = run(&mut s, 0.2);
    assert_eq!(hits_on(&ev, p), vec![(0, Outcome::Dodged)]);
    // With no round trip, the same blow lands at once.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [1.5, 0.0, 0.0]);
    s.strike_fair(k, p, 20, [1.5, 0.0, 0.0], true, false, true, Blow::Light, None, Follow::None);
    let ev = s.step(0.0);
    assert_eq!(hits_on(&ev, p).len(), 1);
}

#[test]
fn a_swing_lands_where_its_striker_saw_the_creature() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.5, 0.0, 0.0]);
    run(&mut s, 0.5);
    s.set_rtt(p, 0.2);
    s.input(p, Input::Attack);
    let st = strike_of(&s, p).unwrap();
    // Just before the blow, the kobold steps out of reach on the server: the striker, a round
    // trip behind, still saw it in reach.
    run(&mut s, st.windup - 0.1);
    s.mobs.get_mut(&k).unwrap().pos = [6.0, 0.0, 0.0];
    let ev = run(&mut s, 0.3);
    assert_eq!(hits_on(&ev, k).len(), 1, "struck where it was seen");
    // With no round trip the same step out is a miss.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.5, 0.0, 0.0]);
    run(&mut s, 0.5);
    s.input(p, Input::Attack);
    run(&mut s, st.windup - 0.1);
    s.mobs.get_mut(&k).unwrap().pos = [6.0, 0.0, 0.0];
    let ev = run(&mut s, 0.3);
    assert!(hits_on(&ev, k).is_empty());
}


#[test]
fn a_heavy_creatures_wind_up_shrugs_off_a_light_blow_but_not_a_heavy_one() {
    let windup_then = |s: &mut Survival, k: Id| {
        for _ in 0..40 {
            s.step(DT);
            if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
                break;
            }
        }
        assert!(matches!(s.mobs[&k].state, MobState::Windup { .. }));
    };
    // The maul's light blow lands, but the brute's swing still comes.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    take_up(&mut s, p, "maul");
    let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    windup_then(&mut s, k);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 1.2);
    assert!(!hits_on(&ev, k).is_empty(), "the maul lands");
    assert!(s.mobs[&k].stagger_until <= s.clock, "no reel");
    // A bash breaks it.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "overseer", [2.0, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    windup_then(&mut s, k);
    s.input(p, Input::Bash);
    let ev = run(&mut s, 0.5);
    assert!(!hits_on(&ev, k).is_empty(), "the bash lands");
    assert!(s.mobs[&k].stagger_until > s.clock, "it reels");
    assert!(s.mobs[&k].riposte, "open to a riposte");
}

#[test]
fn a_kick_through_a_guard_puts_a_hero_down_open_to_a_riposte() {
    let mut s = world();
    s.content.pvp.friendly_fire = true;
    let a = hero(&mut s, "Ann", ORIGIN, 0.0);
    let b = hero(&mut s, "Ben", [2.0, 0.0, 0.0], std::f32::consts::PI);
    s.input(b, Input::Guard);
    s.players.get_mut(&b).unwrap().stamina = 20.0;
    s.input(a, Input::Kick);
    let ev = run(&mut s, 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim: KNOCKDOWN_ANIM, fit, .. } if *id == b && *fit > 1.0)), "down");
    assert_eq!(s.players[&b].action.name(), "stagger");
    assert!(s.players[&b].riposte_until > s.clock, "open to a riposte");
}

#[test]
fn stamina_waits_for_a_blow_to_end_before_it_comes_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.input(p, Input::Heavy);
    s.input(p, Input::AttackUp);
    let after = s.players[&p].stamina;
    // Well past the usual pause, but the heavy attack is still under way.
    run(&mut s, 1.0);
    assert!(matches!(s.players[&p].action, Action::Swing { .. }), "still swinging");
    assert!((s.players[&p].stamina - after).abs() < 0.01, "nothing back yet: {} vs {after}", s.players[&p].stamina);
}

#[test]
fn a_creature_now_and_then_winds_up_its_flashing_special_blow() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // A creature with a plain blow and a special one now and then (here, every time).
    let b = s.content.mob("wolf").unwrap();
    s.content.mobs[b].patterns.clear();
    let sp: crate::content::SpecialDef = toml::from_str("chance = 1.0\nwindup = 0.8\ndamage = 14\nrange = 3.6\nkit = 81").unwrap();
    s.content.mobs[b].special = Some(sp.clone());
    let k = mob(&mut s, "wolf", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(DT));
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            break;
        }
    }
    assert!(s.mobs[&k].special);
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == k && *kit == sp.kit)), "the flash");
    let want = sp.windup.unwrap();
    assert!(ev.iter().any(|e| matches!(e, Event::MobWindup { id, take, windup, .. } if *id == k && *take == sp.take && (*windup - want).abs() < 1e-4)), "its swing");
}

#[test]
fn the_server_carries_a_running_hero_on_between_reports() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    run(&mut s, 0.3);
    let x = s.players[&p].pos[0];
    assert!(x > 1.5 && x < 2.5, "about 0.3 s of running on: {x}");
    // Not for long without word from the client.
    run(&mut s, 1.0);
    assert!(s.players[&p].pos[0] < 5.0, "it stops guessing: {}", s.players[&p].pos[0]);
}

/// Build station `key` where hero `p` stands (paying for it) and hand back its id.
fn put_up(s: &mut Survival, p: Id, key: &str) -> Id {
    let def = s.content.station(key).unwrap();
    for (k, n) in s.content.stations[def].cost.clone() {
        give(s, p, &k, n);
    }
    let before: Vec<Id> = s.objs.keys().copied().collect();
    s.build_start(p, key).unwrap();
    let _ = s.step(DT);
    s.build_confirm(p).unwrap();
    *s.objs.iter().find(|(id, o)| !before.contains(id) && matches!(o.kind, ObjKind::Station { .. })).unwrap().0
}

#[test]
fn armor_takes_its_share_off_a_blow_and_mail_slows_the_stamina() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    let k = mob(&mut s, "grunt", [at[0] + 2.0, at[1], 0.0]);
    let hit = |s: &mut Survival| {
        let full = s.players[&p].max_health;
        s.players.get_mut(&p).unwrap().health = full;
        s.strike_player(k, p, 40, [at[0] + 2.0, at[1], 0.0], true, false, false, Blow::Light);
        full - s.players[&p].health
    };
    assert_eq!(hit(&mut s), 40);
    let rack = put_up(&mut s, p, "tanning");
    give(&mut s, p, "leather", 6);
    give(&mut s, p, "linen", 3);
    s.craft(p, rack, "hide_jerkin").unwrap();
    s.craft(p, rack, "leather_cap").unwrap();
    assert_eq!(hit(&mut s), 34, "a tenth and a little off");
    // Mail: more off, stamina back slower; the jerkin goes in the chest.
    s.move_player(p, [at[0] - 7.0, at[1], 0.0], f, Moving::default());
    let anvil = put_up(&mut s, p, "smithy");
    give(&mut s, p, "bronze", 4);
    give(&mut s, p, "leather", 2);
    s.craft(p, anvil, "bronze_mail").unwrap();
    s.move_player(p, at, f, Moving::default());
    assert_eq!(hit(&mut s), 30);
    assert_eq!(s.storage.get(&s.content.item("hide_jerkin").unwrap()), Some(&1));
    let regen = |s: &mut Survival| {
        s.players.get_mut(&p).unwrap().stamina = 10.0;
        s.players.get_mut(&p).unwrap().spent_at = -100.0;
        run(s, 0.5);
        s.players[&p].stamina - 10.0
    };
    let slow = regen(&mut s);
    assert!(slow < 16.0 * 0.95 && slow > 16.0 * 0.8, "mail slows it: {slow}");
}

#[test]
fn a_stew_feeds_a_salve_heals_on_and_a_stone_hones_the_blade() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    give(&mut s, p, "salve", 1);
    give(&mut s, p, "stew", 1);
    give(&mut s, p, "whetstone", 1);
    // R uses the first; G picks the next.
    let salve = s.content.item("salve").unwrap();
    let stew = s.content.item("stew").unwrap();
    let whet = s.content.item("whetstone").unwrap();
    s.players.get_mut(&p).unwrap().health = 40;
    s.use_item(p, salve);
    run(&mut s, 1.0);
    let after_use = s.players[&p].health;
    assert!(after_use >= 60, "a tenth at once: {after_use}");
    run(&mut s, 5.0);
    assert!(s.players[&p].health > after_use + 20, "and on over time: {}", s.players[&p].health);
    s.use_item(p, stew);
    run(&mut s, 2.5);
    assert!(s.effect_of(&s.players[&p], "stamina") > 0.15, "well fed");
    // Honed: the same blow does more.
    s.use_item(p, whet);
    run(&mut s, 2.5);
    assert!(s.effect_of(&s.players[&p], "might") > 0.05);
    let ev = s.step(0.0);
    let _ = ev;
    // Run out.
    run(&mut s, 125.0);
    assert_eq!(s.effect_of(&s.players[&p], "might"), 0.0, "it wears off");
}

#[test]
fn g_cycles_the_quick_item_and_a_pack_holds_more() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    give(&mut s, p, "salve", 1);
    give(&mut s, p, "tonic", 1);
    let first = s.quick_item(&s.players[&p]).unwrap();
    s.input(p, Input::Cycle);
    let second = s.quick_item(&s.players[&p]).unwrap();
    assert_ne!(first, second);
    let n = s.usable_items(&s.players[&p]).len();
    for _ in 1..n {
        s.input(p, Input::Cycle);
    }
    assert_eq!(s.quick_item(&s.players[&p]), Some(first), "round again");
    let wood = s.content.item("wood").unwrap();
    let base = s.stack_for(wood, s.players[&p].pack);
    let rack = {
        let (at, f) = build_spot(&s);
        s.move_player(p, at, f, Moving::default());
        put_up(&mut s, p, "tanning")
    };
    give(&mut s, p, "linen", 6);
    give(&mut s, p, "leather", 2);
    s.craft(p, rack, "linen_pack").unwrap();
    assert_eq!(s.stack_for(wood, s.players[&p].pack), base * 3 / 2, "timber stacks half again as high");
    // Kept across a save.
    let save = s.save(&Default::default());
    let ps = &save.players["ayla"];
    assert_eq!(ps.pack.as_deref(), Some("linen_pack"));
}

#[test]
fn a_node_gives_its_loot_too() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let def = s.content.nodes.iter().position(|n| n.item == "ore").unwrap();
    s.content.nodes[def].loot = BTreeMap::from([("stone".to_string(), [2, 2])]);
    let node = s.alloc();
    s.objs.insert(node, Obj { kind: ObjKind::Node { def, key: "test".into(), uses: 2, ready: true, back_at: 0.0 }, pos: [1.0, 0.0, 0.0], facing: 0.0 });
    s.gathered(p, node);
    assert_eq!(s.players[&p].bag.get(&s.content.item("stone").unwrap()), Some(&2));
}


#[test]
fn each_raid_beaten_calls_the_next_once_the_camp_has_what_it_needs_and_the_last_comes_back_harder() {
    let c = Content::dev();
    let first = c.raid.plan(0);
    let second = c.raid.plan(1);
    let last = c.raid.plan(c.raid.count() as u32 - 1);
    let again = c.raid.plan(c.raid.count() as u32);
    assert_eq!(first.title, "Warsong Raid");
    assert_eq!(second.title, "Darkspear Raid");
    assert_ne!(second.needs, first.needs);
    let n = |p: &crate::content::RaidPlan| p.raiders.iter().map(|r| r.1).sum::<usize>();
    assert_eq!(n(&again), n(&last) + last.raiders.len(), "a raider more of each kind");
    // The bell asks for what the next raid needs.
    let mut s = world();
    let alarm = s.content.world.alarm;
    let p = hero(&mut s, "Ayla", [alarm[0] + 1.5, alarm[1], 0.0], std::f32::consts::PI);
    s.camp.raids_won = 1;
    let bell = *s.objs.iter().find(|(_, o)| o.kind == ObjKind::Alarm).unwrap().0;
    let ev = {
        s.interact(p, Some(bell));
        s.step(0.0)
    };
    assert!(ev.iter().any(|e| matches!(e, Event::Tell { text, .. } if text.contains("Tanning Rack"))), "{ev:?}");
}

#[test]
fn raiders_break_the_palisade_first_and_the_spikes_bite() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    let fire = put_up(&mut s, p, "cooking");
    s.move_player(p, [at[0] - 7.0, at[1], 0.0], f, Moving::default());
    let wall = put_up(&mut s, p, "barricade");
    // A raider beside the barricade, the fire a little farther: it goes for the barricade.
    let wpos = s.objs[&wall].pos;
    let kobold = s.content.mob("grunt").unwrap();
    let r = s.spawn_mob(kobold, [wpos[0] - 1.5, wpos[1], 0.0], None, true);
    s.raid.phase = RaidPhase::Active;
    s.raid.raiders.push(r);
    // Out of the way, so it goes for the stations.
    s.move_player(p, [at[0] + 60.0, at[1], 0.0], f, Moving::default());
    s.raider_next(r);
    assert!(matches!(s.mobs[&r].state, MobState::Chase { foe: Foe::Obj(o) } if o == wall), "{:?}", s.mobs[&r].state);
    let before = s.mobs[&r].health;
    run(&mut s, 2.1);
    assert!(s.mobs[&r].health < before, "the spikes bite");
    let _ = fire;
}


/// Engage a creature on a hero and step until it winds up; the events on the way.
fn until_windup(s: &mut Survival, k: Id, p: Id) -> Vec<Event> {
    s.engage(k, Foe::Hero(p));
    let mut ev = Vec::new();
    for _ in 0..200 {
        ev.extend(s.step(DT));
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            break;
        }
    }
    ev
}

#[test]
fn a_horde_grunt_strings_its_blows_and_each_can_be_met() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = s.content.mob("grunt").unwrap();
    // Only the two cuts.
    let two = s.content.mobs[g].patterns.iter().position(|p| p.name == "two cuts").unwrap();
    for (k, pat) in s.content.mobs[g].patterns.iter_mut().enumerate() {
        pat.weight = if k == two { 1.0 } else { 0.0 };
    }
    s.content.mobs[g].block = 0.0;
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.players.get_mut(&p).unwrap().health = 10_000;
    s.players.get_mut(&p).unwrap().max_health = 10_000;
    let mut ev = until_windup(&mut s, k, p);
    assert_eq!(s.mobs[&k].pattern, Some((two, 0)));
    ev.extend(run(&mut s, 2.5));
    let hits = hits_on(&ev, p);
    assert_eq!(hits.iter().filter(|(_, o)| *o == Outcome::Hit).count(), 2, "both cuts land: {hits:?}");
    // Each with its own take of the swing, for the clients that play clips.
    let takes: Vec<u32> = ev.iter().filter_map(|e| match e { Event::MobWindup { id, take, .. } if *id == k => Some(*take), _ => None }).collect();
    assert_eq!(takes[..2], [0, 1]);
}

#[test]
fn a_horde_fighter_lunges_in_from_farther_off() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = s.content.mob("grunt").unwrap();
    let k = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    let _ = g;
    // Ready, six yards off: the charge is the only pattern that reaches.
    s.mobs.get_mut(&k).unwrap().ready_at = 0.0;
    let start = s.mobs[&k].pos;
    let mut charged = false;
    s.engage(k, Foe::Hero(p));
    for _ in 0..40 {
        let _ = s.step(DT);
        if let Some((pat, _)) = s.mobs[&k].pattern {
            charged = s.content.mobs[g].patterns[pat].name == "charge";
            break;
        }
    }
    assert!(charged, "it charges");
    run(&mut s, 0.8);
    assert!(dist2d(s.mobs[&k].pos, start) > 3.0, "and closes the gap with the blow: {:?}", s.mobs[&k].pos);
}

#[test]
fn a_shield_bearer_takes_light_blows_on_its_shield_but_not_a_kick() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.mob("deathguard").unwrap();
    s.content.mobs[d].block = 1.0;
    let k = mob(&mut s, "deathguard", [2.0, 0.0, 0.0]);
    // Facing the hero, between blows.
    s.mobs.get_mut(&k).unwrap().facing = std::f32::consts::PI;
    // Between its blows (and not moving off).
    s.mobs.get_mut(&k).unwrap().state = MobState::Recover { foe: Foe::Hero(p), until: 1e9 };
    s.mobs.get_mut(&k).unwrap().ready_at = 1e9;
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(hits_on(&ev, k).iter().all(|(_, o)| *o == Outcome::Blocked), "{:?}", hits_on(&ev, k));
    assert!(!hits_on(&ev, k).is_empty());
    s.input(p, Input::Kick);
    let ev = run(&mut s, 0.8);
    assert!(hits_on(&ev, k).iter().any(|(_, o)| *o == Outcome::Hit), "the kick gets through: {:?}", hits_on(&ev, k));
}

#[test]
fn the_journal_moves_on_as_the_camp_does_each_goal_and_pays_into_the_chest() {
    let mut s = world();
    let heart = s.content.world.heart;
    let p = hero(&mut s, "Ayla", [heart[0] + 1.0, heart[1], 0.0], 0.0);
    assert_eq!(s.goal().unwrap().title, "Timber for the camp");
    give(&mut s, p, "wood", 10);
    s.deposit_all(p);
    let ev = run(&mut s, 0.1);
    assert_eq!(s.camp.goal, 1);
    assert!(ev.iter().any(|e| matches!(e, Event::Goal)));
    let roast = s.content.item("roast").unwrap();
    assert_eq!(s.storage.get(&roast), Some(&2), "the reward");
    assert_eq!(s.goal().unwrap().title, "A fire to cook on");
    // Kept across a save.
    let save = s.save(&Default::default());
    assert_eq!(save.goal, 1);
    assert_eq!(save.counts.get("bank:wood"), Some(&10));
    // Armor worn counts once seen.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let i = s.content.goals.iter().position(|g| g.kind == "wear").unwrap();
    s.camp.goal = i;
    let jerkin = s.content.item("hide_jerkin").unwrap();
    s.players.get_mut(&p).unwrap().armor.insert("chest".into(), jerkin);
    run(&mut s, 0.1);
    assert_eq!(s.camp.goal, i + 1);
}


#[test]
fn only_two_press_a_hero_at_once_and_the_rest_circle_facing_them() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().health = 100_000;
    s.players.get_mut(&p).unwrap().max_health = 100_000;
    let ks: Vec<Id> = (0..4).map(|k| {
        let a = k as f32 * 1.4;
        mob(&mut s, "berserker", [a.cos() * 3.0, a.sin() * 3.0, 0.0])
    }).collect();
    for k in &ks {
        s.engage(*k, Foe::Hero(p));
    }
    let mut most = 0;
    let mut circled = false;
    for _ in 0..200 {
        let ev = s.step(DT);
        let attacking = ks.iter().filter(|k| matches!(s.mobs[k].state, MobState::Windup { .. }) || (matches!(s.mobs[k].state, MobState::Recover { .. }) && s.mobs[k].pattern.is_some())).count();
        most = most.max(attacking);
        circled |= ev.iter().any(|e| matches!(e, Event::MobMove { face: Some(f), .. } if *f == p));
    }
    assert!(most <= 2, "never more than two at once: {most}");
    assert!(circled, "the others circle, facing the hero");
}

#[test]
fn a_berserker_may_step_back_out_of_a_heavy_attack() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = s.content.mob("berserker").unwrap();
    s.content.mobs[b].evade = 1.0;
    let k = mob(&mut s, "berserker", [2.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    s.mobs.get_mut(&k).unwrap().ready_at = 1e9;
    s.input(p, Input::Heavy);
    let ev = s.step(DT);
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim: 30, .. } if *id == k)), "the dodge clip");
    let mut far = 0.0f32;
    for _ in 0..10 {
        let _ = s.step(DT);
        far = far.max(s.mobs[&k].pos[0]);
    }
    assert!(far > 4.5, "stepped back: {far}");
}

#[test]
fn the_warlord_roars_at_half_health_and_strikes_quicker_after() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // (Not his posture here: that's its own test.)
    let wd = s.content.mob("warlord").unwrap();
    s.content.mobs[wd].posture = 0;
    let w = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    s.mobs.get_mut(&w).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let before = s.attack_now(w).windup;
    let max = s.mobs[&w].max_health;
    s.hurt_mob(w, p, max / 2 - 10);
    assert!(!s.mobs[&w].enraged, "not yet");
    s.hurt_mob(w, p, 20);
    let m = &s.mobs[&w];
    assert!(m.enraged);
    let d = &s.content.mobs[m.def];
    let (k, step) = m.pattern.expect("in the war cry");
    assert_eq!((d.patterns[k].name.as_str(), step), ("war cry", 0));
    assert!(matches!(m.state, MobState::Windup { .. }), "{:?}", m.state);
    // The war cry itself is its own; after it, the base blow is quicker.
    s.mobs.get_mut(&w).unwrap().pattern = None;
    assert!(s.attack_now(w).windup < before);
}

#[test]
fn the_warlords_second_phase_blows_wait_for_his_second_wind() {
    let mut s = world();
    let _p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [2.5, 0.0, 0.0]);
    let names = |s: &mut Survival| -> Vec<String> {
        let picks: Vec<usize> = (0..300).filter_map(|_| s.pick_pattern(w, 2.5, 1.0)).collect();
        let def = s.mobs[&w].def;
        picks.into_iter().map(|k| s.content.mobs[def].patterns[k].name.clone()).collect()
    };
    let early = names(&mut s);
    assert!(!early.is_empty());
    assert!(!early.iter().any(|n| n == "whirlwind" || n == "fury" || n == "war cry"), "{early:?}");
    let max = s.mobs[&w].max_health;
    s.mobs.get_mut(&w).unwrap().health = max / 3;
    let late = names(&mut s);
    assert!(late.iter().any(|n| n == "whirlwind" || n == "fury"), "{late:?}");
    assert!(!late.iter().any(|n| n == "war cry"), "the war cry is the enrage's alone");
}

#[test]
fn the_first_warlord_killed_leaves_his_axe_and_helm_in_the_chest() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let axe = s.content.item("warlord_axe").unwrap();
    let helm = s.content.item("warlord_helm").unwrap();
    let w = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    let max = s.mobs[&w].max_health;
    s.hurt_mob(w, p, max);
    let ev = s.step(0.0);
    assert_eq!(s.storage.get(&axe), Some(&1));
    assert_eq!(s.storage.get(&helm), Some(&1));
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("Falls"))));
    let w2 = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    s.hurt_mob(w2, p, max);
    assert_eq!(s.storage.get(&axe), Some(&1), "the first only");
}

#[test]
fn the_last_raid_brings_one_champion_however_often_it_comes() {
    let mut s = world();
    // The warband once, its Warlord one.
    s.camp.raids_won = 2;
    let plan = s.next_raid();
    assert_eq!(plan.title, "Horde Warband");
    assert_eq!(plan.raiders.iter().find(|r| r.0 == "warlord").map(|r| r.1), Some(1));
    // The last (the Blackrock) again and again: Rak'gash one, the rest growing.
    let last = s.content.raid.count() as u32 - 1;
    s.camp.raids_won = last + 3;
    let plan = s.next_raid();
    assert_eq!(plan.raiders.iter().find(|r| r.0 == "rakgash").map(|r| r.1), Some(1));
    assert!(plan.raiders.iter().any(|r| r.0 == "blackrock_grunt" && r.1 > 5), "the rest grow");
}

#[test]
fn the_warlord_comes_up_the_road_on_his_own_after_the_rest() {
    let mut s = world();
    let _p = hero(&mut s, "Ayla", [400.0, 400.0, 0.0], 0.0);
    let last = s.content.raid.count() as u32 - 1;
    s.camp.raids_won = last;
    s.raid.won_before = last;
    let grunt = s.content.mob("grunt").unwrap();
    let warlord = s.content.mob("warlord").unwrap();
    s.raid.phase = RaidPhase::Active;
    s.raid.queue = vec![grunt, warlord];
    s.raid.timer = 0.0;
    let ev = s.step(DT);
    assert_eq!(s.raid.raiders.len(), 1, "the grunt first, the Warlord after");
    assert!(!ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Warlord Gorzak")));
    s.raid.timer = 0.0;
    let ev = s.step(DT);
    assert_eq!(s.raid.raiders.len(), 2);
    assert!(ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title == "Warlord Gorzak")), "{ev:?}");
}

#[test]
fn a_crowded_shaman_throws_you_down_with_thunder_then_backs_off() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "shaman", [2.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let _ = s.step(DT);
    let m = &s.mobs[&k];
    let (pat, _) = m.pattern.expect("a pattern");
    assert_eq!(s.content.mobs[m.def].patterns[pat].name, "thunder");
    assert!(!s.attack_now(k).ranged, "a blast all round, not a bolt");
    // It lands: the hero is on the ground.
    let ev = run(&mut s, 0.8);
    assert!(hits_on(&ev, p).iter().any(|(d, _)| *d > 0), "{:?}", hits_on(&ev, p));
    assert_eq!(s.players[&p].action.name(), "stagger");
    // Then, between casts, it backs off facing its foe.
    run(&mut s, 2.4);
    let far = dist2d(s.mobs[&k].pos, s.players[&p].pos);
    assert!(far > 4.5, "backed off: {far}");
}

#[test]
fn a_witch_doctor_far_off_casts_its_bolt_or_a_volley() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "witchdoctor", [12.0, 0.0, 0.0]);
    let mut seen = std::collections::BTreeSet::new();
    for _ in 0..40 {
        s.mobs.get_mut(&k).unwrap().pattern = None;
        let size = 0.4;
        if let Some(i) = s.pick_pattern(k, 12.0, size) {
            seen.insert(s.content.mobs[s.mobs[&k].def].patterns[i].name.clone());
        }
    }
    assert!(seen.contains("bolt") && seen.contains("volley") && !seen.contains("frost burst"), "{seen:?}");
    let _ = p;
}

#[test]
fn a_patrol_walks_its_route_together_and_back() {
    let mut s = world();
    let _p = hero(&mut s, "Ayla", [500.0, 500.0, 0.0], 0.0);
    s.content.packs.clear();
    s.content.packs.push(crate::content::PackDef { mob: "grunt".into(), at: [0.0, 0.0], radius: 2.0, count: 2, link: 10.0, route: vec![[20.0, 0.0]], wait: 1.0, camp: String::new(),
        z: None,
        key: String::new(),
        elite: false,
        hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
    });
    s.place_packs();
    let ids: Vec<Id> = s.mobs.keys().copied().collect();
    assert_eq!(ids.len(), 2);
    run(&mut s, 0.5);
    for id in &ids {
        assert!(s.mobs[id].home[0] > 17.0, "bound for the far point: {:?}", s.mobs[id].home);
    }
    run(&mut s, 12.0);
    for id in &ids {
        assert!(s.mobs[id].pos[0] > 15.0, "there: {:?}", s.mobs[id].pos);
    }
    // Then back to where it started.
    run(&mut s, 14.0);
    for id in &ids {
        assert!(s.mobs[id].pos[0] < 5.0, "back: {:?}", s.mobs[id].pos);
    }
}

#[test]
fn a_long_fight_with_the_warlord_runs_through_his_patterns_and_never_sticks() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    s.engage(w, Foe::Hero(p));
    let def = s.mobs[&w].def;
    let mut used = std::collections::BTreeSet::new();
    let mut stuck = 0.0f32;
    let mut last_state = String::new();
    for i in 0..(120.0 / DT) as usize {
        // The hero keeps at it, and never runs out.
        if let Some(pl) = s.players.get_mut(&p) {
            pl.stamina = 100.0;
            pl.health = pl.max_health;
        }
        if i % 20 == 0 {
            let at = s.mobs.get(&w).map(|m| m.pos);
            if let Some(at) = at {
                let pos = s.players[&p].pos;
                s.move_player(p, pos, facing(pos, at), Moving::default());
            }
            s.input(p, Input::Attack);
        }
        let _ = s.step(DT);
        let Some(m) = s.mobs.get(&w) else { break };
        if m.health == 0 {
            break;
        }
        if let Some((k, _)) = m.pattern {
            used.insert(s.content.mobs[def].patterns[k].name.clone());
        }
        let st = format!("{:?}", std::mem::discriminant(&m.state));
        if st == last_state && !matches!(m.state, MobState::Chase { .. }) {
            stuck += DT;
        } else {
            stuck = 0.0;
        }
        last_state = st;
        assert!(stuck < 6.0, "stuck in {:?}", m.state);
        // Fight him down to the second phase partway through.
        if i == (40.0 / DT) as usize {
            let max = s.mobs[&w].max_health;
            s.mobs.get_mut(&w).unwrap().health = max / 2 + 5;
        }
    }
    assert!(used.len() >= 5, "{used:?}");
    assert!(used.contains("war cry"), "{used:?}");
    assert!(used.iter().any(|n| n == "whirlwind" || n == "fury"), "{used:?}");
}

#[test]
fn a_watchtower_shoots_the_nearest_raider_in_reach() {
    let mut s = world();
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    let tower = put_up(&mut s, p, "watchtower");
    let tpos = s.objs[&tower].pos;
    // Out of the way.
    s.move_player(p, [tpos[0] + 80.0, tpos[1], 0.0], f, Moving::default());
    let grunt = s.content.mob("grunt").unwrap();
    let near = s.spawn_mob(grunt, [tpos[0] + 10.0, tpos[1], 0.0], None, true);
    let far = s.spawn_mob(grunt, [tpos[0] + 40.0, tpos[1], 0.0], None, true);
    for r in [near, far] {
        s.mobs.get_mut(&r).unwrap().state = MobState::Idle { wander_at: 1e9 };
        s.raid.raiders.push(r);
    }
    s.raid.phase = RaidPhase::Active;
    s.raid.timer = 100.0;
    let (h_near, h_far) = (s.mobs[&near].health, s.mobs[&far].health);
    let ev = run(&mut s, 0.1);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == tower)), "a shot");
    assert!(s.mobs[&near].health < h_near);
    assert_eq!(s.mobs[&far].health, h_far, "out of reach");
    // Not again until it's ready.
    let before = s.mobs[&near].health;
    run(&mut s, 1.0);
    assert!(s.mobs[&near].health >= before.saturating_sub(6), "no second shot yet (only the spikes, if any)");
}

#[test]
fn healing_in_front_of_a_foe_draws_its_attack() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    give(&mut s, p, "roast", 1);
    let k = mob(&mut s, "grunt", [5.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 5.0;
    s.input(p, Input::Use);
    assert_eq!(s.players[&p].action.name(), "eat");
    assert!(s.mobs[&k].ready_at <= s.clock + 0.3, "it comes at once");
}

#[test]
fn a_blow_from_behind_is_a_backstab() {
    let mut s = world();
    // The hero behind a boar that faces away.
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "boar", [2.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().facing = 0.0;
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.6);
    let hits = hits_on(&ev, k);
    assert_eq!(hits.len(), 1);
    let w = s.weapon_of(&s.players[&p]).damage;
    assert!(hits[0].0 as f32 >= w[0] as f32 * 1.4, "harder from behind: {hits:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text == "Backstab!")));
}

#[test]
fn a_spearmans_thrust_reaches_far_but_narrow_and_a_crowded_archer_kicks() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // A thrust from four yards off reaches; the same from a stride to the side doesn't.
    let k = mob(&mut s, "spearman", [4.2, 0.0, 0.0]);
    let def = s.mobs[&k].def;
    let thrust = s.content.mobs[def].patterns.iter().position(|p| p.name == "thrust").unwrap();
    s.mobs.get_mut(&k).unwrap().facing = std::f32::consts::PI;
    s.mobs.get_mut(&k).unwrap().pattern = Some((thrust, 0));
    let a = s.attack_now(k);
    assert!(a.range >= 4.5 && a.arc <= 45.0, "{a:?}");
    // An archer up close kicks rather than shoots.
    let mut s = world();
    let p2 = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "archer", [2.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p2) };
    let _ = s.step(DT);
    let m = &s.mobs[&k];
    let (i, _) = m.pattern.expect("a pattern");
    assert_eq!(s.content.mobs[m.def].patterns[i].name, "kick");
    // From afar, an arrow (a cast, then a missile to where the hero stood).
    let mut s = world();
    let p3 = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "archer", [14.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p3) };
    let ev = run(&mut s, 1.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: 75, .. } if *caster == k)), "it draws");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 75, .. } if *caster == k)), "and looses");
    let _ = p;
}

#[test]
fn a_caster_that_cannot_see_its_foe_comes_round() {
    let land = FlatLand { walls: vec![([6.0, 0.0], 0.8)], ..Default::default() };
    let mut s = world_with(land);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "archer", [12.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let ev = run(&mut s, 0.6);
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == k)), "no shot through the wall");
    assert!(dist2d(s.mobs[&k].pos, ORIGIN) < 10.0, "it moves: {:?}", s.mobs[&k].pos);
}

#[test]
fn the_shadow_hunter_hexes_from_afar_and_thrusts_up_close_and_the_captain_leaves_his_shield() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "shadowhunter", [12.0, 0.0, 0.0]);
    let def = s.mobs[&k].def;
    let names = |s: &mut Survival, d: f32| -> Vec<String> {
        let picks: Vec<usize> = (0..200).filter_map(|_| s.pick_pattern(k, d, 0.4)).collect();
        picks.into_iter().map(|i| s.content.mobs[def].patterns[i].name.clone()).collect()
    };
    let far = names(&mut s, 12.0);
    assert!(far.iter().any(|n| n == "hex") && !far.iter().any(|n| n == "sweep"), "{far:?}");
    let near = names(&mut s, 2.0);
    assert!(near.iter().any(|n| n == "sweep") && !near.iter().any(|n| n == "hex"), "{near:?}");
    // A hex is a cast and a missile.
    s.mobs.get_mut(&k).unwrap().pattern = Some((s.content.mobs[def].patterns.iter().position(|p| p.name == "hex").unwrap(), 0));
    assert!(s.attack_now(k).ranged);
    // The Captain's trophy.
    let c = mob(&mut s, "captain", [3.0, 0.0, 0.0]);
    let max = s.mobs[&c].max_health;
    s.hurt_mob(c, p, max);
    let shield = s.content.item("captain_shield").unwrap();
    assert_eq!(s.storage.get(&shield), Some(&1));
}

#[test]
fn the_champions_trophies_are_not_on_the_rack() {
    let mut s = world();
    let rack = s.content.world.rack;
    let p = hero(&mut s, "Ayla", [rack[0] + 1.0, rack[1], 0.0], 0.0);
    let keys: Vec<String> = s.rack_view(p).into_iter().map(|r| r.0).collect();
    assert!(keys.contains(&"maul".to_string()));
    for t in ["warlord_axe", "zanjin_spear", "captain_shield"] {
        assert!(!keys.contains(&t.to_string()), "{t} on the rack");
        assert!(s.arm(p, t).is_err());
    }
}

#[test]
fn enough_damage_quickly_breaks_a_champions_stance() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    s.engage(w, Foe::Hero(p));
    let limit = s.content.mobs[s.mobs[&w].def].posture;
    s.hurt_mob(w, p, limit / 2);
    assert!(s.mobs[&w].stagger_until <= s.clock, "not yet");
    // Too slowly, and it starts over.
    run(&mut s, 7.0);
    s.hurt_mob(w, p, limit / 2 + 1);
    assert!(s.mobs[&w].stagger_until <= s.clock, "it had its breath back");
    s.hurt_mob(w, p, limit / 2);
    let ev = s.step(0.0);
    assert!(s.mobs[&w].stagger_until > s.clock + 1.5, "broken");
    assert!(s.mobs[&w].riposte, "open to a riposte");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { text, .. } if text == "Stance broken!")));
}

#[test]
fn a_creatures_wind_up_marks_its_blow_with_its_true_reach_arc_and_timing() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.0, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    let mut tele = None;
    for _ in 0..80 {
        let ev = s.step(DT);
        if let Some(t) = ev.iter().find_map(|e| match e {
            Event::Mark { tele: Some(t), .. } => Some(*t),
            _ => None,
        }) {
            tele = Some(t);
            break;
        }
    }
    let t = tele.expect("a telegraphed blow");
    let a = s.attack_now(k);
    assert_eq!(t.mob, k);
    assert_eq!(t.foe, Some(p));
    assert!((t.reach - (a.range + 0.4)).abs() < 1e-4, "the reach a blow lands at");
    assert_eq!(t.arc, a.arc);
    assert!((t.secs - a.windup).abs() < 1e-4);
    assert_eq!(t.blockable, a.blockable);
}

#[test]
fn a_blow_no_guard_stops_warns_its_foe_from_the_creature_winding_it_up() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = s.content.mob("bear").unwrap();
    s.content.mobs[b].patterns.retain(|p| p.name == "maul");
    let k = mob(&mut s, "bear", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    let mut warned = false;
    for _ in 0..40 {
        let ev = s.step(DT);
        warned |= ev.iter().any(|e| matches!(e, Event::Danger { player, secs, from } if *player == p && *secs > 1.0 && *from == Some(k)));
        if matches!(s.mobs[&k].state, MobState::Windup { .. }) {
            break;
        }
    }
    assert!(warned);
}

#[test]
fn a_heavy_blow_that_brings_on_the_second_wind_mid_wind_up_keeps_the_war_cry() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    take_up(&mut s, p, "maul");
    let wd = s.content.mob("warlord").unwrap();
    s.content.mobs[wd].posture = 0;
    // His plain cleave, so the wind-up is one the maul reaches.
    s.content.mobs[wd].patterns.retain(|p| p.name == "sweeping cleave" || p.name == "war cry");
    s.content.mobs[wd].evade = 0.0;
    // A slow one, so the maul lands first.
    for pat in s.content.mobs[wd].patterns.iter_mut().filter(|p| p.name == "sweeping cleave") {
        pat.steps[0].windup = Some(3.0);
    }
    let w = mob(&mut s, "warlord", [2.6, 0.0, 0.0]);
    s.mobs.get_mut(&w).unwrap().facing = std::f32::consts::PI;
    let max = s.mobs[&w].max_health;
    s.mobs.get_mut(&w).unwrap().health = max / 2 + 2;
    s.engage(w, Foe::Hero(p));
    for _ in 0..60 {
        let _ = s.step(DT);
        if matches!(s.mobs[&w].state, MobState::Windup { .. }) {
            break;
        }
    }
    assert!(matches!(s.mobs[&w].state, MobState::Windup { .. }));
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Heavy);
    for _ in 0..30 {
        let _ = s.step(DT);
        if s.mobs[&w].enraged {
            break;
        }
    }
    let m = &s.mobs[&w];
    assert!(m.enraged);
    let (k, _) = m.pattern.expect("still in a pattern");
    assert_eq!(s.content.mobs[m.def].patterns[k].name, "war cry");
}

#[test]
fn a_second_wind_due_while_reeling_comes_once_it_stands() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [3.0, 0.0, 0.0]);
    s.engage(w, Foe::Hero(p));
    // Its stance broken by the blow that takes it under half.
    let max = s.mobs[&w].max_health;
    s.hurt_mob(w, p, max / 2 + 10);
    assert!(s.mobs[&w].stagger_until > s.clock + 2.0, "reeling, the whole break");
    assert!(!s.mobs[&w].enraged, "not while it reels");
    run(&mut s, 2.4);
    assert!(s.mobs[&w].enraged, "then the war cry");
}

#[test]
fn a_patrol_caught_far_from_its_next_point_fights_rather_than_running_on() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", [500.0, 500.0, 0.0], 0.0);
    s.content.packs.clear();
    s.content.packs.push(crate::content::PackDef { mob: "grunt".into(), at: [0.0, 0.0], radius: 0.0, count: 1, link: 0.0, route: vec![[80.0, 0.0]], wait: 1.0, camp: String::new(),
        z: None,
        key: String::new(),
        elite: false,
        hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
    });
    s.place_packs();
    let k = *s.mobs.keys().next().unwrap();
    run(&mut s, 2.0);
    assert!(s.mobs[&k].home[0] > 70.0, "bound for the far point");
    // Caught a few yards along: it fights.
    let at = s.mobs[&k].pos;
    s.move_player(p, [at[0] + 3.0, at[1], 0.0], std::f32::consts::PI, Moving::default());
    let ev = run(&mut s, 7.0);
    assert!(hits_on(&ev, p).iter().any(|(d, _)| *d > 0), "it fights: {:?}", s.mobs[&k].state);
}

#[test]
fn killing_every_fighter_of_a_horde_camp_takes_it_for_a_while() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", [300.0, 300.0, 0.0], 0.0);
    s.content.packs.clear();
    for at in [[0.0, 0.0], [10.0, 0.0]] {
        s.content.packs.push(crate::content::PackDef { mob: "grunt".into(), at, radius: 2.0, count: 2, link: 0.0, route: Vec::new(), wait: 8.0, camp: "mine".into(),
            z: None,
            key: String::new(),
            elite: false,
            hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
        });
    }
    s.place_packs();
    let ids: Vec<Id> = s.mobs.keys().copied().collect();
    assert_eq!(ids.len(), 4);
    let tin = s.content.item("tin").unwrap();
    let before = s.storage.get(&tin).copied().unwrap_or(0);
    for (i, id) in ids.iter().enumerate() {
        let max = s.mobs[id].max_health;
        s.hurt_mob(*id, p, max);
        let ev = s.step(0.0);
        let taken = ev.iter().any(|e| matches!(e, Event::Banner { title, .. } if title.contains("Mine Yard")));
        assert_eq!(taken, i == 3, "taken with the last of them only");
    }
    assert!(s.storage.get(&tin).copied().unwrap_or(0) > before, "the first time pays");
    let hold = s.content.camps.iter().find(|c| c.key == "mine").unwrap().hold;
    assert!(s.respawns.iter().all(|r| r.3 >= s.clock + hold - 1.0), "none back meanwhile");
    assert_eq!(s.camp.counts.get("clear:mine"), Some(&1));
}

#[test]
fn a_save_keeps_its_journal_goal_by_title_and_an_older_one_is_read_through_the_first_journal() {
    let mut s = world();
    let overseer = s.content.goals.iter().position(|g| g.title == "The Overseer").unwrap();
    s.camp.goal = overseer;
    let mut save = s.save(&BTreeMap::new());
    assert_eq!(save.goal_title.as_deref(), Some("The Overseer"));
    let back = Survival::load(Content::dev(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    assert_eq!(back.camp.goal, overseer);
    // An older save: only the place, 12 in the first journal.
    save.goal_title = None;
    save.goal = 12;
    let back = Survival::load(Content::dev(), Box::new(FlatLand::default()), 1, 1_000_000.0, &save);
    assert_eq!(back.content.goals[back.camp.goal].title, "The Overseer");
}

#[test]
fn a_camp_isnt_taken_again_by_killing_the_first_of_them_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", [300.0, 300.0, 0.0], 0.0);
    s.content.packs.clear();
    for at in [[0.0, 0.0], [60.0, 0.0]] {
        s.content.packs.push(crate::content::PackDef { mob: "grunt".into(), at, radius: 0.0, count: 1, link: 0.0, route: Vec::new(), wait: 8.0, camp: "mine".into(),
            z: None,
            key: String::new(),
            elite: false,
            hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
        });
    }
    s.place_packs();
    let ids: Vec<Id> = s.mobs.keys().copied().collect();
    for id in &ids {
        let max = s.mobs[id].max_health;
        s.hurt_mob(*id, p, max);
    }
    assert_eq!(s.camp.counts.get("clear:mine"), Some(&1));
    // The hold over, one is back and the other is held off (a hero by its spot): killing the one
    // back doesn't take it again.
    let _ = s.step(0.0);
    let hold = s.content.camps[0].hold;
    s.move_player(p, [0.0, 3.0, 0.0], 0.0, Moving::default());
    run(&mut s, hold + 6.0);
    let back: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.health > 0).map(|(id, _)| *id).collect();
    assert_eq!(back.len(), 1, "the far one only");
    let max = s.mobs[&back[0]].max_health;
    s.hurt_mob(back[0], p, max);
    assert_eq!(s.camp.counts.get("clear:mine"), Some(&1));
}

#[test]
fn the_shadow_hunter_hexes_from_afar_while_two_others_are_at_the_hero() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Two grunts at the hero, held in their wind-ups.
    for y in [-1.5, 1.5] {
        let g = mob(&mut s, "grunt", [2.0, y, 0.0]);
        s.mobs.get_mut(&g).unwrap().state = MobState::Windup { foe: Foe::Hero(p), t: -100.0, mark: None, shown: true };
    }
    let k = mob(&mut s, "shadowhunter", [15.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let mut cast = false;
    for _ in 0..60 {
        let ev = s.step(DT);
        cast |= ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == k));
        if cast {
            break;
        }
    }
    assert!(cast, "a hex");
}

#[test]
fn a_ring_no_guard_stops_warns_everyone_it_would_reach() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [4.0, 2.0, 0.0], 0.0);
    let far = hero(&mut s, "Cass", [40.0, 0.0, 0.0], 0.0);
    let wd = s.content.mob("witchdoctor").unwrap();
    s.content.mobs[wd].patterns.retain(|p| p.name == "frost burst");
    let k = mob(&mut s, "witchdoctor", [2.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(a) };
    let ev = s.step(DT);
    let warned: Vec<Id> = ev.iter().filter_map(|e| if let Event::Danger { player, .. } = e { Some(*player) } else { None }).collect();
    assert!(warned.contains(&a) && warned.contains(&b) && !warned.contains(&far), "{warned:?}");
    assert_eq!(warned.iter().filter(|w| **w == a).count(), 1, "once each");
}

#[test]
fn a_creature_killed_mid_step_stops_where_it_falls() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "wolf", [3.0, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().path = vec![[10.0, 0.0, 0.0]];
    s.mobs.get_mut(&k).unwrap().speed = 2.5;
    let max = s.mobs[&k].max_health;
    s.hurt_mob(k, p, max);
    let ev = s.step(0.0);
    assert!(ev.iter().any(|e| matches!(e, Event::MobStop { id, .. } if *id == k)), "its walk stopped");
    let at = s.mobs[&k].pos;
    run(&mut s, 1.0);
    assert_eq!(s.mobs[&k].pos, at);
}

#[test]
fn walking_with_the_bow_drawn_lets_the_legs_walk_and_standing_shows_the_draw_again() {
    let mut s = world();
    let p = hero(&mut s, "Bren", ORIGIN, 0.0);
    let _ = s.slot_ability(p, "ranged", "aimed_shot");
    s.input(p, SHOT);
    s.input(p, Input::Attack);
    let ev = run(&mut s, 0.3);
    assert!(ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == p)), "the draw shown");
    // Walking forward, still drawing: the drawn pose's cast is withdrawn.
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == p)));
    assert_eq!(s.players[&p].action.name(), "charge", "still drawing");
    // Stopped: shown again.
    let at = s.players[&p].pos;
    s.move_player(p, at, 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(ev.iter().any(|e| matches!(e, Event::Cast { caster, .. } if *caster == p)));
}

#[test]
fn a_lock_on_a_foe_that_dies_moves_to_the_next_and_its_bar_goes() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "boar", [3.0, 0.0, 0.0]);
    let b = mob(&mut s, "boar", [6.0, 1.0, 0.0]);
    s.input(p, Input::Lock);
    assert_eq!(s.players[&p].lock, Some(a));
    s.players.get_mut(&p).unwrap().foe = Some((a, s.clock));
    let max = s.mobs[&a].max_health;
    s.hurt_mob(a, p, max);
    assert_eq!(s.players[&p].lock, Some(b), "on to the next");
    assert!(s.foe_view(p).is_some(), "the empty bar a moment");
    run(&mut s, 1.2);
    assert!(s.foe_view(p).is_none(), "then gone");
}

#[test]
fn a_lock_lets_go_when_its_foe_goes_home_or_far_or_the_hero_falls() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "boar", [4.0, 0.0, 0.0]);
    s.input(p, Input::Lock);
    assert_eq!(s.players[&p].lock, Some(k));
    // Gone home (leashed): let go.
    s.mobs.get_mut(&k).unwrap().state = MobState::Return;
    let ev = s.step(DT);
    assert_eq!(s.players[&p].lock, None);
    assert!(ev.iter().any(|e| matches!(e, Event::Lock { player, target: None } if *player == p)));
    // Far off: let go.
    s.mobs.get_mut(&k).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.input(p, Input::Lock);
    assert_eq!(s.players[&p].lock, Some(k));
    s.mobs.get_mut(&k).unwrap().pos = [60.0, 0.0, 0.0];
    let _ = s.step(DT);
    assert_eq!(s.players[&p].lock, None);
}

#[test]
fn building_making_and_mending_take_their_time_and_are_paid_as_they_finish() {
    let mut s = world();
    s.content.work = crate::content::WorkDef { craft: 2.0, gear: 1.5, build: 3.0, repair: 2.5 };
    let (at, f) = build_spot(&s);
    let p = hero(&mut s, "Ayla", at, f);
    give(&mut s, p, "wood", 12);
    give(&mut s, p, "meat", 4);
    let wood = s.content.item("wood").unwrap();
    let stations = |s: &Survival| s.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { .. })).count();
    // Building: it goes up after its time, the wood paid then; the ghost stays where it was put.
    s.build_start(p, "cooking").unwrap();
    let _ = s.step(DT);
    let ghost_at = s.players[&p].preview.as_ref().unwrap().pos;
    let had = s.players[&p].bag[&wood];
    s.build_confirm(p).unwrap();
    assert_eq!(s.players[&p].action.name(), "build");
    s.players.get_mut(&p).unwrap().facing = f + 1.0;
    run(&mut s, 1.5);
    assert_eq!(stations(&s), 0, "not up yet");
    assert_eq!(s.players[&p].bag[&wood], had, "nothing paid yet");
    assert!(dist2d(s.players[&p].preview.as_ref().unwrap().pos, ghost_at) < 0.01, "the ghost stays put");
    run(&mut s, 1.7);
    assert_eq!(stations(&s), 1, "built");
    assert!(s.players[&p].bag.get(&wood).copied().unwrap_or(0) < had, "paid as it's done");
    let fire = *s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).unwrap().0;
    // Making: a roast after two seconds; walking off stops it, and nothing's paid.
    let (roast, meat) = (s.content.item("roast").unwrap(), s.content.item("meat").unwrap());
    let before = s.players[&p].bag.get(&roast).copied().unwrap_or(0);
    s.craft(p, fire, "roast").unwrap();
    run(&mut s, 0.5);
    s.move_player(p, [at[0] + 3.0, at[1], 0.0], f, Moving::default());
    run(&mut s, 2.0);
    assert_eq!(s.players[&p].action, Action::Ready, "stopped");
    assert_eq!((s.players[&p].bag.get(&roast).copied().unwrap_or(0), s.players[&p].bag[&meat]), (before, 4), "nothing made, nothing paid");
    // A blow stops it too.
    s.move_player(p, at, f, Moving::default());
    let _ = s.step(DT);
    s.craft(p, fire, "roast").unwrap();
    run(&mut s, 0.5);
    let k = mob(&mut s, "grunt", [at[0] + 2.0, at[1], 0.0]);
    s.strike_player(k, p, 5, [at[0] + 2.0, at[1], 0.0], false, false, false, Blow::Light);
    s.mobs.remove(&k);
    assert_eq!(s.players[&p].action, Action::Ready, "interrupted");
    // Left alone, it's made.
    s.craft(p, fire, "roast").unwrap();
    run(&mut s, 1.5);
    assert_eq!(s.players[&p].bag.get(&roast).copied().unwrap_or(0), before, "still making");
    run(&mut s, 0.7);
    assert_eq!(s.players[&p].bag[&roast], before + 1, "made");
    // Mending a wrecked one takes its time as well.
    s.hurt_station(fire, 100_000);
    give(&mut s, p, "wood", 10);
    s.interact(p, Some(fire));
    assert_eq!(s.players[&p].action.name(), "repair");
    run(&mut s, 2.0);
    assert!(matches!(s.objs[&fire].kind, ObjKind::Station { damaged: true, .. }), "still wrecked");
    run(&mut s, 0.7);
    assert!(matches!(s.objs[&fire].kind, ObjKind::Station { damaged: false, .. }), "mended");
}

#[test]
fn the_building_ground_is_wide_ringed_by_torches_and_says_which_way_it_is() {
    let s = world();
    let w = s.content.world.clone();
    let c = [w.build_center[0], w.build_center[1], 0.0];
    let torches: Vec<Vec3> = s.objs.values().filter(|o| matches!(o.kind, ObjKind::Prop { display, .. } if display == w.build_marker)).map(|o| o.pos).collect();
    assert!(torches.len() >= 15, "a ring of torches: {}", torches.len());
    assert!(torches.iter().all(|t| (dist2d(*t, c) - (w.build_radius + 1.0)).abs() < 0.5), "round its edge");
    let fire = s.content.station("cooking").unwrap();
    // Far out on it: fine.
    let near_edge = [c[0] - (w.build_radius - 4.0), c[1], 0.0];
    assert_eq!(s.site_problem(fire, near_edge, None), None);
    // Past its north edge: which way it is, and how far.
    let out = [c[0] + w.build_radius + 20.0, c[1], 0.0];
    let why = s.site_problem(fire, out, None).unwrap();
    assert!(why.contains("ringed by torches") && why.contains("south of here") && why.contains("yd"), "{why}");
}

// --- attacks out of a dodge and out of a Corey roll ----------------------------------------------

/// The kind and clip of the hero's swing now, if they're swinging.
fn swing_of(s: &Survival, p: Id) -> Option<(StrikeKind, u16, f32)> {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => Some((strike.kind, strike.clip, strike.lunge)),
        _ => None,
    }
}

#[test]
fn an_attack_pressed_in_a_dodge_comes_as_the_roll_ends() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let set = s.content.moveset_of(s.weapon_of(&s.players[&p])).clone();
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    run(&mut s, 0.1);
    // Early in the roll (not only in its last moment, as a buffered press).
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    assert_eq!(s.players[&p].action.name(), "dodge", "the roll goes on");
    let secs = s.content.dodge.secs;
    run(&mut s, secs - 0.05);
    let (kind, clip, lunge) = swing_of(&s, p).expect("the attack out of the dodge");
    assert_eq!(kind, StrikeKind::Roll);
    assert_eq!((clip, lunge), (set.roll_move().clip, set.roll_move().lunge));
    // A dodge with no attack in it ends in nothing.
    run(&mut s, 3.0);
    s.input(p, Input::Dodge);
    run(&mut s, secs + 0.05);
    assert_eq!(s.players[&p].action, Action::Ready, "no attack nobody pressed");
}

#[test]
fn a_jump_in_a_dodges_dash_is_a_corey_roll_and_its_attack_strikes_on_landing() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let set = s.content.moveset_of(s.weapon_of(&s.players[&p])).clone();
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    run(&mut s, 0.2);
    s.jumped(p);
    assert!(s.in_corey(p), "a jump 0.2 s into the dash");
    // The roll ends in the air; an attack pressed now waits for the ground.
    let secs = s.content.dodge.secs;
    run(&mut s, secs);
    assert_eq!(s.players[&p].action, Action::Ready);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run(&mut s, 0.2);
    assert!(swing_of(&s, p).is_none(), "nothing in the air");
    s.landed(p);
    let (kind, clip, lunge) = swing_of(&s, p).expect("the leap strikes as the hero lands");
    assert_eq!(kind, StrikeKind::Leap);
    assert_eq!((clip, lunge), (set.leap_move().clip, set.leap_move().lunge));
    assert!(lunge > set.roll_move().lunge, "carrying the leap's speed into it");
    assert!(!s.in_corey(p), "on the ground again");
}

#[test]
fn an_attack_pressed_in_a_corey_rolls_dodge_strikes_on_landing_too() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    run(&mut s, 0.2);
    s.jumped(p);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    let secs = s.content.dodge.secs;
    run(&mut s, secs);
    assert!(swing_of(&s, p).is_none(), "the roll ended in the air: it waits");
    s.landed(p);
    assert_eq!(swing_of(&s, p).map(|w| w.0), Some(StrikeKind::Leap));
}

#[test]
fn a_jump_after_the_dash_is_only_a_jump() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    let dash = s.content.dodge.dash_secs;
    run(&mut s, dash + 0.1);
    s.jumped(p);
    assert!(!s.in_corey(p), "too late to carry the dash");
    // Nor is a jump standing about.
    run(&mut s, 1.0);
    s.landed(p);
    s.jumped(p);
    assert!(!s.in_corey(p));
    // An attack in an ordinary jump swings at once, as it always has.
    s.input(p, Input::Attack);
    assert_eq!(swing_of(&s, p).map(|w| w.0), Some(StrikeKind::Light));
}

#[test]
fn corey_rolls_chain_one_into_the_next() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    for hop in 0..3 {
        s.input(p, Input::Dodge);
        assert_eq!(s.players[&p].action.name(), "dodge", "hop {hop}: the roll starts");
        run(&mut s, 0.2);
        s.jumped(p);
        assert!(s.in_corey(p), "hop {hop}: a Corey roll");
        // The leap is longer than the roll: it lands after the roll is over, free to roll again.
        run(&mut s, 0.5);
        s.landed(p);
        assert_eq!(s.players[&p].action, Action::Ready, "hop {hop}: landed free");
    }
    // Each one costs a roll's and a jump's stamina.
    let per = s.content.dodge.stamina + s.content.player.jump_stamina;
    assert!(s.players[&p].stamina < s.content.player.stamina - 2.0 * per, "{}", s.players[&p].stamina);
}

#[test]
fn a_landing_that_never_comes_puts_the_hero_back_on_the_ground() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    run(&mut s, 0.2);
    s.jumped(p);
    run(&mut s, 3.5);
    assert!(s.airborne(p).is_none(), "no landing heard: on the ground after a while");
    s.input(p, Input::Attack);
    assert_eq!(swing_of(&s, p).map(|w| w.0), Some(StrikeKind::Light), "and an attack swings at once");
}

#[test]
fn a_leap_whose_landing_went_unheard_takes_its_attack_with_it() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.move_player(p, ORIGIN, 0.0, Moving { forward: true, ..Default::default() });
    s.input(p, Input::Dodge);
    run(&mut s, 0.2);
    s.jumped(p);
    let secs = s.content.dodge.secs;
    run(&mut s, secs);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    assert_eq!(s.players[&p].landing_attack, Some(StrikeKind::Leap));
    // Its landing never comes; a while later the hero steps off a ledge and lands (no jump).
    run(&mut s, 4.0);
    s.landed(p);
    assert!(swing_of(&s, p).is_none(), "no leap out of nowhere");
    assert_eq!(s.players[&p].landing_attack, None);
    // And a fall in the air takes it too.
    s.input(p, Input::Dodge);
    run(&mut s, 0.2);
    s.jumped(p);
    s.input(p, Input::Attack);
    let hp = s.players[&p].health;
    s.hurt_player(p, hp);
    assert_eq!(s.players[&p].landing_attack, None, "fallen");
}
