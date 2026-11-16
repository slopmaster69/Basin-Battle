//! A long SOAK of the sim, headless (`#[ignore]`d: it takes minutes to hours of real time). It
//! finds what only shows over time: a collection nothing prunes, a creature that never leaves a
//! state, a boss that never resets after a wipe, a position gone NaN after the thousandth cast.
//!
//! Run it in release, with a log on stderr:
//!
//! ```text
//! SOAK_MINUTES=120 SOAK_SEED=20261115 cargo test --release -p htl-survival soak_ -j 8 -- --ignored --nocapture --test-threads 1
//! ```
//!
//! - `SOAK_MINUTES` (default 120): simulated minutes in all, at 20 Hz. Elwynn gets 45 %, the
//!   dungeons together 40 %, the Basin Battle siege 15 %. The scouts' whole route takes about 170
//!   simulated minutes of Elwynn (so a run of at least 400 here); give it more to see what the
//!   finished camp does over hours (a run of 1500 is 15 minutes of real time).
//! - `SOAK_SEED` (default 20261115): the seed. It is printed at the start of every phase, so a
//!   failure reproduces with the same two variables.
//! - `SOAK_ONLY` (optional): `elwynn`, `dungeons` or `siege`, to run one phase alone.
//! - `SOAK_SIZES=1` also prints every collection's size at the start, a third in, two thirds in
//!   and at the end; `SOAK_DEBUG=1` prints each dungeon minute's bosses and heroes;
//!   `SOAK_DEATHS=1` logs each hero's first death with what was near.
//! - `SOAK_TRACE=a,b` prints what the journey's scouts see as it happens: `tells` (what the camp
//!   says to them), `chest` (each chest they open), or a creature's key (its state every second
//!   of the fight with it), or `escort` (the seal-cutter's road: where Pell is, and which leg).
//!
//! It runs about 3 simulated hours of Elwynn a minute in release. The Basin Battle card mode (the
//! `htl-sim` crate) has its own, `soak_card_mode_run_after_run` in `htl-sim/src/tests.rs`, run
//! the same way with `-p htl-sim`.
//!
//! The drivers: in Elwynn, a band of four heroes of mixed classes pressing anything at random
//! (every ability and its EX, attack, guard, dodge, bash, kick), walking at the nearest creature,
//! taking weapons from the rack, dying, reviving, leaving and rejoining (sometimes all of them at
//! once); at camp, building every station, raids rung, chests opened; below, each dungeon's
//! bosses pulled, wiped on and pulled again, the run reset; in the siege, whole matches of sends
//! and trainer cards, one after another.
//!
//! And in Elwynn two strong scouts PLAY THE WORLD (`soak_tests/journey.rs`), deed by deed along
//! their `ROUTE` (the camp has no journal): banking, crafting, building, wearing, exploring,
//! finding secrets, opening chests (guardians first, trials won), learning (shrines, relics,
//! champions), killing (a creature in Elwynn, or in the dungeon whose pack it is), taking the
//! Horde's camps, raids met at the heart, dungeons entered by the book, cleared boss by boss, left by a lamp and the
//! news sent home (`Event::ToCamp`) as the server does it, with the camp's people spoken to and
//! their links followed, and the seal-cutter's escort walked beside him to the gate. Each dungeon is its own world beside the camp's. The scouts' blows are
//! `hurt_mob_as` slices (a dev world's abilities do no damage) while they also press buttons at
//! what they fight, so every boss's phases, adds, holds, doors and deaths run. When the route
//! is done they go on: the chests again in order, every place, every camp, every dungeon cleared
//! again. A deed that stays the route's next for three simulated hours fails the run (`SOAK STUCK
//! route`); one the pilot does not know how to play is named (`SOAK ROUTE UNPLAYED`).
//!
//! Every tick: nothing panics and positions and health are finite and in range. Every simulated
//! minute: the size of each collection of the sim is sampled (a debug-string length stands for
//! the ones that hold collections inside, the structs with no `Debug` report their own with
//! `sizes()`), the camp's progress is checked for validity, and creatures are checked for being
//! stuck. At the end of a phase: any collection that only ever grew, and the tick time's creep,
//! are reported (`SOAK GROWS`, `SOAK CREEP`), and anything stuck is a failure (`SOAK STUCK`). The
//! run fails (panics) on a broken invariant, a stuck creature or a boss that did not reset; growth
//! is reported for a human to judge, and fails only past `GROWTH_FAIL` (a collection that grew
//! without bound is a leak).

use super::*;
use crate::content::{Content, SCHOOLS, SLOTS};
use crate::land::FlatLand;
use super::mobs::Foe;
use std::collections::BTreeMap as Map;
use std::time::Instant;

mod journey;

const DT: f32 = 0.05;
/// What a hero below has to stand the bosses' blows (a dev world's heroes are not tuned to them).
const DUNGEON_HP: u32 = 150_000;
const TICKS_PER_MIN: usize = 1200;
/// A collection that ends a phase this many times bigger than it was a third of the way in, and
/// over `GROWTH_MIN`, without ever having shrunk in the last half, is a leak.
const GROWTH_FAIL: f64 = 4.0;
const GROWTH_MIN: f64 = 2_000.0;
/// A creature in one fighting state this long (simulated seconds), or on its way home this long,
/// is stuck.
const STUCK_FIGHT: f32 = 900.0;
const STUCK_RETURN: f32 = 300.0;
const STUCK_DEAD: f32 = 120.0;

struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f32 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 33) as f32) / (1u64 << 31) as f32
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f32) as usize).min(n.max(1) - 1)
    }

    fn chance(&mut self, p: f32) -> bool {
        self.next() < p
    }
}

fn minutes() -> f32 {
    std::env::var("SOAK_MINUTES").ok().and_then(|v| v.parse().ok()).unwrap_or(120.0)
}

fn seed() -> u64 {
    std::env::var("SOAK_SEED").ok().and_then(|v| v.parse().ok()).unwrap_or(20261115)
}

fn only(phase: &str) -> bool {
    std::env::var("SOAK_ONLY").map_or(true, |v| v == phase)
}

// --- the monitor ----------------------------------------------------------------------------------

/// What a phase has seen: each collection's size by minute, the ticks' times, what's stuck.
struct Monitor {
    phase: String,
    sizes: Map<&'static str, Vec<f64>>,
    tick_ms: Vec<f64>,
    minute_ms: Vec<(f64, f64)>,
    stuck: Map<Id, (u8, f32)>,
    stuck_found: Vec<String>,
    dbg: std::collections::BTreeSet<Id>,
    reported: std::collections::BTreeSet<Id>,
    started: Instant,
    last_found: usize,
    last_raids: u32,
    min_ticks: usize,
    last_hidden: Map<Id, String>,
    odd: std::collections::BTreeSet<Id>,
}

