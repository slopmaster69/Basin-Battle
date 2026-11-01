//! A solo balance simulation (not a check: `#[ignore]`d, run on purpose). Each class's own five
//! cards, played by a simple bot that stands its ground: it faces the nearest foe, uses its
//! defense when hurt, its ultimate when the Valor bar is full, its EX versions when it has Valor to
//! spare, otherwise whichever of its ranged, close and utility cards is ready and in reach, and its
//! weapon in between. Against a pack of three grunts and against Hogger alone, it reports how long
//! each class takes, what it lost doing it, and which cards it used.
//!
//! `cargo test --release -p htl-survival --lib solo_sim -- --ignored --nocapture`

use std::collections::BTreeMap;

use crate::content::{Content, SCHOOLS};
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.05;
const EYE: f32 = 1.9;

fn world() -> Survival {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
    s.mobs.clear();
    s.respawns.clear();
    s
}

/// A hero of `class` with that class's five cards on their keys.
fn hero(s: &mut Survival, class: &str) -> Id {
    let id = s.join(class, &format!("Sim{class}"), None);
    s.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().class = class.to_string();
    let cards: Vec<(String, String)> = s
        .content
        .abilities
        .iter()
        .filter(|a| a.school == class && a.on_board())
        .map(|a| (a.slot.clone(), a.key.clone()))
        .collect();
    for (slot, key) in cards {
        s.players.get_mut(&id).unwrap().owned.insert(key.clone());
        let _ = s.slot_ability(id, &slot, &key);
    }
    // Ten times the health, so every fight runs to its end: what's measured is how fast each class
    // kills and how much it takes doing it, not whether a bot that never guards or dodges survives.
    let p = s.players.get_mut(&id).unwrap();
    p.max_health *= 10;
    p.health = p.max_health;
    s.set_valor(id, 0.0);
    id
}

fn foe(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap();
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().pos = pos;
    id
}

fn alive(s: &Survival, m: Id) -> bool {
    s.mobs.get(&m).is_some_and(|x| x.health > 0)
}

/// What a fight came to.
struct Fight {
    secs: f32,
    won: bool,
    lost: u32,
    healed: u32,
    dealt: u32,
    missed: u32,
    casts: BTreeMap<String, u32>,
}

/// The bot's state between ticks: the ability it's aiming and how long it has held the click.
struct Bot {
    held: Option<f32>,
    want: f32,
}

fn fight(class: &str, foes: &[(&str, Vec3)], cap: f32) -> Fight {
    let mut s = world();
    let p = hero(&mut s, class);
    let ids: Vec<Id> = foes.iter().map(|(k, at)| foe(&mut s, k, *at)).collect();
    let mut bot = Bot { held: None, want: 0.0 };
    let mut out = Fight { secs: 0.0, won: false, lost: 0, healed: 0, dealt: 0, missed: 0, casts: BTreeMap::new() };
    let mut last_hp = s.players[&p].health;
    let mut t = 0.0;
    while t < cap {
        if ids.iter().all(|m| !alive(&s, *m)) {
            out.won = true;
            break;
        }
        if s.players[&p].health == 0 || matches!(s.players[&p].action, Action::Dead { .. }) {
            break;
        }
        think(&mut s, p, &ids, &mut bot, &mut out);
        for e in s.step(DT) {
            if let Event::Swing { attacker, damage, outcome, .. } = e {
                if attacker == p {
                    if outcome == Outcome::Hit {
                        out.dealt += damage;
                    } else {
                        out.missed += 1;
                    }
                }
            }
        }
        t += DT;
        let hp = s.players[&p].health;
        if hp < last_hp {
            out.lost += last_hp - hp;
        } else {
            out.healed += hp - last_hp;
        }
        last_hp = hp;
    }
    out.secs = t;
    out
}

