//! The Bookmaker's Board: every wager won, in time or not, on this server and (through the board
//! host) on everyone's. Each server keeps its own runs and the ones it last fetched in one file
//! beside its save. One server is the board host (the one friends join; it runs without
//! `--board`): the others send it their runs and fetch everyone's back, over plain HTTP on the
//! host's login port (the port friends already reach), so nothing new is opened.
//!
//! What the host takes is held to a little: a short request, a few runs at a time, a few requests
//! a minute from one address, each run checked (a known dungeon, a sane level and time, names
//! that are only names) and signed with the game's key; only the best runs of each dungeon are
//! kept. It runs no command and reads or writes nothing but its own file.
//!
//! The ladder ranks heroes across every dungeon by one rating (as Fellowship's dungeon rating): for
//! each dungeon, the hero's best run in time, worth its level, a little for its speed, and more in a
//! harder dungeon ([`run_points`]); the rating is those summed.

use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{IpAddr, TcpStream};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

use htl_survival::sim::wager::{WagerRun, WAGER_MAX};

/// The game's key for a run's signature: keeps a hand-typed run off the board (not a secret
/// from anyone who reads this file; the checks on what's sent are the real guard).
const KEY: &[u8] = b"bookmaker-of-the-brotherhood:wagers:v1";
/// The most runs kept for each dungeon.
const PER_DUNGEON: usize = 200;
/// What the host reads of a request, and the runs in one.
const MAX_REQUEST: usize = 16 * 1024;
const MAX_LINES: usize = 25;
/// Requests a minute from one address.
const PER_MINUTE: u32 = 20;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Run {
    /// Its own (random) id, and the server it was won on.
    pub id: String,
    pub host: String,
    pub dungeon: String,
    pub level: u32,
    pub ms: u64,
    pub limit_ms: u64,
    pub deaths: u32,
    pub timed: bool,
    /// "Name:Class" for each hero in it.
    pub party: Vec<String>,
    /// When (unix seconds).
    pub at: u64,
}

impl Run {
    pub fn from_wager(w: &WagerRun, host: &str) -> Self {
        Run {
            id: format!("{:016x}", rand::random::<u64>()),
            host: name_only(host, 32).replace(':', ""),
            dungeon: w.dungeon.clone(),
            level: w.level,
            ms: w.ms,
            limit_ms: w.limit_ms,
            deaths: w.deaths,
            timed: w.timed,
            party: w.party.iter().map(|(n, c)| format!("{}:{}", name_only(n, 24).replace(':', ""), name_only(c, 12).replace(':', ""))).collect(),
            at: now(),
        }
    }

    /// One line: its fields with `|` between, then its signature.
    pub fn line(&self) -> String {
        let body = format!(
            "v1|{}|{}|{}|{}|{}|{}|{}|{}|{}|{}",
            self.id,
            self.host,
            self.dungeon,
            self.level,
            self.ms,
            self.limit_ms,
            self.deaths,
            u8::from(self.timed),
            self.party.join(";"),
            self.at
        );
        let sig = sign(&body);
        format!("{body}|{sig}")
    }

    /// A line read back, if it's whole, signed and sane.
    pub fn parse(line: &str, dungeons: &[String]) -> Option<Run> {
        let (body, sig) = line.trim().rsplit_once('|')?;
        if sign(body) != sig {
            return None;
        }
        let f: Vec<&str> = body.split('|').collect();
        if f.len() != 11 || f[0] != "v1" {
            return None;
        }
        let id = f[1];
        if id.len() != 16 || !id.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let dungeon = f[3].to_string();
        if !dungeons.contains(&dungeon) {
            return None;
        }
        let level: u32 = f[4].parse().ok()?;
        let ms: u64 = f[5].parse().ok()?;
        let limit_ms: u64 = f[6].parse().ok()?;
        let deaths: u32 = f[7].parse().ok()?;
        let party: Vec<String> = f[9].split(';').filter(|p| !p.is_empty()).map(|p| p.to_string()).collect();
        let sane = (1..=WAGER_MAX).contains(&level)
            && (60_000..=4 * 3_600_000).contains(&ms)
            && (5 * 60_000..=2 * 3_600_000).contains(&limit_ms)
            && deaths <= 500
            && !party.is_empty()
            && party.len() <= 5
            && party.iter().all(|p| p.len() <= 40 && p.split(':').count() == 2 && *p == name_only(p, 40))
            && f[2] == name_only(f[2], 32);
        if !sane {
            return None;
        }
        Some(Run {
            id: id.to_string(),
            host: f[2].to_string(),
            dungeon,
            level,
            ms,
            limit_ms,
            deaths,
            // In time is the clock's say, not the sender's.
            timed: ms <= limit_ms,
            party,
            at: f[10].parse().ok()?,
        })
    }

