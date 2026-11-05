//! The orb: the stop check, rolling. Frozen Orb rolls out along the reticle for its `linger`
//! seconds, `linger_damage` a second to every foe within `radius` of it (and the ability's
//! effects, a slow). The key again stops it where it is and it bursts: `damage` in `stop_radius`,
//! and with a foe within `perfect_yd` of it a perfect, the high end of the damage and
//! `stop_effects` (the freeze) on everything in the burst. Stopped within `backfire_yd` of the
//! caster it's a backfire: `backfire_effects` on them (they freeze). Rolled the whole way, it
//! fades with no burst. Early or late is graded by nature: the orb is simply short of the foe or
//! past them, which anyone can see.
//!
//! Its look while it rolls is `tick_spell`'s missile to where it is, every [`LOOK_EVERY`]
//! seconds; the burst is `spell`'s area look where it stopped.

use super::*;
use crate::content::AbilityDef;

/// How often the rolling orb's look is refreshed.
const LOOK_EVERY: f32 = 0.1;
/// How far ahead of the caster it starts.
const START_AHEAD: f32 = 1.5;
/// The burst's look stays this long.
const BURST_LOOK: f32 = 1.0;

/// An orb out.
#[derive(Debug, Clone)]
pub(crate) struct Orb {
    pub owner: Id,
    pub ability: usize,
    pub pos: Vec3,
    pub dir: [f32; 2],
    pub speed: f32,
    pub until: f32,
    pub next_tick: f32,
    pub next_look: f32,
}

impl Survival {
    /// The orb set rolling from the caster's feet along the reticle.
    pub(crate) fn start_orb(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let (yaw, _) = self.aim_of(id);
        let p = &self.players[&id];
        let dir = [yaw.cos(), yaw.sin()];
        let start = self.ground_near(p.pos[0] + dir[0] * START_AHEAD, p.pos[1] + dir[1] * START_AHEAD, p.pos[2]);
        // Each cast rolls at its own pace (the same reach), so the stop is read, not counted.
        let secs = d.linger.max(0.1) * (0.85 + 0.3 * rand::Rng::gen::<f32>(&mut self.rng));
        let speed = d.range[0] / secs;
        let tick = if d.tick > 0.0 { d.tick } else { 0.5 };
        // The foes along its line, by how far along it they stand, for the caster's track.
        let reach = d.radius[1].max(1.5);
        let pos = self.players[&id].pos;
        let mut along: Vec<f32> = self
            .mobs
            .values()
            .filter(|m| m.health > 0 && !m.hidden)
            .map(|m| m.pos)
            .filter_map(|q| {
                let (dx, dy) = (q[0] - pos[0], q[1] - pos[1]);
                let ahead = dx * dir[0] + dy * dir[1];
                let side = (dx * dir[1] - dy * dir[0]).abs();
                (ahead > 0.0 && ahead <= d.range[0] + 1.0 && side <= reach).then_some(ahead)
            })
            .collect();
        along.sort_by(|a, b| a.total_cmp(b));
        let marks: Vec<String> = along.iter().map(|a| format!("{a:.1}")).collect();
        self.events.push(Event::CheckStart {
            player: id,
            ability,
            kind: "orb",
            a: secs,
            b: d.range[0],
            c: d.perfect_yd,
            pattern: format!("{:.1}|{:.1}|{}", d.backfire_yd, START_AHEAD, marks.join(",")),
        });
        self.orbs.push(Orb { owner: id, ability, pos: start, dir, speed, until: now + secs, next_tick: now + tick, next_look: now });
    }

    /// The orb of this key's base ability the hero has out, if any.
    pub(crate) fn orb_of(&self, id: Id, base: usize) -> Option<usize> {
        self.orbs.iter().position(|o| o.owner == id && self.content.base_of(o.ability) == base)
    }

