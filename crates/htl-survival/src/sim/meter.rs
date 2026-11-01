//! The damage meter: what each hero has dealt, healed and taken, in their current fight and since
//! they last reset it, and by what (a card, Melee, a burn).
//!
//! Only what landed counts: a blow's overkill, a heal past full health and ice that soaks a blow
//! don't. A fight begins with the first blow dealt or taken after a lull of `player.combat_secs`
//! and lasts until the next lull; a heal carries one on (or counts towards a friend's), but a heal
//! out of any fight doesn't begin one. Not saved: a restart starts every meter over.

use super::{dist2d, Id, Survival};

/// How many sources a hero's own meter keeps (the rest fold into the last of them).
const MOST_SOURCES: usize = 24;
/// Heroes fighting at the same time this near count in each other's fight.
const FIGHT_NEAR: f32 = 100.0;

/// A hero's meter.
#[derive(Debug, Clone, Default)]
pub struct Meter {
    /// Since the hero last reset it: seconds in fights, the totals, and each source's damage and
    /// healing.
    pub active: f32,
    pub dealt: u64,
    pub healed: u64,
    pub taken: u64,
    pub by: Vec<(String, u64, u64)>,
    /// When last something counted (fights' seconds add up from it).
    last: Option<f32>,
    /// The current fight, or the last one.
    pub fight: Fight,
}

/// One fight's numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Fight {
    /// There has been one.
    pub seen: bool,
    pub start: f32,
    pub last: f32,
    pub dealt: u64,
    pub healed: u64,
    pub taken: u64,
}

impl Fight {
    /// How long it has lasted (a second at least, for a per-second figure).
    pub fn secs(&self) -> f32 {
        (self.last - self.start).max(1.0)
    }
}

/// A hero's own meter as the add-on shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct OwnMeter {
    pub secs: f32,
    pub dealt: u64,
    pub healed: u64,
    pub taken: u64,
    /// By source, the most first: its name, damage, healing.
    pub rows: Vec<(String, u64, u64)>,
}

/// One hero's line on the fight's meter.
#[derive(Debug, Clone, PartialEq)]
pub struct FightRow {
    pub id: Id,
    pub name: String,
    pub class: String,
    pub dealt: u64,
    pub healed: u64,
    pub taken: u64,
}

/// The fight a hero is in (or was last in): how long, whether it's still on, and everyone in it.
#[derive(Debug, Clone, PartialEq)]
pub struct FightMeter {
    pub secs: f32,
    pub live: bool,
    pub rows: Vec<FightRow>,
}

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Dealt,
    Healed,
    Taken,
}

impl Survival {
    /// A lull this long ends a fight.
    fn fight_gap(&self) -> f32 {
        self.content.player.combat_secs.max(1.0)
    }

    /// Count `amount` on hero `id`'s meter. `begins`: it may begin a fight (a blow; a heal only
    /// carries one on).
    fn count(&mut self, id: Id, kind: Kind, amount: u32, label: &str, begins: bool) {
        if amount == 0 || !self.players.contains_key(&id) {
            return;
        }
        let now = self.clock;
        let gap = self.fight_gap();
        let m = self.meters.entry(id).or_default();
        let fighting = m.fight.seen && now - m.fight.last <= gap;
        if !fighting && begins {
            m.fight = Fight { seen: true, start: now, last: now, ..Fight::default() };
        }
        let fighting = fighting || begins;
        if let Some(last) = m.last {
            if now - last <= gap {
                m.active += now - last;
            }
        }
        m.last = Some(now);
        let n = u64::from(amount);
        match kind {
            Kind::Dealt => m.dealt += n,
            Kind::Healed => m.healed += n,
            Kind::Taken => m.taken += n,
        }
        if fighting {
            m.fight.last = now;
            match kind {
                Kind::Dealt => m.fight.dealt += n,
                Kind::Healed => m.fight.healed += n,
                Kind::Taken => m.fight.taken += n,
            }
        }
        if kind == Kind::Taken {
            return;
        }
        let i = match m.by.iter().position(|(l, _, _)| l == label) {
            Some(i) => i,
            None if m.by.len() < MOST_SOURCES => {
                m.by.push((label.to_string(), 0, 0));
                m.by.len() - 1
            }
            None => m.by.len() - 1,
        };
        if kind == Kind::Dealt {
            m.by[i].1 += n;
        } else {
            m.by[i].2 += n;
        }
    }

