//! The Priest in a duel: every ability that helps friends and hurts foes treats the duel's foe as
//! a foe (struck, never healed, hastened, shielded or rewarded), with friendly fire off, and a
//! friend standing by is still a friend.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const EAST: f32 = 0.0;
const WEST: f32 = std::f32::consts::PI;
/// The Priest's eye above her feet (the reticle's line starts there).
const EYE: f32 = 1.9;

fn world() -> Survival {
    let mut c = Content::dev();
    // Co-op rules: only the duel makes a foe of a hero.
    c.pvp = Default::default();
    c.dev.full_tree = true;
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, f: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, f, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

/// A hero holding `staff` (from the rack), back where they stood, the reticle level.
fn armed(s: &mut Survival, name: &str, pos: Vec3, facing: f32, staff: &str) -> Id {
    let p = hero(s, name, pos, facing);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, staff).unwrap();
    s.players.get_mut(&p).unwrap().pos = pos;
    s.set_aim_line(p, facing, 0.0);
    p
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run on with these heroes held where they stand (a stand-in doesn't walk off).
fn run_held(s: &mut Survival, secs: f32, held: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
        for (h, at) in held {
            if let Some(p) = s.players.get_mut(h) {
                p.pos = *at;
            }
        }
    }
    out
}

fn until(s: &mut Survival, t: f32) -> Vec<Event> {
    let mut out = Vec::new();
    while s.clock + DT * 0.5 < t {
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

fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// Its key pressed (an aimed one let go on a tap of the click).
fn cast(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    learn(s, p, key);
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(key_of(&slot), ex));
    let mut ev = s.step(0.0);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(0.0));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(0.0));
    }
    ev
}

fn press(s: &mut Survival, p: Id, slot: &str) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), false));
    s.step(0.0)
}

fn aim_ground(s: &mut Survival, p: Id, at: Vec3) {
    let from = s.players[&p].pos;
    let d = dist2d(from, at).max(0.5);
    s.set_aim_line(p, facing(from, at), -(EYE + from[2] - at[2]).atan2(d));
}

fn halve(s: &mut Survival, p: Id) -> u32 {
    let now = s.clock;
    let h = s.players.get_mut(&p).unwrap();
    h.health = h.max_health / 2;
    h.combat_at = now;
    h.health
}

fn health(s: &Survival, p: Id) -> u32 {
    s.players[&p].health
}

fn hasted(s: &Survival, p: Id) -> bool {
    s.cc_of(p).iter().any(|c| c.kind == "haste" && c.until > s.clock)
}

fn perfects(ev: &[Event], p: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Perfect { player, .. } if *player == p)).count()
}

/// Ayla (a Priest at the origin, looking east) and Bren 8 yd ahead of her, in a duel under way.
fn duel(s: &mut Survival) -> (Id, Id) {
    let p = hero(s, "Ayla", ORIGIN, EAST);
    s.set_aim_line(p, EAST, 0.0);
    let b = hero(s, "Bren", [8.0, 0.0, 0.0], WEST);
    s.duel_challenge(p, b).unwrap();
    s.duel_accept(b);
    run(s, 3.05);
    assert!(s.duel_foes(p, b), "the duel is on");
    (p, b)
}

// --- Light ------------------------------------------------------------------------------------------

#[test]
fn penance_rings_a_duel_foe_as_a_foe_and_its_bolts_hurt_him() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    let hb = halve(&mut s, b);
    cast(&mut s, p, "penance", false);
    let rings: Vec<(f32, Id, bool)> = s.light.penances.iter().find(|x| x.owner == p).map(|x| x.rings.iter().map(|r| (r.at, r.on, r.friend)).collect()).unwrap_or_default();
    assert!(!rings.is_empty() && rings.iter().all(|r| r.1 == b && !r.2), "every ring on Bren, as a foe: {rings:?}");
    for r in &rings {
        until(&mut s, r.0);
        press(&mut s, p, "ranged");
    }
    run(&mut s, 0.5);
    assert!(health(&s, b) < hb, "struck, not mended: {} from {hb}", health(&s, b));
}

#[test]
fn holy_nova_strikes_a_duel_foe_and_mends_a_friend_beside_him() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    s.move_player(b, [4.0, 0.0, 0.0], WEST, Moving::default());
    let c = hero(&mut s, "Cara", [0.0, 4.0, 0.0], EAST);
    let (hb, hc) = (halve(&mut s, b), halve(&mut s, c));
    cast(&mut s, p, "holy_nova", false);
    assert!(health(&s, b) < hb, "Bren struck: {} from {hb}", health(&s, b));
    assert!(health(&s, c) > hc, "Cara mended");
}

