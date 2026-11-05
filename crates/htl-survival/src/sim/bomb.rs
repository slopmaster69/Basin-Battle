//! The bomb: the release check, on a foe. Living Bomb's bolt plants a bomb on what it hits. It
//! flares `bomb_flares` times over `bomb_secs` (a pulse on the foe each time), then bursts for
//! `damage`'s low end in `radius`, with `bomb_effects` (a burn). Press the key again **on a
//! flare** (within `[checks] window` of one) and it goes off then: the high end, `stop_effects`
//! too (a knockback), a perfect's Valor and burst on the caster. Pressed off a flare, it goes at
//! once, plain. A foe that dies with the bomb on it sets it off where it fell.
//!
//! Combustion (`spread_burns`) is here too: every burn on the foe it reaches catches on each foe
//! within that many yards, for as long as it had left.

use super::*;

/// A bomb on a foe.
#[derive(Debug, Clone)]
pub(crate) struct Bomb {
    pub owner: Id,
    pub ability: usize,
    pub on: Id,
    pub since: f32,
    /// Flares shown so far.
    pub flared: u32,
    /// Where it last was (for a foe gone or dead).
    pub at: Vec3,
}

impl Survival {
    /// Where a living unit is (none once it's dead or gone).
    fn living_pos(&self, id: Id) -> Option<Vec3> {
        self.mobs.get(&id).filter(|m| m.health > 0).map(|m| m.pos).or_else(|| self.players.get(&id).filter(|p| p.health > 0).map(|p| p.pos))
    }

    /// The bolt reached a foe: a bomb on it (one of this caster's at a time on it: a second
    /// replaces the first).
    pub(crate) fn plant_bomb(&mut self, owner: Id, ability: usize, on: Id) {
        let Some(at) = self.living_pos(on) else { return };
        let now = self.clock;
        self.bombs.retain(|b| !(b.owner == owner && b.on == on));
        self.bombs.push(Bomb { owner, ability, on, since: now, flared: 0, at });
        self.events.push(Event::Swing { attacker: owner, victim: Some(on), damage: 0, outcome: Outcome::Hit, anim: false, style: Blow::Arrow(self.content.abilities[ability].spell) });
    }

    /// When flare `k` (1 to `bomb_flares`) comes.
    fn flare_at(&self, b: &Bomb, k: u32) -> f32 {
        let d = &self.content.abilities[b.ability];
        b.since + d.bomb_secs * k as f32 / d.bomb_flares.max(1) as f32
    }

    /// The caster's bomb of this key's base ability that's ticking, if any.
    pub(crate) fn bomb_of(&self, id: Id, base: usize) -> Option<usize> {
        self.bombs.iter().position(|b| b.owner == id && self.content.base_of(b.ability) == base)
    }

    /// Bombs flare, and burst at their end (or where their foe fell).
    pub(crate) fn step_bombs(&mut self) {
        let now = self.clock;
        let mut k = 0;
        while k < self.bombs.len() {
            let b = self.bombs[k].clone();
            let d = self.content.abilities[b.ability].clone();
            let alive = self.living_pos(b.on);
            if let Some(at) = alive {
                self.bombs[k].at = at;
            }
            if alive.is_none() || !self.players.contains_key(&b.owner) || now >= b.since + d.bomb_secs {
                let b = self.bombs.remove(k);
                if self.players.contains_key(&b.owner) {
                    self.burst_bomb(&b, false);
                }
                continue;
            }
            // Its flares: the pulse on the foe, the moment to press.
            let next = b.flared + 1;
            if next <= d.bomb_flares && now + 1.0e-4 >= self.flare_at(&b, next) - self.content.checks.window {
                self.bombs[k].flared = next;
                if d.bomb_kit != 0 {
                    self.events.push(Event::Visual { id: b.on, kit: d.bomb_kit, by: None });
                }
            }
            k += 1;
        }
    }

    /// The key again: on a flare it's a perfect; off one, it goes at once, plain.
    pub(crate) fn press_bomb(&mut self, id: Id, k: usize) {
        let b = self.bombs.remove(k);
        let d = &self.content.abilities[b.ability];
        let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
        let t = self.clock - rewind;
        let w = self.content.checks.window;
        let perfect = (1..=d.bomb_flares).any(|f| (t - self.flare_at(&b, f)).abs() <= w + 1.0e-4);
        self.burst_bomb(&b, perfect);
    }

    /// The bomb goes off where its foe is.
    fn burst_bomb(&mut self, b: &Bomb, perfect: bool) {
        let d = self.content.abilities[b.ability].clone();
        let at = self.living_pos(b.on).unwrap_or(b.at);
        let share = if perfect { 1.0 } else { 0.0 };
        let damage = crate::content::AbilityDef::at(d.damage, share);
        let radius = d.radius[1];
        // The burst's look: `tick_spell`'s, a perfect's `full_spell`'s (its bolt is `spell`).
        let look = if perfect && d.full_spell != 0 { d.full_spell } else { d.tick_spell };
        if look != 0 {
            let m = self.mark_unseen(1.0);
            self.events.push(Event::AreaLook { id: m, caster: b.owner, spell: look, pos: at, radius });
        }
        self.area(b.owner, b.ability, at, radius, share, damage, None);
        let mut fx = d.bomb_effects.clone();
        if perfect {
            fx.extend(d.stop_effects.iter().cloned());
        }
        if !fx.is_empty() {
            let (mobs, heroes) = self.in_wedge(b.owner, b.ability, at, 0.0, 360.0, radius);
            let was = self.fx_from.replace(at);
            for u in mobs.into_iter().chain(heroes) {
                self.apply_effects(b.owner, u, &fx, 1.0, &d.name);
            }
            self.fx_from = was;
        }
        if perfect {
            let c = self.content.checks.clone();
            self.gain_valor(b.owner, c.perfect_valor);
            if c.perfect_kit != 0 {
                self.events.push(Event::Visual { id: b.owner, kit: c.perfect_kit, by: None });
            }
            self.events.push(Event::Perfect { player: b.owner, ability: b.ability });
        }
    }

    /// Combustion: every burn on `on` catches on each foe within `radius` of it, for as long as
    /// it had left.
    pub(crate) fn spread_burns(&mut self, owner: Id, ability: usize, on: Id, radius: f32) {
        let now = self.clock;
        let Some(at) = self.living_pos(on) else { return };
        let burns: Vec<crate::content::AbilityEffect> = self
            .cc_of(on)
            .iter()
            .filter(|c| c.kind == "burn" && c.until > now)
            .map(|c| crate::content::AbilityEffect { kind: "burn".into(), secs: [c.until - now, c.until - now], amount: c.amount, ..Default::default() })
            .collect();
        if burns.is_empty() {
            return;
        }
        let (mobs, heroes) = self.in_wedge(owner, ability, at, 0.0, 360.0, radius);
        let name = self.content.abilities[ability].name.clone();
        let was = self.fx_from.replace(at);
        for u in mobs.into_iter().chain(heroes).filter(|u| *u != on) {
            self.apply_effects(owner, u, &burns, 1.0, &name);
        }
        self.fx_from = was;
    }
}
