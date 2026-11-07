//! Duels: a challenge, the answer, a countdown, then the two heroes are each other's foes alone
//! (struck, never helped) until one would fall (left at 1 health, the duel lost), gives it away
//! (a forfeit, leaving, out of bounds too long) or something else fells one. Nobody else can
//! strike or help a duellist meanwhile, friendly fire or not.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::duel::{DuelNews, DuelPhase};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

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

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn news(ev: &[Event]) -> Vec<DuelNews> {
    ev.iter().filter_map(|e| if let Event::Duel { news, .. } = e { Some(news.clone()) } else { None }).collect()
}

fn ended(ev: &[Event]) -> Option<(bool, Option<Id>, bool)> {
    news(ev).into_iter().find_map(|n| if let DuelNews::Ended { started, winner, fled, .. } = n { Some((started, winner, fled)) } else { None })
}

fn hits_on(events: &[Event], victim: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim && *damage > 0)).count()
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn cast(s: &mut Survival, p: Id, key: &str) -> Vec<Event> {
    let slot = s.content.abilities[s.content.ability(key).unwrap()].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), false));
    let mut ev = s.step(DT);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(DT));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
    }
    ev
}

/// Down to half health, a blow just taken.
fn halve(s: &mut Survival, p: Id) -> u32 {
    let now = s.clock;
    let p = s.players.get_mut(&p).unwrap();
    p.health = p.max_health / 2;
    p.combat_at = now;
    p.health
}

/// Ayla challenges Bren, he takes it, and the countdown runs out: the duel is on.
fn dueling(s: &mut Survival) -> (Id, Id) {
    let a = hero(s, "Ayla", ORIGIN, 0.0);
    let b = hero(s, "Bren", [3.0, 0.0, 0.0], std::f32::consts::PI);
    s.duel_challenge(a, b).unwrap();
    s.duel_accept(b);
    run(s, 3.1);
    assert!(s.duel_foes(a, b), "on");
    (a, b)
}

#[test]
fn a_duel_is_asked_taken_and_counted_down_before_they_are_foes() {
    let mut s = world(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], std::f32::consts::PI);
    assert!(!s.hostile(a, b) && s.helpable(a, b), "friends to start with");
    s.duel_challenge(a, b).unwrap();
    let ev = s.step(DT);
    assert_eq!(news(&ev), vec![DuelNews::Asked]);
    let flag = s.duel_of(a).unwrap().flag;
    assert!(s.objs.contains_key(&flag), "the flag stands between them");
    assert!(!s.hostile(a, b) && s.helpable(a, b), "asked is nothing yet");
    // Only the one challenged can take it.
    s.duel_accept(a);
    assert!(matches!(s.duel_of(a).unwrap().phase, DuelPhase::Asked { .. }));
    s.duel_accept(b);
    let ev = s.step(DT);
    assert_eq!(news(&ev), vec![DuelNews::Counting { secs: 3.0 }]);
    assert!(!s.hostile(a, b) && !s.hostile(b, a), "nobody strikes in the countdown");
    assert!(!s.helpable(a, b), "nor helps the other");
    let ev = run(&mut s, 3.0);
    assert_eq!(news(&ev), vec![DuelNews::Fighting]);
    assert!(s.hostile(a, b) && s.hostile(b, a), "foes");
    assert!(!s.helpable(a, b) && !s.helpable(b, a), "and never helped by each other");
    assert_eq!(s.duel_side(a), Some((flag, 1)));
    assert_eq!(s.duel_side(b), Some((flag, 2)));
}

#[test]
fn a_duel_is_its_two_heroes_alone_friendly_fire_or_not() {
    for pvp in [false, true] {
        let mut s = world(pvp);
        let (a, b) = dueling(&mut s);
        let c = hero(&mut s, "Cass", [0.0, 3.0, 0.0], 0.0);
        let d = hero(&mut s, "Dain", [0.0, -3.0, 0.0], 0.0);
        assert!(!s.hostile(c, a) && !s.hostile(a, c), "pvp {pvp}: a bystander can't strike a duellist, nor be struck");
        assert!(!s.helpable(c, a), "pvp {pvp}: nor help them");
        assert!(s.helpable(a, c), "pvp {pvp}: a duellist can still help a bystander (as WoW has it)");
        assert_eq!(s.hostile(c, d), pvp, "pvp {pvp}: the rest as friendly fire says");
        assert!(s.helpable(c, d));
        let _ = b;
    }
}

