//! The Deadmines balance table (docs/deadmines-balance.md): every boss and every pack between them,
//! fought by a scripted hero of each class, two ways. One who plays it right answers every blow it
//! sees (parries a blade it can block, rolls a heavy blow or a bolt, waits out a string); a sloppy
//! one misses half of those answers and swings when it shouldn't. Both take the Draughts a hero
//! has at the first lamp (3), swing the starting axe and cast their class's two starting cards
//! (Q and E) whenever they're ready, on the built-in numbers a real server runs (not
//! `Content::dev`, which takes the foes' damage back to 1.0).
//!
//! Rerun it (under a minute; the output is the doc's table):
//!   `cargo test --release -p htl-survival balance_table -- --ignored --nocapture`
//! One cell, told moment by moment:
//!   `BALANCE_FOES=dm_smite BALANCE_CLASS=mage BALANCE_SEED=3 BALANCE_SLOPPY=1 BALANCE_TRACE=1 \
//!    cargo test --release -p htl-survival balance_one -- --ignored --nocapture`
//! `BALANCE_SEEDS=n` sets the seeds per cell (default 10 for a boss, 5 for a pack), `BALANCE_ONLY=text`
//! keeps the fights whose name has that text, `BALANCE_CLASSES=warrior,mage` the classes.
//!
//! Not scripted (so what the table can't say): Gilnid's pole (neither engineer is struck down; the
//! plank comes down on the demonstration or the sixth stick), Rhahk'Zor's collapse (his phases but
//! not the run through the falling mine, which wants the real floor), a hero's lamp rests between
//! fights, and any answer but the dodge, the parry, the shield bash and the Draught.

use super::{Foe, MobState};
use crate::content::{Content, SLOTS};
use crate::land::FlatLand;
use crate::sim::*;
use crate::tests_fair::{FOLLOW_REACTION, REACTION};
use crate::tests_fair_hero::Profile;
use std::collections::BTreeSet;

const DT: f32 = 0.02;
/// The longest a fight is played before it's called stalled.
const CAP: f32 = 420.0;
/// The Draughts a hero has at the first lamp (`draughts` in deadmines.toml).
const DRAUGHTS: u32 = 3;
const PARRY_LEAD: f32 = 0.12;
const BASH_REACH: f32 = 2.2;

/// One who answers what they see.
const RIGHT: Profile = Profile { reaction: REACTION, follow: FOLLOW_REACTION, miss: 0.0, patient: true };
/// One who misses half the dodges and blocks.
const SLOPPY: Profile = Profile { reaction: REACTION, follow: FOLLOW_REACTION, miss: 0.5, patient: false };

struct Threat {
    land: f32,
    seen: f32,
    from: Id,
    parry: bool,
    pressed: bool,
}

/// What one fight came to.
#[derive(Default, Debug, Clone)]
pub(super) struct Run {
    pub secs: f32,
    pub won: bool,
    pub fell: bool,
    /// Neither: still going at the cap.
    pub stalled: bool,
    pub damage: u32,
    pub max_health: u32,
    pub draughts: u32,
    pub dodged: u32,
    pub parried: u32,
    pub landed: u32,
    /// The most blows from different creatures due to land within 0.4 s of each other (a dodge's
    /// whole safe window is 0.28 s): what a pack stacks on one hero at once.
    pub peak_stack: u32,
    /// Steps in which a roll was wanted with too little stamina to start one.
    pub dry: u32,
}

