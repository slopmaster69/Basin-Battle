//! Update fields: the indexed values an object carries (health, level, display...), and the
//! templates for the units and players this game creates. Indices are 1.12.1's
//! (`UpdateFields_1_12_1.h`), the same benilla reads.

use std::collections::BTreeMap;

use crate::wire::Body;

pub mod idx {
    pub const GUID: u16 = 0;
    pub const TYPE: u16 = 2;
    pub const ENTRY: u16 = 3;
    pub const SCALE_X: u16 = 4;
    pub const UNIT_TARGET: u16 = 16;
    /// The unit a channel is held on (a guid pair), with `UNIT_CHANNEL_SPELL` its spell.
    pub const UNIT_CHANNEL_OBJECT: u16 = 20;
    pub const UNIT_HEALTH: u16 = 22;
    pub const UNIT_POWER1: u16 = 23;
    pub const UNIT_MAXHEALTH: u16 = 28;
    pub const UNIT_MAXPOWER1: u16 = 29;
    pub const UNIT_LEVEL: u16 = 34;
    pub const UNIT_FACTIONTEMPLATE: u16 = 35;
    pub const UNIT_BYTES_0: u16 = 36;
    /// A creature's weapons: display ids for main hand, off hand and ranged, and two words of
    /// item info each (class, subclass, material, inventory type; then the sheath type).
    pub const UNIT_VIRTUAL_ITEM_SLOT_DISPLAY: u16 = 37;
    pub const UNIT_VIRTUAL_ITEM_INFO: u16 = 40;
    /// Byte 0: the sheath state (0 stowed, 1 melee drawn, 2 ranged drawn).
    pub const UNIT_BYTES_2: u16 = 164;
    pub const UNIT_FLAGS: u16 = 46;
    pub const UNIT_BASEATTACKTIME: u16 = 126;
    pub const UNIT_BOUNDINGRADIUS: u16 = 129;
    pub const UNIT_COMBATREACH: u16 = 130;
    pub const UNIT_DISPLAYID: u16 = 131;
    pub const UNIT_NATIVEDISPLAYID: u16 = 132;
    pub const UNIT_MOUNTDISPLAYID: u16 = 133;
    pub const UNIT_MINDAMAGE: u16 = 134;
    pub const UNIT_MAXDAMAGE: u16 = 135;
    pub const UNIT_BYTES_1: u16 = 138;
    pub const UNIT_DYNAMIC_FLAGS: u16 = 143;
    pub const UNIT_MOD_CAST_SPEED: u16 = 145;
    pub const UNIT_STAT0: u16 = 150;
    pub const UNIT_RESISTANCES: u16 = 155;
    pub const UNIT_BASE_HEALTH: u16 = 163;
    pub const UNIT_ATTACK_POWER: u16 = 165;
    pub const PLAYER_FLAGS: u16 = 190;
    pub const PLAYER_BYTES: u16 = 193;
    pub const PLAYER_BYTES_2: u16 = 194;
    pub const PLAYER_BYTES_3: u16 = 195;
    pub const PLAYER_MOD_DAMAGE_DONE_PCT: u16 = 1215;
    pub const PLAYER_WATCHED_FACTION_INDEX: u16 = 1261;
    pub const UNIT_AURA: u16 = 47;
    pub const UNIT_AURAFLAGS: u16 = 95;
    pub const UNIT_AURALEVELS: u16 = 101;
    pub const UNIT_AURAAPPLICATIONS: u16 = 113;
    pub const UNIT_CHANNEL_SPELL: u16 = 144;
    pub const UNIT_NPC_FLAGS: u16 = 147;
    /// A looping state emote (`Emotes.dbc`): the unit's idle pose while it stands.
    pub const UNIT_NPC_EMOTESTATE: u16 = 148;
    pub const UNIT_RANGEDATTACKTIME: u16 = 128;
    pub const UNIT_MINRANGEDDAMAGE: u16 = 171;
    pub const UNIT_MAXRANGEDDAMAGE: u16 = 172;
    pub const UNIT_RANGED_ATTACK_POWER: u16 = 168;
    /// 12 fields per slot: creator (2), entry, 7 enchants, properties, pad.
    pub const PLAYER_VISIBLE_ITEM_1_0: u16 = 260;
    pub const PLAYER_INV_SLOT_HEAD: u16 = 486;
    pub const PLAYER_PACK_SLOT_1: u16 = 532;
    pub const PLAYER_XP: u16 = 716;
    pub const PLAYER_NEXT_LEVEL_XP: u16 = 717;
    pub const PLAYER_COINAGE: u16 = 1176;
    /// The zones explored (64 words, a bit for each `AreaTable` explore flag): the world map shows
    /// a place's art once its bit is set.
    pub const PLAYER_EXPLORED_ZONES_1: u16 = 1111;
    pub const PLAYER_EXPLORED_ZONES_SLOTS: u16 = 64;
    pub const ITEM_OWNER: u16 = 6;
    pub const ITEM_CONTAINED: u16 = 8;
    pub const ITEM_STACK_COUNT: u16 = 14;
    pub const ITEM_FLAGS: u16 = 21;
    pub const ITEM_ENCHANTMENT: u16 = 22;
    pub const CONTAINER_NUM_SLOTS: u16 = 48;
    /// 2 fields per slot's item guid, 36 slots.
    pub const CONTAINER_SLOT_1: u16 = 50;
    pub const DYNAMICOBJECT_CASTER: u16 = 6;
    pub const DYNAMICOBJECT_BYTES: u16 = 8;
    pub const DYNAMICOBJECT_SPELLID: u16 = 9;
    pub const DYNAMICOBJECT_RADIUS: u16 = 10;
    pub const DYNAMICOBJECT_POS_X: u16 = 11;
    pub const DYNAMICOBJECT_FACING: u16 = 14;
    pub const GAMEOBJECT_DISPLAYID: u16 = 8;
    pub const GAMEOBJECT_FLAGS: u16 = 9;
    pub const GAMEOBJECT_ROTATION: u16 = 10;
    pub const GAMEOBJECT_STATE: u16 = 14;
    pub const GAMEOBJECT_POS_X: u16 = 15;
    pub const GAMEOBJECT_FACING: u16 = 18;
    pub const GAMEOBJECT_FACTION: u16 = 20;
    pub const GAMEOBJECT_TYPE_ID: u16 = 21;
    pub const GAMEOBJECT_ANIMPROGRESS: u16 = 24;
}

