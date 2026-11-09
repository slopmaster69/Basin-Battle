use super::*;
use crate::land::FlatLand;

const DT: f32 = 0.01;

/// Stage one authored pound step in the production final phase; resolve it through the real
/// windup/release and fair-hit paths. Route geometry and phase-entry setup are unrelated here.
fn pound(step: usize, rtt: f32) -> (Survival, Id, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 19, 1_000_000.0);
    let h = s.join("lodge", "Lodge", None);
    s.players.get_mut(&h).unwrap().loadout.clear();
    s.move_player(h, [2.5, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    s.set_rtt(h, rtt);
    let def = s.content.mob("dm_rhahkzor").unwrap();
    let k = s.content.mobs[def].patterns.iter().position(|p| p.name == "pound").unwrap();
    let id = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.phase = 4;
    m.health = 200;
    m.pattern = Some((k, step));
    s.windup(id, mobs::Foe::Hero(h), s.players[&h].pos);
    (s, h, id)
}

fn released(s: &mut Survival, id: Id) -> (f32, Vec<Event>) {
    let end = s.clock + 2.0;
    let mut events = Vec::new();
    while matches!(s.mobs[&id].state, MobState::Windup { .. }) && s.clock < end {
        events.extend(s.step(DT));
    }
    assert!(matches!(s.mobs[&id].state, MobState::Recover { .. }), "actual step released");
    (s.clock, events)
}

fn full_lodge(rtt: f32) {
    for answer in ["hit", "miss", "blocked", "guard break"] {
        let (mut s, h, id) = pound(2, rtt);
        let expected = match answer {
            "miss" => {
                s.move_player(h, [9.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
                Outcome::Miss
            }
            "blocked" | "guard break" => {
                // A real stronger shield can block the final pound; the starting buckler breaks.
                if answer == "blocked" {
                    s.players.get_mut(&h).unwrap().shield = s.content.item("captain_shield").unwrap();
                }
                s.input(h, Input::Guard);
                if answer == "guard break" { s.players.get_mut(&h).unwrap().stamina = 1.0; }
                if answer == "blocked" { Outcome::Blocked } else { Outcome::GuardBroken }
            }
            _ => Outcome::Hit,
        };
        let (at, mut ev) = released(&mut s, id);
        assert!(s.mobs[&id].stagger_until >= at + 3.0 - DT,
            "authored lodge survives {answer}, RTT {rtt}: stagger until {}, release {at}", s.mobs[&id].stagger_until);
        assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until >= at + 3.0 - DT),
            "ordinary recovery must not replace the lodge, {answer}, RTT {rtt}");
        let start = s.mobs[&id].pos;
        while s.clock < at + 3.0 - DT {
            let events = s.step(DT);
            assert!(!events.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)),
                "no attack while lodged, {answer}, RTT {rtt}, elapsed {}", s.clock-at);
            assert_eq!(s.mobs[&id].pos, start, "the lodged boss stays planted");
            ev.extend(events);
        }
        assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome, .. }
            if *attacker == id && *v == h && *outcome == expected)), "actual {answer} resolves at RTT {rtt}");
        let mut next = None;
        for _ in 0..500 {
            if s.step(DT).iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)) {
                next = Some(s.clock);
                break;
            }
        }
        assert!(next.is_some_and(|t| t >= at + 3.0 - DT), "fighting resumes after the full lodge");
    }
}

#[test]
fn boss_lodge_survives_immediate_hit_mercy_and_normal_defenses() { full_lodge(0.0); }

#[test]
fn boss_lodge_outlasts_normal_recovery_with_network_grace() {
    for rtt in [0.1, 0.35] { full_lodge(rtt); }
}

#[test]
fn boss_lodge_preserves_a_longer_normal_slam_recovery() {
    let (mut s, h, id) = pound(2, 0.1);
    let def = s.mobs[&id].def;
    let k = s.content.mobs[def].patterns.iter().position(|p| p.name == "slam").unwrap();
    s.mobs.get_mut(&id).unwrap().pattern = Some((k, 0));
    s.windup(id, mobs::Foe::Hero(h), s.players[&h].pos);
    let ordinary = s.attack_now(id).recovery;
    assert!(ordinary > 1.2);
    s.move_player(h, [9.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let (at, _) = released(&mut s, id);
    assert!((s.mobs[&id].stagger_until - at - 1.2).abs() < DT, "the authored short lodge stays 1.2 seconds");
    assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until >= at + ordinary - DT),
        "a shorter lodge cannot shorten normal recovery");
}

#[test]
fn boss_lodge_does_not_fire_for_earlier_mercy_cutoffs_or_interrupted_last_windup() {
    for rtt in [0.0, 0.1, 0.35] {
        for step in [0, 1] {
            let (mut s, h, id) = pound(step, rtt);
            let (_, mut ev) = released(&mut s, id);
            for _ in 0..40 { ev.extend(s.step(DT)); }
            assert!(ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::Hit, .. }
                if *attacker == id && *v == h)));
            assert!(matches!(s.players[&h].action, Action::Stagger { .. }), "early hit still staggers");
            assert!(s.mobs[&id].pattern.is_none(), "mercy still cancels the unfinished pound");
            assert!(s.mobs[&id].stagger_until <= s.clock, "no final lodge without its released step");
        }
        let (mut s, _, id) = pound(2, rtt);
        s.stagger_mob(id, 0.6, false);
        let ev: Vec<_> = (0..100).flat_map(|_| s.step(DT)).collect();
        assert!(!ev.iter().any(|e| matches!(e, Event::React { id: who, hold, .. } if *who == id && *hold > 0.61)),
            "interrupting the final windup does not manufacture its lodge");
    }
}
