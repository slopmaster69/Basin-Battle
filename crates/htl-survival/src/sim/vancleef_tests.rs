use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

fn fight(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 7, 0.0);
    s.content.pvp = Default::default();
    let mut band = Vec::new();
    for n in 0..heroes {
        let h = s.join(&format!("hero{n}"), &format!("Hero{n}"), None);
        s.move_player(h, [2.0, n as f32, 0.0], std::f32::consts::PI, Moving::default());
        let p = s.players.get_mut(&h).unwrap();
        p.health = 100_000;
        p.max_health = 100_000;
        band.push(h);
    }
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
    s.engage(id, Foe::Hero(band[0]));
    s.clean_duels();
    (s, band, id)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

fn invitation(s: &mut Survival, id: Id, hero: Id) {
    let k = s.duel_pattern_named(id, "invitation").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(hero), s.players[&hero].pos);
    s.events.clear();
}

fn shot(s: &mut Survival, id: Id, hero: Id, damage: f32, how: HitKind) {
    let ability = s.content.ability("fireball").unwrap();
    s.content.abilities[ability].effects.clear();
    s.ability_on_mob(hero, id, ability, 1.0, damage, s.players[&hero].pos, how, 133);
}

fn windups(events: &[Event], id: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)).count()
}

#[test]
fn actual_ai_opens_with_exactly_two_cuts_and_keeps_the_whole_recovery() {
    for heroes in [1, 2, 4] {
        let (mut s, _, id) = fight("dm_vancleef", heroes);
        s.content.ai.cut = 1.0;
        s.content.ai.extend = 1.0;
        s.content.ai.chain = 1.0;
        s.content.ai.hold = 1.0;
        let mut count = 0;
        for _ in 0..300 {
            let ev = s.step(DT);
            count += windups(&ev, id);
            if s.vancleef.duels[&id].invite_next {
                assert_eq!(count, 2);
                assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock >= 1.75));
                assert_eq!(windups(&run(&mut s, 1.6), id), 0);
                break;
            }
        }
        assert_eq!(count, 2);
        run(&mut s, 3.0);
        assert!(s.vancleef.duels[&id].invited, "the completed exchange leads to Invitation");
    }
}

#[test]
fn waiting_out_invitation_never_attacks_and_leaves_an_open_recovery() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    assert!(s.mobs[&id].guarding);
    let health = s.players[&band[0]].health;
    let ev = run(&mut s, INVITATION + 0.2);
    assert_eq!(windups(&ev, id), 0);
    assert_eq!(s.players[&band[0]].health, health);
    assert!(!s.mobs[&id].guarding);
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until > s.clock + 0.8));
}

#[test]
fn a_direct_frontal_hit_provokes_once_toward_that_attacker_and_can_be_sidestepped() {
    let (mut s, band, id) = fight("dm_vancleef", 2);
    invitation(&mut s, id, band[0]);
    let before = s.mobs[&id].health;
    shot(&mut s, id, band[1], 1.0, HitKind::Projectile);
    assert!(s.mobs[&id].health < before, "no invulnerability");
    assert!(matches!(s.mobs[&id].state, MobState::Windup { foe: Foe::Hero(h), t: 0.0, .. } if h == band[1]));
    assert_eq!(windups(&s.events, id), 1);
    let dir = s.mobs[&id].facing;
    shot(&mut s, id, band[0], 1.0, HitKind::Projectile);
    assert_eq!(windups(&s.events, id), 1, "second hero cannot restart the response");
    for h in &band { s.move_player(*h, [-2.0, 4.0, 0.0], 0.0, Moving::default()); }
    let health: Vec<_> = band.iter().map(|h| s.players[h].health).collect();
    run(&mut s, 1.2);
    assert!((s.mobs[&id].facing - dir).abs() < 0.01, "direction committed at contact: {} -> {}", dir, s.mobs[&id].facing);
    assert_eq!(band.iter().map(|h| s.players[h].health).collect::<Vec<_>>(), health);
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until > s.clock + 1.9));
}

#[test]
fn rear_hits_area_damage_periodic_damage_and_zero_damage_do_not_provoke() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    shot(&mut s, id, band[0], 0.0, HitKind::Projectile);
    shot(&mut s, id, band[0], 1.0, HitKind::Area);
    s.hurt_mob_as(id, band[0], 1, "Burn");
    s.move_player(band[0], [-2.0, 0.0, 0.0], 0.0, Moving::default());
    shot(&mut s, id, band[0], 1.0, HitKind::Projectile);
    assert_eq!(windups(&s.events, id), 0);
    assert!(matches!(s.vancleef.duels[&id].beat, Beat::Invitation { .. }));
}

