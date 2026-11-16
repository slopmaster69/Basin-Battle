//! A guard for Corey's freeze (16 Nov, build 008ee00): "Blade dance into heavy pretty much froze
//! my game." A Rogue with Blades' cards casts Blade Dance, a pincer of Goblin Woodcutters comes at
//! it, and the heavy attack (Shift + left) is pressed before, during and at the end of the dance,
//! with a dodge before it so the Shadow Cut is open (every evading dodge is perfect in the dance).
//! The server must hand the hero back free, and nothing it sends may run away: the events of one
//! tick, and the teleports and swings of the whole fight, stay bounded.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.02;
const PI: f32 = std::f32::consts::PI;

/// Corey's keys: Darting Knife, Blade Flurry, Quickstep, Evasion and Blade Dance.
const KEYS: [(&str, &str); 5] = [("ranged", "darting_knife"), ("close", "blade_flurry"), ("utility", "quickstep"), ("defense", "blade_evasion"), ("ultimate", "blade_dance")];

/// The most events one tick may send for one hero alone in a fight with two foes.
const MOST_PER_TICK: usize = 60;

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// A world with a Rogue holding `main` and `off` (`off` None: an empty off hand; `main` a fist
/// weapon fills both), Corey's keys on, and two Goblin Woodcutters either side, fighting.
fn setup(main: &str, off: Option<&str>) -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let save = s.fresh_save("moneyman", "Moneyman", "rogue", HeroLook::default());
    let p = s.join("moneyman", "Moneyman", Some(&save));
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    for (slot, k) in KEYS {
        s.slot_ability(p, slot, k).unwrap();
    }
    let item = |s: &Survival, k: &str| s.content.item(k).unwrap_or_else(|| panic!("no item {k}"));
    let main_i = item(&s, main);
    if crate::sim::rogue::is_fist(&s.content, main_i) {
        s.players.get_mut(&p).unwrap().bag = [(main_i, 1)].into();
        s.wear_from_bag(p, main_i).unwrap();
    } else {
        let off_i = off.map(|o| item(&s, o));
        let h = s.players.get_mut(&p).unwrap();
        h.weapon = main_i;
        h.off_weapon = off_i;
    }
    let h = s.players.get_mut(&p).unwrap();
    // Alive the whole way: the guard is about what the server sends, not who wins.
    h.max_health = 1_000_000;
    h.health = 1_000_000;
    let def = s.content.mob("dm_woodcutter").unwrap();
    for (at, f) in [([2.0, 0.0, 0.0], PI), ([-2.0, 0.0, 0.0], 0.0)] {
        let m = s.spawn_mob(def, at, None, false);
        let m = s.mobs.get_mut(&m).unwrap();
        m.pos = at;
        m.facing = f;
        m.health = 1_000_000;
        m.max_health = 1_000_000;
    }
    (s, p)
}

/// One tick, its events counted against the bound.
fn tick(s: &mut Survival, worst: &mut usize, counts: &mut (usize, usize)) {
    let ev = s.step(DT);
    *worst = (*worst).max(ev.len());
    for e in &ev {
        match e {
            Event::Teleport { .. } => counts.0 += 1,
            Event::Swing { .. } => counts.1 += 1,
            _ => {}
        }
    }
}

fn free(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Ready | Action::Guard)
}

/// Blade Dance, then (after `wait` seconds, a dodge first when `dodge`) the heavy pressed, and
/// pressed again every tick for `mash` seconds; then 3 sec with no input. The hero is free again
/// in them with nothing buffered, and every bound held.
fn dance_then_heavy(main: &str, off: Option<&str>, wait: f32, dodge: bool, mash: f32) {
    let (mut s, p) = setup(main, off);
    let what = format!("{main}/{off:?}, heavy {wait} s after the dance, dodge {dodge}, mash {mash} s");
    let (mut worst, mut counts) = (0usize, (0usize, 0usize));
    // Pressed before the dance: the heavy, and the dance on top of it.
    if wait < 0.0 {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Heavy);
        tick(&mut s, &mut worst, &mut counts);
    }
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(key_of("ultimate"), false));
    tick(&mut s, &mut worst, &mut counts);
    assert!(s.has_cc(p, "blade_dance") || wait < 0.0, "{what}: dancing");
    for _ in 0..(wait.max(0.0) / DT).round() as usize {
        tick(&mut s, &mut worst, &mut counts);
    }
    if dodge {
        s.input(p, Input::Dodge);
        for _ in 0..15 {
            tick(&mut s, &mut worst, &mut counts);
        }
    }
    for _ in 0..((mash / DT).round() as usize).max(1) {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.input(p, Input::Heavy);
        tick(&mut s, &mut worst, &mut counts);
        s.input(p, Input::AttackUp);
    }
    // Hands off: the swing plays out, the buffer empties, the foes keep swinging.
    let mut freed_at = None;
    for i in 0..(3.0 / DT) as usize {
        tick(&mut s, &mut worst, &mut counts);
        if freed_at.is_none() && free(&s, p) {
            freed_at = Some(i as f32 * DT);
        }
    }
    // (Free at some point in those 3 sec: the foes' blows may stagger it again after.)
    assert!(freed_at.is_some(), "{what}: never free again: {:?}", s.players[&p].action);
    assert!(s.players[&p].buffered.is_none(), "{what}: nothing left buffered");
    assert!(worst <= MOST_PER_TICK, "{what}: {worst} events in one tick");
    // A Shadow Cut is one step for one press: no more teleports than presses and dodges.
    let presses = ((mash / DT).round() as usize).max(1) + 1;
    assert!(counts.0 <= presses, "{what}: {} teleports for {presses} presses", counts.0);
    // Swings: the hero's own and the foes' (two foes, about one blow a second each, and every
    // dodge's Blade Dance cut), nowhere near one a tick.
    let ticks = ((wait.max(0.0) + mash + 3.6) / DT) as usize;
    assert!(counts.1 < ticks / 4, "{what}: {} swings in {ticks} ticks", counts.1);
}

#[test]
fn a_heavy_before_during_and_after_blade_dance_hands_the_rogue_back_free() {
    let hands: [(&str, Option<&str>); 4] = [("dagger", Some("dagger")), ("sword", Some("dagger")), ("footpad_knuckles", None), ("dagger", None)];
    for (main, off) in hands {
        for wait in [-1.0, 0.0, 0.1, 0.5, 4.0, 7.9, 8.1] {
            for dodge in [false, true] {
                dance_then_heavy(main, off, wait, dodge, 0.0);
            }
        }
        // Held and mashed through the dance's last moments and past its end.
        dance_then_heavy(main, off, 7.0, true, 2.0);
        dance_then_heavy(main, off, 0.0, false, 1.5);
    }
}
