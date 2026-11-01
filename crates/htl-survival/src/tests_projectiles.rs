//! The projectile mechanics, one rule each, on abilities made for the test: a boomerang that
//! comes back through what it struck going out, a lob that arcs over the foe in the way and bursts
//! where it lands, a sticky fuse that holds fire and then bursts round whoever (or wherever) it
//! stuck to; a plain projectile is as it was, and the rules say what each needs.

use crate::content::{AbilityDef, Content};
use crate::land::FlatLand;
use crate::sim::abilities::Stage;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];
/// The sticky one's look on whoever it's stuck to.
const FUSE_AURA: u32 = 1784;

const PROJECTILES: &str = r#"
[[ability]]
key = "t_boomerang"
name = "Boomerang"
school = "warrior"
slot = "ranged"
kind = "projectile"
speed = 20.0
range = [16.0, 16.0]
radius = [0.3, 0.3]
damage = [10.0, 10.0]
returns = true
return_damage = 0.5
spell = 16075

[[ability]]
key = "t_lob"
name = "Lob"
school = "hunter"
slot = "ranged"
kind = "projectile"
speed = 20.0
range = [20.0, 20.0]
radius = [0.3, 0.3]
damage = [12.0, 12.0]
lob = true
splash = 4.0
spell = 13812

[[ability]]
key = "t_sticky"
name = "Sticky"
school = "rogue"
slot = "ranged"
kind = "projectile"
speed = 30.0
range = [20.0, 20.0]
radius = [0.3, 0.3]
damage = [12.0, 12.0]
fuse = 1.5
fuse_aura = 1784
splash = 4.0
burst_spell = 5162
spell = 133

[[ability]]
key = "t_plain"
name = "Plain"
school = "mage"
slot = "ranged"
kind = "projectile"
speed = 30.0
range = [20.0, 20.0]
radius = [0.3, 0.3]
damage = [10.0, 10.0]
spell = 133
"#;

/// The built-in rules with these abilities (TOML, `[[ability]]` tables) added, off the board.
fn world(extra: &str) -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    for a in abilities(extra) {
        c.abilities.push(a);
    }
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

fn abilities(toml_text: &str) -> Vec<AbilityDef> {
    #[derive(serde::Deserialize)]
    struct Extra {
        ability: Vec<AbilityDef>,
    }
    let e: Extra = toml::from_str(toml_text).expect("test abilities");
    e.ability
        .into_iter()
        .map(|mut a| {
            a.bench = true;
            a
        })
        .collect()
}

fn hero(s: &mut Survival, name: &str, pos: Vec3, facing: f32) -> Id {
    let id = s.join(&name.to_lowercase(), name, None);
    s.move_player(id, pos, facing, Moving::default());
    let _ = s.step(0.0);
    id
}

/// A boar (it notices no one till struck), standing still.
fn boar(s: &mut Survival, pos: Vec3) -> Id {
    let def = s.content.mob("boar").unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    let m = s.mobs.get_mut(&id).unwrap();
    m.state = MobState::Idle { wander_at: 1e9 };
    m.pos = pos;
    m.facing = std::f32::consts::PI;
    id
}

/// Creatures held where they are (struck, they'd come for the hero).
fn pin(s: &mut Survival, pins: &[(Id, Vec3)]) {
    for (m, at) in pins {
        if let Some(x) = s.mobs.get_mut(m) {
            x.state = MobState::Idle { wander_at: 1e9 };
            x.path.clear();
            x.pos = *at;
        }
    }
}

fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        pin(s, pins);
        out.extend(s.step(DT));
    }
    pin(s, pins);
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

/// The damage of each of `p`'s blows that landed on `m`.
fn hits(ev: &[Event], p: Id, m: Id) -> Vec<u32> {
    ev.iter()
        .filter_map(|e| match e {
            Event::Swing { attacker, victim: Some(v), damage, outcome: Outcome::Hit, .. } if *attacker == p && *v == m => Some(*damage),
            _ => None,
        })
        .collect()
}

fn hp(s: &Survival, m: Id) -> u32 {
    s.mobs[&m].health
}