#[test]
fn a_real_weapon_swing_and_a_bursting_projectiles_direct_contact_both_provoke() {
    for projectile in [false, true] {
        let (mut s, band, id) = fight("dm_vancleef", 1);
        invitation(&mut s, id, band[0]);
        if projectile {
            s.move_player(band[0], [6.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
            let ability = s.content.ability("fireball").unwrap();
            s.content.abilities[ability].effects.clear();
            s.launch(band[0], ability, 0.0);
        } else { s.input(band[0], Input::Attack); }
        let ev = run(&mut s, 0.8);
        assert_eq!(windups(&ev, id), 1, "direct contact, projectile={projectile}");
        assert_eq!(s.mobs[&id].pattern.map(|(k, _)| k), s.duel_pattern_named(id, "riposte"));
    }
}

#[test]
fn an_off_hand_weapon_uses_the_same_invitation_contact_path() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let hero = band[0];
    s.players.get_mut(&hero).unwrap().weapon = s.content.item("sword").unwrap();
    s.players.get_mut(&hero).unwrap().off_weapon = s.content.item("axe");
    invitation(&mut s, id, hero);
    s.input(hero, Input::Guard);
    assert!(matches!(&s.players[&hero].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Off));
    let ev = run(&mut s, 0.8);
    assert_eq!(windups(&ev, id), 1);
    assert_eq!(s.mobs[&id].pattern.map(|(k, _)| k), s.duel_pattern_named(id, "riposte"));
}

#[test]
fn solo_vanish_does_not_reset_the_duel_or_replace_its_observers() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    let watchers = s.vancleef.duels[&id].watchers.clone();
    assert!(s.apply_cc(band[0], band[0], "veil", 4.0, 0.0, 0.0, "Vanish"));
    assert!(s.unseen(band[0]));
    assert!(s.next_in_fight(id).is_none());
    run(&mut s, 2.1);
    assert_eq!(s.mobs[&id].state.foe(), Some(Foe::Hero(band[0])));
    assert!(s.vancleef.duels[&id].invited);
    assert_eq!(s.vancleef.duels[&id].watchers, watchers);
}

#[test]
fn control_and_a_posture_break_take_precedence_over_invitation() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    let ability = s.content.ability("fireball").unwrap();
    s.content.abilities[ability].effects = vec![crate::content::AbilityEffect { kind: "stun".into(), secs: [2.0, 2.0], vs_bosses: true, ..Default::default() }];
    s.ability_on_mob(band[0], id, ability, 1.0, 1.0, s.players[&band[0]].pos, HitKind::Projectile, 133);
    assert_eq!(windups(&s.events, id), 0);
    run(&mut s, 0.1);
    assert!(!s.mobs[&id].guarding);
    // A posture break from any damage path preserves the complete reward first.
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    shot(&mut s, id, band[0], 320.0, HitKind::Projectile);
    assert!(matches!(s.vancleef.duels[&id].beat, Beat::Broken));
    assert_eq!(windups(&s.events, id), 0);
    let at = s.mobs[&id].pos;
    run(&mut s, 1.0);
    assert_eq!(s.mobs[&id].pos, at);
    assert!(s.stance_broken(id));
}

#[test]
fn posture_retreat_is_walked_vulnerable_and_smoke_clears_before_advance() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let limit = s.posture_limit(id);
    assert!(s.build_posture(id, band[0], limit));
    let start = s.mobs[&id].pos;
    for _ in 0..150 {
        s.step(DT);
        if matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }) { break; }
    }
    assert!(matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }));
    let before = s.mobs[&id].health;
    s.hurt_mob(id, band[0], 5);
    assert_eq!(s.mobs[&id].health, before - 5);
    run(&mut s, 0.3);
    let moved = dist2d(start, s.mobs[&id].pos);
    assert!(moved > 0.5 && moved < 2.0, "continuous walk: {moved}");
    let mut saw_arrival = false;
    for _ in 0..100 {
        let ev = s.step(DT);
        if matches!(s.vancleef.duels[&id].beat, Beat::Arrival { .. }) {
            saw_arrival = true;
            assert!(s.vancleef.duels[&id].smoke.is_none());
            assert_eq!(windups(&ev, id), 0);
        }
        if windups(&ev, id) > 0 {
            assert!(saw_arrival);
            assert_eq!(s.mobs[&id].pattern.map(|(k, _)| k), s.duel_pattern_named(id, "advance"));
            return;
        }
    }
    panic!("advance never arrived");
}