/// `TYPEMASK_OBJECT | TYPEMASK_ITEM`.
pub const TYPEMASK_ITEM: u32 = 0x01 | 0x02;
/// `TYPEMASK_OBJECT | TYPEMASK_ITEM | TYPEMASK_CONTAINER`.
pub const TYPEMASK_CONTAINER: u32 = 0x01 | 0x02 | 0x04;
/// `TYPEMASK_OBJECT | TYPEMASK_GAMEOBJECT`.
pub const TYPEMASK_GAMEOBJECT: u32 = 0x01 | 0x20;
/// `TYPEMASK_OBJECT | TYPEMASK_DYNAMICOBJECT`.
pub const TYPEMASK_DYNAMICOBJECT: u32 = 0x01 | 0x40;
/// `UNIT_NPC_FLAG_GOSSIP`.
pub const NPC_FLAG_GOSSIP: u32 = 0x01;
/// `UNIT_FLAG_NOT_SELECTABLE`.
pub const UNIT_FLAG_NOT_SELECTABLE: u32 = 0x0200_0000;
/// `UNIT_DYNFLAG_LOOTABLE`.
pub const UNIT_DYNFLAG_LOOTABLE: u32 = 0x01;

/// `TYPEMASK_OBJECT | TYPEMASK_UNIT`, and with `TYPEMASK_PLAYER`.
pub const TYPEMASK_UNIT: u32 = 0x01 | 0x08;
pub const TYPEMASK_PLAYER: u32 = 0x01 | 0x08 | 0x10;

