//! The rhythm check: a spell that goes off on beats. The first `free_beats` go by themselves, so
//! the caster sees the tempo; each further beat goes the moment the key is pressed within its
//! window (`beat_window`, the first's tightening to the last's): the missile leaves on the press,
//! not on the beat. A press out of time, or a beat let pass, ends it where it is: what went, went,
//! and nothing hurts the caster. Each beat strikes harder than the last (`beat_step`); the last,
//! the finisher, `finisher_damage` times as hard and with `finisher_effects`. A full chain is a
//! perfect: a whole segment of Valor and the burst.
//!
//! No two casts keep quite the same time: each rolls a pattern (steady, quickening, or swung
//! long-short), and a long one may hold a rest, a beat where pressing breaks the chain. The
//! schedule goes to the caster's add-on as the rhythm starts (`Event::CheckStart`), so their
//! screen shows every beat before it falls.
//!
//! Lag: the caster follows beats they see a one-way trip late, and their press reaches the server
//! another trip later, so a press is judged at its arrival less their round trip (as far back as
//! `combat.max_rewind`).

use rand::Rng;

use super::*;
use crate::content::AbilityDef;

/// The patterns a rhythm can roll, and how much a quickening one speeds up by its end.
const PATTERNS: [&str; 3] = ["steady", "quicken", "swing"];
const QUICKEN: f32 = 0.3;
/// A swung beat's long and short gaps, as shares of the beat.
const SWING: [f32; 2] = [1.2, 0.8];

impl Survival {
    /// The rhythm begins: its schedule rolled and sent, its first beat goes now.
    pub(crate) fn start_rhythm(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let pattern = PATTERNS[self.rng.gen_range(0..PATTERNS.len())];
        let n = d.most_beats.max(1);
        let mut beats = Vec::with_capacity(n as usize);
        let mut t = 0.0;
        for k in 0..n {
            beats.push((t, false));
            let share = if n > 1 { k as f32 / (n - 1) as f32 } else { 0.0 };
            let gap = match pattern {
                "quicken" => d.beat_secs * (1.0 + QUICKEN / 2.0 - QUICKEN * share),
                "swing" => d.beat_secs * SWING[(k % 2) as usize],
                _ => d.beat_secs,
            };
            t += gap;
        }
        // A long chain may hold one rest, never on a free beat, the first pressed one or the last.
        if n >= 6 && d.free_beats + 1 < n - 1 && self.rng.gen::<f32>() < 0.5 {
            let k = self.rng.gen_range(d.free_beats + 1..n - 1) as usize;
            beats[k].1 = true;
        }
        let text = beats.iter().map(|(t, rest)| format!("{t:.3}{}", if *rest { "r" } else { "" })).collect::<Vec<_>>().join(",");
        if let Some(p) = self.players.get_mut(&id) {
            p.rhythm_beats = beats;
        }
        self.set_action(id, Action::Rhythm { ability, since: now, done: 0, pressed: 0, cued: 0 });
        let crit = self.content.checks.crit_share;
        self.events.push(Event::CheckStart {
            player: id,
            ability,
            kind: "rhythm",
            a: d.beat_window[0],
            b: d.beat_window[1],
            c: crit,
            pattern: format!("{}|{pattern}|{text}", d.free_beats),
        });
        if d.charge_spell != 0 {
            let total = self.players[&id].rhythm_beats.last().map_or(0.0, |b| b.0) + d.beat_secs;
            self.events.push(Event::Cast { caster: id, spell: d.charge_spell, ms: (total * 1000.0) as u32 });
        }
        self.step_rhythm(id);
    }

    /// When beat `k` falls, and whether it's a rest.
    fn beat_of(&self, id: Id, since: f32, k: u32) -> (f32, bool) {
        let (t, rest) = self.players.get(&id).and_then(|p| p.rhythm_beats.get(k as usize).copied()).unwrap_or((0.0, false));
        (since + t, rest)
    }

    /// How close a press must be to beat `k`: the first pressed beat's window, tightening to the
    /// last's.
    fn window_of(d: &AbilityDef, k: u32) -> f32 {
        let first = d.free_beats;
        let span = d.most_beats.saturating_sub(first + 1).max(1) as f32;
        let t = (k.saturating_sub(first) as f32 / span).clamp(0.0, 1.0);
        d.beat_window[0] + (d.beat_window[1] - d.beat_window[0]) * t
    }

