use super::super::tests::{broken, content, emerged, fight, live, run, timed, working, yells, DT};
use super::*;
use crate::land::FlatLand;

fn rig(s: &Survival) -> ShredderRig {
    s.shredder_rig().expect("the post's rig")
}

fn put(s: &mut Survival, h: Id, at: Vec3, toward: Vec3) {
    s.move_player(h, at, facing(at, toward), Moving::default());
}

fn props(s: &Survival, display: u32) -> usize {
    s.objs.values().filter(|o| matches!(o.kind, ObjKind::Prop { display: d, .. } if d == display)).count()
}

fn stunned(s: &Survival, h: Id) -> Option<f32> {
    s.cc_of(h).iter().filter(|c| c.kind == "stun" && c.until > s.clock).map(|c| c.until - s.clock).reduce(f32::max)
}

/// The hero steps to the tie-off, a swing's reach outside it, and cuts it with one plain blow.
fn cut(s: &mut Survival, h: Id) -> Vec<(f32, Event)> {
    let g = rig(s);
    let tie = g.tie.expect("a tie-off");
    let mid = off(g.crash, g.across, g.len / 2.0);
    let stand = off(tie, facing(mid, tie), 1.5);
    put(s, h, stand, tie);
    s.input(h, Input::Attack);
    let mut out = Vec::new();
    for _ in 0..40 {
        let ev = s.step(DT);
        let t = s.clock;
        out.extend(ev.into_iter().map(|e| (t, e)));
        if rig(s).state != "hung" {
            return out;
        }
        put(s, h, stand, tie);
    }
    panic!("the blow on the tie-off cut nothing");
}

/// Steps until the cut log is down.
fn landed(s: &mut Survival) -> Vec<(f32, Event)> {
    let mut out = Vec::new();
    for _ in 0..60 {
        let ev = s.step(DT);
        let t = s.clock;
        out.extend(ev.into_iter().map(|e| (t, e)));
        if rig(s).state == "down" {
            return out;
        }
    }
    panic!("the cut log never came down");
}

/// As `fight`, with the heroes at their own health.
fn mortal(heroes: usize, seed: u64, foe: usize) -> (Survival, Vec<Id>, Id) {
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), seed, 1_000_000.0);
    let machine = live(&s, MACHINE).unwrap();
    let band: Vec<Id> = (0..heroes).map(|n| {
        let h = s.join(&format!("hero{n}"), &format!("Hero{n}"), None);
        s.move_player(h, [4.0, n as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
        h
    }).collect();
    s.engage(machine, Foe::Hero(band[foe]));
    (s, band, machine)
}

fn lost(s: &mut Survival, h: Id, by: Id) {
    s.hurt_player_by(h, u32::MAX, by, "test wipe");
}

#[test]
fn the_log_hangs_over_the_post_from_the_pull_its_drop_marked_and_its_tie_off_outside_it() {
    // Before the pull: hung, its ropes strung, nothing marked.
    let mut s = Survival::new_dungeon(content(), Box::new(FlatLand::default()), 3, 1_000_000.0);
    let h = s.join("hero0", "Hero0", None);
    s.move_player(h, [0.0, -40.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 1.0);
    assert_eq!(rig(&s).state, "hung");
    assert_eq!(ev.iter().filter(|e| matches!(e, Event::Rope { .. })).count(), 3, "one rope to the stake, two slings");
    assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { .. })), "no mark before the pull");

    let (mut s, _, _) = fight(1, 3);
    let ev = run(&mut s, 1.2);
    let g = rig(&s);
    let (work, tie, crawl) = (g.work.unwrap(), g.tie.unwrap(), g.crawl.unwrap());
    assert_eq!((props(&s, LOG), props(&s, STAKE)), (LOG_BUNDLE.len(), 1), "one log (three poles slung together), one tie-off");
    let log = g.log.unwrap();
    assert!(s.objs[&log].pos[2] - g.home[2] >= 5.5, "over the machine's head");
    assert!(s.collapse_piece_tilt(log).is_some() && s.collapse_piece_tilt(g.stake.unwrap()).is_some(), "drawn with no hull");
    // The wreck's place is the drop's start; Sneed works and comes out under the log; the tie-off
    // is well outside it, a few yards from where he works.
    assert!(dist2d(g.crash, off(g.home, g.face, CRASH_AHEAD)) < 0.1);
    assert!(combat::in_strip(g.crash, g.across, g.len, 0.3, work), "he works under the log's own line");
    assert!(combat::in_strip(g.crash, g.across, g.len, g.half, crawl));
    assert!(dist2d(crawl, g.crash) > 3.5, "out of the wreck's middle");
    assert!(!combat::in_strip(g.crash, g.across, g.len, g.half + 2.5, tie), "whoever cuts it stands clear");
    assert!((3.0..=9.0).contains(&dist2d(work, tie)), "{}", dist2d(work, tie));
    // From the pull: the drop on the floor, quiet, its whole length; a ring at the tie-off.
    let lines: Vec<f32> = ev.iter().filter_map(|e| match e {
        Event::GroundBurst { shape: "line", yaw, reach, width, colour, .. } if colour == QUIET && (*yaw - g.across).abs() < 1e-3 && (*width - 2.0 * g.half).abs() < 1e-3 => Some(*reach),
        _ => None,
    }).collect();
    assert!(lines.len() >= 2, "sent again while the fight is on");
    assert!((lines[0] - g.len).abs() < 0.3, "{lines:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::GroundBurst { shape: "ring", at, colour, .. } if colour == QUIET && dist2d(*at, tie) < 0.1)));
    assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { colour, .. } if colour == BRIGHT)));
}

