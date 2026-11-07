//! Every creature's blow against a human hero's reaction: can it be answered (a dodge or the
//! guard) from standing ready, from inside the hero's own light swing, and after answering the blow
//! before it in its string? The souls games' rule: a blow you can't answer in time is a blow you
//! can't learn, only avoid by never acting. (Nov 6 play-test: "mid-swing" was the commonest way a
//! Deadmines foot soldier caught a hero.)
//!
//! `cargo test --release -p htl-survival fair_blows_report -- --ignored --nocapture` prints every
//! blow of every creature with its verdicts.

use crate::content::{AttackDef, Content, MobDef, PatternDef};
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

/// How long a player takes to see a clear wind-up and press (a little over the 0.25 s of a simple
/// reaction test: the tell has to be read, not just seen).
pub(crate) const REACTION: f32 = 0.30;
/// A follow-up in a string already seen coming: read faster.
pub(crate) const FOLLOW_REACTION: f32 = 0.15;

/// The light swing the hero is most often inside when a blow comes: the starting sword's first.
fn sword_lock(c: &Content) -> f32 {
    let w = c.weapons.iter().find(|w| w.key == "sword").expect("the sword");
    w.windup + w.active + w.recovery * c.combat.cancel_after
}

/// When a dodge pressed now first evades.
fn evade_from(c: &Content) -> f32 {
    c.dodge.evade_from
}

/// After a dodge timed to a blow (its evade window centred on the landing), how long until the
/// hero can act again.
fn free_after_dodge(c: &Content) -> f32 {
    let mid = 0.5 * (c.dodge.evade_from + c.dodge.evade_to);
    c.dodge.secs - mid
}

/// The fastest its blows come: its quickest phase's pace (the enrage among them).
fn fastest(d: &MobDef) -> f32 {
    d.phases.iter().map(|p| p.speed).fold(1.0_f32, f32::min)
}

/// One blow, as it lands.
#[derive(Debug, Clone)]
pub(crate) struct Blow {
    pub mob: String,
    pub string: String,
    pub step: usize,
    pub windup: f32,
    /// Its wind-up at the creature's quickest pace.
    pub windup_fast: f32,
    /// From the blow before it landing to this one landing (none for a string's first).
    pub gap: Option<f32>,
    pub blockable: bool,
    pub ranged: bool,
    pub damage: u32,
}

fn steps_of(d: &MobDef, p: &PatternDef) -> Vec<AttackDef> {
    p.steps.iter().map(|s| s.over(&d.attack)).collect()
}

/// Every blow every creature can throw: its strings' steps, or its one attack.
pub(crate) fn blows(c: &Content) -> Vec<Blow> {
    let mut out = Vec::new();
    for d in &c.mobs {
        if d.inert() || d.critter {
            continue;
        }
        let fast = fastest(d);
        let strings: Vec<(String, Vec<AttackDef>)> = if d.patterns.is_empty() {
            let mut v = vec![("attack".to_string(), vec![d.attack.clone()])];
            if let Some(s) = &d.special {
                v.push(("special".to_string(), vec![s.over(&d.attack)]));
            }
            v
        } else {
            d.patterns.iter().map(|p| (p.name.clone(), steps_of(d, p))).collect()
        };
        for (name, steps) in strings {
            for (k, a) in steps.iter().enumerate() {
                let gap = (k > 0).then(|| steps[k - 1].recovery + a.windup);
                let floor = if k == 0 && !a.ranged { c.ai.first_blow } else { 0.0 };
                out.push(Blow {
                    mob: d.key.clone(),
                    string: name.clone(),
                    step: k,
                    windup: a.windup.max(floor),
                    windup_fast: (a.windup * fast).max(floor),
                    gap,
                    blockable: a.blockable,
                    ranged: a.ranged,
                    damage: a.damage,
                });
            }
        }
    }
    out
}

/// The three verdicts on a blow: answerable standing ready, from inside a light swing, after the
/// blow before it (its string's follow-up). At the creature's quickest pace.
pub(crate) fn verdict(c: &Content, b: &Blow) -> (bool, bool, bool) {
    let w = b.windup_fast;
    let first = b.step == 0;
    let ready = !first || w >= REACTION + evade_from(c);
    let swinging = !first || w >= REACTION.max(sword_lock(c)) + evade_from(c);
    let follow = match b.gap {
        // A blow a guard stops can be met with the guard held from the last one; one it doesn't
        // needs the hero free of their last dodge, then a read and a dodge.
        Some(g) if !b.blockable => g * fastest_of(c, &b.mob) >= free_after_dodge(c) + FOLLOW_REACTION + evade_from(c),
        _ => true,
    };
    (ready, swinging, follow)
}

fn fastest_of(c: &Content, key: &str) -> f32 {
    c.mobs.iter().find(|d| d.key == key).map_or(1.0, fastest)
}

#[test]
fn no_first_blow_up_close_comes_quicker_than_a_hero_can_answer_from_a_swing() {
    let c = Content::builtin();
    let need = REACTION.max(sword_lock(&c)) + evade_from(&c);
    assert!(c.ai.first_blow >= need, "the floor {} under what a swing needs, {need}", c.ai.first_blow);
    let late: Vec<String> = blows(&c)
        .iter()
        .filter(|b| b.step == 0 && !b.ranged && b.windup_fast < need)
        .map(|b| format!("{} {}", b.mob, b.string))
        .collect();
    assert!(late.is_empty(), "first blows too quick: {late:?}");
}

