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

// --- the falling mine: its sections, its catch, its faces, and how it looks --------------------

use crate::content::{CollapseDef, CollapseSectionDef, EnrageDef};

/// Rhahk'Zor alone in a flat mine, his post `post` yd along +x from its front door, its phases
/// `phases`, and a hero three yards past him who can take it.
fn mine_at(phases: Vec<EnrageDef>, seed: u64, post: f32) -> (Survival, Id, Id) {
    mine_in(phases, seed, post, FlatLand::default())
}

/// `mine_at` on ground of the test's own (a wall to slam).
fn mine_in(phases: Vec<EnrageDef>, seed: u64, post: f32, land: FlatLand) -> (Survival, Id, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == "rhahkzor");
    d.packs[0].at = [post, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [post + 100.0, -20.0, 0.0];
    let def = c.mob("dm_rhahkzor").unwrap();
    c.mobs[def].phases = phases;
    let mut s = Survival::new_dungeon(c, Box::new(land), seed, 1_000_000.0);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_rhahkzor").unwrap().0;
    let h = s.join("mole", "Mole", None);
    s.move_player(h, [post + 3.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    sturdy(&mut s, h);
    s.engage(boss, mobs::Foe::Hero(h));
    (s, boss, h)
}

fn mine(phases: Vec<EnrageDef>, seed: u64) -> (Survival, Id, Id) {
    mine_at(phases, seed, 0.0)
}

/// A hero who can take whatever comes down.
fn sturdy(s: &mut Survival, h: Id) {
    let p = s.players.get_mut(&h).unwrap();
    p.max_health = 10_000_000;
    p.health = 10_000_000;
}

/// One more hero, set down at `at`.
fn hero_at(s: &mut Survival, key: &str, at: Vec3) -> Id {
    let h = s.join(key, key, None);
    s.move_player(h, at, 0.0, Moving::default());
    sturdy(s, h);
    h
}

/// One long section down a straight tunnel along +x (no face authored: the whole route is its
/// one section, its face a little short of the end).
fn gauntlet(speed: f32, len: f32) -> Vec<EnrageDef> {
    let route = vec![[0.0, 0.0, 0.0], [len, 0.0, 0.0]];
    vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed, lead: 14.0, hold: 0.26, ..Default::default() }), ..Default::default() }]
}

/// Where the faces of `sectioned` stand, yards from its door.
const LINES: [f32; 4] = [30.0, 44.0, 74.0, 88.0];
/// Where Rhahk'Zor stands in it: just past the first line, as in the mine.
const POST: f32 = 35.0;

/// The collapse laid as the real one is, down a straight tunnel along +x, 20 yd wide: the door at
/// 0, four sections, the end at 95 yd; `rest` between surges.
fn sectioned(rest: f32) -> Vec<EnrageDef> {
    lined(&LINES, 95.0, 5.0, rest)
}

fn lined(lines: &[f32], len: f32, speed: f32, rest: f32) -> Vec<EnrageDef> {
    let route = vec![[0.0, 0.0, 0.0], [len, 0.0, 0.0]];
    let sections = lines.iter().map(|x| CollapseSectionDef { face: [[*x, 10.0], [*x, -10.0]], reach: 25.0, height: 8.0 }).collect();
    let collapse = CollapseDef { route, speed, lead: 14.0, hold: 0.26, sections, rest, end_text: "No more mine.".into(), held_text: "Not in a hole!".into(), ..Default::default() };
    vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(collapse), ..Default::default() }]
}

/// A blow that brings on its phase.
fn strike(s: &mut Survival, boss: Id, h: Id) {
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, h, max / 5);
}

/// Into the phase, as a blow brings it (once what the boss was doing is done), the hero at `at`.
fn begin_at(s: &mut Survival, boss: Id, h: Id, at: Vec3) {
    strike(s, boss, h);
    let until = s.clock + 10.0;
    while s.mobs[&boss].phase == 0 && s.clock < until {
        s.move_player(h, at, 0.0, Moving::default());
        s.step(DT);
    }
    assert!(s.mobs[&boss].phase > 0, "the phase begun");
}

fn begin(s: &mut Survival, boss: Id, h: Id) {
    begin_at(s, boss, h, [6.0, 0.0, 0.0]);
}

/// The collapse's pieces standing now: (id, display, position, tilt).
fn pieces(s: &Survival) -> Vec<(Id, u32, Vec3, f32)> {
    s.objs
        .iter()
        .filter_map(|(id, o)| match (s.collapse_piece_tilt(*id), &o.kind) {
            (Some(t), ObjKind::Prop { display, .. }) => Some((*id, *display, o.pos, t)),
            _ => None,
        })
        .collect()
}

/// The hero keeps this far ahead of the front.
fn keep_ahead(s: &mut Survival, h: Id, by: f32) {
    let front = s.collapse_front().map_or(0.0, |f| f.0);
    s.move_player(h, [front + by, 0.0, 0.0], 0.0, Moving::default());
}

/// The hero stands three yards past the line of the section going (or of the last one down).
fn past_the_line(s: &mut Survival, h: Id) {
    let Some((k, ..)) = s.collapse_state() else { return };
    let line = s.collapse_lines().unwrap().0[k];
    s.move_player(h, [line + 3.0, 0.0, 0.0], 0.0, Moving::default());
}

fn run_for(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// On until the collapse stands as `want` (its section, its stage), the hero past its line.
fn run_to(s: &mut Survival, h: Id, want: (usize, &str)) -> Vec<Event> {
    let mut out = Vec::new();
    while s.collapse_state().map(|c| (c.0, c.1)) != Some(want) {
        assert!(s.collapse_state().is_some() && s.clock < 200.0, "the collapse never reached {want:?}: {:?}", s.collapse_state());
        past_the_line(s, h);
        out.extend(s.step(DT));
    }
    out
}

/// On until the collapse is done, the hero past its line.
fn run_out(s: &mut Survival, h: Id) -> Vec<Event> {
    let mut out = Vec::new();
    while s.collapse_front().is_some_and(|f| !f.1) {
        assert!(s.clock < 300.0, "the collapse never done: {:?}", s.collapse_state());
        past_the_line(s, h);
        out.extend(s.step(DT));
    }
    out
}

fn caught(ev: &[Event], name: &str) -> usize {
    let what = format!("the collapse catches {name} ");
    ev.iter().filter(|e| matches!(e, Event::Log(t) if t.starts_with(&what))).count()
}

#[test]
fn the_collapse_starts_at_the_door_and_comes_inward_in_surges() {
    let (mut s, boss, h) = mine_at(sectioned(3.0), 7, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let (lines, total) = s.collapse_lines().unwrap();
    assert_eq!(lines.len(), 4);
    for (line, want) in lines.iter().zip(LINES) {
        assert!((line - want).abs() < 1.0e-3, "a face where its line crosses the route: {lines:?}");
    }
    assert!((total - 95.0).abs() < 1.0e-3, "{total}");
    // It starts at the door, behind him, with a warning: nothing down yet.
    assert_eq!(s.collapse_state(), Some((0, "warn", 0.0, false)), "from the door");
    assert!(s.collapse_face_lines().is_empty());
    // Each surge: the warning with the front still, the wave at its speed, the face, the rest.
    let mut steps: BTreeMap<(usize, &'static str), (usize, f32, f32)> = BTreeMap::new();
    let mut ev = Vec::new();
    let mut last = (0, "warn");
    while s.collapse_front().is_some_and(|f| !f.1) && s.clock < 120.0 {
        past_the_line(&mut s, h);
        let was = s.collapse_front().unwrap().0;
        let e = s.step(DT);
        let (k, stage, front, _) = s.collapse_state().unwrap();
        let from = if k == 0 { 0.0 } else { LINES[k - 1] };
        let entry = steps.entry((k, stage)).or_insert((0, front, front));
        *entry = (entry.0 + 1, entry.1.min(front), entry.2.max(front));
        assert!(front >= was, "the front never goes back");
        assert!(front >= from - 1.0e-3 && front <= LINES[k] + 1.0e-3, "within its section: {front}");
        assert_eq!(s.collapse_face_lines().len(), if stage == "rest" { k + 1 } else { k }, "a face for each section down, {k} {stage}");
        // The band across the section's line, from wall to wall, from the surge's first moment
        // to its last; none while the front holds.
        let band: Vec<(Vec3, f32, f32)> = e
            .iter()
            .filter_map(|e| match e {
                Event::GroundBurst { at, yaw, shape: "line", reach, colour, .. } if colour == BAND_COLOUR => Some((*at, *yaw, *reach)),
                _ => None,
            })
            .collect();
        if stage == "rest" {
            assert!(band.is_empty() || last != (k, "rest"), "no band while the front holds");
        } else if last != (k, stage) && stage == "warn" {
            assert_eq!(band.len(), 2, "the band there the moment the surge begins");
        }
        for (at, yaw, reach) in band {
            let k = if last.1 == "rest" || stage != "rest" { k } else { k };
            assert!((at[0] - LINES[k]).abs() < 0.01 && at[1].abs() < 0.01, "on the line: {at:?}");
            assert!(yaw.cos().abs() < 1.0e-3 && (reach - 10.0).abs() < 0.01, "from the route out to a wall: {yaw} {reach}");
        }
        last = (k, stage);
        ev.extend(e);
    }
    assert_eq!(s.collapse_state(), Some((3, "rest", LINES[3], true)), "done at its last face");
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { id, text } if *id == boss && text == "No more mine.")));
    for (k, line) in LINES.iter().enumerate() {
        let from = if k == 0 { 0.0 } else { LINES[k - 1] };
        let (n, lo, hi) = steps[&(k, "warn")];
        assert!((n as f32 * DT - 2.0).abs() <= 0.05, "a 2 s warning: {n} steps");
        assert!((lo - from).abs() < 1.0e-3 && (hi - from).abs() < 1.0e-3, "the front still through the warning: {lo}..{hi}");
        let (n, ..) = steps[&(k, "wave")];
        assert!((n as f32 * DT - (line - from) / 5.0).abs() <= 0.05, "the wave at 5 yd/s over {} yd: {n} steps", line - from);
        let (n, lo, hi) = steps[&(k, "rest")];
        assert!((lo - line).abs() < 1.0e-3 && (hi - line).abs() < 1.0e-3, "the front holds at the face: {lo}..{hi}");
        if k < 3 {
            assert!((n as f32 * DT - 3.0).abs() <= 0.05, "held for its rest: {n} steps");
        }
    }
    // He fought on through it, never behind his own rubble.
    assert!(!ev.iter().any(|e| matches!(e, Event::Log(t) if t.contains("behind its own collapse"))));
    assert!(s.mobs[&boss].pos[0] > LINES[3], "ahead of the last face: {:?}", s.mobs[&boss].pos);
}

