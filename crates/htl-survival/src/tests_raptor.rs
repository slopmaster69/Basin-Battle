//! Raptor Strike's end (#110): the Uppercut throws the foe up and it comes down, the fourth press
//! is the Stoop with no click, landing first ends the chain, and the Stoop hits harder from higher.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
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

/// A creature standing still, facing the hero, too tough to fall.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.health = 9999;
    m.max_health = 9999;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run on with this creature held where it stands, idle.
fn run_pinned(s: &mut Survival, secs: f32, k: Id) -> Vec<Event> {
    let at = s.mobs[&k].pos;
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        if let Some(x) = s.mobs.get_mut(&k) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = at;
        }
        out.extend(s.step(DT));
    }
    out
}

fn close_key() -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == "close").unwrap() as u8
}

/// The close key pressed (no slotting, no cooldowns touched).
fn press_e(s: &mut Survival, p: Id) -> Vec<Event> {
    s.input(p, Input::Ability(close_key(), false));
    s.step(DT)
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
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

/// A hunter and a grunt in front of her, through the thrust and the sweep to the Uppercut: the
/// events of the Uppercut's press.
fn to_the_uppercut() -> (Survival, Id, Id, Vec<Event>) {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.5, 0.0, 0.0]);
    s.slot_ability(p, "close", "raptor_strike").unwrap();
    s.set_cooldowns(p, true);
    press_e(&mut s, p);
    run_pinned(&mut s, 0.4, k);
    press_e(&mut s, p);
    run_pinned(&mut s, 0.4, k);
    let ev = press_e(&mut s, p);
    assert_eq!(hits(&ev, p, k).len(), 1, "the Uppercut lands");
    (s, p, k, ev)
}

fn stooping(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Dash { ability, .. } if ability == idx(s, "stoop"))
}

#[test]
fn the_uppercut_throws_the_foe_up_and_it_comes_down_unable_to_act() {
    let (mut s, _p, k, ev) = to_the_uppercut();
    // Everyone sees it go up: its way is an arc through the air, down on the ground at its end.
    let path = ev
        .iter()
        .find_map(|e| match e {
            Event::MobMove { id, path, .. } if *id == k => Some(path.clone()),
            _ => None,
        })
        .expect("the foe thrown");
    let top = path.iter().map(|p| p[2]).fold(0.0f32, f32::max);
    assert!(top > 2.0, "up into the air: {path:?}");
    assert!(path.last().unwrap()[2].abs() < 0.05 && path[0][2].abs() < 0.05, "from the ground, back to it: {path:?}");
    // And on the server, over about a second: up, then down, doing nothing the while.
    let mut high = 0.0f32;
    let mut up_for = 0.0;
    for _ in 0..30 {
        let ev = s.step(DT);
        let z = s.mobs[&k].pos[2];
        high = high.max(z);
        if z > 0.05 {
            up_for += DT;
            assert!(s.disabled(k) && s.mobs[&k].stagger_until > s.clock, "stunned and reeling in the air");
            assert!(!matches!(s.mobs[&k].state, MobState::Windup { .. }), "no wind-up in the air");
            assert!(!ev.iter().any(|e| matches!(e, Event::Swing { attacker, .. } if *attacker == k)), "no blow from the air");
        }
    }
    assert!(high > 2.0, "it went up: {high}");
    assert!((0.7..=1.2).contains(&up_for), "about a second up: {up_for}");
    assert!(s.mobs[&k].pos[2].abs() < 0.05, "and came down: {:?}", s.mobs[&k].pos);
}

