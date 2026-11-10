use super::*;
use crate::land::{FlatLand, Land};

const DT: f32 = 0.05;

/// A hero (or more) beside him at the pull, the pull itself not yet stepped.
fn fresh(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 19, 1_000_000.0);
    let band: Vec<_> = (0..heroes).map(|i| {
        let h = s.join(&format!("hero{i}"), &format!("Hero{i}"), None);
        let p = s.players.get_mut(&h).unwrap();
        p.pos = [2.5, -(i as f32), 0.0];
        p.facing = std::f32::consts::PI;
        p.health = 100_000;
        p.max_health = 100_000;
        p.loadout.clear();
        h
    }).collect();
    let id = s.spawn_mob(s.content.mob(key).unwrap(), [0.0, 0.0, 0.0], None, false);
    s.engage(id, Foe::Hero(band[0]));
    (s, band, id)
}

/// A fight already past the pull's two bangs on the pot.
fn fight(key: &str, heroes: usize) -> (Survival, Vec<Id>, Id) {
    let (mut s, band, id) = fresh(key, heroes);
    s.step(DT);
    if let Some(c) = s.kitchen.cooks.get_mut(&id) {
        c.pulled = true;
        if matches!(c.stage, Some(Stage::Pull { .. })) { c.stage = None; }
    }
    (s, band, id)
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
}

fn until(s: &mut Survival, id: Id, limit: f32, f: impl Fn(&Survival, Id) -> bool) -> Vec<Event> {
    let end = s.clock + limit;
    let mut ev = Vec::new();
    while !f(s, id) && s.clock < end { ev.extend(s.step(DT)); }
    assert!(f(s, id), "clock {}, state {:?}, turn {}", s.clock, s.mobs[&id].state,
        s.kitchen.cooks.get(&id).map_or(99, |c| c.turn));
    ev
}

fn scoop(s: &mut Survival, id: Id) {
    until(s, id, 60.0, |s, id| s.kitchen_scooping(id));
}

fn turn(s: &Survival, id: Id) -> u32 { s.kitchen.cooks[&id].turn }

fn pot_of(s: &Survival, id: Id) -> &Pot { &s.kitchen.pots[&s.kitchen.cooks[&id].pot.unwrap()] }

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

fn bursts(ev: &[Event], colour: &str) -> Vec<(Vec3, f32, &'static str, f32, f32, f32)> {
    ev.iter().filter_map(|e| match e {
        Event::GroundBurst { at, yaw, shape, reach, arc, secs, colour: c, .. } if c == colour => Some((*at, *yaw, *shape, *reach, *arc, *secs)),
        _ => None,
    }).collect()
}

fn yelled(ev: &[Event], id: Id, line: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text == line))
}

fn sounded(ev: &[Event], id: Id, sound: u32) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Sound { id: who, sound: s } if *who == id && *s == sound)).count()
}

fn clips(ev: &[Event], id: Id, clip: u16) -> usize {
    ev.iter().filter(|e| matches!(e, Event::Clip { id: who, clip: c, .. } if *who == id && *c == clip)).count()
}

fn held(ev: &[Event], id: Id) -> Vec<(u16, f32)> {
    ev.iter().filter_map(|e| match e { Event::HoldPose { id: who, clip, secs, .. } if *who == id => Some((*clip, *secs)), _ => None }).collect()
}

fn landed_on(ev: &[Event], id: Id, h: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, .. } if *attacker == id && *v == h && *damage > 0))
}

// --- the pull and the rhythm --------------------------------------------------------------------

#[test]
fn cookie_bangs_his_tankard_on_the_pot_twice_before_his_first_blow() {
    let (mut s, _, id) = fresh(COOKIE, 1);
    let ev = run(&mut s, 1.55);
    assert_eq!(clips(&ev, id, BANG_CLIP), 2, "two bangs");
    assert_eq!(sounded(&ev, id, BANG), 2, "each one rings on the pot");
    assert!(yelled(&ev, id, "Mrrgl! Cookie's kitchen!"));
    assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "no blow while he bangs");
    assert_eq!(s.kitchen_hand(id), Some(MUG_HELD), "the tankard in his hand");
    let pot = s.kitchen.cooks[&id].pot.unwrap();
    assert!(matches!(s.objs[&pot].kind, ObjKind::Prop { display: POT, .. }));
    let ev = until(&mut s, id, 4.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
    assert!(ev.iter().any(|e| matches!(e, Event::MobGear { id: who } if *who == id)));
    assert_eq!(s.kitchen_hand(id), None, "the cleaver back for the first pair");
    assert_eq!(s.kitchen_move(id), Some("cleaver"));
}

#[test]
fn any_blow_let_go_counts_its_exchange_once_and_an_unreleased_one_never() {
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
                    if interrupt == "second" {
                        until(&mut s, id, 2.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                    }
                }
                s.stagger_mob(id, 0.8, false);
                assert!(s.mobs[&id].pattern.is_none());
                run(&mut s, 0.85);
                let counted = u32::from(interrupt != "first");
                assert_eq!(turn(&s, id), counted, "{key}, {interrupt}, RTT {rtt}");
                assert_eq!(s.players[&h].health, hp, "the avoided and interrupted blows never land");
                until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
                if counted == 0 {
                    assert_eq!(s.kitchen_move(id), Some(pair.as_str()), "an unreleased pair is tried again");
                } else {
                    assert_ne!(s.kitchen_move(id), Some(pair.as_str()), "a pair with a blow let go moves the rhythm on");
                }
                assert_eq!(s.mobs[&id].pattern.unwrap().1, 0);
            }
        }
    }
}

#[test]
fn a_hero_staggered_by_the_first_slap_still_sees_the_belch() {
    for rtt in [0.1, 0.35, 0.0] {
        let (mut s, h, id) = production_fight(GORGED, rtt);
        s.players.get_mut(&h).unwrap().health = 100_000;
        s.players.get_mut(&h).unwrap().max_health = 100_000;
        let first = until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        let _ = first;
        let began = s.clock;
        assert_eq!(s.kitchen_move(id), Some("slaps"));
        // He never guards, never dodges: every first slap lands and staggers him.
        until(&mut s, id, 15.0, |s, id| s.kitchen_move(id) == Some("belch"));
        assert!(s.clock - began < 15.0, "the belch within 15 s of the first slap, RTT {rtt}: {}", s.clock - began);
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
        assert_eq!(turn(&s, id), 1, "a belch never let go doesn't count");
        assert!(s.kitchen.cooks[&id].stage.is_none());
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert_eq!(s.kitchen_move(id), Some("belch"));
        move_beside_locked_blow(&mut s, &[h], id);
        until(&mut s, id, 3.0, |s, id| matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })));
        let released = s.clock;
        assert_eq!(turn(&s, id), 2);
        let ev = run(&mut s, COUGH - 0.1);
        assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)));
        until(&mut s, id, 8.0, |s, id| matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert!(s.clock >= released + COUGH);
        assert_eq!(s.kitchen_move(id), Some("slaps"));
        assert_eq!(s.players[&h].health, s.players[&h].max_health);
    }
}