#[test]
fn a_hero_on_the_door_side_is_caught_and_thrown_clear_never_left_behind() {
    let (mut s, boss, h) = mine_at(sectioned(3.0), 9, POST);
    // Between the door and his post when it starts: the stretch that goes first.
    begin_at(&mut s, boss, h, [12.0, 6.0, 0.0]);
    assert_eq!(s.collapse_state().map(|c| (c.0, c.1)), Some((0, "warn")));
    let mut ev = Vec::new();
    let mut behind = 0;
    let mut longest = 0;
    let mut first: Option<(f32, Vec3, u32, Vec<Event>)> = None;
    let mut warned = 0;
    let mut rest_since: Option<f32> = None;
    while s.collapse_front().is_some_and(|f| !f.1) && s.clock < 120.0 {
        // They never move a step of their own: the collapse alone moves them.
        let before = s.players[&h].health;
        let e = s.step(DT);
        let (k, stage, front, _) = s.collapse_state().unwrap();
        let pos = s.players[&h].pos;
        if k == 0 && stage == "warn" {
            warned += e.iter().filter(|e| matches!(e, Event::Danger { player, .. } if *player == h)).count();
            assert_eq!(caught(&e, "Mole"), 0, "nobody caught before the rock comes");
            assert!(dist2d(pos, [12.0, 6.0, 0.0]) < 0.01);
        }
        if first.is_none() && caught(&e, "Mole") > 0 {
            first = Some((front, pos, before - s.players[&h].health, e.clone()));
        }
        behind = if s.collapse_behind(pos) { behind + 1 } else { 0 };
        longest = longest.max(behind);
        // (A throw ends by the face's first moment; one whose client never flew it is set down
        // `THROW_GRACE` later, at the catch's next look.)
        rest_since = if stage == "rest" { rest_since.or(Some(s.clock)) } else { None };
        if rest_since.is_some_and(|t| s.clock >= t + FACE_FIRST + THROW_GRACE + 2.0 * CATCH_TICK) {
            assert!(!s.collapse_behind(pos), "nobody behind a face once it stands: {pos:?} at {}", s.clock);
            assert!(pos[0] > LINES[k], "ahead of the face: {pos:?}");
        }
        // (Whole again for the next surge: a hero who never moves takes each one.)
        let p = s.players.get_mut(&h).unwrap();
        p.health = p.max_health;
        ev.extend(e);
    }
    assert!(warned >= 2, "warned from the surge's first moment: {warned}");
    let (front, at, lost, e) = first.expect("caught by the first surge");
    assert!(front > 12.5 && front <= 14.0, "as the wave passed them: {front}");
    assert!(lost >= 3_500_000 && lost < 3_600_000, "35% of their health: {lost}");
    // Thrown clear through the air, not blinked there: their client is told to fly them, toward
    // clear floor 4 yd past the line, up and over, within the time a throw may take.
    let to = [LINES[0] + 4.0, 0.0, 0.0];
    assert!(dist2d(at, [12.0, 6.0, 0.0]) < 0.01, "still where they stood as the throw begins: {at:?}");
    assert!(!e.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == h)), "no blink");
    let (dir, speed, lift) = e
        .iter()
        .find_map(|e| match e {
            Event::Hop { player, dir, speed, lift, .. } if *player == h => Some((*dir, *speed, *lift)),
            _ => None,
        })
        .expect("their client told to fly them");
    assert!((dir - facing([12.0, 6.0, 0.0], to)).abs() < 0.01, "toward the clear floor: {dir}");
    let air = dist2d([12.0, 6.0, 0.0], to) / speed;
    assert!(air >= THROW_AIR.0 && air <= THROW_AIR.1, "a flight seen, and soon over: {air}");
    assert!(lift >= THROW_LIFT, "up and over: {lift}");
    // This hero has no client to fly them: set down there all the same a moment after the throw
    // should have landed, and thrown down as they land.
    assert!(ev.iter().any(|e| matches!(e, Event::Teleport { player, pos, .. } if *player == h && dist2d(*pos, to) < 0.1)), "set down clear: {to:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::React { id, .. } if *id == h)), "and down a moment");
    // Once a surge, however long they lie there.
    assert_eq!(caught(&ev, "Mole"), 4, "caught once by each surge");
    assert!(longest as f32 * DT <= THROW_AIR.1 + THROW_GRACE + 2.0 * CATCH_TICK, "never behind the rubble longer than a throw: {longest} steps");
    let pos = s.players[&h].pos;
    assert!(dist2d(pos, [LINES[3] + 4.0, 0.0, 0.0]) < 0.1, "at the end, ahead of the last face: {pos:?}");
    assert!(s.players[&h].health > 0);
}

#[test]
fn nobody_is_behind_a_face_at_any_side_distance() {
    let (mut s, boss, h) = mine_at(sectioned(8.0), 13, POST);
    // The door pocket's far corner, a side cave, the middle of the tunnel; and one beyond the
    // section's reach, no part of the stretch that falls.
    let side = [[5.0, 18.0, 0.0], [20.0, -24.0, 0.0], [12.0, 0.0, 0.0]];
    let heroes: Vec<Id> = side.iter().enumerate().map(|(k, at)| hero_at(&mut s, ["Ayla", "Bram", "Cora"][k], *at)).collect();
    let far = hero_at(&mut s, "Dunn", [15.0, 40.0, 0.0]);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    for at in side {
        assert!(!s.collapse_behind(at), "nothing down in the warning");
    }
    let mut ev = run_to(&mut s, h, (0, "rest"));
    // (A throw is over by the face's first moment; a hero whose client never flew it, as here,
    // is set down a moment later.)
    ev.extend(run_for(&mut s, FACE_FIRST + THROW_GRACE + 2.0 * CATCH_TICK));
    let at: Vec<Vec3> = heroes.iter().map(|h| s.players[h].pos).collect();
    for (k, p) in at.iter().enumerate() {
        assert!(p[0] > LINES[0] + 3.0 && p[0] < LINES[0] + 5.0 && p[1].abs() < 4.0, "set down past the line: {p:?}");
        assert!(!s.collapse_behind(*p));
        assert!(s.land().footing(*p).is_some());
        assert!(s.players[&heroes[k]].health > 0);
    }
    for name in ["Ayla", "Bram", "Cora"] {
        assert_eq!(caught(&ev, name), 1, "{name} caught once");
    }
    assert_eq!(caught(&ev, "Dunn") + caught(&ev, "Mole"), 0, "nobody else");
    assert_eq!(s.players[&far].pos, [15.0, 40.0, 0.0], "beyond the section's reach: not the stretch that fell");
    // Every spot of the fallen stretch counts as behind it, wall to wall and into its corners.
    for x in [0.5, 8.0, 16.0, 24.0, 29.5] {
        for y in [-24.0, -12.0, 0.0, 9.0, 24.0] {
            assert!(s.collapse_behind([x, y, 0.0]), "({x}, {y}) is behind the face");
        }
    }
    for x in [30.5, 36.0, 60.0, 94.0] {
        for y in [-24.0, 0.0, 24.0] {
            assert!(!s.collapse_behind([x, y, 0.0]), "({x}, {y}) is ahead of it");
        }
    }
    // Put behind it by anything at all (here a hop no walk makes), a hero is set down ahead
    // again within a blink: the second time in a surge with no blow.
    let hp = s.players[&heroes[0]].health;
    s.move_player(heroes[0], [10.0, -15.0, 0.0], 0.0, Moving::default());
    let ev = run_for(&mut s, CATCH_TICK + 0.05);
    let p = s.players[&heroes[0]].pos;
    assert!(p[0] > LINES[0] && !s.collapse_behind(p), "ahead again: {p:?}");
    assert_eq!(caught(&ev, "Ayla"), 0);
    assert_eq!(s.players[&heroes[0]].health, hp, "no second blow in one surge");
    // One the surge never caught, who comes round behind it later, is caught as any.
    s.move_player(far, [15.0, 20.0, 0.0], 0.0, Moving::default());
    let ev = run_for(&mut s, CATCH_TICK + 0.05);
    assert_eq!(caught(&ev, "Dunn"), 1);
    assert!(s.players[&far].pos[0] > LINES[0]);
    // The dead are left where they lie.
    let _ = boss;
}

#[test]
fn nobody_walks_rolls_jumps_or_is_thrown_back_through_a_face() {
    let (mut s, boss, h) = mine_at(sectioned(60.0), 15, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    // Before the face stands the way is open: the hero walks the line freely.
    s.move_player(h, [31.0, 0.0, 0.0], 0.0, Moving::default());
    s.move_player(h, [29.0, 0.0, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&h].pos[0], 29.0);
    run_to(&mut s, h, (0, "rest"));
    assert_eq!(s.collapse_face_lines().len(), 1);
    let line = LINES[0];
    let stand = |s: &mut Survival, at: Vec3| {
        s.set_down(h, at, 0.0);
        s.events.clear();
    };
    // A walk into it, step by step: stopped a step short, their client told each time.
    stand(&mut s, [33.0, 0.0, 0.0]);
    for _ in 0..40 {
        let x = s.players[&h].pos[0];
        s.move_player(h, [x - 0.35, 0.0, 0.0], std::f32::consts::PI, Moving::default());
        assert!(s.players[&h].pos[0] > line, "never through: {:?}", s.players[&h].pos);
    }
    let x = s.players[&h].pos[0];
    assert!(x > line && x < line + 0.8, "stopped at its face: {x}");
    assert!(s.events.iter().any(|e| matches!(e, Event::Teleport { player, pos, .. } if *player == h && pos[0] > line)), "their client put back");
    // A roll's hop.
    stand(&mut s, [34.0, 2.0, 0.0]);
    s.move_player(h, [27.0, 2.0, 0.0], std::f32::consts::PI, Moving::default());
    let p = s.players[&h].pos;
    assert!(p[0] > line && p[0] < line + 0.8 && (p[1] - 2.0).abs() < 0.01, "a roll stopped at it: {p:?}");
    // A jump, off the floor.
    stand(&mut s, [32.0, -3.0, 0.0]);
    s.move_player(h, [31.0, -3.0, 2.5], std::f32::consts::PI, Moving::default());
    s.move_player(h, [28.5, -3.0, 2.5], std::f32::consts::PI, Moving::default());
    assert!(s.players[&h].pos[0] > line, "a jump stopped at it: {:?}", s.players[&h].pos);
    assert_eq!(s.players[&h].pos[2], 2.5, "at the height of their jump");
    // Up a slope into it (their last place lower than the floor at the stop): put on the floor
    // there, never under it (a hero under it falls through the world on their client).
    stand(&mut s, [33.0, 5.0, -0.8]);
    s.move_player(h, [29.0, 5.0, -0.8], std::f32::consts::PI, Moving::default());
    let p = s.players[&h].pos;
    assert!(p[0] > line && p[2] == 0.0, "stopped on the floor: {p:?}");
    assert!(s.events.iter().any(|e| matches!(e, Event::Teleport { player, pos, .. } if *player == h && pos[2] == 0.0)), "their client told so");
    // Along the wall, slantwise, at its very end.
    stand(&mut s, [32.0, 9.0, 0.0]);
    s.move_player(h, [28.0, 10.6, 0.0], std::f32::consts::PI, Moving::default());
    assert!(s.players[&h].pos[0] > line, "no way round its end: {:?}", s.players[&h].pos);
    // Thrown back by a blow: the slide ends at the face.
    let end = s.slide_end([33.0, 4.0, 0.0], std::f32::consts::PI, 10.0);
    assert!(end[0] > line && end[0] < line + 1.0, "a knock-back ends at it: {end:?}");
    let free = s.slide_end([33.0, 4.0, 0.0], 0.0, 10.0);
    assert!((free[0] - 43.0).abs() < 0.6, "and away from it goes its length: {free:?}");
    // Nor out from behind it (the catch takes whoever is there).
    assert!(s.face_crossing([27.0, 0.0, 0.0], [33.0, 0.0, 0.0]).is_some_and(|p| p[0] < line));
    // Nor does he walk into it himself (circling his foe, backing off to make space): sent
    // through it he goes up to it, his back to the rubble, and no farther.
    let put = |s: &mut Survival, at: Vec3| {
        let m = s.mobs.get_mut(&boss).unwrap();
        m.pos = at;
        m.path.clear();
    };
    put(&mut s, [36.0, 1.0, 0.0]);
    s.go(boss, [24.0, 1.0, 0.0], 6.0);
    let end = s.mobs[&boss].path.last().copied().expect("on his way");
    assert!(s.mobs[&boss].path.len() == 1 && end[0] > line + 1.0 && end[0] < line + 1.5, "up to the face: {end:?}");
    put(&mut s, [36.0, 1.0, 0.0]);
    s.sidle_to(boss, [27.0, 4.0, 0.0], 3.0, h);
    let end = s.mobs[&boss].path.last().copied().expect("stepping round");
    assert!(end[0] > line + 1.0 && end[0] < line + 1.6, "sidling, too: {end:?}");
    put(&mut s, [line + 1.2, 1.0, 0.0]);
    s.go(boss, [24.0, 1.0, 0.0], 6.0);
    assert!(s.mobs[&boss].path.is_empty(), "at the face he stays: {:?}", s.mobs[&boss].path);
    s.go(boss, [38.0, 6.0, 0.0], 6.0);
    assert_eq!(s.mobs[&boss].path, vec![[38.0, 6.0, 0.0]], "away from it he goes as ever");
    // Along it and away from it nothing stops them.
    stand(&mut s, [31.0, -6.0, 0.0]);
    s.move_player(h, [31.0, 6.0, 0.0], 0.0, Moving::default());
    s.move_player(h, [40.0, 6.0, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&h].pos, [40.0, 6.0, 0.0]);
    let _ = boss;
}

#[test]
fn he_is_held_until_the_last_section_is_down_and_his_faces_outlast_him() {
    let (mut s, boss, h) = mine_at(sectioned(2.0), 17, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let max = s.mobs[&boss].max_health;
    let floor = (0.26 * max as f32).round() as u32;
    // A blow that would end him, in the warning, in a wave and while the front holds.
    for at in [(0, "warn"), (1, "wave"), (2, "rest"), (3, "wave")] {
        run_to(&mut s, h, at);
        s.events.clear();
        s.hurt_mob(boss, h, max);
        assert_eq!(s.mobs[&boss].health, floor, "held at 26% in {at:?}");
    }
    let ev = run_out(&mut s, h);
    assert_eq!(s.mobs[&boss].health, floor);
    assert_eq!(ev.iter().filter(|e| matches!(e, Event::Yell { text, .. } if text == "Not in a hole!")).count(), 0, "said once, at the first such blow");
    assert!(ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "No more mine.")));
    // The mine all down: an ordinary fight to the death, at the last face.
    run_for(&mut s, 4.0);
    let built = s.collapse_faces();
    assert!(built.len() >= 4 * 12, "four faces built: {}", built.len());
    let left = s.mobs[&boss].health;
    s.hurt_mob(boss, h, left);
    run_for(&mut s, 3.0);
    assert!(s.mobs.get(&boss).is_none_or(|m| m.health == 0), "and now he falls");
    assert_eq!(s.collapse_front(), None);
    // What he brought down stands after him: every face, solid, a stop.
    assert_eq!(s.collapse_face_lines().len(), 4);
    assert!(built.iter().all(|id| s.objs.contains_key(id) && s.collapse_piece_solid(*id)));
    s.set_down(h, [LINES[3] + 3.0, 0.0, 0.0], 0.0);
    s.move_player(h, [LINES[3] - 3.0, 0.0, 0.0], 0.0, Moving::default());
    assert!(s.players[&h].pos[0] > LINES[3], "still no way back through");
    // Until the dungeon resets.
    s.reset_below(false);
    run_for(&mut s, 0.5);
    assert!(s.collapse_face_lines().is_empty() && s.collapse_faces().is_empty() && pieces(&s).is_empty());
    assert!(s.face_crossing([LINES[3] + 3.0, 0.0, 0.0], [LINES[3] - 3.0, 0.0, 0.0]).is_none());
}

