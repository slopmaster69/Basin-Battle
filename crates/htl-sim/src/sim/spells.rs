//! Class spells in play: what a player holds (cards and the spells talents teach), what their
//! upgrades and talents make of them, casting (by hand from the bar, or on their own), where
//! the hits land and everything a hit can leave behind; buffs, rage, the hunter's pet and traps.

use rand::Rng;

use super::cards::{HitInfo, Zone};
use super::enemies::Via;
use super::*;
use crate::kit::{Nums, SpellDef, SpellKind};

/// Upgrades a spell can take (each a card): rank 1 plus four.
pub(super) const MAX_PERKS: usize = 4;
/// Every upgrade also adds this much power.
pub(super) const PERK_POWER: f32 = 0.1;
/// Most stacks of Fire Vulnerability, Winter's Chill, Shadow Vulnerability and Sunder Armor.
pub(super) const MAX_VULN: u8 = 5;
const VULN_SECS: f32 = 15.0;
/// What each stack does.
pub(super) const FIRE_VULN: f32 = 0.04;
pub(super) const SHADOW_VULN: f32 = 0.04;
pub(super) const SUNDER: f32 = 0.05;
pub(super) const WINTERS_CHILL: f32 = 0.02;
/// Seconds an ally of this radius counts as "near" for heals, team buffs and mends.
const ALLY_RANGE: f32 = 30.0;
/// The window Revenge opens after a blow.
const REVENGE_SECS: f32 = 5.0;
/// Rage numbers: per second of swing, per share of max health taken, decay a second.
const RAGE_PER_SWING_SEC: f32 = 4.5;
const RAGE_FROM_DAMAGE: f32 = 75.0;
const RAGE_DECAY: f32 = 3.0;
const MAX_RAGE: f32 = 100.0;

/// A spell a player holds.
/// A spell with a cast time on its way.
#[derive(Debug, Clone)]
pub struct CastJob {
    /// The spell (its index in the class kit).
    pub spell: usize,
    pub done_at: f32,
    /// Where the caster stood: stepping away breaks the cast.
    pub from: Vec3,
    /// The spell id the client shows.
    pub icon: u32,
    /// A channel: it has already gone off, and the job only holds the caster still.
    pub channel: bool,
}

#[derive(Debug, Clone)]
pub(super) struct OwnedSpell {
    /// Index into the class kit's spells.
    pub spell: usize,
    /// The upgrades taken, indices into its perks.
    pub perks: Vec<usize>,
    /// Taught by a talent, not a card.
    pub talent: bool,
    /// Seconds (at haste 1) until it can be cast again.
    pub cooldown: f32,
    /// Its numbers, with upgrades, talents and gear.
    pub nums: Nums,
    /// A toggle that is on.
    pub on: bool,
    /// The key it sits on, 0-5 (1, 2, 3, 4, Q, E): spells can be moved to any key, empty or not.
    pub key: u8,
}

/// Timed effects on a player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BuffKind {
    Clearcasting,
    PresenceOfMind,
    InnerFocus,
    ArcanePower,
    Combustion,
    DeathWish,
    Enrage,
    Flurry,
    BloodCraze,
    SpiritTap,
    QuickShots,
    RapidFire,
    LastStand,
    ShieldWall,
    Deterrence,
    PowerInfusion,
    BestialWrath,
    SweepingStrikes,
    FocusedCasting,
    Inspiration,
    InnerFire,
    Bloodrage,
    FuriousHowl,
    Accelerated,
    Warded,
    PetFury,
    PetWard,
    PetFrenzy,
}

impl BuffKind {
    /// The spell its icon comes from on the buff bar.
    pub fn aura(self) -> u32 {
        match self {
            BuffKind::Clearcasting => 12536,
            BuffKind::PresenceOfMind => 12043,
            BuffKind::InnerFocus => 14751,
            BuffKind::ArcanePower => 12042,
            BuffKind::Combustion => 11129,
            BuffKind::DeathWish => 12328,
            BuffKind::Enrage => 12880,
            BuffKind::Flurry => 12966,
            BuffKind::BloodCraze => 16488,
            BuffKind::SpiritTap => 15271,
            BuffKind::QuickShots => 6150,
            BuffKind::RapidFire => 3045,
            BuffKind::LastStand => 12975,
            BuffKind::ShieldWall => 871,
            BuffKind::Deterrence => 19263,
            BuffKind::PowerInfusion => 10060,
            BuffKind::BestialWrath => 19574,
            BuffKind::SweepingStrikes => 12292,
            BuffKind::FocusedCasting => 14743,
            BuffKind::Inspiration => 14893,
            BuffKind::InnerFire => 588,
            BuffKind::Bloodrage => 2687,
            BuffKind::FuriousHowl => 24597,
            BuffKind::Accelerated => 2983,
            BuffKind::Warded => 6346,
            BuffKind::PetFury => 19574,
            BuffKind::PetWard => 19263,
            BuffKind::PetFrenzy => 19621,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Buff {
    pub kind: BuffKind,
    pub until: f32,
    pub stacks: u32,
    pub value: f32,
    pub value2: f32,
}

/// A heal over time on a player.
#[derive(Debug, Clone)]
pub(super) struct Hot {
    pub healer: EntityId,
    pub spell: u32,
    pub per_tick: f32,
    pub ticks: u32,
    pub next: f32,
}

/// A stacking debuff on an enemy.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct Stack {
    pub n: u8,
    pub until: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Vuln {
    Fire,
    Frost,
    Shadow,
    Sunder,
}

/// Hunter's Mark on an enemy.
#[derive(Debug, Clone, Copy)]
pub(super) struct Mark {
    pub owner: EntityId,
    pub bonus: f32,
    pub crit: f32,
    pub pet_bonus: f32,
    pub jump: bool,
    pub reset: bool,
}

/// Debuffs on an enemy beyond slows, roots and damage over time.
#[derive(Debug, Clone, Default)]
pub(super) struct Afflictions {
    pub frozen_until: f32,
    pub stun_until: f32,
    pub fear_until: f32,
    pub fear_from: Vec3,
    pub vulns: [Stack; 4],
    pub mark: Option<Mark>,
    pub demoralize: f32,
    pub demoralize_until: f32,
    pub taunt: Option<(EntityId, f32)>,
    /// A warrior charged it: defenders nearby turn on it until then.
    pub rallied_until: f32,
    /// The debuffs last shown on it: (spell, stacks), in slot order.
    pub shown: Vec<(u32, u8)>,
}

/// A trap waiting at a hunter's feet.
#[derive(Debug)]
pub(super) struct Trap {
    pub owner: EntityId,
    pub spell: usize,
    pub nums: Nums,
    pub armed_at: f32,
    pub until: f32,
    /// The trap that shows where it lies.
    pub mark: GroundMark,
}

/// Why a cast didn't go off (shown to the player).
pub(super) type CastResult = Result<(), CastError>;

/// How fast a Charge runs, yards a second (1.12's: run speed x4, at most 24).
pub(super) const CHARGE_SPEED: f32 = 24.0;

impl Sim {
    // --- what a player holds -----------------------------------------------------------------

    fn spell_def(&self, id: EntityId, slot: usize) -> Option<&SpellDef> {
        let p = self.entities.get(&id)?.player()?;
        let o = p.spells.get(slot)?;
        Some(&self.content.kit(p.class).spells[o.spell])
    }

    /// The player's spells in bar order: card spells as learned, then those talents taught.
    /// The player's spells in key order.
    pub(super) fn bar_order(&self, id: EntityId) -> Vec<usize> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let mut v: Vec<usize> = (0..p.spells.len()).collect();
        v.sort_by_key(|&i| p.spells[i].key);
        v
    }

    /// The spell on each key, up to the last key in use (`None` for an empty key).
    pub(super) fn key_map(&self, id: EntityId) -> Vec<Option<usize>> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let n = p.spells.iter().map(|o| o.key as usize + 1).max().unwrap_or(0);
        let mut v = vec![None; n];
        for (i, o) in p.spells.iter().enumerate() {
            v[o.key as usize] = Some(i);
        }
        v
    }

    /// Move the spell on key `from` to key `to` (0-based): onto an empty key, or swapping with
    /// the spell there, cards' and talents' alike.
    pub fn move_spell(&mut self, id: EntityId, from: usize, to: usize) -> bool {
        let keys = self.key_map(id);
        let Some(&Some(i)) = keys.get(from) else { return false };
        if from == to || to >= self.content.game.spell_slots.max(keys.len()) {
            return false;
        }
        let other = keys.get(to).copied().flatten();
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return false };
        if let Some(j) = other {
            p.spells[j].key = from as u8;
        }
        p.spells[i].key = to as u8;
        self.events.push(Event::Bar { player: id });
        true
    }

