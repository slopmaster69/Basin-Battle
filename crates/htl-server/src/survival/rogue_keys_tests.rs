//! A Rogue's keys as the real client sends them, through the server's own path: the `.es` lines
//! the visual check says (the add-on's keys, without the add-on's reticle line), the rules stepping
//! at the server's tick, and the client's own movement reports coming back after a teleport.

use super::*;
use htl_survival::{Event, HeroLook, Moving, Outcome};

struct Flat;
impl htl_survival::Land for Flat {
    fn height(&self, _: f32, _: f32) -> f32 {
        0.0
    }
    fn height_near(&self, _: f32, _: f32, z: f32) -> f32 {
        z
    }
}

const PI: f32 = std::f32::consts::PI;

fn game() -> SurvivalGame {
    let mut g = SurvivalGame::new_in(Realm::CAMP, htl_survival::Content::builtin(), Box::new(Flat), None, 11);
    g.sim.mobs.clear();
    g
}

/// A Rogue made as the creation screen makes one, on connection `conn`, at the origin facing +x,
/// on an add-on client.
fn rogue(game: &mut SurvivalGame, conn: ConnId, name: &str) -> (Id, crossbeam_channel::Receiver<Packet>) {
    let account = format!("roguekeys/{}", name.to_lowercase());
    let save = game.sim.fresh_save(&account, name, "rogue", HeroLook::default());
    let id = game.sim.join(&account, name, Some(&save));
    game.sim.move_player(id, [0.0, 0.0, 0.0], 0.0, Moving::default());
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut w = InWorld::new(guid, id, name.to_string(), 0.0);
    (w.addon, w.addon_v, w.anim, w.local, w.level) = (true, ui::ADDON_NEWEST, true, true, play::LAYER_LEVEL);
    game.sessions.insert(conn, Session { account: "roguekeys".into(), hero: account, out: tx, world: Some(w), watching: None });
    (id, rx)
}