#[test]
fn rubble_faces_and_stops_go_when_the_fight_is_lost() {
    let (mut s, boss, h) = mine_at(sectioned(2.0), 19, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (2, "wave"));
    run_for(&mut s, 1.0);
    let rubble = s.collapse_rubble();
    let faces = s.collapse_faces();
    assert!(rubble.len() > 40, "rubble lying: {}", rubble.len());
    assert!(faces.len() >= 2 * 12, "two faces built: {}", faces.len());
    assert_eq!(s.collapse_face_lines().len(), 2);
    assert!(s.face_crossing([33.0, 0.0, 0.0], [27.0, 0.0, 0.0]).is_some());
    // Every hero down: the fight is lost, and the mine stands again for the next try.
    let hp = s.players[&h].health;
    s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    let mut ev = run_for(&mut s, 0.5);
    // Its rules end at once; its rock is seen to stand a moment in its dust, then goes.
    assert_eq!(s.collapse_front(), None, "the collapse is over");
    assert!(s.collapse_face_lines().is_empty() && s.face_crossing([33.0, 0.0, 0.0], [27.0, 0.0, 0.0]).is_none(), "no stop from the first moment");
    assert!(faces.iter().all(|id| s.objs.contains_key(id)), "the faces still seen");
    assert!(ev.iter().filter(|e| matches!(e, Event::MissileFrom { spell: LAND_DUST, .. })).count() >= 6, "in their dust");
    assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { .. })), "and nothing more falls");
    ev.extend(run_for(&mut s, LEAVE_HOLD + LEAVE_SECS + 1.0));
    assert_eq!(s.collapse_front(), None, "the collapse is over");
    assert!(s.collapse_rubble().is_empty() && s.collapse_faces().is_empty());
    assert!(pieces(&s).is_empty(), "nothing of it left standing");
    for id in rubble.iter().chain(&faces) {
        assert!(!s.objs.contains_key(id) && ev.iter().any(|e| matches!(e, Event::ObjGone { id: o } if o == id)), "each taken from the clients");
    }
    assert!(s.collapse_face_lines().is_empty(), "no face");
    assert!(s.face_crossing([33.0, 0.0, 0.0], [27.0, 0.0, 0.0]).is_none() && s.face_crossing([47.0, 3.0, 0.0], [41.0, 3.0, 0.0]).is_none(), "and nothing stops a hero there");
    run_for(&mut s, 20.0);
    let m = &s.mobs[&boss];
    assert!(dist2d(m.pos, [POST, 0.0, 0.0]) < 4.0, "back at his post: {:?}", m.pos);
    assert_eq!(m.health, m.max_health, "whole");
    // The next hero walks the whole mine, and for them it starts at the door again.
    let next = hero_at(&mut s, "Next", [POST + 3.0, 0.0, 0.0]);
    s.move_player(next, [27.0, 0.0, 0.0], 0.0, Moving::default());
    assert_eq!(s.players[&next].pos, [27.0, 0.0, 0.0], "through where a face stood");
    s.move_player(next, [POST + 3.0, 0.0, 0.0], 0.0, Moving::default());
    s.engage(boss, mobs::Foe::Hero(next));
    begin_at(&mut s, boss, next, [POST + 3.0, 0.0, 0.0]);
    assert_eq!(s.collapse_state(), Some((0, "warn", 0.0, false)), "from the door again");
}

#[test]
fn the_newest_face_survives_the_rubble_cap() {
    // A long way down a wide tunnel: more rock than the client is given at once.
    let (mut s, boss, h) = mine_at(lined(&[30.0, 60.0, 330.0], 340.0, 15.0, 0.5), 23, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (2, "warn"));
    let early = s.collapse_faces();
    assert!(early.len() >= 12, "the first face built, the second building: {}", early.len());
    let mut most = 0;
    let mut gone = 0;
    while s.collapse_front().is_some_and(|f| !f.1) && s.clock < 120.0 {
        past_the_line(&mut s, h);
        gone += s.step(DT).iter().filter(|e| matches!(e, Event::ObjGone { .. })).count();
        most = most.max(s.collapse_rubble().len());
    }
    run_for(&mut s, 4.0);
    assert!(gone > 0 && most == RUBBLE_MOST, "the oldest loose rock taken away as more came down: {gone} gone, {most} at most");
    assert!(s.collapse_rubble().len() <= RUBBLE_MOST);
    // Every face stands whatever the count: the oldest too, and the newest whole.
    assert!(early.iter().all(|id| s.objs.contains_key(id)), "the first faces still stand");
    let last: Vec<Vec3> = s.collapse_faces().iter().map(|id| s.objs[id].pos).filter(|p| p[0] > 320.0).collect();
    assert!(last.len() >= 12, "the newest face built whole: {}", last.len());
    for x in [30.0, 60.0] {
        let n = s.collapse_faces().iter().filter(|id| (s.objs[*id].pos[0] - x + 3.0).abs() < 3.5).count();
        assert!(n >= 12, "the face at {x} still whole: {n}");
    }
    assert_eq!(s.collapse_face_lines().len(), 3);
    assert!(s.face_crossing([333.0, 0.0, 0.0], [327.0, 0.0, 0.0]).is_some() && s.face_crossing([33.0, 0.0, 0.0], [27.0, 0.0, 0.0]).is_some());
    let all = pieces(&s).len();
    assert!(all <= RUBBLE_MOST + s.collapse_faces().len() + FALLING_MOST, "at most the cap and the faces at once: {all}");
}

#[test]
fn a_face_is_built_wall_to_wall_to_the_roof_and_solid_as_it_lands() {
    let (mut s, boss, h) = mine_at(sectioned(6.0), 25, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (0, "rest"));
    assert!(s.collapse_faces().is_empty(), "its rock still on the way down as the line is reached");
    let mut shown: BTreeMap<Id, f32> = BTreeMap::new();
    let mut ev = Vec::new();
    for _ in 0..500 {
        past_the_line(&mut s, h);
        ev.extend(s.step(DT));
        for id in s.collapse_faces() {
            shown.entry(id).or_insert(s.clock);
        }
    }
    let face: Vec<(Id, Vec3)> = s.collapse_faces().into_iter().map(|id| (id, s.objs[&id].pos)).collect();
    assert_eq!(face.len(), 18, "four layers of four and five rocks: {}", face.len());
    for (id, p) in &face {
        assert!(matches!(s.objs[id].kind, ObjKind::Prop { display: ROCK, .. }));
        assert!(s.collapse_piece_solid(*id), "solid where it landed");
        assert!(p[0] < LINES[0] && p[0] > LINES[0] - 6.0, "behind the line, on the buried side: {p:?}");
        assert_eq!(s.collapse_piece_tilt(*id), Some(0.0), "lying flat");
        assert!(s.objs[id].facing.sin().abs() < 0.2, "its long side along the line");
        assert!(ev.iter().any(|e| matches!(e, Event::ObjSpawned { id: o } if o == id)));
    }
    let (lo, hi) = face.iter().fold((f32::MAX, f32::MIN), |(lo, hi), (_, p)| (lo.min(p[1]), hi.max(p[1])));
    assert!(lo <= -7.0 && hi >= 7.0, "from wall to wall: {lo}..{hi}");
    for y in [-9.0, -5.0, 0.0, 5.0, 9.0] {
        assert!(face.iter().any(|(_, p)| (p[1] - y).abs() <= FACE_PITCH * 0.5 && p[2] < 0.0), "a rock on the floor at {y}");
    }
    let top = face.iter().map(|(_, p)| p[2]).fold(f32::MIN, f32::max);
    assert!(top > 8.0 - 2.0 * FACE_RISE, "built up to the roof: {top}");
    // Layer on layer: the floor first, the top last, all within two seconds of the line.
    let first = shown.values().copied().fold(f32::MAX, f32::min);
    let last = shown.values().copied().fold(f32::MIN, f32::max);
    assert!(last - first >= 0.5 && last - first <= 2.0, "built up layer by layer: {first}..{last}");
    // What's seen of it is the pale rock that fell: one over each solid rock, with no hull of its
    // own, and none past the line.
    let skin: Vec<Vec3> = s.collapse_face_skin().into_iter().map(|id| s.objs[&id].pos).collect();
    assert_eq!(skin.len(), face.len(), "a pale rock over each solid one");
    for id in s.collapse_face_skin() {
        assert!(matches!(s.objs[&id].kind, ObjKind::Prop { display: PALE, .. }) && !s.collapse_piece_solid(id));
    }
    for (_, p) in &face {
        assert!(skin.iter().any(|q| (q[1] - p[1]).abs() < 0.01 && (q[0] - p[0]).abs() < 2.0 && (q[2] - p[2]).abs() < 1.5), "pale rock over the rock at {p:?}");
    }
    assert!(skin.iter().all(|q| q[0] < LINES[0]), "none past the line");
    // Its rows behind it: rock across the whole width of the section, not down its middle.
    let rubble: Vec<Vec3> = s.collapse_rubble().iter().map(|id| s.objs[id].pos).collect();
    for x in [4.0, 10.0, 16.0, 22.0] {
        for y in [-9.0, 0.0, 9.0] {
            assert!(rubble.iter().any(|p| (p[0] - x).abs() <= 3.5 && (p[1] - y).abs() <= 3.5), "rock lying at ({x}, {y})");
        }
    }
    assert!(rubble.iter().all(|p| p[0] < LINES[0]), "and none past the line: the way out stays clear");
}

