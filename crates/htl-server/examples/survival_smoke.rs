//! Elwynn Survival end to end on the real land: `survival_smoke <Data>` starts the survival server
//! on the host's Elwynn (ports 13724 and 18086, a throwaway save) and plays it with benilla's own
//! protocol client: enters, gathers timber, banks it, builds a cooking fire, walks into the mine
//! yard and fights there, and reports what came back (every packet parsed by benilla).
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use benilla_protocol::messages::{Object, ObjectType, ServerPacket};
use benilla_protocol::WorldSession;

struct Seen {
    objects: BTreeMap<u64, (u32, [f32; 3])>,
    units: BTreeMap<u64, (u32, [f32; 3], bool)>,
    names: BTreeMap<String, usize>,
    lines: Vec<String>,
    chat: Vec<String>,
    blows: Vec<(bool, u32, u32)>,
}

fn pump(s: &mut WorldSession, me: u64, seen: &mut Seen, secs: f32) -> Result<()> {
    let end = Instant::now() + Duration::from_secs_f32(secs);
    while Instant::now() < end {
        let p = match s.recv() {
            Ok(p) => p,
            Err(e) => {
                let text = format!("{e:#}");
                let timeout = text.contains("temporarily unavailable") || text.contains("timed out") || text.contains("would block");
                if timeout {
                    continue;
                }
                return Err(e);
            }
        };
        *seen.names.entry(p.name()).or_default() += 1;
        match &p {
            ServerPacket::UpdateObject { objects, .. } => {
                for o in objects {
                    if let Object::Create { guid, object_type, movement, mask } = o {
                        let at = movement.position.map_or([0.0; 3], |(v, _)| [v.x, v.y, v.z]);
                        let f: Vec<(usize, u32)> = mask.raw_fields().map(|(i, v)| (i as usize, v)).collect();
                        let get = |i: usize| f.iter().find(|(k, _)| *k == i).map_or(0, |(_, v)| *v);
                        match object_type {
                            ObjectType::GameObject => {
                                seen.objects.insert(*guid, (get(8), at));
                            }
                            ObjectType::Unit => {
                                seen.units.insert(*guid, (get(3), at, get(22) > 0));
                            }
                            _ => {}
                        }
                    }
                    if let Object::Values { guid, mask } = o {
                        if mask.raw_fields().any(|(i, v)| i == 22 && v == 0) {
                            if let Some(u) = seen.units.get_mut(guid) {
                                u.2 = false;
                            }
                        }
                    }
                }
            }
            ServerPacket::DestroyObject { guid } => {
                seen.objects.remove(guid);
                seen.units.remove(guid);
            }
            ServerPacket::MonsterMove { guid, start, path, .. } => {
                if let Some(u) = seen.units.get_mut(guid) {
                    let v = path.last().copied().unwrap_or(*start);
                    u.1 = [v.x, v.y, v.z];
                }
            }
            ServerPacket::MessageChat(m) => {
                if let Some(l) = m.text.strip_prefix("ES\t") {
                    seen.lines.push(l.to_string());
                } else {
                    seen.chat.push(m.text.clone());
                }
            }
            ServerPacket::AttackerState(a) => seen.blows.push((a.attacker == me, a.victim_state, a.damage)),
            ServerPacket::Teleport { guid, counter, .. } => {
                s.teleport_ack(*guid, *counter)?;
            }
            _ => {}
        }
    }
    Ok(())
}