/// `UNIT_FLAG_PVP_ATTACKABLE`, which every player carries.
pub const UNIT_FLAG_PVP_ATTACKABLE: u32 = 0x08;
/// `UNIT_DYNFLAG_DEAD`: drawn as a corpse.
pub const UNIT_DYNFLAG_DEAD: u32 = 0x20;

/// Faction templates: monsters hostile to everyone, and a friend to everyone.
pub const FACTION_MONSTER: u32 = 14;
pub const FACTION_FRIENDLY: u32 = 35;

/// A sparse set of update fields.
#[derive(Debug, Clone, Default)]
pub struct Fields(BTreeMap<u16, u32>);

impl Fields {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn set(&mut self, i: u16, v: u32) -> &mut Self {
        self.0.insert(i, v);
        self
    }
    pub fn set_f32(&mut self, i: u16, v: f32) -> &mut Self {
        self.set(i, v.to_bits())
    }
    pub fn set_u64(&mut self, i: u16, v: u64) -> &mut Self {
        self.set(i, v as u32);
        self.set(i + 1, (v >> 32) as u32)
    }
    pub fn get(&self, i: u16) -> Option<u32> {
        self.0.get(&i).copied()
    }
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
    /// The update mask and the values, as the wire carries them.
    pub fn write(&self, b: &mut Body) {
        let max = self.0.keys().next_back().copied().unwrap_or(0) as usize;
        let blocks = if self.0.is_empty() { 0 } else { max / 32 + 1 };
        let mut mask = vec![0u32; blocks];
        for &i in self.0.keys() {
            mask[i as usize / 32] |= 1 << (i % 32);
        }
        b.u8(blocks as u8);
        for m in &mask {
            b.u32(*m);
        }
        for v in self.0.values() {
            b.u32(*v);
        }
    }
}

pub struct UnitLook {
    pub guid: u64,
    pub entry: u32,
    pub display_id: u32,
    pub scale: f32,
    pub level: u32,
    pub faction: u32,
    pub health: u32,
    pub max_health: u32,
    pub attack_ms: u32,
    pub damage: [u32; 2],
    pub run_speed: f32,
    pub npc_flags: u32,
    pub unit_flags: u32,
    /// A one-handed sword drawn in the main hand (its display id): a creature model with no
    /// unarmed attack swings with it instead of standing still.
    pub main_hand: u32,
    /// How it's held (`UNIT_VIRTUAL_ITEM_INFO`'s first word); 0: a one-handed sword.
    pub main_hand_info: u32,
    /// A shield on the arm, and a gun or bow (item displays, 0 for none).
    pub off_hand: u32,
    pub ranged: u32,
}