#[test]
fn the_broken_machine_runs_back_to_its_post_and_crashes_into_the_pile_hurting_nobody() {
    for (far, least, most) in [(true, SPUTTER + 1.0, SPUTTER + RUN_BACK + 0.8), (false, SPUTTER + 0.3, SPUTTER + 1.0)] {
        let (mut s, band, machine) = fight(2, 5);
        run(&mut s, 3.0);
        let g = rig(&s);
        let from = if far { [18.0, -14.0, 0.0] } else { g.home };
        let m = s.mobs.get_mut(&machine).unwrap();
        m.pos = from;
        m.path.clear();
        // One hero beside it, the other standing where it will crash.
        put(&mut s, band[0], [from[0] + 3.5, from[1], 0.0], from);
        put(&mut s, band[1], g.crash, g.home);
        let before: Vec<(u32, Vec3)> = band.iter().map(|h| (s.players[h].health, s.players[h].pos)).collect();
        let t0 = s.clock;
        s.hurt_mob(machine, band[0], u32::MAX);
        assert!(s.machinery_runaway(machine) && s.mobs[&machine].health == 1, "broken, not dead yet");
        s.hurt_mob(machine, band[0], 500);
        assert_eq!(s.mobs[&machine].health, 1, "nothing reaches it on its run");
        assert_eq!(s.posture_of(machine), 0);
        let mut ev = Vec::new();
        while live(&s, DRIVER).is_none() {
            assert!(s.clock - t0 < 8.0, "it never crashed");
            let step = s.step(DT);
            let t = s.clock;
            ev.extend(step.into_iter().map(|e| (t, e)));
        }
        let took = s.clock - t0;
        assert!((least..=most).contains(&took), "{took} s from the break to the crash");
        assert!(yells(&ev).iter().any(|(t, text)| text == LINE_BREAK && *t - t0 < 0.2), "its line as it breaks");
        let moved = ev.iter().find_map(|(t, e)| match e { Event::MobMove { id, speed, .. } if *id == machine => Some((*t, *speed)), _ => None }).expect("it runs");
        assert!(moved.0 - t0 >= SPUTTER - 0.06, "it churns where it stands first");
        assert!((RUN_SLOWEST..=RUN_FASTEST).contains(&moved.1), "{}", moved.1);
        // Smoke and sparks the whole way; the crash is a bang, dust and lumber.
        assert!(ev.iter().filter(|(_, e)| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)).count() >= 5);
        assert!(ev.iter().any(|(_, e)| matches!(e, Event::Sound { id, sound: WOOD_SOUND } if *id == machine)));
        assert!(ev.iter().any(|(_, e)| matches!(e, Event::Sound { id, sound: THUD_SOUND } if *id == machine)));
        assert!(ev.iter().any(|(_, e)| matches!(e, Event::Visual { id, kit: combat::KIT_DUST, .. } if *id == machine)));
        assert_eq!(rig(&s).lumber.len(), 3, "lumber off the pile");
        // The wreck lies under the log, the way the machine faced at its work.
        let wreck = &s.mobs[&machine];
        assert!(wreck.health == 0 && dist2d(wreck.pos, g.crash) < 0.4, "{:?} vs {:?}", wreck.pos, g.crash);
        assert!((wreck.facing - g.face).abs() < 1e-3);
        // Nobody hit, nobody shoved, nobody held: not the hero beside it, not the one at the pile.
        assert!(!ev.iter().any(|(_, e)| matches!(e, Event::Swing { attacker, .. } if *attacker == machine)), "the run is no attack");
        for (h, was) in band.iter().zip(&before) {
            assert_eq!((s.players[h].health, s.players[h].pos), *was);
            assert!(stunned(&s, *h).is_none());
        }
    }
}

