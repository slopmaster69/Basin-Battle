//! What Play does behind the menu: the project's names and servers (project.toml), the saved
//! choices, finding WoW 1.12.1, asking each server whether it's up, the update, starting this PC's
//! own server and the game.
//!
//! It runs from one of two folders (`Kind`):
//! - the Windows download: it does what launcher\play.ps1 and common.ps1 do, the same way and
//!   with the same files (settings.txt, update.ps1, Host.exe, benilla.exe), so the two launchers
//!   can be used in turn on one PC. A change to how one finds WoW, reads settings.txt or picks a
//!   server belongs in both;
//! - this repository, on a Mac (or Linux): it plays through the scripts a Mac already uses
//!   (scripts/play.sh builds the client when its patches changed, Host Basin Battle.command hosts)
//!   and keeps its choices in settings.sh beside theirs. Updates are `git pull` of main.
//!
//! The menu sends it {"do": ...} and is told what happened as {"t": ...} (see Backend::handle and
//! the `emit` calls); every slow step runs on its own thread.

use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::net::{TcpStream, ToSocketAddrs};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

pub type Emit = Arc<dyn Fn(Value) + Send + Sync>;
pub type Quit = Arc<dyn Fn() + Send + Sync>;

// --- where things are -------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The Windows download: Play.exe, launcher\, benilla.exe and server\ side by side.
    Download,
    /// A clone of this repository (how a Mac plays): scripts/, dist/, benilla/ built in place.
    Checkout,
}

pub struct Paths {
    pub kind: Kind,
    pub root: PathBuf,
    pub launcher: PathBuf,
    /// The menu's files, and where a file it asks for that isn't there may be instead (the shield).
    pub ui: PathBuf,
    pub art: PathBuf,
    pub project: PathBuf,
    pub exe: PathBuf,
    pub server_exe: PathBuf,
    pub settings: PathBuf,
    pub version: PathBuf,
    pub log: PathBuf,
}

impl Paths {
    pub fn download(root: PathBuf) -> Self {
        let launcher = root.join("launcher");
        Paths {
            kind: Kind::Download,
            ui: launcher.join("ui"),
            art: launcher.clone(),
            project: launcher.join("project.toml"),
            exe: root.join("benilla.exe"),
            server_exe: root.join("server").join("htl-server.exe"),
            settings: root.join("settings.txt"),
            version: launcher.join("version.txt"),
            log: launcher.join("play.log"),
            launcher,
            root,
        }
    }
    pub fn checkout(root: PathBuf) -> Self {
        let launcher = root.join("dist").join("windows").join("launcher");
        Paths {
            kind: Kind::Checkout,
            ui: launcher.join("ui"),
            art: root.join("dist").join("windows").join("art"),
            project: root.join("project.toml"),
            exe: root.join("benilla").join("target").join("release").join("benilla"),
            server_exe: root.join("target").join("release").join("htl-server"),
            settings: root.join("settings.sh"),
            version: launcher.join("version.txt"),
            log: root.join("logs").join("launcher.log"),
            launcher,
            root,
        }
    }
    /// Where Play is: the download beside Play.exe, or the checkout it was built in (Play is
    /// then somewhere under it, in target\ or in Basin Battle.app). BB_ROOT names it outright.
    pub fn find(exe: &Path) -> Option<Self> {
        if let Some(r) = std::env::var_os("BB_ROOT") {
            let r = PathBuf::from(r);
            return Some(if r.join("launcher").join("ui").join("index.html").is_file() { Paths::download(r) } else { Paths::checkout(r) });
        }
        let dir = exe.parent()?;
        if dir.join("launcher").join("ui").join("index.html").is_file() {
            return Some(Paths::download(dir.to_path_buf()));
        }
        for a in dir.ancestors() {
            if a.join("project.toml").is_file() && a.join("scripts").join("play.sh").is_file()
                && a.join("dist").join("windows").join("launcher").join("ui").join("index.html").is_file() {
                return Some(Paths::checkout(a.to_path_buf()));
            }
        }
        None
    }
}

// --- the names players see, and the group's servers (launcher\project.toml) --------------------

#[derive(Clone, Debug)]
pub struct Server {
    pub key: String,
    pub address: String,
    pub name: String,
    pub pc: String,
}

pub struct Project {
    pub title: String,
    pub survival: String,
    pub basin: String,
    pub repository: String,
    pub servers: Vec<Server>,
}

/// `key = "value"` from project.toml, read as common.ps1's ProjectName reads it: the first line
/// for that key with a value in it.
fn project_value(text: &str, key: &str) -> Option<String> {
    for line in text.lines() {
        let Some(rest) = line.trim_start().strip_prefix(key) else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('=') else { continue };
        let Some(rest) = rest.trim_start().strip_prefix('"') else { continue };
        let Some(end) = rest.find('"') else { continue };
        if end > 0 {
            return Some(rest[..end].to_string());
        }
    }
    None
}

impl Project {
    pub fn read(file: &Path) -> Self {
        let text = read_text(file).unwrap_or_default();
        let get = |k: &str, d: &str| project_value(&text, k).unwrap_or_else(|| d.to_string());
        let mut servers = Vec::new();
        for k in ["server", "backup", "backup2"] {
            let address = get(k, "");
            if !address.is_empty() {
                servers.push(Server {
                    key: k.into(),
                    name: get(&format!("{k}_name"), &address),
                    pc: get(&format!("{k}_pc"), ""),
                    address,
                });
            }
        }
        Project {
            title: get("title", "Basin Battle"),
            survival: get("survival", "Elwynn Survival"),
            basin: get("basin", "Basin Battle"),
            repository: get("repository", "slopmaster69/Basin-Battle"),
            servers,
        }
    }
    fn server(&self, key: &str) -> Option<&Server> {
        self.servers.iter().find(|s| s.key == key)
    }
    fn game_name(&self, game: &str) -> &str {
        if game == "basin" { &self.basin } else { &self.survival }
    }
}

/// A text file as Windows PowerShell may have written it: UTF-8 (with or without its mark), or
/// the PC's own code page, read here as Latin-1.
fn read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&bytes);
    Some(match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    })
}

// --- settings.txt: one "key = value" a line (common.ps1's Read-Settings and Save-Settings) ------

pub type Settings = BTreeMap<String, String>;