fn debug_len<T: std::fmt::Debug>(x: &T) -> f64 {
    format!("{x:?}").len() as f64
}

/// A name for the size table that lives for the whole run (each is made once).
fn interned(name: String) -> &'static str {
    use std::sync::{Mutex, OnceLock};
    static NAMES: OnceLock<Mutex<std::collections::BTreeMap<String, &'static str>>> = OnceLock::new();
    let mut names = NAMES.get_or_init(Default::default).lock().unwrap();
    if let Some(n) = names.get(&name) {
        return n;
    }
    let leaked: &'static str = Box::leak(name.clone().into_boxed_str());
    names.insert(name, leaked);
    leaked
}

/// Every collection of the sim worth watching, with its size now.
fn sizes(s: &Survival) -> Vec<(&'static str, f64)> {
    let mut v: Vec<(&'static str, f64)> = Vec::new();
    macro_rules! n {
        ($($name:literal => $e:expr),* $(,)?) => {$( v.push(($name, ($e) as f64)); )*};
    }
    n! {
        "players" => s.players.len(), "mobs" => s.mobs.len(), "objs" => s.objs.len(), "storage" => s.storage.len(),
        "missiles" => s.missiles.len(), "line_shots" => s.line_shots.len(), "rail_pieces" => s.rail_pieces.len(),
        "projectiles" => s.projectiles.len(), "grounds" => s.grounds.len(), "bounces" => s.bounces.len(),
        "lockouts" => s.lockouts.len(), "burnings" => s.burnings.len(), "lab_quiet" => s.lab_quiet.len(),
        "orbs" => s.orbs.len(), "bombs" => s.bombs.len(), "staff_marks" => s.staff_marks.len(), "flays" => s.flays.len(),
        "totems" => s.totems.len(), "landings" => s.landings.len(), "lines" => s.lines.len(), "said" => s.said.len(),
        "traps" => s.traps.len(), "duels" => s.duels.len(), "windups" => s.windups.len(), "anchors" => s.anchors.len(),
        "slips" => s.slips.len(), "told_immune" => s.told_immune.len(), "told_vocal" => s.told_vocal.len(),
        "no_error_speech" => s.no_error_speech.len(), "stances" => s.stances.len(), "dummies" => s.dummies.len(),
        "targets" => s.targets.len(), "critical_heals" => s.critical_heals.len(), "cc" => s.cc.len(),
        "cc.entries" => s.cc.values().map(|c| c.len()).sum::<usize>(), "grace" => s.grace.len(), "cc_log" => s.cc_log.len(),
        "meters" => s.meters.len(), "pending" => s.pending.len(), "trails" => s.trails.len(),
        "trails.points" => s.trails.values().map(|t| t.len()).sum::<usize>(), "marks" => s.marks.len(),
        "friendly_marks" => s.friendly_marks.len(), "raid.raiders" => s.raid.raiders.len(), "raid.marks" => s.raid.marks.len(),
        "raid.tower_at" => s.raid.tower_at.len(), "raid.asked" => s.raid.asked.len(), "respawns" => s.respawns.len(),
        "patrols" => s.patrols.len(), "hazard_looks" => s.hazard_looks.len(), "unfound" => s.unfound.len(),
        "trials" => s.trials.len(), "yielded" => s.yielded.len(), "camp.counts" => s.camp.counts.len(),
        "camp.found" => s.camp.found.len(), "camp.opened" => s.camp.opened.len(),
        "hunt.planted" => s.hunt.planted.len(), "hunt.wild" => s.hunt.wild.len(), "hunt.first_arrows" => s.hunt.first_arrows.len(),
        "hunt.wires" => s.hunt.wires.len(), "hunt.pounces" => s.hunt.pounces.len(), "hunt.falls" => s.hunt.falls.len(),
        "hunt.recalls" => s.hunt.recalls.len(), "hunt.anchors" => s.hunt.anchors.len(), "hunt.floating" => s.hunt.floating.len(),
        "hunt.hangs" => s.hunt.hangs.len(),
        "beta.yanks" => s.beta.yanks.len(), "beta.later" => s.beta.later.len(), "beta.echoes" => s.beta.echoes.len(),
        "beta.tethers" => s.beta.tethers.len(), "beta.pops" => s.beta.pops.len(), "beta.launched" => s.beta.launched.len(),
        "beta.catches" => s.beta.catches.len(),
        "warlord.tackles" => s.warlord.tackles.len(), "warlord.banners" => s.warlord.banners.len(), "warlord.fled" => s.warlord.fled.len(),
    }
    // The structs that report their own collections (no `Debug` to take the length of).
    let reports: [(&str, Vec<(&'static str, usize)>); 7] = [
        ("light", s.light.sizes()),
        ("machinery", s.machinery.sizes()),
        ("vancleef", s.vancleef.sizes()),
        ("perches", s.perches.sizes()),
        ("arsenal", s.arsenal.sizes()),
        ("firing_line", s.firing_line.sizes()),
        ("kitchen", s.kitchen.sizes()),
    ];
    for (owner, list) in reports {
        for (name, n) in list {
            v.push((interned(format!("{owner}.{name}")), n as f64));
        }
    }
    n! {
        "dbg.hunt" => debug_len(&s.hunt), "dbg.beta" => debug_len(&s.beta), "dbg.warlord" => debug_len(&s.warlord),
        "dbg.bosses" => debug_len(&s.bosses), "dbg.raid" => debug_len(&s.raid), "dbg.arena" => debug_len(&s.arena),
    }
    if let Some(b) = &s.below {
        n! {
            "below.lit" => b.lit.len(), "below.open" => b.open.len(), "below.slain" => b.slain.len(), "below.looted" => b.looted.len(),
            "below.heroes" => b.heroes.len(), "below.fog" => b.fog.len(), "below.noticed" => b.noticed.len(),
            "below.fallen" => b.fallen.len(), "below.log" => debug_len(&b.log),
        }
    }
    if let Some(g) = &s.siege {
        n! {
            "siege.foes" => g.foes.len(), "siege.owner" => g.owner.len(), "siege.gold" => g.gold.len(), "siege.income" => g.income.len(),
            "siege.target" => g.target.len(), "siege.sent" => g.sent.len(), "siege.bought_once" => g.bought_once.len(),
            "siege.ready" => g.ready.len(), "siege.trickle" => g.trickle.len(),
            "siege.camp.heroes" => g.camps.iter().map(|c| c.heroes.len()).sum::<usize>(),
            "siege.camp.coming" => g.camps.iter().map(|c| c.coming.len()).sum::<usize>(),
        }
    }
    v
}

fn disc(m: &Mob) -> u8 {
    match m.state {
        mobs::MobState::Idle { .. } => 0,
        mobs::MobState::Chase { .. } => 1,
        mobs::MobState::Windup { .. } => 2,
        mobs::MobState::Recover { .. } => 3,
        mobs::MobState::Return => 4,
        mobs::MobState::Dead { .. } => 5,
    }
}

impl Monitor {
    fn new(phase: &str, seed: u64) -> Self {
        eprintln!("SOAK phase {phase}: seed {seed}");
        Self {
            phase: phase.into(),
            sizes: Map::new(),
            tick_ms: Vec::new(),
            minute_ms: Vec::new(),
            stuck: Map::new(),
            stuck_found: Vec::new(),
            dbg: Default::default(),
            reported: Default::default(),
            started: Instant::now(),
            last_found: 0,
            last_raids: 0,
            min_ticks: 0,
            last_hidden: Map::new(),
            odd: Default::default(),
        }
    }

    /// Per tick: nothing broken.
    fn tick(&mut self, s: &Survival, ms: f64) {
        self.tick_ms.push(ms);
        let ph = &self.phase;
        if std::env::var("SOAK_DEATHS").is_ok() {
            for (id, p) in &s.players {
                if p.health == 0 && self.dbg.insert(*id) && self.dbg.len() < 4000 {
                    eprintln!("DEATH hero {id} {} at {:.1}: cause {:?} hit_by {:?} wounds {:?} pos {:?} near {:?}", p.class, s.clock, p.cause, p.last_hit_by.map(|b| s.mobs.get(&b).map(|m| s.content.mobs[m.def].key.clone())), p.wounds.iter().rev().take(3).collect::<Vec<_>>(), p.pos, nearest(s, p.pos, 30.0).map(|(i, _)| s.content.mobs[s.mobs[&i].def].key.clone()));
                }
            }
        }
        for (id, p) in &s.players {
            assert!(p.pos.iter().all(|v| v.is_finite() && v.abs() < 20_000.0), "SOAK {ph}: hero {id} {} position {:?} at {:.1}", p.class, p.pos, s.clock);
            assert!(p.facing.is_finite(), "SOAK {ph}: hero {id} facing at {:.1}", s.clock);
            assert!(p.health <= p.max_health.max(p.health), "SOAK {ph}: hero {id} health {} of {}", p.health, p.max_health);
            assert!(p.stamina.is_finite() && p.valor.is_finite(), "SOAK {ph}: hero {id} stamina {} valor {} at {:.1}", p.stamina, p.valor, s.clock);
        }
        for (id, m) in &s.mobs {
            assert!(m.pos.iter().all(|v| v.is_finite() && v.abs() < 20_000.0), "SOAK {ph}: mob {id} ({}) position {:?} at {:.1}", s.content.mobs[m.def].key, m.pos, s.clock);
            assert!(m.health <= m.max_health.max(m.health), "SOAK {ph}: mob {id} health {} of {}", m.health, m.max_health);
            // A creature with no health is `Dead` (and goes in a few seconds): never up and fighting.
            assert!(m.health > 0 || matches!(m.state, mobs::MobState::Dead { .. }), "SOAK {ph}: zombie {} ({id}) with no health in {:?} at {:.1}", s.content.mobs[m.def].key, m.state, s.clock);
        }
        // A creature unseen is waiting (Idle) or about to rise: it never fights, runs or heals while hidden
        // (but for a boss flung out of its wreck, Sneed, who is out of sight in the air).
        for (id, m) in &s.mobs {
            if m.hidden && m.health > 0 && m.rise_at.is_none() && !s.content.mobs[m.def].boss && !matches!(m.state, mobs::MobState::Idle { .. }) && self.odd.insert(*id) {
                eprintln!("SOAK ODD {ph}: hidden {} ({id}) in {:?}, hp {}/{}, pack {:?}, at {:.1}; a tick before: {:?}", s.content.mobs[m.def].key, m.state, m.health, m.max_health, m.pack.map(|k| s.content.packs[k].key.clone()), s.clock, self.last_hidden.get(id));
            }
        }
        self.last_hidden = s.mobs.iter().filter(|(_, m)| m.hidden || matches!(m.state, mobs::MobState::Return)).map(|(id, m)| (*id, format!("hidden {} {:?} hp {}", m.hidden, m.state, m.health))).collect();
        for (id, m) in &s.mobs {
            if m.hidden && m.health < m.max_health && m.health > 0 && self.dbg.insert(*id) {
                eprintln!("DBG hidden hurt {} ({id}) {:?} hp {}/{} at {:.1} pack {:?}", s.content.mobs[m.def].key, m.state, m.health, m.max_health, s.clock, m.pack.map(|k| s.content.packs[k].key.clone()));
            }
        }
        for (id, o) in &s.objs {
            assert!(o.pos.iter().all(|v| v.is_finite()), "SOAK {ph}: object {id} position {:?} at {:.1}", o.pos, s.clock);
        }
    }

    /// Per simulated minute: sizes, validity, stuck creatures.
    fn minute(&mut self, s: &Survival) {
        self.min_ticks += 1;
        let from = self.tick_ms.len().saturating_sub(TICKS_PER_MIN);
        let slice = &self.tick_ms[from..];
        if !slice.is_empty() {
            let mut sorted = slice.to_vec();
            sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let mean = sorted[sorted.len() / 2];
            let max = slice.iter().cloned().fold(0.0, f64::max);
            self.minute_ms.push((mean, max));
        }
        for (name, n) in sizes(s) {
            self.sizes.entry(name).or_default().push(n);
        }
        // The camp and how far it has come.
        if !s.is_below() && !s.is_siege() {
            let found = s.camp.found.len();
            assert!(found >= self.last_found, "SOAK {}: places found went back from {} to {found}", self.phase, self.last_found);
            assert!(s.camp.raids_won >= self.last_raids, "SOAK {}: raids won went back from {} to {}", self.phase, self.last_raids, s.camp.raids_won);
            self.last_found = found;
            self.last_raids = s.camp.raids_won;
            for (k, n) in &s.camp.counts {
                assert!(*n < 100_000_000, "SOAK {}: camp count {k} = {n}", self.phase);
            }
            for (id, o) in &s.objs {
                if let ObjKind::Station { def, health, .. } = o.kind {
                    assert!(health <= s.content.stations[def].health, "SOAK {}: station {id} health {health}", self.phase);
                }
            }
        }
        // Stuck: the same fighting state for a long time, with nothing else to explain it.
        let now = s.clock;
        let mut found = Vec::new();
        // (An empty world is not stepped at all: its creatures wait as they were, so no state's
        // age counts while nobody is on.)
        if s.players.is_empty() {
            self.stuck.clear();
        }
        let ids: std::collections::BTreeSet<Id> = s.mobs.keys().copied().collect();
        self.stuck.retain(|id, _| ids.contains(id));
        for (id, m) in &s.mobs {
            let d = disc(m);
            let e = self.stuck.entry(*id).or_insert((d, now));
            if e.0 != d || d == 0 {
                *e = (d, now);
                continue;
            }
            let age = now - e.1;
            let limit = match d {
                1..=3 => STUCK_FIGHT,
                4 => STUCK_RETURN,
                _ => STUCK_DEAD,
            };
            if age > limit {
                let cc: Vec<String> = s.cc_of(*id).iter().map(|c| format!("{}:{:.0}", c.kind, c.until)).collect();
                if self.reported.insert(*id) {
                    found.push(format!(
                        "{} ({id}) in state {:?} for {age:.0}s at {:.0?}, home {:.0?}, hp {}/{}, pack {:?}, raider {}, clock {now:.0}, path {}, hidden {}, cc {cc:?}, thrown_until {:.0}, rise {:?}, wander_to {:?}, nearest hero {:?}, its foe {:?}, speed {} think {} ready_at {:.0} lost_at {:?} moved_at {:.0} far_at {:.0} stagger_until {:.0} summoned {:?} friendly {}",
                        s.content.mobs[m.def].key, m.state, m.pos, m.home, m.health, m.max_health, m.pack, m.raider, m.path.len(), m.hidden, m.thrown_until, m.rise_at,
                        m.path.first(), s.players.values().map(|p| dist2d(p.pos, m.pos) as i32).min(),
                        m.state.foe().map(|f| match f {
                            mobs::Foe::Hero(h) => format!("{:?}", s.players.get(&h).map(|p| (p.account.clone(), p.health, p.pos.map(|v| v as i32), dist2d(p.pos, m.pos) as i32))),
                            mobs::Foe::Obj(o) => format!(
                                "{:?}; raid {:?}, {} raiders, {} queued",
                                s.objs.get(&o).map(|ob| (match ob.kind { ObjKind::Station { def, health, damaged, .. } => format!("{} {health}{}", s.content.stations[def].key, if damaged { " wrecked" } else { "" }), _ => "an object".to_string() }, ob.pos.map(|v| v as i32))),
                                s.raid.phase,
                                s.raid.raiders.len(),
                                s.raid.queue.len()
                            ),
                            mobs::Foe::Mob(o) => format!("a friendly creature {o:?}"),
                        }),
                        m.speed, m.think, m.ready_at, m.lost_at, m.moved_at, m.far_at, m.stagger_until, m.summoned_by, m.friendly
                    ));
                }
                *e = (d, now);
            }
        }
        for f in found {
            eprintln!("SOAK STUCK {}: {f}", self.phase);
            self.stuck_found.push(f);
        }
        if self.min_ticks % 10 == 0 {
            eprintln!(
                "SOAK {} minute {}: real {:.0}s, tick median {:.3} ms max {:.2} ms, {} heroes, {} creatures, {} objs, clock {:.0}",
                self.phase,
                self.min_ticks,
                self.started.elapsed().as_secs_f32(),
                self.minute_ms.last().map_or(0.0, |m| m.0),
                self.minute_ms.last().map_or(0.0, |m| m.1),
                s.players.len(),
                s.mobs.len(),
                s.objs.len(),
                s.clock
            );
        }
    }

    /// The end of a phase: what grew, how the tick time went, what was stuck.
    fn report(&self) {
        let mut grew = Vec::new();
        for (name, series) in &self.sizes {
            if series.len() < 6 {
                continue;
            }
            let third = series[series.len() / 3];
            let last = *series.last().unwrap();
            let max = series.iter().cloned().fold(0.0, f64::max);
            // Never shrank (by more than a tenth) in the last half: a collection that only grows.
            // (`told_immune` is a cache that sheds its expired entries past 256: bounded.)
            let half = &series[series.len() / 2..];
            let mono = half.windows(2).all(|w| w[1] >= w[0] * 0.9);
            // (The camp's tallies, chest and finds grow with the journey played: bounded by the content.)
            let bounded = ["told_immune", "camp.counts", "camp.found", "camp.opened", "storage"].contains(name);
            let rising = last > third * 1.5 + 8.0 && mono && last > half[0] && !bounded;
            if rising {
                grew.push((name, series[0], third, last, max));
            }
        }
        for (name, first, third, last, max) in &grew {
            eprintln!("SOAK GROWS {}: {name}: {first:.0} -> {third:.0} (a third in) -> {last:.0} (max {max:.0})", self.phase);
        }
        if grew.is_empty() {
            eprintln!("SOAK {}: no collection only grew", self.phase);
        }
        if std::env::var("SOAK_SIZES").is_ok() {
            for (name, series) in &self.sizes {
                let last = *series.last().unwrap_or(&0.0);
                let most = series.iter().cloned().fold(0.0, f64::max);
                if most > 0.0 {
                    let at = |f: f64| series[((series.len() - 1) as f64 * f) as usize];
                    eprintln!("SOAK SIZE {} {name}: {:.0} / {:.0} / {:.0} / {:.0}, most {most:.0} (start, a third, two thirds, end)", self.phase, series[0], at(0.33), at(0.66), last);
                }
            }
        }
        let n = self.minute_ms.len();
        if n >= 10 {
            let q = (n / 5).max(1);
            let avg = |s: &[(f64, f64)]| s.iter().map(|m| m.0).sum::<f64>() / s.len() as f64;
            let (a, b) = (avg(&self.minute_ms[..q]), avg(&self.minute_ms[n - q..]));
            let mid = avg(&self.minute_ms[n / 2 - q / 2..n / 2 + q / 2 + 1]);
            eprintln!("SOAK CREEP {}: tick median first fifth {a:.3} ms, middle {mid:.3} ms, last fifth {b:.3} ms ({:.2}x)", self.phase, b / a.max(1e-9));
        }
        let mut sorted = self.tick_ms.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        if !sorted.is_empty() {
            let q = |p: f64| sorted[((sorted.len() - 1) as f64 * p) as usize];
            eprintln!(
                "SOAK {} ticks: {} in {:.0}s real; median {:.3} ms, p99 {:.3} ms, max {:.2} ms",
                self.phase,
                sorted.len(),
                self.started.elapsed().as_secs_f32(),
                q(0.5),
                q(0.99),
                sorted[sorted.len() - 1]
            );
        }
        for (name, first, third, last, _) in &grew {
            assert!(!(*last > GROWTH_MIN && *last > third.max(1.0) * GROWTH_FAIL), "SOAK {}: {name} grew without bound: {first:.0} -> {third:.0} -> {last:.0}", self.phase);
        }
        assert!(self.stuck_found.is_empty(), "SOAK {}: {} creatures stuck: {:#?}", self.phase, self.stuck_found.len(), self.stuck_found);
    }
}

// --- the band --------------------------------------------------------------------------------------

struct Hero {
    id: Id,
    account: String,
    class: String,
    /// Away (left the world) until this clock.
    away_until: f32,
    save: Option<crate::save::PlayerSave>,
    /// Kept off the fight until then (after a wipe).
    benched_until: f32,
}

struct Band {
    heroes: Vec<Hero>,
    joins: u32,
}

/// A hero of `class`, its five cards on their keys.
fn dress(s: &mut Survival, class: &str, id: Id) {
    let _ = s.step(0.0);
    s.players.get_mut(&id).unwrap().class = class.to_string();
    let cards: Vec<(String, String)> = s.content.abilities.iter().filter(|a| a.school == class && a.on_board()).map(|a| (a.slot.clone(), a.key.clone())).collect();
    for (slot, key) in cards {
        s.players.get_mut(&id).unwrap().owned.insert(key.clone());
        let _ = s.slot_ability(id, &slot, &key);
    }
}

impl Band {
    fn new(s: &mut Survival, n: usize, at: Vec3, hp: u32) -> Self {
        let mut heroes = Vec::new();
        for k in 0..n {
            let class = SCHOOLS[k % SCHOOLS.len()].to_string();
            let account = format!("soak{k}");
            let id = s.join(&account, &format!("Soak{k}"), None);
            s.move_player(id, [at[0] + k as f32 * 2.0, at[1], at[2]], 0.0, Moving::default());
            dress(s, &class, id);
            let p = s.players.get_mut(&id).unwrap();
            p.max_health = hp;
            p.health = hp;
            s.set_cooldowns(id, true);
            s.set_valor(id, 100.0);
            heroes.push(Hero { id, account, class, away_until: 0.0, save: None, benched_until: 0.0 });
        }
        Self { heroes, joins: n as u32 }
    }

    fn live(&self) -> Vec<Id> {
        self.heroes.iter().filter(|h| h.away_until == 0.0).map(|h| h.id).collect()
    }

    /// A hero leaves, and rejoins a while later as a new id.
    fn churn(&mut self, s: &mut Survival, dice: &mut Dice, hp: u32) {
        let now = s.clock;
        for k in 0..self.heroes.len() {
            let h = &mut self.heroes[k];
            if h.away_until > 0.0 {
                if now >= h.away_until {
                    self.joins += 1;
                    let id = s.join(&h.account, &format!("Soak{k}r{}", self.joins), h.save.as_ref());
                    h.id = id;
                    h.away_until = 0.0;
                    h.save = None;
                    // A different hero each time, so every class is played in turn.
                    h.class = SCHOOLS[dice.below(SCHOOLS.len())].to_string();
                    let class = h.class.clone();
                    dress(s, &class, id);
                    if let Some(p) = s.players.get_mut(&id) {
                        p.max_health = hp;
                        p.health = hp;
                    }
                    s.set_cooldowns(id, true);
                    s.set_valor(id, 100.0);
                }
            } else if dice.chance(0.0004) {
                h.save = s.leave(h.id);
                h.away_until = now + 2.0 + dice.next() * 120.0;
            }
        }
    }
}

fn press(s: &mut Survival, dice: &mut Dice, id: Id) {
    let input = match dice.below(18) {
        0..=7 => Input::Ability(dice.below(SLOTS.len()) as u8, dice.chance(0.4)),
        8 => Input::Attack,
        9 => Input::AttackUp,
        10 => Input::Heavy,
        11 => Input::Guard,
        12 => Input::GuardUp,
        13 => Input::Dodge,
        14 => Input::Bash,
        15 => Input::Kick,
        16 => Input::Use,
        _ => {
            if dice.chance(0.5) {
                Input::Cancel
            } else {
                Input::Lock
            }
        }
    };
    s.input(id, input);
}

/// The creature nearest `from` that is up and not a harmless one.
fn nearest(s: &Survival, from: Vec3, within: f32) -> Option<(Id, Vec3)> {
    s.mobs.iter().filter(|(_, m)| m.health > 0 && !matches!(m.state, mobs::MobState::Dead { .. })).map(|(id, m)| (*id, m.pos, dist2d(from, m.pos))).filter(|c| c.2 < within).min_by(|a, b| a.2.total_cmp(&b.2)).map(|(id, p, _)| (id, p))
}

/// One tick of the heroes' hands: press, aim, walk at the nearest creature.
fn hands(s: &mut Survival, band: &Band, dice: &mut Dice, walk: bool) {
    let now = s.clock;
    let ids: Vec<Id> = band.heroes.iter().filter(|h| h.away_until == 0.0 && now >= h.benched_until).map(|h| h.id).collect();
    for id in ids {
        let Some(p) = s.players.get(&id) else { continue };
        let (pos, facing) = (p.pos, p.facing);
        if dice.chance(0.5) {
            press(s, dice, id);
        }
        let foe = nearest(s, pos, 40.0);
        if dice.chance(0.2) {
            match foe {
                Some((_, at)) if dice.chance(0.8) => {
                    let f = facing_to(pos, at);
                    s.set_aim_line(id, f, dice.next() * 0.4 - 0.2);
                }
                _ => s.set_aim_line(id, dice.next() * 6.3, dice.next() * 1.4 - 0.7),
            }
        }
        if dice.chance(0.05) {
            if let Some((_, at)) = foe {
                s.set_aim_point(id, at);
            }
        }
        if walk && dice.chance(0.5) {
            let (to, f) = match foe {
                Some((_, at)) if dist2d(pos, at) > 3.0 => {
                    let f = facing_to(pos, at);
                    ([pos[0] + f.cos() * 0.4, pos[1] + f.sin() * 0.4, pos[2]], f)
                }
                _ => {
                    let f = facing + dice.next() * 0.6 - 0.3;
                    ([pos[0] + f.cos() * 0.3, pos[1] + f.sin() * 0.3, pos[2]], f)
                }
            };
            s.move_player(id, to, f, Moving { forward: true, ..Default::default() });
        }
    }
}

fn facing_to(a: Vec3, b: Vec3) -> f32 {
    (b[1] - a[1]).atan2(b[0] - a[0])
}

/// Keep heroes mostly alive without making them gods: a hurt hero is sometimes mended, now and
/// then one is struck down.
fn mend(s: &mut Survival, band: &Band, dice: &mut Dice) {
    for h in band.heroes.iter().filter(|h| h.away_until == 0.0) {
        if let Some(p) = s.players.get_mut(&h.id) {
            p.stamina = p.stamina.max(30.0);
            if p.health > 0 && p.health * 3 < p.max_health && dice.chance(0.5) {
                p.health = p.max_health;
            }
        }
        s.set_cooldowns(h.id, true);
        if dice.chance(0.05) {
            s.set_valor(h.id, 100.0);
        }
        if dice.chance(0.004) {
            if let Some(p) = s.players.get_mut(&h.id) {
                p.health = 0;
            }
        }
    }
}

// --- Elwynn ------------------------------------------------------------------------------------------

/// Build whatever the camp can: every station once, the raid's needs first.
fn build_all(s: &mut Survival, band: &Band, dice: &mut Dice, built: &mut u32, spill: &mut Vec<Event>) {
    let Some(h) = band.live().first().copied() else { return };
    if s.players.get(&h).is_none_or(|p| p.health == 0) {
        return;
    }
    s.stock_for_building(40);
    let heart = s.content.world.heart;
    let menu: Vec<String> = s.build_menu(h).into_iter().filter(|m| m.4 == 0).map(|m| m.0).collect();
    for key in menu {
        for _ in 0..6 {
            let a = dice.next() * 6.28;
            let r = 6.0 + dice.next() * 18.0;
            let at = s.ground(heart[0] + a.cos() * r, heart[1] + a.sin() * r);
            s.move_player(h, at, a + 3.14, Moving::default());
            if s.build_start(h, &key).is_err() {
                break;
            }
            s.build_rotate(h, dice.below(8) as i32);
            match s.build_confirm(h) {
                Ok(_) => {
                    *built += 1;
                    break;
                }
                Err(_) => s.cancel_preview(h),
            }
        }
        s.cancel_preview(h);
    }
    // Upgrades, and now and then a demolished one.
    let stations: Vec<Id> = s.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Station { .. })).map(|(id, _)| *id).collect();
    for st in stations {
        if let Some(pos) = s.objs.get(&st).map(|o| o.pos) {
            s.move_player(h, [pos[0] + 1.0, pos[1], pos[2]], 0.0, Moving::default());
            // Wrecked by a raid: mended (standing there a few seconds).
            if matches!(s.objs.get(&st).map(|o| &o.kind), Some(ObjKind::Station { damaged: true, .. })) {
                s.interact(h, Some(st));
                for _ in 0..60 {
                    spill.extend(s.step(DT));
                }
                continue;
            }
            let _ = s.upgrade(h, st);
            if dice.chance(0.03) {
                let _ = s.demolish(h, Some(st), false);
                let _ = s.demolish(h, None, true);
            }
        }
    }
}

fn open_chests(s: &mut Survival, band: &Band, spill: &mut Vec<Event>) {
    let Some(h) = band.live().first().copied() else { return };
    let chests: Vec<(Id, Vec3)> = s.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Chest { spent: false, .. })).map(|(id, o)| (*id, o.pos)).collect();
    for (id, at) in chests.into_iter().take(4) {
        s.move_player(h, [at[0] + 1.0, at[1], at[2]], 0.0, Moving::default());
        s.interact(h, Some(id));
        // A chest takes a few seconds, standing still: step them out here (the rest of the
        // world goes on), then back to the fight.
        for _ in 0..80 {
            spill.extend(s.step(DT));
        }
    }
}

