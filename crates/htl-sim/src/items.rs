//! The item catalogue: every piece of gear, worked out from its entry number. The same entry
//! always means the same item (the client caches items by entry between sessions), so nothing
//! about a template is random.
//!
//! `entry = 100000 + class·20000 + tier·2000 + quality·300 + piece·25 + archetype`, where
//! `piece` is 0-7 for the eight armour slots and 8 for the class weapon, and `archetype` picks the
//! stat balance (0 balanced, 1 offence, 2 defence).

use crate::content::Content;

pub const FIRST_GEAR_ENTRY: u32 = 100_000;
/// The Hero tab's wares: 200_000 + ware x 100 + class x 10 + tier (the class picks "main" stats).
pub const FIRST_WARE_ENTRY: u32 = 200_000;
pub const STARTER_SHIRT: u32 = 99_001;
pub const STARTER_PANTS: u32 = 99_002;
/// Takes its owner back to the Ancient.
pub const HEARTHSTONE: u32 = 99_003;
/// The wand a caster's shots come from (its auto-attack is the wand's Shoot): without one in the
/// ranged slot the client has no shot to animate. Entry, then the class key and its look.
pub const STARTER_WAND: u32 = 99_004;
/// (class key, display): a frost-blue wand for the mage, a shadowed one for the priest.
pub const WANDS: [(&str, u32); 2] = [("mage", 6111), ("priest", 9063)];

pub const QUALITY_COMMON: u8 = 1;
pub const QUALITY_UNCOMMON: u8 = 2;
pub const QUALITY_RARE: u8 = 3;
pub const QUALITY_EPIC: u8 = 4;
pub const QUALITY_LEGENDARY: u8 = 5;

/// The highest tier with its own entries.
pub const MAX_TIER: u8 = 9;
/// The piece number of a class's weapon.
pub const WEAPON_PIECE: u8 = 8;
/// A warrior's weapon paths: a one-handed sword (main hand), a shield, an off-hand axe.
pub const ONE_HAND_PIECE: u8 = 9;
pub const SHIELD_PIECE: u8 = 10;
pub const OFF_HAND_PIECE: u8 = 11;
pub const ARCHETYPES: u8 = 3;

/// Equipment slots, as the client numbers them.
pub mod slot {
    pub const HEAD: u8 = 0;
    pub const SHOULDERS: u8 = 2;
    pub const NECK: u8 = 1;
    pub const SHIRT: u8 = 3;
    pub const CHEST: u8 = 4;
    pub const WAIST: u8 = 5;
    pub const LEGS: u8 = 6;
    pub const FEET: u8 = 7;
    pub const WRISTS: u8 = 8;
    pub const HANDS: u8 = 9;
    pub const FINGER1: u8 = 10;
    pub const FINGER2: u8 = 11;
    pub const TRINKET1: u8 = 12;
    pub const TRINKET2: u8 = 13;
    pub const BACK: u8 = 14;
    pub const MAIN_HAND: u8 = 15;
    pub const OFF_HAND: u8 = 16;
    pub const RANGED: u8 = 17;
    pub const TABARD: u8 = 18;
    /// Equipment slots run 0..EQUIPMENT.
    pub const EQUIPMENT: u8 = 19;
    /// Not worn anywhere: a hearthstone.
    pub const NONE: u8 = 255;
    /// The backpack's first slot, in the same numbering.
    pub const PACK_FIRST: u8 = 23;
    pub const PACK_SLOTS: u8 = 16;
}

/// The equipment slot of each armour piece, in set order.
pub const PIECE_SLOTS: [u8; 8] = [
    slot::HEAD,
    slot::SHOULDERS,
    slot::CHEST,
    slot::WAIST,
    slot::LEGS,
    slot::FEET,
    slot::WRISTS,
    slot::HANDS,
];

/// Stat ids as the client knows them (`ItemModType`).
pub mod stat {
    pub const AGILITY: u32 = 3;
    pub const STRENGTH: u32 = 4;
    pub const INTELLECT: u32 = 5;
    pub const SPIRIT: u32 = 6;
    pub const STAMINA: u32 = 7;
}

