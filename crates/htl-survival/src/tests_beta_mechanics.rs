//! The beta page's mechanics, one rule each, on abilities made for the test: a hook that yanks its
//! caster to the wall or the foe it strikes (and does nothing when it misses), an orb that swaps
//! places (with a creature or a friend, never a champion or anything steadfast), a knockback that
//! stuns against a wall, an echo to come back to with the health it held, a jump pad, a boon that
//! cheats death once, a mirror that turns a hero's bolt and a creature's arrow back, a tether that
//! drags a straying foe back and stuns it, and bursts as a shield ends or lingering ground does;
//! and the rules say what each needs.

use crate::content::{AbilityDef, Content};
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules with these abilities (TOML, `[[ability]]` tables) added, on this land.
fn world(pvp: bool, land: FlatLand, extra: &str) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    add(&mut c, extra);
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(land), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// Abilities (TOML `[[ability]]` tables) added to the rules, off the board.
fn add(c: &mut Content, extra: &str) {
    #[derive(serde::Deserialize)]
    struct Extra {
        ability: Vec<AbilityDef>,
    }
    let e: Extra = toml::from_str(extra).expect("test abilities");
    for mut a in e.ability {
        a.bench = true;
        c.abilities.push(a);
    }
}

/// Whether the rules take these abilities.
fn check(extra: &str) -> anyhow::Result<()> {
    let mut c = Content::dev();
    add(&mut c, extra);
    c.validate_abilities()
}

fn open() -> FlatLand {
    FlatLand::default()
}

fn walled(at: [f32; 2], r: f32) -> FlatLand {
    FlatLand { walls: vec![(at, r)], ..Default::default() }
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A creature standing still, noticing no one till struck.
fn mob(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// Run on with these creatures held where they stand, idle (struck, they'd come at the hero).
fn run_still(s: &mut Survival, mobs: &[Id], secs: f32) -> Vec<Event> {
    let at: Vec<(Id, Vec3)> = mobs.iter().map(|m| (*m, s.mobs[m].pos)).collect();
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, pos) in &at {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *pos;
            }
        }
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

/// Put an ability on its key and use it (an aimed one: a tap of the click).
fn cast(s: &mut Survival, p: Id, key: &str) -> Vec<Event> {
    let slot = s.content.abilities[s.content.ability(key).unwrap()].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), false));
    let mut ev = s.step(DT);
    if matches!(s.players[&p].action, Action::Aim { .. }) {
        s.input(p, Input::Attack);
        ev.extend(s.step(DT));
        s.input(p, Input::AttackUp);
        ev.extend(s.step(DT));
    }
    ev
}

/// The blows `a` landed on `v` (their damage).
fn hits(ev: &[Event], a: Id, v: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(x), damage, outcome: Outcome::Hit, .. } if *attacker == a && *x == v && *damage > 0 => Some(*damage),
            _ => None,
        })
        .collect()
}

/// The hero's pulls on a hook's line, flown through the air (a hop with the line's own flat
/// Swim look), or run along the ground (a dash): (facing, length). A hop's length is its speed
/// over its time in the air (the rules' gravity, 19.29 yd/s²).
fn dashes(ev: &[Event], who: Id) -> Vec<(f32, f32)> {
    let looked = ev.iter().any(|e| matches!(e, Event::LeapLook { player, clip: 42, .. } if *player == who));
    ev.iter()
        .filter_map(|e| match e {
            Event::Dash { player, dir, speed, secs } if *player == who => Some((*dir, speed * secs)),
            Event::Hop { player, dir, speed, lift, .. } if *player == who && looked => Some((*dir, speed * 2.0 * lift.max(0.5) / 19.29)),
            _ => None,
        })
        .collect()
}

/// A hook pulls its caster through the air, as a hookshot does (play-test: a run along the
/// ground read as a Charge): a flat hop, the body flat along the line (Swim), landing at the
/// wall.
#[test]
fn a_hooks_pull_flies_its_caster_through_the_air() {
    let mut s = world(false, walled([12.0, 0.0], 1.0), HOOK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut ev = cast(&mut s, p, "t_hook");
    ev.extend(run(&mut s, 1.0));
    assert!(!ev.iter().any(|e| matches!(e, Event::Dash { player, .. } if *player == p)), "no run along the ground");
    let hop = ev.iter().find_map(|e| match e {
        Event::Hop { player, lift, .. } if *player == p => Some(*lift),
        _ => None,
    });
    assert!(hop.is_some_and(|l| l > 0.5 && l < 6.0), "a flat arc through the air: {hop:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::LeapLook { player, clip: 42, .. } if *player == p)), "flat along the line (Swim)");
}

/// The flight's Swim is the dwarf man's on every hero (Corey's review: a human's swim through the
/// air looked silly; the dwarf's read as a flight); a dwarf flies in their own.
#[test]
fn a_hooks_pull_flies_every_hero_in_the_dwarfs_swim() {
    let flight = |race: u8| {
        let mut s = world(false, walled([12.0, 0.0], 1.0), HOOK);
        let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
        s.players.get_mut(&p).unwrap().look.race = race;
        let mut ev = cast(&mut s, p, "t_hook");
        ev.extend(run(&mut s, 1.0));
        ev.into_iter().find_map(|e| match e {
            Event::LeapLook { player, clip: 42, from, .. } if player == p => Some(from),
            _ => None,
        })
    };
    assert_eq!(flight(1).as_deref(), Some("DwarfMale"), "a human flies in the dwarf man's Swim");
    assert_eq!(flight(4).as_deref(), Some("DwarfMale"), "a night elf too");
    assert_eq!(flight(3).as_deref(), Some(""), "a dwarf in their own");
}

fn hops(ev: &[Event], who: Id) -> Vec<(f32, f32)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Hop { player, dir, speed, .. } if *player == who => Some((*dir, *speed)),
            _ => None,
        })
        .collect()
}

