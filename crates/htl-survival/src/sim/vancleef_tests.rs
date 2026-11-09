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
    assert_eq!(watchers.len(), 2);
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
    for _ in 0..600 {
        let ev = s.step(DT);
        shook |= ev.iter().any(|e| matches!(e, Event::Emote { id: w, emote: EMOTE_NO } if *w == watchers[0].0));
        if walking_at.is_none() && matches!(s.vancleef.duels[&id].leave, Leave::Walking { .. }) {
            walking_at = Some(s.clock);
            assert!(shook, "one shakes his head before they go");
            assert!(watchers.iter().all(|(w, _)| !s.mobs[w].path.is_empty()));
        }
        if yells(&ev, id).iter().any(|t| t == "Cowards! All of you!") {
            cowards = Some(s.clock);
            assert!(watchers.iter().all(|(w, _)| !s.mobs.contains_key(w)), "the line comes once they're gone");
        }
        if cowards.is_some() { break; }
    }
    assert!(walking_at.is_some() && cowards.is_some());
    assert!(s.vancleef.duels[&id].alone);
    assert!(s.vancleef.observers.is_empty());
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
        for _ in 0..400 {
            let ev = s.step(DT);
            blows += windups(&ev, id);
            if let Some(secs) = poses(&ev, id).iter().find(|(c, _)| *c == KNEEL_CLIP).map(|p| p.1) { kneel = Some(secs); break; }
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
        assert_eq!(s.vancleef.duels[&id].watchers.len(), 2);
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
fn unbowed_has_longer_fixed_strings_no_invitation_and_only_the_final_hunch_is_open() {
    let (mut s, band, id) = fight("dm_vancleef_unbowed", 2);
    assert!(s.vancleef.duels[&id].watchers.is_empty());
    assert!(s.duel_pattern_named(id, "invitation").is_none() && s.duel_pattern_named(id, "riposte").is_none());
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
                Event::Swing { attacker, victim: Some(v), damage, .. } if attacker == id && v == hero && damage > 0 => {
                    out.hits_taken += 1;
                    out.damage_taken += damage;
                }
                _ => {}
            }
        }
        if out.alone_at.is_none() && s.vancleef.duels.get(&id).is_some_and(|d| d.alone) { out.alone_at = Some(s.clock - start); }
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
        }
    }
}
