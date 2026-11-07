//! Temperaments: how each kind of creature fights (`[[temper]]`, docs/enemy-temperaments.md).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

fn world() -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    if let Some(h) = s.players.get_mut(&p) {
        h.max_health = 100_000;
        h.health = 100_000;
    }
    (s, p)
}

fn on_hero(s: &mut Survival, p: Id, key: &str, at: Vec3) -> Id {
    let def = s.content.mob(key).unwrap_or_else(|| panic!("no {key}"));
    let id = s.spawn_mob(def, at, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    id
}

#[test]
fn every_deadmines_creature_has_a_temperament_that_exists() {
    let c = Content::dev();
    for m in c.mobs.iter().filter(|m| m.key.starts_with("dm_")) {
        assert!(!m.temper.is_empty(), "{} has no temper", m.key);
        assert!(c.tempers.iter().any(|t| t.key == m.temper), "{}: no temper {}", m.key, m.temper);
    }
}

#[test]
fn a_backstabber_facing_its_hero_waits_and_strikes_once_behind_them() {
    let (mut s, p) = world();
    let b = on_hero(&mut s, p, "dm_blackguard", [2.0, 0.0, 0.0]);
    // The hero faces it (facing 0 is +x): it waits, its turn or not.
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    assert!(s.waits_turn(b, Foe::Hero(p), 2.0, false), "faced: it waits");
    // The hero turns away: it's behind them, and comes.
    s.move_player(p, [0.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    assert!(!s.waits_turn(b, Foe::Hero(p), 2.0, false), "behind them: it comes");
}

#[test]
fn a_bruisers_wind_up_shrugs_off_a_light_blow() {
    let (mut s, p) = world();
    let w = on_hero(&mut s, p, "dm_woodcutter", [2.0, 0.0, 0.0]);
    let pirate = on_hero(&mut s, p, "dm_pirate", [2.0, 0.0, 0.0]);
    s.mobs.get_mut(&pirate).unwrap().health = 0;
    assert!(s.temper(w).is_some_and(|t| t.poise));
    // Caught winding up by a light swing: a pirate would reel; a bruiser goes on.
    let mut reeled = false;
    for _ in 0..200 {
        if !matches!(s.mobs[&w].state, MobState::Windup { .. }) {
            let _ = s.step(0.02);
            continue;
        }
        s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
        s.input(p, Input::Attack);
        for _ in 0..30 {
            let _ = s.step(0.02);
            reeled |= s.mobs[&w].stagger_until > s.clock;
        }
        break;
    }
    assert!(!reeled, "a light blow didn't stagger the bruiser");
}

#[test]
fn a_coward_hurt_runs_from_its_hero_once() {
    let (mut s, p) = world();
    let m = on_hero(&mut s, p, "dm_miner", [2.0, 0.0, 0.0]);
    let max = s.mobs[&m].max_health;
    s.mobs.get_mut(&m).unwrap().health = (max as f32 * 0.2) as u32;
    let mut far: f32 = 0.0;
    for _ in 0..100 {
        let _ = s.step(0.02);
        far = far.max(dist2d(s.mobs[&m].pos, [0.0, 0.0, 0.0]));
    }
    assert!(s.mobs[&m].fled, "it ran");
    assert!(far > 6.0, "well away: {far:.1} yd");
}

#[test]
fn a_commander_holds_back_while_its_pack_is_on_the_hero() {
    let (mut s, p) = world();
    let boss = on_hero(&mut s, p, "dm_overseer", [3.0, 0.0, 0.0]);
    let _hand = on_hero(&mut s, p, "dm_miner", [2.0, 1.0, 0.0]);
    let mut struck = false;
    for _ in 0..250 {
        for e in s.step(0.02) {
            if matches!(e, Event::MobWindup { id, .. } if id == boss) {
                struck = true;
            }
        }
    }
    let off = dist2d(s.mobs[&boss].pos, [0.0, 0.0, 0.0]);
    assert!(!struck, "it held back while the miner fought");
    assert!(off > 5.0, "it stood off: {off:.1} yd");
}
