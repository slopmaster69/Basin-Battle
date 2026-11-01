//! The damage meter: what a hero dealt, healed and took, by source, in their fight and since they
//! reset it; only what landed counts.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules on flat land, no creatures but the test's, and no health coming back by
/// itself (so a heal on the meter is all a hero gained).
fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    c.player.field_regen = 0.0;
    c.player.rest_regen = 0.0;
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

fn grunt(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("grunt").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

/// Creatures held idle where they stand while the world runs.
fn run_pinned(s: &mut Survival, secs: f32, pins: &[Id]) {
    let at: Vec<(Id, Vec3)> = pins.iter().filter_map(|m| s.mobs.get(m).map(|x| (*m, x.pos))).collect();
    for _ in 0..(secs / DT).round() as usize {
        for (m, p) in &at {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *p;
            }
        }
        s.step(DT);
    }
}

fn cast(s: &mut Survival, p: Id, key: &str) {
    let a = s.content.ability(key).unwrap();
    let slot = s.content.abilities[a].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
    let n = crate::content::SLOTS.iter().position(|x| *x == slot).unwrap() as u8;
    s.input(p, Input::Ability(n, false));
    s.step(DT);
}

fn row<'a>(m: &'a meter::OwnMeter, label: &str) -> Option<&'a (String, u64, u64)> {
    m.rows.iter().find(|r| r.0 == label)
}

#[test]
fn a_cards_damage_and_healing_count_under_its_name_and_only_what_landed() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [6.0, -0.5, 0.0], 0.0);
    let g = grunt(&mut s, [10.0, 0.0, 0.0]);
    let half = {
        let h = s.players.get_mut(&f).unwrap();
        h.health = h.max_health / 2;
        h.health
    };
    let full = s.mobs[&g].health;
    assert!(s.own_meter(p).unwrap().rows.is_empty(), "nothing before a blow");
    assert!(s.fight_meter(p).is_none(), "and no fight yet");
    cast(&mut s, p, "penance");
    run_pinned(&mut s, 1.6, &[g]);
    let dealt = u64::from(full - s.mobs[&g].health);
    let healed = u64::from(s.players[&f].health - half);
    assert!(dealt > 0 && healed > 0, "the lance burned the grunt ({dealt}) and mended Bren ({healed})");
    let own = s.own_meter(p).unwrap();
    assert_eq!(own.dealt, dealt, "her meter holds what the grunt lost");
    assert_eq!(own.healed, healed, "and what Bren gained");
    assert_eq!(row(&own, "Penance"), Some(&("Penance".to_string(), dealt, healed)), "both under Penance: {:?}", own.rows);
    let fight = s.fight_meter(p).unwrap();
    assert!(fight.live, "her fight is on");
    assert_eq!(fight.rows[0].name, "Ayla");
    assert_eq!((fight.rows[0].dealt, fight.rows[0].healed), (dealt, healed), "the fight's numbers too");
    assert!(s.own_meter(f).unwrap().rows.is_empty(), "nothing on Bren's own meter");
}

#[test]
fn overkill_doesnt_count_and_a_weapon_counts_as_melee() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = grunt(&mut s, [10.0, 0.0, 0.0]);
    let left = 7;
    s.mobs.get_mut(&g).unwrap().health = left;
    s.hurt_mob(g, p, 500);
    let own = s.own_meter(p).unwrap();
    assert_eq!(own.dealt, u64::from(left), "only the health it had left");
    assert_eq!(row(&own, "Melee").map(|r| r.1), Some(u64::from(left)), "a plain blow is Melee: {:?}", own.rows);
}

#[test]
fn damage_taken_counts_and_a_lull_ends_the_fight_but_not_the_running_totals() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = grunt(&mut s, [30.0, 0.0, 0.0]);
    s.hurt_mob_as(g, p, 20, "Fireball");
    s.fall_damage(p, 15);
    let fight = s.fight_meter(p).unwrap();
    assert!(fight.live);
    assert_eq!((fight.rows[0].dealt, fight.rows[0].taken), (20, 15));
    // A lull longer than a fight's: it's over, its clock stopped.
    let lull = s.content.player.combat_secs + 1.0;
    run_pinned(&mut s, lull, &[g]);
    let over = s.fight_meter(p).unwrap();
    assert!(!over.live, "over after a lull");
    assert_eq!(over.rows[0].dealt, 20, "still shown till the next one");
    // The next blow begins a new fight; the running totals keep both.
    s.hurt_mob_as(g, p, 5, "Fireball");
    let next = s.fight_meter(p).unwrap();
    assert!(next.live);
    assert_eq!((next.rows[0].dealt, next.rows[0].taken), (5, 0), "a fresh fight");
    let own = s.own_meter(p).unwrap();
    assert_eq!((own.dealt, own.taken), (25, 15), "since the reset: both fights");
    // The lull's seconds don't count towards the per-second figure.
    assert!(own.secs < 2.0, "only time in fights: {}", own.secs);
}

#[test]
fn a_reset_starts_your_own_meter_over_but_leaves_the_fight() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let g = grunt(&mut s, [30.0, 0.0, 0.0]);
    s.hurt_mob_as(g, p, 20, "Fireball");
    s.reset_meter(p);
    let own = s.own_meter(p).unwrap();
    assert_eq!((own.dealt, own.rows.len()), (0, 0), "started over");
    assert_eq!(s.fight_meter(p).unwrap().rows[0].dealt, 20, "the fight's own numbers stay");
}

#[test]
fn heroes_fighting_together_share_the_fight_meter_most_damage_first_and_a_faraway_one_doesnt() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [4.0, 0.0, 0.0], 0.0);
    let far = hero(&mut s, "Cass", [400.0, 0.0, 0.0], 0.0);
    let g = grunt(&mut s, [8.0, 0.0, 0.0]);
    let h = grunt(&mut s, [404.0, 0.0, 0.0]);
    s.hurt_mob_as(g, a, 10, "Fireball");
    s.hurt_mob_as(g, b, 25, "Melee");
    s.hurt_mob_as(h, far, 40, "Melee");
    let fight = s.fight_meter(a).unwrap();
    let names: Vec<&str> = fight.rows.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Bren", "Ayla"], "Bren hit harder; Cass is fighting elsewhere");
    // A friend's heal on a hero in a fight carries the healer into it.
    let c = hero(&mut s, "Dara", [2.0, 2.0, 0.0], 0.0);
    s.players.get_mut(&a).unwrap().health -= 10;
    s.heal_hero_by(a, 10, c, "Living Seed");
    let names: Vec<String> = s.fight_meter(a).unwrap().rows.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains(&"Dara".to_string()), "the healer is in the fight: {names:?}");
    // A heal out of any fight counts on the healer's own meter but begins no fight.
    let mut s = world();
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let c = hero(&mut s, "Dara", [2.0, 2.0, 0.0], 0.0);
    s.players.get_mut(&a).unwrap().health -= 10;
    s.heal_hero_by(a, 10, c, "Living Seed");
    assert_eq!(s.own_meter(c).unwrap().healed, 10);
    assert!(s.fight_meter(c).is_none(), "no fight begun by mending a friend at rest");
}
