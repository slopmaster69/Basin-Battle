//! Elwynn Survival against benilla's own client code, end to end: its realm beside Basin Battle's,
//! a hero entering Goldshire, the action inputs the add-on sends and what the client is shown,
//! gathering and banking, building and cooking, falling and getting up, and the world kept
//! across a restart. Every packet the server sends must parse cleanly.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use benilla_protocol::messages::{self, opcode as op, ObjectType, ServerPacket};
use htl_server::{Config, SurvivalSetup};

mod common;
use common::{addon_lines, Client};

const REALM: &str = "Elwynn Survival";

fn content() -> htl_survival::Content {
    let mut c = htl_survival::Content::builtin();
    // Enough to build with from the start, and quick to get up again.
    c.start.items.insert("wood".into(), 16);
    c.start.items.insert("meat".into(), 4);
    c.start.items.insert("ore".into(), 6);
    c.player.respawn_secs = 1.5;
    c.raid.warning = 2.0;
    // Co-op rules here; the PvP test turns them back on.
    c.pvp = Default::default();
    // Every card to try (the bench's too), as a dev world has them.
    c.dev.full_tree = true;
    // The camp's work at once (its own test times it).
    c.work = htl_survival::content::WorkDef::instant();
    c
}

fn start_with(save: Option<PathBuf>, basin: bool) -> htl_server::Running {
    start_content(save, basin, content())
}

fn start_content(save: Option<PathBuf>, basin: bool, content: htl_survival::Content) -> htl_server::Running {
    start_below(save, basin, content, &["deadmines"])
}

/// A server with these dungeons below (flat ground for each).
fn start_below(save: Option<PathBuf>, basin: bool, content: htl_survival::Content, below: &[&str]) -> htl_server::Running {
    let config = Config {
        bind: "127.0.0.1".into(),
        login_port: 0,
        world_port: 0,
        survival: Some(SurvivalSetup {
            world_port: 0,
            realm_name: REALM.into(),
            save,
            content,
            land: Box::new(htl_survival::FlatLand::default()),
            below: below.iter().map(|k| (k.to_string(), Box::new(htl_survival::FlatLand::default()) as Box<dyn htl_survival::Land>)).collect(),
            siege: Some(Box::new(htl_survival::FlatLand::default())),
        }),
        ..Config::default()
    };
    let basin = basin.then(|| htl_server::Basin {
        content: htl_sim::Content::builtin(),
        arena: htl_sim::Arena {
            map_id: 529,
            area_id: 3358,
            hub: [0.0, 0.0, 0.0],
            hub_facing: 0.0,
            player_spawn: [8.0, 0.0, 0.0],
            posts: Vec::new(),
            map_rect: None,
            player_facing: 3.14,
            lanes: vec![htl_sim::Lane::new("the north", vec![[60.0, 0.0, 0.0], [30.0, 0.0, 0.0], [4.0, 0.0, 0.0]])],
            bases: Vec::new(),
        },
        ground: Box::new(htl_sim::Flat(0.0)),
    });
    htl_server::start_games(config, basin).expect("server starts")
}

/// The account's characters.
fn list(c: &mut Client) -> Vec<messages::Character> {
    c.send(op::CMSG_CHAR_ENUM, &[]);
    match c.wait(3.0, |p| matches!(p, ServerPacket::CharEnum { .. })) {
        Some(ServerPacket::CharEnum { characters }) => characters,
        _ => panic!("no character list"),
    }
}

/// A character made on the creation screen: its name, race, class, sex and five looks; the
/// server's answer.
fn create(c: &mut Client, name: &str, race: u8, class: u8, gender: u8, look: [u8; 5]) -> u8 {
    let mut b = name.as_bytes().to_vec();
    b.push(0);
    b.extend_from_slice(&[race, class, gender]);
    b.extend_from_slice(&look);
    b.push(0);
    c.send(op::CMSG_CHAR_CREATE, &b);
    match c.wait(3.0, |p| matches!(p, ServerPacket::CharCreate { .. })) {
        Some(ServerPacket::CharCreate { result }) => result,
        _ => panic!("no answer to the character made"),
    }
}

/// Into the world as the account's first character, made first (a human warrior named for the
/// account) if it has none.
fn enter(c: &mut Client) -> u64 {
    if list(c).is_empty() {
        let mut name: String = c.user.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        name[..1].make_ascii_uppercase();
        assert_eq!(create(c, &name, 1, 1, 0, [0, 1, 2, 3, 0]), 0x2E, "{name} made");
    }
    c.enter_with(|chars| {
        assert_eq!(chars[0].map, 0, "on the Eastern Kingdoms");
        0
    })
}

/// Everything created so far: guid, type, where, and its fields (entry 3, display 8 for objects).
fn placed(c: &Client) -> Vec<(u64, ObjectType, [f32; 3], Vec<(u16, u32)>)> {
    let mut out = Vec::new();
    for p in &c.seen {
        if let ServerPacket::UpdateObject { objects, .. } = p {
            for o in objects {
                if let messages::Object::Create { guid, object_type, movement, mask } = o {
                    let pos = movement.position.map_or([0.0; 3], |(v, _)| [v.x, v.y, v.z]);
                    let fields: Vec<(u16, u32)> = mask.raw_fields().map(|(i, v)| (i as u16, v)).collect();
                    out.push((*guid, *object_type, pos, fields));
                }
            }
        }
    }
    out
}

fn field(f: &[(u16, u32)], i: u16) -> Option<u32> {
    f.iter().rev().find(|(k, _)| *k == i).map(|(_, v)| *v)
}

/// The game objects showing `display`, by guid and place.
fn objects_with(c: &Client, display: u32) -> Vec<(u64, [f32; 3])> {
    let gone: Vec<u64> = c.seen.iter().filter_map(|p| if let ServerPacket::DestroyObject { guid } = p { Some(*guid) } else { None }).collect();
    let mut v: Vec<(u64, [f32; 3])> = placed(c)
        .into_iter()
        .filter(|(g, t, _, f)| *t == ObjectType::GameObject && field(f, 8) == Some(display) && !gone.contains(g))
        .map(|(g, _, p, _)| (g, p))
        .collect();
    v.dedup_by_key(|x| x.0);
    v
}

fn said(c: &Client, text: &str) -> bool {
    c.seen.iter().any(|p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains(text)))
}

/// Face (and stand) somewhere: a heartbeat with a facing.
fn stand(c: &mut Client, pos: [f32; 3], facing: f32) {
    moving(c, pos, facing, 0);
}

/// A heartbeat with movement flags (`0x1` running forward).
fn moving(c: &mut Client, pos: [f32; 3], facing: f32, flags: u32) {
    let mut b = Vec::new();
    b.extend_from_slice(&flags.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in pos {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&facing.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    c.send(op::MSG_MOVE_HEARTBEAT, &b);
}

fn hud(c: &Client) -> Option<Vec<String>> {
    addon_lines(c, "ES").into_iter().rev().find(|l| l.starts_with("H\t")).map(|l| l.split('\t').map(str::to_string).collect())
}

/// The board (N) for a hero who hasn't learned the cards is over 32 KB, as every card it lacks
/// says where it's learned: more than a two-byte header holds. It must come in the client's
/// three-byte form, and the stream stay in step after it. (Sent with two bytes, the client read
/// one byte too many and garbage from then on: the game stopped answering anything after N.)
#[test]
fn a_board_too_big_for_a_short_header_arrives_and_what_follows_still_reads() {
    let server = start_content(None, false, {
        let mut c = content();
        c.dev.full_tree = false;
        c
    });
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 1");
    assert!(a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with("V\t"))), "the add-on is greeted");
    a.seen.clear();
    a.chat(".es tree");
    assert!(a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with("TR\t"))), "the board comes");
    let tr = addon_lines(&a, "ES").into_iter().find(|l| l.starts_with("TR\t")).unwrap();
    assert!(tr.len() > 0x7fff, "a board over 32 KB, as a new hero's is: {} bytes", tr.len());
    assert!(tr.matches("~0~").count() > 4, "most cards not learned yet");
    // What comes after it reads too: the server's answer to another hello.
    a.seen.clear();
    a.chat(".es hello 1");
    assert!(a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with("V\t"))), "the next line reads");
}

#[test]
fn both_realms_are_listed_and_basin_battle_still_plays() {
    let server = start_with(None, true);
    let logon = benilla_protocol::logon(&format!("127.0.0.1:{}", server.login.port()), "dora", "hold", u32::from_be_bytes(*b"enUS")).unwrap();
    let names: Vec<&str> = logon.realms.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, ["Basin Battle", REALM]);
    assert_ne!(logon.realms[0].address, logon.realms[1].address, "each game has its own world port");
    // Basin Battle is as it was: four classes in the Basin.
    let mut bb = Client::connect(server.login, "dora", "hold", "Basin Battle").expect("into Basin Battle");
    bb.enter_with(|chars| {
        assert_eq!(chars.len(), 4);
        assert!(chars.iter().all(|c| c.map == 529));
        0
    });
    // The survival add-on's hello is ignored there, quietly.
    bb.chat(".es hello 1");
    bb.drain(0.5);
    assert!(!addon_lines(&bb, "ES").iter().any(|l| l.starts_with('V')), "Basin Battle doesn't answer the survival add-on");
    assert!(!said(&bb, "Commands:"), "nor prints its help for it");
    // And the Basin's add-on's hello is ignored in Elwynn.
    let mut sv = Client::connect(server.login, "erin", "hold", REALM).expect("into Elwynn");
    enter(&mut sv);
    sv.chat(".bb hello 4");
    sv.drain(0.5);
    assert!(addon_lines(&sv, "BB").is_empty() && addon_lines(&sv, "ES").is_empty(), "no reply to the Basin's add-on");
}

