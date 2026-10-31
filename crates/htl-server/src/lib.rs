//! Basin Battle's server: a 1.12.1 login server and world servers in one process, running the
//! wave-defence game and (on its own realm and world port) Elwynn Survival for benilla clients.

pub mod fields;
pub mod game;
pub mod packets;
pub mod realmd;
pub mod survival;
pub mod wire;
pub mod world;

use std::net::{SocketAddr, TcpListener};
use std::sync::OnceLock;
use std::time::Instant;

use anyhow::{Context, Result};
use htl_sim::{Arena, Content, Ground, Sim};

static START: OnceLock<Instant> = OnceLock::new();

/// Milliseconds since the server started, the clock movement timestamps use.
pub fn clock_ms() -> u32 {
    START.get_or_init(Instant::now).elapsed().as_millis() as u32
}

pub struct Config {
    pub bind: String,
    pub login_port: u16,
    pub world_port: u16,
    pub realm_name: String,
    pub password: String,
    pub public_host: Option<String>,
    /// Where scores and Renown are kept; `None` keeps nothing.
    pub saves: Option<std::path::PathBuf>,
    /// Spell id to icon name (`Spell_Frost_FrostBolt02`), for the add-on's card picker.
    pub icons: std::collections::HashMap<u32, String>,
    /// Elwynn Survival, on its own realm, if it's running too.
    pub survival: Option<SurvivalSetup>,
    /// The Bookmaker's Board host (`host:port`) this server sends its wagers to; none: this
    /// server is the board host, and takes everyone's on its login port.
    pub board: Option<String>,
    /// This server's name on the board (a made-up camp's when empty).
    pub board_name: String,
}

/// Elwynn Survival's realm: its world port, name, save file, rules and land.
pub struct SurvivalSetup {
    pub world_port: u16,
    pub realm_name: String,
    /// Where the world is kept; `None` keeps nothing.
    pub save: Option<std::path::PathBuf>,
    pub content: htl_survival::Content,
    pub land: Box<dyn htl_survival::Land>,
    /// Each dungeon's ground that could be read (by the dungeon's key): a world below each,
    /// reached by the book.
    pub below: Vec<(String, Box<dyn htl_survival::Land>)>,
    /// Arathi Basin's ground, if it could be read: Siege's world, reached by the book.
    pub siege: Option<Box<dyn htl_survival::Land>>,
}

/// The default name and port of the survival realm.
pub const SURVIVAL_REALM: &str = "Elwynn Survival";
pub const SURVIVAL_PORT: u16 = 8086;

impl Default for Config {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".into(),
            login_port: 3724,
            world_port: 8085,
            realm_name: "Basin Battle".into(),
            password: "hold".into(),
            public_host: None,
            saves: None,
            icons: Default::default(),
            survival: None,
            board: None,
            board_name: String::new(),
        }
    }
}

/// The ports a started server listens on (useful when asked for port 0 in tests).
pub struct Running {
    pub login: SocketAddr,
    /// Basin Battle's world (or the survival world's, when it runs alone).
    pub world: SocketAddr,
    pub survival: Option<SocketAddr>,
}

/// Basin Battle's rules and arena.
pub struct Basin {
    pub content: Content,
    pub arena: Arena,
    pub ground: Box<dyn Ground>,
}

/// Start Basin Battle (and Elwynn Survival, if configured) on background threads and return once
/// it is listening.
pub fn start(config: Config, content: Content, arena: Arena, ground: Box<dyn Ground>) -> Result<Running> {
    start_games(config, Some(Basin { content, arena, ground }))
}

/// Start the games asked for: Basin Battle if `basin` is given, Elwynn Survival if the config has
/// it. Each has its own realm and world port; one login serves both.
pub fn start_games(mut config: Config, basin: Option<Basin>) -> Result<Running> {
    START.get_or_init(Instant::now);
    let login = TcpListener::bind((config.bind.as_str(), config.login_port))
        .with_context(|| format!("listening for logins on port {}", config.login_port))?;
    let keys = realmd::SessionKeys::default();
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(1);
    let mut realms = Vec::new();
    let mut bb_world = None;
    if let Some(b) = basin {
        let world = TcpListener::bind((config.bind.as_str(), config.world_port))
            .with_context(|| format!("listening for the world on port {}", config.world_port))?;
        let addr = world.local_addr()?;
        realms.push(realmd::Realm { name: config.realm_name.clone(), world_port: addr.port() });
        bb_world = Some(addr);
        let saves = config.saves.clone();
        let icons = config.icons.clone();
        let (tx, rx) = crossbeam_channel::unbounded();
        world::spawn(world, keys.clone(), tx);
        let sim = Sim::new(b.content, b.arena, b.ground, seed);
        std::thread::Builder::new()
            .name("game".into())
            .spawn(move || game::Game::new(sim, saves, icons).run(rx))?;
    }
    let mut survival_world = None;
    let mut board = None;
    if let Some(sv) = config.survival.take() {
        // The Bookmaker's Board, kept beside the survival save.
        let dungeons: Vec<String> = sv.content.dungeons().map(|d| d.key.clone()).collect();
        let path = sv.save.as_ref().map(|p| p.with_file_name("wager-board.toml"));
        let shared: survival::board::Shared = std::sync::Arc::new(std::sync::Mutex::new(survival::board::Board::new(path, &config.board_name, config.board.clone(), dungeons)));
        survival::board::Board::spawn_sync(shared.clone());
        board = Some(shared);
        let world = TcpListener::bind((config.bind.as_str(), sv.world_port))
            .with_context(|| format!("listening for the survival world on port {}", sv.world_port))?;
        let addr = world.local_addr()?;
        realms.push(realmd::Realm { name: sv.realm_name.clone(), world_port: addr.port() });
        survival_world = Some(addr);
        let (tx, rx) = crossbeam_channel::unbounded();
        world::spawn(world, keys.clone(), tx);
        // Each world below keeps its own save beside the camp's.
        let mut below = Vec::new();
        for (n, (key, land)) in sv.below.into_iter().enumerate() {
            let Some(d) = sv.content.dungeons().find(|d| d.key == key).cloned() else {
                eprintln!("[survival] no dungeon {key} in the rules");
                continue;
            };
            let save = sv.save.as_ref().map(|p| p.with_file_name(format!("{key}-survival.toml")));
            let content = sv.content.for_dungeon_of(&key);
            below.push(survival::SurvivalGame::new_in(survival::Realm::dungeon(&d), content, land, save, seed ^ 0xdead ^ (n as u64) << 20));
        }
        let siege = sv.siege.map(|land| survival::SurvivalGame::new_in(survival::Realm::SIEGE, sv.content.clone(), land, None, seed ^ 0x5e1e));
        let game = survival::SurvivalGame::new(sv.content, sv.land, sv.save, seed ^ 0x5eed);
        let mut realms = survival::realms::Realms::with_siege(game, below, siege);
        realms.set_board(board.clone());
        std::thread::Builder::new().name("survival".into()).spawn(move || realms.run(rx))?;
    }
    let Some(world) = bb_world.or(survival_world) else {
        anyhow::bail!("no game to run");
    };
    let running = Running { login: login.local_addr()?, world, survival: survival_world };
    realmd::spawn(
        login,
        realmd::RealmConfig { realms, password: config.password, public_host: config.public_host, board },
        keys,
    );
    Ok(running)
}