#[test]
fn rows_of_rock_are_seen_falling_across_the_tunnel_as_the_wave_comes() {
    let (mut s, boss, h) = mine(gauntlet(4.0, 400.0), 7);
    begin(&mut s, boss, h);
    let mut heights: BTreeMap<Id, Vec<f32>> = BTreeMap::new();
    let mut landed: BTreeMap<Id, (Vec3, f32)> = BTreeMap::new();
    let mut ev = Vec::new();
    for _ in 0..1300 {
        keep_ahead(&mut s, h, 6.0);
        ev.extend(s.step(DT));
        let front = s.collapse_front().unwrap().0;
        for (id, _, pos, _) in pieces(&s) {
            heights.entry(id).or_default().push(pos[2]);
        }
        for id in s.collapse_rubble() {
            landed.entry(id).or_insert((s.objs[&id].pos, front));
        }
    }
    // Rock seen coming down: shown high, moved down in place, and lying on the floor.
    let rock = |s: &Survival, id: &Id| matches!(s.objs[id].kind, ObjKind::Prop { display, .. } if display == PALE || display == PALE_TALL);
    let fell: Vec<Id> = landed
        .keys()
        .copied()
        .filter(|id| rock(&s, id))
        .filter(|id| {
            // Down without a stop until it strikes; then one hop, no higher than a fall's, to rest.
            let z = &heights[id];
            let rest = *z.last().unwrap();
            let struck = strike_of(&z);
            z[0] >= 3.5 && z[..=struck].windows(2).all(|w| w[1] <= w[0] + 1e-4) && z[struck..].iter().all(|h| *h <= rest + HOP_FALL + 1e-3)
        })
        .collect();
    assert!(fell.len() >= 60, "rock seen dropping from overhead: {}", fell.len());
    for id in &fell {
        let z = s.objs[id].pos[2];
        assert!(z > -1.0 && z < ROW_RISE * ROW_LAYERS as f32, "lying on the floor or on the rock under it, not hanging: {z}");
        assert!(ev.iter().any(|e| matches!(e, Event::ObjChanged { id: o } if o == id)), "moved in place as it fell");
    }
    // The wave's own rows: rock landing as the wave reaches it, all along the way, wall to wall.
    let at_front = landed.values().filter(|(p, front)| (p[0] - front).abs() <= ROW_STEP + 2.0).count();
    assert!(at_front >= 60, "rock landing at the wave: {at_front}");
    let front = s.collapse_front().unwrap().0;
    assert!(front > 35.0, "the wave came on: {front}");
    for x in (0..(front as i32 - 4)).step_by(6) {
        for y in [-18.0, -9.0, 0.0, 9.0, 18.0] {
            assert!(landed.values().any(|(p, _)| (p[0] - x as f32).abs() <= 3.5 && (p[1] - y).abs() <= 3.5), "the tunnel buried at ({x}, {y})");
        }
    }
    // The rows are a roof coming down, not blows: no ring under any of them. The only rings are
    // under the falls ahead of the wave, which are blows.
    let rings = ev.iter().filter(|e| matches!(e, Event::GroundBurst { shape: "ring", colour, .. } if colour == mobs::HAZARD_COLOUR)).count();
    let falls = ev.iter().filter(|e| matches!(e, Event::GroundBurst { shape: "ring", reach, colour, .. } if *reach == FALL_RADIUS && colour == mobs::HAZARD_COLOUR)).count();
    assert!(falls >= 4 && rings == falls, "rings only under the falls that are blows: {rings} rings, {falls} falls");
    // The pile builds: rock lands on rock, layer over layer, behind the wave.
    let piled = landed.values().filter(|(p, _)| p[2] > ROW_RISE * 0.5).count();
    assert!(piled >= 40, "rock lying on rock: {piled}");
    // A timber cracks overhead and comes down across the tunnel.
    let beams: Vec<&Id> = landed.keys().filter(|id| matches!(s.objs[*id].kind, ObjKind::Prop { display: BEAM, .. })).collect();
    assert!(beams.len() >= 2, "beams down across the tunnel: {}", beams.len());
    for id in beams {
        let t = s.collapse_piece_tilt(*id).unwrap();
        assert!(t > 1.5 && t < 2.6, "a beam lies or leans, not standing: {t}");
        let yaw = s.objs[id].facing;
        assert!(yaw.sin().abs() > 0.9, "across the tunnel: {yaw}");
        assert!(ev.iter().any(|e| matches!(e, Event::Sound { id: o, sound } if o == id && *sound == SOUND_CRACK)), "it cracks");
    }
    // The ground shakes for the hero in the section, and rock lands beside them.
    assert!(ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_RUMBLE)));
    assert!(ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_ROCKFALL)));
    assert!(ev.iter().any(|e| matches!(e, Event::Sound { sound: SOUND_RUMBLE, .. })));
    // Dust streams from the roof over the section, rolls ahead of the wave and is thrown up where
    // rock lands; the band lies across the line; no Rocky Ground Impact (its pink boulders
    // linger on film).
    let roof = ev.iter().filter(|e| matches!(e, Event::MissileFrom { spell: LAND_DUST, .. })).count();
    assert!(roof >= 200, "soft dust from the roof, over the whole section: {roof}");
    assert!(!ev.iter().any(|e| matches!(e, Event::MissileFrom { spell: DUST, .. })), "no starburst puffs: they read as hits");
    assert!(ev.iter().any(|e| matches!(e, Event::MissileFrom { spell: DUST_ROLL, .. })));
    assert!(ev.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", colour, .. } if colour == BAND_COLOUR)));
    assert!(!ev.iter().any(|e| matches!(e, Event::MissileFrom { spell: 26271, .. } | Event::Missile { spell: 26271, .. })));
}

#[test]
fn a_fall_drops_its_rock_onto_its_spot_as_its_blow_lands() {
    let (mut s, boss, _) = mine(Vec::new(), 3);
    s.events.clear();
    s.rocks_fall(boss, [5.0, 2.0, 0.0], DEBRIS_RADIUS, DEBRIS_WARN, 10, true);
    let blow = s.missiles.iter().find(|m| m.owner == boss).map(|m| m.land_at).unwrap();
    assert!(s.events.iter().any(|e| matches!(e, Event::GroundBurst { shape: "ring", reach, secs, .. } if *reach == DEBRIS_RADIUS && *secs == DEBRIS_WARN)),
        "its ring on the floor till it lands, markers or not");
    let mut shown = None;
    let mut track: Vec<(f32, Vec3)> = Vec::new();
    while s.collapse_rubble().is_empty() && s.clock < 5.0 {
        s.step(DT);
        if let Some((_, _, pos, _)) = pieces(&s).into_iter().next() {
            shown.get_or_insert((s.clock, pos[2]));
            track.push((s.clock, pos));
            assert!(pos[2] > -1.0, "never below its resting place");
        }
    }
    let (at, high) = shown.expect("its rock showed");
    assert!((at - (blow - ROCK_SECS)).abs() <= 2.0 * DT, "it starts down {ROCK_SECS} s before the blow: {at} vs {blow}");
    assert!(high > 4.0, "from overhead: {high}");
    let id = s.collapse_rubble()[0];
    let p = s.objs[&id].pos;
    assert!(dist2d(p, [5.0, 2.0, 0.0]) < 0.01, "on the spot: {p:?}");
    // It strikes the floor as the blow lands, inside its ring, and doesn't stop dead: one hop
    // and a roll, at rest on its spot `SETTLE_SECS` later.
    let (struck, hit) = track.iter().find(|(_, q)| q[2] <= p[2] + 1e-3).copied().unwrap();
    assert!((struck - blow).abs() <= 2.0 * DT, "striking the floor as the blow lands: {struck} vs {blow}");
    assert!(dist2d(hit, [5.0, 2.0, 0.0]) < DEBRIS_RADIUS * 0.5, "inside its ring: {hit:?}");
    let hop = track.iter().filter(|(t, _)| *t > struck).map(|(_, q)| q[2] - p[2]).fold(0.0, f32::max);
    assert!(hop > 0.2 && hop <= HOP_FALL + 1e-3, "one hop: {hop}");
    assert!(dist2d(hit, p) > 0.3, "and a roll: {hit:?} to {p:?}");
    assert!((s.clock - (blow + SETTLE_SECS)).abs() <= 2.0 * DT, "at rest {SETTLE_SECS} s on: {} vs {blow}", s.clock);
    run_for(&mut s, 30.0);
    assert!(s.objs.contains_key(&id), "and it stays");
}

#[test]
fn rubble_stays_behind_the_wave_as_it_goes_on() {
    let (mut s, boss, h) = mine(gauntlet(4.0, 400.0), 11);
    begin(&mut s, boss, h);
    for _ in 0..1000 {
        keep_ahead(&mut s, h, 6.0);
        s.step(DT);
    }
    let early = s.collapse_rubble();
    assert!(early.len() > 30, "rubble lying: {}", early.len());
    for _ in 0..500 {
        keep_ahead(&mut s, h, 6.0);
        s.step(DT);
    }
    assert!(early.iter().all(|id| s.objs.contains_key(id)), "what came down stays as the wave goes on");
    let front = s.collapse_front().unwrap().0;
    let behind = early.iter().filter(|id| s.objs[*id].pos[0] < front - 2.0).count();
    assert!(behind * 10 >= early.len() * 8, "and lies behind it: {behind} of {}", early.len());
}

#[test]
fn rubble_goes_with_a_reset_of_the_dungeon() {
    let (mut s, boss, h) = mine_at(sectioned(2.0), 13, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (1, "wave"));
    run_for(&mut s, 1.5);
    assert!(!s.collapse_rubble().is_empty() && !s.collapse_faces().is_empty());
    s.reset_below(false);
    run_for(&mut s, 0.5);
    assert!(s.collapse_rubble().is_empty() && s.collapse_faces().is_empty());
    assert!(pieces(&s).is_empty());
    assert_eq!(s.collapse_front(), None);
    assert!(s.collapse_face_lines().is_empty());
}

#[test]
fn other_foes_on_the_stretch_run_for_it_and_are_gone() {
    let (mut s, boss, h) = mine_at(sectioned(2.0), 27, POST);
    let pirate = s.content.mob("dm_pirate").unwrap();
    // One in the door pocket, one in the last section, one well past the route's end.
    let near = s.spawn_mob(pirate, [14.0, 4.0, 0.0], None, false);
    let mid = s.spawn_mob(pirate, [80.0, -6.0, 0.0], None, false);
    let off = s.spawn_mob(pirate, [140.0, 0.0, 0.0], None, false);
    s.engage(near, mobs::Foe::Hero(h));
    s.events.clear();
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    assert!(s.step_collapse_flee(near) && s.step_collapse_flee(mid), "both on the stretch run");
    assert!(!s.step_collapse_flee(off) && !s.step_collapse_flee(boss));
    let was = s.mobs[&mid].pos;
    let ev = run_for(&mut s, 1.0);
    assert!(s.mobs[&mid].pos[0] > was[0] + 2.0, "running for the far end: {:?}", s.mobs[&mid].pos);
    assert!(!ev.iter().any(|e| matches!(e, Event::Danger { from: Some(m), .. } if *m == near)), "its fight let go");
    let ev = run_for(&mut s, FLEE_SECS);
    for id in [near, mid] {
        assert!(!s.mobs.contains_key(&id) && ev.iter().any(|e| matches!(e, Event::MobGone { id: m } if *m == id)), "gone from the fight");
    }
    assert!(s.mobs.contains_key(&off) && s.mobs.contains_key(&boss), "the rest stay");
}

#[test]
fn the_mine_as_authored_falls_from_its_door_and_stops_short_of_the_galleries_lamp() {
    let c = Content::dev();
    let def = c.mob("dm_rhahkzor").unwrap();
    let phase = c.mobs[def].phases.iter().find(|p| p.collapse.is_some()).expect("his collapse");
    assert!((phase.at - 0.7).abs() < 1.0e-6, "at 70%");
    let col = phase.collapse.as_ref().unwrap();
    let d = c.dungeon.as_ref().unwrap();
    let post = d.packs.iter().find(|p| p.key == "rhahkzor").unwrap().at;
    // From the buried front door, behind him, through his post, toward the Galleries.
    let (door, end) = (col.route[0], *col.route.last().unwrap());
    assert!(dist2d(door, [-16.4, -383.1, 61.8]) < 13.0 && door[0] > post[0] + 20.0, "it starts at the door, behind him: {door:?}");
    assert!(col.route.windows(2).all(|w| w[1][0] < w[0][0]), "inward all the way");
    let len: f32 = col.route.windows(2).map(|w| dist2d(w[0], w[1])).sum();
    assert!((85.0..=110.0).contains(&len), "about a hundred yards, not the whole mine: {len}");
    // The Galleries lamp, where a hero who died wakes, is always ahead of the rubble.
    let lamp = d.lamps.iter().find(|l| dist2d(l.at, [-112.5, -405.0, 0.0]) < 3.0).expect("the Galleries lamp").at;
    assert!(dist2d(end, lamp) >= 12.0, "the end at least 12 yd short of the lamp: {}", dist2d(end, lamp));
    assert!(lamp[0] < end[0] - 10.0, "and the lamp on past it");
    // Four sections, each face across the route, in order from the door inward; the last one a
    // little short of the end, so there is floor to be thrown to.
    assert_eq!(col.sections.len(), 4);
    let xs: Vec<f32> = col.sections.iter().map(|s| s.face[0][0]).collect();
    assert!(xs.windows(2).all(|w| w[1] < w[0]) && xs[0] < door[0] && *xs.last().unwrap() > end[0] + col.throw, "{xs:?}");
    for s in &col.sections {
        assert!(lamp[0] < s.face[0][0].min(s.face[1][0]) - 12.0, "the lamp ahead of every face");
    }
    assert!(post[0] < xs[0] && post[0] > xs[1], "his post just past the first line: the door pocket goes behind him");
    assert_eq!((col.speed, col.warn, col.rest, col.catch, col.throw, col.down, col.hold), (5.0, 2.0, 12.0, 0.35, 4.0, 1.0, 0.26));
    // The last stand's rock falls where the last stand is, ahead of the last face.
    let last = c.mobs[def].phases.iter().find(|p| (p.at - 0.25).abs() < 1.0e-6).expect("his last stand");
    assert!(col.hold > last.at, "held above it until the mine is down");
    assert!(!last.debris.is_empty() && last.debris.iter().all(|p| p[0] < *xs.last().unwrap() - 2.0), "{:?}", last.debris);
}

