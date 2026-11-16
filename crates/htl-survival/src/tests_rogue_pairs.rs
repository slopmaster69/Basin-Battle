//! The Rogue's dual-wield pairs and the fist weapons (docs/rogue-roles.md, build plan entry 5):
//! Twin Fangs, Blade and Shiv, Crossed Swords, Breaker's Pair and Iron Knuckles, each one rule on
//! the two-blade finisher, judged on both hands' families. Each weapon keeps its own mechanic in
//! its own hand; the fists' own are in `tests_weapon_identity.rs`. A Rogue's fist weapon is one
//! item that fills both hands, always Iron Knuckles (Corey, Nov 16).

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::rogue::Pair;
use crate::sim::*;

const DT: f32 = 0.02;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
const PI: f32 = std::f32::consts::PI;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    // (Each blow at the top of its range: these tests are about the rules, not the dice.)
    for w in c.weapons.iter_mut() {
        w.damage = [w.damage[1], w.damage[1]];
    }
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// A hero of this class, made as the creation screen makes one, at the origin facing +x.
fn hero_of(s: &mut Survival, name: &str, class: &str) -> Id {
    let account = name.to_lowercase();
    let save = s.fresh_save(&account, name, class, HeroLook::default());
    let id = s.join(&account, name, Some(&save));
    s.move_player(id, ORIGIN, 0.0, Moving::default());
    let _ = s.step(0.0);
    id
}

/// These two weapons in a hero's hands (main, off).
fn hands(s: &mut Survival, p: Id, main: &str, off: &str) {
    let (m, o) = (s.content.item(main).unwrap(), s.content.item(off).unwrap());
    let h = s.players.get_mut(&p).unwrap();
    h.weapon = m;
    h.off_weapon = Some(o);
    h.off_empty = false;
}

/// A Rogue holding these two.
fn rogue_with(s: &mut Survival, main: &str, off: &str) -> Id {
    let p = hero_of(s, "Ayla", "rogue");
    hands(s, p, main, off);
    p
}

/// A creature at `at` facing `f`, at rest, that never strikes on its own.
fn mob(s: &mut Survival, key: &str, at: Vec3, f: f32) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, at, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = at;
    m.facing = f;
    m.ready_at = 1e9;
    m.health = 100_000;
    m.max_health = 100_000;
    id
}

/// Hold a creature where it is, facing `f`, at rest (struck, it would turn on the hero).
fn hold(s: &mut Survival, m: Id, f: f32) {
    let m = s.mobs.get_mut(&m).unwrap();
    m.facing = f;
    m.state = MobState::Idle { wander_at: 1e9 };
    m.path.clear();
    m.sidle = None;
    m.ready_at = 1e9;
}

fn strike(s: &Survival, p: Id) -> Option<Strike> {
    match &s.players[&p].action {
        Action::Swing { strike, .. } => Some(strike.clone()),
        _ => None,
    }
}

fn hits_on(ev: &[Event], v: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(x), damage, outcome: Outcome::Hit, style, .. } if *x == v && !matches!(style, Blow::Dot) => Some(*damage),
            _ => None,
        })
        .collect()
}

fn beat(ev: &[Event], p: Id, b: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Beat { player, beat, .. } if *player == p && beat == b))
}

/// One blow of the run (the off hand's on `off`), pressed late in the last one's recovery as a
/// run is, then run on to late in its own; `each` before every tick (holding the creatures). The
/// blow as it started, and the events.
fn hand(s: &mut Survival, p: Id, off: bool, each: &mut dyn FnMut(&mut Survival)) -> (Strike, Vec<Event>) {
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    s.input(p, if off { Input::Guard } else { Input::Attack });
    let st = strike(s, p).expect("a blow");
    let mut ev = Vec::new();
    for _ in 0..80 {
        each(s);
        ev.extend(s.step(DT));
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
            break;
        }
    }
    (st, ev)
}

/// A run of `n` blows, main hand first, alternating; each blow and its events.
fn run_of(s: &mut Survival, p: Id, n: usize, each: &mut dyn FnMut(&mut Survival)) -> Vec<(Strike, Vec<Event>)> {
    (0..n).map(|k| hand(s, p, k % 2 == 1, each)).collect()
}

