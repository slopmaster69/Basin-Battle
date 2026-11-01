//! The beta page's mechanics, one rule each: a projectile that yanks its caster after it (a
//! grappling hook), one that swaps places with what it strikes, a knockback that stuns against a
//! wall, an echo to come back to (Rewind), a jump pad, a boon that cheats death once, a mirror
//! that turns projectiles back, a tether that drags a foe back once it strays, and bursts as a
//! shield breaks or lingering ground ends.

use std::collections::BTreeMap;

use super::abilities::{Burning, Projectile, Stage, HERO_SIZE};
use super::moves::{airtime, Landing, Trap};
use super::*;
use crate::content::{AbilityDef, AbilityEffect};

/// A yank runs this fast (yards a second), stopping this far short of a wall and this far from a
/// foe's body (beside it, not in it).
const YANK_SPEED: f32 = 40.0;
const YANK_WALL: f32 = 0.6;
const YANK_BESIDE: f32 = 0.3;
/// A cheated death mends what it does over this long.
const CHEAT_SECS: f32 = 3.0;
/// A pad launches a hero at most once in this long; its leap's arc where it names none.
const PAD_EVERY: f32 = 1.0;
const PAD_LIFT: f32 = 6.0;
/// A tether's link is drawn again this often (something else may have cleared it); its pull back
/// arcs this high.
const LINK_EVERY: f32 = 1.0;
const TETHER_LIFT: f32 = 3.0;
/// A creature's bolt turned back by a mirror flies this fast, leaving from this high.
const MIRROR_SPEED: f32 = 30.0;
const HAND_UP: f32 = 1.45;

/// What the beta mechanics keep between ticks.
#[derive(Debug, Clone, Default)]
pub(crate) struct Beta {
    /// Heroes being yanked: where they'll stand, and when they get there.
    pub yanks: BTreeMap<Id, (Vec3, f32)>,
    /// Effects due a moment later (a stun as a body hits the wall it was thrown at, or lands from
    /// a tether's pull).
    pub later: Vec<Later>,
    /// Each hero's echo to come back to.
    pub echoes: BTreeMap<Id, Echo>,
    /// Foes bound to their caster.
    pub tethers: Vec<Tether>,
    /// Shields that burst as they end: whose, and the burst (a nova).
    pub pops: Vec<(Id, usize)>,
    /// When each hero was last launched by a pad.
    pub launched: BTreeMap<Id, f32>,
}

/// Effects given at `at`, by `owner` to `victim` (`source` names them in the log).
#[derive(Debug, Clone)]
pub(crate) struct Later {
    pub at: f32,
    pub owner: Id,
    pub victim: Id,
    pub effects: Vec<AbilityEffect>,
    pub source: String,
}

/// An echo a hero left (`ability`, the card): where, with what health, and how they come back to
/// it (by `recast`, that ability's cast; with none, by themselves at `until`).
#[derive(Debug, Clone)]
pub(crate) struct Echo {
    pub ability: usize,
    pub recast: Option<usize>,
    pub at: Vec3,
    pub health: u32,
    pub until: f32,
    pub mark: Id,
}

/// A foe bound to its caster by `ability` until then (its link drawn again at `look_at`).
#[derive(Debug, Clone)]
pub(crate) struct Tether {
    pub owner: Id,
    pub foe: Id,
    pub ability: usize,
    pub until: f32,
    pub look_at: f32,
}