#[derive(Clone)]
pub(super) enum Stage {
    /// These creatures (by key) set 10 yd ahead of the hero, in the chase.
    Pack(Vec<String>),
    /// A boss, staged as its own tests stage it.
    Boss(&'static str),
}

/// The creature keys whose death ends a boss fight (each seen, then none left alive), and the keys
/// the hero walks to and strikes (in order of preference by distance).
fn boss_rules(key: &str) -> (Vec<&'static str>, Vec<&'static str>) {
    match key {
        "sneed" => (vec!["dm_sneed"], vec!["dm_shredder", "dm_sneed"]),
        "greenskin" => (vec!["dm_greenskin"], vec!["dm_deck_gunner", "dm_greenskin"]),
        // (Gorged rises only for the secret ingredient: the plain fight ends with Cookie.)
        "cookie" => (vec!["dm_cookie"], vec!["dm_cookie"]),
        _ => unreachable!("{key}"),
    }
}

/// The bosses, in route order: (name, creature key, stage).
pub(super) fn bosses() -> Vec<(&'static str, &'static str)> {
    vec![
        ("Rhahk'Zor", "dm_rhahkzor"),
        ("Sneed and the Shredder", "dm_shredder"),
        ("Gilnid", "dm_gilnid"),
        ("Smite", "dm_smite"),
        ("Greenskin", "dm_greenskin"),
        ("VanCleef", "dm_vancleef"),
        ("Cookie", "dm_cookie"),
    ]
}

fn content() -> Content {
    let mut c = Content::builtin();
    c.pvp = Default::default();
    c
}

/// A dungeon world with only the pack of this key left (at the origin), for the fights that need
/// the dungeon's own machinery (the Shredder, Rhahk'Zor).
fn lone_pack(key: &str, mut c: Content, seed: u64) -> Survival {
    let d = c.dungeon.as_mut().unwrap();
    d.packs.retain(|p| p.key == key);
    d.packs[0].at = [0.0, 0.0];
    d.packs[0].z = Some(0.0);
    d.lamps[0].at = [100.0, -20.0, 0.0];
    Survival::new_dungeon(c, Box::new(FlatLand::default()), seed, 1_000_000.0)
}

fn live_of(s: &Survival, key: &str) -> Option<Id> {
    s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
}

/// The hero: a class's starting cards on their keys, three Draughts, at `at` facing `face`.
fn hero(s: &mut Survival, class: &str, at: Vec3, face: f32) -> Id {
    let id = s.join("ayla", "Ayla", None);
    s.move_player(id, at, face, Moving::default());
    let _ = s.step(0.0);
    let (loadout, owned) = s.fresh_abilities(class);
    let draught = s.content.item("draught").unwrap();
    let p = s.players.get_mut(&id).unwrap();
    p.class = class.to_string();
    p.loadout = loadout;
    p.owned = owned;
    p.bag.insert(draught, DRAUGHTS);
    p.quick = Some(draught);
    id
}

/// The world, the hero and the creatures to fight, a boss as its own tests stage it.
fn stage(st: &Stage, class: &str, seed: u64) -> (Survival, Id) {
    match st {
        Stage::Pack(foes) => {
            let mut s = Survival::bare(content(), Box::new(FlatLand::default()), seed, 0.0);
            let h = hero(&mut s, class, [0.0, 0.0, 0.0], 0.0);
            for (i, key) in foes.iter().enumerate() {
                let def = s.content.mob(key).unwrap_or_else(|| panic!("no creature {key}"));
                let side = (i as f32 - (foes.len() as f32 - 1.0) / 2.0) * 1.8;
                let id = s.spawn_mob(def, [10.0, side, 0.0], None, false);
                s.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
            }
            (s, h)
        }
        Stage::Boss(key) => match *key {
            "dm_shredder" => {
                let mut s = lone_pack("sneed", content(), seed);
                s.mobs.retain(|_, m| m.health > 0);
                let machine = live_of(&s, "dm_shredder").expect("the Shredder");
                let h = hero(&mut s, class, [4.0, 0.0, 0.0], std::f32::consts::PI);
                s.engage(machine, Foe::Hero(h));
                (s, h)
            }
            "dm_rhahkzor" => {
                // His phases as they are, less the run through the collapsing mine (it's laid on
                // the real floor's coordinates): the maul, the rocks, the last stand.
                let mut c = content();
                let def = c.mob("dm_rhahkzor").unwrap();
                for p in c.mobs[def].phases.iter_mut() {
                    p.collapse = None;
                    p.slam = None;
                }
                let mut s = lone_pack("rhahkzor", c, seed);
                let boss = live_of(&s, "dm_rhahkzor").expect("Rhahk'Zor");
                let h = hero(&mut s, class, [3.0, 0.0, 0.0], std::f32::consts::PI);
                s.engage(boss, Foe::Hero(h));
                (s, h)
            }
            "dm_gilnid" => {
                let mut s = Survival::bare(content(), Box::new(FlatLand::default()), seed, 0.0);
                let h = hero(&mut s, class, [-6.0, 0.0, 0.0], 0.0);
                let boss = s.spawn_mob(s.content.mob("dm_gilnid").unwrap(), [0.0; 3], None, false);
                s.step_perch_crews();
                s.engage(boss, Foe::Hero(h));
                s.step_perch_crews();
                (s, h)
            }
            "dm_greenskin" => {
                let mut s = Survival::bare(content(), Box::new(FlatLand::default()), seed, 0.0);
                let h = hero(&mut s, class, [-8.0, 0.0, 0.0], 0.0);
                let boss = s.spawn_mob(s.content.mob("dm_greenskin").unwrap(), [0.0; 3], None, false);
                let _ = s.step(DT);
                s.engage(boss, Foe::Hero(h));
                (s, h)
            }
            key => {
                // Smite, VanCleef and Cookie: beside him at the pull.
                let mut s = Survival::bare(content(), Box::new(FlatLand::default()), seed, 0.0);
                let h = hero(&mut s, class, [2.5, 0.0, 0.0], std::f32::consts::PI);
                let boss = s.spawn_mob(s.content.mob(key).unwrap(), [0.0; 3], None, false);
                s.engage(boss, Foe::Hero(h));
                s.clean_duels();
                (s, h)
            }
        },
    }
}

/// Which creatures end the fight (all seen, then none alive) and which the hero strikes.
fn rules(st: &Stage) -> (Vec<String>, Vec<String>) {
    match st {
        Stage::Pack(foes) => (vec![], foes.clone()),
        Stage::Boss(key) => {
            let short = match *key {
                "dm_shredder" => "sneed",
                "dm_greenskin" => "greenskin",
                "dm_cookie" => "cookie",
                _ => "",
            };
            if short.is_empty() {
                (vec![key.to_string()], vec![key.to_string()])
            } else {
                let (fin, hit) = boss_rules(short);
                (fin.iter().map(|s| s.to_string()).collect(), hit.iter().map(|s| s.to_string()).collect())
            }
        }
    }
}

/// The creatures alive and hostile now.
fn foes_now(s: &Survival) -> Vec<Id> {
    s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.friendly).map(|(id, _)| *id).collect()
}

