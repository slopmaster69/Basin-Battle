//! Bodies moved and ground set: knockbacks and pulls, leaps and disengages, an anchor to step back
//! to (Demonic Circle), traps, a cone's, a nova's or a strip's wind-up (drawn on the ground for
//! everyone, so it can be dodged), strips laid along the ground, a veil (Vanish), and a grounding
//! totem that swallows a projectile.

use std::f32::consts::PI;

use super::abilities::{Projectile, Shape, HERO_SIZE};
use super::*;
use crate::content::{AbilityDef, AbilityEffect};

/// Gravity in the client's own units (yards a second, a second).
const GRAVITY: f32 = 19.29;
/// A knockback's and a pull's arc (their upward speed) where the effect names none.
const KNOCK_LIFT: f32 = 4.5;
const PULL_LIFT: f32 = 3.0;
/// A pull stops this far short of where it pulls to.
const PULL_STOP: f32 = 1.6;
/// A leap's arc where the ability names none.
const LEAP_LIFT: f32 = 6.0;
/// One step along the ground for a slide's end.
const STEP: f32 = 0.4;
/// The traps of one kind a hero can have set at once.
const MOST_TRAPS: usize = 2;
/// A wind-up's beat after it goes off.
const WINDUP_RECOVERY: f32 = 0.3;

/// A leap in the air: it lands at `at` then, striking what's there.
#[derive(Debug, Clone)]
pub(crate) struct Landing {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub share: f32,
    pub land_at: f32,
    pub mark: Option<Id>,
}

/// A trap set down: armed from `armed_at`, gone at `until`.
#[derive(Debug, Clone)]
pub(crate) struct Trap {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub share: f32,
    pub armed_at: f32,
    pub until: f32,
    pub mark: Id,
    /// The trap itself, on the ground.
    pub obj: Option<Id>,
}

/// A cone, a nova or a strip about to go off: from the caster (`fixed`: from `at` whatever the
/// caster does), along `yaw`, at `go_at`.
#[derive(Debug, Clone)]
pub(crate) struct Windup {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub yaw: f32,
    pub share: f32,
    pub go_at: f32,
    pub mark: Option<Id>,
    pub fixed: bool,
}

/// A hero's anchor (Demonic Circle): where its blink steps back to, until when.
#[derive(Debug, Clone)]
pub(crate) struct Anchor {
    pub ability: usize,
    pub at: Vec3,
    pub until: f32,
    pub mark: Id,
}

/// How long a body thrown up at `lift` stays in the air.
pub(crate) fn airtime(lift: f32) -> f32 {
    2.0 * lift.max(0.5) / GRAVITY
}

impl Survival {
    // --- knockbacks and pulls ------------------------------------------------------------------------

    /// How far along the ground from `from` toward `dir` a body slides before a wall, a drop or a
    /// climb stops it, up to `dist`.
    pub(crate) fn slide_end(&self, from: Vec3, dir: f32, dist: f32) -> Vec3 {
        let (c, s) = (dir.cos(), dir.sin());
        let mut at = from;
        let mut d = 0.0f32;
        while d < dist {
            let next = (d + STEP).min(dist);
            let p = [from[0] + c * next, from[1] + s * next];
            let g = self.land.height_near(p[0], p[1], at[2] + 1.0);
            if self.land.blocked_near(p[0], p[1], g) || (g - at[2]).abs() > 1.5 {
                break;
            }
            at = [p[0], p[1], g];
            d = next;
        }
        at
    }

