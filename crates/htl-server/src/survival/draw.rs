//! Drawing Elwynn for clients: heroes, creatures, the camp's objects and the land's nodes, danger
//! marks, and the items in a hero's bags.

use htl_sim::items::ItemTemplate;
use htl_survival::{Id, ItemKind, Kit, ObjKind, ObjView};

use super::*;
use crate::fields::{PlayerLook, UnitLook};
use crate::packets::{Create, CreatureInfo, Placement};

const HIGH_UNIT: u64 = 0xF130;
const HIGH_GAMEOBJECT: u64 = 0xF110;
const HIGH_ITEM: u64 = 0x4000;

/// Template entries: creatures by kind, items by kind, the outfit, game objects as handed out.
pub(crate) const CREATURE_ENTRY: u32 = 60_000;
pub(crate) const ITEM_ENTRY: u32 = 300_000;
const OUTFIT_ENTRY: u32 = 300_900;
/// The satchels every hero carries beside the backpack.
const BAG_ENTRY: u32 = 300_950;
const BAG_DISPLAY: u32 = 1244;
const GO_ENTRY: u32 = 400_000;

/// A hero's model: their race's own, a man's or a woman's (1.12's `ChrRaces` displays, by race
/// id 1 to 8).
fn native_display(race: u8, gender: u8) -> u32 {
    let [man, woman] = match race {
        2 => [51, 52],
        3 => [53, 54],
        4 => [55, 56],
        5 => [57, 58],
        6 => [59, 60],
        7 => [1563, 1564],
        8 => [1478, 1479],
        _ => [49, 50],
    };
    if gender == 1 { woman } else { man }
}
/// The faction template human players carry.
const FACTION_HUMAN: u32 = 1;

/// Game object types: a "goober" (right-click uses it) and a plain generic object (drawn only).
const GO_TYPE_GOOBER: u32 = 10;
/// A door: the client swings it by its state and walks into it while it's shut.
const GO_TYPE_DOOR: u32 = 0;
const GO_TYPE_GENERIC: u32 = 5;
/// Not selectable (0x10), and the client keeps it (0x20).
const GO_FLAGS_INERT: u32 = 0x10 | 0x20;

/// Danger marks and the placing rings: the ritual circle (drawn 6.48 yd across at scale 1) and a
/// green rune ring for a spot that's good.
const DANGER_DISPLAY: u32 = 5812;
const DANGER_MODEL_RADIUS: f32 = 6.48;
/// A missile's anchor (see [`SurvivalGame::anchor_create`]) is that circle shrunk to nothing.
const ANCHOR_SCALE: f32 = 0.001;
const GOOD_RING_DISPLAY: u32 = 674;

/// Equipment slots, as the client numbers them.
pub(crate) const SLOT_MAIN_HAND: u8 = 15;
pub(crate) const SLOT_OFF_HAND: u8 = 16;
pub(crate) const SLOT_RANGED: u8 = 17;
pub(crate) const PACK_FIRST: u8 = 23;
/// The backpack's slots; the bags' slots follow on from `PACK_FIRST + BACKPACK` in the hero's
/// numbering, a bag after another.
const BACKPACK: u8 = 16;
/// The equipment slots the four bags sit in.
pub(crate) const BAG_SLOT_FIRST: u8 = 19;
const BAGS: u8 = 4;
/// The most a bag holds.
const BAG_MAX: u8 = 36;

/// Where a piece of armor is worn: (the client's inventory type, its equipment slot).
pub(crate) fn armor_slot(slot: &str) -> (u32, u8) {
    match slot {
        "head" => (1, 0),
        "shoulders" => (3, 2),
        "legs" => (7, 6),
        _ => (5, 4),
    }
}

/// What every survivor wears: a plain jerkin, belt, trousers, boots and bracers (cosmetic).
/// (slot, inventory type, display, name)
const OUTFIT: [(u8, u32, u32, &str); 5] = [
    (4, 5, 3413, "Survivor's Jerkin"),
    (5, 6, 3418, "Survivor's Belt"),
    (6, 7, 3414, "Survivor's Trousers"),
    (7, 8, 18576, "Survivor's Boots"),
    (8, 9, 4448, "Survivor's Bracers"),
];

pub(crate) fn creature_guid(entry: u32, id: Id) -> u64 {
    (HIGH_UNIT << 48) | (u64::from(entry) << 24) | u64::from(id & 0x00FF_FFFF)
}

/// A lingering ground look's DynamicObject, by its mark.
pub(crate) fn area_guid(id: Id) -> u64 {
    (0xF100u64 << 48) | u64::from(id & 0x00FF_FFFF)
}

/// A DynamicObject wearing `spell`'s area look at `pos`, `radius` round, cast by `caster`.
pub(crate) fn area_create(guid: u64, caster: u64, spell: u32, pos: [f32; 3], radius: f32) -> crate::packets::Packet {
    let mut f = Fields::new();
    f.set_u64(idx::GUID, guid)
        .set(idx::TYPE, crate::fields::TYPEMASK_DYNAMICOBJECT)
        .set(idx::ENTRY, spell)
        .set_f32(idx::SCALE_X, 1.0)
        .set_u64(idx::DYNAMICOBJECT_CASTER, caster)
        .set(idx::DYNAMICOBJECT_BYTES, 1)
        .set(idx::DYNAMICOBJECT_SPELLID, spell)
        .set_f32(idx::DYNAMICOBJECT_RADIUS, radius)
        .set_f32(idx::DYNAMICOBJECT_POS_X, pos[0])
        .set_f32(idx::DYNAMICOBJECT_POS_X + 1, pos[1])
        .set_f32(idx::DYNAMICOBJECT_POS_X + 2, pos[2])
        .set_f32(idx::DYNAMICOBJECT_FACING, 0.0);
    crate::packets::update_object(
        &[Create { guid, object_type: crate::packets::TYPE_DYNAMICOBJECT, is_self: false, placement: Placement::At { pos, facing: 0.0 }, fields: &f }],
        &[],
    )
}

