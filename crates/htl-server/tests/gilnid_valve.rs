//! Gilnid's valve on the real Foundry floor: the fire's channel, his way to the valve, the race
//! for it and where a hero stands dry. No game assets stored here.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, facing, Content, Event, Id, Input, Survival};

const DT: f32 = 0.05;
/// The gap by the Foundry's door, where a hero pulls him from.
const WAY_IN: [f32; 2] = [-188.0, -580.0];
/// The steam before the fire (`VENT_SECS`), and how long his hands are on the valve (`WORK_SECS`).
const STEAM: f32 = 1.2;
const WORK: f32 = 2.0;
type Vec3 = [f32; 3];

fn along(p: Vec3, yaw: f32, d: f32) -> Vec3 {
    [p[0] + yaw.cos() * d, p[1] + yaw.sin() * d, p[2]]
}

/// The Foundry with only Gilnid's pack in it, a hero just outside his sight.
fn mine() -> (Survival, Id, Id) {
    let data = std::env::var("WOW_DATA").expect("set WOW_DATA to installed Data directory");
    let land = DeadminesLand::new(htl_map::deadmines::load(std::path::Path::new(&data)).unwrap());
    let mut c = Content::builtin();
    c.dungeon.as_mut().unwrap().packs.retain(|p| p.key == "gilnid");
    c.dungeon.as_mut().unwrap().doors.clear();
    let mut s = Survival::new_dungeon(c, Box::new(land), 17, 0.0);
    let hero = s.join("valveproof", "Valveproof", None);
    let p = s.players.get_mut(&hero).unwrap();
    p.loadout.clear();
    p.max_health = 1_000_000;
    p.health = 1_000_000;
    let way_in = s.land().footing([WAY_IN[0], WAY_IN[1], 21.0]).unwrap();
    s.players.get_mut(&hero).unwrap().last_safe = way_in;
    let outside = s.land().footing([-178.0, -579.0, 19.5]).unwrap();
    s.move_player(hero, outside, std::f32::consts::PI, Default::default());
    for _ in 0..20 { step(&mut s); }
    let boss = s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == "dm_gilnid").map(|(id, _)| *id).expect("Gilnid at his post");
    (s, hero, boss)
}

fn step(s: &mut Survival) -> Vec<Event> {
    let ev = s.step(DT);
    for e in &ev {
        if let Event::Log(text) = e {
            assert!(!text.contains("put back") && !text.contains("fell through") && !text.contains("omitted")
                && !text.contains("no channel") && !text.contains("kept from the valve"), "{text}");
        }
    }
    ev
}

fn stage(s: &Survival, boss: Id) -> &'static str {
    s.perch_stage(boss).unwrap_or("gone")
}

fn put(s: &mut Survival, hero: Id, at: Vec3, toward: Vec3) {
    s.move_player(hero, at, facing(at, toward), Default::default());
}

/// Pulled from the way in, then the hero stands at `at` on his floor until he has come to them
/// (or `secs` are up), and his crew is brought down at once (debug): the race begins.
fn race_from(at: [f32; 2], secs: f32) -> (Survival, Id, Id) {
    let (mut s, hero, boss) = mine();
    let post = s.mobs[&boss].home;
    let way_in = s.land().footing([WAY_IN[0], WAY_IN[1], 21.0]).unwrap();
    put(&mut s, hero, way_in, post);
    for _ in 0..10 { step(&mut s); }
    assert!(s.mobs[&boss].state.foe().is_some(), "pulled from the way in");
    let at = s.land().footing([at[0], at[1], post[2]]).expect("a spot on his floor");
    put(&mut s, hero, at, post);
    let t = s.clock;
    while s.clock < t + secs && dist2d(s.mobs[&boss].pos, at) > 3.5 { step(&mut s); }
    for _ in 0..2 { s.dev_hurt(hero, 0.0, "engineer").ok(); }
    let t = s.clock;
    // (At his line, which waits until the pull's line is 2 sec past.)
    while stage(&s, boss) == "crew" && s.clock < t + 3.0 { step(&mut s); }
    assert_eq!(stage(&s, boss), "to valve", "his crew down, he starts for the valve");
    // Whole again, so that what the hero loses from here on is the valve's alone (and nothing
    // of what he dealt meanwhile mends during the measure).
    let p = s.players.get_mut(&hero).unwrap();
    p.health = p.max_health;
    (s, hero, boss)
}

