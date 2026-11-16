//! A hero's melee, counted per fight and written to the host log as one line when the fight ends
//! (all foes dead, the hero fell, or left), so "did I get my fourth blow out?" has an answer.
//!
//! Plain integers on the hero, nothing per swing. A fight here is a run of melee with no gap of
//! `player.combat_secs` and no creature still on the hero (as the damage meter's is). The tally
//! begins with the first swing, guard or dodge and is cleared when its line is written.
//!
//! The light chain: blow 1 starts a chain, each next blow within `combat.combo_grace` of the last
//! one's end carries it on, the set's last blow (the finisher) completes it. A chain is dropped
//! when it ends short of that: the grace ran out, a broken guard or a knockdown staggered the hero
//! mid-chain (a blow that lands doesn't stop a swing), or a heavy attack, a roll, a leap, a weapon
//! art or a feint (the guard raised in a wind-up) replaced the next blow.
//!
//! The hero sees the chain too: each blow's place in it goes out with its swing (`Strike::chain`,
//! the `AN` line) for the add-on's pips under the reticle (`Chain.lua`), and the finisher flashes
//! on the swinger as it comes, struck or not ([`Survival::finisher_cue`]).

use super::{Action, Id, StrikeKind, Survival};
use crate::sim::Outcome;

/// The most blows a light chain's tally keeps (a longer set folds into the last).
const MOST_BLOWS: usize = 8;

/// Why a light chain ended short of its finisher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChainEnd {
    /// The next blow didn't come in time (or the fight ended first).
    TimedOut,
    /// A broken guard or a knockdown staggered the hero mid-chain.
    Hurt,
    /// A heavy attack, a roll, a leap, a weapon art or a feint came instead.
    Switched,
}

/// One hero's melee numbers for the fight they're in.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MeleeTally {
    /// Something has counted since the last line.
    pub live: bool,
    /// When last something counted.
    pub last: f32,
    pub light: u32,
    pub heavy: u32,
    /// Running attacks, rolls and leaps.
    pub other: u32,
    /// How many times the light chain reached blow `i + 1`.
    pub reached: [u32; MOST_BLOWS],
    /// The longest light chain the weapons in hand had.
    pub chain_len: u8,
    /// Blows done in the chain now going (0: none).
    pub open: u8,
    /// A stagger came while a chain was going.
    pub hurt: bool,
    pub timed_out: u32,
    pub hurt_ends: u32,
    pub switched: u32,
    /// Swings that struck someone, and swings that passed through nothing.
    pub hits: u32,
    pub whiffs: u32,
    pub parries: u32,
    pub guards: u32,
    pub broken: u32,
    pub dodges: u32,
}

impl MeleeTally {
    pub fn swings(&self) -> u32 {
        self.light + self.heavy + self.other
    }

    /// Chains that ended short of the finisher.
    pub fn dropped(&self) -> u32 {
        self.timed_out + self.hurt_ends + self.switched
    }

    fn touched(&self) -> bool {
        self.swings() + self.parries + self.guards + self.broken + self.dodges > 0
    }

    fn end_chain(&mut self, why: ChainEnd) {
        if self.open == 0 {
            return;
        }
        self.open = 0;
        let why = if std::mem::take(&mut self.hurt) { ChainEnd::Hurt } else { why };
        match why {
            ChainEnd::TimedOut => self.timed_out += 1,
            ChainEnd::Hurt => self.hurt_ends += 1,
            ChainEnd::Switched => self.switched += 1,
        }
    }

    /// The one line for the log, `None` if nothing was swung, guarded or dodged.
    pub fn line(&self, name: &str) -> Option<String> {
        if !self.touched() {
            return None;
        }
        let mut s = format!("{name}'s fight: {} swings (light {}, heavy {}", self.swings(), self.light, self.heavy);
        if self.other > 0 {
            s += &format!(", other {}", self.other);
        }
        s += ")";
        let len = (self.chain_len as usize).min(MOST_BLOWS);
        if len >= 2 && self.light > 0 {
            s += &format!("; light chain started {}x", self.reached[0]);
            for i in 1..len {
                let tag = if i + 1 == len { " (finisher)" } else { "" };
                s += &format!(", blow {}{tag}: {}x", i + 1, self.reached[i]);
            }
            s += &format!("; chains dropped {} (timed out {}, hurt {}, switched {})", self.dropped(), self.timed_out, self.hurt_ends, self.switched);
        }
        s += &format!("; {} hits, {} whiffed; parries {}, guards {}", self.hits, self.whiffs, self.parries, self.guards);
        if self.broken > 0 {
            s += &format!(" ({} broken)", self.broken);
        }
        s += &format!(", dodges {}", self.dodges);
        Some(s)
    }
}

