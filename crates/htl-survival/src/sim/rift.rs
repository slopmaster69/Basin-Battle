//! Void Rift (docs/priest-draft3.md): gather, then implode. A wound torn in the ground at the
//! reticle grows for a moment; cords of shadow drag every foe in it toward its heart, a little
//! each second. The key again implodes it, harder for every foe it has caught; three or more is
//! the perfect. Left alone it collapses by itself, weakly. The question is when: wait for more,
//! or take what's there before they walk out.
//!
//! The pull on a hero is held to `hero_most` yards in all (no one is dragged across the map), and
//! a boss or an unstoppable foe holds its ground (`pull`'s own rule).

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::content::AbilityDef;

/// A rift open on the ground.
#[derive(Debug, Clone)]
pub(crate) struct Rift {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub since: f32,
    pub until: f32,
    pub next_tick: f32,
    /// Every foe it has had inside (the implosion's count).
    pub caught: BTreeSet<Id>,
    /// How far it has dragged each hero.
    pub dragged: BTreeMap<Id, f32>,
    pub look: Id,
}

/// The cords stay this long past each second's pull (they're drawn again on the next).
const CORD_SECS: f32 = 1.15;

impl Survival {
    /// The rift tears open at the reticle (within its range).
    pub(crate) fn open_rift(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(rd) = d.rift.clone() else { return };
        let range = AbilityDef::at(d.range, share);
        let at = self.aim_point(id, range);
        let at = self.ground_near(at[0], at[1], at[2] + 1.0);
        // One of theirs at a time.
        if let Some(k) = self.light.rifts.iter().position(|r| r.owner == id) {
            let old = self.light.rifts.remove(k);
            self.rift_gone(&old);
        }
        let life = d.linger.max(0.5);
        let look = self.mark_unseen(life + 0.5);
        if d.linger_spell != 0 {
            self.events.push(Event::AreaLook { id: look, caster: id, spell: d.linger_spell, pos: at, radius: rd.grow_to });
        }
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        }
        self.events.push(Event::PriestGround { kind: "rift", key: look, at, radius: d.radius[0], secs: life });
        let tick = if d.tick > 0.0 { d.tick } else { 1.0 };
        self.light.rifts.push(Rift { owner: id, ability, at, since: now, until: now + life, next_tick: now + tick.min(0.5), caught: BTreeSet::new(), dragged: BTreeMap::new(), look });
    }

    /// The rift of this key's ability the hero has open, if any (its key again implodes it).
    pub(crate) fn rift_of(&self, id: Id, base: usize) -> Option<usize> {
        self.light.rifts.iter().position(|r| r.owner == id && self.content.base_of(r.ability) == base)
    }

    /// How far a rift reaches now: from its `radius` to `grow_to` over `grow_secs`.
    fn rift_reach(&self, r: &Rift) -> f32 {
        let d = &self.content.abilities[r.ability];
        let Some(rd) = d.rift.as_ref() else { return d.radius[0] };
        let t = ((self.clock - r.since) / rd.grow_secs.max(0.01)).clamp(0.0, 1.0);
        d.radius[0] + (rd.grow_to - d.radius[0]) * t
    }

    /// The foes inside a rift now: creatures, and hostile heroes.
    fn in_rift(&self, r: &Rift, reach: f32) -> (Vec<Id>, Vec<Id>) {
        let at = r.at;
        let mobs: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(mid, m)| m.health > 0 && !m.hidden && !self.content.mobs[m.def].totem && dist2d(m.pos, at) <= reach + 0.5 * self.content.mobs[m.def].scale && !self.own_totem(**mid, r.owner) && !self.spirit_body(**mid))
            .map(|(id, _)| *id)
            .collect();
        let heroes: Vec<Id> = self
            .players
            .iter()
            .filter(|(h, p)| self.hostile(r.owner, **h) && !self.helpable(r.owner, **h) && !self.spared(r.owner, **h, r.ability) && p.health > 0 && dist2d(p.pos, at) <= reach + abilities::HERO_SIZE)
            .map(|(h, _)| *h)
            .collect();
        (mobs, heroes)
    }

    /// Rifts: each second they hurt and drag what's inside (cords drawn to each); one whose time
    /// is up collapses by itself.
    pub(crate) fn step_rifts(&mut self) {
        let now = self.clock;
        let mut k = 0;
        while k < self.light.rifts.len() {
            let r = self.light.rifts[k].clone();
            if !self.players.contains_key(&r.owner) {
                self.light.rifts.remove(k);
                self.rift_gone(&r);
                continue;
            }
            let d = self.content.abilities[r.ability].clone();
            let Some(rd) = d.rift.clone() else {
                self.light.rifts.remove(k);
                continue;
            };
            if now >= r.until {
                self.light.rifts.remove(k);
                self.rift_burst(&r, rd.collapse, false);
                continue;
            }
            if now >= r.next_tick {
                let tick = if d.tick > 0.0 { d.tick } else { 1.0 };
                self.light.rifts[k].next_tick = now + tick;
                let reach = self.rift_reach(&r);
                let (mobs, heroes) = self.in_rift(&r, reach);
                let per = (d.linger_damage * tick).round();
                for m in &mobs {
                    self.light.rifts[k].caught.insert(*m);
                    if per >= 1.0 {
                        self.channel_hit(r.owner, *m, r.ability, 1.0, per, 0.0);
                    }
                }
                for h in &heroes {
                    self.light.rifts[k].caught.insert(*h);
                    if per >= 1.0 && !self.immune(*h) {
                        self.channel_hit(r.owner, *h, r.ability, 1.0, per, 0.0);
                    }
                }
                // The drag: toward its heart, a hero no farther than their most.
                let was = self.fx_from.replace(r.at);
                for m in mobs.iter().copied() {
                    if self.alive(m) {
                        let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: rd.pull, ..Default::default() };
                        self.apply_effects(r.owner, m, &[pull], 1.0, &d.name);
                        self.events.push(Event::Tether { from: m, to: Target::Point([r.at[0], r.at[1], r.at[2] + 0.3]), secs: CORD_SECS });
                    }
                }
                for h in heroes.iter().copied() {
                    let gone = self.light.rifts[k].dragged.get(&h).copied().unwrap_or(0.0);
                    let left = (rd.hero_most - gone).max(0.0);
                    if left <= 0.05 || !self.alive(h) {
                        continue;
                    }
                    let step = rd.pull.min(left);
                    *self.light.rifts[k].dragged.entry(h).or_default() += step;
                    let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: step, ..Default::default() };
                    self.apply_effects(r.owner, h, &[pull], 1.0, &d.name);
                    self.events.push(Event::Tether { from: h, to: Target::Point([r.at[0], r.at[1], r.at[2] + 0.3]), secs: CORD_SECS });
                }
                self.fx_from = was;
                let n = self.light.rifts[k].caught.len() as u32;
                self.events.push(Event::PriestLook { player: r.owner, kind: "rift", n, on: None, a: reach, b: (r.until - now).max(0.0) });
            }
            k += 1;
        }
    }

    /// The key again: the rift implodes, harder for every foe it has caught.
    pub(crate) fn implode_rift(&mut self, id: Id, k: usize) {
        let r = self.light.rifts.remove(k);
        let d = self.content.abilities[r.ability].clone();
        let Some(rd) = d.rift.clone() else { return };
        let n = r.caught.iter().filter(|u| self.alive(**u)).count() as u32;
        let damage = (d.damage[0] + rd.per_foe * n as f32).min(d.damage[1]);
        let perfect = n >= rd.perfect_foes.max(1);
        self.rift_burst(&r, damage, true);
        if perfect {
            let c = self.content.checks.clone();
            self.gain_valor(id, c.perfect_valor * 2.0);
            let kit = self.perfect_kit_of(r.ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability: r.ability });
        }
        self.check_result(id, r.ability, if perfect { "perfect" } else { "good" });
    }

    /// A rift bursts where it is (imploded, or collapsing by itself), and its looks go.
    fn rift_burst(&mut self, r: &Rift, damage: f32, imploded: bool) {
        let d = self.content.abilities[r.ability].clone();
        let reach = self.rift_reach(r);
        let look = if imploded && d.full_spell != 0 { d.full_spell } else { d.burst_spell };
        if look != 0 {
            let m = self.mark_unseen(1.0);
            self.events.push(Event::AreaLook { id: m, caster: r.owner, spell: look, pos: r.at, radius: reach });
        }
        if let Some(b) = &d.burst {
            self.events.push(Event::GroundBurst { at: r.at, yaw: 0.0, shape: "ring", reach, arc: 360.0, width: 0.0, secs: b.secs, colour: b.colour.clone() });
        }
        if self.players.contains_key(&r.owner) && damage > 0.0 {
            self.area(r.owner, r.ability, r.at, reach, 1.0, damage, None);
        }
        self.rift_gone(r);
    }

    fn rift_gone(&mut self, r: &Rift) {
        self.unmark(r.look);
        self.events.push(Event::PriestGround { kind: "rift", key: r.look, at: r.at, radius: 0.0, secs: 0.0 });
        self.events.push(Event::PriestLook { player: r.owner, kind: "rift", n: 0, on: None, a: 0.0, b: 0.0 });
    }
}