/// The floor at `p`, if it is his pit floor and a hero can stand there.
fn floor(s: &Survival, p: Vec3, z: f32) -> Option<Vec3> {
    let h = s.land().height_near(p[0], p[1], z + 0.5);
    ((h - z).abs() <= 1.0 && !s.land().blocked_near(p[0], p[1], h) && !s.land().water_near(p[0], p[1], h)).then_some([p[0], p[1], h])
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_fires_channel_lies_whole_on_the_pit_floor_with_dry_floor_a_step_to_either_side() {
    let (s, _, boss) = mine();
    let post = s.mobs[&boss].home;
    let (channel, yaw, half) = s.perch_channel(boss).expect("his channel");
    let (ledges, valve) = s.perch_ledges(boss);
    let (wheel, stand) = valve.expect("somewhere to stand at the valve");
    assert!(dist2d(wheel, [-208.1, -583.1, 0.0]) < 0.1, "the melting pot's own valve");
    assert!(dist2d(stand, wheel) < 1.5, "he stands at it: {:.2} yd", dist2d(stand, wheel));
    assert!(s.land().walk_clear(post, stand) || s.land().path(post, stand).is_some(), "he can walk to it");
    assert_eq!(channel.len(), 10, "all of it: 10 sections, 16 yd");
    println!("channel: from {:?} to {:?}, {:.0} degrees, {:.1} yd wide", channel[0], channel[9], yaw.to_degrees(), half * 2.0);
    let run = s.content.player.run_speed;
    let out = half + 0.8;
    let mut one_sided = Vec::new();
    for (k, p) in channel.iter().enumerate() {
        assert!((p[2] - post[2]).abs() < 1.0, "section {k} on his floor: {p:?}");
        assert!(floor(&s, *p, post[2]).is_some(), "section {k} is floor");
        if k > 0 { assert!(s.land().walk_clear(channel[k - 1], *p), "open from section {} to {k}", k - 1); }
        assert!(s.land().walk_clear(post, *p) || s.land().path(post, *p).is_some(), "a hero can walk to section {k}");
        // Dry floor: a full stride past the fire's edge, walked to in a straight line.
        let dry: Vec<bool> = [1.0f32, -1.0].iter().map(|side| {
            let q = along(*p, yaw + side * std::f32::consts::FRAC_PI_2, out);
            floor(&s, q, post[2]).is_some_and(|q| s.land().walk_clear(*p, q))
        }).collect();
        assert!(dry[0] || dry[1], "section {k}: dry floor on one side at least");
        if !(dry[0] && dry[1]) { one_sided.push(k); }
    }
    // From its middle at a run: well inside the steam's 1.2 s.
    assert!(out / run < STEAM - 0.5, "out in {:.2} s", out / run);
    println!("dry floor {out:.1} yd from the middle, {:.2} s at a run; one side only at sections {one_sided:?}", out / run);
    // Only the first section, hard against the pot, has a wall on one side.
    assert!(one_sided.iter().all(|k| *k == 0), "dry floor both sides from section 1 on: {one_sided:?}");
    // Nothing of it under a plank's pole or a landing: the crew's floor stays out of the fire.
    for l in &ledges {
        for p in &channel {
            assert!(dist2d(*p, l.base) > half, "pole {:?} in the fire", l.base);
            assert!(dist2d(*p, l.landing) > half, "landing {:?} in the fire", l.landing);
        }
    }
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn he_reaches_the_valve_from_all_over_his_pit_works_it_only_there_and_the_fire_keeps_to_the_channel() {
    // Where the fight has him when his crew falls: by the door, at each landing, each corner.
    for at in [[-198.5f32, -585.0], [-205.3, -592.0], [-210.5, -592.0], [-199.5, -595.0], [-210.5, -596.0], [-203.0, -588.0]] {
        let (mut s, hero, boss) = race_from(at, 12.0);
        let (channel, _, half) = s.perch_channel(boss).unwrap();
        let (_, valve) = s.perch_ledges(boss);
        let (wheel, stand) = valve.unwrap();
        let (t0, from) = (s.clock, s.mobs[&boss].pos);
        // The hero waits well away from the channel.
        let aside = s.land().footing([-199.0, -584.5, from[2]]).unwrap();
        put(&mut s, hero, aside, from);
        let health = s.players[&hero].health;
        let mut marked = 0;
        while stage(&s, boss) == "to valve" && s.clock < t0 + 10.0 {
            marked += step(&mut s).iter().filter(|e| matches!(e, Event::GroundBurst { shape: "line", .. })).count();
        }
        let ran = s.clock - t0;
        assert_eq!(stage(&s, boss), "working", "from {from:?}");
        assert!(marked > 0, "the channel is marked while he runs");
        let him = s.mobs[&boss].pos;
        assert!(dist2d(him, stand) <= 1.0 && dist2d(him, wheel) < 2.5, "his hands on the valve itself: {him:?}");
        println!("from {:.1},{:.1} ({:.1} yd off): at the valve in {ran:.2} s", from[0], from[1], dist2d(from, stand));
        assert!(ran < 4.0, "{ran}");
        let mut fire: Vec<Vec3> = Vec::new();
        let mut whistle = false;
        while stage(&s, boss) != "finished" && s.clock < t0 + 20.0 {
            let before = stage(&s, boss);
            let ev = step(&mut s);
            whistle |= ev.iter().any(|e| matches!(e, Event::Sound { sound: 3431, .. }));
            if before == "working" && stage(&s, boss) == "steam" {
                fire = s.perch_fire_to_come(boss);
            }
            assert!(dist2d(s.mobs[&boss].pos, stand) <= 1.0, "he stays at it");
        }
        assert!(whistle && !fire.is_empty());
        assert!(fire.iter().all(|p| channel.contains(p)), "fire only on the marked channel");
        assert!(channel.iter().all(|p| fire.contains(p)), "and on all of it");
        assert!(dist2d(aside, channel[0]) > half + 3.0);
        assert_eq!(s.players[&hero].health, health, "(the hero, aside, is dry)");
    }
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn the_race_for_the_valve_is_won_by_a_hero_who_goes_at_once_and_lost_by_one_who_waits() {
    // The hero fights him by the second landing, where the crew comes down; they notice his line
    // `late` s after it, run at him and swing one heavy blow once his hands are on the valve.
    let mut table = Vec::new();
    for late in [0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5] {
        let (mut s, hero, boss) = race_from([-204.0, -592.0], 12.0);
        let run = s.content.player.run_speed;
        let (_, valve) = s.perch_ledges(boss);
        let stand = valve.unwrap().1;
        let (t0, his, theirs) = (s.clock, dist2d(s.mobs[&boss].pos, stand), dist2d(s.players[&hero].pos, stand));
        let (mut at_valve, mut swung, mut reached) = (None, None, None);
        while !matches!(stage(&s, boss), "open" | "steam") && s.clock < t0 + 12.0 {
            if s.clock - t0 >= late {
                let (me, him) = (s.players[&hero].pos, s.mobs[&boss].pos);
                let d = dist2d(me, him);
                if d > 2.4 {
                    let to = along(me, facing(me, him), (run * DT).min(d - 2.2));
                    let to = s.land().footing(to).unwrap_or(to);
                    put(&mut s, hero, to, him);
                } else {
                    put(&mut s, hero, me, him);
                    reached.get_or_insert(s.clock - t0);
                    if stage(&s, boss) == "working" && swung.is_none() { s.input(hero, Input::Heavy); swung = Some(s.clock - t0); }
                }
            }
            step(&mut s);
            if stage(&s, boss) == "working" { at_valve.get_or_insert(s.clock - t0); }
        }
        let won = stage(&s, boss) == "open";
        println!("noticed {late:.1} s late (he {his:.1} yd from the valve, the hero {theirs:.1}): his hands on it at {:.2} s, the hero beside him at {:.2} s, swung at {:.2} s, {} at {:.2} s",
            at_valve.unwrap_or(-1.0), reached.unwrap_or(-1.0), swung.unwrap_or(-1.0), if won { "STOPPED" } else { "the whistle" }, s.clock - t0);
        table.push((late, won, at_valve.unwrap()));
    }
    let won = |late: f32| table.iter().find(|r| r.0 == late).unwrap().1;
    assert!(won(0.0) && won(1.0), "a hero who goes at once wins it");
    assert!(!won(3.5), "a hero who waits loses it");
    // His hands are on the valve `WORK` s: the whistle follows that long after.
    assert!(table.iter().all(|r| r.2 + WORK < 6.0));
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_hero_in_the_channel_at_the_whistle_walks_out_dry_either_side_and_one_who_stays_pays() {
    // (The last two stand still: on a section's middle, and between two sections, where both bite.)
    for (k, side, seam) in [(1usize, 1.0f32, false), (1, -1.0, false), (5, 1.0, false), (5, -1.0, false), (9, 1.0, false), (9, -1.0, false), (5, 0.0, false), (5, 0.0, true)] {
        let (mut s, hero, boss) = race_from([-199.0, -585.0], 6.0);
        let run = s.content.player.run_speed;
        let (channel, yaw, half) = s.perch_channel(boss).unwrap();
        let at = if seam { along(channel[k], yaw, dist2d(channel[k], channel[k + 1]) / 2.0) } else { channel[k] };
        put(&mut s, hero, at, channel[0]);
        let t0 = s.clock;
        while stage(&s, boss) != "steam" && s.clock < t0 + 10.0 { step(&mut s); }
        assert_eq!(stage(&s, boss), "steam");
        let (whistle, before) = (s.clock, s.players[&hero].health);
        let out = yaw + side * std::f32::consts::FRAC_PI_2;
        let mut gone = 0.0;
        while stage(&s, boss) != "finished" && s.clock < whistle + 12.0 {
            // Half a second to take the whistle in, then out the side at a run.
            if side != 0.0 && s.clock - whistle >= 0.5 && gone < half + 0.8 {
                gone = (gone + run * DT).min(half + 0.8);
                let to = along(channel[k], out, gone);
                let to = s.land().footing(to).expect("floor to step onto");
                assert!(s.land().walk_clear(s.players[&hero].pos, to), "an open step");
                put(&mut s, hero, to, channel[k]);
            }
            step(&mut s);
        }
        let cost = before - s.players[&hero].health;
        if side == 0.0 {
            let health = s.content.player.health;
            println!("stood {} to the end: {cost} health of {health}", if seam { "between sections 5 and 6" } else { "on section 5's middle" });
            // The fire, then its last bite (from 2 sections, between them): it costs, and from
            // full health it's survived.
            assert!(cost >= 45 && cost < health * 2 / 3, "{cost} of {health}");
        } else {
            assert_eq!(cost, 0, "section {k}, side {side}: out dry");
        }
    }
}

#[test]
#[ignore = "requires WOW_DATA pointing to installed WoW 1.12.1 Data"]
fn a_hero_up_on_a_plank_or_the_ramp_is_out_of_the_fire_and_the_valve_still_runs_its_course() {
    let (s, _, boss) = mine();
    let post = s.mobs[&boss].home;
    let (ledges, _) = s.perch_ledges(boss);
    let (channel, ..) = s.perch_channel(boss).unwrap();
    // The planks' ends, the ramp behind each, and any floor right over the channel.
    let mut spots: Vec<(String, Vec3)> = Vec::new();
    for (n, l) in ledges.iter().enumerate() {
        spots.push((format!("plank {n}"), l.stand));
        spots.push((format!("ramp behind plank {n}"), l.climb));
    }
    for (k, p) in channel.iter().enumerate() {
        let up = s.land().height_near(p[0], p[1], post[2] + 14.0);
        if up > post[2] + 4.0 { spots.push((format!("the ramp over section {k}"), [p[0], p[1], up])); }
    }
    drop(s);
    println!("{} spots up off his floor", spots.len());
    for (name, at) in spots {
        let (mut s, hero, boss) = mine();
        let way_in = s.land().footing([WAY_IN[0], WAY_IN[1], 21.0]).unwrap();
        put(&mut s, hero, way_in, post);
        for _ in 0..10 { step(&mut s); }
        assert!(s.mobs[&boss].state.foe().is_some());
        put(&mut s, hero, at, post);
        for _ in 0..40 { step(&mut s); }
        for _ in 0..2 { s.dev_hurt(hero, 0.0, "engineer").ok(); }
        let t0 = s.clock;
        let mut seen = Vec::new();
        let mut burned = 0;
        while stage(&s, boss) != "finished" && stage(&s, boss) != "gone" && s.clock < t0 + 25.0 {
            let (before, was) = (s.players[&hero].health, stage(&s, boss));
            step(&mut s);
            if matches!(was, "steam" | "fire") { burned += before - s.players[&hero].health; }
            if seen.last() != Some(&stage(&s, boss)) { seen.push(stage(&s, boss)); }
        }
        println!("{name} {at:?}: {seen:?}, {:.1} s", s.clock - t0);
        assert_eq!(seen, ["to valve", "working", "steam", "fire", "finished"], "{name}: the valve runs its course");
        assert_eq!(burned, 0, "{name}: out of the fire");
        // The fight goes on: he is alive, still theirs, and on to his finish.
        assert!(s.mobs[&boss].health > 0 && s.mobs[&boss].state.foe().is_some(), "{name}");
    }
}