#[test]
fn sneed_crawls_out_of_the_wreck_in_view_three_seconds_before_he_acts_and_nothing_reaches_him_till_he_is_up() {
    let (mut s, band, machine) = fight(1, 2);
    run(&mut s, 3.0);
    let driver = broken(&mut s, machine, band[0]);
    let g = rig(&s);
    let t1 = s.clock;
    assert!(s.mobs[&driver].hidden, "inside the machine as it collapses");
    let hp = s.mobs[&driver].health;
    let lumber = g.lumber.len();
    let (mut out, mut ev) = (None, Vec::new());
    for _ in 0..((COLLAPSE + LIE + RISE + 4.0) / DT) as usize {
        let step = s.step(DT);
        let t = s.clock;
        if out.is_none() && step.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == driver)) {
            out = Some((t, s.mobs[&driver].pos));
        }
        if s.machinery.drivers.get(&driver).is_some_and(|d| d.emerge.is_some()) {
            s.hurt_mob(driver, band[0], 50);
            assert_eq!(s.mobs[&driver].health, hp, "nothing reaches him until he's up");
        }
        ev.extend(step.into_iter().map(|e| (t, e)));
    }
    let (out, at) = out.expect("he comes out");
    assert!(out - t1 >= COLLAPSE - 0.06, "once the machine has fallen");
    // On the open floor beside the wreck, lying in its scrap.
    assert!(dist2d(at, g.crawl.unwrap()) < 0.1 && dist2d(at, g.crash) > 3.5);
    assert!(ev.iter().any(|(t, e)| matches!(e, Event::HoldPose { id, clip: LIE_CLIP, .. } if *id == driver && (*t - out).abs() < 0.06)), "lying in it");
    assert!(ev.iter().any(|(t, e)| matches!(e, Event::MissileFrom { spell: DUST, from, .. } if dist2d(*from, at) < 0.1 && (*t - out).abs() < 0.06)), "scrap and dust burst off it");
    assert!(ev.iter().any(|(t, e)| matches!(e, Event::Sound { id, sound: WOOD_SOUND } if *id == driver && (*t - out).abs() < 0.06)));
    assert_eq!(rig(&s).lumber.len(), lumber + 1, "a plank thrown clear");
    // He gets up, then speaks, then fights.
    let up = ev.iter().find(|(_, e)| matches!(e, Event::React { id, anim: RISE_CLIP, fit, .. } if *id == driver && (*fit - RISE).abs() < 1e-3)).map(|(t, _)| *t).expect("he gets up");
    assert!((up - out - LIE).abs() < 0.11);
    let said = yells(&ev).iter().find(|(_, t)| t == "That was company property!").map(|(t, _)| *t).expect("his line");
    assert!(said - out >= LIE + RISE - 0.06, "in view {} s before his line", said - out);
    let first = ev.iter().find(|(_, e)| matches!(e, Event::MobWindup { id, .. } | Event::Cast { caster: id, .. } if *id == driver)).map(|(t, _)| *t);
    assert!(first.is_none_or(|t| t - said >= FIRST_BLOW - 0.06), "his first blow waits for his line");
    assert!(!ev.iter().any(|(t, e)| *t < said && matches!(e, Event::MobMove { id, .. } if *id == driver)), "he goes nowhere before it");
    assert!(ev.iter().any(|(_, e)| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)), "the wreck smokes");
}

