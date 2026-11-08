//! Greenskin's finite firing line. One shared clock gives the gunners separate turns, then
//! gives the captain the deck while they reload. Killing both crew changes his repertoire.

use super::mobs::{Foe, MobState};
use super::*;
use crate::land::clear_line;

#[cfg(test)]
#[path = "firing_line_tests.rs"]
mod tests;

const SHOT_GAP: f32 = 3.6;
const RELOAD: f32 = 5.0;
// Measured on the actual quarterdeck: both have dry support and short paths from the captain.
const POSTS: [[f32; 2]; 2] = [[4.0, 2.0], [-3.0, 2.0]];
const GUN: u32 = 21071; // Native short blunderbuss. Vanilla has no one-handed pistol model.

#[derive(Default)]
pub(crate) struct FiringLine {
    crews: BTreeMap<Id, Crew>,
    owners: BTreeMap<Id, Id>,
    aims: BTreeMap<Id, Vec3>,
    shouldered: BTreeSet<Id>,
    hooks: BTreeSet<(Id, u32)>,
    drawn: BTreeSet<Id>,
}

struct Crew {
    gunners: Vec<Id>,
    stage: Stage,
    first_shot: bool,
}

#[derive(Clone, Copy)]
enum Stage {
    Volley { turn: usize, next: f32 },
    Reload { until: f32 },
    Alone,
}

