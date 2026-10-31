//! Choosing a card stops the world: no enemy moves, no timer runs, until every waiting pick is
//! made. Nobody can sit on picks for later, and nobody chooses with a ghoul at their throat. A
//! player who leaves their pick for a long time gets the first card, so one idle hero can't hold
//! everyone up forever.

use super::*;

/// After this long paused for one player, their pick is made for them.
pub(super) const PICK_TIMEOUT_SECS: f32 = 60.0;

impl Sim {
    /// Players with a pick waiting.
    /// Whether the window a player has open is their waiting pick (cards or a set piece).
    fn pick_open(&self, id: EntityId) -> bool {
        self.entities
            .get(&id)
            .and_then(|e| e.player())
            .and_then(|p| p.menu.as_ref())
            .is_some_and(|m| m.offer.is_some() || m.kind == MenuKind::SetPiece)
    }

    fn choosers(&self) -> Vec<EntityId> {
        self.order.iter().copied().filter(|id| self.entities.get(id).and_then(|e| e.player()).is_some_and(|p| !p.offers.is_empty())).collect()
    }

    /// The pause, a step of it: true while the world holds still.
    pub(super) fn step_pause(&mut self, dt: f32) -> bool {
        let pausable = self.content.game.pause_for_picks && !matches!(self.phase, Phase::Over { .. });
        let choosing = if pausable { self.choosers() } else { Vec::new() };
        if choosing.is_empty() {
            if !self.paused_for.is_empty() {
                self.resume_world();
            }
            return false;
        }
        if self.paused_for.is_empty() {
            self.pause_world();
            self.pause_secs = 0.0;
        }
        if choosing != self.paused_for {
            self.paused_for = choosing.clone();
            let names = self.names_of(&choosing);
            self.events.push(Event::Paused { choosing: names });
        }
        self.pause_secs += dt;
        // Every waiting pick shows now (no waiting out a revive), and comes back if closed.
        // Another window in the way (the Ancient's shop, a banner, Renown) gives way to the pick.
        for &id in &choosing {
            if !self.pick_open(id) {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.menu_due = false;
                }
                self.show_next_offer(id);
            }
        }
        // Too long: the first card for whoever still hasn't chosen.
        if self.pause_secs >= PICK_TIMEOUT_SECS {
            self.pause_secs = 0.0;
            for id in choosing {
                let name = self.names_of(&[id]).pop().unwrap_or_default();
                if !self.pick_open(id) {
                    self.show_next_offer(id);
                }
                self.menu_select(id, 0);
                self.announce(format!("{name} took too long: the first card was chosen for them."), false);
            }
        }
        let status = self.status();
        if self.last_status != Some(status) {
            self.events.push(Event::Status(status));
            self.last_status = Some(status);
        }
        true
    }

    fn names_of(&self, ids: &[EntityId]) -> Vec<String> {
        ids.iter().filter_map(|id| self.entities.get(id).and_then(|e| e.player()).map(|p| p.name.clone())).collect()
    }

    /// Everything that moves stops where it is, and stops swinging, on every screen.
    fn pause_world(&mut self) {
        let ids: Vec<EntityId> = self.entities.keys().copied().collect();
        for id in ids {
            let Some(e) = self.entities.get(&id) else { continue };
            let (pos, facing) = (e.pos, e.facing);
            let (moving, engaged) = match &e.body {
                Body::Enemy(en) => (en.mover.as_ref().is_some_and(|m| !m.done()), en.engaged),
                Body::Ally(a) => (a.mover.as_ref().is_some_and(|m| !m.done()), a.engaged),
                _ => continue,
            };
            if moving {
                self.events.push(Event::Stop { id, pos, facing });
            }
            if let Some(victim) = engaged {
                self.events.push(Event::AttackStop { attacker: id, victim });
            }
        }
        self.announce_pause();
        // Cooldowns stand still too: every bar shows where it stopped.
        for id in self.player_ids() {
            self.events.push(Event::Bar { player: id });
        }
    }

    fn announce_pause(&mut self) {
        let names = self.names_of(&self.choosers());
        if !names.is_empty() {
            let who = names.join(" and ");
            self.announce(format!("Paused while {who} choose{} a card.", if names.len() == 1 { "s" } else { "" }), false);
        }
    }

    /// Everything picks up where it left off.
    fn resume_world(&mut self) {
        self.paused_for.clear();
        self.pause_secs = 0.0;
        self.events.push(Event::Paused { choosing: Vec::new() });
        for id in self.player_ids() {
            self.events.push(Event::Bar { player: id });
        }
        let ids: Vec<EntityId> = self.entities.keys().copied().collect();
        for id in ids {
            let Some(e) = self.entities.get(&id) else { continue };
            let resume = match &e.body {
                Body::Enemy(en) => (en.mover.as_ref().filter(|m| !m.done()).map(|m| (m.rest(), m.speed)), en.engaged),
                Body::Ally(a) => (a.mover.as_ref().filter(|m| !m.done()).map(|m| (m.rest(), m.speed)), a.engaged),
                _ => continue,
            };
            if let (Some((path, speed)), _) = &resume {
                if path.len() >= 2 {
                    self.events.push(Event::MoveTo { id, path: path.clone(), speed: *speed });
                }
            }
            if let Some(victim) = resume.1 {
                // A ranged enemy shows no swing.
                let ranged = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| self.enemy_def(en.kind).range > 0.0);
                if !ranged {
                    self.events.push(Event::AttackStart { attacker: id, victim });
                }
            }
        }
    }

    /// Who the world is waiting for (empty when it isn't paused).
    pub fn paused_for(&self) -> Vec<String> {
        self.names_of(&self.paused_for)
    }
}
