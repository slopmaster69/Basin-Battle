use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

fn fight(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 19, 1_000_000.0);
    let band: Vec<_> = (0..heroes).map(|i| {
        let h = s.join(&format!("hero{i}"), &format!("Hero{i}"), None);
        let p = s.players.get_mut(&h).unwrap();
        p.pos = [2.5, i as f32, 0.0];
        p.facing = std::f32::consts::PI;
        p.health = 100_000;
        p.max_health = 100_000;
        p.loadout.clear();
        h
    }).collect();
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0, 0.0, 0.0], None, false);
    s.engage(id, Foe::Hero(band[0]));
    s.step(DT);
    (s, band, id)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

fn until(s: &mut Survival, id: Id, limit: f32, f: impl Fn(&Survival, Id) -> bool) {
    let end = s.clock + limit;
    while !f(s, id) && s.clock < end { s.step(DT); }
    assert!(f(s, id), "clock {}, state {:?}, completed {}", s.clock, s.mobs[&id].state,
        s.kitchen.cooks.get(&id).map_or(99, |c| c.completed));
}

fn scoop(s: &mut Survival, id: Id) {
    until(s, id, 45.0, |s, id| s.kitchen_scooping(id));
}

fn production_fight(key: &str, rtt: f32) -> (Survival, Id, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 19, 1_000_000.0);
    let h = s.join("accounting", "Accounting", None);
    s.players.get_mut(&h).unwrap().loadout.clear();
    s.move_player(h, [2.5, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    s.set_rtt(h, rtt);
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0, 0.0, 0.0], None, false);
    // Let ordinary aggro and pattern selection start the encounter.
    s.step(DT);
    (s, h, id)
}

fn move_beside_locked_blow(s: &mut Survival, band: &[Id], id: Id) {
    until(s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { t, .. }
        if t >= s.attack_now(id).windup - s.attack_now(id).lock));
    let (at, dir) = (s.mobs[&id].pos, s.mobs[&id].facing + std::f32::consts::FRAC_PI_2);
    for h in band {
        let pos = [at[0] + dir.cos() * 2.5, at[1] + dir.sin() * 2.5, at[2]];
        s.move_player(*h, pos, facing(pos, at), Moving::default());
    }
}

