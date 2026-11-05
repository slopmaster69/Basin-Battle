//! Corey's Warrior notes (Nov 2): Retaliation parries and answers with the hero's heavy attack,
//! untouchable while it goes out; Shield Wall keeps the shield bash, harder while it's up, and
//! needs a shield; Shockwave leaves its caster where they stand.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

fn world_with(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
    }
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

/// A creature standing still, facing the hero.
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

fn press(s: &mut Survival, p: Id, key: &str, ex: bool) -> Vec<Event> {
    let slot = s.content.abilities[s.content.ability(key).unwrap()].slot.clone();
    s.slot_ability(p, &slot, key).unwrap();
    s.set_cooldowns(p, true);
    s.input(p, Input::Ability(key_of(&slot), ex));
    s.step(DT)
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
}

fn heavy_swing(s: &Survival, p: Id) -> bool {
    matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.kind == StrikeKind::Heavy)
}

#[test]
fn retaliation_parries_the_blow_and_answers_with_a_free_heavy_attack_nothing_can_touch() {
    let mut s = world_with(true);
    let pi = std::f32::consts::PI;
    let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let b = hero(&mut s, "Bren", [1.8, 0.0, 0.0], pi);
    let c = hero(&mut s, "Cara", [0.6, 0.9, 0.0], -0.6);
    press(&mut s, b, "retaliation", false);
    s.players.get_mut(&b).unwrap().stamina = 40.0;
    let (ha, hb) = (s.players[&a].health, s.players[&b].health);
    s.input(a, Input::Attack);
    let mut ev = Vec::new();
    for _ in 0..40 {
        ev.extend(s.step(DT));
        if heavy_swing(&s, b) {
            break;
        }
    }
    assert!(heavy_swing(&s, b), "the parry throws Bren's heavy attack at once");
    assert_eq!(s.players[&b].health, hb, "Ayla's blow was parried");
    assert!(!flashed(&ev, b, "Parry!"), "no word: the parry shows and sounds (#135)");
    assert!(matches!(s.players[&a].action, Action::Stagger { .. }), "Ayla reels, as from any parry");
    assert!(s.players[&b].stamina > 40.0 + 20.0, "a parry's stamina back, and the heavy cost nothing: {}", s.players[&b].stamina);
    assert!(s.players[&b].opening_until > s.clock, "and a parry's opening");
    // While it goes out, Cara's blow, a stagger and an effect find nothing to take hold of.
    let from = s.players[&c].pos;
    let out = s.strike_player(c, b, 30, from, true, true, true, Blow::Heavy);
    assert_eq!(out, Outcome::Blocked);
    assert_eq!(s.players[&b].health, hb, "no damage");
    let until = s.clock + 2.0;
    s.set_action(b, Action::Stagger { until });
    assert!(heavy_swing(&s, b), "no stagger: the heavy still goes out");
    assert!(!s.apply_cc(b, c, "stun", 2.0, 0.0, 0.0, "test"), "no effect takes");
    // It lands on Ayla.
    run(&mut s, 1.2);
    assert!(s.players[&a].health < ha, "the heavy strikes the one it parried");
    // And once it has, blows land again.
    assert!(s.players[&b].untouchable_until <= s.clock);
    let out = s.strike_player(c, b, 10, from, true, false, true, Blow::Light);
    assert_eq!(out, Outcome::Hit);
    assert!(s.players[&b].health < hb);
}

#[test]
fn retaliation_parries_a_creatures_blow_too_and_the_creature_reels() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [1.6, 0.0, 0.0]);
    press(&mut s, p, "retaliation", false);
    let out = s.counter_catch(p, k, crate::sim::abilities::HitKind::Melee, None);
    assert_eq!(out, Some(Outcome::Parried));
    assert!(s.mobs[&k].stagger_until > s.clock, "the creature reels");
    assert!(heavy_swing(&s, p), "and the heavy answer goes out");
    assert!(s.untouchable(p));
}

#[test]
fn retaliation_lets_a_projectile_through_and_ends_when_you_act() {
    let mut s = world_with(true);
    let b = hero(&mut s, "Bren", ORIGIN, 0.0);
    press(&mut s, b, "retaliation", false);
    let out = s.counter_catch(b, b, crate::sim::abilities::HitKind::Projectile, None);
    assert_eq!(out, None, "projectiles aren't caught");
    s.input(b, Input::Attack);
    s.step(DT);
    assert!(!matches!(s.players[&b].action, Action::Stance { .. }), "his own attack ends the brace");
}

