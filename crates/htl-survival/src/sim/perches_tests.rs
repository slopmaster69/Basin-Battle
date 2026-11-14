use super::*;
use crate::land::{FlatLand, Land};

fn fight() -> (Survival, Id, Id, Vec<Id>) {
    let mut s = Survival::bare(Content::dev(), Box::new(FlatLand::default()), 7, 0.0);
    let hero = s.join("smith", "Smith", None);
    let p = s.players.get_mut(&hero).unwrap();
    p.pos = [0.0, 0.0, 0.0];
    p.health = 100_000;
    p.max_health = 100_000;
    let boss = s.spawn_mob(s.content.mob("dm_gilnid").unwrap(), [0.0, 0.0, 0.0], None, false);
    s.step_perch_crews();
    let crew = s.perches.crews[&boss].workers.clone();
    assert_eq!(crew.len(), 2);
    s.engage(boss, Foe::Hero(hero));
    s.step_perch_crews();
    (s, hero, boss, crew)
}

fn tick(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / 0.05).round() as usize {
        s.clock += 0.05;
        s.step_missiles();
        s.step_mobs(0.05);
    }
}

/// Ticks until `done`, at most `secs`; whether it came.
fn until(s: &mut Survival, secs: f32, done: impl Fn(&Survival) -> bool) -> bool {
    for _ in 0..(secs / 0.05).round() as usize {
        if done(s) { return true; }
        tick(s, 0.05);
    }
    done(s)
}

fn shot(s: &mut Survival, hero: Id, worker: Id, damage: f32, how: HitKind) {
    let ability = s.content.ability("fireball").unwrap();
    s.content.abilities[ability].effects.clear();
    s.ability_on_mob(hero, worker, ability, 1.0, damage, s.players[&hero].pos, how, 133);
}

fn perched(s: &Survival, id: Id) -> bool {
    matches!(s.perches.workers[&id].state, Place::Perched)
}

fn display(s: &Survival, id: Id) -> Option<u32> {
    match s.objs.get(&id).map(|o| &o.kind) { Some(ObjKind::Prop { display, .. }) => Some(*display), _ => None }
}

#[test]
fn each_engineer_stands_on_a_plank_over_the_drop_with_a_pole_from_the_floor_below() {
    let (s, _, boss, crew) = fight();
    let defs = s.content.mobs[s.mobs[&boss].def].perch_crew.clone().unwrap().ledges;
    for (id, def) in crew.iter().zip(&defs) {
        let w = &s.perches.workers[id];
        let m = &s.mobs[id];
        assert_eq!(m.pos, w.stand);
        assert!((m.pos[2] - def.at[2]).abs() < 0.01, "{} yd over the floor", def.at[2]);
        assert!((7.0..=10.0).contains(&def.at[2]), "Corey's 7 to 10 yd");
        assert_eq!(display(&s, w.plank), Some(PLANK));
        assert_eq!(display(&s, w.support), Some(POLE));
        // The plank juts the ledge's way, and he stands out over its end, past the rail.
        assert!((s.objs[&w.plank].facing - def.out).abs() < 0.001);
        let rail = along(w.stand, def.out, -STAND_OUT);
        assert!(dist2d(w.base, rail) > dist2d(w.stand, rail), "the pole holds the plank's far part");
        // The pole stands on the floor and reaches the plank's underside.
        let ObjKind::Prop { scale, .. } = s.objs[&w.support].kind else { panic!() };
        let top = w.base[2] + POLE_HEIGHT * scale;
        assert!((top - (w.stand[2] - PLANK_THICK)).abs() < 0.05, "pole top {top}");
        assert_eq!(w.base[2], 0.0);
        // He falls outward, away from the ramp.
        assert!(dist2d(w.landing, rail) > dist2d(w.stand, rail) + 2.0);
    }
    // Two planks splayed apart, not stacked crates beside him.
    assert!(dist2d(s.perches.workers[&crew[0]].stand, s.perches.workers[&crew[1]].stand) > 3.5);
    assert!(!s.objs.values().any(|o| matches!(o.kind, ObjKind::Prop { display: 36, .. })), "no crate stacks");
}

#[test]
fn the_first_stick_has_a_three_second_fuse_an_arc_and_sits_sparking_a_second_before_it_blows() {
    let (mut s, _, boss, crew) = fight();
    s.events.clear();
    assert!(until(&mut s, 3.0, |s| matches!(s.mobs[&crew[0]].state, MobState::Windup { .. })), "the first man lights it");
    assert!(!matches!(s.mobs[&crew[1]].state, MobState::Windup { .. }), "the other waits his turn");
    assert!(s.events.iter().any(|e| matches!(e, Event::Cast { caster, spell: 4062, ms: 3000 } if *caster == crew[0])), "a 3 sec fuse at his hand");
    let lit = s.clock;
    assert!(until(&mut s, 3.5, |s| !s.perches.crews[&s.perches.workers[&crew[0]].owner].sticks.is_empty()));
    assert!((s.clock - lit - 3.0).abs() < 0.11, "thrown as the fuse ends");
    let stick = s.perches.crews[&boss].sticks[0].at;
    let hand = s.mobs[&crew[0]].pos;
    // Up to the top of an arc first, then down from it to where it was marked.
    let top = s.events.iter().find_map(|e| match e {
        Event::Missile { caster, spell: 4062, target: Target::Point(p) } if *caster == crew[0] => Some(*p), _ => None,
    }).expect("thrown");
    assert!(top[2] > hand[2] + 1.0 && dist2d(top, hand) > 1.0 && dist2d(top, stick) > 1.0);
    assert!(s.perch_stick_live(boss));
    let overhead = s.content.mobs[s.mobs[&boss].def].patterns.iter().find(|p| p.name == "overhead").unwrap().clone();
    let knocks = s.content.mobs[s.mobs[&boss].def].patterns.iter().find(|p| p.name == "two knocks").unwrap().clone();
    assert!(!s.perch_pattern_allowed(boss, &overhead), "no heavy blow while it's live");
    assert!(s.perch_pattern_allowed(boss, &knocks));
    let thrown = s.clock;
    assert!(until(&mut s, 2.0, |s| s.events.iter().any(|e| matches!(e, Event::MissileFrom { from, target: Target::Point(p), .. } if *from == top && *p == stick))));
    let c = &s.perches.crews[&boss];
    let (lands, blows) = (c.sticks[0].lands, c.sticks[0].blows);
    assert!(lands - thrown >= 0.7, "seen in the air");
    assert!((blows - lands - STICK_SITS).abs() < 0.01);
    assert!(until(&mut s, 2.0, |s| s.perches.crews[&boss].visuals.iter().any(|id| display(s, *id) == Some(STICK))));
    let prop = *s.perches.crews[&boss].visuals.iter().find(|id| display(&s, **id) == Some(STICK)).unwrap();
    assert_eq!(s.objs[&prop].pos, stick);
    assert!(s.events.iter().any(|e| matches!(e, Event::GroundBurst { shape: "ring", reach, at, .. } if *at == stick && *reach >= 2.5)), "drawn to its size with markers off");
    assert!(s.events.iter().any(|e| matches!(e, Event::Sound { id, sound: 1424 } if *id == prop)), "hissing");
    assert!(s.missiles.iter().any(|m| m.owner == crew[0] && m.land_at == blows), "it hasn't blown yet");
    assert!(until(&mut s, 1.2, |s| !s.objs.contains_key(&prop)));
    assert!(s.clock >= blows - 0.01);
    assert!(!s.missiles.iter().any(|m| m.owner == crew[0]), "blown");
    assert!(!s.perch_stick_live(boss));
    assert!(s.perch_pattern_allowed(boss, &overhead));
}

