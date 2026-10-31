//! Characters: an account's heroes (its first footman, if it has one, and those it made on the
//! creation screen), listed, made, deleted and logged in as, across both worlds (a hero's save is
//! the camp's or, while they're below, the Deadmines'). Each character is its own hero: its name,
//! look and class (its column's Q and E cards from the start), its gear, bags, Valor and cards.

use benilla_protocol::messages::opcode as op;
use htl_survival::save::PlayerSave;
use htl_survival::HeroLook;

use super::realms::Realms;
use super::{hero_name, SurvivalGame};
use crate::packets::{self, Packet};
use crate::wire::{Body, Reader};
use crate::world::ConnId;

/// `SMSG_CHAR_CREATE` / `SMSG_CHAR_DELETE` results (1.12 `ResponseCodes`).
const CREATE_SUCCESS: u8 = 0x2E;
const CREATE_FAILED: u8 = 0x30;
const CREATE_NAME_IN_USE: u8 = 0x31;
const CREATE_SERVER_LIMIT: u8 = 0x34;
const DELETE_SUCCESS: u8 = 0x39;
const DELETE_FAILED: u8 = 0x3A;
const NAME_TOO_SHORT: u8 = 0x46;
const NAME_TOO_LONG: u8 = 0x47;
const NAME_ONLY_LETTERS: u8 = 0x48;
/// Characters an account may hold (the select screen's own limit).
pub const MOST: usize = 10;

/// One of an account's heroes, wherever its save is.
pub(crate) struct Hero {
    pub key: String,
    pub name: String,
    pub class: String,
    pub look: HeroLook,
    pub pos: [f32; 3],
    /// The dungeon whose world it's in (its place among them), if not the camp's.
    pub below: Option<usize>,
}

/// Whose a hero is: its save's owner, else (an account's first footman) its key.
fn owner_of<'a>(key: &'a str, save: &'a PlayerSave) -> &'a str {
    if save.owner.is_empty() {
        key.split('/').next().unwrap_or(key)
    } else {
        &save.owner
    }
}

impl Realms {
    fn games(&self) -> Vec<(&SurvivalGame, Option<usize>)> {
        let mut out = vec![(&self.camp, None)];
        for (k, b) in self.below.iter().enumerate() {
            out.push((b, Some(k)));
        }
        out
    }