#[test]
fn kitchen_interrupted_first_second_and_between_step_windups_retry_the_pair() {
    for key in [COOKIE, GORGED] {
        for rtt in [0.0, 0.1] {
            for interrupt in ["first", "between", "second"] {
                let (mut s, h, id) = production_fight(key, rtt);
                until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                let pair = s.kitchen_move(id).unwrap().to_owned();
                let hp = s.players[&h].health;
                if interrupt != "first" {
                    move_beside_locked_blow(&mut s, &[h], id);
                    until(&mut s, id, 2.0, |s, id| s.mobs[&id].pattern.is_some_and(|(_, n)| n == 1));
                    assert!(matches!(s.mobs[&id].state, MobState::Recover { .. }));
                    if interrupt == "second" {
                        until(&mut s, id, 2.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                        assert_eq!(s.mobs[&id].pattern.unwrap().1, 1, "second windup really reached");
                    }
                }
                // Explicit crowd-control probe, after ordinary AI reached the requested beat.
                s.stagger_mob(id, 0.8, false);
                assert!(s.mobs[&id].pattern.is_none());
                run(&mut s, 0.85);
                assert_eq!(s.kitchen.cooks[&id].completed, 0, "{key}, {interrupt}, RTT {rtt}");
                assert_eq!(s.players[&h].health, hp, "the avoided/interrupted blows never land");
                until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                assert_eq!(s.kitchen_move(id), Some(pair.as_str()));
                assert_eq!(s.mobs[&id].pattern.unwrap().1, 0);
            }
        }
    }
}

#[test]
fn kitchen_released_pairs_count_once_when_the_final_blow_hits_or_misses() {
    for key in [COOKIE, GORGED] {
        for rtt in [0.0, 0.1] {
            for hit in [false, true] {
                let (mut s, h, id) = production_fight(key, rtt);
                let hp = s.players[&h].health;
                move_beside_locked_blow(&mut s, &[h], id);
                until(&mut s, id, 3.0, |s, id| s.mobs[&id].pattern.is_some_and(|(_, n)| n == 1)
                    && matches!(s.mobs[&id].state, MobState::Windup { .. }));
                assert_eq!(s.kitchen.cooks[&id].completed, 0);
                if hit {
                    let (at, dir) = (s.mobs[&id].pos, s.mobs[&id].facing);
                    let pos = [at[0] + dir.cos() * 2.5, at[1] + dir.sin() * 2.5, at[2]];
                    s.move_player(h, pos, facing(pos, at), Moving::default());
                } else {
                    move_beside_locked_blow(&mut s, &[h], id);
                }
                until(&mut s, id, 2.0, |s, id| s.kitchen.cooks[&id].completed == 1);
                run(&mut s, 0.25);
                assert_eq!(s.kitchen.cooks[&id].completed, 1, "the released pair counts once, {key}, RTT {rtt}");
                assert_eq!(s.players[&h].health < hp, hit);
                if key == GORGED && hit {
                    assert!(matches!(s.players[&h].action, Action::Stagger { .. }), "last-slap mercy must still count the fully released pair");
                }
                until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                assert_eq!(s.kitchen_move(id), Some(if key == COOKIE { "ladle" } else { "belch" }));
            }
        }
    }
}

#[test]
fn gorged_interrupted_belch_retries_then_a_full_belch_coughs_and_returns_to_slaps() {
    for rtt in [0.0, 0.1] {
        let (mut s, h, id) = production_fight(GORGED, rtt);
        move_beside_locked_blow(&mut s, &[h], id);
        until(&mut s, id, 3.0, |s, id| s.mobs[&id].pattern.is_some_and(|(_, n)| n == 1)
            && matches!(s.mobs[&id].state, MobState::Windup { .. }));
        move_beside_locked_blow(&mut s, &[h], id);
        until(&mut s, id, 8.0, |s, id| s.kitchen_move(id) == Some("belch"));
        s.stagger_mob(id, 0.8, false);
        run(&mut s, 0.85);
        assert_eq!(s.kitchen.cooks[&id].completed, 1);
        assert!(s.kitchen.cooks[&id].stage.is_none());
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert_eq!(s.kitchen_move(id), Some("belch"));
        move_beside_locked_blow(&mut s, &[h], id);
        until(&mut s, id, 3.0, |s, id| matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })));
        let released = s.clock;
        assert_eq!(s.kitchen.cooks[&id].completed, 0);
        let ev = run(&mut s, COUGH - 0.1);
        assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)));
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert!(s.clock >= released + COUGH);
        assert_eq!(s.kitchen_move(id), Some("slaps"));
        assert_eq!(s.players[&h].health, s.players[&h].max_health);
    }
}

#[test]
fn gorged_mercy_cutoff_repeats_the_unfinished_pair_with_or_without_network_grace() {
    for rtt in [0.1, 0.35, 0.0] {
        let (mut s, h, id) = production_fight(GORGED, rtt);
        let hp = s.players[&h].health;
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert_eq!(s.kitchen_move(id), Some("slaps"));
        until(&mut s, id, 3.0, |s, _| s.players[&h].health < hp);
        assert!(matches!(s.players[&h].action, Action::Stagger { .. }), "the first heavy slap actually staggered the hero, RTT {rtt}");
        assert!(s.mobs[&id].pattern.is_none(), "mercy still cancels the second slap");
        assert_eq!(s.kitchen.cooks[&id].completed, 0, "a mercy-cut first slap is not a full exchange, RTT {rtt}");
        let after_first = s.players[&h].health;
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert_eq!(s.kitchen_move(id), Some("slaps"));
        assert_eq!(s.mobs[&id].pattern.unwrap().1, 0, "the next attack retries the pair");
        assert_eq!(s.players[&h].health, after_first, "the interrupted follow-up never hits");
        println!("RTT {rtt}: first slap staggered the hero; mercy cancelled the follow-up; next windup retries slaps");
    }
}

