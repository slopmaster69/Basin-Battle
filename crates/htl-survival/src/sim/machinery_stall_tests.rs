//! The Grind's stall as the fight's reward (docs/shredder-encounter.md, "Stall"): how many blows a
//! warrior lands in it, how long the catch gives them to get clear, and who it leaves no answer.
use super::tests::{fight, live, timed, DT};
use super::*;
use crate::land::FlatLand;

/// One step of a hero's run toward `to`, looking at `look`.
fn stride(s: &mut Survival, h: Id, to: Vec3, look: Vec3) {
    let me = s.players[&h].pos;
    let d = dist2d(me, to);
    let go = (s.content.player.run_speed * DT).min(d);
    let at = if d > 1e-3 { [me[0] + (to[0] - me[0]) / d * go, me[1] + (to[1] - me[1]) / d * go, me[2]] } else { me };
    s.move_player(h, at, facing(at, look), Moving::default());
}

/// How far a hero's blow reaches the machine from (its legs stand out from its middle).
fn reach(s: &Survival, h: Id, id: Id) -> f32 {
    s.weapon_of(&s.players[&h]).range + s.content.mobs[s.mobs[&id].def].body()
}

/// One step of a warrior on the machine: run to it until well in reach, then face it and `blow`.
fn hit_it(s: &mut Survival, h: Id, id: Id, blow: Option<Input>) {
    let (me, it) = (s.players[&h].pos, s.mobs[&id].pos);
    if dist2d(me, it) > reach(s, h, id) - 0.3 {
        stride(s, h, it, it);
    } else {
        s.move_player(h, me, facing(me, it), Moving::default());
        if let Some(b) = blow { s.input(h, b); }
    }
}

/// Steps until the Grind's rev begins: where it plants and the way it will drive.
fn to_rev(s: &mut Survival, h: Id, id: Id, blow: Option<Input>) -> Option<(Vec3, f32)> {
    for _ in 0..600 {
        if s.machinery.revs.get(&id).is_some_and(|r| r.kind == Move::Grind) {
            let m = &s.mobs[&id];
            return Some((m.pos, m.facing));
        }
        if s.machinery.runaways.contains_key(&id) || s.mobs[&id].health == 0 { return None; }
        hit_it(s, h, id, blow);
        s.step(DT);
    }
    panic!("it never revved");
}

/// A stride off the lane: the nearest spot 1 yd clear of its edge, level with where they stand.
const STRIDE: f32 = HALF_WIDTH + 1.0;
/// As far as a hero gets who runs from the lane the whole rev.
const FAR: f32 = 9.8;

/// The spot `off` yd to the side of the lane's middle, level with where they stand.
fn aside(at: Vec3, dir: f32, me: Vec3, off: f32) -> Vec3 {
    let (dx, dy) = (me[0] - at[0], me[1] - at[1]);
    let (ahead, side) = (dir.cos() * dx + dir.sin() * dy, -dir.sin() * dx + dir.cos() * dy);
    let side = if side < 0.0 { -off } else { off };
    [at[0] + dir.cos() * ahead - dir.sin() * side, at[1] + dir.sin() * ahead + dir.cos() * side, me[2]]
}

/// A warrior who steps off the lane during the rev and goes straight back to the stalled machine
/// `react` s after its backfire: when each blow lands, from the stall's start, and how long the
/// stall was. `blows` are tried in order, the last one over again.
fn punish(seed: u64, blows: &[Input], react: f32, off: f32) -> (Vec<f32>, f32) {
    let (mut s, band, id) = fight(1, seed);
    let h = band[0];
    let (at, dir) = to_rev(&mut s, h, id, None).unwrap();
    let safe = aside(at, dir, s.players[&h].pos, off);
    while !s.machinery.stalls.contains_key(&id) {
        let it = s.mobs[&id].pos;
        stride(&mut s, h, safe, it);
        s.step(DT);
    }
    let t0 = s.clock;
    let until = s.machinery.stalls[&id].until;
    let mut landed = Vec::new();
    let mut hp = s.mobs[&id].health;
    while s.machinery.stalls.contains_key(&id) {
        if s.clock - t0 >= react {
            hit_it(&mut s, h, id, Some(blows[landed.len().min(blows.len() - 1)]));
        }
        s.step(DT);
        if s.mobs[&id].health < hp {
            landed.push(s.clock - t0);
            hp = s.mobs[&id].health;
        }
    }
    (landed, until - t0)
}

