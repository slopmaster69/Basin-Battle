//! Where heroes are set down below, against each dungeon's own map (read from the WoW 1.12.1
//! data): every lamp (where a hero comes down, wakes and is put back) and every way out stands on
//! a floor, and none is in the hollow under a building's floor. Shadowfang Keep's first lamp stood
//! on the terrain 4.5 yards under its gatehouse's floor until Nov 2026, and every hero came down
//! under the floor; the land's own floors can't tell, as that hidden terrain is a floor of its own.
//!
//! Needs the game data: `WOW_DATA`, or the first line of `settings-windows.txt` where the server is
//! hosted. `cargo test --release -p htl-server --test arrivals -- --ignored --nocapture`. It also
//! lists (without failing) every other place of a dungeon's (its packs, notes, boxes and doors)
//! that's in such a hollow, for a content pass.

use std::path::PathBuf;

use htl_map::deadmines::Columns;

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

#[test]
#[ignore]
fn every_dungeons_lamps_and_exit_stand_on_its_floor() {
    let Some(data) = data_dir() else {
        eprintln!("No WoW data (WOW_DATA): skipped.");
        return;
    };
    let content = htl_survival::Content::builtin();
    let mut wrong = Vec::new();
    let mut checked = 0;
    for d in content.dungeons() {
        let cols = match Columns::load(&data, &d.map_dir) {
            Ok(c) => c,
            Err(e) => {
                println!("{}: no map ({e:#})", d.name);
                continue;
            }
        };
        let mut spots: Vec<(String, [f32; 3])> = d.lamps.iter().map(|l| (format!("lamp {}", l.key), l.at)).collect();
        spots.push(("the exit".into(), d.exit));
        spots.extend(d.more_exits.iter().enumerate().map(|(k, e)| (format!("way out {k}"), *e)));
        for (what, at) in spots {
            checked += 1;
            let under = cols.sealed_over(at);
            let on = cols.stands_on(at);
            println!("{} {what} at {at:?}: stands on {on:?}, sealed under {under:?}", d.name);
            if let Some(z) = under {
                wrong.push(format!("{}: {what} at {at:?} is under a floor at {z:.1}", d.name));
            } else if on.is_none() {
                wrong.push(format!("{}: {what} at {at:?} stands on nothing", d.name));
            }
        }
        // The rest, for a content pass: listed, not failed.
        let mut rest: Vec<(String, [f32; 3])> = Vec::new();
        rest.extend(d.packs.iter().filter_map(|p| Some((format!("pack {} {}", p.mob, p.key), [p.at[0], p.at[1], p.z?]))));
        rest.extend(d.notes.iter().map(|n| (format!("note {}", n.key), n.at)));
        rest.extend(d.boxes.iter().map(|b| (format!("box {}", b.key), b.at)));
        rest.extend(d.doors.iter().map(|o| (format!("door {}", o.key), o.at)));
        for (what, at) in rest {
            if let Some(z) = cols.sealed_over(at).filter(|_| cols.terrain(at[0], at[1]).is_some_and(|t| (t - at[2]).abs() < 2.0)) {
                println!("  (content pass) {} {what} at {at:?}: on the terrain under a floor at {z:.1}", d.name);
            }
        }
    }
    assert!(checked > 0, "no dungeon's map was read");
    assert!(wrong.is_empty(), "set down where no floor holds them:\n{}", wrong.join("\n"));
}