#[test]
fn a_hero_fights_gathers_builds_cooks_falls_and_gets_up() {
    let server = start_with(None, false);
    let mut c = Client::connect(server.login, "alice", "hold", REALM).expect("alice logs in");
    let me = enter(&mut c);
    c.drain(0.5);
    let all = placed(&c);
    let (_, _, spawn, mine) = all.iter().find(|(g, ..)| *g == me).cloned().expect("our own create");
    // A footman's look, energy as stamina (power type 3 in byte 3), full.
    assert_eq!(field(&mine, 131), Some(49), "human male");
    assert_eq!(field(&mine, 36).map(|b| b >> 24), Some(3), "energy bar");
    assert_eq!(field(&mine, 23 + 3), Some(100), "stamina full");
    assert_eq!(field(&mine, 164).map(|b| b & 0xFF), Some(1), "axe and shield out");
    // The axe, shield and bow are worn; the starting roast and the materials are in the bags.
    assert!(field(&mine, 260 + 12 * 15).is_some_and(|e| e > 0), "an axe in hand");
    let items = all.iter().filter(|(_, t, ..)| *t == ObjectType::Item).count();
    assert!(items >= 8 + 3, "worn gear and bag stacks: {items}");
    // The camp: its chest and bell; the land's creatures and timber.
    assert_eq!(objects_with(&c, 10).len(), 1, "the camp chest");
    assert_eq!(objects_with(&c, 4571).len(), 1, "the alarm bell");
    assert!(objects_with(&c, 1248).len() >= 5, "timber");
    assert!(all.iter().filter(|(_, t, ..)| *t == ObjectType::Unit).count() >= 8, "Elwynn's creatures");
    // Templates come fresh: items, creatures, objects.
    assert!(c.seen.iter().any(|p| matches!(p, ServerPacket::ItemQueryResponse { .. })));
    assert!(c.seen.iter().any(|p| matches!(p, ServerPacket::CreatureQueryResponse { .. })));
    assert!(c.seen.iter().any(|p| p.name() == "SMSG_GAMEOBJECT_QUERY_RESPONSE"));

    // The add-on wakes on the server's answer, and the HUD follows.
    c.chat(".es hello 1");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "ES\tV\t2")).is_some(), "the hello is answered");
    c.drain(0.4);
    let h = hud(&c).expect("a HUD line");
    assert_eq!(h.len(), 40, "{h:?}");
    // And which way north is, relative to the hero's heading (facing 0 is north).
    let north: i32 = h[39].parse().unwrap();
    assert!((-180..=180).contains(&north), "{h:?}");
    // The corner points to the nearest place not yet found (Goldshire, next to the camp).
    assert_eq!(h[36], "Goldshire", "{h:?}");
    assert_eq!(h[1], "200");
    assert_eq!(h[3], "100");
    assert_eq!(h[5], "melee");

    // A swing at nothing: the swing plays, stamina is spent, the action bar shows it.
    c.chat(".es in a");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::Emote { guid, .. } if *guid == me)).is_some(), "the swing plays");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tA\tswing\t"))).is_some(), "the action bar");
    c.drain(0.6);
    let spent = c.seen.iter().any(|p| match p {
        ServerPacket::UpdateObject { objects, .. } => objects.iter().any(|o| {
            matches!(o, messages::Object::Values { guid, mask } if *guid == me && mask.raw_fields().any(|(i, v)| i == 26 && v < 100))
        }),
        _ => false,
    });
    assert!(spent, "stamina spent on the swing");
    // The shield goes up: slower.
    c.chat(".es in g");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::ForceSpeedChange { .. })).is_some(), "guarding slows you");
    c.chat(".es in gu");
    // A dodge hops the hero (a step back with no keys held).
    c.drain(1.0);
    c.chat(".es in d");
    assert!(c.wait(1.0, |p| p.name() == "SMSG_MOVE_KNOCK_BACK").is_some(), "the dodge's dash");
    c.drain(0.6);
    // No Sprint cue with it (the roll is its own sound now).
    assert!(!c.seen.iter().any(|p| matches!(p, ServerPacket::SpellGo(g) if g.spell_id == 2983)), "no Sprint whoosh");
    // Running, a dodge dashes along the ground: both run speeds up for a moment, and no throw
    // (a flight that fast could come to rest up in a tree's roots).
    c.drain(1.5);
    moving(&mut c, [spawn[0], spawn[1], spawn[2]], 0.0, 0x1);
    c.drain(0.2);
    let mark = c.seen.len();
    c.chat(".es in d");
    let fast = |p: &ServerPacket, kind: messages::SpeedKind| matches!(p, ServerPacket::ForceSpeedChange { kind: k, speed, .. } if *k == kind && *speed > 10.0);
    assert!(c.wait(1.0, |p| fast(p, messages::SpeedKind::RunBack)).is_some(), "the dash, backwards too");
    assert!(c.seen[mark..].iter().any(|p| fast(p, messages::SpeedKind::Run)), "the dash");
    assert!(!c.seen[mark..].iter().any(|p| p.name() == "SMSG_MOVE_KNOCK_BACK"), "no throw");
    assert!(
        c.wait(1.0, |p| matches!(p, ServerPacket::ForceSpeedChange { kind: messages::SpeedKind::RunBack, speed, .. } if *speed < 5.0)).is_some(),
        "and the speeds come back"
    );
    moving(&mut c, [spawn[0], spawn[1], spawn[2]], 0.0, 0);
    // Sprinting (Shift held) while running: faster, with no look or sound of its own.
    let here = [spawn[0], spawn[1], spawn[2]];
    moving(&mut c, here, 0.0, 0x1);
    let mark = c.seen.len();
    c.chat(".es in sp");
    assert!(c.wait(1.0, |p| fast(p, messages::SpeedKind::Run)).is_some(), "sprint's pace");
    assert!(!c.seen[mark..].iter().any(|p| matches!(p, ServerPacket::SpellGo(g) if g.spell_id == 2983)), "no sprint look");
    c.chat(".es in spu");
    stand(&mut c, here, 0.0);
    c.drain(0.5);
    // The ability bar arrives; Aimed Shot (off the bench, on Q) draws the bow for one shot, and
    // it's put away.
    assert!(c.seen.iter().any(|p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tAB\tranged~hurled_axe~"))), "the bar");
    c.chat(".es slot q aimed_shot");
    c.drain(0.2);
    c.chat(".es in ab 0 0");
    c.chat(".es in a");
    let bow = c.wait(1.0, |p| match p {
        ServerPacket::UpdateObject { objects, .. } => objects.iter().any(|o| {
            matches!(o, messages::Object::Values { guid, mask } if *guid == me && mask.raw_fields().any(|(i, v)| i == 164 && v & 0xFF == 2))
        }),
        _ => false,
    });
    assert!(bow.is_some(), "the bow drawn");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::SpellStart(g) if g.spell_id == 75)).is_some(), "everyone sees the draw");
    c.drain(0.8);
    c.chat(".es in au 0.0 0.0");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::SpellGo(g) if g.spell_id == 75)).is_some(), "loosed");
    c.drain(1.0);
    // A Frostbolt on Q: charged, let go along the reticle, flown to a point.
    c.chat(".es slot q frostbolt");
    c.drain(0.2);
    c.chat(".es in ab 0 0");
    c.chat(".es in a");
    c.drain(0.8);
    c.chat(".es in au 0.0 -0.05");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::SpellGo(g) if g.spell_id == 116)).is_some(), "the Frostbolt flies");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tAB\tranged~frostbolt~"))).is_some(), "its cooldown on the bar");
    c.drain(0.5);

    // Timber: right-click the nearest pile and wait out the gather.
    let (node, at) = objects_with(&c, 1248).into_iter().min_by(|a, b| {
        let d = |p: [f32; 3]| (p[0] - spawn[0]).hypot(p[1] - spawn[1]);
        d(a.1).total_cmp(&d(b.1))
    }).unwrap();
    stand(&mut c, [at[0] + 1.5, at[1], at[2]], std::f32::consts::PI);
    c.drain(0.2);
    c.send(op::CMSG_GAMEOBJ_USE, &node.to_le_bytes());
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tA\tgather\t"))).is_some(), "gathering");
    assert!(c.wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("Elwynn Timber") && m.text.contains("carried"))).is_some(), "timber in the bags");

    // Bank it at the chest.
    let (chest, chest_at) = objects_with(&c, 10)[0];
    stand(&mut c, [chest_at[0] + 2.0, chest_at[1], chest_at[2]], std::f32::consts::PI);
    c.drain(0.2);
    c.send(op::CMSG_GAMEOBJ_USE, &chest.to_le_bytes());
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.trim_start_matches("ES\tT\t").starts_with("Banked:"))).is_some(), "banked");
    let s = c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tS\t")));
    match s {
        Some(ServerPacket::MessageChat(m)) => assert!(m.text.contains("wood~Elwynn Timber~"), "{}", m.text),
        _ => panic!("the chest's window"),
    }
    // Some back out, to build with and to carry.
    c.chat(".es take wood 10");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tS\t"))).is_some(), "the chest's window refreshed");

    // Build a cooking fire on the meadow: the menu, the ghost and its ring, then placed.
    c.chat(".es bm");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBM\tcooking~"))).is_some(), "the build menu");
    let site = [-9534.0f32, 122.0, 0.0];
    stand(&mut c, [site[0] - 4.0, site[1], 0.0], 0.0);
    c.drain(0.2);
    c.chat(".es build cooking");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBP\tCooking Fire\t") && m.text.split('\t').nth(4) == Some(""))).is_some(), "a good spot: {:?}", addon_lines(&c, "ES").last());
    assert!(!objects_with(&c, 674).is_empty(), "the green ring under it");
    c.chat(".es place");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("builds a Cooking Fire"))).is_some(), "built");
    c.drain(0.3);
    let fires = objects_with(&c, 192);
    assert_eq!(fires.len(), 1, "one fire (the ghost gone)");
    assert!(objects_with(&c, 674).is_empty(), "the ring gone");
    // Cook: the fire's window, then a roast.
    c.send(op::CMSG_GAMEOBJ_USE, &fires[0].0.to_le_bytes());
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tW\t") && m.text.contains("roast~"))).is_some(), "the fire's window");
    c.chat(".es craft roast");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You make 1"))).is_some(), "a roast");

    // A long fall: down, supplies left where they fell, up again at the camp.
    stand(&mut c, [-9560.0, 160.0, 120.0], 0.0);
    let mut b = Vec::new();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in [-9560.0f32, 160.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0f32.to_le_bytes());
    b.extend_from_slice(&6000u32.to_le_bytes());
    c.send(op::MSG_MOVE_FALL_LAND, &b);
    assert!(c.wait(1.0, |p| p.name() == "SMSG_ENVIRONMENTALDAMAGELOG").is_some(), "the fall");
    let down = c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tDE\t")));
    // And the recap says what felled us: the fall, and a word on it.
    let down = match down {
        Some(ServerPacket::MessageChat(m)) => m.text,
        _ => panic!("down"),
    };
    assert!(down.contains("\tFall (") && down.ends_with("\tMind the drop."), "the recap: {down:?}");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("waits where you fell"))).is_some(), "supplies left");
    assert_eq!(objects_with(&c, 2890).len(), 1, "the sack where we fell");
    assert!(c.wait(3.0, |p| p.name() == "MSG_MOVE_TELEPORT_ACK").is_some(), "up at the camp");
    c.drain(0.4);
    let h = hud(&c).unwrap();
    assert!(h[9].parse::<i32>().unwrap() > 20, "the sack is far off now: {h:?}");
    // ADDON_DUMP=<file>: every add-on line this session saw, for the add-on's own harness.
    if let Some(path) = std::env::var_os("ADDON_DUMP") {
        c.chat(".es bm");
        c.chat(".es build smithy");
        c.drain(0.5);
        std::fs::write(path, addon_lines(&c, "ES").join("\n")).unwrap();
    }
}