#[test]
fn kitchen_first_blow_breaking_a_tired_guard_does_not_complete_the_pair() {
    for key in [COOKIE, GORGED] {
        for rtt in [0.0, 0.1] {
            let (mut s, h, id) = production_fight(key, rtt);
            until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
            let pair = s.kitchen_move(id).unwrap().to_owned();
            s.input(h, Input::Guard);
            s.players.get_mut(&h).unwrap().stamina = 1.0;
            let ev = run(&mut s, 1.15);
            assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::GuardBroken, .. }
                if *attacker == id && *v == h)), "the first blow really broke a tired guard, {key}, RTT {rtt}");
            assert!(s.mobs[&id].pattern.is_none());
            assert_eq!(s.kitchen.cooks[&id].completed, 0);
            until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
            assert_eq!(s.kitchen_move(id), Some(pair.as_str()));
            assert_eq!(s.mobs[&id].pattern.unwrap().1, 0);
        }
    }
}

#[test]
fn kitchen_counts_only_completed_strings_then_waits_for_the_single_patch_to_cool() {
    for (heroes, low) in [(1, false), (2, true), (4, false)] {
        let (mut s, _, id) = fight(COOKIE, heroes);
        if low { s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health / 5; }
        until(&mut s, id, 8.0, |s, id| s.kitchen_move(id) == Some("cleaver"));
        assert_eq!(s.kitchen.cooks[&id].completed, 0);
        s.stagger_mob(id, 0.8, false);
        assert_eq!(s.kitchen.cooks[&id].completed, 0, "an interrupted wind-up never counts");
        until(&mut s, id, 8.0, |s, id| s.mobs[&id].pattern.is_some_and(|(_, n)| n == 1));
        assert_eq!(s.kitchen.cooks[&id].completed, 0, "the first cut is not an exchange");
        until(&mut s, id, 10.0, |s, id| s.kitchen_move(id) == Some("ladle"));
        assert_eq!(s.kitchen.cooks[&id].completed, 1);
        until(&mut s, id, 10.0, |s, id| s.kitchen_move(id) == Some("serve it hot"));
        assert_eq!(s.kitchen.cooks[&id].completed, 2);
        until(&mut s, id, 4.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
        let start = s.kitchen.cooks[&id].patch.as_ref().unwrap().hot_at.unwrap();
        assert_eq!(s.kitchen.cooks[&id].completed, 3);
        run(&mut s, HOT + 0.1);
        assert!(s.kitchen.cooks[&id].patch.is_some());
        assert_eq!(s.kitchen.cooks[&id].completed, 3, "no fourth exchange during cooling");
        assert!(s.kitchen_pattern(id, 2.0, 0.4).is_some(), "AI selection stays defined while the beat owns cooling");
        scoop(&mut s, id);
        assert!(s.clock >= start + HOT + COOL);
        assert!(s.kitchen.cooks[&id].patch.is_none());
        assert!(!s.mobs[&id].enraged && s.mobs[&id].phase == 0);
    }
}

#[test]
fn kitchen_pour_locks_one_patch_and_the_dry_side_is_safe_with_marks_hidden() {
    let (mut s, band, id) = fight(COOKIE, 2);
    until(&mut s, id, 20.0, |s, id| s.kitchen_move(id) == Some("serve it hot"));
    let patch = s.kitchen.cooks[&id].patch.as_ref().unwrap();
    let at = patch.at;
    assert_eq!(patch.props.len(), 5);
    assert!(patch.props.iter().all(|p| s.is_kitchen_prop(*p)));
    let dir = s.mobs[&id].facing;
    s.move_player(band[0], [at[0] - dir.sin() * 3.0, at[1] + dir.cos() * 3.0, at[2]], 0.0, Moving::default());
    s.move_player(band[1], at, 0.0, Moving::default());
    let hp = [s.players[&band[0]].health, s.players[&band[1]].health];
    run(&mut s, 1.0);
    assert_eq!(s.mobs[&id].facing, dir, "the pour cannot chase a sidestep");
    assert_eq!(s.players[&band[1]].health, hp[1], "warning steam is harmless");
    let mut launched = Vec::new();
    for _ in 0..30 {
        launched.extend(s.step(DT));
        if s.kitchen.cooks[&id].patch.as_ref().unwrap().hot_at.is_some() { break; }
    }
    assert_eq!(launched.iter().filter(|e| matches!(e, Event::Missile { caster, spell: MUG, target: Target::Point(p) } if *caster == id && *p == at)).count(), 1, "one native mug flies to the committed floor point");
    assert!(s.kitchen.cooks[&id].patch.as_ref().unwrap().hot_at.unwrap() >= s.clock + 0.14);
    let flight = run(&mut s, 0.1);
    assert!(!flight.iter().any(|e| matches!(e, Event::Swing { attacker, damage, .. } if *attacker == id && *damage > 0)), "the mug travels before the spill can burn");
    let ev = run(&mut s, HOT + 0.3);
    assert!(!ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(h), damage, .. } if *attacker == id && *h == band[0] && *damage > 0)), "dry side requires only movement");
    assert!(s.players[&band[1]].health < hp[1]);
    let patch = s.kitchen.cooks[&id].patch.as_ref().unwrap();
    assert_eq!(patch.at, at);
    assert_eq!(patch.props.len(), 1, "the last harmless wisp thins the old patch");
    let ev = run(&mut s, 0.8);
    assert!(!ev.iter().any(|e| matches!(e, Event::Swing { attacker, damage, .. } if *attacker == id && *damage > 0)), "cooling does not burn");
}

