//! The sweep: the stop check. A jet (`arc` wide, `range` long) starts at one side of the hero's
//! front and sweeps `sweep` degrees across it over `secs`, striking each foe it passes once. The
//! key pressed again stops it: the jet holds where it is for `stop_secs`, the foes in it take the
//! stop's effects (Cone of Cold's freeze), and a foe within `perfect_arc` of the jet's centre
//! makes it a perfect (`[checks]`: Valor, a burst). Not stopped, the sweep finishes and they're
//! only slowed. Early or late is graded by nature: the jet is simply a little to one side of
//! the foe, which anyone can see.
//!
//! The hero turns with the jet, so the cone look at the stop faces where it held. The jet's own
//! look while it sweeps is a stream of `tick_spell`'s missile to its reach, one every [`PULSE`]
//! seconds, which reads as a jet crossing the ground, until a client can draw a moving one.

use super::abilities::HERO_SIZE;
use super::*;
use crate::content::AbilityDef;

/// How often the jet's reach puffs while it sweeps.
const PULSE: f32 = 0.1;

impl Survival {
    /// The sweep begins from the hero's left, turning rightward across the front.
    pub(crate) fn start_sweep(&mut self, id: Id, ability: usize, yaw: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let from = yaw + d.sweep.to_radians() / 2.0;
        self.set_action(id, Action::Sweep { ability, from, since: now, until: now + d.secs.max(0.1), struck: Vec::new(), held: None });
        self.events.push(Event::Auras { id });
        self.pulse_sweep(id, ability, from);
    }

    /// Where the jet points now: the facing it swept to, or holds at.
    fn sweep_facing(d: &AbilityDef, from: f32, since: f32, now: f32, held: Option<f32>) -> f32 {
        let t = held.unwrap_or(now) - since;
        let share = (t / d.secs.max(0.1)).clamp(0.0, 1.0);
        from - d.sweep.to_radians() * share
    }

    /// One of the jet's missiles, to its reach.
    fn pulse_sweep(&mut self, id: Id, ability: usize, yaw: f32) {
        let d = &self.content.abilities[ability];
        if d.tick_spell == 0 {
            return;
        }
        let p = &self.players[&id];
        let r = d.range[0];
        let at = [p.pos[0] + yaw.cos() * r, p.pos[1] + yaw.sin() * r, p.pos[2] + 1.0];
        let spell = d.tick_spell;
        self.events.push(Event::Missile { caster: id, spell, target: Target::Point(at) });
    }

    /// The sweep's tick: the jet turns, strikes what it newly covers, and ends when its time
    /// (or its hold) is up.
    pub(crate) fn step_sweep(&mut self, id: Id, dt: f32) {
        let now = self.clock;
        let Some(Action::Sweep { ability, from, since, until, struck, held }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if now >= until {
            self.end_ability_action(id);
            // Back to ready, or the guard held meanwhile.
            self.finish_commitment(id);
            return;
        }
        let d = self.content.abilities[ability].clone();
        let yaw = Self::sweep_facing(&d, from, since, now, held);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = yaw;
        }
        if held.is_some() {
            return;
        }
        // A puff at the reach, now and then.
        let k = ((now - since) / PULSE).floor();
        if k > ((now - dt - since) / PULSE).floor() {
            self.pulse_sweep(id, ability, yaw);
        }
        // Whom the jet covers now and hasn't struck.
        let at = self.players[&id].pos;
        let (mobs, heroes) = self.in_wedge(id, ability, at, yaw, d.arc, d.range[0]);
        let fresh: Vec<Id> = mobs.iter().chain(&heroes).copied().filter(|u| !struck.contains(u)).collect();
        if fresh.is_empty() {
            return;
        }
        let damage = d.damage[0];
        for u in &fresh {
            if mobs.contains(u) {
                self.ability_on_mob(id, *u, ability, 1.0, damage, at, HitKind::Area, d.spell);
            } else {
                let dmg = self.ability_damage(id, *u, ability, damage);
                let area = Some(combat::Area::Arc { pos: at, facing: yaw, range: d.range[0], arc: d.arc });
                self.strike_fair_as(id, *u, dmg, at, false, false, false, Blow::Arrow(d.spell), area, combat::Follow::Ability { ability, share: 1.0 }, HitKind::Area);
            }
        }
        if let Some(p) = self.players.get_mut(&id) {
            if let Action::Sweep { struck, .. } = &mut p.action {
                struck.extend(fresh);
            }
        }
    }

    /// The key again: the jet stops where it is. The foes in it take the stop's effects; one
    /// at the jet's centre makes it a perfect.
    pub(crate) fn stop_sweep(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Sweep { ability, from, since, held, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if held.is_some() {
            return;
        }
        let d = self.content.abilities[ability].clone();
        let yaw = Self::sweep_facing(&d, from, since, now, None);
        let hold = d.stop_secs.max(0.1);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = yaw;
            if let Action::Sweep { held, until, .. } = &mut p.action {
                *held = Some(now);
                *until = now + hold;
            }
        }
        // The cone's own look, held where it stopped.
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        let at = self.players[&id].pos;
        let (mobs, heroes) = self.in_wedge(id, ability, at, yaw, d.arc, d.range[0]);
        let perfect = d.perfect_arc > 0.0
            && mobs.iter().map(|m| self.mobs[m].pos).chain(heroes.iter().map(|h| self.players[h].pos)).any(|pos| angle_between(yaw, facing(at, pos)) <= d.perfect_arc);
        if !d.stop_effects.is_empty() {
            let was = self.fx_from.replace(at);
            for u in mobs.iter().chain(&heroes).copied().collect::<Vec<_>>() {
                self.apply_effects(id, u, &d.stop_effects, 1.0, &d.name);
            }
            self.fx_from = was;
        }
        if perfect {
            let k = self.content.checks.clone();
            self.gain_valor(id, k.perfect_valor);
            if k.perfect_kit != 0 {
                self.events.push(Event::Visual { id, kit: k.perfect_kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
        }
    }

    /// The foes a wedge from `at` covers: creatures and (with friendly fire) hostile heroes.
    pub(crate) fn in_wedge(&self, owner: Id, ability: usize, at: Vec3, yaw: f32, arc: f32, range: f32) -> (Vec<Id>, Vec<Id>) {
        let inside = |pos: Vec3, size: f32| -> bool {
            let dd = dist2d(pos, at);
            // (`angle_between` answers in degrees, as the arcs are written.)
            dd <= range + size && (pos[2] - at[2]).abs() <= 4.0 && (dd < 0.8 || angle_between(yaw, facing(at, pos)) <= arc / 2.0)
        };
        let mobs: Vec<Id> = self.mobs.iter().filter(|(mid, m)| **mid != owner && m.health > 0 && !m.hidden && inside(m.pos, 0.5 * self.content.mobs[m.def].scale) && !self.own_totem(**mid, owner) && !self.spirit_body(**mid)).map(|(id, _)| *id).collect();
        let heroes: Vec<Id> = if self.content.pvp.friendly_fire {
            self.players.iter().filter(|(h, p)| self.hostile(owner, **h) && !self.spared(owner, **h, ability) && p.health > 0 && inside(p.pos, HERO_SIZE)).map(|(h, _)| *h).collect()
        } else {
            Vec::new()
        };
        (mobs, heroes)
    }
}