impl Survival {
    fn melee_mut(&mut self, id: Id) -> Option<&mut MeleeTally> {
        let now = self.clock;
        let t = &mut self.players.get_mut(&id)?.melee;
        t.live = true;
        t.last = now;
        Some(t)
    }

    /// A swing of `kind` starts: the `step`th blow of a set `set_len` long.
    pub(crate) fn melee_swing(&mut self, id: Id, kind: StrikeKind, step: u8, set_len: u8) {
        let Some(t) = self.melee_mut(id) else { return };
        match kind {
            StrikeKind::Light => {
                t.light += 1;
                if set_len >= 2 {
                    t.chain_len = t.chain_len.max(set_len);
                    let i = (step as usize).min(MOST_BLOWS - 1);
                    if step == 0 {
                        t.end_chain(ChainEnd::TimedOut);
                        t.open = 1;
                    } else if t.open > 0 {
                        t.open = step + 1;
                    }
                    t.reached[i] += 1;
                    if step + 1 >= set_len {
                        t.open = 0;
                        t.hurt = false;
                    }
                }
            }
            StrikeKind::Off => t.light += 1,
            StrikeKind::Heavy => {
                t.heavy += 1;
                t.end_chain(ChainEnd::Switched);
            }
            _ => {
                t.other += 1;
                t.end_chain(ChainEnd::Switched);
            }
        }
    }

    /// The chain's last blow comes (its active moment begins), struck or not: its flash and sound
    /// on the swinger, for everyone near. The swinger's own add-on plays it at the impact as the
    /// clip crosses it (`Chain.lua`), so the server leaves that client out (`play.rs`).
    pub(crate) fn finisher_cue(&mut self, id: Id) {
        self.events.push(super::Event::Visual { id, kit: super::combat::KIT_FINISHER, by: Some(id) });
    }

    /// How long a chain stays open after its blow ends: the light chain's grace, and a two-blade
    /// run's `DUAL_WINDOW` past that (what the hero's chain cue counts down).
    pub fn chain_window(&self, run: bool) -> f32 {
        self.content.combat.combo_grace + if run { super::DUAL_WINDOW } else { 0.0 }
    }

    /// Something other than a light blow takes the place of the chain's next (a weapon art, a feint).
    pub(crate) fn melee_switched(&mut self, id: Id) {
        if let Some(t) = self.players.get_mut(&id).map(|p| &mut p.melee) {
            t.end_chain(ChainEnd::Switched);
        }
    }

    /// A swing struck someone (the first moment it did), or its active moment ended on nothing.
    pub(crate) fn melee_struck(&mut self, id: Id, hit: bool) {
        if let Some(t) = self.melee_mut(id) {
            if hit {
                t.hits += 1;
            } else {
                t.whiffs += 1;
            }
        }
    }

    pub(crate) fn melee_dodged(&mut self, id: Id) {
        if let Some(t) = self.melee_mut(id) {
            t.dodges += 1;
        }
    }

    /// A staggered hero's chain is broken for good unless it carries on after the stagger: the
    /// chain's end, if it comes, is theirs to be hurt.
    pub(crate) fn melee_staggered(&mut self, id: Id) {
        if let Some(p) = self.players.get_mut(&id) {
            if matches!(p.action, Action::Swing { .. }) && p.melee.open > 0 {
                p.melee.hurt = true;
            }
        }
    }

    /// A creature's blow met hero `id`'s defence.
    pub(crate) fn melee_defended(&mut self, id: Id, outcome: Outcome) {
        if !matches!(outcome, Outcome::Parried | Outcome::Blocked | Outcome::GuardBroken) {
            return;
        }
        if let Some(t) = self.melee_mut(id) {
            match outcome {
                Outcome::Parried => t.parries += 1,
                Outcome::Blocked => t.guards += 1,
                _ => t.broken += 1,
            }
        }
    }

    /// Write hero `id`'s line and clear the tally.
    pub(crate) fn melee_flush(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id) else { return };
        if !p.melee.live {
            return;
        }
        p.melee.end_chain(ChainEnd::TimedOut);
        let tally = std::mem::take(&mut p.melee);
        let name = p.name.clone();
        if let Some(line) = tally.line(&name) {
            self.events.push(super::Event::Log(line));
        }
    }

    /// Each step: a fight that has gone quiet (a lull, and no creature on the hero) is over.
    pub(crate) fn melee_check(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        if p.melee.live && self.clock - p.melee.last >= self.fight_gap() && !self.engaged(id) && !self.fight_on(id) {
            self.melee_flush(id);
        }
    }
}
