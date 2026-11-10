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
//!
//! **How the falling mine looks** (`Looks`, Corey on the footage, Nov 9: "NO falling rocks, no
//! debris, the map doesn't appear to be falling apart"). None of it strikes, warns or times
//! anything: the blows, their warnings and the front's crush are exactly as they were.
//! - Every fall of rock (`rocks_fall`) shows a rock dropping from overhead onto the spot as its
//!   blow lands, its ring on the floor till then (markers or not), and a burst of rock and a
//!   shake for heroes beside it where it lands. The rock stays.
//! - The collapse's front brings a big rock down every few yards (`WALL_STEP`), and a timber
//!   cracks and swings down across the tunnel now and then (`BEAM_STEP`): the tunnel behind is
//!   buried under them. Dust rolls ahead of the front, a dusty line marks where it is, the
//!   ground rumbles, and a hero near it feels the shaking.
//! - The 45% ring (`rocks_round`) brings a beam down over each of its falls, a ring of timbers
//!   round the boss.
//! - Everything that came down stays (at most `RUBBLE_MOST`, the oldest going first) until the
//!   fight is lost or the dungeon resets. They're the props the finished ruin is built of
//!   (`htl-server`'s `aftermath.rs`): the dark Burning Steppes rocks and a dig site's timber.
//!   The pieces have no hull: a hero or the boss walks through them, as before.

use std::collections::VecDeque;

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

// --- how it looks (`Looks`): nothing below strikes, warns or times anything ----------------------

/// The props, by `GameObjectDisplayInfo` id, measured from the installed meshes (`godisp`), the
/// ones the finished ruin is built of: `BurningSteppesBoulders05.mdx` (a broken dark rock, z
/// 0.41..3.61), `BurningSteppesBoulders01.mdx` (a tall dark boulder, z -0.10..9.42), and the dig
/// site's `ExcavationBarrierPole.mdx` (a timber post, z -5.77..23.87 along its up, 2.4 thick).
pub(crate) const ROCK: u32 = 5741;
pub(crate) const BOULDER: u32 = 5737;
pub(crate) const BEAM: u32 = 1508;
const ROCK_BOTTOM: f32 = 0.41;
const BOULDER_BOTTOM: f32 = -0.10;
/// A rock is sunk this much of its size below its lowest point, as the ruin's are, so its open
/// underside stays under the floor.
const EMBED: f32 = 0.4;
const BEAM_LEN: f32 = 23.87;
const BEAM_HALF: f32 = 1.2;
/// Sizes: a fall's rock (about 1.7 by 2.3 yd, a yard high; a heavy fall's a little more), the
/// front's rocks (3 to 4 yd across, nearly 2 high) and its boulders, and a beam (7 yd long).
const CHUNK_SCALE: f32 = 0.3;
const CHUNK_HEAVY: f32 = 0.36;
const WALL_SCALES: [f32; 4] = [0.55, 0.62, 0.48, 0.68];
const WALL_BOULDER: f32 = 0.27;
pub(crate) const BEAM_SCALE: f32 = 0.3;
/// Rock falls from this high over the floor (lower where a floor lies overhead, never under
/// `DROP_LOW`), in the last `ROCK_SECS` before its blow lands.
const DROP_HIGH: f32 = 5.0;
const DROP_LOW: f32 = 2.0;
pub(crate) const ROCK_SECS: f32 = 0.45;
/// A beam shows cracked overhead `BEAM_HOLD`, then comes down over `BEAM_SECS`: its loose end
/// swings to the floor (the first `BEAM_SWING` of it), then the rest crashes down flat.
const BEAM_HOLD: f32 = 0.3;
pub(crate) const BEAM_SECS: f32 = 0.8;
const BEAM_SWING: f32 = 0.55;
/// The front's wall: a rock down at the front every this far along the route, a beam every this far.
pub(crate) const WALL_STEP: f32 = 3.0;
pub(crate) const BEAM_STEP: f32 = 14.0;
/// The most pieces left lying (the oldest, farthest behind, goes first), and in the air at once.
pub(crate) const RUBBLE_MOST: usize = 400;
const FALLING_MOST: usize = 24;
/// `Plant Thornling`'s ground look (`DustCloud_Land.mdx`, Charge's dust, nothing but dust): the
/// dust rolling ahead of the front, and thrown up where a piece lands. From an unseen stand-in on
/// the spot (its own cast pose isn't anyone's). Not `Rocky Ground Impact` (26271): on film (take
/// 344, and Smite's slam) its boulders show pink and linger.
const DUST_ROLL: u32 = 22792;
const LAND_DUST: u32 = DUST_ROLL;
/// The front's line and a landing's ring on the floor (drawn whatever the markers), dust brown.
const DUST_COLOUR: &str = "A88E6A";
/// `DeadmineDoor01Destroy` (wood smashing) as a beam cracks, `BlackRockDoorSingleDestroy` (stone
/// crashing) as it and the front's rocks land, `EarthquakeCameraShake` (a deep rumble) at the
/// front every `RUMBLE_EVERY`.
const SOUND_CRACK: u32 = 3079;
const SOUND_CRASH: u32 = 6533;
const SOUND_RUMBLE: u32 = 1485;
const RUMBLE_EVERY: f32 = 3.0;
/// The shaking: `BEAT_RUMBLE` for a hero this near the front, each dust beat; `BEAT_ROCKFALL` for
/// one this near a piece landing, at most once in `SHAKE_GAP` (the add-on's `Cues.lua`).
pub(crate) const BEAT_RUMBLE: &str = "rumble";
pub(crate) const BEAT_ROCKFALL: &str = "rockfall";
const RUMBLE_NEAR: f32 = 20.0;
const SHAKE_NEAR: f32 = 6.0;
const SHAKE_GAP: f32 = 0.3;

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
    /// What the falling mine looks like.
    pub(crate) looks: Looks,
}