#[test]
fn a_blow_on_the_tie_off_cuts_it_the_log_creaks_and_comes_down_once() {
    let (mut s, band, _) = fight(1, 4);
    run(&mut s, 0.5);
    let g = rig(&s);
    let (log, stake) = (g.log.unwrap(), g.stake.unwrap());
    let top = s.objs[&log].pos[2];
    let mut ev = cut(&mut s, band[0]);
    let t0 = s.clock;
    assert_eq!(rig(&s).state, "cut");
    assert!(ev.iter().any(|(_, e)| matches!(e, Event::Sound { id, sound: CREAK_SOUND } if *id == log)), "it creaks");
    assert!(ev.iter().any(|(_, e)| matches!(e, Event::ObjChanged { id } if *id == stake)), "the stake is knocked flat");
    assert!(ev.iter().filter(|(_, e)| matches!(e, Event::MarkGone { .. })).count() >= 1, "the rope parts");
    let fall = landed(&mut s);
    let took = s.clock - t0;
    assert!((took - CREAK - FALL).abs() < 0.11, "{took}");
    // Bright on the floor from the cut until it lands; it hangs through the creak, then falls.
    assert!(ev.iter().chain(&fall).filter(|(_, e)| matches!(e, Event::GroundBurst { shape: "line", colour, .. } if colour == BRIGHT)).count() >= 3);
    assert!(ev.iter().any(|(_, e)| matches!(e, Event::GroundBurst { shape: "line", colour, .. } if colour == BRIGHT)), "bright from the blow itself");
    let moved: Vec<f32> = fall.iter().filter(|(_, e)| matches!(e, Event::ObjChanged { id } if *id == log)).map(|(t, _)| *t - t0).collect();
    assert!(moved.len() >= 3 && moved[0] >= CREAK - 0.11, "{moved:?}");
    assert!(top - s.objs[&log].pos[2] > 5.0, "down on the floor");
    assert!(fall.iter().any(|(_, e)| matches!(e, Event::Sound { id, sound: THUD_SOUND } if *id == log)));
    assert!(stunned(&s, band[0]).is_none(), "the hero at the tie-off stands clear");
    assert!(s.collapse_piece_tilt(log).is_some(), "it lies with no hull: it walls nobody in");
    ev.extend(fall);
    // One log, one use: another blow on the stake does nothing, and the floor's mark is gone.
    let tie = g.tie.unwrap();
    let stand = off(tie, g.face, 1.5);
    put(&mut s, band[0], stand, tie);
    s.input(band[0], Input::Attack);
    let after = run(&mut s, 2.0);
    assert_eq!(rig(&s).state, "down");
    assert!(!after.iter().any(|e| matches!(e, Event::Sound { sound: CREAK_SOUND, .. })));
    assert!(!after.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", .. })), "no mark for a log that's down");
    assert_eq!(props(&s, LOG), LOG_BUNDLE.len());
}

#[test]
fn a_hero_in_the_drop_who_moves_at_the_creak_is_out_either_side_and_one_who_stays_is_stunned_2_s_unhurt() {
    for side in [1.0f32, -1.0] {
        let (mut s, band, machine) = fight(3, 6);
        run(&mut s, 0.5);
        let g = rig(&s);
        // (The machine out of it: this is the log's own cost.)
        s.mobs.get_mut(&machine).unwrap().pos = off(g.home, g.face, -80.0);
        let mid = off(g.crash, g.across, g.len / 2.0);
        put(&mut s, band[1], g.work.unwrap(), g.crash);
        put(&mut s, band[2], mid, g.crash);
        let hp = s.players[&band[1]].health;
        cut(&mut s, band[0]);
        let (t0, pace) = (s.clock, s.content.player.run_speed);
        while rig(&s).state == "cut" {
            // band[2] sees it 0.3 s late, then runs straight out across the drop.
            let gone = ((s.clock - t0 - 0.3).max(0.0) * pace).min(4.0);
            put(&mut s, band[2], off(mid, g.face, side * gone), g.crash);
            s.step(DT);
        }
        assert!(stunned(&s, band[2]).is_none(), "out in time, side {side}");
        assert!(stunned(&s, band[0]).is_none());
        let held = stunned(&s, band[1]).expect("under it: stunned");
        assert!((held - LOG_STUN * s.content.cc.player_share).abs() < 0.11, "{held}");
        assert!((held - 2.0).abs() < 0.11, "2 s for a hero");
        assert_eq!(s.players[&band[1]].health, hp, "the log itself hurts no hero");
        run(&mut s, held + 0.1);
        assert!(stunned(&s, band[1]).is_none(), "and up again");
    }
}

#[test]
fn the_log_on_sneed_at_his_repair_stops_it_for_good_he_says_so_and_is_open_4_s() {
    let (mut s, band, machine) = fight(1, 13);
    let driver = broken(&mut s, machine, band[0]);
    working(&mut s, driver);
    let hp = s.mobs[&driver].health;
    cut(&mut s, band[0]);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }), "he hammers on through the creak");
    let mut ev = landed(&mut s);
    let t0 = s.clock;
    let d = &s.machinery.drivers[&driver];
    assert!(matches!(d.stage, RepairStage::Spent) && d.wreck.is_none() && d.quiet == Some(machine));
    assert_eq!(s.mobs[&driver].health, hp, "the log stuns him, it doesn't wound him");
    assert!(s.mobs[&driver].stagger_until - t0 >= LOG_STUN - 0.06, "flattened 4 s");
    assert!(ev.iter().any(|(_, e)| matches!(e, Event::CastEnd { caster } if *caster == driver)));
    ev.extend(timed(&mut s, LOG_STUN - 0.2));
    assert!(yells(&ev).iter().any(|(t, text)| text == LINE_LOG && *t - t0 <= LINE_GAP + 0.06), "he says what happened");
    assert!(!ev.iter().any(|(t, e)| *t > t0 && matches!(e, Event::MobWindup { id, .. } | Event::Cast { caster: id, .. } | Event::MobMove { id, .. } if *id == driver)), "open: he does nothing");
    assert!(!ev.iter().any(|(t, e)| *t > t0 + 0.06 && matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)), "the wreck goes quiet");
    // A hero's blow lands on him there.
    let at = s.mobs[&driver].pos;
    let face = g_face(&s);
    put(&mut s, band[0], off(at, face, 1.6), at);
    s.input(band[0], Input::Attack);
    run(&mut s, 1.0);
    assert!(s.mobs[&driver].health < hp, "hit while he's down");
    // Then he fights on foot, and never mends it again.
    let ev = run(&mut s, 14.0);
    assert!(ev.iter().any(|e| matches!(e, Event::MobWindup { id, .. } | Event::Cast { caster: id, .. } if *id == driver)), "he fights on");
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
    assert!(live(&s, MACHINE).is_none() && matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_infinite()), "the wreck stays, quiet");
}

fn g_face(s: &Survival) -> f32 {
    rig(s).face
}