fn rack_round(s: &mut Survival, band: &Band, dice: &mut Dice, armed: &mut u32) {
    let rack = s.rack_id();
    let Some(at) = s.objs.get(&rack).map(|o| o.pos) else { return };
    for h in band.heroes.iter().filter(|h| h.away_until == 0.0) {
        let Some(back) = s.players.get(&h.id).map(|p| p.pos) else { continue };
        let view = s.rack_view(h.id);
        if view.is_empty() {
            return;
        }
        s.move_player(h.id, [at[0] + 1.0, at[1], at[2]], 0.0, Moving::default());
        let (key, ..) = &view[dice.below(view.len())];
        let ok = if dice.chance(0.2) { s.arm_off(h.id, key) } else { s.arm(h.id, key) };
        if ok.is_ok() {
            *armed += 1;
        }
        if dice.chance(0.05) {
            let _ = s.stow_off(h.id);
        }
        s.move_player(h.id, back, 0.0, Moving::default());
    }
}

#[test]
#[ignore]
fn soak_elwynn_wave_after_wave() {
    if only("elwynn") {
        journey::soak_elwynn(minutes() * 0.45, seed());
    }
}

// --- the dungeons ------------------------------------------------------------------------------------

/// A boss's pack: the creatures `boss` marks, with where each was put.
fn bosses(s: &Survival) -> Vec<(Id, Vec3)> {
    s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].boss && m.health > 0).map(|(id, m)| (*id, m.home)).collect()
}

