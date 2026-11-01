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

/// The hero's dashes: (facing, length).
fn dashes(ev: &[Event], who: Id) -> Vec<(f32, f32)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Dash { player, dir, speed, secs } if *player == who => Some((*dir, speed * secs)),
            _ => None,
        })
        .collect()
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
    ev.extend(run(&mut s, 1.0));
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
    ev.extend(run_still(&mut s, &[b], 1.0));
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
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == b && text == "Mirror")), "he sees it catch");
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
