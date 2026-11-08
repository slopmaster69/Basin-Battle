//! Basin Battle's boss beats, beyond what a phase does by itself (`[[mob.phase]]`):
//!
//! - **The run** (`run_to`): the boss breaks off and runs a route to another room, heeding nobody,
//!   shaking rocks loose behind it (`run_falls`). At the route's end it waits for the heroes
//!   (`wait_for`), then takes up the fight there (`arrive_text`, `arrive_kit`), its home from then.
//!   A fight lost there puts it back where its fight began.
//! - **Burning blows** (`burn`): from the phase on, each heavy blow leaves burning ground where
//!   it lands, for a while.
//! - **Debris** (`debris`): rocks brought down over set points as the phase begins, dust first.
//! - **A lodged weapon** (a step's `lodge`): its weapon sticks where the blow lands, and it hauls
//!   at it, open to every blow.
//!
//! Each danger shows before it lands (dust before rocks, the flames where they burn), with or
//! without the floor markers.

use super::mobs::MobState;
use super::*;
use crate::content::BurnDef;

#[cfg(test)]
#[path = "bosses_tests.rs"]
mod tests;

/// The dust over a spot where rocks are about to come down, and where they land.
const DUST: u32 = 7272;
/// How long the dust shows before the rocks of a set piece fall, how wide they come down, and
/// their blow as a share of the boss's own (heavy: they throw down).
const DEBRIS_WARN: f32 = 1.3;
const DEBRIS_RADIUS: f32 = 3.0;
const DEBRIS_SHARE: f32 = 0.6;
/// Rocks shaken loose behind it on the run: a lighter fall, and a shorter warning.
const FALL_WARN: f32 = 1.0;
const FALL_RADIUS: f32 = 2.5;
const FALL_SHARE: f32 = 0.25;
/// At the route's end it waits this near for a hero (yards), at most this long (seconds).
const WAIT_FOR: f32 = 22.0;
const WAIT_MOST: f32 = 30.0;
/// A leg it has made no way along for this long: it's set down at the leg's end.
const STUCK: f32 = 3.0;
/// It runs its route this much quicker than it chases: well ahead of the heroes, there first.
const RUN_HASTE: f32 = 1.6;
/// A leg taking this many times longer than a straight run along it is helped along.
const LEG_SLACK: f32 = 2.5;
/// Burning ground bites a hero standing in it this often.
const BURN_TICK: f32 = 1.0;
/// The collapse: how often it crushes a hero behind the front, brings rock down behind it (over
/// the last this-many yards), drops a warning fall ahead, and raises dust at the front.
const COLLAPSE_CRUSH_TICK: f32 = 0.5;
const BURIED_EVERY: f32 = 0.35;
const BURIED_SPAN: f32 = 12.0;
const WARN_EVERY: f32 = 1.1;
const DUST_EVERY: f32 = 0.6;
/// A hero farther than this off the route is not on it (another floor, a side cave).
const OFF_ROUTE: f32 = 20.0;
/// Its hero this far past the warning band has run off: the boss holds the line, and hurls rock at
/// them this often.
const LINE_SLACK: f32 = 6.0;
const HURL_EVERY: f32 = 4.0;

#[derive(Debug, Clone, Default)]
pub(crate) struct Bosses {
    /// Bosses on the run, by creature.
    runs: BTreeMap<Id, Run>,
    /// Where a boss that ran began its fight, to be put back there once the fight is lost.
    began: BTreeMap<Id, Vec3>,
    /// Bosses whose heavy blows burn, and how.
    burning: BTreeMap<Id, BurnDef>,
    /// Burning ground.
    fires: Vec<Fire>,
    /// The place coming down behind the heroes, if a boss has brought it down.
    collapse: Option<Collapse>,
}

