//! Penance (docs/priest-draft3.md): the timing, while you fight. Up to `targets` bodies in front
//! of the Priest are picked as the key goes, hurt friends and foes taking turns, and a ring closes
//! on each in turn. The key pressed as a ring closes sends a bolt of light to that body (healing a
//! friend, hurting a foe), harder dead on the beat. A ring let pass is that bolt lost; the rest
//! still come. Every ring landed is the perfect.
//!
//! The Priest isn't held by it: they move, roll and strike between the rings (it's no stance and
//! no channel), so the skill is keeping time while fighting, never swinging the camera about.
//!
//! Lag: the rings go to the caster's add-on as Penance starts (`PriestLook "penance"`, one a ring,
//! with how long until it closes), so their screen shows each a one-way trip late and their press
//! reaches the server another trip later; as for the rhythm check, a press is judged as it arrives
//! (the two trips cancel), and a ring waits out its window and their round trip before it's lost.

use super::*;

/// A Penance under way: whose, which, and its rings (when each closes, on whom, whether a friend,
/// and how it went: none yet, landed, or lost).
#[derive(Debug, Clone)]
pub(crate) struct Penance {
    pub owner: Id,
    pub ability: usize,
    pub rings: Vec<Ring>,
}

#[derive(Debug, Clone)]
pub(crate) struct Ring {
    pub at: f32,
    pub on: Id,
    pub friend: bool,
    pub landed: Option<bool>,
}

impl Survival {
    /// The bodies Penance can pick in front of `id`: (who, a friend, its angle off the reticle),
    /// friends only when they're hurt.
    fn penance_bodies(&self, id: Id, range: f32, cone: f32) -> Vec<(Id, bool, f32)> {
        let Some(me) = self.players.get(&id) else { return Vec::new() };
        let (yaw, _) = self.aim_of(id);
        let eye = [me.pos[0], me.pos[1], me.pos[2] + 1.6];
        let mut out = Vec::new();
        let consider = |who: Id, at: Vec3, friend: bool, out: &mut Vec<(Id, bool, f32)>| {
            let d = dist2d(me.pos, at);
            if d > range || d < 0.3 {
                return;
            }
            let off = angle_between(yaw, facing(me.pos, at));
            if off > cone / 2.0 {
                return;
            }
            // In sight: nothing solid between the eyes and their chest.
            let chest = [at[0], at[1], at[2] + 1.2];
            let to = [chest[0] - eye[0], chest[1] - eye[1], chest[2] - eye[2]];
            let len = (to[0] * to[0] + to[1] * to[1] + to[2] * to[2]).sqrt().max(0.01);
            let t = self.wall_trace(eye, [to[0] / len, to[1] / len, to[2] / len], len, id);
            if t.solid && t.dist < len - 0.6 {
                return;
            }
            out.push((who, friend, off));
        };
        for (h, p) in &self.players {
            if *h == id || p.health == 0 {
                continue;
            }
            if self.helpable(id, *h) {
                if p.health < p.max_health {
                    consider(*h, p.pos, true, &mut out);
                }
            } else if self.hostile(id, *h) && !self.immune(*h) {
                consider(*h, p.pos, false, &mut out);
            }
        }
        for (m, x) in &self.mobs {
            let def = &self.content.mobs[x.def];
            if x.health == 0 || x.hidden || def.totem || def.critter || self.own_totem(*m, id) || self.spirit_body(*m) {
                continue;
            }
            consider(*m, x.pos, false, &mut out);
        }
        out.sort_by(|a, b| a.2.total_cmp(&b.2));
        out
    }

