//! The spell lab (`.es lab`, dev builds): a fixed scene set out in front of a hero, so each
//! ability can be cast the same way again and again and its looks captured and compared. The
//! server drives the presses and records what it sent; the add-on takes the screenshots.

use super::mobs::{Foe, MobState};
use super::*;

/// Where a scene's bodies stand, from the hero along their facing: (yards ahead, degrees off).
fn layout(scene: &str) -> Option<&'static [(f32, f32)]> {
    Some(match scene {
        // One body at a middling reach: a shot, a cone, a leap's landing.
        "one" => &[(5.0, 0.0)],
        // Three spread across the front: a cone, a cleave, a line that pierces.
        "pack" => &[(4.5, -22.0), (5.5, 0.0), (4.5, 22.0)],
        // One within arm's reach: a blow, a stance's catch.
        "close" => &[(1.8, 0.0)],
        // Three within arm's reach: a bash or a sweep that strikes more than one.
        "ring" => &[(1.9, -32.0), (2.1, 0.0), (1.9, 32.0)],
        // Far: a long shot, a charge's run.
        "far" => &[(14.0, 0.0)],
        // Two in a row along the hero's facing: a rush that drives one into the other, a shot
        // that pierces.
        "line" => &[(4.0, 0.0), (7.5, 0.0)],
        // Three spread across the front at 8 yd: a jet that sweeps across them.
        "spread" => &[(8.0, -28.0), (8.0, 0.0), (8.0, 28.0)],
        // Three in a row at 6, 10 and 14 yd: an orb that rolls through them.
        "row" => &[(6.0, 0.0), (10.0, 0.0), (14.0, 0.0)],
        // One at 12 yd: a bolt's flight.
        "mid" => &[(12.0, 0.0)],
        // One within arm's reach at the hero's side: a dodge or a glide forward leaves its blow.
        "side" => &[(1.8, 90.0)],
        // At the edge of a long reach: Mind Flay's taut zone (26 to 30 yd).
        "edge" => &[(28.0, 0.0)],
        _ => return None,
    })
}

/// The account every spell lab stand-in ally hero's starts with: never saved.
pub const LAB_ALLY: &str = "lab~ally";

pub const LAB_SCENES: &str = "one, pack, close, ring, far, line, spread, row, mid, edge or side";

impl Survival {
    /// Sets out a scene in front of hero `id`: `creature`s (a `[[mob]]` key; "dummy" for the
    /// training dummy) facing them, idle until struck. The bodies set out, nearest first.
    pub fn lab_scene(&mut self, id: Id, scene: &str, creature: &str) -> Result<Vec<Id>, String> {
        let spots = layout(scene).ok_or_else(|| format!("No scene called {scene}. Try {LAB_SCENES}."))?;
        let def = if creature == "dummy" {
            self.content.mobs.iter().position(|m| m.dummy).ok_or("No training dummy in the rules.")?
        } else {
            self.content.mob(creature).ok_or_else(|| format!("No creature called {creature}."))?
        };
        let p = self.players.get(&id).ok_or("No hero.")?;
        let (pos, f) = (p.pos, p.facing);
        let mut out = Vec::new();
        for (ahead, off) in spots {
            let a = f + off.to_radians();
            let at = self.ground_near(pos[0] + a.cos() * ahead, pos[1] + a.sin() * ahead, pos[2] + 1.0);
            let mid = self.spawn_mob(def, at, None, false);
            if let Some(m) = self.mobs.get_mut(&mid) {
                m.facing = facing(at, pos);
                m.home = at;
                m.state = MobState::Idle { wander_at: f32::MAX };
            }
            out.push(mid);
        }
        self.lab_quiet.extend(out.iter().copied());
        self.lab_lock(id, &out);
        Ok(out)
    }

    /// Hero `id` locked on to the middle of a scene's bodies, so the client turns the hero (and
    /// the camera behind them) to the scene: turning the camera alone turned the body with it and
    /// the blows went wide. Again as the burst starts: a hero just set down on the stage lost the
    /// lock at once, to a step their client sent from where they had stood.
    pub fn lab_lock(&mut self, id: Id, scene: &[Id]) {
        let Some(&mid) = scene.get(scene.len() / 2) else { return };
        let Some(at) = self.mobs.get(&mid).filter(|m| m.health > 0).map(|m| m.pos) else { return };
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.lock == Some(mid) {
            return;
        }
        p.lock = Some(mid);
        p.lock_seen.clear();
        p.facing = facing(p.pos, at);
        self.events.push(Event::Lock { player: id, target: Some(mid) });
    }

