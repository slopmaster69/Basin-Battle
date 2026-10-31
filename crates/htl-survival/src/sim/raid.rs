//! The settlement attack: rung for at the alarm bell once the camp has its stations, warned and
//! marked, a few raiders at a time from one road; won by killing them all, lost (and recovered
//! from) if every hero is down for a while. With nobody online it waits.

use super::mobs::{Foe, MobState};
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RaidPhase {
    #[default]
    Idle,
    /// Rung: raiders on the way.
    Warning,
    Active,
}

#[derive(Debug, Clone, Default)]
pub struct Raid {
    pub phase: RaidPhase,
    /// Seconds left of the warning, or until the next raiders.
    pub timer: f32,
    /// Raiders still to come, by creature.
    pub queue: Vec<usize>,
    pub raiders: Vec<Id>,
    /// How long every hero has been down.
    pub rout: f32,
    /// Waiting for a hero to come back online.
    pub suspended: bool,
    /// Heroes asked to confirm ringing the bell.
    pub(crate) asked: Vec<Id>,
    /// Danger marks along the approach.
    pub(crate) marks: Vec<Id>,
    /// Which raid it is (how many had been beaten when it was called).
    pub won_before: u32,
    /// The spikes' second.
    pub(crate) spike_t: f32,
    /// When each tower shoots next.
    pub(crate) tower_at: BTreeMap<Id, f32>,
}

/// A hero coming back to a waiting raid gets this long before it goes on.
const RESUME_GRACE: f32 = 10.0;
/// Raiders turn on a hero this near them rather than the station they were after.
const RAIDER_SIGHT: f32 = 12.0;
/// How high up a tower its archers shoot from.
const TOWER_EYE: f32 = 6.0;
/// A defence counts this much nearer to a raider than it is.
const DEFENSE_PULL: f32 = 10.0;

impl Raid {
    pub fn active(&self) -> bool {
        matches!(self.phase, RaidPhase::Warning | RaidPhase::Active)
    }

    pub(crate) fn raiders_near(&self, s: &Survival, station: Id) -> bool {
        let Some(at) = s.objs.get(&station).map(|o| o.pos) else { return false };
        self.raiders.iter().any(|r| s.mobs.get(r).is_some_and(|m| m.health > 0 && dist2d(m.pos, at) < 6.0))
    }
}

impl Survival {
    /// The bell: ask before calling the raid.
    pub(crate) fn ring_bell(&mut self, id: Id) {
        match self.raid.phase {
            RaidPhase::Warning | RaidPhase::Active => {
                let left = self.raid.queue.len() + self.raid.raiders.iter().filter(|r| self.mobs.get(r).is_some_and(|m| m.health > 0)).count();
                self.tell(id, format!("The camp is under attack: {left} raiders left."));
            }
            RaidPhase::Idle => {
                let plan = self.next_raid();
                if !self.camp_ready() {
                    let need: Vec<String> = plan
                        .needs
                        .iter()
                        .filter(|k| !self.has_station(k))
                        .map(|k| self.content.stations[self.content.station(k).unwrap()].name.clone())
                        .collect();
                    self.tell(id, format!("The bell calls the {} down on the camp. Build a {} first.", plan.title, need.join(" and a ")));
                    return;
                }
                if !self.raid.asked.contains(&id) {
                    self.raid.asked.push(id);
                }
                let who = self.raiders_text(&plan.raiders);
                self.events.push(Event::Confirm {
                    player: id,
                    what: "raid".into(),
                    text: format!(
                        "Ring the bell for the {}? {who} will march on the camp from {} in {} seconds. They'll wreck the stations they reach.",
                        plan.title, plan.road, plan.warning as u32
                    ),
                });
            }
        }
    }

