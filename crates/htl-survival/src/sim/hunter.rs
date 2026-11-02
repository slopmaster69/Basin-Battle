//! The Hunter's own mechanics: arrows that stay where they land and come back (planted arrows,
//! Arrow Recall), the tripwire, Call of the Wild's chain of spirit beasts, Trapper's Instinct's
//! escape, the haul that brings what won't move to its caster's side, and Skyfall's volley.

use std::collections::BTreeMap;

use rand::Rng;

use super::abilities::HitKind;
use super::mobs::MobState;
use super::*;
use crate::content::AbilityDef;
use crate::land::clear_line;

/// A planted arrow lasts this long; a hunter has at most this many (a new one takes the oldest's
/// place).
pub const PLANT_SECS: f32 = 6.0;
pub const PLANT_MOST: usize = 12;
/// How it looks stuck in the ground: a big brown feather, the fletching standing up.
const PLANT_LOOK: u32 = 2630;
const PLANT_SCALE: f32 = 0.45;
/// A tripwire's arrows plant this far each side of its middle; it trips anything within this of
/// the wire.
const WIRE_REACH: f32 = 0.7;
/// A tripwire's two arrows stay in the ground this long, snapped or not: past Tripwire's own
/// cooldown, so Arrow Recall (the same key) can bring them home once it's back.
pub const ANCHOR_SECS: f32 = 18.0;
/// Call of the Wild: the chain falls back to its first stage after this long without a call.
pub const WILD_SECS: f32 = 4.0;
/// The stamina each stage costs (half when empowered), and the damage each stage's beast does.
pub const WILD_STAMINA: [f32; 3] = [15.0, 20.0, 25.0];
pub const WILD_DAMAGE: [f32; 3] = [12.0, 18.0, 30.0];
/// Empowered (a hit, a parry or a perfect dodge since the last call): this much more damage.
const WILD_EMPOWERED: f32 = 1.25;
/// A beast is on its target this long after the call, and gone this long after.
const POUNCE_SECS: f32 = 0.3;
const BEAST_STAYS: f32 = 0.9;
/// A lit beast takes a foe that qualifies within this of the reticle's foe instead.
const LIT_REACH: f32 = 8.0;
/// Arrow Recall: a foe struck by this many arrows or more is Lacerated, a second longer for each
/// past it.
const LACERATE_AT: u32 = 3;
const LACERATE_DPS: f32 = 4.0;
const LACERATE_SECS: f32 = 4.0;
/// A bleed's look (Rend's).
const BLEED_AURA: u32 = 772;
/// Trapper's Instinct: the arrows scattered where the hunter stood, and how far round.
const SCATTER: u32 = 5;
const SCATTER_RING: f32 = 1.6;
/// Skyfall's arrows come down over this long.
const VOLLEY_SECS: f32 = 1.0;
/// Skyfall's hang: the hero falls slowly this long after the leap.
const FLOAT_SECS: f32 = 4.0;
/// Each of its arrows strikes everyone within this of where it lands.
const VOLLEY_SPLASH: f32 = 2.5;

/// One of Call of the Wild's beasts: its creature (a `[[mob]]` with `totem = true`), its icon on
/// the key, and its stage (0 sets up, 1 exploits, 2 finishes).
pub struct Beast {
    pub key: &'static str,
    pub name: &'static str,
    pub mob: &'static str,
    pub icon: &'static str,
    pub stage: usize,
}

/// Three a stage, in stage order.
pub const BEASTS: [Beast; 9] = [
    Beast { key: "wolf", name: "Wolf", mob: "spirit_wolf", icon: "Ability_Hunter_Pet_Wolf", stage: 0 },
    Beast { key: "owl", name: "Owl", mob: "spirit_owl", icon: "Ability_Hunter_Pet_Owl", stage: 0 },
    Beast { key: "scorpid", name: "Scorpid", mob: "spirit_scorpid", icon: "Ability_Hunter_Pet_Scorpid", stage: 0 },
    Beast { key: "gorilla", name: "Gorilla", mob: "spirit_gorilla", icon: "Ability_Hunter_Pet_Gorilla", stage: 1 },
    Beast { key: "raptor", name: "Raptor", mob: "spirit_raptor", icon: "Ability_Hunter_Pet_Raptor", stage: 1 },
    Beast { key: "basilisk", name: "Basilisk", mob: "spirit_basilisk", icon: "Ability_Hunter_Pet_Crocolisk", stage: 1 },
    Beast { key: "bear", name: "Bear", mob: "spirit_bear", icon: "Ability_Hunter_Pet_Bear", stage: 2 },
    Beast { key: "kodo", name: "Kodo", mob: "spirit_kodo", icon: "INV_Misc_Foot_Kodo", stage: 2 },
    Beast { key: "thunder_lizard", name: "Thunder Lizard", mob: "spirit_thunder_lizard", icon: "Ability_ThunderBolt", stage: 2 },
];

/// An arrow stuck in the ground.
#[derive(Debug, Clone)]
pub struct Planted {
    pub owner: Id,
    pub pos: Vec3,
    pub until: f32,
    pub obj: Id,
}