fn teleported(ev: &[Event], who: Id) -> bool {
    ev.iter().any(|e| matches!(e, Event::Teleport { player, .. } if *player == who))
}

/// Where a creature is headed (its slide's end), or where it stands.
fn bound(s: &Survival, m: Id) -> Vec3 {
    s.mobs[&m].path.last().copied().unwrap_or(s.mobs[&m].pos)
}

/// A hero's health set (and in a fight just now, so nothing mends it meanwhile).
fn set_health(s: &mut Survival, p: Id, h: u32) {
    let now = s.clock;
    let x = s.players.get_mut(&p).unwrap();
    x.health = h;
    x.combat_at = now;
}

// --- a hook that yanks ----------------------------------------------------------------------------

const HOOK: &str = r#"
[[ability]]
key = "t_hook"
name = "Hook"
school = "warrior"
slot = "ranged"
kind = "projectile"
speed = 60.0
range = [24.0, 24.0]
radius = [0.3, 0.3]
damage = [10.0, 10.0]
yank = true
spell = 16075
"#;

#[test]
fn a_hook_into_a_wall_yanks_its_caster_up_to_it() {
    let mut s = world(false, walled([12.0, 0.0], 1.0), HOOK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let mut ev = cast(&mut s, p, "t_hook");
    ev.extend(run(&mut s, 1.5));
    let d = dashes(&ev, p);
    assert_eq!(d.len(), 1, "one dash to the wall: {d:?}");
    assert!(d[0].0.sin().abs() < 0.15 && d[0].0.cos() > 0.0 && d[0].1 > 8.5 && d[0].1 < 11.0, "along +x, short of the wall: {d:?}");
    let at = s.players[&p].pos;
    assert!(at[0] > 8.5 && at[0] < 11.0 && at[1].abs() < 0.6, "she stands at the wall's foot, not in it: {at:?}");
}

#[test]
fn a_hook_into_a_foe_yanks_its_caster_beside_it_and_a_miss_does_nothing() {
    let mut s = world(false, open(), HOOK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [14.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "t_hook");
    ev.extend(run_still(&mut s, &[b], 1.5));
    assert_eq!(hits(&ev, p, b).len(), 1, "the boar is struck");
    assert_eq!(dashes(&ev, p).len(), 1, "and she's pulled to it");
    let gap = dist2d(s.players[&p].pos, s.mobs[&b].pos);
    assert!(gap > 0.6 && gap < 2.0, "beside it, not in it: {gap}");
    // Nothing in reach: the hook comes to nothing, and she stays.
    s.mobs.clear();
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    let mut ev = cast(&mut s, p, "t_hook");
    ev.extend(run(&mut s, 1.0));
    assert!(dashes(&ev, p).is_empty(), "no pull on a miss");
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.1, "she stays: {:?}", s.players[&p].pos);
}

// --- an orb that swaps places ------------------------------------------------------------------------

const ORB: &str = r#"
[[ability]]
key = "t_orb"
name = "Orb"
school = "mage"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [22.0, 22.0]
radius = [0.4, 0.4]
damage = [6.0, 6.0]
swap = true
burst_spell = 1953
spell = 133
"#;

/// Run until the hero is set down somewhere (at most `secs`): the events till then.
fn run_till_teleport(s: &mut Survival, p: Id, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        let ev = s.step(DT);
        let done = teleported(&ev, p);
        out.extend(ev);
        if done {
            break;
        }
    }
    out
}

#[test]
fn an_orb_swaps_its_caster_with_the_creature_it_strikes() {
    let mut s = world(false, open(), ORB);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [10.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "t_orb");
    ev.extend(run_till_teleport(&mut s, p, 1.0));
    assert_eq!(hits(&ev, p, b).len(), 1, "struck");
    assert!(teleported(&ev, p), "she's set down");
    assert!(dist2d(s.players[&p].pos, [10.0, 0.0, 0.0]) < 0.5, "where the boar stood: {:?}", s.players[&p].pos);
    assert!(ev.iter().any(|e| matches!(e, Event::MobStop { id, pos, .. } if *id == b && dist2d(*pos, ORIGIN) < 0.2)), "and the boar where she stood");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 1953, .. })), "a flash at the spots");
}