/// The finisher's damage on a creature held facing `f`, its memory of backstabs cleared just
/// before (so the first three blows' stabs don't take from it).
fn finisher_on(s: &mut Survival, p: Id, m: Id, f: f32, last_off: bool) -> (Strike, u32) {
    let mut keep = |s: &mut Survival| hold(s, m, f);
    // Three blows, ending on the hand before the last.
    for k in 0..3 {
        // (Alternating, so the 4th is `last_off`'s hand.)
        let off = if k % 2 == 1 { last_off } else { !last_off };
        hand(s, p, off, &mut keep);
    }
    s.mobs.get_mut(&m).unwrap().backstabs.clear();
    let (st, ev) = hand(s, p, last_off, &mut keep);
    assert!(st.finish, "the 4th is the finisher: {st:?}");
    (st, hits_on(&ev, m).into_iter().max().unwrap_or(0))
}

// --- which hands make a pair -------------------------------------------------------------------------

#[test]
fn a_pairs_rule_needs_both_hands() {
    let mut s = world();
    let p = hero_of(&mut s, "Ayla", "rogue");
    // A new Rogue: two Poacher's Knives.
    assert_eq!(s.pair_of(&s.players[&p]), Some(Pair::TwinFangs));
    // Either hand: a sword and a dagger, a dagger and a sword.
    for (main, off, want) in [
        ("sword", "dagger", Some(Pair::BladeAndShiv)),
        ("dagger", "sword", Some(Pair::BladeAndShiv)),
        ("sword", "silver_sword", Some(Pair::CrossedSwords)),
        ("mace", "dagger", Some(Pair::BreakersPair)),
        ("bazil_shiv", "mace", Some(Pair::BreakersPair)),
        ("mace", "hogger_club", Some(Pair::BreakersPair)),
        ("mace", "sword", Some(Pair::BreakersPair)),
        ("footpad_knuckles", "footpad_knuckles", Some(Pair::IronKnuckles)),
        ("cutpurse_claw", "cutpurse_claw", Some(Pair::IronKnuckles)),
    ] {
        hands(&mut s, p, main, off);
        assert_eq!(s.pair_of(&s.players[&p]), want, "{main} and {off}");
    }
    // A dagger and an empty hand (it jabs): no pair, and the plain run of 4.
    let h = s.players.get_mut(&p).unwrap();
    h.weapon = s.content.item("dagger").unwrap();
    h.off_weapon = None;
    h.off_empty = true;
    assert_eq!(s.pair_of(&s.players[&p]), None);
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    let run = run_of(&mut s, p, 4, &mut |s| hold(s, m, PI));
    assert!(run[3].0.finish && run[3].0.pair.is_none(), "a finisher, no pair: {:?}", run[3].0);
    // Another class's two daggers or two fists: no pair, and a run of 4.
    let w = hero_of(&mut s, "Bren", "warrior");
    hands(&mut s, w, "dagger", "dagger");
    assert_eq!(s.pair_of(&s.players[&w]), None);
    hands(&mut s, w, "footpad_knuckles", "footpad_knuckles");
    assert_eq!(s.pair_of(&s.players[&w]), None);
    assert_eq!(s.run_length(&s.players[&w]), 4);
}

// --- Twin Fangs --------------------------------------------------------------------------------------

/// A perfect dodge of the creature's blow, then an attack: the Shadow Cut into its back. Its
/// damage, and how many backstabs the creature remembers after.
fn shadow_cut(main: &str, off: &str, remembered: usize) -> (u32, usize) {
    let mut s = world();
    let p = rogue_with(&mut s, main, off);
    let m = mob(&mut s, "grunt", [2.0, 0.0, 0.0], PI);
    let now = s.clock;
    s.mobs.get_mut(&m).unwrap().backstabs = vec![now; remembered];
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Dodge);
    for _ in 0..5 {
        let _ = s.step(DT);
    }
    assert_eq!(s.strike_player(m, p, 16, [2.0, 0.0, 0.0], true, false, true, Blow::Light), Outcome::Dodged);
    for _ in 0..15 {
        let _ = s.step(DT);
    }
    // It stands where it was, facing the Rogue's first spot, after its dodged blow.
    let until = s.clock + 1.0;
    let mm = s.mobs.get_mut(&m).unwrap();
    mm.state = MobState::Recover { foe: Foe::Hero(p), until };
    mm.pos = [2.0, 0.0, 0.0];
    mm.facing = PI;
    mm.path.clear();
    mm.sidle = None;
    s.events.clear();
    s.input(p, Input::Attack);
    assert!(strike(&s, p).is_some_and(|st| st.cut), "the Shadow Cut");
    let mut ev = std::mem::take(&mut s.events);
    for _ in 0..30 {
        ev.extend(s.step(DT));
    }
    assert!(beat(&ev, p, "backstab"), "in its back");
    (hits_on(&ev, m).into_iter().max().unwrap_or(0), s.mobs[&m].backstabs.len())
}