/// The measure the stall's length is set by (`-- --nocapture` prints it). The hero reacts 0.3 s
/// after the backfire; one stepped a stride off the lane, one ran from it the whole rev.
#[test]
fn a_warrior_back_from_the_lane_lands_a_heavy_blow_and_two_light_ones_or_four_light_ones_before_the_engine_catches() {
    for (name, blows, need) in [("light", &[Input::Attack][..], 4), ("heavy then light", &[Input::Heavy, Input::Attack][..], 3)] {
        for off in [STRIDE, FAR] {
            for seed in [2u64, 7, 19] {
                let (landed, len) = punish(seed, blows, 0.3, off);
                assert!((len - STALL).abs() < 1e-3);
                let safe = landed.iter().filter(|t| **t <= len - CATCH + 1e-3).count();
                // The stall as it was: 3.5 s, its only warning the heave 0.45 s before the end.
                let old = landed.iter().filter(|t| **t <= 3.5 - 0.45 + 1e-3).count();
                println!("{name}, {off:.1} yd off, seed {seed}: blows at {landed:.2?}; {safe} before the catch ({old} before the old stall's warning)");
                assert!(safe >= need, "{name}, {off} yd off: {landed:?}");
                // Why not the old 3.5 s with the new catch inside it: the far hero's third blow
                // would land in the catch.
                if off == FAR && need == 3 {
                    assert!(landed[2] > 3.5 - CATCH, "{landed:?}");
                }
            }
        }
    }
}

/// The whole machine, fought by a scripted warrior who takes its blows, steps off the lane in
/// the rev and comes back in the stall: seconds from the pull to its break, and Grinds seen.
/// With `always` they strike whenever in reach; else only in a stall.
fn to_the_break(seed: u64, always: bool) -> (f32, u32) {
    let (mut s, band, id) = fight(1, seed);
    let h = band[0];
    let t0 = s.clock;
    let mut grinds = 0;
    while let Some((at, dir)) = to_rev(&mut s, h, id, always.then_some(Input::Attack)) {
        grinds += 1;
        let safe = aside(at, dir, s.players[&h].pos, STRIDE);
        while !s.machinery.stalls.contains_key(&id) && !s.machinery.runaways.contains_key(&id) {
            let it = s.mobs[&id].pos;
            stride(&mut s, h, safe, it);
            s.step(DT);
        }
        while s.machinery.stalls.contains_key(&id) {
            hit_it(&mut s, h, id, Some(Input::Attack));
            s.step(DT);
        }
        assert!(s.clock - t0 < 600.0, "the fight never ends");
    }
    (s.clock - t0, grinds)
}

/// What the longer stall does to the fight's length (`-- --nocapture` prints it): no health was
/// retuned for it.
#[test]
fn the_fight_to_the_break_with_a_scripted_warrior_is_measured() {
    for always in [true, false] {
        for seed in [2u64, 7, 19] {
            let (secs, grinds) = to_the_break(seed, always);
            assert!(grinds >= 1 && secs < 120.0, "{secs} s, {grinds} Grinds");
            println!("{}, seed {seed}: broken {secs:.1} s after the pull, {grinds} Grinds", if always { "striking whenever in reach" } else { "striking only in the stall" });
        }
    }
}

/// After a stall with a hero standing in its reach the whole time: seconds from the stall's end
/// to its next wind-up's start and to that blow.
fn after_standing(seed: u64) -> (f32, f32) {
    let (mut s, band, id) = fight(1, seed);
    let h = band[0];
    let (at, dir) = to_rev(&mut s, h, id, None).unwrap();
    let safe = aside(at, dir, s.players[&h].pos, STRIDE);
    while !s.machinery.stalls.contains_key(&id) {
        let it = s.mobs[&id].pos;
        stride(&mut s, h, safe, it);
        s.step(DT);
    }
    let until = s.machinery.stalls[&id].until;
    let (mut wound, mut blow) = (None, None);
    while blow.is_none() {
        assert!(s.clock < until + 10.0, "it never struck again");
        hit_it(&mut s, h, id, None);
        for e in s.step(DT) {
            match e {
                Event::MobWindup { id: who, .. } if who == id && wound.is_none() => wound = Some(s.clock - until),
                Event::Swing { attacker, anim: false, .. } if attacker == id && wound.is_some() => blow = Some(s.clock - until),
                _ => {}
            }
        }
    }
    (wound.unwrap(), blow.unwrap())
}