pub fn read_settings(file: &Path) -> Settings {
    let mut s = Settings::new();
    let Some(text) = read_text(file) else { return s };
    let lines: Vec<&str> = text.lines().collect();
    let is_pair = |l: &str| {
        let l = l.trim_start();
        l.starts_with('#') || {
            let word: String = l.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
            !word.is_empty() && l[word.len()..].trim_start().starts_with('=')
        }
    };
    // An older launcher's settings.txt: the Data folder on line 1, the address on line 2.
    if let Some(first) = lines.first() {
        if !is_pair(first) {
            if !first.trim().is_empty() { s.insert("wow".into(), first.trim().into()); }
            if let Some(second) = lines.get(1) {
                if !second.trim().is_empty() { s.insert("address".into(), second.trim().into()); }
            }
            return s;
        }
    }
    for l in lines {
        let l = l.trim();
        if l.starts_with('#') { continue; }
        if let Some((k, v)) = l.split_once('=') {
            let k = k.trim();
            if !k.is_empty() && k.chars().all(|c| c.is_alphanumeric() || c == '_') {
                s.insert(k.into(), v.trim().into());
            }
        }
    }
    s
}

pub fn save_settings(file: &Path, s: &Settings) {
    let mut keys: Vec<&String> = s.keys().filter(|k| !s[*k].is_empty()).collect();
    keys.sort_by_key(|k| k.to_lowercase());
    let mut out = String::from("\u{FEFF}# Your choices, kept across updates. Delete this file to be asked again.\r\n");
    for k in keys {
        // One line each, whatever was typed or pasted: a line break in a value would otherwise
        // write a second setting.
        let v: String = s[k].chars().filter(|c| !c.is_control()).collect();
        out.push_str(&format!("{k} = {}\r\n", v.trim()));
    }
    // With its UTF-8 mark, so the plain launcher (Windows PowerShell) reads a folder name with
    // accents in it as it is.
    let _ = fs::write(file, out);
}

// --- settings.sh: the Mac scripts' settings (common.sh's save_setting), with Play's beside them --

/// settings.txt's names for what Play keeps, and settings.sh's.
const SH_NAMES: [(&str, &str); 8] = [
    ("wow", "WOW_DATA"), ("game", "HTL_LAUNCHER_GAME"), ("place", "HTL_LAUNCHER_PLACE"), ("address", "HTL_LAUNCHER_ADDRESS"),
    ("launcher_fx", "HTL_LAUNCHER_FX"), ("launcher_still", "HTL_LAUNCHER_STILL"), ("launcher_keep", "HTL_LAUNCHER_KEEP"), ("launcher_backups", "HTL_LAUNCHER_BACKUPS"),
];

/// settings.sh read the way the scripts read it (bash sources it), so any quoting they wrote reads back.
pub fn read_settings_sh(file: &Path) -> Settings {
    let mut s = Settings::new();
    if !file.is_file() { return s; }
    let names: Vec<&str> = SH_NAMES.iter().map(|(_, n)| *n).collect();
    let script = format!(". \"$1\" >/dev/null 2>&1; for n in {}; do printf '%s=%s\\0' \"$n\" \"${{!n}}\"; done", names.join(" "));
    let Ok(out) = Command::new("bash").args(["-c", &script, "settings"]).arg(file).stderr(Stdio::null()).output() else { return s };
    for pair in String::from_utf8_lossy(&out.stdout).split('\0') {
        if let Some((n, v)) = pair.split_once('=') {
            if let Some((k, _)) = SH_NAMES.iter().find(|(_, sh)| *sh == n) {
                if !v.is_empty() { s.insert((*k).into(), v.into()); }
            }
        }
    }
    s
}

/// Play's lines in settings.sh, rewritten; the scripts' own lines kept as they are.
pub fn save_settings_sh(file: &Path, s: &Settings) {
    let mut lines: Vec<String> = fs::read_to_string(file).unwrap_or_default().lines()
        .filter(|l| !SH_NAMES.iter().any(|(_, n)| l.starts_with(&format!("{n}="))))
        .map(String::from).collect();
    for (k, n) in SH_NAMES {
        if let Some(v) = s.get(k) {
            let v: String = v.chars().filter(|c| !c.is_control()).collect();
            if !v.trim().is_empty() { lines.push(format!("{n}='{}'", v.trim().replace('\'', "'\\''"))); }
        }
    }
    let _ = fs::write(file, lines.join("\n") + "\n");
}

// --- addresses and ports ------------------------------------------------------------------------

/// "host:port", or "host" (port 3724).
pub fn split_address(address: &str) -> (String, u16) {
    if let Some((h, p)) = address.rsplit_once(':') {
        if !h.is_empty() && !h.contains(':') && !p.is_empty() && p.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(p) = p.parse() { return (h.into(), p); }
        }
    }
    (address.into(), 3724)
}

/// An address on a Tailscale network: 100.64.0.0/10, or a MagicDNS name (*.ts.net).
pub fn is_tailscale(address: &str) -> bool {
    let (h, _) = split_address(address);
    if h.to_lowercase().ends_with(".ts.net") { return true; }
    let parts: Vec<&str> = h.split('.').collect();
    parts.len() == 4 && parts[0] == "100" && parts.iter().all(|p| p.parse::<u8>().is_ok())
        && (64..=127).contains(&parts[1].parse::<u8>().unwrap_or(0))
}

/// Whether something answers on host:port within `ms`.
pub fn reach(host: &str, port: u16, ms: u64) -> bool {
    let end = Instant::now() + Duration::from_millis(ms);
    let Ok(addrs) = (host, port).to_socket_addrs() else { return false };
    for a in addrs {
        let left = end.saturating_duration_since(Instant::now());
        if left.is_zero() { break; }
        if TcpStream::connect_timeout(&a, left).is_ok() { return true; }
    }
    false
}

#[derive(Clone, Copy)]
pub struct Ports { pub login: u16, pub basin: u16, pub survival: u16 }

/// This PC's own server's ports (common.ps1's Own-Ports): 3724 and 8085/8086; on the PC that hosts
/// one of the group's servers, that server's; and over either, settings.txt's ports.
pub fn own_ports(s: &Settings, project: &Project, this_pc: &str) -> Ports {
    let mut p = Ports { login: 3724, basin: 8085, survival: 8086 };
    if let Some(mine) = project.servers.iter().find(|x| !x.pc.is_empty() && x.pc.eq_ignore_ascii_case(this_pc)) {
        let login = split_address(&mine.address).1;
        let step = 2 * (login as i32 - 3724);
        p = Ports { login, basin: (8085 + step) as u16, survival: (8086 + step) as u16 };
    }
    let get = |k: &str| s.get(k).and_then(|v| v.parse::<u16>().ok()).filter(|v| *v > 0);
    if let Some(v) = get("login_port") { p.login = v; }
    if let Some(v) = get("world_port") { p.basin = v; }
    if let Some(v) = get("survival_port") { p.survival = v; }
    p
}