#[test]
fn at_45_percent_a_ring_of_beams_comes_down_round_him() {
    let phases = vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, rocks_round: 7.0, ..Default::default() }];
    let (mut s, boss, h) = mine(phases, 5);
    begin(&mut s, boss, h);
    let at = s.mobs[&boss].pos;
    let ev = run_for(&mut s, DEBRIS_WARN + 0.3);
    let beams: Vec<(Id, u32, Vec3, f32)> = pieces(&s).into_iter().filter(|p| p.1 == BEAM).collect();
    assert_eq!(beams.len(), 6, "a beam over each of the six falls");
    let len = BEAM_LEN * BEAM_SCALE;
    let mut bearings = Vec::new();
    for (id, _, foot, tilt) in &beams {
        assert!(s.collapse_rubble().contains(id), "down and lying");
        assert!((tilt - std::f32::consts::FRAC_PI_2).abs() < 0.15, "lying flat: {tilt}");
        let yaw = s.objs[id].facing;
        let mid = [foot[0] + yaw.cos() * len * 0.5, foot[1] + yaw.sin() * len * 0.5, 0.0];
        let r = dist2d(mid, at);
        assert!((4.0..=10.0).contains(&r), "round him at the ring's reach: {r}");
        bearings.push((mid[1] - at[1]).atan2(mid[0] - at[0]));
        assert!(ev.iter().any(|e| matches!(e, Event::Sound { id: o, sound } if o == id && *sound == SOUND_CRACK)));
    }
    // All the way round: no two on the same side.
    bearings.sort_by(f32::total_cmp);
    for w in bearings.windows(2) {
        assert!(w[1] - w[0] > 0.6, "spread round the ring: {bearings:?}");
    }
}

#[test]
fn the_falling_mine_changes_no_blow_and_no_timing() {
    let mut phases = sectioned(2.0);
    phases.push(EnrageDef { at: 0.45, speed: 0.9, might: 1.0, rocks_round: 7.0, ..Default::default() });
    let (mut shown, sb, sh) = mine_at(phases.clone(), 21, POST);
    let (mut plain, pb, ph) = mine_at(phases, 21, POST);
    plain.bosses.looks.off = true;
    strike(&mut shown, sb, sh);
    strike(&mut plain, pb, ph);
    let mut looked = 0;
    let mut thrown = 0;
    for k in 0..3600 {
        // The same hero in each: past the line, lagging inside the section a while (to be
        // caught there), walking into the face, striking once more.
        for (s, h, b) in [(&mut shown, sh, sb), (&mut plain, ph, pb)] {
            if (300..700).contains(&k) || (1500..1900).contains(&k) {
                // (Left where the collapse puts them.)
            } else if (900..1000).contains(&k) {
                let x = s.players[&h].pos[0];
                s.move_player(h, [x - 0.2, 0.0, 0.0], 0.0, Moving::default());
            } else {
                past_the_line(s, h);
            }
            if k == 300 || k == 1500 {
                let front = s.collapse_front().map_or(0.0, |f| f.0);
                s.move_player(h, [front + 2.0, 3.0, 0.0], 0.0, Moving::default());
            }
            if k == 1500 {
                let max = s.mobs[&b].max_health;
                s.hurt_mob(b, h, max / 2);
            }
        }
        let ea = shown.step(DT);
        let eb = plain.step(DT);
        looked += ea.iter().filter(|e| matches!(e, Event::ObjChanged { .. })).count();
        thrown += caught(&ea, "Mole");
        assert!(!eb.iter().any(|e| matches!(e, Event::Beat { beat, .. } if beat == BEAT_RUMBLE || beat == BEAT_ROCKFALL)), "no shaking with the looks off");
        assert!(pieces(&plain).is_empty(), "nothing shown with the looks off");
        let (a, b) = (&shown.players[&sh], &plain.players[&ph]);
        assert_eq!((a.health, a.pos), (b.health, b.pos), "the same blows and the same hero at step {k}");
        let (a, b) = (&shown.mobs[&sb], &plain.mobs[&pb]);
        assert_eq!((a.health, a.pos, a.phase), (b.health, b.pos, b.phase), "the same boss at step {k}");
        assert_eq!(shown.collapse_state(), plain.collapse_state(), "the same front at step {k}");
        assert_eq!(shown.collapse_face_lines(), plain.collapse_face_lines(), "the same faces at step {k}");
        let falls = |s: &Survival| s.missiles.iter().map(|m| (m.at, m.land_at, m.damage, m.radius, m.heavy)).collect::<Vec<_>>();
        assert_eq!(falls(&shown), falls(&plain), "the same falls, where and when, at step {k}");
        let dangers = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::Danger { .. })).count();
        assert_eq!(dangers(&ea), dangers(&eb), "the same warnings at step {k}");
        assert_eq!(caught(&ea, "Mole"), caught(&eb, "Mole"), "the same catch at step {k}");
    }
    assert!(looked > 100, "and the looks were there to see: {looked}");
    assert!(thrown >= 1, "the catch among what was compared: {thrown}");
    assert_eq!(shown.mobs[&sb].phase, 2, "through the collapse and the ring of beams");
    assert!(shown.collapse_front().is_some_and(|f| f.1), "the whole collapse down");
    assert!(!shown.collapse_faces().is_empty() && plain.collapse_faces().is_empty(), "the faces' rock for the eye alone");
    assert_eq!(shown.rng.gen::<u64>(), plain.rng.gen::<u64>(), "the rules' dice untouched");
}

#[test]
fn the_loose_rubble_left_lying_is_capped_for_the_client() {
    let (mut s, boss, h) = mine(gauntlet(15.0, 2400.0), 17);
    begin(&mut s, boss, h);
    let mut most = 0;
    let mut gone = 0;
    while s.collapse_front().is_some_and(|f| !f.1) && s.clock < 60.0 {
        keep_ahead(&mut s, h, 6.0);
        gone += s.step(DT).iter().filter(|e| matches!(e, Event::ObjGone { .. })).count();
        most = most.max(pieces(&s).len());
    }
    assert!(s.collapse_rubble().len() <= RUBBLE_MOST);
    assert!(most <= RUBBLE_MOST + FALLING_MOST, "at most {} pieces at once: {most}", RUBBLE_MOST + FALLING_MOST);
    assert!(s.collapse_rubble().len() >= RUBBLE_MOST - 5, "the cap reached on a long run: {}", s.collapse_rubble().len());
    assert!(gone > 0, "the oldest taken away as more come down");
}

/// The filming rig's `.es restock`: after a take's `clearfoes` has taken every other boss away,
/// a reset of the dungeon (as a beaten run's) stands them all again, with the heroes still there
/// and found secrets kept.
#[test]
fn a_restock_after_clearfoes_stands_every_boss_again() {
    let mut s = Survival::new_dungeon(Content::dev(), Box::new(FlatLand::default()), 1, 0.0);
    let hero = s.join("hero", "Hero", None);
    s.below.as_mut().unwrap().open.insert("vancleef_quarters".into());
    let bosses = ["dm_rhahkzor", "dm_smite", "dm_shredder", "dm_gilnid", "dm_greenskin", "dm_cookie", "dm_vancleef"];
    let standing = |s: &Survival, key: &str| s.mobs.values().any(|m| s.content.mobs[m.def].key == key);
    for b in bosses { assert!(standing(&s, b), "{b} at the start"); }
    s.dev_clear_foes("vancleef").unwrap();
    assert!(!standing(&s, "dm_smite") && standing(&s, "dm_vancleef"));
    s.reset_below(false);
    for b in bosses { assert!(standing(&s, b), "{b} after the restock"); }
    assert!(s.players.contains_key(&hero));
    assert!(s.below.as_ref().unwrap().open.contains("vancleef_quarters"), "found secrets stay");
}

// --- the hold, the end of the collapse, the burning iron: what is seen of them -----------------

const HELD_LOOK: u32 = 11426;
const HELD_FLASH: u32 = 555;
const STAND_LOOK: u32 = 8599;
const BURN_LOOK: u32 = 18499;

/// `sectioned`, with the looks of its hold and of its end authored as the mine has them.
fn dressed(rest: f32) -> Vec<EnrageDef> {
    let mut p = sectioned(rest);
    let c = p[0].collapse.as_mut().unwrap();
    c.held_aura = HELD_LOOK;
    c.held_kit = HELD_FLASH;
    c.end_aura = STAND_LOOK;
    c.end_grow = 1.15;
    p
}

fn flashes(ev: &[Event], boss: Id) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Visual { id, kit: HELD_FLASH, .. } if *id == boss)).count()
}

/// Filmed, November 13: a hero hitting him at 26% saw one yell and then nothing, for up to 40
/// sec. The hold is on him to see: from the first blow refused until the last section is down.
#[test]
fn a_held_boss_wears_his_hold_from_the_first_blow_refused_until_the_mine_is_down() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 23, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let max = s.mobs[&boss].max_health;
    let floor = (0.26 * max as f32).round() as u32;
    assert!(s.auras_of(boss).is_empty(), "nothing on him before he is held: {:?}", s.auras_of(boss));
    // A blow that leaves him above his hold shows nothing.
    run_to(&mut s, h, (1, "wave"));
    s.events.clear();
    s.hurt_mob(boss, h, max / 10);
    assert!(s.mobs[&boss].health > floor);
    assert!(s.auras_of(boss).is_empty() && flashes(&s.events, boss) == 0, "not held yet");
    // The first blow refused: his line, the look on him, the blow seen to glance off.
    s.events.clear();
    s.hurt_mob(boss, h, max);
    assert_eq!(s.mobs[&boss].health, floor);
    assert_eq!(s.auras_of(boss), vec![HELD_LOOK], "held, and seen to be");
    assert!(s.events.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)), "the clients told");
    assert_eq!(flashes(&s.events, boss), 1);
    // Every blow refused glances off him, a flurry of them no more than a flash each 0.6 sec.
    s.events.clear();
    s.hurt_mob(boss, h, 50);
    s.hurt_mob(boss, h, 50);
    assert_eq!(flashes(&s.events, boss), 0, "not again within the gap");
    run_for(&mut s, HELD_FLASH_GAP + 0.05);
    past_the_line(&mut s, h);
    s.events.clear();
    s.hurt_mob(boss, h, 50);
    assert_eq!((flashes(&s.events, boss), s.mobs[&boss].health), (1, floor));
    assert!(!s.events.iter().any(|e| matches!(e, Event::Yell { .. })), "his line only once");
    // It stays on him through every section to come.
    run_to(&mut s, h, (3, "wave"));
    assert_eq!(s.auras_of(boss), vec![HELD_LOOK]);
    // The last section down: the hold is off him in that moment, and the next blow bites.
    let ev = run_out(&mut s, h);
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)), "the clients told");
    assert!(!s.auras_of(boss).contains(&HELD_LOOK), "the hold gone with the last section: {:?}", s.auras_of(boss));
    s.events.clear();
    s.hurt_mob(boss, h, 50);
    assert_eq!((flashes(&s.events, boss), s.mobs[&boss].health), (0, floor - 50));
}

/// A boss never held (the heroes too slow to bring him to 26%) never wears the hold.
#[test]
fn a_boss_never_brought_to_his_hold_never_wears_it() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 24, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let mut seen = false;
    while s.collapse_front().is_some_and(|f| !f.1) {
        past_the_line(&mut s, h);
        s.step(DT);
        seen |= s.auras_of(boss).contains(&HELD_LOOK);
    }
    assert!(!seen);
    assert_eq!(s.auras_of(boss), vec![STAND_LOOK], "his last stand, all the same");
}

/// Filmed, November 13: the last face stood, he said his line, and nothing on the screen changed.
/// The end of the collapse is a turn seen with no line read: the last face throws its dust out
/// with a crash, he roars, and from then on he stands larger and wears his rage.
#[test]
fn the_end_of_the_collapse_is_a_turn_that_is_seen() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 25, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let own = s.content.mobs[s.mobs[&boss].def].scale;
    run_to(&mut s, h, (3, "wave"));
    assert!(!s.auras_of(boss).contains(&STAND_LOOK));
    let mut ev = Vec::new();
    let mut turn = Vec::new();
    while s.collapse_front().is_some_and(|f| !f.1) {
        past_the_line(&mut s, h);
        turn = s.step(DT);
        ev.extend(turn.iter().cloned());
    }
    // All of it in the one step that ends it, with his line.
    assert!(turn.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "No more mine.")));
    assert!(turn.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == boss && *emote == mobs::ROAR_EMOTE)), "he roars");
    assert!(turn.iter().any(|e| matches!(e, Event::Sound { id, sound: SOUND_CRASH } if *id == boss)), "a crash");
    assert!(turn.iter().any(|e| matches!(e, Event::Grow { id, scale } if *id == boss && (*scale - own * 1.15).abs() < 1.0e-4)), "he stands larger");
    assert!(turn.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)));
    assert_eq!(s.auras_of(boss), vec![STAND_LOOK], "and wears it from then on");
    // The last face's dust, wall to wall and out across the floor toward the heroes.
    let dust: Vec<Vec3> = turn.iter().filter_map(|e| match e { Event::MissileFrom { spell: LAND_DUST, target: Target::Point(p), .. } => Some(*p), _ => None }).collect();
    assert!(dust.len() >= 2 * 2 * 8, "two rows of dust, low and high: {}", dust.len());
    assert!(dust.iter().all(|p| p[0] > LINES[3] && p[0] < LINES[3] + 5.0), "ahead of the last face, on the heroes' side");
    assert!(dust.iter().any(|p| p[1] > 9.0) && dust.iter().any(|p| p[1] < -9.0), "from wall to wall");
    assert!(turn.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", colour, width, .. } if colour == DUST_COLOUR && *width >= 19.0)), "its dust drawn across the floor");
    assert!(turn.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_ROCKFALL)), "the ground shakes");
    // Once: nothing of it again while he fights on.
    let after = run_for(&mut s, 5.0);
    assert!(!after.iter().any(|e| matches!(e, Event::Grow { .. } | Event::Emote { emote: mobs::ROAR_EMOTE, .. })));
    assert!(!after.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", .. })), "nothing more comes down");
    // He falls: nothing shows on a body.
    let left = s.mobs[&boss].health;
    s.hurt_mob(boss, h, left);
    run_for(&mut s, 0.2);
    assert!(s.mobs.get(&boss).is_none_or(|m| m.health == 0));
    assert!(s.auras_of(boss).is_empty(), "nothing burns on his body");
    let _ = ev;
}

