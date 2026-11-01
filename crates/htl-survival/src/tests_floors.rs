//! Creatures on floors over floors, below (a deck over the ground under it, as the Mast Room's):
//! a run across the deck never drops through it, and one that ends up under it anyway, or where
//! it can find no way to its foe nor home, is put back in its fight, and the log says so.

use crate::content::{Content, LampDef};
use crate::land::Land;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
/// The deck's height over the ground under it.
const DECK: f32 = 10.0;

/// Ground at 0 everywhere, and a deck 10 yards over it from -20 to 20 each way, with a slit
/// through its planks 0.3 yards wide (4.1 < x < 4.4, |y| < 1): narrower than the looks along a
/// walk, half a yard apart, but not than a runner's steps. No way runs between the deck and the
/// ground under it.
struct Deck;

impl Deck {
    fn on_deck(x: f32, y: f32) -> bool {
        x.abs() <= 20.0 && y.abs() <= 20.0 && !(x > 4.1 && x < 4.4 && y.abs() < 1.0)
    }

    /// The floor a walker coming from height `z` stands on (as the Deadmines' floors pick it).
    fn floor(x: f32, y: f32, z: f32) -> f32 {
        if Self::on_deck(x, y) && z >= DECK - 1.3 {
            DECK
        } else {
            0.0
        }
    }
}

impl Land for Deck {
    fn height(&self, x: f32, y: f32) -> f32 {
        Self::floor(x, y, 1.0e4)
    }
    fn height_near(&self, x: f32, y: f32, z: f32) -> f32 {
        Self::floor(x, y, z)
    }
    fn slab_between(&self, x: f32, y: f32, a: f32, b: f32) -> bool {
        let (lo, hi) = if a < b { (a, b) } else { (b, a) };
        Self::on_deck(x, y) && DECK > lo + 0.05 && DECK < hi - 0.05
    }
    fn plans(&self) -> bool {
        true
    }
    /// As the Deadmines' own: a look every half yard, floor to floor, never a drop.
    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool {
        let d = dist2d(from, to);
        let steps = (d / 0.5).ceil().max(1.0) as usize;
        let mut z = from[2];
        for k in 1..=steps {
            let t = k as f32 / steps as f32;
            let f = Self::floor(from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t, z);
            if (f - z).abs() > 1.2 * d / steps as f32 + 0.05 {
                return false;
            }
            z = f;
        }
        (z - to[2]).abs() < 1.0
    }
    /// A way across one floor only: none between the deck and the ground under it.
    fn path(&self, from: Vec3, to: Vec3) -> Option<Vec<Vec3>> {
        let (a, b) = (Self::floor(from[0], from[1], from[2]), Self::floor(to[0], to[1], to[2]));
        ((a - b).abs() < 1.0).then(|| vec![[to[0], to[1], b]])
    }
}

