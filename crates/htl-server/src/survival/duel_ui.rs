//! Duels on the wire: WoW's own. The client casts the Duel spell at a hero (their portrait's Duel,
//! or `/duel`), the one challenged answers the client's popup (`CMSG_DUEL_ACCEPTED` or
//! `CMSG_DUEL_CANCELLED`, which is also `/forfeit`), and the server tells both the countdown, the
//! bounds and the end, and everyone the winner. Each duellist's `PLAYER_DUEL_ARBITER` (the flag)
//! and `PLAYER_DUEL_TEAM` make the other a foe on the client too (their name red, their bars a
//! foe's). `.es duel <name>`, `.es duel accept` and `.es duel forfeit` do the same by typing.

use htl_survival::sim::duel::DuelNews;
use htl_survival::Id;

use super::{ConnId, SurvivalGame};
use crate::fields::{idx, Fields};
use crate::packets;

impl SurvivalGame {
    /// A spell cast from the client's book (`CMSG_CAST_SPELL`): the Duel spell at a hero. Every
    /// cast is answered, so the client's cast doesn't hang.
    pub(crate) fn cast_from_book(&mut self, conn: ConnId, me: Id, spell: u32, target: Option<u64>) {
        if spell != u32::from(packets::DUEL_SPELL) {
            self.send(conn, packets::cast_result(spell, Some(packets::SPELL_FAILED_BAD_TARGETS)));
            return;
        }
        let them = target.and_then(|g| self.ids.get(&g).copied()).filter(|t| self.sim.players.contains_key(t));
        let done = match them {
            Some(t) => self.sim.duel_challenge(me, t),
            None => Err("Only another hero can be challenged to a duel.".into()),
        };
        self.send(conn, packets::cast_result(spell, done.is_err().then_some(packets::SPELL_FAILED_BAD_TARGETS)));
        if let Err(e) = done {
            self.say_to(conn, &e);
        }
    }

    /// `.es duel <name>` (a challenge), `.es duel accept`, `.es duel decline` or `.es duel forfeit`.
    pub(crate) fn duel_command(&mut self, me: Id, arg: &str) -> Result<(), String> {
        match arg {
            "accept" | "yes" => {
                if !self.sim.duel_of(me).is_some_and(|d| d.b == me) {
                    return Err("Nobody has challenged you to a duel.".into());
                }
                self.sim.duel_accept(me);
                Ok(())
            }
            "decline" | "no" | "forfeit" | "cancel" => {
                if self.sim.duel_of(me).is_none() {
                    return Err("You aren't in a duel.".into());
                }
                self.sim.duel_cancel(me);
                Ok(())
            }
            "" => Err("Usage: .es duel <name>, .es duel accept, .es duel forfeit.".into()),
            name => {
                let them = self.sim.players.iter().find(|(_, p)| p.name.eq_ignore_ascii_case(name)).map(|(id, _)| *id);
                match them {
                    Some(t) => self.sim.duel_challenge(me, t),
                    None => Err(format!("No hero named {name} is here.")),
                }
            }
        }
    }

    /// A hero's duel fields as their body is made for a client.
    pub(crate) fn duel_fields(&self, id: Id, f: &mut Fields) {
        if let Some((flag, side)) = self.sim.duel_side(id) {
            if let Some(g) = self.guid_of(flag) {
                f.set_u64(idx::PLAYER_DUEL_ARBITER, g).set(idx::PLAYER_DUEL_TEAM, side);
            }
        }
    }

    /// Both duellists' duel fields as they stand now, to everyone.
    fn send_duel_fields(&self, ids: [Id; 2]) {
        for id in ids {
            let Some(g) = self.guid_of(id) else { continue };
            let mut f = Fields::new();
            match self.sim.duel_side(id).and_then(|(flag, side)| Some((self.guid_of(flag)?, side))) {
                Some((flag, side)) => f.set_u64(idx::PLAYER_DUEL_ARBITER, flag).set(idx::PLAYER_DUEL_TEAM, side),
                None => f.set_u64(idx::PLAYER_DUEL_ARBITER, 0).set(idx::PLAYER_DUEL_TEAM, 0),
            };
            self.broadcast(&packets::update_object(&[], &[(g, &f)]));
        }
    }

    pub(crate) fn duel_news(&mut self, a: Id, b: Id, news: DuelNews) {
        match news {
            DuelNews::Asked => {
                let flag = self.sim.duel_of(a).and_then(|d| self.guid_of(d.flag)).unwrap_or(0);
                let challenger = self.guid_of(a).unwrap_or(0);
                for id in [a, b] {
                    self.to_player(id, packets::duel_requested(flag, challenger));
                }
                self.send_duel_fields([a, b]);
                // A hero with nobody playing them (a lab's stand-in) takes it at once.
                if self.conn_of(b).is_none() {
                    self.sim.duel_accept(b);
                }
            }
            DuelNews::Counting { secs } => {
                for id in [a, b] {
                    self.to_player(id, packets::duel_countdown((secs * 1000.0).round() as u32));
                }
            }
            DuelNews::Fighting => self.send_duel_fields([a, b]),
            DuelNews::Bounds { who, out, .. } => self.to_player(who, packets::duel_bounds(out)),
            DuelNews::Ended { started, winner, fled, ref names } => {
                for id in [a, b] {
                    self.to_player(id, packets::duel_complete(started));
                }
                self.send_duel_fields([a, b]);
                if let Some(w) = winner {
                    let (won, lost) = if w == a { (&names[0], &names[1]) } else { (&names[1], &names[0]) };
                    self.broadcast(&packets::duel_winner(fled, won, lost));
                }
            }
        }
    }
}