/// The fight lost in his last stand: he is his own size again and wears nothing, for the next try.
#[test]
fn a_last_stand_lost_puts_his_size_and_his_looks_back() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 26, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let own = s.content.mobs[s.mobs[&boss].def].scale;
    run_out(&mut s, h);
    assert_eq!(s.auras_of(boss), vec![STAND_LOOK]);
    let hp = s.players[&h].health;
    s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    let ev = run_for(&mut s, 0.5);
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id, scale } if *id == boss && (*scale - own).abs() < 1.0e-4)), "his own size again");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)));
    assert!(s.auras_of(boss).is_empty(), "nothing on him: {:?}", s.auras_of(boss));
    // A fight lost before the end never grew him: no size sent.
    let (mut s, boss, h) = mine_at(dressed(2.0), 27, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (1, "wave"));
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, h, max);
    assert_eq!(s.auras_of(boss), vec![HELD_LOOK]);
    let hp = s.players[&h].health;
    s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    let ev = run_for(&mut s, 0.5);
    assert!(!ev.iter().any(|e| matches!(e, Event::Grow { .. })));
    assert!(s.auras_of(boss).is_empty(), "the hold off him too: {:?}", s.auras_of(boss));
}

/// A boss alone on a flat floor whose weapon burns from 90%.
fn smith(seed: u64) -> (Survival, Id, Id) {
    use crate::content::BurnDef;
    let burn = BurnDef { radius: 3.0, damage: 6, secs: 7.0, look: 22191, aura: BURN_LOOK };
    let (mut s, boss, h) = mine_at(vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, burn: Some(burn), ..Default::default() }], seed, POST);
    assert!(s.auras_of(boss).is_empty());
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    (s, boss, h)
}

fn flames(s: &Survival) -> Vec<Vec3> {
    s.objs.values().filter(|o| matches!(o.kind, ObjKind::Fire { .. })).map(|o| o.pos).collect()
}

/// Filmed, November 13: one patch of burning iron 3 yd across drew its spell over the whole cave
/// floor (a ground spell cast at a point covers a room), so nobody could tell where it bit. A
/// patch is drawn as wide as it bites: flames across it and its edge ringed, no room-wide spell.
#[test]
fn burning_iron_is_drawn_as_wide_as_it_bites() {
    let (mut s, boss, h) = smith(31);
    assert_eq!(s.auras_of(boss), vec![BURN_LOOK], "his weapon burns, on him to see");
    let pos = s.mobs[&boss].pos;
    s.move_player(h, [pos[0] - 30.0, pos[1], 0.0], 0.0, Moving::default());
    s.events.clear();
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, true);
    let heart = [pos[0] + 2.1, pos[1], pos[2]];
    assert!(!s.events.iter().any(|e| matches!(e, Event::Missile { spell: 22191, .. })), "no spell over the whole floor for a patch this small");
    let lit = flames(&s);
    assert_eq!(lit.len(), 1 + FIRE_FLAMES, "flames across it: {lit:?}");
    assert!(lit.iter().all(|p| dist2d(*p, heart) < 3.0), "all inside where it bites: {lit:?}");
    assert!(lit.iter().any(|p| dist2d(*p, heart) < 0.1) && lit.iter().filter(|p| dist2d(**p, heart) > 1.0).count() == FIRE_FLAMES);
    let first: Vec<Id> = s.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Fire { .. })).map(|(id, _)| *id).collect();
    // Its edge, ringed for as long as it burns, at the radius it bites.
    let ev = run_for(&mut s, 7.5);
    let rings: Vec<(f32, f32)> = ev
        .iter()
        .filter_map(|e| match e {
            Event::GroundBurst { at, shape: "ring", reach, secs, colour, .. } if colour == FIRE_COLOUR && dist2d(*at, heart) < 0.1 => Some((*reach, *secs)),
            _ => None,
        })
        .collect();
    assert!(rings.len() >= 13 && rings.len() <= 15, "redrawn each half second for its 7 sec: {}", rings.len());
    assert!(rings.iter().all(|(reach, secs)| *reach == 3.0 && *secs <= FIRE_REDRAW + 0.06), "{rings:?}");
    assert!(first.iter().all(|f| !s.objs.contains_key(f)), "burned out, every flame of it");
    // A patch wide enough for the spell (a room's worth) still casts it.
    s.bosses.burning.get_mut(&boss).unwrap().radius = GROUND_LOOK_MIN;
    s.events.clear();
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, true);
    assert!(s.events.iter().any(|e| matches!(e, Event::Missile { spell: 22191, .. })));
}

/// Filmed, November 13: the floor burned on round his body and his drops for 7 sec after he fell.
/// His fires go out with him, and when his fight is lost.
#[test]
fn his_fires_go_out_when_he_falls_and_when_his_fight_is_lost() {
    let (mut s, boss, h) = smith(32);
    let pos = s.mobs[&boss].pos;
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, true);
    let heart = [pos[0] + 2.1, pos[1], pos[2]];
    s.move_player(h, heart, 0.0, Moving::default());
    assert_eq!(flames(&s).len(), 1 + FIRE_FLAMES);
    let lit: Vec<Id> = s.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Fire { .. })).map(|(id, _)| *id).collect();
    let left = s.mobs[&boss].health;
    s.hurt_mob(boss, h, left);
    let ev = run_for(&mut s, 0.1);
    assert!(s.mobs.get(&boss).is_none_or(|m| m.health == 0), "he falls");
    assert!(flames(&s).is_empty(), "no flame left by his body");
    assert!(lit.iter().all(|f| ev.iter().any(|e| matches!(e, Event::ObjGone { id } if id == f))), "each taken from the clients");
    let hp = s.players[&h].health;
    let ev = run_for(&mut s, 8.0);
    assert_eq!(s.players[&h].health, hp, "and nothing bites where it burned");
    assert!(!ev.iter().any(|e| matches!(e, Event::GroundBurst { .. })), "nothing drawn there");
    // The fight lost: the fires out too, and the look off him.
    let (mut s, boss, h) = smith(33);
    let pos = s.mobs[&boss].pos;
    s.boss_blow_lands(boss, pos, 0.0, 3.0, 90.0, true);
    let hp = s.players[&h].health;
    s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    run_for(&mut s, 25.0);
    assert!(flames(&s).is_empty(), "no flame left for the next try");
    assert_eq!(s.mobs[&boss].health, s.mobs[&boss].max_health, "whole again");
    assert!(s.auras_of(boss).is_empty(), "his weapon cold: {:?}", s.auras_of(boss));
}

/// Filmed, November 13, on the hero's own screen: blows on him at his hold floated their full
/// numbers (29, 21) off a health bar that never moved. A blow says what it took: the one that
/// brings him to his hold shows what was left to take, and every blow after shows none and is
/// seen to glance off.
#[test]
fn a_blow_refused_at_the_hold_shows_what_it_took() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 29, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (1, "wave"));
    let max = s.mobs[&boss].max_health;
    let floor = (0.26 * max as f32).round() as u32;
    let blow = |s: &mut Survival, damage: u32| {
        s.events.clear();
        s.events.push(Event::Swing { attacker: h, victim: Some(boss), damage, outcome: Outcome::Hit, anim: false, style: Blow::Light });
        s.hurt_mob(boss, h, damage);
        s.events.iter().find_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, .. } if *v == boss => Some((*damage, *outcome)),
            _ => None,
        })
    };
    // Above his hold a blow is what it is.
    assert_eq!(blow(&mut s, 40), Some((40, Outcome::Hit)));
    // The blow that reaches the hold: what was left to take.
    let left = s.mobs[&boss].health - floor;
    assert_eq!(blow(&mut s, left + 25), Some((left, Outcome::Hit)));
    assert_eq!(s.mobs[&boss].health, floor);
    // Every blow after: nothing, off him.
    assert_eq!(blow(&mut s, 21), Some((0, Outcome::Blocked)));
    assert_eq!(blow(&mut s, 29), Some((0, Outcome::Blocked)));
    // The hold lifted, a blow is what it is again.
    run_out(&mut s, h);
    assert_eq!(blow(&mut s, 21), Some((21, Outcome::Hit)));
}

/// Two heroes at the hold in one tick: a hurt refused that has no blow of its own (a bleed, a
/// burn) leaves another hero's blow of the same size as it was, and a hero's own refused blow
/// is still the one rewritten when a friend's same-sized blow came before it.
#[test]
fn a_hurt_refused_at_the_hold_rewrites_nobody_elses_blow() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 29, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (1, "wave"));
    let friend = s.join("friend", "Friend", None);
    let floor = (0.26 * s.mobs[&boss].max_health as f32).round() as u32;
    let left = s.mobs[&boss].health - floor;
    s.hurt_mob(boss, h, left - 29);
    let swing = |by: Id| Event::Swing { attacker: by, victim: Some(boss), damage: 29, outcome: Outcome::Hit, anim: false, style: Blow::Light };
    let shown = |s: &Survival, by: Id| {
        s.events.iter().rev().find_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome, .. } if *attacker == by && *v == boss => Some((*damage, *outcome)),
            _ => None,
        })
    };
    // The friend's blow takes him to his hold, whole.
    s.events.clear();
    s.events.push(swing(friend));
    s.hurt_mob(boss, friend, 29);
    assert_eq!(s.mobs[&boss].health, floor);
    // The hero's bleed of the same size is refused in the same tick: it has no blow to rewrite.
    s.hurt_mob(boss, h, 29);
    assert_eq!(shown(&s, friend), Some((29, Outcome::Hit)), "the friend's blow keeps its number");
    // The hero's own blow of that size, refused: his is the one that glances off.
    s.events.push(swing(h));
    s.hurt_mob(boss, h, 29);
    assert_eq!(shown(&s, h), Some((0, Outcome::Blocked)));
    assert_eq!(shown(&s, friend), Some((29, Outcome::Hit)));
    assert_eq!(s.mobs[&boss].health, floor);
}

/// Filmed, November 13: with his maul lodged he stood in a stun's pose, the maul still in his
/// hand, and nothing said "now". The fire goes out of him for exactly as long as it is stuck:
/// what he wears for his burning weapon and his last stand is off him, and back as it comes free.
#[test]
fn a_lodged_weapon_is_seen_for_as_long_as_it_is_stuck() {
    let (mut s, boss, _) = smith(30);
    assert_eq!(s.auras_of(boss), vec![BURN_LOOK]);
    s.events.clear();
    s.boss_weapon_lodges(boss, 3.0);
    assert!(s.auras_of(boss).is_empty(), "out as it sticks: {:?}", s.auras_of(boss));
    assert!(s.events.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)), "the clients told");
    run_for(&mut s, 2.8);
    assert!(s.auras_of(boss).is_empty(), "still stuck at 2.8 sec");
    let ev = run_for(&mut s, 0.4);
    assert_eq!(s.auras_of(boss), vec![BURN_LOOK], "free again, it burns again");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == boss)), "the clients told");
    // A shorter lodge inside a longer one doesn't free it early.
    s.boss_weapon_lodges(boss, 3.0);
    s.boss_weapon_lodges(boss, 1.2);
    run_for(&mut s, 2.0);
    assert!(s.auras_of(boss).is_empty(), "the longer lodge still holds");
    run_for(&mut s, 1.2);
    assert_eq!(s.auras_of(boss), vec![BURN_LOOK]);
}

/// A boss held by its collapse and lodged keeps the look of its hold: it is not open.
#[test]
fn a_lodged_boss_still_held_keeps_the_look_of_its_hold() {
    let (mut s, boss, h) = mine_at(dressed(2.0), 29, POST);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    run_to(&mut s, h, (1, "wave"));
    let hp = s.mobs[&boss].health;
    s.hurt_mob(boss, h, hp);
    assert_eq!(s.auras_of(boss), vec![HELD_LOOK]);
    s.boss_weapon_lodges(boss, 3.0);
    assert_eq!(s.auras_of(boss), vec![HELD_LOOK], "held and lodged");
    // The collapse over, the look of his last stand is what a lodge takes off him.
    run_out(&mut s, h);
    run_for(&mut s, 3.2);
    assert_eq!(s.auras_of(boss), vec![STAND_LOOK]);
    s.boss_weapon_lodges(boss, 3.0);
    assert!(s.auras_of(boss).is_empty(), "the fire out of him: {:?}", s.auras_of(boss));
}

