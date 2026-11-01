//! The Bookmaker's Wager on screen: its page in the book (the levels open, its terms, the camp's
//! best, a level to take), the Bookmaker's Board, and the clock below while one's on.

use htl_survival::sim::wager::{clock_words, terms_at, wager_forces_needed, wager_forces_total, wager_limit, TERMS, WAGER_DEATH};
use htl_survival::sim::Id;

use super::play::clean;
use super::SurvivalGame;
use crate::world::ConnId;

impl SurvivalGame {
    /// A dungeon's wager page: what it is, the levels open, the camp's best, the board's.
    pub(crate) fn ui_wager_page(&mut self, conn: ConnId, id: Id, dungeon: &str) -> Result<(), String> {
        let c = &self.sim.content;
        let d = c.dungeons().find(|d| d.key == dungeon).cloned().ok_or("No such dungeon in the book: .es wager deadmines (or stockade, ...).")?;
        let most = self.sim.wager_unlocked(&d.key);
        let limit = wager_limit(&d, c);
        let (needed, total) = (wager_forces_needed(&d, c), wager_forces_total(&d, c));
        // Plain words, a line a thought (play-test: the page didn't say how to open +2).
        let mut paras = vec![
            format!("The Bookmaker bets you can't clear {} before the sand runs out.", d.name),
            format!(
                "Take a wager below and the dungeon starts fresh for your run: every boss up again, every secret hidden, every shortcut shut. At the first lamp, each of you says you're ready (the button on the clock, or .es ready). When everyone has, the clock starts after a short count, or at once if someone steps away from the lamp first. It stops when the last boss falls and enough of the dungeon's foes are down too: {needed} of its {total}. Each death costs {}.",
                clock_words(WAGER_DEATH)
            ),
            "Foes chase much farther during a wager, so you can pull one pack into the next and fight them together. Big pulls are the quick way to the count.".to_string(),
            "Beat the clock at your highest open wager and the next one opens. Finish with a fifth of the time to spare and you skip a level; with two-fifths to spare, two.".to_string(),
            "Going home through a lamp takes you out of the run. Once everyone is out, the run is over and the dungeon is back as you left it.".to_string(),
            "The higher the wager, the tougher the foes and the better the pay: plunder for everyone, and the dungeon's gear in the camp's chest, the finer pieces from +3. From +4 that gear comes honed, better than any foe's own: Honed at +4, Keen at +8, Peerless at +12.".to_string(),
        ];
        if most <= 1 {
            paras.push("Only +1 is open for now. Beat it in time to open +2.".to_string());
        }
        let terms: Vec<String> = TERMS.iter().map(|t| format!("+{} {}: {}", t.level, t.name, t.text)).collect();
        paras.push(format!("As the stakes rise:\n{}", terms.join("\n")));
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
        if let Some(line) = self.rating_words(id) {
            paras.push(line);
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
        links.push((".es ladder".to_string(), "The Bookmaker's Ladder".to_string()));
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
        let links = vec![(format!(".es wager {}", d.key), "The Bookmaker's Wager".to_string()), (".es ladder".to_string(), "The Bookmaker's Ladder".to_string())];
        self.letter(conn, "board", &format!("The Bookmaker's Board: {}", d.name), &place, &text, "", &links);
        Ok(())
    }

    /// A hero's rating in words, for the wager's page ("Your rating: 412, 3rd of 9"), if they have
    /// one.
    fn rating_words(&self, id: Id) -> Option<String> {
        let name = self.sim.players.get(&id)?.name.clone();
        let b = self.board.as_ref()?.lock().unwrap();
        let all = b.ladder().len();
        let (place, r) = b.place_of(&name)?;
        Some(format!("Your rating on the Bookmaker's Ladder: {}, {} of {all} (from {} dungeon{}).", r.rating, ordinal(place), r.dungeons, if r.dungeons == 1 { "" } else { "s" }))
    }

    /// The Bookmaker's Ladder: every hero across every dungeon by their rating.
    pub(crate) fn ui_ladder_page(&mut self, conn: ConnId, id: Id) -> Result<(), String> {
        let board = self.board.clone().ok_or("This server keeps no board.")?;
        let (ladder, here) = {
            let b = board.lock().unwrap();
            (b.ladder(), b.host.clone())
        };
        let me = self.sim.players.get(&id).map(|p| p.name.clone()).unwrap_or_default();
        let mut lines: Vec<String> = ladder
            .iter()
            .take(20)
            .enumerate()
            .map(|(k, r)| {
                let you = if r.name == me && r.host == here { "  (you)" } else { "" };
                let class = if r.class.is_empty() { String::new() } else { format!(" ({})", title_case(&r.class)) };
                format!("{}. {}{class} of {}: {} from {} dungeon{}{you}", k + 1, r.name, r.host, r.rating, r.dungeons, if r.dungeons == 1 { "" } else { "s" })
            })
            .collect();
        if lines.is_empty() {
            lines.push("Nobody has won a wager in time yet. The first to do it tops the ladder.".to_string());
        }
        let text = format!(
            "{}\n\nA rating is each dungeon's best run in time, summed: 50, and 20 for each level, and up to 20 more for time to spare. A harder dungeon is worth a quarter more for each tier.",
            lines.join("\n")
        );
        let place = match self.rating_words(id) {
            Some(w) => w,
            None => "Win a wager in time to get on it".to_string(),
        };
        self.letter(conn, "board", "The Bookmaker's Ladder", &place, &text, "", &[]);
        Ok(())
    }

    /// The clock below (`WT name level elapsed limit falls state terms bosses forces`), or `WT`
    /// with none. `bosses`: `name~down^...`, each boss the run has to beat and whether it's down
    /// (1) yet. `forces`: `felled~needed~total`, the foes counted toward the run. `ready`, before
    /// the clock: `ready~heroes~count` (the count's seconds left, empty till everyone's ready).
    /// `splits`: for each boss in `bosses`' order, `split~best` in seconds, the clock when it fell
    /// and when it fell in this server's quickest run in time at this level (either empty).
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
        let forces = self.sim.wager_forces().map_or(String::new(), |(f, n, t)| format!("{f}~{n}~{t}"));
        let best = self.board.as_ref().and_then(|b| b.lock().unwrap().best_splits(self.sim.content.dungeon.as_ref().map_or("", |d| d.key.as_str()), w.level));
        let splits: Vec<String> = self
            .sim
            .wager_splits()
            .iter()
            .enumerate()
            .map(|(k, s)| {
                let b = best.as_ref().and_then(|b| b.get(k)).map_or(String::new(), |(_, ms)| format!("{}", ms / 1000));
                format!("{}~{b}", s.map_or(String::new(), |s| format!("{}", s.floor())))
            })
            .collect();
        let ready = self.sim.wager_readiness().map_or(String::new(), |(r, n, c)| format!("{r}~{n}~{}", c.map_or(String::new(), |c| format!("{}", c.ceil()))));
        format!("WT\t{}\t{}\t{:.0}\t{:.0}\t{}\t{state}\t{}\t{}\t{forces}\t{ready}\t{}", clean(&name), w.level, self.sim.wager_elapsed().floor(), w.limit, w.deaths, clean(&terms.join(", ")), bosses.join("^"), splits.join("^"))
    }
}

/// 1st, 2nd, 3rd, 4th...
fn ordinal(n: usize) -> String {
    let suffix = match (n % 10, n % 100) {
        (1, x) if x != 11 => "st",
        (2, x) if x != 12 => "nd",
        (3, x) if x != 13 => "rd",
        _ => "th",
    };
    format!("{n}{suffix}")
}

fn title_case(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}
