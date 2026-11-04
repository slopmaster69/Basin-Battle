//! The Warrior's two trees' own mechanics.
//!
//! The Warlord (the second choices, the commander who holds the line): Tackle's rush that carries
//! one enemy and Crashes it into what's behind it, Intervene's rush to a friend to take the blow
//! meant for them, the War Banner planted for the group (moved by its recast, felled by blows),
//! Intimidating Roar's Cowed, Rising Cut's Exposed, Cleave's cash-in on an opened enemy, and
//! Rallying Cry's group back on its feet.
//!
//! The Juggernaut (the first choices, the berserker): Momentum, stacked by the hero's own blows,
//! Berserker Rage breaking free of what holds them, and Blood Fury's attacks paid for in blood.
//!
//! An **opened** enemy is one that's staggered, knocked down, stunned, Exposed, or (a boss)
//! stance-broken: [`Survival::opened`] is the one check, for Cleave and anything else that pays
//! one off.

use std::collections::BTreeMap;

use super::abilities::HERO_SIZE;
use super::*;
use crate::content::{AbilityDef, AbilityEffect, BannerDef};

/// The fall a hop flies under (as the clients fly it).
const GRAVITY: f32 = 19.29;
/// A rush (and a carried body) goes this far a step as it's traced.
const STEP: f32 = 0.4;
/// A carried body is held this far in front of the hero's body.
const CARRY_GAP: f32 = 0.2;
/// Intervene holds while the friend stays this near the Warrior (yards).
const INTERVENE_REACH: f32 = 8.0;
/// A Cowed creature winds its blows up this much slower (Intimidating Roar: 15%, the designer's,
/// enough to read and parry, not enough to stop it fighting). A boss's don't slow at all.
const COWED_SLOW: f32 = 0.15;
/// A Cowed boss's blows are only this much softer (10%): never shut down, as every boss.
const COWED_BOSS_CUT: f32 = 0.10;
/// A Cowed creature (not a boss) below this share of its health flees instead of fighting, once a
/// Cow (20%: the ones about to die anyway, so it ends a fight rather than resets it).
const COWED_FLEE_BELOW: f32 = 0.2;
/// Every hero's hit on an Exposed enemy staggers it this much longer (Rising Cut: 30%).
const EXPOSED_STAGGER: f32 = 0.3;
/// A boss is Exposed this share of the time (Rising Cut's 5 sec are 2.5 on one: a window, never
/// a shutdown).
const BOSS_EXPOSED_SHARE: f32 = 0.5;
/// A War Banner is this wide: a blow reaching this near it strikes it.
const BANNER_SIZE: f32 = 0.5;
/// The look over a hero near a War Banner is kept on this long at a time, again each tick they
/// stand near it (so it goes soon after they step away).
const BANNER_HOLD: f32 = 0.3;
/// A Tackle's Crash on the ground: War Stomp's shock ring under the body as it hits the wall or
/// the other enemy (the kit Heroic Leap lands with, so it's known to show), beside the dust and the
/// heavy flash. Corey's review: the hit against a unit or a wall needs an impact mark.
const KIT_CRASH: u32 = 5089;
/// With nothing behind it, a tackled enemy isn't set down: it's flung this far ahead of the hero
/// (Corey's review: in the open the knockback has to be seen to move them).
const DROP_SHOVE: f32 = 4.0;
/// The enemy a tackled body is driven into is shoved this far by the Crash.
const INTO_SHOVE: f32 = 2.5;

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

/// A War Banner standing: the ability that planted it (its `banner` rules), its look on the
/// ground, where, what's left of its health, and when it comes down.
#[derive(Debug, Clone)]
pub(crate) struct Banner {
    pub ability: usize,
    pub obj: Id,
    pub at: Vec3,
    pub health: f32,
    pub until: f32,
}

/// What the Warrior's mechanics keep between ticks.
#[derive(Debug, Clone, Default)]
pub(crate) struct Warlord {
    /// Tackles under way: the rush's ability, how it ends, and where the hero stands then.
    pub tackles: BTreeMap<Id, (usize, Crash, Vec3)>,
    /// Each hero's War Banner, while it stands (one each).
    pub banners: BTreeMap<Id, Banner>,
    /// Cowed creatures that have fled, until their Cow ends (each flees once a Cow).
    pub fled: BTreeMap<Id, f32>,
}

