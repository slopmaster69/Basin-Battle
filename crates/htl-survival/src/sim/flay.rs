//! Tension: the Shadow Priest's Mind Flay. A cord of shadow to the first foe along the reticle,
//! held up to `secs` while the caster walks slowly. It grows stronger the farther they stand
//! (its `damage` a second, from the low end up close to the high end at the taut zone), and from
//! `taut` yards to its range it's **taut**: a tighter look on the cord, a flare on the caster as
//! it tightens. Held taut a whole `taut_secs`, once a cast, it rings: a perfect. Past the range it
//! **snaps** by itself: the channel ends with a whip-crack and the caster flinches. Its key again
//! while taut whips it into the foe for `snap` times what the rest of the channel would have dealt
//! (at most `snap_cap`): the cash-in. Mind Blast cast on the flayed foe while the cord is taut
//! (its `on_taut`) is the other cash-in: at once on that foe, double, a perfect.
//!
//! Lag: a press is judged on the caster's own screen, at its arrival less their round trip (as far
//! back as `combat.max_rewind`): a cord taut on their screen as they pressed is taut.

use super::*;
use crate::content::AbilityDef;
use crate::land::clear_line;

/// A hero's flay (its ability and foe are its channel's): how taut its cord has been.
#[derive(Debug, Clone)]
pub(crate) struct Flay {
    /// Taut since then, unbroken (none: slack now).
    pub taut_since: Option<f32>,
    /// The last moment it was taut.
    pub last_taut: f32,
    /// Its perfect paid (once a cast).
    pub rang: bool,
}

impl Survival {
    /// A flay begins (its channel already set going), slack.
    pub(crate) fn start_flay(&mut self, id: Id) {
        self.flays.insert(id, Flay { taut_since: None, last_taut: f32::NEG_INFINITY, rang: false });
    }

    /// The hero's flay in hand: its channel, the ability, the foe, until when, its share.
    fn flaying(&self, id: Id) -> Option<(usize, Id, f32, f32, f32)> {
        match self.players.get(&id).map(|p| &p.action) {
            Some(Action::Channel { ability, target, until, next, share }) if self.content.abilities[*ability].flay.is_some() => Some((*ability, *target, *until, *next, *share)),
            _ => None,
        }
    }

    /// The foe of the hero's flay if its cord is taut on the hero's own screen now.
    pub(crate) fn taut_flay(&self, id: Id) -> Option<Id> {
        let (_, target, ..) = self.flaying(id)?;
        let f = self.flays.get(&id)?;
        let rewind = self.players.get(&id).map_or(0.0, |p| p.rtt.min(self.content.combat.max_rewind));
        let taut = f.taut_since.is_some() || self.clock - rewind <= f.last_taut + 1.0e-4;
        (taut && self.alive(target)).then_some(target)
    }

    /// Whether the hero is holding a flay (another key goes out of it).
    pub(crate) fn is_flaying(&self, id: Id) -> bool {
        self.flaying(id).is_some()
    }

    /// The cord's distance, its look as it tightens and slackens, the ring of a taut second, its
    /// ticks, and its end: run out, lost from sight, or snapped past its range.
    pub(crate) fn step_flay(&mut self, id: Id) {
        let now = self.clock;
        let Some((ability, target, until, next, share)) = self.flaying(id) else { return };
        let d = self.content.abilities[ability].clone();
        let Some(f) = d.flay.clone() else { return };
        let range = AbilityDef::at(d.range, share);
        let pos = self.players[&id].pos;
        let at = self.unit_pos(target).filter(|_| self.alive(target));
        let Some(at) = at.filter(|a| clear_line(self.land.as_ref(), pos, *a)) else {
            self.let_go(id);
            return;
        };
        let dist = dist2d(pos, at);
        if dist > range {
            self.overstretched(id, at, &f);
            return;
        }
        if now >= until {
            self.let_go(id);
            return;
        }
        // Taut or slack: the look of the cord, and the ring of a whole taut second.
        let taut = dist >= f.taut;
        let was = self.flays.get(&id).and_then(|x| x.taut_since);
        let ms = ((until - now) * 1000.0).max(0.0) as u32;
        match (taut, was) {
            (true, None) => {
                if let Some(x) = self.flays.get_mut(&id) {
                    x.taut_since = Some(now);
                }
                let spell = if f.taut_spell != 0 { f.taut_spell } else { d.spell };
                self.events.push(Event::Channel { caster: id, target: Some(target), spell, ms });
                if f.taut_kit != 0 {
                    self.events.push(Event::Visual { id, kit: f.taut_kit, by: None });
                }
            }
            (false, Some(_)) => {
                if let Some(x) = self.flays.get_mut(&id) {
                    x.taut_since = None;
                }
                self.events.push(Event::Channel { caster: id, target: Some(target), spell: d.spell, ms });
            }
            _ => {}
        }
        if taut {
            let ring = self.flays.get_mut(&id).and_then(|x| {
                x.last_taut = now;
                let since = x.taut_since?;
                (!x.rang && now - since + 1.0e-4 >= f.taut_secs).then(|| x.rang = true)
            });
            if ring.is_some() {
                let k = self.content.checks.clone();
                self.gain_valor(id, k.perfect_valor);
                if k.perfect_kit != 0 {
                    self.events.push(Event::Visual { id, kit: k.perfect_kit, by: None });
                }
                self.events.push(Event::Perfect { player: id, ability });
            }
        }
        if now < next {
            return;
        }
        let tick = d.tick.max(0.2);
        if let Some(p) = self.players.get_mut(&id) {
            p.action = Action::Channel { ability, target, until, next: next + tick, share };
        }
        let per = f.dps(d.damage, dist) * tick;
        self.flay_blow(id, target, ability, share, per, tick);
    }

