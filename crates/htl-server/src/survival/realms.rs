//! The survival realm's worlds on one thread: the camp's (Elwynn), the dungeons below (the
//! Deadmines and the others, each on its own instance map) and Siege's (Arathi Basin), each a
//! [`SurvivalGame`] of its own. A connection is in
//! one of them at a time; the book at camp, the lamps below and Siege's banner move a hero (and
//! their connection) from one to another with the client's far teleport. What's done below that
//! the camp counts (its journal, trophies) is passed up, and the camp's book learns which lamps
//! are lit. A hero in Siege plays a fresh hero there: their own save stays with the camp,
//! untouched, until they're back.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use benilla_protocol::messages::opcode as op;
use crossbeam_channel::{Receiver, RecvTimeoutError};

use super::{SurvivalGame, TICK};
use crate::packets;
use crate::world::{ConnId, Inbound};

pub struct Realms {
    pub(crate) camp: SurvivalGame,
    /// The dungeons' worlds, the Deadmines' first.
    pub(crate) below: Vec<SurvivalGame>,
    pub(crate) siege: Option<SurvivalGame>,
    /// Which dungeon's world each connection is in (by its place in `below`); absent: the camp.
    pub(crate) in_below: HashMap<ConnId, usize>,
    /// The connections in Siege's world.
    pub(crate) in_siege: HashSet<ConnId>,
    /// The lamps the camp's book was last told of.
    pages: Vec<String>,
    /// The Bookmaker's Board.
    pub(crate) board: Option<super::board::Shared>,
}

impl Realms {
    pub fn new(camp: SurvivalGame, below: Vec<SurvivalGame>) -> Self {
        Self::with_siege(camp, below, None)
    }

    pub fn with_siege(camp: SurvivalGame, below: Vec<SurvivalGame>, siege: Option<SurvivalGame>) -> Self {
        let mut r = Self {
            camp,
            below,
            siege,
            in_below: HashMap::new(),
            in_siege: HashSet::new(),
            pages: Vec::new(),
            board: None,
        };
        // An account kept in two saves (a restart mid-crossing): it's below.
        for b in &r.below {
            let below_accounts: Vec<String> = b.offline.keys().cloned().collect();
            for a in below_accounts {
                r.camp.offline.remove(&a);
            }
        }
        r.sync_pages();
        r
    }

    /// The Bookmaker's Board, for the camp's pages and the wagers won below.
    pub fn set_board(&mut self, board: Option<super::board::Shared>) {
        self.camp.board = board.clone();
        // (Below too, for the board's and the ladder's pages.)
        for b in &mut self.below {
            b.board = board.clone();
        }
        self.board = board;
    }

    pub fn run(mut self, inbound: Receiver<Inbound>) {
        let mut next = Instant::now() + TICK;
        loop {
            let now = Instant::now();
            if now >= next {
                self.camp.tick();
                for b in &mut self.below {
                    b.tick();
                }
                if let Some(s) = self.siege.as_mut() {
                    s.tick();
                }
                self.exchange();
                next += TICK;
                if next < now {
                    next = now + TICK;
                }
                continue;
            }
            match inbound.recv_timeout(next - now) {
                Ok(msg) => self.handle(msg),
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => {
                    self.camp.save();
                    for b in &mut self.below {
                        b.save();
                    }
                    return;
                }
            }
        }
    }

    pub(crate) fn game(&mut self, conn: ConnId) -> &mut SurvivalGame {
        if self.in_siege.contains(&conn) {
            if let Some(s) = self.siege.as_mut() {
                return s;
            }
        }
        match self.in_below.get(&conn).copied() {
            Some(k) if k < self.below.len() => &mut self.below[k],
            _ => &mut self.camp,
        }
    }