#[test]
fn twin_fangs_shadow_cut_adds_nothing_to_the_backstab_memory() {
    // Twin Fangs: the cut isn't remembered, and three stabs remembered take nothing from it.
    let (fresh, after) = shadow_cut("dagger", "dagger", 0);
    assert_eq!(after, 0, "not remembered");
    let (worn, after) = shadow_cut("dagger", "dagger", 3);
    assert_eq!(after, 3, "still only the three");
    assert_eq!(worn, fresh, "its backstab whole");
    // Any other pair's cut is remembered, and three before take from it.
    let (fresh, after) = shadow_cut("dagger", "sword", 0);
    assert_eq!(after, 1, "remembered");
    let (worn, _) = shadow_cut("dagger", "sword", 3);
    assert!(worn < fresh, "worn down: {worn} against {fresh}");
}

// --- Blade and Shiv ----------------------------------------------------------------------------------

#[test]
fn blade_and_shiv_finisher_reaches_3_yd_and_backstabs_for_2_5() {
    // Pressed by the sword hand, from behind: the dagger's 2.5, where two swords' is the rules' 1.5.
    let mut s = world();
    let p = rogue_with(&mut s, "sword", "dagger");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], 0.0);
    let (st, shiv) = finisher_on(&mut s, p, m, 0.0, false);
    assert_eq!(st.kind, StrikeKind::Light, "the sword's hand");
    let mut s = world();
    let p = rogue_with(&mut s, "sword", "sword");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], 0.0);
    let (_, swords) = finisher_on(&mut s, p, m, 0.0, false);
    let ratio = shiv as f32 / swords as f32;
    assert!((ratio - 2.5 / 1.5).abs() < 0.05, "2.5 against 1.5 from behind: {shiv} / {swords} = {ratio}");
    // Pressed by the dagger's hand, it has the sword's reach.
    let mut s = world();
    let p = rogue_with(&mut s, "sword", "dagger");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    let (st, _) = finisher_on(&mut s, p, m, PI, true);
    assert_eq!(st.kind, StrikeKind::Off, "the dagger's hand");
    let sword = s.content.weapon("sword").unwrap().range;
    assert!((st.range - sword).abs() < 1e-4 && sword >= 3.0, "the sword's {sword} yd: {}", st.range);
    // Twin Fangs' finisher keeps a dagger's reach.
    let mut s = world();
    let p = rogue_with(&mut s, "dagger", "dagger");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    let (st, _) = finisher_on(&mut s, p, m, PI, true);
    assert!(st.range < sword - 0.5, "a dagger's reach: {}", st.range);
}

#[test]
fn blade_and_shiv_finisher_lands_where_a_daggers_falls_short() {
    // A creature just past a dagger's reach and inside a sword's.
    for (main, off, lands) in [("sword", "dagger", true), ("dagger", "dagger", false)] {
        let mut s = world();
        let p = rogue_with(&mut s, main, off);
        let size = s.content.mobs[s.content.mob("dm_woodcutter").unwrap()].body();
        let dagger = s.content.weapon("dagger").unwrap().range;
        let at = [dagger + size + 0.3, 0.0, 0.0];
        let m = mob(&mut s, "dm_woodcutter", at, PI);
        let mut keep = |s: &mut Survival| {
            hold(s, m, PI);
            s.mobs.get_mut(&m).unwrap().pos = at;
        };
        let run = run_of(&mut s, p, 4, &mut keep);
        assert!(run[3].0.finish && run[3].0.kind == StrikeKind::Off, "the dagger's hand presses the finisher");
        assert_eq!(!hits_on(&run[3].1, m).is_empty(), lands, "{main} and {off}");
    }
}

// --- Crossed Swords ----------------------------------------------------------------------------------