    /// Confirmed: the raid is called.
    pub fn raid_start(&mut self, id: Id) -> Result<(), String> {
        if !self.raid.asked.contains(&id) {
            return Err("Ring the bell at the camp first.".into());
        }
        if self.raid.phase != RaidPhase::Idle {
            return Err("The raid's already coming.".into());
        }
        if !self.camp_ready() {
            return Err("The camp isn't ready.".into());
        }
        self.raid.asked.clear();
        let r = self.next_raid();
        let mut queue = Vec::new();
        for (mob, count) in &r.raiders {
            let def = self.content.mob(mob).unwrap();
            for _ in 0..*count {
                queue.push(def);
            }
        }
        // Mixed, so each wave isn't all of one kind.
        for i in (1..queue.len()).rev() {
            let j = self.rng.gen_range(0..=i);
            queue.swap(i, j);
        }
        // A champion comes with the last of them.
        let content = &self.content;
        queue.sort_by_key(|d| content.mobs[*d].boss);
        self.raid.won_before = self.camp.raids_won;
        self.raid.phase = RaidPhase::Warning;
        self.raid.timer = r.warning;
        self.raid.queue = queue;
        self.raid.raiders.clear();
        self.raid.rout = 0.0;
        self.raid.suspended = false;
        // Mark the way they'll come.
        let from = self.ground(r.from[0], r.from[1]);
        let heart = self.ground(self.content.world.heart[0], self.content.world.heart[1]);
        let mut marks = vec![self.mark(from, 6.0, r.warning + 5.0)];
        for k in 1..4 {
            let t = k as f32 / 4.0;
            let at = self.ground(from[0] + (heart[0] - from[0]) * t, from[1] + (heart[1] - from[1]) * t);
            marks.push(self.mark(at, 2.5, r.warning + 5.0));
        }
        self.raid.marks = marks;
        let name = self.players.get(&id).map_or("Someone".to_string(), |p| p.name.clone());
        let road = r.road.clone();
        self.events.push(Event::Banner { player: None, title: r.title.clone(), sub: format!("From {road} in {} seconds", r.warning as u32) });
        self.announce(format!("{name} rings the alarm bell! The {} marches on the camp from {road}: {} seconds.", r.title, r.warning as u32));
        self.events.push(Event::Dirty);
        Ok(())
    }

    pub fn raid_decline(&mut self, id: Id) {
        self.raid.asked.retain(|a| *a != id);
    }

    /// For tests and development: call off any raid and clear its raiders.
    pub fn raid_reset(&mut self) {
        for r in std::mem::take(&mut self.raid.raiders) {
            if self.mobs.remove(&r).is_some() {
                self.events.push(Event::MobGone { id: r });
            }
        }
        for m in std::mem::take(&mut self.raid.marks) {
            self.unmark(m);
        }
        self.raid = raid::Raid::default();
        self.events.push(Event::Dirty);
    }

    pub(crate) fn step_raid(&mut self, dt: f32) {
        if !self.raid.active() {
            return;
        }
        if self.players.is_empty() {
            if !self.raid.suspended {
                self.raid.suspended = true;
                self.events.push(Event::Log("raid suspended: nobody online".into()));
                self.events.push(Event::Dirty);
            }
            return;
        }
        if self.raid.suspended {
            self.raid.suspended = false;
            self.raid.timer = self.raid.timer.max(RESUME_GRACE);
            let title = self.current_raid().title;
            self.events.push(Event::Banner { player: None, title: "The Raid Resumes".into(), sub: format!("The {title} at the camp in {} seconds", RESUME_GRACE as u32) });
            self.announce(format!("The {title} goes on in {} seconds.", RESUME_GRACE as u32));
            if self.raid.phase == RaidPhase::Active && !self.raid.raiders.is_empty() {
                // Raiders that were here stand down for the grace, then come on.
                self.raid.queue.extend(std::mem::take(&mut self.raid.raiders).into_iter().filter_map(|r| {
                    let m = self.mobs.remove(&r)?;
                    self.events.push(Event::MobGone { id: r });
                    (m.health > 0).then_some(m.def)
                }));
                // The champion still last.
                let content = &self.content;
                self.raid.queue.sort_by_key(|d| content.mobs[*d].boss);
                self.raid.phase = RaidPhase::Warning;
            }
            return;
        }
        let r = self.content.raid.clone();
        let plan = self.current_raid();
        self.raid.timer -= dt;
        if self.raid.phase == RaidPhase::Warning {
            if self.raid.timer <= 0.0 {
                self.raid.phase = RaidPhase::Active;
                self.raid.timer = 0.0;
                for m in std::mem::take(&mut self.raid.marks) {
                    self.unmark(m);
                }
                self.announce(format!("The {} is here!", plan.title));
            }
            return;
        }
        // Spikes bite the raiders at them, once a second.
        self.raid.spike_t += dt;
        if self.raid.spike_t >= 1.0 {
            self.raid.spike_t -= 1.0;
            let spikes: Vec<(Vec3, f32, u32)> = self
                .objs
                .values()
                .filter_map(|o| match o.kind {
                    ObjKind::Station { def, damaged: false, .. } if self.content.stations[def].spikes > 0 => {
                        let d = &self.content.stations[def];
                        Some((o.pos, d.footprint + 1.2, d.spikes))
                    }
                    _ => None,
                })
                .collect();
            for (at, reach, dmg) in spikes {
                let hit: Vec<Id> = self.raid.raiders.iter().copied().filter(|r| self.mobs.get(r).is_some_and(|m| m.health > 0 && dist2d(m.pos, at) <= reach)).collect();
                for r in hit {
                    self.events.push(Event::React { id: r, anim: combat::WOUND_ANIM, from: at, lean: 0.15, hold: 0.0, fit: 0.0 });
                    self.hurt_mob(r, 0, dmg);
                }
            }
        }
        self.towers_shoot();
        // Raiders come in a few at a time.
        if self.raid.timer <= 0.0 && !self.raid.queue.is_empty() {
            for k in 0..plan.wave_size.min(self.raid.queue.len()) {
                // A champion comes on his own, and says so.
                let boss = self.content.mobs[self.raid.queue[0]].boss;
                if boss && k > 0 {
                    break;
                }
                let def = self.raid.queue.remove(0);
                let a = k as f32 * 2.1;
                let at = self.ground(plan.from[0] + a.cos() * 2.0, plan.from[1] + a.sin() * 2.0);
                let id = self.spawn_mob(def, at, None, true);
                self.raid.raiders.push(id);
                self.raider_next(id);
                if boss {
                    self.events.push(Event::Emote { id, emote: super::mobs::ROAR_EMOTE });
                    let d = &self.content.mobs[def];
                    let (name, sub) = (d.name.clone(), d.subname.clone());
                    self.events.push(Event::Banner { player: None, title: name.clone(), sub: if sub.is_empty() { format!("From {}", plan.road) } else { format!("{sub}, from {}", plan.road) } });
                    self.announce(format!("{name} comes from {} with the last of them!", plan.road));
                    break;
                }
            }
            self.raid.timer = r.wave_gap;
        }
        // Every hero down: the raiders win.
        let all_down = self.players.values().all(|p| p.health == 0);
        if all_down {
            self.raid.rout += dt;
            if self.raid.rout >= r.rout_secs {
                self.raid_lost();
            }
        } else {
            self.raid.rout = 0.0;
        }
    }

