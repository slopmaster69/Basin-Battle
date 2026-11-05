//! The rhythm check: a spell that goes off on beats. The first few go by themselves; each further
//! one only if the key is pressed within its window of the beat, up to its most. A press out of
//! time, or a beat let pass, ends it where it is. Each beat harder; the last the finisher; a full
//! chain a perfect. Lag: a press is judged at its arrival less the presser's round trip.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.01;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().cd_off = false;
    id
}

fn dummy(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("training_dummy").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().state = MobState::Idle { wander_at: 1e9 };
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn press(s: &mut Survival, p: Id, slot: &str) {
    s.input(p, Input::Ability(key_of(slot), false));
}

/// The ability slotted and begun (its key, and the click if it aims).
/// Begin the rhythm, then make its schedule a plain steady beat with no rest, so a test can count
/// on `beat` apart (each cast rolls its own pattern; `every_cast_rolls...` tests that).
fn begin(s: &mut Survival, p: Id, slot: &str, key: &str) -> Vec<Event> {
    let ev = begin_rolled(s, p, slot, key);
    let beat = s.content.abilities[s.content.ability(key).unwrap()].beat_secs;
    for (k, b) in s.players.get_mut(&p).unwrap().rhythm_beats.iter_mut().enumerate() {
        *b = (beat * k as f32, false);
    }
    ev
}

/// Begin the rhythm as a player would, its pattern as rolled.
fn begin_rolled(s: &mut Survival, p: Id, slot: &str, key: &str) -> Vec<Event> {
    s.slot_ability(p, slot, key).unwrap();
    press(s, p, slot);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        s.input(p, Input::AttackUp);
    }
    s.step(0.0)
}

fn missiles(events: &[Event], caster: Id, spell: u32) -> usize {
    events.iter().filter(|e| matches!(e, Event::Missile { caster: c, spell: sp, .. } if *c == caster && *sp == spell)).count()
}

fn blows_on(events: &[Event], victim: Id) -> Vec<u32> {
    events
        .iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

fn perfect_in(events: &[Event], p: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Perfect { player, .. } if *player == p))
}

/// Run on to `t` seconds after `since`.
fn run_to(s: &mut Survival, since: f32, t: f32) -> Vec<Event> {
    let left = since + t - s.clock;
    if left > 0.0 { run(s, left) } else { Vec::new() }
}

#[test]
fn arcane_missiles_fires_its_free_three_on_the_beat_then_ends_unpressed() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    let mut ev = begin(&mut s, p, "ranged", "arcane_missiles");
    assert_eq!(missiles(&ev, p, d.spell), 1, "the first goes at once");
    assert_eq!(s.players[&p].action.name(), "channel");
    ev.extend(run(&mut s, d.beat_secs * 2.0 + 0.02));
    assert_eq!(missiles(&ev, p, d.spell), 3, "then two more on the beat");
    // Nobody presses: the fourth beat's window passes, and it ends.
    ev.extend(run(&mut s, d.beat_secs + 0.3));
    assert_eq!(missiles(&ev, p, d.spell), 3, "no fourth unpressed");
    assert_eq!(s.players[&p].action, Action::Ready, "over");
    ev.extend(run(&mut s, 1.5));
    let hits = blows_on(&ev, k);
    assert_eq!(hits, vec![5, 6, 7], "each a point harder than the last: {hits:?}");
    assert!(!perfect_in(&ev, p));
    // Every beat pulsed on the caster, the awaited fourth too.
    let pulses = ev.iter().filter(|e| matches!(e, Event::Visual { id, kit, .. } if *id == p && *kit == d.beat_kit)).count();
    assert_eq!(pulses, 4, "a pulse on each beat, the one to press on included");
}

#[test]
fn pressing_on_every_beat_runs_the_whole_volley_to_the_finisher_and_a_full_chain_is_a_perfect() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    let valor = s.players[&p].valor;
    let since = s.clock;
    let mut ev = begin(&mut s, p, "ranged", "arcane_missiles");
    // Run to each pressable beat and press right on it.
    for b in d.free_beats..d.most_beats {
        ev.extend(run_to(&mut s, since, d.beat_secs * b as f32));
        press(&mut s, p, "ranged");
        ev.extend(s.step(0.0));
    }
    ev.extend(run(&mut s, 1.5));
    assert_eq!(missiles(&ev, p, d.spell), d.most_beats as usize, "all eight");
    let hits = blows_on(&ev, k);
    assert_eq!(hits.len(), 8, "{hits:?}");
    assert_eq!(*hits.last().unwrap(), 24, "the eighth, the finisher, double its 12: {hits:?}");
    assert!(perfect_in(&ev, p), "a full chain is a perfect");
    assert!(s.players[&p].valor >= valor + s.content.checks.chain_valor, "a whole segment: {} from {valor}", s.players[&p].valor);
    assert_eq!(s.players[&p].action, Action::Ready);
}

