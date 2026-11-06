//! The Priest's core (docs/priest-draft3.md): a weapon whose right click is an ability (the holy
//! staff's aimed Healing Gust, the shadow staff's Dark Reckoning, the Hollow Star's orb) and so
//! no guard at all; and Power Word: Shield given to a friend just as a blow lands on them,
//! parrying it for them.
//!
//! **The right click** (`alt` on the [[weapon]]): a `channel` is held while the button is and
//! ends as it's let go; one that aims (`aim_alt`) is aimed while the button is held, the click
//! casting it (the staff's heavy) and the let-go ending the aim; anything else goes on the press,
//! its `cooldown` the fire rate. With one in hand nothing guards: the right mouse never raises a
//! guard, a block or a ward.
//!
//! **The Shield's perfect** (`parry_for`): the shield lands on a friend (never the caster) at
//! most `[checks] window` before a blow lands on them, the caster's round trip allowed on the
//! early side (as far back as `combat.max_rewind`: they pressed on their own screen, which is
//! that far behind), or while the blow waits on the friend's round trip (`resolve`). The blow
//! is then parried as a parry of the friend's would be (the striker reels, a boss's posture
//! fills), the shield stays, and the caster gets a counter's Valor and the perfect, once a cast.
//! On themself it only absorbs.

use std::f32::consts::{FRAC_PI_2, PI};

use super::*;
use combat::PendingStrike;

