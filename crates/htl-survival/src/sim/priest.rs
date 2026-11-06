//! The Priest's core (claude/priest-kit-draft.md): a weapon whose right click is an ability (a
//! holy staff's Flash Heal, a shadow staff's Drain) and so no guard at all; Power Word: Shield
//! given to a friend just as a blow lands on them, parrying it for them; and Grace, the thread
//! the Priest's perfects build and the holy staff's Heal spends.
//!
//! **The right click** (`alt` on the [[weapon]]): a `channel` is held while the button is and
//! ends as it's let go; anything else goes on the press, its `cooldown` the fire rate. With one
//! in hand nothing guards: the right mouse never raises a guard, a block or a ward.
//!
//! **The Shield's perfect** (`parry_for`): the shield lands on a friend (never the caster) at
//! most `[checks] window` before a blow lands on them, the caster's round trip allowed on the
//! early side (as far back as `combat.max_rewind`: they pressed on their own screen, which is
//! that far behind), or while the blow waits on the friend's round trip (`resolve`). The blow
//! is then parried as a parry of the friend's would be (the striker reels, a boss's posture
//! fills), the shield stays, and the caster gets a counter's Valor and the perfect, once a cast.
//! On themself it only absorbs.
//!
//! **Grace** (`[grace]`): every `Event::Perfect` of a spell whose card is the Priest's (a perfect
//! Shield, and whatever perfects the other Priest spells push) raises the hero's Grace a level,
//! found centrally as `step` hands its events out, so no spell needs to raise it itself. It
//! fades `secs` after the last rise; a blow that hurts the hero costs a level. A Heal that spends
//! it (`spends_grace`) takes it as it begins: cast `haste` faster, and healing past full health
//! stays on as a shield. Each level shows as one more look on the hero.

use super::*;
use crate::content::{AbilityDef, GraceDef};
use combat::PendingStrike;

/// A hero's share of the Priest's core: their Grace, the Grace the Heal in hand was begun with,
/// and the last shield a friend gave them (for its perfect).
#[derive(Debug, Clone, Default)]
pub struct Priest {
    pub grace: u32,
    /// When Grace last rose (it fades `[grace] secs` after).
    pub grace_at: f32,
    pub cast_grace: u32,
    pub shield: Option<ShieldMark>,
}

/// A shield given by a friend: who, which ability, when it landed (at the server), how far back
/// their press is judged (their round trip, up to `max_rewind`), and whether its perfect has paid.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShieldMark {
    pub by: Id,
    pub ability: usize,
    pub at: f32,
    pub rewind: f32,
    pub paid: bool,
}

impl Survival {
    // --- the right click as an ability --------------------------------------------------------------

    /// The ability the hero's weapon puts on the right click, if it puts one there.
    pub(crate) fn alt_of(&self, p: &Player) -> Option<usize> {
        let w = self.weapon_of(p);
        if !w.has_alt() {
            return None;
        }
        self.content.ability(&w.alt)
    }

