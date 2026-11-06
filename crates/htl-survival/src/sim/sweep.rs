//! The sweep: the stop check. A jet (`arc` wide, `range` long) starts at one side of the hero's
//! front and sweeps `sweep` degrees across it over `secs`, striking each foe it passes once. The
//! key pressed again stops it: the jet holds where it is for `stop_secs`, the foes in it take the
//! stop's effects (Cone of Cold's freeze), and a foe within `perfect_arc` of the jet's centre
//! makes it a perfect (`[checks]`: Valor, a burst). Not stopped, the sweep finishes and they're
//! only slowed. Early or late is graded by nature: the jet is simply a little to one side of
//! the foe, which anyone can see.
//!
//! The hero turns with the jet, so the cone look at the stop faces where it held. The jet's own
//! look while it sweeps is a stream of `tick_spell`'s missile to its reach, one every [`PULSE`]
//! seconds, which reads as a jet crossing the ground, until a client can draw a moving one.

use super::abilities::HERO_SIZE;
use super::*;
use crate::content::AbilityDef;

/// How often the jet's reach puffs while it sweeps.
const PULSE: f32 = 0.1;
/// A sweep's channel look replays this often.
const CHANNEL_EVERY: f32 = 0.3;

impl Survival {
    /// The sweep begins from one side, turning across the front: which side and how fast, rolled
    /// (each cast its own), and the caster's add-on told, with where the foes in reach stand, so
    /// the jet's needle and their marks are on the dial before it gets to them.
    pub(crate) fn start_sweep(&mut self, id: Id, ability: usize, yaw: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        // A meter always runs left to right, as a gauge reads (Corey); a plain sweep either way.
        let dir = if d.meter || rand::Rng::gen::<bool>(&mut self.rng) { 1.0 } else { -1.0 };
        let secs = d.secs.max(0.1) * (0.85 + 0.3 * rand::Rng::gen::<f32>(&mut self.rng));
        if let Some(p) = self.players.get_mut(&id) {
            p.sweep_dir = dir;
            p.sweep_secs = secs;
        }
        let from = yaw + dir * d.sweep.to_radians() / 2.0;
        self.set_action(id, Action::Sweep { ability, from, since: now, until: now + secs, struck: Vec::new(), held: None });
        self.events.push(Event::Auras { id });
        // The foes in the sweep's reach, by their bearing off the front (degrees, + to the left).
        let at = self.players[&id].pos;
        let (mobs, heroes) = self.in_wedge(id, ability, at, yaw, 360.0, d.range[0]);
        let reach = (d.sweep + d.arc) / 2.0;
        let marks: Vec<String> = mobs
            .iter()
            .map(|m| self.mobs[m].pos)
            .chain(heroes.iter().map(|h| self.players[h].pos))
            .map(|pos| {
                let b = facing(at, pos) - yaw;
                b.sin().atan2(b.cos()).to_degrees()
            })
            .filter(|b| b.abs() <= reach)
            .map(|b| format!("{b:.1}"))
            .collect();
        // A meter: the sweet spot rolled along the sweep, and the caster told where.
        if d.meter {
            let lo = 0.3 + d.meter_good;
            let hi = (0.92 - d.meter_good).max(lo);
            let at_share = lo + (hi - lo) * rand::Rng::gen::<f32>(&mut self.rng);
            if let Some(p) = self.players.get_mut(&id) {
                p.meter_at = at_share;
            }
            self.events.push(Event::CheckStart {
                player: id,
                ability,
                kind: "meter",
                a: secs,
                b: at_share,
                c: d.meter_perfect,
                pattern: format!("{}|{:.3}", if dir > 0.0 { "lr" } else { "rl" }, d.meter_good),
            });
            self.pulse_sweep(id, ability, from);
            return;
        }
        self.events.push(Event::CheckStart {
            player: id,
            ability,
            kind: "sweep",
            a: secs,
            b: d.sweep,
            c: d.perfect_arc,
            pattern: format!("{}|{:.1}|{}", if dir > 0.0 { "lr" } else { "rl" }, d.arc, marks.join(",")),
        });
        self.pulse_sweep(id, ability, from);
    }

    /// Where the jet points now: the facing it swept to, or holds at.
    fn sweep_facing(&self, id: Id, d: &AbilityDef, from: f32, since: f32, now: f32, held: Option<f32>) -> f32 {
        let (dir, secs) = self.players.get(&id).map_or((1.0, d.secs), |p| (p.sweep_dir, p.sweep_secs));
        let t = held.unwrap_or(now) - since;
        let share = (t / secs.max(0.1)).clamp(0.0, 1.0);
        from - dir * d.sweep.to_radians() * share
    }