impl Survival {
    pub(crate) fn is_firing_captain(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == "dm_greenskin")
    }

    pub(crate) fn firing_sees_veil(&self, id: Id) -> bool {
        self.firing_line.crews.contains_key(&id) || self.firing_line.owners.contains_key(&id)
    }

    pub(crate) fn firing_pattern_allowed(&self, id: Id, name: &str) -> bool {
        !self.is_firing_captain(id) || !matches!(name, "pistol shot" | "harpoon")
            || self.firing_line.crews.get(&id).is_some_and(|c| matches!(c.stage, Stage::Alone))
    }

    /// The create packet and later equipment changes agree, including for late observers.
    pub fn firing_line_gun(&self, id: Id) -> Option<u32> {
        (self.firing_line.owners.contains_key(&id) || self.firing_line.drawn.contains(&id)).then_some(GUN)
    }

    fn firing_floor(&self, home: Vec3, off: [f32; 2]) -> Option<Vec3> {
        let p = self.ground_near(home[0] + off[0], home[1] + off[1], home[2]);
        ((p[2] - home[2]).abs() < 1.5
            && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2])
            && (self.land.walk_clear(home, p) || self.land.path(home, p).is_some_and(|v| !v.is_empty())))
            .then_some(p)
    }

    fn place_firing_line(&mut self, owner: Id, foe: Foe) {
        let Some(def) = self.content.mob("dm_deck_gunner") else { return };
        let home = self.mobs[&owner].home;
        let towards = self.foe_pos(foe).unwrap_or(home);
        let mut gunners = Vec::new();
        for off in POSTS {
            // A custom arena with a blocked post uses a checked spot beside the captain.
            // Never put a melee target into the water or on the floor below the ship.
            let at = self.firing_floor(home, off).or_else(|| self.firing_floor(home, [off[0].signum(), 0.0]));
            let Some(at) = at else {
                self.events.push(Event::Log(format!("firing line {owner}: unsafe gunner post omitted")));
                continue;
            };
            let id = self.spawn_mob(def, at, None, false);
            let m = self.mobs.get_mut(&id).unwrap();
            m.summoned_by = Some(owner);
            m.state = MobState::Chase { foe };
            m.home = home;
            m.facing = facing(at, towards);
            self.firing_line.owners.insert(id, owner);
            self.events.push(Event::MobGear { id });
            gunners.push(id);
        }
        self.firing_line.crews.insert(owner, Crew {
            gunners, stage: Stage::Volley { turn: 0, next: self.clock + 1.0 }, first_shot: true,
        });
        self.halt(owner);
        self.yell(owner, "Steady... FIRE!");
    }

    /// Includes an existing living foe even while veiled. These scheduled combatants never
    /// mistake a solo hero's Vanish for a wipe, nor lose a surviving party member.
    fn firing_foe(&self, owner: Id) -> Option<Id> {
        let m = self.mobs.get(&owner)?;
        let d = &self.content.mobs[m.def];
        let valid = |h: &Id| self.players.get(h).is_some_and(|p| p.health > 0
            && dist2d(p.pos, m.home) <= d.leash && (p.pos[2] - m.home[2]).abs() < 6.0);
        m.state.target().filter(valid).or_else(|| self.players.keys().filter(|h| valid(h))
            .min_by(|a, b| dist2d(self.players[a].pos, m.pos).total_cmp(&dist2d(self.players[b].pos, m.pos))).copied())
    }

    pub(crate) fn clean_firing_lines(&mut self) {
        let gone: Vec<_> = self.firing_line.crews.keys().copied().filter(|id| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 && m.state.foe().is_some()
                && !matches!(m.state, MobState::Return)) || self.firing_foe(*id).is_none()
        }).collect();
        for id in gone {
            self.remove_firing_line(id);
            self.halt(id);
            if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                m.pattern = None;
                m.state = MobState::Return;
            }
        }
        let new: Vec<_> = self.mobs.iter().filter(|(id, m)| self.is_firing_captain(**id)
            && m.health > 0 && m.state.foe().is_some() && !self.firing_line.crews.contains_key(id))
            .map(|(id, m)| (*id, m.state.foe().unwrap())).collect();
        for (id, foe) in new { self.place_firing_line(id, foe); }
        let sheathe: Vec<_> = self.firing_line.drawn.iter().copied().filter(|id| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 && matches!(m.state, MobState::Windup { .. } | MobState::Recover { .. }))
        }).collect();
        for id in sheathe {
            self.firing_line.drawn.remove(&id);
            self.events.push(Event::MobGear { id });
        }
        self.firing_line.aims.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0 && matches!(m.state, MobState::Windup { .. })));
        self.firing_line.shouldered.retain(|id| self.firing_line.aims.contains_key(id));
    }

    pub(crate) fn remove_firing_line(&mut self, owner: Id) {
        let Some(c) = self.firing_line.crews.remove(&owner) else { return };
        let mut ids = c.gunners;
        ids.push(owner);
        for id in &ids {
            self.firing_line.owners.remove(id);
            self.firing_line.aims.remove(id);
            self.firing_line.shouldered.remove(id);
            self.firing_line.drawn.remove(id);
            self.events.push(Event::CastEnd { caster: *id });
            if *id != owner { self.dismiss(*id); }
        }
        let marks: Vec<_> = self.missiles.iter().filter(|m| ids.contains(&m.owner)).map(|m| m.mark).collect();
        self.missiles.retain(|m| !ids.contains(&m.owner));
        self.pending.retain(|s| !ids.contains(&s.attacker));
        self.firing_line.hooks.retain(|(id, _)| !ids.contains(id));
        for mark in marks { self.unmark(mark); }
        if let Some(m) = self.mobs.get_mut(&owner) { m.phase = 0; }
        self.events.push(Event::MobGear { id: owner });
    }

    pub(crate) fn firing_died(&mut self, id: Id) {
        self.remove_firing_line(id);
        if !self.firing_line.owners.contains_key(&id) { return; }
        self.firing_line.aims.remove(&id);
        self.firing_line.shouldered.remove(&id);
        self.missiles.retain(|m| m.owner != id);
        self.pending.retain(|s| s.attacker != id);
    }

    pub(crate) fn reset_firing_lines(&mut self) {
        let ids: Vec<_> = self.firing_line.crews.keys().copied().collect();
        for id in ids {
            self.remove_firing_line(id);
            self.halt(id);
            if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                m.pattern = None;
                m.state = MobState::Idle { wander_at: self.clock + 5.0 };
            }
        }
    }

    fn firing_safe(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| m.pattern.is_none() && !matches!(m.state, MobState::Windup { .. })
            && !matches!(m.state, MobState::Recover { until, .. } if until > self.clock)
            && m.stagger_until <= self.clock && !self.disabled(id))
    }

    /// The gunners finish only the shots the shared clock assigned. They never run the
    /// independent caster AI, so a stagger or a party retarget cannot bunch the next volley.
    pub(crate) fn step_firing_gunner(&mut self, id: Id, dt: f32) -> bool {
        let Some(owner) = self.firing_line.owners.get(&id).copied() else { return false };
        // Their own legs stay planted, but ordinary melee knockbacks and pulls still move them.
        if self.mobs[&id].thrown_until > self.clock { return true; }
        self.halt(id);
        if self.disabled(id) || self.mobs[&id].stagger_until > self.clock { return true; }
        self.firing_shoulder(id);
        if matches!(self.mobs[&id].state, MobState::Windup { .. }) {
            self.think(id, dt);
        } else if let Some(h) = self.firing_foe(owner) {
            self.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        }
        true
    }

    pub(crate) fn step_firing_captain(&mut self, id: Id) -> bool {
        self.firing_shoulder(id);
        let Some(c) = self.firing_line.crews.get(&id) else { return false };
        let Some(hero) = self.firing_foe(id) else { return true };
        let foe = Foe::Hero(hero);
        let (stage, ids, first) = (c.stage, c.gunners.clone(), c.first_shot);
        let live: Vec<_> = ids.iter().copied().filter(|g| self.mobs.get(g).is_some_and(|m| m.health > 0)).collect();
        let busy = ids.iter().any(|g| self.mobs.get(g).is_some_and(|m| m.health > 0 && matches!(m.state, MobState::Windup { .. })))
            || self.missiles.iter().any(|m| ids.contains(&m.owner));
        if live.is_empty() && !matches!(stage, Stage::Alone) && !busy && self.firing_safe(id) {
            self.firing_line.crews.get_mut(&id).unwrap().stage = Stage::Alone;
            self.mobs.get_mut(&id).unwrap().phase = 1;
            self.yell(id, "I'll do it meself.");
            if let Some(k) = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "pistol shot") {
                self.begin_pattern(id, k, false);
                self.windup(id, foe, self.players[&hero].pos);
            }
            return true;
        }
        match stage {
            Stage::Alone => false,
            Stage::Reload { until } if self.clock < until || !self.firing_safe(id) => false,
            Stage::Reload { .. } => {
                self.firing_line.crews.get_mut(&id).unwrap().stage = Stage::Volley { turn: 0, next: self.clock + 0.6 };
                self.halt(id);
                self.yell(id, "Steady... FIRE!");
                true
            }
            Stage::Volley { turn, next } => {
                // An externally forced attack is allowed to finish before the firing line takes over.
                if !self.firing_safe(id) { return false; }
                self.halt(id);
                if self.clock < next || busy { return true; }
                if turn >= ids.len() {
                    self.firing_line.crews.get_mut(&id).unwrap().stage = Stage::Reload { until: self.clock + RELOAD };
                    self.mobs.get_mut(&id).unwrap().ready_at = self.clock + 0.3;
                    return true;
                }
                let gunner = ids[turn];
                if live.contains(&gunner) && !self.disabled(gunner) && self.mobs[&gunner].stagger_until <= self.clock {
                    let target = self.spread_foe(gunner, foe);
                    let to = self.foe_pos(target).unwrap_or(self.players[&hero].pos);
                    let pos = self.mobs[&gunner].pos;
                    if clear_line(self.land.as_ref(), pos, to) && dist2d(pos, to) <= 32.0 {
                        // Fixed target from shoulder to release, including while the hero moves.
                        self.windup(gunner, target, to);
                    }
                }
                let c = self.firing_line.crews.get_mut(&id).unwrap();
                c.first_shot = false;
                c.stage = Stage::Volley { turn: turn + 1, next: self.clock + SHOT_GAP };
                if first { self.events.push(Event::Emote { id, emote: 25 }); }
                true
            }
        }
    }

    pub(crate) fn firing_windup(&mut self, id: Id, to: Vec3) {
        let gunner = self.firing_line.owners.contains_key(&id);
        let name = self.mobs.get(&id).and_then(|m| m.pattern.map(|(k, _)| self.content.mobs[m.def].patterns[k].name.as_str()));
        if !gunner && !(self.is_firing_captain(id) && matches!(name, Some("pistol shot" | "harpoon"))) { return; }
        self.firing_line.aims.insert(id, to);
        self.firing_line.shouldered.remove(&id);
        if name == Some("pistol shot") {
            self.firing_line.drawn.insert(id);
            self.events.push(Event::MobGear { id });
            self.events.push(Event::Sound { id, sound: 700 });
        }
    }

    fn firing_shoulder(&mut self, id: Id) {
        if !self.firing_line.aims.contains_key(&id) || self.firing_line.shouldered.contains(&id) { return; }
        let Some(MobState::Windup { t, .. }) = self.mobs.get(&id).map(|m| &m.state) else { return };
        let a = self.attack_now(id);
        if *t < 0.6 || a.spell != 8682 { return; }
        let secs = (a.windup - *t).max(0.1);
        self.firing_line.shouldered.insert(id);
        // Native loading hands first, then the gun held to the shoulder until release.
        self.events.push(Event::Clip { id, clip: 48, take: 0, from: None, secs, at: 0.0, body: ClipBody::Full });
    }

    pub(crate) fn firing_release(&mut self, id: Id) -> bool {
        let Some(at) = self.firing_line.aims.remove(&id) else { return false };
        let a = self.attack_now(id);
        let m = &self.mobs[&id];
        let pos = m.pos;
        let hook = m.pattern.is_some_and(|(k, _)| self.content.mobs[m.def].patterns[k].name == "harpoon");
        let land_at = self.clock + dist2d(pos, at) / a.speed.max(1.0);
        self.events.push(Event::CastEnd { caster: id });
        self.events.push(Event::Missile { caster: id, spell: a.spell, target: Target::Point(at) });
        if hook {
            self.events.push(Event::Clip { id, clip: 107, take: 0, from: None, secs: 0.65, at: 0.0, body: ClipBody::Full });
        }
        // No ground disk: load/shoulder, committed facing, release and sound carry the tell.
        self.missiles.push(combat::Missile { owner: id, from: pos, at, radius: a.radius,
            damage: a.damage, blockable: a.blockable, heavy: false, land_at, mark: 0, look: 0 });
        if hook { self.firing_line.hooks.insert((id, land_at.to_bits())); }
        true
    }

    pub(crate) fn firing_follow(&mut self, id: Id, land_at: f32) -> combat::Follow {
        if self.firing_line.hooks.remove(&(id, land_at.to_bits())) { combat::Follow::Pull { yards: 8.0 } }
        else { combat::Follow::None }
    }
}
