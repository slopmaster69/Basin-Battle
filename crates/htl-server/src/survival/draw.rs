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
/// The camp's people (`[[prop]]` with `npc`): creature templates by prop.
pub(crate) const RESIDENT_ENTRY: u32 = 65_000;
/// Not to be attacked (`UNIT_FLAG_NON_ATTACKABLE`).
const UNIT_FLAG_NON_ATTACKABLE: u32 = 0x02;
pub(crate) const ITEM_ENTRY: u32 = 300_000;
const OUTFIT_ENTRY: u32 = 300_900;
/// The satchels every hero carries beside the backpack.
const BAG_ENTRY: u32 = 300_950;
const BAG_DISPLAY: u32 = 1244;
const GO_ENTRY: u32 = 400_000;

/// Whether a creature (by its key) is one of Call of the Wild's spirit beasts.
fn spirit_beast(key: &str) -> bool {
    htl_survival::sim::hunter::BEASTS.iter().any(|b| b.mob == key)
}

/// A hero's model: their race's own, a man's or a woman's (1.12's `ChrRaces` displays, by race
/// id 1 to 8).
pub(crate) fn native_display(race: u8, gender: u8) -> u32 {
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
/// Ours, read by client patch 0017: a live preview, moved in place by a create sent again for
/// the same guid, never solid and never faded in or out (`PREVIEW_MOVE`); and drawn see-through
/// and tinted light blue (`PREVIEW_GHOST`): a station being placed, and its ring.
pub(crate) const GO_FLAG_PREVIEW_MOVE: u32 = 0x1000_0000;
pub(crate) const GO_FLAG_PREVIEW_GHOST: u32 = 0x2000_0000;
/// Ours, read by client patch 0028: drawn longer along the model's X axis by the factor
/// `GAMEOBJECT_LEVEL` holds as an `f32`'s bits (a rope from one point to another).
pub(crate) const GO_FLAG_STRETCH_X: u32 = 0x4000_0000;
/// The clients that draw a stretched object and patch 0028's displays (`ac19`): the rest are
/// sent no rope (a short piece would lie at one end of the wire), and keep the strip.
pub(crate) const ROPE_LEVEL: u8 = 19;
/// The clients that draw a missile's look (patch 0030's `SetMissileLook`, `ac20`): a ribbon
/// behind it, its size and a spin (the `ML` line).
pub(crate) const MISSILE_LOOK_LEVEL: u8 = 20;
/// The clients that draw a rope from a unit's hand (patch 0031's `SetUnitRope`, `ac21`): a
/// grappling hook's line, a haul's (the `RP` line).
pub(crate) const UNIT_ROPE_LEVEL: u8 = 21;

/// The add-on's `ML` line for a spell's missiles: `ML <spell> <RRGGBB or -> <width> <secs>
/// <scale> <spin> [<caster's guid, for its next missile alone> [<spin axis> [<speed>]]]`.
pub(crate) fn missile_look_line(spell: u32, trail: &str, width: f32, secs: f32, scale: f32, spin: f32, caster: Option<u64>, tumble: bool, speed: f32) -> String {
    let trail = if trail.is_empty() { "-" } else { trail };
    let mut line = format!("ML\t{spell}\t{trail}\t{width:.2}\t{secs:.2}\t{scale:.2}\t{spin:.2}");
    // The tail is written as far as it's needed, `-` in an empty place before a set one: the
    // caster, then a tumble's axis (end over end, patch 0036), then a speed of its own in yards
    // a second (patch 0040; 0: the spell's).
    let caster = caster.map_or_else(|| "-".to_string(), |g| format!("0x{g:016X}"));
    let axis = if tumble { "side" } else { "-" };
    if speed > 0.0 {
        line += &format!("\t{caster}\t{axis}\t{speed:.0}");
    } else if tumble {
        line += &format!("\t{caster}\t{axis}");
    } else if caster != "-" {
        line += &format!("\t{caster}");
    }
    line
}
/// Patch 0028's own display for the dwarven dig site's rope (`ExcavationRope.mdx`; the client's
/// `EXTRA_DISPLAYS`, which must change with this): a straight rope 3.18 yd along its X axis
/// from its origin, 0.14 yd thick, a little above the ground.
const ROPE_DISPLAY: u32 = 60_001;
const ROPE_MODEL_LEN: f32 = 3.18;
/// The model's rope runs this high over its origin (the middle of its 0.22 to 0.41).
const ROPE_MODEL_MID_Z: f32 = 0.315;
/// Its size: about 0.2 yd thick (at 2 it read as a hawser at knee height: visual check, run 020).
const ROPE_SCALE: f32 = 1.4;
/// How high it's strung over the arrows: an ankle's height, where a tripwire catches a foot.
const ROPE_HEIGHT: f32 = 0.2;
const ROPE_ENTRY_NAME: &str = "Tripwire";
/// Ours, read by client patch 0039: the object glows softly white, with a small white light at
/// it (a planted arrow, so it can be found in the grass: Corey's review, "I want the arrows to
/// still have a white glow or indicator"). Older clients ignore it.
pub(crate) const GO_FLAG_GLOW: u32 = 0x0800_0000;

/// Danger marks and the placing rings: the ritual circle (drawn 6.48 yd across at scale 1) and a
/// green rune ring for a spot that's good.
const DANGER_DISPLAY: u32 = 5812;
const DANGER_MODEL_RADIUS: f32 = 6.48;
/// A missile's stand-in (see [`SurvivalGame::stand_in_create`]): a creature of its own template,
/// a grunt's body shrunk to nothing (a body with the casting animations, so the cast's release
/// comes as it does for anyone).
const STAND_IN_ENTRY: u32 = 69_000;
const STAND_IN_DISPLAY: u32 = 1359;
const STAND_IN_SCALE: f32 = 0.001;
const GOOD_RING_DISPLAY: u32 = 674;
/// The clients that have patch 0035's arrow display (`ac22`): for them a planted arrow is a real
/// arrow stood head-down in the ground; the rest keep the feather (`PLANT_LOOK`).
pub(crate) const ARROW_LEVEL: u8 = 22;
/// That display (the client's `EXTRA_DISPLAYS`, which must change with this): the human weapons
/// doodad arrow (60003, `HumanArrow.m2`), placed as the doodad is, at its size; 60002 is the
/// Hunter's own arcane arrow (`ArcaneShot_Missile.mdx`, +X nock to head, which wants a tilt of
/// a quarter turn to stand: run 061 read it as a purple spike), 60004 the plain ammo arrow in
/// flight (+X too). `ARROW_TILT` turns the model about its Y after its facing (radians). Half
/// again the doodad's size: at its own it was lost in the grass (Corey's review, "can you make
/// the ground arrows bigger?").
const ARROW_DISPLAY: u32 = 60_003;
const ARROW_SCALE: f32 = 1.5;
const ARROW_TILT: f32 = 0.0;

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

/// What's worn, slot by slot (item entries): the plain outfit, with the armor in its place, then
/// the weapon, the shield and the bow, the ones there are.
fn worn_of(armor: &[(&str, usize)], weapon: Option<usize>, shield: Option<usize>, bow: Option<usize>) -> Vec<(u8, u32)> {
    let mut v: Vec<(u8, u32)> = OUTFIT
        .iter()
        .enumerate()
        .filter(|(_, (slot, ..))| !armor.iter().any(|(a, _)| armor_slot(a).1 == *slot))
        .map(|(k, (slot, ..))| (*slot, OUTFIT_ENTRY + k as u32))
        .collect();
    for (slot, item) in armor {
        v.push((armor_slot(slot).1, ITEM_ENTRY + *item as u32));
    }
    let hands = [(SLOT_MAIN_HAND, weapon), (SLOT_OFF_HAND, shield), (SLOT_RANGED, bow)];
    v.extend(hands.into_iter().filter_map(|(slot, i)| i.map(|i| (slot, ITEM_ENTRY + i as u32))));
    v
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

/// A rope's game object, by the mark it goes with.
pub(crate) fn rope_guid(id: Id) -> u64 {
    (HIGH_GAMEOBJECT << 48) | (0xFF_FFFF << 24) | u64::from(id & 0x00FF_FFFF)
}

/// The rotation (`GAMEOBJECT_ROTATION`, x y z w) turning a model's X axis from `a` toward `b`:
/// its yaw about Z, then its pitch up or down to `b`'s height.
pub(crate) fn rope_rotation(a: [f32; 3], b: [f32; 3]) -> [f32; 4] {
    let (dx, dy, dz) = (b[0] - a[0], b[1] - a[1], b[2] - a[2]);
    let yaw = dy.atan2(dx);
    let pitch = dz.atan2((dx * dx + dy * dy).sqrt());
    let (sz, cz) = (yaw / 2.0).sin_cos();
    // Raising X toward Z is a turn about −Y.
    let (sy, cy) = (-pitch / 2.0).sin_cos();
    [-sz * sy, cz * sy, sz * cy, cz * cy]
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

    fn obj_look_in(&self, v: &ObjView) -> (u32, u32) {
        let (kind, flags) = Self::obj_look(v);
        // A gate: walked through on a patched client (0017's no-hull flag), raiders still break it.
        // A trap: there the moment it's set. The client fades every new object in over 2 s, and
        // the flag is also its "never fade" (Corey's review: the trap should appear instantly).
        match v.kind {
            ObjKind::Station { def, .. } if self.sim.content.stations.get(def).is_some_and(|s| s.passable) => (kind, flags | GO_FLAG_PREVIEW_MOVE),
            ObjKind::Prop { .. } if self.sim.is_trap_obj(v.id) || self.sim.is_perch_visual_prop(v.id) => (kind, flags | GO_FLAG_PREVIEW_MOVE),
            _ => (kind, flags),
        }
    }

    fn obj_look(v: &ObjView) -> (u32, u32) {
        // Usable things are right-clicked; the rest (ghosts, fires) are only seen.
        if matches!(v.kind, ObjKind::Door { .. }) {
            (GO_TYPE_DOOR, 0x20)
        } else if matches!(v.kind, ObjKind::Ghost { .. }) {
            (GO_TYPE_GENERIC, GO_FLAGS_INERT | GO_FLAG_PREVIEW_MOVE | GO_FLAG_PREVIEW_GHOST)
        } else if v.usable {
            (GO_TYPE_GOOBER, 0x20)
        } else {
            (GO_TYPE_GENERIC, GO_FLAGS_INERT)
        }
    }

    /// One of the camp's people (`[[prop]]` with `npc`): their prop.
    pub(crate) fn resident(&self, v: &ObjView) -> Option<usize> {
        match v.kind {
            ObjKind::Piece { def } if self.sim.content.props.get(def).is_some_and(|d| d.npc) => Some(def),
            _ => None,
        }
    }

    pub(crate) fn obj_guid_for(&mut self, v: &ObjView) -> u64 {
        if let Some(def) = self.resident(v) {
            return creature_guid(RESIDENT_ENTRY + def as u32, v.id);
        }
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
        for (k, d) in self.sim.content.props.iter().enumerate() {
            if d.npc {
                self.creature_query(conn, RESIDENT_ENTRY + k as u32);
            }
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
        // One of the camp's people: a humanoid with their trade under their name.
        if let Some(d) = entry.checked_sub(RESIDENT_ENTRY).and_then(|k| self.sim.content.props.get(k as usize)).filter(|d| d.npc) {
            let info = CreatureInfo { entry, name: &d.name, subname: &d.subname, creature_type: 7, rank: 0, display_id: d.display };
            self.send(conn, packets::creature_query_response(&info));
            return;
        }
        // A missile's stand-in: nameless (nothing to show if a name were ever drawn).
        if entry == STAND_IN_ENTRY {
            let info = CreatureInfo { entry, name: "", subname: "", creature_type: 10, rank: 0, display_id: STAND_IN_DISPLAY };
            self.send(conn, packets::creature_query_response(&info));
            return;
        }
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
        // A set's piece is rare (blue); a piece with a perk at least uncommon (green). Honed, a
        // step better for each tier, up to epic (purple).
        if !d.set.is_empty() {
            t.quality = t.quality.max(3);
        } else if !d.perks.is_empty() {
            t.quality = t.quality.max(2);
        }
        if d.hone > 0 {
            t.quality = (t.quality + d.hone).min(4);
        }
        Some((t, stack))
    }

    // --- heroes ------------------------------------------------------------------------------------

    /// The gear a hero shows: (slot, item entry), the outfit included.
    pub(crate) fn worn(&self, id: Id) -> Vec<(u8, u32)> {
        let Some(p) = self.sim.players.get(&id) else { return Vec::new() };
        let armor: Vec<(&str, usize)> = p.armor.iter().map(|(s, i)| (s.as_str(), *i)).collect();
        // A two-handed grip leaves no hand for the shield; the off hand holds a second weapon, or
        // nothing, or the shield.
        let shield = if self.sim.weapon_of(p).two_handed || p.off_empty {
            None
        } else if self.sim.off_weapon_of(p).is_some() {
            p.off_weapon
        } else {
            Some(p.shield)
        };
        // (A bow ability's bow in hand: the one it draws.)
        worn_of(&armor, Some(p.weapon), shield, Some(self.sim.ranged_shown(p)))
    }

    /// What a hero out of the world wears, from its save: as `worn` would have it in the world
    /// (a key the rules no longer have left off).
    pub(crate) fn worn_saved(&self, s: &PlayerSave) -> Vec<(u8, u32)> {
        let c = &self.sim.content;
        let armor: Vec<(&str, usize)> = s.armor.iter().filter_map(|(slot, k)| c.item(k).map(|i| (slot.as_str(), i))).collect();
        let two = c.weapon(&s.weapon).is_some_and(|w| w.two_handed);
        let shield = c.item(&s.shield).filter(|_| !two);
        worn_of(&armor, c.item(&s.weapon), shield, c.item(&s.bow))
    }

    /// The character screen's look of what's worn: each slot's display and inventory type.
    pub(crate) fn dress(&self, worn: &[(u8, u32)]) -> [(u32, u8); 19] {
        let mut out = [(0, 0); 19];
        for (slot, entry) in worn {
            if let (Some(o), Some((t, _))) = (out.get_mut(usize::from(*slot)), self.item_template(*entry)) {
                *o = (t.display_id, t.inventory_type as u8);
            }
        }
        out
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
        // Put away for a meal too, the hero's own toggle untouched (Corey's review: eating with
        // the axe still in hand).
        let sheathed = self.sim.players.get(&id).is_some_and(|p| p.sheathed || matches!(p.action, htl_survival::Action::Eat { .. }));
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
        } else if let Some(pose) = self.poses.get(&id) {
            f.set(idx::UNIT_NPC_EMOTESTATE, *pose);
        }
        self.gear_fields(id, &mut f);
        self.duel_fields(id, &mut f);
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

    /// Equipment slots are updated on the existing unit, without destroying/recreating it.
    pub(crate) fn mob_gear_fields(&self, id: Id, f: &mut Fields) {
        if let Some(g) = self.sim.arsenal_loadout(id) {
            arsenal_gear_fields(g, f);
        } else if let Some(m) = self.sim.mobs.get(&id) {
            mob_def_gear_fields(&self.sim.content.mobs[m.def], f);
        }
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
            faction: if m.friendly { fields::FACTION_FRIENDLY } else { fields::FACTION_MONSTER },
            health: m.health,
            max_health: m.max_health,
            attack_ms: 2000,
            damage: [d.attack.damage; 2],
            run_speed: d.run_speed,
            npc_flags: 0,
            // Call of the Wild's beasts can't be selected: no nameplate, no health bar, no name
            // over them (the client draws none for a `NOT_SELECTABLE` unit), nothing to click.
            unit_flags: if spirit_beast(&d.key) { fields::UNIT_FLAG_NOT_SELECTABLE } else { 0 },
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
        self.mob_gear_fields(id, &mut f);
        // A shield up between blows, for those who arrive meanwhile.
        if m.guarding && m.health > 0 {
            f.set(idx::UNIT_NPC_EMOTESTATE, super::play::POSE_SHIELD);
        }
        // A spirit beast is there the moment it's called and gone the moment it goes: the client's
        // two-second fade in had it still ghostly as it struck, and its fade out left it standing
        // after (Corey's review: "the fades are the issue"). Patch 0025's flag; older clients fade.
        if spirit_beast(&d.key) {
            f.set(idx::UNIT_DYNAMIC_FLAGS, fields::UNIT_DYNFLAG_NO_FADE);
        }
        if m.health == 0 {
            f.set(idx::UNIT_DYNAMIC_FLAGS, fields::UNIT_DYNFLAG_DEAD);
        }
        // What it wears from the start (a spirit beast's ghostly look) and whatever's on it, in
        // the packet that makes it: an aura sent apart could reach the client before the unit.
        let auras = self.sim.auras_of(id);
        if !auras.is_empty() {
            let mut bar = [(0u32, 0u8); 48];
            for (k, a) in auras.iter().take(16).enumerate() {
                bar[k] = (*a, 1);
            }
            for slot in 0..auras.len().min(16) {
                fields::aura_words(&mut f, &bar, slot);
            }
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

    /// A rope from `a` to `b`: the dig site's rope at its origin on `a`, turned toward `b`, drawn
    /// as long as the span (`GO_FLAG_STRETCH_X`) and strung an ankle's height over the ground,
    /// never solid (`GO_FLAG_PREVIEW_MOVE`).
    pub(crate) fn rope_create(&mut self, id: Id, a: [f32; 3], b: [f32; 3]) -> Packet {
        // The model's own rope sits above its origin: lowered so it's strung at ROPE_HEIGHT.
        let drop = ROPE_HEIGHT - ROPE_MODEL_MID_Z * ROPE_SCALE;
        let (a, b) = ([a[0], a[1], a[2] + drop], [b[0], b[1], b[2] + drop]);
        let entry = self.go_entry(ROPE_DISPLAY, ROPE_ENTRY_NAME, GO_TYPE_GENERIC);
        let guid = rope_guid(id);
        let span = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt();
        let stretch = (span / (ROPE_MODEL_LEN * ROPE_SCALE)).clamp(0.05, 50.0);
        let rot = rope_rotation(a, b);
        let facing = (b[1] - a[1]).atan2(b[0] - a[0]);
        let mut f = Fields::new();
        f.set_u64(idx::GUID, guid)
            .set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT)
            .set(idx::ENTRY, entry)
            .set_f32(idx::SCALE_X, ROPE_SCALE)
            .set(idx::GAMEOBJECT_DISPLAYID, ROPE_DISPLAY)
            .set(idx::GAMEOBJECT_FLAGS, GO_FLAGS_INERT | GO_FLAG_PREVIEW_MOVE | GO_FLAG_STRETCH_X)
            .set_f32(idx::GAMEOBJECT_ROTATION, rot[0])
            .set_f32(idx::GAMEOBJECT_ROTATION + 1, rot[1])
            .set_f32(idx::GAMEOBJECT_ROTATION + 2, rot[2])
            .set_f32(idx::GAMEOBJECT_ROTATION + 3, rot[3])
            .set(idx::GAMEOBJECT_STATE, 1)
            .set_f32(idx::GAMEOBJECT_POS_X, a[0])
            .set_f32(idx::GAMEOBJECT_POS_X + 1, a[1])
            .set_f32(idx::GAMEOBJECT_POS_X + 2, a[2])
            .set_f32(idx::GAMEOBJECT_FACING, facing)
            .set(idx::GAMEOBJECT_FACTION, 35)
            .set(idx::GAMEOBJECT_TYPE_ID, GO_TYPE_GENERIC)
            .set(idx::GAMEOBJECT_LEVEL, stretch.to_bits())
            .set(idx::GAMEOBJECT_ANIMPROGRESS, 100);
        packets::update_object(
            &[Create { guid, object_type: packets::TYPE_GAMEOBJECT, is_self: false, placement: Placement::At { pos: a, facing }, fields: &f }],
            &[],
        )
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
        Self::go_packet_tilted(guid, entry, display, kind, flags, scale, pos, facing, state, 0.0)
    }

    /// An object's create, tipped over by `tilt` (radians) about its model's own Y after its
    /// facing: an arrow stood head-down in the ground. The client places an object by its
    /// rotation quaternion `(x, y, z, w)`, `yaw × tilt`; at 0 it's the plain facing.
    #[allow(clippy::too_many_arguments)]
    fn go_packet_tilted(
        guid: u64,
        entry: u32,
        display: u32,
        kind: u32,
        flags: u32,
        scale: f32,
        pos: [f32; 3],
        facing: f32,
        state: u32,
        tilt: f32,
    ) -> Packet {
        let mut f = Fields::new();
        let (s, c) = (facing / 2.0).sin_cos();
        let (ts, tc) = (tilt / 2.0).sin_cos();
        f.set_u64(idx::GUID, guid)
            .set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT)
            .set(idx::ENTRY, entry)
            .set_f32(idx::SCALE_X, scale)
            .set(idx::GAMEOBJECT_DISPLAYID, display)
            .set(idx::GAMEOBJECT_FLAGS, flags)
            .set_f32(idx::GAMEOBJECT_ROTATION, -s * ts)
            .set_f32(idx::GAMEOBJECT_ROTATION + 1, c * ts)
            .set_f32(idx::GAMEOBJECT_ROTATION + 2, tc * s)
            .set_f32(idx::GAMEOBJECT_ROTATION + 3, c * tc)
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
        self.obj_create_for(id, 0)
    }

    /// The same, as a client of round `level` draws it: from `ARROW_LEVEL` a planted arrow is a
    /// real arrow stood in the ground (patch 0035's display), the feather for the rest. The entry
    /// and the guid are the feather's for everyone, so one destroy takes it from all of them; a
    /// client draws the display the create names, not the entry's.
    pub(crate) fn obj_create_for(&mut self, id: Id, level: u8) -> Vec<Packet> {
        let Some(v) = self.sim.object(id) else { return Vec::new() };
        if let Some(def) = self.resident(&v) {
            return vec![self.resident_create(def, &v)];
        }
        let (kind, flags) = self.obj_look_in(&v);
        let entry = self.go_entry(v.display, &v.name, kind);
        let guid = object_guid(entry, id);
        self.bind(id, guid);
        let state = self.obj_state(&v);
        let arrow = Self::planted_arrow(&v) && level >= ARROW_LEVEL;
        // The arrow glows white (patch 0039's flag) so it can be found in the grass; a client
        // without the patch ignores the bit.
        let (display, scale, tilt, flags) =
            if arrow { (ARROW_DISPLAY, ARROW_SCALE, ARROW_TILT, flags | GO_FLAG_GLOW) } else { (v.display, v.scale, 0.0, flags) };
        let mut out = vec![Self::go_packet_tilted(guid, entry, display, kind, flags, scale, v.pos, v.facing, state, tilt)];
        if let ObjKind::Ghost { def, ok, .. } = v.kind {
            let r = self.sim.content.stations[def].footprint;
            let (display, name, scale) =
                if ok { (GOOD_RING_DISPLAY, "Building Spot", (r / 1.5).max(0.4)) } else { (DANGER_DISPLAY, "Danger", (r / DANGER_MODEL_RADIUS).max(0.15)) };
            let e = self.go_entry(display, name, GO_TYPE_GENERIC);
            // The same guid whichever ring it is, so a patched client swaps it in place as the
            // ghost moves (green where it can go, red where it can't).
            let good = self.go_entry(GOOD_RING_DISPLAY, "Building Spot", GO_TYPE_GENERIC);
            let ring = object_guid(good, id) ^ (1 << 47);
            self.rings.insert(id, ring);
            out.push(Self::go_packet(ring, e, display, GO_TYPE_GENERIC, GO_FLAGS_INERT | GO_FLAG_PREVIEW_MOVE, scale, v.pos, 0.0));
        }
        out
    }

    /// One of the camp's people, standing at their post: friendly, not to be attacked.
    fn resident_create(&mut self, def: usize, v: &ObjView) -> Packet {
        let guid = creature_guid(RESIDENT_ENTRY + def as u32, v.id);
        self.bind(v.id, guid);
        let d = &self.sim.content.props[def];
        let look = UnitLook {
            guid,
            entry: RESIDENT_ENTRY + def as u32,
            display_id: d.display,
            scale: d.scale,
            level: 10,
            faction: fields::FACTION_FRIENDLY,
            health: 100,
            max_health: 100,
            attack_ms: 2000,
            damage: [1, 1],
            run_speed: 7.0,
            npc_flags: 0,
            unit_flags: UNIT_FLAG_NON_ATTACKABLE,
            main_hand: 0,
            main_hand_info: 0,
            off_hand: 0,
            ranged: 0,
        };
        let f = fields::unit_fields(&look);
        packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_UNIT,
                is_self: false,
                placement: Placement::Living { pos: v.pos, facing: v.facing, speeds: packets::speeds(7.0) },
                fields: &f,
            }],
            &[],
        )
    }

    /// Whose a station being placed is (its ghost): only they see it, so it never stands in
    /// anyone else's way (a model blocks the walk on the client that has it).
    pub(crate) fn ghost_owner(&self, id: Id) -> Option<Id> {
        match self.sim.objs.get(&id).map(|o| &o.kind) {
            Some(ObjKind::Ghost { player, .. }) => Some(*player),
            _ => None,
        }
    }

    /// A planted arrow's prop (the Hunter's: Multi-Shot's misses, Tripwire's two, Skyfall's
    /// twelve), drawn as an arrow for the clients that have the display.
    fn planted_arrow(v: &ObjView) -> bool {
        matches!(v.kind, ObjKind::Prop { display, .. } if display == htl_survival::sim::hunter::PLANT_LOOK)
    }

    /// Draw an object for everyone who should see it (a ghost: its placer alone), each client as
    /// its round draws it (`obj_create_for`).
    pub(crate) fn obj_show(&mut self, id: Id) {
        match self.ghost_owner(id) {
            Some(owner) => {
                if let Some(c) = self.conn_of(owner) {
                    let level = self.world_of(c).map_or(0, |w| w.level);
                    for p in self.obj_create_for(id, level) {
                        self.send(c, p);
                    }
                }
            }
            None => {
                let packets = self.obj_create(id);
                let arrow = self.sim.object(id).is_some_and(|v| Self::planted_arrow(&v));
                if !arrow {
                    for p in packets {
                        self.broadcast(&p);
                    }
                    return;
                }
                let arrows = self.obj_create_for(id, ARROW_LEVEL);
                let conns: Vec<(ConnId, bool)> = self.in_world().map(|(c, _, w)| (*c, w.level >= ARROW_LEVEL)).collect();
                for (c, newer) in conns {
                    for p in if newer { &arrows } else { &packets } {
                        self.send(c, p.clone());
                    }
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

    /// An unseen creature standing at `pos` to fly a missile from there (a boomerang turning
    /// back, an arrow called home): its guid, and the packet that makes it. A unit, as the client
    /// launches a missile only from one: a game object's cast has no caster unit, and the client
    /// draws nothing for it. Never selectable (no nameplate, no name, no tooltip), friendly to all.
    pub(crate) fn stand_in_create(&mut self, id: Id, pos: [f32; 3]) -> (u64, Packet) {
        let guid = creature_guid(STAND_IN_ENTRY, id);
        self.bind(id, guid);
        let look = UnitLook {
            guid,
            entry: STAND_IN_ENTRY,
            display_id: STAND_IN_DISPLAY,
            scale: STAND_IN_SCALE,
            level: 1,
            faction: fields::FACTION_FRIENDLY,
            health: 1,
            max_health: 1,
            attack_ms: 2000,
            damage: [1, 1],
            run_speed: 7.0,
            npc_flags: 0,
            unit_flags: UNIT_FLAG_NON_ATTACKABLE | fields::UNIT_FLAG_NOT_SELECTABLE,
            main_hand: 0,
            main_hand_info: 0,
            off_hand: 0,
            ranged: 0,
        };
        let f = fields::unit_fields(&look);
        let p = packets::update_object(
            &[Create { guid, object_type: packets::TYPE_UNIT, is_self: false, placement: Placement::Living { pos, facing: 0.0, speeds: packets::speeds(7.0) }, fields: &f }],
            &[],
        );
        (guid, p)
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
        let level = self.world_of(conn).map_or(0, |w| w.level);
        let objs: Vec<Id> = self.sim.objs.keys().copied().collect();
        for id in objs {
            // Someone else's ghost isn't theirs to see.
            if self.ghost_owner(id).is_some_and(|o| Some(o) != me) {
                continue;
            }
            for p in self.obj_create_for(id, level) {
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
    /// a foe's (`unblockable`: red), a duel's foe too.
    fn hero_tele_kind(&self, owner: htl_survival::Id, viewer: htl_survival::Id) -> &'static str {
        if owner == viewer || !self.sim.hostile(owner, viewer) {
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

fn arsenal_gear_fields(g: &htl_survival::content::ArsenalLoadout, f: &mut Fields) {
    let info = grip_info(&g.grip);
    f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY, g.weapon)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO, info)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 1, if info >> 24 == 17 { 1 } else { 3 })
        .set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1, g.off_hand)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 2, if g.off_hand == 0 { 0 } else { grip_info(&g.grip) })
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 3, if g.off_hand == 0 { 0 } else { 3 })
        .set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2, 0)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 4, 0)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 5, 0)
        .set(idx::UNIT_BYTES_2, 1);
}

