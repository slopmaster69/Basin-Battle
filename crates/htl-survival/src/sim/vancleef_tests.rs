use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

/// A duel already on the deck: he has stepped out (`entered`), so a test starts at its beat.
fn fight(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
    let (mut s, band, id) = pulled(key, heroes);
    s.vancleef.duels.get_mut(&id).unwrap().entered = true;
    (s, band, id)
}

/// A fresh pull: he still has to step out of his cabin.
fn pulled(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
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

/// The open look shown on him: its clip and how long (0: let go).
fn poses(events: &[Event], id: Id) -> Vec<(u16, f32)> {
    events.iter().filter_map(|e| match e { Event::HoldPose { id: who, clip, secs, .. } if *who == id => Some((*clip, *secs)), _ => None }).collect()
}

fn yells(events: &[Event], id: Id) -> Vec<String> {
    events.iter().filter_map(|e| match e { Event::Yell { id: who, text } if *who == id => Some(text.clone()), _ => None }).collect()
}

#[test]
fn the_pull_steps_out_of_the_cabin_to_the_lit_deck_before_any_blow() {
    let (mut s, band, id) = pulled("dm_vancleef", 1);
    s.move_player(band[0], [12.0, 0.5, 0.0], std::f32::consts::PI, Moving::default());
    let home = s.mobs[&id].home;
    let mut steps = 0;
    for _ in 0..200 {
        let ev = s.step(DT);
        if windups(&ev, id) > 0 { break; }
        steps += 1;
    }
    let walked = dist2d(home, s.mobs[&id].pos);
    assert!(walked > 6.0, "he stepped out onto the deck: {walked}");
    assert!(steps as f32 * DT > 2.0, "no blow while he walks out");
}

#[test]
fn actual_ai_opens_with_exactly_two_cuts_then_a_hunch_that_is_open() {
    for heroes in [1, 2, 4] {
        let (mut s, _, id) = fight("dm_vancleef", heroes);
        s.content.ai.cut = 1.0;
        s.content.ai.extend = 1.0;
        s.content.ai.chain = 1.0;
        s.content.ai.hold = 1.0;
        let mut count = 0;
        let mut hunched = false;
        for _ in 0..300 {
            let ev = s.step(DT);
            count += windups(&ev, id);
            if poses(&ev, id).iter().any(|(clip, secs)| *clip == HUNCH_CLIP && (*secs - 1.8).abs() < 0.06) { hunched = true; }
            if s.vancleef.duels[&id].invite_next {
                assert_eq!(count, 2);
                assert!(hunched, "the honest recovery is a hunch");
                assert!(s.vancleef_open(id));
                assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock >= 1.75));
                assert_eq!(windups(&run(&mut s, 1.6), id), 0);
                break;
            }
        }
        assert_eq!(count, 2);
        // (A lunge may close the distance first without spending it.)
        run(&mut s, 8.0);
        assert!(s.vancleef.duels[&id].invited, "the completed exchange leads to Invitation");
    }
}

#[test]
fn hits_in_his_hunch_count_extra_toward_posture_and_debug_damage_never_does() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let hero = band[0];
    s.hurt_mob(id, hero, 20);
    let plain = s.posture_of(id);
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.mobs.get_mut(&id).unwrap().state = MobState::Recover { foe: Foe::Hero(hero), until: s.clock + 1.8 };
    s.duel_open(id, 1.8, false);
    s.hurt_mob(id, hero, 20);
    let open = s.posture_of(id);
    assert!(open as f32 >= plain as f32 * 2.9, "hunch {open} vs plain {plain}");
    // A debug health change never builds posture or breaks him.
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.dev_hurt(hero, 0.35, "vancleef").unwrap();
    assert_eq!(s.posture_of(id), 0);
    assert!(!s.stance_broken(id));
}

#[test]
fn waiting_out_invitation_never_attacks_and_leaves_a_short_hunch() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation(&mut s, id, band[0]);
    assert!(s.mobs[&id].guarding);
    let health = s.players[&band[0]].health;
    let ev = run(&mut s, INVITATION + 0.2);
    assert_eq!(windups(&ev, id), 0);
    assert_eq!(s.players[&band[0]].health, health);
    assert!(!s.mobs[&id].guarding);
    assert!(poses(&ev, id).contains(&(HUNCH_CLIP, WAITED_HUNCH)));
    assert!(s.vancleef_open(id));
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until > s.clock + 0.8));
}

#[test]
fn the_invitation_scrapes_steel_and_says_its_line_only_now_and_then() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let k = s.duel_pattern_named(id, "invitation").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(band[0]), s.players[&band[0]].pos);
    assert!(s.events.iter().any(|e| matches!(e, Event::Sound { id: who, sound: 700 } if *who == id)));
    assert_eq!(yells(&s.events, id), vec!["Go on, then.".to_string()]);
    run(&mut s, 3.5);
    s.events.clear();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(band[0]), s.players[&band[0]].pos);
    assert!(yells(&s.events, id).is_empty(), "not every Invitation");
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
    assert!(s.events.iter().any(|e| matches!(e, Event::Glow { id: who, .. } if *who == id)), "his weapon glows: the riposte's own tell");
    assert!(s.events.iter().any(|e| matches!(e, Event::MobWindup { id: who, clip: 85, blockable: false, .. } if *who == id)), "a thrust, not the cuts' raised arm");
    let dir = s.mobs[&id].facing;
    shot(&mut s, id, band[0], 1.0, HitKind::Projectile);
    assert_eq!(windups(&s.events, id), 1, "second hero cannot restart the response");
    for h in &band { s.move_player(*h, [-2.0, 4.0, 0.0], 0.0, Moving::default()); }
    let health: Vec<_> = band.iter().map(|h| s.players[h].health).collect();
    run(&mut s, 1.2);
    assert!((s.mobs[&id].facing - dir).abs() < 0.01, "direction committed at contact: {} -> {}", dir, s.mobs[&id].facing);
    assert_eq!(band.iter().map(|h| s.players[h].health).collect::<Vec<_>>(), health);
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until > s.clock + 1.9));
    assert!(s.vancleef_open(id), "and he hunches after it");
}

#[test]
fn the_riposte_lunges_past_where_the_hero_stood_a_sidestep_leaves_them_behind_him() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let hero = band[0];
    invitation(&mut s, id, hero);
    let stood = s.players[&hero].pos;
    shot(&mut s, id, hero, 1.0, HitKind::Projectile);
    // Sidestep two yards.
    s.move_player(hero, [stood[0], stood[1] + 2.2, stood[2]], std::f32::consts::PI, Moving::default());
    let health = s.players[&hero].health;
    run(&mut s, 1.0);
    let m = &s.mobs[&id];
    assert!(dist2d([0.0; 3], m.pos) > dist2d([0.0; 3], stood) + 1.0, "past the spot: {:?}", m.pos);
    assert_eq!(s.players[&hero].health, health);
    assert!(s.behind(id, hero) || angle_between(m.facing, facing(m.pos, s.players[&hero].pos)) > 90.0, "the hero is behind him");
}