// --- the WoW 1.12.1 install (common.ps1's DataFolderOf and Find-WowData) ------------------------

/// The Data folder, if `dir` is it or the WoW folder holding it (with dbc.MPQ in it: modern WoW
/// Classic has no MPQs, so it never matches).
pub fn data_folder_of(dir: &str) -> Option<PathBuf> {
    let d = dir.trim().trim_matches('"');
    if d.is_empty() { return None; }
    let p = PathBuf::from(d);
    if p.join("dbc.MPQ").is_file() { return Some(p); }
    let inner = p.join("Data");
    if inner.join("dbc.MPQ").is_file() { return Some(inner); }
    None
}

/// Where people keep it: the usual folder names on every local drive, Program Files, the
/// desktop, Documents, Downloads and Games folders, one level into any folder whose name says WoW.
pub fn find_wow_data(root: &Path) -> Option<PathBuf> {
    let mut places: Vec<PathBuf> = Vec::new();
    // On a Mac, common.sh's own places first (beside this folder, in it, in Applications).
    #[cfg(not(windows))]
    {
        for p in [root.join("WoW").join("Data"), root.join("..").join("WoW Classic").join("WoW-1.12.1").join("Data")] {
            if let Some(found) = data_folder_of(&p.to_string_lossy()) { return Some(found); }
        }
        places.push(root.to_path_buf());
        if let Some(parent) = root.parent() { places.push(parent.to_path_buf()); }
        places.push(PathBuf::from("/Applications"));
        if let Some(h) = dirs::home_dir() { places.push(h.join("Applications")); }
    }
    #[cfg(windows)]
    let _ = root;
    for d in fixed_drives() {
        places.push(d.clone());
        places.push(d.join("Games"));
    }
    let home = dirs::home_dir();
    if let Some(h) = &home { places.push(h.clone()); }
    if let Some(d) = dirs::desktop_dir() { places.push(d); }
    if let Some(d) = dirs::document_dir() { places.push(d); }
    if let Some(h) = &home {
        places.push(h.join("Downloads"));
        places.push(h.join("Games"));
    }
    for v in ["ProgramFiles", "ProgramFiles(x86)"] {
        if let Some(p) = std::env::var_os(v) { places.push(PathBuf::from(p)); }
    }
    let wowish = |name: &str| {
        let n = name.to_lowercase();
        ["wow", "warcraft", "1.12", "vanilla", "azeroth"].iter().any(|w| n.contains(w))
    };
    let subdirs = |p: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = fs::read_dir(p).map(|r| r.flatten().filter(|e| e.path().is_dir()).map(|e| e.path()).collect()).unwrap_or_default();
        v.sort_by_key(|p| p.to_string_lossy().to_lowercase());
        v
    };
    for place in places {
        if !place.is_dir() { continue; }
        for f in subdirs(&place) {
            if !f.file_name().map(|n| wowish(&n.to_string_lossy())).unwrap_or(false) { continue; }
            if let Some(found) = data_folder_of(&f.to_string_lossy()) { return Some(found); }
            for inner in subdirs(&f) {
                if let Some(found) = data_folder_of(&inner.to_string_lossy()) { return Some(found); }
            }
        }
    }
    None
}

/// Local drives only: a network drive could keep the search waiting.
#[cfg(windows)]
fn fixed_drives() -> Vec<PathBuf> {
    use windows_sys::Win32::Storage::FileSystem::{GetDriveTypeW, GetLogicalDrives};
    let mask = unsafe { GetLogicalDrives() };
    (0..26u8)
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| format!("{}:\\", (b'A' + i) as char))
        .filter(|root| {
            let w: Vec<u16> = root.encode_utf16().chain([0]).collect();
            unsafe { GetDriveTypeW(w.as_ptr()) == 3 } // DRIVE_FIXED
        })
        .map(PathBuf::from)
        .collect()
}
#[cfg(not(windows))]
fn fixed_drives() -> Vec<PathBuf> { Vec::new() }

// --- other programs -------------------------------------------------------------------------------

#[cfg(windows)]
fn quiet(c: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    c.creation_flags(0x0800_0000) // CREATE_NO_WINDOW
}
#[cfg(not(windows))]
fn quiet(c: &mut Command) -> &mut Command { c }

/// Whether the game is open (a process called benilla.exe).
#[cfg(windows)]
fn game_running() -> bool {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap == INVALID_HANDLE_VALUE { return false; }
        let mut e: PROCESSENTRY32W = std::mem::zeroed();
        e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut found = false;
        let mut ok = Process32FirstW(snap, &mut e) != 0;
        while ok {
            let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
            if String::from_utf16_lossy(&e.szExeFile[..len]).eq_ignore_ascii_case("benilla.exe") { found = true; break; }
            ok = Process32NextW(snap, &mut e) != 0;
        }
        CloseHandle(snap);
        found
    }
}
#[cfg(not(windows))]
fn game_running() -> bool {
    Command::new("pgrep").args(["-x", "benilla"]).stdout(Stdio::null()).stderr(Stdio::null()).status().map(|s| s.success()).unwrap_or(false)
}

#[cfg(windows)]
fn tailscale_exe() -> PathBuf {
    let pf = std::env::var_os("ProgramFiles").map(PathBuf::from).unwrap_or_else(|| PathBuf::from(r"C:\Program Files"));
    pf.join("Tailscale").join("tailscale.exe")
}
/// The Mac app's own command line, or `tailscale` on the PATH (as host.sh finds it).
#[cfg(not(windows))]
fn tailscale_exe() -> PathBuf {
    let app = PathBuf::from("/Applications/Tailscale.app/Contents/MacOS/Tailscale");
    if app.is_file() { return app; }
    for dir in std::env::var("PATH").unwrap_or_default().split(':').chain(["/usr/local/bin", "/opt/homebrew/bin", "/usr/bin"]) {
        let p = Path::new(dir).join("tailscale");
        if p.is_file() { return p; }
    }
    app
}
fn tailscale_installed() -> bool { tailscale_exe().is_file() }
const TAILSCALE_DOWNLOAD: &str = if cfg!(target_os = "macos") { "https://tailscale.com/download/mac" } else if cfg!(windows) { "https://tailscale.com/download/windows" } else { "https://tailscale.com/download/linux" };
fn tailscale_up() -> bool {
    quiet(Command::new(tailscale_exe()).arg("status").stdout(Stdio::null()).stderr(Stdio::null()))
        .status().map(|s| s.success()).unwrap_or(false)
}

