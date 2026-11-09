//! The cameraman (`.es watch <hero>`): a hero's client that films another hero's fight from
//! outside it, for the visual check's boss clips (docs/visual-check.md, "Filming a boss"). Client
//! patch 0088 (`WOW_SPECTATE`) points its camera; this is what the server does for it.
//!
//! A watcher is out of the rules: its body leaves `sim.players` for [`SurvivalGame::watchers`],
//! so no creature sees it, no blow or area reaches it, nothing collides with it, and no count of
//! heroes (a dungeon's, a raid's, an encounter's) includes it. The body comes back into the
//! rules only while the watcher's own client is heard (its `.es` lines, its moves), while it
//! leaves or changes worlds, and as its save. No other client is told of it, and it goes where
//! the hero it watches goes: into their world ([`super::realms`]) and near them, so the world
//! around the fight is loaded for its camera.

use super::*;

/// A watcher's body is brought to the hero it watches once it's this far from them (yd).
const NEAR: f32 = 25.0;

impl SurvivalGame {
    /// `.es watch <hero>`, `.es watch off`, `.es watch` (what it's doing).
    pub(crate) fn watch_command(&mut self, conn: ConnId, arg: &str) -> Result<(), String> {
        let Some((me, guid, own)) = self.world_of(conn).map(|w| (w.id, w.guid, w.name.clone())) else { return Ok(()) };
        let was = self.sessions.get(&conn).and_then(|s| s.watching.clone());
        match arg {
            "" => {
                let text = match &was {
                    Some(name) => format!("You're watching {name}. Use .es watch off to play again."),
                    None => "You're playing. Use .es watch <hero> to film a hero's fight from outside it.".to_string(),
                };
                self.say_to(conn, &text);
                Ok(())
            }
            off if off.eq_ignore_ascii_case("off") => {
                let Some(name) = was else { return Err("You aren't watching anyone.".into()) };
                if let Some(s) = self.sessions.get_mut(&conn) {
                    s.watching = None;
                }
                // Seen again: the body is in the rules for this packet, and stays.
                let body = self.player_create(me, false);
                self.broadcast_except(conn, &body);
                println!("[survival] {own} stopped watching {name}");
                self.say_to(conn, "You're playing again. Other heroes can see you.");
                Ok(())
            }
            name if name.eq_ignore_ascii_case(&own) => Err("Watch another hero, not yourself.".into()),
            name => {
                let name = self.hero_name_like(name).unwrap_or_else(|| name.to_string());
                if let Some(s) = self.sessions.get_mut(&conn) {
                    s.watching = Some(name.clone());
                }
                if was.is_none() {
                    self.sim.cancel_preview(me);
                    self.broadcast_except(conn, &packets::destroy_object(guid));
                }
                println!("[survival] {own} is watching {name}");
                self.say_to(conn, &format!("You're watching {name}. No one sees you, and no fight counts you. Use .es watch off to play again."));
                Ok(())
            }
        }
    }

    /// Whether this client is a watcher.
    pub(crate) fn watching(&self, conn: ConnId) -> bool {
        self.sessions.get(&conn).is_some_and(|s| s.watching.is_some())
    }

    /// A watcher's body back in the rules, while its own client is heard (or it leaves).
    pub(crate) fn lend_watcher(&mut self, conn: ConnId) {
        let Some(id) = self.world_of(conn).map(|w| w.id) else { return };
        if let Some(body) = self.watchers.remove(&id) {
            self.sim.players.insert(id, body);
        }
    }

    /// A watcher's body out of the rules again (nothing for anyone else).
    pub(crate) fn set_watcher_aside(&mut self, conn: ConnId) {
        if !self.watching(conn) {
            return;
        }
        let Some(id) = self.world_of(conn).map(|w| w.id) else { return };
        if let Some(body) = self.sim.players.remove(&id) {
            self.watchers.insert(id, body);
        }
    }

    /// The heroes who aren't here, and the watchers' bodies: what the save keeps of heroes
    /// beside those in the rules.
    pub(crate) fn offline_and_watchers(&self) -> BTreeMap<String, PlayerSave> {
        let mut all = self.offline.clone();
        for p in self.watchers.values() {
            all.insert(p.account.clone(), htl_survival::save::save_player(&self.sim.content, p));
        }
        all
    }

    /// A hero in this world by name, as the server knows it (any case), not a watcher.
    fn hero_name_like(&self, name: &str) -> Option<String> {
        self.in_world().find(|(_, s, w)| s.watching.is_none() && w.name.eq_ignore_ascii_case(name)).map(|(_, _, w)| w.name.clone())
    }

    /// Where a hero in this world stands, by name (any case), if they're here and not a watcher.
    pub(crate) fn hero_at(&self, name: &str) -> Option<([f32; 3], f32)> {
        let (_, _, w) = self.in_world().find(|(_, s, w)| s.watching.is_none() && !w.porting && w.name.eq_ignore_ascii_case(name))?;
        self.sim.players.get(&w.id).map(|p| (p.pos, p.facing))
    }

    /// Each watcher whose hero is in this world, and has gone more than [`NEAR`] from it, is put
    /// where the hero stands (its own client unseen there, its camera on the fight).
    pub(crate) fn keep_watchers_near(&mut self) {
        let far: Vec<(ConnId, Id, u64, [f32; 3], f32)> = self
            .in_world()
            .filter(|(_, _, w)| !w.porting)
            .filter_map(|(c, s, w)| {
                let (at, facing) = self.hero_at(s.watching.as_deref()?)?;
                let body = self.watchers.get(&w.id)?;
                let d = (0..3).map(|i| (body.pos[i] - at[i]).powi(2)).sum::<f32>().sqrt();
                (d > NEAR).then_some((*c, w.id, w.guid, at, facing))
            })
            .collect();
        for (conn, id, guid, at, facing) in far {
            if let Some(b) = self.watchers.get_mut(&id) {
                b.pos = at;
                b.facing = facing;
                b.last_safe = at;
            }
            let counter = self.world_mut(conn).map_or(0, |w| {
                w.counter = w.counter.wrapping_add(1);
                w.ground_z = at[2];
                w.fall_top = None;
                w.counter
            });
            self.send(conn, packets::teleport(guid, counter, at, facing));
        }
    }
}

#[cfg(test)]
#[path = "watch_tests.rs"]
mod tests;
