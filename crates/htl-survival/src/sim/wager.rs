//! The Bookmaker's Wager: a dungeon run against the clock at a level (+1, +2, ...), as a mythic
//! keystone's. A wager starts the dungeon fresh (its bosses up, its secrets hidden again, its
//! shortcuts shut) for the heroes who take it from the book; the clock starts when the first of
//! them leaves the first lamp and stops when the last of its bosses falls. Each fall costs time.
//! Beaten in time, the camp may wager higher on that dungeon (two or three levels higher for a
//! quick one); the foes are tougher every level and take on the wager's terms as it climbs
//! (three a week, in turn: tougher packs or bosses, a pack's fall rallying or a frenzy at low
//! health, burning ground where foes fall or eruptions under the heroes). What it pays grows with the
//! level: plunder for everyone there, and the dungeon's gear into the camp's chest, the better
//! pieces from +3 on. Every run is the server's to put on the Bookmaker's Board.
//!
//! The bosses aren't all a run must beat: its foes are counted too (the forces, as a keystone's),
//! and enough of them must fall ([`WAGER_FORCES`] of those that stand from the start). Foes chase
//! farther in a wager ([`WAGER_LEASH`]), so a pack can be drawn into the next and both fought at
//! once: pulling big is the quick way to fill the count.
//!
//! The clock waits for the group: each hero at the first lamp says they're ready (`.es ready`, or
//! the slip's button), and once everyone down there is, a short count runs and the clock starts.
//! A hero who steps off from the lamp before then starts it at once (no head start for free); one
//! arriving mid-count stops it till they're ready too.

use std::collections::{BTreeMap, BTreeSet};

use rand::Rng;

use super::*;
use crate::content::DungeonDef;

/// The highest level a wager goes to.
pub const WAGER_MAX: u32 = 20;
/// What each level adds to the foes (compounding): health, and the weight of their blows.
pub const WAGER_HEALTH: f32 = 0.09;
pub const WAGER_DAMAGE: f32 = 0.06;
/// What a fall costs on the clock (seconds).
pub const WAGER_DEATH: f32 = 10.0;
/// The share of a dungeon's foes (those standing from the start, bosses aside) a run must fell.
/// (A first guess: a run skips a pack or two, not half the dungeon.)
pub const WAGER_FORCES: f32 = 0.8;
/// How much farther foes chase in a wager than in a run of their own (their leash times this),
/// so packs can be pulled together. (A first guess, to try in play: about a room and a half.)
pub const WAGER_LEASH: f32 = 1.75;
/// How far from the first lamp a hero goes to start the clock at once (yards).
const WAGER_START_YARDS: f32 = 12.0;
/// The count from everyone ready to the clock (seconds).
pub const WAGER_COUNTDOWN: f64 = 5.0;
/// How long the ready wait for a hero who never says so, once someone has (seconds): then the
/// count starts anyway, so one hero away from the keys can't hold the rest forever.
pub const WAGER_READY_WAIT: f64 = 90.0;
/// Smouldering ground: how wide, how long, how often it burns and how hard (share of health).
const EMBER_RADIUS: f32 = 2.6;
const EMBER_SECS: f32 = 6.0;
const EMBER_SHARE: f32 = 0.05;
const EMBER_LOOK: u32 = 22191;
/// Eruptions: how wide, the warning before the ground burns, and how often in a fight (seconds).
const ERUPTION_RADIUS: f32 = 3.0;
const ERUPTION_WARN: f32 = 1.5;
const ERUPTION_EVERY: f32 = 18.0;
/// Frenzied: below this share of their health, foes hit this much harder.
pub const FRENZY_BELOW: f32 = 1.0 / 3.0;
pub const FRENZY_MIGHT: f32 = 1.33;
/// The Call: a pack still fighting this long after it began calls the nearest pack at rest
/// this near it (seconds, yards). A first guess, to try in play.
pub const CALL_AFTER: f32 = 5.0;
pub const CALL_RANGE: f32 = 30.0;
/// Quickening: a boss hits this much harder for every step this long its fight has lasted, up
/// to the most (a first guess, to try in play).
pub const QUICKEN_STEP: f32 = 20.0;
pub const QUICKEN_MIGHT: f32 = 0.1;
pub const QUICKEN_MOST: f32 = 2.0;

/// One of a wager's terms.
pub struct Term {
    pub key: &'static str,
    pub name: &'static str,
    pub text: &'static str,
}

