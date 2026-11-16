//! The Elwynn soak's JOURNEY: the camp's journal played out for real, goal after goal, from the
//! first timber to the last "every corner", by two scouts of the band's own world. Nothing is
//! tallied for them: a goal that asks for a dungeon's clear sends the scouts down by the book, to
//! the dungeon's own world (one for each, as the server keeps them), where they pull every boss
//! and bring the news home the way the server does (`Event::ToCamp`, `from_below_at`); a goal that
//! asks for a lesson has them open the shrine and win its trial.
//!
//! The pilot is DATA-DRIVEN from `content.goals`: it reads each goal's `kind` and `what` and does
//! what a hero would (bank, craft, build, explore, kill, clear, raid, delve, open, learn, wear,
//! secret, cleared). A goal added to the content is played with no change here, as long as it
//! uses those kinds; a kind this file doesn't know is reported (`SOAK JOURNAL UNPLAYED`) and fails
//! the run once it has stood in the way for [`STALL_SECS`].
//!
//! The scouts are strong (a dev world's heroes are not tuned to the bosses) and the damage is
//! dealt by `hurt_mob_as` in slices, as the dungeon phase does: the foes' own code (phases,
//! adds, holds, deaths, trophies, doors) all runs; only the heroes' abilities are tuned off in a
//! dev world, so their hits are stood in for. The scouts also press random buttons at what they
//! fight, as the band does.
//!
//! What the journal doesn't ask for, played too once it is done (and while it is stuck): every
//! person in the camp spoken to and every link they offer followed, every chest opened in order
//! (their guardians first), every place found, every Horde camp taken, every dungeon cleared again.

use super::*;
use crate::content::{DungeonDef, GoalDef};

/// A goal that stays the journal's this long (simulated seconds) with the pilot trying is a failure
/// to find out about: either a bug in the sim, or something the pilot can't play yet.
pub(super) const STALL_SECS: f32 = 3.0 * 3600.0;
/// How long one boss gets before the pilot gives up on that pull (and the goal's stall clock
/// goes on).
const BOSS_SECS: f32 = 480.0;
/// The raids the band rings in a run, the journal's own and after (each comes with more raiders).
const MAX_RAIDS: u32 = 16;
const SCOUTS: usize = 2;
const ACCOUNTS: [&str; SCOUTS] = ["scout0", "scout1"];

/// One dungeon's own world, as the server keeps it, with the monitor of what it holds.
pub(super) struct Dungeon {
    pub key: String,
    pub s: Survival,
    pub mon: Monitor,
    /// Ticks with a hero in it (a minute of them is a monitor minute).
    pub ticks_in: usize,
    pub visits: u32,
}

/// Everything the Elwynn phase runs: the camp, the dungeons beneath it, the chaos band and the
/// bookkeeping.
pub(super) struct Ctx {
    pub s: Survival,
    pub dungeons: Vec<Dungeon>,
    pub band: Band,
    pub dice: Dice,
    pub mon: Monitor,
    pub tick: usize,
    pub ticks: usize,
    pub at: Vec3,
    /// Events from the steps the helpers take inside themselves (a hero crossing must not be lost).
    pub spill: Vec<Event>,
    pub pages: Vec<String>,
    pub built: u32,
    pub armed: u32,
    pub raids_rung: u32,
    pub raids_started: u32,
    pub raid_since: Option<f32>,
    pub crossings: u32,
    pub news: u32,
    /// `SOAK_TRACE=tells`: what the camp says to a hero, as it is said.
    pub trace_tells: bool,
}

/// `SOAK_TRACE=a,b`: what to print as it happens (`tells`, `chest`, or a creature's key).
fn trace(what: &str) -> bool {
    std::env::var("SOAK_TRACE").is_ok_and(|v| v.split(',').any(|k| k == what))
}

/// A scout set down somewhere at once, as a teleport sets a hero (`put_at`, the ground a hair under
/// them): `move_player` is the client's own report of a walk, and holds back a step through a shut
/// door or down a floor ("fell through the world").
fn tele(s: &mut Survival, id: Id, at: Vec3) {
    s.put_at(id, [at[0], at[1], at[2] + ARRIVE_LIFT], 0.0);
    s.land_from_far(id);
}

pub(super) fn dungeon_keys(c: &Content) -> Vec<String> {
    c.dungeons().map(|d| d.key.clone()).chain(c.elsewhere.iter().map(|d| d.key.clone())).collect()
}

impl Ctx {
    pub fn new(seed: u64, minutes: f32) -> Self {
        let mut c = Content::dev();
        c.pvp = Default::default();
        // The tree as a player has it: what a shrine teaches, a hero learns there.
        c.dev.full_tree = false;
        c.combat.foe_health = 1.0;
        c.combat.foe_damage = 1.0;
        let keys = dungeon_keys(&c);
        let mut s = Survival::new(c.clone(), Box::new(FlatLand::default()), seed, 1_700_000_000.0);
        s.free_build = true;
        let mut dungeons = Vec::new();
        for key in keys {
            // (Its content is the camp's: a scout who went down with a full tree would come home with it.)
            let dc = c.for_dungeon_of(&key);
            if dc.dungeon.is_none() {
                continue;
            }
            let d = Survival::new_dungeon(dc, Box::new(FlatLand::default()), seed ^ key.len() as u64, 1_700_000_000.0);
            dungeons.push(Dungeon { mon: Monitor::new(&format!("journey {key}"), seed), key, s: d, ticks_in: 0, visits: 0 });
        }
        let heart = s.content.world.heart;
        let at = s.ground(heart[0], heart[1]);
        let band = Band::new(&mut s, 4, at, 4_000);
        let mut me = Self {
            s,
            dungeons,
            band,
            dice: Dice(seed ^ 0xE1),
            mon: Monitor::new("elwynn", seed),
            tick: 0,
            ticks: (minutes * TICKS_PER_MIN as f32) as usize,
            at,
            spill: Vec::new(),
            pages: Vec::new(),
            built: 0,
            armed: 0,
            raids_rung: 0,
            raids_started: 0,
            raid_since: None,
            crossings: 0,
            news: 0,
            trace_tells: trace("tells"),
        };
        for (k, a) in ACCOUNTS.iter().enumerate() {
            let id = me.s.join(a, &format!("Scout{k}"), None);
            tele(&mut me.s, id, [at[0] + 3.0 + k as f32, at[1] + 3.0, at[2]]);
            me.strong(None, id);
        }
        me
    }

    pub fn live(&self) -> bool {
        self.tick < self.ticks
    }

    /// A scout fit for any boss: health and stamina and valor, cooldowns off.
    fn strong(&mut self, at: Option<usize>, id: Id) {
        let s = self.world(at);
        let Some(p) = s.players.get_mut(&id) else { return };
        p.max_health = DUNGEON_HP;
        p.health = DUNGEON_HP;
        p.stamina = p.stamina.max(60.0);
        s.set_cooldowns(id, true);
        s.set_valor(id, 100.0);
    }

    pub fn world(&mut self, at: Option<usize>) -> &mut Survival {
        match at {
            Some(k) => &mut self.dungeons[k].s,
            None => &mut self.s,
        }
    }

    fn world_ref(&self, at: Option<usize>) -> &Survival {
        match at {
            Some(k) => &self.dungeons[k].s,
            None => &self.s,
        }
    }

    /// The scouts of one world (the camp, or a dungeon by index).
    pub fn scouts_at(&self, at: Option<usize>) -> Vec<Id> {
        let s = self.world_ref(at);
        ACCOUNTS.iter().filter_map(|a| s.players.iter().find(|(_, p)| p.account == *a).map(|(id, _)| *id)).collect()
    }

    /// The scouts: where most of them are (the camp, or a dungeon by index, which wins a tie, as
    /// the party goes down together), and their ids there.
    pub fn scouts(&self) -> (Option<usize>, Vec<Id>) {
        let mut best = (None, self.scouts_at(None));
        for k in 0..self.dungeons.len() {
            let there = self.scouts_at(Some(k));
            if !there.is_empty() && there.len() >= best.1.len() {
                best = (Some(k), there);
            }
        }
        best
    }

    // --- a tick of everything ---------------------------------------------------------------------

