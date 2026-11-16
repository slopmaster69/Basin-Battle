//! A creature at rest far from every hero sleeps (the land is wide), but a raider never does.
//! Found by the soak (Nov 16): a raider left with no foe (no station standing and no hero up, for a
//! moment) went `Idle`, slept where it stood far from the heroes, and never looked for a foe
//! again, so the raid stayed `Active` for good with its last raiders asleep in the hills.

use super::*;
use crate::land::FlatLand;

const DT: f32 = 0.05;

#[test]
fn a_raider_asleep_far_from_the_heroes_wakes_and_marches_on() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 9, 1_000_000.0);
    let h = s.join("camp", "Camp", None);
    s.move_player(h, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let far = mobs::SLEEP_RANGE * 3.0;
    let raider = s.spawn_mob(s.content.mob("grunt").unwrap(), [far, 0.0, 0.0], None, true);
    s.raid.raiders.push(raider);
    s.raid.phase = raid::RaidPhase::Active;
    s.mobs.get_mut(&raider).unwrap().state = mobs::MobState::Idle { wander_at: 0.0 };
    for _ in 0..40 {
        s.step(DT);
    }
    let m = &s.mobs[&raider];
    assert!(matches!(m.state, mobs::MobState::Chase { .. }), "it went after the hero: {:?}", m.state);
    assert!(m.pos[0] < far, "and is on its way: {:?}", m.pos);
}

#[test]
fn an_ordinary_creature_far_from_every_hero_still_sleeps() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 9, 1_000_000.0);
    let h = s.join("camp", "Camp", None);
    s.move_player(h, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let far = mobs::SLEEP_RANGE * 3.0;
    let grunt = s.spawn_mob(s.content.mob("grunt").unwrap(), [far, 0.0, 0.0], None, false);
    s.mobs.get_mut(&grunt).unwrap().state = mobs::MobState::Idle { wander_at: 0.0 };
    for _ in 0..40 {
        s.step(DT);
    }
    assert_eq!(s.mobs[&grunt].pos, [far, 0.0, 0.0], "it never stirred");
}