/// The Grind, now, down the lane from the machine through `aim`.
fn grind(s: &mut Survival, id: Id, h: Id, aim: Vec3) {
    let k = s.content.mobs[s.mobs[&id].def].patterns.iter().position(|p| p.name == "grind").unwrap();
    s.begin_pattern(id, k, false);
    s.windup(id, Foe::Hero(h), aim);
}

/// A fight on a flat floor, the Grind begun at once down +x from the origin; `wall` is a pillar
/// (0.7 yd round) on the lane, that far down it.
fn grinding(heroes: usize, seed: u64, wall: Option<f32>) -> (Survival, Vec<Id>, Id) {
    let (mut s, band, id) = fight(heroes, seed);
    if let Some(x) = wall {
        s.land = Box::new(FlatLand { walls: vec![([x, 0.0], 0.7)], ..Default::default() });
    }
    grind(&mut s, id, band[0], [8.0, 0.0, 0.0]);
    (s, band, id)
}

/// Steps until its stall begins, each hero running to their spot (`None`: they stand). Every
/// event, timed; the last step's are the stall's first.
fn to_stall(s: &mut Survival, id: Id, spots: &[(Id, Option<Vec3>)]) -> Vec<(f32, Event)> {
    let mut out = Vec::new();
    for _ in 0..200 {
        for (h, spot) in spots {
            if let Some(to) = spot {
                let it = s.mobs[&id].pos;
                stride(s, *h, *to, it);
            }
        }
        out.extend(timed(s, DT));
        if s.machinery.stalls.contains_key(&id) {
            return out;
        }
    }
    panic!("it never stalled");
}

fn sounds(ev: &[(f32, Event)], id: Id, which: u32) -> Vec<f32> {
    ev.iter().filter(|(_, e)| matches!(e, Event::Sound { id: who, sound } if *who == id && *sound == which)).map(|(t, _)| *t).collect()
}

fn smokes(ev: &[(f32, Event)], id: Id) -> Vec<f32> {
    ev.iter().filter(|(_, e)| matches!(e, Event::Missile { caster, spell: SMOKE, .. } if *caster == id)).map(|(t, _)| *t).collect()
}

fn sparks(ev: &[(f32, Event)], id: Id) -> Vec<f32> {
    ev.iter().filter(|(_, e)| matches!(e, Event::Visual { id: who, kit: combat::KIT_PARRY, by: None } if *who == id)).map(|(t, _)| *t).collect()
}

fn notes(ev: &[(f32, Event)], what: &str) -> usize {
    ev.iter().filter(|(_, e)| matches!(e, Event::Log(t) if t.contains(what))).count()
}

fn slump(ev: &[(f32, Event)], id: Id, at: f32) -> Option<(f32, f32)> {
    ev.iter().find_map(|(t, e)| match e {
        Event::Freeze { id: who, after, secs } if *who == id && *t == at && *secs > 0.0 => Some((*after, *secs)),
        _ => None,
    })
}

fn let_go(ev: &[Event], id: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Freeze { id: who, secs, .. } if *who == id && *secs == 0.0))
}

/// The farthest any of its blows but the Grind reaches, from its middle.
fn its_reach(s: &Survival, id: Id) -> f32 {
    let def = &s.content.mobs[s.mobs[&id].def];
    def.patterns.iter().filter(|p| p.name != "grind").flat_map(|p| &p.steps).filter_map(|st| st.range).fold(def.attack.range, f32::max)
}

const PI: f32 = std::f32::consts::PI;