#[test]
fn standing_still_in_the_riposte_hurts_but_never_kills() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let hero = band[0];
    let p = s.players.get_mut(&hero).unwrap();
    p.max_health = 200;
    p.health = 30;
    invitation(&mut s, id, hero);
    shot(&mut s, id, hero, 1.0, HitKind::Projectile);
    run(&mut s, 1.2);
    let h = s.players[&hero].health;
    assert!(h > 0 && h < 30, "hurt, alive: {h}");
    // With room to spare, the whole blow lands.
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let hero = band[0];
    s.players.get_mut(&hero).unwrap().health = 200;
    invitation(&mut s, id, hero);
    shot(&mut s, id, hero, 1.0, HitKind::Projectile);
    run(&mut s, 1.2);
    assert!(s.players[&hero].health < 200 - 30);
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
    // A posture break from any real damage path preserves the complete reward first.
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
fn posture_retreat_backs_away_facing_the_hero_out_of_smoke_then_a_scuff_and_three_blows() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    // On the lit deck, the hero before him.
    s.mobs.get_mut(&id).unwrap().pos = [7.0, 0.5, 0.0];
    s.move_player(band[0], [9.0, 0.5, 0.0], std::f32::consts::PI, Moving::default());
    let limit = s.posture_limit(id);
    assert!(s.build_posture(id, band[0], limit));
    assert_eq!(yells(&s.events, id), vec!["You've learned something.".to_string()]);
    let start = s.mobs[&id].pos;
    let mut smoked = false;
    for _ in 0..150 {
        let ev = s.step(DT);
        smoked |= ev.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Unit(u), .. } if *caster == id && *u == id));
        if matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }) { break; }
    }
    assert!(smoked, "smoke at his feet");
    assert!(matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }));
    assert_eq!(s.mobs[&id].sidle, Some(band[0]), "he backs away still facing the hero");
    let before = s.mobs[&id].health;
    s.hurt_mob(id, band[0], 5);
    assert_eq!(s.mobs[&id].health, before - 5);
    run(&mut s, 0.5);
    let moved = dist2d(start, s.mobs[&id].pos);
    assert!(moved > 0.6 && moved < 1.6, "a walk, not a snap: {moved}");
    let mut saw_arrival = false;
    let mut blows = 0;
    for _ in 0..300 {
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Sound { id: who, sound: 649 } if *who == id)) {
            saw_arrival = true;
            assert!((dist2d(start, s.mobs[&id].pos) - RETREAT_YARDS).abs() < 0.2, "walked out of it");
        }
        let w = windups(&ev, id);
        if w > 0 {
            assert!(saw_arrival);
            assert_eq!(s.mobs[&id].pattern.map(|(k, _)| k), s.duel_pattern_named(id, "advance"));
        }
        blows += w;
        if blows == 3 && s.vancleef_open(id) {
            assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock > 2.3), "punish his hunch");
            return;
        }
    }
    panic!("advance never ended in a hunch: {blows} blows");
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
fn the_blackguards_wait_for_a_quiet_moment_turn_shake_a_head_and_walk_off_then_he_is_alone() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let watchers = s.vancleef.duels[&id].watchers.clone();
    assert_eq!(watchers.len(), RING_GUARDS);
    for (w, _) in &watchers {
        s.hurt_mob(*w, band[0], 100);
        s.engage(*w, Foe::Hero(band[0]));
        assert_eq!(s.mobs[w].health, 1);
        assert!(s.mobs[w].state.foe().is_none());
        assert!(!s.takes_help(band[0], *w), "no heal interception or ally-target slot");
    }
    // Mid-string at 30%: they stay put until it's over.
    let k = s.duel_pattern_named(id, "string").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(band[0]), s.players[&band[0]].pos);
    s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
    s.events.clear();
    s.step_vancleef(id);
    assert!(matches!(s.mobs[&id].state, MobState::Windup { .. }));
    assert_eq!(s.vancleef.duels[&id].leave, Leave::Watching);
    let mut shook = false;
    let mut walking_at = None;
    let mut cowards = None;
    let mut gone_at = None;
    for _ in 0..800 {
        let ev = s.step(DT);
        shook |= ev.iter().any(|e| matches!(e, Event::Emote { id: w, emote: EMOTE_NO } if *w == watchers[0].0));
        if cowards.is_none() && yells(&ev, id).iter().any(|t| t == "Cowards! All of you!") {
            cowards = Some(s.clock);
            // The one moment: the ring is down, he calls after them, his gaps halve, all while they
            // still stand where they were.
            assert!(!s.ring_up(id) && s.vancleef.duels[&id].held.is_empty());
            assert!(s.vancleef.duels[&id].alone);
            assert!(watchers.iter().all(|(w, _)| s.mobs.contains_key(w) && s.mobs[w].path.is_empty()), "they have not yet set off");
        }
        if walking_at.is_none() && matches!(s.vancleef.duels[&id].leave, Leave::Walking { .. }) {
            walking_at = Some(s.clock);
            assert!(shook, "one shakes his head before they go");
            assert!(cowards.is_some_and(|c| s.clock - c >= LEAVE_TURN - 0.1), "the line comes as they turn, before they walk");
            assert!(watchers.iter().all(|(w, _)| !s.mobs[w].path.is_empty()));
        }
        if watchers.iter().all(|(w, _)| !s.mobs.contains_key(w)) { gone_at = Some(s.clock); break; }
    }
    assert!(walking_at.is_some() && cowards.is_some() && gone_at.is_some());
    assert!(s.vancleef.duels[&id].alone);
    assert!(s.vancleef.observers.is_empty());
    assert_eq!(s.vancleef.duels[&id].leave, Leave::Gone);
    assert_eq!(s.duel_gap(id), GAP * 0.5, "alone, his gaps halve");
}

#[test]
fn unsupported_observer_departures_stay_absent_on_custom_land() {
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.remove_duel(id);
    s.land = Box::new(NoPath);
    s.clean_duels();
    assert!(s.vancleef.duels[&id].watchers.is_empty());
    assert!(s.vancleef.observers.is_empty());
    s.vancleef.duels.get_mut(&id).unwrap().entered = true;
    s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
    s.step_vancleef(id);
    run(&mut s, 9.0);
    assert!(!s.mobs.keys().any(|w| s.duel_observer(*w)), "no replacement combat or stranded actors");
}

#[test]
fn his_lines_never_come_closer_than_two_seconds() {
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.duel_say(id, "Cowards! All of you!");
    s.duel_say(id, "This ends here.");
    assert_eq!(yells(&s.events, id).len(), 1);
    s.events.clear();
    let ev = run(&mut s, 1.9);
    assert!(yells(&ev, id).is_empty());
    let ev = run(&mut s, 0.3);
    assert_eq!(yells(&ev, id), vec!["This ends here.".to_string()]);
}

#[test]
fn last_stand_waits_for_recovery_ends_in_a_kneel_and_death_stops_the_pose() {
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
        let steps = s.content.mobs[s.mobs[&id].def].patterns[s.duel_pattern_named(id, "last stand").unwrap()].steps.len();
        let mut blows = 0;
        let mut kneel = None;
        let mut moved = false;
        for _ in 0..400 {
            let ev = s.step(DT);
            blows += windups(&ev, id);
            // The hero steps aside under the last blow: the kneel turns to them.
            if blows == steps && !moved {
                moved = true;
                let at = s.mobs[&id].pos;
                s.players.get_mut(&band[0]).unwrap().pos = [at[0] - 3.0, at[1] + 3.0, at[2]];
            }
            if let Some(secs) = poses(&ev, id).iter().find(|(c, _)| *c == KNEEL_CLIP).map(|p| p.1) {
                kneel = Some(secs);
                let want = facing(s.mobs[&id].pos, s.players[&band[0]].pos);
                let turned = ev.iter().any(|e| matches!(e, Event::MobStop { id: who, facing: f, .. } if *who == id && angle_between(*f, want) < 1.0));
                assert!(turned, "{key}: he kneels facing the hero");
                assert!(angle_between(s.mobs[&id].facing, want) < 1.0, "{key}: and stays so");
                break;
            }
        }
        assert_eq!(blows, steps);
        assert!(kneel.is_some_and(|k| k >= 2.9), "{key}: the last opening is a kneel: {kneel:?}");
        s.events.clear();
        s.hurt_mob(id, band[0], 100_000);
        assert_eq!(s.mobs[&id].health, 0, "can be killed in the kneel");
        assert!(poses(&s.events, id).contains(&(HUNCH_CLIP, 0.0)), "death stops his pose at once");
        assert!(!s.vancleef.duels.contains_key(&id));
    }
}