impl Survival {
    // --- opened -------------------------------------------------------------------------------------

    /// Opened: staggered, knocked down or stunned, Exposed, or (a boss) reeling from a broken
    /// stance. A knockdown is a stun that looks it, so one check covers both.
    pub fn opened(&self, id: Id) -> bool {
        let now = self.clock;
        if self.has_cc(id, "stun") || self.has_cc(id, "exposed") {
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

    fn is_boss(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].boss)
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
        // The shoulder hit's clip over the rush, as a dash's (twice the rush, so its blow lands as
        // the rush ends). Without it the Tackle was Charge's run and nothing else.
        if d.clip != 0 {
            self.events.push(Event::Clip { id, clip: d.clip, take: d.take, from: d.from.clone(), secs: (secs * 2.0).max(0.6) });
        }
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
                        // The one driven into is shoved on by the Crash, then both go down.
                        if !first {
                            self.shove(id, u, INTO_SHOVE);
                        }
                        self.knock_down(id, u, crash.knockdown, &d.name);
                    }
                }
                self.crash_looks(id, b, "Crash!");
                if self.alive(o) {
                    self.events.push(Event::Visual { id: o, kit: combat::KIT_DUST, by: None });
                    self.events.push(Event::Visual { id: o, kit: KIT_CRASH, by: None });
                }
            }
            Crash::Drop(b) | Crash::Wall(b) => {
                if self.alive(b) {
                    let dmg = AbilityDef::at(d.damage, 1.0);
                    self.crash_hit(id, b, ability, dmg, from);
                    // Nothing to crash into: flung on ahead, so the hit is seen to move them.
                    if self.alive(b) && matches!(outcome, Crash::Drop(_)) {
                        self.shove(id, b, DROP_SHOVE);
                    }
                }
            }
        }
    }

    /// A body thrown `yards` on from the hero (a Tackle's drop, or the one its Crash shoves): a
    /// knockback away from the hero, as the rules fly one.
    fn shove(&mut self, owner: Id, victim: Id, yards: f32) {
        let e = AbilityEffect { kind: "knockback".into(), amount: yards, from_caster: true, ..Default::default() };
        self.displace(owner, victim, &e);
    }

    fn crash_hit(&mut self, id: Id, v: Id, ability: usize, damage: f32, from: Vec3) {
        if damage > 0.0 {
            self.hunt_hit(id, v, ability, damage, from);
        }
    }

    /// The Crash seen: dust, the shock ring and a heavy flash on the one crashed, and the hero
    /// told.
    fn crash_looks(&mut self, id: Id, b: Id, text: &str) {
        self.events.push(Event::Visual { id: b, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::Visual { id: b, kit: KIT_CRASH, by: None });
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

    // --- Rallying Cry -------------------------------------------------------------------------------

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

    /// How much quicker a hero's blows come (Rallying Cry's `rallied`: the Warrior's own 20%, the
    /// group's 10%).
    pub(crate) fn rallied_haste(&self, id: Id) -> f32 {
        let now = self.clock;
        self.cc_of(id).iter().filter(|c| c.kind == "rallied" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max)
    }

    // --- breaking free (Rallying Cry, Berserker Rage) ------------------------------------------------

    /// Up from the ground and out of a stagger: what holds them of `kinds` ends (a knockdown is a
    /// stun), and a stagger in hand is over at once, its clip let go.
    pub(crate) fn break_free(&mut self, id: Id, kinds: &[&str]) {
        let now = self.clock;
        let held = self.cc_of(id).iter().any(|c| c.until > now && kinds.contains(&c.kind.as_str()));
        self.cleanse(id, kinds);
        let staggered = matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Stagger { .. }));
        if staggered {
            self.set_action(id, Action::Ready);
        }
        if held || staggered {
            self.events.push(Event::ClipStop { id });
            let line = format!("{} breaks free", self.unit_name(id));
            self.cc_note(id, line);
        }
    }

    // --- Blood Fury, fury ---------------------------------------------------------------------------

    /// Blood Fury's price: `share` of the health they have now, never their last point.
    pub(crate) fn pay_health(&mut self, id: Id, share: f32, source: &str) {
        let Some(h) = self.players.get(&id).map(|p| p.health) else { return };
        let cost = ((h as f32 * share).round() as u32).min(h.saturating_sub(1));
        if cost > 0 {
            self.events.push(Event::Swing { attacker: id, victim: Some(id), damage: cost, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
            self.hurt_player_by(id, cost, id, source);
        }
    }

    /// Their attacks cost no stamina: Blood Fury, or the Warrior's own Rallying Cry.
    pub(crate) fn tireless(&self, id: Id) -> bool {
        self.has_cc(id, "fury")
    }

    /// How much more a hero's attacks deal under `fury` (0 without it).
    fn fury(&self, id: Id) -> f32 {
        let now = self.clock;
        self.cc_of(id).iter().filter(|c| c.kind == "fury" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max)
    }

    /// A hero's attack's stamina (a blow, a weapon art, a shove, an ability's): nothing in a fury.
    pub(crate) fn spend_attack(&mut self, id: Id, amount: f32) {
        let amount = if self.tireless(id) { 0.0 } else { amount };
        self.spend(id, amount);
    }

    // --- Momentum -----------------------------------------------------------------------------------

    /// Whether the hero has the Juggernaut's Momentum: one of the Juggernaut's cards (`passive =
    /// "momentum"`) on one of their keys. Keyed off the keys, not the class, because any hero can
    /// take any class's cards: it comes with the berserker's tree, and the more of it a hero plays
    /// the more of their keys feed it (Bladestorm's spin, the hits Berserker Rage gets them back
    /// up for). A Warrior all on the Warlord's side has none.
    pub fn momentum_on(&self, id: Id) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        p.loadout.values().filter_map(|k| self.content.ability(k)).any(|a| self.content.abilities[a].passive == "momentum")
    }

    /// The hero's Momentum stacks now (gone 4 sec after their last hit: the effect runs out).
    pub fn momentum(&self, id: Id) -> u32 {
        let now = self.clock;
        self.cc_of(id).iter().find(|c| c.kind == "momentum" && c.until > now).map_or(0, |c| c.amount.round() as u32)
    }

    /// One of the hero's own blows landed: a stack more (up to the most), and 4 sec more before
    /// they're lost.
    pub(crate) fn momentum_hit(&mut self, id: Id) {
        if !self.momentum_on(id) {
            return;
        }
        let m = self.content.momentum.clone();
        let stacks = (self.momentum(id) + 1).min(m.most);
        // One effect of each kind, the larger amount standing: so it's ended first, then given
        // again at its new count.
        self.cleanse(id, &["momentum"]);
        self.apply_cc(id, id, "momentum", m.secs, stacks as f32, 0.0, "Momentum");
    }

    /// The guard goes up: every stack is lost.
    pub(crate) fn momentum_lost(&mut self, id: Id) {
        if self.has_cc(id, "momentum") {
            self.cleanse(id, &["momentum"]);
        }
    }

    /// How much harder a hero's own blows land: Momentum's stacks and a fury.
    pub(crate) fn blow_boost(&self, id: Id) -> f32 {
        let stacks = self.momentum(id) as f32 * self.content.momentum.damage;
        (1.0 + stacks) * (1.0 + self.fury(id))
    }

    /// A hero's own blow's share of its damage: [`Self::blow_boost`], and less if Cowed.
    pub(crate) fn blow_share(&self, id: Id) -> f32 {
        self.blow_boost(id) * (1.0 - self.cowed_cut(id))
    }

    // --- Cowed, Exposed -----------------------------------------------------------------------------

    /// How much softer a Cowed unit's blows land (Intimidating Roar's 25%; a boss's only 10%).
    pub(crate) fn cowed_cut(&self, id: Id) -> f32 {
        let now = self.clock;
        let cut = self.cc_of(id).iter().filter(|c| c.kind == "cowed" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max);
        if cut > 0.0 && self.is_boss(id) {
            return cut.min(COWED_BOSS_CUT);
        }
        cut
    }

    /// How much slower a Cowed unit winds its blows up (a boss's not at all).
    pub(crate) fn cowed_slow(&self, id: Id) -> f32 {
        if self.has_cc(id, "cowed") && !self.is_boss(id) {
            COWED_SLOW
        } else {
            0.0
        }
    }

    /// How much longer a hero's blow staggers `victim`: an Exposed one 30% longer, and a heavy
    /// attack by a hero near their War Banner its `stagger` longer.
    pub(crate) fn stagger_share(&self, attacker: Id, victim: Id, heavy: bool) -> f32 {
        let banner = if heavy { self.under_banner(attacker).map_or(0.0, |b| b.stagger) } else { 0.0 };
        let exposed = if self.has_cc(victim, "exposed") { EXPOSED_STAGGER } else { 0.0 };
        (1.0 + banner) * (1.0 + exposed)
    }

    /// The Warrior's effects that are more than a timer: Exposed's half time on a boss, Rallying
    /// Cry's `rallied` getting them up first.
    pub(crate) fn warrior_effect(&mut self, owner: Id, victim: Id, e: &AbilityEffect, secs: f32, source: &str) {
        let secs = if e.kind == "exposed" && self.is_boss(victim) { secs * BOSS_EXPOSED_SHARE } else { secs };
        if e.kind == "rallied" {
            self.break_free(victim, &["stun"]);
        }
        self.apply_cc_look(victim, owner, &e.kind, secs, e.amount, 0.0, source, e.aura);
    }

    // --- the War Banner -----------------------------------------------------------------------------

    /// War Banner, or its recast: a banner planted at the reticle. With one of theirs standing
    /// (the recast), it's uprooted and planted there instead, its health and time left with it.
    pub(crate) fn plant_banner(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let at = self.aim_point(id, AbilityDef::at(d.range, share));
        let facing = self.players.get(&id).map_or(0.0, |p| p.facing);
        let (planted, health, until) = match self.warlord.banners.remove(&id) {
            Some(b) => {
                if self.objs.remove(&b.obj).is_some() {
                    self.events.push(Event::ObjGone { id: b.obj });
                }
                (b.ability, b.health, b.until)
            }
            None => {
                let Some(rules) = d.banner.as_ref() else {
                    self.events.push(Event::Flash { player: id, text: format!("No {} to move", d.name) });
                    return;
                };
                (ability, rules.health, now + d.secs)
            }
        };
        let pd = self.content.abilities[planted].clone();
        let Some(rules) = pd.banner.as_ref() else { return };
        let obj = self.alloc();
        self.objs.insert(obj, Obj { kind: ObjKind::Prop { display: pd.object, scale: rules.scale }, pos: at, facing });
        self.events.push(Event::ObjSpawned { id: obj });
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        }
        self.warlord.banners.insert(id, Banner { ability: planted, obj, at, health, until });
        let line = format!("{} plants a {} ({health:.0} health)", self.unit_name(id), pd.name);
        self.cc_note(id, line);
    }

    /// A hero's War Banner comes down (felled, or its time up): its look gone, and its recast with
    /// it (there's nothing left to move).
    fn banner_falls(&mut self, owner: Id) {
        let Some(b) = self.warlord.banners.remove(&owner) else { return };
        if self.objs.remove(&b.obj).is_some() {
            self.events.push(Event::ObjGone { id: b.obj });
        }
        let base = self.content.base_of(b.ability);
        if let Some(p) = self.players.get_mut(&owner) {
            if p.combos.recasts.remove(&base).is_some() {
                self.events.push(Event::Abilities { player: owner });
            }
        }
    }

    /// The War Banner hero `id` stands near, of a hero who can help them (their own too): whose,
    /// and its rules.
    fn banner_near(&self, id: Id) -> Option<(Id, &BannerDef)> {
        if self.warlord.banners.is_empty() {
            return None;
        }
        let pos = self.players.get(&id).filter(|p| p.health > 0)?.pos;
        self.warlord.banners.iter().find_map(|(owner, b)| {
            let d = &self.content.abilities[b.ability];
            let rules = d.banner.as_ref()?;
            (self.helpable(*owner, id) && dist2d(pos, b.at) <= d.radius[1] + HERO_SIZE && (pos[2] - b.at[2]).abs() < 6.0).then_some((*owner, rules))
        })
    }

    /// A War Banner's rules if `id` stands near one.
    pub(crate) fn under_banner(&self, id: Id) -> Option<&BannerDef> {
        self.banner_near(id).map(|(_, r)| r)
    }

    /// How many times as fast a hero's stamina comes back (a War Banner near them: twice).
    pub(crate) fn banner_regen(&self, id: Id) -> f32 {
        self.under_banner(id).map_or(1.0, |b| b.regen.max(1.0))
    }

    /// A blow from `pos` along `f` (its reach and arc) lands: every War Banner in it takes
    /// `damage`, whoever struck it (a foe's blow, a hero's own, friend or not), and falls at 0.
    pub(crate) fn banners_struck(&mut self, pos: Vec3, f: f32, range: f32, arc: f32, damage: f32) {
        if self.warlord.banners.is_empty() || damage <= 0.0 {
            return;
        }
        let hit: Vec<Id> = self
            .warlord
            .banners
            .iter()
            .filter(|(_, b)| {
                let d = dist2d(pos, b.at);
                d <= range + BANNER_SIZE && (pos[2] - b.at[2]).abs() < 4.0 && (d < 0.8 || angle_between(f, facing(pos, b.at)) <= arc / 2.0)
            })
            .map(|(o, _)| *o)
            .collect();
        for owner in hit {
            let Some(b) = self.warlord.banners.get_mut(&owner) else { continue };
            b.health -= damage;
            let left = b.health;
            let line = format!("War Banner takes {damage:.0} ({:.0} left)", left.max(0.0));
            self.cc_note(owner, line);
            if left <= 0.0 {
                self.banner_falls(owner);
            }
        }
    }

    /// A hero's swing's damage on a War Banner, as it's rolled for a creature (only rolled with one
    /// standing, so a fight without one rolls as it always did).
    pub(crate) fn swing_on_banners(&mut self, id: Id, w: &crate::content::WeaponDef, share: f32, f: f32, range: f32, arc: f32) {
        if self.warlord.banners.is_empty() {
            return;
        }
        let Some(pos) = self.players.get(&id).map(|p| p.pos) else { return };
        let damage = self.roll(w.damage) as f32 * share * self.blow_share(id);
        self.banners_struck(pos, f, range, arc, damage);
    }

    // --- every tick ---------------------------------------------------------------------------------

    /// Every tick: War Banners come down when their time is up (or their hero is gone) and mark
    /// the heroes near them; a Cowed creature low on health flees.
    pub(crate) fn step_warlord(&mut self) {
        let now = self.clock;
        if !self.warlord.banners.is_empty() {
            let done: Vec<Id> = self.warlord.banners.iter().filter(|(o, b)| b.until <= now || !self.players.contains_key(o)).map(|(o, _)| *o).collect();
            for o in done {
                self.banner_falls(o);
            }
            let near: Vec<(Id, Id, u32)> = self.players.keys().filter_map(|h| self.banner_near(*h).map(|(o, r)| (*h, o, r.aura))).collect();
            for (h, owner, aura) in near {
                self.apply_cc_look(h, owner, "war_banner", BANNER_HOLD, 1.0, 0.0, "War Banner", aura);
            }
        }
        // Cowed and near death: it runs (once a Cow), for what's left of it.
        self.warlord.fled.retain(|_, until| *until > now);
        let flee: Vec<(Id, Id, f32)> = self
            .mobs
            .iter()
            .filter(|(mid, m)| m.health > 0 && (m.health as f32) < m.max_health as f32 * COWED_FLEE_BELOW && !self.content.mobs[m.def].boss && !self.warlord.fled.contains_key(mid))
            .filter_map(|(mid, _)| self.cc_of(*mid).iter().find(|c| c.kind == "cowed" && c.until > now).map(|c| (*mid, c.from, c.until)))
            .collect();
        for (mid, from, until) in flee {
            self.warlord.fled.insert(mid, until);
            self.apply_cc(mid, from, "panic", until - now, 0.0, 0.0, "Intimidating Roar");
        }
    }
}
