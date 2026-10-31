//! Siege's lines to the add-on and its commands:
//! - `SG\t<phase>\t<wave>\t<waves>\t<secs>\t<gold>\t<income>\t<camp...>^...\t<coming name~n^...>\t<result>\t<ready>~<of>~<mine>\t<sent>~<cap>`:
//!   the match in the corner (`SG` alone: not in one); each camp
//!   `name~hearth~most~out~mine~sent to~heroes (comma-separated)`; `phase` is `countdown`, `running`
//!   or `over`.
//! - `SL\t<open>\t<status>\t<secs>\t<way...>^...`: the hall's muster banner, each way
//!   `key~name~text~seats~heroes (comma-separated)~mine` (`open` 0: only a refresh of the window if
//!   it's up).
//! - `SB\t<open>\t<gold>\t<income>\t<sent to>\t<wave>\t<send...>^...\t<sent>~<cap>`: a camp's
//!   Mercenary Ledger, each send `key~name~cost~income~from~can~why~text~mob`; the creatures sent
//!   for the next wave of the most that may be.
//!
//! Commands: `.siege join <way>`, `.siege leave`, `.siege start`, `.siege send <send>`,
//! `.siege target [camp]`, `.siege ready` (or F at your Hearth), `.siege` (how it stands);
//! `.es travel camp` home.

use super::play::clean;
use super::SurvivalGame;
use crate::world::ConnId;
use htl_survival::{Id, ObjKind};

impl SurvivalGame {
    pub(super) fn siege_line(&self, id: Id) -> String {
        let Some(h) = self.sim.siege_hud(id) else { return "SG".to_string() };
        let camps: Vec<String> = h
            .camps
            .iter()
            .map(|c| format!("{}~{}~{}~{}~{}~{}~{}", clean(&c.name), c.hearth, c.max, u8::from(c.out), u8::from(c.mine), u8::from(c.target), clean(&c.heroes.join(", "))))
            .collect();
        let coming: Vec<String> = h.coming.iter().map(|(n, k)| format!("{}~{k}", clean(n))).collect();
        let (ready, of, mine) = h.ready;
        format!(
            "SG\t{}\t{}\t{}\t{:.0}\t{}\t{}\t{}\t{}\t{}\t{ready}~{of}~{}\t{}~{}",
            h.phase,
            h.wave,
            h.waves,
            h.secs.ceil(),
            h.gold,
            h.income,
            camps.join("^"),
            coming.join("^"),
            clean(&h.result),
            u8::from(mine),
            h.sent.0,
            h.sent.1
        )
    }

    /// The hall's muster banner: the ways to play and who waits for each.
    pub(super) fn ui_muster(&mut self, conn: ConnId, id: Id, open: bool) {
        let modes = self.sim.siege_modes(id);
        let (status, secs) = self.sim.siege_status();
        if !self.has_addon(conn) {
            if !open {
                return;
            }
            let list: Vec<String> = modes.iter().map(|m| format!(".siege join {} ({}, {} of {})", m.key, m.name, m.heroes.len(), m.seats)).collect();
            self.say_to(conn, &format!("The Siege Muster: {status}. {}; .siege start begins yours with whoever's waiting; .es travel camp goes home.", list.join("; ")));
            return;
        }
        let ways: Vec<String> = modes
            .iter()
            .map(|m| format!("{}~{}~{}~{}~{}~{}", clean(&m.key), clean(&m.name), clean(&m.text), m.seats, clean(&m.heroes.join(", ")), u8::from(m.mine)))
            .collect();
        self.ui_line(conn, &format!("SL\t{}\t{}\t{secs:.0}\t{}", u8::from(open), clean(&status), ways.join("^")));
    }

