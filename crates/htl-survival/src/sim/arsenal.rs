//! Smite's arsenal: health asks for a change; a safe boundary and a visit to the rack deliver it.
//! The transition never damages or disables heroes. Damage and posture buy time, not a softlock.

use super::mobs::{Foe, MobState};
use super::*;

#[cfg(test)]
#[path = "arsenal_tests.rs"]
mod tests;

const CHANGE_LIMIT: f32 = 14.0;
const AXE_SHOWING: f32 = 7.0;
const FIT_WEAPON: f32 = 2.0;
/// TaurenMale's KneelLoop: down on one knee at the rack, rummaging, hands empty.
const KNEEL_CLIP: u16 = 115;
/// Struck while he kneels at the rack, his posture takes this much more.
const KNEEL_POSTURE: f32 = 1.5;
/// The weapons left lying at the rack: the sword he put down, the hammer waiting for him.
const SWORD_LEFT: u32 = 670;
const HAMMER_WAITING: u32 = 1067;
/// The rattle of the rack as he rummages.
const RACK_RATTLE: u32 = 698;

#[derive(Default)]
pub(crate) struct Arsenal {
    racks: BTreeMap<Id, Rack>,
}

struct Rack {
    prop: Id,
    /// What lies at its foot: the hammer until he takes it, the sword once he has put it down.
    lying: Vec<Id>,
    at: Vec3,
    began_fight: bool,
    next_change: f32,
    transition: Option<Change>,
}

#[derive(Clone, Copy)]
struct Change {
    since: f32,
    pause_until: f32,
    stage: ChangeStage,
    working: bool,
    /// His hands are empty: the old weapon down, the new one not yet up.
    bare: bool,
}

#[derive(Clone, Copy)]
enum ChangeStage {
    Back { until: f32 },
    Point { until: f32 },
    Run,
    Equip { until: f32 },
}