    /// Spells on the bar: cards' and the ones talents taught. Six keys hold them all.
    pub(super) fn card_spells(&self, id: EntityId) -> usize {
        self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.spells.len())
    }

    /// Learn a spell from a card (rank 1).
    pub(super) fn learn_spell(&mut self, id: EntityId, spell: usize, talent: bool) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if p.spells.iter().any(|o| o.spell == spell) {
            return;
        }
        let cooldown = if p.manual || talent { 0.0 } else { 1.0 };
        // The first free key.
        let key = (0u8..).find(|k| !p.spells.iter().any(|o| o.key == *k)).unwrap_or(0);
        p.spells.push(OwnedSpell { spell, perks: Vec::new(), talent, cooldown, nums: Nums::default(), on: false, key });
        self.recompute(id);
        self.events.push(Event::Bar { player: id });
    }

    /// Take an upgrade for a held spell.
    pub(super) fn take_perk(&mut self, id: EntityId, spell: usize, perk: usize) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        let Some(o) = p.spells.iter_mut().find(|o| o.spell == spell) else { return };
        if o.perks.contains(&perk) || o.perks.len() >= MAX_PERKS {
            return;
        }
        o.perks.push(perk);
        self.recompute(id);
        self.events.push(Event::Bar { player: id });
    }

    /// Work out every held spell's numbers: base, upgrades, talents, gear.
    pub(super) fn recompute_spells(&mut self, id: EntityId) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let kit = self.content.kit(p.class);
        let class_key = self.content.classes[p.class].key.as_str();
        let pieces = p.stats.set_pieces;
        let enchant = p.stats.enchant_school;
        let wares = self.worn_ware_effects(p);
        let mut out = Vec::new();
        for o in &p.spells {
            let def = &kit.spells[o.spell];
            let mut n = def.nums.clone();
            for &pk in &o.perks {
                if let Some(perk) = def.perks.get(pk) {
                    perk.patch.apply(&mut n, 1.0);
                }
                n.power += PERK_POWER;
            }
            for (ti, t) in kit.talents.iter().enumerate() {
                let rank = p.talents.get(ti).copied().unwrap_or(0);
                if rank == 0 {
                    continue;
                }
                for e in &t.effects {
                    if e.filter.as_ref().is_some_and(|f| f.matches(def)) {
                        e.patch.apply(&mut n, rank as f32);
                    }
                }
            }
            for e in &wares {
                if e.filter.as_ref().is_some_and(|f| f.matches(def)) {
                    e.patch.apply(&mut n, 1.0);
                }
            }
            if enchant == Some(def.school) {
                n.power += 0.1;
            }
            // Class set bonuses.
            let fire_frost = def.school == 2 || def.school == 4;
            match class_key {
                "mage" => {
                    if pieces >= 4 && fire_frost {
                        n.power += 0.25;
                    }
                    if pieces >= 6 && fire_frost {
                        n.echo += 0.25;
                    }
                }
                "warrior" => {
                    if pieces >= 4 && matches!(def.key.as_str(), "thunder_clap" | "whirlwind") {
                        n.radius *= 1.4;
                    }
                }
                "priest" => {
                    if pieces >= 4 && def.key == "holy_nova" {
                        n.heal *= 1.5;
                    }
                    if pieces >= 6 && matches!(def.key.as_str(), "smite" | "mind_blast") {
                        n.echo += 0.2;
                    }
                }
                "hunter" => {
                    if pieces >= 2 && def.key == "multi_shot" {
                        n.targets += 1.0;
                    }
                    if pieces >= 4 && matches!(def.key.as_str(), "serpent_sting" | "arcane_shot") {
                        n.power += 0.25;
                    }
                }
                _ => {}
            }
            if def.kind != SpellKind::Toggle {
                n.cooldown = n.cooldown.max(0.5);
            }
            n.cost = n.cost.max(0.0);
            n.slow = n.slow.min(0.9);
            n.only_below = n.only_below.min(1.0);
            out.push(n);
        }
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            for (o, n) in p.spells.iter_mut().zip(out) {
                o.nums = n;
            }
        }
    }

    // --- buffs -------------------------------------------------------------------------------

    pub(super) fn buff(&self, id: EntityId, kind: BuffKind) -> Option<Buff> {
        let p = self.entities.get(&id)?.player()?;
        p.buffs.iter().find(|b| b.kind == kind && b.until > self.clock).copied()
    }

    pub(super) fn buff_value(&self, id: EntityId, kind: BuffKind) -> f32 {
        self.buff(id, kind).map_or(0.0, |b| b.value)
    }

    /// Put a buff on a player (refreshing one already there).
    pub(super) fn give_buff(&mut self, id: EntityId, kind: BuffKind, secs: f32, value: f32, stacks: u32) {
        let until = self.clock + secs;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        match p.buffs.iter_mut().find(|b| b.kind == kind) {
            Some(b) => {
                b.until = until;
                b.value = b.value.max(value);
                b.stacks = stacks;
            }
            None => p.buffs.push(Buff { kind, until, stacks, value, value2: 0.0 }),
        }
        let max_health_changes = kind == BuffKind::LastStand;
        if max_health_changes {
            self.recompute(id);
        }
        self.refresh_auras(id);
    }

    pub(super) fn drop_buff(&mut self, id: EntityId, kind: BuffKind) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        let before = p.buffs.len();
        p.buffs.retain(|b| b.kind != kind);
        if p.buffs.len() != before {
            if kind == BuffKind::LastStand {
                self.recompute(id);
            }
            self.refresh_auras(id);
        }
    }

    /// Damage multiplier from timed buffs (and Shadowform for shadow spells).
    pub(super) fn buff_damage(&self, id: EntityId, school: Option<u8>) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 1.0 };
        let now = self.clock;
        let mut m = 1.0;
        for b in p.buffs.iter().filter(|b| b.until > now) {
            match b.kind {
                BuffKind::ArcanePower | BuffKind::DeathWish | BuffKind::PowerInfusion | BuffKind::FuriousHowl => m *= 1.0 + b.value,
                BuffKind::Enrage | BuffKind::SpiritTap | BuffKind::InnerFire => m *= 1.0 + b.value,
                BuffKind::Bloodrage => m *= 1.0 + b.value2,
                _ => {}
            }
        }
        if school == Some(5) {
            if let Some(o) = self.toggle_on(id, "shadowform") {
                m *= 1.0 + o.buff_value;
            }
        }
        m
    }

    /// How fast a player's cooldowns run.
    pub(super) fn cooldown_rate(&self, id: EntityId, school: u8) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 1.0 };
        let now = self.clock;
        let mut r = (1.0 + p.stats.haste) * (1.0 + p.stats.cdr) * (1.0 + self.team_aura(id, |s| (s.team_haste, s.team_haste_radius)));
        for b in p.buffs.iter().filter(|b| b.until > now) {
            match b.kind {
                BuffKind::ArcanePower => r *= 1.0 + b.value,
                BuffKind::RapidFire | BuffKind::QuickShots => r *= 1.0 + b.value,
                BuffKind::PowerInfusion => r *= 1.0 + b.value,
                BuffKind::FocusedCasting => r *= 1.3,
                _ => {}
            }
        }
        if self.content.classes[p.class].key == "mage" && p.stats.set_pieces >= 2 && (school == 2 || school == 4) {
            r *= 1.1;
        }
        r
    }

    /// Attack speed multiplier from buffs (Flurry, Quick Shots, Rapid Fire).
    pub(super) fn attack_haste(&self, id: EntityId) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 1.0 };
        let now = self.clock;
        let mut r = 1.0;
        for b in p.buffs.iter().filter(|b| b.until > now) {
            match b.kind {
                BuffKind::Flurry if b.stacks > 0 => r *= 1.0 + b.value,
                BuffKind::QuickShots | BuffKind::RapidFire | BuffKind::PowerInfusion => r *= 1.0 + b.value,
                _ => {}
            }
        }
        r
    }

    /// Share of damage a player's buffs take off.
    pub(super) fn buff_taken(&self, id: EntityId) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 1.0 };
        let now = self.clock;
        let mut m = 1.0;
        for b in p.buffs.iter().filter(|b| b.until > now) {
            match b.kind {
                BuffKind::ShieldWall | BuffKind::Deterrence | BuffKind::Warded | BuffKind::Inspiration => m *= 1.0 - b.value.min(0.9),
                BuffKind::InnerFire => m *= 1.0 - b.value2.min(0.9),
                _ => {}
            }
        }
        if let Some(o) = self.toggle_on(id, "shadowform") {
            m *= 1.0 - o.buff_value2.min(0.9);
        }
        m
    }

    /// The numbers of a toggle that is on (`effect` named).
    pub(super) fn toggle_on(&self, id: EntityId, effect: &str) -> Option<&Nums> {
        let p = self.entities.get(&id)?.player()?;
        let kit = self.content.kit(p.class);
        p.spells.iter().find(|o| o.on && kit.spells[o.spell].effect == effect).map(|o| &o.nums)
    }

    pub(super) fn step_buffs(&mut self, dt: f32) {
        let now = self.clock;
        for id in self.player_ids() {
            let (expired, alive) = {
                let e = &self.entities[&id];
                let Some(p) = e.player() else { continue };
                (p.buffs.iter().any(|b| b.until <= now), e.alive())
            };
            if expired {
                let last_stand = self.entities[&id].player().unwrap().buffs.iter().any(|b| b.until <= now && b.kind == BuffKind::LastStand);
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.buffs.retain(|b| b.until > now);
                }
                if last_stand {
                    self.recompute(id);
                }
                self.refresh_auras(id);
            }
            if !alive {
                continue;
            }
            // Bloodrage: rage over time; Blood Craze and Spirit Tap: regeneration.
            let (rage, regen) = {
                let p = self.entities[&id].player().unwrap();
                let rage = p.buffs.iter().find(|b| b.kind == BuffKind::Bloodrage).map_or(0.0, |b| b.value);
                let regen: f32 = p
                    .buffs
                    .iter()
                    .filter(|b| matches!(b.kind, BuffKind::BloodCraze))
                    .map(|b| b.value)
                    .sum::<f32>()
                    + p.buffs.iter().filter(|b| b.kind == BuffKind::SpiritTap).map(|b| b.value2).sum::<f32>();
                (rage, regen)
            };
            if rage > 0.0 {
                self.gain_rage(id, rage * dt);
            }
            if regen > 0.0 {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.buff_regen += regen * dt;
                }
            }
            self.step_hots(id);
        }
    }

    fn step_hots(&mut self, id: EntityId) {
        let now = self.clock;
        let due: Vec<(EntityId, u32, u32)> = {
            let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
            let mut out = Vec::new();
            for h in p.hots.iter_mut() {
                if h.next <= now && h.ticks > 0 {
                    h.ticks -= 1;
                    h.next += 1.0;
                    out.push((h.healer, h.spell, h.per_tick.round().max(1.0) as u32));
                }
            }
            p.hots.retain(|h| h.ticks > 0);
            out
        };
        for (healer, spell, amount) in due {
            self.heal(healer, id, spell, amount, false);
        }
        // Banked buff regeneration.
        let (bank, max) = {
            let e = &self.entities[&id];
            (e.player().map_or(0.0, |p| p.buff_regen), e.max_health)
        };
        if bank >= 0.01 {
            let amount = (max as f32 * bank).floor();
            if amount >= 1.0 {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.buff_regen -= amount / max as f32;
                }
                self.heal(id, id, 16488, amount as u32, false);
            }
        }
    }

    pub(super) fn add_hot(&mut self, healer: EntityId, target: EntityId, spell: u32, total: f32, secs: f32) {
        let ticks = secs.round().max(1.0) as u32;
        let now = self.clock;
        let per_tick = total / ticks as f32;
        let Some(p) = self.entities.get_mut(&target).and_then(|e| e.player_mut()) else {
            // Pets and other allies heal at once.
            let amount = total.round().max(1.0) as u32;
            self.heal(healer, target, spell, amount, false);
            return;
        };
        match p.hots.iter_mut().find(|h| h.spell == spell && h.healer == healer) {
            Some(h) => {
                h.per_tick = h.per_tick.max(per_tick);
                h.ticks = ticks;
            }
            None => p.hots.push(Hot { healer, spell, per_tick, ticks, next: now + 1.0 }),
        }
    }

    /// An absorb shield on a player (stacking onto what's left of one).
    pub(super) fn give_shield(&mut self, caster: EntityId, target: EntityId, amount: f32, secs: f32, reflect: f32, burst: f32, spell: u32) {
        let strength = self.entities.get(&caster).and_then(|e| e.player()).map_or(1.0, |p| (1.0 + p.stats.absorb) * p.stats.heal);
        let now = self.clock;
        let Some(p) = self.entities.get_mut(&target).and_then(|e| e.player_mut()) else { return };
        let amount = amount * strength;
        p.shield_pool = if p.shield_until > now { p.shield_pool + amount } else { amount };
        p.shield_until = now + secs.max(1.0);
        p.shield_reflect = reflect;
        p.shield_burst = burst;
        p.shield_caster = caster;
        p.shield_spell = spell;
        self.events.push(Event::Aura { unit: target, slot: 30, spell, stacks: 1, secs });
    }

    // --- rage --------------------------------------------------------------------------------

    pub(super) fn uses_rage(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| self.content.classes[p.class].power == 1)
    }

    pub(super) fn gain_rage(&mut self, id: EntityId, amount: f32) {
        if !self.uses_rage(id) {
            return;
        }
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        p.rage = (p.rage + amount).clamp(0.0, MAX_RAGE);
        self.send_rage(id);
    }

    pub(super) fn send_rage(&mut self, id: EntityId) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        let shown = p.rage.floor() as u32;
        if shown != p.rage_sent {
            p.rage_sent = shown;
            self.events.push(Event::Power { player: id, power: shown * 10 });
        }
    }

    /// Rage from a swing that landed.
    pub(super) fn rage_from_swing(&mut self, id: EntityId, swing_secs: f32) {
        let mult = self.entities.get(&id).and_then(|e| e.player()).map_or(1.0, |p| 1.0 + p.stats.rage_auto);
        self.gain_rage(id, RAGE_PER_SWING_SEC * swing_secs * mult);
    }

    /// Rage from a blow taken; it also opens Revenge.
    pub(super) fn rage_from_hit(&mut self, id: EntityId, damage: u32) {
        let now = self.clock;
        let (max, mult) = match self.entities.get(&id) {
            Some(e) => (e.max_health.max(1) as f32, e.player().map_or(1.0, |p| 1.0 + p.stats.rage_hit)),
            None => return,
        };
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.revenge_until = now + REVENGE_SECS;
            p.fighting_until = now + 6.0;
        }
        self.gain_rage(id, RAGE_FROM_DAMAGE * damage as f32 / max * mult);
    }

    pub(super) fn step_rage(&mut self, dt: f32) {
        let now = self.clock;
        for id in self.player_ids() {
            if !self.uses_rage(id) {
                continue;
            }
            let (fighting, regen, keep) = {
                let p = self.entities[&id].player().unwrap();
                (p.fighting_until > now, p.stats.rage_regen, p.stats.rage_keep)
            };
            if fighting {
                if regen > 0.0 {
                    self.gain_rage(id, regen * dt);
                }
            } else {
                let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { continue };
                if p.rage > keep {
                    p.rage = (p.rage - RAGE_DECAY * dt).max(keep);
                    self.send_rage(id);
                }
            }
        }
    }

    // --- the bar --------------------------------------------------------------------------------

    /// Switch a player between casting by hand and spells that fire on their own.
    pub fn set_manual(&mut self, id: EntityId, on: bool) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.manual = on;
            for o in p.spells.iter_mut() {
                o.cooldown = o.cooldown.max(0.0);
            }
            self.events.push(Event::Bar { player: id });
        }
        self.refresh_auras(id);
    }

    /// The unit a player has selected (`None` when they clear it).
    pub fn select(&mut self, id: EntityId, target: Option<EntityId>) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.selected = target;
        }
    }

    /// The player's spells, in key order (six at most: cards' and talents' share the keys).
    pub fn spell_bar(&self, id: EntityId) -> Option<SpellBar> {
        let p = self.entities.get(&id)?.player()?;
        let kit = self.content.kit(p.class);
        let slots = self
            .key_map(id)
            .into_iter()
            .map(|slot| {
                // An empty key between spells.
                let Some(i) = slot else { return BarSlot::empty() };
                let o = &p.spells[i];
                let def = &kit.spells[o.spell];
                let r = self.cooldown_rate(id, def.school).max(0.01);
                BarSlot {
                    key: def.key.clone(),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    rarity: def.rarity,
                    rank: o.perks.len() as u8 + 1,
                    cooldown: o.nums.cooldown / r,
                    remaining: o.cooldown.max(0.0) / r,
                    cost: o.nums.cost.round() as u32,
                    talent: o.talent,
                    on: o.on,
                }
            })
            .collect();
        let rage = if self.uses_rage(id) { Some(p.rage.floor() as u32) } else { None };
        Some(SpellBar { manual: p.manual, gcd: (p.gcd_until - self.clock).max(0.0), slots, rage })
    }

    // --- casting -----------------------------------------------------------------------------

    /// Start casting spell `i` if it has a cast time (and nothing makes it instant): true when a
    /// cast began, false when it should go off at once.
    fn begin_cast(&mut self, id: EntityId, i: usize) -> Result<bool, CastError> {
        let (def, nums, pos) = {
            let e = &self.entities[&id];
            let p = e.player().unwrap();
            let o = &p.spells[i];
            (self.content.kit(p.class).spells[o.spell].clone(), o.nums.clone(), e.pos)
        };
        if nums.cast <= 0.05 {
            return Ok(false);
        }
        // Presence of Mind, Clearcasting and Inner Focus make the next cast instant.
        let instant = self.buff(id, BuffKind::PresenceOfMind).is_some()
            || self.buff(id, BuffKind::InnerFocus).is_some()
            || self.buff(id, BuffKind::Clearcasting).is_some();
        if instant {
            return Ok(false);
        }
        let shadowform = self.toggle_on(id, "shadowform").is_some();
        if shadowform && (def.school == 1 || def.is_heal()) {
            return Err(CastError::Shadowform);
        }
        // Something to cast at, before the wait.
        let reach = if nums.range > 0.0 { nums.range } else { 30.0 } + 2.0;
        let target = if def.is_heal() || matches!(def.kind, SpellKind::Buff | SpellKind::Shield | SpellKind::Hot) {
            None
        } else {
            let t = self.chosen_enemy(id, pos, reach).or_else(|| self.enemies_near(pos, reach).first().map(|(t, _)| *t));
            if t.is_none() {
                return Err(CastError::NoTarget);
            }
            t
        };
        let haste = self.entities[&id].player().map_or(0.0, |p| p.stats.haste) + self.buff_value(id, BuffKind::PowerInfusion);
        let secs = nums.cast / (1.0 + haste.max(0.0));
        let now = self.clock;
        let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
        p.casting = Some(CastJob { spell: p.spells[i].spell, done_at: now + secs, from: pos, icon: def.icon, channel: false });
        self.events.push(Event::CastStart { caster: id, spell: def.icon, ms: (secs * 1000.0) as u32, target });
        Ok(true)
    }

    /// Whether a player is casting a spell with a cast time.
    pub fn casting(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.casting.is_some())
    }

    /// Casts finishing (or broken by moving).
    pub(super) fn step_casts(&mut self) {
        let now = self.clock;
        for id in self.player_ids() {
            let Some(e) = self.entities.get(&id) else { continue };
            let Some(job) = e.player().and_then(|p| p.casting.clone()) else { continue };
            let moved = dist2d(e.pos, job.from) > 1.0;
            if !e.alive() || moved {
                self.entities.get_mut(&id).unwrap().player_mut().unwrap().casting = None;
                if job.channel {
                    // The missiles still to fly are lost.
                    let spell = self.content.kit(self.entities[&id].player().unwrap().class).spells[job.spell].visual();
                    self.hits.retain(|h| !(h.caster == id && h.spell == spell && h.at > now));
                    self.later.retain(|l| !(l.1 == id && l.2 == spell));
                    self.events.push(Event::ChannelEnd { player: id });
                } else {
                    self.events.push(Event::CastStop { caster: id, spell: job.icon });
                }
                if moved {
                    self.events.push(Event::CastFailed { player: id, error: CastError::Moved });
                }
                continue;
            }
            if now < job.done_at {
                continue;
            }
            if job.channel {
                self.entities.get_mut(&id).unwrap().player_mut().unwrap().casting = None;
                continue;
            }
            let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
            p.casting = None;
            let Some(i) = p.spells.iter().position(|o| o.spell == job.spell) else { continue };
            match self.cast(id, i) {
                Ok(()) => self.events.push(Event::Bar { player: id }),
                Err(e) => {
                    self.events.push(Event::CastStop { caster: id, spell: job.icon });
                    self.events.push(Event::CastFailed { player: id, error: e });
                }
            }
        }
    }

    /// Cast the spell on key `slot` (0 for key 1) by hand.
    pub fn cast_slot(&mut self, id: EntityId, slot: usize) -> Result<(), CastError> {
        let i = self.key_map(id).get(slot).copied().flatten().ok_or(CastError::NoSpell)?;
        let e = self.entities.get(&id).ok_or(CastError::NoSpell)?;
        let p = e.player().ok_or(CastError::NoSpell)?;
        if !e.alive() {
            return Err(CastError::Dead);
        }
        if p.channel.is_some() || p.casting.is_some() {
            return Err(CastError::Busy);
        }
        if !self.combat_on() || !self.paused_for.is_empty() {
            return Err(CastError::NotNow);
        }
        if p.spells[i].cooldown > 0.05 {
            return Err(CastError::NotReady);
        }
        if p.gcd_until > self.clock + 0.05 {
            return Err(CastError::Gcd);
        }
        if !self.begin_cast(id, i)? {
            self.cast(id, i)?;
        }
        let haste = self.entities[&id].player().map_or(0.0, |p| p.stats.haste);
        let gcd = self.content.game.gcd_secs / (1.0 + haste);
        let now = self.clock;
        let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
        p.gcd_until = now + gcd;
        p.hand_casts += 1;
        self.events.push(Event::Bar { player: id });
        Ok(())
    }

    /// Spells that fire on their own (players without the bar), and cooldowns running down.
    pub(super) fn step_spells(&mut self, dt: f32) {
        if !self.combat_on() {
            return;
        }
        for id in self.player_ids() {
            let e = &self.entities[&id];
            if !e.alive() {
                continue;
            }
            let p = e.player().unwrap();
            let manual = p.manual;
            let busy = p.channel.is_some() || p.casting.is_some();
            let kit = self.content.kit(p.class);
            let schools: Vec<u8> = p.spells.iter().map(|o| kit.spells[o.spell].school).collect();
            for (i, school) in schools.into_iter().enumerate() {
                let r = self.cooldown_rate(id, school);
                let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                let o = &mut p.spells[i];
                o.cooldown = (o.cooldown - dt * r).max(if manual { 0.0 } else { -1.0 });
            }
            if manual || busy {
                continue;
            }
            // On their own: the first spell ready that has something to do.
            for i in self.bar_order(id) {
                let ready = self.entities[&id].player().unwrap().spells[i].cooldown <= 0.0;
                if !ready || !self.worth_casting(id, i) {
                    continue;
                }
                let started = match self.begin_cast(id, i) {
                    Ok(true) => Ok(()),
                    Ok(false) => self.cast(id, i),
                    Err(e) => Err(e),
                };
                match started {
                    Ok(()) => {
                        self.events.push(Event::Bar { player: id });
                        break;
                    }
                    Err(CastError::NoRage) => {}
                    Err(_) => {
                        let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                        p.spells[i].cooldown = 0.25;
                    }
                }
            }
        }
    }

    /// Whether a spell firing on its own would do anything useful now.
    fn worth_casting(&self, id: EntityId, i: usize) -> bool {
        let Some(def) = self.spell_def(id, i) else { return false };
        let pos = self.entities[&id].pos;
        let near = !self.enemies_near(pos, 30.0).is_empty();
        match def.kind {
            SpellKind::Buff => near && (def.effect != "inner_fire" || self.buff(id, BuffKind::InnerFire).is_none()),
            SpellKind::Toggle => !self.entities[&id].player().unwrap().spells[i].on,
            SpellKind::Heal | SpellKind::Hot | SpellKind::Shield => {
                let range = def.nums.range.max(ALLY_RANGE);
                self.most_hurt(id, pos, range, 0.8).is_some() || (def.kind == SpellKind::Shield && near)
            }
            SpellKind::Trap => !self.enemies_near(pos, 25.0).is_empty(),
            SpellKind::Blink => false,
            SpellKind::Special => match def.effect.as_str() {
                "cold_snap" => self.entities[&id].player().unwrap().spells.iter().any(|o| o.cooldown > 5.0),
                "feign_death" => {
                    let e = &self.entities[&id];
                    e.health * 2 < e.max_health
                }
                "mend_pet" => self.pet_hurt(id),
                _ => near,
            },
            _ => near,
        }
    }

    /// Cast the player's `i`th spell now (checks rage, targets; starts the cooldown).
    pub(super) fn cast(&mut self, id: EntityId, i: usize) -> CastResult {
        let (def, nums) = {
            let p = self.entities[&id].player().unwrap();
            let o = &p.spells[i];
            (self.content.kit(p.class).spells[o.spell].clone(), o.nums.clone())
        };
        let shadowform = self.toggle_on(id, "shadowform").is_some();
        if shadowform && (def.school == 1 || def.is_heal()) && def.effect != "shadowform" {
            return Err(CastError::Shadowform);
        }
        if nums.cost > 0.0 && self.entities[&id].player().unwrap().rage + 0.01 < nums.cost {
            return Err(CastError::NoRage);
        }
        if nums.after_hit > 0.0 && self.entities[&id].player().unwrap().revenge_until < self.clock {
            return Err(CastError::NotNow);
        }
        // Free casts: Clearcasting, Presence of Mind, Inner Focus.
        let free = self.buff(id, BuffKind::PresenceOfMind).is_some()
            || self.buff(id, BuffKind::InnerFocus).is_some()
            || self.buff(id, BuffKind::Clearcasting).is_some();
        let talent_buff = def.kind == SpellKind::Buff && matches!(def.effect.as_str(), "presence_of_mind" | "inner_focus");
        let ok = self.cast_kind(id, &def, &nums, i)?;
        if !ok {
            return Err(CastError::NoTarget);
        }
        // A channel holds the caster still while its missiles fly.
        if nums.channel > 0.05 {
            let (pos, now) = (self.entities[&id].pos, self.clock);
            let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
            p.casting = Some(CastJob { spell: p.spells[i].spell, done_at: now + nums.channel, from: pos, icon: def.icon, channel: true });
            self.events.push(Event::Channel { player: id, spell: def.icon, ms: (nums.channel * 1000.0) as u32 });
        }
        // Paid for.
        let mut no_cooldown = false;
        if !talent_buff && free {
            no_cooldown = true;
            for k in [BuffKind::PresenceOfMind, BuffKind::InnerFocus, BuffKind::Clearcasting] {
                if self.buff(id, k).is_some() {
                    self.drop_buff(id, k);
                    break;
                }
            }
        }
        {
            let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
            if nums.cost > 0.0 {
                p.rage = (p.rage - nums.cost).max(0.0);
            }
            p.spells[i].cooldown = if no_cooldown { 0.0 } else { nums.cooldown };
            p.casts += 1;
            p.fighting_until = self.clock + 6.0;
        }
        if nums.cost > 0.0 {
            self.send_rage(id);
        }
        if nums.rage > 0.0 && def.kind != SpellKind::Charge {
            self.gain_rage(id, nums.rage);
        }
        self.after_cast(id, &def, &nums);
        // Echo: sometimes it goes off again, free.
        if nums.echo > 0.0 && self.rng.gen::<f32>() < nums.echo.min(1.0) && !matches!(def.kind, SpellKind::Buff | SpellKind::Toggle) {
            let _ = self.cast_kind(id, &def, &nums, i);
        }
        Ok(())
    }

    /// What follows a cast: Clearcasting, Combustion stacks, resets.
    fn after_cast(&mut self, id: EntityId, def: &SpellDef, nums: &Nums) {
        let procs = self.entities[&id].player().unwrap().stats.procs.clone();
        if procs.clearcasting > 0.0 && self.rng.gen::<f32>() < procs.clearcasting && def.kind != SpellKind::Buff {
            self.give_buff(id, BuffKind::Clearcasting, 15.0, 1.0, 1);
            self.events.push(Event::SpellGo { caster: id, spell: 12536, target: SpellTarget::Unit(id), hits: vec![id] });
        }
        if nums.clearcast > 0.0 {
            self.give_buff(id, BuffKind::Clearcasting, 15.0, 1.0, 1);
        }
        if def.school == 2 {
            if let Some(b) = self.buff(id, BuffKind::Combustion) {
                let stacks = b.stacks + 1;
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    if let Some(c) = p.buffs.iter_mut().find(|x| x.kind == BuffKind::Combustion) {
                        c.stacks = stacks;
                    }
                }
                self.refresh_auras(id);
            }
        }
        if nums.reset_others > 0.0 {
            let key = def.key.clone();
            let kit_spells: Vec<String> = {
                let p = self.entities[&id].player().unwrap();
                p.spells.iter().map(|o| self.content.kit(p.class).spells[o.spell].key.clone()).collect()
            };
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                for (o, k) in p.spells.iter_mut().zip(kit_spells) {
                    if k != key {
                        o.cooldown = 0.0;
                    }
                }
            }
        }
    }

    /// Cast a held spell for free, with no cooldown (set bonuses).
    pub(super) fn free_cast(&mut self, id: EntityId, key: &str) {
        let found = {
            let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
            let kit = self.content.kit(p.class);
            p.spells.iter().position(|o| kit.spells[o.spell].key == key).map(|i| (i, kit.spells[p.spells[i].spell].clone(), p.spells[i].nums.clone()))
        };
        if let Some((i, def, nums)) = found {
            let _ = self.cast_kind(id, &def, &nums, i);
        }
    }

    /// Does the spell's work; false when it had nothing to act on.
    fn cast_kind(&mut self, id: EntityId, def: &SpellDef, n: &Nums, slot: usize) -> Result<bool, CastError> {
        let pos = self.entities[&id].pos;
        let visual = def.visual();
        match def.kind {
            SpellKind::Bolt => self.cast_bolt(id, def, n),
            SpellKind::Nova => {
                let hits: Vec<EntityId> = self.enemies_near(pos, n.radius).into_iter().map(|(v, _)| v).collect();
                let heals = n.heal > 0.0;
                if hits.is_empty() && !heals && n.taunt_secs <= 0.0 {
                    return Ok(false);
                }
                if hits.is_empty() && heals && self.most_hurt(id, pos, n.radius, 1.0).is_none() {
                    return Ok(false);
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(id), hits: hits.clone() });
                let base = self.spell_base(id, def, n);
                let info = self.hit_info(id, def, n);
                let mut cdr = 0.0;
                for v in &hits {
                    self.land_hit(id, *v, base, &info);
                    cdr += n.cdr_per_hit;
                }
                self.after_area(id, def, n, hits.len(), cdr);
                if heals {
                    for a in self.allies_near(pos, n.radius) {
                        let amount = self.entities[&a].max_health as f32 * n.heal * (1.0 + n.power);
                        self.heal(id, a, visual, amount.round() as u32, false);
                    }
                }
                Ok(true)
            }
            SpellKind::Cone => {
                // Towards the chosen enemy, else the way the player faces.
                let facing_ = match self.chosen_enemy(id, pos, n.radius + 5.0) {
                    Some(t) => facing(pos, self.entities[&t].pos),
                    None => self.entities[&id].facing,
                };
                let half = (n.angle.max(30.0) / 2.0).to_radians();
                let hits: Vec<EntityId> = self
                    .enemies_near(pos, n.radius)
                    .into_iter()
                    .map(|(v, _)| v)
                    .filter(|v| {
                        let t = self.entities[v].pos;
                        let a = (t[1] - pos[1]).atan2(t[0] - pos[0]);
                        let mut d = (a - facing_).rem_euclid(std::f32::consts::TAU);
                        if d > std::f32::consts::PI {
                            d -= std::f32::consts::TAU;
                        }
                        d.abs() <= half || dist2d(t, pos) < 2.0
                    })
                    .collect();
                if hits.is_empty() {
                    return Ok(false);
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(id), hits: hits.clone() });
                let base = self.spell_base(id, def, n);
                let info = self.hit_info(id, def, n);
                for v in &hits {
                    self.land_hit(id, *v, base, &info);
                }
                self.after_area(id, def, n, hits.len(), 0.0);
                Ok(true)
            }
            SpellKind::Zone => {
                let heal_zone = n.heal > 0.0 && n.tick <= 0.0;
                let center = if def.target == "self" || heal_zone {
                    if !heal_zone && self.enemies_near(pos, n.radius).is_empty() {
                        return Ok(false);
                    }
                    pos
                } else {
                    let range = if n.range > 0.0 { n.range } else { 30.0 };
                    match self.chosen_enemy(id, pos, range) {
                        Some(t) => self.entities[&t].pos,
                        None => match self.best_cluster(pos, range, n.radius) {
                            Some(c) => c,
                            None => return Ok(false),
                        },
                    }
                };
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Point(center), hits: vec![] });
                if n.damage > 0.0 {
                    let base = self.spell_base(id, def, n);
                    let info = self.hit_info(id, def, n);
                    for (v, _) in self.enemies_near(center, n.radius) {
                        self.land_hit(id, v, base, &info);
                    }
                }
                self.make_zone(id, def, n, center);
                Ok(true)
            }
            SpellKind::Heal | SpellKind::Hot | SpellKind::Shield => {
                let range = if n.range > 0.0 { n.range } else { 40.0 };
                let targets = if def.target == "self" {
                    vec![id]
                } else {
                    let threshold = if def.kind == SpellKind::Shield { 1.01 } else { 1.0 };
                    let mut t = self.hurt_allies(id, pos, range, threshold);
                    if t.is_empty() {
                        t.push(id);
                    }
                    t.truncate(n.targets.max(1.0) as usize);
                    t
                };
                let healer = id;
                for (k, t) in targets.iter().enumerate() {
                    let max = self.entities[t].max_health as f32;
                    let falloff = 0.9f32.powi(k as i32);
                    let crit = self.rng.gen::<f32>() < self.player_crit(id) + n.crit;
                    let mut mult = (1.0 + n.power) * falloff;
                    if crit {
                        mult *= 1.5;
                    }
                    if n.heal > 0.0 {
                        self.heal(healer, *t, visual, (max * n.heal * mult).round() as u32, crit);
                    }
                    if n.hot > 0.0 {
                        let secs = if n.hot_secs > 0.0 { n.hot_secs } else { 12.0 };
                        self.add_hot(healer, *t, visual, max * n.hot * (1.0 + n.power) * falloff, secs);
                    }
                    if n.absorb > 0.0 {
                        let secs = if n.duration > 0.0 { n.duration } else { 15.0 };
                        self.give_shield(healer, *t, max * n.absorb * (1.0 + n.power) * falloff, secs, n.reflect, n.burst, visual);
                    }
                    if n.buff_value > 0.0 && n.duration > 0.0 {
                        self.give_buff(*t, BuffKind::Warded, n.duration, n.buff_value, 1);
                    }
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(targets[0]), hits: targets.clone() });
                if n.damage > 0.0 && n.radius > 0.0 {
                    self.burst_at(id, def, n, pos);
                }
                Ok(true)
            }
            SpellKind::Buff => {
                self.cast_buff(id, def, n);
                Ok(true)
            }
            SpellKind::Toggle => {
                let now_on = {
                    let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                    let o = &mut p.spells[slot];
                    o.on = !o.on;
                    o.on
                };
                if now_on {
                    self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(id), hits: vec![id] });
                }
                self.refresh_auras(id);
                Ok(true)
            }
            SpellKind::Trap => {
                self.place_trap(id, def, n, slot);
                Ok(true)
            }
            SpellKind::Blink => {
                self.blink(id, def, n);
                Ok(true)
            }
            SpellKind::Charge => self.charge(id, def, n),
            SpellKind::Pet => self.pet_strike(id, def, n),
            SpellKind::Special => self.cast_special(id, def, n),
        }
    }

    /// A bolt (or strike) at the chosen enemy, the nearest one, or one not yet afflicted.
    fn cast_bolt(&mut self, id: EntityId, def: &SpellDef, n: &Nums) -> Result<bool, CastError> {
        let pos = self.entities[&id].pos;
        let range = if n.range > 0.0 { n.range } else { 30.0 };
        let near = self.enemies_near(pos, range);
        if near.is_empty() {
            return Ok(false);
        }
        let dot_only = n.damage <= 0.0 && n.dot > 0.0;
        let chosen = self.chosen_enemy(id, pos, range);
        let first = match chosen {
            Some(t) => t,
            None if dot_only => near
                .iter()
                .find(|(v, _)| !self.has_dot(*v, id, def.visual()))
                .or(near.first())
                .map(|(v, _)| *v)
                .unwrap(),
            None if n.mark > 0.0 => near
                .iter()
                .max_by_key(|(v, _)| self.entities[v].max_health)
                .map(|(v, _)| *v)
                .unwrap(),
            None if n.only_below > 0.0 => match near.iter().find(|(v, _)| self.health_share(*v) < n.only_below) {
                Some((v, _)) => *v,
                None => return Err(CastError::NotNow),
            },
            None => near[0].0,
        };
        if n.only_below > 0.0 && self.health_share(first) >= n.only_below {
            return Err(CastError::NotNow);
        }
        if n.mark > 0.0 {
            self.apply_mark(id, first, n);
            self.events.push(Event::SpellGo { caster: id, spell: def.visual(), target: SpellTarget::Unit(first), hits: vec![first] });
            return Ok(true);
        }
        let mut victims = vec![first];
        let extra = n.targets.round().max(0.0) as usize;
        if extra > 0 {
            let at = self.entities[&first].pos;
            let around: Vec<EntityId> = self
                .enemies_near(at, 12.0)
                .into_iter()
                .map(|(v, _)| v)
                .filter(|v| *v != first)
                .filter(|v| n.only_below <= 0.0 || self.health_share(*v) < n.only_below)
                .filter(|v| !(def.key == "mind_blast" && !self.afflicted(*v)))
                .take(extra)
                .collect();
            victims.extend(around);
        }
        // Sweeping Strikes: melee strikes hit one more.
        if range <= 8.0 && def.school == 0 {
            if let Some(b) = self.buff(id, BuffKind::SweepingStrikes) {
                let at = self.entities[&first].pos;
                if let Some((o, _)) = self.enemies_near(at, 8.0).into_iter().find(|(o, _)| !victims.contains(o)) {
                    victims.push(o);
                }
                self.use_charge(id, BuffKind::SweepingStrikes, b);
            }
        }
        let base = self.spell_base(id, def, n);
        let info = self.hit_info(id, def, n);
        let visual = def.visual();
        let missiles = n.projectiles.round().max(1.0) as usize;
        if n.chain > 0.0 {
            let mut hits = vec![first];
            let mut at = self.entities[&first].pos;
            while hits.len() <= n.chain as usize {
                match self.enemies_near(at, 12.0).into_iter().find(|(o, _)| !hits.contains(o)) {
                    Some((o, _)) => {
                        hits.push(o);
                        at = self.entities[&o].pos;
                    }
                    None => break,
                }
            }
            self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(first), hits: hits.clone() });
            let mut dmg = base;
            for (k, h) in hits.into_iter().enumerate() {
                self.queue_spell_hit(id, h, dmg, 0.15 + 0.12 * k as f32, info.clone());
                dmg *= if n.falloff > 0.0 { n.falloff } else { 0.8 };
            }
            return Ok(true);
        }
        for (k, &v) in victims.iter().enumerate() {
            for m in 0..missiles {
                let d = dist2d(pos, self.entities[&v].pos);
                let travel = if n.speed > 0.0 { d / n.speed } else { 0.0 };
                // A channel spreads its missiles over the channel.
                let gap = if n.channel > 0.05 { n.channel / missiles as f32 } else { 0.28 };
                let delay = travel + gap * m as f32 + 0.04 * k as f32;
                let go = Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(v), hits: vec![v] };
                if n.channel > 0.05 && m > 0 {
                    // A channel's missiles leave one by one, each as it would.
                    self.later.push((self.clock + gap * m as f32, id, visual, go));
                } else {
                    self.events.push(go);
                }
                self.queue_spell_hit(id, v, base, delay, info.clone());
            }
        }
        Ok(true)
    }

    fn cast_buff(&mut self, id: EntityId, def: &SpellDef, n: &Nums) {
        let secs = if n.duration > 0.0 { n.duration } else { 15.0 };
        let visual = def.visual();
        let pos = self.entities[&id].pos;
        let mut targets = vec![id];
        match def.effect.as_str() {
            "presence_of_mind" => self.give_buff(id, BuffKind::PresenceOfMind, secs, 0.2, 1),
            "inner_focus" => self.give_buff(id, BuffKind::InnerFocus, secs, 1.0, 1),
            "arcane_power" => self.give_buff(id, BuffKind::ArcanePower, secs, n.buff_value, 1),
            "combustion" => self.give_buff(id, BuffKind::Combustion, secs, n.buff_value, 0),
            "death_wish" => self.give_buff(id, BuffKind::DeathWish, secs, n.buff_value, 1),
            "last_stand" => {
                self.give_buff(id, BuffKind::LastStand, secs, n.buff_value, 1);
                let e = &self.entities[&id];
                let gain = (e.max_health as f32 * n.buff_value / (1.0 + n.buff_value)).round() as u32;
                self.heal(id, id, visual, gain, false);
            }
            "shield_wall" => self.give_buff(id, BuffKind::ShieldWall, secs, n.buff_value, 1),
            "deterrence" => self.give_buff(id, BuffKind::Deterrence, secs, n.buff_value, 1),
            "rapid_fire" => {
                self.give_buff(id, BuffKind::RapidFire, secs, n.buff_value, 1);
                if n.buff_value2 > 0.0 {
                    if let Some(pet) = self.pet_of(id) {
                        let _ = pet;
                        self.give_buff(id, BuffKind::PetFrenzy, secs, n.buff_value2, 1);
                    }
                }
            }
            "sweeping_strikes" => self.give_buff(id, BuffKind::SweepingStrikes, secs, 1.0, n.charges.max(1.0) as u32),
            "bloodrage" => {
                let rage_per_sec = n.buff_value / secs.min(5.0);
                self.give_buff(id, BuffKind::Bloodrage, secs.max(5.0), rage_per_sec, 1);
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    if let Some(b) = p.buffs.iter_mut().find(|b| b.kind == BuffKind::Bloodrage) {
                        b.value2 = n.buff_value2;
                    }
                }
                if n.heal_self < 0.0 {
                    let e = self.entities.get_mut(&id).unwrap();
                    let cost = (e.max_health as f32 * -n.heal_self) as u32;
                    e.health = e.health.saturating_sub(cost).max(1);
                    let (health, max_health) = (e.health, e.max_health);
                    self.events.push(Event::Health { id, health, max_health });
                }
            }
            "bestial_wrath" => self.give_buff(id, BuffKind::BestialWrath, secs, n.buff_value, 1),
            "power_infusion" => {
                let range = if n.range > 0.0 { n.range } else { 40.0 };
                let sel = self.entities[&id].player().and_then(|p| p.selected);
                let t = sel
                    .filter(|s| *s != id && self.entities.get(s).is_some_and(|e| e.is_player() && e.alive() && dist2d(e.pos, pos) <= range))
                    .or_else(|| {
                        self.player_ids()
                            .into_iter()
                            .filter(|o| *o != id && self.entities[o].alive() && dist2d(self.entities[o].pos, pos) <= range)
                            .min_by(|a, b| dist2d(self.entities[a].pos, pos).total_cmp(&dist2d(self.entities[b].pos, pos)))
                    })
                    .unwrap_or(id);
                targets = vec![t];
                self.give_buff(t, BuffKind::PowerInfusion, secs, n.buff_value, 1);
            }
            "inner_fire" => {
                targets = if n.targets > 0.0 { self.allies_near(pos, n.range.max(ALLY_RANGE)) } else { vec![id] };
                for &t in &targets {
                    if self.entities[&t].is_player() {
                        self.give_buff(t, BuffKind::InnerFire, secs, n.buff_value, 1);
                        if let Some(p) = self.entities.get_mut(&t).and_then(|e| e.player_mut()) {
                            if let Some(b) = p.buffs.iter_mut().find(|b| b.kind == BuffKind::InnerFire) {
                                b.value2 = n.buff_value2;
                            }
                        }
                    }
                }
            }
            "furious_howl" => {
                let range = if n.range > 0.0 { n.range } else { ALLY_RANGE };
                targets = self.allies_near(pos, range);
                for &t in &targets {
                    if self.entities[&t].is_player() {
                        self.give_buff(t, BuffKind::FuriousHowl, secs, n.buff_value, 1);
                    }
                    if n.heal > 0.0 {
                        let amount = self.entities[&t].max_health as f32 * n.heal;
                        self.heal(id, t, visual, amount.round() as u32, false);
                    }
                }
                if n.fear_secs > 0.0 {
                    let at = self.pet_of(id).map_or(pos, |p| self.entities[&p].pos);
                    for (v, _) in self.enemies_near(at, n.radius.max(8.0)) {
                        self.fear_enemy(v, at, n.fear_secs);
                    }
                }
            }
            _ => {}
        }
        if n.hot > 0.0 {
            for &t in &targets {
                let max = self.entities[&t].max_health as f32;
                self.add_hot(id, t, visual, max * n.hot, if n.hot_secs > 0.0 { n.hot_secs } else { 12.0 });
            }
        }
        if n.damage > 0.0 && n.radius > 0.0 {
            self.burst_at(id, def, n, pos);
        }
        self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(targets[0]), hits: targets });
    }

    /// A blast around a point that comes with another spell (Inner Flame, Arcane Wake...).
    fn burst_at(&mut self, id: EntityId, def: &SpellDef, n: &Nums, at: Vec3) {
        let hits: Vec<EntityId> = self.enemies_near(at, n.radius).into_iter().map(|(v, _)| v).collect();
        let base = self.spell_base(id, def, n);
        let info = self.hit_info(id, def, n);
        for v in hits {
            self.land_hit(id, v, base, &info);
        }
    }

    fn cast_special(&mut self, id: EntityId, def: &SpellDef, n: &Nums) -> Result<bool, CastError> {
        let pos = self.entities[&id].pos;
        let visual = def.visual();
        match def.effect.as_str() {
            "cold_snap" => {
                let frost: Vec<bool> = {
                    let p = self.entities[&id].player().unwrap();
                    let kit = self.content.kit(p.class);
                    p.spells.iter().map(|o| kit.spells[o.spell].school == 4 && kit.spells[o.spell].key != def.key).collect()
                };
                let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                for (o, f) in p.spells.iter_mut().zip(frost) {
                    if f {
                        o.cooldown = 0.0;
                    }
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(id), hits: vec![id] });
                Ok(true)
            }
            "feign_death" => {
                let attackers: Vec<EntityId> = self
                    .entities
                    .iter()
                    .filter(|(_, e)| {
                        e.enemy().is_some_and(|en| en.goal == Goal::Player(id) || en.engaged == Some(id) || en.goal == Goal::Attack(id))
                    })
                    .map(|(&v, _)| v)
                    .collect();
                for v in attackers {
                    self.drop_aggro(v, id);
                }
                let max = self.entities[&id].max_health as f32;
                self.heal(id, id, visual, (max * n.heal).round() as u32, false);
                if n.speed_buff > 0.0 {
                    self.give_buff(id, BuffKind::Accelerated, n.speed_secs.max(1.0), n.speed_buff, 1);
                    self.update_speed(id);
                }
                if n.radius > 0.0 && (n.dot > 0.0 || n.stun_chance > 0.0) {
                    let info = self.hit_info(id, def, n);
                    let base = self.spell_base(id, def, n);
                    for (v, _) in self.enemies_near(pos, n.radius) {
                        self.land_hit(id, v, base, &info);
                    }
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(id), hits: vec![id] });
                Ok(true)
            }
            "mend_pet" => {
                let pet = match self.pet_of(id) {
                    Some(p) => p,
                    None => {
                        self.summon_pet(id);
                        match self.pet_of(id) {
                            Some(p) => p,
                            None => return Ok(false),
                        }
                    }
                };
                let max = self.entities[&pet].max_health as f32;
                let amount = max * n.hot * (1.0 + n.power) * self.player_heal_mult(id);
                self.heal(id, pet, visual, amount.round() as u32, false);
                if n.heal > 0.0 {
                    let me = self.entities[&id].max_health as f32;
                    self.add_hot(id, id, visual, me * n.heal, n.hot_secs.max(1.0));
                }
                if n.buff_value > 0.0 {
                    self.give_buff(id, BuffKind::PetFury, n.hot_secs.max(1.0), n.buff_value, 1);
                }
                if n.buff_value2 > 0.0 {
                    self.give_buff(id, BuffKind::PetWard, n.hot_secs.max(1.0), n.buff_value2, 1);
                }
                self.events.push(Event::SpellGo { caster: id, spell: visual, target: SpellTarget::Unit(pet), hits: vec![pet] });
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    fn after_area(&mut self, id: EntityId, _def: &SpellDef, n: &Nums, hit: usize, cdr: f32) {
        if hit == 0 {
            return;
        }
        if n.heal_per_hit > 0.0 {
            let max = self.entities[&id].max_health as f32;
            self.heal(id, id, 0, (max * n.heal_per_hit * hit as f32).round() as u32, false);
        }
        if n.ward > 0.0 {
            let max = self.entities[&id].max_health as f32;
            let amount = max * (n.ward * hit as f32).min(0.4);
            self.give_shield(id, id, amount, 15.0, 0.0, 0.0, 11426);
        }
        if n.refund > 0.0 && n.taunt_secs > 0.0 {
            self.gain_rage(id, n.refund * hit as f32);
        }
        if cdr > 0.0 {
            let cut = if n.cdr_cap > 0.0 { cdr.min(n.cdr_cap) } else { cdr };
            self.cut_cooldowns(id, cut, None, Some(n.cooldown));
        }
    }

    /// Take `secs` off the player's cooldowns: all, or one school's.
    pub(super) fn cut_cooldowns(&mut self, id: EntityId, secs: f32, school: Option<u8>, except_cd: Option<f32>) {
        let schools: Vec<u8> = {
            let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
            let kit = self.content.kit(p.class);
            p.spells.iter().map(|o| kit.spells[o.spell].school).collect()
        };
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        for (o, s) in p.spells.iter_mut().zip(schools) {
            if school.is_some_and(|x| x != s) {
                continue;
            }
            // The spell just cast keeps its full cooldown.
            if except_cd.is_some_and(|c| (o.cooldown - c).abs() < 0.01) {
                continue;
            }
            o.cooldown = (o.cooldown - secs).max(0.0);
        }
        self.events.push(Event::Bar { player: id });
    }

    fn use_charge(&mut self, id: EntityId, kind: BuffKind, b: Buff) {
        if b.stacks <= 1 {
            self.drop_buff(id, kind);
        } else if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            if let Some(x) = p.buffs.iter_mut().find(|x| x.kind == kind) {
                x.stacks -= 1;
            }
            self.refresh_auras(id);
        }
    }

    // --- aiming -------------------------------------------------------------------------------

    /// The enemy a player has selected (or is hitting), if it's standing within `range` of `pos`.
    pub(super) fn chosen_enemy(&self, id: EntityId, pos: Vec3, range: f32) -> Option<EntityId> {
        let p = self.entities.get(&id)?.player()?;
        [p.selected, p.attacking].into_iter().flatten().find(|t| {
            self.entities.get(t).is_some_and(|e| e.is_enemy() && e.alive() && dist2d(e.pos, pos) <= range)
        })
    }

    /// The spot within `range` with the most enemies around it.
    fn best_cluster(&self, pos: Vec3, range: f32, radius: f32) -> Option<Vec3> {
        self.enemies_near(pos, range)
            .iter()
            .map(|(n, _)| {
                let p = self.entities[n].pos;
                (p, self.enemies_near(p, radius).len())
            })
            .max_by_key(|(_, c)| *c)
            .map(|(p, _)| p)
    }

    /// Players (and pets and the Ancient's defenders) within `r` of `pos`.
    pub(super) fn allies_near(&self, pos: Vec3, r: f32) -> Vec<EntityId> {
        self.entities
            .iter()
            .filter(|(_, e)| e.alive() && (e.is_player() || matches!(e.body, Body::Ally(_))) && dist2d(e.pos, pos) <= r)
            .map(|(&id, _)| id)
            .collect()
    }

    /// Allies within `r`, most hurt first, below `below` of their health.
    fn hurt_allies(&self, id: EntityId, pos: Vec3, r: f32, below: f32) -> Vec<EntityId> {
        let sel = self.entities[&id].player().and_then(|p| p.selected);
        let mut v: Vec<(EntityId, f32)> = self
            .allies_near(pos, r)
            .into_iter()
            .map(|a| (a, self.health_share(a)))
            .filter(|(_, s)| *s < below)
            .collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        let mut out: Vec<EntityId> = v.into_iter().map(|(a, _)| a).collect();
        // A hurt friend the player has selected comes first.
        if let Some(s) = sel {
            if let Some(k) = out.iter().position(|a| *a == s) {
                let a = out.remove(k);
                out.insert(0, a);
            }
        }
        out
    }

    fn most_hurt(&self, id: EntityId, pos: Vec3, r: f32, below: f32) -> Option<EntityId> {
        self.hurt_allies(id, pos, r, below).into_iter().next()
    }

    pub(super) fn health_share(&self, id: EntityId) -> f32 {
        self.entities.get(&id).map_or(1.0, |e| e.health as f32 / e.max_health.max(1) as f32)
    }

    fn has_dot(&self, victim: EntityId, caster: EntityId, spell: u32) -> bool {
        self.entities.get(&victim).and_then(|e| e.enemy()).is_some_and(|en| en.dots.iter().any(|d| d.caster == caster && d.spell == spell))
    }

    // --- hits ---------------------------------------------------------------------------------

    /// A spell's hit before the target: the caster's hit times the spell's damage and every
    /// bonus that doesn't depend on who it lands on.
    pub(super) fn spell_base(&self, id: EntityId, def: &SpellDef, n: &Nums) -> f32 {
        self.player_hit(id) * n.damage * (1.0 + n.power) * self.buff_damage(id, Some(def.school))
    }

    pub(super) fn hit_info(&self, id: EntityId, def: &SpellDef, n: &Nums) -> HitInfo {
        let hit = self.player_hit(id) * (1.0 + n.power) * self.buff_damage(id, Some(def.school));
        let force_crit = self.buff(id, BuffKind::InnerFocus).is_some();
        HitInfo {
            nums: Box::new(n.clone()),
            school: def.school,
            spell: def.visual(),
            dot_total: hit * n.dot,
            splash: false,
            force_crit,
            from_pet: false,
            key_kind: def.kind,
        }
    }

    pub(super) fn queue_spell_hit(&mut self, caster: EntityId, victim: EntityId, base: f32, delay: f32, info: HitInfo) {
        if delay <= 0.01 {
            self.land_hit(caster, victim, base, &info);
            return;
        }
        self.hits.push(cards::PendingHit {
            at: self.clock + delay,
            caster,
            victim,
            spell: info.spell,
            damage: base,
            crit: false,
            school: info.school,
            fx: cards::HitFx::default(),
            info: Some(Box::new(info)),
        });
    }

    pub(super) fn player_crit(&self, id: EntityId) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 0.05 };
        p.stats.crit
    }

    pub(super) fn player_heal_mult(&self, id: EntityId) -> f32 {
        self.entities.get(&id).and_then(|e| e.player()).map_or(1.0, |p| p.stats.heal)
    }

    /// What a hit landing on `victim` does: damage with every bonus against it, a crit roll,
    /// and whatever it leaves behind.
    pub(super) fn land_hit(&mut self, caster: EntityId, victim: EntityId, base: f32, info: &HitInfo) {
        let Some(v) = self.entities.get(&victim) else { return };
        if !v.alive() || !v.is_enemy() {
            return;
        }
        let n = &*info.nums;
        let owner = self.owner_of(caster);
        let (mult, crit_bonus) = self.against(owner, victim, info.school, n, info.from_pet);
        let crit_chance = self.player_crit(owner) + n.crit + crit_bonus + if info.from_pet { self.pet_crit(owner) } else { 0.0 };
        let crit = info.force_crit || self.rng.gen::<f32>() < crit_chance;
        let mut dmg = base * mult;
        if crit {
            dmg *= 1.5 + n.crit_mult;
        }
        if base > 0.0 && dmg >= 0.5 {
            let periodic = false;
            self.deal(caster, victim, dmg.round().max(1.0) as u32, crit, Via::Spell { spell: info.spell, school: info.school, periodic });
            self.after_damage(owner, victim, dmg, info.school, crit, info);
        }
        let alive = self.entities.get(&victim).is_some_and(|e| e.alive());
        // What it leaves behind.
        if alive {
            self.apply_effects(caster, victim, info, crit);
        } else if !info.splash {
            self.on_spell_kill(owner, info);
        }
        // Splash: the same hit, weaker, on what's around (carrying slows and stuns, not more splash).
        if n.splash > 0.0 && !info.splash {
            let at = self.entities.get(&victim).map(|e| e.pos);
            if let Some(at) = at {
                let r = if n.splash_radius > 0.0 { n.splash_radius } else { 8.0 };
                let others: Vec<EntityId> = self.enemies_near(at, r).into_iter().map(|(o, _)| o).filter(|o| *o != victim).take(12).collect();
                let mut sinfo = info.clone();
                sinfo.splash = true;
                sinfo.dot_total = 0.0;
                for o in others {
                    self.land_hit(caster, o, base * n.splash, &sinfo);
                }
            }
        }
        if !info.splash && n.cdr_per_hit > 0.0 && info.key_kind == SpellKind::Bolt {
            self.cut_cooldowns(owner, n.cdr_per_hit.min(n.cdr_cap.max(n.cdr_per_hit)), None, Some(n.cooldown));
        }
    }

    /// The player behind a hit (a pet's owner).
    pub(super) fn owner_of(&self, caster: EntityId) -> EntityId {
        match self.entities.get(&caster).map(|e| &e.body) {
            Some(Body::Ally(a)) => a.owner,
            _ => caster,
        }
    }

    /// Damage multiplier and extra crit chance for a hit of `school` from `owner` on `victim`.
    pub(super) fn against(&self, owner: EntityId, victim: EntityId, school: u8, n: &Nums, pet: bool) -> (f32, f32) {
        let Some(e) = self.entities.get(&victim) else { return (1.0, 0.0) };
        let Some(en) = e.enemy() else { return (1.0, 0.0) };
        let now = self.clock;
        let a = &en.aff;
        let mut m = 1.0;
        let mut crit = 0.0;
        let frozen = a.frozen_until > now;
        let stunned = a.stun_until > now;
        let chilled = en.slow_until > now;
        let burning = en.dots.iter().any(|d| d.school == 2 || d.school == 1);
        let bleeding = en.dots.iter().any(|d| d.school == 0);
        let afflicted = en.dots.iter().any(|d| d.school == 5);
        let stung = en.dots.iter().any(|d| d.school == 3);
        if frozen {
            m *= 1.0 + n.vs_frozen;
            crit += n.crit_vs_frozen;
        }
        if chilled {
            m *= 1.0 + n.vs_chilled;
        }
        if burning {
            m *= 1.0 + n.vs_burning;
        }
        if bleeding {
            m *= 1.0 + n.vs_bleeding;
        }
        if afflicted {
            m *= 1.0 + n.vs_afflicted;
        }
        if stung {
            m *= 1.0 + n.vs_stung;
        }
        if stunned {
            m *= 1.0 + n.vs_stunned;
        }
        let stacks = |v: Vuln| {
            let s = a.vulns[v as usize];
            if s.until > now {
                s.n as f32
            } else {
                0.0
            }
        };
        match school {
            2 => m *= 1.0 + FIRE_VULN * stacks(Vuln::Fire),
            5 => m *= 1.0 + SHADOW_VULN * stacks(Vuln::Shadow),
            0 => m *= 1.0 + SUNDER * stacks(Vuln::Sunder),
            _ => {}
        }
        if school == 4 || n.vuln_frost > 0.0 {
            crit += WINTERS_CHILL * stacks(Vuln::Frost);
        }
        m *= 1.0 + n.vs_sunder * stacks(Vuln::Sunder);
        if let Some(mk) = a.mark {
            if mk.owner == owner {
                m *= 1.0 + mk.bonus + if pet { mk.pet_bonus } else { 0.0 };
                crit += mk.crit;
                m *= 1.0 + n.vs_marked;
            }
        }
        if n.execute > 0.0 && e.health as f32 / e.max_health.max(1) as f32 <= n.execute_at.max(0.2) {
            m *= 1.0 + n.execute;
        }
        if (en.elite || en.kind.boss) && !pet {
            let elite = self.entities.get(&owner).and_then(|o| o.player()).map_or(0.0, |p| p.stats.elite_damage);
            m *= 1.0 + elite;
        }
        (m, crit)
    }

    /// After damage lands: leech, mends, Vampiric Embrace, crit procs, refunds.
    fn after_damage(&mut self, owner: EntityId, victim: EntityId, dmg: f32, school: u8, crit: bool, info: &HitInfo) {
        let n = &*info.nums;
        if !self.entities.get(&owner).is_some_and(|e| e.is_player() && e.alive()) {
            return;
        }
        if n.leech > 0.0 {
            self.heal(owner, owner, info.spell, (dmg * n.leech).round() as u32, false);
        }
        if n.mend > 0.0 {
            let pos = self.entities[&owner].pos;
            if let Some(a) = self.most_hurt(owner, pos, ALLY_RANGE, 1.0) {
                self.heal(owner, a, 2061, (dmg * n.mend).round() as u32, false);
            }
        }
        if n.heal_self > 0.0 && !info.splash {
            let max = self.entities[&owner].max_health as f32;
            self.heal(owner, owner, info.spell, (max * n.heal_self).round() as u32, false);
        }
        if n.heal_per_hit > 0.0 && matches!(info.key_kind, SpellKind::Bolt) {
            let max = self.entities[&owner].max_health as f32;
            self.heal(owner, owner, info.spell, (max * n.heal_per_hit).round() as u32, false);
        }
        let procs = self.entities[&owner].player().unwrap().stats.procs.clone();
        if school == 5 && procs.vampiric_embrace > 0.0 {
            let pos = self.entities[&owner].pos;
            let amount = (dmg * procs.vampiric_embrace).round() as u32;
            if amount > 0 {
                for a in self.allies_near(pos, ALLY_RANGE) {
                    if self.entities[&a].health < self.entities[&a].max_health {
                        self.heal(owner, a, 15290, amount, false);
                    }
                }
            }
        }
        if crit {
            if n.refund > 0.0 && n.taunt_secs <= 0.0 {
                self.gain_rage(owner, n.refund);
            }
            self.on_crit(owner, victim, dmg, school, info.from_pet);
        }
    }

    /// A player's critical strike: Ignite, Deep Wounds, Master of Elements, Enrage, Flurry,
    /// Blood Craze, Combustion's count.
    pub(super) fn on_crit(&mut self, owner: EntityId, victim: EntityId, dmg: f32, school: u8, pet: bool) {
        let Some(p) = self.entities.get(&owner).and_then(|e| e.player()) else { return };
        let procs = p.stats.procs.clone();
        let alive = self.entities.get(&victim).is_some_and(|e| e.alive());
        if pet {
            if procs.frenzy > 0.0 && self.rng.gen::<f32>() < procs.frenzy {
                self.give_buff(owner, BuffKind::PetFrenzy, 8.0, 0.3, 1);
            }
            return;
        }
        if school == 2 && procs.ignite > 0.0 && alive {
            self.add_dot(owner, victim, 12654, 2, dmg * procs.ignite, 4.0, true);
        }
        if school == 0 && procs.deep_wounds > 0.0 && alive {
            let total = self.player_hit(owner) * procs.deep_wounds;
            self.add_dot(owner, victim, 12721, 0, total, 6.0, false);
        }
        if school == 2 && procs.master_of_elements > 0.0 {
            self.cut_cooldowns(owner, procs.master_of_elements, Some(2), None);
        }
        if procs.enrage > 0.0 {
            self.give_buff(owner, BuffKind::Enrage, 8.0, procs.enrage, 1);
        }
        if procs.flurry > 0.0 {
            self.give_buff(owner, BuffKind::Flurry, 15.0, procs.flurry, 3);
        }
        if procs.blood_craze > 0.0 {
            self.give_buff(owner, BuffKind::BloodCraze, 6.0, procs.blood_craze, 1);
        }
        if school == 2 {
            if let Some(b) = self.buff(owner, BuffKind::Combustion) {
                // Three crits and it's spent.
                let crits = b.value2 + 1.0;
                if crits >= 3.0 {
                    self.drop_buff(owner, BuffKind::Combustion);
                } else if let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) {
                    if let Some(c) = p.buffs.iter_mut().find(|x| x.kind == BuffKind::Combustion) {
                        c.value2 = crits;
                    }
                }
            }
        }
    }

    /// Statuses a hit leaves: damage over time, chills, freezes, stuns, fear, vulnerabilities,
    /// taunts, demoralize; spreading and setting off what's there.
    fn apply_effects(&mut self, caster: EntityId, victim: EntityId, info: &HitInfo, _crit: bool) {
        let n = &*info.nums;
        let owner = self.owner_of(caster);
        let now = self.clock;
        if info.dot_total > 0.0 && n.dot_secs > 0.0 {
            let leech = n.leech;
            self.add_dot_full(owner, victim, info.spell, info.school, info.dot_total, n.dot_secs, n.dot_stack > 0.0, leech, n.spread_on_death);
        }
        if n.slow > 0.0 {
            self.slow_enemy(victim, n.slow, if n.slow_secs > 0.0 { n.slow_secs } else { 3.0 });
        }
        if n.freeze_chance > 0.0 && self.rng.gen::<f32>() < n.freeze_chance {
            self.freeze_enemy(victim, if n.freeze_secs > 0.0 { n.freeze_secs } else { 3.0 });
        }
        if n.root_secs > 0.0 && (n.root_chance <= 0.0 || self.rng.gen::<f32>() < n.root_chance) {
            self.root_enemy(victim, n.root_secs);
        }
        if n.stun_chance > 0.0 && self.rng.gen::<f32>() < n.stun_chance {
            self.stun_enemy(victim, if n.stun_secs > 0.0 { n.stun_secs } else { 2.0 });
        }
        if n.fear_secs > 0.0 {
            let from = self.entities[&caster].pos;
            self.fear_enemy(victim, from, n.fear_secs);
        }
        for (v, amount) in [(Vuln::Fire, n.vuln_fire), (Vuln::Frost, n.vuln_frost), (Vuln::Shadow, n.vuln_shadow), (Vuln::Sunder, n.vuln_sunder)] {
            if amount > 0.0 {
                let whole = amount.floor();
                let extra = if self.rng.gen::<f32>() < amount - whole { 1.0 } else { 0.0 };
                let stacks = (whole + extra) as u8;
                if stacks > 0 {
                    self.add_vuln(victim, v, stacks);
                }
            }
        }
        if n.demoralize > 0.0 {
            if let Some(en) = self.entities.get_mut(&victim).and_then(|e| e.enemy_mut()) {
                en.aff.demoralize = en.aff.demoralize.max(n.demoralize);
                en.aff.demoralize_until = now + n.demoralize_secs.max(1.0);
            }
        }
        if n.taunt_secs > 0.0 {
            self.taunt_enemy(victim, owner, n.taunt_secs);
        }
        if n.spread > 0.0 && !info.splash {
            self.spread_burns(owner, victim, n.spread as usize);
        }
        if n.detonate > 0.0 {
            self.detonate_poison(owner, victim, n.detonate);
        }
        self.dirty_debuffs(victim);
    }

    /// A kill by a spell: rage, resets.
    fn on_spell_kill(&mut self, owner: EntityId, info: &HitInfo) {
        let n = &*info.nums;
        if n.rage_on_kill > 0.0 {
            self.gain_rage(owner, n.rage_on_kill);
        }
        if n.reset_on_kill > 0.0 {
            let key_cd = n.cooldown;
            if let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) {
                for o in p.spells.iter_mut() {
                    if (o.nums.cooldown - key_cd).abs() < 0.01 && o.cooldown > 0.0 && o.nums.only_below > 0.0 {
                        o.cooldown = 0.0;
                    }
                }
            }
            self.events.push(Event::Bar { player: owner });
        }
    }

    // --- statuses on enemies -------------------------------------------------------------------

    pub(super) fn freeze_enemy(&mut self, id: EntityId, secs: f32) {
        let now = self.clock;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            let secs = if en.kind.boss { secs * 0.25 } else if en.elite { secs * 0.5 } else { secs };
            en.aff.frozen_until = en.aff.frozen_until.max(now + secs);
            en.root_until = en.root_until.max(now + secs);
        }
        self.halt_frozen(id);
        self.dirty_debuffs(id);
    }

    pub(super) fn stun_enemy(&mut self, id: EntityId, secs: f32) {
        let now = self.clock;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            let secs = if en.kind.boss { secs * 0.25 } else if en.elite { secs * 0.5 } else { secs };
            en.aff.stun_until = en.aff.stun_until.max(now + secs);
            en.root_until = en.root_until.max(now + secs);
        }
        self.halt_frozen(id);
        self.dirty_debuffs(id);
    }

    pub(super) fn fear_enemy(&mut self, id: EntityId, from: Vec3, secs: f32) {
        let now = self.clock;
        let boss = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.kind.boss);
        if boss {
            return;
        }
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.aff.fear_until = now + secs;
            en.aff.fear_from = from;
            en.think = 0.0;
        }
        self.dirty_debuffs(id);
    }

    fn taunt_enemy(&mut self, id: EntityId, by: EntityId, secs: f32) {
        let now = self.clock;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            if en.guard.is_some() && !matches!(en.goal, Goal::Player(_)) {
                return;
            }
            en.aff.taunt = Some((by, now + secs));
            en.goal = Goal::Player(by);
            en.think = 0.0;
        }
    }

    fn add_vuln(&mut self, id: EntityId, v: Vuln, stacks: u8) {
        let now = self.clock;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            let s = &mut en.aff.vulns[v as usize];
            if s.until < now {
                s.n = 0;
            }
            s.n = (s.n + stacks).min(MAX_VULN);
            s.until = now + if v == Vuln::Sunder { 20.0 } else { VULN_SECS };
        }
    }

    pub(super) fn apply_mark(&mut self, owner: EntityId, target: EntityId, n: &Nums) {
        // One mark at a time.
        let marked: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| e.enemy().is_some_and(|en| en.aff.mark.is_some_and(|m| m.owner == owner)))
            .map(|(&id, _)| id)
            .collect();
        for m in marked {
            if let Some(en) = self.entities.get_mut(&m).and_then(|e| e.enemy_mut()) {
                en.aff.mark = None;
            }
            self.dirty_debuffs(m);
        }
        if let Some(en) = self.entities.get_mut(&target).and_then(|e| e.enemy_mut()) {
            en.aff.mark = Some(Mark {
                owner,
                bonus: n.mark,
                crit: n.crit,
                pet_bonus: n.buff_value,
                jump: n.jump_on_death > 0.0,
                reset: n.reset_on_kill > 0.0,
            });
        }
        self.dirty_debuffs(target);
        // Pack Leader: the pet goes for it.
        if n.buff_value > 0.0 {
            if let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) {
                p.pet_target = Some(target);
            }
        }
    }

    /// Spread the target's burns to `n` enemies near it.
    fn spread_burns(&mut self, owner: EntityId, victim: EntityId, n: usize) {
        let Some(e) = self.entities.get(&victim) else { return };
        let at = e.pos;
        let burns: Vec<(u32, u8, f32, u32)> = e
            .enemy()
            .map(|en| en.dots.iter().filter(|d| d.school == 2).map(|d| (d.spell, d.school, d.per_tick, d.ticks_left)).collect())
            .unwrap_or_default();
        if burns.is_empty() {
            return;
        }
        let others: Vec<EntityId> = self.enemies_near(at, 10.0).into_iter().map(|(o, _)| o).filter(|o| *o != victim).take(n).collect();
        for o in others {
            for &(spell, school, per_tick, ticks) in &burns {
                self.add_dot(owner, o, spell, school, per_tick * ticks as f32, ticks as f32, false);
            }
            self.events.push(Event::SpellGo { caster: owner, spell: 2136, target: SpellTarget::Unit(o), hits: vec![o] });
            self.dirty_debuffs(o);
        }
    }

    /// Set off the target's poison: a share of what's left lands at once.
    fn detonate_poison(&mut self, owner: EntityId, victim: EntityId, share: f32) {
        let Some(en) = self.entities.get(&victim).and_then(|e| e.enemy()) else { return };
        let left: f32 = en.dots.iter().filter(|d| d.school == 3 && d.caster == owner).map(|d| d.per_tick * d.ticks_left as f32).sum();
        if left <= 0.0 {
            return;
        }
        let dmg = (left * share).round().max(1.0) as u32;
        self.deal(owner, victim, dmg, false, Via::Spell { spell: 1978, school: 3, periodic: false });
    }

    /// Whether an enemy suffers a shadow damage over time.
    fn afflicted(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.dots.iter().any(|d| d.school == 5))
    }

    pub(super) fn enemy_frozen(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.aff.frozen_until > self.clock || en.aff.stun_until > self.clock)
    }

    pub(super) fn enemy_feared(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.aff.fear_until > self.clock)
    }

    /// Stop a frozen or stunned enemy in its tracks.
    fn halt_frozen(&mut self, id: EntityId) {
        let Some(e) = self.entities.get_mut(&id) else { return };
        let (pos, facing) = (e.pos, e.facing);
        let Some(en) = e.enemy_mut() else { return };
        let moving = en.mover.take().is_some_and(|m| !m.done());
        let victim = en.engaged.take();
        if moving {
            self.events.push(Event::Stop { id, pos, facing });
        }
        if let Some(v) = victim {
            self.events.push(Event::AttackStop { attacker: id, victim: v });
        }
    }

    /// An enemy forgets `who` and heads back for the Ancient.
    pub(super) fn drop_aggro(&mut self, enemy: EntityId, who: EntityId) {
        let Some(en) = self.entities.get_mut(&enemy).and_then(|e| e.enemy_mut()) else { return };
        let engaged = en.engaged == Some(who);
        if engaged {
            en.engaged = None;
        }
        en.goal = if en.guard.is_some() { Goal::Guard } else { Goal::Hub };
        en.aff.taunt = None;
        en.think = 0.0;
        en.ignore = Some((who, self.clock + 4.0));
        if engaged {
            self.events.push(Event::AttackStop { attacker: enemy, victim: who });
        }
    }

    pub(super) fn dirty_debuffs(&mut self, id: EntityId) {
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.debuffs_dirty = true;
        }
    }

    /// Show enemies' debuffs (for the target frame): each status as the spell that caused it.
    pub(super) fn step_debuffs(&mut self) {
        let now = self.clock;
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| e.enemy().is_some_and(|en| en.debuffs_dirty || en.debuff_check <= now))
            .map(|(&id, _)| id)
            .collect();
        for id in ids {
            let (want, next) = {
                let e = &self.entities[&id];
                let en = e.enemy().unwrap();
                let a = &en.aff;
                let mut want: Vec<(u32, u8)> = Vec::new();
                let mut next = f32::INFINITY;
                let mut add = |spell: u32, stacks: u8, until: f32| {
                    if until > now && want.len() < 16 && !want.iter().any(|(s, _)| *s == spell) {
                        want.push((spell, stacks.max(1)));
                        next = next.min(until);
                    }
                };
                if e.alive() {
                    add(122, 1, a.frozen_until);
                    add(12355, 1, a.stun_until);
                    add(8122, 1, a.fear_until);
                    add(6136, 1, en.slow_until);
                    add(22959, a.vulns[0].n, a.vulns[0].until);
                    add(12579, a.vulns[1].n, a.vulns[1].until);
                    add(15258, a.vulns[2].n, a.vulns[2].until);
                    add(7386, a.vulns[3].n, a.vulns[3].until);
                    if a.mark.is_some() {
                        add(1130, 1, f32::INFINITY);
                    }
                    add(1160, 1, a.demoralize_until);
                    for d in &en.dots {
                        add(d.spell, d.stacks, now + d.ticks_left as f32 + 0.5);
                    }
                }
                (want, next)
            };
            let shown = self.entities[&id].enemy().unwrap().aff.shown.clone();
            let mut events = Vec::new();
            for slot in 0..want.len().max(shown.len()) {
                let w = want.get(slot).copied().unwrap_or((0, 0));
                if shown.get(slot).copied() != Some(w) {
                    events.push(Event::Aura { unit: id, slot: 32 + slot as u8, spell: w.0, stacks: w.1, secs: 0.0 });
                }
            }
            self.events.extend(events);
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                en.aff.shown = want;
                en.debuffs_dirty = false;
                en.debuff_check = next.min(now + 1.0);
            }
        }
    }

    // --- zones and traps --------------------------------------------------------------------

    fn make_zone(&mut self, id: EntityId, def: &SpellDef, n: &Nums, center: Vec3) {
        let zid = self.alloc();
        let spell = def.visual();
        let hit = self.player_hit(id) * (1.0 + n.power) * self.buff_damage(id, Some(def.school));
        let mut info = self.hit_info(id, def, n);
        info.dot_total = hit * n.dot;
        let duration = if n.duration > 0.0 { n.duration } else { 6.0 };
        self.entities.insert(
            zid,
            Entity {
                pos: center,
                facing: 0.0,
                health: 1,
                max_health: 1,
                level: 1,
                body: Body::Zone(Zone {
                    caster: id,
                    spell,
                    radius: n.radius,
                    until: self.clock + duration,
                    next_tick: self.clock + 1.0,
                    per_tick: hit * n.tick,
                    school: def.school,
                    slow: n.slow,
                    hostile: false,
                    heal: if n.tick <= 0.0 { n.heal } else { 0.0 },
                    share: 0.0,
                    info: Some(Box::new(info)),
                    cast: 0,
                }),
            },
        );
        self.events.push(Event::ZoneStart { id: zid, caster: id, spell, pos: center, radius: n.radius, mark: GroundMark::None });
    }

    fn place_trap(&mut self, id: EntityId, def: &SpellDef, n: &Nums, slot: usize) {
        let pos = self.entities[&id].pos;
        let spell_index = self.entities[&id].player().unwrap().spells[slot].spell;
        // One of each trap at a time.
        let old: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(&e.body, Body::Trap(t) if t.owner == id && t.spell == spell_index))
            .map(|(&t, _)| t)
            .collect();
        for t in old {
            self.entities.remove(&t);
            self.events.push(Event::ZoneEnd { id: t });
        }
        // The trap's look by its school: fire's Explosive Trap, frost's Freezing (or Frost) Trap.
        let mark = match def.school {
            2 => GroundMark::ExplosiveTrap,
            4 if n.radius > 6.0 => GroundMark::FrostTrap,
            4 => GroundMark::FreezingTrap,
            _ => GroundMark::FrostTrap,
        };
        let tid = self.alloc();
        self.entities.insert(
            tid,
            Entity {
                pos,
                facing: 0.0,
                health: 1,
                max_health: 1,
                level: 1,
                body: Body::Trap(Trap { owner: id, spell: spell_index, nums: n.clone(), armed_at: self.clock + 1.0, until: self.clock + 60.0, mark }),
            },
        );
        self.events.push(Event::SpellGo { caster: id, spell: def.icon, target: SpellTarget::Point(pos), hits: vec![] });
        self.events.push(Event::ZoneStart { id: tid, caster: id, spell: TRAP_LOOK, pos, radius: 2.0, mark });
    }

    pub(super) fn step_traps(&mut self) {
        let now = self.clock;
        let ids: Vec<EntityId> = self.entities.iter().filter(|(_, e)| matches!(e.body, Body::Trap(_))).map(|(&id, _)| id).collect();
        for tid in ids {
            let (owner, spell, n, armed, until, pos) = match &self.entities[&tid].body {
                Body::Trap(t) => (t.owner, t.spell, t.nums.clone(), t.armed_at, t.until, self.entities[&tid].pos),
                _ => continue,
            };
            if until <= now || !self.entities.contains_key(&owner) || !self.combat_on() {
                self.entities.remove(&tid);
                self.events.push(Event::ZoneEnd { id: tid });
                continue;
            }
            if armed > now {
                continue;
            }
            let trigger = if n.trigger_radius > 0.0 { n.trigger_radius } else { 4.0 };
            if self.enemies_near(pos, trigger).is_empty() {
                continue;
            }
            // Spring it.
            self.entities.remove(&tid);
            self.events.push(Event::ZoneEnd { id: tid });
            let Some(p) = self.entities.get(&owner).and_then(|e| e.player()) else { continue };
            let def = self.content.kit(p.class).spells[spell].clone();
            let visual = def.visual();
            let hits: Vec<EntityId> = self.enemies_near(pos, n.radius.max(trigger)).into_iter().map(|(v, _)| v).collect();
            self.events.push(Event::SpellGo { caster: owner, spell: visual, target: SpellTarget::Point(pos), hits: hits.clone() });
            let base = self.spell_base(owner, &def, &n);
            let info = self.hit_info(owner, &def, &n);
            for v in hits {
                self.land_hit(owner, v, base, &info);
            }
            if n.tick > 0.0 && n.duration > 0.0 {
                self.make_zone(owner, &def, &Nums { damage: 0.0, freeze_chance: 0.0, stun_chance: 0.0, ..n.clone() }, pos);
            }
        }
    }

    // --- blink and charge ---------------------------------------------------------------------

    fn blink(&mut self, id: EntityId, def: &SpellDef, n: &Nums) {
        let (pos, facing_) = {
            let e = &self.entities[&id];
            (e.pos, e.facing)
        };
        let dist = if n.range > 0.0 { n.range } else { 18.0 };
        let (x, y) = (pos[0] + dist * facing_.cos(), pos[1] + dist * facing_.sin());
        let dest = [x, y, self.ground.height(x, y)];
        self.events.push(Event::SpellGo { caster: id, spell: def.visual(), target: SpellTarget::Unit(id), hits: vec![id] });
        if n.radius > 0.0 && (n.damage > 0.0 || n.freeze_chance > 0.0) {
            let info = self.hit_info(id, def, n);
            let base = self.spell_base(id, def, n);
            let hits: Vec<EntityId> = self.enemies_near(pos, n.radius).into_iter().map(|(v, _)| v).collect();
            if !hits.is_empty() {
                let look = if n.damage > 0.0 { 1449 } else { 122 };
                self.events.push(Event::SpellGo { caster: id, spell: look, target: SpellTarget::Point(pos), hits: hits.clone() });
            }
            for v in hits {
                self.land_hit(id, v, base, &info);
            }
        }
        if let Some(e) = self.entities.get_mut(&id) {
            e.pos = dest;
        }
        self.events.push(Event::Teleport { player: id, pos: dest, facing: facing_ });
        if n.speed_buff > 0.0 {
            self.give_buff(id, BuffKind::Accelerated, n.speed_secs.max(1.0), n.speed_buff, 1);
            self.update_speed(id);
        }
    }

    fn charge(&mut self, id: EntityId, def: &SpellDef, n: &Nums) -> Result<bool, CastError> {
        let pos = self.entities[&id].pos;
        let range = if n.range > 0.0 { n.range } else { 25.0 };
        // By hand it rushes the nearest enemy when nothing is chosen; on its own only at a
        // chosen one (so it never drags a warrior off into a pack).
        let manual = self.entities[&id].player().is_some_and(|p| p.manual);
        let target = self.chosen_enemy(id, pos, range).or_else(|| {
            if manual {
                self.enemies_near(pos, range).into_iter().find(|(_, d)| *d >= n.min_range).map(|(v, _)| v)
            } else {
                None
            }
        });
        let Some(t) = target else { return Ok(false) };
        let tpos = self.entities[&t].pos;
        if dist2d(pos, tpos) < n.min_range {
            return Err(CastError::TooClose);
        }
        // A rush, not a jump: the hero runs at the target (the client plays the run) and the blow
        // lands as they arrive, stopping just short of it on their side.
        let f = facing(pos, tpos);
        let back = 2.0;
        let d = dist2d(pos, tpos).max(0.1);
        let (x, y) = (tpos[0] - (tpos[0] - pos[0]) / d * back, tpos[1] - (tpos[1] - pos[1]) / d * back);
        let dest = [x, y, self.ground.height(x, y)];
        let secs = (dist2d(pos, dest) / CHARGE_SPEED).max(0.15);
        if let Some(e) = self.entities.get_mut(&id) {
            e.pos = dest;
            e.facing = f;
        }
        self.events.push(Event::Rush { player: id, from: pos, to: dest, secs });
        self.events.push(Event::SpellGo { caster: id, spell: def.visual(), target: SpellTarget::Unit(t), hits: vec![] });
        self.charges.push((id, t, self.clock + secs, def.clone(), n.clone()));
        self.gain_rage(id, n.rage);
        Ok(true)
    }

    /// Charges arriving: the blow (and Trample's shockwave) lands, the defenders rally.
    pub(super) fn step_charges(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.charges).into_iter().partition(|c| c.2 <= now);
        self.charges = keep;
        for (id, t, _, def, n) in due {
            if !self.entities.get(&id).is_some_and(|e| e.alive()) {
                continue;
            }
            self.charge_lands(id, t, &def, &n);
        }
    }

    fn charge_lands(&mut self, id: EntityId, t: EntityId, def: &SpellDef, n: &Nums) {
        let Some(tpos) = self.entities.get(&t).filter(|e| e.alive()).map(|e| e.pos) else { return };
        let info = self.hit_info(id, def, n);
        let base = self.spell_base(id, def, n);
        let victims: Vec<EntityId> = if n.radius > 0.0 {
            self.enemies_near(tpos, n.radius).into_iter().map(|(v, _)| v).collect()
        } else {
            vec![t]
        };
        for v in victims {
            self.land_hit(id, v, base, &info);
        }
        self.rally_defenders(id, t);
        self.attack(id, t);
    }

    // --- the hunter's pet ------------------------------------------------------------------------

    pub(super) fn has_pet(&self, id: EntityId) -> bool {
        self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| self.content.classes[p.class].key == "hunter")
    }

    pub(super) fn pet_of(&self, id: EntityId) -> Option<EntityId> {
        let p = self.entities.get(&id)?.player()?;
        p.pet.filter(|pet| self.entities.get(pet).is_some_and(|e| e.alive()))
    }

    fn pet_hurt(&self, id: EntityId) -> bool {
        match self.pet_of(id) {
            Some(p) => self.health_share(p) < 0.6,
            None => self.has_pet(id),
        }
    }

    pub(super) fn pet_crit(&self, id: EntityId) -> f32 {
        self.entities.get(&id).and_then(|e| e.player()).map_or(0.0, |p| p.stats.pet_crit)
    }

    /// The pet's hit: a share of its master's, with Beast Mastery and buffs.
    pub(super) fn pet_hit(&self, owner: EntityId) -> f32 {
        let Some(p) = self.entities.get(&owner).and_then(|e| e.player()) else { return 0.0 };
        let mut m = (1.0 + p.stats.pet_damage) * self.content.pet.damage;
        m *= 1.0 + self.buff_value(owner, BuffKind::BestialWrath);
        m *= 1.0 + self.buff_value(owner, BuffKind::PetFury);
        m *= 1.0 + self.buff_value(owner, BuffKind::FuriousHowl);
        if self.content.classes[p.class].key == "hunter" && p.stats.set_pieces >= 2 {
            m *= 1.2;
        }
        self.player_hit(owner) * m
    }

    /// Call the pet (at the start of a run, or when it's back).
    pub(super) fn summon_pet(&mut self, owner: EntityId) {
        if !self.has_pet(owner) || self.pet_of(owner).is_some() {
            return;
        }
        let Some(o) = self.entities.get(&owner) else { return };
        if !o.alive() {
            return;
        }
        let (pos, level, max) = (o.pos, o.level, o.max_health);
        let health_share = self.content.pet.health
            * (1.0 + self.entities[&owner].player().map_or(0.0, |p| p.stats.pet_health));
        let health = (max as f32 * health_share).round().max(1.0) as u32;
        let hit = self.pet_hit(owner);
        let id = self.alloc();
        let a = self.rng.gen::<f32>() * std::f32::consts::TAU;
        let at = self.ring(pos, 2.5, a);
        self.entities.insert(
            id,
            Entity {
                pos: at,
                facing: 0.0,
                health,
                max_health: health,
                level,
                body: Body::Ally(enemies::Ally {
                    owner,
                    until: f32::INFINITY,
                    mover: None,
                    engaged: None,
                    swing: 0.0,
                    think: 0.0,
                    hit,
                    pet: true,
                    blessed_until: 0.0,
                }),
            },
        );
        if let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) {
            p.pet = Some(id);
            p.pet_back_at = f32::INFINITY;
        }
        self.events.push(Event::SpawnAlly { id, owner, level, pos: at, facing: 0.0, health, pet: true });
    }

    /// Keep the pet's numbers in step with its master, and bring it back when it's time.
    pub(super) fn step_pets(&mut self) {
        let now = self.clock;
        for id in self.player_ids() {
            if !self.has_pet(id) {
                continue;
            }
            let alive = self.entities[&id].alive();
            match self.pet_of(id) {
                Some(pet) => {
                    let hit = self.pet_hit(id);
                    let regen = self.entities[&id].player().map_or(0.0, |p| p.stats.pet_regen);
                    if let Some(Body::Ally(a)) = self.entities.get_mut(&pet).map(|e| &mut e.body) {
                        a.hit = hit;
                    }
                    if regen > 0.0 && (now * 2.0).fract() < 0.03 {
                        let max = self.entities[&pet].max_health as f32;
                        self.heal(id, pet, 19579, (max * regen).round().max(1.0) as u32, false);
                    }
                }
                None => {
                    let back = self.entities[&id].player().unwrap().pet_back_at;
                    if alive && back <= now && self.combat_on() {
                        self.summon_pet(id);
                    }
                }
            }
        }
    }

    /// The pet fell: it comes back after a rest.
    pub(super) fn pet_fell(&mut self, pet: EntityId, owner: EntityId) {
        let now = self.clock;
        let rest = self.content.pet.respawn;
        let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) else { return };
        if p.pet != Some(pet) {
            return;
        }
        p.pet = None;
        p.pet_back_at = now + (rest - p.stats.respawn).max(3.0);
        let secs = (p.pet_back_at - now).round() as u32;
        self.tell(owner, format!("Your pet has fallen. It returns in {secs} seconds (or cast Mend Pet)."));
    }

    fn pet_strike(&mut self, id: EntityId, def: &SpellDef, n: &Nums) -> Result<bool, CastError> {
        let Some(pet) = self.pet_of(id) else { return Err(CastError::NoPet) };
        let pos = self.entities[&id].pos;
        let range = if n.range > 0.0 { n.range } else { 40.0 };
        let target = self.chosen_enemy(id, pos, range).or_else(|| {
            let ppos = self.entities[&pet].pos;
            self.enemies_near(ppos, range).first().map(|(v, _)| *v)
        });
        let Some(t) = target else { return Ok(false) };
        // The pet leaps to it.
        let tpos = self.entities[&t].pos;
        let ppos = self.entities[&pet].pos;
        if dist2d(ppos, tpos) > MELEE_RANGE + 1.0 {
            let d = dist2d(ppos, tpos).max(0.1);
            let (x, y) = (tpos[0] - (tpos[0] - ppos[0]) / d * 1.5, tpos[1] - (tpos[1] - ppos[1]) / d * 1.5);
            let dest = [x, y, self.ground.height(x, y)];
            self.halt_enemy(pet);
            if let Some(e) = self.entities.get_mut(&pet) {
                e.pos = dest;
            }
            self.events.push(Event::Stop { id: pet, pos: dest, facing: facing(dest, tpos) });
        }
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.pet_target = Some(t);
        }
        let mut info = self.hit_info(id, def, n);
        info.from_pet = true;
        let hit = self.pet_hit(id) / self.content.pet.damage.max(0.01);
        let base = hit * n.damage * (1.0 + n.power);
        info.dot_total = hit * n.dot * (1.0 + n.power);
        self.events.push(Event::SpellGo { caster: pet, spell: def.visual(), target: SpellTarget::Unit(t), hits: vec![t] });
        let mut victims = vec![t];
        if n.targets > 0.0 {
            victims.extend(self.enemies_near(tpos, 8.0).into_iter().map(|(v, _)| v).filter(|v| *v != t).take(n.targets as usize));
        }
        for v in victims {
            self.land_hit(pet, v, base, &info);
        }
        Ok(true)
    }

    // --- talents --------------------------------------------------------------------------------

    /// Talent points a player has to spend.
    /// Talents come from level-up cards now: there are no points to spend by hand.
    pub fn talent_points(&self, _id: EntityId) -> u32 {
        0
    }

    /// A talent taken from a card: straight to its full rank (and its spell, if it teaches one).
    pub(super) fn grant_talent(&mut self, id: EntityId, index: usize) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let kit = self.content.kit(p.class);
        let Some(t) = kit.talents.get(index) else { return };
        let (ranks, grants) = (t.ranks, t.grants);
        let had = p.talents.get(index).copied().unwrap_or(0);
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if p.talents.len() <= index {
            p.talents.resize(index + 1, 0);
        }
        p.talents[index] = ranks;
        if had == 0 {
            if let Some(spell) = grants {
                self.learn_spell(id, spell, true);
            }
        }
        let path = self.entities.get(&id).and_then(|e| e.player()).and_then(|p| self.content.kit(p.class).talents.get(index)).and_then(|t| t.path.clone());
        if let Some(path) = path {
            self.take_path(id, &path);
        }
        self.recompute(id);
        self.events.push(Event::Talents { player: id });
        self.events.push(Event::Bar { player: id });
    }

    /// Points spent in each tree.
    pub fn tree_points(&self, id: EntityId) -> [u32; 3] {
        let mut out = [0; 3];
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return out };
        let kit = self.content.kit(p.class);
        for (i, &r) in p.talents.iter().enumerate() {
            if let Some(t) = kit.talents.get(i) {
                out[t.tree.min(2)] += r as u32;
            }
        }
        out
    }

    /// The tree a player leans to: most points, else their first spell's.
    pub(super) fn main_tree(&self, id: EntityId) -> Option<usize> {
        let pts = self.tree_points(id);
        let best = (0..3).max_by_key(|&t| pts[t]).unwrap();
        if pts[best] > 0 {
            return Some(best);
        }
        let p = self.entities.get(&id)?.player()?;
        let first = p.spells.iter().find(|o| !o.talent)?;
        Some(self.content.kit(p.class).spells[first.spell].tree)
    }

    /// Spend a point on talent `index` of the player's class.
    pub fn learn_talent(&mut self, id: EntityId, index: usize) -> Result<(), &'static str> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Err("No such player.") };
        let kit = self.content.kit(p.class);
        let Some(t) = kit.talents.get(index) else { return Err("No such talent.") };
        let rank = p.talents.get(index).copied().unwrap_or(0);
        if rank >= t.ranks {
            return Err("That talent is already at its top rank.");
        }
        if self.talent_points(id) == 0 {
            return Err("Talents come from your level-up cards.");
        }
        let in_tree = self.tree_points(id)[t.tree.min(2)];
        if in_tree < crate::kit::tier_points(t.tier) {
            return Err("Spend more points in that tree first.");
        }
        if t.path.is_some() && p.path.is_some() {
            return Err("You've already chosen your weapons this run.");
        }
        let grants = if rank == 0 { t.grants } else { None };
        let path = if rank == 0 { t.path.clone() } else { None };
        let name = t.name.clone();
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return Err("No such player.") };
        if p.talents.len() <= index {
            p.talents.resize(index + 1, 0);
        }
        p.talents[index] += 1;
        let new_rank = p.talents[index];
        if let Some(spell) = grants {
            self.learn_spell(id, spell, true);
            let def = self.content.kit(self.entities[&id].player().unwrap().class).spells[spell].clone();
            self.tell(id, format!("You have learned |cff71d5ff{}|r: it's on your bar.", def.name));
        } else if new_rank == 1 {
            self.tell(id, format!("You have learned {name}."));
        }
        if let Some(path) = path {
            self.take_path(id, &path);
        }
        self.recompute(id);
        self.events.push(Event::Talents { player: id });
        self.events.push(Event::Bar { player: id });
        Ok(())
    }

    /// Talents come from level-up cards and stay for the run: nothing to take back.
    pub fn reset_talents(&mut self, _id: EntityId) -> Result<(), &'static str> {
        Err("Talents come from your level-up cards and stay for the run.")
    }

    /// Spend points on their own (players without the talent window, and bots): down the
    /// tree the build leans to, then the others.
    pub fn auto_talents(&mut self, id: EntityId) {
        let mut guard = 0;
        while self.talent_points(id) > 0 && guard < 80 {
            guard += 1;
            let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
            let kit = self.content.kit(p.class);
            let main = self.main_tree(id).unwrap_or(0);
            let mut order: Vec<usize> = (0..kit.talents.len()).collect();
            order.sort_by_key(|&i| {
                let t = &kit.talents[i];
                (if t.tree == main { 0 } else { 1 }, t.tier, t.col)
            });
            let pick = order.into_iter().find(|&i| {
                let t = &kit.talents[i];
                let rank = p.talents.get(i).copied().unwrap_or(0);
                rank < t.ranks && self.tree_points(id)[t.tree.min(2)] >= crate::kit::tier_points(t.tier)
            });
            match pick {
                Some(i) => {
                    if self.learn_talent(id, i).is_err() {
                        return;
                    }
                }
                None => return,
            }
        }
    }

    /// A player's talents, for the talent window: rank of each of the class's talents.
    pub fn talent_ranks(&self, id: EntityId) -> Vec<u8> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let n = self.content.kit(p.class).talents.len();
        (0..n).map(|i| p.talents.get(i).copied().unwrap_or(0)).collect()
    }
}

