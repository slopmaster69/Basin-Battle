use super::*;
use crate::land::FlatLand;

fn fight(duo: bool) -> (Survival, Id, Id, Vec<Id>, Option<Id>) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let hero = s.join("boarder", "Boarder", None);
    let other = duo.then(|| s.join("mate", "Mate", None));
    for id in [Some(hero), other].into_iter().flatten() {
        let p = s.players.get_mut(&id).unwrap();
        p.pos = [8.0, 0.0, 0.0];
        p.health = 100_000;
        p.max_health = 100_000;
    }
    let boss = s.spawn_mob(s.content.mob("dm_greenskin").unwrap(), [0.0; 3], None, false);
    s.engage(boss, Foe::Hero(hero));
    tick(&mut s, 0.05);
    let crew = s.firing_line.crews[&boss].gunners.clone();
    assert_eq!(crew.len(), 2);
    (s, hero, boss, crew, other)
}

fn tick(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / 0.05).round() as usize {
        s.clock += 0.05;
        s.step_players(0.05);
        s.step_projectiles(0.05);
        s.step_pending();
        s.step_missiles();
        s.step_mobs(0.05);
    }
}

#[test]
fn executed_opening_staggers_both_shots_then_gives_the_captain_the_reload() {
    for duo in [false, true] {
        let (mut s, _, boss, crew, _) = fight(duo);
        let mut shots = Vec::new();
        let mut first_cut = None;
        while s.clock < 15.0 {
            s.events.clear();
            tick(&mut s, 0.05);
            for e in &s.events {
                if let Event::Missile { caster, spell: 8682, .. } = e {
                    if crew.contains(caster) { shots.push((*caster, s.clock)); }
                }
                if matches!(e, Event::MobWindup { id, .. } if *id == boss) { first_cut.get_or_insert(s.clock); }
            }
        }
        assert!(shots.len() >= 2, "the actual AI must fire, not only set timers");
        assert_eq!((shots[0].0, shots[1].0), (crew[0], crew[1]));
        assert!(shots[1].1 - shots[0].1 >= SHOT_GAP - 0.1);
        let cut = first_cut.expect("captain attacks while they reload");
        assert!(cut > shots[1].1 + 0.7, "no three-way opening burst");
    }
}

#[test]
fn shouldered_aim_stays_frozen_and_a_sidestep_avoids_the_actual_shot() {
    let (mut s, h, _, crew, _) = fight(false);
    let g = crew[0];
    while !matches!(s.mobs[&g].state, MobState::Windup { .. }) { tick(&mut s, 0.05); }
    let at = s.players[&h].pos;
    let face = s.mobs[&g].facing;
    let health = s.players[&h].health;
    tick(&mut s, 0.5);
    s.players.get_mut(&h).unwrap().pos[1] += 3.0;
    tick(&mut s, 1.0);
    assert_eq!(s.mobs[&g].facing, face);
    assert_eq!(s.firing_line.aims[&g], at);
    assert!(s.events.iter().any(|e| matches!(e, Event::Clip { id, clip: 48, .. } if *id == g)));
    tick(&mut s, 1.0);
    assert_eq!(s.players[&h].health, health);
    assert!(s.events.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Point(p), .. } if *caster == g && *p == at)));
    assert!(!s.events.iter().any(|e| matches!(e, Event::Mark { .. })), "the first volley needs no floor disk");
}

#[test]
fn crew_deaths_not_health_unlock_one_visible_pistol_draw() {
    let (mut s, h, boss, crew, _) = fight(false);
    let hp = s.mobs[&boss].health;
    s.hurt_mob(boss, h, hp - 1);
    tick(&mut s, 10.0);
    assert_eq!(s.mobs[&boss].phase, 0);
    assert!(!s.firing_pattern_allowed(boss, "pistol shot"));
    s.hurt_mob(crew[0], h, u32::MAX);
    tick(&mut s, 4.0);
    assert_eq!(s.mobs[&boss].phase, 0);
    assert!(!s.firing_pattern_allowed(boss, "harpoon"));
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
    s.hurt_mob(crew[1], h, u32::MAX);
    let until = s.clock + 5.0;
    while s.mobs[&boss].phase == 0 && s.clock < until { tick(&mut s, 0.05); }
    assert_eq!(s.mobs[&boss].phase, 1);
    assert!(s.firing_line_gun(boss).is_some());
    assert!(s.firing_pattern_allowed(boss, "harpoon"));
    assert!(s.events.iter().any(|e| matches!(e, Event::Cast { caster, spell: 8682, ms: 2400 } if *caster == boss)));
    assert!(s.content.mobs[s.mobs[&boss].def].phases.is_empty());
    tick(&mut s, 15.0);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew, "no replacement wave");
}