    /// The jet's look this pulse: with `burst` set, a wedge of frost flashed on the ground along
    /// it (clients with the ground burst, patch 0026), so the pulses read as one jet turning;
    /// else one of `tick_spell`'s missiles to its reach.
    fn pulse_sweep(&mut self, id: Id, ability: usize, yaw: f32) {
        // The channel's spray from the hands, every few pulses.
        let kit = self.content.abilities[ability].channel_kit;
        if kit != 0 {
            let since = match self.players[&id].action { Action::Sweep { since, .. } => since, _ => self.clock };
            let t = self.clock - since;
            if t < 1.0e-4 || (t / CHANNEL_EVERY).floor() != ((t - PULSE) / CHANNEL_EVERY).floor() {
                self.events.push(Event::Visual { id, kit, by: None });
            }
        }
        let d = &self.content.abilities[ability];
        if let Some(b) = &d.burst {
            let at = self.players[&id].pos;
            let (reach, arc, secs, colour) = (d.range[0], d.arc.max(10.0), b.secs, b.colour.clone());
            self.events.push(Event::GroundBurst { at, yaw, shape: "wedge", reach, arc, width: 0.0, secs, colour });
            return;
        }
        if d.tick_spell == 0 {
            return;
        }
        let p = &self.players[&id];
        let r = d.range[0];
        let at = [p.pos[0] + yaw.cos() * r, p.pos[1] + yaw.sin() * r, p.pos[2] + 1.0];
        let spell = d.tick_spell;
        self.events.push(Event::Missile { caster: id, spell, target: Target::Point(at) });
    }

    /// The sweep's tick: the jet turns, strikes what it newly covers, and ends when its time
    /// (or its hold) is up.
    pub(crate) fn step_sweep(&mut self, id: Id, dt: f32) {
        let now = self.clock;
        let Some(Action::Sweep { ability, from, since, until, struck, held }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if now >= until {
            // A meter never stopped: it ran into the overrun.
            if held.is_none() && self.content.abilities[ability].meter {
                self.check_result(id, ability, "fail");
            }
            self.end_ability_action(id);
            // Back to ready, or the guard held meanwhile.
            self.finish_commitment(id);
            return;
        }
        let d = self.content.abilities[ability].clone();
        let yaw = self.sweep_facing(id, &d, from, since, now, held);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = yaw;
        }
        if held.is_some() {
            return;
        }
        // A puff at the reach, now and then.
        let k = ((now - since) / PULSE).floor();
        if k > ((now - dt - since) / PULSE).floor() {
            self.pulse_sweep(id, ability, yaw);
        }
        // Whom the jet covers now and hasn't struck.
        let at = self.players[&id].pos;
        let (mobs, heroes) = self.in_wedge(id, ability, at, yaw, d.arc, d.range[0]);
        let fresh: Vec<Id> = mobs.iter().chain(&heroes).copied().filter(|u| !struck.contains(u)).collect();
        if fresh.is_empty() {
            return;
        }
        let damage = d.damage[0];
        for u in &fresh {
            if mobs.contains(u) {
                self.ability_on_mob(id, *u, ability, 1.0, damage, at, HitKind::Area, d.spell);
            } else {
                let dmg = self.ability_damage(id, *u, ability, damage);
                let area = Some(combat::Area::Arc { pos: at, facing: yaw, range: d.range[0], arc: d.arc });
                self.strike_fair_as(id, *u, dmg, at, false, false, false, Blow::Arrow(d.spell), area, combat::Follow::Ability { ability, share: 1.0 }, HitKind::Area);
            }
        }
        if let Some(p) = self.players.get_mut(&id) {
            if let Action::Sweep { struck, .. } = &mut p.action {
                struck.extend(fresh);
            }
        }
    }