#[test]
fn the_stall_starts_with_a_bang_and_the_slump_inside_a_quarter_second() {
    let (mut s, band, id) = grinding(1, 5, None);
    let (h, rev, from) = (band[0], s.clock, s.mobs[&id].pos);
    let ev = to_stall(&mut s, id, &[(h, Some([4.0, STRIDE, 0.0]))]);
    let t0 = s.clock;
    // Kept as they were: the rev's 1.4 s, the drive's 0.9 s and 6 yd.
    assert!((t0 - rev - (1.4 + GRIND)).abs() <= 2.0 * DT + 1e-3, "rev and drive took {:.2} s", t0 - rev);
    assert!((dist2d(from, s.mobs[&id].pos) - DISTANCE).abs() < 0.2, "{:?}", s.mobs[&id].pos);
    assert_eq!(s.players[&h].health, 100_000, "a stride off the lane is safe");
    // The backfire, the machine's cry, smoke and sparks, all as the drive ends, once.
    assert_eq!(sounds(&ev, id, BACKFIRE_SOUND), [t0]);
    assert_eq!(sounds(&ev, id, STALL_SOUND), [t0]);
    assert!(smokes(&ev, id).contains(&t0) && sparks(&ev, id).contains(&t0));
    assert!(sounds(&ev, id, rig::THUD_SOUND).is_empty(), "no crash on an open floor");
    assert!(ev.iter().any(|(t, e)| *t == t0 && matches!(e, Event::React { id: who, anim: SLUMP_CLIP, .. } if *who == id)));
    let (after, secs) = slump(&ev, id, t0).expect("held in its slump");
    assert!(after <= 0.25 && (after + secs - (STALL - SLUMP_RISE)).abs() < 1e-3, "slumped {after} s in, held {secs} s");
    assert!((s.machinery.stalls[&id].until - t0 - STALL).abs() < 1e-3);
    // It pours until the engine catches.
    let mid = timed(&mut s, STALL - CATCH - 0.1);
    let (smoke, spark, sputter) = (smokes(&mid, id).len(), sparks(&mid, id).len(), sounds(&mid, id, SPUTTER_SOUND).len());
    assert!(smoke >= 12 && spark >= 7 && sputter >= 5, "{smoke} smokes, {spark} sparks, {sputter} sputters");
}

#[test]
fn driven_into_a_wall_it_crashes_with_a_bigger_bang_and_stalls_a_second_and_a_half_longer() {
    let (mut s, band, id) = grinding(1, 5, Some(4.0));
    let (h, rev) = (band[0], s.clock);
    s.move_player(h, [4.8, 0.0, 0.0], PI, Moving::default());
    let ev = to_stall(&mut s, id, &[(h, None)]);
    let t0 = s.clock;
    let way = s.mobs[&id].pos[0];
    assert!(way > 2.5 && way <= 3.3, "stopped at the wall, {way:.2} yd on");
    // At the pace it always drives: there sooner than 0.9 s, not crawling.
    assert!((t0 - rev - (1.4 + GRIND * way / DISTANCE)).abs() <= 2.0 * DT + 1e-3, "rev and drive took {:.2} s", t0 - rev);
    assert_eq!(s.players[&h].health, 100_000, "no hit through the wall");
    assert_eq!(sounds(&ev, id, BACKFIRE_SOUND), [t0]);
    assert_eq!(sounds(&ev, id, rig::THUD_SOUND), [t0], "the crash is heard");
    assert!(ev.iter().any(|(t, e)| *t == t0 && matches!(e, Event::Visual { id: who, kit: combat::KIT_DUST, .. } if *who == id)));
    let until = s.machinery.stalls[&id].until;
    assert!((until - t0 - (STALL + CRASH_STALL)).abs() < 1e-3, "{:.2} s", until - t0);
    assert!((CRASH_STALL - 1.5).abs() < 1e-3);
    let (after, secs) = slump(&ev, id, t0).expect("held in its slump");
    assert!(after <= 0.25 && (after + secs - (STALL + CRASH_STALL - SLUMP_RISE)).abs() < 1e-3);
    // Its catch comes a second before the longer end, once.
    let rest = timed(&mut s, STALL + CRASH_STALL - 0.1);
    let caught = sounds(&rest, id, ENGINE_SOUND);
    assert!(caught.len() == 1 && (caught[0] - (until - CATCH)).abs() <= DT + 1e-3, "{caught:?} against {until}");

    // Stopped in its drive by a hero (a stagger), not by a wall: the plain stall, no crash.
    let (mut s, band, id) = grinding(1, 5, None);
    let h = band[0];
    s.move_player(h, [4.0, STRIDE, 0.0], PI, Moving::default());
    while !s.machinery.grinds.contains_key(&id) {
        s.step(DT);
    }
    timed(&mut s, 0.3);
    s.stagger_mob(id, 0.6, false);
    let ev = to_stall(&mut s, id, &[(h, None)]);
    assert!(s.mobs[&id].pos[0] < DISTANCE - 1.0, "stopped short");
    assert!((s.machinery.stalls[&id].until - s.clock - STALL).abs() < 1e-3);
    assert!(sounds(&ev, id, rig::THUD_SOUND).is_empty());
}