#[test]
fn an_orb_swaps_with_a_friend_unharmed_but_never_a_champion_or_a_steadfast_foe() {
    let mut s = world(false, open(), ORB);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let hp = s.players[&f].health;
    let mut ev = cast(&mut s, p, "t_orb");
    ev.extend(run_till_teleport(&mut s, p, 1.0));
    assert!(dist2d(s.players[&p].pos, [10.0, 0.0, 0.0]) < 0.5, "she's where he stood: {:?}", s.players[&p].pos);
    assert!(dist2d(s.players[&f].pos, ORIGIN) < 0.5, "he's where she stood: {:?}", s.players[&f].pos);
    assert_eq!(s.players[&f].health, hp, "and he's not hurt");
    // A champion holds its ground (it's only struck).
    let mut s = world(false, open(), ORB);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let w = mob(&mut s, "warlord", [10.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "t_orb");
    ev.extend(run_still(&mut s, &[w], 1.0));
    assert_eq!(hits(&ev, p, w).len(), 1, "the champion is struck");
    assert!(!teleported(&ev, p) && dist2d(s.players[&p].pos, ORIGIN) < 0.1, "but nobody moves");
    // So does a steadfast creature.
    let mut s = world(false, open(), ORB);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [10.0, 0.0, 0.0]);
    s.apply_cc(b, p, "steadfast", 5.0, 1.0, 0.0, "test");
    let mut ev = cast(&mut s, p, "t_orb");
    ev.extend(run_still(&mut s, &[b], 1.0));
    assert_eq!(hits(&ev, p, b).len(), 1, "struck");
    assert!(!teleported(&ev, p), "not swapped");
}

// --- a knockback into a wall -------------------------------------------------------------------------

const SLAM: &str = r#"
[[ability]]
key = "t_slam"
name = "Slam"
school = "warrior"
slot = "close"
kind = "nova"
damage = [2.0, 2.0]
radius = [6.0, 6.0]
spell = 6343
effects = [{ kind = "knockback", amount = 7.0, wall_stun = 1.5 }]
"#;

#[test]
fn a_foe_knocked_into_a_wall_is_stunned_one_thrown_clear_is_not() {
    let mut s = world(false, walled([6.0, 0.0], 1.0), SLAM);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let pinned = mob(&mut s, "grunt", [3.0, 0.0, 0.0]);
    let clear = mob(&mut s, "grunt", [0.0, -3.0, 0.0]);
    cast(&mut s, p, "t_slam");
    let to = bound(&s, pinned);
    assert!(to[0] < 5.0, "it stops at the wall: {to:?}");
    assert!(!s.has_cc(pinned, "stun"), "not before it hits it");
    run(&mut s, 0.6);
    assert!(s.has_cc(pinned, "stun"), "stunned against the wall");
    assert!(!s.has_cc(clear, "stun"), "the one thrown clear isn't");
}

// --- an echo to come back to -------------------------------------------------------------------------

const REWIND: &str = r#"
[[ability]]
key = "t_rewind"
name = "Rewind"
school = "mage"
slot = "utility"
kind = "buff"
rewind = true
recast = "t_rewind_back"
recast_secs = 4.0
spell = 1953
linger_spell = 22191
burst_spell = 5162

[[ability]]
key = "t_rewind_back"
name = "Rewind: Return"
slot = "utility"
kind = "buff"
inner = true
spell = 1953

[[ability]]
key = "t_recall"
name = "Recall"
school = "priest"
slot = "utility"
kind = "buff"
rewind = true
rewind_secs = 2.0
spell = 1953
"#;

#[test]
fn an_echo_brings_its_hero_back_with_the_health_it_held_never_less() {
    let mut s = world(false, open(), REWIND);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let max = s.players[&p].max_health;
    let half = max / 2;
    set_health(&mut s, p, half);
    let ev = cast(&mut s, p, "t_rewind");
    assert!(ev.iter().any(|e| matches!(e, Event::AreaLook { spell: 22191, .. })), "the echo's look lies where she stood");
    assert!(s.beta.echoes.contains_key(&p), "an echo");
    // Away, and hurt; the key again: back, whole as she was.
    s.move_player(p, [10.0, 0.0, 0.0], 0.0, Moving::default());
    set_health(&mut s, p, half - 20);
    run(&mut s, 0.5);
    s.input(p, Input::Ability(key_of("utility"), false));
    run(&mut s, 0.1);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back at the echo: {:?}", s.players[&p].pos);
    let h = s.players[&p].health;
    assert!(h >= half && h <= half + 2, "the health the echo held: {h} of {half}");
    assert!(!s.beta.echoes.contains_key(&p), "the echo is spent");
    // Healthier now than the echo: it never takes health away.
    run(&mut s, 0.5);
    cast(&mut s, p, "t_rewind");
    s.move_player(p, [6.0, 0.0, 0.0], 0.0, Moving::default());
    set_health(&mut s, p, max - 5);
    run(&mut s, 0.5);
    s.input(p, Input::Ability(key_of("utility"), false));
    run(&mut s, 0.1);
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back again");
    assert!(s.players[&p].health >= max - 5, "no health lost: {}", s.players[&p].health);
    // Left unused, it fades with the window.
    run(&mut s, 0.5);
    cast(&mut s, p, "t_rewind");
    assert!(s.beta.echoes.contains_key(&p), "a fresh echo");
    run(&mut s, 4.5);
    assert!(!s.beta.echoes.contains_key(&p), "faded");
}