/// The create fields of a creature.
pub fn unit_fields(u: &UnitLook) -> Fields {
    let mut f = Fields::new();
    f.set_u64(idx::GUID, u.guid)
        .set(idx::TYPE, TYPEMASK_UNIT)
        .set(idx::ENTRY, u.entry)
        .set_f32(idx::SCALE_X, u.scale)
        .set(idx::UNIT_HEALTH, u.health)
        .set(idx::UNIT_MAXHEALTH, u.max_health)
        .set(idx::UNIT_LEVEL, u.level)
        .set(idx::UNIT_FACTIONTEMPLATE, u.faction)
        // race 0, class warrior, gender male, power mana (none held)
        .set(idx::UNIT_BYTES_0, 1 << 8)
        .set(idx::UNIT_BASEATTACKTIME, u.attack_ms)
        .set(idx::UNIT_BASEATTACKTIME + 1, u.attack_ms)
        .set_f32(idx::UNIT_BOUNDINGRADIUS, 0.5 * u.scale)
        .set_f32(idx::UNIT_COMBATREACH, 1.5 * u.scale)
        .set(idx::UNIT_DISPLAYID, u.display_id)
        .set(idx::UNIT_NATIVEDISPLAYID, u.display_id)
        .set_f32(idx::UNIT_MINDAMAGE, u.damage[0] as f32)
        .set_f32(idx::UNIT_MAXDAMAGE, u.damage[1] as f32)
        .set_f32(idx::UNIT_MOD_CAST_SPEED, 1.0);
    if u.npc_flags != 0 {
        f.set(idx::UNIT_NPC_FLAGS, u.npc_flags);
    }
    if u.unit_flags != 0 {
        f.set(idx::UNIT_FLAGS, u.unit_flags);
    }
    if u.main_hand != 0 {
        // Weapon (2), one-handed sword (7), metal (1), one-hand (13); sheathed at the side (3).
        let info = if u.main_hand_info != 0 { u.main_hand_info } else { 2 | 7 << 8 | 1 << 16 | 13 << 24 };
        // Two-handed across the back, else at the side.
        let sheath = if info >> 24 == 17 { 1 } else { 3 };
        f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY, u.main_hand)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO, info)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO + 1, sheath)
            .set(idx::UNIT_BYTES_2, 1);
    }
    if u.off_hand != 0 {
        // Armour (4), shield (6), metal (1), shield (14); on the arm.
        f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1, u.off_hand)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO + 2, 4 | 6 << 8 | 1 << 16 | 14 << 24)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO + 3, 4);
    }
    if u.ranged != 0 {
        // Weapon (2), gun (3), wood (2), ranged right (26).
        f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2, u.ranged)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO + 4, 2 | 3 << 8 | 2 << 16 | 26 << 24)
            .set(idx::UNIT_VIRTUAL_ITEM_INFO + 5, 0)
            .set(idx::UNIT_BYTES_2, 2);
    }
    f
}

pub struct PlayerLook {
    pub guid: u64,
    pub race: u8,
    pub class: u8,
    pub gender: u8,
    pub power: u8,
    pub max_power: u32,
    pub appearance: [u8; 5],
    pub display_id: u32,
    pub faction: u32,
    pub level: u32,
    pub health: u32,
    pub max_health: u32,
    pub attack_ms: u32,
    pub damage: [f32; 2],
    pub ranged: bool,
    pub armor: u32,
    /// Strength, agility, stamina, intellect, spirit.
    pub attributes: [i32; 5],
    /// How big they stand.
    pub scale: f32,
}

/// The create fields of a player.
pub fn player_fields(p: &PlayerLook) -> Fields {
    let mut f = Fields::new();
    let power_idx = p.power as u16;
    let a = p.appearance;
    f.set_u64(idx::GUID, p.guid)
        .set(idx::TYPE, TYPEMASK_PLAYER)
        .set_f32(idx::SCALE_X, p.scale)
        .set(idx::UNIT_HEALTH, p.health)
        .set(idx::UNIT_MAXHEALTH, p.max_health)
        // Mana and energy start full; rage starts empty.
        .set(
            idx::UNIT_POWER1 + power_idx,
            if p.power == 1 { 0 } else { p.max_power },
        )
        .set(idx::UNIT_MAXPOWER1 + power_idx, p.max_power)
        .set(idx::UNIT_LEVEL, p.level)
        .set(idx::UNIT_FACTIONTEMPLATE, p.faction)
        .set(
            idx::UNIT_BYTES_0,
            p.race as u32 | (p.class as u32) << 8 | (p.gender as u32) << 16 | (p.power as u32) << 24,
        )
        .set(idx::UNIT_FLAGS, UNIT_FLAG_PVP_ATTACKABLE)
        .set(idx::UNIT_BASEATTACKTIME, p.attack_ms)
        .set(idx::UNIT_BASEATTACKTIME + 1, p.attack_ms)
        .set_f32(idx::UNIT_BOUNDINGRADIUS, 0.389)
        .set_f32(idx::UNIT_COMBATREACH, 1.5)
        .set(idx::UNIT_DISPLAYID, p.display_id)
        .set(idx::UNIT_NATIVEDISPLAYID, p.display_id)
        .set_f32(idx::UNIT_MOD_CAST_SPEED, 1.0)
        .set(idx::UNIT_BASE_HEALTH, p.max_health)
        .set(idx::PLAYER_BYTES, u32::from_le_bytes([a[0], a[1], a[2], a[3]]))
        .set(idx::PLAYER_BYTES_2, a[4] as u32 | 0x02 << 24) // facial hair; rest state normal
        .set(idx::PLAYER_BYTES_3, p.gender as u32)
        .set(idx::PLAYER_WATCHED_FACTION_INDEX, u32::MAX);
    player_stats(&mut f, p);
    for school in 0..7 {
        f.set_f32(idx::PLAYER_MOD_DAMAGE_DONE_PCT + school, 1.0);
    }
    if p.health == 0 {
        f.set(idx::UNIT_DYNAMIC_FLAGS, UNIT_DYNFLAG_DEAD);
    }
    f
}