#[test]
fn cookies_rhythm_is_cleaver_ladle_pour_and_he_fights_on_while_the_puddle_cools() {
    for heroes in [1, 2, 4] {
        let (mut s, _, id) = fight(COOKIE, heroes);
        until(&mut s, id, 8.0, |s, id| s.kitchen_move(id) == Some("cleaver"));
        until(&mut s, id, 10.0, |s, id| s.kitchen_move(id) == Some("ladle"));
        assert_eq!(turn(&s, id), 1);
        until(&mut s, id, 10.0, |s, id| s.kitchen_move(id) == Some("serve it hot"));
        assert_eq!(turn(&s, id), 2);
        until(&mut s, id, 4.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
        assert_eq!(turn(&s, id), 3);
        // While it burns he goes straight on (here to his first scoop): no standing idle by it.
        until(&mut s, id, 3.5, |s, id| matches!(s.kitchen.cooks[&id].stage, Some(Stage::Seeking { .. }))
            || matches!(s.mobs[&id].state, MobState::Windup { .. }));
        assert!(s.kitchen.cooks[&id].patch.is_some(), "he moves on while the puddle still burns");
        assert!(!s.mobs[&id].enraged && s.mobs[&id].phase == 0);
    }
}

#[test]
fn the_scoops_come_after_3_then_4_then_5_exchanges() {
    let (mut s, band, id) = fight(COOKIE, 1);
    s.players.get_mut(&band[0]).unwrap().pos = [2.5, 0.0, 0.0];
    let mut seen = Vec::new();
    let mut last = 0u32;
    let mut total = 0u32;
    for _ in 0..(200.0 / DT) as usize {
        let before = turn(&s, id);
        let was = s.kitchen.cooks[&id].scoops;
        s.step(DT);
        total += turn(&s, id) - before;
        if s.kitchen.cooks[&id].scoops > was {
            seen.push(total - last);
            last = total;
            if seen.len() == 3 { break; }
            // Break each scoop, so the pot stays full and he comes back.
            scoop(&mut s, id);
            s.kitchen_broken(id);
            run(&mut s, REEL + 0.1);
        }
    }
    assert_eq!(seen, vec![3, 4, 5]);
}

// --- Serve It Hot ---------------------------------------------------------------------------------

#[test]
fn serve_it_hot_lifts_a_tankard_and_throws_it_in_a_visible_arc_onto_where_the_hero_stood() {
    for stand in [2.5, 6.0] {
        let (mut s, band, id) = fight(COOKIE, 1);
        s.kitchen.cooks.get_mut(&id).unwrap().turn = 2;
        s.move_player(band[0], [stand, 0.0, 0.0], std::f32::consts::PI, Moving::default());
        let ev = until(&mut s, id, 8.0, |s, id| s.kitchen_winding(id, "serve it hot"));
        let from = s.mobs[&id].pos;
        assert_eq!(s.kitchen_hand(id), Some(MUG_HELD), "the cleaver swapped for the tankard");
        assert!(ev.iter().any(|e| matches!(e, Event::MobGear { id: who } if *who == id)));
        assert!(held(&ev, id).contains(&(HOLD_CLIP, 1.8)), "the tankard held up the whole wind-up");
        assert_eq!(sounded(&ev, id, BUBBLING), 1);
        let at = s.kitchen.cooks[&id].patch.as_ref().unwrap().at;
        let d = dist2d(from, at);
        assert!((POUR_NEAR - 0.01..=POUR_FAR + 0.01).contains(&d), "lands {d} yd off");
        if stand > POUR_NEAR { assert!(dist2d(at, [stand, 0.0, 0.0]) < 0.3, "on the hero's own spot"); }
        else { assert!(dist2d(at, [stand, 0.0, 0.0]) <= RADIUS, "pushed out, still over the hero"); }
        let warn = bursts(&ev, HAZARD_COLOUR);
        assert!(warn.iter().any(|b| b.2 == "ring" && dist2d(b.0, at) < 0.01 && (b.3 - RADIUS).abs() < 0.01),
            "where it lands drawn at its true size from the start, markers or not");
        // He follows the hero no more: the spot stays.
        s.move_player(band[0], [stand, 4.0, 0.0], 0.0, Moving::default());
        let mut ev = until(&mut s, id, 2.5, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
        let released = s.clock;
        let p = s.kitchen.cooks[&id].patch.as_ref().unwrap();
        assert_eq!(p.at, at);
        let lands = p.hot_at.unwrap();
        assert!(lands - released >= 0.3 - DT, "at least 0.3 s in the air: {}", lands - released);
        assert!(clips(&ev, id, THROW_CLIP) == 1);
        assert!(held(&ev, id).contains(&(0, 0.0)), "the held lift let go on the throw");
        assert_eq!(s.kitchen_hand(id), Some(0), "his hand empty once the tankard flies");
        let left = lands - s.clock + DT;
        ev.extend(run(&mut s, left));
        let legs: Vec<Vec3> = ev.iter().filter_map(|e| match e {
            Event::Missile { caster, spell: MUG, target: Target::Point(p) } if *caster == id => Some(*p),
            Event::MissileFrom { caster, spell: MUG, target: Target::Point(p), .. } if *caster == id => Some(*p),
            _ => None,
        }).collect();
        assert_eq!(legs.len(), LEGS, "the mug flies its arc in {LEGS} legs");
        assert!(legs[0][2] > at[2] + HAND + 0.5, "up first");
        assert_eq!(*legs.last().unwrap(), at, "and down onto the spot");
        assert_eq!(sounded(&ev, id, SPLASH), 1, "a splash as it lands");
        assert!(bursts(&ev, STEW_COLOUR).iter().any(|b| dist2d(b.0, at) < 0.01 && (b.3 - RADIUS).abs() < 0.01 && b.5 >= 1.0), "a tan ring at its true edge, held over a second");
        let patch = s.kitchen.cooks[&id].patch.as_ref().unwrap();
        assert_eq!(patch.props.len(), EDGE_PUFFS);
        assert!(patch.props.iter().all(|p| (dist2d(s.objs[p].pos, at) - (RADIUS - 0.2)).abs() < 0.05), "puffs along the edge, none in the middle");
        assert!(patch.props.iter().all(|p| s.is_kitchen_prop(*p)));
        run(&mut s, 2.0);
        assert_eq!(s.kitchen_hand(id), None, "the cleaver back after the throw's recovery");
    }
}

#[test]
fn the_puddle_burns_inside_its_edge_until_the_last_puff_drops_and_never_outside() {
    let (mut s, band, id) = fight(COOKIE, 2);
    s.kitchen.cooks.get_mut(&id).unwrap().turn = 2;
    for h in &band { s.move_player(*h, [6.0, 0.0, 0.0], std::f32::consts::PI, Moving::default()); }
    until(&mut s, id, 8.0, |s, id| s.kitchen_winding(id, "serve it hot"));
    let at = s.kitchen.cooks[&id].patch.as_ref().unwrap().at;
    // One just inside the edge, one just outside it.
    s.move_player(band[0], [at[0], at[1] + RADIUS - 0.15, at[2]], 0.0, Moving::default());
    s.move_player(band[1], [at[0], at[1] + RADIUS + 0.15, at[2]], 0.0, Moving::default());
    let hp = [s.players[&band[0]].health, s.players[&band[1]].health];
    until(&mut s, id, 3.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some_and(|t| s.clock >= t)));
    let start = s.kitchen.cooks[&id].patch.as_ref().unwrap().hot_at.unwrap();
    // Held where he stands, so only the puddle can touch them.
    s.stagger_mob(id, HOT + FADE + 1.0, false);
    let mut ev = run(&mut s, HOT - 0.1);
    assert_eq!(s.kitchen.cooks[&id].patch.as_ref().unwrap().props.len(), EDGE_PUFFS, "every puff while it's hot");
    let mut counts = Vec::new();
    while s.clock < start + HOT + FADE + 0.2 {
        ev.extend(s.step(DT));
        counts.push(s.kitchen.cooks[&id].patch.as_ref().map_or(0, |p| p.props.len()));
    }
    counts.dedup();
    assert!(counts.len() >= EDGE_PUFFS, "the puffs drop away one by one: {counts:?}");
    assert!(counts.windows(2).all(|w| w[1] < w[0]));
    assert!(s.kitchen.cooks[&id].patch.is_none());
    assert!(s.players[&band[0]].health < hp[0], "just inside burns");
    assert_eq!(s.players[&band[1]].health, hp[1], "just outside never does");
    let ticks = ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, .. } if *attacker == id && *v == band[0] && *damage > 0)).count();
    assert!(ticks >= 4, "20 as it lands, then 4 a second while any steam shows: {ticks}");
    // Its edge is drawn while it burns and stops soon after.
    let last = bursts(&ev, HAZARD_COLOUR).iter().filter(|b| dist2d(b.0, at) < 0.01).count();
    assert!(last >= 8);
}