    /// Hero `id` lets go of whatever they're locked on to (the lab's camera hero, watching a
    /// stand-in from the side, holds no lock: it would turn them to it).
    pub fn unlock_hero(&mut self, id: Id) {
        self.unlock(id);
    }

    /// One of the lab's creatures swings at the hero now (its own blow, wind-up and all).
    /// The creature brought within arm's reach of `hero` first (just beyond them from the lab's
    /// hero, so a ray or the reticle meets the ally first, facing them), then its blow: a blow meant for a stand-in ally that stands farther off.
    pub fn lab_blow_near(&mut self, mob: Id, hero: Id, from: Id) {
        let (Some(at), Some(me)) = (self.players.get(&hero).map(|p| p.pos), self.players.get(&from).map(|p| p.pos)) else { return };
        let d = dist2d(at, me).max(0.1);
        let pos = [at[0] - (me[0] - at[0]) / d * 1.8, at[1] - (me[1] - at[1]) / d * 1.8, at[2]];
        if let Some(m) = self.mobs.get_mut(&mob) {
            m.pos = pos;
            m.facing = facing(pos, at);
        }
        self.events.push(Event::MobStop { id: mob, pos, facing: facing(pos, at) });
        self.lab_blow(mob, hero);
    }

    pub fn lab_blow(&mut self, mob: Id, hero: Id) {
        let Some(at) = self.players.get(&hero).map(|p| p.pos) else { return };
        if self.mobs.get(&mob).is_some_and(|m| m.health > 0) {
            if let Some(m) = self.mobs.get_mut(&mob) {
                m.facing = facing(m.pos, at);
            }
            self.lab_blowing = Some(mob);
            self.windup(mob, Foe::Hero(hero), at);
            self.lab_blowing = None;
        }
    }

    /// The lab's creatures held where they stand: struck, they'd come at the hero, and the next
    /// run's frames would be of a brawl. A wind-up the lab asked for (`lab_blow`) and a reel go on;
    /// they never start one of their own (`lab_quiet`): a struck brave's own blow once staggered
    /// the hero and the run's next ability never came out.
    pub fn lab_hold(&mut self, mobs: &[Id]) {
        for m in mobs {
            let chasing = self.mobs.get(m).is_some_and(|x| matches!(x.state, MobState::Chase { .. } | MobState::Recover { .. }));
            if chasing {
                self.halt_mob(*m);
                if let Some(x) = self.mobs.get_mut(m) {
                    x.state = MobState::Idle { wander_at: f32::MAX };
                }
            }
        }
    }

    /// Hero `id` ready for the next run: what they left burning on the ground (a Ravager's axe
    /// still spinning in the last run's spot struck the next scene) put out, and a stagger the last
    /// run's creature gave them ended.
    pub fn lab_reset(&mut self, id: Id) {
        let theirs: Vec<Option<Id>> = self.burnings.iter().filter(|b| b.owner == id).map(|b| b.mark).collect();
        self.burnings.retain(|b| b.owner != id);
        for m in theirs.into_iter().flatten() {
            self.unmark(m);
        }
        if self.players.get(&id).is_some_and(|p| matches!(p.action, Action::Stagger { .. })) {
            self.set_action(id, Action::Ready);
        }
        // An aim or a charge left from the last run would eat this run's first key.
        self.ability_cancel(id);
    }

    /// A weapon or shield in the lab hero's hands for a run (`weapon ember_staff`), wherever they
    /// stand: the rack's `arm` without the rack.
    pub fn lab_arm(&mut self, id: Id, key: &str) -> Result<String, String> {
        let item = self.content.item(key).ok_or_else(|| format!("No weapon called {key}."))?;
        let kind = self.content.items[item].kind;
        let p = self.players.get_mut(&id).ok_or("No such hero.")?;
        match kind {
            ItemKind::Weapon => p.weapon = item,
            ItemKind::Shield => p.shield = item,
            _ => return Err(format!("{key} isn't a weapon.")),
        }
        p.action = Action::Ready;
        p.guard_held = false;
        let name = self.content.items[item].name.clone();
        self.events.push(Event::Guard { player: id, up: false });
        self.events.push(Event::Gear { player: id });
        self.update_speed(id);
        self.events.push(Event::Dirty);
        Ok(name)
    }