/// A collapse under way (`[mob.phase.collapse]`).
#[derive(Debug, Clone)]
struct Collapse {
    boss: Id,
    route: Vec<Vec3>,
    /// How far along the route each point is (yards).
    along: Vec<f32>,
    /// How far along the route the front has come.
    front: f32,
    speed: f32,
    lead: f32,
    crush: f32,
    hold: f32,
    end_text: String,
    held_text: String,
    /// It has reached the route's end: the last of the fight.
    done: bool,
    /// Its own beats: the next crush of the buried, rock behind the front, a warning fall ahead
    /// of it, the dust at the front.
    crush_at: f32,
    buried_at: f32,
    warn_at: f32,
    dust_at: f32,
    /// The boss was kept from falling before the end (said once).
    held: bool,
    /// When it next hurls rock at a hero who ran off ahead.
    hurl_at: f32,
    /// Next walking corner while holding the line. Kept in order through hairpins where
    /// the same position occurs twice; a nearest-point projection alone can go backwards.
    boss_leg: usize,
    holding: bool,
}

#[derive(Debug, Clone)]
struct Run {
    /// The phase (its index) whose route it runs.
    phase: usize,
    /// The point it's making for.
    leg: usize,
    /// When it next shakes rocks loose.
    falls_at: f32,
    /// When it reached the route's end (waiting there for the heroes).
    arrived: Option<f32>,
    /// Where it last made headway, and when: a run that stands still this long is helped along.
    went_from: Vec3,
    went_at: f32,
    /// When it set off on this leg, and from where: a leg taking far longer than a run along it
    /// would is helped along too.
    leg_at: f32,
    leg_from: Vec3,
}

#[derive(Debug, Clone)]
struct Fire {
    at: Vec3,
    radius: f32,
    damage: u32,
    until: f32,
    bite_at: f32,
    owner: Id,
    obj: Id,
}