#[test]
fn a_party_death_retargets_the_same_finite_crew() {
    let (mut s, first, boss, crew, other) = fight(true);
    let second = other.unwrap();
    tick(&mut s, 1.3);
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    tick(&mut s, 0.1);
    assert_eq!(s.mobs[&boss].state.target(), Some(second));
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
    assert!(crew.iter().all(|g| s.mobs[g].state.target() == Some(second)));
    tick(&mut s, 12.0);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
}

#[test]
fn wipe_repull_reset_and_captain_death_clear_every_owned_attack() {
    let (mut s, h, boss, crew, _) = fight(false);
    tick(&mut s, 1.4);
    s.players.get_mut(&h).unwrap().health = 0;
    tick(&mut s, 0.1);
    assert!(s.firing_line.crews.is_empty());
    assert!(crew.iter().all(|g| !s.mobs.contains_key(g)));
    assert!(s.firing_line.aims.is_empty());
    tick(&mut s, 2.0);
    s.players.get_mut(&h).unwrap().health = 100_000;
    s.engage(boss, Foe::Hero(h));
    tick(&mut s, 0.1);
    let next = s.firing_line.crews[&boss].gunners.clone();
    assert_eq!(next.len(), 2);
    assert!(next.iter().all(|g| !crew.contains(g)));
    s.reset_firing_lines();
    assert!(s.firing_line.owners.is_empty());
    s.engage(boss, Foe::Hero(h));
    tick(&mut s, 0.1);
    let last = s.firing_line.crews[&boss].gunners.clone();
    tick(&mut s, 3.2);
    s.hurt_mob(boss, h, u32::MAX);
    assert!(last.iter().all(|g| !s.mobs.contains_key(g)));
    assert!(s.firing_line.crews.is_empty() && s.firing_line.aims.is_empty() && s.firing_line.hooks.is_empty());
    assert!(s.missiles.iter().all(|m| m.owner != boss && !last.contains(&m.owner)));
    assert!(s.pending.iter().all(|p| p.attacker != boss && !last.contains(&p.attacker)));
}

#[test]
fn harpoon_pull_waits_for_the_fair_hit_and_respects_a_late_sidestep() {
    for sidestep in [false, true] {
        let (mut s, h, boss, crew, _) = fight(false);
        for g in crew { s.hurt_mob(g, h, u32::MAX); }
        tick(&mut s, 0.1);
        s.stagger_mob(boss, 0.01, false);
        s.mobs.get_mut(&boss).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        let k = s.content.mobs[s.mobs[&boss].def].patterns.iter().position(|p| p.name == "harpoon").unwrap();
        s.players.get_mut(&h).unwrap().rtt = 0.25;
        s.begin_pattern(boss, k, false);
        s.windup(boss, Foe::Hero(h), s.players[&h].pos);
        let until = s.clock + 4.0;
        while s.pending.is_empty() && s.clock < until { tick(&mut s, 0.05); }
        assert!(!s.pending.is_empty(), "the harpoon reaches the hero's fair-hit queue");
        assert!(!s.events.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == h)));
        if sidestep { s.players.get_mut(&h).unwrap().pos[1] += 3.0; }
        tick(&mut s, 0.3);
        assert_eq!(s.events.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == h)), !sidestep);
    }
}

#[test]
fn a_raised_guard_blocks_the_harpoon_without_a_pull() {
    let (mut s, h, boss, crew, _) = fight(false);
    for g in crew { s.hurt_mob(g, h, u32::MAX); }
    tick(&mut s, 0.1);
    s.stagger_mob(boss, 0.01, false);
    s.mobs.get_mut(&boss).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
    let k = s.content.mobs[s.mobs[&boss].def].patterns.iter().position(|p| p.name == "harpoon").unwrap();
    s.players.get_mut(&h).unwrap().facing = std::f32::consts::PI;
    s.input(h, Input::Guard);
    s.begin_pattern(boss, k, false);
    s.windup(boss, Foe::Hero(h), s.players[&h].pos);
    s.events.clear();
    tick(&mut s, 2.4);
    assert!(s.events.iter().any(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Blocked, .. } if *v == h)));
    assert!(!s.events.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == h)));
}

#[test]
fn gunner_posts_reject_water_and_the_wrong_floor_before_spawning() {
    struct Ship;
    impl Land for Ship {
        fn height(&self, x: f32, y: f32) -> f32 { if y > 1.0 && x > 0.0 { -16.0 } else { 0.0 } }
        fn water(&self, x: f32, y: f32) -> bool { y > 1.0 && x < 0.0 }
    }
    let (mut s, h, boss, _, _) = fight(false);
    s.reset_firing_lines();
    s.land = Box::new(Ship);
    s.engage(boss, Foe::Hero(h));
    tick(&mut s, 0.05);
    let c = &s.firing_line.crews[&boss];
    assert_eq!(c.gunners.len(), 2);
    for id in &c.gunners {
        let p = s.mobs[id].pos;
        assert_eq!(p[2], 0.0);
        assert!(!s.land.water_near(p[0], p[1], p[2]));
        assert!(s.land.walk_clear(s.mobs[&boss].home, p));
    }
}