#[test]
fn after_the_first_stick_the_furnace_sets_the_first_pole_alight_and_he_falls_lands_sitting_and_fights() {
    let (mut s, _, boss, crew) = fight();
    let w = &s.perches.workers[&crew[0]];
    let (base, landing, stand, plank, pole) = (w.base, w.landing, w.stand, w.plank, w.support);
    let furnace = s.perches.crews[&boss].furnace;
    assert!(until(&mut s, 10.0, |s| s.perches.crews[&boss].demo.is_some()));
    assert!(s.events.iter().any(|e| matches!(e, Event::Missile { caster, spell: MOLTEN, target: Target::Point(p) } if *caster == boss && *p == furnace)), "the furnace belches");
    // The spill runs toward his pole, and only there.
    let spill: Vec<Vec3> = s.missiles.iter().filter(|m| m.owner == boss).map(|m| m.at).collect();
    assert!(spill.len() >= 3);
    let near = spill.iter().map(|p| dist2d(*p, base)).fold(f32::MAX, f32::min);
    assert!(near < 2.0, "it reaches the pole: {near}");
    assert!(s.events.iter().any(|e| matches!(e, Event::GroundBurst { shape: "line", .. })));
    assert!(until(&mut s, 4.0, |s| s.events.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Mind the equipment!"))));
    assert!(s.perches.crews[&boss].visuals.iter().any(|id| display(&s, *id) == Some(FIRE) && s.objs[id].pos == base), "his pole alight");
    assert!(perched(&s, crew[0]), "the line comes before the drop");
    assert!(until(&mut s, 2.0, |s| !perched(s, crew[0])));
    assert!(!s.objs.contains_key(&plank) && !s.objs.contains_key(&pole), "the plank drops");
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: FALL_CLIP, .. } if *id == crew[0])));
    tick(&mut s, FALL_SECS / 2.0);
    let mid = s.mobs[&crew[0]].pos;
    assert!(mid[2] < stand[2] && mid[2] > landing[2] && dist2d(mid, stand) > 0.5, "falling outward, in plain view");
    assert!(until(&mut s, 1.0, |s| matches!(s.perches.workers[&crew[0]].state, Place::Ground)));
    assert_eq!(s.mobs[&crew[0]].pos, landing);
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: SIT_CLIP, secs, .. } if *id == crew[0] && *secs == SIT_SECS)), "lands sitting");
    assert!(!s.events.iter().any(|e| matches!(e, Event::Emote { id, emote: STUN_EMOTE } if *id == crew[0])), "no stagger's reel over the sit");
    assert_eq!(s.content.mobs[s.mobs[&crew[0]].def].weapon, 1610, "his wrench out");
    assert!(until(&mut s, SIT_SECS + 0.1, |s| s.events.iter().any(|e| matches!(e, Event::Clip { id, clip: SIT_UP_CLIP, .. } if *id == crew[0]))), "gets up");
    assert!(s.mobs[&crew[0]].stagger_until > s.clock + GET_UP_SECS - 0.1, "and only then fights");
    assert!(until(&mut s, GET_UP_SECS + 2.0, |s| matches!(s.mobs[&crew[0]].state, MobState::Chase { .. } | MobState::Windup { .. })), "he fights");
}

#[test]
fn one_projectile_drops_once_then_lands_sitting_as_a_melee_worker() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[1];
    let from = s.mobs[&id].pos;
    let landing = s.perches.workers[&id].landing;
    shot(&mut s, hero, id, 1.0, HitKind::Projectile);
    let since = match s.perches.workers[&id].state { Place::Falling { since, .. } => since, _ => panic!("one hit drops it") };
    tick(&mut s, 0.4);
    assert!(s.mobs[&id].pos[2] < from[2] && s.mobs[&id].pos[2] > landing[2]);
    shot(&mut s, hero, id, 1.0, HitKind::Projectile);
    assert!(matches!(s.perches.workers[&id].state, Place::Falling { since: t, .. } if t == since));
    tick(&mut s, 0.55);
    let m = &s.mobs[&id];
    assert_eq!(m.pos, landing);
    assert!(m.stagger_until > s.clock + 2.0);
    assert!(!s.content.mobs[m.def].attack.ranged);
    assert!(matches!(s.perches.workers[&id].state, Place::Ground));
    assert_eq!(s.content.mobs[m.def].weapon, 1610);
    assert!(s.events.iter().any(|e| matches!(e, Event::MobGear { id: who } if *who == id)));
}

#[test]
fn area_damage_and_zero_damage_do_not_count_as_a_direct_shot() {
    let (mut s, hero, _, crew) = fight();
    shot(&mut s, hero, crew[0], 1.0, HitKind::Area);
    shot(&mut s, hero, crew[0], 0.0, HitKind::Projectile);
    s.hurt_mob_as(crew[0], hero, 1, "burning");
    assert!(perched(&s, crew[0]));
}

#[test]
fn a_real_fireball_drops_its_contact_but_not_a_nearby_splash_victim() {
    let (mut s, hero, _, crew) = fight();
    let at = s.mobs[&crew[0]].pos;
    s.players.get_mut(&hero).unwrap().pos = [at[0] - 6.0, at[1], at[2]];
    s.players.get_mut(&hero).unwrap().facing = 0.0;
    s.mobs.get_mut(&crew[1]).unwrap().pos = [at[0], at[1] + 1.5, at[2]];
    let ability = s.content.ability("fireball").unwrap();
    s.launch(hero, ability, 0.0);
    for _ in 0..20 { s.clock += 0.05; s.step_projectiles(0.05); }
    assert!(s.mobs[&crew[1]].health < s.mobs[&crew[1]].max_health, "the neighbor took splash damage");
    assert!(matches!(s.perches.workers[&crew[0]].state, Place::Falling { .. }));
    assert!(perched(&s, crew[1]));
}

#[test]
fn a_blow_on_his_pole_cracks_it_from_the_same_floor_in_reach_and_facing() {
    let (mut s, hero, boss, crew) = fight();
    let id = crew[1];
    let base = s.perches.workers[&id].base;
    s.players.get_mut(&hero).unwrap().pos = [base[0] - 2.0, base[1], base[2]];
    s.swing_on_perches(hero, std::f32::consts::PI, 3.0, 90.0);
    assert!(perched(&s, id), "facing away");
    s.players.get_mut(&hero).unwrap().pos[2] += 20.0;
    s.swing_on_perches(hero, 0.0, 3.0, 90.0);
    assert!(perched(&s, id), "another floor");
    s.players.get_mut(&hero).unwrap().pos = [base[0] - 5.0, base[1], base[2]];
    s.swing_on_perches(hero, 0.0, 3.0, 90.0);
    assert!(perched(&s, id), "out of reach");
    s.players.get_mut(&hero).unwrap().pos = [base[0] - 2.0, base[1], base[2]];
    s.swing_on_perches(hero, 0.0, 3.0, 90.0);
    assert!(matches!(s.perches.workers[&id].state, Place::Falling { .. }));
    let lumber = s.perches.crews[&boss].visuals.iter().any(|v| display(&s, *v) == Some(LUMBER) && s.objs[v].pos == base);
    assert!(lumber, "the cracked pole lies where it stood");
}