impl Survival {
    /// A boss's phase has begun (`enrage`): its beats beyond the phase's own.
    pub(crate) fn begin_boss_beats(&mut self, mid: Id, phase: usize) {
        let Some((def, home)) = self.mobs.get(&mid).map(|m| (m.def, m.home)) else { return };
        let Some(e) = self.content.mobs[def].phases.get(phase).cloned() else { return };
        if let Some(b) = e.burn {
            self.bosses.burning.insert(mid, b);
        }
        if !e.debris.is_empty() {
            let damage = self.boss_blow(mid, DEBRIS_SHARE);
            for p in &e.debris {
                self.rocks_fall(mid, *p, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
            }
        }
        if e.rocks_round > 0.0 {
            let at = self.mobs[&mid].pos;
            let damage = self.boss_blow(mid, DEBRIS_SHARE);
            for k in 0..6 {
                let a = k as f32 * std::f32::consts::TAU / 6.0 + 0.4;
                let p = [at[0] + a.cos() * e.rocks_round, at[1] + a.sin() * e.rocks_round, at[2]];
                self.rocks_fall(mid, p, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
            }
        }
        if let Some(c) = e.collapse.filter(|c| c.route.len() >= 2) {
            let Some(route) = crate::land::walking_route(self.land.as_ref(), &c.route) else {
                self.events.push(Event::Log(format!("{}: collapse route has no safe walk; phase stopped", self.content.mobs[def].name)));
                return;
            };
            let now = self.clock;
            self.bosses.began.entry(mid).or_insert(home);
            let mut along = vec![0.0];
            for w in route.windows(2) {
                let d = dist3(w[0], w[1]);
                along.push(along.last().unwrap() + d);
            }
            self.bosses.collapse = Some(Collapse {
                boss: mid,
                route,
                along,
                front: 0.0,
                speed: c.speed.max(0.5),
                lead: c.lead.max(0.0),
                crush: c.crush.max(0.0),
                hold: c.hold.clamp(0.0, 1.0),
                end_text: c.end_text.clone(),
                held_text: c.held_text.clone(),
                done: false,
                crush_at: now + COLLAPSE_CRUSH_TICK,
                buried_at: now,
                warn_at: now + 0.5,
                dust_at: now,
                held: false,
                hurl_at: now,
                boss_leg: 1,
                holding: false,
            });
            let name = self.content.mobs[def].name.clone();
            self.events.push(Event::Log(format!("{name} brings the place down: a collapse along {:.0} yd at {:.1} yd/s", self.bosses.collapse.as_ref().unwrap().along.last().copied().unwrap_or(0.0), c.speed)));
        }
        if !e.run_to.is_empty() {
            let now = self.clock;
            self.bosses.began.entry(mid).or_insert(home);
            self.bosses.runs.insert(
                mid,
                Run {
                    phase,
                    leg: 0,
                    falls_at: now + e.run_falls.max(0.5),
                    arrived: None,
                    went_from: home,
                    went_at: now,
                    leg_at: now,
                    leg_from: home,
                },
            );
            // Whatever it was winding up is let go: it runs.
            if let MobState::Windup { mark: Some(mk), .. } = self.mobs[&mid].state {
                self.unmark(mk);
            }
            let m = self.mobs.get_mut(&mid).unwrap();
            m.pattern = None;
            m.special = false;
            m.stagger_until = 0.0;
            if let Some(foe) = m.state.foe() {
                m.state = MobState::Chase { foe };
            }
            self.halt(mid);
        }
    }

    /// One step of a collapse: the front comes on along its route; behind it rock falls and the
    /// buried are crushed; ahead of it dust and warning falls; the boss fights on through it,
    /// leashed to nowhere. A fight lost puts everything back: the boss where its fight began.
    pub(crate) fn step_boss_collapse(&mut self, dt: f32) {
        let Some(c) = self.bosses.collapse.clone() else { return };
        let now = self.clock;
        let boss = c.boss;
        let Some(m) = self.mobs.get(&boss) else {
            self.bosses.collapse = None;
            return;
        };
        if m.health == 0 {
            self.bosses.collapse = None;
            return;
        }
        // The fight lost (every hero down or gone): the collapse ends, and the boss goes back to
        // where its fight began, to be whole again there.
        if matches!(m.state, MobState::Return | MobState::Idle { .. }) {
            self.bosses.collapse = None;
            if let Some(at) = self.bosses.began.get(&boss).copied() {
                self.mobs.get_mut(&boss).unwrap().home = at;
                self.set_back(boss, at, "the collapse lost: back where the fight began");
            }
            return;
        }
        // Its leash runs from wherever it is while the place comes down round it.
        if !c.done {
            if let Some(pos) = self.land.footing(m.pos) {
                self.mobs.get_mut(&boss).unwrap().home = pos;
            }
        }
        let total = *c.along.last().unwrap();
        let mut c = c;
        if !c.done {
            c.front = (c.front + c.speed * dt).min(total);
            if c.front >= total {
                c.done = true;
                let name = self.content.mobs[self.mobs[&boss].def].name.clone();
                self.events.push(Event::Log(format!("{name}'s collapse reached its end")));
                self.yell(boss, c.end_text.clone());
                let pos = self.mobs[&boss].pos;
                self.mobs.get_mut(&boss).unwrap().home = pos;
            }
        }
        if !c.done {
            // Dust at the front.
            if now >= c.dust_at {
                c.dust_at = now + DUST_EVERY;
                let at = route_point(&c.route, &c.along, c.front);
                self.events.push(Event::Missile { caster: boss, spell: DUST, target: Target::Point(at) });
            }
            // Rock behind the front, everywhere: what the buried see.
            if now >= c.buried_at {
                c.buried_at = now + BURIED_EVERY;
                let d = (c.front - self.rng.gen::<f32>() * BURIED_SPAN).max(0.0);
                let at = route_point(&c.route, &c.along, d);
                let damage = self.boss_blow(boss, DEBRIS_SHARE * 0.5);
                self.rocks_fall(boss, at, DEBRIS_RADIUS, DEBRIS_WARN * 0.5, damage, true);
            }
            // A warning fall ahead of the front: keep moving.
            if now >= c.warn_at && c.lead > 0.0 {
                c.warn_at = now + WARN_EVERY;
                let d = (c.front + self.rng.gen::<f32>() * c.lead).min(total);
                let at = route_point(&c.route, &c.along, d);
                let damage = self.boss_blow(boss, FALL_SHARE);
                self.rocks_fall(boss, at, FALL_RADIUS, FALL_WARN, damage, false);
            }
            // The buried crushed; those close ahead of it warned.
            if now >= c.crush_at {
                c.crush_at = now + COLLAPSE_CRUSH_TICK;
                let heroes: Vec<(Id, Vec3, u32)> = self.players.iter().filter(|(_, p)| p.health > 0).map(|(h, p)| (*h, p.pos, p.max_health)).collect();
                for (h, pos, max) in heroes {
                    let Some((along, off)) = route_progress(&c.route, &c.along, pos) else { continue };
                    if off > OFF_ROUTE {
                        continue;
                    }
                    if along < c.front - 1.0 {
                        let damage = ((max as f32) * c.crush * COLLAPSE_CRUSH_TICK).round().max(1.0) as u32;
                        self.hurt_player_by(h, damage, boss, "the falling mine");
                    } else if along < c.front + c.lead {
                        self.events.push(Event::Danger { player: h, secs: COLLAPSE_CRUSH_TICK * 2.0, from: None });
                    }
                }
            }
        }
        self.bosses.collapse = Some(c);
    }

    /// A blow on a boss whose collapse is still coming down: it can't be brought below its hold
    /// before the front reaches the end (the last of the fight is fought there).
    pub(crate) fn boss_hold(&mut self, mid: Id, damage: u32) -> u32 {
        let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == mid && !c.done && c.hold > 0.0) else { return damage };
        let Some(m) = self.mobs.get(&mid) else { return damage };
        let floor = (c.hold * m.max_health as f32).round() as u32;
        let most = m.health.saturating_sub(floor);
        if damage <= most {
            return damage;
        }
        if !c.held {
            let line = c.held_text.clone();
            self.bosses.collapse.as_mut().unwrap().held = true;
            self.yell(mid, line);
        }
        most
    }

    /// A boss whose collapse is coming down holds the line, in place of its thinking, when its
    /// hero has run off ahead of the fight (past the warning band): it walks the route with the
    /// front, smashing as it comes, and hurls rock down the tunnel at whoever ran. The collapse and
    /// the boss set where the fight is, never the heroes. False when its hero is in reach of the
    /// fight (it fights as ever) or for any other creature.
    pub(crate) fn step_boss_holdline(&mut self, id: Id) -> bool {
        let Some(c) = self.bosses.collapse.as_ref().filter(|c| c.boss == id && !c.done) else { return false };
        let (front, lead, total) = (c.front, c.lead, *c.along.last().unwrap());
        let (mut leg, holding) = (c.boss_leg, c.holding);
        let Some(m) = self.mobs.get(&id) else { return false };
        let Some(mobs::Foe::Hero(h)) = m.state.foe() else { return false };
        let Some(hero) = self.players.get(&h).filter(|p| p.health > 0).map(|p| p.pos) else { return false };
        let now = self.clock;
        let (route, along) = (c.route.clone(), c.along.clone());
        let Some((hero_at, hero_off)) = route_progress(&route, &along, hero) else { return false };
        let line = (front + lead + LINE_SLACK).min(total);
        let lost = m.lost_at.is_some_and(|t| now - t >= STUCK);
        if hero_at <= line && hero_off <= OFF_ROUTE && !lost {
            self.bosses.collapse.as_mut().unwrap().holding = false;
            return false;
        }
        let def = m.def;
        // Follow the next native corner, never the chord from the creature to a distant
        // front. A hero on another floor doesn't pull it out of the route either.
        let want = (front + lead * 0.5).min(total);
        if !holding {
            let start = leg.saturating_sub(1);
            let (at, _) = route_progress(&route[start..], &along[start..], m.pos).unwrap();
            leg = along.iter().position(|a| *a > at + 0.01).unwrap_or(route.len() - 1).max(1);
        }
        while leg + 1 < route.len() && dist3(m.pos, route[leg]) < 0.08 { leg += 1; }
        let (boss_at, boss_off) = route_progress(&route[leg-1..=leg], &along[leg-1..=leg], m.pos).unwrap();
        let to = if boss_off > 1.0 {
            route_point(&route, &along, boss_at)
        } else {
            route_point(&route, &along, want.min(along[leg]).max(boss_at))
        };
        let c = self.bosses.collapse.as_mut().unwrap();
        c.boss_leg = leg;
        c.holding = true;
        if let Some(to) = self.land.footing(to).filter(|_| boss_at < want || boss_off > 1.0) {
            let speed = self.content.mobs[def].run_speed;
            let end = self.mobs[&id].path.last().copied();
            if end.is_none_or(|end| dist3(end, to) > 0.5) { self.go(id, to, speed); }
        } else if !self.mobs[&id].path.is_empty() {
            self.halt(id);
        }
        // Rock hurled down the tunnel at the hero who ran.
        let c = self.bosses.collapse.as_mut().unwrap();
        if now >= c.hurl_at {
            c.hurl_at = now + HURL_EVERY;
            let damage = self.boss_blow(id, DEBRIS_SHARE);
            self.events.push(Event::Emote { id, emote: mobs::ROAR_EMOTE });
            self.rocks_fall(id, hero, DEBRIS_RADIUS, DEBRIS_WARN, damage, true);
        }
        true
    }

    /// Whether a boss's collapse is still coming down: it chases its heroes however far they run.
    pub(crate) fn boss_unleashed(&self, id: Id) -> bool {
        self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id && !c.done)
    }

