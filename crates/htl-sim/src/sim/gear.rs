//! Gear in play: the starting kit, drops, corpse loot, bags and equipping.

use rand::Rng;

use super::*;
use crate::items::{self, slot, GearKey, ItemTemplate, OFF_HAND_PIECE, ONE_HAND_PIECE, QUALITY_COMMON, QUALITY_EPIC, QUALITY_LEGENDARY, SHIELD_PIECE, WEAPON_PIECE};

/// One item a player owns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ItemInst {
    /// Unique for the server's lifetime: the item's object id.
    pub id: u32,
    pub entry: u32,
    pub enchant: u32,
}

/// What a corpse holds for one player.
#[derive(Debug, Clone)]
pub(super) struct CorpseLoot {
    pub player: EntityId,
    pub gold: u32,
    pub items: Vec<u32>,
    /// Taking one item takes them all away (a boss's choice).
    pub choose_one: bool,
}

/// A loot window's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LootView {
    pub gold: u32,
    /// Loot slot and item entry.
    pub items: Vec<(u8, u32)>,
}

/// Why an inventory change was refused, as the client's `EQUIP_ERR` codes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EquipError {
    /// "You must reach level N to use that item."
    Level(u32),
    /// "You can't equip that."
    CantEquip,
    /// "That item doesn't go in that slot."
    WrongSlot,
    /// "Your inventory is full."
    BagFull,
    /// "Item not found."
    NotFound,
}

impl Sim {
    /// The item template behind an entry.
    pub fn item_template(&self, entry: u32) -> Option<ItemTemplate> {
        items::template(&self.content, entry)
    }

