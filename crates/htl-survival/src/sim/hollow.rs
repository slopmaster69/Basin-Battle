//! The Staff of the Hollow Star (docs/priest-draft3.md): tend a living orb. The right click throws
//! a dark orb to the reticle (a totem with health); every second it hurts the foes and heals the
//! friends within its reach. What it deals fills it, what it gives drains it, and its reach and
//! size grow and shrink with its health. Foes can break it. The left click, held, is the Void
//! Tether: a cord to whatever is under the reticle, ramping up the longer it holds. On a foe it
//! hurts, on a friend it heals, and on the Priest's own orb it fills it and draws it toward them.
//!
//! The orb is drawn as a model rolling between where it was and where it's going (`OrbRoll`), on
//! an unseen body the blows find.

use super::*;
use crate::content::AbilityDef;

/// A Hollow Star out: its body (a totem), whose, which, and its look.
#[derive(Debug, Clone)]
pub(crate) struct Hollow {
    pub mob: Id,
    pub owner: Id,
    pub ability: usize,
    pub look: Id,
    pub next: f32,
    pub until: f32,
    /// Where its look last rolled to (a drag rolls it on from there).
    pub shown: Vec3,
}

impl Survival {
    /// The orb thrown to the reticle: its body, its health, its look. One of theirs at a time.
    pub(crate) fn throw_hollow(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(hd) = d.hollow.clone() else { return };
        let Some(def) = self.content.mob(&hd.mob) else { return };
        let range = AbilityDef::at(d.range, share);
        let at = self.aim_point(id, range);
        let at = self.ground_near(at[0], at[1], at[2] + 1.0);
        if let Some(k) = self.light.hollows.iter().position(|h| h.owner == id) {
            let old = self.light.hollows.remove(k);
            self.hollow_gone(&old);
        }
        let mid = self.spawn_mob(def, at, None, false);
        if let Some(m) = self.mobs.get_mut(&mid) {
            m.max_health = hd.most.round().max(1.0) as u32;
            m.health = hd.health.round().max(1.0) as u32;
        }
        // Its owner's: their own blows pass it by, as a totem's do.
        self.totems.push(abilities::Totem { mob: mid, owner: id, ability, until: now + d.secs.max(1.0), next: f32::MAX });
        let look = self.alloc();
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        }
        let lifted = [at[0], at[1], at[2] + 1.0];
        let from = self.players.get(&id).map_or(lifted, |p| [p.pos[0], p.pos[1], p.pos[2] + 1.3]);
        let scale = self.hollow_scale(&hd, hd.health);
        self.events.push(Event::OrbRoll { look, from, to: lifted, secs: 0.35, model: d.orb_model.clone(), scale });
        self.light.hollows.push(Hollow { mob: mid, owner: id, ability, look, next: now + 0.5, until: now + d.secs.max(1.0), shown: lifted });
    }

    /// Its size at this health.
    fn hollow_scale(&self, hd: &crate::content::HollowDef, health: f32) -> f32 {
        let t = (health / hd.most.max(1.0)).clamp(0.0, 1.0);
        hd.scale[0] + (hd.scale[1] - hd.scale[0]) * t
    }

    /// Its reach at this health.
    fn hollow_reach(hd: &crate::content::HollowDef, health: f32) -> f32 {
        let t = (health / hd.most.max(1.0)).clamp(0.0, 1.0);
        hd.reach[0] + (hd.reach[1] - hd.reach[0]) * t
    }

    /// Whether `mid` is a Hollow Star of `owner`'s.
    pub(crate) fn own_hollow(&self, mid: Id, owner: Id) -> bool {
        self.light.hollows.iter().any(|h| h.mob == mid && h.owner == owner)
    }

    /// The orbs pulse: hurting foes and healing friends within reach, filling and draining by it;
    /// one broken, spent or out of time goes.
    pub(crate) fn step_hollows(&mut self) {
        let now = self.clock;
        let mut k = 0;
        while k < self.light.hollows.len() {
            let h = self.light.hollows[k].clone();
            let alive = self.mobs.get(&h.mob).is_some_and(|m| m.health > 0);
            if !alive || now >= h.until || !self.players.contains_key(&h.owner) {
                self.light.hollows.remove(k);
                self.hollow_gone(&h);
                continue;
            }
            if now < h.next {
                k += 1;
                continue;
            }
            let d = self.content.abilities[h.ability].clone();
            let Some(hd) = d.hollow.clone() else {
                k += 1;
                continue;
            };
            let tick = if d.tick > 0.0 { d.tick } else { 1.0 };
            self.light.hollows[k].next = now + tick;
            let at = self.mobs[&h.mob].pos;
            let health = self.mobs[&h.mob].health as f32;
            let reach = Self::hollow_reach(&hd, health);
            // Foes: hurt, and the orb filled by what it dealt.
            let foes: Vec<Id> = self
                .mobs
                .iter()
                .filter(|(mid, m)| **mid != h.mob && m.health > 0 && !m.hidden && !self.content.mobs[m.def].totem && !self.content.mobs[m.def].critter && dist2d(m.pos, at) <= reach + 0.5 * self.content.mobs[m.def].scale)
                .map(|(id, _)| *id)
                .chain(self.players.iter().filter(|(p, x)| self.hostile(h.owner, **p) && !self.helpable(h.owner, **p) && !self.spared(h.owner, **p, h.ability) && x.health > 0 && dist2d(x.pos, at) <= reach + abilities::HERO_SIZE).map(|(p, _)| *p))
                .collect();
            let mut gained = 0.0;
            for f in foes {
                let before = self.unit_health(f);
                self.channel_hit(h.owner, f, h.ability, 1.0, hd.damage, 0.0);
                gained += before.saturating_sub(self.unit_health(f)) as f32;
            }
            // Friends (the owner too): healed, the orb drained by what it gave; and quicker in it.
            let friends: Vec<Id> = self.players.iter().filter(|(p, x)| x.health > 0 && (**p == h.owner || self.helpable(h.owner, **p)) && dist2d(x.pos, at) <= reach + abilities::HERO_SIZE).map(|(p, _)| *p).collect();
            let mut given = 0.0;
            for f in friends {
                let before = self.unit_health(f);
                let room = self.players.get(&f).map_or(0, |p| p.max_health.saturating_sub(p.health));
                if room > 0 {
                    self.heal_hero_by(f, (hd.heal * self.weakened(h.owner)).round() as u32, h.owner, &d.name);
                    given += self.unit_health(f).saturating_sub(before) as f32;
                }
                if !d.effects.is_empty() {
                    self.apply_effects(h.owner, f, &d.effects, 1.0, &d.name);
                }
            }
            self.hollow_fill(h.mob, gained - given);
            k += 1;
        }
    }

    fn unit_health(&self, id: Id) -> u32 {
        self.players.get(&id).map(|p| p.health).or_else(|| self.mobs.get(&id).map(|m| m.health)).unwrap_or(0)
    }

    /// The orb's health by `by` (up to its most; at 0 it's spent), its size with it.
    pub(crate) fn hollow_fill(&mut self, mob: Id, by: f32) {
        let Some(k) = self.light.hollows.iter().position(|h| h.mob == mob) else { return };
        let h = self.light.hollows[k].clone();
        let Some(hd) = self.content.abilities[h.ability].hollow.clone() else { return };
        let Some(m) = self.mobs.get_mut(&mob) else { return };
        let health = (m.health as f32 + by).clamp(0.0, hd.most);
        m.health = health.round() as u32;
        let (hp, max) = (m.health, m.max_health);
        self.events.push(Event::Health { id: mob, health: hp, max });
        if hp == 0 {
            self.light.hollows.remove(k);
            self.hollow_gone(&h);
            return;
        }
        let scale = self.hollow_scale(&hd, health);
        let model = self.content.abilities[h.ability].orb_model.clone();
        self.events.push(Event::OrbRoll { look: h.look, from: h.shown, to: h.shown, secs: 0.2, model, scale });
        self.events.push(Event::PriestLook { player: h.owner, kind: "hollow", n: hp, on: Some(mob), a: Self::hollow_reach(&hd, health), b: max as f32 });
    }

    /// The tether on the orb draws it toward its owner: `yards` this tick.
    pub(crate) fn drag_hollow(&mut self, owner: Id, mob: Id, yards: f32) {
        let Some(k) = self.light.hollows.iter().position(|h| h.mob == mob && h.owner == owner) else { return };
        let Some(me) = self.players.get(&owner).map(|p| p.pos) else { return };
        let Some(at) = self.mobs.get(&mob).map(|m| m.pos) else { return };
        let dist = dist2d(at, me);
        if dist <= 2.0 {
            return;
        }
        let step = yards.min(dist - 2.0);
        let f = facing(at, me);
        let to = self.ground_near(at[0] + f.cos() * step, at[1] + f.sin() * step, at[2] + 1.0);
        if self.land.blocked_near(to[0], to[1], to[2]) {
            return;
        }
        if let Some(m) = self.mobs.get_mut(&mob) {
            m.pos = to;
        }
        self.events.push(Event::MobStop { id: mob, pos: to, facing: f });
        let h = self.light.hollows[k].clone();
        let lifted = [to[0], to[1], to[2] + 1.0];
        let hd = self.content.abilities[h.ability].hollow.clone();
        let health = self.mobs.get(&mob).map_or(0.0, |m| m.health as f32);
        let scale = hd.as_ref().map_or(1.0, |hd| self.hollow_scale(hd, health));
        let model = self.content.abilities[h.ability].orb_model.clone();
        self.events.push(Event::OrbRoll { look: h.look, from: h.shown, to: lifted, secs: 0.25, model, scale });
        self.light.hollows[k].shown = lifted;
    }

    fn hollow_gone(&mut self, h: &Hollow) {
        self.events.push(Event::OrbGone { look: h.look });
        self.remove_totem(h.mob);
        self.events.push(Event::PriestLook { player: h.owner, kind: "hollow", n: 0, on: None, a: 0.0, b: 0.0 });
    }

    // --- the Void Tether ---------------------------------------------------------------------------

    /// The tether begins (the left click held): to the first body along the reticle, friend or
    /// foe, or the caster's own orb if that's nearer. Nothing there: nothing spent.
    pub(crate) fn start_tether(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let (o, dir) = self.reticle(id);
        let t = self.trace_with(o, dir, range + 3.0, 0.8, id, true);
        let me = self.players[&id].pos;
        let mut best = t.unit.filter(|u| self.unit_pos(*u).is_some_and(|at| dist2d(at, me) <= range + 1.0)).map(|u| (u, t.dist));
        // The caster's own orb: their blows pass it by, so the trace never meets it.
        for h in self.light.hollows.iter().filter(|h| h.owner == id) {
            let Some(at) = self.mobs.get(&h.mob).map(|m| [m.pos[0], m.pos[1], m.pos[2] + 1.0]) else { continue };
            let rel = [at[0] - o[0], at[1] - o[1], at[2] - o[2]];
            let ahead = rel[0] * dir[0] + rel[1] * dir[1] + rel[2] * dir[2];
            if ahead <= 0.0 || ahead > range + 1.0 {
                continue;
            }
            let side = [rel[0] - dir[0] * ahead, rel[1] - dir[1] * ahead, rel[2] - dir[2] * ahead];
            let off = (side[0] * side[0] + side[1] * side[1] + side[2] * side[2]).sqrt();
            if off <= 1.6 && best.is_none_or(|(_, d)| ahead < d) {
                best = Some((h.mob, ahead));
            }
        }
        let Some((target, _)) = best else {
            if let Some(p) = self.players.get_mut(&id) {
                p.cooldowns.remove(&self.content.base_of(ability));
            }
            self.set_action(id, Action::Ready);
            return;
        };
        let tick = d.tick.max(0.2);
        self.set_action(id, Action::Channel { ability, target, until: now + d.secs, next: now + tick, share });
        self.events.push(Event::Channel { caster: id, target: Some(target), spell: d.spell, ms: (d.secs * 1000.0) as u32 });
        self.events.push(Event::Auras { id });
    }


    /// A tether's tick on what it holds, `held` seconds in: a foe hurt, a friend healed, the
    /// owner's orb filled and drawn in, each ramping up over its `ramp`.
    pub(crate) fn tether_tick(&mut self, id: Id, target: Id, ability: usize, tick: f32, held: f32) {
        let d = self.content.abilities[ability].clone();
        let Some(td) = d.tether.clone() else { return };
        let t = (held / td.ramp.max(0.01)).clamp(0.0, 1.0);
        if self.own_hollow(target, id) {
            self.hollow_fill(target, td.fill * tick);
            self.drag_hollow(id, target, td.drag * tick);
            return;
        }
        if self.players.contains_key(&target) && self.helpable(id, target) {
            let per = (td.heal[0] + (td.heal[1] - td.heal[0]) * t) * tick;
            self.heal_hero_by(target, (per * self.weakened(id)).round() as u32, id, &d.name);
            return;
        }
        let per = (d.damage[0] + (d.damage[1] - d.damage[0]) * t) * tick;
        self.channel_hit(id, target, ability, 1.0, per, td.valor * tick);
    }
}
