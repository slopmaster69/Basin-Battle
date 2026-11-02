//! The moving mechanics, one rule each, on abilities made for the test: knockbacks and pulls,
//! leaps and disengages, strips, wind-ups, traps, an anchor, piercing, chaining and bursting
//! projectiles, shields, mending, haste, a veil, a grounding totem, Evasion, a vortex, and an
//! ability fired with another.

use crate::content::{AbilityDef, Content};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules with these abilities (TOML, `[[ability]]` tables) added.
fn world(pvp: bool, extra: &str) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
    #[derive(serde::Deserialize)]
    struct Extra {
        ability: Vec<AbilityDef>,
    }
    let e: Extra = toml::from_str(extra).expect("test abilities");
    for mut a in e.ability {
        a.bench = true;
        c.abilities.push(a);
    }
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
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

fn hops(ev: &[Event], who: Id) -> Vec<(f32, f32)> {
    ev.iter().filter_map(|e| match e { Event::Hop { player, dir, speed, .. } if *player == who => Some((*dir, *speed)), _ => None }).collect()
}

/// Where a creature is headed (its slide's end), or where it stands.
fn bound(s: &Survival, m: Id) -> Vec3 {
    s.mobs[&m].path.last().copied().unwrap_or(s.mobs[&m].pos)
}

const NOVA_KNOCK: &str = r#"
[[ability]]
key = "t_thunder"
name = "Thunder"
school = "shaman"
slot = "close"
kind = "nova"
damage = [5.0, 5.0]
radius = [6.0, 6.0]
spell = 6343
effects = [{ kind = "knockback", amount = 7.0 }]
"#;

#[test]
fn a_knockback_throws_creatures_and_heroes_away_from_its_middle() {
    let mut s = world(true, NOVA_KNOCK);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let m = mob(&mut s, "grunt", [0.0, 3.0, 0.0]);
    let ev = cast(&mut s, p, "t_thunder");
    let to = bound(&s, m);
    assert!(to[1] > 8.0 && to[0].abs() < 0.5, "the creature thrown north-west, away: {to:?}");
    assert!(s.mobs[&m].stagger_until > s.clock, "and reeling while it flies");
    let h = hops(&ev, b);
    assert_eq!(h.len(), 1, "the hero's client flies it");
    assert!(h[0].0.abs() < 0.1 && h[0].1 > 5.0, "away along +x: {h:?}");
}

const PULL_BOLT: &str = r#"
[[ability]]
key = "t_grip"
name = "Grip"
school = "priest"
slot = "utility"
kind = "projectile"
speed = 40.0
range = [20.0, 20.0]
radius = [0.4, 0.4]
friendly = true
spell = 6533
effects = [{ kind = "pull", amount = 20.0, from_caster = true }]
"#;

#[test]
fn a_pull_brings_a_creature_to_the_caster_and_a_friend_to_safety() {
    let mut s = world(false, PULL_BOLT);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    cast(&mut s, p, "t_grip");
    run(&mut s, 0.5);
    let to = bound(&s, m);
    assert!(to[0] < 2.5 && to[0] > 0.5, "pulled to just in front: {to:?}");
    // A friend down the line (no friendly fire): pulled too, not struck.
    let f = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    s.mobs.clear();
    let hp = s.players[&f].health;
    let mut ev = cast(&mut s, p, "t_grip");
    ev.extend(run(&mut s, 0.5));
    let h = hops(&ev, f);
    assert_eq!(h.len(), 1, "the friend is pulled");
    assert!((h[0].0 - std::f32::consts::PI).abs() < 0.1, "toward the caster");
    assert_eq!(s.players[&f].health, hp, "and not hurt");
}

const LEAPS: &str = r#"
[[ability]]
key = "t_leap"
name = "Leap"
school = "druid"
slot = "utility"
kind = "leap"
range = [12.0, 12.0]
radius = [3.0, 3.0]
damage = [10.0, 10.0]
spell = 16979
effects = [{ kind = "stagger", secs = [1.0, 1.0] }]

[[ability]]
key = "t_back"
name = "Back"
school = "hunter"
slot = "utility"
kind = "leap"
back = true
range = [9.0, 9.0]
cleanse = true
spell = 781
also = "t_net"

[[ability]]
key = "t_net"
name = "Net"
inner = true
slot = "utility"
kind = "projectile"
speed = 40.0
range = [10.0, 10.0]
radius = [0.4, 0.4]
spell = 6533
effects = [{ kind = "root", secs = [2.0, 2.0] }]
"#;