    /// The key again: the jet stops where it is. The foes in it take the stop's effects; one
    /// at the jet's centre makes it a perfect.
    pub(crate) fn stop_sweep(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Sweep { ability, from, since, held, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if held.is_some() {
            return;
        }
        let d = self.content.abilities[ability].clone();
        let yaw = self.sweep_facing(id, &d, from, since, now, None);
        let hold = d.stop_secs.max(0.1);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = yaw;
            if let Action::Sweep { held, until, .. } = &mut p.action {
                *held = Some(now);
                *until = now + hold;
            }
        }
        // The cone's own look, held where it stopped.
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        let at = self.players[&id].pos;
        let (mobs, heroes) = self.in_wedge(id, ability, at, yaw, d.arc, d.range[0]);
        if d.meter {
            // A meter's stop lets the frost go over the whole cone it swept (its sweet spot is
            // rolled apart from where the foes stand, so it can't hang on the jet's place).
            let dir = self.players.get(&id).map_or(1.0, |p| p.sweep_dir);
            let centre = from - dir * d.sweep.to_radians() / 2.0;
            let (all_mobs, all_heroes) = self.in_wedge(id, ability, at, centre, d.sweep + d.arc, d.range[0]);
            self.stop_meter(id, ability, since, &all_mobs, &all_heroes, at);
            return;
        }
        let perfect = d.perfect_arc > 0.0
            && mobs.iter().map(|m| self.mobs[m].pos).chain(heroes.iter().map(|h| self.players[h].pos)).any(|pos| angle_between(yaw, facing(at, pos)) <= d.perfect_arc);
        if !d.stop_effects.is_empty() {
            let was = self.fx_from.replace(at);
            for u in mobs.iter().chain(&heroes).copied().collect::<Vec<_>>() {
                self.apply_effects(id, u, &d.stop_effects, 1.0, &d.name);
            }
            self.fx_from = was;
        }
        if perfect {
            let k = self.content.checks.clone();
            self.gain_valor(id, k.perfect_valor);
            let kit = self.perfect_kit_of(ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
        }
        let grade = if perfect { "perfect" } else if mobs.is_empty() && heroes.is_empty() { "fail" } else { "good" };
        self.check_result(id, ability, grade);
    }

    /// A meter sweep stopped: graded by where along it, against this cast's sweet spot.
    fn stop_meter(&mut self, id: Id, ability: usize, since: f32, mobs: &[Id], heroes: &[Id], at: Vec3) {
        let d = self.content.abilities[ability].clone();
        let (secs, spot) = self.players.get(&id).map_or((d.secs, 0.5), |p| (p.sweep_secs, p.meter_at));
        let share = ((self.clock - since) / secs.max(0.1)).clamp(0.0, 1.0);
        let off = (share - spot).abs();
        let grade = if off <= d.meter_perfect {
            "perfect"
        } else if off <= d.meter_good {
            "good"
        } else if share < spot {
            "early"
        } else {
            "late"
        };
        // Perfect: the stop's whole; good: its least; off it, nothing more than the jet's slow.
        let share_fx = match grade {
            "perfect" => Some(1.0),
            "good" => Some(0.0),
            _ => None,
        };
        if let Some(sh) = share_fx {
            let was = self.fx_from.replace(at);
            for u in mobs.iter().chain(heroes).copied().collect::<Vec<_>>() {
                self.apply_effects(id, u, &d.stop_effects, sh, &d.name);
            }
            self.fx_from = was;
        }
        if grade == "perfect" {
            let k = self.content.checks.clone();
            self.gain_valor(id, k.perfect_valor);
            let kit = self.perfect_kit_of(ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
        }
        self.check_result(id, ability, grade);
    }

    /// The foes a wedge from `at` covers: creatures and (with friendly fire) hostile heroes.
    pub(crate) fn in_wedge(&self, owner: Id, ability: usize, at: Vec3, yaw: f32, arc: f32, range: f32) -> (Vec<Id>, Vec<Id>) {
        let inside = |pos: Vec3, size: f32| -> bool {
            let dd = dist2d(pos, at);
            // (`angle_between` answers in degrees, as the arcs are written.)
            dd <= range + size && (pos[2] - at[2]).abs() <= 4.0 && (dd < 0.8 || angle_between(yaw, facing(at, pos)) <= arc / 2.0)
        };
        let mobs: Vec<Id> = self.mobs.iter().filter(|(mid, m)| **mid != owner && m.health > 0 && !m.hidden && inside(m.pos, 0.5 * self.content.mobs[m.def].scale) && !self.own_totem(**mid, owner) && !self.spirit_body(**mid)).map(|(id, _)| *id).collect();
        let heroes: Vec<Id> = if self.content.pvp.friendly_fire {
            self.players.iter().filter(|(h, p)| self.hostile(owner, **h) && !self.spared(owner, **h, ability) && p.health > 0 && inside(p.pos, HERO_SIZE)).map(|(h, _)| *h).collect()
        } else {
            Vec::new()
        };
        (mobs, heroes)
    }
}