#[test]
fn a_boomerang_strikes_each_foe_going_out_and_again_coming_back() {
    let mut s = world(PROJECTILES);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = boar(&mut s, [6.0, 0.0, 0.0]);
    let far = boar(&mut s, [11.0, 0.0, 0.0]);
    let aside = boar(&mut s, [8.0, 5.0, 0.0]);
    let pins = [(near, [6.0, 0.0, 0.0]), (far, [11.0, 0.0, 0.0]), (aside, [8.0, 5.0, 0.0])];
    let mut ev = cast(&mut s, p, "t_boomerang");
    assert!(
        ev.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Point(end), .. } if *caster == p && end[0] > 15.0)),
        "its look flies out to its full range, through the foes"
    );
    ev.extend(run_pinned(&mut s, 3.0, &pins));
    let turn = ev
        .iter()
        .position(|e| matches!(e, Event::MissileFrom { caster, target: Target::Unit(t), .. } if *caster == p && *t == p))
        .expect("it turns, its look flown from there back to the thrower");
    let (out, back) = ev.split_at(turn);
    for m in [near, far] {
        assert_eq!(hits(out, p, m).len(), 1, "struck once going out");
        assert_eq!(hits(back, p, m).len(), 1, "and once coming back");
    }
    assert!(hits(back, p, near)[0] < hits(out, p, near)[0], "the way back at its share of the damage");
    assert!(hits(&ev, p, aside).is_empty(), "not the one aside");
    assert!(s.projectiles.is_empty(), "caught at the end");
    let anchor = ev[turn..].iter().find_map(|e| match e {
        Event::MissileFrom { anchor, .. } => Some(*anchor),
        _ => None,
    });
    assert!(ev.iter().any(|e| matches!(e, Event::ObjGone { id } if Some(*id) == anchor)), "the look's anchor gone with it");
}

#[test]
fn a_lob_flies_over_a_foe_in_the_way_and_bursts_where_it_lands() {
    let mut s = world(PROJECTILES);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    // Beside the reticle's line, in the hands' (a thrown bolt would strike it).
    let in_way = boar(&mut s, [6.0, 0.5, 0.0]);
    let there = boar(&mut s, [14.0, 0.0, 0.0]);
    let beside = boar(&mut s, [14.0, 2.5, 0.0]);
    let pins = [(in_way, [6.0, 0.5, 0.0]), (there, [14.0, 0.0, 0.0]), (beside, [14.0, 2.5, 0.0])];
    let (hw, ht, hb) = (hp(&s, in_way), hp(&s, there), hp(&s, beside));
    // Aimed at the ground 14 yd ahead.
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(14.0));
    let mut ev = cast(&mut s, p, "t_lob");
    let (ring, mark) = ev
        .iter()
        .find_map(|e| match e {
            Event::Mark { id, pos, radius, tele: Some(t), .. } if t.at.is_some() && (*radius - 4.0).abs() < 0.01 => Some((*pos, *id)),
            _ => None,
        })
        .expect("its landing ring, for everyone, before it lands");
    assert!(dist2d(ring, [14.0, 0.0, 0.0]) < 1.5, "at the spot aimed at: {ring:?}");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Point(at), .. } if *caster == p && dist2d(*at, ring) < 0.1)), "its look flown there");
    ev.extend(run_pinned(&mut s, 0.3, &pins));
    assert_eq!((hp(&s, there), hp(&s, beside)), (ht, hb), "still in the air");
    ev.extend(run_pinned(&mut s, 0.8, &pins));
    assert!(hp(&s, there) < ht && hp(&s, beside) < hb, "landed: it burst there");
    assert_eq!(hp(&s, in_way), hw, "over the foe in the way");
    assert!(ev.iter().any(|e| matches!(e, Event::MarkGone { id } if *id == mark)), "its ring gone as it lands");
    assert!(s.projectiles.is_empty());
}

#[test]
fn a_sticky_fuse_holds_fire_then_bursts_round_whoever_it_stuck_to() {
    let mut s = world(PROJECTILES);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let stuck = boar(&mut s, [8.0, 0.0, 0.0]);
    let left_behind = boar(&mut s, [9.5, 2.0, 0.0]);
    let waiting = boar(&mut s, [9.5, -8.0, 0.0]);
    let mut pins = [(stuck, [8.0, 0.0, 0.0]), (left_behind, [9.5, 2.0, 0.0]), (waiting, [9.5, -8.0, 0.0])];
    let (hs, hl, hw) = (hp(&s, stuck), hp(&s, left_behind), hp(&s, waiting));
    let mut ev = cast(&mut s, p, "t_sticky");
    ev.extend(run_pinned(&mut s, 0.5, &pins));
    assert!(hits(&ev, p, stuck).is_empty() && hp(&s, stuck) == hs, "stuck, it does nothing yet");
    assert!(s.projectiles.iter().any(|x| matches!(x.stage, Stage::Stuck { on: Some(u), .. } if u == stuck)), "on it");
    assert!(s.auras_of(stuck).contains(&FUSE_AURA), "its fuse shows on it");
    assert!(ev.iter().any(|e| matches!(e, Event::Auras { id } if *id == stuck)));
    // It walks off; the fuse goes with it.
    pins[0].1 = [8.0, -8.0, 0.0];
    let more = run_pinned(&mut s, 1.6, &pins);
    let ring = more
        .iter()
        .find_map(|e| match e {
            Event::Mark { tele: Some(t), .. } if t.at.is_none() => Some(*t),
            _ => None,
        })
        .expect("a ring under it for its last second");
    assert_eq!(ring.mob, stuck, "following it");
    assert!(ring.secs <= 1.0 + 1.0e-3 && (ring.reach - 4.0).abs() < 0.01);
    assert!(hp(&s, stuck) < hs && hp(&s, waiting) < hw, "it went off round it, where it is now");
    assert_eq!(hp(&s, left_behind), hl, "not where it was struck");
    assert!(!s.auras_of(stuck).contains(&FUSE_AURA), "its fuse gone off it");
    assert!(more.iter().any(|e| matches!(e, Event::Missile { spell: 5162, target: Target::Point(at), .. } if dist2d(*at, [8.0, -8.0, 0.0]) < 0.5)), "its burst's look there");
    assert!(s.projectiles.is_empty());
}