#[test]
fn an_echo_with_no_recast_calls_its_hero_back_by_itself() {
    let mut s = world(false, open(), REWIND);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    cast(&mut s, p, "t_recall");
    s.move_player(p, [8.0, 3.0, 0.0], 0.0, Moving::default());
    let ev = run(&mut s, 1.5);
    assert!(!teleported(&ev, p), "not yet");
    let ev = run(&mut s, 1.0);
    assert!(teleported(&ev, p), "called back");
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "to where she cast it: {:?}", s.players[&p].pos);
}

// --- a jump pad --------------------------------------------------------------------------------------

const PAD: &str = r#"
[[ability]]
key = "t_pad"
name = "Pad"
school = "hunter"
slot = "utility"
kind = "trap"
at_self = true
radius = [1.5, 1.5]
secs = 8.0
launch = 14.0
spell = 13812
"#;

#[test]
fn a_pad_launches_its_caster_and_a_friend_the_way_they_face_and_stays() {
    let mut s = world(false, open(), PAD);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [0.0, 6.0, 0.0], 0.0);
    let mut ev = cast(&mut s, p, "t_pad");
    ev.extend(run(&mut s, 1.0));
    let h = hops(&ev, p);
    assert_eq!(h.len(), 1, "she's launched once: {h:?}");
    assert!(h[0].0.abs() < 0.05 && h[0].1 > 5.0, "the way she faces: {h:?}");
    assert!(dist2d(s.players[&p].pos, [14.0, 0.0, 0.0]) < 0.5, "and lands 14 yd on: {:?}", s.players[&p].pos);
    assert_eq!(s.traps.len(), 1, "the pad stays");
    // A friend steps on it, facing west: away along +y.
    s.move_player(f, ORIGIN, std::f32::consts::FRAC_PI_2, Moving::default());
    let ev = run(&mut s, 0.2);
    let h = hops(&ev, f);
    assert_eq!(h.len(), 1, "he's launched: {h:?}");
    assert!((h[0].0 - std::f32::consts::FRAC_PI_2).abs() < 0.05, "his way: {h:?}");
    // A creature on it sets nothing off.
    run(&mut s, 1.0);
    let b = mob(&mut s, "boar", [0.5, 0.0, 0.0]);
    let hp = s.mobs[&b].health;
    run_still(&mut s, &[b], 0.5);
    assert_eq!(s.mobs[&b].health, hp, "untouched");
    assert_eq!(s.traps.len(), 1, "and the pad's still there");
}

// --- cheating death -------------------------------------------------------------------------------------

const CHEAT: &str = r#"
[[ability]]
key = "t_cheat"
name = "Cheat"
school = "priest"
slot = "defense"
kind = "buff"
spell = 20711
effects = [{ kind = "cheat_death", secs = [6.0, 6.0], amount = 0.3 }]
"#;

#[test]
fn a_blow_that_would_fell_a_hero_cheating_death_leaves_them_at_one_and_mends_them_once() {
    let mut s = world(false, open(), CHEAT);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let max = s.players[&p].max_health;
    cast(&mut s, p, "t_cheat");
    assert!(s.has_cc(p, "cheat_death"), "the boon");
    s.hurt_player(p, 9999);
    assert_eq!(s.players[&p].health, 1, "left at 1");
    assert!(!s.has_cc(p, "cheat_death") && s.has_cc(p, "regen"), "the boon spent, mending");
    run(&mut s, 3.2);
    let h = s.players[&p].health;
    assert!(h >= 1 + (max as f32 * 0.25) as u32, "about 30% back over 3 s: {h} of {max}");
    // Spent: the next killing blow fells them.
    s.hurt_player(p, 9999);
    assert_eq!(s.players[&p].health, 0, "down");
}

// --- a mirror -------------------------------------------------------------------------------------------

const MIRROR: &str = r#"
[[ability]]
key = "t_mirror"
name = "Mirror"
school = "mage"
slot = "defense"
kind = "stance"
secs = 4.0
spell = 543
aura = 543
counter = { mirror = true }

[[ability]]
key = "t_bolt"
name = "Bolt"
school = "warlock"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [20.0, 20.0]
radius = [0.4, 0.4]
damage = [12.0, 12.0]
spell = 686
"#;