#[test]
fn a_pirates_first_slash_waits_for_the_floor_and_its_second_keeps_its_pace() {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.ai.first_blow = 0.6;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    let def = s.content.mob("dm_pirate").unwrap();
    let m = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&m).unwrap().state = MobState::Idle { wander_at: 1e9 };
    let k = s.content.mobs[def].patterns.iter().position(|p| p.name == "two slashes").unwrap();
    s.begin_pattern(m, k, false);
    assert!((s.attack_now(m).windup - 0.6).abs() < 1e-4, "first slash 0.5 raised to 0.6: {}", s.attack_now(m).windup);
    s.mobs.get_mut(&m).unwrap().pattern = Some((k, 1));
    assert!((s.attack_now(m).windup - 0.4).abs() < 1e-4, "the second keeps its 0.4: {}", s.attack_now(m).windup);
    let _ = Foe::Hero(p);
}

/// A quiet world with the built-in rules, the turns and the arrival beat on.
fn turns_world() -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.ai.turn_beat = 0.35;
    c.ai.arrive_beat = 0.35;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 3, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    (s, p)
}

#[test]
fn two_pirates_on_one_hero_never_wind_up_at_once() {
    // Corey, Nov 7: "two pirates are spam attacking so quick and together that you don't really
    // feel like you have a gap", "both doing the same animation in different frames".
    let (mut s, p) = turns_world();
    let def = s.content.mob("dm_pirate").unwrap();
    let a = s.spawn_mob(def, [1.8, 0.6, 0.0], None, false);
    let b = s.spawn_mob(def, [1.8, -0.6, 0.0], None, false);
    for m in [a, b] {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Chase { foe: Foe::Hero(p) };
        x.ready_at = 0.0;
    }
    let mut both = 0;
    let mut windups = 0;
    let mut was = (false, false);
    for _ in 0..400 {
        // The hero stands, full health, where they are.
        if let Some(h) = s.players.get_mut(&p) {
            h.health = h.max_health;
        }
        let _ = s.step(0.02);
        let w = |s: &Survival, m: Id| matches!(s.mobs[&m].state, MobState::Windup { .. });
        let now = (w(&s, a), w(&s, b));
        if now.0 && now.1 {
            both += 1;
        }
        windups += usize::from(now.0 && !was.0) + usize::from(now.1 && !was.1);
        was = now;
    }
    assert!(windups >= 4, "they did swing: {windups}");
    assert_eq!(both, 0, "never both winding up at once");
}

#[test]
fn a_creature_that_just_walked_up_settles_before_its_first_blow() {
    let (mut s, p) = turns_world();
    // A miner: strings up close only (a pirate lunges in from afar, a tell of its own).
    let def = s.content.mob("dm_miner").unwrap();
    assert!(s.content.mobs[def].patterns.iter().all(|p| p.min_range < 3.0));
    let m = s.spawn_mob(def, [12.0, 0.0, 0.0], None, false);
    let x = s.mobs.get_mut(&m).unwrap();
    x.state = MobState::Chase { foe: Foe::Hero(p) };
    x.ready_at = 0.0;
    // Until its first wind-up: when it last walked, and when it began.
    let mut began = None;
    for _ in 0..300 {
        let _ = s.step(0.02);
        if matches!(s.mobs[&m].state, MobState::Windup { .. }) {
            began = Some(s.clock);
            break;
        }
    }
    let began = began.expect("it swung");
    let moved = s.mobs[&m].moved_at;
    assert!(began - moved >= 0.35 - 0.021, "settled {:.2} s after walking up", began - moved);
}

#[test]
#[ignore]
fn fair_blows_report() {
    let c = Content::builtin();
    println!("reaction {REACTION} s; the sword's light swing locks {:.2} s; free {:.2} s after a timed dodge", sword_lock(&c), free_after_dodge(&c));
    println!("{:<22} {:<20} {:>2} {:>6} {:>6} {:>6} {:>5} {:>4}  verdicts", "creature", "string", "#", "windup", "fast", "gap", "block", "dmg");
    let mut bad = (0, 0, 0);
    for b in blows(&c) {
        let (ready, swinging, follow) = verdict(&c, &b);
        bad.0 += usize::from(!ready);
        bad.1 += usize::from(!swinging);
        bad.2 += usize::from(!follow);
        let mut why = Vec::new();
        if !ready {
            why.push("TOO FAST to answer at all");
        } else if !swinging {
            why.push("catches a swing");
        }
        if !follow {
            why.push("follow-up too soon to dodge");
        }
        println!(
            "{:<22} {:<20} {:>2} {:>6.2} {:>6.2} {:>6} {:>5} {:>4}  {}",
            b.mob,
            b.string.chars().take(20).collect::<String>(),
            b.step + 1,
            b.windup,
            b.windup_fast,
            b.gap.map_or("-".to_string(), |g| format!("{g:.2}")),
            if b.ranged { "shot" } else if b.blockable { "yes" } else { "NO" },
            b.damage,
            if why.is_empty() { "ok".to_string() } else { why.join("; ") }
        );
    }
    println!("too fast to answer at all: {}; catch a light swing: {}; follow-ups too soon: {}", bad.0, bad.1, bad.2);
}