#[test]
fn crossed_swords_finisher_strikes_every_enemy_in_150_degrees() {
    for (main, off, every) in [("sword", "sword", true), ("sword", "dagger", false)] {
        let mut s = world();
        let p = rogue_with(&mut s, main, off);
        let at = |deg: f32, d: f32| [d * deg.to_radians().cos(), d * deg.to_radians().sin(), 0.0];
        let ahead = mob(&mut s, "dm_woodcutter", at(0.0, 2.0), PI);
        let left = mob(&mut s, "dm_woodcutter", at(60.0, 2.4), PI);
        let right = mob(&mut s, "dm_woodcutter", at(-60.0, 2.4), PI);
        let wide = mob(&mut s, "dm_woodcutter", at(100.0, 2.4), PI);
        let all = [(ahead, at(0.0, 2.0)), (left, at(60.0, 2.4)), (right, at(-60.0, 2.4)), (wide, at(100.0, 2.4))];
        let mut keep = |s: &mut Survival| {
            for (m, pos) in all {
                hold(s, m, PI);
                s.mobs.get_mut(&m).unwrap().pos = pos;
            }
        };
        let run = run_of(&mut s, p, 4, &mut keep);
        for (st, ev) in &run[..3] {
            let struck = all.iter().filter(|(m, _)| !hits_on(ev, *m).is_empty()).count();
            assert!(!st.finish && struck == 1, "{main} and {off}: a plain blow strikes one ({struck})");
        }
        let (st, ev) = &run[3];
        assert!(st.finish);
        let struck: Vec<bool> = all.iter().map(|(m, _)| !hits_on(ev, *m).is_empty()).collect();
        if every {
            assert!((st.arc - 150.0).abs() < 1e-3, "a 150 degree cross-cut: {}", st.arc);
            assert_eq!(struck, vec![true, true, true, false], "every enemy in 150 degrees, none past it");
        } else {
            assert_eq!(struck.iter().filter(|x| **x).count(), 1, "{main} and {off}: one enemy");
        }
    }
}

// --- Breaker's Pair ----------------------------------------------------------------------------------

/// The posture one blow builds on a creature: its posture cleared every tick of the blow's
/// wind-up, so what it holds after is what the blow built. (The blow, its damage, what it built.)
fn posture_blow(s: &mut Survival, p: Id, m: Id, off: bool) -> (Strike, u32, u32) {
    let mut keep = |s: &mut Survival| {
        hold(s, m, PI);
        if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t < strike.windup) {
            let now = s.clock;
            let mm = s.mobs.get_mut(&m).unwrap();
            mm.posture = 0;
            mm.posture_from = now;
        }
    };
    let (st, ev) = hand(s, p, off, &mut keep);
    let dealt = hits_on(&ev, m).into_iter().max().expect("it lands");
    (st, dealt, s.mobs[&m].posture)
}

/// The share of a blow's damage it built as posture.
fn heft(dealt: u32, built: u32) -> f32 {
    built as f32 / dealt as f32
}

#[test]
fn breakers_pair_finisher_counts_3_times_toward_posture() {
    let mut s = world();
    let p = rogue_with(&mut s, "mace", "dagger");
    let m = mob(&mut s, "overseer", [2.0, 0.0, 0.0], PI);
    s.mobs.get_mut(&m).unwrap().max_health = 100_000;
    assert!(s.posture_limit(m) > 0, "a champion");
    // The mace's own blow counts twice; the dagger's once.
    let (_, dealt, built) = posture_blow(&mut s, p, m, false);
    assert!((heft(dealt, built) - 2.0).abs() < 0.05, "the mace's 2: {dealt} built {built}");
    let (_, dealt, built) = posture_blow(&mut s, p, m, true);
    assert!((heft(dealt, built) - 1.0).abs() < 0.05, "the dagger's 1: {dealt} built {built}");
    let _ = posture_blow(&mut s, p, m, false);
    // The finisher, pressed by the dagger's hand: 3.
    let (st, dealt, built) = posture_blow(&mut s, p, m, true);
    assert!(st.finish);
    assert!((heft(dealt, built) - 3.0).abs() < 0.05, "3 times: {dealt} built {built}");
    // Twin Fangs' finisher counts once.
    let mut s = world();
    let p = rogue_with(&mut s, "dagger", "dagger");
    let m = mob(&mut s, "overseer", [2.0, 0.0, 0.0], PI);
    for k in 0..3 {
        let _ = posture_blow(&mut s, p, m, k % 2 == 1);
    }
    let (st, dealt, built) = posture_blow(&mut s, p, m, true);
    assert!(st.finish);
    assert!((heft(dealt, built) - 1.0).abs() < 0.05, "once: {dealt} built {built}");
}

#[test]
fn a_warriors_two_hands_count_toward_posture_as_before() {
    // The main hand's weight for every blow, as ever: a dagger in the main hand, a mace in the off.
    let mut s = world();
    let w = hero_of(&mut s, "Bren", "warrior");
    hands(&mut s, w, "dagger", "mace");
    let m = mob(&mut s, "overseer", [2.0, 0.0, 0.0], PI);
    let (_, dealt, built) = posture_blow(&mut s, w, m, false);
    assert!((heft(dealt, built) - 1.0).abs() < 0.05);
    let (_, dealt, built) = posture_blow(&mut s, w, m, true);
    assert!((heft(dealt, built) - 1.0).abs() < 0.05, "the off-hand mace as the main dagger: {dealt} built {built}");
}

