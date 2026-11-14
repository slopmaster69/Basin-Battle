//! Dominate Mind (docs/priest-draft4.md): the creature the Priest's bolt strikes is taken for the
//! ability's `secs` (the `dominated` effect). It is on the heroes' side while it lasts: no creature
//! or hero's blow finds it, it never takes up a fight against a hero, and it goes for the nearest
//! creature of the foe's side with its own blow, as hard and as often as it strikes a hero. With
//! nothing near to fight it follows its master. A boss, a totem or a creature in an unstoppable
//! blow can't be taken (the rules every control shares). Free again, it turns on whoever is near,
//! its master first if it is the nearest.
//!
//! A hero taken (a duel, Siege) isn't a thrall: they're walked toward the Priest and can't act
//! (`sim::cc`).

use super::mobs::MobState;
use super::*;

/// A creature fighting for a hero.
#[derive(Debug, Clone, PartialEq)]
pub struct Thrall {
    /// Whose will it is.
    pub owner: Id,
    /// When it is its own again.
    pub until: f32,
    /// Its next blow is allowed at.
    pub next_blow: f32,
}

/// How far from itself a thrall looks for something to fight (yards).
pub const THRALL_SIGHT: f32 = 14.0;
/// With nothing to fight it keeps this close to its master (yards).
const THRALL_FOLLOW: f32 = 4.0;
/// The first blow waits this long after it is taken (seconds).
const THRALL_FIRST: f32 = 0.6;

impl Survival {
    /// `id` is taken by `owner` for `secs`.
    pub(crate) fn take_thrall(&mut self, id: Id, owner: Id, secs: f32) {
        // Whatever it was winding up is lost (its ground mark with it).
        self.stagger_mob(id, 0.05, false);
        let now = self.clock;
        let Some(m) = self.mobs.get_mut(&id) else { return };
        m.thrall = Some(Thrall { owner, until: now + secs, next_blow: now + THRALL_FIRST });
        m.friendly = true;
        m.state = MobState::Idle { wander_at: f32::MAX };
        m.threat.clear();
        m.turn_on = None;
        m.drawn_off = None;
        self.halt(id);
    }

    /// Its own again: back to its rules' side, at rest, to notice whoever is near.
    pub(crate) fn release_thrall(&mut self, id: Id) {
        let now = self.clock;
        let Some(m) = self.mobs.get_mut(&id) else { return };
        if m.thrall.take().is_none() {
            return;
        }
        m.friendly = self.content.mobs[m.def].friendly;
        if m.health > 0 && !matches!(m.state, MobState::Dead { .. }) {
            m.state = MobState::Idle { wander_at: now };
        }
        self.halt(id);
    }

    /// Walk to `to`, unless it is already on its way to about there (a path asked again every
    /// step would be a walk restarted every step).
    fn thrall_goes(&mut self, id: Id, to: Vec3, speed: f32) {
        if self.mobs.get(&id).is_some_and(|m| m.path.last().is_some_and(|end| dist2d(*end, to) < 1.5)) {
            return;
        }
        self.go(id, to, speed);
    }

    /// The nearest creature a thrall at `from` would fight: alive, on the foe's side, not a boss,
    /// a totem, a dummy, a harmless critter or one lying in wait.
    fn thrall_foe(&self, me: Id, from: Vec3) -> Option<(Id, f32)> {
        self.mobs
            .iter()
            .filter(|(id, m)| {
                let d = &self.content.mobs[m.def];
                **id != me && m.health > 0 && !m.friendly && !m.hidden && m.rise_at.is_none() && !matches!(m.state, MobState::Dead { .. }) && !d.boss && !d.totem && !d.dummy && !d.critter
            })
            .map(|(id, m)| (*id, dist2d(from, m.pos)))
            .filter(|(_, d)| *d <= THRALL_SIGHT)
            .min_by(|a, b| a.1.total_cmp(&b.1))
    }

    /// Every thrall's turn: it strikes what is in reach, walks to what is near, or follows its
    /// master. Its master gone (fallen, left) ends it.
    pub(crate) fn step_thralls(&mut self) {
        let now = self.clock;
        let ids: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.thrall.is_some() && m.health > 0).map(|(id, _)| *id).collect();
        for id in ids {
            let Some(t) = self.mobs.get(&id).and_then(|m| m.thrall.clone()) else { continue };
            if !self.players.get(&t.owner).is_some_and(|p| p.health > 0) {
                self.cleanse(id, &["dominated"]);
                // (Gone with its cc somehow: set free all the same.)
                self.release_thrall(id);
                continue;
            }
            let (pos, def) = {
                let m = &self.mobs[&id];
                (m.pos, m.def)
            };
            let atk = self.content.mobs[def].attack.clone();
            let run = self.content.mobs[def].run_speed;
            match self.thrall_foe(id, pos) {
                Some((foe, dist)) => {
                    let foe_pos = self.mobs[&foe].pos;
                    let reach = atk.range.max(1.5);
                    if dist > reach {
                        // Toward it, stopping a little inside its reach.
                        let step = (dist - reach * 0.8).max(0.0) / dist.max(0.01);
                        let to = [pos[0] + (foe_pos[0] - pos[0]) * step, pos[1] + (foe_pos[1] - pos[1]) * step, foe_pos[2]];
                        self.thrall_goes(id, to, run);
                        continue;
                    }
                    if !self.mobs[&id].path.is_empty() {
                        self.halt(id);
                    }
                    if let Some(m) = self.mobs.get_mut(&id) {
                        m.facing = facing(pos, foe_pos);
                    }
                    if now < t.next_blow {
                        continue;
                    }
                    let gap = (atk.windup + atk.recovery + atk.cooldown).max(1.0);
                    if let Some(th) = self.mobs.get_mut(&id).and_then(|m| m.thrall.as_mut()) {
                        th.next_blow = now + gap;
                    }
                    let damage = atk.damage;
                    let style = if atk.heavy { Blow::Heavy } else { Blow::Light };
                    self.events.push(Event::Swing { attacker: id, victim: Some(foe), damage, outcome: Outcome::Hit, anim: true, style });
                    self.hurt_mob_as(foe, t.owner, damage, "Dominate Mind");
                }
                None => {
                    // Nothing to fight: it stays by its master.
                    let Some(to) = self.unit_pos(t.owner) else { continue };
                    if dist2d(pos, to) > THRALL_FOLLOW + 2.0 {
                        self.thrall_goes(id, to, run);
                    } else if !self.mobs[&id].path.is_empty() && dist2d(pos, to) <= THRALL_FOLLOW {
                        self.halt(id);
                    }
                }
            }
        }
    }
}