    pub(super) fn starting_kit(&mut self, id: EntityId) {
        let Some(class) = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.class) else { return };
        let weapon_slot = self.content.classes[class].weapon_slot as usize;
        let mut kit = vec![
            (slot::SHIRT as usize, items::STARTER_SHIRT),
            (slot::LEGS as usize, items::STARTER_PANTS),
            (weapon_slot, items::starter_weapon(class)),
        ];
        if let Some(wand) = items::starter_wand(&self.content, class) {
            kit.push((slot::RANGED as usize, wand));
        }
        for (s, entry) in kit {
            let inst = ItemInst { id: self.alloc_item(), entry, enchant: 0 };
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.equip[s] = Some(inst);
            }
        }
        let stone = ItemInst { id: self.alloc_item(), entry: items::HEARTHSTONE, enchant: 0 };
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.pack[0] = Some(stone);
        }
        self.events.push(Event::Inventory { player: id });
        self.events.push(Event::Visible { player: id });
    }

    /// A new item: into the bags, for the hero to choose whether to wear it (right-click it).
    /// The message says how it compares with what's worn there.
    pub(super) fn give_item(&mut self, id: EntityId, entry: u32, enchant: u32) {
        let Some(t) = items::template(&self.content, entry) else { return };
        let enchant = if enchant != 0 { enchant } else { t.enchant };
        let inst = ItemInst { id: self.alloc_item(), entry, enchant };
        let level = self.entities.get(&id).map_or(1, |e| e.level);
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let name = t.name.clone();
        let color = quality_color(t.quality);
        if t.slot == slot::NONE || t.class.is_some_and(|c| c != p.class) {
            return;
        }
        let free = p.pack.iter().position(|s| s.is_none());
        let Some(slot_index) = free else {
            self.tell(id, format!("Your bags are full: |c{color}[{name}]|r was left behind. Drop something you don't need (drag it out of your bags)."));
            return;
        };
        let compare = self.compared_with_worn(id, &t);
        let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
        p.pack[slot_index] = Some(inst);
        let s = slot::PACK_FIRST + slot_index as u8;
        self.events.push(Event::ItemReceived { player: id, entry, slot: s });
        self.events.push(Event::Inventory { player: id });
        if level < t.required_level {
            self.tell(id, format!("You loot |c{color}[{name}]|r: you can wear it from level {}. {compare}", t.required_level));
        } else {
            self.tell(id, format!("You loot |c{color}[{name}]|r. {compare} Right-click it in your bags to wear it."));
        }
    }

    /// "Better than your [Worn Helm]." and the like, for a loot message.
    fn compared_with_worn(&self, id: EntityId, t: &items::ItemTemplate) -> String {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return String::new() };
        let worn = (0..slot::EQUIPMENT)
            .filter(|&s| items::fits_slot(t.slot, s))
            .filter_map(|s| p.equip[s as usize].and_then(|w| items::template(&self.content, w.entry)))
            .min_by(|a, b| a.score().total_cmp(&b.score()));
        match worn {
            None => "Nothing is worn there yet.".to_string(),
            Some(w) if w.quality <= QUALITY_COMMON || t.score() > w.score() => {
                format!("An upgrade over your |c{}[{}]|r.", quality_color(w.quality), w.name)
            }
            Some(w) => format!("Not as good as your |c{}[{}]|r.", quality_color(w.quality), w.name),
        }
    }

    /// Whether `t` beats what the player wears in its slot (starter pieces always lose).
    fn better_than_worn(&self, id: EntityId, t: &items::ItemTemplate) -> bool {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return false };
        match p.equip[t.slot as usize].and_then(|w| items::template(&self.content, w.entry)) {
            None => true,
            Some(w) => w.quality <= QUALITY_COMMON || t.score() > w.score(),
        }
    }

    /// How much wearing `entry` would improve the player: its score over what's worn there.
    fn upgrade_gain(&self, id: EntityId, entry: u32) -> f32 {
        let Some(t) = items::template(&self.content, entry) else { return f32::MIN };
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return f32::MIN };
        let worn = p.equip[t.slot as usize]
            .and_then(|w| items::template(&self.content, w.entry))
            .filter(|w| w.quality > QUALITY_COMMON)
            .map_or(0.0, |w| w.score() as f32);
        t.score() as f32 - worn
    }

    /// Put `inst` on; whatever it replaces is gone.
    fn wear(&mut self, id: EntityId, inst: ItemInst, t: &items::ItemTemplate) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        p.equip[t.slot as usize] = Some(inst);
        self.events.push(Event::ItemReceived { player: id, entry: inst.entry, slot: t.slot });
        self.events.push(Event::Inventory { player: id });
        self.events.push(Event::Visible { player: id });
        let color = quality_color(t.quality);
        self.tell(id, format!("You put on |c{color}[{}]|r.", t.name));
        self.recompute(id);
    }

    /// After a level-up: say which pieces in the bags can be worn now (the hero puts them on).
    pub(super) fn wear_from_bags(&mut self, id: EntityId) {
        let level = self.entities.get(&id).map_or(1, |e| e.level);
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let ready: Vec<items::ItemTemplate> = p
            .pack
            .iter()
            .flatten()
            .filter_map(|s| items::template(&self.content, s.entry))
            .filter(|t| t.slot != slot::NONE && t.required_level == level)
            .collect();
        for t in ready {
            if self.better_than_worn(id, &t) {
                let color = quality_color(t.quality);
                self.tell(id, format!("You can wear |c{color}[{}]|r now: it's in your bags (right-click it).", t.name));
            }
        }
    }

    fn item_at(&self, id: EntityId, s: u8) -> Option<ItemInst> {
        let p = self.entities.get(&id)?.player()?;
        if s < slot::EQUIPMENT {
            p.equip[s as usize]
        } else if (slot::PACK_FIRST..slot::PACK_FIRST + slot::PACK_SLOTS).contains(&s) {
            p.pack[(s - slot::PACK_FIRST) as usize]
        } else {
            None
        }
    }

    fn set_item_at(&mut self, id: EntityId, s: u8, item: Option<ItemInst>) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if s < slot::EQUIPMENT {
            p.equip[s as usize] = item;
        } else if (slot::PACK_FIRST..slot::PACK_FIRST + slot::PACK_SLOTS).contains(&s) {
            p.pack[(s - slot::PACK_FIRST) as usize] = item;
        }
    }

    /// Whether `item` may sit in slot `s` for this player.
    fn fits(&self, id: EntityId, item: &ItemInst, s: u8) -> Result<(), EquipError> {
        let in_pack = (slot::PACK_FIRST..slot::PACK_FIRST + slot::PACK_SLOTS).contains(&s);
        if s >= slot::EQUIPMENT && !in_pack {
            return Err(EquipError::WrongSlot);
        }
        if s >= slot::EQUIPMENT {
            return Ok(());
        }
        let t = items::template(&self.content, item.entry).ok_or(EquipError::NotFound)?;
        let e = &self.entities[&id];
        let p = e.player().ok_or(EquipError::NotFound)?;
        if !items::fits_slot(t.slot, s) {
            return Err(EquipError::WrongSlot);
        }
        if t.class.is_some_and(|c| c != p.class) {
            return Err(EquipError::CantEquip);
        }
        if e.level < t.required_level {
            return Err(EquipError::Level(t.required_level));
        }
        // A weapon path's hands: no greatsword with a shield or a second weapon, and the other
        // hand holds only what the path does.
        if let Some(key) = GearKey::decode(item.entry) {
            if key.piece >= WEAPON_PIECE && p.path.is_some() && !self.path_weapon_pieces(id).contains(&key.piece) {
                return Err(EquipError::CantEquip);
            }
        }
        Ok(())
    }

    /// Commit hero `id` to a weapon path: the greatsword stays, or it becomes a sword and a
    /// shield, or a sword and an axe, of the same tier and quality.
    pub(super) fn take_path(&mut self, id: EntityId, path: &str) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if p.path.is_some() {
            return;
        }
        p.path = Some(path.to_string());
        let class = p.class;
        let main = p.equip[slot::MAIN_HAND as usize];
        let (tier, quality, archetype) = main
            .and_then(|m| GearKey::decode(m.entry))
            .map(|k| (k.tier, k.quality, k.archetype))
            .unwrap_or((0, QUALITY_COMMON, 0));
        let pieces: &[u8] = match path {
            "shield" => &[ONE_HAND_PIECE, SHIELD_PIECE],
            "dual" => &[ONE_HAND_PIECE, OFF_HAND_PIECE],
            _ => return,
        };
        let enchant = main.map_or(0, |m| m.enchant);
        for &piece in pieces {
            let key = GearKey { class, tier, quality, piece, archetype };
            let Some(t) = items::template(&self.content, key.entry()) else { continue };
            // The weapon's enchantment carries over to the sword.
            let ench = if piece == ONE_HAND_PIECE && enchant != 0 { enchant } else { t.enchant };
            let inst = ItemInst { id: self.alloc_item(), entry: key.entry(), enchant: ench };
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.equip[t.slot as usize] = Some(inst);
            }
        }
        // Greatswords in the bags are no use now.
        let content = &self.content;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            for s in p.pack.iter_mut() {
                if s.and_then(|i| GearKey::decode(i.entry)).is_some_and(|k| k.piece == WEAPON_PIECE) {
                    *s = None;
                }
            }
        }
        let _ = content;
        self.events.push(Event::Inventory { player: id });
        self.events.push(Event::Visible { player: id });
        self.recompute(id);
    }

    /// Swap whatever is in slots `a` and `b` (equipment 0-18, backpack 23-38).
    pub fn swap_items(&mut self, id: EntityId, a: u8, b: u8) -> Result<(), EquipError> {
        let ia = self.item_at(id, a);
        let ib = self.item_at(id, b);
        if ia.is_none() && ib.is_none() {
            return Err(EquipError::NotFound);
        }
        if let Some(i) = &ia {
            self.fits(id, i, b)?;
        }
        if let Some(i) = &ib {
            self.fits(id, i, a)?;
        }
        // Leaving a weapon or other slot empty is fine.
        self.set_item_at(id, a, ib);
        self.set_item_at(id, b, ia);
        self.after_inventory_change(id, a < slot::EQUIPMENT || b < slot::EQUIPMENT);
        Ok(())
    }

    /// Right-click in the bags: wear it in its own slot.
    pub fn auto_equip(&mut self, id: EntityId, from: u8) -> Result<(), EquipError> {
        let item = self.item_at(id, from).ok_or(EquipError::NotFound)?;
        let t = items::template(&self.content, item.entry).ok_or(EquipError::NotFound)?;
        if t.slot == slot::NONE {
            return Err(EquipError::WrongSlot);
        }
        if from < slot::EQUIPMENT {
            // Right-click on a worn piece: take it off into the bags.
            let free = self.entities[&id].player().unwrap().pack.iter().position(|s| s.is_none());
            let Some(free) = free else { return Err(EquipError::BagFull) };
            return self.swap_items(id, from, slot::PACK_FIRST + free as u8);
        }
        // A ring or trinket goes in whichever of its pair is free.
        let target = self.free_pair_slot(id, t.slot).unwrap_or(t.slot);
        self.swap_items(id, from, target)
    }

    /// The first empty slot an item made for `made` fits (rings and trinkets have two).
    pub(super) fn free_pair_slot(&self, id: EntityId, made: u8) -> Option<u8> {
        let p = self.entities.get(&id)?.player()?;
        (0..slot::EQUIPMENT).find(|&s| items::fits_slot(made, s) && p.equip[s as usize].is_none())
    }

    /// A ware just bought: worn at once in a free slot it fits, else into the bags. Err when
    /// there's no room for it anywhere.
    pub(super) fn take_ware(&mut self, id: EntityId, entry: u32) -> Result<(), String> {
        let t = items::template(&self.content, entry).ok_or("Nothing like that for sale.")?;
        let inst = ItemInst { id: self.alloc_item(), entry, enchant: 0 };
        if let Some(s) = self.free_pair_slot(id, t.slot) {
            let t2 = items::ItemTemplate { slot: s, ..t };
            self.wear(id, inst, &t2);
            return Ok(());
        }
        let p = self.entities.get(&id).and_then(|e| e.player()).ok_or("No such hero.")?;
        let free = p.pack.iter().position(|s| s.is_none()).ok_or("Your bags are full.")?;
        let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
        p.pack[free] = Some(inst);
        let s = slot::PACK_FIRST + free as u8;
        self.events.push(Event::ItemReceived { player: id, entry, slot: s });
        self.events.push(Event::Inventory { player: id });
        let color = quality_color(t.quality);
        self.tell(id, format!("|c{color}[{}]|r is in your bags: your {} slots are full. Drag it onto your character to swap.", t.name, t2_noun(&self.content, entry)));
        Ok(())
    }

    /// Drop an item on the ground: it's gone.
    pub fn destroy_item(&mut self, id: EntityId, s: u8) -> Result<(), EquipError> {
        if self.item_at(id, s).is_none() {
            return Err(EquipError::NotFound);
        }
        self.set_item_at(id, s, None);
        self.after_inventory_change(id, s < slot::EQUIPMENT);
        Ok(())
    }

    fn after_inventory_change(&mut self, id: EntityId, worn: bool) {
        self.events.push(Event::Inventory { player: id });
        if worn {
            self.events.push(Event::Visible { player: id });
            self.recompute(id);
        }
    }

    /// The tier gear drops at now: one per Escalation.
    pub(super) fn drop_tier(&self) -> u8 {
        (self.run.cycle as u8).min(items::MAX_TIER)
    }

    /// The weapon pieces a hero's path wears: the greatsword, or a sword and a shield or an axe.
    pub(super) fn path_weapon_pieces(&self, id: EntityId) -> &'static [u8] {
        match self.entities.get(&id).and_then(|e| e.player()).and_then(|p| p.path.as_deref()) {
            Some("shield") => &[ONE_HAND_PIECE, SHIELD_PIECE],
            Some("dual") => &[ONE_HAND_PIECE, OFF_HAND_PIECE],
            _ => &[WEAPON_PIECE],
        }
    }

    /// A random piece of the player's class at `tier`, as an entry.
    pub(super) fn random_piece(&mut self, id: EntityId, tier: u8, quality: u8, weapon_chance: f32) -> Option<u32> {
        let class = self.entities.get(&id)?.player()?.class;
        let weapon = self.rng.gen::<f32>() < weapon_chance;
        let weapons = self.path_weapon_pieces(id);
        for _ in 0..12 {
            let piece = if weapon { weapons[self.rng.gen_range(0..weapons.len())] } else { self.rng.gen_range(0..8u8) };
            let archetype = self.rng.gen_range(0..items::ARCHETYPES);
            let key = GearKey { class, tier, quality, piece, archetype };
            if items::exists(&self.content, key) {
                return Some(key.entry());
            }
        }
        None
    }

    /// End-of-wave drops: a piece for some players, a sure thing every third wave without one.
    pub(super) fn wave_drops(&mut self, boss: bool) {
        let tier = self.drop_tier();
        let quality = items::tier_quality(tier);
        for id in self.player_ids() {
            let drought = *self.run.drought.get(&id).unwrap_or(&0);
            let lucky = self.rng.gen::<f32>() < 0.35 || drought >= 2;
            if lucky {
                if let Some(entry) = self.random_piece(id, tier, quality, 0.15) {
                    self.give_item(id, entry, 0);
                }
                self.run.drought.insert(id, 0);
            } else {
                self.run.drought.insert(id, drought + 1);
            }
        }
        let _ = boss;
    }

    /// Fill a fresh corpse with loot: elites sometimes, bosses always, guardians the next tier.
    pub(super) fn roll_corpse_loot(&mut self, corpse: EntityId, kind: EnemyKind, elite: bool, guard: Option<usize>) {
        let tier = self.drop_tier();
        let mut loots = Vec::new();
        let pos = self.entities[&corpse].pos;
        for id in self.player_ids() {
            let mut items_ = Vec::new();
            let mut choose_one = false;
            let mut gold = 0;
            if kind.boss {
                // A choice of two epic pieces.
                let q = (items::tier_quality(tier) + 1).min(QUALITY_EPIC);
                for _ in 0..2 {
                    if let Some(e) = self.random_piece(id, tier, q, 0.3) {
                        if !items_.contains(&e) {
                            items_.push(e);
                        }
                    }
                }
                choose_one = items_.len() > 1;
                gold = 30 + 6 * self.run.wave;
            } else if guard.is_some() && elite {
                // Base guardians carry the next tier: the only way to wear it early.
                let near = self.entities.get(&id).is_some_and(|e| dist2d(e.pos, pos) < 80.0);
                if near {
                    let next = (tier + 1).min(items::MAX_TIER);
                    if let Some(e) = self.random_piece(id, next, items::tier_quality(next), 0.2) {
                        items_.push(e);
                    }
                    gold = 15 + 3 * self.run.wave;
                }
            } else if elite && self.rng.gen::<f32>() < 0.10 {
                if let Some(e) = self.random_piece(id, tier, items::tier_quality(tier), 0.15) {
                    items_.push(e);
                }
                gold = 5 + self.run.wave;
            }
            if !items_.is_empty() || gold > 0 {
                loots.push(CorpseLoot { player: id, gold, items: items_, choose_one });
            }
        }
        if loots.is_empty() {
            return;
        }
        if self.content.game.auto_loot {
            // No corpse to click: gold and gear go straight to each player, a boss's choice
            // falling on the piece that improves them most.
            for l in loots {
                if l.gold > 0 {
                    self.earn_gold(l.player, l.gold);
                }
                let picks: Vec<u32> = if l.choose_one {
                    let best = l
                        .items
                        .iter()
                        .copied()
                        .max_by(|a, b| self.upgrade_gain(l.player, *a).total_cmp(&self.upgrade_gain(l.player, *b)));
                    best.into_iter().collect()
                } else {
                    l.items.clone()
                };
                for e in picks {
                    self.give_item(l.player, e, 0);
                }
            }
            return;
        }
        for l in &loots {
            self.events.push(Event::Lootable { corpse, player: l.player, lootable: true });
        }
        if let Some(en) = self.entities.get_mut(&corpse).and_then(|e| e.enemy_mut()) {
            en.corpse = Some(self.content.game.loot_corpse_secs);
        }
        self.loot.insert(corpse, loots);
    }

    /// The player right-clicked a corpse.
    pub fn loot_open(&self, id: EntityId, corpse: EntityId) -> Option<LootView> {
        let l = self.loot.get(&corpse)?.iter().find(|l| l.player == id)?;
        let (p, c) = (self.entities.get(&id)?, self.entities.get(&corpse)?);
        if dist2d(p.pos, c.pos) > 12.0 {
            return None;
        }
        Some(LootView {
            gold: l.gold,
            items: l.items.iter().enumerate().map(|(i, e)| (i as u8, *e)).collect(),
        })
    }

    /// Take one item from the corpse's loot. Returns the slots that disappeared from the window.
    pub fn loot_take(&mut self, id: EntityId, corpse: EntityId, loot_slot: u8) -> Vec<u8> {
        let Some(loots) = self.loot.get_mut(&corpse) else { return Vec::new() };
        let Some(l) = loots.iter_mut().find(|l| l.player == id) else { return Vec::new() };
        let Some(&entry) = l.items.get(loot_slot as usize) else { return Vec::new() };
        if entry == 0 {
            return Vec::new();
        }
        let gone: Vec<u8> = if l.choose_one {
            (0..l.items.len() as u8).filter(|&s| l.items[s as usize] != 0).collect()
        } else {
            vec![loot_slot]
        };
        for &s in &gone {
            l.items[s as usize] = 0;
        }
        self.give_item(id, entry, 0);
        self.tidy_loot(id, corpse);
        gone
    }

    pub fn loot_gold(&mut self, id: EntityId, corpse: EntityId) -> u32 {
        let Some(l) = self.loot.get_mut(&corpse).and_then(|ls| ls.iter_mut().find(|l| l.player == id)) else { return 0 };
        let gold = std::mem::take(&mut l.gold);
        if gold > 0 {
            self.earn_gold(id, gold);
        }
        self.tidy_loot(id, corpse);
        gold
    }

    /// Drop the player's share once it's empty.
    fn tidy_loot(&mut self, id: EntityId, corpse: EntityId) {
        let Some(loots) = self.loot.get_mut(&corpse) else { return };
        let before = loots.len();
        loots.retain(|l| l.player != id || l.gold > 0 || l.items.iter().any(|&e| e != 0));
        if loots.len() != before {
            self.events.push(Event::Lootable { corpse, player: id, lootable: false });
        }
        if loots.is_empty() {
            self.loot.remove(&corpse);
            if let Some(en) = self.entities.get_mut(&corpse).and_then(|e| e.enemy_mut()) {
                en.corpse = Some(en.corpse.unwrap_or(0.0).min(3.0));
            }
        }
    }

    /// Escalation gifts: a set piece and a weapon of the new tier.
    pub(super) fn escalation_gear(&mut self, id: EntityId) {
        let tier = self.drop_tier();
        let quality = items::tier_quality(tier);
        let class = match self.entities.get(&id).and_then(|e| e.player()) {
            Some(p) => p.class,
            None => return,
        };
        let legendary = tier >= 5;
        let wq = if legendary { QUALITY_LEGENDARY } else { quality };
        for &piece in self.path_weapon_pieces(id) {
            let weapon = GearKey { class, tier, quality: wq, piece, archetype: self.rng.gen_range(0..items::ARCHETYPES) };
            if items::exists(&self.content, weapon) {
                self.give_item(id, weapon.entry(), 0);
            }
        }
        self.offer(id, cards::OfferSource::SetPiece { tier });
    }
}