#[test]
fn the_world_is_kept_across_a_restart() {
    let dir = std::env::temp_dir().join(format!("es-test-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let save = dir.join("world.toml");
    {
        let server = start_with(Some(save.clone()), false);
        let mut c = Client::connect(server.login, "bob", "hold", REALM).unwrap();
        enter(&mut c);
        // Off the building ground: refused, with the reason, and nothing paid.
        stand(&mut c, [-9600.0, 60.0, 0.0], 0.0);
        c.drain(0.2);
        c.chat(".build cooking");
        c.drain(0.3);
        c.chat(".place");
        assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("Outside the camp's building ground"))).is_some(), "refused");
        c.chat(".cancel");
        stand(&mut c, [-9538.0, 122.0, 0.0], 0.0);
        c.drain(0.2);
        c.chat(".build cooking");
        c.drain(0.2);
        c.chat(".place");
        assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("builds a Cooking Fire"))).is_some(), "built");
        stand(&mut c, [-9520.0, 110.0, 0.0], 1.0);
        c.send(op::CMSG_LOGOUT_REQUEST, &[]);
        assert!(c.wait(2.0, |p| p.name() == "SMSG_LOGOUT_COMPLETE").is_some());
        let end = Instant::now() + Duration::from_secs(3);
        while !save.exists() && Instant::now() < end {
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    let text = std::fs::read_to_string(&save).expect("the world is saved");
    assert!(text.contains("cooking"), "{text}");
    // Bob as a save from before characters had their own: kept under his account, no name, class
    // nor look of his own.
    let mut old = String::new();
    let mut skip = false;
    for line in text.lines() {
        if line.starts_with('[') {
            skip = line.ends_with(".look]");
        }
        if skip || ["owner =", "name =", "class ="].iter().any(|k| line.starts_with(k)) {
            continue;
        }
        old.push_str(&line.replace("\"bob/bob\"", "bob"));
        old.push('\n');
    }
    std::fs::write(&save, old).unwrap();
    // A new server on the same file: the fire is still there, and Bob (his first footman, a
    // human warrior) is where he left.
    let server = start_with(Some(save.clone()), false);
    let mut c = Client::connect(server.login, "bob", "hold", REALM).unwrap();
    let chars = list(&mut c);
    assert_eq!(chars.len(), 1);
    assert_eq!((chars[0].name.as_str(), chars[0].race, chars[0].class, chars[0].gender), ("Bob", 1, 1, 0));
    let me = enter(&mut c);
    c.drain(0.5);
    assert_eq!(objects_with(&c, 192).len(), 1, "the cooking fire stands");
    let (_, _, pos, _) = placed(&c).into_iter().find(|(g, ..)| *g == me).unwrap();
    assert!((pos[0] + 9520.0).abs() < 0.5 && (pos[1] - 110.0).abs() < 0.5, "where he left: {pos:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// Where a creature is now: its last walk's end, or where it was made.
fn unit_at(c: &Client, guid: u64) -> Option<[f32; 3]> {
    let mut at = placed(c).into_iter().find(|(g, ..)| *g == guid).map(|(_, _, p, _)| p);
    for p in &c.seen {
        if let ServerPacket::MonsterMove { guid: g, start, path, .. } = p {
            if *g == guid {
                let v = path.last().copied().unwrap_or(*start);
                at = Some([v.x, v.y, v.z]);
            }
        }
    }
    at
}

#[test]
fn blows_land_on_a_boar_and_the_shield_blocks_its_charge() {
    let server = start_with(None, false);
    let mut c = Client::connect(server.login, "cass", "hold", REALM).expect("cass logs in");
    let me = enter(&mut c);
    c.chat(".es hello 1");
    c.drain(0.5);
    let boar_entry = 60_000 + htl_survival::Content::builtin().mob("boar").unwrap() as u32;
    let boar = placed(&c)
        .into_iter()
        .find(|(g, t, ..)| *t == ObjectType::Unit && ((g >> 24) & 0xFF_FFFF) as u32 == boar_entry)
        .map(|(g, ..)| g)
        .expect("a boar in the forest");
    // Up to it, facing it, and swing until a blow lands.
    let face = |c: &mut Client| {
        let b = unit_at(c, boar).unwrap();
        let from = [b[0] - 1.6, b[1], b[2]];
        stand(c, from, 0.0);
    };
    let mut hit = false;
    for _ in 0..12 {
        face(&mut c);
        c.drain(0.1);
        c.chat(".es in a");
        if c.wait_exactly(1.0, |p| matches!(p, ServerPacket::AttackerState(s) if s.attacker == me && s.victim == boar && s.damage > 0)).is_some() {
            hit = true;
            break;
        }
    }
    assert!(hit, "the axe bites");
    // Shield up, facing it as it comes: its blow is blocked.
    c.drain(1.0);
    c.chat(".es in g");
    let end = Instant::now() + Duration::from_secs(10);
    let mut blocked = false;
    while Instant::now() < end {
        face(&mut c);
        if c.wait_exactly(0.3, |p| matches!(p, ServerPacket::AttackerState(s) if s.attacker == boar && s.victim == me && s.victim_state == 5)).is_some() {
            blocked = true;
            break;
        }
    }
    assert!(blocked, "the shield takes the tusks: {:?}", c.seen.iter().filter_map(|p| if let ServerPacket::AttackerState(s) = p { Some((s.attacker == me, s.victim_state, s.damage)) } else { None }).collect::<Vec<_>>());
    assert!(c.seen.iter().any(|p| matches!(p, ServerPacket::Emote { guid, .. } if *guid == boar)), "its charge plays");
}

#[test]
fn two_heroes_share_the_camp_race_for_a_node_and_ring_the_bell() {
    // No wild creatures (as a Siege world has none): timber lies at random in three spots, one
    // of them on a Darkspear Berserker patrol, and heroes sent to a pile there fell before they
    // could use it (#2). The raid brings its own.
    let mut rules = content();
    rules.packs.clear();
    rules.roams.clear();
    rules.camps.clear();
    let spawn = rules.world.spawn;
    let server = start_content(None, false, rules);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let ag = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let bg = enter(&mut b);
    a.chat(".es hello 1");
    b.chat(".es hello 1");
    // Each sees the other.
    assert!(a.wait(2.0, |p| creates_guid(p, bg)).is_some(), "ann sees ben");
    assert!(b.until(0.5, |c| placed(c).iter().any(|(g, ..)| *g == ag)), "ben sees ann");
    // For a failure to say what went on: who felled whom (the kill feed) and their falls and
    // gettings-up from the start, then what a hero's been told lately and their bar.
    let story = |c: &Client| {
        let lines = addon_lines(c, "ES");
        let mut l: Vec<String> = lines.iter().filter(|l| l.starts_with("KF\t") || l.starts_with("DE\t")).cloned().collect();
        let mut recent: Vec<String> = lines.into_iter().filter(|l| l.starts_with("T\t") || l.starts_with("H\t")).collect();
        let last = recent.len().saturating_sub(6);
        l.append(&mut recent.split_off(last));
        l
    };

    // Both at the same timber pile, both using it at once: two uses between them, then it's gone
    // for both, and nobody gets a third. The pile nearest them, not whichever was sent first.
    let near = |p: &[f32; 3]| (p[0] - spawn[0]).powi(2) + (p[1] - spawn[1]).powi(2);
    let (node, at) = objects_with(&a, 1248).into_iter().min_by(|x, y| near(&x.1).total_cmp(&near(&y.1))).expect("a timber pile");
    for c in [&mut a, &mut b] {
        stand(c, [at[0] + 1.5, at[1], at[2]], std::f32::consts::PI);
    }
    a.drain(0.2);
    a.send(op::CMSG_GAMEOBJ_USE, &node.to_le_bytes());
    b.send(op::CMSG_GAMEOBJ_USE, &node.to_le_bytes());
    assert!(
        a.wait(6.0, |p| matches!(p, ServerPacket::DestroyObject { guid } if *guid == node)).is_some(),
        "the pile is spent for ann.\nann: {:?}\nben: {:?}",
        story(&a),
        story(&b)
    );
    assert!(b.wait(1.0, |p| matches!(p, ServerPacket::DestroyObject { guid } if *guid == node)).is_some() || b.seen.iter().any(|p| matches!(p, ServerPacket::DestroyObject { guid } if *guid == node)), "and for ben");
    let got = |c: &Client| c.seen.iter().filter(|p| matches!(p, ServerPacket::MessageChat(m) if m.text.trim_start_matches("ES\tT\t").starts_with('+') && m.text.contains("Timber"))).count();
    // Each hero's "+Timber" can come a little after the pile goes.
    let end = Instant::now() + Duration::from_secs_f32(0.6 * common::SLACK);
    while got(&a) + got(&b) < 2 && Instant::now() < end {
        a.drain(0.05);
        b.drain(0.05);
    }
    assert_eq!(got(&a) + got(&b), 2, "two gathers from a two-use pile");
    a.send(op::CMSG_GAMEOBJ_USE, &node.to_le_bytes());
    a.drain(2.0);
    assert_eq!(got(&a) + got(&b), 2, "no third");

    // The camp chest is shared: what Ann banks, Ben sees.
    let (chest, cat) = objects_with(&a, 10)[0];
    for c in [&mut a, &mut b] {
        stand(c, [cat[0] + 2.0, cat[1], cat[2]], std::f32::consts::PI);
    }
    b.drain(0.2);
    b.send(op::CMSG_GAMEOBJ_USE, &chest.to_le_bytes());
    assert!(b.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tS\t"))).is_some(), "ben's chest window");
    a.send(op::CMSG_GAMEOBJ_USE, &chest.to_le_bytes());
    assert!(
        b.wait(1.5, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tS\t") && m.text.contains("ore~"))).is_some(),
        "ben's window shows what ann banked"
    );

    // Ann builds the cooking fire and Ben the smithy: both see both.
    stand(&mut a, [-9538.0, 112.0, 0.0], 0.0);
    stand(&mut b, [-9538.0, 128.0, 0.0], 0.0);
    a.drain(0.2);
    a.chat(".es build cooking");
    b.chat(".es build smithy");
    a.drain(0.3);
    a.chat(".es place");
    b.chat(".es place");
    b.drain(0.5);
    assert!(a.wait(1.5, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("builds a Smithing Anvil"))).is_some() || said(&a, "builds a Smithing Anvil"), "the smithy: {:?}", addon_lines(&b, "ES").iter().filter(|l| l.starts_with("BP") || l.starts_with("T")).collect::<Vec<_>>());
    b.until(0.3, |c| !objects_with(c, 192).is_empty());
    a.until(0.3, |c| !objects_with(c, 166).is_empty());
    assert_eq!(objects_with(&b, 192).len(), 1, "ben sees ann's fire");
    assert_eq!(objects_with(&a, 166).len(), 1, "ann sees ben's anvil");

    // The bell: asked first, then warned, the road marked, and raiders come.
    let (bell, bat) = objects_with(&a, 4571)[0];
    stand(&mut a, [bat[0] + 2.0, bat[1], bat[2]], std::f32::consts::PI);
    a.drain(0.2);
    a.send(op::CMSG_GAMEOBJ_USE, &bell.to_le_bytes());
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tCF\traid\t"))).is_some(), "asked first");
    a.chat(".es raid yes");
    assert!(b.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBN\tWarsong Raid"))).is_some(), "ben is warned");
    assert!(b.until(0.5, |c| objects_with(c, 5812).len() >= 4), "the way they come is marked");
    let raider = b.wait(4.0, |p| matches!(p, ServerPacket::UpdateObject { .. }) && placed_in(p).iter().any(|(_, t)| *t == ObjectType::Unit));
    assert!(raider.is_some(), "raiders arrive");
    b.until(0.4, |c| hud(c).is_some_and(|h| h.get(11).is_some_and(|s| s == "active")));
    let h = hud(&b).unwrap();
    assert_eq!(h[11], "active", "{h:?}");
}

fn creates_guid(p: &ServerPacket, guid: u64) -> bool {
    placed_in(p).iter().any(|(g, _)| *g == guid)
}

fn placed_in(p: &ServerPacket) -> Vec<(u64, ObjectType)> {
    match p {
        ServerPacket::UpdateObject { objects, .. } => objects
            .iter()
            .filter_map(|o| match o {
                messages::Object::Create { guid, object_type, .. } => Some((*guid, *object_type)),
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    }
}

/// A field of one object as the latest values say.
fn value_of(c: &Client, guid: u64, field: u16) -> Option<u32> {
    let mut v = None;
    for p in &c.seen {
        if let ServerPacket::UpdateObject { objects, .. } = p {
            for o in objects {
                let (g, mask) = match o {
                    messages::Object::Values { guid, mask } => (*guid, mask),
                    messages::Object::Create { guid, mask, .. } => (*guid, mask),
                    _ => continue,
                };
                if g == guid {
                    if let Some((_, x)) = mask.raw_fields().find(|(i, _)| *i as u16 == field) {
                        v = Some(x);
                    }
                }
            }
        }
    }
    v
}

#[test]
fn the_guard_shows_its_pose_the_rack_arms_and_a_timed_guard_parries() {
    // The wolves' plain bite only, for a steady rhythm to time the guard to.
    let mut c0 = content();
    for m in c0.mobs.iter_mut() {
        m.special = None;
    }
    let server = start_content(None, false, c0);
    let mut c = Client::connect(server.login, "dain", "hold", REALM).unwrap();
    let me = enter(&mut c);
    c.chat(".es hello 1");
    c.drain(0.5);
    // The guard's pose: the shield up while it's held.
    c.chat(".es in g");
    c.drain(0.3);
    assert_eq!(value_of(&c, me, 148), Some(43), "shield up");
    c.chat(".es in gu");
    c.drain(0.3);
    assert_eq!(value_of(&c, me, 148), Some(0), "and down");
    // The rack: its window, then a claymore in both hands (no shield on the arm).
    let (rack, at) = objects_with(&c, 130)[0];
    stand(&mut c, [at[0] + 2.0, at[1], at[2]], std::f32::consts::PI);
    c.drain(0.2);
    c.send(op::CMSG_GAMEOBJ_USE, &rack.to_le_bytes());
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tAR\t") && m.text.contains("greatsword~"))).is_some(), "the rack's window");
    c.chat(".es arm greatsword");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You take up the Footman's Claymore"))).is_some());
    c.drain(0.3);
    assert_eq!(value_of(&c, me, 260 + 12 * 16), Some(0), "the off hand is empty");
    assert!(value_of(&c, me, 260 + 12 * 15).is_some_and(|e| e > 0), "the claymore in hand");
    c.chat(".es in g");
    c.drain(0.3);
    assert_eq!(value_of(&c, me, 148), Some(375), "the two-handed guard");
    c.chat(".es in gu");
    c.chat(".es arm axe");
    c.drain(0.3);

    // A wolf: keep raising the guard, freshly, until one of its bites lands just after a raise.
    let wolf_entry = 60_000 + htl_survival::Content::builtin().mob("wolf").unwrap() as u32;
    let wolf = placed(&c)
        .into_iter()
        .find(|(g, t, ..)| *t == ObjectType::Unit && ((g >> 24) & 0xFF_FFFF) as u32 == wolf_entry)
        .map(|(g, ..)| g)
        .expect("a wolf near the camp");
    let end = Instant::now() + Duration::from_secs(20);
    let mut parried = false;
    while Instant::now() < end && !parried {
        let w = unit_at(&c, wolf).unwrap();
        stand(&mut c, [w[0] - 1.8, w[1], w[2]], 0.0);
        c.chat(".es in gu");
        c.drain(0.1);
        c.chat(".es in g");
        parried = c.wait_exactly(0.65, |p| matches!(p, ServerPacket::AttackerState(s) if s.attacker == wolf && s.victim == me && s.victim_state == 3)).is_some();
    }
    assert!(parried, "a parry: {:?}", c.seen.iter().filter_map(|p| if let ServerPacket::AttackerState(s) = p { Some((s.attacker == me, s.victim_state, s.damage)) } else { None }).collect::<Vec<_>>());
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "ES\tFL\tParry!")).is_some() || addon_lines(&c, "ES").iter().any(|l| l == "FL\tParry!"), "the flash");
    assert!(c.seen.iter().any(|p| matches!(p, ServerPacket::Emote { guid, emote_id } if *guid == wolf && *emote_id == 64)), "the wolf reels");
}

#[test]
fn with_friendly_fire_heroes_strike_and_parry_each_other_and_get_up_at_once() {
    let mut content = content();
    content.pvp = htl_survival::Content::builtin().pvp;
    content.player.health = 30;
    let server = start_content(None, false, content);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let ag = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let bg = enter(&mut b);
    a.chat(".es hello 1");
    b.chat(".es hello 1");
    assert!(a.wait(2.0, |p| creates_guid(p, bg)).is_some(), "ann sees ben");
    // Face to face, two strides apart, out in the field.
    let spot = [-9500.0, 0.0, 0.0];
    stand(&mut a, spot, 0.0);
    stand(&mut b, [spot[0] + 2.0, spot[1], spot[2]], std::f32::consts::PI);
    a.drain(0.3);
    b.drain(0.1);

    // Ben keeps raising his guard, freshly, as Ann swings, until one is raised just in time.
    let end = Instant::now() + Duration::from_secs(20);
    let mut parried = false;
    while Instant::now() < end && !parried {
        b.chat(".es in gu");
        a.chat(".es in a");
        a.chat(".es in au");
        std::thread::sleep(Duration::from_millis(180));
        b.chat(".es in g");
        parried = a.wait_exactly(1.2, |p| matches!(p, ServerPacket::AttackerState(s) if s.attacker == ag && s.victim == bg && s.victim_state == 3)).is_some();
        a.drain(0.8);
    }
    assert!(parried, "a parry between heroes");
    assert!(a.wait(0.5, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "ES\tFL\tParried!")).is_some() || addon_lines(&a, "ES").iter().any(|l| l == "FL\tParried!"), "ann is told she was parried");
    assert!(a.seen.iter().any(|p| matches!(p, ServerPacket::Emote { guid, emote_id } if *guid == ag && *emote_id == 64)), "ann reels");

    // Ben strikes back with his guard down until Ann falls: announced, and up again at once.
    b.chat(".es in gu");
    let end = Instant::now() + Duration::from_secs(12);
    let mut felled = false;
    while Instant::now() < end && !felled {
        b.chat(".es in a");
        b.chat(".es in au");
        felled = a.wait_exactly(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if { let t = m.text.to_lowercase(); t.contains("ben fells ann") || t == "es\tkf\tben\tann" })).is_some();
    }
    assert!(felled, "the fall is announced: {:?}", a.seen.iter().filter_map(|p| if let ServerPacket::MessageChat(m) = p { Some(m.text.clone()) } else { None }).collect::<Vec<_>>());
    let t0 = Instant::now();
    assert!(a.wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "ES\tDE\t0")).is_some(), "up again");
    assert!(t0.elapsed() < Duration::from_millis(2000), "at once: {:?}", t0.elapsed());
    a.drain(0.3);
    assert_eq!(value_of(&a, ag, 22), Some(30), "whole");
}