#[test]
fn a_mirror_turns_a_rivals_bolt_back_at_them() {
    let mut s = world(true, open(), MIRROR);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], std::f32::consts::PI);
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    cast(&mut s, b, "t_mirror");
    let mut ev = cast(&mut s, a, "t_bolt");
    ev.extend(run(&mut s, 1.5));
    assert_eq!(s.players[&b].health, hb, "the mirror took nothing");
    assert!(!hits(&ev, b, a).is_empty(), "the bolt came back at her, his now");
    assert!(s.players[&a].health < ha, "and hurt her");
    // He sees it catch by its look on him (the spell), not its name in words.
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 543, target: Target::Unit(t) } if *caster == b && *t == b)), "he sees it catch");
    assert!(!ev.iter().any(|e| matches!(e, Event::Flash { player, .. } if *player == b)), "no word");
}

#[test]
fn a_mirror_turns_a_creatures_arrow_back_at_it_but_lets_a_blow_through() {
    let mut s = world(false, open(), MIRROR);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let max = s.players[&p].max_health;
    let k = mob(&mut s, "archer", [14.0, 0.0, 0.0]);
    cast(&mut s, p, "t_mirror");
    s.mobs.get_mut(&k).unwrap().state = MobState::Chase { foe: Foe::Hero(p) };
    let ev = run(&mut s, 3.0);
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 75, .. } if *caster == k)), "it shoots");
    assert!(!hits(&ev, p, k).is_empty(), "its arrow comes back at it");
    assert_eq!(s.players[&p].health, max, "and never reaches her");
}

// --- a tether -------------------------------------------------------------------------------------------

const CHAIN: &str = r#"
[[ability]]
key = "t_chain"
name = "Chain"
school = "warlock"
slot = "ultimate"
kind = "projectile"
speed = 40.0
range = [20.0, 20.0]
radius = [0.4, 0.4]
damage = [14.0, 14.0]
tether_secs = 5.0
tether_range = 8.0
tether_effects = [{ kind = "stun", secs = [1.5, 1.5] }]
tick_spell = 689
spell = 686
"#;

#[test]
fn a_tethered_foe_that_strays_too_far_is_dragged_back_and_stunned_once() {
    let mut s = world(false, open(), CHAIN);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [6.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "t_chain");
    ev.extend(run_still(&mut s, &[b], 0.5));
    assert_eq!(hits(&ev, p, b).len(), 1, "struck");
    assert_eq!(s.beta.tethers.len(), 1, "and bound");
    assert!(ev.iter().any(|e| matches!(e, Event::Channel { caster, target: Some(t), spell: 689, .. } if *caster == p && *t == b)), "the link drawn between them");
    assert!(!s.has_cc(b, "stun"), "close by, nothing more");
    // Gone too far: dragged back to her, and stunned as it lands.
    s.mobs.get_mut(&b).unwrap().pos = [12.0, 0.0, 0.0];
    let ev = run(&mut s, 0.05);
    let to = bound(&s, b);
    assert!(to[0] < 3.0, "pulled back to her: {to:?}");
    assert!(s.beta.tethers.is_empty(), "the tether's done");
    assert!(ev.iter().any(|e| matches!(e, Event::Channel { caster, target: None, .. } if *caster == p)), "its link gone");
    run(&mut s, 0.5);
    assert!(s.has_cc(b, "stun"), "stunned");
}

// --- bursts as a shield or lingering ground ends -------------------------------------------------------

const BURSTS: &str = r#"
[[ability]]
key = "t_bubble"
name = "Bubble"
school = "priest"
slot = "defense"
kind = "buff"
spell = 17
effects = [{ kind = "shield", amount = 20.0, secs = [4.0, 4.0], pop = "t_pop" }]

[[ability]]
key = "t_pop"
name = "Pop"
slot = "defense"
kind = "nova"
inner = true
damage = [5.0, 5.0]
radius = [4.0, 4.0]
spell = 10187
effects = [{ kind = "knockback", amount = 5.0 }]

[[ability]]
key = "t_hole"
name = "Hole"
school = "warlock"
slot = "ultimate"
kind = "ground"
at_self = true
radius = [6.0, 6.0]
linger = 2.0
end_burst = "t_collapse"
spell = 27810

[[ability]]
key = "t_collapse"
name = "Collapse"
slot = "ultimate"
kind = "nova"
inner = true
damage = [30.0, 30.0]
spell = 8092
effects = [{ kind = "knockback", amount = 6.0 }]
"#;

#[test]
fn a_shield_bursts_round_its_holder_as_it_breaks_or_runs_out() {
    let mut s = world(false, open(), BURSTS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [2.0, 0.0, 0.0]);
    cast(&mut s, p, "t_bubble");
    let ev = run_still(&mut s, &[b], 0.5);
    assert!(hits(&ev, p, b).is_empty(), "nothing while it holds");
    // Broken: it bursts.
    s.hurt_player(p, 25);
    let ev = run(&mut s, 0.1);
    assert_eq!(hits(&ev, p, b).len(), 1, "the boar caught in the burst");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 10187, .. })), "its look");
    // Run out: it bursts too.
    let mut s = world(false, open(), BURSTS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [2.0, 0.0, 0.0]);
    cast(&mut s, p, "t_bubble");
    let ev = run_still(&mut s, &[b], 3.5);
    assert!(hits(&ev, p, b).is_empty(), "not yet");
    let ev = run(&mut s, 1.0);
    assert_eq!(hits(&ev, p, b).len(), 1, "as it runs out");
}