    /// One tick of the whole world: the band's hands, the camp's chores, the dungeons beneath,
    /// heroes crossing, and the monitors.
    pub fn tick(&mut self) {
        let tick = self.tick;
        self.tick += 1;
        {
            let Ctx { s, band, dice, spill, at, built, armed, raids_rung, raids_started, raid_since, mon, .. } = self;
            let at = *at;
            hands(s, band, dice, true);
            // Everyone back from a long way out, to find creatures to fight: a hero with nothing
            // near goes to one, or home to the camp.
            if tick % 100 == 0 {
                for h in band.live() {
                    if let Some(p) = s.players.get(&h) {
                        if p.health > 0 && nearest(s, p.pos, 25.0).is_none() {
                            let to = match s.mobs.iter().filter(|(_, m)| m.health > 0).map(|(_, m)| m.pos).nth(dice.below(s.mobs.len().max(1))) {
                                Some(m) if dice.chance(0.7) => [m[0] + 4.0, m[1], m[2]],
                                _ => at,
                            };
                            s.move_player(h, to, 0.0, Moving::default());
                        }
                    }
                }
            }
            if tick % 20 == 0 {
                mend(s, band, dice);
            }
            band.churn(s, dice, 4_000);
            if tick % 3_000 == 1_000 {
                build_all(s, band, dice, built, spill);
            }
            if tick % 2_400 == 1_200 {
                open_chests(s, band, spill);
            }
            if tick % 2_000 == 500 {
                rack_round(s, band, dice, armed);
            }
            if tick % 400 == 0 {
                if let Some(&h) = band.live().first() {
                    if let Some(p) = s.players.get(&h) {
                        let storage = s.objs.get(&s.storage_id()).map(|o| o.pos);
                        if let (Some(st), true) = (storage, p.health > 0 && dice.chance(0.3)) {
                            let back = p.pos;
                            s.move_player(h, [st[0] + 1.0, st[1], st[2]], 0.0, Moving::default());
                            s.deposit_all(h);
                            s.move_player(h, back, 0.0, Moving::default());
                        }
                    }
                }
            }
            // Raids: ring the bell whenever the camp is ready.
            // (A raid more each time comes with a raider more of each kind: past the journal's own the
            // band rings it a dozen times over, not for ever, or the raid is a hundred strong.)
            if tick % 100 == 50 && s.raid.phase == raid::RaidPhase::Idle && s.camp_ready() && *raids_started < MAX_RAIDS {
                if let Some(&h) = band.live().first() {
                    s.ring_bell(h);
                    *raids_rung += 1;
                    if s.raid_start(h).is_ok() {
                        *raids_started += 1;
                    }
                }
            }
            match (s.raid.phase == raid::RaidPhase::Idle, *raid_since) {
                (false, None) => *raid_since = Some(s.clock),
                (true, Some(_)) => *raid_since = None,
                (false, Some(t0)) if s.clock - t0 > 1_800.0 => {
                    eprintln!("SOAK STUCK elwynn: a raid has been {:?} for {:.0} s ({} raiders, {} queued)", s.raid.phase, s.clock - t0, s.raid.raiders.len(), s.raid.queue.len());
                    for r in &s.raid.raiders {
                        if let Some(m) = s.mobs.get(r) {
                            let cc: Vec<String> = s.cc_of(*r).iter().map(|c| format!("{}:{:.0}", c.kind, c.until)).collect();
                            eprintln!("SOAK   raider {} ({r}) {:?} hp {}/{} at {:.0?}, path {} to {:?}, hidden {}, cc {cc:?}", s.content.mobs[m.def].key, m.state, m.health, m.max_health, m.pos, m.path.len(), m.path.last(), m.hidden);
                        } else {
                            eprintln!("SOAK   raider {r} is not a creature any more");
                        }
                    }
                    let stations: Vec<String> = s.objs.values().filter_map(|o| match o.kind { ObjKind::Station { def, health, damaged, .. } => Some(format!("{}{}:{health}@{:.0?}", s.content.stations[def].key, if damaged { "(wrecked)" } else { "" }, o.pos)), _ => None }).collect();
                    eprintln!("SOAK   stations {stations:?}; heroes {:?}", s.players.values().map(|p| (p.health, p.pos.map(|v| v as i32))).collect::<Vec<_>>());
                    mon.stuck_found.push("raid".into());
                    *raid_since = Some(s.clock);
                }
                _ => {}
            }
        }
        self.tend();
        // The camp's step, then the dungeons', then what crosses between them.
        let t = Instant::now();
        let mut events = std::mem::take(&mut self.spill);
        events.extend(self.s.step(DT));
        let ms = t.elapsed().as_secs_f64() * 1000.0;
        std::hint::black_box(&events);
        if self.trace_tells {
            for e in &events {
                if let Event::Tell { player, text } = e {
                    if self.s.players.get(player).is_some_and(|p| ACCOUNTS.contains(&p.account.as_str())) {
                        eprintln!("TELL at {:.1}: {text}", self.s.clock);
                    }
                }
            }
        }
        self.mon.tick(&self.s, ms);
        if tick % TICKS_PER_MIN == TICKS_PER_MIN - 1 {
            self.mon.minute(&self.s);
        }
        // `SOAK_WATCH=<creature id>:<from clock>`: that creature and the hero it is on, each second.
        if tick % 20 == 0 {
            if let Some((id, from)) = std::env::var("SOAK_WATCH").ok().and_then(|v| v.split_once(':').map(|(a, b)| (a.parse::<Id>().ok(), b.parse::<f32>().ok()))).and_then(|(a, b)| Some((a?, b?))) {
                let s = &self.s;
                if s.clock >= from {
                    if let Some(m) = s.mobs.get(&id) {
                        let foe = m.state.target().and_then(|h| s.players.get(&h).map(|p| format!("{} hp {} {:?} untouchable_until {:.0} pos {:.0?}", p.account, p.health, p.action, p.untouchable_until, p.pos)));
                        eprintln!("WATCH {:.1}: {:?} hp {}/{} pos {:.0?} ready_at {:.1} stagger_until {:.1} path {} speed {} think {:.2} role {:?} cc {:?} hero: {foe:?}", s.clock, m.state, m.health, m.max_health, m.pos, m.ready_at, m.stagger_until, m.path.len(), m.speed, m.think, m.role, s.cc_of(id).iter().map(|c| format!("{}:{:.0}", c.kind, c.until)).collect::<Vec<_>>());
                    }
                }
            }
        }
        self.exchange(events);
    }

    /// The scouts let go of every key the random hands held down.
    pub fn release(&mut self) {
        for k in std::iter::once(None).chain((0..self.dungeons.len()).map(Some)) {
            for id in self.scouts_at(k) {
                let s = self.world(k);
                s.input(id, Input::GuardUp);
                s.input(id, Input::AttackUp);
            }
        }
    }

    /// The scouts stay on their feet (the band is the one that dies).
    fn tend(&mut self) {
        for k in std::iter::once(None).chain((0..self.dungeons.len()).map(Some)) {
            let s = self.world(k);
            for p in s.players.values_mut().filter(|p| ACCOUNTS.contains(&p.account.as_str())) {
                if p.health > 0 {
                    p.health = p.max_health;
                    p.stamina = p.stamina.max(30.0);
                }
            }
        }
    }