// --- Iron Knuckles -----------------------------------------------------------------------------------

#[test]
fn iron_knuckles_run_is_6_and_its_finisher_turns_a_non_boss_away() {
    let mut s = world();
    let p = rogue_with(&mut s, "cutpurse_claw", "cutpurse_claw");
    assert_eq!(s.run_length(&s.players[&p]), 6);
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    let wind = s.content.weapon("footpad_knuckles").unwrap().windup;
    assert!((wind - 0.15).abs() < 1e-4, "a fist's wind-up: {wind}");
    let run = run_of(&mut s, p, 5, &mut |s| hold(s, m, PI));
    assert!(run.iter().all(|(st, ev)| !st.finish && !hits_on(ev, m).is_empty()), "five blows land, none the finisher");
    // The 6th: both fists (AttackUnarmed).
    s.players.get_mut(&p).unwrap().stamina = 100.0;
    s.input(p, Input::Guard);
    let st = strike(&s, p).unwrap();
    assert!(st.finish && st.clip == 16, "the finisher, both fists: {st:?}");
    let mut ev = Vec::new();
    for _ in 0..20 {
        ev.extend(s.step(DT));
        if !hits_on(&ev, m).is_empty() {
            break;
        }
    }
    assert!(!hits_on(&ev, m).is_empty(), "it lands");
    // Its back to the Rogue: it faces away, and stays so.
    for _ in 0..10 {
        let _ = s.step(DT);
    }
    let (mpos, rpos) = (s.mobs[&m].pos, s.players[&p].pos);
    let away = facing(rpos, mpos);
    assert!(s.has_cc(m, "distract"), "wheeled");
    assert!(angle_between(s.mobs[&m].facing, away) < 20.0, "its back to her: faces {} where away is {away}", s.mobs[&m].facing);
    // The next blow is a backstab.
    let (_, ev) = hand(&mut s, p, false, &mut |_| {});
    assert!(beat(&ev, p, "backstab"), "into its back");
    // After 1 sec it turns back.
    for _ in 0..60 {
        let _ = s.step(DT);
    }
    assert!(!s.has_cc(m, "distract"), "it turns back after 1 sec");
    // Twin Fangs' finisher turns nobody.
    let mut s = world();
    let p = rogue_with(&mut s, "dagger", "dagger");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    let run = run_of(&mut s, p, 4, &mut |s| hold(s, m, PI));
    assert!(run[3].0.finish);
    assert!(!s.has_cc(m, "distract"));
}

#[test]
fn iron_knuckles_finisher_on_a_boss_counts_twice_toward_posture() {
    let mut s = world();
    let p = rogue_with(&mut s, "footpad_knuckles", "footpad_knuckles");
    let m = mob(&mut s, "goldtooth", [2.0, 0.0, 0.0], PI);
    assert!(s.content.mobs[s.mobs[&m].def].boss && s.posture_limit(m) > 0, "a boss with a stance");
    for k in 0..5 {
        let (st, dealt, built) = posture_blow(&mut s, p, m, k % 2 == 1);
        assert!(!st.finish);
        assert!((heft(dealt, built) - 1.0).abs() < 0.05, "a fist's blow, once: {dealt} built {built}");
    }
    let (st, dealt, built) = posture_blow(&mut s, p, m, true);
    assert!(st.finish);
    assert!((heft(dealt, built) - 2.0).abs() < 0.05, "twice: {dealt} built {built}");
    assert!(!s.has_cc(m, "distract"), "a boss is never wheeled");
}

#[test]
fn blade_flurry_and_a_darting_knife_follow_the_knuckles_longer_run() {
    // Blade Flurry's 4 cuts still end in the finisher; a knife carries the run to the 5th at most.
    let mut s = world();
    let p = rogue_with(&mut s, "footpad_knuckles", "footpad_knuckles");
    s.players.get_mut(&p).unwrap().dual_run = 4;
    s.players.get_mut(&p).unwrap().combo_until = f32::INFINITY;
    s.knife_hand(p);
    assert_eq!(s.players[&p].dual_run, 5, "the 5th, never the 6th");
    s.knife_hand(p);
    assert_eq!(s.players[&p].dual_run, 5);
}

// --- each weapon its own, in either hand -------------------------------------------------------------