#[test]
fn a_plain_hit_hurts_sneed_at_his_repair_and_does_not_stop_it_a_stagger_does_and_he_says_so() {
    let (mut s, band, machine) = fight(1, 17);
    let h = band[0];
    s.players.get_mut(&h).unwrap().loadout.clear();
    let driver = broken(&mut s, machine, h);
    working(&mut s, driver);
    let at = s.mobs[&driver].pos;
    let face = g_face(&s);
    put(&mut s, h, off(at, face, 1.4), at);
    let hp = s.mobs[&driver].health;
    for _ in 0..2 {
        s.input(h, Input::Attack);
        run(&mut s, 1.2);
    }
    assert!(s.mobs[&driver].health < hp, "the ordinary swings landed");
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }), "and he hammers on");
    assert!(s.posture_of(driver) > 0, "each blow wears his stance");
    // His hands on the wreck, his stance is lower than on foot.
    assert_eq!(s.posture_limit(driver), MEND_POSTURE);
    assert!(MEND_POSTURE < s.content.mobs[s.mobs[&driver].def].posture);
    // A heavy blow wound up at him: he doesn't step back out of it (quick as he is on foot) and
    // drop his work with no blow landed.
    let def = s.mobs[&driver].def;
    s.content.mobs[def].evade = 1.0;
    s.input(h, Input::Heavy);
    let ev = run(&mut s, 1.6);
    assert!(!ev.iter().any(|e| matches!(e, Event::React { id, anim: 30, .. } if *id == driver)), "no step back");
    assert_eq!(s.mobs[&driver].pos, at);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }), "and he hammers on");
    s.stagger_mob(driver, 1.0, false);
    let ev = run(&mut s, LINE_GAP + 0.2);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
    assert!(s.machinery.drivers[&driver].wreck.is_none());
    assert_eq!(s.posture_limit(driver), s.content.mobs[s.mobs[&driver].def].posture, "his own stance again on foot");
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == driver && text == LINE_STOPPED)), "never given up without a word");
    // The wreck goes quiet where it lies: no more smoke or sparks, and it stays.
    let quiet = ev.iter().position(|e| matches!(e, Event::CastEnd { caster } if *caster == driver)).unwrap();
    assert!(!ev[quiet..].iter().any(|e| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine)));
    let lay = s.mobs[&machine].pos;
    let ev = run(&mut s, 18.0);
    assert!(matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_infinite()), "the wreck stays");
    assert_eq!(s.mobs[&machine].pos, lay, "where it lay");
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } | Event::Visual { id: caster, .. } | Event::MobGone { id: caster } if *caster == machine)), "quiet: no smoke, no sparks");
    assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
    assert!(live(&s, MACHINE).is_none() && !s.slain("sneed"));
    // Sneed down: the wreck and the crash's lumber go with his body; the log he never met hangs on.
    s.hurt_mob(driver, h, u32::MAX);
    assert!(s.slain("sneed"));
    assert!(matches!(s.mobs[&machine].state, MobState::Dead { gone_at } if gone_at.is_finite()));
    run(&mut s, BODY_SECS + 1.0);
    assert!(!s.mobs.contains_key(&machine) && !s.mobs.contains_key(&driver), "no ghost wreck");
    assert!(rig(&s).lumber.is_empty() && props(&s, LUMBER) + props(&s, PLANK) == 0, "no lumber left lying");
    assert_eq!((rig(&s).state, props(&s, LOG)), ("hung", LOG_BUNDLE.len()));
}

#[test]
fn the_repair_left_alone_is_seen_coming_on_and_stands_the_machine_up_and_its_second_break_ends_in_the_same_place() {
    for heroes in [1, 2] {
        let (mut s, band, machine) = fight(heroes, 13);
        let driver = broken(&mut s, machine, band[0]);
        emerged(&mut s, driver);
        assert!(!s.slain("sneed"));
        s.hurt_mob(driver, band[0], 20);
        let driver_hp = s.mobs[&driver].health;
        let g = rig(&s);
        let wreck = s.mobs[&machine].pos;
        // The heroes stand well off and watch.
        for (n, h) in band.iter().enumerate() {
            put(&mut s, *h, off(off(g.home, g.face, -14.0), g.across, n as f32 * 2.0), g.home);
        }
        let began = working(&mut s, driver);
        assert!(dist2d(s.mobs[&driver].pos, g.work.unwrap()) <= WORK_ARRIVAL + 0.05, "at his one place under the log");
        let ev = timed(&mut s, REPAIR - 0.15);
        assert!(live(&s, MACHINE).is_none(), "no early rebuild");
        // Seen coming on: the wreck coughs three times, he says it's turning over, his sparks
        // quicken and the smoke thickens.
        let coughs: Vec<f32> = ev.iter().filter(|(_, e)| matches!(e, Event::Sound { id, sound: SPUTTER_SOUND } if *id == machine)).map(|(t, _)| *t - began).collect();
        assert_eq!(coughs.len(), 3, "{coughs:?}");
        assert!(coughs.iter().zip([0.25, 0.5, 0.75]).all(|(t, q)| (*t - q * REPAIR).abs() < 0.11), "{coughs:?}");
        assert!(yells(&ev).iter().any(|(_, t)| t == LINE_HALF));
        let count = |from: f32, to: f32, of: &dyn Fn(&Event) -> bool| ev.iter().filter(|(t, e)| (from..to).contains(&(*t - began)) && of(e)).count();
        let sparks = |e: &Event| matches!(e, Event::Visual { id, kit: combat::KIT_PARRY, .. } if *id == driver);
        let smoke = |e: &Event| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == machine);
        let struck: Vec<f32> = ev.iter().filter(|(_, e)| sparks(e)).map(|(t, _)| *t - began).collect();
        let gaps: Vec<f32> = struck.windows(2).map(|w| w[1] - w[0]).collect();
        assert!(gaps.len() >= 4 && gaps[gaps.len() - 1] < 0.6 * gaps[0], "sparks faster at the end: {struck:?}");
        assert!(count(REPAIR * 2.0 / 3.0, REPAIR, &sparks) > count(0.0, REPAIR / 3.0, &sparks), "{struck:?}");
        assert!(count(REPAIR / 2.0, REPAIR, &smoke) > count(0.0, REPAIR / 2.0, &smoke), "smoke thicker at the end");
        let ev = run(&mut s, 0.25);
        let back = live(&s, MACHINE).expect("repair succeeds");
        assert!(s.clock - began >= REPAIR);
        assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health / 3);
        assert!(dist2d(s.mobs[&back].pos, wreck) < 0.1, "where the wreck lay");
        assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim: HEAVE_CLIP, .. } if *id == back)), "it heaves up");
        assert!(!ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Clear the floor!")), "not a fresh machine's pull");
        assert!(matches!(s.mobs[&back].state, MobState::Recover { until, .. } if until - s.clock >= REBUILT_WAIT - 0.3));
        assert!(!s.mobs.contains_key(&driver) && !s.mobs.contains_key(&machine));
        assert!(!s.slain("sneed"));
        assert_eq!(rig(&s).state, "hung", "the log still hangs over it");
        // Broken again: the same run, the same place, no second heap of lumber; no second repair.
        run(&mut s, 4.0);
        let final_driver = broken(&mut s, back, band[0]);
        assert!(dist2d(s.mobs[&back].pos, g.crash) < 0.4);
        assert_eq!(props(&s, LUMBER), 1);
        assert_eq!(s.mobs[&final_driver].health, driver_hp);
        let ev = run(&mut s, 20.0);
        assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
        assert!(ev.iter().any(|e| matches!(e, Event::MobSpawned { id } if *id == final_driver)), "he crawls out the same way");
        assert!(live(&s, MACHINE).is_none());
        assert!(s.mobs.contains_key(&back), "the second wreck lies there while he fights on");
        s.hurt_mob(final_driver, band[0], u32::MAX);
        assert!(s.slain("sneed"));
        assert_eq!(s.camp.counts.get("kill:dm_sneed"), Some(&1));
        run(&mut s, 7.0);
        assert!(!s.mobs.values().any(|m| matches!(s.content.mobs[m.def].key.as_str(), MACHINE | DRIVER)), "no ghost body");
        assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty());
        assert!(rig(&s).lumber.is_empty());
    }
}

