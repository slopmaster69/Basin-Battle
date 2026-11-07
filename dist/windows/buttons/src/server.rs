//! This PC's own server (server\htl-server.exe, the one that came in the download): its ports,
//! its arguments, starting it and waiting for its world.

use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use crate::folder::{Paths, Project, Settings};
use crate::net::reach;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ports {
    pub login: u16,
    pub basin: u16,
    pub survival: u16,
}

/// The usual 3724 and 8085/8086, or settings.txt's login_port, world_port and survival_port.
pub fn own_ports(s: &Settings) -> Ports {
    let get = |k: &str| s.get(k).and_then(|v| v.parse::<u16>().ok()).filter(|v| *v > 0);
    Ports {
        login: get("login_port").unwrap_or(3724),
        basin: get("world_port").unwrap_or(8085),
        survival: get("survival_port").unwrap_or(8086),
    }
}

impl Ports {
    pub fn world(&self, game: &str) -> u16 {
        if game == "basin" { self.basin } else { self.survival }
    }
    /// How this PC's address is written for a friend: with :port when the login port isn't 3724.
    pub fn suffix(&self) -> String {
        if self.login == 3724 { String::new() } else { format!(":{}", self.login) }
    }
}

/// The server's arguments: both games, the player's game files, its content and saves beside it.
pub fn args(project: &Project, data: &Path, ports: Ports) -> Vec<String> {
    vec![
        "--realm".into(), project.basin.clone(),
        "--survival-realm".into(), project.survival.clone(),
        "--data".into(), data.to_string_lossy().to_string(),
        "--content".into(), "content\\game.toml".replace('\\', std::path::MAIN_SEPARATOR_STR),
        "--saves".into(), "saves\\basin-battle.toml".replace('\\', std::path::MAIN_SEPARATOR_STR),
        "--survival-content".into(), "content\\survival.toml".replace('\\', std::path::MAIN_SEPARATOR_STR),
        "--survival-save".into(), "saves\\elwynn-survival.toml".replace('\\', std::path::MAIN_SEPARATOR_STR),
        "--login-port".into(), ports.login.to_string(),
        "--world-port".into(), ports.basin.to_string(),
        "--survival-port".into(), ports.survival.to_string(),
    ]
}

/// Whether this PC's own server is up: its login port and the game's world port both answer.
pub fn up(ports: Ports, game: &str) -> bool {
    reach("127.0.0.1", ports.login, 400) && reach("127.0.0.1", ports.world(game), 400)
}

/// The server started quietly in the background (for Play alone), its output in
/// server\logs\play-alone.log.
pub fn start_quiet(paths: &Paths, args: &[String]) -> Result<Child, String> {
    if !paths.server_exe.is_file() {
        return Err("This download has no server in it (server\\htl-server.exe). Download the newest version, or play on the group's server.".into());
    }
    let logs = paths.server_dir.join("logs");
    let _ = std::fs::create_dir_all(&logs);
    let _ = std::fs::create_dir_all(paths.server_dir.join("saves"));
    let log = std::fs::File::create(logs.join("play-alone.log")).map_err(|e| format!("Couldn't write server\\logs ({e})."))?;
    let err = log.try_clone().map_err(|e| e.to_string())?;
    let mut c = Command::new(&paths.server_exe);
    c.args(args).current_dir(&paths.server_dir).stdin(Stdio::null()).stdout(log).stderr(err);
    crate::proc::quiet(&mut c);
    c.spawn().map_err(|e| format!("Your server didn't start ({e}). Your anti-virus may have stopped htl-server.exe."))
}

/// Waits for the world to be up, telling `say` how long it's been, for up to `most`.
pub fn wait_up(ports: Ports, game: &str, most: Duration, mut say: impl FnMut(u64)) -> Result<(), String> {
    let t0 = Instant::now();
    let mut told = 0;
    while !up(ports, game) {
        std::thread::sleep(Duration::from_millis(700));
        let s = t0.elapsed().as_secs();
        if s / 5 > told {
            told = s / 5;
            say(s);
        }
        if t0.elapsed() > most {
            return Err(format!("Your server didn't come up in {} minutes. server\\logs\\play-alone.log says why.", most.as_secs() / 60));
        }
    }
    Ok(())
}
