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

/// What one ability did in the fight (keyed by the name the damage meter uses).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AbilityTally {
    pub name: String,
    pub casts: u32,
    /// Damage events it dealt (a tick or a blow each), the distinct foes they struck, and the total.
    pub hits: u32,
    pub foes: Vec<Id>,
    pub damage: u64,
}

/// The most abilities a fight's line lists.
const MOST_ABILITIES: usize = 12;

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
    /// Of those dodges, the ones that slipped a real blow (once per dodge, however many blows it
    /// slipped), and the ones that did so in the perfect moment (a Rogue's: `dodged_perfectly`).
    pub evaded: u32,
    pub perfect_dodges: u32,
    /// The same three for the dodges that were a Quickstep's (a dodge ability); `dodges`,
    /// `evaded` and `perfect_dodges` count every kind, so a plain roll's are the difference.
    pub quick_dodges: u32,
    pub quick_evaded: u32,
    pub quick_perfect: u32,
    /// Parries made with a ward (a staff's perfect ward).
    pub perfect_parries: u32,
    /// The dodge now going: a Quickstep's, and what it has already counted.
    pub dodge_quick: bool,
    pub dodge_evaded: bool,
    pub dodge_perfect: bool,
    /// Evasion: casts, real blows that landed while it was up, real blows dodged while it was up,
    /// and of those the perfect ones its wider window made perfect (a plain dodge's would not).
    pub evasion_casts: u32,
    pub evasion_landed: u32,
    pub evasion_dodged: u32,
    pub evasion_perfect: u32,
    /// Each ability cast or dealing damage (not Melee, which has its own numbers), in the order seen.
    pub abilities: Vec<AbilityTally>,
    /// Damage dealt and taken in the fight (what landed), and the hero's lowest health with their
    /// most then.
    pub dealt: u64,
    pub taken: u64,
    pub lowest: Option<(u32, u32)>,
}

impl MeleeTally {
    /// This ability's row, made when first seen (none past the most).
    fn ability(&mut self, name: &str) -> Option<&mut AbilityTally> {
        if let Some(i) = self.abilities.iter().position(|a| a.name == name) {
            return self.abilities.get_mut(i);
        }
        if self.abilities.len() >= MOST_ABILITIES {
            return None;
        }
        self.abilities.push(AbilityTally { name: name.to_string(), ..AbilityTally::default() });
        self.abilities.last_mut()
    }

    pub fn swings(&self) -> u32 {
        self.light + self.heavy + self.other
    }

    /// Chains that ended short of the finisher.
    pub fn dropped(&self) -> u32 {
        self.timed_out + self.hurt_ends + self.switched
    }

    fn touched(&self) -> bool {
        self.swings() + self.parries + self.guards + self.broken + self.dodges + self.evasion_casts > 0 || !self.abilities.is_empty() || self.dealt + self.taken > 0
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
        s += &format!("; {} hits, {} whiffed; parries {} (perfect {}), guards {}", self.hits, self.whiffs, self.parries, self.perfect_parries, self.guards);
        if self.broken > 0 {
            s += &format!(" ({} broken)", self.broken);
        }
        s += &format!(", dodges {} (perfect {})", self.dodges, self.perfect_dodges);
        s += &format!("; dealt {}, taken {}", self.dealt, self.taken);
        if let Some((low, max)) = self.lowest {
            s += &format!(", lowest health {low}/{max}");
        }
        Some(s)
    }

    /// The dodges' line: each kind's dodges, those that slipped a blow and those that were
    /// perfect, then Evasion. `None` if neither happened.
    pub fn dodge_line(&self, name: &str) -> Option<String> {
        if self.dodges + self.evasion_casts + self.evasion_landed == 0 {
            return None;
        }
        let kind = |label: &str, d: u32, e: u32, p: u32| format!("{label} {d} (slipped a blow {e}, perfect {p})");
        let mut s = format!(
            "{name}'s dodges: {}, {}",
            kind("rolls", self.dodges - self.quick_dodges.min(self.dodges), self.evaded.saturating_sub(self.quick_evaded), self.perfect_dodges.saturating_sub(self.quick_perfect)),
            kind("quickstep", self.quick_dodges, self.quick_evaded, self.quick_perfect),
        );
        if self.evasion_casts + self.evasion_landed + self.evasion_dodged > 0 {
            s += &format!(
                "; evasion cast {}x: {} blows landed while up, {} dodged ({} perfect only thanks to it)",
                self.evasion_casts, self.evasion_landed, self.evasion_dodged, self.evasion_perfect
            );
        }
        Some(s)
    }

