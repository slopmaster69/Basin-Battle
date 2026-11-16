//! The Rogue's core (docs/rogue-roles.md, "The core: footwork and two blades"): a hero created as a
//! Rogue fights with a blade in each hand and has no guard, no block and no parry.
//!
//! **Two hands.** The right click swings the off hand (dual wield's `off_press`), whatever is in
//! it: an empty off hand jabs for half the main hand's blow (`[rogue] jab`). A Rogue can't hold a
//! shield, a two-hander, a staff or an axe (`Content::rogue_can_hold`); an old save that held one
//! loads with it in the bags ([`rogue_hands`]). A card that blocks or parries refuses.
//!
//! **The perfect dodge.** A blow that lands 0.04 to `[dodge] perfect_to` sec into a Rogue's dodge
//! (pressed in its last moment, judged a round trip late as every dodge is) pays: more stamina
//! back, a parry's Valor, a share of a boss's or champion's posture, and the **Shadow Cut**: for
//! `[rogue] cut_secs`, either attack steps behind the one who swung and strikes its back. It reads
//! with no floor marker: the body flickers (the Stealth look), Vanish's puff and sound where the
//! blow fell, and a short stop on the Rogue's own screen (the `perfect_dodge` beat).
//!
//! **Footwork in the run.** A dodge pressed in the Rogue's own wind-up calls the blow off, its
//! stamina back (the dodge-feint); a dodge cuts an off-hand blow short at once; a dodge between
//! blows keeps the two-blade run, and the Shadow Cut counts as its next hand.
//!
//! None of it is any other class's: a Warrior with two blades has the off hand and the run, not
//! the footwork.

use super::*;
use crate::land::clear_line;

/// What a Rogue's footwork holds between ticks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rogue {
    /// The Shadow Cut is open until then...
    pub cut_until: f32,
    /// ...behind this one (who swung).
    pub cut_on: Option<Id>,
    /// The perfect dodge's flicker shows on the body until then (0: none showing).
    pub flicker_until: f32,
}

/// The Rogue's class key.
pub const ROGUE: &str = "rogue";

/// A Rogue's hands put right, as a save is loaded: what a Rogue can't hold goes into the bags. A
/// main hand that can't be a Rogue's takes the Rogue's start blade; an off hand that held a shield
/// or a blade a Rogue can't use is left empty (it jabs). Once put right, it stays: the save then
/// holds the blade and an empty off hand.
pub(crate) fn rogue_hands(c: &crate::content::Content, p: &mut Player) {
    if p.class != ROGUE {
        return;
    }
    if c.rogue_can_hold(p.weapon).is_err() {
        if let Some(knife) = c.item(&c.rogue.weapon) {
            *p.bag.entry(p.weapon).or_default() += 1;
            p.weapon = knife;
        }
    }
    if let Some(o) = p.off_weapon.filter(|o| c.rogue_can_hold(*o).is_err()) {
        *p.bag.entry(o).or_default() += 1;
        p.off_weapon = None;
        p.off_empty = true;
    }
    // A shield held (no blade in the off hand, and not left empty): into the bags.
    if p.off_weapon.is_none() && !p.off_empty {
        *p.bag.entry(p.shield).or_default() += 1;
        p.off_empty = true;
    }
}

impl Survival {
    /// A hero created as a Rogue.
    pub fn is_rogue(&self, p: &Player) -> bool {
        p.class == ROGUE
    }

    /// Fights with two hands: a blade in the off hand (dual wield), or a Rogue's (an empty off
    /// hand jabs). The right click is then the off hand, never a guard.
    pub fn two_hands(&self, p: &Player) -> bool {
        self.off_weapon_of(p).is_some() || self.is_rogue(p)
    }

    /// Whether this hero may take up this item, and if not, the words why (a Rogue's hands).
    pub(crate) fn can_hold(&self, id: Id, item: usize) -> Result<(), String> {
        match self.players.get(&id) {
            Some(p) if self.is_rogue(p) => self.content.rogue_can_hold(item).map_err(String::from),
            _ => Ok(()),
        }
    }

