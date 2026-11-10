use super::*;
use crate::land::FlatLand;

fn fight(duo: bool) -> (Survival, Id, Id, Option<Id>) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 17, 0.0);
    let hero = s.join("sword", "Sword", None);
    let other = duo.then(|| s.join("friend", "Friend", None));
    for id in [Some(hero), other].into_iter().flatten() {
        let p = s.players.get_mut(&id).unwrap();
        p.pos = [2.5, 0.0, 0.0];
        p.health = 100_000;
        p.max_health = 100_000;
    }
    let boss = s.spawn_mob(s.content.mob("dm_smite").unwrap(), [0.0, 0.0, 0.0], None, false);
    s.engage(boss, Foe::Hero(hero));
    tick(&mut s, 0.05);
    (s, hero, boss, other)
}

fn tick(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / 0.05).round() as usize {
        s.clock += 0.05;
        s.step_players(0.05);
        s.step_mobs(0.05);
    }
}

fn hurt_to(s: &mut Survival, hero: Id, boss: Id, share: f32) {
    let target = (s.mobs[&boss].max_health as f32 * share) as u32;
    let damage = s.mobs[&boss].health.saturating_sub(target);
    s.hurt_mob(boss, hero, damage);
}

fn wait_stage(s: &mut Survival, boss: Id, stage: usize, limit: f32) {
    let until = s.clock + limit;
    while s.mobs[&boss].phase != stage && s.clock < until { tick(s, 0.05); }
    assert_eq!(s.mobs[&boss].phase, stage, "clock {}, state {:?}", s.clock, s.mobs[&boss].state);
}

#[test]
fn ai_runs_to_the_rack_and_equips_in_order_solo_and_duo() {
    for duo in [false, true] {
        let (mut s, h, boss, _) = fight(duo);
        let sword = s.arsenal_loadout(boss).unwrap().weapon;
        hurt_to(&mut s, h, boss, 0.25); // Cross both lines in one ordinary damaging hit.
        wait_stage(&mut s, boss, 1, 22.0);
        let axe_at = s.clock;
        let axes = s.arsenal_loadout(boss).unwrap();
        assert_ne!(axes.weapon, sword);
        assert_eq!(axes.off_hand, axes.weapon);
        assert!(dist2d(s.mobs[&boss].pos, s.arsenal.racks[&boss].at) < 1.8);
        assert!(s.events.iter().any(|e| matches!(e, Event::MobMove { id, path, .. } if *id == boss && path.len() >= 2)));
        tick(&mut s, 4.0);
        assert_eq!(s.mobs[&boss].phase, 1, "burst damage cannot skip the axe showing");
        wait_stage(&mut s, boss, 2, 24.0);
        assert!(s.clock >= axe_at + AXE_SHOWING);
        assert_eq!(s.arsenal_loadout(boss).unwrap().off_hand, 0);
        assert_eq!(s.arsenal_loadout(boss).unwrap().grip, "hammer");
        // Each change: hands emptied as he kneels, the new weapon up as he stands.
        assert_eq!(s.events.iter().filter(|e| matches!(e, Event::MobGear { id } if *id == boss)).count(), 4);
    }
}

#[test]
fn damage_waits_for_a_committed_string_and_its_recovery() {
    let (mut s, h, boss, _) = fight(false);
    let k = s.content.mobs[s.mobs[&boss].def].patterns.iter().position(|p| p.name == "delayed overhead").unwrap();
    s.begin_pattern(boss, k, false);
    s.windup(boss, Foe::Hero(h), s.players[&h].pos);
    // The threshold comes from a low posture source; it must not erase an already committed tell.
    s.mobs.get_mut(&boss).unwrap().health = 730;
    s.hurt_mob(boss, h, 10);
    tick(&mut s, 0.5);
    assert!(matches!(s.mobs[&boss].state, MobState::Windup { .. }));
    assert!(s.arsenal.racks[&boss].transition.is_none());
    tick(&mut s, 1.1);
    assert!(matches!(s.mobs[&boss].state, MobState::Recover { .. }));
    assert!(s.arsenal.racks[&boss].transition.is_none());
    wait_stage(&mut s, boss, 1, 20.0);
}