pub(super) fn quality_color(q: u8) -> &'static str {
    match q {
        0 => "ff9d9d9d",
        1 => "ffffffff",
        2 => "ff1eff00",
        3 => "ff0070dd",
        4 => "ffa335ee",
        _ => "ffff8000",
    }
}

#[cfg(test)]
impl Sim {
    /// Hand a player an item, as a drop would.
    /// Hand a hero a piece and put it on (when they can wear it).
    pub fn give_test_item(&mut self, id: EntityId, entry: u32) {
        self.give_item(id, entry, 0);
        let slot = self.player_view(id).and_then(|v| v.pack.iter().find(|(_, _, e, _)| *e == entry).map(|p| p.0));
        if let Some(s) = slot {
            let _ = self.auto_equip(id, s);
        }
    }

    /// An Escalation's gifts, as one would give them.
    pub fn escalation_gear_for_test(&mut self, id: EntityId) {
        self.escalation_gear(id);
    }

    /// Hand a hero a piece as loot does: into the bags, for them to choose.
    pub fn loot_test_item(&mut self, id: EntityId, entry: u32) {
        self.give_item(id, entry, 0);
    }
}

/// "ring", "trinket"... for a ware's entry.
fn t2_noun(content: &Content, entry: u32) -> &'static str {
    items::decode_ware(entry).and_then(|(w, _, _)| content.wares.get(w)).map_or("item", |w| match w.slot {
        crate::wares::WareSlot::Neck => "neck",
        crate::wares::WareSlot::Finger => "ring",
        crate::wares::WareSlot::Trinket => "trinket",
        crate::wares::WareSlot::Back => "cloak",
        crate::wares::WareSlot::Head => "helm",
        crate::wares::WareSlot::Tabard => "tabard",
    })
}