/// One fight, played to the end: won, fell, or stalled at the cap.
pub(super) fn run(st: &Stage, class: &str, seed: u64, who: Profile) -> Run {
    let (reaction, follow) = (who.reaction, who.follow);
    let mut dice = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let mut misread = move || {
        dice ^= dice << 13;
        dice ^= dice >> 7;
        dice ^= dice << 17;
        ((dice >> 11) as f64 / (1u64 << 53) as f64) < who.miss as f64
    };
    let trace = std::env::var("BALANCE_TRACE").is_ok();
    let (fin, hit) = rules(st);
    let (mut s, p) = stage(st, class, seed);
    let (evade_from, evade_to) = (s.content.dodge.evade_from, s.content.dodge.evade_to);
    let mid = 0.5 * (evade_from + evade_to);
    let weapon = s.weapon_of(&s.players[&p]).clone();
    let lock = weapon.windup + weapon.active + weapon.recovery * s.content.combat.cancel_after;
    let draught = s.content.item("draught").unwrap();
    let drink_secs = s.content.items[draught].use_secs;
    let mut out = Run { max_health: s.players[&p].max_health, ..Default::default() };
    // The class's cards on Q and E: (slot index, ability, least charge, full charge, reach).
    let kit: Vec<(u8, usize, f32, f32, f32)> = ["ranged", "close"]
        .iter()
        .filter_map(|slot| {
            let key = s.players[&p].loadout.get(*slot)?.clone();
            let a = s.content.ability(&key)?;
            let d = &s.content.abilities[a];
            let reach = d.range[1].max(d.radius[1]).max(6.0);
            Some((SLOTS.iter().position(|x| x == slot).unwrap() as u8, a, d.charge[0].max(0.0), d.charge[1].max(0.0), reach))
        })
        .collect();
    let mut threats: Vec<Threat> = Vec::new();
    let mut seen_bolts: Vec<Id> = Vec::new();
    let mut casts: Vec<(Id, f32, f32)> = Vec::new();
    let mut landed_at: Vec<(Id, f32)> = Vec::new();
    let mut seen: BTreeSet<String> = BTreeSet::new();
    // A card held: the moment its charge is let go.
    let mut held: Option<f32> = None;
    let start = s.clock;
    let mut hp = s.players[&p].health;
    let name = |s: &Survival, id: Id| s.mobs.get(&id).map_or("?".to_string(), |m| s.content.mobs[m.def].key.clone());
    while s.clock - start < CAP {
        let now = s.clock;
        let me = s.players[&p].pos;
        let ids = foes_now(&s);
        for id in &ids {
            seen.insert(s.content.mobs[s.mobs[id].def].key.clone());
        }
        // The fight is over when every key that ends it was seen and none is alive.
        let alive_fin = ids.iter().any(|id| fin.contains(&s.content.mobs[s.mobs[id].def].key));
        if !fin.is_empty() && fin.iter().all(|k| seen.contains(k)) && !alive_fin {
            out.won = true;
            break;
        }
        if fin.is_empty() && ids.is_empty() {
            out.won = true;
            break;
        }
        // `BALANCE_TRACE`: where everyone stands every 1 s.
        if trace && ((now - start) / 1.0).fract() < DT / 1.0 {
            let who: Vec<String> = ids.iter().map(|i| format!("{} {:?} at {:.1},{:.1},{:.1} hp {}", name(&s, *i), s.mobs[i].state, s.mobs[i].pos[0], s.mobs[i].pos[1], s.mobs[i].pos[2], s.mobs[i].health)).collect();
            println!("{:6.1} hero {:.1},{:.1} hp {} st {:.0} {:?} held {:?} | {}", now - start, me[0], me[1], s.players[&p].health, s.players[&p].stamina, s.players[&p].action, held, who.join("; "));
        }
        for ms in &s.missiles {
            if !seen_bolts.contains(&ms.mark) && dist2d(ms.at, me) <= ms.radius + 1.0 {
                seen_bolts.push(ms.mark);
                if let Some(t) = threats.iter_mut().find(|t| t.from == ms.owner && !t.parry && !t.pressed && t.seen <= now) {
                    t.land = ms.land_at;
                    continue;
                }
                threats.push(Threat { land: ms.land_at, seen: now + reaction, from: ms.owner, parry: false, pressed: misread() });
            }
        }
        threats.retain(|t| t.land + 0.3 > now);
        {
            let due: Vec<&Threat> = threats.iter().filter(|t| t.land > now).collect();
            for a in &due {
                let mut from: Vec<Id> = due.iter().filter(|b| (b.land - a.land).abs() < 0.4).map(|b| b.from).collect();
                from.sort_unstable();
                from.dedup();
                let n = from.len() as u32;
                out.peak_stack = out.peak_stack.max(n);
            }
        }
        casts.retain(|c| c.2 > now && s.mobs.get(&c.0).is_some_and(|m| matches!(m.state, MobState::Windup { .. })));
        let acting = s.players[&p].action.clone();
        let free = matches!(acting, Action::Ready | Action::Guard);
        // A cast in reach of the shield: bashed aside.
        let bash = casts.iter().find(|c| now >= c.1 && s.mobs.get(&c.0).is_some_and(|m| dist2d(m.pos, me) <= BASH_REACH)).map(|c| c.0);
        if let Some(caster) = bash {
            if free {
                let at = s.mobs[&caster].pos;
                s.move_player(p, me, facing(me, at), Moving::default());
                s.input(p, Input::Guard);
                s.input(p, Input::Bash);
                s.input(p, Input::GuardUp);
            }
        }
        let ready = s.players[&p].stamina >= s.content.dodge.needs();
        let rolling = matches!(acting, Action::Dodge { .. });
        let guarding = matches!(acting, Action::Guard);
        let mut dodge = false;
        let mut parry: Option<Id> = None;
        for t in threats.iter_mut().filter(|t| !t.pressed && now >= t.seen) {
            if t.parry && now >= t.land - PARRY_LEAD {
                if guarding {
                    t.pressed = true;
                } else {
                    parry = Some(t.from);
                }
            } else if !t.parry && now >= t.land - mid {
                if rolling {
                    t.pressed = true;
                } else {
                    dodge = true;
                }
            }
        }
        let hold_guard = threats.iter().any(|t| t.parry && t.pressed && now < t.land + 0.1);
        if let Some(from) = parry {
            // A cast in hand is called off for the guard.
            if held.is_some() {
                s.input(p, Input::Cancel);
                held = None;
            }
            if let Some(at) = s.mobs.get(&from).map(|m| m.pos) {
                s.move_player(p, me, facing(me, at), Moving::default());
            }
            s.input(p, Input::Guard);
        } else if guarding && !hold_guard {
            s.input(p, Input::GuardUp);
        }
        let dodge = dodge && parry.is_none();
        if dodge && !ready && !rolling {
            out.dry += 1;
        }
        if dodge && ready {
            if held.is_some() {
                s.input(p, Input::Cancel);
                held = None;
            }
            let f = s.players[&p].facing;
            s.move_player(p, me, f, Moving { left: true, ..Moving::default() });
            s.input(p, Input::Dodge);
        }
        // The Draught: below half health, when nothing seen lands while they drink (the sloppy
        // drink regardless).
        let free_now = matches!(s.players[&p].action, Action::Ready | Action::Guard);
        let left = s.players[&p].bag.get(&draught).copied().unwrap_or(0);
        let calm = !who.patient || !threats.iter().any(|t| t.land > now && t.land - now < drink_secs);
        if left > 0 && free_now && !dodge && parry.is_none() && s.players[&p].health * 2 < s.players[&p].max_health && calm {
            s.input(p, Input::Use);
        }
        // A held card goes at its time.
        if let Some(at) = held {
            if now >= at {
                s.input(p, Input::AttackUp);
                held = None;
            } else if !matches!(s.players[&p].action, Action::Aim { .. } | Action::Charge { .. } | Action::Swing { .. } | Action::Channel { .. }) && now > at - 0.05 {
                held = None;
            }
        }
        // The nearest foe the hero strikes (a boss's own, in the ones that stand on the ground).
        let floor = s.players[&p].pos[2];
        let near = ids
            .iter()
            .filter(|m| hit.contains(&s.content.mobs[s.mobs[*m].def].key) && (s.mobs[*m].pos[2] - floor).abs() < 3.0)
            .min_by(|a, b| dist2d(s.mobs[*a].pos, me).total_cmp(&dist2d(s.mobs[*b].pos, me)))
            .or_else(|| ids.iter().min_by(|a, b| dist2d(s.mobs[*a].pos, me).total_cmp(&dist2d(s.mobs[*b].pos, me))))
            .copied();
        if let Some(target) = near {
            let tpos = s.mobs[&target].pos;
            let d = dist2d(tpos, me);
            let mid_string = ids.iter().any(|m| {
                let strung = landed_at.iter().any(|(x, t)| x == m && now - *t < 0.6);
                !s.attack_now(*m).ranged
                    && s.mobs.get(m).is_some_and(|x| {
                        x.health > 0
                            && dist2d(x.pos, me) <= 6.0
                            && match x.state {
                                MobState::Windup { .. } => strung,
                                MobState::Recover { until, .. } => until - now < 0.35,
                                _ => false,
                            }
                    })
            });
            let spare = s.players[&p].stamina >= s.content.dodge.needs() + 17.0;
            let busy = (who.patient && (!spare || mid_string)) || threats.iter().any(|t| t.land > now && t.land - now < lock + mid);
            if matches!(s.players[&p].action, Action::Ready) && !dodge && parry.is_none() && held.is_none() {
                let f = facing(me, tpos);
                // A card that's ready and in reach, before the swing.
                // (Point blank, only the least charge: a full one never finishes under blows.)
                let close = ids.iter().any(|m| dist2d(s.mobs[m].pos, me) < 6.0);
                let card = kit.iter().find(|(_, a, least, full, reach)| {
                    let hold = if close { *least } else { *full };
                    s.ready(p, *a) && d <= *reach && !threats.iter().any(|t| t.land > now && t.land - now < hold + mid)
                });
                if let (Some((slot, _, least, full, _)), false) = (card, busy) {
                    let hold = if close { *least } else { *full };
                    s.move_player(p, me, f, Moving::default());
                    s.input(p, Input::Ability(*slot, false));
                    if matches!(s.players[&p].action, Action::Aim { .. }) {
                        s.input(p, Input::Attack);
                        held = Some(now + hold + 0.05);
                    }
                } else if d <= weapon.range + 0.5 {
                    s.move_player(p, me, f, Moving::default());
                    if !busy {
                        s.input(p, Input::Attack);
                    }
                } else {
                    let speed = 7.0 * DT;
                    let to = [me[0] + f.cos() * speed, me[1] + f.sin() * speed, me[2]];
                    s.move_player(p, to, f, Moving { forward: true, ..Moving::default() });
                }
            }
        }
        for e in s.step(DT) {
            if let Event::Swing { attacker, victim: Some(v), .. } = &e {
                if *v == p && ids.contains(attacker) {
                    landed_at.retain(|(m, _)| m != attacker);
                    landed_at.push((*attacker, s.clock));
                }
            }
            match e {
                Event::Cast { caster, ms, .. } if ids.contains(&caster) => {
                    let at_me = s.mobs.get(&caster).is_some_and(|m| matches!(m.state, MobState::Windup { foe: Foe::Hero(h), .. } if h == p));
                    if at_me && !casts.iter().any(|c| c.0 == caster) {
                        let windup = ms as f32 / 1000.0;
                        let a = s.attack_now(caster);
                        let far = dist2d(s.mobs[&caster].pos, s.players[&p].pos);
                        let flight = if a.speed > 0.0 { far / a.speed } else { 0.0 };
                        casts.push((caster, now + reaction, now + windup));
                        threats.push(Threat { land: now + windup + flight, seen: now + reaction, from: caster, parry: false, pressed: misread() });
                    }
                }
                // A gunner's aim locking (ReadyRifle, clip 48): the shot goes down the barrel's line
                // when it ends, so it's rolled out of like a bolt.
                Event::Clip { id, clip: 48, secs, .. } if matches!(name(&s, id).as_str(), "dm_deck_gunner" | "dm_greenskin") => {
                    let at_me = s.mobs.get(&id).is_some_and(|m| matches!(m.state, MobState::Windup { foe: Foe::Hero(h), .. } if h == p));
                    if at_me {
                        threats.push(Threat { land: now + secs, seen: now + reaction, from: id, parry: false, pressed: misread() });
                    }
                }
                Event::MobWindup { id, windup, blockable, .. } => {
                    let at_me = s.mobs.get(&id).is_some_and(|m| matches!(m.state, MobState::Windup { foe: Foe::Hero(h), .. } if h == p));
                    let a = s.attack_now(id);
                    let react = if landed_at.iter().any(|(m, t)| *m == id && now - *t < 0.6) { follow } else { reaction };
                    if trace && at_me {
                        println!("{:6.2} {} winds up {:.2} s", now - start, name(&s, id), windup);
                    }
                    if at_me && !a.ranged {
                        threats.push(Threat { land: now + windup, seen: now + react, from: id, parry: blockable && !a.heavy, pressed: misread() });
                    } else if at_me && !casts.iter().any(|c| c.0 == id) {
                        let far = s.mobs.get(&id).map_or(0.0, |m| dist2d(m.pos, s.players[&p].pos));
                        let flight = if a.speed > 0.0 { far / a.speed } else { 0.0 };
                        casts.push((id, now + reaction, now + windup));
                        threats.push(Threat { land: now + windup + flight, seen: now + reaction, from: id, parry: false, pressed: misread() });
                    }
                }
                Event::Swing { attacker, victim: Some(v), damage: 0, outcome, .. } if v == p && ids.contains(&attacker) => {
                    if outcome == crate::sim::Outcome::Parried {
                        out.parried += 1;
                    } else {
                        out.dodged += 1;
                    }
                }
                Event::Swing { attacker, victim: Some(v), damage, style, .. } if v == p && damage > 0 && ids.contains(&attacker) && !matches!(style, Blow::Dot) => {
                    if trace {
                        println!("{:6.2}   LANDS {} {} hp {} ({:?}, {:?})", now - start, name(&s, attacker), damage, s.players[&p].health, style, s.players[&p].action);
                    }
                    out.landed += 1;
                }
                _ => {}
            }
        }
        // Damage taken is the health lost between steps (every path to the hero: blows, burns,
        // falling rock, a shove); what a Draught or a heal gives back isn't taken off.
        let now_hp = s.players[&p].health;
        if now_hp < hp {
            out.damage += hp - now_hp;
        }
        hp = now_hp;
        if now_hp == 0 {
            out.fell = true;
            break;
        }
    }
    out.secs = s.clock - start;
    out.stalled = !out.won && !out.fell;
    out.draughts = DRAUGHTS - s.players[&p].bag.get(&draught).copied().unwrap_or(0).min(DRAUGHTS);
    out
}

