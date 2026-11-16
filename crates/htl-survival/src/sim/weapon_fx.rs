//! The weapon identity pass (docs/weapon-identities.md): the one mechanic each weapon has that no
//! other does. The fields live on `WeaponDef`; what they do is here, called from the one place a
//! hero's blow lands on a creature (`combat.rs`) and the few where a creature falls or mends.

use super::*;
use crate::content::{WeaponDef, SILVER_SECS};

/// What weapons have done to a creature, kept on it.
#[derive(Debug, Clone, Default)]
pub struct MobFx {
    /// Silvered until: it doesn't mend on its way home, and stays down once slain.
    pub silvered_until: f32,
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

    /// What a blow that landed sets off besides its damage.
    pub(crate) fn weapon_hit(&mut self, id: Id, mid: Id, w: &WeaponDef, strike: &Strike, damage: u32) {
        if strike.shove() || damage == 0 {
            return;
        }
        let now = self.clock;
        // Silver: a bane foe it strikes neither mends nor rises.
        if w.silver && w.bane.iter().any(|t| self.has_tag(mid, t)) {
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.fx.silvered_until = now + SILVER_SECS;
            }
        }
        // The hook: a heavy blow drags the foe in.
        if w.hook > 0.0 && strike.heavy() && self.alive(mid) {
            let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: w.hook, from_caster: true, ..Default::default() };
            self.displace(id, mid, &pull);
        }
        // The fel step: the chain's last blow carries the hero round behind the foe.
        if w.blink_behind && strike.kind == StrikeKind::Light && self.alive(mid) {
            let last = self.content.moveset_of(w).light.len().saturating_sub(1);
            if strike.step as usize == last && last > 0 {
                self.step_behind(id, mid);
            }
        }
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