/// A hero's share of the Priest's core: the last shield a friend gave them (for its perfect).
#[derive(Debug, Clone, Default)]
pub struct Priest {
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
        // Held, it aims (the holy staff's heal): the click casts it, the let-go ends the aim.
        if d.aim_alt {
            if !p.guard_held || !free {
                return true;
            }
            // Its charges spent: nothing to aim.
            if !self.ready(id, ability) {
                return true;
            }
            self.unsheathe(id);
            self.set_action(id, Action::Aim { ability });
            return true;
        }
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
        // The aim it held ends (a charge begun from it goes on: the click is still down).
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Aim { ability: a }) if *a == ability) {
            self.ability_cancel(id);
            return;
        }
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Channel { ability: a, .. }) if *a == ability) {
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
        }
    }

    // --- a bolt for friends (Healing Gust) --------------------------------------------------------------

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
            let k = self.perfect_kit_of(mark.ability);
            if k != 0 {
                self.events.push(Event::Visual { id: s.victim, kit: k, by: Some(mark.by) });
            }
            self.events.push(Event::Perfect { player: mark.by, ability: mark.ability });
        }
        true
    }

    // --- draft 3's hooks (docs/priest-draft3.md) ---------------------------------------------------

    /// A cast that found nothing to do (Penance with no one in front of them): its cooldown, and
    /// the Valor it took, given back.
    pub(crate) fn refund_cast(&mut self, id: Id, ability: usize, free: bool) {
        let base = self.content.base_of(ability);
        let d = self.content.abilities[ability].clone();
        let cost = if free { 0.0 } else { self.valor_cost(&d) };
        let max = self.content.valor.max;
        if let Some(p) = self.players.get_mut(&id) {
            p.cooldowns.remove(&base);
            p.valor = (p.valor + cost).min(max);
        }
        if cost > 0.0 {
            self.events.push(Event::Valor { player: id });
        }
        self.events.push(Event::Abilities { player: id });
    }

    /// Fade's sidestep: a dash the way the hero is moving (a step back if they aren't), their body
    /// swaying aside as a dodge's does, untouchable a moment, the blows it slips noted.
    pub(crate) fn sidestep(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let Some(p) = self.players.get(&id) else { return };
        let m = p.moving;
        let (mut fx, mut fy) = (0.0f32, 0.0f32);
        for (on, a) in [(m.forward, 0.0), (m.back, PI), (m.left, FRAC_PI_2), (m.right, -FRAC_PI_2)] {
            if on {
                fx += f32::cos(a);
                fy += f32::sin(a);
            }
        }
        let moving = fx.abs() + fy.abs() >= 0.01;
        let rel = if moving { fy.atan2(fx) } else { PI };
        let yaw = (p.facing + rel).rem_euclid(std::f32::consts::TAU);
        let from = p.pos;
        let style = if !moving || rel.abs() >= 2.2 {
            "back"
        } else if rel.abs() <= 1.0 {
            "roll"
        } else if rel > 0.0 {
            "left"
        } else {
            "right"
        };
        self.start_dash(id, ability, yaw, share);
        // A sidestep strikes nothing it passes (a dash `through` bodies would strike each).
        if let Some(p) = self.players.get_mut(&id) {
            p.combos.through.clear();
        }
        // The body sways aside as a dodge's does (the dash itself carries them).
        self.events.push(Event::Hop { player: id, dir: yaw, speed: 0.0, lift: 0.0, roll: true, style });
        if d.slip {
            self.note_slip(id, ability, from);
        }
        if d.invulnerable && d.secs > 0.0 {
            let until = self.clock + d.secs;
            if let Some(p) = self.players.get_mut(&id) {
                p.untouchable_until = p.untouchable_until.max(until);
            }
        }
    }

    /// The Priest's part of an ability's damage on a foe: the caster's Shadow Weaving on it makes
    /// their staff's bolts stronger; a shadowed foe (Weaving, or a shadow burning on it) takes a
    /// Mind Blast's `shadowed` more; a Mind Blast's heart strikes harder, true on a foe on the
    /// move; and any other Priest spell
    /// on a foe the caster's Smite marked tears the mark out, harder, Healing Gust sooner back.
    pub(crate) fn priest_damage(&mut self, owner: Id, victim: Id, ability: usize, dmg: f32) -> f32 {
        let d = self.content.abilities[ability].clone();
        let k = self.content.priest.clone();
        let mut dmg = dmg;
        let weave = self.staff_marks.iter().find(|m| m.owner == owner && m.on == victim && m.kind == "weave").map_or(0, |m| m.n);
        if weave > 0 && (d.builds == "weave" || d.woven) {
            dmg *= 1.0 + k.weave_step * weave as f32;
        }
        if d.shadowed > 0.0 && (weave > 0 || self.shadow_burning(victim)) {
            dmg *= 1.0 + d.shadowed;
        }
        // A Mind Blast's heart: the foe within it takes the heart's damage in place of its own;
        // struck true (the perfect) if it was on the move, when the blast asks that.
        if let Some((at, r, heart, base, moving)) = self.light.heart {
            if d.heart[0] > 0.0 && self.unit_pos(victim).is_some_and(|p| dist2d(p, at) <= r + 0.3) && base > 0.0 {
                if moving <= 0.0 || !self.on_the_move(victim) {
                    dmg *= heart / base;
                    self.light.heart_struck |= moving <= 0.0;
                } else {
                    dmg *= moving / base;
                    self.light.heart_struck = true;
                }
            }
        }
        let priest = self.content.abilities[self.content.base_of(ability)].school == "priest" || d.school == "priest";
        if priest && d.builds != "sear" {
            if let Some(i) = self.staff_marks.iter().position(|m| m.owner == owner && m.on == victim && m.kind == "sear") {
                self.staff_marks.remove(i);
                dmg += k.sear_damage;
                self.events.push(Event::Stacks { player: owner, kind: "sear", n: 0, on: Some(victim) });
                if k.sear_kit != 0 {
                    self.events.push(Event::Visual { id: victim, kit: k.sear_kit, by: Some(owner) });
                }
                if let Some(gust) = self.content.ability(&k.gust) {
                    let every = self.content.abilities[gust].cooldown.max(0.01);
                    self.hasten(owner, gust, k.sear_refund / every);
                }
            }
        }
        dmg
    }

    /// Whether this body is on the move: a hero holding a way to go, a creature walking a path.
    pub(crate) fn on_the_move(&self, id: Id) -> bool {
        if let Some(p) = self.players.get(&id) {
            let m = &p.moving;
            return m.forward || m.back || m.left || m.right;
        }
        self.mobs.get(&id).is_some_and(|m| !m.path.is_empty() && m.speed > 0.0)
    }

    /// Whether a shadow burns on this body (a Warlock's rot, a shadow DoT of anyone's).
    fn shadow_burning(&self, id: Id) -> bool {
        let now = self.clock;
        self.cc_of(id).iter().any(|c| {
            c.kind == "burn"
                && c.until > now
                && self.content.abilities.iter().any(|a| a.name == c.source && matches!(a.school.as_str(), "warlock" | "priest"))
        })
    }

    /// A Shadow Word: Death that killed: one free cast of it a moment, unless it was the free one.
    pub(crate) fn death_kill(&mut self, owner: Id, ability: usize) {
        let was_free = self.light.deaths.remove(&owner).unwrap_or(false);
        if was_free {
            return;
        }
        let now = self.clock;
        let base = self.content.base_of(ability);
        let secs = self.content.abilities[ability].kill_recast;
        if let Some(p) = self.players.get_mut(&owner) {
            p.combos.empowered.insert(base, now + secs);
            p.cooldowns.remove(&base);
        }
        self.events.push(Event::Abilities { player: owner });
    }

    /// Greater Smite let go in the air: the Priest thrown back, once a jump.
    pub(crate) fn air_recoil(&mut self, id: Id, ability: usize) {
        let yards = self.content.abilities[ability].air_recoil;
        if yards <= 0.0 {
            return;
        }
        let Some(air) = self.airborne(id) else { return };
        if self.light.recoiled.get(&id).is_some_and(|t| *t >= air.since) {
            return;
        }
        self.light.recoiled.insert(id, self.clock);
        let f = self.players[&id].facing;
        // Thrown back over about a third of a second, a little lift to carry it.
        self.events.push(Event::Hop { player: id, dir: (f + PI).rem_euclid(std::f32::consts::TAU), speed: yards / 0.35, lift: 3.0, roll: false, style: "" });
    }

    /// The shadow staff's patience: how long since the hero's last bolt of it, and whether the
    /// next is Empowered (its look on them, once, as it comes ready).
    pub(crate) fn empowered_now(&self, id: Id, ability: usize) -> bool {
        let d = &self.content.abilities[ability];
        d.builds == "weave" && self.light.last_bolt.get(&id).is_none_or(|t| self.clock - t >= self.content.priest.weave_after)
    }

    /// Each tick: a shadow staff's next bolt comes Empowered (its look on the Priest, once).
    pub(crate) fn step_empower(&mut self) {
        let now = self.clock;
        let after = self.content.priest.weave_after;
        let ready: Vec<Id> = self
            .players
            .iter()
            .filter(|(h, p)| p.health > 0 && self.bolt_of(p, false).is_some_and(|b| self.content.abilities[b].builds == "weave") && !self.light.empower_shown.contains(*h))
            .filter(|(h, _)| self.light.last_bolt.get(*h).is_none_or(|t| now - t >= after))
            .map(|(h, _)| *h)
            .collect();
        for h in ready {
            self.light.empower_shown.insert(h);
            self.events.push(Event::PriestLook { player: h, kind: "empower", n: 1, on: None, a: 0.0, b: 0.0 });
            self.events.push(Event::Auras { id: h });
        }
    }

    /// A shadow staff's bolt has gone: its patience starts over (the Empowered look off).
    pub(crate) fn bolt_fired(&mut self, id: Id) {
        self.light.last_bolt.insert(id, self.clock);
        if self.light.empower_shown.remove(&id) {
            self.events.push(Event::PriestLook { player: id, kind: "empower", n: 0, on: None, a: 0.0, b: 0.0 });
            self.events.push(Event::Auras { id });
        }
    }

    /// The looks a Priest wears for their own: an Empowered staff, Salvation's wings.
    pub(crate) fn priest_auras(&self, id: Id) -> Vec<u32> {
        let mut out = Vec::new();
        if self.light.empower_shown.contains(&id) && self.content.priest.empower_aura != 0 {
            out.push(self.content.priest.empower_aura);
        }
        if let Some((_, ability, ..)) = self.light.aloft.iter().find(|a| a.0 == id) {
            let a = self.content.abilities[*ability].float_aura;
            if a != 0 {
                out.push(a);
            }
        }
        out
    }
}