    /// A knockback or a pull on someone: thrown through the air (a hero's own client flies it),
    /// whatever they had in hand lost. Nothing moves a body in Ice Block, Onslaught or Sanctuary.
    pub(crate) fn displace(&mut self, owner: Id, victim: Id, e: &AbilityEffect) {
        let now = self.clock;
        let Some(v) = self.unit_pos(victim) else { return };
        if !self.alive(victim) || self.immune(victim) || self.unstaggerable(victim) {
            return;
        }
        // A boss, a totem, a creature winding up a blow no guard stops: not moved (an ultimate's
        // aside).
        if !e.vs_bosses && self.holds_ground(victim) {
            self.shrugged_off(owner, victim);
            return;
        }
        if self.mobs.get(&victim).is_some_and(|m| self.content.mobs[m.def].inert()) {
            return;
        }
        let caster = self.unit_pos(owner);
        let origin = if e.from_caster { caster } else { self.fx_from.or(caster) }.unwrap_or(v);
        let pull = e.kind == "pull";
        let d = dist2d(v, origin);
        let (dir, dist) = if pull {
            (facing(v, origin), (d - PULL_STOP).clamp(0.0, e.amount))
        } else {
            let dir = if d < 0.3 { self.players.get(&owner).map_or(0.0, |p| p.facing) } else { facing(origin, v) };
            (dir, e.amount)
        };
        if dist < 0.3 {
            return;
        }
        let to = self.slide_end(v, dir, dist);
        let moved = dist2d(v, to);
        // Thrown into a wall (or up against a steep step): stunned as it hits.
        if !pull && e.wall_stun > 0.0 && moved + 0.3 < dist && self.walled(to, dir) {
            let air = if moved < 0.3 { 0.0 } else { airtime(if e.lift > 0.0 { e.lift } else { KNOCK_LIFT }) };
            self.wall_slam(owner, victim, e.wall_stun, air);
        }
        if moved < 0.3 {
            return;
        }
        let lift = if e.lift > 0.0 {
            e.lift
        } else if pull {
            PULL_LIFT
        } else {
            KNOCK_LIFT
        };
        let air = airtime(lift);
        let line = format!("{} on {} ({moved:.1} yd) from {}", e.kind, self.unit_name(victim), self.unit_name(owner));
        self.cc_note(victim, line.clone());
        self.cc_note(owner, line);
        if self.players.contains_key(&victim) {
            self.interrupt_hero(victim);
            self.set_action(victim, Action::Stagger { until: now + air });
            self.events.push(Event::Hop { player: victim, dir, speed: moved / air, lift, roll: false, style: "" });
        } else {
            self.stagger_mob(victim, air + 0.15, false);
            self.throw_mob(victim, to, air, Some(owner));
        }
    }

    // --- a veil (Vanish) -----------------------------------------------------------------------------

