//! Where things are, what the project is called, and the player's saved choices.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// The repository's project.toml, built into Play and Host: the names players see, the group's
/// server and the GitHub repository releases come from. A folder carries no copy of it.
pub const PROJECT_TOML: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/../../../project.toml"));

/// The names and addresses from project.toml.
#[derive(Clone, Debug)]
pub struct Project {
    /// The folder's, the zip's and the window's name.
    pub title: String,
    pub survival: String,
    pub basin: String,
    /// owner/name on GitHub.
    pub repository: String,
    /// The group's server, "host" or "host:port"; empty when the group has none.
    pub server: String,
    pub server_name: String,
    /// The password everyone logs in with.
    pub password: String,
}

/// `key = "value"` from project.toml: the first line for that key with a value in it (the file
/// is read with a plain search here and in the scripts, never a TOML library).
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
    pub fn built_in() -> Self {
        Self::parse(PROJECT_TOML)
    }

    pub fn parse(text: &str) -> Self {
        let get = |k: &str, d: &str| project_value(text, k).unwrap_or_else(|| d.to_string());
        let server = get("server", "");
        Project {
            title: get("title", "Elwynn Survival"),
            survival: get("survival", "Elwynn Survival"),
            basin: get("basin", "Basin Battle"),
            repository: get("repository", "slopmaster69/Basin-Battle"),
            server_name: get("server_name", &server),
            server,
            password: get("password", "hold"),
        }
    }

    /// The realm a game runs on: settings.txt's `game` is "survival" or "basin".
    pub fn game_name(&self, game: &str) -> &str {
        if game == "basin" { &self.basin } else { &self.survival }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The Windows download: everything beside Play.exe.
    Download,
    /// A clone of this repository (how a Mac plays): scripts/, benilla/ built in place.
    Checkout,
}

#[derive(Clone, Debug)]
pub struct Paths {
    pub kind: Kind,
    pub root: PathBuf,
    /// The game.
    pub exe: PathBuf,
    pub server_dir: PathBuf,
    pub server_exe: PathBuf,
    pub settings: PathBuf,
    pub version: PathBuf,
    pub log: PathBuf,
}

impl Paths {
    pub fn download(root: PathBuf) -> Self {
        let server_dir = root.join("server");
        Paths {
            kind: Kind::Download,
            exe: root.join(if cfg!(windows) { "benilla.exe" } else { "benilla" }),
            server_exe: server_dir.join(if cfg!(windows) { "htl-server.exe" } else { "htl-server" }),
            settings: root.join("settings.txt"),
            version: root.join("version.txt"),
            log: root.join("play.log"),
            server_dir,
            root,
        }
    }

    pub fn checkout(root: PathBuf) -> Self {
        Paths {
            kind: Kind::Checkout,
            exe: root.join("benilla").join("target").join("release").join("benilla"),
            server_dir: root.clone(),
            server_exe: root.join("target").join("release").join("htl-server"),
            settings: root.join("settings.sh"),
            version: root.join("version.txt"),
            log: root.join("logs").join("launcher.log"),
            root,
        }
    }

    /// Where Play or Host is: the download they sit in, or the checkout they were built in
    /// (somewhere under it, in target/ or in the Mac app). `BB_ROOT` names it outright.
    pub fn find(exe: &Path) -> Option<Self> {
        if let Some(r) = std::env::var_os("BB_ROOT") {
            let r = PathBuf::from(r);
            return Some(if r.join("scripts").join("play.sh").is_file() { Paths::checkout(r) } else { Paths::download(r) });
        }
        let dir = exe.parent()?;
        if dir.join("version.txt").is_file() || dir.join("benilla.exe").is_file() || dir.join("server").is_dir() {
            return Some(Paths::download(dir.to_path_buf()));
        }
        for a in dir.ancestors() {
            if a.join("project.toml").is_file() && a.join("scripts").join("play.sh").is_file() {
                return Some(Paths::checkout(a.to_path_buf()));
            }
        }
        // A Play.exe copied somewhere on its own: the folder it's in, so it can still say what's wrong.
        Some(Paths::download(dir.to_path_buf()))
    }

    /// version.txt's first line: "v0.5.0" (a release), "latest 1a2b3c4" (the newest main, for
    /// playtests), "branch x (1a2b3c4)" (a branch's build, never updated), or "" when there is none.
    pub fn installed_version(&self) -> String {
        read_text(&self.version)
            .and_then(|t| t.lines().next().map(|l| l.trim().to_string()))
            .unwrap_or_default()
    }
}

/// Where a folder's updates come from.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Channel {
    /// Releases (v0.5.0...): what players run.
    Release,
    /// The `latest` pre-release, rebuilt from main at every merge: a playtest folder.
    Latest,
    /// A branch's build or a folder built by hand: never updated over.
    Fixed,
}

pub fn channel(version: &str) -> Channel {
    let v = version.trim();
    if v.starts_with('v') && v[1..].starts_with(|c: char| c.is_ascii_digit()) {
        Channel::Release
    } else if v.starts_with("latest") {
        Channel::Latest
    } else {
        Channel::Fixed
    }
}

/// Is release `tag` newer than the installed `mine`? Releases by their numbers; the `latest`
/// channel by its commit (any other commit is newer, and so is one over "latest none").
pub fn is_newer(tag: &str, mine: &str) -> bool {
    match channel(mine) {
        Channel::Fixed => false,
        Channel::Latest => {
            let a = mine.split_whitespace().nth(1).unwrap_or("none");
            let b = tag.split_whitespace().nth(1).unwrap_or("");
            !b.is_empty() && a != b
        }
        Channel::Release => {
            if !tag.starts_with('v') || tag == mine {
                return false;
            }
            let nums = |s: &str| -> Vec<u64> {
                s.trim_start_matches('v')
                    .split(|c: char| !c.is_ascii_digit())
                    .take_while(|p| !p.is_empty())
                    .filter_map(|p| p.parse().ok())
                    .collect()
            };
            let (a, b) = (nums(tag), nums(mine));
            if a.is_empty() || b.is_empty() {
                return false;
            }
            a > b
        }
    }
}

