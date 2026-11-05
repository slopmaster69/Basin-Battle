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
/// How it looks stuck in the ground: a big brown feather, the fletching standing up, at its
/// model's own size (at 0.45 it was too small to tell from the ground in a fight).
pub const PLANT_LOOK: u32 = 2630;
pub const PLANT_SCALE: f32 = 1.0;
/// A tripwire trips anything within this of the wire.
const WIRE_REACH: f32 = 0.7;
/// The most a tripwire spans: its second arrow plants within this of the first (the ring shown
/// round the first while the key waits for the second).
pub const WIRE_SPAN: f32 = 10.0;
/// A tripwire's two arrows stay in the ground this long, snapped or not: past Tripwire's own
/// cooldown, so Arrow Recall (the same key) can bring them home once it's back.
pub const ANCHOR_SECS: f32 = 18.0;
/// Call of the Wild: the chain falls back to its first stage after this long without a call.
pub const WILD_SECS: f32 = 4.0;
/// The stamina each stage costs (half when empowered), and the damage each stage's beast does.
/// (Play-test: at 15 / 20 / 25 a hero cast it 5 to 8 times from range and never thought about
/// stamina. At 25 / 30 / 35 a whole chain is 90 of the 100 stamina: the finisher leaves no dodge
/// unless a parry or a perfect dodge paid for half of a call.)
pub const WILD_STAMINA: [f32; 3] = [25.0, 30.0, 35.0];
pub const WILD_DAMAGE: [f32; 3] = [12.0, 18.0, 30.0];
/// Empowered (a parry or a perfect dodge in the last `WILD_PRIMED_SECS`): this much more damage.
const WILD_EMPOWERED: f32 = 1.25;
/// A parry or a perfect dodge empowers the next call this long. (Play-test: a landed hit or an
/// arrow empowered it too, so from range every call was half price. Now it pays for the core's
/// own moves, and only for a few seconds: answer the parry with a beast.)
pub const WILD_PRIMED_SECS: f32 = 4.0;
/// A beast appears near its target and charges it, striking as it arrives this long after the
/// call (Corey's review: at 0.3 sec, out of the hunter, it was on the foe before anyone could see
/// which beast came), then stays this long after its strike, so who came and what it did can be seen
/// (play-test: at 0.9 sec they were gone before anyone could tell), then goes in the flash it
/// came in.
const POUNCE_SECS: f32 = 0.6;
const BEAST_STAYS: f32 = 1.8;
/// How far round the foe from the near side the beast strikes from (radians): about a right
/// angle, so it stands beside the foe, clear of the hunter's back.
const BEAST_FLANK: f32 = 1.4;
/// Where a beast appears: this far from its foe (yards), this far round from the hunter's side of
/// it (radians), so it charges in across the camera's view.
const BEAST_RUN_UP: f32 = 6.0;
const BEAST_START: f32 = 1.0;
/// It roars (or howls, or hisses: its model's own roar) this long after its strike.
const BEAST_ROAR_AFTER: f32 = 0.5;
/// `EMOTE_ONESHOT_ROAR`.
const BEAST_ROAR: u32 = 15;
/// A lit beast takes a foe that qualifies within this of the reticle's foe instead.
const LIT_REACH: f32 = 8.0;
/// No lock: the call goes for the foe the camera looks at, within this of the view's centre
/// line (radians), by the line the client sent with the press (this old at most: a press kept
/// for the end of a swing still goes where the camera was).
const CALL_CONE: f32 = 0.6;
const CALL_AIM_FRESH: f32 = 2.0;
/// Arrow Recall: a foe struck by this many arrows or more is Lacerated, a second longer for each
/// past it.
const LACERATE_AT: u32 = 3;
const LACERATE_DPS: f32 = 4.0;
const LACERATE_SECS: f32 = 4.0;
/// A bleed's look (Rend's).
const BLEED_AURA: u32 = 772;
/// Arrow Recall's hand: the arm goes out at the press (a held pose, `Emotes.dbc` STATE_POINT) and
/// the arrows leave the ground this much later, into it; it comes down once the last is home
/// (Corey's review: the hand came out late, after the arrows, with the one-shot point).
pub const RECALL_POSE: u32 = 29;
pub const RECALL_HAND_SECS: f32 = 0.25;
/// Trapper's Instinct: the arrows scattered where the hunter stood, and how far round.
const SCATTER: u32 = 5;
const SCATTER_RING: f32 = 1.6;
/// Skyfall's arrows come down over this long.
const VOLLEY_SECS: f32 = 1.0;
/// Skyfall's hang: held at the leap's top this long, the bow drawn on the volley. The add-on
/// looses it as it runs out; the rules a moment after (`HANG_GRACE`), at the last reticle heard.
const FLOAT_SECS: f32 = 5.5;
const HANG_GRACE: f32 = 0.5;
/// The pose held through the hang: the Jump clip (38), a character's mid-air pose with the legs
/// drawn up (1.12 has no hover of its own), played on the lower body only for the whole hang
/// (`ClipBody::Lower`): the arms stay on the bow's draw.
const HOVER_CLIP: u16 = 38;
/// After the volley: feather fall, untouchable, till they land (or this long at most).
const DESCENT_SECS: f32 = 3.0;
/// Each of its arrows strikes everyone within this of where it lands.
const VOLLEY_SPLASH: f32 = 2.5;

