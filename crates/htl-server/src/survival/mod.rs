//! The Elwynn Survival world server: its own realm and world port beside Basin Battle's, the same
//! client and login. It answers the client, feeds [`Survival`] what heroes do (movement, the
//! add-on's action inputs, right-clicks on objects) and plays the rules' events out as packets.
//! The survival add-on wakes only for this server; Basin Battle's stays asleep here.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use benilla_protocol::messages::opcode as op;
use crossbeam_channel::{Receiver, Sender};
use htl_survival::save::{PlayerSave, WorldSave};
use htl_survival::{Id, Moving, Survival};

use crate::fields::{self, idx, Fields};
use crate::packets::{self, Packet};
use crate::wire::{Body, Reader};
use crate::world::{ConnId, Inbound};

pub mod board;
mod draw;
mod heroes;
pub mod land;
mod play;
pub mod realms;
mod siege_ui;
mod ui;
mod wager_ui;

pub(crate) const TICK: Duration = Duration::from_millis(50);
/// Saves at most this often (and when a hero leaves).
const SAVE_EVERY: Duration = Duration::from_secs(5);

/// Elwynn Forest, and Goldshire in it, on the Eastern Kingdoms.
pub const MAP: u32 = 0;
pub const ZONE: u32 = 12;
pub const AREA: u32 = 87;

/// Which world a game is: the camp's (Elwynn), the one below (the Deadmines' instance map), or
/// Siege's (Arathi Basin).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Realm {
    pub map: u32,
    pub zone: u32,
    pub area: u32,
    pub below: bool,
    pub siege: bool,
}

impl Realm {
    pub const CAMP: Realm = Realm {
        map: MAP,
        zone: ZONE,
        area: AREA,
        below: false,
        siege: false,
    };
    pub const BELOW: Realm = Realm {
        map: htl_map::deadmines::MAP_ID,
        zone: htl_map::deadmines::AREA_ID,
        area: htl_map::deadmines::AREA_ID,
        below: true,
        siege: false,
    };

    /// A dungeon's world, on its instance map.
    pub fn dungeon(d: &htl_survival::content::DungeonDef) -> Realm {
        Realm { map: d.map, zone: d.area, area: d.area, below: true, siege: false }
    }
    pub const SIEGE: Realm = Realm {
        map: htl_map::siege::MAP_ID,
        zone: htl_map::siege::AREA_ID,
        area: htl_map::siege::AREA_ID,
        below: false,
        siege: true,
    };

    /// The world's name, for the log.
    pub fn name(&self) -> &'static str {
        if self.below && self.map == htl_map::deadmines::MAP_ID {
            "the Deadmines"
        } else if self.below {
            "a dungeon below"
        } else if self.siege {
            "Arathi Basin"
        } else {
            "Elwynn"
        }
    }
}

/// What a hero's client brings from one world to the other: its add-on and its round trip.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Carry {
    pub addon: bool,
    pub anim: bool,
    pub local: bool,
    pub level: u8,
    pub rtt: f32,
}

/// Client movement opcodes, relayed to everyone else.
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

pub(crate) struct Session {
    pub account: String,
    /// The character picked to play (its key: `account/name`, or the account's own for its first
    /// footman), set as it logs in.
    pub hero: String,
    pub out: Sender<Packet>,
    pub world: Option<InWorld>,
}

impl Session {
    pub fn send(&self, p: Packet) {
        let _ = self.out.send(p);
    }
}

pub(crate) struct InWorld {
    pub guid: u64,
    pub id: Id,
    pub name: String,
    /// Item objects the client knows: guid to (entry, count).
    pub items: BTreeMap<u64, (u32, u32)>,
    pub fresh_items: BTreeSet<u32>,
    pub counter: u32,
    /// The client runs the survival add-on.
    pub addon: bool,
    /// The client plays clips, hit-stops and rolls itself (the action-combat client's second
    /// round): it gets the `AN`, `IM` and `RO` lines in place of emotes.
    pub anim: bool,
    /// The client runs its own dashes and lunges (the third round, `ac3`).
    pub local: bool,
    /// Which round of the action-combat client it is (0: a stock client): `ac4` plays its own
    /// blows' weight (hit-stop, shake, flash) the moment they connect.
    pub level: u8,
    /// The round trip as the add-on's pings measure it (seconds), and the ping out now.
    pub rtt: f32,
    pub ping: Option<(u32, Instant)>,
    pub ping_n: u32,
    pub ping_at: Option<Instant>,
    /// Add-on lines as last sent, by verb.
    pub sent: HashMap<&'static str, String>,
    /// The station whose window is open, if any.
    pub station: Option<Id>,
    /// Fall tracking (as Basin Battle's): the height a fall began from.
    pub fall_top: Option<f32>,
    pub ground_z: f32,
    /// Come from the other world, the client is loading this one: it isn't drawn in until it
    /// says it's here.
    pub porting: bool,
}