    /// Every hero there is (offline in either world, or in one now), with whose it is.
    fn all_heroes(&self) -> Vec<(String, Hero)> {
        let mut out: Vec<(String, Hero)> = Vec::new();
        for (g, below) in self.games() {
            for (key, s) in &g.offline {
                out.push((owner_of(key, s).to_lowercase(), Hero { key: key.clone(), name: hero_name(key, Some(s)), class: s.class.clone(), look: s.look.unwrap_or_default(), pos: s.pos, below }));
            }
            for p in g.sim.players.values() {
                let owner = if p.owner.is_empty() { p.account.split('/').next().unwrap_or(&p.account).to_string() } else { p.owner.clone() };
                out.push((owner.to_lowercase(), Hero { key: p.account.clone(), name: p.name.clone(), class: p.class.clone(), look: p.look, pos: p.pos, below }));
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        out.retain(|(_, h)| seen.insert(h.key.clone()));
        out
    }

    /// An account's heroes, its first footman first, then by name.
    pub(crate) fn heroes_of(&self, account: &str) -> Vec<Hero> {
        let account = account.to_lowercase();
        let mut mine: Vec<Hero> = self.all_heroes().into_iter().filter(|(o, _)| *o == account).map(|(_, h)| h).collect();
        mine.sort_by_key(|h| (h.key.contains('/'), h.name.to_lowercase()));
        mine
    }

    fn session_account(&self, conn: ConnId) -> Option<String> {
        self.games().into_iter().chain(self.siege.iter().map(|s| (s, None))).find_map(|(g, _)| g.sessions.get(&conn).map(|s| s.account.clone()))
    }

    /// The character list (`CMSG_CHAR_ENUM`).
    pub(crate) fn char_enum(&mut self, conn: ConnId) {
        let Some(account) = self.session_account(conn) else { return };
        let heroes = self.heroes_of(&account);
        let entries: Vec<packets::CharEntry> = heroes
            .iter()
            .map(|h| {
                let realm = h.below.and_then(|k| self.below.get(k)).map_or(self.camp.realm, |b| b.realm);
                packets::CharEntry {
                    guid: SurvivalGame::player_guid(&h.key),
                    name: &h.name,
                    race: h.look.race,
                    class: htl_survival::content::class_id_of(&h.class),
                    gender: h.look.gender,
                    appearance: h.look.bytes(),
                    level: 1,
                    zone: realm.zone,
                    map: realm.map,
                    pos: h.pos,
                }
            })
            .collect();
        let game = self.game(conn);
        game.send(conn, packets::tutorial_flags_all_seen());
        game.send(conn, packets::char_enum(&entries));
    }

    /// A character made on the creation screen (`CMSG_CHAR_CREATE`): its name (letters only, two to
    /// twelve, no one else's), race, class, sex and looks; a fresh hero of its class at the camp.
    pub(crate) fn char_create(&mut self, conn: ConnId, body: &[u8]) -> std::io::Result<()> {
        let mut r = Reader::new(body);
        let name = r.cstr()?;
        let [race, class, gender, skin, face, hair, hair_color, facial, _outfit] = r.array::<9>()?;
        let result = self.try_create(conn, &name, class, HeroLook { race, gender, skin, face, hair, hair_color, facial });
        let mut b = Body::new();
        b.u8(result);
        self.game(conn).send(conn, Packet::new(op::SMSG_CHAR_CREATE, b));
        Ok(())
    }

    fn try_create(&mut self, conn: ConnId, name: &str, class: u8, look: HeroLook) -> u8 {
        let Some(account) = self.session_account(conn).map(|a| a.to_lowercase()) else { return CREATE_FAILED };
        let n = name.chars().count();
        if n < 2 {
            return NAME_TOO_SHORT;
        }
        if n > 12 {
            return NAME_TOO_LONG;
        }
        if !name.chars().all(|c| c.is_ascii_alphabetic()) {
            return NAME_ONLY_LETTERS;
        }
        let Some(school) = htl_survival::content::school_of_class(class) else { return CREATE_FAILED };
        if !(1..=8).contains(&look.race) || look.gender > 1 {
            return CREATE_FAILED;
        }
        let lower = name.to_lowercase();
        let name = crate::game::display_name(&lower);
        // No one else's name: no hero's, and no account's first footman's.
        let taken = self.all_heroes().iter().any(|(_, h)| h.name.to_lowercase() == lower) || self.games().iter().any(|(g, _)| g.sessions.values().any(|s| s.account.to_lowercase() == lower && s.account.to_lowercase() != account));
        if taken {
            return CREATE_NAME_IN_USE;
        }
        if self.heroes_of(&account).len() >= MOST {
            return CREATE_SERVER_LIMIT;
        }
        let key = format!("{account}/{lower}");
        let save = self.camp.sim.fresh_save(&account, &name, school, look);
        self.camp.offline.insert(key, save);
        self.camp.mark_dirty();
        self.camp.save();
        println!("[survival] {account} made {name}, a {school}");
        CREATE_SUCCESS
    }

    /// A character deleted from the select screen (`CMSG_CHAR_DELETE`): one of the account's, not
    /// in a world now; gone from its save for good.
    pub(crate) fn char_delete(&mut self, conn: ConnId, body: &[u8]) -> std::io::Result<()> {
        let guid = Reader::new(body).u64()?;
        let account = self.session_account(conn).unwrap_or_default();
        let hero = self.heroes_of(&account).into_iter().find(|h| SurvivalGame::player_guid(&h.key) == guid);
        let online = |g: &SurvivalGame, key: &str| g.sim.players.values().any(|p| p.account == key);
        let mut result = DELETE_FAILED;
        if let Some(h) = hero {
            let busy = online(&self.camp, &h.key) || self.below.iter().any(|b| online(b, &h.key));
            if !busy {
                let mut gone = self.camp.offline.remove(&h.key).is_some();
                if gone {
                    self.camp.mark_dirty();
                    self.camp.save();
                }
                for b in &mut self.below {
                    if b.offline.remove(&h.key).is_some() {
                        gone = true;
                        b.mark_dirty();
                        b.save();
                    }
                }
                if gone {
                    println!("[survival] {account} deleted {}", h.name);
                    result = DELETE_SUCCESS;
                }
            }
        }
        let mut b = Body::new();
        b.u8(result);
        self.game(conn).send(conn, Packet::new(op::SMSG_CHAR_DELETE, b));
        Ok(())
    }

    /// Logging in as a character (`CMSG_PLAYER_LOGIN`): the session takes that hero, in the world
    /// their save is in. False if it's none of the account's.
    pub(crate) fn pick_hero(&mut self, conn: ConnId, guid: u64) -> bool {
        let Some(account) = self.session_account(conn) else { return false };
        let Some(h) = self.heroes_of(&account).into_iter().find(|h| SurvivalGame::player_guid(&h.key) == guid) else { return false };
        // The session into the right world.
        let here = self.in_below.get(&conn).copied();
        let wants = h.below.filter(|k| *k < self.below.len());
        if here != wants {
            let session = match here {
                Some(k) => self.below.get_mut(k).and_then(|b| b.sessions.remove(&conn)),
                None => self.camp.sessions.remove(&conn),
            };
            if let Some(s) = session {
                match wants {
                    Some(k) => {
                        self.below[k].sessions.insert(conn, s);
                    }
                    None => {
                        self.camp.sessions.insert(conn, s);
                    }
                }
            }
            match wants {
                Some(k) => self.in_below.insert(conn, k),
                None => self.in_below.remove(&conn),
            };
        }
        if let Some(s) = self.game(conn).sessions.get_mut(&conn) {
            s.hero = h.key;
        }
        true
    }
}
