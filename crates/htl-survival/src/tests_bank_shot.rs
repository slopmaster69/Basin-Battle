//! Bank Shot and Split Arrow (#110): an arrow seen the whole way, every ricochet flown on from
//! where it struck (no burst there: a bounce isn't an explosion, Corey's review), and the arrow
//! planted where it ends; Split Arrow's long flight and its burst.

use crate::content::Content;
use crate::land::{FlatLand, Land};
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
/// Aimed Shot's arrow, Bank Shot's look; Split Arrow's burst where it splits; its glowing arrow.
const ARROW: u32 = 19434;
const FLASH: u32 = 5162;
const GLOW: u32 = 3044;

/// Level ground with walls: everything at or past `east` is solid, and with `west`, everything at
/// or short of it too (a corridor).
struct Walls {
    east: f32,
    west: Option<f32>,
}

impl Land for Walls {
    fn height(&self, _x: f32, _y: f32) -> f32 {
        0.0
    }
    fn blocked(&self, x: f32, _y: f32) -> bool {
        x >= self.east || self.west.is_some_and(|w| x <= w)
    }
}

fn world(land: Box<dyn Land>) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, land, 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, pos: Vec3) -> Id {
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A grunt standing still that can take a lot.
fn mob(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    m.health = 9999;
    id
}

/// Run on, these creatures held where they stand.
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

/// Bank Shot (its EX with `ex`) on the Q key, aimed along (facing, elevation) and loosed.
fn shoot(s: &mut Survival, p: Id, ex: bool, aim: (f32, f32)) -> Vec<Event> {
    s.slot_ability(p, "ranged", "bank_shot").unwrap();
    s.set_cooldowns(p, true);
    s.set_valor(p, 100.0);
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::Ability(0, ex));
    let mut ev = s.step(DT);
    s.set_aim_line(p, aim.0, aim.1);
    s.input(p, Input::Attack);
    ev.extend(s.step(DT));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, anim: false, .. } if *attacker == p && *v == m && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

/// The legs flown from a spot (off a wall, out of a burst): (anchor, from, to).
fn legs(ev: &[Event], p: Id, spell: u32) -> Vec<(Id, Vec3, Vec3)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::MissileFrom { anchor, caster, spell: s, from, target: Target::Point(to) } if *caster == p && *s == spell => Some((*anchor, *from, *to)),
            _ => None,
        })
        .collect()
}

/// The looks shown at a spot by a leg's anchor: (anchor, where).
fn flashes(ev: &[Event], spell: u32) -> Vec<(Id, Vec3)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Missile { caster, spell: s, target: Target::Point(at) } if *s == spell => Some((*caster, *at)),
            _ => None,
        })
        .collect()
}

#[test]
fn bank_shot_is_seen_off_the_wall_and_plants_where_it_ends() {
    let mut s = world(Box::new(Walls { east: 10.0, west: None }));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, false, (std::f32::consts::FRAC_PI_4, 0.0));
    ev.extend(run_pinned(&mut s, 2.5, &[]));
    // The arrow from the hands to the wall.
    let first: Vec<Vec3> = ev
        .iter()
        .filter_map(|e| match e {
            Event::Missile { caster, spell, target: Target::Point(to) } if *caster == p && *spell == ARROW => Some(*to),
            _ => None,
        })
        .collect();
    assert_eq!(first.len(), 1, "one arrow loosed");
    assert!((first[0][0] - 10.0).abs() < 0.8, "flown to the wall: {:?}", first[0]);
    // Off the wall: a leg flown from where it struck, back away from the wall and on up the
    // field, and nothing bursting there (Corey's review: a burst read as the arrow going off in
    // the inn wall; a bounce shows no burst).
    let l = legs(&ev, p, ARROW);
    assert_eq!(l.len(), 1, "one ricochet: {l:?}");
    let (anchor, from, to) = l[0];
    assert!(from[0] < 10.0 && from[0] > 8.5, "from the wall: {from:?}");
    assert!(to[0] < from[0] - 5.0 && to[1] > from[1] + 5.0, "off it, mirrored: {from:?} to {to:?}");
    assert!(flashes(&ev, FLASH).is_empty(), "no burst where it struck: {:?}", flashes(&ev, FLASH));
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == anchor)), "the leg's anchor shows nothing but the arrow");
    // Where it ends: planted at its range's end, out past the wall's corner of the field.
    let planted = s.planted_of(p);
    assert_eq!(planted.len(), 1, "planted once: {planted:?}");
    assert!(planted[0][0] < 9.0 && planted[0][1] > 15.0, "where its flight ended: {planted:?}");
    // And its anchor gone once it's done.
    assert!(ev.iter().any(|e| matches!(e, Event::ObjGone { id } if *id == anchor)), "the anchor goes");
}