    /// A card this hero can't use for their hands: one that blocks or parries, on a Rogue; one that
    /// needs two blades, with one.
    pub(crate) fn hands_refuse_card(&self, id: Id, d: &crate::content::AbilityDef) -> Option<String> {
        let p = self.players.get(&id)?;
        if self.is_rogue(p) {
            if let Some(k) = d.counter.as_ref().filter(|k| k.block) {
                // A brace held toward the front blocks (Retaliation, Shield Wall); one from
                // anywhere turns the blow aside, a parry (Riposte).
                return Some(if k.front > 0.0 { "Rogues can't block." } else { "Rogues can't parry." }.to_string());
            }
        }
        if d.needs_two_blades && self.off_weapon_of(p).is_none() {
            return Some(format!("{} requires a weapon in each hand", d.name));
        }
        None
    }

    /// A Rogue's dodge took this blow in its perfect moment.
    pub(crate) fn dodged_perfectly(&self, victim: Id, doing: &Action) -> bool {
        let d = &self.content.dodge;
        d.perfect_to > 0.0 && self.players.get(&victim).is_some_and(|p| self.is_rogue(p)) && matches!(doing, Action::Dodge { t, .. } if *t >= d.evade_from && *t <= d.perfect_to)
    }

    /// What a perfect dodge pays besides the breath and Valor (`strike_player`): a share of the
    /// striker's posture, the Shadow Cut opened behind it, and how it shows.
    pub(crate) fn perfect_dodge(&mut self, victim: Id, attacker: Id) {
        let now = self.clock;
        let r = self.content.rogue.clone();
        let share = self.content.dodge.perfect_posture;
        // A boss's or champion's posture fills; nothing reels from one dodge.
        if let Some(m) = self.mobs.get(&attacker).filter(|m| m.health > 0) {
            if share > 0.0 && self.content.mobs[m.def].posture > 0 {
                let fill = (self.posture_limit(attacker) as f32 * share).round().max(1.0) as u32;
                self.build_posture(attacker, victim, fill);
            }
        }
        let alive = self.mobs.get(&attacker).is_some_and(|m| m.health > 0) || self.players.get(&attacker).is_some_and(|h| h.health > 0 && attacker != victim);
        let Some(p) = self.players.get_mut(&victim) else { return };
        let at = p.pos;
        if alive {
            p.rogue.cut_until = now + r.cut_secs;
            p.rogue.cut_on = Some(attacker);
        }
        // No word: the body flickers, a puff of smoke where the blow fell, Vanish's sound, and a
        // short stop on their own screen.
        if r.flicker_aura != 0 && r.flicker_secs > 0.0 {
            p.rogue.flicker_until = now + r.flicker_secs;
            self.events.push(Event::Auras { id: victim });
        }
        if r.puff != 0 {
            self.events.push(Event::Missile { caster: victim, spell: r.puff, target: Target::Point(at) });
        }
        self.cue(Some(victim), victim, 0, cues::SOUND_PERFECT_DODGE, cues::BEAT_PERFECT_DODGE);
    }

    /// The perfect dodge's flicker on the body, while it lasts (for `auras_of`).
    pub(crate) fn rogue_auras(&self, p: &Player) -> Option<u32> {
        let a = self.content.rogue.flicker_aura;
        (a != 0 && p.rogue.flicker_until > self.clock).then_some(a)
    }

    /// Each tick: the flicker gone from the body once its time is up.
    pub(crate) fn step_rogue(&mut self, id: Id) {
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.rogue.flicker_until > 0.0 && now >= p.rogue.flicker_until {
            p.rogue.flicker_until = 0.0;
            self.events.push(Event::Auras { id });
        }
    }

