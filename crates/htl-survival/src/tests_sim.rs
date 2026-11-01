//! The solo balance simulation, after SimulationCraft's way of working, but on the game's own
//! rules: the real sim runs every fight, so a missed skillshot or a broken charge counts as it would
//! for a player.
//!
//! - **Fight styles**: the same few fights for every class ([`STYLES`]): a pack of three grunts, a
//!   big pack of six, archers that keep their distance, and Hogger alone.
//! - **Many runs**: each fight is played with different seeds (`SIM_RUNS`, 10 by default) and
//!   reported as its mean, spread and range, so a real gap shows apart from a lucky run.
//! - **A plan for each class** (`content/sim.toml`): the class's simplest tactics, tried before
//!   the plain bot (a Frost Nova and a step back, a Shadowfury and a Chaos Bolt on the stunned).
//! - **What-if sweeps**: `SIM_TUNE="ability.penance.damage=[16,16]|[20,20]|[24,24]"` plays the
//!   fights once for each value, so a number can be picked from the curve.
//! - **A band** ([`classes_clear_a_pack_within_a_band`], a real test): every class clears the pack,
//!   none more than [`BAND`] times slower than the fastest, so a tuning change can't quietly leave
//!   a class behind.
//!
//! The hero has ten times their health, so every fight runs to its end: what's measured is how fast
//! each class kills and how much it takes doing it, not whether a bot that never guards or dodges
//! lives. It's a guide, not a verdict.
//!
//! ```text
//! cargo test --release -p htl-survival --lib solo_sim -- --ignored --nocapture
//! SIM_CLASS=mage SIM_STYLE=pack SIM_RUNS=20 cargo test ...   (one class, one fight)
//! ```

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::content::{Content, SCHOOLS};
use crate::land::FlatLand;
use crate::sim::*;

const DT: f32 = 0.05;
const EYE: f32 = 1.9;
/// The band: no class's mean time on the pack more than this many times the fastest's.
const BAND: f32 = 3.0;
/// Farther than this from the nearest foe, the bot walks back in.
const APPROACH: f32 = 16.0;
/// The ground the bot keeps to: it steps back, blinks or leaps out only within this of where the
/// fight began, so it never drags a foe past its leash (home again, it's whole again).
const ROOM: f32 = 8.0;
const PLANS: &str = include_str!("../../../content/sim.toml");

/// A fight every class plays: its name, what it is, the foes and where, and how long it may run.
struct Style {
    key: &'static str,
    about: &'static str,
    foes: &'static [(&'static str, [f32; 3])],
    cap: f32,
}

const STYLES: &[Style] = &[
    Style { key: "pack", about: "three grunts, coming in", foes: &[("grunt", [12.0, 0.0, 0.0]), ("grunt", [13.0, 3.0, 0.0]), ("grunt", [13.0, -3.0, 0.0])], cap: 120.0 },
    Style {
        key: "bigpack",
        about: "six grunts, coming in",
        foes: &[
            ("grunt", [12.0, 0.0, 0.0]),
            ("grunt", [13.0, 3.0, 0.0]),
            ("grunt", [13.0, -3.0, 0.0]),
            ("grunt", [15.0, 1.5, 0.0]),
            ("grunt", [15.0, -1.5, 0.0]),
            ("grunt", [16.0, 0.0, 0.0]),
        ],
        cap: 180.0,
    },
    Style { key: "archers", about: "two archers keeping their distance, a grunt between", foes: &[("archer", [18.0, 4.0, 0.0]), ("archer", [18.0, -4.0, 0.0]), ("grunt", [12.0, 0.0, 0.0])], cap: 150.0 },
    Style { key: "hogger", about: "Hogger alone", foes: &[("hogger", [15.0, 0.0, 0.0])], cap: 180.0 },
];

// --- the plans ------------------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
struct Plans {
    #[serde(default)]
    class: BTreeMap<String, Vec<Rule>>,
}