    /// A charge under way: when it began and this cast's rolled flare (the release check's
    /// moment), for the lab to let go on it, or off it.
    pub fn lab_check_flare(&self, id: Id) -> Option<(f32, f32)> {
        let p = self.players.get(&id)?;
        match p.action {
            Action::Charge { since, .. } => Some((since, p.check_flare)),
            _ => None,
        }
    }

    /// A meter sweep under way: when it began, its rolled length and its sweet spot (a share), for
    /// the lab to stop it on the spot, or off it.
    pub fn lab_meter(&self, id: Id) -> Option<(f32, f32, f32)> {
        let p = self.players.get(&id)?;
        match p.action {
            Action::Sweep { since, .. } => Some((since, p.sweep_secs, p.meter_at)),
            _ => None,
        }
    }

    /// A rhythm under way: when it began, its rolled beats, its free beats and each beat's window,
    /// for the lab to press them.
    /// Penance under way: when each of its rings closes (the game clock) and its window.
    pub fn lab_penance(&self, id: Id) -> Vec<(f32, f32)> {
        let Some(x) = self.light.penances.iter().find(|x| x.owner == id) else { return Vec::new() };
        let w = self.content.abilities[x.ability].penance.as_ref().map_or(0.15, |p| p.window);
        x.rings.iter().map(|r| (r.at, w)).collect()
    }

    pub fn lab_rhythm(&self, id: Id) -> Option<(f32, Vec<(f32, bool, f32)>, u32)> {
        let p = self.players.get(&id)?;
        let Action::Rhythm { ability, since, .. } = p.action else { return None };
        let d = &self.content.abilities[ability];
        let span = d.most_beats.saturating_sub(d.free_beats + 1).max(1) as f32;
        let beats = p
            .rhythm_beats
            .iter()
            .enumerate()
            .map(|(k, (t, rest))| {
                let s = ((k as u32).saturating_sub(d.free_beats) as f32 / span).clamp(0.0, 1.0);
                (*t, *rest, d.beat_window[0] + (d.beat_window[1] - d.beat_window[0]) * s)
            })
            .collect();
        Some((since, beats, d.free_beats))
    }

    /// The lab's bodies taken away.
    pub fn lab_clear(&mut self, mobs: &[Id]) {
        let held: Vec<Id> = self.players.iter().filter(|(_, p)| p.lock.is_some_and(|t| mobs.contains(&t))).map(|(id, _)| *id).collect();
        for h in held {
            self.unlock(h);
        }
        self.lab_quiet.retain(|q| !mobs.contains(q));
        for m in mobs {
            if self.mobs.remove(m).is_some() {
                self.events.push(Event::MobGone { id: *m });
            }
        }
    }

    /// What the hero is doing, in a word, for the lab's trace.
    pub fn lab_action(&self, id: Id) -> String {
        match self.players.get(&id).map(|p| &p.action) {
            Some(Action::Swing { strike, .. }) => format!("swing {:?}", strike.kind),
            Some(a) => format!("{a:?}").split([' ', '{', '(']).next().unwrap_or("").to_string(),
            None => String::new(),
        }
    }

    /// Aims hero `id`'s reticle through `at`, from where the reticle's line starts (over the
    /// shoulder when the camera is there), not from the hero's middle: aimed from the middle, the
    /// line passed beside a body at arm's length and a leap flew on to the ground beyond it.
    pub fn lab_aim_at(&mut self, id: Id, at: Vec3) {
        for _ in 0..3 {
            if !self.players.contains_key(&id) {
                return;
            }
            let (o, _) = self.reticle(id);
            let flat = dist2d(o, at).max(0.3);
            let yaw = (at[1] - o[1]).atan2(at[0] - o[0]);
            self.set_aim_line(id, yaw, (at[2] - o[2]).atan2(flat));
        }
    }