#[test]
fn reset_wipe_and_death_clear_every_encounter_beat_and_observer() {
    for beat in [Beat::Invitation { until: 8.0 }, Beat::Broken, Beat::Retreat { until: 8.0 }, Beat::Arrival { until: 8.0 }, Beat::Entrance { until: 8.0 }] {
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
fn a_rooted_retreat_times_out_without_a_teleport_and_reset_clears_the_pose() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    s.vancleef_broken(id);
    s.step_vancleef(id);
    let at = s.mobs[&id].pos;
    let root = crate::content::AbilityEffect { kind: "root".into(), secs: [10.0, 10.0], vs_bosses: true, ..Default::default() };
    s.apply_effects(band[0], id, &[root], 1.0, "Roots");
    run(&mut s, RETREAT_LIMIT + 0.1);
    assert_eq!(s.mobs[&id].pos, at);
    assert!(!matches!(s.vancleef.duels[&id].beat, Beat::Retreat { .. }));
    s.reset_duels();
    assert!(s.vancleef.observers.is_empty());
    assert!(s.vancleef.duels.is_empty());
    assert!(!s.mobs[&id].guarding);
    assert!(s.mobs[&id].pattern.is_none());
    assert!(s.mobs[&id].state.foe().is_none());
}

#[test]
fn the_actual_secret_handoff_has_one_unbowed_who_kneels_roars_and_waits_before_his_first_blow() {
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
        assert_eq!(s.vancleef.duels[&id].watchers.len(), RING_GUARDS);
        for (w, _) in &s.vancleef.duels[&id].watchers {
            assert!(s.mobs[w].pack.is_none() && s.mobs[w].summoned_by.is_none());
            let d = &s.content.mobs[s.mobs[w].def];
            assert!(d.drops.is_empty() && d.trophy.is_empty() && d.gear.is_empty());
        }
        s.hurt_mob(id, hero, 100_000);
        assert!(s.vancleef.observers.is_empty());
        assert_eq!(s.below.as_ref().unwrap().slain.contains("vancleef"), !secret);
        let mut ev = Vec::new();
        for _ in 0..((4.2 / DT) as usize) { ev.extend(s.step(DT)); }
        let forms: Vec<_> = s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].key == "dm_vancleef_unbowed").map(|(id, _)| *id).collect();
        assert_eq!(forms.len(), usize::from(secret));
        if let Some(form) = forms.first().copied() {
            assert!(s.vancleef.duels[&form].watchers.is_empty());
            assert!(poses(&ev, form).contains(&(KNEEL_CLIP, RISE_KNEEL)), "he rises from a kneel");
            assert!(!ev.iter().any(|e| matches!(e, Event::Emote { id, .. } if *id == form)), "the roar comes as he stands");
            let mut roared = None;
            let mut first = None;
            for _ in 0..200 {
                let e = s.step(DT);
                if e.iter().any(|e| matches!(e, Event::Emote { id, emote: ROAR_EMOTE } if *id == form)) { roared.get_or_insert(s.clock); }
                if windups(&e, form) > 0 { first = Some(s.clock); break; }
            }
            let (roared, first) = (roared.unwrap(), first.unwrap());
            assert!(first - roared >= RISE_WAIT - 0.06, "waits {:.2} after the roar", first - roared);
            s.hurt_mob(form, hero, 100_000);
            assert!(s.below.as_ref().unwrap().slain.contains("vancleef"));
            run(&mut s, 5.0);
            assert!(!s.mobs.values().any(|m| m.health > 0 && s.content.mobs[m.def].key.contains("vancleef")));
        }
    }
}

#[test]
fn unbowed_has_longer_fixed_strings_the_turned_invitation_and_only_the_final_hunch_of_a_string_is_open() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 2);
    assert!(s.vancleef.duels[&id].watchers.is_empty());
    assert!(s.duel_pattern_named(id, "invitation").is_some() && s.duel_pattern_named(id, "riposte").is_some());
    let normal = &s.content.mobs[s.content.mob("dm_vancleef").unwrap()];
    let second = &s.content.mobs[s.mobs[&id].def];
    assert!(normal.patterns.iter().all(|p| p.steps.iter().all(|st| st.windup.unwrap_or(0.0) >= 0.0)));
    assert_ne!((second.weapon, second.off_hand), (normal.weapon, normal.off_hand), "he looks different");
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
    s.windup(id, Foe::Hero(band[0]), s.players[&band[0]].pos);
    let mut hunches = Vec::new();
    let mut blows = 0;
    for _ in 0..200 {
        let ev = s.step(DT);
        blows += windups(&ev, id);
        hunches.extend(poses(&ev, id).into_iter().filter(|p| p.1 > 0.0).map(|p| (blows, p.1)));
        if !hunches.is_empty() { break; }
    }
    assert_eq!(hunches, vec![(4, hunches[0].1)], "only after the fourth blow");
    assert!(hunches[0].1 >= 2.7);
}

// --- a hero who plays by what they see ---------------------------------------------------------

/// What a duel against one form cost a hero playing by its tells alone.
#[derive(Debug, Default)]
struct Duelled {
    cleared: bool,
    secs: f32,
    hits_taken: u32,
    damage_taken: u32,
    breaks: u32,
    ripostes: u32,
    invitations_waited: u32,
    health_left: u32,
    alone_at: Option<f32>,
    /// Shoves back in from the guards' line, and how far from the ring's middle the hero ever stood.
    shoves: u32,
    farthest: f32,
}

