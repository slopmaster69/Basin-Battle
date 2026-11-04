//! The rhythm check: a spell that goes off on beats. The first `free_beats` go by themselves, a
//! fixed `beat` apart, so the caster hears and sees the tempo; each further beat goes only if the
//! key is pressed within its window of it (`beat_window`, the first's tightening to the last's).
//! A press out of time, or a beat let pass, ends it where it is: what went, went, and nothing
//! hurts the caster. Each beat strikes harder than the last (`beat_step`); the last, the
//! finisher, `finisher_damage` times as hard and with `finisher_effects`. A full chain is a
//! perfect: a whole segment of Valor and the burst.
//!
//! Lag: the caster follows beats they see a one-way trip late, and their press reaches the server
//! another trip later, so a press is judged at its arrival less their round trip (as far back as
//! `combat.max_rewind`). Chain in Europe keeps the beat as Corey does by the server.

use super::*;
use crate::content::AbilityDef;

impl Survival {
    /// The rhythm begins: its first beat goes now.
    pub(crate) fn start_rhythm(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        self.set_action(id, Action::Rhythm { ability, since: now, done: 0, pressed: 0, cued: 0 });
        let d = &self.content.abilities[ability];
        if d.charge_spell != 0 {
            let ms = (d.beat * d.most_beats as f32 * 1000.0) as u32;
            self.events.push(Event::Cast { caster: id, spell: d.charge_spell, ms });
        }
        self.step_rhythm(id);
    }

    /// When beat `k` falls, and how close a press must be.
    fn beat_at(d: &AbilityDef, since: f32, k: u32) -> f32 {
        since + d.beat * k as f32
    }

    fn window_of(d: &AbilityDef, k: u32) -> f32 {
        let first = d.free_beats;
        let span = d.most_beats.saturating_sub(first + 1).max(1) as f32;
        let t = (k.saturating_sub(first) as f32 / span).clamp(0.0, 1.0);
        d.beat_window[0] + (d.beat_window[1] - d.beat_window[0]) * t
    }

    /// The rhythm's tick: every beat that's due and earned goes; one let pass ends it.
    pub(crate) fn step_rhythm(&mut self, id: Id) {
        let now = self.clock;
        loop {
            let Some(Action::Rhythm { ability, since, done, pressed, cued }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
            let d = self.content.abilities[ability].clone();
            if done >= d.most_beats {
                self.end_rhythm(id, ability, true);
                return;
            }
            let at = Self::beat_at(&d, since, done);
            // Every beat pulses on the caster as it falls, gone or awaited: the metronome.
            if now + 1.0e-4 >= at && cued <= done {
                if d.beat_kit != 0 {
                    self.events.push(Event::Visual { id, kit: d.beat_kit, by: None });
                }
                if let Some(p) = self.players.get_mut(&id) {
                    if let Action::Rhythm { cued: x, .. } = &mut p.action {
                        *x = done + 1;
                    }
                }
            }
            let earned = done < d.free_beats || pressed > done;
            if earned {
                if now + 1.0e-4 < at {
                    return;
                }
                self.go_beat(id, ability, done);
                if let Some(p) = self.players.get_mut(&id) {
                    if let Action::Rhythm { done: x, .. } = &mut p.action {
                        *x = done + 1;
                    }
                }
                continue;
            }
            // Not pressed for: wait out its window (and the caster's round trip, for a press
            // made in time that's still on its way), then it's over.
            let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
            if now > at + Self::window_of(&d, done) + rewind {
                self.end_rhythm(id, ability, false);
            }
            return;
        }
    }

    /// The key again: a press for the next beat. On time, that beat goes when it falls (at once if
    /// it's already due); out of time, the rhythm ends where it is.
    pub(crate) fn press_beat(&mut self, id: Id) {
        let Some(Action::Rhythm { ability, since, done, pressed, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        let d = self.content.abilities[ability].clone();
        // The press as they made it: its arrival less their round trip.
        let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
        let t = self.clock - rewind;
        // The beat it's for: the first not yet earned.
        let k = pressed.max(done).max(d.free_beats);
        if k >= d.most_beats {
            return;
        }
        let at = Self::beat_at(&d, since, k);
        if (t - at).abs() <= Self::window_of(&d, k) + 1.0e-4 {
            if let Some(p) = self.players.get_mut(&id) {
                if let Action::Rhythm { pressed: x, .. } = &mut p.action {
                    *x = k + 1;
                }
            }
            self.step_rhythm(id);
        } else {
            self.end_rhythm(id, ability, false);
        }
    }

    /// Beat `k` goes: a missile along the reticle, or a burst round the caster, each harder than
    /// the last; the last the finisher.
    fn go_beat(&mut self, id: Id, ability: usize, k: u32) {
        let d = self.content.abilities[ability].clone();
        let last = k + 1 >= d.most_beats;
        let mut damage = d.damage[0] + d.beat_step * k as f32;
        if last && d.finisher_damage > 0.0 {
            damage *= d.finisher_damage;
        }
        let share = if d.most_beats > 1 { k as f32 / (d.most_beats - 1) as f32 } else { 1.0 };
        if d.beat_kind == "nova" {
            let at = self.players[&id].pos;
            let radius = AbilityDef::at(d.radius, share);
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
            self.area(id, ability, at, radius, share, damage, None);
            if last && !d.finisher_effects.is_empty() {
                let (mobs, heroes) = self.in_wedge(id, ability, at, 0.0, 360.0, radius);
                let was = self.fx_from.replace(at);
                for u in mobs.into_iter().chain(heroes) {
                    self.apply_effects(id, u, &d.finisher_effects, 1.0, &d.name);
                }
                self.fx_from = was;
            }
        } else {
            let (yaw, _) = self.aim_of(id);
            if let Some(p) = self.players.get_mut(&id) {
                p.facing = yaw;
                p.cast_damage = Some(damage);
            }
            self.launch(id, ability, 1.0);
            if let Some(p) = self.players.get_mut(&id) {
                p.cast_damage = None;
            }
        }
    }

    /// The rhythm ends: a full chain is a perfect.
    fn end_rhythm(&mut self, id: Id, ability: usize, full: bool) {
        if full {
            let c = self.content.checks.clone();
            self.gain_valor(id, c.chain_valor);
            if c.perfect_kit != 0 {
                self.events.push(Event::Visual { id, kit: c.perfect_kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
        }
        self.end_ability_action(id);
        self.finish_commitment(id);
    }
}
