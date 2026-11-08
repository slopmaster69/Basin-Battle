//! VanCleef's false opening. The Invitation never deals damage on its own; a direct
//! frontal hit spends it once, then the ordinary, committed melee engine answers.

use super::mobs::{Foe, MobState};
use super::*;

#[cfg(test)]
#[path = "vancleef_tests.rs"]
mod tests;

const INVITATION: f32 = 1.8;
const ARRIVAL: f32 = 0.9;
const RETREAT_LIMIT: f32 = 2.5;
const WATCHER: &str = "dm_blackguard_observer";

#[derive(Default)]
pub(crate) struct VanCleef {
    duels: BTreeMap<Id, Duel>,
    observers: BTreeSet<Id>,
}

#[derive(Default)]
struct Duel {
    began: bool,
    invited: bool,
    invite_next: bool,
    final_done: bool,
    departed: bool,
    watchers: Vec<(Id, Vec3)>,
    leaving_until: Option<f32>,
    smoke: Option<(Id, f32)>,
    beat: Beat,
}

#[derive(Clone, Copy, Default)]
enum Beat {
    #[default]
    Fighting,
    Invitation { until: f32 },
    Broken,
    Retreat { until: f32 },
    Arrival { until: f32 },
}

impl Survival {
    pub(crate) fn duel_observer(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == WATCHER)
    }

    pub(crate) fn is_vancleef(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| matches!(self.content.mobs[m.def].key.as_str(), "dm_vancleef" | "dm_vancleef_unbowed"))
    }

    fn duel_pattern_named(&self, id: Id, name: &str) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        self.content.mobs[m.def].patterns.iter().position(|p| p.name == name)
    }

    /// The first exchange is honest. Invitations follow a completed string, not a timer
    /// or a health line. A lunge closes distance without spending that next Invitation.
    pub(crate) fn vancleef_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let near = dist <= d.attack.range + size - 0.3;
        let invite = self.vancleef.duels.get(&id).is_some_and(|d| d.invite_next);
        let name = if near { if invite { "invitation" } else { "string" } } else { "lunge" };
        self.duel_pattern_named(id, name).filter(|k| near || dist <= d.patterns[*k].max_range)
    }

    pub(crate) fn vancleef_string_landed(&mut self, id: Id, pattern: Option<usize>) {
        if !self.is_vancleef(id) { return; }
        if pattern == self.duel_pattern_named(id, "string") {
            self.vancleef.duels.entry(id).or_default().invite_next = true;
        }
    }

    /// Intercept only the harmless stance. The actual riposte uses `windup` normally.
    pub(crate) fn vancleef_invitation(&mut self, id: Id, foe: Foe, to: Vec3) -> bool {
        if !self.is_vancleef(id) || self.mobs[&id].pattern.map(|(k, _)| k) != self.duel_pattern_named(id, "invitation") { return false; }
        self.halt(id);
        let until = self.clock + INVITATION;
        let duel = self.vancleef.duels.entry(id).or_default();
        let first = !duel.invited;
        duel.invited = true;
        duel.invite_next = false;
        duel.beat = Beat::Invitation { until };
        let m = self.mobs.get_mut(&id).unwrap();
        // Keep the harmless pattern while held, including in a controlled lab scene.
        // Only this module advances it; waiting never releases its dummy step.
        m.hold = 0.0;
        m.guarding = true;
        m.facing = facing(m.pos, to);
        m.state = MobState::Recover { foe, until };
        self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
        // The native held guard pose, with both swords still equipped, is distinct from
        // the ordinary release back to ready. It has no block or damage immunity.
        self.events.push(Event::MobGuard { id, up: true });
        if first { self.yell(id, "Go on, then."); }
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
        // Aim at the attacker's present position once. Content gives zero turning and
        // locks for the entire windup, so moving after provoking it always works.
        let to = self.players[&hero].pos;
        self.windup(id, Foe::Hero(hero), to);
    }

    pub(crate) fn vancleef_broken(&mut self, id: Id) {
        if !self.is_vancleef(id) { return; }
        self.lower_invitation(id);
        self.vancleef.duels.entry(id).or_default().beat = Beat::Broken;
        self.yell(id, "You've learned something.");
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

    fn place_duel(&mut self, id: Id) {
        let m = &self.mobs[&id];
        let home = m.home;
        let mut duel = Duel::default();
        // Unbowed inherits the empty deck. These are observers, never a combat wave.
        if self.content.mobs[m.def].key == "dm_vancleef" {
            if let Some(def) = self.content.mob(WATCHER) {
                for side in [-1.0, 1.0] {
                    let Some(at) = self.duel_floor(home, [home[0] + side * 4.0, home[1] + 1.5, home[2]]) else { continue };
                    let Some(exit) = self.duel_floor(at, [home[0] + side * 4.0, home[1] + 13.0, home[2]]) else { continue };
                    let watcher = self.spawn_mob(def, at, None, false);
                    let m = self.mobs.get_mut(&watcher).unwrap();
                    m.state = MobState::Idle { wander_at: f32::MAX };
                    m.facing = facing(at, home);
                    self.vancleef.observers.insert(watcher);
                    duel.watchers.push((watcher, exit));
                }
            }
        }
        self.vancleef.duels.insert(id, duel);
    }

    fn clear_duel_smoke(&mut self, id: Id) {
        if self.objs.remove(&id).is_some() { self.events.push(Event::ObjGone { id }); }
    }

    pub(crate) fn remove_duel(&mut self, id: Id) {
        let Some(duel) = self.vancleef.duels.remove(&id) else { return };
        self.lower_invitation(id);
        if let Some((smoke, _)) = duel.smoke { self.clear_duel_smoke(smoke); }
        for (watcher, _) in duel.watchers {
            self.vancleef.observers.remove(&watcher);
            if self.mobs.remove(&watcher).is_some() { self.events.push(Event::MobGone { id: watcher }); }
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

    fn duel_arrival(&mut self, id: Id) {
        self.halt(id);
        if let Some(hero) = self.duel_hero(id) {
            let to = self.players[&hero].pos;
            let m = self.mobs.get_mut(&id).unwrap();
            m.facing = facing(m.pos, to);
            self.events.push(Event::MobStop { id, pos: m.pos, facing: m.facing });
        }
        // Clear the cloud before the cue and the entire first windup, even on a failed path.
        if let Some((smoke, _)) = self.vancleef.duels.get_mut(&id).unwrap().smoke.take() { self.clear_duel_smoke(smoke); }
        self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Arrival { until: self.clock + ARRIVAL };
        self.events.push(Event::Sound { id, sound: 1335 });
        self.events.push(Event::Visual { id, kit: 379, by: None });
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

    /// Runs after walking, before ordinary thought and the disabled check. This lets a
    /// rooted retreat time out safely; a stunned arrival still waits for control to end.
    pub(crate) fn step_vancleef(&mut self, id: Id) -> bool {
        if self.vancleef.observers.contains(&id) { return true; }
        if !self.is_vancleef(id) || !self.vancleef.duels.contains_key(&id) { return false; }
        let now = self.clock;
        if let Some((smoke, until)) = self.vancleef.duels[&id].smoke {
            if now >= until {
                self.clear_duel_smoke(smoke);
                self.vancleef.duels.get_mut(&id).unwrap().smoke = None;
            }
        }
        let Some(foe) = self.mobs[&id].state.foe() else { return false };
        self.vancleef.duels.get_mut(&id).unwrap().began = true;
        // Wipes and leash exits clear poses, smoke and observers even during the stance/run.
        let Some(hero) = self.duel_hero(id) else {
            self.remove_duel(id);
            self.halt(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.state = MobState::Return;
            m.pattern = None;
            return true;
        };
        if !self.vancleef.duels[&id].departed && self.mobs[&id].health as f32 <= self.mobs[&id].max_health as f32 * 0.30 {
            let duel = self.vancleef.duels.get_mut(&id).unwrap();
            duel.departed = true;
            duel.leaving_until = Some(now + 8.0);
            let watchers = duel.watchers.clone();
            if !watchers.is_empty() { self.yell(id, "Cowards! All of you!"); }
            for (watcher, exit) in watchers { self.go(watcher, exit, 2.2); }
        }
        if self.vancleef.duels[&id].leaving_until.is_some_and(|until| now >= until) {
            let duel = self.vancleef.duels.get_mut(&id).unwrap();
            duel.leaving_until = None;
            for (watcher, _) in std::mem::take(&mut duel.watchers) {
                self.vancleef.observers.remove(&watcher);
                if self.mobs.remove(&watcher).is_some() { self.events.push(Event::MobGone { id: watcher }); }
            }
        }
        match self.vancleef.duels[&id].beat {
            Beat::Invitation { until } => {
                if self.disabled(id) || self.mobs[&id].stagger_until > now {
                    self.lower_invitation(id);
                    self.mobs.get_mut(&id).unwrap().pattern = None;
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                } else if now >= until {
                    self.lower_invitation(id);
                    self.vancleef.duels.get_mut(&id).unwrap().beat = Beat::Fighting;
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.pattern = None;
                    m.state = MobState::Recover { foe, until: now + 1.2 };
                    m.ready_at = now + 1.2;
                }
                true
            }
            Beat::Broken => {
                if self.disabled(id) || self.mobs[&id].stagger_until > now { return true; }
                let (at, home) = (self.mobs[&id].pos, self.mobs[&id].home);
                let away = facing(self.players[&hero].pos, at);
                let to = [0.0f32, 0.6, -0.6, 1.2, -1.2].into_iter().find_map(|turn| {
                    self.duel_floor(at, [at[0] + (away + turn).cos() * 4.0, at[1] + (away + turn).sin() * 4.0, at[2]])
                        .filter(|p| dist2d(*p, home) < 18.0)
                });
                if let Some(to) = to {
                    // A small native smoke emitter, removed before arrival. Visual only:
                    // no concealment, threat loss, collision or damage.
                    let smoke = self.alloc();
                    self.objs.insert(smoke, Obj { kind: ObjKind::Prop { display: 45, scale: 0.35 }, pos: at, facing: 0.0 });
                    self.events.push(Event::ObjSpawned { id: smoke });
                    self.vancleef.duels.get_mut(&id).unwrap().smoke = Some((smoke, now + 0.65));
                    self.go(id, to, 4.0);
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
                let boundary = matches!(m.state, MobState::Chase { .. }) || matches!(m.state, MobState::Recover { until, .. } if now >= until && m.pattern.is_none());
                if boundary && !self.disabled(id) && m.stagger_until <= now && !self.vancleef.duels[&id].final_done
                    && m.health as f32 <= m.max_health as f32 * 0.12 {
                    self.vancleef.duels.get_mut(&id).unwrap().final_done = true;
                    self.yell(id, "This ends here.");
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
