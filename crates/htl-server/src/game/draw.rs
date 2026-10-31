//! Drawing the world: creating units, players, items and ground effects for clients, and keeping
//! their fields up to date.

use htl_sim::{EntityId, EntityView, GroundMark, NpcKind, What};

use super::*;
use crate::fields::{PlayerLook, UnitLook};
use crate::packets::{Create, CreatureInfo, GroupMember, Placement};

impl Game {
    pub(crate) fn player_look(&self, entity: EntityId, guid: u64) -> Option<PlayerLook> {
        let v = self.sim.player_view(entity)?;
        let c = &self.sim.content().classes[v.class];
        let health = self.sim.entities().into_iter().find(|e| e.id == entity).map_or(1, |e| e.health);
        Some(PlayerLook {
            guid,
            race: c.race,
            class: c.class_id,
            gender: c.gender,
            power: c.power,
            max_power: c.max_power,
            appearance: c.appearance,
            display_id: c.display_id,
            faction: c.faction,
            level: v.level,
            health,
            max_health: v.stats.max_health,
            attack_ms: v.stats.attack_ms,
            damage: [v.stats.damage.0, v.stats.damage.1],
            ranged: c.ranged(),
            armor: v.stats.armor,
            attributes: v.stats.attributes,
            scale: v.stats.scale,
        })
    }