    /// The right mouse down with a weapon whose right click is an ability: it goes (or, a cord,
    /// begins), once the hero is free. False: it waits for the end of what they're doing.
    pub(crate) fn alt_press(&mut self, id: Id, ability: usize) -> bool {
        let Some(p) = self.players.get(&id) else { return true };
        // An aim or a charge in hand: the right click calls it off, as ever.
        if matches!(p.action, Action::Aim { .. } | Action::Charge { .. }) {
            self.ability_cancel(id);
            return true;
        }
        if matches!(p.action, Action::Channel { .. } | Action::Stance { .. } | Action::Dash { .. } | Action::Sweep { .. } | Action::Rhythm { .. } | Action::Stagger { .. }) {
            return true;
        }
        let free = matches!(p.action, Action::Ready | Action::Guard);
        let busy_soft = matches!(p.action, Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. });
        if !(free || busy_soft) {
            return false;
        }
        let d = self.content.abilities[ability].clone();
        // A cord is held: pressed long ago and let go meanwhile (a press that waited), nothing.
        if d.kind == "channel" && !p.guard_held {
            return true;
        }
        if d.stamina > 0.0 && p.stamina < self.content.player.exhausted_below {
            self.winded(id);
            return true;
        }
        // The fire rate: too soon after the last, nothing.
        if !self.ready(id, ability) {
            return true;
        }
        self.unsheathe(id);
        self.fire(id, ability, 1.0);
        true
    }

    /// The right mouse let go: a cord held from it ends.
    pub(crate) fn alt_release(&mut self, id: Id, ability: usize) {
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Channel { ability: a, .. }) if *a == ability) {
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
        }
    }

    // --- Flash Heal: a bolt for friends --------------------------------------------------------------

    /// The bodies a bolt that passes foes by leaves out: every creature, and every hero its caster
    /// can't help (with those it has already met).
    pub(crate) fn passes_by(&self, owner: Id, met: &[Id]) -> Vec<Id> {
        let mut skip: Vec<Id> = met.to_vec();
        skip.extend(self.mobs.keys().copied());
        skip.extend(self.players.keys().copied().filter(|h| *h != owner && !self.helpable(owner, *h)));
        skip
    }

    /// A bolt for friends reaches one: healed, its landing look on them, no words.
    pub(crate) fn friendly_bolt_lands(&mut self, owner: Id, friend: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(f) = self.players.get(&friend).filter(|f| f.health > 0) else { return };
        let amount = (f.max_health as f32 * d.ally_heal * self.weakened(owner)).round() as u32;
        self.heal_hero_by(friend, amount, owner, &d.name);
        if d.burst_spell != 0 {
            self.events.push(Event::Missile { caster: owner, spell: d.burst_spell, target: Target::Unit(friend) });
        }
        if d.valor_hit > 0.0 {
            self.gain_valor(owner, d.valor_hit);
        }
    }

    // --- the Heal that spends Grace ------------------------------------------------------------------

    /// A Heal that spends Grace begins: what Grace there is goes into it (and off the hero).
    pub(crate) fn take_grace(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.priest.cast_grace = p.priest.grace;
        if p.priest.grace > 0 {
            p.priest.grace = 0;
            self.events.push(Event::Auras { id });
            self.events.push(Event::PriestLook { player: id, kind: "grace", n: 0, on: None, a: 1.0, b: 0.0 });
        }
    }

    /// How much quicker a Heal that spends Grace is cast for the Grace it was begun with (a share
    /// of its cast time: 1 is instant).
    pub(crate) fn grace_haste(&self, id: Id) -> f32 {
        let level = self.players.get(&id).map_or(0, |p| p.priest.cast_grace);
        GraceDef::at(&self.content.grace.haste, level).clamp(0.0, 1.0)
    }

    /// The Heal goes: on the friend under the reticle (yourself if none), its look on them; with
    /// the Grace it was begun with, healing past full health stays on as a shield.
    pub(crate) fn grace_heal(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let g = self.content.grace.clone();
        let level = self.players.get_mut(&id).map_or(0, |p| std::mem::take(&mut p.priest.cast_grace));
        let range = AbilityDef::at(d.range, share);
        let radius = AbilityDef::at(d.radius, share);
        let target = self.ally_target(id, range, if radius > 0.0 { radius } else { 1.0 });
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(target) });
        let Some(t) = self.players.get(&target).filter(|t| t.health > 0) else { return };
        let (before, max) = (t.health, t.max_health);
        let amount = (max as f32 * d.ally_heal * self.weakened(id)).round() as u32;
        self.heal_hero_by(target, amount, id, &d.name);
        let healed = self.players.get(&target).map_or(0, |t| t.health.saturating_sub(before));
        let over = amount.saturating_sub(healed) as f32 * GraceDef::at(&g.overheal, level);
        if over >= 1.0 {
            self.apply_cc(target, id, "shield", g.overheal_secs, over.round(), 0.0, &d.name);
            self.events.push(Event::Auras { id: target });
        }
        if level >= g.most && g.top_spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: g.top_spell, target: Target::Unit(target) });
        }
        if target != id {
            let v = if d.valor_hit > 0.0 { d.valor_hit } else { self.content.valor.ability };
            self.gain_valor(id, v);
        }
    }

    // --- Grace ---------------------------------------------------------------------------------------

    pub fn grace_of(&self, id: Id) -> u32 {
        self.players.get(&id).map_or(0, |p| p.priest.grace)
    }

    /// Whether a perfect of this ability is a Priest's (its card is in the Priest's column).
    fn priests(&self, ability: usize) -> bool {
        self.content.abilities.get(ability).is_some_and(|_| self.content.abilities[self.content.base_of(ability)].school == "priest")
    }

    /// Grace a level higher (up to its most), the fade begun again.
    pub(crate) fn raise_grace(&mut self, id: Id) {
        let (now, most) = (self.clock, self.content.grace.most);
        let Some(p) = self.players.get_mut(&id).filter(|p| p.health > 0) else { return };
        p.priest.grace = (p.priest.grace + 1).min(most);
        p.priest.grace_at = now;
        let n = p.priest.grace;
        self.events.push(Event::Auras { id });
        self.events.push(Event::PriestLook { player: id, kind: "grace", n, on: None, a: 0.0, b: 0.0 });
    }

    /// A blow that hurt the hero: a level of Grace lost.
    pub(crate) fn lose_grace(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id).filter(|p| p.priest.grace > 0) else { return };
        p.priest.grace -= 1;
        let n = p.priest.grace;
        self.events.push(Event::Auras { id });
        self.events.push(Event::PriestLook { player: id, kind: "grace", n, on: None, a: 0.0, b: 0.0 });
    }

    /// Each tick, as its events go out: every Priest's perfect among them raises its hero's
    /// Grace; Grace whose time is up fades.
    pub(crate) fn step_grace(&mut self) {
        let risen: Vec<Id> = self
            .events
            .iter()
            .filter_map(|e| match e {
                // Only for a perfect of the hero's own: a friend landing the Priest's Salvation
                // beat is the Priest's Grace, not theirs.
                Event::Perfect { player, ability } if self.priests(*ability) && self.own_ability(*player, *ability) => Some(*player),
                _ => None,
            })
            .collect();
        for id in risen {
            self.raise_grace(id);
        }
        let (now, secs) = (self.clock, self.content.grace.secs);
        let faded: Vec<Id> = self.players.iter().filter(|(_, p)| p.priest.grace > 0 && (now - p.priest.grace_at >= secs || p.health == 0)).map(|(id, _)| *id).collect();
        for id in faded {
            if let Some(p) = self.players.get_mut(&id) {
                p.priest.grace = 0;
            }
            self.events.push(Event::Auras { id });
            self.events.push(Event::PriestLook { player: id, kind: "grace", n: 0, on: None, a: 0.0, b: 0.0 });
        }
    }

    /// Whether `ability` (or the ability it's the EX of) is on one of the hero's keys.
    fn own_ability(&self, id: Id, ability: usize) -> bool {
        let key = &self.content.abilities[self.content.base_of(ability)].key;
        self.players.get(&id).is_some_and(|p| p.loadout.values().any(|k| k == key))
    }

    /// Grace's looks on a hero: one more for each level.
    pub(crate) fn grace_auras(&self, p: &Player) -> impl Iterator<Item = u32> + '_ {
        self.content.grace.auras.iter().take(p.priest.grace as usize).copied().filter(|a| *a != 0)
    }

    // --- Power Word: Shield's perfect ----------------------------------------------------------------

    /// A shield that parries for a friend landed on them: marked, for the blows that land on them
    /// next (and those waiting on their round trip now).
    pub(crate) fn mark_shield(&mut self, by: Id, on: Id, ability: usize) {
        let now = self.clock;
        let rewind = self.players.get(&by).map_or(0.0, |p| p.rtt.min(self.content.combat.max_rewind));
        if let Some(p) = self.players.get_mut(&on) {
            p.priest.shield = Some(ShieldMark { by, ability, at: now, rewind, paid: false });
        }
    }

    /// A blow settling on a hero a friend's shield landed on in time: parried for them (true),
    /// as a parry of theirs would be; the shield stays, and the first pays its giver.
    pub(crate) fn shield_parries(&mut self, s: &PendingStrike) -> bool {
        if s.how == HitKind::Area || !s.parryable || s.attacker == s.victim {
            return false;
        }
        // A giant's blow can't be parried, by anyone.
        if self.mobs.get(&s.attacker).is_some_and(|m| self.content.mobs[m.def].unparryable) {
            return false;
        }
        let Some(v) = self.players.get(&s.victim) else { return false };
        let Some(mark) = v.priest.shield.filter(|m| m.by != s.victim) else { return false };
        // When it landed on the server: before its wait on their round trip.
        let landed = s.at - v.rtt.min(self.content.combat.max_grace);
        let window = self.content.checks.window;
        if mark.at + 1.0e-4 < landed - window - mark.rewind || mark.at > self.clock + 1.0e-4 {
            return false;
        }
        let from = s.from;
        self.parry_pays(s.victim, s.attacker);
        self.parry_looks(s.victim, from);
        self.events.push(Event::Swing { attacker: s.attacker, victim: Some(s.victim), damage: 0, outcome: Outcome::Parried, anim: false, style: s.style });
        self.events.push(Event::PriestLook { player: mark.by, kind: "shield", n: 1, on: Some(s.victim), a: 0.0, b: 0.0 });
        self.trade_blows(s.attacker, s.victim);
        if !mark.paid {
            if let Some(m) = self.players.get_mut(&s.victim).and_then(|p| p.priest.shield.as_mut()) {
                m.paid = true;
            }
            self.gain_valor(mark.by, self.content.valor.counter);
            let k = self.content.checks.perfect_kit;
            if k != 0 {
                self.events.push(Event::Visual { id: s.victim, kit: k, by: Some(mark.by) });
            }
            self.events.push(Event::Perfect { player: mark.by, ability: mark.ability });
        }
        true
    }
}