/// A tripwire: its two anchors, until when, and which side of it each creature near it was on
/// (a crossing is a change of side).
#[derive(Debug, Clone)]
pub(crate) struct Wire {
    pub owner: Id,
    pub ability: usize,
    pub a: Vec3,
    pub b: Vec3,
    pub until: f32,
    pub sides: BTreeMap<Id, bool>,
    pub mark: Id,
}

/// A hero's Call of the Wild chain: the stage the next call is, the beast drawn for it, until
/// when the chain holds, and whether the next call is empowered.
#[derive(Debug, Clone, Default)]
pub struct Wild {
    pub stage: usize,
    pub next: usize,
    pub until: f32,
    pub primed: bool,
    pub drawn: bool,
}

/// A beast on its way to its target: it strikes at `at`, and its body goes at `gone`.
#[derive(Debug, Clone)]
pub(crate) struct Pounce {
    pub owner: Id,
    pub ability: usize,
    pub beast: usize,
    pub target: Id,
    pub at: f32,
    pub gone: f32,
    pub empowered: bool,
    pub body: Option<Id>,
    pub struck: bool,
}

/// One of Skyfall's arrows coming down.
#[derive(Debug, Clone)]
pub(crate) struct Arrowfall {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub land_at: f32,
}

/// The Hunter's state in the world.
#[derive(Debug, Clone, Default)]
pub struct Hunt {
    pub planted: Vec<Planted>,
    pub(crate) wires: Vec<Wire>,
    pub wild: BTreeMap<Id, Wild>,
    pub(crate) pounces: Vec<Pounce>,
    pub(crate) falls: Vec<Arrowfall>,
    /// Recalled arrows' looks on their way home: each one's unseen anchor, and when it's home.
    pub(crate) anchors: Vec<(Id, f32)>,
    /// Heroes hanging over the fight (Skyfall's feather fall), until then.
    pub(crate) floating: Vec<(Id, f32)>,
}

impl Survival {
    // --- planted arrows ----------------------------------------------------------------------------

    /// An arrow of `owner`'s sticks in the ground at (over) `at`.
    pub(crate) fn plant_arrow(&mut self, owner: Id, at: Vec3) {
        self.plant_arrow_for(owner, at, PLANT_SECS);
    }

    /// An arrow that stays `secs`. Past the cap, the one nearest fading makes way.
    pub(crate) fn plant_arrow_for(&mut self, owner: Id, at: Vec3, secs: f32) {
        let now = self.clock;
        let pos = self.ground_near(at[0], at[1], at[2] + 1.0);
        let mine: Vec<usize> = self.hunt.planted.iter().enumerate().filter(|(_, a)| a.owner == owner).map(|(k, _)| k).collect();
        if mine.len() >= PLANT_MOST {
            let soonest = mine.iter().copied().min_by(|x, y| self.hunt.planted[*x].until.total_cmp(&self.hunt.planted[*y].until)).unwrap_or(mine[0]);
            let old = self.hunt.planted.remove(soonest);
            self.unplant(&old);
        }
        let obj = self.alloc();
        let facing = self.rng.gen::<f32>() * std::f32::consts::TAU;
        self.objs.insert(obj, Obj { kind: ObjKind::Prop { display: PLANT_LOOK, scale: PLANT_SCALE }, pos, facing });
        self.events.push(Event::ObjSpawned { id: obj });
        self.hunt.planted.push(Planted { owner, pos, until: now + secs, obj });
    }

    fn unplant(&mut self, a: &Planted) {
        if self.objs.remove(&a.obj).is_some() {
            self.events.push(Event::ObjGone { id: a.obj });
        }
    }

    /// A hero's planted arrows, where they stand.
    pub fn planted_of(&self, owner: Id) -> Vec<Vec3> {
        self.hunt.planted.iter().filter(|a| a.owner == owner).map(|a| a.pos).collect()
    }

    /// A projectile that plants has ended its flight at `at` (`past`: a foe stopped it, so the
    /// arrow goes a yard on past them, along `dir`).
    pub(crate) fn plant_at_end(&mut self, owner: Id, ability: usize, at: Vec3, dir: Vec3, past: bool) {
        if !self.content.abilities[ability].plants || !self.players.contains_key(&owner) {
            return;
        }
        let at = if past { [at[0] + dir[0], at[1] + dir[1], at[2]] } else { at };
        // Out of a wall it hit: a little back toward where it came from.
        let at = if self.land.blocked_near(at[0], at[1], at[2]) { [at[0] - dir[0] * 0.6, at[1] - dir[1] * 0.6, at[2]] } else { at };
        self.plant_arrow(owner, at);
    }

    // --- the step ----------------------------------------------------------------------------------

