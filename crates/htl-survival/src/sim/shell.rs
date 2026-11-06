//! Shadowguard (docs/priest-draft3.md): take it, then give it back. A shell of shadow for a moment
//! (a stance): the Priest takes less, moves slowly and can't attack, and what the shell stops is
//! stored. Its key again, or its time up, lets the stored shadow fly at the first foe along the
//! reticle; let go just after a blow lands on them, harder (the perfect). Nothing stored, nothing
//! flies.
//!
//! Lag: a blow is stored as it settles on the server; the let-go is judged on the Priest's own
//! screen (its arrival less their round trip), as Ice Block's break-out is.

use super::*;

impl Survival {
    /// A blow settled on a hero: in a shell, what it stopped is stored, and the blow noted.
    /// `landed` is what got through (the shell's cut already taken).
    pub(crate) fn shell_store(&mut self, victim: Id, attacker: Id, landed: u32) {
        let now = self.clock;
        let Some(s) = self.stances.get(&victim).filter(|s| s.until > now) else { return };
        let Some(sd) = self.content.abilities[s.ability].shell.clone() else { return };
        let cut = sd.cut.clamp(0.0, 0.95);
        let stopped = landed as f32 * cut / (1.0 - cut);
        if let Some(s) = self.stances.get_mut(&victim) {
            s.stored = (s.stored + stopped).min(sd.cap);
            s.struck = Some((now, attacker));
            let n = s.stored.round() as u32;
            self.events.push(Event::PriestLook { player: victim, kind: "shell", n, on: None, a: n as f32 / sd.cap.max(1.0), b: 0.0 });
        }
    }

    /// A shell's damage cut, if the hero is in one.
    pub(crate) fn shell_cut(&self, id: Id) -> f32 {
        let now = self.clock;
        self.stances.get(&id).filter(|s| s.until > now).and_then(|s| self.content.abilities[s.ability].shell.as_ref()).map_or(0.0, |sd| sd.cut)
    }

    /// The shell lets go: what it stored flies at the first foe along the reticle (harder let go
    /// just after a blow, on the hero's own screen). Called as the stance ends, by its key or its
    /// time.
    pub(crate) fn shell_release(&mut self, id: Id, ability: usize, stored: f32, struck: Option<(f32, Id)>) {
        let d = self.content.abilities[ability].clone();
        let Some(sd) = d.shell.clone() else { return };
        self.events.push(Event::PriestLook { player: id, kind: "shell", n: 0, on: None, a: 0.0, b: 1.0 });
        if stored < 1.0 || !self.players.get(&id).is_some_and(|p| p.health > 0) {
            return;
        }
        let Some(shard) = self.content.ability(&sd.shard) else { return };
        let rewind = self.players.get(&id).map_or(0.0, |p| p.rtt.min(self.content.combat.max_rewind));
        let perfect = struck.is_some_and(|(at, _)| self.clock - rewind - at <= sd.window + 1.0e-4);
        let damage = if perfect { (stored * sd.perfect).min(sd.perfect_cap) } else { stored.min(sd.cap) };
        if perfect {
            self.gain_valor(id, self.content.checks.perfect_valor);
            let kit = self.perfect_kit_of(ability);
            if kit != 0 {
                self.events.push(Event::Visual { id, kit, by: None });
            }
            self.events.push(Event::Perfect { player: id, ability });
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.cast_damage = Some(damage);
        }
        self.launch(id, shard, 1.0);
        if let Some(p) = self.players.get_mut(&id) {
            p.cast_damage = None;
        }
    }
}
