//! A hero's abilities: the keys they're on (the loadout), the ring tree they're taken from,
//! aiming along the reticle, charging, letting go, and what each kind does when it goes:
//! projectiles that fly and strike the first thing they touch, ground that's marked and then
//! lands, totems, cones, novas, tethers, sweeping beams, help aimed at a friend, dashes and blinks,
//! and the counters' stances. Valor is earned by fighting well and spent on the EX versions and
//! the ultimate.
//!
//! Nothing here finds a foe for the hero: every aimed ability goes where the reticle points. Only
//! help aimed at a friend (`ally`) leans toward the friend nearest the reticle, else the caster.

use std::collections::{BTreeMap, BTreeSet};

use super::recasts::Free;
use super::*;
use crate::content::{slot_role, AbilityDef, SLOTS};
use crate::land::clear_line;

/// A projectile in flight: where it is, which way and how fast it goes, how far it has left,
/// what it does when it touches something.
#[derive(Debug, Clone)]
pub(crate) struct Projectile {
    pub owner: Id,
    pub ability: usize,
    pub pos: Vec3,
    pub dir: Vec3,
    pub speed: f32,
    pub radius: f32,
    pub left: f32,
    /// The share of the full charge it went at.
    pub share: f32,
    /// Its damage (the bow's, for a shot; otherwise the ability's at that share).
    pub damage: f32,
    pub spell: u32,
    /// It meets friends too (to heal them).
    pub allies: bool,
    /// Who it has gone through already, and how many more it can.
    pub hits: Vec<Id>,
    pub pierce: u32,
    /// Where it is in its flight (going out, as most only ever do).
    pub stage: Stage,
    /// When it left the hands.
    pub born: f32,
}

/// Where a projectile is in its flight: going out (every projectile, at first); coming back to
/// its thrower (a boomerang, its look flown from where it turned by the unseen `anchor`); arcing
/// to the spot `at`, landing at `land_at` (a lob, its ring `mark` shown till then); or stuck to a
/// unit (`on`; none: to the ground or a wall), at `at`, going off at `until` (a fuse, its ring
/// `mark` shown for the last second).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Stage {
    Out,
    Back { anchor: Id },
    Lob { at: Vec3, land_at: f32, mark: Option<Id> },
    Stuck { on: Option<Id>, at: Vec3, until: f32, mark: Option<Id> },
}

/// Ground marked for an ability, landing at `land_at`.
#[derive(Debug, Clone)]
pub(crate) struct GroundCast {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub radius: f32,
    pub share: f32,
    pub land_at: f32,
    pub mark: Option<Id>,
    /// Its look already on its way down (`look_lead`).
    pub shown: bool,
}

/// Ground left burning: whoever stands in it takes `dps` a second until then.
#[derive(Debug, Clone)]
pub(crate) struct Burning {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub radius: f32,
    pub dps: f32,
    pub until: f32,
    pub next: f32,
    pub mark: Option<Id>,
}

/// A totem a hero set down: the creature it is, whose, and when it pulses next and falls.
#[derive(Debug, Clone)]
pub(crate) struct Totem {
    pub mob: Id,
    pub owner: Id,
    pub ability: usize,
    pub until: f32,
    pub next: f32,
}

/// A counter's stance on a unit (a hero's, or a training dummy's): which, until when, and whether
/// it has caught anything.
#[derive(Debug, Clone)]
pub struct Stance {
    pub ability: usize,
    pub until: f32,
    pub caught: bool,
}

/// An ability's cooldown: the charges in hand, and when the next comes back.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Cooldown {
    pub charges: u32,
    pub next_at: f32,
}

/// What a projectile's or a reticle's line meets first.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Trace {
    pub dist: f32,
    pub point: Vec3,
    pub unit: Option<Id>,
    /// It met the ground or a wall (no unit, short of its range).
    pub solid: bool,
}

/// How an ability's blow reaches someone: counters fire on melee and projectiles only.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    Melee,
    Projectile,
    /// Ground, a cone, a nova, a beam: counters let it by.
    Area,
}

/// The ground an ability strikes: a circle round its spot, a wedge in front (facing, arc), or a
/// strip along a facing (`len` long, `half` either side).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Shape {
    Circle,
    Cone(f32, f32),
    Line(f32, f32, f32),
}

/// The aiming camera's pivot over a hero's feet, and how far right of them it sits.
const EYE: f32 = 1.9;
const SHOULDER: f32 = 0.75;
/// Where a cast leaves the hands: this high, this far in front and to the right.
const HAND_UP: f32 = 1.45;
const HAND_AHEAD: f32 = 0.6;
const HAND_RIGHT: f32 = 0.3;
/// The reticle's line a cast carries counts this long, and this near the hero's facing.
const AIM_FRESH: f32 = 0.6;
/// After letting an ability go, a beat before the next thing.
const CAST_RECOVERY: f32 = 0.35;
/// The throw's clip (AttackThrown) and the pose a thrown charge holds till it goes (HoldThrown):
/// the axe drawn back, so everyone can see a throw coming, as a bow's draw shows.
const THROW_CLIP: u16 = 107;
const HOLD_THROWN: u16 = 111;
/// Holy ground's boon lasts this long past each tick (so it holds while they stand in it).
const BOON_HOLD: f32 = 1.5;
/// A friend healed: Flash of Light's look on them.
const HEAL_LOOK: u32 = 19750;
/// Help aimed at a friend that misses them along the reticle still finds the friend nearest it
/// within this many degrees of it (and an ally's trace is this wide, unless it says).
const ALLY_CONE: f32 = 15.0;
const ALLY_TRACE: f32 = 1.0;
/// A frozen foe struck by a shattering ability takes this many times the damage.
const SHATTER: f32 = 1.5;
/// A dash runs this fast; a blink's landing must be this near the ground the line meets.
const DASH_SPEED: f32 = 30.0;
/// How far a projectile goes in one step of its flight.
const STEP: f32 = 0.4;
/// A hero's and a creature's bodies for blows and projectiles: how wide, how tall.
pub(crate) const HERO_SIZE: f32 = 0.4;
const HERO_TALL: f32 = 2.0;

/// The direction a facing and an elevation look (x north, y west, z up).
pub fn look_dir(yaw: f32, pitch: f32) -> Vec3 {
    [yaw.cos() * pitch.cos(), yaw.sin() * pitch.cos(), pitch.sin()]
}

fn add(a: Vec3, b: Vec3, k: f32) -> Vec3 {
    [a[0] + b[0] * k, a[1] + b[1] * k, a[2] + b[2] * k]
}

fn norm(v: Vec3) -> Vec3 {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1.0e-5);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// From `a` to `b`, and how far that is.
fn sub(b: Vec3, a: Vec3) -> Vec3 {
    [b[0] - a[0], b[1] - a[1], b[2] - a[2]]
}

