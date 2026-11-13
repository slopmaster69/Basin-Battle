//! VanCleef's false opening (docs/vancleef-encounter.md). The rule a hero learns: hunched means
//! hit, blades up means wait. Every completed combination ends in a hunch (a kneel for the last
//! one), and only a hunch is open; the Invitation's raised blades never deal damage on their own,
//! but a direct frontal hit spends it once and the ordinary, committed melee engine answers.
//!
//! What the shared fixes (Corey's Nov 8 table) do engine-wide is done here for VanCleef and
//! Unbowed alone: the hunch's "open" look, debug damage never counting toward his posture, lines
//! at least 2 sec apart, the Blackguards leaving, Unbowed's rise before his first attack, and
//! his death stopping his animation.

use std::collections::VecDeque;

use super::mobs::{Foe, MobState, ROAR_EMOTE};
use super::*;

#[cfg(test)]
#[path = "vancleef_tests.rs"]
mod tests;

/// The raised blades, held about 2 sec.
const INVITATION: f32 = 2.0;
/// Waited out, he drops into this short hunch.
const WAITED_HUNCH: f32 = 1.2;
/// From the end of a hunch to the next combination; half that once he's alone.
const GAP: f32 = 1.0;
/// The scuff and glint, before the advance's first windup.
const ARRIVAL: f32 = 0.9;
/// The longest the backward walk out of the smoke may take before he stops where he is.
const RETREAT_LIMIT: f32 = 3.5;
const RETREAT_YARDS: f32 = 4.0;
/// He backs away at a walk, facing the hero.
const RETREAT_SPEED: f32 = 2.2;
/// Hits in a hunch count this much more toward breaking his posture.
const HUNCH_POSTURE: f32 = 2.0;
/// The riposte's lunge carries him this far past where the hero stood.
const RIPOSTE_PAST: f32 = 1.5;
const RIPOSTE_HALF_WIDTH: f32 = 1.0;
/// The lit deck in front of his cabin door, from his home in the doorway (the cabin opens
/// north onto the Juggernaut's main deck: home (-77, -820), the spot (-70, -819.5)).
const SPOT: [f32; 2] = [7.0, 0.5];
const ENTRANCE_SPEED: f32 = 2.6;
const ENTRANCE_LIMIT: f32 = 5.0;
/// The two Blackguards' posts on the main deck's west rail, and the way off the ship at its
/// bow, down onto the scaffold: (-64, -808), (-68, -807.5) and (-54, -793) on the native deck.
const POSTS: [[f32; 2]; 2] = [[13.0, 12.0], [9.0, 12.5]];
const EXIT: [f32; 2] = [23.0, 27.0];
/// Their longest acceptable way off, walked at their pace.
const EXIT_LIMIT: f32 = 45.0;
const LEAVE_SPEED: f32 = 3.0;
/// Turned to go and one shaking his head, this long before they walk.
const LEAVE_TURN: f32 = 1.5;
/// Whatever stops them, they're gone this long after they set off.
const LEAVE_LIMIT: f32 = 20.0;
/// Unbowed kneels this long, roars, then waits this long before his first attack.
const RISE_KNEEL: f32 = 1.0;
const RISE_WAIT: f32 = 2.5;
/// Off the deck: the floor under a hero this far below where they last stood firm (or water).
/// The sea is far below the Juggernaut's rail; a stair or a ramp never drops this much in a step.
const OVERBOARD_DROP: f32 = 4.0;
/// His lines never come closer together than this.
const LINE_GAP: f32 = 2.0;
/// "Go on, then." again only after this long.
const INVITE_LINE_AGAIN: f32 = 20.0;
const WATCHER: &str = "dm_blackguard_observer";
/// The open look: hunched low (StealthStand, bent well forward with the blades down, looping as
/// he breathes), or the kneel (EmoteKneel) for the last opening. Lower than any wind-up of his,
/// which all rise (Nov 9 filming: StandWound stood nearly upright, and read weaker than his old
/// crouching wind-ups, so "hunched means hit" was back to front on screen). Measured on the human
/// male's bones: the head at 0.77 of its standing height and the back 35 degrees forward, where
/// StandWound held 0.96 and 11.
pub(crate) const HUNCH_CLIP: u16 = 120;
pub(crate) const KNEEL_CLIP: u16 = 75;
/// A Blackguard shakes his head (`ONESHOT_NO`).
const EMOTE_NO: u32 = 274;
/// The steel scrape as the blades come up (metal unsheathed), and the scuff on the deck boards.
const SCRAPE: u32 = 700;
const SCUFF: u32 = 649;
/// The glint before the advance.
const GLINT: u32 = 379;
/// A smoke puff at his feet (Vanish's), seen through: no concealment, collision or damage.
const SMOKE: u32 = 1856;