impl InWorld {
    fn new(guid: u64, id: Id, name: String, z: f32) -> Self {
        Self {
            guid,
            id,
            name,
            items: BTreeMap::new(),
            fresh_items: BTreeSet::new(),
            counter: 0,
            addon: false,
            anim: false,
            local: false,
            level: 0,
            rtt: 0.0,
            ping: None,
            ping_n: 0,
            ping_at: None,
            sent: HashMap::new(),
            station: None,
            fall_top: None,
            ground_z: z,
            porting: false,
        }
    }
}

pub struct SurvivalGame {
    pub(crate) sim: Survival,
    pub(crate) sessions: HashMap<ConnId, Session>,
    /// Object and creature guids by the rules' ids, and back.
    pub(crate) guids: HashMap<Id, u64>,
    pub(crate) ids: HashMap<u64, Id>,
    /// Game object templates handed out: (display, name, type) to entry.
    pub(crate) go_entries: HashMap<(u32, String, u32), u32>,
    pub(crate) spline_id: u32,
    /// The ring under each station being placed.
    pub(crate) rings: HashMap<Id, u64>,
    /// Lingering ground's looks (DynamicObjects), by the mark they go with.
    pub(crate) area_looks: HashMap<Id, u64>,
    /// Danger marks on the ground: where, and how wide.
    pub(crate) mark_at: HashMap<Id, ([f32; 3], f32)>,
    /// The creatures whose weapons were last shown red, until when (one `WG` per wind-up).
    pub(crate) warned_glow: HashMap<Id, f32>,
    /// The add-on's lines waiting for the next flush.
    pub(crate) hud_at: Instant,
    /// Who's casting what (to stop it cleanly).
    pub(crate) casting: HashMap<Id, u32>,
    /// The chest changed: tell whoever has it open.
    pub(crate) storage_dirty: bool,
    /// Heroes whose ability bar or tree changed (their add-on is told on the next flush), and how
    /// many aura slots each unit showed last.
    pub(crate) abilities_dirty: BTreeSet<Id>,
    pub(crate) auras_shown: HashMap<Id, usize>,
    /// Healing come in since the last flush, by hero (shown as one green number).
    pub(crate) healed: HashMap<Id, u32>,
    pub(crate) save_path: Option<PathBuf>,
    pub(crate) offline: BTreeMap<String, PlayerSave>,
    /// What's been tuned in play since the rules were read (`.es tune`): each path and value.
    pub(crate) tuned: Vec<(String, String)>,
    pub(crate) realm: Realm,
    /// What arriving heroes' clients brought, until they've loaded this world.
    pub(crate) pending_carry: HashMap<ConnId, Carry>,
    /// Heroes going to the other world: their connection, and where (a lamp below, or the camp).
    pub(crate) transfers: Vec<(ConnId, Option<String>)>,
    /// The camp started over (`.es reset camp`): the Deadmines start over with it.
    pub(crate) reset_below: bool,
    /// Heroes whose view of Siege changed (their corner and open window are told on the next
    /// flush).
    pub(crate) siege_dirty: BTreeSet<Id>,
    /// Done below, for the camp: journal counts and trophies.
    pub(crate) to_camp: Vec<(Vec<(String, u32)>, BTreeMap<usize, u32>)>,
    /// Wagers won below, for the Bookmaker's Board.
    pub(crate) wagers: Vec<htl_survival::sim::wager::WagerRun>,
    /// The Bookmaker's Board (shared with the login port, which serves it as the board host).
    pub(crate) board: Option<board::Shared>,
    dirty: bool,
    saved_at: Instant,
}

/// A hero's name: their save's (a character's own), else their account's (its first footman).
pub(crate) fn hero_name(key: &str, save: Option<&PlayerSave>) -> String {
    match save.map(|s| s.name.as_str()).filter(|n| !n.is_empty()) {
        Some(n) => n.to_string(),
        None => crate::game::display_name(key.split('/').next().unwrap_or(key)),
    }
}

/// Seconds since the epoch, for nodes that come back by real time.
pub fn unix_now() -> f64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0.0, |d| d.as_secs_f64())
}

