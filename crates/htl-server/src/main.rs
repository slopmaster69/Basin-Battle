//! `htl-server`: run Basin Battle, and Elwynn Survival on a second realm.
//!
//! ```text
//! htl-server --data "/path/to/WoW-1.12.1/Data" [--password hold] [--public-host my.ip.or.name]
//! ```

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use htl_server::Config;
use htl_sim::Content;

const HELP: &str = "\
Basin Battle and Elwynn Survival server

USAGE:
    htl-server --data <WoW 1.12.1 Data folder> [options]

OPTIONS:
    --data <path>         The Data folder of your 1.12.1 install (or set WOW_DATA)
    --password <text>     The password everyone logs in with (default: hold)
    --public-host <host>  Your internet address or hostname, for friends outside your network
    --realm <name>        The realm name shown in the realm list (default: Basin Battle)
    --content <file>      A game.toml to use instead of the built-in one (cards.toml and
                          gear.toml are read from the same folder)
    --saves <file>        Where high scores and Renown are kept (default: saves/basin-battle.toml)
    --login-port <port>   Default 3724
    --world-port <port>   Default 8085
    --game <which>        both (default), basin or survival: the games to run, each on its own
                          realm (\"Basin Battle\" and \"Elwynn Survival\")
    --survival-port <p>   Elwynn Survival's world port (default 8086)
    --survival-save <f>   Where the survival world is kept (default saves/elwynn-survival.toml)
    --survival-content <f> A survival.toml to use instead of the built-in one
    --help                This text
";

fn main() -> Result<()> {
    let mut config = Config { saves: Some("saves/basin-battle.toml".into()), ..Config::default() };
    let mut data: Option<PathBuf> = std::env::var_os("WOW_DATA").map(PathBuf::from);
    let mut content_path: Option<PathBuf> = None;
    let mut game = String::from("both");
    let mut survival_port = htl_server::SURVIVAL_PORT;
    let mut survival_save: PathBuf = "saves/elwynn-survival.toml".into();
    let mut survival_content: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        let mut value = |name: &str| args.next().with_context(|| format!("{name} needs a value"));
        match a.as_str() {
            "--data" => data = Some(value("--data")?.into()),
            "--password" => config.password = value("--password")?,
            "--public-host" => config.public_host = Some(value("--public-host")?),
            "--realm" => config.realm_name = value("--realm")?,
            "--content" => content_path = Some(value("--content")?.into()),
            "--saves" => config.saves = Some(value("--saves")?.into()),
            "--login-port" => config.login_port = value("--login-port")?.parse()?,
            "--world-port" => config.world_port = value("--world-port")?.parse()?,
            "--game" => game = value("--game")?.to_lowercase(),
            "--survival-port" => survival_port = value("--survival-port")?.parse()?,
            "--survival-save" => survival_save = value("--survival-save")?.into(),
            "--survival-content" => survival_content = Some(value("--survival-content")?.into()),
            "--help" | "-h" => {
                print!("{HELP}");
                return Ok(());
            }
            other => bail!("unknown option {other}\n\n{HELP}"),
        }
    }
    let Some(data) = data else {
        bail!("tell me where your 1.12.1 game data is: --data \"/path/to/WoW/Data\"\n\n{HELP}");
    };
    let (basin_on, survival_on) = match game.as_str() {
        "both" => (true, true),
        "basin" | "bb" => (true, false),
        "survival" | "elwynn" => (false, true),
        other => bail!("--game is both, basin or survival (not {other})"),
    };
    let basin = if basin_on {
        let content = match content_path {
            Some(p) => Content::load(&p)?,
            None => Content::builtin(),
        };
        println!("Reading the arena from {} ...", data.display());
        let loaded = htl_map::load(&data)?;
        let lanes: Vec<&str> = loaded.arena.lanes.iter().map(|l| l.name.as_str()).collect();
        println!("Arena ready: lanes from {}.", lanes.join(", "));
        // Every icon the add-on draws: cards, class spells, talents and talent trees.
        let mut spells: Vec<u32> = content.cards.iter().flat_map(|c| [c.spell, c.aura()]).collect();
        for kit in &content.kits {
            spells.extend(kit.spells.iter().map(|s| s.icon));
            spells.extend(kit.talents.iter().map(|t| t.icon));
            spells.extend(kit.trees.iter().map(|t| t.icon));
        }
        spells.extend(content.powers.iter().flat_map(|p| [p.icon, p.buff_spell()]));
        spells.extend(htl_sim::SHOP_ICONS);
        config.icons = htl_map::spell_icons(&data, &spells).unwrap_or_else(|e| {
            eprintln!("No spell icons for the card picker: {e:#}");
            Default::default()
        });
        Some(htl_server::Basin { content, arena: loaded.arena, ground: Box::new(loaded.ground) })
    } else {
        None
    };
    if survival_on {
        let content = match &survival_content {
            Some(p) => htl_survival::Content::load(p)?,
            None => htl_survival::Content::builtin(),
        };
        println!("Reading Elwynn Forest from {} ...", data.display());
        let loaded = htl_map::elwynn::load(&data)?;
        println!("Elwynn ready: {} trees around Goldshire.", loaded.trunks.len());
        // The dungeons below, each on its own instance map: without its ground, its page stays shut.
        let mut below: Vec<(String, Box<dyn htl_survival::Land>)> = Vec::new();
        for d in content.dungeons() {
            match htl_map::deadmines::load_instance_with(&data, &d.map_dir, 1.0, d.window.map(|w| (w[0], w[1]))) {
                Ok(g) => {
                    println!("{} ready: {} floors.", d.name, g.ground.floor_count());
                    below.push((d.key.clone(), Box::new(htl_server::survival::land::DeadminesLand::new(g))));
                }
                Err(e) => eprintln!("No {} ({e:#}): its page in the book stays shut.", d.name),
            }
        }
        // Arathi Basin, for Siege: without it, the book's Siege page says so.
        let siege: Option<Box<dyn htl_survival::Land>> = match htl_map::siege::load(&data) {
            Ok(a) => {
                println!("Arathi Basin ready for Siege: {} trees.", a.trunks.len());
                Some(Box::new(htl_server::survival::land::ElwynnLand::new(a)))
            }
            Err(e) => {
                eprintln!("No Arathi Basin ({e:#}): no Siege.");
                None
            }
        };
        config.survival = Some(htl_server::SurvivalSetup {
            world_port: survival_port,
            realm_name: htl_server::SURVIVAL_REALM.into(),
            save: Some(survival_save),
            content,
            land: Box::new(htl_server::survival::land::ElwynnLand::new(loaded)),
            below,
            siege,
        });
    }
    let password = config.password.clone();
    let public = config.public_host.clone();
    let running = htl_server::start_games(config, basin)?;
    println!();
    match (basin_on, running.survival) {
        (true, Some(sv)) => {
            println!("Basin Battle and Elwynn Survival are running.");
            println!("  Login server: port {}   Basin Battle: port {}   Elwynn Survival: port {}", running.login.port(), running.world.port(), sv.port());
        }
        (true, None) => {
            println!("Basin Battle is running.");
            println!("  Login server: port {}   World server: port {}", running.login.port(), running.world.port());
        }
        (false, _) => {
            println!("Elwynn Survival is running.");
            println!("  Login server: port {}   World server: port {}", running.login.port(), running.world.port());
        }
    }
    println!("  Password: {password}");
    match public {
        Some(host) => println!("  Friends on the internet connect to: {host}"),
        None => println!("  Friends outside your network: start with --public-host <your address>"),
    }
    println!("Press Ctrl+C to stop.");
    loop {
        std::thread::park();
    }
}