    /// Every tick: planted arrows fade, tripwires trip or fade, beasts strike and go, Skyfall's
    /// arrows land.
    pub(crate) fn step_hunt(&mut self) {
        let now = self.clock;
        if !self.hunt.planted.is_empty() {
            let (gone, keep): (Vec<Planted>, Vec<Planted>) = std::mem::take(&mut self.hunt.planted).into_iter().partition(|a| a.until <= now || !self.players.contains_key(&a.owner));
            self.hunt.planted = keep;
            for a in gone {
                self.unplant(&a);
            }
        }
        if !self.hunt.wires.is_empty() {
            self.step_wires();
        }
        if !self.hunt.pounces.is_empty() {
            self.step_pounces();
        }
        if !self.hunt.floating.is_empty() {
            let (down, keep): (Vec<(Id, f32)>, Vec<(Id, f32)>) = std::mem::take(&mut self.hunt.floating).into_iter().partition(|(_, t)| *t <= now);
            self.hunt.floating = keep;
            for (h, _) in down {
                self.events.push(Event::FeatherFall { player: h, on: false });
            }
        }
        if !self.hunt.anchors.is_empty() {
            let (home, keep): (Vec<(Id, f32)>, Vec<(Id, f32)>) = std::mem::take(&mut self.hunt.anchors).into_iter().partition(|(_, t)| *t <= now);
            self.hunt.anchors = keep;
            for (a, _) in home {
                self.events.push(Event::ObjGone { id: a });
            }
        }
        if !self.hunt.falls.is_empty() {
            let (due, keep): (Vec<Arrowfall>, Vec<Arrowfall>) = std::mem::take(&mut self.hunt.falls).into_iter().partition(|f| f.land_at <= now);
            self.hunt.falls = keep;
            for f in due {
                self.arrow_lands(f);
            }
        }
    }

    // --- the special casts -------------------------------------------------------------------------

    /// An ability with a Hunter mechanic of its own goes (true: it's been handled).
    pub(crate) fn fire_hunt(&mut self, id: Id, ability: usize, share: f32) -> bool {
        let hunt = self.content.abilities[ability].hunt.clone();
        match hunt.as_str() {
            "call" => self.call_wild(id, ability, false),
            "double_call" => self.call_wild(id, ability, true),
            "tripwire" => self.lay_tripwire(id, ability, share),
            "recall" => self.recall_arrows(id, ability),
            "volley" => self.skyfall_volley(id, ability, share),
            _ => return false,
        }
        true
    }

    /// The stamina an ability takes now: its own, or for Call of the Wild its stage's (half,
    /// empowered).
    pub(crate) fn hunt_stamina(&self, id: Id, ability: usize) -> f32 {
        let d = &self.content.abilities[ability];
        match d.hunt.as_str() {
            "call" | "double_call" => {
                let w = self.wild_now(id);
                WILD_STAMINA[w.stage] * if w.primed { 0.5 } else { 1.0 }
            }
            _ => d.stamina,
        }
    }

    // --- the tripwire ------------------------------------------------------------------------------

    fn lay_tripwire(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let half = AbilityDef::at(d.radius, share).max(1.0);
        let at = self.aim_point(id, range);
        // Across the hero's line of fire, so what comes at them crosses it.
        let f = self.players[&id].facing + std::f32::consts::FRAC_PI_2;
        let a = self.ground_near(at[0] + f.cos() * half, at[1] + f.sin() * half, at[2] + 1.0);
        let b = self.ground_near(at[0] - f.cos() * half, at[1] - f.sin() * half, at[2] + 1.0);
        // One wire a hunter.
        let old: Vec<Wire> = self.hunt.wires.iter().filter(|w| w.owner == id).cloned().collect();
        self.hunt.wires.retain(|w| w.owner != id);
        for w in old {
            self.unmark(w.mark);
        }
        for end in [a, b] {
            self.events.push(Event::Missile { caster: id, spell: 19434, target: Target::Point(end) });
            self.plant_arrow_for(id, end, ANCHOR_SECS);
        }
        let life = d.secs.max(1.0);
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
        let len = dist2d(a, b);
        let tele = Telegraph { own_hidden: false, ..Telegraph::hero_shape(id, WIRE_REACH, 0.0, len, life, Some((mid, f))) };
        let mark = self.mark_as(mid, WIRE_REACH, life, Some(tele));
        let sides = self.wire_sides(a, b, id);
        self.hunt.wires.push(Wire { owner: id, ability, a, b, until: now + life, sides, mark });
    }