#[test]
fn a_pour_broken_off_in_its_windup_leaves_nothing_behind() {
    let (mut s, band, id) = fight(COOKIE, 1);
    s.kitchen.cooks.get_mut(&id).unwrap().turn = 2;
    until(&mut s, id, 8.0, |s, id| s.kitchen_winding(id, "serve it hot"));
    s.stagger_mob(id, 1.0, false);
    run(&mut s, DT);
    assert!(s.kitchen.cooks[&id].patch.is_none());
    let ev = run(&mut s, 0.9);
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, .. } if *caster == id)));
    assert!(bursts(&ev, HAZARD_COLOUR).iter().all(|b| b.2 != "ring"), "its warning stops being drawn");
    until(&mut s, id, 12.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| p.hot_at.is_some()));
    s.move_player(band[0], [100.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, DT);
    assert!(!s.kitchen.cooks.contains_key(&id));
    assert!(s.pending.iter().all(|p| p.attacker != id));
    assert!(matches!(s.mobs[&id].state, MobState::Return));
}

// --- the scoop ----------------------------------------------------------------------------------

#[test]
fn he_scoops_beside_the_pot_side_on_dipping_his_tankard_with_a_slurp_each_time() {
    let (mut s, _, id) = fight(COOKIE, 1);
    let mut ev = until(&mut s, id, 60.0, |s, id| s.kitchen_scooping(id));
    let spot = pot_of(&s, id).spot;
    let pot = pot_of(&s, id).at;
    assert!(dist2d(s.mobs[&id].pos, spot) <= 0.8);
    assert!((dist2d(spot, pot) - 1.6).abs() < 0.01, "beside it");
    assert!(angle_between(s.mobs[&id].facing, facing(s.mobs[&id].pos, pot)) < 1.0, "facing the pot");
    assert!(ev.iter().any(|e| matches!(e, Event::MobGear { id: who } if *who == id)));
    assert_eq!(s.kitchen_hand(id), Some(MUG_HELD));
    ev.extend(run(&mut s, SCOOP - 0.2));
    assert!(yelled(&ev, id, "Mmm... one taste.") || s.lines.iter().any(|l| l.text == "Mmm... one taste."));
    assert!(clips(&ev, id, DIP_CLIP) >= 3, "dips again and again, not one frozen pose");
    assert_eq!(clips(&ev, id, DIP_CLIP), sounded(&ev, id, SLURP));
}

#[test]
fn a_lone_heros_two_seconds_of_blows_or_one_heavy_blow_breaks_the_scoop_but_two_jabs_dont() {
    // Two jabs: not enough.
    let (mut s, band, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    let at = s.mobs[&id].pos;
    s.move_player(band[0], [at[0] - 1.2, at[1], at[2]], 0.0, Moving::default());
    let mut jabs = 0;
    let mut hits = 0;
    while jabs < 2 && s.kitchen_scooping(id) {
        if matches!(s.players[&band[0]].action, Action::Ready) { s.input(band[0], Input::Attack); jabs += 1; }
        let ev = s.step(DT);
        hits += ev.iter().filter(|e| matches!(e, Event::Health { id: who, .. } if *who == id)).count();
    }
    run(&mut s, 0.6);
    assert!(hits > 0);
    assert!(s.kitchen_scooping(id) || s.kitchen.cooks[&id].healed > 0, "two jabs don't spill it");
    // Committed light blows: broken in about 2 s.
    let (mut s, band, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    let at = s.mobs[&id].pos;
    s.move_player(band[0], [at[0] - 1.2, at[1], at[2]], 0.0, Moving::default());
    let began = s.clock;
    while s.kitchen_scooping(id) && s.clock < began + SCOOP {
        s.input(band[0], Input::Attack);
        s.step(DT);
    }
    let took = s.clock - began;
    assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })), "broken");
    assert!((1.4..=2.8).contains(&took), "about 2 s of blows: {took}");
    // One heavy blow.
    let (mut s, band, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    let at = s.mobs[&id].pos;
    s.move_player(band[0], [at[0] - 1.2, at[1], at[2]], 0.0, Moving::default());
    s.input(band[0], Input::Heavy);
    for _ in 0..40 {
        if !s.kitchen_scooping(id) { break; }
        s.step(DT);
        if matches!(s.players[&band[0]].action, Action::Ready) { break; }
    }
    run(&mut s, 0.4);
    assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. })), "one heavy blow spills it");
}