#[derive(Default)]
pub(crate) struct VanCleef {
    duels: BTreeMap<Id, Duel>,
    observers: BTreeSet<Id>,
}

#[derive(Default)]
struct Duel {
    began: bool,
    entered: bool,
    invited: bool,
    invite_next: bool,
    invite_said: Option<f32>,
    final_done: bool,
    /// The 30% gate has passed: they go at the next quiet moment.
    departed: bool,
    leave: Leave,
    /// The Blackguards are gone and he's said so: his gaps between combinations halve.
    alone: bool,
    watchers: Vec<(Id, Vec3)>,
    /// Until when his hunch (or kneel) is open: hits count extra toward his posture.
    open_until: f32,
    /// The riposte's start and the length of its lunge, fixed at contact.
    riposte: Option<(Vec3, f32)>,
    lines: VecDeque<String>,
    said_at: Option<f32>,
    beat: Beat,
}

#[derive(Clone, Copy, Default, PartialEq, Debug)]
enum Leave {
    #[default]
    Watching,
    Turning { walk_at: f32 },
    Walking { since: f32 },
    Gone,
}

#[derive(Clone, Copy, Default, Debug)]
enum Beat {
    #[default]
    Fighting,
    Entrance { until: f32 },
    Invitation { until: f32 },
    Broken,
    Retreat { until: f32 },
    Arrival { until: f32 },
    Rising { roar_at: f32, until: f32 },
}

