use super::*;
use crate::land::FlatLand;

fn fight() -> (Survival, Id, Id, Vec<Id>) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 7, 0.0);
    let hero = s.join("smith", "Smith", None);
    let p = s.players.get_mut(&hero).unwrap();
    p.pos = [0.0, 0.0, 0.0];
    p.health = 100_000;
    p.max_health = 100_000;
    let boss = s.spawn_mob(s.content.mob("dm_gilnid").unwrap(), [0.0, 0.0, 0.0], None, false);
    s.step_perch_crews();
    let crew = s.perches.crews[&boss].workers.clone();
    assert_eq!(crew.len(), 2);
    s.engage(boss, Foe::Hero(hero));
    s.step_perch_crews();
    (s, hero, boss, crew)
}

fn tick(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / 0.05).round() as usize {
        s.clock += 0.05;
        s.step_missiles();
        s.step_mobs(0.05);
    }
}

fn shot(s: &mut Survival, hero: Id, worker: Id, damage: f32, how: HitKind) {
    let ability = s.content.ability("fireball").unwrap();
    s.content.abilities[ability].effects.clear();
    s.ability_on_mob(hero, worker, ability, 1.0, damage, s.players[&hero].pos, how, 133);
}

#[test]
fn one_projectile_drops_once_then_lands_stunned_as_a_melee_worker() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[0];
    let from = s.mobs[&id].pos;
    let landing = s.perches.workers[&id].landing;
    shot(&mut s, hero, id, 1.0, HitKind::Projectile);
    let since = match s.perches.workers[&id].state { Place::Falling { since, .. } => since, _ => panic!("one hit drops it") };
    tick(&mut s, 0.4);
    assert!(s.mobs[&id].pos[2] < from[2] && s.mobs[&id].pos[2] > landing[2]);
    shot(&mut s, hero, id, 1.0, HitKind::Projectile);
    assert!(matches!(s.perches.workers[&id].state, Place::Falling { since: t, .. } if t == since));
    tick(&mut s, 0.5);
    let m = &s.mobs[&id];
    assert_eq!(m.pos, landing);
    assert!(m.stagger_until > s.clock + 2.0);
    assert!(!s.content.mobs[m.def].attack.ranged);
    assert!(matches!(s.perches.workers[&id].state, Place::Ground));
}

#[test]
fn area_damage_and_zero_damage_do_not_count_as_a_direct_shot() {
    let (mut s, hero, _, crew) = fight();
    shot(&mut s, hero, crew[0], 1.0, HitKind::Area);
    shot(&mut s, hero, crew[0], 0.0, HitKind::Projectile);
    s.hurt_mob_as(crew[0], hero, 1, "burning");
    assert!(matches!(s.perches.workers[&crew[0]].state, Place::Perched));
}

#[test]
fn a_real_fireball_drops_its_contact_but_not_a_nearby_splash_victim() {
    let (mut s, hero, _, crew) = fight();
    let at = s.mobs[&crew[0]].pos;
    s.players.get_mut(&hero).unwrap().pos = [at[0] - 6.0, at[1], at[2]];
    s.players.get_mut(&hero).unwrap().facing = 0.0;
    s.mobs.get_mut(&crew[1]).unwrap().pos = [at[0], at[1] + 1.5, at[2]];
    let ability = s.content.ability("fireball").unwrap();
    s.launch(hero, ability, 0.0);
    for _ in 0..20 { s.clock += 0.05; s.step_projectiles(0.05); }
    assert!(s.mobs[&crew[1]].health < s.mobs[&crew[1]].max_health, "the neighbor took splash damage");
    assert!(matches!(s.perches.workers[&crew[0]].state, Place::Falling { .. }));
    assert!(matches!(s.perches.workers[&crew[1]].state, Place::Perched));
}

#[test]
fn a_melee_support_requires_reach_facing_and_the_same_floor() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[0];
    let base = s.perches.workers[&id].base;
    s.players.get_mut(&hero).unwrap().pos = [base[0] - 2.0, base[1], base[2]];
    s.swing_on_perches(hero, std::f32::consts::PI, 3.0, 90.0);
    assert!(matches!(s.perches.workers[&id].state, Place::Perched));
    s.players.get_mut(&hero).unwrap().pos[2] += 20.0;
    s.swing_on_perches(hero, 0.0, 3.0, 90.0);
    assert!(matches!(s.perches.workers[&id].state, Place::Perched));
    s.players.get_mut(&hero).unwrap().pos[2] = base[2];
    s.swing_on_perches(hero, 0.0, 3.0, 90.0);
    assert!(matches!(s.perches.workers[&id].state, Place::Falling { .. }));
}

#[test]
fn a_lethal_shot_leaves_a_falling_corpse_and_never_resurrects_it() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[0];
    let landing = s.perches.workers[&id].landing;
    shot(&mut s, hero, id, 10_000.0, HitKind::Projectile);
    assert_eq!(s.mobs[&id].health, 0);
    tick(&mut s, 0.4);
    assert!(s.mobs[&id].pos[2] > landing[2]);
    tick(&mut s, 0.5);
    assert_eq!(s.mobs[&id].pos, landing);
    assert_eq!(s.mobs[&id].health, 0);
    tick(&mut s, 4.0);
    assert!(!s.mobs.contains_key(&id));
}