/// Ordinary definition handoffs (an engineer landing with a wrench) use the same live update.
/// Explicit zeros matter: an off-hand weapon or bow from the old definition must disappear.
fn mob_def_gear_fields(d: &htl_survival::content::MobDef, f: &mut Fields) {
    let info = match grip_info(&d.grip) { 0 => 2 | 7 << 8 | 1 << 16 | 13 << 24, n => n };
    let (off, off_info, sheath) = if d.shield != 0 {
        (d.shield, 4 | 6 << 8 | 1 << 16 | 14 << 24, 4)
    } else if d.off_hand != 0 {
        (d.off_hand, 2 | 15 << 8 | 1 << 16 | 13 << 24, 3)
    } else { (0, 0, 0) };
    f.set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY, d.weapon)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO, if d.weapon == 0 { 0 } else { info })
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 1, if d.weapon == 0 { 0 } else if info >> 24 == 17 { 1 } else { 3 })
        .set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1, off)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 2, off_info)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 3, sheath)
        .set(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2, d.bow)
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 4, if d.bow == 0 { 0 } else { 2 | 2 << 8 | 2 << 16 | 15 << 24 })
        .set(idx::UNIT_VIRTUAL_ITEM_INFO + 5, 0)
        .set(idx::UNIT_BYTES_2, if d.bow != 0 { 2 } else { u32::from(d.weapon != 0) });
}

