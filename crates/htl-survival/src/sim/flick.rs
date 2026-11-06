//! Penance's flick: three bolts over a second, each along the reticle as it goes (a rhythm whose
//! beats all go by themselves, `rhythm.rs`), the first friend each meets healed and the first foe
//! hurt. Sweep the reticle between a friend and a foe: every bolt landing, on at least one of each,
//! is a perfect, and the last bolt bursts in a ring of light where it lands.
//!
//! Holy Fire's Kindle is here too: the burn it lays is fanned by each of the caster's projectiles
//! that strikes the burning foe after (a staff's bolts, Penance's), hotter and starting over.

use super::*;

/// A Penance cast, its bolts counted as they land.
#[derive(Debug, Clone)]
pub(crate) struct Flick {
    pub owner: Id,
    pub ability: usize,
    pub since: f32,
    pub landed: u32,
    pub friend: bool,
    pub foe: bool,
}

/// A flick's bolts have this long from the first to land; then it's forgotten.
const FLICK_LIFE: f32 = 4.0;

impl Survival {
    /// Penance begins: its bolts are counted from now (a cast before it is forgotten).
    pub(crate) fn start_flick(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        self.light.flicks.retain(|f| f.owner != id);
        self.light.flicks.push(Flick { owner: id, ability, since: now, landed: 0, friend: false, foe: false });
        self.events.push(Event::PriestLook { player: id, kind: "penance", n: 0, on: None, a: 0.0, b: 0.0 });
    }

    pub(crate) fn step_flicks(&mut self) {
        let now = self.clock;
        self.light.flicks.retain(|f| now - f.since <= FLICK_LIFE);
    }

    /// One of a flick's bolts reached someone (`friend`: a friend it heals, else a foe). The last
    /// of them, all landed, on a friend and a foe: the perfect, and its burst where it landed.
    pub(crate) fn flick_landed(&mut self, p: &abilities::Projectile, friend: bool) {
        let Some(k) = self.light.flicks.iter().position(|f| f.owner == p.owner && f.ability == p.ability) else { return };
        let most = self.content.abilities[p.ability].most_beats;
        let f = &mut self.light.flicks[k];
        f.landed += 1;
        if friend {
            f.friend = true;
        } else {
            f.foe = true;
        }
        let (n, a, b, owner) = (f.landed, f32::from(u8::from(f.friend)), f32::from(u8::from(f.foe)), f.owner);
        self.events.push(Event::PriestLook { player: owner, kind: "penance", n, on: None, a, b });
        let f = &mut self.light.flicks[k];
        if f.landed < most {
            return;
        }
        let f = self.light.flicks.remove(k);
        if !(f.friend && f.foe) {
            return;
        }
        let d = self.content.abilities[f.ability].clone();
        let c = self.content.checks.clone();
        self.gain_valor(f.owner, c.perfect_valor);
        if c.perfect_kit != 0 {
            self.events.push(Event::Visual { id: f.owner, kit: c.perfect_kit, by: None });
        }
        self.events.push(Event::Perfect { player: f.owner, ability: f.ability });
        // The last bolt bursts in a ring of light where it landed.
        if d.burst_spell != 0 {
            self.events.push(Event::Missile { caster: f.owner, spell: d.burst_spell, target: Target::Point(p.pos) });
        }
        if let Some(b) = &d.burst {
            let at = self.ground_near(p.pos[0], p.pos[1], p.pos[2]);
            self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: 4.0, arc: 360.0, width: 0.0, secs: b.secs, colour: b.colour.clone() });
        }
    }

    /// Kindle: one of `owner`'s projectiles struck `on`. A burn of theirs on it that kindles (Holy
    /// Fire's) burns hotter, up to its most, and starts over.
    pub(crate) fn kindle(&mut self, owner: Id, on: Id) {
        let now = self.clock;
        let hero = self.players.contains_key(&on);
        let share = if hero { self.content.cc.player_share } else { 1.0 };
        let fans: Vec<(String, f32, f32, f32, f32)> = self
            .content
            .abilities
            .iter()
            .filter(|a| a.kindle > 0.0)
            .filter_map(|a| a.effects.iter().find(|e| e.kind == "burn").map(|e| (a.name.clone(), a.kindle, a.kindle_most, e.amount, e.secs[1])))
            .collect();
        let Some(list) = self.cc.get_mut(&on) else { return };
        let Some(c) = list.iter_mut().find(|c| c.kind == "burn" && c.from == owner && c.until > now) else { return };
        let Some((_, step, most, base, secs)) = fans.into_iter().find(|f| f.0 == c.source) else { return };
        c.amount = (c.amount + step).min(base + most);
        c.until = c.until.max(now + secs * share);
        let n = ((c.amount - base) / step.max(1.0e-3)).round().max(0.0) as u32;
        self.events.push(Event::Auras { id: on });
        self.events.push(Event::PriestLook { player: owner, kind: "kindle", n, on: Some(on), a: 0.0, b: 0.0 });
    }
}