#[test]
fn lingering_ground_bursts_over_its_whole_ground_as_it_ends() {
    let mut s = world(false, open(), BURSTS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [5.0, 0.0, 0.0]);
    let mut ev = cast(&mut s, p, "t_hole");
    ev.extend(run_still(&mut s, &[b], 1.5));
    assert!(hits(&ev, p, b).is_empty(), "nothing while it lingers");
    let ev = run_still(&mut s, &[b], 1.0);
    assert_eq!(hits(&ev, p, b), vec![30], "the collapse, as wide as the ground");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 8092, .. })), "its look");
}

// --- what the rules take -----------------------------------------------------------------------------------

#[test]
fn the_rules_take_the_beta_mechanics_and_turn_away_what_makes_no_sense() {
    for ok in [HOOK, ORB, SLAM, REWIND, PAD, CHEAT, MIRROR, CHAIN, BURSTS] {
        check(ok).unwrap();
    }
    let card = |kind: &str, more: &str| {
        format!("[[ability]]\nkey = \"t_bad\"\nname = \"Bad\"\nschool = \"mage\"\nslot = \"ranged\"\nkind = \"{kind}\"\nspeed = 30.0\nradius = [1.0, 1.0]\nsecs = 5.0\n{more}\n")
    };
    let bad = [
        card("nova", "yank = true"),
        card("projectile", "swap = true\nreturns = true"),
        card("projectile", "yank = true\nswap = true"),
        card("projectile", "swap = true\npierce = 2"),
        card("projectile", "tether_secs = 5.0"),
        card("projectile", "tether_range = 8.0"),
        card("nova", "tether_effects = [{ kind = \"stun\", secs = [1.0, 1.0] }]"),
        card("projectile", "rewind = true\nrewind_secs = 2.0"),
        card("buff", "rewind = true"),
        card("buff", "rewind_secs = 2.0"),
        card("nova", "launch = 10.0"),
        card("ground", "end_burst = \"t_pop\""),
        card("nova", "effects = [{ kind = \"pull\", amount = 5.0, wall_stun = 1.0 }]"),
        card("buff", "effects = [{ kind = \"haste\", amount = 0.3, secs = [2.0, 2.0], pop = \"t_pop\" }]"),
        card("buff", "effects = [{ kind = \"shield\", amount = 30.0, secs = [2.0, 2.0], pop = \"t_bubble\" }]"),
        card("buff", "effects = [{ kind = \"cheat_death\", secs = [6.0, 6.0], amount = 2.0 }]"),
        card("stance", "counter = { mirror = true, block = true }"),
    ];
    for b in bad {
        // (With the bursts to name.)
        let text = format!("{}\n{}", BURSTS, b);
        assert!(check(&text).is_err(), "should be turned away:\n{b}");
    }
}

// --- Grapple Arrow: what it catches, the aim's word on it, and Disengage's flip (#110) ------------

/// Open ground with a boulder only the land's shapes know (`Land::ray`; no cell a walker can't
/// enter), as Elwynn's rocks, fences and roofs were missing from what a hook could meet.
struct Boulder {
    at: [f32; 2],
    r: f32,
    top: f32,
}

impl crate::land::Land for Boulder {
    fn height(&self, _x: f32, _y: f32) -> f32 {
        0.0
    }
    fn ray(&self, from: Vec3, dir: Vec3, max: f32) -> Option<f32> {
        let mut d = 0.0f32;
        while d <= max {
            let p = [from[0] + dir[0] * d, from[1] + dir[1] * d, from[2] + dir[2] * d];
            if dist2d(p, [self.at[0], self.at[1], 0.0]) <= self.r && (0.0..=self.top).contains(&p[2]) {
                return Some(d);
            }
            d += 0.05;
        }
        None
    }
}

/// The built-in rules (Grapple Arrow and Ensnaring Retreat as they ship) on this land.
fn shipped_on(land: Box<dyn crate::land::Land>) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, land, 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// Grapple Arrow aimed along the reticle's line (facing, elevation) and loosed.
fn grapple(s: &mut Survival, p: Id, yaw: f32, pitch: f32) -> Vec<Event> {
    s.set_aim_line(p, yaw, pitch);
    let mut ev = cast(s, p, "grapple_arrow");
    ev.extend(run(s, 1.5));
    ev
}

/// What the hero was told about where the aimed hook would catch, in order.
fn catches(ev: &[Event], who: Id) -> Vec<Option<(Vec3, bool)>> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Catch { player, at, .. } if *player == who => Some(*at),
            _ => None,
        })
        .collect()
}