struct NoPath;
impl Land for NoPath {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
    fn plans(&self) -> bool { true }
}

#[test]
fn unreachable_retreat_stays_in_place_and_still_gives_the_full_arrival_cue() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    s.land = Box::new(NoPath);
    s.vancleef_broken(id);
    let start = s.mobs[&id].pos;
    assert!(s.step_vancleef(id));
    assert!(matches!(s.vancleef.duels[&id].beat, Beat::Arrival { until } if until >= s.clock + ARRIVAL));
    assert_eq!(s.mobs[&id].pos, start);
    assert!(s.mobs[&id].path.is_empty());
    s.clock += ARRIVAL + 0.01;
    s.step_vancleef(id);
    assert!(matches!(s.mobs[&id].state, MobState::Windup { foe: Foe::Hero(h), .. } if h == band[0]));
}

#[test]
fn spectators_are_finite_noncombat_and_leave_once_without_stopping_an_attack() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let watchers = s.vancleef.duels[&id].watchers.clone();
    assert_eq!(watchers.len(), 2);
    for (w, _) in &watchers {
        s.hurt_mob(*w, band[0], 100);
        s.engage(*w, Foe::Hero(band[0]));
        assert_eq!(s.mobs[w].health, 1);
        assert!(s.mobs[w].state.foe().is_none());
        assert!(!s.takes_help(band[0], *w), "no heal interception or ally-target slot");
    }
    let k = s.duel_pattern_named(id, "string").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(band[0]), s.players[&band[0]].pos);
    s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
    s.step_vancleef(id);
    assert!(matches!(s.mobs[&id].state, MobState::Windup { .. }));
    assert!(watchers.iter().all(|(w, _)| !s.mobs[w].path.is_empty()));
    let first = s.mobs[&watchers[0].0].pos;
    run(&mut s, 1.0);
    assert!(dist2d(first, s.mobs[&watchers[0].0].pos) > 1.8);
    run(&mut s, 6.0);
    assert!(watchers.iter().all(|(w, exit)| dist2d(s.mobs[w].pos, *exit) < 0.1), "both walks finish before cleanup");
    run(&mut s, 3.0);
    assert!(watchers.iter().all(|(w, _)| !s.mobs.contains_key(w)));
    assert!(s.vancleef.duels[&id].departed);
    assert!(s.vancleef.observers.is_empty());
}

#[test]
fn unsupported_observer_departures_stay_absent_on_custom_land() {
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.remove_duel(id);
    s.land = Box::new(NoPath);
    s.clean_duels();
    assert!(s.vancleef.duels[&id].watchers.is_empty());
    assert!(s.vancleef.observers.is_empty());
    s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
    s.step_vancleef(id);
    run(&mut s, 9.0);
    assert!(!s.mobs.keys().any(|w| s.duel_observer(*w)), "no replacement combat or stranded actors");
}

#[test]
fn last_stand_waits_for_recovery_and_occurs_once_with_no_immunity_or_speedup() {
    for key in ["dm_vancleef", "dm_vancleef_unbowed"] {
        let (mut s, band, id) = fight(key, 1);
        s.mobs.get_mut(&id).unwrap().state = MobState::Recover { foe: Foe::Hero(band[0]), until: 2.0 };
        s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health / 10;
        assert!(!s.step_vancleef(id));
        assert!(!s.vancleef.duels[&id].final_done);
        s.clock = 2.0;
        assert!(s.step_vancleef(id));
        assert!(s.vancleef.duels[&id].final_done);
        assert_eq!(s.mobs[&id].pattern.map(|(k, _)| k), s.duel_pattern_named(id, "last stand"));
        s.hurt_mob(id, band[0], 100_000);
        assert_eq!(s.mobs[&id].health, 0, "can be killed during the dramatic combo");
        assert!(!s.vancleef.duels.contains_key(&id));
    }
}

#[test]
fn reset_wipe_and_death_clear_every_encounter_beat_and_observer() {
    for beat in [Beat::Invitation { until: 8.0 }, Beat::Broken, Beat::Retreat { until: 8.0 }, Beat::Arrival { until: 8.0 }] {
        for death in [false, true] {
            let (mut s, band, id) = fight("dm_vancleef", 1);
            s.vancleef.duels.get_mut(&id).unwrap().beat = beat;
            let watchers = s.vancleef.duels[&id].watchers.clone();
            if death { s.hurt_mob(id, band[0], 100_000); }
            else {
                s.players.get_mut(&band[0]).unwrap().health = 0;
                s.step_vancleef(id);
                assert!(matches!(s.mobs[&id].state, MobState::Return));
            }
            assert!(!s.vancleef.duels.contains_key(&id));
            assert!(s.vancleef.observers.is_empty());
            assert!(watchers.iter().all(|(w, _)| !s.mobs.contains_key(w)));
            s.reset_duels();
            s.remove_duel(id); // idempotent
        }
    }
}

