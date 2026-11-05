//! Staffs that cast: a weapon with a `bolt` turns the left click into a cast of that benched
//! projectile instead of a swing (and the heavy attack into its `heavy_bolt`), so a caster's
//! plain attack is a bolt from the staff along the reticle. The staff's bolts are abilities, so
//! their looks, speeds, damage and effects are content, one block beside the staff: an Ember
//! staff's bolts burn, a Rime staff's chill, an Arcane staff's fly quick.
//!
//! A bolt is cast as any ability is ([`Survival::fire`]): it spends its stamina, takes the
//! cast's recovery, and flies as a projectile. With the guard up the click still shoves (a kick,
//! as with any two-handed weapon).

use super::*;

impl Survival {
    /// The bolt the hero's weapon casts for this attack, if it casts one: the heavy bolt for the
    /// heavy attack (the plain bolt when the staff has no heavy of its own).
    pub(crate) fn bolt_of(&self, p: &Player, heavy: bool) -> Option<usize> {
        let w = self.weapon_of(p);
        if !w.casts() {
            return None;
        }
        let key = if heavy && !w.heavy_bolt.is_empty() { &w.heavy_bolt } else { &w.bolt };
        self.content.ability(key)
    }

    /// The staff's attack: the bolt cast now, aimed along the reticle; a bolt with a charge (a
    /// heavy one) begins its charge instead, let go with the button (its release check with it).
    /// A Heal that spends Grace takes it as it begins (quicker for it; at the top, at once).
    pub(crate) fn cast_bolt(&mut self, id: Id, ability: usize) {
        if self.content.abilities[ability].spends_grace {
            self.take_grace(id);
        }
        let (_, hi) = self.charge_of(id, ability);
        if hi > 0.0 {
            self.begin_charge(id, ability, hi);
        } else {
            self.fire(id, ability, 1.0);
        }
    }
}