/// Every term there is. Which three are on rotates weekly ([`ROTATION`]); a wager takes on the
/// first from [`TERM_LEVELS`]'s first level, the second from its second, the third from its third.
pub const ALL_TERMS: [Term; 6] = [
    Term { key: "calling", name: "The Call", text: "A pack still fighting after five seconds calls the nearest pack at rest: kill quickly, or pull with care." },
    Term { key: "quickening", name: "Quickening", text: "Bosses hit a tenth harder for every twenty seconds the fight lasts: bring them down fast." },
    Term { key: "rallying", name: "Rallying", text: "When a foe falls, the rest of its pack heal a little and hit harder." },
    Term { key: "frenzied", name: "Frenzied", text: "Foes below a third of their health hit a third harder: finish them quickly." },
    Term { key: "smouldering", name: "Smouldering", text: "Fallen foes leave burning ground behind for a few seconds." },
    Term { key: "eruptions", name: "Eruptions", text: "In a fight, the ground under one of you erupts now and then: step off the mark." },
];

/// The wager levels the week's three terms come in at.
pub const TERM_LEVELS: [u32; 3] = [2, 4, 7];

/// The weeks' terms, in turn: (packs or bosses, a pack's fall, the ground). Four weeks round.
pub const ROTATION: [[&str; 3]; 4] = [
    ["calling", "rallying", "smouldering"],
    ["quickening", "frenzied", "eruptions"],
    ["calling", "frenzied", "eruptions"],
    ["quickening", "rallying", "smouldering"],
];

/// The week a moment falls in (unix seconds), weeks starting on Monday.
pub fn week_of(wall: f64) -> u64 {
    // (The first of January 1970 was a Thursday.)
    ((wall / 86_400.0).floor() as i64 + 3).div_euclid(7).max(0) as u64
}

/// The week's three terms, in the order they come in.
pub fn week_terms(wall: f64) -> [&'static Term; 3] {
    let keys = ROTATION[(week_of(wall) % ROTATION.len() as u64) as usize];
    keys.map(|k| ALL_TERMS.iter().find(|t| t.key == k).expect("a term"))
}

/// The terms a wager at `level` takes on this week: (the level it comes in at, the term).
pub fn terms_at(level: u32, wall: f64) -> Vec<(u32, &'static Term)> {
    TERM_LEVELS.iter().zip(week_terms(wall)).filter(|(at, _)| level >= **at).map(|(at, t)| (*at, t)).collect()
}

/// A wager under way below.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Wager {
    pub level: u32,
    /// The time to beat (seconds).
    pub limit: f32,
    /// When the first hero came down (wall seconds), and when the clock started.
    pub opened: f64,
    pub started: Option<f64>,
    pub deaths: u32,
    /// Who took part (account: name, class), in the order they came.
    pub party: Vec<(String, String, String)>,
    /// Its end, once its last boss fell.
    pub done: Option<WagerEnd>,
    /// The doors open before it (shortcuts, secrets found), back once it's over.
    pub kept_open: BTreeSet<String>,
    /// Smouldering ground: where, until when, when it burns next, its mark.
    pub embers: Vec<(Vec3, f32, f32, Id)>,
    /// Foes felled so far (bosses aside), toward the count it needs.
    pub forces: u32,
    /// The bosses are down but the count isn't, and the heroes have been told.
    pub short_told: bool,
    /// Who has said they're ready (accounts), and when the count ends (wall seconds) once all
    /// have; the last whole second it called.
    pub ready: BTreeSet<String>,
    /// When the first hero said they're ready (wall seconds).
    pub first_ready: Option<f64>,
    pub countdown: Option<f64>,
    pub called: i64,
    /// Each boss's split: the clock (with falls) when it fell, by its pack's key.
    pub splits: BTreeMap<String, f32>,
    /// Its terms (keys), as the week was when it was taken: a run keeps them if the week turns.
    pub terms: Vec<String>,
    /// When the next eruption may come (the sim's clock).
    pub next_eruption: f32,
    /// The Call: since when each pack has been fighting (the sim's clock), and the packs that
    /// have called or been called (each only once a run).
    pub fighting: BTreeMap<usize, f32>,
    pub called_packs: BTreeSet<usize>,
    /// Quickening: since when each boss has been fighting (the sim's clock).
    pub boss_since: BTreeMap<Id, f32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WagerEnd {
    /// The time it took with its falls (milliseconds).
    pub ms: u64,
    pub timed: bool,
    /// The levels it raised the camp's wager on this dungeon by.
    pub up: u32,
}

/// A wager's result, for the camp and the board.
#[derive(Debug, Clone, PartialEq)]
pub struct WagerRun {
    pub dungeon: String,
    pub name: String,
    pub level: u32,
    pub ms: u64,
    pub limit_ms: u64,
    pub deaths: u32,
    pub timed: bool,
    /// (name, class) of each hero in it.
    pub party: Vec<(String, String)>,
    /// Each boss as it fell: its name and the clock then (milliseconds, falls counted), in the
    /// order the run has to beat them.
    pub splits: Vec<(String, u64)>,
}