#[test]
fn the_log_dropped_on_the_machine_stalls_it_and_a_log_cut_early_leaves_a_stagger_as_the_only_stop() {
    let (mut s, band, machine) = fight(1, 9);
    run(&mut s, 3.0);
    let g = rig(&s);
    let under = off(g.crash, g.across, g.len / 2.0);
    cut(&mut s, band[0]);
    let mut ev = Vec::new();
    while rig(&s).state == "cut" {
        // (Held under it: how a hero gets it there is the fight's, filmed and on the real floor.)
        let m = s.mobs.get_mut(&machine).unwrap();
        m.pos = under;
        m.path.clear();
        ev.extend(s.step(DT));
    }
    let now = s.clock;
    assert!(s.machinery.stalls.contains_key(&machine), "its engine stalls under the log");
    assert!(matches!(s.mobs[&machine].state, MobState::Recover { until, .. } if (until - now - STALL).abs() < 0.11), "its own stall, nothing new");
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, anim: SLUMP_CLIP, .. } if *id == machine)));
    let after = run(&mut s, STALL - 0.3);
    assert!(!after.iter().any(|e| matches!(e, Event::MobWindup { id, .. } if *id == machine)), "open the whole stall");
    ev.extend(after);
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == machine && text.contains("heap of junk"))));
    // The log is spent. Broken now, Sneed still gets to his place under where it lies (no hull),
    // and only a stagger stops him.
    let driver = broken(&mut s, machine, band[0]);
    working(&mut s, driver);
    assert_eq!(rig(&s).state, "down");
    let tie = g.tie.unwrap();
    put(&mut s, band[0], off(tie, g.face, 1.5), tie);
    s.input(band[0], Input::Attack);
    run(&mut s, 1.2);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }), "the stake has nothing left to drop");
    s.stagger_mob(driver, 1.0, false);
    s.step(DT);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
}

