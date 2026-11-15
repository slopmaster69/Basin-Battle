//! Captain Greenskin: break the firing line (docs/greenskin-encounter.md). Two deck gunners stand
//! at their posts either side of the quarterdeck before the pull, and are pulled with him. He
//! orders them to fire, and they take turns, one shot about every 3.6 sec, never on the same beat:
//! a load with a click, the gun raised to the shoulder (where its aim locks), and a shot along a
//! line the hero steps out of. He never idles: he fights all the while, only never starting a
//! blow that would land within 0.8 sec of a shot. The last gunner left shoots on every beat, harder.
//! After every third blow he steps back and throws a harpoon: a fair hit hauls the hero in to his
//! clearing cut, which lands once they are on their feet. With both gunners down he draws his own
//! gun, keeps it out a while, and attacks without pause.

use super::mobs::{Foe, MobState};
use super::*;
use crate::land::clear_line;

#[cfg(test)]
#[path = "firing_line_tests.rs"]
mod tests;

/// A gunner's turn comes this often; the two take turns.
const SHOT_GAP: f32 = 3.6;
/// The last gunner left shoots on every beat (the pair together shot one a beat; one alone used
/// to shoot one every two) and hits this much harder: the fight never gets gentler as the crew
/// falls.
pub(crate) const LAST_MIGHT: f32 = 1.25;
/// The captain never starts a blow that lands this near a gunner's shot.
const CLASH: f32 = 0.8;
/// After the pull he advances; this long after it he points his cutlass at his first gunner,
/// holds it a moment, and the first load starts a second after.
const ORDER_AFTER: f32 = 1.0;
const ORDER_POINT: f32 = 0.8;
const FIRST_LOAD: f32 = 1.0;
/// The load (LoadRifle and its click), before the gun reaches the shoulder and the aim locks.
const LOAD: f32 = 0.6;
/// The captain's draw (GoblinMale's Sheath: the gun is in his hand at 0.3 sec), and how long it
/// stays out after a shot before he puts it away the same way.
const DRAW: f32 = 0.5;
const DRAW_SWAP: f32 = 0.3;
const GUN_OUT: f32 = 6.0;
/// Alone, a shot at most this often: the gun is for heroes who kite him.
const GUN_EVERY: f32 = 10.0;
/// The harpoon comes on a count: after every third blow of his cutlass (a thrust or a cut, not
/// a shot) he steps back from his hero to this far, the bulwark or a wall stopping him sooner,
/// and throws. A hero fighting him always sees it, however near they stay.
const HARPOON_EVERY: u32 = 3;
const HARPOON_FAR: f32 = 8.0;
/// He aims to stand this much farther than `HARPOON_FAR` when his step back ends; at this pace
/// (4.5 yd/s, 2.25 times his walk, under a hero's 7 yd/s run) and for at most this long.
const HARPOON_OVER: f32 = 1.0;
const HARPOON_PACE: f32 = 4.5;
const HARPOON_STEP: f32 = 2.5;
/// A hauled hero is carried in the air for about 1.7 sec, helpless. His clearing cut lands this
/// long after they are on their feet again: a hero's 0.3 sec reaction and 0.6 sec to guard or step.
const HAUL_FREE: f32 = 0.9;
/// A fair hit hauls them in to this far from him.
const PULL_TO: f32 = 3.0;
/// The posts, from his home (-62, -815, 41.5): (-58, -806.5) and (-58, -824.5) on the quarterdeck,
/// 9.4 and 10.3 yd out on either side of him, on dry deck with a walk to him (dmprobe, Nov 9).
const POSTS: [[f32; 2]; 2] = [[4.0, 8.5], [4.0, -9.5]];
/// The crew is at its posts once a hero comes this near his home.
const POST_NEAR: f32 = 40.0;
/// A short native blunderbuss (1.12 has no one-handed pistol model), and a harpoon.
const GUN: u32 = 21071;
const SPEAR: u32 = 5949;
/// GoblinMale's and HumanMale's clips: Sheath (the draw), LoadRifle, ReadyRifle (aimed),
/// AttackRifle, ReadyThrown (the harpoon held back) and AttackThrown.
const CLIP_SHEATH: u16 = 89;
const CLIP_LOAD: u16 = 106;
const CLIP_AIM: u16 = 48;
const CLIP_FIRE: u16 = 49;
const CLIP_THROW_HOLD: u16 = 108;
const CLIP_THROW: u16 = 107;
/// GunPrecastOneShot (the click of the load), and PutDownLargeMetal (a dropped gun).
const CLICK: u32 = 1147;
const CLATTER: u32 = 1207;
/// EmotePoint: his cutlass at the gunner whose turn comes first.
const POINT: u32 = 25;
/// The shot in flight: Fire Shot's bolt, flown at the shot's own speed (`[[missile_look]]`).
const TRACER: u32 = 6980;
/// Where a shot stops, hit or miss: Irritating Spores' dust burst (Rocky Ground Impact's debris
/// filmed as pink boulders that lingered, take 336).
const SPLINTERS: u32 = 22786;

