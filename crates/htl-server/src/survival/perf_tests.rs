//! Network-path benchmarks (run by hand, `#[ignore]`d): what the server builds and sends per tick
//! for 5 connected heroes (add-on, action layer) in a heavy scene: the 150-raider Elwynn wave and
//! a Deadmines boss fight. Packets go to in-process channels as they go to the writer threads, so
//! this is the server's own cost (rules step, events played into packets, add-on lines), not the
//! socket's. Release build:
//! `cargo test --release -p htl-server --lib perf_ -- --ignored --nocapture --test-threads 1`

use super::*;
use crossbeam_channel::Receiver;
use htl_survival::{Input, Moving};
use std::collections::BTreeMap as Map;

struct Flat;
impl htl_survival::Land for Flat {
    fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
    fn height_near(&self, _: f32, _: f32, z: f32) -> f32 { z }
}

const HEROES: usize = 5;
const TICKS: usize = 600;

fn hero(game: &mut SurvivalGame, conn: ConnId, k: usize, place: bool) -> (Id, Receiver<Packet>) {
    let name = format!("Perf{k}");
    let account = format!("perftest/{}", name.to_lowercase());
    let id = game.sim.join(&account, &name, None);
    if place {
        game.sim.move_player(id, [3.0 + k as f32, k as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
    }
    let p = game.sim.players.get_mut(&id).unwrap();
    p.health = 1_000_000;
    p.max_health = 1_000_000;
    let guid = SurvivalGame::player_guid(&account);
    game.bind(id, guid);
    let (tx, rx) = crossbeam_channel::unbounded();
    let mut w = InWorld::new(guid, id, name, 0.0);
    w.addon = true;
    w.anim = true;
    w.local = true;
    w.level = play::LAYER_LEVEL;
    game.sessions.insert(conn, Session { account: "perftest".into(), hero: account, out: tx, world: Some(w), watching: None });
    game.send_world_to(conn, id);
    (id, rx)
}

/// What one packet is, for the tally: the opcode, and for the add-on's lines (chat in the party
/// channel in the add-on's language) the verb.
fn kind(p: &Packet) -> String {
    if p.opcode == op::SMSG_MESSAGECHAT {
        if let Some(i) = p.body.windows(5).position(|w| w == b"HTLES") {
            let tail = &p.body[i..];
            let tab = tail.iter().position(|b| *b == b'\t').unwrap_or(0);
            let end = tail[tab + 1..].iter().position(|b| *b == b'\t' || *b == 0).unwrap_or(0);
            return format!("addon {}", String::from_utf8_lossy(&tail[tab + 1..tab + 1 + end]));
        }
    }
    format!("op {:#06x}", p.opcode)
}

#[derive(Default)]
struct Tally {
    n: usize,
    bytes: usize,
}

fn run(name: &str, mut game: SurvivalGame, rxs: Vec<(Id, Receiver<Packet>)>, ticks: usize) {
    let band: Vec<Id> = rxs.iter().map(|(id, _)| *id).collect();
    for (_, rx) in &rxs {
        rx.try_iter().for_each(drop);
    }
    let mut per_tick: Vec<f64> = Vec::new();
    let (mut step_ms, mut play_ms, mut flush_ms, mut move_ms) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut bytes_tick: Vec<usize> = Vec::new();
    let mut by_kind: Map<String, Tally> = Map::new();
    let mut by_event: Map<String, (usize, f64)> = Map::new();
    let hud_back = Instant::now().checked_sub(Duration::from_millis(150));
    let meter_back = Instant::now().checked_sub(Duration::from_millis(1000));
    for tick in 0..ticks {
        // The wall clock the HUD and meters wait on, run as a real 20 Hz server would.
        if tick % 3 == 0 {
            if let Some(t) = hud_back {
                game.hud_at = t;
            }
        }
        if tick % 20 == 0 {
            if let Some(t) = meter_back {
                game.meter_at = t;
            }
        }
        if tick % 40 == 0 {
            for s in game.sessions.values_mut() {
                if let Some(w) = s.world.as_mut() {
                    w.ping_at = None;
                }
            }
        }
        // The heroes keep up their fight; their clients send a heartbeat every half second.
        for (k, h) in band.iter().enumerate() {
            if let Some(p) = game.sim.players.get_mut(h) {
                p.health = p.max_health;
                p.stamina = 100.0;
            }
            if tick % 8 == k % 8 {
                game.sim.input(*h, Input::Attack);
            }
        }
        let t = Instant::now();
        for (k, h) in band.iter().enumerate() {
            if tick % 10 == k {
                let pos = game.sim.players.get(h).map_or([0.0; 3], |p| p.pos);
                let a = tick as f32 * 0.05 + k as f32;
                let mut b = crate::wire::Body::new();
                b.u32(1).u32(tick as u32 * 50).f32(pos[0] + a.cos() * 0.5).f32(pos[1] + a.sin() * 0.5).f32(pos[2]).f32(a);
                b.u32(0);
                game.handle(Inbound::Packet { conn: k as ConnId + 1, opcode: u32::from(op::MSG_MOVE_HEARTBEAT), body: b.into_vec() });
            }
        }
        move_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        let events = game.sim.step(TICK.as_secs_f32());
        step_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        for e in events {
            let label = format!("{e:?}");
            let label = label.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string();
            let t1 = Instant::now();
            game.play(vec![e]);
            let slot = by_event.entry(label).or_default();
            slot.0 += 1;
            slot.1 += t1.elapsed().as_secs_f64() * 1000.0;
        }
        play_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        let t = Instant::now();
        game.wreck_tick();
        game.keep_watchers_near();
        game.ui_flush();
        game.hold_clock();
        game.weather_tick(false);
        flush_ms.push(t.elapsed().as_secs_f64() * 1000.0);
        per_tick.push(step_ms.last().unwrap() + play_ms.last().unwrap() + flush_ms.last().unwrap());
        let mut total = 0;
        for (_, rx) in &rxs {
            for p in rx.try_iter() {
                let size = p.body.len() + 4;
                total += size;
                let slot = by_kind.entry(kind(&p)).or_default();
                slot.n += 1;
                slot.bytes += size;
            }
        }
        bytes_tick.push(total);
    }
    let mut states: Map<String, usize> = Map::new();
    for m in game.sim.mobs.values().filter(|m| m.health > 0) {
        let s = format!("{:?}", m.state);
        *states.entry(s.split(|c: char| !c.is_alphanumeric()).next().unwrap_or("").to_string()).or_default() += 1;
    }
    eprintln!("NET {name}: creatures alive at the end by state: {states:?}");
    let med = |pick: &dyn Fn(usize) -> bool| {
        let mut v: Vec<f64> = flush_ms.iter().enumerate().filter(|(i, _)| pick(*i)).map(|(_, m)| *m).collect();
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        (v[v.len() / 2], v[v.len() - 1])
    };
    let (meter, hud, plain) = (med(&|i| i % 20 == 0), med(&|i| i % 3 == 0 && i % 20 != 0), med(&|i| i % 3 != 0 && i % 20 != 0));
    eprintln!(
        "NET {name}: ui_flush ticks (median/max ms): with meters {:.3}/{:.3}, with the HUD lines {:.3}/{:.3}, neither {:.3}/{:.3}",
        meter.0, meter.1, hud.0, hud.1, plain.0, plain.1
    );
    let q = |v: &mut Vec<f64>, p: f64| {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
        v[((v.len() - 1) as f64 * p) as usize]
    };
    let mut all = per_tick.clone();
    eprintln!(
        "NET {name}: {} heroes, {} ticks; server cost per tick (step+play+flush): median {:.3} ms, p99 {:.3} ms, max {:.3} ms",
        band.len(), ticks, q(&mut all, 0.5), q(&mut all, 0.99), all[all.len() - 1]
    );
    eprintln!(
        "NET {name}: of which step median {:.3} ms, play (events to packets) median {:.3} ms, ui_flush & co median {:.3} ms (p99 {:.3}); client movement in median {:.3} ms",
        q(&mut step_ms, 0.5), q(&mut play_ms, 0.5), q(&mut flush_ms.clone(), 0.5), q(&mut flush_ms, 0.99), q(&mut move_ms, 0.5)
    );
    let sum: usize = bytes_tick.iter().sum();
    let packets: usize = by_kind.values().map(|t| t.n).sum();
    let mut b = bytes_tick.clone();
    b.sort();
    eprintln!(
        "NET {name}: bytes per tick to all {} clients: mean {:.0}, median {}, p99 {}, max {}; {:.1} packets per tick ({:.0} bytes per hero per tick)",
        band.len(), sum as f64 / ticks as f64, b[b.len() / 2], b[(b.len() - 1) * 99 / 100], b[b.len() - 1], packets as f64 / ticks as f64,
        sum as f64 / ticks as f64 / band.len() as f64
    );
    let mut kinds: Vec<_> = by_kind.iter().collect();
    kinds.sort_by_key(|(_, t)| std::cmp::Reverse(t.bytes));
    for (k, t) in kinds.iter().take(14) {
        eprintln!("NET   {k:<28} {:>7} packets {:>9} bytes ({:.1}%)", t.n, t.bytes, t.bytes as f64 / sum.max(1) as f64 * 100.0);
    }
    let mut events: Vec<_> = by_event.iter().collect();
    events.sort_by(|a, b| b.1 .1.partial_cmp(&a.1 .1).unwrap());
    for (k, (n, ms)) in events.iter().take(8) {
        eprintln!("NET   event {k:<22} {n:>6} played, {:.3} ms total, {:.2} us each", ms, ms / *n as f64 * 1000.0);
    }
}

fn camp_wave(raiders: usize) -> (SurvivalGame, Vec<(Id, Receiver<Packet>)>) {
    let mut content = htl_survival::Content::builtin();
    content.pvp = Default::default();
    let mut game = SurvivalGame::new_in(Realm::CAMP, content, Box::new(Flat), None, 11);
    game.sim.bench_clear_creatures();
    let heroes: Vec<_> = (0..HEROES).map(|k| hero(&mut game, k as ConnId + 1, k, true)).collect();
    game.tick();
    let band: Vec<Id> = heroes.iter().map(|(id, _)| *id).collect();
    game.sim.bench_crowd(&["blackrock_grunt", "blackrock_caster", "prowler", "kobold_tunneler", "grunt", "archer", "shaman", "wolf"], raiders, true, &band, [0.0; 3]);
    (game, heroes)
}

#[test]
#[ignore]
fn perf_net_elwynn_wave_5_heroes_150_raiders() {
    let (game, heroes) = camp_wave(150);
    run("elwynn wave x150", game, heroes, TICKS);
}

#[test]
#[ignore]
fn perf_net_deadmines_boss_5_heroes() {
    for key in ["dm_vancleef", "dm_shredder", "dm_sneed"] {
        let content = htl_survival::Content::builtin().for_dungeon_of("deadmines");
        let mut game = SurvivalGame::new_in(Realm::BELOW, content, Box::new(Flat), None, 11);
        game.sim.bench_clear_creatures();
        let heroes: Vec<_> = (0..HEROES).map(|k| hero(&mut game, k as ConnId + 1, k, false)).collect();
        game.tick();
        let band: Vec<Id> = heroes.iter().map(|(id, _)| *id).collect();
        let at = game.sim.players[&band[0]].pos;
        game.sim.bench_boss(key, band[0], [at[0] + 6.0, at[1], at[2]]);
        game.sim.bench_crowd(&["blackrock_grunt", "prowler", "kobold_tunneler", "grunt"], 20, false, &band, at);
        run(&format!("boss {key} + 20 adds"), game, heroes, TICKS);
    }
}

/// The host's log notes a hero with no add-on once, a while after they come in, and never one
/// whose client said hello.
#[test]
fn a_hero_without_the_addon_is_noted_once_after_a_grace() {
    let content = htl_survival::Content::builtin().for_dungeon_of("deadmines");
    let mut game = SurvivalGame::new_in(Realm::BELOW, content, Box::new(Flat), None, 11);
    let (_, _rx1) = hero(&mut game, 1, 0, false);
    let (_, _rx2) = hero(&mut game, 2, 1, false);
    game.world_mut(1).unwrap().addon = false;
    game.log_heroes_without_addon();
    assert!(!game.world_of(1).unwrap().no_addon_logged, "a new arrival gets time to say hello");
    for c in [1, 2] {
        game.world_mut(c).unwrap().entered_at = Instant::now() - Duration::from_secs(60);
    }
    game.log_heroes_without_addon();
    assert!(game.world_of(1).unwrap().no_addon_logged, "no hello after the grace: noted");
    assert!(!game.world_of(2).unwrap().no_addon_logged, "an add-on hero is not");
}
