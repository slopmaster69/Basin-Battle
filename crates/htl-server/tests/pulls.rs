//! Who comes when a hero walks into each pack below: on each dungeon's own ground (read from the
//! WoW 1.12.1 data), a hero stands in one pack at a time for a while, and every creature that ends
//! up on them is listed by its pack. A pack that brings another it shouldn't (a boss joining the
//! fight before it, a caster's slope pulling the beach) shows here before anyone plays it.
//!
//! Needs the game data: `WOW_DATA`, or the first line of `settings-windows.txt` where the server is
//! hosted. `cargo test --release -p htl-server --test pulls -- --ignored --nocapture`, with
//! `PULLS=deadmines` (a dungeon's key) for one dungeon.

use std::collections::BTreeSet;
use std::path::PathBuf;

use htl_survival::mobs::MobState;
use htl_survival::{Moving, Survival};

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

/// Every creature back home and at rest, at full health.
fn settle(s: &mut Survival) {
    for m in s.mobs.values_mut() {
        m.pos = m.home;
        m.path.clear();
        m.health = m.max_health;
        m.state = MobState::Idle { wander_at: 1.0e9 };
    }
}

#[test]
#[ignore]
fn who_comes_when_a_hero_walks_into_each_pack() {
    let Some(data) = data_dir() else {
        eprintln!("No WoW data (WOW_DATA): skipped.");
        return;
    };
    let only = std::env::var("PULLS").ok();
    let content = htl_survival::Content::builtin();
    let keys: Vec<String> = content.dungeons().map(|d| d.key.clone()).filter(|k| only.as_ref().is_none_or(|o| o == k)).collect();
    for key in keys {
        let c = content.for_dungeon_of(&key);
        let d = c.dungeon.clone().unwrap();
        let loaded = match htl_map::deadmines::load_instance_with(&data, &d.map_dir, 1.0, d.window.map(|w| (w[0], w[1]))) {
            Ok(l) => l,
            Err(e) => {
                println!("{}: no ground ({e:#})", d.name);
                continue;
            }
        };
        let land = htl_server::survival::land::DeadminesLand::new(loaded);
        let mut s = Survival::new_dungeon(c.clone(), Box::new(land), 1, 1.0e6);
        let hero = s.join("pulls", "Pulls", None);
        println!("== {} ({} packs)", d.name, d.packs.len());
        let name_of = |s: &Survival, k: usize| {
            let p = &d.packs[k];
            let n = s.content.mob(&p.mob).map_or(p.mob.clone(), |i| s.content.mobs[i].name.clone());
            format!("{k}:{n}x{}{}", p.count, if p.key.is_empty() { String::new() } else { format!(" [{}]", p.key) })
        };
        for (k, p) in d.packs.iter().enumerate() {
            // Far off (not just above: a creature notices a hero overhead), everyone home, then in.
            s.move_player(hero, [p.at[0] + 5000.0, p.at[1] + 5000.0, 0.0], 0.0, Moving::default());
            settle(&mut s);
            let _ = s.step(0.05);
            settle(&mut s);
            let z = p.z.unwrap_or(0.0);
            s.move_player(hero, [p.at[0], p.at[1], z], 0.0, Moving::default());
            let mut came: BTreeSet<usize> = BTreeSet::new();
            for _ in 0..(12.0 / 0.05) as usize {
                if let Some(h) = s.players.get_mut(&hero) {
                    h.health = h.max_health;
                }
                let _ = s.step(0.05);
                for m in s.mobs.values() {
                    if m.state.target() == Some(hero) {
                        if let Some(j) = m.pack {
                            came.insert(j);
                        }
                    }
                }
            }
            let others: Vec<String> = came.iter().filter(|j| **j != k).map(|j| name_of(&s, *j)).collect();
            let own = if came.contains(&k) { "" } else { " (its own didn't come)" };
            println!(
                "{} at ({:.0}, {:.0}){own}: {}",
                name_of(&s, k),
                p.at[0],
                p.at[1],
                if others.is_empty() { "alone".to_string() } else { format!("brings {}", others.join(", ")) }
            );
        }
    }
}
