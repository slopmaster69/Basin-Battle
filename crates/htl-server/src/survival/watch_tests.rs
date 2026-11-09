use super::*;
use benilla_protocol::messages::{parse_server_with_tail, Object, ServerPacket};
use crossbeam_channel::Receiver;

struct Flat;
impl htl_survival::Land for Flat {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn height_near(&self, _: f32, _: f32, z: f32) -> f32 { z }
}

fn world(realm: Realm) -> SurvivalGame {
    let content = htl_survival::Content::builtin();
    let content = if realm.below { content.for_dungeon_of("deadmines") } else { content };
    let mut game = SurvivalGame::new_in(realm, content, Box::new(Flat), None, 11);
    game.sim.debug = true;
    game
}

/// A hero of that name in the world on connection `conn`, and what their client is sent.
fn hero(game: &mut SurvivalGame, conn: ConnId, name: &str) -> (Id, u64, Receiver<Packet>) {
    let account = format!("spelltest/{}", name.to_lowercase());
    let id = game.sim.join(&account, name, None);
    let pos = game.sim.players[&id].pos;
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    game.sessions.insert(conn, Session { account: "spelltest".into(), hero: account, out: tx,
        world: Some(InWorld::new(guid, id, name.to_string(), pos[2])), watching: None });
    (id, guid, rx)
}

/// A line said in chat, as the client sends it.
fn say(game: &mut SurvivalGame, conn: ConnId, text: &str) {
    let mut b = crate::wire::Body::new();
    b.u32(0).u32(0).cstr(text);
    game.handle(Inbound::Packet { conn, opcode: u32::from(op::CMSG_MESSAGECHAT), body: b.into_vec() });
}

fn drain(rx: &Receiver<Packet>) -> Vec<Packet> {
    rx.try_iter().collect()
}

fn created(packets: &[Packet]) -> Vec<u64> {
    let mut out = Vec::new();
    for p in packets {
        let Ok((ServerPacket::UpdateObject { objects, .. }, _)) = parse_server_with_tail(p.opcode, &p.body) else { continue };
        for o in objects {
            if let Object::Create { guid, .. } = o {
                out.push(guid);
            }
        }
    }
    out
}

fn destroyed(packets: &[Packet], guid: u64) -> bool {
    let gone = packets::destroy_object(guid);
    packets.iter().any(|p| p.opcode == gone.opcode && p.body == gone.body)
}

fn told(packets: &[Packet], words: &str) -> bool {
    packets.iter().any(|p| p.opcode == op::SMSG_MESSAGECHAT && p.body.windows(words.len()).any(|w| w == words.as_bytes()))
}

#[test]
fn a_watcher_leaves_the_rules_and_every_other_client() {
    let mut game = world(Realm::CAMP);
    let (hero_id, _, hero_rx) = hero(&mut game, 1, "Vcwarrior");
    let (cam, cam_guid, cam_rx) = hero(&mut game, 2, "Vccamera");
    say(&mut game, 2, ".es watch vcwarrior");
    assert!(told(&drain(&cam_rx), "You're watching Vcwarrior."), "the hero's name as the server has it");
    assert!(!game.sim.players.contains_key(&cam), "no rule sees the watcher");
    assert!(game.watchers.contains_key(&cam));
    assert!(destroyed(&drain(&hero_rx), cam_guid), "the hero's client lets the watcher's body go");
    // Another client's packets and the rules' steps never bring it back.
    say(&mut game, 1, ".es where");
    for _ in 0..20 {
        game.tick();
    }
    assert!(!game.sim.players.contains_key(&cam));
    assert!(game.sim.players.contains_key(&hero_id));
    // A hero coming in later is never told of it.
    let (_, _, late_rx) = hero(&mut game, 3, "Late");
    game.send_world_to(3, game.world_of(3).unwrap().id);
    let seen = created(&drain(&late_rx));
    assert!(!seen.contains(&cam_guid), "{seen:?}");
    // Its own lines still answer, and it's still out after them.
    say(&mut game, 2, ".es watch");
    assert!(told(&drain(&cam_rx), "You're watching Vcwarrior."));
    assert!(!game.sim.players.contains_key(&cam));
    // It plays no part: its keys and the rest of its lines do nothing.
    let foes = game.sim.mobs.len();
    say(&mut game, 2, ".es spawn dm_greenskin");
    say(&mut game, 2, ".es in a");
    game.handle(Inbound::Packet { conn: 2, opcode: u32::from(op::CMSG_ATTACKSWING), body: 1u64.to_le_bytes().to_vec() });
    assert_eq!(game.sim.mobs.len(), foes);
    let heard = drain(&cam_rx);
    assert!(told(&heard, "You're watching. Use .es watch off to play again."), "the spawn is turned down");
    assert!(heard.iter().all(|p| p.opcode != op::SMSG_ATTACKSWING_CANT_ATTACK), "the attack isn't even heard");
    assert!(!game.sim.players.contains_key(&cam));
}