/// One rule of a class's plan (see `content/sim.toml`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct Rule {
    card: Option<String>,
    #[serde(default)]
    weapon: bool,
    foes_within: Option<(f32, usize)>,
    target_within: Option<f32>,
    target_beyond: Option<f32>,
    health_below: Option<f32>,
    valor_at_least: Option<f32>,
    target_has: Option<String>,
    target_lacks: Option<String>,
    #[serde(default)]
    away: bool,
    then_back: Option<f32>,
}

fn plans() -> Plans {
    toml::from_str(PLANS).expect("content/sim.toml reads")
}

// --- one fight ------------------------------------------------------------------------------------

fn world(c: &Content, seed: u64) -> Survival {
    let mut c = c.clone();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    let mut s = Survival::new(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
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
    let cards: Vec<(String, String)> = s.content.abilities.iter().filter(|a| a.school == class && a.on_board()).map(|a| (a.slot.clone(), a.key.clone())).collect();
    for (slot, key) in cards {
        s.players.get_mut(&id).unwrap().owned.insert(key.clone());
        let _ = s.slot_ability(id, &slot, &key);
    }
    let p = s.players.get_mut(&id).unwrap();
    p.max_health *= 10;
    p.health = p.max_health;
    s.set_valor(id, 0.0);
    id
}

fn foe(s: &mut Survival, key: &str, pos: Vec3) -> Id {
    let def = s.content.mob(key).unwrap_or_else(|| panic!("no creature {key}"));
    let id = s.spawn_mob(def, pos, None, false);
    s.mobs.get_mut(&id).unwrap().pos = pos;
    id
}

fn alive(s: &Survival, m: Id) -> bool {
    s.mobs.get(&m).is_some_and(|x| x.health > 0)
}

/// What a fight came to.
#[derive(Debug, Clone, Default)]
struct Fight {
    secs: f32,
    won: bool,
    lost: u32,
    dealt: u32,
    missed: u32,
    casts: BTreeMap<String, u32>,
}

/// The bot between ticks: the click it's holding (how long, and for how long it means to), the
/// aim it's taking (turned away), and until when it's backing off.
#[derive(Default)]
struct Bot {
    held: Option<f32>,
    want: f32,
    away: bool,
    back_until: f32,
    /// Keys that didn't go off when pressed, left alone until then.
    refused: BTreeMap<usize, f32>,
}

/// Press a key (its EX version with `ex`): counted as used when it went off; when it didn't (out of
/// reach, no target, refused), left alone for a second.
fn press(s: &mut Survival, p: Id, slot: usize, ex: bool, bot: &mut Bot, out: &mut Fight) {
    let bar = s.bar(p);
    let before = (s.players[&p].action.clone(), bar[slot].3.charges, s.players[&p].valor);
    let a = if ex { bar[slot].2 } else { bar[slot].1 };
    s.input(p, Input::Ability(slot as u8, ex));
    let after = (s.players[&p].action.clone(), s.bar(p)[slot].3.charges, s.players[&p].valor);
    if after == before {
        bot.refused.insert(slot, s.clock + 1.0);
        return;
    }
    if let Some(a) = a {
        *out.casts.entry(s.content.abilities[a].name.clone()).or_default() += 1;
    }
}

fn refused(s: &Survival, bot: &Bot, slot: usize) -> bool {
    bot.refused.get(&slot).is_some_and(|t| *t > s.clock)
}

fn fight(c: &Content, plan: &[Rule], class: &str, style: &Style, seed: u64) -> Fight {
    let mut s = world(c, seed);
    let p = hero(&mut s, class);
    let ids: Vec<Id> = style.foes.iter().map(|(k, at)| foe(&mut s, k, *at)).collect();
    let mut bot = Bot::default();
    let mut out = Fight::default();
    let mut last_hp = s.players[&p].health;
    let mut t = 0.0;
    // SIM_TRACE=1: the fight a second at a time (where everyone is, what they're doing).
    let trace = std::env::var("SIM_TRACE").is_ok_and(|v| v == "1");
    let mut tick = 0usize;
    while t < style.cap {
        if trace && tick % 20 == 0 {
            let h = &s.players[&p];
            let foes: Vec<String> = ids
                .iter()
                .filter_map(|m| s.mobs.get(m))
                .map(|m| format!("{:.0}yd {}hp {:?}", dist2d(h.pos, m.pos), m.health, m.state).chars().take(60).collect())
                .collect();
            let doing: String = format!("{:?}", h.action).chars().take(40).collect();
            println!("TRACE {t:5.1} {class} at [{:.0},{:.0}] {}hp {doing} valor {:.0} | {}", h.pos[0], h.pos[1], h.health, h.valor, foes.join(" | "));
        }
        tick += 1;
        if ids.iter().all(|m| !alive(&s, *m)) {
            out.won = true;
            break;
        }
        if s.players[&p].health == 0 || matches!(s.players[&p].action, Action::Dead { .. }) {
            break;
        }
        think(&mut s, p, &ids, plan, &mut bot, &mut out);
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
        out.lost += last_hp.saturating_sub(hp);
        last_hp = hp;
    }
    out.secs = t;
    out
}

/// The card on a key, and whether it's that key's EX version.
fn key_for(s: &Survival, p: Id, card: &str) -> Option<(usize, bool)> {
    let a = s.content.ability(card)?;
    let bar = s.bar(p);
    bar.iter().enumerate().find_map(|(slot, (_, base, ex, _))| {
        if *base == Some(a) {
            Some((slot, false))
        } else if *ex == Some(a) {
            Some((slot, true))
        } else {
            None
        }
    })
}

/// Whether a card would reach a foe `d` yd away.
fn reaches(s: &Survival, a: usize, d: f32) -> bool {
    let x = &s.content.abilities[a];
    let r = match x.kind.as_str() {
        "nova" | "cone" | "art" | "beam" => x.range[1].max(x.radius[1]).max(4.0),
        "buff" | "stance" | "ally" | "blink" | "leap" => 99.0,
        _ => x.range[1].max(4.0),
    };
    d <= r
}

/// The first rule of the plan that holds, as the key to press (`None`: none does).
fn planned(s: &Survival, p: Id, foes: &[Id], target: Id, d: f32, plan: &[Rule], bot: &Bot) -> Option<(Rule, Option<(usize, bool)>)> {
    let me = s.players[&p].pos;
    let hp = s.players[&p].health as f32 / s.players[&p].max_health as f32;
    let valor = s.players[&p].valor;
    let bar = s.bar(p);
    for r in plan {
        let near = |yd: f32| foes.iter().filter(|m| alive(s, **m) && dist2d(me, s.mobs[m].pos) <= yd).count();
        let holds = r.foes_within.is_none_or(|(yd, n)| near(yd) >= n)
            && r.target_within.is_none_or(|yd| d <= yd)
            && r.target_beyond.is_none_or(|yd| d > yd)
            && r.health_below.is_none_or(|h| hp < h)
            && r.valor_at_least.is_none_or(|v| valor >= v)
            && r.target_has.as_ref().is_none_or(|k| s.has_cc(target, k))
            && r.target_lacks.as_ref().is_none_or(|k| !s.has_cc(target, k));
        if !holds {
            continue;
        }
        if r.weapon {
            if d <= 3.5 {
                return Some((r.clone(), None));
            }
            continue;
        }
        let Some(card) = &r.card else { continue };
        let Some((slot, ex)) = key_for(s, p, card) else { continue };
        let a = if ex { bar[slot].2 } else { bar[slot].1 };
        let outward = r.away || r.then_back.is_some() || a.is_some_and(|a| matches!(s.content.abilities[a].kind.as_str(), "leap" | "blink") && s.content.abilities[a].back);
        if outward && dist2d(me, [0.0, 0.0, 0.0]) > ROOM {
            continue;
        }
        let ready = bar[slot].1.is_some() && bar[slot].3.charges > 0 && !refused(s, bot, slot);
        let Some(a) = a else { continue };
        let affords = !ex || valor >= s.content.abilities[a].valor as f32;
        if ready && affords && reaches(s, a, d) {
            return Some((r.clone(), Some((slot, ex))));
        }
    }
    None
}

fn think(s: &mut Survival, p: Id, foes: &[Id], plan: &[Rule], bot: &mut Bot, out: &mut Fight) {
    let me = s.players[&p].pos;
    let Some((target, at)) = foes.iter().filter(|m| alive(s, **m)).map(|m| (*m, s.mobs[m].pos)).min_by(|a, b| dist2d(me, a.1).total_cmp(&dist2d(me, b.1))) else {
        return;
    };
    let d = dist2d(me, at);
    let yaw = (at[1] - me[1]).atan2(at[0] - me[0]);
    let action = s.players[&p].action.clone();
    let free = matches!(action, Action::Ready | Action::Aim { .. } | Action::Charge { .. } | Action::Channel { .. });
    // Backing off: a stride away from the foe, still facing it.
    if bot.back_until > s.clock && free && dist2d(me, [0.0, 0.0, 0.0]) < ROOM + 6.0 {
        let speed = s.players[&p].sent_speed.max(3.0);
        let step = speed * DT;
        let to = [me[0] - yaw.cos() * step, me[1] - yaw.sin() * step, me[2]];
        s.move_player(p, to, yaw, Moving { back: true, ..Moving::default() });
    } else if free && d > APPROACH {
        // Drifted far off (a leap out, a step back, foes gone home past their leash): back in, as a
        // player would.
        let speed = s.players[&p].sent_speed.max(3.0);
        let step = speed * DT;
        let to = [me[0] + yaw.cos() * step, me[1] + yaw.sin() * step, me[2]];
        s.move_player(p, to, yaw, Moving { forward: true, ..Moving::default() });
    } else if free {
        // Face the foe whenever the hands are free (never mid-dash or leap: the rules move the hero).
        s.move_player(p, me, yaw, Moving::default());
    }
    match action {
        Action::Aim { ability } => {
            let a = &s.content.abilities[ability];
            // On the ground at the foe for what lands there, at its middle for what flies; turned
            // away, on open ground well behind.
            let ground = matches!(a.kind.as_str(), "ground" | "trap" | "place" | "blink") || a.lob;
            let fixed = a.charge[0] >= a.charge[1] && a.charge[1] > 0.0;
            // Up close a charge would only be broken by the blows: a quick release there, unless the
            // card must be held to its end to go off at all (Pyroblast, Chaos Bolt: they wait).
            let want = if d < 6.0 && !fixed { a.charge[0] + 0.1 } else { a.charge[1] };
            let (aim_yaw, aim_d) = if bot.away { (yaw + std::f32::consts::PI, 15.0) } else { (yaw, d) };
            let elev = if ground { -(EYE / aim_d.max(1.0)).atan() } else { ((1.0 - EYE) / aim_d.max(1.0)).atan() };
            s.set_aim_line(p, aim_yaw, elev);
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
            bot.away = false;
            // The class's plan first.
            if let Some((rule, key)) = planned(s, p, foes, target, d, plan, bot) {
                if let Some(back) = rule.then_back {
                    bot.back_until = s.clock + back;
                }
                match key {
                    Some((slot, ex)) => {
                        if rule.away {
                            bot.away = true;
                            let here = s.players[&p].pos;
                            s.move_player(p, here, yaw + std::f32::consts::PI, Moving::default());
                        }
                        press(s, p, slot, ex, bot, out);
                    }
                    None => {
                        s.input(p, Input::Attack);
                        bot.held = Some(0.0);
                        bot.want = 0.0;
                    }
                }
                return;
            }
            plain(s, p, d, bot, out);
        }
        _ => {}
    }
}

/// The plain bot: the defense when hurt, the ultimate on a full bar, an EX version with Valor to
/// spare, otherwise whatever's ready and in reach, and the weapon up close.
fn plain(s: &mut Survival, p: Id, d: f32, bot: &mut Bot, out: &mut Fight) {
    let bar = s.bar(p);
    let valor = s.players[&p].valor;
    let hp = s.players[&p].health as f32 / s.players[&p].max_health as f32;
    let ready = |slot: usize| bar[slot].1.is_some() && bar[slot].3.charges > 0 && !refused(s, bot, slot);
    let mut pick: Option<(usize, bool)> = None;
    if hp < 0.5 && ready(3) {
        pick = Some((3, false));
    } else if valor >= 100.0 && ready(4) && reaches(s, bar[4].1.unwrap(), d) {
        pick = Some((4, false));
    } else {
        for slot in [0usize, 1, 2] {
            if !ready(slot) || !reaches(s, bar[slot].1.unwrap(), d) {
                continue;
            }
            let base = bar[slot].1.unwrap();
            let a = &s.content.abilities[base];
            // Utility cards that don't strike (a blink, a speed burst) wait for a reason.
            let strikes = a.damage[1] > 0.0 || a.effects.iter().any(|e| ["stun", "root", "incapacitate", "panic", "knockback", "pull", "snare"].contains(&e.kind.as_str())) || !a.recast.is_empty();
            if slot == 2 && !strikes {
                continue;
            }
            let ex = bar[slot].2.is_some_and(|x| valor >= s.content.abilities[x].valor as f32 + 25.0);
            pick = Some((slot, ex));
            break;
        }
    }
    if let Some((slot, ex)) = pick {
        press(s, p, slot, ex, bot, out);
    } else if d <= 3.5 {
        s.input(p, Input::Attack);
        bot.held = Some(0.0);
        bot.want = 0.0;
    }
}

// --- many runs ------------------------------------------------------------------------------------

/// A class's runs of one fight, summed up.
struct Summary {
    runs: usize,
    won: usize,
    mean: f32,
    sd: f32,
    min: f32,
    max: f32,
    dps: f32,
    took: f32,
    missed: f32,
    casts: BTreeMap<String, u32>,
}

fn summarize(fights: &[Fight]) -> Summary {
    let n = fights.len().max(1) as f32;
    let secs: Vec<f32> = fights.iter().map(|f| f.secs).collect();
    let mean = secs.iter().sum::<f32>() / n;
    let sd = (secs.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / n).sqrt();
    let mut casts = BTreeMap::new();
    for f in fights {
        for (k, c) in &f.casts {
            *casts.entry(k.clone()).or_default() += c;
        }
    }
    Summary {
        runs: fights.len(),
        won: fights.iter().filter(|f| f.won).count(),
        mean,
        sd,
        min: secs.iter().copied().fold(f32::MAX, f32::min),
        max: secs.iter().copied().fold(0.0, f32::max),
        dps: fights.iter().map(|f| f.dealt as f32 / f.secs.max(1.0)).sum::<f32>() / n,
        took: fights.iter().map(|f| f.lost as f32 / f.secs.max(1.0)).sum::<f32>() / n,
        missed: fights.iter().map(|f| f.missed as f32).sum::<f32>() / n,
        casts,
    }
}

fn run(c: &Content, plans: &Plans, class: &str, style: &Style, runs: usize) -> Summary {
    let plan = plans.class.get(class).cloned().unwrap_or_default();
    let fights: Vec<Fight> = (0..runs).map(|i| fight(c, &plan, class, style, 7 + i as u64 * 7919)).collect();
    summarize(&fights)
}

fn env_list(name: &str) -> Option<Vec<String>> {
    std::env::var(name).ok().filter(|v| !v.trim().is_empty()).map(|v| v.split(',').map(|x| x.trim().to_string()).collect())
}

fn report(c: &Content, plans: &Plans, runs: usize, classes: &[String], styles: &[&Style]) {
    for style in styles {
        println!();
        println!("SIM == {}: {} (up to {:.0} s), {runs} runs each", style.key, style.about, style.cap);
        println!("SIM class    |   mean    sd     min-max   won |  dps  took/s  miss | vs best  cards a run");
        let mut rows: Vec<(String, Summary)> = classes.iter().map(|class| (class.clone(), run(c, plans, class, style, runs))).collect();
        rows.sort_by(|a, b| a.1.mean.total_cmp(&b.1.mean));
        let best = rows.first().map_or(1.0, |r| r.1.mean.max(0.1));
        for (class, r) in &rows {
            let mut used: Vec<(&String, &u32)> = r.casts.iter().collect();
            used.sort_by(|a, b| b.1.cmp(a.1));
            let used: Vec<String> = used.iter().take(5).map(|(k, n)| format!("{k} {:.1}", **n as f32 / r.runs as f32)).collect();
            println!(
                "SIM {class:8} | {:6.1} {:5.1} {:5.0}-{:<5.0} {:2}/{:<2} | {:4.1} {:6.1} {:5.1} | x{:.2}  {}",
                r.mean,
                r.sd,
                r.min,
                r.max,
                r.won,
                r.runs,
                r.dps,
                r.took,
                r.missed,
                r.mean / best,
                used.join(", ")
            );
        }
    }
}

#[test]
#[ignore]
fn solo_sim() {
    let runs: usize = std::env::var("SIM_RUNS").ok().and_then(|v| v.parse().ok()).unwrap_or(10).max(1);
    let classes: Vec<String> = env_list("SIM_CLASS").unwrap_or_else(|| SCHOOLS.iter().map(|s| s.to_string()).collect());
    let picked = env_list("SIM_STYLE");
    let styles: Vec<&Style> = STYLES.iter().filter(|s| picked.as_ref().is_none_or(|p| p.iter().any(|k| k == s.key))).collect();
    let plans = plans();
    // A sweep: the fights once for each value of one number.
    if let Ok(tune) = std::env::var("SIM_TUNE") {
        let (path, values) = tune.split_once('=').expect("SIM_TUNE=path=value|value|...");
        for v in values.split('|') {
            let (text, _) = crate::tune::set(crate::content::DEFAULT, path, v).expect("a path survival.toml has");
            let c = Content::parse_with(&text, crate::content::DEFAULT_DUNGEON).expect("the tuned rules read");
            let mut c = dev(c);
            c.dev.full_tree = true;
            println!();
            println!("SIM ######## {path} = {v}");
            report(&c, &plans, runs, &classes, &styles);
        }
        return;
    }
    report(&Content::dev(), &plans, runs, &classes, &styles);
}

/// The dev rules' changes from the built-in ones, on rules read some other way.
fn dev(mut c: Content) -> Content {
    let d = Content::dev();
    c.dev = d.dev.clone();
    c.work = d.work.clone();
    c.combat.foe_health = d.combat.foe_health;
    c.combat.foe_damage = d.combat.foe_damage;
    c.combat.hero_health = d.combat.hero_health;
    c.combat.hero_damage = d.combat.hero_damage;
    c
}

#[test]
fn the_class_plans_read_and_name_real_cards_of_their_class() {
    let c = Content::dev();
    let plans = plans();
    for (class, rules) in &plans.class {
        assert!(SCHOOLS.contains(&class.as_str()), "{class}: no such class");
        for r in rules {
            assert!(r.card.is_some() != r.weapon, "{class}: a rule presses a card or swings the weapon: {r:?}");
            if let Some(card) = &r.card {
                let a = c.ability(card).unwrap_or_else(|| panic!("{class}: no card {card}"));
                assert_eq!(c.abilities[c.base_of(a)].school, *class, "{class}: {card} is another class's");
            }
        }
    }
}

/// Every class clears a pack of three grunts, none more than [`BAND`] times slower than the
/// fastest (three runs each).
#[test]
fn classes_clear_a_pack_within_a_band() {
    let c = Content::dev();
    let plans = plans();
    let pack = STYLES.iter().find(|s| s.key == "pack").unwrap();
    let times: Vec<(&str, Summary)> = SCHOOLS.iter().map(|class| (*class, run(&c, &plans, class, pack, 3))).collect();
    let best = times.iter().map(|t| t.1.mean).fold(f32::MAX, f32::min);
    for (class, t) in &times {
        assert_eq!(t.won, t.runs, "{class} didn't clear the pack every time ({:.1} s on average)", t.mean);
        assert!(t.mean <= best * BAND, "{class} took {:.1} s on the pack, more than {BAND} times the fastest class's {best:.1} s", t.mean);
    }
}