    /// A camp's Mercenary Ledger: every send, what it costs and adds, and why not now.
    pub(super) fn ui_ledger(&mut self, conn: ConnId, id: Id, open: bool) {
        let sends = self.sim.siege_ledger(id);
        let gold = self.sim.gold_of(id);
        let income = self.sim.siege.as_ref().and_then(|s| s.income.get(&id)).copied().unwrap_or(0);
        let wave = self.sim.siege.as_ref().map_or(0, |s| s.wave);
        let to = self.sim.siege_target_name(id).unwrap_or_default();
        if !self.has_addon(conn) {
            if !open {
                return;
            }
            let list: Vec<String> = sends.iter().map(|s| format!(".siege send {} ({}, {} gold, +{} income)", s.key, s.name, s.cost, s.income)).collect();
            self.say_to(conn, &format!("Mercenary Ledger: {gold} gold, {income} a wave; sends go to {to} (.siege target changes it). {}", list.join("; ")));
            return;
        }
        let list: Vec<String> = sends
            .iter()
            .map(|s| format!("{}~{}~{}~{}~{}~{}~{}~{}~{}", clean(&s.key), clean(&s.name), s.cost, s.income, s.from, u8::from(s.why.is_none()), clean(s.why.as_deref().unwrap_or("")), clean(&s.text), clean(&s.icon_mob)))
            .collect();
        let (sent, cap) = self.sim.siege_hud(id).map_or((0, 0), |h| h.sent);
        self.ui_line(conn, &format!("SB\t{}\t{gold}\t{income}\t{}\t{wave}\t{}\t{sent}~{cap}", u8::from(open), clean(&to), list.join("^")));
    }

    /// A camp's Battle Trainer: the hero's class's other cards, for gold (a station's window: its
    /// rows bought with `.es craft <card>`, which here learns it).
    pub(super) fn ui_trainer(&mut self, conn: ConnId, id: Id, station: Id) {
        let cards = self.sim.siege_trainer(id);
        if !self.has_addon(conn) {
            let list: Vec<String> = cards.iter().map(|(k, n, p, ..)| format!(".es craft {k} ({n}, {p} gold)")).collect();
            self.say_to(conn, &format!("Battle Trainer: {}", if list.is_empty() { "you know every card of your class.".to_string() } else { list.join("; ") }));
            return;
        }
        let rows: Vec<String> = cards
            .iter()
            .map(|(key, name, price, can, text, icon)| {
                let cost = self.cost_detail(id, &std::collections::BTreeMap::from([("gold".to_string(), *price)]));
                format!("{}~{}~{}~{}~{}~{}", clean(key), clean(name), cost, u8::from(*can), clean(text), clean(icon))
            })
            .collect();
        self.ui_line(conn, &format!("W\t{station}\tBattle Trainer\t{}\tAbility_Warrior_BattleShout", rows.join("^")));
    }

    /// Whether `station` is one of Siege's windows; if so, shown.
    pub(super) fn ui_siege_station(&mut self, conn: ConnId, id: Id, station: Id, open: bool) -> bool {
        match self.sim.objs.get(&station).map(|o| o.kind.clone()) {
            Some(ObjKind::SiegeBanner) => {
                self.ui_muster(conn, id, open);
                true
            }
            Some(ObjKind::SiegeLedger { .. }) => {
                self.ui_ledger(conn, id, open);
                true
            }
            Some(ObjKind::SiegeTrainer { .. }) => {
                if open {
                    self.ui_trainer(conn, id, station);
                }
                true
            }
            _ => false,
        }
    }

    /// `.siege ...`
    pub(super) fn siege_command(&mut self, conn: ConnId, me: Id, arg: &str, arg2: &str) -> Result<(), String> {
        if !self.sim.is_siege() {
            return Err("Siege is played in Arathi Basin: read its page in the book at camp.".into());
        }
        let said = match arg {
            "join" => self.sim.siege_join(me, &arg2.to_lowercase())?,
            "leave" | "out" => self.sim.siege_leave_queue(me)?,
            "start" | "now" => self.sim.siege_start_now(me)?,
            "send" => self.sim.siege_send(me, &arg2.to_lowercase())?,
            "target" | "to" => self.sim.siege_target(me, (!arg2.is_empty()).then_some(arg2))?,
            "ready" => self.sim.siege_ready(me)?,
            "" | "status" => {
                let (status, secs) = self.sim.siege_status();
                format!("{status}{}", if secs > 0.0 { format!(" ({secs:.0} s)") } else { String::new() })
            }
            other => self.sim.siege_join(me, other)?,
        };
        self.tell(conn, &said);
        // The window it came from, as it stands now.
        if let Some(st) = self.world_of(conn).and_then(|w| w.station) {
            self.ui_siege_station(conn, me, st, false);
        }
        let line = self.siege_line(me);
        self.ui_line_changed(conn, "SG", line);
        Ok(())
    }
}