    /// What the step's events say: heroes down to a dungeon by the book; below, the dungeons'
    /// steps, the news they send up, and heroes coming home.
    fn exchange(&mut self, events: Vec<Event>) {
        for e in events {
            if let Event::Travel { player, to: Some(lamp) } = e {
                self.descend(player, &lamp);
            }
        }
        for k in 0..self.dungeons.len() {
            let t = Instant::now();
            let ev = self.dungeons[k].s.step(DT);
            let ms = t.elapsed().as_secs_f64() * 1000.0;
            std::hint::black_box(&ev);
            let d = &mut self.dungeons[k];
            // (An empty world is not stepped: no creature's state ages while nobody is on.)
            if d.s.players.is_empty() {
                d.mon.stuck.clear();
            } else {
                d.mon.tick(&d.s, ms);
                d.ticks_in += 1;
                if d.ticks_in % TICKS_PER_MIN == TICKS_PER_MIN - 1 {
                    d.mon.minute(&d.s);
                }
            }
            let place = d.s.dungeon_name();
            let mut home = Vec::new();
            for e in ev {
                match e {
                    Event::ToCamp { counts, items } => {
                        self.s.from_below_at(place.as_deref(), &counts, &items);
                        self.news += 1;
                    }
                    Event::Travel { player, to: None } => home.push(player),
                    _ => {}
                }
            }
            for id in home {
                self.ascend(k, id);
            }
            // The journal below is the camp's.
            let (goal, counts) = (self.s.camp.goal, &self.s.camp.counts);
            self.dungeons[k].s.mirror_journal(goal, counts);
        }
        let lit: Vec<String> = self.dungeons.iter().flat_map(|d| d.s.lit_lamps().into_iter().map(|(k, _)| k)).collect();
        if lit != self.pages {
            self.pages = lit.clone();
            self.s.set_pages(lit);
        }
    }

    fn descend(&mut self, player: Id, lamp: &str) {
        let Some(k) = self.dungeons.iter().position(|d| d.s.content.dungeon.as_ref().is_some_and(|dd| dd.lamps.iter().any(|l| l.key == lamp))) else {
            eprintln!("SOAK JOURNAL: no dungeon has the lamp {lamp}");
            return;
        };
        let Some(p) = self.s.players.get(&player) else { return };
        let (account, name) = (p.account.clone(), p.name.clone());
        let Some(save) = self.s.leave(player) else { return };
        let d = &mut self.dungeons[k];
        let id = d.s.join(&account, &name, Some(&save));
        d.s.arrive_at(id, lamp);
        d.s.come_down(id);
        d.visits += 1;
        self.crossings += 1;
        self.strong(Some(k), id);
    }

    fn ascend(&mut self, k: usize, player: Id) {
        let Some(p) = self.dungeons[k].s.players.get(&player) else { return };
        let (account, name) = (p.account.clone(), p.name.clone());
        let Some(save) = self.dungeons[k].s.leave(player) else { return };
        let id = self.s.join(&account, &name, Some(&save));
        if let Some(book) = self.s.objs.get(&self.s.book_id()).map(|o| o.pos) {
            tele(&mut self.s, id, [book[0] + 2.5, book[1], book[2]]);
        }
        self.crossings += 1;
        self.strong(None, id);
    }

    // --- scripted play ----------------------------------------------------------------------------

    pub fn wait(&mut self, secs: f32) {
        for _ in 0..(secs / DT).ceil() as usize {
            if !self.live() {
                return;
            }
            self.tick();
        }
    }

    /// Set the scouts down by a spot (spread a little), in the world they are in.
    fn put_party(&mut self, to: Vec3) {
        let (at, ids) = self.scouts();
        for (k, id) in ids.into_iter().enumerate() {
            tele(self.world(at), id, [to[0] + 1.0 + k as f32 * 1.5, to[1], to[2]]);
        }
    }

    /// The scouts at a world, as a band of hands.
    fn party(&self, at: Option<usize>, ids: &[Id]) -> Band {
        let _ = at;
        Band { heroes: ids.iter().map(|id| Hero { id: *id, account: String::new(), class: String::new(), away_until: 0.0, save: None, benched_until: 0.0 }).collect(), joins: 0 }
    }

    /// The party takes on creature `mid` of the world `at`: set down by it, it is pulled, and
    /// the scouts' blows (a slice of its health a few times a second, a few buttons pressed at it)
    /// go on until it falls, or `secs` run out. True if it is down (or gone).
    fn fight(&mut self, at: Option<usize>, mid: Id, secs: f32) -> bool {
        let t0 = self.world_ref(at).clock;
        let mut n = 0usize;
        loop {
            if !self.live() {
                return false;
            }
            let ids = self.scouts_at(at);
            if ids.is_empty() {
                return false;
            }
            let party = self.party(at, &ids);
            let Ctx { s, dungeons, dice, .. } = self;
            let w = match at {
                Some(k) => &mut dungeons[k].s,
                None => s,
            };
            let Some(m) = w.mobs.get(&mid) else {
                self.release();
                return true;
            };
            if m.health == 0 || matches!(m.state, mobs::MobState::Dead { .. }) {
                self.release();
                return true;
            }
            if w.clock - t0 > secs {
                self.release();
                return false;
            }
            let (pos, max, hidden) = (m.pos, m.max_health, m.hidden);
            if n % 20 == 0 && trace(&w.content.mobs[m.def].key) {
                eprintln!("TRACE {} at {:.1}: {:?} hp {}/{} pos {:.0?} home {:.0?} hero {:.0?} cc {:?}", w.content.mobs[m.def].key, w.clock, m.state, m.health, m.max_health, m.pos, m.home, w.players.get(&ids[0]).map(|p| p.pos), w.cc_of(mid).iter().map(|c| c.kind.clone()).collect::<Vec<_>>());
            }
            let alive: Vec<Id> = ids.iter().copied().filter(|id| w.players.get(id).is_some_and(|p| p.health > 0)).collect();
            if !alive.is_empty() {
                // Set down by it when far (a boss that walked off, a creature that fled).
                if n % 40 == 0 {
                    for (k, id) in alive.iter().enumerate() {
                        if w.players.get(id).is_some_and(|p| dist2d(p.pos, pos) > 14.0) {
                            let to = w.ground_near(pos[0] + 5.0 + k as f32, pos[1] + 2.0, pos[2]);
                            tele(w, *id, to);
                        }
                    }
                }
                if !hidden && n % 10 == 0 && w.mobs.get(&mid).is_some_and(|m| m.state.foe().is_none()) {
                    w.engage(mid, Foe::Hero(alive[0]));
                }
                if !hidden && n % 5 == 0 {
                    let by = alive[dice.below(alive.len())];
                    let share = 0.004 + dice.next() * 0.016;
                    let slice = (max as f32 * share) as u32 + 1;
                    w.hurt_mob_as(mid, by, slice, "Melee");
                    // A raid is many: the raiders about the one fought take the sweep too (a boss's
                    // adds are left to its own fight).
                    if w.mobs.get(&mid).is_some_and(|m| m.raider) {
                        let near: Vec<(Id, u32)> = w.mobs.iter().filter(|(id, m)| **id != mid && m.raider && m.health > 0 && !m.hidden && dist2d(m.pos, pos) < 8.0).take(16).map(|(id, m)| (*id, m.max_health)).collect();
                        for (id, other) in near {
                            w.hurt_mob_as(id, by, (other as f32 * share) as u32 + 1, "Melee");
                        }
                    }
                }
                hands(w, &party, dice, false);
            }
            n += 1;
            self.tick();
        }
    }
}

// --- what a mob is, where it is -----------------------------------------------------------------------

/// The creatures of a world with this content key, up and not yet dead: (id, hidden).
fn alive_of(s: &Survival, key: &str) -> Vec<(Id, bool)> {
    let Some(def) = s.content.mob(key) else { return Vec::new() };
    s.mobs.iter().filter(|(_, m)| m.def == def && m.health > 0 && !matches!(m.state, mobs::MobState::Dead { .. })).map(|(id, m)| (*id, m.hidden)).collect()
}

fn kills(s: &Survival, key: &str) -> u32 {
    s.camp.counts.get(&format!("kill:{key}")).copied().unwrap_or(0)
}

/// What the pilot wants of a dungeon.
#[derive(Clone, Debug)]
enum Target {
    /// In and out by the book (lighting the first lamp on the way).
    Visit,
    /// A lamp lit (the lamp's key).
    Lamp(String),
    /// A boss (or any creature) of this mob key felled.
    Kill(String),
    /// Every boss of a run down: the dungeon cleared.
    Clear,
    /// Out of the front door: the exit boss down, the way walked.
    End,
}

