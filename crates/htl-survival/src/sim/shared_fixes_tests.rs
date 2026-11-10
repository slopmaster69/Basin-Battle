//! The engine-wide boss fixes from the Nov 8 review: debug damage moves health only, a second
//! form shows itself before it swings, a death stops the swing and takes its adds and throws
//! with it, lines in one place come apart, and a blow's look lands with the blow.

use super::*;
use crate::content::SpecialDef;
use crate::land::FlatLand;

const DT: f32 = 0.01;

fn world() -> (Survival, Id) {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    let mut s = Survival::bare(c, Box::new(FlatLand::default()), 23, 1_000_000.0);
    let h = s.join("shared", "Shared", None);
    s.players.get_mut(&h).unwrap().loadout.clear();
    s.move_player(h, [2.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    (s, h)
}

fn special(toml_text: &str) -> SpecialDef {
    toml::from_str(toml_text).expect("a special blow")
}

#[test]
fn debug_damage_never_builds_posture() {
    let (mut s, h) = world();
    let def = s.content.mob("dm_smite").unwrap();
    assert!(s.content.mobs[def].posture > 0, "Smite has a posture to break");
    let id = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    s.hurt_mob_as(id, h, 40, DEBUG_LABEL);
    assert_eq!(s.mobs[&id].posture, 0, "debug damage builds no posture");
    assert!(s.mobs[&id].health < s.mobs[&id].max_health, "but it does take health");
    s.hurt_mob_as(id, h, 40, "Melee");
    assert!(s.mobs[&id].posture > 0, "a hero's blow does");
}

#[test]
fn a_second_form_shows_itself_before_its_first_blow() {
    let (mut s, h) = world();
    let cookie = s.content.mob("dm_cookie").unwrap();
    let gorged = s.content.mob("dm_cookie_gorged").unwrap();
    assert!(s.content.mobs[cookie].then_after > 0.0);
    let id = s.spawn_mob(gorged, [0.0, 0.0, 0.0], None, false);
    s.rise(id, h, cookie);
    let risen = s.clock;
    assert!(s.mobs[&id].ready_at >= risen + SECOND_FORM_WAIT - 1e-3, "it waits before its first blow");
    while s.clock < risen + SECOND_FORM_WAIT - 0.1 {
        s.step(DT);
        assert!(!matches!(s.mobs[&id].state, MobState::Windup { .. }), "no swing {:.2} s after rising", s.clock - risen);
    }
}

#[test]
fn a_death_stops_its_swing_and_takes_its_adds_and_throws() {
    let (mut s, h) = world();
    let def = s.content.mob("dm_smite").unwrap();
    let boss = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    let add_def = s.content.mob("dm_deck_gunner").unwrap_or(def);
    let add = s.spawn_mob(add_def, [6.0, 0.0, 0.0], None, false);
    s.mobs.get_mut(&add).unwrap().summoned_by = Some(boss);
    s.missiles.push(combat::Missile { look: 0, owner: boss, from: [0.0, 0.0, 0.0], at: [2.0, 0.0, 0.0], radius: 2.0, damage: 50, blockable: false, heavy: false, land_at: s.clock + 1.0, mark: 0 });
    s.windup(boss, mobs::Foe::Hero(h), s.players[&h].pos);
    s.events.clear();
    let health = s.mobs[&boss].health;
    s.hurt_mob_as(boss, h, health, "Melee");
    assert!(matches!(s.mobs[&boss].state, MobState::Dead { .. }));
    assert!(s.events.iter().any(|e| matches!(e, Event::HoldPose { id, clip: 0, .. } if *id == boss)), "its swing stops as it falls");
    assert!(!s.mobs.contains_key(&add), "its add goes with it");
    assert!(s.missiles.iter().all(|m| m.owner != boss), "what it threw is spent");
    let before = s.players[&h].health;
    for _ in 0..150 { s.step(DT); }
    assert_eq!(s.players[&h].health, before, "nothing it threw lands after its death");
}

#[test]
fn lines_in_one_place_come_two_seconds_apart() {
    let (mut s, _) = world();
    let def = s.content.mob("dm_smite").unwrap();
    let a = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    let b = s.spawn_mob(def, [5.0, 0.0, 0.0], None, false);
    let far = s.spawn_mob(def, [200.0, 0.0, 0.0], None, false);
    s.events.clear();
    s.yell(a, "First!");
    s.yell(b, "Second!");
    s.yell(far, "Elsewhere!");
    let said = |ev: &[Event], text: &str| ev.iter().any(|e| matches!(e, Event::Yell { text: t, .. } if t == text));
    assert!(said(&s.events, "First!"));
    assert!(said(&s.events, "Elsewhere!"), "a line far off isn't held up");
    assert!(!said(&s.events, "Second!"), "the second waits its turn");
    let start = s.clock;
    let mut ev = Vec::new();
    while s.clock < start + LINE_GAP - 0.05 {
        ev.extend(s.step(DT));
    }
    assert!(!said(&ev, "Second!"), "not before the gap");
    for _ in 0..10 { ev.extend(s.step(DT)); }
    assert!(said(&ev, "Second!"), "said once the gap is up");
}

#[test]
fn a_blows_impact_look_lands_with_the_blow() {
    let (mut s, h) = world();
    let def = s.content.mob("dm_smite").unwrap();
    s.content.mobs[def].patterns.clear();
    s.content.mobs[def].special = Some(special("chance = 1.0\nkit = 1111\nimpact_kit = 4321\nheavy = true\nburst = \"FF0000\"\nwindup = 0.8"));
    let id = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    s.events.clear();
    s.windup(id, mobs::Foe::Hero(h), s.players[&h].pos);
    assert!(s.mobs[&id].special, "the special blow was chosen");
    let wound: Vec<Event> = std::mem::take(&mut s.events);
    let kit = |ev: &[Event], k: u32| ev.iter().any(|e| matches!(e, Event::Visual { kit, .. } if *kit == k));
    assert!(kit(&wound, 1111), "the tell plays as it winds up");
    assert!(!kit(&wound, 4321), "the impact doesn't");
    let mut ev = Vec::new();
    while matches!(s.mobs[&id].state, MobState::Windup { .. }) {
        ev.extend(s.step(DT));
    }
    assert!(kit(&ev, 4321), "the impact plays as it lands");
    assert!(kit(&ev, combat::KIT_DUST), "a heavy blow shakes the ground");
    assert!(ev.iter().any(|e| matches!(e, Event::GroundBurst { shape: "wedge", colour, .. } if colour == "FF0000")), "its shape flashes on the ground");
}

#[test]
fn a_small_lobbed_splash_is_drawn_with_the_markers_off() {
    let (mut s, h) = world();
    s.move_player(h, [8.0, 0.0, 0.0], std::f32::consts::PI, Moving::default());
    let def = s.content.mob("dm_smite").unwrap();
    s.content.mobs[def].patterns.clear();
    s.content.mobs[def].special = Some(special("chance = 1.0\nranged = true\nradius = 3.0\nspeed = 12.0\nspell = 7978\nwindup = 1.0"));
    let id = s.spawn_mob(def, [0.0, 0.0, 0.0], None, false);
    s.events.clear();
    s.windup(id, mobs::Foe::Hero(h), s.players[&h].pos);
    assert!(3.0 < GROUND_LOOK_MIN);
    assert!(s.events.iter().any(|e| matches!(e, Event::GroundBurst { shape: "ring", reach, .. } if (*reach - 3.0).abs() < 1e-3)),
        "its true size drawn on the ground");
}

/// A dodge with no keys held is a hop back. On a client that doesn't run its own dodges the
/// server throws it, and the throw stops where a body would: short of a drop behind (the
/// Juggernaut's open side, take 346 and 347) or a wall, never over them.
#[test]
fn a_hop_back_stops_short_of_a_drop_or_a_wall_behind() {
    struct Deck;
    impl crate::land::Land for Deck {
        // The deck ends 1 yd behind a hero at the origin facing +x; below it, the water.
        fn height(&self, x: f32, _: f32) -> f32 { if x < -1.0 { -16.0 } else { 0.0 } }
    }
    let hop = |s: &mut Survival, h: Id| -> f32 {
        s.players.get_mut(&h).unwrap().stamina = 100.0;
        s.events.clear();
        s.input(h, Input::Dodge);
        s.events.iter().find_map(|e| match e { Event::Hop { player, speed, .. } if *player == h => Some(*speed), _ => None }).expect("a hop")
    };
    let (mut s, h) = world();
    s.set_local_moves(h, false);
    s.move_player(h, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let open = hop(&mut s, h);
    assert!((open - s.content.dodge.back_step[0]).abs() < 1e-3, "on open ground, the whole step back");
    run_to(&mut s, 2.0);
    s.land = Box::new(Deck);
    let edge = hop(&mut s, h);
    let air = super::moves::airtime(s.content.dodge.back_step[1]);
    assert!(edge * air <= 1.0 + 1e-3, "no farther than the deck's edge: {:.2} yd", edge * air);
}

fn run_to(s: &mut Survival, secs: f32) {
    for _ in 0..(secs / DT).round() as usize {
        s.step(DT);
    }
}
