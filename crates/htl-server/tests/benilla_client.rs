//! The server against benilla's own client code: its logon, its world handshake, and its packet
//! parser and event decoder over everything the server sends. A packet benilla can't read, or
//! reads with bytes left over, fails the test.

use std::time::{Duration, Instant};

use benilla_protocol::messages::{self, opcode as op, ServerPacket};
use htl_server::Config;
use htl_sim::{Arena, BaseSite, Content, Flat, Lane};

fn test_arena() -> Arena {
    let lane = |name: &str, x: f32, y: f32| Lane::new(name, vec![[x, y, 0.0], [x * 0.5, y * 0.5, 0.0], [x * 0.06, y * 0.06, 0.0]]);
    Arena {
        map_id: 529,
        area_id: 3358,
        hub: [0.0, 0.0, 0.0],
        hub_facing: 0.0,
        player_spawn: [8.0, 0.0, 0.0],
        posts: Vec::new(),
        map_rect: None,
        player_facing: 3.14,
        lanes: vec![lane("the north", 60.0, 0.0), lane("the south", -60.0, 0.0)],
        bases: vec![
            BaseSite { name: "the Farm".into(), pos: [100.0, 0.0, 0.0], lane: 0 },
            BaseSite { name: "the Stables".into(), pos: [-100.0, 0.0, 0.0], lane: 1 },
        ],
    }
}

fn start() -> htl_server::Running {
    let config = Config {
        bind: "127.0.0.1".into(),
        login_port: 0,
        world_port: 0,
        ..Config::default()
    };
    let mut content = Content::builtin();
    content.game.countdown_secs = 600.0;
    // The protocol, not the balance: a pair fights a wave as one hero would.
    content.scaling.party_health = 0.0;
    content.scaling.party_damage = 0.0;
    content.scaling.party_spawn = 0.0;
    // Soft enough that Alice's sword finishes one in time even on a busy machine.
    content.scaling.health_base *= 0.5;
    htl_server::start(config, content, test_arena(), Box::new(Flat(0.0))).expect("server starts")
}

mod common;
use common::{creates, Client};

/// Into the world as the class at `class_index` (one character per class, all in the Basin).
fn enter(c: &mut Client, class_index: usize) -> u64 {
    c.enter_with(|chars| {
        assert_eq!(chars.len(), 4, "one character per class");
        assert!(chars.iter().all(|c| c.level == 1 && c.map == 529));
        class_index
    })
}

#[test]
fn wrong_password_is_refused() {
    let server = start();
    assert!(Client::connect(server.login, "mallory", "nope", "Basin Battle").is_err());
}