#[test]
fn kitchen_posture_break_from_plain_light_attacks_spills_without_healing_or_breath() {
    let (mut s, band, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    let at = s.mobs[&id].pos;
    s.move_player(band[0], [at[0] + 1.1, at[1], at[2]], std::f32::consts::PI, Moving::default());
    let before = s.mobs[&id].health;
    let end = s.clock + SCOOP;
    let mut events = Vec::new();
    while s.kitchen_scooping(id) && s.clock < end {
        s.input(band[0], Input::Attack);
        events.extend(s.step(DT));
    }
    assert!(s.mobs[&id].health < before, "starting weapon attacks actually connected");
    assert!(events.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text == "MRGLGLGL!")));
    assert_eq!(s.kitchen.cooks[&id].healed, 0);
    assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { until }) if until - s.clock >= COUGH - DT));
    let events = run(&mut s, 3.5);
    assert!(!events.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)));
}

#[test]
fn kitchen_successful_scoop_heals_twice_at_most_then_commits_a_short_cone() {
    let (mut s, band, id) = fight(COOKIE, 1);
    let max = s.mobs[&id].max_health;
    s.mobs.get_mut(&id).unwrap().health = max / 2;
    let original = s.mobs[&id].health;
    for turn in 0..4 {
        scoop(&mut s, id);
        let at = s.mobs[&id].pos;
        s.move_player(band[0], [at[0] + 2.5, at[1], at[2]], 0.0, Moving::default());
        until(&mut s, id, SCOOP + 1.0, |s, id| s.kitchen_move(id) == Some("steam"));
        assert_eq!(s.mobs[&id].health, original + (turn + 1).min(2) * (max / 25));
        let dir = s.mobs[&id].facing;
        s.move_player(band[0], [at[0] - dir.sin() * 3.0, at[1] + dir.cos() * 3.0, at[2]], 0.0, Moving::default());
        let hp = s.players[&band[0]].health;
        run(&mut s, 1.8);
        assert_eq!(s.players[&band[0]].health, hp);
        assert_eq!(s.mobs[&id].facing, dir);
        assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })));
        run(&mut s, 4.1);
    }
    assert_eq!(s.kitchen.cooks[&id].healed, 2);
}

#[test]
fn kitchen_waiting_out_every_scoop_still_dies_to_a_plain_starting_weapon_solo_and_duo() {
    for heroes in [1, 2] {
        let (mut s, band, id) = fight(COOKIE, heroes);
        let mut scoops = 0;
        let mut spill = false;
        let mut attacks = 0;
        let mut damage_taken = 0;
        for _ in 0..8000 {
            if s.mobs[&id].health == 0 { break; }
            // Let every scoop complete. Attack the ordinary recoveries after the first taste.
            if scoops > 0 && s.kitchen.cooks[&id].stage.is_none()
                && matches!(s.mobs[&id].state, MobState::Recover { .. }) {
                let at = s.mobs[&id].pos;
                for (n, h) in band.iter().enumerate() {
                    s.move_player(*h, [at[0] + 1.2, at[1] + n as f32 * 0.3, at[2]], std::f32::consts::PI, Moving::default());
                    s.input(*h, Input::Attack);
                    attacks += 1;
                }
            }
            let ev = s.step(DT);
            damage_taken += ev.iter().filter_map(|e| match e {
                Event::Swing { attacker, victim: Some(h), damage, .. } if *attacker == id && band.contains(h) => Some(*damage),
                _ => None,
            }).sum::<u32>();
            scoops += ev.iter().filter(|e| matches!(e, Event::Yell { id: who, text } if *who == id && (text == "Mmm... one taste." || text == "Mrrgl... scraps."))).count();
            spill |= ev.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text == "MRGLGLGL!"));
        }
        assert!(attacks > 0 && scoops > 0);
        assert!(!spill, "the controller did not interrupt a scoop");
        assert_eq!(s.mobs[&id].health, 0, "waiting remained winnable at {} sec", s.clock);
        println!("Cookie {heroes} hero starting-weapon controller: {:.2} sec, {scoops} scoops, {damage_taken} total damage taken (inflated hero health)", s.clock);
    }
}