pub(crate) fn object_guid(entry: u32, id: Id) -> u64 {
    (HIGH_GAMEOBJECT << 48) | (u64::from(entry & 0x00FF_FFFF) << 24) | u64::from(id & 0x00FF_FFFF)
}

/// A hero's item in a slot: unique per hero and slot.
pub(crate) fn item_guid(player: u64, slot: u8) -> u64 {
    (HIGH_ITEM << 48) | ((player & 0x00FF_FFFF) << 8) | u64::from(slot)
}

impl SurvivalGame {
    /// Guids for everything already in the world (at start-up).
    pub(crate) fn bind_world(&mut self) {
        let mobs: Vec<(Id, usize)> = self.sim.mobs.iter().map(|(id, m)| (*id, m.def)).collect();
        for (id, def) in mobs {
            self.bind(id, creature_guid(CREATURE_ENTRY + def as u32, id));
        }
        for v in self.sim.objects() {
            let guid = self.obj_guid_for(&v);
            self.bind(v.id, guid);
        }
    }

    pub(crate) fn bind(&mut self, id: Id, guid: u64) {
        if let Some(old) = self.guids.insert(id, guid) {
            self.ids.remove(&old);
        }
        self.ids.insert(guid, id);
    }

    pub(crate) fn unbind(&mut self, id: Id) -> Option<u64> {
        let g = self.guids.remove(&id)?;
        self.ids.remove(&g);
        Some(g)
    }

    // --- templates ---------------------------------------------------------------------------------

    /// A game object template's entry for a look, name and kind, handed out (and sent to everyone
    /// here) the first time it's needed.
    pub(crate) fn go_entry(&mut self, display: u32, name: &str, kind: u32) -> u32 {
        let key = (display, name.to_string(), kind);
        if let Some(e) = self.go_entries.get(&key) {
            return *e;
        }
        let entry = GO_ENTRY + self.go_entries.len() as u32;
        self.go_entries.insert(key, entry);
        let p = packets::gameobject_query_response(entry, kind, display, name);
        for (_, s, _) in self.in_world() {
            s.send(p.clone());
        }
        entry
    }

    fn obj_look(v: &ObjView) -> (u32, u32) {
        // Usable things are right-clicked; the rest (ghosts, fires) are only seen.
        if matches!(v.kind, ObjKind::Door { .. }) {
            (GO_TYPE_DOOR, 0x20)
        } else if v.usable {
            (GO_TYPE_GOOBER, 0x20)
        } else {
            (GO_TYPE_GENERIC, GO_FLAGS_INERT)
        }
    }

    pub(crate) fn obj_guid_for(&mut self, v: &ObjView) -> u64 {
        let (kind, _) = Self::obj_look(v);
        let entry = self.go_entry(v.display, &v.name, kind);
        object_guid(entry, v.id)
    }

    /// Every template a client needs, fresh (the client keeps old ones between sessions and never
    /// asks again for one it holds).
    pub(crate) fn push_templates(&mut self, conn: ConnId) {
        for def in 0..self.sim.content.mobs.len() {
            self.creature_query(conn, CREATURE_ENTRY + def as u32);
        }
        let gos: Vec<((u32, String, u32), u32)> = self.go_entries.iter().map(|(k, v)| (k.clone(), *v)).collect();
        for ((display, name, kind), entry) in gos {
            self.send(conn, packets::gameobject_query_response(entry, kind, display, &name));
        }
        // The marks and rings are made on demand: hand their entries out now.
        self.go_entry(DANGER_DISPLAY, "Danger", GO_TYPE_GENERIC);
        self.go_entry(GOOD_RING_DISPLAY, "Building Spot", GO_TYPE_GENERIC);
        for i in 0..self.sim.content.items.len() {
            self.item_query(conn, ITEM_ENTRY + i as u32);
        }
        for k in 0..OUTFIT.len() {
            self.item_query(conn, OUTFIT_ENTRY + k as u32);
        }
        self.item_query(conn, BAG_ENTRY);
        if let Some(w) = self.world_mut(conn) {
            w.fresh_items.clear();
        }
    }

    pub(crate) fn creature_query(&self, conn: ConnId, entry: u32) {
        let p = match entry.checked_sub(CREATURE_ENTRY).and_then(|d| self.sim.content.mobs.get(d as usize)) {
            Some(d) => {
                // A beast, or one of the Horde (humanoid) with its clan under its name.
                let kind = if d.weapon == 0 && d.grip.is_empty() { 1 } else { 7 };
                let sub = d.subname.as_str();
                // Elite (the gold dragon) for the heavy fighters; a boss (the skull) for a champion.
                let rank = if d.boss { 3 } else { u32::from(d.attack.heavy) };
                packets::creature_query_response(&CreatureInfo { entry, name: &d.name, subname: sub, creature_type: kind, rank, display_id: d.display })
            }
            None => packets::creature_query_miss(entry),
        };
        self.send(conn, p);
    }

    pub(crate) fn gameobject_query(&self, conn: ConnId, entry: u32) {
        let found = self.go_entries.iter().find(|(_, e)| **e == entry).map(|(k, _)| k.clone());
        let p = match found {
            Some((display, name, kind)) => packets::gameobject_query_response(entry, kind, display, &name),
            None => packets::gameobject_query_miss(entry),
        };
        self.send(conn, p);
    }

    pub(crate) fn item_query(&self, conn: ConnId, entry: u32) {
        match self.item_template(entry) {
            Some((t, _)) if entry == BAG_ENTRY => {
                self.send(conn, packets::item_query_response_bag(&t, -1, 1, u32::from(self.bag_size())))
            }
            Some((t, stack)) => self.send(conn, packets::item_query_response_stacked(&t, -1, stack)),
            None => self.send(conn, packets::item_query_miss(entry)),
        }
    }

