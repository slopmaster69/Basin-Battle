//! The weapon identity pass (docs/weapon-identities.md): the one mechanic each weapon has that no
//! other does. The fields live on `WeaponDef`; what they do is here, called from the one place a
//! hero's blow lands on a creature (`combat.rs`) and the few where a creature falls or mends.

use super::*;
use crate::content::{WeaponDef, AbilityEffect, BLOODLUST_SECS, BLOODLUST_STACKS, COW_SECS, CURSE_RADIUS, CURSE_SECS, IRON_SKIN_CAP, IRON_SKIN_SECS, MEND_RANGE, SILVER_SECS, SNARE_SECS, TENDER_SECS};

/// What weapons have done to a creature, kept on it.
#[derive(Debug, Clone, Default)]
pub struct MobFx {
    /// Silvered until: it doesn't mend on its way home, and stays down once slain.
    pub silvered_until: f32,
    /// A hook has taken its shield aside until: it can't block.
    pub guard_down_until: f32,
    /// Tenderized: the next hit does this much more, until when.
    pub tender: (f32, f32),
    /// Cursed until, by whom, and the burst it ends in.
    pub curse: Option<(f32, Id, u32)>,
}

/// What a weapon has built up in the hero holding it.
#[derive(Debug, Clone, Default)]
pub struct HeroFx {
    /// Bloodlust: kills in a row, and until when the latest one counts.
    pub bloodlust: (u32, f32),
}

impl Survival {
    /// Whether a creature is of a kind (`[[mob]] tags`).
    pub(crate) fn has_tag(&self, mid: Id, tag: &str) -> bool {
        self.mobs.get(&mid).is_some_and(|m| self.content.mobs[m.def].tags.iter().any(|t| t == tag))
    }

    /// Whether a creature is under silver.
    pub(crate) fn silvered(&self, mid: Id) -> bool {
        self.mobs.get(&mid).is_some_and(|m| m.fx.silvered_until > self.clock)
    }

    /// How much more (or less) a blow of this weapon does to this creature: a bane foe takes more.
    pub(crate) fn weapon_share(&self, mid: Id, w: &WeaponDef, strike: &Strike) -> f32 {
        let mut k = 1.0;
        if w.bane_damage > 0.0 && !strike.shove() && w.bane.iter().any(|t| self.has_tag(mid, t)) {
            k += w.bane_damage;
        }
        k
    }