    /// A player's create block: everything for their own client, the public part for others.
    pub(crate) fn player_create(&self, entity: EntityId, guid: u64, is_self: bool) -> Packet {
        let look = self.player_look(entity, guid).expect("a player in the sim");
        let mut f = fields::player_fields(&look);
        let view = self.sim.player_view(entity).expect("a player in the sim");
        self.visible_fields(&mut f, &view);
        if is_self {
            f.set(idx::PLAYER_XP, view.xp)
                .set(idx::PLAYER_NEXT_LEVEL_XP, view.next_xp)
                .set(idx::PLAYER_COINAGE, view.gold.saturating_mul(10_000));
        }
        if let Some(auras) = self.auras.get(&entity) {
            for slot in 0..48 {
                if auras[slot].0 != 0 {
                    fields::aura_words(&mut f, auras, slot);
                }
            }
        }
        let (pos, facing) = self
            .sim
            .entities()
            .into_iter()
            .find(|e| e.id == entity)
            .map(|e| (e.pos, e.facing))
            .unwrap_or((self.sim.arena().player_spawn, self.sim.arena().player_facing));
        let speed = self.sim.player_speed(entity);
        packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_PLAYER,
                is_self,
                placement: Placement::Living { pos, facing, speeds: packets::speeds(speed) },
                fields: &f,
            }],
            &[],
        )
    }

    /// What others see a player wearing: each slot's item and its glow.
    pub(crate) fn visible_fields(&self, f: &mut Fields, view: &htl_sim::PlayerView) {
        for slot in 0..19u16 {
            let worn = view.equip.iter().find(|(s, ..)| *s as u16 == slot);
            let (entry, enchant) = worn.map_or((0, 0), |(_, _, e, en)| (*e, *en));
            f.set(idx::PLAYER_VISIBLE_ITEM_1_0 + 12 * slot, entry);
            f.set(idx::PLAYER_VISIBLE_ITEM_1_0 + 12 * slot + 1, enchant);
        }
    }

    /// The character sheet: damage, attack speed, armour, attributes.
    pub(crate) fn stat_fields(&self, entity: EntityId) -> Option<Fields> {
        let guid = *self.guids.get(&entity)?;
        let look = self.player_look(entity, guid)?;
        let mut f = Fields::new();
        fields::player_stats(&mut f, &look);
        Some(f)
    }

    pub(crate) fn unit_look(&self, view: &EntityView, guid: u64) -> Option<(UnitLook, f32)> {
        let content = self.sim.content();
        let base = |entry, display_id, scale, level, faction| UnitLook {
            guid,
            entry,
            display_id,
            scale,
            level,
            faction,
            health: view.health,
            max_health: view.max_health,
            attack_ms: 2000,
            damage: [0, 0],
            run_speed: 0.0,
            npc_flags: 0,
            unit_flags: 0,
            main_hand: 0,
            main_hand_info: 0,
            off_hand: 0,
            ranged: 0,
        };
        Some(match &view.what {
            What::Hub => {
                let mut u = base(HUB_ENTRY, content.hub.display_id, 1.0, content.hub.level, fields::FACTION_FRIENDLY);
                u.npc_flags = fields::NPC_FLAG_GOSSIP;
                (u, 0.0)
            }
            What::Enemy { kind, elite } => {
                let d = self.sim.enemy_def(*kind);
                let scale = if *elite { d.scale * content.scaling.elite_scale } else { d.scale };
                let twist = self.sim.enemy_modifier(view.id);
                let mut u = base(entry_of(*kind, *elite, twist), d.display_id, scale, view.level, fields::FACTION_MONSTER);
                u.attack_ms = (d.attack_secs * 1000.0) as u32;
                u.damage = d.damage;
                u.run_speed = d.speed;
                (u, d.speed)
            }
            What::Npc { npc, .. } => {
                let b = &content.bases;
                let (display, scale) = match npc {
                    NpcKind::Gate => (b.gate_display, 1.0),
                    NpcKind::Cache => (b.cache_display, 1.0),
                    NpcKind::Banner { claimed: false } => (b.banner_display, 1.5),
                    NpcKind::Banner { claimed: true } => (b.claimed_display, 1.5),
                    NpcKind::Sprint => (content.sprint.display_id, content.sprint.scale),
                    NpcKind::Protector => (content.defenses.protector_display, content.defenses.protector_scale),
                    NpcKind::Spring => (content.defenses.spring_display, content.defenses.spring_scale),
                    NpcKind::Post(_) => (b.claimed_display, 1.2),
                    NpcKind::Vendor => (content.hub.vendor.display_id, 1.0),
                };
                let level = if matches!(npc, NpcKind::Protector | NpcKind::Spring) { view.level } else { 1 };
                let mut u = base(npc_entry(*npc), display, scale, level, fields::FACTION_FRIENDLY);
                if *npc == NpcKind::Vendor {
                    u.main_hand = content.hub.vendor.weapon;
                    u.level = 30;
                }
                match npc {
                    NpcKind::Gate | NpcKind::Sprint => u.unit_flags = fields::UNIT_FLAG_NOT_SELECTABLE,
                    NpcKind::Protector | NpcKind::Spring => {}
                    _ => u.npc_flags = fields::NPC_FLAG_GOSSIP,
                }
                (u, 0.0)
            }
            What::Ally { pet: true, .. } => {
                let mut u = base(PET_ENTRY, content.pet.display, content.pet.scale, view.level, fields::FACTION_FRIENDLY);
                u.run_speed = content.pet.speed;
                (u, content.pet.speed)
            }
            What::Ally { owner, .. } if *owner == self.sim.hub_id() => {
                let d = &content.defenses;
                let mut u = match self.sim.soldier_kind(view.id) {
                    Some(htl_sim::SoldierKind::Rifleman) => {
                        let mut u = base(RIFLEMAN_ENTRY, d.rifleman_display, 1.0, view.level, fields::FACTION_FRIENDLY);
                        u.ranged = d.rifleman_weapon;
                        u
                    }
                    Some(htl_sim::SoldierKind::Cleric) => {
                        let mut u = base(CLERIC_ENTRY, d.cleric_display, 1.0, view.level, fields::FACTION_FRIENDLY);
                        u.main_hand = d.cleric_weapon;
                        u
                    }
                    Some(htl_sim::SoldierKind::Defender(r)) if r > 0 => {
                        let rank = d.ranks.get(r as usize).or(d.ranks.last());
                        let display = rank.map_or(d.defender_display, |x| x.display);
                        let mut u = base(DEFENDER_ENTRY + RANK_ENTRY_STEP * r as u32, display, 1.0, view.level, fields::FACTION_FRIENDLY);
                        u.main_hand = rank.map_or(d.defender_weapon, |x| x.weapon);
                        u.off_hand = rank.map_or(0, |x| x.shield);
                        u
                    }
                    _ => {
                        let mut u = base(DEFENDER_ENTRY, d.defender_display, 1.0, view.level, fields::FACTION_FRIENDLY);
                        u.main_hand = d.defender_weapon;
                        u
                    }
                };
                u.run_speed = 7.0;
                (u, 7.0)
            }
            What::Ally { .. } => {
                let mut u = base(ALLY_ENTRY, 158, 1.0, view.level, fields::FACTION_FRIENDLY);
                u.run_speed = 7.0;
                (u, 7.0)
            }
            What::Player { .. } | What::Zone { .. } => return None,
        })
    }

    /// A unit's create packet.
    pub(crate) fn unit_create(&self, view: &EntityView, guid: u64) -> Option<Packet> {
        let (look, speed) = self.unit_look(view, guid)?;
        let mut f = fields::unit_fields(&look);
        if let Some(auras) = self.auras.get(&view.id) {
            for slot in 0..48 {
                if auras[slot].0 != 0 {
                    fields::aura_words(&mut f, auras, slot);
                }
            }
        }
        Some(packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_UNIT,
                is_self: false,
                placement: Placement::Living { pos: view.pos, facing: view.facing, speeds: packets::speeds(speed.max(1.0)) },
                fields: &f,
            }],
            &[],
        ))
    }

    /// A ground effect's create packet.
    pub(crate) fn zone_create(&self, id: EntityId, caster: EntityId, spell: u32, pos: [f32; 3], radius: f32) -> Packet {
        let guid = zone_guid(id);
        let caster = self.guids.get(&caster).copied().unwrap_or(0);
        let mut f = Fields::new();
        f.set_u64(idx::GUID, guid)
            .set(idx::TYPE, fields::TYPEMASK_DYNAMICOBJECT)
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
        packets::update_object(
            &[Create {
                guid,
                object_type: packets::TYPE_DYNAMICOBJECT,
                is_self: false,
                placement: Placement::At { pos, facing: 0.0 },
                fields: &f,
            }],
            &[],
        )
    }

    /// A ground mark's game object: a danger circle drawn to the zone's size, or a trap.
    pub(crate) fn mark_create(&self, id: EntityId, mark: GroundMark, pos: [f32; 3], radius: f32) -> Option<Packet> {
        mark_packet(id, mark, pos, radius)
    }

    pub(crate) fn creature_query(&self, conn: ConnId, entry: u32) {
        let content = self.sim.content();
        let info = |name: &str, subname: &str, creature_type: u32, rank: u32, display_id: u32| {
            packets::creature_query_response(&CreatureInfo { entry, name, subname, creature_type, rank, display_id })
        };
        let b = &content.bases;
        let p = match entry {
            HUB_ENTRY => info(&content.hub.name, &content.hub.subname, 10, 1, content.hub.display_id),
            GATE_ENTRY => info("Blight Gate", "", 10, 0, b.gate_display),
            CACHE_ENTRY => info("Relic Cache", "Right-click for a relic", 10, 0, b.cache_display),
            BANNER_ENTRY => info("Scourge Banner", "Right-click to claim", 10, 0, b.banner_display),
            BANNER_CLAIMED_ENTRY => info("Alliance Banner", "Claimed", 10, 0, b.claimed_display),
            ALLY_ENTRY => info("Risen Servant", "", 6, 0, 158),
            SPRINT_ENTRY => info("Swiftwind Totem", "Walk through for speed", 11, 0, content.sprint.display_id),
            PROTECTOR_ENTRY => info("Treant Protector", "Defender of the Ancient", 4, 1, content.defenses.protector_display),
            SPRING_ENTRY => info("Healing Spring", "Stand near it to heal", 11, 0, content.defenses.spring_display),
            DEFENDER_ENTRY => info("Basin Defender", "Defender of the Ancient", 7, 0, content.defenses.defender_display),
            VENDOR_ENTRY => info(&content.hub.vendor.name, &content.hub.vendor.subname, 7, 0, content.hub.vendor.display_id),
            RIFLEMAN_ENTRY => info("Basin Rifleman", "Defender of the Ancient", 7, 0, content.defenses.rifleman_display),
            CLERIC_ENTRY => info("Basin Cleric", "Defender of the Ancient", 7, 0, content.defenses.cleric_display),
            e if e > DEFENDER_ENTRY && (e - DEFENDER_ENTRY) % RANK_ENTRY_STEP == 0 && (e - DEFENDER_ENTRY) / RANK_ENTRY_STEP < 8 => {
                let r = ((e - DEFENDER_ENTRY) / RANK_ENTRY_STEP) as usize;
                let rank = content.defenses.ranks.get(r).or(content.defenses.ranks.last());
                info(rank.map_or("Basin Defender", |x| x.name.as_str()), "Defender of the Ancient", 7, 0, rank.map_or(content.defenses.defender_display, |x| x.display))
            }
            PET_ENTRY => info(&content.pet.name, "Hunter's Pet", 1, 0, content.pet.display),
            e if (POST_ENTRY..POST_ENTRY + 8).contains(&e) => {
                let k = (e - POST_ENTRY) as usize;
                match self.sim.arena().posts.get(k) {
                    Some(post) => {
                        // "the north bridge" -> "North Bridge"
                        let name: String = post
                            .name
                            .trim_start_matches("the ")
                            .split(' ')
                            .map(|w| {
                                let mut c = w.chars();
                                c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
                            })
                            .collect::<Vec<_>>()
                            .join(" ");
                        info(&name, "Right-click to build defences", 10, 0, b.claimed_display)
                    }
                    None => packets::creature_query_miss(entry),
                }
            }
            _ => match kind_of_entry(entry, content.enemies.len(), content.bosses.len()) {
                Some((kind, elite)) => {
                    let d = self.sim.enemy_def(kind);
                    let rank = if elite { d.rank.max(1) } else { d.rank };
                    // An elite with a twist wears it in its name.
                    let twist = if (TWIST_ENTRY..TWIST_ENTRY + 8 * content.enemies.len() as u32).contains(&entry) {
                        ((entry - TWIST_ENTRY) % 8) as usize
                    } else {
                        0
                    };
                    match htl_sim::MODIFIERS.get(twist.wrapping_sub(1)) {
                        Some((prefix, line)) => info(&format!("{prefix} {}", d.name), line, d.creature_type, rank, d.display_id),
                        None => info(&d.name, &d.title, d.creature_type, rank, d.display_id),
                    }
                }
                None => packets::creature_query_miss(entry),
            },
        };
        self.send(conn, p);
    }

    /// Draw the whole world for a player who just arrived.
    pub(crate) fn send_world_to(&mut self, conn: ConnId, me: EntityId) {
        let views = self.sim.entities();
        for v in &views {
            if v.id == me {
                continue;
            }
            match &v.what {
                What::Player { .. } => {
                    if let Some(&guid) = self.guids.get(&v.id) {
                        let p = self.player_create(v.id, guid, false);
                        self.send(conn, p);
                    }
                }
                What::Zone { caster, spell, radius, mark } => {
                    let p = self.zone_create(v.id, *caster, *spell, v.pos, *radius);
                    self.send(conn, p);
                    if let Some(p) = self.mark_create(v.id, *mark, v.pos, *radius) {
                        self.send(conn, p);
                    }
                }
                _ => {
                    let Some(&guid) = self.guids.get(&v.id) else { continue };
                    if let Some(p) = self.unit_create(v, guid) {
                        self.send(conn, p);
                    }
                }
            }
        }
        // Anyone walking keeps walking; anyone fighting keeps swinging.
        for v in &views {
            let Some(&guid) = self.guids.get(&v.id) else { continue };
            if let Some((path, speed)) = &v.moving {
                if let Some(p) = self.move_packet(guid, path, *speed) {
                    self.send(conn, p);
                }
            }
            if let Some(victim) = v.attacking.and_then(|a| self.guids.get(&a)) {
                self.send(conn, packets::attack_start(guid, *victim));
            }
        }
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

    pub(crate) fn send_groups(&self) {
        let members: Vec<(ConnId, u64, String)> = self.in_world().map(|(c, _, w)| (*c, w.guid, w.name.clone())).collect();
        let leader = members.iter().map(|m| m.1).min().unwrap_or(0);
        for (conn, guid, _) in &members {
            let others: Vec<GroupMember> = members
                .iter()
                .filter(|m| m.1 != *guid)
                .take(4)
                .map(|m| GroupMember { name: &m.2, guid: m.1 })
                .collect();
            let p = if others.is_empty() { packets::group_none() } else { packets::group_list(&others, leader) };
            self.send(*conn, p);
        }
    }

    // --- items ----------------------------------------------------------------------------------

    /// Bring a player's client up to date with their items: new objects, gone objects, and the
    /// slots that hold them.
    pub(crate) fn sync_items(&mut self, conn: ConnId) {
        let Some(w) = self.world_of(conn) else { return };
        let (entity, owner) = (w.entity, w.guid);
        let Some(view) = self.sim.player_view(entity) else { return };
        let mut now: BTreeMap<u32, (u32, u32)> = BTreeMap::new();
        for (_, id, entry, enchant) in view.equip.iter().chain(view.pack.iter()) {
            now.insert(*id, (*entry, *enchant));
        }
        let before = w.items.clone();
        // Fresh templates first, for this player and everyone who sees them.
        let entries: Vec<u32> = now.values().map(|(e, _)| *e).collect();
        self.push_item_templates(entries);
        // New items, and items whose enchant changed.
        let mut creates = Vec::new();
        let mut changed = Vec::new();
        for (id, (entry, enchant)) in &now {
            match before.get(id) {
                None => {
                    let mut f = Fields::new();
                    f.set_u64(idx::GUID, item_guid(*id))
                        .set(idx::TYPE, fields::TYPEMASK_ITEM)
                        .set(idx::ENTRY, *entry)
                        .set_f32(idx::SCALE_X, 1.0)
                        .set_u64(idx::ITEM_OWNER, owner)
                        .set_u64(idx::ITEM_CONTAINED, owner)
                        .set(idx::ITEM_STACK_COUNT, 1)
                        .set(idx::ITEM_FLAGS, 1) // soulbound
                        .set(idx::ITEM_ENCHANTMENT, *enchant);
                    creates.push((item_guid(*id), f));
                }
                Some((_, old)) if old != enchant => {
                    let mut f = Fields::new();
                    f.set(idx::ITEM_ENCHANTMENT, *enchant);
                    changed.push((item_guid(*id), f));
                }
                _ => {}
            }
        }
        for chunk in creates.chunks(16) {
            let c: Vec<Create> = chunk
                .iter()
                .map(|(g, f)| Create { guid: *g, object_type: packets::TYPE_ITEM, is_self: false, placement: Placement::Nowhere, fields: f })
                .collect();
            self.send(conn, packets::update_object(&c, &[]));
        }
        if !changed.is_empty() {
            let v: Vec<(u64, &Fields)> = changed.iter().map(|(g, f)| (*g, f)).collect();
            self.send(conn, packets::update_object(&[], &v));
        }
        // The player's own slots.
        let mut f = Fields::new();
        for slot in 0..19u16 {
            let id = view.equip.iter().find(|(s, ..)| *s as u16 == slot).map_or(0, |(_, id, ..)| item_guid(*id));
            f.set_u64(idx::PLAYER_INV_SLOT_HEAD + 2 * slot, id);
        }
        for i in 0..16u16 {
            let id = view
                .pack
                .iter()
                .find(|(s, ..)| *s as u16 == htl_sim::items::slot::PACK_FIRST as u16 + i)
                .map_or(0, |(_, id, ..)| item_guid(*id));
            f.set_u64(idx::PLAYER_PACK_SLOT_1 + 2 * i, id);
        }
        self.send(conn, packets::update_object(&[], &[(owner, &f)]));
        for id in before.keys().filter(|id| !now.contains_key(id)) {
            self.send(conn, packets::destroy_object(item_guid(*id)));
        }
        if let Some(w) = self.world_mut(conn) {
            w.items = now;
        }
    }

    // --- loot ------------------------------------------------------------------------------------

    pub(crate) fn loot_open(&mut self, conn: ConnId, corpse: u64) {
        let Some(me) = self.world_of(conn).map(|w| w.entity) else { return };
        let Some(&cid) = self.entities.get(&corpse) else {
            self.send(conn, packets::loot_error(corpse));
            return;
        };
        match self.sim.loot_open(me, cid) {
            Some(view) => {
                let items: Vec<(u8, u32, u32)> = view
                    .items
                    .iter()
                    .filter(|(_, e)| *e != 0)
                    .map(|(s, e)| (*s, *e, self.sim.item_template(*e).map_or(0, |t| t.display_id)))
                    .collect();
                self.send(conn, packets::loot_response(corpse, view.gold.saturating_mul(10_000), &items));
                if let Some(w) = self.world_mut(conn) {
                    w.looting = Some(corpse);
                }
            }
            None => self.send(conn, packets::loot_error(corpse)),
        }
    }

    pub(crate) fn loot_take(&mut self, conn: ConnId, slot: u8) {
        let Some((me, corpse)) = self.world_of(conn).and_then(|w| w.looting.map(|l| (w.entity, l))) else { return };
        let Some(&cid) = self.entities.get(&corpse) else { return };
        for gone in self.sim.loot_take(me, cid, slot) {
            self.send(conn, packets::loot_removed(gone));
        }
    }

    pub(crate) fn loot_money(&mut self, conn: ConnId) {
        let Some((me, corpse)) = self.world_of(conn).and_then(|w| w.looting.map(|l| (w.entity, l))) else { return };
        let Some(&cid) = self.entities.get(&corpse) else { return };
        let gold = self.sim.loot_gold(me, cid);
        if gold > 0 {
            self.send(conn, packets::loot_money_notify(gold.saturating_mul(10_000)));
            self.send(conn, packets::loot_clear_money());
        }
    }
}