#[test]
fn a_blow_on_the_man_himself_from_the_ramp_behind_him_knocks_him_off() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[1];
    let w = &s.perches.workers[&id];
    let (stand, out) = (w.stand, w.out);
    // On the ramp's walkway, just behind the rail.
    let behind = along(stand, out, -(STAND_OUT + 0.8));
    s.players.get_mut(&hero).unwrap().pos = [behind[0], behind[1], stand[2] - 1.6];
    let face = facing(s.players[&hero].pos, stand);
    s.swing_on_perches(hero, face, 3.0, 90.0);
    assert!(matches!(s.perches.workers[&id].state, Place::Falling { .. }));
}

#[test]
fn a_lethal_shot_leaves_a_falling_corpse_and_never_resurrects_it() {
    let (mut s, hero, _, crew) = fight();
    let id = crew[0];
    let landing = s.perches.workers[&id].landing;
    shot(&mut s, hero, id, 10_000.0, HitKind::Projectile);
    assert_eq!(s.mobs[&id].health, 0);
    tick(&mut s, 0.4);
    assert!(s.mobs[&id].pos[2] > landing[2]);
    tick(&mut s, 0.55);
    assert_eq!(s.mobs[&id].pos, landing);
    assert_eq!(s.mobs[&id].health, 0);
    assert!(!s.events.iter().any(|e| matches!(e, Event::HoldPose { id: who, clip: SIT_CLIP, .. } if *who == id)), "a dead man doesn't sit up");
    tick(&mut s, 4.0);
    assert!(!s.mobs.contains_key(&id));
}

#[test]
fn left_alone_the_second_man_throws_three_sticks_then_climbs_down_the_ramp() {
    let (mut s, _, boss, crew) = fight();
    let mut live_most = 0;
    let mut saw_demo = false;
    for _ in 0..1100 {
        tick(&mut s, 0.05);
        let c = &s.perches.crews[&boss];
        let winding = crew.iter().filter(|id| s.perch_controlled(**id)
            && s.mobs.get(id).is_some_and(|m| matches!(m.state, MobState::Windup { .. }))).count();
        live_most = live_most.max(c.sticks.len() + winding);
        saw_demo |= c.demo.is_some();
        if matches!(s.perches.workers[&crew[1]].state, Place::Ground) { break; }
    }
    assert_eq!(live_most, 1, "one stick at a time");
    assert!(saw_demo);
    assert_eq!(s.perches.workers[&crew[0]].throws, 1, "the first man throws once, then the furnace takes him");
    assert_eq!(s.perches.workers[&crew[1]].throws, THROWS_BEFORE_CLIMB);
    let w = &s.perches.workers[&crew[1]];
    assert!(matches!(w.state, Place::Ground), "he came down");
    assert_eq!(s.mobs[&crew[1]].pos, w.climb, "by the ramp, not a fall");
    assert_eq!(s.content.mobs[s.mobs[&crew[1]].def].weapon, 1610);
    assert!(s.objs.contains_key(&w.plank), "his plank is still there");
}

#[test]
fn a_hero_who_brought_one_down_first_gets_no_demonstration() {
    let (mut s, hero, boss, crew) = fight();
    shot(&mut s, hero, crew[0], 1.0, HitKind::Projectile);
    assert!(until(&mut s, 12.0, |s| s.perches.crews[&boss].first_blown));
    tick(&mut s, 3.0);
    assert!(s.perches.crews[&boss].demo.is_none());
    assert!(!s.events.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == "Mind the equipment!")));
    assert_eq!(s.perches.workers[&crew[1]].throws, 1, "the other took the first throw");
}

/// The crew down (debug kills) and his line said: the race for the valve has begun.
fn race() -> (Survival, Id, Id) {
    let (mut s, hero, boss, crew) = fight();
    // (The pull's line is long past: his lines keep `LINE_GAP` apart.)
    s.clock += 5.0;
    for id in crew { s.hurt_mob_as(id, hero, 10_000, "test"); }
    tick(&mut s, 0.1);
    assert_eq!(stage(&s, boss), "to valve");
    assert!(said(&s, "Useless. Every one of you."));
    (s, hero, boss)
}

fn stage(s: &Survival, boss: Id) -> &'static str {
    s.perch_stage(boss).unwrap_or("gone")
}

/// The channel's bursts among `ev` in `colour`: where each begins, its way, reach, width, secs.
fn marks(ev: &[Event], colour: &str) -> Vec<(Vec3, f32, f32, f32, f32)> {
    ev.iter().filter_map(|e| match e {
        Event::GroundBurst { at, yaw, shape: "line", reach, width, secs, colour: c, .. } if c == colour => Some((*at, *yaw, *reach, *width, *secs)),
        _ => None,
    }).collect()
}

/// How many bursts of the channel's mark, of any look, are among `ev`.
fn marked(ev: &[Event]) -> usize {
    [MARK_WILL, MARK_SOON, MARK_FIRE, MARK_OUT].iter().map(|c| marks(ev, c).len()).sum()
}

/// Steam and flames standing in the world.
fn vapour(s: &Survival) -> usize {
    s.objs.values().filter(|o| matches!(o.kind, ObjKind::Prop { display, .. } if display == STEAM || display == FIRE)).count()
}

fn friend(s: &mut Survival, key: &str, at: Vec3) -> Id {
    let id = s.join(key, key, None);
    let p = s.players.get_mut(&id).unwrap();
    p.pos = at;
    p.health = 100_000;
    p.max_health = 100_000;
    id
}

fn said(s: &Survival, line: &str) -> bool {
    s.events.iter().any(|e| matches!(e, Event::Yell { text, .. } if text == line))
}

/// A hero beside him, facing him.
fn beside(s: &mut Survival, hero: Id, boss: Id) {
    let at = s.mobs[&boss].pos;
    s.move_player(hero, [at[0] - 1.2, at[1], at[2]], 0.0, Moving::default());
}