/// How a creature holds its weapon, as `UNIT_VIRTUAL_ITEM_INFO` packs it (class, subclass,
/// material, inventory type): a one-handed axe at the hip, a two-handed weapon across the back.
fn grip_info(grip: &str) -> u32 {
    let (sub, inv) = match grip {
        "2h" => (1, 17),
        "hammer" => (5, 17),
        "sword" => (7, 13),
        "staff" => (10, 17),
        "polearm" => (6, 17),
        "dagger" => (15, 13),
        "1h" => (0, 13),
        _ => return 0,
    };
    2 | sub << 8 | 1 << 16 | inv << 24
}

#[cfg(test)]
mod rope_tests {
    #[test]
    fn gilnid_platforms_keep_collision_while_visual_cues_appear_immediately() {
        use htl_survival::{content::Content, land::FlatLand};
        let mut game = super::SurvivalGame::new(Content::builtin(), Box::new(FlatLand::default()), None, 7);
        game.sim.mobs.clear();
        game.sim.objs.clear();
        let hero = game.sim.join("smith", "Smith", None);
        let p = game.sim.players.get_mut(&hero).unwrap();
        p.pos = [0.0, 0.0, 0.0];
        p.facing = 0.0;
        p.health = 100_000;
        p.max_health = 100_000;
        game.sim.spawn_target(hero, "dm_gilnid", 8.0, false, false, 1.0).unwrap();
        game.sim.step(0.05);
        assert_eq!(game.sim.objects().iter().filter(|o| matches!(o.display, 36 | 6481)).count(), 4);
        let mut seen = [false; 3];
        for _ in 0..300 {
            for obj in game.sim.objects() {
                let no_hull = game.obj_look_in(&obj).1 & super::GO_FLAG_PREVIEW_MOVE != 0;
                match obj.display {
                    36 | 6481 => assert!(!no_hull, "platform display {} must retain its collision hull", obj.display),
                    353 | 755 | 1108 => {
                        assert!(no_hull, "visual display {} must be immediate", obj.display);
                        seen[match obj.display { 353 => 0, 755 => 1, _ => 2 }] = true;
                    }
                    _ => {}
                }
            }
            game.sim.step(0.05);
        }
        assert_eq!(seen, [true; 3], "exercise steam, valve and dropped support debris");
    }

