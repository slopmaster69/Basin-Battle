//! Gilnid's finite workforce. Platforms, throws, falls and the valve share one encounter
//! clock. Neither a health threshold nor a hero's loadout can create another worker.

use super::mobs::{Foe, MobState};
use super::*;
use crate::land::clear_line;

#[cfg(test)]
#[path = "perches_tests.rs"]
mod tests;

const HEIGHT: f32 = 2.85;
const FALL_SECS: f32 = 0.85;
const LAND_STUN: f32 = 2.4;
const FIRST_OVERFLOW: f32 = 8.0;
const DESCEND_BY: f32 = 26.0;
const MOLTEN: u32 = 5213;
const WORK_VALVE: u32 = 21651; // Opening: the goblin's native UseStandingLoop.

fn falling_point(from: Vec3, to: Vec3, t: f32) -> Vec3 {
    [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t * t]
}

#[derive(Default)]
pub(crate) struct Perches {
    crews: BTreeMap<Id, Crew>,
    workers: BTreeMap<Id, Worker>,
}

struct Crew {
    home: Vec3,
    workers: Vec<Id>,
    props: Vec<Id>,
    steam: Vec<(Id, f32)>,
    began: Option<f32>,
    next_throw: f32,
    turn: usize,
    demonstration: Option<(Id, f32)>,
    demonstrated: bool,
    first_fall: bool,
    valve: Vec3,
    stage: Stage,
}

#[derive(Clone, Copy)]
enum Stage {
    Crew,
    ToValve { since: f32 },
    Opening { until: f32 },
    Cooling { until: f32 },
    Finished,
}

struct Worker {
    owner: Id,
    support: Id,
    platform: Id,
    base: Vec3,
    landing: Vec3,
    grounded: usize,
    state: Place,
}

#[derive(Clone, Copy)]
enum Place {
    Perched,
    Falling { from: Vec3, since: f32 },
    Ground,
}

impl Survival {
    /// Encounter props must be present for the whole warning, without the normal 2 sec fade.
    /// Reuse the client's existing noncolliding, immediate-prop flag; no floor overlay needed.
    pub fn is_perch_prop(&self, id: Id) -> bool {
        self.perches.crews.values().any(|c| c.props.contains(&id) || c.steam.iter().any(|(p, _)| *p == id))
    }

