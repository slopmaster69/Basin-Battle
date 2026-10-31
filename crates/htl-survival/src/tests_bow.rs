//! Shots up and down slopes and decks: an arrow flies the reticle's line in three dimensions (its
//! facing and its pitch, as the client sends them), and strikes the first thing it touches.

use crate::content::Content;
use crate::land::{FlatLand, Land};
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;

fn world_on(land: Box<dyn Land>) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, land, 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, pos: Vec3, facing: f32) -> Id {
    let id = s.join("bren", "Bren", None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    id
}

/// Back where it stood, calm (a creature struck turns on the shooter and comes).
fn still(s: &mut Survival, id: Id, pos: Vec3) {
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.path.clear();
    m.health = m.max_health;
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn struck(events: &[Event], victim: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Hit, anim: false, .. } if *v == victim))
}

/// Draw the bow, set the reticle's line (when given), and loose.
fn shoot(s: &mut Survival, p: Id, line: Option<(f32, f32)>) -> Vec<Event> {
    s.players.get_mut(&p).unwrap().cd_off = true;
    let _ = s.slot_ability(p, "ranged", "aimed_shot");
    s.input(p, Input::Ability(0, false));
    s.input(p, Input::Attack);
    run(s, 0.8);
    if let Some((f, e)) = line {
        s.set_aim_line(p, f, e);
    }
    s.input(p, Input::AttackUp);
    run(s, 1.4)
}

/// A slope rising away to the north: 0.4 yards up a yard past x = 5, up to 6.
struct Slope;

impl Land for Slope {
    fn height(&self, x: f32, _y: f32) -> f32 {
        ((x - 5.0) * 0.4).clamp(0.0, 6.0)
    }
}

#[test]
fn a_shot_up_a_slope_reaches_the_one_at_the_top() {
    let mut s = world_on(Box::new(Slope));
    let p = hero(&mut s, [0.0, 0.0, 0.0], 0.0);
    let up = mob(&mut s, [20.0, 0.0, 6.0]);
    // Aimed at their chest up there: the rising ground under the line doesn't stop it.
    let pitch = ((6.0 + 1.0 - 1.9) / 20.0f32).atan();
    let ev = shoot(&mut s, p, Some((0.0, pitch)));
    assert!(struck(&ev, up), "up the slope");
    // Level, it flies into the slope.
    still(&mut s, up, [20.0, 0.0, 6.0]);
    let ev = shoot(&mut s, p, Some((0.0, 0.0)));
    assert!(!struck(&ev, up), "into the ground");
}

#[test]
fn the_reticle_s_pitch_picks_who_the_arrow_finds_on_the_line() {
    let mut s = world_on(Box::new(FlatLand::default()));
    let p = hero(&mut s, [0.0, 0.0, 0.0], 0.0);
    // One close on the ground, one far and up high (on a deck), both on the line.
    let low = mob(&mut s, [12.0, 0.0, 0.0]);
    let high = mob(&mut s, [24.0, 0.0, 8.0]);
    let up = ((8.0 + 1.0 - 1.9) / 24.0f32).atan();
    let ev = shoot(&mut s, p, Some((0.0, up)));
    assert!(struck(&ev, high) && !struck(&ev, low), "aimed up, it passes over the near one");
    still(&mut s, low, [12.0, 0.0, 0.0]);
    still(&mut s, high, [24.0, 0.0, 8.0]);
    let ev = shoot(&mut s, p, Some((0.0, 0.0)));
    assert!(struck(&ev, low), "level, the near one");
}

#[test]
fn an_aimed_shot_flies_along_the_reticle_s_facing_not_a_stale_one() {
    let mut s = world_on(Box::new(FlatLand::default()));
    // The server last heard the hero facing north; the client has turned a little since.
    let p = hero(&mut s, [0.0, 0.0, 0.0], 0.0);
    let turn = 0.3f32;
    let there = mob(&mut s, [20.0 * turn.cos(), 20.0 * turn.sin(), 0.0]);
    let ahead = mob(&mut s, [20.0, -1.5, 0.0]);
    let ev = shoot(&mut s, p, Some((turn, 0.0)));
    assert!(struck(&ev, there) && !struck(&ev, ahead), "where the reticle was");
}