    /// Which side of the line a..b each creature (and rival hero) near it stands on.
    fn wire_sides(&self, a: Vec3, b: Vec3, owner: Id) -> BTreeMap<Id, bool> {
        let mut out = BTreeMap::new();
        let reach = dist2d(a, b) / 2.0 + 4.0;
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, 0.0];
        for (v, at) in self.foes_near(owner, mid, reach) {
            out.insert(v, side(a, b, at));
        }
        out
    }

    /// The units `owner` can strike within `reach` of `at` (alive, not their own).
    fn foes_near(&self, owner: Id, at: Vec3, reach: f32) -> Vec<(Id, Vec3)> {
        let mut out: Vec<(Id, Vec3)> = self
            .mobs
            .iter()
            .filter(|(m, x)| x.health > 0 && !x.hidden && !self.content.mobs[x.def].inert() && !self.own_totem(**m, owner) && dist2d(x.pos, at) <= reach)
            .map(|(m, x)| (*m, x.pos))
            .collect();
        if self.content.pvp.friendly_fire {
            out.extend(self.players.iter().filter(|(h, p)| **h != owner && p.health > 0 && self.hostile(owner, **h) && dist2d(p.pos, at) <= reach).map(|(h, p)| (*h, p.pos)));
        }
        out
    }

    fn step_wires(&mut self) {
        let now = self.clock;
        let wires = std::mem::take(&mut self.hunt.wires);
        let mut keep = Vec::new();
        for mut w in wires {
            if w.until <= now || !self.players.contains_key(&w.owner) {
                self.unmark(w.mark);
                continue;
            }
            let reach = dist2d(w.a, w.b) / 2.0 + 4.0;
            let mid = [(w.a[0] + w.b[0]) / 2.0, (w.a[1] + w.b[1]) / 2.0, 0.0];
            let mut tripped = None;
            for (v, at) in self.foes_near(w.owner, mid, reach) {
                let now_side = side(w.a, w.b, at);
                let crossed = w.sides.get(&v).is_some_and(|s| *s != now_side);
                let on = seg_dist(w.a, w.b, at) <= WIRE_REACH;
                w.sides.insert(v, now_side);
                if (crossed || on) && seg_dist(w.a, w.b, at) <= WIRE_REACH + 1.5 {
                    tripped = Some(v);
                    break;
                }
            }
            match tripped {
                Some(v) => {
                    self.unmark(w.mark);
                    self.trip(w.owner, v, w.ability);
                }
                None => keep.push(w),
            }
        }
        keep.extend(std::mem::take(&mut self.hunt.wires));
        self.hunt.wires = keep;
    }

    /// The wire catches `v`: its damage, and it goes down (a boss stumbles: posture instead).
    fn trip(&mut self, owner: Id, v: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(at) = self.unit_pos(v) else { return };
        self.events.push(Event::Visual { id: v, kit: KIT_DUST, by: None });
        self.hunt_hit(owner, v, ability, AbilityDef::at(d.damage, 1.0), at);
        if self.holds_ground(v) {
            self.build_posture(v, owner, 40);
        }
        self.events.push(Event::Flash { player: owner, text: "Tripped!".into() });
    }

    // --- knockdown ---------------------------------------------------------------------------------

    /// Knocked down: a stun that looks it (thrown to the ground, dust), or on what holds its
    /// ground (a boss) a blow to its posture instead.
    pub(crate) fn knock_down(&mut self, owner: Id, victim: Id, secs: f32, source: &str) {
        if self.holds_ground(victim) {
            if self.players.contains_key(&owner) {
                self.build_posture(victim, owner, (secs * 20.0).round() as u32);
            }
            return;
        }
        if self.apply_cc_look(victim, owner, "stun", secs, 0.0, 0.0, source, 0) {
            let from = self.unit_pos(owner).unwrap_or([0.0; 3]);
            let secs = if self.players.contains_key(&victim) { secs * self.content.cc.player_share } else { secs };
            self.events.push(Event::React { id: victim, anim: KNOCKDOWN_ANIM, from, lean: 0.0, hold: 0.0, fit: secs });
            self.events.push(Event::Visual { id: victim, kit: KIT_DUST, by: None });
        }
    }

    /// An ability's blow from `owner` on `v` (a creature or a rival hero), as an area's: its
    /// damage and effects.
    pub(crate) fn hunt_hit(&mut self, owner: Id, v: Id, ability: usize, damage: f32, from: Vec3) {
        if !self.alive(v) {
            return;
        }
        let spell = self.content.abilities[ability].spell;
        if self.mobs.contains_key(&v) {
            self.ability_on_mob(owner, v, ability, 1.0, damage, from, HitKind::Area, spell);
        } else {
            let dmg = self.ability_damage(owner, v, ability, damage);
            self.strike_fair_as(owner, v, dmg, from, false, false, false, Blow::Arrow(spell), None, combat::Follow::Ability { ability, share: 1.0 }, HitKind::Area);
        }
    }

    // --- Arrow Recall ------------------------------------------------------------------------------

    /// Every arrow the hunter has planted flies back to them, striking every foe it passes through
    /// (each once, at full damage); three arrows or more in one foe leave it Lacerated.
    fn recall_arrows(&mut self, id: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(home) = self.players.get(&id).map(|p| p.pos) else { return };
        let mine: Vec<Planted> = self.hunt.planted.iter().filter(|a| a.owner == id).cloned().collect();
        self.hunt.planted.retain(|a| a.owner != id);
        if mine.is_empty() {
            self.events.push(Event::Flash { player: id, text: "No arrows planted".into() });
            return;
        }
        let radius = AbilityDef::at(d.radius, 1.0).max(0.4);
        let damage = AbilityDef::at(d.damage, 1.0);
        let mut struck: BTreeMap<Id, u32> = BTreeMap::new();
        for a in &mine {
            self.unplant(a);
            // Its look, flown home from where it stood.
            let anchor = self.alloc();
            let from = [a.pos[0], a.pos[1], a.pos[2] + 0.8];
            self.events.push(Event::MissileFrom { anchor, caster: id, spell: d.spell, from, target: Target::Unit(id) });
            self.hunt.anchors.push((anchor, self.clock + dist2d(from, home) / d.speed.max(10.0) + 0.2));
            // Everyone on its way home.
            for (v, at) in self.foes_near(id, home, dist2d(a.pos, home) + 2.0) {
                if seg_dist(a.pos, home, at) <= radius + 0.5 {
                    *struck.entry(v).or_default() += 1;
                }
            }
        }
        for (v, n) in struck {
            let Some(at) = self.unit_pos(v) else { continue };
            for _ in 0..n {
                self.hunt_hit(id, v, ability, damage, at);
            }
            if n >= LACERATE_AT && self.alive(v) {
                let secs = LACERATE_SECS + (n - LACERATE_AT) as f32;
                self.apply_cc_look(v, id, "burn", secs, LACERATE_DPS, 0.0, "Lacerate", BLEED_AURA);
                self.events.push(Event::Flash { player: id, text: format!("Lacerated! ({n} arrows)") });
            }
        }
    }

    // --- Trapper's Instinct ------------------------------------------------------------------------

    /// Trapper's Instinct caught `attacker`'s blow: the net on them, arrows scattered where the
    /// hunter stood (striking whoever's by each), and the leap away from them.
    pub(crate) fn trappers_answer(&mut self, id: Id, attacker: Id, leap: usize) {
        let Some(at) = self.players.get(&id).map(|p| p.pos) else { return };
        let d = self.content.abilities[leap].clone();
        // The net on whoever struck (a boss's posture instead).
        if let Some(net) = self.content.ability("hunter_net") {
            self.events.push(Event::Missile { caster: id, spell: self.content.abilities[net].spell, target: Target::Unit(attacker) });
            if self.holds_ground(attacker) {
                self.build_posture(attacker, id, 30);
            } else {
                let e = self.content.abilities[net].effects.clone();
                self.apply_effects(id, attacker, &e, 1.0, &d.name);
            }
        }
        // Arrows scattered round where the hunter stood, each striking whoever's by it.
        let radius = AbilityDef::at(d.radius, 1.0).max(1.0);
        let damage = AbilityDef::at(d.damage, 1.0);
        let mut hit: Vec<Id> = Vec::new();
        for k in 0..SCATTER {
            let a = k as f32 / SCATTER as f32 * std::f32::consts::TAU;
            let spot = [at[0] + a.cos() * SCATTER_RING, at[1] + a.sin() * SCATTER_RING, at[2]];
            self.plant_arrow(id, spot);
            for (v, vat) in self.foes_near(id, spot, radius) {
                if !hit.contains(&v) {
                    hit.push(v);
                    self.hunt_hit(id, v, leap, damage, vat);
                }
            }
        }
        // Away: facing whoever struck, the leap goes straight back from them.
        if let (Some(a), Some(p)) = (self.unit_pos(attacker), self.players.get_mut(&id)) {
            p.facing = facing(at, a);
            p.aim_line = None;
        }
        self.leap_to(id, leap, 1.0, None);
    }

    // --- the haul ----------------------------------------------------------------------------------

    /// A haul struck `v`: what can be moved comes to the hunter; what can't (a boss, the
    /// unstoppable) hauls the hunter to it (true: the hunter goes).
    pub(crate) fn haul_reverses(&self, v: Id) -> bool {
        self.holds_ground(v) || self.mobs.get(&v).is_some_and(|m| self.content.mobs[m.def].inert())
    }

    // --- Skyfall -----------------------------------------------------------------------------------

    /// Skyfall's leap: the hero falls slowly for a while, hanging over the fight to aim.
    pub(crate) fn skyfall_float(&mut self, id: Id) {
        let until = self.clock + FLOAT_SECS;
        self.hunt.floating.retain(|(h, _)| *h != id);
        self.hunt.floating.push((id, until));
        self.events.push(Event::FeatherFall { player: id, on: true });
    }

    /// Skyfall's volley: its arrows come down over a second, scattered over the circle at the
    /// reticle.
    fn skyfall_volley(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let radius = AbilityDef::at(d.radius, share);
        let at = self.aim_point(id, range);
        let n = d.count.max(1);
        let tele = Telegraph::hero_shape(id, radius, 360.0, 0.0, VOLLEY_SECS, Some((at, 0.0)));
        self.mark_as(at, radius, VOLLEY_SECS + 0.2, Some(tele));
        for k in 0..n {
            // Spread evenly over the circle (a sunflower's spiral), each a beat after the last.
            let r = radius * ((k as f32 + 0.5) / n as f32).sqrt();
            let a = k as f32 * 2.399_963;
            let spot = self.ground_near(at[0] + a.cos() * r, at[1] + a.sin() * r, at[2] + 2.0);
            let land_at = now + 0.2 + VOLLEY_SECS * k as f32 / n as f32;
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(spot) });
            self.hunt.falls.push(Arrowfall { owner: id, ability, at: spot, land_at });
        }
    }

    /// One of Skyfall's arrows lands.
    fn arrow_lands(&mut self, f: Arrowfall) {
        let d = self.content.abilities[f.ability].clone();
        let damage = AbilityDef::at(d.damage, 1.0);
        for (v, at) in self.foes_near(f.owner, f.at, VOLLEY_SPLASH) {
            let _ = at;
            self.hunt_hit(f.owner, v, f.ability, damage, f.at);
        }
        if self.players.contains_key(&f.owner) {
            self.plant_arrow(f.owner, f.at);
        }
    }

    // --- Call of the Wild --------------------------------------------------------------------------

    /// A hero's chain as it stands now (back at the first stage once its time is up; the first
    /// stage's beast drawn if none is).
    pub fn wild_now(&self, id: Id) -> Wild {
        let mut w = self.hunt.wild.get(&id).cloned().unwrap_or_default();
        if w.until <= self.clock && w.stage != 0 {
            w.stage = 0;
            w.drawn = false;
        }
        if !w.drawn {
            // (The draw shown before it's made: the first of the stage, until a call draws.)
            w.next = w.stage * 3;
        }
        w
    }

    /// The beast the hero's next call brings, for the key's icon.
    pub fn next_beast(&self, id: Id) -> &'static Beast {
        &BEASTS[self.wild_now(id).next]
    }

    /// Something the hero did that empowers their next call: a hit landed, a parry, a perfect
    /// dodge.
    pub(crate) fn prime_wild(&mut self, id: Id) {
        if let Some(w) = self.hunt.wild.get_mut(&id) {
            if !w.primed {
                w.primed = true;
                self.events.push(Event::Abilities { player: id });
            }
        }
    }

    fn draw_beast(&mut self, stage: usize) -> usize {
        stage * 3 + self.rng.gen_range(0..3)
    }

    /// A call: the beast shown (and with `double`, one from the next stage at once), at the foe
    /// nearest the reticle, or a nearby foe its bonus fits.
    fn call_wild(&mut self, id: Id, ability: usize, double: bool) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let mut w = self.wild_now(id);
        if !w.drawn {
            w.next = self.draw_beast(w.stage);
            w.drawn = true;
        }
        let range = AbilityDef::at(d.range, 1.0);
        let Some(target) = self.beast_target(id, range, w.next) else {
            self.events.push(Event::Flash { player: id, text: "No enemy in range".into() });
            self.hunt.wild.insert(id, w);
            return;
        };
        let cost = WILD_STAMINA[w.stage] * if w.primed { 0.5 } else { 1.0 };
        self.spend(id, cost);
        let empowered = w.primed;
        self.send_beast(id, ability, w.next, target, empowered, 0.0);
        let mut stage = w.stage + 1;
        if double {
            // A second at once, from the next stage (a third stage's: another of the third); not
            // empowered; the chain skips a stage.
            let s2 = (w.stage + 1).min(2);
            let b2 = self.draw_beast(s2);
            self.send_beast(id, ability, b2, target, false, 0.15);
            stage += 1;
        }
        if w.stage == 2 || stage > 2 {
            stage = 0;
        }
        let next = self.draw_beast(stage);
        self.hunt.wild.insert(id, Wild { stage, next, until: now + WILD_SECS, primed: false, drawn: true });
        self.events.push(Event::Abilities { player: id });
    }

    /// Whom a beast goes for: the hero's lock, else the foe nearest the reticle's line within
    /// range; a beast whose bonus a foe near that one fits goes for that one instead, unless the
    /// aimed one fits.
    fn beast_target(&self, id: Id, range: f32, beast: usize) -> Option<Id> {
        let p = self.players.get(&id)?;
        let pos = p.pos;
        let locked = p.lock.filter(|v| self.alive(*v) && self.unit_pos(*v).is_some_and(|at| dist2d(at, pos) <= range));
        let aimed = locked.or_else(|| {
            let (yaw, _) = self.aim_of(id);
            self.foes_near(id, pos, range)
                .into_iter()
                .filter(|(_, at)| angle_between(yaw, facing(pos, *at)) <= 0.6 && clear_line(self.land.as_ref(), pos, *at))
                .min_by(|a, b| {
                    let off = |at: Vec3| angle_between(yaw, facing(pos, at)) * 10.0 + dist2d(pos, at) * 0.05;
                    off(a.1).total_cmp(&off(b.1))
                })
                .map(|(v, _)| v)
        })?;
        if self.beast_lit(beast, aimed) {
            return Some(aimed);
        }
        let at = self.unit_pos(aimed)?;
        let other = self.foes_near(id, at, LIT_REACH).into_iter().filter(|(v, vat)| *v != aimed && dist2d(*vat, pos) <= range && self.beast_lit(beast, *v)).min_by(|a, b| dist2d(a.1, at).total_cmp(&dist2d(b.1, at)));
        Some(other.map_or(aimed, |(v, _)| v))
    }

    /// Whether a beast's bonus fits `v` now.
    pub fn beast_lit(&self, beast: usize, v: Id) -> bool {
        let now = self.clock;
        let staggered = self.mobs.get(&v).is_some_and(|m| m.stagger_until > now);
        match BEASTS[beast].key {
            "wolf" => self.mobs.get(&v).is_some_and(|m| matches!(m.state, MobState::Windup { .. })),
            "owl" => true,
            "scorpid" => self.has_cc(v, "mark") || self.has_cc(v, "sunder"),
            "gorilla" => self.unit_pos(v).is_some_and(|at| self.mobs.values().filter(|m| m.health > 0 && dist2d(m.pos, at) <= 6.0).count() >= 3),
            "raptor" => self.has_cc(v, "burn"),
            "basilisk" => self.has_cc(v, "snare"),
            "bear" => staggered || self.has_cc(v, "stun") || self.has_cc(v, "petrify") || self.stance_broken(v),
            "kodo" => self.has_cc(v, "mark"),
            "thunder_lizard" => self.mobs.get(&v).is_some_and(|m| m.guarding) || self.players.get(&v).is_some_and(|p| matches!(p.action, Action::Guard)),
            _ => false,
        }
    }

    /// A beast bursts out at the hero's side and runs at `target`.
    fn send_beast(&mut self, id: Id, ability: usize, beast: usize, target: Id, empowered: bool, delay: f32) {
        let now = self.clock;
        let Some(from) = self.players.get(&id).map(|p| p.pos) else { return };
        let Some(to) = self.unit_pos(target) else { return };
        let f = facing(from, to);
        let side = if self.rng.gen::<bool>() { 1.0 } else { -1.0 };
        let start = self.ground_near(from[0] + (f + side * 1.2).cos() * 1.5, from[1] + (f + side * 1.2).sin() * 1.5, from[2] + 1.0);
        let body = self.content.mob(BEASTS[beast].mob).map(|def| {
            let mid = self.spawn_mob(def, start, None, false);
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.facing = f;
            }
            mid
        });
        let end = [to[0] - f.cos() * 1.5, to[1] - f.sin() * 1.5, to[2]];
        if let Some(b) = body {
            let dist = dist2d(start, end).max(0.5);
            self.events.push(Event::MobMove { id: b, path: vec![start, end], speed: dist / (POUNCE_SECS + delay).max(0.1), face: None });
        }
        self.events.push(Event::Missile { caster: id, spell: SPIRIT_FLASH, target: Target::Point(start) });
        self.hunt.pounces.push(Pounce { owner: id, ability, beast, target, at: now + POUNCE_SECS + delay, gone: now + POUNCE_SECS + delay + BEAST_STAYS, empowered, body, struck: false });
    }

    fn step_pounces(&mut self) {
        let now = self.clock;
        let list = std::mem::take(&mut self.hunt.pounces);
        let mut keep = Vec::new();
        for mut p in list {
            if !p.struck && p.at <= now {
                p.struck = true;
                if let Some(b) = p.body {
                    self.events.push(Event::Swing { attacker: b, victim: Some(p.target), damage: 0, outcome: Outcome::Hit, anim: true, style: Blow::Heavy });
                }
                if self.alive(p.target) && self.players.contains_key(&p.owner) {
                    self.beast_strikes(&p);
                }
            }
            if p.gone <= now {
                if let Some(b) = p.body {
                    if self.mobs.remove(&b).is_some() {
                        self.events.push(Event::MobGone { id: b });
                    }
                }
                continue;
            }
            keep.push(p);
        }
        keep.extend(std::mem::take(&mut self.hunt.pounces));
        self.hunt.pounces = keep;
    }

    /// The beast's strike: its stage's damage and its own effect, its bonus if it fits.
    fn beast_strikes(&mut self, p: &Pounce) {
        let now = self.clock;
        let (owner, v, ability) = (p.owner, p.target, p.ability);
        let beast = &BEASTS[p.beast];
        let lit = self.beast_lit(p.beast, v);
        let more = if p.empowered { WILD_EMPOWERED } else { 1.0 };
        let base = WILD_DAMAGE[beast.stage] * more;
        let Some(at) = self.unit_pos(v) else { return };
        let boss = self.holds_ground(v);
        let name = format!("{} ({})", self.content.abilities[ability].name, beast.name);
        if lit {
            self.events.push(Event::Flash { player: owner, text: format!("{}!", beast.name) });
        }
        match beast.key {
            "wolf" => {
                self.hunt_hit(owner, v, ability, base, at);
                if lit {
                    if boss {
                        self.build_posture(v, owner, 20);
                    } else if self.mobs.contains_key(&v) {
                        self.stagger_mob(v, 0.8, false);
                    }
                }
                if p.empowered {
                    self.apply_cc(v, owner, "snare", 2.0, 0.3, 0.0, &name);
                }
            }
            "owl" => {
                self.hunt_hit(owner, v, ability, base, at);
                let secs = if p.empowered { 12.0 } else { 8.0 };
                self.apply_cc_look(v, owner, "mark", secs, 0.5, 0.0, &name, 1130);
            }
            "scorpid" => {
                self.hunt_hit(owner, v, ability, base, at);
                if lit {
                    if boss || self.content.mobs.get(self.mobs.get(&v).map_or(usize::MAX, |m| m.def)).is_some_and(|d| d.posture > 0) {
                        self.build_posture(v, owner, 30);
                    } else if let Some(h) = self.players.get_mut(&v) {
                        h.stamina = (h.stamina - 30.0).max(0.0);
                        self.events.push(Event::Stamina { player: v });
                    } else {
                        self.apply_cc(v, owner, "stagger", 0.6, 0.0, 0.0, &name);
                    }
                }
                let secs = if p.empowered { 8.0 } else { 6.0 };
                self.apply_cc_look(v, owner, "weaken", secs, 0.25, 0.0, &name, 0);
            }
            "gorilla" => {
                let reach = if p.empowered { 4.0 } else { 3.0 };
                if lit {
                    // A pile: everyone within 6 yd dragged onto the one it struck.
                    let pull = crate::content::AbilityEffect { kind: "pull".into(), amount: 6.0, ..Default::default() };
                    let was = self.fx_from.replace(at);
                    for (u, _) in self.foes_near(owner, at, 6.0) {
                        if u != v {
                            self.displace(owner, u, &pull);
                        }
                    }
                    self.fx_from = was;
                }
                self.hunt_hit(owner, v, ability, base, at);
                for (u, _) in self.foes_near(owner, at, reach) {
                    if self.holds_ground(u) {
                        self.build_posture(u, owner, 15);
                    } else {
                        self.apply_cc(u, owner, "stagger", 0.8, 0.0, 0.0, &name);
                    }
                }
                self.events.push(Event::Visual { id: v, kit: KIT_DUST, by: None });
            }
            "raptor" => {
                if lit {
                    // Every bleed on it ripped out at once: what it had left, and more.
                    let left: f32 = self.cc_of(v).iter().filter(|c| c.kind == "burn" && c.until > now).map(|c| c.amount * (c.until - now).max(0.0)).sum();
                    self.cleanse(v, &["burn"]);
                    let rip = left * if p.empowered { 1.5 } else { 1.25 };
                    self.hunt_hit(owner, v, ability, base + rip, at);
                    self.events.push(Event::Visual { id: v, kit: combat::KIT_CLEAVE, by: None });
                } else {
                    self.hunt_hit(owner, v, ability, base, at);
                    self.apply_cc_look(v, owner, "burn", 4.0, 3.0, 0.0, &name, BLEED_AURA);
                }
            }
            "basilisk" => {
                self.hunt_hit(owner, v, ability, base, at);
                if lit {
                    let secs = if p.empowered { 2.0 } else { 1.5 };
                    self.apply_cc(v, owner, "petrify", secs, 0.0, 0.0, &name);
                } else {
                    self.apply_cc(v, owner, "snare", 3.0, 0.5, 0.0, &name);
                }
            }
            "bear" => {
                let dmg = if lit { base * 3.0 } else { base };
                self.hunt_hit(owner, v, ability, dmg, at);
                if p.empowered && self.players.contains_key(&owner) {
                    self.build_posture(v, owner, 25);
                }
                self.events.push(Event::Visual { id: v, kit: combat::KIT_HEAVY, by: None });
            }
            "kodo" => {
                // A stampede along the line from the hunter to the foe.
                let Some(from) = self.players.get(&owner).map(|h| h.pos) else { return };
                let half = if p.empowered { 2.0 } else { 1.5 };
                let marked = lit;
                if marked {
                    self.take_mark(v);
                }
                let push = crate::content::AbilityEffect { kind: "knockback".into(), amount: 4.0, from_caster: true, ..Default::default() };
                let end = [at[0] + (at[0] - from[0]) * 0.1, at[1] + (at[1] - from[1]) * 0.1, at[2]];
                let trampled: Vec<(Id, Vec3)> = self.foes_near(owner, from, dist2d(from, end) + 2.0).into_iter().filter(|(_, u)| seg_dist(from, end, *u) <= half).collect();
                for (u, uat) in trampled {
                    self.hunt_hit(owner, u, ability, base, uat);
                    self.displace(owner, u, &push);
                    if marked {
                        self.knock_down(owner, u, 1.0, &name);
                    }
                }
            }
            "thunder_lizard" => {
                self.hunt_hit(owner, v, ability, base, at);
                if self.content.mobs.get(self.mobs.get(&v).map_or(usize::MAX, |m| m.def)).is_some_and(|d| d.posture > 0) {
                    self.build_posture(v, owner, 40);
                } else if let Some(h) = self.players.get_mut(&v) {
                    h.stamina = (h.stamina - 40.0).max(0.0);
                    self.events.push(Event::Stamina { player: v });
                }
                if lit {
                    if let Some(m) = self.mobs.get_mut(&v) {
                        m.guarding = false;
                    }
                    if boss {
                        self.build_posture(v, owner, 30);
                    } else {
                        self.apply_cc(v, owner, "stagger", 2.0, 0.0, 0.0, &name);
                    }
                }
                if p.empowered {
                    if let Some((u, uat)) = self.foes_near(owner, at, 6.0).into_iter().find(|(u, _)| *u != v) {
                        self.hunt_hit(owner, u, ability, base * 0.5, uat);
                    }
                }
                self.events.push(Event::Visual { id: v, kit: THUNDER_KIT, by: None });
            }
            _ => self.hunt_hit(owner, v, ability, base, at),
        }
    }
}

/// The flash a spirit beast bursts out of (Spirit Wolf's summoning).
const SPIRIT_FLASH: u32 = 2645;
/// The Thunder Lizard's shock on what it strikes (Thunderclap's).
const THUNDER_KIT: u32 = 2709;

/// Which side of the line a..b the point p is on.
fn side(a: Vec3, b: Vec3, p: Vec3) -> bool {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) >= 0.0
}

/// How far p is from the segment a..b, across the ground.
pub(crate) fn seg_dist(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1.0e-6 { 0.0 } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) };
    let (qx, qy) = (a[0] + dx * t, a[1] + dy * t);
    ((p[0] - qx).powi(2) + (p[1] - qy).powi(2)).sqrt()
}