#[test]
fn the_fires_channel_is_fixed_in_the_world_and_marked_from_his_line() {
    let (mut s, hero, boss, crew) = fight();
    let (channel, yaw, half) = s.perch_channel(boss).unwrap();
    let n = channel.len();
    let valve = s.perches.crews[&boss].valve;
    assert_eq!(n, STRIPE_MOST, "the whole of it on an open floor");
    assert!((dist2d(channel[0], valve) - CHANNEL_OUT).abs() < 0.01, "from the valve");
    let d = s.content.mobs[s.mobs[&boss].def].perch_crew.clone().unwrap().channel.unwrap();
    assert!(angle_between(yaw, d[1].atan2(d[0])) < 0.5, "the content's way");
    for w in channel.windows(2) {
        assert!((dist2d(w[0], w[1]) - STRIPE_STEP).abs() < 0.01);
        assert!(angle_between(facing(w[0], w[1]), yaw) < 0.5, "one straight line");
    }
    tick(&mut s, 1.0);
    assert_eq!(marked(&s.events), 0, "nothing is marked while the crew stands");
    assert!(s.perches.crews[&boss].wisps.is_empty());
    // Wherever the heroes stand when his line comes, it's the same channel.
    s.players.get_mut(&hero).unwrap().pos = [14.0, 9.0, 0.0];
    s.events.clear();
    for id in crew { s.hurt_mob_as(id, hero, 10_000, "test"); }
    tick(&mut s, 0.1);
    assert_eq!(stage(&s, boss), "to valve");
    assert_eq!(s.perch_channel(boss).unwrap().0, channel);
    // Marked from that moment: a pale band its true width the whole channel long, and a wisp of
    // steam on every section.
    let will = marks(&s.events, MARK_WILL);
    assert!(!will.is_empty(), "marked as his line is said");
    let whole = dist2d(channel[0], channel[n - 1]) + STRIPE_STEP;
    for (at, way, reach, width, secs) in &will {
        assert!(dist2d(*at, along(channel[0], yaw, -STRIPE_STEP / 2.0)) < 0.01);
        assert!(angle_between(*way, yaw) < 0.5);
        assert!((reach - whole).abs() < 0.01, "the whole channel");
        assert!((width - 2.0 * half).abs() < 0.01, "its true width");
        assert!(*secs <= 1.0, "never drawn longer than a moment ahead: it can go out");
    }
    let wisps = s.perches.crews[&boss].wisps.clone();
    assert_eq!(wisps.len(), n);
    for (id, p) in wisps.iter().zip(&channel) {
        assert_eq!(display(&s, *id), Some(STEAM));
        assert_eq!(s.objs[id].pos, *p);
    }
    assert!(!s.missiles.iter().any(|m| m.owner == boss), "nothing burns yet");
    // It stays marked all his way to the valve and while his hands are on it.
    s.events.clear();
    let t0 = s.clock;
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    tick(&mut s, 1.0);
    let sent = marks(&s.events, MARK_WILL).len() as f32;
    assert!(sent >= (s.clock - t0) / WILL_EVERY - 1.5, "sent again each {WILL_EVERY} s: {sent} in {}", s.clock - t0);
    assert_eq!(marked(&s.events) as f32, sent, "in the one look");
    assert_eq!(s.perches.crews[&boss].wisps, wisps);
}

#[test]
fn left_to_it_a_whistle_then_steam_then_fire_down_the_marked_channel_with_dry_ground_either_side() {
    let (mut s, hero, boss) = race();
    let (channel, yaw, half) = s.perch_channel(boss).unwrap();
    let n = channel.len();
    // One hero stands in the channel's middle and stays; another a step outside its edge.
    s.players.get_mut(&hero).unwrap().pos = channel[n / 2];
    let dry = friend(&mut s, "dry", along(channel[n / 2], yaw + std::f32::consts::FRAC_PI_2, half + 0.3));
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    let c = &s.perches.crews[&boss];
    let stand = c.stand.unwrap();
    assert!(dist2d(s.mobs[&boss].pos, stand) <= 1.0, "standing at it");
    assert!(dist2d(stand, c.valve) < 1.5, "at the furnace's own valve");
    let ms = (WORK_SECS * 1000.0) as u32;
    assert!(s.events.iter().any(|e| matches!(e, Event::Cast { caster, spell: WORK_VALVE, ms: m } if *caster == boss && *m == ms)));
    assert!(!s.missiles.iter().any(|m| m.owner == boss));
    let before = (s.players[&hero].health, s.players[&dry].health);
    assert!(until(&mut s, WORK_SECS + 0.2, |s| stage(s, boss) == "steam"));
    let opened = s.clock;
    assert!(s.events.iter().any(|e| matches!(e, Event::Sound { id, sound: 3431 } if *id == boss)), "a whistle");
    assert!(s.perches.crews[&boss].wisps.is_empty(), "the wisps give way to the full steam");
    let steam = s.perches.crews[&boss].visuals.iter().filter(|id| display(&s, **id) == Some(STEAM)).count();
    assert_eq!(steam, n, "steam on every section");
    // The fire is laid on the marked channel and nowhere else, a section after a section.
    let stripe: Vec<(Vec3, f32)> = s.missiles.iter().filter(|m| m.owner == boss && m.look == MOLTEN).map(|m| (m.at, m.land_at)).collect();
    assert_eq!(stripe.iter().map(|x| x.0).collect::<Vec<_>>(), channel);
    assert!(s.missiles.iter().filter(|m| m.owner == boss).all(|m| channel.contains(&m.at)));
    for (k, sec) in stripe.iter().enumerate() {
        assert!((sec.1 - (opened + VENT_SECS + k as f32 * STRIPE_GAP)).abs() < 0.06, "the steam first, then a section each {STRIPE_GAP} s");
    }
    // The mark changes its look: bright where it's about to burn, amber where it burns.
    s.events.clear();
    let mut log: Vec<(f32, String, Vec3, f32, f32)> = Vec::new();
    let mut from = 0;
    loop {
        for e in &s.events[from..] {
            if let Event::GroundBurst { at, reach, colour, secs, shape: "line", .. } = e { log.push((s.clock, colour.clone(), *at, *reach, *secs)); }
        }
        from = s.events.len();
        if stage(&s, boss) == "finished" || s.clock > opened + 12.0 { break; }
        tick(&mut s, 0.05);
    }
    assert_eq!(stage(&s, boss), "finished");
    let whole = dist2d(channel[0], channel[n - 1]) + STRIPE_STEP;
    let start = along(channel[0], yaw, -STRIPE_STEP / 2.0);
    assert!(log.iter().all(|l| l.1 == MARK_SOON || l.1 == MARK_FIRE), "only those two looks once the valve is open");
    let first = &log[0];
    assert!(first.0 <= opened + 0.11 && first.1 == MARK_SOON && (first.3 - whole).abs() < 0.01, "the whole channel bright at the whistle");
    let fire_at = opened + VENT_SECS;
    assert!(log.iter().filter(|l| l.0 < fire_at - 0.06).all(|l| l.1 == MARK_SOON), "no amber before the fire");
    assert!(log.iter().any(|l| l.1 == MARK_FIRE));
    for l in &log {
        // Each lies inside the channel, on its line.
        let along_it = dist2d(start, l.2);
        assert!(along_it < 0.01 || angle_between(facing(start, l.2), yaw) < 0.5, "{l:?}");
        assert!(along_it + l.3 <= whole + 0.01, "{l:?}");
        assert!(l.4 <= 1.0);
    }
    // Amber only where a section has caught, the rest still bright.
    for l in log.iter().filter(|l| l.1 == MARK_FIRE) {
        let lit = (((l.0 - fire_at) / STRIPE_GAP) as usize + 1).min(n) as f32;
        assert!(dist2d(start, l.2) + l.3 <= lit * STRIPE_STEP + 0.3, "{l:?}");
    }
    assert!(log.last().unwrap().0 <= fire_at + n as f32 * STRIPE_GAP + STRIPE_BURN + 0.1, "the mark doesn't outlast the fire");
    let cost = before.0 - s.players[&hero].health;
    assert!(cost > 0, "standing in it to the end costs");
    assert_eq!(s.players[&dry].health, before.1, "a step outside its edge is dry");
    assert!(said(&s, LINE_MELT) && !said(&s, LINE_STOPPED) && !said(&s, LINE_HIMSELF));
    s.events.clear();
    tick(&mut s, 1.5);
    assert_eq!(marked(&s.events), 0);
    assert_eq!(vapour(&s), 0, "steam and flames are out");
    assert_eq!(s.perches.workers.len(), 2, "no replacement workers");
    s.hurt_mob_as(boss, hero, 100_000, "test");
    assert!(s.perches.crews.is_empty() && s.perches.workers.is_empty());
}