/// A warrior with the starting sword and their own health, reading only what the client shows:
/// the hunch and the kneel (hit), the raised blades (wait), a windup (parry it as it lands), the
/// red glow (sidestep it). `provoke`: they hit into every other Invitation, to learn the riposte.
fn duel_by_tells(key: &str, seed: u64, provoke: bool) -> Duelled {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), seed, 0.0);
    s.content.pvp = Default::default();
    let hero = s.join("hero", "Hero", None);
    s.move_player(hero, [9.0, 0.5, 0.0], std::f32::consts::PI, Moving::default());
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
    s.engage(id, Foe::Hero(hero));
    s.clean_duels();
    if !s.first_form(id) { s.vancleef_risen(id); }
    let reaction = crate::tests_fair::REACTION;
    let weapon = s.weapon_of(&s.players[&hero]).clone();
    let swing = weapon.windup + weapon.active;
    let mut out = Duelled::default();
    let start = s.clock;
    let mut open_until = -1.0f32;
    let mut blades_up = false;
    let mut provoked = false;
    // A blow seen coming: when it lands, when it was seen, whether it glows (sidestep it).
    let mut threat: Option<(f32, f32, bool)> = None;
    let mut glow_seen = -1.0f32;
    let mut parity = 0;
    while s.clock - start < 240.0 {
        let now = s.clock;
        let Some(m) = s.mobs.get(&id) else { out.cleared = true; break };
        if m.health == 0 { out.cleared = true; break; }
        if s.players[&hero].health == 0 { break; }
        let (bpos, me) = (m.pos, s.players[&hero].pos);
        let d = dist2d(bpos, me);
        let toward = facing(me, bpos);
        let action_ready = matches!(s.players[&hero].action, Action::Ready);
        let guarding = matches!(s.players[&hero].action, Action::Guard);
        let mut acted = false;
        if let Some((land, seen, glow)) = threat {
            if now >= seen {
                if glow {
                    // A sidestep across his line as it comes.
                    if land - now < 0.32 && action_ready {
                        s.move_player(hero, me, toward, Moving { left: true, ..Moving::default() });
                        s.input(hero, Input::Dodge);
                        threat = None;
                        acted = true;
                    }
                } else if land - now <= 0.12 && !guarding {
                    s.move_player(hero, me, toward, Moving::default());
                    s.input(hero, Input::Guard);
                    acted = true;
                }
            }
            if threat.is_some_and(|t| now > t.0 + 0.1) {
                threat = None;
                if guarding { s.input(hero, Input::GuardUp); }
            }
        } else if guarding {
            s.input(hero, Input::GuardUp);
        }
        let open = now < open_until - swing;
        let invite_hit = blades_up && provoke && !provoked && parity % 2 == 0;
        if !acted && action_ready && threat.is_none() {
            if (open || invite_hit) && d <= weapon.range + 0.4 {
                s.move_player(hero, me, toward, Moving::default());
                s.input(hero, Input::Attack);
                if invite_hit { provoked = true; }
            } else if d > 2.6 && !blades_up {
                let step = 7.0 * DT;
                let to = [me[0] + toward.cos() * step, me[1] + toward.sin() * step, me[2]];
                s.move_player(hero, to, toward, Moving { forward: true, ..Moving::default() });
            } else {
                s.move_player(hero, me, toward, Moving::default());
            }
        }
        for e in s.step(DT) {
            match e {
                Event::HoldPose { id: who, clip, secs, .. } if who == id && (clip == HUNCH_CLIP || clip == KNEEL_CLIP) => {
                    open_until = if secs > 0.0 { s.clock + secs - 0.05 } else { -1.0 };
                }
                Event::MobGuard { id: who, up } if who == id => {
                    if blades_up && !up && !provoked { out.invitations_waited += 1; }
                    if up { parity += 1; provoked = false; }
                    blades_up = up;
                }
                Event::Glow { id: who, .. } if who == id => {
                    glow_seen = s.clock;
                    out.ripostes += 1;
                    if let Some(t) = threat.as_mut() { t.2 = true; }
                }
                Event::Yell { id: who, ref text } if who == id && text == "You've learned something." => { out.breaks += 1; }
                Event::MobWindup { id: who, windup, .. } if who == id => {
                    let glow = (s.clock - glow_seen).abs() < 0.01;
                    threat = Some((s.clock + windup, s.clock + reaction, glow));
                }
                Event::Hop { player, dir, speed, .. } if player == hero => {
                    // The hop is flown by the hero's own client: they land where it carries them.
                    out.shoves += 1;
                    let (p, yards) = (s.players[&hero].pos, speed * moves::airtime(SHOVE_LIFT));
                    s.move_player(hero, [p[0] + dir.cos() * yards, p[1] + dir.sin() * yards, p[2]], dir, Moving::default());
                }
                Event::Swing { attacker, victim: Some(v), damage, .. } if attacker == id && v == hero && damage > 0 => {
                    out.hits_taken += 1;
                    out.damage_taken += damage;
                }
                _ => {}
            }
        }
        if out.alone_at.is_none() && s.vancleef.duels.get(&id).is_some_and(|d| d.alone) { out.alone_at = Some(s.clock - start); }
        if let Some(c) = s.vancleef.duels.get(&id).map(|d| d.ring) { out.farthest = out.farthest.max(dist2d(s.players[&hero].pos, c)); }
    }
    out.secs = s.clock - start;
    out.health_left = s.players[&hero].health;
    out
}

#[test]
fn a_lone_sword_hero_playing_by_the_tells_clears_both_forms() {
    for key in ["dm_vancleef", "dm_vancleef_unbowed"] {
        for (seed, provoke) in [(3, false), (5, true), (11, true)] {
            let out = duel_by_tells(key, seed, provoke);
            eprintln!("{key} seed {seed} provoke {provoke}: {out:?}");
            assert!(out.cleared, "{key}: {out:?}");
            assert!(out.health_left > 0);
            assert!(out.breaks >= 1, "{key}: the posture break is earned: {out:?}");
            if key == "dm_vancleef" && provoke { assert!(out.ripostes >= 1); }
            // The turned Invitation is answered every time: the thrust came (red blades and its
            // windup) and the hero was never hit by it. Clearly under 2 min (it was 108 to 110
            // sec before the Invitation came back and his health fell from 1100 to 900).
            if key == "dm_vancleef_unbowed" {
                assert!(out.ripostes >= 2 && out.hits_taken == 0, "{out:?}");
                assert!(out.secs < 100.0, "Unbowed is clearly shorter than 2 min: {out:?}");
            }
            // The ring never gets in a hero's way: they stood no farther out than its line.
            if key == "dm_vancleef" { assert!(out.farthest <= RING_RADIUS, "{out:?}"); }
        }
    }
}

/// His wind-ups rise, blades up and back over the head (the Undead man's overheads), and only his
/// hunch is low: none of his blows plays the human man's crouching cuts or spinning special
/// (Nov 9 filming: a wind-up crouch read as more of a hunch than the hunch did).
#[test]
fn his_wind_ups_rise_and_only_the_hunch_is_low() {
    assert_eq!(HUNCH_CLIP, 120, "StealthStand: bent well forward, blades down");
    let content = Content::dev();
    for key in ["dm_vancleef", "dm_vancleef_unbowed"] {
        let d = &content.mobs[content.mob(key).unwrap()];
        for p in d.patterns.iter().filter(|p| p.name != "riposte" && p.name != "invitation") {
            for st in &p.steps {
                assert_eq!(st.from.as_deref(), Some("ScourgeMale"), "{key} {}: a rising take", p.name);
                assert!(matches!((st.clip, st.take), (Some(17), 0 | 2) | (Some(18), 0)), "{key} {}: {:?} take {}", p.name, st.clip, st.take);
            }
        }
    }
}

/// A hit into his hunch doesn't flinch him out of it: the pose holds through the punish (a
/// flinch replaces any held pose on the clients). Outside the hunch, hits flinch him as ever.
#[test]
fn hits_into_the_hunch_leave_it_held() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    let until = s.clock + 1.8;
    s.mobs.get_mut(&id).unwrap().state = MobState::Recover { foe: Foe::Hero(band[0]), until };
    s.vancleef_string_landed(id, s.duel_pattern_named(id, "string"));
    assert!(s.vancleef_open(id) && s.posed(id));
    assert!(poses(&s.events, id).contains(&(HUNCH_CLIP, 1.8)));
    let flinches = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::React { id: who, hold, fit, .. } if *who == id && *hold == 0.0 && *fit == 0.0)).count();
    let pos = s.mobs[&id].pos;
    s.move_player(band[0], [pos[0] + 1.5, pos[1], pos[2]], std::f32::consts::PI, Moving::default());
    s.events.clear();
    s.input(band[0], Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(s.mobs[&id].health < s.mobs[&id].max_health, "the hit landed");
    assert_eq!(flinches(&ev), 0, "no flinch in the hunch");
    s.clock = until + 0.1;
    assert!(!s.posed(id), "the pose is over with the hunch");
}

/// Unbowed's rise-kneel holds through a hit too: a blow while he kneels doesn't flinch him up
/// before he stands to roar.
#[test]
fn a_hit_while_unbowed_kneels_leaves_the_kneel_held() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    s.vancleef_risen(id);
    assert!(s.posed(id), "the kneel is an open pose");
    assert!(poses(&s.events, id).contains(&(KNEEL_CLIP, RISE_KNEEL)));
    let flinches = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::React { id: who, hold, fit, .. } if *who == id && *hold == 0.0 && *fit == 0.0)).count();
    let pos = s.mobs[&id].pos;
    s.move_player(band[0], [pos[0] + 1.5, pos[1], pos[2]], std::f32::consts::PI, Moving::default());
    s.events.clear();
    s.input(band[0], Input::Attack);
    let ev = run(&mut s, 0.8);
    assert!(s.mobs[&id].health < s.mobs[&id].max_health, "the hit landed");
    assert_eq!(flinches(&ev), 0, "no flinch in the kneel");
    run(&mut s, RISE_KNEEL);
    assert!(!s.posed(id), "the pose is over once he stands");
}

