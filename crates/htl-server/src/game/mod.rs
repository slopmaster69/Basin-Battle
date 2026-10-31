//! The game thread: player sessions on one side, the [`Sim`] on the other. It answers the
//! client's requests, feeds the sim what players do, and plays the sim's events out as packets.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use benilla_protocol::messages::opcode as op;
use crossbeam_channel::{Receiver, RecvTimeoutError, Sender};
use htl_sim::{EnemyKind, EntityId, EquipError, NpcKind, Sim};

use crate::fields::{self, idx, Fields};
use crate::packets::{self, Packet};
use crate::wire::{Body, Reader};
use crate::world::{ConnId, Inbound};

mod draw;
mod play;
mod saves;
mod ui;

pub use saves::Saves;

const TICK: Duration = Duration::from_millis(50);

/// Creature entries: the client caches each entry's name and model by number, so these never
/// change meaning.
pub(crate) const HUB_ENTRY: u32 = 1000;
pub(crate) const ENEMY_ENTRY: u32 = 2000;
pub(crate) const ELITE_ENTRY: u32 = 2500;
pub(crate) const BOSS_ENTRY: u32 = 3000;
pub(crate) const GATE_ENTRY: u32 = 4000;
pub(crate) const CACHE_ENTRY: u32 = 4001;
pub(crate) const BANNER_ENTRY: u32 = 4002;
pub(crate) const BANNER_CLAIMED_ENTRY: u32 = 4003;
pub(crate) const ALLY_ENTRY: u32 = 4010;
pub(crate) const SPRINT_ENTRY: u32 = 4020;
pub(crate) const PROTECTOR_ENTRY: u32 = 4021;
pub(crate) const SPRING_ENTRY: u32 = 4022;
pub(crate) const DEFENDER_ENTRY: u32 = 4023;
pub(crate) const PET_ENTRY: u32 = 4024;
pub(crate) const VENDOR_ENTRY: u32 = 4025;
pub(crate) const RIFLEMAN_ENTRY: u32 = 4026;
pub(crate) const CLERIC_ENTRY: u32 = 4027;
/// A promoted defender's entry: DEFENDER_ENTRY + this x its rank (4123, 4223, 4323).
pub(crate) const RANK_ENTRY_STEP: u32 = 100;
/// The posts' standards: one entry each, so each shows its own name.
pub(crate) const POST_ENTRY: u32 = 4030;
/// An elite with a twist: 5000 + 8 * kind + twist.
pub(crate) const TWIST_ENTRY: u32 = 5000;

/// High guids: a creature, a ground effect, an item.
const HIGH_UNIT: u64 = 0xF130;
const HIGH_DYNAMIC: u64 = 0xF100;
const HIGH_GAMEOBJECT: u64 = 0xF110;
const HIGH_ITEM: u64 = 0x4000;

/// Client movement opcodes the server relays to everyone else.
const MOVE_OPCODES: &[u16] = &[
    op::MSG_MOVE_START_FORWARD,
    op::MSG_MOVE_START_BACKWARD,
    op::MSG_MOVE_STOP,
    op::MSG_MOVE_START_STRAFE_LEFT,
    op::MSG_MOVE_START_STRAFE_RIGHT,
    op::MSG_MOVE_STOP_STRAFE,
    op::MSG_MOVE_JUMP,
    op::MSG_MOVE_START_TURN_LEFT,
    op::MSG_MOVE_START_TURN_RIGHT,
    op::MSG_MOVE_STOP_TURN,
    op::MSG_MOVE_START_PITCH_UP,
    op::MSG_MOVE_START_PITCH_DOWN,
    op::MSG_MOVE_STOP_PITCH,
    op::MSG_MOVE_SET_RUN_MODE,
    op::MSG_MOVE_SET_WALK_MODE,
    op::MSG_MOVE_FALL_LAND,
    op::MSG_MOVE_START_SWIM,
    op::MSG_MOVE_STOP_SWIM,
    op::MSG_MOVE_SET_FACING,
    op::MSG_MOVE_SET_PITCH,
    op::MSG_MOVE_HEARTBEAT,
];

const CHAR_CREATE_DISABLED: u8 = 0x32;
const CHAR_DELETE_FAILED: u8 = 0x3A;

/// The backpack, as the client numbers bags.
const BAG_BACKPACK: u8 = 255;

pub(crate) struct InWorld {
    pub guid: u64,
    pub entity: EntityId,
    pub class: usize,
    pub name: String,
    /// The item objects this player's client knows: item id to (entry, enchant).
    pub items: BTreeMap<u32, (u32, u32)>,
    /// The corpse whose loot window is open.
    pub looting: Option<u64>,
    /// A Renown menu the server itself has open (not the sim's).
    pub renown_menu: Option<Vec<saves::RenownChoice>>,
    /// Teleports and speed changes are numbered.
    pub counter: u32,
    pub speed: f32,
    /// The height a fall started from: the last ground the client reported, raised by anything
    /// higher it reported while in the air.
    pub fall_top: Option<f32>,
    pub ground_z: f32,
    /// Held in place while the world is paused for a card pick.
    pub rooted: bool,
    /// The damage meters as last sent (own, party) and when.
    pub meters: (String, String, Option<Instant>),
    /// The client runs the Basin Battle add-on (it said `.bb hello`).
    pub addon: bool,
    /// Which version of the add-on.
    pub addon_version: u32,
    /// The objective tracker as last sent.
    pub guide: String,
    /// The shop as last sent.
    pub shop: String,
    /// Whether they were at home, by the shopkeeper, when last told.
    pub at_shop: Option<bool>,
    /// The fronts tracker as last sent.
    pub fronts: String,
    /// The world map is open: send it the battlefield.
    pub map_open: bool,
    /// Item templates sent fresh this session ([`Game::push_item_templates`]).
    pub fresh_items: BTreeSet<u32>,
    /// The boss bar as last sent.
    pub boss: String,
}