#[test]
fn two_players_enter_see_each_other_and_fight_a_wave() {
    let server = start();
    let mut alice = Client::connect(server.login, "alice", "hold", "Basin Battle").expect("alice logs in");
    let a_guid = enter(&mut alice, 0); // warrior
    // The hub is in the world.
    assert!(
        alice.seen.iter().any(|p| creates(p).iter().any(|(_, t)| *t == messages::ObjectType::Unit))
            || alice
                .wait(2.0, |p| creates(p).iter().any(|(_, t)| *t == messages::ObjectType::Unit))
                .is_some(),
        "alice should see the hub"
    );

    // Her starting kit arrives as item objects, and the starting pick as a dialog.
    assert!(
        alice.seen.iter().any(|p| creates(p).iter().any(|(_, t)| *t == messages::ObjectType::Item))
            || alice.wait(2.0, |p| creates(p).iter().any(|(_, t)| *t == messages::ObjectType::Item)).is_some(),
        "alice's items"
    );
    let menu = alice.wait(3.0, |p| matches!(p, ServerPacket::GossipMessage { .. }));
    let Some(ServerPacket::GossipMessage { npc: npc_guid, text_id, options, .. }) = menu else { panic!("no starting pick") };
    assert_eq!(npc_guid, a_guid, "the pick is the player's own dialog");
    assert!(options.len() >= 3);
    alice.send(op::CMSG_NPC_TEXT_QUERY, &messages::npc_text_query(text_id, npc_guid));
    assert!(alice.wait(2.0, |p| matches!(p, ServerPacket::NpcText { .. })).is_some(), "the dialog's text");
    alice.send(op::CMSG_GOSSIP_SELECT_OPTION, &messages::gossip_select_option(npc_guid, 0, None));
    assert!(alice.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You learn"))).is_some(), "the card is taken");
    // Item tooltips: the starting weapon (warrior: class 0, tier 0, common, the weapon piece).
    let sword = htl_sim::items::starter_weapon(0);
    // Templates come fresh without asking, so a client's cache from an older build can't show
    // stale gear or names.
    assert!(
        alice.seen.iter().any(|p| matches!(p, ServerPacket::ItemQueryResponse { .. })),
        "item templates are pushed"
    );
    assert!(
        alice.seen.iter().any(|p| matches!(p, ServerPacket::CreatureQueryResponse { .. })),
        "creature templates are pushed"
    );
    alice.send(op::CMSG_ITEM_QUERY_SINGLE, &messages::item_query(sword, 0));
    assert!(alice.wait(2.0, |p| matches!(p, ServerPacket::ItemQueryResponse { .. })).is_some(), "item template");

    let mut bob = Client::connect(server.login, "bob", "hold", "Basin Battle").expect("bob logs in");
    let b_guid = enter(&mut bob, 2); // mage
    // Bob casts by hand (and so not at all here): Alice's kill is her own.
    bob.chat(".bb auto");
    // Bob chooses his first spell too: the world waits for everyone's pick.
    let menu = bob.wait(3.0, |p| matches!(p, ServerPacket::GossipMessage { .. }));
    let Some(ServerPacket::GossipMessage { npc: bob_npc, .. }) = menu else { panic!("no starting pick for bob") };
    bob.send(op::CMSG_GOSSIP_SELECT_OPTION, &messages::gossip_select_option(bob_npc, 0, None));
    assert!(bob.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("You learn"))).is_some(), "bob's card is taken");
    // Each sees the other.
    assert!(
        bob.seen.iter().any(|p| creates(p).iter().any(|(g, _)| *g == a_guid))
            || bob.wait(2.0, |p| creates(p).iter().any(|(g, _)| *g == a_guid)).is_some(),
        "bob sees alice"
    );
    assert!(alice.wait(2.0, |p| creates(p).iter().any(|(g, _)| *g == b_guid)).is_some(), "alice sees bob");
    // They are a party.
    assert!(alice
        .wait(2.0, |p| matches!(p, ServerPacket::GroupList { members, .. } if members.iter().any(|m| m.guid == b_guid)))
        .is_some());

    // Alice's moves reach Bob.
    alice.move_to([10.0, 2.0, 0.0]);
    assert!(bob
        .wait(2.0, |p| matches!(p, ServerPacket::MonsterMove { .. }) || p.name().starts_with("MSG_MOVE"))
        .is_some(), "bob should see alice move");

    // Chat, and starting the wave.
    alice.chat("hello bob");
    assert!(bob.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "hello bob")).is_some());
    // (Whispered, as a fallen player's add-on sends it.)
    alice.whisper("Alice", ".start");
    assert!(bob
        .wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("Wave 1:")))
        .is_some(), "the wave announcement");

    // Enemies arrive and walk.
    let enemy = match alice.wait(5.0, |p| creates(p).iter().any(|(g, t)| *t == messages::ObjectType::Unit && (g >> 24) & 0xFF_FFFF == 2000)) {
        Some(p) => creates(&p).into_iter().find(|(g, _)| (g >> 24) & 0xFF_FFFF == 2000).unwrap().0,
        None => panic!("no enemy spawned"),
    };
    assert!(alice
        .wait(2.0, |p| matches!(p, ServerPacket::MonsterMove { guid, .. } if *guid == enemy))
        .is_some(), "enemies walk");

    // Alice runs up to it and fights with her sword alone (her spell casts by hand, so not
    // here). Track where it is from its moves.
    alice.chat(".bb auto");
    let mut killed = false;
    let end = Instant::now() + Duration::from_secs(60);
    alice.send(op::CMSG_ATTACKSWING, &enemy.to_le_bytes());
    let mut last_pos = None;
    while Instant::now() < end {
        if let Some(p) = alice.wait(0.3, |p| matches!(p, ServerPacket::MonsterMove { guid, .. } if *guid == enemy)) {
            if let ServerPacket::MonsterMove { path, start, .. } = p {
                last_pos = Some(path.last().copied().unwrap_or(start));
            }
        }
        if let Some(pos) = last_pos {
            alice.move_to([pos.x + 1.5, pos.y, pos.z]);
        }
        let died = alice.seen.iter().any(|p| match p {
            ServerPacket::UpdateObject { objects, .. } => objects.iter().any(|o| matches!(o,
                messages::Object::Values { guid, mask } if *guid == enemy && mask.raw_fields().any(|(i, v)| i == 22 && v == 0))),
            _ => false,
        });
        if died {
            killed = true;
            break;
        }
    }
    assert!(alice.seen.iter().any(|p| matches!(p, ServerPacket::AttackStart { attacker, .. } if *attacker == a_guid)), "alice swings");
    assert!(alice.seen.iter().any(|p| matches!(p, ServerPacket::AttackerState(s) if s.attacker == a_guid && s.damage > 0)), "alice hits");
    assert!(killed, "alice kills the enemy");
    // The Ancient's forge opens on a right-click.
    let hub = alice.seen.iter().find_map(|p| creates(p).into_iter().find(|(g, _)| (g >> 24) & 0xFF_FFFF == 1000)).unwrap().0;
    alice.move_to([4.0, 0.0, 0.0]);
    alice.drain(0.3);
    alice.send(op::CMSG_GOSSIP_HELLO, &messages::gossip_hello(hub));
    assert!(alice.wait(2.0, |p| matches!(p, ServerPacket::GossipMessage { npc, .. } if *npc == hub)).is_some(), "the forge");

    // Logging out removes Alice from Bob's world.
    alice.send(op::CMSG_LOGOUT_REQUEST, &[]);
    assert!(alice.wait(2.0, |p| p.name() == "SMSG_LOGOUT_COMPLETE").is_some());
    assert!(bob.wait(3.0, |p| matches!(p, ServerPacket::DestroyObject { guid } if *guid == a_guid)).is_some());
    bob.drain(0.5);
}