// ---- The slam: he brings it down with his own body ----

use crate::content::SlamDef;

/// The phases with the slam (as the content sets it) on the first.
fn with_slam(mut phases: Vec<EnrageDef>) -> Vec<EnrageDef> {
    phases[0].slam = Some(SlamDef::default());
    phases
}

/// A phase that is the slam alone: no collapse behind it.
fn slam_only() -> Vec<EnrageDef> {
    vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, slam: Some(SlamDef::default()), ..Default::default() }]
}

/// A wall (a pillar 4 yd across its half) whose face is 9 yd from a boss at `x` on the +y side.
fn wall_by(x: f32) -> FlatLand {
    FlatLand { walls: vec![([x, 13.0], 4.0)], ..Default::default() }
}

/// Into the phase as `begin_at` goes, with what was logged on the way.
fn begin_seen(s: &mut Survival, boss: Id, h: Id, at: Vec3) -> Vec<Event> {
    strike(s, boss, h);
    let until = s.clock + 10.0;
    let mut out = Vec::new();
    while s.mobs[&boss].phase == 0 && s.clock < until {
        s.move_player(h, at, 0.0, Moving::default());
        out.extend(s.step(DT));
    }
    assert!(s.mobs[&boss].phase > 0, "the phase begun");
    out
}

/// On for `secs`, each event with the clock it came at.
fn timed(s: &mut Survival, secs: f32) -> Vec<(f32, Event)> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        let ev = s.step(DT);
        let t = s.clock;
        out.extend(ev.into_iter().map(|e| (t, e)));
    }
    out
}

fn beat_at(ev: &[(f32, Event)], h: Id, name: &str) -> Option<f32> {
    ev.iter().find(|(_, e)| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == name)).map(|(t, _)| *t)
}

fn logged(ev: &[Event], what: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Log(t) if t.contains(what)))
}

/// On to the step the slam lands in: that step's events.
fn to_the_blow(s: &mut Survival) -> Vec<Event> {
    loop {
        assert!(s.clock < 60.0, "the slam never landed");
        let ev = s.step(DT);
        if ev.iter().any(|e| matches!(e, Event::Beat { beat, .. } if beat == BEAT_SLAM_NEAR)) {
            return ev;
        }
    }
}

/// The rings of rocks that hurt, shown in these events.
fn marked(ev: &[Event]) -> Vec<Vec3> {
    ev.iter()
        .filter_map(|e| match e {
            Event::GroundBurst { at, shape: "ring", reach, secs, .. } if *reach == DEBRIS_RADIUS && *secs == DEBRIS_WARN => Some(*at),
            _ => None,
        })
        .collect()
}

#[test]
fn he_slams_the_nearest_wall_two_seconds_after_the_yell_and_every_hero_is_shaken_by_how_near() {
    let (mut s, boss, h) = mine_in(with_slam(sectioned(12.0)), 5, POST, wall_by(POST));
    let mid = hero_at(&mut s, "Mid", [POST + 25.0, 0.0, 0.0]);
    let far = hero_at(&mut s, "Far", [POST + 58.0, 0.0, 0.0]);
    let began = begin_seen(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let yell = s.clock;
    assert!(logged(&began, "goes to slam a wall"), "he picks the wall beside him");
    assert!(!began.iter().any(|e| matches!(e, Event::Emote { id, emote } if *id == boss && *emote == mobs::ROAR_EMOTE)), "no roar with lit hands: it read as a cast");
    assert!(s.boss_slamming(boss));
    let mut ev = Vec::new();
    let mut blow = None;
    for _ in 0..300 {
        let step = s.step(DT);
        let t = s.clock;
        if blow.is_none() && step.iter().any(|e| matches!(e, Event::Beat { beat, .. } if beat == BEAT_SLAM_NEAR)) {
            blow = Some((t, s.mobs[&boss].pos, s.mobs[&boss].facing, s.collapse_state().map(|c| c.1)));
        }
        ev.extend(step.into_iter().map(|e| (t, e)));
    }
    let (blow, at, face, stage) = blow.expect("the blow lands");
    assert!((blow - (yell + 2.0)).abs() <= 3.0 * DT, "2 s after the yell: {}", blow - yell);
    // His body at the wall: he ran to it, stands short of it and faces it.
    assert!((at[0] - POST).abs() < 1.0 && at[1] > 5.5 && at[1] < 8.0, "at the wall, 9 yd off his post: {at:?}");
    assert!(face.sin() > 0.95, "facing it: {face}");
    // The swing is the ogre's own overhead blow, begun so that it lands on that frame.
    let swing = ev.iter().find_map(|(t, e)| match e {
        Event::Clip { id, clip, at, .. } if *id == boss => Some((*t, *clip, *at)),
        _ => None,
    });
    let (swung, clip, lands) = swing.expect("he swings");
    assert_eq!(clip, SlamDef::default().clip);
    assert!((swung + lands - blow).abs() <= 3.0 * DT, "the swing lands on the blow: {swung} + {lands} vs {blow}");
    assert!(!ev.iter().any(|(t, e)| *t <= blow + 0.5 && matches!(e, Event::Emote { id, emote } if *id == boss && *emote == mobs::ROAR_EMOTE)), "and no roar");
    // One frame: every hero in the mine is shaken by how near him they stand, dust comes off the
    // wall, and the first wave starts at the door.
    assert_eq!(beat_at(&ev, h, BEAT_SLAM_NEAR), Some(blow));
    assert_eq!(beat_at(&ev, mid, BEAT_SLAM_MID), Some(blow));
    assert_eq!(beat_at(&ev, far, BEAT_SLAM_FAR), Some(blow), "even at the far end of the mine");
    for (who, not) in [(h, [BEAT_SLAM_MID, BEAT_SLAM_FAR]), (mid, [BEAT_SLAM_NEAR, BEAT_SLAM_FAR]), (far, [BEAT_SLAM_NEAR, BEAT_SLAM_MID])] {
        assert!(not.iter().all(|b| beat_at(&ev, who, b).is_none()), "one shake each");
    }
    assert!(ev.iter().any(|(t, e)| *t == blow && matches!(e, Event::GroundBurst { shape: "ring", colour, .. } if colour == DUST_COLOUR)), "dust at the wall's foot");
    assert!(ev.iter().any(|(t, e)| *t == blow && matches!(e, Event::Sound { id, sound: SOUND_CRASH } if *id == boss)), "its crash");
    assert!(matches!(stage, Some("wave")), "the first wave starts with it: {stage:?}");
    // Then he is back in his fight.
    assert!(!s.boss_slamming(boss));
}

#[test]
fn the_slam_changes_none_of_the_collapses_times() {
    let run = |slam: bool| {
        let phases = if slam { with_slam(sectioned(2.0)) } else { sectioned(2.0) };
        let (mut s, boss, h) = mine_in(phases, 11, POST, wall_by(POST));
        begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
        let yell = s.clock;
        let mut seen: Vec<(usize, &'static str, f32)> = Vec::new();
        while s.collapse_front().is_some_and(|f| !f.1) {
            assert!(s.clock < 300.0, "the collapse never done");
            past_the_line(&mut s, h);
            s.step(DT);
            if let Some((k, stage, ..)) = s.collapse_state() {
                if seen.last().map(|l| (l.0, l.1)) != Some((k, stage)) {
                    seen.push((k, stage, s.clock - yell));
                }
            }
        }
        (seen, s.mobs[&boss].pos)
    };
    let ((with, at), (without, _)) = (run(true), run(false));
    assert_eq!(with.len(), without.len(), "{with:?} vs {without:?}");
    for (a, b) in with.iter().zip(&without) {
        assert_eq!((a.0, a.1), (b.0, b.1));
        assert!((a.2 - b.2).abs() <= 2.0 * DT, "section {} {} at {} with the slam, {} without", a.0, a.1, a.2, b.2);
    }
    let wave = with.iter().find(|s| s.1 == "wave").unwrap().2;
    assert!((wave - 2.0).abs() <= 3.0 * DT, "the first wave 2 s after the yell, on the slam: {wave}");
    assert!(at[0] > LINES[3], "and he ends past the last face as ever: {at:?}");
}

/// The slam alone by a wall, with three heroes in reach of its rock: his foe, one who will
/// stand and one who will step out. Returned as the yell is given.
fn slam_alone(seed: u64) -> (Survival, Id, [Id; 3]) {
    let (mut s, boss, h) = mine_in(slam_only(), seed, 0.0, wall_by(0.0));
    let stay = hero_at(&mut s, "Stay", [-7.0, -6.0, 0.0]);
    let step = hero_at(&mut s, "Step", [6.0, -9.0, 0.0]);
    begin_at(&mut s, boss, h, [3.0, 0.0, 0.0]);
    (s, boss, [h, stay, step])
}

/// Where a rock followed down strikes the floor: its first low point (it hops from there).
fn strike_of(z: &[f32]) -> usize {
    z.windows(2).position(|w| w[1] > w[0] + 1e-5).unwrap_or(z.len() - 1)
}

/// What the slam's rock did over the 3 s after the blow: each rock's track (clock, place), kind
/// and size.
type Tracks = BTreeMap<Id, (u32, f32, Vec<(f32, Vec3)>)>;

fn watch_rocks(s: &mut Survival, secs: f32) -> (Tracks, Vec<Event>) {
    let mut seen: Tracks = BTreeMap::new();
    let mut all = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        all.extend(s.step(DT));
        for (id, display, pos, _) in pieces(s) {
            let scale = match s.objs[&id].kind {
                ObjKind::Prop { scale, .. } => scale,
                _ => 0.0,
            };
            seen.entry(id).or_insert((display, scale, Vec::new())).2.push((s.clock, pos));
        }
    }
    (seen, all)
}

#[test]
fn the_slam_shakes_rock_loose_for_show_and_marks_one_under_each_hero_near() {
    let (mut s, boss, [h, stay, step]) = slam_alone(7);
    let ev = to_the_blow(&mut s);
    let blow = s.clock;
    let stood = [h, stay, step].map(|id| s.players[&id].pos);
    // A rock that can hurt is marked on the floor under each hero, 1.3 s before it lands.
    let rings = marked(&ev);
    assert_eq!(rings.len(), 3, "one under each hero: {rings:?}");
    for p in stood {
        assert!(rings.iter().any(|r| dist2d(*r, p) < 0.1), "a ring under the hero at {p:?}");
    }
    assert!(logged(&ev, "12 rocks come loose for show, 3 marked under heroes"), "{:?}", ev.iter().filter(|e| matches!(e, Event::Log(_))).collect::<Vec<_>>());
    // One steps out of the ring; one stands in it. (His foe steps out too.)
    s.move_player(step, [11.0, -12.0, 0.0], 0.0, Moving::default());
    s.move_player(h, [3.0, -5.0, 0.0], 0.0, Moving::default());
    let before = [stay, step].map(|id| s.players[&id].health);
    let mut struck_at = None;
    let (mut seen, mut all): (Tracks, Vec<Event>) = (BTreeMap::new(), Vec::new());
    for _ in 0..30 {
        let (more, ev) = watch_rocks(&mut s, 0.1);
        for (id, (d, sc, t)) in more {
            seen.entry(id).or_insert((d, sc, Vec::new())).2.extend(t);
        }
        all.extend(ev);
        if struck_at.is_none() && s.players[&stay].health < before[0] {
            struck_at = Some(s.clock);
        }
    }
    let struck_at = struck_at.expect("the hero who stood in the ring is struck");
    assert!((struck_at - (blow + DEBRIS_WARN)).abs() <= 0.1 + DT, "as its rock lands, {DEBRIS_WARN} s after the ring: {}", struck_at - blow);
    let cost = before[0] - s.players[&stay].health;
    assert_eq!(cost, s.boss_blow(boss, SlamDef::default().share), "for the slam's share of his blow, once");
    assert_eq!(s.players[&step].health, before[1], "the hero who stepped out is not touched");
    assert!(marked(&all).is_empty(), "no ring after the blow's own frame");
    // The rest is for show: seen falling from overhead, in three sizes, one after another over
    // 1.5 s, each striking, hopping and rolling to rest; none on a hero.
    let rest = |t: &Vec<(f32, Vec3)>| t.last().unwrap().1;
    let show: Vec<&(u32, f32, Vec<(f32, Vec3)>)> = seen.values().filter(|(_, _, t)| rings.iter().all(|r| dist2d(*r, rest(t)) > 1.0)).collect();
    assert_eq!(show.len(), 12, "3 off the wall and 9 round him");
    assert_eq!(seen.len(), show.len() + 3, "and the 3 that were marked");
    let mut sizes: Vec<(u32, u32)> = show.iter().map(|(d, sc, _)| (*d, (sc * 1000.0) as u32)).collect();
    sizes.sort();
    sizes.dedup();
    assert_eq!(sizes.len(), 3, "three sizes: {sizes:?}");
    let mut downs = Vec::new();
    for (_, _, t) in &show {
        let end = rest(t);
        assert!(t[0].1[2] - end[2] >= 1.5, "seen falling, not appearing: from {:?} to {end:?}", t[0].1);
        let k = strike_of(&t.iter().map(|(_, p)| p[2]).collect::<Vec<_>>());
        let (down, hit) = t[k];
        assert!(down - t[0].0 >= 0.35, "on its way down long enough to be seen: {}", down - t[0].0);
        assert!(t[..=k].windows(2).all(|w| w[1].1[2] <= w[0].1[2] + 1e-4), "down without a stop: {:?}", &t[..=k.min(12)]);
        let hop = t[k..].iter().map(|(_, p)| p[2] - end[2]).fold(0.0, f32::max);
        assert!(hop > 0.05 && dist2d(hit, end) > 0.1, "it hops and rolls to rest, not stopping dead: {hop}");
        assert!(stood.iter().all(|p| dist2d(*p, end) >= LOOSE_CLEAR - 0.01), "none where a hero stood: {end:?}");
        downs.push(down - blow);
    }
    downs.sort_by(f32::total_cmp);
    assert!(downs[0] >= 0.3 && *downs.last().unwrap() <= SlamDef::default().volley + 0.05, "over 1.5 s from the blow: {downs:?}");
    assert!(downs.windows(2).filter(|w| w[1] - w[0] > 0.02).count() >= 8, "one after another, not all at once: {downs:?}");
    for id in seen.keys() {
        assert!(s.collapse_rubble().contains(id), "each lies where it fell");
    }
}

#[test]
fn a_rock_for_show_hurts_nobody_it_lands_on() {
    // The same fight twice: the first shows where the biggest rock for show lands; in the
    // second a hero walks under it after the blow and stands there.
    let play = |under: Option<Vec3>| {
        let (mut s, _, [h, stay, step]) = slam_alone(7);
        let ev = to_the_blow(&mut s);
        let rings = marked(&ev);
        s.move_player(h, [3.0, -5.0, 0.0], 0.0, Moving::default());
        s.move_player(stay, [-12.0, -10.0, 0.0], 0.0, Moving::default());
        if let Some(at) = under {
            s.move_player(step, at, 0.0, Moving::default());
        } else {
            s.move_player(step, [11.0, -12.0, 0.0], 0.0, Moving::default());
        }
        let before = s.players[&step].health;
        let (seen, _) = watch_rocks(&mut s, 3.0);
        let biggest = seen
            .values()
            .filter(|(_, _, t)| rings.iter().all(|r| dist2d(*r, t.last().unwrap().1) > 1.0))
            .max_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(_, _, t)| t.last().unwrap().1)
            .unwrap();
        (biggest, before - s.players[&step].health)
    };
    let (spot, _) = play(None);
    let (again, cost) = play(Some([spot[0], spot[1], 0.0]));
    assert!(dist2d(spot, again) < 0.01, "the same rock on the same spot: {spot:?} vs {again:?}");
    assert_eq!(cost, 0, "it lands on the hero and does nothing");
}

