//! Cookie serves one patch at a time, then risks a greedy scoop after completed exchanges.
//! Gorged shares the committed breath and cough, without a health phase or replacement wave.

use super::mobs::{Foe, MobState};
use super::*;
use crate::land::clear_line;

#[cfg(test)]
#[path = "kitchen_tests.rs"]
mod tests;

const COOKIE: &str = "dm_cookie";
const GORGED: &str = "dm_cookie_gorged";
const EXCHANGES: u8 = 3;
const SCOOP: f32 = 4.0;
const COUGH: f32 = 4.0;
const HOT: f32 = 3.0;
const COOL: f32 = 1.5;
const RADIUS: f32 = 2.0;
// Local 1.12.1 Steam02 is particles only, without the metal valve mesh of display 353.
// Its particle coverage still needs a real-client check; five anchors cover the small patch.
const STEAM: u32 = 431;

#[derive(Default)]
pub(crate) struct Kitchen {
    cooks: BTreeMap<Id, Cook>,
}

struct Cook {
    pot: Option<Id>,
    at: Vec3,
    began: bool,
    completed: u8,
    healed: u8,
    stage: Option<Stage>,
    patch: Option<Patch>,
    breath: Vec<(Id, f32)>,
}

#[derive(Clone, Copy)]
enum Stage {
    Seeking { until: f32 },
    Scooping { until: f32 },
    Coughing { until: f32 },
}

struct Patch {
    at: Vec3,
    props: Vec<Id>,
    hot_at: Option<f32>,
    next_tick: f32,
    damage: u32,
}