impl Survival {
    pub(crate) fn has_arsenal(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].arsenal.is_some())
    }

    /// A late observer and an observer already present see exactly the same loadout.
    pub fn arsenal_loadout(&self, id: Id) -> Option<&crate::content::ArsenalLoadout> {
        let m = self.mobs.get(&id)?;
        self.content.mobs[m.def].arsenal.as_ref()?.loadouts.get(m.phase)
    }

    /// Kneeling at the rack between weapons, his hands empty (a late observer sees them so too).
    pub fn arsenal_hands_empty(&self, id: Id) -> bool {
        self.arsenal.racks.get(&id).and_then(|r| r.transition).is_some_and(|c| c.bare)
    }

    /// His posture's share of a blow: more while he kneels at the rack. 1 for anyone else.
    pub(crate) fn arsenal_posture_share(&self, id: Id) -> f32 {
        let kneeling = self.arsenal.racks.get(&id).and_then(|r| r.transition).is_some_and(|c| matches!(c.stage, ChangeStage::Equip { .. }));
        if kneeling { KNEEL_POSTURE } else { 1.0 }
    }

    /// A weapon laid at the rack's foot, a little to one side.
    fn lay_at_rack(&mut self, id: Id, display: u32, side: f32) {
        let Some(r) = self.arsenal.racks.get(&id) else { return };
        let at = r.at;
        let p = self.alloc();
        let pos = self.ground_near(at[0] + 0.9, at[1] + side, at[2]);
        self.objs.insert(p, Obj { kind: ObjKind::Prop { display, scale: 1.2 }, pos, facing: 0.4 + side });
        self.events.push(Event::ObjSpawned { id: p });
        self.arsenal.racks.get_mut(&id).unwrap().lying.push(p);
    }

    /// The weapon of `display` lying at the rack is taken up.
    fn take_from_rack(&mut self, id: Id, display: u32) {
        let Some(r) = self.arsenal.racks.get(&id) else { return };
        let Some(k) = r.lying.iter().position(|p| matches!(self.objs.get(p).map(|o| &o.kind), Some(ObjKind::Prop { display: d, .. }) if *d == display)) else { return };
        let p = self.arsenal.racks.get_mut(&id).unwrap().lying.remove(k);
        if self.objs.remove(&p).is_some() { self.events.push(Event::ObjGone { id: p }); }
    }

    pub(crate) fn arsenal_pattern_allowed(&self, id: Id, name: &str) -> bool {
        self.arsenal_loadout(id).is_none_or(|g| g.patterns.iter().any(|p| p == name))
    }

    fn rack_floor(&self, home: Vec3, offset: [f32; 2]) -> Option<Vec3> {
        let p = self.ground_near(home[0] + offset[0], home[1] + offset[1], home[2]);
        ((p[2] - home[2]).abs() < 2.0
            && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2])
            && (self.land.walk_clear(home, p) || self.land.path(home, p).is_some_and(|v| !v.is_empty())))
            .then_some(p)
    }

    pub(crate) fn clean_arsenals(&mut self) {
        let gone: Vec<_> = self.arsenal.racks.iter().filter(|(id, r)| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 && !matches!(m.state, MobState::Return)
                && (!r.began_fight || m.state.foe().is_some()))
        }).map(|(id, _)| *id).collect();
        for id in gone { self.remove_arsenal(id); }
        let fresh: Vec<_> = self.mobs.iter().filter(|(id, m)| {
            m.health > 0 && !m.hidden && !matches!(m.state, MobState::Return)
                && self.content.mobs[m.def].arsenal.is_some() && !self.arsenal.racks.contains_key(id)
        }).map(|(id, _)| *id).collect();
        for id in fresh {
            let m = &self.mobs[&id];
            let d = self.content.mobs[m.def].arsenal.as_ref().unwrap();
            // A bad authored point falls back to the known spawn floor; never another floor.
            let at = self.rack_floor(m.home, d.rack).unwrap_or(m.home);
            let display = d.display;
            let prop = self.alloc();
            self.objs.insert(prop, Obj { kind: ObjKind::Prop { display, scale: 1.0 }, pos: at, facing: 0.0 });
            self.events.push(Event::ObjSpawned { id: prop });
            self.arsenal.racks.insert(id, Rack { prop, lying: Vec::new(), at, began_fight: false, next_change: 0.0, transition: None });
            self.lay_at_rack(id, HAMMER_WAITING, -0.6);
        }
    }

    pub(crate) fn remove_arsenal(&mut self, id: Id) {
        if let Some(r) = self.arsenal.racks.remove(&id) {
            for p in std::iter::once(r.prop).chain(r.lying) {
                if self.objs.remove(&p).is_some() { self.events.push(Event::ObjGone { id: p }); }
            }
            self.events.push(Event::Pose { id, pose: 0 });
            if r.transition.is_some_and(|c| c.working) {
                self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
                if self.mobs.get(&id).is_some_and(|m| m.health > 0) { self.events.push(Event::MobGear { id }); }
            }
        }
        if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
            if m.phase != 0 {
                m.phase = 0;
                m.enraged = false;
                self.events.push(Event::MobGear { id });
            }
        }
    }

    pub(crate) fn reset_arsenals(&mut self) {
        let ids: Vec<_> = self.arsenal.racks.keys().copied().collect();
        for id in ids {
            self.remove_arsenal(id);
            self.halt(id);
            if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                m.pattern = None;
                m.stagger_until = 0.0;
                m.state = MobState::Idle { wander_at: self.clock + 5.0 };
            }
        }
    }

    pub(crate) fn arsenal_hurt(&mut self, id: Id) {
        let now = self.clock;
        if let Some(c) = self.arsenal.racks.get_mut(&id).and_then(|r| r.transition.as_mut()) {
            // Any ordinary damaging blow buys a visible half beat, once per current half beat.
            c.pause_until = (now + 0.45).min(c.since + CHANGE_LIMIT - FIT_WEAPON);
            if let ChangeStage::Equip { until } = &mut c.stage {
                *until = until.max(now + 0.65).min(c.since + CHANGE_LIMIT);
            }
        }
    }

    /// Runs after walking, before ordinary stun/AI handling, so even repeated posture breaks have
    /// a bounded ending. He remains damageable throughout; a lethal blow still ends the fight.
    pub(crate) fn step_arsenal(&mut self, id: Id) -> bool {
        let Some(r) = self.arsenal.racks.get(&id) else { return false };
        let now = self.clock;
        let m = &self.mobs[&id];
        let Some(foe) = m.state.foe() else { return false };
        let d = &self.content.mobs[m.def];
        let target = match foe {
            Foe::Hero(h) if self.players.get(&h).is_some_and(|p| p.health > 0 && dist2d(p.pos, m.home) <= d.leash) => Some(h),
            _ => self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, m.home) <= d.leash && (p.pos[2] - m.home[2]).abs() < 6.0)
                .min_by(|a, b| dist2d(a.1.pos, m.pos).total_cmp(&dist2d(b.1.pos, m.pos))).map(|(h, _)| *h),
        };
        let Some(hero) = target else {
            self.halt(id);
            self.remove_arsenal(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.pattern = None;
            m.state = MobState::Return;
            return true;
        };
        let foe = Foe::Hero(hero);
        let (at, change, next_change) = (r.at, r.transition, r.next_change);
        self.arsenal.racks.get_mut(&id).unwrap().began_fight = true;
        if change.is_none() {
            let m = &self.mobs[&id];
            let d = &self.content.mobs[m.def];
            let due = d.phases.get(m.phase).is_some_and(|p| m.health as f32 <= p.at * m.max_health as f32);
            let safe = m.pattern.is_none() && !matches!(m.state, MobState::Windup { .. })
                && !matches!(m.state, MobState::Recover { until, .. } if until > now);
            if !due || !safe || now < next_change || self.disabled(id) || m.stagger_until > now { return false; }
            let phase = d.phases[m.phase].clone();
            self.halt(id);
            self.yell(id, phase.text);
            if phase.sound != 0 { self.events.push(Event::Sound { id, sound: phase.sound }); }
            let pos = self.mobs[&id].pos;
            let away = facing(self.players[&hero].pos, pos);
            let back = self.ground_near(pos[0] + away.cos() * 1.2, pos[1] + away.sin() * 1.2, pos[2]);
            if (back[2] - pos[2]).abs() < 1.0 && !self.land.water_near(back[0], back[1], back[2])
                && self.land.walk_clear(pos, back) {
                self.sidle_to(id, back, 2.0, hero);
            }
            self.arsenal.racks.get_mut(&id).unwrap().transition = Some(Change {
                since: now, pause_until: now, stage: ChangeStage::Back { until: now + 0.65 }, working: false, bare: false,
            });
            return true;
        }
        let mut change = change.unwrap();
        let forced = now >= change.since + CHANGE_LIMIT - FIT_WEAPON;
        if !forced && (now < change.pause_until || self.disabled(id) || self.mobs[&id].stagger_until > now) {
            self.halt(id);
            // A posture break during the fitting restarts its hands, within the same deadline.
            if matches!(change.stage, ChangeStage::Equip { .. }) {
                change.stage = ChangeStage::Equip { until: (self.mobs[&id].stagger_until.max(now) + FIT_WEAPON).min(change.since + CHANGE_LIMIT) };
                if change.working { self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None }); }
                change.working = false;
            }
            self.arsenal.racks.get_mut(&id).unwrap().transition = Some(change);
            return true;
        }
        match change.stage {
            ChangeStage::Back { until } if now < until => {}
            ChangeStage::Back { .. } => {
                self.halt(id);
                let m = self.mobs.get_mut(&id).unwrap();
                m.facing = facing(m.pos, at);
                self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                self.events.push(Event::Emote { id, emote: 25 }); // Native point, then a run.
                change.stage = ChangeStage::Point { until: now + 0.8 };
            }
            ChangeStage::Point { until } if now < until => { self.halt(id); }
            ChangeStage::Point { .. } => { change.stage = ChangeStage::Run; }
            ChangeStage::Run => {
                let pos = self.mobs[&id].pos;
                if dist2d(pos, at) <= 1.7 && (pos[2] - at[2]).abs() < 2.0 || forced {
                    self.halt(id);
                    if forced && dist2d(pos, at) > 1.7 {
                        // A blocked route uses carried spares where he stands. Never warp through
                        // a wall, keep retrying forever, or make the damaged encounter unwinnable.
                        self.events.push(Event::Log(format!("arsenal {id}: rack route timed out; fitting carried spares")));
                    }
                    // Down on one knee at the rack, the old weapon put down and his hands empty,
                    // rummaging with a rattle (Nov 8 review: no weapon was seen to change hands).
                    let face = facing(self.mobs[&id].pos, at);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = face;
                    let (pos, phase) = (m.pos, m.phase);
                    self.events.push(Event::MobStop { id, pos, facing: face });
                    self.events.push(Event::HoldPose { id, clip: KNEEL_CLIP, secs: FIT_WEAPON + 0.3, from: None });
                    self.events.push(Event::Sound { id, sound: RACK_RATTLE });
                    change.working = true;
                    change.bare = true;
                    self.arsenal.racks.get_mut(&id).unwrap().transition = Some(change);
                    self.events.push(Event::MobGear { id });
                    if phase == 0 { self.lay_at_rack(id, SWORD_LEFT, 0.6); }
                    change.stage = ChangeStage::Equip { until: now + FIT_WEAPON };
                } else if self.mobs[&id].path.is_empty() {
                    let speed = self.content.mobs[self.mobs[&id].def].run_speed;
                    // Explicit reachability prevents go()'s no-planner fallback crossing a wall.
                    if self.land.walk_clear(pos, at) || self.land.path(pos, at).is_some_and(|p| !p.is_empty()) {
                        self.go(id, at, speed);
                    }
                }
            }
            ChangeStage::Equip { until } => {
                self.halt(id);
                if !change.working {
                    self.events.push(Event::HoldPose { id, clip: KNEEL_CLIP, secs: (until - now).max(0.3) + 0.3, from: None });
                    self.events.push(Event::Sound { id, sound: RACK_RATTLE });
                    change.working = true;
                }
                if now >= until {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.phase += 1;
                    m.enraged = true;
                    m.pattern = None;
                    m.space = None;
                    m.last_pattern = None;
                    m.ready_at = now + 0.8;
                    m.state = MobState::Chase { foe };
                    m.think = 0.0;
                    let phase = m.phase;
                    // He stands with the new weapon up and roars; the hammer leaves the rack.
                    self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
                    self.arsenal.racks.get_mut(&id).unwrap().transition = None;
                    self.events.push(Event::MobGear { id });
                    self.events.push(Event::Sound { id, sound: 700 }); // Verified UnsheathMetal.
                    self.events.push(Event::Emote { id, emote: super::mobs::ROAR_EMOTE });
                    if phase == 2 { self.take_from_rack(id, HAMMER_WAITING); }
                    let r = self.arsenal.racks.get_mut(&id).unwrap();
                    r.next_change = now + AXE_SHOWING;
                    return true;
                }
            }
        }
        self.arsenal.racks.get_mut(&id).unwrap().transition = Some(change);
        true
    }
}