fn len(v: Vec3) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// How far a point is from a body standing at `feet`, `tall` high.
fn to_body(p: Vec3, feet: Vec3, tall: f32) -> f32 {
    let z = p[2].clamp(feet[2], feet[2] + tall);
    let (dx, dy, dz) = (p[0] - feet[0], p[1] - feet[1], p[2] - z);
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// A class's first cards: its column's cards on the rows a new hero starts with (`start.cards`).
pub(crate) fn first_cards(c: &Content, class: &str) -> BTreeSet<String> {
    c.abilities.iter().filter(|a| a.on_board() && a.school == class && c.start.cards.contains(&a.slot)).map(|a| a.key.clone()).collect()
}

impl Survival {
    // --- the loadout and the board ---------------------------------------------------------------

    /// A fresh hero's cards and keys: their class's first cards (its Q and E, on those keys);
    /// every card besides in a dev build (the bench's too), the class's whole column on its keys.
    pub(crate) fn fresh_abilities(&self, class: &str) -> (BTreeMap<String, String>, BTreeSet<String>) {
        let full = self.content.dev.full_tree;
        let owned = if full { self.all_nodes() } else { self.core_nodes(class) };
        let mut loadout = BTreeMap::new();
        for a in self.content.abilities.iter().filter(|a| a.on_board() && a.school == class && (full || owned.contains(&a.key))) {
            loadout.insert(a.slot.clone(), a.key.clone());
        }
        (loadout, owned)
    }

    /// Every card there is (the abilities that aren't EX versions, nor fired only by others; the
    /// bench's too).
    pub fn all_nodes(&self) -> BTreeSet<String> {
        self.content.abilities.iter().filter(|a| !a.is_ex() && !a.inner).map(|a| a.key.clone()).collect()
    }

    /// A class's first cards: what a new hero of it knows (its column's Q and E, `start.cards`).
    pub(crate) fn core_nodes(&self, class: &str) -> BTreeSet<String> {
        first_cards(&self.content, class)
    }

    /// Whether a card can be taken from the board as it stands: in a dev world (debug on), any on
    /// it; a beta card, anywhere but Siege (they're there to be tried); otherwise none, since
    /// cards are learned out in Elwynn (`learned_where`).
    pub fn reachable(&self, _owned: &BTreeSet<String>, key: &str) -> bool {
        let Some(d) = self.content.ability(key).map(|k| &self.content.abilities[k]).filter(|d| d.on_board()) else { return false };
        if d.is_beta() {
            return self.debug || !self.is_siege();
        }
        // In a Siege match, any card is bought with gold.
        self.debug || (self.is_siege() && self.card_price(key).is_some())
    }

    /// Unlock a card of the board (`.es tree take`).
    pub fn take_node(&mut self, id: Id, key: &str) -> Result<String, String> {
        let a = self.content.ability(key).ok_or("No such ability.")?;
        let def = self.content.abilities[a].clone();
        if def.is_ex() {
            return Err("An EX version comes with its ability.".into());
        }
        let owned = self.players.get(&id).ok_or("No hero.")?.owned.clone();
        if owned.contains(key) {
            return Ok(def.name);
        }
        if !self.reachable(&owned, key) {
            if def.is_beta() && def.on_board() && self.is_siege() {
                return Err(format!("{} is a beta card: Siege leaves those out.", def.name));
            }
            let at = self.learned_where(key);
            return Err(if at.is_empty() { "That card isn't on the board.".into() } else { format!("{} is learned out in Elwynn: {at}.", def.name) });
        }
        // Siege: bought, in a match.
        if self.is_siege() && !self.debug {
            if self.camp_of(id).is_none() {
                return Err("Cards are bought in a match.".into());
            }
            let price = self.card_price(key).unwrap_or(0);
            self.spend_gold(id, price)?;
            self.players.get_mut(&id).unwrap().owned.insert(key.to_string());
            self.siege_slot_bought(id, key);
            let on = self.players[&id].loadout.iter().find(|(_, k)| *k == key).map(|(s, _)| s.clone());
            let tail = match on {
                Some(s) => format!("it's on your {s} key"),
                None => "put it on its key from the board (N)".to_string(),
            };
            self.tell(id, format!("{} is yours for {price} gold: {tail}.", def.name));
        }
        self.players.get_mut(&id).unwrap().owned.insert(key.to_string());
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Dirty);
        Ok(def.name)
    }

    /// The whole tree, or none of it (dev: `.es tree full`, `.es tree reset`).
    pub fn set_tree(&mut self, id: Id, full: bool) {
        let class = self.players.get(&id).map_or("warrior".to_string(), |p| p.class.clone());
        let owned = if full { self.all_nodes() } else { self.core_nodes(&class) };
        if let Some(p) = self.players.get_mut(&id) {
            p.owned = owned.clone();
            // What's no longer owned comes off its key.
            p.loadout.retain(|_, k| owned.contains(k));
        }
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Dirty);
    }

    /// Put an ability on a key: `slot` a slot name (`spell1`...) or the ability's own role (the
    /// first slot of it, or the second spell slot when the first holds another).
    pub fn slot_ability(&mut self, id: Id, slot: &str, key: &str) -> Result<String, String> {
        let a = self.content.ability(key).ok_or_else(|| {
            let keys: Vec<&str> = self.content.abilities.iter().filter(|a| !a.is_ex() && !a.inner).map(|a| a.key.as_str()).collect();
            format!("No ability {key}: try {}.", keys.join(", "))
        })?;
        let def = self.content.abilities[a].clone();
        if def.is_ex() {
            return Err(format!("{} is an EX version: hold {}'s key.", def.name, self.content.ability(&def.ex_of).map_or("its ability", |b| self.content.abilities[b].name.as_str())));
        }
        let p = self.players.get(&id).ok_or("No hero.")?;
        if !p.owned.contains(key) {
            return Err(format!("{} isn't yours yet.", def.name));
        }
        let slot = if SLOTS.contains(&slot) {
            slot.to_string()
        } else {
            let role = def.slot.as_str();
            let free = SLOTS.iter().find(|s| slot_role(s) == role && p.loadout.get(**s).is_none_or(|k| k == key));
            free.or_else(|| SLOTS.iter().find(|s| slot_role(s) == role)).map(|s| s.to_string()).unwrap_or_default()
        };
        if slot_role(&slot) != def.slot {
            return Err(format!("{} goes on the {} key, not {slot}.", def.name, def.slot));
        }
        let p = self.players.get_mut(&id).unwrap();
        // Off the other key it was on.
        p.loadout.retain(|_, k| k != key);
        p.loadout.insert(slot.clone(), key.to_string());
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Dirty);
        Ok(format!("{} on {slot}", def.name))
    }

    // --- Valor -------------------------------------------------------------------------------------

    pub(crate) fn gain_valor(&mut self, id: Id, amount: f32) {
        let max = self.content.valor.max;
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.health == 0 || amount <= 0.0 {
            return;
        }
        let before = p.valor.floor();
        p.valor = (p.valor + amount).min(max);
        if p.valor.floor() != before {
            self.events.push(Event::Valor { player: id });
        }
    }

    pub fn set_valor(&mut self, id: Id, v: f32) {
        let max = self.content.valor.max;
        if let Some(p) = self.players.get_mut(&id) {
            p.valor = v.clamp(0.0, max);
            self.events.push(Event::Valor { player: id });
        }
    }

    // --- cooldowns ---------------------------------------------------------------------------------

    /// An ability's cooldown as it stands now (charges back that are due).
    pub fn cooldown(&self, id: Id, ability: usize) -> Cooldown {
        let base = self.content.base_of(ability);
        let d = &self.content.abilities[base];
        let max = d.charges.max(1);
        let Some(cd) = self.players.get(&id).and_then(|p| p.cooldowns.get(&base)).copied() else {
            return Cooldown { charges: max, next_at: 0.0 };
        };
        let mut cd = cd;
        let every = d.cooldown.max(0.0);
        while cd.charges < max && self.clock >= cd.next_at {
            cd.charges += 1;
            cd.next_at += every;
        }
        cd
    }

    pub(crate) fn ready(&self, id: Id, ability: usize) -> bool {
        self.players.get(&id).is_some_and(|p| p.cd_off) || self.cooldown(id, ability).charges > 0
    }

    /// Spend a charge of an ability (`share` of its cooldown: a counter that caught something
    /// comes back sooner).
    fn spend_charge(&mut self, id: Id, ability: usize, share: f32) {
        let base = self.content.base_of(ability);
        let d = self.content.abilities[base].clone();
        let max = d.charges.max(1);
        let now = self.clock;
        let mut cd = self.cooldown(id, ability);
        // A clarity draught: the cooldown is shorter while it lasts.
        let focus = self.players.get(&id).map_or(0.0, |p| self.effect_of(p, "focus")).clamp(0.0, 0.5);
        let every = d.cooldown * share * (1.0 - focus);
        if cd.charges >= max {
            cd.next_at = now + every;
        }
        cd.charges = cd.charges.saturating_sub(1);
        if let Some(p) = self.players.get_mut(&id) {
            p.cooldowns.insert(base, cd);
        }
        self.events.push(Event::Abilities { player: id });
    }

    /// Cooldowns off or on (dev), and every one ready now.
    pub fn set_cooldowns(&mut self, id: Id, off: bool) {
        if let Some(p) = self.players.get_mut(&id) {
            p.cd_off = off;
            p.cooldowns.clear();
        }
        self.events.push(Event::Abilities { player: id });
    }

    // --- the bar -------------------------------------------------------------------------------------

    /// What's on a hero's ability keys, for their bar: each slot's ability (if any), its EX
    /// version, its cooldown.
    pub fn bar(&self, id: Id) -> Vec<(String, Option<usize>, Option<usize>, Cooldown)> {
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        SLOTS
            .iter()
            .map(|s| {
                let a = p.loadout.get(*s).and_then(|k| self.content.ability(k)).filter(|a| p.owned.contains(&self.content.abilities[*a].key));
                let ex = a.and_then(|a| self.content.ex_of(a));
                let cd = a.map_or(Cooldown { charges: 0, next_at: 0.0 }, |a| self.cooldown(id, a));
                (s.to_string(), a, ex, cd)
            })
            .collect()
    }

    // --- aiming --------------------------------------------------------------------------------------

    /// The reticle's line: its facing and elevation as the client last told it (fresh), else the
    /// hero's facing, level.
    pub(crate) fn aim_of(&self, id: Id) -> (f32, f32) {
        let Some(p) = self.players.get(&id) else { return (0.0, 0.0) };
        match p.aim_line {
            Some((f, e, at)) if self.clock - at <= AIM_FRESH => (f, e),
            _ => (p.facing, 0.0),
        }
    }

    /// The reticle's line from the aiming camera: where it starts (over the right shoulder, as the
    /// camera sits) and which way it goes.
    pub(crate) fn reticle(&self, id: Id) -> (Vec3, Vec3) {
        let p = &self.players[&id];
        let (yaw, pitch) = self.aim_of(id);
        let right = if p.shoulder_cam { SHOULDER } else { 0.0 };
        let o = [p.pos[0] + yaw.sin() * right, p.pos[1] - yaw.cos() * right, p.pos[2] + EYE];
        (o, look_dir(yaw, pitch))
    }

    /// Where a cast leaves the hero's hands.
    fn hands(&self, id: Id, yaw: f32) -> Vec3 {
        let p = &self.players[&id];
        [
            p.pos[0] + yaw.cos() * HAND_AHEAD + yaw.sin() * HAND_RIGHT,
            p.pos[1] + yaw.sin() * HAND_AHEAD - yaw.cos() * HAND_RIGHT,
            p.pos[2] + HAND_UP,
        ]
    }

    /// The bodies a line can meet: (unit, feet, height, width), not `skip`'s (and heroes only with
    /// friendly fire on).
    fn bodies(&self, skip: Id) -> Vec<(Id, Vec3, f32, f32)> {
        self.bodies_with(skip, false)
    }

    /// [`Self::bodies`], with every other hero among them when `allies` (a friend to heal).
    fn bodies_with(&self, skip: Id, allies: bool) -> Vec<(Id, Vec3, f32, f32)> {
        let mut out: Vec<(Id, Vec3, f32, f32)> = self
            .mobs
            .iter()
            .filter(|(id, m)| **id != skip && m.health > 0 && !m.hidden)
            .map(|(id, m)| {
                let s = self.content.mobs[m.def].scale;
                (*id, m.pos, HERO_TALL * s, 0.5 * s)
            })
            .collect();
        if self.content.pvp.friendly_fire || allies {
            out.extend(self.players.iter().filter(|(id, p)| **id != skip && p.health > 0 && (allies || self.hostile(skip, **id))).map(|(id, p)| (*id, p.pos, HERO_TALL, HERO_SIZE)));
        }
        out
    }

    /// Follow a line from `from` along `dir` for `range`: the first body it passes within
    /// `radius` of, the ground it goes into or the wall it meets.
    pub(crate) fn trace(&self, from: Vec3, dir: Vec3, range: f32, radius: f32, skip: Id) -> Trace {
        self.trace_with(from, dir, range, radius, skip, false)
    }

    /// [`Self::trace`], meeting friends too when `allies`.
    pub(crate) fn trace_with(&self, from: Vec3, dir: Vec3, range: f32, radius: f32, skip: Id, allies: bool) -> Trace {
        let bodies = self.bodies_with(skip, allies);
        let mut d = 0.0f32;
        while d < range {
            let next = (d + STEP).min(range);
            let p = add(from, dir, next);
            for (id, feet, tall, size) in &bodies {
                if to_body(p, *feet, *tall) <= size + radius {
                    return Trace { dist: next, point: p, unit: Some(*id), solid: false };
                }
            }
            let ground = self.land.height_near(p[0], p[1], p[2] + 0.6);
            if p[2] < ground - 0.05 || self.land.blocked_near(p[0], p[1], p[2]) {
                return Trace { dist: d, point: add(from, dir, d), unit: None, solid: true };
            }
            d = next;
        }
        Trace { dist: range, point: add(from, dir, range), unit: None, solid: false }
    }

    /// Where the reticle's line meets the ground (or a body, or its range's end, dropped to the
    /// ground there), within `range` of the hero.
    pub(crate) fn aim_point(&self, id: Id, range: f32) -> Vec3 {
        let (o, dir) = self.reticle(id);
        let pos = self.players[&id].pos;
        // From the camera, the line goes a little farther than the range from the hero.
        let t = self.trace(o, dir, range + 3.0, 0.0, id);
        let mut at = t.point;
        let flat = dist2d(pos, at);
        if flat > range {
            let k = range / flat.max(0.01);
            at = [pos[0] + (at[0] - pos[0]) * k, pos[1] + (at[1] - pos[1]) * k, at[2]];
        }
        self.ground_near(at[0], at[1], at[2] + 1.0)
    }

    // --- pressing the keys -----------------------------------------------------------------------------

    /// An ability key pressed (`ex`: with Ctrl): aim it, or (a stance, a nova) cast it now.
    pub(crate) fn ability_key(&mut self, id: Id, slot: usize, ex: bool) -> bool {
        let Some(p) = self.players.get(&id) else { return true };
        let Some(slot_name) = SLOTS.get(slot) else { return true };
        let Some(base) = p.loadout.get(*slot_name).and_then(|k| self.content.ability(k)) else {
            self.tell(id, format!("Nothing on that key yet: open the board (N) and choose a card for it ({}).", slot_role(slot_name)));
            return true;
        };
        // Ice Block: its key again breaks out of it.
        if let Action::Stance { ability, .. } = p.action {
            if self.content.abilities[ability].counter.as_ref().is_some_and(|c| c.immune) && self.content.base_of(ability) == base {
                self.end_stance(id);
                return true;
            }
        }
        // A beam: its key again ends it early.
        if let Action::Channel { ability, .. } = p.action {
            if self.content.abilities[ability].kind == "beam" && self.content.base_of(ability) == base {
                self.end_ability_action(id);
                self.set_action(id, Action::Ready);
                return true;
            }
        }
        // Its recast aimed: the key again calls it off (the window stays open while it lasts).
        if let Some(r) = self.recast_armed(id, base) {
            if matches!(p.action, Action::Aim { ability } if ability == r.ability) {
                if !ex {
                    self.end_ability_action(id);
                    self.set_action(id, Action::Ready);
                }
                return true;
            }
        }
        if let Action::Aim { ability } = p.action {
            if self.content.base_of(ability) == base {
                // Its key held (or Ctrl with it) while it's aimed: the aim becomes its EX
                // version's, with Valor enough; already the EX, nothing changes.
                if ex {
                    if ability == base {
                        if let Some(x) = self.content.ex_of(base) {
                            let cost = self.valor_cost(&self.content.abilities[x]);
                            if p.valor + 0.001 < cost {
                                self.events.push(Event::Flash { player: id, text: format!("{} Valor", cost.round()) });
                            } else {
                                self.set_action(id, Action::Aim { ability: x });
                            }
                        }
                    }
                    return true;
                }
                // The same key again while aiming it lets it go.
                self.end_ability_action(id);
                self.set_action(id, Action::Ready);
                return true;
            }
        }
        if !p.owned.contains(&self.content.abilities[base].key) {
            self.tell(id, format!("{} isn't yours yet.", self.content.abilities[base].name));
            return true;
        }
        // The key armed for its recast: pressed again (Ctrl or not), the recast goes, for nothing
        // (the first cast spent the cooldown). Empowered (a combo), it's the EX version, free.
        let recast = self.recast_armed(id, base).map(|r| r.ability);
        let empowered = recast.is_none() && self.empowered(id, base);
        let ability = if let Some(r) = recast {
            r
        } else if ex || empowered {
            match self.content.ex_of(base) {
                Some(x) => x,
                None => {
                    self.tell(id, format!("{} has no EX version.", self.content.abilities[base].name));
                    return true;
                }
            }
        } else {
            base
        };
        let d = self.content.abilities[ability].clone();
        if self.silenced(id) || self.immune(id) {
            // Why nothing happened, said where the hero is looking.
            let why = if self.silenced(id) { "Silenced: no abilities for now" } else { "Can't cast now" };
            self.events.push(Event::Flash { player: id, text: why.into() });
            return true;
        }
        let free = matches!(p.action, Action::Ready | Action::Guard | Action::Aim { .. } | Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. });
        if !free && !self.cancelable_now(id) {
            return false;
        }
        if recast.is_none() && !self.ready(id, ability) {
            let left = (self.cooldown(id, ability).next_at - self.clock).max(0.0);
            let text = if left > 0.05 { format!("{}: ready in {:.0} s", d.name, left.ceil()) } else { format!("{}: not ready", d.name) };
            self.events.push(Event::Flash { player: id, text });
            return true;
        }
        let cost = if recast.is_some() || empowered { 0.0 } else { self.valor_cost(&d) };
        let valor = self.players[&id].valor;
        if valor + 0.001 < cost {
            self.events.push(Event::Flash { player: id, text: format!("{} needs {} Valor (you have {})", d.name, cost.round(), valor.floor()) });
            return true;
        }
        // A weapon art takes its stamina; an Ambush, a foe to step behind.
        if d.kind == "art" {
            if self.players[&id].stamina + 0.001 < d.stamina && !self.has_cc(id, "onslaught") {
                self.winded(id);
                self.events.push(Event::Flash { player: id, text: format!("{} needs {:.0} stamina", d.name, d.stamina) });
                return true;
            }
            if d.ambush > 0.0 && self.ambush_spot(id, d.ambush).is_none() {
                self.events.push(Event::Flash { player: id, text: "No foe in reach to step behind: put the reticle on one, or lock on".into() });
                return true;
            }
        }
        self.unsheathe(id);
        self.end_ability_action(id);
        // Demonic Circle: the first press sets the anchor, costing nothing.
        if recast.is_none() && d.kind == "blink" && d.anchor > 0.0 && self.anchor_of(id, base).is_none() {
            self.set_anchor(id, base);
            return true;
        }
        if d.aimed() {
            self.set_action(id, Action::Aim { ability });
            if d.kind == "shot" {
                self.set_kit(id, Kit::Bow);
            }
        } else {
            self.fire(id, ability, 1.0);
        }
        true
    }

    /// What an ability costs in Valor (an ultimate: the whole bar).
    fn valor_cost(&self, d: &AbilityDef) -> f32 {
        if d.slot == "ultimate" {
            self.content.valor.max.min(d.valor.max(1) as f32).max(d.valor as f32)
        } else {
            d.valor as f32
        }
    }

    /// The flare a charge tops out with: the ability's own, else its school's (an EX version's
    /// is its card's).
    pub(crate) fn full_kit(&self, ability: usize) -> u32 {
        let d = &self.content.abilities[ability];
        if d.full_kit != 0 {
            return d.full_kit;
        }
        let school = if d.school.is_empty() {
            self.content.abilities.iter().find(|b| b.key == d.ex_of).map_or("", |b| b.school.as_str())
        } else {
            d.school.as_str()
        };
        crate::content_abilities::full_kit_of(school)
    }

    /// Left click while aiming: a charge begins (or a tap ability goes at once).
    pub(crate) fn ability_press(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Aim { ability }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        let (lo, hi) = self.charge_of(id, ability);
        if hi <= 0.0 {
            self.fire(id, ability, 1.0);
            return;
        }
        let d = self.content.abilities[ability].clone();
        self.set_action(id, Action::Charge { ability, since: now });
        let spell = if d.kind == "shot" { self.ranged_of(&self.players[&id]).spell } else { d.charge_spell };
        if spell != 0 {
            let ms = if d.kind == "shot" { DRAW_HOLD_MS } else { ((hi + d.hold.max(0.5)) * 1000.0) as u32 };
            self.events.push(Event::Cast { caster: id, spell, ms });
        } else if d.clip == THROW_CLIP {
            self.events.push(Event::Clip { id, clip: HOLD_THROWN, secs: hi + d.hold.max(0.5) });
        }
        let _ = lo;
    }

    /// Left click let go: a charge goes (at what it's reached), or waits for its least.
    pub(crate) fn ability_release(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Charge { ability, since }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        let (lo, hi) = self.charge_of(id, ability);
        let held = now - since;
        if held + 1.0e-4 < lo {
            // A fixed charge (Pyroblast) let go early is dropped; one with a range goes at its
            // least, as soon as it's reached.
            if (hi - lo).abs() < 1.0e-3 {
                self.tell(id, "Let go too soon: the charge is lost.");
                self.end_ability_action(id);
                self.set_action(id, Action::Ready);
            } else if let Some(p) = self.players.get_mut(&id) {
                p.release_at_least = true;
            }
            return;
        }
        let share = if hi > lo { ((held - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 1.0 };
        self.fire(id, ability, share);
    }

    /// An aim or a charge called off (right click, a roll, the key again): no cooldown spent.
    pub(crate) fn ability_cancel(&mut self, id: Id) {
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Aim { .. } | Action::Charge { .. })) {
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
        }
    }

    /// An ability's charge (least, full), the bow's for a shot.
    fn charge_of(&self, id: Id, ability: usize) -> (f32, f32) {
        let d = &self.content.abilities[ability];
        if d.kind == "shot" {
            let r = self.ranged_of(&self.players[&id]);
            if r.draw_secs <= 0.0 {
                return (0.0, 0.0);
            }
            return (r.min_draw.max(d.charge[0]), r.draw_secs);
        }
        (d.charge[0], d.charge[1])
    }

    /// The end of whatever ability action is in hand: its looks off (a charge's glow, a channel),
    /// the bow put away.
    pub(crate) fn end_ability_action(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        match p.action.clone() {
            Action::Charge { ability, .. } => {
                self.events.push(Event::CastEnd { caster: id });
                // A thrown charge let go of without a throw: the drawn-back arm comes down.
                let d = &self.content.abilities[ability];
                if d.charge_spell == 0 && d.clip == THROW_CLIP && d.kind != "shot" {
                    self.events.push(Event::ClipStop { id });
                }
            }
            Action::Channel { .. } => {
                self.events.push(Event::CastEnd { caster: id });
                self.events.push(Event::Auras { id });
                self.events.push(Event::Channel { caster: id, target: None, spell: 0, ms: 0 });
            }
            Action::Stance { .. } => {
                self.end_stance(id);
                return;
            }
            _ => {}
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.release_at_least = false;
        }
        if self.players.get(&id).is_some_and(|p| p.kit == Kit::Bow) {
            self.set_kit(id, Kit::Melee);
        }
    }

    pub(crate) fn set_kit(&mut self, id: Id, kit: Kit) {
        if let Some(p) = self.players.get_mut(&id) {
            if p.kit != kit {
                p.kit = kit;
                self.events.push(Event::Gear { player: id });
            }
        }
    }

    /// Weapons out (a key that fights draws them).
    pub(crate) fn unsheathe(&mut self, id: Id) {
        if let Some(p) = self.players.get_mut(&id) {
            if p.sheathed {
                p.sheathed = false;
                self.events.push(Event::Gear { player: id });
            }
        }
    }

    /// Late enough in a swing's recovery to cut it short for a cast.
    fn cancelable_now(&self, id: Id) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        let c = &self.content.combat;
        match &p.action {
            Action::Swing { t, strike, .. } => *t >= strike.windup + strike.active + strike.recovery * if strike.heavy() { c.heavy_cancel_after } else { c.cancel_after },
            Action::Cast { t, total, .. } => *t >= total * 0.6,
            _ => false,
        }
    }

    // --- letting it go -------------------------------------------------------------------------------

    /// An ability goes, at `share` of its full charge.
    pub(crate) fn fire(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let began = self.players[&id].pos;
        // A recast, or an empowered EX version: free (what made it so is used up).
        let free = self.take_free(id, ability);
        // The recast an echo waits for: back to the echo first (Rewind), then it goes.
        if matches!(free, Some(Free::Recast { .. })) {
            self.rewind_recast(id, ability);
        }
        // A cast brings them out of a veil.
        self.end_veil(id);
        let cost = if free.is_some() { 0.0 } else { self.valor_cost(&d) };
        if let Some(p) = self.players.get_mut(&id) {
            p.valor = (p.valor - cost).max(0.0);
            p.release_at_least = false;
            p.combat_at = now;
            p.last_cast = Some((ability, now));
        }
        if cost > 0.0 {
            self.events.push(Event::Valor { player: id });
        }
        // A charge's glow ends as it goes (the bow's draw ends with its shot).
        if matches!(self.players[&id].action, Action::Charge { .. }) {
            self.events.push(Event::CastDone { caster: id, spell: 0 });
        }
        if d.counter.is_none() && !matches!(free, Some(Free::Recast { .. })) {
            self.spend_charge(id, ability, 1.0);
        }
        let (yaw, _) = self.aim_of(id);
        if let Some(p) = self.players.get_mut(&id) {
            p.facing = yaw;
        }
        if d.clip != 0 && d.kind != "art" {
            // A stance holds its clip while it lasts (Feign Death lies there).
            let secs = if d.kind == "stance" { d.secs } else { CAST_RECOVERY + 0.2 };
            self.events.push(Event::Clip { id, clip: d.clip, secs });
        }
        // The bow's own release before the sword comes back (a crossbow's long reload).
        let total = if d.kind == "shot" { self.ranged_of(&self.players[&id]).recovery.max(CAST_RECOVERY) } else { CAST_RECOVERY };
        let next = Action::Cast { t: 0.0, total, ability };
        match d.kind.as_str() {
            "projectile" | "shot" => {
                self.launch(id, ability, share);
                self.set_action(id, next);
            }
            "ground" => {
                let at = if d.at_self {
                    self.players[&id].pos
                } else {
                    let range = AbilityDef::at(d.range, share);
                    self.aim_point(id, range)
                };
                self.ground_cast(id, ability, at, share);
                self.set_action(id, next);
            }
            "art" => {
                if d.ambush > 0.0 {
                    match self.ambush_spot(id, d.ambush) {
                        Some((spot, f)) => {
                            let from = self.players[&id].pos;
                            // A puff of smoke where they were and where they come out, so the
                            // step reads as one (a blink, not a teleport).
                            self.events.push(Event::Missile { caster: id, spell: 1856, target: Target::Point(from) });
                            self.set_down(id, spot, f);
                            self.events.push(Event::Missile { caster: id, spell: 1856, target: Target::Point(spot) });
                        }
                        None => {
                            self.set_action(id, Action::Ready);
                            return;
                        }
                    }
                }
                self.start_art(id, ability);
            }
            "buff" => {
                self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
                // Lay on Hands: what's on them ends.
                if d.cleanse {
                    self.cleanse(id, &crate::content::HARMFUL);
                }
                self.apply_effects(id, id, &d.effects, 1.0, &d.name);
                self.set_action(id, next);
            }
            "place" => {
                let range = AbilityDef::at(d.range, share);
                let at = self.aim_point(id, range);
                self.place_totem(id, ability, at);
                self.set_action(id, next);
            }
            "cone" | "nova" if d.cast > 0.0 => self.start_windup(id, ability, yaw, share),
            "cone" => {
                self.cone(id, ability, yaw, share);
                self.set_action(id, next);
            }
            "nova" => {
                let at = self.players[&id].pos;
                self.nova(id, ability, at, share);
                self.set_action(id, next);
            }
            "line" => {
                if !self.start_line(id, ability, yaw, share) {
                    self.set_action(id, next);
                }
            }
            "leap" => match free {
                // A leap back to where the cast before it began.
                Some(Free::Recast { start }) if d.recast_to_start => self.leap_to(id, ability, share, Some(start)),
                _ => self.start_leap(id, ability, share),
            },
            "trap" => {
                self.place_trap(id, ability, share);
                self.set_action(id, next);
            }
            "channel" => self.start_channel(id, ability, share),
            "beam" => self.start_beam(id, ability, share),
            "ally" => {
                self.ally(id, ability, share);
                self.set_action(id, next);
            }
            "dash" => self.start_dash(id, ability, yaw, share),
            "blink" if d.anchor > 0.0 => {
                self.return_to_anchor(id, ability);
                self.set_action(id, next);
            }
            "blink" => {
                match free {
                    // A blink back to where the cast before it began (there and back).
                    Some(Free::Recast { start }) if d.recast_to_start => self.step_back_to(id, ability, start),
                    _ => self.blink(id, ability, share),
                }
                self.set_action(id, next);
            }
            "stance" => self.start_stance(id, ability),
            _ => self.set_action(id, next),
        }
        // What it does to the caster as it goes, and what goes with it.
        if !d.self_effects.is_empty() {
            self.apply_effects(id, id, &d.self_effects, share, &d.name);
        }
        if let Some(also) = self.content.ability(&d.also) {
            self.fire_also(id, also, yaw, share);
        }
        // An echo left where it began (Rewind); its key armed for its recast, the card it empowers
        // empowered.
        self.leave_echo(id, ability, began);
        self.arm_combos(id, ability, began, free);
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Log(format!("{} casts {} at {:.0}%", self.players[&id].name, d.name, share * 100.0)));
    }

    /// The projectiles of an ability (or of the bow) leave the hands, along the reticle's line to
    /// what it meets.
    fn launch(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let (yaw, _) = self.aim_of(id);
        let (o, dir) = self.reticle(id);
        let (speed, range, radius, damage, spell) = if d.kind == "shot" {
            let r = self.ranged_of(&self.players[&id]).clone();
            let dmg = r.damage[0] as f32 + (r.damage[1] as f32 - r.damage[0] as f32) * share;
            (r.arrow_speed.max(d.speed), r.range, r.hit_radius, dmg, r.spell)
        } else {
            // Full: the faster missile, when it has one.
            let full = AbilityDef::full(share) && d.full_spell != 0;
            let (speed, spell) = if full { (d.full_speed.max(d.speed), d.full_spell) } else { (d.speed, d.spell) };
            (speed, AbilityDef::at(d.range, share), AbilityDef::at(d.radius, share), AbilityDef::at(d.damage, share), spell)
        };
        // Where the reticle points: the projectile flies from the hands to there, and on.
        let aim = self.trace(o, dir, range + 4.0, 0.0, id);
        let from = self.hands(id, yaw);
        let to_aim = norm([aim.point[0] - from[0], aim.point[1] - from[1], aim.point[2] - from[2]]);
        let base = if dist2d(from, aim.point) < 2.0 { dir } else { to_aim };
        // A thrown axe that comes back skims out level at hand height: aimed down at the ground
        // it would bury itself a few yards out and turn there.
        let base = if d.returns && base[2] < 0.0 { norm([base[0], base[1], 0.0]) } else { base };
        let n = d.count.max(1);
        // A lob arcs to the ground at the reticle (within its range), over whatever is between.
        let lob_at = d.lob.then(|| self.aim_point(id, range));
        let feet = self.players[&id].pos;
        for k in 0..n {
            let off = (k as f32 - (n - 1) as f32 / 2.0) * d.spread.to_radians();
            let (c, s) = (off.cos(), off.sin());
            let flat = (base[0] * base[0] + base[1] * base[1]).sqrt();
            let b = [base[0] * c - base[1] * s, base[0] * s + base[1] * c, base[2]];
            let dir = if flat > 1.0e-4 { norm(b) } else { base };
            if let Some(spot) = lob_at {
                // A fan of lobs lands its spread round the caster.
                let (dx, dy) = (spot[0] - feet[0], spot[1] - feet[1]);
                let at = self.ground_near(feet[0] + dx * c - dy * s, feet[1] + dx * s + dy * c, spot[2] + 1.0);
                self.lob(id, ability, from, at, (speed, radius, share, damage, spell));
                continue;
            }
            // The look: flown to where it would end if nothing moved, never after a unit (a
            // boomerang's to where it turns, as it flies on through bodies).
            let end = if d.returns { self.wall_trace(from, dir, range, id).point } else { self.trace_with(from, dir, range, radius, id, d.friends()).point };
            self.events.push(Event::Missile { caster: id, spell, target: Target::Point(end) });
            let born = self.clock;
            self.projectiles.push(Projectile {
                owner: id,
                ability,
                pos: from,
                dir,
                speed,
                radius,
                left: range,
                share,
                damage,
                spell,
                allies: d.friends(),
                hits: Vec::new(),
                // A full draw can go through more (Aimed Shot's).
                pierce: if AbilityDef::full(share) { d.pierce.max(d.full_pierce) } else { d.pierce },
                stage: Stage::Out,
                born,
            });
        }
        if self.debug {
            let end = add(from, base, range);
            self.mark(end, radius.max(0.3), range / speed.max(1.0));
        }
    }

    /// Projectiles fly on, and strike what they touch (a piercing one goes on through; a bursting
    /// one bursts where it hits); a foe's grounding totem swallows one flying past.
    pub(crate) fn step_projectiles(&mut self, dt: f32) {
        let list = std::mem::take(&mut self.projectiles);
        let mut keep = Vec::new();
        for mut p in list {
            // Coming back, arcing over, or stuck: each goes its own way.
            let going = match p.stage {
                Stage::Out => true,
                Stage::Back { .. } => self.fly_back(&mut p, dt),
                Stage::Lob { .. } => !self.lob_lands(&p),
                Stage::Stuck { .. } => self.fuse_burns(&mut p),
            };
            if p.stage != Stage::Out {
                if going {
                    keep.push(p);
                }
                continue;
            }
            if self.grounded(&p) {
                continue;
            }
            let (returns, sticks) = {
                let d = &self.content.abilities[p.ability];
                (d.returns, d.fuse > 0.0)
            };
            let go = (p.speed * dt).min(p.left);
            let t = self.trace_skipping(p.pos, p.dir, go, p.radius, p.owner, p.allies, &p.hits);
            if let Some(v) = t.unit {
                let from = p.pos;
                p.pos = t.point;
                // A mirror's stance turns a foe's projectile back at whoever threw it.
                if self.mirrors(&mut p, v) {
                    keep.push(p);
                    continue;
                }
                // A sticky one sticks to what it strikes (a friend it's for is met as ever).
                let friend = p.allies && self.players.contains_key(&v) && !self.hostile(p.owner, v);
                if sticks && !friend {
                    self.stick(&mut p, Some(v));
                    keep.push(p);
                    continue;
                }
                self.projectile_hit(&p, v, from);
                // A hook yanks its caster to it, an orb swaps them, a chain binds the foe.
                self.beta_hit(&p, Some(v), t.point);
                if returns {
                    // A boomerang goes on through everything it meets, each once a way.
                    p.hits.push(v);
                    p.left -= t.dist;
                    if p.left <= 0.01 {
                        self.turn_back(&mut p);
                    }
                    keep.push(p);
                    continue;
                }
                if p.pierce > 0 {
                    p.pierce -= 1;
                    p.hits.push(v);
                    p.left -= t.dist;
                    if p.left > 0.01 {
                        keep.push(p);
                    }
                }
                continue;
            }
            if t.solid {
                // A boomerang turns at a wall; a sticky one sits on it (or on the ground).
                if returns || sticks {
                    p.pos = t.point;
                    if returns {
                        // Off the wall a little first, so its way back doesn't start in it.
                        p.pos = add(t.point, p.dir, -0.6);
                        self.turn_back(&mut p);
                    } else {
                        self.stick(&mut p, None);
                    }
                    keep.push(p);
                    continue;
                }
                // A hook that meets a wall or the ground yanks its caster there.
                self.beta_hit(&p, None, t.point);
                let d = &self.content.abilities[p.ability];
                let (kit, splash) = (d.impact_kit, d.splash);
                if kit != 0 && self.debug {
                    self.mark(t.point, 0.4, 0.6);
                }
                if splash > 0.0 {
                    self.burst(&p, t.point, None);
                }
                continue;
            }
            // `.es debug`: the projectile's body drawn every two yards of its flight.
            if self.debug && (p.left / 2.0).floor() != ((p.left - go) / 2.0).floor() {
                self.mark(t.point, p.radius.max(0.2), 0.4);
            }
            p.pos = t.point;
            p.left -= go;
            if p.left > 0.01 {
                keep.push(p);
            } else if returns {
                self.turn_back(&mut p);
                keep.push(p);
            } else if sticks {
                // Its range spent in the air: it drops there and sits on the ground.
                p.pos = self.ground_near(p.pos[0], p.pos[1], p.pos[2]);
                self.stick(&mut p, None);
                keep.push(p);
            }
        }
        keep.extend(std::mem::take(&mut self.projectiles));
        self.projectiles = keep;
    }

    /// The line `from` along `dir` for `range` to the first ground or wall, through every body.
    fn wall_trace(&self, from: Vec3, dir: Vec3, range: f32, owner: Id) -> Trace {
        let all: Vec<Id> = self.bodies_with(owner, true).into_iter().map(|b| b.0).collect();
        if all.is_empty() {
            return self.trace(from, dir, range, 0.0, owner);
        }
        self.trace_skipping(from, dir, range, 0.0, owner, true, &all)
    }

    /// A boomerang at its turn: its look flown back from there to its thrower (by an unseen
    /// anchor standing there), and a fresh pass (what it struck going out it can strike again),
    /// at its way back's share of the damage.
    fn turn_back(&mut self, p: &mut Projectile) {
        let share = self.content.abilities[p.ability].return_damage;
        // It turns at hand height over the ground there: thrown high, it doesn't come back over
        // everyone's heads.
        let floor = self.ground_near(p.pos[0], p.pos[1], p.pos[2]);
        p.pos[2] = p.pos[2].min(floor[2] + HAND_UP);
        let anchor = self.alloc();
        self.events.push(Event::MissileFrom { anchor, caster: p.owner, spell: p.spell, from: p.pos, target: Target::Unit(p.owner) });
        p.hits.clear();
        p.damage *= share;
        p.stage = Stage::Back { anchor };
    }

    /// A boomerang flies back toward where its thrower is now, striking each foe it passes once.
    /// It ends in their hands, at a wall, swallowed by a grounding totem, or when its time is up
    /// (false: it's done, and its look's anchor goes).
    fn fly_back(&mut self, p: &mut Projectile, dt: f32) -> bool {
        let Stage::Back { anchor } = p.stage else { return false };
        let range = AbilityDef::at(self.content.abilities[p.ability].range, p.share);
        let late = self.clock - p.born > 2.0 * range / p.speed.max(1.0) + 2.0;
        let home = self.unit_pos(p.owner).map(|o| [o[0], o[1], o[2] + HAND_UP]);
        let to = home.map(|h| sub(h, p.pos)).filter(|_| !late);
        let to = if to.is_some() && self.grounded(p) { None } else { to };
        let gone = match to {
            Some(to) => {
                let dist = len(to);
                let step = (p.speed * dt).min(dist);
                p.dir = norm(to);
                let t = self.trace_skipping(p.pos, p.dir, step, p.radius, p.owner, p.allies, &p.hits);
                if let Some(v) = t.unit {
                    let from = p.pos;
                    p.pos = t.point;
                    self.projectile_hit(p, v, from);
                    p.hits.push(v);
                    false
                } else if t.solid {
                    if self.content.abilities[p.ability].splash > 0.0 {
                        self.burst(p, t.point, None);
                    }
                    true
                } else {
                    p.pos = t.point;
                    // Caught.
                    dist - step <= HERO_SIZE
                }
            }
            _ => true,
        };
        if gone {
            self.events.push(Event::ObjGone { id: anchor });
        }
        !gone
    }

    /// A lob's flight: its look flown to `at`, the ring everyone sees there until it lands
    /// (distance / speed from now), and a warning for the foes near it. `(speed, radius, share,
    /// damage, spell)` as [`Self::launch`] reckoned them.
    fn lob(&mut self, id: Id, ability: usize, from: Vec3, at: Vec3, (speed, radius, share, damage, spell): (f32, f32, f32, f32, u32)) {
        let splash = self.content.abilities[ability].splash;
        let dist = len(sub(at, from));
        let flight = dist / speed.max(1.0);
        let tele = Telegraph::hero_shape(id, splash, 360.0, 0.0, flight, Some((at, 0.0)));
        let mark = Some(self.mark_as(at, splash, flight + 0.1, Some(tele)));
        let foes: Vec<Id> = self.players.iter().filter(|(h, x)| self.hostile(id, **h) && x.health > 0 && dist2d(x.pos, at) <= splash + 4.0).map(|(h, _)| *h).collect();
        for h in foes {
            self.events.push(Event::Danger { player: h, secs: flight, from: None });
        }
        self.events.push(Event::Missile { caster: id, spell, target: Target::Point(at) });
        let (dir, land_at, born) = (norm(sub(at, from)), self.clock + flight, self.clock);
        self.projectiles.push(Projectile {
            owner: id,
            ability,
            pos: from,
            dir,
            speed,
            radius,
            left: dist,
            share,
            damage,
            spell,
            allies: false,
            hits: Vec::new(),
            pierce: 0,
            stage: Stage::Lob { at, land_at, mark },
            born,
        });
    }

    /// A lob lands when its flight is over (true): its ring gone, it bursts there, showing its
    /// `burst_spell` at the spot (none: the missile's own impact is the look).
    fn lob_lands(&mut self, p: &Projectile) -> bool {
        let Stage::Lob { at, land_at, mark } = p.stage else { return true };
        if self.clock < land_at {
            return false;
        }
        if let Some(m) = mark {
            self.unmark(m);
        }
        let look = self.content.abilities[p.ability].burst_spell;
        self.burst_look(p, at, (look != 0).then_some(look));
        true
    }

    /// A sticky projectile sticks to the unit it struck (and follows it), or where it is (the
    /// ground, a wall), until its fuse runs out; the unit shows its aura meanwhile.
    fn stick(&mut self, p: &mut Projectile, on: Option<Id>) {
        let fuse = self.content.abilities[p.ability].fuse;
        let at = on.and_then(|u| self.unit_pos(u)).unwrap_or(p.pos);
        p.stage = Stage::Stuck { on, at, until: self.clock + fuse, mark: None };
        if let Some(u) = on {
            self.events.push(Event::Auras { id: u });
        }
    }

    /// A stuck fuse burns down (false once it's gone off): it follows its unit while that lives
    /// (and stays where it fell), a ring round it for everyone for its last second, then it bursts
    /// there, showing its `burst_spell` at the spot (none: its own spell, as any burst).
    fn fuse_burns(&mut self, p: &mut Projectile) -> bool {
        let Stage::Stuck { on, mut at, until, mut mark } = p.stage else { return false };
        let now = self.clock;
        let alive = on.filter(|u| self.players.get(u).map(|x| x.health > 0).or_else(|| self.mobs.get(u).map(|m| m.health > 0)).unwrap_or(false));
        if let Some(pos) = alive.and_then(|u| self.unit_pos(u)) {
            at = pos;
        }
        let splash = self.content.abilities[p.ability].splash;
        if mark.is_none() && until - now <= 1.0 {
            let secs = (until - now).max(0.0);
            // Under a living unit the ring goes with them (a warning to all); else it's laid where
            // it lies, as the caster's ground.
            let tele = match alive {
                Some(u) => Telegraph { mob: u, reach: splash, arc: 360.0, secs, blockable: false, foe: self.players.contains_key(&u).then_some(u), width: 0.0, at: None, hero: false },
                None => Telegraph::hero_shape(p.owner, splash, 360.0, 0.0, secs, Some((at, 0.0))),
            };
            mark = Some(self.mark_as(at, splash, secs + 0.1, Some(tele)));
            let owner = p.owner;
            let foes: Vec<Id> = self.players.iter().filter(|(h, x)| self.hostile(owner, **h) && x.health > 0 && dist2d(x.pos, at) <= splash + 3.0).map(|(h, _)| *h).collect();
            for h in foes {
                self.events.push(Event::Danger { player: h, secs, from: None });
            }
        }
        p.pos = at;
        if now < until {
            p.stage = Stage::Stuck { on, at, until, mark };
            return true;
        }
        if let Some(m) = mark {
            self.unmark(m);
        }
        // Gone off: off them too (it's no longer among the projectiles once this step is done).
        if let Some(u) = on {
            self.events.push(Event::Auras { id: u });
        }
        let d = &self.content.abilities[p.ability];
        let look = if d.burst_spell != 0 { d.burst_spell } else { d.spell };
        self.burst_look(p, at, Some(look));
        false
    }

    /// The looks of the sticky projectiles stuck on a unit (their `fuse_aura`s).
    pub(crate) fn fuse_auras(&self, id: Id) -> Vec<u32> {
        self.projectiles
            .iter()
            .filter(|p| matches!(p.stage, Stage::Stuck { on: Some(u), .. } if u == id))
            .map(|p| self.content.abilities[p.ability].fuse_aura)
            .filter(|a| *a != 0)
            .collect()
    }

    /// Everything in flight gone (the world cleared): the looks' anchors with it.
    pub(crate) fn drop_projectiles(&mut self) {
        for p in std::mem::take(&mut self.projectiles) {
            if let Stage::Back { anchor } = p.stage {
                self.events.push(Event::ObjGone { id: anchor });
            }
        }
    }

    /// [`Self::trace_with`], past the bodies in `skip` too.
    #[allow(clippy::too_many_arguments)]
    fn trace_skipping(&self, from: Vec3, dir: Vec3, range: f32, radius: f32, owner: Id, allies: bool, skip: &[Id]) -> Trace {
        if skip.is_empty() {
            return self.trace_with(from, dir, range, radius, owner, allies);
        }
        let bodies: Vec<_> = self.bodies_with(owner, allies).into_iter().filter(|b| !skip.contains(&b.0)).collect();
        let mut d = 0.0f32;
        while d < range {
            let next = (d + STEP).min(range);
            let q = add(from, dir, next);
            for (id, feet, tall, size) in &bodies {
                if to_body(q, *feet, *tall) <= size + radius {
                    return Trace { dist: next, point: q, unit: Some(*id), solid: false };
                }
            }
            let ground = self.land.height_near(q[0], q[1], q[2] + 0.6);
            if q[2] < ground - 0.05 || self.land.blocked_near(q[0], q[1], q[2]) {
                return Trace { dist: d, point: add(from, dir, d), unit: None, solid: true };
            }
            d = next;
        }
        Trace { dist: range, point: add(from, dir, range), unit: None, solid: false }
    }

    /// A bursting projectile goes off at `at`: everyone in its burst struck (and its effects from
    /// there).
    fn burst(&mut self, p: &Projectile, at: Vec3, _direct: Option<Id>) {
        let spell = self.content.abilities[p.ability].spell;
        self.burst_look(p, at, Some(spell));
    }

    /// [`Self::burst`], showing `look` at the spot (none: nothing of its own).
    fn burst_look(&mut self, p: &Projectile, at: Vec3, look: Option<u32>) {
        let d = self.content.abilities[p.ability].clone();
        let ground = self.ground_near(at[0], at[1], at[2] + 0.5);
        let at = if (at[2] - ground[2]).abs() < 2.5 { ground } else { at };
        if let Some(spell) = look {
            self.events.push(Event::Missile { caster: p.owner, spell, target: Target::Point(at) });
        }
        self.area(p.owner, p.ability, at, d.splash, p.share, p.damage, None);
        self.linger_at(p.owner, p.ability, at, d.splash);
    }

    /// A hit that leaps on (Chain Lightning): to the nearest other foe each time, weaker each.
    fn chain_on(&mut self, p: &Projectile, first: Id) {
        let d = self.content.abilities[p.ability].clone();
        let mut hit = vec![first];
        let mut from = first;
        let mut damage = p.damage;
        for _ in 0..d.chain {
            let Some(at) = self.unit_pos(from) else { break };
            let range = if d.chain_range > 0.0 { d.chain_range } else { 8.0 };
            let next = self
                .mobs
                .iter()
                .filter(|(m, x)| x.health > 0 && !x.hidden && !hit.contains(m) && !self.content.mobs[x.def].inert() && dist2d(x.pos, at) <= range)
                .map(|(m, x)| (*m, dist2d(x.pos, at)))
                .chain(self.players.iter().filter(|(h, x)| self.hostile(p.owner, **h) && x.health > 0 && !hit.contains(h) && dist2d(x.pos, at) <= range).map(|(h, x)| (*h, dist2d(x.pos, at))))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(u, _)| u);
            let Some(v) = next else { break };
            damage *= d.chain_falloff;
            self.events.push(Event::Missile { caster: from, spell: d.spell, target: Target::Unit(v) });
            let jump = Projectile { damage, pos: at, ..p.clone() };
            self.strike_with(&jump, v, at);
            hit.push(v);
            from = v;
        }
    }

    /// A projectile reaches someone: a friend it's for is healed (and moved, with a pull); a foe
    /// is struck, or caught in its burst; a chaining one leaps on.
    fn projectile_hit(&mut self, p: &Projectile, victim: Id, from: Vec3) {
        // A friend touched by light: healed, not struck.
        // (Outside Siege that's any other hero, friendly fire or not: a bolt that would both heal
        // and hurt them heals them, as the caster meant.)
        if p.allies && self.players.contains_key(&victim) && self.helpable(p.owner, victim) {
            self.friend_hit(p, victim);
            return;
        }
        let d = self.content.abilities[p.ability].clone();
        if d.splash > 0.0 {
            self.burst(p, p.pos, Some(victim));
            return;
        }
        self.strike_with(p, victim, from);
        if d.chain > 0 {
            self.chain_on(p, victim);
        }
    }

    /// A projectile's blow on one foe.
    fn strike_with(&mut self, p: &Projectile, victim: Id, from: Vec3) {
        // Fire Ward's own Fire Blasts and the like are owned by whoever fired them back.
        if self.players.contains_key(&victim) {
            let damage = self.ability_damage(p.owner, victim, p.ability, p.damage);
            let blow = Blow::Arrow(p.spell);
            self.strike_fair_as(p.owner, victim, damage, from, true, false, false, blow, None, combat::Follow::Ability { ability: p.ability, share: p.share }, HitKind::Projectile);
        } else {
            self.ability_on_mob(p.owner, victim, p.ability, p.share, p.damage, from, HitKind::Projectile, p.spell);
        }
    }

    /// A projectile meant for friends too reaches one: its heal, and what helps them or moves
    /// them (a pull to the caster) of its effects.
    fn friend_hit(&mut self, p: &Projectile, friend: Id) {
        let d = self.content.abilities[p.ability].clone();
        if d.ally_heal > 0.0 {
            self.heal_friend(p.owner, friend, p.ability);
        } else if d.cleanse {
            self.cleanse(friend, &crate::content::HARMFUL);
        }
        let kind = |e: &&crate::content::AbilityEffect| crate::content::BOONS.contains(&e.kind.as_str()) || crate::content::DISPLACE.contains(&e.kind.as_str());
        let help: Vec<_> = d.effects.iter().filter(kind).cloned().collect();
        if !help.is_empty() {
            let was = self.fx_from.replace(p.pos);
            self.apply_effects(p.owner, friend, &help, p.share, &d.name);
            self.fx_from = was;
        }
        if d.impact_kit != 0 {
            self.events.push(Event::Visual { id: friend, kit: d.impact_kit, by: None });
        }
    }

    /// An ability's damage on someone: its own, the caster's might and weakness, a mark on the
    /// target used up, and an execute.
    pub(crate) fn ability_damage(&mut self, owner: Id, victim: Id, ability: usize, base: f32) -> u32 {
        let d = &self.content.abilities[ability];
        let might = self.players.get(&owner).map_or(1.0, |p| 1.0 + self.effect_of(p, "might"));
        let execute = d.execute;
        let mut dmg = base * might * self.weakened(owner);
        let shatter = d.shatter;
        let mark_crit = d.mark_crit;
        let mark = self.take_mark(victim);
        dmg *= 1.0 + mark;
        if mark > 0.0 && mark_crit > 1.0 {
            dmg *= mark_crit;
            self.events.push(Event::Flash { player: owner, text: "Critical!".into() });
        }
        // Fire, or a sundering blow, on a frozen foe: the ice shatters.
        if shatter && self.has_cc(victim, "petrify") {
            dmg *= SHATTER;
            self.cleanse(victim, &["petrify"]);
            self.events.push(Event::Flash { player: owner, text: "Shatter!".into() });
        }
        // A foe reeling from a parry takes it as a riposte.
        dmg = if self.players.contains_key(&victim) {
            self.hero_riposted(victim, dmg.round() as u32) as f32
        } else {
            self.riposted(victim, dmg.round() as u32) as f32
        };
        if execute > 0.0 {
            let (h, max) = self
                .players
                .get(&victim)
                .map(|p| (p.health, p.max_health))
                .or_else(|| self.mobs.get(&victim).map(|m| (m.health, m.max_health)))
                .unwrap_or((1, 1));
            if (h as f32) < execute * max as f32 {
                self.events.push(Event::Flash { player: owner, text: "Execute!".into() });
                dmg = dmg.max(h as f32);
            }
        }
        dmg.round().max(0.0) as u32
    }

    /// An ability lands on a creature (a totem, a dummy): damage, its effects, the look.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn ability_on_mob(&mut self, owner: Id, mid: Id, ability: usize, share: f32, base: f32, from: Vec3, how: HitKind, spell: u32) {
        let now = self.clock;
        if !self.mobs.get(&mid).is_some_and(|m| m.health > 0) {
            return;
        }
        if self.players.contains_key(&owner) && self.evading(mid, owner) {
            self.events.push(Event::Swing { attacker: owner, victim: Some(mid), damage: 0, outcome: Outcome::Miss, anim: false, style: Blow::Arrow(spell) });
            self.events.push(Event::Flash { player: owner, text: "Evade".into() });
            return;
        }
        // A dummy in a counter's stance.
        if how != HitKind::Area {
            if let Some(out) = self.counter_catch(mid, owner, how, Some((ability, share))) {
                if out != Outcome::Hit {
                    self.events.push(Event::Swing { attacker: owner, victim: Some(mid), damage: 0, outcome: out, anim: false, style: Blow::Arrow(spell) });
                    return;
                }
            }
            if self.dummy_rolls(mid) {
                self.events.push(Event::Swing { attacker: owner, victim: Some(mid), damage: 0, outcome: Outcome::Dodged, anim: false, style: Blow::Arrow(spell) });
                return;
            }
            // A raised shield stops a projectile that hits it from the front.
            if how == HitKind::Projectile && self.dummy_guards(mid, from) {
                self.events.push(Event::Swing { attacker: owner, victim: Some(mid), damage: 0, outcome: Outcome::Blocked, anim: false, style: Blow::Arrow(spell) });
                self.events.push(Event::React { id: mid, anim: combat::BLOCK_ANIM, from, lean: 0.1, hold: 0.0, fit: 0.0 });
                return;
            }
        }
        let damage = self.ability_damage(owner, mid, ability, base);
        let damage = (damage as f32 * self.taken_share(mid)).round() as u32;
        let style = if self.content.abilities[ability].kind == "art" { Blow::Heavy } else { Blow::Arrow(spell) };
        self.events.push(Event::Swing { attacker: owner, victim: Some(mid), damage, outcome: Outcome::Hit, anim: false, style });
        let kit = self.content.abilities[ability].impact_kit;
        if kit != 0 {
            self.events.push(Event::Visual { id: mid, kit, by: None });
        }
        self.trade_blows(owner, mid);
        if damage > 0 {
            let name = self.content.abilities[ability].name.clone();
            self.hurt_mob_as(mid, owner, damage, &name);
        }
        let killed = !self.mobs.get(&mid).is_some_and(|m| m.health > 0);
        let was = self.fx_from.replace(from);
        self.ability_after(owner, mid, ability, share, damage, killed);
        self.fx_from = was;
        if !killed && self.mobs.get(&mid).is_some_and(|m| m.stagger_until <= now) {
            let lean = self.content.combat.lean_light * 0.6;
            self.events.push(Event::React { id: mid, anim: combat::WOUND_ANIM, from, lean, hold: 0.0, fit: 0.0 });
        }
    }

    /// What follows an ability that landed: its effects on the struck, the drain and heal for the
    /// caster, Valor, the refund on a kill.
    pub(crate) fn ability_after(&mut self, owner: Id, victim: Id, ability: usize, share: f32, damage: u32, killed: bool) {
        self.ability_after_at(owner, victim, ability, share, damage, killed, 1.0);
    }

    /// [`Self::ability_after`], with `valor` of the Valor it gives for the foe (a beam's tick: its
    /// share of a second, so a beam gives its Valor a second in it).
    #[allow(clippy::too_many_arguments)]
    fn ability_after_at(&mut self, owner: Id, victim: Id, ability: usize, share: f32, damage: u32, killed: bool, valor: f32) {
        let d = self.content.abilities[ability].clone();
        if !killed {
            // One that's for friends too gives a foe none of what helps.
            if d.friendly {
                let harm: Vec<_> = d.effects.iter().filter(|e| !crate::content::BOONS.contains(&e.kind.as_str())).cloned().collect();
                self.apply_effects(owner, victim, &harm, share, &d.name);
            } else {
                self.apply_effects(owner, victim, &d.effects, share, &d.name);
            }
        }
        if d.drain > 0.0 && damage > 0 {
            let heal = (damage as f32 * d.drain * self.weakened(owner)).round() as u32;
            self.heal_hero_by(owner, heal, owner, &d.name);
        }
        if d.heal > 0.0 {
            let heal = self.players.get(&owner).map_or(0, |p| (p.max_health as f32 * d.heal * self.weakened(owner)).round() as u32);
            self.heal_hero_by(owner, heal, owner, &d.name);
        }
        let inert = self.mobs.get(&victim).is_some_and(|m| self.content.mobs[m.def].totem);
        if !inert {
            let v = if d.valor_hit > 0.0 { d.valor_hit } else { self.content.valor.ability };
            self.gain_valor(owner, v * valor);
        }
        if killed && d.refund_kill && d.valor > 0 {
            self.gain_valor(owner, d.valor as f32);
            self.events.push(Event::Flash { player: owner, text: "Valor back".into() });
        }
    }

    /// An ability's effects on someone, at a share of the charge.
    pub(crate) fn apply_effects(&mut self, owner: Id, victim: Id, effects: &[crate::content::AbilityEffect], share: f32, source: &str) {
        for e in effects {
            let secs = AbilityDef::at(e.secs, share);
            match e.kind.as_str() {
                "heal" => {
                    let heal = self.players.get(&owner).map_or(0, |p| (p.max_health as f32 * e.amount).round() as u32);
                    self.heal_hero_by(owner, heal, owner, source);
                }
                "cleanse_panic" => self.cleanse(victim, &["panic"]),
                "knockback" | "pull" => self.displace(owner, victim, e),
                "burn" | "mark" => {
                    self.apply_cc_look(victim, owner, &e.kind, secs, e.amount, 0.0, source, e.aura);
                }
                "snare" if e.freeze_if_snared && self.has_cc(victim, "snare") => {
                    self.apply_cc(victim, owner, "petrify", secs.min(2.0).max(1.0), 0.0, 0.0, source);
                }
                kind => {
                    let took = self.apply_cc_look(victim, owner, kind, secs, e.amount, e.break_after, source, e.aura);
                    // A shield that bursts as it ends: its burst waits on whoever holds it.
                    if took && !e.pop.is_empty() {
                        self.hold_pop(victim, &e.pop);
                    }
                }
            }
        }
    }

    /// A friend reached by a healing ability (Holy Shock): its share of their health back, and
    /// what's on them ended (with `cleanse`).
    fn heal_friend(&mut self, owner: Id, friend: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(f) = self.players.get(&friend).filter(|f| f.health > 0) else { return };
        let (max, name) = (f.max_health, f.name.clone());
        let amount = (max as f32 * d.ally_heal * self.weakened(owner)).round() as u32;
        self.heal_hero_by(friend, amount, owner, &d.name);
        if d.cleanse {
            self.cleanse(friend, &crate::content::HARMFUL);
        }
        self.events.push(Event::Missile { caster: owner, spell: HEAL_LOOK, target: Target::Unit(friend) });
        self.events.push(Event::Flash { player: owner, text: format!("Healed {name}") });
        let v = if d.valor_hit > 0.0 { d.valor_hit } else { self.content.valor.ability };
        self.gain_valor(owner, v);
    }

    // --- help aimed at a friend ------------------------------------------------------------------------

    /// Who help aimed at a friend (`ally`) lands on: the first friendly hero along the reticle
    /// within `range` (traced `radius` wide), else the friend nearest the reticle's line within
    /// [`ALLY_CONE`] of it and `range` (in sight), else the caster (alone, it's their own).
    pub(crate) fn ally_target(&self, id: Id, range: f32, radius: f32) -> Id {
        let Some(me) = self.players.get(&id) else { return id };
        let pos = me.pos;
        let friends: Vec<(Id, Vec3)> = self.players.iter().filter(|(h, p)| **h != id && p.health > 0 && self.helpable(id, **h)).map(|(h, p)| (*h, p.pos)).collect();
        if friends.is_empty() {
            return id;
        }
        // Along the reticle (from the camera, so a little past the range from the hero), to the
        // ground or a wall it meets.
        let (o, dir) = self.reticle(id);
        let reach = range + 3.0;
        let mut d = 0.0f32;
        while d < reach {
            let next = (d + STEP).min(reach);
            let q = add(o, dir, next);
            if let Some((f, _)) = friends.iter().find(|(_, feet)| to_body(q, *feet, HERO_TALL) <= HERO_SIZE + radius && dist2d(*feet, pos) <= range + 1.0) {
                return *f;
            }
            let ground = self.land.height_near(q[0], q[1], q[2] + 0.6);
            if q[2] < ground - 0.05 || self.land.blocked_near(q[0], q[1], q[2]) {
                break;
            }
            d = next;
        }
        // Near it: the friend closest to the line, within the cone and the range, in sight.
        let (yaw, _) = self.aim_of(id);
        let (c, s) = (yaw.cos(), yaw.sin());
        let off = |at: Vec3| (-(at[0] - pos[0]) * s + (at[1] - pos[1]) * c).abs();
        friends
            .iter()
            .filter(|(_, at)| dist2d(pos, *at) <= range && angle_between(yaw, facing(pos, *at)) <= ALLY_CONE && clear_line(self.land.as_ref(), pos, *at))
            .min_by(|a, b| off(a.1).total_cmp(&off(b.1)))
            .map_or(id, |(f, _)| *f)
    }

    /// Help aimed at a friend goes: on whoever [`Self::ally_target`] picks, its heal, its cleanse
    /// and the boons among its effects (what would harm or move them is left out), with its look
    /// from the caster to them.
    fn ally(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let radius = AbilityDef::at(d.radius, share);
        let target = self.ally_target(id, range, if radius > 0.0 { radius } else { ALLY_TRACE });
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(target) });
        if d.impact_kit != 0 {
            self.events.push(Event::Visual { id: target, kit: d.impact_kit, by: None });
        }
        if d.ally_heal > 0.0 {
            let amount = self.players.get(&target).map_or(0, |p| (p.max_health as f32 * d.ally_heal * self.weakened(id)).round() as u32);
            self.heal_hero_by(target, amount, id, &d.name);
        }
        if d.cleanse {
            self.cleanse(target, &crate::content::HARMFUL);
        }
        let boons: Vec<_> = d.effects.iter().filter(|e| crate::content::BOONS.contains(&e.kind.as_str())).cloned().collect();
        if !boons.is_empty() {
            self.apply_effects(id, target, &boons, share, &d.name);
        }
        // On another hero: said to the caster, and Valor for it (not for helping yourself).
        if target != id {
            let name = self.players.get(&target).map_or_else(String::new, |p| p.name.clone());
            let text = if d.ally_heal > 0.0 { format!("Healed {name}") } else { format!("{} on {name}", d.name) };
            self.events.push(Event::Flash { player: id, text });
            let v = if d.valor_hit > 0.0 { d.valor_hit } else { self.content.valor.ability };
            self.gain_valor(id, v);
        }
    }

    /// Heal a hero (on nobody's meter: [`Self::heal_hero_by`] counts it); what it mended.
    pub(crate) fn heal_hero(&mut self, id: Id, amount: u32) -> u32 {
        let Some(p) = self.players.get_mut(&id) else { return 0 };
        if p.health == 0 || amount == 0 {
            return 0;
        }
        let before = p.health;
        p.health = (p.health + amount).min(p.max_health);
        let (h, m) = (p.health, p.max_health);
        if h != before {
            self.events.push(Event::Health { id, health: h, max: m });
            self.events.push(Event::Healed { id, amount: h - before });
        }
        h - before
    }

    // --- ground, totems, cones, novas ----------------------------------------------------------------

    /// Ground marked at `at` (a sky-fall's ring for everyone), landing after its delay.
    pub(crate) fn ground_at(&mut self, id: Id, ability: usize, at: Vec3, share: f32) {
        self.ground_cast(id, ability, at, share);
    }

    /// Another ability that goes with one, at once, from the same aim (a Disengage's Net).
    fn fire_also(&mut self, id: Id, ability: usize, yaw: f32, share: f32) {
        let d = self.content.abilities[ability].clone();
        let pos = self.players[&id].pos;
        match d.kind.as_str() {
            "projectile" => self.launch(id, ability, share),
            "nova" => self.nova(id, ability, pos, share),
            "cone" => self.cone_at(id, ability, pos, yaw, share),
            "line" => self.line_go(id, ability, pos, yaw, share),
            "trap" => self.place_trap(id, ability, share),
            "ground" => {
                let at = if d.at_self { pos } else { self.aim_point(id, AbilityDef::at(d.range, share)) };
                self.ground_cast(id, ability, at, share);
            }
            _ => {}
        }
    }

    /// Ground left burning (or hallowed) where an ability landed, for its `linger`.
    pub(crate) fn linger_at(&mut self, owner: Id, ability: usize, at: Vec3, radius: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        if d.linger <= 0.0 {
            return;
        }
        // Holy ground is a friend's mark (green), burning ground a warning.
        let friendly = d.linger_heal > 0.0 || !d.boon.is_empty();
        let mark = Some(self.mark_kind(at, radius, d.linger, friendly));
        let spell = if d.linger_spell != 0 { d.linger_spell } else { d.spell };
        if let (Some(m), true) = (mark, spell != 0) {
            self.events.push(Event::AreaLook { id: m, caster: owner, spell, pos: at, radius });
        }
        self.burnings.push(Burning { owner, ability, at, radius, dps: d.linger_damage, until: now + d.linger, next: now + 1.0, mark });
    }

    fn ground_cast(&mut self, id: Id, ability: usize, at: Vec3, share: f32) {
        let d = self.content.abilities[ability].clone();
        let radius = AbilityDef::at(d.radius, share);
        let land_at = self.clock + d.delay;
        // The mark everyone sees before it lands (the true area, under debug too).
        let tele = Telegraph::hero_shape(id, radius, 360.0, 0.0, d.delay, Some((at, 0.0)));
        let mark = (d.delay > 0.0).then(|| self.mark_as(at, radius, d.delay + 0.1, Some(tele)));
        for (h, p) in &self.players {
            if *h != id && p.health > 0 && dist2d(p.pos, at) <= radius + 4.0 && d.delay > 0.0 {
                self.events.push(Event::Danger { player: *h, secs: d.delay, from: None });
            }
        }
        if d.delay > 0.0 {
            self.events.push(Event::Cast { caster: id, spell: d.spell, ms: (d.delay * 1000.0) as u32 });
        }
        self.grounds.push(GroundCast { owner: id, ability, at, radius, share, land_at, mark, shown: false });
        if d.delay <= 0.0 {
            self.step_grounds();
        }
    }

    /// Ground marked for an ability lands; burning ground burns.
    pub(crate) fn step_grounds(&mut self) {
        let now = self.clock;
        // A look that takes its time to come down starts that much early.
        for k in 0..self.grounds.len() {
            let g = &self.grounds[k];
            let d = &self.content.abilities[g.ability];
            if !g.shown && d.look_lead > 0.0 && g.land_at - d.look_lead <= now {
                let target = if d.at_self { Target::Unit(g.owner) } else { Target::Point(g.at) };
                let (owner, spell) = (g.owner, d.spell);
                self.grounds[k].shown = true;
                self.events.push(Event::Missile { caster: owner, spell, target });
            }
        }
        let (due, keep): (Vec<GroundCast>, Vec<GroundCast>) = std::mem::take(&mut self.grounds).into_iter().partition(|g| g.land_at <= now);
        self.grounds = keep;
        for g in due {
            if let Some(m) = g.mark {
                self.unmark(m);
            }
            let d = self.content.abilities[g.ability].clone();
            let target = if d.at_self { Target::Unit(g.owner) } else { Target::Point(g.at) };
            if !g.shown {
                self.events.push(Event::Missile { caster: g.owner, spell: d.spell, target });
            }
            let damage = AbilityDef::at(d.damage, g.share);
            self.area(g.owner, g.ability, g.at, g.radius, g.share, damage, None);
            self.linger_at(g.owner, g.ability, g.at, g.radius);
            // What it calls down stands there (an Infernal).
            if d.totem.is_some() {
                self.place_totem(g.owner, g.ability, g.at);
            }
        }
        let (live, done): (Vec<Burning>, Vec<Burning>) = std::mem::take(&mut self.burnings).into_iter().partition(|b| b.until > now);
        for b in done {
            if let Some(m) = b.mark {
                self.unmark(m);
            }
            // Ground that bursts as it ends (a black hole's collapse).
            self.end_burst(&b);
        }
        self.burnings = live;
        for k in 0..self.burnings.len() {
            if self.burnings[k].next > now {
                continue;
            }
            self.burnings[k].next += 1.0;
            let b = self.burnings[k].clone();
            let tick = self.content.abilities[b.ability].tick_spell;
            if tick != 0 {
                self.events.push(Event::Missile { caster: b.owner, spell: tick, target: Target::Point(b.at) });
            }
            self.area_tick(b.owner, b.ability, b.at, b.radius, b.dps);
        }
    }

    /// Everyone in a circle (or, with `cone`, in front within its arc) struck by an ability.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn area(&mut self, owner: Id, ability: usize, at: Vec3, radius: f32, share: f32, damage: f32, cone: Option<(f32, f32)>) {
        let shape = match cone {
            Some((f, arc)) => Shape::Cone(f, arc),
            None => Shape::Circle,
        };
        self.area_shape(owner, ability, at, radius, share, damage, shape);
    }

    /// Everyone in an ability's ground (a circle, a wedge, a strip) struck by it; its effects
    /// come from `at` (a knockback throws them away from there).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn area_shape(&mut self, owner: Id, ability: usize, at: Vec3, radius: f32, share: f32, damage: f32, shape: Shape) {
        let spell = self.content.abilities[ability].spell;
        // Holy Nova: friends in it mended.
        let heal = self.content.abilities[ability].area_heal;
        if heal > 0.0 {
            let friends: Vec<Id> = self.players.iter().filter(|(h, p)| self.helpable(owner, **h) && p.health > 0 && dist2d(p.pos, at) <= radius + HERO_SIZE).map(|(h, _)| *h).collect();
            let name = self.content.abilities[ability].name.clone();
            for f in friends {
                let amount = self.players.get(&f).map_or(0, |p| (p.max_health as f32 * heal * self.weakened(owner)).round() as u32);
                self.heal_hero_by(f, amount, owner, &name);
            }
        }
        let inside = |pos: Vec3, size: f32| -> bool {
            if let Shape::Line(f, len, half) = shape {
                return combat::in_strip(at, f, len, half + size, pos);
            }
            let d = dist2d(pos, at);
            if d > radius + size || (pos[2] - at[2]).abs() > 4.0 {
                return false;
            }
            match shape {
                Shape::Cone(f, arc) => d < 0.8 || angle_between(f, facing(at, pos)) <= arc / 2.0,
                _ => true,
            }
        };
        let mobs: Vec<Id> = self.mobs.iter().filter(|(mid, m)| **mid != owner && m.health > 0 && !m.hidden && inside(m.pos, 0.5 * self.content.mobs[m.def].scale) && !self.own_totem(**mid, owner)).map(|(id, _)| *id).collect();
        let heroes: Vec<Id> = if self.content.pvp.friendly_fire {
            // (Not one it mends: a healing area spares a hero it can help.)
            self.players.iter().filter(|(h, p)| self.hostile(owner, **h) && !self.spared(owner, **h, ability) && p.health > 0 && inside(p.pos, HERO_SIZE)).map(|(h, _)| *h).collect()
        } else {
            Vec::new()
        };
        // Nothing to strike with (Sanctuary): only what it does to them.
        if damage <= 0.0 {
            let d = self.content.abilities[ability].clone();
            let was = self.fx_from.replace(at);
            for u in mobs.into_iter().chain(heroes) {
                self.apply_effects(owner, u, &d.effects, share, &d.name);
            }
            self.fx_from = was;
            return;
        }
        for m in mobs {
            self.ability_on_mob(owner, m, ability, share, damage, at, HitKind::Area, spell);
        }
        for h in heroes {
            let dmg = self.ability_damage(owner, h, ability, damage);
            let area = match shape {
                Shape::Cone(f, arc) => Some(combat::Area::Arc { pos: at, facing: f, range: radius, arc }),
                Shape::Circle => Some(combat::Area::Circle { at, radius: radius + HERO_SIZE }),
                Shape::Line(f, len, half) => Some(combat::Area::Line { pos: at, facing: f, len, half: half + HERO_SIZE }),
            };
            self.strike_fair_as(owner, h, dmg, at, false, false, false, Blow::Arrow(spell), area, combat::Follow::Ability { ability, share }, HitKind::Area);
        }
    }

    /// Ground's tick on whoever stands in it: foes burn (and take what it does), friends heal and
    /// take its boon.
    fn area_tick(&mut self, owner: Id, ability: usize, at: Vec3, radius: f32, dps: f32) {
        let d = self.content.abilities[ability].clone();
        let ff = self.content.pvp.friendly_fire;
        let mobs: Vec<Id> = self.mobs.iter().filter(|(mid, m)| m.health > 0 && dist2d(m.pos, at) <= radius && !self.own_totem(**mid, owner) && !self.content.mobs[m.def].inert()).map(|(id, _)| *id).collect();
        let damage = dps.round().max(1.0) as u32;
        for m in &mobs {
            if dps > 0.0 {
                self.events.push(Event::Swing { attacker: owner, victim: Some(*m), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
                self.hurt_mob_as(*m, owner, damage, &d.name);
            }
            if d.at_self && !d.effects.is_empty() {
                self.apply_effects(owner, *m, &d.effects, 1.0, &d.name);
            }
        }
        // What the ground does to every foe in it each tick (a vortex's pull to its middle); the
        // boons among it go to friends instead, as its `boon` does (a barrier's footing).
        let (gifts, harm): (Vec<_>, Vec<_>) = d.tick_effects.iter().cloned().partition(|e| crate::content::BOONS.contains(&e.kind.as_str()));
        let was = self.fx_from.replace(at);
        if !harm.is_empty() {
            for m in &mobs {
                self.apply_effects(owner, *m, &harm, 1.0, &d.name);
            }
        }
        // Friends: the caster, and every other hero unless they're all foes.
        let inside = |p: &Player| p.health > 0 && dist2d(p.pos, at) <= radius && (p.pos[2] - at[2]).abs() < 4.0;
        // (Holy ground mends any hero it can help, friendly fire or not, and never burns them.)
        let friends: Vec<Id> = self.players.iter().filter(|(h, p)| self.helpable(owner, **h) && inside(p)).map(|(h, _)| *h).collect();
        let foes: Vec<Id> = if ff { self.players.iter().filter(|(h, p)| self.hostile(owner, **h) && !self.spared(owner, **h, ability) && inside(p)).map(|(h, _)| *h).collect() } else { Vec::new() };
        for h in friends {
            if d.linger_heal > 0.0 {
                let amount = self.players.get(&h).map_or(0, |p| (p.max_health as f32 * d.linger_heal * self.weakened(owner)).round() as u32);
                self.heal_hero_by(h, amount, owner, &d.name);
            }
            if !d.boon.is_empty() {
                self.apply_cc(h, owner, &d.boon, BOON_HOLD, d.boon_amount, 0.0, &d.name);
            }
            for e in &gifts {
                self.apply_cc(h, owner, &e.kind, BOON_HOLD, e.amount, 0.0, &d.name);
            }
        }
        for h in foes {
            // Burning ground never lands the killing blow on a hero.
            let hp = self.players[&h].health;
            let damage = damage.min(hp.saturating_sub(1));
            if dps > 0.0 && damage > 0 && !self.immune(h) {
                self.events.push(Event::Swing { attacker: owner, victim: Some(h), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
                self.hurt_player_by(h, damage, owner, &d.name);
            }
            if d.at_self && !d.effects.is_empty() {
                self.apply_effects(owner, h, &d.effects, 1.0, &d.name);
            }
            if !harm.is_empty() {
                self.apply_effects(owner, h, &harm, 1.0, &d.name);
            }
        }
        self.fx_from = was;
    }

    fn own_totem(&self, mid: Id, owner: Id) -> bool {
        self.totems.iter().any(|t| t.mob == mid && t.owner == owner)
    }

    fn cone(&mut self, id: Id, ability: usize, yaw: f32, share: f32) {
        let at = self.players[&id].pos;
        self.cone_at(id, ability, at, yaw, share);
    }

    /// A cone from `at` along `yaw`.
    pub(crate) fn cone_at(&mut self, id: Id, ability: usize, at: Vec3, yaw: f32, share: f32) {
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        let damage = AbilityDef::at(d.damage, share);
        self.area(id, ability, at, range, share, damage, Some((yaw, d.arc.max(10.0))));
        if self.debug {
            for k in -2..=2 {
                let a = yaw + (k as f32 / 2.0) * (d.arc / 2.0).to_radians();
                self.mark([at[0] + a.cos() * range, at[1] + a.sin() * range, at[2]], 0.3, 0.8);
            }
        }
    }

    /// An ability all round a spot (Frost Nova, and where a Frost Blink started).
    pub(crate) fn nova(&mut self, id: Id, ability: usize, at: Vec3, share: f32) {
        let d = self.content.abilities[ability].clone();
        let radius = AbilityDef::at(d.radius, share);
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        let damage = AbilityDef::at(d.damage, share);
        self.area(id, ability, at, radius, share, damage, None);
        if self.debug {
            self.mark(at, radius, 0.8);
        }
    }

    fn place_totem(&mut self, id: Id, ability: usize, at: Vec3) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(t) = d.totem.clone() else { return };
        let Some(def) = self.content.mob(&t.mob) else { return };
        // One of each kind a hero has standing.
        let old: Vec<Id> = self.totems.iter().filter(|x| x.owner == id && x.ability == ability).map(|x| x.mob).collect();
        for m in old {
            self.remove_totem(m);
        }
        let mid = self.spawn_mob(def, at, None, false);
        if let Some(m) = self.mobs.get_mut(&mid) {
            m.facing = self.players.get(&id).map_or(0.0, |p| p.facing);
        }
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        self.totems.push(Totem { mob: mid, owner: id, ability, until: now + d.secs.max(1.0), next: now });
        if self.debug {
            self.mark(at, t.radius, d.secs.max(1.0));
        }
    }

    pub(crate) fn remove_totem(&mut self, mid: Id) {
        self.totems.retain(|t| t.mob != mid);
        if self.mobs.remove(&mid).is_some() {
            self.events.push(Event::MobGone { id: mid });
        }
    }

    /// Totems pulse, and fall when their time is up (or they're broken).
    pub(crate) fn step_totems(&mut self) {
        let now = self.clock;
        let list = self.totems.clone();
        for t in list {
            let alive = self.mobs.get(&t.mob).is_some_and(|m| m.health > 0);
            if !alive || now >= t.until || !self.players.contains_key(&t.owner) {
                if !alive {
                    self.totems.retain(|x| x.mob != t.mob);
                } else {
                    self.remove_totem(t.mob);
                }
                continue;
            }
            if now < t.next {
                continue;
            }
            let d = self.content.abilities[t.ability].clone();
            if let Some(x) = self.totems.iter_mut().find(|x| x.mob == t.mob) {
                x.next = now + d.tick.max(0.25);
            }
            let radius = d.totem.as_ref().map_or(8.0, |x| x.radius);
            let at = self.mobs[&t.mob].pos;
            // A healing totem mends its owner and their friends near it.
            if d.linger_heal > 0.0 {
                let near: Vec<Id> = self.players.iter().filter(|(h, p)| self.helpable(t.owner, **h) && p.health > 0 && dist2d(p.pos, at) <= radius).map(|(h, _)| *h).collect();
                for h in near {
                    let amount = self.players.get(&h).map_or(0, |p| (p.max_health as f32 * d.linger_heal).round() as u32);
                    self.heal_hero_by(h, amount, t.owner, &d.name);
                }
            }
            // A totem an ability calls down pulses its `tick_effects` (its landing had `effects`).
            let pulses = if d.tick_effects.is_empty() { &d.effects } else { &d.tick_effects };
            let friendly: Vec<_> = pulses.iter().filter(|e| e.kind == "cleanse_panic").cloned().collect();
            let hostile: Vec<_> = pulses.iter().filter(|e| e.kind != "cleanse_panic").cloned().collect();
            if !friendly.is_empty() {
                let owner = t.owner;
                if self.players.get(&owner).is_some_and(|p| dist2d(p.pos, at) <= radius) {
                    self.apply_effects(owner, owner, &friendly, 1.0, &d.name);
                }
            }
            if !hostile.is_empty() {
                let foes: Vec<Id> = self
                    .mobs
                    .iter()
                    .filter(|(mid, m)| **mid != t.mob && m.health > 0 && !self.content.mobs[m.def].inert() && dist2d(m.pos, at) <= radius)
                    .map(|(id, _)| *id)
                    .chain(self.players.iter().filter(|(h, p)| self.hostile(t.owner, **h) && !self.spared(t.owner, **h, t.ability) && p.health > 0 && dist2d(p.pos, at) <= radius).map(|(h, _)| *h))
                    .collect();
                for f in foes {
                    self.apply_effects(t.owner, f, &hostile, 1.0, &d.name);
                }
            }
        }
    }

    // --- channels -------------------------------------------------------------------------------------

    fn start_channel(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let (o, dir) = self.reticle(id);
        // The first foe along the reticle (a little wide of it still counts), in sight.
        let t = self.trace(o, dir, range + 3.0, 0.8, id);
        let target = t.unit.filter(|u| self.unit_pos(*u).is_some_and(|at| dist2d(at, self.players[&id].pos) <= range + 1.0));
        let Some(target) = target else {
            self.tell(id, format!("{}: nothing there to drain.", d.name));
            // Nothing spent for a miss.
            if let Some(p) = self.players.get_mut(&id) {
                p.cooldowns.remove(&self.content.base_of(ability));
            }
            self.set_action(id, Action::Ready);
            return;
        };
        self.set_action(id, Action::Channel { ability, target, until: now + d.secs, next: now + d.tick.max(0.2), share });
        self.events.push(Event::Channel { caster: id, target: Some(target), spell: d.spell, ms: (d.secs * 1000.0) as u32 });
        self.events.push(Event::Auras { id });
    }

    /// A channel's tick: on while the target is near enough and in sight.
    pub(crate) fn step_channel(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Channel { ability, target, until, next, share }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let pos = self.players[&id].pos;
        let alive = self.alive(target);
        let at = self.unit_pos(target);
        let holds = alive && at.is_some_and(|a| dist2d(a, pos) <= range + 2.0 && clear_line(self.land.as_ref(), pos, a));
        if !holds || now >= until {
            if !holds {
                self.tell(id, format!("{} breaks.", d.name));
            }
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
            return;
        }
        if now < next {
            return;
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.action = Action::Channel { ability, target, until, next: next + d.tick.max(0.2), share };
        }
        let per = AbilityDef::at(d.damage, share) * d.tick.max(0.2);
        self.channel_hit(id, target, ability, share, per, 1.0);
    }

    /// A tether's or a beam's tick on one foe: `per` damage (none on a hero in Ice Block), then
    /// what follows a blow of it (its effects, the drain, `valor` of its Valor).
    fn channel_hit(&mut self, id: Id, target: Id, ability: usize, share: f32, per: f32, valor: f32) {
        let damage = self.ability_damage(id, target, ability, per);
        let name = self.content.abilities[ability].name.clone();
        if self.players.contains_key(&target) {
            if self.immune(target) {
                return;
            }
            self.events.push(Event::Swing { attacker: id, victim: Some(target), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
            self.trade_blows(id, target);
            self.hurt_player_by(target, damage, id, &name);
        } else {
            self.events.push(Event::Swing { attacker: id, victim: Some(target), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
            self.trade_blows(id, target);
            self.hurt_mob_as(target, id, damage, &name);
        }
        let killed = !self.alive(target);
        self.ability_after_at(id, target, ability, share, damage, killed, valor);
    }

    // --- beams ------------------------------------------------------------------------------------------

    /// A beam begins: held as a channel (its `target` the caster itself, as there's no one
    /// unit), its aura on the caster, its look on them as it starts.
    fn start_beam(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        self.set_action(id, Action::Channel { ability, target: id, until: now + d.secs, next: now + d.tick.max(0.2), share });
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        }
        self.events.push(Event::Auras { id });
    }

    /// A beam's tick: the strip from the caster along their aim as it is now (they turn to sweep
    /// it) strikes every foe in it and heals every friend in it. A silence ends it (a stun, a
    /// stagger or a roll already has); so does its time.
    pub(crate) fn step_beam(&mut self, id: Id) {
        let now = self.clock;
        let Some(Action::Channel { ability, target, until, next, share }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        if now >= until || self.silenced(id) {
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
            return;
        }
        if now < next {
            return;
        }
        let d = self.content.abilities[ability].clone();
        let tick = d.tick.max(0.2);
        if let Some(p) = self.players.get_mut(&id) {
            p.action = Action::Channel { ability, target, until, next: next + tick, share };
        }
        let pos = self.players[&id].pos;
        let (yaw, _) = self.aim_of(id);
        let len = AbilityDef::at(d.range, share);
        let half = AbilityDef::at(d.radius, share);
        let (c, s) = (yaw.cos(), yaw.sin());
        let along = |at: Vec3| (at[0] - pos[0]) * c + (at[1] - pos[1]) * s;
        // A hero it can help is a friend in it (a healing beam heals a co-op hero, friendly fire
        // or not, and doesn't burn them); one it can't, if friendly fire lets it, a foe.
        let friend = |h: Id| self.helpable(id, h) && (d.heals() || !self.hostile(id, h));
        // Foes in it, nearest first: creatures (not the caster's own totems) and rival heroes.
        let mut foes: Vec<(Id, f32)> = self
            .mobs
            .iter()
            .filter(|(mid, m)| m.health > 0 && !m.hidden && !self.own_totem(**mid, id) && combat::in_strip(pos, yaw, len, half + 0.5 * self.content.mobs[m.def].scale, m.pos))
            .map(|(mid, m)| (*mid, along(m.pos)))
            .chain(self.players.iter().filter(|(h, p)| **h != id && p.health > 0 && self.hostile(id, **h) && !friend(**h) && combat::in_strip(pos, yaw, len, half + HERO_SIZE, p.pos)).map(|(h, p)| (*h, along(p.pos))))
            .collect();
        foes.sort_by(|a, b| a.1.total_cmp(&b.1));
        let friends: Vec<Id> = self.players.iter().filter(|(h, p)| **h != id && p.health > 0 && friend(**h) && combat::in_strip(pos, yaw, len, half + HERO_SIZE, p.pos)).map(|(h, _)| *h).collect();
        // Its look: to the first foe in it, else to its far end.
        if d.tick_spell != 0 {
            let to = match foes.first() {
                Some((f, _)) => Target::Unit(*f),
                None => Target::Point(self.slide_end(pos, yaw, len)),
            };
            self.events.push(Event::Missile { caster: id, spell: d.tick_spell, target: to });
        }
        let per = AbilityDef::at(d.damage, share) * tick;
        if per > 0.0 || !d.effects.is_empty() {
            for (f, _) in foes {
                if self.alive(f) {
                    self.channel_hit(id, f, ability, share, per, tick);
                }
            }
        }
        if d.ally_heal > 0.0 {
            for f in friends {
                let amount = self.players.get(&f).map_or(0, |p| (p.max_health as f32 * d.ally_heal * tick * self.weakened(id)).round() as u32);
                self.heal_hero_by(f, amount, id, &d.name);
            }
        }
        if self.debug {
            let end = self.slide_end(pos, yaw, len);
            self.mark(end, half.max(0.3), tick);
        }
    }

    // --- dashes and blinks ----------------------------------------------------------------------------

    fn start_dash(&mut self, id: Id, ability: usize, yaw: f32, share: f32) {
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let pos = self.players[&id].pos;
        let dir = [yaw.cos(), yaw.sin(), 0.0];
        // Along the ground: as far as the range, a wall, a drop or a climb, or the first body
        // (going `through`, past every body, each struck as it's passed).
        let bodies = self.bodies(id);
        let mut dist = 0.0f32;
        let mut z = pos[2];
        let mut target = None;
        let mut passed: Vec<(f32, Id, Vec3)> = Vec::new();
        while dist < range {
            let next = (dist + STEP).min(range);
            let p = add(pos, dir, next);
            let g = self.land.height_near(p[0], p[1], z + 1.0);
            if self.land.blocked_near(p[0], p[1], g) || (g - z).abs() > 1.2 {
                break;
            }
            let at = [p[0], p[1], g];
            let near = |feet: &Vec3, size: f32| dist2d(*feet, at) <= size + 0.9 && (feet[2] - g).abs() < 2.5;
            if d.through {
                for (b, feet, _, size) in &bodies {
                    if near(feet, *size) && !passed.iter().any(|(_, x, _)| x == b) {
                        passed.push(((next / DASH_SPEED).max(0.0), *b, at));
                    }
                }
            } else if let Some((b, ..)) = bodies.iter().find(|(_, feet, _, size)| near(feet, *size)) {
                target = Some(*b);
                break;
            }
            z = g;
            dist = next;
        }
        let secs = (dist / DASH_SPEED).max(0.08);
        if let Some(p) = self.players.get_mut(&id) {
            p.combos.through = passed.into_iter().map(|(t, b, at)| (t.min(secs), b, at)).collect();
        }
        self.set_action(id, Action::Dash { t: 0.0, secs, ability, target, share });
        self.events.push(Event::Dash { player: id, dir: yaw, speed: dist / secs, secs });
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
    }

    /// A dash ends: who it ran into takes its blow (going through, whoever's left to pass).
    pub(crate) fn end_dash(&mut self, id: Id) {
        let Some(Action::Dash { ability, target, share, secs, .. }) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        self.set_action(id, Action::Ready);
        self.dash_through(id, ability, share, secs + 1.0);
        let Some(v) = target.filter(|v| self.alive(*v)) else { return };
        let from = self.players[&id].pos;
        self.dash_strike(id, v, ability, share, from);
    }

    fn blink(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let start = self.players[&id].pos;
        let (o, dir) = self.reticle(id);
        let t = self.trace(o, dir, range + 3.0, 0.0, id);
        // Short of what it meets, within the range of where they stand, on the ground.
        let mut at = add(o, dir, (t.dist - 0.8).max(0.0));
        let flat = dist2d(start, at);
        if flat > range {
            let k = range / flat.max(0.01);
            at = [start[0] + (at[0] - start[0]) * k, start[1] + (at[1] - start[1]) * k, at[2]];
        }
        let mut land = self.ground_near(at[0], at[1], at[2] + 1.0);
        // Never into a wall or off into nothing: back along the way till there's ground.
        let mut k = 1.0f32;
        while (self.land.blocked_near(land[0], land[1], land[2]) || (land[2] - at[2]).abs() > 12.0) && k > 0.0 {
            k -= 0.1;
            let p = [start[0] + (at[0] - start[0]) * k, start[1] + (at[1] - start[1]) * k, start[2] + (at[2] - start[2]) * k];
            land = self.ground_near(p[0], p[1], p[2] + 1.0);
        }
        if k <= 0.0 {
            land = self.standing_spot(start);
        }
        let f = self.players[&id].facing;
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        self.set_down(id, land, f);
        if d.cleanse {
            self.cleanse(id, &["root", "snare"]);
        }
        if !d.leaves.is_empty() {
            if let Some(nova) = self.content.ability(&d.leaves) {
                self.nova(id, nova, start, 1.0);
            }
        }
    }

    // --- stances (counters) ---------------------------------------------------------------------------

    fn start_stance(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        self.stances.insert(id, Stance { ability, until: now + d.secs, caught: false });
        self.set_action(id, Action::Stance { ability, until: now + d.secs });
        self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Unit(id) });
        self.events.push(Event::Auras { id });
        self.update_rooted(id);
    }

    /// A stance ends: its cooldown (sooner if it caught something).
    pub(crate) fn end_stance(&mut self, id: Id) {
        let Some(s) = self.stances.remove(&id) else { return };
        let d = self.content.abilities[s.ability].clone();
        // Cold Snap: its ice bursts as it breaks.
        if let (Some(nova), Some(at)) = (self.content.ability(&d.leaves), self.players.get(&id).filter(|p| p.health > 0).map(|p| p.pos)) {
            self.nova(id, nova, at, 1.0);
        }
        let share = if s.caught { d.counter.as_ref().map_or(1.0, |c| c.caught_cooldown) } else { 1.0 };
        if self.players.contains_key(&id) {
            self.spend_charge(id, s.ability, share);
            if matches!(self.players[&id].action, Action::Stance { .. }) {
                self.set_action(id, Action::Ready);
            }
            if !s.caught {
                self.events.push(Event::Flash { player: id, text: format!("{}: caught nothing", d.name) });
            }
            self.update_rooted(id);
        }
        self.events.push(Event::Auras { id });
    }

    /// Stances run out.
    pub(crate) fn step_stances(&mut self) {
        let now = self.clock;
        let done: Vec<Id> = self.stances.iter().filter(|(_, s)| s.until <= now).map(|(id, _)| *id).collect();
        for id in done {
            self.end_stance(id);
        }
    }

    /// A melee blow or a projectile reaches someone in a counter's stance: what the counter makes
    /// of it (`None`: no stance, it goes on as ever; `Some(Hit)`: the counter fired but the blow
    /// still lands).
    pub(crate) fn counter_catch(&mut self, victim: Id, attacker: Id, how: HitKind, projectile: Option<(usize, f32)>) -> Option<Outcome> {
        let now = self.clock;
        if how == HitKind::Area {
            return None;
        }
        let s = self.stances.get(&victim).filter(|s| s.until > now)?.clone();
        let d = self.content.abilities[s.ability].clone();
        let c = d.counter.clone()?;
        // A mirror turns projectiles back as they reach the hero (see `mirrors`); blows pass it.
        if c.mirror || (c.melee_only && how != HitKind::Melee) || (c.ranged_only && how != HitKind::Projectile) {
            return None;
        }
        if let Some(st) = self.stances.get_mut(&victim) {
            st.caught = true;
        }
        if self.players.contains_key(&victim) {
            self.gain_valor(victim, self.content.valor.counter);
            self.events.push(Event::Flash { player: victim, text: d.name.clone() });
        }
        self.events.push(Event::Missile { caster: victim, spell: d.spell, target: Target::Unit(victim) });
        // What it does to the one who struck.
        let close = self.unit_pos(attacker).zip(self.unit_pos(victim)).is_some_and(|(a, v)| dist2d(a, v) <= 6.0);
        if !c.attacker.is_empty() && (how == HitKind::Melee || c.any_attacker || close) {
            self.apply_effects(victim, attacker, &c.attacker, 1.0, &d.name);
        }
        let out = if c.dodge {
            Outcome::Dodged
        } else if c.immune || c.block {
            Outcome::Blocked
        } else if c.absorb_heal > 0.0 {
            let heal = self.max_hp(victim) as f32 * c.absorb_heal;
            self.heal_hero_by(victim, heal.round() as u32, victim, &d.name);
            Outcome::Blocked
        } else if !c.reflect.is_empty() && how == HitKind::Projectile {
            if let Some(r) = self.content.ability(&c.reflect) {
                self.fire_back(victim, attacker, r, projectile.map_or(1.0, |p| p.1));
            }
            Outcome::Blocked
        } else if c.once && !c.attacker.is_empty() && c.any_attacker {
            Outcome::Blocked
        } else {
            Outcome::Hit
        };
        if c.once {
            self.end_stance(victim);
        }
        Some(out)
    }

    fn max_hp(&self, id: Id) -> u32 {
        self.players.get(&id).map(|p| p.max_health).or_else(|| self.mobs.get(&id).map(|m| m.max_health)).unwrap_or(1)
    }

    /// A counter fires a projectile back at the one who shot (Fire Ward's Fire Blast).
    fn fire_back(&mut self, from: Id, at: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        let Some(start) = self.unit_pos(from).map(|p| [p[0], p[1], p[2] + HAND_UP]) else { return };
        let Some(end) = self.unit_pos(at).map(|p| [p[0], p[1], p[2] + 1.0]) else { return };
        let dir = norm([end[0] - start[0], end[1] - start[1], end[2] - start[2]]);
        let range = AbilityDef::at(d.range, share).max(dist2d(start, end) + 4.0);
        self.events.push(Event::Missile { caster: from, spell: d.spell, target: Target::Point(end) });
        let born = self.clock;
        self.projectiles.push(Projectile {
            owner: from,
            ability,
            pos: start,
            dir,
            speed: d.speed.max(10.0),
            radius: AbilityDef::at(d.radius, share),
            left: range,
            share,
            damage: AbilityDef::at(d.damage, share),
            spell: d.spell,
            allies: false,
            hits: Vec::new(),
            pierce: 0,
            stage: Stage::Out,
            born,
        });
    }

    // --- every tick, for heroes -----------------------------------------------------------------------

    /// A hero's ability action carries on: a charge reaching its least (let go already) or its hold,
    /// a channel ticking, a dash running, a cast's beat ending.
    pub(crate) fn step_ability_action(&mut self, id: Id, dt: f32) {
        let now = self.clock;
        let Some(action) = self.players.get(&id).map(|p| p.action.clone()) else { return };
        match action {
            Action::Charge { ability, since } => {
                let (lo, hi) = self.charge_of(id, ability);
                let held = now - since;
                let pending = self.players[&id].release_at_least;
                let hold = self.content.abilities[ability].hold;
                // Topped out: the flare on the caster (everyone sees it), once.
                if hi > 0.0 && held >= hi && held - dt < hi {
                    let kit = self.full_kit(ability);
                    if kit != 0 {
                        self.events.push(Event::Visual { id, kit, by: None });
                    }
                }
                if pending && held >= lo {
                    self.fire(id, ability, if hi > lo { ((held - lo) / (hi - lo)).clamp(0.0, 1.0) } else { 1.0 });
                } else if hold > 0.0 && held >= hi + hold {
                    self.fire(id, ability, 1.0);
                }
            }
            Action::Cast { t, total, ability } => {
                if t + dt >= total {
                    let shot = self.content.abilities[ability].kind == "shot";
                    if shot {
                        self.set_kit(id, Kit::Melee);
                    }
                    self.finish_cast(id);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Cast { t: t + dt, total, ability };
                }
            }
            Action::Channel { ability, .. } if self.content.abilities[ability].kind == "beam" => self.step_beam(id),
            Action::Channel { .. } => self.step_channel(id),
            Action::Dash { t, secs, ability, target, share } => {
                // Going through: each body passed by now takes its blow (and an answer to one, a
                // counter's stagger, ends the dash).
                self.dash_through(id, ability, share, t + dt);
                if !matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Dash { .. })) {
                    return;
                }
                if t + dt >= secs {
                    self.end_dash(id);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Dash { t: t + dt, secs, ability, target, share };
                }
            }
            Action::Stance { until, .. } => {
                if now >= until && !self.stances.contains_key(&id) {
                    self.set_action(id, Action::Ready);
                }
            }
            _ => {}
        }
    }

    // --- training dummies -----------------------------------------------------------------------------

    /// A training dummy in front of the hero (`mode`: `guard`, `roll`, `counter <school>`, or
    /// plain).
    pub fn spawn_dummy(&mut self, id: Id, mode: &str) -> Result<String, String> {
        let def = self.content.mobs.iter().position(|m| m.dummy).ok_or("No training dummy in the rules.")?;
        let p = self.players.get(&id).ok_or("No hero.")?;
        let at = self.ground_near(p.pos[0] + p.facing.cos() * 6.0, p.pos[1] + p.facing.sin() * 6.0, p.pos[2]);
        let face = facing(at, p.pos);
        let mid = self.spawn_mob(def, at, None, false);
        if let Some(m) = self.mobs.get_mut(&mid) {
            m.facing = face;
            m.home = at;
        }
        let mode = mode.trim().to_lowercase();
        if let Some(school) = mode.strip_prefix("counter") {
            let school = school.trim();
            let c = self.content.abilities.iter().position(|a| a.counter.is_some() && (school.is_empty() || a.school == school)).ok_or("No counter of that school.")?;
            self.dummies.insert(mid, format!("counter {}", self.content.abilities[c].key));
        } else if !mode.is_empty() {
            self.dummies.insert(mid, mode.clone());
        }
        if mode == "guard" {
            self.events.push(Event::MobGuard { id: mid, up: true });
        }
        Ok(if mode.is_empty() { "A training dummy".into() } else { format!("A training dummy ({mode})") })
    }

    /// A dummy rolling just now: what reaches it misses.
    pub(crate) fn dummy_rolls(&self, mid: Id) -> bool {
        self.dummies.get(&mid).is_some_and(|m| m == "roll") && (self.clock % 2.0) < 0.6
    }

    /// A guarding dummy facing the blow takes it on its shield.
    pub(crate) fn dummy_guards(&self, mid: Id, from: Vec3) -> bool {
        self.dummies.get(&mid).is_some_and(|m| m == "guard") && self.mobs.get(&mid).is_some_and(|m| angle_between(m.facing, facing(m.pos, from)) <= 70.0)
    }

    /// Dummies on a loop: a counter's stance every few seconds; whole again a while after a blow.
    pub(crate) fn step_dummies(&mut self) {
        let now = self.clock;
        self.dummies.retain(|id, _| self.mobs.contains_key(id));
        let list: Vec<(Id, String)> = self.dummies.iter().map(|(k, v)| (*k, v.clone())).collect();
        for (mid, mode) in list {
            if let Some(key) = mode.strip_prefix("counter ") {
                let Some(a) = self.content.ability(key) else { continue };
                let free = !self.stances.contains_key(&mid);
                if free && (now % 4.0) < 0.06 {
                    let secs = self.content.abilities[a].secs;
                    self.stances.insert(mid, Stance { ability: a, until: now + secs, caught: false });
                    self.events.push(Event::Auras { id: mid });
                }
            }
        }
        // Dummies heal back up when left alone.
        let dummies: Vec<Id> = self.mobs.iter().filter(|(_, m)| self.content.mobs[m.def].dummy && m.health > 0 && m.health < m.max_health).map(|(id, _)| *id).collect();
        for d in dummies {
            let idle = self.trails.get(&d).is_none() || self.players.values().all(|p| p.foe.is_none_or(|(f, at)| f != d || now - at > 5.0));
            if idle {
                let m = self.mobs.get_mut(&d).unwrap();
                m.health = m.max_health;
                let (h, mx) = (m.health, m.max_health);
                self.events.push(Event::Health { id: d, health: h, max: mx });
            }
        }
    }

    /// `.es heal`: whole, rested, no effects.
    pub fn heal_full(&mut self, id: Id) {
        let stamina = self.content.player.stamina;
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.health == 0 {
            return;
        }
        p.health = p.max_health;
        p.stamina = stamina;
        p.effects.clear();
        let (h, m) = (p.health, p.max_health);
        self.events.push(Event::Health { id, health: h, max: m });
        self.events.push(Event::Stamina { player: id });
        self.events.push(Event::Effects { player: id });
        self.clear_cc(id);
    }
}

/// How long the bow's drawn pose is cast for: held until the shot or the aim is let go.
pub(crate) const DRAW_HOLD_MS: u32 = 60_000;
