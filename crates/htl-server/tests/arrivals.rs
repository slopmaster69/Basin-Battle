//! Where heroes are set down below, and where its foes and finds stand, against each dungeon's own
//! map (read from the WoW 1.12.1 data): every lamp (where a hero comes down, wakes and is put back),
//! way out, pack, strongbox, note and secret stands on a floor, and none is in the hollow under a
//! building's floor. Shadowfang Keep's first lamp stood on the terrain 4.5 yards under its
//! gatehouse's floor until Nov 2026, and every hero came down under the floor; the land's own
//! floors can't tell, as that hidden terrain is a floor of its own.
//!
//! And every one of them can be walked to from the dungeon's first lamp on the server's own ground
//! (the floors the rules walk on), keeping to the floors a hero walks on in the game: never through
//! the terrain in the hollow under a building's floor, which the server's ground has as a floor of
//! its own, nor, in a dungeon walked indoors (its first lamp on a building's floor), out onto the
//! terrain outside its buildings. Shadowfang Keep's Fenrus, Wolf Master Nandos and four of its packs
//! stood on the hillside outside the keep's walls until Nov 2026, the server's ground reaching them
//! only under the courtyard's floor or out of the kitchen's passage and round the hill; one of
//! Arugal's voids hung 19 yards over the great hall, off the edge of his floor.
//!
//! Needs the game data: `WOW_DATA`, or the first line of `settings-windows.txt` where the server is
//! hosted. `cargo test --release -p htl-server --test arrivals -- --ignored --nocapture`. (Eighteen
//! more of Shadowfang Keep's places stood under the keep's floors, and its second lamp outside the
//! keep, until a second pass, Nov 2026.)

use std::path::PathBuf;

use htl_map::deadmines::Columns;
use htl_map::layers::{Layers, PIECE_SLOPE, STEP_UP};
use htl_survival::content::DungeonDef;

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
fn every_dungeons_places_stand_on_its_floor() {
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
        // The lamps and the ways out (where heroes are set down): on a floor, and under none.
        let mut spots: Vec<(String, [f32; 3])> = d.lamps.iter().map(|l| (format!("lamp {}", l.key), l.at)).collect();
        spots.push(("the exit".into(), d.exit));
        spots.extend(d.more_exits.iter().enumerate().map(|(k, e)| (format!("way out {k}"), *e)));
        for (what, at) in spots {
            checked += 1;
            if let Some(z) = cols.sealed_over(at) {
                wrong.push(format!("{}: {what} at {at:?} is under a floor at {z:.1}", d.name));
            } else if cols.stands_on(at).is_none() {
                wrong.push(format!("{}: {what} at {at:?} stands on nothing", d.name));
            }
        }
        // The rest of its places: its packs (those given a height; the others take the land's
        // highest floor), strongboxes, notes and secrets. None on the terrain under a building's
        // floor (a foe there fights from under the floor; a find there can't be reached).
        let mut rest: Vec<(String, [f32; 3])> = Vec::new();
        rest.extend(d.packs.iter().filter_map(|p| Some((format!("pack {} {}", p.mob, p.key), [p.at[0], p.at[1], p.z?]))));
        rest.extend(d.boxes.iter().map(|b| (format!("box {}", b.key), b.at)));
        rest.extend(d.notes.iter().map(|n| (format!("note {}", n.key), n.at)));
        rest.extend(d.doors.iter().map(|o| (format!("door {}", o.key), o.at)));
        for (what, at) in rest {
            checked += 1;
            let on_terrain = cols.terrain(at[0], at[1]).is_some_and(|t| (t - at[2]).abs() < 2.0);
            if let Some(z) = cols.sealed_over(at).filter(|_| on_terrain) {
                wrong.push(format!("{}: {what} at {at:?} is on the terrain under a floor at {z:.1}", d.name));
            }
        }
        // Every place, walked to from the first lamp on the server's ground, kept to the floors a
        // hero walks on.
        let ground = match htl_map::deadmines::load_instance_with(&data, &d.map_dir, 1.0, d.window.map(|w| (w[0], w[1]))) {
            Ok(l) => l.ground,
            Err(e) => {
                println!("{}: no ground ({e:#})", d.name);
                continue;
            }
        };
        // A dungeon whose first lamp stands on a building's floor is walked inside its buildings.
        let start = d.lamps[0].at;
        let indoors = cols.walked_indoors(&ground, start);
        println!("{}: walked {}", d.name, if indoors { "indoors" } else { "anywhere" });
        let reached = ground.reachable_avoiding(d.lamps[0].at, PIECE_SLOPE, &cols.off_limits(&ground, indoors));
        for (what, at, kind) in places(d, &ground) {
            checked += 1;
            if let Err(why) = walked_to(&ground, &reached, at, kind) {
                wrong.push(format!("{}: {what} at {at:?} {why}", d.name));
            }
        }
    }
    assert!(checked > 0, "no dungeon's map was read");
    println!("{checked} places checked");
    assert!(wrong.is_empty(), "placed where no floor holds them:\n{}", wrong.join("\n"));
}