#[test]
fn ordinary_hits_pause_the_run_and_posture_cannot_hold_it_forever() {
    let (mut s, h, boss, _) = fight(false);
    s.mobs.get_mut(&boss).unwrap().health = 730;
    s.hurt_mob(boss, h, 10);
    while s.arsenal.racks[&boss].transition.is_none() { tick(&mut s, 0.05); }
    tick(&mut s, 1.2);
    s.hurt_mob(boss, h, 1);
    let at = s.mobs[&boss].pos;
    tick(&mut s, 0.2);
    assert!(dist2d(at, s.mobs[&boss].pos) < 0.4, "an ordinary hit stops the chase for a half beat");
    let began = s.arsenal.racks[&boss].transition.unwrap().since;
    let limit = s.posture_limit(boss);
    assert!(s.build_posture(boss, h, limit));
    assert!(s.stance_broken(boss));
    while s.mobs[&boss].phase == 0 && s.clock < began + CHANGE_LIMIT + 1.0 {
        // Deliberate repeated stuns are stronger than normal posture grace permits.
        s.stagger_mob(boss, 2.0, true);
        tick(&mut s, 0.2);
    }
    assert_eq!(s.mobs[&boss].phase, 1);
    assert!(s.clock <= began + CHANGE_LIMIT + 0.3);
}

#[test]
fn each_equipped_stage_drives_real_ai_attacks_with_its_own_recovery() {
    let (mut s, h, boss, _) = fight(false);
    for stage in 0..3 {
        if stage > 0 {
            hurt_to(&mut s, h, boss, if stage == 1 { 0.6 } else { 0.3 });
            wait_stage(&mut s, boss, stage, 25.0);
        }
        s.events.clear();
        let mut observed = BTreeSet::new();
        for _ in 0..1200 {
            // A pursuing ordinary hero stays within reach, so the AI has real exchanges.
            let at = s.mobs[&boss].pos;
            s.players.get_mut(&h).unwrap().pos = [at[0] + 2.2, at[1], at[2]];
            tick(&mut s, 0.05);
            if let Some((k, _)) = s.mobs[&boss].pattern {
                let name = &s.content.mobs[s.mobs[&boss].def].patterns[k].name;
                assert!(s.arsenal_pattern_allowed(boss, name));
                observed.insert(name.clone());
            }
        }
        assert_eq!(observed.len(), 2, "both authored attacks used at stage {stage}");
        assert!(s.events.iter().any(|e| matches!(e, Event::Swing { attacker, .. } if *attacker == boss)));
        let tells: Vec<_> = s.events.iter().filter_map(|e| match e {
            Event::MobWindup { id, windup, recovery, blockable, .. } if *id == boss => Some((*windup, *recovery, *blockable)), _ => None,
        }).collect();
        assert!(!tells.is_empty());
        if stage == 2 {
            assert!(tells.iter().all(|(w, r, b)| *w >= 1.35 && *r >= 2.1 && !b));
        }
    }
}

#[test]
fn rack_changes_never_damage_or_disable_a_hero() {
    let (mut s, h, boss, _) = fight(false);
    hurt_to(&mut s, h, boss, 0.6);
    while s.arsenal.racks[&boss].transition.is_none() { tick(&mut s, 0.05); }
    s.events.clear();
    let health = s.players[&h].health;
    wait_stage(&mut s, boss, 1, 18.0);
    assert_eq!(s.players[&h].health, health);
    assert!(!s.events.iter().any(|e| matches!(e, Event::Swing { attacker, .. } if *attacker == boss)));
    assert!(!s.disabled(h));
}

struct NoRoute;
impl Land for NoRoute {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
    fn plans(&self) -> bool { true }
}