#[test]
fn a_leap_lands_where_it_was_aimed_drawn_for_everyone_and_strikes_there() {
    let mut s = world(false, LEAPS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [12.5, 0.0, 0.0]);
    let ev = cast(&mut s, p, "t_leap");
    let h = hops(&ev, p);
    assert_eq!(h.len(), 1);
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(t), .. } if t.hero && t.at.is_some())), "its landing ring");
    // Drawn for everyone but the hero leaping (they know where they aimed).
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(t), .. } if t.own_hidden && t.mob == p)), "not for her");
    let hp = s.mobs[&m].health;
    run(&mut s, 0.8);
    assert!((s.players[&p].pos[0] - 12.0).abs() < 0.5, "landed: {:?}", s.players[&p].pos);
    assert!(s.mobs[&m].health < hp, "what's there is struck");
}

#[test]
fn a_disengage_leaps_back_from_the_facing_and_fires_its_net_forward() {
    let mut s = world(false, LEAPS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [4.0, 0.0, 0.0]);
    let ev = cast(&mut s, p, "t_back");
    let h = hops(&ev, p);
    assert_eq!(h.len(), 1);
    assert!((h[0].0.abs() - std::f32::consts::PI).abs() < 0.1, "backward: {h:?}");
    run(&mut s, 0.6);
    assert!(s.players[&p].pos[0] < -8.0, "back the range: {:?}", s.players[&p].pos);
    assert!(s.has_cc(m, "root"), "the net caught the foe in front");
}

const STRIP: &str = r#"
[[ability]]
key = "t_strip"
name = "Strip"
school = "shaman"
slot = "close"
kind = "line"
range = [14.0, 14.0]
radius = [1.5, 1.5]
damage = [20.0, 20.0]
cast = 0.4
spell = 8042
"#;

#[test]
fn a_strip_winds_up_drawn_on_the_ground_then_strikes_only_what_is_in_it() {
    let mut s = world(false, STRIP);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let inside = mob(&mut s, "grunt", [10.0, 1.0, 0.0]);
    let outside = mob(&mut s, "grunt", [10.0, 4.0, 0.0]);
    let (hi, ho) = (s.mobs[&inside].health, s.mobs[&outside].health);
    let ev = cast(&mut s, p, "t_strip");
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(t), .. } if t.hero && t.width > 2.9)), "the strip drawn");
    assert_eq!(s.mobs[&inside].health, hi, "not yet");
    run(&mut s, 0.5);
    assert!(s.mobs[&inside].health < hi);
    assert_eq!(s.mobs[&outside].health, ho);
}

#[test]
fn a_stun_in_the_wind_up_loses_it() {
    let mut s = world(false, STRIP);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [8.0, 0.0, 0.0]);
    let hp = s.mobs[&m].health;
    cast(&mut s, p, "t_strip");
    s.apply_cc(p, m, "stun", 2.0, 0.0, 0.0, "test");
    run(&mut s, 0.6);
    assert_eq!(s.mobs[&m].health, hp, "lost");
}

const TRAP: &str = r#"
[[ability]]
key = "t_trap"
name = "Trap"
school = "hunter"
slot = "close"
kind = "trap"
range = [8.0, 8.0]
radius = [2.0, 2.0]
splash = 4.0
delay = 0.5
secs = 20.0
damage = [15.0, 15.0]
spell = 13812
object = 3072
burst_spell = 5162
effects = [{ kind = "knockback", amount = 5.0 }]
"#;

#[test]
fn a_trap_arms_then_springs_on_the_first_foe_to_step_near() {
    let mut s = world(false, TRAP);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [30.0, 0.0, 0.0]);
    let near = mob(&mut s, "grunt", [30.0, 3.0, 0.0]);
    let set = cast(&mut s, p, "t_trap");
    assert_eq!(s.traps.len(), 1);
    // The trap itself lies on the ground there, and no spell's look is laid at the spot.
    let trap = s.traps[0].obj.expect("the trap's object");
    assert!(matches!(s.objs[&trap].kind, ObjKind::Prop { display: 3072, .. }));
    assert!(dist2d(s.objs[&trap].pos, [8.0, 0.0, 0.0]) < 0.5, "{:?}", s.objs[&trap].pos);
    assert!(set.iter().any(|e| matches!(e, Event::ObjSpawned { id } if *id == trap)));
    assert!(!set.iter().any(|e| matches!(e, Event::Missile { .. })), "no spell at the spot");
    run(&mut s, 1.0);
    let hp = s.mobs[&m].health;
    // The foe walks onto it (the trap lies 8 yd ahead): it springs, its burst reaching the other.
    s.mobs.get_mut(&m).unwrap().pos = [8.5, 0.0, 0.0];
    s.mobs.get_mut(&near).unwrap().pos = [8.0, 3.0, 0.0];
    let hn = s.mobs[&near].health;
    let seen = run(&mut s, 0.1);
    assert!(s.traps.is_empty(), "sprung");
    assert!(!s.objs.contains_key(&trap), "the trap's gone");
    assert!(seen.iter().any(|e| matches!(e, Event::ObjGone { id } if *id == trap)));
    assert!(seen.iter().any(|e| matches!(e, Event::Missile { spell: 5162, target: Target::Point(_), .. })), "its burst at the spot");
    assert!(s.mobs[&m].health < hp && s.mobs[&near].health < hn, "the burst");
    assert!(bound(&s, near)[1] > 5.0, "thrown out from it");
}