#[test]
fn breaking_the_scoop_knocks_the_pot_splashes_stew_drops_its_steam_and_leaves_him_reeling_open() {
    let (mut s, band, id) = fight(COOKIE, 1);
    scoop(&mut s, id);
    let pot = s.kitchen.cooks[&id].pot.unwrap();
    let stew = s.kitchen.pots[&pot].stew.clone();
    assert_eq!(stew.len(), 1 + HELPINGS as usize, "bubbles and two plumes over a full pot");
    let at = s.mobs[&id].pos;
    s.move_player(band[0], [at[0] - 1.2, at[1], at[2]], 0.0, Moving::default());
    let mut ev = Vec::new();
    while s.kitchen_scooping(id) {
        s.input(band[0], Input::Attack);
        ev.extend(s.step(DT));
    }
    assert!(held(&ev, id).contains(&(OPEN_CLIP, REEL)), "the open look, held the whole reel");
    assert!(bursts(&ev, STEW_COLOUR).iter().any(|b| b.2 == "ring" && dist2d(b.0, s.kitchen.pots[&pot].at) < 0.01), "stew splashes round the pot");
    assert!(sounded(&ev, id, SPLASH) >= 1);
    assert!(stew.iter().all(|p| !s.objs.contains_key(p)), "the pot's steam drops");
    assert_eq!(s.kitchen.cooks[&id].healed, 0);
    // Beaten while he reels.
    let before = s.mobs[&id].health;
    let reel_end = s.clock + REEL - 0.3;
    while s.clock < reel_end {
        s.input(band[0], Input::Attack);
        ev.extend(s.step(DT));
    }
    assert!(s.mobs[&id].health < before, "his health falls through the reel");
    assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "no blow while he reels");
    assert!(yelled(&ev, id, "MRGLGLGL!") || s.lines.iter().any(|l| l.text == "MRGLGLGL!"));
    run(&mut s, 0.5);
    assert_eq!(s.kitchen.pots[&pot].stew.len(), 1 + HELPINGS as usize, "a knock slops it, it doesn't empty it");
}

#[test]
fn a_scoop_left_alone_heals_then_breathes_a_steam_cone_drawn_to_its_shape_and_after_two_gulps_the_pot_is_empty() {
    let (mut s, band, id) = fight(COOKIE, 1);
    let max = s.mobs[&id].max_health;
    s.mobs.get_mut(&id).unwrap().health = max / 2;
    let original = s.mobs[&id].health;
    let mut said = Vec::new();
    for turn in 0..2u32 {
        scoop(&mut s, id);
        let at = s.mobs[&id].pos;
        s.move_player(band[0], [at[0] + 2.5, at[1], at[2]], 0.0, Moving::default());
        let ev = until(&mut s, id, SCOOP + 1.0, |s, id| s.kitchen_move(id) == Some("steam"));
        said.extend(ev.iter().cloned());
        assert_eq!(s.mobs[&id].health, original + (turn + 1) * (max / 25));
        assert!(sounded(&ev, id, GULP) == 1);
        let (pos, dir) = (s.mobs[&id].pos, s.mobs[&id].facing);
        let warn = bursts(&ev, HAZARD_COLOUR);
        assert!(warn.iter().any(|b| b.2 == "wedge" && (b.3 - 5.4).abs() < 0.01 && (b.4 - 65.0).abs() < 0.01 && (b.5 - 1.6).abs() < 0.01),
            "the cone drawn at the reach and width it hits, the whole wind-up: {warn:?}");
        assert!(held(&ev, id).contains(&(HOLD_CLIP, 1.6)), "the breath drawn in, held");
        // One beside the cone, one just inside its far edge.
        s.move_player(band[0], [pos[0] - dir.sin() * 3.0, pos[1] + dir.cos() * 3.0, pos[2]], 0.0, Moving::default());
        let hp = s.players[&band[0]].health;
        let ev = run(&mut s, 1.7);
        assert_eq!(s.players[&band[0]].health, hp, "beside the cone is safe");
        assert_eq!(s.mobs[&id].facing, dir);
        assert!(ev.iter().any(|e| matches!(e, Event::Visual { id: who, kit: STEAM_KIT, .. } if *who == id)));
        let puffs = &s.kitchen.cooks[&id].puffs;
        assert_eq!(puffs.len(), 6);
        assert!(puffs.iter().all(|(p, _)| { let q = s.objs[p].pos; dist2d(q, pos) <= 5.4 && angle_between(dir, facing(pos, q)) <= 33.0 }), "the steam fills the cone");
        let spread = puffs.iter().map(|(p, _)| angle_between(dir, facing(pos, s.objs[p].pos))).fold(0.0f32, f32::max);
        assert!(spread > 15.0, "out to its sides, not only down its middle");
        assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { until }) if (until - s.clock - STEAM_RECOVER).abs() < 0.2), "a short opening");
        said.extend(run(&mut s, STEAM_RECOVER + 0.1));
        if turn == 0 { assert_eq!(pot_of(&s, id).stew.len(), 2, "one plume left"); }
    }
    assert_eq!(pot_of(&s, id).helpings, 0);
    assert!(pot_of(&s, id).stew.is_empty(), "the pot's steam is gone: you can see it's empty");
    assert!(yelled(&said, id, "Mrrgl... pot's empty.") || s.lines.iter().any(|l| l.text == "Mrrgl... pot's empty."));
    // No scoop at an empty pot.
    let ev = run(&mut s, 60.0);
    assert!(!yelled(&ev, id, "Mmm... one taste."));
    assert_eq!(s.kitchen.cooks[&id].healed, 2);
}

#[test]
fn a_cone_hits_inside_and_misses_outside_its_drawn_edge() {
    for inside in [true, false] {
        let (mut s, band, id) = fight(COOKIE, 1);
        scoop(&mut s, id);
        until(&mut s, id, SCOOP + 1.0, |s, id| s.kitchen_move(id) == Some("steam"));
        let (pos, dir) = (s.mobs[&id].pos, s.mobs[&id].facing);
        let r = if inside { 5.2 } else { 5.7 };
        s.move_player(band[0], [pos[0] + dir.cos() * r, pos[1] + dir.sin() * r, pos[2]], 0.0, Moving::default());
        let hp = s.players[&band[0]].health;
        run(&mut s, 1.8);
        assert_eq!(s.players[&band[0]].health < hp, inside, "at {r} yd");
    }
}

