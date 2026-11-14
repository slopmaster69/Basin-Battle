//! Cookie's kitchen (`docs/cookie-encounter.md`): a cleaver pair, a broad ladle and Serve It Hot,
//! and after 3, then 4, then 5 exchanges a greedy scoop at his cauldron that a hero breaks or
//! leaves. The cauldron stands all fight and after his death; Gorged rises beside it, empties it,
//! and swells before his first slap.

use super::mobs::{Foe, MobState, HAZARD_COLOUR, ROAR_EMOTE, SECOND_FORM_WAIT};
use super::*;
use crate::land::clear_line;

#[cfg(test)]
#[path = "kitchen_tests.rs"]
mod tests;

const COOKIE: &str = "dm_cookie";
const GORGED: &str = "dm_cookie_gorged";

/// The exchanges before each scoop: the first, the second, then every later one.
const SCOOP_AFTER: [u32; 3] = [3, 4, 5];
/// How long he scoops before he gulps.
const SCOOP: f32 = 4.0;
/// One dip of the mug into the pot, with a slurp.
const DIP: f32 = 1.0;
/// What breaks a scoop: about 2 s of a lone hero's committed light blows, or one heavy blow
/// (a heavy blow counts `SCOOP_HEAVY` times over while he scoops).
pub(crate) const SCOOP_POSTURE: u32 = 120;
const SCOOP_HEAVY: f32 = 2.5;
/// The reel after the pot is knocked: the long opening.
const REEL: f32 = 5.5;
/// The short opening after the steam cone.
const STEAM_RECOVER: f32 = 1.5;
/// The cough after Gorged's belch.
const COUGH: f32 = 4.0;
/// Helpings in a full pot. Each gulp eats one and heals 4%; a knock only slops some out.
const HELPINGS: u8 = 2;

/// The puddle: hot this long, then its edge puffs drop away one by one; it burns until the last.
const HOT: f32 = 3.0;
const FADE: f32 = 1.2;
const RADIUS: f32 = 2.0;
/// Steam puffs on its edge, and their size: big and close enough to read as a rim from a duel
/// camera (take 341: eight small ones barely showed from above).
const EDGE_PUFFS: usize = 12;
const EDGE_PUFF_SCALE: f32 = 1.5;
/// The splash: a tan ring at the puddle's true edge, held long enough to be seen.
const SPLASH_SECS: f32 = 1.2;
/// Where the pour lands: on the hero's spot, never nearer than this or farther than that.
const POUR_NEAR: f32 = 4.0;
const POUR_FAR: f32 = 8.0;
/// The mug's arc: from his hand, this high over the straight line at its middle, in legs.
const HAND: f32 = 1.0;
const ARC_LIFT: f32 = 4.0;
const LEGS: usize = 4;
const LEG_TICK: f32 = 0.05;
/// A shape on the ground is drawn again this often, so it ends soon after what it shows does.
const REDRAW: f32 = 0.5;

/// Throw Dark Iron Ale: a flying mug at 30 yd/s.
const MUG: u32 = 14814;
const MUG_SPEED: f32 = 30.0;
/// The metal tankard item display he holds to pour and to scoop (no ladle or bowl in 1.12.1).
const MUG_HELD: u32 = 2723;
/// Cauldron, Steam02 particles, Bubbles01 particles.
const POT: u32 = 216;
const STEAM: u32 = 431;
const BUBBLES: u32 = 42;
/// Frost Breath's white cone for the steam; Acid Breath's liquid gush for the belch.
const STEAM_KIT: u32 = 2610;
const BELCH_KIT: u32 = 2373;
/// Murloc clips: Attack1H, CombatWound, SpellCast, AttackThrown, HoldThrown, LoadThrown.
const BANG_CLIP: u16 = 17;
const OPEN_CLIP: u16 = 9;
const BREATH_CLIP: u16 = 32;
const THROW_CLIP: u16 = 107;
const HOLD_CLIP: u16 = 111;
const DIP_CLIP: u16 = 112;
/// Sounds: a mace on metal (the pot banged), Blood Boil's bubbling, Drinking (a slurp), Eat (a
/// gulp), a medium splash, a steam burst, MurlocWound, MurlocAggro.
const BANG: u32 = 971;
const BUBBLING: u32 = 3135;
const SLURP: u32 = 3373;
const GULP: u32 = 45;
const SPLASH: u32 = 1096;
const HISS: u32 = 4858;
const WOUND: u32 = 418;
const AGGRO: u32 = 416;
/// Stew and steam, drawn on the ground (markers off); the danger is `HAZARD_COLOUR`.
const STEW_COLOUR: &str = "D9A35B";
const STEAM_COLOUR: &str = "F2F2F2";

/// Gorged walks to the pot, gulps it empty, and swells over this long.
const GORGED_WALK: f32 = 6.0;
const GORGED_GULP: f32 = 1.8;
const SWELL: f32 = 1.5;
/// The swell runs past his full size by this much, then settles back: a gulp's heave.
const SWELL_OVER: f32 = 0.08;
/// How long a cook waits for a hero he can find no way to, and how soon he evades meanwhile.
pub(crate) const HOLD_ON: f32 = 10.0;
pub(crate) const EVADE_AFTER: f32 = 1.0;
/// The belch's swelling pulse at its fullest.
const PULSE: f32 = 0.1;

#[derive(Default)]
pub(crate) struct Kitchen {
    cooks: BTreeMap<Id, Cook>,
    /// Every cauldron, by its prop: it outlives the cook who stands at it.
    pots: BTreeMap<Id, Pot>,
}