#[test]
fn a_hero_in_the_channel_at_the_whistle_reaches_dry_floor_either_side_before_it_burns() {
    let speed = Content::dev().player.run_speed;
    for side in [1.0f32, -1.0] {
        for k in [0usize, 4, 9] {
            let (mut s, hero, boss) = race();
            let (channel, yaw, half) = s.perch_channel(boss).unwrap();
            s.players.get_mut(&hero).unwrap().pos = channel[k];
            assert!(until(&mut s, 8.0, |s| stage(s, boss) == "steam"));
            let before = s.players[&hero].health;
            // Half a second to take the whistle in, then out the side at a run.
            tick(&mut s, 0.5);
            let out = yaw + side * std::f32::consts::FRAC_PI_2;
            let mut gone = 0.0;
            while gone < half + 0.5 {
                gone += speed * 0.05;
                s.players.get_mut(&hero).unwrap().pos = along(channel[k], out, gone);
                tick(&mut s, 0.05);
            }
            assert!(until(&mut s, 8.0, |s| stage(s, boss) == "finished"));
            assert_eq!(s.players[&hero].health, before, "section {k}, side {side}");
        }
    }
}

#[test]
fn a_stagger_on_his_way_to_the_valve_only_delays_him() {
    let (mut s, _, boss) = race();
    let n = s.perch_channel(boss).unwrap().0.len();
    // Longer than he'd take to give the valve up: time reeling isn't time kept from it.
    s.stagger_mob(boss, GIVE_UP + 1.0, true);
    s.mobs.get_mut(&boss).unwrap().posture_guard = s.clock + 20.0;
    tick(&mut s, GIVE_UP + 0.5);
    assert_eq!(stage(&s, boss), "to valve");
    assert_eq!(s.perches.crews[&boss].wisps.len(), n, "still marked");
    s.events.clear();
    tick(&mut s, 0.6);
    assert!(!marks(&s.events, MARK_WILL).is_empty());
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"), "on to the valve after");
    assert!(!said(&s, LINE_HIMSELF) && !said(&s, LINE_STOPPED));
    assert!(s.mobs[&boss].posture_guard <= s.clock, "a stance broken on the way doesn't shield him at the valve");
}

#[test]
fn a_stagger_with_his_hands_on_the_valve_stops_it_for_good_and_leaves_him_open() {
    let (mut s, hero, boss) = race();
    let (channel, yaw, _) = s.perch_channel(boss).unwrap();
    let n = channel.len();
    // A hero standing in the channel: nothing may burn them now.
    s.players.get_mut(&hero).unwrap().pos = channel[3];
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    tick(&mut s, 1.0);
    let before = s.players[&hero].health;
    s.events.clear();
    let t = s.clock;
    s.stagger_mob(boss, 2.2, true);
    tick(&mut s, 0.1);
    assert_eq!(stage(&s, boss), "open");
    assert!(s.events.iter().any(|e| matches!(e, Event::CastEnd { caster } if *caster == boss)), "his hands come off it");
    // The mark goes out where it can be seen: one grey burst drawn back toward the valve, and
    // the wisps out one after another from the far end.
    let out = marks(&s.events, MARK_OUT);
    assert_eq!(out.len(), 1);
    assert!(angle_between(out[0].1, yaw + std::f32::consts::PI) < 0.5, "drawn back toward the valve");
    assert!(dist2d(out[0].0, along(channel[n - 1], yaw, STRIPE_STEP / 2.0)) < 0.01);
    assert_eq!(marks(&s.events, MARK_WILL).len(), 0, "no more of the mark");
    let left: Vec<Vec3> = s.perches.crews[&boss].visuals.iter().filter(|id| display(&s, **id) == Some(STEAM)).map(|id| s.objs[id].pos).collect();
    assert!(!left.is_empty() && left.len() < n, "going out: {} left", left.len());
    assert_eq!(left, channel[..left.len()], "the far end first");
    let m = &s.mobs[&boss];
    assert!(m.stagger_until >= t + VALVE_OPEN - 0.1 && m.riposte, "open");
    tick(&mut s, 0.8);
    assert_eq!(vapour(&s), 0, "every wisp out");
    assert!(said(&s, LINE_STOPPED), "(his lines keep 2 s apart, so within that of the blow)");
    assert_eq!(s.posture_of(boss), 0);
    s.hurt_mob_as(boss, hero, 300, "Melee");
    assert_eq!(s.posture_of(boss), 0, "one stop, not a chain of them");
    assert!(until(&mut s, VALVE_OPEN, |s| stage(s, boss) == "finished"));
    assert!((s.clock - t - VALVE_OPEN).abs() < 0.2, "open {VALVE_OPEN} s");
    assert!(said(&s, LINE_HIMSELF) && !said(&s, LINE_MELT), "his line fits what happened");
    let def = s.mobs[&boss].def;
    let flurry = s.content.mobs[def].patterns.iter().find(|p| p.name == FINISH).unwrap().clone();
    assert!(s.perch_pattern_allowed(boss, &flurry), "on to his finish");
    // For good: no whistle, no steam, no fire, however long the fight goes on.
    s.players.get_mut(&hero).unwrap().pos = channel[n - 1];
    tick(&mut s, 8.0);
    assert_eq!(stage(&s, boss), "finished");
    assert!(!s.events.iter().any(|e| matches!(e, Event::Sound { sound: 3431, .. })));
    assert!(!s.events.iter().any(|e| matches!(e, Event::Cast { spell: WORK_VALVE, .. })), "he doesn't go back to it");
    assert_eq!(marks(&s.events, MARK_SOON).len() + marks(&s.events, MARK_FIRE).len() + marks(&s.events, MARK_WILL).len(), 0);
    assert_eq!(vapour(&s), 0);
    assert!(s.players[&hero].health <= before, "(only his own blows since)");
    assert!(!s.events.iter().any(|e| matches!(e, Event::Missile { spell: MOLTEN, .. })));
}