#[test]
fn stoop_is_armed_by_the_uppercut_and_the_fourth_press_dives_with_no_click() {
    let (mut s, p, k, ev) = to_the_uppercut();
    // The hunter goes up high with it.
    assert!(ev.iter().any(|e| matches!(e, Event::Hop { player, lift, .. } if *player == p && *lift >= 12.0)), "a high rise");
    // Stoop is on the key already.
    let stoop = idx(&s, "stoop");
    assert!(s.recast_keys(p).iter().any(|(n, a, _)| *n == close_key() as usize && *a == stoop), "Stoop armed on E");
    let mut ev = run(&mut s, 0.3);
    ev.extend(press_e(&mut s, p));
    assert!(stooping(&s, p), "the press dives at once: {:?}", s.players[&p].action);
    // Driven down, fast, onto the foe it launched.
    let dive = ev.iter().find_map(|e| match e {
        Event::Hop { player, lift, speed, .. } if *player == p && *lift < 0.0 => Some((*lift, *speed)),
        _ => None,
    });
    assert!(dive.is_some(), "a dive down: {:?}", ev.iter().filter(|e| matches!(e, Event::Hop { .. })).collect::<Vec<_>>());
    let ev = run(&mut s, 0.35);
    assert!(!stooping(&s, p), "down within 0.35 sec");
    assert_eq!(hits(&ev, p, k).len(), 1, "the slam lands on it");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == KIT_SLAM)), "its impact");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), style: Blow::Heavy, .. } if *attacker == p && *v == k)), "landing as heavily as a heavy attack");
    // Beside it, not inside its body (the visual check showed the hunter standing in the dummy).
    let gap = dist2d(s.players[&p].pos, s.mobs[&k].pos);
    assert!(gap > 0.5 && gap < 1.5, "at the foe's side: {gap:.2} {:?} {:?}", s.players[&p].pos, s.mobs[&k].pos);
}

#[test]
fn landing_before_the_stoop_ends_the_chain() {
    // Down early (on a ledge, say): its window had over a second left.
    let (mut s, p, _k, _) = to_the_uppercut();
    let stoop = idx(&s, "stoop");
    run(&mut s, 0.2);
    s.end_dash(p);
    let mut ev = s.step(DT);
    assert!(!s.recast_keys(p).iter().any(|(_, a, _)| *a == stoop), "no Stoop on the key once down");
    ev.extend(press_e(&mut s, p));
    ev.extend(run(&mut s, 0.4));
    assert!(!stooping(&s, p), "no Stoop from the ground");
    assert!(!ev.iter().any(|e| matches!(e, Event::Log(l) if l.contains("casts Stoop"))), "no Stoop at all");
    // And down in its own time.
    let (mut s, p, _k, _) = to_the_uppercut();
    run(&mut s, 1.6);
    assert!(!matches!(s.players[&p].action, Action::Dash { .. }), "landed");
    let mut ev = press_e(&mut s, p);
    ev.extend(run(&mut s, 0.4));
    assert!(!stooping(&s, p) && !ev.iter().any(|e| matches!(e, Event::Log(l) if l.contains("casts Stoop"))), "no Stoop after landing");
}

#[test]
fn the_stoop_hits_harder_the_higher_it_starts() {
    // At once, barely off the ground.
    let (mut s, p, k, _) = to_the_uppercut();
    let low_from = s.leap_height(p);
    press_e(&mut s, p);
    let low = hits(&run(&mut s, 0.4), p, k);
    // At the top of the rise.
    let (mut s, q, m, _) = to_the_uppercut();
    let mut last = s.leap_height(q);
    loop {
        s.step(DT);
        let h = s.leap_height(q);
        if h <= last {
            break;
        }
        last = h;
    }
    let high_from = s.leap_height(q);
    assert!(high_from > 4.5, "the rise peaks over 4.5 yd: {high_from}");
    press_e(&mut s, q);
    let high = hits(&run(&mut s, 0.4), q, m);
    assert_eq!((low.len(), high.len()), (1, 1), "each lands once: {low:?} {high:?}");
    assert!(low_from < 1.5, "the low one starts low: {low_from}");
    assert!(high[0] >= 25 && low[0] <= 15, "6 to 30 by height: {} from {low_from:.1} yd, {} from {high_from:.1} yd", low[0], high[0]);
    assert!(high[0] >= low[0] * 2, "the higher, the harder: {low:?} {high:?}");
}