fn soak_dungeon(key: &str, minutes: f32, seed: u64) {
    let mut c = Content::dev();
    c.pvp = Default::default();
    c.dev.full_tree = true;
    c.combat.foe_health = 1.0;
    c.combat.foe_damage = 1.0;
    let c = c.for_dungeon_of(key);
    let Some(d) = c.dungeon.clone() else { return };
    let lamp = d.lamps[0].clone();
    let reset_secs = d.reset_minutes as f32 * 60.0;
    let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), seed, 1_700_000_000.0);
    let mut dice = Dice(seed ^ 0xD0 ^ key.len() as u64);
    let mut band = Band::new(&mut s, 4, lamp.at, DUNGEON_HP);
    for h in band.live() {
        s.arrive_at(h, &lamp.key);
    }
    let mut mon = Monitor::new(&format!("dungeon {key}"), seed);
    let ticks = (minutes * TICKS_PER_MIN as f32) as usize;
    // The pulls: which boss the band is on, when it wiped last, and which wiped bosses are owed
    // a full reset by when.
    let (mut pull, mut wipes, mut resets, mut cleared) = (0usize, 0u32, 0u32, 0u32);
    let mut owed: Vec<(Id, f32)> = Vec::new();
    let mut next_wipe = 200.0 + dice.next() * 400.0;
    let mut homed: Option<f32> = None;
    let mut current: Option<(Id, f32)> = None;
    for tick in 0..ticks {
        let now = s.clock;
        // On the pull: the band is put at the next boss that stands, and fights it.
        // (A pull lasts until its boss is down, or two minutes.)
        let on = current.filter(|(b, at)| now - at < 120.0 && s.mobs.get(b).is_some_and(|m| m.health > 0));
        if tick % 200 == 0 && homed.is_none() && on.is_none() {
            let all = bosses(&s);
            if all.is_empty() {
                // Everything's down: the dungeon is cleared. Home, and wait out its reset.
                cleared += 1;
                for h in &mut band.heroes {
                    if h.away_until == 0.0 {
                        h.save = s.leave(h.id);
                        h.away_until = f32::MAX / 2.0;
                    }
                }
                homed = Some(now + reset_secs + 120.0);
            } else {
                let (boss, home) = all[pull % all.len()];
                pull += 1;
                current = Some((boss, now));
                for (k, h) in band.heroes.iter().enumerate().filter(|(_, h)| h.away_until == 0.0 && now >= h.benched_until) {
                    let to = [home[0] + 5.0 + k as f32, home[1] + 2.0, home[2]];
                    s.move_player(h.id, to, 0.0, Moving::default());
                }
                if let Some(&h) = band.live().first() {
                    s.engage(boss, Foe::Hero(h));
                }
            }
        }
        // Back from home after the dungeon's reset time: a fresh run.
        if let Some(t) = homed {
            if now >= t {
                homed = None;
                resets += 1;
                for h in band.heroes.iter_mut() {
                    h.away_until = now;
                }
                band.churn(&mut s, &mut dice, DUNGEON_HP);
                for h in band.live() {
                    s.arrive_at(h, &lamp.key);
                }
                // Everything that wasn't gone for good, or the whole run, is back.
                let left = bosses(&s).len();
                if left == 0 {
                    eprintln!("SOAK dungeon {key}: after the reset no boss stands ({:.0}s since the clear)", reset_secs + 120.0);
                    mon.stuck_found.push("no reset".into());
                }
            }
        } else {
            hands(&mut s, &band, &mut dice, true);
            if tick % 20 == 0 {
                mend(&mut s, &band, &mut dice);
            }
            band.churn(&mut s, &mut dice, DUNGEON_HP);
            // Blows that tell: the abilities' own damage is tuned off in a dev world, so a
            // hero's hand now and then takes a slice off a creature near them, enough that the
            // bosses fall in a minute or two and their phases, adds and deaths all happen.
            if dice.chance(0.25) {
                let live = band.live();
                if let Some(&by) = live.get(dice.below(live.len().max(1))) {
                    if let Some(p) = s.players.get(&by).filter(|p| p.health > 0 && now >= band.heroes.iter().find(|h| h.id == by).map_or(0.0, |h| h.benched_until)) {
                        let near: Vec<Id> = s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.friendly && !m.hidden && dist2d(m.pos, p.pos) < 12.0).map(|(id, _)| *id).collect();
                        if !near.is_empty() {
                            let id = near[dice.below(near.len())];
                            let max = s.mobs[&id].max_health as f32;
                            s.hurt_mob_as(id, by, (max * (0.004 + dice.next() * 0.02)) as u32 + 1, "Melee");
                        }
                    }
                }
            }
            // A wipe: everyone down at once while a boss fights, and kept off it a while so the
            // boss, left alone, owes a reset.
            if now > next_wipe {
                if let Some((boss, _)) = s.mobs.iter().filter(|(_, m)| s.content.mobs[m.def].boss && m.health > 0 && m.state.foe().is_some()).map(|(id, m)| (*id, m.pos)).next() {
                    wipes += 1;
                    for h in band.heroes.iter_mut().filter(|h| h.away_until == 0.0) {
                        h.benched_until = now + 240.0;
                        if let Some(p) = s.players.get_mut(&h.id) {
                            p.health = 0;
                        }
                    }
                    owed.push((boss, now + 200.0));
                    next_wipe = now + 300.0 + dice.next() * 500.0;
                }
            }
        }
        let t = Instant::now();
        let ev = s.step(DT);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        std::hint::black_box(&ev);
        mon.tick(&s, ms);
        // A wiped boss, left alone: whole and settled by its due time.
        owed.retain(|(boss, due)| {
            if s.clock < *due {
                return true;
            }
            if let Some(m) = s.mobs.get(boss) {
                let near = s.players.values().any(|p| p.health > 0 && dist2d(p.pos, m.pos) < 40.0);
                let settled = matches!(m.state, mobs::MobState::Idle { .. } | mobs::MobState::Return) && m.health == m.max_health;
                if m.health > 0 && !near && !settled {
                    let msg = format!("{} did not reset after a wipe: {:?}, hp {}/{}, at {:.0?} home {:.0?}", s.content.mobs[m.def].key, m.state, m.health, m.max_health, m.pos, m.home);
                    eprintln!("SOAK STUCK dungeon {key}: {msg}");
                    mon.stuck_found.push(msg);
                }
            }
            false
        });
        if tick % TICKS_PER_MIN == TICKS_PER_MIN - 1 {
            mon.minute(&s);
            if std::env::var("SOAK_DEBUG").is_ok() {
                let b = bosses(&s);
                eprintln!("DBG {key}: {} bosses up, {} creatures, foes down {}, heroes {:?}", b.len(), s.mobs.len(), s.below.as_ref().map_or(0, |b| b.log.foes), s.players.values().map(|p| (p.health, p.cause.as_ref().map(|c| c.what.clone()), p.pos.map(|v| v as i32))).collect::<Vec<_>>());
            }
        }
    }
    eprintln!("SOAK dungeon {key} done: {:.0} simulated minutes; {wipes} wipes, {cleared} clears, {resets} resets, {} pulls, {} heroes joined, lit {}, slain {}", s.clock / 60.0, pull, band.joins, s.below.as_ref().map_or(0, |b| b.lit.len()), s.below.as_ref().map_or(0, |b| b.slain.len()));
    mon.report();
}