#[test]
fn one_heavy_blow_knocks_him_off_the_valve_whatever_it_deals_and_light_blows_take_longer() {
    // One heavy blow, begun as his hands go on it.
    let (mut s, hero, boss) = race();
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    beside(&mut s, hero, boss);
    let began = s.clock;
    s.input(hero, Input::Heavy);
    while stage(&s, boss) == "working" { s.step(0.05); }
    let heavy = s.clock - began;
    assert_eq!(stage(&s, boss), "open", "one heavy blow");
    assert!(heavy < WORK_SECS - 0.5, "with time to spare: {heavy}");
    // Two light blows don't.
    let (mut s, hero, boss) = race();
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    beside(&mut s, hero, boss);
    let mut jabs = 0;
    while jabs < 2 && stage(&s, boss) == "working" {
        if matches!(s.players[&hero].action, Action::Ready) { s.input(hero, Input::Attack); jabs += 1; }
        s.step(0.05);
    }
    for _ in 0..12 { if stage(&s, boss) == "working" { s.step(0.05); } }
    assert_ne!(stage(&s, boss), "open", "two jabs don't");
    // Committed light blows from the moment his hands go on: about 2 s, as long as he needs.
    let (mut s, hero, boss) = race();
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    s.perches.crews.get_mut(&boss).unwrap().stage = Stage::Working { until: s.clock + 10.0 };
    beside(&mut s, hero, boss);
    let began = s.clock;
    while stage(&s, boss) == "working" && s.clock < began + 5.0 {
        s.input(hero, Input::Attack);
        s.step(0.05);
    }
    let light = s.clock - began;
    assert_eq!(stage(&s, boss), "open");
    assert!((1.2..=3.2).contains(&light), "light blows: {light}");
    // A heavy blow that deals next to nothing still does it, and with four in his fight.
    let (mut s, hero, boss) = race();
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    s.mobs.get_mut(&boss).unwrap().fighters = 4;
    beside(&mut s, hero, boss);
    s.input(hero, Input::Heavy);
    s.step(0.05);
    assert!(matches!(&s.players[&hero].action, Action::Swing { strike, .. } if strike.heavy()));
    s.hurt_mob_as(boss, hero, 1, "Melee");
    tick(&mut s, 0.05);
    assert_eq!(stage(&s, boss), "open");
    println!("at the valve: one heavy blow lands {heavy:.2} s after it's begun; light blows alone take {light:.2} s");
}

#[test]
fn kept_from_the_valve_he_gives_it_up_and_never_works_it_from_where_he_stands() {
    let (mut s, _, boss) = race();
    let held = s.mobs[&boss].pos;
    s.events.clear();
    let t = s.clock;
    // Held where he is, as a floor he can't cross would hold him.
    while stage(&s, boss) == "to valve" && s.clock < t + 12.0 {
        s.mobs.get_mut(&boss).unwrap().pos = held;
        tick(&mut s, 0.05);
    }
    assert_eq!(stage(&s, boss), "finished", "he gives it up");
    assert!((s.clock - t - GIVE_UP).abs() < 0.4, "after {GIVE_UP} s no nearer: {}", s.clock - t);
    assert!(!s.events.iter().any(|e| matches!(e, Event::Cast { spell: WORK_VALVE, .. })), "never worked from afar");
    assert!(!s.events.iter().any(|e| matches!(e, Event::Sound { sound: 3431, .. })));
    assert!(!s.missiles.iter().any(|m| m.owner == boss));
    assert_eq!(marks(&s.events, MARK_OUT).len(), 1, "the mark goes out");
    tick(&mut s, 0.1);
    assert!(said(&s, LINE_HIMSELF));
    s.events.clear();
    tick(&mut s, 1.0);
    assert_eq!(vapour(&s), 0);
    assert_eq!(marked(&s.events), 0);
}

#[test]
fn with_no_channel_or_nowhere_to_stand_at_the_valve_he_leaves_it_alone() {
    for case in 0..2 {
        let (mut s, hero, boss, crew) = fight();
        let c = s.perches.crews.get_mut(&boss).unwrap();
        if case == 0 { c.channel.truncate(CHANNEL_LEAST - 1); } else { c.stand = None; }
        s.events.clear();
        for id in crew { s.hurt_mob_as(id, hero, 10_000, "test"); }
        tick(&mut s, 0.1);
        assert_eq!(stage(&s, boss), "finished", "case {case}: straight on to his finish");
        tick(&mut s, 6.0);
        assert!(!s.events.iter().any(|e| matches!(e, Event::Cast { spell: WORK_VALVE, .. })), "a valve never worked for nothing");
        assert!(!s.events.iter().any(|e| matches!(e, Event::Sound { sound: 3431, .. })));
        assert_eq!(marked(&s.events), 0, "no mark, so no fire");
        assert!(!s.events.iter().any(|e| matches!(e, Event::Missile { spell: MOLTEN, .. })));
        assert_eq!(vapour(&s), 0);
        assert!(said(&s, "Useless. Every one of you.") && said(&s, LINE_HIMSELF));
    }
}

#[test]
fn no_fire_comes_from_a_dead_man() {
    for at in ["to valve", "working", "steam", "fire"] {
        let (mut s, hero, boss) = race();
        let (channel, ..) = s.perch_channel(boss).unwrap();
        // At the far end, where it burns last.
        s.players.get_mut(&hero).unwrap().pos = channel[channel.len() - 1];
        assert!(until(&mut s, 8.0, |s| stage(s, boss) == at));
        if at == "steam" || at == "fire" { assert!(s.missiles.iter().any(|m| m.owner == boss)); }
        let marks_: Vec<_> = s.missiles.iter().filter(|m| m.owner == boss).map(|m| m.mark).collect();
        let before = s.players[&hero].health;
        s.hurt_mob_as(boss, hero, 100_000, "test");
        assert!(s.perches.crews.is_empty());
        assert!(!s.missiles.iter().any(|m| m.owner == boss), "{at}: the fire still to come goes with him");
        assert!(marks_.iter().all(|id| s.mark_left(*id) == 0.0));
        assert_eq!(vapour(&s), 0, "{at}: steam and flames go at once");
        s.events.clear();
        tick(&mut s, 5.0);
        assert_eq!(marked(&s.events), 0, "{at}: no more of the mark");
        assert_eq!(s.players[&hero].health, before, "{at}");
    }
}

#[test]
fn a_lost_fight_takes_the_mark_the_steam_and_the_fire_at_once() {
    for at in ["to valve", "working", "steam", "fire"] {
        let (mut s, hero, boss) = race();
        assert!(until(&mut s, 8.0, |s| stage(s, boss) == at));
        if at == "fire" { tick(&mut s, 0.6); }
        assert!(vapour(&s) > 0);
        s.players.get_mut(&hero).unwrap().health = 0;
        s.foe_gone(hero);
        s.events.clear();
        tick(&mut s, 0.05);
        assert!(s.perches.crews.is_empty(), "{at}");
        assert!(!s.missiles.iter().any(|m| m.owner == boss), "{at}: the fire still to come");
        assert_eq!(vapour(&s), 0, "{at}: steam and flames");
        tick(&mut s, 5.0);
        assert_eq!(marked(&s.events), 0, "{at}: the mark");
    }
    // The last hero walks out of the fight instead: the same.
    for at in ["working", "steam"] {
        let (mut s, hero, boss) = race();
        assert!(until(&mut s, 8.0, |s| stage(s, boss) == at));
        s.players.get_mut(&hero).unwrap().pos = [300.0, 300.0, 0.0];
        assert!(until(&mut s, 3.0, |s| s.perches.crews.is_empty()), "{at}: he goes home");
        assert!(!s.missiles.iter().any(|m| m.owner == boss));
        assert_eq!(vapour(&s), 0);
        s.events.clear();
        tick(&mut s, 5.0);
        assert_eq!(marked(&s.events), 0);
    }
}