/// A movement report from `conn`'s client (`MSG_MOVE_*`): at `pos` facing `facing`, with these
/// movement flags.
fn report(game: &mut SurvivalGame, conn: ConnId, opcode: u16, flags: u32, pos: [f32; 3], facing: f32) {
    let mut b = Vec::new();
    b.extend_from_slice(&flags.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    for v in pos {
        b.extend_from_slice(&v.to_le_bytes());
    }
    b.extend_from_slice(&facing.to_le_bytes());
    b.extend_from_slice(&0u32.to_le_bytes());
    game.movement(conn, opcode, &b).unwrap();
}

/// The server's ticks until `until` (sec on the script's clock), the client answering each of the
/// hero's teleports a tick later with a report of where it stands: facing the pose's own way
/// (`echo` `None`), or the way its view still looked (`Some`). The events seen.
fn run_to(game: &mut SurvivalGame, conn: ConnId, me: Id, echo: Option<f32>, clock: &mut f32, until: f32, seen: &mut Vec<Event>) {
    run_client(game, conn, me, Client { view: echo, ..Client::default() }, clock, until, seen);
}

/// How the film's client answers the server.
#[derive(Clone, Copy, Default)]
struct Client {
    /// After a teleport: the facing its view still holds (`None`: the pose it was put in).
    view: Option<f32>,
    /// From the first teleport on, it says that view on every tick while she stands, not once: an
    /// action client turns the body to its camera, and its camera never turned with the step.
    steady: bool,
    /// From this time on (the script's clock), for 0.3 sec, it carries her along its view as a
    /// forward step on the wire: the art's 0.6 yd lunge, which the add-on plays on its own blow
    /// (`ActionDash`, whose movement flags are the dash's), the film's "you 0.6 yd/s" in Garrote's
    /// wind-up and 0.6 yd off where she was put by the next `.es where`.
    lunge_from: Option<f32>,
}

/// [`run_to`] with the client `client`.
fn run_client(game: &mut SurvivalGame, conn: ConnId, me: Id, client: Client, clock: &mut f32, until: f32, seen: &mut Vec<Event>) {
    let dt = TICK.as_secs_f32();
    let mut owed: Option<([f32; 3], f32)> = None;
    while *clock + dt / 2.0 < until {
        let stepped = game.sim.players[&me].pos[0] > 5.0;
        if let Some((pos, facing)) = owed.take() {
            report(game, conn, op::MSG_MOVE_SET_FACING, 0, pos, client.view.unwrap_or(facing));
        } else if let (Some(view), true) = (client.view, client.steady && stepped) {
            let mut pos = game.sim.players[&me].pos;
            let lunging = client.lunge_from.is_some_and(|t| *clock >= t && *clock < t + 0.3);
            if lunging {
                pos[0] += view.cos() * 2.0 * dt;
                pos[1] += view.sin() * 2.0 * dt;
            }
            // (0x1: forward, the dash's flags.)
            report(game, conn, op::MSG_MOVE_HEARTBEAT, if lunging { 0x1 } else { 0 }, pos, view);
        }
        let events = game.sim.step(dt);
        owed = events.iter().rev().find_map(|e| match e {
            Event::Teleport { player, pos, facing } if *player == me => Some((*pos, *facing)),
            _ => None,
        });
        seen.extend(events.iter().cloned());
        game.play(events);
        *clock += dt;
    }
    if let Some((pos, facing)) = owed {
        report(game, conn, op::MSG_MOVE_SET_FACING, 0, pos, client.view.unwrap_or(facing));
    }
}

/// The visual check's Garrote take (`rogue-garrote.txt`, Nov 17), line for line at the times its
/// client said them (its run.txt), with the keys the film's Rogue had (Shadow's trait on:
/// Backstab, Distract, Cloak and Dagger and Shadowstep, so the step shrouds her). Returns the game,
/// the Rogue, the kobold and the events.
fn film_take(echo: Option<f32>) -> (SurvivalGame, Id, Id, Vec<Event>) {
    film_take_by(Client { view: echo, ..Client::default() }, 0.0)
}

/// [`film_take`] with the client `client`, and the Garrote presses `late` sec after the film's
/// (its clock's beat: the film's own runs pressed it 0.69 to 0.72 sec after the step's click).
fn film_take_by(client: Client, late: f32) -> (SurvivalGame, Id, Id, Vec<Event>) {
    let mut game = game();
    let (me, _rx) = rogue(&mut game, 1, "Vcrogue");
    let script: &[(f32, &str)] = &[
        (0.02, "es debug quiet"),
        (0.33, "es spells"),
        (0.62, "es cd off"),
        (3.91, "es valor full"),
        (4.40, "es slot close backstab"),
        (4.40, "es slot defense leeching_venom"),
        (4.40, "es slot ranged distract"),
        (4.40, "es slot ultimate cloak_and_dagger"),
        (4.43, "es slot 1 shadowstep"),
        (4.61, "es target clear"),
        (4.81, "es target kobold_geomancer 12"),
        // 1: Shadowstep's reticle up, and its click along the line the hero faces.
        (6.61, "es in ab 2 0"),
        (6.82, "es in a 0.0000 -0.15"),
        (6.90, "es in au"),
        (7.11, "es slot 1 garrote"),
        // 1: Garrote, with no line (the visual check's key carries none), and a spare.
        (7.51, "es in ab 2 0"),
        (8.41, "es in ab 2 0"),
    ];
    let mut clock = 0.0;
    let mut seen = Vec::new();
    let client = Client { lunge_from: client.lunge_from.map(|t| t + late), ..client };
    for (at, line) in script {
        let at = if *at > 7.4 { *at + late } else { *at };
        run_client(&mut game, 1, me, client, &mut clock, at, &mut seen);
        game.command(1, line);
    }
    let t = clock + 1.0;
    run_client(&mut game, 1, me, client, &mut clock, t, &mut seen);
    let kobold = game.sim.mobs.iter().find(|(_, m)| game.sim.content.mobs[m.def].key == "kobold_geomancer").map(|(id, _)| *id).expect("the kobold");
    (game, me, kobold, seen)
}

/// Garrote's cut landed on the kobold, and its silence and bleed are on it.
fn garroted(game: &SurvivalGame, me: Id, kobold: Id, seen: &[Event]) -> bool {
    let cut = seen.iter().any(|e| matches!(e, Event::Swing { attacker, victim: Some(v), outcome: Outcome::Hit, .. } if *attacker == me && *v == kobold));
    cut && game.sim.has_cc(kobold, "silence") && game.sim.has_cc(kobold, "burn")
}

/// The take with a client that answers the step's teleport with the pose it was put in: the cut
/// lands in the kobold's back (silenced, bleeding).
#[test]
fn garrote_after_a_shadowstep_lands_through_the_servers_key_path() {
    let (game, me, kobold, seen) = film_take(None);
    let (p, k) = (&game.sim.players[&me], &game.sim.mobs[&kobold]);
    assert!(htl_survival::dist2d(p.pos, k.pos) < 4.0, "behind it: {:?} and {:?}", p.pos, k.pos);
    assert!(garroted(&game, me, kobold, &seen), "the cut lands, silences and bleeds");
}

/// The Nov 17 film: the client answered the step's teleport with the facing its view still held
/// (the line Shadowstep was clicked along, 0, away from the back she was put behind). The server
/// took it, and Garrote, pressed with no line of its own, swung along it: 0 hits, 0 foes, twice.
/// A Rogue's step now keeps the facing it put her in against a standing client's word for a moment.
#[test]
fn garrote_after_a_shadowstep_lands_when_the_client_reports_its_old_view() {
    let (game, me, kobold, seen) = film_take(Some(0.0));
    assert!(garroted(&game, me, kobold, &seen), "the cut lands in the back, as the step put her");
}

/// The Nov 17 refilm's 1 in 5: the client holds its view's facing (0, the line Shadowstep was
/// clicked along) on every tick, and lunges along it as Garrote winds up. Garrote pressed 0.7 sec
/// after the step winds up 0.3 sec, so it struck just as the step's facing hold and Shrouded Step's
/// veil (1 sec each) ran out: a beat later and it swung along the client's view (0 foes), or the
/// kobold had turned round (a 50% cut in its face, no silence). A blow begun inside the step's
/// moment now finishes inside it, its own lunge included, so it lands on any beat of the film's
/// clock, and on any beat up to the end of the hold.
#[test]
fn garrote_after_a_shadowstep_lands_on_any_beat_inside_the_steps_moment() {
    // The lunge begins a little before the blow's impact (the add-on's dash: impact less its own
    // length), so on the wire before the server's strike.
    let client = Client { view: Some(0.0), steady: true, lunge_from: Some(7.51 + 0.12) };
    // The film's own presses were 0.69 to 0.72 sec after the click (t=7.50, 7.52); up to 0.95.
    for i in 0..=14 {
        let late = -0.06 + i as f32 * 0.02;
        let (game, me, kobold, seen) = film_take_by(client, late);
        assert!(garroted(&game, me, kobold, &seen), "pressed {late:+.2} sec off the film's beat: the cut lands in the back, silences and bleeds");
    }
}

/// The hold is only against a standing client's word, and only for a moment: a hero who moves,
/// or whose client sends a new reticle line, faces where they say at once; after the hold, a
/// report is taken as always.
#[test]
fn a_rogues_step_facing_gives_way_to_movement_a_new_line_and_time() {
    let fresh = || {
        let mut game = game();
        let (me, rx) = rogue(&mut game, 1, "Ayla");
        game.command(1, "es debug quiet");
        game.command(1, "es spells");
        game.command(1, "es slot 1 shadowstep");
        game.command(1, "es target kobold_geomancer 12");
        game.sim.set_valor(me, 100.0);
        game.command(1, "es in ab 2 0");
        let mut clock = 0.0;
        let mut seen = Vec::new();
        run_to(&mut game, 1, me, None, &mut clock, 0.2, &mut seen);
        game.command(1, "es in a 0.0000 -0.15");
        run_to(&mut game, 1, me, None, &mut clock, 0.3, &mut seen);
        assert!((game.sim.players[&me].facing - PI).abs() < 0.1, "behind it, facing its back");
        (game, me, rx)
    };
    // Standing: the old view isn't taken.
    let (mut game, me, _rx) = fresh();
    let pos = game.sim.players[&me].pos;
    report(&mut game, 1, op::MSG_MOVE_SET_FACING, 0, pos, 0.0);
    assert!((game.sim.players[&me].facing - PI).abs() < 0.1, "held");
    // Moving (forward): taken.
    report(&mut game, 1, op::MSG_MOVE_START_FORWARD, 0x1, pos, 0.5);
    assert!((game.sim.players[&me].facing - 0.5).abs() < 1e-3, "a hero walking faces their own way");
    // A new reticle line: the hold is over.
    let (mut game, me, _rx) = fresh();
    game.sim.set_aim_line(me, 1.0, 0.0);
    report(&mut game, 1, op::MSG_MOVE_SET_FACING, 0, pos, 1.0);
    assert!((game.sim.players[&me].facing - 1.0).abs() < 1e-3, "a new line, the client's facing again");
    // After the hold: taken.
    let (mut game, me, _rx) = fresh();
    for _ in 0..30 {
        let ev = game.sim.step(TICK.as_secs_f32());
        game.play(ev);
    }
    report(&mut game, 1, op::MSG_MOVE_SET_FACING, 0, pos, 0.0);
    assert!(game.sim.players[&me].facing.abs() < 1e-3, "the hold is a moment's");
}
