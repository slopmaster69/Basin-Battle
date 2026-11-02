//! How a creature fights a band of heroes ([`crate::content::AiDef`]): whom it goes for (who
//! hurts it, who's open, who's hurt, who's near, who isn't already beset), a pack spreading over
//! the band as it comes (not all onto whoever drew it) and one with no turn at its hero going for
//! a hero nobody's pressing, pressing an opening at
//! once, reaching for the blow no guard stops against a raised guard, now and then giving ground
//! after a string, working round to a hero's back while it waits its turn, and blows that don't
//! come on a metronome.

use super::mobs::{Foe, MobState};
use super::*;

/// A hero this far off isn't worth turning for.
const LOOK: f32 = 22.0;
/// A raider on its way to a station turns on a hero this near.
const RAIDER_TURN: f32 = 9.0;

impl Survival {
    /// A hero hurt this creature: it remembers (the memory fades).
    pub(crate) fn note_threat(&mut self, mid: Id, by: Id, damage: u32) {
        if !self.players.contains_key(&by) || damage == 0 {
            return;
        }
        let now = self.clock;
        let half = self.content.ai.threat_half.max(0.1);
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        let share = damage as f32 / m.max_health.max(1) as f32;
        match m.threat.iter_mut().find(|t| t.0 == by) {
            Some(t) => {
                t.1 = t.1 * 0.5f32.powf((now - t.2) / half) + share;
                t.2 = now;
            }
            None => m.threat.push((by, share, now)),
        }
    }

    /// What a hero's done to this creature lately (a share of its health).
    pub(crate) fn threat_on(&self, m: &Mob, hero: Id) -> f32 {
        self.threat_of(m, hero)
    }

    fn threat_of(&self, m: &Mob, hero: Id) -> f32 {
        let half = self.content.ai.threat_half.max(0.1);
        m.threat.iter().find(|t| t.0 == hero).map_or(0.0, |t| t.1 * 0.5f32.powf((self.clock - t.2) / half))
    }