/// The falling mine's pieces: on their way down, and lying where they fell.
#[derive(Debug, Clone, Default)]
pub(crate) struct Looks {
    falling: Vec<Falling>,
    /// What came down and stays, oldest first: (whose, the prop).
    rubble: VecDeque<(Id, Id)>,
    /// Every piece's tilt (falling or lying), for the drawing.
    tilt: BTreeMap<Id, f32>,
    /// When each hero last shook from a landing.
    shook: BTreeMap<Id, f32>,
    /// A count for the pieces' own variety (never the rules' dice, whose draws stay as they were).
    seq: u32,
    /// (Tests) none of it shown, to prove the rules don't change with it.
    pub(crate) off: bool,
}

#[derive(Debug, Clone)]
struct Falling {
    owner: Id,
    /// Its prop, once it shows.
    id: Option<Id>,
    shows: f32,
    start: f32,
    land: f32,
    display: u32,
    scale: f32,
    path: Path,
    /// Where it lands (the floor), and how wide its ring and the shaking round it.
    at: Vec3,
    ring: f32,
    /// Its sound as it starts down (0: none) and as it lands.
    crack: u32,
    crash: u32,
    landed: bool,
}

#[derive(Debug, Clone, Copy)]
enum Path {
    /// Straight down from `from` to `to` (its resting place), turning as it goes.
    Drop { from: Vec3, to: Vec3, yaw: f32, tilt: (f32, f32) },
    /// A beam hinged overhead at `hinge`, lying along `yaw`: its loose end swings to the floor
    /// (`floor`), then (`flat`) the rest comes down after it.
    Beam { hinge: Vec3, yaw: f32, floor: f32, flat: bool },
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
    /// How far along the route the front's rocks and beams have come down, and the next rumble.
    walled: f32,
    beamed: f32,
    rumble_at: f32,
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
                // The beam over each comes down with its rock, lying round the ring's edge.
                let p = self.ground_near(p[0], p[1], p[2]);
                let yaw = a + std::f32::consts::FRAC_PI_2;
                let hinge = [p[0] - yaw.cos() * 3.0, p[1] - yaw.sin() * 3.0, p[2]];
                let land = self.clock + DEBRIS_WARN;
                self.beam_falls(mid, hinge, yaw, land, true);
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
                walled: 0.0,
                beamed: 0.0,
                rumble_at: now,
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
        self.step_falling_looks();
        let Some(c) = self.bosses.collapse.clone() else { return };
        let now = self.clock;
        let boss = c.boss;
        let Some(m) = self.mobs.get(&boss) else {
            self.bosses.collapse = None;
            self.clear_looks(boss);
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
            // The mine stands again for the next try: what came down is gone.
            self.clear_looks(boss);
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
            // The front's own rock and timbers coming down (for the eye alone).
            self.front_comes_down(&mut c, boss);
            // Dust at the front.
            if now >= c.dust_at {
                c.dust_at = now + DUST_EVERY;
                let at = route_point(&c.route, &c.along, c.front);
                self.events.push(Event::Missile { caster: boss, spell: DUST, target: Target::Point(at) });
                self.front_dust(&mut c, boss);
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
        self.clear_looks(id);
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
        // How it looks: its ring on the floor till it lands, and a rock dropping onto it.
        if !self.bosses.looks.off {
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: radius, arc: 360.0, width: 0.0, secs: warn, colour: mobs::HAZARD_COLOUR.into() });
            let scale = if heavy { CHUNK_HEAVY } else { CHUNK_SCALE };
            self.rock_drops(owner, at, ROCK, scale, now + warn, radius, 0);
        }
    }

    // --- how the falling mine looks --------------------------------------------------------------

    /// A piece of the falling mine (on its way down or lying where it fell), and its tilt: the
    /// server draws it at once with no hull, moved in place as it falls.
    pub fn collapse_piece_tilt(&self, id: Id) -> Option<f32> {
        self.bosses.looks.tilt.get(&id).copied()
    }

    /// The pieces lying where they fell (for tests and the log).
    pub fn collapse_rubble(&self) -> Vec<Id> {
        self.bosses.looks.rubble.iter().map(|(_, id)| *id).collect()
    }

    /// The next of the looks' own count (their variety, never the rules' dice).
    fn look_seq(&mut self) -> u32 {
        let l = &mut self.bosses.looks;
        l.seq = l.seq.wrapping_add(1);
        l.seq
    }

    /// How high over the floor at `at` a piece starts down: the ceiling's height in a tunnel, or
    /// lower where a floor lies overhead (the Foundry's ring over its pit).
    fn drop_height(&self, at: Vec3) -> f32 {
        let mut h = DROP_HIGH;
        while h > DROP_LOW && self.land.slab_between(at[0], at[1], at[2] + 0.3, at[2] + h + 0.5) {
            h -= 0.5;
        }
        h
    }

    /// A rock (`display` at `scale`) drops onto `at`, landing at `land` (shaking the heroes beside
    /// it, `ring` its ring's reach), and stays there.
    #[allow(clippy::too_many_arguments)]
    fn rock_drops(&mut self, owner: Id, at: Vec3, display: u32, scale: f32, land: f32, ring: f32, crash: u32) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let k = self.look_seq();
        let yaw = (k as f32 * 2.399) % std::f32::consts::TAU;
        let bottom = if display == BOULDER { BOULDER_BOTTOM } else { ROCK_BOTTOM };
        let to = [at[0], at[1], at[2] - (bottom + EMBED) * scale];
        let h = self.drop_height(at);
        // In from a little to one side, as rock coming away from a wall.
        let from = [to[0] + yaw.cos() * 0.7, to[1] + yaw.sin() * 0.7, at[2] + h];
        let tilt = (0.5 + (k % 5) as f32 * 0.15, (k % 3) as f32 * 0.04);
        let start = (land - ROCK_SECS).max(self.clock);
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: start,
            start,
            land,
            display,
            scale,
            path: Path::Drop { from, to, yaw, tilt },
            at,
            ring,
            crack: 0,
            crash,
            landed: false,
        });
    }

    /// A beam cracks overhead at `hinge` (its floor beneath), lying along `yaw`, and comes down to
    /// land at `land`: its loose end to the floor, then (`flat`) the rest after it. It stays.
    fn beam_falls(&mut self, owner: Id, hinge: Vec3, yaw: f32, land: f32, flat: bool) {
        if self.bosses.looks.off || self.bosses.looks.falling.len() >= FALLING_MOST {
            return;
        }
        let floor = hinge[2];
        let h = self.drop_height(hinge);
        let start = (land - BEAM_SECS).max(self.clock);
        let path = Path::Beam { hinge: [hinge[0], hinge[1], floor + h], yaw, floor, flat };
        let (foot, _, _) = beam_pose(&path, 1.0);
        let len = BEAM_LEN * BEAM_SCALE;
        // Where it comes down: about the middle of where it lies (or leans).
        let at = [foot[0] + yaw.cos() * len * 0.4, foot[1] + yaw.sin() * len * 0.4, floor];
        self.bosses.looks.falling.push(Falling {
            owner,
            id: None,
            shows: (start - BEAM_HOLD).max(self.clock),
            start,
            land,
            display: BEAM,
            scale: BEAM_SCALE,
            path,
            at,
            ring: len * 0.5,
            crack: SOUND_CRACK,
            crash: SOUND_CRASH,
            landed: false,
        });
    }

    /// The front's own rock (a big one every `WALL_STEP` it comes) and timbers (one across the
    /// tunnel every `BEAM_STEP`): what buries the tunnel behind it.
    fn front_comes_down(&mut self, c: &mut Collapse, boss: Id) {
        if self.bosses.looks.off {
            return;
        }
        let now = self.clock;
        while c.walled + WALL_STEP <= c.front {
            c.walled += WALL_STEP;
            let k = self.look_seq();
            let (p, yaw) = route_pose(&c.route, &c.along, c.walled);
            let side = [-yaw.sin(), yaw.cos()];
            let off = [-1.3, 1.1, 0.2, -0.6, 1.5, -1.6, 0.7][k as usize % 7];
            let at = self.floor_beside(p, [p[0] + side[0] * off, p[1] + side[1] * off]);
            let (display, scale) = if k % 4 == 3 { (BOULDER, WALL_BOULDER) } else { (ROCK, WALL_SCALES[k as usize % 4]) };
            let crash = if k % 2 == 0 { SOUND_CRASH } else { 0 };
            self.rock_drops(boss, at, display, scale, now + ROCK_SECS, 2.0, crash);
        }
        while c.beamed + BEAM_STEP <= c.front {
            c.beamed += BEAM_STEP;
            let k = self.look_seq();
            let (p, yaw) = route_pose(&c.route, &c.along, c.beamed);
            // Hinged up by one wall, across the tunnel to the other.
            let hand = if k % 2 == 0 { 1.0 } else { -1.0 };
            let across = yaw + hand * std::f32::consts::FRAC_PI_2;
            let hinge = self.floor_beside(p, [p[0] - across.cos() * 2.6, p[1] - across.sin() * 2.6]);
            // Every other one stays leaning where its end struck the floor; the rest come down flat.
            self.beam_falls(boss, hinge, across, now + BEAM_HOLD + BEAM_SECS, k % 4 < 2);
        }
    }

    /// Each dust beat at the front: dust rolling on ahead of it, its line across the tunnel, the
    /// rumble, and the shaking for every hero near it.
    fn front_dust(&mut self, c: &mut Collapse, boss: Id) {
        if self.bosses.looks.off {
            return;
        }
        let now = self.clock;
        let total = *c.along.last().unwrap();
        let (p, yaw) = route_pose(&c.route, &c.along, c.front);
        self.events.push(Event::GroundBurst { at: p, yaw: yaw + std::f32::consts::FRAC_PI_2, shape: "line", reach: 3.5, arc: 0.0, width: 1.4, secs: DUST_EVERY + 0.15, colour: DUST_COLOUR.into() });
        for ahead in [1.5, 4.5] {
            let at = route_point(&c.route, &c.along, (c.front + ahead).min(total));
            self.point_look(boss, at, DUST_ROLL);
        }
        if now >= c.rumble_at {
            c.rumble_at = now + RUMBLE_EVERY;
            if let Some(&(_, piece)) = self.bosses.looks.rubble.back() {
                self.events.push(Event::Sound { id: piece, sound: SOUND_RUMBLE });
            }
        }
        let near: Vec<Id> = self.players.iter().filter(|(_, h)| h.health > 0 && dist3(h.pos, p) <= RUMBLE_NEAR).map(|(h, _)| *h).collect();
        for h in near {
            self.events.push(Event::Beat { player: h, beat: BEAT_RUMBLE.into(), on: None });
        }
    }

    /// A spot beside the route's point `p`, on its floor (the route's own height if the spot is
    /// off any floor near it).
    fn floor_beside(&self, p: Vec3, xy: [f32; 2]) -> Vec3 {
        let g = self.ground_near(xy[0], xy[1], p[2]);
        if g[2].is_finite() && (g[2] - p[2]).abs() < 2.0 { g } else { [xy[0], xy[1], p[2]] }
    }

    /// A ground look at `at` from an unseen stand-in on the spot (no caster's pose, and seen
    /// however far its owner is).
    fn point_look(&mut self, owner: Id, at: Vec3, spell: u32) {
        let anchor = self.alloc();
        self.events.push(Event::MissileFrom { anchor, caster: owner, spell, from: at, target: Target::Point(at) });
        self.events.push(Event::ObjGone { id: anchor });
    }

    /// The pieces on their way down move; those landing burst, shake whoever is beside them and
    /// stay. What a reset took away is let go.
    fn step_falling_looks(&mut self) {
        let l = &self.bosses.looks;
        if l.falling.is_empty() && l.rubble.is_empty() && l.tilt.is_empty() {
            return;
        }
        let objs = &self.objs;
        let l = &mut self.bosses.looks;
        l.rubble.retain(|(_, id)| objs.contains_key(id));
        l.falling.retain(|f| f.id.is_none_or(|id| objs.contains_key(&id)));
        l.tilt.retain(|id, _| objs.contains_key(id));
        let now = self.clock;
        let mut landed = Vec::new();
        for k in 0..self.bosses.looks.falling.len() {
            let f = self.bosses.looks.falling[k].clone();
            if now < f.shows {
                continue;
            }
            let t = if f.land > f.start { ((now - f.start) / (f.land - f.start)).clamp(0.0, 1.0) } else { 1.0 };
            let (pos, facing, tilt) = f.path.pose(t);
            match f.id {
                None => {
                    let id = self.alloc();
                    self.objs.insert(id, Obj { kind: ObjKind::Prop { display: f.display, scale: f.scale }, pos, facing });
                    self.bosses.looks.tilt.insert(id, tilt);
                    self.events.push(Event::ObjSpawned { id });
                    if f.crack != 0 {
                        self.events.push(Event::Sound { id, sound: f.crack });
                    }
                    self.bosses.looks.falling[k].id = Some(id);
                }
                Some(id) if now >= f.start => {
                    if let Some(o) = self.objs.get_mut(&id) {
                        o.pos = pos;
                        o.facing = facing;
                    }
                    self.bosses.looks.tilt.insert(id, tilt);
                    self.events.push(Event::ObjChanged { id });
                }
                Some(_) => {}
            }
            if t >= 1.0 && now >= f.start {
                self.bosses.looks.falling[k].landed = true;
                landed.push(k);
            }
        }
        for k in landed {
            let f = self.bosses.looks.falling[k].clone();
            let Some(id) = f.id else { continue };
            // Dust thrown up where it struck, its ring on the floor, its crash.
            self.point_look(f.owner, f.at, LAND_DUST);
            self.events.push(Event::GroundBurst { at: f.at, yaw: 0.0, shape: "ring", reach: f.ring, arc: 360.0, width: 0.0, secs: 0.5, colour: DUST_COLOUR.into() });
            if f.crash != 0 {
                self.events.push(Event::Sound { id, sound: f.crash });
            }
            let shaken: Vec<Id> = self
                .players
                .iter()
                .filter(|(h, p)| p.health > 0 && dist3(p.pos, f.at) <= SHAKE_NEAR.max(f.ring + 2.0)
                    && self.bosses.looks.shook.get(*h).is_none_or(|t| now - t >= SHAKE_GAP))
                .map(|(h, _)| *h)
                .collect();
            for h in shaken {
                self.bosses.looks.shook.insert(h, now);
                self.events.push(Event::Beat { player: h, beat: BEAT_ROCKFALL.into(), on: None });
            }
            let l = &mut self.bosses.looks;
            l.rubble.push_back((f.owner, id));
            while l.rubble.len() > RUBBLE_MOST {
                let (_, old) = l.rubble.pop_front().unwrap();
                l.tilt.remove(&old);
                if self.objs.remove(&old).is_some() {
                    self.events.push(Event::ObjGone { id: old });
                }
            }
        }
        self.bosses.looks.falling.retain(|f| !f.landed);
        self.bosses.looks.shook.retain(|_, t| now - *t < SHAKE_GAP);
    }

    /// Everything of `owner`'s that came down (or is coming down) gone: its fight lost, the mine
    /// stands again.
    fn clear_looks(&mut self, owner: Id) {
        let l = &mut self.bosses.looks;
        let mut gone: Vec<Id> = l.rubble.iter().filter(|(o, _)| *o == owner).map(|(_, id)| *id).collect();
        gone.extend(l.falling.iter().filter(|f| f.owner == owner).filter_map(|f| f.id));
        l.rubble.retain(|(o, _)| *o != owner);
        l.falling.retain(|f| f.owner != owner);
        for id in gone {
            self.bosses.looks.tilt.remove(&id);
            if self.objs.remove(&id).is_some() {
                self.events.push(Event::ObjGone { id });
            }
        }
    }
}