    /// The hero is aiming an ability (the lab's click lets it go).
    pub fn lab_aiming(&self, id: Id) -> bool {
        matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Aim { .. }))
    }

    /// The lab's stage: the nearest open, level ground to `near` (the camp's spawn, for one),
    /// with no trunk, wall or water within [`STAGE_CLEAR`] yards and no creature of the world
    /// within [`STAGE_QUIET`], outside the camp's yard (its fire and stores stood behind the
    /// scene, a creature in the flames), and the way to face there. Every run starts on it, so the camera
    /// never ends in a canopy and each run is framed like the last (a run that left the hero
    /// wherever a charge or a leap put them once framed the next one through a tree).
    pub fn lab_stage(&self, near: [f32; 2], skip: &[Id]) -> Option<(Vec3, f32)> {
        let trees = self.land.trees();
        let f = self.content.world.spawn_facing;
        let w = &self.content.world;
        let (yard_at, yard) = (w.build_center, w.build_radius + STAGE_CLEAR);
        let open = |x: f32, y: f32| -> bool {
            if !self.land.in_zone(x, y) || self.land.blocked(x, y) || self.land.water(x, y) {
                return false;
            }
            if yard > STAGE_CLEAR && dist2d([yard_at[0], yard_at[1], 0.0], [x, y, 0.0]) < yard {
                return false;
            }
            let z = self.land.height(x, y);
            if trees.iter().any(|t| dist2d(*t, [x, y, 0.0]) < STAGE_CLEAR) {
                return false;
            }
            // Level, and clear all round: the scene ahead and the camera behind.
            for k in 0..16 {
                let a = k as f32 * std::f32::consts::TAU / 16.0;
                for r in [STAGE_CLEAR * 0.5, STAGE_CLEAR] {
                    let (px, py) = (x + a.cos() * r, y + a.sin() * r);
                    if !self.land.in_zone(px, py) || self.land.blocked(px, py) || self.land.water(px, py) {
                        return false;
                    }
                    if (self.land.height(px, py) - z).abs() > STAGE_LEVEL * r / STAGE_CLEAR {
                        return false;
                    }
                }
            }
            !self.mobs.iter().any(|(id, m)| m.health > 0 && !skip.contains(id) && dist2d(m.pos, [x, y, 0.0]) < STAGE_QUIET)
        };
        // Ring by ring out from `near`, so the nearest open spot is the one found, each time.
        let mut r = 0.0f32;
        while r <= STAGE_SEARCH {
            let n = ((r * std::f32::consts::TAU / STAGE_STEP).ceil() as usize).max(1);
            for k in 0..n {
                let a = k as f32 * std::f32::consts::TAU / n as f32;
                let (x, y) = (near[0] + a.cos() * r, near[1] + a.sin() * r);
                if open(x, y) {
                    return Some((self.ground(x, y), f));
                }
            }
            r += STAGE_STEP;
        }
        None
    }

    /// Puts hero `id` on the lab's stage, facing its way, unless they stand there already: true
    /// if they were moved (their client then needs a moment to settle there).
    pub fn lab_to_stage(&mut self, id: Id, stage: (Vec3, f32)) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        if dist2d(p.pos, stage.0) < 0.5 && (p.pos[2] - stage.0[2]).abs() < 2.0 {
            if let Some(p) = self.players.get_mut(&id) {
                p.facing = stage.1;
            }
            return false;
        }
        self.set_down(id, stage.0, stage.1);
        true
    }
}

