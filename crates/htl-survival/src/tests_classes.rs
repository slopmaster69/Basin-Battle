//! The nine classes' board as a whole: every card and its charged-up version goes off without
//! fault, in a fight with creatures and in a duel, and the board keeps to its budgets (damage over
//! a cooldown, crowd control on a hero, how far a knockback throws) so no card runs away with PvP.

use crate::content::{AbilityDef, Content, SLOTS};
use crate::land::FlatLand;
use crate::sim::mobs::MobState;
use crate::sim::*;

const DT: f32 = 0.05;
const ORIGIN: Vec3 = [0.0, 0.0, 0.0];

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

fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
    let mut out = Vec::new();
    for _ in 0..(secs / DT).round() as usize {
        out.extend(s.step(DT));
    }
    out
}

/// The board's cards (and their EX versions).
fn board(c: &Content) -> Vec<(usize, Option<usize>)> {
    c.abilities.iter().enumerate().filter(|(_, a)| a.on_board()).map(|(k, _)| (k, c.ex_of(k))).collect()
}

/// Press its key (the EX with `ex`), and with an aimed one, a full charge.
fn use_card(s: &mut Survival, p: Id, card: &AbilityDef, ex: bool) -> Vec<Event> {
    let slot = SLOTS.iter().position(|r| *r == card.slot).unwrap() as u8;
    s.slot_ability(p, &card.slot, &card.key).unwrap();
    s.set_cooldowns(p, true);
    s.set_valor(p, 100.0);
    s.input(p, Input::Ability(slot, ex));
    let mut ev = s.step(DT);
    if let Action::Aim { ability } = s.players[&p].action {
        s.input(p, Input::Attack);
        // The charge of what's aimed: an EX version may be held longer than its card.
        let full = s.content.abilities[ability].charge[1];
        ev.extend(run(s, full + 0.1));
        s.input(p, Input::AttackUp);
    }
    // An anchor's second press steps back to it.
    if card.anchor > 0.0 {
        ev.extend(run(s, 0.3));
        s.input(p, Input::Ability(slot, ex));
    }
    ev.extend(run(s, 2.5));
    ev
}

#[test]
fn every_card_and_its_charged_up_version_goes_off_on_creatures() {
    let mut s = world(false);
    let c = s.content.clone();
    for (k, ex) in board(&c) {
        for which in [Some(k), ex].into_iter().flatten() {
            let d = c.abilities[which].clone();
            let is_ex = which != k;
            s.mobs.clear();
            let p = hero(&mut s, "Ayla", ORIGIN, 0.0);
            // Foes in front at every reach, one to the side.
            let near = |s: &mut Survival, at: Vec3| {
                let def = s.content.mob("grunt").unwrap();
                let m = s.spawn_mob(def, at, None, false);
                let x = s.mobs.get_mut(&m).unwrap();
                x.state = MobState::Idle { wander_at: 1e9 };
                x.pos = at;
                m
            };
            let foes: Vec<Id> = [[3.0, 0.0, 0.0], [8.0, 0.5, 0.0], [14.0, 0.0, 0.0], [20.0, -0.5, 0.0], [3.0, 3.0, 0.0]].into_iter().map(|at| near(&mut s, at)).collect();
            let health: Vec<u32> = foes.iter().map(|m| s.mobs[m].health).collect();
            let at: Vec<Vec3> = foes.iter().map(|m| s.mobs[m].pos).collect();
            let ev = use_card(&mut s, p, &c.abilities[k], is_ex);
            assert!(!ev.is_empty(), "{}: nothing happened", d.key);
            let touched = foes.iter().zip(&health).zip(&at).any(|((m, h), was)| {
                s.mobs.get(m).is_none_or(|x| x.health < *h || dist2d(x.pos, *was) > 0.5) || !s.cc_of(*m).is_empty()
            });
            let for_self = matches!(d.kind.as_str(), "stance" | "buff" | "blink" | "place") || (d.kind == "leap" && d.back) || d.damage[1] <= 0.0 && d.effects.is_empty();
            assert!(touched || for_self, "{}: it reached no foe", d.key);
            s.leave(p);
            s.traps.clear();
            s.windups.clear();
        }
    }
}