#[test]
fn each_weapons_own_mechanic_still_works_in_either_hand() {
    // Bazil's Shiv poisons from either hand; a Poacher's Knife beside it doesn't.
    for off in [false, true] {
        let mut s = world();
        let (main, second) = if off { ("dagger", "bazil_shiv") } else { ("bazil_shiv", "dagger") };
        let p = rogue_with(&mut s, main, second);
        let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
        let (_, ev) = hand(&mut s, p, off, &mut |s| hold(s, m, PI));
        assert!(!hits_on(&ev, m).is_empty());
        assert!(s.has_cc(m, "poison"), "the Shiv's poison from the {} hand", if off { "off" } else { "main" });
        let (_, _) = hand(&mut s, p, !off, &mut |s| hold(s, m, PI));
        let stacks = s.cc_of(m).iter().find(|c| c.kind == "poison").map_or(0.0, |c| c.amount);
        let one = s.content.weapon("bazil_shiv").unwrap().poison;
        assert!((stacks - one).abs() < 1e-3, "the knife's hand lays none: {stacks}");
    }
    // A Silver-Edged Longsword's bane from the off hand of Crossed Swords.
    let mut s = world();
    let p = rogue_with(&mut s, "sword", "silver_sword");
    let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
    s.content.mobs[s.mobs[&m].def].tags = vec!["undead".into()];
    let (_, ev) = hand(&mut s, p, true, &mut |s| hold(s, m, PI));
    assert!(!hits_on(&ev, m).is_empty());
    assert!(s.silvered(m), "silvered by the off hand");
}

// --- words and the rack ------------------------------------------------------------------------------

#[test]
fn each_pairs_words_say_what_its_data_does() {
    let mut s = world();
    let p = hero_of(&mut s, "Ayla", "rogue");
    let r = s.content.rogue.clone();
    let says = |s: &Survival| s.pair_words(&s.players[&p]).unwrap_or_default();
    assert!(says(&s).starts_with("Your pair is Twin Fangs.") && says(&s).contains("full damage from behind"), "{}", says(&s));
    hands(&mut s, p, "sword", "dagger");
    let (reach, stab) = (s.content.weapon("sword").unwrap().range, s.content.weapon("dagger").unwrap().backstab);
    assert!(says(&s).contains(&format!("reaches {reach} yd")) && says(&s).contains(&format!("{}% damage from behind", (stab * 100.0).round())), "{}", says(&s));
    hands(&mut s, p, "sword", "sword");
    assert!(says(&s).contains(&format!("within {} degrees", r.cross_arc)), "{}", says(&s));
    hands(&mut s, p, "mace", "sword");
    assert!(says(&s).contains(&format!("breaks stances {}", crate::content::rate_words(r.breaker_heft))), "{}", says(&s));
    hands(&mut s, p, "footpad_knuckles", "footpad_knuckles");
    let w = says(&s);
    assert!(w.starts_with("Your pair is Iron Knuckles.") && w.contains(&format!("the {}th is with both", r.knuckle_run)) && w.contains(&format!("for {} sec", r.wheel_secs)), "{w}");
    assert!(w.contains(&format!("breaks stances {} instead", crate::content::rate_words(r.knuckle_boss_heft))), "{w}");
    // No pair (an empty off hand), no words; nor for a Warrior.
    let h = s.players.get_mut(&p).unwrap();
    h.off_weapon = None;
    h.off_empty = true;
    assert_eq!(s.pair_words(&s.players[&p]), None);
    let b = hero_of(&mut s, "Bren", "warrior");
    hands(&mut s, b, "dagger", "dagger");
    assert_eq!(s.pair_words(&s.players[&b]), None);
}

// --- a fist weapon is one item in both hands (Corey, Nov 16) ----------------------------------------

/// A hero of this class at the weapon rack, the armory open.
fn at_rack(s: &mut Survival, name: &str, class: &str) -> Id {
    s.content.world.armory = true;
    let p = hero_of(s, name, class);
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    p
}

/// What a hero's hands hold: (main hand, off hand's weapon, off hand left empty).
fn held(s: &Survival, p: Id) -> (String, Option<String>, bool) {
    let h = &s.players[&p];
    (s.content.items[h.weapon].key.clone(), h.off_weapon.map(|o| s.content.items[o].key.clone()), h.off_empty)
}

/// One fist weapon in both hands.
fn both(k: &str) -> (String, Option<String>, bool) {
    (k.to_string(), Some(k.to_string()), false)
}

fn bag_keys(s: &Survival, p: Id) -> std::collections::BTreeMap<String, u32> {
    s.players[&p].bag.iter().map(|(i, n)| (s.content.items[*i].key.clone(), *n)).collect()
}