/// One item template: what the tooltip shows and what the game uses.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemTemplate {
    pub entry: u32,
    pub name: String,
    pub display_id: u32,
    pub quality: u8,
    pub item_class: u32,
    pub subclass: u32,
    pub inventory_type: u32,
    /// Where it's worn.
    pub slot: u8,
    /// Which content class may wear it; `None` for anyone.
    pub class: Option<usize>,
    pub item_level: u32,
    pub required_level: u32,
    pub stats: Vec<(u32, i32)>,
    /// Weapon damage range and swing time.
    pub damage: Option<(f32, f32)>,
    pub delay_ms: u32,
    pub armor: u32,
    pub description: String,
    pub sheath: u32,
    pub material: u32,
    pub tier: u8,
    /// Counts towards the class set bonus.
    pub set_piece: bool,
    /// The enchant (and so the glow) a fresh copy comes with.
    pub enchant: u32,
    /// A spell the item casts when used, and its cooldown in milliseconds.
    pub use_spell: Option<(u32, i32)>,
}

impl ItemTemplate {
    pub fn stat(&self, id: u32) -> i32 {
        self.stats.iter().filter(|(s, _)| *s == id).map(|(_, v)| v).sum()
    }
    /// A rough worth for comparing two pieces for the same slot.
    pub fn score(&self) -> f32 {
        let stats: i32 = self.stats.iter().map(|(_, v)| *v).sum();
        let dps = self.damage.map_or(0.0, |(a, b)| (a + b) / 2.0 / (self.delay_ms.max(1) as f32 / 1000.0));
        stats as f32 + self.armor as f32 * 0.05 + dps * 2.0
    }
}

/// What an entry number says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GearKey {
    pub class: usize,
    pub tier: u8,
    pub quality: u8,
    pub piece: u8,
    pub archetype: u8,
}

impl GearKey {
    pub fn entry(self) -> u32 {
        FIRST_GEAR_ENTRY
            + self.class as u32 * 20_000
            + self.tier as u32 * 2_000
            + self.quality as u32 * 300
            + self.piece as u32 * 25
            + self.archetype as u32
    }

    pub fn decode(entry: u32) -> Option<Self> {
        let n = entry.checked_sub(FIRST_GEAR_ENTRY)?;
        let class = (n / 20_000) as usize;
        let n = n % 20_000;
        let tier = (n / 2_000) as u8;
        let n = n % 2_000;
        let quality = (n / 300) as u8;
        let n = n % 300;
        let piece = (n / 25) as u8;
        let archetype = (n % 25) as u8;
        if tier > MAX_TIER || quality > QUALITY_LEGENDARY || piece > OFF_HAND_PIECE || archetype >= ARCHETYPES {
            return None;
        }
        Some(Self { class, tier, quality, piece, archetype })
    }
}

/// The quality drops of a tier come in by default: greens in the first cycle, blues after, epics
/// from Tier 2 on. Bosses add one step.
pub fn tier_quality(tier: u8) -> u8 {
    match tier {
        0 => QUALITY_UNCOMMON,
        1 => QUALITY_RARE,
        _ => QUALITY_EPIC,
    }
}

/// A ware's entry for a hero of `class` bought at `tier`.
pub fn ware_entry(ware: usize, class: usize, tier: u8) -> u32 {
    FIRST_WARE_ENTRY + ware as u32 * 100 + class as u32 * 10 + tier.min(MAX_TIER) as u32
}

/// (ware, class, tier) from a ware's entry.
pub fn decode_ware(entry: u32) -> Option<(usize, usize, u8)> {
    let n = entry.checked_sub(FIRST_WARE_ENTRY)?;
    if n >= 100 * 100 {
        return None;
    }
    Some(((n / 100) as usize, (n / 10 % 10) as usize, (n % 10) as u8))
}

/// Whether an item made for slot `made` goes in slot `s`: rings and trinkets come in pairs.
pub fn fits_slot(made: u8, s: u8) -> bool {
    made == s
        || matches!((made, s), (slot::FINGER1, slot::FINGER2) | (slot::FINGER2, slot::FINGER1))
        || matches!((made, s), (slot::TRINKET1, slot::TRINKET2) | (slot::TRINKET2, slot::TRINKET1))
}