/// The fights: each boss, then each pack of the Deadmines' route (a boss's own pack left out).
fn fights() -> (Vec<(String, Stage)>, Vec<(String, Stage)>) {
    let c = content();
    let d = c.dungeon.as_ref().expect("the Deadmines");
    let mut packs = Vec::new();
    for (i, pk) in d.packs.iter().enumerate() {
        let Some(def) = c.mob(&pk.mob) else { continue };
        // (A boss's own pull is in the boss table, with its crew.)
        if c.mobs[def].boss || bosses().iter().any(|(_, k)| *k == pk.mob) || !pk.key.is_empty() && matches!(pk.key.as_str(), "sneed" | "gilnid" | "rhahkzor") {
            continue;
        }
        let foes = vec![pk.mob.clone(); pk.count.max(1)];
        // The same pack standing in several places is fought once; its numbers are listed.
        match packs.iter_mut().find(|(_, st): &&mut (String, Stage)| matches!(st, Stage::Pack(f) if *f == foes)) {
            Some((what, _)) => *what = what.replacen(':', &format!(", {}:", i + 1), 1),
            None => packs.push((format!("{}: {} x{}", i + 1, c.mobs[def].name, foes.len()), Stage::Pack(foes))),
        }
    }
    let bosses = bosses().into_iter().map(|(n, k)| (n.to_string(), Stage::Boss(k))).collect();
    (bosses, packs)
}