#[test]
fn a_short_stagger_delays_the_gulp_but_only_a_posture_break_spills_it() {
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

#[test]
fn waiting_out_every_scoop_still_dies_to_a_plain_starting_weapon_solo_and_duo() {
    for heroes in [1, 2] {
        let (mut s, band, id) = fight(COOKIE, heroes);
        let mut scoops = 0;
        let mut spill = false;
        let mut attacks = 0;
        for _ in 0..12000 {
            if s.mobs[&id].health == 0 { break; }
            // Let every scoop complete. Attack the ordinary recoveries after the first taste.
            if scoops > 0 && s.kitchen.cooks.get(&id).is_some_and(|c| c.stage.is_none() || matches!(c.stage, Some(Stage::Coughing { .. })))
                && matches!(s.mobs[&id].state, MobState::Recover { .. }) {
                let at = s.mobs[&id].pos;
                for (n, h) in band.iter().enumerate() {
                    s.move_player(*h, [at[0] + 1.2, at[1] + n as f32 * 0.3, at[2]], std::f32::consts::PI, Moving::default());
                    s.input(*h, Input::Attack);
                    attacks += 1;
                }
            }
            let ev = s.step(DT);
            scoops += ev.iter().filter(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text == "Mmm... one taste.")).count();
            spill |= ev.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text == "MRGLGLGL!"));
        }
        assert!(attacks > 0 && scoops > 0);
        assert!(!spill, "the controller did not interrupt a scoop");
        assert_eq!(s.mobs[&id].health, 0, "waiting remained winnable at {} sec", s.clock);
        println!("Cookie {heroes} hero, every scoop left alone: {:.2} sec, {scoops} scoops", s.clock);
    }
}

#[test]
fn an_unreachable_pot_gives_up_without_healing_or_teleporting() {
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

struct NoRoute;
impl Land for NoRoute {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn plans(&self) -> bool { true }
    fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
}

// --- death, the pot, and the reset ----------------------------------------------------------------

#[test]
fn the_pot_stays_through_his_death_and_only_a_reset_takes_it() {
    for end in [0, 1, 2] {
        let (mut s, band, id) = fight(COOKIE, 1);
        s.kitchen.cooks.get_mut(&id).unwrap().turn = 2;
        until(&mut s, id, 22.0, |s, id| s.kitchen.cooks[&id].patch.as_ref().is_some_and(|p| !p.props.is_empty()));
        let props: Vec<_> = s.kitchen.cooks[&id].patch.as_ref().unwrap().props.clone();
        let pot = s.kitchen.cooks[&id].pot.unwrap();
        let stew = s.kitchen.pots[&pot].stew.clone();
        assert!(!s.is_kitchen_prop(pot), "the solid cauldron keeps its normal collision");
        assert!(stew.iter().all(|p| s.is_kitchen_prop(*p)));
        s.events.clear();
        match end {
            0 => s.hurt_mob(id, band[0], u32::MAX),
            1 => { s.players.get_mut(&band[0]).unwrap().health = 0; s.step_kitchen(id); }
            _ => s.reset_kitchens(),
        }
        s.remove_kitchen(id);
        for p in &props {
            assert!(!s.objs.contains_key(p));
            assert_eq!(s.events.iter().filter(|e| matches!(e, Event::ObjGone { id: who } if who == p)).count(), 1);
        }
        assert!(!s.kitchen.cooks.contains_key(&id));
        let kept = end != 2;
        assert_eq!(s.objs.contains_key(&pot), kept, "end {end}");
        assert_eq!(stew.iter().all(|p| s.objs.contains_key(p)), kept);
        if end == 1 {
            // Back home, the same pot, full again, and a fresh rhythm.
            s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
            s.clean_kitchens();
            assert_eq!(s.kitchen.cooks[&id].pot, Some(pot));
            assert_eq!(s.kitchen.pots[&pot].helpings, HELPINGS);
            assert_eq!(s.kitchen.cooks[&id].turn, 0);
            assert_eq!(s.kitchen.cooks[&id].healed, 0);
            assert!(s.kitchen.cooks[&id].patch.is_none());
            assert_eq!(s.kitchen.pots.len(), 1);
        }
    }
}

#[test]
fn the_dungeon_wipe_restores_one_cook_and_a_full_pot() {
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
    s.hurt_player_by(h, u32::MAX, id, "test wipe");
    run(&mut s, 0.1);
    let alive: Vec<_> = s.mobs.iter().filter(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == COOKIE).collect();
    assert_eq!(alive.len(), 1);
    assert_eq!(alive[0].1.health, alive[0].1.max_health);
    assert!(!s.slain("cookie"));
    let fresh = *alive[0].0;
    s.clean_kitchens();
    assert_eq!(s.kitchen.cooks[&fresh].healed, 0);
    assert!(s.kitchen.cooks[&fresh].patch.is_none());
    assert_eq!(s.kitchen.pots.len(), 1, "one pot, never two");
    assert_eq!(pot_of(&s, fresh).helpings, HELPINGS);
}

// --- Gorged -------------------------------------------------------------------------------------

#[test]
fn gorged_lies_two_seconds_walks_to_the_pot_gulps_it_empty_swells_past_cookie_and_waits_before_his_first_slap() {
    let (mut s, band, id) = fight(COOKIE, 1);
    let item = s.content.item("secret_ingredient").unwrap();
    s.give(band[0], item, 1);
    let pot = s.kitchen.cooks[&id].pot.unwrap();
    let stew = s.kitchen.pots[&pot].stew.clone();
    s.move_player(band[0], [-3.0, -2.0, 0.0], 0.0, Moving::default());
    s.events.clear();
    s.hurt_mob(id, band[0], u32::MAX);
    let mut ev: Vec<Event> = s.events.drain(..).collect();
    assert!(s.objs.contains_key(&pot), "the pot stays as he falls");
    assert!(yelled(&ev, id, "...burnt.") || s.lines.iter().any(|l| l.text == "...burnt."));
    let next = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == GORGED).unwrap().0;
    assert!(s.mobs[&next].hidden);
    ev.extend(run(&mut s, 1.9));
    assert!(s.mobs[&next].hidden, "the body lies still 2 s");
    let fell = s.clock;
    ev.extend(until(&mut s, next, 0.5, |s, n| !s.mobs[&n].hidden));
    assert!(!ev.iter().any(|e| matches!(e, Event::Visual { kit: 356, .. })), "no arcane swirl");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id: who, kit: combat::KIT_FALL_DUST, .. } if *who == next)), "he gets up out of a cloud of dust");
    assert!(bursts(&ev, STEW_COLOUR).iter().any(|b| b.2 == "ring" && b.3 >= 3.0));
    assert_eq!(s.content.mobs[s.mobs[&next].def].name, "Gorged Cookie", "his own name, so the forms can be told apart");
    assert_ne!(s.content.mobs[s.mobs[&next].def].name, s.content.mobs[s.content.mob(COOKIE).unwrap()].name);
    assert!(!ev.iter().any(|e| matches!(e, Event::Yell { text, .. } if text.contains("gulps"))), "no narration yelled");
    let cookie_scale = s.content.mobs[s.content.mob(COOKIE).unwrap()].scale;
    let gorged_scale = s.content.mobs[s.mobs[&next].def].scale;
    assert_eq!(s.content.mobs[s.mobs[&next].def].display, s.content.mobs[s.content.mob(COOKIE).unwrap()].display);
    assert_eq!(s.kitchen_scale(next), Some(cookie_scale), "he gets up at Cookie's size");
    assert_eq!(s.kitchen.cooks[&next].pot, Some(pot), "the pot is his");
    // He walks to it and gulps.
    ev.extend(until(&mut s, next, 7.0, |s, n| matches!(s.kitchen.cooks[&n].stage, Some(Stage::Rising(Rise::Gulping { .. })))));
    assert!(ev.iter().any(|e| matches!(e, Event::MobMove { id: who, .. } if *who == next)), "he walks there");
    assert!(dist2d(s.mobs[&next].pos, s.kitchen.pots[&pot].spot) <= 0.8);
    let gulp = until(&mut s, next, 3.0, |s, n| matches!(s.kitchen.cooks[&n].stage, Some(Stage::Rising(Rise::Swelling { .. }))));
    assert!(clips(&gulp, next, DIP_CLIP) >= 2 && sounded(&gulp, next, SLURP) >= 2, "slurps it");
    assert!(stew.iter().all(|p| !s.objs.contains_key(p)) && s.kitchen.pots[&pot].stew.is_empty(), "empty: no steam over it");
    let swelled_from = s.clock;
    let swell = until(&mut s, next, 2.0, |s, n| matches!(s.kitchen.cooks[&n].stage, Some(Stage::Rising(Rise::Waiting { .. }))));
    let grows: Vec<f32> = swell.iter().filter_map(|e| match e { Event::Grow { id: who, scale } if *who == next => Some(*scale), _ => None }).collect();
    assert!(grows.len() >= 8, "a swelling, not a jump: {grows:?}");
    let top = grows.iter().cloned().fold(0.0f32, f32::max);
    let peak = grows.iter().position(|g| *g == top).unwrap();
    assert!(grows[..=peak].windows(2).all(|w| w[1] > w[0]) && grows[peak..].windows(2).all(|w| w[1] < w[0]), "up past his size, then settling: {grows:?}");
    assert!(top > gorged_scale * 1.05);
    assert!((s.clock - swelled_from - SWELL).abs() < 0.15);
    assert!((grows.last().unwrap() - gorged_scale).abs() < 1e-4);
    assert!(gorged_scale >= cookie_scale * 1.4, "clearly bigger than Cookie");
    assert!(yelled(&swell, next, "MORE STEW!") || s.lines.iter().any(|l| l.text == "MORE STEW!"));
    assert!(swell.iter().any(|e| matches!(e, Event::Emote { id: who, emote: ROAR_EMOTE } if *who == next)));
    let grown = s.clock;
    let wait = until(&mut s, next, 4.0, |s, n| matches!(s.mobs[&n].state, MobState::Windup { .. }));
    assert!(s.clock - grown >= SECOND_FORM_WAIT - DT, "2 s before his first blow");
    assert!(!wait.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == next && s.clock < grown + SECOND_FORM_WAIT - DT)));
    assert!(s.clock - fell < 12.0);
    assert_eq!(s.kitchen_move(next), Some("slaps"));
}