#[test]
fn the_action_combat_client_gets_its_clips_impacts_and_rolls_and_others_the_emotes() {
    let mut content = content();
    content.pvp = htl_survival::Content::builtin().pvp;
    let server = start_content(None, false, content);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let ag = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let bg = enter(&mut b);
    // Ann plays clips herself; Ben's client is a stock one.
    a.chat(".es hello 5 ac2");
    b.chat(".es hello 5");
    assert!(a.wait(2.0, |p| creates_guid(p, bg)).is_some(), "ann sees ben");
    let spot = [-9500.0, 0.0, 0.0];
    stand(&mut a, spot, 0.0);
    stand(&mut b, [spot[0] + 2.0, spot[1], spot[2]], std::f32::consts::PI);
    a.drain(0.3);
    b.drain(0.3);
    // Ann swings at Ben: her own clip (marked hers), the impact; Ben sees the emote.
    a.chat(".es in a");
    a.chat(".es in au");
    a.drain(1.2);
    b.drain(0.2);
    let lines = addon_lines(&a, "ES");
    let swing = lines.iter().find(|l| l.starts_with("AN\t")).cloned().expect("ann's swing clip");
    let f: Vec<&str> = swing.split('\t').collect();
    assert_eq!(f[1], format!("{ag:#X}"));
    assert_eq!(f[2], "17", "a one-handed swing");
    assert_eq!(f[6], "1", "hers");
    let impact = lines.iter().find(|l| l.starts_with("IM\t")).cloned().expect("the impact");
    assert!(impact.starts_with(&format!("IM\t{ag:#X}\t{bg:#X}\tlight\thit\t1\t0")), "{impact}");
    assert!(b.seen.iter().any(|p| matches!(p, ServerPacket::Emote { guid, .. } if *guid == ag)), "ben sees the emote");
    assert!(!addon_lines(&b, "ES").iter().any(|l| l.starts_with("AN\t") || l.starts_with("IM\t")), "and no clips");
    // A heavy attack: Special1H.
    a.chat(".es in h");
    a.chat(".es in au");
    a.drain(1.6);
    assert!(addon_lines(&a, "ES").iter().any(|l| l.starts_with(&format!("AN\t{ag:#X}\t57\t"))), "the heavy attack's clip");
    // Ben dodges: Ann sees him roll.
    a.drain(1.0);
    b.chat(".es in d");
    a.drain(0.6);
    assert!(addon_lines(&a, "ES").iter().any(|l| *l == format!("RO\t{bg:#X}\t0.45\t0\tback")), "ben steps back (no keys held)");
}