    pub(crate) fn boss_fight_home(&self, id: Id) -> Option<Vec3> {
        self.bosses.began.get(&id).copied()
    }

    /// The collapse's front, as far along its route as it has come (for tests and the log).
    pub(crate) fn collapse_front(&self) -> Option<(f32, bool)> {
        self.bosses.collapse.as_ref().map(|c| (c.front, c.done))
    }

    /// Whether a boss is on its run (its phases wait until it has taken up the fight again).
    pub(crate) fn boss_running(&self, mid: Id) -> bool {
        self.bosses.runs.contains_key(&mid)
    }

    /// One step of a boss on its run, in place of its thinking: along its route, rocks behind it,
    /// then waiting at the end for a hero. False once it fights again (or for any other creature).
    pub(crate) fn step_boss_run(&mut self, id: Id) -> bool {
        let Some(run) = self.bosses.runs.get(&id).cloned() else { return false };
        let Some(m) = self.mobs.get(&id) else {
            self.bosses.runs.remove(&id);
            return false;
        };
        // Its fight lost on the way (every hero down or gone): the run ends, and it heads home
        // to be whole again where its fight began.
        if matches!(m.state, MobState::Return | MobState::Idle { .. }) {
            self.bosses.runs.remove(&id);
            return false;
        }
        let now = self.clock;
        let def = m.def;
        let Some(e) = self.content.mobs[def].phases.get(run.phase).cloned() else {
            self.bosses.runs.remove(&id);
            return false;
        };
        let pos = m.pos;
        let last = *e.run_to.last().unwrap();
        if run.arrived.is_none() {
            let to = e.run_to[run.leg.min(e.run_to.len() - 1)];
            // Headway: where it last moved a yard, and when.
            if dist2d(pos, run.went_from) >= 1.0 {
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.went_from = pos;
                r.went_at = now;
            }
            if dist2d(pos, to) < 1.5 && (pos[2] - to[2]).abs() < 4.0 {
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.leg += 1;
                let (leg, of) = (r.leg, e.run_to.len());
                let name = self.content.mobs[def].name.clone();
                self.events.push(Event::Log(format!("{name} on its run: point {leg} of {of} at {:.0} {:.0} {:.0}, {:.1} s in", pos[0], pos[1], pos[2], now - run.leg_at)));
                if leg >= of {
                    self.bosses.runs.get_mut(&id).unwrap().arrived = Some(now);
                    self.halt(id);
                    return true;
                }
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.leg_at = now;
                r.leg_from = pos;
                let next = e.run_to[leg];
                let speed = self.content.mobs[def].run_speed * RUN_HASTE;
                self.go(id, next, speed);
            } else if self.mobs[&id].lost_at.is_some_and(|t| now - t >= STUCK)
                || (now - run.went_at >= STUCK && dist2d(pos, run.went_from) < 1.0)
                || now - run.leg_at > dist2d(run.leg_from, to) / (self.content.mobs[def].run_speed * RUN_HASTE).max(1.0) * LEG_SLACK + STUCK
            {
                // No way along this leg, no headway on it, or far longer on it than a run along
                // it takes: set down at its end rather than standing in the wall or working along
                // a ledge (the log says where, to be looked into).
                self.set_back(id, to, "boss run, no headway on the leg");
                let r = self.bosses.runs.get_mut(&id).unwrap();
                r.went_from = to;
                r.went_at = now;
                r.leg_at = now;
                r.leg_from = to;
            } else if self.mobs[&id].path.is_empty() {
                let speed = self.content.mobs[def].run_speed * RUN_HASTE;
                self.go(id, to, speed);
            }
            // Rocks shaken loose where it has just been.
            if e.run_falls > 0.0 && now >= run.falls_at {
                self.bosses.runs.get_mut(&id).unwrap().falls_at = now + e.run_falls;
                let damage = self.boss_blow(id, FALL_SHARE);
                self.rocks_fall(id, pos, FALL_RADIUS, FALL_WARN, damage, false);
            }
            return true;
        }
        // At the route's end: it waits for a hero to come (not a lone one ambushed the moment they
        // step in, nor the party left to find it standing in a doorway forever).
        let near = if e.wait_for > 0.0 { e.wait_for } else { WAIT_FOR };
        let come = self
            .players
            .iter()
            .filter(|(_, p)| p.health > 0 && dist2d(p.pos, last) <= near && (p.pos[2] - last[2]).abs() < 8.0)
            .min_by(|a, b| dist2d(a.1.pos, last).total_cmp(&dist2d(b.1.pos, last)))
            .map(|(h, _)| *h);
        let waited = run.arrived.is_some_and(|t| now - t >= WAIT_MOST);
        if come.is_none() && !waited {
            return true;
        }
        self.bosses.runs.remove(&id);
        let m = self.mobs.get_mut(&id).unwrap();
        m.home = last;
        m.think = 0.0;
        if let Some(h) = come {
            m.state = MobState::Chase { foe: mobs::Foe::Hero(h) };
        }
        if e.arrive_kit != 0 {
            self.events.push(Event::Visual { id, kit: e.arrive_kit, by: None });
        }
        self.yell(id, e.arrive_text.clone());
        false
    }