/// Unbowed's dark swirl is on him while he fights and goes when he falls (take 421 filmed it
/// burning on his body after the fall).
#[test]
fn unbowed_swirl_goes_with_his_fall() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    let swirl = s.content.mobs[s.mobs[&id].def].aura;
    assert_ne!(swirl, 0);
    assert!(s.auras_of(id).contains(&swirl));
    s.events.clear();
    s.hurt_mob(id, band[0], u32::MAX);
    assert!(!s.auras_of(id).contains(&swirl), "no swirl on the body");
    assert!(s.events.iter().any(|e| matches!(e, Event::Auras { id: who } if *who == id)), "the clients are told");
}

/// A deck 6 yd either side of the keel line, the sea 12 yd under it beyond.
struct Ship;

impl Land for Ship {
    fn height(&self, _x: f32, y: f32) -> f32 {
        if y.abs() <= 6.0 { 0.0 } else { -12.0 }
    }
    fn water(&self, _x: f32, y: f32) -> bool {
        y.abs() > 6.0
    }
}

fn on_ship(key: &str) -> (Survival, Id, Id) {
    let mut s = Survival::bare(Content::dev(), Box::new(Ship), 7, 0.0);
    s.content.pvp = Default::default();
    let hero = s.join("hero", "Hero", None);
    s.move_player(hero, [2.0, 4.0, 0.0], std::f32::consts::PI, Moving::default());
    let p = s.players.get_mut(&hero).unwrap();
    p.health = 100_000;
    p.max_health = 100_000;
    p.last_safe = [2.0, 4.0, 0.0];
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
    s.engage(id, Foe::Hero(hero));
    s.clean_duels();
    s.vancleef.duels.get_mut(&id).unwrap().entered = true;
    (s, hero, id)
}

#[test]
fn a_roll_over_the_rail_mid_duel_sets_the_hero_back_on_the_deck_and_the_fight_goes_on() {
    for key in ["dm_vancleef", "dm_vancleef_unbowed"] {
        let (mut s, hero, id) = on_ship(key);
        run(&mut s, 0.2);
        let firm = s.players[&hero].last_safe;
        assert!(firm[1].abs() <= 6.0 && firm[2].abs() < 0.01, "{key}: firm on the deck: {firm:?}");
        s.events.clear();
        // The roll carries them 3 yd past the rail, still at the deck's height (in the air).
        s.move_player(hero, [2.0, 9.0, 0.0], std::f32::consts::PI, Moving::default());
        let at = s.players[&hero].pos;
        assert!(at[1].abs() <= 6.0 && at[2] >= 0.0, "{key}: back on the deck: {at:?}");
        assert!(s.events.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == hero)), "{key}: their client is told");
        // And already falling, the next report under the deck: still back on it.
        s.move_player(hero, [2.0, 9.5, -5.0], std::f32::consts::PI, Moving::default());
        assert!(s.players[&hero].pos[1].abs() <= 6.0, "{key}");
        run(&mut s, 1.0);
        assert_eq!(s.mobs[&id].state.foe(), Some(Foe::Hero(hero)), "{key}: the duel goes on");
        // Across the deck it's an ordinary step.
        s.events.clear();
        s.move_player(hero, [3.0, -5.0, 0.0], std::f32::consts::PI, Moving::default());
        assert_eq!(s.players[&hero].pos, [3.0, -5.0, 0.0], "{key}");
        assert!(!s.events.iter().any(|e| matches!(e, Event::Teleport { .. })), "{key}");
    }
}

#[test]
fn out_of_his_fight_the_ships_side_is_an_ordinary_fall() {
    let (mut s, hero, id) = on_ship("dm_vancleef");
    s.remove_duel(id);
    s.mobs.remove(&id);
    run(&mut s, 0.2);
    s.move_player(hero, [2.0, 9.0, 0.0], std::f32::consts::PI, Moving::default());
    assert_eq!(s.players[&hero].pos, [2.0, 9.0, 0.0]);
}

// --- the Blackguards' ring ------------------------------------------------------------------------

/// Where a hero stands in the ring: `d` yd from its middle, at `angle`.
fn ring_at(s: &Survival, id: Id, angle: f32, d: f32) -> Vec3 {
    let c = s.vancleef.duels[&id].ring;
    [c[0] + angle.cos() * d, c[1] + angle.sin() * d, c[2]]
}

fn put(s: &mut Survival, hero: Id, at: Vec3) {
    s.move_player(hero, at, std::f32::consts::PI, Moving::default());
}

/// A fight with the ring up and every hero held inside it (standing at its middle).
fn ringed(heroes: usize) -> (Survival, Vec<Id>, Id) {
    let (mut s, band, id) = fight("dm_vancleef", heroes);
    for h in &band {
        let c = s.vancleef.duels[&id].ring;
        put(&mut s, *h, c);
    }
    run(&mut s, 0.1);
    assert!(s.ring_up(id), "the ring stands once the fight has begun");
    for h in &band { assert!(s.vancleef.duels[&id].held.contains(h)); }
    (s, band, id)
}

fn hops(ev: &[Event], hero: Id) -> Vec<(f32, f32)> {
    ev.iter().filter_map(|e| match e { Event::Hop { player, dir, speed, .. } if *player == hero => Some((*dir, *speed)), _ => None }).collect()
}

fn guard_heads_shaken(ev: &[Event], s: &Survival, id: Id) -> usize {
    let guards: Vec<Id> = s.vancleef.duels[&id].watchers.iter().map(|(w, _)| *w).collect();
    ev.iter().filter(|e| matches!(e, Event::Emote { id: w, emote: EMOTE_NO } if guards.contains(w))).count()
}

#[test]
fn eight_guards_stand_in_a_ring_round_the_fight_facing_in_and_the_deck_is_closed() {
    let (s, _, id) = ringed(1);
    let duel = &s.vancleef.duels[&id];
    assert_eq!(duel.watchers.len(), RING_GUARDS);
    let mut spots = Vec::new();
    for (w, _) in &duel.watchers {
        let m = &s.mobs[w];
        assert!((dist2d(m.pos, duel.ring) - RING_RADIUS).abs() < 0.05, "on the ring: {:?}", m.pos);
        assert!(angle_between(m.facing, facing(m.pos, duel.ring)) < 1.0, "facing in");
        spots.push(m.pos);
    }
    // 6.9 yd between neighbors, and none stands within 3.5 yd of his lit spot.
    let spot = s.duel_spot(id);
    for (i, a) in spots.iter().enumerate() {
        assert!(dist2d(*a, spot) > 3.5, "no guard on his spot: {a:?}");
        let b = spots[(i + 1) % spots.len()];
        assert!((dist2d(*a, b) - 6.9).abs() < 0.2, "{a:?} {b:?}");
    }
    assert!(dist2d(spot, duel.ring) < RING_RADIUS - RING_ROOM - 2.0, "his spot has room behind it: {}", dist2d(spot, duel.ring));
}

#[test]
fn a_hero_nearing_the_line_sees_a_guard_turn_and_shake_his_head_before_any_shove() {
    let (mut s, band, id) = ringed(1);
    let hero = band[0];
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 1.6);
    put(&mut s, hero, at);
    s.events.clear();
    let ev = run(&mut s, 0.1);
    assert!(guard_heads_shaken(&ev, &s, id) >= 1, "a guard shakes his head");
    assert!(hops(&ev, hero).is_empty(), "no shove yet");
    assert!(!ev.iter().any(|e| matches!(e, Event::Flash { .. })));
    // And not again at once (a hero walking the line is not nagged every step).
    let again = run(&mut s, 0.5);
    assert_eq!(guard_heads_shaken(&again, &s, id), 0);
}

