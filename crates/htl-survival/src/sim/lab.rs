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