    /// A boss whose fight was lost is home again (`MobState::Return`): one that ran is put back
    /// where its fight began, whole, and its blows burn no more.
    pub(crate) fn boss_home(&mut self, id: Id) {
        self.bosses.runs.remove(&id);
        self.bosses.burning.remove(&id);
        if let Some(at) = self.bosses.began.remove(&id) {
            if let Some(m) = self.mobs.get_mut(&id) {
                m.home = at;
            }
            self.set_back(id, at, "boss put back where its fight began");
        }
    }

    /// A boss is down: its beats end (the ground it set burning burns out).
    pub(crate) fn boss_gone(&mut self, id: Id) {
        self.bosses.runs.remove(&id);
        self.bosses.burning.remove(&id);
        self.bosses.began.remove(&id);
        if self.bosses.collapse.as_ref().is_some_and(|c| c.boss == id) {
            self.bosses.collapse = None;
        }
    }

    /// A boss's melee blow leaves burning ground where a heated weapon lands.
    pub(crate) fn boss_blow_lands(&mut self, id: Id, pos: Vec3, facing: f32, range: f32, arc: f32, heavy: bool) {
        let now = self.clock;
        if heavy {
            if let Some(b) = self.bosses.burning.get(&id).cloned() {
                let at = if arc >= 300.0 { pos } else { [pos[0] + facing.cos() * range * 0.7, pos[1] + facing.sin() * range * 0.7, pos[2]] };
                let at = self.ground_near(at[0], at[1], at[2]);
                if b.look != 0 {
                    self.events.push(Event::Missile { caster: id, spell: b.look, target: Target::Point(at) });
                }
                let obj = self.alloc();
                self.objs.insert(obj, Obj { kind: ObjKind::Fire { on: 0 }, pos: at, facing: 0.0 });
                self.events.push(Event::ObjSpawned { id: obj });
                self.bosses.fires.push(Fire { at, radius: b.radius, damage: b.damage, until: now + b.secs, bite_at: now + BURN_TICK * 0.5, owner: id, obj });
            }
        }
    }