#[test]
fn kitchen_wipe_death_and_reset_clear_each_owned_prop_once_and_repull_starts_empty() {
    for end in [0, 1, 2] {
        let (mut s, band, id) = fight(COOKIE, 1);
        until(&mut s, id, 22.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
        let mut props: Vec<_> = s.objs.keys().copied().filter(|p| s.is_kitchen_prop(*p)).collect();
        let pot = s.kitchen.cooks[&id].pot.unwrap();
        assert!(!s.is_kitchen_prop(pot), "solid cauldron keeps its normal collision");
        props.push(pot);
        s.events.clear();
        match end {
            0 => s.hurt_mob(id, band[0], u32::MAX),
            1 => { s.players.get_mut(&band[0]).unwrap().health = 0; s.step_kitchen(id); }
            _ => s.reset_kitchens(),
        }
        s.remove_kitchen(id);
        for p in props {
            assert!(!s.objs.contains_key(&p));
            assert_eq!(s.events.iter().filter(|e| matches!(e, Event::ObjGone { id: who } if *who == p)).count(), 1);
        }
        assert!(!s.kitchen.cooks.contains_key(&id));
        if end != 0 {
            s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
            s.clean_kitchens();
            assert_eq!(s.kitchen.cooks[&id].completed, 0);
            assert_eq!(s.kitchen.cooks[&id].healed, 0);
            assert!(s.kitchen.cooks[&id].patch.is_none());
        }
    }
}

#[test]
fn gorged_inhales_then_commits_belch_and_coughs_without_health_line_adds() {
    for low in [false, true] {
        let (mut s, band, id) = fight(GORGED, 2);
        if low { s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health / 5; }
        // Every nearby hero avoids the first slap so mercy does not truncate the pair.
        move_beside_locked_blow(&mut s, &band, id);
        until(&mut s, id, 15.0, |s, id| s.kitchen_move(id) == Some("belch"));
        let (at, dir) = (s.mobs[&id].pos, s.mobs[&id].facing);
        s.move_player(band[0], [at[0] - dir.sin() * 3.0, at[1] + dir.cos() * 3.0, at[2]], 0.0, Moving::default());
        s.move_player(band[1], [at[0] + dir.cos() * 4.0, at[1] + dir.sin() * 4.0, at[2]], 0.0, Moving::default());
        let hp = [s.players[&band[0]].health, s.players[&band[1]].health];
        run(&mut s, 1.6);
        assert_eq!(s.players[&band[1]].health, hp[1], "the inhale is a long warning");
        run(&mut s, 0.8);
        assert_eq!(s.players[&band[0]].health, hp[0]);
        assert!(s.players[&band[1]].health < hp[1]);
        assert_eq!(s.mobs[&id].facing, dir);
        assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { until }) if until - s.clock > 3.5));
        assert_eq!(s.mobs.len(), 1);
        assert!(s.content.mobs[s.mobs[&id].def].phases.is_empty());
        run(&mut s, 3.4);
        assert!(s.mobs[&id].pattern.is_none());
    }
}

struct NoRoute;
impl Land for NoRoute {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn plans(&self) -> bool { true }
    fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
}

#[test]
fn kitchen_unreachable_pot_gives_up_without_healing_or_teleporting() {
    let (mut s, _, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    s.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Seeking { until: s.clock + 1.0 });
    s.mobs.get_mut(&id).unwrap().pos = [8.0, 0.0, 0.0];
    s.land = Box::new(NoRoute);
    let hp = s.mobs[&id].health;
    run(&mut s, 1.2);
    assert_eq!(s.mobs[&id].pos, [8.0, 0.0, 0.0]);
    assert_eq!(s.mobs[&id].health, hp);
    assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })));
}