pub(crate) struct Session {
    pub account: String,
    pub out: Sender<Packet>,
    pub world: Option<InWorld>,
}

impl Session {
    fn send(&self, p: Packet) {
        let _ = self.out.send(p);
    }
}

pub struct Game {
    sim: Sim,
    sessions: HashMap<ConnId, Session>,
    accounts: HashMap<String, u32>,
    /// Sim entity to wire guid, for everything in the world.
    guids: HashMap<EntityId, u64>,
    /// Wire guid to sim entity.
    entities: HashMap<u64, EntityId>,
    /// Each unit's buff bar, as last sent.
    auras: HashMap<EntityId, [(u32, u8); 48]>,
    /// Zones marked on the ground by a game object.
    marks: BTreeSet<EntityId>,
    spline_id: u32,
    /// Hearthstones cast this session (which line the hero says).
    pub(crate) hearths: u32,
    started: Instant,
    saves: Saves,
    /// Spell id to icon name, for the add-on's card picker.
    icons: HashMap<u32, String>,
    hud: ui::Hud,
    offer_seq: u32,
    guide_at: Option<std::time::Instant>,
    /// Recent calls for help, shown on the world map for a while: when, and where.
    pub(crate) pings: Vec<(std::time::Instant, [f32; 3])>,
}