#[test]
fn a_press_out_of_time_ends_the_volley_where_it_is_and_never_hurts() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    let hp = s.players[&p].health;
    let mut ev = begin(&mut s, p, "ranged", "arcane_missiles");
    // Halfway between the second and third beats: out of time.
    ev.extend(run(&mut s, d.beat_secs * 1.5));
    press(&mut s, p, "ranged");
    ev.extend(s.step(0.0));
    assert_eq!(s.players[&p].action, Action::Ready, "a press out of time ends it");
    ev.extend(run(&mut s, 1.0));
    assert_eq!(missiles(&ev, p, d.spell), 2, "what went, went");
    assert_eq!(s.players[&p].health, hp, "no harm to the caster");
    assert!(!perfect_in(&ev, p));
}

#[test]
fn the_window_tightens_a_press_that_counts_on_the_fourth_beat_misses_on_the_eighth() {
    let s = world();
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    assert_eq!(d.beat_window, [0.15, 0.08]);
    // 0.12 s off: inside the first window, outside the last.
    for (beat, off, holds) in [(3u32, 0.12f32, true), (7, 0.12, false), (7, 0.07, true)] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
        let since = s.clock;
        begin(&mut s, p, "ranged", "arcane_missiles");
        // Press on every beat before it, then this one `off` late.
        for b in d.free_beats..beat {
            run_to(&mut s, since, d.beat_secs * b as f32);
            press(&mut s, p, "ranged");
            s.step(0.0);
        }
        run_to(&mut s, since, d.beat_secs * beat as f32 + off);
        // (Too late for its window, the rhythm may already be over when the press comes.)
        press(&mut s, p, "ranged");
        let ev = s.step(0.0);
        // Counted: the rhythm goes on past it, or (on the last beat) the chain is whole.
        let counts = matches!(s.players[&p].action, Action::Rhythm { pressed, .. } if pressed > beat) || perfect_in(&ev, p);
        assert_eq!(counts, holds, "beat {beat}, {off} s late: counts {holds}");
    }
}

#[test]
fn a_late_press_from_a_far_hero_counts_as_they_made_it() {
    // 180 ms round trip (Europe to North America): the beat they saw came 90 ms late, and their
    // press reaches the server 90 ms after they made it.
    let mut s = world();
    let p = hero(&mut s, "Chain", ORIGIN, 0.0);
    s.set_rtt(p, 0.18);
    let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    let since = s.clock;
    begin(&mut s, p, "ranged", "arcane_missiles");
    // Pressed right on the fourth beat as they saw it: it arrives a round trip after the beat,
    // past the window as the server counts it.
    run_to(&mut s, since, d.beat_secs * 3.0 + 0.18);
    press(&mut s, p, "ranged");
    s.step(0.0);
    assert!(matches!(s.players[&p].action, Action::Rhythm { pressed, .. } if pressed == 4), "on time for them: {:?}", s.players[&p].action.name());
}

#[test]
fn arcane_explosion_bursts_then_twice_more_on_the_beat_wider_and_harder_the_third_throwing_them_back() {
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let d = s.content.abilities[s.content.ability("arcane_explosion").unwrap()].clone();
    let near = dummy(&mut s, [4.0, 0.0, 0.0]);
    let edge = dummy(&mut s, [7.5, 0.0, 0.0]);
    let since = s.clock;
    let mut ev = begin(&mut s, p, "close", "arcane_explosion");
    assert_eq!(blows_on(&ev, near), vec![10], "the first burst at once, 6 yd");
    assert!(blows_on(&ev, edge).is_empty(), "7.5 yd: outside the first");
    for b in 1..d.most_beats {
        ev.extend(run_to(&mut s, since, d.beat_secs * b as f32));
        press(&mut s, p, "close");
        ev.extend(s.step(0.0));
    }
    ev.extend(run(&mut s, 0.3));
    assert_eq!(blows_on(&ev, near), vec![10, 14, 18], "each harder");
    assert_eq!(blows_on(&ev, edge), vec![14, 18], "and wider: the second reaches 7 yd and more");
    // The third throws them back (a dummy is thrown a step at most, but it's thrown).
    assert!(s.cc_log_of(p).iter().any(|l| l.contains("knockback on")), "the third throws them back: {:?}", s.cc_log_of(p));
    assert!(perfect_in(&ev, p), "the full chain");
}

