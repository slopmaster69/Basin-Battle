//! A warned restart. The host asks for one (`Restart Basin Battle.command` writes
//! [`REQUEST_FILE`] in the folder the server runs in); every game warns its players as the time
//! comes and saves; then the server exits with [`EXIT_RESTART`], and the host script builds and
//! starts it again. Nobody is dropped without warning, and the host never has to ask who's on.

use std::path::Path;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// The exit code that tells the host script to build and start the server again.
pub const EXIT_RESTART: i32 = 75;
/// The file the host writes to ask for a restart: minutes until it (2 when it says none), or
/// `cancel`.
pub const REQUEST_FILE: &str = "restart.request";
/// The minutes when a request names none, and the most it may name.
pub const DEFAULT_MINUTES: f32 = 2.0;
const MOST_MINUTES: f32 = 60.0;
/// The warnings, by seconds left: each said once, as the time passes it.
const WARN_AT: [u64; 9] = [300, 120, 60, 30, 10, 5, 3, 2, 1];
/// How long the server waits at the time for every game to say it has saved.
const SAVE_GRACE: Duration = Duration::from_secs(5);

struct State {
    at: Option<Instant>,
    saved: u32,
}

static STATE: Mutex<State> = Mutex::new(State { at: None, saved: 0 });

/// Restart after this long (a new ask replaces an earlier one).
pub fn ask(after: Duration) {
    let mut s = STATE.lock().unwrap();
    s.at = Some(Instant::now() + after);
    s.saved = 0;
}

/// Call off the restart, if one is coming.
pub fn cancel() {
    let mut s = STATE.lock().unwrap();
    s.at = None;
    s.saved = 0;
}

/// When the restart is, if one is coming.
pub fn due_at() -> Option<Instant> {
    STATE.lock().unwrap().at
}

/// A game has saved for the restart.
pub fn saved() {
    STATE.lock().unwrap().saved += 1;
}

/// The time has come and every one of `games` has saved (or had its few seconds to).
pub fn ready_to_exit(games: u32) -> bool {
    let s = STATE.lock().unwrap();
    s.at.is_some_and(|at| {
        let now = Instant::now();
        now >= at && (s.saved >= games || now >= at + SAVE_GRACE)
    })
}

/// What a request file asks: `Some(Some(minutes))` a restart, `Some(None)` to call it off.
pub fn parse_request(text: &str) -> Option<Option<f32>> {
    let t = text.trim();
    if t.eq_ignore_ascii_case("cancel") {
        return Some(None);
    }
    if t.is_empty() {
        return Some(Some(DEFAULT_MINUTES));
    }
    t.parse::<f32>().ok().filter(|m| m.is_finite() && *m >= 0.0).map(|m| Some(m.min(MOST_MINUTES)))
}

/// Read and remove a request file in `dir`, acting on it. What it did, for the log.
pub fn poll_request(dir: &Path) -> Option<String> {
    let path = dir.join(REQUEST_FILE);
    let text = std::fs::read_to_string(&path).ok()?;
    let _ = std::fs::remove_file(&path);
    match parse_request(&text) {
        Some(Some(m)) => {
            ask(Duration::from_secs_f32(m * 60.0));
            Some(format!("[restart] asked for, in {}", words(m * 60.0)))
        }
        Some(None) => {
            cancel();
            Some("[restart] called off".to_string())
        }
        None => Some(format!("[restart] couldn't read {REQUEST_FILE} ({:?}): say a number of minutes, or cancel", text.trim())),
    }
}

/// "2 minutes", "1 minute", "30 seconds".
pub fn words(secs: f32) -> String {
    let secs = secs.max(0.0).ceil() as u64;
    let (n, unit) = if secs >= 60 { ((secs + 30) / 60, "minute") } else { (secs, "second") };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}

/// What a game says, as a restart comes.
#[derive(Debug, Clone, PartialEq)]
pub enum Step {
    /// A warning: the title and a line under it.
    Warn(String, String),
    /// The time: say so, save, and call [`saved`].
    Now(String, String),
    /// Called off.
    Off(String, String),
}

/// Each game's own count of what it has said (one per game loop).
#[derive(Debug, Default)]
pub struct Warner {
    /// The restart it's warning of, and the warning last said (seconds; `u64::MAX` for the first
    /// word on a restart further off than any warning).
    at: Option<Instant>,
    said: Option<u64>,
    done: bool,
}

