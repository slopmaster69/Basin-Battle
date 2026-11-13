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

// --- how the falling mine looks ----------------------------------------------------------------

use crate::content::{CollapseDef, EnrageDef};

/// Rhahk'Zor alone at the mouth of a flat mine, its phases `phases`, and a hero who can take it.
fn mine(phases: Vec<EnrageDef>, seed: u64) -> (Survival, Id, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == "rhahkzor");
    d.packs[0].at = [0.0, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [0.0, -20.0, 0.0];
    let def = c.mob("dm_rhahkzor").unwrap();
    c.mobs[def].phases = phases;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
    let boss = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == "dm_rhahkzor").unwrap().0;
    let h = s.join("mole", "Mole", None);
    s.move_player(h, [3.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let p = s.players.get_mut(&h).unwrap();
    p.max_health = 10_000_000;
    p.health = 10_000_000;
    s.engage(boss, mobs::Foe::Hero(h));
    (s, boss, h)
}

/// The gauntlet down a straight tunnel along +x, as the real one runs (its lead, crush and hold).
fn gauntlet(speed: f32, len: f32) -> Vec<EnrageDef> {
    let route = vec![[0.0, 0.0, 0.0], [len, 0.0, 0.0]];
    vec![EnrageDef { at: 0.9, speed: 1.0, might: 1.0, collapse: Some(CollapseDef { route, speed, lead: 14.0, crush: 0.2, hold: 0.26, end_text: String::new(), held_text: String::new() }), ..Default::default() }]
}

/// A blow that brings on its phase.
fn strike(s: &mut Survival, boss: Id, h: Id) {
    let max = s.mobs[&boss].max_health;
    s.hurt_mob(boss, h, max / 5);
}

/// Into the phase, as a blow brings it (once what the boss was doing is done).
fn begin(s: &mut Survival, boss: Id, h: Id) {
    strike(s, boss, h);
    while s.mobs[&boss].phase == 0 && s.clock < 10.0 {
        keep_ahead(s, h, 6.0);
        s.step(DT);
    }
    assert!(s.mobs[&boss].phase > 0, "the phase begun");
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

fn run_for(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

#[test]
fn rocks_are_seen_falling_from_overhead_and_landing_at_the_front() {
    let (mut s, boss, h) = mine(gauntlet(4.0, 400.0), 7);
    begin(&mut s, boss, h);
    let mut heights: BTreeMap<Id, Vec<f32>> = BTreeMap::new();
    let mut landed: BTreeMap<Id, (Vec3, f32)> = BTreeMap::new();
    let mut ev = Vec::new();
    for _ in 0..1000 {
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
    let rock = |s: &Survival, id: &Id| matches!(s.objs[id].kind, ObjKind::Prop { display, .. } if display == ROCK || display == BOULDER);
    let fell: Vec<Id> = landed
        .keys()
        .copied()
        .filter(|id| rock(&s, id))
        .filter(|id| heights[id].first().copied().unwrap_or(0.0) >= 3.5 && heights[id].windows(2).all(|w| w[1] <= w[0] + 1e-4))
        .collect();
    assert!(fell.len() >= 20, "rock seen dropping from overhead: {}", fell.len());
    for id in &fell {
        let z = s.objs[id].pos[2];
        assert!(z < 0.0 && z > -1.0, "lying sunk in the floor, not hanging: {z}");
        assert!(ev.iter().any(|e| matches!(e, Event::ObjChanged { id: o } if o == id)), "moved in place as it fell");
    }
    // The front's own wall: rock landing right at the front, all along the way.
    let at_front = landed.values().filter(|(p, front)| (p[0] - front).abs() <= WALL_STEP + 2.0).count();
    assert!(at_front >= 10, "rock landing at the front: {at_front}");
    let front = s.collapse_front().unwrap().0;
    assert!(front > 35.0, "the front came on: {front}");
    for x in (0..(front as i32 - 4)).step_by(6) {
        assert!(landed.values().any(|(p, _)| (p[0] - x as f32).abs() <= 3.5), "the tunnel buried at {x} yd");
    }
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
    // The ground shakes for the hero near the front, and rock lands beside them.
    assert!(ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_RUMBLE)));
    assert!(ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == h && beat == BEAT_ROCKFALL)));
    // Dust rolls ahead of the front and is thrown up where rock lands, its line on the floor; no
    // Rocky Ground Impact (its pink boulders linger on film).
    assert!(ev.iter().any(|e| matches!(e, Event::MissileFrom { spell: DUST_ROLL, .. })));
    assert!(ev.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", .. })));
    let dust = ev.iter().filter(|e| matches!(e, Event::MissileFrom { spell: LAND_DUST, .. })).count();
    assert!(dust >= landed.len(), "dust where each piece lands: {dust} for {}", landed.len());
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
    while s.collapse_rubble().is_empty() && s.clock < 5.0 {
        s.step(DT);
        if let Some((_, _, pos, _)) = pieces(&s).into_iter().next() {
            shown.get_or_insert((s.clock, pos[2]));
            assert!(pos[2] > -1.0, "never below its resting place");
        }
    }
    let (at, high) = shown.expect("its rock showed");
    assert!((at - (blow - ROCK_SECS)).abs() <= 2.0 * DT, "it starts down {ROCK_SECS} s before the blow: {at} vs {blow}");
    assert!(high > 4.0, "from overhead: {high}");
    assert!((s.clock - blow).abs() <= 2.0 * DT, "landing as the blow lands: {} vs {blow}", s.clock);
    let id = s.collapse_rubble()[0];
    let p = s.objs[&id].pos;
    assert!(dist2d(p, [5.0, 2.0, 0.0]) < 0.01, "on the spot: {p:?}");
    run_for(&mut s, 30.0);
    assert!(s.objs.contains_key(&id), "and it stays");
}

#[test]
fn rubble_stays_behind_the_front_and_goes_when_the_fight_is_lost() {
    let (mut s, boss, h) = mine(gauntlet(4.0, 400.0), 11);
    begin(&mut s, boss, h);
    for _ in 0..800 {
        keep_ahead(&mut s, h, 6.0);
        s.step(DT);
    }
    let early = s.collapse_rubble();
    assert!(early.len() > 10, "rubble lying: {}", early.len());
    for _ in 0..500 {
        keep_ahead(&mut s, h, 6.0);
        s.step(DT);
    }
    assert!(early.iter().all(|id| s.objs.contains_key(id)), "what came down stays as the front goes on");
    let front = s.collapse_front().unwrap().0;
    let behind = early.iter().filter(|id| s.objs[*id].pos[0] < front - 2.0).count();
    assert!(behind * 10 >= early.len() * 8, "and lies behind it: {behind} of {}", early.len());
    // Every hero down: the fight is lost, and the mine stands again for the next try.
    let hp = s.players[&h].health;
    s.hurt_player_by(h, hp + 10_000, boss, "a blow");
    let ev = run_for(&mut s, 5.0);
    assert_eq!(s.collapse_front(), None, "the collapse is over");
    assert!(s.collapse_rubble().is_empty());
    assert!(pieces(&s).is_empty(), "nothing of it left standing");
    assert!(early.iter().all(|id| !s.objs.contains_key(id) && ev.iter().any(|e| matches!(e, Event::ObjGone { id: o } if o == id))),
        "each taken from the clients");
}

#[test]
fn rubble_goes_with_a_reset_of_the_dungeon() {
    let (mut s, boss, h) = mine(gauntlet(4.0, 400.0), 13);
    begin(&mut s, boss, h);
    for _ in 0..600 {
        keep_ahead(&mut s, h, 6.0);
        s.step(DT);
    }
    assert!(!s.collapse_rubble().is_empty());
    s.reset_below(false);
    run_for(&mut s, 0.5);
    assert!(s.collapse_rubble().is_empty());
    assert!(pieces(&s).is_empty());
    assert_eq!(s.collapse_front(), None);
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
    let mut phases = gauntlet(4.0, 300.0);
    phases.push(EnrageDef { at: 0.45, speed: 0.9, might: 1.0, rocks_round: 7.0, ..Default::default() });
    let (mut shown, sb, sh) = mine(phases.clone(), 21);
    let (mut plain, pb, ph) = mine(phases, 21);
    plain.bosses.looks.off = true;
    strike(&mut shown, sb, sh);
    strike(&mut plain, pb, ph);
    let mut looked = 0;
    for k in 0..3000 {
        // The same hero in each: keeping ahead, falling behind a while, striking once more.
        let by = if (800..1100).contains(&k) { -3.0 } else { 5.0 };
        keep_ahead(&mut shown, sh, by);
        keep_ahead(&mut plain, ph, by);
        if k == 1500 {
            let max = shown.mobs[&sb].max_health;
            shown.hurt_mob(sb, sh, max / 2);
            plain.hurt_mob(pb, ph, max / 2);
        }
        let ea = shown.step(DT);
        let eb = plain.step(DT);
        looked += ea.iter().filter(|e| matches!(e, Event::ObjChanged { .. })).count();
        assert!(!eb.iter().any(|e| matches!(e, Event::Beat { beat, .. } if beat == BEAT_RUMBLE || beat == BEAT_ROCKFALL)), "no shaking with the looks off");
        assert!(pieces(&plain).is_empty(), "nothing shown with the looks off");
        assert_eq!(shown.players[&sh].health, plain.players[&ph].health, "the same blows at step {k}");
        let (a, b) = (&shown.mobs[&sb], &plain.mobs[&pb]);
        assert_eq!((a.health, a.pos, a.phase), (b.health, b.pos, b.phase), "the same boss at step {k}");
        assert_eq!(shown.collapse_front(), plain.collapse_front(), "the same front at step {k}");
        let falls = |s: &Survival| s.missiles.iter().map(|m| (m.at, m.land_at, m.damage, m.radius, m.heavy)).collect::<Vec<_>>();
        assert_eq!(falls(&shown), falls(&plain), "the same falls, where and when, at step {k}");
        let dangers = |ev: &[Event]| ev.iter().filter(|e| matches!(e, Event::Danger { .. })).count();
        assert_eq!(dangers(&ea), dangers(&eb), "the same warnings at step {k}");
    }
    assert!(looked > 100, "and the looks were there to see: {looked}");
    assert_eq!(shown.mobs[&sb].phase, 2, "through the collapse and the ring of beams");
    assert!(shown.collapse_front().is_some_and(|f| f.0 > 40.0), "well along the gauntlet");
    assert_eq!(shown.rng.gen::<u64>(), plain.rng.gen::<u64>(), "the rules' dice untouched");
}

#[test]
fn the_rubble_left_lying_is_capped_for_the_client() {
    let (mut s, boss, h) = mine(gauntlet(30.0, 2400.0), 17);
    begin(&mut s, boss, h);
    let mut most = 0;
    let mut gone = 0;
    while s.collapse_front().is_some_and(|f| !f.1) && s.clock < 120.0 {
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