/// The add-on's lines, without the `BB` prefix.
fn addon_lines(c: &Client) -> Vec<String> {
    common::addon_lines(c, "BB")
}

#[test]
fn the_addon_gets_the_card_picker_and_the_hearthstone_works() {
    let server = start();
    let mut c = Client::connect(server.login, "carol", "hold", "Basin Battle").expect("carol logs in");
    enter(&mut c, 2); // mage
    // Without the add-on, the starting pick comes as a dialog.
    assert!(c.wait(3.0, |p| matches!(p, ServerPacket::GossipMessage { .. })).is_some(), "the dialog first");
    // The add-on says hello: the dialog gives way to the picker.
    c.chat(".bb hello 3");
    assert!(c.wait(2.0, |p| p.name() == "SMSG_GOSSIP_COMPLETE").is_some(), "the dialog closes");
    let shown = c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tOS~")));
    assert!(shown.is_some(), "the picker is shown: {:?}", addon_lines(&c));
    let lines = addon_lines(&c);
    let offer = lines.iter().find(|l| l.starts_with("O~")).expect("an offer line");
    let f: Vec<&str> = offer.split('~').collect();
    assert_eq!(f[2], "start");
    assert_eq!(f[6], "3", "three cards: {offer}");
    let cards: Vec<&String> = lines.iter().filter(|l| l.starts_with("C~")).collect();
    assert_eq!(cards.len(), 3);
    for card in &cards {
        let f: Vec<&str> = card.split('~').collect();
        assert_eq!(f.len(), 15, "{card}");
        assert_eq!(f[8], "Spell");
        assert!(f[11].contains("damage"), "{card}");
    }
    assert!(lines.iter().any(|l| l.starts_with("H~")), "the wave tracker");
    // Picking from the picker.
    c.chat(".bb pick 0");
    assert!(
        c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tK~"))).is_some(),
        "the card is taken"
    );
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tB~") && m.text.len() > 5)).is_some(), "the spells row");
    // Announcements come as add-on lines, not raid warnings.
    c.chat(".start");
    assert!(c.wait(3.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tS~wave~Wave 1"))).is_some(), "the wave banner");
    assert!(!c.seen.iter().any(|p| matches!(p, ServerPacket::MessageChat(m) if m.chat_type == 0x58 && m.text.contains("Wave 1"))), "no raid warning");
    // The spell bar: the picked spell sits on key 1, cast by hand.
    let bar = addon_lines(&c).into_iter().rev().find(|l| l.starts_with("Q~")).expect("a spell bar line");
    let f: Vec<&str> = bar.split('~').collect();
    assert_eq!(f[1], "1", "cast by hand: {bar}");
    assert_eq!(f[4].split('^').count(), 10, "{bar}");
    // The talent window: three trees and their talents, and the player's points.
    let lines = addon_lines(&c);
    assert_eq!(lines.iter().filter(|l| l.starts_with("TT~")).count(), 3);
    assert!(lines.iter().filter(|l| l.starts_with("TD~")).count() >= 25);
    assert!(lines.iter().any(|l| l.starts_with("TR~")));
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tT~main^"))).is_some(), "the objective tracker");
    // No enemy near the Ancient yet: the cast says why it didn't go off.
    c.chat(".bb cast 1");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tE~"))).is_some(), "an error line");
    c.chat(".bb cast 5");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tE~No spell"))).is_some(), "an empty key");

    // Version 4: the Ancient's shop as cards, bought from anywhere.
    c.chat(".bb hello 4");
    let shop = c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tSH~")));
    let shop = match shop {
        Some(ServerPacket::MessageChat(m)) => m.text.clone(),
        _ => panic!("the shop: {:?}", addon_lines(&c)),
    };
    let items: Vec<&str> = shop.trim_start_matches("BB\tSH~").split(';').collect();
    assert!(items.len() >= 8, "{shop}");
    assert!(items.iter().all(|i| i.split('^').count() == 9), "{shop}");
    assert!(items[0].starts_with("protector^"), "{}", items[0]);
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tFR~"))).is_some(), "the fronts");
    c.chat(".bb hold 0");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tPG~0~"))).is_some(), "a hold ping");
    c.chat(".bb buy thornbark");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.starts_with("BB\tE~That costs"))).is_some(), "too poor");
    c.chat(".bb shop");
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text == "BB\tSO~0")).is_some(), "the shop opens");

    // The Hearthstone sits in the first bag slot; away from the Ancient it starts a cast everyone
    // sees, and the hero says they're heading home.
    let stone = htl_sim::items::HEARTHSTONE;
    c.send(op::CMSG_ITEM_QUERY_SINGLE, &messages::item_query(stone, 0));
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::ItemQueryResponse { .. })).is_some(), "the stone's tooltip");
    c.move_to([60.0, 20.0, 0.0]);
    c.drain(0.3);
    c.send(op::CMSG_USE_ITEM, &messages::use_item(255, 23, 0, messages::UseItemTarget::SelfImplicit));
    assert!(c.wait(2.0, |p| p.name() == "SMSG_SPELL_START").is_some(), "hearthing: {:?}", c.seen.last().map(|p| p.name()));
    assert!(c.wait(2.0, |p| matches!(p, ServerPacket::MessageChat(m) if m.text.contains("Ancient") && !m.text.starts_with("BB\t"))).is_some(), "a line said");

    // ADDON_DUMP=<file>: every add-on line this session saw, for the add-on's own harness.
    if let Some(path) = std::env::var_os("ADDON_DUMP") {
        c.chat(".bb map 1");
        c.chat(".bb help");
        c.chat(".sos");
        c.drain(1.5);
        std::fs::write(path, addon_lines(&c).join("\n")).unwrap();
    }
}

