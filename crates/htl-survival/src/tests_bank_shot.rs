//! Bank Shot (#110): an arrow seen the whole way, every ricochet flown on from where it struck (no
//! burst there: a bounce isn't an explosion, Corey's review), and the arrow planted where it ends;
//! and its EX, Scatter Shot.

use crate::content::Content;
use crate::land::{FlatLand, Land};
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
/// Aimed Shot's arrow, Bank Shot's look.
const ARROW: u32 = 19434;
/// The burst of fire a bounce has none of.
const FLASH: u32 = 5162;
/// The splash of earth where it skips off the ground (not the wall's fire: Corey's review).
const SPLASH: u32 = 26271;

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
fn bank_shots_arrow_takes_a_hotter_colour_each_bounce_for_its_own_leg() {
    // Corey's review: "every bounce should change the color of the trail". Each leg is flown by its
    // own stand-in, so its colour is named for that one alone, after it's made.
    let mut s = world(Box::new(Walls { east: 4.0, west: Some(-4.0) }));
    let p = hero(&mut s, ORIGIN);
    let mut ev = shoot(&mut s, p, false, (std::f32::consts::FRAC_PI_4, 0.0));
    ev.extend(run_pinned(&mut s, 2.5, &[]));
    let tints: Vec<(Id, u8)> = ev
        .iter()
        .filter_map(|e| match e {
            Event::MissileTint { caster, spell: ARROW, bounce, .. } => Some((*caster, *bounce)),
            _ => None,
        })
        .collect();
    let l = legs(&ev, p, ARROW);
    assert_eq!(tints, vec![(p, 0), (l[0].0, 1), (l[1].0, 2)], "the hunter's arrow, then each leg's own");
    let bank = s.content.ability("bank_shot").unwrap();
    assert_eq!(s.content.abilities[bank].bounce_trail.len(), 3, "a colour for each");
    for (anchor, _, _) in &l {
        let made = ev.iter().position(|e| matches!(e, Event::MissileFrom { anchor: a, .. } if a == anchor)).unwrap();
        let tint = ev.iter().position(|e| matches!(e, Event::MissileTint { caster, .. } if caster == anchor)).unwrap();
        assert!(made < tint, "the stand-in is made before its colour is named");
    }
    // An arrow with no colours of its own (Aimed Shot's) names none.
    s.slot_ability(p, "ranged", "aimed_arrow").unwrap();
    s.input(p, Input::Ability(0, false));
    let mut ev = s.step(DT);
    s.input(p, Input::Attack);
    ev.extend(run_pinned(&mut s, 0.6, &[]));
    s.input(p, Input::AttackUp);
    ev.extend(run_pinned(&mut s, 1.0, &[]));
    assert!(!ev.iter().any(|e| matches!(e, Event::MissileTint { .. })));
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
    // No fire on the ground where it skipped: it isn't an explosion (the arrow's flight on from
    // there is the look; no small splash of dirt exists in the data).
    assert!(flashes(&ev, FLASH).is_empty(), "no fire on the ground: it isn't an explosion");
    assert!(flashes(&ev, SPLASH).is_empty(), "and no boulders either");
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

// --- Scatter Shot ------------------------------------------------------------------------------

/// How long an effect of `kind` has left on `u`.
fn left(s: &Survival, u: Id, kind: &str) -> f32 {
    s.cc_of(u).iter().find(|c| c.kind == kind).map_or(0.0, |c| c.until - s.clock)
}

#[test]
fn scatter_shot_disorients_the_first_foe_it_meets_for_4_sec_and_deals_nothing() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let d = s.content.abilities[s.content.ability("scatter_shot").unwrap()].clone();
    let e = d.effects[0].clone();
    assert_eq!((d.range[1], d.valor, e.kind.as_str(), e.secs[1], e.hero_secs, d.damage[1]), (10.0, 25, "disorient", 4.0, 1.5, 0.0));
    // One in reach, one behind it, one past its 10 yd.
    let a = mob(&mut s, [7.0, 0.0, 0.0]);
    let b = mob(&mut s, [9.0, 0.0, 0.0]);
    let out = mob(&mut s, [14.0, 3.0, 0.0]);
    let before = (s.mobs[&a].health, s.mobs[&b].health, s.mobs[&out].health);
    let mut ev = shoot(&mut s, p, true, (0.0, 0.0));
    ev.extend(run_pinned(&mut s, 0.6, &[a, b, out]));
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 19503, .. } if *caster == p)), "its look flies");
    assert!(s.has_cc(a, "disorient"), "the first foe it meets");
    assert!(left(&s, a, "disorient") > 3.0 && left(&s, a, "disorient") <= 4.0, "4 sec less the flight: {}", left(&s, a, "disorient"));
    assert!(!s.has_cc(b, "disorient") && !s.has_cc(out, "disorient"), "one foe, and nothing past 10 yd");
    assert!(s.auras_of(a).contains(&19503), "it shows");
    assert_eq!((s.mobs[&a].health, s.mobs[&b].health, s.mobs[&out].health), before, "no damage");
    assert!(hits(&ev, p, a).is_empty());
    // 25 Valor paid of the 100, and a little earned back for the shot landing.
    assert!((75.0..90.0).contains(&s.players[&p].valor), "25 Valor: {}", s.players[&p].valor);
}

