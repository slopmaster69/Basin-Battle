//! Cards: offers and picks (new spells, upgrades for spells you hold, legendaries), menus, the
//! buff bar, ground zones and missiles in flight.

use rand::Rng;

use super::enemies::Via;
use super::spells::BuffKind;
use super::*;
use crate::content::Rarity;
use crate::kit::{Nums, SpellKind};

/// A legendary (or other non-spell) card a player holds.
#[derive(Debug, Clone)]
pub(super) struct Owned {
    pub card: usize,
    pub rank: u8,
}

/// What a card in an offer is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Pick {
    /// A new spell of the player's class (index into the kit's spells).
    Spell(usize),
    /// An upgrade for a spell held: (spell, perk).
    Perk(usize, usize),
    /// A legendary card (index into the content's cards).
    Card(usize),
    /// A talent of the player's class, taken at its full rank (index into the kit's talents).
    Talent(usize),
    /// A power (index into the content's powers): an Escalation's legendary or a boss's.
    Power(usize),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OfferSource {
    #[allow(dead_code)]
    Wave,
    /// A boss fell (which boss, as an index into the content's bosses).
    Boss(usize),
    Relic,
    Legendary,
    /// The starting spell, before the first wave.
    Start,
    /// A second starting pick, from Renown.
    HeadStart,
    /// A level-up.
    Level,
    /// Pick a set piece of this tier.
    SetPiece { tier: u8 },
}

#[derive(Debug, Clone)]
pub(super) struct Offer {
    pub source: OfferSource,
    pub cards: Vec<Pick>,
}

/// What choosing a menu option does.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum MenuAction {
    Take(Pick),
    Reroll,
    GoldReroll(u32),
    OpenBanish,
    Banish(Pick),
    Skip(u32),
    OpenLock,
    Lock(Pick),
    Back,
    Piece { tier: u8, piece: u8 },
    Forge(super::bases::ForgeAction),
    Claim(usize),
    OpenCache(usize),
    Close,
}

/// A lasting ground effect.
#[derive(Debug)]
pub(super) struct Zone {
    pub caster: EntityId,
    pub spell: u32,
    pub radius: f32,
    pub until: f32,
    pub next_tick: f32,
    pub per_tick: f32,
    pub school: u8,
    pub slow: f32,
    /// An enemy's danger zone: it hurts players, not enemies.
    pub hostile: bool,
    /// A healing zone (Lightwell): share of max health a second to allies in it.
    pub heal: f32,
    /// A hostile zone that takes this share of a hero's max health a tick (0: `per_tick`).
    pub share: f32,
    /// A player's spell: what each tick carries.
    pub info: Option<Box<HitInfo>>,
    /// Marks of one cast (a line, a ring) share this number: a player standing in two of them
    /// is hit once. 0 for a lone zone.
    pub cast: u32,
}

impl Zone {
    /// Enemies' zones are danger circles; the heroes' own need no warning.
    pub(super) fn mark(&self) -> GroundMark {
        if self.hostile {
            GroundMark::Danger
        } else {
            GroundMark::None
        }
    }
}

/// What a spell's hit carries to where it lands.
#[derive(Debug, Clone)]
pub(super) struct HitInfo {
    pub nums: Box<Nums>,
    pub school: u8,
    pub spell: u32,
    /// The damage over time it leaves, worked out at the cast.
    pub dot_total: f32,
    /// A splash from another hit (no further splash).
    pub splash: bool,
    pub force_crit: bool,
    pub from_pet: bool,
    pub key_kind: SpellKind,
    /// The spell (kit index) it came from, for its upgrades' triggers.
    pub src: Option<usize>,
}

/// A missile on its way.
#[derive(Debug)]
pub(super) struct PendingHit {
    pub at: f32,
    pub caster: EntityId,
    pub victim: EntityId,
    pub spell: u32,
    pub damage: f32,
    pub crit: bool,
    pub school: u8,
    pub fx: HitFx,
    /// A class spell's hit (everything else uses `fx`).
    pub info: Option<Box<HitInfo>>,
}

/// What a simple missile (an Auto Shot, a Protector's bolt) does besides damage.
#[derive(Debug, Clone, Default)]
pub(super) struct HitFx {
    pub auto: bool,
}

impl HitFx {
    pub fn auto() -> Self {
        Self { auto: true }
    }
}

/// Class-style kits: the odds climb with level, and Epic cards turn up from level 10.
fn class_rarity_roll(source: OfferSource, level: u32, r: f32) -> Rarity {
    let table: [f32; 4] = match source {
        OfferSource::Wave | OfferSource::Level => match level {
            0..=9 => [0.70, 0.25, 0.05, 0.0],
            10..=24 => [0.58, 0.30, 0.10, 0.02],
            25..=39 => [0.50, 0.30, 0.15, 0.05],
            _ => [0.45, 0.30, 0.18, 0.07],
        },
        OfferSource::Boss(_) => [0.0, 0.30, 0.55, 0.15],
        OfferSource::Relic => [0.0, 0.30, 0.55, 0.15],
        _ => return rarity_roll(source, r),
    };
    let mut acc = 0.0;
    for (k, rar) in [Rarity::Common, Rarity::Uncommon, Rarity::Rare, Rarity::Epic].into_iter().enumerate() {
        acc += table[k];
        if r < acc {
            return rar;
        }
    }
    Rarity::Epic
}

/// Offers in a row without a Rare or better before the next one is sure to have one.
const RARE_PITY: u8 = 6;

fn rarity_roll(source: OfferSource, r: f32) -> Rarity {
    match source {
        OfferSource::Wave | OfferSource::Level => {
            if r < 0.58 {
                Rarity::Common
            } else if r < 0.9 {
                Rarity::Uncommon
            } else {
                Rarity::Rare
            }
        }
        OfferSource::Boss(_) => {
            if r < 0.4 {
                Rarity::Uncommon
            } else {
                Rarity::Rare
            }
        }
        OfferSource::Relic => {
            if r < 0.35 {
                Rarity::Uncommon
            } else {
                Rarity::Rare
            }
        }
        OfferSource::Legendary => Rarity::Legendary,
        OfferSource::Start | OfferSource::HeadStart | OfferSource::SetPiece { .. } => Rarity::Common,
    }
}

impl Sim {
    // --- offers -----------------------------------------------------------------------------------

    /// Every player's card pick for a cleared wave, and the drops.
    pub(super) fn wave_rewards(&mut self, boss: bool) {
        // Picks come with levels now; a boss leaves one of its powers behind.
        if boss {
            let every = self.content.scaling.boss_every.max(1);
            let index = (self.run.wave / every).max(1) as usize - 1;
            let index = index % self.content.bosses.len().max(1);
            for id in self.player_ids() {
                self.offer(id, OfferSource::Boss(index));
            }
        }
        self.wave_drops(boss);
    }

    pub(super) fn start_of_run_offers(&mut self) {
        for id in self.player_ids() {
            self.starting_offers(id);
        }
    }

    pub(super) fn starting_offers(&mut self, id: EntityId) {
        self.offer(id, OfferSource::Start);
        let head_start = self.entities[&id].player().is_some_and(|p| p.luck.head_start);
        if head_start {
            self.offer(id, OfferSource::HeadStart);
        }
    }

    /// Queue an offer for a player, showing it if nothing else is open. Level-up picks that come
    /// during a wave wait for the wave to end (or for the player to open them), so a pick never
    /// lands under keys being pressed to cast.
    pub(super) fn offer(&mut self, id: EntityId, source: OfferSource) {
        // A set piece goes straight to the slot it improves most: no menu for that.
        if let OfferSource::SetPiece { tier } = source {
            self.auto_set_piece(id, tier);
            return;
        }
        let cards = match source {
            OfferSource::SetPiece { .. } => Vec::new(),
            _ => self.roll_offer(id, source),
        };
        if cards.is_empty() && !matches!(source, OfferSource::SetPiece { .. }) {
            // A finished build: every slot full and every spell fully upgraded.
            if !matches!(source, OfferSource::Start | OfferSource::HeadStart) {
                let gold = 25 + 5 * self.run.wave;
                self.give_gold(id, gold);
                self.tell(id, format!("Your build is complete: {gold} gold instead of a card."));
            }
            return;
        }
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        p.offers.push_back(Offer { source, cards });
        // Show it now unless another kind of window (the forge, a banner) is open; a card window
        // the player may have closed is sent again.
        let card_window = p.menu.as_ref().map_or(true, |m| {
            matches!(m.kind, MenuKind::Card | MenuKind::Relic | MenuKind::Legendary | MenuKind::SetPiece)
        });
        if card_window {
            if self.clock < p.menu_hold {
                p.menu_due = true;
            } else {
                self.show_next_offer(id);
            }
        }
    }