    /// Each standing tower shoots the nearest raider it can see in reach, every so often.
    fn towers_shoot(&mut self) {
        let now = self.clock;
        let towers: Vec<(Id, Vec3, crate::content::ShootDef)> = self
            .objs
            .iter()
            .filter_map(|(id, o)| match o.kind {
                ObjKind::Station { def, damaged: false, .. } => self.content.stations[def].shoots.clone().map(|sh| (*id, o.pos, sh)),
                _ => None,
            })
            .collect();
        let spell = self.content.ranged.first().map_or(0, |r| r.spell);
        for (t, at, sh) in towers {
            if self.raid.tower_at.get(&t).is_some_and(|n| now < *n) {
                continue;
            }
            // From up on the tower: over the low things in the way.
            let eye = [at[0], at[1], at[2] + TOWER_EYE];
            let target = self
                .raid
                .raiders
                .iter()
                .filter_map(|r| self.mobs.get(r).filter(|m| m.health > 0).map(|m| (*r, m.pos)))
                // Up on the tower they shoot over the trees and walls: only the hills hide a raider.
                .filter(|(_, p)| dist2d(*p, at) <= sh.range && self.over_the_hills(eye, [p[0], p[1], p[2] + 1.0]))
                .min_by(|a, b| dist2d(a.1, at).total_cmp(&dist2d(b.1, at)));
            let Some((r, _)) = target else { continue };
            self.raid.tower_at.insert(t, now + sh.every);
            self.events.push(Event::Missile { caster: t, spell, target: Target::Unit(r) });
            self.events.push(Event::React { id: r, anim: combat::WOUND_ANIM, from: at, lean: 0.15, hold: 0.0, fit: 0.0 });
            self.hurt_mob(r, 0, sh.damage);
        }
    }

    /// Whether the ground doesn't rise above the line from `a` to `b`.
    fn over_the_hills(&self, a: Vec3, b: Vec3) -> bool {
        let d = dist2d(a, b);
        let steps = (d / 1.0).ceil().max(1.0) as usize;
        (1..steps).all(|k| {
            let t = k as f32 / steps as f32;
            let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
            self.land.height(x, y) <= a[2] + (b[2] - a[2]) * t
        })
    }