// --- ground marks --------------------------------------------------------------------------------

/// Ground marks are generic game objects (type 5): drawn, never used.
pub(crate) const MARK_TYPE: u32 = 5;
/// A mark's template entry: this plus its display.
const MARK_ENTRY: u32 = 190_000;
/// The Dire Maul ritual circle: a flat glowing ring on the ground, 6.5 yd in radius as drawn.
const DANGER_DISPLAY: u32 = 5812;
const DANGER_MODEL_RADIUS: f32 = 6.48;
/// The hunters' trap models (World\Goober), about a yard across as drawn.
const FREEZING_TRAP_DISPLAY: u32 = 3071;
const EXPLOSIVE_TRAP_DISPLAY: u32 = 3072;
const FROST_TRAP_DISPLAY: u32 = 3073;
const TRAP_SCALE: f32 = 1.6;

/// A mark's display and the size it's drawn at.
fn mark_look(mark: GroundMark, radius: f32) -> Option<(u32, f32)> {
    match mark {
        GroundMark::None => None,
        GroundMark::Danger => Some((DANGER_DISPLAY, (radius / DANGER_MODEL_RADIUS).max(0.15))),
        GroundMark::FreezingTrap => Some((FREEZING_TRAP_DISPLAY, TRAP_SCALE)),
        GroundMark::ExplosiveTrap => Some((EXPLOSIVE_TRAP_DISPLAY, TRAP_SCALE)),
        GroundMark::FrostTrap => Some((FROST_TRAP_DISPLAY, TRAP_SCALE)),
    }
}

