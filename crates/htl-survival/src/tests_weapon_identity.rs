//! Each weapon's identity (docs/weapon-identities.md): what sets it apart in a fight.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::{Foe, MobState};
use crate::sim::*;

fn world() -> (Survival, Id) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 5, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    let p = s.join("ayla", "Ayla", None);
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    (s, p)
}

fn wield(s: &mut Survival, p: Id, weapon: &str) {
    let i = s.content.item(weapon).unwrap_or_else(|| panic!("no item {weapon}"));
    s.players.get_mut(&p).unwrap().weapon = i;
}

/// A creature 2 yd in front of the hero that stands still and never strikes back.
fn foe(s: &mut Survival, key: &str) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, [2.0, 0.0, 0.0], None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1.0e9 };
    m.facing = std::f32::consts::PI;
    m.ready_at = 1.0e9;
    id
}

/// One light swing's events, and a moment after.
fn swing(s: &mut Survival, p: Id) -> Vec<Event> {
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(0.02));
    }
    ev
}

fn hits_on(ev: &[Event], m: Id) -> Vec<(u32, Outcome)> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { victim: Some(v), damage, outcome, style, .. } if *v == m && !matches!(style, Blow::Dot) => Some((*damage, *outcome)),
            _ => None,
        })
        .collect()
}

#[test]
fn every_weapon_but_a_staff_has_an_identity() {
    let c = Content::dev();
    for w in c.weapons.iter().filter(|w| !w.guard.contains("ward") && !w.key.ends_with("staff")) {
        let has = w.parry > 0.0 || w.riposte > 0.0 || w.bleed > 0.0 || w.posture > 0.0 || w.backstab > 0.0 || w.through_guard || w.swing_poise || w.execute > 0.0 || w.stagger > 0.0;
        assert!(has, "{} has nothing of its own", w.key);
    }
}

#[test]
fn an_axes_hit_bleeds() {
    let (mut s, p) = world();
    wield(&mut s, p, "axe");
    // (A foe with no shield to take it on.)
    let m = foe(&mut s, "dm_woodcutter");
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, _)| *d > 0), "it hit");
    let after = s.mobs[&m].health;
    let mut ticks = 0;
    for _ in 0..150 {
        for e in s.step(0.02) {
            if matches!(e, Event::Swing { victim: Some(v), style: Blow::Dot, .. } if v == m) {
                ticks += 1;
            }
        }
    }
    assert!(ticks >= 2, "it bled: {ticks} ticks");
    assert!(s.mobs[&m].health < after, "and lost health to it");
}

#[test]
fn a_spear_goes_through_a_raised_shield_a_sword_doesnt() {
    let (mut s, p) = world();
    // A shield-bearer that blocks every light blow it faces.
    let pirate = s.content.mob("dm_pirate").unwrap();
    assert!(s.content.mobs[pirate].shield != 0, "the pirate carries a shield");
    s.content.mobs[pirate].block = 1.0;
    wield(&mut s, p, "sword");
    let m = foe(&mut s, "dm_pirate");
    // Between its blows, shield to the hero, standing where it is.
    let hold = |s: &mut Survival| {
        let x = s.mobs.get_mut(&m).unwrap();
        x.pos = [2.0, 0.0, 0.0];
        x.facing = std::f32::consts::PI;
        x.state = MobState::Recover { foe: Foe::Hero(p), until: 1.0e9 };
    };
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().all(|(_, o)| *o == Outcome::Blocked), "the sword is blocked: {:?}", hits_on(&ev, m));
    for _ in 0..50 {
        let _ = s.step(0.02);
    }
    wield(&mut s, p, "zanjin_spear");
    hold(&mut s);
    let ev = swing(&mut s, p);
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "the spear goes through: {:?}", hits_on(&ev, m));
}