#[test]
#[ignore]
fn soak_dungeons_bosses_wipes_and_resets() {
    if !only("dungeons") {
        return;
    }
    let c = Content::dev();
    let keys: Vec<String> = c.dungeons().map(|d| d.key.clone()).chain(c.elsewhere.iter().map(|d| d.key.clone())).collect();
    assert!(!keys.is_empty(), "the content has dungeons");
    let each = minutes() * 0.40 / keys.len() as f32;
    eprintln!("SOAK dungeons: {keys:?}, {each:.1} minutes each");
    for key in keys {
        soak_dungeon(&key, each, seed());
    }
}

// --- the siege ------------------------------------------------------------------------------------------

fn soak_siege(minutes: f32, seed: u64) {
    let mut c = Content::dev();
    c.dev.full_tree = false;
    c.combat.foe_health = 1.0;
    c.combat.foe_damage = 1.0;
    let mut s = Survival::new_siege(c, Box::new(FlatLand::default()), seed, 1_700_000_000.0);
    let mut dice = Dice(seed ^ 0x51);
    let mut mon = Monitor::new("siege", seed);
    let modes: Vec<String> = s.siege_def().map(|d| d.modes.iter().map(|m| m.key.clone()).collect()).unwrap_or_default();
    eprintln!("SOAK siege modes: {modes:?}");
    let mut ids: Vec<(Id, String)> = Vec::new();
    let mut joins = 0u32;
    let (mut matches, mut sends, mut cards, mut waves_seen, mut stations) = (0u32, 0u32, 0u32, 0u32, 0u32);
    let mut last_phase_since = (0u8, 0.0f32);
    let ticks = (minutes * TICKS_PER_MIN as f32) as usize;
    for tick in 0..ticks {
        // A new match: heroes arrive and sit in a mode's seats.
        let phase = s.siege.as_ref().unwrap().phase;
        let code = match phase {
            siege::SiegePhase::Lobby => 0,
            siege::SiegePhase::Countdown { .. } => 1,
            siege::SiegePhase::Running => 2,
            siege::SiegePhase::Over { .. } => 3,
        };
        if last_phase_since.0 != code {
            last_phase_since = (code, s.clock);
        } else if code != 0 && s.clock - last_phase_since.1 > 3_600.0 {
            let msg = format!("the match has been in phase {phase:?} for {:.0} s", s.clock - last_phase_since.1);
            eprintln!("SOAK STUCK siege: {msg}");
            mon.stuck_found.push(msg);
            last_phase_since.1 = s.clock;
        }
        if tick % 100 == 0 && matches!(phase, siege::SiegePhase::Lobby) {
            // Heroes gone back to the camp (their world is elsewhere): new ones arrive.
            let mode = &modes[dice.below(modes.len())];
            let seats = s.siege_def().and_then(|d| d.modes.iter().find(|m| &m.key == mode).map(|m| m.bases.len())).unwrap_or(2).max(1);
            for (id, _) in ids.drain(..) {
                let _ = s.leave(id);
            }
            for k in 0..seats.min(4) {
                joins += 1;
                let class = SCHOOLS[(joins as usize + k) % SCHOOLS.len()].to_string();
                let h = s.join(&format!("siege{k}"), &format!("Siege{joins}"), None);
                // (The class's own starting kit, from `siege_arrive`: the rest is bought below.)
                s.players.get_mut(&h).unwrap().class = class.clone();
                s.siege_arrive(h);
                if s.siege_join(h, mode).is_ok() {
                    ids.push((h, class));
                } else {
                    let _ = s.leave(h);
                }
            }
            if matches!(s.siege.as_ref().unwrap().phase, siege::SiegePhase::Countdown { .. }) {
                matches += 1;
            }
        }
        let live: Vec<Id> = ids.iter().map(|(h, _)| *h).filter(|h| s.players.contains_key(h)).collect();
        if matches!(phase, siege::SiegePhase::Running) {
            for &h in &live {
                if s.players.get(&h).is_none_or(|p| p.health == 0) {
                    continue;
                }
                // Card picks: the trainer's cards for gold, a send from the ledger.
                if tick % 40 == 0 {
                    let trainer = s.siege_trainer(h);
                    if let Some((key, ..)) = trainer.iter().find(|c| c.3 && dice.chance(0.5)) {
                        let key = key.clone();
                        if s.take_node(h, &key).is_ok() {
                            cards += 1;
                        }
                    }
                    let ledger = s.siege_ledger(h);
                    let open: Vec<&crate::sim::siege::SendView> = ledger.iter().filter(|l| l.why.is_none()).collect();
                    if !open.is_empty() {
                        let k = open[dice.below(open.len())].key.clone();
                        if s.siege_send(h, &k).is_ok() {
                            sends += 1;
                        }
                    }
                    if dice.chance(0.1) {
                        let _ = s.siege_target(h, None);
                    }
                    if dice.chance(0.3) {
                        let _ = s.siege_ready(h);
                    }
                    if dice.chance(0.2) {
                        if let Some(menu) = s.build_menu(h).into_iter().find(|m| m.3 && dice.chance(0.3)) {
                            if s.build_start(h, &menu.0).is_ok() && s.build_confirm(h).is_ok() {
                                stations += 1;
                            }
                            s.cancel_preview(h);
                        }
                    }
                }
            }
            if tick % 400 == 0 && dice.chance(0.5) {
                if let Some(g) = s.siege.as_mut() {
                    g.timer = g.timer.min(0.0);
                }
            }
        }
        let band = Band { heroes: live.iter().map(|h| Hero { id: *h, account: String::new(), class: String::new(), away_until: 0.0, save: None, benched_until: 0.0 }).collect(), joins: 0 };
        hands(&mut s, &band, &mut dice, true);
        if tick % 20 == 0 {
            mend(&mut s, &band, &mut dice);
        }
        if let Some(g) = s.siege.as_ref() {
            waves_seen = waves_seen.max(g.wave);
        }
        // Everyone who left a finished match for the hall is sent home.
        let t = Instant::now();
        let ev = s.step(DT);
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        std::hint::black_box(&ev);
        mon.tick(&s, ms);
        if tick % TICKS_PER_MIN == TICKS_PER_MIN - 1 {
            mon.minute(&s);
        }
    }
    eprintln!("SOAK siege done: {:.0} simulated minutes; {matches} matches, most waves {waves_seen}, {sends} sends, {cards} cards bought, {stations} stations, {joins} heroes joined", s.clock / 60.0);
    mon.report();
}

#[test]
#[ignore]
fn soak_siege_match_after_match() {
    if only("siege") {
        soak_siege(minutes() * 0.15, seed());
    }
}