#[test]
fn a_duel_foe_never_takes_the_priests_feather_nor_her_body_and_soul() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    // The feather 8 yd out, under Bren's feet.
    s.set_aim_line(p, EAST, -(EYE).atan2(8.0));
    cast(&mut s, p, "angelic_feather", false);
    let at = s.light.feathers.last().expect("laid").at;
    s.move_player(b, at, WEST, Moving::default());
    run(&mut s, 0.3);
    assert!(!hasted(&s, b), "Bren on it: not for him");
    assert_eq!(s.light.feathers.len(), 1, "still lying");
    // Body and Soul round her: Bren 5 yd off isn't quickened.
    s.move_player(b, [5.0, 0.0, 0.0], WEST, Moving::default());
    run(&mut s, 0.05);
    cast(&mut s, p, "angelic_feather", true);
    assert!(hasted(&s, p), "Ayla quickened");
    assert!(!hasted(&s, b), "Bren not");
}

#[test]
fn power_word_shield_never_lands_on_a_duel_foe() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    cast(&mut s, p, "power_word_shield", false);
    assert!(!s.has_cc(b, "shield"), "not on Bren under the reticle");
}

#[test]
fn power_word_barrier_holds_off_nothing_for_a_duel_foe_under_it() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    s.move_player(b, [3.0, 0.0, 0.0], WEST, Moving::default());
    aim_ground(&mut s, p, ORIGIN);
    cast(&mut s, p, "power_word_shield", true);
    run_held(&mut s, 0.5, &[(b, [3.0, 0.0, 0.0])]);
    assert!(s.has_cc(p, "steadfast"), "Ayla under her dome");
    assert!(!s.has_cc(b, "steadfast") && !s.has_cc(b, "hallowed"), "Bren gets nothing of it");
}

#[test]
fn a_duel_foe_in_salvations_circle_lands_none_of_its_beats() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    s.move_player(b, [4.0, 0.0, 0.0], WEST, Moving::default());
    let c = hero(&mut s, "Cara", [-3.0, 2.0, 0.0], EAST);
    halve(&mut s, b);
    s.set_valor(p, 100.0);
    let mut ev = cast(&mut s, p, "holy_word_salvation", false);
    let beats = s.light.sigils.last().expect("the circle laid").beats.clone();
    for at in &beats {
        ev.extend(until(&mut s, *at));
        for h in [b, c] {
            s.input(h, Input::Guard);
            s.input(h, Input::GuardUp);
        }
    }
    let hb = health(&s, b);
    ev.extend(run(&mut s, 0.6));
    assert_eq!(s.holy_stacks(b), 0, "no Holy Light for Bren");
    assert_eq!(perfects(&ev, b), 0, "no perfect");
    assert_eq!(s.holy_stacks(c), 3, "Cara, a friend standing by, lands all three");
    assert!(health(&s, b) < hb, "the pillar strikes him, never mends him");
}

// --- Shadow -----------------------------------------------------------------------------------------

#[test]
fn mind_blast_erupts_on_a_duel_foe() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    let hb = health(&s, b);
    aim_ground(&mut s, p, [8.0, 0.0, 0.0]);
    cast(&mut s, p, "mind_blast", false);
    run_held(&mut s, 1.2, &[(b, [8.0, 0.0, 0.0])]);
    assert!(health(&s, b) + 26 <= hb, "26 or more: {} from {hb}", health(&s, b));
}

#[test]
fn psychic_scream_panics_a_duel_foe_for_15_sec() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    s.move_player(b, [4.0, 0.0, 0.0], WEST, Moving::default());
    run(&mut s, 0.05);
    cast(&mut s, p, "psychic_scream", false);
    let left = s.cc_of(b).iter().find(|c| c.kind == "panic" && c.until > s.clock).map_or(0.0, |c| c.until - s.clock);
    assert!(left > 1.3 && left <= 1.5 + 1.0e-3, "a hero flees 1.5 sec: {left}");
}