#[test]
fn a_rooted_retreat_times_out_without_a_teleport_and_reset_clears_smoke_and_pose() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    s.vancleef_broken(id);
    s.step_vancleef(id);
    let at = s.mobs[&id].pos;
    let smoke = s.vancleef.duels[&id].smoke.unwrap().0;
    let root = crate::content::AbilityEffect { kind: "root".into(), secs: [10.0, 10.0], vs_bosses: true, ..Default::default() };
    s.apply_effects(band[0], id, &[root], 1.0, "Roots");
    run(&mut s, RETREAT_LIMIT + 0.1);
    assert_eq!(s.mobs[&id].pos, at);
    assert!(!s.objs.contains_key(&smoke));
    assert!(!matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }));
    s.reset_duels();
    assert!(s.vancleef.observers.is_empty());
    assert!(s.vancleef.duels.is_empty());
    assert!(!s.mobs[&id].guarding);
    assert!(s.mobs[&id].pattern.is_none());
    assert!(s.mobs[&id].state.foe().is_none());
}

#[test]
fn the_actual_secret_handoff_has_one_unbowed_and_spectators_never_gate_pack_clear() {
    for secret in [false, true] {
        let mut c = Content::dev();
        let d = c.dungeon.as_mut().unwrap();
        d.packs.retain(|p| p.key == "vancleef");
        d.packs[0].at = [0.0, 0.0];
        d.packs[0].z = Some(0.0);
        d.lamps[0].at = [0.0, -20.0, 0.0];
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 1, 0.0);
        if secret { s.below.as_mut().unwrap().open.insert("vancleef_quarters".into()); }
        let hero = s.join("hero", "Hero", None);
        s.move_player(hero, [2.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
        let id = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_vancleef").unwrap().0;
        s.engage(id, Foe::Hero(hero));
        s.clean_duels();
        assert_eq!(s.vancleef.duels[&id].watchers.len(), 2);
        for (w, _) in &s.vancleef.duels[&id].watchers {
            assert!(s.mobs[w].pack.is_none() && s.mobs[w].summoned_by.is_none());
            let d = &s.content.mobs[s.mobs[w].def];
            assert!(d.drops.is_empty() && d.trophy.is_empty() && d.gear.is_empty());
        }
        s.hurt_mob(id, hero, 100_000);
        assert!(s.vancleef.observers.is_empty());
        assert_eq!(s.below.as_ref().unwrap().slain.contains("vancleef"), !secret);
        run(&mut s, 4.2);
        let forms: Vec<_> = s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "dm_vancleef_unbowed").map(|(id, _)| *id).collect();
        assert_eq!(forms.len(), usize::from(secret));
        if let Some(form) = forms.first() {
            assert!(s.vancleef.duels[form].watchers.is_empty());
            s.hurt_mob(*form, hero, 100_000);
            assert!(s.below.as_ref().unwrap().slain.contains("vancleef"));
            run(&mut s, 5.0);
            assert!(!s.mobs.values().any(|m| m.health > 0 && s.content.mobs[m.def].key.contains("vancleef")));
        }
    }
}

#[test]
fn unbowed_has_longer_fixed_strings_larger_recoveries_and_no_duplicate_or_summon() {
    let (mut s, _, id) = fight("dm_vancleef_unbowed", 2);
    assert!(s.vancleef.duels[&id].watchers.is_empty());
    let normal = &s.content.mobs[s.content.mob("dm_vancleef").unwrap()];
    let second = &s.content.mobs[s.mobs[&id].def];
    assert!(normal.phases.is_empty() && second.phases.is_empty());
    for name in ["string", "advance", "last stand"] {
        let a = normal.patterns.iter().find(|p| p.name == name).unwrap();
        let b = second.patterns.iter().find(|p| p.name == name).unwrap();
        assert!(b.steps.len() > a.steps.len());
        assert!(b.steps.last().unwrap().recovery > a.steps.last().unwrap().recovery);
        for st in &b.steps { assert!(st.windup.unwrap() >= 0.65); }
    }
    s.content.ai.cut = 1.0;
    s.content.ai.extend = 1.0;
    let k = s.duel_pattern_named(id, "string").unwrap();
    s.begin_pattern(id, k, true);
    assert_eq!(s.mobs[&id].string_len, 0);
    assert_eq!(s.mobs[&id].held_step, None);
}