#[test]
fn ordinary_sword_inputs_can_clear_the_crew_and_captain_solo_and_duo() {
    // Controlled pressure fixture: normal input, sword and health scaling, inflated hero HP.
    for duo in [false, true] {
        let (mut s, h, boss, crew, other) = fight(duo);
        let heroes: Vec<_> = [Some(h), other].into_iter().flatten().collect();
        while s.clock < 150.0 && s.mobs[&boss].health > 0 {
            let target = crew.iter().copied().find(|g| s.mobs.get(g).is_some_and(|m| m.health > 0)).unwrap_or(boss);
            let at = s.mobs[&target].pos;
            for hero in &heroes {
                let p = &s.players[hero];
                let face = facing(p.pos, at);
                let pos = if dist2d(p.pos, at) > 1.8 {
                    [p.pos[0] + face.cos() * 0.3, p.pos[1] + face.sin() * 0.3, p.pos[2]]
                } else { p.pos };
                let ready = matches!(p.action, Action::Ready);
                s.move_player(*hero, pos, face, Moving::default());
                if ready { s.input(*hero, Input::Attack); }
            }
            tick(&mut s, 0.05);
        }
        eprintln!("Greenskin sword fixture heroes={} seconds={:.2}", heroes.len(), s.clock);
        assert_eq!(s.mobs[&boss].health, 0);
        assert!(s.firing_line.crews.is_empty());
    }
}

#[test]
fn a_solo_vanish_keeps_the_encounter_and_its_existing_crew() {
    let (mut s, h, boss, crew, _) = fight(false);
    s.apply_cc(h, h, "veil", 20.0, 0.0, 0.0, "test");
    assert!(s.has_cc(h, "veil"));
    tick(&mut s, 9.0);
    assert_eq!(s.firing_line.crews[&boss].gunners, crew);
    assert!(s.mobs[&boss].state.foe().is_some());
    assert!(crew.iter().all(|g| !s.unseen_by(*g, h)));
}

#[test]
fn normal_charged_projectile_inputs_can_kill_a_gunner() {
    let (mut s, h, _, crew, _) = fight(false);
    let g = crew[0];
    s.slot_ability(h, "ranged", "frostbolt").unwrap();
    let key = crate::content::SLOTS.iter().position(|k| *k == "ranged").unwrap() as u8;
    for _ in 0..12 {
        if !s.mobs.get(&g).is_some_and(|m| m.health > 0) { break; }
        let at = s.mobs[&g].pos;
        let pos = [at[0] + 8.0, at[1], at[2]];
        s.move_player(h, pos, facing(pos, at), Moving::default());
        s.input(h, Input::Ability(key, false));
        s.input(h, Input::Attack);
        tick(&mut s, 1.2);
        s.input(h, Input::AttackUp);
        tick(&mut s, 2.0);
    }
    assert!(!s.mobs.get(&g).is_some_and(|m| m.health > 0), "normal ranged input must reach and kill the deck crew");
}

#[test]
fn a_gunner_keeps_normal_knockback_motion_while_his_ai_holds_the_post() {
    let (mut s, h, _, crew, _) = fight(false);
    let g = crew[0];
    let before = s.mobs[&g].pos;
    let effect = crate::content::AbilityEffect { kind: "knockback".into(), amount: 3.0, from_caster: true, ..Default::default() };
    s.displace(h, g, &effect);
    tick(&mut s, 0.4);
    assert!(dist2d(s.mobs[&g].pos, before) > 2.0);
    assert!(s.firing_line.owners.contains_key(&g));
}

#[test]
fn killing_the_captain_first_clears_his_elite_pack_and_pays_once() {
    let mut s = Survival::new_dungeon(Content::dev(), Box::new(FlatLand::default()), 31, 0.0);
    let h = s.join("boarder", "Boarder", None);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_greenskin").unwrap().0;
    let home = s.mobs[&boss].home;
    s.players.get_mut(&h).unwrap().pos = home;
    s.engage(boss, Foe::Hero(h));
    s.clean_firing_lines();
    let crew = s.firing_line.crews[&boss].gunners.clone();
    assert!(!s.slain("greenskin"));
    s.hurt_mob(boss, h, u32::MAX);
    assert!(s.slain("greenskin"));
    assert!(crew.iter().all(|g| !s.mobs.contains_key(g)));
    let bags = s.players[&h].bags.clone();
    let kills = s.camp.counts.get("kill:dm_greenskin").copied();
    s.hurt_mob(boss, h, u32::MAX);
    assert_eq!(s.players[&h].bags, bags);
    assert_eq!(s.camp.counts.get("kill:dm_greenskin").copied(), kills);
    assert_eq!(kills, Some(1));
}