    /// The order the rings go in: friends and foes taking turns (whichever is nearest the reticle
    /// first), round the bodies again when there are fewer than the beats.
    fn penance_order(bodies: &[(Id, bool, f32)], n: usize, beats: u32) -> Vec<(Id, bool)> {
        let picked: Vec<(Id, bool, f32)> = {
            let mut friends: Vec<_> = bodies.iter().filter(|b| b.1).cloned().collect();
            let mut foes: Vec<_> = bodies.iter().filter(|b| !b.1).cloned().collect();
            let mut out = Vec::new();
            let mut friend_turn = bodies.first().is_some_and(|b| b.1);
            while out.len() < n && (!friends.is_empty() || !foes.is_empty()) {
                let from = if (friend_turn && !friends.is_empty()) || foes.is_empty() { &mut friends } else { &mut foes };
                out.push(from.remove(0));
                friend_turn = !friend_turn;
            }
            out
        };
        if picked.is_empty() {
            return Vec::new();
        }
        (0..beats as usize).map(|k| (picked[k % picked.len()].0, picked[k % picked.len()].1)).collect()
    }

    /// Penance goes: its bodies picked, its rings sent. False: no one in front of the caster
    /// (nothing spent).
    pub(crate) fn start_penance(&mut self, id: Id, ability: usize) -> bool {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(pd) = d.penance.clone() else { return false };
        let bodies = self.penance_bodies(id, d.range[1], pd.cone);
        let order = Self::penance_order(&bodies, pd.targets.max(1) as usize, pd.beats.max(1));
        if order.is_empty() {
            self.refuse(id, "No one in front of you", cues::Vocal::NoTarget);
            return false;
        }
        let rings: Vec<Ring> = order.iter().enumerate().map(|(k, (on, friend))| Ring { at: now + pd.first + pd.gap * k as f32, on: *on, friend: *friend, landed: None }).collect();
        // A Penance of theirs still running ends: its rings are lost.
        self.light.penances.retain(|x| x.owner != id);
        for (k, r) in rings.iter().enumerate() {
            self.events.push(Event::PriestLook { player: id, kind: "penance", n: k as u32 + 1, on: Some(r.on), a: r.at - now, b: if r.friend { 1.0 } else { 0.0 } });
        }
        self.light.penances.push(Penance { owner: id, ability, rings });
        true
    }

    /// The key again during Penance: a press for the first ring not yet settled. Within its window,
    /// its bolt goes; early, that ring is lost.
    pub(crate) fn press_penance(&mut self, id: Id) {
        let now = self.clock;
        let Some(k) = self.light.penances.iter().position(|x| x.owner == id) else { return };
        let ability = self.light.penances[k].ability;
        let Some(pd) = self.content.abilities[ability].penance.clone() else { return };
        let Some(r) = self.light.penances[k].rings.iter().position(|r| r.landed.is_none()) else { return };
        let at = self.light.penances[k].rings[r].at;
        let off = (now - at).abs();
        // Well before its ring is near, a press is nothing (no penalty for a stray press).
        if now < at - pd.window - 0.35 {
            return;
        }
        if off > pd.window + 1.0e-4 {
            self.penance_ring(k, r, false, false);
            return;
        }
        let critical = off <= pd.window * self.content.checks.crit_share + 1.0e-4;
        self.penance_ring(k, r, true, critical);
    }

    /// Ring `r` of Penance `k` settled: landed (its bolt to its body) or lost; the last settled,
    /// the Penance is over (every ring landed: the perfect).
    fn penance_ring(&mut self, k: usize, r: usize, landed: bool, critical: bool) {
        let x = self.light.penances[k].clone();
        let ring = x.rings[r].clone();
        self.light.penances[k].rings[r].landed = Some(landed);
        let d = self.content.abilities[x.ability].clone();
        let grade = if !landed { "fail" } else if critical { "perfect" } else { "good" };
        self.events.push(Event::PriestLook { player: x.owner, kind: "penance_hit", n: r as u32 + 1, on: Some(ring.on), a: if landed { 1.0 } else { 0.0 }, b: if critical { 1.0 } else { 0.0 } });
        if landed && self.alive(ring.on) {
            self.penance_bolt(x.owner, x.ability, ring.on, ring.friend, critical);
        } else if !landed {
            self.check_result(x.owner, x.ability, grade);
        }
        let done = self.light.penances[k].rings.iter().all(|r| r.landed.is_some());
        if !done {
            return;
        }
        let all = self.light.penances[k].rings.iter().all(|r| r.landed == Some(true));
        self.light.penances.remove(k);
        if all {
            let c = self.content.checks.clone();
            self.gain_valor(x.owner, c.perfect_valor);
            let kit = self.perfect_kit_of(x.ability);
            if kit != 0 {
                self.events.push(Event::Visual { id: x.owner, kit, by: None });
            }
            self.events.push(Event::Perfect { player: x.owner, ability: x.ability });
            self.events.push(Event::CheckResult { player: x.owner, ability: x.ability, grade: "full" });
            if d.burst_spell != 0 {
                if let Some(at) = self.unit_pos(ring.on) {
                    self.events.push(Event::Missile { caster: x.owner, spell: d.burst_spell, target: Target::Point(at) });
                }
            }
        }
    }