#[test]
fn the_third_round_client_gets_its_blows_reactions_lock_on_and_runs_its_own_dodge() {
    let mut content = content();
    content.pvp = htl_survival::Content::builtin().pvp;
    let server = start_content(None, false, content);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let ag = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let bg = enter(&mut b);
    a.chat(".es hello 5 ac3");
    b.chat(".es hello 5 ac2");
    assert!(a.wait(2.0, |p| creates_guid(p, bg)).is_some(), "ann sees ben");
    let spot = [-9500.0, 0.0, 0.0];
    stand(&mut a, spot, 0.0);
    stand(&mut b, [spot[0] + 2.0, spot[1], spot[2]], std::f32::consts::PI);
    a.drain(0.5);
    b.drain(0.3);
    // Her blows, for her own instant play, and a ping to time her round trip.
    a.until(1.0, |c| {
        let l = addon_lines(c, "ES");
        l.iter().any(|l| l.starts_with("MV\t")) && l.iter().any(|l| l.starts_with("PI\t"))
    });
    let lines = addon_lines(&a, "ES");
    let mv = lines.iter().find(|l| l.starts_with("MV\t")).cloned().expect("the moves line");
    let f: Vec<&str> = mv.split('\t').collect();
    assert_eq!(f[1], "0", "one-handed");
    assert_eq!(f[10].split('^').count(), 3, "three light blows: {mv}");
    assert!(f[10].starts_with("17~0~") && f[13].starts_with("59~") && f[14].starts_with("95~"), "{mv}");
    let ping = lines.iter().find(|l| l.starts_with("PI\t")).cloned().expect("a ping");
    a.chat(&format!(".es po {}", &ping[3..]));
    // Ann's light blow: the clip, its take, recovery, lunge and kind.
    a.chat(".es in a");
    a.chat(".es in au");
    a.drain(1.2);
    b.drain(0.3);
    let swung = format!("AN\t{ag:#X}\t17\t");
    let re = format!("RE\t{bg:#X}\t9\t");
    let flinch = |c: &Client, own: &str| addon_lines(c, "ES").iter().any(|l| l.starts_with(&re) && l.ends_with(own));
    // On a busy machine the swing and flinch lines can come after the drains above.
    a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with(&swung)) && flinch(c, "\t0\t0.00"));
    let lines = addon_lines(&a, "ES");
    let swing = lines.iter().find(|l| l.starts_with(&swung)).cloned().expect("ann's swing");
    let f: Vec<&str> = swing.split('\t').collect();
    assert_eq!((f[6], f[7], f[10], f[11]), ("1", "0", "light", "0"), "{swing}");
    assert!(f[9].parse::<f32>().unwrap() > 0.0, "a lunge");
    // Ben flinches back from it, on both screens; his own marked his.
    assert!(flinch(&a, "\t0\t0.00"), "ann sees ben flinch: {:?}", lines.iter().filter(|l| l.starts_with("RE\t")).collect::<Vec<_>>());
    assert!(
        b.until(1.0, |c| flinch(c, "\t1\t0.00")),
        "ben's own: {:?}",
        addon_lines(&b, "ES").iter().filter(|l| l.starts_with("RE\t")).collect::<Vec<_>>()
    );
    // A heavy attack's flash on him.
    a.drain(1.0);
    a.chat(".es in h");
    a.chat(".es in au");
    assert!(
        b.wait(2.0, |p| matches!(p, ServerPacket::PlaySpellVisual { unit, kit_id } if *unit == bg && (*kit_id == htl_survival::KIT_HEAVY || *kit_id == htl_survival::KIT_SLAM))).is_some(),
        "the heavy blow's flash"
    );
    // The lock-on: Ben, in front of her.
    a.drain(1.5);
    a.chat(".es in lock");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == format!("ES\tLO\t{bg:#X}"))).is_some(), "locked on to ben");
    a.chat(".es in unlock");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "ES\tLO\t0x0")).is_some(), "let go");
    // Her dodge running forward is her own: no speed burst comes to her, but Ben sees her pace.
    a.drain(1.0);
    moving(&mut a, spot, 0.0, 0x1);
    a.drain(0.2);
    b.drain(0.2);
    let mark = a.seen.len();
    a.chat(".es in d");
    a.drain(0.6);
    assert!(!a.seen[mark..].iter().any(|p| matches!(p, ServerPacket::ForceSpeedChange { speed, .. } if *speed > 10.0)), "no burst for her");
    assert!(!a.seen[mark..].iter().any(|p| p.name() == "SMSG_MOVE_KNOCK_BACK"), "no throw");
    assert!(
        b.wait(1.0, |p| matches!(p, ServerPacket::SplineSpeedChange { guid, speed, .. } if *guid == ag && *speed > 10.0)).is_some(),
        "ben sees her dash"
    );
}


#[test]
fn the_fourth_round_client_flashes_its_own_blows_and_others_see_the_flash_and_its_dodges_aside() {
    let mut content = content();
    content.pvp = htl_survival::Content::builtin().pvp;
    let server = start_content(None, false, content);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let bg = enter(&mut b);
    a.chat(".es hello 5 ac4");
    b.chat(".es hello 5 ac3");
    assert!(a.wait(2.0, |p| creates_guid(p, bg)).is_some(), "ann sees ben");
    let spot = [-9500.0, 0.0, 0.0];
    stand(&mut a, spot, 0.0);
    stand(&mut b, [spot[0] + 2.0, spot[1], spot[2]], std::f32::consts::PI);
    a.drain(0.5);
    b.drain(0.3);
    // Her blows carry their reach, and friendly fire.
    a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with("MV\t")));
    let mv = addon_lines(&a, "ES").into_iter().find(|l| l.starts_with("MV\t")).expect("the moves line");
    let f: Vec<&str> = mv.split('\t').collect();
    assert_eq!(f.len(), 18, "{mv}");
    assert_eq!(f[15], "1", "friendly fire");
    // (Each blow: clip, take, impact, recovery, lunge, stamina, range, arc, and the model its
    // clip comes from, empty for her own.)
    assert_eq!(f[10].split('^').next().unwrap().split('~').count(), 9, "range and arc: {mv}");
    // Her heavy blow's flash: on Ben's screen, not hers (she played her own).
    let mark = a.seen.len();
    a.chat(".es in h");
    a.chat(".es in au");
    assert!(
        b.wait(2.5, |p| matches!(p, ServerPacket::PlaySpellVisual { unit, kit_id } if *unit == bg && (*kit_id == htl_survival::KIT_HEAVY || *kit_id == htl_survival::KIT_SLAM))).is_some(),
        "ben sees the flash"
    );
    a.drain(0.3);
    assert!(
        !a.seen[mark..].iter().any(|p| matches!(p, ServerPacket::PlaySpellVisual { kit_id, .. } if *kit_id == htl_survival::KIT_HEAVY || *kit_id == htl_survival::KIT_SLAM)),
        "not sent back to ann"
    );
    // Ben steps aside: Ann sees which way.
    b.drain(1.5);
    a.drain(0.2);
    moving(&mut b, [spot[0] + 2.0, spot[1], spot[2]], std::f32::consts::PI, 0x4);
    b.drain(0.2);
    b.chat(".es in d");
    assert!(
        a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with(&format!("ES\tRO\t{bg:#X}\t")) && m.text.ends_with("\tleft"))).is_some(),
        "a step aside to the left"
    );
}

#[test]
fn the_rules_are_tuned_in_play_and_everyone_gets_the_new_blows() {
    let server = start_content(None, false, content());
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.drain(0.6);
    a.chat(".es tune combat.chain_after 0.4");
    assert!(
        a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tT\t") && m.text.contains("combat.chain_after") && m.text.contains("0.4"))).is_some(),
        "told"
    );
    a.drain(0.5);
    let mv = addon_lines(&a, "ES").into_iter().rfind(|l| l.starts_with("MV\t")).expect("the moves line");
    assert_eq!(mv.split('\t').nth(2), Some("0.40"), "{mv}");
    // A bad one says why and changes nothing.
    a.chat(".es tune combat.chain_after lots");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tT\tcombat.chain_after"))).is_some(), "refused");
    a.chat(".es tune moveset.one_hand.light.0.lunge 1.2");
    a.drain(0.5);
    let mv = addon_lines(&a, "ES").into_iter().rfind(|l| l.starts_with("MV\t")).unwrap();
    assert!(mv.split('\t').nth(10).unwrap().starts_with("17~0~0.380~0.480~1.20"), "{mv}");
}

#[test]
fn a_champion_called_up_shows_its_bar() {
    let server = start_content(None, false, content());
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.drain(0.6);
    a.chat(".es spawn warlord");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("debug"))).is_some(), "debug first");
    a.chat(".es debug");
    a.chat(".es spawn warlord");
    a.drain(1.0);
    let bs = addon_lines(&a, "ES").into_iter().rfind(|l| l.starts_with("BS")).expect("the boss line");
    // His health: his own, times the rules' share for every foe.
    let c = htl_survival::Content::builtin();
    let hp = (c.mobs[c.mob("warlord").unwrap()].health as f32 * c.combat.foe_health).round() as u32;
    assert!(bs.starts_with(&format!("BS\tWarlord Gorzak\t{hp}\t{hp}\t0")), "{bs}");
    // The whole journal came too: every goal, the first the one it's on.
    let gj = addon_lines(&a, "ES").into_iter().rfind(|l| l.starts_with("GJ\t")).expect("the journal line");
    let goals: Vec<&str> = gj[3..].split('^').collect();
    assert_eq!(goals.len(), htl_survival::Content::builtin().goals.len());
    assert_eq!(goals[0].split('~').nth(1), Some("1"), "{}", goals[0]);
    assert_eq!(goals[1].split('~').nth(1), Some("2"));
}

#[test]
fn an_archer_shows_its_bow_and_a_cutthroat_both_daggers() {
    use htl_server::fields::idx;
    let server = start_content(None, false, content());
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.chat(".es debug");
    a.chat(".es spawn archer");
    a.chat(".es spawn cutthroat");
    a.drain(1.0);
    let c = htl_survival::Content::builtin();
    let entry = |k: &str| 60_000 + c.mob(k).unwrap() as u32;
    let units = placed(&a);
    let of = |k: &str| units.iter().find(|(g, t, ..)| *t == ObjectType::Unit && ((g >> 24) & 0xFF_FFFF) as u32 == entry(k)).map(|u| u.3.clone()).expect(k);
    let archer = of("archer");
    assert_eq!(field(&archer, idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 2), Some(2826), "the bow");
    let cut = of("cutthroat");
    assert_eq!(field(&cut, idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY), Some(2699), "a dagger");
    assert_eq!(field(&cut, idx::UNIT_VIRTUAL_ITEM_SLOT_DISPLAY + 1), Some(2635), "and the other");
}