struct Pot {
    at: Vec3,
    /// Where a cook stands to scoop: beside it, side-on.
    spot: Vec3,
    helpings: u8,
    /// The steam and bubbles over it: one plume for each helping left.
    stew: Vec<Id>,
    /// Knocked: its steam is down until then.
    down_until: Option<f32>,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Hand {
    /// What his definition holds (Cookie's cleaver, Gorged's bare hands).
    Own,
    Mug,
    Empty,
}

struct Cook {
    pot: Option<Id>,
    began: bool,
    pulled: bool,
    /// Where the rhythm stands (every exchange moves it on).
    turn: u32,
    /// Exchanges since the last scoop (or the pull).
    since: u32,
    scoops: u8,
    healed: u8,
    stage: Option<Stage>,
    patch: Option<Patch>,
    /// The aim of the pour now winding up.
    aim: Option<Vec3>,
    /// The mug's later legs: when each flies, from where, to where.
    legs: Vec<(f32, Vec3, Vec3)>,
    /// Short-lived steam (a breath's cone) and when each puff goes.
    puffs: Vec<(Id, f32)>,
    hand: Hand,
    thrown_until: f32,
    /// Gorged's drawn size now (Cookie's own stays as authored).
    scale: f32,
    /// The belch's swelling: since when, and when it was last drawn.
    pulse: Option<(f32, f32)>,
}

#[derive(Clone, Copy, Debug)]
enum Stage {
    /// The ladle... the mug banged on the pot twice as the fight begins.
    Pull { since: f32, bangs: u8 },
    Seeking { until: f32 },
    Scooping { until: f32, next_dip: f32 },
    Coughing { until: f32 },
    Rising(Rise),
}

#[derive(Clone, Copy, Debug)]
enum Rise {
    Walking { until: f32 },
    Gulping { until: f32, next_dip: f32 },
    Swelling { since: f32, drawn: f32 },
    Waiting { until: f32 },
}

struct Patch {
    at: Vec3,
    props: Vec<Id>,
    hot_at: Option<f32>,
    next_tick: f32,
    damage: u32,
    landed: bool,
    next_ring: f32,
}

impl Survival {
    pub(crate) fn is_kitchen(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| matches!(self.content.mobs[m.def].key.as_str(), COOKIE | GORGED))
    }