#[test]
fn bank_shot_banks_twice_down_a_corridor_then_plants_in_the_third_wall() {
    let mut s = world(Box::new(Walls { east: 4.0, west: Some(-4.0) }));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, false, (std::f32::consts::FRAC_PI_4, 0.0));
    ev.extend(run_pinned(&mut s, 2.5, &[]));
    let l = legs(&ev, p, ARROW);
    assert_eq!(l.len(), 2, "two ricochets, no more: {l:?}");
    assert!(l[0].1[0] > 3.0 && l[0].2[0] < -3.0, "east wall, then across: {:?}", l[0]);
    assert!(l[1].1[0] < -3.0 && l[1].2[0] > 3.0, "west wall, then back across: {:?}", l[1]);
    assert!(l[1].1[1] > l[0].1[1] + 4.0, "each further up the corridor");
    assert!(flashes(&ev, FLASH).is_empty(), "no burst at either: {:?}", flashes(&ev, FLASH));
    // Out of bounces, it stops in the third wall and plants under it.
    let planted = s.planted_of(p);
    assert_eq!(planted.len(), 1, "{planted:?}");
    assert!(planted[0][0] > 2.5 && planted[0][0] < 4.0, "at the east wall: {planted:?}");
    assert!((planted[0][1] - l[1].2[1]).abs() < 1.5, "where the last leg was flown to: {planted:?} {:?}", l[1]);
}

#[test]
fn bank_shot_aimed_low_skips_off_the_ground_into_the_foe() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let k = mob(&mut s, [25.0, 0.0, 0.0]);
    // The reticle on the ground 19 yd out: a shallow arrow down into it.
    let mut ev = shoot(&mut s, p, false, (0.0, -(1.9f32).atan2(19.0)));
    ev.extend(run_pinned(&mut s, 1.5, &[k]));
    let l = legs(&ev, p, ARROW);
    assert_eq!(l.len(), 1, "it skips: {l:?}");
    let (_, from, to) = l[0];
    assert!(from[2] < 0.6 && from[0] > 15.0, "off the ground where it came down: {from:?}");
    assert!(to[2] > from[2], "and up from it: {from:?} to {to:?}");
    assert!(flashes(&ev, FLASH).is_empty(), "no burst where it skipped");
    // Into the foe past it, no harder (a skip isn't a ricochet), and planted past it.
    let h = hits(&ev, p, k);
    assert!(h.len() == 1 && h[0] < 24, "it still strikes, no harder: {h:?}");
    let planted = s.planted_of(p);
    assert_eq!(planted.len(), 1);
    assert!(planted[0][0] > 24.0, "past the foe: {planted:?}");
}

#[test]
fn bank_shot_aimed_steeply_down_plants_where_it_lands() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, false, (0.0, -(1.9f32).atan2(3.0)));
    ev.extend(run_pinned(&mut s, 1.0, &[]));
    assert!(legs(&ev, p, ARROW).is_empty(), "no skip");
    let planted = s.planted_of(p);
    assert_eq!(planted.len(), 1);
    assert!(planted[0][0] < 5.0, "in the ground where it came down: {planted:?}");
}

