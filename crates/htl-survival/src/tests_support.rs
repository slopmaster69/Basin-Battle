//! Help for friends, one rule each, on abilities made for the test: an `ally` spell picks the
//! friend on the reticle (or near it, or the caster) and gives them its heal and its boons but none
//! of its harm; a `beam` sweeps a strip from the caster as they turn, striking every foe in it and
//! mending every friend, until its time is up, a stun, a silence or its key again. Help reaches
//! every other hero outside Siege, friendly fire or not (a bolt that would heal and hurt a co-op
//! friend heals them); in Siege, partners only.

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
    add_abilities(&mut c, extra);
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// Abilities (TOML `[[ability]]` tables) added to the rules, off the board.
fn add_abilities(c: &mut Content, extra: &str) {
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

fn hits_on(events: &[Event], victim: Id) -> usize {
    events.iter().filter(|e| matches!(e, Event::Swing { victim: Some(v), damage, anim: false, .. } if *v == victim && *damage > 0)).count()
}

/// Its look went from the caster to that unit.
fn looked_at(events: &[Event], caster: Id, spell: u32, unit: Id) -> bool {
    events.iter().any(|e| matches!(e, Event::Missile { caster: c, spell: s, target: Target::Unit(t) } if *c == caster && *s == spell && *t == unit))
}

fn halve(s: &mut Survival, p: Id) -> u32 {
    let p = s.players.get_mut(&p).unwrap();
    p.health = p.max_health / 2;
    p.health
}

fn channelling(s: &Survival, p: Id) -> bool {
    matches!(s.players[&p].action, Action::Channel { .. })
}

const MEND: &str = r#"
[[ability]]
key = "t_mend"
name = "Mend"
school = "priest"
slot = "defense"
kind = "ally"
range = [30.0, 30.0]
ally_heal = 0.2
spell = 2061
effects = [{ kind = "shield", amount = 30.0, secs = [5.0, 5.0] }, { kind = "stun", secs = [2.0, 2.0] }]
"#;

#[test]
fn help_aimed_at_a_friend_heals_and_shields_them_not_the_caster() {
    let mut s = world(false, MEND);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let (ha, hb) = (halve(&mut s, a), halve(&mut s, b));
    let ev = cast(&mut s, a, "t_mend");
    assert!(looked_at(&ev, a, 2061, b), "its look from her to him");
    let max = s.players[&b].max_health;
    assert_eq!(s.players[&b].health, hb + (max as f32 * 0.2).round() as u32, "a fifth of his health");
    assert!(s.has_cc(b, "shield"), "his shield");
    assert_eq!(s.players[&a].health, ha, "not hers");
    assert!(!s.has_cc(a, "shield"), "no shield on her");
    assert!(ev.iter().any(|e| matches!(e, Event::Flash { player, text } if *player == a && text.contains("Bren"))), "she's told");
}

#[test]
fn help_aimed_at_nothing_finds_a_friend_near_the_reticle_or_lands_on_the_caster() {
    // A friend behind her, far off the reticle: it's hers.
    let mut s = world(false, MEND);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [-10.0, 0.0, 0.0], 0.0);
    let (ha, hb) = (halve(&mut s, a), halve(&mut s, b));
    let ev = cast(&mut s, a, "t_mend");
    assert!(looked_at(&ev, a, 2061, a), "on herself");
    assert!(s.players[&a].health > ha && s.has_cc(a, "shield"), "healed and shielded");
    assert_eq!(s.players[&b].health, hb);
    assert!(!s.has_cc(b, "shield"));
    // Alone, the same.
    let mut s = world(false, MEND);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let ha = halve(&mut s, a);
    cast(&mut s, a, "t_mend");
    assert!(s.players[&a].health > ha && s.has_cc(a, "shield"), "solo: her own");
    // A friend a little wide of the line (missed by its trace, within the cone) is still found.
    let mut s = world(false, MEND);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 2.0, 0.0], 0.0);
    let hb = halve(&mut s, b);
    let ev = cast(&mut s, a, "t_mend");
    assert!(looked_at(&ev, a, 2061, b), "on him");
    assert!(s.players[&b].health > hb && s.has_cc(b, "shield"));
}

#[test]
fn with_friendly_fire_on_help_still_reaches_a_co_op_friend() {
    // Friendly fire decides only who can be harmed: outside Siege every other hero can be helped.
    let mut s = world(true, MEND);
    assert!(s.content.pvp.friendly_fire);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    let (ha, hb) = (halve(&mut s, a), halve(&mut s, b));
    let ev = cast(&mut s, a, "t_mend");
    assert!(looked_at(&ev, a, 2061, b), "on him");
    assert!(s.players[&b].health > hb && s.has_cc(b, "shield"), "healed and shielded");
    assert_eq!(s.players[&a].health, ha, "not her");
    assert!(!s.has_cc(b, "stun"), "and none of its harm");
}

/// A Siege world (the built-in rules' hall and match) with these abilities added.
fn siege_world(extra: &str) -> Survival {
    let mut c = Content::dev();
    c.dev.full_tree = false;
    add_abilities(&mut c, extra);
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), 1, 1_000_000.0);
    s.events.clear();
    s
}