    /// A hero's blow (a card, a burn, their weapon: `label`) took `amount` from someone else.
    pub(crate) fn meter_dealt(&mut self, by: Id, victim: Id, amount: u32, label: &str) {
        if by != victim {
            self.count(by, Kind::Dealt, amount, label, true);
        }
    }

    /// A hero lost `amount` of health.
    pub(crate) fn meter_taken(&mut self, id: Id, amount: u32) {
        self.count(id, Kind::Taken, amount, "", true);
    }

    /// A hero's card mended `target` (themselves too) by `amount`: it counts in the healer's
    /// fight, or carries a friend's on.
    pub(crate) fn meter_healed(&mut self, healer: Id, target: Id, amount: u32, label: &str) {
        let gap = self.fight_gap();
        let now = self.clock;
        let in_fight = |s: &Self, h: Id| s.meters.get(&h).is_some_and(|m| m.fight.seen && now - m.fight.last <= gap);
        let begins = in_fight(self, target) && !in_fight(self, healer);
        self.count(healer, Kind::Healed, amount, label, begins);
    }

    /// Heal a hero, counting it on `healer`'s meter as `label`.
    pub(crate) fn heal_hero_by(&mut self, id: Id, amount: u32, healer: Id, label: &str) {
        let healed = self.heal_hero(id, amount);
        self.meter_healed(healer, id, healed, label);
    }

    /// Hurt a hero, counting what it took on `by`'s meter as `label` (when `by` is a hero).
    pub(crate) fn hurt_player_by(&mut self, id: Id, damage: u32, by: Id, label: &str) {
        let before = self.players.get(&id).map_or(0, |p| p.health);
        self.hurt_player(id, damage);
        let after = self.players.get(&id).map_or(0, |p| p.health);
        self.meter_dealt(by, id, before.saturating_sub(after), label);
    }

    /// A hero's own meter since they last reset it.
    pub fn own_meter(&self, id: Id) -> Option<OwnMeter> {
        self.players.get(&id)?;
        let m = self.meters.get(&id).cloned().unwrap_or_default();
        let mut rows = m.by;
        rows.sort_by(|a, b| (b.1 + b.2).cmp(&(a.1 + a.2)).then_with(|| a.0.cmp(&b.0)));
        Some(OwnMeter { secs: m.active.max(1.0), dealt: m.dealt, healed: m.healed, taken: m.taken, rows })
    }

    /// Start a hero's own meter over (their fight's numbers stay).
    pub fn reset_meter(&mut self, id: Id) {
        if let Some(m) = self.meters.get_mut(&id) {
            let fight = std::mem::take(&mut m.fight);
            *m = Meter { fight, ..Meter::default() };
        }
    }

    /// The fight a hero is in, or was last in: them, and every hero near them fighting at the same
    /// time, the most damage first. `None` before their first fight.
    pub fn fight_meter(&self, id: Id) -> Option<FightMeter> {
        let me = self.players.get(&id)?;
        let mine = self.meters.get(&id).map(|m| m.fight.clone()).filter(|f| f.seen)?;
        let gap = self.fight_gap();
        let mut rows: Vec<FightRow> = self
            .players
            .iter()
            .filter_map(|(h, p)| {
                let f = &self.meters.get(h)?.fight;
                let together = *h == id || (f.seen && f.start <= mine.last + gap && f.last + gap >= mine.start && dist2d(p.pos, me.pos) <= FIGHT_NEAR);
                together.then(|| FightRow { id: *h, name: p.name.clone(), class: p.class.clone(), dealt: f.dealt, healed: f.healed, taken: f.taken })
            })
            .collect();
        rows.sort_by(|a, b| b.dealt.cmp(&a.dealt).then_with(|| b.healed.cmp(&a.healed)).then_with(|| a.name.cmp(&b.name)));
        // While it's on, its clock runs; once over, it stopped at its last blow.
        let live = self.clock - mine.last <= gap;
        let secs = if live { (self.clock - mine.start).max(1.0) } else { mine.secs() };
        Some(FightMeter { secs, live, rows })
    }
}