#[test]
fn a_hero_stunned_under_the_log_beside_the_machine_is_not_a_sure_death() {
    let mut worst = 0.0f32;
    // The machine drawn under the log with them (it's caught too), and the worst there is: the
    // machine beside the drop, in reach of them and clear of the log.
    for (seed, beside) in (1..=8u64).flat_map(|n| [(n, false), (n, true)]) {
        let (mut s, band, machine) = mortal(2, seed, 1);
        let g = rig_after(&mut s, 0.2);
        // band[1] fights it under the log; band[0] waits at the tie-off and cuts when they like.
        let stand = if beside { off(g.crash, g.across, g.len / 2.0) } else { g.work.unwrap() };
        if beside {
            let m = s.mobs.get_mut(&machine).unwrap();
            m.pos = off(stand, g.face, -4.0);
            m.path.clear();
        }
        put(&mut s, band[1], stand, g.crash);
        let tie = g.tie.unwrap();
        put(&mut s, band[0], off(tie, g.face, 1.5), tie);
        let wait = 2.0 + seed as f32 * 0.55;
        let t = s.clock;
        while s.clock < t + wait {
            let it = s.mobs[&machine].pos;
            put(&mut s, band[1], stand, it);
            s.step(DT);
            // (Whole until the cut: what they lose from here is the stun's.)
            let p = s.players.get_mut(&band[1]).unwrap();
            p.health = p.max_health;
        }
        let full = s.players[&band[1]].max_health;
        cut(&mut s, band[0]);
        landed(&mut s);
        let held = stunned(&s, band[1]).expect("stunned under it");
        let stalled = s.machinery.stalls.contains_key(&machine);
        run(&mut s, held + 1.0);
        let left = s.players[&band[1]].health;
        let lost = 1.0 - left as f32 / full as f32;
        println!("seed {seed}: cut {wait:.1} s in, the machine {} yd from them{}: lost {:.0}% of {full} in the stun", dist2d(s.mobs[&machine].pos, stand).round(), if stalled { " (caught too, stalled)" } else { "" }, lost * 100.0);
        assert!(left > 0, "seed {seed}: dead from full health under the log");
        assert_eq!(stalled, !beside, "seed {seed}");
        worst = worst.max(lost);
    }
    println!("worst: {:.0}% of full health", worst * 100.0);
    assert!(worst < 0.8, "never near a sure death: {worst}");
}

fn rig_after(s: &mut Survival, secs: f32) -> ShredderRig {
    run(s, secs);
    rig(s)
}

#[test]
fn a_hero_who_falls_while_a_friend_fights_on_leaves_the_fight_and_the_log_as_they_are() {
    let (mut s, band, machine) = fight(2, 15);
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    lost(&mut s, band[1], driver);
    run(&mut s, 1.0);
    assert!(live(&s, DRIVER) == Some(driver) && s.mobs.contains_key(&machine), "the fight goes on");
    assert_eq!(rig(&s).state, "hung");
    working(&mut s, driver);
    cut(&mut s, band[0]);
    landed(&mut s);
    assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent), "and the friend still has the log");
}

#[test]
fn a_lost_fight_at_any_stage_hangs_the_log_again_over_a_clean_floor_with_one_whole_machine() {
    for stage in 0..=8 {
        let (mut s, band, machine) = fight(1, 20 + stage);
        let h = band[0];
        run(&mut s, 0.5);
        let mut by = machine;
        match stage {
            // The machine whole, the log cut and down.
            0 => { cut(&mut s, h); landed(&mut s); }
            // On its run back.
            1 => { s.hurt_mob(machine, h, u32::MAX); run(&mut s, 0.3); }
            // Sneed inside the wreck; lying beside it; on foot; hammering.
            2 => { by = broken(&mut s, machine, h); }
            3 => { by = broken(&mut s, machine, h); run(&mut s, COLLAPSE + 0.3); }
            4 => { by = broken(&mut s, machine, h); emerged(&mut s, by); }
            5 => { by = broken(&mut s, machine, h); working(&mut s, by); }
            // The log on its way down on him; down on him.
            6 => { by = broken(&mut s, machine, h); working(&mut s, by); cut(&mut s, h); }
            7 => { by = broken(&mut s, machine, h); working(&mut s, by); cut(&mut s, h); landed(&mut s); run(&mut s, 1.0); }
            // The machine stood up again.
            _ => { by = broken(&mut s, machine, h); working(&mut s, by); run(&mut s, REPAIR + 0.3); by = live(&s, MACHINE).unwrap(); }
        }
        lost(&mut s, h, by);
        run(&mut s, 0.3);
        let g = rig(&s);
        assert_eq!(g.state, "hung", "stage {stage}: hung again at once");
        assert!(g.lumber.is_empty(), "stage {stage}");
        assert_eq!((props(&s, LOG), props(&s, STAKE), props(&s, LUMBER), props(&s, PLANK)), (LOG_BUNDLE.len(), 1, 0, 0), "stage {stage}");
        assert!(live(&s, DRIVER).is_none(), "stage {stage}");
        // (A machine on its run goes home as any creature does, and is whole there.)
        run(&mut s, 8.0);
        let machines: Vec<&Mob> = s.mobs.values().filter(|m| s.content.mobs[m.def].key == MACHINE).collect();
        assert_eq!(machines.len(), 1, "stage {stage}: one machine, no wreck");
        assert!(machines[0].health == machines[0].max_health && dist2d(machines[0].pos, g.home) < 1.0, "stage {stage}: whole, at its post");
        assert!(!s.mobs.values().any(|m| s.content.mobs[m.def].key == DRIVER), "stage {stage}");
        assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty() && s.machinery.runaways.is_empty(), "stage {stage}");
        assert!(!s.slain("sneed"));
        let ev = run(&mut s, 1.5);
        assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { .. })), "stage {stage}: no mark left on the floor");
    }
}