#[test]
fn gorgeds_belch_rears_back_swells_and_hits_only_inside_its_drawn_cone_then_he_coughs() {
    let (mut s, band, id) = fight(GORGED, 2);
    move_beside_locked_blow(&mut s, &band, id);
    let ev = until(&mut s, id, 15.0, |s, id| s.kitchen_move(id) == Some("belch"));
    let (at, dir) = (s.mobs[&id].pos, s.mobs[&id].facing);
    let warn = bursts(&ev, HAZARD_COLOUR);
    assert!(warn.iter().any(|b| b.2 == "wedge" && (b.3 - 7.4).abs() < 0.01 && (b.4 - 70.0).abs() < 0.01), "drawn to the length that hits: {warn:?}");
    assert!(held(&ev, id).contains(&(HOLD_CLIP, 2.2)));
    s.move_player(band[0], [at[0] - dir.sin() * 3.0, at[1] + dir.cos() * 3.0, at[2]], 0.0, Moving::default());
    s.move_player(band[1], [at[0] + dir.cos() * 7.2, at[1] + dir.sin() * 7.2, at[2]], 0.0, Moving::default());
    let hp = [s.players[&band[0]].health, s.players[&band[1]].health];
    let ev = run(&mut s, 1.6);
    assert_eq!(s.players[&band[1]].health, hp[1], "the wind-up is a long warning");
    let pulse: Vec<f32> = ev.iter().filter_map(|e| match e { Event::Grow { id: who, scale } if *who == id => Some(*scale), _ => None }).collect();
    assert!(pulse.len() >= 4 && pulse.windows(2).all(|w| w[1] >= w[0]), "a swelling pulse: {pulse:?}");
    let ev = run(&mut s, 0.8);
    assert_eq!(s.players[&band[0]].health, hp[0]);
    assert!(s.players[&band[1]].health < hp[1], "the far edge of the drawn cone still hits");
    assert!(ev.iter().any(|e| matches!(e, Event::Visual { id: who, kit: BELCH_KIT, .. } if *who == id)));
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id: who, scale } if *who == id && (*scale - s.kitchen.cooks[&id].scale).abs() < 1e-4)), "back to his size");
    assert_eq!(s.mobs[&id].facing, dir);
    assert!(matches!(s.kitchen.cooks[&id].stage, Some(Stage::Coughing { until }) if until - s.clock > 3.5));
    assert!(held(&ev, id).contains(&(OPEN_CLIP, COUGH)));
    assert_eq!(s.mobs.len(), 1, "no adds");
    assert!(s.content.mobs[s.mobs[&id].def].phases.is_empty());
}

#[test]
fn a_belch_broken_off_lets_go_of_its_pulse() {
    let (mut s, band, id) = fight(GORGED, 1);
    move_beside_locked_blow(&mut s, &band, id);
    until(&mut s, id, 15.0, |s, id| s.kitchen_move(id) == Some("belch"));
    run(&mut s, 0.6);
    s.stagger_mob(id, 1.0, false);
    let ev = run(&mut s, DT * 2.0);
    let scale = s.kitchen.cooks[&id].scale;
    assert!(ev.iter().any(|e| matches!(e, Event::Grow { id: who, scale: g } if *who == id && (*g - scale).abs() < 1e-4)));
    assert!(s.kitchen.cooks[&id].pulse.is_none());
}

