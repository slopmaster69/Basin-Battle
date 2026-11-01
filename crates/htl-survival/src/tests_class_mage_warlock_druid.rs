//! The redesigned mage, warlock and druid cards (#40), each cast from the real content: the thing that makes it
//! play differently.

use crate::content::Content;
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

/// The built-in rules, every card owned (friendly fire on with `pvp`).
fn world(pvp: bool) -> Survival {
    let mut c = Content::dev();
    if !pvp {
        c.pvp = Default::default();
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

/// Run on with these creatures held where they are (struck, they'd come for the hero).
fn run_pinned(s: &mut Survival, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        for (m, at) in pins {
            if let Some(x) = s.mobs.get_mut(m) {
                x.state = MobState::Idle { wander_at: 1e9 };
                x.path.clear();
                x.pos = *at;
            }
        }
        out.extend(s.step(DT));
    }
    out
}

fn key_of(slot: &str) -> u8 {
    crate::content::SLOTS.iter().position(|s| *s == slot).unwrap() as u8
}

fn idx(s: &Survival, key: &str) -> usize {
    s.content.ability(key).unwrap()
}

/// Own a card and put it on its key.
fn learn(s: &mut Survival, p: Id, key: &str) {
    let slot = s.content.abilities[idx(s, key)].slot.clone();
    s.players.get_mut(&p).unwrap().owned.insert(key.to_string());
    s.slot_ability(p, &slot, key).unwrap();
}

/// A key pressed (`ex`: with Ctrl).
fn press(s: &mut Survival, p: Id, slot: &str, ex: bool) -> Vec<Event> {
    s.input(p, Input::Ability(key_of(slot), ex));
    s.step(DT)
}

/// The click held `secs` (a tap: 0) and let go, the creatures held where they are.
fn hold(s: &mut Survival, p: Id, secs: f32, pins: &[(Id, Vec3)]) -> Vec<Event> {
    s.input(p, Input::Attack);
    let mut ev = s.step(DT);
    ev.extend(run_pinned(s, secs, pins));
    s.input(p, Input::AttackUp);
    ev.extend(s.step(DT));
    ev
}

fn aiming(s: &Survival, p: Id) -> Option<usize> {
    match s.players[&p].action {
        Action::Aim { ability } => Some(ability),
        _ => None,
    }
}

fn missile(ev: &[Event], spell: u32) -> bool {
    ev.iter().any(|e| matches!(e, Event::Missile { spell: s, .. } if *s == spell))
}

fn flashed(ev: &[Event], p: Id, text: &str) -> bool {
    ev.iter().any(|e| matches!(e, Event::Flash { player, text: t } if *player == p && t.contains(text)))
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

// --- Mage: freeze, then shatter ---------------------------------------------------------------------

#[test]
fn frost_nova_makes_the_next_fireball_a_free_pyroblast_that_shatters_the_frozen() {
    let mut s = world(false);
    let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
    learn(&mut s, p, "fireball");
    learn(&mut s, p, "cone_of_cold");
    s.set_cooldowns(p, true);
    let k = boar(&mut s, [6.0, 0.0, 0.0]);
    let pins = [(k, [6.0, 0.0, 0.0])];
    // Frost Nova (Cone of Cold's EX, 25 Valor): the boar frozen, the Fireball key empowered 5 s.
    s.set_valor(p, 25.0);
    let ev = press(&mut s, p, "close", true);
    assert!(missile(&ev, 122), "the nova goes");
    assert!(s.has_cc(k, "petrify"), "the boar frozen");
    let em = s.empowered_keys(p);
    assert_eq!(em.len(), 1, "one key empowered: {em:?}");
    assert_eq!(em[0].0, key_of("ranged") as usize, "the Fireball's");
    assert!((em[0].1 - (s.clock + 5.0)).abs() < 0.2, "for 5 s");
    let valor = s.players[&p].valor;
    // The plain Fireball key now aims a Pyroblast; held its 1.5 s it goes, for no Valor.
    run_pinned(&mut s, 0.3, &pins);
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "pyroblast")), "the key's EX version, Pyroblast");
    let mut ev = hold(&mut s, p, 1.6, &pins);
    assert!(missile(&ev, 11366), "the Pyroblast flies");
    ev.extend(run_pinned(&mut s, 0.6, &pins));
    assert!(flashed(&ev, p, "Shatter!"), "the frozen boar shatters");
    assert!(hits(&ev, p, k).iter().any(|d| *d >= 70), "half again its 48: {:?}", hits(&ev, p, k));
    assert!(s.players[&p].valor + 0.001 >= valor, "no Valor spent: {} then {}", valor, s.players[&p].valor);
    assert!(s.empowered_keys(p).is_empty(), "the combo spent");
    // Then the key is Fireball again.
    press(&mut s, p, "ranged", false);
    assert_eq!(aiming(&s, p), Some(idx(&s, "fireball")));
}
