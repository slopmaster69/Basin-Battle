//! An ambusher lying unseen (`hidden`: a trial's guardians, a strongbox's keepers) is not there to
//! be struck. Found by the soak (Nov 16): a hero's ordinary swing, and the ground a hero left
//! burning, hit hidden creatures that nothing had woken; they took the blow, turned on the hero
//! and went home, and a hidden creature never thinks, so they stood in `Return` at home, hurt,
//! for good.

use super::*;
use crate::land::FlatLand;

const DT: f32 = 0.05;

fn world() -> (Survival, Id, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    let h = s.join("amb", "Amb", None);
    s.move_player(h, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    // A grunt a step in front of the hero, lying unseen.
    let grunt = s.spawn_mob(s.content.mob("grunt").unwrap(), [1.5, 0.0, 0.0], None, false);
    s.mobs.get_mut(&grunt).unwrap().hidden = true;
    (s, h, grunt)
}

fn untouched(s: &Survival, grunt: Id) {
    let m = &s.mobs[&grunt];
    assert_eq!(m.health, m.max_health, "the hidden creature took no blow");
    assert!(matches!(m.state, mobs::MobState::Idle { .. }), "and was not drawn into a fight: {:?}", m.state);
}

#[test]
fn a_swing_does_not_find_a_hidden_creature() {
    let (mut s, h, grunt) = world();
    s.set_aim_line(h, 0.0, 0.0);
    for _ in 0..40 {
        s.input(h, Input::Attack);
        s.input(h, Input::AttackUp);
        s.step(DT);
    }
    untouched(&s, grunt);
}

#[test]
fn burning_ground_does_not_find_a_hidden_creature() {
    let (mut s, h, grunt) = world();
    let ability = s.content.abilities.iter().position(|a| a.kind == "ground").expect("a ground card");
    let now = s.clock;
    s.burnings.push(abilities::Burning { owner: h, ability, at: [1.5, 0.0, 0.0], radius: 4.0, dps: 20.0, until: now + 10.0, next: now, mark: None });
    for _ in 0..40 {
        s.step(DT);
    }
    untouched(&s, grunt);
}

/// (Found by the soak a second time, Nov 16: a Warlord's "Summon Infernal" landing near a trial
/// shrine hurt its guardians the same way, and an ability's blow reaches a creature by
/// `ability_on_mob`, whatever chose it: the swing, the ground, the infernal, a bolt.)
#[test]
fn an_ability_that_lands_does_not_find_a_hidden_creature() {
    let (mut s, h, grunt) = world();
    for ability in 0..s.content.abilities.len() {
        s.ability_on_mob(h, grunt, ability, 1.0, 60.0, [0.0, 0.0, 0.0], abilities::HitKind::Area, 0);
        s.ability_on_mob(h, grunt, ability, 1.0, 60.0, [0.0, 0.0, 0.0], abilities::HitKind::Projectile, 0);
        untouched(&s, grunt);
    }
}

/// ...and whatever else deals a creature damage (an orb, a channel, a meteor, a curse) goes
/// through `hurt_mob_as`, which is not there to be hurt either.
#[test]
fn nothing_hurts_a_hidden_creature() {
    let (mut s, h, grunt) = world();
    s.hurt_mob_as(grunt, h, 40, "Meteor");
    s.hurt_mob_as(grunt, h, 40, "Melee");
    untouched(&s, grunt);
}

/// ...and none of what a blow sets off on the way to the health: a freeze it would end, a shield
/// it would soak.
#[test]
fn a_blow_on_a_hidden_creature_ends_none_of_its_holds() {
    let (mut s, h, grunt) = world();
    assert!(s.apply_cc(grunt, h, "petrify", 30.0, 1.0, 0.0, "Ice"));
    s.hurt_mob_as(grunt, h, 40, "Meteor");
    s.ability_on_mob(h, grunt, 0, 1.0, 40.0, [0.0, 0.0, 0.0], abilities::HitKind::Area, 0);
    assert!(s.has_cc(grunt, "petrify"), "the freeze was not touched");
    untouched(&s, grunt);
}
