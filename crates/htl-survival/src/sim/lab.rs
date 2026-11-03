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
        _ => return None,
    })
}

pub const LAB_SCENES: &str = "one, pack, close, ring or far";

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
        // Locked on to the middle body, so the client turns the hero (and the camera behind them)
        // to the scene: turning the camera alone turned the body with it and the blows went wide.
        if let Some(&mid) = out.get(out.len() / 2) {
            if let Some(p) = self.players.get_mut(&id) {
                p.lock = Some(mid);
                p.lock_seen.clear();
                p.facing = facing(pos, self.mobs[&mid].pos);
            }
            self.events.push(Event::Lock { player: id, target: Some(mid) });
        }
        Ok(out)
    }

    /// One of the lab's creatures swings at the hero now (its own blow, wind-up and all).
    pub fn lab_blow(&mut self, mob: Id, hero: Id) {
        let Some(at) = self.players.get(&hero).map(|p| p.pos) else { return };
        if self.mobs.get(&mob).is_some_and(|m| m.health > 0) {
            if let Some(m) = self.mobs.get_mut(&mob) {
                m.facing = facing(m.pos, at);
            }
            self.windup(mob, Foe::Hero(hero), at);
        }
    }

    /// The lab's creatures held where they stand: struck, they'd come at the hero, and the next
    /// run's frames would be of a brawl. A wind-up the lab asked for (`lab_blow`) and a reel go on.
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

    /// The lab's bodies taken away.
    pub fn lab_clear(&mut self, mobs: &[Id]) {
        let held: Vec<Id> = self.players.iter().filter(|(_, p)| p.lock.is_some_and(|t| mobs.contains(&t))).map(|(id, _)| *id).collect();
        for h in held {
            self.unlock(h);
        }
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

    /// The hero is aiming an ability (the lab's click lets it go).
    pub fn lab_aiming(&self, id: Id) -> bool {
        matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Aim { .. }))
    }
}

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
        s.lab_clear(&mobs);
        assert_eq!(s.players[&p].lock, None);
    }
}