impl Warner {
    /// What to say now, if anything: call it every tick.
    pub fn step(&mut self) -> Option<Step> {
        self.step_at(due_at(), Instant::now())
    }

    fn step_at(&mut self, due: Option<Instant>, now: Instant) -> Option<Step> {
        let Some(at) = due else {
            let was = self.at.take().is_some() && !self.done;
            self.said = None;
            self.done = false;
            return was.then(|| Step::Off("The restart is called off".into(), "Play on.".into()));
        };
        if self.at != Some(at) {
            self.at = Some(at);
            self.said = None;
            self.done = false;
        }
        if self.done {
            return None;
        }
        let left = at.saturating_duration_since(now).as_secs_f32();
        if left <= 0.0 {
            self.done = true;
            return Some(Step::Now("The server is restarting".into(), "Everything is saved. Back in a minute or two.".into()));
        }
        let secs = left.ceil() as u64;
        let due = WARN_AT.iter().copied().filter(|t| secs <= *t).min().unwrap_or(u64::MAX);
        if self.said == Some(due) {
            return None;
        }
        self.said = Some(due);
        Some(Step::Warn(format!("Server restart in {}", words(left)), "For an update. Finish your fight: everything is saved.".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_request_says_minutes_or_cancel_and_nothing_else() {
        assert_eq!(parse_request(""), Some(Some(DEFAULT_MINUTES)));
        assert_eq!(parse_request(" 5\n"), Some(Some(5.0)));
        assert_eq!(parse_request("0"), Some(Some(0.0)));
        assert_eq!(parse_request("999"), Some(Some(MOST_MINUTES)));
        assert_eq!(parse_request("Cancel"), Some(None));
        assert_eq!(parse_request("soon"), None);
        assert_eq!(parse_request("-1"), None);
    }

    #[test]
    fn time_reads_in_minutes_then_seconds() {
        assert_eq!(words(120.0), "2 minutes");
        assert_eq!(words(61.0), "1 minute");
        assert_eq!(words(30.0), "30 seconds");
        assert_eq!(words(0.4), "1 second");
    }

    #[test]
    fn each_warning_is_said_once_then_the_time_once_and_a_call_off_once() {
        let t0 = Instant::now();
        let at = t0 + Duration::from_secs(120);
        let mut w = Warner::default();
        let said = |w: &mut Warner, secs_in: f32| w.step_at(Some(at), t0 + Duration::from_secs_f32(secs_in));
        assert!(matches!(said(&mut w, 0.0), Some(Step::Warn(t, _)) if t == "Server restart in 2 minutes"), "the first word at once");
        assert_eq!(said(&mut w, 1.0), None, "not again");
        assert_eq!(said(&mut w, 59.0), None);
        assert!(matches!(said(&mut w, 60.0), Some(Step::Warn(t, _)) if t == "Server restart in 1 minute"));
        assert!(matches!(said(&mut w, 90.0), Some(Step::Warn(t, _)) if t == "Server restart in 30 seconds"));
        assert!(matches!(said(&mut w, 119.5), Some(Step::Warn(t, _)) if t == "Server restart in 1 second"));
        assert!(matches!(said(&mut w, 120.0), Some(Step::Now(..))));
        assert_eq!(said(&mut w, 121.0), None, "the time is said once");
        // Called off before its time: said once.
        let mut w = Warner::default();
        w.step_at(Some(at), t0);
        assert!(matches!(w.step_at(None, t0), Some(Step::Off(..))));
        assert_eq!(w.step_at(None, t0), None);
    }

    #[test]
    fn a_restart_far_off_is_said_at_once_and_then_at_five_minutes() {
        let t0 = Instant::now();
        let at = t0 + Duration::from_secs(600);
        let mut w = Warner::default();
        assert!(matches!(w.step_at(Some(at), t0), Some(Step::Warn(t, _)) if t == "Server restart in 10 minutes"));
        assert_eq!(w.step_at(Some(at), t0 + Duration::from_secs(200)), None);
        assert!(matches!(w.step_at(Some(at), t0 + Duration::from_secs(300)), Some(Step::Warn(t, _)) if t == "Server restart in 5 minutes"));
    }
}