#[test]
fn a_hero_who_falls_while_a_friend_fights_on_ends_nothing() {
    let (mut s, first, boss) = race();
    let (channel, yaw, half) = s.perch_channel(boss).unwrap();
    let second = friend(&mut s, "second", along(channel[2], yaw + std::f32::consts::FRAC_PI_2, half + 1.0));
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    tick(&mut s, 0.1);
    assert_eq!(stage(&s, boss), "working", "his hands stay on the valve");
    assert_eq!(s.mobs[&boss].state.target(), Some(second));
    assert_eq!(s.perches.crews[&boss].wisps.len(), channel.len(), "still marked");
    let before = s.players[&second].health;
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "fire"));
    assert!(until(&mut s, 8.0, |s| stage(s, boss) == "finished"));
    assert_eq!(s.players[&second].health, before, "the friend, beside it, is dry");
    // And the friend can still stop it.
    let (mut s, first, boss) = race();
    let second = friend(&mut s, "second", [3.0, 3.0, 0.0]);
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    tick(&mut s, 0.1);
    beside(&mut s, second, boss);
    s.input(second, Input::Heavy);
    while stage(&s, boss) == "working" { s.step(0.05); }
    assert_eq!(stage(&s, boss), "open");
}

#[test]
fn two_heroes_one_at_the_valve_and_one_in_the_channel() {
    let (mut s, a, boss) = race();
    let (channel, ..) = s.perch_channel(boss).unwrap();
    let b = friend(&mut s, "b", channel[5]);
    s.join_fight(boss);
    assert_eq!(s.mobs[&boss].fighters, 2);
    assert!(until(&mut s, 4.0, |s| stage(s, boss) == "working"));
    let before = s.players[&b].health;
    // One heavy blow from the one at the valve is still enough with two in his fight.
    beside(&mut s, a, boss);
    s.input(a, Input::Heavy);
    while stage(&s, boss) == "working" { s.step(0.05); }
    assert_eq!(stage(&s, boss), "open", "stopped by the one at the valve");
    // The one in the channel never has to move.
    assert!(until(&mut s, 6.0, |s| stage(s, boss) == "finished"));
    assert_eq!(s.players[&b].health, before);
}

#[test]
fn a_hero_up_on_a_plank_is_not_burned_by_the_fire_on_the_floor_below() {
    let (mut s, hero, boss) = race();
    let (channel, ..) = s.perch_channel(boss).unwrap();
    let below = friend(&mut s, "below", channel[4]);
    let mut up = channel[4];
    up[2] += 8.6;
    s.players.get_mut(&hero).unwrap().pos = up;
    let before = (s.players[&hero].health, s.players[&below].health);
    assert!(until(&mut s, 12.0, |s| stage(s, boss) == "fire"));
    tick(&mut s, 3.0);
    assert_eq!(s.players[&hero].health, before.0, "8.6 yd over it");
    assert!(s.players[&below].health < before.1, "on the floor under them it burns");
}

#[test]
fn his_lines_never_come_closer_than_two_seconds() {
    let (mut s, hero, boss, crew) = fight();
    s.events.clear();
    let mut said = vec![(s.clock, "Back to work! All of you!".to_string())];
    let mut seen = 0;
    // Every worker down at once, the moment the first is lit: the lines pile up.
    let mut killed = false;
    for _ in 0..600 {
        tick(&mut s, 0.05);
        if !killed && s.perches.crews.get(&boss).is_some_and(|c| c.demo.is_some() && c.demo.as_ref().unwrap().alight) {
            for id in &crew { s.hurt_mob_as(*id, hero, 10_000, "test"); }
            killed = true;
        }
        for e in &s.events[seen..] {
            if let Event::Yell { id, text } = e { if *id == boss { said.push((s.clock, text.clone())); } }
        }
        seen = s.events.len();
        if s.perches.crews.get(&boss).is_some_and(|c| matches!(c.stage, Stage::Finished)) && said.len() >= 4 { break; }
    }
    let lines: Vec<&str> = said.iter().map(|(_, t)| t.as_str()).collect();
    assert_eq!(lines, ["Back to work! All of you!", "Mind the equipment!", "Useless. Every one of you.", "Then we'll melt the lot!"]);
    for w in said.windows(2) { assert!(w[1].0 - w[0].0 >= LINE_GAP - 0.01, "{w:?}"); }
}

#[test]
fn the_finish_is_a_flurry_that_sticks_his_spanner_in_the_floor_open_two_and_a_half_seconds() {
    let (mut s, _, boss, _) = fight();
    let def = s.mobs[&boss].def;
    let patterns = s.content.mobs[def].patterns.clone();
    let flurry = patterns.iter().position(|p| p.name == FINISH).unwrap();
    assert_eq!(patterns[flurry].steps.len(), 3);
    assert!(patterns[flurry].steps.iter().all(|st| st.lunge > 0.0), "pressing forward");
    assert!(!s.perch_pattern_allowed(boss, &patterns[flurry]), "not before the valve");
    for _ in 0..50 {
        s.begin_pattern(boss, flurry, true);
        assert_eq!(s.mobs[&boss].string_len, 0, "never cut short or run on");
    }
    s.perches.crews.get_mut(&boss).unwrap().stage = Stage::Finished;
    for p in &patterns { assert_eq!(s.perch_pattern_allowed(boss, p), p.name == FINISH, "{}", p.name); }
    let foe = s.mobs[&boss].state.foe().unwrap();
    let until_at = s.clock + patterns[flurry].steps[2].recovery.unwrap();
    s.mobs.get_mut(&boss).unwrap().state = MobState::Recover { foe, until: until_at };
    s.events.clear();
    s.perch_string_landed(boss, Some(flurry));
    // The last blow is the two-handed chop into the floor, and he's caught bent over it, held.
    let last = &patterns[flurry].steps[2];
    assert_eq!((last.clip, last.take), (Some(18), 1), "Attack2H's chop to the floor");
    assert!(s.events.iter().any(|e| matches!(e, Event::Freeze { id, after, secs } if *id == boss && *after == STUCK_AT && (*secs - (2.5 - STUCK_AT - 0.1)).abs() < 0.01)));
    assert!(s.posed(boss), "no flinch stands him up");
    s.clock = until_at - 0.05;
    assert!(s.posed(boss), "held the full 2.5 sec");
    s.clock = until_at - 2.5;
    assert!(s.events.iter().any(|e| matches!(e, Event::Visual { id, kit: combat::KIT_DUST, .. } if *id == boss)));
    // Another string doesn't stick.
    s.events.clear();
    s.perch_string_landed(boss, patterns.iter().position(|p| p.name == "two knocks"));
    assert!(s.events.is_empty());
    // Dying lets go of the pose.
    s.events.clear();
    s.remove_perch_crew(boss);
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: 0, .. } if *id == boss)));
    assert!(s.events.iter().any(|e| matches!(e, Event::Freeze { id, secs, .. } if *id == boss && *secs == 0.0)));
}

#[test]
fn debug_damage_never_counts_toward_his_posture() {
    let (mut s, hero, boss, _) = fight();
    s.hurt_mob_as(boss, hero, 200, "debug");
    assert_eq!(s.posture_of(boss), 0);
    s.hurt_mob_as(boss, hero, 20, "Melee");
    assert!(s.posture_of(boss) > 0);
}