    fn handle(&mut self, msg: Inbound) {
        match &msg {
            Inbound::Connected { conn, .. } => {
                self.in_below.remove(conn);
                self.camp.handle(msg);
            }
            Inbound::Disconnected { conn } => {
                let conn = *conn;
                self.game(conn).handle(msg);
                self.in_below.remove(&conn);
                self.in_siege.remove(&conn);
            }
            Inbound::Packet { conn, opcode, body } => {
                let conn = *conn;
                // Before a hero is in a world: the account's characters, listed, made and
                // deleted here (they're in either world's save), and logging in as one puts the
                // connection in the world that hero is in.
                if self.game(conn).world_of(conn).is_none() {
                    let Ok(code) = u16::try_from(*opcode) else { return };
                    let done = match code {
                        op::CMSG_CHAR_ENUM => {
                            self.char_enum(conn);
                            Ok(true)
                        }
                        op::CMSG_CHAR_CREATE => self.char_create(conn, body).map(|_| true),
                        op::CMSG_CHAR_DELETE => self.char_delete(conn, body).map(|_| true),
                        op::CMSG_PLAYER_LOGIN => {
                            if let Ok(guid) = crate::wire::Reader::new(body).u64() {
                                self.pick_hero(conn, guid);
                            }
                            Ok(false)
                        }
                        _ => Ok(false),
                    };
                    match done {
                        Ok(true) => return,
                        Ok(false) => {}
                        Err(e) => {
                            eprintln!("[survival] bad character packet {code:#x}: {e}");
                            return;
                        }
                    }
                }
                self.game(conn).handle(msg);
                // Logged out of Siege's world to the character list: the connection is the
                // camp's again (the hero's save is there).
                if self.in_siege.contains(&conn) {
                    if let Some(s) = self.siege.as_mut() {
                        if s.sessions.get(&conn).is_some_and(|x| x.world.is_none()) {
                            if let Some(session) = s.sessions.remove(&conn) {
                                self.camp.sessions.insert(conn, session);
                            }
                            self.in_siege.remove(&conn);
                        }
                    }
                }
            }
        }
    }

    /// After a tick: heroes crossing between the worlds, what's done below counted at camp, and
    /// the book's pages.
    fn exchange(&mut self) {
        self.exchange_siege();
        let down: Vec<(ConnId, Option<String>)> = std::mem::take(&mut self.camp.transfers);
        for (conn, lamp) in down {
            let Some(lamp) = lamp else { continue };
            if self.in_below.contains_key(&conn) {
                continue;
            }
            // A wager: `lamp@level`.
            let (lamp, level) = match lamp.split_once('@') {
                Some((l, n)) => (l.to_string(), n.parse::<u32>().ok()),
                None => (lamp, None),
            };
            // The dungeon whose page it is.
            let Some(k) = self.below.iter().position(|b| b.sim.content.dungeon.as_ref().is_some_and(|d| d.lamps.iter().any(|l| l.key == lamp))) else {
                self.camp.say_to(conn, "That place couldn't be read from the game data on this server.");
                continue;
            };
            let below = &mut self.below[k];
            let name = below.sim.content.dungeon.as_ref().map_or(String::new(), |d| d.name.clone());
            let busy = !below.sim.players.is_empty();
            let wager = below.sim.wager().map(|w| (w.level, w.done.is_some()));
            match (level, wager) {
                // A wager on an empty dungeon starts it fresh; one under way at that level takes
                // whoever joins.
                (Some(n), Some((at, false))) if busy && at == n => {}
                (Some(n), _) if !busy => below.sim.start_wager(n),
                (Some(_), Some((at, false))) => {
                    self.camp.say_to(conn, &format!("A wager is under way in {name} at +{at}: join that one, or wait for it to end."));
                    continue;
                }
                (Some(_), _) => {
                    self.camp.say_to(conn, &format!("Heroes are down in {name} already: a wager starts on an empty dungeon."));
                    continue;
                }
                (None, Some((at, _))) if busy => {
                    self.camp.say_to(conn, &format!("{name} is the Bookmaker's for now: a wager at +{at} is on. Join it from the book's wager page."));
                    continue;
                }
                (None, _) => {}
            }
            self.camp.send(conn, packets::transfer_pending(below.realm.map));
            if let Some((session, save, carry)) = self.camp.depart(conn) {
                below.arrive(conn, session, save, carry, Some(&lamp));
                self.in_below.insert(conn, k);
            }
        }
        for k in 0..self.below.len() {
            let up: Vec<(ConnId, Option<String>)> = std::mem::take(&mut self.below[k].transfers);
            for (conn, _) in up {
                if self.in_below.get(&conn) != Some(&k) {
                    continue;
                }
                let below = &mut self.below[k];
                below.send(conn, packets::transfer_pending(self.camp.realm.map));
                if let Some((session, save, carry)) = below.depart(conn) {
                    self.camp.arrive(conn, session, save, carry, None);
                    self.in_below.remove(&conn);
                }
            }
            // Wagers won, onto the board (and to the board host, from a server that isn't it).
            let won = std::mem::take(&mut self.below[k].wagers);
            if let Some(b) = &self.board {
                for w in &won {
                    let run = b.lock().unwrap().record(w);
                    println!("[survival] wager: {} +{} in {} ms ({} falls, {}) by {}", w.name, w.level, w.ms, w.deaths, if w.timed { "in time" } else { "over time" }, run.heroes());
                }
            }
            let news = std::mem::take(&mut self.below[k].to_camp);
            for (counts, items) in news {
                self.camp.sim.from_below(&counts, &items);
            }
        }
        self.sync_pages();
    }