    /// An item's template: the rules' things (their kinds decide how the client treats them) and
    /// the outfit; with how high it stacks.
    pub(crate) fn item_template(&self, entry: u32) -> Option<(ItemTemplate, u32)> {
        let blank = |entry: u32, name: &str, display: u32| ItemTemplate {
            entry,
            name: name.to_string(),
            display_id: display,
            quality: 1,
            item_class: 0,
            subclass: 0,
            inventory_type: 0,
            slot: 0,
            class: None,
            item_level: 5,
            required_level: 0,
            stats: Vec::new(),
            damage: None,
            delay_ms: 0,
            armor: 0,
            description: String::new(),
            sheath: 0,
            material: 0,
            tier: 0,
            set_piece: false,
            enchant: 0,
            use_spell: None,
        };
        if let Some(k) = entry.checked_sub(OUTFIT_ENTRY).filter(|k| (*k as usize) < OUTFIT.len()) {
            let (slot, inv, display, name) = OUTFIT[k as usize];
            let mut t = blank(entry, name, display);
            t.item_class = 4;
            t.subclass = 2; // leather
            t.inventory_type = inv;
            t.slot = slot;
            t.material = 8;
            t.description = "Plain and sturdy.".into();
            return Some((t, 1));
        }
        if entry == BAG_ENTRY {
            let mut t = blank(entry, "Survivor's Satchel", BAG_DISPLAY);
            t.item_class = 1; // a container
            t.inventory_type = 18; // a bag
            t.slot = BAG_SLOT_FIRST;
            t.description = "Holds what the backpack can't.".into();
            return Some((t, 1));
        }
        let i = entry.checked_sub(ITEM_ENTRY)? as usize;
        let c = &self.sim.content;
        let d = c.items.get(i)?;
        let mut t = blank(entry, &d.name, d.display);
        // A weapon's numbers after its words, so two can be told apart.
        t.description = match self.sim.weapon_stats(&d.key) {
            Some(stats) => format!("{} {stats}", d.text),
            None => d.text.clone(),
        };
        // Its perks, and its set's bonuses, after its words.
        let extra = self.sim.gear_words(i);
        if !extra.is_empty() {
            t.description.push(' ');
            t.description.push_str(&extra);
        }
        let mut stack = 1;
        match d.kind {
            ItemKind::Resource => {
                t.item_class = 7; // trade goods
                stack = d.stack.max(1);
            }
            ItemKind::Food => {
                t.item_class = 0;
                t.subclass = 0;
                stack = d.stack.max(1);
                // A use spell, so a right-click in the bags eats it.
                t.use_spell = Some((htl_survival::EAT_SPELL, 0));
            }
            ItemKind::Weapon => {
                let w = c.weapon(&d.key)?;
                t.item_class = 2;
                t.subclass = w.subclass;
                // One-hand at the hip, or two-hand across the back.
                t.inventory_type = if w.two_handed { 17 } else { 13 };
                t.slot = SLOT_MAIN_HAND;
                t.sheath = if w.two_handed { 1 } else { 3 };
                t.material = 1;
                t.damage = Some((w.damage[0] as f32, w.damage[1] as f32));
                t.delay_ms = ((w.windup + w.active + w.recovery) * 1000.0) as u32;
                t.quality = if w.damage[0] > 24 { 2 } else { 1 };
            }
            ItemKind::Shield => {
                let s = c.shield(&d.key)?;
                t.item_class = 4;
                t.subclass = 6;
                t.inventory_type = 14;
                t.slot = SLOT_OFF_HAND;
                t.sheath = 4;
                t.material = 1;
                t.armor = (300.0 * (1.0 - s.cost_per_damage)) as u32;
                t.quality = if s.cost_per_damage < 0.8 { 2 } else { 1 };
            }
            ItemKind::Armor => {
                // Leather or mail, on the chest or the head; its armor value says how much it takes.
                t.item_class = 4;
                t.subclass = if d.weight > 0.0 { 3 } else { 2 };
                let (inv, slot) = armor_slot(&d.slot ) ;
                t.inventory_type = inv;
                t.slot = slot;
                t.material = if d.weight > 0.0 { 5 } else { 8 };
                t.armor = (d.reduce * 1000.0).round() as u32;
                t.quality = if d.reduce >= 0.15 || !d.perks.is_empty() { 2 } else { 1 };
            }
            ItemKind::Pack => {
                t.item_class = 15;
                t.quality = 1;
            }
            ItemKind::Bow => {
                let b = c.ranged(&d.key)?;
                t.item_class = 2;
                t.subclass = b.subclass;
                // A bow, or a crossbow (ranged, right hand).
                t.inventory_type = if b.subclass == 18 { 26 } else { 15 };
                t.slot = SLOT_RANGED;
                t.sheath = 2;
                t.material = 1;
                t.damage = Some((b.damage[0] as f32, b.damage[1] as f32));
                t.delay_ms = ((b.draw_secs + b.recovery) * 1000.0) as u32;
            }
        }
        // A set's piece is rare (blue); a piece with a perk at least uncommon (green).
        if !d.set.is_empty() {
            t.quality = t.quality.max(3);
        } else if !d.perks.is_empty() {
            t.quality = t.quality.max(2);
        }
        Some((t, stack))
    }

    // --- heroes ------------------------------------------------------------------------------------