impl SurvivalGame {
    /// The world from its save file if there is one, else a new one.
    pub fn new(content: htl_survival::Content, land: Box<dyn htl_survival::Land>, save_path: Option<PathBuf>, seed: u64,
    ) -> Self {
        Self::new_in(Realm::CAMP, content, land, save_path, seed)
    }

    /// A game for one of the worlds: the camp's, or the Deadmines'.
    pub fn new_in(
        realm: Realm,
        content: htl_survival::Content,
        land: Box<dyn htl_survival::Land>,
        save_path: Option<PathBuf>,
        seed: u64,
    ) -> Self {
        let saved: Option<WorldSave> = save_path.as_ref().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|t| match htl_survival::save::from_toml(&t) {
            Ok(s) => Some(s),
            Err(e) => {
                eprintln!("[survival] couldn't read the save ({e:#}); starting a new world");
                None
            }
        });
        let now = unix_now();
        let (sim, offline) = match (saved, realm.below) {
            // Siege keeps nothing: every match starts the same.
            _ if realm.siege => (Survival::new_siege(content, land, seed, now), BTreeMap::new()),
            (Some(s), false) => {
                let players = s.players.clone();
                println!("[survival] world loaded: {} stations, {} heroes known", s.stations.len(), players.len());
                (Survival::load(content, land, seed, now, &s), players)
            }
            (Some(s), true) => {
                let players = s.players.clone();
                println!(
                    "[survival] the Deadmines loaded: {} heroes below",
                    players.len()
                );
                (
                    Survival::load_dungeon(content, land, seed, now, &s),
                    players,
                )
            }
            (None, false) => (Survival::new(content, land, seed, now), BTreeMap::new()),
            (None, true) => (
                Survival::new_dungeon(content, land, seed, now),
                BTreeMap::new(),
            ),
        };
        let mut g = Self {
            sim,
            sessions: HashMap::new(),
            guids: HashMap::new(),
            ids: HashMap::new(),
            go_entries: HashMap::new(),
            spline_id: 1,
            rings: HashMap::new(),
            area_looks: HashMap::new(),
            mark_at: HashMap::new(),
            warned_glow: HashMap::new(),
            hud_at: Instant::now(),
            casting: HashMap::new(),
            storage_dirty: false,
            abilities_dirty: BTreeSet::new(),
            auras_shown: HashMap::new(),
            healed: HashMap::new(),
            save_path,
            offline,
            tuned: Vec::new(),
            realm,
            pending_carry: HashMap::new(),
            transfers: Vec::new(),
            reset_below: false,
            siege_dirty: BTreeSet::new(),
            to_camp: Vec::new(),
            wagers: Vec::new(),
            board: None,
            dirty: true,
            saved_at: Instant::now(),
        };
        g.bind_world();
        g
    }

    pub fn run( self, inbound: Receiver<Inbound>) {
        realms::Realms::new(self, Vec::new()).run(inbound);
            }

    /// One tick: the rules step, their events play out, the add-ons hear what changed, and the
    /// world is saved now and then.
    pub(crate) fn tick(&mut self) {
                let events = self.sim.step(TICK.as_secs_f32());
                self.play(events);
                self.ui_flush();
                if self.dirty && self.saved_at.elapsed() >= SAVE_EVERY {
                    self.save();
        }
    }

    pub(crate) fn save(&mut self) {
        self.dirty = false;
        self.saved_at = Instant::now();
        let Some(path) = &self.save_path else { return };
        let save = self.sim.save(&self.offline);
        match htl_survival::save::to_toml(&save) {
            Ok(text) => {
                if let Some(dir) = path.parent() {
                    let _ = std::fs::create_dir_all(dir);
                }
                // Written whole, then moved into place: a crash mid-write never leaves half a file.
                let tmp = path.with_extension("tmp");
                if std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path)).is_err() {
                    eprintln!("[survival] couldn't write {}", path.display());
                }
            }
            Err(e) => eprintln!("[survival] couldn't save: {e:#}"),
        }
    }

    /// The save as it stands, written and then copied beside itself (`<save>.before-reset-<time>`):
    /// where the copy is, if there's a save at all.
    /// This world's name, for the log: a dungeon's own.
    pub(crate) fn realm_name(&self) -> String {
        match self.sim.content.dungeon.as_ref().filter(|_| self.realm.below) {
            Some(d) => d.name.clone(),
            None => self.realm.name().to_string(),
        }
    }

    pub(crate) fn keep_save_aside(&mut self) -> Result<Option<PathBuf>, String> {
        self.save();
        let Some(p) = self.save_path.clone().filter(|p| p.exists()) else { return Ok(None) };
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let mut to = p.clone().into_os_string();
        to.push(format!(".before-reset-{secs}"));
        let to = PathBuf::from(to);
        std::fs::copy(&p, &to).map_err(|e| format!("Couldn't keep the save at {} ({e}); nothing was reset.", to.display()))?;
        Ok(Some(to))
    }

    pub(crate) fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    pub(crate) fn handle(&mut self, msg: Inbound) {
        match msg {
            Inbound::Connected { conn, account, out } => {
                let hero = account.to_lowercase();
                self.sessions.insert(conn, Session { account, hero, out, world: None });
            }
            Inbound::Disconnected { conn } => {
                self.leave_world(conn);
                self.sessions.remove(&conn);
            }
            Inbound::Packet { conn, opcode, body } => {
                if let Err(e) = self.packet(conn, opcode, &body) {
                    eprintln!("[survival] bad {opcode:#06x} from connection {conn}: {e}");
                }
            }
        }
    }

    pub(crate) fn world_of(&self, conn: ConnId) -> Option<&InWorld> {
        self.sessions.get(&conn).and_then(|s| s.world.as_ref())
    }

    pub(crate) fn world_mut(&mut self, conn: ConnId) -> Option<&mut InWorld> {
        self.sessions.get_mut(&conn).and_then(|s| s.world.as_mut())
    }

    pub(crate) fn in_world(&self) -> impl Iterator<Item = (&ConnId, &Session, &InWorld)> {
        self.sessions.iter().filter_map(|(c, s)| s.world.as_ref().map(|w| (c, s, w)))
    }

    pub(crate) fn conn_of(&self, id: Id) -> Option<ConnId> {
        self.in_world().find(|(_, _, w)| w.id == id).map(|(c, _, _)| *c)
    }

    pub(crate) fn send(&self, conn: ConnId, p: Packet) {
        if let Some(s) = self.sessions.get(&conn) {
            s.send(p);
        }
    }

    pub(crate) fn broadcast(&self, p: &Packet) {
        for (_, s, _) in self.in_world() {
            s.send(p.clone());
        }
    }

    /// To the clients that don't play the action layer's clips (whose reactions already show what
    /// an emote would, and on which a state emote sent as a one-shot would loop on and on).
    pub(crate) fn broadcast_stock(&self, p: &Packet) {
        for (_, s, w) in self.in_world() {
            if !w.anim {
                s.send(p.clone());
            }
        }
    }

    pub(crate) fn broadcast_except(&self, except: ConnId, p: &Packet) {
        for (c, s, _) in self.in_world() {
            if *c != except {
                s.send(p.clone());
            }
        }
    }

    pub(crate) fn to_player(&self, id: Id, p: Packet) {
        if let Some(c) = self.conn_of(id) {
            self.send(c, p);
        }
    }

    pub(crate) fn say_to(&self, conn: ConnId, text: &str) {
        self.send(conn, packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, text));
    }

    pub(crate) fn guid_of(&self, id: Id) -> Option<u64> {
        self.guids.get(&id).copied()
    }

    /// A stable player guid for an account (the same every login).
    pub(crate) fn player_guid(account: &str) -> u64 {
        let mut h: u32 = 0x811C_9DC5;
        for b in account.to_lowercase().bytes() {
            h ^= b as u32;
            h = h.wrapping_mul(0x0100_0193);
        }
        u64::from(h & 0x00FF_FFFF) | 1
    }

    fn packet(&mut self, conn: ConnId, opcode: u32, body: &[u8]) -> std::io::Result<()> {
        let Ok(opcode) = u16::try_from(opcode) else { return Ok(()) };
        let mut r = Reader::new(body);
        let me = self.world_of(conn).map(|w| w.id);
        match opcode {
            op::CMSG_PING => {
                let seq = r.u32()?;
                self.send(conn, packets::pong(seq));
            }
            op::CMSG_QUERY_TIME => self.send(conn, packets::query_time_response()),
            // The character list, made and deleted: the realms' (`heroes`).
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
            op::CMSG_CREATURE_QUERY => {
                let entry = r.u32()?;
                self.creature_query(conn, entry);
            }
            op::CMSG_GAMEOBJECT_QUERY => {
                let entry = r.u32()?;
                self.gameobject_query(conn, entry);
            }
            op::CMSG_ITEM_QUERY_SINGLE => {
                let entry = r.u32()?;
                self.item_query(conn, entry);
            }
            op::MSG_MOVE_WORLDPORT_ACK => self.worldport_ack(conn),
            op::CMSG_SET_ACTIVE_MOVER => {
                if self.world_of(conn).is_some() {
                    self.send(conn, packets::tutorial_flags_all_seen());
                }
            }
            op::CMSG_GAMEOBJ_USE => {
                let guid = r.u64()?;
                if let (Some(me), Some(&obj)) = (me, self.ids.get(&guid)) {
                    self.sim.interact(me, Some(obj));
                }
            }
            // Right-clicking a creature would start the client's own auto-attack: here every blow
            // is the hero's own swing, so it's turned down.
            op::CMSG_ATTACKSWING => {
                let _ = r.u64()?;
                self.send(conn, Packet::empty(op::SMSG_ATTACKSWING_CANT_ATTACK));
            }
            op::CMSG_USE_ITEM => {
                // Something in the bags, right-clicked: use that one (it becomes the quick item).
                let (bag, slot) = (r.u8().unwrap_or(0), r.u8().unwrap_or(0));
                if let Some(me) = me {
                    let item = self.pack_slot(bag, slot).and_then(|k| self.pack(me).into_iter().find(|(s, _, _)| *s == k)).map(|(_, entry, _)| entry.wrapping_sub(draw::ITEM_ENTRY) as usize);
                    match item {
                        Some(i) if i < self.sim.content.items.len() => self.sim.use_item(me, i),
                        _ => self.sim.input(me, htl_survival::Input::Use),
                    }
                }
            }
            op::CMSG_SWAP_INV_ITEM | op::CMSG_SWAP_ITEM | op::CMSG_AUTOEQUIP_ITEM | op::CMSG_AUTOSTORE_BAG_ITEM | op::CMSG_SPLIT_ITEM | op::CMSG_DESTROYITEM => {
                // (container, slot) pairs as the client numbers them: where from, and where to.
                let mut b = [0u8; 4];
                for x in &mut b {
                    *x = r.u8().unwrap_or(0);
                }
                let (from, to) = match opcode {
                    // Right-clicked in the bags: wear it.
                    op::CMSG_AUTOEQUIP_ITEM => ((b[0], b[1]), Some((255, 0))),
                    op::CMSG_SWAP_INV_ITEM => ((255, b[0]), Some((255, b[1]))),
                    op::CMSG_SWAP_ITEM => ((b[2], b[3]), Some((b[0], b[1]))),
                    _ => ((b[0], b[1]), None),
                };
                if let Some(me) = me {
                    // Gear from the bags dropped on the hero (or right-clicked): put it on.
                    let onto_hero = to.is_some_and(|(bag, slot)| bag == 255 && slot < draw::BAG_SLOT_FIRST);
                    let item = self
                        .pack_slot(from.0, from.1)
                        .and_then(|k| self.pack(me).into_iter().find(|(s, _, _)| *s == k))
                        .map(|(_, entry, _)| entry.wrapping_sub(draw::ITEM_ENTRY) as usize);
                    if let (true, Some(i)) = (onto_hero, item) {
                        if let Err(e) = self.sim.wear_from_bag(me, i) {
                            self.say_to(conn, &e);
                        }
                    }
                }
                // The bags are the rules' to keep: the client's view put back as they say, and what
                // it held up (greyed) let go by making it afresh.
                let owner = self.world_of(conn).map_or(0, |w| w.guid);
                let touched: Vec<u64> = [Some(from), to].into_iter().flatten().filter_map(|(bag, slot)| self.seat_guid(owner, bag, slot)).collect();
                for g in touched {
                    if self.world_mut(conn).is_some_and(|w| w.items.remove(&g).is_some()) {
                        self.send(conn, packets::destroy_object(g));
                    }
                }
                self.sync_items(conn);
            }
            op::CMSG_MESSAGECHAT => self.chat(conn, &mut r)?,
            o if MOVE_OPCODES.contains(&o) => self.movement(conn, o, body)?,
            _ => {}
        }
        Ok(())
    }

    fn enter_world(&mut self, conn: ConnId, guid: u64) {
        let Some(account) = self.sessions.get(&conn).map(|s| s.hero.clone()) else { return };
        if self.world_of(conn).is_some() {
            return;
        }
        if guid != Self::player_guid(&account) || !self.offline.contains_key(&account) && !self.sim.players.values().any(|p| p.account == account) {
            let mut b = Body::new();
            b.u8(0x01);
            self.send(conn, Packet::new(op::SMSG_CHARACTER_LOGIN_FAILED, b));
            return;
        }
        // One of each hero in the world: a second login as them replaces the first.
        let dup: Vec<ConnId> = self.in_world().filter(|(c, s, _)| **c != conn && s.hero == account).map(|(c, _, _)| *c).collect();
        for c in dup {
            self.leave_world(c);
        }
        let saved = self.offline.remove(&account);
        let name = hero_name(&account, saved.as_ref());
        let id = self.sim.join(&account, &name, saved.as_ref());
        self.guids.insert(id, guid);
        self.ids.insert(guid, id);
        let (pos, facing) = {
            let p = &self.sim.players[&id];
            (p.pos, p.facing)
        };
        self.send(
            conn,
            packets::login_verify_world(self.realm.map, pos, facing),
        );
        if let Some(s) = self.sessions.get_mut(&conn) {
            s.world = Some(InWorld::new(guid, id, name, pos[2]));
        }
        self.finish_entry(conn, id, None);
        let caches = self.sim.caches_of(&account).len();
        if caches > 0 && !self.realm.below && !self.realm.siege {
            self.say_to(
                conn,
                &format!("You have {caches} lot(s) of supplies waiting where you fell."),
            );
        }
        self.mark_dirty();
    }

    /// Where a hero with no save stands: the camp's spawn, or the first lamp below.
    fn spawn_point(&self) -> [f32; 3] {
        if let Some(d) = self.sim.content.siege.as_ref().filter(|_| self.realm.siege) {
            return d.lobby;
        }
        if self.realm.below {
            if let Some(l) = self
                .sim
                .content
                .dungeon
                .as_ref()
                .and_then(|d| d.lamps.first())
            {
                return l.at;
            }
        }
        let s = self.sim.content.world.spawn;
            [s[0], s[1], self.sim.land().height(s[0], s[1])]
        }/// The rest of coming into this world (at login, or once the client has loaded it after the
    /// other): the bind point, the hero drawn, their gear, the world around them, and a welcome.
    fn finish_entry(&mut self, conn: ConnId, id: Id, carry: Option<Carry>) {
        let spawn = self.spawn_point();
        self.send(conn, packets::tutorial_flags_all_seen());
        self.send(conn, packets::bindpoint(spawn, self.realm.map, self.realm.area));
        self.send(conn, packets::login_set_time_speed());
        self.send(conn, packets::initial_spells(&[]));
        self.send(conn, packets::action_buttons_empty());
        self.send(conn, packets::initialize_factions_empty());
        self.push_templates(conn);
        let me = self.player_create(id, true);
        self.send(conn, me);
        self.sync_items(conn);
        self.send_world_to(conn, id);
        let other = self.player_create(id, false);
        self.broadcast_except(conn, &other);
        let name = self
            .world_of(conn)
            .map(|w| w.name.clone())
            .unwrap_or_default();
        match (carry, self.realm.below) {
            (Some(c), _) => {
                // From the other world: the add-on carries on as it was.
                if let Some(w) = self.world_mut(conn) {
                    w.addon = c.addon;
                    w.anim = c.anim;
                    w.local = c.local;
                    w.level = c.level;
                    w.rtt = c.rtt;
                }
                if let Some(p) = self.sim.players.get_mut(&id) {
                    p.shoulder_cam = c.level >= 1;
                }
                self.sim.set_local_moves(id, c.level >= 3);
                self.sim.set_rtt(id, c.rtt);
                if c.addon {
                    self.ui_line(conn, &format!("V\t{}", ui::VERSION));
                }
            }
            (None, false) if self.realm.siege => {}
            (None, false) => self.say_to(
            conn,
            &format!(
                "Welcome to Elwynn Survival. The camp by Goldshire is home: gather timber, meat and \
                 Fargodeep copper, bank it at the camp's chest, and build. {} Friendly fire is on: \
                 heroes can strike each other and get up again a second later. Try every weapon at \
                 the rack by the chest. The key icon at the bottom right lists your controls.",
                ui::CONTROLS_TEXT
            ),
        ),
            (None, true) => {}
        }
        if self.realm.siege {
            let page = self.sim.content.siege.as_ref().map_or(String::new(), |d| d.page.clone());
            self.say_to(conn, &format!("{page}. You're in Trollbane Hall: muster at the banner to join a way to play (one against one, two against two, three or four each for themselves). Every match starts the same: your class's Q and E and the plain kit; your camp's things wait at Goldshire. The banner takes you home."));
        }
        if self.realm.below {
            let own = self.sim.content.dungeon.as_ref().map(|d| d.welcome.clone()).filter(|w| !w.is_empty());
            let text = own.unwrap_or_else(|| {
                "The Deadmines. Light the lamps as you go: a lamp is where you wake, and resting at one \
                 heals you and fills your Draughts (T), the only heal down here; it also brings the \
                 Brotherhood back to their posts. Fall, and what you found waits in a sack where you \
                 fell: fall again before you win it back and it's gone. Any lit lamp takes you home."
                    .to_string()
            });
            self.say_to(conn, &text);
        }
        println!("[survival] {name} entered {}", self.realm_name());
    }

    /// A hero leaves for the other world: off this one (saved), and their connection with them.
    pub(crate) fn depart(&mut self, conn: ConnId) -> Option<(Session, PlayerSave, Carry)> {
        let w = self.sessions.get_mut(&conn).and_then(|s| s.world.take())?;
        let carry = Carry {
            addon: w.addon,
            anim: w.anim,
            local: w.local,
            level: w.level,
            rtt: w.rtt,
        };
        let save = self.sim.leave(w.id)?;
        self.guids.remove(&w.id);
        self.ids.remove(&w.guid);
        self.broadcast(&packets::destroy_object(w.guid));
        let mut session = self.sessions.remove(&conn)?;
        self.offline.remove(&session.hero);
        session.world = None;
        println!("[survival] {} left {}", w.name, self.realm_name());
        self.save();
        Some((session, save, carry))
    }

    /// A hero arrives from the other world: at a lamp below (`lamp`), or by the book at camp.
    pub(crate) fn arrive(
        &mut self,
        conn: ConnId,
        session: Session,
        save: PlayerSave,
        carry: Carry,
        lamp: Option<&str>,
    ) {
        let account = session.hero.clone();
        self.sessions.insert(conn, session);
        let name = hero_name(&account, Some(&save));
        let guid = Self::player_guid(&account);
        let id = self.sim.join(&account, &name, Some(&save));
        match lamp {
            Some(l) if self.realm.below => self.sim.arrive_at(id, l),
            _ if self.realm.siege => self.sim.siege_arrive(id),
            _ => {
                // Home by the book, beside it.
                let book = self
                    .sim
                    .objs
                    .get(&self.sim.book_id())
                    .map(|o| (o.pos, o.facing));
                if let Some((at, f)) = book {
                    let (x, y) = (
                        at[0] + (f + std::f32::consts::FRAC_PI_2).cos() * 2.5,
                        at[1] + (f + std::f32::consts::FRAC_PI_2).sin() * 2.5,
                    );
                    let z = self.sim.land().height(x, y);
                    if let Some(p) = self.sim.players.get_mut(&id) {
                        p.pos = [x, y, z];
                        p.facing = f;
                        p.last_safe = p.pos;
                    }
                }
            }
        }
        self.guids.insert(id, guid);
        self.ids.insert(guid, id);
        let (pos, facing) = {
            let p = &self.sim.players[&id];
            (p.pos, p.facing)
        };
        if let Some(s) = self.sessions.get_mut(&conn) {
            let mut w = InWorld::new(guid, id, name, pos[2]);
            w.porting = true;
            w.addon = carry.addon;
            s.world = Some(w);
        }
        self.pending_carry.insert(conn, carry);
        self.send(conn, packets::new_world(self.realm.map, pos, facing));
        self.save();
    }

    /// The client has loaded this world: draw it in.
    fn worldport_ack(&mut self, conn: ConnId) {
        let Some(w) = self.world_mut(conn) else {
            return;
        };
        if !w.porting {
            return;
        }
        w.porting = false;
        let id = w.id;
        let carry = self.pending_carry.remove(&conn);
        self.finish_entry(conn, id, carry);
        self.mark_dirty();
    }

    pub(crate) fn leave_world(&mut self, conn: ConnId) {
        let Some(w) = self.sessions.get_mut(&conn).and_then(|s| s.world.take()) else { return };
        let account = self.sessions.get(&conn).map(|s| s.hero.clone()).unwrap_or_default();
        // (Siege keeps no hero: the camp holds their save meanwhile.)
        if let Some(save) = self.sim.leave(w.id).filter(|_| !self.realm.siege) {
            self.offline.insert(account, save);
        }
        self.guids.remove(&w.id);
        self.ids.remove(&w.guid);
        self.broadcast(&packets::destroy_object(w.guid));
        println!("[survival] {} left", w.name);
        self.save();
    }

    fn name_query(&self, conn: ConnId, guid: u64) {
        let found = self
            .in_world()
            .find(|(_, _, w)| w.guid == guid)
            .and_then(|(_, _, w)| self.sim.players.get(&w.id).map(|p| (w.name.clone(), p.look, p.class.clone())))
            .or_else(|| self.offline.iter().find(|(k, _)| Self::player_guid(k) == guid).map(|(k, s)| (hero_name(k, Some(s)), s.look.unwrap_or_default(), s.class.clone())));
        if let Some((name, look, class)) = found {
            self.send(conn, packets::name_query_response(guid, &name, look.race, look.gender, htl_survival::content::class_id_of(&class)));
        }
    }

    fn movement(&mut self, conn: ConnId, opcode: u16, body: &[u8]) -> std::io::Result<()> {
        let Some(w) = self.world_of(conn).filter(|w| !w.porting) else { return Ok(()) };
        let (guid, id) = (w.guid, w.id);
        let mut r = Reader::new(body);
        let flags = r.u32()?;
        let _time = r.u32()?;
        let pos = [r.f32()?, r.f32()?, r.f32()?];
        let facing = r.f32()?;
        // MOVEFLAG_WALK_MODE (0x100): the walk toggle.
        let moving = Moving { forward: flags & 0x1 != 0, back: flags & 0x2 != 0, left: flags & 0x4 != 0, right: flags & 0x8 != 0, walk: flags & 0x100 != 0 };
        if pos.iter().all(|v| v.is_finite()) {
            self.sim.move_player(id, pos, facing, moving);
        }
        if opcode == op::MSG_MOVE_JUMP {
            self.sim.jumped(id);
        }
        // Long falls hurt, as in Basin Battle.
        if opcode == op::MSG_MOVE_FALL_LAND {
            self.sim.landed(id);
            let fall_ms = crate::game::fall_time(&mut r, flags).unwrap_or(0);
            if let Some(w) = self.world_mut(conn) {
                let top = w.fall_top.take().unwrap_or(w.ground_z).max(pos[2]);
                w.ground_z = pos[2];
                let drop = (top - pos[2]).min(crate::game::fall_distance(fall_ms as f32 / 1000.0));
                if drop > 28.0 {
                    let share = 0.018 * (drop - 14.57) - 0.2426;
                    if let Some(p) = self.sim.players.get(&id) {
                        let damage = (share * p.max_health as f32).round().max(1.0) as u32;
                        self.broadcast(&packets::environmental_damage(guid, 2, damage));
                        self.sim.fall_damage(id, damage);
                    }
                }
            }
        } else if let Some(w) = self.world_mut(conn) {
            if flags & 0x6000 != 0 {
                let top = w.fall_top.unwrap_or(w.ground_z);
                w.fall_top = Some(top.max(pos[2]));
            } else {
                w.ground_z = pos[2];
                w.fall_top = None;
            }
        }
        let mut b = Body::new();
        b.packed_guid(guid).bytes(body);
        self.broadcast_except(conn, &Packet::new(opcode, b));
        Ok(())
    }

    fn chat(&mut self, conn: ConnId, r: &mut Reader) -> std::io::Result<()> {
        let kind = r.u32()?;
        let _language = r.u32()?;
        // A channel's line, and the client's own away and busy notes (its auto-away after a
        // while without input, which isn't something to say out loud to the camp).
        if kind == 14 || kind == CHAT_MSG_AFK || kind == CHAT_MSG_DND {
            return Ok(());
        }
        if kind == 6 {
            // The add-on whispers its commands while its hero is down.
            let _target = r.cstr()?;
            let text = r.cstr()?;
            if let Some(cmd) = text.trim().strip_prefix('.') {
                self.command(conn, cmd);
            }
            return Ok(());
        }
        let text = r.cstr()?;
        let Some(w) = self.world_of(conn) else { return Ok(()) };
        let (guid, name) = (w.guid, w.name.clone());
        let trimmed = text.trim();
        if let Some(cmd) = trimmed.strip_prefix('.') {
            self.command(conn, cmd);
            return Ok(());
        }
        println!("[survival] {name}: {trimmed}");
        let out = if kind as u8 == packets::CHAT_YELL { packets::CHAT_YELL } else { packets::CHAT_SAY };
        self.broadcast(&packets::chat(out, packets::LANG_UNIVERSAL, guid, trimmed));
        Ok(())
    }
}

/// `ChatMsg` (1.12's numbering): away and busy.
const CHAT_MSG_AFK: u32 = 0x14;
const CHAT_MSG_DND: u32 = 0x15;

