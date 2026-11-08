//! How a creature fights a band of heroes ([`crate::content::AiDef`]): whom it goes for (who
//! hurts it, who's open, who's hurt, who's near, who isn't already beset), a pack spreading over
//! the band as it comes (not all onto whoever drew it) and one with no turn at its hero going for
//! a hero nobody's pressing, pressing an opening at
//! once, reaching for the blow no guard stops against a raised guard, now and then giving ground
//! after a string, working round to a hero's back while it waits its turn, and blows that don't
//! come on a metronome. A boss is harder to learn still: it seldom begins the same string twice
//! running, holds a blow a beat now and then, stops a long string short or goes straight into
//! another, and backs off to where its charges, lunges and throws can come from.

use super::mobs::{roll01, Foe, MobState, Role};
use super::*;

/// A hero this far off isn't worth turning for.
const LOOK: f32 = 22.0;
/// A raider on its way to a station turns on a hero this near.
const RAIDER_TURN: f32 = 9.0;
/// A boss making space backs off this much quicker than it walks.
const SPACE_PACE: f32 = 1.6;
/// It backs off no farther than this from where it stands (yards), and not for less than a yard.
const SPACE_MOST: f32 = 12.0;
/// Backed off, it waits this long at most for its blow from afar before it closes in again.
const SPACE_WAIT: f32 = 2.0;
/// An eager creature's pace between strings, times its own (`all_in`).
const EAGER_PACE: f32 = 0.85;
/// A guard waits this far off its hero, toward the caster it covers.
const GUARD_OFF: f32 = 2.6;

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
        // (A right mouse held on a weapon with no guard, a Drain, guards nothing.)
        self.players.get(&hero).is_some_and(|p| matches!(p.action, Action::Guard) || (p.guard_held && !self.weapon_of(p).has_alt() && self.off_weapon_of(p).is_none()))
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
    /// fewer are on and nobody's pressing (the nearest such), so a band's foes spread over the
    /// band. (Only to one with fewer on them: it doesn't turn back and forth.) Whether it turned.
    pub(crate) fn turn_to_free(&mut self, id: Id, hero: Id) -> bool {
        let now = self.clock;
        let Some(m) = self.mobs.get(&id) else { return false };
        if m.raider || m.pattern.is_some() || self.content.mobs[m.def].boss {
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
        let Some((h, _)) = best else { return false };
        let secs = self.content.ai.retarget_secs.max(0.2);
        let m = self.mobs.get_mut(&id).unwrap();
        m.retarget_at = now + secs;
        m.state = MobState::Chase { foe: Foe::Hero(h) };
        m.think = 0.0;
        m.path.clear();
        true
    }

    /// Creature `id`'s temperament, if it has one.
    pub(crate) fn temper(&self, id: Id) -> Option<&crate::content::TemperDef> {
        let m = self.mobs.get(&id)?;
        let key = &self.content.mobs[m.def].temper;
        if key.is_empty() {
            return None;
        }
        self.content.tempers.iter().find(|t| &t.key == key)
    }

    /// Its cooldown between strings, as its temperament paces it, its own quirk, and its part in
    /// its fight's plan.
    pub(crate) fn pace(&self, id: Id) -> f32 {
        let own = self.mobs.get(&id).map_or(1.0, |m| m.quirk_pace * if m.role == Role::Eager { EAGER_PACE } else { 1.0 });
        self.temper(id).map_or(1.0, |t| t.pace) * own
    }

    /// Whether creature `id` stands behind `hero` (outside the half of the world they face).
    pub(crate) fn behind(&self, id: Id, hero: Id) -> bool {
        let (Some(m), Some(p)) = (self.mobs.get(&id), self.players.get(&hero)) else { return false };
        let to = facing(p.pos, m.pos);
        let off = (to - p.facing + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        off.abs() > std::f32::consts::FRAC_PI_2 * 1.1
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
        // Its temperament's own: a boss too, when it says so.
        let (chance, yards, own) = match self.temper(id) {
            Some(t) => (t.retreat.unwrap_or(ai.retreat), t.retreat_yards.unwrap_or(ai.retreat_yards), t.retreat.is_some()),
            None => (ai.retreat, ai.retreat_yards, false),
        };
        let Some(m) = self.mobs.get(&id) else { return };
        let chance = chance * m.quirk_retreat;
        let d = &self.content.mobs[m.def];
        if chance <= 0.0 || (d.boss && !own) || d.attack.ranged || m.raider && self.is_siege_foe(id) && self.rng.gen::<f32>() < 0.5 {
            return;
        }
        if self.rng.gen::<f32>() >= chance {
            return;
        }
        let Some(to) = self.players.get(&hero).map(|p| p.pos) else { return };
        let pos = m.pos;
        let away = facing(to, pos) + (self.rng.gen::<f32>() - 0.5) * 0.9;
        let goal = self.ground_near(pos[0] + away.cos() * yards, pos[1] + away.sin() * yards, pos[2]);
        let speed = d.walk_speed.max(2.5) * 1.4;
        self.sidle_to(id, goal, speed, hero);
        let m = self.mobs.get_mut(&id).unwrap();
        m.ready_at = m.ready_at.max(self.clock + yards / speed);
    }

    /// Which way round a waiting creature steps: toward the hero's back (the shorter way), or its
    /// own way once it's there.
    pub(crate) fn flank_way(&self, id: Id, hero: Id) -> Option<f32> {
        let m = self.mobs.get(&id)?;
        // Its fight's plan first: a flanker goes round, a guard holds its line.
        let flanks = match m.role {
            Role::Flanker => true,
            Role::Guard => false,
            _ => self.temper(id).and_then(|t| t.flank).unwrap_or(self.content.ai.flank),
        };
        if !flanks {
            return None;
        }
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

    /// A fight begins (the creatures drawn in together, `ids`): if two or more of them, not a
    /// boss's, it takes one of the plans that fit who's in it, and each its part. Rolled from the
    /// first one's id and the hour, not the world's dice.
    pub(crate) fn plan_fight(&mut self, ids: &[Id]) {
        let now = self.clock;
        let mut fight: Vec<Id> = ids
            .iter()
            .copied()
            .filter(|id| self.mobs.get(id).is_some_and(|m| m.health > 0 && !m.friendly && matches!(m.state.foe(), Some(Foe::Hero(_)))))
            .collect();
        fight.sort_unstable();
        for id in &fight {
            let m = self.mobs.get_mut(id).unwrap();
            m.role = Role::None;
            m.role_at = now;
        }
        let boss = fight.iter().any(|id| self.content.mobs[self.mobs[id].def].boss);
        if fight.len() < 2 || boss {
            return;
        }
        let ranged = |s: &Self, id: &Id| s.content.mobs[s.mobs[id].def].attack.ranged;
        let melee: Vec<Id> = fight.iter().copied().filter(|id| !ranged(self, id) && !self.content.mobs[self.mobs[id].def].patterns.is_empty()).collect();
        let shooters = fight.iter().filter(|id| ranged(self, id)).count();
        let fits: Vec<(String, f32)> = self
            .content
            .plans
            .iter()
            .filter(|p| p.weight > 0.0 && melee.len() >= p.min_melee.max(1) && (!p.ranged || shooters > 0))
            .map(|p| (p.key.clone(), p.weight))
            .collect();
        let total: f32 = fits.iter().map(|f| f.1).sum();
        if total <= 0.0 {
            return;
        }
        let mut roll = roll01(fight[0] ^ now.to_bits(), 7) * total;
        let mut plan = fits[fits.len() - 1].0.clone();
        for (k, w) in &fits {
            if roll < *w {
                plan = k.clone();
                break;
            }
            roll -= w;
        }
        // Who can take a part: a coward or a commander keeps its own way, and a bruiser plods
        // straight in (it never goes round, nor hangs back).
        let temper = |s: &Self, id: &Id| s.temper(*id).map(|t| t.key.clone()).unwrap_or_default();
        let free: Vec<Id> = melee.iter().copied().filter(|id| !matches!(temper(self, id).as_str(), "commander" | "coward")).collect();
        let roams: Vec<Id> = free.iter().copied().filter(|id| temper(self, id) != "bruiser").collect();
        let first = melee.first().copied();
        let mut parts: Vec<(Id, Role)> = Vec::new();
        match plan.as_str() {
            "all_in" => parts.extend(melee.iter().map(|id| (*id, Role::Eager))),
            // One holds the front (the first of them), the next that can go round goes round.
            "pincer" => {
                if let Some(id) = roams.iter().find(|id| Some(**id) != first && temper(self, id) != "backstabber") {
                    parts.push((*id, Role::Flanker));
                }
            }
            "screen" => {
                if let Some(id) = free.iter().find(|id| temper(self, id) != "backstabber") {
                    parts.push((*id, Role::Guard));
                }
            }
            // The last that can hang back does, while the rest fight.
            "bait" => {
                if let Some(id) = roams.iter().rev().find(|id| Some(**id) != first) {
                    parts.push((*id, Role::Reserve));
                }
            }
            "waves" => {
                let n = melee.len() / 2;
                parts.extend(roams.iter().rev().filter(|id| Some(**id) != first).take(n).map(|id| (*id, Role::Reserve)));
            }
            _ => {}
        }
        for (id, role) in parts {
            if let Some(m) = self.mobs.get_mut(&id) {
                m.role = role;
            }
        }
        let names: Vec<String> = fight
            .iter()
            .map(|id| {
                let m = &self.mobs[id];
                let name = &self.content.mobs[m.def].name;
                if m.role == Role::None { name.clone() } else { format!("{name} ({})", format!("{:?}", m.role).to_lowercase()) }
            })
            .collect();
        self.events.push(Event::Log(format!("A fight begins, {plan}: {}", names.join(", "))));
    }

    /// A held-back creature (`Role::Reserve`) comes in: when nobody else of its fight is on its
    /// hero, one of them is hurt past half, its hero opens up near it, or it has waited long enough.
    pub(crate) fn reserve_comes(&self, id: Id, hero: Id, dist: f32) -> bool {
        let Some(m) = self.mobs.get(&id) else { return true };
        if self.clock - m.role_at >= mobs::RESERVE_MOST || self.presses(id, hero, dist) {
            return true;
        }
        let Some(hp) = self.players.get(&hero).map(|p| p.pos) else { return true };
        let mut others = false;
        for (oid, o) in &self.mobs {
            if *oid == id || o.state.target() != Some(hero) || dist2d(o.pos, hp) > 20.0 {
                continue;
            }
            if o.health > 0 && (o.health as f32) < 0.5 * o.max_health as f32 {
                return true;
            }
            others |= o.health > 0 && o.role != Role::Reserve;
        }
        !others
    }

    /// Where a guard (`Role::Guard`) waits: between its hero and the nearest caster or shooter of
    /// its fight, a little off the hero.
    pub(crate) fn guard_spot(&self, id: Id, hero: Id, to: Vec3) -> Option<Vec3> {
        let m = self.mobs.get(&id)?;
        let shooter = self
            .mobs
            .iter()
            .filter(|(oid, o)| **oid != id && o.health > 0 && o.state.target() == Some(hero) && self.content.mobs[o.def].attack.ranged)
            .min_by(|a, b| dist2d(a.1.pos, m.pos).total_cmp(&dist2d(b.1.pos, m.pos)))?;
        let way = facing(to, shooter.1.pos);
        Some(self.ground_near(to[0] + way.cos() * GUARD_OFF, to[1] + way.sin() * GUARD_OFF, to[2]))
    }

    /// A creature begins one of its strings: remembered (a boss seldom begins the same one next).
    /// With `vary`, a boss's string up close may stop one short or run one longer, and may have
    /// one of its blows held a beat.
    pub(crate) fn begin_pattern(&mut self, id: Id, k: usize, vary: bool) {
        let ai = self.content.ai.clone();
        let Some(m) = self.mobs.get(&id) else { return };
        let d = &self.content.mobs[m.def];
        let close = d.patterns.get(k).filter(|p| {
            vary && d.boss && !d.attack.ranged && p.weight > 0.0 && p.min_range < mobs::SPACE_FROM && p.steps.iter().all(|s| s.ranged != Some(true))
        });
        let n = close.map_or(0, |p| p.steps.len());
        let (mut len, mut held) = (0, None);
        if n > 0 {
            let roll = self.rng.gen::<f32>();
            if n >= 3 && roll < ai.cut {
                len = n - 1;
            } else if n >= 2 && roll < ai.cut + ai.extend {
                len = n + 1;
            }
            if ai.hold > 0.0 && ai.hold_secs > 0.0 && self.rng.gen::<f32>() < ai.hold {
                let blows = if len > 0 { len } else { n };
                held = Some(self.rng.gen_range(0..blows));
            }
        }
        let m = self.mobs.get_mut(&id).unwrap();
        m.pattern = Some((k, 0));
        m.last_pattern = Some(k);
        m.string_len = len;
        m.held_step = held;
        m.space = None;
    }

    /// The blow a boss winds up: how much longer it holds it (its string's held blow, a beat).
    pub(crate) fn hold_beat(&mut self, id: Id) -> f32 {
        let secs = self.content.ai.hold_secs;
        let Some(m) = self.mobs.get(&id) else { return 0.0 };
        match (m.pattern, m.held_step) {
            // Never so slight a delay it can't be seen, nor longer than set.
            (Some((_, s)), Some(h)) if s == h => secs * (0.4 + 0.6 * self.rng.gen::<f32>()),
            _ => 0.0,
        }
    }

    /// A boss's string up close done: now and then it goes straight into another (its rest cut
    /// to nothing), not past three close strings in a row.
    pub(crate) fn maybe_chain(&mut self, id: Id) {
        let chain = self.content.ai.chain;
        let Some(m) = self.mobs.get(&id) else { return };
        let d = &self.content.mobs[m.def];
        if !d.boss || d.attack.ranged || m.close_run == 0 || m.close_run >= 3 || chain <= 0.0 {
            return;
        }
        if self.rng.gen::<f32>() >= chain {
            return;
        }
        let now = self.clock;
        let m = self.mobs.get_mut(&id).unwrap();
        m.ready_at = m.ready_at.min(now + 0.15);
    }

    /// A boss's string up close done: now and then it backs off, facing its foe, to where one of
    /// its blows from afar (a charge, a lunge, a throw) can come from, the likelier the more
    /// close strings it has done in a row, and less with a band on it. Whether it did.
    pub(crate) fn maybe_make_space(&mut self, id: Id, hero: Id) -> bool {
        let ai = self.content.ai.clone();
        let now = self.clock;
        let Some(m) = self.mobs.get(&id) else { return false };
        let d = self.content.mobs[m.def].clone();
        // (After a string up close: never straight after one from afar.)
        if !d.boss || d.attack.ranged || d.patterns.is_empty() || ai.space <= 0.0 || m.stagger_until > now || m.close_run == 0 {
            return false;
        }
        let Some(at) = self.players.get(&hero).filter(|p| p.health > 0).map(|p| p.pos) else { return false };
        let (pos, home, health, max) = (m.pos, m.home, m.health, m.max_health);
        let dist = dist2d(pos, at);
        // Its blows from afar it has now (by its health), each with the distance it wants.
        let far: Vec<(usize, f32, f32, f32)> = d
            .patterns
            .iter()
            .enumerate()
            .filter(|(_, p)| p.weight > 0.0 && p.min_range >= mobs::SPACE_FROM && health as f32 <= p.below * max as f32)
            .filter_map(|(k, p)| {
                let first = p.steps.first()?;
                let reach = first.range.unwrap_or(d.attack.range) + 0.1 + first.lunge;
                let most = if p.max_range > 0.0 { p.max_range } else { reach };
                (most > p.min_range + 0.5 && dist < p.min_range).then_some((k, p.weight, p.min_range, most))
            })
            .collect();
        if far.is_empty() {
            return false;
        }
        // (Its first close string counts nothing more; each one after it, `space_streak`.)
        let run = self.mobs[&id].close_run.saturating_sub(1) as f32;
        let band = if self.heroes_in_fight(id) > 1 { ai.space_band } else { 1.0 };
        let chance = (ai.space + ai.space_streak * run).min(ai.space_most) * band;
        if self.rng.gen::<f32>() >= chance {
            return false;
        }
        // Which of them, weighted.
        let total: f32 = far.iter().map(|f| f.1).sum();
        let mut roll = self.rng.gen::<f32>() * total;
        let mut pick = far[far.len() - 1];
        for f in &far {
            if roll < f.1 {
                pick = *f;
                break;
            }
            roll -= f.1;
        }
        let (k, _, lo, hi) = pick;
        // A little past the nearest it can use it from, so a hero's step after it doesn't spoil it.
        let want = (lo + 1.0 + self.rng.gen::<f32>() * 2.5).min(hi - 0.5).min(dist + SPACE_MOST);
        // Straight away from its foe, else a little to either side (a wall behind it), inside its
        // leash.
        let leash = self.leash_of(self.mobs[&id].def) * 0.8;
        let away = facing(at, pos);
        let goal = [0.0f32, 0.5, -0.5, 1.0, -1.0].iter().find_map(|off| {
            let b = away + off;
            let to = self.ground_near(at[0] + b.cos() * want, at[1] + b.sin() * want, pos[2]);
            let ok = dist2d(to, pos) >= 1.0
                && dist2d(to, home) <= leash
                && self.land.walk_clear(pos, to)
                && !self.land.blocked_near(to[0], to[1], to[2]);
            ok.then_some(to)
        });
        let Some(to) = goal else { return false };
        let speed = d.walk_speed.max(2.5) * SPACE_PACE;
        self.sidle_to(id, to, speed, hero);
        if self.mobs[&id].path.is_empty() {
            return false;
        }
        let back = dist2d(pos, to) / (speed * self.speed_share(id)).max(0.5);
        let m = self.mobs.get_mut(&id).unwrap();
        // Backing off is its rest: its blow comes as it gets there.
        m.ready_at = now + back + 0.2;
        m.space = Some((now + back + SPACE_WAIT, k));
        true
    }
}