    /// Show offers that waited out a hold.
    pub(super) fn step_menus(&mut self) {
        for id in self.player_ids() {
            let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { continue };
            if !p.menu_due || self.clock < p.menu_hold {
                continue;
            }
            p.menu_due = false;
            let card_window = p.menu.as_ref().map_or(true, |m| m.offer.is_some() || m.kind == MenuKind::SetPiece);
            if card_window && !p.offers.is_empty() {
                self.show_next_offer(id);
            }
        }
    }

    /// The wave is over: picks that waited for it come up.
    pub(super) fn show_waiting_offers(&mut self) {
        for id in self.player_ids() {
            let waiting = self.entities[&id].player().is_some_and(|p| !p.offers.is_empty() && p.menu.is_none());
            if waiting {
                self.show_next_offer(id);
            }
        }
    }

    pub(super) fn pick_available(&self, id: EntityId, pick: Pick) -> bool {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return false };
        if p.banished.contains(&pick) {
            return false;
        }
        let kit = self.content.kit(p.class);
        match pick {
            Pick::Spell(s) => {
                let def = &kit.spells[s];
                // A path's skills come only to warriors on that path.
                let path_ok = def.path.as_ref().is_none_or(|need| p.path.as_ref() == Some(need));
                // They ride on extra keys, so a full bar never shuts them out.
                let room = if def.path.is_some() { p.spells.len() < 12 } else { self.card_spells(id) < self.content.game.spell_slots };
                path_ok && !def.talent && !p.spells.iter().any(|o| o.spell == s) && room
            }
            Pick::Perk(s, k) => p.spells.iter().find(|o| o.spell == s).is_some_and(|o| !o.talent && kit.spells[s].perk_open(&o.perks, k, kit.upgrades_per_spell)),
            Pick::Card(c) => {
                let max = self.content.cards[c].max_rank();
                !p.cards.iter().any(|o| o.card == c && o.rank >= max)
            }
            Pick::Power(k) => {
                let Some(def) = self.content.powers.get(k) else { return false };
                !p.powers.contains(&k) && def.class.is_none_or(|c| c == p.class)
            }
            Pick::Talent(t) => {
                let Some(def) = kit.talents.get(t) else { return false };
                let rank = p.talents.get(t).copied().unwrap_or(0);
                // A talent that teaches a spell needs a free key for it.
                let teaches = def.grants.is_some_and(|g| !p.spells.iter().any(|o| o.spell == g));
                let room = !teaches || self.card_spells(id) < self.content.game.spell_slots;
                let holds = |s: usize| p.spells.iter().any(|o| o.spell == s);
                // A combination talent needs all its spells held.
                let combo = def.requires.iter().all(|&s| holds(s));
                // One that only reshapes spells is offered only to a hero holding one it reshapes
                // (no Thunder Clap talents without Thunder Clap).
                let relevant = def.grants.is_some()
                    || !def.only_patches_spells()
                    || p.spells.iter().any(|o| def.spell_filters().iter().any(|f| f.matches(&kit.spells[o.spell])));
                // One weapon path a run.
                let path_open = def.path.is_none() || p.path.is_none();
                // Class talents open whatever's been taken; tree talents tier by tier.
                let tier_open = kit.class_talents || self.tree_points(id)[def.tree.min(2)] >= crate::kit::tier_points(def.tier);
                // Path cards: on that path only; a fork closes its siblings; a follow-up waits for
                // its first step; a capstone for enough of the path's cards.
                let has = |i: usize| p.talents.get(i).copied().unwrap_or(0) > 0;
                let on_path = def.path_only.as_ref().is_none_or(|need| p.path.as_ref() == Some(need));
                let fork_open = def.fork.as_ref().is_none_or(|f| {
                    !kit.talents.iter().enumerate().any(|(i, o)| i != t && has(i) && o.fork.as_ref() == Some(f))
                });
                let after_ok = def.after.is_none_or(has);
                let needs_ok = def.needs == 0
                    || kit
                        .talents
                        .iter()
                        .enumerate()
                        .filter(|(_, o)| o.path_only.is_some() && o.path_only == def.path_only)
                        .map(|(i, _)| p.talents.get(i).copied().unwrap_or(0) as usize)
                        .sum::<usize>()
                        >= def.needs as usize;
                room && combo && relevant && path_open && rank < def.ranks && tier_open && on_path && fork_open && after_ok && needs_ok
            }
        }
    }

    fn pick_rarity(&self, id: EntityId, pick: Pick) -> Rarity {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Rarity::Common };
        let kit = self.content.kit(p.class);
        match pick {
            Pick::Spell(s) => kit.spells[s].rarity,
            Pick::Perk(s, k) => kit.spells[s].perks[k].rarity,
            Pick::Card(c) => self.content.cards[c].rarity,
            Pick::Power(k) => self.content.powers[k].rarity,
            Pick::Talent(t) if kit.talents[t].rarity.is_some() => kit.talents[t].rarity.unwrap(),
            Pick::Talent(t) if kit.class_talents => {
                if kit.talents[t].requires.len() >= 2 || kit.talents[t].path.is_some() {
                    Rarity::Rare
                } else {
                    Rarity::Uncommon
                }
            }
            Pick::Talent(t) => match kit.talents[t].tier {
                0..=2 => Rarity::Common,
                3 => Rarity::Uncommon,
                4 => Rarity::Rare,
                _ => Rarity::Epic,
            },
        }
    }

    fn pick_tree(&self, id: EntityId, pick: Pick) -> Option<usize> {
        let p = self.entities.get(&id)?.player()?;
        let kit = self.content.kit(p.class);
        match pick {
            Pick::Spell(s) | Pick::Perk(s, _) => Some(kit.spells[s].tree),
            Pick::Talent(t) => Some(kit.talents[t].tree),
            Pick::Card(_) | Pick::Power(_) => None,
        }
    }

    fn roll_offer(&mut self, id: EntityId, source: OfferSource) -> Vec<Pick> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let n = if p.luck.wider { 4 } else { 3 };
        let kit = self.content.kit(p.class).clone();
        let locked = p.locked;
        let mut picks: Vec<Pick> = Vec::new();
        if let Some(l) = locked {
            if self.pick_available(id, l) {
                picks.push(l);
            }
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.locked = None;
            }
        }
        // The first spell: the class's core spells, one per tree where it can.
        if matches!(source, OfferSource::Start | OfferSource::HeadStart) {
            let mut core: Vec<usize> =
                (0..kit.spells.len()).filter(|&s| kit.spells[s].core && self.pick_available(id, Pick::Spell(s))).collect();
            let mut trees_seen = Vec::new();
            let mut out = Vec::new();
            for _ in 0..core.len() {
                if out.len() >= n || core.is_empty() {
                    break;
                }
                let fresh: Vec<usize> = core.iter().copied().filter(|s| !trees_seen.contains(&kit.spells[*s].tree)).collect();
                let from = if fresh.is_empty() { core.clone() } else { fresh };
                let s = from[self.rng.gen_range(0..from.len())];
                trees_seen.push(kit.spells[s].tree);
                core.retain(|x| *x != s);
                out.push(Pick::Spell(s));
            }
            return out;
        }
        if source == OfferSource::Legendary {
            // The legendary cards and the Escalation powers, the class's own twice as likely.
            // (One already held ranks up, but new ones come first: no single best pick to stack.)
            let held: Vec<usize> = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.cards.iter().map(|o| o.card).collect()).unwrap_or_default();
            let mut pool: Vec<(Pick, f32)> = (0..self.content.cards.len())
                .filter(|&c| self.content.cards[c].rarity == Rarity::Legendary && self.pick_available(id, Pick::Card(c)))
                .map(|c| (Pick::Card(c), if held.contains(&c) { 0.3 } else { 1.0 }))
                .collect();
            pool.extend(
                (0..self.content.powers.len())
                    .filter(|&k| self.content.powers[k].source == crate::powers::PowerSource::Escalation && self.pick_available(id, Pick::Power(k)))
                    .map(|k| (Pick::Power(k), if self.content.powers[k].class.is_some() { 2.0 } else { 1.0 })),
            );
            self.draw_weighted(&mut picks, pool, n);
            return picks;
        }
        if let OfferSource::Boss(boss) = source {
            // This boss's own power (always, while it's to be had), then powers any boss leaves:
            // the class's own twice as likely.
            let key = self.content.bosses.get(boss).map(|b| b.key.clone()).unwrap_or_default();
            let own: Vec<usize> = (0..self.content.powers.len())
                .filter(|&k| self.content.powers[k].boss.as_deref() == Some(key.as_str()) && self.pick_available(id, Pick::Power(k)))
                .collect();
            for k in own {
                if picks.len() < n {
                    picks.push(Pick::Power(k));
                }
            }
            let pool: Vec<(Pick, f32)> = (0..self.content.powers.len())
                .filter(|&k| {
                    let d = &self.content.powers[k];
                    d.source == crate::powers::PowerSource::Boss && d.boss.is_none() && self.pick_available(id, Pick::Power(k)) && !picks.contains(&Pick::Power(k))
                })
                .map(|k| (Pick::Power(k), if self.content.powers[k].class.is_some() { 2.0 } else { 1.0 }))
                .collect();
            self.draw_weighted(&mut picks, pool, n);
            if !picks.is_empty() {
                return picks;
            }
            // Every power taken: an ordinary rare card, as before.
        }
        if kit.class_talents {
            self.roll_class_offer(id, source, &kit, &mut picks, n);
            return picks;
        }
        let main = self.main_tree(id);
        let held = self.card_spells(id);
        let mut candidates: Vec<Pick> = Vec::new();
        // Talents come every other level (a choice to make, not a tree to fill), and with any
        // other kind of pick.
        let level = self.entities.get(&id).and_then(|e| e.player()).map_or(1, |p| p.level);
        if source != OfferSource::Level || level % 2 == 0 {
            for t in 0..kit.talents.len() {
                if self.pick_available(id, Pick::Talent(t)) {
                    candidates.push(Pick::Talent(t));
                }
            }
        }
        // A level-up always shows a talent (while any are left) and something for your spells.
        if source == OfferSource::Level {
            let talents: Vec<Pick> = candidates.iter().copied().filter(|c| matches!(c, Pick::Talent(_)) && !picks.contains(c)).collect();
            if !talents.is_empty() && !picks.iter().any(|c| matches!(c, Pick::Talent(_))) {
                // The main tree's first; combinations and weapon paths, when they open, most of all.
                let w: Vec<f32> = talents
                    .iter()
                    .map(|c| {
                        let special = match c {
                            Pick::Talent(t) => !kit.talents[*t].requires.is_empty() || kit.talents[*t].path.is_some(),
                            _ => false,
                        };
                        let tree = if main.is_some() && self.pick_tree(id, *c) == main { 2.5 } else { 1.0 };
                        tree * if special { 4.0 } else { 1.0 }
                    })
                    .collect();
                let total: f32 = w.iter().sum();
                let mut roll = self.rng.gen::<f32>() * total;
                let mut chosen = talents[0];
                for (c, x) in talents.iter().zip(&w) {
                    if roll < *x {
                        chosen = *c;
                        break;
                    }
                    roll -= x;
                }
                picks.push(chosen);
            }
        }
        for s in 0..kit.spells.len() {
            if self.pick_available(id, Pick::Spell(s)) {
                candidates.push(Pick::Spell(s));
            }
            for k in 0..kit.spells[s].perks.len() {
                if self.pick_available(id, Pick::Perk(s, k)) {
                    candidates.push(Pick::Perk(s, k));
                }
            }
        }
        let mut guard = 0;
        while picks.len() < n && guard < 60 {
            guard += 1;
            let want = rarity_roll(source, self.rng.gen::<f32>());
            let rarity_pool: Vec<Pick> = candidates.iter().copied().filter(|c| !picks.contains(c) && self.pick_rarity(id, *c) == want).collect();
            let pool: Vec<Pick> = if rarity_pool.is_empty() {
                candidates.iter().copied().filter(|c| !picks.contains(c)).collect()
            } else {
                rarity_pool
            };
            if pool.is_empty() {
                break;
            }
            // The first slot of an early offer is a new spell; later offers lean on upgrades,
            // and everything leans towards the tree the build is going down.
            let first = picks.is_empty();
            let weights: Vec<f32> = pool
                .iter()
                .map(|c| {
                    let mut w = 1.0;
                    match c {
                        Pick::Spell(_) => {
                            if held < 3 {
                                w *= if first { 6.0 } else { 2.0 };
                            } else if held < 5 {
                                w *= 1.2;
                            }
                        }
                        Pick::Perk(..) => {
                            if held >= 3 {
                                w *= 1.6;
                            }
                        }
                        Pick::Card(_) | Pick::Power(_) => {}
                        // Talents mostly come one to a level-up (above).
                        Pick::Talent(_) => w *= 0.5,
                    }
                    if main.is_some() && self.pick_tree(id, *c) == main {
                        w *= 1.8;
                    }
                    w
                })
                .collect();
            let total: f32 = weights.iter().sum();
            let mut roll = self.rng.gen::<f32>() * total;
            let mut pick = pool[0];
            for (c, w) in pool.iter().zip(&weights) {
                if roll < *w {
                    pick = *c;
                    break;
                }
                roll -= w;
            }
            // One upgrade for each spell at most in a single offer, so there's a real choice.
            if let Pick::Perk(s, _) = pick {
                if picks.iter().any(|x| matches!(x, Pick::Perk(t, _) if *t == s)) && pool.len() > 3 {
                    continue;
                }
            }
            picks.push(pick);
        }
        picks
    }

    /// A class-talent kit's offer: new spells while there's room, upgrades for the spells held
    /// (most of the cards), class talents, and combinations when two spells that go together are
    /// held. At most one upgrade per spell and one talent of each kind in an offer.
    fn roll_class_offer(&mut self, id: EntityId, source: OfferSource, kit: &crate::kit::Kit, picks: &mut Vec<Pick>, n: usize) {
        let held = self.card_spells(id);
        let slots = self.content.game.spell_slots;
        let level = self.entities.get(&id).and_then(|e| e.player()).map_or(1, |p| p.level);
        let mut candidates: Vec<Pick> = Vec::new();
        for s in 0..kit.spells.len() {
            if self.pick_available(id, Pick::Spell(s)) {
                candidates.push(Pick::Spell(s));
            }
            for k in 0..kit.spells[s].perks.len() {
                if self.pick_available(id, Pick::Perk(s, k)) {
                    candidates.push(Pick::Perk(s, k));
                }
            }
        }
        for t in 0..kit.talents.len() {
            if self.pick_available(id, Pick::Talent(t)) {
                candidates.push(Pick::Talent(t));
            }
        }
        let is_combo = |p: &Pick| matches!(p, Pick::Talent(t) if kit.talents[*t].requires.len() >= 2 || kit.talents[*t].path.is_some());
        let is_class = |p: &Pick| matches!(p, Pick::Talent(t) if kit.talents[*t].requires.len() < 2 && kit.talents[*t].path.is_none());
        let mut guard = 0;
        while picks.len() < n && guard < 80 {
            guard += 1;
            let mut want = class_rarity_roll(source, level, self.rng.gen::<f32>());
            // The safety net: a long dry run of commons ends with a Rare (or better) for sure.
            let dry = self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.since_rare);
            if picks.is_empty() && dry >= RARE_PITY && want < Rarity::Rare && matches!(source, OfferSource::Level | OfferSource::Wave) {
                want = Rarity::Rare;
            }
            let open: Vec<Pick> = candidates
                .iter()
                .copied()
                .filter(|c| !picks.contains(c))
                // One upgrade per spell, one class talent and one combination per offer.
                .filter(|c| match c {
                    Pick::Perk(s, _) => !picks.iter().any(|x| matches!(x, Pick::Perk(t, _) if t == s)),
                    _ if is_combo(c) => !picks.iter().any(&is_combo),
                    _ if is_class(c) => !picks.iter().any(&is_class),
                    _ => true,
                })
                .collect();
            if open.is_empty() {
                break;
            }
            // Skills come by what the bar needs, not by luck: rarity filters the rest.
            let rarity_pool: Vec<Pick> = open
                .iter()
                .copied()
                .filter(|c| {
                    // A weapon path shows as a Legendary but comes as often as a Rare.
                    let r = match c {
                        Pick::Talent(t) if kit.talents[*t].path.is_some() => Rarity::Rare,
                        _ => self.pick_rarity(id, *c),
                    };
                    matches!(c, Pick::Spell(_)) || r == want || (want == Rarity::Rare && dry >= RARE_PITY && r > Rarity::Rare)
                })
                .collect();
            let pool = if rarity_pool.is_empty() { open } else { rarity_pool };
            let first = picks.is_empty();
            let weights: Vec<f32> = pool
                .iter()
                .map(|c| match c {
                    // A path's own skills come soon after the path.
                    Pick::Spell(s) if kit.spells[*s].path.is_some() => 4.0,
                    Pick::Spell(_) => {
                        if held < 3 {
                            if first { 8.0 } else { 2.5 }
                        } else if held < slots {
                            // The bar should fill by the mid-teens.
                            if level >= 12 { 2.0 } else { 1.0 }
                        } else {
                            0.0
                        }
                    }
                    Pick::Perk(s, k) => {
                        let perk = &kit.spells[*s].perks[*k];
                        let plain = perk.triggers.is_empty() && perk.repeat > 1;
                        let chained = perk.after.is_some();
                        // Each held spell gets about the same share of the cards, however many
                        // upgrades it has: spread over the spell's open upgrades.
                        let open_here = candidates.iter().filter(|x| matches!(x, Pick::Perk(t, _) if t == s)).count().max(1) as f32;
                        let share = 6.0 / open_here;
                        share * if plain { 0.5 } else if chained { 1.4 } else { 1.0 }
                    }
                    _ if is_combo(c) => 5.0,
                    // The path's own deck: about as often as a held skill's.
                    Pick::Talent(t) if kit.talents[*t].path_only.is_some() => 2.0,
                    Pick::Talent(_) => 1.0,
                    Pick::Card(_) | Pick::Power(_) => 1.0,
                })
                .collect();
            let total: f32 = weights.iter().sum();
            if total <= 0.0 {
                break;
            }
            let mut roll = self.rng.gen::<f32>() * total;
            let mut pick = pool[0];
            for (c, w) in pool.iter().zip(&weights) {
                if roll < *w {
                    pick = *c;
                    break;
                }
                roll -= w;
            }
            picks.push(pick);
        }
        // The safety net's count: offers since the last Rare or better.
        if matches!(source, OfferSource::Level | OfferSource::Wave) {
            let rare = picks.iter().any(|c| !matches!(c, Pick::Spell(_)) && self.pick_rarity(id, *c) >= Rarity::Rare);
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.since_rare = if rare { 0 } else { p.since_rare.saturating_add(1) };
            }
        }
    }

    /// Show the player's next queued offer, or close the menu if there's none.
    pub(super) fn show_next_offer(&mut self, id: EntityId) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let Some(offer) = p.offers.front().cloned() else {
            self.close_menu(id);
            return;
        };
        match offer.source {
            OfferSource::SetPiece { tier } => self.show_piece_menu(id, tier),
            _ => self.show_card_menu(id, &offer),
        }
    }

    /// Draw up to `n` picks in all from `pool` (pick, weight) into `picks`, without repeats.
    fn draw_weighted(&mut self, picks: &mut Vec<Pick>, mut pool: Vec<(Pick, f32)>, n: usize) {
        pool.retain(|(p, _)| !picks.contains(p));
        while picks.len() < n && !pool.is_empty() {
            let total: f32 = pool.iter().map(|(_, w)| w).sum();
            let mut roll = self.rng.gen::<f32>() * total;
            let mut k = pool.len() - 1;
            for (i, (_, w)) in pool.iter().enumerate() {
                if roll < *w {
                    k = i;
                    break;
                }
                roll -= w;
            }
            picks.push(pool.remove(k).0);
        }
    }

    /// How a pick reads on a card.
    pub(super) fn pick_view(&self, id: EntityId, pick: Pick, color: bool) -> CardView {
        let p = self.entities[&id].player().unwrap();
        let kit = self.content.kit(p.class);
        let hit = self.player_hit(id);
        match pick {
            Pick::Power(k) => {
                let def = &self.content.powers[k];
                let boss = def.source == crate::powers::PowerSource::Boss;
                CardView {
                    key: format!("power:{}", def.key),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    rarity: def.rarity,
                    rank: 1,
                    max_rank: 1,
                    text: crate::describe::CardText {
                        text: def.text.clone(),
                        upgrade: String::new(),
                        kind: if boss { "Boss power" } else { "Legendary" },
                        school: "",
                        cooldown: 0.0,
                    },
                    tags: def.class.map(|c| vec![super::shop::capitalize(&self.content.classes[c].key)]).unwrap_or_default(),
                    slot: if boss { "Boss power · lasts the run".into() } else { "Legendary · lasts the run".into() },
                }
            }
            Pick::Spell(s) => {
                let def = &kit.spells[s];
                let nums = self.preview_nums(id, s, None);
                CardView {
                    key: def.key.clone(),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    rarity: def.rarity,
                    rank: 1,
                    max_rank: (kit.upgrades_per_spell + 1) as u8,
                    text: crate::describe::spell_text(def, &nums, hit, color),
                    tags: vec![kit.trees[def.tree].name.clone()],
                    slot: self.slot_line(id, pick),
                }
            }
            Pick::Perk(s, k) => {
                let def = &kit.spells[s];
                let perk = &def.perks[k];
                let held = p.spells.iter().find(|o| o.spell == s);
                let rank = held.map_or(1, |o| o.perks.len() as u8 + 1) + 1;
                CardView {
                    key: format!("{}:{}", def.key, perk.key),
                    name: perk.name.clone(),
                    icon_spell: def.icon,
                    rarity: perk.rarity,
                    rank,
                    max_rank: (kit.upgrades_per_spell + 1) as u8,
                    text: crate::describe::perk_text_with(def, perk, rank, kit.upgrade_power, kit.upgrade_cost_cut, color),
                    tags: vec![kit.trees[def.tree].name.clone()],
                    slot: self.slot_line(id, pick),
                }
            }
            Pick::Talent(t) if kit.class_talents => {
                let def = &kit.talents[t];
                // (The card's kind is a fixed word: the two a class may call its paths.)
                let label = match (def.path.is_some(), kit.path_label.as_str()) {
                    (true, "Specialization") => "Specialization",
                    (true, _) => "Weapon Path",
                    _ => def.kind_label(),
                };
                let mut tags: Vec<String> = def.requires.iter().map(|&s| kit.spells[s].name.clone()).collect();
                if tags.is_empty() {
                    tags.push(super::shop::capitalize(&kit.class));
                }
                CardView {
                    key: format!("talent:{}", def.key),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    rarity: self.pick_rarity(id, pick),
                    // Class talents come back a rank at a time.
                    rank: p.talents.get(t).copied().unwrap_or(0) + 1,
                    max_rank: def.ranks,
                    text: crate::describe::CardText {
                        text: def.text_at(p.talents.get(t).copied().unwrap_or(0) + 1),
                        upgrade: String::new(),
                        kind: label,
                        school: "",
                        cooldown: 0.0,
                    },
                    tags,
                    slot: self.slot_line(id, pick),
                }
            }
            Pick::Talent(t) => {
                let def = &kit.talents[t];
                let tree = kit.trees[def.tree.min(2)].name.clone();
                CardView {
                    key: format!("talent:{}", def.key),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    rarity: self.pick_rarity(id, pick),
                    rank: 1,
                    max_rank: 1,
                    text: crate::describe::CardText {
                        text: def.text_at(def.ranks),
                        upgrade: String::new(),
                        kind: "Talent",
                        school: "",
                        cooldown: 0.0,
                    },
                    tags: vec![tree.clone()],
                    slot: format!("Talent · {tree} tree"),
                }
            }
            Pick::Card(c) => {
                let def = &self.content.cards[c];
                let rank = p.cards.iter().find(|o| o.card == c).map_or(0, |o| o.rank) + 1;
                CardView {
                    key: def.key.clone(),
                    name: def.name.clone(),
                    icon_spell: def.spell,
                    rarity: def.rarity,
                    rank,
                    max_rank: def.max_rank(),
                    text: crate::describe::card_text(def, rank, hit, 1.0, color),
                    tags: def.tags.clone(),
                    slot: "Legendary".into(),
                }
            }
        }
    }

    /// A spell's numbers as they'd be with an extra perk, for offers.
    fn preview_nums(&self, id: EntityId, spell: usize, _perk: Option<usize>) -> Nums {
        let p = self.entities[&id].player().unwrap();
        if let Some(o) = p.spells.iter().find(|o| o.spell == spell) {
            return o.nums.clone();
        }
        // A spell not yet held: base numbers with talents on.
        let kit = self.content.kit(p.class);
        let def = &kit.spells[spell];
        let mut n = def.nums.clone();
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
        for e in self.worn_ware_effects(p) {
            if e.filter.as_ref().is_some_and(|f| f.matches(def)) {
                e.patch.apply(&mut n, 1.0);
            }
        }
        n
    }

    fn pick_name(&self, id: EntityId, pick: Pick) -> String {
        let p = self.entities[&id].player().unwrap();
        let kit = self.content.kit(p.class);
        match pick {
            Pick::Spell(s) => kit.spells[s].name.clone(),
            Pick::Perk(s, k) => format!("{}: {}", kit.spells[s].name, kit.spells[s].perks[k].name),
            Pick::Card(c) => self.content.cards[c].name.clone(),
            Pick::Power(k) => self.content.powers[k].name.clone(),
            Pick::Talent(t) => kit.talents[t].name.clone(),
        }
    }

    fn show_card_menu(&mut self, id: EntityId, offer: &Offer) {
        let mut options = Vec::new();
        let mut actions = Vec::new();
        let mut cards = Vec::new();
        for &c in &offer.cards {
            let view = self.pick_view(id, c, true);
            let plain = self.pick_view(id, c, false);
            let label = if plain.rank > 1 { format!(" ({})", plain.text.upgrade) } else { String::new() };
            options.push(MenuOption { icon: 3, text: format!("|c{}{}|r{}: {}", view.rarity.color(), view.name, label, plain.text.text) });
            actions.push(MenuAction::Take(c));
            cards.push(view);
        }
        let p = self.entities[&id].player().unwrap();
        let gold_cost = 25 * (1 << p.gold_rerolls.min(8));
        let mut reroll = None;
        if p.rerolls > 0 {
            reroll = Some((options.len(), RerollCost::Free(p.rerolls)));
            options.push(MenuOption { icon: 4, text: format!("Reroll ({} left)", p.rerolls) });
            actions.push(MenuAction::Reroll);
        } else if !matches!(offer.source, OfferSource::Legendary | OfferSource::Start) {
            reroll = Some((options.len(), RerollCost::Gold(gold_cost)));
            options.push(MenuOption { icon: 6, text: format!("Reroll for {gold_cost} gold") });
            actions.push(MenuAction::GoldReroll(gold_cost));
        }
        if p.banishes > 0 {
            options.push(MenuOption { icon: 0, text: format!("Banish a card... ({} left)", p.banishes) });
            actions.push(MenuAction::OpenBanish);
        }
        let can_lock = p.luck.lock && p.locked.is_none();
        if can_lock {
            options.push(MenuOption { icon: 0, text: "Lock a card for next time...".into() });
            actions.push(MenuAction::OpenLock);
        }
        let mut skip = None;
        if p.skips > 0 {
            let gold = 20 + 5 * self.run.wave;
            skip = Some((options.len(), gold, p.skips));
            options.push(MenuOption { icon: 6, text: format!("Skip for {gold} gold ({} left)", p.skips) });
            actions.push(MenuAction::Skip(gold));
        }
        let (kind, title, offer_kind) = match offer.source {
            OfferSource::Relic => (MenuKind::Relic, "Relic: choose a card", OfferKind::Relic),
            OfferSource::Legendary => (MenuKind::Legendary, "Escalation: choose a legendary", OfferKind::Legendary),
            OfferSource::Start => (MenuKind::Card, "Choose your first spell", OfferKind::Start),
            OfferSource::HeadStart => (MenuKind::Card, "Head start: choose a second spell", OfferKind::HeadStart),
            OfferSource::Level => (MenuKind::Card, "Level up! Choose a card", OfferKind::Level),
            OfferSource::Boss(_) => (MenuKind::Card, "Boss defeated! Choose a power", OfferKind::Boss),
            _ => (MenuKind::Card, "Choose a card", OfferKind::Wave),
        };
        let view = OfferView {
            kind: offer_kind,
            cards,
            reroll,
            skip,
            banishes: p.banishes,
            can_lock,
            more: p.offers.len().saturating_sub(1),
            level: p.level,
            wave: self.run.wave,
            gold: p.gold,
        };
        self.open_menu(id, Menu { kind, source: id, title: title.into(), options, offer: Some(view) }, actions);
    }

    /// The spells (and legendaries) a player holds, described as they stand.
    pub fn held_cards(&self, id: EntityId) -> Vec<HeldCard> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let kit = self.content.kit(p.class);
        let hit = self.player_hit(id);
        let mut out: Vec<HeldCard> = self
            .bar_order(id)
            .into_iter()
            .map(|i| {
                let o = &p.spells[i];
                let def = &kit.spells[o.spell];
                let perks: Vec<String> = o.perks.iter().map(|&k| def.perks[k].name.clone()).collect();
                let next = if o.talent {
                    "Learned from a talent.".to_string()
                } else if perks.is_empty() {
                    format!("No upgrades yet: cards can upgrade it {} times.", kit.upgrades_per_spell)
                } else {
                    format!("Upgrades: {}", perks.join(", "))
                };
                HeldCard {
                    key: def.key.clone(),
                    name: def.name.clone(),
                    icon_spell: def.icon,
                    aura_spell: def.icon,
                    rarity: def.rarity,
                    rank: o.perks.len() as u8 + 1,
                    max_rank: if o.talent { 1 } else { (kit.upgrades_per_spell + 1) as u8 },
                    text: crate::describe::spell_text(def, &o.nums, hit, true),
                    next,
                }
            })
            .collect();
        for &k in &p.powers {
            let Some(def) = self.content.powers.get(k) else { continue };
            out.push(HeldCard {
                key: format!("power:{}", def.key),
                name: def.name.clone(),
                icon_spell: def.icon,
                aura_spell: def.buff_spell(),
                rarity: def.rarity,
                rank: 1,
                max_rank: 1,
                text: crate::describe::CardText {
                    text: def.text.clone(),
                    upgrade: String::new(),
                    kind: if def.source == crate::powers::PowerSource::Boss { "Boss power" } else { "Legendary" },
                    school: "",
                    cooldown: 0.0,
                },
                next: String::new(),
            });
        }
        for o in &p.cards {
            let def = &self.content.cards[o.card];
            out.push(HeldCard {
                key: def.key.clone(),
                name: def.name.clone(),
                icon_spell: def.spell,
                aura_spell: def.aura(),
                rarity: def.rarity,
                rank: o.rank,
                max_rank: def.max_rank(),
                text: crate::describe::card_text(def, o.rank, hit, 1.0, true),
                next: String::new(),
            });
        }
        out
    }

    /// "Takes a spell slot · 3/6" for a new spell, "Upgrade 2 of 4 for Frostbolt" for a perk.
    fn slot_line(&self, id: EntityId, pick: Pick) -> String {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return String::new() };
        let kit = self.content.kit(p.class);
        match pick {
            Pick::Spell(s) if kit.spells[s].path.is_some() => {
                format!("{} skill · its own key", kit.path_name(kit.spells[s].path.as_deref().unwrap_or("")))
            }
            Pick::Spell(_) => format!("Takes a spell slot · {}/{}", self.card_spells(id) + 1, self.content.game.spell_slots),
            Pick::Perk(s, _) => {
                let n = p.spells.iter().find(|o| o.spell == s).map_or(0, |o| o.perks.len()) + 1;
                format!("Upgrade {n} of {} for {}", kit.upgrades_per_spell, kit.spells[s].name)
            }
            Pick::Card(_) => "Legendary".into(),
            Pick::Power(k) => match self.content.powers[k].source {
                crate::powers::PowerSource::Boss => "Boss power".into(),
                crate::powers::PowerSource::Escalation => "Legendary".into(),
            },
            Pick::Talent(t) if kit.talents[t].grants.is_some() => format!(
                "Talent · {} tree · takes a spell slot {}/{}",
                kit.trees[kit.talents[t].tree.min(2)].name,
                self.card_spells(id) + 1,
                self.content.game.spell_slots
            ),
            Pick::Talent(t) if kit.class_talents => {
                let def = &kit.talents[t];
                if def.requires.len() >= 2 {
                    let names: Vec<&str> = def.requires.iter().map(|&s| kit.spells[s].name.as_str()).collect();
                    format!("Combination · {}", names.join(" + "))
                } else if def.path.is_some() {
                    format!("{} · one a run", kit.path_label)
                } else if let Some(path) = &def.path_only {
                    format!("{} · path card", kit.path_name(path))
                } else if def.ranks > 1 {
                    let have = self.entities[&id].player().and_then(|p| p.talents.get(t).copied()).unwrap_or(0);
                    format!("{} Talent · rank {} of {}", super::shop::capitalize(&kit.class), have + 1, def.ranks)
                } else {
                    format!("{} Talent · lasts the run", super::shop::capitalize(&kit.class))
                }
            }
            Pick::Talent(t) => format!("Talent · {} tree", kit.trees[kit.talents[t].tree.min(2)].name),
        }
    }

    /// Banish card `slot` (0-based) of the open offer and roll a new one; false if there's no
    /// banish left or no such card.
    pub fn banish_offered(&mut self, id: EntityId, slot: usize) -> bool {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return false };
        let Some(&card) = p.offers.front().and_then(|o| o.cards.get(slot)) else { return false };
        if p.banishes == 0 || p.menu.as_ref().and_then(|m| m.offer.as_ref()).is_none() {
            return false;
        }
        self.do_banish(id, card);
        true
    }

    /// Lock card `slot` of the open offer into the next one.
    pub fn lock_offered(&mut self, id: EntityId, slot: usize) -> bool {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return false };
        let Some(&card) = p.offers.front().and_then(|o| o.cards.get(slot)) else { return false };
        if !(p.luck.lock && p.locked.is_none()) || p.menu.as_ref().and_then(|m| m.offer.as_ref()).is_none() {
            return false;
        }
        self.do_lock(id, card);
        true
    }

    fn do_banish(&mut self, id: EntityId, card: Pick) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.banishes = p.banishes.saturating_sub(1);
            p.banished.push(card);
        }
        let name = self.pick_name(id, card);
        self.tell(id, format!("{name} won't be offered again this run."));
        self.reroll_front(id);
    }

    fn do_lock(&mut self, id: EntityId, card: Pick) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.locked = Some(card);
        }
        let name = self.pick_name(id, card);
        self.tell(id, format!("{name} is locked into your next offer."));
        self.show_next_offer(id);
    }

    /// The set piece of `tier` that most improves what the player wears.
    fn auto_set_piece(&mut self, id: EntityId, tier: u8) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let class = p.class;
        let quality = crate::items::tier_quality(tier);
        let mut best: Option<(f32, u8)> = None;
        for piece in 0..8u8 {
            let key = crate::items::GearKey { class, tier, quality, piece, archetype: 0 };
            let Some(t) = crate::items::template(&self.content, key.entry()) else { continue };
            let worn = p.equip[t.slot as usize].and_then(|w| crate::items::template(&self.content, w.entry)).map_or(0.0, |w| w.score() as f32);
            let gain = t.score() as f32 - worn;
            if best.map_or(true, |(g, _)| gain > g) {
                best = Some((gain, piece));
            }
        }
        if let Some((_, piece)) = best {
            let archetype = self.rng.gen_range(0..crate::items::ARCHETYPES);
            let entry = crate::items::GearKey { class, tier, quality, piece, archetype }.entry();
            self.give_item(id, entry, 0);
        }
    }

    #[allow(dead_code)]
    fn show_piece_menu(&mut self, id: EntityId, tier: u8) {
        let p = self.entities[&id].player().unwrap();
        let class = p.class;
        let mut options = Vec::new();
        let mut actions = Vec::new();
        for piece in 0..8u8 {
            let key = crate::items::GearKey { class, tier, quality: crate::items::tier_quality(tier), piece, archetype: 0 };
            if let Some(t) = crate::items::template(&self.content, key.entry()) {
                options.push(MenuOption { icon: 1, text: t.name.clone() });
                actions.push(MenuAction::Piece { tier, piece });
            }
        }
        self.open_menu(
            id,
            Menu { kind: MenuKind::SetPiece, source: id, title: "Relic: choose a set piece".into(), options, offer: None },
            actions,
        );
    }

    pub(super) fn open_menu(&mut self, id: EntityId, menu: Menu, actions: Vec<MenuAction>) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        p.menu = Some(menu.clone());
        p.menu_actions = actions;
        self.events.push(Event::Menu { player: id, menu: Some(menu) });
    }

    pub(super) fn close_menu(&mut self, id: EntityId) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if p.menu.take().is_some() {
            p.menu_actions.clear();
            self.events.push(Event::Menu { player: id, menu: None });
        }
    }

    /// The menu a player has open, if any.
    pub fn menu(&self, id: EntityId) -> Option<&Menu> {
        self.entities.get(&id)?.player()?.menu.as_ref()
    }

    /// The player closed the menu window without choosing: offers wait for `.pick` or reopen
    /// with `.cards`.
    pub fn menu_closed(&mut self, id: EntityId) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.menu = None;
            p.menu_actions.clear();
        }
    }

    /// Reopen the next waiting offer (`.cards`).
    pub fn reopen_offers(&mut self, id: EntityId) -> bool {
        let waiting = self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| !p.offers.is_empty());
        if waiting {
            self.show_next_offer(id);
        }
        waiting
    }

    /// Picks waiting for the player.
    pub fn offers_waiting(&self, id: EntityId) -> usize {
        self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.offers.len())
    }

    /// The player chose option `index` (0-based) of their open menu.
    pub fn menu_select(&mut self, id: EntityId, index: usize) {
        let Some(action) = self
            .entities
            .get(&id)
            .and_then(|e| e.player())
            .and_then(|p| p.menu_actions.get(index).cloned())
        else {
            return;
        };
        match action {
            MenuAction::Take(pick) => {
                self.take_pick(id, pick);
                self.finish_offer(id);
            }
            MenuAction::Reroll => {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.rerolls = p.rerolls.saturating_sub(1);
                }
                self.reroll_front(id);
            }
            MenuAction::GoldReroll(cost) => {
                if self.take_gold(id, cost) {
                    if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                        p.gold_rerolls += 1;
                    }
                    self.reroll_front(id);
                } else {
                    self.tell(id, format!("You need {cost} gold to reroll."));
                    self.show_next_offer(id);
                }
            }
            MenuAction::OpenBanish | MenuAction::OpenLock => {
                let banish = action == MenuAction::OpenBanish;
                let p = self.entities[&id].player().unwrap();
                let Some(offer) = p.offers.front().cloned() else { return };
                let mut options = Vec::new();
                let mut actions = Vec::new();
                for &c in &offer.cards {
                    options.push(MenuOption {
                        icon: 0,
                        text: format!("{} {}", if banish { "Banish" } else { "Lock" }, self.pick_name(id, c)),
                    });
                    actions.push(if banish { MenuAction::Banish(c) } else { MenuAction::Lock(c) });
                }
                options.push(MenuOption { icon: 0, text: "Back".into() });
                actions.push(MenuAction::Back);
                let kind = if banish { MenuKind::Banish } else { MenuKind::Lock };
                let title = if banish { "Banish which card for this run?" } else { "Lock which card?" };
                self.open_menu(id, Menu { kind, source: id, title: title.into(), options, offer: None }, actions);
            }
            MenuAction::Banish(card) => self.do_banish(id, card),
            MenuAction::Lock(card) => self.do_lock(id, card),
            MenuAction::Back => self.show_next_offer(id),
            MenuAction::Skip(gold) => {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.skips = p.skips.saturating_sub(1);
                }
                self.give_gold(id, gold);
                self.finish_offer(id);
            }
            MenuAction::Piece { tier, piece } => {
                let class = self.entities[&id].player().unwrap().class;
                let quality = crate::items::tier_quality(tier);
                let archetype = self.rng.gen_range(0..crate::items::ARCHETYPES);
                let entry = crate::items::GearKey { class, tier, quality, piece, archetype }.entry();
                self.give_item(id, entry, 0);
                self.finish_offer(id);
            }
            MenuAction::Forge(f) => self.forge(id, f),
            MenuAction::Claim(base) => {
                self.close_menu(id);
                self.begin_claim(id, base);
            }
            MenuAction::OpenCache(base) => {
                self.close_menu(id);
                self.open_cache(id, base);
            }
            MenuAction::Close => self.close_menu(id),
        }
    }

    fn reroll_front(&mut self, id: EntityId) {
        let source = match self.entities.get(&id).and_then(|e| e.player()).and_then(|p| p.offers.front()) {
            Some(o) => o.source,
            None => return,
        };
        let cards = self.roll_offer(id, source);
        if let Some(o) = self.entities.get_mut(&id).and_then(|e| e.player_mut()).and_then(|p| p.offers.front_mut()) {
            o.cards = cards;
        }
        self.show_next_offer(id);
    }

    fn finish_offer(&mut self, id: EntityId) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.offers.pop_front();
            p.menu = None;
            p.menu_actions.clear();
        }
        let more = self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| !p.offers.is_empty());
        if more {
            self.show_next_offer(id);
        } else {
            self.events.push(Event::Menu { player: id, menu: None });
        }
    }

    /// Take a card: learn the spell, apply the upgrade, or add the legendary.
    pub(super) fn take_pick(&mut self, id: EntityId, pick: Pick) {
        let class = match self.entities.get(&id).and_then(|e| e.player()) {
            Some(p) => p.class,
            None => return,
        };
        let kit = self.content.kit(class).clone();
        let (key, rank, text) = match pick {
            Pick::Power(k) => {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.powers.push(k);
                }
                self.recompute(id);
                let def = &self.content.powers[k];
                let color = def.rarity.color();
                (format!("power:{}", def.key), 1, format!("You gain |c{color}{}|r: {}", def.name, def.text))
            }
            Pick::Spell(s) => {
                self.learn_spell(id, s, false);
                let def = &kit.spells[s];
                let nums = self.entities[&id].player().unwrap().spells.iter().find(|o| o.spell == s).map(|o| o.nums.clone()).unwrap_or_default();
                let t = crate::describe::spell_text(def, &nums, self.player_hit(id), false);
                (def.key.clone(), 1, format!("You learn |cff71d5ff{}|r: {}", def.name, t.text))
            }
            Pick::Perk(s, k) => {
                self.take_perk(id, s, k);
                let def = &kit.spells[s];
                let rank = self.entities[&id].player().unwrap().spells.iter().find(|o| o.spell == s).map_or(1, |o| o.perks.len() as u8 + 1);
                (
                    format!("{}:{}", def.key, def.perks[k].key),
                    rank,
                    format!("|cff71d5ff{}|r gains |c{}{}|r: {}", def.name, def.perks[k].rarity.color(), def.perks[k].name, def.perks[k].text),
                )
            }
            Pick::Talent(t) => {
                self.grant_talent(id, t);
                let def = &kit.talents[t];
                let rank = self.entities[&id].player().and_then(|p| p.talents.get(t).copied()).unwrap_or(def.ranks).max(1);
                let label = if def.ranks > 1 { format!(" (rank {rank} of {})", def.ranks) } else { String::new() };
                (format!("talent:{}", def.key), rank, format!("You learn the talent |cffffd060{}|r{label}: {}", def.name, def.text_at(rank)))
            }
            Pick::Card(c) => {
                let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
                let rank = match p.cards.iter_mut().find(|o| o.card == c) {
                    Some(o) => {
                        o.rank = (o.rank + 1).min(5);
                        o.rank
                    }
                    None => {
                        p.cards.push(Owned { card: c, rank: 1 });
                        1
                    }
                };
                self.recompute(id);
                let def = &self.content.cards[c];
                let t = crate::describe::card_text(def, rank, self.player_hit(id), 1.0, false);
                (def.key.clone(), rank, format!("You gain |c{}{}|r: {}", def.rarity.color(), def.name, t.text))
            }
        };
        self.tell(id, text);
        self.events.push(Event::CardTaken { player: id, card: key, rank });
    }

    /// `.pick n` from chat: the same as clicking option n.
    pub fn pick(&mut self, id: EntityId, n: usize) -> bool {
        let has_menu = self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.menu.is_some());
        if !has_menu {
            if !self.reopen_offers(id) {
                return false;
            }
        }
        if n == 0 {
            return false;
        }
        self.menu_select(id, n - 1);
        true
    }

    // --- the buff bar -------------------------------------------------------------------------------

    /// Show the player's buffs: timed ones first (Clearcasting, Enrage...), then toggles and
    /// talent auras; for players without the add-on, their spells too (rank as stacks).
    pub(super) fn refresh_auras(&mut self, id: EntityId) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let now = self.clock;
        let kit = self.content.kit(p.class);
        let mut want: Vec<(u32, u8)> = Vec::new();
        if p.together {
            want.push((super::players::TOGETHER_SPELL, 1));
        }
        for b in p.buffs.iter().filter(|b| b.until > now) {
            let stacks = match b.kind {
                BuffKind::Combustion => b.stacks.max(1),
                BuffKind::Flurry | BuffKind::SweepingStrikes => b.stacks.max(1),
                _ => 1,
            };
            let spell = b.kind.aura();
            if !want.iter().any(|(s, _)| *s == spell) {
                want.push((spell, stacks.min(255) as u8));
            }
        }
        for o in &p.spells {
            if o.on {
                want.push((kit.spells[o.spell].icon, 1));
            }
        }
        // Buffs from upgrades' and talents' triggers.
        for (icon, stacks) in self.tbuff_auras(id) {
            if !want.iter().any(|(s, _)| *s == icon) {
                want.push((icon, stacks));
            }
        }
        // Passive talents that are auras.
        for (ti, t) in kit.talents.iter().enumerate() {
            if p.talents.get(ti).copied().unwrap_or(0) > 0 && matches!(t.key.as_str(), "trueshot_aura" | "improved_inner_fire" | "improved_battle_shout" | "divine_spirit") {
                let spell = match t.key.as_str() {
                    "improved_battle_shout" => 6673,
                    "improved_inner_fire" => 588,
                    other => {
                        let _ = other;
                        t.icon
                    }
                };
                want.push((spell, 1));
            }
        }
        if !p.manual {
            for i in self.bar_order(id) {
                let o = &p.spells[i];
                want.push((kit.spells[o.spell].icon, o.perks.len() as u8 + 1));
            }
        }
        // Powers: their look (a lasting aura) or their icon.
        for &k in &p.powers {
            if let Some(pw) = self.content.powers.get(k) {
                if !want.iter().any(|(s, _)| *s == pw.buff_spell()) {
                    want.push((pw.buff_spell(), 1));
                }
            }
        }
        for o in &p.cards {
            want.push((self.content.cards[o.card].aura(), o.rank));
        }
        want.truncate(28);
        let sent = p.sent_auras.clone();
        let mut events = Vec::new();
        for slot in 0..want.len().max(sent.len()) {
            let w = want.get(slot).copied().unwrap_or((0, 0));
            if sent.get(slot).copied() != Some(w) {
                events.push(Event::Aura { unit: id, slot: slot as u8, spell: w.0, stacks: w.1, secs: 0.0 });
            }
        }
        self.events.extend(events);
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.sent_auras = want;
        }
    }

    /// Re-send everything on a player's buff bar (a fresh login).
    pub fn resend_auras(&mut self, id: EntityId) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.sent_auras.clear();
        }
        self.refresh_auras(id);
    }

    // --- zones and missiles --------------------------------------------------------------------------

    pub(super) fn step_zones(&mut self, _dt: f32) {
        // Players already hit this step by a mark of the same cast.
        let mut struck: std::collections::HashSet<(u32, EntityId)> = std::collections::HashSet::new();
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.body, Body::Zone(_)))
            .map(|(&id, _)| id)
            .collect();
        for zid in ids {
            let cast = match &self.entities[&zid].body {
                Body::Zone(z) => z.cast,
                _ => 0,
            };
            let (pos, caster, spell, radius, until, tick, per_tick, school, slow, hostile, heal, share, info) = match &self.entities[&zid].body {
                Body::Zone(z) => (
                    self.entities[&zid].pos,
                    z.caster,
                    z.spell,
                    z.radius,
                    z.until,
                    z.next_tick,
                    z.per_tick,
                    z.school,
                    z.slow,
                    z.hostile,
                    z.heal,
                    z.share,
                    z.info.clone(),
                ),
                _ => continue,
            };
            // An enemy killed before its strike lands takes the strike with it.
            let cut_short = hostile && tick > self.clock && !self.entities.get(&caster).is_some_and(|e| e.alive());
            if until <= self.clock || !self.entities.contains_key(&caster) || !self.combat_on() || cut_short {
                self.entities.remove(&zid);
                self.events.push(Event::ZoneEnd { id: zid });
                continue;
            }
            if tick > self.clock {
                continue;
            }
            if let Body::Zone(z) = &mut self.entities.get_mut(&zid).unwrap().body {
                z.next_tick += 1.0;
            }
            if hostile {
                let skip: Vec<EntityId> = if cast == 0 {
                    Vec::new()
                } else {
                    struck.iter().filter(|(c, _)| *c == cast).map(|(_, p)| *p).collect()
                };
                let hit = self.hazard_tick(caster, pos, radius, spell, per_tick, school, slow, &skip, share);
                if cast != 0 {
                    struck.extend(hit.into_iter().map(|p| (cast, p)));
                }
                continue;
            }
            if heal > 0.0 {
                for a in self.allies_near(pos, radius) {
                    let amount = self.entities[&a].max_health as f32 * heal;
                    self.heal(caster, a, spell, amount.round().max(1.0) as u32, false);
                }
                continue;
            }
            match info {
                Some(info) => {
                    // A tick carries the spell's chills, freezes and vulnerabilities, but not its
                    // first blast's stun.
                    let mut tick_info = (*info).clone();
                    tick_info.nums.stun_chance = 0.0;
                    tick_info.nums.splash = 0.0;
                    for (v, _) in self.enemies_near(pos, radius) {
                        self.land_hit(caster, v, per_tick, &tick_info);
                    }
                }
                None => {
                    for (v, _) in self.enemies_near(pos, radius) {
                        self.deal(caster, v, per_tick.round().max(1.0) as u32, false, Via::Spell { spell, school, periodic: true });
                        if slow > 0.0 {
                            self.slow_enemy(v, slow, 1.5);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn queue_hit(
        &mut self,
        caster: EntityId,
        victim: EntityId,
        spell: u32,
        damage: f32,
        crit: bool,
        school: u8,
        delay: f32,
        fx: HitFx,
    ) {
        let damage = if crit && !fx.auto { damage * 1.5 } else { damage };
        self.hits.push(PendingHit { at: self.clock + delay, caster, victim, spell, damage, crit, school, fx, info: None });
    }

    pub(super) fn step_hits(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<PendingHit>, Vec<PendingHit>) = std::mem::take(&mut self.hits).into_iter().partition(|h| h.at <= now);
        self.hits = keep;
        for h in due {
            let alive = self.entities.get(&h.victim).is_some_and(|e| e.alive()) && self.entities.contains_key(&h.caster);
            if !alive {
                continue;
            }
            if let Some(info) = h.info {
                self.land_hit(h.caster, h.victim, h.damage, &info);
                continue;
            }
            let mut damage = h.damage;
            if h.fx.auto {
                let owner = self.owner_of(h.caster);
                let (m, _) = self.against(owner, h.victim, h.school, &Nums::default(), false);
                damage *= m;
            }
            if damage >= 0.5 {
                let dmg = damage.round().max(1.0) as u32;
                self.deal(h.caster, h.victim, dmg, h.crit, Via::Spell { spell: h.spell, school: h.school, periodic: false });
            }
            if h.fx.auto && self.entities.get(&h.caster).is_some_and(|e| e.is_player()) {
                self.after_auto_hit(h.caster, h.victim, damage, h.crit);
            }
        }
    }
}

#[cfg(test)]
impl Sim {
    /// The cards a wave's offer would show this player now, by key.
    /// Take talent `key` at full rank, as a level-up card would give it.
    pub fn give_test_talent(&mut self, id: EntityId, key: &str) {
        let Some(class) = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.class) else { return };
        if let Some(t) = self.content.kit(class).talents.iter().position(|t| t.key == key) {
            self.grant_talent(id, t);
        }
    }

    pub fn roll_for_test(&mut self, id: EntityId) -> Vec<String> {
        let picks = self.roll_offer(id, OfferSource::Wave);
        picks.into_iter().map(|c| self.pick_view(id, c, false).key).collect()
    }

    /// A boss standing at `pos`.
    pub fn spawn_boss_for_test(&mut self, index: usize, pos: Vec3) -> EntityId {
        let id = self.alloc();
        self.spawn_enemy(id, EnemyKind { boss: true, index }, false, None, pos, 0.0, 30.0, 1.0);
        id
    }

    /// Hand a player a spell by key, with its first `perks` upgrades.
    pub fn give_test_card(&mut self, id: EntityId, key: &str, perks: u8) {
        let class = self.entities[&id].player().unwrap().class;
        let kit = self.content.kit(class);
        let s = kit.spell_index(key).unwrap_or_else(|| panic!("no spell {key}"));
        let talent = kit.spells[s].talent;
        let count = kit.spells[s].perks.len();
        self.learn_spell(id, s, talent);
        for k in 0..(perks as usize).min(count) {
            self.take_perk(id, s, k);
        }
    }

    /// One level-up: roll a level offer (a start offer while the bar is empty), show it as
    /// cards, take the one `choose` picks. Empty when the build is complete.
    pub fn level_up_for_test(&mut self, id: EntityId, choose: &mut dyn FnMut(&[CardView]) -> usize) -> (Vec<CardView>, usize) {
        let empty = self.card_spells(id) == 0;
        let picks = self.roll_offer(id, if empty { OfferSource::Start } else { OfferSource::Level });
        let views: Vec<CardView> = picks.iter().map(|&c| self.pick_view(id, c, false)).collect();
        if picks.is_empty() {
            return (views, 0);
        }
        let k = choose(&views).min(picks.len() - 1);
        self.take_pick(id, picks[k]);
        (views, k)
    }

    /// Whether a spell's card could be offered now (its path taken, a slot free, not held).
    pub fn spell_open_for_test(&self, id: EntityId, key: &str) -> bool {
        let class = self.entities[&id].player().unwrap().class;
        let s = self.content.kit(class).spell_index(key).unwrap_or_else(|| panic!("no spell {key}"));
        self.pick_available(id, Pick::Spell(s))
    }

    /// Take one upgrade of a held spell by key, as a card would (if it's open).
    pub fn perk_for_test(&mut self, id: EntityId, spell: &str, perk: &str) -> Result<(), &'static str> {
        let class = self.entities[&id].player().unwrap().class;
        let kit = self.content.kit(class);
        let s = kit.spell_index(spell).ok_or("no such spell")?;
        let k = kit.spells[s].perks.iter().position(|p| p.key == perk).ok_or("no such upgrade")?;
        if !self.pick_available(id, Pick::Perk(s, k)) {
            return Err("not open");
        }
        self.take_perk(id, s, k);
        Ok(())
    }

    /// Offer a hero what boss `boss` (an index into the bosses) leaves.
    pub fn boss_offer_for_test(&mut self, id: EntityId, boss: usize) {
        self.offer(id, OfferSource::Boss(boss));
    }

    /// Offer a hero an Escalation's legendary pick.
    pub fn legendary_offer_for_test(&mut self, id: EntityId) {
        self.offer(id, OfferSource::Legendary);
    }

    /// Hand a player a power from powers.toml.
    pub fn give_test_power(&mut self, id: EntityId, key: &str) {
        let k = self.content.powers.iter().position(|p| p.key == key).unwrap_or_else(|| panic!("no power {key}"));
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.powers.push(k);
        }
        self.recompute(id);
    }

    /// Hand a player a card from cards.toml (a legendary, say).
    pub fn give_test_legendary(&mut self, id: EntityId, key: &str) {
        let c = self.content.cards.iter().position(|c| c.key == key).unwrap_or_else(|| panic!("no card {key}"));
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.cards.push(Owned { card: c, rank: 1 });
        }
        self.recompute(id);
    }

    /// Hand a player one named upgrade.
    pub fn give_test_perk(&mut self, id: EntityId, spell: &str, perk: &str) {
        let class = self.entities[&id].player().unwrap().class;
        let kit = self.content.kit(class);
        let s = kit.spell_index(spell).expect("a spell");
        let k = kit.spells[s].perks.iter().position(|p| p.key == perk).expect("a perk");
        self.take_perk(id, s, k);
    }
}