#[derive(Default)]
pub(crate) struct FiringLine {
    crews: BTreeMap<Id, Crew>,
    /// Each gunner's captain.
    owners: BTreeMap<Id, Id>,
    /// Shots and throws being readied, by whoever readies them.
    aims: BTreeMap<Id, Aim>,
    /// The captain's own gun.
    guns: BTreeMap<Id, Gun>,
    /// The harpoon in his hand, through its held throw.
    spears: BTreeSet<Id>,
}

struct Crew {
    gunners: Vec<Id>,
    stage: Stage,
    /// He holds his point at the first gunner until then.
    hold_until: f32,
    /// His cutlass blows since the last harpoon (a thrust or a cut each).
    blows: u32,
    /// Stepping back for the harpoon: until when, at the latest.
    stepping_until: Option<f32>,
    /// A hauled hero's clearing cut waits for them to be on their feet: when it starts, at whom.
    cut: Option<(f32, Id)>,
    /// Alone: his gun is ready again then.
    gun_ready: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Stage {
    /// At their posts, guns at ease: nobody has pulled him yet.
    Posted,
    /// Pulled: he advances, then points his cutlass at his first gunner.
    Order { point_at: f32 },
    /// Whose turn comes next, and when it starts to load.
    Turns { turn: usize, next: f32 },
    /// Both gunners down.
    Alone,
}

struct Aim {
    to: Vec3,
    foe: Option<Id>,
    /// When the load starts, and when the gun reaches the shoulder and the aim locks.
    load_at: f32,
    lock_at: f32,
    loaded: bool,
    locked: bool,
}

#[derive(Default)]
struct Gun {
    /// In his hand (what the clients are shown).
    out: bool,
    /// Put away after this.
    until: f32,
    /// In his hand (true) or put away (false) at this moment, partway through the Sheath clip.
    swap: Option<(f32, bool)>,
}

impl Survival {
    pub(crate) fn is_firing_captain(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == "dm_greenskin")
    }

    pub(crate) fn firing_sees_veil(&self, id: Id) -> bool {
        self.firing_line.crews.contains_key(&id) || self.firing_line.owners.contains_key(&id)
    }

    fn firing_pattern_name(&self, id: Id) -> Option<&str> {
        let m = self.mobs.get(&id)?;
        let (k, _) = m.pattern?;
        Some(self.content.mobs[m.def].patterns.get(k)?.name.as_str())
    }

    /// A shot or a throw the firing line readies itself (its load, aim and hold), with no cast.
    pub(crate) fn firing_shooter(&self, id: Id) -> bool {
        self.firing_line.owners.contains_key(&id)
            || (self.is_firing_captain(id) && matches!(self.firing_pattern_name(id), Some("pistol shot" | "harpoon")))
    }

    /// Both his gunners are down: he attacks without a pause.
    pub(crate) fn firing_alone(&self, id: Id) -> bool {
        self.firing_line.crews.get(&id).is_some_and(|c| c.stage == Stage::Alone)
    }

    /// His gun and harpoon wait for his crew to fall; his cutlass waits for a moment clear of
    /// their shots.
    pub(crate) fn firing_pattern_allowed(&self, id: Id, name: &str) -> bool {
        if !self.is_firing_captain(id) {
            return true;
        }
        let now = self.clock;
        let Some(c) = self.firing_line.crews.get(&id) else { return !matches!(name, "pistol shot" | "harpoon") };
        match name {
            // He advances, and gives his order before his first blow.
            _ if matches!(c.stage, Stage::Order { .. }) => false,
            // The harpoon is never picked: `firing_hook` throws it on the count.
            "harpoon" => false,
            // A hauled hero is let up first, then the cut comes (`step_firing_captain`).
            _ if c.cut.is_some() => false,
            // The third blow was his last until the harpoon has been thrown.
            _ if c.blows >= HARPOON_EVERY => false,
            "pistol shot" => c.stage == Stage::Alone && now >= c.gun_ready,
            _ => !self.firing_clashes(id, name),
        }
    }

    /// Whether his blow `name`, begun now, would land within `CLASH` of a gunner's shot (held a
    /// beat or not: `ai.hold_secs`).
    fn firing_clashes(&self, id: Id, name: &str) -> bool {
        let Some(m) = self.mobs.get(&id) else { return false };
        let d = &self.content.mobs[m.def];
        let Some(p) = d.patterns.iter().find(|p| p.name == name) else { return false };
        let w = p.steps.first().and_then(|s| s.windup).unwrap_or(d.attack.windup).max(self.content.ai.first_blow);
        let (soonest, latest) = (self.clock + w, self.clock + w + self.content.ai.hold_secs.max(0.0));
        self.firing_shots_due(id).iter().any(|t| *t > soonest - CLASH && *t < latest + CLASH)
    }

