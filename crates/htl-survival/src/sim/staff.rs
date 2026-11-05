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
    pub(crate) fn cast_bolt(&mut self, id: Id, ability: usize) {
        let hi = self.content.abilities[ability].charge[1];
        if hi > 0.0 {
            self.begin_charge(id, ability, hi);
        } else {
            self.fire(id, ability, 1.0);
        }
    }
}

/// Heat or chill a staff's bolts left on a foe (`[staffs]`).
#[derive(Debug, Clone)]
pub(crate) struct StaffMark {
    pub owner: Id,
    pub on: Id,
    pub kind: &'static str,
    pub n: u32,
    pub until: f32,
}

/// The kinds a staff builds and spends.
fn stack_kind(word: &str) -> Option<&'static str> {
    match word {
        "heat" => Some("heat"),
        "chill" => Some("chill"),
        "charge" => Some("charge"),
        _ => None,
    }
}

impl Survival {
    /// A staff's plain bolt struck a foe: its heat or chill on them (the last ignites or freezes),
    /// or a charge gathered at the caster's staff.
    pub(crate) fn staff_build(&mut self, owner: Id, victim: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(kind) = stack_kind(&d.builds) else { return };
        let k = self.content.staffs.clone();
        let now = self.clock;
        if kind == "charge" {
            let Some(p) = self.players.get_mut(&owner) else { return };
            let before = p.staff_charges;
            p.staff_charges = (p.staff_charges + 1).min(k.most);
            p.staff_charges_until = now + k.keep;
            let n = p.staff_charges;
            if n == k.most && before < k.most && k.charged_kit != 0 {
                self.events.push(Event::Visual { id: owner, kit: k.charged_kit, by: None });
            }
            self.events.push(Event::Stacks { player: owner, kind, n, on: None });
            return;
        }
        let i = match self.staff_marks.iter().position(|m| m.owner == owner && m.on == victim && m.kind == kind) {
            Some(i) => i,
            None => {
                self.staff_marks.push(StaffMark { owner, on: victim, kind, n: 0, until: now });
                self.staff_marks.len() - 1
            }
        };
        let m = &mut self.staff_marks[i];
        m.n += 1;
        m.until = now + k.keep;
        let n = m.n;
        self.events.push(Event::Stacks { player: owner, kind, n, on: Some(victim) });
        if n < k.most {
            // Chill: slower with each.
            if kind == "chill" && k.chill_slow > 0.0 {
                let slow = crate::content::AbilityEffect { kind: "snare".into(), secs: [k.keep, k.keep], amount: (k.chill_slow * n as f32).min(0.9), ..Default::default() };
                self.apply_effects(owner, victim, &[slow], 1.0, &d.name);
            }
            return;
        }
        self.staff_marks.remove(i);
        let at = self.unit_pos(victim);
        if kind == "heat" {
            // The last heat ignites: a burst round the foe with the bolt's burn.
            if let Some(at) = at {
                if k.ignite_spell != 0 {
                    let mk = self.mark_unseen(1.0);
                    self.events.push(Event::AreaLook { id: mk, caster: owner, spell: k.ignite_spell, pos: at, radius: k.ignite_radius });
                }
                self.area(owner, ability, at, k.ignite_radius, 1.0, k.ignite_damage, None);
            }
        } else {
            // The last chill freezes them (a frozen foe shatters under fire or a heavy lance).
            let freeze = crate::content::AbilityEffect { kind: "petrify".into(), secs: [k.freeze_secs, k.freeze_secs], ..Default::default() };
            self.apply_effects(owner, victim, &[freeze], 1.0, &d.name);
        }
    }

    /// An Arcane staff's heavy orb as it goes: the charges stored in the staff go with it (`None`
    /// for any other bolt): how many, its share of full strength, and its look (0: its own).
    pub(crate) fn staff_release(&mut self, id: Id, ability: usize) -> Option<(u32, f32, u32)> {
        if self.content.abilities[ability].spends != "charge" {
            return None;
        }
        let k = self.content.staffs.clone();
        let p = self.players.get_mut(&id)?;
        let n = std::mem::take(&mut p.staff_charges).min(3);
        if n > 0 {
            self.events.push(Event::Stacks { player: id, kind: "charge", n: 0, on: None });
        }
        let i = n as usize;
        Some((n, k.charge_scale[i], k.charge_spells[i]))
    }

    /// A staff's heavy bolt reaching a foe spends what was built: its damage times this (1 for
    /// none). Heat and chill are the foe's, charges the caster's (all of them throw the foe back).
    pub(crate) fn staff_spend(&mut self, owner: Id, victim: Id, ability: usize) -> f32 {
        let d = &self.content.abilities[ability];
        let Some(kind) = stack_kind(&d.spends) else { return 1.0 };
        let k = self.content.staffs.clone();
        let n = if kind == "charge" {
            let Some(p) = self.players.get_mut(&owner) else { return 1.0 };
            std::mem::take(&mut p.staff_charges)
        } else {
            match self.staff_marks.iter().position(|m| m.owner == owner && m.on == victim && m.kind == kind) {
                Some(i) => self.staff_marks.remove(i).n,
                None => 0,
            }
        };
        if n == 0 {
            return 1.0;
        }
        let on = (kind != "charge").then_some(victim);
        self.events.push(Event::Stacks { player: owner, kind, n: 0, on });
        if kind == "charge" && n >= k.most && k.charged_knockback > 0.0 {
            let name = d.name.clone();
            let back = crate::content::AbilityEffect { kind: "knockback".into(), amount: k.charged_knockback, ..Default::default() };
            self.apply_effects(owner, victim, &[back], 1.0, &name);
        }
        1.0 + k.spend_share * n as f32
    }

    /// Charges and marks fade a while after the last bolt.
    pub(crate) fn step_staff(&mut self, id: Id) {
        let now = self.clock;
        if let Some(p) = self.players.get_mut(&id) {
            if p.staff_charges > 0 && now >= p.staff_charges_until {
                p.staff_charges = 0;
                self.events.push(Event::Stacks { player: id, kind: "charge", n: 0, on: None });
            }
        }
        let alive: Vec<bool> = self.staff_marks.iter().map(|m| m.owner != id || (now < m.until && self.alive(m.on))).collect();
        let mut k = 0;
        let mut gone = Vec::new();
        self.staff_marks.retain(|m| {
            let keep = alive[k];
            k += 1;
            if !keep {
                gone.push((m.kind, m.on));
            }
            keep
        });
        for (kind, on) in gone {
            self.events.push(Event::Stacks { player: id, kind, n: 0, on: Some(on) });
        }
    }
}