    /// A ring landed: a bolt of light to its body, healing a friend, hurting a foe.
    fn penance_bolt(&mut self, owner: Id, ability: usize, on: Id, friend: bool, critical: bool) {
        let d = self.content.abilities[ability].clone();
        let Some(pd) = d.penance.clone() else { return };
        // The hands cast it, taking turns (the upper body only: the legs keep running).
        if !d.beat_clips.is_empty() {
            let n = self.light.penances.iter().find(|x| x.owner == owner).map_or(0, |x| x.rings.iter().filter(|r| r.landed.is_some()).count());
            let clip = d.beat_clips[n % d.beat_clips.len()];
            self.events.push(Event::Clip { id: owner, clip, take: 0, from: None, secs: pd.gap.max(0.3), at: 0.1, body: ClipBody::Upper });
        }
        self.events.push(Event::Missile { caster: owner, spell: d.spell, target: Target::Unit(on) });
        // Dead on: the holy flash on a friend (a foe's comes with the blow, `channel_hit`).
        if critical && friend && d.impact_kit != 0 {
            self.events.push(Event::Visual { id: on, kit: d.impact_kit, by: Some(owner) });
        }
        if friend {
            let base = if critical { pd.crit } else { d.mend };
            let amount = (base * self.weakened(owner)).round() as u32;
            self.heal_hero_by(on, amount, owner, &d.name);
            self.events.push(Event::Missile { caster: owner, spell: abilities::HEAL_LOOK, target: Target::Unit(on) });
            self.gain_valor(owner, self.content.valor.ability / 3.0);
        } else {
            let base = if critical { pd.crit } else { d.damage[0] };
            self.channel_hit(owner, on, ability, 1.0, base, self.content.valor.ability / 3.0);
        }
    }

    /// Penance's every tick: rings let pass are lost (once their window and the caster's round trip
    /// are up), and a Penance whose caster has fallen ends.
    pub(crate) fn step_penances(&mut self) {
        let now = self.clock;
        let mut k = 0;
        while k < self.light.penances.len() {
            let x = &self.light.penances[k];
            if !self.players.get(&x.owner).is_some_and(|p| p.health > 0) {
                self.light.penances.remove(k);
                continue;
            }
            let Some(pd) = self.content.abilities[x.ability].penance.clone() else {
                self.light.penances.remove(k);
                continue;
            };
            let rewind = self.players[&x.owner].rtt.min(self.content.combat.max_rewind);
            let due = x.rings.iter().position(|r| r.landed.is_none() && now > r.at + pd.window + rewind);
            if let Some(r) = due {
                let before = self.light.penances.len();
                self.penance_ring(k, r, false, false);
                if self.light.penances.len() < before {
                    continue;
                }
                // (More may be due: look again next tick.)
            }
            k += 1;
        }
    }

    /// Which ring of `id`'s Penance is next, and when it closes (for the lab and the tests).
    pub fn penance_next(&self, id: Id) -> Option<(usize, f32, Id)> {
        let x = self.light.penances.iter().find(|x| x.owner == id)?;
        let r = x.rings.iter().position(|r| r.landed.is_none())?;
        Some((r, x.rings[r].at, x.rings[r].on))
    }
}