#[test]
fn a_route_blocked_after_the_pull_times_out_without_warping() {
    let (mut s, h, boss, _) = fight(false);
    s.land = Box::new(NoRoute);
    let at = s.mobs[&boss].pos;
    hurt_to(&mut s, h, boss, 0.6);
    wait_stage(&mut s, boss, 1, 21.0);
    assert_eq!(s.mobs[&boss].pos, at);
    assert!(s.events.iter().any(|e| matches!(e, Event::Log(text) if text.contains("rack route timed out"))));
}

#[test]
fn one_hero_dying_during_the_change_keeps_the_other_in_the_fight() {
    let (mut s, h, boss, other) = fight(true);
    hurt_to(&mut s, h, boss, 0.6);
    while s.arsenal.racks[&boss].transition.is_none() { tick(&mut s, 0.05); }
    s.players.get_mut(&h).unwrap().health = 0;
    wait_stage(&mut s, boss, 1, 18.0);
    assert_eq!(s.mobs[&boss].state.target(), other);
}

#[test]
fn wipe_and_repull_restore_the_sword_without_duplicate_racks() {
    let (mut s, h, boss, _) = fight(false);
    hurt_to(&mut s, h, boss, 0.6);
    wait_stage(&mut s, boss, 1, 20.0);
    let old = s.arsenal.racks[&boss].prop;
    s.players.get_mut(&h).unwrap().health = 0;
    tick(&mut s, 5.0);
    assert!(!s.objs.contains_key(&old));
    assert_eq!(s.mobs[&boss].phase, 0);
    s.players.get_mut(&h).unwrap().health = 100_000;
    s.engage(boss, Foe::Hero(h));
    tick(&mut s, 0.1);
    assert_eq!(s.arsenal.racks.len(), 1);
    assert_eq!(s.arsenal_loadout(boss).unwrap().grip, "sword");
    let rack = s.arsenal.racks[&boss].prop;
    s.hurt_mob(boss, h, u32::MAX);
    assert!(!s.objs.contains_key(&rack));
    assert!(s.arsenal.racks.is_empty());
}

#[test]
fn explicit_dungeon_reset_clears_the_pending_change() {
    let (mut s, h, boss, _) = fight(false);
    hurt_to(&mut s, h, boss, 0.6);
    while s.arsenal.racks[&boss].transition.is_none() { tick(&mut s, 0.05); }
    let rack = s.arsenal.racks[&boss].prop;
    s.reset_arsenals();
    assert!(!s.objs.contains_key(&rack));
    assert!(s.arsenal.racks.is_empty());
    assert_eq!(s.mobs[&boss].phase, 0);
    assert!(matches!(s.mobs[&boss].state, MobState::Idle { .. }));
}

#[test]
fn ordinary_melee_inputs_complete_scaled_solo_and_duo_fights() {
    // A pressure fixture, not a balance proof: normal starting sword/input/actions and party
    // health scaling, but inflated hero HP lets us measure the cost without stopping at death.
    for duo in [false, true] {
        let (mut s, h, boss, other) = fight(duo);
        let heroes: Vec<_> = [Some(h), other].into_iter().flatten().collect();
        let mut stages = BTreeMap::from([(0usize, 0.0f32)]);
        while s.clock < 180.0 && s.mobs[&boss].health > 0 {
            let at = s.mobs[&boss].pos;
            for (n, hero) in heroes.iter().enumerate() {
                let p = &s.players[hero];
                let face = facing(p.pos, at);
                let pos = if dist2d(p.pos, at) > 2.0 {
                    [p.pos[0] + face.cos() * 0.35, p.pos[1] + face.sin() * 0.35, p.pos[2]]
                } else { p.pos };
                let ready = matches!(p.action, Action::Ready);
                s.move_player(*hero, pos, face, Moving::default());
                if ready && (n == 0 || s.clock > 0.3) { s.input(*hero, Input::Attack); }
            }
            tick(&mut s, 0.05);
            stages.entry(s.mobs[&boss].phase).or_insert(s.clock);
        }
        let damage: u32 = heroes.iter().map(|h| 100_000 - s.players[h].health).sum();
        eprintln!("Smite pressure fixture heroes={} secs={:.2} damage_taken={} stages={:?}", heroes.len(), s.clock, damage, stages);
        assert_eq!(s.mobs[&boss].health, 0, "ordinary melee must finish the encounter");
        // Relentless tanking can kill him during a change. No invulnerability is introduced
        // just to force the last weapon; every completed change must still be ordered.
        let seen: Vec<_> = stages.keys().copied().collect();
        // (A duo pressing him at the rack, where he kneels open, can fell him there: Nov 9.)
        assert!([vec![0], vec![0, 1], vec![0, 1, 2]].contains(&seen), "{seen:?}");
        assert!(s.arsenal.racks.is_empty());
    }
}