    /// When his gunners will fire: the shots being readied, and those of the turns to come.
    pub(crate) fn firing_shots_due(&self, owner: Id) -> Vec<f32> {
        let now = self.clock;
        let Some(c) = self.firing_line.crews.get(&owner) else { return Vec::new() };
        let live = |g: &Id| self.mobs.get(g).is_some_and(|m| m.health > 0);
        let windup = |g: &Id| self.mobs.get(g).map_or(0.0, |m| self.content.mobs[m.def].attack.windup);
        let mut due = Vec::new();
        for g in c.gunners.iter().filter(|g| live(g)) {
            if let MobState::Windup { t, .. } = self.mobs[g].state {
                due.push(now + (self.attack_now(*g).windup - t).max(0.0));
            }
        }
        let n = c.gunners.len();
        let (turn, next) = match c.stage {
            Stage::Turns { turn, next } => (turn, next),
            Stage::Order { point_at } => (0, point_at.max(now) + FIRST_LOAD),
            _ => return due,
        };
        for k in 0..2 * n {
            if let Some(g) = Self::next_gunner(&c.gunners, turn + k, live) {
                due.push(next + k as f32 * SHOT_GAP + windup(&g));
            }
        }
        due
    }

    /// Whose shot the beat `turn` is: that gunner's if he lives, else the next one standing, so a
    /// lone gunner shoots on every beat.
    fn next_gunner(gunners: &[Id], turn: usize, live: impl Fn(&Id) -> bool) -> Option<Id> {
        let n = gunners.len();
        (0..n).map(|k| gunners[(turn + k) % n]).find(|g| live(g))
    }

    /// His last gunner's shot hits harder than either of a pair's (`LAST_MIGHT`).
    pub(crate) fn firing_last_gunner(&self, id: Id) -> bool {
        let Some(owner) = self.firing_line.owners.get(&id) else { return false };
        let Some(c) = self.firing_line.crews.get(owner) else { return false };
        let alive = c.gunners.iter().filter(|g| self.mobs.get(g).is_some_and(|m| m.health > 0)).count();
        alive == 1 && self.mobs.get(&id).is_some_and(|m| m.health > 0)
    }

    /// The create packet and later equipment changes agree, including for late observers.
    pub fn firing_line_gun(&self, id: Id) -> Option<u32> {
        (self.firing_line.owners.contains_key(&id) || self.firing_line.guns.get(&id).is_some_and(|g| g.out)).then_some(GUN)
    }

    /// The harpoon in his hand, through its held throw.
    pub fn firing_line_spear(&self, id: Id) -> Option<u32> {
        self.firing_line.spears.contains(&id).then_some(SPEAR)
    }

    fn firing_floor(&self, home: Vec3, off: [f32; 2]) -> Option<Vec3> {
        let p = self.ground_near(home[0] + off[0], home[1] + off[1], home[2]);
        ((p[2] - home[2]).abs() < 1.5
            && !self.land.blocked_near(p[0], p[1], p[2])
            && !self.land.water_near(p[0], p[1], p[2])
            && (self.land.walk_clear(home, p) || self.land.path(home, p).is_some_and(|v| !v.is_empty())))
            .then_some(p)
    }

    /// The crew at their posts before the pull, guns at ease, facing the way he faces.
    fn post_firing_line(&mut self, owner: Id) {
        let Some(def) = self.content.mob("dm_deck_gunner") else { return };
        let (home, face) = (self.mobs[&owner].home, self.mobs[&owner].facing);
        let mut gunners = Vec::new();
        for off in POSTS {
            // A custom arena with a blocked post uses a checked spot beside him, on the same side.
            // Never a gunner in the water or on the floor below the ship.
            let at = self.firing_floor(home, off).or_else(|| self.firing_floor(home, [0.0, off[1].signum() * 1.5]));
            let Some(at) = at else {
                self.events.push(Event::Log(format!("firing line {owner}: unsafe gunner post omitted")));
                continue;
            };
            let id = self.spawn_mob(def, at, None, false);
            let m = self.mobs.get_mut(&id).unwrap();
            m.summoned_by = Some(owner);
            m.state = MobState::Idle { wander_at: f32::MAX };
            m.facing = face;
            self.firing_line.owners.insert(id, owner);
            self.events.push(Event::MobGear { id });
            gunners.push(id);
        }
        self.firing_line.crews.insert(owner, Crew { gunners, stage: Stage::Posted, hold_until: 0.0, blows: 0, stepping_until: None, cut: None, gun_ready: 0.0 });
    }

    /// Pulled (him, or one of his crew first: they all come): the crew turns on the hero, and a
    /// second after the pull he points his cutlass at his first gunner.
    fn start_firing_line(&mut self, owner: Id, foe: Foe) {
        let now = self.clock;
        let Some(c) = self.firing_line.crews.get_mut(&owner).filter(|c| c.stage == Stage::Posted) else { return };
        c.stage = Stage::Order { point_at: now + ORDER_AFTER };
        for g in c.gunners.clone() {
            if let Some(m) = self.mobs.get_mut(&g).filter(|m| m.health > 0) {
                m.state = MobState::Chase { foe };
            }
        }
        if self.mobs.get(&owner).is_some_and(|m| m.state.foe().is_none()) {
            self.engage(owner, foe);
        }
    }