/// Why a server can't be reached, in words a player can act on (common.ps1's Why-Unreachable).
fn why_unreachable(address: &str) -> String {
    if is_tailscale(address) {
        if !tailscale_installed() {
            return "This server is on the group's Tailscale network, and Tailscale isn't on this PC yet. Install it from the page that just opened, sign in with the account the host invited, then press Play again.".into();
        }
        if !tailscale_up() {
            return if cfg!(target_os = "macos") { "Tailscale is off or signed out. Open it (its icon is in the menu bar), log in and connect, then try again." } else { "Tailscale is off or signed out. Open it (its icon is by the clock), log in and connect, then try again." }.into();
        }
        return format!("Tailscale is on, but the server at {address} doesn't answer. Is the host's server running? If you were only just invited, the host may need to share their PC with you in Tailscale.");
    }
    format!("The server at {address} doesn't answer. Is the host's server running, and is the address right?")
}

pub fn open_in_shell(target: &str) {
    #[cfg(windows)]
    let mut c = Command::new("explorer");
    #[cfg(target_os = "macos")]
    let mut c = Command::new("open");
    #[cfg(not(any(windows, target_os = "macos")))]
    let mut c = Command::new("xdg-open");
    let _ = c.arg(target).spawn();
}

// --- the backend ------------------------------------------------------------------------------------

pub struct Options {
    pub change: bool,
    pub no_update: bool,
    pub game: Option<String>,
    pub bench: bool,
}

pub struct Backend {
    pub paths: Paths,
    pub project: Project,
    this_pc: String,
    emit: Emit,
    quit: Quit,
    opts: Options,
    settings: Mutex<Settings>,
    updating: AtomicBool,
    playing: AtomicBool,
    started: Instant,
}

/// The menu calls the games elwynn and basin; settings.txt calls them survival and basin.
fn page_game(g: &str) -> &'static str { if g == "basin" { "basin" } else { "elwynn" } }
fn saved_game(g: &str) -> &'static str { if g == "basin" { "basin" } else { "survival" } }

impl Backend {
    pub fn new(paths: Paths, opts: Options, emit: Emit, quit: Quit) -> Arc<Self> {
        let project = Project::read(&paths.project);
        let settings = match paths.kind { Kind::Download => read_settings(&paths.settings), Kind::Checkout => read_settings_sh(&paths.settings) };
        let this_pc = std::env::var("COMPUTERNAME").unwrap_or_default();
        if let Some(d) = paths.log.parent() { let _ = fs::create_dir_all(d); }
        let b = Arc::new(Backend {
            paths, project, this_pc, emit, quit, opts,
            settings: Mutex::new(settings),
            updating: AtomicBool::new(false),
            playing: AtomicBool::new(false),
            started: Instant::now(),
        });
        let _ = fs::write(&b.paths.log, format!("Play (the War Table), {}\r\n", b.version()));
        b
    }

    pub fn log(&self, text: &str) {
        use std::io::Write;
        if let Ok(mut f) = fs::OpenOptions::new().append(true).create(true).open(&self.paths.log) {
            let _ = write!(f, "{:>7.1}s {}\r\n", self.started.elapsed().as_secs_f32(), text);
        }
    }

    fn version(&self) -> String {
        if self.paths.kind == Kind::Checkout { return self.git(&["describe", "--tags", "--always"]).unwrap_or_default(); }
        read_text(&self.paths.version).and_then(|t| t.lines().next().map(|l| l.trim().to_string())).unwrap_or_default()
    }

    /// git in the checkout: its output, if it worked.
    fn git(&self, args: &[&str]) -> Option<String> {
        let out = Command::new("git").arg("-C").arg(&self.paths.root).args(args).stdin(Stdio::null()).stderr(Stdio::null()).output().ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }

    fn save(&self, f: impl FnOnce(&mut Settings)) {
        let mut s = self.settings.lock().unwrap();
        f(&mut s);
        match self.paths.kind { Kind::Download => save_settings(&self.paths.settings, &s), Kind::Checkout => save_settings_sh(&self.paths.settings, &s) }
    }

    /// own, address, or one of project.toml's servers.
    fn known_place(&self, p: &str) -> bool { p == "own" || p == "address" || self.project.server(p).is_some() }

    fn here(&self, s: &Server) -> bool { !s.pc.is_empty() && s.pc.eq_ignore_ascii_case(&self.this_pc) }

    fn say(&self, t: &str, stage: &str, now: &str, then: &str) {
        self.log(&format!("{t} {stage}: {now} {then}"));
        (self.emit)(json!({ "t": t, "stage": stage, "now": now, "then": then }));
    }

    /// A message from the menu.
    pub fn handle(self: &Arc<Self>, m: &Value) {
        let s = |k: &str| m.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
        match m.get("do").and_then(|v| v.as_str()).unwrap_or("") {
            "hello" => self.hello(),
            "check" => { let b = self.clone(); thread::spawn(move || b.check_servers()); }
            "choose" => {
                let (game, place, address) = (s("game"), s("place"), s("address"));
                let place = if self.known_place(&place) { place } else { String::new() };
                self.save(|st| {
                    if !game.is_empty() { st.insert("game".into(), saved_game(&game).into()); }
                    if !place.is_empty() { st.insert("place".into(), place.clone()); }
                    if place == "address" { st.insert("address".into(), address.trim().into()); }
                });
            }
            "pref" => {
                let (k, v) = (s("key"), m.get("value").and_then(|v| v.as_bool()).unwrap_or(false));
                if ["fx", "still", "keep", "backups"].contains(&k.as_str()) {
                    self.save(|st| { st.insert(format!("launcher_{k}"), if v { "on" } else { "off" }.into()); });
                }
            }
            "play" => {
                if self.playing.swap(true, Ordering::SeqCst) { return; }
                let b = self.clone();
                let (game, place, address, solo) = (s("game"), s("place"), s("address"), m.get("solo").and_then(|v| v.as_bool()).unwrap_or(false));
                let place = if self.known_place(&place) { place } else { "own".to_string() };
                thread::spawn(move || {
                    b.play(&game, &place, &address, solo);
                    b.playing.store(false, Ordering::SeqCst);
                });
            }
            "update" => { let b = self.clone(); thread::spawn(move || b.update(true)); }
            "host" => { self.open_host(); }
            "folder" => open_in_shell(&self.paths.root.to_string_lossy()),
            "url" => {
                let href = s("href");
                if href.starts_with("https://github.com/") || href.starts_with("https://tailscale.com/") { open_in_shell(&href); }
            }
            // What the menu noticed about this PC (its GPU), for play.log.
            "diag" => { let w: String = s("what").chars().filter(|c| !c.is_control()).take(400).collect(); self.log(&format!("menu: {w}")); }
            other => self.log(&format!("the menu sent something unknown: {other}")),
        }
    }