const CIRCLE: &str = r#"
[[ability]]
key = "t_circle"
name = "Circle"
school = "warlock"
slot = "utility"
kind = "blink"
anchor = 10.0
cooldown = 20.0
cleanse = true
spell = 1856
"#;

#[test]
fn an_anchor_is_set_free_and_the_key_again_steps_back_to_it() {
    let mut s = world(false, CIRCLE);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().owned.insert("t_circle".into());
    s.slot_ability(p, "utility", "t_circle").unwrap();
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(s.anchors.contains_key(&p), "set");
    assert!(s.ready(p, s.content.ability("t_circle").unwrap()), "costing nothing");
    s.move_player(p, [15.0, 4.0, 0.0], 0.0, Moving::default());
    s.apply_cc(p, p, "root", 5.0, 0.0, 0.0, "test");
    let _ = s.step(DT);
    s.input(p, Input::Ability(key_of("utility"), false));
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.5, "back: {:?}", s.players[&p].pos);
    assert!(!s.has_cc(p, "root"), "out of the root");
    assert!(!s.ready(p, s.content.ability("t_circle").unwrap()), "now it cools down");
}

const BOLTS: &str = r#"
[[ability]]
key = "t_pierce"
name = "Pierce"
school = "hunter"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [10.0, 10.0]
pierce = 2
spell = 3044

[[ability]]
key = "t_chain"
name = "Chain"
school = "shaman"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [20.0, 20.0]
chain = 2
chain_range = 8.0
chain_falloff = 0.5
spell = 403

[[ability]]
key = "t_burst"
name = "Burst"
school = "mage"
slot = "ranged"
kind = "projectile"
speed = 40.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [12.0, 12.0]
splash = 4.0
spell = 133
"#;

#[test]
fn a_piercing_bolt_goes_through_a_chain_leaps_on_and_a_burst_catches_neighbours() {
    let mut s = world(false, BOLTS);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    let b = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let side = mob(&mut s, "grunt", [12.0, 5.0, 0.0]);
    let hp = |s: &Survival, m: Id| s.mobs[&m].health;
    let (ha, hb, hs) = (hp(&s, a), hp(&s, b), hp(&s, side));
    cast(&mut s, p, "t_pierce");
    run(&mut s, 1.0);
    assert!(hp(&s, a) < ha && hp(&s, b) < hb, "both in the line");
    assert_eq!(hp(&s, side), hs, "not the one aside");
    let (ha, hb, hs) = (hp(&s, a), hp(&s, b), hp(&s, side));
    cast(&mut s, p, "t_chain");
    run(&mut s, 1.0);
    assert!(hp(&s, a) < ha, "struck");
    assert!(hp(&s, b) < hb && hp(&s, side) < hs, "and it leapt on twice");
    assert!(ha - hp(&s, a) > hb - hp(&s, b), "weaker each leap");
    for (m, at) in [(a, [6.0, 0.0, 0.0]), (b, [6.0, 3.0, 0.0])] {
        let x = s.mobs.get_mut(&m).unwrap();
        x.state = MobState::Idle { wander_at: 1e9 };
        x.path.clear();
        x.pos = at;
    }
    let (ha, hb) = (hp(&s, a), hp(&s, b));
    cast(&mut s, p, "t_burst");
    run(&mut s, 1.0);
    assert!(hp(&s, a) < ha && hp(&s, b) < hb, "the burst caught the neighbour");
}