#[test]
fn a_hero_who_backs_into_the_line_is_shoved_back_in_with_no_damage_and_no_stagger() {
    let (mut s, band, id) = ringed(1);
    let hero = band[0];
    let health = s.players[&hero].health;
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 0.5);
    put(&mut s, hero, at);
    s.events.clear();
    let ev = run(&mut s, 0.1);
    let hop = hops(&ev, hero);
    assert_eq!(hop.len(), 1, "one shove: {hop:?}");
    // Inward, and ending inside the line.
    let c = s.vancleef.duels[&id].ring;
    assert!(angle_between(hop[0].0, facing(at, c)) < 45.0, "toward the middle");
    let yards = hop[0].1 * moves::airtime(SHOVE_LIFT);
    let end = [at[0] + hop[0].0.cos() * yards, at[1] + hop[0].0.sin() * yards, 0.0];
    assert!(dist2d(end, c) <= RING_RADIUS - RING_EDGE, "ends inside the line");
    assert!(yards >= 2.0 && yards <= SHOVE_MAX, "{yards}");
    // Its tells: the guard's strike, dust and a knock on the hero, and the words.
    assert!(ev.iter().any(|e| matches!(e, Event::Clip { clip: SHOVE_ANIM, .. })));
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id: v, kit: combat::KIT_BASH, .. } if *v == hero)));
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == hero && text == "A guard shoved you back.")));
    let guards: Vec<Id> = s.vancleef.duels[&id].watchers.iter().map(|(w, _)| *w).collect();
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id: w, text } if guards.contains(w) && text == "Nobody leaves.")));
    // No damage, no stagger, no control lost.
    assert_eq!(s.players[&hero].health, health);
    assert!(!matches!(s.players[&hero].action, Action::Stagger { .. }));
    // Pressed against the line again at once: not shoved twice inside the gap, and the cry is once.
    s.events.clear();
    put(&mut s, hero, at);
    let ev = run(&mut s, 0.3);
    assert!(hops(&ev, hero).is_empty(), "a hero is moved, not juggled");
    put(&mut s, hero, at);
    let ev = run(&mut s, SHOVE_GAP);
    assert_eq!(hops(&ev, hero).len(), 1, "then again");
    assert!(!ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Nobody leaves.")), "the cry comes once a fight");
}

#[test]
fn a_dodge_over_the_line_is_never_cut_short_and_its_landing_is_shoved_in_after() {
    let (mut s, band, id) = ringed(1);
    let hero = band[0];
    let out = ring_at(&s, id, 2.0, RING_RADIUS + 1.5);
    put(&mut s, hero, out);
    s.players.get_mut(&hero).unwrap().action = Action::Dodge { t: 0.1, dir: 0.0, dash: true };
    s.events.clear();
    let ev = run(&mut s, 0.1);
    assert!(hops(&ev, hero).is_empty(), "no shove in a dodge");
    s.players.get_mut(&hero).unwrap().action = Action::Ready;
    let ev = run(&mut s, 0.1);
    assert_eq!(hops(&ev, hero).len(), 1, "set back in once it ends");
}

#[test]
fn a_hero_outside_walks_in_unhindered_and_only_those_inside_are_held() {
    let (mut s, band, id) = ringed(2);
    let (inside, outside) = (band[0], band[1]);
    // The second hero was outside: they walk the line and in; nothing touches them until they stand deep in.
    let out = ring_at(&s, id, 1.0, RING_RADIUS + RING_REACH + 1.0);
    put(&mut s, outside, out);
    run(&mut s, 0.1);
    assert!(!s.vancleef.duels[&id].held.contains(&outside));
    let mut shoves = 0;
    for d in [RING_RADIUS + 1.0, RING_RADIUS - 0.5, RING_RADIUS - 1.5, RING_RADIUS - 2.4] {
        let at = ring_at(&s, id, 1.0, d);
        put(&mut s, outside, at);
        shoves += hops(&run(&mut s, 0.1), outside).len();
    }
    assert_eq!(shoves, 0, "let through the line");
    let deep = ring_at(&s, id, 1.0, RING_RADIUS - RING_DEEP - 0.5);
    put(&mut s, outside, deep);
    run(&mut s, 0.1);
    assert!(s.vancleef.duels[&id].held.contains(&outside), "now held");
    // The hero who stood in all along is shoved at the line; the one deep inside is not.
    let at = ring_at(&s, id, 4.0, RING_RADIUS - 0.5);
    put(&mut s, inside, at);
    s.events.clear();
    let ev = run(&mut s, 0.1);
    assert_eq!(hops(&ev, inside).len(), 1);
    assert!(hops(&ev, outside).is_empty());
}

#[test]
fn a_hero_a_guard_cannot_reach_walks_out_through_the_gap_and_a_dead_hero_is_never_shoved() {
    let (mut s, band, id) = ringed(2);
    let (hero, dead) = (band[0], band[1]);
    // The guards near the hero's angle are not there: a gap.
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 0.5);
    let near: Vec<Id> = s.vancleef.duels[&id].watchers.iter().map(|(w, _)| *w)
        .filter(|w| dist2d(s.mobs[w].pos, at) < RING_REACH + 4.0).collect();
    for w in near {
        s.remove_watcher(w);
        s.vancleef.duels.get_mut(&id).unwrap().watchers.retain(|(g, _)| *g != w);
    }
    put(&mut s, hero, at);
    s.events.clear();
    let ev = run(&mut s, 0.1);
    assert!(hops(&ev, hero).is_empty(), "nobody there to shove them");
    // The dead are not held.
    let at = ring_at(&s, id, 3.5, RING_RADIUS - 0.5);
    s.players.get_mut(&dead).unwrap().health = 0;
    put(&mut s, dead, at);
    let ev = run(&mut s, 0.1);
    assert!(hops(&ev, dead).is_empty());
}

#[test]
fn a_shove_never_sets_a_hero_down_on_him_or_outside_the_line_and_a_hero_is_never_pinned() {
    let (mut s, _, id) = ringed(1);
    let c = s.vancleef.duels[&id].ring;
    for step in 0..36 {
        let angle = step as f32 * std::f32::consts::TAU / 36.0;
        let at = ring_at(&s, id, angle, RING_RADIUS - 0.4);
        // Him, as close to the hero as he fights: 2.5 yd in from them, straight in, or to either side.
        for aside in [0.0f32, 0.9, -0.9] {
            let a = angle + std::f32::consts::PI + aside;
            let boss = [at[0] + a.cos() * 2.5, at[1] + a.sin() * 2.5, 0.0];
            s.mobs.get_mut(&id).unwrap().pos = boss;
            let to = s.shove_to(id, at).unwrap_or_else(|| panic!("a shove is always possible: angle {angle} aside {aside}"));
            assert!(dist2d(to, boss) >= SHOVE_CLEAR, "not on top of him: {to:?} vs {boss:?}");
            assert!(dist2d(to, c) <= RING_RADIUS - RING_EDGE, "inside the line");
            assert!(dist2d(at, to) <= SHOVE_MAX + 0.01);
        }
    }
}

#[test]
fn his_own_lunge_ends_inside_the_ring_and_never_carries_the_fight_through_the_guards() {
    let (mut s, band, id) = ringed(1);
    let hero = band[0];
    let c = s.vancleef.duels[&id].ring;
    // Him 6 yd out on the east, the hero 1.8 yd further: the riposte would carry 1.5 yd past them.
    let (boss, stood) = ([c[0] + 6.0, c[1], 0.0], [c[0] + RING_RADIUS - 1.2, c[1], 0.0]);
    s.mobs.get_mut(&id).unwrap().pos = boss;
    put(&mut s, hero, stood);
    invitation(&mut s, id, hero);
    shot(&mut s, id, hero, 1.0, HitKind::Projectile);
    run(&mut s, 1.2);
    let m = &s.mobs[&id];
    assert!(dist2d(m.pos, c) <= RING_RADIUS - RING_ROOM + 0.05, "stopped inside the line: {}", dist2d(m.pos, c));
    // Once the ring is down, the whole deck is his again.
    s.vancleef.duels.get_mut(&id).unwrap().leave = Leave::Gone;
    assert!(s.ring_room(id, [c[0] + RING_RADIUS + 3.0, c[1], 0.0]));
}