#[test]
fn nothing_strikes_a_watcher_even_a_foe_set_on_it() {
    let foe = world(Realm::BELOW).sim.content.mobs[0].key.clone();
    // A hero who plays is struck in the same time.
    let mut control = world(Realm::BELOW);
    let (played, _, _rx) = hero(&mut control, 1, "Vcwarrior");
    say(&mut control, 1, &format!(".es spawn {foe}"));
    for _ in 0..200 {
        control.tick();
    }
    assert!(control.sim.players[&played].health < control.sim.players[&played].max_health, "the control hero is struck");
    let mut game = world(Realm::BELOW);
    let (cam, _, _cam_rx) = hero(&mut game, 2, "Vccamera");
    // Set on it while it still played, then watching (a hero who isn't on).
    say(&mut game, 2, &format!(".es spawn {foe}"));
    say(&mut game, 2, ".es watch Nobody");
    let whole = game.watchers[&cam].max_health;
    for _ in 0..200 {
        game.tick();
    }
    assert_eq!(game.watchers[&cam].health, whole);
    assert!(game.sim.players.is_empty(), "a dungeon with only a watcher in it has no one in it");
}

#[test]
fn a_watcher_is_saved_and_leaves_whole() {
    let mut game = world(Realm::CAMP);
    let (_, _, _rx) = hero(&mut game, 1, "Vcwarrior");
    let (_, _, _cam_rx) = hero(&mut game, 2, "Vccamera");
    say(&mut game, 2, ".es watch Vcwarrior");
    assert!(game.offline_and_watchers().contains_key("spelltest/vccamera"), "the save keeps the watcher's hero");
    game.handle(Inbound::Disconnected { conn: 2 });
    assert!(game.offline.contains_key("spelltest/vccamera"), "and so does a logout");
    assert!(game.watchers.is_empty());
}

#[test]
fn a_watcher_stays_near_its_hero_and_shows_again_when_it_stops() {
    let mut game = world(Realm::CAMP);
    let (hero_id, _, hero_rx) = hero(&mut game, 1, "Vcwarrior");
    let (cam, cam_guid, cam_rx) = hero(&mut game, 2, "Vccamera");
    say(&mut game, 2, ".es watch Vcwarrior");
    let far = {
        let p = game.sim.players.get_mut(&hero_id).unwrap();
        p.pos[0] += 120.0;
        p.pos
    };
    drain(&cam_rx);
    game.tick();
    assert_eq!(game.watchers[&cam].pos, far, "its body goes to the hero");
    assert!(drain(&cam_rx).iter().any(|p| p.opcode == op::MSG_MOVE_TELEPORT_ACK), "and its client is moved there");
    // A watcher's own moves are told to no one.
    drain(&hero_rx);
    let mut b = crate::wire::Body::new();
    b.u32(1).u32(0).vec3([0.0, 0.0, 0.0]).f32(0.0).u32(0);
    game.handle(Inbound::Packet { conn: 2, opcode: u32::from(op::MSG_MOVE_START_FORWARD), body: b.into_vec() });
    assert!(drain(&hero_rx).is_empty());
    assert_eq!(game.watchers[&cam].pos, far);
    say(&mut game, 2, ".es watch off");
    assert!(game.sim.players.contains_key(&cam), "back in the rules");
    assert!(created(&drain(&hero_rx)).contains(&cam_guid), "and seen again");
}

#[test]
fn a_watcher_follows_its_hero_into_another_world() {
    let mut camp = world(Realm::CAMP);
    let below = world(Realm::BELOW);
    let (_, _, _cam_rx) = hero(&mut camp, 2, "Vccamera");
    say(&mut camp, 2, ".es watch Vcwarrior");
    let mut realms = crate::survival::realms::Realms::new(camp, vec![below]);
    let (hero_id, _, _rx) = hero(&mut realms.below[0], 1, "Vcwarrior");
    realms.in_below.insert(1, 0);
    let at = realms.below[0].sim.players[&hero_id].pos;
    realms.follow_watched();
    assert_eq!(realms.in_below.get(&2), Some(&0), "the watcher went below with its hero");
    let below = &realms.below[0];
    let w = below.world_of(2).expect("in the dungeon's world");
    assert!(w.porting && below.watching(2));
    assert_eq!(below.watchers[&w.id].pos, at, "beside the hero, and out of the rules there too");
    assert!(!below.sim.players.contains_key(&w.id));
    assert!(realms.camp.world_of(2).is_none());
}
