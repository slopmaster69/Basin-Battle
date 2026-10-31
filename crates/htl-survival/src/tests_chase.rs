//! How creatures chase: the leash measured from home to the foe (a hero just past it still draws
//! the fight), evading on the way home, spreading round their foe, and giving up on a foe they
//! can find no way to.

use crate::content::Content;
use crate::land::{FlatLand, Land};
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;

fn world_on(land: Box<dyn Land>) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, land, 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    // The same rolls whatever the rules lay out first.
    s.rng = rand::SeedableRng::seed_from_u64(1);
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
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

#[test]
fn a_hero_shooting_from_just_past_the_leash_still_draws_the_fight() {
    let mut s = world_on(Box::new(FlatLand::default()));
    let leash = s.content.mobs[s.content.mob("grunt").unwrap()].leash;
    // The hero stands a little past the leash from where the grunt lives.
    let at = leash + 3.0;
    let p = hero(&mut s, "Ayla", [at, 0.0, 0.0]);
    let g = mob(&mut s, "grunt", [0.0, 0.0, 0.0]);
    s.hurt_mob(g, p, 1);
    run(&mut s, 6.0);
    assert!(matches!(s.mobs[&g].state, MobState::Chase { .. } | MobState::Windup { .. } | MobState::Recover { .. }), "it came: {:?}", s.mobs[&g].state);
    assert!(dist2d(s.mobs[&g].pos, [at, 0.0, 0.0]) < 4.0, "and reached the hero: {:?}", s.mobs[&g].pos);
}

#[test]
fn a_creature_gone_home_past_its_leash_evades_until_its_foe_comes_back() {
    let mut s = world_on(Box::new(FlatLand::default()));
    let leash = s.content.mobs[s.content.mob("grunt").unwrap()].leash;
    let p = hero(&mut s, "Ayla", [3.0, 0.0, 0.0]);
    let g = mob(&mut s, "grunt", [0.0, 0.0, 0.0]);
    // Drawn a way from home already.
    s.mobs.get_mut(&g).unwrap().home = [-20.0, 0.0, 0.0];
    s.mobs.get_mut(&g).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    // The hero runs well away: it gives up and heads home.
    s.move_player(p, [leash + 20.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 0.3);
    assert!(matches!(s.mobs[&g].state, MobState::Return), "{:?}", s.mobs[&g].state);
    assert!(s.evading(g, p));
    // Struck from out there, it takes nothing and keeps going home.
    let hp = s.mobs[&g].health;
    s.hurt_mob(g, p, 0);
    assert_eq!(s.mobs[&g].health, hp);
    // Back within the leash, it's a fair target again.
    s.move_player(p, [-10.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(DT);
    assert!(!s.evading(g, p));
}

#[test]
fn creatures_spread_round_their_foe_instead_of_standing_in_one_another() {
    let mut s = world_on(Box::new(FlatLand::default()));
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    // Both come from the same side.
    let a = mob(&mut s, "grunt", [12.0, 0.2, 0.0]);
    let b = mob(&mut s, "grunt", [12.5, -0.2, 0.0]);
    for m in [a, b] {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Chase { foe: Foe::Hero(p) };
        // Not ready to strike yet: they come up and wait.
        x.ready_at = 1e9;
    }
    run(&mut s, 4.0);
    let (pa, pb) = (s.mobs[&a].pos, s.mobs[&b].pos);
    assert!(dist2d(pa, pb) >= 1.2, "apart: {pa:?} {pb:?}");
    assert!(dist2d(pa, [0.0; 3]) < 6.0 && dist2d(pb, [0.0; 3]) < 6.0, "both up close: {pa:?} {pb:?}");
}

/// Level ground with a wall right across it, and a search that knows there's no way round.
struct Walled(FlatLand);

impl Land for Walled {
    fn height(&self, x: f32, y: f32) -> f32 {
        self.0.height(x, y)
    }
    fn blocked(&self, x: f32, _y: f32) -> bool {
        (x - 6.0).abs() < 0.6
    }
    fn plans(&self) -> bool {
        true
    }
}

#[test]
fn a_foe_it_can_find_no_way_to_is_stood_off_then_given_up() {
    let mut s = world_on(Box::new(Walled(FlatLand::default())));
    let p = hero(&mut s, "Ayla", [0.0, 0.0, 0.0]);
    let g = mob(&mut s, "grunt", [14.0, 0.0, 0.0]);
    s.mobs.get_mut(&g).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    run(&mut s, 1.0);
    assert!(s.mobs[&g].pos[0] > 6.5, "it doesn't walk through the wall: {:?}", s.mobs[&g].pos);
    run(&mut s, 5.0);
    assert!(matches!(s.mobs[&g].state, MobState::Return | MobState::Idle { .. }), "{:?}", s.mobs[&g].state);
}

