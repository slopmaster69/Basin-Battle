//! The Proving Ring: a page of the book at camp that takes heroes to a ring of braziers out in the
//! land, where foes come in waves that never end. Each wave is worth more than the last (in its
//! foes' cost, more again for each hero past the first) up to a few at once, past which they only
//! grow tougher; a champion comes every few waves with half a wave beside him. A wave beaten
//! gives everyone in the ring a breather and some health back; a champion's wave pays into the
//! camp's chest. A hero's run ends when they fall (nothing they carry is lost there: they get up
//! at the camp), leave by the banner, or walk out of the ring; when the last is gone the ring
//! clears. The camp keeps its best wave.

use super::mobs::{Foe, MobState};
use super::*;
use crate::content::ArenaDef;

/// The ring as it is: who's fighting in it, the wave, and its foes.
#[derive(Debug, Clone, Default)]
pub struct Arena {
    pub heroes: BTreeSet<Id>,
    /// The wave under way, or the last beaten (0: none yet).
    pub wave: u32,
    /// A wave is under way.
    pub fighting: bool,
    /// Seconds until the next wave, between waves.
    pub timer: f32,
    /// This wave's foes (the fallen among them too, until they're gone).
    pub foes: Vec<Id>,
}

/// Past the ring's edge by this much, a hero has walked out.
const STRAY: f32 = 6.0;
/// Foes come in this far out from the middle (a share of the ring's reach).
const ENTRY: f32 = 0.9;
/// Heroes come in this far out.
const ARRIVAL: f32 = 0.7;
/// The banner (the way out) stands this far out beyond the arrival, the braziers round the edge
/// this far apart.
const BANNER_BACK: f32 = 3.0;
const BRAZIER_EVERY: f32 = 9.0;
/// The ring's looks (`GameObjectDisplayInfo`): Elwynn's battle-glade banner, a tall brazier.
pub const ARENA_BANNER_DISPLAY: u32 = 4313;
pub const ARENA_BRAZIER_DISPLAY: u32 = 655;
/// The camp's best wave, kept with its counts.
pub(crate) const BEST_KEY: &str = "arena:best";

/// Why a hero's run ends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Exit {
    /// Home by the banner.
    Home,
    Fell,
    Strayed,
    /// Logged out.
    Gone,
}

impl Survival {
    pub(crate) fn arena_def(&self) -> Option<&ArenaDef> {
        self.content.arena.as_ref()
    }

    /// The camp's best wave in the ring.
    pub fn arena_best(&self) -> u32 {
        self.camp.counts.get(BEST_KEY).copied().unwrap_or(0)
    }

    /// The book's page for the ring: its key, its name, the camp's best wave.
    pub fn arena_page(&self) -> Option<(String, String, u32)> {
        let a = self.arena_def()?;
        Some(("arena".into(), a.page.clone(), self.arena_best()))
    }

    pub fn in_arena(&self, id: Id) -> bool {
        self.arena.heroes.contains(&id)
    }

    fn arena_center(&self) -> Option<Vec3> {
        let a = self.arena_def()?;
        Some(self.ground(a.at[0], a.at[1]))
    }

    /// Whether a spot is in the ring (or near enough its edge).
    pub(crate) fn within_arena(&self, pos: Vec3) -> bool {
        self.arena_def().is_some_and(|a| dist2d(pos, [a.at[0], a.at[1], 0.0]) <= a.radius + STRAY)
    }