#[test]
fn one_explosive_at_a_time_and_every_worker_comes_down_without_ranged_abilities() {
    let (mut s, _, boss, crew) = fight();
    let mut saw_throw = false;
    let mut saw_demo = false;
    for _ in 0..700 {
        tick(&mut s, 0.05);
        let flying = s.missiles.iter().filter(|m| crew.contains(&m.owner)).count();
        let winding = crew.iter().filter(|id| s.perch_controlled(**id)
            && s.mobs.get(id).is_some_and(|m| matches!(m.state, MobState::Windup { .. }))).count();
        assert!(flying + winding <= 1, "one committed dynamite at a time");
        for id in &crew {
            if s.perch_controlled(*id) && matches!(s.mobs[id].state, MobState::Windup { .. }) {
                assert!(s.mobs[id].lob.is_some(), "the fuse warns a fixed landing before release");
            }
        }
        saw_throw |= flying > 0;
        saw_demo |= s.perches.crews[&boss].demonstration.is_some();
    }
    assert!(saw_throw && saw_demo);
    assert!(crew.iter().all(|id| matches!(s.perches.workers[id].state, Place::Ground)));
}

#[test]
fn wiping_removes_crew_props_and_projectiles_and_repull_creates_only_one_crew() {
    let (mut s, _, boss, crew) = fight();
    tick(&mut s, 5.0);
    let props = s.perches.crews[&boss].props.clone();
    s.mobs.get_mut(&boss).unwrap().state = MobState::Return;
    s.step_perch_crews();
    assert!(crew.iter().all(|id| !s.mobs.contains_key(id)));
    assert!(props.iter().all(|id| !s.objs.contains_key(id)));
    assert!(s.missiles.iter().all(|m| !crew.contains(&m.owner)));
    s.mobs.get_mut(&boss).unwrap().state = MobState::Idle { wander_at: 99.0 };
    s.step_perch_crews();
    s.step_perch_crews();
    assert_eq!(s.perches.workers.len(), 2);
    assert_eq!(s.perches.crews[&boss].props.len(), 5);
}

#[test]
fn the_valve_follows_the_workforce_not_the_boss_health_and_can_be_delayed() {
    let (mut s, hero, boss, crew) = fight();
    for id in crew { s.hurt_mob_as(id, hero, 10_000, "test"); }
    tick(&mut s, 1.0);
    assert_eq!(s.mobs[&boss].health, s.mobs[&boss].max_health);
    assert!(matches!(s.perches.crews[&boss].stage, Stage::ToValve { .. }));
    tick(&mut s, 1.5);
    assert!(matches!(s.perches.crews[&boss].stage, Stage::Opening { .. }));
    s.stagger_mob(boss, 2.0, false);
    tick(&mut s, 1.0);
    assert!(!s.missiles.iter().any(|m| m.owner == boss));
    tick(&mut s, 13.0);
    assert!(matches!(s.perches.crews[&boss].stage, Stage::Finished));
    assert_eq!(s.perches.workers.len(), 2, "no replacement workers");
    s.hurt_mob_as(boss, hero, 100_000, "test");
    assert!(s.perches.crews.is_empty() && s.perches.workers.is_empty());
}

#[test]
fn a_platform_on_another_floor_is_omitted_without_stranding_the_encounter() {
    let (mut s, _, boss, _) = fight();
    s.remove_perch_crew(boss);
    s.land = Box::new(FlatLand { z: 30.0, ..Default::default() });
    s.place_perch_crew(boss);
    assert!(s.perches.crews[&boss].workers.is_empty());
    s.step_perch_crews();
    assert!(matches!(s.perches.crews[&boss].stage, Stage::ToValve { .. }));
}

#[test]
fn native_steam_expires_at_impact_and_owner_death_cleans_pending_warnings_and_throws() {
    let (mut s, hero, boss, crew) = fight();
    s.perch_overflow(boss, [4.0, 0.0, 0.0], 0.5, 1.5, 12);
    let steam = s.perches.crews[&boss].steam[0].0;
    assert!(matches!(s.objs[&steam].kind, ObjKind::Prop { display: 353, .. }));
    tick(&mut s, 0.6);
    assert!(!s.objs.contains_key(&steam));
    tick(&mut s, 4.4);
    assert!(s.missiles.iter().any(|m| crew.contains(&m.owner)), "a dynamite is in flight");
    s.perch_overflow(boss, [4.0, 0.0, 0.0], 1.4, 1.5, 12);
    let steam = s.perches.crews[&boss].steam[0].0;
    let props = s.perches.crews[&boss].props.clone();
    let marks: Vec<_> = s.missiles.iter().filter(|m| m.owner == boss || crew.contains(&m.owner)).map(|m| m.mark).collect();
    s.hurt_mob_as(boss, hero, 100_000, "test");
    assert!(!s.objs.contains_key(&steam));
    assert!(props.iter().all(|id| !s.objs.contains_key(id)));
    assert!(crew.iter().all(|id| !s.mobs.contains_key(id)));
    assert!(s.missiles.iter().all(|m| m.owner != boss && !crew.contains(&m.owner)));
    assert!(marks.iter().all(|id| s.mark_left(*id) == 0.0));
}

#[test]
fn a_surviving_party_member_keeps_the_same_crew_and_can_drop_a_worker() {
    let (mut s, first, boss, crew) = fight();
    let second = s.join("smith2", "Smith2", None);
    let p = s.players.get_mut(&second).unwrap();
    p.pos = [2.0, 0.0, 0.0];
    p.health = 100_000;
    p.max_health = 100_000;
    tick(&mut s, 0.2);
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    tick(&mut s, 0.1);
    assert_eq!(s.mobs[&boss].state.target(), Some(second));
    assert_eq!(s.perches.crews[&boss].workers, crew);
    assert!(crew.iter().all(|id| s.mobs[id].state.target() == Some(second)));
    shot(&mut s, second, crew[0], 1.0, HitKind::Projectile);
    assert!(matches!(s.perches.workers[&crew[0]].state, Place::Falling { .. }));
    assert_eq!(s.perches.workers.len(), 2);
}