fn ware_template(content: &Content, entry: u32) -> Option<ItemTemplate> {
    let (w, class, tier) = decode_ware(entry)?;
    let ware = content.wares.get(w)?;
    let cdef = content.classes.get(class)?;
    let budget = &content.gear.budget;
    let q = budget.quality.get(ware.quality.saturating_sub(1) as usize).copied().unwrap_or(1.0);
    let points = budget.base * budget.tier_mult.powi(tier as i32) * q * ware.slot.weight();
    let main_stat = match cdef.main_stat.as_str() {
        "strength" => stat::STRENGTH,
        "agility" => stat::AGILITY,
        "spirit" => stat::SPIRIT,
        _ => stat::INTELLECT,
    };
    let mut stats: Vec<(u32, i32)> = Vec::new();
    for (k, share) in &ware.stats {
        let id = match k.as_str() {
            "stamina" => stat::STAMINA,
            "strength" => stat::STRENGTH,
            "agility" => stat::AGILITY,
            "intellect" => stat::INTELLECT,
            "spirit" => stat::SPIRIT,
            _ => main_stat,
        };
        let v = (points * share).round().max(1.0) as i32;
        match stats.iter_mut().find(|(s, _)| *s == id) {
            Some((_, n)) => *n += v,
            None => stats.push((id, v)),
        }
    }
    let back = ware.slot == crate::wares::WareSlot::Back;
    let head = ware.slot == crate::wares::WareSlot::Head;
    // A helm is of the wearer's own armour: plate for a warrior, cloth for a mage.
    let (head_subclass, head_material) = match cdef.armor.as_str() {
        "plate" => (4, 6),
        "mail" => (3, 5),
        "leather" => (2, 8),
        _ => (1, 7),
    };
    let tier_i = tier as usize;
    let item_level = budget.item_level.get(tier_i).or(budget.item_level.last()).copied().unwrap_or(1) + ware.quality as u32 * 2;
    Some(ItemTemplate {
        entry,
        name: ware.name.clone(),
        display_id: ware.display,
        quality: ware.quality,
        item_class: 4,
        subclass: if back { 1 } else if head { head_subclass } else { 0 },
        inventory_type: ware.slot.inventory_type(),
        slot: ware.slot.slots()[0],
        class: ware.class,
        item_level,
        required_level: 1,
        stats,
        damage: None,
        delay_ms: 0,
        armor: if back {
            (budget.armor.get("cloth").copied().unwrap_or(30.0) * 0.6 * budget.tier_mult.powi(tier as i32)).round() as u32
        } else if head {
            (budget.armor.get(cdef.armor.as_str()).copied().unwrap_or(30.0) * 0.8 * budget.tier_mult.powi(tier as i32)).round() as u32
        } else {
            0
        },
        description: ware.text.clone(),
        sheath: 0,
        material: if back { 7 } else if head { head_material } else { 4 },
        tier,
        set_piece: false,
        enchant: 0,
        use_spell: None,
    })
}