#[test]
fn a_rogues_fist_weapon_fills_both_hands_from_the_rack() {
    let mut s = world();
    let p = at_rack(&mut s, "Ayla", "rogue");
    // The rack says so, in the row's kind and first in its words.
    let view = s.rack_view(p);
    for k in ["footpad_knuckles", "cutpurse_claw"] {
        let row = view.iter().find(|r| r.0 == k).unwrap_or_else(|| panic!("the rack offers {k}"));
        assert_eq!(row.2, "both hands", "{k}");
        assert!(row.4.starts_with("Equipped in both hands. "), "{}", row.4);
        let w = s.content.weapon(k).unwrap();
        assert!(w.subclass == crate::sim::rogue::FIST && !w.two_handed && w.moveset_key() == "fist", "{k} is a fist weapon");
    }
    // Either button: both hands, Iron Knuckles.
    s.arm(p, "footpad_knuckles").unwrap();
    assert_eq!(held(&s, p), both("footpad_knuckles"));
    assert_eq!(s.pair_of(&s.players[&p]), Some(Pair::IronKnuckles));
    assert!(s.fists_of(&s.players[&p]));
    assert!(s.rack_view(p).iter().any(|r| r.0 == "footpad_knuckles" && r.3), "shown as equipped");
    s.arm_off(p, "cutpurse_claw").unwrap();
    assert_eq!(held(&s, p), both("cutpurse_claw"));
    assert_eq!(s.run_length(&s.players[&p]), s.content.rogue.knuckle_run);
    // The off hand can't be emptied alone.
    assert_eq!(s.stow_off(p), Err("Fist weapons take both hands already.".to_string()));
    assert_eq!(held(&s, p), both("cutpurse_claw"));
}

#[test]
fn a_blade_in_either_hand_takes_the_fists_off() {
    let mut s = world();
    let p = at_rack(&mut s, "Ayla", "rogue");
    // In the main hand: the off hand is left empty (it jabs).
    s.arm(p, "footpad_knuckles").unwrap();
    s.arm(p, "sword").unwrap();
    assert_eq!(held(&s, p), ("sword".to_string(), None, true));
    assert_eq!(s.pair_of(&s.players[&p]), None);
    // In the off hand: the main hand takes the Rogue's start blade (it's never empty).
    s.arm(p, "cutpurse_claw").unwrap();
    s.arm_off(p, "mace").unwrap();
    let knife = s.content.rogue.weapon.clone();
    assert_eq!(held(&s, p), (knife, Some("mace".to_string()), false));
    assert_eq!(s.pair_of(&s.players[&p]), Some(Pair::BreakersPair));
    // Fists over two blades take both hands.
    s.arm(p, "sword").unwrap();
    assert_eq!(held(&s, p), ("sword".to_string(), Some("mace".to_string()), false));
    s.arm_off(p, "footpad_knuckles").unwrap();
    assert_eq!(held(&s, p), both("footpad_knuckles"));
}

#[test]
fn fists_from_the_bags_or_storage_take_both_hands_and_put_both_blades_away() {
    let mut s = world();
    let p = hero_of(&mut s, "Ayla", "rogue");
    let (fist, dagger) = (s.content.item("footpad_knuckles").unwrap(), s.content.item("dagger").unwrap());
    // From the bags: the sword and the dagger both go to the bags.
    hands(&mut s, p, "sword", "dagger");
    s.players.get_mut(&p).unwrap().bag = [(fist, 1)].into();
    let said = s.wear_from_bag(p, fist).unwrap();
    assert_eq!(held(&s, p), both("footpad_knuckles"));
    assert!(said.contains("in both hands") && said.contains(" and "), "{said}");
    assert_eq!(bag_keys(&s, p), [("sword".to_string(), 1), ("dagger".to_string(), 1)].into());
    // A blade from the bags over them: the fists go to the bags once, the off hand is empty.
    s.wear_from_bag(p, dagger).unwrap();
    assert_eq!(held(&s, p), ("dagger".to_string(), None, true));
    assert_eq!(bag_keys(&s, p), [("sword".to_string(), 1), ("footpad_knuckles".to_string(), 1)].into());
    // Bags with room for one but not two: refused, the hands as they were.
    hands(&mut s, p, "sword", "dagger");
    let ore = s.content.item("ore").unwrap();
    let room = s.content.player.bag_slots as u32 - 1;
    s.players.get_mut(&p).unwrap().bag = [(fist, 1), (ore, room * s.stack_for(ore, None))].into();
    assert!(s.wear_from_bag(p, fist).is_err());
    assert_eq!(held(&s, p), ("sword".to_string(), Some("dagger".to_string()), false));
    // From storage: both blades go to storage.
    s.players.get_mut(&p).unwrap().bag.clear();
    s.storage.insert(fist, 1);
    let chest = s.objs[&s.storage_id()].pos;
    s.players.get_mut(&p).unwrap().pos = chest;
    s.equip_from_chest(p, "footpad_knuckles").unwrap();
    assert_eq!(held(&s, p), both("footpad_knuckles"));
    let (sword, fist_left) = (s.storage.get(&s.content.item("sword").unwrap()).copied(), s.storage.get(&fist).copied().unwrap_or(0));
    assert_eq!((sword, s.storage.get(&dagger).copied(), fist_left), (Some(1), Some(1), 0));
}