#[test]
fn holy_nova_strikes_a_duel_foe_and_never_mends_them() {
    // Friendly fire on: a co-op friend in it is mended; a duel's foe is struck.
    let mut s = world(true);
    let (a, b) = dueling(&mut s);
    let hb = halve(&mut s, b);
    let ev = cast(&mut s, a, "holy_nova");
    assert!(hits_on(&ev, b) >= 1, "struck");
    assert!(s.players[&b].health < hb, "and hurt, not mended");
}

#[test]
fn a_duel_foes_blows_land_with_friendly_fire_off() {
    let mut s = world(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.5, 0.0, 0.0], std::f32::consts::PI);
    s.input(a, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert_eq!(hits_on(&ev, b), 0, "a friend isn't struck");
    s.duel_challenge(a, b).unwrap();
    s.duel_accept(b);
    run(&mut s, 3.1);
    s.input(a, Input::Attack);
    let ev = run(&mut s, 1.0);
    assert!(hits_on(&ev, b) >= 1, "the duel's foe is");
}

#[test]
fn the_last_blow_leaves_the_loser_standing_and_ends_the_duel() {
    let mut s = world(false);
    let (a, b) = dueling(&mut s);
    s.apply_cc(b, a, "slow", 10.0, 0.5, 0.0, "test");
    assert!(s.has_cc(b, "slow"));
    s.players.get_mut(&b).unwrap().health = 5;
    s.hurt_player_by(b, 50, a, "test");
    assert_eq!(s.players[&b].health, 1, "left at 1 health");
    let ev = s.step(DT);
    assert_eq!(ended(&ev), Some((true, Some(a), false)), "Ayla won it, a knockout");
    assert!(s.duel_of(a).is_none() && s.duel_of(b).is_none());
    assert!(!s.has_cc(b, "slow"), "what she put on him is gone");
    assert!(!s.hostile(a, b) && s.helpable(a, b), "friends again");
    assert!(!s.objs.values().any(|o| matches!(o.kind, ObjKind::Prop { display: 787, .. })), "the flag is down");
}

#[test]
fn something_else_felling_a_duellist_ends_the_duel_with_no_winner() {
    let mut s = world(false);
    let (a, b) = dueling(&mut s);
    s.players.get_mut(&b).unwrap().health = 5;
    s.set_cause(b, recap::Cause::world("fall"));
    s.hurt_player(b, 50);
    assert_eq!(s.players[&b].health, 0, "a fall fells him");
    let ev = s.step(DT);
    assert_eq!(ended(&ev), Some((true, None, false)));
    assert!(s.duel_of(a).is_none());
}

#[test]
fn out_of_bounds_too_long_gives_the_duel_away() {
    let mut s = world(false);
    let (a, b) = dueling(&mut s);
    s.move_player(b, [60.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 5.0);
    assert!(news(&ev).contains(&DuelNews::Bounds { who: b, out: true, secs: 10.0 }));
    // Back in time: still on.
    s.move_player(b, [10.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 0.2);
    assert!(news(&ev).contains(&DuelNews::Bounds { who: b, out: false, secs: 0.0 }));
    s.move_player(b, [60.0, 0.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 10.5);
    assert_eq!(ended(&ev), Some((true, Some(a), true)), "fled");
}

#[test]
fn a_forfeit_or_leaving_hands_the_other_the_duel() {
    let mut s = world(false);
    let (a, b) = dueling(&mut s);
    s.duel_cancel(b);
    assert_eq!(ended(&s.step(DT)), Some((true, Some(a), true)));
    let mut s = world(false);
    let (a, b) = dueling(&mut s);
    s.leave(a);
    assert_eq!(ended(&s.step(DT)), Some((true, Some(b), true)));
    assert!(s.duels.is_empty());
}

#[test]
fn a_challenge_declined_or_unanswered_never_starts() {
    let mut s = world(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    s.duel_challenge(a, b).unwrap();
    assert!(s.duel_challenge(b, a).is_err(), "already asked");
    s.duel_cancel(b);
    assert_eq!(ended(&s.step(DT)), Some((false, None, false)), "declined");
    s.duel_challenge(a, b).unwrap();
    let flag = s.duel_of(a).unwrap().flag;
    let ev = run(&mut s, 60.1);
    assert_eq!(ended(&ev), Some((false, None, false)), "lapsed");
    assert!(s.duels.is_empty() && !s.objs.contains_key(&flag));
}

#[test]
fn a_challenge_needs_another_hero_near() {
    let mut s = world(false);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [40.0, 0.0, 0.0], 0.0);
    assert!(s.duel_challenge(a, a).is_err(), "not oneself");
    assert!(s.duel_challenge(a, b).is_err(), "not 40 yd off");
    s.move_player(b, [20.0, 0.0, 0.0], 0.0, Moving::default());
    assert!(s.duel_challenge(a, b).is_ok(), "20 yd is near enough");
}
