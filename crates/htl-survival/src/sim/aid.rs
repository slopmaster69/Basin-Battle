//! Counters for others: the Light Priest reads the blows landing on their friends. Holy Nova cast
//! as a blow lands on a friend inside it catches the blow whole; Leap of Faith pulling a friend as
//! a blow lands on them pulls them out of it, and it misses. Either way the first blow caught pays
//! the Priest a counter's Valor and the perfect (its burst on them, for everyone).
//!
//! Lag: the Priest's press is judged on their own screen (its arrival less their round trip, as
//! the rhythm check's), and the blow waits on the friend's round trip as it always does (see
//! `resolve`), so a cover made while it waits still catches it. A friend pulled out of a creature's
//! reach just before it swings is slipped where it swings (as Blink's slip is).

use super::*;

/// A friend a counter for others reached: by whom, with what, when (on the caster's screen), and
/// from where a pull took them (none for a nova).
#[derive(Debug, Clone)]
pub(crate) struct Cover {
    pub by: Id,
    pub ability: usize,
    pub on: Id,
    pub at: f32,
    pub from: Option<Vec3>,
    pub paid: bool,
}

impl Survival {
    /// When the caster did it, on their own screen: now less their round trip.
    fn own_screen(&self, id: Id) -> f32 {
        let rewind = self.players.get(&id).map_or(0.0, |p| p.rtt.min(self.content.combat.max_rewind));
        self.clock - rewind
    }

    /// A Holy Nova that covers: every other friend inside it is covered.
    pub(crate) fn cover_nova(&mut self, id: Id, ability: usize, at: Vec3, radius: f32) {
        let t = self.own_screen(id);
        let friends: Vec<Id> = self
            .players
            .iter()
            .filter(|(h, p)| **h != id && p.health > 0 && self.helpable(id, **h) && dist2d(p.pos, at) <= radius + abilities::HERO_SIZE)
            .map(|(h, _)| *h)
            .collect();
        for on in friends {
            self.light.covers.push(Cover { by: id, ability, on, at: t, from: None, paid: false });
        }
    }

    /// A Leap of Faith that covers reached a friend at `from`: they're covered as it pulls them.
    pub(crate) fn cover_pull(&mut self, id: Id, ability: usize, on: Id, from: Vec3) {
        if on == id {
            return;
        }
        let t = self.own_screen(id);
        self.light.covers.push(Cover { by: id, ability, on, at: t, from: Some(from), paid: false });
    }

    /// Covers too old for any blow to be caught by.
    pub(crate) fn step_covers(&mut self) {
        let now = self.clock;
        let keep = self.content.checks.window + self.content.combat.max_grace + self.content.combat.max_rewind + 0.5;
        self.light.covers.retain(|c| now - c.at <= keep);
    }

    /// A blow from `attacker` on `victim` that landed at `landed` (and settles now): the cover
    /// that catches it, if one does (made within the window before it landed, or since, while it
    /// waited on the round trip).
    fn cover_for(&self, victim: Id, attacker: Id, landed: f32) -> Option<usize> {
        let w = self.content.checks.window;
        self.light.covers.iter().position(|c| c.on == victim && c.by != attacker && c.at >= landed - w - 1.0e-4 && c.at <= self.clock + 1.0e-4)
    }

    /// A blow on a hero settles (`resolve`): caught by a cover, it's over (a nova's light caught
    /// it, or a pull took them out of it). True: caught.
    pub(crate) fn covered(&mut self, victim: Id, attacker: Id, landed: f32, style: Blow) -> bool {
        let Some(k) = self.cover_for(victim, attacker, landed) else { return false };
        let pulled = self.light.covers[k].from.is_some();
        let outcome = if pulled { Outcome::Miss } else { Outcome::Blocked };
        self.events.push(Event::Swing { attacker, victim: Some(victim), damage: 0, outcome, anim: false, style });
        self.cover_pays(k);
        true
    }

    /// Where a friend was pulled from, if a covering pull took them within the window before
    /// now: a creature's blow swung now that would have caught them there missed.
    pub(crate) fn pulled_from(&self, id: Id) -> Option<Vec3> {
        let w = self.content.checks.window;
        let now = self.clock;
        self.light.covers.iter().rev().find(|c| c.on == id && c.from.is_some() && now - c.at <= w + 1.0e-4).and_then(|c| c.from)
    }

    /// A creature's blow that would have caught a friend where a pull took them from: it misses.
    pub(crate) fn pulled_out(&mut self, id: Id, attacker: Id, style: Blow) {
        self.events.push(Event::Swing { attacker, victim: Some(id), damage: 0, outcome: Outcome::Miss, anim: false, style });
        let now = self.clock;
        let w = self.content.checks.window;
        if let Some(k) = self.light.covers.iter().rposition(|c| c.on == id && c.from.is_some() && now - c.at <= w + 1.0e-4) {
            self.cover_pays(k);
        }
    }

    /// The first blow a cast's cover caught pays its caster: a counter's Valor, the perfect and
    /// its burst. The friend shows the catch too.
    fn cover_pays(&mut self, k: usize) {
        let c = self.light.covers[k].clone();
        // A nova covers everyone in it at once: one pay for the cast.
        let paid = self.light.covers.iter().any(|x| x.by == c.by && x.ability == c.ability && (x.at - c.at).abs() < 1.0e-4 && x.paid);
        if !paid {
            self.light.covers[k].paid = true;
            if self.players.contains_key(&c.by) {
                self.gain_valor(c.by, self.content.valor.counter);
                let kit = self.content.checks.perfect_kit;
                if kit != 0 {
                    self.events.push(Event::Visual { id: c.by, kit, by: None });
                }
                self.events.push(Event::Perfect { player: c.by, ability: c.ability });
            }
        }
        // The light that caught it, on the friend.
        let spell = self.content.abilities[c.ability].spell;
        if spell != 0 && self.players.contains_key(&c.on) {
            self.events.push(Event::Missile { caster: c.by, spell, target: Target::Unit(c.on) });
        }
    }
}