impl Survival {
    pub(crate) fn is_kitchen(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| matches!(self.content.mobs[m.def].key.as_str(), COOKIE | GORGED))
    }

    fn kitchen_move(&self, id: Id) -> Option<&str> {
        let m = self.mobs.get(&id)?;
        let (k, _) = m.pattern?;
        Some(&self.content.mobs[m.def].patterns.get(k)?.name)
    }

    pub(crate) fn kitchen_committed(&self, id: Id) -> bool {
        self.is_kitchen(id) && matches!(self.kitchen_move(id), Some("serve it hot" | "steam" | "belch"))
    }

    pub(crate) fn kitchen_scooping(&self, id: Id) -> bool {
        self.kitchen.cooks.get(&id).is_some_and(|c| matches!(c.stage, Some(Stage::Scooping { .. })))
    }

    /// Particle-only cues must show now. The cauldron keeps its normal fade and collision hull.
    pub fn is_kitchen_prop(&self, prop: Id) -> bool {
        self.kitchen.cooks.values().any(|c| c.patch.as_ref().is_some_and(|p| p.props.contains(&prop))
            || c.breath.iter().any(|(id, _)| *id == prop))
    }

    fn kitchen_prop(&mut self, at: Vec3, display: u32, scale: f32) -> Id {
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Prop { display, scale }, pos: at, facing: 0.0 });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    fn kitchen_drop_prop(&mut self, id: Id) {
        if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
    }

    fn kitchen_floor(&self, from: Vec3, x: f32, y: f32) -> Option<Vec3> {
        let p = self.ground_near(x, y, from[2]);
        ((p[2] - from[2]).abs() < 1.5 && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2]) && self.land.walk_clear(from, p)).then_some(p)
    }

    pub(crate) fn clean_kitchens(&mut self) {
        let stale: Vec<_> = self.kitchen.cooks.iter().filter(|(id, c)| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 && !matches!(m.state, MobState::Return)
                && (!c.began || m.state.foe().is_some()))
        }).map(|(id, _)| *id).collect();
        for id in stale { self.remove_kitchen(id); }
        let fresh: Vec<_> = self.mobs.iter().filter(|(id, m)| self.is_kitchen(**id)
            && m.health > 0 && !m.hidden && m.rise_at.is_none() && !matches!(m.state, MobState::Return)
            && !self.kitchen.cooks.contains_key(id)).map(|(id, _)| *id).collect();
        for id in fresh {
            let m = &self.mobs[&id];
            let at = self.kitchen_floor(m.home, m.home[0], m.home[1] + 1.8).unwrap_or(m.home);
            let pot = (self.content.mobs[m.def].key == COOKIE).then(|| self.kitchen_prop(at, 216, 1.0));
            self.kitchen.cooks.insert(id, Cook { pot, at, began: false, completed: 0, healed: 0,
                stage: None, patch: None, breath: Vec::new() });
        }
        let ids: Vec<_> = self.kitchen.cooks.keys().copied().collect();
        for id in ids { self.step_supper(id); }
    }

    pub(crate) fn remove_kitchen(&mut self, id: Id) {
        let Some(c) = self.kitchen.cooks.remove(&id) else { return };
        self.pending.retain(|p| p.attacker != id);
        if let Some(MobState::Windup { mark: Some(mark), .. }) = self.mobs.get(&id).map(|m| m.state.clone()) {
            self.unmark(mark);
        }
        if let Some(prop) = c.pot { self.kitchen_drop_prop(prop); }
        if let Some(p) = c.patch { for prop in p.props { self.kitchen_drop_prop(prop); } }
        for (prop, _) in c.breath { self.kitchen_drop_prop(prop); }
        if c.stage.is_some() { self.events.push(Event::ClipStop { id }); }
    }

    pub(crate) fn reset_kitchens(&mut self) {
        let ids: Vec<_> = self.kitchen.cooks.keys().copied().collect();
        for id in ids { self.remove_kitchen(id); }
    }

    /// The selection is a rhythm, not a health phase. A second pour cannot overlap the first.
    pub(crate) fn kitchen_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let c = self.kitchen.cooks.get(&id)?;
        let name = if d.key == GORGED {
            if c.completed % 2 == 0 { "slaps" } else { "belch" }
        } else {
            match c.completed % EXCHANGES { 0 => "cleaver", 1 => "ladle", _ => "serve it hot" }
        };
        if name == "serve it hot" && c.patch.is_some() { return None; }
        d.patterns.iter().position(|p| p.name == name
            && dist <= if p.max_range > 0.0 { p.max_range } else { p.steps[0].range.unwrap_or(d.attack.range) + size - 0.3 })
    }

    pub(crate) fn kitchen_windup(&mut self, id: Id) {
        if !self.kitchen_committed(id) || !self.kitchen.cooks.contains_key(&id) { return; }
        let name = self.kitchen_move(id).unwrap().to_owned();
        let m = &self.mobs[&id];
        let (pos, dir) = (m.pos, m.facing);
        let a = self.attack_now(id);
        if name == "serve it hot" {
            if self.kitchen.cooks[&id].patch.is_some() { return; }
            let Some(at) = self.kitchen_floor(pos, pos[0] + dir.cos() * 3.0, pos[1] + dir.sin() * 3.0) else { return };
            let mut props = Vec::new();
            for [dx, dy] in [[0.0, 0.0], [1.3, 0.0], [-1.3, 0.0], [0.0, 1.3], [0.0, -1.3]] {
                if let Some(p) = self.kitchen_floor(at, at[0] + dx, at[1] + dy) { props.push(self.kitchen_prop(p, STEAM, 1.4)); }
            }
            self.kitchen.cooks.get_mut(&id).unwrap().patch = Some(Patch { at, props, hot_at: None,
                next_tick: 0.0, damage: a.damage });
        } else {
            // Native steam marks the fixed direction before either breath damages the heroes.
            for ahead in [1.0, 2.5, 4.0] {
                if let Some(at) = self.kitchen_floor(pos, pos[0] + dir.cos() * ahead, pos[1] + dir.sin() * ahead) {
                    let prop = self.kitchen_prop(at, STEAM, 1.4);
                    self.kitchen.cooks.get_mut(&id).unwrap().breath.push((prop, self.clock + a.windup + 0.6));
                }
            }
        }
    }

    /// The optional marker must match the fixed patch too, rather than advertising a melee arc.
    pub(crate) fn kitchen_pour_mark(&mut self, id: Id, tele: Telegraph) -> Option<Id> {
        if !self.is_kitchen(id) || self.kitchen_move(id) != Some("serve it hot") { return None; }
        let m = &self.mobs[&id];
        let at = self.kitchen_floor(m.pos, m.pos[0] + m.facing.cos() * 3.0, m.pos[1] + m.facing.sin() * 3.0).unwrap_or(m.pos);
        Some(self.mark_as(at, RADIUS, tele.secs, Some(Telegraph { reach: RADIUS, arc: 360.0,
            at: Some((at, 0.0)), foe: None, ..tele })))
    }

    /// Pour damage belongs to its one patch. The generic swing must not also hit an arc.
    pub(crate) fn kitchen_release(&mut self, id: Id) -> bool {
        if !self.is_kitchen(id) { return false; }
        if matches!(self.kitchen_move(id), Some("steam" | "belch")) {
            self.events.push(Event::Sound { id, sound: 4858 }); // One-shot SteamGeyserVar1, no orphaned loop.
        }
        if self.kitchen_move(id) != Some("serve it hot") { return false; }
        self.events.push(Event::Sound { id, sound: 1096 }); // Native medium water splash.
        if let Some(p) = self.kitchen.cooks.get_mut(&id).and_then(|c| c.patch.as_mut()) {
            if p.hot_at.is_none() {
                p.hot_at = Some(self.clock);
                p.next_tick = self.clock;
            }
        }
        true
    }

    pub(crate) fn kitchen_string_landed(&mut self, id: Id, pattern: Option<usize>) {
        if !self.is_kitchen(id) { return; }
        let Some(k) = pattern else { return };
        let m = &self.mobs[&id];
        let name = self.content.mobs[m.def].patterns[k].name.as_str();
        let Some(c) = self.kitchen.cooks.get_mut(&id) else { return };
        if name != "steam" { c.completed = if self.content.mobs[m.def].key == GORGED { (c.completed + 1) % 2 } else { c.completed.saturating_add(1) }; }
        if matches!(name, "steam" | "belch") { self.kitchen_cough(id, COUGH); }
    }

    fn kitchen_cough(&mut self, id: Id, secs: f32) {
        let now = self.clock;
        self.halt(id);
        let m = self.mobs.get_mut(&id).unwrap();
        if let Some(foe) = m.state.foe() {
            m.pattern = None;
            m.ready_at = m.ready_at.max(now + secs);
            m.state = MobState::Recover { foe, until: now + secs };
        }
        self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Coughing { until: now + secs });
        // Murloc has no cough/eat/work clips. Its native wounded recoil is the visible recovery.
        self.events.push(Event::Clip { id, clip: 9, take: 0, from: None, secs, at: 0.0, body: ClipBody::Full });
        self.events.push(Event::Sound { id, sound: 418 });
    }

    pub(crate) fn kitchen_broken(&mut self, id: Id) {
        if !self.kitchen_scooping(id) { return; }
        self.yell(id, "MRGLGLGL!");
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::Sound { id, sound: 1096 });
        let prop = self.kitchen_prop(self.mobs[&id].pos, STEAM, 1.2);
        self.kitchen.cooks.get_mut(&id).unwrap().breath.push((prop, self.clock + 1.3));
        self.kitchen_cough(id, COUGH);
    }

    fn step_supper(&mut self, id: Id) {
        let now = self.clock;
        let cancelled = !self.kitchen_committed(id) && !matches!(self.kitchen.cooks[&id].stage, Some(Stage::Coughing { .. }));
        let expired: Vec<_> = self.kitchen.cooks[&id].breath.iter().filter(|(_, end)| now >= *end || cancelled).map(|(p, _)| *p).collect();
        self.kitchen.cooks.get_mut(&id).unwrap().breath.retain(|(_, end)| now < *end && !cancelled);
        for p in expired { self.kitchen_drop_prop(p); }
        let Some(mut patch) = self.kitchen.cooks.get_mut(&id).unwrap().patch.take() else { return };
        let Some(start) = patch.hot_at else {
            if self.kitchen_move(id) == Some("serve it hot") && matches!(self.mobs[&id].state, MobState::Windup { .. }) {
                self.kitchen.cooks.get_mut(&id).unwrap().patch = Some(patch);
            } else { for p in patch.props { self.kitchen_drop_prop(p); } }
            return;
        };
        if now >= start + HOT + COOL {
            for p in patch.props { self.kitchen_drop_prop(p); }
            return;
        }
        if now >= start + HOT {
            // Last thin wisp is harmless cooling. No new patch until it disappears too.
            if patch.props.len() > 1 { for p in patch.props.drain(1..) { self.kitchen_drop_prop(p); } }
        } else if now >= patch.next_tick {
            let damage = if patch.next_tick == start { patch.damage } else { (patch.damage / 5).max(1) };
            patch.next_tick = now + 1.0;
            let heroes: Vec<_> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, patch.at) <= RADIUS
                && (p.pos[2] - patch.at[2]).abs() < 2.0 && clear_line(self.land.as_ref(), patch.at, p.pos)).map(|(h, _)| *h).collect();
            for h in heroes {
                self.strike_fair(id, h, damage, patch.at, false, false, false, Blow::Light,
                    Some(combat::Area::Circle { at: patch.at, radius: RADIUS }), combat::Follow::None);
            }
        }
        self.kitchen.cooks.get_mut(&id).unwrap().patch = Some(patch);
    }

    /// A scoop only starts after the preceding recovery and cooling, and never steals a turn.
    pub(crate) fn step_kitchen(&mut self, id: Id) -> bool {
        let Some(c) = self.kitchen.cooks.get(&id) else { return false };
        let now = self.clock;
        let m = &self.mobs[&id];
        let Some(foe) = m.state.foe() else { return false };
        let target = self.players.iter().filter(|(_, p)| p.health > 0
            && dist2d(p.pos, m.home) <= self.content.mobs[m.def].leash && (p.pos[2] - m.home[2]).abs() < 6.0)
            .min_by(|a, b| dist2d(a.1.pos, m.pos).total_cmp(&dist2d(b.1.pos, m.pos))).map(|(h, _)| *h);
        let Some(hero) = target else {
            self.remove_kitchen(id);
            self.halt(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.pattern = None;
            m.state = MobState::Return;
            return true;
        };
        let foe = if self.foe_pos(foe).is_some() { foe } else { Foe::Hero(hero) };
        let (at, stage) = (c.at, c.stage);
        if stage.is_none() {
            let safe = m.pattern.is_none() && !matches!(m.state, MobState::Windup { .. })
                && !matches!(m.state, MobState::Recover { until, .. } if until > now);
            let due = self.content.mobs[m.def].key == COOKIE && c.completed >= EXCHANGES;
            let cooling = c.patch.is_some();
            self.kitchen.cooks.get_mut(&id).unwrap().began = true;
            if !due || !safe || self.disabled(id) || self.mobs[&id].stagger_until > now { return false; }
            if cooling { self.halt(id); return true; }
            let c = self.kitchen.cooks.get_mut(&id).unwrap();
            c.completed = 0;
            c.stage = Some(Stage::Seeking { until: now + 6.0 });
            return true;
        }
        match stage.unwrap() {
            Stage::Seeking { until } => {
                let pos = self.mobs[&id].pos;
                if dist2d(pos, at) <= 1.8 {
                    self.halt(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(pos, at);
                    m.state = MobState::Recover { foe, until: now + SCOOP };
                    m.posture_guard = m.posture_guard.min(now);
                    self.events.push(Event::MobStop { id, pos, facing: m.facing });
                    self.yell(id, if self.kitchen.cooks[&id].healed >= 2 { "Mrrgl... scraps." } else { "Mmm... one taste." });
                    // Native LoadThrown uses his hands at the pot; no nonexistent work emote.
                    self.events.push(Event::Clip { id, clip: 112, take: 0, from: None, secs: SCOOP, at: 0.0, body: ClipBody::Full });
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Scooping { until: now + SCOOP });
                } else if now >= until {
                    // A blocked pot gives up this scoop. No teleport and no heal from afar.
                    self.kitchen_cough(id, 1.5);
                } else if !self.disabled(id) && self.mobs[&id].stagger_until <= now && self.mobs[&id].path.is_empty()
                    && (self.land.walk_clear(pos, at) || self.land.path(pos, at).is_some_and(|p| !p.is_empty())) {
                    self.go(id, at, 4.0);
                }
            }
            Stage::Scooping { until } => {
                self.halt(id);
                if self.disabled(id) || self.mobs[&id].stagger_until > now { self.kitchen_broken(id); }
                else if now >= until {
                    let c = self.kitchen.cooks.get_mut(&id).unwrap();
                    // Two tastes of 4% at most in a pull, including tastes at full health.
                    let heal = c.healed < 2;
                    c.healed = c.healed.saturating_add(1).min(2);
                    c.stage = None;
                    let emptied = heal && c.healed == 2;
                    if heal {
                        let m = self.mobs.get_mut(&id).unwrap();
                        m.health = (m.health + (m.max_health / 25).max(1)).min(m.max_health);
                        self.events.push(Event::Health { id, health: m.health, max: m.max_health });
                    }
                    if emptied { self.yell(id, "Mrrgl... pot's empty."); }
                    self.events.push(Event::ClipStop { id });
                    let k = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "steam").unwrap();
                    self.begin_pattern(id, k, false);
                    self.windup(id, foe, self.players[&hero].pos);
                }
            }
            Stage::Coughing { until } => {
                self.halt(id);
                if now >= until.max(self.mobs[&id].stagger_until) {
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = None;
                    self.events.push(Event::ClipStop { id });
                }
            }
        }
        true
    }
}