    /// Its heroes, in words: "Ann (Warrior), Bob (Mage)".
    pub fn heroes(&self) -> String {
        self.party
            .iter()
            .map(|p| match p.split_once(':') {
                Some((n, c)) if !c.is_empty() => format!("{n} ({})", title(c)),
                _ => p.clone(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The board as kept: this server's runs and everyone's last fetched, and what's still to send.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Store {
    /// This server's name on the board (made up once, unless it's given).
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub runs: Vec<Run>,
    #[serde(default)]
    pub unsent: Vec<String>,
}

pub struct Board {
    pub store: Store,
    path: Option<PathBuf>,
    /// This server's name on the board.
    pub host: String,
    /// The board host's address (`host:port`), for a server that isn't it.
    pub remote: Option<String>,
    /// The dungeons this server knows.
    dungeons: Vec<String>,
    /// Requests lately from each address (the host's).
    asks: HashMap<IpAddr, (Instant, u32)>,
    /// When it last heard from the board host, and whether that went well.
    pub synced: Option<(Instant, bool)>,
    /// Each dungeon's tier (1 to 3), for the ladder's points.
    tiers: HashMap<String, u32>,
}

/// A hero on the ladder: their name, class and camp, their rating, and the dungeons it's from.
#[derive(Debug, Clone, PartialEq)]
pub struct Rank {
    pub name: String,
    pub class: String,
    pub host: String,
    pub rating: u32,
    pub dungeons: usize,
}

/// What a run in time is worth on the ladder: 50, and 20 a level, and up to 20 more the quicker
/// it beat its time (none for exactly on it); a quarter more a dungeon tier past the first. (A
/// first pass: a level is worth more than any speed, a harder dungeon more than an easier.)
pub fn run_points(run: &Run, tier: u32) -> f32 {
    if !run.timed || run.limit_ms == 0 {
        return 0.0;
    }
    let spare = (1.0 - run.ms as f32 / run.limit_ms as f32).clamp(0.0, 1.0);
    (50.0 + 20.0 * run.level as f32 + 20.0 * spare) * (1.0 + 0.25 * tier.saturating_sub(1) as f32)
}

pub type Shared = Arc<Mutex<Board>>;

impl Board {
    /// `host`: this server's name on the board (a made-up camp's, kept, when empty).
    pub fn new(path: Option<PathBuf>, host: &str, remote: Option<String>, dungeons: Vec<String>) -> Self {
        let mut store = path.as_deref().map(load).unwrap_or_default();
        if !host.is_empty() {
            store.name = name_only(host, 32).replace(':', "");
        }
        if store.name.is_empty() {
            store.name = format!("Camp {:04X}", rand::random::<u16>());
        }
        let host = store.name.clone();
        let b = Board { store, path, host, remote, dungeons, asks: HashMap::new(), synced: None, tiers: HashMap::new() };
        b.save();
        b
    }

    /// A run won here: kept, and (for a server that isn't the host) to send.
    pub fn record(&mut self, w: &WagerRun) -> Run {
        let run = Run::from_wager(w, &self.host);
        if self.remote.is_some() {
            self.store.unsent.push(run.line());
        }
        self.add(run.clone());
        self.save();
        run
    }

    /// A run into the board (once), the best of each dungeon kept.
    fn add(&mut self, run: Run) -> bool {
        if self.store.runs.iter().any(|r| r.id == run.id) {
            return false;
        }
        let dungeon = run.dungeon.clone();
        self.store.runs.push(run);
        let mut theirs: Vec<Run> = self.store.runs.iter().filter(|r| r.dungeon == dungeon).cloned().collect();
        if theirs.len() > PER_DUNGEON {
            theirs.sort_by(rank);
            let keep: Vec<String> = theirs.iter().take(PER_DUNGEON).map(|r| r.id.clone()).collect();
            self.store.runs.retain(|r| r.dungeon != dungeon || keep.contains(&r.id));
        }
        true
    }

    pub fn save(&self) {
        let Some(p) = &self.path else { return };
        if let Ok(text) = toml::to_string(&self.store) {
            let tmp = p.with_extension("toml.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, p);
            }
        }
    }

    /// A dungeon's board: its runs in time, highest level first, then quickest.
    pub fn top(&self, dungeon: &str, n: usize) -> Vec<Run> {
        let mut runs: Vec<Run> = self.store.runs.iter().filter(|r| r.dungeon == dungeon && r.timed).cloned().collect();
        runs.sort_by(rank);
        runs.truncate(n);
        runs
    }

    /// This server's best in time on a dungeon at each level: (level, ms).
    pub fn bests_here(&self, dungeon: &str) -> Vec<(u32, u64)> {
        let mut best: std::collections::BTreeMap<u32, u64> = Default::default();
        for r in self.store.runs.iter().filter(|r| r.dungeon == dungeon && r.timed && r.host == self.host) {
            let e = best.entry(r.level).or_insert(r.ms);
            *e = (*e).min(r.ms);
        }
        best.into_iter().rev().collect()
    }

    /// Each dungeon's tier, for the ladder.
    pub fn set_tiers(&mut self, tiers: HashMap<String, u32>) {
        self.tiers = tiers;
    }

    /// The ladder: every hero with a run in time, by rating, highest first.
    pub fn ladder(&self) -> Vec<Rank> {
        // Each hero's best points in each dungeon (a hero is their name and camp).
        let mut best: HashMap<(String, String), (String, HashMap<String, f32>)> = HashMap::new();
        for r in self.store.runs.iter().filter(|r| r.timed) {
            let pts = run_points(r, self.tiers.get(&r.dungeon).copied().unwrap_or(1));
            for p in &r.party {
                let (name, class) = p.split_once(':').unwrap_or((p.as_str(), ""));
                let e = best.entry((name.to_string(), r.host.clone())).or_insert_with(|| (class.to_string(), HashMap::new()));
                let d = e.1.entry(r.dungeon.clone()).or_insert(0.0);
                *d = d.max(pts);
            }
        }
        let mut out: Vec<Rank> = best
            .into_iter()
            .map(|((name, host), (class, ds))| Rank { name, class, host, rating: ds.values().sum::<f32>().round() as u32, dungeons: ds.len() })
            .collect();
        out.sort_by(|a, b| b.rating.cmp(&a.rating).then(a.name.cmp(&b.name)));
        out
    }

    /// A hero's place on the ladder (from 1) and their rank, by name on this server.
    pub fn place_of(&self, name: &str) -> Option<(usize, Rank)> {
        self.ladder().into_iter().enumerate().find(|(_, r)| r.name == name && r.host == self.host).map(|(k, r)| (k + 1, r))
    }

    /// How many servers' runs are on the board.
    pub fn hosts(&self) -> usize {
        let mut h: Vec<&str> = self.store.runs.iter().map(|r| r.host.as_str()).collect();
        h.sort();
        h.dedup();
        h.len()
    }

    /// The host: one request (it's an HTTP one if it's here). Runs sent are taken; the board is
    /// sent back either way.
    pub fn serve(shared: &Shared, mut s: TcpStream) {
        let _ = s.set_read_timeout(Some(Duration::from_secs(4)));
        let _ = s.set_write_timeout(Some(Duration::from_secs(4)));
        let peer = s.peer_addr().map(|a| a.ip()).ok();
        let reply = match read_request(&mut s) {
            Some((method, path, body)) => {
                let mut b = shared.lock().unwrap();
                if b.remote.is_some() {
                    (404, "this server isn't the board host\n".to_string())
                } else if peer.is_some_and(|ip| !b.allow(ip)) {
                    (429, "slow down\n".to_string())
                } else if method == "POST" && path == "/wagers" {
                    let mut took = 0;
                    for line in body.lines().take(MAX_LINES) {
                        let dungeons = b.dungeons.clone();
                        if let Some(run) = Run::parse(line, &dungeons) {
                            if b.add(run) {
                                took += 1;
                            }
                        }
                    }
                    if took > 0 {
                        b.save();
                    }
                    (200, b.dump())
                } else if method == "GET" && path == "/wagers" {
                    (200, b.dump())
                } else {
                    (404, "no such page\n".to_string())
                }
            }
            None => (400, "bad request\n".to_string()),
        };
        let (code, text) = reply;
        let status = match code {
            200 => "OK",
            404 => "Not Found",
            429 => "Too Many Requests",
            _ => "Bad Request",
        };
        let _ = write!(s, "HTTP/1.0 {code} {status}\r\nContent-Type: text/plain; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{text}", text.len());
    }

    /// Whether an address may ask again now (a few times a minute).
    fn allow(&mut self, ip: IpAddr) -> bool {
        let now = Instant::now();
        if self.asks.len() > 4096 {
            self.asks.retain(|_, (t, _)| now.duration_since(*t) < Duration::from_secs(60));
        }
        let e = self.asks.entry(ip).or_insert((now, 0));
        if now.duration_since(e.0) >= Duration::from_secs(60) {
            *e = (now, 0);
        }
        e.1 += 1;
        e.1 <= PER_MINUTE
    }

    /// Every run on the board, a line each.
    fn dump(&self) -> String {
        let mut out = String::new();
        for r in &self.store.runs {
            out.push_str(&r.line());
            out.push('\n');
        }
        out
    }

    /// A server that isn't the host, now and then: its runs sent, everyone's fetched.
    pub fn sync(shared: &Shared) {
        let (remote, unsent) = {
            let b = shared.lock().unwrap();
            let Some(r) = b.remote.clone() else { return };
            (r, b.store.unsent.clone())
        };
        let body = unsent.iter().take(MAX_LINES).cloned().collect::<Vec<_>>().join("\n");
        let result = request(&remote, if unsent.is_empty() { "GET" } else { "POST" }, &body);
        let mut b = shared.lock().unwrap();
        match result {
            Some(text) => {
                let sent = unsent.len().min(MAX_LINES);
                b.store.unsent.drain(..sent);
                let dungeons = b.dungeons.clone();
                for line in text.lines() {
                    if let Some(run) = Run::parse(line, &dungeons) {
                        b.add(run);
                    }
                }
                b.save();
                b.synced = Some((Instant::now(), true));
            }
            None => b.synced = Some((Instant::now(), false)),
        }
    }

    /// Syncing in the background (a server that isn't the host): every minute or so.
    pub fn spawn_sync(shared: Shared) {
        if shared.lock().unwrap().remote.is_none() {
            return;
        }
        let _ = std::thread::Builder::new().name("board".into()).spawn(move || loop {
            Board::sync(&shared);
            std::thread::sleep(Duration::from_secs(60));
        });
    }
}

/// The board as kept in its file: empty if there's none yet. A file that can't be read (a hand
/// edit, a newer format, cut short) is moved aside to `<name>.bad` first, never written over:
/// on the board host it's everyone's runs.
fn load(path: &std::path::Path) -> Store {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Store::default(),
        Err(e) => return set_aside(path, &e.to_string()),
    };
    match toml::from_str::<Store>(&text) {
        Ok(s) => s,
        Err(e) => set_aside(path, &e.to_string()),
    }
}

fn set_aside(path: &std::path::Path, why: &str) -> Store {
    let mut bad = path.as_os_str().to_owned();
    bad.push(".bad");
    let bad = PathBuf::from(bad);
    match std::fs::rename(path, &bad) {
        Ok(()) => eprintln!("[survival] the wager board {} couldn't be read ({why}); kept as {}, starting a new one", path.display(), bad.display()),
        Err(e) => eprintln!("[survival] the wager board {} couldn't be read ({why}) or moved aside ({e})", path.display()),
    }
    Store::default()
}

/// The board's order: highest level first, then quickest, then earliest.
fn rank(a: &Run, b: &Run) -> std::cmp::Ordering {
    b.level.cmp(&a.level).then(a.ms.cmp(&b.ms)).then(a.at.cmp(&b.at))
}

/// Whether a connection's first bytes are an HTTP request's (for the login port to hand over).
pub fn is_http(first: &[u8]) -> bool {
    first.starts_with(b"GET ") || first.starts_with(b"POST")
}

/// A request: its method, path and body (none past the limits).
fn read_request(s: &mut TcpStream) -> Option<(String, String, String)> {
    let mut buf = Vec::new();
    let mut chunk = [0u8; 2048];
    let head_end = loop {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            return None;
        }
        buf.extend_from_slice(&chunk[..n]);
        if buf.len() > MAX_REQUEST {
            return None;
        }
        if let Some(k) = find(&buf, b"\r\n\r\n") {
            break k;
        }
    };
    let head = std::str::from_utf8(&buf[..head_end]).ok()?;
    let mut lines = head.split("\r\n");
    let mut first = lines.next()?.split(' ');
    let method = first.next()?.to_string();
    let path = first.next()?.to_string();
    let len: usize = lines
        .filter_map(|l| l.split_once(':'))
        .find(|(k, _)| k.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse().ok())
        .unwrap_or(0);
    if len > MAX_REQUEST {
        return None;
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < len {
        let n = s.read(&mut chunk).ok()?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&chunk[..n]);
        if body.len() > MAX_REQUEST {
            return None;
        }
    }
    body.truncate(len);
    Some((method, path, String::from_utf8(body).ok()?))
}

/// A request to the board host; its answer's body if it said OK.
fn request(remote: &str, method: &str, body: &str) -> Option<String> {
    use std::net::ToSocketAddrs;
    let addr = remote.to_socket_addrs().ok()?.next()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(5)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(10))).ok()?;
    s.set_write_timeout(Some(Duration::from_secs(10))).ok()?;
    write!(s, "{method} /wagers HTTP/1.0\r\nHost: {remote}\r\nContent-Type: text/plain\r\nContent-Length: {}\r\n\r\n{body}", body.len()).ok()?;
    let mut out = Vec::new();
    s.take(4 * 1024 * 1024).read_to_end(&mut out).ok()?;
    let text = String::from_utf8(out).ok()?;
    let (head, rest) = text.split_once("\r\n\r\n")?;
    head.starts_with("HTTP/1.0 200").then(|| rest.to_string()).or_else(|| head.starts_with("HTTP/1.1 200").then(|| rest.to_string()))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

/// HMAC-SHA1 of a line with the game's key, in hex.
fn sign(body: &str) -> String {
    let mut k = [0u8; 64];
    k[..KEY.len()].copy_from_slice(KEY);
    let mut inner = Sha1::new();
    inner.update(k.iter().map(|b| b ^ 0x36).collect::<Vec<_>>());
    inner.update(body.as_bytes());
    let mut outer = Sha1::new();
    outer.update(k.iter().map(|b| b ^ 0x5c).collect::<Vec<_>>());
    outer.update(inner.finalize());
    outer.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

/// A name as the board keeps it: letters, digits, spaces and a few marks, no longer than `max`.
pub fn name_only(s: &str, max: usize) -> String {
    s.chars().filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '\'' | '.' | '_' | ':')).take(max).collect()
}