    /// What a blow does as it lands, before its damage: silver marks the foe, so the blow that
    /// fells it lays it to rest too.
    pub(crate) fn weapon_before(&mut self, mid: Id, w: &WeaponDef, strike: &Strike) {
        if w.silver && !strike.shove() && w.bane.iter().any(|t| self.has_tag(mid, t)) {
            let until = self.clock + SILVER_SECS;
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.fx.silvered_until = until;
            }
        }
    }

    /// What a blow that landed sets off besides its damage.
    pub(crate) fn weapon_hit(&mut self, id: Id, mid: Id, w: &WeaponDef, strike: &Strike, damage: u32) {
        if strike.shove() || damage == 0 {
            return;
        }
        let now = self.clock;
        // The hook: a heavy blow drags the foe in.
        if w.hook > 0.0 && strike.heavy() && self.alive(mid) {
            let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: w.hook, from_caster: true, ..Default::default() };
            self.displace(id, mid, &pull);
        }
        // The hooked guard: a shield-bearer struck by the heavy can't block for a while.
        if w.guard_hook > 0.0 && strike.heavy() {
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.fx.guard_down_until = now + w.guard_hook;
            }
        }
        // The reaper's cut mends the reaper, foe by foe.
        if w.reap_heal > 0.0 {
            let amount = self.players.get(&id).map_or(0, |p| (p.max_health as f32 * w.reap_heal).round() as u32);
            self.heal_hero_by(id, amount, id, "Reaping");
        }
        // Hogger's heavy knocks the foe back.
        if w.knock > 0.0 && strike.heavy() && self.alive(mid) {
            let push = AbilityEffect { kind: "knockback".into(), amount: w.knock, from_caster: true, ..Default::default() };
            self.displace(id, mid, &push);
        }
        // The meat mallet tenderizes: the next hit on it, from anyone, does more.
        if w.tender > 0.0 && self.alive(mid) {
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.fx.tender = (w.tender, now + TENDER_SECS);
            }
        }
        // The wrench shorts a machine out.
        if w.machine_stun > 0.0 && strike.heavy() && self.alive(mid) && self.has_tag(mid, "mechanical") {
            self.apply_cc(mid, id, "stun", w.machine_stun, 0.0, 0.0, "Short circuit");
        }
        // The Hand mends the most wounded hero near.
        if w.mend_ally > 0.0 {
            self.mend_nearest_hurt(id, w.mend_ally);
        }
        // The scepter curses: its burst waits on the foe's death.
        if w.curse > 0.0 && self.alive(mid) {
            let burst = ((w.damage[0] + w.damage[1]) as f32 / 2.0 * w.curse).round() as u32;
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.fx.curse = Some((now + CURSE_SECS, id, burst));
            }
        }
        // The fist hardens its wielder.
        if w.iron_skin > 0.0 {
            let have = self.cc_of(id).iter().find(|c| c.kind == "shield" && c.until > now).map_or(0.0, |c| c.left);
            self.apply_cc(id, id, "shield", IRON_SKIN_SECS, (have + w.iron_skin).min(IRON_SKIN_CAP), 0.0, "Iron skin");
        }
        // The pin and the fright: a slow, a weaker foe.
        if w.snare > 0.0 && self.alive(mid) {
            self.apply_cc(mid, id, "snare", SNARE_SECS, w.snare, 0.0, "Hamstring");
        }
        if w.cow > 0.0 && self.alive(mid) {
            self.apply_cc(mid, id, "cowed", COW_SECS, w.cow, 0.0, "Dread");
        }
        // The fel step: the chain's last blow carries the hero round behind the foe.
        if w.blink_behind && strike.kind == StrikeKind::Light && self.alive(mid) {
            let last = self.content.moveset_of(w).light.len().saturating_sub(1);
            if strike.step as usize == last && last > 0 {
                self.step_behind(id, mid);
            }
        }
    }

    /// The next hit on a tenderized foe does more, and uses it up.
    pub(crate) fn take_tender(&mut self, mid: Id) -> f32 {
        let now = self.clock;
        match self.mobs.get_mut(&mid) {
            Some(m) if m.fx.tender.1 > now => std::mem::take(&mut m.fx.tender).0,
            _ => 0.0,
        }
    }

    /// The most wounded hero within reach of `id` (themselves too) is healed this share of their health.
    fn mend_nearest_hurt(&mut self, id: Id, share: f32) {
        let Some(at) = self.players.get(&id).map(|p| p.pos) else { return };
        let target = self
            .players
            .iter()
            .filter(|(_, p)| p.health > 0 && p.health < p.max_health && dist2d(p.pos, at) <= MEND_RANGE)
            .min_by(|a, b| (a.1.health as f32 / a.1.max_health as f32).total_cmp(&(b.1.health as f32 / b.1.max_health as f32)))
            .map(|(h, p)| (*h, p.max_health));
        if let Some((h, max)) = target {
            self.heal_hero_by(h, (max as f32 * share).round() as u32, id, "Mending");
        }
    }

    /// A cursed foe's fall: it bursts in shadow, and what stands near takes it.
    pub(crate) fn curse_burst(&mut self, mid: Id) {
        let now = self.clock;
        let Some((_, by, damage)) = self.mobs.get(&mid).and_then(|m| m.fx.curse).filter(|(u, _, _)| *u > now) else { return };
        let Some(at) = self.mobs.get(&mid).map(|m| m.pos) else { return };
        self.mobs.get_mut(&mid).unwrap().fx.curse = None;
        let near: Vec<Id> = self.mobs.iter().filter(|(k, m)| **k != mid && m.health > 0 && dist2d(m.pos, at) <= CURSE_RADIUS).map(|(k, _)| *k).collect();
        for k in near {
            self.hurt_mob_as(k, by, damage, "Curse");
        }
    }

    /// A hero's kill: what the weapon in hand makes of it (a bloodlust builds).
    pub(crate) fn weapon_kill(&mut self, killer: Id) {
        let now = self.clock;
        let Some(p) = self.players.get(&killer) else { return };
        let lust = self.weapon_of(p).bloodlust;
        if lust > 0.0 {
            let p = self.players.get_mut(&killer).unwrap();
            let (n, until) = p.fx.bloodlust;
            let n = if until > now { (n + 1).min(BLOODLUST_STACKS) } else { 1 };
            p.fx.bloodlust = (n, now + BLOODLUST_SECS);
        }
    }

    /// The share a hero's swings come quicker by: a bloodlust's stacks while they last.
    pub(crate) fn bloodlust_haste(&self, id: Id) -> f32 {
        let Some(p) = self.players.get(&id) else { return 0.0 };
        let (n, until) = p.fx.bloodlust;
        if until > self.clock { self.weapon_of(p).bloodlust * n as f32 } else { 0.0 }
    }

    /// The hero comes out just behind a foe (on the far side from where they stood), facing it.
    fn step_behind(&mut self, id: Id, mid: Id) {
        let (Some(me), Some(foe)) = (self.players.get(&id).map(|p| p.pos), self.mobs.get(&mid).map(|m| (m.pos, self.content.mobs[m.def].body()))) else { return };
        let (at, size) = foe;
        let away = facing(me, at);
        let spot = [at[0] + away.cos() * (size + BEHIND_GAP), at[1] + away.sin() * (size + BEHIND_GAP), at[2]];
        let land = self.ground_near(spot[0], spot[1], spot[2] + 1.0);
        if self.land.blocked_near(land[0], land[1], land[2]) || (land[2] - at[2]).abs() > 3.0 {
            return;
        }
        self.set_down(id, land, facing(land, at));
    }
}

/// How far past the foe's body the fel step lands.
pub const BEHIND_GAP: f32 = 0.9;
