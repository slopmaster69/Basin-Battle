//! Gilnid's finite workforce. Ledges, throws, falls, the valve and his finish share one encounter
//! clock. Neither a health threshold nor a hero's loadout can create another worker.
//!
//! The beats (docs/gilnid-encounter.md): two engineers stand on planks laid from the Foundry
//! ramp's rail out over the pit, each plank on a pole from the floor. One throws: a 3 sec fuse lit
//! in his hand, a stick thrown in an arc that sits sparking a second before it blows. Gilnid holds
//! his heavy blows while one is live. After the first stick the furnace spills onto the first
//! engineer's pole and sets it alight, and he falls in plain view, lands sitting and gets up to
//! fight. The second comes down to one ranged hit, a blow on him or his pole, or climbs down after
//! three throws. With the workers down Gilnid works the furnace's own valve 2 sec, steam runs
//! along a channel and a stripe of fire crosses the pit with dry ground either side. Then a
//! flurry of three that plants his spanner in the floor, open 2.5 sec, until he falls.

use std::collections::VecDeque;

use super::mobs::{Foe, MobState};
use super::*;
use crate::content::PatternDef;
use crate::land::clear_line;

#[cfg(test)]
#[path = "perches_tests.rs"]
mod tests;

/// The plank (DarkIronWoodPlanks07): its long axis is the model's X, from -1.69 to 0.87, its
/// width the model's Y from -0.32 to 0.80, 0.22 thick. Laid with its back end on the rail.
const PLANK: u32 = 6481;
const PLANK_SCALE: f32 = 1.8;
const PLANK_BACK: f32 = 1.69 * PLANK_SCALE;
const PLANK_SIDE: f32 = (0.80 - 0.32) / 2.0 * PLANK_SCALE;
const PLANK_THICK: f32 = 0.22 * PLANK_SCALE;
/// The support (ExcavationBarrierPole, a timber pole with an iron foot): 23.87 above its origin,
/// scaled so its top meets the plank.
const POLE: u32 = 1508;
const POLE_HEIGHT: f32 = 23.87;
/// The engineer stands this far out from the rail, the pole is under the plank this far, and he
/// falls outward this much farther than where he stood.
const STAND_OUT: f32 = 2.4;
const POLE_OUT: f32 = 3.6;
const FALL_OUT: f32 = 2.6;
/// Steps back onto the ramp this far behind the rail when he climbs down.
const CLIMB_BACK: f32 = 2.5;
const CLIMB_SECS: f32 = 1.0;
/// Low lumber where a pole stood, the stick on the floor (GunShopDynamite), small flames
/// (UndeadFireSmall, particles only), and the valve steam.
const LUMBER: u32 = 1108;
const STICK: u32 = 423;
const FIRE: u32 = 397;
const STEAM: u32 = 353;

const FALL_SECS: f32 = 0.9;
/// He lands sitting (SitGround), then gets up (SitGroundUp) before he fights.
const SIT_SECS: f32 = 1.3;
const GET_UP_SECS: f32 = 1.2;
const FALL_CLIP: u16 = 40;
const SIT_CLIP: u16 = 97;
const SIT_UP_CLIP: u16 = 98;
/// A stick sits sparking on the floor this long before it blows.
const STICK_SITS: f32 = 1.0;
/// The arc's top, over the line from his hand to where it lands, and the client's flight speed
/// for Heavy Dynamite (yd a second). The second leg flies from an unseen stand-in a tick later.
const ARC_LIFT: f32 = 1.8;
const HAND: f32 = 1.0;
const LEG_TICK: f32 = 0.05;
/// The first stick is lit this long after the pull, the next this long after one blows.
const FIRST_THROW: f32 = 1.5;
const NEXT_THROW: f32 = 1.5;
/// The second engineer climbs down after this many throws of his own.
const THROWS_BEFORE_CLIMB: u32 = 3;
/// The demonstration: the furnace belches this long after the first stick blows, the spill runs
/// a section each `SPILL_GAP`, and the plank drops this long after his pole takes light.
const BELCH_AFTER: f32 = 0.6;
const SPILL_STEP: f32 = 1.5;
const SPILL_GAP: f32 = 0.2;
const ALIGHT_SECS: f32 = 1.4;
/// The valve: 2 sec of work, then steam along the channel, then the stripe of fire advances a
/// section each `STRIPE_GAP`; each section burns, and bites again once, before it's out.
const WORK_SECS: f32 = 2.0;
const VENT_SECS: f32 = 1.2;
const STRIPE_STEP: f32 = 1.6;
const STRIPE_RADIUS: f32 = 1.5;
const STRIPE_GAP: f32 = 0.25;
const STRIPE_MOST: usize = 10;
const STRIPE_BURN: f32 = 1.2;
/// The finish: his spanner stuck in the floor, hunched over it (StandWound).
const STUCK_CLIP: u16 = 8;
/// His lines never come closer together than this (the pull line included).
const LINE_GAP: f32 = 2.0;
const MOLTEN: u32 = 5213;
const WORK_VALVE: u32 = 21651; // Opening: the goblin's native UseStandingLoop.
const FINISH: &str = "flurry";

fn falling_point(from: Vec3, to: Vec3, t: f32) -> Vec3 {
    [from[0] + (to[0] - from[0]) * t, from[1] + (to[1] - from[1]) * t,
        from[2] + (to[2] - from[2]) * t * t]
}

fn along(p: Vec3, yaw: f32, d: f32) -> Vec3 {
    [p[0] + yaw.cos() * d, p[1] + yaw.sin() * d, p[2]]
}

fn length(a: Vec3, b: Vec3) -> f32 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

