//! Creatures' tactics: whom they go for, an opening pressed at once, a guard met with the blow
//! no guard stops, ground given after a string, a hero's back sought while waiting a turn, a
//! raider drawn off its station by a hero near; a boss that varies its strings and makes space.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    id
}

fn run(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / DT).round() as usize {
        s.step(DT);
    }
}

#[test]
fn it_turns_on_the_hero_who_hurts_it_most_not_only_the_first_it_saw() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 8.0, 0.0], 0.0);
    let k = mob(&mut s, "grunt", [4.0, 4.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().health = 5000;
    s.mobs.get_mut(&k).unwrap().max_health = 5000;
    s.engage(k, Foe::Hero(a));
    s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 100.0;
    run(&mut s, 2.0);
    assert_eq!(s.mobs[&k].state.target(), Some(a), "on the first it saw");
    // Bren lays into it.
    for _ in 0..3 {
        s.hurt_mob(k, b, 600);
    }
    let t = s.content.ai.retarget_secs + 0.3;
    run(&mut s, t);
    assert_eq!(s.mobs[&k].state.target(), Some(b), "on whoever hurts it");
}

#[test]
fn two_creatures_on_two_heroes_spread_between_them() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 6.0, 0.0], 0.0);
    let ks: Vec<Id> = (0..4).map(|i| mob(&mut s, "wolf", [6.0, i as f32 * 2.0 - 1.0, 0.0])).collect();
    for k in &ks {
        s.mobs.get_mut(k).unwrap().ready_at = s.clock + 100.0;
        s.engage(*k, Foe::Hero(a));
    }
    let t = s.content.ai.retarget_secs * 2.0 + 0.5;
    run(&mut s, t);
    let on_b = ks.iter().filter(|k| s.mobs[k].state.target() == Some(b)).count();
    assert!(on_b >= 1, "not all of them on Ayla: {on_b} on Bren");
}

#[test]
fn a_heal_in_reach_is_pressed_at_once_past_its_cooldown() {
    for eating in [false, true] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        let k = mob(&mut s, "grunt", [2.5, 0.0, 0.0]);
        s.engage(k, Foe::Hero(p));
        s.step(DT);
        s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
        s.mobs.get_mut(&k).unwrap().ready_at = s.clock + 0.9;
        if eating {
            let meat = s.content.item("roast").unwrap();
            s.players.get_mut(&p).unwrap().action = Action::Eat { t: 0.0, item: meat };
        }
        s.step(DT);
        let winding = matches!(s.mobs[&k].state, MobState::Windup { .. });
        assert_eq!(winding, eating, "eating {eating}");
    }
}

#[test]
fn a_raised_guard_draws_the_blow_no_guard_stops() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "overseer", [2.5, 0.0, 0.0]);
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let d = s.content.mobs[s.mobs[&k].def].clone();
    let breaker = |k: usize| d.patterns[k].steps.iter().any(|st| st.blockable == Some(false) || st.heavy == Some(true));
    let count = |s: &mut Survival| (0..400).filter_map(|_| s.pick_pattern(k, 2.5, 0.4)).filter(|i| breaker(*i)).count();
    let open = count(&mut s);
    s.players.get_mut(&p).unwrap().guard_held = true;
    s.players.get_mut(&p).unwrap().action = Action::Guard;
    let guarded = count(&mut s);
    assert!(guarded as f32 > open as f32 * 1.4, "{open} open, {guarded} against a guard");
}

#[test]
fn now_and_then_it_gives_ground_after_a_string() {
    let mut s = world();
    s.content.ai.retreat = 1.0;
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "grunt", [2.2, 0.0, 0.0]);
    s.players.get_mut(&p).unwrap().health = 100_000;
    s.players.get_mut(&p).unwrap().max_health = 100_000;
    s.engage(k, Foe::Hero(p));
    // Until its first string is done and it's back to chasing.
    let mut furthest: f32 = 0.0;
    let mut done = false;
    for _ in 0..200 {
        let was = matches!(s.mobs[&k].state, MobState::Recover { .. });
        s.step(DT);
        if was && matches!(s.mobs[&k].state, MobState::Chase { .. }) {
            done = true;
        }
        if done {
            furthest = furthest.max(dist2d(s.mobs[&k].pos, ORIGIN));
        }
        if done && furthest > 4.0 {
            break;
        }
    }
    assert!(done, "a string came and went");
    assert!(furthest > 4.0, "it stepped back: {furthest}");
}