    /// Heroes crossing to Siege (its page in the book at camp) and back (its banner): there, a
    /// fresh hero of their class; their own save kept at camp meanwhile, and theirs again home.
    fn exchange_siege(&mut self) {
        let page = htl_survival::sim::siege::SIEGE_PAGE;
        let (to_siege, rest): (Vec<(ConnId, Option<String>)>, Vec<(ConnId, Option<String>)>) = std::mem::take(&mut self.camp.transfers).into_iter().partition(|(_, to)| to.as_deref() == Some(page));
        self.camp.transfers = rest;
        let Some(siege) = self.siege.as_mut() else {
            for (conn, _) in to_siege {
                self.camp.say_to(conn, "Siege isn't open on this server (Arathi Basin couldn't be read).");
            }
            return;
        };
        for (conn, _) in to_siege {
            if self.in_siege.contains(&conn) || self.in_below.contains_key(&conn) {
                continue;
            }
            self.camp.send(conn, packets::transfer_pending(siege.realm.map));
            if let Some((session, save, carry)) = self.camp.depart(conn) {
                // Theirs, kept at camp for when they're back.
                let key = session.hero.clone();
                let fresh = siege.sim.fresh_save(&save.owner, &save.name, &save.class, save.look.unwrap_or_default());
                self.camp.offline.insert(key, save);
                self.camp.mark_dirty();
                self.camp.save();
                siege.arrive(conn, session, fresh, carry, None);
                self.in_siege.insert(conn);
            }
        }
        let back: Vec<(ConnId, Option<String>)> = std::mem::take(&mut siege.transfers);
        for (conn, _) in back {
            if !self.in_siege.contains(&conn) {
                continue;
            }
            siege.send(conn, packets::transfer_pending(self.camp.realm.map));
            if let Some((session, played, carry)) = siege.depart(conn) {
                let save = self.camp.offline.remove(&session.hero).unwrap_or(played);
                self.camp.arrive(conn, session, save, carry, None);
                self.in_siege.remove(&conn);
                self.in_below.remove(&conn);
            }
        }
    }

    fn sync_pages(&mut self) {
        // The camp started over: the dungeons with it (their saves kept aside first).
        if std::mem::take(&mut self.camp.reset_below) {
            for b in &mut self.below {
                match b.keep_save_aside() {
                    Ok(kept) => {
                        if let Some(to) = kept {
                            println!("[survival] {} started over; its old save is at {}", b.realm_name(), to.display());
                        }
                        b.sim.reset_below(true);
                        b.mark_dirty();
                        b.save();
                    }
                    Err(e) => eprintln!("[survival] {e}"),
                }
            }
        }
        // The journal below is the camp's.
        for b in &mut self.below {
            b.sim.mirror_journal(self.camp.sim.camp.goal, &self.camp.sim.camp.counts);
        }
        let lit: Vec<String> = self.below.iter().flat_map(|b| b.sim.lit_lamps().into_iter().map(|(k, _)| k)).collect();
        if lit != self.pages {
            self.pages = lit.clone();
            self.camp.sim.set_pages(lit);
        }
    }
}
