//! Every patrol below walks its route on its floor: on each dungeon's own ground (read from the
//! WoW 1.12.1 data), the world below runs with each patrol awake in turn and nobody in a fight, and
//! each patrolling pack walks its route there and back the way the rules walk it (each foe to its
//! own place round the turn, by the dungeon's straight walk or its planned way). Each foe is sent
//! round its whole route and comes to every place it's sent; none is put back for falling through
//! its floor or finding no way, then or when a rest sets every foe at its home.
//!
//! Nov 2026: a Defias Miner of the Deadmines' gallery patrol "fell through the floor" in real play
//! (its place round the turn was in the wall), and this found eight more patrols in five dungeons
//! that stood still at a turn (a turn up a stair looked for from below the landing, or more than a
//! planned walk away).
//!
//! Needs the game data: `WOW_DATA`, or the first line of `settings-windows.txt` where the server is
//! hosted. `cargo test --release -p htl-server --test patrols -- --ignored --nocapture`, with
//! `PATROLS=deadmines` (a dungeon's key) for one dungeon.

use std::collections::BTreeMap;
use std::path::PathBuf;

use htl_survival::mobs::MobState;
use htl_survival::{Event, Moving, Survival};

fn data_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("WOW_DATA") {
        return Some(PathBuf::from(d));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for f in ["settings-windows.txt", "../Basin-Battle/settings-windows.txt"] {
        if let Ok(t) = std::fs::read_to_string(root.join(f)) {
            if let Some(l) = t.lines().next() {
                return Some(PathBuf::from(l.trim()));
            }
        }
    }
    None
}

fn dist2d(a: [f32; 3], b: [f32; 3]) -> f32 {
    (a[0] - b[0]).hypot(a[1] - b[1])
}

#[test]
#[ignore]
fn every_patrol_walks_its_route_on_its_floor() {
    let Some(data) = data_dir() else {
        eprintln!("No WoW data (WOW_DATA): skipped.");
        return;
    };
    let only = std::env::var("PATROLS").ok();
    let content = htl_survival::Content::builtin();
    let keys: Vec<String> = content.dungeons().map(|d| d.key.clone()).filter(|k| only.as_ref().is_none_or(|o| o == k)).collect();
    let mut wrong = Vec::new();
    let mut walked = 0;
    for key in keys {
        let c = content.for_dungeon_of(&key);
        let d = c.dungeon.clone().unwrap();
        let load = || htl_map::deadmines::load_instance_with(&data, &d.map_dir, 1.0, d.window.map(|w| (w[0], w[1])));
        let patrols: Vec<usize> = (0..d.packs.len()).filter(|k| !d.packs[*k].route.is_empty()).collect();
        println!("== {} ({} patrols)", d.name, patrols.len());
        if patrols.is_empty() {
            continue;
        }
        // Two worlds, so the foes stand at different spots round their packs' middles.
        for seed in [1, 2] {
            let land = match load() {
                Ok(l) => htl_server::survival::land::DeadminesLand::new(l),
                Err(e) => {
                    println!("{}: no ground ({e:#})", d.name);
                    break;
                }
            };
            let mut s = Survival::new_dungeon(c.clone(), Box::new(land), seed, 1.0e6);
            let hero = s.join("patrols", "Patrols", None);
            // Each patrol's round: there and back, a walk and a wait at each stop (the walks
            // allowed as long again as the rules allow them).
            let rounds: Vec<f32> = patrols.iter().map(|k| {
                let p = &d.packs[*k];
                let mut stops = vec![p.at];
                stops.extend(p.route.iter().copied());
                let way: f32 = stops.windows(2).map(|w| (w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1])).sum();
                2.0 * (way * 1.3 * 2.0 + p.wait * stops.len() as f32)
            }).collect();
            let all: f32 = rounds.iter().sum::<f32>() + 30.0;
            // Each patrolling foe: the places it was sent to (and whether it came there).
            let mut sent: BTreeMap<u32, Vec<([f32; 3], bool)>> = BTreeMap::new();
            let dt = 0.1;
            for tick in 0..(all / dt) as usize {
                // A hero lying dead high over one patrol after another: each in turn is awake (a
                // creature far from every hero sleeps, and doesn't wander) and none has a foe.
                let mut t = tick as f32 * dt;
                let over = patrols.iter().zip(&rounds).find(|(_, r)| { t -= **r; t < 0.0 }).map_or(patrols[0], |(k, _)| *k);
                let p = &d.packs[over];
                s.move_player(hero, [p.at[0], p.at[1], 1000.0], 0.0, Moving::default());
                if let Some(h) = s.players.get_mut(&hero) {
                    h.health = 0;
                }
                for e in s.step(dt) {
                    if let Event::Log(t) = e {
                        if t.contains("put back") {
                            println!("{} (world {seed}): {t}", d.name);
                            wrong.push(format!("{} (world {seed}): {t}", d.name));
                        }
                    }
                }
                for (id, m) in &s.mobs {
                    if m.pack.is_none_or(|k| d.packs[k].route.is_empty()) || m.health == 0 {
                        continue;
                    }
                    let to = sent.entry(*id).or_default();
                    if to.last().is_none_or(|(h, _)| dist2d(*h, m.home) > 0.01) {
                        to.push((m.home, false));
                    }
                    let came = m.path.is_empty() && dist2d(m.pos, m.home) < 0.5 && (m.pos[2] - m.home[2]).abs() < 1.5;
                    if let Some(last) = to.last_mut() {
                        last.1 |= came;
                    }
                }
            }
            // A hero rests: every foe set at its home, wherever its patrol had it bound (a Defias
            // Miner's home was in the gallery's wall, Nov 2026, and it "fell through the floor").
            // (As the rules' rest does it: at home, its walk dropped, at rest.)
            let now = s.clock;
            for m in s.mobs.values_mut().filter(|m| m.health > 0 && m.pos != m.home) {
                m.pos = m.home;
                m.path.clear();
                m.state = MobState::Idle { wander_at: now + 5.0 };
            }
            for _ in 0..(2.0 / dt) as usize {
                for e in s.step(dt) {
                    if let Event::Log(t) = e {
                        if t.contains("put back") {
                            wrong.push(format!("{} (world {seed}), set at its home: {t}", d.name));
                        }
                    }
                }
            }
            for (id, to) in &sent {
                let m = &s.mobs[id];
                let k = m.pack.unwrap();
                let p = &d.packs[k];
                let name = &s.content.mobs[m.def].name;
                walked += to.len();
                // Sent round its whole route: to each turn (its own place round it) and home.
                for stop in std::iter::once(p.at).chain(p.route.iter().copied()) {
                    if !to.iter().any(|(h, _)| (h[0] - stop[0]).hypot(h[1] - stop[1]) < p.radius.max(1.8) + 1.0) {
                        wrong.push(format!("{} (world {seed}): {name} of pack {k} at {:?} was never sent to ({:.1}, {:.1}) in {all:.0} s", d.name, p.at, stop[0], stop[1]));
                    }
                }
                // Its last place it may still be on its way to.
                for (at, came) in &to[..to.len() - 1] {
                    if !came {
                        wrong.push(format!("{} (world {seed}): {name} of pack {k} at {:?} never came to ({:.1}, {:.1}, {:.1})", d.name, d.packs[k].at, at[0], at[1], at[2]));
                    }
                }
            }
        }
    }
    assert!(walked > 0, "no patrol was walked");
    println!("{walked} places walked to");
    assert!(wrong.is_empty(), "patrols off their floors:\n{}", wrong.join("\n"));
}