#[test]
fn waiting_its_turn_it_works_round_to_the_heros_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // In front of her, a little to one side.
    let k = mob(&mut s, "wolf", [3.0, 0.8, 0.0]);
    let way = s.flank_way(k, p).expect("not at her back yet");
    let at = facing(ORIGIN, s.mobs[&k].pos);
    let back = std::f32::consts::PI;
    let off = |a: f32| ((a - back + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI).abs();
    assert!(off(at + way * 0.6) < off(at), "toward her back");
    // Behind her: its own way.
    s.mobs.get_mut(&k).unwrap().pos = [-3.0, 0.2, 0.0];
    assert!(s.flank_way(k, p).is_none());
}

#[test]
fn a_raider_at_a_station_turns_on_a_hero_who_comes_near() {
    let mut s = world();
    let def = s.content.mob("grunt").unwrap();
    let k = s.spawn_mob(def, [30.0, 0.0, 0.0], None, true);
    let st = s.alloc();
    let pal = s.content.station("palisade").unwrap();
    s.objs.insert(st, Obj { kind: ObjKind::Station { def: pal, uid: 99, health: 700, damaged: false }, pos: [32.0, 0.0, 0.0], facing: 0.0 });
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Obj(st) };
    let p = hero(&mut s, "Ayla", [80.0, 0.0, 0.0], 0.0);
    run(&mut s, 1.5);
    assert!(matches!(s.mobs[&k].state.foe(), Some(Foe::Obj(_))), "nobody near: the station");
    let at = s.mobs[&k].pos;
    s.move_player(p, [at[0] - 4.0, at[1], 0.0], 0.0, Moving::default());

    let t = s.content.ai.retarget_secs + 0.3;
    run(&mut s, t);
    assert_eq!(s.mobs[&k].state.target(), Some(p), "drawn off by the hero");
}

/// A pack of `n` wolves at `at`, linked: drawing one draws them all.
fn pack(s: &mut Survival, n: usize, at: [f32; 2]) -> Vec<Id> {
    s.content.packs.push(crate::content::PackDef { mob: "wolf".into(), at, radius: 2.0, count: n, link: 12.0, route: Vec::new(), wait: 1.0, camp: String::new(),
        z: None,
        key: String::new(),
        elite: false,
        hidden: 0.0, wake: String::new(), spots: Vec::new(), rare: false,
    });
    let k = s.content.packs.len() - 1;
    let def = s.content.mob("wolf").unwrap();
    (0..n)
        .map(|i| {
            let id = s.spawn_mob(def, [at[0] + i as f32, at[1], 0.0], Some(k), false);
            s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
            id
        })
        .collect()
}

#[test]
fn a_pack_drawn_by_one_hero_spreads_over_the_band_as_it_comes() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 5.0, 0.0], 0.0);
    let c = hero(&mut s, "Cass", [0.0, -5.0, 0.0], 0.0);
    let ks = pack(&mut s, 6, [8.0, 0.0]);
    // Ayla pulls: the first comes for her, the rest spread over the three of them.
    s.engage(ks[0], Foe::Hero(a));
    let on = |s: &Survival, h: Id| ks.iter().filter(|k| s.mobs[k].state.target() == Some(h)).count();
    assert_eq!(on(&s, a) + on(&s, b) + on(&s, c), 6, "all six came");
    assert!(on(&s, a) <= 3 && on(&s, b) >= 1 && on(&s, c) >= 1, "spread: {} {} {}", on(&s, a), on(&s, b), on(&s, c));
    // And over a fight they stay spread rather than piling onto one.
    run(&mut s, 6.0);
    let most = [a, b, c].iter().map(|h| on(&s, *h)).max().unwrap();
    assert!(most <= 3, "no one hero with most of them: {} {} {}", on(&s, a), on(&s, b), on(&s, c));
}

#[test]
fn alone_a_hero_has_the_whole_pack() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let ks = pack(&mut s, 4, [8.0, 0.0]);
    s.engage(ks[0], Foe::Hero(a));
    assert!(ks.iter().all(|k| s.mobs[k].state.target() == Some(a)));
}

#[test]
fn a_creature_with_no_turn_at_its_hero_goes_for_one_nobody_is_pressing() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
    let b = hero(&mut s, "Bren", [0.0, 7.0, 0.0], 0.0);
    // Three on Ayla, none on Bren: two press her, the third goes for Bren.
    let ks: Vec<Id> = (0..3).map(|i| mob(&mut s, "wolf", [2.0, i as f32 - 1.0, 0.0])).collect();
    for k in &ks {
        s.engage(*k, Foe::Hero(a));
    }
    run(&mut s, 3.0);
    assert!(ks.iter().any(|k| s.mobs[k].state.target() == Some(b)), "one turned to Bren: {:?}", ks.iter().map(|k| s.mobs[k].state.target()).collect::<Vec<_>>());
}