    /// The gear a hero shows: (slot, item entry), the outfit included.
    pub(crate) fn worn(&self, id: Id) -> Vec<(u8, u32)> {
        let Some(p) = self.sim.players.get(&id) else { return Vec::new() };
        // The plain outfit, with what armor they wear in its place.
        let mut v: Vec<(u8, u32)> = OUTFIT
            .iter()
            .enumerate()
            .filter(|(_, (slot, ..))| ! p.armor.keys().any(|a| armor_slot(a).1 == *slot))
            .map(|(k, (slot, ..))| (*slot, OUTFIT_ENTRY + k as u32))
            .collect();
        for (slot, item) in &p.armor {
            v.push((armor_slot(slot).1, ITEM_ENTRY + *item as u32));
        }
        v.push((SLOT_MAIN_HAND, ITEM_ENTRY + p.weapon as u32));
        // A two-handed grip leaves no hand for the shield.
        if !self.sim.weapon_of(p).two_handed {
            v.push((SLOT_OFF_HAND, ITEM_ENTRY + p.shield as u32));
        }
        v.push((SLOT_RANGED, ITEM_ENTRY + p.bow as u32));
        v
    }

    /// How many slots each of the four bags has: what the rules let a hero carry past the
    /// backpack, shared out (0: no bags).
    pub(crate) fn bag_size(&self) -> u8 {
        let past = self.sim.content.player.bag_slots.saturating_sub(usize::from(BACKPACK));
        past.div_ceil(usize::from(BAGS)).min(usize::from(BAG_MAX)) as u8
    }

    /// The hero's slot for slot `slot` of container `bag` as the client numbers them (255 the
    /// backpack, with slots from 23; 19 to 22 the bags, from 0).
    pub(crate) fn pack_slot(&self, bag: u8, slot: u8) -> Option<u8> {
        let size = self.bag_size();
        match bag {
            255 => (PACK_FIRST..PACK_FIRST + BACKPACK).contains(&slot).then_some(slot),
            b if (BAG_SLOT_FIRST..BAG_SLOT_FIRST + BAGS).contains(&b) && slot < size => {
                Some(PACK_FIRST + BACKPACK + (b - BAG_SLOT_FIRST) * size + slot)
            }
            _ => None,
        }
    }

    /// The guid of what's in slot `slot` of container `bag` (255: the hero's own array: worn
    /// gear, the bags themselves and the backpack).
    pub(crate) fn seat_guid(&self, owner: u64, bag: u8, slot: u8) -> Option<u64> {
        let k = if bag == 255 { Some(slot).filter(|s| *s < PACK_FIRST + BACKPACK) } else { self.pack_slot(bag, slot) };
        k.map(|k| item_guid(owner, k))
    }

    /// Where the client finds a hero's slot: in bag `n` (from 0) at an index, or the backpack's.
    fn bag_of(&self, slot: u8) -> Option<(u8, u8)> {
        let size = self.bag_size().max(1);
        let k = slot.checked_sub(PACK_FIRST + BACKPACK)?;
        Some((k / size, k % size))
    }

    /// What the hero's own bags hold: (pack slot, entry, count), a stack to a slot.
    pub(crate) fn pack(&self, id: Id) -> Vec<(u8, u32, u32)> {
        let Some(p) = self.sim.players.get(&id) else { return Vec::new() };
        let mut out = Vec::new();
        let mut slot = PACK_FIRST;
        let end = PACK_FIRST + BACKPACK + BAGS * self.bag_size();
        for (i, n) in &p.bag {
            let stack = self.sim.stack_for(*i, p.pack);
            let mut left = *n;
            while left > 0 && slot < end {
                let k = left.min(stack);
                out.push((slot, ITEM_ENTRY + *i as u32, k));
                left -= k;
                slot += 1;
            }
        }
        out
    }

    /// The fields that show a hero's gear and how they hold it.
    pub(crate) fn gear_fields(&self, id: Id, f: &mut Fields) {
        for slot in 0..19u16 {
            f.set(idx::PLAYER_VISIBLE_ITEM_1_0 + 12 * slot, 0);
        }
        for (slot, entry) in self.worn(id) {
            f.set(idx::PLAYER_VISIBLE_ITEM_1_0 + 12 * slot as u16, entry);
        }
        let kit = self.sim.players.get(&id).map_or(Kit::Melee, |p| p.kit);
        let sheathed = self.sim.players.get(&id).is_some_and(|p| p.sheathed);
        // Weapons out: the axe and shield in hand, or the bow; put away (R), on the back.
        f.set(idx::UNIT_BYTES_2, if sheathed { 0 } else if kit == Kit::Bow { 2 } else { 1 });
        if let Some(p) = self.sim.players.get(&id) {
            if let Some(w) = self.sim.content.weapon(&self.sim.content.items[p.weapon].key) {
                f.set_f32(idx::UNIT_MINDAMAGE, w.damage[0] as f32).set_f32(idx::UNIT_MAXDAMAGE, w.damage[1] as f32);
            }
        }
    }

