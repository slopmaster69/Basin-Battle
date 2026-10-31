//! What lasts between runs: the high-score boards and each player's Renown, kept in one small
//! file on the host's computer.

use std::collections::BTreeMap;
use std::path::PathBuf;

use htl_sim::{Content, Luck, RunResult};
use serde::{Deserialize, Serialize};

use super::*;

#[derive(Debug, Default, Serialize, Deserialize)]
struct Data {
    #[serde(default)]
    renown: BTreeMap<String, Renown>,
    #[serde(default)]
    scores: Vec<Score>,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
struct Renown {
    /// Unspent.
    points: u32,
    /// Everything ever earned.
    earned: u32,
    /// Unlock key to level bought.
    #[serde(default)]
    levels: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Score {
    players: Vec<String>,
    classes: Vec<String>,
    depth: u32,
    cycles: u32,
    secs: u32,
    /// Seconds since 1970.
    when: u64,
    /// The run was won (its victory wave cleared).
    #[serde(default)]
    won: bool,
    /// The win itself, not where the run ended: kept apart so a run counts once on the boards.
    #[serde(default)]
    victory_mark: bool,
}

/// Renown for winning a run, on top of what its depth earns when it ends.
const VICTORY_RENOWN: u32 = 50;

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub struct Saves {
    path: Option<PathBuf>,
    data: Data,
}

/// A Renown menu option.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RenownChoice {
    Buy(String, u32),
    Leave,
}

impl Saves {
    pub fn load(path: Option<PathBuf>) -> Self {
        let data = path
            .as_ref()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| match toml::from_str(&s) {
                Ok(d) => Some(d),
                Err(e) => {
                    eprintln!("[saves] couldn't read the saves file ({e}); starting fresh");
                    None
                }
            })
            .unwrap_or_default();
        Self { path, data }
    }

    fn write(&self) {
        let Some(path) = &self.path else { return };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        match toml::to_string_pretty(&self.data) {
            Ok(text) => {
                let tmp = path.with_extension("tmp");
                if std::fs::write(&tmp, text).and_then(|_| std::fs::rename(&tmp, path)).is_err() {
                    eprintln!("[saves] couldn't write {}", path.display());
                }
            }
            Err(e) => eprintln!("[saves] {e}"),
        }
    }

    pub fn renown_points(&self, name: &str) -> u32 {
        self.data.renown.get(name).map_or(0, |r| r.points)
    }

    fn level(&self, name: &str, key: &str) -> u32 {
        self.data.renown.get(name).and_then(|r| r.levels.get(key)).copied().unwrap_or(0)
    }

    /// What a player's Renown buys them in a run.
    pub fn luck(&self, name: &str, _content: &Content) -> Luck {
        Luck {
            rerolls: 1 + self.level(name, "reroll"),
            banishes: self.level(name, "banish"),
            skips: self.level(name, "skip"),
            wider: self.level(name, "wider") > 0,
            lock: self.level(name, "lock") > 0,
            head_start: self.level(name, "head_start") > 0,
        }
    }

    /// Record a won run the moment it's won (in case the team stops there); returns the bonus
    /// Renown each player earned.
    fn record_victory(&mut self, r: &RunResult) -> u32 {
        let earned = VICTORY_RENOWN;
        for p in &r.players {
            let e = self.data.renown.entry(p.name.clone()).or_default();
            e.points += earned;
            e.earned += earned;
        }
        self.data.scores.push(Score {
            players: r.players.iter().map(|p| p.name.clone()).collect(),
            classes: r.players.iter().map(|p| p.class.clone()).collect(),
            depth: r.depth,
            cycles: r.cycles,
            secs: r.secs,
            when: now_secs(),
            won: true,
            victory_mark: true,
        });
        self.write();
        earned
    }

    /// How many runs have been won.
    pub fn victories(&self) -> usize {
        self.data.scores.iter().filter(|s| s.won).count()
    }