#[test]
fn a_fist_weapons_own_mechanic_is_in_both_hands() {
    for off in [false, true] {
        // The knuckles' breath back.
        let mut s = world();
        let p = rogue_with(&mut s, "footpad_knuckles", "footpad_knuckles");
        let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
        if off {
            let _ = hand(&mut s, p, false, &mut |s| hold(s, m, PI));
        }
        s.players.get_mut(&p).unwrap().stamina = 50.0;
        s.input(p, if off { Input::Guard } else { Input::Attack });
        let cost = strike(&s, p).unwrap().stamina;
        for _ in 0..20 {
            hold(&mut s, m, PI);
            let _ = s.step(DT);
        }
        let wind = s.content.weapon("footpad_knuckles").unwrap().wind;
        let have = s.players[&p].stamina;
        assert!(wind > 0.0 && have > 50.0 - cost + wind - 1.0, "{} hand: {wind} back after the {cost} the blow cost: {have}", if off { "off" } else { "main" });
        // The claw's counter, just after a dodge slipped a blow.
        let blow = |slipped: bool| -> u32 {
            let mut s = world();
            let p = rogue_with(&mut s, "cutpurse_claw", "cutpurse_claw");
            let m = mob(&mut s, "dm_woodcutter", [2.0, 0.0, 0.0], PI);
            if off {
                let _ = hand(&mut s, p, false, &mut |s| hold(s, m, PI));
            }
            let now = s.clock;
            s.players.get_mut(&p).unwrap().fx.slipped_at = slipped.then_some(now);
            let (st, ev) = hand(&mut s, p, off, &mut |s| hold(s, m, PI));
            assert!(!st.finish && st.pair == Some(Pair::IronKnuckles));
            hits_on(&ev, m).into_iter().max().unwrap_or(0)
        };
        let (plain, slipped) = (blow(false), blow(true));
        let slip = s.content.weapon("cutpurse_claw").unwrap().slip;
        let ratio = slipped as f32 / plain.max(1) as f32;
        assert!(plain > 0 && (ratio - (1.0 + slip)).abs() < 0.08, "{} hand: {slip} more: {slipped} / {plain}", if off { "off" } else { "main" });
    }
}

#[test]
fn anyone_but_a_rogue_holds_a_fist_weapon_as_a_one_hander_as_before() {
    let mut s = world();
    let w = at_rack(&mut s, "Bren", "warrior");
    let row = s.rack_view(w).into_iter().find(|r| r.0 == "footpad_knuckles").unwrap();
    assert_eq!(row.2, "one-handed");
    assert!(!row.4.contains("both hands"), "{}", row.4);
    // The main hand, the shield kept.
    let shield = s.players[&w].shield;
    s.arm(w, "footpad_knuckles").unwrap();
    let h = &s.players[&w];
    assert_eq!(s.content.items[h.weapon].key, "footpad_knuckles");
    assert!(h.off_weapon.is_none() && !h.off_empty && h.shield == shield, "the shield stays");
    // A fist in the off hand, and a blade beside it: one hand each, no pair.
    s.arm_off(w, "cutpurse_claw").unwrap();
    assert_eq!(held(&s, w), ("footpad_knuckles".to_string(), Some("cutpurse_claw".to_string()), false));
    s.arm(w, "dagger").unwrap();
    assert_eq!(held(&s, w), ("dagger".to_string(), Some("cutpurse_claw".to_string()), false));
    assert_eq!(s.pair_of(&s.players[&w]), None);
    assert!(!s.fists_of(&s.players[&w]));
    assert!(s.stow_off(w).is_ok());
    // From the bags, the main hand only.
    hands(&mut s, w, "sword", "dagger");
    let fist = s.content.item("footpad_knuckles").unwrap();
    s.players.get_mut(&w).unwrap().bag = [(fist, 1)].into();
    s.wear_from_bag(w, fist).unwrap();
    assert_eq!(held(&s, w), ("footpad_knuckles".to_string(), Some("dagger".to_string()), false));
    assert_eq!(bag_keys(&s, w), [("sword".to_string(), 1)].into());
}
