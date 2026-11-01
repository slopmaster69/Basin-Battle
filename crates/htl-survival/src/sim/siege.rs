//! Siege: a world of its own (Arathi Basin) reached by a page of the book at camp. Heroes wait in
//! the hall and join one of the ways to play at its banner (one against one, two against two,
//! three or four each for themselves); once a way's seats are full (or the heroes waiting call
//! it on) the match begins. Each camp is a Hearth with its building ground round it, a weapon
//! rack and the mercenary ledger; its heroes start with their class's Q and E and the plain kit.
//!
//! Every `wave_secs` a wave comes in at every camp still standing, through its gate (out along the
//! way to the middle): the base wave everyone faces, and what its rivals bought for it at their
//! ledgers. Each send raises its buyer's income, paid at every wave's start; felling a foe pays
//! gold, as does the middle's veins, and felling a rival takes a share of theirs. Gold builds
//! the camp's defences and buys the board's other cards. A camp whose Hearth falls is out; the
//! last standing wins, or the healthiest Hearth once the last wave's time is up. Then everyone is
//! back in the hall, fresh, for the next.

use super::mobs::{Foe, MobState};
use super::world::compass;
use super::*;
use crate::content::{SendDef, SiegeDef};

/// What the match is doing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SiegePhase {
    /// Waiting in the hall.
    Lobby,
    /// A way's seats filled: it begins in this many seconds.
    Countdown { left: f32 },
    Running,
    /// Over: who won (a camp's place in the match, or none for a draw), back to the hall in.
    Over { winner: Option<usize>, left: f32 },
}

/// A camp in the match.
#[derive(Debug, Clone)]
pub struct SiegeCamp {
    /// Its `[[siege.base]]`.
    pub def: usize,
    pub heroes: Vec<Id>,
    pub hearth: Id,
    pub ledger: Id,
    /// The Battle Trainer: the class's other cards, for gold.
    pub trainer: Id,
    pub rack: Id,
    /// Where waves come in.
    pub gate: Vec3,
    /// Its Hearth has fallen.
    pub out: bool,
    /// Bought for it by its rivals, coming with its next wave: (the send, who bought it).
    pub coming: Vec<(usize, Id)>,
}

#[derive(Debug, Clone)]
pub struct Siege {
    pub phase: SiegePhase,
    /// The way being played (or about to be).
    pub mode: Option<usize>,
    /// Heroes waiting in the hall for each way, in the order they joined.
    pub queues: Vec<Vec<Id>>,
    pub camps: Vec<SiegeCamp>,
    pub wave: u32,
    /// Seconds to the next wave.
    pub timer: f32,
    pub warned: bool,
    pub gold: BTreeMap<Id, u32>,
    pub income: BTreeMap<Id, u32>,
    /// Which camp each hero's sends go to.
    pub target: BTreeMap<Id, usize>,
    /// The once-a-match sends each hero has bought.
    pub bought_once: BTreeSet<(Id, usize)>,
    /// How many creatures each hero has sent for the next wave.
    pub sent: BTreeMap<Id, u32>,
    /// Each creature in the match and the camp it's sent against.
    pub foes: BTreeMap<Id, usize>,
    /// Each station built in the match and its camp.
    pub owner: BTreeMap<Id, usize>,
    /// The hall's banner.
    pub banner: Id,
    /// Each base's gate, found once (a long walk's search is dear).
    pub gates: Vec<Vec3>,
    /// Heroes ready for the next wave (F at their Hearth): everyone ready, it comes early.
    pub ready: BTreeSet<Id>,
    /// A wave's later groups, still to come in: (when, camp, creature, its place in the wave).
    pub trickle: Vec<(f32, usize, usize, usize)>,
}

impl Default for Siege {
    fn default() -> Self {
        Self {
            phase: SiegePhase::Lobby,
            mode: None,
            queues: Vec::new(),
            camps: Vec::new(),
            wave: 0,
            timer: 0.0,
            warned: false,
            gold: BTreeMap::new(),
            income: BTreeMap::new(),
            target: BTreeMap::new(),
            bought_once: BTreeSet::new(),
            sent: BTreeMap::new(),
            foes: BTreeMap::new(),
            owner: BTreeMap::new(),
            banner: 0,
            gates: Vec::new(),
            ready: BTreeSet::new(),
            trickle: Vec::new(),
        }
    }
}

/// How the hall's banner shows a way: its key, name, text, seats, the heroes in it, whether the
/// hero asking is.
#[derive(Debug, Clone)]
pub struct ModeView {
    pub key: String,
    pub name: String,
    pub text: String,
    pub seats: usize,
    pub heroes: Vec<String>,
    pub mine: bool,
}

/// A camp as the HUD shows it.
#[derive(Debug, Clone)]
pub struct CampView {
    pub name: String,
    pub heroes: Vec<String>,
    pub hearth: u32,
    pub max: u32,
    pub out: bool,
    pub mine: bool,
    pub target: bool,
}

/// A send as the ledger shows it.
#[derive(Debug, Clone)]
pub struct SendView {
    pub key: String,
    pub name: String,
    pub icon_mob: String,
    pub count: u32,
    pub cost: u32,
    pub income: u32,
    pub from: u32,
    pub text: String,
    /// Why not, if it can't be bought now.
    pub why: Option<String>,
}

/// The match as a hero in it sees it.
#[derive(Debug, Clone)]
pub struct SiegeHud {
    pub wave: u32,
    pub waves: u32,
    /// Seconds to the next wave (or to the start, counting down; or back to the hall, over).
    pub secs: f32,
    pub phase: &'static str,
    pub gold: u32,
    pub income: u32,
    pub camps: Vec<CampView>,
    /// What's coming at the hero's camp with the next wave (once warned): (name, count).
    pub coming: Vec<(String, u32)>,
    pub result: String,
    /// Heroes ready for the next wave, of how many, and whether this one is.
    pub ready: (usize, usize, bool),
    /// Creatures this hero has sent for the next wave, of the most they may.
    pub sent: (u32, u32),
}

/// The ledger and the Hearth stand this far to either side of the Hearth, the rack behind it;
/// heroes get up this far in front of it.
const SIDE: f32 = 6.0;
const RISE: f32 = 4.0;
/// The hall's banner stands this far in front of where heroes wait.
const BANNER_BACK: f32 = 5.0;

impl Survival {
    /// A Siege world: the hall's banner and the middle's veins.
    pub fn new_siege(content: Content, land: Box<dyn Land>, seed: u64, wall_now: f64) -> Self {
        let mut s = Self::bare(content.for_siege(), land, seed, wall_now);
        let modes = s.siege_def().map_or(0, |d| d.modes.len());
        s.siege = Some(Siege { queues: vec![Vec::new(); modes], ..Siege::default() });
        s.place_hall();
        s.place_nodes(None);
        let gates: Vec<Vec3> = s.siege_def().cloned().map(|d| d.bases.iter().map(|b| s.gate_of(s.clear_spot_near(s.ground_near(b.at[0], b.at[1], b.at[2]), HEARTH_CLEAR), d.middle, d.gate_walk)).collect()).unwrap_or_default();
        s.siege.as_mut().unwrap().gates = gates;
        s.events.clear();
        s
    }

    pub fn is_siege(&self) -> bool {
        self.siege.is_some()
    }

    pub(crate) fn siege_def(&self) -> Option<&SiegeDef> {
        self.content.siege.as_ref()
    }

    /// The book's page at camp for Siege: its key, name and what it is.
    pub fn siege_page(&self) -> Option<(String, String, String)> {
        let d = self.siege_def()?;
        Some((SIEGE_PAGE.to_string(), d.page.clone(), d.text.clone()))
    }