const CLASSES: [&str; 4] = ["warrior", "mage", "priest", "hunter"];

/// One cell of the table: a fight, a class, a profile, over `seeds`.
#[derive(Debug, Clone)]
struct Cell {
    runs: Vec<Run>,
}

impl Cell {
    fn mean(&self, f: impl Fn(&Run) -> f32) -> f32 {
        self.runs.iter().map(f).sum::<f32>() / self.runs.len() as f32
    }

    fn count(&self, f: impl Fn(&Run) -> bool) -> usize {
        self.runs.iter().filter(|r| f(r)).count()
    }

    /// Mean seconds of the runs that ended in a win (0 if none).
    fn win_secs(&self) -> f32 {
        let w: Vec<f32> = self.runs.iter().filter(|r| r.won).map(|r| r.secs).collect();
        if w.is_empty() { 0.0 } else { w.iter().sum::<f32>() / w.len() as f32 }
    }

    fn line(&self) -> String {
        let n = self.runs.len();
        let ws = self.win_secs();
        format!(
            "{:.0} s ({}) · {:.0}% · {:.1} · {}/{} · {}/{}{}",
            self.mean(|r| r.secs),
            if ws > 0.0 { format!("wins {ws:.0}") } else { "no win".into() },
            100.0 * self.mean(|r| r.damage as f32 / r.max_health as f32),
            self.mean(|r| r.draughts as f32),
            self.count(|r| r.fell),
            n,
            self.count(|r| r.won),
            n,
            if self.count(|r| r.stalled) > 0 { format!(" ({} stalled)", self.count(|r| r.stalled)) } else { String::new() },
        )
    }
}