/// What a place is, for how near its floor it must be.
#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Set down where it says (a lamp, a find, a way out): on its floor, as a walker's footing is.
    Spot,
    /// A pack's middle: its foes are set on the floor near it, if one is within 3 yards of it.
    Pack,
    /// A patrol's turn: walked to from the pack's floor, at whatever height its floor there has.
    Turn,
    /// A hazard: hits round it, so it's reached if open floor at its height within its radius is.
    Hazard(f32),
}

/// Every place in a dungeon a hero walks to or a foe stands on: its lamps and ways out, its packs
/// (at their height, or the land's highest floor without one), their other spots and their
/// patrols' turns, its strongboxes, notes, secrets and shortcuts' levers, and its hazards.
fn places(d: &DungeonDef, ground: &Layers) -> Vec<(String, [f32; 3], Kind)> {
    let mut out: Vec<(String, [f32; 3], Kind)> = d.lamps.iter().map(|l| (format!("lamp {}", l.key), l.at, Kind::Spot)).collect();
    out.push(("the exit".into(), d.exit, Kind::Spot));
    out.extend(d.more_exits.iter().enumerate().map(|(k, e)| (format!("way out {k}"), *e, Kind::Spot)));
    for p in &d.packs {
        let name = format!("pack {}{}", p.mob, if p.key.is_empty() { String::new() } else { format!(" {}", p.key) });
        for (k, at) in std::iter::once(p.at).chain(p.spots.iter().copied()).enumerate() {
            let z = p.z.unwrap_or_else(|| ground.height_near(at[0], at[1], 1.0e4));
            let what = if k == 0 { name.clone() } else { format!("{name}'s spot {k}") };
            out.push((what, [at[0], at[1], z], Kind::Pack));
        }
        let z = p.z.unwrap_or_else(|| ground.height_near(p.at[0], p.at[1], 1.0e4));
        out.extend(p.route.iter().enumerate().map(|(k, r)| (format!("{name}'s patrol, turn {k}"), [r[0], r[1], z], Kind::Turn)));
    }
    out.extend(d.boxes.iter().map(|b| (format!("box {}", b.key), b.at, Kind::Spot)));
    out.extend(d.notes.iter().map(|n| (format!("note {}", n.key), n.at, Kind::Spot)));
    out.extend(d.doors.iter().filter(|o| o.secret()).map(|o| (format!("secret {}", o.key), o.at, Kind::Spot)));
    out.extend(d.doors.iter().filter_map(|o| Some((format!("door {}'s lever", o.key), o.from?, Kind::Spot))));
    out.extend(d.hazards.iter().map(|h| (format!("hazard {} (for {})", h.name, h.boss), h.at, Kind::Hazard(h.radius))));
    out
}

/// Whether the floor the rules put something at `at` on (the highest no more than a step over it,
/// as a walker coming from its height finds it) holds it, is open, and was `reached`.
fn walked_to(ground: &Layers, reached: &[bool], at: [f32; 3], kind: Kind) -> Result<(), String> {
    let (i, j) = ground.cell_at(at[0], at[1]).ok_or("is off the ground")?;
    let open = |f: usize| reached[f] && !ground.floor(f).blocked;
    // Cells round (x, y) within `r` yards, nearest first.
    let round = |r: f32| {
        let n = r.ceil() as i32;
        let mut out: Vec<(usize, usize)> = Vec::new();
        for dj in -n..=n {
            for di in -n..=n {
                let (a, b) = (i as i32 + di, j as i32 + dj);
                if a >= 0 && b >= 0 && ((di * di + dj * dj) as f32).sqrt() * ground.cell() <= r.max(ground.cell() * 1.5) {
                    out.push((a as usize, b as usize));
                }
            }
        }
        out
    };
    if kind == Kind::Turn {
        // Any open floor walked to there, or a step from there.
        return round(1.5).into_iter().any(|(a, b)| ground.floor_range(a, b).any(open)).then_some(()).ok_or_else(|| "can't be walked to from the first lamp".into());
    }
    let r = ground.floor_range(i, j);
    let f = r.clone().rev().find(|&f| ground.floor(f).z <= at[2] + STEP_UP).or(r.clone().next()).ok_or("has no floor under it")?;
    let fl = ground.floor(f);
    let near = if kind == Kind::Pack { 3.0 } else { 1.5 };
    if (fl.z - at[2]).abs() > near {
        let how = if fl.z < at[2] { "floats" } else { "sinks" };
        return Err(format!("{how} {:.1} yd off its floor at {:.1}", (fl.z - at[2]).abs(), fl.z));
    }
    if let Kind::Hazard(radius) = kind {
        let at_height = |f: usize| (ground.floor(f).z - at[2]).abs() <= 1.5 && open(f);
        return round(radius).into_iter().any(|(a, b)| ground.floor_range(a, b).any(at_height)).then_some(()).ok_or_else(|| format!("has no floor walked to round it (its floor at {:.1})", fl.z));
    }
    if fl.blocked {
        return Err(format!("is walled in (its floor at {:.1})", fl.z));
    }
    if !reached[f] {
        return Err(format!("can't be walked to from the first lamp (its floor at {:.1})", fl.z));
    }
    Ok(())
}