/// The template for `entry`, if it is one of ours.
pub fn template(content: &Content, entry: u32) -> Option<ItemTemplate> {
    if entry >= FIRST_WARE_ENTRY {
        return ware_template(content, entry);
    }
    match entry {
        STARTER_SHIRT => return Some(plain(entry, "Recruit's Shirt", 1817, slot::SHIRT, 4)),
        STARTER_PANTS => return Some(plain(entry, "Recruit's Trousers", 1818, slot::LEGS, 7)),
        e if (STARTER_WAND..STARTER_WAND + WANDS.len() as u32).contains(&e) => {
            let (key, display) = WANDS[(e - STARTER_WAND) as usize];
            let mut t = plain(entry, "Basin Wand", display, slot::RANGED, 26);
            t.item_class = 2;
            t.subclass = 19;
            t.material = 1;
            t.class = content.class_index(key);
            t.description = "Your shots at range come from it.".into();
            return Some(t);
        }
        HEARTHSTONE => {
            let mut t = plain(entry, "Hearthstone", 6418, slot::NONE, 0);
            t.item_class = 15;
            t.subclass = 0;
            t.material = 0;
            t.description = "Channel for 5 sec to return to the Ancient of the Basin. You can still be hit.".into();
            t.use_spell = Some((crate::sim::HEARTH_SPELL_ID, (content.game.hearth_cooldown * 1000.0) as i32));
            return Some(t);
        }
        _ => {}
    }
    let key = GearKey::decode(entry)?;
    let class = content.classes.get(key.class)?;
    let set = content.set_for(&class.key);
    let budget = &content.gear.budget;
    let tier = key.tier as usize;
    let q = budget.quality.get(key.quality.saturating_sub(1) as usize).copied().unwrap_or(1.0);
    let tier_mult = budget.tier_mult.powi(key.tier as i32);
    let required_level = budget.required_level.get(tier).or(budget.required_level.last()).copied().unwrap_or(1);
    let item_level = budget.item_level.get(tier).or(budget.item_level.last()).copied().unwrap_or(1)
        + key.quality as u32 * 2;
    let set_name = set.names.get(tier).or(set.names.last()).cloned().unwrap_or_default();
    let main_stat = match class.main_stat.as_str() {
        "strength" => stat::STRENGTH,
        "agility" => stat::AGILITY,
        "spirit" => stat::SPIRIT,
        _ => stat::INTELLECT,
    };
    let (stamina_share, extra_spirit) = match key.archetype {
        1 => (0.3, 0.0),
        2 => (0.7, 0.0),
        _ => (0.5, if main_stat == stat::INTELLECT && class.key == "priest" { 0.2 } else { 0.0 }),
    };
    let split = |points: f32| -> Vec<(u32, i32)> {
        let stamina = (points * stamina_share).round() as i32;
        let spirit = (points * extra_spirit).round() as i32;
        let main = (points - stamina as f32 - spirit as f32).round().max(0.0) as i32;
        let mut s = vec![(main_stat, main), (stat::STAMINA, stamina)];
        if spirit > 0 {
            s.push((stat::SPIRIT, spirit));
        }
        s.retain(|(_, v)| *v > 0);
        s
    };
    let description = set_description(content, class.key.as_str(), key);

    if key.piece == WEAPON_PIECE {
        let wt = tier.min(set.weapons.len() - 1);
        let legendary = key.quality == QUALITY_LEGENDARY;
        let display_id = if legendary { set.legendary_weapon } else { set.weapons[wt] };
        let name = if legendary { set.legendary_name.clone() } else { set.weapon_names[wt].clone() };
        let name = if key.quality <= QUALITY_COMMON { format!("Worn {name}") } else { name };
        // The starting weapon (tier 0, common) is the class's level-1 damage.
        let speed = class.attack_secs;
        let mult = tier_mult * q * if key.quality <= QUALITY_COMMON { 1.0 } else { 1.12 };
        let (lo, hi) = (class.damage[0] as f32 * mult, class.damage[1] as f32 * mult);
        let (item_class, subclass, inventory_type, sheath) = match class.weapon_slot {
            slot::RANGED => (2, 2, 15, 0),            // bow
            _ if class.key == "warrior" => (2, 8, 17, 1), // two-handed sword
            _ => (2, 10, 17, 2),                      // staff
        };
        let stats = if key.quality <= QUALITY_COMMON { Vec::new() } else { split(budget.base * tier_mult * q) };
        let glow = if legendary {
            set.weapon_glow.last().copied().unwrap_or(0)
        } else {
            set.weapon_glow.get(wt).copied().unwrap_or(0)
        };
        return Some(ItemTemplate {
            entry,
            name,
            display_id,
            quality: key.quality,
            item_class,
            subclass,
            inventory_type,
            slot: class.weapon_slot,
            class: Some(key.class),
            item_level,
            required_level: if key.quality <= QUALITY_COMMON { 1 } else { required_level },
            stats,
            damage: Some((lo.round(), hi.round())),
            delay_ms: (speed * 1000.0).round() as u32,
            armor: 0,
            description,
            sheath,
            material: 1,
            tier: key.tier,
            set_piece: false,
            enchant: glow,
            use_spell: None,
        });
    }

    if matches!(key.piece, ONE_HAND_PIECE | SHIELD_PIECE | OFF_HAND_PIECE) {
        // A path's gear: only for a class with path gear (the warrior).
        let pg = set.path_gear.as_ref()?;
        let legendary = key.quality == QUALITY_LEGENDARY;
        let k = (key.piece - ONE_HAND_PIECE) as usize;
        let (names, looks) = match key.piece {
            ONE_HAND_PIECE => (&pg.one_hand_names, &pg.one_hands),
            SHIELD_PIECE => (&pg.shield_names, &pg.shields),
            _ => (&pg.off_hand_names, &pg.off_hands),
        };
        let wt = tier.min(looks.len().saturating_sub(1));
        let display_id = if legendary { pg.legendary[k] } else { *looks.get(wt)? };
        let name = if legendary { pg.legendary_names[k].clone() } else { names.get(wt)?.clone() };
        let name = if key.quality <= QUALITY_COMMON { format!("Worn {name}") } else { name };
        let mult = tier_mult * q * if key.quality <= QUALITY_COMMON { 1.0 } else { 1.12 };
        let stats = if key.quality <= QUALITY_COMMON { Vec::new() } else { split(budget.base * tier_mult * q * if key.piece == ONE_HAND_PIECE { 0.6 } else { 0.4 }) };
        let glow = if legendary { set.weapon_glow.last().copied().unwrap_or(0) } else { set.weapon_glow.get(wt).copied().unwrap_or(0) };
        let mut t = ItemTemplate {
            entry,
            name,
            display_id,
            quality: key.quality,
            item_class: 2,
            subclass: 7,
            inventory_type: 21,
            slot: slot::MAIN_HAND,
            class: Some(key.class),
            item_level,
            required_level: if key.quality <= QUALITY_COMMON { 1 } else { required_level },
            stats,
            damage: None,
            delay_ms: 0,
            armor: 0,
            description: String::new(),
            sheath: 3,
            material: 1,
            tier: key.tier,
            set_piece: false,
            enchant: 0,
            use_spell: None,
        };
        match key.piece {
            ONE_HAND_PIECE => {
                // Quicker and lighter than the greatsword: the path's talent makes up the rest.
                let speed = class.attack_secs * 0.8;
                t.damage = Some(((class.damage[0] as f32 * mult * 0.85).round(), (class.damage[1] as f32 * mult * 0.85).round()));
                t.delay_ms = (speed * 1000.0).round() as u32;
                t.enchant = glow;
                t.description = "One-handed: a shield or a second weapon goes in the other hand.".into();
            }
            SHIELD_PIECE => {
                t.item_class = 4;
                t.subclass = 6;
                t.inventory_type = 14;
                t.slot = slot::OFF_HAND;
                t.sheath = 4;
                t.material = 6;
                t.armor = (budget.armor.get(&class.armor).copied().unwrap_or(0.0) * 1.2 * tier_mult * q).round() as u32;
                t.description = "A shield: worn on the arm, and in your guard.".into();
            }
            _ => {
                t.subclass = 0;
                t.inventory_type = 22;
                t.slot = slot::OFF_HAND;
                t.damage = Some(((class.damage[0] as f32 * mult * 0.5).round(), (class.damage[1] as f32 * mult * 0.5).round()));
                t.delay_ms = (class.attack_secs * 0.8 * 1000.0).round() as u32;
                t.enchant = glow;
                t.description = "Off-hand: it swings beside your sword.".into();
            }
        }
        return Some(t);
    }

    let piece = key.piece as usize;
    let display_id = display_for(&set.displays, tier, piece)?;
    let weight = budget.slot_weight[piece];
    let points = budget.base * tier_mult * q * weight;
    let armor_base = budget.armor.get(&class.armor).copied().unwrap_or(0.0);
    let subclass = match class.armor.as_str() {
        "plate" => 4,
        "mail" => 3,
        "leather" => 2,
        _ => 1,
    };
    let mut inventory_type = [1, 3, 5, 6, 7, 8, 9, 10][piece];
    if piece == 2 && subclass == 1 {
        inventory_type = 20; // a robe
    }
    let noun = &set.slots[piece];
    Some(ItemTemplate {
        entry,
        name: format!("{set_name} {noun}"),
        display_id,
        quality: key.quality,
        item_class: 4,
        subclass,
        inventory_type,
        slot: PIECE_SLOTS[piece],
        class: Some(key.class),
        item_level,
        required_level,
        stats: split(points),
        damage: None,
        delay_ms: 0,
        armor: (armor_base * weight * tier_mult * q).round() as u32,
        description,
        sheath: 0,
        material: match subclass {
            4 => 6,
            3 => 5,
            _ => 7,
        },
        tier: key.tier,
        set_piece: key.tier >= 1,
        enchant: 0,
        use_spell: None,
    })
}