#[test]
fn the_book_takes_a_hero_down_to_the_deadmines_and_a_lamp_brings_them_home() {
    // The book's gate, set for this test (the built-in dungeons start open for now).
    let mut c = content();
    for d in c.dungeon.iter_mut().chain(c.more_dungeons.iter_mut()) {
        d.gate_raids = 1;
    }
    let server = start_content(None, false, c);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.drain(0.6);
    // The first goal done at camp (the timber banked): the journal's on its second.
    a.chat(".es bank");
    a.drain(1.0);
    let goal_no = |a: &Client| addon_lines(a, "ES").into_iter().rev().find(|l| l.starts_with("GL\t")).map(|l| l.split('\t').nth(1).unwrap_or("").to_string());
    assert_eq!(goal_no(&a).as_deref(), Some("2"), "on the second goal at camp");
    let d = htl_survival::Content::builtin().dungeon.unwrap();
    stand(&mut a, [d.book[0], d.book[1], 0.0], 0.0);
    // Shut until the camp holds a raid (debug opens it).
    a.chat(".es travel landing");
    assert!(
        a.wait(
            1.0,
            |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("first raid"))
        )
        .is_some(),
        "the book is shut at first"
    );
    a.chat(".es debug");
    a.chat(".es e");
    let bk = a.wait(
        1.0,
        |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBK\t")),
    );
    assert!(bk.is_some(), "the book's window");
    a.chat(".es travel landing");
    assert!(
        a.wait(2.0, |p| p.name() == "SMSG_TRANSFER_PENDING")
            .is_some(),
        "the far teleport starts"
    );
    assert!(
        a.wait(2.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(),
        "onto the Deadmines' map"
    );
    a.seen.clear();
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    a.drain(1.0);
    assert!(
        placed(&a).iter().any(|(_, t, ..)| *t == ObjectType::Player),
        "drawn in below"
    );
    assert!(
        addon_lines(&a, "ES").iter().any(|l| l == "WR\tbelow"),
        "the add-on knows it's below"
    );
    // Below, the same journal: the camp's goal, not the first.
    assert_eq!(goal_no(&a).as_deref(), Some("2"), "the camp's goal below too");
    // The lamp where they stand: lit, and its window.
    let lamp = d.lamps[0].at;
    stand(&mut a, lamp, 0.0);
    a.chat(".es e");
    assert!(
        a.wait(
            1.0,
            |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tLP\tlanding"))
        )
        .is_some(),
        "the lamp's window"
    );
    a.chat(".es rest");
    a.drain(0.5);
    a.seen.clear();
    a.chat(".es travel camp");
    assert!(
        // (A whole Elwynn to draw again: give it time when the machine is busy.)
        a.wait(8.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(),
        "home again: {:?}",
        addon_lines(&a, "ES").into_iter().filter(|l| l.starts_with("T\t")).collect::<Vec<_>>()
    );
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    a.drain(1.0);
    assert!(addon_lines(&a, "ES").iter().any(|l| l == "WR\tcamp"));
    assert!(objects_with(&a, 10).len() == 1, "the camp's chest in sight");
}

#[test]
fn a_hero_carries_four_satchels_beside_the_backpack() {
    use htl_server::fields::idx;
    let mut c = content();
    c.player.bag_slots = 80;
    // Two dozen stacks of timber: the backpack's 16 and more.
    c.start.items.insert("wood".into(), 20 * 24);
    let server = start_content(None, false, c);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let me = enter(&mut a);
    a.drain(1.0);
    let guid_at = |c: &Client, of: u64, i: u16| {
        u64::from(value_of(c, of, i).unwrap_or(0)) | (u64::from(value_of(c, of, i + 1).unwrap_or(0)) << 32)
    };
    let bags: Vec<u64> = (19..23u16).map(|s| guid_at(&a, me, idx::PLAYER_INV_SLOT_HEAD + 2 * s)).collect();
    assert!(bags.iter().all(|g| *g != 0), "a bag in each of the four slots: {bags:x?}");
    let objs = placed(&a);
    for g in &bags {
        let (_, t, _, f) = objs.iter().find(|(x, ..)| x == g).expect("the bag is made");
        assert_eq!(*t, ObjectType::Container);
        assert_eq!(field(f, idx::CONTAINER_NUM_SLOTS), Some(16));
    }
    // The backpack is full and the first satchel holds the rest; the second is empty.
    assert!((0..16u16).all(|i| guid_at(&a, me, idx::PLAYER_PACK_SLOT_1 + 2 * i) != 0), "the backpack is full");
    let first = guid_at(&a, bags[0], idx::CONTAINER_SLOT_1);
    assert_ne!(first, 0, "the first satchel holds a stack");
    assert_eq!(guid_at(&a, first, idx::ITEM_CONTAINED), bags[0], "and the stack knows its bag");
    assert_eq!(guid_at(&a, bags[1], idx::CONTAINER_SLOT_1), 0, "the second satchel is empty");
}

#[test]
fn gear_in_the_bags_is_put_on_with_a_right_click_or_a_drag() {
    use htl_server::fields::idx;
    let mut c = content();
    c.start.items.clear();
    c.start.items.insert("leather_cap".into(), 1);
    c.start.items.insert("hide_jerkin".into(), 1);
    let items = c.items.clone();
    let entry = |k: &str| 300_000 + items.iter().position(|d| d.key == k).unwrap() as u32;
    let server = start_content(None, false, c);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let me = enter(&mut a);
    a.drain(1.0);
    let guid_at = |c: &Client, of: u64, i: u16| {
        u64::from(value_of(c, of, i).unwrap_or(0)) | (u64::from(value_of(c, of, i + 1).unwrap_or(0)) << 32)
    };
    let worn = |c: &Client, slot: u16| {
        let g = guid_at(c, me, idx::PLAYER_INV_SLOT_HEAD + 2 * slot);
        value_of(c, g, 3)
    };
    let destroyed = |c: &Client, g: u64| c.seen.iter().filter(|p| matches!(p, ServerPacket::DestroyObject { guid } if *guid == g)).count();
    let carried = |c: &Client, k: &str| (0..16u16).find(|i| value_of(c, guid_at(c, me, idx::PLAYER_PACK_SLOT_1 + 2 * i), 3) == Some(entry(k))).expect(k);
    // The cap: right-clicked in the backpack (its slots from 23).
    let at = carried(&a, "leather_cap");
    let first = guid_at(&a, me, idx::PLAYER_PACK_SLOT_1 + 2 * at);
    let before = destroyed(&a, first);
    a.send(op::CMSG_AUTOEQUIP_ITEM, &[255, 23 + at as u8]);
    a.drain(1.0);
    assert_eq!(worn(&a, 0), Some(entry("leather_cap")), "the cap is worn");
    assert!(said(&a, "You put on the Leather Cap"));
    assert!(destroyed(&a, first) > before, "what the client held up is let go");
    // The jerkin: dragged from the backpack onto the chest slot (4).
    let at = carried(&a, "hide_jerkin");
    a.send(op::CMSG_SWAP_INV_ITEM, &[23 + at as u8, 4]);
    a.drain(1.0);
    assert_eq!(worn(&a, 4), Some(entry("hide_jerkin")), "the jerkin is worn");
    // Nothing is left in the backpack: neither replaced a carried piece.
    assert!((0..16u16).all(|i| guid_at(&a, me, idx::PLAYER_PACK_SLOT_1 + 2 * i) == 0), "the backpack is empty");
}

#[test]
fn an_eighth_round_client_sees_a_sky_fall_laid_at_its_spot_and_an_older_one_its_circle() {
    let server = start_content(None, false, content());
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac8");
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let _ = enter(&mut b);
    b.chat(".es hello 5 ac5");
    a.drain(0.6);
    b.drain(0.3);
    // Mind Blast (a dark star at the reticle 0.6 s later): Penance's EX version, on Q with Ctrl.
    a.chat(".es slot q penance");
    a.chat(".es valor full");
    a.drain(0.2);
    a.seen.clear();
    b.seen.clear();
    a.chat(".es in ab 0 1");
    a.chat(".es in a");
    a.chat(".es in au 0.0 -0.3");
    let ta = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tTA\t")));
    match ta {
        Some(ServerPacket::MessageChat(m)) => {
            let f: Vec<&str> = m.text.split('\t').collect();
            assert_eq!(f[7], "ring", "{}", m.text);
            assert_eq!(f[11], "friendly", "its own: a friend's colour");
        }
        _ => panic!("the ring laid at its spot"),
    }
    assert!(b.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tMK\t"))).is_some(), "the older client: its circle");
}

#[test]
fn the_world_map_shows_elwynn_explored_and_its_places_marked() {
    let server = start_content(None, false, content());
    let mut c = Client::connect(server.login, "alice", "hold", REALM).unwrap();
    let me = enter(&mut c);
    c.drain(0.5);
    // Every place's art on the map: all the explored-zone bits set on the hero's own create.
    let all = placed(&c);
    let (_, _, _, mine) = all.iter().find(|(g, ..)| *g == me).cloned().expect("our own create");
    for slot in 0..64u16 {
        assert_eq!(field(&mine, 1111 + slot), Some(u32::MAX), "explored-zone word {slot}");
    }
    // The add-on hears of every place: where, what kind, whether the camp has found it.
    c.chat(".es hello 5 ac8");
    c.drain(1.0);
    let pm = addon_lines(&c, "ES").into_iter().find(|l| l.starts_with("PM\t")).expect("the map's marks");
    let f: Vec<&str> = pm.split('\t').collect();
    let tally: Vec<&str> = f[1].split('~').collect();
    let sites = htl_survival::Content::builtin().sites.len();
    assert_eq!(tally[1], sites.to_string(), "{pm}");
    let marks: Vec<&str> = f[2].split('^').collect();
    assert_eq!(marks.len(), sites);
    let camp: Vec<&str> = marks.iter().find(|m| m.starts_with("camp~")).unwrap().split('~').collect();
    assert_eq!(camp[3], "camp");
    // The hero stands in the camp: it's found at once.
    assert_eq!(camp[4], "1", "{pm}");
    let hogger: Vec<&str> = marks.iter().find(|m| m.starts_with("forests_edge~")).unwrap().split('~').collect();
    assert_eq!(hogger[4], "0", "not yet found");
}

#[test]
fn characters_are_made_on_the_creation_screen_listed_played_and_deleted() {
    let mut rules = content();
    rules.dev.full_tree = false;
    let server = start_content(None, false, rules);
    let mut c = Client::connect(server.login, "cara", "hold", REALM).unwrap();
    assert!(list(&mut c).is_empty(), "a new account has no one yet");
    // Names: letters, two to twelve, no one else's.
    assert_eq!(create(&mut c, "A", 1, 8, 1, [0; 5]), 0x46, "too short");
    assert_eq!(create(&mut c, "Thirteenchars", 1, 8, 1, [0; 5]), 0x47, "too long");
    assert_eq!(create(&mut c, "Mor9ana", 1, 8, 1, [0; 5]), 0x48, "letters only");
    assert_eq!(create(&mut c, "Morgana", 1, 6, 1, [0; 5]), 0x30, "no class 6 (a death knight)");
    assert_eq!(create(&mut c, "morgana", 1, 8, 1, [3, 4, 5, 6, 0]), 0x2E, "a human woman, a mage");
    assert_eq!(create(&mut c, "MORGANA", 3, 1, 0, [0; 5]), 0x31, "the name's taken");
    assert_eq!(create(&mut c, "Thrall", 2, 7, 0, [1, 2, 3, 4, 5]), 0x2E, "an orc man, a shaman");
    let chars = list(&mut c);
    let names: Vec<&str> = chars.iter().map(|h| h.name.as_str()).collect();
    assert_eq!(names, ["Morgana", "Thrall"], "both listed, the name as a name");
    let m = &chars[0];
    assert_eq!((m.race, m.class, m.gender, m.skin, m.face, m.hair_style, m.hair_color, m.facial_hair), (1, 8, 1, 3, 4, 5, 6, 0));
    let t = &chars[1];
    assert_eq!((t.race, t.class, t.gender, t.facial_hair), (2, 7, 0, 5));
    assert_ne!(m.guid, t.guid);

    // In as Morgana: drawn as she was made, and she knows the mage's Q and E (no more yet).
    let me = c.enter_with(|chars| chars.iter().position(|h| h.name == "Morgana").unwrap());
    assert_eq!(me, m.guid);
    c.drain(0.5);
    let (_, _, _, f) = placed(&c).into_iter().find(|(g, ..)| *g == me).expect("our own create");
    let bytes0 = field(&f, 36).unwrap();
    assert_eq!((bytes0 & 0xFF, bytes0 >> 8 & 0xFF, bytes0 >> 16 & 0xFF), (1, 8, 1), "race, class, sex");
    assert_eq!(field(&f, 193), Some(u32::from_le_bytes([3, 4, 5, 6])), "skin, face, hair, its colour");
    assert_eq!(field(&f, 131), Some(50), "a human woman's model");
    c.chat(".es tree");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(mc) if mc.text.contains("Your cards"))).is_some());
    let cards = c.seen.iter().rev().find_map(|p| if let ServerPacket::MessageChat(mc) = p { mc.text.contains("Your cards").then(|| mc.text.clone()) } else { None }).unwrap();
    let rules = htl_survival::Content::builtin();
    for a in rules.abilities.iter().filter(|a| a.on_board()) {
        let k = format!("{},", a.key);
        let knows = cards.contains(&k) || cards.contains(&format!("{}.", a.key));
        assert_eq!(knows, a.school == "mage" && (a.slot == "ranged" || a.slot == "close"), "{} ({} {}) in: {cards}", a.key, a.school, a.slot);
    }
    // Every spell at once: only with debug on.
    c.chat(".es spells");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(mc) if mc.text.contains("Turn debug on first"))).is_some(), "refused without debug");
    c.chat(".es debug");
    c.chat(".es spells");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(mc) if mc.text.contains("Every spell is yours"))).is_some());
    c.seen.clear();
    c.chat(".es tree");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(mc) if mc.text.contains("Your cards"))).is_some());
    let cards = c.seen.iter().rev().find_map(|p| if let ServerPacket::MessageChat(mc) = p { mc.text.contains("Your cards").then(|| mc.text.clone()) } else { None }).unwrap();
    for a in rules.abilities.iter().filter(|a| a.on_board() && !a.is_ex() && !a.inner) {
        assert!(cards.contains(&format!("{},", a.key)) || cards.contains(&format!("{}.", a.key)), "{} in: {cards}", a.key);
    }
    c.chat(".es debug");
    // Out, and Thrall deleted; Morgana stays.
    c.send(op::CMSG_LOGOUT_REQUEST, &[]);
    assert!(c.wait(2.0, |p| p.name() == "SMSG_LOGOUT_COMPLETE").is_some());
    let mut other = Client::connect(server.login, "dan", "hold", REALM).unwrap();
    other.send(op::CMSG_CHAR_DELETE, &m.guid.to_le_bytes());
    assert!(matches!(other.wait(2.0, |p| matches!(p, ServerPacket::CharDelete { .. })), Some(ServerPacket::CharDelete { result: 0x3A })), "not dan's to delete");
    c.send(op::CMSG_CHAR_DELETE, &t.guid.to_le_bytes());
    assert!(matches!(c.wait(2.0, |p| matches!(p, ServerPacket::CharDelete { .. })), Some(ServerPacket::CharDelete { result: 0x39 })), "Thrall deleted");
    let names: Vec<String> = list(&mut c).into_iter().map(|h| h.name).collect();
    assert_eq!(names, ["Morgana"]);
    assert!(list(&mut other).is_empty(), "dan's list is his own");
}