#[test]
fn void_rift_drags_a_duel_foe_and_leaves_a_friend_be() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    let c = hero(&mut s, "Cara", [12.0, -3.0, 0.0], EAST);
    s.move_player(b, [12.0, 3.0, 0.0], WEST, Moving::default());
    let (hb, hc) = (health(&s, b), health(&s, c));
    aim_ground(&mut s, p, [12.0, 0.0, 0.0]);
    cast(&mut s, p, "void_rift", false);
    assert_eq!(s.light.rifts.len(), 1, "torn open");
    run(&mut s, 1.1);
    assert!(health(&s, b) < hb, "it hurts Bren");
    assert_eq!(health(&s, c), hc, "never Cara");
    press(&mut s, p, "ultimate");
    run(&mut s, 0.1);
    assert!(health(&s, b) + 40 < hb, "the implosion on him");
    assert_eq!(health(&s, c), hc);
}

#[test]
fn shadow_word_death_on_a_beaten_duel_foe_wins_the_duel_and_is_no_kill() {
    let mut s = world();
    let (p, b) = duel(&mut s);
    s.players.get_mut(&b).unwrap().health = 10;
    // Shadow Word: Death (Mind Blast's EX) along the reticle onto him.
    let mut ev = cast(&mut s, p, "mind_blast", true);
    ev.extend(run_held(&mut s, 0.6, &[(b, [8.0, 0.0, 0.0])]));
    assert_eq!(health(&s, b), 1, "left standing at 1");
    assert!(ev.iter().any(|e| matches!(e, Event::Duel { news: duel::DuelNews::Ended { winner: Some(w), .. }, .. } if *w == p)), "Ayla won");
    assert!(s.duel_of(p).is_none());
    assert!(!ev.iter().any(|e| matches!(e, Event::Log(l) if l.contains("free"))), "no free Death: nobody fell");
}

// --- the staffs ---------------------------------------------------------------------------------------

#[test]
fn healing_gust_passes_a_duel_foe_by() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "acolyte_staff");
    let b = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    s.duel_challenge(p, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.05);
    let hb = halve(&mut s, b);
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    s.step(0.0);
    run_held(&mut s, 1.5, &[(b, [8.0, 0.0, 0.0])]);
    assert_eq!(health(&s, b), hb, "not mended");
}

#[test]
fn the_hollow_star_hurts_a_duel_foe_and_heals_a_friend() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let b = hero(&mut s, "Bren", [8.0, 0.0, 0.0], WEST);
    s.duel_challenge(p, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.05);
    s.input(p, Input::Guard);
    s.step(0.0);
    s.input(p, Input::GuardUp);
    let h = s.light.hollows.iter().find(|h| h.owner == p).expect("the orb out").clone();
    let at = s.mobs[&h.mob].pos;
    let bat = [at[0], at[1] + 2.0, 0.0];
    s.move_player(b, bat, WEST, Moving::default());
    let c = hero(&mut s, "Cara", [at[0], at[1] - 2.0, 0.0], WEST);
    let (hb, hc) = (halve(&mut s, b), halve(&mut s, c));
    run_held(&mut s, 1.05, &[(b, bat)]);
    assert_eq!(hb - health(&s, b), 5, "5 to Bren");
    assert_eq!(health(&s, c), hc + 5, "5 to Cara");
}

#[test]
fn the_void_tether_on_a_duel_foe_burns_him() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "hollow_star_staff");
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    s.duel_challenge(p, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.05);
    let hb = halve(&mut s, b);
    s.input(p, Input::Attack);
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Channel { target, .. } if target == b), "bound to Bren: {:?}", s.players[&p].action);
    run_held(&mut s, 1.0, &[(b, [10.0, 0.0, 0.0])]);
    assert!(health(&s, b) < hb, "burned, not mended: {} from {hb}", health(&s, b));
}

#[test]
fn the_shadow_staffs_bolts_weave_into_a_duel_foe() {
    let mut s = world();
    let p = armed(&mut s, "Ayla", ORIGIN, EAST, "shadowcaller_staff");
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], WEST);
    s.duel_challenge(p, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.05);
    let hb = health(&s, b);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    run_held(&mut s, 1.0, &[(b, [10.0, 0.0, 0.0])]);
    assert!(health(&s, b) < hb, "the bolt lands on him");
}

#[test]
fn a_priest_standing_by_cant_mend_a_duellist() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [8.0, 0.0, 0.0], WEST);
    let b = hero(&mut s, "Bren", [4.0, 0.0, 0.0], WEST);
    s.duel_challenge(a, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.05);
    let p = hero(&mut s, "Cara", ORIGIN, EAST);
    let hb = halve(&mut s, b);
    cast(&mut s, p, "holy_nova", false);
    assert_eq!(health(&s, b), hb, "not Bren: the duel is his and Ayla's");
}
