//! The Warrior's second choices, the Warlord: Tackle's rush that carries one enemy and Crashes it
//! into what's behind it, Intervene's rush to a friend to take the blow meant for them, the shouts
//! that give the group stamina (Rallying Cry) and harder heavy attacks (Battle Cry), Bladestorm's
//! ramping spin, Cleave's cash-in on an opened enemy, Thunder Clap's slowed swings, Ravager's axe
//! torn back through the pack, and Heroic Throw's Taunt.
//!
//! An **opened** enemy is one that's staggered, knocked down, stunned, or (a boss) stance-broken:
//! [`Survival::opened`] is the one check, for Cleave and anything else that pays one off.

use std::collections::BTreeMap;

use super::abilities::HERO_SIZE;
use super::*;
use crate::content::{AbilityDef, AbilityEffect};

/// The fall a hop flies under (as the clients fly it).
const GRAVITY: f32 = 19.29;
/// A rush (and a carried body) goes this far a step as it's traced.
const STEP: f32 = 0.4;
/// A carried body is held this far in front of the hero's body.
const CARRY_GAP: f32 = 0.2;
/// Intervene holds while the friend stays this near the Warrior (yards).
const INTERVENE_REACH: f32 = 8.0;
/// A boss shrugs off this share of a slowed swing's slow, and of its time (Thunder Clap: 30%
/// slower for 6 sec on creatures, 15% for 4 sec on a boss).
const BOSS_SLOW_SHARE: f32 = 0.5;
const BOSS_SLOW_SECS: f32 = 2.0 / 3.0;

/// Where a Tackle's rush ends for the body it carries.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Crash {
    /// Nothing met: the hero just rushes.
    None,
    /// Something that won't move stopped the rush (a boss): posture.
    Bang(Id),
    /// Carried into a wall, a tree or a rock.
    Wall(Id),
    /// Carried into another body (the second is `into`).
    Into(Id, Id),
    /// Carried to the end of the rush and dropped.
    Drop(Id),
}

/// What the Warlord's mechanics keep between ticks.
#[derive(Debug, Clone, Default)]
pub(crate) struct Warlord {
    /// Tackles under way: the rush's ability and how it ends.
    /// Tackles under way: the rush's ability, how it ends, and where the hero stands then.
    pub tackles: BTreeMap<Id, (usize, Crash, Vec3)>,
}

impl Survival {
    // --- opened -------------------------------------------------------------------------------------

    /// Opened: staggered, knocked down or stunned, or (a boss) reeling from a broken stance. A
    /// knockdown is a stun that looks it, so one check covers both.
    pub fn opened(&self, id: Id) -> bool {
        let now = self.clock;
        if self.has_cc(id, "stun") {
            return true;
        }
        if let Some(m) = self.mobs.get(&id) {
            if self.content.mobs[m.def].boss {
                return self.stance_broken(id);
            }
            return m.stagger_until > now;
        }
        self.players.get(&id).is_some_and(|p| matches!(p.action, Action::Stagger { until } if until > now))
    }

    // --- Tackle -------------------------------------------------------------------------------------