#[test]
fn a_shield_soaks_mending_heals_haste_speeds() {
    let mut s = world(false, "ability = []");
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.apply_cc(p, p, "shield", 5.0, 30.0, 0.0, "test");
    let hp = s.players[&p].health;
    s.hurt_player(p, 20);
    assert_eq!(s.players[&p].health, hp, "soaked");
    s.hurt_player(p, 20);
    assert_eq!(s.players[&p].health, hp - 10, "the rest through");
    s.apply_cc(p, p, "regen", 3.0, 0.05, 0.0, "test");
    run(&mut s, 2.1);
    assert!(s.players[&p].health > hp - 10, "mended");
    let before = s.speed_share(p);
    s.apply_cc(p, p, "haste", 3.0, 0.3, 0.0, "test");
    assert!(s.speed_share(p) > before * 1.25);
}

#[test]
fn a_veil_loses_the_creatures_and_lets_go_the_locks_until_they_strike() {
    let mut s = world(true, "ability = []");
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [25.0, 0.0, 0.0], std::f32::consts::PI);
    let m = mob(&mut s, "grunt", [-4.0, 2.0, 0.0]);
    s.engage(m, mobs::Foe::Hero(p));
    s.input(b, Input::Lock);
    assert_eq!(s.players[&b].lock, Some(p));
    s.apply_cc(p, p, "veil", 5.0, 0.0, 0.0, "test");
    assert_eq!(s.players[&b].lock, None, "the lock lets go");
    assert!(matches!(s.mobs[&m].state, MobState::Idle { .. }), "lost");
    run(&mut s, 0.5);
    assert!(matches!(s.mobs[&m].state, MobState::Idle { .. }), "and not found again: {:?}", s.mobs[&m].state);
    s.input(p, Input::Attack);
    s.input(p, Input::AttackUp);
    assert!(!s.has_cc(p, "veil"), "a swing ends it");
    assert!(s.players[&p].veil_strike_until > s.clock, "from behind, this once");
}

const GROUNDING: &str = r#"
[[ability]]
key = "t_ground"
name = "Grounding"
school = "shaman"
slot = "defense"
kind = "place"
range = [3.0, 3.0]
secs = 30.0
grounding = true
spell = 8177
totem = { mob = "earthbind_totem", radius = 6.0 }

[[ability]]
key = "t_bolt"
name = "Bolt"
school = "mage"
slot = "ranged"
kind = "projectile"
speed = 30.0
range = [30.0, 30.0]
radius = [0.3, 0.3]
damage = [20.0, 20.0]
spell = 133
"#;

#[test]
fn a_grounding_totem_swallows_a_foes_bolt_and_falls() {
    let mut s = world(true, GROUNDING);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [20.0, 0.0, 0.0], std::f32::consts::PI);
    cast(&mut s, b, "t_ground");
    assert_eq!(s.totems.len(), 1);
    let hp = s.players[&b].health;
    cast(&mut s, a, "t_bolt");
    run(&mut s, 1.5);
    assert_eq!(s.players[&b].health, hp, "swallowed");
    assert!(s.totems.is_empty(), "and the totem fell");
}

const EVASION: &str = r#"
[[ability]]
key = "t_evade"
name = "Evade"
school = "rogue"
slot = "defense"
kind = "stance"
secs = 2.0
spell = 5277
counter = { dodge = true }
"#;

#[test]
fn evasion_dodges_every_blow() {
    let mut s = world(true, EVASION);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.5, 0.0, 0.0], std::f32::consts::PI);
    cast(&mut s, b, "t_evade");
    let mut dodged = 0;
    for _ in 0..2 {
        s.input(a, Input::Attack);
        s.input(a, Input::AttackUp);
        let ev = run(&mut s, 0.8);
        dodged += ev.iter().filter(|e| matches!(e, Event::Swing { victim: Some(v), outcome: Outcome::Dodged, anim: false, .. } if *v == b)).count();
    }
    assert_eq!(dodged, 2);
}

const VORTEX: &str = r#"
[[ability]]
key = "t_vortex"
name = "Vortex"
school = "priest"
slot = "ultimate"
kind = "ground"
range = [15.0, 15.0]
radius = [8.0, 8.0]
delay = 0.3
linger = 3.0
linger_damage = 4.0
spell = 10
tick_effects = [{ kind = "pull", amount = 3.0 }]
"#;

#[test]
fn a_vortex_drags_what_stands_in_it_to_its_middle() {
    let mut s = world(false, VORTEX);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    s.players.get_mut(&p).unwrap().valor = 100.0;
    let m = mob(&mut s, "grunt", [15.0, 6.0, 0.0]);
    cast(&mut s, p, "t_vortex");
    run(&mut s, 1.5);
    let to = bound(&s, m);
    assert!(dist2d(to, [15.0, 0.0, 0.0]) < 4.0, "toward the middle: {to:?}");
}