#[test]
fn grapple_arrow_catches_a_wall_and_a_boulder_the_walking_grid_never_had() {
    // A wall: pulled to its foot.
    let mut s = shipped_on(Box::new(walled([12.0, 0.0], 1.0)));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ev = grapple(&mut s, p, 0.0, 0.0);
    let d = dashes(&ev, p);
    assert_eq!(d.len(), 1, "one pull to the wall: {d:?}");
    let at = s.players[&p].pos;
    assert!(at[0] > 8.5 && at[0] < 11.0 && at[1].abs() < 0.8, "at the wall's foot: {at:?}");
    // Its line from the fist to where it bit, for as long as the pull (Corey's review: a rope).
    let lines: Vec<(Target, f32)> = ev.iter().filter_map(|e| match e {
        Event::Tether { from, to, secs } if *from == p => Some((*to, *secs)),
        _ => None,
    }).collect();
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(matches!(lines[0].0, Target::Point(at) if at[0] > 10.0 && at[0] < 13.0), "to the wall: {lines:?}");
    let flight = ev.iter().find_map(|e| match e { Event::LeapLook { player, secs, .. } if *player == p => Some(*secs), _ => None }).expect("the pull's flight");
    assert!((lines[0].1 - flight).abs() < 1e-3 && flight >= 0.25, "as long as the pull ({flight} s): {lines:?}");
    // A boulder only the land's shapes know: caught all the same.
    let mut s = shipped_on(Box::new(Boulder { at: [15.0, 0.0], r: 1.5, top: 2.5 }));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ev = grapple(&mut s, p, 0.0, 0.0);
    assert_eq!(dashes(&ev, p).len(), 1, "pulled to the boulder");
    let at = s.players[&p].pos;
    assert!(at[0] > 10.5 && at[0] < 13.6, "short of the boulder, not in it: {at:?}");
    // Over its top, nothing to catch: the arrow runs out in the open.
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    let ev = grapple(&mut s, p, 0.0, 0.35);
    assert!(dashes(&ev, p).is_empty(), "nothing caught over the top");
}

#[test]
fn grapple_arrow_into_open_air_does_nothing() {
    let mut s = shipped_on(Box::new(open()));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ev = grapple(&mut s, p, 1.0, 0.3);
    assert!(dashes(&ev, p).is_empty(), "no pull on a miss");
    assert!(!ev.iter().any(|e| matches!(e, Event::Tether { .. })), "and no line");
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.1, "she stays: {:?}", s.players[&p].pos);
}

#[test]
fn grapple_arrow_goes_where_the_reticle_is_not_to_the_locked_foe() {
    let mut s = shipped_on(Box::new(open()));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = mob(&mut s, "boar", [14.0, 0.0, 0.0]);
    s.players.get_mut(&p).unwrap().lock = Some(b);
    // Locked on, the reticle off to the side and up: the boar isn't caught.
    s.set_aim_line(p, 0.9, 0.3);
    let mut ev = cast(&mut s, p, "grapple_arrow");
    ev.extend(run_still(&mut s, &[b], 1.0));
    assert!(dashes(&ev, p).is_empty(), "the lock alone pulls nothing: {:?}", dashes(&ev, p));
    // Wide of it by most of a yard: the boar isn't caught (a foe counts only on the line), the
    // ground past it is.
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    s.set_aim_line(p, (0.85f32 / 14.0).atan(), -0.1);
    let mut ev = cast(&mut s, p, "grapple_arrow");
    ev.extend(run_still(&mut s, &[b], 1.5));
    assert_eq!(dashes(&ev, p).len(), 1, "pulled to the ground");
    assert!(s.players[&p].pos[0] > 15.5, "past the boar, not beside it: {:?}", s.players[&p].pos);
    // The reticle on it: caught, and pulled beside it, lock or no lock.
    s.players.get_mut(&p).unwrap().lock = None;
    s.move_player(p, ORIGIN, 0.0, Moving::default());
    s.set_aim_line(p, 0.0, -0.1);
    let mut ev = cast(&mut s, p, "grapple_arrow");
    ev.extend(run_still(&mut s, &[b], 1.0));
    assert_eq!(dashes(&ev, p).len(), 1, "on the line, the boar is caught");
    let gap = dist2d(s.players[&p].pos, s.mobs[&b].pos);
    assert!(gap > 0.6 && gap < 2.0, "beside it: {gap}");
}