#[test]
fn a_hero_between_the_machine_and_a_wall_steps_aside_unhurt_or_is_hit_once_and_either_way_strikes_it_in_its_crash() {
    for stays in [false, true] {
        let (mut s, band, id) = grinding(1, 6, Some(5.0));
        let h = band[0];
        s.move_player(h, [3.0, 0.0, 0.0], PI, Moving::default());
        let (mut hits, mut hp) = (0, s.players[&h].health);
        let mut count = |s: &Survival| {
            if s.players[&h].health < hp {
                hits += 1;
                hp = s.players[&h].health;
            }
            hits
        };
        while !s.machinery.stalls.contains_key(&id) {
            if !stays {
                let it = s.mobs[&id].pos;
                stride(&mut s, h, [3.0, STRIDE, 0.0], it);
            }
            s.step(DT);
            count(&s);
        }
        let (t0, until) = (s.clock, s.machinery.stalls[&id].until);
        assert!((until - t0 - (STALL + CRASH_STALL)).abs() < 1e-3, "the wall behind them is its crash");
        assert!(s.cc_of(h).iter().all(|c| c.until <= s.clock), "not held by the blow");
        // Back to it (or already beside it), striking until the engine catches.
        let (mut landed, mut mhp) = (Vec::new(), s.mobs[&id].health);
        while !s.machinery.stalls[&id].caught {
            if s.clock - t0 >= 0.3 {
                hit_it(&mut s, h, id, Some(Input::Attack));
            }
            s.step(DT);
            count(&s);
            if s.mobs[&id].health < mhp {
                landed.push(s.clock - t0);
                mhp = s.mobs[&id].health;
            }
        }
        println!("between it and a wall, {}: hit {} times, light blows at {landed:.2?} before the catch", if stays { "staying" } else { "stepping aside" }, count(&s));
        assert_eq!(count(&s), if stays { 1 } else { 0 });
        assert!(landed.len() >= 6, "{landed:?}");
    }
}