    /// Veiled: every lock on them lets go, and the creatures after them lose them. Not a boss: a
    /// champion in its fight keeps it (a veil never resets one, nor empties its fight).
    pub(crate) fn veiled(&mut self, id: Id) {
        let now = self.clock;
        let lockers: Vec<Id> = self.players.iter().filter(|(h, p)| **h != id && p.lock == Some(id)).map(|(h, _)| *h).collect();
        for h in lockers {
            self.players.get_mut(&h).unwrap().lock = None;
            self.events.push(Event::Lock { player: h, target: None });
        }
        let hunters: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0 && m.state.foe() == Some(mobs::Foe::Hero(id)) && !self.content.mobs[m.def].boss)
            .map(|(m, _)| *m)
            .collect();
        for m in hunters {
            self.halt_mob(m);
            if let Some(mob) = self.mobs.get_mut(&m) {
                mob.state = mobs::MobState::Idle { wander_at: now + 3.0 };
                mob.pattern = None;
            }
        }
    }

    /// A hero who strikes or casts comes out of a veil; their next blow is from behind.
    pub(crate) fn end_veil(&mut self, id: Id) {
        if self.has_cc(id, "veil") {
            self.cleanse(id, &["veil"]);
            if let Some(p) = self.players.get_mut(&id) {
                p.veil_strike_until = self.clock + 2.0;
            }
        }
    }

    /// Hidden from creatures (veiled).
    pub(crate) fn unseen(&self, id: Id) -> bool {
        self.has_cc(id, "veil")
    }

    /// Hidden from this creature: veiled, and it's no boss (a champion sees through a veil).
    pub(crate) fn unseen_by(&self, mob: Id, hero: Id) -> bool {
        self.unseen(hero) && !self.mobs.get(&mob).is_some_and(|m| self.content.mobs[m.def].boss)
    }

    // --- leaps ----------------------------------------------------------------------------------------

    /// A leap: to the reticle's point (or the range back from the facing), through the air; where
    /// it lands is drawn for everyone, and it strikes what's there as it does.
    pub(crate) fn start_leap(&mut self, id: Id, ability: usize, share: f32) {
        self.leap_to(id, ability, share, None);
    }

    /// [`Self::start_leap`], to a spot of its own when `to` names one (a recast's leap back to
    /// where the first cast began).
    pub(crate) fn leap_to(&mut self, id: Id, ability: usize, share: f32, to: Option<Vec3>) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let pos = self.players[&id].pos;
        let f = self.players[&id].facing;
        let target = if let Some(at) = to {
            at
        } else if d.back {
            self.slide_end(pos, f + PI, range)
        } else {
            let at = self.aim_point(id, range);
            if self.land.blocked_near(at[0], at[1], at[2]) || (at[2] - pos[2]).abs() > 8.0 {
                self.slide_end(pos, facing(pos, at), dist2d(pos, at))
            } else {
                at
            }
        };
        let dist = dist2d(pos, target);
        let dir = if dist > 0.1 { facing(pos, target) } else { f };
        let lift = if d.lift > 0.0 { d.lift } else { LEAP_LIFT };
        let air = airtime(lift);
        if d.cleanse {
            self.cleanse(id, &["root", "snare"]);
        }
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        self.events.push(Event::Hop { player: id, dir, speed: dist / air, lift, roll: false, style: "" });
        self.set_action(id, Action::Dash { t: 0.0, secs: air, ability, target: None, share });
        let radius = AbilityDef::at(d.radius, share);
        let mark = (radius > 0.0).then(|| self.mark_as(target, radius, air + 0.05, Some(Telegraph { own_hidden: true, ..Telegraph::hero_shape(id, radius, 360.0, 0.0, air, Some((target, dir))) })));
        // A rival where it comes down hears it coming too (the warning over them), not only the
        // mark on the ground, which they may have turned off or not be looking at.
        if radius > 0.0 {
            let near: Vec<Id> = self.players.iter().filter(|(h, p)| **h != id && p.health > 0 && dist2d(p.pos, target) <= radius + 3.0 && self.hostile(id, **h)).map(|(h, _)| *h).collect();
            for h in near {
                self.events.push(Event::Danger { player: h, secs: air, from: Some(id) });
            }
        }
        // What it leaves where it began (a Disengage's trap).
        if let Some(l) = self.content.ability(&d.leaves) {
            self.leave_at(id, l, pos, f);
        }
        self.landings.push(Landing { owner: id, ability, at: target, share, land_at: now + air, mark });
    }

    /// Something an ability leaves where it went from: a trap set, a nova, ground.
    fn leave_at(&mut self, id: Id, ability: usize, at: Vec3, yaw: f32) {
        match self.content.abilities[ability].kind.as_str() {
            "trap" => self.set_trap(id, ability, at, 1.0),
            "ground" => self.ground_at(id, ability, at, 1.0),
            "line" => self.line_go(id, ability, at, yaw, 1.0),
            _ => self.nova(id, ability, at, 1.0),
        }
    }

    // --- strips along the ground -----------------------------------------------------------------

    /// A strip along the reticle from the caster: at once, after a wind-up drawn for everyone
    /// (`cast`), or laid down to go off after `delay` whatever the caster does.
    pub(crate) fn start_line(&mut self, id: Id, ability: usize, yaw: f32, share: f32) -> bool {
        let d = self.content.abilities[ability].clone();
        let pos = self.players[&id].pos;
        if d.cast > 0.0 {
            self.start_windup(id, ability, yaw, share);
            return true;
        }
        if d.delay > 0.0 {
            let now = self.clock;
            let len = AbilityDef::at(d.range, share);
            let half = AbilityDef::at(d.radius, share);
            let tele = Telegraph::hero_shape(id, len, 0.0, half * 2.0, d.delay, Some((pos, yaw)));
            let mark = self.mark_as(pos, half, d.delay + 0.05, Some(tele));
            self.events.push(Event::Cast { caster: id, spell: d.spell, ms: (d.delay * 1000.0) as u32 });
            self.windups.push(Windup { owner: id, ability, at: pos, yaw, share, go_at: now + d.delay, mark: Some(mark), fixed: true });
            return false;
        }
        self.line_go(id, ability, pos, yaw, share);
        false
    }

    /// A strip strikes everyone in it.
    pub(crate) fn line_go(&mut self, id: Id, ability: usize, at: Vec3, yaw: f32, share: f32) {
        let d = self.content.abilities[ability].clone();
        let len = AbilityDef::at(d.range, share);
        let half = AbilityDef::at(d.radius, share);
        let end = self.slide_end(at, yaw, len);
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(end) });
        let damage = AbilityDef::at(d.damage, share);
        self.area_shape(id, ability, at, len, share, damage, Shape::Line(yaw, len, half));
        if self.debug {
            for k in 0..=4 {
                let t = len * k as f32 / 4.0;
                self.mark([at[0] + yaw.cos() * t, at[1] + yaw.sin() * t, at[2]], half, 0.8);
            }
        }
    }

    // --- wind-ups -------------------------------------------------------------------------------------

    /// A cone, a nova or a strip winds up: its shape on the ground around the caster for everyone
    /// to see, and the caster committed till it goes.
    pub(crate) fn start_windup(&mut self, id: Id, ability: usize, yaw: f32, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let pos = self.players[&id].pos;
        let tele = match d.kind.as_str() {
            "cone" => Telegraph::hero_shape(id, AbilityDef::at(d.range, share), d.arc.max(10.0), 0.0, d.cast, None),
            "line" => Telegraph::hero_shape(id, AbilityDef::at(d.range, share), 0.0, AbilityDef::at(d.radius, share) * 2.0, d.cast, None),
            _ => Telegraph::hero_shape(id, AbilityDef::at(d.radius, share), 360.0, 0.0, d.cast, None),
        };
        let mark = self.mark_as(pos, tele.reach.min(4.0), d.cast + 0.05, Some(tele));
        if d.charge_spell != 0 {
            self.events.push(Event::Cast { caster: id, spell: d.charge_spell, ms: (d.cast * 1000.0) as u32 });
        }
        self.set_action(id, Action::Cast { t: 0.0, total: d.cast + WINDUP_RECOVERY, ability });
        self.windups.push(Windup { owner: id, ability, at: pos, yaw, share, go_at: now + d.cast, mark: Some(mark), fixed: false });
    }

    // --- traps ----------------------------------------------------------------------------------------

    /// A trap set at the reticle (or the feet): the oldest of its kind goes if there are too many.
    pub(crate) fn place_trap(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let at = if d.at_self { self.players[&id].pos } else { self.aim_point(id, AbilityDef::at(d.range, share)) };
        self.set_trap(id, ability, at, share);
    }

    fn set_trap(&mut self, id: Id, ability: usize, at: Vec3, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let mine: Vec<usize> = self.traps.iter().enumerate().filter(|(_, t)| t.owner == id && t.ability == ability).map(|(k, _)| k).collect();
        if mine.len() >= MOST_TRAPS {
            let t = self.traps.remove(mine[0]);
            self.clear_trap(&t);
        }
        let radius = AbilityDef::at(d.radius, share);
        let life = d.secs.max(1.0);
        let mark = self.mark_kind(at, radius, life, true);
        // The trap on the ground where it's set (no spell's look there: Explosive Trap's own
        // shows the client's placeholder at a point).
        let obj = (d.object != 0).then(|| {
            let o = self.alloc();
            let facing = self.players.get(&id).map_or(0.0, |p| p.facing);
            self.objs.insert(o, Obj { kind: ObjKind::Prop { display: d.object, scale: 1.0 }, pos: at, facing });
            self.events.push(Event::ObjSpawned { id: o });
            o
        });
        self.traps.push(Trap { owner: id, ability, at, share, armed_at: now + d.delay, until: now + life, mark, obj });
    }

    /// A trap gone (sprung, faded, or the oldest of too many): its mark and the trap itself.
    fn clear_trap(&mut self, t: &Trap) {
        self.unmark(t.mark);
        if let Some(o) = t.obj {
            if self.objs.remove(&o).is_some() {
                self.events.push(Event::ObjGone { id: o });
            }
        }
    }

    /// The first foe within an armed trap's reach.
    fn trap_victim(&self, t: &Trap) -> Option<Id> {
        let radius = AbilityDef::at(self.content.abilities[t.ability].radius, t.share);
        let mob = self.mobs.iter().find(|(mid, m)| {
            m.health > 0 && !m.hidden && !self.content.mobs[m.def].inert() && dist2d(m.pos, t.at) <= radius + 0.5 * self.content.mobs[m.def].scale && (m.pos[2] - t.at[2]).abs() < 3.0 && !self.totems.iter().any(|x| x.mob == **mid)
        });
        if let Some((mid, _)) = mob {
            return Some(*mid);
        }
        if self.content.pvp.friendly_fire {
            return self.players.iter().find(|(h, p)| self.hostile(t.owner, **h) && p.health > 0 && dist2d(p.pos, t.at) <= radius + HERO_SIZE && (p.pos[2] - t.at[2]).abs() < 3.0).map(|(h, _)| *h);
        }
        None
    }

    /// A trap goes off: its burst (the `splash`, else its reach) strikes everyone in it, and it may
    /// leave ground burning.
    fn spring_trap(&mut self, t: Trap) {
        let d = self.content.abilities[t.ability].clone();
        self.clear_trap(&t);
        let radius = AbilityDef::at(d.radius, t.share);
        let burst = if d.splash > 0.0 { d.splash } else { radius };
        if d.burst_spell != 0 {
            self.events.push(Event::Missile { caster: t.owner, spell: d.burst_spell, target: Target::Point(t.at) });
        }
        let damage = AbilityDef::at(d.damage, t.share);
        self.area(t.owner, t.ability, t.at, burst, t.share, damage, None);
        self.linger_at(t.owner, t.ability, t.at, burst);
    }

    // --- anchors (Demonic Circle) -----------------------------------------------------------------

    /// The anchor a hero's blink would step back to, if it's still there.
    pub(crate) fn anchor_of(&self, id: Id, base: usize) -> Option<&Anchor> {
        self.anchors.get(&id).filter(|a| a.ability == base && a.until > self.clock)
    }

    /// Set an anchor at the feet (it costs nothing; the step back to it spends the cooldown).
    pub(crate) fn set_anchor(&mut self, id: Id, base: usize) {
        let now = self.clock;
        let d = self.content.abilities[base].clone();
        // On the ground under them (not under the ground: see `standing_spot`).
        let pos = self.standing_spot(self.players[&id].pos);
        if let Some(old) = self.anchors.remove(&id) {
            self.unmark(old.mark);
        }
        let mark = self.mark_kind(pos, 1.5, d.anchor, true);
        self.events.push(Event::Missile { caster: id, spell: d.charge_spell.max(d.spell), target: Target::Point(pos) });
        self.events.push(Event::Flash { player: id, text: format!("{}: press again to return", d.name) });
        self.anchors.insert(id, Anchor { ability: base, at: pos, until: now + d.anchor, mark });
        self.events.push(Event::Abilities { player: id });
    }

    /// Step back to the anchor (its blink); false with none.
    pub(crate) fn return_to_anchor(&mut self, id: Id, ability: usize) -> bool {
        let base = self.content.base_of(ability);
        let Some(a) = self.anchor_of(id, base).cloned() else { return false };
        self.anchors.remove(&id);
        self.unmark(a.mark);
        self.step_back_to(id, ability, a.at);
        true
    }

    /// A blink back to a spot it left (an anchor; where the cast a recast follows began): its
    /// look, out of roots and snares with `cleanse`, and what it `leaves` where it went from.
    pub(crate) fn step_back_to(&mut self, id: Id, ability: usize, at: Vec3) {
        let d = self.content.abilities[ability].clone();
        let start = self.players[&id].pos;
        let f = self.players[&id].facing;
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        self.set_down(id, at, f);
        if d.cleanse {
            self.cleanse(id, &["root", "snare"]);
        }
        if let Some(l) = self.content.ability(&d.leaves) {
            self.leave_at(id, l, start, f);
        }
    }

    // --- a grounding totem ----------------------------------------------------------------------------

    /// Whether a projectile of `owner`'s flying at `pos` is swallowed by a foe's grounding totem
    /// there (the totem falls doing it).
    pub(crate) fn grounded(&mut self, p: &Projectile) -> bool {
        let hit = self.totems.iter().find(|t| {
            t.owner != p.owner
                && self.content.abilities[t.ability].grounding
                && self.mobs.get(&t.mob).is_some_and(|m| dist2d(m.pos, p.pos) <= self.content.abilities[t.ability].totem.as_ref().map_or(8.0, |x| x.radius) && (p.pos[2] - m.pos[2]).abs() < 6.0)
                && (!self.players.contains_key(&p.owner) || self.hostile(t.owner, p.owner))
        });
        let Some(t) = hit.cloned() else { return false };
        let at = self.mobs[&t.mob].pos;
        self.events.push(Event::Missile { caster: p.owner, spell: p.spell, target: Target::Point([at[0], at[1], at[2] + 1.0]) });
        self.events.push(Event::Flash { player: t.owner, text: "Grounded".into() });
        self.remove_totem(t.mob);
        true
    }

    /// Whether a creature's missile landing at `at` is swallowed by a hero's grounding totem.
    pub(crate) fn grounded_at(&mut self, at: Vec3) -> bool {
        let hit = self.totems.iter().find(|t| {
            self.content.abilities[t.ability].grounding && self.mobs.get(&t.mob).is_some_and(|m| dist2d(m.pos, at) <= self.content.abilities[t.ability].totem.as_ref().map_or(8.0, |x| x.radius))
        });
        let Some(t) = hit.cloned() else { return false };
        self.events.push(Event::Flash { player: t.owner, text: "Grounded".into() });
        self.remove_totem(t.mob);
        true
    }

    // --- every tick -----------------------------------------------------------------------------------

    /// Leaps land, wind-ups and strips go off, traps spring or fade, anchors fade.
    pub(crate) fn step_moves(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<Landing>, Vec<Landing>) = std::mem::take(&mut self.landings).into_iter().partition(|l| l.land_at <= now);
        self.landings = keep;
        for l in due {
            if let Some(m) = l.mark {
                self.unmark(m);
            }
            if !self.players.get(&l.owner).is_some_and(|p| p.health > 0) {
                continue;
            }
            if let Some(p) = self.players.get_mut(&l.owner) {
                p.pos = l.at;
            }
            let d = self.content.abilities[l.ability].clone();
            let radius = AbilityDef::at(d.radius, l.share);
            if d.impact_kit != 0 {
                self.events.push(Event::Visual { id: l.owner, kit: d.impact_kit, by: None });
            }
            if radius > 0.0 {
                let damage = AbilityDef::at(d.damage, l.share);
                self.area(l.owner, l.ability, l.at, radius, l.share, damage, None);
            }
        }
        let (due, keep): (Vec<Windup>, Vec<Windup>) = std::mem::take(&mut self.windups).into_iter().partition(|w| w.go_at <= now);
        self.windups = keep;
        for w in due {
            if let Some(m) = w.mark {
                self.unmark(m);
            }
            // Cut short (a stun, a stagger, a roll): lost.
            let still = self.players.get(&w.owner).is_some_and(|p| p.health > 0 && (w.fixed || matches!(p.action, Action::Cast { ability, .. } if ability == w.ability)));
            // Its gathering look ends as it goes or is lost (a strip's own missile ends its look:
            // the same spell).
            let d = self.content.abilities[w.ability].clone();
            let look = if w.fixed { d.spell } else { d.charge_spell };
            if look != 0 && (!w.fixed || !still) {
                self.events.push(Event::CastDone { caster: w.owner, spell: look });
            }
            if !still {
                continue;
            }
            let at = if w.fixed { w.at } else { self.players[&w.owner].pos };
            match d.kind.as_str() {
                "cone" => self.cone_at(w.owner, w.ability, at, w.yaw, w.share),
                "line" => self.line_go(w.owner, w.ability, at, w.yaw, w.share),
                _ => self.nova(w.owner, w.ability, at, w.share),
            }
        }
        // Traps: armed ones spring on the first foe near; spent or faded ones go.
        let traps = std::mem::take(&mut self.traps);
        let mut keep = Vec::new();
        for t in traps {
            if now >= t.until || !self.players.contains_key(&t.owner) {
                self.clear_trap(&t);
                continue;
            }
            // A jump pad: it launches the caster and friends on it, and stays.
            if self.content.abilities[t.ability].launch > 0.0 {
                if now >= t.armed_at {
                    self.pad_launches(&t);
                }
                keep.push(t);
                continue;
            }
            if now >= t.armed_at && self.trap_victim(&t).is_some() {
                self.spring_trap(t);
                continue;
            }
            keep.push(t);
        }
        keep.extend(std::mem::take(&mut self.traps));
        self.traps = keep;
        let gone: Vec<Id> = self.anchors.iter().filter(|(h, a)| a.until <= now || !self.players.contains_key(h)).map(|(h, _)| *h).collect();
        for h in gone {
            if let Some(a) = self.anchors.remove(&h) {
                self.unmark(a.mark);
                self.events.push(Event::Abilities { player: h });
            }
        }
    }
}