    fn is_gorged(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == GORGED)
    }

    fn kitchen_move(&self, id: Id) -> Option<&str> {
        let m = self.mobs.get(&id)?;
        let (k, _) = m.pattern?;
        Some(&self.content.mobs[m.def].patterns.get(k)?.name)
    }

    pub(crate) fn kitchen_committed(&self, id: Id) -> bool {
        self.is_kitchen(id) && matches!(self.kitchen_move(id), Some("serve it hot" | "steam" | "belch"))
    }

    fn kitchen_winding(&self, id: Id, name: &str) -> bool {
        self.kitchen_move(id) == Some(name) && matches!(self.mobs.get(&id).map(|m| &m.state), Some(MobState::Windup { .. }))
    }

    pub(crate) fn kitchen_scooping(&self, id: Id) -> bool {
        self.kitchen.cooks.get(&id).is_some_and(|c| matches!(c.stage, Some(Stage::Scooping { .. })))
    }

    /// Particle-only cues must show now. The cauldron keeps its normal fade and collision hull.
    pub fn is_kitchen_prop(&self, prop: Id) -> bool {
        self.kitchen.cooks.values().any(|c| c.patch.as_ref().is_some_and(|p| p.props.contains(&prop))
            || c.puffs.iter().any(|(id, _)| *id == prop))
            || self.kitchen.pots.values().any(|p| p.stew.contains(&prop))
    }

    /// What a cook holds when it isn't his own weapon: the tankard, or nothing (the mug thrown).
    pub fn kitchen_hand(&self, id: Id) -> Option<u32> {
        match self.kitchen.cooks.get(&id)?.hand {
            Hand::Own => None,
            Hand::Mug => Some(MUG_HELD),
            Hand::Empty => Some(0),
        }
    }

    /// Gorged's drawn size while he swells (and after): his body as the clients draw it.
    pub fn kitchen_scale(&self, id: Id) -> Option<f32> {
        self.is_gorged(id).then(|| self.kitchen.cooks.get(&id).map(|c| c.scale)).flatten()
    }

    fn kitchen_prop(&mut self, at: Vec3, display: u32, scale: f32) -> Id {
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Prop { display, scale }, pos: at, facing: 0.0 });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    fn kitchen_drop_prop(&mut self, id: Id) {
        if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
    }

    fn kitchen_floor(&self, from: Vec3, x: f32, y: f32) -> Option<Vec3> {
        let p = self.ground_near(x, y, from[2]);
        ((p[2] - from[2]).abs() < 1.5 && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2]) && self.land.walk_clear(from, p)).then_some(p)
    }

    // --- the pot ------------------------------------------------------------------------------

    /// A pot by Cookie's post: the one already standing there (left from a wipe), else a new one.
    fn kitchen_pot_for(&mut self, home: Vec3) -> Id {
        let at = self.kitchen_floor(home, home[0], home[1] + 1.8).unwrap_or(home);
        if let Some((&prop, _)) = self.kitchen.pots.iter().find(|(_, p)| dist2d(p.at, at) < 4.0) {
            let p = self.kitchen.pots.get_mut(&prop).unwrap();
            // A fresh pull: a full pot again.
            p.helpings = HELPINGS;
            p.down_until = None;
            self.kitchen_show_stew(prop);
            return prop;
        }
        // He scoops from its side, so the pot and his dipping arm are seen side-on.
        let spot = [[1.6, 0.0], [-1.6, 0.0], [0.0, -1.6]].iter()
            .find_map(|[dx, dy]| self.kitchen_floor(home, at[0] + dx, at[1] + dy)).unwrap_or(home);
        let prop = self.kitchen_prop(at, POT, 1.0);
        self.kitchen.pots.insert(prop, Pot { at, spot, helpings: HELPINGS, stew: Vec::new(), down_until: None });
        self.kitchen_show_stew(prop);
        prop
    }

    /// The pot's steam and bubbles as its helpings stand: two plumes full, one half, none empty.
    fn kitchen_show_stew(&mut self, pot: Id) {
        let Some(p) = self.kitchen.pots.get_mut(&pot) else { return };
        let old = std::mem::take(&mut p.stew);
        let (at, helpings, down) = (p.at, p.helpings, p.down_until.is_some());
        for prop in old { self.kitchen_drop_prop(prop); }
        if down || helpings == 0 { return; }
        let mut stew = vec![self.kitchen_prop([at[0], at[1], at[2] + 1.35], BUBBLES, 1.5)];
        for k in 0..helpings {
            let dx = if k == 0 { 0.0 } else { 0.35 };
            stew.push(self.kitchen_prop([at[0] + dx, at[1], at[2] + 1.4], STEAM, 1.0));
        }
        self.kitchen.pots.get_mut(&pot).unwrap().stew = stew;
    }

    fn kitchen_drop_pot(&mut self, pot: Id) {
        if let Some(p) = self.kitchen.pots.remove(&pot) {
            for prop in p.stew { self.kitchen_drop_prop(prop); }
        }
        self.kitchen_drop_prop(pot);
    }

    // --- cooks --------------------------------------------------------------------------------

    pub(crate) fn clean_kitchens(&mut self) {
        let stale: Vec<_> = self.kitchen.cooks.iter().filter(|(id, c)| {
            !self.mobs.get(id).is_some_and(|m| m.health > 0 && !matches!(m.state, MobState::Return)
                && (!c.began || m.state.foe().is_some()))
        }).map(|(id, _)| *id).collect();
        for id in stale { self.remove_kitchen(id); }
        let fresh: Vec<_> = self.mobs.iter().filter(|(id, m)| self.is_kitchen(**id)
            && m.health > 0 && !m.hidden && m.rise_at.is_none() && !matches!(m.state, MobState::Return)
            && !self.kitchen.cooks.contains_key(id)).map(|(id, _)| *id).collect();
        for id in fresh {
            let (home, def) = (self.mobs[&id].home, self.mobs[&id].def);
            let pot = (self.content.mobs[def].key == COOKIE).then(|| self.kitchen_pot_for(home));
            let scale = self.content.mobs[def].scale;
            self.kitchen.cooks.insert(id, Cook::new(pot, scale));
        }
        let ids: Vec<_> = self.kitchen.cooks.keys().copied().collect();
        for id in ids { self.step_supper(id); }
    }

    /// The cook goes (death, home, a wipe, the handoff); his cauldron stays where it stands.
    pub(crate) fn remove_kitchen(&mut self, id: Id) {
        let Some(c) = self.kitchen.cooks.remove(&id) else { return };
        self.pending.retain(|p| p.attacker != id);
        if let Some(MobState::Windup { mark: Some(mark), .. }) = self.mobs.get(&id).map(|m| m.state.clone()) {
            self.unmark(mark);
        }
        if let Some(p) = c.patch { for prop in p.props { self.kitchen_drop_prop(prop); } }
        for (prop, _) in c.puffs { self.kitchen_drop_prop(prop); }
        let alive = self.mobs.get(&id).is_some_and(|m| m.health > 0);
        if c.stage.is_some() && alive { self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None }); }
        if c.hand != Hand::Own && alive { self.events.push(Event::MobGear { id }); }
        if c.pulse.is_some() && alive { self.events.push(Event::Grow { id, scale: c.scale }); }
        // A knocked pot's steam comes back as the cook recovers, which a death or a wipe cuts
        // short. It stays as it stood: Gorged and the next pull find it steaming.
        if let Some(pot) = c.pot {
            if self.kitchen.pots.get(&pot).is_some_and(|p| p.down_until.is_some()) {
                self.kitchen.pots.get_mut(&pot).unwrap().down_until = None;
                self.kitchen_show_stew(pot);
            }
        }
    }

    /// A dungeon reset: every cook and every pot goes.
    pub(crate) fn reset_kitchens(&mut self) {
        let ids: Vec<_> = self.kitchen.cooks.keys().copied().collect();
        for id in ids { self.remove_kitchen(id); }
        let pots: Vec<_> = self.kitchen.pots.keys().copied().collect();
        for pot in pots { self.kitchen_drop_pot(pot); }
    }

    /// The rhythm, not a health phase. A second pour never lands on the first.
    pub(crate) fn kitchen_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let c = self.kitchen.cooks.get(&id)?;
        let name = if d.key == GORGED {
            if c.turn % 2 == 0 { "slaps" } else { "belch" }
        } else {
            match c.turn % 3 { 0 => "cleaver", 1 => "ladle", _ if c.patch.is_some() => "cleaver", _ => "serve it hot" }
        };
        d.patterns.iter().position(|p| p.name == name
            && dist <= if p.max_range > 0.0 { p.max_range } else { p.steps[0].range.unwrap_or(d.attack.range) + size - 0.3 })
    }

    /// Where the pour lands: where the hero stood as he began, pushed out to at least 4 yd and
    /// in to at most 8, on a dry floor he can see.
    fn pour_point(&self, from: Vec3, hero: Vec3, dir: f32) -> Option<Vec3> {
        let d = dist2d(from, hero).clamp(POUR_NEAR, POUR_FAR);
        let dir = if dist2d(from, hero) > 0.1 { facing(from, hero) } else { dir };
        [0.0, -0.7, 0.7, -1.4].iter().map(|k| (d + k).clamp(POUR_NEAR - 0.5, POUR_FAR + 0.5))
            .find_map(|r| self.kitchen_floor(from, from[0] + dir.cos() * r, from[1] + dir.sin() * r))
    }

    /// The optional floor marker matches the puddle, rather than advertising a melee arc.
    pub(crate) fn kitchen_pour_mark(&mut self, id: Id, tele: Telegraph) -> Option<Id> {
        if !self.is_kitchen(id) || self.kitchen_move(id) != Some("serve it hot") { return None; }
        let m = &self.mobs[&id];
        let hero = tele.foe.and_then(|h| self.players.get(&h)).map_or(m.pos, |p| p.pos);
        let at = self.pour_point(m.pos, hero, m.facing)?;
        self.kitchen.cooks.get_mut(&id)?.aim = Some(at);
        Some(self.mark_as(at, RADIUS, tele.secs, Some(Telegraph { reach: RADIUS, arc: 360.0,
            at: Some((at, 0.0)), foe: None, ..tele })))
    }

    pub(crate) fn kitchen_windup(&mut self, id: Id) {
        if !self.kitchen_committed(id) || !self.kitchen.cooks.contains_key(&id) { return; }
        let name = self.kitchen_move(id).unwrap().to_owned();
        let (pos, dir) = (self.mobs[&id].pos, self.mobs[&id].facing);
        let a = self.attack_now(id);
        let now = self.clock;
        // A held lift (the mug up, or a breath drawn in) until the real release.
        self.events.push(Event::HoldPose { id, clip: HOLD_CLIP, secs: a.windup, from: None });
        if name == "serve it hot" {
            let c = self.kitchen.cooks.get_mut(&id).unwrap();
            c.patch = None;
            if let Some(at) = c.aim.take() {
                c.patch = Some(Patch { at, props: Vec::new(), hot_at: None, next_tick: 0.0, damage: a.damage, landed: false, next_ring: now + REDRAW });
                // Where it will land, drawn at its true size from the first moment, markers or not.
                self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: RADIUS, arc: 360.0,
                    width: 0.0, secs: REDRAW + 0.05, colour: HAZARD_COLOUR.into() });
            }
            self.events.push(Event::Sound { id, sound: BUBBLING });
        } else {
            // The cone drawn to the reach and width that hit, the whole wind-up.
            self.events.push(Event::GroundBurst { at: pos, yaw: dir, shape: "wedge", reach: a.range + 0.4, arc: a.arc,
                width: 0.0, secs: a.windup, colour: HAZARD_COLOUR.into() });
            if name == "belch" { self.kitchen.cooks.get_mut(&id).unwrap().pulse = Some((now, now)); }
        }
        self.kitchen_hand_now(id);
    }

    /// The pour's damage belongs to its puddle, the breaths' to the generic cone they draw.
    pub(crate) fn kitchen_release(&mut self, id: Id) -> bool {
        if !self.is_kitchen(id) || !self.kitchen.cooks.contains_key(&id) { return false; }
        let Some(name) = self.kitchen_move(id).map(str::to_owned) else { return false };
        let now = self.clock;
        let (pos, dir) = (self.mobs[&id].pos, self.mobs[&id].facing);
        let a = self.attack_now(id);
        if name == "steam" || name == "belch" {
            self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
            self.events.push(Event::Clip { id, clip: BREATH_CLIP, take: 0, from: None, secs: 0.8, at: 0.15, body: ClipBody::Full });
            self.events.push(Event::Visual { id, kit: if name == "steam" { STEAM_KIT } else { BELCH_KIT }, by: None });
            self.events.push(Event::Sound { id, sound: HISS });
            let colour = if name == "steam" { STEAM_COLOUR } else { STEW_COLOUR };
            self.events.push(Event::GroundBurst { at: pos, yaw: dir, shape: "wedge", reach: a.range + 0.4, arc: a.arc,
                width: 0.0, secs: 0.8, colour: colour.into() });
            // Steam through the whole cone, not down its middle.
            let half = a.arc.to_radians() / 2.0;
            for (r, k) in [(0.3, 0.0), (0.55, -0.6), (0.55, 0.6), (0.85, -0.75), (0.85, 0.0), (0.85, 0.75)] {
                let (r, yaw) = (r * a.range, dir + k * half);
                let at = self.ground_near(pos[0] + yaw.cos() * r, pos[1] + yaw.sin() * r, pos[2]);
                let prop = self.kitchen_prop(at, STEAM, 1.3);
                self.kitchen.cooks.get_mut(&id).unwrap().puffs.push((prop, now + 1.0));
            }
            let c = self.kitchen.cooks.get_mut(&id).unwrap();
            if c.pulse.take().is_some() { let scale = c.scale; self.events.push(Event::Grow { id, scale }); }
            return false;
        }
        if name != "serve it hot" { return false; }
        self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
        self.events.push(Event::Clip { id, clip: THROW_CLIP, take: 0, from: None, secs: 0.6, at: 0.13, body: ClipBody::Full });
        let c = self.kitchen.cooks.get_mut(&id).unwrap();
        c.thrown_until = now + a.recovery.max(0.8);
        let Some(p) = c.patch.as_mut().filter(|p| p.hot_at.is_none()) else { self.kitchen_hand_now(id); return true };
        // The mug in an arc: a leg at a time from his hand, over its top, down onto the spot.
        let to = p.at;
        let hand = [pos[0], pos[1], pos[2] + HAND];
        let points: Vec<Vec3> = (0..=LEGS).map(|k| {
            let t = k as f32 / LEGS as f32;
            let lerp = |i: usize| hand[i] + (to[i] - hand[i]) * t;
            [lerp(0), lerp(1), lerp(2) + 4.0 * ARC_LIFT * t * (1.0 - t)]
        }).collect();
        let mut when = now;
        let mut legs = Vec::new();
        for k in 0..LEGS {
            if k > 0 { when += LEG_TICK; legs.push((when, points[k], points[k + 1])); }
            when += length(points[k], points[k + 1]) / MUG_SPEED;
        }
        p.hot_at = Some(when);
        p.next_tick = when;
        c.legs = legs;
        self.events.push(Event::Missile { caster: id, spell: MUG, target: Target::Point(points[1]) });
        self.kitchen_hand_now(id);
        true
    }

    /// Any blow let go counts the exchange, once a string: a hero left reeling by the first slap
    /// still sees the belch come.
    pub(crate) fn kitchen_blow_released(&mut self, id: Id, released: Option<(usize, usize)>) {
        if !self.is_kitchen(id) { return; }
        let Some((k, 0)) = released else { return };
        let name = self.content.mobs[self.mobs[&id].def].patterns[k].name.clone();
        let Some(c) = self.kitchen.cooks.get_mut(&id) else { return };
        if name == "steam" { return; }
        c.turn = c.turn.wrapping_add(1);
        c.since = c.since.saturating_add(1);
    }

    /// The end of a string: the breaths leave him open.
    pub(crate) fn kitchen_string_landed(&mut self, id: Id, released: Option<(usize, usize)>) {
        if !self.is_kitchen(id) || !self.kitchen.cooks.contains_key(&id) { return; }
        let Some((k, step)) = released else { return };
        let pattern = &self.content.mobs[self.mobs[&id].def].patterns[k];
        if step + 1 != pattern.steps.len() { return; }
        match pattern.name.as_str() {
            "steam" => self.kitchen_cough(id, STEAM_RECOVER),
            "belch" => self.kitchen_cough(id, COUGH),
            _ => {}
        }
    }

    /// Open: hunched in the murloc's own wounded recoil, held, a hit away.
    fn kitchen_cough(&mut self, id: Id, secs: f32) {
        let now = self.clock;
        self.halt(id);
        let m = self.mobs.get_mut(&id).unwrap();
        if let Some(foe) = m.state.foe() {
            m.pattern = None;
            m.ready_at = m.ready_at.max(now + secs);
            m.state = MobState::Recover { foe, until: now + secs };
        }
        self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Coughing { until: now + secs });
        self.events.push(Event::HoldPose { id, clip: OPEN_CLIP, secs, from: None });
        self.events.push(Event::Sound { id, sound: WOUND });
        self.kitchen_hand_now(id);
    }

    /// The scoop broken: he knocks the pot, stew splashes out round it, its steam drops, and he
    /// reels long enough to be beaten for it.
    pub(crate) fn kitchen_broken(&mut self, id: Id) {
        if !self.kitchen_scooping(id) { return; }
        let now = self.clock;
        self.yell(id, "MRGLGLGL!");
        self.events.push(Event::Sound { id, sound: SPLASH });
        if let Some(pot) = self.kitchen.cooks[&id].pot {
            if let Some(p) = self.kitchen.pots.get_mut(&pot) {
                p.down_until = Some(now + REEL);
                let at = p.at;
                self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: 2.4, arc: 360.0, width: 0.0,
                    secs: 1.2, colour: STEW_COLOUR.into() });
                self.kitchen_show_stew(pot);
            }
        }
        self.kitchen_cough(id, REEL);
    }

    /// A heavy blow on a scooping cook counts more: one is enough to knock the pot.
    pub(crate) fn kitchen_posture_share(&self, id: Id, by: Id, label: &str) -> f32 {
        if !self.kitchen_scooping(id) || label != "Melee" { return 1.0; }
        let heavy = self.players.get(&by).is_some_and(|p| matches!(&p.action, Action::Swing { strike, .. } if strike.heavy()));
        if heavy { SCOOP_HEAVY } else { 1.0 }
    }

    /// A cook keeps to a hero still in his fight even when he can find no way to them (a dock,
    /// behind the pot): he waits for them rather than going home (take 348), for `HOLD_ON`; then
    /// he gives up and goes home as any creature does.
    pub(crate) fn kitchen_holds_on(&self, id: Id, foe: Foe) -> bool {
        let (Some(m), Foe::Hero(h)) = (self.mobs.get(&id), foe) else { return false };
        self.kitchen.cooks.contains_key(&id) && self.in_fight_of(m, h)
            && m.lost_at.is_some_and(|t| self.clock - t < HOLD_ON)
    }

    /// While he waits with no way to his hero, nothing reaches him: blows and shots find nothing
    /// and he's seen to slip them, as a creature evades on its way home past its leash. Standing
    /// where he can't walk buys no free hits.
    pub(crate) fn kitchen_evading(&self, id: Id) -> bool {
        self.kitchen.cooks.contains_key(&id) && self.mobs.get(&id).is_some_and(|m| m.health > 0
            && matches!(m.state, MobState::Chase { .. }) && m.lost_at.is_some_and(|t| self.clock - t >= EVADE_AFTER))
    }

    /// What his hand should hold now, sent as it changes.
    fn kitchen_hand_now(&mut self, id: Id) {
        let Some(c) = self.kitchen.cooks.get(&id) else { return };
        let serving = self.kitchen_winding(id, "serve it hot");
        let want = match c.stage {
            Some(Stage::Pull { .. } | Stage::Seeking { .. } | Stage::Scooping { .. }) => Hand::Mug,
            Some(Stage::Rising(Rise::Gulping { .. })) => Hand::Mug,
            _ if self.clock < c.thrown_until => Hand::Empty,
            _ if serving => Hand::Mug,
            _ => Hand::Own,
        };
        if want != c.hand {
            self.kitchen.cooks.get_mut(&id).unwrap().hand = want;
            self.events.push(Event::MobGear { id });
        }
    }

    fn step_supper(&mut self, id: Id) {
        let now = self.clock;
        // The mug's later legs, each from where the last ended.
        let c = self.kitchen.cooks.get_mut(&id).unwrap();
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut c.legs).into_iter().partition(|l| l.0 <= now);
        c.legs = keep;
        for (_, from, to) in due {
            let anchor = self.alloc();
            self.events.push(Event::MissileFrom { anchor, caster: id, spell: MUG, from, target: Target::Point(to) });
        }
        let c = self.kitchen.cooks.get_mut(&id).unwrap();
        let expired: Vec<_> = c.puffs.iter().filter(|(_, end)| now >= *end).map(|(p, _)| *p).collect();
        c.puffs.retain(|(_, end)| now < *end);
        for p in expired { self.kitchen_drop_prop(p); }
        // The knocked pot's steam comes back.
        if let Some(pot) = self.kitchen.cooks[&id].pot {
            if self.kitchen.pots.get(&pot).is_some_and(|p| p.down_until.is_some_and(|t| now >= t)) {
                self.kitchen.pots.get_mut(&pot).unwrap().down_until = None;
                self.kitchen_show_stew(pot);
            }
        }
        // The belch's swelling pulse, drawn a step at a time; let go if it's broken off.
        if let Some((since, drawn)) = self.kitchen.cooks[&id].pulse {
            let scale = self.kitchen.cooks[&id].scale;
            if !self.kitchen_winding(id, "belch") {
                self.kitchen.cooks.get_mut(&id).unwrap().pulse = None;
                self.events.push(Event::Grow { id, scale });
            } else if now >= drawn + 0.15 {
                let windup = self.attack_now(id).windup.max(0.1);
                let f = 1.0 + PULSE * ((now - since) / windup).min(1.0);
                self.kitchen.cooks.get_mut(&id).unwrap().pulse = Some((since, now));
                self.events.push(Event::Grow { id, scale: scale * f });
            }
        }
        self.kitchen_hand_now(id);
        self.step_puddle(id);
    }

    /// The pour: warned where it lands through the wind-up and the flight, a splash, then a
    /// puddle with a hard edge at its true radius, burning until the last edge puff drops.
    fn step_puddle(&mut self, id: Id) {
        let now = self.clock;
        let Some(mut patch) = self.kitchen.cooks.get_mut(&id).unwrap().patch.take() else { return };
        let Some(start) = patch.hot_at else {
            if self.kitchen_winding(id, "serve it hot") {
                if now >= patch.next_ring {
                    patch.next_ring = now + REDRAW;
                    self.events.push(Event::GroundBurst { at: patch.at, yaw: 0.0, shape: "ring", reach: RADIUS, arc: 360.0,
                        width: 0.0, secs: REDRAW + 0.05, colour: HAZARD_COLOUR.into() });
                }
                self.kitchen.cooks.get_mut(&id).unwrap().patch = Some(patch);
            }
            return;
        };
        let end = start + HOT + FADE;
        if now >= end {
            for p in patch.props { self.kitchen_drop_prop(p); }
            return;
        }
        if now >= patch.next_ring {
            patch.next_ring = now + REDRAW;
            self.events.push(Event::GroundBurst { at: patch.at, yaw: 0.0, shape: "ring", reach: RADIUS, arc: 360.0,
                width: 0.0, secs: (REDRAW + 0.05).min(end - now), colour: HAZARD_COLOUR.into() });
        }
        if now >= start && !patch.landed {
            patch.landed = true;
            let at = patch.at;
            self.events.push(Event::Sound { id, sound: SPLASH });
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: RADIUS, arc: 360.0, width: 0.0,
                secs: SPLASH_SECS, colour: STEW_COLOUR.into() });
            for k in 0..EDGE_PUFFS {
                let a = k as f32 / EDGE_PUFFS as f32 * std::f32::consts::TAU;
                let p = self.ground_near(at[0] + a.cos() * (RADIUS - 0.2), at[1] + a.sin() * (RADIUS - 0.2), at[2]);
                patch.props.push(self.kitchen_prop(p, STEAM, EDGE_PUFF_SCALE));
            }
        }
        // Cooling: the edge puffs drop away one by one, the last as it's safe.
        if now >= start + HOT {
            let left = (EDGE_PUFFS as f32 * (end - now) / FADE).ceil().max(1.0) as usize;
            while patch.props.len() > left { let p = patch.props.pop().unwrap(); self.kitchen_drop_prop(p); }
        }
        if patch.landed && now >= patch.next_tick {
            let first = patch.next_tick == start;
            let damage = if first { patch.damage } else { (patch.damage / 5).max(1) };
            patch.next_tick = now + 1.0;
            let heroes: Vec<_> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, patch.at) <= RADIUS
                && (p.pos[2] - patch.at[2]).abs() < 2.0 && clear_line(self.land.as_ref(), patch.at, p.pos)).map(|(h, _)| *h).collect();
            for h in heroes {
                self.strike_fair(id, h, damage, patch.at, false, false, false, Blow::Light,
                    Some(combat::Area::Circle { at: patch.at, radius: RADIUS }), combat::Follow::None);
            }
        }
        self.kitchen.cooks.get_mut(&id).unwrap().patch = Some(patch);
    }

    /// Gorged rises where Cookie fell: to the pot, gulps it empty, and swells.
    pub(crate) fn kitchen_risen(&mut self, id: Id) {
        if !self.is_gorged(id) { return; }
        let pos = self.mobs[&id].pos;
        let pot = self.kitchen.pots.iter().filter(|(_, p)| dist2d(p.at, pos) < 15.0)
            .min_by(|a, b| dist2d(a.1.at, pos).total_cmp(&dist2d(b.1.at, pos))).map(|(k, _)| *k);
        let start = self.content.mob(COOKIE).map_or(1.0, |d| self.content.mobs[d].scale);
        let mut c = Cook::new(pot, start);
        c.began = true;
        c.pulled = true;
        let now = self.clock;
        c.stage = Some(Stage::Rising(match pot {
            Some(_) => Rise::Walking { until: now + GORGED_WALK },
            None => Rise::Swelling { since: now, drawn: now },
        }));
        self.kitchen.cooks.insert(id, c);
        self.halt(id);
        // He gets up out of a cloud of dust: a fall's and a stomp's, and its ring on the ground.
        self.events.push(Event::Visual { id, kit: combat::KIT_FALL_DUST, by: None });
        let at = self.mobs[&id].pos;
        self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: 3.0, arc: 360.0, width: 0.0,
            secs: 1.2, colour: STEW_COLOUR.into() });
        let m = self.mobs.get_mut(&id).unwrap();
        m.ready_at = m.ready_at.max(now + 30.0);
    }

    fn step_rising(&mut self, id: Id, rise: Rise, hero: Id) {
        let now = self.clock;
        let pot = self.kitchen.cooks[&id].pot;
        let set = |s: &mut Survival, r: Rise| s.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Rising(r));
        match rise {
            Rise::Walking { until } => {
                let Some(p) = pot.and_then(|p| self.kitchen.pots.get(&p)) else { return set(self, Rise::Swelling { since: now, drawn: now }) };
                let (spot, at) = (p.spot, p.at);
                let pos = self.mobs[&id].pos;
                if dist2d(pos, spot) <= 0.8 || now >= until {
                    self.halt(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(pos, at);
                    self.events.push(Event::MobStop { id, pos, facing: m.facing });
                    set(self, Rise::Gulping { until: now + GORGED_GULP, next_dip: now });
                } else if self.mobs[&id].path.is_empty() && !self.disabled(id) {
                    let speed = self.content.mobs[self.mobs[&id].def].walk_speed.max(1.0) * 1.25;
                    self.go(id, spot, speed);
                }
            }
            Rise::Gulping { until, next_dip } => {
                self.halt(id);
                if now >= next_dip {
                    self.events.push(Event::Clip { id, clip: DIP_CLIP, take: 0, from: None, secs: 0.83, at: 0.0, body: ClipBody::Full });
                    self.events.push(Event::Sound { id, sound: SLURP });
                    set(self, Rise::Gulping { until, next_dip: now + DIP * 0.9 });
                }
                if now >= until {
                    // The last of it: the pot's steam is gone.
                    if let Some(p) = pot {
                        if let Some(pt) = self.kitchen.pots.get_mut(&p) { pt.helpings = 0; pt.down_until = None; }
                        self.kitchen_show_stew(p);
                    }
                    self.events.push(Event::Sound { id, sound: GULP });
                    set(self, Rise::Swelling { since: now, drawn: now });
                }
            }
            Rise::Swelling { since, drawn } => {
                self.halt(id);
                let start = self.content.mob(COOKIE).map_or(1.0, |d| self.content.mobs[d].scale);
                let end = self.content.mobs[self.mobs[&id].def].scale;
                let t = ((now - since) / SWELL).min(1.0);
                // Up past his size by 80% of the way, then settling back onto it.
                let over = end * SWELL_OVER;
                let scale = if t < 0.8 { start + (end + over - start) * (t / 0.8) } else { end + over * (1.0 - t) / 0.2 };
                if now >= drawn + 0.1 || t >= 1.0 {
                    self.kitchen.cooks.get_mut(&id).unwrap().scale = scale;
                    self.events.push(Event::Grow { id, scale });
                    set(self, Rise::Swelling { since, drawn: now });
                }
                if t >= 1.0 {
                    self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
                    self.yell(id, "MORE STEW!");
                    self.events.push(Event::Emote { id, emote: ROAR_EMOTE });
                    self.events.push(Event::Sound { id, sound: AGGRO });
                    let to = self.players[&hero].pos;
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, to);
                    m.ready_at = now + SECOND_FORM_WAIT;
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                    set(self, Rise::Waiting { until: now + SECOND_FORM_WAIT });
                }
            }
            Rise::Waiting { until } => {
                self.halt(id);
                if now >= until { self.kitchen.cooks.get_mut(&id).unwrap().stage = None; }
            }
        }
    }

    /// A scoop only starts between exchanges, never over a turn already begun.
    pub(crate) fn step_kitchen(&mut self, id: Id) -> bool {
        let Some(c) = self.kitchen.cooks.get(&id) else { return false };
        let now = self.clock;
        let m = &self.mobs[&id];
        let Some(foe) = m.state.foe() else { return false };
        // The hero he's on while they're in his fight (the engine's own rule, as every creature's),
        // else the next in it. (A filter of its own, 6 yd of height from his post, sent Gorged home
        // mid-fight when a hero dodged up the cove's slope: take 341.)
        let target = match foe {
            Foe::Hero(h) if self.in_fight_of(m, h) => Some(h),
            _ => self.next_in_fight(id),
        };
        let Some(hero) = target else {
            self.remove_kitchen(id);
            self.halt(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.pattern = None;
            m.state = MobState::Return;
            return true;
        };
        let foe = if self.foe_pos(foe).is_some() { foe } else { Foe::Hero(hero) };
        let cookie = self.content.mobs[m.def].key == COOKIE;
        let stage = c.stage;
        let pot = c.pot.and_then(|p| self.kitchen.pots.get(&p)).map(|p| (p.at, p.spot, p.helpings));
        if stage.is_none() {
            let safe = m.pattern.is_none() && !matches!(m.state, MobState::Windup { .. })
                && !matches!(m.state, MobState::Recover { until, .. } if until > now);
            let c = self.kitchen.cooks.get_mut(&id).unwrap();
            c.began = true;
            if cookie && !c.pulled && safe {
                c.pulled = true;
                c.stage = Some(Stage::Pull { since: now, bangs: 0 });
                self.halt(id);
                if let Some((at, _, _)) = pot {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, at);
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                }
                self.kitchen_hand_now(id);
                return true;
            }
            let after = SCOOP_AFTER[(c.scoops as usize).min(SCOOP_AFTER.len() - 1)];
            let due = cookie && pot.is_some_and(|p| p.2 > 0) && c.since >= after;
            if !due || !safe || self.disabled(id) || self.mobs[&id].stagger_until > now { return false; }
            let c = self.kitchen.cooks.get_mut(&id).unwrap();
            c.since = 0;
            c.scoops = c.scoops.saturating_add(1);
            c.stage = Some(Stage::Seeking { until: now + 6.0 });
            self.kitchen_hand_now(id);
            return true;
        }
        match stage.unwrap() {
            Stage::Pull { since, bangs } => {
                self.halt(id);
                if bangs < 2 && now >= since + 0.25 + 0.6 * bangs as f32 {
                    self.events.push(Event::Clip { id, clip: BANG_CLIP, take: 0, from: None, secs: 0.55, at: 0.0, body: ClipBody::Full });
                    self.events.push(Event::Sound { id, sound: BANG });
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Pull { since, bangs: bangs + 1 });
                } else if now >= since + 1.6 {
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = None;
                    self.kitchen_hand_now(id);
                }
            }
            Stage::Seeking { until } => {
                let pos = self.mobs[&id].pos;
                let Some((at, spot, _)) = pot else {
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = None;
                    return false;
                };
                if dist2d(pos, spot) <= 0.8 {
                    self.halt(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(pos, at);
                    m.state = MobState::Recover { foe, until: now + SCOOP };
                    m.posture_guard = m.posture_guard.min(now);
                    self.events.push(Event::MobStop { id, pos, facing: m.facing });
                    self.yell(id, "Mmm... one taste.");
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Scooping { until: now + SCOOP, next_dip: now });
                } else if now >= until {
                    // A blocked pot gives up this scoop. No teleport and no heal from afar.
                    self.kitchen_cough(id, 1.5);
                } else if !self.disabled(id) && self.mobs[&id].stagger_until <= now && self.mobs[&id].path.is_empty()
                    && (self.land.walk_clear(pos, spot) || self.land.path(pos, spot).is_some_and(|p| !p.is_empty())) {
                    self.go(id, spot, 4.0);
                }
            }
            Stage::Scooping { until, next_dip } => {
                self.halt(id);
                let held = self.disabled(id) || self.mobs[&id].stagger_until > now;
                // The mug dipped again and again, a slurp each time.
                if now >= next_dip && !held && now + 0.5 < until {
                    self.events.push(Event::Clip { id, clip: DIP_CLIP, take: 0, from: None, secs: 0.83, at: 0.0, body: ClipBody::Full });
                    self.events.push(Event::Sound { id, sound: SLURP });
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = Some(Stage::Scooping { until, next_dip: now + DIP });
                }
                // Only the posture break spills it. Other crowd control delays the gulp.
                if now >= until && !held {
                    let c = self.kitchen.cooks.get_mut(&id).unwrap();
                    c.stage = None;
                    let heal = c.healed < HELPINGS;
                    c.healed = c.healed.saturating_add(1).min(HELPINGS);
                    let pot = c.pot;
                    let mut emptied = false;
                    if let Some(p) = pot.and_then(|p| self.kitchen.pots.get_mut(&p)) {
                        p.helpings = p.helpings.saturating_sub(1);
                        emptied = p.helpings == 0;
                    }
                    if let Some(p) = pot { self.kitchen_show_stew(p); }
                    if heal {
                        let m = self.mobs.get_mut(&id).unwrap();
                        m.health = (m.health + (m.max_health / 25).max(1)).min(m.max_health);
                        self.events.push(Event::Health { id, health: m.health, max: m.max_health });
                    }
                    self.events.push(Event::Sound { id, sound: GULP });
                    if emptied { self.yell(id, "Mrrgl... pot's empty."); }
                    let k = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "steam").unwrap();
                    self.begin_pattern(id, k, false);
                    self.windup(id, foe, self.players[&hero].pos);
                }
            }
            Stage::Coughing { until } => {
                self.halt(id);
                if now >= until.max(self.mobs[&id].stagger_until) {
                    self.kitchen.cooks.get_mut(&id).unwrap().stage = None;
                    self.kitchen_hand_now(id);
                }
            }
            Stage::Rising(rise) => self.step_rising(id, rise, hero),
        }
        true
    }
}

fn length(a: Vec3, b: Vec3) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

impl Cook {
    fn new(pot: Option<Id>, scale: f32) -> Self {
        Cook { pot, began: false, pulled: false, turn: 0, since: 0, scoops: 0, healed: 0, stage: None, patch: None,
            aim: None, legs: Vec::new(), puffs: Vec::new(), hand: Hand::Own, thrown_until: 0.0, scale, pulse: None }
    }
}