/// One of Call of the Wild's beasts: its creature (a `[[mob]]` with `totem = true`), its icon on
/// the key (a 1.12 icon of that animal), its stage (0 sets up, 1 exploits, 2 finishes), and the
/// spell whose look its strike plays from it to its foe (its own pet ability's, in 1.12).
pub struct Beast {
    pub key: &'static str,
    pub name: &'static str,
    pub mob: &'static str,
    pub icon: &'static str,
    pub stage: usize,
    pub strike: u32,
}

/// Three a stage, in stage order. (No 1.12 icon shows a basilisk or a thunder lizard: the
/// basilisk wears the crocolisk's, the nearest beast, and the lightning finisher is a wind
/// serpent, Lightning Breath's own beast, where an Ability_ThunderBolt hammer stood before.)
pub const BEASTS: [Beast; 9] = [
    // Bite.
    Beast { key: "wolf", name: "Wolf", mob: "spirit_wolf", icon: "Ability_Hunter_Pet_Wolf", stage: 0, strike: 17253 },
    // Screech.
    Beast { key: "owl", name: "Owl", mob: "spirit_owl", icon: "Ability_Hunter_Pet_Owl", stage: 0, strike: 24423 },
    // Scorpid Poison.
    Beast { key: "scorpid", name: "Scorpid", mob: "spirit_scorpid", icon: "Ability_Hunter_Pet_Scorpid", stage: 0, strike: 24640 },
    // Thunderstomp.
    Beast { key: "gorilla", name: "Gorilla", mob: "spirit_gorilla", icon: "Ability_Hunter_Pet_Gorilla", stage: 1, strike: 26090 },
    // Claw.
    Beast { key: "raptor", name: "Raptor", mob: "spirit_raptor", icon: "Ability_Hunter_Pet_Raptor", stage: 1, strike: 16827 },
    // Crystal Gaze.
    Beast { key: "basilisk", name: "Basilisk", mob: "spirit_basilisk", icon: "Ability_Hunter_Pet_Crocolisk", stage: 1, strike: 3635 },
    // Maul.
    Beast { key: "bear", name: "Bear", mob: "spirit_bear", icon: "Ability_Hunter_Pet_Bear", stage: 2, strike: 6807 },
    // War Stomp.
    Beast { key: "kodo", name: "Kodo", mob: "spirit_kodo", icon: "Ability_Mount_Kodo_01", stage: 2, strike: 20549 },
    // Lightning Breath.
    Beast { key: "wind_serpent", name: "Wind Serpent", mob: "spirit_wind_serpent", icon: "Ability_Hunter_Pet_WindSerpent", stage: 2, strike: 24844 },
];

/// An arrow stuck in the ground.
#[derive(Debug, Clone)]
pub struct Planted {
    pub owner: Id,
    pub pos: Vec3,
    pub until: f32,
    pub obj: Id,
    /// Its ring on the ground.
    pub mark: Id,
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

/// An Arrow Recall under way: the hand went out at the press; the arrows (taken off the ground's
/// list then, so nothing else moves them) fly at `at`, and the hand comes down at `release_at`,
/// once the last is home (set as they fly).
#[derive(Debug, Clone)]
pub(crate) struct Recall {
    pub owner: Id,
    pub ability: usize,
    pub at: f32,
    pub arrows: Vec<Planted>,
    pub release_at: f32,
}

/// A tripwire's first arrow, planted, waiting for the key again to place the second: the ability
/// that laid it (its wire's damage, life and effects), where it stands, and the ring round it.
#[derive(Debug, Clone)]
pub(crate) struct FirstArrow {
    pub ability: usize,
    pub at: Vec3,
    pub ring: Id,
}

/// A hero's Call of the Wild chain: the stage the next call is, the beast drawn for it, until
/// when the chain holds, and whether the next call is empowered (as [`Survival::wild_now`] has
/// it: until `primed_until`).
#[derive(Debug, Clone, Default)]
pub struct Wild {
    pub stage: usize,
    pub next: usize,
    pub until: f32,
    pub primed: bool,
    pub drawn: bool,
    pub primed_until: f32,
}

/// A beast on its way to its target: it strikes at `at`, roars, and its body goes at `gone`.
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
    pub roared: bool,
}

/// One of Skyfall's arrows coming down.
#[derive(Debug, Clone)]
pub(crate) struct Arrowfall {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub land_at: f32,
}