    /// The ring's braziers round its edge and its banner, at camp.
    pub(crate) fn place_arena(&mut self) {
        let Some(a) = self.arena_def().cloned() else { return };
        let round = std::f32::consts::TAU * a.radius;
        let n = ((round / BRAZIER_EVERY).round() as usize).max(4);
        for k in 0..n {
            let ang = a.facing + std::f32::consts::PI + (k as f32 + 0.5) / n as f32 * std::f32::consts::TAU;
            let at = self.ground(a.at[0] + ang.cos() * (a.radius + 0.5), a.at[1] + ang.sin() * (a.radius + 0.5));
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Prop { display: ARENA_BRAZIER_DISPLAY, scale: 1.0 }, pos: at, facing: ang });
        }
        let back = a.facing + std::f32::consts::PI;
        let r = a.radius * ARRIVAL + BANNER_BACK;
        let at = self.ground(a.at[0] + back.cos() * r, a.at[1] + back.sin() * r);
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::ArenaBanner, pos: at, facing: a.facing });
    }

    /// The book's page: into the ring (joining the run under way, if there is one).
    pub(crate) fn enter_arena(&mut self, id: Id) -> Result<(), String> {
        let a = self.arena_def().cloned().ok_or("There's no such page.")?;
        if self.in_arena(id) {
            return Err(format!("You're in {} already.", a.name));
        }
        let back = a.facing + std::f32::consts::PI;
        let spread = (self.arena.heroes.len() as f32) * 1.5;
        let side = a.facing + std::f32::consts::FRAC_PI_2;
        let r = a.radius * ARRIVAL;
        let (x, y) = (a.at[0] + back.cos() * r + side.cos() * spread, a.at[1] + back.sin() * r + side.sin() * spread);
        let at = self.landing_spot(self.ground(x, y));
        self.cancel_preview(id);
        self.put_at(id, at, a.facing);
        self.land_from_far(id);
        let fresh = self.arena.heroes.is_empty();
        self.arena.heroes.insert(id);
        let name = self.players[&id].name.clone();
        if fresh {
            self.arena = Arena { heroes: self.arena.heroes.clone(), timer: a.first_gap, ..Arena::default() };
            let best = self.arena_best();
            let record = if best > 0 { format!("; the camp's best is wave {best}") } else { String::new() };
            self.events.push(Event::Banner {
                player: Some(id),
                title: a.page.clone(),
                sub: format!("The first wave in {} seconds{record}", a.first_gap as u32),
            });
            self.tell(id, format!("{}: they come in waves that never end, more and tougher every time, a champion every {}th. Fall and you get up at the camp, nothing lost; the banner behind you takes you home.", a.page.clone(), a.boss_every));
        } else {
            let now = if self.arena.fighting {
                format!("Wave {} under way", self.arena.wave)
            } else {
                format!("Wave {} in {} seconds", self.arena.wave + 1, self.arena.timer.ceil() as u32)
            };
            self.events.push(Event::Banner { player: Some(id), title: a.page.clone(), sub: now });
            for h in self.arena.heroes.clone() {
                if h != id {
                    self.tell(h, format!("{name} joins you in the ring."));
                }
            }
        }
        self.events.push(Event::Log(format!("{name} enters the arena")));
        self.arena_news();
        Ok(())
    }

    /// A hero's run ends.
    pub(crate) fn leave_arena(&mut self, id: Id, how: Exit) {
        if !self.arena.heroes.remove(&id) {
            return;
        }
        let Some(a) = self.arena_def().cloned() else { return };
        let wave = self.arena.wave;
        let reached = if wave == 0 { "before the first wave".to_string() } else if self.arena.fighting { format!("in wave {wave}") } else { format!("with wave {wave} beaten") };
        let name = self.players.get(&id).map_or_else(String::new, |p| p.name.clone());
        match how {
            Exit::Home => {
                let w = self.content.world.clone();
                let at = self.landing_spot(self.ground(w.spawn[0], w.spawn[1]));
                self.put_at(id, at, w.spawn_facing);
                self.land_from_far(id);
                self.tell(id, format!("You leave {} {reached}, and make your way back to the camp.", a.name));
            }
            Exit::Fell => self.tell(id, format!("You fall in {} {reached}. Nothing you carried is lost: you'll get up at the camp.", a.name)),
            Exit::Strayed => self.tell(id, format!("You've walked out of {}: your run's over, {reached}.", a.name)),
            Exit::Gone => {}
        }
        if !name.is_empty() {
            for h in self.arena.heroes.clone() {
                let why = match how {
                    Exit::Fell => "falls",
                    _ => "leaves the ring",
                };
                self.tell(h, format!("{name} {why}."));
            }
        }
        self.events.push(Event::Arena { player: id });
        // Its foes look for whoever is left.
        for f in self.arena.foes.clone() {
            if self.mobs.get(&f).is_some_and(|m| m.health > 0 && m.state.target() == Some(id)) {
                self.arena_next(f);
            }
        }
        if self.arena.heroes.is_empty() {
            self.close_arena();
        } else {
            self.arena_news();
        }
    }

    /// The last hero gone: the ring clears.
    fn close_arena(&mut self) {
        for f in std::mem::take(&mut self.arena.foes) {
            if self.mobs.remove(&f).is_some() {
                self.events.push(Event::MobGone { id: f });
            }
        }
        if self.arena.wave > 0 {
            self.events.push(Event::Log(format!("the arena falls quiet after wave {}", self.arena.wave)));
        }
        self.arena = Arena::default();
    }

    /// Between waves, the call to bring on the next at once (the banner's).
    pub fn arena_next_now(&mut self, id: Id) -> Result<(), String> {
        if !self.in_arena(id) {
            return Err("You're not in the ring.".into());
        }
        if self.arena.fighting {
            return Err("A wave's already on you.".into());
        }
        self.arena.timer = 0.0;
        Ok(())
    }

    pub(crate) fn step_arena(&mut self, dt: f32) {
        if self.arena_def().is_none() || (self.arena.heroes.is_empty() && self.arena.foes.is_empty()) {
            return;
        }
        // Who's walked out (or gone).
        let out: Vec<(Id, Exit)> = self
            .arena
            .heroes
            .iter()
            .filter_map(|h| match self.players.get(h) {
                None => Some((*h, Exit::Gone)),
                Some(p) if p.health > 0 && !self.within_arena(p.pos) => Some((*h, Exit::Strayed)),
                _ => None,
            })
            .collect();
        for (h, how) in out {
            self.leave_arena(h, how);
        }
        if self.arena.heroes.is_empty() {
            if !self.arena.foes.is_empty() {
                self.close_arena();
            }
            return;
        }
        if self.arena.fighting {
            let standing = self.arena.foes.iter().any(|f| self.mobs.get(f).is_some_and(|m| m.health > 0));
            if !standing {
                self.wave_beaten();
            }
            return;
        }
        self.arena.timer -= dt;
        if self.arena.timer <= 0.0 {
            self.start_wave();
        }
    }

    /// Who wave `wave` brings, for `heroes` heroes: its champion first, if it has one.
    pub fn arena_wave(&mut self, wave: u32, heroes: usize) -> Vec<usize> {
        let Some(a) = self.arena_def().cloned() else { return Vec::new() };
        let mut worth = (a.budget + a.growth * (wave.saturating_sub(1)) as f32) * (1.0 + a.per_hero * heroes.saturating_sub(1) as f32);
        let mut out = Vec::new();
        if wave % a.boss_every == 0 && !a.bosses.is_empty() {
            let k = ((wave / a.boss_every - 1) as usize) % a.bosses.len();
            if let Some(b) = self.content.mob(&a.bosses[k]) {
                out.push(b);
                worth *= 0.5;
            }
        }
        let open: Vec<(usize, f32)> = a.roster.iter().filter(|f| f.from <= wave).filter_map(|f| Some((self.content.mob(&f.mob)?, f.cost))).collect();
        let cheapest = open.iter().map(|o| o.1).fold(f32::MAX, f32::min);
        while out.len() < a.most && worth >= cheapest - 1.0e-3 {
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

    fn start_wave(&mut self) {
        let Some(a) = self.arena_def().cloned() else { return };
        let Some(center) = self.arena_center() else { return };
        self.arena.wave += 1;
        let wave = self.arena.wave;
        let foes = self.arena_wave(wave, self.arena.heroes.len());
        let tough = 1.0 + a.toughen * (wave - 1) as f32;
        let n = foes.len().max(1);
        let base = self.rng.gen::<f32>() * std::f32::consts::TAU;
        self.arena.fighting = true;
        self.arena.foes.clear();
        let mut boss = None;
        for (k, def) in foes.iter().enumerate() {
            let ang = base + k as f32 / n as f32 * std::f32::consts::TAU + (self.rng.gen::<f32>() - 0.5) * 0.4;
            let r = a.radius * ENTRY;
            let at = self.ground(a.at[0] + ang.cos() * r, a.at[1] + ang.sin() * r);
            let id = self.spawn_mob(*def, at, None, true);
            let max = ((self.base_health(*def) as f32) * tough).round() as u32;
            if let Some(m) = self.mobs.get_mut(&id) {
                m.health = max;
                m.max_health = max;
                m.facing = facing(at, center);
            }
            self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
            self.arena.foes.push(id);
            self.arena_next(id);
            if self.content.mobs[*def].boss {
                boss = Some((id, *def));
            }
        }
        let mut count: Vec<(String, usize)> = Vec::new();
        for d in &foes {
            let key = self.content.mobs[*d].key.clone();
            match count.iter_mut().find(|c| c.0 == key) {
                Some(c) => c.1 += 1,
                None => count.push((key, 1)),
            }
        }
        let who = self.raiders_text(&count);
        let (title, sub) = match boss {
            Some((id, def)) => {
                self.events.push(Event::Emote { id, emote: mobs::ROAR_EMOTE });
                (self.content.mobs[def].name.clone(), format!("Wave {wave}: {who}"))
            }
            None => (format!("Wave {wave}"), who.clone()),
        };
        for h in self.arena.heroes.clone() {
            self.events.push(Event::Banner { player: Some(h), title: title.clone(), sub: upper_first(&sub) });
        }
        self.events.push(Event::Log(format!("arena wave {wave}: {who}")));
        self.arena_news();
    }

    fn wave_beaten(&mut self) {
        let Some(a) = self.arena_def().cloned() else { return };
        let wave = self.arena.wave;
        self.arena.fighting = false;
        self.arena.timer = a.gap;
        self.arena.foes.retain(|f| self.mobs.get(f).is_some());
        // A breather: some health back.
        for h in self.arena.heroes.clone() {
            let Some(p) = self.players.get_mut(&h) else { continue };
            if p.health == 0 {
                continue;
            }
            let back = ((p.max_health as f32 * a.mend).round() as u32).min(p.max_health - p.health);
            p.health += back;
            let (hp, mx) = (p.health, p.max_health);
            if back > 0 {
                self.events.push(Event::Health { id: h, health: hp, max: mx });
                self.events.push(Event::Healed { id: h, amount: back });
            }
        }
        let champion = wave % a.boss_every == 0;
        let mut paid = String::new();
        if champion && !a.reward.is_empty() {
            for (k, n) in &a.reward {
                if let Some(i) = self.content.item(k) {
                    *self.storage.entry(i).or_default() += n;
                }
            }
            self.events.push(Event::Storage);
            paid = format!("; into the camp's chest: {}", self.cost_text(&a.reward));
        }
        for h in self.arena.heroes.clone() {
            self.events.push(Event::Banner {
                player: Some(h),
                title: format!("Wave {wave} Beaten"),
                sub: format!("Wave {} in {} seconds{paid}", wave + 1, a.gap as u32),
            });
        }
        self.count("arena", "wave", 1);
        if wave > self.arena_best() {
            self.camp.counts.insert(BEST_KEY.to_string(), wave);
            // A new best is the camp's news (from the fifth on: the first few are a warm-up).
            if wave >= a.boss_every {
                let names: Vec<String> = self.arena.heroes.iter().filter_map(|h| self.players.get(h).map(|p| p.name.clone())).collect();
                let who = match names.len() {
                    0 => "Someone".to_string(),
                    1 => names[0].clone(),
                    n => format!("{} and {}", names[..n - 1].join(", "), names[n - 1]),
                };
                self.announce(format!("{who} beat wave {wave} in {}: the camp's best yet.", a.name));
            }
        }
        self.events.push(Event::Dirty);
        self.arena_news();
    }

    /// An arena foe looks for whom to fight: the nearest hero in the ring standing.
    pub(crate) fn arena_next(&mut self, id: Id) {
        let Some(m) = self.mobs.get(&id) else { return };
        let pos = m.pos;
        let hero = self
            .arena
            .heroes
            .iter()
            .filter_map(|h| self.players.get(h).filter(|p| p.health > 0).map(|p| (*h, dist2d(p.pos, pos))))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(h, _)| h);
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = match hero {
            Some(h) => MobState::Chase { foe: Foe::Hero(h) },
            None => MobState::Idle { wander_at: self.clock + 2.0 },
        };
        m.think = 0.0;
    }

    pub(crate) fn is_arena_foe(&self, id: Id) -> bool {
        self.arena.foes.contains(&id)
    }

    /// Every hero in the ring hears how it stands (the add-on's line).
    pub(crate) fn arena_news(&mut self) {
        for h in self.arena.heroes.clone() {
            self.events.push(Event::Arena { player: h });
        }
    }

    /// The ring as a hero in it sees it: the wave, foes standing, seconds to the next (between
    /// waves), the camp's best; `None` for a hero not in it.
    pub fn arena_view(&self, id: Id) -> Option<(u32, usize, Option<f32>, u32)> {
        if !self.in_arena(id) {
            return None;
        }
        let standing = self.arena.foes.iter().filter(|f| self.mobs.get(f).is_some_and(|m| m.health > 0)).count();
        let next = (!self.arena.fighting).then_some(self.arena.timer.max(0.0));
        Some((self.arena.wave, standing, next, self.arena_best()))
    }
}

fn upper_first(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}