impl Survival {
    pub(crate) fn duel_observer(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == WATCHER)
    }

    pub(crate) fn is_vancleef(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| matches!(self.content.mobs[m.def].key.as_str(), "dm_vancleef" | "dm_vancleef_unbowed"))
    }

    fn first_form(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == "dm_vancleef")
    }

    fn duel_pattern_named(&self, id: Id, name: &str) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        self.content.mobs[m.def].patterns.iter().position(|p| p.name == name)
    }

    fn duel_pattern_is(&self, id: Id, k: Option<usize>, name: &str) -> bool {
        k.is_some() && k == self.duel_pattern_named(id, name)
    }

    /// The first exchange is honest. Invitations follow a completed string, not a timer
    /// or a health line. A lunge closes distance without spending that next Invitation.
    /// Unbowed has no Invitation at all.
    pub(crate) fn vancleef_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let near = dist <= d.attack.range + size - 0.3;
        let invite = self.vancleef.duels.get(&id).is_some_and(|d| d.invite_next) && self.duel_pattern_named(id, "invitation").is_some();
        let name = if near { if invite { "invitation" } else { "string" } } else { "lunge" };
        self.duel_pattern_named(id, name).filter(|k| near || dist <= d.patterns[*k].max_range)
    }

    /// His lines, at least `LINE_GAP` apart: one due sooner waits its turn.
    fn duel_say(&mut self, id: Id, text: &str) {
        let Some(duel) = self.vancleef.duels.get_mut(&id) else {
            self.yell(id, text);
            return;
        };
        duel.lines.push_back(text.to_string());
        self.duel_lines(id);
    }

    fn duel_lines(&mut self, id: Id) {
        let now = self.clock;
        let Some(duel) = self.vancleef.duels.get_mut(&id) else { return };
        if duel.said_at.is_some_and(|t| now - t < LINE_GAP) { return; }
        let Some(text) = duel.lines.pop_front() else { return };
        duel.said_at = Some(now);
        self.yell(id, text);
    }

    /// The open look for `secs`: a hunch, or the kneel. Hits meanwhile count extra.
    fn duel_open(&mut self, id: Id, secs: f32, kneel: bool) {
        if secs <= 0.0 { return; }
        let until = self.clock + secs;
        if let Some(duel) = self.vancleef.duels.get_mut(&id) { duel.open_until = until; }
        let clip = if kneel { KNEEL_CLIP } else { HUNCH_CLIP };
        // Held through the hits that punish it: no flinch stands him up.
        self.hold_open_pose(id, clip, secs);
    }

    /// Whether VanCleef is hunched (or kneeling) open now.
    pub(crate) fn vancleef_open(&self, id: Id) -> bool {
        self.vancleef.duels.get(&id).is_some_and(|d| d.open_until > self.clock)
    }

    /// (Tests) A whole posture's worth from `by` at once, as enough parries and hunch hits do.
    #[doc(hidden)]
    pub fn test_break_stance(&mut self, id: Id, by: Id) -> bool {
        let limit = self.posture_limit(id);
        self.build_posture(id, by, limit)
    }

    /// His posture's share of a hit: none from debug damage, more in his hunch. 1 for anyone else.
    pub(crate) fn vancleef_posture_share(&self, id: Id, label: &str) -> f32 {
        if !self.is_vancleef(id) { return 1.0; }
        if label == "debug" { return 0.0; }
        if self.vancleef_open(id) { HUNCH_POSTURE } else { 1.0 }
    }

    /// A completed combination (`pattern` its index): its recovery is a hunch, the last stand's
    /// a kneel, and the next comes only after his gap.
    pub(crate) fn vancleef_string_landed(&mut self, id: Id, pattern: Option<usize>) {
        if !self.is_vancleef(id) { return; }
        let now = self.clock;
        let Some(MobState::Recover { until, .. }) = self.mobs.get(&id).map(|m| m.state.clone()) else { return };
        if self.duel_pattern_is(id, pattern, "string") {
            self.vancleef.duels.entry(id).or_default().invite_next = true;
        }
        let kneel = self.duel_pattern_is(id, pattern, "last stand");
        // The last stand's kneel faces the hero it was fought against, so it's seen from the
        // front, not over his back (Nov 9 filming).
        if kneel { self.duel_face_foe(id); }
        if self.duel_pattern_is(id, pattern, "riposte") {
            if let Some(duel) = self.vancleef.duels.get_mut(&id) { duel.riposte = None; }
        }
        self.duel_open(id, until - now, kneel);
        let gap = self.duel_gap(id);
        let m = self.mobs.get_mut(&id).unwrap();
        m.ready_at = m.ready_at.max(until + gap);
    }

    /// He turns to face the hero he's fighting, where he stands.
    fn duel_face_foe(&mut self, id: Id) {
        let Some(m) = self.mobs.get(&id) else { return };
        let Some(at) = m.state.target().and_then(|h| self.players.get(&h)).map(|p| p.pos) else { return };
        let pos = m.pos;
        if dist2d(pos, at) < 0.1 { return; }
        let face = facing(pos, at);
        self.mobs.get_mut(&id).unwrap().facing = face;
        self.events.push(Event::MobStop { id, pos, facing: face });
    }

    fn duel_gap(&self, id: Id) -> f32 {
        let alone = !self.first_form(id) || self.vancleef.duels.get(&id).is_some_and(|d| d.alone);
        if alone { GAP * 0.5 } else { GAP }
    }

    /// Intercept only the harmless stance. The actual riposte uses `windup` normally.
    pub(crate) fn vancleef_invitation(&mut self, id: Id, foe: Foe, to: Vec3) -> bool {
        if !self.is_vancleef(id) || self.mobs[&id].pattern.map(|(k, _)| k) != self.duel_pattern_named(id, "invitation") { return false; }
        self.halt(id);
        let now = self.clock;
        let until = now + INVITATION;
        let duel = self.vancleef.duels.entry(id).or_default();
        duel.invited = true;
        duel.invite_next = false;
        duel.beat = Beat::Invitation { until };
        let say = duel.invite_said.is_none_or(|t| now - t >= INVITE_LINE_AGAIN);
        if say { duel.invite_said = Some(now); }
        let m = self.mobs.get_mut(&id).unwrap();
        // Keep the harmless pattern while held, including in a controlled lab scene.
        // Only this module advances it; waiting never releases its dummy step.
        m.hold = 0.0;
        m.guarding = true;
        m.facing = facing(m.pos, to);
        m.state = MobState::Recover { foe, until };
        self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
        // Blades up, feet planted: the native held guard pose, with both swords still equipped,
        // and the steel's scrape. It has no block or damage immunity.
        self.events.push(Event::MobGuard { id, up: true });
        self.events.push(Event::Sound { id, sound: SCRAPE });
        if say { self.duel_say(id, "Go on, then."); }
        true
    }

    fn lower_invitation(&mut self, id: Id) {
        if let Some(m) = self.mobs.get_mut(&id) { m.guarding = false; }
        self.events.push(Event::MobGuard { id, up: false });
    }

    /// Called after a direct melee/projectile contact and all its crowd control. Periodic
    /// damage, splashes and pets do not count. The hit still hurts; death, posture breaks
    /// and control take precedence. Two heroes hitting together cannot restart the tell.
    pub(crate) fn vancleef_hit(&mut self, id: Id, hero: Id, from: Vec3) {
        let Some(Beat::Invitation { until }) = self.vancleef.duels.get(&id).map(|d| d.beat) else { return };
        let Some(m) = self.mobs.get(&id) else { return };
        if self.clock >= until || m.health == 0 || !self.players.get(&hero).is_some_and(|p| p.health > 0)
            || self.disabled(id) || m.stagger_until > self.clock
            || !matches!(m.state, MobState::Recover { .. })
            || angle_between(m.facing, facing(m.pos, from)) > 70.0
            || (m.pos[2] - from[2]).abs() > 6.0 { return; }
        self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
        self.lower_invitation(id);
        let Some(k) = self.duel_pattern_named(id, "riposte") else { return };
        self.begin_pattern(id, k, false);
        // Aim at where the attacker stands now, once, and carry the lunge past it. Content gives
        // zero turning and locks for the entire windup, so moving after provoking it always works.
        let to = self.players[&hero].pos;
        let at = self.mobs[&id].pos;
        let len = (dist2d(at, to) + RIPOSTE_PAST).clamp(2.0, 7.0);
        self.vancleef.duels.get_mut(&id).unwrap().riposte = Some((at, len));
        self.windup(id, Foe::Hero(hero), to);
        // The riposte's own tell, used for nothing else: his weapon glows red through the windup.
        if let MobState::Windup { .. } = self.mobs[&id].state {
            let secs = self.attack_now(id).windup;
            self.events.push(Event::Glow { id, secs });
        }
    }

    /// The riposte's lunge: from where he stood when provoked, along his committed facing, as
    /// far past the hero's spot as the deck allows. None for any other blow.
    pub(crate) fn vancleef_lunge(&self, id: Id) -> Option<f32> {
        let m = self.mobs.get(&id)?;
        if !self.duel_pattern_is(id, m.pattern.map(|(k, _)| k), "riposte") { return None; }
        let (start, len) = self.vancleef.duels.get(&id)?.riposte?;
        let left = len - dist2d(start, m.pos);
        let f = m.facing;
        let mut reach = left.max(0.0);
        while reach > 0.5 {
            let to = self.ground_near(m.pos[0] + f.cos() * reach, m.pos[1] + f.sin() * reach, m.pos[2]);
            if self.land.walk_clear(m.pos, to) { return Some(reach); }
            reach -= 0.5;
        }
        Some(0.0)
    }

    /// The riposte lands along the whole line of its lunge: a hero who stood still is run
    /// through (it hurts, but never kills), one who stepped aside is behind him.
    pub(crate) fn vancleef_release(&mut self, id: Id) -> bool {
        let Some(m) = self.mobs.get(&id) else { return false };
        if !self.duel_pattern_is(id, m.pattern.map(|(k, _)| k), "riposte") { return false; }
        let Some((start, len)) = self.vancleef.duels.get(&id).and_then(|d| d.riposte) else { return false };
        let a = self.attack_now(id);
        let (f, pos) = (m.facing, m.pos);
        let reach = dist2d(start, pos).max(len.min(dist2d(start, pos) + 0.5)) + a.range.min(1.5);
        let share = self.foe_blow_share(id).max(0.01);
        let caught: Vec<(Id, u32)> = self.players.iter()
            .filter(|(_, p)| p.health > 0 && combat::in_strip(start, f, reach, RIPOSTE_HALF_WIDTH, p.pos))
            .map(|(h, p)| (*h, ((p.health - 1) as f32 / share).floor() as u32))
            .collect();
        let mut any = false;
        for (h, cap) in caught {
            let damage = a.damage.min(cap);
            if damage == 0 { continue; }
            any = true;
            let area = Some(combat::Area::Line { pos: start, facing: f, len: reach, half: RIPOSTE_HALF_WIDTH });
            self.strike_fair_as(id, h, damage, start, a.blockable, a.heavy, a.blockable, Blow::Light, area, combat::Follow::None, super::abilities::HitKind::Melee);
        }
        if !any {
            if let Some(Foe::Hero(h)) = self.mobs[&id].state.foe() {
                self.events.push(Event::Swing { attacker: id, victim: Some(h), damage: 0, outcome: Outcome::Miss, anim: false, style: Blow::Light });
            }
        }
        true
    }

    pub(crate) fn vancleef_broken(&mut self, id: Id) {
        if !self.is_vancleef(id) { return; }
        self.lower_invitation(id);
        let duel = self.vancleef.duels.entry(id).or_default();
        duel.beat = Beat::Broken;
        duel.open_until = 0.0;
        duel.riposte = None;
        // The reel replaces the hunch; hits show again.
        if let Some(m) = self.mobs.get_mut(&id) { m.posed_until = 0.0; }
        self.duel_say(id, "You've learned something.");
        self.events.push(Event::Sound { id, sound: 5782 });
    }

    /// A short same-floor path only. Reject water, gaps and long detours: failure means
    /// he stays put and gives the same arrival cue, never a teleport or hidden attack.
    fn duel_floor(&self, from: Vec3, to: Vec3) -> Option<Vec3> {
        let to = self.ground_near(to[0], to[1], from[2]);
        if (to[2] - from[2]).abs() > 1.5 || self.land.blocked_near(to[0], to[1], to[2]) || self.land.water_near(to[0], to[1], to[2]) { return None; }
        let path = if self.land.walk_clear(from, to) { vec![to] } else { self.land.path(from, to)? };
        if path.is_empty() || dist2d(*path.last()?, to) > 0.5 { return None; }
        let mut last = from;
        let mut distance = 0.0;
        for p in path {
            if (p[2] - from[2]).abs() > 1.5 || !self.land.walk_clear(last, p) { return None; }
            distance += dist2d(last, p);
            last = p;
        }
        (distance <= dist2d(from, to) * 1.6 + 1.0).then_some(to)
    }

    /// A Blackguard's way off the ship: a searched walk, each stride a walker's, no longer than
    /// `EXIT_LIMIT`. None leaves that Blackguard out.
    fn duel_exit(&self, from: Vec3, to: Vec3) -> Option<Vec3> {
        let to = self.land.footing(self.ground_near(to[0], to[1], from[2])).unwrap_or_else(|| self.ground_near(to[0], to[1], from[2]));
        if self.land.water_near(to[0], to[1], to[2]) || (to[2] - from[2]).abs() > 4.0 { return None; }
        let path = if self.land.walk_clear(from, to) { vec![to] } else { self.land.path(from, to)? };
        if path.is_empty() || dist2d(*path.last()?, to) > 0.5 { return None; }
        let mut last = from;
        let mut distance = 0.0;
        for p in path {
            if !self.land.walk_clear(last, p) { return None; }
            distance += dist2d(last, p);
            last = p;
        }
        (distance <= EXIT_LIMIT).then_some(to)
    }

    /// Out on the open deck, not under his cabin's roof (or any other).
    fn duel_open_sky(&self, at: Vec3) -> bool {
        !self.land.slab_between(at[0], at[1], at[2] + 2.5, at[2] + 40.0)
    }

    /// The ship's side in his duel: a hero carried over it (a roll from his follow-up; in the
    /// Nov 10 filming three takes in four ended in the sea, and he reset) is set back on the deck
    /// where they last stood firm, and the fight goes on. Only for a hero in his fight (or
    /// Unbowed's) on his deck, and only off it, under the open sky: over water, or over a floor
    /// `OVERBOARD_DROP` below.
    pub(crate) fn hold_on_deck(&mut self, id: Id, to: Vec3) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        if p.health == 0 || to.iter().any(|v| !v.is_finite()) { return false; }
        let (safe, facing) = (p.last_safe, p.facing);
        let dueling = self.vancleef.duels.keys().any(|v| self.mobs.get(v).is_some_and(|m| {
            m.health > 0 && m.state.foe().is_some() && (safe[2] - m.pos[2]).abs() < 3.0 && self.in_fight_of(m, id)
        }));
        if !dueling { return false; }
        let deck = safe[2] + 1.0;
        // (Under a roof, his cabin's, a gap in the land's floors is the map's, not the sea.)
        let over_side = (self.land.water_near(to[0], to[1], deck)
            || self.land.height_near(to[0], to[1], deck) < safe[2] - OVERBOARD_DROP)
            && self.duel_open_sky([to[0], to[1], safe[2]]);
        if !over_side { return false; }
        self.put_at(id, [safe[0], safe[1], safe[2] + ARRIVE_LIFT], facing);
        true
    }

    /// Where he duels: the lit deck in front of his cabin (Unbowed: where he rose).
    fn duel_spot(&self, id: Id) -> Vec3 {
        let m = &self.mobs[&id];
        if !self.first_form(id) { return m.pos; }
        let home = m.home;
        self.duel_floor(home, [home[0] + SPOT[0], home[1] + SPOT[1], home[2]]).unwrap_or(home)
    }

    fn place_duel(&mut self, id: Id) {
        let m = &self.mobs[&id];
        let home = m.home;
        let mut duel = Duel::default();
        // Unbowed inherits the empty deck. These are observers, never a combat wave.
        if self.first_form(id) {
            let spot = self.duel_spot(id);
            if let Some(def) = self.content.mob(WATCHER) {
                for post in POSTS {
                    let Some(at) = self.duel_floor(spot, [home[0] + post[0], home[1] + post[1], home[2]]) else { continue };
                    let Some(exit) = self.duel_exit(at, [home[0] + EXIT[0], home[1] + EXIT[1], home[2]]) else { continue };
                    let watcher = self.spawn_mob(def, at, None, false);
                    let m = self.mobs.get_mut(&watcher).unwrap();
                    m.state = MobState::Idle { wander_at: f32::MAX };
                    m.facing = facing(at, spot);
                    self.vancleef.observers.insert(watcher);
                    duel.watchers.push((watcher, exit));
                }
            }
        }
        self.vancleef.duels.insert(id, duel);
    }

    fn remove_watcher(&mut self, watcher: Id) {
        self.vancleef.observers.remove(&watcher);
        if self.mobs.remove(&watcher).is_some() { self.events.push(Event::MobGone { id: watcher }); }
    }

    pub(crate) fn remove_duel(&mut self, id: Id) {
        let Some(duel) = self.vancleef.duels.remove(&id) else { return };
        self.lower_invitation(id);
        // Whatever pose he holds ends now: a death plays at once.
        self.hold_open_pose(id, HUNCH_CLIP, 0.0);
        for (watcher, _) in duel.watchers {
            self.remove_watcher(watcher);
        }
    }

    pub(crate) fn reset_duels(&mut self) {
        let ids: Vec<_> = self.vancleef.duels.keys().copied().collect();
        for id in ids {
            self.remove_duel(id);
            if let Some(MobState::Windup { mark: Some(mark), .. }) = self.mobs.get(&id).map(|m| m.state.clone()) { self.unmark(mark); }
            self.halt(id);
            self.clear_cc(id);
            if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                m.pattern = None;
                m.special = false;
                m.hold = 0.0;
                m.string_len = 0;
                m.held_step = None;
                m.last_pattern = None;
                m.voiced = false;
                m.riposte = false;
                m.ready_at = self.clock + 0.6;
                m.stagger_until = 0.0;
                m.posture = 0;
                m.posture_guard = 0.0;
                m.state = MobState::Idle { wander_at: self.clock + 3.0 };
            }
        }
    }

    pub(crate) fn clean_duels(&mut self) {
        let stale: Vec<_> = self.vancleef.duels.iter().filter(|(id, d)| !self.mobs.get(id).is_some_and(|m| m.health > 0
            && !matches!(m.state, MobState::Return) && (!d.began || m.state.foe().is_some()))).map(|(id, _)| *id).collect();
        for id in stale { self.remove_duel(id); }
        let new: Vec<_> = self.mobs.iter().filter(|(id, m)| self.is_vancleef(**id) && m.health > 0 && !m.hidden
            && m.rise_at.is_none() && !matches!(m.state, MobState::Return) && !self.vancleef.duels.contains_key(id)
            && self.players.values().any(|p| dist2d(p.pos, m.home) < 40.0 && (p.pos[2] - m.home[2]).abs() < 6.0)).map(|(id, _)| *id).collect();
        for id in new { self.place_duel(id); }
    }

    /// Unbowed has risen (`Survival::rise`): he kneels where the body lay, roars, and waits before
    /// his first attack, so he never swings while still appearing on top of a hero.
    pub(crate) fn vancleef_risen(&mut self, id: Id) {
        if !self.is_vancleef(id) || self.first_form(id) { return; }
        if !self.vancleef.duels.contains_key(&id) { self.place_duel(id); }
        let now = self.clock;
        let duel = self.vancleef.duels.get_mut(&id).unwrap();
        duel.beat = Beat::Rising { roar_at: now + RISE_KNEEL, until: now + RISE_KNEEL + RISE_WAIT };
        duel.entered = true;
        self.halt(id);
        // Held as an open pose: a hit while he kneels mustn't flinch him upright (a flinch replaces
        // any held pose on the clients).
        self.hold_open_pose(id, KNEEL_CLIP, RISE_KNEEL);
    }

    fn duel_arrival(&mut self, id: Id) {
        self.halt(id);
        if let Some(hero) = self.duel_hero(id) {
            let to = self.players[&hero].pos;
            let m = self.mobs.get_mut(&id).unwrap();
            m.facing = facing(m.pos, to);
            self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
        }
        self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Arrival { until: self.clock + ARRIVAL };
        // A scuff on the boards and a glint: the advance is coming.
        self.events.push(Event::Sound { id, sound: SCUFF });
        self.events.push(Event::Visual { id, kit: GLINT, by: None });
    }

    /// Bosses see through a veil (`unseen_by`). The generic replacement-target query
    /// excludes veiled heroes, so using it alone here would turn solo Vanish into a wipe.
    fn duel_hero(&self, id: Id) -> Option<Id> {
        self.next_in_fight(id).or_else(|| {
            let m = self.mobs.get(&id)?;
            let leash = self.leash_of(m.def);
            self.players.iter().filter(|(_, p)| p.health > 0 && (p.pos[2] - m.pos[2]).abs() < 10.0
                && dist2d(p.pos, m.pos) <= self.content.mobs[m.def].leash
                && !self.leashed(m.home, m.pos, p.pos, leash))
                .min_by(|a, b| dist2d(a.1.pos, m.pos).total_cmp(&dist2d(b.1.pos, m.pos)))
                .map(|(hero, _)| *hero)
        })
    }

    /// A quiet moment: no blow in hand, between combinations or hunched.
    fn duel_quiet(&self, id: Id) -> bool {
        let m = &self.mobs[&id];
        matches!(self.vancleef.duels[&id].beat, Beat::Fighting) && m.pattern.is_none()
            && matches!(m.state, MobState::Recover { .. } | MobState::Chase { .. }) && m.stagger_until <= self.clock
    }

    /// The Blackguards: at 30% they wait for a quiet moment, turn, one shakes his head, and both
    /// walk off the ship. Once they're gone he says so, and he's alone.
    fn step_departure(&mut self, id: Id) {
        let now = self.clock;
        let (health, max) = (self.mobs[&id].health, self.mobs[&id].max_health);
        let duel = &self.vancleef.duels[&id];
        if !duel.departed && health as f32 <= max as f32 * 0.30 {
            let duel = self.vancleef.duels.get_mut(&id).unwrap();
            duel.departed = true;
            if duel.watchers.is_empty() { duel.leave = Leave::Gone; }
        }
        let duel = &self.vancleef.duels[&id];
        match duel.leave {
            Leave::Watching if duel.departed && self.duel_quiet(id) => {
                let watchers = duel.watchers.clone();
                for (k, (w, exit)) in watchers.iter().enumerate() {
                    let Some(m) = self.mobs.get_mut(w) else { continue };
                    m.facing = facing(m.pos, *exit);
                    let (pos, f) = (m.pos, m.facing);
                    self.events.push(Event::MobStop { id: *w, pos, facing: f });
                    if k == 0 { self.events.push(Event::Emote { id: *w, emote: EMOTE_NO }); }
                }
                self.vancleef.duels.get_mut(&id).unwrap().leave = Leave::Turning { walk_at: now + LEAVE_TURN };
            }
            Leave::Turning { walk_at } if now >= walk_at => {
                for (w, exit) in duel.watchers.clone() { self.go(w, exit, LEAVE_SPEED); }
                self.vancleef.duels.get_mut(&id).unwrap().leave = Leave::Walking { since: now };
            }
            Leave::Walking { since } => {
                let gone: Vec<Id> = duel.watchers.iter().filter(|(w, exit)| now >= since + LEAVE_LIMIT
                    || self.mobs.get(w).is_none_or(|m| m.path.is_empty() && dist2d(m.pos, *exit) < 0.6)).map(|(w, _)| *w).collect();
                for w in &gone { self.remove_watcher(*w); }
                let duel = self.vancleef.duels.get_mut(&id).unwrap();
                duel.watchers.retain(|(w, _)| !gone.contains(w));
                if duel.watchers.is_empty() {
                    duel.leave = Leave::Gone;
                    duel.alone = true;
                    self.duel_say(id, "Cowards! All of you!");
                }
            }
            _ => {}
        }
    }

    /// Runs after walking, before ordinary thought and the disabled check. This lets a
    /// rooted retreat time out safely; a stunned arrival still waits for control to end.
    pub(crate) fn step_vancleef(&mut self, id: Id) -> bool {
        if self.vancleef.observers.contains(&id) { return true; }
        if !self.is_vancleef(id) || !self.vancleef.duels.contains_key(&id) { return false; }
        let now = self.clock;
        self.duel_lines(id);
        let Some(foe) = self.mobs[&id].state.foe() else { return false };
        self.vancleef.duels.get_mut(&id).unwrap().began = true;
        // Wipes and leash exits clear poses and observers even during the stance/run.
        let Some(hero) = self.duel_hero(id) else {
            self.remove_duel(id);
            self.halt(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.state = MobState::Return;
            m.pattern = None;
            return true;
        };
        // The pull: he steps out of the cabin onto the lit deck before his first blow.
        if !self.vancleef.duels[&id].entered {
            self.vancleef.duels.get_mut(&id).unwrap().entered = true;
            let spot = self.duel_spot(id);
            if self.first_form(id) && dist2d(spot, self.mobs[&id].pos) > 0.5 {
                self.go(id, spot, ENTRANCE_SPEED);
                self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Entrance { until: now + ENTRANCE_LIMIT };
            }
        }
        self.step_departure(id);
        match self.vancleef.duels[&id].beat {
            Beat::Entrance { until } => {
                if now >= until || self.mobs[&id].path.is_empty() {
                    self.halt(id);
                    let to = self.players[&hero].pos;
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, to);
                    m.state = MobState::Chase { foe };
                    m.ready_at = m.ready_at.max(now + 0.4);
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                }
                true
            }
            Beat::Rising { roar_at, until } => {
                if now >= roar_at {
                    // The roar once, as he stands from the kneel.
                    let duel = self.vancleef.duels.get_mut(&id).unwrap();
                    duel.beat = Beat::Rising { roar_at: f32::MAX, until };
                    self.events.push(Event::Emote { id, emote: ROAR_EMOTE });
                    let sound = self.content.mobs[self.mobs[&id].def].aggro_sound;
                    if sound != 0 { self.events.push(Event::Sound { id, sound }); }
                }
                let to = self.players[&hero].pos;
                self.hold_facing(id, to);
                if now >= until {
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.state = MobState::Chase { foe };
                    m.ready_at = now;
                }
                true
            }
            Beat::Invitation { until } => {
                if self.disabled(id) || self.mobs[&id].stagger_until > now {
                    self.lower_invitation(id);
                    self.mobs.get_mut(&id).unwrap().pattern = None;
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                } else if now >= until {
                    // Waited out: blades down into a short hunch. Punish it.
                    self.lower_invitation(id);
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                    let gap = self.duel_gap(id);
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.pattern = None;
                    m.state = MobState::Recover { foe, until: now + WAITED_HUNCH };
                    m.ready_at = now + WAITED_HUNCH + gap;
                    self.duel_open(id, WAITED_HUNCH, false);
                }
                true
            }
            Beat::Broken => {
                if self.disabled(id) || self.mobs[&id].stagger_until > now { return true; }
                let (at, spot) = (self.mobs[&id].pos, self.duel_spot(id));
                let away = facing(self.players[&hero].pos, at);
                // Straight back, facing the hero, staying on the lit deck.
                let to = [0.0f32, 0.6, -0.6, 1.2, -1.2].into_iter().find_map(|turn| {
                    let to = [at[0] + (away + turn).cos() * RETREAT_YARDS, at[1] + (away + turn).sin() * RETREAT_YARDS, at[2]];
                    self.duel_floor(at, to).filter(|p| self.land.walk_clear(at, *p) && self.duel_open_sky(*p)
                        && (dist2d(*p, spot) < 9.0 || dist2d(*p, spot) <= dist2d(at, spot)))
                });
                // Smoke at his feet, then he walks backward out of it.
                self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                if let Some(to) = to {
                    self.sidle_to(id, to, RETREAT_SPEED, hero);
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Retreat { until: now + RETREAT_LIMIT };
                } else { self.duel_arrival(id); }
                true
            }
            Beat::Retreat { until } => {
                if now >= until || self.mobs[&id].path.is_empty() { self.duel_arrival(id); }
                true
            }
            Beat::Arrival { until } => {
                if now >= until && !self.disabled(id) && self.mobs[&id].stagger_until <= now {
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                    if let Some(k) = self.duel_pattern_named(id, "advance") {
                        self.begin_pattern(id, k, false);
                        self.windup(id, Foe::Hero(hero), self.players[&hero].pos);
                    }
                }
                true
            }
            Beat::Fighting => {
                let m = &self.mobs[&id];
                // His gap between combinations: planted, squared up to the hero, not circling
                // off the lit deck. (Out of reach, he comes on as anyone would.)
                let to = self.players[&hero].pos;
                let reach = self.content.mobs[m.def].attack.range + 1.2;
                if matches!(m.state, MobState::Chase { .. }) && now < m.ready_at && dist2d(m.pos, to) <= reach
                    && m.health as f32 > m.max_health as f32 * 0.12 {
                    self.hold_facing(id, to);
                    return true;
                }
                let boundary = matches!(m.state, MobState::Chase { .. }) || matches!(m.state, MobState::Recover { until, .. } if now >= until && m.pattern.is_none());
                if boundary && !self.disabled(id) && m.stagger_until <= now && !self.vancleef.duels[&id].final_done
                    && m.health as f32 <= m.max_health as f32 * 0.12 {
                    self.vancleef.duels.get_mut(&id).unwrap().final_done = true;
                    self.duel_say(id, "This ends here.");
                    if let Some(k) = self.duel_pattern_named(id, "last stand") {
                        self.begin_pattern(id, k, false);
                        self.windup(id, Foe::Hero(hero), self.players[&hero].pos);
                        return true;
                    }
                }
                false
            }
        }
    }
}