fn title(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run() -> WagerRun {
        WagerRun { dungeon: "stockade".into(), name: "The Stockade".into(), level: 4, ms: 900_000, limit_ms: 1_500_000, deaths: 2, timed: true, party: vec![("Ann".into(), "warrior".into()), ("Bob|x".into(), "mage".into())] }
    }

    #[test]
    fn a_run_goes_out_as_a_line_and_comes_back_only_if_signed_and_sane() {
        let dungeons = vec!["stockade".to_string()];
        let r = Run::from_wager(&run(), "Dave's Mac");
        let line = r.line();
        let back = Run::parse(&line, &dungeons).expect("its own line");
        assert_eq!(back, r);
        assert_eq!(back.heroes(), "Ann (Warrior), Bobx (Mage)");
        // Changed after signing, of an unknown dungeon, or out of reason: refused.
        assert!(Run::parse(&line.replace("|4|900000|", "|9|900000|"), &dungeons).is_none());
        assert!(Run::parse(&line, &["deadmines".to_string()]).is_none());
        let mut fast = r.clone();
        fast.ms = 5_000;
        assert!(Run::parse(&fast.line(), &dungeons).is_none(), "too quick to be true");
        // In time is the clock's say.
        let mut slow = r.clone();
        slow.ms = 1_600_000;
        assert!(!Run::parse(&slow.line(), &dungeons).unwrap().timed);
    }

    #[test]
    fn a_board_file_that_cant_be_read_is_kept_aside_not_written_over() {
        let dir = std::env::temp_dir().join(format!("board-bad-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("wager-board.toml");
        std::fs::write(&path, "runs = [ this is not toml").unwrap();
        let b = Board::new(Some(path.clone()), "here", None, vec!["stockade".into()]);
        assert!(b.store.runs.is_empty());
        assert_eq!(std::fs::read_to_string(dir.join("wager-board.toml.bad")).unwrap(), "runs = [ this is not toml", "kept as it was");
        assert!(std::fs::read_to_string(&path).unwrap().contains("name = \"here\""), "a new one beside it");
        // A good file is read back, its runs and all.
        let mut b = b;
        b.record(&run());
        let again = Board::new(Some(path.clone()), "", None, vec!["stockade".into()]);
        assert_eq!(again.store.runs.len(), 1);
        assert_eq!(again.host, "here");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_ladder_sums_each_heros_best_in_each_dungeon_worth_more_in_a_harder_one() {
        let mut b = Board::new(None, "here", None, vec!["stockade".into(), "cathedral".into()]);
        b.set_tiers(HashMap::from([("stockade".to_string(), 2), ("cathedral".to_string(), 3)]));
        let mut a = run();
        // Ann and Bob at +4 in the Stockade with 40% to spare: (50 + 80 + 8) x 1.25.
        a.level = 4;
        a.ms = 900_000;
        let r = b.record(&a);
        assert!((run_points(&r, 2) - 172.5).abs() < 0.01, "{}", run_points(&r, 2));
        // A slower +2 there doesn't count over it; over time counts for nothing.
        a.level = 2;
        a.ms = 1_400_000;
        b.record(&a);
        a.level = 9;
        a.ms = 1_600_000;
        a.timed = false;
        b.record(&a);
        // Ann alone at +1 in the Cathedral, right on time: 70 x 1.5.
        let solo = WagerRun { dungeon: "cathedral".into(), name: "The Cathedral".into(), level: 1, ms: 1_200_000, limit_ms: 1_200_000, deaths: 0, timed: true, party: vec![("Ann".into(), "warrior".into())] };
        b.record(&solo);
        let ladder = b.ladder();
        assert_eq!(ladder.iter().map(|r| (r.name.as_str(), r.rating, r.dungeons)).collect::<Vec<_>>(), vec![("Ann", 278, 2), ("Bobx", 173, 1)]);
        assert_eq!(ladder[0].class, "warrior");
        let (place, me) = b.place_of("Bobx").unwrap();
        assert_eq!((place, me.rating), (2, 173));
        assert!(b.place_of("Nobody").is_none());
    }

    #[test]
    fn the_board_keeps_the_highest_then_the_quickest_and_each_run_once() {
        let mut b = Board::new(None, "here", None, vec!["stockade".into()]);
        let mut a = run();
        a.level = 3;
        a.ms = 600_000;
        let first = b.record(&a);
        a.level = 5;
        a.ms = 1_400_000;
        b.record(&a);
        a.level = 5;
        a.ms = 1_000_000;
        b.record(&a);
        let top = b.top("stockade", 10);
        assert_eq!(top.iter().map(|r| (r.level, r.ms)).collect::<Vec<_>>(), vec![(5, 1_000_000), (5, 1_400_000), (3, 600_000)]);
        assert!(!b.add(first), "once");
        assert_eq!(b.bests_here("stockade"), vec![(5, 1_000_000), (3, 600_000)]);
    }
}
