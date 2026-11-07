//! Starting the game, logged in.

use std::path::Path;
use std::process::{Child, Command, Stdio};

use crate::folder::{Kind, Paths};

/// Who plays where: handed to the client in its environment, so the login screen is never seen
/// (`WOW_USER` and `WOW_PASS` together make the client log in by itself; `WOW_HOST` pins the
/// realm list; `WOW_REALM` picks the game's realm; `WOW_CHAR` picks the hero when given).
pub struct Launch<'a> {
    pub data: &'a Path,
    pub address: &'a str,
    pub realm: &'a str,
    pub user: &'a str,
    pub pass: &'a str,
    pub hero: Option<&'a str>,
}

/// The game, started. Its window is the only one: benilla is a console program, started here
/// without a console behind it.
pub fn start(paths: &Paths, l: &Launch) -> Result<Child, String> {
    if paths.kind == Kind::Checkout {
        return start_checkout(paths, l);
    }
    if !paths.exe.is_file() {
        return Err("benilla.exe is missing from this folder. Your anti-virus may have taken it: restore it, or download the game again.".into());
    }
    let cfg = paths.root.join("benilla-config");
    // The game's memory of item and creature details: cleared, so it always asks the server afresh.
    let _ = std::fs::remove_dir_all(cfg.join("cache"));
    // The bundled settings only for a first run: an update keeps your mouse speed and the rest.
    let (config, defaults) = (cfg.join("config.toml"), cfg.join("config.default.toml"));
    if !config.exists() && defaults.exists() {
        let _ = std::fs::copy(&defaults, &config);
    }
    let mut c = Command::new(&paths.exe);
    c.current_dir(&paths.root)
        .env("WOW_DATA", l.data)
        .env("WOW_HOST", l.address)
        .env("WOW_REALM", l.realm)
        .env("WOW_USER", l.user)
        .env("WOW_PASS", l.pass)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    if let Some(h) = l.hero.filter(|h| !h.is_empty()) {
        c.env("WOW_CHAR", h);
    }
    crate::proc::quiet(&mut c);
    c.spawn().map_err(|e| format!("The game didn't start ({e}). Your anti-virus may have stopped benilla.exe."))
}

/// A Mac (a clone of the repository): scripts/play.sh, as the Mac scripts play. It rebuilds the
/// client after an update, puts in the add-ons and starts it; the login is handed over the same way.
fn start_checkout(paths: &Paths, l: &Launch) -> Result<Child, String> {
    let root = &paths.root;
    let _ = std::fs::create_dir_all(root.join("logs"));
    let log = std::fs::File::create(root.join("logs").join("launcher-game.log")).map_err(|e| format!("Couldn't write logs/launcher-game.log ({e})."))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut c = Command::new("bash");
    c.arg(root.join("scripts").join("play.sh"))
        .arg(l.address)
        .current_dir(root)
        .env("HTL_GAME", if l.realm.to_lowercase().contains("basin") { "basin" } else { "survival" })
        .env("WOW_DATA", l.data)
        .env("WOW_USER", l.user)
        .env("WOW_PASS", l.pass)
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(err);
    if let Some(h) = l.hero.filter(|h| !h.is_empty()) {
        c.env("WOW_CHAR", h);
    }
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.process_group(0);
    }
    c.spawn().map_err(|e| format!("The game didn't start ({e})."))
}