    /// Record a finished run; returns the Renown each player earned.
    fn record(&mut self, r: &RunResult) -> u32 {
        // A won run's victory entry gives way to where it really ended.
        if r.won {
            let team: Vec<String> = r.players.iter().map(|p| p.name.clone()).collect();
            if let Some(i) = self.data.scores.iter().rposition(|s| s.victory_mark && s.players == team) {
                self.data.scores.remove(i);
            }
        }
        let earned = r.depth * 3 + r.cycles * 20;
        for p in &r.players {
            let e = self.data.renown.entry(p.name.clone()).or_default();
            e.points += earned;
            e.earned += earned;
        }
        if r.depth > 0 {
            self.data.scores.push(Score {
                players: r.players.iter().map(|p| p.name.clone()).collect(),
                classes: r.players.iter().map(|p| p.class.clone()).collect(),
                depth: r.depth,
                cycles: r.cycles,
                secs: r.secs,
                when: now_secs(),
                won: r.won,
                victory_mark: false,
            });
        }
        self.write();
        earned
    }

    /// The deepest wave any recorded run reached.
    pub fn best_depth(&self) -> u32 {
        self.data.scores.iter().map(|s| s.depth).max().unwrap_or(0)
    }

    /// The best runs for each team size, deepest first.
    pub fn best_lines(&self) -> Vec<String> {
        let mut lines = Vec::new();
        for size in 1..=4usize {
            let mut runs: Vec<&Score> = self.data.scores.iter().filter(|s| s.players.len() == size).collect();
            if runs.is_empty() {
                continue;
            }
            runs.sort_by(|a, b| (b.depth, b.cycles, std::cmp::Reverse(b.secs)).cmp(&(a.depth, a.cycles, std::cmp::Reverse(a.secs))));
            let label = match size {
                1 => "Solo",
                2 => "Duo",
                3 => "Trio",
                _ => "Four",
            };
            lines.push(format!("{label} best:"));
            for (i, s) in runs.iter().take(3).enumerate() {
                let team: Vec<String> = s.players.iter().zip(&s.classes).map(|(n, c)| format!("{n} ({c})")).collect();
                lines.push(format!(
                    "  {}. {}Wave {}{} in {}m: {}",
                    i + 1,
                    if s.won { "VICTORY, " } else { "" },
                    s.depth,
                    if s.cycles > 0 { format!(", Escalation {}", s.cycles) } else { String::new() },
                    s.secs / 60,
                    team.join(", ")
                ));
            }
        }
        if lines.is_empty() {
            lines.push("No runs recorded yet.".into());
        } else {
            let wins = self.victories();
            lines.push(if wins == 0 { "No victories yet: hold the Basin through the final wave.".into() } else { format!("Victories: {wins}.") });
        }
        lines
    }
}

/// The static greeting of each dialog; the client caches these by number.
/// Every dialog text the game shows.
pub(crate) const NPC_TEXTS: std::ops::RangeInclusive<u32> = 90001..=90009;

pub(crate) fn npc_text(text_id: u32) -> &'static str {
    match text_id {
        90001 => "Choose a card. A card you already hold ranks up instead.",
        90002 => "The relic cache opens. Choose a relic card.",
        90003 => "The Scourge escalates! Choose a legendary card.",
        90004 => "Choose a card.",
        90005 => "Choose the set piece you want.",
        90006 => "The Ancient's roots run deep. Spend gold to strengthen the defence, or enchant your weapon.",
        90007 => "A Scourge banner. Claim it to take this base for the Alliance.",
        90008 => "A relic cache, freed from its guardians.",
        90009 => "Renown stays with you from run to run. It never makes you stronger, but it bends your luck.",
        _ => "...",
    }
}