    #[test]
    fn smite_gear_updates_clear_the_offhand_and_match_a_late_observer() {
        use crate::fields::{idx, Fields};
        let c = htl_survival::content::Content::builtin();
        let gear = &c.mobs[c.mob("dm_smite").unwrap()].arsenal.as_ref().unwrap().loadouts;
        let mut live = Fields::new();
        for g in gear {
            super::arsenal_gear_fields(g, &mut live);
            let mut late = Fields::new();
            super::arsenal_gear_fields(g, &mut late);
            for field in [idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY, idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1,
                idx::UNIT_VIRTUAL_ITEM_INFO, idx::UNIT_VIRTUAL_ITEM_INFO + 1,
                idx::UNIT_VIRTUAL_ITEM_INFO + 2, idx::UNIT_VIRTUAL_ITEM_INFO + 3, idx::UNIT_BYTES_2] {
                assert_eq!(live.get(field), late.get(field));
            }
        }
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY), Some(2056));
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1), Some(0));
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_INFO + 2), Some(0));
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_INFO + 3), Some(0));
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_INFO + 1), Some(1), "hammer sheath across the back");
        assert_eq!(live.get(idx::UNIT_VIRTUAL_ITEM_INFO).unwrap() >> 8 & 255, 5, "two-handed mace subclass");
    }

    #[test]
    fn a_definition_handoff_replaces_the_weapon_and_clears_old_shield_and_bow() {
        use crate::fields::{idx, Fields};
        let c = htl_survival::content::Content::builtin();
        let mut air = c.mobs[c.mob("dm_perched_engineer").unwrap()].clone();
        air.weapon = 18575;
        air.shield = 1;
        air.bow = 2;
        let ground = &c.mobs[c.mob("dm_ground_engineer").unwrap()];
        let mut f = Fields::new();
        super::mob_def_gear_fields(&air, &mut f);
        assert_eq!(f.get(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY), Some(18575));
        super::mob_def_gear_fields(ground, &mut f);
        assert_eq!(f.get(idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY), Some(ground.weapon));
        for field in [idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1, idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2,
            idx::UNIT_VIRTUAL_ITEM_INFO + 2, idx::UNIT_VIRTUAL_ITEM_INFO + 3, idx::UNIT_VIRTUAL_ITEM_INFO + 4] {
            assert_eq!(f.get(field), Some(0));
        }
    }

    #[test]
    fn a_missile_look_line_says_each_field_and_a_caster_only_when_its_for_one_missile() {
        assert_eq!(super::missile_look_line(19434, "F2E6C8", 0.18, 0.3, 1.0, 0.0, None, false, 0.0), "ML\t19434\tF2E6C8\t0.18\t0.30\t1.00\t0.00");
        assert_eq!(super::missile_look_line(16075, "", 0.3, 0.35, 1.8, 2.5, Some(0xF130_0000_0000_002A), false, 0.0), "ML\t16075\t-\t0.30\t0.35\t1.80\t2.50\t0xF13000000000002A");
        // A tumble names its axis after the caster's place.
        assert_eq!(super::missile_look_line(16075, "", 0.3, 0.35, 1.8, 2.5, None, true, 0.0), "ML\t16075\t-\t0.30\t0.35\t1.80\t2.50\t-\tside");
        assert_eq!(super::missile_look_line(16075, "", 0.3, 0.35, 1.8, 2.5, Some(0x2A), true, 0.0), "ML\t16075\t-\t0.30\t0.35\t1.80\t2.50\t0x000000000000002A\tside");
        // A speed of its own comes tenth, after the caster's and the axis's places (Kill Shot).
        assert_eq!(super::missile_look_line(10346, "FF6A2A", 0.5, 0.5, 1.5, 0.0, None, false, 120.0), "ML\t10346\tFF6A2A\t0.50\t0.50\t1.50\t0.00\t-\t-\t120");
        assert_eq!(super::missile_look_line(10346, "FF6A2A", 0.5, 0.5, 1.5, 0.0, Some(0x2A), false, 120.0), "ML\t10346\tFF6A2A\t0.50\t0.50\t1.50\t0.00\t0x000000000000002A\t-\t120");
        assert_eq!(super::missile_look_line(16075, "", 0.3, 0.35, 1.8, 2.5, None, true, 90.0), "ML\t16075\t-\t0.30\t0.35\t1.80\t2.50\t-\tside\t90");
    }

    use super::rope_rotation;

    /// The model's X axis turned by a rotation (x y z w).
    fn turned_x(q: [f32; 4]) -> [f32; 3] {
        let [x, y, z, w] = q;
        [1.0 - 2.0 * (y * y + z * z), 2.0 * (x * y + w * z), 2.0 * (x * z - w * y)]
    }

    fn near(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|k| (a[k] - b[k]).abs() < 1.0e-4)
    }

    #[test]
    fn a_rope_points_from_its_first_end_to_its_second_on_the_level_and_up_a_slope() {
        // Level, along +Y: a quarter turn, no tilt.
        assert!(near(turned_x(rope_rotation([0.0, 0.0, 0.0], [0.0, 5.0, 0.0])), [0.0, 1.0, 0.0]));
        // Back along −X.
        assert!(near(turned_x(rope_rotation([0.0, 0.0, 0.0], [-4.0, 0.0, 0.0])), [-1.0, 0.0, 0.0]));
        // Up a slope: 3 along, 4 up.
        let s = (3.0f32 * 3.0 + 4.0 * 4.0).sqrt();
        assert!(near(turned_x(rope_rotation([1.0, 1.0, 0.0], [1.0, 4.0, 4.0])), [0.0, 3.0 / s, 4.0 / s]));
    }
}
