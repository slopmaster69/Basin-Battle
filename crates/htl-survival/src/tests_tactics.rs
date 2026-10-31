//! Creatures' tactics: whom they go for, an opening pressed at once, a guard met with the blow
//! no guard stops, ground given after a string, a hero's back sought while waiting a turn, a
//! raider drawn off its station by a hero near.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
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

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    id
}

fn run(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / DT).round() as usize {
        s.step(DT);
    }
}

#[test]
fn it_turns_on_the_hero_who_hurts_it_most_not_only_the_first_it_saw() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 8.0, 0.0], 0.0);
    let k = mob(&mut s, "grunt", [4.0, 4.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().health = 5000;
    s.mobs.get_mut(&k).unwrap().max_health = 5000;
    s.engage(k, Foe::Hero(a));
    s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 100.0;
    run(&mut s, 2.0);
    assert_eq!(s.mobs[&k].state.target(), Some(a), "on the first it saw");
    // Bren lays into it.
    for _ in 0..3 {
        s.hurt_mob(k, b, 600);
    }
    let t = s.content.ai.retarget_secs + 0.3;
    run(&mut s, t);
    assert_eq!(s.mobs[&k].state.target(), Some(b), "on whoever hurts it");
}

#[test]
fn two_creatures_on_two_heroes_spread_between_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 6.0, 0.0], 0.0);
    let ks: Vec<Id> = (0..4).map(|i| mob(&mut s, "wolf", [6.0, i as f32 * 2.0 - 1.0, 0.0])).collect();
    for k in &ks {
        s.mobs.get_mut(k).unwrap().ready_at = s.clock + 100.0;
        s.engage(*k, Foe::Hero(a));
    }
    let t = s.content.ai.retarget_secs * 2.0 + 0.5;
    run(&mut s, t);
    let on_b = ks.iter().filter(|k| s.mobs[k].state.target() == Some(b)).count();
    assert!(on_b >= 1, "not all of them on Ayla: {on_b} on Bren");
}

#[test]
fn a_heal_in_reach_is_pressed_at_once_past_its_cooldown() {
    for eating in [false, true] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        let k = mob(&mut s, "grunt", [2.5, 0.0, 0.0]);
        s.engage(k, Foe::Hero(p));
        s.step(DT);
        s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
        s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 0.9;
        if eating {
            let meat = s.content.item("roast").unwrap();
            s.players.get_mut(&p).unwrap().action = Action::Eat { t: 0.0, item: meat };
        }
        s.step(DT);
        let winding = matches!(s.mobs[&k].state, MobState::Windup { .. });
        assert_eq!(winding, eating, "eating {eating}");
    }
}

#[test]
fn a_raised_guard_draws_the_blow_no_guard_stops() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let d = s.content.mobs[s.mobs[&k].def].clone();
    let breaker = |k: usize| d.patterns[k].steps.iter().any(|st| st.blockable == Some(false) || st.heavy == Some(true));
    let count = |s: &mut Survival| (0..400).filter_map(|_| s.pick_pattern(k, 2.5, 0.4)).filter(|i| breaker(*i)).count();
    let open = count(&mut s);
    s.players.get_mut(&p).unwrap().guard_held = true;
    s.players.get_mut(&p).unwrap().action = Action::Guard;
    let guarded = count(&mut s);
    assert!(guarded as f32 > open as f32 * 1.4, "{open} open, {guarded} against a guard");
}

#[test]
fn now_and_then_it_gives_ground_after_a_string() {
    let mut s = world();
    s.content.ai.retreat = 1.0;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.2, 0.0, 0.0]);
    s.players.get_mut(&p).unwrap().health = 100_000;
    s.players.get_mut(&p).unwrap().max_health = 100_000;
    s.engage(k, Foe::Hero(p));
    // Until its first string is done and it's back to chasing.
    let mut furthest: f32 = 0.0;
    let mut done = false;
    for _ in 0..200 {
        let was = matches!(s.mobs[&k].state, MobState::Recover { .. });
        s.step(DT);
        if was && matches!(s.mobs[&k].state, MobState::Chase { .. }) {
            done = true;
        }
        if done {
            furthest = furthest.max(dist2d(s.mobs[&k].pos, ORIGIN));
        }
        if done && furthest > 4.0 {
            break;
        }
    }
    assert!(done, "a string came and went");
    assert!(furthest > 4.0, "it stepped back: {furthest}");
}

#[test]
fn waiting_its_turn_it_works_round_to_the_heros_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // In front of her, a little to one side.
    let k = mob(&mut s, "wolf", [3.0, 0.8, 0.0]);
    let way = s.flank_way(k, p).expect("not at her back yet");
    let at = facing(ORIGIN, s.mobs[&k].pos);
    let back = std::f32::consts::PI;
    let off = |a: f32| ((a - back + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs();
    assert!(off(at + way * 0.6) < off(at), "toward her back");
    // Behind her: its own way.
    s.mobs.get_mut(&k).unwrap().pos = [-3.0, 0.2, 0.0];
    assert!(s.flank_way(k, p).is_none());
}

#[test]
fn a_raider_at_a_station_turns_on_a_hero_who_comes_near() {
    let mut s = world();
    let def = s.content.mob("grunt").unwrap();
    let k = s.spawn_mob(def, [30.0, 0.0, 0.0], None, true);
    let st = s.alloc();
    let pal = s.content.station("palisade").unwrap();
    s.objs.insert(st, Obj { kind: ObjKind::Station { def: pal, uid: 99, health: 700, damaged: false }, pos: [32.0, 0.0, 0.0], facing: 0.0 });
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Obj(st) };
    let p = hero(&mut s, "Ayla", [80.0, 0.0, 0.0], 0.0);
    run(&mut s, 1.5);
    assert!(matches!(s.mobs[&k].state.foe(), Some(Foe::Obj(_))), "nobody near: the station");
    let at = s.mobs[&k].pos;
    s.move_player(p, [at[0] - 4.0, at[1], 0.0], 0.0, Moving::default());

    let t = s.content.ai.retarget_secs + 0.3;
    run(&mut s, t);
    assert_eq!(s.mobs[&k].state.target(), Some(p), "drawn off by the hero");
}