#[test]
fn every_card_goes_off_in_a_duel() {
    let mut s = world(true);
    let c = s.content.clone();
    for (k, ex) in board(&c) {
        for is_ex in [false, true] {
            if is_ex && ex.is_none() {
                continue;
            }
            let a = hero(&mut s, "Ayla", ORIGIN, 0.0);
            let b = hero(&mut s, "Bren", [6.0, 0.0, 0.0], std::f32::consts::PI);
            let _ = use_card(&mut s, a, &c.abilities[k], is_ex);
            assert!(s.players[&a].health > 0, "{}: the caster stands", c.abilities[k].key);
            s.leave(a);
            s.leave(b);
            s.traps.clear();
            s.windups.clear();
            s.totems.clear();
        }
    }
}

/// What a card's hit is worth in damage: its blow (every projectile, every leap of a chain), and
/// half of what burns after (it can be walked out of).
fn worth(d: &AbilityDef) -> f32 {
    let hit = (d.damage[0] + d.damage[1]) / 2.0 * d.count.max(1) as f32;
    let chain: f32 = (1..=d.chain).map(|k| hit * d.chain_falloff.powi(k as i32)).sum();
    let burn: f32 = d.effects.iter().filter(|e| e.kind == "burn").map(|e| e.amount * e.secs[1]).sum();
    let linger = d.linger_damage * d.linger;
    hit + chain + burn + 0.5 * linger
}

/// The hard crowd control a hero gets from it (half a creature's).
fn hard_cc(d: &AbilityDef) -> f32 {
    d.effects
        .iter()
        .chain(&d.tick_effects)
        .filter(|e| matches!(e.kind.as_str(), "stun" | "incapacitate" | "petrify" | "panic"))
        .map(|e| e.secs[1] * 0.5)
        .fold(0.0, f32::max)
}

#[test]
fn the_board_keeps_to_its_budgets() {
    let s = world(true);
    let c = &s.content;
    for row in SLOTS {
        let cards: Vec<&AbilityDef> = c.abilities.iter().filter(|a| a.on_board() && a.slot == row).collect();
        assert_eq!(cards.len(), 9, "{row}");
    }
    for (k, ex) in board(c) {
        let d = &c.abilities[k];
        assert!(!d.text.is_empty() && !d.icon.is_empty(), "{}: its words and icon", d.key);
        assert!(d.spell != 0 || d.kind == "art", "{}: its look", d.key);
        if d.kind == "art" {
            continue;
        }
        // A cycle: the cooldown (shared by its charges) and the charge before it goes.
        let per = d.cooldown / d.charges.max(1) as f32 + d.charge[1];
        match d.slot.as_str() {
            "ranged" => {
                assert!(worth(d) <= 45.0, "{}: {} a hit is too much for a poke", d.key, worth(d));
                assert!(worth(d) / per.max(1.0) <= 9.5, "{}: {:.1} a second off cooldown", d.key, worth(d) / per.max(1.0));
            }
            "close" => assert!(worth(d) <= 45.0, "{}: {}", d.key, worth(d)),
            "utility" => assert!(worth(d) <= 20.0 && per >= 10.0, "{}: a utility hits little and comes back slowly", d.key),
            "defense" => assert!(per >= 12.0, "{}: a defense comes back slowly", d.key),
            "ultimate" => {
                assert_eq!(d.valor, 100, "{}: the whole bar", d.key);
                assert!(d.cooldown >= 40.0, "{}", d.key);
                assert!(worth(d) <= 110.0, "{}: {}", d.key, worth(d));
            }
            _ => unreachable!(),
        }
        for card in [Some(k), ex].into_iter().flatten().map(|x| &c.abilities[x]) {
            assert!(hard_cc(card) <= 1.5 + 1.0e-3, "{}: {} s of hard crowd control on a hero", card.key, hard_cc(card));
            for e in card.effects.iter().chain(&card.tick_effects) {
                if e.kind == "knockback" {
                    assert!(e.amount <= 10.0, "{}: thrown {} yd", card.key, e.amount);
                }
                if e.kind == "root" {
                    assert!(e.secs[1] * 0.5 <= 1.5 + 1.0e-3, "{}: rooted too long", card.key);
                }
            }
            if card.is_ex() {
                assert!([25, 50].contains(&card.valor), "{}: an EX costs a segment or two ({})", card.key, card.valor);
                assert!(worth(card) <= worth(d) * 1.8 + 20.0, "{}: an EX is better, not a different league", card.key);
            }
        }
    }
}