fn sub3(b: Vec3, a: Vec3) -> Vec3 {
    [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
}

fn len3(v: Vec3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

fn unit3(v: Vec3) -> Vec3 {
    let l = len3(v).max(1.0e-5);
    [v[0] / l, v[1] / l, v[2] / l]
}

impl Survival {
    /// Every tick: yanks arrive, what was due a moment later lands, echoes call their heroes back
    /// or fade, tethers hold or drag, shields that burst as they end burst.
    pub(crate) fn step_beta(&mut self) {
        let now = self.clock;
        // A yanked hero is where the dash took them (their client ran it), unless something held
        // them on the way.
        let done: Vec<(Id, Vec3)> = self.beta.yanks.iter().filter(|(_, (_, t))| *t <= now).map(|(h, (at, _))| (*h, *at)).collect();
        for (h, at) in done {
            self.beta.yanks.remove(&h);
            let held = self.held_still(h);
            if let Some(p) = self.players.get_mut(&h).filter(|p| p.health > 0 && !held) {
                p.pos = at;
            }
        }
        let (due, keep): (Vec<Later>, Vec<Later>) = std::mem::take(&mut self.beta.later).into_iter().partition(|l| l.at <= now);
        self.beta.later = keep;
        for l in due {
            if self.alive(l.victim) {
                self.apply_effects(l.owner, l.victim, &l.effects, 1.0, &l.source);
            }
        }
        self.step_echoes();
        self.step_tethers();
        let pops = std::mem::take(&mut self.beta.pops);
        for (h, nova) in pops {
            if self.has_cc(h, "shield") {
                self.beta.pops.push((h, nova));
                continue;
            }
            // Broken or run out: it bursts round them (not off a fallen hero).
            if let Some(at) = self.unit_pos(h).filter(|_| self.alive(h)) {
                self.nova(h, nova, at, 1.0);
            }
        }
        self.beta.launched.retain(|_, t| now - *t < PAD_EVERY);
    }

    // --- what a projectile does past its blow ----------------------------------------------------

    /// A projectile struck `unit` (none: a wall or the ground) at `at`: it yanks its caster there,
    /// swaps them with the one struck, or binds that one to them.
    pub(crate) fn beta_hit(&mut self, p: &Projectile, unit: Option<Id>, at: Vec3) {
        let d = &self.content.abilities[p.ability];
        let (yank, swap, tether) = (d.yank, d.swap, d.tether_secs > 0.0);
        if yank {
            self.yank_to(p.owner, p.ability, p.share, at, unit);
        }
        let Some(v) = unit else { return };
        if swap {
            self.swap_with(p.owner, v, p.ability);
        } else if tether {
            self.bind(p.owner, v, p.ability);
        }
    }

    /// A grappling hook's pull: its caster dashes along the ground to where it struck (beside the
    /// one it struck), a wall stopping them short. Rooted or stunned, they stay.
    fn yank_to(&mut self, id: Id, ability: usize, share: f32, at: Vec3, unit: Option<Id>) {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return };
        if p.health == 0 || self.held_still(id) {
            return;
        }
        let from = p.pos;
        let (spot, stop) = match unit.and_then(|u| self.unit_pos(u).map(|pos| (u, pos))) {
            Some((u, pos)) => (pos, self.body_of(u) + HERO_SIZE + YANK_BESIDE),
            None => (at, YANK_WALL),
        };
        let dist = dist2d(from, spot) - stop;
        if dist < 0.5 {
            return;
        }
        let dir = facing(from, spot);
        let end = self.slide_end(from, dir, dist);
        let moved = dist2d(from, end);
        if moved < 0.5 {
            return;
        }
        let secs = (moved / YANK_SPEED).max(0.08);
        self.interrupt_hero(id);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = dir;
        }
        self.set_action(id, Action::Dash { t: 0.0, secs, ability, target: None, share });
        self.events.push(Event::Dash { player: id, dir, speed: moved / secs, secs });
        self.beta.yanks.insert(id, (end, now + secs));
    }

    /// How wide a unit's body is.
    fn body_of(&self, id: Id) -> f32 {
        self.mobs.get(&id).map_or(HERO_SIZE, |m| 0.5 * self.content.mobs[m.def].scale)
    }

    /// Firm ground to stand on at `at`: nothing solid there, and the ground near its height.
    fn firm(&self, at: Vec3) -> bool {
        let g = self.land.height_near(at[0], at[1], at[2] + 1.0);
        !self.land.blocked_near(at[0], at[1], g) && (g - at[2]).abs() < 3.0
    }

    /// The caster and the one their projectile struck trade places, each set down where the other
    /// stood (the ability's `burst_spell` at both spots), unless that one holds its ground: a
    /// champion, a totem, anything steadfast or untouchable.
    fn swap_with(&mut self, id: Id, v: Id, ability: usize) {
        if id == v || !self.players.contains_key(&id) || !self.alive(id) || !self.alive(v) {
            return;
        }
        let rooted = self.mobs.get(&v).is_some_and(|m| {
            let d = &self.content.mobs[m.def];
            d.boss || d.totem
        });
        if rooted || self.immune(v) || self.unstaggerable(v) {
            let text = format!("{} holds its ground", self.unit_name(v));
            self.events.push(Event::Flash { player: id, text });
            return;
        }
        let (Some(a), Some(b)) = (self.unit_pos(id), self.unit_pos(v)) else { return };
        let (a, b) = (self.standing_spot(a), self.standing_spot(b));
        if !self.firm(a) || !self.firm(b) {
            return;
        }
        let f = self.players[&id].facing;
        self.set_down(id, b, f);
        if let Some(f) = self.players.get(&v).map(|p| p.facing) {
            self.set_down(v, a, f);
        } else if let Some(m) = self.mobs.get_mut(&v) {
            m.pos = a;
            m.path.clear();
            m.speed = 0.0;
            m.sidle = None;
            m.plan_to = None;
            let f = m.facing;
            self.events.push(Event::MobStop { id: v, pos: a, facing: f });
        }
        let look = self.content.abilities[ability].burst_spell;
        if look != 0 {
            for at in [a, b] {
                self.events.push(Event::Missile { caster: id, spell: look, target: Target::Point(at) });
            }
        }
        let line = format!("swap: {} and {}", self.unit_name(id), self.unit_name(v));
        self.cc_note(id, line.clone());
        self.cc_note(v, line);
    }

    // --- tethers ------------------------------------------------------------------------------------

    /// A tether's projectile struck a foe: it's bound to the caster (with `chain`, so are the foes
    /// nearest it, as many as that).
    fn bind(&mut self, id: Id, v: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        if !self.players.contains_key(&id) || !self.tetherable(id, v) {
            return;
        }
        let mut bound = vec![v];
        if d.chain > 0 {
            if let Some(at) = self.unit_pos(v) {
                let range = if d.chain_range > 0.0 { d.chain_range } else { 8.0 };
                let mut near: Vec<(Id, f32)> = self
                    .mobs
                    .iter()
                    .map(|(m, x)| (*m, x.pos))
                    .chain(self.players.iter().map(|(h, x)| (*h, x.pos)))
                    .filter(|(u, pos)| *u != v && dist2d(*pos, at) <= range && self.tetherable(id, *u))
                    .map(|(u, pos)| (u, dist2d(pos, at)))
                    .collect();
                near.sort_by(|a, b| a.1.total_cmp(&b.1));
                bound.extend(near.into_iter().take(d.chain as usize).map(|(u, _)| u));
            }
        }
        let now = self.clock;
        for u in bound {
            self.beta.tethers.retain(|t| !(t.owner == id && t.foe == u));
            self.beta.tethers.push(Tether { owner: id, foe: u, ability, until: now + d.tether_secs, look_at: now });
            let line = format!("{}: tether on {} from {}", d.name, self.unit_name(u), self.unit_name(id));
            self.cc_note(u, line.clone());
            self.cc_note(id, line);
        }
    }

    /// Whether a tether of `id`'s can hold `u`: a living creature (not a hero's totem) or a hero
    /// they may harm.
    fn tetherable(&self, id: Id, u: Id) -> bool {
        if let Some(m) = self.mobs.get(&u) {
            return m.health > 0 && !m.hidden && !self.content.mobs[m.def].totem;
        }
        self.players.get(&u).is_some_and(|p| p.health > 0) && self.hostile(id, u)
    }

    /// Tethers hold while both live and the time lasts; a foe gone past the tether's reach is
    /// pulled back to the caster, given its `tether_effects` as it lands, and let go.
    fn step_tethers(&mut self) {
        let now = self.clock;
        let list = std::mem::take(&mut self.beta.tethers);
        let mut keep: Vec<Tether> = Vec::new();
        for mut t in list {
            let (a, b) = (self.unit_pos(t.owner), self.unit_pos(t.foe));
            let held = self.players.contains_key(&t.owner) && self.alive(t.owner) && self.alive(t.foe);
            let (Some(a), Some(b), true) = (a, b, held && now < t.until) else {
                self.unlink(&t);
                continue;
            };
            let d = self.content.abilities[t.ability].clone();
            if dist2d(a, b) > d.tether_range {
                self.drag_back(&t, &d);
                self.unlink(&t);
                continue;
            }
            // Its link, from the caster to the first foe they hold (a channel's look).
            let first = !keep.iter().any(|k| k.owner == t.owner);
            if d.tick_spell != 0 && first && now >= t.look_at && !self.channelling(t.owner) {
                let ms = ((t.until - now) * 1000.0) as u32;
                self.events.push(Event::Channel { caster: t.owner, target: Some(t.foe), spell: d.tick_spell, ms });
                t.look_at = now + LINK_EVERY;
            }
            keep.push(t);
        }
        keep.extend(std::mem::take(&mut self.beta.tethers));
        self.beta.tethers = keep;
    }

    fn channelling(&self, id: Id) -> bool {
        matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Channel { .. }))
    }

    /// A tether's link comes down (not a channel the caster is holding meanwhile).
    fn unlink(&mut self, t: &Tether) {
        if self.content.abilities[t.ability].tick_spell != 0 && self.players.contains_key(&t.owner) && !self.channelling(t.owner) {
            self.events.push(Event::Channel { caster: t.owner, target: None, spell: 0, ms: 0 });
        }
    }

    /// A foe that strayed too far is dragged back to the caster; its tether's effects land as it
    /// does.
    fn drag_back(&mut self, t: &Tether, d: &AbilityDef) {
        let (Some(a), Some(b)) = (self.unit_pos(t.owner), self.unit_pos(t.foe)) else { return };
        let pull = AbilityEffect { kind: "pull".into(), amount: dist2d(a, b) + 1.0, lift: TETHER_LIFT, from_caster: true, ..Default::default() };
        self.displace(t.owner, t.foe, &pull);
        if !d.tether_effects.is_empty() {
            self.beta.later.push(Later { at: self.clock + airtime(TETHER_LIFT), owner: t.owner, victim: t.foe, effects: d.tether_effects.clone(), source: d.name.clone() });
        }
        if self.players.contains_key(&t.owner) {
            self.events.push(Event::Flash { player: t.owner, text: format!("{}: dragged back", d.name) });
        }
    }

    // --- a knockback into a wall ---------------------------------------------------------------------

    /// Whether a body slid to `at` along `dir` was stopped there by a wall or a steep step up (not
    /// a drop).
    pub(crate) fn walled(&self, at: Vec3, dir: f32) -> bool {
        let p = [at[0] + dir.cos() * 0.4, at[1] + dir.sin() * 0.4];
        let g = self.land.height_near(p[0], p[1], at[2] + 1.0);
        self.land.blocked_near(p[0], p[1], g) || g - at[2] > 1.5
    }

    /// A body thrown into a wall is stunned `secs` as it hits it, `air` from now.
    pub(crate) fn wall_slam(&mut self, owner: Id, victim: Id, secs: f32, air: f32) {
        let stun = AbilityEffect { kind: "stun".into(), secs: [secs, secs], ..Default::default() };
        self.beta.later.push(Later { at: self.clock + air, owner, victim, effects: vec![stun], source: "Wall slam".into() });
        if self.players.contains_key(&owner) {
            self.events.push(Event::Flash { player: owner, text: "Into the wall!".into() });
        }
    }

    // --- echoes (Rewind) ------------------------------------------------------------------------------

    /// A buff that leaves an echo has gone (from `at`): the echo there, holding the hero's health
    /// now, its look on the ground while it lasts.
    pub(crate) fn leave_echo(&mut self, id: Id, ability: usize, at: Vec3) {
        if !self.content.abilities[ability].rewind {
            return;
        }
        let d = self.content.abilities[ability].clone();
        let Some(health) = self.players.get(&id).map(|p| p.health) else { return };
        let now = self.clock;
        let at = self.standing_spot(at);
        let recast = self.content.ability(&d.recast);
        let life = if recast.is_some() { d.recast_secs } else { d.rewind_secs };
        if let Some(old) = self.beta.echoes.remove(&id) {
            self.unmark(old.mark);
        }
        let mark = self.mark_kind(at, 1.0, life + 0.1, true);
        if d.linger_spell != 0 {
            self.events.push(Event::AreaLook { id: mark, caster: id, spell: d.linger_spell, pos: at, radius: 1.0 });
        }
        if d.burst_spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.burst_spell, target: Target::Point(at) });
        }
        self.beta.echoes.insert(id, Echo { ability, recast, at, health, until: now + life, mark });
    }

    /// A recast goes: if it's the one an echo waits for, the hero is back at the echo first.
    pub(crate) fn rewind_recast(&mut self, id: Id, recast: usize) {
        if self.beta.echoes.get(&id).is_some_and(|e| e.recast == Some(recast)) {
            self.snap_to_echo(id);
        }
    }

    /// Back to the echo: set down there, with the health it held if that's more than now.
    fn snap_to_echo(&mut self, id: Id) {
        let Some(e) = self.beta.echoes.remove(&id) else { return };
        self.unmark(e.mark);
        let Some((f, health)) = self.players.get(&id).filter(|p| p.health > 0).map(|p| (p.facing, p.health)) else { return };
        let spell = self.content.abilities[e.ability].spell;
        self.set_down(id, e.at, f);
        if spell != 0 {
            self.events.push(Event::Missile { caster: id, spell, target: Target::Unit(id) });
        }
        if e.health > health {
            self.heal_hero(id, e.health - health);
        }
        let line = format!("{}: back to the echo", self.content.abilities[e.ability].name);
        self.cc_note(id, line);
    }

    /// Echoes with no recast call their heroes back when their time is up; a recast's fades once
    /// its window has closed unused; a fallen hero's goes.
    fn step_echoes(&mut self) {
        let now = self.clock;
        let ids: Vec<Id> = self.beta.echoes.keys().copied().collect();
        for h in ids {
            let Some(e) = self.beta.echoes.get(&h).cloned() else { continue };
            let alive = self.players.get(&h).is_some_and(|p| p.health > 0);
            let base = self.content.base_of(e.ability);
            match e.recast {
                None if alive && now >= e.until => self.snap_to_echo(h),
                Some(_) if alive && self.recast_armed(h, base).is_some() => {}
                None if alive => {}
                _ => {
                    self.beta.echoes.remove(&h);
                    self.unmark(e.mark);
                }
            }
        }
    }

    // --- jump pads ------------------------------------------------------------------------------------

    /// An armed pad launches the caster or a friend standing on it (each at most once a second).
    pub(crate) fn pad_launches(&mut self, t: &Trap) {
        let now = self.clock;
        let d = self.content.abilities[t.ability].clone();
        let radius = AbilityDef::at(d.radius, t.share);
        let on: Vec<Id> = self
            .players
            .iter()
            .filter(|(h, p)| {
                p.health > 0
                    && (**h == t.owner || self.helpable(t.owner, **h))
                    && dist2d(p.pos, t.at) <= radius + HERO_SIZE
                    && (p.pos[2] - t.at[2]).abs() < 3.0
                    && self.beta.launched.get(*h).is_none_or(|w| now - *w >= PAD_EVERY)
            })
            .map(|(h, _)| *h)
            .collect();
        for h in on {
            if self.launch_hero(h, t.ability, t.share, d.launch) && d.burst_spell != 0 {
                self.events.push(Event::Missile { caster: t.owner, spell: d.burst_spell, target: Target::Point(t.at) });
            }
        }
    }

    /// A hero thrown `dist` yards the way they face, as a leap (it lands as the pad's ability
    /// does: `damage` and `effects` within its `radius`, as theirs). False: held, they stay.
    fn launch_hero(&mut self, h: Id, ability: usize, share: f32, dist: f32) -> bool {
        let now = self.clock;
        let Some((pos, f)) = self.players.get(&h).map(|p| (p.pos, p.facing)) else { return false };
        if self.held_still(h) || self.immune(h) {
            return false;
        }
        let d = self.content.abilities[ability].clone();
        let aim = self.ground_near(pos[0] + f.cos() * dist, pos[1] + f.sin() * dist, pos[2] + 1.0);
        let to = if self.land.blocked_near(aim[0], aim[1], aim[2]) || (aim[2] - pos[2]).abs() > 8.0 { self.slide_end(pos, f, dist) } else { aim };
        let moved = dist2d(pos, to);
        let lift = if d.lift > 0.0 { d.lift } else { PAD_LIFT };
        let air = airtime(lift);
        self.beta.launched.insert(h, now);
        self.interrupt_hero(h);
        self.events.push(Event::Hop { player: h, dir: f, speed: moved / air, lift, roll: false, style: "" });
        self.set_action(h, Action::Dash { t: 0.0, secs: air, ability, target: None, share });
        self.landings.push(Landing { owner: h, ability, at: to, share, land_at: now + air, mark: None });
        let line = format!("{}: launched {} {moved:.1} yd", d.name, self.unit_name(h));
        self.cc_note(h, line);
        true
    }

    // --- cheating death ---------------------------------------------------------------------------------

    /// A blow of `damage` on a hero with a boon that cheats death, that would fell them, leaves
    /// them at 1 health instead: the boon ends, and they mend its share of their health over the
    /// next few seconds. What's left of the blow.
    pub(crate) fn cheat_death(&mut self, id: Id, damage: u32) -> u32 {
        let now = self.clock;
        let Some(h) = self.players.get(&id).map(|p| p.health) else { return damage };
        if h == 0 || damage < h {
            return damage;
        }
        let Some(c) = self.cc_of(id).iter().find(|c| c.kind == "cheat_death" && c.until > now).cloned() else { return damage };
        self.cleanse(id, &["cheat_death"]);
        if c.amount > 0.0 {
            self.apply_cc(id, c.from, "regen", CHEAT_SECS, c.amount / CHEAT_SECS, 0.0, &c.source);
        }
        self.events.push(Event::Flash { player: id, text: "Death cheated!".into() });
        self.cc_note(id, format!("{}: a killing blow cheated", c.source));
        h - 1
    }

    // --- mirrors ----------------------------------------------------------------------------------------

    /// The mirror's stance a unit is in, if any (its ability).
    fn mirror_of(&self, id: Id) -> Option<usize> {
        let s = self.stances.get(&id).filter(|s| s.until > self.clock)?;
        self.content.abilities[s.ability].counter.as_ref().is_some_and(|c| c.mirror).then_some(s.ability)
    }

    /// A projectile reaching `v`: in a mirror's stance, it's turned back at whoever threw it (theirs
    /// now, as hard and with what it does), and true. Help from a friend goes on to them.
    pub(crate) fn mirrors(&mut self, p: &mut Projectile, v: Id) -> bool {
        let Some(stance) = self.mirror_of(v) else { return false };
        if p.owner == v || (p.allies && self.players.contains_key(&v) && self.helpable(p.owner, v)) {
            return false;
        }
        let thrower = p.owner;
        let back = self.unit_pos(thrower).map(|o| [o[0], o[1], o[2] + 1.0]);
        let range = AbilityDef::at(self.content.abilities[p.ability].range, p.share).max(4.0);
        let (dir, range) = match back {
            Some(b) => (unit3(sub3(b, p.pos)), range.max(len3(sub3(b, p.pos)) + 4.0)),
            None => ([-p.dir[0], -p.dir[1], -p.dir[2]], range),
        };
        p.owner = v;
        p.dir = dir;
        p.left = range;
        p.hits = vec![v];
        p.allies = false;
        p.stage = Stage::Out;
        let end = back.unwrap_or([p.pos[0] + dir[0] * range, p.pos[1] + dir[1] * range, p.pos[2] + dir[2] * range]);
        self.events.push(Event::Missile { caster: v, spell: p.spell, target: Target::Point(end) });
        self.mirror_caught(v, stance, thrower);
        true
    }

    /// A creature's bolt coming down on a hero in a mirror's stance flies back at the creature
    /// (theirs now, as hard as it would have struck them), and true; it strikes no one there.
    pub(crate) fn mirror_missile(&mut self, m: &combat::Missile) -> bool {
        // Only what flew (a creature's shove at its own feet isn't a projectile).
        if !self.mobs.contains_key(&m.owner) || dist2d(m.from, m.at) < 2.0 {
            return false;
        }
        let found = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, m.at) <= m.radius.max(HERO_SIZE)).find_map(|(h, _)| self.mirror_of(*h).map(|s| (*h, s)));
        let Some((h, stance)) = found else { return false };
        let (Some(start), Some(to)) = (self.unit_pos(h), self.unit_pos(m.owner)) else { return false };
        let start = [start[0], start[1], start[2] + HAND_UP];
        let to = [to[0], to[1], to[2] + 1.0];
        let a = self.attack_now(m.owner);
        let spell = if a.spell != 0 { a.spell } else { self.content.abilities[stance].spell };
        let damage = m.damage as f32 * self.foe_blow_share(m.owner);
        let way = sub3(to, start);
        self.events.push(Event::Missile { caster: h, spell, target: Target::Point(to) });
        let born = self.clock;
        self.projectiles.push(Projectile {
            owner: h,
            ability: stance,
            pos: start,
            dir: unit3(way),
            speed: MIRROR_SPEED,
            radius: 0.5,
            left: len3(way) + 4.0,
            share: 1.0,
            damage,
            spell,
            allies: false,
            hits: vec![h],
            pierce: 0,
            stage: Stage::Out,
            born,
        });
        self.mirror_caught(h, stance, m.owner);
        true
    }

    /// A mirror caught something: Valor and its name for the hero, its look, what it does to the
    /// thrower (as a counter's `attacker`), and a `once` stance ends.
    fn mirror_caught(&mut self, v: Id, stance: usize, thrower: Id) {
        let d = self.content.abilities[stance].clone();
        let Some(c) = d.counter.clone() else { return };
        if let Some(s) = self.stances.get_mut(&v) {
            s.caught = true;
        }
        if self.players.contains_key(&v) {
            self.gain_valor(v, self.content.valor.counter);
            self.events.push(Event::Flash { player: v, text: d.name.clone() });
        }
        self.events.push(Event::Missile { caster: v, spell: d.spell, target: Target::Unit(v) });
        let close = self.unit_pos(thrower).zip(self.unit_pos(v)).is_some_and(|(a, b)| dist2d(a, b) <= 6.0);
        if !c.attacker.is_empty() && (c.any_attacker || close) {
            self.apply_effects(v, thrower, &c.attacker, 1.0, &d.name);
        }
        if c.once {
            self.end_stance(v);
        }
    }

    // --- bursts ---------------------------------------------------------------------------------------

    /// A shield that bursts as it ends was given to `holder`: its burst (`pop`, a nova) waits on
    /// it (one a hero, the latest).
    pub(crate) fn hold_pop(&mut self, holder: Id, pop: &str) {
        let Some(nova) = self.content.ability(pop) else { return };
        self.beta.pops.retain(|(h, _)| *h != holder);
        self.beta.pops.push((holder, nova));
    }

    /// Lingering ground that bursts as it ends goes off over its whole ground (or the burst's own
    /// radius): the burst's damage, effects and look.
    pub(crate) fn end_burst(&mut self, b: &Burning) {
        let Some(nova) = self.content.ability(&self.content.abilities[b.ability].end_burst) else { return };
        if !self.players.contains_key(&b.owner) {
            return;
        }
        let d = self.content.abilities[nova].clone();
        let radius = if d.radius[1] > 0.0 { AbilityDef::at(d.radius, 1.0) } else { b.radius };
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: b.owner, spell: d.spell, target: Target::Point(b.at) });
        }
        self.area(b.owner, nova, b.at, radius, 1.0, AbilityDef::at(d.damage, 1.0), None);
    }
}