    /// A hero with their guard down: healing or working, in a swing's follow-through, drawing a
    /// spell back, reeling or down.
    pub(crate) fn hero_open(&self, hero: Id) -> bool {
        let Some(p) = self.players.get(&hero) else { return false };
        match &p.action {
            Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. } | Action::Charge { .. } | Action::Aim { .. } | Action::Cast { .. } | Action::Channel { .. } => true,
            Action::Stagger { until } => *until > self.clock,
            Action::Swing { t, strike, .. } => *t > strike.windup + strike.active,
            _ => false,
        }
    }

    /// Whether a hero holds their guard up.
    pub(crate) fn hero_guarding(&self, hero: Id) -> bool {
        self.players.get(&hero).is_some_and(|p| matches!(p.action, Action::Guard) || p.guard_held)
    }

    /// Creatures other than `except` on `hero` now (chasing them, winding up, between blows).
    fn on_hero(&self, hero: Id, except: Id) -> usize {
        self.mobs.iter().filter(|(id, m)| **id != except && m.health > 0 && m.state.target() == Some(hero)).count()
    }

    /// Now and then, between strings, a creature looks round for whom it would rather fight: a
    /// hero who's hurt it, one open, one hurt, one near, one not already beset; its foe now
    /// counts for more. A raider on its way to a station turns on a hero who comes near.
    /// Whether it changed its foe.
    pub(crate) fn retarget(&mut self, id: Id) -> bool {
        let now = self.clock;
        let ai = self.content.ai.clone();
        let Some(m) = self.mobs.get(&id) else { return false };
        let MobState::Chase { foe } = m.state else { return false };
        // (At a station, a raider looks every moment: a hero coming near draws it at once.)
        let due = now >= m.retarget_at || matches!(foe, Foe::Obj(_));
        if !due || m.pattern.is_some() || m.hidden || self.content.mobs[m.def].critter {
            return false;
        }
        let pos = m.pos;
        let raider = m.raider;
        self.mobs.get_mut(&id).unwrap().retarget_at = now + ai.retarget_secs.max(0.2);
        let current = match foe {
            Foe::Hero(h) => Some(h),
            Foe::Obj(_) => None,
        };
        // A raider at a station: only a hero coming near draws it off.
        let look = if current.is_none() {
            if !raider {
                return false;
            }
            RAIDER_TURN
        } else {
            LOOK
        };
        let m = &self.mobs[&id];
        let mut best: Option<(Id, f32)> = None;
        for (h, p) in &self.players {
            if p.health == 0 || self.unseen_by(id, *h) {
                continue;
            }
            let d = dist2d(p.pos, pos);
            if d > look || (p.pos[2] - pos[2]).abs() > 8.0 {
                continue;
            }
            // Out of a leash a raider ignores; a pack's creature only within its own reach.
            if !raider && self.leashed(m.home, pos, p.pos, self.leash_of(m.def)) {
                continue;
            }
            let mut score = ai.threat * self.threat_of(m, *h) - ai.distance * d / 10.0;
            if self.hero_open(*h) {
                score += ai.opening;
            }
            score += ai.wounded * (1.0 - p.health as f32 / p.max_health.max(1) as f32);
            score -= ai.crowd * self.on_hero(*h, id) as f32;
            if Some(*h) == current {
                score += ai.stick;
            }
            if best.is_none_or(|b| score > b.1) {
                best = Some((*h, score));
            }
        }
        let Some((h, _)) = best else { return false };
        if Some(h) == current {
            return false;
        }
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = MobState::Chase { foe: Foe::Hero(h) };
        m.think = 0.0;
        true
    }

    /// Whom a creature brought into a fight by its pack goes for: the hero in its fight with the
    /// fewest creatures on them already, the nearer first, the one who drew it a little ahead
    /// (`crowd` for each creature on a hero, `distance` for every 10 yards, `stick` for the first).
    /// Alone, or with nobody else in its fight, the one who drew it.
    pub(crate) fn spread_foe(&self, id: Id, first: Foe) -> Foe {
        let Foe::Hero(drew) = first else { return first };
        let Some(m) = self.mobs.get(&id) else { return first };
        if m.raider || self.content.mobs[m.def].critter {
            return first;
        }
        let ai = &self.content.ai;
        let mut best: Option<(Id, f32)> = None;
        for (h, p) in &self.players {
            if !self.in_fight_of(m, *h) || self.unseen_by(id, *h) {
                continue;
            }
            let mut cost = ai.crowd * self.on_hero(*h, id) as f32 + ai.distance * dist2d(p.pos, m.pos) / 10.0;
            if *h == drew {
                cost -= ai.stick;
            }
            if best.is_none_or(|b| cost < b.1) {
                best = Some((*h, cost));
            }
        }
        best.map_or(first, |(h, _)| Foe::Hero(h))
    }

    /// A creature whose hero has two pressing them already: it turns on a hero in its fight that
    /// fewer are on and nobody's pressing (the nearest such), at most once a look
    /// (`retarget_secs`), so a band's foes spread over the band. Whether it turned.
    pub(crate) fn turn_to_free(&mut self, id: Id, hero: Id) -> bool {
        let now = self.clock;
        let Some(m) = self.mobs.get(&id) else { return false };
        if now < m.retarget_at || m.raider || m.pattern.is_some() || self.content.mobs[m.def].boss {
            return false;
        }
        let here = self.on_hero(hero, id);
        let mut best: Option<(Id, f32)> = None;
        for (h, p) in &self.players {
            if *h == hero || !self.in_fight_of(m, *h) || self.unseen_by(id, *h) {
                continue;
            }
            let d = dist2d(p.pos, m.pos);
            if d > mobs::FIGHT_RANGE * 0.6 || self.attackers_on(*h, id) >= mobs::MAX_ATTACKERS || self.on_hero(*h, id) >= here {
                continue;
            }
            if best.is_none_or(|b| d < b.1) {
                best = Some((*h, d));
            }
        }
        let secs = self.content.ai.retarget_secs.max(0.2);
        let m = self.mobs.get_mut(&id).unwrap();
        m.retarget_at = now + secs;
        let Some((h, _)) = best else { return false };
        m.state = MobState::Chase { foe: Foe::Hero(h) };
        m.think = 0.0;
        m.path.clear();
        true
    }

    /// Whether a creature presses an opening on `hero` now: open, near, and its cooldown nearly
    /// done.
    pub(crate) fn presses(&self, id: Id, hero: Id, dist: f32) -> bool {
        let Some(m) = self.mobs.get(&id) else { return false };
        dist <= mobs::PRESS_RANGE && self.clock + self.content.ai.press_early >= m.ready_at && m.stagger_until <= self.clock && self.hero_open(hero)
    }

    /// A string done: its cooldown, give or take a little.
    pub(crate) fn string_cooldown(&mut self, secs: f32) -> f32 {
        let j = self.content.ai.jitter.clamp(0.0, 0.9);
        if j <= 0.0 {
            return secs;
        }
        secs * (1.0 + (self.rng.gen::<f32>() * 2.0 - 1.0) * j)
    }

    /// A string done: now and then it gives ground, facing its foe (a beast circling back, a
    /// fighter stepping out of reach to make you come to it).
    pub(crate) fn maybe_give_ground(&mut self, id: Id, hero: Id) {
        let ai = self.content.ai.clone();
        let Some(m) = self.mobs.get(&id) else { return };
        let d = &self.content.mobs[m.def];
        if ai.retreat <= 0.0 || d.boss || d.attack.ranged || m.raider && self.is_siege_foe(id) && self.rng.gen::<f32>() < 0.5 {
            return;
        }
        if self.rng.gen::<f32>() >= ai.retreat {
            return;
        }
        let Some(to) = self.players.get(&hero).map(|p| p.pos) else { return };
        let pos = m.pos;
        let away = facing(to, pos) + (self.rng.gen::<f32>() - 0.5) * 0.9;
        let goal = self.ground_near(pos[0] + away.cos() * ai.retreat_yards, pos[1] + away.sin() * ai.retreat_yards, pos[2]);
        let speed = d.walk_speed.max(2.5) * 1.4;
        self.sidle_to(id, goal, speed, hero);
        let m = self.mobs.get_mut(&id).unwrap();
        m.ready_at = m.ready_at.max(self.clock + ai.retreat_yards / speed);
    }

    /// Which way round a waiting creature steps: toward the hero's back (the shorter way), or its
    /// own way once it's there.
    pub(crate) fn flank_way(&self, id: Id, hero: Id) -> Option<f32> {
        if !self.content.ai.flank {
            return None;
        }
        let m = self.mobs.get(&id)?;
        let p = self.players.get(&hero)?;
        // Where it stands round them, against where their back is.
        let at = facing(p.pos, m.pos);
        let back = p.facing + std::f32::consts::PI;
        let off = (at - back + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        if off.abs() < 0.9 {
            return None;
        }
        Some(if off > 0.0 { -1.0 } else { 1.0 })
    }
}