/// A dungeon on the deck: one lamp at its west end, and nobody down there but who a test brings.
fn below_deck() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.lamps = vec![LampDef { key: "deck".into(), name: "The deck".into(), at: [-15.0, 0.0, DECK], facing: 0.0 }];
    d.packs.clear();
    d.doors.clear();
    d.hazards.clear();
    d.boxes.clear();
    let mut s = Survival::new_dungeon(c, Box::new(Deck), 3, 1_000_000.0);
    s.mobs.clear();
    s.rng = rand::SeedableRng::seed_from_u64(1);
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// Sneed (out of his shredder), at rest at `pos`: his home.
fn sneed(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("dm_sneed").unwrap();
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

/// A hero on the deck far from where the tests' creatures are, out of their sight (the world
/// below is awake while someone's in it).
const AWAY: Vec3 = [-19.0, -19.0, DECK];

#[test]
fn a_creature_running_across_a_deck_never_drops_through_a_slit_its_way_was_looked_past() {
    let mut s = below_deck();
    hero(&mut s, "Ayla", AWAY);
    let g = sneed(&mut s, [0.0, 0.0, DECK]);
    let to = [8.0, 0.0, DECK];
    assert!(s.land().walk_clear([0.0, 0.0, DECK], to), "the looks along the way miss the slit");
    assert_eq!(s.land().height_near(4.125, 0.0, DECK), 0.0, "though a step in it is a step to the ground");
    // It runs along the deck over the slit (its eleventh step lands in it).
    s.flee_to(g, to, 7.5);
    let mut lowest = DECK;
    for _ in 0..12 {
        s.step(DT);
        lowest = lowest.min(s.mobs[&g].pos[2]);
    }
    let at = s.mobs[&g].pos;
    assert!(at[0] > 4.4, "past the slit: {at:?}");
    assert_eq!(lowest, DECK, "never down through it");
    // Stopped just past it (struck, its foe gone), it stands on the deck, not on the ground under
    // it with no way back up.
    s.halt_mob(g);
    s.step(DT);
    assert_eq!(s.mobs[&g].pos[2], DECK);
}

#[test]
fn below_a_creature_that_drops_through_its_floor_anyway_is_put_back_in_its_fight_and_logged() {
    let mut s = below_deck();
    let p = hero(&mut s, "Ayla", [12.0, 6.0, DECK]);
    let g = sneed(&mut s, [0.0, 0.0, DECK]);
    s.engage(g, Foe::Hero(p));
    // Whatever sends it there: a walk whose end is on the ground under the deck.
    {
        let m = s.mobs.get_mut(&g).unwrap();
        m.path = vec![[0.2, 0.0, 0.0]];
        m.speed = 7.5;
    }
    let ev = s.step(DT);
    let m = &s.mobs[&g];
    assert_eq!(m.pos[2], DECK, "back up on the deck: {:?}", m.pos);
    assert_eq!(m.state.foe(), Some(Foe::Hero(p)), "still after its foe: {:?}", m.state);
    assert!(ev.iter().any(|e| matches!(e, Event::Log(t) if t.starts_with("Sneed fell through the floor"))), "the log says so: {ev:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::MobStop { id, pos, .. } if *id == g && pos[2] == DECK)), "seen back up there");
}

#[test]
fn below_a_boss_under_its_deck_with_no_way_to_its_foe_nor_home_is_put_back_home_and_fights_on() {
    // The play-test's: Sneed down under the Mast Room's deck, his foe up on it, no way up.
    let mut s = below_deck();
    let p = hero(&mut s, "Ayla", [10.0, 0.0, DECK]);
    let g = sneed(&mut s, [0.0, 0.0, DECK]);
    s.engage(g, Foe::Hero(p));
    {
        let m = s.mobs.get_mut(&g).unwrap();
        m.health -= 100;
        // Not striking for now, only coming for them.
        m.ready_at = 1e9;
        m.pos = [0.0, 0.0, 0.0];
    }
    let ev = run(&mut s, 5.0);
    let m = &s.mobs[&g];
    assert_eq!(m.pos[2], DECK, "back up on the deck: {:?}", m.pos);
    assert_eq!(m.state.foe(), Some(Foe::Hero(p)), "still in the fight: {:?}", m.state);
    assert!(m.health < m.max_health, "not made whole as if the fight were over");
    assert!(ev.iter().any(|e| matches!(e, Event::Log(t) if t.starts_with("Sneed found no way to its foe nor home"))), "the log says so");
    // Up there it comes for its foe.
    run(&mut s, 2.0);
    let (at, foe) = (s.mobs[&g].pos, s.players[&p].pos);
    assert!(dist2d(at, foe) < 4.5 && at[2] == DECK, "at them: {at:?}");
}

#[test]
fn below_a_creature_sent_home_from_under_its_deck_is_home_only_up_on_it() {
    let mut s = below_deck();
    hero(&mut s, "Ayla", AWAY);
    let going = sneed(&mut s, [0.0, 0.0, DECK]);
    let resting = sneed(&mut s, [6.0, -6.0, DECK]);
    // Under their homes (however they came there): one on its way home, one at rest.
    s.mobs.get_mut(&going).unwrap().pos = [0.5, 0.0, 0.0];
    s.mobs.get_mut(&going).unwrap().state = MobState::Return;
    s.mobs.get_mut(&resting).unwrap().pos = [6.0, -5.5, 0.0];
    let ev = run(&mut s, 1.0);
    for g in [going, resting] {
        let m = &s.mobs[&g];
        assert_eq!(m.pos[2], DECK, "home, up on the deck: {:?}", m.pos);
        assert!(matches!(m.state, MobState::Idle { .. }), "at rest there: {:?}", m.state);
    }
    let lines = ev.iter().filter(|e| matches!(e, Event::Log(t) if t.contains("found no way home"))).count();
    assert_eq!(lines, 2, "each logged");
}