    /// The abilities' line, the most damage first. `None` if none was cast or dealt damage.
    pub fn abilities_line(&self, name: &str) -> Option<String> {
        if self.abilities.is_empty() {
            return None;
        }
        let mut rows: Vec<&AbilityTally> = self.abilities.iter().collect();
        rows.sort_by(|a, b| b.damage.cmp(&a.damage).then_with(|| a.name.cmp(&b.name)));
        let rows: Vec<String> = rows.iter().map(|a| format!("{} cast {}x, {} hits, {} foes, {} damage", a.name, a.casts, a.hits, a.foes.len(), a.damage)).collect();
        Some(format!("{name}'s abilities: {}", rows.join("; ")))
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

    /// A dodge starts; `quick`, a Quickstep's.
    pub(crate) fn melee_dodge_started(&mut self, id: Id, quick: bool) {
        if let Some(t) = self.melee_mut(id) {
            t.dodges += 1;
            t.quick_dodges += quick as u32;
            t.dodge_quick = quick;
            t.dodge_evaded = false;
            t.dodge_perfect = false;
        }
    }

    /// The dodge going slipped a real blow, `perfect` if in the perfect moment (`thanks`: only
    /// Evasion's wider window made it so). Each counts once per dodge.
    pub(crate) fn melee_evaded(&mut self, id: Id, perfect: bool, thanks: bool) {
        if let Some(t) = self.melee_mut(id) {
            if !std::mem::replace(&mut t.dodge_evaded, true) {
                t.evaded += 1;
                t.quick_evaded += t.dodge_quick as u32;
            }
            if perfect && !std::mem::replace(&mut t.dodge_perfect, true) {
                t.perfect_dodges += 1;
                t.quick_perfect += t.dodge_quick as u32;
                t.evasion_perfect += thanks as u32;
            }
        }
    }

    /// Hero `id` casts `ability` (by its name; Evasion is counted too).
    pub(crate) fn melee_cast(&mut self, id: Id, ability: usize) {
        let d = &self.content.abilities[ability];
        let (name, evasion) = (d.name.clone(), d.effects.iter().any(|e| e.kind == "evasion"));
        if let Some(t) = self.melee_mut(id) {
            t.evasion_casts += evasion as u32;
            if let Some(a) = t.ability(&name) {
                a.casts += 1;
            }
        }
    }

    /// Hero `by` dealt `amount` to `victim` as `label` (the damage meter's name for it).
    pub(crate) fn melee_dealt(&mut self, by: Id, victim: Id, amount: u32, label: &str) {
        if amount == 0 {
            return;
        }
        if let Some(t) = self.melee_mut(by) {
            t.dealt += amount as u64;
            if label != "Melee" {
                if let Some(a) = t.ability(label) {
                    a.hits += 1;
                    a.damage += amount as u64;
                    if !a.foes.contains(&victim) {
                        a.foes.push(victim);
                    }
                }
            }
        }
    }

    /// Hero `id` took `amount` and has `health` of `max` left.
    pub(crate) fn melee_taken(&mut self, id: Id, amount: u32, health: u32, max: u32) {
        if let Some(t) = self.melee_mut(id) {
            t.taken += amount as u64;
            if t.lowest.map_or(true, |(low, _)| health < low) {
                t.lowest = Some((health, max));
            }
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
    /// `perfect`: a parry made with a ward.
    /// `real`: a blow with damage, not a dash's untouchable moment.
    pub(crate) fn melee_defended(&mut self, id: Id, outcome: Outcome, perfect: bool, real: bool) {
        let evasion = real && self.has_cc(id, "evasion");
        if evasion && matches!(outcome, Outcome::Hit | Outcome::GuardBroken | Outcome::Dodged) {
            if let Some(t) = self.melee_mut(id) {
                if outcome == Outcome::Dodged {
                    t.evasion_dodged += 1;
                } else {
                    t.evasion_landed += 1;
                }
            }
        }
        if !matches!(outcome, Outcome::Parried | Outcome::Blocked | Outcome::GuardBroken) {
            return;
        }
        if let Some(t) = self.melee_mut(id) {
            match outcome {
                Outcome::Parried => {
                    t.parries += 1;
                    t.perfect_parries += perfect as u32;
                }
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
        for line in [tally.line(&name), tally.dodge_line(&name), tally.abilities_line(&name)].into_iter().flatten() {
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
