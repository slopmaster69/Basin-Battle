//! Angelic Feather (docs/priest-draft3.md): a feather of light laid at the reticle, or at the
//! Priest's feet. The first hero to touch it, the Priest or a friend, runs faster for a moment and
//! is freed of slows and roots. Nobody is moved who didn't choose to step on it: a road out for
//! the Priest alone, or for a friend.

use super::*;

/// A feather on the ground: whose, which, where, until when, and its look.
#[derive(Debug, Clone)]
pub(crate) struct Feather {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub until: f32,
    pub look: Id,
}

impl Survival {
    /// A feather laid: at the reticle within the ability's range, else at the caster's feet. The
    /// caster's oldest goes when they'd have more than its most.
    pub(crate) fn lay_feather(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(fd) = d.feather.clone() else { return };
        let range = crate::content::AbilityDef::at(d.range, share);
        let at = if range > 0.0 { self.aim_point(id, range) } else { self.players[&id].pos };
        let at = self.ground_near(at[0], at[1], at[2] + 1.0);
        let mine: Vec<usize> = self.light.feathers.iter().enumerate().filter(|(_, f)| f.owner == id).map(|(k, _)| k).collect();
        if mine.len() as u32 >= fd.most.max(1) {
            let old = self.light.feathers.remove(mine[0]);
            self.feather_gone(&old);
        }
        let look = self.mark_unseen(fd.secs);
        if d.linger_spell != 0 {
            self.events.push(Event::AreaLook { id: look, caster: id, spell: d.linger_spell, pos: at, radius: fd.reach });
        }
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        }
        self.events.push(Event::PriestGround { kind: "feather", key: look, at, radius: fd.reach, secs: fd.secs });
        self.light.feathers.push(Feather { owner: id, ability, at, until: now + fd.secs, look });
    }

    /// A feather's looks taken off the ground.
    fn feather_gone(&mut self, f: &Feather) {
        self.unmark(f.look);
        self.events.push(Event::PriestGround { kind: "feather", key: f.look, at: f.at, radius: 0.0, secs: 0.0 });
    }

    /// Feathers: the first hero to touch one takes it; one whose time is up fades.
    pub(crate) fn step_feathers(&mut self) {
        let now = self.clock;
        let mut k = 0;
        while k < self.light.feathers.len() {
            let f = self.light.feathers[k].clone();
            if now >= f.until || !self.players.contains_key(&f.owner) {
                self.light.feathers.remove(k);
                self.feather_gone(&f);
                continue;
            }
            let d = self.content.abilities[f.ability].clone();
            let reach = d.feather.as_ref().map_or(1.5, |x| x.reach);
            // The nearest who may take it: its owner, or a hero they can help.
            let taker = self
                .players
                .iter()
                .filter(|(h, p)| p.health > 0 && (**h == f.owner || self.helpable(f.owner, **h)) && dist2d(p.pos, f.at) <= reach && (p.pos[2] - f.at[2]).abs() < 3.0)
                .min_by(|a, b| dist2d(a.1.pos, f.at).total_cmp(&dist2d(b.1.pos, f.at)))
                .map(|(h, _)| *h);
            let Some(h) = taker else {
                k += 1;
                continue;
            };
            self.light.feathers.remove(k);
            self.feather_gone(&f);
            self.cleanse(h, &["root", "snare"]);
            self.apply_effects(f.owner, h, &d.effects, 1.0, &d.name);
            if d.impact_kit != 0 {
                self.events.push(Event::Visual { id: h, kit: d.impact_kit, by: Some(f.owner) });
            }
            if d.burst_spell != 0 {
                self.events.push(Event::Missile { caster: f.owner, spell: d.burst_spell, target: Target::Unit(h) });
            }
            self.events.push(Event::Auras { id: h });
        }
    }
}