/// A mark's template, by its entry: its display and name.
pub(crate) fn mark_template(entry: u32) -> Option<(u32, &'static str)> {
    let display = entry.checked_sub(MARK_ENTRY)?;
    let name = match display {
        DANGER_DISPLAY => "Danger",
        FREEZING_TRAP_DISPLAY => "Freezing Trap",
        EXPLOSIVE_TRAP_DISPLAY => "Explosive Trap",
        FROST_TRAP_DISPLAY => "Frost Trap",
        _ => return None,
    };
    Some((display, name))
}

/// A ground mark's game object: a danger circle drawn to the zone's size, or a trap.
fn mark_packet(id: EntityId, mark: GroundMark, pos: [f32; 3], radius: f32) -> Option<Packet> {
    let (display, scale) = mark_look(mark, radius)?;
    let guid = mark_guid(id);
    let mut f = Fields::new();
    f.set_u64(idx::GUID, guid)
        .set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT)
        .set(idx::ENTRY, MARK_ENTRY + display)
        .set_f32(idx::SCALE_X, scale)
        .set(idx::GAMEOBJECT_DISPLAYID, display)
        // Not selectable, and not despawned by the client.
        .set(idx::GAMEOBJECT_FLAGS, 0x10 | 0x20)
        .set_f32(idx::GAMEOBJECT_ROTATION + 3, 1.0)
        .set(idx::GAMEOBJECT_STATE, 1)
        .set_f32(idx::GAMEOBJECT_POS_X, pos[0])
        .set_f32(idx::GAMEOBJECT_POS_X + 1, pos[1])
        .set_f32(idx::GAMEOBJECT_POS_X + 2, pos[2])
        .set_f32(idx::GAMEOBJECT_FACING, 0.0)
        .set(idx::GAMEOBJECT_FACTION, 35)
        .set(idx::GAMEOBJECT_TYPE_ID, MARK_TYPE)
        .set(idx::GAMEOBJECT_ANIMPROGRESS, 100);
    Some(packets::update_object(
        &[Create { guid, object_type: packets::TYPE_GAMEOBJECT, is_self: false, placement: Placement::At { pos, facing: 0.0 }, fields: &f }],
        &[],
    ))
}

#[cfg(test)]
mod mark_tests {
    use super::*;
    use benilla_protocol::messages::{parse_server_with_tail, ServerPacket};

    #[test]
    fn the_client_reads_a_ground_mark_and_its_template() {
        let p = mark_packet(7, GroundMark::Danger, [1.0, 2.0, 3.0], 13.0).expect("a mark");
        let (packet, tail) = parse_server_with_tail(p.opcode, &p.body).expect("benilla parses the mark");
        assert_eq!(tail, 0);
        assert!(matches!(packet, ServerPacket::UpdateObject { .. }), "{}", packet.name());
        let ev = format!("{:?}", benilla_protocol::decode(packet));
        assert!(ev.contains("5812") && ev.contains("GameObject"), "{ev}");
        assert!(mark_packet(7, GroundMark::None, [0.0; 3], 3.0).is_none());
        let (display, name) = mark_template(MARK_ENTRY + DANGER_DISPLAY).unwrap();
        let q = packets::gameobject_query_response(MARK_ENTRY + display, MARK_TYPE, display, name);
        let (packet, tail) = parse_server_with_tail(q.opcode, &q.body).expect("benilla parses the template");
        assert_eq!(tail, 0, "{}", packet.name());
    }
}