#[test]
fn the_engine_catches_a_full_second_before_it_stands_and_a_hero_who_leaves_then_is_out_of_reach_of_its_next_blow() {
    for react in [0.0, 0.3] {
        let (mut s, band, id) = grinding(1, 7, None);
        let h = band[0];
        to_stall(&mut s, id, &[(h, Some([4.0, STRIDE, 0.0]))]);
        let (t0, until) = (s.clock, s.machinery.stalls[&id].until);
        let (mut ev, mut at_end, mut caught_at, mut blow) = (Vec::new(), None, None, None);
        // They strike it until the engine catches, then run straight from it.
        while s.clock < until + 3.0 {
            let (me, it) = (s.players[&h].pos, s.mobs[&id].pos);
            if caught_at.is_none() && s.machinery.stalls.get(&id).is_some_and(|st| st.caught) {
                caught_at = Some(s.clock);
            }
            match caught_at {
                Some(t) if s.clock - t >= react => {
                    let d = dist2d(me, it).max(1e-3);
                    let away = [me[0] + (me[0] - it[0]) / d * 60.0, me[1] + (me[1] - it[1]) / d * 60.0, me[2]];
                    stride(&mut s, h, away, away);
                }
                _ => hit_it(&mut s, h, id, Some(Input::Attack)),
            }
            let step = timed(&mut s, DT);
            let gap = dist2d(s.players[&h].pos, s.mobs[&id].pos);
            if at_end.is_none() && !s.machinery.stalls.contains_key(&id) {
                at_end = Some(gap);
            }
            if blow.is_none() && step.iter().any(|(_, e)| matches!(e, Event::Swing { attacker, anim: false, .. } if *attacker == id)) {
                blow = Some((s.clock - until, gap));
            }
            ev.extend(step);
        }
        let stalled: Vec<(f32, Event)> = ev.into_iter().filter(|(t, _)| *t <= until).collect();
        // The count: one steady roar a full second before the end, and what poured from it stops.
        let roar = sounds(&stalled, id, ENGINE_SOUND);
        assert!(roar.len() == 1 && (roar[0] - (until - CATCH)).abs() <= DT + 1e-3, "{roar:?} against {until}");
        assert!((CATCH - 1.0).abs() < 1e-3);
        assert_eq!(notes(&stalled, "the engine catches"), 1);
        let last = |ts: Vec<f32>| ts.into_iter().fold(t0, f32::max);
        let (smoke, spark, sputter) = (last(smokes(&stalled, id)), last(sparks(&stalled, id)), last(sounds(&stalled, id, SPUTTER_SOUND)));
        assert!(smoke < roar[0] && spark < roar[0] && sputter < roar[0], "smoke till {smoke}, sparks till {spark}, sputter till {sputter}; caught {}", roar[0]);
        assert!(smoke > roar[0] - 0.3 && sputter > roar[0] - 0.8, "it pours right up to the catch");
        assert!(!stalled.iter().any(|(_, e)| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "no wind-up in the stall");
        // Clear: past any blow it has as it stands, and never hit.
        let at_end = at_end.unwrap();
        println!("leaving {react:.1} s after the catch: {at_end:.1} yd off as it stands (its blows reach {:.1} yd), its next blow {blow:.2?}", its_reach(&s, id));
        assert!(at_end > its_reach(&s, id) + 1.0, "{at_end}");
        assert_eq!(s.players[&h].health, 100_000);
    }
    // One who stays in its reach: its blow comes no sooner after it stands than it did (1.05 s).
    for seed in [2u64, 7, 19, 23] {
        let (wound, blow) = after_standing(seed);
        println!("seed {seed}: staying, it winds up {wound:.2} s after the stall's end and strikes {blow:.2} s after it");
        assert!(wound >= 0.0 && blow >= 1.05 - 1e-3, "{wound} {blow}");
    }
}

/// What its break must do from any stall: the stall gone at once, the machine on its run, its
/// crash at the post and Sneed in the wreck; never a catch, a wind-up or a second stall on the way.
fn breaks_cleanly(s: &mut Survival, id: Id, what: &str) {
    assert!(!s.machinery.stalls.contains_key(&id), "{what}");
    assert_eq!(s.shredder_stage(), "runaway", "{what}");
    let mut rest = Vec::new();
    for _ in 0..400 {
        if live(s, DRIVER).is_some() {
            break;
        }
        rest.extend(timed(s, DT));
    }
    assert!(live(s, DRIVER).is_some(), "{what}: it crashed and Sneed is in the wreck");
    assert_eq!(s.shredder_stage(), "inside", "{what}");
    assert_eq!(notes(&rest, "engine catches") + notes(&rest, "stalls"), 0, "{what}");
    assert!(sounds(&rest, id, BACKFIRE_SOUND).is_empty() && sounds(&rest, id, SPUTTER_SOUND).is_empty(), "{what}");
    assert!(!rest.iter().any(|(_, e)| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "{what}");
}

#[test]
fn a_stall_begun_at_1_health_holds_until_one_blow_breaks_it_into_its_run_and_crash() {
    for wall in [None, Some(5.0)] {
        let what = if wall.is_some() { "crashed into a wall" } else { "on open floor" };
        let (mut s, band, id) = grinding(1, 8, wall);
        let h = band[0];
        s.move_player(h, [3.0, 0.0, 0.0], PI, Moving::default());
        s.mobs.get_mut(&id).unwrap().health = 1;
        to_stall(&mut s, id, &[(h, Some([3.0, STRIDE, 0.0]))]);
        assert_eq!(s.mobs[&id].health, 1, "{what}: neither the backfire nor the crash hurts it");
        assert_eq!(s.shredder_stage(), "machine", "{what}");
        let (t0, until) = (s.clock, s.machinery.stalls[&id].until);
        assert!((until - t0 - if wall.is_some() { STALL + CRASH_STALL } else { STALL }).abs() < 1e-3, "{what}");
        let mut ev = Vec::new();
        while s.machinery.stalls.contains_key(&id) {
            assert!(s.clock < t0 + 2.0, "{what}: the hero's first blow breaks it");
            hit_it(&mut s, h, id, Some(Input::Attack));
            ev.extend(s.step(DT));
        }
        assert!(let_go(&ev, id), "{what}: the slump lets go as it breaks");
        breaks_cleanly(&mut s, id, what);
    }
}

#[test]
fn a_break_at_any_moment_of_a_stall_goes_straight_into_the_run_and_the_crash() {
    let cases = [
        (None, 1.0, "smoking"),
        (None, STALL - 0.6, "its engine caught"),
        (None, STALL - 0.2, "heaving itself up"),
        (Some(5.0), STALL, "crashed, smoking"),
        (Some(5.0), STALL + CRASH_STALL - 0.6, "crashed, its engine caught"),
    ];
    for (wall, at, what) in cases {
        let (mut s, band, id) = grinding(1, 11, wall);
        let h = band[0];
        s.move_player(h, [3.0, 0.0, 0.0], PI, Moving::default());
        to_stall(&mut s, id, &[(h, Some([3.0, STRIDE, 0.0]))]);
        timed(&mut s, at);
        assert_eq!(s.machinery.stalls[&id].caught, what.contains("caught") || what.contains("heaving"), "{what}");
        s.events.clear();
        s.hurt_mob(id, h, u32::MAX);
        assert!(let_go(&s.events, id), "{what}: the slump lets go as it breaks");
        breaks_cleanly(&mut s, id, what);
    }
}

#[test]
fn two_heroes_both_get_the_stall_and_both_get_the_count() {
    let (mut s, band, id) = grinding(2, 9, None);
    // Both on the lane as it revs, one each side of its middle.
    s.move_player(band[1], [3.0, -1.0, 0.0], PI, Moving::default());
    let spots = [(band[0], Some([4.0, STRIDE, 0.0])), (band[1], Some([3.0, -STRIDE, 0.0]))];
    to_stall(&mut s, id, &spots);
    let (t0, until) = (s.clock, s.machinery.stalls[&id].until);
    assert!((until - t0 - STALL).abs() < 1e-3, "one stall, its own length");
    assert!(band.iter().all(|h| s.players[h].health == 100_000), "each stepped off their own side");
    // Both strike it until the engine catches; both leave then, each their own way.
    let mut landed = [0usize; 2];
    while s.clock < until + 1.0 {
        let caught = s.machinery.stalls.get(&id).is_none_or(|st| st.caught);
        for h in &band {
            let (me, it) = (s.players[h].pos, s.mobs[&id].pos);
            if caught {
                let d = dist2d(me, it).max(1e-3);
                let away = [me[0] + (me[0] - it[0]) / d * 60.0, me[1] + (me[1] - it[1]) / d * 60.0, me[2]];
                stride(&mut s, *h, away, away);
            } else if s.clock - t0 >= 0.3 {
                hit_it(&mut s, *h, id, Some(Input::Attack));
            }
        }
        for e in s.step(DT) {
            if let Event::Swing { attacker, victim: Some(who), anim: false, damage, .. } = e {
                if who == id && damage > 0 {
                    if let Some(n) = band.iter().position(|h| *h == attacker) {
                        landed[n] += 1;
                    }
                }
            }
        }
        if s.clock <= until {
            assert!(!matches!(s.mobs[&id].state, MobState::Windup { .. }), "open to both the whole stall");
        }
    }
    println!("2 heroes: {landed:?} light blows each before the catch");
    assert!(landed.iter().all(|n| *n >= 4), "{landed:?}");
    assert!(band.iter().all(|h| s.players[h].health == 100_000), "both clear of its first blow");
    assert!(band.iter().all(|h| dist2d(s.players[h].pos, s.mobs[&id].pos) > its_reach(&s, id)), "and out of its reach");
}