#[test]
fn kitchen_secret_handoff_clears_old_props_before_gorged_rises() {
    let (mut s, band, id) = fight(COOKIE, 1);
    let item = s.content.item("secret_ingredient").unwrap();
    s.give(band[0], item, 1);
    until(&mut s, id, 22.0, |s, id| s.kitchen.cooks[&id].patch.is_some());
    let pot = s.kitchen.cooks[&id].pot.unwrap();
    let props = s.kitchen.cooks[&id].patch.as_ref().unwrap().props.clone();
    s.hurt_mob(id, band[0], u32::MAX);
    assert!(!s.objs.contains_key(&pot) && props.iter().all(|p| !s.objs.contains_key(p)));
    assert!(!s.kitchen.cooks.contains_key(&id));
    let next = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == GORGED).unwrap().0;
    assert!(s.mobs[&next].hidden);
    run(&mut s, 3.3);
    assert!(!s.kitchen.cooks.contains_key(&next));
    run(&mut s, 0.4);
    assert!(!s.mobs[&next].hidden);
    assert!(s.kitchen.cooks[&next].pot.is_none());
    assert_eq!(s.kitchen.cooks[&next].completed, 0);
}

#[test]
fn kitchen_actual_dungeon_wipe_restores_one_cook_and_fresh_heal_budget() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == "cookie");
    d.packs[0].at = [0.0, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [0.0, -20.0, 0.0];
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), 19, 1_000_000.0);
    let id = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == COOKIE).unwrap().0;
    let h = s.join("wipe", "Wipe", None);
    s.move_player(h, [2.5, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    s.players.get_mut(&h).unwrap().health = 100_000;
    s.players.get_mut(&h).unwrap().max_health = 100_000;
    s.engage(id, Foe::Hero(h));
    s.step(DT);
    scoop(&mut s, id);
    run(&mut s, SCOOP + 0.1);
    assert_eq!(s.kitchen.cooks[&id].healed, 1);
    let old_pot = s.kitchen.cooks[&id].pot.unwrap();
    s.hurt_player_by(h, u32::MAX, id, "test wipe");
    run(&mut s, 0.1);
    assert!(!s.objs.contains_key(&old_pot));
    let alive: Vec<_> = s.mobs.iter().filter(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == COOKIE).collect();
    assert_eq!(alive.len(), 1);
    assert_eq!(alive[0].1.health, alive[0].1.max_health);
    assert!(!s.slain("cookie"));
    let fresh = *alive[0].0;
    s.clean_kitchens();
    assert_eq!(s.kitchen.cooks[&fresh].healed, 0);
    assert!(s.kitchen.cooks[&fresh].patch.is_none());
}

#[test]
fn kitchen_cancelled_warning_and_departed_party_leave_no_late_damage() {
    let (mut s, band, id) = fight(COOKIE, 1);
    until(&mut s, id, 22.0, |s, id| s.kitchen.cooks[&id].patch.is_some());
    let props = s.kitchen.cooks[&id].patch.as_ref().unwrap().props.clone();
    s.stagger_mob(id, 1.0, false);
    run(&mut s, DT);
    assert!(s.kitchen.cooks[&id].patch.is_none());
    assert!(props.iter().all(|p| !s.objs.contains_key(p)));
    until(&mut s, id, 12.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
    s.move_player(band[0], [100.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, DT);
    assert!(!s.kitchen.cooks.contains_key(&id));
    assert!(s.pending.iter().all(|p| p.attacker != id));
    assert!(matches!(s.mobs[&id].state, MobState::Return));
}

#[test]
fn kitchen_short_stagger_delays_the_gulp_but_only_posture_break_spills() {
    let (mut s, _, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    s.stagger_mob(id, SCOOP + 1.0, false);
    let ev = run(&mut s, SCOOP + 0.2);
    assert!(s.kitchen_scooping(id));
    assert_eq!(s.kitchen.cooks[&id].healed, 0);
    assert!(!ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "MRGLGLGL!")));
    run(&mut s, 1.0);
    assert_eq!(s.kitchen.cooks[&id].healed, 1);
    assert_eq!(s.kitchen_move(id), Some("steam"));
}