#[test]
fn a_sticky_fuse_on_the_ground_sits_there_and_goes_off_there() {
    let mut s = world(PROJECTILES);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let by = boar(&mut s, [8.0, 3.0, 0.0]);
    let pins = [(by, [8.0, 3.0, 0.0])];
    let hb = hp(&s, by);
    // Aimed at the ground 8 yd ahead.
    s.set_aim_line(p, 0.0, -(1.9f32).atan2(8.0));
    let mut ev = cast(&mut s, p, "t_sticky");
    ev.extend(run_pinned(&mut s, 0.5, &pins));
    assert_eq!(hp(&s, by), hb, "nothing yet");
    let at = s
        .projectiles
        .iter()
        .find_map(|x| match x.stage {
            Stage::Stuck { on: None, at, .. } => Some(at),
            _ => None,
        })
        .expect("it sits on the ground");
    assert!(dist2d(at, [8.0, 0.0, 0.0]) < 1.5 && at[2].abs() < 0.5, "where it met the ground: {at:?}");
    ev.extend(run_pinned(&mut s, 1.6, &pins));
    assert!(hp(&s, by) < hb, "it went off there, catching the foe beside it");
    assert!(ev.iter().any(|e| matches!(e, Event::Mark { tele: Some(t), .. } if t.at.is_some())), "its ring there for its last second");
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { spell: 5162, target: Target::Point(x), .. } if dist2d(*x, at) < 0.5)), "its burst's look there");
    assert!(s.projectiles.is_empty());
}

#[test]
fn a_plain_projectile_still_stops_at_the_first_foe() {
    let mut s = world(PROJECTILES);
    let d = &s.content.abilities[s.content.ability("t_plain").unwrap()];
    assert!(!d.returns && !d.lob && d.fuse == 0.0 && d.fuse_aura == 0 && d.return_damage == 1.0, "none of it unless asked");
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let a = boar(&mut s, [6.0, 0.0, 0.0]);
    let b = boar(&mut s, [11.0, 0.0, 0.0]);
    let pins = [(a, [6.0, 0.0, 0.0]), (b, [11.0, 0.0, 0.0])];
    let mut ev = cast(&mut s, p, "t_plain");
    ev.extend(run_pinned(&mut s, 1.5, &pins));
    assert_eq!(hits(&ev, p, a).len(), 1, "the first struck");
    assert!(hits(&ev, p, b).is_empty(), "and no further");
    assert!(!ev.iter().any(|e| matches!(e, Event::MissileFrom { .. })), "nothing comes back");
    assert!(s.projectiles.is_empty());
}

#[test]
fn the_rules_say_what_each_projectile_mechanic_needs() {
    let mut c = Content::dev();
    c.abilities.extend(abilities(PROJECTILES));
    c.validate_abilities().expect("the test's own load");
    let head = "[[ability]]\nkey = \"t_bad\"\nname = \"Bad\"\nschool = \"mage\"\nslot = \"ranged\"\n";
    let bad = [
        ("a lob with no burst", "kind = \"projectile\"\nspeed = 20.0\nlob = true"),
        ("a fuse with no burst", "kind = \"projectile\"\nspeed = 20.0\nfuse = 1.0"),
        ("a nova that comes back", "kind = \"nova\"\nreturns = true"),
        ("ground lobbed", "kind = \"ground\"\nlob = true\nsplash = 3.0"),
        ("a trap with a fuse", "kind = \"trap\"\nradius = [2.0, 2.0]\nsecs = 10.0\nfuse = 1.0\nsplash = 3.0"),
        ("a boomerang lobbed", "kind = \"projectile\"\nspeed = 20.0\nreturns = true\nlob = true\nsplash = 3.0"),
        ("a lob that sticks", "kind = \"projectile\"\nspeed = 20.0\nlob = true\nfuse = 1.0\nsplash = 3.0"),
    ];
    for (what, body) in bad {
        let mut c = Content::dev();
        c.abilities.extend(abilities(&format!("{head}{body}\n")));
        assert!(c.validate_abilities().is_err(), "{what} is refused");
    }
}