#[test]
fn shield_wall_bashes_with_the_guard_held_harder_and_wider_and_the_wall_holds() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let pack = [mob(&mut s, "brave", [1.9, -1.0, 0.0]), mob(&mut s, "brave", [2.2, 0.0, 0.0]), mob(&mut s, "brave", [1.9, 1.0, 0.0])];
    let hp: Vec<u32> = pack.iter().map(|m| s.mobs[m].health).collect();
    s.set_valor(p, 25.0);
    press(&mut s, p, "retaliation", true);
    assert!(matches!(s.players[&p].action, Action::Stance { .. }), "the wall is up");
    let stamina = s.players[&p].stamina;
    // Right-click held: the shield is up already, nothing changes.
    s.input(p, Input::Guard);
    s.step(DT);
    assert!(matches!(s.players[&p].action, Action::Stance { .. }), "the guard doesn't lower the wall");
    // Then a left click: the bash.
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    assert!(matches!(&s.players[&p].action, Action::Swing { strike, .. } if strike.wall_bash()), "a wall bash");
    ev.extend(run(&mut s, 0.35));
    for (m, before) in pack.iter().zip(&hp) {
        assert!(s.mobs[m].health < *before, "every one of the three struck");
        assert!(s.mobs[m].stagger_until > s.clock + 0.6, "and reeling, longer than a plain bash's 0.7 sec");
    }
    let shoved = ev.iter().filter(|e| matches!(e, Event::MobMove { id, .. } if pack.contains(id))).count();
    assert!(shoved >= 3, "each driven back off the shield: {shoved}");
    assert_eq!(s.players[&p].stamina, stamina, "it costs no stamina");
    // Done, the shield goes back up and the wall holds.
    let ev = run(&mut s, 0.6);
    assert!(matches!(s.players[&p].action, Action::Stance { .. }), "back behind the wall");
    assert!(ev.iter().any(|e| matches!(e, Event::Guard { player, up: true } if *player == p)), "the shield up again");
    assert!(s.stances.contains_key(&p));
    // A dodge still ends it.
    s.input(p, Input::GuardUp);
    s.input(p, Input::Dodge);
    s.step(DT);
    assert!(!s.stances.contains_key(&p), "a dodge ends the wall");
}

#[test]
fn shield_wall_needs_a_shield() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let back = s.players[&p].pos;
    let rack = s.objs[&s.rack_id()].pos;
    s.players.get_mut(&p).unwrap().pos = rack;
    s.arm(p, "greatsword").unwrap();
    s.players.get_mut(&p).unwrap().pos = back;
    s.set_valor(p, 25.0);
    let ev = press(&mut s, p, "retaliation", true);
    assert!(!matches!(s.players[&p].action, Action::Stance { .. }), "no wall with a two-handed weapon");
    assert!(flashed(&ev, p, "Shield Wall requires a shield"));
    assert_eq!(s.players[&p].valor, 25.0, "and no Valor spent");
    // Retaliation itself still goes with a two-hander.
    press(&mut s, p, "retaliation", false);
    assert!(matches!(s.players[&p].action, Action::Stance { .. }));
}

#[test]
fn shockwave_throws_the_pack_and_leaves_its_caster_where_they_stand() {
    // Friendly fire on, as the group plays: the caster still isn't in their own cone.
    let mut s = world_with(true);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [3.0, 0.0, 0.0]);
    let from = s.mobs[&k].pos;
    press(&mut s, p, "shockwave", false);
    s.set_aim_line(p, 0.0, 0.0);
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    s.input(p, Input::AttackUp);
    ev.extend(run(&mut s, 1.2));
    assert!(ev.iter().any(|e| matches!(e, Event::MobMove { id, path, .. } if *id == k && path.last().is_some_and(|end| dist2d(*end, from) > 3.0))), "the creature thrown back");
    assert!(!ev.iter().any(|e| matches!(e, Event::Hop { player, .. } if *player == p)), "the caster isn't thrown");
    assert!(dist2d(s.players[&p].pos, ORIGIN) < 0.3, "and stands where they cast it");
    // Its look goes to the caster's spot, never to the caster: the client plays a spell's impact
    // (a knockdown, for this one) on every unit the spell names as struck.
    assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Point(_), .. } if *caster == p)), "the look at the spot");
    assert!(!ev.iter().any(|e| matches!(e, Event::Missile { caster, target: Target::Unit(t), .. } if *caster == p && *t == p)), "not on the caster");
}

/// Seen in the lab: the client's body faced 55 degrees off the brave, every step sent that facing
/// back, and the free heavy swung at nothing.
#[test]
fn retaliation_answer_lands_though_the_client_faces_away_and_locks_on() {
    let mut s = world_with(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    let k = mob(&mut s, "brave", [0.5, 1.5, 0.0]);
    let hk = s.mobs[&k].health;
    press(&mut s, p, "retaliation", false);
    let out = s.counter_catch(p, k, crate::sim::abilities::HitKind::Melee, None);
    assert_eq!(out, Some(Outcome::Parried));
    let mut ev = Vec::new();
    for _ in 0..30 {
        // The client still faces the way it did: straight along +x.
        s.move_player(p, ORIGIN, 0.0, Moving::default());
        ev.extend(s.step(DT));
    }
    assert!(s.mobs[&k].health < hk, "the free heavy lands on the one parried");
    assert_eq!(s.players[&p].lock, Some(k), "the hero locks on to the one parried");
    assert!(s.players[&p].facing.abs() < 1e-3, "the client's facing counts again once it has struck");
}