#[test]
fn in_siege_help_reaches_a_partner_but_never_a_rival() {
    let mut s = siege_world(MEND);
    let mut ids = Vec::new();
    for n in ["Ayla", "Bren", "Cass", "Dorn"] {
        let id = s.join(&n.to_lowercase(), n, None);
        s.siege_arrive(id);
        ids.push(id);
    }
    for id in &ids {
        s.siege_join(*id, "2v2").unwrap();
    }
    let countdown = Content::dev().siege.unwrap().countdown;
    run_still(&mut s, &[], countdown + 0.2);
    let sg = s.siege.as_ref().unwrap();
    let (x, y, z, w) = (sg.camps[0].heroes[0], sg.camps[0].heroes[1], sg.camps[1].heroes[0], sg.camps[1].heroes[1]);
    assert!(s.hostile(x, z) && !s.hostile(x, y), "rivals and partners");
    // Her partner ahead of her: him.
    let at = s.players[&x].pos;
    let away = |k: f32| [at[0] - 60.0, at[1] + k * 5.0, at[2]];
    s.move_player(x, at, 0.0, Moving::default());
    s.move_player(y, [at[0] + 10.0, at[1], at[2]], 0.0, Moving::default());
    s.move_player(z, away(1.0), 0.0, Moving::default());
    s.move_player(w, away(2.0), 0.0, Moving::default());
    let hy = halve(&mut s, y);
    let ev = cast(&mut s, x, "t_mend");
    assert!(looked_at(&ev, x, 2061, y), "her partner");
    assert!(s.players[&y].health > hy && s.has_cc(y, "shield"));
    // A rival there instead: never him, hers.
    s.move_player(y, away(3.0), 0.0, Moving::default());
    s.move_player(z, [at[0] + 10.0, at[1], at[2]], 0.0, Moving::default());
    let hz = halve(&mut s, z);
    let ev = cast(&mut s, x, "t_mend");
    assert!(looked_at(&ev, x, 2061, x), "a rival isn't helped: hers");
    assert_eq!(s.players[&z].health, hz);
    assert!(!s.has_cc(z, "shield"));
}

const SHOCK: &str = r#"
[[ability]]
key = "t_shock"
name = "Shock"
school = "paladin"
slot = "ranged"
kind = "projectile"
damage = [14.0, 14.0]
range = [28.0, 28.0]
radius = [0.45, 0.45]
speed = 30.0
ally_heal = 0.12
spell = 2812
"#;

#[test]
fn with_friendly_fire_on_a_bolt_that_heals_and_hurts_heals_a_co_op_friend() {
    let mut s = world(true, SHOCK);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [8.0, 0.0, 0.0], 0.0);
    let hb = halve(&mut s, b);
    let mut ev = cast(&mut s, a, "t_shock");
    ev.extend(run_still(&mut s, &[], 1.0));
    let max = s.players[&b].max_health;
    assert_eq!(s.players[&b].health, hb + (max as f32 * 0.12).round() as u32, "healed, not struck");
    assert_eq!(hits_on(&ev, b), 0, "no blow on him");
}

#[test]
fn help_for_a_friend_gives_them_none_of_its_harm() {
    let mut s = world(false, MEND);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [10.0, 0.0, 0.0], 0.0);
    cast(&mut s, a, "t_mend");
    assert!(s.has_cc(b, "shield"), "the boon");
    assert!(!s.has_cc(b, "stun"), "not the stun");
    assert!(!s.has_cc(a, "stun"), "nor on her");
    assert!(!s.cc_log_of(b).iter().any(|l| l.contains("stun")), "{:?}", s.cc_log_of(b));
}

const BEAM: &str = r#"
[[ability]]
key = "t_beam"
name = "Beam"
school = "priest"
slot = "ranged"
kind = "beam"
range = [16.0, 16.0]
radius = [1.0, 1.0]
secs = 2.0
tick = 0.25
damage = [8.0, 8.0]
ally_heal = 0.1
spell = 15237
aura = 10060
tick_spell = 25345
"#;

