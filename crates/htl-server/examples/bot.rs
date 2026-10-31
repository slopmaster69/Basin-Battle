//! A bot player, built on benilla's own protocol code: logs in, enters as a class, and fights the
//! nearest enemy like a player holding auto-attack. Handy for filling a game while testing.
//!
//! `cargo run -p htl-server --example bot -- <host[:port]> <name> <class 0-7> [password] [--start]`

use std::collections::HashMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use benilla_protocol::messages::{Object, ServerPacket};
use benilla_protocol::WorldSession;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let host = args.first().cloned().unwrap_or_else(|| "127.0.0.1".into());
    let name = args.get(1).cloned().unwrap_or_else(|| "Bot".into());
    let class: usize = args.get(2).and_then(|c| c.parse().ok()).unwrap_or(0);
    let password = args.get(3).filter(|a| !a.starts_with("--")).cloned().unwrap_or_else(|| "hold".into());
    let start = args.iter().any(|a| a == "--start");

    let logon = benilla_protocol::logon(&host, &name, &password, u32::from_be_bytes(*b"enUS"))
        .context("logging on")?;
    let realm = logon.realms.first().context("no realm")?;
    let mut s = WorldSession::connect(&realm.address, &name, logon.session_key)?;
    let chars = s.char_enum()?;
    let me = chars.get(class).context("no such class")?;
    let my_guid = me.guid;
    s.player_login(my_guid)?;
    s.set_read_timeout(Some(Duration::from_millis(100)))?;
    println!("{name} entered as character {class}");

    let mut pos = [me.position.x, me.position.y, me.position.z];
    // Enemies (high guid 0xF130, entry 2000+): where they are heading and whether alive.
    let mut enemies: HashMap<u64, ([f32; 3], bool)> = HashMap::new();
    let mut target: Option<u64> = None;
    let mut last = Instant::now();
    let mut started = !start;
    loop {
        match s.recv() {
            Ok(p) => match p {
                ServerPacket::UpdateObject { objects, .. } => {
                    for o in objects {
                        match o {
                            Object::Create { guid, movement, mask, .. } if is_enemy(guid) => {
                                let at = movement.position.map(|(v, _)| [v.x, v.y, v.z]).unwrap_or(pos);
                                let alive = !mask.raw_fields().any(|(i, v)| i == 22 && v == 0);
                                enemies.insert(guid, (at, alive));
                            }
                            Object::Create { guid, movement, .. } if guid == my_guid => {
                                if let Some((v, _)) = movement.position {
                                    pos = [v.x, v.y, v.z];
                                }
                            }
                            Object::Values { guid, mask } if is_enemy(guid) => {
                                if mask.raw_fields().any(|(i, v)| i == 22 && v == 0) {
                                    if let Some(e) = enemies.get_mut(&guid) {
                                        e.1 = false;
                                    }
                                }
                            }
                            _ => {}
                        }
                    }
                }
                ServerPacket::MonsterMove { guid, path, start, .. } if is_enemy(guid) => {
                    let end = path.last().copied().unwrap_or(start);
                    if let Some(e) = enemies.get_mut(&guid) {
                        e.0 = [end.x, end.y, end.z];
                    }
                }
                ServerPacket::DestroyObject { guid } => {
                    enemies.remove(&guid);
                }
                ServerPacket::MessageChat(m) if m.chat_type == 0x58 => println!("[{name}] {}", m.text),
                _ => {}
            },
            Err(e) => {
                let text = format!("{e:#}");
                let timeout = text.contains("os error 11")
                    || text.contains("temporarily unavailable")
                    || text.contains("timed out");
                if !timeout && e.downcast_ref::<benilla_protocol::Unparsed>().is_none() {
                    return Err(e);
                }
            }
        }
        if !started {
            std::thread::sleep(Duration::from_secs(3));
            s.send_chat(".start")?;
            started = true;
        }
        let dt = last.elapsed().as_secs_f32();
        if dt < 0.25 {
            continue;
        }
        last = Instant::now();
        // Pick the nearest live enemy and walk to it.
        let alive = |g: &u64| enemies.get(g).is_some_and(|e| e.1);
        if !target.as_ref().is_some_and(alive) {
            target = enemies
                .iter()
                .filter(|(_, e)| e.1)
                .min_by(|a, b| dist(a.1 .0, pos).total_cmp(&dist(b.1 .0, pos)))
                .map(|(g, _)| *g);
            if let Some(t) = target {
                s.set_selection(t)?;
                s.attack_swing(t)?;
            }
        }
        if let Some(t) = target {
            let goal = enemies[&t].0;
            let d = dist(goal, pos);
            if d > 3.0 {
                let step = (7.0 * dt).min(d - 2.5);
                let f = (goal[1] - pos[1]).atan2(goal[0] - pos[0]);
                pos = [pos[0] + step * f.cos(), pos[1] + step * f.sin(), goal[2]];
                s.heartbeat(pos, f)?;
            }
        }
    }
}

fn is_enemy(guid: u64) -> bool {
    guid >> 48 == 0xF130 && (guid >> 24) & 0xFF_FFFF >= 2000
}

fn dist(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt()
}