#[test]
fn the_executioners_axe_hits_twice_as_hard_on_a_foe_near_its_end() {
    let (mut s, p) = world();
    wield(&mut s, p, "warlord_axe");
    let m = foe(&mut s, "dm_woodcutter");
    let full = hits_on(&swing(&mut s, p), m).iter().map(|h| h.0).max().unwrap_or(0);
    for _ in 0..60 {
        let _ = s.step(0.02);
    }
    let max = s.mobs[&m].max_health;
    s.mobs.get_mut(&m).unwrap().health = max / 4;
    s.mobs.get_mut(&m).unwrap().max_health = 100_000.max(max);
    // (A share kept under a third whatever its health was.)
    let low = s.mobs[&m].max_health / 5;
    s.mobs.get_mut(&m).unwrap().health = low;
    let near = hits_on(&swing(&mut s, p), m).iter().map(|h| h.0).max().unwrap_or(0);
    let w = s.content.weapon("warlord_axe").unwrap();
    assert!(near as f32 >= w.damage[0] as f32 * 2.0 - 1.0, "near its end: {near} (fresh {full})");
}

// --- the off hand -----------------------------------------------------------------------------------

#[test]
fn with_two_blades_the_right_click_swings_the_off_hand_instead_of_guarding() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("axe");
    let m = foe(&mut s, "dm_woodcutter");
    s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
    s.input(p, Input::Guard);
    assert!(matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Off), "an off-hand blow: {:?}", s.players[&p].action);
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(0.02));
    }
    assert!(hits_on(&ev, m).iter().any(|(d, o)| *o == Outcome::Hit && *d > 0), "it lands");
    assert!(!matches!(s.players[&p].action, Action::Guard), "and no guard comes up");
}

#[test]
fn left_and_right_alternating_four_times_ends_in_a_finisher() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("sword");
    let m = foe(&mut s, "dm_woodcutter");
    s.mobs.get_mut(&m).unwrap().health = 100_000;
    s.mobs.get_mut(&m).unwrap().max_health = 100_000;
    let mut damage = Vec::new();
    for k in 0..4 {
        s.players.get_mut(&p).unwrap().stamina = 100.0;
        s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
        s.input(p, if k % 2 == 0 { Input::Attack } else { Input::Guard });
        let mut ev = Vec::new();
        // Each blow pressed again late in the last one's recovery, as a run is.
        for _ in 0..60 {
            ev.extend(s.step(0.02));
            if matches!(&s.players[&p].action, Action::Swing { t, strike, .. } if *t >= strike.windup + strike.active + strike.recovery * 0.75) {
                break;
            }
        }
        damage.push(hits_on(&ev, m).iter().map(|h| h.0).max().unwrap_or(0));
    }
    let w = s.content.weapon("sword").unwrap();
    assert!(damage[..3].iter().all(|d| *d > 0), "each blow lands: {damage:?}");
    assert!(damage[3] as f32 > w.damage[1] as f32 * 1.5, "the fourth, both blades, hits far harder: {damage:?}");
}

#[test]
fn a_weapon_alone_guards_with_its_blade_and_parries_wider() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    let shielded = s.guard_of(&s.players[&p]).key.clone();
    s.players.get_mut(&p).unwrap().off_empty = true;
    assert_eq!(s.guard_of(&s.players[&p]).key, crate::sim::ONE_HAND_GUARD, "its own blade (was {shielded})");
    assert!(s.one_hand_alone(&s.players[&p]));
}

#[test]
fn a_save_keeps_the_off_hand() {
    let (mut s, p) = world();
    wield(&mut s, p, "sword");
    s.players.get_mut(&p).unwrap().off_weapon = s.content.item("axe");
    let saved = crate::save::save_player(&s.content, &s.players[&p]);
    assert_eq!(saved.off_hand, "axe");
    let (mut t, _) = world();
    let q = t.join("bren", "Bren", Some(&saved));
    assert_eq!(t.players[&q].off_weapon, t.content.item("axe"), "the axe is back in the off hand");
}