fn lying(s: &Survival, display: u32) -> bool {
    s.objs.values().any(|o| matches!(o.kind, ObjKind::Prop { display: d, .. } if d == display))
}

#[test]
fn he_kneels_empty_handed_at_the_rack_and_stands_with_the_new_weapon_roaring() {
    let (mut s, h, boss, _) = fight(false);
    assert!(lying(&s, HAMMER_WAITING), "the hammer waits at the rack from the pull");
    assert!(!lying(&s, SWORD_LEFT));
    hurt_to(&mut s, h, boss, 0.6);
    let until = s.clock + 20.0;
    while !s.arsenal_hands_empty(boss) && s.clock < until { tick(&mut s, 0.05); }
    assert!(s.arsenal_hands_empty(boss), "his hands empty at the rack");
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: KNEEL_CLIP, secs, .. } if *id == boss && *secs >= FIT_WEAPON)), "down on one knee");
    assert!(lying(&s, SWORD_LEFT), "the sword put down at the rack");
    assert_eq!(s.arsenal_posture_share(boss), KNEEL_POSTURE, "open while he kneels");
    s.events.clear();
    wait_stage(&mut s, boss, 1, 10.0);
    assert!(!s.arsenal_hands_empty(boss));
    assert_eq!(s.arsenal_posture_share(boss), 1.0);
    assert!(s.events.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == boss && *emote == super::mobs::ROAR_EMOTE)), "he roars as he stands");
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: 0, .. } if *id == boss)), "and lets go of the kneel");
    hurt_to(&mut s, h, boss, 0.3);
    wait_stage(&mut s, boss, 2, 30.0);
    assert!(!lying(&s, HAMMER_WAITING), "the hammer taken up");
    assert!(lying(&s, SWORD_LEFT), "the sword still lies there");
    let health = s.mobs[&boss].health;
    s.hurt_mob(boss, h, health);
    assert!(!lying(&s, SWORD_LEFT) && !lying(&s, HAMMER_WAITING), "the rack and what lies there go with him");
}

#[test]
fn hits_while_he_kneels_dont_keep_him_at_the_rack() {
    let (mut s, h, boss, _) = fight(false);
    hurt_to(&mut s, h, boss, 0.6);
    let until = s.clock + 20.0;
    while !s.arsenal_hands_empty(boss) && s.clock < until { tick(&mut s, 0.05); }
    assert!(s.arsenal_hands_empty(boss));
    let knelt = s.clock;
    let mut next = s.clock;
    while s.arsenal_hands_empty(boss) && s.clock < knelt + 6.0 {
        if s.clock >= next {
            s.hurt_mob(boss, h, 5);
            next = s.clock + 0.3;
        }
        tick(&mut s, 0.05);
    }
    assert!(!s.arsenal_hands_empty(boss), "up with the new weapon");
    assert!(s.clock <= knelt + FIT_WEAPON + 0.2, "hits held him at the rack until {:.2} s after he knelt", s.clock - knelt);
}