impl Game {
    pub(crate) fn victory(&mut self, r: RunResult) {
        let earned = self.saves.record_victory(&r);
        self.scoreboard(&r, earned);
        for p in &r.players {
            let total = self.saves.renown_points(&p.name);
            let text = format!("{} earns {earned} Renown for the victory ({total} to spend: type .renown).", p.name);
            self.broadcast(&packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text));
        }
    }

    /// The run's scoreboard: add-on players get a panel (`RS~won~depth~secs~rows`, a row
    /// `name^class^level^kills^damage^falls^dodges`), everyone else a few chat lines.
    fn scoreboard(&self, r: &RunResult, renown: u32) {
        let rows: Vec<String> = r
            .players
            .iter()
            .map(|p| format!("{}^{}^{}^{}^{}^{}^{}", p.name.replace(['^', ';', '~'], " "), p.class, p.level, p.kills, p.dealt, p.falls, p.dodges))
            .collect();
        let line = format!("RS~{}~{}~{}~{}~{renown}", u8::from(r.won), r.depth, r.secs, rows.join(";"));
        let conns: Vec<(ConnId, bool)> = self.in_world().map(|(c, _, w)| (*c, w.addon)).collect();
        for (conn, addon) in conns {
            if addon {
                self.ui_line(conn, &line);
            } else {
                for p in &r.players {
                    let text = format!(
                        "{} the {} (level {}): {} kills, {} damage, {} dodged, fell {} time{}.",
                        p.name,
                        p.class,
                        p.level,
                        p.kills,
                        p.dealt,
                        p.dodges,
                        p.falls,
                        if p.falls == 1 { "" } else { "s" }
                    );
                    self.send(conn, packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text));
                }
            }
        }
    }

    pub(crate) fn run_over(&mut self, r: RunResult) {
        let earned = self.saves.record(&r);
        self.scoreboard(&r, earned);
        if earned > 0 {
            for p in &r.players {
                let total = self.saves.renown_points(&p.name);
                let text = format!("{} earns {earned} Renown ({total} to spend: type .renown).", p.name);
                self.broadcast(&packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text));
            }
        }
        // Renown bought during the run applies from the next one.
        let players: Vec<(EntityId, String)> = self.in_world().map(|(_, _, w)| (w.entity, w.name.clone())).collect();
        for (id, name) in players {
            let luck = self.saves.luck(&name, self.sim.content());
            self.sim.set_luck(id, luck);
        }
    }

    pub(crate) fn open_renown(&mut self, conn: ConnId) {
        let Some((guid, name)) = self.world_of(conn).map(|w| (w.guid, w.name.clone())) else { return };
        let points = self.saves.renown_points(&name);
        let mut options: Vec<(u8, String)> = Vec::new();
        let mut choices = Vec::new();
        for def in &self.sim.content().renown {
            let level = self.saves.level(&name, &def.key);
            match def.costs.get(level as usize) {
                Some(&cost) => {
                    options.push((
                        6,
                        format!("{} {} ({} Renown): {}", def.name, level + 1, cost, def.text),
                    ));
                    choices.push(RenownChoice::Buy(def.key.clone(), cost));
                }
                None => {
                    options.push((0, format!("{} (all bought)", def.name)));
                    choices.push(RenownChoice::Leave);
                }
            }
        }
        options.push((0, format!("Leave. (You have {points} Renown.)")));
        choices.push(RenownChoice::Leave);
        let opts: Vec<(u8, &str)> = options.iter().map(|(i, t)| (*i, t.as_str())).collect();
        self.send(conn, packets::gossip_message(guid, 90009, &opts));
        if let Some(w) = self.world_mut(conn) {
            w.renown_menu = Some(choices);
        }
    }

    pub(crate) fn renown_choice(&mut self, conn: ConnId, choices: Vec<RenownChoice>, index: usize) {
        let Some(name) = self.world_of(conn).map(|w| w.name.clone()) else { return };
        match choices.get(index) {
            Some(RenownChoice::Buy(key, cost)) => {
                let r = self.saves.data.renown.entry(name.clone()).or_default();
                let msg = if r.points < *cost {
                    format!("That needs {cost} Renown; you have {}.", r.points)
                } else {
                    r.points -= cost;
                    *r.levels.entry(key.clone()).or_insert(0) += 1;
                    self.saves.write();
                    "Bought. It takes effect from your next run.".to_string()
                };
                self.say_to(conn, &msg);
                self.open_renown(conn);
            }
            _ => self.send(conn, packets::gossip_complete()),
        }
    }
}