#[test]
fn while_aimed_the_hero_is_told_where_the_hook_would_catch_and_when_it_would_miss() {
    let mut s = shipped_on(Box::new(walled([12.0, 0.0], 1.0)));
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = s.content.ability("grapple_arrow").unwrap();
    s.players.get_mut(&p).unwrap().owned.insert("grapple_arrow".into());
    s.slot_ability(p, "utility", "grapple_arrow").unwrap();
    s.set_cooldowns(p, true);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Ability(key_of("utility"), false));
    let ev = run(&mut s, 0.1);
    assert!(matches!(s.players[&p].action, Action::Aim { .. }), "aiming");
    // At the wall: a catch, at its face.
    let told = catches(&ev, p);
    let Some(Some((at, true))) = told.last().copied() else { panic!("a catch at the wall: {told:?}") };
    assert!(at[0] > 10.0 && at[0] < 11.5 && at[1].abs() < 0.8, "at the wall's face: {at:?}");
    assert_eq!(s.catch_preview(p, k), Some((at, true)), "the same as asked now");
    // Held still, nothing new to say.
    s.set_aim_line(p, 0.0, 0.0);
    assert!(catches(&run(&mut s, 0.3), p).is_empty(), "no repeats");
    // Up at the sky the other way (this test's wall stands to any height): a miss, out at the end
    // of its range.
    s.set_aim_line(p, std::f32::consts::PI, 0.5);
    let told = catches(&run(&mut s, 0.1), p);
    let Some(Some((end, false))) = told.last().copied() else { panic!("a miss: {told:?}") };
    assert!(dist2d(end, ORIGIN) > 18.0, "out at its range's end: {end:?}");
    // Back on the wall and loosed: it catches where it said, and the word is taken back.
    s.set_aim_line(p, 0.0, 0.0);
    let _ = run(&mut s, 0.1);
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 1.0));
    assert!(catches(&ev, p).contains(&None), "the aim let go: {:?}", catches(&ev, p));
    assert_eq!(dashes(&ev, p).len(), 1, "and the pull to the wall");
}

#[test]
fn every_disengage_plays_the_same_borrowed_flip_from_the_ground_too() {
    let c = Content::dev();
    for key in ["ensnaring_retreat", "trappers_leap"] {
        let d = &c.abilities[c.ability(key).unwrap()];
        assert!(d.leap_flip && d.leap_clip != 0 && d.leap_from == "NightElfFemale", "{key}: the Night Elf woman's jump");
    }
    let mut s = shipped_on(Box::new(open()));
    for name in ["Ayla", "Grosh"] {
        let p = hero(&mut s, name, ORIGIN, 0.0);
        let ev = cast(&mut s, p, "ensnaring_retreat");
        let looks: Vec<(u16, String, bool, f32)> = ev
            .iter()
            .filter_map(|e| match e {
                Event::LeapLook { player, clip, from, flip, secs, .. } if *player == p => Some((*clip, from.clone(), *flip, *secs)),
                _ => None,
            })
            .collect();
        assert_eq!(looks.len(), 1, "{name}: one flip: {looks:?}");
        let (clip, from, flip, secs) = looks[0].clone();
        assert!(clip == 38 && from == "NightElfFemale" && flip && secs > 0.3, "{name}: {looks:?}");
        assert!(!hops(&ev, p).is_empty(), "{name}: a hop back into it, from the ground");
        // As the feet touch down, the hero's own JumpEnd (39) takes the landing, so the borrowed
        // clip's tucked legs never snap back to rest on the ground.
        let landed = run(&mut s, 2.0);
        let ends: Vec<f32> = landed
            .iter()
            .filter_map(|e| match e {
                Event::Clip { id, clip: 39, secs, .. } if *id == p => Some(*secs),
                _ => None,
            })
            .collect();
        assert!(ends.len() == 1 && ends[0] > 0.2 && ends[0] < 0.5, "{name}: one JumpEnd at the landing: {ends:?}");
    }
    // A leap's look on anything but a leap, or from no model the clients know, is turned away.
    let card = |kind: &str, more: &str| format!("[[ability]]\nkey = \"t_bad\"\nname = \"Bad\"\nschool = \"hunter\"\nslot = \"utility\"\nkind = \"{kind}\"\nrange = [8.0, 8.0]\n{more}\n");
    check(&card("leap", "back = true\nleap_clip = 38\nleap_from = \"NightElfFemale\"\nleap_flip = true")).unwrap();
    for bad in [card("nova", "leap_flip = true"), card("leap", "leap_from = \"ScourgeFemale\""), card("leap", "leap_clip = 38\nleap_from = \"Murloc\"")] {
        assert!(check(&bad).is_err(), "should be turned away:\n{bad}");
    }
}

#[test]
fn a_pulls_look_on_its_caster_is_only_for_an_ability_that_pulls_from_a_model_the_clients_know() {
    let card = |more: &str| format!("[[ability]]\nkey = \"t_bad\"\nname = \"Bad\"\nschool = \"hunter\"\nslot = \"utility\"\nkind = \"projectile\"\nrange = [8.0, 8.0]\nspeed = 40.0\n{more}\n");
    let pull = "effects = [{ kind = \"pull\", amount = 10.0, from_caster = true }]";
    check(&card(&format!("pull_clip = 63\n{pull}"))).unwrap();
    check(&card(&format!("pull_clip = 63\npull_from = \"ScourgeFemale\"\n{pull}"))).unwrap();
    for bad in [card("pull_clip = 63"), card(&format!("pull_from = \"ScourgeFemale\"\n{pull}")), card(&format!("pull_clip = 63\npull_from = \"Murloc\"\n{pull}"))] {
        assert!(check(&bad).is_err(), "should be turned away:\n{bad}");
    }
}