    /// What the menu needs to draw itself, then the slow look-ups.
    fn hello(self: &Arc<Self>) {
        let s = self.settings.lock().unwrap().clone();
        let mut game = s.get("game").cloned().unwrap_or_default();
        if let Some(g) = &self.opts.game { game = if g.to_lowercase().contains("basin") { "basin".into() } else { "survival".into() }; }
        // An older launcher's "group" is the first server; a server project.toml no longer names
        // is forgotten; a first run goes to the first of the group's servers.
        let mut place = s.get("place").cloned().unwrap_or_default();
        if place == "group" { place = "server".into(); }
        if !place.is_empty() && place != "own" && place != "address" && self.project.server(&place).is_none() { place.clear(); }
        if place.is_empty() { place = self.project.servers.first().map(|x| x.key.clone()).unwrap_or_else(|| "own".into()); }
        let pref = |k: &str, d: bool| s.get(&format!("launcher_{k}")).map(|v| v == "on").unwrap_or(d);
        let servers: Vec<Value> = self.project.servers.iter().map(|x| json!({
            "key": x.key, "name": x.name, "address": x.address, "tailscale": is_tailscale(&x.address), "here": self.here(x),
        })).collect();
        let wow = data_folder_of(s.get("wow").map(String::as_str).unwrap_or(""));
        (self.emit)(json!({
            "t": "hello",
            "title": self.project.title, "repository": self.project.repository,
            "games": { "elwynn": self.project.survival, "basin": self.project.basin },
            "version": self.version(),
            "servers": servers, "place": place, "address": s.get("address").cloned().unwrap_or_default(),
            "game": page_game(&game), "first": game.is_empty(),
            "wow": wow.as_ref().map(|p| p.to_string_lossy().to_string()),
            "prefs": { "fx": pref("fx", true), "still": pref("still", false), "keep": pref("keep", false), "backups": pref("backups", true) },
            "tailscale": tailscale_installed(),
            "hostable": match self.paths.kind { Kind::Download => self.paths.server_exe.is_file(), Kind::Checkout => self.paths.root.join("scripts").join("host.sh").is_file() },
            "platform": if cfg!(target_os = "macos") { "mac" } else if cfg!(windows) { "windows" } else { "linux" },
            "checkout": self.paths.kind == Kind::Checkout,
            "change": self.opts.change, "bench": self.opts.bench,
        }));
        self.log(&format!("menu open: {} on {place}", page_game(&game)));
        if wow.is_none() {
            let b = self.clone();
            thread::spawn(move || b.look_for_wow());
        }
        let b = self.clone();
        thread::spawn(move || b.check_servers());
        if !self.opts.no_update {
            let b = self.clone();
            thread::spawn(move || b.update(false));
        }
    }

    fn look_for_wow(&self) {
        (self.emit)(json!({ "t": "wow", "searching": true }));
        let found = find_wow_data(&self.paths.root);
        if let Some(p) = &found {
            self.log(&format!("found WoW: {}", p.display()));
            let p = p.to_string_lossy().to_string();
            self.save(|s| { s.insert("wow".into(), p); });
        }
        (self.emit)(json!({ "t": "wow", "path": found.map(|p| p.to_string_lossy().to_string()) }));
    }

    /// A folder picked by hand: its Data folder, if it has one.
    pub fn picked_wow(&self, dir: &Path) -> bool {
        match data_folder_of(&dir.to_string_lossy()) {
            Some(p) => {
                let p = p.to_string_lossy().to_string();
                self.save(|s| { s.insert("wow".into(), p.clone()); });
                (self.emit)(json!({ "t": "wow", "path": p }));
                true
            }
            None => {
                (self.emit)(json!({ "t": "wow", "path": null, "wrong": dir.to_string_lossy() }));
                false
            }
        }
    }

    /// Each of the group's servers, all at once: up, down, this PC's own, or needing Tailscale.
    fn check_servers(self: &Arc<Self>) {
        let ts = tailscale_installed();
        for srv in self.project.servers.clone() {
            let b = self.clone();
            thread::spawn(move || {
                let state = if b.here(&srv) {
                    "here"
                } else if is_tailscale(&srv.address) && !ts {
                    "tailscale"
                } else {
                    (b.emit)(json!({ "t": "server", "key": srv.key, "state": "checking" }));
                    let (h, p) = split_address(&srv.address);
                    if reach(&h, p, 4000) { "up" } else { "down" }
                };
                (b.emit)(json!({ "t": "server", "key": srv.key, "state": state }));
            });
        }
    }