#[derive(Default)]
pub(crate) struct Perches {
    crews: BTreeMap<Id, Crew>,
    workers: BTreeMap<Id, Worker>,
}

struct Crew {
    home: Vec3,
    /// In turn order: the first is the one the furnace brings down.
    workers: Vec<Id>,
    /// Everything the encounter put in the world, removed with it.
    props: Vec<Id>,
    /// Of those, the ones with no hull that appear at once (the client's no-fade flag also
    /// removes collision, so never a plank or pole that stands).
    visuals: Vec<Id>,
    /// Props that go at a time (steam, a stick, a flame).
    timed: Vec<(Id, f32)>,
    /// Flames that come where the fire arrives: (when, where, display, scale, until).
    later: Vec<(f32, Vec3, u32, f32, f32)>,
    began: Option<f32>,
    next_throw: f32,
    /// Live dynamite: thrown and not yet blown.
    sticks: Vec<Stick>,
    /// The arc's second legs (when, thrower, spell, from, to), flown from the top of the arc
    /// when the first arrives.
    legs: Vec<(f32, Id, u32, Vec3, Vec3)>,
    first_blown: bool,
    demo: Option<Demo>,
    valve: Vec3,
    stand: Vec3,
    furnace: Vec3,
    stage: Stage,
    lines: VecDeque<&'static str>,
    said_at: Option<f32>,
    stuck_until: f32,
}

struct Stick {
    thrower: Id,
    at: Vec3,
    lands: f32,
    blows: f32,
    landed: bool,
}

struct Demo {
    worker: Id,
    base: Vec3,
    alight_at: f32,
    drop_at: f32,
    alight: bool,
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    Crew,
    ToValve { since: f32 },
    Working { until: f32 },
    Cooling { until: f32 },
    Finished,
}

struct Worker {
    owner: Id,
    plank: Id,
    support: Id,
    stand: Vec3,
    base: Vec3,
    landing: Vec3,
    climb: Vec3,
    out: f32,
    grounded: usize,
    throws: u32,
    up_at: Option<f32>,
    state: Place,
}

#[derive(Clone, Copy)]
enum Place {
    Perched,
    Falling { from: Vec3, since: f32 },
    Climbing { since: f32 },
    Ground,
}

/// A placed ledge, read by the installed-data checks (`tests/gilnid_native.rs`).
#[derive(Debug, Clone, Copy)]
pub struct LedgeView {
    pub worker: Id,
    pub stand: Vec3,
    /// Where the plank's back end rests (its underside).
    pub rail: Vec3,
    pub base: Vec3,
    pub landing: Vec3,
    pub climb: Vec3,
}

/// Where a ledge's pieces go: its rail, plank, pole, the landing below and the way back.
struct Ledge {
    stand: Vec3,
    rail: Vec3,
    plank: Vec3,
    base: Vec3,
    landing: Vec3,
    climb: Vec3,
}

impl Survival {
    /// A boss's placed ledges (none until his crew is placed), and the valve and where he
    /// stands to work it.
    pub fn perch_ledges(&self, owner: Id) -> (Vec<LedgeView>, Option<(Vec3, Vec3)>) {
        let Some(c) = self.perches.crews.get(&owner) else { return (Vec::new(), None) };
        let ledges = c.workers.iter().filter_map(|id| self.perches.workers.get(id).map(|w| LedgeView {
            worker: *id, stand: w.stand, rail: along([w.stand[0], w.stand[1], w.stand[2] - PLANK_THICK], w.out, -STAND_OUT),
            base: w.base, landing: w.landing, climb: w.climb,
        })).collect();
        (ledges, Some((c.valve, c.stand)))
    }

    /// Only visual cues may skip the 2 sec fade: that client flag also removes collision.
    /// Planks (6481) and poles (1508) that stand keep their hulls; the engineers stand on them.
    pub fn is_perch_visual_prop(&self, id: Id) -> bool {
        self.perches.crews.values().any(|c| c.visuals.contains(&id))
    }