#[test]
fn a_beam_strikes_every_foe_in_its_strip_mends_a_friend_follows_the_turn_and_ends() {
    let mut s = world(false, BEAM);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let near = mob(&mut s, "grunt", [5.0, 0.0, 0.0]);
    let far = mob(&mut s, "grunt", [12.0, 0.5, 0.0]);
    let side = mob(&mut s, "grunt", [8.0, 5.0, 0.0]);
    let north = mob(&mut s, "grunt", [0.0, 8.0, 0.0]);
    let all = [near, far, side, north];
    let f = hero(&mut s, "Bren", [9.0, -0.5, 0.0], 0.0);
    let hf = halve(&mut s, f);
    let mut ev = cast(&mut s, p, "t_beam");
    assert!(channelling(&s, p), "held");
    assert!(s.auras_of(p).contains(&10060), "its aura on her");
    ev.extend(run_still(&mut s, &all, 1.0));
    assert!(hits_on(&ev, near) >= 3, "the near one, each tick: {}", hits_on(&ev, near));
    assert!(hits_on(&ev, far) >= 3, "and the far one: {}", hits_on(&ev, far));
    assert_eq!(hits_on(&ev, side), 0, "not the one off to the side");
    assert_eq!(hits_on(&ev, north), 0, "nor the one to the north, yet");
    assert!(s.players[&f].health > hf, "the friend in it mended");
    assert!(looked_at(&ev, p, 25345, near), "its look reaches the first foe in it");
    // She turns north: the strip turns with her.
    s.move_player(p, ORIGIN, std::f32::consts::FRAC_PI_2, Moving::default());
    let ev = run_still(&mut s, &all, 0.6);
    assert!(hits_on(&ev, north) >= 2, "now the north one: {}", hits_on(&ev, north));
    assert_eq!(hits_on(&ev, near) + hits_on(&ev, far), 0, "and no longer those");
    // Its time up, it ends.
    run_still(&mut s, &all, 0.8);
    assert!(!channelling(&s, p), "over after its secs");
    assert!(!s.auras_of(p).contains(&10060), "its aura gone");
    let ev = run_still(&mut s, &all, 0.5);
    assert_eq!(hits_on(&ev, north), 0, "nothing more");
}

#[test]
fn with_friendly_fire_on_a_healing_beam_mends_a_co_op_friend_and_never_burns_them() {
    let mut s = world(true, BEAM);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let f = hero(&mut s, "Bren", [8.0, 0.0, 0.0], 0.0);
    let m = mob(&mut s, "grunt", [12.0, 0.0, 0.0]);
    let hf = halve(&mut s, f);
    let mut ev = cast(&mut s, p, "t_beam");
    ev.extend(run_still(&mut s, &[m], 1.0));
    assert!(s.players[&f].health > hf, "mended");
    assert_eq!(hits_on(&ev, f), 0, "never burned");
    assert!(hits_on(&ev, m) >= 3, "the foe beyond him still is: {}", hits_on(&ev, m));
}

const BLAST: &str = r#"
[[ability]]
key = "t_blast"
name = "Blast"
school = "priest"
slot = "close"
kind = "nova"
damage = [10.0, 10.0]
radius = [7.0, 7.0]
spell = 15237
"#;

#[test]
fn with_friendly_fire_on_holy_nova_heals_a_co_op_friend_and_never_strikes_them() {
    let mut s = world(true, BLAST);
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let hb = halve(&mut s, b);
    let ev = cast(&mut s, a, "holy_nova");
    let max = s.players[&b].max_health;
    assert_eq!(s.players[&b].health, hb + (max as f32 * 0.06).round() as u32, "mended, not struck");
    assert_eq!(hits_on(&ev, b), 0, "no blow on him");
    // A nova that mends nobody still strikes whoever friendly fire allows.
    run_still(&mut s, &[], 1.0);
    let ev = cast(&mut s, a, "t_blast");
    assert!(hits_on(&ev, b) >= 1, "friendly fire: struck");
}

#[test]
fn with_friendly_fire_on_consecration_mends_a_co_op_friend_and_never_burns_them() {
    let mut s = world(true, "ability = []");
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [3.0, 0.0, 0.0], 0.0);
    let hb = halve(&mut s, b);
    let mut ev = cast(&mut s, a, "consecration");
    ev.extend(run_still(&mut s, &[], 3.0));
    assert!(s.players[&b].health > hb, "mended");
    assert_eq!(hits_on(&ev, b), 0, "never burned");
}

#[test]
fn a_stun_on_the_caster_ends_a_beam() {
    let mut s = world(false, BEAM);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    cast(&mut s, p, "t_beam");
    let ev = run_still(&mut s, &[m], 0.3);
    assert!(hits_on(&ev, m) >= 1, "ticking");
    s.apply_cc(p, m, "stun", 2.0, 0.0, 0.0, "test");
    assert!(!channelling(&s, p), "the stun ends it");
    let hp = s.mobs[&m].health;
    run_still(&mut s, &[m], 1.5);
    assert_eq!(s.mobs[&m].health, hp, "no more ticks");
    assert!(!channelling(&s, p));
}

#[test]
fn a_silence_or_its_key_again_ends_a_beam() {
    let mut s = world(false, BEAM);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let m = mob(&mut s, "grunt", [6.0, 0.0, 0.0]);
    cast(&mut s, p, "t_beam");
    run_still(&mut s, &[m], 0.3);
    s.apply_cc(p, m, "silence", 2.0, 0.0, 0.0, "test");
    run_still(&mut s, &[m], 0.1);
    assert!(!channelling(&s, p), "silenced: over");
    run_still(&mut s, &[m], 1.2);
    // Again, and its key pressed again lets it go early.
    cast(&mut s, p, "t_beam");
    assert!(channelling(&s, p));
    run_still(&mut s, &[m], 0.3);
    s.input(p, Input::Ability(key_of("ranged"), false));
    assert!(!channelling(&s, p), "its key again ends it");
    let hp = s.mobs[&m].health;
    run_still(&mut s, &[m], 0.6);
    assert_eq!(s.mobs[&m].health, hp, "no more ticks");
}