    /// A raider looks for what to fight: a hero near it, else the nearest standing station, else
    /// the camp's heart.
    pub(crate) fn raider_next(&mut self, id: Id) {
        if self.is_arena_foe(id) {
            self.arena_next(id);
            return;
        }
        if self.is_siege() {
            self.siege_next(id);
            return;
        }
        let Some(m) = self.mobs.get(&id) else { return };
        let pos = m.pos;
        let hero = self
            .players
            .iter()
            .filter(|(_, p)| p.health > 0 && dist2d(p.pos, pos) <= RAIDER_SIGHT)
            .min_by(|a, b| dist2d(a.1.pos, pos).total_cmp(&dist2d(b.1.pos, pos)))
            .map(|(id, _)| *id);
        let foe = match hero {
            Some(h) => Some(Foe::Hero(h)),
            // The nearest standing station, a defence counted nearer still: they break through
            // the palisade before they reach what it guards.
            None => self
                .objs
                .iter()
                .filter_map(|(id, o)| match o.kind {
                    ObjKind::Station { def, damaged: false, .. } => {
                        let near = if self.content.stations[def].defense { DEFENSE_PULL } else { 0.0 };
                        Some((*id, dist2d(o.pos, pos) - near))
                    }
                    _ => None,
                })
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(id, _)| Foe::Obj(id))
                .or_else(|| {
                    self.players
                        .iter()
                        .filter(|(_, p)| p.health > 0)
                        .min_by(|a, b| dist2d(a.1.pos, pos).total_cmp(&dist2d(b.1.pos, pos)))
                        .map(|(id, _)| Foe::Hero(*id))
                }),
        };
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = match foe {
            Some(f) => MobState::Chase { foe: f },
            None => MobState::Idle { wander_at: self.clock + 2.0 },
        };
        m.think = 0.0;
    }

    pub(crate) fn raider_down(&mut self) {
        if self.raid.phase != RaidPhase::Active {
            return;
        }
        let standing = self.raid.raiders.iter().any(|r| self.mobs.get(r).is_some_and(|m| m.health > 0));
        if standing || !self.raid.queue.is_empty() {
            return;
        }
        let plan = self.current_raid();
        let reward = plan.reward.clone();
        for (k, n) in &reward {
            let i = self.content.item(k).unwrap();
            *self.storage.entry(i).or_default() += n;
        }
        self.raid = Raid::default();
        self.camp.raids_won += 1;
        self.count("raid", &format!("{}", plan.tier), 1);
        self.events.push(Event::Storage);
        self.events.push(Event::Banner { player: None, title: "The Camp Holds".into(), sub: format!("Spoils in the chest: {}", self.cost_text(&reward)) });
        let next = self.next_raid();
        let after = if next.tier > plan.tier {
            let need: Vec<String> = next.needs.iter().map(|k| self.content.stations[self.content.station(k).unwrap()].name.clone()).collect();
            format!(" Word is the {} will come next: the bell calls them once the camp has a {}.", next.title, need.join(" and a "))
        } else {
            format!(" They'll be back, more of them: the bell calls the {} when you're ready.", next.title)
        };
        self.announce(format!("The {} is beaten! The spoils go to the chest: {}.{after}", plan.title, self.cost_text(&reward)));
        self.events.push(Event::Dirty);
    }

    fn raid_lost(&mut self) {
        for r in std::mem::take(&mut self.raid.raiders) {
            if self.mobs.remove(&r).is_some() {
                self.events.push(Event::MobGone { id: r });
            }
        }
        self.raid = Raid::default();
        self.events.push(Event::Banner { player: None, title: "The Camp Falls".into(), sub: "The raiders burn what they reached and go".into() });
        self.announce("The raiders burn what they reached and go. Mend the wrecked stations with timber; the chest is untouched.");
        self.events.push(Event::Dirty);
    }
}

impl Survival {
    /// The raid the bell calls next.
    pub fn next_raid(&self) -> crate::content::RaidPlan {
        self.capped(self.content.raid.plan(self.camp.raids_won))
    }

    /// Never more than one of a champion, however often the raid has come back.
    fn capped(&self, mut plan: crate::content::RaidPlan) -> crate::content::RaidPlan {
        for (k, n) in plan.raiders.iter_mut() {
            if self.content.mob(k).is_some_and(|m| self.content.mobs[m].boss) {
                *n = (*n).min(1);
            }
        }
        plan
    }

    /// The raid under way (or, with none, the next).
    pub fn current_raid(&self) -> crate::content::RaidPlan {
        if self.raid.active() {
            self.capped(self.content.raid.plan(self.raid.won_before))
        } else {
            self.next_raid()
        }
    }

    /// "4 Fargodeep Tunnelers, a Fargodeep Geomancer and a Taskmaster".
    pub(crate) fn raiders_text(&self, raiders: &[(String, usize)]) -> String {
        let words: Vec<String> = raiders
            .iter()
            .filter(|(_, n)| *n > 0)
            .map(|(k, n)| {
                let name = self.content.mob(k).map_or(k.clone(), |m| self.content.mobs[m].name.clone());
                if self.content.mob(k).is_some_and(|m| self.content.mobs[m].boss) {
                    name
                } else if *n == 1 {
                    format!("a {name}")
                } else {
                    format!("{n} {name}s")
                }
            })
            .collect();
        match words.len() {
            0 => "nobody".into(),
            1 => words[0].clone(),
            n => format!("{} and {}", words[..n - 1].join(", "), words[n - 1]),
        }
    }
}