/// A piece's look: this tier's, else the nearest lower tier's that has one.
fn display_for(displays: &[Vec<u32>], tier: usize, piece: usize) -> Option<u32> {
    let top = tier.min(displays.len() - 1);
    (0..=top).rev().map(|t| displays[t][piece]).find(|&d| d != 0)
}

/// Whether `key` names an item that exists (the armour slot has a look at that tier).
pub fn exists(content: &Content, key: GearKey) -> bool {
    template(content, key.entry()).is_some()
}

fn set_description(content: &Content, class: &str, key: GearKey) -> String {
    if key.tier == 0 || key.piece >= WEAPON_PIECE {
        return String::new();
    }
    match content.bonus_for(class) {
        Some(b) => format!("Basin set (2): {} (4): {} (6): {}", b.two, b.four, b.six),
        None => String::new(),
    }
}

fn plain(entry: u32, name: &str, display_id: u32, slot: u8, inventory_type: u32) -> ItemTemplate {
    ItemTemplate {
        entry,
        name: name.to_string(),
        display_id,
        quality: QUALITY_COMMON,
        item_class: 4,
        subclass: 1,
        inventory_type,
        slot,
        class: None,
        item_level: 1,
        required_level: 1,
        stats: Vec::new(),
        damage: None,
        delay_ms: 0,
        armor: 0,
        description: String::new(),
        sheath: 0,
        material: 7,
        tier: 0,
        set_piece: false,
        enchant: 0,
        use_spell: None,
    }
}