/// Every job of the table run over all the cores, in order.
fn table(fights: &[(String, Stage)], classes: &[&str], seeds: u64) -> Vec<Vec<(Cell, Cell)>> {
    let jobs: Vec<(usize, usize, bool, u64)> = (0..fights.len())
        .flat_map(|f| (0..classes.len()).flat_map(move |c| [false, true].into_iter().flat_map(move |sl| (1..=seeds).map(move |sd| (f, c, sl, sd)))))
        .collect();
    let next = std::sync::atomic::AtomicUsize::new(0);
    let results = std::sync::Mutex::new(Vec::new());
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get()).min(12);
    std::thread::scope(|sc| {
        for _ in 0..threads {
            std::thread::Builder::new()
                .stack_size(32 << 20)
                .spawn_scoped(sc, || loop {
                    let i = next.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                    let Some(&(f, c, sl, sd)) = jobs.get(i) else { break };
                    let r = run(&fights[f].1, classes[c], sd, if sl { SLOPPY } else { RIGHT });
                    results.lock().unwrap().push(((f, c, sl, sd), r));
                })
                .unwrap();
        }
    });
    let mut results = results.into_inner().unwrap();
    results.sort_by_key(|(k, _)| *k);
    (0..fights.len())
        .map(|f| {
            (0..classes.len())
                .map(|c| {
                    let pick = |sl: bool| Cell { runs: results.iter().filter(|(k, _)| k.0 == f && k.1 == c && k.2 == sl).map(|(_, r)| r.clone()).collect() };
                    (pick(false), pick(true))
                })
                .collect()
        })
        .collect()
}