// --- a hero who plays by what they see ---------------------------------------------------------

/// What both forms cost a hero playing by their tells alone.
#[derive(Debug, Default)]
struct Dinner {
    cleared: bool,
    cookie_secs: f32,
    gorged_secs: f32,
    hits_taken: u32,
    damage_taken: u32,
    health_left: u32,
    scoops: u32,
    broken: u32,
    pours: u32,
    belches: u32,
}

/// A hazard drawn on the ground, as a client draws it: ring or wedge, until when.
struct Drawn { at: Vec3, yaw: f32, ring: bool, reach: f32, arc: f32, seen: f32, until: f32 }

impl Drawn {
    fn holds(&self, p: Vec3, margin: f32) -> bool {
        let d = dist2d(self.at, p);
        if self.ring { d <= self.reach + margin } else { d <= self.reach + margin && (d < 0.8 + margin || angle_between(self.yaw, facing(self.at, p)) <= self.arc / 2.0 + 12.0) }
    }
    /// The nearest way out: away from a ring's middle, across a wedge.
    fn way_out(&self, p: Vec3) -> f32 {
        if self.ring { return facing(self.at, p); }
        let side = (self.yaw.cos() * (p[1] - self.at[1]) - self.yaw.sin() * (p[0] - self.at[0])).signum();
        self.yaw + side.max(0.0).mul_add(2.0, -1.0) * std::f32::consts::FRAC_PI_2
    }
}

/// A warrior with the starting weapon and their own health, reading only what the client shows:
/// the hazards drawn on the ground (step out of them), a wind-up (guard it as it lands), the held
/// open look and the dipping tankard (hit him), and a blow just landed (one hit back).
fn dinner_by_tells(seed: u64, secret: bool) -> Dinner {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), seed, 0.0);
    s.content.pvp = Default::default();
    let hero = s.join("hero", "Hero", None);
    if secret {
        let item = s.content.item("secret_ingredient").unwrap();
        s.give(hero, item, 1);
    }
    s.move_player(hero, [9.0, 0.5, 0.0], std::f32::consts::PI, Moving::default());
    let cookie = s.spawn_mob(s.content.mob(COOKIE).unwrap(), [0.0; 3], None, false);
    s.engage(cookie, Foe::Hero(hero));
    let reaction = crate::tests_fair::REACTION;
    let weapon = s.weapon_of(&s.players[&hero]).clone();
    let mut out = Dinner::default();
    let start = s.clock;
    let mut drawn: Vec<Drawn> = Vec::new();
    let mut open_until = -1.0f32;
    let mut scoop_until = -1.0f32;
    // A blow seen coming (guard it as it lands), and one just landed (hit back once).
    let mut threat: Option<(f32, f32)> = None;
    let mut answer_until = -1.0f32;
    let mut gorged_from: Option<f32> = None;
    // The way out of what's drawn, kept while it works: turned a quarter when something solid
    // (the pot, his body) stops the hero.
    let mut escape: Option<f32> = None;
    let mut last = s.players[&hero].pos;
    while s.clock - start < 420.0 {
        let now = s.clock;
        if s.players[&hero].health == 0 { break; }
        // The cook standing: whichever kitchen body is up and alive.
        let Some(boss) = s.mobs.iter().filter(|(id, m)| m.health > 0 && !m.hidden && s.is_kitchen(**id)).map(|(id, _)| *id).next() else {
            if s.mobs.values().any(|m| m.hidden && m.health > 0) { s.step(DT); continue; }
            out.cleared = true;
            break;
        };
        if s.is_gorged(boss) && gorged_from.is_none() {
            gorged_from = Some(now);
            out.cookie_secs = now - start;
        }
        let (bpos, me) = (s.mobs[&boss].pos, s.players[&hero].pos);
        let d = dist2d(bpos, me);
        let toward = facing(me, bpos);
        let ready = matches!(s.players[&hero].action, Action::Ready);
        let guarding = matches!(s.players[&hero].action, Action::Guard);
        drawn.retain(|h| h.until > now);
        let danger = drawn.iter().find(|h| now >= h.seen && h.holds(me, 0.6));
        let mut acted = false;
        if let Some(h) = danger {
            // Step out of what's drawn on the ground.
            if guarding { s.input(hero, Input::GuardUp); }
            let stuck = escape.is_some() && dist2d(last, me) < 0.1;
            let way = match escape {
                Some(w) if !stuck => w,
                _ => {
                    // The way out nearest the plain one that the pot and his body leave open.
                    let plain = h.way_out(me);
                    let pots: Vec<Vec3> = s.objs.values().filter(|o| matches!(o.kind, ObjKind::Prop { display: POT, .. })).map(|o| o.pos).collect();
                    (0..16).map(|k| plain + k as f32 * std::f32::consts::PI / 8.0)
                        .filter(|w| {
                            let to = [me[0] + w.cos() * 2.5, me[1] + w.sin() * 2.5, me[2]];
                            !drawn.iter().any(|d| d.holds(to, 0.3)) && pots.iter().all(|p| dist2d(*p, to) > 1.8) && dist2d(bpos, to) > 1.5
                        })
                        .min_by(|a, b| angle_between(plain, *a).total_cmp(&angle_between(plain, *b)))
                        .unwrap_or(plain)
                }
            };
            escape = Some(way);
            let step = 7.0 * DT;
            let to = [me[0] + way.cos() * step, me[1] + way.sin() * step, me[2]];
            s.move_player(hero, to, toward, Moving { forward: true, ..Moving::default() });
            acted = true;
        } else if let Some((land, seen)) = threat {
            escape = None;
            if now >= seen && land - now <= 0.12 && !guarding && d < 5.0 {
                s.move_player(hero, me, toward, Moving::default());
                s.input(hero, Input::Guard);
                acted = true;
            }
            if now > land + 0.1 {
                threat = None;
                answer_until = now + 0.6;
                if guarding { s.input(hero, Input::GuardUp); }
            }
        } else if guarding {
            escape = None;
            s.input(hero, Input::GuardUp);
        } else {
            escape = None;
        }
        last = me;
        let open = now < open_until - weapon.windup || now < scoop_until || (now < answer_until && threat.is_none());
        if !acted && ready {
            if open && d <= weapon.range + 0.4 {
                s.move_player(hero, me, toward, Moving::default());
                s.input(hero, Input::Attack);
            } else if d < 1.6 && !open {
                // Not inside his body: a step back out of it.
                let away = facing(bpos, me);
                let step = 4.0 * DT;
                let to = [me[0] + away.cos() * step, me[1] + away.sin() * step, me[2]];
                s.move_player(hero, to, toward, Moving { back: true, ..Moving::default() });
            } else if d > 2.2 {
                let step = 7.0 * DT;
                let to = [me[0] + toward.cos() * step, me[1] + toward.sin() * step, me[2]];
                if !drawn.iter().any(|h| now >= h.seen && h.holds(to, 0.6)) {
                    s.move_player(hero, to, toward, Moving { forward: true, ..Moving::default() });
                }
            } else {
                s.move_player(hero, me, toward, Moving::default());
            }
        }
        for e in s.step(DT) {
            let seen = s.clock + reaction;
            match e {
                Event::GroundBurst { at, yaw, shape, reach, arc, secs, colour, .. } if colour == HAZARD_COLOUR => {
                    drawn.push(Drawn { at, yaw, ring: shape == "ring", reach, arc, seen, until: s.clock + secs });
                }
                Event::HoldPose { id: who, clip: OPEN_CLIP, secs, .. } if who == boss => {
                    open_until = if secs > 0.0 { seen + secs - reaction } else { -1.0 };
                }
                Event::Clip { id: who, clip: DIP_CLIP, .. } if who == boss && !s.is_gorged(boss) => { scoop_until = seen + DIP + 0.1; }
                Event::MobWindup { id: who, windup, blockable, .. } if who == boss => {
                    answer_until = -1.0;
                    if blockable { threat = Some((s.clock + windup, seen)); }
                }
                Event::Missile { caster, spell: MUG, .. } if caster == boss => out.pours += 1,
                Event::Yell { id: who, ref text } if who == boss && text == "Mmm... one taste." => out.scoops += 1,
                Event::Yell { id: who, ref text } if who == boss && text == "MRGLGLGL!" => out.broken += 1,
                Event::Visual { id: who, kit: BELCH_KIT, .. } if who == boss => out.belches += 1,
                Event::Swing { attacker, victim: Some(v), damage, .. } if attacker == boss && v == hero && damage > 0 => {
                    out.hits_taken += 1;
                    out.damage_taken += damage;
                }
                _ => {}
            }
        }
    }
    if let Some(g) = gorged_from { out.gorged_secs = s.clock - g; } else { out.cookie_secs = s.clock - start; }
    out.health_left = s.players[&hero].health;
    out
}