    /// launcher\update.ps1, as the plain launcher runs it. 0: updated (the new Play takes over),
    /// 2: nothing newer, 3: not now, 1: it didn't work (nothing changed).
    fn update(self: &Arc<Self>, asked: bool) {
        if self.paths.kind == Kind::Checkout { return self.git_update(asked); }
        let version = self.version();
        if !version.starts_with('v') {
            // A branch's test build (or a folder built by hand) is never updated over.
            if asked { self.say("update", "none", "This is a test build.", "It doesn't update itself. Download a release for updates."); }
            return;
        }
        if self.updating.swap(true, Ordering::SeqCst) { return; }
        self.say("update", "checking", "Checking for a new version…", "");
        let child = quiet(Command::new("powershell.exe")
            .args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(self.paths.launcher.join("update.ps1"))
            .arg("-Quiet")
            .current_dir(&self.paths.root)
            .stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::null()))
            .spawn();
        let mut child = match child {
            Ok(c) => c,
            Err(e) => {
                self.log(&format!("update: couldn't start PowerShell: {e}"));
                if asked { self.say("update", "failed", "Couldn't check for a new version.", "") } else { self.say("update", "none", "", "") }
                self.updating.store(false, Ordering::SeqCst);
                return;
            }
        };
        let mut last = String::new();
        if let Some(out) = child.stdout.take() {
            for line in BufReader::new(out).lines().map_while(Result::ok) {
                let line = line.trim().to_string();
                if line.is_empty() { continue; }
                let stage = if line.starts_with("Downloading") { "downloading" } else if line.starts_with("Updated to") { "installing" } else { "working" };
                self.say("update", stage, &line, "Your settings and heroes stay as they are.");
                last = line;
            }
        }
        let code = child.wait().ok().and_then(|s| s.code()).unwrap_or(1);
        self.log(&format!("update: {code}"));
        match code {
            0 => {
                let now = self.version();
                self.say("update", "done", &format!("Updated to {now}. Starting it…"), "");
                // The new version's own Play takes it from here (this one was renamed Play.exe.old).
                let mut c = Command::new(self.paths.root.join("Play.exe"));
                c.arg("-NoUpdate").current_dir(&self.paths.root);
                if let Some(g) = self.settings.lock().unwrap().get("game") { c.args(["-Game", g]); }
                if c.spawn().is_ok() {
                    thread::sleep(Duration::from_millis(600));
                    (self.quit)();
                }
            }
            2 => self.say("update", "none", if asked { "You have the newest version." } else { "" }, ""),
            3 => self.say("update", "later", &last, ""),
            _ => self.say("update", "failed", if last.is_empty() { "The update didn't work. Nothing was changed." } else { &last }, ""),
        }
        self.updating.store(false, Ordering::SeqCst);
    }

    /// This PC's own server, started (Host.exe, in its own window) if it isn't running. Its
    /// address for the game.
    fn start_own(&self, world: u16, own: Ports) -> Result<String, (String, &'static str)> {
        let up = || reach("127.0.0.1", own.login, 500) && reach("127.0.0.1", world, 500);
        if !up() {
            if self.paths.kind == Kind::Download && !self.paths.server_exe.is_file() {
                return Err(("This download has no server in it (server\\htl-server.exe). Download the newest version, or play on someone else's server.".into(), ""));
            }
            self.say("play", "own", "Starting your server…", "It opens in its own window. Leave that open while you play.");
            let started = self.open_host();
            if !started { return Err(("Your server didn't start. Run Host to see why.".into(), "")); }
            let t0 = Instant::now();
            let mut said = 0;
            while !up() {
                thread::sleep(Duration::from_millis(1000));
                let s = t0.elapsed().as_secs();
                if s / 10 > said {
                    said = s / 10;
                    self.say("play", "own", &format!("Your server is loading the world ({s} sec)."), "Its window shows how it's going.");
                }
                if s >= 1200 { return Err(("Your server didn't start in 20 min. Its window says what went wrong.".into(), "")); }
            }
        }
        Ok(if own.login != 3724 { format!("127.0.0.1:{}", own.login) } else { "127.0.0.1".into() })
    }

    /// Play: the server (the chosen one, then each after it until one answers), then the game.
    fn play(&self, game: &str, place: &str, address: &str, solo: bool) {
        let game = saved_game(game);
        let fail = |now: &str, offer: &str| {
            self.log(&format!("play failed: {now}"));
            (self.emit)(json!({ "t": "play", "stage": "failed", "now": now, "then": "", "offer": offer }));
        };
        while self.updating.load(Ordering::SeqCst) {
            self.say("play", "waiting", "Checking for a new version…", "The game starts once that's done.");
            thread::sleep(Duration::from_millis(300));
        }
        if !self.paths.exe.is_file() {
            return fail(match self.paths.kind {
                Kind::Download => "benilla.exe is missing from this folder. Your anti-virus may have taken it. Restore it, or download the game again.",
                Kind::Checkout => "The game isn't built on this computer yet. Double-click Build Basin Battle.command in this folder (10-30 min the first time), then press Play.",
            }, "");
        }
        if game_running() {
            return fail("The game is already open (its window is called benilla). Close it first: only one at a time.", "retry");
        }
        let s = self.settings.lock().unwrap().clone();
        let Some(data) = data_folder_of(s.get("wow").map(String::as_str).unwrap_or("")) else {
            return fail("Show me your World of Warcraft 1.12.1: the folder with a Data folder in it. Modern WoW Classic won't work.", "wow");
        };
        // Saved for next time (single player for this time only is not).
        self.save(|st| {
            st.insert("game".into(), game.into());
            if !solo && !place.is_empty() { st.insert("place".into(), place.into()); }
            if !solo && place == "address" { st.insert("address".into(), address.trim().into()); }
        });
        let s = self.settings.lock().unwrap().clone();
        let own = own_ports(&s, &self.project, &self.this_pc);
        let world = if game == "basin" { own.basin } else { own.survival };
        let game_name = self.project.game_name(game).to_string();
        let place = if solo { "own" } else { place };

        let (address, where_) = if place == "own" {
            match self.start_own(world, own) {
                Ok(a) => (a, "your own server on this PC".to_string()),
                Err((why, offer)) => return fail(&why, offer),
            }
        } else {
            let candidates: Vec<Server> = if place == "address" {
                let a = address.trim().to_string();
                if a.is_empty() { return fail("Type the address the host gave you.", "address"); }
                vec![Server { key: "address".into(), name: a.clone(), address: a, pc: String::new() }]
            } else {
                let backups = s.get("launcher_backups").map(|v| v != "off").unwrap_or(true);
                let from = self.project.servers.iter().position(|x| x.key == place).unwrap_or(0);
                let mut v: Vec<Server> = self.project.servers.iter().skip(from).cloned().collect();
                if !backups { v.truncate(1); }
                v
            };
            let mut found = None;
            let mut tried: Vec<Server> = Vec::new();
            for srv in &candidates {
                // A Tailscale address only answers with Tailscale on this PC: skipped then, unless it was picked.
                if is_tailscale(&srv.address) && !tailscale_installed() && srv.key != place { continue; }
                if self.here(srv) {
                    match self.start_own(world, own) {
                        Ok(a) => { found = Some((a, format!("{} (this PC)", srv.name))); break; }
                        Err((why, offer)) => return fail(&why, offer),
                    }
                }
                let then = match tried.last() { Some(t) => format!("{} didn't answer, so trying the next server.", t.name), None => "Asking the server if it's up.".into() };
                self.say("play", "checking", &format!("Looking for {}…", srv.name), &then);
                let (h, p) = split_address(&srv.address);
                if reach(&h, p, 4000) {
                    (self.emit)(json!({ "t": "server", "key": srv.key, "state": "up" }));
                    found = Some((srv.address.clone(), srv.name.clone()));
                    break;
                }
                self.log(&format!("no answer from {} at {}", srv.name, srv.address));
                (self.emit)(json!({ "t": "server", "key": srv.key, "state": "down" }));
                tried.push(srv.clone());
            }
            match found {
                Some(f) => f,
                None => {
                    let why = if tried.len() == 1 {
                        if is_tailscale(&tried[0].address) && !tailscale_installed() { open_in_shell(TAILSCALE_DOWNLOAD); }
                        why_unreachable(&tried[0].address)
                    } else if tried.is_empty() {
                        "None of the group's servers can be reached from this PC.".into()
                    } else {
                        let names: Vec<&str> = tried.iter().map(|t| t.name.as_str()).collect();
                        format!("None of the group's servers is up right now. Tried {}. Someone has to be hosting for you to join.", names.join(" and "))
                    };
                    return fail(&why, "solo");
                }
            }
        };
        self.say("play", "answered", &format!("{where_} answered."), "");

        // The game.
        self.say("play", "launching", &format!("Starting {game_name}…"), "Opening the game window.");
        if self.paths.kind == Kind::Checkout {
            if let Err(why) = self.play_checkout(game, &address) { return fail(&why, "retry"); }
            self.log(&format!("joined {game_name} at {address} ({where_})"));
            (self.emit)(json!({ "t": "play", "stage": "opened", "now": "Game opened.", "then": "Log in with any name you like (it becomes your hero) and the password <b>hold</b>.", "where": where_ }));
            if !s.get("launcher_keep").map(|v| v == "on").unwrap_or(false) { thread::sleep(Duration::from_secs(7)); (self.quit)(); }
            return;
        }
        let cfg = self.paths.root.join("benilla-config");
        // The game's memory of item and creature details: cleared so it always asks the server afresh.
        let _ = fs::remove_dir_all(cfg.join("cache"));
        // The bundled settings only for a first run: an update keeps your mouse speed and the rest.
        let (config, defaults) = (cfg.join("config.toml"), cfg.join("config.default.toml"));
        if !config.exists() && defaults.exists() { let _ = fs::copy(&defaults, &config); }
        // benilla is a console program: started like this it opens no console window behind the game.
        let started = quiet(Command::new(&self.paths.exe)
            .current_dir(&self.paths.root)
            .env("WOW_DATA", &data)
            .env("WOW_HOST", &address)
            .env("WOW_REALM", &game_name)
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()))
            .spawn();
        if let Err(e) = started {
            return fail(&format!("The game didn't start ({e}). Your anti-virus may have stopped benilla.exe."), "retry");
        }
        self.log(&format!("joining {game_name} at {address} ({where_})"));
        (self.emit)(json!({ "t": "play", "stage": "opened", "now": "Game opened.", "then": "Log in with any name you like (it becomes your hero) and the password <b>hold</b>.", "where": where_ }));
        let keep = s.get("launcher_keep").map(|v| v == "on").unwrap_or(false);
        if !keep {
            thread::sleep(Duration::from_secs(7));
            (self.quit)();
        }
    }
}