/// The lab's stage is this far from any trunk, wall or water (the camera stands ~8 yd behind
/// the hero, a scene's far body 14 yd ahead).
pub const STAGE_CLEAR: f32 = 22.0;
/// No creature of the world this near the stage when it's chosen.
const STAGE_QUIET: f32 = 40.0;
/// The ground within the clear ring rises or falls no more than this.
const STAGE_LEVEL: f32 = 2.5;
/// How far from the spawn the stage is looked for, in steps of this.
const STAGE_SEARCH: f32 = 600.0;
const STAGE_STEP: f32 = 6.0;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::Content;
    use crate::land::FlatLand;

    #[test]
    fn a_scene_locks_the_hero_on_to_its_middle_body_and_clearing_it_lets_go() {
        let mut s = Survival::new(Content::dev(), Box::new(FlatLand::default()), 7, 1_000_000.0);
        s.mobs.clear();
        s.respawns.clear();
        let p = s.join("ayla", "Ayla", None);
        s.move_player(p, [0.0, 0.0, 0.0], 1.0, Moving::default());
        let _ = s.step(0.0);
        let mobs = s.lab_scene(p, "ring", "dummy").unwrap();
        assert_eq!(s.players[&p].lock, Some(mobs[1]));
        assert!((s.players[&p].facing - 1.0).abs() < 0.05, "facing the middle body, straight ahead");
        // Lost (a stale step from far off), and locked on again as the burst starts.
        s.unlock(p);
        s.lab_lock(p, &mobs);
        assert_eq!(s.players[&p].lock, Some(mobs[1]));
        s.lab_clear(&mobs);
        assert_eq!(s.players[&p].lock, None);
    }

    #[test]
    fn the_labs_aim_meets_the_body_it_is_aimed_at_from_over_the_shoulder() {
        let mut s = Survival::new(Content::dev(), Box::new(FlatLand::default()), 7, 1_000_000.0);
        s.mobs.clear();
        s.respawns.clear();
        let p = s.join("ayla", "Ayla", None);
        s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
        let _ = s.step(0.0);
        if let Some(h) = s.players.get_mut(&p) {
            h.shoulder_cam = true;
        }
        let m = s.lab_scene(p, "one", "brave").unwrap()[0];
        let at = s.mobs[&m].pos;
        s.lab_aim_at(p, at);
        let landing = s.aim_point(p, 16.0);
        assert!(dist2d(landing, at) < 1.0, "the aim lands on the brave 5 yd off, not beyond: {landing:?} vs {at:?}");
    }

    #[test]
    fn a_scene_creature_swings_only_when_the_lab_asks_it_to() {
        let mut s = Survival::new(Content::dev(), Box::new(FlatLand::default()), 7, 1_000_000.0);
        s.mobs.clear();
        s.respawns.clear();
        let p = s.join("ayla", "Ayla", None);
        s.move_player(p, [0.0, 0.0, 0.0], 0.0, Moving::default());
        let _ = s.step(0.0);
        let m = s.lab_scene(p, "close", "brave").unwrap()[0];
        // Its own blow (struck, it would turn on the hero): never started.
        s.windup(m, Foe::Hero(p), s.players[&p].pos);
        assert!(!matches!(s.mobs[&m].state, MobState::Windup { .. }), "no blow of its own");
        // The one a `blow` step asks for: it goes on.
        s.lab_blow(m, p);
        s.lab_hold(&[m]);
        assert!(matches!(s.mobs[&m].state, MobState::Windup { .. }), "the lab's blow goes on");
        // Cleared away, it's an ordinary creature again.
        s.lab_clear(&[m]);
        assert!(s.lab_quiet.is_empty());
    }

    #[test]
    fn the_labs_stage_is_the_nearest_ground_clear_of_trees_and_each_run_starts_on_it() {
        // Trees all round the spawn, out to 30 yd: the stage is past them, and the same each time.
        let trees: Vec<Vec3> = (0..12).flat_map(|k| {
            let a = k as f32 * std::f32::consts::TAU / 12.0;
            [10.0f32, 30.0].map(|r| [a.cos() * r, a.sin() * r, 0.0])
        }).collect();
        let land = FlatLand { trees: trees.clone(), ..Default::default() };
        let mut s = Survival::new(Content::dev(), Box::new(land), 7, 1_000_000.0);
        s.mobs.clear();
        s.respawns.clear();
        let p = s.join("ayla", "Ayla", None);
        s.move_player(p, [3.0, 0.0, 0.0], 0.0, Moving::default());
        let _ = s.step(0.0);
        let stage = s.lab_stage([0.0, 0.0], &[]).expect("open ground past the trees");
        assert!(trees.iter().all(|t| dist2d(*t, stage.0) >= STAGE_CLEAR), "clear of every trunk: {stage:?}");
        assert!(dist2d(stage.0, [0.0, 0.0, 0.0]) < 30.0 + STAGE_CLEAR + 8.0, "and the nearest such ground: {stage:?}");
        assert_eq!(s.lab_stage([0.0, 0.0], &[]).map(|x| x.0), Some(stage.0), "the same stage each time");
        // Out of the camp's yard too: its fire once burned behind the scene's body.
        let mut c = Content::dev();
        c.world.build_center = [200.0, 0.0];
        c.world.build_radius = 30.0;
        let mut t = Survival::new(c, Box::new(FlatLand::default()), 7, 1_000_000.0);
        t.mobs.clear();
        let yard = t.lab_stage([200.0, 0.0], &[]).expect("open ground past the yard").0;
        assert!(dist2d(yard, [200.0, 0.0, 0.0]) >= 30.0 + STAGE_CLEAR - 0.1, "outside the yard: {yard:?}");
        assert!(s.lab_to_stage(p, stage), "moved there");
        assert!(dist2d(s.players[&p].pos, stage.0) < 0.1);
        assert!(!s.lab_to_stage(p, stage), "already there: not moved again");
    }
}