    /// The released step's weapon lodges after its ordinary recovery has been recorded.
    pub(crate) fn boss_weapon_lodges(&mut self, id: Id, secs: f32) {
        let recovery = match self.mobs.get(&id).map(|m| &m.state) {
            Some(MobState::Recover { until, .. }) => *until,
            _ => self.clock,
        };
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.stagger_mob(id, secs, false);
        // A short lodge (the ordinary slam) must not shorten a longer normal recovery either.
        if let Some(Mob { state: MobState::Recover { until, .. }, .. }) = self.mobs.get_mut(&id) {
            *until = until.max(recovery);
        }
    }

    /// Burning ground: it bites whoever stands in it, then burns out.
    pub(crate) fn step_boss_fires(&mut self) {
        if self.bosses.fires.is_empty() {
            return;
        }
        let now = self.clock;
        let mut out = Vec::new();
        let mut bites = Vec::new();
        for f in self.bosses.fires.iter_mut() {
            if now >= f.until {
                out.push(f.obj);
                continue;
            }
            if now >= f.bite_at {
                f.bite_at = now + BURN_TICK;
                bites.push((f.at, f.radius, f.damage, f.owner));
            }
        }
        self.bosses.fires.retain(|f| now < f.until);
        for obj in out {
            self.objs.remove(&obj);
            self.events.push(Event::ObjGone { id: obj });
        }
        for (at, radius, damage, owner) in bites {
            let caught: Vec<Id> = self
                .players
                .iter()
                .filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= radius && (p.pos[2] - at[2]).abs() < 3.0)
                .map(|(h, _)| *h)
                .collect();
            for h in caught {
                self.hurt_player_by(h, damage, owner, "burning iron");
            }
        }
    }

    /// (Debug) The way below opened without its bosses beaten: every gate and shortcut (`all`),
    /// or one door by its key. For filming a fight that crosses rooms.
    pub fn unbar(&mut self, key: &str) -> Result<String, String> {
        let d = self.dungeon_def().cloned().ok_or("Not below.")?;
        let doors: Vec<String> = d
            .doors
            .iter()
            .filter(|dr| matches!(dr.kind.as_str(), "gate" | "shortcut") && (key == "all" || key.is_empty() || dr.key == key))
            .map(|dr| dr.key.clone())
            .collect();
        if doors.is_empty() {
            return Err(format!("No gate or shortcut called {key} here."));
        }
        for k in &doors {
            self.below.as_mut().unwrap().open.insert(k.clone());
            self.door_changed(k);
        }
        Ok(format!("Opened {}.", doors.join(", ")))
    }

    /// (Debug) The nearest foe within 40 yards (whose name or key holds `named`, if given) brought down to `share` of its health by this hero's
    /// hand, as a blow would (its phases come as they would): for filming a fight's turns.
    pub fn dev_hurt(&mut self, me: Id, share: f32, named: &str) -> Result<String, String> {
        let named = named.to_lowercase();
        let at = self.players.get(&me).map(|p| p.pos).ok_or("No hero.")?;
        let (mid, health, max) = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0 && !m.friendly && dist2d(m.pos, at) <= 40.0)
            .filter(|(_, m)| named.is_empty() || self.content.mobs[m.def].key.to_lowercase().contains(&named) || self.content.mobs[m.def].name.to_lowercase().contains(&named))
            .min_by(|a, b| dist2d(a.1.pos, at).total_cmp(&dist2d(b.1.pos, at)))
            .map(|(id, m)| (*id, m.health, m.max_health))
            .ok_or("No foe within 40 yards.")?;
        let to = (share.clamp(0.0, 1.0) * max as f32).round() as u32;
        if to >= health {
            return Err("It's already that low.".into());
        }
        let name = self.content.mobs[self.mobs[&mid].def].name.clone();
        if self.mobs[&mid].state.foe().is_none() {
            self.engage(mid, mobs::Foe::Hero(me));
        }
        self.hurt_mob_as(mid, me, health - to, "debug");
        Ok(format!("{name} is down to {}%.", (share * 100.0).round()))
    }

    /// (Debug) Every creature below taken away but those whose name or key holds `keep`, for
    /// filming one fight with nothing else in it. They're back at the next rest or reset.
    pub fn dev_clear_foes(&mut self, keep: &str) -> Result<String, String> {
        let keep = keep.to_lowercase();
        let gone: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| !m.friendly && !self.content.mobs[m.def].inert())
            .filter(|(_, m)| {
                let d = &self.content.mobs[m.def];
                keep.is_empty() || !(d.key.to_lowercase().contains(&keep) || d.name.to_lowercase().contains(&keep))
            })
            .map(|(id, _)| *id)
            .collect();
        for id in &gone {
            self.mobs.remove(id);
            self.events.push(Event::MobGone { id: *id });
        }
        Ok(format!("Took away {} creatures.", gone.len()))
    }

    /// This share of the boss's own blow (as hard as its fight has grown).
    fn boss_blow(&self, id: Id, share: f32) -> u32 {
        let d = self.mobs.get(&id).map_or(0, |m| self.content.mobs[m.def].attack.damage);
        (d as f32 * share * self.foe_blow_share(id)).round().max(1.0) as u32
    }

    /// Rocks come down on `at`: dust there first (`warn` seconds), then the fall, which strikes
    /// whoever is in it (heavy: thrown down).
    fn rocks_fall(&mut self, owner: Id, at: Vec3, radius: f32, warn: f32, damage: u32, heavy: bool) {
        let now = self.clock;
        let at = self.ground_near(at[0], at[1], at[2]);
        self.events.push(Event::Missile { caster: owner, spell: DUST, target: Target::Point(at) });
        let mark = self.mark_ground(at, radius, warn, DUST);
        self.missiles.push(combat::Missile { owner, from: at, at, radius, damage, blockable: false, heavy, land_at: now + warn, mark, look: DUST });
        let near: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= radius + 2.0).map(|(h, _)| *h).collect();
        for v in near {
            self.events.push(Event::Danger { player: v, secs: warn, from: None });
        }
    }
}