/// A program that outlives Play: its own process group, so neither closing Play nor the Terminal
/// it was started from takes it down.
#[cfg(unix)]
fn detached(c: &mut Command) -> &mut Command {
    use std::os::unix::process::CommandExt;
    c.process_group(0)
}

impl Backend {
    /// This PC's own server, in its own window: Host.exe on Windows, Host Basin Battle.command on
    /// a Mac (Terminal), scripts/host.sh in the background elsewhere.
    fn open_host(&self) -> bool {
        let root = &self.paths.root;
        match self.paths.kind {
            Kind::Download => {
                let host = root.join("Host.exe");
                if host.is_file() { return Command::new(host).current_dir(root).spawn().is_ok(); }
                Command::new("powershell.exe").args(["-NoProfile", "-ExecutionPolicy", "Bypass", "-File"])
                    .arg(self.paths.launcher.join("host.ps1")).current_dir(root).spawn().is_ok()
            }
            Kind::Checkout => {
                #[cfg(target_os = "macos")]
                { Command::new("open").arg(root.join("Host Basin Battle.command")).spawn().is_ok() }
                #[cfg(not(target_os = "macos"))]
                {
                    let log = fs::File::create(root.join("logs").join("launcher-host.log"));
                    let mut c = Command::new("bash");
                    c.arg(root.join("scripts").join("host.sh")).current_dir(root).stdin(Stdio::null());
                    if let Ok(f) = log { if let Ok(g) = f.try_clone() { c.stdout(f).stderr(g); } }
                    #[cfg(unix)]
                    detached(&mut c);
                    c.spawn().is_ok()
                }
            }
        }
    }

    /// The checkout's update: main, fast-forwarded to GitHub's. Only on main with nothing of its
    /// own changed, so a developer's work is never touched. The game client rebuilds itself the
    /// next time it starts (scripts/play.sh); a change to Play itself takes effect when it's
    /// opened next (Basin Battle.app rebuilds it).
    fn git_update(self: &Arc<Self>, asked: bool) {
        if self.updating.swap(true, Ordering::SeqCst) { return; }
        let done = |b: &Self| b.updating.store(false, Ordering::SeqCst);
        let branch = self.git(&["rev-parse", "--abbrev-ref", "HEAD"]).unwrap_or_default();
        if branch != "main" {
            if asked { self.say("update", "none", &format!("This folder is on {branch}, not main."), "Play only updates main. Use git yourself here."); }
            return done(self);
        }
        if !self.git(&["status", "--porcelain", "--untracked-files=no"]).map(|o| o.is_empty()).unwrap_or(false) {
            if asked { self.say("update", "none", "This folder has changes of its own.", "Play doesn't update over them. Use git yourself here."); }
            return done(self);
        }
        self.say("update", "checking", "Checking for a new version…", "");
        // Offline, a fetch can hang for a long time: given 30 sec.
        let fetched = Command::new("git").arg("-C").arg(&self.paths.root).args(["fetch", "-q", "origin", "main"])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn()
            .map(|mut c| {
                let t0 = Instant::now();
                loop {
                    if let Ok(Some(st)) = c.try_wait() { break st.success(); }
                    if t0.elapsed() > Duration::from_secs(30) { let _ = c.kill(); break false; }
                    thread::sleep(Duration::from_millis(200));
                }
            })
            .unwrap_or(false);
        let behind: u32 = if fetched { self.git(&["rev-list", "--count", "HEAD..FETCH_HEAD"]).and_then(|n| n.parse().ok()).unwrap_or(0) } else { 0 };
        if behind == 0 {
            self.say("update", "none", if !asked { "" } else if fetched { "You have the newest version." } else { "Couldn't reach GitHub to check for a new version." }, "");
            return done(self);
        }
        let before = self.git(&["rev-parse", "HEAD"]).unwrap_or_default();
        self.say("update", "working", &format!("Updating ({behind} new changes)…"), "Your settings and heroes stay as they are.");
        if self.git(&["merge", "--ff-only", "-q", "FETCH_HEAD"]).is_none() {
            self.say("update", "failed", "The update didn't work. Nothing was changed.", "");
            return done(self);
        }
        self.log(&format!("updated from {before} to {}", self.version()));
        let changed = self.git(&["diff", "--name-only", &before, "HEAD"]).unwrap_or_default();
        let play_changed = changed.lines().any(|f| f.starts_with("dist/windows/buttons/") || f.starts_with("dist/windows/launcher/ui/"));
        if play_changed {
            if let Some(app) = std::env::current_exe().ok().and_then(|e| e.ancestors().find(|a| a.extension().map(|x| x == "app").unwrap_or(false)).map(Path::to_path_buf)) {
                // The app builds the new Play, then opens it.
                self.say("update", "done", &format!("Updated to {}. Starting it…", self.version()), "");
                thread::sleep(Duration::from_millis(600));
                let _ = Command::new("open").arg("-n").arg(&app).args(["--args", "-NoUpdate"]).spawn();
                (self.quit)();
                return;
            }
        }
        self.say("update", "none", &format!("Updated to {}.", self.version()), "The game rebuilds itself the first time you play, which takes a few minutes.");
        done(self);
    }