/// The wand a class shoots with, if it shoots with one.
pub fn starter_wand(content: &Content, class: usize) -> Option<u32> {
    let key = &content.classes.get(class)?.key;
    WANDS.iter().position(|(k, _)| k == key).map(|i| STARTER_WAND + i as u32)
}

/// The weapon a class starts with.
pub fn starter_weapon(class: usize) -> u32 {
    GearKey { class, tier: 0, quality: QUALITY_COMMON, piece: WEAPON_PIECE, archetype: 0 }.entry()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip() {
        let k = GearKey { class: 3, tier: 4, quality: QUALITY_EPIC, piece: 7, archetype: 2 };
        assert_eq!(GearKey::decode(k.entry()), Some(k));
        assert!(GearKey::decode(99_999).is_none());
    }

    #[test]
    fn every_class_has_a_starting_kit_and_tiers() {
        let c = Content::builtin();
        for class in 0..c.classes.len() {
            let w = template(&c, starter_weapon(class)).expect("starter weapon");
            assert!(w.damage.is_some());
            assert_eq!(w.required_level, 1);
            for tier in 0..=MAX_TIER {
                let chest = GearKey { class, tier, quality: tier_quality(tier), piece: 2, archetype: 0 };
                let t = template(&c, chest.entry()).expect("every tier has a chest");
                assert!(t.display_id != 0 && !t.stats.is_empty(), "{t:?}");
            }
        }
        // Higher tiers are stronger.
        let a = template(&c, GearKey { class: 0, tier: 1, quality: 3, piece: 2, archetype: 0 }.entry()).unwrap();
        let b = template(&c, GearKey { class: 0, tier: 2, quality: 3, piece: 2, archetype: 0 }.entry()).unwrap();
        assert!(b.score() > a.score() * 1.2);
        assert!(template(&c, STARTER_SHIRT).is_some());
    }
}