/// The time to beat in a dungeon (seconds): its own, else by its size.
pub fn wager_limit(d: &DungeonDef, content: &crate::content::Content) -> f32 {
    if d.wager_minutes > 0.0 {
        return d.wager_minutes * 60.0;
    }
    let bosses = wager_bosses(d, content).len() as f32;
    let minutes = 8.0 + 0.4 * d.packs.len() as f32 + 1.5 * bosses;
    (minutes / 2.5).round() * 2.5 * 60.0
}

/// The bosses a wager has to beat: every boss that stands from the start (not one a secret keeps,
/// an ambush, or a rare).
pub fn wager_bosses(d: &DungeonDef, content: &crate::content::Content) -> Vec<String> {
    let kept: BTreeSet<&String> = d.doors.iter().flat_map(|dr| dr.reveals.iter()).collect();
    d.packs
        .iter()
        .filter(|p| !p.key.is_empty() && !p.rare && p.hidden <= 0.0 && p.wake.is_empty() && !kept.contains(&p.key))
        .filter(|p| content.mobs.iter().any(|m| m.key == p.mob && m.boss))
        .map(|p| p.key.clone())
        .collect()
}

/// The foes a wager counts for one hero: those standing from the start that aren't bosses (not a
/// secret's, a strongbox's guards, a rare, a totem or a dummy). Others felled count as well;
/// these make the total.
pub fn wager_forces_total(d: &DungeonDef, content: &crate::content::Content) -> u32 {
    wager_forces_total_for(d, content, 1)
}

/// ...and for a band of `heroes`, its packs grown for them ([`super::dungeon::pack_count`]).
pub fn wager_forces_total_for(d: &DungeonDef, content: &crate::content::Content, heroes: usize) -> u32 {
    let kept: BTreeSet<&String> = d.doors.iter().flat_map(|dr| dr.reveals.iter()).collect();
    d.packs
        .iter()
        .filter(|p| !p.rare && p.wake.is_empty() && !kept.contains(&p.key))
        .filter(|p| content.mobs.iter().any(|m| m.key == p.mob && !m.boss && !m.inert() && !m.critter))
        .map(|p| super::dungeon::pack_count(p, d, content, heroes).max(1) as u32)
        .sum()
}

/// How many of them a run must fell, for one hero.
pub fn wager_forces_needed(d: &DungeonDef, content: &crate::content::Content) -> u32 {
    wager_forces_needed_for(d, content, 1)
}

/// ...and for a band of `heroes`.
pub fn wager_forces_needed_for(d: &DungeonDef, content: &crate::content::Content, heroes: usize) -> u32 {
    (wager_forces_total_for(d, content, heroes) as f32 * WAGER_FORCES).ceil() as u32
}

impl Survival {
    /// A foe's leash: how far from home it chases (`ai.leash_scale` times its own; farther
    /// during a wager).
    pub(crate) fn leash_of(&self, def: usize) -> f32 {
        let leash = self.content.mobs[def].leash;
        let scale = self.content.ai.leash_scale.max(0.1);
        if self.wager_running() {
            leash * scale.max(WAGER_LEASH)
        } else {
            leash * scale
        }
    }

    /// A wager's count of foes: (felled, needed, standing from the start), for the heroes below
    /// now, as its packs at rest are grown for them.
    pub fn wager_forces(&self) -> Option<(u32, u32, u32)> {
        let w = self.wager()?;
        let d = self.dungeon_def()?;
        let n = self.wager_band();
        Some((w.forces, wager_forces_needed_for(d, &self.content, n), wager_forces_total_for(d, &self.content, n)))
    }

    /// How many heroes a wager's count is for: those below now (the band the packs are grown for).
    pub(crate) fn wager_band(&self) -> usize {
        self.players.len().max(1)
    }

    /// Whether the wager under way has a term (by key).
    pub fn wager_has(&self, key: &str) -> bool {
        self.wager().is_some_and(|w| w.terms.iter().any(|t| t == key))
    }