#[test]
fn split_arrow_flies_long_then_bursts_into_a_spray() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    // One far off, another behind it.
    let a = mob(&mut s, [30.0, 0.0, 0.0]);
    let b = mob(&mut s, [36.0, 0.0, 0.0]);
    let mut ev = shoot(&mut s, p, true, (0.0, 0.0));
    // One arrow goes, nothing else, for its long flight.
    let loosed = ev.iter().filter(|e| matches!(e, Event::Missile { caster, spell, .. } if *caster == p && *spell == GLOW)).count();
    assert_eq!(loosed, 1, "one arrow");
    let mut burst_after = None;
    for k in 0..40 {
        let step = run_pinned(&mut s, DT, &[a, b]);
        if burst_after.is_none() && !flashes(&step, FLASH).is_empty() {
            burst_after = Some(k as f32 * DT);
        }
        ev.extend(step);
    }
    let after = burst_after.expect("it bursts");
    assert!(after > 0.35, "after a long flight: {after}");
    // At the first foe: struck, and the spray fanned out from there, by unseen anchors.
    let f = flashes(&ev, FLASH);
    assert_eq!(f.len(), 1, "{f:?}");
    assert!((f[0].1[0] - 29.5).abs() < 1.0, "the burst where it struck: {:?}", f[0].1);
    let l = legs(&ev, p, GLOW);
    assert_eq!(l.len(), 6, "a spray of 6: {l:?}");
    assert!(l.iter().all(|(_, from, _)| (from[0] - 29.5).abs() < 1.0), "out of the burst");
    assert!(l.iter().any(|(id, ..)| *id == f[0].0), "the burst shown by one of their anchors");
    let ways: Vec<f32> = l.iter().map(|(_, from, to)| (to[1] - from[1]).atan2(to[0] - from[0])).collect();
    let spread = ways.iter().cloned().fold(f32::MIN, f32::max) - ways.iter().cloned().fold(f32::MAX, f32::min);
    assert!(spread > 0.8, "fanned wide: {ways:?}");
    // The one it burst in takes the arrow and the spray through it; the one behind, some of it.
    let (ha, hb) = (hits(&ev, p, a), hits(&ev, p, b));
    let arrow = ha.iter().copied().max().unwrap_or(0);
    assert!(ha.iter().filter(|d| **d < arrow).count() >= 3, "the arrow and the spray: {ha:?}");
    assert!(!hb.is_empty() && hb.iter().all(|d| *d < arrow), "the spray on through: {hb:?}");
    // Every arrow of the spray planted; the one that burst didn't.
    assert_eq!(s.planted_of(p).len(), 6, "{:?}", s.planted_of(p));
}

#[test]
fn split_arrow_with_nothing_in_the_way_bursts_at_the_end_of_its_flight() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, true, (0.0, 0.0));
    ev.extend(run_pinned(&mut s, 1.5, &[]));
    let f = flashes(&ev, FLASH);
    assert_eq!(f.len(), 1);
    assert!(f[0].1[0] > 34.0, "far out: {:?}", f[0].1);
    assert_eq!(legs(&ev, p, GLOW).len(), 6);
    let planted = s.planted_of(p);
    assert_eq!(planted.len(), 6);
    assert!(planted.iter().all(|a| a[0] > 40.0), "the spray on past the burst: {planted:?}");
}

#[test]
fn split_arrow_against_a_wall_sprays_back_off_it() {
    let mut s = world(Box::new(Walls { east: 15.0, west: None }));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, true, (0.0, 0.0));
    ev.extend(run_pinned(&mut s, 1.5, &[]));
    let l = legs(&ev, p, GLOW);
    assert_eq!(l.len(), 6);
    assert!(l.iter().all(|(_, from, to)| from[0] < 15.0 && to[0] < from[0]), "back off the wall: {l:?}");
    assert!(s.planted_of(p).iter().all(|a| a[0] < 15.0));
}