    /// Won't be carried: a boss or a totem, something untouchable or unstaggerable, a training
    /// dummy.
    fn stands_firm(&self, id: Id) -> bool {
        self.holds_ground(id) || self.immune(id) || self.unstaggerable(id) || self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].inert())
    }

    fn body_size(&self, id: Id) -> f32 {
        self.mobs.get(&id).map_or(HERO_SIZE, |m| 0.5 * self.content.mobs[m.def].scale)
    }

    /// Tackle: the rush along the aim. The first body it meets is carried with it to its end (a
    /// hero at most `hero_carry` yd), and what's there decides the Crash; something that won't move
    /// stops it. It all lands as the rush ends ([`Self::tackle_lands`]).
    pub(crate) fn start_tackle(&mut self, id: Id, ability: usize, yaw: f32, share: f32) {
        let d = self.content.abilities[ability].clone();
        let Some(crash) = d.crash.clone() else { return };
        let range = AbilityDef::at(d.range, share);
        let pos = self.players[&id].pos;
        let (c, s) = (yaw.cos(), yaw.sin());
        let bodies = self.bodies(id);
        // The hero's rush, to the first body (or a wall, a drop, a climb, the range).
        let mut dist = 0.0f32;
        let mut z = pos[2];
        let mut met = None;
        while dist < range {
            let next = (dist + STEP).min(range);
            let (x, y) = (pos[0] + c * next, pos[1] + s * next);
            let g = self.land.height_near(x, y, z + 1.0);
            if self.land.blocked_near(x, y, g) || (g - z).abs() > 1.2 {
                break;
            }
            if let Some((b, ..)) = bodies.iter().find(|(_, feet, _, size)| dist2d(*feet, [x, y, g]) <= size + HERO_SIZE + CARRY_GAP && (feet[2] - g).abs() < 2.5) {
                met = Some(*b);
                break;
            }
            z = g;
            dist = next;
        }
        let (outcome, rush) = match met {
            None => (Crash::None, dist),
            Some(b) if self.stands_firm(b) => (Crash::Bang(b), dist),
            Some(b) => {
                let left = (range - dist).max(0.0);
                let carry = if self.players.contains_key(&b) { left.min(crash.hero_carry) } else { left };
                let (end, outcome) = self.carry_path(id, b, yaw, carry, &bodies);
                let size = self.body_size(b);
                // The hero ends behind the body, shoulder to it.
                let back = size + HERO_SIZE + CARRY_GAP;
                let along = (end[0] - pos[0]) * c + (end[1] - pos[1]) * s - back;
                let secs = (along.max(dist) / DASH_SPEED).max(0.08);
                self.carry_body(id, b, end, yaw, secs);
                (outcome, along.max(dist))
            }
        };
        let secs = (rush / DASH_SPEED).max(0.08);
        let end = [pos[0] + c * rush, pos[1] + s * rush, pos[2]];
        let end = self.ground_near(end[0], end[1], pos[2] + 1.0);
        self.warlord.tackles.insert(id, (ability, outcome, end));
        self.set_action(id, Action::Dash { t: 0.0, secs, ability, target: None, share });
        self.events.push(Event::Dash { player: id, dir: yaw, speed: rush / secs, secs });
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
    }

    /// Where body `b` carried `carry` yd along `yaw` stops, and why: a wall (or a steep step up)
    /// in its way, another body it meets, or the end of the carry.
    fn carry_path(&self, owner: Id, b: Id, yaw: f32, carry: f32, bodies: &[(Id, Vec3, f32, f32)]) -> (Vec3, Crash) {
        let Some(from) = self.unit_pos(b) else { return ([0.0; 3], Crash::Drop(b)) };
        let size = self.body_size(b);
        let (c, s) = (yaw.cos(), yaw.sin());
        let mut at = from;
        let mut t = 0.0f32;
        loop {
            // What's just in front of the body where it is now.
            let ahead = [at[0] + c * (size + 0.2), at[1] + s * (size + 0.2)];
            let g = self.land.height_near(ahead[0], ahead[1], at[2] + 1.0);
            if self.land.blocked_near(ahead[0], ahead[1], g) || g - at[2] > 1.5 {
                return (at, Crash::Wall(b));
            }
            let other = bodies.iter().find(|(o, feet, _, osize)| *o != b && *o != owner && dist2d(*feet, at) <= size + osize + 0.25 && (feet[2] - at[2]).abs() < 2.5);
            if let Some((o, ..)) = other {
                return (at, Crash::Into(b, *o));
            }
            if t >= carry {
                return (at, Crash::Drop(b));
            }
            let next = (t + STEP).min(carry);
            let (x, y) = (from[0] + c * next, from[1] + s * next);
            let g = self.land.height_near(x, y, at[2] + 1.0);
            if (g - at[2]).abs() > 1.5 {
                // A drop: it's let go at the edge.
                return (at, Crash::Drop(b));
            }
            at = [x, y, g];
            t = next;
        }
    }

    /// Body `b` carried to `to` over `secs` on the hero's shoulder: whatever it had in hand lost.
    fn carry_body(&mut self, owner: Id, b: Id, to: Vec3, yaw: f32, secs: f32) {
        let Some(from) = self.unit_pos(b) else { return };
        let moved = dist2d(from, to);
        let line = format!("tackle carries {} ({moved:.1} yd) from {}", self.unit_name(b), self.unit_name(owner));
        self.cc_note(b, line.clone());
        self.cc_note(owner, line);
        if self.players.contains_key(&b) {
            self.interrupt_hero(b);
            let now = self.clock;
            self.set_action(b, Action::Stagger { until: now + secs + 0.1 });
            // A low hop the length of the rush (the client flies it).
            let lift = 0.5 * GRAVITY * secs;
            self.events.push(Event::Hop { player: b, dir: yaw, speed: moved / secs.max(0.05), lift, roll: false, style: "" });
            if let Some(p) = self.players.get_mut(&b) {
                p.pos = to;
            }
        } else {
            self.stagger_mob(b, secs + 0.1, false);
            if moved > 0.05 {
                self.throw_mob(b, to, secs, Some(owner));
            }
        }
    }

    /// A Tackle's rush ends: the Crash it was heading for lands.
    pub(crate) fn tackle_lands(&mut self, id: Id) {
        let Some((ability, outcome, end)) = self.warlord.tackles.remove(&id) else { return };
        let d = self.content.abilities[ability].clone();
        let Some(crash) = d.crash.clone() else { return };
        // Where the rush took them (their client ran it), unless something held them on the way.
        let held = self.held_still(id);
        if let Some(p) = self.players.get_mut(&id).filter(|p| p.health > 0 && !held) {
            p.pos = end;
        }
        let from = self.players.get(&id).map_or([0.0; 3], |p| p.pos);
        match outcome {
            Crash::None => {}
            Crash::Bang(b) => {
                if self.alive(b) && self.mobs.contains_key(&b) {
                    self.build_posture(b, id, crash.posture);
                }
                self.crash_looks(id, b, "Stopped hard");
            }
            Crash::Wall(b) if self.alive(b) => {
                self.crash_hit(id, b, ability, crash.wall_damage, from);
                if self.alive(b) {
                    self.apply_cc_look(b, id, "stun", crash.wall_stun, 0.0, 0.0, &d.name, 0);
                }
                self.crash_looks(id, b, "Crash!");
            }
            Crash::Into(b, o) => {
                for (u, first) in [(b, true), (o, false)] {
                    if !self.alive(u) {
                        continue;
                    }
                    // A body carried into a boss: the boss's posture takes it, not its health.
                    if self.stands_firm(u) && !first {
                        if self.mobs.contains_key(&u) {
                            self.build_posture(u, id, crash.boss_posture);
                        }
                        continue;
                    }
                    self.crash_hit(id, u, ability, crash.foe_damage, from);
                    if self.alive(u) {
                        self.knock_down(id, u, crash.knockdown, &d.name);
                    }
                }
                self.crash_looks(id, b, "Crash!");
                if self.alive(o) {
                    self.events.push(Event::Visual { id: o, kit: combat::KIT_DUST, by: None });
                }
            }
            Crash::Drop(b) | Crash::Wall(b) => {
                if self.alive(b) {
                    let dmg = AbilityDef::at(d.damage, 1.0);
                    self.crash_hit(id, b, ability, dmg, from);
                }
            }
        }
    }

    fn crash_hit(&mut self, id: Id, v: Id, ability: usize, damage: f32, from: Vec3) {
        if damage > 0.0 {
            self.hunt_hit(id, v, ability, damage, from);
        }
    }

    /// The Crash seen: dust and a heavy flash on the one crashed, and the hero told.
    fn crash_looks(&mut self, id: Id, b: Id, text: &str) {
        self.events.push(Event::Visual { id: b, kit: combat::KIT_DUST, by: None });
        let kit = self.heavy_kit();
        self.events.push(Event::Visual { id: b, kit, by: Some(id) });
        self.events.push(Event::Flash { player: id, text: text.into() });
    }

    // --- Intervene ----------------------------------------------------------------------------------

    /// Intervene: the rush to the friend nearest the reticle, and the first blow meant for them
    /// within `intervene` seconds strikes the Warrior instead. With no friend in reach nothing is
    /// spent.
    pub(crate) fn intervene(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let radius = AbilityDef::at(d.radius, share);
        let friend = self.ally_target(id, range, radius);
        let reach = self.unit_pos(friend).zip(self.unit_pos(id)).is_some_and(|(a, b)| dist2d(a, b) <= range + 1.0);
        if friend == id || !reach {
            self.events.push(Event::Flash { player: id, text: "No ally in reach".into() });
            let base = self.content.base_of(ability);
            if let Some(p) = self.players.get_mut(&id) {
                p.cooldowns.remove(&base);
                p.valor = (p.valor + d.valor as f32).min(self.content.valor.max);
            }
            self.events.push(Event::Valor { player: id });
            self.events.push(Event::Abilities { player: id });
            self.set_action(id, Action::Ready);
            return;
        }
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        let at = self.unit_pos(friend).unwrap_or(self.players[&id].pos);
        self.yank_to(id, ability, share, at, Some(friend));
        if !matches!(self.players[&id].action, Action::Dash { .. }) {
            self.set_action(id, Action::Ready);
        }
        // The guard on the friend, its look on them while it holds.
        self.apply_cc_look(friend, id, "guarded", d.intervene, 1.0, 0.0, &d.name, d.aura);
    }

    /// A blow from `attacker` meant for `victim`: if a Warrior has Intervened for them (near, and
    /// up), it strikes the Warrior instead, turned to it; the guard is spent.
    pub(crate) fn intervened(&mut self, attacker: Id, victim: Id) -> Option<Id> {
        let now = self.clock;
        let g = self.cc_of(victim).iter().find(|c| c.kind == "guarded" && c.until > now)?.from;
        if g == attacker || g == victim || !self.players.get(&g).is_some_and(|p| p.health > 0) {
            return None;
        }
        let (Some(at), Some(v)) = (self.unit_pos(g), self.unit_pos(victim)) else { return None };
        if dist2d(at, v) > INTERVENE_REACH {
            return None;
        }
        self.cleanse(victim, &["guarded"]);
        // Turned to the blow, so a raised guard takes it.
        if let Some(from) = self.unit_pos(attacker) {
            if let Some(p) = self.players.get_mut(&g) {
                p.facing = facing(p.pos, from);
            }
        }
        self.events.push(Event::Visual { id: g, kit: combat::KIT_BASH, by: None });
        self.events.push(Event::Flash { player: g, text: "Intervened".into() });
        let line = format!("Intervene: {} takes the blow meant for {}", self.unit_name(g), self.unit_name(victim));
        self.cc_note(g, line.clone());
        self.cc_note(victim, line);
        Some(g)
    }

    // --- the shouts ---------------------------------------------------------------------------------

    /// A nova's friends in it (the caster too): stamina back, its boons, its look on each.
    pub(crate) fn shout_to_friends(&mut self, owner: Id, ability: usize, at: Vec3, radius: f32) {
        let d = self.content.abilities[ability].clone();
        if d.ally_stamina <= 0.0 && d.ally_effects.is_empty() && d.ally_spell == 0 {
            return;
        }
        let most = self.content.player.stamina;
        let friends: Vec<Id> = self.players.iter().filter(|(h, p)| self.helpable(owner, **h) && p.health > 0 && dist2d(p.pos, at) <= radius + HERO_SIZE && (p.pos[2] - at[2]).abs() < 6.0).map(|(h, _)| *h).collect();
        for f in friends {
            if d.ally_stamina > 0.0 {
                if let Some(p) = self.players.get_mut(&f) {
                    p.stamina = (p.stamina + d.ally_stamina).min(most);
                }
                self.events.push(Event::Stamina { player: f });
            }
            if !d.ally_effects.is_empty() {
                self.apply_effects(owner, f, &d.ally_effects, 1.0, &d.name);
            }
            if d.ally_spell != 0 {
                self.events.push(Event::Missile { caster: owner, spell: d.ally_spell, target: Target::Unit(f) });
            }
        }
    }

    /// Battle Cry on a hero: how much more their heavy attacks do to posture and to a stagger's
    /// length (0 without it).
    pub(crate) fn battle_cry(&self, id: Id) -> f32 {
        let now = self.clock;
        self.cc_of(id).iter().filter(|c| c.kind == "battle_cry" && c.until > now).map(|c| c.amount).fold(0.0, f32::max)
    }

    /// A hero's heavy attack is landing now (Battle Cry's heavies).
    pub(crate) fn heavy_landing(&self, id: Id) -> bool {
        matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Swing { strike, .. }) if strike.heavy())
    }

    // --- Thunder Clap -------------------------------------------------------------------------------

    /// How much slower a creature's attacks wind up (Thunder Clap), a boss's by half.
    pub(crate) fn swing_slow(&self, mid: Id) -> f32 {
        let now = self.clock;
        let slow = self.cc_of(mid).iter().filter(|c| c.kind == "slow_attacks" && c.until > now).map(|c| c.amount).fold(0.0, f32::max);
        if self.mobs.get(&mid).is_some_and(|m| self.content.mobs[m.def].boss) {
            slow * BOSS_SLOW_SHARE
        } else {
            slow
        }
    }

    /// A slowed swing's time on a boss: shorter (its 6 sec are 4 on one).
    pub(crate) fn slow_secs(&self, victim: Id, secs: f32) -> f32 {
        if self.mobs.get(&victim).is_some_and(|m| self.content.mobs[m.def].boss) {
            secs * BOSS_SLOW_SECS
        } else {
            secs
        }
    }

    // --- Taunt --------------------------------------------------------------------------------------

    /// Who taunted this unit, while it lasts.
    pub(crate) fn taunted_by(&self, id: Id) -> Option<Id> {
        let now = self.clock;
        self.cc_of(id).iter().find(|c| c.kind == "taunt" && c.until > now).map(|c| c.from)
    }

    /// A Taunted hero's damage on anyone but the one who taunted them is less (by the Taunt's
    /// `amount`).
    pub(crate) fn taunted_damage(&self, attacker: Id, victim: Id, damage: u32) -> u32 {
        if !self.players.contains_key(&attacker) {
            return damage;
        }
        let now = self.clock;
        match self.cc_of(attacker).iter().find(|c| c.kind == "taunt" && c.until > now) {
            Some(c) if c.from != victim => (damage as f32 * (1.0 - c.amount).clamp(0.0, 1.0)).round() as u32,
            _ => damage,
        }
    }

    /// Every tick: a Taunted creature turns on the hero who taunted it (once its blow in hand is
    /// done), whoever it was fighting.
    pub(crate) fn step_warlord(&mut self) {
        let turn: Vec<(Id, Id)> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0 && !matches!(m.state, mobs::MobState::Windup { .. } | mobs::MobState::Dead { .. }))
            .filter_map(|(mid, m)| {
                let t = self.taunted_by(*mid)?;
                let alive = self.players.get(&t).is_some_and(|p| p.health > 0);
                (alive && m.state.target() != Some(t)).then_some((*mid, t))
            })
            .collect();
        for (mid, t) in turn {
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.state = match m.state {
                    mobs::MobState::Recover { until, .. } => mobs::MobState::Recover { foe: mobs::Foe::Hero(t), until },
                    _ => mobs::MobState::Chase { foe: mobs::Foe::Hero(t) },
                };
                m.pattern = None;
                m.lost_at = None;
            }
        }
    }

    // --- Ravager ------------------------------------------------------------------------------------

    /// Ravager's recall: the axe its first cast left spinning is torn out and back to the hero,
    /// striking every enemy on its way (once), and the ground it held ends.
    pub(crate) fn tear_back(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let key = d.key.clone();
        let Some(k) = self.burnings.iter().position(|b| b.owner == id && self.content.abilities[b.ability].recast == key) else {
            self.events.push(Event::Flash { player: id, text: format!("No {} to recall", d.name) });
            return;
        };
        let b = self.burnings.remove(k);
        if let Some(m) = b.mark {
            self.unmark(m);
        }
        let Some(home) = self.players.get(&id).map(|p| p.pos) else { return };
        let len = dist2d(b.at, home);
        let yaw = facing(b.at, home);
        let half = AbilityDef::at(d.radius, share);
        // Its look flown home from where it stood.
        let anchor = self.alloc();
        let from = [b.at[0], b.at[1], b.at[2] + 1.0];
        self.events.push(Event::MissileFrom { anchor, caster: id, spell: d.spell, from, target: Target::Unit(id) });
        let speed = if d.speed > 0.0 { d.speed } else { 30.0 };
        self.hunt.anchors.push((anchor, self.clock + len / speed + 0.2));
        let damage = AbilityDef::at(d.damage, share);
        self.area_shape(id, ability, b.at, len, share, damage, super::abilities::Shape::Line(yaw, len, half));
    }

    /// A projectile's hit that taunts (Heroic Throw) or slows swings: the time on a boss is its own.
    pub(crate) fn warlord_effect(&mut self, owner: Id, victim: Id, e: &AbilityEffect, secs: f32, source: &str) {
        let secs = if e.kind == "slow_attacks" { self.slow_secs(victim, secs) } else { secs };
        let took = self.apply_cc_look(victim, owner, &e.kind, secs, e.amount, 0.0, source, e.aura);
        // Taunted: it turns on them at once.
        if took && e.kind == "taunt" && self.mobs.contains_key(&victim) && self.players.contains_key(&owner) {
            let winding = self.mobs.get(&victim).is_some_and(|m| matches!(m.state, mobs::MobState::Windup { .. }));
            if !winding {
                if let Some(m) = self.mobs.get_mut(&victim) {
                    if matches!(m.state, mobs::MobState::Idle { .. } | mobs::MobState::Return) {
                        m.state = mobs::MobState::Chase { foe: mobs::Foe::Hero(owner) };
                    }
                }
            }
            self.step_warlord();
        }
    }
}