/// The character sheet's numbers: damage, speed, armour, attributes.
pub fn player_stats(f: &mut Fields, p: &PlayerLook) {
    f.set_f32(idx::SCALE_X, p.scale);
    f.set(idx::UNIT_BASEATTACKTIME, p.attack_ms)
        .set(idx::UNIT_BASEATTACKTIME + 1, p.attack_ms)
        .set(idx::UNIT_RANGEDATTACKTIME, p.attack_ms)
        .set_f32(idx::UNIT_MINDAMAGE, p.damage[0])
        .set_f32(idx::UNIT_MAXDAMAGE, p.damage[1])
        .set_f32(idx::UNIT_MINRANGEDDAMAGE, if p.ranged { p.damage[0] } else { 0.0 })
        .set_f32(idx::UNIT_MAXRANGEDDAMAGE, if p.ranged { p.damage[1] } else { 0.0 })
        .set(idx::UNIT_RESISTANCES, p.armor)
        .set(idx::UNIT_ATTACK_POWER, 0)
        .set(idx::UNIT_RANGED_ATTACK_POWER, 0);
    for (i, v) in p.attributes.iter().enumerate() {
        f.set(idx::UNIT_STAT0 + i as u16, (*v).max(0) as u32);
    }
}

/// One aura slot's words, given the whole bar.
pub fn aura_words(f: &mut Fields, auras: &[(u32, u8); 48], slot: usize) {
    f.set(idx::UNIT_AURA + slot as u16, auras[slot].0);
    let word = slot / 8;
    let mut flags = 0u32;
    for s in word * 8..word * 8 + 8 {
        if auras[s].0 != 0 {
            // Effect 0 live; a buff can't be cancelled (the cards are the build).
            flags |= 0x2 << ((s % 8) * 4);
        }
    }
    f.set(idx::UNIT_AURAFLAGS + word as u16, flags);
    let w4 = slot / 4;
    let mut levels = 0u32;
    let mut apps = 0u32;
    for s in w4 * 4..w4 * 4 + 4 {
        if auras[s].0 != 0 {
            levels |= 60 << ((s % 4) * 8);
            apps |= (auras[s].1.saturating_sub(1) as u32) << ((s % 4) * 8);
        }
    }
    f.set(idx::UNIT_AURALEVELS + w4 as u16, levels);
    f.set(idx::UNIT_AURAAPPLICATIONS + w4 as u16, apps);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mask_covers_the_highest_field() {
        let mut f = Fields::new();
        f.set(2, 7).set(34, 60).set(1261, 1);
        let mut b = Body::new();
        f.write(&mut b);
        let v = b.into_vec();
        assert_eq!(v[0] as usize, 1261 / 32 + 1);
        // three values follow the mask
        assert_eq!(v.len(), 1 + 4 * (1261 / 32 + 1) + 12);
    }
}