#[test]
fn a_disoriented_foe_wanders_cannot_act_and_any_damage_ends_it() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let a = mob(&mut s, [6.0, 0.0, 0.0]);
    shoot(&mut s, p, true, (0.0, 0.0));
    run_pinned(&mut s, 0.3, &[]);
    assert!(s.has_cc(a, "disorient"));
    assert!(s.disabled(a), "it can't act");
    assert!(!s.held_still(a), "but it isn't held: it wanders");
    // It wanders, not away from the hero in particular: it ends up somewhere other than where it
    // stood, in a few seconds, and the hero is the one it never goes for.
    let start = s.mobs[&a].pos;
    let mut walked = 0.0f32;
    let mut at = start;
    for _ in 0..30 {
        run_pinned(&mut s, 0.1, &[]);
        walked += dist2d(at, s.mobs[&a].pos);
        at = s.mobs[&a].pos;
    }
    assert!(walked > 1.0, "it wandered: {walked}");
    assert!(!matches!(s.mobs[&a].state, MobState::Windup { .. }), "and it never swings");
    // Any damage ends it (even a scratch), and the foe is its own again.
    assert_eq!(s.cc_on_damage(a, 1), 1);
    assert!(!s.has_cc(a, "disorient"), "damage breaks it");
}

#[test]
fn scatter_shot_on_a_hero_lasts_1_5_sec_and_a_boss_shrugs_it_off() {
    let mut s = world(Box::new(FlatLand::default()));
    let p = hero(&mut s, ORIGIN);
    let b = s.join("bren", "Bren", None);
    s.move_player(b, [6.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let _ = s.step(0.0);
    let d = s.content.abilities[s.content.ability("scatter_shot").unwrap()].clone();
    s.apply_effects(p, b, &d.effects, 1.0, "Scatter Shot");
    assert!((left(&s, b, "disorient") - 1.5).abs() < 0.1, "a hero 1.5 sec: {}", left(&s, b, "disorient"));
    assert!(s.disabled(b), "and can't act");
    // Not again straight after: the grace that follows any hard control.
    run_pinned(&mut s, 1.6, &[]);
    assert!(!s.has_cc(b, "disorient"));
    assert!(!s.apply_cc(b, p, "disorient", 4.0, 0.0, 0.0, "again"), "the grace");
    // A boss holds its ground.
    let def = s.content.mob("warlord").unwrap();
    let boss = s.spawn_mob(def, [8.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&boss).unwrap().state = MobState::Idle { wander_at: 1e9 };
    s.apply_effects(p, boss, &d.effects, 1.0, "Scatter Shot");
    assert!(!s.has_cc(boss, "disorient"), "a boss shrugs it off");
}

#[test]
fn scatter_shots_words_say_what_the_data_does() {
    let s = world(Box::new(FlatLand::default()));
    let d = &s.content.abilities[s.content.ability("scatter_shot").unwrap()];
    let e = &d.effects[0];
    assert!(d.text.contains(&format!("up to {} yd", d.range[1])), "{}", d.text);
    assert!(d.text.contains(&format!("for {} sec ({} sec against heroes)", e.secs[1], e.hero_secs)), "{}", d.text);
    assert_eq!(s.content.ex_of(s.content.ability("bank_shot").unwrap()), s.content.ability("scatter_shot"));
}