#[test]
fn a_boss_whose_foe_runs_past_its_leash_turns_on_another_in_its_fight_not_home() {
    let mut s = world();
    let a = hero(&mut s, "Ayla", [3.0, 0.0, 0.0], 0.0);
    // Bren shoots it from across its room (farther than a pack's fight reaches, inside the room:
    // its own leash).
    let leash = s.leash_of(s.content.mob("wolf").unwrap());
    let room = s.content.mobs[s.content.mob("wolf").unwrap()].leash;
    let b = hero(&mut s, "Bren", [-(room - 2.0), 0.0, 0.0], 0.0);
    let k = mob(&mut s, "wolf", ORIGIN);
    let def = s.mobs[&k].def;
    s.content.mobs[def].boss = true;
    s.mobs.get_mut(&k).unwrap().health = 50_000;
    s.mobs.get_mut(&k).unwrap().max_health = 50_000;
    s.engage(k, Foe::Hero(a));
    s.hurt_mob(k, b, 10);
    // Ayla runs far off, past its leash.
    s.move_player(a, [leash + 40.0, 0.0, 0.0], 0.0, Moving::default());
    run(&mut s, 1.0);
    assert_eq!(s.mobs[&k].state.target(), Some(b), "on Bren: {:?}", s.mobs[&k].state);
    assert!(s.mobs[&k].health < 50_000, "not home healed");
}

#[test]
fn a_cast_naming_a_clip_plays_it_and_one_that_doesnt_plays_its_spells_own() {
    let parse = |t: &str| -> crate::content::PatternDef { toml::from_str(t).unwrap() };
    for (with, clip) in [(true, Some(54u16)), (false, None)] {
        let mut s = world();
        let a = hero(&mut s, "Ayla", [0.0, 0.0, 0.0], 0.0);
        let k = mob(&mut s, "grunt", [8.0, 0.0, 0.0]);
        let def = s.mobs[&k].def;
        let step = if with { "{ ranged = true, clip = 54, take = 0, windup = 1.0, damage = 5, range = 20.0, radius = 1.5, speed = 20.0, spell = 133 }" } else { "{ ranged = true, windup = 1.0, damage = 5, range = 20.0, radius = 1.5, speed = 20.0, spell = 133 }" };
        s.content.mobs[def].patterns = vec![parse(&format!("name = \"volley\"\nmin_range = 3.0\nmax_range = 20.0\nsteps = [ {step} ]"))];
        s.content.mobs[def].attack.ranged = true;
        s.content.mobs[def].attack.range = 20.0;
        s.engage(k, Foe::Hero(a));
        let mut seen = None;
        for _ in 0..60 {
            for e in s.step(DT) {
                if let Event::MobWindup { id, clip, take, .. } = e {
                    if id == k {
                        seen = Some((clip, take));
                    }
                }
            }
        }
        assert_eq!(seen.map(|c| c.0), clip, "with a clip: {with}");
    }
}

/// A real boss from the Deadmines, alone with a hero who can't fall, its health past dying too.
fn boss(s: &mut Survival, key: &str, at: Vec3) -> Id {
    let k = mob(s, key, at);
    assert!(s.content.mobs[s.mobs[&k].def].boss, "{key} is a boss");
    let m = s.mobs.get_mut(&k).unwrap();
    m.health = 1_000_000;
    m.max_health = 1_000_000;
    k
}

fn sturdy(s: &mut Survival, name: &str, pos: Vec3) -> Id {
    let p = hero(s, name, pos, 0.0);
    let pl = s.players.get_mut(&p).unwrap();
    pl.health = 10_000_000;
    pl.max_health = 10_000_000;
    p
}

fn pattern(s: &Survival, k: Id, name: &str) -> usize {
    s.content.mobs[s.mobs[&k].def].patterns.iter().position(|p| p.name == name).unwrap_or_else(|| panic!("{name}"))
}

#[test]
fn a_boss_seldom_begins_the_same_string_twice_running() {
    let mut s = world();
    let k = boss(&mut s, "dm_rhahkzor", ORIGIN);
    let two = pattern(&s, k, "two swings");
    let picks = |s: &mut Survival| {
        let mut again = 0;
        for _ in 0..2000 {
            s.mobs.get_mut(&k).unwrap().last_pattern = Some(two);
            if s.pick_pattern(k, 2.0, 0.4) == Some(two) {
                again += 1;
            }
        }
        again
    };
    s.content.ai.repeat = 1.0;
    let free = picks(&mut s);
    s.content.ai.repeat = 0.3;
    let damped = picks(&mut s);
    assert!(free > 700, "its heaviest string up close, most of the time: {free}");
    assert!((damped as f32) < free as f32 * 0.6, "seldom twice running: {damped} against {free}");
    assert!(damped > 0, "still now and then");
}