/// A hero on Skyfall: the leap and its volley, when the leap tops out and where, and until when
/// they hang there (`held`: up there now, held still, aiming).
#[derive(Debug, Clone)]
pub(crate) struct Hang {
    pub owner: Id,
    pub leap: usize,
    pub volley: usize,
    pub top_at: f32,
    pub top: Vec3,
    pub until: f32,
    pub held: bool,
}

/// The Hunter's state in the world.
#[derive(Debug, Clone, Default)]
pub struct Hunt {
    pub planted: Vec<Planted>,
    pub(crate) wires: Vec<Wire>,
    /// Tripwires half laid: each hero's first arrow, until the key's window for the second closes.
    pub(crate) first_arrows: BTreeMap<Id, FirstArrow>,
    pub wild: BTreeMap<Id, Wild>,
    pub(crate) pounces: Vec<Pounce>,
    pub(crate) falls: Vec<Arrowfall>,
    /// Arrow Recalls pressed, their arrows yet to fly or their hand yet to come down.
    pub(crate) recalls: Vec<Recall>,
    /// Recalled arrows' looks on their way home: each one's unseen anchor, and when it's home.
    pub(crate) anchors: Vec<(Id, f32)>,
    /// Heroes floating down after Skyfall's volley (feather fall), until then at most.
    pub(crate) floating: Vec<(Id, f32)>,
    /// Heroes on Skyfall: going up, or hanging over the fight.
    pub(crate) hangs: Vec<Hang>,
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
        // No ring under it: the arrow standing in the ground shows where it is (Corey's review:
        // a hero's own circles are floor clutter; a dozen of these lay at once).
        let mark = self.mark_unseen(secs);
        self.hunt.planted.push(Planted { owner, pos, until: now + secs, obj, mark });
    }

    fn unplant(&mut self, a: &Planted) {
        if self.objs.remove(&a.obj).is_some() {
            self.events.push(Event::ObjGone { id: a.obj });
        }
        self.unmark(a.mark);
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
        if !self.hunt.first_arrows.is_empty() {
            self.step_first_arrows();
        }
        if !self.hunt.pounces.is_empty() {
            self.step_pounces();
        }
        if !self.hunt.hangs.is_empty() {
            self.step_hangs();
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
        // After the anchors: the hand comes down in the tick the last arrow is home, after it.
        if !self.hunt.recalls.is_empty() {
            self.step_recalls();
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

    /// Tripwire's first press plants one arrow at the reticle and arms the key for the second (its
    /// recast); the second press (the recast, `inner`) plants the other within [`WIRE_SPAN`] of it
    /// and the wire forms between them.
    fn lay_tripwire(&mut self, id: Id, ability: usize, share: f32) {
        let d = self.content.abilities[ability].clone();
        if d.inner {
            self.tie_tripwire(id, ability, share);
            return;
        }
        let at = self.aim_point(id, AbilityDef::at(d.range, share));
        self.events.push(Event::Missile { caster: id, spell: 19434, target: Target::Point(at) });
        self.plant_arrow_for(id, at, ANCHOR_SECS);
        if let Some(old) = self.hunt.first_arrows.remove(&id) {
            self.unmark(old.ring);
        }
        // How long the key waits for the second (a little past, while it's aimed). No ring is
        // drawn round the first for how far the wire spans (Corey's review: floor clutter); a
        // second aimed past the span still lands on its edge (`within_span`).
        let window = d.recast_secs.max(0.5);
        let ring = self.mark_unseen(window + 10.0);
        self.hunt.first_arrows.insert(id, FirstArrow { ability, at, ring });
    }

    /// The second arrow: within [`WIRE_SPAN`] of the first (aimed past it, at that distance), and
    /// the wire between the two.
    fn tie_tripwire(&mut self, id: Id, ability: usize, share: f32) {
        let now = self.clock;
        let range = AbilityDef::at(self.content.abilities[ability].range, share);
        let aimed = self.aim_point(id, range);
        let Some(first) = self.hunt.first_arrows.remove(&id) else {
            // The first is gone (its window closed as this went): just an arrow.
            self.events.push(Event::Missile { caster: id, spell: 19434, target: Target::Point(aimed) });
            self.plant_arrow_for(id, aimed, ANCHOR_SECS);
            return;
        };
        self.unmark(first.ring);
        let a = first.at;
        let to = within_span(a, aimed);
        let b = if dist2d(to, aimed) > 0.01 { self.ground_near(to[0], to[1], aimed[2].max(a[2]) + 1.0) } else { aimed };
        self.events.push(Event::Missile { caster: id, spell: 19434, target: Target::Point(b) });
        self.plant_arrow_for(id, b, ANCHOR_SECS);
        // One wire a hunter.
        let old: Vec<Wire> = self.hunt.wires.iter().filter(|w| w.owner == id).cloned().collect();
        self.hunt.wires.retain(|w| w.owner != id);
        for w in old {
            self.unmark(w.mark);
        }
        let life = self.content.abilities[first.ability].secs.max(1.0);
        let mid = [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0, (a[2] + b[2]) / 2.0];
        // The wire on the ground: a strip from the first arrow to the second (its length along
        // the way it faces, its width across).
        let (yaw, len) = wire_line(a, b);
        let tele = Telegraph { own_hidden: false, ..Telegraph::hero_shape(id, len, 0.0, WIRE_REACH * 2.0, life, Some((a, yaw))) };
        let mark = self.mark_as(mid, WIRE_REACH, life, Some(tele));
        // And the wire itself, a rope between the two arrows, gone with the strip.
        self.events.push(Event::Rope { id: mark, a, b });
        let sides = self.wire_sides(a, b, id);
        self.hunt.wires.push(Wire { owner: id, ability: first.ability, a, b, until: now + life, sides, mark });
    }

    /// First arrows whose key no longer waits for the second (the window closed, the hero fell or
    /// left): just planted arrows now, their rings gone.
    fn step_first_arrows(&mut self) {
        let lapsed: Vec<Id> = self.hunt.first_arrows.iter().filter(|(h, f)| self.recast_armed(**h, self.content.base_of(f.ability)).is_none()).map(|(h, _)| *h).collect();
        for h in lapsed {
            if let Some(f) = self.hunt.first_arrows.remove(&h) {
                self.unmark(f.ring);
            }
        }
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
            .filter(|(m, x)| x.health > 0 && !x.hidden && !self.content.mobs[x.def].totem && !self.own_totem(**m, owner) && dist2d(x.pos, at) <= reach)
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
    /// No word says so: the rope snaps and the foe falls (Corey's rule). The fall's own dust is
    /// the wire's knockdown's ([`knock_down`]): nothing else is played here, so it comes up once.
    fn trip(&mut self, owner: Id, v: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let Some(at) = self.unit_pos(v) else { return };
        self.hunt_hit(owner, v, ability, AbilityDef::at(d.damage, 1.0), at);
        if self.holds_ground(v) {
            self.build_posture(v, owner, 40);
        }
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
            self.events.push(Event::Visual { id: victim, kit: KIT_FALL_DUST, by: None });
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
    /// (each once, at full damage); three arrows or more in one foe leave it Lacerated. The hand
    /// goes out at the press; the arrows follow it a moment later ([`fly_recall`]), and it comes
    /// down once the last is home ([`step_recalls`]).
    fn recall_arrows(&mut self, id: Id, ability: usize) {
        if !self.players.contains_key(&id) {
            return;
        }
        let mine: Vec<Planted> = self.hunt.planted.iter().filter(|a| a.owner == id).cloned().collect();
        self.hunt.planted.retain(|a| a.owner != id);
        if mine.is_empty() {
            self.events.push(Event::Flash { player: id, text: "No arrows planted".into() });
            return;
        }
        self.events.push(Event::Pose { id, pose: RECALL_POSE });
        self.hunt.recalls.push(Recall { owner: id, ability, at: self.clock + RECALL_HAND_SECS, arrows: mine, release_at: f32::INFINITY });
    }

    /// Recalls pressed: their arrows fly at their time, and the hand comes down once the last is
    /// home, or at once if the hunter is gone (dead or left) before then, their arrows with it.
    fn step_recalls(&mut self) {
        let now = self.clock;
        let recalls = std::mem::take(&mut self.hunt.recalls);
        let mut keep = Vec::new();
        for mut r in recalls {
            if !self.alive(r.owner) {
                for a in &r.arrows {
                    self.unplant(a);
                }
                self.events.push(Event::Pose { id: r.owner, pose: 0 });
                continue;
            }
            if !r.arrows.is_empty() && r.at <= now {
                let arrows = std::mem::take(&mut r.arrows);
                r.release_at = self.fly_recall(r.owner, r.ability, &arrows);
            }
            if r.arrows.is_empty() && r.release_at <= now {
                self.events.push(Event::Pose { id: r.owner, pose: 0 });
                continue;
            }
            keep.push(r);
        }
        keep.extend(std::mem::take(&mut self.hunt.recalls));
        self.hunt.recalls = keep;
    }

    /// The arrows fly home: each leaves the ground and strikes what it passes through on the way;
    /// the time the last is home.
    fn fly_recall(&mut self, id: Id, ability: usize, arrows: &[Planted]) -> f32 {
        let d = self.content.abilities[ability].clone();
        let Some(home) = self.players.get(&id).map(|p| p.pos) else { return self.clock };
        let radius = AbilityDef::at(d.radius, 1.0).max(0.4);
        let damage = AbilityDef::at(d.damage, 1.0);
        let mut struck: BTreeMap<Id, u32> = BTreeMap::new();
        let mut last_home = self.clock;
        for a in arrows {
            self.unplant(a);
            // Its look, flown home from where it stood.
            let anchor = self.alloc();
            let from = [a.pos[0], a.pos[1], a.pos[2] + 0.8];
            self.events.push(Event::MissileFrom { anchor, caster: id, spell: d.spell, from, target: Target::Unit(id) });
            let arrival = self.clock + dist2d(from, home) / d.speed.max(10.0) + 0.2;
            self.hunt.anchors.push((anchor, arrival));
            last_home = last_home.max(arrival);
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
                // The bleed's own look on the foe says it, no word.
                self.apply_cc_look(v, id, "burn", secs, LACERATE_DPS, 0.0, "Lacerate", BLEED_AURA);
            }
        }
        last_home
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

    /// Skyfall's leap: up to its top, then held there over the fight with the bow drawn, aiming
    /// its volley (its `recast`) down at the ground.
    pub(crate) fn skyfall_float(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let Some(volley) = self.content.ability(&d.recast) else { return };
        let Some(pos) = self.players.get(&id).map(|p| p.pos) else { return };
        // Straight up: the top is where the leap's thrown speed runs out.
        let lift = d.lift.max(0.5);
        let rise = super::moves::airtime(lift) / 2.0;
        let top = [pos[0], pos[1], pos[2] + lift * rise / 2.0];
        // It comes down when the volley's gone, not when the leap would have.
        self.landings.retain(|l| l.owner != id);
        self.hunt.floating.retain(|(h, _)| *h != id);
        self.hunt.hangs.retain(|h| h.owner != id);
        self.hunt.hangs.push(Hang { owner: id, leap: ability, volley, top_at: now + rise, top, until: now + rise + FLOAT_SECS, held: false });
    }

    /// Hanging at Skyfall's top now, held still in the air.
    pub(crate) fn hanging(&self, id: Id) -> bool {
        self.hunt.hangs.iter().any(|h| h.owner == id && h.held)
    }

    /// Up on Skyfall, from the leap till they land again: nothing touches them, no control takes.
    pub(crate) fn skyfall_aloft(&self, id: Id) -> bool {
        self.hunt.hangs.iter().any(|h| h.owner == id) || self.hunt.floating.iter().any(|(h, _)| *h == id)
    }

    /// How long they've hung and how long the hang is (the add-on's bar, and its own loose as it
    /// runs out).
    pub fn hang_time(&self, id: Id) -> Option<(f32, f32)> {
        let h = self.hunt.hangs.iter().find(|h| h.owner == id && h.held)?;
        Some(((self.clock - (h.until - FLOAT_SECS)).max(0.0), FLOAT_SECS))
    }

    /// Skyfall's leap tops out (held there, the bow drawn on the volley), or its hang runs out (the
    /// volley goes at the reticle as last heard); a fallen hero's hang ends.
    fn step_hangs(&mut self) {
        let now = self.clock;
        for h in self.hunt.hangs.clone() {
            if !self.players.get(&h.owner).is_some_and(|p| p.health > 0) {
                self.end_hang(h.owner);
            } else if !h.held && now >= h.top_at {
                self.hold_hang(&h);
            } else if h.held && now >= h.until + HANG_GRACE {
                if let Some(p) = self.players.get_mut(&h.owner) {
                    if let Some((f, e, _)) = p.aim_line {
                        p.aim_line = Some((f, e, now));
                    }
                }
                self.loose_volley(h.owner);
            }
        }
    }

    /// The top of Skyfall's leap: held there, the bow drawn, the volley's circle aimed.
    fn hold_hang(&mut self, h: &Hang) {
        let id = h.owner;
        if let Some(x) = self.hunt.hangs.iter_mut().find(|x| x.owner == id) {
            x.held = true;
        }
        // Where they hang (their client's last word may be from on the way up).
        if let Some(p) = self.players.get_mut(&id) {
            p.pos[2] = p.pos[2].max(h.top[2]);
        }
        self.update_rooted(id);
        self.events.push(Event::FeatherFall { player: id, on: true });
        self.set_action(id, Action::Aim { ability: h.volley });
        self.set_kit(id, Kit::Bow);
        self.draw_bow(id);
        // Hung in the air in a hover's pose, not standing on nothing (Corey's review: "it looks
        // like they're just standing in the air. The legs are the problem"): the Jump clip's
        // mid-air pose, the legs drawn up, held for the whole hang. On the legs only: the arms
        // keep drawing the bow (his second review: a whole-body hold "ruined" it, "his arms still
        // need to be drawing a bow"). Let go as the hang ends (`end_hang`).
        self.events.push(Event::Clip { id, clip: HOVER_CLIP, take: 0, from: None, secs: FLOAT_SECS + HANG_GRACE + 0.5, at: 0.0, body: ClipBody::Lower });
        self.events.push(Event::Abilities { player: id });
    }

    /// Skyfall's hang over (its volley gone, or the hero fallen): let go, and down slowly,
    /// untouchable till they land.
    fn end_hang(&mut self, id: Id) {
        let was = self.hunt.hangs.len();
        self.hunt.hangs.retain(|h| h.owner != id);
        if self.hunt.hangs.len() == was {
            return;
        }
        self.update_rooted(id);
        self.hunt.floating.retain(|(h, _)| *h != id);
        self.hunt.floating.push((id, self.clock + DESCENT_SECS));
        self.events.push(Event::FeatherFall { player: id, on: true });
        // The hover's pose let go: the legs come down for the fall.
        self.events.push(Event::ClipStop { id });
    }

    /// Down from Skyfall: the slow fall ends as they land.
    pub(crate) fn skyfall_landed(&mut self, id: Id) {
        if self.hunt.floating.iter().any(|(h, _)| *h == id) {
            self.hunt.floating.retain(|(h, _)| *h != id);
            self.events.push(Event::FeatherFall { player: id, on: false });
        }
    }

    /// The volley loosed from the hang.
    fn loose_volley(&mut self, id: Id) {
        let Some(volley) = self.hunt.hangs.iter().find(|h| h.owner == id && h.held).map(|h| h.volley) else { return };
        self.fire(id, volley, 1.0);
        // (However it went, the hang is over.)
        self.end_hang(id);
    }

    /// An input on Skyfall (true: it's taken): up there, the click or the key again looses the
    /// volley, and nothing else does anything; on the way up, nothing at all.
    pub(crate) fn hang_input(&mut self, id: Id, input: Input) -> bool {
        let Some(h) = self.hunt.hangs.iter().find(|h| h.owner == id).cloned() else { return false };
        let own_key = match input {
            Input::Ability(slot, _) => crate::content::SLOTS
                .get(slot as usize)
                .and_then(|s| self.players.get(&id).and_then(|p| p.loadout.get(*s)))
                .and_then(|k| self.content.ability(k))
                .is_some_and(|a| a == self.content.base_of(h.leap)),
            _ => false,
        };
        if h.held && (own_key || matches!(input, Input::Attack | Input::Heavy)) {
            self.loose_volley(id);
        }
        true
    }

    /// Skyfall's volley: from the hero, its arrows come down over a second, scattered over the
    /// circle at the reticle; then down they float.
    fn skyfall_volley(&mut self, id: Id, ability: usize, share: f32) {
        self.end_hang(id);
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let range = AbilityDef::at(d.range, share);
        let radius = AbilityDef::at(d.radius, share);
        let at = self.aim_point(id, range);
        let n = d.count.max(1);
        let tele = Telegraph::hero_shape(id, radius, 360.0, 0.0, VOLLEY_SECS, Some((at, 0.0)));
        self.mark_as(at, radius, VOLLEY_SECS + 0.2, Some(tele));
        for k in 0..n {
            // Spread evenly over the circle (a sunflower's spiral), each a beat after the last;
            // the first lands on the aimed spot itself (Corey's review: "the volley doesn't seem
            // to go where the marker is": no arrow ever fell within 1.2 yd of its centre).
            let r = radius * (k as f32 / n as f32).sqrt();
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
        w.primed = w.primed && w.primed_until > self.clock;
        w
    }

    /// The beast the hero's next call brings, for the key's icon.
    pub fn next_beast(&self, id: Id) -> &'static Beast {
        &BEASTS[self.wild_now(id).next]
    }

    /// How long the hero's next call stays empowered (0: it isn't), for the key's look.
    pub fn wild_primed_left(&self, id: Id) -> f32 {
        let w = self.wild_now(id);
        if w.primed {
            (w.primed_until - self.clock).max(0.0)
        } else {
            0.0
        }
    }

    /// Something the hero did that empowers their next call for [`WILD_PRIMED_SECS`]: a parry,
    /// a perfect dodge (a hero with Call of the Wild on a key, before their first call too).
    pub(crate) fn prime_wild(&mut self, id: Id) {
        let now = self.clock;
        let calls = self.players.get(&id).is_some_and(|p| p.loadout.values().any(|k| self.content.ability(k).is_some_and(|a| matches!(self.content.abilities[a].hunt.as_str(), "call" | "double_call"))));
        if !calls {
            return;
        }
        let w = self.hunt.wild.entry(id).or_default();
        w.primed = true;
        w.primed_until = now + WILD_PRIMED_SECS;
        self.events.push(Event::Abilities { player: id });
    }

    /// Whether `mid` is a beast's body (never struck, never in the way, never locked on).
    pub fn spirit_body(&self, mid: Id) -> bool {
        self.hunt.pounces.iter().any(|p| p.body == Some(mid))
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
            // No beast went: its recharge isn't spent either.
            let base = self.content.base_of(ability);
            if let Some(p) = self.players.get_mut(&id) {
                p.cooldowns.remove(&base);
            }
            self.events.push(Event::Abilities { player: id });
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
        self.hunt.wild.insert(id, Wild { stage, next, until: now + WILD_SECS, primed: false, drawn: true, primed_until: 0.0 });
        self.events.push(Event::Abilities { player: id });
    }

    /// Whom a beast goes for: the hero's lock, else the foe the camera looks at (one press, no
    /// aiming: [`Self::called_at`]); a beast whose bonus a foe near that one fits goes for that
    /// one instead, unless the aimed one fits.
    fn beast_target(&self, id: Id, range: f32, beast: usize) -> Option<Id> {
        let p = self.players.get(&id)?;
        let pos = p.pos;
        let foes = self.foes_near(id, pos, range);
        let locked = p.lock.filter(|v| foes.iter().any(|(f, _)| f == v));
        let aimed = locked.or_else(|| self.called_at(id, &foes, range))?;
        if self.beast_lit(beast, aimed) {
            return Some(aimed);
        }
        let at = self.unit_pos(aimed)?;
        let other = self.foes_near(id, at, LIT_REACH).into_iter().filter(|(v, vat)| *v != aimed && dist2d(*vat, pos) <= range && self.beast_lit(beast, *v)).min_by(|a, b| dist2d(a.1, at).total_cmp(&dist2d(b.1, at)));
        Some(other.map_or(aimed, |(v, _)| v))
    }

    /// The foe the camera looks at, by the line the client sent with the press (the hero's
    /// facing without one): the first of `foes` that line meets from the hero's head, else the
    /// one nearest the line within [`CALL_CONE`] of it, in sight.
    fn called_at(&self, id: Id, foes: &[(Id, Vec3)], range: f32) -> Option<Id> {
        let p = self.players.get(&id)?;
        let pos = p.pos;
        let (yaw, pitch) = match p.aim_line {
            Some((f, e, at)) if self.clock - at <= CALL_AIM_FRESH => (f, e),
            _ => (p.facing, 0.0),
        };
        let head = [pos[0], pos[1], pos[2] + 1.7];
        let met = self.trace(head, look_dir(yaw, pitch), range + 3.0, 0.4, id).unit.filter(|u| foes.iter().any(|(f, _)| f == u));
        if met.is_some() {
            return met;
        }
        let (c, s) = (yaw.cos(), yaw.sin());
        // How far off the line a foe stands (yards across it), a little for how far it is.
        let off = |at: Vec3| (-(at[0] - pos[0]) * s + (at[1] - pos[1]) * c).abs() + dist2d(pos, at) * 0.02;
        foes.iter()
            .filter(|(_, at)| angle_between(yaw, facing(pos, *at)) <= CALL_CONE && clear_line(self.land.as_ref(), pos, *at))
            .min_by(|a, b| off(a.1).total_cmp(&off(b.1)))
            .map(|(v, _)| *v)
    }

    /// Whether a beast's bonus fits `v` now.
    pub fn beast_lit(&self, beast: usize, v: Id) -> bool {
        let now = self.clock;
        let staggered = self.mobs.get(&v).is_some_and(|m| m.stagger_until > now);
        match BEASTS[beast].key {
            "wolf" => self.mobs.get(&v).is_some_and(|m| matches!(m.state, MobState::Windup { .. })),
            "owl" => true,
            "scorpid" => self.has_cc(v, "mark") || self.has_cc(v, "sunder"),
            "gorilla" => self.unit_pos(v).is_some_and(|at| self.mobs.values().filter(|m| m.health > 0 && !self.content.mobs[m.def].inert() && dist2d(m.pos, at) <= 6.0).count() >= 3),
            "raptor" => self.has_cc(v, "burn"),
            "basilisk" => self.has_cc(v, "snare"),
            "bear" => staggered || self.has_cc(v, "stun") || self.has_cc(v, "petrify") || self.stance_broken(v),
            "kodo" => self.has_cc(v, "mark"),
            "wind_serpent" => self.mobs.get(&v).is_some_and(|m| m.guarding) || self.players.get(&v).is_some_and(|p| matches!(p.action, Action::Guard)),
            _ => false,
        }
    }

    /// A beast bursts out of the ground near `target`, to one side of it, and charges it (Corey's
    /// review: not out of the hunter; it comes in close and rushes the foe).
    fn send_beast(&mut self, id: Id, ability: usize, beast: usize, target: Id, empowered: bool, delay: f32) {
        let now = self.clock;
        let Some(from) = self.players.get(&id).map(|p| p.pos) else { return };
        let Some(to) = self.unit_pos(target) else { return };
        let f = facing(from, to);
        let side = if self.rng.gen::<bool>() { 1.0 } else { -1.0 };
        // It strikes from the foe's flank on its own side, not from the near side: there it stood
        // between the hunter and the foe, behind her back from the action camera (run 015: the
        // first two beasts of a chain were never seen).
        let flank = f + std::f32::consts::PI - side * BEAST_FLANK;
        let end = [to[0] + flank.cos() * 1.5, to[1] + flank.sin() * 1.5, to[2]];
        // It appears a run-up out from that flank, off to the side and a little toward the hunter
        // (in the camera's view), never farther out than most of the way to her.
        let run_up = BEAST_RUN_UP.min(dist2d(from, to) * 0.6).max(2.0);
        let out = f + std::f32::consts::PI - side * BEAST_START;
        let start = self.ground_near(to[0] + out.cos() * run_up, to[1] + out.sin() * run_up, to[2] + 1.0);
        let body = self.content.mob(BEASTS[beast].mob).map(|def| {
            let mid = self.spawn_mob(def, start, None, false);
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.facing = facing(end, to);
            }
            mid
        });
        if let Some(b) = body {
            let dist = dist2d(start, end).max(0.5);
            self.events.push(Event::MobMove { id: b, path: vec![start, end], speed: dist / (POUNCE_SECS + delay).max(0.1), face: None });
            // Where it stands from its strike on (for whoever comes into sight of it later).
            if let Some(m) = self.mobs.get_mut(&b) {
                m.pos = end;
            }
        }
        self.events.push(Event::Missile { caster: id, spell: SPIRIT_FLASH, target: Target::Point(start) });
        self.hunt.pounces.push(Pounce { owner: id, ability, beast, target, at: now + POUNCE_SECS + delay, gone: now + POUNCE_SECS + delay + BEAST_STAYS, empowered, body, struck: false, roared: false });
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
                    // Its own pet ability's look, from it onto the foe: which beast, what it did.
                    if self.alive(p.target) {
                        self.events.push(Event::Missile { caster: b, spell: BEASTS[p.beast].strike, target: Target::Unit(p.target) });
                    }
                }
                if self.alive(p.target) && self.players.contains_key(&p.owner) {
                    self.beast_strikes(&p);
                }
            }
            if p.struck && !p.roared && now >= p.at + BEAST_ROAR_AFTER {
                p.roared = true;
                if let Some(b) = p.body.filter(|b| self.mobs.contains_key(b)) {
                    self.events.push(Event::Emote { id: b, emote: BEAST_ROAR });
                }
            }
            if p.gone <= now {
                if let Some(b) = p.body {
                    // Gone in the flash it came in (laid on the ground where it stood, so it plays
                    // with the body already gone).
                    if let Some(at) = self.mobs.get(&b).map(|m| m.pos).filter(|_| self.players.contains_key(&p.owner)) {
                        self.events.push(Event::Missile { caster: p.owner, spell: SPIRIT_FLASH, target: Target::Point(at) });
                    }
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
        // Its bonus fits: the blow lands with a heavier impact on the foe (Slam's), no word.
        // Which beast it was is plain from the beast itself.
        if lit {
            self.events.push(Event::Visual { id: v, kit: combat::KIT_SLAM, by: None });
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
            "wind_serpent" => {
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
/// The Wind Serpent's shock on what it strikes (Thunderclap's).
const THUNDER_KIT: u32 = 2709;

/// Which side of the line a..b the point p is on.
fn side(a: Vec3, b: Vec3, p: Vec3) -> bool {
    (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]) >= 0.0
}

/// Where a tripwire's second arrow goes, aimed at `at` with the first at `a`: there, or on the
/// span's end ([`WIRE_SPAN`] from the first) when aimed past it.
pub(crate) fn within_span(a: Vec3, at: Vec3) -> Vec3 {
    let d = dist2d(a, at);
    if d <= WIRE_SPAN {
        return at;
    }
    let k = WIRE_SPAN / d;
    [a[0] + (at[0] - a[0]) * k, a[1] + (at[1] - a[1]) * k, at[2]]
}

/// A wire's line on the ground from `a` to `b`: the way it faces (WoW's facing, from +x toward
/// +y) and its length.
pub(crate) fn wire_line(a: Vec3, b: Vec3) -> (f32, f32) {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let yaw = if dx.abs() + dy.abs() < 1.0e-4 { 0.0 } else { dy.atan2(dx) };
    (yaw.rem_euclid(std::f32::consts::TAU), (dx * dx + dy * dy).sqrt())
}

/// How far p is from the segment a..b, across the ground.
pub(crate) fn seg_dist(a: Vec3, b: Vec3, p: Vec3) -> f32 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 < 1.0e-6 { 0.0 } else { (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0) };
    let (qx, qy) = (a[0] + dx * t, a[1] + dy * t);
    ((p[0] - qx).powi(2) + (p[1] - qy).powi(2)).sqrt()
}