#[test]
fn a_hero_and_the_camp_start_over_and_the_old_save_is_kept() {
    let dir = std::env::temp_dir().join(format!("es-reset-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let save = dir.join("world.toml");
    let server = start_with(Some(save.clone()), false);
    let mut c = Client::connect(server.login, "rhea", "hold", REALM).unwrap();
    enter(&mut c);
    // Something to start over from: a fire built at camp.
    stand(&mut c, [-9538.0, 122.0, 0.0], 0.0);
    c.drain(0.2);
    c.chat(".build cooking");
    c.drain(0.2);
    c.chat(".place");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("builds a Cooking Fire"))).is_some(), "built");
    c.drain(1.5);
    assert!(save.exists(), "saved");
    c.chat(".es reset camp");
    assert!(c.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("Turn debug on first"))).is_some(), "debug only");
    c.chat(".es debug");
    c.chat(".es reset me");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You start over"))).is_some(), "the hero starts over");
    c.chat(".es reset camp");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("The camp starts over"))).is_some(), "and the camp");
    assert!(said(&c, "The old world is kept beside the save"));
    let kept: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with("world.toml.before-reset-")).collect();
    assert_eq!(kept.len(), 1, "the old save kept aside");
    let old = std::fs::read_to_string(kept[0].path()).unwrap();
    let new = std::fs::read_to_string(&save).unwrap();
    assert!(old.contains("cooking"), "the old world had its fire");
    assert!(!new.contains("\"cooking\"") && !new.contains("= \"cooking\""), "the new one hasn't");
    drop(server);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_books_siege_page_takes_heroes_to_arathi_a_match_begins_and_home_their_own_hero_is_back() {
    let server = start_content(None, false, content());
    let d = htl_survival::Content::builtin().dungeon.unwrap();
    let sg = htl_survival::Content::builtin().siege.unwrap();
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    let mut b = Client::connect(server.login, "bob", "hold", REALM).unwrap();
    let _ = enter(&mut b);
    for c in [&mut a, &mut b] {
        c.chat(".es hello 5 ac4");
        c.drain(0.4);
        // Something of theirs at camp, to find again.
        c.chat(".es bank");
        stand(c, [d.book[0], d.book[1], 0.0], 0.0);
    }
    a.chat(".es e");
    let bk = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBK\t") && m.text.contains("siege~")));
    assert!(bk.is_some(), "the book has Siege's page from the start");
    for c in [&mut a, &mut b] {
        c.seen.clear();
        c.chat(".es travel siege");
        assert!(c.wait(3.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(), "onto Arathi Basin's map");
        c.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
        assert!(c.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l == "WR\tsiege")), "the add-on knows it's Siege");
    }
    // The muster banner in the hall, and a way to play.
    let banner_at = {
        let f = sg.lobby_facing;
        [sg.lobby[0] + f.cos() * 5.0, sg.lobby[1] + f.sin() * 5.0, 0.0]
    };
    stand(&mut a, [banner_at[0] - 1.0, banner_at[1], 0.0], 0.0);
    a.seen.clear();
    a.chat(".es e");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tSL\t1\t"))).is_some(), "the muster's window");
    // Ann first: the first in the queue holds the first base (the Stables), and two chats sent
    // at once can reach the server in either order (#2).
    a.chat(".siege join 1v1");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You wait for"))).is_some(), "ann waits for a match");
    b.chat(".siege join 1v1");
    b.seen.clear();
    b.drain(sg.countdown);
    b.until(1.5, |c| {
        addon_lines(c, "ES").iter().rev().find(|l| l.starts_with("SG\trunning\t")).is_some_and(|l| l.contains("the Stables") && l.contains("the Farm"))
    });
    let lines = addon_lines(&b, "ES");
    let sgl = lines.iter().rev().find(|l| l.starts_with("SG\trunning\t"));
    assert!(sgl.is_some_and(|l| l.contains("the Stables") && l.contains("the Farm")), "the match in the corner: {:?}", lines.iter().filter(|l| l.starts_with("SG")).collect::<Vec<_>>());
    // A send from the ledger.
    b.chat(".siege send kobolds");
    let sent = b.wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("march on the Stables")));
    assert!(sent.is_some() || said(&b, "march on the Stables"), "sent at Ann's camp");
    // The Battle Trainer behind her Hearth: her class's other cards, bought with gold.
    let st = &sg.bases[0];
    let back = st.facing + std::f32::consts::PI;
    let trainer = [st.at[0] + back.cos() * 6.0, st.at[1] + back.sin() * 6.0, 0.0];
    stand(&mut a, [trainer[0] + back.cos() * 1.5, trainer[1] + back.sin() * 1.5, 0.0], st.facing);
    a.seen.clear();
    a.chat(".es e");
    let w = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tW\t") && m.text.contains("Battle Trainer")));
    assert!(w.is_some(), "the trainer's window: {:?}", addon_lines(&a, "ES").iter().filter(|l| l.starts_with("NP")).collect::<Vec<_>>());
    // (A dev world's hero has every card already: here the trainer has nothing left to teach.)
    // Home: Ann's own hero again (her timber still banked at camp, her cards hers).
    a.seen.clear();
    a.chat(".es travel camp");
    assert!(a.wait(8.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(), "home again");
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    assert!(a.until(1.0, |c| addon_lines(c, "ES").iter().any(|l| l == "WR\tcamp")), "the add-on knows it's home");
    let tr = {
        a.seen.clear();
        a.chat(".es tree");
        a.until(0.5, |c| addon_lines(c, "ES").iter().any(|l| l.starts_with("TR\t")));
        addon_lines(&a, "ES").into_iter().find(|l| l.starts_with("TR\t"))
    };
    let tr = tr.unwrap_or_default();
    assert!(tr.matches("~1~").count() > 4, "every card of her dev-world hero, not Siege's two");
    // Each card ends with its page: 1 a class's, 2 the beta page's (its column `beta`).
    let cards: Vec<&str> = tr.splitn(4, '\t').nth(3).unwrap_or("").split('^').collect();
    let beta = |c: &&str| c.split('~').nth(3) == Some("beta");
    assert!(cards.iter().any(beta), "the beta page's cards are sent: {tr}");
    assert!(cards.iter().filter(|c| beta(*c)).all(|c| c.ends_with("~2")), "on page 2: {tr}");
    assert!(cards.iter().filter(|c| !beta(*c)).all(|c| c.ends_with("~1")), "the classes' on page 1: {tr}");
}