#[test]
fn the_ring_opens_at_the_turn_and_a_hero_at_the_line_is_free() {
    let (mut s, band, id) = ringed(1);
    let hero = band[0];
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 0.5);
    s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
    s.mobs.get_mut(&id).unwrap().pattern = None;
    s.mobs.get_mut(&id).unwrap().state = MobState::Recover { foe: Foe::Hero(hero), until: s.clock + 1.0 };
    let mut turned = false;
    for _ in 0..200 {
        let inside = ring_at(&s, id, 0.3, RING_RADIUS - 3.0);
        put(&mut s, hero, inside);
        s.step(DT);
        if matches!(s.vancleef.duels[&id].leave, Leave::Turning { .. }) { turned = true; break; }
    }
    assert!(turned, "at the next quiet moment they turn");
    assert!(!s.ring_up(id));
    put(&mut s, hero, at);
    s.events.clear();
    let ev = run(&mut s, 1.0);
    assert!(hops(&ev, hero).is_empty(), "the whole deck is open");
    assert!(s.vancleef.duels[&id].held.is_empty());
    assert_eq!(s.duel_gap(id), GAP * 0.5);
}

#[test]
fn a_lost_fight_puts_the_ring_back_whole() {
    let (mut s, band, id) = ringed(2);
    // Shoved once: the cry is spent, the hero held.
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 0.5);
    put(&mut s, band[0], at);
    run(&mut s, 0.1);
    assert!(s.vancleef.duels[&id].cried);
    // Everyone falls: he returns, the duel and its guards are gone.
    for h in &band { s.players.get_mut(h).unwrap().health = 0; }
    run(&mut s, 0.2);
    s.reset_duels();
    s.remove_duel(id);
    assert!(s.vancleef.observers.is_empty());
    // The heroes are back and he is pulled again: eight guards, nobody held, nothing spent.
    for h in &band {
        let p = s.players.get_mut(h).unwrap();
        p.health = p.max_health;
    }
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: s.clock + 3.0 };
    s.clean_duels();
    let duel = &s.vancleef.duels[&id];
    assert_eq!(duel.watchers.len(), RING_GUARDS);
    assert!(duel.held.is_empty() && !duel.cried && !duel.alone && duel.leave == Leave::Watching);
}

#[test]
fn with_no_guards_placed_there_is_no_ring() {
    let (mut s, band, id) = fight("dm_vancleef", 1);
    s.remove_duel(id);
    s.land = Box::new(NoPath);
    s.clean_duels();
    s.vancleef.duels.get_mut(&id).unwrap().entered = true;
    let c = s.vancleef.duels[&id].ring;
    put(&mut s, band[0], c);
    run(&mut s, 0.1);
    let at = ring_at(&s, id, 0.3, RING_RADIUS - 0.5);
    put(&mut s, band[0], at);
    s.events.clear();
    let ev = run(&mut s, 0.2);
    assert!(!s.ring_up(id) && hops(&ev, band[0]).is_empty());
}

#[test]
fn the_ring_leaves_room_for_his_longest_lunge_and_every_dodge_the_fight_asks_for() {
    let (s, _, id) = ringed(1);
    let content = &s.content;
    let def = &content.mobs[s.mobs[&id].def];
    // His longest lunge, over every pattern he has; the riposte's is clamped to 7 yd in `vancleef_hit`.
    let longest = def.patterns.iter().flat_map(|p| p.steps.iter().map(|st| st.lunge)).fold(0.0f32, f32::max);
    let (line, spot) = (RING_RADIUS - RING_EDGE, dist2d(s.duel_spot(id), s.vancleef.duels[&id].ring));
    // Between his spot and the line behind it, and across the ring from it, there is room to stand
    // and to be lunged at: the lunge, a hero's reach (2.9 yd) and a step.
    let behind = line - spot;
    let across = line + spot;
    assert!(across >= longest + def.attack.range + 3.0, "room across the ring: {across} for a lunge of {longest}");
    assert!(behind >= 2.0, "a step of room behind his spot: {behind}");
    // The dodges: the sidestep dash along the line, and the step back with nothing held.
    let dash = content.dodge.speed * content.dodge.dash_secs;
    let along_line = (line * line + dash * dash).sqrt();
    assert!(along_line < RING_RADIUS, "a sidestep dash along the line stays inside the guards: {along_line} of {RING_RADIUS}");
    let back = content.dodge.back_step[0] * moves::airtime(content.dodge.back_step[1]);
    assert!(line + back < RING_RADIUS + RING_REACH, "a step back from the line ends where a guard can still set them in: {}", line + back);
    // The riposte's strip (1 yd either side) is cleared by a dash of 4 yd with room to spare.
    assert!(dash >= RIPOSTE_HALF_WIDTH * 2.0);
    eprintln!("ring {RING_RADIUS} yd, line {line}, his spot {spot:.1} from the middle ({behind:.1} behind, {across:.1} across); longest lunge {longest}, riposte up to 7; dash {dash}, step back {back:.1}");
}

#[test]
fn with_nobody_on_the_deck_there_is_no_turn_and_no_cry_at_30_percent() {
    // Unbowed's empty deck, and a first form whose guards found no place.
    let (mut s, _, id) = fight("dm_vancleef", 1);
    s.remove_duel(id);
    s.land = Box::new(NoPath);
    s.clean_duels();
    s.vancleef.duels.get_mut(&id).unwrap().entered = true;
    let (mut s2, _, unbowed) = fight("dm_vancleef_unbowed", 1);
    for (s, id) in [(&mut s, id), (&mut s2, unbowed)] {
        assert!(s.vancleef.duels[&id].watchers.is_empty());
        s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health * 3 / 10;
        let ev = run(s, 4.0);
        assert!(!yells(&ev, id).iter().any(|t| t == "Cowards! All of you!"), "nobody left to call after");
        assert_eq!(s.vancleef.duels[&id].leave, Leave::Gone);
        assert!(!s.vancleef.duels[&id].alone || !s.first_form(id));
    }
}

// --- Unbowed turns the Invitation round ---------------------------------------------------------

/// A thrust seen coming: a windup of the riposte's clip, no guard stops it.
fn thrusts(events: &[Event], id: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::MobWindup { id: who, clip: 85, blockable: false, .. } if *who == id)).count()
}

fn glows(events: &[Event], id: Id) -> Vec<f32> {
    events.iter().filter_map(|e| match e { Event::Glow { id: who, secs } if *who == id => Some(*secs), _ => None }).collect()
}

/// The Invitation's own events kept (the shared helper clears them).
fn invitation_events(s: &mut Survival, id: Id, hero: Id) {
    let k = s.duel_pattern_named(id, "invitation").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(hero), s.players[&hero].pos);
}

#[test]
fn the_turned_invitation_reads_on_his_body_red_blades_a_line_and_the_scrape_unlike_the_first_forms() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    invitation_events(&mut s, id, band[0]);
    assert!(s.mobs[&id].guarding, "blades up, the native held guard");
    assert_eq!(glows(&s.events, id), vec![INVITATION], "red from the first moment, the whole hold");
    assert!(s.events.iter().any(|e| matches!(e, Event::Sound { id: who, sound: 700 } if *who == id)));
    assert_eq!(yells(&s.events, id), vec![TURNED_LINE.to_string()]);
    // The first form's blades stay plain until the riposte itself.
    let (mut s, band, id) = fight("dm_vancleef", 1);
    invitation_events(&mut s, id, band[0]);
    assert!(glows(&s.events, id).is_empty(), "the first form's Invitation does not glow");
    assert_eq!(yells(&s.events, id), vec!["Go on, then.".to_string()]);
}