#[test]
fn wiping_removes_crew_props_and_projectiles_and_repull_creates_only_one_crew() {
    let (mut s, _, boss, crew) = fight();
    tick(&mut s, 5.0);
    let props = s.perches.crews[&boss].props.clone();
    s.mobs.get_mut(&boss).unwrap().state = MobState::Return;
    s.step_perch_crews();
    assert!(crew.iter().all(|id| !s.mobs.contains_key(id)));
    assert!(props.iter().all(|id| !s.objs.contains_key(id)));
    assert!(s.missiles.iter().all(|m| !crew.contains(&m.owner)));
    s.mobs.get_mut(&boss).unwrap().state = MobState::Idle { wander_at: 99.0 };
    s.step_perch_crews();
    s.step_perch_crews();
    assert_eq!(s.perches.workers.len(), 2);
    assert_eq!(s.perches.crews[&boss].props.len(), 4, "two planks, two poles");
}

#[test]
fn a_ledge_with_no_floor_of_his_under_it_is_omitted_without_stranding_the_encounter() {
    let (mut s, _, boss, _) = fight();
    s.remove_perch_crew(boss);
    s.land = Box::new(FlatLand { z: 30.0, ..Default::default() });
    s.place_perch_crew(boss);
    assert!(s.perches.crews[&boss].workers.is_empty());
    s.step_perch_crews();
    // (That floor carries no channel either: no valve, straight on to his finish.)
    assert!(matches!(s.perches.crews[&boss].stage, Stage::Finished));
}

#[test]
fn owner_death_cleans_pending_sticks_flames_and_throws() {
    let (mut s, hero, boss, crew) = fight();
    assert!(until(&mut s, 8.0, |s| s.perches.crews[&boss].visuals.iter().any(|id| display(s, *id) == Some(STICK))));
    let props = s.perches.crews[&boss].props.clone();
    let marks: Vec<_> = s.missiles.iter().filter(|m| m.owner == boss || crew.contains(&m.owner)).map(|m| m.mark).collect();
    assert!(!marks.is_empty());
    s.hurt_mob_as(boss, hero, 100_000, "test");
    assert!(props.iter().all(|id| !s.objs.contains_key(id)));
    assert!(crew.iter().all(|id| !s.mobs.contains_key(id)));
    assert!(s.missiles.iter().all(|m| m.owner != boss && !crew.contains(&m.owner)));
    assert!(marks.iter().all(|id| s.mark_left(*id) == 0.0));
}

#[test]
fn a_surviving_party_member_keeps_the_same_crew_and_can_drop_a_worker() {
    let (mut s, first, boss, crew) = fight();
    let second = s.join("smith2", "Smith2", None);
    let p = s.players.get_mut(&second).unwrap();
    p.pos = [2.0, 0.0, 0.0];
    p.health = 100_000;
    p.max_health = 100_000;
    tick(&mut s, 0.2);
    s.players.get_mut(&first).unwrap().health = 0;
    s.foe_gone(first);
    tick(&mut s, 0.1);
    assert_eq!(s.mobs[&boss].state.target(), Some(second));
    assert_eq!(s.perches.crews[&boss].workers, crew);
    assert!(crew.iter().all(|id| s.mobs[id].state.target() == Some(second)));
    shot(&mut s, second, crew[0], 1.0, HitKind::Projectile);
    assert!(matches!(s.perches.workers[&crew[0]].state, Place::Falling { .. }));
    assert_eq!(s.perches.workers.len(), 2);
}

#[test]
fn immediate_visuals_are_encounter_owned_and_never_a_standing_plank_or_pole() {
    let (mut s, _, boss, crew) = fight();
    for id in &crew {
        let w = &s.perches.workers[id];
        assert!(!s.is_perch_visual_prop(w.support));
        assert!(!s.is_perch_visual_prop(w.plank));
    }
    for d in [PLANK, POLE, STEAM, LUMBER, STICK, FIRE] {
        let unrelated = s.perch_prop([20.0, 0.0, 0.0], d, 1.0, 0.0);
        assert!(!s.is_perch_visual_prop(unrelated));
    }
    s.drop_perch(crew[0]);
    let fallen: Vec<Id> = s.perches.crews[&boss].visuals.clone();
    assert!(fallen.iter().any(|id| display(&s, *id) == Some(LUMBER)));
    assert!(fallen.iter().any(|id| display(&s, *id) == Some(PLANK)), "the fallen plank lies flat, no hull");
    assert!(fallen.iter().all(|id| s.is_perch_visual_prop(*id)));
    s.remove_perch_crew(boss);
    assert!(fallen.iter().all(|id| !s.is_perch_visual_prop(*id) && !s.objs.contains_key(id)));
}

/// Nothing walks north of y 3: a hero out there stands where Gilnid finds no way (a plank over the
/// pit is that on the real map: an object, not floor, with the pit 9 yd under it).
struct OverTheDrop;
impl Land for OverTheDrop {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn plans(&self) -> bool { true }
    fn walk_clear(&self, from: Vec3, to: Vec3) -> bool { from[1] <= 3.0 && to[1] <= 3.0 }
}

/// A hero out on a plank used to have him give up after 4 sec: crew cleared, the fight reset. He
/// waits for them now, as Cookie does, slipping their shots; stepping back where he can walk ends
/// it, and after 10 sec with no way he goes home as before.
#[test]
fn gilnid_waits_for_a_hero_out_where_he_cannot_walk_and_slips_their_shots() {
    for come_back in [false, true] {
        let (mut s, hero, boss, _) = fight();
        s.land = Box::new(OverTheDrop);
        s.move_player(hero, [0.0, 9.0, 0.0], 0.0, Moving::default());
        assert!(until(&mut s, 5.0, |s| s.mobs[&boss].lost_at.is_some()), "he finds no way");
        let lost = s.mobs[&boss].lost_at.unwrap();
        assert!(until(&mut s, 2.0, |s| s.perch_evading(boss)));
        assert!(s.evading(boss, hero));
        let hp = s.mobs[&boss].health;
        shot(&mut s, hero, boss, 50.0, HitKind::Projectile);
        tick(&mut s, 0.05);
        assert_eq!(s.mobs[&boss].health, hp, "nothing reaches him while he waits");
        // Past the old 4 sec: still in his fight, his crew still up.
        let wait = (lost + 6.0 - s.clock).max(0.0);
        tick(&mut s, wait);
        assert!(!matches!(s.mobs[&boss].state, MobState::Return | MobState::Idle { .. }), "{:?}", s.mobs[&boss].state);
        assert!(s.perches.crews.contains_key(&boss), "the crew stays");
        if come_back {
            s.move_player(hero, [1.5, 0.0, 0.0], 0.0, Moving::default());
            assert!(until(&mut s, 6.0, |s| !s.evading(boss, hero) && s.mobs[&boss].lost_at.is_none()), "back where he walks, he comes");
            assert!(!matches!(s.mobs[&boss].state, MobState::Return));
        } else {
            assert!(until(&mut s, super::kitchen::HOLD_ON + 1.0, |s| matches!(s.mobs[&boss].state, MobState::Return)));
            assert!(s.clock - lost >= super::kitchen::HOLD_ON - 0.06, "he waited {:.1} s", s.clock - lost);
        }
    }
}