/// The look of a primed trap on the ground.
pub(super) const TRAP_LOOK: u32 = 13797;

/// Talent points earned by `level`: one a level from 2.
pub fn talent_points_at(level: u32) -> u32 {
    level.saturating_sub(1)
}

#[cfg(test)]
impl Sim {
    /// Ready to cast anything: full rage, Revenge open, a little hurt, the pet at hand.
    pub fn prime_for_test(&mut self, id: EntityId) {
        let now = self.clock;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.rage = 100.0;
            p.revenge_until = now + 5.0;
            p.gcd_until = 0.0;
        }
        if let Some(e) = self.entities.get_mut(&id) {
            e.health = (e.max_health as f32 * 0.6) as u32;
        }
        if let Some(pet) = self.pet_of(id) {
            if let Some(e) = self.entities.get_mut(&pet) {
                e.health = (e.max_health as f32 * 0.5) as u32;
            }
        } else {
            self.summon_pet(id);
        }
        // Execute: the target is nearly dead.
        let sel = self.entities[&id].player().and_then(|p| p.selected);
        let needs_low = self.entities[&id].player().is_some_and(|p| p.spells.iter().any(|o| o.nums.only_below > 0.0));
        if let (Some(t), true) = (sel, needs_low) {
            if let Some(e) = self.entities.get_mut(&t) {
                e.health = (e.max_health / 10).max(1);
            }
        }
    }

    /// Keeps a test player alive (bosses hit hard).
    /// A talent as a card gives it: the whole of it, if its row is open.
    pub fn talent_for_test(&mut self, id: EntityId, index: usize) -> Result<(), &'static str> {
        if !self.pick_available(id, super::cards::Pick::Talent(index)) {
            return Err("not available");
        }
        self.grant_talent(id, index);
        Ok(())
    }

    pub fn prime_hp_for_test(&mut self, id: EntityId) {
        if let Some(e) = self.entities.get_mut(&id) {
            e.health = e.max_health;
        }
    }

    pub fn tough_for_test(&mut self, id: EntityId) {
        if let Some(e) = self.entities.get_mut(&id) {
            e.max_health = 10_000_000;
            e.health = 10_000_000;
        }
    }

    pub fn owner_for_test(&self, id: EntityId) -> EntityId {
        self.owner_of(id)
    }

    pub fn hit_for_test(&self, id: EntityId) -> f32 {
        self.player_hit(id)
    }

    pub fn casts_for_test(&self, id: EntityId) -> u32 {
        self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.casts)
    }

    pub fn spell_nums_for_test(&self, id: EntityId, key: &str) -> Nums {
        let p = self.entities[&id].player().unwrap();
        let kit = self.content.kit(p.class);
        p.spells.iter().find(|o| kit.spells[o.spell].key == key).expect("held").nums.clone()
    }

    pub fn is_frozen_for_test(&self, id: EntityId) -> bool {
        self.enemy_frozen(id)
    }

    pub fn kit_for_test(&self, id: EntityId) -> crate::kit::Kit {
        let p = self.entities[&id].player().unwrap();
        self.content.kit(p.class).clone()
    }
}