    fn place_hall(&mut self) {
        let Some(d) = self.siege_def().cloned() else { return };
        let back = d.lobby_facing + std::f32::consts::PI;
        let at = self.ground_near(d.lobby[0] + d.lobby_facing.cos() * BANNER_BACK, d.lobby[1] + d.lobby_facing.sin() * BANNER_BACK, d.lobby[2]);
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::SiegeBanner, pos: at, facing: back });
        if let Some(s) = self.siege.as_mut() {
            s.banner = id;
        }
    }

    /// Where a hero stands in the hall (spread a little by how many are there).
    fn hall_spot(&self, k: usize) -> Vec3 {
        let Some(d) = self.siege_def() else { return [0.0; 3] };
        let side = d.lobby_facing + std::f32::consts::FRAC_PI_2;
        let off = (k % 6) as f32 * 1.6 - 4.0;
        self.ground_near(d.lobby[0] + side.cos() * off, d.lobby[1] + side.sin() * off, d.lobby[2])
    }

    /// A hero come to Siege's world: in the hall, fresh.
    pub fn siege_arrive(&mut self, id: Id) {
        let k = self.players.len();
        let at = self.hall_spot(k);
        let f = self.siege_def().map_or(0.0, |d| d.lobby_facing);
        self.fresh_kit(id);
        if let Some(p) = self.players.get_mut(&id) {
            p.pos = at;
            p.facing = f;
            p.last_safe = at;
        }
        self.land_from_far(id);
        self.siege_news();
    }

    /// The camp a hero is in, in the match under way.
    pub fn camp_of(&self, id: Id) -> Option<usize> {
        let s = self.siege.as_ref()?;
        if !matches!(s.phase, SiegePhase::Running | SiegePhase::Over { .. }) {
            return None;
        }
        s.camps.iter().position(|c| c.heroes.contains(&id))
    }

    /// Whether two heroes are each other's foes in the match (rivals' camps). Out of a match, in
    /// Siege's world, nobody is.
    pub(crate) fn siege_foes(&self, a: Id, b: Id) -> bool {
        let s = self.siege.as_ref();
        if !s.is_some_and(|s| s.phase == SiegePhase::Running) {
            return false;
        }
        match (self.camp_of(a), self.camp_of(b)) {
            (Some(x), Some(y)) => x != y,
            _ => false,
        }
    }

    /// Whether `a` can strike `b` (both heroes): in Siege, rivals; elsewhere, friendly fire.
    pub(crate) fn hostile(&self, a: Id, b: Id) -> bool {
        if a == b {
            return false;
        }
        if self.is_siege() {
            return self.siege_foes(a, b);
        }
        self.content.pvp.friendly_fire
    }

    /// Whether `a` can help hero `b` (a heal, a shield, a boon, a pull to safety): in Siege,
    /// teammates only; elsewhere every hero, as friendly fire decides only who can be harmed.
    pub(crate) fn helpable(&self, a: Id, b: Id) -> bool {
        if a == b {
            return true;
        }
        if !self.players.contains_key(&b) {
            return false;
        }
        if self.is_siege() {
            return !self.siege_foes(a, b);
        }
        true
    }

    /// Whether hero `b` is spared `a`'s ability: one that mends friends (`heals`) neither strikes
    /// nor hinders a hero it can help, friendly fire or not.
    pub(crate) fn spared(&self, a: Id, b: Id, ability: usize) -> bool {
        self.content.abilities[ability].heals() && self.helpable(a, b)
    }

    /// A hero's gold in the match.
    pub fn gold_of(&self, id: Id) -> u32 {
        self.siege.as_ref().and_then(|s| s.gold.get(&id)).copied().unwrap_or(0)
    }

    pub(crate) fn spend_gold(&mut self, id: Id, n: u32) -> Result<(), String> {
        let have = self.gold_of(id);
        if have < n {
            return Err(format!("That costs {n} gold; you have {have}."));
        }
        if let Some(s) = self.siege.as_mut() {
            s.gold.insert(id, have - n);
        }
        self.events.push(Event::Siege { player: id });
        Ok(())
    }

    pub(crate) fn add_gold(&mut self, id: Id, n: u32) {
        if n == 0 {
            return;
        }
        if let Some(s) = self.siege.as_mut() {
            if s.phase == SiegePhase::Running && s.gold.contains_key(&id) {
                *s.gold.entry(id).or_default() += n;
                self.events.push(Event::Siege { player: id });
            }
        }
    }

    /// What a station costs here: in a match, its gold (none: it isn't built in Siege); else its
    /// own materials.
    pub(crate) fn station_cost(&self, def: usize) -> BTreeMap<String, u32> {
        let s = &self.content.stations[def];
        match self.siege_def() {
            Some(d) if self.is_siege() => d.build.get(&s.key).map(|n| BTreeMap::from([("gold".to_string(), *n)])).unwrap_or_default(),
            _ => s.cost.clone(),
        }
    }

    /// What mending a wrecked station costs here: half its gold, in a match.
    pub(crate) fn station_repair(&self, def: usize) -> BTreeMap<String, u32> {
        let s = &self.content.stations[def];
        match self.siege_def() {
            Some(d) if self.is_siege() => d.build.get(&s.key).map(|n| BTreeMap::from([("gold".to_string(), n.div_ceil(2))])).unwrap_or_default(),
            _ => s.repair.clone(),
        }
    }

    /// The building ground a hero builds on, for its ring on the ground: their camp's in a
    /// match, the camp's meadow at camp; none below, nor in Siege's hall.
    pub fn build_ground(&self, id: Id) -> Option<(Vec3, f32)> {
        if self.is_below() {
            return None;
        }
        if self.is_siege() {
            let d = self.siege_def()?;
            let camp = self.camp_of(id)?;
            let c = &self.siege.as_ref()?.camps[camp];
            return self.objs.get(&c.hearth).map(|o| (o.pos, d.build_radius));
        }
        let w = &self.content.world;
        Some((self.ground(w.build_center[0], w.build_center[1]), w.build_radius))
    }

    /// Whether station `def` is built here: in a match, those with a price; at camp, those in the
    /// menu.
    pub(crate) fn buildable(&self, def: usize) -> bool {
        let s = &self.content.stations[def];
        match self.siege_def() {
            Some(d) if self.is_siege() => d.build.contains_key(&s.key),
            _ => s.menu,
        }
    }

    /// Why a station can't stand at `pos` in a match, if it can't: only on the hero's own camp's
    /// ground, during a match.
    pub(crate) fn siege_site_problem(&self, builder: Option<Id>, pos: Vec3) -> Option<String> {
        let d = self.siege_def()?;
        let Some(camp) = builder.and_then(|b| self.camp_of(b)) else {
            return Some("Only in a match, on your own camp's ground.".into());
        };
        let s = self.siege.as_ref()?;
        let c = &s.camps[camp];
        let hearth = self.objs.get(&c.hearth).map_or(d.bases[c.def].at, |o| o.pos);
        let off = dist2d(pos, hearth) - d.build_radius;
        if off > 0.0 {
            return Some(format!("Outside your camp's ground ({:.0} yd round the Hearth): {:.0} yd {} of here.", d.build_radius, off.ceil(), compass(facing(pos, hearth))));
        }
        if dist2d(pos, c.gate) < 6.0 {
            return Some("That's the camp's gate: keep it clear.".into());
        }
        None
    }

    // --- the hall ----------------------------------------------------------------------------------

    /// Join the heroes waiting for a way to play (`key`), leaving any other.
    pub fn siege_join(&mut self, id: Id, key: &str) -> Result<String, String> {
        let d = self.siege_def().cloned().ok_or("There's no Siege here.")?;
        let m = d.modes.iter().position(|m| m.key == key || m.name.eq_ignore_ascii_case(key)).ok_or_else(|| {
            let keys: Vec<&str> = d.modes.iter().map(|m| m.key.as_str()).collect();
            format!("No way to play called {key}: {}.", keys.join(", "))
        })?;
        let s = self.siege.as_ref().ok_or("There's no Siege here.")?;
        if self.camp_of(id).is_some() {
            return Err("You're in the match.".into());
        }
        if matches!(s.phase, SiegePhase::Countdown { .. }) && s.mode != Some(m) {
            return Err("A match is about to begin: wait for the next.".into());
        }
        let seats = d.modes[m].bases.len() * d.modes[m].per_base;
        if s.queues[m].len() >= seats && !s.queues[m].contains(&id) {
            return Err(format!("{} is full.", d.modes[m].name));
        }
        let s = self.siege.as_mut().unwrap();
        for q in s.queues.iter_mut() {
            q.retain(|h| *h != id);
        }
        s.queues[m].push(id);
        let filled = s.queues[m].len();
        let name = self.players.get(&id).map_or(String::new(), |p| p.name.clone());
        for h in self.players.keys().copied().collect::<Vec<_>>() {
            if h != id {
                self.tell(h, format!("{name} waits for {} ({filled} of {seats}).", d.modes[m].name.to_lowercase()));
            }
        }
        // Full and nothing under way: it begins.
        if filled == seats && matches!(self.siege.as_ref().unwrap().phase, SiegePhase::Lobby) {
            self.siege_countdown(m);
        }
        self.siege_news();
        Ok(format!("You wait for {} ({filled} of {seats}).", d.modes[m].name.to_lowercase()))
    }

    /// Stop waiting.
    pub fn siege_leave_queue(&mut self, id: Id) -> Result<String, String> {
        let s = self.siege.as_mut().ok_or("There's no Siege here.")?;
        let was = s.queues.iter().any(|q| q.contains(&id));
        for q in s.queues.iter_mut() {
            q.retain(|h| *h != id);
        }
        // A countdown with too few left falls through.
        if let (SiegePhase::Countdown { .. }, Some(m)) = (s.phase, s.mode) {
            if s.queues[m].len() < 2 {
                s.phase = SiegePhase::Lobby;
                s.mode = None;
            }
        }
        self.siege_news();
        Ok(if was { "You stop waiting.".into() } else { "You weren't waiting.".into() })
    }

    /// Begin the way a hero waits for now, with whoever's waiting (two at least), its empty seats
    /// left empty (a camp with nobody isn't played).
    pub fn siege_start_now(&mut self, id: Id) -> Result<String, String> {
        let s = self.siege.as_ref().ok_or("There's no Siege here.")?;
        if !matches!(s.phase, SiegePhase::Lobby) {
            return Err("A match is under way: wait for it to end.".into());
        }
        let m = s.queues.iter().position(|q| q.contains(&id)).ok_or("Join a way to play first.")?;
        if s.queues[m].len() < 2 {
            return Err("It takes two at least.".into());
        }
        self.siege_countdown(m);
        self.siege_news();
        Ok("The match begins.".into())
    }

    fn siege_countdown(&mut self, m: usize) {
        let Some(d) = self.siege_def().cloned() else { return };
        let s = self.siege.as_mut().unwrap();
        s.phase = SiegePhase::Countdown { left: d.countdown };
        s.mode = Some(m);
        // Who holds which camp (as `begin_match` deals them out), said now, while there's time
        // to read it.
        let heroes = s.queues[m].clone();
        let mode = d.modes[m].clone();
        let n = heroes.len().min(mode.bases.len()).max(1);
        for (k, h) in heroes.iter().enumerate() {
            let base = d.bases.iter().find(|b| b.key == mode.bases[k % n]);
            let sub = match base {
                Some(b) => format!("You'll hold {} ({}). It begins in {} seconds", b.name, self.base_side(b.at), d.countdown as u32),
                None => format!("It begins in {} seconds", d.countdown as u32),
            };
            self.events.push(Event::Banner { player: Some(*h), title: mode.name.clone(), sub });
        }
    }

    // --- the match ---------------------------------------------------------------------------------

    pub(crate) fn step_siege(&mut self, dt: f32) {
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(phase) = self.siege.as_ref().map(|s| s.phase) else { return };
        // Who's gone (left the world).
        let gone: Vec<Id> = {
            let s = self.siege.as_ref().unwrap();
            s.queues.iter().flatten().chain(s.camps.iter().flat_map(|c| c.heroes.iter())).filter(|h| !self.players.contains_key(h)).copied().collect()
        };
        for h in gone {
            self.siege_gone(h);
        }
        match phase {
            SiegePhase::Lobby => {}
            SiegePhase::Countdown { left } => {
                let left = left - dt;
                if left <= 0.0 {
                    self.begin_match();
                } else if let Some(s) = self.siege.as_mut() {
                    s.phase = SiegePhase::Countdown { left };
                }
            }
            SiegePhase::Running => {
                let s = self.siege.as_mut().unwrap();
                s.timer -= dt;
                let (timer, warned) = (s.timer, s.warned);
                if timer <= d.warn_secs && !warned {
                    self.warn_camps();
                }
                if timer <= 0.0 {
                    self.next_wave();
                }
                self.step_trickle();
                self.check_hearths();
            }
            SiegePhase::Over { winner, left } => {
                let left = left - dt;
                if left <= 0.0 {
                    self.back_to_hall();
                } else if let Some(s) = self.siege.as_mut() {
                    s.phase = SiegePhase::Over { winner, left };
                }
            }
        }
    }

    /// A hero gone from the world: out of every queue and camp.
    fn siege_gone(&mut self, id: Id) {
        let Some(s) = self.siege.as_mut() else { return };
        for q in s.queues.iter_mut() {
            q.retain(|h| *h != id);
        }
        for c in s.camps.iter_mut() {
            c.heroes.retain(|h| *h != id);
        }
        s.gold.remove(&id);
        s.income.remove(&id);
        s.target.remove(&id);
        if let (SiegePhase::Countdown { .. }, Some(m)) = (s.phase, s.mode) {
            if s.queues[m].len() < 2 {
                s.phase = SiegePhase::Lobby;
                s.mode = None;
            }
        }
    }

    /// The countdown's done: the camps go up, the heroes to them, fresh, and the first wave's
    /// timer starts.
    fn begin_match(&mut self) {
        let Some(d) = self.siege_def().cloned() else { return };
        let s = self.siege.as_ref().unwrap();
        let Some(m) = s.mode else { return };
        let mode = d.modes[m].clone();
        let heroes: Vec<Id> = s.queues[m].clone();
        self.clear_match();
        // Heroes to camps in turn (two to a camp in a two against two), every camp with at least
        // one; a way short of heroes plays the camps that have someone.
        let mut camps: Vec<SiegeCamp> = Vec::new();
        let used = heroes.len().min(mode.bases.len());
        for key in mode.bases.iter().take(used) {
            let def = d.bases.iter().position(|b| &b.key == key).unwrap();
            camps.push(SiegeCamp { def, heroes: Vec::new(), hearth: 0, ledger: 0, trainer: 0, rack: 0, gate: d.bases[def].at, out: false, coming: Vec::new() });
        }
        let n = camps.len();
        for (k, h) in heroes.iter().enumerate() {
            camps[k % n].heroes.push(*h);
        }
        camps.retain(|c| !c.heroes.is_empty());
        if camps.len() < 2 {
            let s = self.siege.as_mut().unwrap();
            s.phase = SiegePhase::Lobby;
            s.mode = None;
            return;
        }
        for c in camps.iter_mut() {
            let b = d.bases[c.def].clone();
            // On open ground (a base's own spot can be in its flag's footing).
            let at = self.clear_spot_near(self.ground_near(b.at[0], b.at[1], b.at[2]), HEARTH_CLEAR);
            let hearth_def = self.content.station(&d.hearth).unwrap();
            let health = self.content.stations[hearth_def].health;
            let uid = self.uid();
            c.hearth = self.alloc();
            self.objs.insert(c.hearth, Obj { kind: ObjKind::Station { def: hearth_def, uid, health, damaged: false }, pos: at, facing: b.facing });
            self.events.push(Event::ObjSpawned { id: c.hearth });
            let side = b.facing + std::f32::consts::FRAC_PI_2;
            let spot = |s: &Self, ang: f32, r: f32| s.clear_spot_near([at[0] + ang.cos() * r, at[1] + ang.sin() * r, at[2]], 1.2);
            let ledger_at = spot(self, side, SIDE);
            c.ledger = self.alloc();
            self.objs.insert(c.ledger, Obj { kind: ObjKind::SiegeLedger { camp: c.def }, pos: ledger_at, facing: b.facing });
            self.events.push(Event::ObjSpawned { id: c.ledger });
            let rack_at = spot(self, side + std::f32::consts::PI, SIDE);
            c.rack = self.alloc();
            self.objs.insert(c.rack, Obj { kind: ObjKind::Rack, pos: rack_at, facing: b.facing });
            self.events.push(Event::ObjSpawned { id: c.rack });
            // The trainer behind the Hearth.
            let trainer_at = spot(self, b.facing + std::f32::consts::PI, SIDE);
            c.trainer = self.alloc();
            self.objs.insert(c.trainer, Obj { kind: ObjKind::SiegeTrainer { camp: c.def }, pos: trainer_at, facing: b.facing });
            self.events.push(Event::ObjSpawned { id: c.trainer });
            c.gate = self.siege.as_ref().and_then(|s| s.gates.get(c.def).copied()).unwrap_or(at);
        }
        let names: Vec<String> = camps.iter().map(|c| d.bases[c.def].name.clone()).collect();
        // Where each camp's waves come in, marked on the ground till the first.
        for c in &camps {
            self.mark(c.gate, GATE_MARK, d.first_gap);
        }
        let s = self.siege.as_mut().unwrap();
        s.camps = camps.clone();
        s.ready.clear();
        s.phase = SiegePhase::Running;
        s.wave = 0;
        s.timer = d.first_gap;
        s.warned = false;
        s.sent.clear();
        s.gold.clear();
        s.income.clear();
        s.target.clear();
        s.bought_once.clear();
        for q in s.queues.iter_mut() {
            q.retain(|h| !heroes.contains(h));
        }
        for (k, c) in camps.iter().enumerate() {
            for h in &c.heroes {
                s.gold.insert(*h, d.start_gold);
                s.income.insert(*h, d.start_income);
                // Sends go to the next camp round (the only rival, one against one).
                s.target.insert(*h, (k + 1) % camps.len());
            }
        }
        for (k, c) in camps.iter().enumerate() {
            for h in c.heroes.clone() {
                self.fresh_kit(h);
                self.to_camp(h, k);
                let rivals: Vec<&str> = names.iter().enumerate().filter(|(j, _)| *j != k).map(|(_, n)| n.as_str()).collect();
                let partner = c.heroes.iter().filter(|o| **o != h).filter_map(|o| self.players.get(o).map(|p| p.name.clone())).next();
                let with = partner.map_or(String::new(), |p| format!(" with {p}"));
                let gate = compass(facing(self.players.get(&h).map_or(c.gate, |p| p.pos), c.gate));
                self.events.push(Event::Banner { player: Some(h), title: format!("Hold {}", names[k]), sub: format!("Keep your Hearth standing. Wave 1 in {} s, from the {gate}", d.first_gap as u32) });
                // A few short lines, one thing each, rather than one long one.
                self.tell(h, format!("{}: you hold {}{with} against {}.", mode.name, names[k], rivals.join(" and ")));
                self.tell(h, format!("Waves come in at your gate, {gate} of your Hearth (marked red on the ground), every {} seconds; your rivals' sends come with them.", d.wave_secs as u32));
                self.tell(h, "Gold: build defences (H), send creatures at a rival from the Mercenary Ledger (F at it: each raises your income), learn cards at the Battle Trainer.");
                self.tell(h, "Food and salves come with every wave (T eats). Ready early? F at your Hearth: when everyone is, the next wave comes at once.");
            }
        }
        self.events.push(Event::Log(format!("siege: {} begins ({})", mode.name, names.join(", "))));
        self.siege_news();
        self.events.push(Event::Dirty);
    }

    /// Where waves come in to a camp at `at`: this far along the way from it to the middle.
    /// On dry, open ground: if the spot that far along is in water or a wall, nearer in, a few
    /// yards at a time.
    fn gate_of(&self, at: Vec3, middle: [f32; 3], walk: f32) -> Vec3 {
        let mid = self.ground_near(middle[0], middle[1], middle[2]);
        let path = self.land.path(at, mid).filter(|p| !p.is_empty()).unwrap_or_else(|| vec![mid]);
        let along = |walk: f32| -> Vec3 {
            let mut from = at;
            let mut left = walk;
            for q in &path {
                let d = dist2d(from, *q);
                if d >= left {
                    let k = left / d.max(0.01);
                    let p = [from[0] + (q[0] - from[0]) * k, from[1] + (q[1] - from[1]) * k, from[2]];
                    return self.ground_near(p[0], p[1], from[2] + (q[2] - from[2]) * k);
                }
                left -= d;
                from = *q;
            }
            from
        };
        let open = |p: Vec3| (0..8).all(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            let (x, y) = (p[0] + a.cos() * 3.0, p[1] + a.sin() * 3.0);
            !self.land.blocked(x, y) && !self.land.water(x, y)
        }) && !self.land.water(p[0], p[1]);
        let mut w = walk;
        while w > 20.0 {
            let p = along(w);
            if open(p) {
                return p;
            }
            w -= 4.0;
        }
        along(walk)
    }

    /// Open ground near `at` (an object's spot): nothing solid nor wet `clear` round it.
    fn clear_spot_near(&self, at: Vec3, clear: f32) -> Vec3 {
        for ring in 0..12 {
            let r = ring as f32 * 1.5;
            let n = (ring * 6).max(1);
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let (x, y) = (at[0] + a.cos() * r, at[1] + a.sin() * r);
                let clear = !self.land.blocked(x, y) && !self.land.water(x, y) && [clear * 0.5, clear].iter().all(|d| (0..8).all(|q| {
                    let b = q as f32 * std::f32::consts::FRAC_PI_4;
                    !self.land.blocked(x + b.cos() * d, y + b.sin() * d) && !self.land.water(x + b.cos() * d, y + b.sin() * d)
                }));
                if clear {
                    return self.ground_near(x, y, at[2]);
                }
            }
        }
        self.ground_near(at[0], at[1], at[2])
    }

    /// Put a hero by their camp's Hearth (getting up, or at the start).
    fn to_camp(&mut self, id: Id, camp: usize) {
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(c) = self.siege.as_ref().and_then(|s| s.camps.get(camp)).cloned() else { return };
        let at = self.objs.get(&c.hearth).map_or(d.bases[c.def].at, |o| o.pos);
        let f = d.bases[c.def].facing;
        let k = c.heroes.iter().position(|h| *h == id).unwrap_or(0) as f32;
        let side = f + std::f32::consts::FRAC_PI_2;
        let p = [at[0] + f.cos() * RISE + side.cos() * (k * 1.5), at[1] + f.sin() * RISE + side.sin() * (k * 1.5), at[2]];
        let spot = self.landing_spot(self.ground_near(p[0], p[1], at[2]));
        self.cancel_preview(id);
        self.put_at(id, spot, f);
        self.land_from_far(id);
    }

    /// A hero as a match begins (and ends): their class's first cards, the plain kit, whole.
    pub(crate) fn fresh_kit(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let class = p.class.clone();
        let (loadout, mut owned) = self.fresh_abilities(&class);
        // Siege leaves the beta page out (a dev build owns it elsewhere).
        owned.retain(|k| !self.content.ability(k).is_some_and(|a| self.content.abilities[a].is_beta()));
        let c = self.content.clone();
        let item = |k: &str| c.item(k).unwrap_or(0);
        self.cancel_preview(id);
        self.end_ability_action(id);
        let p = self.players.get_mut(&id).unwrap();
        p.loadout = loadout;
        p.owned = owned;
        p.weapon = item(&c.start.weapon);
        p.shield = item(&c.start.shield);
        p.bow = item(&c.start.bow);
        p.bag = c.start.items.iter().filter_map(|(k, n)| Some((c.item(k)?, *n))).collect();
        p.armor.clear();
        p.pack = None;
        p.quick = None;
        p.effects.clear();
        p.cooldowns.clear();
        p.valor = c.valor.start;
        p.health = p.max_health;
        p.stamina = c.player.stamina;
        if matches!(p.action, Action::Dead { .. }) {
            p.action = Action::Ready;
        }
        let (h, m) = (p.health, p.max_health);
        self.cc.remove(&id);
        self.stances.remove(&id);
        self.anchors.remove(&id);
        self.events.push(Event::Health { id, health: h, max: m });
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Effects { player: id });
        self.events.push(Event::Gear { player: id });
        self.update_speed(id);
    }

    /// The last match's leavings gone: its creatures, stations, ledgers and racks.
    fn clear_match(&mut self) {
        let Some(s) = self.siege.as_mut() else { return };
        s.trickle.clear();
        s.ready.clear();
        let mobs: Vec<Id> = std::mem::take(&mut s.foes).into_keys().collect();
        let mut objs: Vec<Id> = std::mem::take(&mut s.owner).into_keys().collect();
        for c in std::mem::take(&mut s.camps) {
            objs.extend([c.hearth, c.ledger, c.trainer, c.rack]);
        }
        for m in mobs {
            if self.mobs.remove(&m).is_some() {
                self.events.push(Event::MobGone { id: m });
            }
        }
        // Creatures left from anything else (a second form stepped out), and fires.
        let strays: Vec<Id> = self.mobs.keys().copied().collect();
        for m in strays {
            self.mobs.remove(&m);
            self.events.push(Event::MobGone { id: m });
        }
        let fires: Vec<Id> = self.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Fire { .. } | ObjKind::Ghost { .. })).map(|(id, _)| *id).collect();
        for o in objs.into_iter().chain(fires) {
            if self.objs.remove(&o).is_some() {
                self.events.push(Event::ObjGone { id: o });
            }
        }
        self.missiles.clear();
        self.pending.clear();
    }

    /// A wave comes in at every camp standing: the base wave, and what was bought for it. Every
    /// hero in the match is paid their income first.
    fn next_wave(&mut self) {
        let Some(d) = self.siege_def().cloned() else { return };
        let s = self.siege.as_mut().unwrap();
        // The last wave's time is up: the healthiest Hearth wins.
        if s.wave >= d.waves {
            self.end_on_health();
            return;
        }
        s.wave += 1;
        s.timer = d.wave_secs;
        s.warned = false;
        s.sent.clear();
        s.ready.clear();
        let wave = s.wave;
        let paid: Vec<(Id, u32)> = s.income.iter().map(|(h, n)| (*h, *n)).collect();
        for (h, n) in &paid {
            *s.gold.entry(*h).or_default() += n;
        }
        let camps = s.camps.clone();
        let tough = 1.0 + d.toughen * wave.saturating_sub(1) as f32;
        for (k, c) in camps.iter().enumerate() {
            if c.out {
                continue;
            }
            let mut foes: Vec<(usize, Option<Id>)> = self.base_wave(wave, c.heroes.len()).into_iter().map(|f| (f, None)).collect();
            let mut bought: Vec<(String, u32)> = Vec::new();
            for (send, from) in &c.coming {
                let sd = &d.sends[*send];
                if let Some(def) = self.content.mob(&sd.mob) {
                    for _ in 0..sd.count {
                        foes.push((def, Some(*from)));
                    }
                    let name = self.content.mobs[def].name.clone();
                    match bought.iter_mut().find(|b| b.0 == name) {
                        Some(b) => b.1 += sd.count,
                        None => bought.push((name, sd.count)),
                    }
                }
            }
            self.siege.as_mut().unwrap().camps[k].coming.clear();
            let n = foes.len();
            // In groups a few seconds apart, not all at once: a hero with a sword meets a few at
            // a time, as one with a fire nova does.
            for (j, (def, _)) in foes.iter().enumerate() {
                let group = j / TRICKLE_GROUP;
                if group == 0 {
                    self.wave_foe_in(k, *def, j, n, tough);
                } else {
                    let when = self.clock + group as f32 * TRICKLE_SECS;
                    self.siege.as_mut().unwrap().trickle.push((when, k, *def, j));
                }
            }
            let sub = if bought.is_empty() {
                format!("{n} at your gate")
            } else {
                let list: Vec<String> = bought.iter().map(|(name, n)| if *n == 1 { name.clone() } else { format!("{n} {name}") }).collect();
                format!("{n} at your gate, among them {}", list.join(", "))
            };
            for h in &c.heroes {
                self.events.push(Event::Banner { player: Some(*h), title: format!("Wave {wave} of {}", d.waves), sub: sub.clone() });
            }
        }
        // Supplies for everyone still standing.
        let standing: Vec<Id> = camps.iter().filter(|c| !c.out).flat_map(|c| c.heroes.iter().copied()).collect();
        let mut given: BTreeMap<Id, Vec<String>> = BTreeMap::new();
        for h in &standing {
            for (key, n) in &d.supply {
                let Some(item) = self.content.item(key) else { continue };
                let Some(p) = self.players.get_mut(h) else { continue };
                let have = p.bag.get(&item).copied().unwrap_or(0);
                let add = (*n).min(d.supply_most.saturating_sub(have));
                if add > 0 {
                    *p.bag.entry(item).or_default() += add;
                    given.entry(*h).or_default().push(format!("{add} {}", self.content.items[item].name));
                }
            }
            if given.contains_key(h) {
                self.events.push(Event::Bags { player: *h });
            }
        }
        for (h, n) in paid {
            let more = given.get(&h).map_or(String::new(), |g| format!(" Supplies: {}.", g.join(", ")));
            self.tell(h, format!("Wave {wave}: your income, {n} gold, is paid.{more}"));
        }
        self.events.push(Event::Log(format!("siege wave {wave}")));
        self.siege_news();
    }

    /// One of a wave's creatures comes in at camp `k`'s gate (the `j`th of `n`, spread across the
    /// way in, a few deep).
    fn wave_foe_in(&mut self, k: usize, def: usize, j: usize, n: usize, tough: f32) {
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(c) = self.siege.as_ref().and_then(|s| s.camps.get(k)).cloned() else { return };
        if c.out {
            return;
        }
        let ang = (j as f32 / n.max(1) as f32) * std::f32::consts::TAU;
        let r = 1.5 + (j % 3) as f32 * 1.5;
        let at = self.ground_near(c.gate[0] + ang.cos() * r, c.gate[1] + ang.sin() * r, c.gate[2]);
        let at = if self.land.blocked_near(at[0], at[1], at[2]) { c.gate } else { at };
        let id = self.spawn_mob(def, at, None, true);
        let max = ((self.base_health(def) as f32) * tough * d.foe_health).round().max(1.0) as u32;
        if let Some(m) = self.mobs.get_mut(&id) {
            m.health = max;
            m.max_health = max;
        }
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.siege.as_mut().unwrap().foes.insert(id, k);
        self.siege_next(id);
    }

    /// The groups of a wave due now come in.
    fn step_trickle(&mut self) {
        let now = self.clock;
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(s) = self.siege.as_mut() else { return };
        if s.trickle.is_empty() {
            return;
        }
        let wave = s.wave;
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut s.trickle).into_iter().partition(|t| t.0 <= now);
        s.trickle = keep;
        let tough = 1.0 + d.toughen * wave.saturating_sub(1) as f32;
        for (_, k, def, j) in due {
            self.wave_foe_in(k, def, j, j + 1, tough);
        }
    }

    /// Who a camp's base wave brings in wave `wave`, for `heroes` heroes in it.
    pub fn base_wave(&mut self, wave: u32, heroes: usize) -> Vec<usize> {
        let Some(d) = self.siege_def().cloned() else { return Vec::new() };
        let mut worth = (d.budget + d.growth * wave.saturating_sub(1) as f32) * (1.0 + d.per_hero * heroes.saturating_sub(1) as f32);
        let open: Vec<(usize, f32)> = d.roster.iter().filter(|f| f.from <= wave).filter_map(|f| Some((self.content.mob(&f.mob)?, f.cost))).collect();
        let cheapest = open.iter().map(|o| o.1).fold(f32::MAX, f32::min);
        let mut out = Vec::new();
        while out.len() < d.most && worth >= cheapest - 1.0e-3 {
            let can: Vec<&(usize, f32)> = open.iter().filter(|o| o.1 <= worth + 1.0e-3).collect();
            if can.is_empty() {
                break;
            }
            let (def, cost) = *can[self.rng.gen_range(0..can.len())];
            out.push(def);
            worth -= cost;
        }
        if out.is_empty() {
            if let Some(o) = open.first() {
                out.push(o.0);
            }
        }
        out
    }

    /// `warn_secs` before a wave: each camp's heroes hear what's been bought for them.
    fn warn_camps(&mut self) {
        let Some(d) = self.siege_def().cloned() else { return };
        let s = self.siege.as_mut().unwrap();
        s.warned = true;
        let camps = s.camps.clone();
        let left = s.timer.max(1.0);
        for c in camps.iter().filter(|c| !c.out) {
            self.mark(c.gate, GATE_MARK, left);
        }
        for c in camps.iter().filter(|c| !c.out && !c.coming.is_empty()) {
            let list = self.coming_text(c);
            let from: BTreeSet<String> = c.coming.iter().filter_map(|(_, h)| self.players.get(h).map(|p| p.name.clone())).collect();
            let from: Vec<String> = from.into_iter().collect();
            for h in &c.heroes {
                self.events.push(Event::Banner { player: Some(*h), title: "Coming at you".into(), sub: format!("{list}, from {} (in {} s)", from.join(" and "), d.warn_secs as u32) });
            }
        }
        self.siege_news();
    }

    fn coming_list(&self, c: &SiegeCamp) -> Vec<(String, u32)> {
        let Some(d) = self.siege_def() else { return Vec::new() };
        let mut out: Vec<(String, u32)> = Vec::new();
        for (send, _) in &c.coming {
            let sd = &d.sends[*send];
            let name = self.content.mob(&sd.mob).map_or(sd.mob.clone(), |m| self.content.mobs[m].name.clone());
            match out.iter_mut().find(|o| o.0 == name) {
                Some(o) => o.1 += sd.count,
                None => out.push((name, sd.count)),
            }
        }
        out
    }

    fn coming_text(&self, c: &SiegeCamp) -> String {
        let list: Vec<String> = self.coming_list(c).into_iter().map(|(n, k)| if k == 1 { n } else { format!("{k} {n}") }).collect();
        list.join(", ")
    }

    /// A creature in the match looks for whom to fight: a hero near it, else the nearest
    /// standing defence of the camp it was sent at (a defence counted nearer, as raids do), else
    /// that camp's Hearth. A camp out: it's gone.
    pub(crate) fn siege_next(&mut self, id: Id) {
        let Some(camp) = self.siege.as_ref().and_then(|s| s.foes.get(&id)).copied() else { return };
        let Some(c) = self.siege.as_ref().and_then(|s| s.camps.get(camp)).cloned() else { return };
        let Some(m) = self.mobs.get(&id) else { return };
        let pos = m.pos;
        if c.out {
            self.mobs.remove(&id);
            self.events.push(Event::MobGone { id });
            return;
        }
        let hero = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, pos) <= SIEGE_SIGHT && (p.pos[2] - pos[2]).abs() < 8.0).min_by(|a, b| dist2d(a.1.pos, pos).total_cmp(&dist2d(b.1.pos, pos))).map(|(h, _)| *h);
        let foe = match hero {
            Some(h) => Some(Foe::Hero(h)),
            None => {
                let owner = &self.siege.as_ref().unwrap().owner;
                self.objs
                    .iter()
                    .filter_map(|(oid, o)| match o.kind {
                        ObjKind::Station { def, damaged: false, .. } if owner.get(oid) == Some(&camp) && self.content.stations[def].defense => Some((*oid, dist2d(o.pos, pos) - SIEGE_PULL)),
                        _ => None,
                    })
                    .filter(|(_, d)| *d <= SIEGE_DEFENSE_REACH)
                    .min_by(|a, b| a.1.total_cmp(&b.1))
                    .map(|(o, _)| Foe::Obj(o))
                    .or_else(|| self.objs.get(&c.hearth).filter(|o| matches!(o.kind, ObjKind::Station { damaged: false, .. })).map(|_| Foe::Obj(c.hearth)))
            }
        };
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = match foe {
            Some(f) => MobState::Chase { foe: f },
            None => MobState::Idle { wander_at: self.clock + 2.0 },
        };
        m.think = 0.0;
    }

    /// A creature of the match felled: gold for the hero who did it.
    pub(crate) fn siege_kill(&mut self, mid: Id, killer: Id) {
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(m) = self.mobs.get(&mid) else { return };
        let gold = (self.content.mobs[m.def].health as f32 * d.kill_gold).round() as u32;
        if let Some(s) = self.siege.as_mut() {
            s.foes.remove(&mid);
        }
        if self.players.contains_key(&killer) && gold > 0 {
            self.add_gold(killer, gold);
            self.events.push(Event::Flash { player: killer, text: format!("+{gold} gold") });
        }
    }

    /// A hero felled in the match: a rival who did it takes a share of their gold.
    pub(crate) fn siege_fell(&mut self, id: Id, by: Option<Id>) {
        let Some(d) = self.siege_def().cloned() else { return };
        let Some(k) = by.filter(|k| self.siege_foes(*k, id)) else { return };
        let take = (self.gold_of(id) as f32 * d.steal).floor() as u32;
        if take == 0 {
            return;
        }
        let s = self.siege.as_mut().unwrap();
        *s.gold.entry(id).or_default() -= take;
        *s.gold.entry(k).or_default() += take;
        let (a, b) = (self.players.get(&k).map_or(String::new(), |p| p.name.clone()), self.players.get(&id).map_or(String::new(), |p| p.name.clone()));
        self.tell(k, format!("You take {take} gold from {b}."));
        self.tell(id, format!("{a} takes {take} of your gold."));
        self.events.push(Event::Siege { player: k });
        self.events.push(Event::Siege { player: id });
    }

    /// Where a hero gets up in a match (their camp's Hearth), and how long they're down.
    pub(crate) fn siege_revive_at(&self, id: Id) -> Option<(Vec3, f32)> {
        let d = self.siege_def()?;
        let camp = self.camp_of(id)?;
        let c = &self.siege.as_ref()?.camps[camp];
        let at = self.objs.get(&c.hearth).map_or(d.bases[c.def].at, |o| o.pos);
        let f = d.bases[c.def].facing;
        let p = [at[0] + f.cos() * RISE, at[1] + f.sin() * RISE, at[2]];
        Some((self.landing_spot(self.ground_near(p[0], p[1], at[2])), f))
    }

    pub(crate) fn siege_respawn_secs(&self, id: Id) -> Option<f32> {
        self.camp_of(id)?;
        self.siege_def().map(|d| d.respawn_secs)
    }

    /// A camp whose Hearth has fallen is out; one camp left standing wins.
    fn check_hearths(&mut self) {
        let Some(d) = self.siege_def().cloned() else { return };
        let camps = self.siege.as_ref().unwrap().camps.clone();
        for (k, c) in camps.iter().enumerate() {
            if c.out {
                continue;
            }
            let fallen = self.objs.get(&c.hearth).is_none_or(|o| matches!(o.kind, ObjKind::Station { damaged: true, .. }));
            if !fallen {
                continue;
            }
            let name = d.bases[c.def].name.clone();
            self.siege.as_mut().unwrap().camps[k].out = true;
            self.announce(format!("The Hearth at {name} has fallen: {} is out.", upper(&name)));
            for h in &c.heroes {
                self.events.push(Event::Banner { player: Some(*h), title: "Your Hearth Falls".into(), sub: "You're out of the match".into() });
            }
            // Its waves are done with.
            let theirs: Vec<Id> = self.siege.as_ref().unwrap().foes.iter().filter(|(_, c)| **c == k).map(|(m, _)| *m).collect();
            for m in theirs {
                self.siege.as_mut().unwrap().foes.remove(&m);
                if self.mobs.remove(&m).is_some() {
                    self.events.push(Event::MobGone { id: m });
                }
            }
            // Sends at it go elsewhere: whoever targeted it turns to the next camp standing.
            let s = self.siege.as_mut().unwrap();
            let n = s.camps.len();
            let standing: Vec<bool> = s.camps.iter().map(|c| !c.out).collect();
            for (h, t) in s.target.iter_mut() {
                if *t == k {
                    let mine = s.camps.iter().position(|c| c.heroes.contains(h));
                    if let Some(j) = (1..=n).map(|i| (k + i) % n).find(|j| standing[*j] && Some(*j) != mine) {
                        *t = j;
                    }
                }
            }
        }
        let s = self.siege.as_ref().unwrap();
        let standing: Vec<usize> = s.camps.iter().enumerate().filter(|(_, c)| !c.out).map(|(k, _)| k).collect();
        match standing.len() {
            0 => self.end_match(None),
            1 => self.end_match(Some(standing[0])),
            _ => {}
        }
    }

    /// The last wave's time is up: the Hearth with the most of its health left wins (a draw if
    /// it's even).
    fn end_on_health(&mut self) {
        let s = self.siege.as_ref().unwrap();
        let share = |c: &SiegeCamp| match self.objs.get(&c.hearth).map(|o| &o.kind) {
            Some(ObjKind::Station { def, health, damaged: false, .. }) => *health as f32 / self.content.stations[*def].health.max(1) as f32,
            _ => 0.0,
        };
        let mut best: Vec<(usize, f32)> = s.camps.iter().enumerate().filter(|(_, c)| !c.out).map(|(k, c)| (k, share(c))).collect();
        best.sort_by(|a, b| b.1.total_cmp(&a.1));
        let winner = match best.as_slice() {
            [a, b, ..] if (a.1 - b.1).abs() < 1.0e-4 => None,
            [a, ..] => Some(a.0),
            [] => None,
        };
        self.end_match(winner);
    }

    fn end_match(&mut self, winner: Option<usize>) {
        let Some(d) = self.siege_def().cloned() else { return };
        let s = self.siege.as_mut().unwrap();
        if !matches!(s.phase, SiegePhase::Running) {
            return;
        }
        s.phase = SiegePhase::Over { winner, left: d.over_secs };
        let camps = s.camps.clone();
        let names = |c: &SiegeCamp, s: &Self| -> String {
            let n: Vec<String> = c.heroes.iter().filter_map(|h| s.players.get(h).map(|p| p.name.clone())).collect();
            n.join(" and ")
        };
        let text = match winner {
            Some(k) => format!("{} holds {} and wins the siege!", names(&camps[k], self), d.bases[camps[k].def].name),
            None => "The siege ends in a draw.".to_string(),
        };
        self.announce(text.clone());
        for (k, c) in camps.iter().enumerate() {
            for h in &c.heroes {
                let title = match winner {
                    Some(w) if w == k => "Victory",
                    Some(_) => "Defeat",
                    None => "A Draw",
                };
                self.events.push(Event::Banner { player: Some(*h), title: title.into(), sub: format!("Back to the hall in {} seconds", d.over_secs as u32) });
            }
        }
        // Their creatures stand down.
        let foes: Vec<Id> = std::mem::take(&mut self.siege.as_mut().unwrap().foes).into_keys().collect();
        for m in foes {
            if self.mobs.remove(&m).is_some() {
                self.events.push(Event::MobGone { id: m });
            }
        }
        self.events.push(Event::Log(format!("siege over: {text}")));
        self.siege_news();
    }

    /// Everyone back in the hall, fresh; the field cleared for the next match.
    fn back_to_hall(&mut self) {
        let heroes: Vec<Id> = self.siege.as_ref().map(|s| s.camps.iter().flat_map(|c| c.heroes.clone()).collect()).unwrap_or_default();
        self.clear_match();
        let s = self.siege.as_mut().unwrap();
        s.phase = SiegePhase::Lobby;
        s.mode = None;
        s.wave = 0;
        s.gold.clear();
        s.income.clear();
        s.target.clear();
        s.bought_once.clear();
        let f = self.siege_def().map_or(0.0, |d| d.lobby_facing);
        for (k, h) in heroes.into_iter().enumerate() {
            if !self.players.contains_key(&h) {
                continue;
            }
            self.fresh_kit(h);
            let at = self.landing_spot(self.hall_spot(k));
            self.put_at(h, at, f);
            self.land_from_far(h);
        }
        // A way that filled meanwhile begins.
        let full = self.siege_def().and_then(|d| {
            let s = self.siege.as_ref()?;
            d.modes.iter().enumerate().position(|(m, md)| s.queues[m].len() >= md.bases.len() * md.per_base)
        });
        if let Some(m) = full {
            self.siege_countdown(m);
        }
        self.siege_news();
    }

    /// A hero leaves Siege's world (home by the book): out of the match and the queues.
    pub fn siege_depart(&mut self, id: Id) {
        self.siege_gone(id);
        self.siege_news();
    }

    // --- the ledger ------------------------------------------------------------------------------

    /// Buy a send at the ledger: it comes at the hero's target with its next wave, and the
    /// hero's income goes up.
    pub fn siege_send(&mut self, id: Id, key: &str) -> Result<String, String> {
        let d = self.siege_def().cloned().ok_or("There's no Siege here.")?;
        let k = d.sends.iter().position(|s| s.key == key).ok_or_else(|| {
            let keys: Vec<&str> = d.sends.iter().map(|s| s.key.as_str()).collect();
            format!("No send called {key}: {}.", keys.join(", "))
        })?;
        let sd = d.sends[k].clone();
        if let Some(why) = self.send_problem(id, k, &sd) {
            return Err(why);
        }
        let target = self.siege.as_ref().unwrap().target[&id];
        self.spend_gold(id, sd.cost)?;
        let s = self.siege.as_mut().unwrap();
        *s.income.entry(id).or_default() += sd.income;
        s.camps[target].coming.push((k, id));
        *s.sent.entry(id).or_default() += sd.count;
        if sd.once {
            s.bought_once.insert((id, k));
        }
        let to = d.bases[s.camps[target].def].name.clone();
        let mob = self.content.mob(&sd.mob).map_or(sd.mob.clone(), |m| self.content.mobs[m].name.clone());
        let what = if sd.count == 1 { mob } else { format!("{} {mob}", sd.count) };
        self.siege_news();
        Ok(format!("{what} will march on {to} with its next wave; your income is up {} a wave.", sd.income))
    }

    fn send_problem(&self, id: Id, k: usize, sd: &SendDef) -> Option<String> {
        let s = self.siege.as_ref()?;
        if s.phase != SiegePhase::Running {
            return Some("Only in a match.".into());
        }
        let mine = self.camp_of(id)?;
        if s.camps[mine].out {
            return Some("Your Hearth has fallen.".into());
        }
        let t = *s.target.get(&id)?;
        if t == mine || s.camps.get(t).is_none_or(|c| c.out) {
            return Some("No rival to send it to.".into());
        }
        // Bought now, it marches with the next wave.
        if s.wave + 1 < sd.from {
            return Some(format!("From wave {}.", sd.from));
        }
        if sd.once && s.bought_once.contains(&(id, k)) {
            return Some("Once a match, and you've sent it.".into());
        }
        // Only so many for one wave.
        let d = self.siege_def()?;
        let cap = d.send_cap.saturating_add(d.send_cap_growth.saturating_mul(s.wave));
        let sent = s.sent.get(&id).copied().unwrap_or(0);
        if sent + sd.count > cap {
            let what = if sd.count == 1 { "it's 1".to_string() } else { format!("this is {}", sd.count) };
            return Some(format!("Wave {}'s limit: {sent} of {cap} creatures sent, {what}. {} more each wave", s.wave + 1, d.send_cap_growth));
        }
        if self.gold_of(id) < sd.cost {
            return Some(format!("Needs {} gold (you have {})", sd.cost, self.gold_of(id)));
        }
        None
    }

    /// Whether a creature is one of a match's (sent or of the base wave).
    pub(crate) fn is_siege_foe(&self, id: Id) -> bool {
        self.siege.as_ref().is_some_and(|s| s.foes.contains_key(&id))
    }

    /// Send at the next rival camp round (or a named one).
    pub fn siege_target(&mut self, id: Id, to: Option<&str>) -> Result<String, String> {
        let d = self.siege_def().cloned().ok_or("There's no Siege here.")?;
        let mine = self.camp_of(id).ok_or("Only in a match.")?;
        let s = self.siege.as_ref().unwrap();
        let n = s.camps.len();
        let now = s.target.get(&id).copied().unwrap_or((mine + 1) % n);
        let next = match to {
            Some(name) => s.camps.iter().position(|c| d.bases[c.def].key == name || d.bases[c.def].name.to_lowercase().contains(&name.to_lowercase())).filter(|j| *j != mine && !s.camps[*j].out).ok_or("No such rival camp standing.")?,
            None => (1..=n).map(|i| (now + i) % n).find(|j| *j != mine && !s.camps[*j].out).ok_or("No rival camp standing.")?,
        };
        let name = d.bases[s.camps[next].def].name.clone();
        self.siege.as_mut().unwrap().target.insert(id, next);
        self.siege_news();
        Ok(format!("Your sends go to {name}."))
    }

    /// The ledger as a hero sees it: every send, what it costs and does, and why not now.
    pub fn siege_ledger(&self, id: Id) -> Vec<SendView> {
        let Some(d) = self.siege_def() else { return Vec::new() };
        d.sends
            .iter()
            .enumerate()
            .map(|(k, sd)| {
                let mob = self.content.mob(&sd.mob);
                let name = mob.map_or(sd.mob.clone(), |m| self.content.mobs[m].name.clone());
                SendView {
                    key: sd.key.clone(),
                    name: if sd.count == 1 { name } else { format!("{} {name}", sd.count) },
                    icon_mob: sd.mob.clone(),
                    count: sd.count,
                    cost: sd.cost,
                    income: sd.income,
                    from: sd.from,
                    text: sd.text.clone(),
                    why: self.send_problem(id, k, sd),
                }
            })
            .collect()
    }

    /// The camp a hero's sends go to now (its name).
    pub fn siege_target_name(&self, id: Id) -> Option<String> {
        let d = self.siege_def()?;
        let s = self.siege.as_ref()?;
        let t = *s.target.get(&id)?;
        Some(d.bases[s.camps.get(t)?.def].name.clone())
    }

    /// The Battle Trainer's cards for a hero: their class's they don't have yet, cheapest first,
    /// each (key, name, price, can afford, text, icon).
    pub fn siege_trainer(&self, id: Id) -> Vec<(String, String, u32, bool, String, String)> {
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        let gold = self.gold_of(id);
        let mut out: Vec<(String, String, u32, bool, String, String)> = self
            .content
            .abilities
            .iter()
            .filter(|a| a.on_board() && a.school == p.class && !p.owned.contains(&a.key))
            .filter_map(|a| {
                let price = self.card_price(&a.key)?;
                Some((a.key.clone(), a.name.clone(), price, gold >= price, a.text.clone(), a.icon.clone()))
            })
            .collect();
        out.sort_by_key(|c| c.2);
        out
    }

    /// A card bought in a match goes on its key at once if that key is free.
    pub(crate) fn siege_slot_bought(&mut self, id: Id, key: &str) {
        let Some(a) = self.content.ability(key) else { return };
        let slot = self.content.abilities[a].slot.clone();
        let free = self.players.get(&id).is_some_and(|p| !p.loadout.values().any(|k| self.content.ability(k).is_some_and(|b| self.content.abilities[b].slot == slot)));
        if free {
            let _ = self.slot_ability(id, &slot, key);
        }
    }

    /// What a board card costs here (in a match), by its row (a beta card: none, it isn't sold).
    pub(crate) fn card_price(&self, key: &str) -> Option<u32> {
        let d = self.siege_def()?;
        let a = self.content.ability(key).filter(|a| !self.content.abilities[*a].is_beta())?;
        d.cards.get(&self.content.abilities[a].slot).copied()
    }

    /// Who must be ready for a wave to come early: every hero of a camp still standing.
    fn ready_needed(&self) -> Vec<Id> {
        let Some(s) = self.siege.as_ref() else { return Vec::new() };
        s.camps.iter().filter(|c| !c.out).flat_map(|c| c.heroes.iter().copied()).filter(|h| self.players.contains_key(h)).collect()
    }

    /// Whether `obj` is a camp's Hearth in the match.
    pub(crate) fn is_siege_hearth(&self, obj: Id) -> bool {
        self.siege.as_ref().is_some_and(|s| s.phase == SiegePhase::Running && s.camps.iter().any(|c| c.hearth == obj))
    }

    /// Ready (or not) for the next wave: once every hero still in it is, it comes in
    /// `ready_secs`.
    pub fn siege_ready(&mut self, id: Id) -> Result<String, String> {
        let d = self.siege_def().cloned().ok_or("There's no Siege here.")?;
        let mine = self.camp_of(id).ok_or("Only in a match.")?;
        let s = self.siege.as_mut().ok_or("There's no Siege here.")?;
        if s.phase != SiegePhase::Running {
            return Err("Only while a match is under way.".into());
        }
        if s.camps[mine].out {
            return Err("Your Hearth has fallen.".into());
        }
        let now_ready = if s.ready.remove(&id) {
            false
        } else {
            s.ready.insert(id);
            true
        };
        let need = self.ready_needed();
        let s = self.siege.as_mut().unwrap();
        let have = need.iter().filter(|h| s.ready.contains(h)).count();
        let next = s.wave + 1;
        let said = if !now_ready {
            format!("Not ready after all ({have} of {} ready).", need.len())
        } else if have == need.len() && s.timer > d.ready_secs {
            s.timer = d.ready_secs;
            let name = self.players.get(&id).map_or(String::new(), |p| p.name.clone());
            for h in &need {
                self.events.push(Event::Banner { player: Some(*h), title: "Everyone Ready".into(), sub: format!("Wave {next} in {} s", d.ready_secs as u32) });
                if *h != id {
                    self.tell(*h, format!("{name} is ready too: wave {next} comes in {} seconds.", d.ready_secs as u32));
                }
            }
            format!("Everyone's ready: wave {next} comes in {} seconds.", d.ready_secs as u32)
        } else if have == need.len() {
            format!("Ready: wave {next} is on its way.")
        } else {
            let name = self.players.get(&id).map_or(String::new(), |p| p.name.clone());
            for h in need.iter().filter(|h| **h != id) {
                self.tell(*h, format!("{name} is ready for wave {next} ({have} of {}): F at your Hearth when you are.", need.len()));
            }
            format!("Ready for wave {next} ({have} of {}): it comes early once everyone is. F again to take it back.", need.len())
        };
        self.siege_news();
        Ok(said)
    }

    /// Which way a base lies from the middle of the basin, in words.
    fn base_side(&self, at: [f32; 3]) -> &'static str {
        let mid = self.siege_def().map_or(at, |d| d.middle);
        compass(facing(mid, at))
    }

    // --- views ----------------------------------------------------------------------------------

    /// The hall's banner: each way to play, its seats and who's waiting.
    pub fn siege_modes(&self, id: Id) -> Vec<ModeView> {
        let (Some(d), Some(s)) = (self.siege_def(), self.siege.as_ref()) else { return Vec::new() };
        d.modes
            .iter()
            .enumerate()
            .map(|(m, md)| ModeView {
                key: md.key.clone(),
                name: md.name.clone(),
                text: md.text.clone(),
                seats: md.bases.len() * md.per_base,
                heroes: s.queues[m].iter().filter_map(|h| self.players.get(h).map(|p| p.name.clone())).collect(),
                mine: s.queues[m].contains(&id),
            })
            .collect()
    }

    /// How things stand: the match's phase in words, the way under way, seconds.
    pub fn siege_status(&self) -> (String, f32) {
        let (Some(d), Some(s)) = (self.siege_def(), self.siege.as_ref()) else { return (String::new(), 0.0) };
        let mode = s.mode.map_or(String::new(), |m| d.modes[m].name.clone());
        match s.phase {
            SiegePhase::Lobby => ("No match under way".into(), 0.0),
            SiegePhase::Countdown { left } => (format!("{mode} begins"), left),
            SiegePhase::Running => (format!("{mode}: wave {} of {}", s.wave, d.waves), s.timer.max(0.0)),
            SiegePhase::Over { left, .. } => (format!("{mode} is over"), left),
        }
    }

    /// The match as a hero sees it (none out of one, or before it).
    pub fn siege_hud(&self, id: Id) -> Option<SiegeHud> {
        let d = self.siege_def()?;
        let s = self.siege.as_ref()?;
        let mine = self.camp_of(id);
        let (phase, secs) = match s.phase {
            SiegePhase::Lobby => return None,
            SiegePhase::Countdown { left } => {
                if !s.mode.is_some_and(|m| s.queues[m].contains(&id)) {
                    return None;
                }
                ("countdown", left)
            }
            SiegePhase::Running => {
                mine?;
                ("running", s.timer.max(0.0))
            }
            SiegePhase::Over { left, .. } => {
                mine?;
                ("over", left)
            }
        };
        let target = s.target.get(&id).copied();
        let camps = s
            .camps
            .iter()
            .enumerate()
            .map(|(k, c)| {
                let (hearth, max) = match self.objs.get(&c.hearth).map(|o| &o.kind) {
                    Some(ObjKind::Station { def, health, damaged, .. }) => (if *damaged { 0 } else { *health }, self.content.stations[*def].health),
                    _ => (0, 0),
                };
                CampView {
                    name: d.bases[c.def].name.clone(),
                    heroes: c.heroes.iter().filter_map(|h| self.players.get(h).map(|p| p.name.clone())).collect(),
                    hearth,
                    max,
                    out: c.out,
                    mine: mine == Some(k),
                    target: target == Some(k),
                }
            })
            .collect();
        let coming = match mine {
            Some(k) if s.warned => self.coming_list(&s.camps[k]),
            _ => Vec::new(),
        };
        let result = match s.phase {
            SiegePhase::Over { winner: Some(w), .. } if Some(w) == mine => "Victory".into(),
            SiegePhase::Over { winner: Some(_), .. } => "Defeat".into(),
            SiegePhase::Over { winner: None, .. } => "A draw".into(),
            _ => String::new(),
        };
        let need = self.ready_needed();
        let ready = (need.iter().filter(|h| s.ready.contains(h)).count(), need.len(), s.ready.contains(&id));
        let cap = d.send_cap.saturating_add(d.send_cap_growth.saturating_mul(s.wave));
        Some(SiegeHud {
            wave: s.wave,
            waves: d.waves,
            secs,
            phase,
            gold: s.gold.get(&id).copied().unwrap_or(0),
            income: s.income.get(&id).copied().unwrap_or(0),
            camps,
            coming,
            result,
            ready,
            sent: (s.sent.get(&id).copied().unwrap_or(0), cap),
        })
    }

    /// Everyone in Siege's world hears that something changed.
    pub(crate) fn siege_news(&mut self) {
        let heroes: Vec<Id> = self.players.keys().copied().collect();
        for h in heroes {
            self.events.push(Event::Siege { player: h });
        }
    }
}

/// A wave comes in groups of this many, this many seconds apart.
const TRICKLE_GROUP: usize = 3;
const TRICKLE_SECS: f32 = 6.0;
/// A gate's mark on the ground (yards round).
const GATE_MARK: f32 = 3.0;
/// A Hearth stands with this much open ground round its middle.
const HEARTH_CLEAR: f32 = 4.0;
/// The book's page key for Siege.
pub const SIEGE_PAGE: &str = "siege";
/// A creature in the match turns on a hero this near it.
const SIEGE_SIGHT: f32 = 14.0;
/// A defence counts this much nearer than it is, and draws creatures no farther than this.
const SIEGE_PULL: f32 = 10.0;
const SIEGE_DEFENSE_REACH: f32 = 30.0;
/// The Hall's banner's look, and the ledger's (a book on a crate, as at camp).
pub const SIEGE_BANNER_DISPLAY: u32 = arena::ARENA_BANNER_DISPLAY;

fn upper(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