fn think(s: &mut Survival, p: Id, foes: &[Id], bot: &mut Bot, out: &mut Fight) {
    let me = s.players[&p].pos;
    let Some((target, at)) = foes.iter().filter(|m| alive(s, **m)).map(|m| (*m, s.mobs[m].pos)).min_by(|a, b| dist2d(me, a.1).total_cmp(&dist2d(me, b.1))) else {
        return;
    };
    let _ = target;
    let d = dist2d(me, at);
    let yaw = (at[1] - me[1]).atan2(at[0] - me[0]);
    let action = s.players[&p].action.clone();
    // Face the foe whenever the hands are free (never mid-dash or leap: the rules move the hero then).
    if matches!(action, Action::Ready | Action::Aim { .. } | Action::Charge { .. } | Action::Channel { .. }) {
        s.move_player(p, me, yaw, Moving::default());
    }
    match action {
        Action::Aim { ability } => {
            let a = &s.content.abilities[ability];
            // On the ground at the foe for what lands there, at its middle for what flies.
            let ground = matches!(a.kind.as_str(), "ground" | "trap" | "place") || a.lob;
            // Up close a charge would only be broken by the blows: a quick release there, unless the
            // card must be held to its end to go off at all (Pyroblast, Chaos Bolt: they wait).
            let fixed = a.charge[0] >= a.charge[1] && a.charge[1] > 0.0;
            let want = if d < 6.0 && !fixed { a.charge[0] + 0.1 } else { a.charge[1] };
            let elev = if ground { -(EYE / d.max(1.0)).atan() } else { ((1.0 - EYE) / d.max(1.0)).atan() };
            s.set_aim_line(p, yaw, elev);
            bot.want = want;
            bot.held = Some(0.0);
            s.input(p, Input::Attack);
        }
        Action::Charge { .. } => {
            let elev = ((1.0 - EYE) / d.max(1.0)).atan();
            s.set_aim_line(p, yaw, elev);
            let h = bot.held.unwrap_or(0.0) + DT;
            bot.held = Some(h);
            if h >= bot.want + DT {
                s.input(p, Input::AttackUp);
                bot.held = None;
            }
        }
        Action::Ready => {
            if bot.held.take().is_some() {
                s.input(p, Input::AttackUp);
                return;
            }
            let bar = s.bar(p);
            let valor = s.players[&p].valor;
            let hp = s.players[&p].health as f32 / s.players[&p].max_health as f32;
            let ready = |slot: usize| bar[slot].1.is_some() && bar[slot].3.charges > 0;
            let reach = |s: &Survival, a: usize| {
                let x = &s.content.abilities[a];
                let r = match x.kind.as_str() {
                    "nova" | "cone" | "art" | "beam" => x.range[1].max(x.radius[1]).max(4.0),
                    "buff" | "stance" | "ally" => 99.0,
                    _ => x.range[1].max(4.0),
                };
                d <= r
            };
            let mut pick: Option<(usize, bool)> = None;
            if hp < 0.5 && ready(3) {
                pick = Some((3, false));
            } else if valor >= 100.0 && ready(4) && reach(s, bar[4].1.unwrap()) {
                pick = Some((4, false));
            } else {
                for slot in [0usize, 1, 2] {
                    if !ready(slot) || !reach(s, bar[slot].1.unwrap()) {
                        continue;
                    }
                    let base = bar[slot].1.unwrap();
                    let d = &s.content.abilities[base];
                    // Utility cards that don't strike (a blink, a speed burst) wait for a reason.
                    let strikes = d.damage[1] > 0.0 || d.effects.iter().any(|e| ["stun", "root", "incapacitate", "panic", "knockback", "pull", "snare"].contains(&e.kind.as_str())) || !d.recast.is_empty();
                    if slot == 2 && !strikes {
                        continue;
                    }
                    let ex = bar[slot].2.is_some_and(|x| valor >= s.content.abilities[x].valor as f32 + 25.0);
                    pick = Some((slot, ex));
                    break;
                }
            }
            if let Some((slot, ex)) = pick {
                let key = match (ex, bar[slot].2) {
                    (true, Some(x)) => s.content.abilities[x].name.clone(),
                    _ => s.content.abilities[bar[slot].1.unwrap()].name.clone(),
                };
                *out.casts.entry(key).or_default() += 1;
                s.input(p, Input::Ability(slot as u8, ex));
            } else if d <= 3.5 {
                // The weapon in between.
                s.input(p, Input::Attack);
                bot.held = Some(0.0);
                bot.want = 0.0;
            }
        }
        _ => {}
    }
}

#[test]
#[ignore]
fn solo_sim() {
    let pack: Vec<(&str, Vec3)> = vec![("grunt", [12.0, 0.0, 0.0]), ("grunt", [13.0, 3.0, 0.0]), ("grunt", [13.0, -3.0, 0.0])];
    let boss: Vec<(&str, Vec3)> = vec![("hogger", [15.0, 0.0, 0.0])];
    println!();
    println!("SIM class    | 3 grunts: secs  dps lost/s healed miss | Hogger: secs  dps lost/s healed miss | cards used (pack + Hogger)");
    for class in SCHOOLS {
        let a = fight(class, &pack, 120.0);
        let b = fight(class, &boss, 180.0);
        let mut casts = a.casts.clone();
        for (k, n) in &b.casts {
            *casts.entry(k.clone()).or_default() += n;
        }
        let used: Vec<String> = casts.iter().map(|(k, n)| format!("{k} {n}")).collect();
        println!(
            "SIM {class:8} | {:6.1}{} {:5.1} {:6.1} {:6} {:4} | {:6.1}{} {:5.1} {:6.1} {:6} {:4} | {}",
            a.secs,
            if a.won { " " } else { "+" },
            a.dealt as f32 / a.secs.max(1.0),
            a.lost as f32 / a.secs.max(1.0),
            a.healed,
            a.missed,
            b.secs,
            if b.won { " " } else { "+" },
            b.dealt as f32 / b.secs.max(1.0),
            b.lost as f32 / b.secs.max(1.0),
            b.healed,
            b.missed,
            used.join(", ")
        );
    }
}