    /// The wager under way's terms, as players read them.
    pub fn wager_terms(&self) -> Vec<&'static Term> {
        self.wager().map_or(Vec::new(), |w| w.terms.iter().filter_map(|k| ALL_TERMS.iter().find(|t| t.key == k)).collect())
    }

    /// The wager under way here, if any.
    pub fn wager(&self) -> Option<&Wager> {
        self.below.as_ref()?.wager.as_ref()
    }

    /// The bosses a wager has to beat, as players know them (the last form one becomes, as the
    /// shredder's driver; never a secret form), and whether each is down yet.
    pub fn wager_progress(&self) -> Vec<(String, bool)> {
        let Some(d) = self.dungeon_def() else { return Vec::new() };
        let c = &self.content;
        wager_bosses(d, c)
            .iter()
            .filter_map(|k| {
                let p = d.packs.iter().find(|p| &p.key == k)?;
                let mut m = c.mobs.iter().find(|m| m.key == p.mob)?;
                for _ in 0..4 {
                    if m.then.is_empty() || m.then_after > 0.0 || !m.then_needs.is_empty() {
                        break;
                    }
                    match c.mobs.iter().find(|x| x.key == m.then) {
                        Some(next) => m = next,
                        None => break,
                    }
                }
                Some((m.name.clone(), self.slain(k)))
            })
            .collect()
    }

    /// Each boss's split so far, in the order of [`Self::wager_progress`] (none for one still up).
    pub fn wager_splits(&self) -> Vec<Option<f32>> {
        let (Some(w), Some(d)) = (self.wager(), self.dungeon_def()) else { return Vec::new() };
        wager_bosses(d, &self.content).iter().map(|k| w.splits.get(k).copied()).collect()
    }

    /// A wager running (not yet won or lost).
    pub fn wager_running(&self) -> bool {
        self.wager().is_some_and(|w| w.done.is_none())
    }

    /// The level the camp may wager on a dungeon up to (1 to start with).
    pub fn wager_unlocked(&self, dungeon: &str) -> u32 {
        let n = self.camp.counts.get(&format!("wager:{dungeon}")).copied().unwrap_or(0);
        (1 + n).min(WAGER_MAX)
    }

    /// Below: the dungeon starts fresh for a wager at `level`.
    pub fn start_wager(&mut self, level: u32) {
        let Some(d) = self.dungeon_def().cloned() else { return };
        let limit = wager_limit(&d, &self.content);
        let wall = self.wall();
        let Some(b) = self.below.as_mut() else { return };
        // (A wager left behind gives back what it shut first.)
        if let Some(old) = b.wager.take() {
            b.open = old.kept_open;
        }
        let kept_open = std::mem::take(&mut b.open);
        b.noticed.clear();
        let level = level.clamp(1, WAGER_MAX);
        let terms = terms_at(level, wall).into_iter().map(|(_, t)| t.key.to_string()).collect();
        b.wager = Some(Wager { level, limit, opened: wall, kept_open, terms, ..Default::default() });
        // Every hero's lamp is the first again, with a full hand of Draughts.
        let first = d.lamps.first().map(|l| l.key.clone()).unwrap_or_default();
        for h in b.heroes.values_mut() {
            h.lamp = first.clone();
            h.draughts = d.draughts;
        }
        self.reset_below(false);
        self.scale_wager_foes();
        let names: Vec<&str> = self.wager_terms().iter().map(|t| t.name).collect();
        let terms = if names.is_empty() { String::new() } else { format!(" Its terms: {}.", names.join(", ")) };
        self.announce(format!("The Bookmaker takes a wager on {} at +{level}: {} to beat.{terms}", d.name, clock_words(limit)));
    }

    /// Below: a hero arrives for a wager (their name and class on its slip).
    pub(crate) fn wager_join(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let (account, name, class) = (p.account.clone(), p.name.clone(), p.class.clone());
        let Some(w) = self.below.as_mut().and_then(|b| b.wager.as_mut()) else { return };
        if !w.party.iter().any(|(a, ..)| *a == account) {
            w.party.push((account, name, class));
        }
        // Arriving before the clock: the count (if one's running) waits for them too.
        let waiting = w.started.is_none();
        let stopped = waiting && w.countdown.take().is_some();
        let (level, limit) = (w.level, w.limit);
        if stopped {
            let name = self.players.get(&id).map_or(String::new(), |p| p.name.clone());
            self.announce(format!("{name} arrives: the count waits till everyone's ready."));
        }
        let terms: Vec<String> = self.wager_terms().iter().map(|t| format!("{}: {}", t.name, t.text)).collect();
        let how = if waiting { "say you're ready (.es ready) and the clock starts when everyone is" } else { "the clock is already running" };
        self.events.push(Event::Banner { player: Some(id), title: format!("The Bookmaker's Wager: +{level}"), sub: format!("{} to beat; {how}", clock_words(limit)) });
        if !terms.is_empty() {
            self.tell(id, terms.join(" "));
        }
    }

    /// A hero says they're ready for the clock; once everyone below is, the count starts.
    pub fn wager_ready(&mut self, id: Id) -> Result<(), String> {
        let Some(p) = self.players.get(&id) else { return Err("No hero.".into()) };
        let (account, name) = (p.account.clone(), p.name.clone());
        let wall = self.wall();
        let accounts: BTreeSet<String> = self.players.values().map(|p| p.account.clone()).collect();
        let w = self.below.as_mut().and_then(|b| b.wager.as_mut()).ok_or("There's no wager on down here.")?;
        if w.started.is_some() {
            return Err("The clock is already running.".into());
        }
        w.first_ready.get_or_insert(wall);
        if !w.ready.insert(account) {
            return Err("You're ready; waiting on the rest.".into());
        }
        let ready = w.ready.intersection(&accounts).count();
        let all = ready == accounts.len();
        if all {
            w.countdown = Some(wall + WAGER_COUNTDOWN);
            w.called = WAGER_COUNTDOWN as i64 + 1;
        }
        if all {
            self.announce(format!("{name} is ready. Everyone is: the clock starts in {}.", WAGER_COUNTDOWN as i64));
        } else {
            self.announce(format!("{name} is ready ({ready} of {}).", accounts.len()));
        }
        Ok(())
    }

    /// Before the clock: (heroes ready, heroes below, seconds left of the count if it's running).
    pub fn wager_readiness(&self) -> Option<(usize, usize, Option<f32>)> {
        let w = self.wager().filter(|w| w.started.is_none())?;
        let accounts: BTreeSet<&String> = self.players.values().map(|p| &p.account).collect();
        let ready = w.ready.iter().filter(|a| accounts.contains(a)).count();
        Some((ready, accounts.len(), w.countdown.map(|t| (t - self.wall()).max(0.0) as f32)))
    }

    /// The dungeon's tier on its foes: their health, and the weight of their blows (1 up top, and
    /// in a first dungeon).
    pub(crate) fn tier_scale(&self) -> (f32, f32) {
        let Some(d) = self.dungeon_def().filter(|_| self.is_below()) else { return (1.0, 1.0) };
        let past = d.tier.saturating_sub(1) as f32;
        (1.0 + crate::content::TIER_HEALTH * past, 1.0 + crate::content::TIER_DAMAGE * past)
    }

    /// A foe's health and the weight of its blows at this wager (1 with none).
    pub(crate) fn wager_scale(&self, _def: usize) -> (f32, f32) {
        let Some(w) = self.wager() else { return (1.0, 1.0) };
        let l = w.level.saturating_sub(1) as i32;
        // (Only the level: the week's terms change how a fight is fought, not its numbers.)
        ((1.0 + WAGER_HEALTH).powi(l), (1.0 + WAGER_DAMAGE).powi(l))
    }

    /// Every foe unhurt set to the wager's health.
    fn scale_wager_foes(&mut self) {
        let ids: Vec<Id> = self.mobs.keys().copied().collect();
        for id in ids {
            let m = &self.mobs[&id];
            if m.health == 0 || m.health < m.max_health {
                continue;
            }
            let max = self.foe_health(m.def, m.pack);
            let m = self.mobs.get_mut(&id).unwrap();
            m.max_health = max;
            m.health = max;
            self.events.push(Event::Health { id, health: max, max });
        }
    }

    /// A hero fell during a wager: it costs time.
    pub(crate) fn wager_fall(&mut self) {
        if let Some(w) = self.below.as_mut().and_then(|b| b.wager.as_mut()).filter(|w| w.done.is_none() && w.started.is_some()) {
            w.deaths += 1;
            let n = w.deaths;
            self.announce(format!("A fall: the Bookmaker adds {} to the clock ({n} so far).", clock_words(WAGER_DEATH)));
        }
    }

    /// A foe fell during a wager: counted (if it's not a boss, nor a form coming back as its first),
    /// its pack rallies, its ground smoulders.
    pub(crate) fn wager_kill(&mut self, def: usize, at: Vec3, pack: Option<usize>, form: bool) {
        if !self.wager().is_some_and(|w| w.done.is_none()) {
            return;
        }
        let d = &self.content.mobs[def];
        if !form && !d.boss && !d.inert() && !d.critter {
            if let Some(w) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                w.forces += 1;
            }
        }
        if self.wager_has("rallying") {
            if let Some(k) = pack {
                let mates: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.health > 0 && m.pack == Some(k)).map(|(id, _)| *id).collect();
                for id in mates {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.rally = (m.rally + 1).min(5);
                    m.health = (m.health + m.max_health * 15 / 100).min(m.max_health);
                    let (health, max) = (m.health, m.max_health);
                    self.events.push(Event::Health { id, health, max });
                }
            }
        }
        if self.wager_has("smouldering") && !self.content.mobs[def].boss {
            let until = self.clock + EMBER_SECS;
            let mark = self.mark_ground(at, EMBER_RADIUS, EMBER_SECS, EMBER_LOOK);
            if let Some(w) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                w.embers.push((at, until, self.clock + 1.0, mark));
            }
        }
    }

    /// The Call: a pack fighting [`CALL_AFTER`] seconds calls the nearest pack at rest within
    /// [`CALL_RANGE`] yards onto whoever the caller fights. Bosses neither call nor come.
    fn step_calling(&mut self, now: f32) {
        use super::mobs::MobState;
        let resting = |m: &Mob| matches!(m.state, MobState::Idle { .. });
        // The packs in a fight now, and a foe each is on.
        let mut fights: BTreeMap<usize, super::mobs::Foe> = BTreeMap::new();
        for m in self.mobs.values() {
            let Some(k) = m.pack else { continue };
            if m.health == 0 || m.raider || m.hidden || self.content.mobs[m.def].boss {
                continue;
            }
            // In a fight: chasing, winding up or recovering, on someone.
            if let Some(foe) = m.state.foe() {
                fights.entry(k).or_insert(foe);
            }
        }
        let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) else { return };
        wm.fighting.retain(|k, _| fights.contains_key(k));
        for k in fights.keys() {
            wm.fighting.entry(*k).or_insert(now);
        }
        let due: Vec<(usize, super::mobs::Foe)> = wm
            .fighting
            .iter()
            .filter(|(k, since)| now - **since >= CALL_AFTER && !wm.called_packs.contains(k))
            .map(|(k, _)| (*k, fights[k]))
            .collect();
        for (k, foe) in due {
            let caller: Vec<(Id, Vec3)> = self.mobs.iter().filter(|(_, m)| m.pack == Some(k) && m.health > 0).map(|(id, m)| (*id, m.pos)).collect();
            let called = self.below.as_ref().and_then(|b| b.wager.as_ref()).map(|w| w.called_packs.clone()).unwrap_or_default();
            // The nearest member of a pack at rest, near any of the caller's.
            let near = self
                .mobs
                .iter()
                .filter(|(_, m)| m.health > 0 && !m.raider && !m.hidden && resting(m) && !self.content.mobs[m.def].boss)
                .filter_map(|(id, m)| {
                    let j = m.pack?;
                    if j == k || called.contains(&j) {
                        return None;
                    }
                    let d = caller.iter().map(|(_, at)| dist2d(*at, m.pos)).fold(f32::MAX, f32::min);
                    (d <= CALL_RANGE).then_some((*id, j, d))
                })
                .min_by(|a, b| a.2.total_cmp(&b.2));
            if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                wm.called_packs.insert(k);
            }
            let Some((_, j, _)) = near else { continue };
            if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                wm.called_packs.insert(j);
            }
            let name = caller.first().map_or(String::new(), |(id, _)| self.unit_name(*id));
            // Only the called pack comes: not through its link to whatever else rests nearby (as
            // `engage` would), so no boss and no third pack.
            for m in self.mobs.values_mut().filter(|m| m.pack == Some(j) && m.health > 0 && resting(m)) {
                m.state = MobState::Chase { foe };
                m.think = 0.0;
            }
            self.announce(format!("The {name} calls for help: another pack comes!"));
        }
    }

    /// Quickening: when each boss's fight began (a boss at rest or gone home starts over).
    fn step_quickening(&mut self, now: f32) {
        use super::mobs::MobState;
        let fighting: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0 && self.content.mobs[m.def].boss && !matches!(m.state, MobState::Idle { .. } | MobState::Return))
            .map(|(id, _)| *id)
            .collect();
        let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) else { return };
        wm.boss_since.retain(|id, _| fighting.contains(id));
        for id in fighting {
            wm.boss_since.entry(id).or_insert(now);
        }
    }

    /// Quickening's share for a creature's blow now: more the longer a boss's fight has lasted.
    pub(crate) fn quickening(&self, id: Id) -> f32 {
        let Some(w) = self.wager().filter(|w| w.terms.iter().any(|t| t == "quickening")) else { return 1.0 };
        let Some(since) = w.boss_since.get(&id) else { return 1.0 };
        let steps = ((self.clock - since) / QUICKEN_STEP).floor().max(0.0);
        (1.0 + QUICKEN_MIGHT * steps).min(QUICKEN_MOST)
    }

    /// Each tick below: the clock, the smouldering ground, the end of the wager.
    pub(crate) fn step_wager(&mut self) {
        let Some(w) = self.wager().cloned() else { return };
        let Some(d) = self.dungeon_def().cloned() else { return };
        let wall = self.wall();
        // Nobody left down here: it's over, and the dungeon is as it was before it.
        if self.players.is_empty() {
            self.end_wager();
            return;
        }
        if w.done.is_some() {
            return;
        }
        // The clock starts when the count runs out, once everyone has said they're ready; or at
        // once if a hero steps off from the first lamp first.
        if w.started.is_none() {
            let lamp = d.lamps.first().map_or([0.0; 3], |l| l.at);
            let away = self.players.values().find(|p| p.health > 0 && dist2d(p.pos, lamp) > WAGER_START_YARDS).map(|p| p.name.clone());
            // Someone gone home before the count: are the rest all ready now?
            let accounts: BTreeSet<String> = self.players.values().map(|p| p.account.clone()).collect();
            // (Or someone has been ready long enough: the rest can't be held up for ever.)
            let waited = w.first_ready.is_some_and(|t| wall >= t + WAGER_READY_WAIT);
            if w.countdown.is_none() && !accounts.is_empty() && (accounts.iter().all(|a| w.ready.contains(a)) || waited) {
                if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                    wm.countdown = Some(wall + WAGER_COUNTDOWN);
                    wm.called = WAGER_COUNTDOWN as i64 + 1;
                }
            }
            let w = self.wager().cloned().unwrap();
            let done = w.countdown.is_some_and(|t| wall >= t);
            if away.is_some() || done {
                if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                    wm.started = Some(wall);
                    wm.countdown = None;
                }
                let sub = match (&away, done) {
                    (Some(name), false) => format!("{name} stepped off from the lamp; {} to beat", clock_words(w.limit)),
                    _ => format!("{} to beat", clock_words(w.limit)),
                };
                self.events.push(Event::Banner { player: None, title: format!("+{}: the clock is running", w.level), sub });
            } else if let Some(t) = w.countdown {
                // The count, a banner a second.
                let left = (t - wall).ceil() as i64;
                if left < w.called && left > 0 {
                    if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                        wm.called = left;
                    }
                    self.events.push(Event::Banner { player: None, title: format!("{left}"), sub: format!("+{}: the clock starts", w.level) });
                }
            }
            return;
        }
        // Eruptions: in a fight, now and then, the ground under a hero (marked first).
        let now = self.clock;
        if w.terms.iter().any(|t| t == "eruptions") && now >= w.next_eruption {
            let fighting = self.mobs.values().any(|m| m.health > 0 && !m.raider && !matches!(m.state, super::mobs::MobState::Idle { .. } | super::mobs::MobState::Return));
            let alive: Vec<Vec3> = self.players.values().filter(|p| p.health > 0).map(|p| p.pos).collect();
            if fighting && !alive.is_empty() {
                let at = alive[self.rng.gen_range(0..alive.len())];
                let mark = self.mark_ground(at, ERUPTION_RADIUS, ERUPTION_WARN + EMBER_SECS, EMBER_LOOK);
                if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                    wm.embers.push((at, now + ERUPTION_WARN + EMBER_SECS, now + ERUPTION_WARN, mark));
                    wm.next_eruption = now + ERUPTION_EVERY;
                }
            }
        }
        if w.terms.iter().any(|t| t == "calling") {
            self.step_calling(now);
        }
        if w.terms.iter().any(|t| t == "quickening") {
            self.step_quickening(now);
        }
        // Smouldering ground burns whoever stands in it.
        let mut burns = Vec::new();
        if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
            wm.embers.retain(|e| e.1 > now);
            for e in wm.embers.iter_mut().filter(|e| e.2 <= now) {
                e.2 = now + 1.0;
                burns.push(e.0);
            }
        }
        for at in burns {
            let hit: Vec<(Id, u32)> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= EMBER_RADIUS && (p.pos[2] - at[2]).abs() < 4.0).map(|(id, p)| (*id, ((p.max_health as f32) * EMBER_SHARE).round().max(1.0) as u32)).collect();
            for (id, dmg) in hit {
                self.set_cause(id, super::recap::Cause::world("burning ground"));
                self.hurt_player(id, dmg);
            }
        }
        // Each boss's split, as it falls.
        let bosses = wager_bosses(&d, &self.content);
        let now_secs = self.wager_elapsed();
        let fell: Vec<String> = bosses.iter().filter(|k| self.slain(k) && !w.splits.contains_key(*k)).cloned().collect();
        if !fell.is_empty() {
            if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                for k in fell {
                    wm.splits.insert(k, now_secs);
                }
            }
        }
        let w = self.wager().cloned().unwrap();
        // Its last boss down and enough of its foes: won, in time or not.
        if !bosses.is_empty() && bosses.iter().all(|k| self.slain(k)) {
            let needed = wager_forces_needed_for(&d, &self.content, self.wager_band());
            if w.forces >= needed {
                self.win_wager(&d);
            } else if !w.short_told {
                if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
                    wm.short_told = true;
                }
                let left = needed - w.forces;
                self.events.push(Event::Banner { player: None, title: "The bosses are down".into(), sub: format!("The Bookmaker wants {left} more foe{} felled: the clock runs on", if left == 1 { "" } else { "s" }) });
            }
        }
    }

    /// The wager's time so far, its falls counted (seconds).
    pub fn wager_elapsed(&self) -> f32 {
        let Some(w) = self.wager() else { return 0.0 };
        if let Some(e) = &w.done {
            return e.ms as f32 / 1000.0;
        }
        let run = w.started.map_or(0.0, |s| (self.wall() - s) as f32);
        run + w.deaths as f32 * WAGER_DEATH
    }

    fn win_wager(&mut self, d: &DungeonDef) {
        let Some(w) = self.wager().cloned() else { return };
        let secs = self.wager_elapsed();
        let ms = (secs * 1000.0).round() as u64;
        let timed = secs <= w.limit;
        let steps = if !timed {
            0
        } else if secs <= w.limit * 0.6 {
            3
        } else if secs <= w.limit * 0.8 {
            2
        } else {
            1
        };
        let had = self.wager_unlocked(&d.key);
        let reach = (w.level + steps).min(WAGER_MAX);
        let up = reach.saturating_sub(had);
        if let Some(wm) = self.below.as_mut().and_then(|b| b.wager.as_mut()) {
            wm.done = Some(WagerEnd { ms, timed, up });
        }
        if up > 0 {
            self.forward_count("wager", &d.key, up);
        }
        let party: Vec<(String, String)> = w.party.iter().map(|(_, n, c)| (n.clone(), c.clone())).collect();
        let splits: Vec<(String, u64)> = self.wager_progress().into_iter().zip(self.wager_splits()).map(|((name, _), s)| (name, (s.unwrap_or(secs) * 1000.0).round() as u64)).collect();
        self.events.push(Event::WagerDone(WagerRun {
            dungeon: d.key.clone(),
            name: d.name.clone(),
            level: w.level,
            ms,
            limit_ms: (w.limit * 1000.0) as u64,
            deaths: w.deaths,
            timed,
            party,
            splits,
        }));
        let (title, sub) = if timed {
            (format!("Wager Won: +{}", w.level), format!("{} of {}{}", clock_words(secs), clock_words(w.limit), if up > 0 { format!(": +{reach} is open now") } else { String::new() }))
        } else {
            (format!("+{}: Over Time", w.level), format!("{} of {}: the Bookmaker pays half, and the stakes stay", clock_words(secs), clock_words(w.limit)))
        };
        self.events.push(Event::Banner { player: None, title, sub });
        self.announce(format!(
            "{} at +{}: {} ({} fall{}), {}.",
            d.name,
            w.level,
            clock_words(secs),
            w.deaths,
            if w.deaths == 1 { "" } else { "s" },
            if timed { "in time" } else { "over time" }
        ));
        self.pay_wager(d, w.level, timed);
    }

    /// What a won wager pays: plunder to everyone down here, and gear into the camp's chest.
    fn pay_wager(&mut self, d: &DungeonDef, level: u32, timed: bool) {
        if let Some(plunder) = self.content.item(&d.plunder) {
            // (A harder dungeon pays more a tier.)
            let tier = 1.0 + crate::content::TIER_PLUNDER * d.tier.saturating_sub(1) as f32;
            let n = ((6 + 3 * level) as f32 * tier * if timed { 1.0 } else { 0.5 }).round() as u32;
            let heroes: Vec<Id> = self.players.keys().copied().collect();
            for id in heroes {
                let left = self.give(id, plunder, n);
                self.tell(id, format!("The Bookmaker pays out: {} {}.", n - left, self.content.items[plunder].name));
                if left > 0 {
                    self.events.push(Event::ToCamp { counts: Vec::new(), items: BTreeMap::from([(plunder, left)]) });
                }
            }
        }
        if !timed {
            return;
        }
        // The dungeon's own gear: the plain pieces at first, the better (a perk, a set's) from +3,
        // one of those for certain from +6.
        let mut pool: Vec<usize> = Vec::new();
        for p in &d.packs {
            if let Some(m) = self.content.mob(&p.mob) {
                for g in &self.content.mobs[m].gear {
                    if let Some(i) = self.content.item(g) {
                        if !pool.contains(&i) {
                            pool.push(i);
                        }
                    }
                }
            }
        }
        let fine = |c: &crate::content::Content, i: usize| !c.items[i].perks.is_empty() || !c.items[i].set.is_empty();
        let plain: Vec<usize> = pool.iter().copied().filter(|i| !fine(&self.content, *i)).collect();
        let better: Vec<usize> = pool.iter().copied().filter(|i| fine(&self.content, *i)).collect();
        let rolls = 1 + level / 3;
        let mut won: BTreeMap<usize, u32> = BTreeMap::new();
        // From +4, honed: better the higher the wager.
        let tier = crate::content::hone_tier(level);
        for k in 0..rolls {
            let sure = level >= 6 && k == 0;
            let from = if (sure || (level >= 3 && self.rng.gen::<f32>() < 0.25 + 0.05 * level as f32)) && !better.is_empty() { &better } else { &plain };
            if from.is_empty() {
                continue;
            }
            let i = from[self.rng.gen_range(0..from.len())];
            *won.entry(self.content.honed(i, tier)).or_default() += 1;
        }
        if !won.is_empty() {
            self.events.push(Event::ToCamp { counts: Vec::new(), items: won });
        }
    }

    /// The wager over (won, lost or left): the dungeon as it was before it.
    fn end_wager(&mut self) {
        let Some(b) = self.below.as_mut() else { return };
        let Some(w) = b.wager.take() else { return };
        b.open = w.kept_open;
        for (.., mark) in &w.embers {
            self.unmark(*mark);
        }
        self.reset_below(false);
    }
}

/// A time in words: 12:04.
pub fn clock_words(secs: f32) -> String {
    let s = secs.max(0.0).round() as u64;
    format!("{}:{:02}", s / 60, s % 60)
}