#[test]
fn the_arcane_spells_are_the_mages_second_choices_taught_at_its_shrine() {
    let s = world();
    let c = &s.content;
    for (key, slot) in [("arcane_missiles", "ranged"), ("arcane_explosion", "close")] {
        let d = &c.abilities[c.ability(key).unwrap()];
        assert_eq!((d.school.as_str(), d.slot.as_str(), d.choice, d.kind.as_str()), ("mage", slot, 2, "rhythm"), "{key}");
        assert!(d.on_board(), "{key} on the board");
    }
}

#[test]
fn every_cast_rolls_its_own_beat_sent_ahead_a_missile_leaves_on_each_press_and_a_rest_must_be_let_pass() {
    // The schedule is rolled and sent as the rhythm starts.
    let mut patterns = std::collections::BTreeSet::new();
    let mut rests = 0;
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
    s.set_cooldowns(p, true);
    for _ in 0..24 {
        // Each cast left unpressed: its free beats go and it ends.
        run(&mut s, 3.0);
        let ev = begin_rolled(&mut s, p, "ranged", "arcane_missiles");
        let sent = ev.iter().find_map(|e| match e {
            Event::CheckStart { player, kind: "rhythm", pattern, .. } if *player == p => Some(pattern.clone()),
            _ => None,
        });
        let sent = sent.expect("the schedule goes to the caster");
        let parts: Vec<&str> = sent.split('|').collect();
        assert_eq!(parts.len(), 3, "{sent}");
        patterns.insert(parts[1].to_string());
        assert_eq!(parts[2].split(',').count(), s.players[&p].rhythm_beats.len());
        rests += s.players[&p].rhythm_beats.iter().filter(|b| b.1).count();
    }
    assert!(patterns.len() >= 2, "more than one pattern rolled: {patterns:?}");
    assert!(rests > 0, "a long chain holds a rest some of the time");
    // A press in time fires its missile at once, not at the beat.
    let mut s = world();
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
    let d = s.content.abilities[s.content.ability("arcane_missiles").unwrap()].clone();
    let since = s.clock;
    begin(&mut s, p, "ranged", "arcane_missiles");
    run_to(&mut s, since, d.beat_secs * 3.0 - 0.06);
    press(&mut s, p, "ranged");
    let ev = s.step(0.0);
    assert_eq!(missiles(&ev, p, d.spell), 1, "the fourth leaves on the press, a little early");
    assert!(ev.iter().any(|e| matches!(e, Event::CheckResult { grade: "good" | "perfect", .. })));
    // A rest: pressing on it breaks the chain; letting it pass keeps it.
    for press_rest in [true, false] {
        let mut s = world();
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        let _k = dummy(&mut s, [12.0, 0.0, 0.0]);
        let since = s.clock;
        begin(&mut s, p, "ranged", "arcane_missiles");
        s.players.get_mut(&p).unwrap().rhythm_beats[4].1 = true;
        run_to(&mut s, since, d.beat_secs * 3.0);
        press(&mut s, p, "ranged");
        s.step(0.0);
        run_to(&mut s, since, d.beat_secs * 4.0);
        if press_rest {
            press(&mut s, p, "ranged");
            let ev = s.step(0.0);
            assert!(ev.iter().any(|e| matches!(e, Event::CheckResult { grade: "fail", .. })), "pressed on the rest");
            assert_eq!(s.players[&p].action, Action::Ready);
        } else {
            run_to(&mut s, since, d.beat_secs * 5.0);
            press(&mut s, p, "ranged");
            s.step(0.0);
            assert!(matches!(s.players[&p].action, Action::Rhythm { .. }), "the rest let pass, the chain goes on");
        }
    }
}