#[test]
fn a_lamp_rest_rigs_the_log_again_with_nothing_left_over() {
    let (mut s, band, machine) = fight(1, 31);
    run(&mut s, 0.5);
    let old = rig(&s);
    cut(&mut s, band[0]);
    landed(&mut s);
    s.hurt_mob(machine, band[0], u32::MAX);
    run(&mut s, 4.0);
    assert!(props(&s, PLANK) + props(&s, LUMBER) > 0);
    // (What a rest does to the encounter: every foe back at its post, whole.)
    s.reset_machinery();
    let gone: Vec<Id> = s.mobs.keys().copied().collect();
    for id in gone { s.dismiss(id); }
    let def = s.content.mobs.iter().position(|m| m.key == MACHINE).unwrap();
    let pack = s.content.dungeon.as_ref().unwrap().packs.iter().position(|p| p.key == "sneed");
    s.spawn_mob(def, old.home, pack, false);
    s.move_player(band[0], off(old.home, old.face, -40.0), 0.0, Moving::default());
    let ev = run(&mut s, 1.0);
    let g = rig(&s);
    assert_eq!(g.state, "hung");
    assert_ne!(g.log, old.log, "a fresh log");
    assert_eq!((props(&s, LOG), props(&s, STAKE), props(&s, LUMBER), props(&s, PLANK)), (LOG_BUNDLE.len(), 1, 0, 0));
    assert!(ev.iter().any(|e| matches!(e, Event::ObjGone { id } if Some(*id) == old.log)));
    assert_eq!((g.crash, g.work, g.tie), (old.crash, old.work, old.tie), "in the same places");
}

#[test]
fn sneed_killed_while_stunned_under_the_log_takes_it_and_the_wreck_with_his_body() {
    let (mut s, band, machine) = fight(1, 33);
    let driver = broken(&mut s, machine, band[0]);
    working(&mut s, driver);
    cut(&mut s, band[0]);
    landed(&mut s);
    s.hurt_mob(driver, band[0], u32::MAX);
    assert!(s.slain("sneed"));
    run(&mut s, BODY_SECS + 1.0);
    assert!(!s.mobs.contains_key(&machine) && !s.mobs.contains_key(&driver));
    assert_eq!((props(&s, LOG), props(&s, LUMBER), props(&s, PLANK)), (0, 0, 0), "the fallen log and the lumber go with him");
    let ev = run(&mut s, 2.0);
    assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { .. } | Event::Rope { .. })));
}

#[test]
fn a_log_cut_loose_after_the_fight_is_won_still_stuns_whoever_is_under_it_and_is_cleared_away() {
    let (mut s, band, machine) = fight(2, 35);
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    s.hurt_mob(driver, band[0], u32::MAX);
    run(&mut s, BODY_SECS + 1.0);
    let g = rig(&s);
    assert_eq!(g.state, "hung");
    put(&mut s, band[1], g.work.unwrap(), g.crash);
    cut(&mut s, band[0]);
    landed(&mut s);
    assert!(stunned(&s, band[1]).is_some());
    run(&mut s, BODY_SECS + 0.5);
    assert_eq!(props(&s, LOG) + props(&s, PLANK), 0);
}

/// A blow by the stake: the hero a swing's reach outside it, Sneed (held still) a yard to the
/// stake's side, the hero facing `at_him` or the stake. (the rig's state, Sneed's health lost).
fn blow_by_the_stake(at_him: bool, sneed_there: bool) -> (&'static str, u32) {
    let (mut s, band, machine) = fight(1, 6);
    run(&mut s, 0.5);
    let driver = broken(&mut s, machine, band[0]);
    emerged(&mut s, driver);
    let g = rig(&s);
    let tie = g.tie.unwrap();
    let mid = off(g.crash, g.across, g.len / 2.0);
    let out = facing(mid, tie);
    let stand = off(tie, out, 1.5);
    let him = off(tie, out + std::f32::consts::FRAC_PI_2, 1.0);
    let hp = s.mobs[&driver].health;
    let aim = if at_him { him } else { tie };
    put(&mut s, band[0], stand, aim);
    s.input(band[0], Input::Attack);
    for _ in 0..24 {
        let now = s.clock;
        let m = s.mobs.get_mut(&driver).expect("Sneed in the fight");
        m.pos = if sneed_there { him } else { off(tie, out, 14.0) };
        m.stagger_until = now + 1.0;
        put(&mut s, band[0], stand, aim);
        s.step(DT);
    }
    (rig(&s).state, hp.saturating_sub(s.mobs[&driver].health))
}

#[test]
fn a_blow_at_sneed_beside_the_tie_off_leaves_the_rope_whole_and_a_blow_at_the_stake_cuts_it() {
    // Filmed: a hero fighting Sneed by the stake cut the log loose without meaning to.
    let (state, hurt) = blow_by_the_stake(true, true);
    assert!(hurt > 0, "the blow lands on him");
    assert_eq!(state, "hung", "a blow at Sneed by the stake leaves the rope whole");
    // (The stake was in that blow's arc: the same blow with nobody there cuts it.)
    assert_ne!(blow_by_the_stake(true, false).0, "hung", "the same blow at nobody cuts it");
    // Aimed at the stake it cuts, whether or not it also catches him.
    assert_ne!(blow_by_the_stake(false, true).0, "hung", "a blow at the stake cuts it, Sneed beside it or not");
}