    fn perch_prop(&mut self, at: Vec3, display: u32, scale: f32, facing: f32) -> Id {
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Prop { display, scale }, pos: at, facing });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    /// A prop with no hull that shows at once, gone at `until` (or with the crew).
    fn perch_visual(&mut self, owner: Id, at: Vec3, display: u32, scale: f32, facing: f32, until: Option<f32>) -> Id {
        let id = self.perch_prop(at, display, scale, facing);
        if let Some(c) = self.perches.crews.get_mut(&owner) {
            c.props.push(id);
            c.visuals.push(id);
            if let Some(t) = until { c.timed.push((id, t)); }
        }
        id
    }

    fn perch_gone(&mut self, owner: Id, id: Id) {
        if let Some(c) = self.perches.crews.get_mut(&owner) {
            c.props.retain(|p| *p != id);
            c.visuals.retain(|p| *p != id);
            c.timed.retain(|(p, _)| *p != id);
        }
        if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
    }

    /// Stay on the boss's floor and off water. A bad content point is rejected rather than
    /// putting a worker on the upper ring, in a wall, or where melee cannot reach it.
    fn perch_floor(&self, home: Vec3, offset: [f32; 2]) -> Option<Vec3> {
        let p = self.ground_near(home[0] + offset[0], home[1] + offset[1], home[2]);
        ((p[2] - home[2]).abs() < 2.5
            && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2])
            && (self.land.walk_clear(home, p) || self.land.path(home, p).is_some_and(|v| !v.is_empty())))
            .then_some(p)
    }

    /// A ledge's pieces, or none where its pole has no floor of his to stand on, where something
    /// is overhead, or where he would fall somewhere a walker can't reach.
    fn ledge(&self, home: Vec3, def: &crate::content::LedgeDef) -> Option<Ledge> {
        let stand = [home[0] + def.at[0], home[1] + def.at[1], home[2] + def.at[2]];
        let f = def.out;
        let rail = along([stand[0], stand[1], stand[2] - PLANK_THICK], f, -STAND_OUT);
        let off = |p: Vec3| [p[0] - home[0], p[1] - home[1]];
        let base = self.perch_floor(home, off(along(rail, f, POLE_OUT)))?;
        let landing = self.perch_floor(home, off(along(stand, f, FALL_OUT)))?;
        let high = stand[2] - base[2];
        if !(5.0..=12.0).contains(&high) { return None; }
        // Open air from the floor to the plank, under the pole and under him.
        for p in [base, stand] {
            if self.land.slab_between(p[0], p[1], base[2] + 0.2, stand[2] - PLANK_THICK - 0.1) { return None; }
        }
        // The plank's back end on the rail; its middle along its width.
        let left = f + std::f32::consts::FRAC_PI_2;
        let plank = along(along(rail, f, PLANK_BACK), left, -PLANK_SIDE);
        let back = along(rail, f, -CLIMB_BACK);
        let climb = self.ground_near(back[0], back[1], rail[2]);
        Some(Ledge { stand, rail, plank, base, landing, climb })
    }

    fn place_perch_crew(&mut self, owner: Id) {
        let m = &self.mobs[&owner];
        let home = m.home;
        let Some(d) = self.content.mobs[m.def].perch_crew.clone() else { return };
        let (Some(air), Some(ground)) = (self.content.mob(&d.airborne), self.content.mob(&d.grounded)) else { return };
        let mut workers = Vec::new();
        let mut props = Vec::new();
        for def in &d.ledges {
            let Some(l) = self.ledge(home, def) else {
                self.events.push(Event::Log(format!("perch crew {owner}: unplaceable ledge {:?} omitted", def.at)));
                continue;
            };
            let plank = self.perch_prop(l.plank, PLANK, PLANK_SCALE, def.out);
            let support = self.perch_prop(l.base, POLE, ((l.rail[2] - l.base[2]) / POLE_HEIGHT).max(0.05), def.out);
            props.extend([plank, support]);
            let id = self.spawn_mob(air, l.stand, None, false);
            let health = self.foe_health(air, None);
            let m = self.mobs.get_mut(&id).unwrap();
            m.health = health;
            m.max_health = health;
            m.home = l.landing;
            m.facing = def.out;
            self.perches.workers.insert(id, Worker { owner, plank, support, stand: l.stand, base: l.base,
                landing: l.landing, climb: l.climb, out: def.out, grounded: ground, throws: 0, up_at: None, state: Place::Perched });
            workers.push(id);
        }
        let valve = self.ground_near(home[0] + d.valve[0], home[1] + d.valve[1], home[2]);
        // He works it from the side of his post, an arm's length off.
        let toward = facing(valve, home);
        let near = along(valve, toward, 1.3);
        let stand = self.perch_floor(home, [near[0] - home[0], near[1] - home[1]]).unwrap_or(home);
        let furnace = [home[0] + d.furnace[0], home[1] + d.furnace[1], home[2] + d.furnace[2]];
        self.perches.crews.insert(owner, Crew {
            home, workers, props, visuals: Vec::new(), timed: Vec::new(), later: Vec::new(), began: None, next_throw: 0.0,
            sticks: Vec::new(), legs: Vec::new(), first_blown: false, demo: None, valve, stand, furnace,
            stage: Stage::Crew, lines: VecDeque::new(), said_at: None, stuck_until: 0.0,
        });
    }

    /// Says `text` now, or once his last line is `LINE_GAP` behind him.
    fn perch_say(&mut self, owner: Id, text: &'static str) {
        let Some(c) = self.perches.crews.get_mut(&owner) else { return };
        c.lines.push_back(text);
        self.perch_lines(owner);
    }

    fn perch_lines(&mut self, owner: Id) {
        let now = self.clock;
        let Some(c) = self.perches.crews.get_mut(&owner) else { return };
        if c.said_at.is_some_and(|t| now - t < LINE_GAP) { return; }
        let Some(text) = c.lines.pop_front() else { return };
        c.said_at = Some(now);
        self.yell(owner, text);
    }

    /// Held still for `secs` without the stagger's reel (a fall, a sit, a climb).
    fn perch_hold(&mut self, id: Id, secs: f32) {
        let now = self.clock;
        let Some(state) = self.mobs.get(&id).map(|m| m.state.clone()) else { return };
        if let MobState::Windup { mark: Some(mk), .. } = state { self.unmark(mk); }
        if matches!(state, MobState::Windup { .. }) && self.attack_now(id).ranged {
            self.events.push(Event::CastEnd { caster: id });
        }
        if let Some((_, mk)) = self.mobs.get_mut(&id).and_then(|m| m.lob.take()) { self.unmark(mk); }
        let m = self.mobs.get_mut(&id).unwrap();
        m.stagger_until = m.stagger_until.max(now + secs);
        m.ready_at = m.ready_at.max(now + secs);
        m.pattern = None;
        m.special = false;
        m.path.clear();
        m.thrown_lift = 0.0;
        m.thrown_until = 0.0;
        if let Some(foe) = m.state.foe() { m.state = MobState::Recover { foe, until: now + secs }; }
    }

    /// Called before the ordinary minds. A waiting crew can be seen before the pull.
    pub(crate) fn step_perch_crews(&mut self) {
        let now = self.clock;
        let gone: Vec<_> = self.perches.crews.iter().filter(|(id, c)| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 &&
                !matches!(m.state, MobState::Return) && (c.began.is_none() || m.state.foe().is_some()))
        }).map(|(id, _)| *id).collect();
        for id in gone { self.remove_perch_crew(id); }
        let expired: Vec<(Id, Id)> = self.perches.crews.iter()
            .flat_map(|(o, c)| c.timed.iter().filter(|(_, t)| *t <= now).map(|(p, _)| (*o, *p)))
            .collect();
        for (owner, id) in expired { self.perch_gone(owner, id); }
        let due: Vec<(Id, (f32, Vec3, u32, f32, f32))> = self.perches.crews.iter_mut()
            .flat_map(|(o, c)| {
                let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut c.later).into_iter().partition(|l| l.0 <= now);
                c.later = keep;
                due.into_iter().map(move |l| (*o, l))
            })
            .collect();
        for (owner, (_, at, display, scale, until)) in due { self.perch_visual(owner, at, display, scale, 0.0, Some(until)); }
        let new: Vec<_> = self.mobs.iter().filter(|(id, m)| {
            m.health > 0 && !m.hidden && !matches!(m.state, MobState::Return)
                && self.content.mobs[m.def].perch_crew.is_some() && !self.perches.crews.contains_key(id)
                && self.players.values().any(|p| dist2d(p.pos, m.home) < 45.0 && (p.pos[2] - m.home[2]).abs() < 8.0)
        }).map(|(id, _)| *id).collect();
        for id in new { self.place_perch_crew(id); }
        self.step_perch_bodies();

        let owners: Vec<_> = self.perches.crews.keys().copied().collect();
        for owner in owners {
            let Some(foe) = self.mobs[&owner].state.foe() else { continue };
            if self.perches.crews[&owner].began.is_none() {
                let c = self.perches.crews.get_mut(&owner).unwrap();
                c.began = Some(now);
                // The pull line has just been said.
                c.said_at = Some(now);
                c.next_throw = now + FIRST_THROW;
            }
            self.perch_lines(owner);
            let ids = self.perches.crews[&owner].workers.clone();
            for id in &ids {
                if let Some(m) = self.mobs.get_mut(id).filter(|m| m.health > 0 && m.state.foe().is_none()) {
                    m.state = MobState::Chase { foe };
                }
            }
            self.step_perch_sticks(owner);
            if !matches!(self.perches.crews[&owner].stage, Stage::Crew) { continue; }
            if self.step_perch_demo(owner) { continue; }
            let alive = |s: &Self, id: &Id| s.mobs.get(id).is_some_and(|m| m.health > 0);
            if ids.iter().all(|id| !alive(self, id)) {
                self.perch_say(owner, "Useless. Every one of you.");
                self.stagger_mob(owner, 0.05, false); // Let go of the current blow before running.
                self.perches.crews.get_mut(&owner).unwrap().stage = Stage::ToValve { since: now };
                continue;
            }
            // Neither two sticks at once, nor one while a man is falling.
            let c = &self.perches.crews[&owner];
            let busy = !c.sticks.is_empty() || c.demo.is_some()
                || ids.iter().any(|id| self.perch_controlled(*id) && self.mobs.get(id).is_some_and(|m| matches!(m.state, MobState::Windup { .. })))
                || ids.iter().any(|id| self.perches.workers.get(id).is_some_and(|w| !matches!(w.state, Place::Perched | Place::Ground)));
            if busy || now < c.next_throw { continue; }
            let perched: Vec<_> = ids.iter().copied().filter(|id| alive(self, id)
                && self.perches.workers.get(id).is_some_and(|w| matches!(w.state, Place::Perched))).collect();
            // Three throws and he climbs down rather than wait to be knocked off.
            if let Some(&id) = perched.iter().find(|id| self.perches.workers[id].throws >= THROWS_BEFORE_CLIMB) {
                self.climb_down(id);
                continue;
            }
            // In turn: the first until the furnace takes him down, then the next.
            let Some(&id) = perched.first() else { continue };
            let to = self.foe_pos(foe);
            let from = self.mobs[&id].pos;
            if let Some(to) = to.filter(|to| dist2d(*to, from) <= 26.0 && clear_line(self.land.as_ref(), from, *to)) {
                if !self.disabled(id) && self.mobs[&id].stagger_until <= now {
                    self.mobs.get_mut(&id).unwrap().pattern = None;
                    self.windup(id, foe, to);
                    // The next waits for this one to blow (or, cut short, for its time to pass).
                    let fuse = self.attack_now(id).windup;
                    self.perches.crews.get_mut(&owner).unwrap().next_throw = now + fuse + 4.0;
                }
            }
        }
    }

    /// Falls finish, climbers arrive, and a landed man gets up. A corpse also completes its fall;
    /// ordinary corpse expiry must not leave it in the air.
    fn step_perch_bodies(&mut self) {
        let now = self.clock;
        let moving: Vec<_> = self.perches.workers.iter().map(|(id, w)| (*id, w.state)).collect();
        for (id, state) in moving {
            let w = &self.perches.workers[&id];
            let (to, climb, stand, ground, out) = (w.landing, w.climb, w.stand, w.grounded, w.out);
            match state {
                Place::Falling { from, since } => {
                    let t = ((now - since) / FALL_SECS).clamp(0.0, 1.0);
                    let Some(m) = self.mobs.get_mut(&id) else { continue };
                    // The client follows the same accelerating fall, without floor projection.
                    m.pos = falling_point(from, to, t);
                    m.path.clear();
                    if t < 1.0 { continue; }
                    m.def = ground;
                    m.home = to;
                    let alive = m.health > 0;
                    let face = m.facing;
                    self.events.push(Event::MobGear { id });
                    self.events.push(Event::MobStop { id, pos: to, facing: face });
                    self.events.push(Event::Visual { id, kit: combat::KIT_FALL_DUST, by: None });
                    self.perches.workers.get_mut(&id).unwrap().state = Place::Ground;
                    if alive {
                        // Lands sitting, then gets up.
                        self.perch_hold(id, SIT_SECS + GET_UP_SECS);
                        self.events.push(Event::HoldPose { id, clip: SIT_CLIP, secs: SIT_SECS, from: None });
                        self.perches.workers.get_mut(&id).unwrap().up_at = Some(now + SIT_SECS);
                    }
                }
                Place::Climbing { since } => {
                    let t = ((now - since) / CLIMB_SECS).clamp(0.0, 1.0);
                    let Some(m) = self.mobs.get_mut(&id) else { continue };
                    m.pos = [stand[0] + (climb[0] - stand[0]) * t, stand[1] + (climb[1] - stand[1]) * t, stand[2] + (climb[2] - stand[2]) * t];
                    m.path.clear();
                    if t < 1.0 { continue; }
                    m.def = ground;
                    m.home = climb;
                    m.facing = out + std::f32::consts::PI;
                    self.events.push(Event::MobGear { id });
                    self.events.push(Event::MobStop { id, pos: climb, facing: out + std::f32::consts::PI });
                    self.perches.workers.get_mut(&id).unwrap().state = Place::Ground;
                }
                Place::Ground => {
                    if self.perches.workers[&id].up_at.is_some_and(|t| now >= t) {
                        self.perches.workers.get_mut(&id).unwrap().up_at = None;
                        if self.mobs.get(&id).is_some_and(|m| m.health > 0) {
                            self.events.push(Event::Clip { id, clip: SIT_UP_CLIP, take: 0, from: None, secs: GET_UP_SECS, at: 0.0, body: ClipBody::Full });
                        }
                    }
                }
                Place::Perched => {}
            }
        }
    }

    /// The arc's second leg, the stick on the floor sparking, and the blast as it blows.
    fn step_perch_sticks(&mut self, owner: Id) {
        let now = self.clock;
        let c = self.perches.crews.get_mut(&owner).unwrap();
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut c.legs).into_iter().partition(|l| l.0 <= now);
        c.legs = keep;
        for (_, thrower, spell, from, at) in due {
            let anchor = self.alloc();
            self.events.push(Event::MissileFrom { anchor, caster: thrower, spell, from, target: Target::Point(at) });
        }
        let mut landed = Vec::new();
        let mut blown = Vec::new();
        let c = self.perches.crews.get_mut(&owner).unwrap();
        for (k, s) in c.sticks.iter_mut().enumerate() {
            if !s.landed && now >= s.lands {
                s.landed = true;
                landed.push((s.at, s.blows, s.thrower));
            }
            if now >= s.blows { blown.push(k); }
        }
        for (at, blows, thrower) in landed {
            // It sits there sparking, hissing, its blast drawn to its true size.
            let stick = self.perch_visual(owner, at, STICK, 2.0, 0.0, Some(blows));
            self.events.push(Event::Sound { id: stick, sound: 1424 });
            let radius = self.mobs.get(&thrower).map_or(2.5, |m| self.content.mobs[m.def].attack.radius);
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: radius, arc: 360.0, width: 0.0,
                secs: (blows - now).max(0.3), colour: "FF7A1A".into() });
        }
        let mut first = false;
        for k in blown.into_iter().rev() {
            let c = self.perches.crews.get_mut(&owner).unwrap();
            c.sticks.remove(k);
            c.next_throw = now + NEXT_THROW;
            first |= !std::mem::replace(&mut c.first_blown, true);
        }
        if first { self.begin_demo(owner); }
    }

    /// After the first stick: the furnace belches, a spill runs to the first engineer's pole and
    /// sets it alight, and his plank goes. If the hero has already brought one down, they've
    /// learned it: no lesson.
    fn begin_demo(&mut self, owner: Id) {
        let now = self.clock;
        let ids = self.perches.crews[&owner].workers.clone();
        let perched = |s: &Self, id: &Id| s.mobs.get(id).is_some_and(|m| m.health > 0)
            && s.perches.workers.get(id).is_some_and(|w| matches!(w.state, Place::Perched));
        if !ids.iter().all(|id| perched(self, id)) { return; }
        let Some(&worker) = ids.first() else { return };
        let base = self.perches.workers[&worker].base;
        let c = &self.perches.crews[&owner];
        let furnace = c.furnace;
        let home = c.home;
        let dir = facing(furnace, base);
        let foot = self.ground_near(furnace[0] + dir.cos() * 4.8, furnace[1] + dir.sin() * 4.8, home[2]);
        let start = now + BELCH_AFTER;
        self.events.push(Event::Missile { caster: owner, spell: MOLTEN, target: Target::Point(furnace) });
        self.events.push(Event::Sound { id: owner, sound: 1507 }); // Molten Blast Impact.
        let reach = (dist2d(foot, base) - 0.8).max(0.0);
        let points = self.perch_run_points(owner, foot, dir, reach, SPILL_STEP);
        let n = self.perch_run(owner, &points, dir, 1.0, start, SPILL_GAP, 8, 0.0, false);
        let alight_at = start + n as f32 * SPILL_GAP;
        self.perches.crews.get_mut(&owner).unwrap().demo = Some(Demo { worker, base, alight_at, drop_at: alight_at + ALIGHT_SECS, alight: false });
    }

    /// The demonstration's beats; true while it runs.
    fn step_perch_demo(&mut self, owner: Id) -> bool {
        let now = self.clock;
        let Some(d) = self.perches.crews[&owner].demo.as_ref() else { return false };
        let (worker, base, alight_at, drop_at, alight) = (d.worker, d.base, d.alight_at, d.drop_at, d.alight);
        if !self.perches.workers.get(&worker).is_some_and(|w| matches!(w.state, Place::Perched)) {
            // Knocked down another way meanwhile: the lesson is learned.
            self.perches.crews.get_mut(&owner).unwrap().demo = None;
            return false;
        }
        if !alight && now >= alight_at {
            self.perches.crews.get_mut(&owner).unwrap().demo.as_mut().unwrap().alight = true;
            let flame = self.perch_visual(owner, base, FIRE, 1.0, 0.0, Some(drop_at + 2.5));
            self.events.push(Event::Sound { id: flame, sound: 1484 }); // Fire Cast.
            self.perch_say(owner, "Mind the equipment!");
        }
        if now >= drop_at {
            self.perches.crews.get_mut(&owner).unwrap().demo = None;
            self.drop_perch(worker);
            return false;
        }
        true
    }

    pub(crate) fn perch_controlled(&self, id: Id) -> bool {
        self.perches.workers.get(&id).is_some_and(|w| !matches!(w.state, Place::Ground))
    }

    /// A perched engineer lets go of his stick: thrown up to the top of an arc and down from it
    /// (the client flies a missile straight, so the arc is two), landing where it was marked.
    pub(crate) fn perch_release(&mut self, id: Id) -> bool {
        let Some(owner) = self.perches.workers.get(&id).filter(|w| matches!(w.state, Place::Perched)).map(|w| w.owner) else { return false };
        let now = self.clock;
        let a = self.attack_now(id);
        let pos = self.mobs[&id].pos;
        self.events.push(Event::CastEnd { caster: id });
        let Some((at, mark)) = self.mobs.get_mut(&id).and_then(|m| m.lob.take()) else { return true };
        self.unmark(mark);
        let hand = [pos[0], pos[1], pos[2] + HAND];
        let top = [hand[0] + (at[0] - hand[0]) * 0.4, hand[1] + (at[1] - hand[1]) * 0.4, hand[2].max(at[2]) + ARC_LIFT];
        let speed = a.speed.max(1.0);
        let first = length(hand, top) / speed;
        let lands = now + first + LEG_TICK + length(top, at) / speed;
        let blows = lands + STICK_SITS;
        self.events.push(Event::Missile { caster: id, spell: a.spell, target: Target::Point(top) });
        let tele = Telegraph { mob: id, reach: a.radius, arc: 360.0, secs: blows - now, blockable: false, foe: None,
            width: 0.0, at: Some((at, 0.0)), hero: false, own_hidden: false };
        let mark = self.mark_as(at, a.radius, blows - now + 0.1, Some(tele));
        self.missiles.push(combat::Missile { owner: id, from: pos, at, radius: a.radius, damage: a.damage,
            blockable: false, heavy: false, land_at: blows, mark, look: a.look });
        let w = self.perches.workers.get_mut(&id).unwrap();
        w.throws += 1;
        let c = self.perches.crews.get_mut(&owner).unwrap();
        c.legs.push((now + first, id, a.spell, top, at));
        c.sticks.push(Stick { thrower: id, at, lands, blows, landed: false });
        true
    }

    /// He steps back off his plank onto the ramp and comes down it to fight.
    fn climb_down(&mut self, id: Id) {
        let Some(w) = self.perches.workers.get(&id).filter(|w| matches!(w.state, Place::Perched)) else { return };
        let (from, to) = (w.stand, w.climb);
        self.perch_hold(id, CLIMB_SECS);
        self.perches.workers.get_mut(&id).unwrap().state = Place::Climbing { since: self.clock };
        self.events.push(Event::MobMove { id, path: vec![from, to], speed: length(from, to) / CLIMB_SECS, face: None });
    }

    /// A real projectile hit, a blow on him or his pole, or the furnace drops him exactly once:
    /// the plank goes, and he falls outward in plain view.
    pub(crate) fn drop_perch(&mut self, id: Id) {
        let Some(w) = self.perches.workers.get(&id).filter(|w| matches!(w.state, Place::Perched)) else { return };
        let (owner, support, plank, base, out) = (w.owner, w.support, w.plank, w.base, w.out);
        let Some(from) = self.mobs.get(&id).map(|m| m.pos) else { return };
        let landing = self.perches.workers[&id].landing;
        self.perch_hold(id, FALL_SECS);
        self.perches.workers.get_mut(&id).unwrap().state = Place::Falling { from, since: self.clock };
        for prop in [support, plank] { self.perch_gone(owner, prop); }
        // The broken pole and the plank lie where they fell, without hulls to trip on.
        self.perch_visual(owner, base, LUMBER, 1.0, out, None);
        let lie = self.ground_near(base[0] + out.cos() * 1.2, base[1] + out.sin() * 1.2, base[2]);
        self.perch_visual(owner, lie, PLANK, PLANK_SCALE, out + 0.5, None);
        self.events.push(Event::Sound { id, sound: 3720 }); // Native wooden impact.
        self.events.push(Event::HoldPose { id, clip: FALL_CLIP, secs: FALL_SECS, from: None });
        let path = (0..=8).map(|k| falling_point(from, landing, k as f32 / 8.0)).collect();
        self.events.push(Event::MobMove { id, path, speed: dist2d(from, landing).max(0.5) / FALL_SECS, face: None });
    }

    /// Hitting a worker starts the same encounter as hitting its foreman.
    pub(crate) fn perch_hurt(&mut self, id: Id, by: Id) {
        let Some(owner) = self.perches.workers.get(&id).map(|w| w.owner) else { return };
        if self.players.get(&by).is_some_and(|p| p.health > 0)
            && self.mobs.get(&owner).is_some_and(|m| m.health > 0 && m.state.foe().is_none() && !matches!(m.state, MobState::Return)) {
            self.engage(owner, Foe::Hero(by));
        }
    }

    /// A hero's swing: a pole in reach on the hero's floor cracks and drops its man; so does a
    /// blow on the man himself, from the ramp behind him.
    pub(crate) fn swing_on_perches(&mut self, hero: Id, face: f32, range: f32, arc: f32) {
        let Some(pos) = self.players.get(&hero).map(|p| p.pos) else { return };
        let reach = |at: Vec3, rise: f32| dist2d(pos, at) <= range + 0.6 && (pos[2] - at[2]).abs() < rise
            && angle_between(face, facing(pos, at)) <= arc / 2.0;
        let hits: Vec<_> = self.perches.workers.iter().filter(|(id, w)| matches!(w.state, Place::Perched)
            && ((reach(w.base, 2.0) && clear_line(self.land.as_ref(), pos, w.base))
                || self.mobs.get(id).is_some_and(|m| m.health > 0 && reach(m.pos, 2.5)))).map(|(id, _)| *id).collect();
        for id in hits {
            self.perch_hurt(id, hero);
            self.drop_perch(id);
        }
    }

    /// The sections of a run along the floor from `from` along `yaw`, one each `step` for
    /// `reach` yards. Only the boss's own floor carries it: it may begin a little way out (past
    /// the furnace's foot), and it stops at a wall, water, a step or a drop.
    fn perch_run_points(&self, owner: Id, from: Vec3, yaw: f32, reach: f32, step: f32) -> Vec<Vec3> {
        let floor = self.perches.crews.get(&owner).map_or(from[2], |c| c.home[2]);
        let mut out: Vec<Vec3> = Vec::new();
        let mut k = 0;
        while (k as f32) * step <= reach && out.len() < STRIPE_MOST {
            let p = along(from, yaw, k as f32 * step);
            let p = self.ground_near(p[0], p[1], floor);
            let ok = (p[2] - floor).abs() <= 1.0 && !self.land.water_near(p[0], p[1], p[2]) && !self.land.blocked_near(p[0], p[1], p[2])
                && out.last().is_none_or(|last| self.land.walk_clear(*last, p));
            k += 1;
            if ok { out.push(p); } else if !out.is_empty() || (k as f32) * step > 3.0 { break; }
        }
        out
    }

    /// Molten iron (or fire) run along the floor: a section each point, the first landing at
    /// `start` and each next `gap` later, every one drawn to its true width whatever the floor
    /// markers. The number of sections laid.
    #[allow(clippy::too_many_arguments)]
    fn perch_run(&mut self, owner: Id, points: &[Vec3], yaw: f32, radius: f32, start: f32,
        gap: f32, damage: u32, burn: f32, steam: bool) -> usize {
        let now = self.clock;
        for (n, p) in points.iter().copied().enumerate() {
            let lands = start + n as f32 * gap;
            let mark = self.mark_ground(p, radius, lands - now, 0);
            self.missiles.push(combat::Missile { owner, from: p, at: p, radius, damage, blockable: false, heavy: false,
                land_at: lands, mark, look: MOLTEN });
            if burn > 0.0 {
                // Still burning where it came: it bites once more, then it's out.
                if let Some(c) = self.perches.crews.get_mut(&owner) { c.later.push((lands, p, FIRE, 1.0, lands + burn + 0.4)); }
                let mark = self.mark_ground(p, radius, lands + burn - now, 0);
                self.missiles.push(combat::Missile { owner, from: p, at: p, radius, damage: damage / 2, blockable: false,
                    heavy: false, land_at: lands + burn, mark, look: 0 });
            }
            if steam && n % 2 == 0 {
                self.perch_visual(owner, p, STEAM, 1.4, 0.0, Some(start));
            }
        }
        let n = points.len();
        if let (Some(first), Some(last)) = (points.first(), points.last()) {
            let len = dist2d(*first, *last);
            let mid = [(first[0] + last[0]) / 2.0, (first[1] + last[1]) / 2.0, first[2]];
            self.events.push(Event::GroundBurst { at: mid, yaw, shape: "line", reach: len / 2.0 + radius,
                arc: 0.0, width: radius * 2.0, secs: start - now + n as f32 * gap + burn, colour: "FF5A14".into() });
        }
        n
    }

    /// Walk to the furnace's valve and work it, then the fire runs across the pit. A stagger
    /// delays his hands; it cannot restart the event or summon another workforce.
    pub(crate) fn step_perch_valve(&mut self, id: Id) -> bool {
        let Some(c) = self.perches.crews.get(&id) else { return false };
        let (stage, valve, stand, furnace) = (c.stage, c.valve, c.stand, c.furnace);
        let now = self.clock;
        self.perch_lines(id);
        if !matches!(stage, Stage::Crew | Stage::Finished)
            && (self.disabled(id) || self.mobs[&id].stagger_until > now) {
            self.halt(id);
            if matches!(stage, Stage::Working { .. }) {
                self.events.push(Event::CastEnd { caster: id });
                self.perches.crews.get_mut(&id).unwrap().stage = Stage::ToValve { since: now };
            }
            return true;
        }
        match stage {
            Stage::ToValve { since } => {
                if dist2d(self.mobs[&id].pos, stand) > 1.0 && now - since < 8.0 {
                    self.go(id, stand, self.content.mobs[self.mobs[&id].def].run_speed);
                } else {
                    self.halt(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, valve);
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                    self.events.push(Event::Cast { caster: id, spell: WORK_VALVE, ms: (WORK_SECS * 1000.0) as u32 });
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Working { until: now + WORK_SECS };
                }
                true
            }
            Stage::Working { until } => {
                self.halt(id);
                if now >= until {
                    self.events.push(Event::CastEnd { caster: id });
                    // The furnace glows, it whistles, steam runs along the channel, then the fire.
                    self.events.push(Event::Missile { caster: id, spell: MOLTEN, target: Target::Point(furnace) });
                    self.perch_visual(id, furnace, FIRE, 2.0, 0.0, Some(now + VENT_SECS + 2.0));
                    self.events.push(Event::Sound { id, sound: 3431 }); // Deadmines steam whistle.
                    // At his foe if the floor runs that way, else straight out from the furnace.
                    let pos = self.mobs[&id].pos;
                    let aim = self.mobs[&id].state.foe().and_then(|f| self.foe_pos(f));
                    let reach = STRIPE_STEP * (STRIPE_MOST - 1) as f32;
                    let mut ways = aim.map(|a| facing(pos, a)).into_iter().chain([facing(furnace, pos)]);
                    let (yaw, points) = ways.by_ref().map(|yaw| (yaw, self.perch_run_points(id, along(pos, yaw, 1.0), yaw, reach, STRIPE_STEP)))
                        .find(|(_, p)| p.len() >= 4).unwrap_or((0.0, Vec::new()));
                    let n = self.perch_run(id, &points, yaw, STRIPE_RADIUS, now + VENT_SECS, STRIPE_GAP, 30, STRIPE_BURN, true);
                    let done = now + VENT_SECS + n as f32 * STRIPE_GAP + STRIPE_BURN + 0.2;
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Cooling { until: done };
                }
                true
            }
            Stage::Cooling { until } => {
                self.halt(id);
                if now >= until {
                    self.perches.crews.get_mut(&id).unwrap().stage = Stage::Finished;
                    self.perch_say(id, "Then we'll melt the lot!");
                }
                true
            }
            _ => false,
        }
    }

    /// His finish only once the valve is done, and then nothing else; no heavy blow of his while
    /// a stick of his crew's is live.
    pub(crate) fn perch_pattern_allowed(&self, id: Id, p: &PatternDef) -> bool {
        let Some(m) = self.mobs.get(&id) else { return true };
        if self.content.mobs[m.def].perch_crew.is_none() { return true; }
        let finished = self.perches.crews.get(&id).is_some_and(|c| matches!(c.stage, Stage::Finished));
        if finished != (p.name == FINISH) { return false; }
        !(self.perch_stick_live(id) && p.steps.iter().any(|s| s.heavy == Some(true)))
    }

    /// One of his crew's sticks is lit, in the air or on the floor.
    pub(crate) fn perch_stick_live(&self, owner: Id) -> bool {
        self.perches.crews.get(&owner).is_some_and(|c| !c.sticks.is_empty()
            || c.workers.iter().any(|w| self.perch_controlled(*w)
                && self.mobs.get(w).is_some_and(|m| matches!(m.state, MobState::Windup { .. }))))
    }

    /// Pattern `k` is his finish: always its three blows, never cut short or run on, so the
    /// last one is the one that sticks.
    pub(crate) fn perch_finish(&self, id: Id, k: usize) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].perch_crew.is_some()
            && self.content.mobs[m.def].patterns.get(k).is_some_and(|p| p.name == FINISH))
    }

    /// His finish landed: the spanner sticks in the floor and he hauls at it, open.
    pub(crate) fn perch_string_landed(&mut self, id: Id, pattern: Option<usize>) {
        let Some(k) = pattern else { return };
        let Some(m) = self.mobs.get(&id) else { return };
        if !self.perches.crews.contains_key(&id) || self.content.mobs[m.def].patterns.get(k).is_none_or(|p| p.name != FINISH) { return; }
        let MobState::Recover { until, .. } = m.state else { return };
        let secs = until - self.clock;
        self.perches.crews.get_mut(&id).unwrap().stuck_until = until;
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::Sound { id, sound: 3720 });
        self.events.push(Event::HoldPose { id, clip: STUCK_CLIP, secs, from: None });
    }

    /// Debug damage never counts toward his or his crew's posture.
    pub(crate) fn perch_posture_share(&self, id: Id, label: &str) -> f32 {
        let ours = self.perches.crews.contains_key(&id) || self.perches.workers.contains_key(&id)
            || self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].perch_crew.is_some());
        if ours && label == "debug" { 0.0 } else { 1.0 }
    }

    pub(crate) fn remove_perch_crew(&mut self, owner: Id) {
        let Some(c) = self.perches.crews.remove(&owner) else { return };
        self.events.push(Event::CastEnd { caster: owner });
        // A held pose (the stuck spanner) lets go with him.
        if c.stuck_until > self.clock { self.events.push(Event::HoldPose { id: owner, clip: 0, secs: 0.0, from: None }); }
        for id in &c.workers {
            if let Some(m) = self.mobs.get(id) {
                if let MobState::Windup { mark: Some(mark), .. } = m.state { self.unmark(mark); }
            }
            if let Some((_, mark)) = self.mobs.get_mut(id).and_then(|m| m.lob.take()) { self.unmark(mark); }
            self.clear_cc(*id);
            self.perches.workers.remove(id);
            if self.mobs.remove(id).is_some() { self.events.push(Event::MobGone { id: *id }); }
        }
        let marks: Vec<_> = self.missiles.iter().filter(|m| m.owner == owner || c.workers.contains(&m.owner)).map(|m| m.mark).collect();
        self.missiles.retain(|m| m.owner != owner && !c.workers.contains(&m.owner));
        for mark in marks { self.unmark(mark); }
        for id in c.props {
            if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
        }
    }
}