/// A text file as Windows PowerShell may once have written it: UTF-8 with or without its mark,
/// or the PC's own code page, read here as Latin-1.
pub fn read_text(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF][..]).unwrap_or(&bytes);
    Some(match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => bytes.iter().map(|&b| b as char).collect(),
    })
}

// --- settings.txt: one "key = value" a line, kept across updates -------------------------------
//
// wow        the WoW 1.12.1 Data folder
// name       the account name (any name becomes a hero on the server)
// password   only when the host changed it from the project's
// game       survival (the default) or basin
// place      group (the project's server), own (this PC) or address (the one below)
// address    "host" or "host:port", for place = address
// login_port, world_port, survival_port   this PC's own server's ports, when not the usual

pub type Settings = BTreeMap<String, String>;

pub fn read_settings(file: &Path) -> Settings {
    let mut s = Settings::new();
    let Some(text) = read_text(file) else { return s };
    for l in text.lines() {
        let l = l.trim();
        if l.starts_with('#') {
            continue;
        }
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
    let mut out = String::from("\u{FEFF}# Your choices, kept across updates. Delete this file to be asked again.\r\n");
    for (k, v) in s {
        // One line each, whatever was typed or pasted: a line break would write a second setting.
        let v: String = v.chars().filter(|c| !c.is_control()).collect();
        if !v.trim().is_empty() {
            out.push_str(&format!("{k} = {}\r\n", v.trim()));
        }
    }
    let _ = fs::write(file, out);
}

/// The account name: settings.txt's, else this PC's user name (letters and digits only, the
/// game's limit of 16), else Player.
pub fn account_name(s: &Settings) -> String {
    let clean = |v: &str| -> String { v.chars().filter(|c| c.is_ascii_alphanumeric()).take(16).collect() };
    if let Some(n) = s.get("name").map(|n| clean(n)).filter(|n| !n.is_empty()) {
        return n;
    }
    std::env::var("USERNAME")
        .or_else(|_| std::env::var("USER"))
        .ok()
        .map(|n| clean(&n))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Player".into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_repositorys_project_toml_reads() {
        let p = Project::built_in();
        assert_eq!(p.title, "Elwynn Survival");
        assert!(!p.server.is_empty(), "project.toml names the group's server");
        assert!(!p.repository.is_empty());
    }

    #[test]
    fn project_values() {
        let t = "title = \"Basin Battle\"\nserver = \"24.64.116.126\"\nserver_name = \"Corios's desktop\"\nbackup = \"\"\n# backup = \"x\"\n";
        assert_eq!(project_value(t, "server").as_deref(), Some("24.64.116.126"));
        assert_eq!(project_value(t, "server_name").as_deref(), Some("Corios's desktop"));
        assert_eq!(project_value(t, "backup"), None);
    }

    #[test]
    fn channels_and_what_is_newer() {
        assert_eq!(channel("v0.5.0"), Channel::Release);
        assert_eq!(channel("latest 1a2b3c4"), Channel::Latest);
        assert_eq!(channel("branch corios/x (1a2b3c4)"), Channel::Fixed);
        assert_eq!(channel(""), Channel::Fixed);
        assert!(is_newer("v0.5.1", "v0.5.0"));
        assert!(is_newer("v0.10.0", "v0.9.3"));
        assert!(!is_newer("v0.5.0", "v0.5.0"));
        assert!(!is_newer("v0.4.9", "v0.5.0"));
        assert!(!is_newer("latest 1a2b3c4", "v0.5.0"), "a release folder never takes the playtest build");
        assert!(!is_newer("v9.9.9", "branch x (abc)"), "a branch build is never updated over");
        assert!(is_newer("latest 1a2b3c4", "latest none"));
        assert!(is_newer("latest 1a2b3c4", "latest 0000000"));
        assert!(!is_newer("latest 1a2b3c4", "latest 1a2b3c4"));
    }

    #[test]
    fn settings_round_trip() {
        let dir = std::env::temp_dir().join(format!("launcher-settings-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let f = dir.join("settings.txt");
        // As Windows PowerShell once wrote it, in the PC's code page.
        fs::write(&f, b"# Your choices\r\ngame = basin\r\nwow = C:\\Users\\Jos\xe9\\WoW\\Data\r\nlogin_port = 3725\r\n").unwrap();
        let mut s = read_settings(&f);
        assert_eq!(s["wow"], "C:\\Users\\José\\WoW\\Data");
        s.insert("place".into(), "own".into());
        save_settings(&f, &s);
        assert_eq!(read_settings(&f), s);
        // A pasted address with a line break in it stays one setting.
        s.insert("address".into(), "1.2.3.4\r\nwow = C:\\x".into());
        save_settings(&f, &s);
        let again = read_settings(&f);
        assert_eq!(again["address"], "1.2.3.4wow = C:\\x");
        assert_eq!(again["wow"], "C:\\Users\\José\\WoW\\Data");
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn account_names_are_the_games_kind() {
        let mut s = Settings::new();
        s.insert("name".into(), "Corey O'Brien 2!".into());
        assert_eq!(account_name(&s), "CoreyOBrien2");
        s.insert("name".into(), "abcdefghijklmnopqrstuvwxyz".into());
        assert_eq!(account_name(&s).len(), 16);
    }
}