    /// One blow of the cord on its foe (and on the foes near it, a share, with a spill), Valor
    /// `valor` of a hit's.
    fn flay_blow(&mut self, id: Id, target: Id, ability: usize, share: f32, damage: f32, valor: f32) {
        let d = self.content.abilities[ability].clone();
        let Some(f) = d.flay else { return };
        let at = self.unit_pos(target);
        self.channel_hit(id, target, ability, share, damage, valor);
        let Some(at) = at.filter(|_| f.spill > 0.0) else { return };
        let (mobs, heroes) = self.in_wedge(id, ability, at, 0.0, 360.0, f.spill_radius);
        for u in mobs.into_iter().chain(heroes).filter(|u| *u != target) {
            if self.alive(u) {
                self.channel_hit(id, u, ability, share, damage * f.spill, valor * f.spill);
            }
        }
    }

    /// Its key again while it's held: taut (on the hero's screen), the cord whips into the foe for
    /// twice what's left of it; slack, it's simply let go.
    pub(crate) fn press_flay(&mut self, id: Id) {
        let now = self.clock;
        let Some((ability, _, until, _, share)) = self.flaying(id) else { return };
        let taut = self.taut_flay(id);
        self.let_go(id);
        let Some(target) = taut else { return };
        let d = self.content.abilities[ability].clone();
        let Some(f) = d.flay.clone() else { return };
        let left = (until - now).max(0.0);
        let dist = self.unit_pos(target).zip(self.players.get(&id).map(|p| p.pos)).map_or(f.taut, |(a, b)| dist2d(a, b));
        let damage = (f.snap * f.dps(d.damage, dist.max(f.taut)) * left).min(f.snap_cap);
        if f.whip_spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: f.whip_spell, target: Target::Unit(target) });
        }
        if damage > 0.0 {
            self.flay_blow(id, target, ability, share, damage, 1.0);
        }
    }

    /// Stretched past its range: the cord snaps back with a crack, and the caster flinches.
    fn overstretched(&mut self, id: Id, from: Vec3, f: &crate::content::FlayDef) {
        self.let_go(id);
        if f.snap_kit != 0 {
            self.events.push(Event::Visual { id, kit: f.snap_kit, by: None });
        }
        if f.flinch > 0.0 {
            let now = self.clock;
            self.set_action(id, Action::Stagger { until: now + f.flinch });
            let lean = self.content.combat.lean_light;
            self.events.push(Event::React { id, anim: combat::WOUND_ANIM, from, lean, hold: f.flinch, fit: 0.0 });
        }
    }

    /// The flay ends (its look off, the hero free).
    fn let_go(&mut self, id: Id) {
        self.end_ability_action(id);
        self.set_action(id, Action::Ready);
    }

    /// Mind Blast on the foe of a taut cord: at once on that foe, `on_taut` times as hard, the
    /// cord snapping into them with black lightning: a perfect (Valor, the burst everyone sees).
    pub(crate) fn snap_blast(&mut self, id: Id, ability: usize, target: Id) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(pos) = self.players.get(&id).map(|p| p.pos) else { return };
        let Some(at) = self.unit_pos(target) else { return };
        self.end_veil(id);
        if let Some(p) = self.players.get_mut(&id) {
            p.combat_at = now;
            p.last_cast = Some((ability, now));
            p.facing = facing(pos, at);
        }
        self.spend_charge(id, ability, 1.0);
        let k = self.content.checks.clone();
        self.gain_valor(id, k.perfect_valor);
        if k.perfect_kit != 0 {
            self.events.push(Event::Visual { id, kit: k.perfect_kit, by: None });
        }
        self.events.push(Event::Perfect { player: id, ability });
        if d.clip != 0 {
            self.events.push(Event::Clip { id, clip: d.clip, take: d.take, from: d.from.clone(), secs: super::abilities::CAST_RECOVERY + 0.2, at: 0.0 });
        }
        // The dark star on them, and the black lightning of the cord snapping into them.
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(target) });
        if d.full_spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.full_spell, target: Target::Unit(target) });
        }
        let damage = AbilityDef::at(d.damage, 1.0) * d.on_taut.max(1.0);
        if self.mobs.contains_key(&target) {
            self.ability_on_mob(id, target, ability, 1.0, damage, pos, abilities::HitKind::Projectile, d.spell);
        } else {
            let dmg = self.ability_damage(id, target, ability, damage);
            self.strike_fair_as(id, target, dmg, pos, true, false, false, Blow::Arrow(d.spell), None, combat::Follow::Ability { ability, share: 1.0 }, abilities::HitKind::Projectile);
        }
        self.set_action(id, Action::Cast { t: 0.0, total: super::abilities::CAST_RECOVERY, ability });
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Log(format!("{} snaps {} into {}", self.players[&id].name, d.name, self.unit_name(target))));
    }
}