/// The goal pilot, and what it has seen.
#[derive(Default)]
pub(super) struct Pilot {
    /// The journal's goal and the clock it became current.
    last: (usize, f32),
    began: bool,
    pub stalled: Vec<String>,
    pub unplayed: Vec<String>,
    /// Goal title, when it was done (simulated minutes).
    pub done: Vec<(String, f32)>,
    told: std::collections::BTreeSet<String>,
    extras: usize,
    /// The clock the camp last had no raid on it (a raid defended for long is left to the band).
    raid_since: f32,
    pub trips: u32,
    pub boss_kills: u32,
    pub talked: u32,
    pub links: u32,
    pub chests: u32,
    pub trials: u32,
    pub clears: u32,
    pub explored: u32,
    cleared_dungeons: BTreeSet<String>,
    depth: u32,
}

impl Pilot {
    fn note(&mut self, key: &str, msg: String) {
        if self.told.insert(key.to_string()) {
            eprintln!("SOAK JOURNAL: {msg}");
        }
    }

    /// One step of the pilot: a unit of work towards the journal's goal (or, with the journal
    /// done or stuck, an extra), and always at least one tick.
    pub fn step(&mut self, c: &mut Ctx) {
        let before = c.tick;
        let now = c.s.clock;
        let goal = c.s.camp.goal;
        if !self.began {
            self.began = true;
            self.last = (goal, now);
        }
        if goal != self.last.0 {
            let title = c.s.content.goals.get(self.last.0).map_or(String::new(), |g| g.title.clone());
            self.done.push((title.clone(), now / 60.0));
            eprintln!("SOAK JOURNAL: goal {} of {} done at {:.0} min: {title}", self.last.0 + 1, c.s.content.goals.len(), now / 60.0);
            self.last = (goal, now);
            self.talk_round(c);
        }
        let (here, ids) = c.scouts();
        if ids.is_empty() {
            c.wait(5.0);
            return;
        }
        // A raid on the camp: the scouts meet it where it comes, at the heart, whatever else the
        // journal asks (the raiders follow the heroes wherever they go, so the heroes go home).
        if here.is_none() && c.s.raid.active() && now - self.raid_since < 1500.0 {
            self.defend(c);
            return;
        }
        if !c.s.raid.active() {
            self.raid_since = now;
        }
        // Scouts are home to play the camp's goals (a dungeon's are played from the book).
        if here.is_some() {
            self.home(c);
        } else if let Some(g) = c.s.goal().cloned() {
            let stalled_for = now - self.last.1;
            if stalled_for > STALL_SECS {
                let msg = format!("goal {} of {} ({} {} x{}: {}) has been the journal's for {:.0} simulated minutes", goal + 1, c.s.content.goals.len(), g.kind, g.what, g.count, g.title, stalled_for / 60.0);
                eprintln!("SOAK STUCK journal: {msg}");
                self.stalled.push(msg);
                self.last.1 = now;
            }
            self.play(c, &g);
            // A long stand: the rest of the world gets played too (it may be what unlocks it).
            if stalled_for > 1800.0 {
                self.extra(c);
            }
        } else {
            self.extra(c);
        }
        // (The scouts' random buttons are let go: a hero left guarding or charging for ever is no
        // hero, and what fights them waits on it.)
        c.release();
        if c.tick == before {
            c.tick();
        }
    }

    /// The scouts at the camp's heart, and the raider nearest it fought.
    fn defend(&mut self, c: &mut Ctx) {
        let heart = c.at;
        for id in c.scouts_at(None) {
            if c.s.players.get(&id).is_some_and(|p| dist2d(p.pos, heart) > 40.0) {
                tele(&mut c.s, id, [heart[0] + 4.0, heart[1], heart[2]]);
            }
        }
        let s = &c.s;
        let next = s.raid.raiders.iter().filter_map(|r| s.mobs.get(r).filter(|m| m.health > 0).map(|m| (*r, dist2d(m.pos, heart)))).min_by(|a, b| a.1.total_cmp(&b.1));
        match next {
            // (Fought where it stands if it is near, else waited for: it is on its way.)
            Some((mid, d)) if d < 60.0 => {
                self.pull(c, None, mid);
            }
            _ => c.wait(2.0),
        }
    }

    // --- goals ------------------------------------------------------------------------------------

    fn play(&mut self, c: &mut Ctx, g: &GoalDef) {
        let (have, need) = c.s.goal_progress(g);
        let missing = need.saturating_sub(have);
        let key = format!("{}:{}", g.kind, g.what);
        match g.kind.as_str() {
            "bank" => self.bank(c, &g.what, missing),
            "craft" => self.craft(c, &g.what),
            "build" => {
                if !self.build(c, &g.what) {
                    c.wait(10.0);
                }
            }
            "gather" => self.gather(c, &g.what),
            "wear" => self.wear(c, &g.what),
            // A choice a person offers (once it is theirs to offer): they are spoken to, and the choice made.
            "chose" => {
                self.talk_round(c);
                c.wait(5.0);
            }
            "kill" => self.kill(c, &g.what),
            "clear" => self.clear_camp(c, &g.what),
            "raid" => self.raid(c, &g.what),
            "explore" => self.explore(c, &g.what),
            "secret" => self.secrets(c),
            "open" => self.open_goal(c, &g.what),
            "learn" => self.learn(c),
            "delve" => self.delve(c, &g.what),
            "cleared" => {
                let d = c.s.content.ladder().find(|d| d.key == g.what).cloned();
                match d {
                    Some(d) => {
                        self.dungeon(c, &d, Target::Clear);
                    }
                    None => self.note(&key, format!("no dungeon {} for the goal {}", g.what, g.title)),
                }
            }
            kind => {
                self.note(&key, format!("UNPLAYED goal kind {kind} ({}: {})", g.what, g.title));
                self.unplayed.push(format!("{kind} {} ({})", g.what, g.title));
                c.wait(30.0);
            }
        }
    }

    /// Scouts in a dungeon with the camp's goal in front of them: home by a lamp.
    fn home(&mut self, c: &mut Ctx) {
        let (here, _) = c.scouts();
        let Some(k) = here else { return };
        self.go_home(c, k);
    }

    fn bank(&mut self, c: &mut Ctx, what: &str, n: u32) {
        let (_, ids) = c.scouts();
        let h = ids[0];
        let Some(st) = c.s.objs.get(&c.s.storage_id()).map(|o| o.pos) else { return };
        tele(&mut c.s, h, [st[0] + 1.0, st[1], st[2]]);
        if let Err(e) = c.s.give_for_testing(h, what, n.max(1)) {
            self.note(&format!("give {what}"), format!("{what} not given: {e}"));
        }
        c.s.deposit_all(h);
        c.wait(2.0);
    }