fn classes_wanted() -> Vec<&'static str> {
    match std::env::var("BALANCE_CLASSES") {
        Ok(v) => CLASSES.iter().copied().filter(|c| v.split(',').any(|x| x == *c)).collect(),
        Err(_) => CLASSES.to_vec(),
    }
}

fn print(title: &str, fights: &[(String, Stage)], classes: &[&str], t: &[Vec<(Cell, Cell)>]) {
    println!("\n### {title}\n");
    println!("Each cell: mean seconds (mean seconds of the wins) · damage taken as a share of the hero's health · Draughts drunk · falls/runs · wins/runs.\n");
    println!("| fight | class | plays it right | sloppy |");
    println!("| --- | --- | --- | --- |");
    for (f, (name, _)) in fights.iter().enumerate() {
        for (c, class) in classes.iter().enumerate() {
            println!("| {} | {} | {} | {} |", if c == 0 { name.as_str() } else { "" }, class, t[f][c].0.line(), t[f][c].1.line());
        }
    }
}

#[test]
#[ignore]
fn balance_table() {
    let (bosses, packs) = fights();
    let classes = classes_wanted();
    let only = std::env::var("BALANCE_ONLY").ok();
    let keep = |v: Vec<(String, Stage)>| -> Vec<(String, Stage)> { v.into_iter().filter(|(n, _)| only.as_ref().is_none_or(|o| n.contains(o.as_str()))).collect() };
    let (bosses, packs) = (keep(bosses), keep(packs));
    let seeds = |d: u64| std::env::var("BALANCE_SEEDS").ok().and_then(|x| x.parse().ok()).unwrap_or(d);
    let t = std::time::Instant::now();
    let probe = run(&Stage::Pack(vec!["dm_pirate".into()]), "warrior", 1, RIGHT);
    println!("A hero with {} health, 3 Draughts of 40%, the starting axe and buckler and their class's Q and E cards.", probe.max_health);
    let bt = table(&bosses, &classes, seeds(10));
    print("Bosses", &bosses, &classes, &bt);
    let pt = table(&packs, &classes, seeds(5));
    print("Packs between them", &packs, &classes, &pt);
    println!("\n{:.0} s of real time.", t.elapsed().as_secs_f32());
}

/// The packs the way the route meets them: neighbours whose `link` pulls them in together (the
/// table's pack rows fight each pack alone). Route order.
fn linked_groups() -> Vec<(String, Stage)> {
    let g = |name: &str, foes: &[(&str, usize)]| -> (String, Stage) {
        let v = foes.iter().flat_map(|(k, n)| vec![format!("dm_{k}"); *n]).collect();
        (name.to_string(), Stage::Pack(v))
    };
    vec![
        g("Cove: Pirate x2 + Squallshaper", &[("pirate", 2), ("squallshaper", 1)]),
        g("Scaffold: Pirate x2 + Squallshaper + Wizard", &[("pirate", 2), ("squallshaper", 1), ("wizard", 1)]),
        g("Deck foot: Pirate x2 + Evoker x2", &[("pirate", 2), ("evoker", 2)]),
        g("Deck: Pirate x2 + Squallshaper x2", &[("pirate", 2), ("squallshaper", 2)]),
        g("Gallery: Pirate x2 + Woodcutter x2", &[("pirate", 2), ("woodcutter", 2)]),
        g("Climb: Woodcutter x2 + Engineer", &[("woodcutter", 2), ("engineer", 1)]),
        g("Foundry: Engineer x2", &[("engineer", 2)]),
        g("Alcove: Blackguard x3 + Evoker", &[("blackguard", 3), ("evoker", 1)]),
        g("Passage: Miner x2 + Dynamiter", &[("miner", 2), ("dynamiter", 1)]),
        g("Galleries: Overseer + Miner x2", &[("overseer", 1), ("miner", 2)]),
        g("Galleries: Overseer + Miner x2 + Wizard", &[("overseer", 1), ("miner", 2), ("wizard", 1)]),
        g("Wizards: Wizard + Evoker", &[("wizard", 1), ("evoker", 1)]),
        g("Wizards (before): Wizard x2 + Evoker", &[("wizard", 2), ("evoker", 1)]),
        g("Taskmaster + Strip Miner x3", &[("taskmaster", 1), ("strip_miner", 3)]),
    ]
}