    /// Orbs roll on, hurt what they pass, and fade when spent.
    pub(crate) fn step_orbs(&mut self, dt: f32) {
        let now = self.clock;
        let list = std::mem::take(&mut self.orbs);
        let mut keep = Vec::new();
        for mut o in list {
            if now >= o.until || !self.players.contains_key(&o.owner) {
                continue;
            }
            let d = self.content.abilities[o.ability].clone();
            let go = o.speed * dt;
            let (x, y) = (o.pos[0] + o.dir[0] * go, o.pos[1] + o.dir[1] * go);
            // It rolls over the ground, and a wall ends it (no burst: it was never stopped).
            if self.land.blocked_near(x, y, o.pos[2]) {
                continue;
            }
            o.pos = self.ground_near(x, y, o.pos[2]);
            if now >= o.next_look {
                o.next_look = now + LOOK_EVERY;
                if d.tick_spell != 0 {
                    let at = [o.pos[0], o.pos[1], o.pos[2] + 1.0];
                    self.events.push(Event::Missile { caster: o.owner, spell: d.tick_spell, target: Target::Point(at) });
                }
            }
            if now >= o.next_tick {
                let tick = if d.tick > 0.0 { d.tick } else { 0.5 };
                o.next_tick = now + tick;
                let dps = d.linger_damage;
                let per = (dps * tick).round().max(1.0) as u32;
                let (mobs, heroes) = self.near_orb(&o, d.radius[0]);
                for m in &mobs {
                    self.events.push(Event::Swing { attacker: o.owner, victim: Some(*m), damage: per, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
                    self.hurt_mob_as(*m, o.owner, per, &d.name);
                }
                for h in &heroes {
                    let dmg = self.ability_damage(o.owner, *h, o.ability, per as f32);
                    self.hurt_player_by(*h, dmg, o.owner, &d.name);
                }
                if !d.effects.is_empty() {
                    let was = self.fx_from.replace(o.pos);
                    for u in mobs.iter().chain(&heroes).copied().collect::<Vec<_>>() {
                        self.apply_effects(o.owner, u, &d.effects, 1.0, &d.name);
                    }
                    self.fx_from = was;
                }
            }
            keep.push(o);
        }
        self.orbs.extend(keep);
    }

    /// The foes within `radius` of the orb: creatures and (with friendly fire) hostile heroes.
    fn near_orb(&self, o: &Orb, radius: f32) -> (Vec<Id>, Vec<Id>) {
        let at = o.pos;
        let mobs: Vec<Id> = self.mobs.iter().filter(|(mid, m)| m.health > 0 && !m.hidden && dist2d(m.pos, at) <= radius + 0.5 * self.content.mobs[m.def].scale && !self.own_totem(**mid, o.owner) && !self.spirit_body(**mid)).map(|(id, _)| *id).collect();
        let heroes: Vec<Id> = if self.content.pvp.friendly_fire {
            self.players.iter().filter(|(h, p)| self.hostile(o.owner, **h) && !self.spared(o.owner, **h, o.ability) && p.health > 0 && dist2d(p.pos, at) <= radius + abilities::HERO_SIZE).map(|(h, _)| *h).collect()
        } else {
            Vec::new()
        };
        (mobs, heroes)
    }

    /// The key again: the orb stops where it is and bursts. A foe under it makes it a perfect;
    /// the caster under it makes it a backfire.
    pub(crate) fn stop_orb(&mut self, id: Id, k: usize) {
        let o = self.orbs.remove(k);
        let d = self.content.abilities[o.ability].clone();
        let at = o.pos;
        let (mobs, heroes) = self.near_orb(&o, d.perfect_yd.max(0.0));
        let perfect = d.perfect_yd > 0.0 && !(mobs.is_empty() && heroes.is_empty());
        let share = if perfect { 1.0 } else { 0.0 };
        let damage = AbilityDef::at(d.damage, share);
        let radius = if d.stop_radius > 0.0 { d.stop_radius } else { d.radius[1] };
        // The burst's look where it stopped, then its blow.
        if d.spell != 0 {
            let m = self.mark_unseen(BURST_LOOK);
            self.events.push(Event::AreaLook { id: m, caster: id, spell: d.spell, pos: at, radius });
        }
        self.area(id, o.ability, at, radius, share, damage, None);
        if perfect {
            if !d.stop_effects.is_empty() {
                let (mobs, heroes) = self.near_orb(&o, radius);
                let was = self.fx_from.replace(at);
                for u in mobs.iter().chain(&heroes).copied().collect::<Vec<_>>() {
                    self.apply_effects(id, u, &d.stop_effects, 1.0, &d.name);
                }
                self.fx_from = was;
            }
            let c = self.content.checks.clone();
            self.gain_valor(id, c.perfect_valor);
            let kit = self.perfect_kit_of(o.ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability: o.ability });
        }
        let own_yd = self.players.get(&id).map(|p| dist2d(p.pos, at)).unwrap_or(f32::MAX);
        let backfire = d.backfire_yd > 0.0 && own_yd <= d.backfire_yd;
        let grade = if backfire { "fail" } else if perfect { "perfect" } else { "good" };
        self.check_result(id, o.ability, grade);
        // Stopped at the caster's own feet: the cold takes them too.
        let own = self.players.get(&id).map(|p| dist2d(p.pos, at)).unwrap_or(f32::MAX);
        if d.backfire_yd > 0.0 && own <= d.backfire_yd && !d.backfire_effects.is_empty() {
            let was = self.fx_from.replace(at);
            self.apply_effects(id, id, &d.backfire_effects, 1.0, &d.name);
            self.fx_from = was;
        }
    }
}