    /// The Shadow Cut: an attack while it's open steps the Rogue behind the one who swung (a puff
    /// at both ends) and strikes as that hand's next blow, from behind. False (and the press goes
    /// on as any other) when it isn't open, the Rogue isn't free to strike, or there's no spot
    /// behind within reach (a wall, a drop): no step, and the cut is spent.
    pub(crate) fn shadow_cut(&mut self, id: Id, kind: StrikeKind) -> bool {
        let now = self.clock;
        let c = &self.content;
        let Some(p) = self.players.get(&id) else { return false };
        if !self.is_rogue(p) || p.rogue.cut_until < now {
            return false;
        }
        let Some(on) = p.rogue.cut_on else { return false };
        // Out of the dodge, standing, or late in a blow's recovery (as a chain's next blow).
        let free = match &p.action {
            Action::Ready | Action::Dodge { .. } => true,
            Action::Swing { t, strike, .. } => strike.art.is_none() && *t >= strike.windup + strike.active + strike.recovery * c.combat.chain_after,
            _ => false,
        };
        if !free || (p.stamina < c.player.exhausted_below && !self.tireless(id)) {
            return false;
        }
        let (from, reach) = (p.pos, c.rogue.cut_reach);
        let puff = c.rogue.puff;
        let foe = match (self.mobs.get(&on), self.players.get(&on)) {
            (Some(m), _) if m.health > 0 => Some((m.pos, m.facing)),
            (_, Some(h)) if on != id && h.health > 0 => Some((h.pos, h.facing)),
            _ => None,
        };
        // One press: open or not, it's spent.
        let p = self.players.get_mut(&id).unwrap();
        p.rogue.cut_until = -100.0;
        p.rogue.cut_on = None;
        let Some((at, f)) = foe else { return false };
        // Squarely behind, or a little to a side (still in its back): never round its flank.
        let Some((spot, face)) = self.spot_behind(at, f, &[0.0, 0.5, -0.5]) else { return false };
        if dist2d(from, spot) > reach || !clear_line(self.land.as_ref(), from, spot) {
            return false;
        }
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(from) });
        }
        self.set_down(id, spot, face);
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(spot) });
        }
        // It counts as the run's next hand, whichever button it was.
        let p = self.players.get_mut(&id).unwrap();
        if p.dual_run > 0 {
            p.dual_last_off = kind != StrikeKind::Off;
        }
        p.dodge_attack = false;
        self.set_action(id, Action::Ready);
        self.unsheathe(id);
        self.start_swing(id, kind);
        true
    }

    /// A Rogue's dodge keeps the two-blade run going: its window counts from the dodge's end, as
    /// from a blow's.
    pub(crate) fn run_through_dodge(&mut self, id: Id) {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return };
        if !self.is_rogue(p) {
            return;
        }
        let p = self.players.get_mut(&id).unwrap();
        if p.dual_run > 0 && now <= p.combo_until + DUAL_WINDOW {
            p.combo_until = f32::INFINITY;
        }
    }

    /// A Rogue's dodge-feint: the dodge pressed while their own blow (not a card's) still winds up
    /// calls it off, its stamina back, as the guard's feint does; the run it was part of ends.
    /// True if one was called off (the dodge then goes).
    pub(crate) fn dodge_feint(&mut self, id: Id) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        if !self.is_rogue(p) {
            return false;
        }
        let Action::Swing { t, strike, .. } = &p.action else { return false };
        if *t >= strike.windup || strike.art.is_some() {
            return false;
        }
        let (refund, most, now) = (strike.stamina, self.content.player.stamina, self.clock);
        let p = self.players.get_mut(&id).unwrap();
        p.stamina = (p.stamina + refund).min(most);
        p.combo_next = 0;
        p.off_next = 0;
        p.combo_until = now;
        p.dual_run = 0;
        self.events.push(Event::Stamina { player: id });
        self.events.push(Event::ClipStop { id });
        self.set_action(id, Action::Ready);
        true
    }
}
