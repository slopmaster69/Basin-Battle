//! A warned restart, end to end: asked for by the request file, a hero in Elwynn Survival sees
//! the warnings as banners, the camp is saved, and the server is ready to go. A file of its own:
//! a restart is for the whole process, so no other test may share it.

use std::path::PathBuf;
use std::time::Duration;

use benilla_protocol::messages::{self, opcode as op, ServerPacket};
use htl_server::{restart, Config, SurvivalSetup};

mod common;
use common::{addon_lines, Client};

const REALM: &str = "Elwynn Survival";

fn enter(c: &mut Client) -> u64 {
    c.send(op::CMSG_CHAR_ENUM, &[]);
    let none = match c.wait(3.0, |p| matches!(p, ServerPacket::CharEnum { .. })) {
        Some(ServerPacket::CharEnum { characters }) => characters.is_empty(),
        _ => panic!("no character list"),
    };
    if none {
        let mut b = b"Rhea".to_vec();
        b.push(0);
        b.extend_from_slice(&[1, 1, 0, 0, 1, 2, 3, 0, 0]);
        c.send(op::CMSG_CHAR_CREATE, &b);
        assert!(c.wait(3.0, |p| matches!(p, ServerPacket::CharCreate { result: 0x2E })).is_some(), "made");
    }
    c.enter_with(|_: &[messages::Character]| 0)
}

#[test]
fn a_restart_asked_for_warns_the_heroes_saves_the_camp_and_is_ready_to_go() {
    let dir = std::env::temp_dir().join(format!("htl-restart-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let save: PathBuf = dir.join("elwynn-survival.toml");
    let _ = std::fs::remove_file(&save);
    let config = Config {
        bind: "127.0.0.1".into(),
        login_port: 0,
        world_port: 0,
        survival: Some(SurvivalSetup {
            world_port: 0,
            realm_name: REALM.into(),
            save: Some(save.clone()),
            content: htl_survival::Content::builtin(),
            land: Box::new(htl_survival::FlatLand::default()),
            below: Vec::new(),
            siege: None,
        }),
        ..Config::default()
    };
    let server = htl_server::start_games(config, None).expect("server starts");
    let mut c = Client::connect(server.login, "rhea", "hold", REALM).unwrap();
    enter(&mut c);
    c.chat(".es hello 1");
    c.drain(0.5);

    // The host's request: "0.05" minutes, three seconds.
    std::fs::write(dir.join(restart::REQUEST_FILE), "0.05").unwrap();
    assert!(restart::poll_request(&dir).is_some_and(|s| s.contains("asked for")));
    assert!(!dir.join(restart::REQUEST_FILE).exists(), "the request is taken");
    let banner = |c: &Client, words: &str| addon_lines(c, "ES").iter().any(|l| l.starts_with("BN\t") && l.contains(words));
    assert!(c.until(2.0, |c| banner(c, "Server restart in 3 seconds")), "warned: {:?}", addon_lines(&c, "ES"));
    assert!(!restart::ready_to_exit(1), "not before its time");
    assert!(c.until(5.0, |c| banner(c, "The server is restarting")), "the time: {:?}", addon_lines(&c, "ES"));
    // Saved, and so ready to go (well before the grace for a game that never says so).
    let mut ready = false;
    for _ in 0..20 {
        if restart::ready_to_exit(1) {
            ready = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(ready, "every game saved: ready to restart");
    assert!(save.exists(), "the camp is saved");

    // A restart called off says so, and isn't ready.
    restart::ask(Duration::from_secs(60));
    assert!(c.until(2.0, |c| banner(c, "Server restart in 1 minute")));
    std::fs::write(dir.join(restart::REQUEST_FILE), "cancel").unwrap();
    assert!(restart::poll_request(&dir).is_some_and(|s| s.contains("called off")));
    assert!(c.until(2.0, |c| banner(c, "The restart is called off")));
    assert!(!restart::ready_to_exit(1));
    let _ = std::fs::remove_dir_all(&dir);
}
