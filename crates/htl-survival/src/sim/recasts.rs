//! Combos between casts: a recast (a key's second activation, as in Battlerite: for a moment
//! after the first cast its key fires another ability, for free) and an empower (one ability's
//! cast makes another card's next press its EX version, for no Valor). And a dash that runs
//! through bodies, striking each as it passes.

use std::collections::BTreeMap;

use super::*;
use crate::content::{AbilityDef, SLOTS};

/// A key armed for its recast: the ability it fires, until when, and where the cast that armed
/// it began (a blink back goes there).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Recast {
    pub ability: usize,
    pub until: f32,
    pub start: Vec3,
}

/// A hero's combos in hand.
#[derive(Debug, Clone, Default)]
pub struct Combos {
    /// Keys armed for a recast, by the base ability on the key.
    pub recasts: BTreeMap<usize, Recast>,
    /// Base abilities empowered until then: their next press is their EX version, free.
    pub empowered: BTreeMap<usize, f32>,
    /// A dash running through bodies: whom it passes, when (seconds into the dash) and where,
    /// not yet struck.
    pub(crate) through: Vec<(f32, Id, Vec3)>,
}

/// What makes a cast free.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Free {
    /// A recast: no Valor and no cooldown; `start` is where the cast that armed it began.
    Recast { start: Vec3 },
    /// An empowered EX version: no Valor (its cooldown as usual).
    Empowered,
}

/// The hero is aiming or charging this ability (a window held open meanwhile, so a recast or an
/// empowered cast that's up when its time runs out still goes free).
fn in_hand(action: &Action, ability: usize) -> bool {
    matches!(action, Action::Aim { ability: a } | Action::Charge { ability: a, .. } if *a == ability)
}

impl Survival {
    /// The recast a key would fire now (the key's base ability), if it's armed.
    pub(crate) fn recast_armed(&self, id: Id, base: usize) -> Option<Recast> {
        let p = self.players.get(&id)?;
        let r = *p.combos.recasts.get(&base)?;
        (r.until > self.clock || in_hand(&p.action, r.ability)).then_some(r)
    }