/// A long fall hurts: the client's landing (with its fall time) after standing on high ground.
#[test]
fn a_long_fall_hurts() {
    let server = start();
    let mut c = Client::connect(server.login, "fay", "hold", "Basin Battle").expect("fay logs in");
    enter(&mut c, 2);
    c.drain(0.5);
    // On a ledge 50 yards up, then a landing 3.2 seconds later at the bottom.
    c.move_to([20.0, 0.0, 50.0]);
    let mut b = Vec::new();
    b.extend_from_slice(&0u32.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in [24.0f32, 0.0, 0.0] {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&0f32.to_le_bytes());
    b.extend_from_slice(&3200u32.to_le_bytes());
    c.send(op::MSG_MOVE_FALL_LAND, &b);
    let hurt = c.wait(2.0, |p| p.name() == "SMSG_ENVIRONMENTALDAMAGELOG");
    assert!(hurt.is_some(), "the fall is logged as fall damage");
    if let Some(ServerPacket::EnvironmentalDamageLog(d)) = hurt {
        assert_eq!(d.damage_type, 2);
        assert!(d.damage > 0);
    }
    // A short hop costs nothing.
    c.move_to([24.0, 0.0, 0.0]);
    let mut b2 = b.clone();
    let n = b2.len();
    b2[n - 4..].copy_from_slice(&700u32.to_le_bytes());
    c.send(op::MSG_MOVE_FALL_LAND, &b2);
    assert!(c.wait(1.0, |p| p.name() == "SMSG_ENVIRONMENTALDAMAGELOG").is_none(), "a hop is free");
}
