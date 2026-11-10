//! The Juggernaut's starboard rail as a hero's client meets it: movement reports through the
//! server, its own carrying-on between them, and the stop it sends back.
use super::*;
use crossbeam_channel::Receiver;

struct Flat;
impl htl_survival::Land for Flat {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn height_near(&self, _: f32, _: f32, z: f32) -> f32 { z }
}

/// The rail's line at x -64 (between its points at x -68 and -60).
const RAIL_Y: f32 = -839.75;

fn deck() -> (SurvivalGame, Id, Receiver<Packet>) {
    let content = htl_survival::Content::builtin().for_dungeon_of("deadmines");
    let mut game = SurvivalGame::new_in(Realm::BELOW, content, Box::new(Flat), None, 11);
    game.sim.debug = true;
    let account = "railtest/vcwarrior".to_string();
    let id = game.sim.join(&account, "Vcwarrior", None);
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    let pos = game.sim.players[&id].pos;
    game.sessions.insert(1, Session { account: "railtest".into(), hero: account, out: tx,
        world: Some(InWorld::new(guid, id, "Vcwarrior".into(), pos[2])), watching: None });
    let mut b = crate::wire::Body::new();
    b.u32(0).u32(0).cstr(".es goto -64 -835.5 41.2 4.712389");
    game.handle(Inbound::Packet { conn: 1, opcode: u32::from(op::CMSG_MESSAGECHAT), body: b.into_vec() });
    for _ in 0..5 { game.tick(); }
    let _: Vec<_> = rx.try_iter().collect();
    (game, id, rx)
}

/// A movement report from the hero's client: running forward (south, toward the rail) or stopped.
fn report(game: &mut SurvivalGame, y: f32, running: bool) {
    let mut b = crate::wire::Body::new();
    b.u32(u32::from(running)).u32(0).vec3([-64.0, y, 41.25]).f32(4.712389).u32(0);
    game.movement(1, if running { op::MSG_MOVE_HEARTBEAT } else { op::MSG_MOVE_STOP }, &b.into_vec()).unwrap();
}

fn stops(rx: &Receiver<Packet>) -> Vec<[f32; 3]> {
    rx.try_iter().filter(|p| p.opcode == op::MSG_MOVE_TELEPORT_ACK).filter_map(|p| {
        let b = &p.body;
        // Packed guid, then the counter, the flags and the time, then the position.
        let mask = b[0];
        let at = 1 + mask.count_ones() as usize + 4 + 4 + 4;
        let f = |k: usize| f32::from_le_bytes(b[at + 4 * k..at + 4 * k + 4].try_into().unwrap());
        Some([f(0), f(1), f(2)])
    }).collect()
}

/// Take 370: a hero walking (a dash, held keys) through the rail went through, and was snapped
/// seconds later to 3 yd past it. The server carried him on between reports and over the rail,
/// then took his next report, still short of where it had carried him, for a crossing back.
#[test]
fn a_hero_running_through_the_rail_is_stopped_at_it_on_the_deck_at_once() {
    let (mut game, id, rx) = deck();
    let mut y = -835.5;
    let mut held = Vec::new();
    for _ in 0..30 {
        report(&mut game, y, true);
        // Two server ticks between reports: it carries him on meanwhile.
        game.tick();
        game.tick();
        let p = game.sim.players[&id].pos;
        assert!(p[1] > RAIL_Y, "never past the rail on the server: {p:?}");
        assert!(game.sim.players[&id].last_safe[1] > RAIL_Y, "nor its firm ground");
        held.extend(stops(&rx));
        // His client, uncorrected, keeps going.
        y -= 0.5;
    }
    assert!(!held.is_empty(), "told to stop");
    for at in &held {
        assert!(at[1] > RAIL_Y && at[1] - RAIL_Y <= 0.5, "stopped on the deck, at the rail: {at:?}");
    }
    // Stopping there, he stays.
    report(&mut game, -839.3, false);
    assert_eq!(game.sim.players[&id].pos[1], -839.3);
}

/// Coming back aboard over the rail is never held (a hero who got off the deck some other way).
#[test]
fn coming_back_aboard_over_the_rail_is_free() {
    let (mut game, id, rx) = deck();
    game.sim.players.get_mut(&id).unwrap().pos = [-64.0, -841.5, 41.25];
    report(&mut game, -838.0, false);
    assert_eq!(game.sim.players[&id].pos[1], -838.0);
    assert!(stops(&rx).is_empty());
}