    /// Whether a base ability's next press is its EX version, free.
    pub(crate) fn empowered(&self, id: Id, base: usize) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        let Some(until) = p.combos.empowered.get(&base) else { return false };
        *until > self.clock || self.content.ex_of(base).is_some_and(|x| in_hand(&p.action, x))
    }

    /// Whether a cast of `ability` going now is free (a recast, an empowered EX version), using
    /// up what makes it so.
    pub(crate) fn take_free(&mut self, id: Id, ability: usize) -> Option<Free> {
        let now = self.clock;
        let p = self.players.get(&id)?;
        let key = p.combos.recasts.iter().find(|(_, r)| r.ability == ability && (r.until > now || in_hand(&p.action, ability))).map(|(b, _)| *b);
        if let Some(b) = key {
            let r = self.players.get_mut(&id)?.combos.recasts.remove(&b)?;
            self.events.push(Event::Abilities { player: id });
            return Some(Free::Recast { start: r.start });
        }
        if self.content.abilities[ability].is_ex() {
            let base = self.content.base_of(ability);
            if self.empowered(id, base) {
                self.players.get_mut(&id)?.combos.empowered.remove(&base);
                self.events.push(Event::Abilities { player: id });
                return Some(Free::Empowered);
            }
        }
        None
    }

    /// An ability has gone (from `began`): its key armed for its recast, the card it empowers
    /// empowered. A recast arms nothing (it is the second activation).
    pub(crate) fn arm_combos(&mut self, id: Id, ability: usize, began: Vec3, free: Option<Free>) {
        let now = self.clock;
        let d = &self.content.abilities[ability];
        let recast = self.content.ability(&d.recast).filter(|_| !matches!(free, Some(Free::Recast { .. })));
        let window = d.recast_secs;
        let empower = d.empower.as_ref().and_then(|e| Some((self.content.ability(&e.ability)?, e.secs)));
        let base = self.content.base_of(ability);
        let start = self.standing_spot(began);
        let Some(p) = self.players.get_mut(&id) else { return };
        let mut changed = false;
        if let Some(r) = recast {
            p.combos.recasts.insert(base, Recast { ability: r, until: now + window, start });
            changed = true;
        }
        if let Some((b, secs)) = empower {
            p.combos.empowered.insert(b, now + secs);
            changed = true;
        }
        if changed {
            self.events.push(Event::Abilities { player: id });
        }
    }

    /// Every tick: recast windows and empowers whose time is up close (one aimed or charging
    /// meanwhile waits for its cast); a fallen hero loses them all.
    pub(crate) fn step_combos(&mut self) {
        let now = self.clock;
        let content = &self.content;
        let mut changed = Vec::new();
        for (id, p) in self.players.iter_mut() {
            let before = p.combos.recasts.len() + p.combos.empowered.len();
            if before == 0 {
                continue;
            }
            if p.health == 0 {
                p.combos.recasts.clear();
                p.combos.empowered.clear();
            } else {
                let action = p.action.clone();
                p.combos.recasts.retain(|_, r| r.until > now || in_hand(&action, r.ability));
                p.combos.empowered.retain(|b, until| *until > now || content.ex_of(*b).is_some_and(|x| in_hand(&action, x)));
            }
            if p.combos.recasts.len() + p.combos.empowered.len() != before {
                changed.push(*id);
            }
        }
        for id in changed {
            self.events.push(Event::Abilities { player: id });
        }
    }

    /// The keys armed for a recast, for the bar: (slot, the recast ability, until when).
    pub fn recast_keys(&self, id: Id) -> Vec<(usize, usize, f32)> {
        self.combo_slots(id).into_iter().filter_map(|(n, base)| self.recast_armed(id, base).map(|r| (n, r.ability, r.until))).collect()
    }

    /// The keys empowered (their next press is their EX version, free), for the bar: (slot, until
    /// when).
    pub fn empowered_keys(&self, id: Id) -> Vec<(usize, f32)> {
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        self.combo_slots(id).into_iter().filter(|(_, base)| self.empowered(id, *base)).filter_map(|(n, base)| Some((n, *p.combos.empowered.get(&base)?))).collect()
    }

    /// Each key's slot and the base ability on it.
    fn combo_slots(&self, id: Id) -> Vec<(usize, usize)> {
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        SLOTS.iter().enumerate().filter_map(|(n, s)| Some((n, self.content.ability(p.loadout.get(*s)?)?))).collect()
    }

    // --- a dash through bodies --------------------------------------------------------------------

    /// A dash's blow on one body it meets (its end's, or each it passes going through), as a
    /// melee hit from `from`.
    pub(crate) fn dash_strike(&mut self, id: Id, victim: Id, ability: usize, share: f32, from: Vec3) {
        if !self.alive(victim) {
            return;
        }
        let d = self.content.abilities[ability].clone();
        let damage = AbilityDef::at(d.damage, share);
        if self.players.contains_key(&victim) {
            let dmg = self.ability_damage(id, victim, ability, damage);
            self.strike_fair_as(id, victim, dmg, from, true, true, false, Blow::Heavy, None, combat::Follow::Ability { ability, share }, HitKind::Melee);
        } else {
            self.ability_on_mob(id, victim, ability, share, damage, from, HitKind::Melee, d.spell);
        }
    }

    /// A dash going through: everyone it has reached by `t` seconds in is struck (once). Anything
    /// else in `Action::Dash` (a leap) strikes none, and drops what a dash cut short left.
    pub(crate) fn dash_through(&mut self, id: Id, ability: usize, share: f32, t: f32) {
        let through = self.content.abilities[ability].through;
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.combos.through.is_empty() {
            return;
        }
        if !through {
            p.combos.through.clear();
            return;
        }
        let (due, keep): (Vec<(f32, Id, Vec3)>, Vec<(f32, Id, Vec3)>) = std::mem::take(&mut p.combos.through).into_iter().partition(|(at, ..)| *at <= t);
        p.combos.through = keep;
        for (_, v, from) in due {
            self.dash_strike(id, v, ability, share, from);
        }
    }
}