#[test]
fn alone_up_close_a_boss_backs_off_and_uses_a_blow_from_afar() {
    let mut s = world();
    s.content.ai.space = 1.0;
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_rhahkzor", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    // Ayla stands her ground, right on it.
    let mut farthest: f32 = 0.0;
    let mut from_afar = None;
    for _ in 0..(30.0 / DT) as usize {
        s.step(DT);
        let m = &s.mobs[&k];
        farthest = farthest.max(dist2d(m.pos, ORIGIN));
        if let (Some((pk, 0)), MobState::Windup { .. }) = (m.pattern, &m.state) {
            let pat = &s.content.mobs[m.def].patterns[pk];
            if pat.min_range >= 3.0 {
                from_afar = Some(pat.name.clone());
                break;
            }
        }
    }
    assert!(farthest >= 6.0, "it backed off: {farthest}");
    assert_eq!(from_afar.as_deref(), Some("rush"), "and charged from there");
}

#[test]
fn a_boss_that_never_makes_space_never_charges_a_hero_standing_on_it() {
    // What it was before: alone, up close, its charge never came.
    let mut s = world();
    s.content.ai.space = 0.0;
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_rhahkzor", [2.5, 0.0, 0.0]);
    s.engage(k, Foe::Hero(p));
    let rush = pattern(&s, k, "rush");
    for _ in 0..(20.0 / DT) as usize {
        s.step(DT);
        assert_ne!(s.mobs[&k].pattern.map(|p| p.0), Some(rush), "no room for a rush");
    }
}

#[test]
fn with_a_band_on_it_a_boss_makes_space_less_often() {
    let tries = |band: bool| {
        let mut s = world();
        s.content.ai.space = 0.4;
        s.content.ai.space_streak = 0.0;
        let p = sturdy(&mut s, "Ayla", ORIGIN);
        let k = boss(&mut s, "dm_rhahkzor", [2.5, 0.0, 0.0]);
        if band {
            let b = sturdy(&mut s, "Bren", [0.0, 3.0, 0.0]);
            s.engage(k, Foe::Hero(p));
            s.hurt_mob(k, b, 1);
        } else {
            s.engage(k, Foe::Hero(p));
        }
        let mut made = 0;
        for _ in 0..400 {
            let m = s.mobs.get_mut(&k).unwrap();
            m.pos = [2.5, 0.0, 0.0];
            m.path.clear();
            m.space = None;
            m.stagger_until = 0.0;
            m.close_run = 1;
            if s.maybe_make_space(k, p) {
                made += 1;
            }
        }
        made
    };
    let (alone, band) = (tries(false), tries(true));
    assert!(alone > 120 && alone < 200, "about 40% alone: {alone}");
    assert!(band < alone * 2 / 3, "about half that with a band: {band} against {alone}");
}

#[test]
fn a_boss_holds_one_blow_of_a_string_up_close_a_beat_now_and_then() {
    let mut s = world();
    s.content.ai.hold = 1.0;
    s.content.ai.cut = 0.0;
    s.content.ai.extend = 0.0;
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_rhahkzor", [2.0, 0.0, 0.0]);
    let two = pattern(&s, k, "two swings");
    s.begin_pattern(k, two, true);
    let Some(held) = s.mobs[&k].held_step else { panic!("one of its blows is held") };
    assert!(held < 2, "one of its two");
    let mut beats = Vec::new();
    for step in 0..2 {
        s.mobs.get_mut(&k).unwrap().pattern = Some((two, step));
        let own = s.pattern_step(k).unwrap().windup.unwrap_or(s.content.mobs[s.mobs[&k].def].attack.windup);
        s.windup(k, Foe::Hero(p), ORIGIN);
        let hold = s.mobs[&k].hold;
        assert!((s.attack_now(k).windup - (own + hold)).abs() < 1e-3, "its whole wind-up the longer");
        beats.push(hold);
    }
    let most = s.content.ai.hold_secs;
    let h = beats[held];
    assert!(h >= most * 0.4 - 1e-4 && h <= most + 1e-4, "held a beat: {h}");
    assert_eq!(beats[1 - held], 0.0, "the other comes on time");
    // Never a blow from afar (its charge comes as it always has), nor a grunt's.
    let rush = pattern(&s, k, "rush");
    s.begin_pattern(k, rush, true);
    assert_eq!(s.mobs[&k].held_step, None, "a charge isn't held");
    let g = mob(&mut s, "grunt", [-2.0, 0.0, 0.0]);
    if !s.content.mobs[s.mobs[&g].def].patterns.is_empty() {
        s.begin_pattern(g, 0, true);
        assert_eq!(s.mobs[&g].held_step, None, "a grunt holds none");
    }
}