#[test]
fn the_book_lists_every_dungeon_and_a_page_of_another_takes_a_hero_to_its_own_world() {
    let server = start_below(None, false, content(), &["deadmines", "stockade"]);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.drain(0.6);
    let c = htl_survival::Content::builtin();
    let book = c.dungeon.as_ref().unwrap().book;
    stand(&mut a, [book[0], book[1], 0.0], 0.0);
    a.chat(".es debug");
    a.chat(".es e");
    let bk = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tBK\t")));
    let ServerPacket::MessageChat(m) = bk.expect("the book's window") else { unreachable!() };
    let dungeons = m.text.split('\t').nth(6).unwrap_or("").to_string();
    for d in c.dungeons() {
        assert!(dungeons.contains(&format!("{}~{}~", d.key, d.name)), "{} in the book: {dungeons}", d.name);
    }
    assert!(m.text.contains("stockade_gate~The Stockade's Gate~stockade"), "its first page, with its dungeon");
    // Each dungeon's record of secrets and notes: its tallies in the book, its page on asking.
    let dm = c.dungeons().find(|d| d.key == "deadmines").unwrap();
    let (secrets, notes) = (dm.doors.iter().filter(|d| d.secret()).count(), dm.notes.len());
    let entry = dungeons.split('^').find(|e| e.starts_with("deadmines~")).unwrap_or("");
    assert!(entry.contains(&format!("~0~{secrets}~0~{notes}~1~")), "none found yet, and wagers open to +1: {entry}");
    a.chat(".es lore deadmines");
    let nt = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tNT\tlore\t")));
    let ServerPacket::MessageChat(m) = nt.expect("the record's page") else { unreachable!() };
    // Notes only: the secrets aren't listed or counted on the page.
    assert!(m.text.contains(&format!("\t0 of {notes} notes read\t")) && !m.text.contains("Secrets found") && !m.text.contains("secrets found"), "{}", m.text);
    a.chat(".es note smugglers_tally");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You haven't read that one yet"))).is_some(), "not read: not in the book");
    a.chat(".es travel stockade_gate");
    assert!(a.wait(2.0, |p| p.name() == "SMSG_TRANSFER_PENDING").is_some(), "the far teleport starts");
    assert!(a.wait(2.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(), "onto the Stockade's map");
    a.seen.clear();
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    a.drain(1.0);
    assert!(placed(&a).iter().any(|(_, t, ..)| *t == ObjectType::Player), "drawn in below");
    assert!(said(&a, "The Stockade"), "its own welcome");
    // Its own lamp, lit, takes them home.
    let gate = c.dungeons().find(|d| d.key == "stockade").unwrap().lamps[0].at;
    stand(&mut a, gate, 0.0);
    a.chat(".es e");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tLP\tstockade_gate"))).is_some(), "the Stockade's own lamp");
    a.seen.clear();
    a.chat(".es travel camp");
    assert!(a.wait(8.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(), "home again");
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    a.drain(1.0);
    // A page of a dungeon this server couldn't read: said so.
    stand(&mut a, [book[0], book[1], 0.0], 0.0);
    a.seen.clear();
    a.chat(".es travel ragefire_top");
    assert!(a.wait(1.5, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("couldn't be read"))).is_some(), "a dungeon with no ground says so");
}

#[test]
fn the_bookmaker_takes_a_wager_from_the_book_and_keeps_a_board_for_every_server() {
    use std::io::{Read, Write};
    let server = start_below(None, false, content(), &["deadmines", "stockade"]);
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    a.chat(".es hello 5 ac4");
    a.drain(0.6);
    let c = htl_survival::Content::builtin();
    let book = c.dungeon.as_ref().unwrap().book;
    stand(&mut a, [book[0], book[1], 0.0], 0.0);
    a.chat(".es debug");
    // Its page: open to +1, the time to beat, the levels to take, the board.
    a.chat(".es wager stockade");
    let nt = a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tNT\twager\t")));
    let ServerPacket::MessageChat(m) = nt.expect("the wager's page") else { unreachable!() };
    assert!(m.text.contains("Open to +1,") && m.text.contains(".es wager stockade 1~Wager +1") && m.text.contains(".es board stockade~"), "{}", m.text);
    a.chat(".es wager stockade 2");
    assert!(a.wait(1.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("up to +1"))).is_some(), "not won yet");
    // Taken: down, fresh, and the clock waiting at the lamp.
    a.chat(".es wager stockade 1");
    assert!(a.wait(2.0, |p| p.name() == "SMSG_NEW_WORLD").is_some(), "down to the Stockade");
    a.seen.clear();
    a.send(op::MSG_MOVE_WORLDPORT_ACK, &[]);
    let wt = a.wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tWT\tThe Stockade\t1\t")));
    let ServerPacket::MessageChat(m) = wt.expect("the Bookmaker's clock") else { unreachable!() };
    assert!(m.text.contains("\twait\t"), "{}", m.text);
    // Its count of foes last: none felled yet, of those it needs and those standing.
    let fields: Vec<&str> = m.text.split('\t').collect();
    let forces: Vec<u32> = fields.get(10).unwrap_or(&"").split('~').filter_map(|n| n.parse().ok()).collect();
    assert!(forces.len() == 3 && forces[0] == 0 && forces[1] > 0 && forces[1] < forces[2], "{}", m.text);
    // And who's ready: nobody yet, of one hero, no count.
    assert_eq!(fields.get(11).copied(), Some("0~1~"), "{}", m.text);
    // Ready: alone, the count starts at once.
    a.chat(".es ready");
    assert!(a.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tWT\t") && m.text.contains(&format!("\t1~1~{}\t", htl_survival::sim::wager::WAGER_COUNTDOWN as u32)))).is_some(), "the count on the slip");
    // Another server's run, sent to this one (the board host) on its login port, comes back on
    // the board; a forged one doesn't.
    let w = htl_survival::sim::wager::WagerRun { dungeon: "stockade".into(), name: "The Stockade".into(), level: 6, ms: 1_000_000, limit_ms: 1_800_000, deaths: 1, timed: true, party: vec![("Zed".into(), "mage".into())], splits: Vec::new() };
    let line = htl_server::survival::board::Run::from_wager(&w, "Far Camp").line();
    let forged = line.replace("|6|", "|9|");
    let body = format!("{line}\n{forged}\n");
    let mut s = std::net::TcpStream::connect(server.login).unwrap();
    write!(s, "POST /wagers HTTP/1.0\r\nContent-Length: {}\r\n\r\n{body}", body.len()).unwrap();
    let mut reply = String::new();
    s.read_to_string(&mut reply).unwrap();
    assert!(reply.starts_with("HTTP/1.0 200") && reply.contains("|Far Camp|stockade|6|") && !reply.contains("|9|"), "{reply}");
    // The ladder: Zed, from that run, rated across the dungeons.
    a.chat(".es ladder");
    let nt = a.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("ES\tNT\tboard\tThe Bookmaker's Ladder\t")));
    let ServerPacket::MessageChat(m) = nt.expect("the ladder's page") else { unreachable!() };
    assert!(m.text.contains("1. Zed (Mage) of Far Camp: ") && m.text.contains("Win a wager in time to get on it"), "{}", m.text);
    // And the login port still logs in.
    assert!(Client::connect(server.login, "bob", "hold", REALM).is_ok());
}

#[test]
fn a_client_running_its_own_dodge_is_told_its_run_again_after_a_sprinting_roll() {
    let server = start_with(None, false);
    let mut c = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut c);
    // A client that runs its own dodge (ac3): the dash's burst isn't sent to it.
    c.chat(".es hello 5 ac3");
    c.drain(0.5);
    let here = [-9500.0, 0.0, 0.0];
    moving(&mut c, here, 0.0, 0x1);
    let run = |p: &ServerPacket, lo: f32, hi: f32| matches!(p, ServerPacket::ForceSpeedChange { kind: messages::SpeedKind::Run, speed, .. } if *speed > lo && *speed < hi);
    c.chat(".es in sp");
    assert!(c.wait(1.0, |p| run(p, 10.0, 14.0)).is_some(), "sprinting");
    // A roll mid-sprint, Shift let go during it: once the burst is spent, the client is told it
    // runs again. (It wasn't, and kept sprinting while the server charged it nothing.)
    let mark = c.seen.len();
    c.chat(".es in d");
    c.chat(".es in spu");
    let told = c.wait(1.5, |p| run(p, 5.0, 9.0));
    let speeds: Vec<String> = c.seen[mark..].iter().filter_map(|p| if let ServerPacket::ForceSpeedChange { speed, .. } = p { Some(format!("{speed}")) } else { None }).collect();
    assert!(told.is_some(), "told it runs again: {speeds:?}");
    assert!(!c.seen[mark..].iter().any(|p| run(p, 14.0, 100.0)), "the burst itself is the client's: {speeds:?}");
}

#[test]
fn a_corey_roll_out_of_the_clients_own_jump_strikes_as_it_lands() {
    let server = start_with(None, false);
    let mut c = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut c);
    c.chat(".es hello 5 ac3");
    c.drain(0.5);
    let here = [-9500.0, 0.0, 0.0];
    moving(&mut c, here, 0.0, 0x1);
    // A roll forward, and the jump taken in its dash (the client's own: its JUMP and its landing).
    c.chat(".es in d");
    std::thread::sleep(std::time::Duration::from_millis(150));
    let mut jump = Vec::new();
    jump.extend_from_slice(&0x2001u32.to_le_bytes());
    jump.extend_from_slice(&0u32.to_le_bytes());
    for v in here {
        jump.extend_from_slice(&v.to_le_bytes());
    }
    jump.extend_from_slice(&0f32.to_le_bytes());
    jump.extend_from_slice(&0u32.to_le_bytes());
    c.send(op::MSG_MOVE_JUMP, &jump);
    c.drain(0.5);
    // An attack in the air: nothing yet.
    c.chat(".es in a");
    c.chat(".es in au");
    c.drain(0.3);
    let swung = |c: &Client| addon_lines(c, "ES").into_iter().filter(|l| l.starts_with("AN\t") && l.split('\t').nth(10) != Some("mob")).collect::<Vec<_>>();
    assert!(swung(&c).is_empty(), "nothing swings in the air: {:?}", swung(&c));
    // Down: the leap strikes, heavy.
    let mut land = Vec::new();
    land.extend_from_slice(&0x1u32.to_le_bytes());
    land.extend_from_slice(&0u32.to_le_bytes());
    for v in [here[0] + 9.0, here[1], here[2]] {
        land.extend_from_slice(&v.to_le_bytes());
    }
    land.extend_from_slice(&0f32.to_le_bytes());
    land.extend_from_slice(&700u32.to_le_bytes());
    c.send(op::MSG_MOVE_FALL_LAND, &land);
    c.drain(0.4);
    let lines = swung(&c);
    let leap = lines.iter().find(|l| l.split('\t').nth(10) == Some("leap")).unwrap_or_else(|| panic!("the leap on landing: {lines:?}"));
    assert_eq!(leap.split('\t').nth(4), Some("78FF9A3C"), "with a heavy blow's trail: {leap}");
}

#[test]
fn a_creatures_swing_trails_its_last_stroke_on_a_client_that_draws_it_so() {
    let server = start_content(None, false, content());
    let mut a = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut a);
    let mut b = Client::connect(server.login, "ben", "hold", REALM).unwrap();
    let _ = enter(&mut b);
    // Ann's client starts a creature's trail late (ac9); Ben's is the round before.
    a.chat(".es hello 5 ac9");
    b.chat(".es hello 5 ac8");
    a.drain(0.6);
    b.drain(0.2);
    a.chat(".es debug");
    a.chat(".es spawn grunt");
    let mob_swing = |l: &String| l.starts_with("AN\t") && l.split('\t').nth(10) == Some("mob");
    let mut seen = None;
    for _ in 0..20 {
        a.drain(0.5);
        if let Some(l) = addon_lines(&a, "ES").into_iter().find(|l| mob_swing(l)) {
            seen = Some(l);
            break;
        }
    }
    let line = seen.expect("the grunt winds up a blow at ann");
    let f: Vec<&str> = line.split('\t').collect();
    // Amber for a blow a guard stops, red for one it doesn't, started 320 ms before it lands.
    assert!(f[4] == "70FFC24A" || f[4] == "A0FF2A1A", "{line}");
    assert_eq!(f[5], "320", "{line}");
    b.drain(0.5);
    let theirs: Vec<String> = addon_lines(&b, "ES").into_iter().filter(|l| mob_swing(l)).collect();
    assert!(!theirs.is_empty(), "ben sees it wind up too");
    assert!(theirs.iter().all(|l| l.split('\t').nth(4) == Some("-")), "with no trail on the older client: {theirs:?}");
}

#[test]
fn a_blow_borrowing_another_models_clip_names_it_to_the_clients() {
    let mut content = content();
    for m in &mut content.movesets {
        for l in &mut m.light {
            l.from = Some("ScourgeFemale".into());
        }
    }
    let server = start_content(None, false, content);
    let mut c = Client::connect(server.login, "ann", "hold", REALM).unwrap();
    let _ = enter(&mut c);
    c.chat(".es hello 5 ac10");
    c.drain(0.5);
    stand(&mut c, [-9500.0, 0.0, 0.0], 0.0);
    c.chat(".es in a");
    c.chat(".es in au");
    c.drain(0.6);
    let lines = addon_lines(&c, "ES");
    let swing = lines.iter().find(|l| l.starts_with("AN\t") && l.split('\t').nth(10) == Some("light")).unwrap_or_else(|| panic!("the swing: {lines:?}"));
    assert_eq!(swing.split('\t').nth(12), Some("ScourgeFemale"), "{swing}");
    // And the moves the add-on plays itself carry it too.
    let mv = lines.iter().rfind(|l| l.starts_with("MV\t")).expect("the moves");
    let first = mv.split('\t').nth(10).unwrap().split('^').next().unwrap();
    assert!(first.ends_with("~ScourgeFemale"), "{first}");
}

#[test]
fn an_older_add_on_or_client_hears_how_to_update_and_a_current_one_hears_nothing() {
    let server = start_with(None, false);
    let mut c = Client::connect(server.login, "vera", "hold", REALM).unwrap();
    enter(&mut c);
    let told = |c: &Client| addon_lines(c, "ES").iter().any(|l| l.starts_with("T\t") && l.contains("older than this server's"));
    // An old add-on on a plain client: told, on screen.
    c.chat(".es hello 1");
    assert!(c.until(2.0, told), "told: {:?}", addon_lines(&c, "ES"));
    // The newest add-on on the newest client: not a word.
    let mut d = Client::connect(server.login, "wren", "hold", REALM).unwrap();
    enter(&mut d);
    d.chat(".es hello 7 ac10");
    assert!(d.until(2.0, |d| addon_lines(d, "ES").iter().any(|l| l.starts_with("V\t"))), "awake");
    d.drain(0.5);
    assert!(!told(&d), "nothing to update");
}