fn dist3(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// The point `d` yards along a route (its points, and how far along each is).
fn route_point(route: &[Vec3], along: &[f32], d: f32) -> Vec3 {
    for i in 0..route.len().saturating_sub(1) {
        if d <= along[i + 1] || i + 2 == route.len() {
            let len = (along[i + 1] - along[i]).max(1.0e-3);
            let t = ((d - along[i]) / len).clamp(0.0, 1.0);
            let (a, b) = (route[i], route[i + 1]);
            return [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t];
        }
    }
    route[0]
}

/// How far along a route a spot is (its nearest point on the route, by height too, so a floor
/// above isn't taken for one below), and how far off the route it stands.
fn route_progress(route: &[Vec3], along: &[f32], pos: Vec3) -> Option<(f32, f32)> {
    let mut best: Option<(f32, f32)> = None;
    for i in 0..route.len().saturating_sub(1) {
        let (a, b) = (route[i], route[i + 1]);
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ap = [pos[0] - a[0], pos[1] - a[1], pos[2] - a[2]];
        let len2 = (ab[0] * ab[0] + ab[1] * ab[1] + ab[2] * ab[2]).max(1.0e-6);
        let t = ((ap[0] * ab[0] + ap[1] * ab[1] + ap[2] * ab[2]) / len2).clamp(0.0, 1.0);
        let q = [a[0] + ab[0] * t, a[1] + ab[1] * t, a[2] + ab[2] * t];
        let off = dist3(pos, q);
        let at = along[i] + t * (along[i + 1] - along[i]);
        if best.is_none_or(|(_, o)| off < o) {
            best = Some((at, off));
        }
    }
    best
}