impl Path {
    /// Where its prop stands `t` of the way down (0 to 1, falling faster as it goes): its
    /// position, facing and tilt (about the model's own Y after its facing, the drawing's).
    fn pose(&self, t: f32) -> (Vec3, f32, f32) {
        match *self {
            Path::Drop { from, to, yaw, tilt } => {
                let u = t * t;
                let pos = [from[0] + (to[0] - from[0]) * u, from[1] + (to[1] - from[1]) * u, from[2] + (to[2] - from[2]) * u];
                (pos, yaw + (1.0 - u) * 1.3, tilt.0 + (tilt.1 - tilt.0) * u)
            }
            Path::Beam { .. } => beam_pose(self, t),
        }
    }
}

/// A beam's pose `t` of the way down: the post's foot (its origin) at the hinge, the post along
/// `yaw` tipped from lying level (a tilt of a quarter turn from upright) until its far end strikes
/// the floor; then, coming down flat, its foot drops after it, the far end where it struck.
fn beam_pose(path: &Path, t: f32) -> (Vec3, f32, f32) {
    let Path::Beam { hinge, yaw, floor, flat } = *path else { return ([0.0; 3], 0.0, 0.0) };
    use std::f32::consts::FRAC_PI_2;
    let len = BEAM_LEN * BEAM_SCALE;
    let rest = floor + BEAM_HALF * BEAM_SCALE;
    let d = [yaw.cos(), yaw.sin()];
    // The tilt that puts the far end on the floor, swung from the hinge.
    let struck = (-((hinge[2] - rest) / len).clamp(-1.0, 1.0)).acos();
    let swing = if flat { BEAM_SWING } else { 1.0 };
    if t <= swing {
        let u = (t / swing).powi(2);
        return (hinge, yaw, FRAC_PI_2 + (struck - FRAC_PI_2) * u);
    }
    let end = [hinge[0] + d[0] * len * struck.sin(), hinge[1] + d[1] * len * struck.sin(), rest];
    let u = ((t - swing) / (1.0 - swing)).powi(2);
    let z = hinge[2] + (rest - hinge[2]) * u;
    let reach = (len * len - (z - end[2]).powi(2)).max(0.0).sqrt();
    let foot = [end[0] - d[0] * reach, end[1] - d[1] * reach, z];
    let tilt = ((end[2] - z) / len).clamp(-1.0, 1.0).acos();
    (foot, yaw, tilt)
}

/// The point `d` yards along a route and the way the route runs there.
fn route_pose(route: &[Vec3], along: &[f32], d: f32) -> (Vec3, f32) {
    let a = route_point(route, along, (d - 0.5).max(0.0));
    let b = route_point(route, along, d + 0.5);
    let yaw = (b[1] - a[1]).atan2(b[0] - a[0]);
    (route_point(route, along, d), yaw)
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