    fn perch_prop(&mut self, at: Vec3, display: u32, scale: f32) -> Id {
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Prop { display, scale }, pos: at, facing: 0.0 });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    /// Stay on the boss's floor and off water. A bad content point is rejected rather than
    /// putting a worker on the upper ring, in a wall, or where melee cannot reach it.
    fn perch_floor(&self, home: Vec3, offset: [f32; 2]) -> Option<Vec3> {
        let p = self.ground_near(home[0] + offset[0], home[1] + offset[1], home[2]);
        ((p[2] - home[2]).abs() < 2.5
            && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2])
            && (self.land.walk_clear(home, p) || self.land.path(home, p).is_some_and(|v| !v.is_empty())))
            .then_some(p)
    }

    fn place_perch_crew(&mut self, owner: Id) {
        let m = &self.mobs[&owner];
        let home = m.home;
        let Some(d) = self.content.mobs[m.def].perch_crew.clone() else { return };
        let (Some(air), Some(ground)) = (self.content.mob(&d.airborne), self.content.mob(&d.grounded)) else { return };
        let mut workers = Vec::new();
        let mut props = Vec::new();
        for offset in &d.spots {
            let Some(base) = self.perch_floor(home, *offset) else {
                self.events.push(Event::Log(format!("perch crew {owner}: unreachable platform {offset:?} omitted")));
                continue;
            };
            // Land beside the support, towards the room, with a nonzero horizontal flight.
            let f = facing(base, home);
            let off = [offset[0] + f.cos() * 1.8, offset[1] + f.sin() * 1.8];
            let Some(landing) = self.perch_floor(home, off) else { continue };
            let at = [base[0], base[1], base[2] + HEIGHT];
            if self.land.slab_between(at[0], at[1], base[2] + 0.1, at[2]) { continue; }
            // Native Deadmines cargo stack (1.76 yd high at scale 1), with a plank on top.
            let support = self.perch_prop(base, 36, 1.5);
            let platform = self.perch_prop([base[0], base[1], base[2] + 2.64], 6481, 1.0);
            props.extend([support, platform]);
            let id = self.spawn_mob(air, at, None, false);
            let health = self.foe_health(air, None);
            let m = self.mobs.get_mut(&id).unwrap();
            m.health = health;
            m.max_health = health;
            m.home = landing;
            m.facing = f;
            self.perches.workers.insert(id, Worker { owner, support, platform, base, landing, grounded: ground, state: Place::Perched });
            workers.push(id);
        }
        let valve = self.perch_floor(home, d.valve).unwrap_or(home);
        props.push(self.perch_prop(valve, 755, 2.0));
        self.perches.crews.insert(owner, Crew {
            home, workers, props, steam: Vec::new(), began: None, next_throw: 0.0, turn: 0,
            demonstration: None, demonstrated: false, first_fall: false, valve, stage: Stage::Crew,
        });
    }

    /// Called before the ordinary minds. A waiting crew can be seen before the pull.
    pub(crate) fn step_perch_crews(&mut self) {
        let now = self.clock;
        let gone: Vec<_> = self.perches.crews.iter().filter(|(id, c)| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 &&
                !matches!(m.state, MobState::Return) && (c.began.is_none() || m.state.foe().is_some()))
        }).map(|(id, _)| *id).collect();
        for id in gone { self.remove_perch_crew(id); }
        for c in self.perches.crews.values_mut() {
            c.steam.retain(|(id, until)| {
                if *until > now { return true; }
                if self.objs.remove(id).is_some() { self.events.push(Event::ObjGone { id: *id }); }
                false
            });
        }
        let new: Vec<_> = self.mobs.iter().filter(|(id, m)| {
            m.health > 0 && !m.hidden && !matches!(m.state, MobState::Return)
                && self.content.mobs[m.def].perch_crew.is_some() && !self.perches.crews.contains_key(id)
                && self.players.values().any(|p| dist2d(p.pos, m.home) < 45.0 && (p.pos[2] - m.home[2]).abs() < 8.0)
        }).map(|(id, _)| *id).collect();
        for id in new { self.place_perch_crew(id); }

        // A corpse also completes its fall; ordinary corpse expiry must not leave it in the air.
        let falling: Vec<_> = self.perches.workers.iter().filter_map(|(id, w)| {
            if let Place::Falling { from, since } = w.state { Some((*id, from, since, w.landing, w.grounded)) } else { None }
        }).collect();
        for (id, from, since, to, ground) in falling {
            let t = ((now - since) / FALL_SECS).clamp(0.0, 1.0);
            if let Some(m) = self.mobs.get_mut(&id) {
                // The client follows the same accelerating fall, without floor projection.
                m.pos = falling_point(from, to, t);
                m.path.clear();
                if t >= 1.0 {
                    m.def = ground;
                    m.home = to;
                    m.thrown_lift = 0.0;
                    m.thrown_until = 0.0;
                    self.events.push(Event::MobGear { id });
                    self.events.push(Event::MobStop { id, pos: to, facing: m.facing });
                    self.events.push(Event::Visual { id, kit: combat::KIT_FALL_DUST, by: None });
                    self.perches.workers.get_mut(&id).unwrap().state = Place::Ground;
                    self.stagger_mob(id, LAND_STUN, false);
                }
            }
        }

        let owners: Vec<_> = self.perches.crews.keys().copied().collect();
        for owner in owners {
            let Some(foe) = self.mobs[&owner].state.foe() else { continue };
            if self.perches.crews[&owner].began.is_none() {
                let c = self.perches.crews.get_mut(&owner).unwrap();
                c.began = Some(now);
                c.next_throw = now + 3.0;
            }
            let ids = self.perches.crews[&owner].workers.clone();
            for id in &ids {
                if let Some(m) = self.mobs.get_mut(id).filter(|m| m.health > 0 && m.state.foe().is_none()) {
                    m.state = MobState::Chase { foe };
                }
            }
            let c = &self.perches.crews[&owner];
            if !matches!(c.stage, Stage::Crew) { continue; }
            let elapsed = now - c.began.unwrap();
            if let Some((worker, at)) = c.demonstration {
                if now >= at {
                    self.perches.crews.get_mut(&owner).unwrap().demonstration = None;
                    self.drop_perch(worker);
                }
                continue;
            }
            // Neither concurrent throws nor a valve blast overlapping an in-flight stick.
            let busy = self.missiles.iter().any(|m| m.owner == owner || ids.contains(&m.owner))
                || ids.iter().any(|id| self.mobs.get(id).is_some_and(|m| matches!(m.state, MobState::Windup { .. })))
                || ids.iter().any(|id| self.perches.workers.get(id).is_some_and(|w| matches!(w.state, Place::Falling { .. })));
            if busy { continue; }
            let perched: Vec<_> = ids.iter().copied().filter(|id| self.mobs.get(id).is_some_and(|m| m.health > 0)
                && self.perches.workers.get(id).is_some_and(|w| matches!(w.state, Place::Perched))).collect();
            if !c.demonstrated && elapsed >= FIRST_OVERFLOW {
                self.perches.crews.get_mut(&owner).unwrap().demonstrated = true;
                if let Some(&id) = perched.first() {
                    let at = self.perches.workers[&id].base;
                    self.perch_overflow(owner, at, 1.4, 1.5, 12);
                    let c = self.perches.crews.get_mut(&owner).unwrap();
                    c.demonstration = Some((id, now + 1.4));
                    c.next_throw = now + 5.0;
                    continue;
                }
            }
            if elapsed >= DESCEND_BY {
                if let Some(&id) = perched.first() {
                    self.drop_perch(id);
                    continue;
                }
            }
            if ids.iter().all(|id| !self.mobs.get(id).is_some_and(|m| m.health > 0)) {
                self.yell(owner, "Useless. Every one of you.");
                self.stagger_mob(owner, 0.05, false); // Let go of the current blow before running.
                self.perches.crews.get_mut(&owner).unwrap().stage = Stage::ToValve { since: now };
                continue;
            }
            if perched.is_empty() || now < self.perches.crews[&owner].next_throw { continue; }
            let turn = self.perches.crews[&owner].turn % perched.len();
            let id = perched[turn];
            let to = match foe { Foe::Hero(h) => self.players.get(&h).map(|p| p.pos), Foe::Obj(o) => self.objs.get(&o).map(|p| p.pos) };
            if let Some(to) = to.filter(|to| dist2d(*to, self.mobs[&id].pos) <= 26.0 && clear_line(self.land.as_ref(), self.mobs[&id].pos, *to)) {
                if !self.disabled(id) && self.mobs[&id].stagger_until <= now {
                    self.mobs.get_mut(&id).unwrap().pattern = None;
                    self.windup(id, foe, to);
                    let c = self.perches.crews.get_mut(&owner).unwrap();
                    c.turn += 1;
                    c.next_throw = now + 5.5;
                }
            }
        }
    }

    pub(crate) fn perch_controlled(&self, id: Id) -> bool {
        self.perches.workers.get(&id).is_some_and(|w| !matches!(w.state, Place::Ground))
    }

    /// A real projectile hit, a struck support, or the demonstration drops it exactly once.
    pub(crate) fn drop_perch(&mut self, id: Id) {
        let Some(w) = self.perches.workers.get(&id).filter(|w| matches!(w.state, Place::Perched)) else { return };
        let (owner, support, platform, base, to) = (w.owner, w.support, w.platform, w.base, w.landing);
        let Some(from) = self.mobs.get(&id).map(|m| m.pos) else { return };
        self.stagger_mob(id, FALL_SECS, false);
        let m = self.mobs.get_mut(&id).unwrap();
        m.path.clear();
        m.thrown_lift = 0.0;
        m.thrown_until = 0.0;
        self.perches.workers.get_mut(&id).unwrap().state = Place::Falling { from, since: self.clock };
        for prop in [support, platform] {
            self.objs.remove(&prop);
            self.events.push(Event::ObjGone { id: prop });
        }
        // The broken wood stays below the falling body. Dust belongs at the landing, where
        // it cannot hide the release and the first half of this short fall.
        let debris = self.perch_prop(base, 1108, 1.0);
        if let Some(c) = self.perches.crews.get_mut(&owner) { c.props.push(debris); }
        self.events.push(Event::Sound { id, sound: 3720 }); // Native wooden impact.
        let path = (0..=8).map(|k| falling_point(from, to, k as f32 / 8.0)).collect();
        self.events.push(Event::MobMove { id, path, speed: dist2d(from, to) / FALL_SECS, face: None });
        if let Some(c) = self.perches.crews.get_mut(&owner) {
            if !c.first_fall {
                c.first_fall = true;
                self.yell(owner, "Mind the equipment!");
            }
        }
    }

    /// Hitting a worker starts the same encounter as hitting its foreman.
    pub(crate) fn perch_hurt(&mut self, id: Id, by: Id) {
        let Some(owner) = self.perches.workers.get(&id).map(|w| w.owner) else { return };
        if self.players.get(&by).is_some_and(|p| p.health > 0)
            && self.mobs.get(&owner).is_some_and(|m| m.health > 0 && m.state.foe().is_none() && !matches!(m.state, MobState::Return)) {
            self.engage(owner, Foe::Hero(by));
        }
    }

    pub(crate) fn swing_on_perches(&mut self, hero: Id, face: f32, range: f32, arc: f32) {
        let Some(pos) = self.players.get(&hero).map(|p| p.pos) else { return };
        let hits: Vec<_> = self.perches.workers.iter().filter(|(_, w)| matches!(w.state, Place::Perched)
            && dist2d(pos, w.base) <= range + 0.6 && (pos[2] - w.base[2]).abs() < 2.0
            && angle_between(face, facing(pos, w.base)) <= arc / 2.0
            && clear_line(self.land.as_ref(), pos, w.base)).map(|(id, _)| *id).collect();
        for id in hits {
            self.perch_hurt(id, hero);
            self.drop_perch(id);
        }
    }

    fn perch_overflow(&mut self, owner: Id, at: Vec3, warn: f32, radius: f32, damage: u32) {
        // Five native vents cover the actual danger's width, not just its center. Their
        // anchors stay inside the damage disk, leaving the sides of the channel dry. Do not
        // enlarge a generic ground spell: its native model need not match its damage radius.
        if !self.perches.crews.contains_key(&owner) { return; }
        let edge = (radius - 0.7).max(0.0);
        for [x, y] in [[0.0, 0.0], [-edge, 0.0], [edge, 0.0], [0.0, -edge], [0.0, edge]] {
            let p = self.ground_near(at[0] + x, at[1] + y, at[2]);
            if (p[2] - at[2]).abs() > 1.0 || self.land.water_near(p[0], p[1], p[2])
                || self.land.blocked_near(p[0], p[1], p[2]) || !self.land.walk_clear(at, p) { continue; }
            let steam = self.perch_prop(p, 353, 1.4);
            self.perches.crews.get_mut(&owner).unwrap().steam.push((steam, self.clock + warn));
        }
        let mark = self.mark_ground(at, radius, warn, MOLTEN);
        self.missiles.push(combat::Missile { owner, from: at, at, radius, damage,
            blockable: false, heavy: false, land_at: self.clock + warn, mark, look: MOLTEN });
    }

    /// Walk to the visible valve, work it, then expose the foreman during the one discharge.
    /// A stagger delays his hands; it cannot restart the event or summon another workforce.
    pub(crate) fn step_perch_valve(&mut self, id: Id) -> bool {
        let Some(c) = self.perches.crews.get(&id) else { return false };
        let (stage, valve, home) = (c.stage, c.valve, c.home);
        let now = self.clock;
        if !matches!(stage, Stage::Crew | Stage::Finished)
            && (self.disabled(id) || self.mobs[&id].stagger_until > now) {
            self.halt(id);
            if matches!(stage, Stage::Opening { .. }) {
                self.events.push(Event::CastEnd { caster: id });
                self.perches.crews.get_mut(&id).unwrap().stage = Stage::ToValve { since: now };
            }
            return true;
        }
        match stage {
            Stage::ToValve { since } => {
                if dist2d(self.mobs[&id].pos, valve) > 1.5 && now - since < 8.0 {
                    self.go(id, valve, self.content.mobs[self.mobs[&id].def].run_speed);
                } else {
                    self.halt(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, valve);
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                    self.yell(id, "Then we'll melt the lot!");
                    self.events.push(Event::Cast { caster: id, spell: WORK_VALVE, ms: 2200 });
                    self.events.push(Event::Sound { id, sound: 3431 }); // Deadmines steam whistle.
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Opening { until: now + 2.2 };
                }
                true
            }
            Stage::Opening { until } => {
                self.halt(id);
                if now >= until {
                    self.events.push(Event::CastEnd { caster: id });
                    let d = self.content.mobs[self.mobs[&id].def].perch_crew.clone().unwrap();
                    // One narrow channel, never a room-wide carpet. Heat travels along it.
                    for (k, off) in d.channel.iter().enumerate() {
                        if let Some(at) = self.perch_floor(home, *off) {
                            self.perch_overflow(id, at, 1.8 + k as f32 * 0.7, 2.2, 30);
                        }
                    }
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Cooling { until: now + 1.8 + d.channel.len() as f32 * 0.7 + 1.8 };
                }
                true
            }
            Stage::Cooling { until } => {
                self.halt(id);
                if now >= until {
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Finished;
                }
                true
            }
            _ => false,
        }
    }

    pub(crate) fn remove_perch_crew(&mut self, owner: Id) {
        let Some(c) = self.perches.crews.remove(&owner) else { return };
        self.events.push(Event::CastEnd { caster: owner });
        for id in &c.workers {
            if let Some(m) = self.mobs.get(id) {
                if let MobState::Windup { mark: Some(mark), .. } = m.state { self.unmark(mark); }
            }
            if let Some((_, mark)) = self.mobs.get_mut(id).and_then(|m| m.lob.take()) { self.unmark(mark); }
            self.clear_cc(*id);
            self.perches.workers.remove(id);
            if self.mobs.remove(id).is_some() { self.events.push(Event::MobGone { id: *id }); }
        }
        let marks: Vec<_> = self.missiles.iter().filter(|m| m.owner == owner || c.workers.contains(&m.owner)).map(|m| m.mark).collect();
        self.missiles.retain(|m| m.owner != owner && !c.workers.contains(&m.owner));
        for mark in marks { self.unmark(mark); }
        for id in c.props.into_iter().chain(c.steam.into_iter().map(|(id, _)| id)) {
            if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
        }
    }
}