/// The packs of the route that a pull brings in together (a pack's `link` reaches the next, either
/// way), as lists of creature keys.
fn pulled_together() -> Vec<Vec<String>> {
    let c = content();
    let d = c.dungeon.as_ref().expect("the Deadmines");
    let n = d.packs.len();
    let mut group: Vec<usize> = (0..n).collect();
    for a in 0..n {
        for b in 0..a {
            let reach = d.packs[a].link.max(d.packs[b].link);
            let apart = ((d.packs[a].at[0] - d.packs[b].at[0]).powi(2) + (d.packs[a].at[1] - d.packs[b].at[1]).powi(2)).sqrt();
            if reach > 0.0 && apart <= reach {
                let (ga, gb) = (group[a], group[b]);
                group.iter_mut().filter(|g| **g == ga).for_each(|g| *g = gb);
            }
        }
    }
    let mut out: Vec<Vec<String>> = Vec::new();
    for g in 0..n {
        let keys: Vec<String> = (0..n).filter(|i| group[*i] == g).flat_map(|i| vec![d.packs[i].mob.clone(); d.packs[i].count.max(1)]).collect();
        if !keys.is_empty() {
            out.push(keys);
        }
    }
    out
}

/// Casters answer each other's rolls badly: every one casts on its own clock, and a roll costs
/// stamina the third bolt finds spent. Wizard x2 + Evoker, with no blade in front of them, felled a
/// right-playing warrior 3 and a hunter 4 times in 30 (two casters, 0 in 30); so a pull of three
/// shooters or casters has a creature up close in it.
#[test]
fn no_pull_is_three_casters_with_nobody_up_close() {
    let c = content();
    for keys in pulled_together() {
        let ranged = |k: &String| c.mob(k).is_some_and(|d| c.mobs[d].attack.ranged);
        let (far, near) = (keys.iter().filter(|k| ranged(k)).count(), keys.iter().filter(|k| !ranged(k)).count());
        assert!(far <= 2 || near > 0, "{keys:?}: {far} casters or shooters and nobody up close");
    }
}

/// The pull that was Wizard x2 + Evoker is two casters now, and a right-playing hero of every class
/// wins it (it fell a warrior in 10% of runs and a hunter in 13% before).
#[test]
fn the_caster_pair_at_the_deck_is_won_by_a_right_playing_hero_of_every_class() {
    let pair = Stage::Pack(vec!["dm_wizard".into(), "dm_evoker".into()]);
    for class in CLASSES {
        for seed in 1..=8 {
            let r = run(&pair, class, seed, RIGHT);
            assert!(r.won && !r.fell && r.damage * 10 < r.max_health * 9, "{class} seed {seed}: {r:?}");
        }
    }
}

#[test]
#[ignore]
fn balance_groups() {
    let only = std::env::var("BALANCE_ONLY").ok();
    let groups: Vec<_> = linked_groups().into_iter().filter(|(n, _)| only.as_ref().is_none_or(|o| n.contains(o.as_str()))).collect();
    let classes = classes_wanted();
    let seeds = std::env::var("BALANCE_SEEDS").ok().and_then(|x| x.parse().ok()).unwrap_or(5);
    let t = table(&groups, &classes, seeds);
    print("Linked groups", &groups, &classes, &t);
    println!("
| group | peak stack (right / sloppy) | dry rolls (right / sloppy) | landed blows (right) |
| --- | --- | --- | --- |");
    for (f, (name, _)) in groups.iter().enumerate() {
        let peak = |sl: bool| classes.iter().enumerate().flat_map(|(c, _)| if sl { t[f][c].1.runs.iter() } else { t[f][c].0.runs.iter() }).map(|r| r.peak_stack).max().unwrap_or(0);
        let dry = |sl: bool| classes.iter().enumerate().map(|(c, _)| if sl { t[f][c].1.mean(|r| r.dry as f32) } else { t[f][c].0.mean(|r| r.dry as f32) }).sum::<f32>() / classes.len() as f32;
        let landed = classes.iter().enumerate().map(|(c, _)| t[f][c].0.mean(|r| r.landed as f32)).sum::<f32>() / classes.len() as f32;
        println!("| {} | {} / {} | {:.0} / {:.0} | {:.1} |", name, peak(false), peak(true), dry(false), dry(true), landed);
    }
}

#[test]
#[ignore]
fn balance_one() {
    let foes = std::env::var("BALANCE_FOES").unwrap_or_else(|_| "dm_smite".into());
    let class = std::env::var("BALANCE_CLASS").unwrap_or_else(|_| "warrior".into());
    let seed = std::env::var("BALANCE_SEED").ok().and_then(|x| x.parse().ok()).unwrap_or(1);
    let who = if std::env::var("BALANCE_SLOPPY").is_ok() { SLOPPY } else { RIGHT };
    let boss = bosses().into_iter().find(|(_, k)| *k == foes);
    let st = match boss {
        Some((_, k)) => Stage::Boss(k),
        None => Stage::Pack(foes.split(',').map(String::from).collect()),
    };
    println!("{:?}", run(&st, &class, seed, who));
}

/// A fast check that the scripted hero plays: one pirate falls to a hero who answers its blades.
#[test]
fn a_scripted_hero_beats_one_pirate_and_a_pack_is_fought_to_its_end() {
    for class in ["warrior", "mage", "priest"] {
        let r = run(&Stage::Pack(vec!["dm_pirate".into()]), class, 1, RIGHT);
        assert!(r.won && !r.fell, "{class}: {r:?}");
        assert!(r.damage > 0 || r.parried + r.dodged > 0, "{class} was fought, not skipped: {r:?}");
    }
}