/// How many blows a boss throws of a string it begins.
fn blows_of(s: &mut Survival, k: Id, p: Id, which: usize) -> usize {
    s.engage(k, Foe::Hero(p));
    s.begin_pattern(k, which, true);
    s.windup(k, Foe::Hero(p), ORIGIN);
    let mut blows = 0;
    let mut was_windup = true;
    for _ in 0..(15.0 / DT) as usize {
        s.step(DT);
        let w = matches!(s.mobs[&k].state, MobState::Windup { .. });
        if was_windup && !w {
            blows += 1;
        }
        was_windup = w;
        if s.mobs[&k].pattern.is_none() {
            break;
        }
    }
    blows
}

#[test]
fn a_boss_stops_a_long_string_short_or_runs_it_one_longer_now_and_then() {
    let calm = |s: &mut Survival| {
        s.content.ai.space = 0.0;
        s.content.ai.chain = 0.0;
        s.content.ai.hold = 0.0;
    };
    let mut s = world();
    calm(&mut s);
    s.content.ai.cut = 1.0;
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_vancleef", [2.0, 0.0, 0.0]);
    let string = pattern(&s, k, "string");
    assert_eq!(s.content.mobs[s.mobs[&k].def].patterns[string].steps.len(), 4);
    assert_eq!(blows_of(&mut s, k, p, string), 3, "one short of its four");

    let mut s = world();
    calm(&mut s);
    s.content.ai.cut = 0.0;
    s.content.ai.extend = 1.0;
    // Its fourth blow light here: a heavy one knocks the hero down, and a knockdown ends a string.
    let def = s.content.mob("dm_vancleef").unwrap();
    for step in s.content.mobs[def].patterns[string].steps.iter_mut() {
        step.heavy = Some(false);
    }
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_vancleef", [2.0, 0.0, 0.0]);
    assert_eq!(blows_of(&mut s, k, p, string), 5, "one past its four");

    let mut s = world();
    calm(&mut s);
    s.content.ai.cut = 0.0;
    s.content.ai.extend = 0.0;
    let p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_vancleef", [2.0, 0.0, 0.0]);
    assert_eq!(blows_of(&mut s, k, p, string), 4, "as written, otherwise");
}

#[test]
fn a_boss_goes_straight_into_another_string_now_and_then() {
    let mut s = world();
    s.content.ai.chain = 1.0;
    let _p = sturdy(&mut s, "Ayla", ORIGIN);
    let k = boss(&mut s, "dm_rhahkzor", [2.0, 0.0, 0.0]);
    let now = s.clock;
    let m = s.mobs.get_mut(&k).unwrap();
    m.ready_at = now + 3.0;
    m.close_run = 1;
    s.maybe_chain(k);
    assert!(s.mobs[&k].ready_at <= now + 0.2, "no rest: {}", s.mobs[&k].ready_at - now);
    // Not a fourth close string in a row.
    let m = s.mobs.get_mut(&k).unwrap();
    m.ready_at = now + 3.0;
    m.close_run = 3;
    s.maybe_chain(k);
    assert!(s.mobs[&k].ready_at >= now + 2.9, "it rests after three");
}

#[test]
fn a_boss_backed_off_too_long_ago_isnt_held_to_its_plan() {
    let mut s = world();
    let k = boss(&mut s, "dm_cookie_gorged", ORIGIN);
    // Synthetic generic boss: Cookie's authored kitchen rhythm no longer uses this planner.
    let def = s.mobs[&k].def;
    let d = &mut s.content.mobs[def];
    d.key = "test_planning_boss".into();
    for p in &mut d.patterns { p.max_range = 12.0; }
    let rush = pattern(&s, k, "slaps");
    s.content.ai.repeat = 1.0;
    let now = s.clock;
    // Its wait still on: the rush it backed off for.
    s.mobs.get_mut(&k).unwrap().space = Some((now + 1.0, rush));
    assert!((0..50).all(|_| s.pick_pattern(k, 8.0, 0.4) == Some(rush)), "the planned string");
    // Its wait out: it picks among what fits again.
    s.mobs.get_mut(&k).unwrap().space = Some((now - 0.1, rush));
    let other = (0..400).filter(|_| s.pick_pattern(k, 8.0, 0.4) != Some(rush)).count();
    assert!(other > 0, "not held to a stale plan");
}