pub fn display_name(account: &str) -> String {
    let lower = account.to_lowercase();
    let mut c = lower.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

impl Game {
    pub fn new(sim: Sim, saves: Option<PathBuf>, icons: HashMap<u32, String>) -> Self {
        let mut g = Self {
            sim,
            sessions: HashMap::new(),
            accounts: HashMap::new(),
            guids: HashMap::new(),
            entities: HashMap::new(),
            auras: HashMap::new(),
            marks: BTreeSet::new(),
            spline_id: 1,
            hearths: 0,
            started: Instant::now(),
            saves: Saves::load(saves),
            icons,
            hud: ui::Hud::default(),
            offer_seq: 0,
            guide_at: None,
            pings: Vec::new(),
        };
        let hub = g.sim.hub_id();
        g.bind(hub, creature_guid(HUB_ENTRY, hub));
        g
    }

    /// Run until the inbound channel closes.
    pub fn run(mut self, inbound: Receiver<Inbound>) {
        let mut next = Instant::now() + TICK;
        loop {
            let now = Instant::now();
            if now >= next {
                let dt = TICK.as_secs_f32();
                let events = self.sim.step(dt);
                self.play(events);
                self.ui_flush_hud(false);
                self.ui_guides();
                self.ui_meters(false);
                next += TICK;
                if next < now {
                    next = now + TICK; // fell behind: don't try to catch up
                }
                continue;
            }
            match inbound.recv_timeout(next - now) {
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }
    }

    pub(crate) fn bind(&mut self, entity: EntityId, guid: u64) {
        self.guids.insert(entity, guid);
        self.entities.insert(guid, entity);
    }

    pub(crate) fn unbind(&mut self, entity: EntityId) {
        if let Some(guid) = self.guids.remove(&entity) {
            self.entities.remove(&guid);
        }
        self.auras.remove(&entity);
    }

    fn account_id(&mut self, account: &str) -> u32 {
        let next = self.accounts.len() as u32 + 1;
        *self.accounts.entry(account.to_string()).or_insert(next)
    }

    fn player_guid(account_id: u32, class: usize) -> u64 {
        u64::from(account_id) * 16 + class as u64 + 1
    }

    pub(crate) fn in_world(&self) -> impl Iterator<Item = (&ConnId, &Session, &InWorld)> {
        self.sessions.iter().filter_map(|(c, s)| s.world.as_ref().map(|w| (c, s, w)))
    }

    pub(crate) fn conn_of(&self, entity: EntityId) -> Option<ConnId> {
        self.in_world().find(|(_, _, w)| w.entity == entity).map(|(c, _, _)| *c)
    }

    pub(crate) fn world_mut(&mut self, conn: ConnId) -> Option<&mut InWorld> {
        self.sessions.get_mut(&conn).and_then(|s| s.world.as_mut())
    }

    pub(crate) fn broadcast(&self, p: &Packet) {
        for (_, s, _) in self.in_world() {
            s.send(p.clone());
        }
    }

    pub(crate) fn broadcast_except(&self, except: ConnId, p: &Packet) {
        for (c, s, _) in self.in_world() {
            if *c != except {
                s.send(p.clone());
            }
        }
    }

    pub(crate) fn send(&self, conn: ConnId, p: Packet) {
        if let Some(s) = self.sessions.get(&conn) {
            s.send(p);
        }
    }

    pub(crate) fn world_of(&self, conn: ConnId) -> Option<&InWorld> {
        self.sessions.get(&conn).and_then(|s| s.world.as_ref())
    }

    pub(crate) fn say_to(&self, conn: ConnId, text: &str) {
        self.send(conn, packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, text));
    }

    // --- inbound ------------------------------------------------------------------------------

    fn handle(&mut self, msg: Inbound) {
        match msg {
            Inbound::Connected { conn, account, out } => {
                self.account_id(&account);
                self.sessions.insert(conn, Session { account, out, world: None });
            }
            Inbound::Disconnected { conn } => {
                self.leave_world(conn);
                self.sessions.remove(&conn);
            }
            Inbound::Packet { conn, opcode, body } => {
                if let Err(e) = self.packet(conn, opcode, &body) {
                    eprintln!("[game] bad {opcode:#06x} from connection {conn}: {e}");
                }
            }
        }
    }

    fn packet(&mut self, conn: ConnId, opcode: u32, body: &[u8]) -> std::io::Result<()> {
        let Ok(opcode) = u16::try_from(opcode) else {
            return Ok(());
        };
        let mut r = Reader::new(body);
        let me = self.world_of(conn).map(|w| w.entity);
        match opcode {
            op::CMSG_PING => {
                let seq = r.u32()?;
                self.send(conn, packets::pong(seq));
            }
            op::CMSG_QUERY_TIME => self.send(conn, packets::query_time_response()),
            op::CMSG_CHAR_ENUM => self.char_enum(conn),
            op::CMSG_CHAR_CREATE => {
                let mut b = Body::new();
                b.u8(CHAR_CREATE_DISABLED);
                self.send(conn, Packet::new(op::SMSG_CHAR_CREATE, b));
            }
            op::CMSG_CHAR_DELETE => {
                let mut b = Body::new();
                b.u8(CHAR_DELETE_FAILED);
                self.send(conn, Packet::new(op::SMSG_CHAR_DELETE, b));
            }
            op::CMSG_PLAYER_LOGIN => {
                let guid = r.u64()?;
                self.enter_world(conn, guid);
            }
            op::CMSG_LOGOUT_REQUEST => {
                self.send(conn, packets::logout_response_ok());
                self.leave_world(conn);
                self.send(conn, Packet::empty(op::SMSG_LOGOUT_COMPLETE));
            }
            op::CMSG_NAME_QUERY => {
                let guid = r.u64()?;
                self.name_query(conn, guid);
            }
            op::CMSG_GAMEOBJECT_QUERY => {
                let entry = r.u32()?;
                let p = match draw::mark_template(entry) {
                    Some((display, name)) => packets::gameobject_query_response(entry, draw::MARK_TYPE, display, name),
                    None => packets::gameobject_query_miss(entry),
                };
                self.send(conn, p);
            }
            op::CMSG_CREATURE_QUERY => {
                let entry = r.u32()?;
                self.creature_query(conn, entry);
            }
            op::CMSG_ITEM_QUERY_SINGLE => {
                let entry = r.u32()?;
                self.item_query(conn, entry);
            }
            op::CMSG_NPC_TEXT_QUERY => {
                let text_id = r.u32()?;
                self.send(conn, packets::npc_text(text_id, saves::npc_text(text_id)));
            }
            op::CMSG_ATTACKSWING => {
                let guid = r.u64()?;
                self.attack_swing(conn, guid);
            }
            op::CMSG_ATTACKSTOP => {
                if let Some(me) = me {
                    self.sim.stop_attack(me);
                }
            }
            op::CMSG_SET_SELECTION => {
                let target = r.u64()?;
                if let Some(me) = me {
                    let picked = self.entities.get(&target).copied();
                    self.sim.select(me, picked);
                }
                if let Some(w) = self.world_of(conn) {
                    let mut f = Fields::new();
                    f.set_u64(idx::UNIT_TARGET, target);
                    let p = packets::update_object(&[], &[(w.guid, &f)]);
                    self.broadcast(&p);
                }
            }
            op::CMSG_SET_ACTIVE_MOVER => {
                // The client takes control of its character once its world is up; benilla zeroes
                // its tutorial memory on entering the world, so "all seen" is sent again now.
                if self.world_of(conn).is_some() {
                    self.send(conn, packets::tutorial_flags_all_seen());
                }
            }
            op::CMSG_GOSSIP_HELLO => {
                let guid = r.u64()?;
                if let (Some(me), Some(&target)) = (me, self.entities.get(&guid)) {
                    if let Some(w) = self.world_mut(conn) {
                        w.renown_menu = None;
                    }
                    self.sim.interact(me, target);
                }
            }
            op::CMSG_GOSSIP_SELECT_OPTION => {
                let _guid = r.u64()?;
                let index = r.u32()? as usize;
                if let Some(me) = me {
                    let renown = self.world_mut(conn).and_then(|w| w.renown_menu.take());
                    match renown {
                        Some(choices) => self.renown_choice(conn, choices, index),
                        None => self.sim.menu_select(me, index),
                    }
                }
            }
            op::CMSG_AUTOEQUIP_ITEM => {
                let (bag, slot) = (r.u8()?, r.u8()?);
                if let (Some(me), BAG_BACKPACK) = (me, bag) {
                    let res = self.sim.auto_equip(me, slot);
                    self.equip_result(conn, res, slot, slot);
                }
            }
            op::CMSG_AUTOSTORE_BAG_ITEM => {
                let (src_bag, src_slot, _dst_bag) = (r.u8()?, r.u8()?, r.u8()?);
                if let (Some(me), BAG_BACKPACK) = (me, src_bag) {
                    let res = self.sim.auto_equip(me, src_slot);
                    self.equip_result(conn, res, src_slot, src_slot);
                }
            }
            op::CMSG_SWAP_INV_ITEM => {
                let (src, dst) = (r.u8()?, r.u8()?);
                if let Some(me) = me {
                    let res = self.sim.swap_items(me, src, dst);
                    self.equip_result(conn, res, src, dst);
                }
            }
            op::CMSG_SWAP_ITEM => {
                let (dst_bag, dst, src_bag, src) = (r.u8()?, r.u8()?, r.u8()?, r.u8()?);
                if let (Some(me), BAG_BACKPACK, BAG_BACKPACK) = (me, dst_bag, src_bag) {
                    let res = self.sim.swap_items(me, src, dst);
                    self.equip_result(conn, res, src, dst);
                }
            }
            op::CMSG_USE_ITEM => {
                let (bag, slot) = (r.u8()?, r.u8()?);
                if let (Some(me), BAG_BACKPACK) = (me, bag) {
                    let entry = self
                        .sim
                        .player_view(me)
                        .and_then(|v| v.pack.iter().find(|(s, ..)| *s == slot).map(|(_, _, e, _)| *e));
                    if entry == Some(htl_sim::items::HEARTHSTONE) {
                        self.send(conn, packets::cast_result(htl_sim::sim::HEARTH_SPELL_ID, None));
                        self.sim.hearth(me);
                    }
                }
            }
            op::CMSG_DESTROYITEM => {
                let (bag, slot) = (r.u8()?, r.u8()?);
                if let (Some(me), BAG_BACKPACK) = (me, bag) {
                    let res = self.sim.destroy_item(me, slot);
                    self.equip_result(conn, res, slot, slot);
                }
            }
            op::CMSG_LOOT => {
                let guid = r.u64()?;
                self.loot_open(conn, guid);
            }
            op::CMSG_AUTOSTORE_LOOT_ITEM => {
                let slot = r.u8()?;
                self.loot_take(conn, slot);
            }
            op::CMSG_LOOT_MONEY => self.loot_money(conn),
            op::CMSG_LOOT_RELEASE => {
                let guid = r.u64()?;
                if let Some(w) = self.world_mut(conn) {
                    w.looting = None;
                }
                self.send(conn, packets::loot_release(guid));
            }
            op::CMSG_MESSAGECHAT => self.chat(conn, &mut r)?,
            o if MOVE_OPCODES.contains(&o) => self.movement(conn, o, body)?,
            _ => {} // everything else is ignored
        }
        Ok(())
    }

    fn equip_result(&self, conn: ConnId, res: Result<(), EquipError>, a: u8, b: u8) {
        let Err(e) = res else { return };
        let (reason, level) = match e {
            EquipError::Level(l) => (1, Some(l)),
            EquipError::CantEquip => (10, None),
            EquipError::WrongSlot => (3, None),
            EquipError::BagFull => (50, None),
            EquipError::NotFound => (23, None),
        };
        let item_guid = |slot: u8| -> u64 {
            let Some(w) = self.world_of(conn) else { return 0 };
            let Some(v) = self.sim.player_view(w.entity) else { return 0 };
            v.equip.iter().chain(v.pack.iter()).find(|(s, ..)| *s == slot).map_or(0, |(_, id, ..)| item_guid(*id))
        };
        self.send(conn, packets::inventory_failure(reason, level, item_guid(a), item_guid(b)));
    }

    fn char_enum(&mut self, conn: ConnId) {
        let Some(account) = self.sessions.get(&conn).map(|s| s.account.clone()) else {
            return;
        };
        let id = self.account_id(&account);
        let name = display_name(&account);
        let arena = self.sim.arena();
        let content = self.sim.content();
        let entries: Vec<packets::CharEntry> = content
            .classes
            .iter()
            .enumerate()
            .map(|(i, c)| packets::CharEntry {
                guid: Self::player_guid(id, i),
                name: &name,
                race: c.race,
                class: c.class_id,
                gender: c.gender,
                appearance: c.appearance,
                level: 1,
                zone: arena.area_id,
                map: arena.map_id,
                pos: arena.player_spawn,
            })
            .collect();
        // Sent at character select so the client keeps it for its world entry: a bank that
        // arrives in the world is wiped by the client's own world-enter bring-up.
        self.send(conn, packets::tutorial_flags_all_seen());
        self.send(conn, packets::char_enum(&entries));
    }

    fn enter_world(&mut self, conn: ConnId, guid: u64) {
        let Some(account) = self.sessions.get(&conn).map(|s| s.account.clone()) else {
            return;
        };
        if self.world_of(conn).is_some() {
            return;
        }
        let id = self.account_id(&account);
        let class = (0..self.sim.content().classes.len()).find(|&i| Self::player_guid(id, i) == guid);
        let Some(class) = class else {
            let mut b = Body::new();
            b.u8(0x01); // CHAR_LOGIN_FAILED
            self.send(conn, Packet::new(op::SMSG_CHARACTER_LOGIN_FAILED, b));
            return;
        };
        // One character per account in the world: a second login replaces the first.
        let dup: Vec<ConnId> = self
            .in_world()
            .filter(|(c, s, _)| **c != conn && s.account == account)
            .map(|(c, _, _)| *c)
            .collect();
        for c in dup {
            self.leave_world(c);
        }

        let name = display_name(&account);
        let arena = self.sim.arena().clone();
        let class_key = self.sim.content().classes[class].key.clone();
        let luck = self.saves.luck(&name, self.sim.content());
        let entity = self.sim.add_player(&name, &class_key, luck);
        self.bind(entity, guid);
        let (spawn, facing) = (arena.player_spawn, arena.player_facing);

        // The login sequence.
        self.send(conn, packets::login_verify_world(arena.map_id, spawn, facing));
        self.send(conn, packets::tutorial_flags_all_seen());
        self.send(conn, packets::bindpoint(spawn, arena.map_id, arena.area_id));
        self.send(conn, packets::login_set_time_speed());
        self.send(conn, packets::initial_spells(&[]));
        self.send(conn, packets::action_buttons_empty());
        self.send(conn, packets::initialize_factions_empty());
        // The client keeps templates between sessions and never asks again for one it holds, so a
        // creature, dialog or item changed since its last visit would show as it was: send them all
        // fresh before anything is drawn.
        self.push_creature_templates(conn);
        if let Some(s) = self.sessions.get_mut(&conn) {
            s.world = Some(InWorld {
                guid,
                entity,
                class,
                name: name.clone(),
                items: BTreeMap::new(),
                looting: None,
                renown_menu: None,
                counter: 0,
                fall_top: None,
                ground_z: 0.0,
                rooted: false,
                meters: (String::new(), String::new(), None),
                speed: 7.0,
                addon: false,
                guide: String::new(),
                shop: String::new(),
                at_shop: None,
                fronts: String::new(),
                map_open: false,
                addon_version: 0,
                fresh_items: BTreeSet::new(),
                boss: "BH~".to_string(),
            });
        }
        // What everyone already here wears, and what this player starts with.
        let worn: Vec<u32> = self
            .in_world()
            .filter_map(|(_, _, w)| self.sim.player_view(w.entity))
            .flat_map(|v| v.equip.iter().chain(v.pack.iter()).map(|(_, _, entry, _)| *entry).collect::<Vec<_>>())
            .collect();
        self.push_item_templates(worn);
        let me = self.player_create(entity, guid, true);
        self.send(conn, me);
        self.sync_items(conn);
        self.sim.resend_auras(entity);
        // Everything already here, then this player to everyone else.
        self.send_world_to(conn, entity);
        let p = self.player_create(entity, guid, false);
        self.broadcast_except(conn, &p);
        self.send_groups();
        let hub = self.sim.content().hub.name.clone();
        self.say_to(
            conn,
            &format!(
                "Welcome to Basin Battle. Keep the {hub} alive: cast your spells with 1, 2, 3, 4, Q \
                 and E, step out of marked ground, and pick a new card every level. Press M for the \
                 map; the Shop sells defences. Type .help for commands."
            ),
        );
        let renown = self.saves.renown_points(&name);
        if renown > 0 {
            self.say_to(conn, &format!("You have {renown} Renown to spend: type .renown."));
        }
        println!("[game] {name} entered as {class_key}");
    }

    fn leave_world(&mut self, conn: ConnId) {
        let Some(w) = self.sessions.get_mut(&conn).and_then(|s| s.world.take()) else {
            return;
        };
        self.sim.remove_player(w.entity);
        self.unbind(w.entity);
        self.broadcast(&packets::destroy_object(w.guid));
        for id in w.items.keys() {
            self.send(conn, packets::destroy_object(item_guid(*id)));
        }
        self.send(conn, packets::group_none());
        self.send_groups();
        println!("[game] {} left the world", w.name);
    }

    fn name_query(&self, conn: ConnId, guid: u64) {
        let found = self.in_world().find(|(_, _, w)| w.guid == guid).map(|(_, _, w)| {
            let c = &self.sim.content().classes[w.class];
            (w.name.clone(), c.race, c.gender, c.class_id)
        });
        let found = found.or_else(|| {
            // A character on the select screen: work out which from the guid.
            self.sessions.values().find_map(|s| {
                let id = *self.accounts.get(&s.account)?;
                let class = (0..self.sim.content().classes.len()).find(|&i| Self::player_guid(id, i) == guid)?;
                let c = &self.sim.content().classes[class];
                Some((display_name(&s.account), c.race, c.gender, c.class_id))
            })
        });
        if let Some((name, race, gender, class)) = found {
            self.send(conn, packets::name_query_response(guid, &name, race, gender, class));
        }
    }

    fn item_query(&self, conn: ConnId, entry: u32) {
        match self.item_packet(entry) {
            Some(p) => self.send(conn, p),
            None => self.send(conn, packets::item_query_miss(entry)),
        }
    }

    fn item_packet(&self, entry: u32) -> Option<Packet> {
        let t = self.sim.item_template(entry)?;
        let mask = match t.class {
            Some(c) => 1i32 << (self.sim.content().classes[c].class_id as i32 - 1),
            None => -1,
        };
        Some(packets::item_query_response(&t, mask))
    }

    /// Send every player in the world the templates of these items they haven't had fresh this
    /// session: their own and what others wear.
    pub(crate) fn push_item_templates(&mut self, entries: impl IntoIterator<Item = u32>) {
        let entries: BTreeSet<u32> = entries.into_iter().filter(|e| *e != 0).collect();
        let conns: Vec<ConnId> = self.in_world().map(|(c, _, _)| *c).collect();
        for conn in conns {
            let missing: Vec<u32> = match self.world_of(conn) {
                Some(w) => entries.iter().copied().filter(|e| !w.fresh_items.contains(e)).collect(),
                None => continue,
            };
            for e in &missing {
                if let Some(p) = self.item_packet(*e) {
                    self.send(conn, p);
                }
            }
            if let Some(w) = self.world_mut(conn) {
                w.fresh_items.extend(missing);
            }
        }
    }

    /// Every creature and dialog template, fresh.
    fn push_creature_templates(&self, conn: ConnId) {
        let content = self.sim.content();
        let (enemies, bosses) = (content.enemies.len() as u32, content.bosses.len() as u32);
        let fixed = [
            HUB_ENTRY, GATE_ENTRY, CACHE_ENTRY, BANNER_ENTRY, BANNER_CLAIMED_ENTRY, ALLY_ENTRY, SPRINT_ENTRY,
            PROTECTOR_ENTRY, SPRING_ENTRY, DEFENDER_ENTRY, PET_ENTRY, VENDOR_ENTRY, RIFLEMAN_ENTRY, CLERIC_ENTRY,
            DEFENDER_ENTRY + RANK_ENTRY_STEP, DEFENDER_ENTRY + 2 * RANK_ENTRY_STEP, DEFENDER_ENTRY + 3 * RANK_ENTRY_STEP,
        ];
        let posts = POST_ENTRY..POST_ENTRY + self.sim.arena().posts.len() as u32;
        let all = fixed
            .into_iter()
            .chain(posts)
            .chain(ENEMY_ENTRY..ENEMY_ENTRY + enemies)
            .chain(ELITE_ENTRY..ELITE_ENTRY + enemies)
            .chain(BOSS_ENTRY..BOSS_ENTRY + bosses)
            .chain((0..enemies).flat_map(|k| (1..=htl_sim::MODIFIERS.len() as u32).map(move |t| TWIST_ENTRY + 8 * k + t)));
        for entry in all {
            self.creature_query(conn, entry);
        }
        for text_id in saves::NPC_TEXTS {
            self.send(conn, packets::npc_text(text_id, saves::npc_text(text_id)));
        }
    }

    fn attack_swing(&mut self, conn: ConnId, target: u64) {
        let Some(me) = self.world_of(conn).map(|w| w.entity) else {
            return;
        };
        let victim = self.entities.get(&target).copied();
        let view = victim.and_then(|v| self.sim.entities().into_iter().find(|e| e.id == v));
        match view {
            Some(e) if matches!(e.what, htl_sim::What::Enemy { .. }) && e.alive => self.sim.attack(me, e.id),
            Some(e) if !e.alive => self.send(conn, Packet::empty(op::SMSG_ATTACKSWING_DEADTARGET)),
            _ => self.send(conn, Packet::empty(op::SMSG_ATTACKSWING_CANT_ATTACK)),
        }
    }

    fn movement(&mut self, conn: ConnId, opcode: u16, body: &[u8]) -> std::io::Result<()> {
        let Some(w) = self.world_of(conn) else {
            return Ok(());
        };
        let (guid, entity) = (w.guid, w.entity);
        let mut r = Reader::new(body);
        let flags = r.u32()?;
        let _time = r.u32()?;
        let pos = [r.f32()?, r.f32()?, r.f32()?];
        let facing = r.f32()?;
        if pos.iter().all(|v| v.is_finite()) {
            self.sim.move_player(entity, pos, facing);
            if opcode == op::MSG_MOVE_FALL_LAND {
                let fall_ms = fall_time(&mut r, flags).unwrap_or(0);
                self.landed(conn, pos[2], fall_ms);
            } else if let Some(w) = self.world_mut(conn) {
                if flags & (MOVE_FALLING | MOVE_FALLING_FAR) != 0 {
                    let top = w.fall_top.unwrap_or(w.ground_z);
                    w.fall_top = Some(top.max(pos[2]));
                } else {
                    w.ground_z = pos[2];
                    w.fall_top = None;
                }
            }
        }
        let mut b = Body::new();
        b.packed_guid(guid).bytes(body);
        self.broadcast_except(conn, &Packet::new(opcode, b));
        Ok(())
    }

    /// A landing: the drop is the lesser of where the fall began less where it ended, and how far
    /// anything falls in the client's fall time (each overestimates in its own way: a stale
    /// ground height, a jump's rise). The sim decides what that costs.
    fn landed(&mut self, conn: ConnId, z: f32, fall_ms: u32) {
        let Some(w) = self.world_mut(conn) else { return };
        let top = w.fall_top.take().unwrap_or(w.ground_z).max(z);
        w.ground_z = z;
        let entity = w.entity;
        let by_height = top - z;
        let by_time = fall_distance(fall_ms as f32 / 1000.0);
        let drop = by_height.min(by_time);
        if drop > 1.0 {
            self.sim.fall(entity, drop);
        }
    }

    fn chat(&mut self, conn: ConnId, r: &mut Reader) -> std::io::Result<()> {
        let kind = r.u32()?;
        let language = r.u32()?;
        // Whispers and channels carry a target first. There are no whispers between players; a
        // whisper is how the add-on sends a command while its player is dead (the client won't
        // let the dead say anything aloud).
        if kind == 14 {
            return Ok(());
        }
        if kind == 6 {
            let _target = r.cstr()?;
            let text = r.cstr()?;
            let Some(name) = self.world_of(conn).map(|w| w.name.clone()) else { return Ok(()) };
            if let Some(cmd) = text.trim().strip_prefix('.') {
                self.command(conn, &name, cmd);
            }
            return Ok(());
        }
        let text = r.cstr()?;
        let Some(w) = self.world_of(conn) else {
            return Ok(());
        };
        let (guid, name) = (w.guid, w.name.clone());
        let trimmed = text.trim();
        if let Some(cmd) = trimmed.strip_prefix('.') {
            self.command(conn, &name, cmd);
            return Ok(());
        }
        let out_kind = match kind as u8 {
            packets::CHAT_YELL => packets::CHAT_YELL,
            packets::CHAT_PARTY => packets::CHAT_PARTY,
            _ => packets::CHAT_SAY,
        };
        let lang = if language == 0xFFFF_FFFF { packets::LANG_UNIVERSAL } else { language };
        self.broadcast(&packets::chat(out_kind, lang, guid, &text));
        Ok(())
    }

    fn command(&mut self, conn: ConnId, name: &str, cmd: &str) {
        let Some(me) = self.world_of(conn).map(|w| w.entity) else { return };
        let mut words = cmd.split_whitespace();
        match words.next().unwrap_or("") {
            "start" => {
                self.sim.start_now();
                println!("[game] {name} started the wave");
            }
            "restart" => {
                self.sim.restart();
                println!("[game] {name} restarted the run");
            }
            "status" => {
                let s = self.sim.status();
                let affixes = self.sim.affix_names();
                let mut text = format!(
                    "Wave {} ({:?}){}. Deepest wave held: {}. {}: {}/{}. Enemies left: {}. Bases claimed: {}/{}.",
                    s.wave,
                    s.phase,
                    if s.cycle > 0 { format!(", Escalation {}", s.cycle) } else { String::new() },
                    self.sim.depth(),
                    self.sim.content().hub.name,
                    s.hub_health,
                    s.hub_max_health,
                    s.enemies_left,
                    s.bases_claimed,
                    s.bases
                );
                if !affixes.is_empty() {
                    text.push_str(&format!(" Affixes: {}.", affixes.join(", ")));
                }
                self.say_to(conn, &text);
            }
            "cards" => {
                if let Some(v) = self.sim.player_view(me) {
                    let list: Vec<String> = v.cards.iter().map(|(n, r)| format!("{n} {r}")).collect();
                    let text = if list.is_empty() {
                        "You hold no cards yet.".to_string()
                    } else {
                        format!("Your cards: {}.", list.join(", "))
                    };
                    self.say_to(conn, &text);
                }
                if !self.sim.reopen_offers(me) {
                    self.say_to(conn, "No card picks are waiting.");
                }
            }
            "pick" => {
                let n = words.next().and_then(|w| w.parse::<usize>().ok()).unwrap_or(0);
                if !self.sim.pick(me, n) {
                    self.say_to(conn, "Nothing to pick: type .cards to see your picks.");
                }
            }
            "hearth" | "recall" => self.sim.hearth(me),
            "best" => {
                let lines = self.saves.best_lines();
                for l in lines {
                    self.say_to(conn, &l);
                }
            }
            "renown" => self.open_renown(conn),
            "bb" => self.ui_command(conn, me, &mut words),
            // Elwynn Survival's add-on says hello too: it's asleep here.
            "es" => {}
            // `.hold 1`: you'll hold the first bridge (0 the Ancient); `.sos`: help!
            "hold" => {
                let site = words.next().and_then(|w| w.parse::<usize>().ok()).unwrap_or(0);
                if self.sim.ping(me, htl_sim::Ping::Hold(site)).is_none() {
                    let places: Vec<String> = (0..self.sim.site_count()).map(|s| format!("{s}: {}", self.sim.site_name(s))).collect();
                    self.say_to(conn, &format!("Hold where? {}", places.join(", ")));
                }
            }
            "sos" => {
                self.sim.ping(me, htl_sim::Ping::Help);
            }
            _ => self.say_to(
                conn,
                "Commands: .start (next wave now), .status, .cards (your cards and waiting picks), \
                 .pick 1-4, .hearth (back to the Ancient), .best (high scores), .renown (spend Renown), \
                 .hold <n> (say you'll hold a bridge), .sos (call for help), .restart (start the run over).",
            ),
        }
    }

    /// Milliseconds since the server started.
    pub fn uptime_ms(&self) -> u32 {
        self.started.elapsed().as_millis() as u32
    }
}

pub(crate) fn creature_guid(entry: u32, id: EntityId) -> u64 {
    (HIGH_UNIT << 48) | (u64::from(entry) << 24) | u64::from(id & 0x00FF_FFFF)
}

pub(crate) fn zone_guid(id: EntityId) -> u64 {
    (HIGH_DYNAMIC << 48) | u64::from(id)
}

/// A ground mark's game object, beside the zone it marks.
pub(crate) fn mark_guid(id: EntityId) -> u64 {
    (HIGH_GAMEOBJECT << 48) | u64::from(id)
}

pub(crate) fn item_guid(id: u32) -> u64 {
    (HIGH_ITEM << 48) | u64::from(id)
}

pub(crate) fn entry_of(kind: EnemyKind, elite: bool, twist: u8) -> u32 {
    if kind.boss {
        BOSS_ENTRY + kind.index as u32
    } else if elite && twist > 0 {
        TWIST_ENTRY + 8 * kind.index as u32 + twist as u32
    } else if elite {
        ELITE_ENTRY + kind.index as u32
    } else {
        ENEMY_ENTRY + kind.index as u32
    }
}

pub(crate) fn npc_entry(npc: NpcKind) -> u32 {
    match npc {
        NpcKind::Gate => GATE_ENTRY,
        NpcKind::Cache => CACHE_ENTRY,
        NpcKind::Banner { claimed: false } => BANNER_ENTRY,
        NpcKind::Banner { claimed: true } => BANNER_CLAIMED_ENTRY,
        NpcKind::Sprint => SPRINT_ENTRY,
        NpcKind::Protector => PROTECTOR_ENTRY,
        NpcKind::Spring => SPRING_ENTRY,
        NpcKind::Post(k) => POST_ENTRY + k as u32,
        NpcKind::Vendor => VENDOR_ENTRY,
    }
}

/// Which enemy an entry is, and whether it's the elite version.
pub(crate) fn kind_of_entry(entry: u32, enemies: usize, bosses: usize) -> Option<(EnemyKind, bool)> {
    if (ENEMY_ENTRY..ENEMY_ENTRY + enemies as u32).contains(&entry) {
        Some((EnemyKind { boss: false, index: (entry - ENEMY_ENTRY) as usize }, false))
    } else if (ELITE_ENTRY..ELITE_ENTRY + enemies as u32).contains(&entry) {
        Some((EnemyKind { boss: false, index: (entry - ELITE_ENTRY) as usize }, true))
    } else if (TWIST_ENTRY..TWIST_ENTRY + 8 * enemies as u32).contains(&entry) {
        Some((EnemyKind { boss: false, index: ((entry - TWIST_ENTRY) / 8) as usize }, true))
    } else if (BOSS_ENTRY..BOSS_ENTRY + bosses as u32).contains(&entry) {
        Some((EnemyKind { boss: true, index: (entry - BOSS_ENTRY) as usize }, false))
    } else {
        None
    }
}

/// `fields` is used by the submodules.
#[allow(unused_imports)]
use fields as _;

// The movement flags a fall shows (1.12): in the air, and falling past a jump's arc; and the two
// whose tails come before the fall time.
const MOVE_FALLING: u32 = 0x0000_2000;
const MOVE_FALLING_FAR: u32 = 0x0000_4000;
const MOVE_SWIMMING: u32 = 0x0020_0000;
const MOVE_ON_TRANSPORT: u32 = 0x0200_0000;

/// The fall time in a movement packet's tail, after the transport pose and the swim pitch.
pub(crate) fn fall_time(r: &mut Reader, flags: u32) -> std::io::Result<u32> {
    if flags & MOVE_ON_TRANSPORT != 0 {
        let _ = (r.u64()?, r.f32()?, r.f32()?, r.f32()?, r.f32()?);
    }
    if flags & MOVE_SWIMMING != 0 {
        let _ = r.f32()?;
    }
    r.u32()
}

/// How far something falls in `secs` from rest: the client's gravity (19.29 yd/s²) up to its
/// terminal speed (60 yd/s).
pub(crate) fn fall_distance(secs: f32) -> f32 {
    const G: f32 = 19.291_105;
    const TERMINAL: f32 = 60.148_003;
    let t_max = TERMINAL / G;
    if secs <= t_max {
        0.5 * G * secs * secs
    } else {
        0.5 * G * t_max * t_max + TERMINAL * (secs - t_max)
    }
}

#[cfg(test)]
mod fall_tests {
    #[test]
    fn falls_follow_the_clients_gravity() {
        assert!((super::fall_distance(1.0) - 9.65).abs() < 0.01);
        // Past terminal speed it's a straight line.
        let (a, b) = (super::fall_distance(5.0), super::fall_distance(6.0));
        assert!((b - a - 60.15).abs() < 0.01);
    }
}

#[cfg(test)]
mod ware_tests {
    use benilla_protocol::messages::parse_server_with_tail;

    /// Every ware's template, for every class and tier, reads cleanly in the client.
    #[test]
    fn the_client_reads_every_ware() {
        let content = htl_sim::Content::builtin();
        for w in 0..content.wares.len() {
            for class in 0..content.classes.len() {
                for tier in [0u8, 4, 9] {
                    let entry = htl_sim::items::ware_entry(w, class, tier);
                    let t = htl_sim::items::template(&content, entry).expect("a template");
                    assert!(!t.stats.is_empty(), "{} has stats", t.name);
                    let p = crate::packets::item_query_response(&t, -1);
                    let (packet, tail) = parse_server_with_tail(p.opcode, &p.body).expect("benilla parses it");
                    assert_eq!(tail, 0, "{}", packet.name());
                }
            }
        }
    }
}