    fn craft(&mut self, c: &mut Ctx, what: &str) {
        let (_, ids) = c.scouts();
        let h = ids[0];
        let Some(r) = c.s.content.recipes.iter().find(|r| r.key == what).cloned() else {
            self.note(&format!("recipe {what}"), format!("no recipe {what}"));
            c.wait(30.0);
            return;
        };
        for (k, n) in &r.inputs {
            if let Some(i) = c.s.content.item(k) {
                *c.s.storage.entry(i).or_default() += n * 4;
            }
        }
        if !c.s.has_station(&r.station) {
            self.build(c, &r.station);
        }
        let s = &c.s;
        let at = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Station { def, damaged: false, .. } if s.station_does(def, &r.station))).map(|(id, o)| (*id, o.pos));
        match at {
            Some((st, pos)) => {
                tele(&mut c.s, h, [pos[0] + 1.0, pos[1], pos[2]]);
                if let Err(e) = c.s.craft(h, st, &r.key) {
                    self.note(&format!("craft {}", r.key), format!("craft {} failed: {e}", r.key));
                }
                c.wait(5.0);
            }
            None => {
                self.note(&format!("station {}", r.station), format!("no station for {} ({})", r.key, r.station));
                c.wait(30.0);
            }
        }
    }

    /// Build a station the camp has none of (by the build menu, in a ring about the heart).
    fn build(&mut self, c: &mut Ctx, key: &str) -> bool {
        if c.s.has_station(key) {
            return true;
        }
        let (_, ids) = c.scouts();
        let h = ids[0];
        c.s.stock_for_building(40);
        let heart = c.s.content.world.heart;
        let mut built = false;
        for _ in 0..12 {
            let a = c.dice.next() * 6.28;
            let r = 6.0 + c.dice.next() * 18.0;
            let at = c.s.ground(heart[0] + a.cos() * r, heart[1] + a.sin() * r);
            tele(&mut c.s, h, at);
            if let Err(e) = c.s.build_start(h, key) {
                self.note(&format!("build {key}"), format!("cannot start building {key}: {e}"));
                break;
            }
            c.s.build_rotate(h, c.dice.below(8) as i32);
            match c.s.build_confirm(h) {
                Ok(_) => {
                    c.built += 1;
                    built = true;
                    break;
                }
                Err(_) => c.s.cancel_preview(h),
            }
        }
        c.s.cancel_preview(h);
        c.wait(3.0);
        built || c.s.has_station(key)
    }

    fn gather(&mut self, c: &mut Ctx, what: &str) {
        let (_, ids) = c.scouts();
        let h = ids[0];
        let s = &c.s;
        let node = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Node { def, ready: true, .. } if s.content.nodes[def].item == what)).map(|(id, o)| (*id, o.pos));
        match node {
            Some((id, pos)) => {
                tele(&mut c.s, h, [pos[0] + 1.0, pos[1], pos[2]]);
                c.s.interact(h, Some(id));
                c.wait(8.0);
                let st = c.s.objs.get(&c.s.storage_id()).map(|o| o.pos);
                if let Some(st) = st {
                    tele(&mut c.s, h, [st[0] + 1.0, st[1], st[2]]);
                    c.s.deposit_all(h);
                }
            }
            None => c.wait(30.0),
        }
    }

    fn wear(&mut self, c: &mut Ctx, slot: &str) {
        let (_, ids) = c.scouts();
        let h = ids[0];
        let Some(item) = (0..c.s.content.items.len()).find(|i| c.s.content.items[*i].kind == crate::content::ItemKind::Armor && c.s.content.items[*i].slot == slot) else {
            self.note(&format!("wear {slot}"), format!("no armor for the slot {slot}"));
            c.wait(30.0);
            return;
        };
        let key = c.s.content.items[item].key.clone();
        let _ = c.s.give_for_testing(h, &key, 1);
        if let Err(e) = c.s.wear_from_bag(h, item) {
            self.note(&format!("wear {slot}"), format!("wearing {key} failed: {e}"));
        }
        c.wait(2.0);
    }

    // --- Elwynn: creatures, camps, chests, places --------------------------------------------------

    /// A creature (by content key) felled: in Elwynn where it stands, woken if it is hidden, or in
    /// the dungeon whose pack it is.
    fn kill(&mut self, c: &mut Ctx, key: &str) {
        let alive = alive_of(&c.s, key);
        if let Some(&(mid, hidden)) = alive.iter().find(|(_, h)| !h).or(alive.first()) {
            if hidden {
                // (A second form waiting to rise needs only its time.)
                if c.s.mobs.get(&mid).is_some_and(|m| m.rise_at.is_some()) {
                    c.wait(2.0);
                    return;
                }
                self.wake(c, mid);
            }
            self.pull(c, None, mid);
            return;
        }
        // A creature that is the second form of another (`then`): the first form is what stands.
        if let Some(first) = c.s.content.mobs.iter().find(|m| m.then == key).map(|m| m.key.clone()) {
            self.kill(c, &first);
            return;
        }
        // Stands in Elwynn's packs, waiting to come back.
        if c.s.content.packs.iter().any(|p| p.mob == key) {
            // Chests with an ambush of it, shut until their guardian falls, may be what wakes it.
            self.wake_by_chest(c, key);
            c.wait(20.0);
            return;
        }
        let home = c.s.content.ladder().find(|d| d.packs.iter().any(|p| p.mob == key)).cloned();
        match home {
            Some(d) => {
                self.dungeon(c, &d, Target::Kill(key.to_string()));
            }
            None => {
                self.note(&format!("kill {key}"), format!("UNPLAYED: no pack of {key} in Elwynn or any dungeon"));
                self.unplayed.push(format!("kill {key}"));
                c.wait(30.0);
            }
        }
    }

    /// A creature in the camp's world pulled and felled (the party first set down by it).
    fn pull(&mut self, c: &mut Ctx, at: Option<usize>, mid: Id) {
        let Some(m) = c.world_ref(at).mobs.get(&mid) else { return };
        let pos = m.pos;
        let to = [pos[0] + 6.0, pos[1] + 2.0, pos[2]];
        let to = c.world(at).ground_near(to[0], to[1], to[2]);
        c.put_party(to);
        if c.fight(at, mid, BOSS_SECS) {
            self.boss_kills += 1;
        }
    }

    /// Wake a hidden creature (a pack set to wait): open the chest that wakes it, or walk to it.
    fn wake(&mut self, c: &mut Ctx, mid: Id) {
        let Some(m) = c.s.mobs.get(&mid) else { return };
        let (pos, pack) = (m.pos, m.pack);
        let Some(k) = pack else { return };
        let p = c.s.content.packs[k].clone();
        if !p.wake.is_empty() {
            self.open_chest(c, &p.wake);
        } else if let Some(ch) = c.s.content.chests.iter().find(|ch| !p.key.is_empty() && ch.ambush.contains(&p.key)).map(|ch| ch.key.clone()) {
            self.open_chest(c, &ch);
        } else {
            let to = c.s.ground(pos[0] + p.hidden.max(2.0) * 0.5, pos[1]);
            c.put_party(to);
            c.wait(2.0);
        }
    }

    fn wake_by_chest(&mut self, c: &mut Ctx, key: &str) {
        let packs: Vec<String> = c.s.content.packs.iter().filter(|p| p.mob == key && !p.key.is_empty()).map(|p| p.key.clone()).collect();
        let ch = c.s.content.chests.iter().find(|ch| ch.ambush.iter().any(|a| packs.contains(a))).map(|ch| ch.key.clone());
        if let Some(ch) = ch {
            self.open_chest(c, &ch);
        }
    }

    /// Every fighter of a Horde camp felled, the camp taken (held for its hold).
    fn clear_camp(&mut self, c: &mut Ctx, camp: &str) {
        let packs: Vec<usize> = c.s.content.packs.iter().enumerate().filter(|(_, p)| p.camp == camp).map(|(k, _)| k).collect();
        if packs.is_empty() {
            self.note(&format!("camp {camp}"), format!("UNPLAYED: no pack holds a camp {camp}"));
            self.unplayed.push(format!("clear {camp}"));
            c.wait(30.0);
            return;
        }
        for _ in 0..40 {
            if !c.live() {
                return;
            }
            let s = &c.s;
            let up: Vec<(Id, f32)> = s.mobs.iter().filter(|(_, m)| m.health > 0 && m.pack.is_some_and(|p| packs.contains(&p))).map(|(id, m)| (*id, dist2d(m.pos, c.at))).collect();
            let Some(&(mid, _)) = up.iter().min_by(|a, b| a.1.total_cmp(&b.1)) else {
                // None stand: the camp is taken (a hold), or the rest are on their way back.
                c.wait(5.0);
                return;
            };
            if c.s.mobs.get(&mid).is_some_and(|m| m.hidden) {
                self.wake(c, mid);
            }
            self.pull(c, None, mid);
        }
    }

    /// A chest opened, by key (its guardian felled first if it needs one, a secret one found,
    /// a trial won). Quietly false when it is there no more.
    fn open_chest(&mut self, c: &mut Ctx, key: &str) -> bool {
        if self.depth > 4 {
            return false;
        }
        self.depth += 1;
        let r = self.open_chest_in(c, key);
        self.depth -= 1;
        r
    }

    fn open_chest_in(&mut self, c: &mut Ctx, key: &str) -> bool {
        let Some(def) = c.s.content.chests.iter().position(|ch| ch.key == key) else { return false };
        let ch = c.s.content.chests[def].clone();
        if !ch.after.is_empty() && !c.s.chest_unlocked(&ch.after) {
            self.fulfil(c, &ch.after);
            return false;
        }
        // A secret one: come within its reach.
        if c.s.unfound.contains(&def) {
            let to = c.s.ground(ch.at[0], ch.at[1]);
            c.put_party(to);
            c.wait(2.0);
        }
        let s = &c.s;
        let obj = s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, spent: false } if d == def)).map(|(id, o)| (*id, o.pos));
        let Some((obj, pos)) = obj else { return false };
        let (_, ids) = c.scouts();
        let h = ids[0];
        c.put_party([pos[0] + 1.0, pos[1], pos[2]]);
        tele(&mut c.s, h, [pos[0] + 1.0, pos[1], pos[2]]);
        let acted = c.s.players.get(&h).map(|p| format!("{:?}", p.action));
        c.s.interact(h, Some(obj));
        let started = c.s.players.get(&h).map(|p| format!("{:?}", p.action));
        c.wait(ch.secs + 1.5);
        self.chests += 1;
        if trace("chest") {
            eprintln!("TRACE chest {key}: before {acted:?}, after use {started:?}, now {:?}, hero at {:.0?} chest at {:.0?}, trial {}, opened {}", c.s.players.get(&h).map(|p| format!("{:?}", p.action)), c.s.players.get(&h).map(|p| p.pos), pos, c.s.trials.contains_key(&def), c.s.camp.opened.contains_key(key));
        }
        // A trial: its guardians on the party, and the trial passed when the last falls. A hold
        // (the wagon) is that and its time, kept near the chest while its waves come to the party
        // (a wave is one pack or several joined with `+`).
        if c.s.trials.contains_key(&def) {
            self.trials += 1;
            let packs: Vec<usize> = ch.trial.iter().flat_map(|key| key.split('+')).filter_map(|key| c.s.content.packs.iter().position(|p| p.key == key.trim())).collect();
            let t0 = c.s.clock;
            while c.live() && c.s.trials.contains_key(&def) && c.s.clock - t0 < ch.hold + 600.0 {
                // The nearest of them to the chest, fought where it stands if it is near, else its coming waited for.
                let s = &c.s;
                let up = s.mobs.iter().filter(|(_, m)| m.health > 0 && !m.hidden && m.pack.is_some_and(|p| packs.contains(&p))).map(|(id, m)| (*id, dist2d(m.pos, pos))).min_by(|a, b| a.1.total_cmp(&b.1));
                match up {
                    Some((mid, d)) if d < 50.0 || ch.hold <= 0.0 => self.pull(c, None, mid),
                    _ => c.wait(1.0),
                }
            }
        }
        true
    }

    /// Do what a condition asks (a chest's `after`, a person's): a creature felled, a chest opened,
    /// a place found, a station built, a choice made.
    fn fulfil(&mut self, c: &mut Ctx, cond: &str) {
        let (what, arg) = cond.split_once(':').unwrap_or(("kill", cond));
        let (what, arg) = (what.trim(), arg.trim());
        match what {
            "kill" => self.kill(c, arg),
            "open" => {
                self.open_chest(c, arg);
            }
            "found" => self.explore(c, arg),
            "built" => {
                self.build(c, arg);
            }
            "chose" => self.talk_round(c),
            // (The raids and goals come by themselves, in their order.)
            _ => c.wait(20.0),
        }
    }

    fn open_goal(&mut self, c: &mut Ctx, what: &str) {
        if what == "any" {
            // The next chest not yet opened, easiest first.
            let key = c.s.content.chests.iter().filter(|ch| !c.s.camp.opened.contains_key(&ch.key)).map(|ch| ch.key.clone()).next();
            if let Some(k) = key {
                self.open_chest(c, &k);
            } else {
                c.wait(30.0);
            }
        } else if c.s.content.chests.iter().any(|ch| ch.key == what) {
            self.open_chest(c, what);
        } else {
            self.note(&format!("open {what}"), format!("UNPLAYED: no chest {what}"));
            self.unplayed.push(format!("open {what}"));
            c.wait(30.0);
        }
    }

    /// Cards learned: a shrine or relic's trial not yet won by a scout, else a champion not yet felled.
    fn learn(&mut self, c: &mut Ctx) {
        let (_, ids) = c.scouts();
        let owned = |c: &Ctx, keys: &[String]| c.s.players.get(&ids[0]).is_some_and(|p| keys.iter().all(|k| p.owned.contains(k)));
        let chest = c.s.content.chests.iter().filter(|ch| !ch.teaches.is_empty() && !owned(c, &ch.teaches)).map(|ch| (ch.secret > 0.0, ch.key.clone())).min();
        if let Some((_, key)) = chest {
            self.open_chest(c, &key);
            return;
        }
        let champ = c.s.content.mobs.iter().filter(|m| !m.teaches.is_empty() && !owned(c, &m.teaches)).map(|m| m.key.clone()).next();
        match champ {
            Some(k) => self.kill(c, &k),
            None => c.wait(30.0),
        }
    }

    fn explore(&mut self, c: &mut Ctx, what: &str) {
        let next = c.s.content.sites.iter().find(|s| !c.s.camp.found.contains(&s.key) && (what == "any" || s.kind == what || s.key == what)).map(|s| s.key.clone());
        let (_, ids) = c.scouts();
        match next {
            Some(k) => {
                for id in &ids {
                    let _ = c.s.goto_site(*id, &k);
                }
                c.wait(1.5);
                self.explored += 1;
            }
            None => c.wait(30.0),
        }
    }

    fn secrets(&mut self, c: &mut Ctx) {
        let next = c.s.unfound.iter().map(|d| c.s.content.chests[*d].key.clone()).next();
        match next {
            Some(k) => {
                self.open_chest(c, &k);
            }
            None => c.wait(30.0),
        }
    }

    // --- raids ------------------------------------------------------------------------------------

    /// Raid `tier`: the camp made ready, the bell rung, and the raiders met as they come.
    fn raid(&mut self, c: &mut Ctx, tier: &str) {
        let _ = tier;
        let (_, ids) = c.scouts();
        let h = ids[0];
        match c.s.raid.phase {
            raid::RaidPhase::Idle => {
                let plan = c.s.next_raid();
                for k in &plan.needs {
                    if !c.s.has_station(k) {
                        self.build(c, k);
                    }
                }
                if c.s.camp_ready() {
                    let bell = c.s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Alarm)).map(|(_, o)| o.pos);
                    if let Some(b) = bell {
                        tele(&mut c.s, h, [b[0] + 1.0, b[1], b[2]]);
                    }
                    c.s.ring_bell(h);
                    c.raids_rung += 1;
                    if c.s.raid_start(h).is_ok() {
                        c.raids_started += 1;
                    }
                    c.wait(2.0);
                } else {
                    c.wait(10.0);
                }
            }
            _ => {
                // Met where they come in: the one nearest the heart first.
                let s = &c.s;
                let next = s.raid.raiders.iter().filter_map(|r| s.mobs.get(r).filter(|m| m.health > 0).map(|m| (*r, dist2d(m.pos, c.at)))).min_by(|a, b| a.1.total_cmp(&b.1));
                match next {
                    Some((mid, _)) => self.pull(c, None, mid),
                    None => c.wait(3.0),
                }
            }
        }
    }

    // --- dungeons ---------------------------------------------------------------------------------

    fn delve(&mut self, c: &mut Ctx, what: &str) {
        let ladder: Vec<DungeonDef> = c.s.content.ladder().cloned().collect();
        let open = |c: &Ctx, d: &DungeonDef| c.s.dungeon_open(d);
        if what == "book" {
            if let Some(d) = ladder.iter().filter(|d| open(c, d)).min_by_key(|d| d.tier).cloned() {
                self.dungeon(c, &d, Target::Visit);
            } else {
                c.wait(20.0);
            }
        } else if let Some(d) = ladder.iter().find(|d| d.lamps.iter().any(|l| l.key == what)).cloned() {
            self.dungeon(c, &d, Target::Lamp(what.to_string()));
        } else if let Some(key) = what.strip_prefix("end:") {
            match ladder.iter().find(|d| d.key == key).cloned() {
                Some(d) => {
                    self.dungeon(c, &d, Target::End);
                }
                None => self.note(&format!("delve {what}"), format!("UNPLAYED: no dungeon {key}")),
            }
        } else if what == "end" {
            if let Some(d) = ladder.iter().filter(|d| open(c, d)).min_by_key(|d| d.tier).cloned() {
                self.dungeon(c, &d, Target::End);
            }
        } else {
            self.note(&format!("delve {what}"), format!("UNPLAYED: delve {what}"));
            self.unplayed.push(format!("delve {what}"));
            c.wait(30.0);
        }
    }

    /// The prerequisites of a dungeon the book reads: the raids it wants (waited for) and the
    /// dungeons of the tier below it cleared. True when its pages can be read.
    fn ensure_open(&mut self, c: &mut Ctx, d: &DungeonDef) -> bool {
        for _ in 0..8 {
            if c.s.dungeon_open(d) {
                return true;
            }
            if c.s.camp.raids_won < d.gate_raids {
                return false;
            }
            let next = c.s.content.ladder().filter(|o| o.tier + 1 == d.tier && !c.s.ever("cleared", &o.key) && o.key != d.key).cloned().next();
            match next {
                Some(o) if self.depth < 3 => {
                    self.depth += 1;
                    self.dungeon(c, &o, Target::Clear);
                    self.depth -= 1;
                }
                _ => return false,
            }
        }
        false
    }

    /// The scouts go down by the book to `d`, do `target`, and come home by a lamp. True if it was done.
    fn dungeon(&mut self, c: &mut Ctx, d: &DungeonDef, target: Target) -> bool {
        if !c.s.book_open() || !self.ensure_open(c, d) {
            c.wait(20.0);
            return false;
        }
        let Some(k) = c.dungeons.iter().position(|x| x.key == d.key) else { return false };
        let lamp = d.lamps[0].key.clone();
        // Down: each scout at the book, reading the page.
        let (_, ids) = c.scouts();
        let Some(book) = c.s.objs.get(&c.s.book_id()).map(|o| o.pos) else { return false };
        for id in &ids {
            tele(&mut c.s, *id, [book[0] + 1.0, book[1], book[2]]);
            if let Err(e) = c.s.travel(*id, &lamp) {
                self.note(&format!("travel {lamp}"), format!("the book would not take the scouts to {lamp}: {e}"));
                c.wait(10.0);
                return false;
            }
        }
        c.wait(0.2);
        if c.scouts().0 != Some(k) {
            self.note(&format!("arrive {lamp}"), format!("the scouts did not arrive in {} ({:?})", d.key, c.scouts().0));
            return false;
        }
        self.trips += 1;
        let ok = match &target {
            Target::Visit => true,
            Target::Lamp(key) => self.light(c, k, key),
            Target::Kill(mob) => self.kill_below(c, k, mob),
            Target::Clear => self.clear_below(c, k),
            Target::End => self.walk_out(c, k),
        };
        if c.scouts().0 == Some(k) {
            self.go_home(c, k);
        }
        ok
    }

    /// A lamp lit below: the scouts at it, using it.
    fn light(&mut self, c: &mut Ctx, k: usize, key: &str) -> bool {
        let Some(l) = c.dungeons[k].s.dungeon_def().and_then(|d| d.lamps.iter().find(|l| l.key == key).cloned()) else { return false };
        let (_, ids) = c.scouts();
        for id in &ids {
            let w = &mut c.dungeons[k].s;
            tele(w, *id, [l.at[0] + 1.0, l.at[1], l.at[2]]);
        }
        let lamp = c.dungeons[k].s.objs.iter().find(|(_, o)| matches!(&o.kind, ObjKind::Lamp { key: lk } if lk == key)).map(|(id, _)| *id);
        if let Some(lamp) = lamp {
            c.dungeons[k].s.interact(ids[0], Some(lamp));
        }
        c.wait(1.0);
        c.dungeons[k].s.lit_lamps().iter().any(|(lk, _)| lk == key)
    }

    /// Every creature of a mob key felled below.
    fn kill_below(&mut self, c: &mut Ctx, k: usize, mob: &str) -> bool {
        for _ in 0..12 {
            if !c.live() {
                return false;
            }
            let alive = alive_of(&c.dungeons[k].s, mob);
            let Some(&(mid, hidden)) = alive.iter().find(|(_, h)| !h).or(alive.first()) else { return true };
            if hidden {
                // (A boss that steps out when its place is entered.) Walk to where it waits.
                if let Some(m) = c.dungeons[k].s.mobs.get(&mid) {
                    let to = [m.pos[0] + 3.0, m.pos[1], m.pos[2]];
                    c.put_party(to);
                    c.wait(3.0);
                }
            }
            self.pull(c, Some(k), mid);
        }
        alive_of(&c.dungeons[k].s, mob).is_empty()
    }

    /// Every boss of the run felled, until the dungeon says it is cleared.
    fn clear_below(&mut self, c: &mut Ctx, k: usize) -> bool {
        let Some(d) = c.dungeons[k].s.dungeon_def().cloned() else { return false };
        let bosses = crate::sim::wager::wager_bosses(&d, &c.dungeons[k].s.content);
        let t0 = c.dungeons[k].s.clock;
        for _ in 0..200 {
            if !c.live() {
                return false;
            }
            let w = &c.dungeons[k].s;
            if bosses.iter().all(|b| w.slain(b)) {
                self.clears += 1;
                self.cleared_dungeons.insert(d.key.clone());
                c.wait(2.0);
                return true;
            }
            if w.clock - t0 > 3600.0 {
                break;
            }
            // The boss whose pack comes first in the dungeon's order, still standing.
            let next = bosses.iter().find(|b| !w.slain(b)).and_then(|b| d.packs.iter().position(|p| &p.key == b));
            let target = next.and_then(|pk| w.mobs.iter().filter(|(_, m)| m.pack == Some(pk) && m.health > 0).map(|(id, m)| (*id, m.hidden)).next());
            match target {
                Some((mid, hidden)) => {
                    if hidden {
                        if let Some(m) = c.dungeons[k].s.mobs.get(&mid) {
                            c.put_party([m.pos[0] + 3.0, m.pos[1], m.pos[2]]);
                            c.wait(3.0);
                        }
                    }
                    self.pull(c, Some(k), mid);
                }
                // Not there yet (a boss that comes when another falls): the other bosses first.
                None => {
                    let any = w.mobs.iter().filter(|(_, m)| w.content.mobs[m.def].boss && m.health > 0).map(|(id, m)| (*id, m.hidden)).next();
                    match any {
                        Some((mid, _)) => self.pull(c, Some(k), mid),
                        None => c.wait(5.0),
                    }
                }
            }
        }
        let w = &c.dungeons[k].s;
        let left: Vec<&String> = bosses.iter().filter(|b| !w.slain(b)).collect();
        self.note(&format!("clear {}", d.key), format!("{} not cleared: {left:?} still standing (a minute of scripted play can't reach them?)", d.key));
        false
    }

    /// Out the front door: the exit boss down (a clear does that), the scouts walked to the exit.
    fn walk_out(&mut self, c: &mut Ctx, k: usize) -> bool {
        let Some(d) = c.dungeons[k].s.dungeon_def().cloned() else { return false };
        if !d.exit_boss.is_empty() && !c.dungeons[k].s.slain(&d.exit_boss) {
            let mob = d.packs.iter().find(|p| p.key == d.exit_boss).map(|p| p.mob.clone());
            if let Some(m) = mob {
                self.kill_below(c, k, &m);
            }
        }
        c.put_party(d.exit);
        c.wait(6.0);
        true
    }

    /// Home from a dungeon: at its first lamp, no foe on them, by the lamp's page ("home").
    fn go_home(&mut self, c: &mut Ctx, k: usize) {
        let Some(d) = c.dungeons[k].s.dungeon_def().cloned() else { return };
        let lamp = d.lamps[0].clone();
        for _ in 0..40 {
            if !c.live() {
                return;
            }
            let ids = c.scouts_at(Some(k));
            if ids.is_empty() {
                return;
            }
            for id in &ids {
                c.dungeons[k].s.arrive_at(*id, &lamp.key);
            }
            // Whatever is on them is felled first (the lamp refuses while foes are on).
            let chasing: Vec<Id> = {
                let w = &c.dungeons[k].s;
                w.mobs.iter().filter(|(_, m)| m.health > 0 && m.state.target().is_some_and(|t| ids.contains(&t))).map(|(id, _)| *id).collect()
            };
            if let Some(&mid) = chasing.first() {
                self.pull(c, Some(k), mid);
                continue;
            }
            // Its flame lit (the first use of a lamp lights it; a lamp unlit takes nobody home).
            if !c.dungeons[k].s.lit_lamps().iter().any(|(lk, _)| *lk == lamp.key) {
                let obj = c.dungeons[k].s.objs.iter().find(|(_, o)| matches!(&o.kind, ObjKind::Lamp { key } if *key == lamp.key)).map(|(id, _)| *id);
                c.dungeons[k].s.interact(ids[0], obj);
                c.wait(0.2);
                continue;
            }
            let mut all_home = true;
            for id in &ids {
                if let Err(e) = c.dungeons[k].s.travel(*id, "camp") {
                    self.note(&format!("home {} {e}", d.key), format!("the lamp at {} would not take scout {id} home: {e} (it is {:?} of {:?}; {} heroes in the dungeon)", d.key, c.dungeons[k].s.players.get(id).map(|p| (p.account.clone(), p.health)), ids, c.dungeons[k].s.players.len()));
                    all_home = false;
                }
            }
            c.wait(0.3);
            if all_home || c.scouts_at(Some(k)).is_empty() {
                return;
            }
            c.wait(3.0);
        }
        eprintln!("SOAK JOURNAL: the scouts could not leave {} by its lamp", d.key);
        // (Out some other way would still be the server's: the soak does not mask it.)
        self.stalled.push(format!("the scouts could not go home from {}", d.key));
        for id in c.scouts_at(Some(k)) {
            let _ = c.dungeons[k].s.leave(id);
        }
        // (A fresh pair of scouts back at camp, as a hero who logged in again would be.)
        for (n, a) in ACCOUNTS.iter().enumerate() {
            if c.scouts_at(None).iter().any(|id| c.s.players[id].account == *a) {
                continue;
            }
            let id = c.s.join(a, &format!("Scout{n}"), None);
            c.strong(None, id);
        }
    }

    // --- the rest of the world ----------------------------------------------------------------------

    /// Everyone in the camp spoken to who has something to say, and each link followed.
    fn talk_round(&mut self, c: &mut Ctx) {
        let (here, ids) = c.scouts();
        if here.is_some() || ids.is_empty() {
            return;
        }
        let h = ids[0];
        let people: Vec<usize> = (0..c.s.content.props.len()).filter(|k| c.s.content.props[*k].npc).collect();
        for def in people {
            let Some(t) = c.s.talk_line(def).cloned() else { continue };
            let Some(at) = c.s.objs.iter().find(|(_, o)| matches!(o.kind, ObjKind::Piece { def: d } if d == def)).map(|(id, o)| (*id, o.pos)) else { continue };
            tele(&mut c.s, h, [at.1[0] + 1.5, at.1[1], at.1[2]]);
            let before = c.s.events.len();
            c.s.interact(h, Some(at.0));
            self.talked += 1;
            let _ = before;
            // (In an order of the dice's: a choice is made once, and every option gets its turn over the seeds.)
            let mut links: Vec<&String> = t.links.iter().collect();
            for k in (1..links.len()).rev() {
                links.swap(k, c.dice.below(k + 1));
            }
            for l in links {
                let Some((cmd, _)) = l.split_once('~') else { continue };
                self.follow(c, h, cmd.trim());
            }
        }
    }

    /// A link a person offers (a chat command the client would send).
    fn follow(&mut self, c: &mut Ctx, h: Id, cmd: &str) {
        let words: Vec<&str> = cmd.split_whitespace().collect();
        let r = match words.as_slice() {
            [".es", "lore" | "secrets", key] => c.s.lore_page(h, key),
            [".es", "note", key] => c.s.reread(h, key),
            [".es", "choose", spec] => c.s.choose(h, spec),
            _ => {
                self.note(&format!("link {cmd}"), format!("UNPLAYED link: {cmd}"));
                self.unplayed.push(format!("link {cmd}"));
                return;
            }
        };
        self.links += 1;
        if let Err(e) = r {
            // (A note not read yet is no fault: the page says so.)
            self.note(&format!("link {cmd} {e}"), format!("link {cmd}: {e}"));
        }
    }

    /// With the journal done, or stuck: one more round of what a thorough hero would play.
    fn extra(&mut self, c: &mut Ctx) {
        let n = self.extras;
        self.extras += 1;
        match n % 6 {
            0 => self.talk_round(c),
            1 => {
                // Every chest in the order of the land's list, their guardians first.
                let keys: Vec<String> = c.s.content.chests.iter().map(|ch| ch.key.clone()).collect();
                for k in keys {
                    if !c.live() {
                        return;
                    }
                    if !c.s.camp.opened.contains_key(&k) {
                        self.open_chest(c, &k);
                    }
                }
            }
            2 => {
                let sites: Vec<String> = c.s.content.sites.iter().map(|s| s.key.clone()).collect();
                let (_, ids) = c.scouts();
                for k in sites {
                    for id in &ids {
                        let _ = c.s.goto_site(*id, &k);
                    }
                    c.wait(1.0);
                }
                self.explored += 1;
            }
            3 => {
                let camps: Vec<String> = c.s.content.camps.iter().map(|k| k.key.clone()).collect();
                for k in camps {
                    self.clear_camp(c, &k);
                }
            }
            4 => {
                // A dungeon cleared, the easiest first, one a round.
                let mut ladder: Vec<DungeonDef> = c.s.content.ladder().cloned().collect();
                ladder.sort_by_key(|d| d.tier);
                let pick = ladder.iter().filter(|d| c.s.dungeon_open(d)).nth((n / 6) % ladder.len().max(1)).cloned();
                if let Some(d) = pick {
                    self.dungeon(c, &d, Target::Clear);
                }
            }
            _ => c.wait(60.0),
        }
    }
}

