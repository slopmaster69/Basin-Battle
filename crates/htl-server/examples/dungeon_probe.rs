//! Every dungeon on the real ground: `dungeon_probe <Data> [key]` loads each dungeon's instance
//! map and lays out its world from the rules, then reports every lamp, door, secret, strongbox,
//! note and foe (the floor under each, and whether it's walled in), walks from the first lamp to
//! each, and says whether each false wall seals what it hides (the only way in crosses it).
use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, Content, Survival};

fn crosses(a: [f32; 3], b: [f32; 3], c: [f32; 2], d: [f32; 2]) -> bool {
    let o = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (a, b) = ([a[0], a[1]], [b[0], b[1]]);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// Whether a leg of a walk passes the height of something at `z` (its ends either side, or near).
fn near_z(a: [f32; 3], b: [f32; 3], z: f32) -> bool {
    z >= a[2].min(b[2]) - 4.0 && z <= a[2].max(b[2]) + 4.0
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let only = args.get(2).cloned();
    let c = match std::env::var("SURVIVAL_TOML") {
        Ok(p) => Content::load(std::path::Path::new(&p))?,
        Err(_) => Content::builtin(),
    };
    for d in c.dungeons().cloned().collect::<Vec<_>>() {
        if only.as_ref().is_some_and(|k| *k != d.key) {
            continue;
        }
        println!("=== {} ({}, map {} {})", d.name, d.key, d.map, d.map_dir);
        let t0 = std::time::Instant::now();
        let window = d.window.map(|w| (w[0], w[1]));
        let land = DeadminesLand::new(htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?);
        let far = DeadminesLand::new(htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?);
        println!("loaded in {:.1}s", t0.elapsed().as_secs_f32());
        let s = Survival::new_dungeon(c.for_dungeon_of(&d.key), Box::new(land), 7, 0.0);
        let report = |what: &str, at: [f32; 3]| {
            let z = s.land().height_near(at[0], at[1], at[2]);
            let blocked = s.land().blocked_near(at[0], at[1], at[2]);
            let off = if (z - at[2]).abs() > 2.5 { format!("  OFF THE FLOOR by {:.1}", z - at[2]) } else { String::new() };
            println!("  {what:<40} at {:>7.1} {:>7.1} {:>6.1}  floor {z:>6.1}{}{off}", at[0], at[1], at[2], if blocked { "  WALLED" } else { "" });
        };
        let start = d.lamps[0].at;
        let walk = |what: &str, to: [f32; 3]| -> Option<Vec<[f32; 3]>> {
            let p = far.walk(start, to);
            match &p {
                Some(p) => {
                    let end = p.last().copied().unwrap_or(start);
                    let short = dist2d(end, to);
                    println!("      walk to {what:<34} {:>4} turns{}", p.len(), if short > 3.0 { format!("  STOPS {short:.1} SHORT") } else { String::new() });
                }
                None => println!("      walk to {what:<34} NO WAY"),
            }
            p
        };
        for l in &d.lamps {
            report(&format!("lamp {}", l.key), l.at);
            walk(&format!("lamp {}", l.key), l.at);
        }
        for dr in &d.doors {
            report(&format!("door {} ({})", dr.key, dr.kind), dr.at);
            if let Some(f) = dr.from {
                report(&format!("door {} from", dr.key), f);
            }
            if dr.secret() {
                walk(&format!("secret {}", dr.key), dr.at);
            }
        }
        for b in &d.boxes {
            report(&format!("box {}", b.key), b.at);
            walk(&format!("box {}", b.key), b.at);
        }
        for n in &d.notes {
            report(&format!("note {}", n.key), n.at);
            walk(&format!("note {}", n.key), n.at);
        }
        for p in &d.packs {
            let z = p.z.unwrap_or(s.land().height(p.at[0], p.at[1]));
            let at = [p.at[0], p.at[1], z];
            let what = format!("pack {} {}", p.mob, p.key);
            report(&what, at);
            walk(&what, at);
        }
        report("the way out", d.exit);
        walk("the way out", d.exit);
        // Each false wall: the way to what it hides crosses it.
        for dr in d.doors.iter().filter(|x| x.kind == "illusory") {
            let Some([a, b]) = dr.bars else { continue };
            for r in &dr.reveals {
                let at = d.boxes.iter().find(|x| &x.key == r).map(|x| x.at).or_else(|| d.packs.iter().find(|x| &x.key == r).map(|x| [x.at[0], x.at[1], x.z.unwrap_or(0.0)]));
                let Some(at) = at else { continue };
                match far.walk(start, at) {
                    Some(p) => {
                        let sealed = p.windows(2).any(|w| near_z(w[0], w[1], dr.at[2]) && crosses(w[0], w[1], a, b));
                        println!("  wall {} -> {r}: {}", dr.key, if sealed { "sealed (the way in crosses it)" } else { "NOT SEALED: a way round it" });
                    }
                    None => println!("  wall {} -> {r}: no way there at all", dr.key),
                }
            }
        }
        // Each gate and fog: the way to its boss crosses it.
        for dr in d.doors.iter().filter(|x| matches!(x.kind.as_str(), "gate" | "fog")) {
            let (Some([a, b]), Some(p)) = (dr.bars, d.packs.iter().find(|p| p.key == dr.boss)) else { continue };
            if dr.kind == "gate" {
                // A gate bars the way on, past its boss: which keyed packs lie beyond it.
                let beyond: Vec<String> = d
                    .packs
                    .iter()
                    .filter(|q| !q.key.is_empty())
                    .filter(|q| far.walk(start, [q.at[0], q.at[1], q.z.unwrap_or(0.0)]).is_some_and(|path| path.windows(2).any(|w| near_z(w[0], w[1], dr.at[2]) && crosses(w[0], w[1], a, b))))
                    .map(|q| q.key.clone())
                    .collect();
                println!("  gate {}: beyond it {:?}", dr.key, beyond);
                continue;
            }
            let past = [p.at[0], p.at[1], p.z.unwrap_or(0.0)];
            match far.walk(start, past) {
                Some(path) => {
                    let sealed = path.windows(2).any(|w| near_z(w[0], w[1], dr.at[2]) && crosses(w[0], w[1], a, b));
                    println!("  {} {}: {}", dr.kind, dr.key, if sealed { "crossed on the way" } else { "NOT CROSSED (a way round, or it's elsewhere)" });
                }
                None => println!("  {} {}: no way past at all", dr.kind, dr.key),
            }
        }
        let foes = s.mobs.len();
        println!("  {foes} foes laid out");
    }
    Ok(())
}