#[test]
fn nothing_lands_on_the_lit_band_or_under_a_hero_on_it() {
    // The collapse's first line is at 30 yd; he stands at 35. One hero stands on the band.
    let (mut s, boss, h) = mine_in(with_slam(sectioned(12.0)), 5, POST, wall_by(POST));
    let on_band = hero_at(&mut s, "Band", [LINES[0] + 1.0, 4.0, 0.0]);
    begin_at(&mut s, boss, h, [POST + 4.0, -3.0, 0.0]);
    let ev = to_the_blow(&mut s);
    let rings = marked(&ev);
    let (p, q) = (s.players[&h].pos, s.players[&on_band].pos);
    assert!(rings.iter().any(|r| dist2d(*r, p) < 0.1), "his foe is marked");
    assert!(rings.iter().all(|r| dist2d(*r, q) > 1.0), "no rock is dropped on the band, where the hero must cross");
    let (seen, _) = watch_rocks(&mut s, 2.4);
    // (The wave's own rows fall behind the line, from the door: they are not the slam's.)
    // (And what never moved is no rock: the band's own lamps.)
    let near_him: Vec<(Vec3, Vec3, f32)> = seen.values().map(|(_, sc, t)| (t.last().unwrap().1, t[0].1, *sc)).filter(|p| p.0[0] > LINES[0] - 6.0 && p.0 != p.1).collect();
    assert!(near_him.len() >= 8, "rock round him: {}", near_him.len());
    for (p, from, size) in &near_him {
        assert!((p[0] - LINES[0]).abs() >= LOOSE_BAND - 0.01 || p[1].abs() > 10.0 + LOOSE_BAND - 0.01, "the band stays clear to read and to cross: {p:?} (from {from:?}, size {size}), he at {:?}", s.mobs[&boss].pos);
    }
    // At the door too, where the collapse starts.
    let at_door = seen.values().filter(|(_, sc, t)| t.last().unwrap().1[0] < 7.0 && LOOSE.iter().any(|l| l.1 == *sc)).count();
    assert!(at_door >= 4, "rock at the door: {at_door}");
}

#[test]
fn he_can_be_hit_at_his_slam_and_nothing_stops_it() {
    let (mut s, boss, [h, ..]) = slam_alone(9);
    let yell = s.clock;
    run_for(&mut s, 0.5);
    assert!(s.boss_slamming(boss));
    let hp = s.mobs[&boss].health;
    s.hurt_mob(boss, h, 500);
    assert_eq!(s.mobs[&boss].health, hp - 500, "hit like any other");
    // A stagger, and a stance filled past its limit: neither takes him out of it.
    s.stagger_mob(boss, 3.0, false);
    let limit = s.posture_limit(boss);
    s.mobs.get_mut(&boss).unwrap().posture_guard = 0.0;
    assert!(!s.build_posture(boss, h, limit * 3), "his stance does not break");
    assert!(!s.disabled(boss), "nor is he staggered");
    let ev = timed(&mut s, 2.0);
    let blow = beat_at(&ev, h, BEAT_SLAM_NEAR).expect("the blow lands all the same");
    assert!((blow - (yell + 2.0)).abs() <= 3.0 * DT, "on its time: {}", blow - yell);
    // After it he breaks as ever.
    run_for(&mut s, 1.0);
    assert!(!s.boss_slamming(boss));
    assert!(s.build_posture(boss, h, limit * 3), "his stance breaks again once it is done");
}

#[test]
fn with_no_wall_in_reach_he_slams_the_floor_where_he_stands() {
    let (mut s, boss, h) = mine_at(slam_only(), 3, 0.0);
    let began = begin_seen(&mut s, boss, h, [3.0, 0.0, 0.0]);
    assert!(logged(&began, "goes to slam the floor"), "open ground all round");
    let from = s.mobs[&boss].pos;
    let ev = to_the_blow(&mut s);
    assert!(dist2d(s.mobs[&boss].pos, from) < 0.6, "he does not wander off: {from:?} to {:?}", s.mobs[&boss].pos);
    assert!(logged(&ev, "slams the floor: 9 rocks come loose for show"), "the rock still comes down");
}

#[test]
fn he_slams_the_frame_of_the_front_door_when_the_fight_is_near_it_and_is_not_buried() {
    // The door's frame: rock behind the route's start. His fight is 8 yd inside it.
    let land = FlatLand { walls: vec![([-4.6, 0.0], 3.5)], ..Default::default() };
    let (mut s, boss, h) = mine_in(with_slam(sectioned(12.0)), 5, 8.0, land);
    let began = begin_seen(&mut s, boss, h, [11.0, 0.0, 0.0]);
    let yell = s.clock;
    assert!(logged(&began, "goes to slam the door's frame"), "{:?}", began.iter().filter(|e| matches!(e, Event::Log(_))).collect::<Vec<_>>());
    to_the_blow(&mut s);
    let m = &s.mobs[&boss];
    assert!((s.clock - (yell + 2.0)).abs() <= 3.0 * DT, "on its time, from 8 yd off: {}", s.clock - yell);
    assert!(dist2d(m.pos, [0.0, 0.0, 0.0]) < 1.0 && m.facing.cos() < -0.9, "at the frame, facing it: {:?} {}", m.pos, m.facing);
    // The wave starts under him; he walks out of the section ahead of it, as in any wave.
    while s.collapse_state().map(|c| (c.0, c.1)) == Some((0, "wave")) {
        assert!(s.clock < 60.0);
        past_the_line(&mut s, h);
        s.step(DT);
    }
    let m = &s.mobs[&boss];
    assert!(m.health > 0 && m.pos[0] > LINES[0], "past the first face before it stands: {:?}", m.pos);
}

#[test]
fn each_later_section_goes_as_he_pounds_the_floor_and_each_face_is_felt_as_it_lands() {
    let (mut s, boss, h) = mine_in(with_slam(sectioned(2.0)), 13, POST, wall_by(POST));
    let far = hero_at(&mut s, "Far", [LINES[0] + 22.0, 0.0, 0.0]);
    let off = hero_at(&mut s, "Off", [LINES[0] + 45.0, 0.0, 0.0]);
    begin_at(&mut s, boss, h, [POST + 3.0, 0.0, 0.0]);
    let mut ev: Vec<(f32, Event)> = Vec::new();
    let mut first_rest = None;
    while s.collapse_front().is_some_and(|f| !f.1) {
        assert!(s.clock < 300.0, "the collapse never done");
        past_the_line(&mut s, h);
        let step = s.step(DT);
        let t = s.clock;
        if first_rest.is_none() && s.collapse_state().map(|c| (c.0, c.1)) == Some((0, "rest")) {
            first_rest = Some(t);
        }
        ev.extend(step.into_iter().map(|e| (t, e)));
    }
    let done = s.clock;
    // (The last face's first rock lands a moment after the mine is down.)
    for _ in 0..(1.0 / DT) as usize {
        let step = s.step(DT);
        let t = s.clock;
        ev.extend(step.into_iter().map(|e| (t, e)));
    }
    // One swing for the opening and one for each later section; no roar until the mine is down.
    let swings = ev.iter().filter(|(_, e)| matches!(e, Event::Clip { id, clip, .. } if *id == boss && *clip == SlamDef::default().clip)).count();
    assert!(swings >= 3, "the opening's slam and a pound for the later sections: {swings}");
    assert!(!ev.iter().any(|(t, e)| *t < done && matches!(e, Event::Emote { id, emote } if *id == boss && *emote == mobs::ROAR_EMOTE)), "no roar with lit hands before the end");
    // (The hero is kept just past each line: beside him for some, a way off for others.)
    let pounds = ev.iter().filter(|(_, e)| matches!(e, Event::Beat { player, beat, .. } if *player == h && (beat == BEAT_SLAM_MID || beat == BEAT_SLAM_FAR))).count();
    assert_eq!(pounds + 1, swings, "each pound felt by the hero in sight of him, a size under the slam");
    assert_eq!(ev.iter().filter(|(_, e)| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_SLAM_NEAR)).count(), 1);
    // Each face landing shakes the heroes near it: hard beside it, less a way off, not at all
    // far down the mine.
    let thuds = ev.iter().filter(|(_, e)| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_FACE_NEAR)).count();
    assert_eq!(thuds, 4, "a face for each section");
    let first_rest = first_rest.unwrap();
    let first = |who: Id, name: &str| ev.iter().find(|(_, e)| matches!(e, Event::Beat { player, beat, .. } if *player == who && beat == name)).map(|(t, _)| *t);
    let felt = first(h, BEAT_FACE_NEAR).unwrap();
    assert!(felt >= first_rest - 2.0 * DT && felt <= first_rest + FACE_FIRST + 0.1, "as its first rock lands: {felt} vs {first_rest}");
    assert_eq!(first(far, BEAT_FACE_FAR), Some(felt), "less, 22 yd off");
    assert!(first(off, BEAT_FACE_NEAR).is_none_or(|t| t > felt) && first(off, BEAT_FACE_FAR).is_none_or(|t| t > felt), "nothing 45 yd off");
}

#[test]
fn the_slams_rock_goes_when_the_fight_is_lost() {
    let (mut s, boss, heroes) = slam_alone(7);
    to_the_blow(&mut s);
    run_for(&mut s, 0.3);
    assert!(!pieces(&s).is_empty() || !s.missiles.is_empty(), "rock in the air");
    for h in heroes {
        let hp = s.players[&h].health;
        s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    }
    run_for(&mut s, 30.0);
    assert!(pieces(&s).is_empty(), "nothing of it left lying: {}", pieces(&s).len());
    assert!(!s.boss_slamming(boss));
    let m = &s.mobs[&boss];
    assert_eq!(m.health, m.max_health, "and he is whole at his post");
}