// --- the phase ---------------------------------------------------------------------------------------

pub(super) fn soak_elwynn(minutes: f32, seed: u64) {
    let mut c = Ctx::new(seed, minutes);
    let mut pilot = Pilot::default();
    while c.live() {
        pilot.step(&mut c);
    }
    let goals = c.s.content.goals.len();
    eprintln!(
        "SOAK elwynn done: {:.0} simulated minutes; journal goal {} of {}, {} built, {} armed, {} raids rung, {} started, {} won, {} chests opened ({} by scouts, {} trials), {} heroes joined, {} found",
        c.s.clock / 60.0,
        c.s.camp.goal,
        goals,
        c.built,
        c.armed,
        c.raids_rung,
        c.raids_started,
        c.s.camp.raids_won,
        c.s.camp.opened.len(),
        pilot.chests,
        pilot.trials,
        c.band.joins,
        c.s.camp.found.len()
    );
    let camps_taken: u32 = c.s.camp.counts.iter().filter(|(k, _)| k.starts_with("clear:")).map(|(_, n)| *n).sum();
    eprintln!(
        "SOAK journey: {} dungeon trips, {} boss pulls, {} clears, {} camps taken, {} places visited, {} people spoken to, {} links followed, {} crossings, {} news items sent up",
        pilot.trips, pilot.boss_kills, pilot.clears, camps_taken, pilot.explored, pilot.talked, pilot.links, c.crossings, c.news
    );
    if c.s.camp.goal < goals {
        let g = &c.s.content.goals[c.s.camp.goal];
        eprintln!("SOAK JOURNAL: the journal stands at goal {} of {}: {} ({} {})", c.s.camp.goal + 1, goals, g.title, g.kind, g.what);
    } else {
        eprintln!("SOAK JOURNAL: the whole journal played, all {goals} goals");
    }
    for (title, min) in &pilot.done {
        eprintln!("SOAK JOURNAL   {min:6.0} min  {title}");
    }
    for d in c.dungeons.iter_mut() {
        if d.mon.min_ticks >= 1 {
            eprintln!("SOAK journey {}: {} visits, {} minutes with heroes in", d.key, d.visits, d.mon.min_ticks);
            d.mon.report();
        }
    }
    for u in &pilot.unplayed {
        eprintln!("SOAK JOURNAL UNPLAYED {u}");
    }
    c.mon.report();
    assert!(pilot.stalled.is_empty(), "SOAK elwynn: the journal stalled: {:#?}", pilot.stalled);
}
