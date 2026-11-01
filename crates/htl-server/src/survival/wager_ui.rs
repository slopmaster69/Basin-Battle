//! The Bookmaker's Wager on screen: its page in the book (the levels open, its terms, the camp's
//! best, a level to take), the Bookmaker's Board, and the clock below while one's on.

use htl_survival::sim::wager::{clock_words, terms_at, wager_limit, TERMS, WAGER_DEATH};
use htl_survival::sim::Id;

use super::play::clean;
use super::SurvivalGame;
use crate::world::ConnId;

impl SurvivalGame {
    /// A dungeon's wager page: what it is, the levels open, the camp's best, the board's.
    pub(crate) fn ui_wager_page(&mut self, conn: ConnId, _id: Id, dungeon: &str) -> Result<(), String> {
        let c = &self.sim.content;
        let d = c.dungeons().find(|d| d.key == dungeon).cloned().ok_or("No such dungeon in the book: .es wager deadmines (or stockade, ...).")?;
        let most = self.sim.wager_unlocked(&d.key);
        let limit = wager_limit(&d, c);
        let mut paras = vec![format!(
            "The Bookmaker takes your wager on a run against the clock. It starts {} fresh: its bosses up, its secrets hidden, its shortcuts shut. The clock starts when the first of you leaves the first lamp and stops when its last boss falls; each fall adds {}. Win in time and the stakes go up (two levels for a run in four-fifths of the time, three in three-fifths). The higher the wager, the tougher the foes, and the more it pays: plunder for all of you, and the dungeon's gear into the camp's chest, the finer pieces from +3.",
            d.name,
            clock_words(WAGER_DEATH)
        )];
        let terms: Vec<String> = TERMS.iter().map(|t| format!("+{} {}: {}", t.level, t.name, t.text)).collect();
        paras.push(format!("The terms, as the stakes rise:\n{}", terms.join("\n")));
        let (bests, top) = self.board.as_ref().map_or((Vec::new(), Vec::new()), |b| {
            let b = b.lock().unwrap();
            (b.bests_here(&d.key), b.top(&d.key, 1))
        });
        if !bests.is_empty() {
            let list: Vec<String> = bests.iter().take(4).map(|(l, ms)| format!("+{l} in {}", clock_words(*ms as f32 / 1000.0))).collect();
            paras.push(format!("Your camp's best in time: {}.", list.join("; ")));
        }
        if let Some(r) = top.first() {
            paras.push(format!("The board's best: +{} in {}, by {} of {}.", r.level, clock_words(r.ms as f32 / 1000.0), r.heroes(), r.host));
        }
        let mut links: Vec<(String, String)> = (most.saturating_sub(4).max(1)..=most)
            .rev()
            .map(|l| {
                let named: Vec<&str> = terms_at(l).iter().map(|t| t.name).collect();
                let label = if named.is_empty() { format!("Wager +{l}") } else { format!("Wager +{l} ({})", named.join(", ")) };
                (format!(".es wager {} {l}", d.key), label)
            })
            .collect();
        links.push((format!(".es board {}", d.key), "The Bookmaker's Board".to_string()));
        let place = format!("Open to +{most}, {} to beat", clock_words(limit));
        self.letter(conn, "wager", &format!("The Bookmaker's Wager: {}", d.name), &place, &paras.join("\n\n"), "", &links);
        Ok(())
    }

    /// The Bookmaker's Board for a dungeon: its runs in time, highest first, then quickest.
    pub(crate) fn ui_board_page(&mut self, conn: ConnId, dungeon: &str) -> Result<(), String> {
        let d = self.sim.content.dungeons().find(|d| d.key == dungeon).cloned().ok_or("No such dungeon in the book.")?;
        let board = self.board.clone().ok_or("This server keeps no board.")?;
        let (top, hosts, remote, synced, here) = {
            let b = board.lock().unwrap();
            (b.top(&d.key, 15), b.hosts(), b.remote.clone(), b.synced, b.host.clone())
        };
        let lines: Vec<String> = top
            .iter()
            .enumerate()
            .map(|(k, r)| {
                let mine = if r.host == here { " (here)" } else { "" };
                format!("{}. +{}  {}  {}, of {}{mine}{}", k + 1, r.level, clock_words(r.ms as f32 / 1000.0), r.heroes(), r.host, if r.deaths > 0 { format!(", {} fall{}", r.deaths, if r.deaths == 1 { "" } else { "s" }) } else { String::new() })
            })
            .collect();
        let text = if lines.is_empty() { "No wager won in time here yet. Be the first on the board.".to_string() } else { lines.join("\n") };
        let reach = match (&remote, synced) {
            (None, _) => "this server keeps the board for everyone".to_string(),
            (Some(_), Some((at, true))) => format!("last word from the board's keeper {} ago", clock_words(at.elapsed().as_secs_f32())),
            (Some(_), Some((_, false))) => "the board's keeper can't be reached just now: your runs wait to be sent".to_string(),
            (Some(_), None) => "asking the board's keeper".to_string(),
        };
        let place = format!("In time only, highest first, then quickest: {hosts} camp{} on it, {reach}", if hosts == 1 { "" } else { "s" });
        let links = vec![(format!(".es wager {}", d.key), "The Bookmaker's Wager".to_string())];
        self.letter(conn, "board", &format!("The Bookmaker's Board: {}", d.name), &place, &text, "", &links);
        Ok(())
    }

    /// The clock below (`WT name level elapsed limit falls state terms bosses`), or `WT` with none.
    /// `bosses`: `name~down^...`, each boss the run has to beat and whether it's down (1) yet.
    pub(crate) fn wager_line(&self) -> String {
        let Some(w) = self.sim.wager() else { return "WT".to_string() };
        let name = self.sim.content.dungeon.as_ref().map_or(String::new(), |d| d.name.clone());
        let state = match (&w.done, w.started) {
            (Some(e), _) if e.timed => "won",
            (Some(_), _) => "over",
            (None, Some(_)) => "run",
            (None, None) => "wait",
        };
        let terms: Vec<&str> = terms_at(w.level).iter().map(|t| t.name).collect();
        let bosses: Vec<String> = self.sim.wager_progress().iter().map(|(n, down)| format!("{}~{}", clean(n), u8::from(*down))).collect();
        format!("WT\t{}\t{}\t{:.0}\t{:.0}\t{}\t{state}\t{}\t{}", clean(&name), w.level, self.sim.wager_elapsed().floor(), w.limit, w.deaths, clean(&terms.join(", ")), bosses.join("^"))
    }
}