    /// The game on a Mac: scripts/play.sh, as the Mac scripts play (it rebuilds the client after
    /// an update, puts in the add-ons and starts it). Its output goes to logs/launcher-game.log;
    /// the game counts as open once its process is up.
    fn play_checkout(&self, game: &str, address: &str) -> Result<(), String> {
        let root = &self.paths.root;
        let log_path = root.join("logs").join("launcher-game.log");
        let log = fs::File::create(&log_path).map_err(|e| format!("Couldn't write logs/launcher-game.log ({e})."))?;
        let mut c = Command::new("bash");
        c.arg(root.join("scripts").join("play.sh")).arg(address).current_dir(root)
            .env("HTL_GAME", if game == "basin" { "basin" } else { "survival" })
            .stdin(Stdio::null()).stdout(log.try_clone().map_err(|e| e.to_string())?).stderr(log);
        #[cfg(unix)]
        detached(&mut c);
        let mut child = c.spawn().map_err(|e| format!("The game didn't start ({e})."))?;
        let t0 = Instant::now();
        let mut building = false;
        loop {
            thread::sleep(Duration::from_millis(700));
            if game_running() { return Ok(()); }
            let text = fs::read_to_string(&log_path).unwrap_or_default();
            if !building && (text.contains("Building") || text.contains("Compiling")) {
                building = true;
                self.say("play", "launching", "Building the game client with the latest changes…", "A few minutes after an update, then it starts by itself.");
            }
            if let Ok(Some(st)) = child.try_wait() {
                if game_running() { return Ok(()); }
                let last = text.lines().rev().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").to_string();
                self.log(&format!("scripts/play.sh ended ({st}): {last}"));
                return Err(if last.is_empty() { "The game closed before it opened. See logs/launcher-game.log.".into() } else { format!("The game didn't start: {last} (more in logs/launcher-game.log)") });
            }
            if t0.elapsed() > Duration::from_secs(30 * 60) { return Err("The game took over 30 min to start. See logs/launcher-game.log.".into()); }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_values() {
        let t = "title = \"Basin Battle\"\nserver = \"24.64.116.126\"\nserver_name = \"Corios's desktop\"\nbackup = \"\"\n# backup = \"x\"\n";
        assert_eq!(project_value(t, "server").as_deref(), Some("24.64.116.126"));
        assert_eq!(project_value(t, "server_name").as_deref(), Some("Corios's desktop"));
        assert_eq!(project_value(t, "backup"), None);
    }

    #[test]
    fn the_repositorys_project_toml() {
        let dir = std::env::temp_dir().join(format!("play-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../project.toml"), dir.join("project.toml")).unwrap();
        let p = Project::read(&dir.join("project.toml"));
        assert_eq!(p.servers.len(), 3);
        assert_eq!(p.servers[1].pc, "CHANDRIAN");
        assert!(is_tailscale(&p.servers[2].address));
        let own = own_ports(&Settings::new(), &p, "chandrian");
        assert_eq!((own.login, own.basin, own.survival), (3725, 8087, 8088));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn addresses() {
        assert_eq!(split_address("24.64.116.126:3725"), ("24.64.116.126".into(), 3725));
        assert_eq!(split_address("host"), ("host".into(), 3724));
        assert!(is_tailscale("100.125.4.34"));
        assert!(is_tailscale("pc.tail1234.ts.net:3725"));
        assert!(!is_tailscale("100.20.4.34"));
        assert!(!is_tailscale("192.168.1.5"));
    }

    #[test]
    #[cfg(unix)] // bash reads settings.sh, as the Mac scripts do
    fn settings_sh_beside_the_scripts() {
        let dir = std::env::temp_dir().join(format!("play-settings-sh-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("settings.sh");
        // As common.sh's save_setting writes it (printf %q).
        fs::write(&f, "WOW_DATA=/Users/david/Games/World\\ of\\ Warcraft/Data\nHTL_HOST=100.125.4.34\n").unwrap();
        let mut s = read_settings_sh(&f);
        assert_eq!(s["wow"], "/Users/david/Games/World of Warcraft/Data");
        s.insert("place".into(), "backup2".into());
        s.insert("address".into(), "it's\nHTL_HOST=evil".into());
        save_settings_sh(&f, &s);
        let text = fs::read_to_string(&f).unwrap();
        assert!(text.contains("HTL_HOST=100.125.4.34"), "the scripts' own lines stay: {text}");
        let again = read_settings_sh(&f);
        assert_eq!(again["wow"], s["wow"]);
        assert_eq!(again["place"], "backup2");
        assert_eq!(again["address"], "it'sHTL_HOST=evil");
        // A choice cleared is gone from the file, not read back from an older line.
        s.remove("place");
        save_settings_sh(&f, &s);
        assert!(!read_settings_sh(&f).contains_key("place"));
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("play-settings-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("settings.txt");
        fs::write(&f, "C:\\Games\\WoW\\Data\r\n1.2.3.4\r\n").unwrap();
        let s = read_settings(&f);
        assert_eq!(s["wow"], "C:\\Games\\WoW\\Data");
        assert_eq!(s["address"], "1.2.3.4");
        // As Windows PowerShell writes it, in the PC's code page.
        fs::write(&f, b"# Your choices\r\ngame = basin\r\nwow = C:\\Users\\Jos\xe9\\WoW\\Data\r\nlogin_port = 3725\r\n").unwrap();
        let mut s = read_settings(&f);
        assert_eq!(s["wow"], "C:\\Users\\José\\WoW\\Data");
        s.insert("place".into(), "own".into());
        save_settings(&f, &s);
        let again = read_settings(&f);
        assert_eq!(again, s);
        // A pasted address with a line break in it stays one setting.
        s.insert("address".into(), "1.2.3.4\r\nwow = C:\\x".into());
        save_settings(&f, &s);
        let again = read_settings(&f);
        assert_eq!(again["address"], "1.2.3.4wow = C:\\x");
        assert_eq!(again["wow"], "C:\\Users\\José\\WoW\\Data");
        let _ = fs::remove_dir_all(dir);
    }
}