    pub(crate) fn player_create(&self, id: Id, is_self: bool) -> Packet {
        let p = &self.sim.players[&id];
        let guid = self.guid_of(id).unwrap_or(0);
        let max_stamina = self.sim.content.player.stamina.round() as u32;
        let look = PlayerLook {
            guid,
            race: p.look.race,
            class: htl_survival::content::class_id_of(&p.class),
            gender: p.look.gender,
            power: 3,
            max_power: max_stamina,
            appearance: p.look.bytes(),
            display_id: native_display(p.look.race, p.look.gender),
            faction: FACTION_HUMAN,
            level: 1,
            health: p.health,
            max_health: p.max_health,
            attack_ms: 2000,
            damage: [0.0, 0.0],
            ranged: false,
            armor: 0,
            attributes: [0; 5],
            scale: 1.0,
        };
        let mut f = fields::player_fields(&look);
        f.set(idx::UNIT_POWER1 + 3, p.stamina.round() as u32);
        if matches!(p.action, htl_survival::Action::Guard) {
            f.set(idx::UNIT_NPC_EMOTESTATE, self.guard_pose(id));
        }
        self.gear_fields(id, &mut f);
        // The whole of the world map shown: every place is known from the start.
        if is_self {
            for slot in 0..idx::PLAYER_EXPLORED_ZONES_SLOTS {
                f.set(idx::PLAYER_EXPLORED_ZONES_1 + slot, u32::MAX);
            }
        }
        packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_PLAYER,
                is_self,
                placement: Placement::Living { pos: p.pos, facing: p.facing, speeds: packets::speeds_turning(p.sent_speed, self.sim.content.player.turn_rate) },
                fields: &f,
            }],
            &[],
        )
    }

    // --- creatures ---------------------------------------------------------------------------------

    pub(crate) fn mob_create(&self, id: Id) -> Option<Packet> {
        let m = self.sim.mobs.get(&id)?;
        let d = &self.sim.content.mobs[m.def];
        let guid = self.guid_of(id)?;
        let level = ((d.health as f32 / 25.0).round() as u32).clamp(3, 12);
        let look = UnitLook {
            guid,
            entry: CREATURE_ENTRY + m.def as u32,
            display_id: d.display,
            scale: d.scale,
            level,
            faction: fields::FACTION_MONSTER,
            health: m.health,
            max_health: m.max_health,
            attack_ms: 2000,
            damage: [d.attack.damage; 2],
            run_speed: d.run_speed,
            npc_flags: 0,
            unit_flags: 0,
            main_hand: d.weapon,
            main_hand_info: grip_info(&d.grip),
            off_hand: d.shield,
            ranged: 0,
        };
        let mut f = fields::unit_fields(&look);
        // A dagger in the off hand; a bow in its hands.
        if d.off_hand != 0 && d.shield == 0 {
            f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1, d.off_hand)
                .set(idx::UNIT_VIRTUAL_ITEM_INFO + 2, 2 | 15 << 8 | 1 << 16 | 13 << 24)
                .set(idx::UNIT_VIRTUAL_ITEM_INFO + 3, 3);
        }
        if d.bow != 0 {
            f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2, d.bow)
                .set(idx::UNIT_VIRTUAL_ITEM_INFO + 4, 2 | 2 << 8 | 2 << 16 | 15 << 24)
                .set(idx::UNIT_VIRTUAL_ITEM_INFO + 5, 0)
                .set(idx::UNIT_BYTES_2, 2);
        }
        // A shield up between blows, for those who arrive meanwhile.
        if m.guarding && m.health > 0 {
            f.set(idx::UNIT_NPC_EMOTESTATE, super::play::POSE_SHIELD);
        }
        if m.health == 0 {
            f.set(idx::UNIT_DYNAMIC_FLAGS, fields::UNIT_DYNFLAG_DEAD);
        }
        Some(packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_UNIT,
                is_self: false,
                placement: Placement::Living { pos: m.pos, facing: m.facing, speeds: packets::speeds(d.run_speed.max(1.0)) },
                fields: &f,
            }],
            &[],
        ))
    }

    // --- objects -----------------------------------------------------------------------------------

    #[allow(clippy::too_many_arguments)]
    /// A door's state: open (0) or shut (1); everything else rests shut (its idle pose).
    pub(crate) fn obj_state(&self, v: &ObjView) -> u32 {
        match &v.kind {
            ObjKind::Door { key } if self.sim.door_is_open(key) => 0,
            _ => 1,
        }
    }

    fn go_packet(guid: u64, entry: u32, display: u32, kind: u32, flags: u32, scale: f32, pos: [f32; 3], facing: f32,
    ) -> Packet {
        Self::go_packet_state(guid, entry, display, kind, flags, scale, pos, facing, 1)
    }

    #[allow(clippy::too_many_arguments)]
    fn go_packet_state(
        guid: u64,
        entry: u32,
        display: u32,
        kind: u32,
        flags: u32,
        scale: f32,
        pos: [f32; 3],
        facing: f32,
        state: u32,
    ) -> Packet {
        let mut f = Fields::new();
        let (s, c) = (facing / 2.0).sin_cos();
        f.set_u64(idx::GUID, guid)
            .set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT)
            .set(idx::ENTRY, entry)
            .set_f32(idx::SCALE_X, scale)
            .set(idx::GAMEOBJECT_DISPLAYID, display)
            .set(idx::GAMEOBJECT_FLAGS, flags)
            .set_f32(idx::GAMEOBJECT_ROTATION + 2, s)
            .set_f32(idx::GAMEOBJECT_ROTATION + 3, c)
            .set(idx::GAMEOBJECT_STATE, state)
            .set_f32(idx::GAMEOBJECT_POS_X, pos[0])
            .set_f32(idx::GAMEOBJECT_POS_X + 1, pos[1])
            .set_f32(idx::GAMEOBJECT_POS_X + 2, pos[2])
            .set_f32(idx::GAMEOBJECT_FACING, facing)
            .set(idx::GAMEOBJECT_FACTION, 35)
            .set(idx::GAMEOBJECT_TYPE_ID, kind)
            .set(idx::GAMEOBJECT_ANIMPROGRESS, 100);
        packets::update_object(
            &[Create { guid, object_type: packets::TYPE_GAMEOBJECT, is_self: false, placement: Placement::At { pos, facing }, fields: &f }],
            &[],
        )
    }

    /// An object's create packets: the thing, and for a station being placed, the ring under it
    /// (green where it can go, red where it can't).
    pub(crate) fn obj_create(&mut self, id: Id) -> Vec<Packet> {
        let Some(v) = self.sim.object(id) else { return Vec::new() };
        let (kind, flags) = Self::obj_look(&v);
        let entry = self.go_entry(v.display, &v.name, kind);
        let guid = object_guid(entry, id);
        self.bind(id, guid);
        let state = self.obj_state(&v);
        let mut out = vec![Self::go_packet_state(guid, entry, v.display, kind, flags, v.scale, v.pos, v.facing, state,
        )];
        if let ObjKind::Ghost { def, ok, .. } = v.kind {
            let r = self.sim.content.stations[def].footprint;
            let (display, name, scale) =
                if ok { (GOOD_RING_DISPLAY, "Building Spot", (r / 1.5).max(0.4)) } else { (DANGER_DISPLAY, "Danger", (r / DANGER_MODEL_RADIUS).max(0.15)) };
            let e = self.go_entry(display, name, GO_TYPE_GENERIC);
            let ring = object_guid(e, id) ^ (1 << 47);
            self.rings.insert(id, ring);
            out.push(Self::go_packet(ring, e, display, GO_TYPE_GENERIC, GO_FLAGS_INERT, scale, v.pos, 0.0));
        }
        out
    }

    /// Whose a station being placed is (its ghost): only they see it, so it never stands in
    /// anyone else's way (a model blocks the walk on the client that has it).
    pub(crate) fn ghost_owner(&self, id: Id) -> Option<Id> {
        match self.sim.objs.get(&id).map(|o| &o.kind) {
            Some(ObjKind::Ghost { player, .. }) => Some(*player),
            _ => None,
        }
    }

    /// Draw an object for everyone who should see it (a ghost: its placer alone).
    pub(crate) fn obj_show(&mut self, id: Id) {
        let packets = self.obj_create(id);
        match self.ghost_owner(id) {
            Some(owner) => {
                if let Some(c) = self.conn_of(owner) {
                    for p in packets {
                        self.send(c, p);
                    }
                }
            }
            None => {
                for p in packets {
                    self.broadcast(&p);
                }
            }
        }
    }

    /// Take an object (and its ring) away for everyone.
    pub(crate) fn obj_destroy(&mut self, id: Id) {
        if let Some(g) = self.unbind(id) {
            self.broadcast(&packets::destroy_object(g));
        }
        if let Some(r) = self.rings.remove(&id) {
            self.broadcast(&packets::destroy_object(r));
        }
    }

    /// An unseen object standing at `pos` to fly a missile from there, not from a unit (a
    /// boomerang turning back): its guid, and the packet that makes it. It's gone with the
    /// rules' `ObjGone` of `id`.
    pub(crate) fn anchor_create(&mut self, id: Id, pos: [f32; 3]) -> (u64, Packet) {
        let entry = self.go_entry(DANGER_DISPLAY, "", GO_TYPE_GENERIC);
        let guid = object_guid(entry, id);
        self.bind(id, guid);
        (guid, Self::go_packet(guid, entry, DANGER_DISPLAY, GO_TYPE_GENERIC, GO_FLAGS_INERT, ANCHOR_SCALE, pos, 0.0))
    }

    /// A danger circle drawn to `radius`.
    pub(crate) fn mark_create(&mut self, id: Id, pos: [f32; 3], radius: f32) -> Packet {
        let entry = self.go_entry(DANGER_DISPLAY, "Danger", GO_TYPE_GENERIC);
        let guid = object_guid(entry, id);
        self.bind(id, guid);
        self.mark_at.insert(id, (pos, radius));
        Self::go_packet(guid, entry, DANGER_DISPLAY, GO_TYPE_GENERIC, GO_FLAGS_INERT, (radius / DANGER_MODEL_RADIUS).max(0.15), pos, 0.0)
    }

    /// Draw the whole world for a hero who just arrived.
    pub(crate) fn send_world_to(&mut self, conn: ConnId, me: Id) {
        let others: Vec<Id> = self.sim.players.keys().copied().filter(|id| *id != me && self.guids.contains_key(id)).collect();
        for id in others {
            let p = self.player_create(id, false);
            self.send(conn, p);
        }
        // (Not an ambusher still lying in wait.)
        let mobs: Vec<Id> = self.sim.mobs.iter().filter(|(_, m)| !m.hidden)
            .map(|(id, _)| *id).collect();
        for id in mobs {
            if !self.guids.contains_key(&id) {
                let def = self.sim.mobs[&id].def;
                self.bind(id, creature_guid(CREATURE_ENTRY + def as u32, id));
            }
            if let Some(p) = self.mob_create(id) {
                self.send(conn, p);
            }
            let m = &self.sim.mobs[&id];
            if m.path.len() >= 2 {
                let (path, speed) = (m.path.clone(), m.speed);
                if let Some(guid) = self.guid_of(id) {
                    if let Some(p) = self.move_packet(guid, &path, speed) {
                        self.send(conn, p);
                    }
                }
            }
        }
        let me = self.world_of(conn).map(|w| w.id);
        let objs: Vec<Id> = self.sim.objs.keys().copied().collect();
        for id in objs {
            // Someone else's ghost isn't theirs to see.
            if self.ghost_owner(id).is_some_and(|o| Some(o) != me) {
                continue;
            }
            for p in self.obj_create(id) {
                self.send(conn, p);
            }
        }
        let marks: Vec<(Id, ([f32; 3], f32))> = self.mark_at.iter().map(|(k, v)| (*k, *v)).collect();
        let decal = self.world_of(conn).is_some_and(|w| w.addon && w.level >= 5);
        for (id, (pos, r)) in marks {
            if decal {
                let line = self.mark_line(id, pos, r);
                self.ui_line(conn, &line);
            } else {
                let p = self.mark_create(id, pos, r);
                self.send(conn, p);
            }
        }
    }

    /// A creature's blow (or a hero's wind-up) for a client that draws it under the unit:
    /// `TG\t<key>\t<guid>\t<wedge|ring|line>\t<reach>\t<arc>\t<seconds to the blow>\t<block|unblockable|friendly>\t<the parry's window>\t<meant for this hero>\t<a line's width>\t<the roll's window>`
    /// (`TG\t<key>` once it's gone). The window is this hero's own (their weapon's share in it). A
    /// client before the eighth round gets no lines, and no `friendly`. The roll's window: a dodge
    /// started up to this long before the blow lands evades it (`[dodge] evade_to`), which the
    /// add-on's tells cue an unblockable blow by.
    pub(crate) fn telegraph_line(&self, key: Id, t: &htl_survival::Telegraph, hero: Id, level: u8) -> Option<String> {
        let g = self.guid_of(t.mob)?;
        let c = &self.sim.content;
        let ring = t.arc >= 300.0;
        let p = self.sim.players.get(&hero);
        let parry = p.map_or(c.parry.window, |p| c.parry.window + self.sim.weapon_of(p).parry);
        let mob_pos = self.sim.unit_pos(t.mob);
        let near = match (p, mob_pos) {
            (Some(p), Some(m)) => htl_survival::dist2d(p.pos, m) <= t.reach + 3.0,
            _ => false,
        };
        let shape = if t.width > 0.0 {
            "line"
        } else if ring {
            "ring"
        } else {
            "wedge"
        };
        let (kind, focus) = if t.hero {
            let kind = self.hero_tele_kind(t.mob, hero);
            (if level >= 8 { kind } else if kind == "friendly" { return None } else { kind }, t.mob == hero || near || kind != "friendly")
        } else {
            (if t.blockable { "block" } else { "unblockable" }, t.foe == Some(hero) || (ring && near) || p.is_some_and(|p| p.lock == Some(t.mob)))
        };
        Some(format!(
            "TG\t{key}\t{g:#X}\t{shape}\t{:.2}\t{:.0}\t{:.2}\t{kind}\t{parry:.2}\t{}\t{:.2}\t{:.2}",
            t.reach,
            t.arc.min(360.0),
            t.secs,
            u8::from(focus),
            t.width,
            c.dodge.evade_to
        ))
    }

    /// How a hero's ability on the ground shows to `viewer`: theirs or a friend's (`friendly`), or
    /// a foe's (`unblockable`: red).
    fn hero_tele_kind(&self, owner: htl_survival::Id, viewer: htl_survival::Id) -> &'static str {
        if owner == viewer || !self.sim.content.pvp.friendly_fire {
            "friendly"
        } else {
            "unblockable"
        }
    }

    /// A shape laid on the ground at a spot (a sky-fall's ring, a strip), for the eighth-round
    /// clients: `TA\t<key>\t<x>\t<y>\t<z>\t<facing>\t<wedge|ring|line>\t<reach>\t<arc>\t<seconds>\t<kind>\t<width>\t<focus>`
    /// (gone as a `TG\t<key>`).
    pub(crate) fn telegraph_at_line(&self, key: Id, t: &htl_survival::Telegraph, hero: htl_survival::Id) -> String {
        let (at, yaw) = t.at.unwrap_or(([0.0; 3], 0.0));
        let shape = if t.width > 0.0 {
            "line"
        } else if t.arc >= 300.0 {
            "ring"
        } else {
            "wedge"
        };
        let kind = if t.hero { self.hero_tele_kind(t.mob, hero) } else if t.blockable { "block" } else { "unblockable" };
        let near = self.sim.players.get(&hero).is_some_and(|p| htl_survival::dist2d(p.pos, at) <= t.reach + 4.0);
        format!(
            "TA\t{key}\t{:.2}\t{:.2}\t{:.2}\t{yaw:.3}\t{shape}\t{:.2}\t{:.0}\t{:.2}\t{kind}\t{:.2}\t{}",
            at[0],
            at[1],
            at[2],
            t.reach,
            t.arc.min(360.0),
            t.secs,
            t.width,
            u8::from(near || t.mob == hero || kind != "friendly")
        )
    }

    /// A danger mark for the clients that lay it on the ground themselves:
    /// `MK\t<id>\t<x>\t<y>\t<z>\t<radius>\t<seconds left>` (`MK\t<id>` once it's gone).
    pub(crate) fn mark_line(&self, id: Id, pos: [f32; 3], radius: f32) -> String {
        format!("MK\t{id}\t{:.2}\t{:.2}\t{:.2}\t{radius:.2}\t{:.2}\t{}", pos[0], pos[1], pos[2], self.sim.mark_left(id), u8::from(self.sim.mark_friendly(id)))
    }

    pub(crate) fn move_packet(&mut self, guid: u64, path: &[[f32; 3]], speed: f32) -> Option<Packet> {
        if path.len() < 2 || speed <= 0.0 {
            return None;
        }
        let len = htl_sim::geom::path_len(path);
        let duration = ((len / speed) * 1000.0).round().max(1.0) as u32;
        self.spline_id = self.spline_id.wrapping_add(1);
        Some(packets::monster_move(guid, self.spline_id, path, duration))
    }

    // --- items -------------------------------------------------------------------------------------

    /// Bring a hero's client up to date with their items: worn gear and the stacks in their bags.
    pub(crate) fn sync_items(&mut self, conn: ConnId) {
        let Some(w) = self.world_of(conn) else { return };
        let (id, owner) = (w.id, w.guid);
        let mut now: BTreeMap<u64, (u32, u32)> = BTreeMap::new();
        let mut slots: Vec<(u8, u64)> = Vec::new();
        for (slot, entry) in self.worn(id) {
            let g = item_guid(owner, slot);
            now.insert(g, (entry, 1));
            slots.push((slot, g));
        }
        let size = self.bag_size();
        let bags: Vec<u64> = if size == 0 { Vec::new() } else { (0..BAGS).map(|b| item_guid(owner, BAG_SLOT_FIRST + b)).collect() };
        for (b, g) in bags.iter().enumerate() {
            now.insert(*g, (BAG_ENTRY, 1));
            slots.push((BAG_SLOT_FIRST + b as u8, *g));
        }
        // What each bag holds, slot by slot.
        let mut held: Vec<Vec<u64>> = vec![vec![0; usize::from(size)]; bags.len()];
        let mut inside: BTreeMap<u64, u64> = BTreeMap::new();
        for (slot, entry, n) in self.pack(id) {
            let g = item_guid(owner, slot);
            now.insert(g, (entry, n));
            slots.push((slot, g));
            if let Some((b, k)) = self.bag_of(slot) {
                if let Some(bag) = bags.get(usize::from(b)) {
                    held[usize::from(b)][usize::from(k)] = g;
                    inside.insert(g, *bag);
                }
            }
        }
        let bag_fields = |g: u64, f: &mut Fields| {
            if let Some(b) = bags.iter().position(|x| *x == g) {
                f.set(idx::CONTAINER_NUM_SLOTS, u32::from(size));
                for (k, item) in held[b].iter().enumerate() {
                    f.set_u64(idx::CONTAINER_SLOT_1 + 2 * k as u16, *item);
                }
            }
        };
        let before = w.items.clone();
        // Fresh templates for anything new to this client.
        let fresh: Vec<u32> = now.values().map(|(e, _)| *e).filter(|e| !w.fresh_items.contains(e)).collect();
        for e in &fresh {
            self.item_query(conn, *e);
        }
        if let Some(w) = self.world_mut(conn) {
            w.fresh_items.extend(fresh);
        }
        let mut creates: Vec<(u64, Fields)> = Vec::new();
        let mut changed: Vec<(u64, Fields)> = Vec::new();
        let fresh = |g: u64, entry: u32, n: u32| {
            let mut f = Self::item_fields(g, owner, inside.get(&g).copied().unwrap_or(owner), entry, n);
            if bags.contains(&g) {
                f.set(idx::TYPE, fields::TYPEMASK_CONTAINER);
                bag_fields(g, &mut f);
            }
            f
        };
        for (g, (entry, n)) in &now {
            match before.get(g) {
                Some((e, _)) if e != entry => {
                    // Something else in that slot now: a new object.
                    self.send(conn, packets::destroy_object(*g));
                    creates.push((*g, fresh(*g, *entry, *n)));
                }
                None => creates.push((*g, fresh(*g, *entry, *n))),
                Some((_, old)) if old != n => {
                    let mut f = Fields::new();
                    f.set(idx::ITEM_STACK_COUNT, *n);
                    changed.push((*g, f));
                }
                _ if bags.contains(g) => {
                    // A bag already there: what it holds now.
                    let mut f = Fields::new();
                    bag_fields(*g, &mut f);
                    changed.push((*g, f));
                }
                _ => {}
            }
        }
        // The bags first, so what goes in them has somewhere to be.
        creates.sort_by_key(|(g, _)| !bags.contains(g));
        for chunk in creates.chunks(16) {
            let c: Vec<Create> = chunk
                .iter()
                .map(|(g, f)| {
                    let object_type = if bags.contains(g) { packets::TYPE_CONTAINER } else { packets::TYPE_ITEM };
                    Create { guid: *g, object_type, is_self: false, placement: Placement::Nowhere, fields: f }
                })
                .collect();
            self.send(conn, packets::update_object(&c, &[]));
        }
        if !changed.is_empty() {
            let v: Vec<(u64, &Fields)> = changed.iter().map(|(g, f)| (*g, f)).collect();
            self.send(conn, packets::update_object(&[], &v));
        }
        let mut f = Fields::new();
        // Worn gear, then the bags in their slots.
        for slot in 0..u16::from(BAG_SLOT_FIRST + BAGS) {
            let g = slots.iter().find(|(s, _)| u16::from(*s) == slot).map_or(0, |(_, g)| *g);
            f.set_u64(idx::PLAYER_INV_SLOT_HEAD + 2 * slot, g);
        }
        for i in 0..u16::from(BACKPACK) {
            let g = slots.iter().find(|(s, _)| u16::from(*s) == u16::from(PACK_FIRST) + i).map_or(0, |(_, g)| *g);
            f.set_u64(idx::PLAYER_PACK_SLOT_1 + 2 * i, g);
        }
        self.gear_fields(id, &mut f);
        self.send(conn, packets::update_object(&[], &[(owner, &f)]));
        for g in before.keys().filter(|g| !now.contains_key(g)) {
            self.send(conn, packets::destroy_object(*g));
        }
        if let Some(w) = self.world_mut(conn) {
            w.items = now;
        }
    }

    fn item_fields(guid: u64, owner: u64, contained: u64, entry: u32, count: u32) -> Fields {
        let mut f = Fields::new();
        f.set_u64(idx::GUID, guid)
            .set(idx::TYPE, fields::TYPEMASK_ITEM)
            .set(idx::ENTRY, entry)
            .set_f32(idx::SCALE_X, 1.0)
            .set_u64(idx::ITEM_OWNER, owner)
            .set_u64(idx::ITEM_CONTAINED, contained)
            .set(idx::ITEM_STACK_COUNT, count)
            .set(idx::ITEM_FLAGS, 1);
        f
    }

    /// Everyone else sees a hero's new gear.
    pub(crate) fn show_gear(&mut self, id: Id) {
        let Some(guid) = self.guid_of(id) else { return };
        let mut f = Fields::new();
        self.gear_fields(id, &mut f);
        if let Some(c) = self.conn_of(id) {
            self.sync_items(c);
            self.broadcast_except(c, &packets::update_object(&[], &[(guid, &f)]));
        }
    }
}

/// How a creature holds its weapon, as `UNIT_VIRTUAL_ITEM_INFO` packs it (class, subclass,
/// material, inventory type): a one-handed axe at the hip, a two-handed weapon across the back.
fn grip_info(grip: &str) -> u32 {
    let (sub, inv) = match grip {
        "2h" => (1, 17),
        "staff" => (10, 17),
        "polearm" => (6, 17),
        "dagger" => (15, 13),
        "1h" => (0, 13),
        _ => return 0,
    };
    2 | sub << 8 | 1 << 16 | inv << 24
}