    /// Includes an existing living foe even while veiled. These scheduled combatants never
    /// mistake a solo hero's Vanish for a wipe, nor lose a surviving party member.
    fn firing_foe(&self, owner: Id) -> Option<Id> {
        let m = self.mobs.get(&owner)?;
        let d = &self.content.mobs[m.def];
        let valid = |h: &Id| self.players.get(h).is_some_and(|p| p.health > 0
            && dist2d(p.pos, m.home) <= d.leash && (p.pos[2] - m.home[2]).abs() < 6.0);
        m.state.target().filter(valid).or_else(|| self.players.keys().filter(|h| valid(h))
            .min_by(|a, b| dist2d(self.players[a].pos, m.pos).total_cmp(&dist2d(self.players[b].pos, m.pos))).copied())
    }

    pub(crate) fn clean_firing_lines(&mut self) {
        let now = self.clock;
        // A pulled crew whose fight is over (a wipe, the leash, the captain gone): it goes, and he
        // heads home. Home again, his crew stands at its posts again, whole.
        let gone: Vec<(Id, bool)> = self.firing_line.crews.iter().filter_map(|(id, c)| {
            let Some(m) = self.mobs.get(id).filter(|m| m.health > 0) else { return Some((*id, false)) };
            let over = if c.stage == Stage::Posted {
                // (A gunner taken away unfought, `.es clearfoes`: the crew is posted afresh.)
                matches!(m.state, MobState::Return) || c.gunners.iter().any(|g| !self.mobs.contains_key(g))
            } else {
                m.state.foe().is_none() || matches!(m.state, MobState::Return) || self.firing_foe(*id).is_none()
            };
            over.then_some((*id, c.stage != Stage::Posted))
        }).collect();
        for (id, pulled) in gone {
            self.remove_firing_line(id, true);
            if pulled {
                self.halt(id);
                if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                    m.pattern = None;
                    m.state = MobState::Return;
                }
            }
        }
        let post: Vec<Id> = self.mobs.iter().filter(|(id, m)| self.is_firing_captain(**id) && m.health > 0 && !m.hidden
            && !matches!(m.state, MobState::Return) && !self.firing_line.crews.contains_key(id)
            && self.players.values().any(|p| dist2d(p.pos, m.home) < POST_NEAR && (p.pos[2] - m.home[2]).abs() < 6.0))
            .map(|(id, _)| *id).collect();
        for id in post {
            self.post_firing_line(id);
        }
        // The pull: him, or a gunner hurt, felled or turned on a hero.
        let pulled: Vec<(Id, Foe)> = self.firing_line.crews.iter().filter(|(_, c)| c.stage == Stage::Posted).filter_map(|(id, c)| {
            let own = self.mobs.get(id).and_then(|m| m.state.foe());
            let crew = c.gunners.iter().any(|g| self.mobs.get(g).is_none_or(|m| m.health < m.max_health || m.state.foe().is_some()));
            let crew_foe = c.gunners.iter().find_map(|g| self.mobs.get(g).and_then(|m| m.state.foe()));
            own.or(crew_foe).or_else(|| crew.then(|| self.firing_foe(*id).map(Foe::Hero)).flatten()).map(|f| (*id, f))
        }).collect();
        for (id, foe) in pulled {
            self.start_firing_line(id, foe);
        }
        // Readied shots knocked out of hand: the held pose goes.
        let stale: Vec<Id> = self.firing_line.aims.keys().copied()
            .filter(|id| !self.mobs.get(id).is_some_and(|m| m.health > 0 && matches!(m.state, MobState::Windup { .. }))).collect();
        for id in stale {
            self.firing_line.aims.remove(&id);
            self.events.push(Event::ClipStop { id });
        }
        let dropped: Vec<Id> = self.firing_line.spears.iter().copied().filter(|id| !self.firing_line.aims.contains_key(id)).collect();
        for id in dropped {
            self.firing_line.spears.remove(&id);
            self.events.push(Event::MobGear { id });
        }
        // His gun: in his hand partway through the draw, out a while after a shot, then put away.
        let guns: Vec<Id> = self.firing_line.guns.keys().copied().collect();
        for id in guns {
            if !self.mobs.get(&id).is_some_and(|m| m.health > 0) {
                self.firing_line.guns.remove(&id);
                continue;
            }
            let aiming = self.firing_line.aims.contains_key(&id);
            let g = self.firing_line.guns.get_mut(&id).unwrap();
            if let Some((_, out)) = g.swap.filter(|(at, _)| now >= *at) {
                g.swap = None;
                g.out = out;
                self.events.push(Event::MobGear { id });
            } else if g.out && g.swap.is_none() && !aiming && now >= g.until {
                g.swap = Some((now + DRAW_SWAP, false));
                self.events.push(Event::Clip { id, clip: CLIP_SHEATH, take: 0, from: None, secs: DRAW, at: DRAW, body: ClipBody::Full });
            }
        }
    }

    /// The crew, its shots and the captain's gun go. `bodies`: the fallen go too (a wipe, a
    /// reset); a captain's death leaves them lying where they fell.
    pub(crate) fn remove_firing_line(&mut self, owner: Id, bodies: bool) {
        let Some(c) = self.firing_line.crews.remove(&owner) else { return };
        let mut ids = c.gunners;
        ids.push(owner);
        for id in &ids {
            self.firing_line.owners.remove(id);
            if self.firing_line.aims.remove(id).is_some() {
                self.events.push(Event::ClipStop { id: *id });
            }
            self.firing_line.spears.remove(id);
            self.firing_line.guns.remove(id);
            let fallen = !self.mobs.get(id).is_some_and(|m| m.health > 0);
            if *id != owner && (bodies || !fallen) {
                self.dismiss(*id);
            }
        }
        self.line_shots.retain(|s| !ids.contains(&s.owner));
        self.pending.retain(|s| !ids.contains(&s.attacker));
        if let Some(m) = self.mobs.get_mut(&owner) {
            m.phase = 0;
        }
        self.events.push(Event::MobGear { id: owner });
    }

    pub(crate) fn firing_died(&mut self, id: Id) {
        if self.firing_line.crews.contains_key(&id) {
            self.remove_firing_line(id, false);
            return;
        }
        if !self.firing_line.owners.contains_key(&id) {
            return;
        }
        // A gunner falls: his gun clatters to the deck, and his turns go with him.
        if self.firing_line.aims.remove(&id).is_some() {
            self.events.push(Event::ClipStop { id });
        }
        self.line_shots.retain(|s| s.owner != id);
        self.pending.retain(|s| s.attacker != id);
        self.events.push(Event::Sound { id, sound: CLATTER });
    }

    pub(crate) fn reset_firing_lines(&mut self) {
        let ids: Vec<_> = self.firing_line.crews.keys().copied().collect();
        for id in ids {
            self.remove_firing_line(id, true);
            self.halt(id);
            if let Some(m) = self.mobs.get_mut(&id).filter(|m| m.health > 0) {
                m.pattern = None;
                m.state = MobState::Idle { wander_at: self.clock + 5.0 };
            }
        }
    }

    fn firing_safe(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| m.pattern.is_none() && !matches!(m.state, MobState::Windup { .. })
            && !matches!(m.state, MobState::Recover { until, .. } if until > self.clock)
            && m.stagger_until <= self.clock && !self.disabled(id))
    }

    /// The gunners stand at their posts and fire only on the turns the captain's clock gives
    /// them. They never run the caster AI, so a stagger or a party retarget cannot bunch shots.
    pub(crate) fn step_firing_gunner(&mut self, id: Id, dt: f32) -> bool {
        let Some(owner) = self.firing_line.owners.get(&id).copied() else { return false };
        // Their own legs stay planted, but ordinary melee knockbacks and pulls still move them.
        if self.mobs[&id].thrown_until > self.clock {
            return true;
        }
        self.halt(id);
        if self.disabled(id) || self.mobs[&id].stagger_until > self.clock {
            return true;
        }
        self.firing_aim_beats(id);
        // At ease until the pull (noticed in `clean_firing_lines`).
        if self.firing_line.crews.get(&owner).is_none_or(|c| c.stage == Stage::Posted) {
            return true;
        }
        if matches!(self.mobs[&id].state, MobState::Windup { .. }) {
            self.think(id, dt);
        } else if let Some(h) = self.firing_foe(owner) {
            self.mobs.get_mut(&id).unwrap().state = MobState::Chase { foe: Foe::Hero(h) };
        }
        true
    }

    /// The captain's turn clock. It never holds him still, but for his point at the first gunner:
    /// his own fighting is the ordinary AI, kept clear of the shots by `firing_pattern_allowed`.
    pub(crate) fn step_firing_captain(&mut self, id: Id) -> bool {
        self.firing_aim_beats(id);
        let now = self.clock;
        let Some(c) = self.firing_line.crews.get(&id) else { return false };
        if c.stage == Stage::Posted {
            return false;
        }
        let Some(hero) = self.firing_foe(id) else { return true };
        let foe = Foe::Hero(hero);
        let (stage, hold) = (c.stage, c.hold_until);
        if let Some((at, hauled)) = c.cut {
            // (Never into a gunner's shot, as with his every blow: the hero is up a while longer.)
            if now < at || self.firing_clashes(id, "clearing cut") {
                self.halt(id);
                return true;
            }
            self.firing_line.crews.get_mut(&id).unwrap().cut = None;
            self.firing_cut(id, hauled);
            return true;
        }
        let live = c.gunners.iter().any(|g| self.mobs.get(g).is_some_and(|m| m.health > 0));
        // The last gunner down: at his next free moment he draws his own gun.
        if !live && stage != Stage::Alone {
            if !self.firing_safe(id) {
                return false;
            }
            let c = self.firing_line.crews.get_mut(&id).unwrap();
            c.stage = Stage::Alone;
            c.stepping_until = None;
            self.mobs.get_mut(&id).unwrap().phase = 1;
            self.yell(id, "I'll do it meself.");
            if let Some(k) = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "pistol shot") {
                self.begin_pattern(id, k, false);
                self.windup(id, foe, self.players[&hero].pos);
            }
            return true;
        }
        match stage {
            Stage::Posted => false,
            Stage::Order { point_at } => {
                if now < point_at || !self.firing_safe(id) {
                    return false;
                }
                // His cutlass at the gunner whose turn is first: "Steady... FIRE!"
                let first = self.firing_line.crews[&id].gunners.iter().copied().find(|g| self.mobs.get(g).is_some_and(|m| m.health > 0));
                self.halt(id);
                if let Some(at) = first.map(|g| self.mobs[&g].pos) {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.facing = facing(m.pos, at);
                    self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
                }
                self.events.push(Event::Emote { id, emote: POINT });
                self.yell(id, "Steady... FIRE!");
                let c = self.firing_line.crews.get_mut(&id).unwrap();
                c.stage = Stage::Turns { turn: 0, next: now + FIRST_LOAD };
                c.hold_until = now + ORDER_POINT;
                true
            }
            Stage::Turns { turn, next } => {
                if now >= next {
                    self.firing_turn(id, turn, foe);
                    self.firing_line.crews.get_mut(&id).unwrap().stage = Stage::Turns { turn: turn + 1, next: next + SHOT_GAP };
                }
                now < hold || self.firing_hook(id, hero, foe)
            }
            Stage::Alone => self.firing_hook(id, hero, foe),
        }
    }

    /// The harpoon on its count: after his third blow, once he is free, he steps back from his
    /// hero (to `HARPOON_FAR`, or as far as the bulwark, a wall or `HARPOON_STEP` lets him) and
    /// throws. Whether he is on it now (his own blows wait).
    fn firing_hook(&mut self, id: Id, hero: Id, foe: Foe) -> bool {
        let now = self.clock;
        let Some(c) = self.firing_line.crews.get(&id) else { return false };
        let (blows, stepping) = (c.blows, c.stepping_until);
        let (Some(at), Some(pos)) = (self.players.get(&hero).map(|p| p.pos), self.mobs.get(&id).map(|m| m.pos)) else { return false };
        let dist = dist2d(pos, at);
        let Some(until) = stepping else {
            if blows < HARPOON_EVERY || !self.firing_safe(id) {
                return false;
            }
            // Already far enough, or nowhere to step to (the bulwark at his back): he throws.
            let home = self.mobs[&id].home;
            let leash = self.leash_of(self.mobs[&id].def) * 0.8;
            let want = HARPOON_FAR + HARPOON_OVER;
            let away = facing(at, pos);
            let goal = [0.0f32, 0.5, -0.5, 1.0, -1.0].iter().find_map(|off| {
                let b = away + off;
                let to = self.ground_near(at[0] + b.cos() * want, at[1] + b.sin() * want, pos[2]);
                let ok = dist2d(to, pos) >= 1.0
                    && dist2d(to, home) <= leash
                    && (to[2] - pos[2]).abs() < 1.5
                    && self.land.walk_clear(pos, to)
                    && !self.land.blocked_near(to[0], to[1], to[2])
                    && !self.land.water_near(to[0], to[1], to[2])
                    && !self.through_rail(pos, to);
                ok.then_some(to)
            });
            let Some(to) = goal.filter(|_| dist < HARPOON_FAR) else {
                self.firing_throw(id, hero, foe);
                return true;
            };
            self.sidle_to(id, to, HARPOON_PACE, hero);
            if self.mobs[&id].path.is_empty() {
                self.firing_throw(id, hero, foe);
                return true;
            }
            self.firing_line.crews.get_mut(&id).unwrap().stepping_until = Some(now + HARPOON_STEP);
            return true;
        };
        // Stepping back: stopped by a stagger or a stun, he starts the step again when free.
        if !self.firing_safe(id) {
            self.firing_line.crews.get_mut(&id).unwrap().stepping_until = None;
            return false;
        }
        let arrived = self.mobs[&id].path.is_empty();
        if dist >= HARPOON_FAR || arrived || now >= until {
            // Never into a gunner's shot: he holds his place until the beat is clear.
            if !self.firing_clashes(id, "harpoon") {
                self.firing_throw(id, hero, foe);
            }
        }
        true
    }

    /// The count met and his step done: the harpoon is held back and thrown (`firing_windup`).
    fn firing_throw(&mut self, id: Id, hero: Id, foe: Foe) {
        let Some(k) = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "harpoon") else { return };
        let Some(to) = self.players.get(&hero).map(|p| p.pos) else { return };
        let c = self.firing_line.crews.get_mut(&id).unwrap();
        c.blows = 0;
        c.stepping_until = None;
        self.halt(id);
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = MobState::Chase { foe };
        self.begin_pattern(id, k, false);
        self.yell(id, "Ye're not gettin' away!");
        self.windup(id, foe, to);
    }

    /// One more cutlass blow begun, for the harpoon's count (his shots are not blows).
    pub(crate) fn firing_blow_begun(&mut self, id: Id, k: usize) {
        let Some(m) = self.mobs.get(&id) else { return };
        let shot = self.content.mobs[m.def].patterns.get(k).is_none_or(|p| p.steps.iter().any(|s| s.ranged == Some(true)));
        if let Some(c) = self.firing_line.crews.get_mut(&id).filter(|_| !shot) {
            c.blows += 1;
        }
    }

    /// One gunner's turn: he loads at the hero, or, fallen or reeling, his turn passes unshot.
    fn firing_turn(&mut self, owner: Id, turn: usize, foe: Foe) {
        let gunners = &self.firing_line.crews[&owner].gunners;
        if gunners.is_empty() {
            return;
        }
        let live = |g: &Id| self.mobs.get(g).is_some_and(|m| m.health > 0);
        let Some(g) = Self::next_gunner(gunners, turn, live) else { return };
        let ready = self.mobs.get(&g).is_some_and(|m| m.health > 0 && m.stagger_until <= self.clock
            && !matches!(m.state, MobState::Windup { .. })) && !self.disabled(g);
        if !ready {
            return;
        }
        let target = self.spread_foe(g, foe);
        let Some(to) = self.foe_pos(target) else { return };
        let pos = self.mobs[&g].pos;
        let range = self.attack_now(g).range;
        if clear_line(self.land.as_ref(), pos, to) && dist2d(pos, to) <= range {
            self.windup(g, target, to);
        }
    }

    /// A shot or a throw begins (`Survival::windup`): a gunner's load, his captain's draw, load
    /// and aim, or the harpoon held back in his hand for the whole windup.
    pub(crate) fn firing_windup(&mut self, id: Id, to: Vec3) {
        if !self.firing_shooter(id) {
            return;
        }
        let now = self.clock;
        let foe = self.mobs[&id].state.target();
        let name = self.firing_pattern_name(id).map(str::to_string);
        let windup = self.attack_now(id).windup;
        let (load_at, lock_at, ready) = match name.as_deref() {
            Some("harpoon") => {
                // The harpoon in his hand (the gun put away), held back over his shoulder.
                self.firing_line.guns.remove(&id);
                self.firing_line.spears.insert(id);
                self.events.push(Event::MobGear { id });
                self.events.push(Event::Clip { id, clip: CLIP_THROW_HOLD, take: 0, from: None, secs: windup, at: windup, body: ClipBody::Full });
                (now, now, true)
            }
            Some("pistol shot") if !self.firing_line.owners.contains_key(&id) => {
                // A real draw when the gun is put away: Sheath, the gun in his hand partway.
                let g = self.firing_line.guns.entry(id).or_default();
                let drawn = g.out && g.swap.is_none();
                g.until = f32::MAX;
                let draw = if drawn { 0.0 } else { DRAW };
                if !drawn {
                    g.swap = Some((now + DRAW_SWAP, true));
                    self.events.push(Event::Clip { id, clip: CLIP_SHEATH, take: 0, from: None, secs: DRAW, at: DRAW, body: ClipBody::Full });
                    self.mobs.get_mut(&id).unwrap().hold += DRAW;
                }
                (now + draw, now + draw + LOAD, false)
            }
            _ => (now, now + LOAD, false),
        };
        self.firing_line.aims.insert(id, Aim { to, foe, load_at, lock_at, loaded: ready, locked: ready });
        self.firing_aim_beats(id);
    }

    /// Through a readied shot: the load and its click, the barrel following its hero while it
    /// loads, then the gun at the shoulder and the aim locked there until it fires.
    fn firing_aim_beats(&mut self, id: Id) {
        let now = self.clock;
        let Some(MobState::Windup { t, .. }) = self.mobs.get(&id).map(|m| m.state.clone()) else { return };
        let Some(a) = self.firing_line.aims.get(&id) else { return };
        let (load_at, lock_at, loaded, locked, foe) = (a.load_at, a.lock_at, a.loaded, a.locked, a.foe);
        if locked {
            return;
        }
        if !loaded && now >= load_at {
            self.firing_line.aims.get_mut(&id).unwrap().loaded = true;
            self.events.push(Event::Clip { id, clip: CLIP_LOAD, take: 0, from: None, secs: LOAD, at: LOAD, body: ClipBody::Full });
            self.events.push(Event::Sound { id, sound: CLICK });
        }
        if now < load_at {
            return;
        }
        let pos = self.mobs[&id].pos;
        if let Some(p) = foe.and_then(|h| self.players.get(&h)).filter(|p| p.health > 0).map(|p| p.pos) {
            let f = facing(pos, p);
            let m = self.mobs.get_mut(&id).unwrap();
            if angle_between(m.facing, f) > 4.0 {
                m.facing = f;
                self.events.push(Event::MobStop { id, pos, facing: f });
            }
            self.firing_line.aims.get_mut(&id).unwrap().to = p;
        }
        if now >= lock_at {
            self.firing_line.aims.get_mut(&id).unwrap().locked = true;
            let left = (self.attack_now(id).windup - t).max(0.1);
            self.events.push(Event::Clip { id, clip: CLIP_AIM, take: 0, from: None, secs: left, at: left, body: ClipBody::Full });
        }
    }

    /// Along `facing` from `from`, as far as `range` or the first thing in the way.
    fn firing_line_end(&self, from: Vec3, f: f32, range: f32) -> Vec3 {
        let at = |d: f32| [from[0] + f.cos() * d, from[1] + f.sin() * d, from[2]];
        let mut d = 1.0f32.min(range);
        while d < range {
            let next = (d + 1.0).min(range);
            if !clear_line(self.land.as_ref(), from, at(next)) {
                break;
            }
            d = next;
        }
        at(d)
    }

    /// It fires (or the harpoon flies) along the line its aim locked on: a puff at the muzzle,
    /// the bang, the ball flown at its own speed, the line flashed on the ground for its flight (the optional floor mark),
    /// and a dust burst where it stops.
    pub(crate) fn firing_release(&mut self, id: Id) -> bool {
        if self.firing_line.aims.remove(&id).is_none() {
            return false;
        }
        let now = self.clock;
        let a = self.attack_now(id);
        let (pos, f) = (self.mobs[&id].pos, self.mobs[&id].facing);
        let hook = self.firing_pattern_name(id) == Some("harpoon");
        let end = self.firing_line_end(pos, f, a.range);
        let half = a.radius.max(0.3);
        let speed = a.speed.max(1.0);
        self.events.push(Event::ClipStop { id });
        if hook {
            self.firing_line.spears.remove(&id);
            self.events.push(Event::MobGear { id });
            self.events.push(Event::Missile { caster: id, spell: a.spell, target: Target::Point(end) });
            self.events.push(Event::Clip { id, clip: CLIP_THROW, take: 0, from: None, secs: 0.65, at: 0.2, body: ClipBody::Full });
        } else {
            self.events.push(Event::Clip { id, clip: CLIP_FIRE, take: 0, from: None, secs: 0.8, at: 0.05, body: ClipBody::Full });
            self.events.push(Event::Visual { id, kit: combat::KIT_FALL_DUST, by: None });
            self.events.push(Event::Missile { caster: id, spell: TRACER, target: Target::Point(end) });
            // The gun's own: AttackRifle and the bang (its cast kit), with nothing in flight.
            self.events.push(Event::Missile { caster: id, spell: a.spell, target: Target::Point(end) });
            if let Some(g) = self.firing_line.guns.get_mut(&id) {
                g.until = now + GUN_OUT;
            }
            if let Some(c) = self.firing_line.crews.get_mut(&id) {
                c.gun_ready = now + GUN_EVERY;
            }
        }
        let len = dist2d(pos, end);
        let mid = [(pos[0] + end[0]) / 2.0, (pos[1] + end[1]) / 2.0, pos[2]];
        self.events.push(Event::GroundWarn { at: mid, yaw: f, shape: "line", reach: len / 2.0, arc: 0.0, width: half * 2.0,
            secs: len / speed + 0.2, colour: mobs::HAZARD_COLOUR.into() });
        self.line_shots.push(combat::LineShot { owner: id, from: pos, to: end, radius: half, speed, start: now, reached: 0.0,
            damage: a.damage, blockable: a.blockable, pull_to: hook.then_some(PULL_TO), look: if hook { 0 } else { SPLINTERS },
            struck: Vec::new() });
        true
    }

    /// Hauled in by his harpoon (a fair hit): into his clearing cut, which lands `HAUL_FREE` after
    /// the hero is back on their feet (the haul is a long flight, 1.7 sec, helpless).
    pub(crate) fn firing_pulled(&mut self, id: Id, hero: Id) {
        if !self.mobs.get(&id).is_some_and(|m| m.health > 0) || !self.firing_line.crews.contains_key(&id) {
            return;
        }
        let Some(k) = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "clearing cut") else { return };
        let now = self.clock;
        let stands = match self.players.get(&hero).map(|p| p.action.clone()) {
            Some(Action::Stagger { until }) => until,
            _ => now,
        };
        let d = &self.content.mobs[self.mobs[&id].def];
        let w = d.patterns[k].steps.first().and_then(|s| s.windup).unwrap_or(d.attack.windup).max(self.content.ai.first_blow);
        let start = (stands + HAUL_FREE - w).max(now);
        let m = self.mobs.get_mut(&id).unwrap();
        m.pattern = None;
        m.state = MobState::Chase { foe: Foe::Hero(hero) };
        self.firing_line.crews.get_mut(&id).unwrap().cut = Some((start, hero));
    }

    fn firing_cut(&mut self, id: Id, hero: Id) {
        let Some(k) = self.content.mobs[self.mobs[&id].def].patterns.iter().position(|p| p.name == "clearing cut") else { return };
        let Some(to) = self.players.get(&hero).map(|p| p.pos) else { return };
        self.begin_pattern(id, k, false);
        // The haul's cut is not a blow of the count.
        if let Some(c) = self.firing_line.crews.get_mut(&id) {
            c.blows = 0;
        }
        self.windup(id, Foe::Hero(hero), to);
    }
}