#[test]
fn a_lone_hero_playing_by_the_tells_clears_cookie_and_gorged() {
    for seed in [3, 5, 11] {
        let out = dinner_by_tells(seed, true);
        eprintln!("seed {seed}: {out:?}");
        assert!(out.cleared, "{out:?}");
        assert!(out.health_left > 0);
        assert!(out.broken >= 1, "the scoop is broken by honest hits: {out:?}");
        assert!(out.pours >= 1 && out.belches >= 1, "{out:?}");
        assert!(out.gorged_secs > 0.0);
    }
}


/// The cove east of Cookie's post: the ground climbs 8 yd in 12 (the real map's slope there).
struct Slope;
impl Land for Slope {
    fn height(&self, x: f32, _: f32) -> f32 { (x - 3.0).max(0.0) * 0.8 }
}

#[test]
fn gorged_stays_on_a_hero_who_dodged_up_the_slope_until_he_dies() {
    for gorged in [false, true] {
        let (mut s, band, id) = fight(if gorged { GORGED } else { COOKIE }, 1);
        s.land = Box::new(Slope);
        let h = band[0];
        let max = s.mobs[&id].max_health;
        let mut hits = 0;
        // The hero rolls 12 yd up the slope (over 7 yd above his post), then fights him there.
        let up = [12.0, 0.0, Slope.height(12.0, 0.0)];
        s.move_player(h, up, std::f32::consts::PI, Moving::default());
        while s.mobs[&id].health > 0 && s.clock < 120.0 {
            let m = &s.mobs[&id];
            assert!(!matches!(m.state, MobState::Return | MobState::Idle { .. }), "{}: home mid-fight at {:.1}: {:?}", if gorged { "Gorged" } else { "Cookie" }, s.clock, m.state);
            assert!(s.kitchen.cooks.contains_key(&id));
            // Every 2 s a blow from the hero, wherever the fight has got to.
            if (s.clock / 2.0).floor() as u32 > hits {
                hits += 1;
                s.hurt_mob_as(id, h, max / 20 + 1, "Melee");
                let p = s.players[&h].pos;
                s.move_player(h, [p[0], p[1], Slope.height(p[0], p[1])], facing(p, s.mobs[&id].pos), Moving::default());
            }
            s.step(DT);
        }
        assert_eq!(s.mobs[&id].health, 0, "engaged until he died");
    }
}

/// The cove's floor with no way behind the pot (y over 3): nothing walks there, and no search
/// finds a way, as on a dock or beyond the cauldron's hull.
struct Fenced;
impl Land for Fenced {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn plans(&self) -> bool { true }
    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool { from[1] <= 3.0 && to[1] <= 3.0 }
}

#[test]
fn gorged_always_turns_on_the_hero_after_the_gulp_wherever_they_stand() {
    // Beside the pot either side, up close, far off, and behind it where he can't walk.
    for (spot, reachable) in [([2.6, 1.8], true), ([-2.0, 1.8], true), ([0.6, -0.6], true), ([9.0, -6.0], true), ([0.0, 7.5], false)] {
        let (mut s, band, id) = fight(COOKIE, 1);
        s.land = Box::new(Fenced);
        let h = band[0];
        let item = s.content.item("secret_ingredient").unwrap();
        s.give(h, item, 1);
        s.hurt_mob(id, h, u32::MAX);
        let next = *s.mobs.iter().find(|(_, m)| s.content.mobs[m.def].key == GORGED).unwrap().0;
        let at = [spot[0], spot[1], 0.0];
        s.move_player(h, at, 0.0, Moving::default());
        until(&mut s, next, 15.0, |s, n| s.kitchen.cooks.get(&n).is_some_and(|c| c.stage.is_none()) && !s.mobs[&n].hidden);
        let done = s.clock;
        if reachable {
            until(&mut s, next, 10.0, |s, n| matches!(s.mobs[&n].state, MobState::Windup { .. }));
            assert!(s.clock - done < 8.0, "at {spot:?} he came for the hero {:.1} s after his wait", s.clock - done);
        } else {
            // He can't reach them: he waits for them, still in the fight, not home.
            for _ in 0..(12.0 / DT) as usize {
                s.step(DT);
                assert!(!matches!(s.mobs[&next].state, MobState::Return | MobState::Idle { .. }), "behind the pot at {:.1}: {:?}", s.clock, s.mobs[&next].state);
            }
            s.move_player(h, [3.0, 0.0, 0.0], 0.0, Moving::default());
            until(&mut s, next, 8.0, |s, n| matches!(s.mobs[&n].state, MobState::Windup { .. }));
        }
        assert!(s.kitchen.cooks.contains_key(&next));
    }
}