fn near(seen: &Seen, display: u32, at: [f32; 3]) -> Option<(u64, [f32; 3])> {
    seen.objects
        .iter()
        .filter(|(_, (d, _))| *d == display)
        .map(|(g, (_, p))| (*g, *p))
        .min_by(|a, b| ((a.1[0] - at[0]).hypot(a.1[1] - at[1])).total_cmp(&(b.1[0] - at[0]).hypot(b.1[1] - at[1])))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::PathBuf::from(&args[1]);
    let loaded = htl_map::elwynn::load(&data)?;
    let save = std::env::temp_dir().join("survival-smoke.toml");
    let _ = std::fs::remove_file(&save);
    let config = htl_server::Config {
        bind: "127.0.0.1".into(),
        login_port: 13724,
        survival: Some(htl_server::SurvivalSetup {
            world_port: 18086,
            realm_name: htl_server::SURVIVAL_REALM.into(),
            save: Some(save.clone()),
            content: htl_survival::Content::builtin(),
            land: Box::new(htl_server::survival::land::ElwynnLand::new(loaded)),
            below: Vec::new(),
            siege: None,
        }),
        ..htl_server::Config::default()
    };
    htl_server::start_games(config, None)?;
    let logon = benilla_protocol::logon("127.0.0.1:13724", "smoke", "hold", u32::from_be_bytes(*b"enUS")).context("logon")?;
    let realm = logon.realms.iter().find(|r| r.name == htl_server::SURVIVAL_REALM).context("no realm")?;
    let mut s = WorldSession::connect(&realm.address, "smoke", logon.session_key)?;
    let chars = s.char_enum()?;
    let me = chars[0].guid;
    let mut pos = [chars[0].position.x, chars[0].position.y, chars[0].position.z];
    s.player_login(me)?;
    s.set_read_timeout(Some(Duration::from_millis(50)))?;
    let mut seen = Seen { objects: BTreeMap::new(), units: BTreeMap::new(), names: BTreeMap::new(), lines: Vec::new(), chat: Vec::new(), blows: Vec::new() };
    pump(&mut s, me, &mut seen, 1.0)?;
    s.send_chat(".es hello 1")?;
    pump(&mut s, me, &mut seen, 0.5)?;
    println!("in the world at {pos:?}: {} objects, {} creatures", seen.objects.len(), seen.units.len());

    // Timber.
    let (pile, at) = near(&seen, 1248, pos).context("no timber")?;
    pos = [at[0] + 1.5, at[1], at[2]];
    s.heartbeat(pos, std::f32::consts::PI)?;
    pump(&mut s, me, &mut seen, 0.3)?;
    s.gameobj_use(pile)?;
    pump(&mut s, me, &mut seen, 2.0)?;
    // Bank.
    let (chest, at) = near(&seen, 10, pos).context("no chest")?;
    pos = [at[0] + 2.0, at[1], at[2]];
    s.heartbeat(pos, std::f32::consts::PI)?;
    pump(&mut s, me, &mut seen, 0.3)?;
    s.gameobj_use(chest)?;
    pump(&mut s, me, &mut seen, 0.5)?;
    // Take timber back out and build a cooking fire on the meadow.
    s.send_chat(".es take wood 8")?;
    pump(&mut s, me, &mut seen, 0.3)?;
    pos = [-9538.0, 122.0, pos[2]];
    s.heartbeat(pos, 0.0)?;
    pump(&mut s, me, &mut seen, 0.3)?;
    s.send_chat(".es build cooking")?;
    pump(&mut s, me, &mut seen, 0.4)?;
    s.send_chat(".es place")?;
    pump(&mut s, me, &mut seen, 0.5)?;
    println!("cooking fires: {}", seen.objects.values().filter(|(d, _)| *d == 192).count());

    // The mine yard: into the Warsong grunts, swinging at the nearest.
    pos = [-9870.0, 200.0, 15.0];
    s.heartbeat(pos, 0.0)?;
    let t0 = Instant::now();
    while t0.elapsed() < Duration::from_secs(20) {
        let foe = seen
            .units
            .values()
            .filter(|(e, _, alive)| *alive && *e >= 60_001)
            .map(|(_, p, _)| *p)
            .min_by(|a, b| ((a[0] - pos[0]).hypot(a[1] - pos[1])).total_cmp(&(b[0] - pos[0]).hypot(b[1] - pos[1])));
        if let Some(f) = foe {
            let face = (f[1] - pos[1]).atan2(f[0] - pos[0]);
            let d = (f[0] - pos[0]).hypot(f[1] - pos[1]);
            if d > 2.5 {
                pos = [f[0] - face.cos() * 2.0, f[1] - face.sin() * 2.0, f[2]];
            }
            s.heartbeat(pos, face)?;
            s.send_chat(".es in a")?;
        }
        pump(&mut s, me, &mut seen, 1.0)?;
    }
    let mine = seen.blows.iter().filter(|b| b.0 && b.2 > 0).count();
    let taken: Vec<&(bool, u32, u32)> = seen.blows.iter().filter(|b| !b.0).collect();
    println!("my blows that landed: {mine}; blows at me: {} (blocked {}, dodged {}, damage {})", taken.len(), taken.iter().filter(|b| b.1 == 5).count(), taken.iter().filter(|b| b.1 == 2).count(), taken.iter().map(|b| b.2).sum::<u32>());
    println!("deaths: {}", seen.lines.iter().filter(|l| l.starts_with("DE\t") && l != &"DE\t0").count());
    let mut verbs: BTreeMap<String, usize> = BTreeMap::new();
    for l in &seen.lines {
        *verbs.entry(l.split('\t').next().unwrap_or("").to_string()).or_default() += 1;
    }
    println!("add-on lines: {verbs:?}");
    println!("last HUD: {:?}", seen.lines.iter().rev().find(|l| l.starts_with("H\t")));
    println!("chat:");
    for c in seen.chat.iter().filter(|c| !c.starts_with("Welcome")) {
        println!("  {c}");
    }
    println!("packets: {:?}", seen.names);
    s.send_chat(".status")?;
    pump(&mut s, me, &mut seen, 0.5)?;
    println!("{}", seen.chat.last().cloned().unwrap_or_default());
    Ok(())
}