#[test]
fn a_string_is_followed_by_the_turned_invitation_and_a_lunge_does_not_spend_it() {
    let (mut s, _band, id) = fight("dm_vancleef_unbowed", 1);
    let near = s.content.mobs[s.mobs[&id].def].attack.range;
    let k = |s: &Survival, n: &str| s.duel_pattern_named(id, n);
    assert_eq!(s.vancleef_pattern(id, near - 1.0, 0.5), k(&s, "string"), "an honest string first");
    s.vancleef.duels.get_mut(&id).unwrap().invite_next = true;
    assert_eq!(s.vancleef_pattern(id, 6.0, 0.5), k(&s, "lunge"), "a lunge closes distance first");
    assert!(s.vancleef.duels[&id].invite_next, "and keeps the next Invitation");
    assert_eq!(s.vancleef_pattern(id, near - 1.0, 0.5), k(&s, "invitation"));
}

#[test]
fn standing_through_the_turned_invitation_is_met_with_the_closing_thrust() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    let hero = band[0];
    invitation(&mut s, id, hero);
    let health = s.players[&hero].health;
    // The hold itself costs nothing.
    let ev = run(&mut s, INVITATION - 0.2);
    assert_eq!(windups(&ev, id), 0);
    assert_eq!(s.players[&hero].health, health);
    // Then the thrust: a Pierce windup no guard stops, glowing, at where the hero stands.
    let ev = run(&mut s, 0.4);
    assert_eq!(thrusts(&ev, id), 1, "exactly one thrust");
    assert!(!glows(&ev, id).is_empty(), "his weapon is red through the windup");
    assert!(!s.mobs[&id].guarding);
    assert!(matches!(s.mobs[&id].state, MobState::Windup { foe: Foe::Hero(h), .. } if h == hero));
    let ev = run(&mut s, 1.2);
    assert!(s.players[&hero].health < health, "a hero who stood still is run through");
    assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, .. } if *attacker == id && *v == hero && *damage > 0)));
    assert!(s.vancleef_open(id), "and he is open after it either way");
}

#[test]
fn a_sidestep_of_the_closing_thrust_leaves_the_hero_behind_him_and_opens_him_like_the_first_forms() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    let hero = band[0];
    invitation(&mut s, id, hero);
    let stood = s.players[&hero].pos;
    let health = s.players[&hero].health;
    // Stand through the hold, then step aside as the windup begins (the same two yards the first
    // form's riposte is answered with).
    let mut came = false;
    for _ in 0..80 {
        let ev = s.step(DT);
        if thrusts(&ev, id) > 0 {
            s.move_player(hero, [stood[0], stood[1] + 2.2, stood[2]], std::f32::consts::PI, Moving::default());
            came = true;
            break;
        }
    }
    assert!(came, "the thrust came by itself, 2 sec after the blades rose");
    run(&mut s, 1.1);
    let m = &s.mobs[&id];
    assert!(dist2d([0.0; 3], m.pos) > dist2d([0.0; 3], stood) + 1.0, "past the spot: {:?}", m.pos);
    assert_eq!(s.players[&hero].health, health, "no damage to a hero who stepped aside");
    assert!(s.vancleef_open(id), "the opening, as the first form's riposte leaves");
    // The window is the thrust's own recovery: 2.4 sec after a 0.9 sec windup, as the first form's,
    // with every hit in it counting extra toward his posture.
    let step = &s.content.mobs[s.mobs[&id].def].patterns[s.duel_pattern_named(id, "riposte").unwrap()].steps[0];
    assert!((step.recovery.unwrap() - 2.4).abs() < 0.01 && (step.windup.unwrap() - 0.9).abs() < 0.01);
    assert!(s.vancleef_posture_share(id, "sword") >= HUNCH_POSTURE);
}

#[test]
fn hitting_into_the_turned_invitation_brings_the_same_thrust_at_once_and_it_too_is_sidestepped() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 2);
    invitation(&mut s, id, band[0]);
    shot(&mut s, id, band[1], 1.0, HitKind::Projectile);
    assert_eq!(thrusts(&s.events, id), 1);
    assert!(matches!(s.mobs[&id].state, MobState::Windup { foe: Foe::Hero(h), .. } if h == band[1]));
    shot(&mut s, id, band[0], 1.0, HitKind::Projectile);
    assert_eq!(thrusts(&s.events, id), 1, "a second hero cannot restart it");
    for h in &band { s.move_player(*h, [-2.0, 4.0, 0.0], 0.0, Moving::default()); }
    let health: Vec<_> = band.iter().map(|h| s.players[h].health).collect();
    let ev = run(&mut s, 1.2 + INVITATION);
    assert_eq!(band.iter().map(|h| s.players[h].health).collect::<Vec<_>>(), health);
    assert_eq!(thrusts(&ev, id), 1, "only the provoked thrust (its events come with this run): the Invitation was spent, none from the timer");
    assert!(s.vancleef_open(id));
}

#[test]
fn the_closing_thrust_never_takes_a_last_point_of_health_and_control_cancels_it() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    let hero = band[0];
    let p = s.players.get_mut(&hero).unwrap();
    p.max_health = 200;
    p.health = 30;
    invitation(&mut s, id, hero);
    run(&mut s, INVITATION + 1.6);
    let h = s.players[&hero].health;
    assert!(h > 0 && h < 30, "hurt, alive: {h}");
    // A stun in the hold drops the blades and there is no thrust.
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    invitation(&mut s, id, band[0]);
    let ability = s.content.ability("fireball").unwrap();
    s.content.abilities[ability].effects = vec![crate::content::AbilityEffect { kind: "stun".into(), secs: [2.0, 2.0], vs_bosses: true, ..Default::default() }];
    s.ability_on_mob(band[0], id, ability, 1.0, 1.0, s.players[&band[0]].pos, HitKind::Projectile, 133);
    let health = s.players[&band[0]].health;
    let ev = run(&mut s, INVITATION + 0.5);
    assert_eq!(thrusts(&ev, id), 0);
    assert!(!s.mobs[&id].guarding);
    assert_eq!(s.players[&band[0]].health, health);
}

#[test]
fn a_hero_out_of_reach_is_never_trapped_by_the_thrust_and_a_wipe_puts_it_back() {
    // Far from him when the hold ends: the thrust is a lunge of at most 7 yd and then his opening.
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    let hero = band[0];
    invitation(&mut s, id, hero);
    s.move_player(hero, [18.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let health = s.players[&hero].health;
    run(&mut s, INVITATION + 1.4);
    assert_eq!(s.players[&hero].health, health, "out of the lunge's 7 yd");
    assert!(dist2d(s.mobs[&id].pos, [0.0; 3]) <= 7.5, "and he did not chase it down: {:?}", s.mobs[&id].pos);
    // A lost fight puts everything back: no blades, no thrust, nothing pending.
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 1);
    invitation(&mut s, id, band[0]);
    run(&mut s, 0.5);
    s.reset_duels();
    assert!(!s.vancleef.duels.contains_key(&id));
    assert!(!s.mobs[&id].guarding);
    let ev = run(&mut s, INVITATION + 2.0);
    assert_eq!(thrusts(&ev, id), 0, "no thrust from a wiped Invitation");
}

#[test]
fn the_one_thrust_hits_only_those_on_its_line() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 2);
    // Hero 0 is the foe, hero 1 stands well off the line.
    s.move_player(band[1], [2.0, 6.0, 0.0], std::f32::consts::PI, Moving::default());
    invitation(&mut s, id, band[0]);
    let health: Vec<_> = band.iter().map(|h| s.players[h].health).collect();
    let ev = run(&mut s, INVITATION + 1.6);
    assert_eq!(thrusts(&ev, id), 1, "one thrust, not one each");
    assert!(s.players[&band[0]].health < health[0]);
    assert_eq!(s.players[&band[1]].health, health[1], "the one off the line is left alone");
}