    /// The pulse on the caster as beat `k` falls (the metronome), once.
    fn cue_beat(&mut self, id: Id, k: u32) {
        let Some(Action::Rhythm { ability, cued, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if cued > k {
            return;
        }
        let kit = self.content.abilities[ability].beat_kit;
        if kit != 0 {
            self.events.push(Event::Visual { id, kit, by: None });
        }
        if let Some(p) = self.players.get_mut(&id) {
            if let Action::Rhythm { cued: x, .. } = &mut p.action {
                *x = k + 1;
            }
        }
    }

    fn set_done(&mut self, id: Id, done: u32, pressed: u32) {
        if let Some(p) = self.players.get_mut(&id) {
            if let Action::Rhythm { done: d, pressed: x, .. } = &mut p.action {
                *d = done;
                *x = pressed;
            }
        }
    }

    /// The rhythm's tick: the free beats go as they fall, a rest passes, a beat already fired on
    /// a press is behind it, and one let pass ends it.
    pub(crate) fn step_rhythm(&mut self, id: Id) {
        let now = self.clock;
        loop {
            let Some(Action::Rhythm { ability, since, done, pressed, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
            let d = self.content.abilities[ability].clone();
            if done >= d.most_beats {
                self.end_rhythm(id, ability, true);
                return;
            }
            let (at, rest) = self.beat_of(id, since, done);
            if now + 1.0e-4 >= at {
                self.cue_beat(id, done);
            }
            if done < d.free_beats {
                if now + 1.0e-4 < at {
                    return;
                }
                self.go_beat(id, ability, done, false);
                self.set_done(id, done + 1, pressed);
                continue;
            }
            if pressed > done {
                self.set_done(id, done + 1, pressed);
                continue;
            }
            // Not pressed for (or a rest, rightly left): wait out its window and the caster's
            // round trip, for a press made in time that's still on its way.
            let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
            if now > at + Self::window_of(&d, done) + rewind {
                if rest {
                    self.set_done(id, done + 1, pressed.max(done + 1));
                    continue;
                }
                self.check_result(id, ability, "fail");
                self.end_rhythm(id, ability, false);
            }
            return;
        }
    }

    /// The key again: a press for the next beat. In time, that beat goes now; on a rest, or out of
    /// time, the rhythm ends where it is.
    pub(crate) fn press_beat(&mut self, id: Id) {
        let Some(Action::Rhythm { ability, since, done, pressed, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        let d = self.content.abilities[ability].clone();
        // The press as it arrives: the add-on draws the beats from the caster's own press, which
        // reached the server the same one-way trip later as this press does, so the two cancel
        // (as for the release, the meter, the sweep and the orb). Rewinding it as well put a far
        // hero's press on the beat a whole round trip early.
        let t = self.clock;
        // The beat it's for: the first not yet earned.
        let mut k = pressed.max(done).max(d.free_beats);
        if k >= d.most_beats {
            return;
        }
        let (at, rest) = self.beat_of(id, since, k);
        if rest {
            // Pressed on a rest: the chain breaks. Pressed well after it: it was for the next.
            if (t - at).abs() <= Self::window_of(&d, k) + 1.0e-4 || t < at {
                self.check_result(id, ability, "fail");
                self.end_rhythm(id, ability, false);
                return;
            }
            k += 1;
            if k >= d.most_beats {
                return;
            }
        }
        let (at, _) = self.beat_of(id, since, k);
        let w = Self::window_of(&d, k);
        let off = (t - at).abs();
        if off > w + 1.0e-4 {
            self.check_result(id, ability, "fail");
            self.end_rhythm(id, ability, false);
            return;
        }
        let critical = off <= w * self.content.checks.crit_share;
        self.events.push(Event::CheckResult { player: id, ability, grade: if critical { "perfect" } else { "good" } });
        self.cue_beat(id, k);
        self.go_beat(id, ability, k, critical);
        self.set_done(id, k + 1, k + 1);
        self.step_rhythm(id);
    }

    /// Beat `k` goes: a missile along the reticle, or a burst round the caster, each harder than
    /// the last; the last the finisher. A critical press adds the ability's `crit_effects` to
    /// what it strikes.
    fn go_beat(&mut self, id: Id, ability: usize, k: u32, critical: bool) {
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
            let mut extra = Vec::new();
            if last {
                extra.extend(d.finisher_effects.iter().cloned());
            }
            if critical {
                extra.extend(d.crit_effects.iter().cloned());
            }
            if !extra.is_empty() {
                let (mobs, heroes) = self.in_wedge(id, ability, at, 0.0, 360.0, radius);
                let was = self.fx_from.replace(at);
                for u in mobs.into_iter().chain(heroes) {
                    self.apply_effects(id, u, &extra, 1.0, &d.name);
                }
                self.fx_from = was;
            }
        } else {
            let (yaw, _) = self.aim_of(id);
            // The body's cast for this missile: the hands take turns.
            if !d.beat_clips.is_empty() {
                let clip = d.beat_clips[k as usize % d.beat_clips.len()];
                self.events.push(Event::Clip { id, clip, take: 0, from: None, secs: d.beat_secs.max(0.25), at: 0.1, body: ClipBody::Full });
            }
            if let Some(p) = self.players.get_mut(&id) {
                p.facing = yaw;
                p.cast_damage = Some(damage);
                p.cast_critical = critical;
            }
            self.launch(id, ability, 1.0);
            if let Some(p) = self.players.get_mut(&id) {
                p.cast_damage = None;
                p.cast_critical = false;
            }
        }
    }

    /// The rhythm ends: a full chain is a perfect.
    fn end_rhythm(&mut self, id: Id, ability: usize, full: bool) {
        if full {
            let c = self.content.checks.clone();
            self.gain_valor(id, c.chain_valor);
            let kit = self.perfect_kit_of(ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
            self.events.push(Event::CheckResult { player: id, ability, grade: "full" });
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.rhythm_beats.clear();
        }
        self.end_ability_action(id);
        self.finish_commitment(id);
    }
}
