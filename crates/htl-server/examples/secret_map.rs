//! A map of every dungeon's secrets, for checking them by hand: `secret_map <Data> <out dir>
//! [key]` loads each dungeon's ground and draws it from above (north up, 3 px a yard): what can
//! be walked from its first lamp green (lighter is higher), the rest grey, walls red, water blue.
//! On it, each secret (hidden spot, false wall), what a secret reveals, each note, rare and boss
//! with a secret form, with the server's own walk to it from the first lamp in yellow.
//! `<key>.png` and `<key>.tsv` (one marker a row: number, kind, name, x, y, z, pixel x, pixel y,
//! walk) go in the out dir; the labels are left to whoever reads the table.
//!
//! And `<key>-audit.tsv`: every lamp, shortcut, secret, box, note and creature's spot checked at
//! the height it stands at, not just the place on the map: `ok` (walkable from the first lamp
//! climbing no steeper than about 40 degrees), `steep` (only by the client's steepest, 50), or
//! `UNREACHABLE` (up on a ledge or rock: the floor under it is, the one it stands on isn't), and
//! for anything not `ok` the nearest spot that is: kind, name, x, y, z, status, new x, y, z.
use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, Content};

const SC: u32 = 3;
/// The climb walked comfortably (rise per yard, about 40 degrees).
const COMFORT: f32 = 0.84;
/// The client's steepest: a slope of 50 degrees (`GROUND_COS`).
const CLIENT: f32 = 1.19;
/// Cut off from this share of the ground around it or more: up a cliff (or down in a pit).
const CLIFF: f32 = 0.4;
/// What counts as a cliff between two spots: more than this many yards of height.
const CLIFF_RISE: f32 = 4.0;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let out = std::path::Path::new(&args[2]);
    let only = args.get(3).cloned();
    std::fs::create_dir_all(out)?;
    let c = match std::env::var("SURVIVAL_TOML") {
        Ok(p) => Content::load(std::path::Path::new(&p))?,
        Err(_) => Content::builtin(),
    };
    // `elwynn`: the open world's creatures instead, checked the same way on its ground.
    if only.as_deref() == Some("elwynn") {
        return elwynn(&c, data, out);
    }
    for d in c.dungeons().cloned().collect::<Vec<_>>() {
        if only.as_ref().is_some_and(|k| *k != d.key) {
            continue;
        }
        let window = d.window.map(|w| (w[0], w[1]));
        let loaded = htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?;
        let land = DeadminesLand::new(htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?);
        let g = &loaded.ground;
        let start = d.lamps[0].at;
        let reach = g.reachable(start, COMFORT);
        let reach_hard = g.reachable(start, CLIENT);
        audit(&c, &d, g, &land, &reach, &reach_hard, out)?;
        // What's worth drawing: the cells any marker or walkable floor is in, with a margin.
        let mut marks: Vec<(String, String, [f32; 3])> = Vec::new();
        for l in &d.lamps {
            marks.push(("lamp".into(), l.name.clone(), l.at));
        }
        let revealed: Vec<String> = d.doors.iter().filter(|x| x.secret()).flat_map(|x| x.reveals.clone()).collect();
        for x in d.doors.iter().filter(|x| x.secret()) {
            let kind = if x.kind == "illusory" { "false wall" } else { "hidden spot" };
            marks.push((kind.into(), x.name.clone(), x.at));
        }
        for b in &d.boxes {
            let kind = if revealed.contains(&b.key) { "revealed box" } else { "box" };
            marks.push((kind.into(), b.key.clone(), b.at));
        }
        for n in &d.notes {
            marks.push(("note".into(), n.name.clone(), n.at));
        }
        for p in &d.packs {
            let Some(def) = c.mob(&p.mob) else { continue };
            let m = &c.mobs[def];
            let z = p.z.unwrap_or(land_height(&land, p.at));
            let at = [p.at[0], p.at[1], z];
            if !m.then_needs.is_empty() {
                marks.push(("secret phase".into(), format!("{} (needs {})", m.name, m.then_needs), at));
            } else if p.rare {
                let kind = if revealed.contains(&p.key) { "revealed rare" } else { "rare" };
                marks.push((kind.into(), m.name.clone(), at));
            }
        }
        // The picture's box: every walkable floor and every marker.
        let (w, h) = g.size();
        let (mut i0, mut i1, mut j0, mut j1) = (w, 0usize, h, 0usize);
        for j in 0..h {
            for i in 0..w {
                if g.floor_range(i, j).any(|f| reach[f]) {
                    i0 = i0.min(i);
                    i1 = i1.max(i);
                    j0 = j0.min(j);
                    j1 = j1.max(j);
                }
            }
        }
        for (_, _, at) in &marks {
            if let Some((i, j)) = g.cell_at(at[0], at[1]) {
                i0 = i0.min(i);
                i1 = i1.max(i);
                j0 = j0.min(j);
                j1 = j1.max(j);
            }
        }
        let pad = 12;
        let (i0, j0) = (i0.saturating_sub(pad), j0.saturating_sub(pad));
        let (i1, j1) = ((i1 + pad).min(w - 1), (j1 + pad).min(h - 1));
        // North up: x (i) grows upwards, y (j) grows leftwards.
        let (iw, ih) = ((j1 - j0 + 1) as u32 * SC, (i1 - i0 + 1) as u32 * SC);
        let px = |i: usize, j: usize| ((j1 - j) as u32 * SC, (i1 - i) as u32 * SC);
        let (mut zlo, mut zhi) = (f32::MAX, f32::MIN);
        for j in j0..=j1 {
            for i in i0..=i1 {
                for f in g.floor_range(i, j).filter(|f| reach[*f]) {
                    zlo = zlo.min(g.floor(f).z);
                    zhi = zhi.max(g.floor(f).z);
                }
            }
        }
        let mut img = image::RgbImage::from_pixel(iw, ih, image::Rgb([12, 12, 14]));
        for j in j0..=j1 {
            for i in i0..=i1 {
                let fl: Vec<usize> = g.floor_range(i, j).collect();
                let walk = fl.iter().rev().find(|f| reach[**f]).copied();
                let steep = fl.iter().rev().find(|f| reach_hard[**f]).copied();
                let c = match (walk, steep) {
                    (Some(f), _) => {
                        let t = (((g.floor(f).z - zlo) / (zhi - zlo).max(1.0)) * 150.0 + 60.0) as u8;
                        [t / 4, t, t / 3]
                    }
                    // Only by the steepest climbing: olive.
                    (None, Some(_)) => [120, 120, 30],
                    (None, None) => match fl.last().map(|f| g.floor(*f)) {
                        Some(f) if f.water => [30, 60, 200],
                        Some(f) if f.blocked => [150, 40, 40],
                        Some(_) => [70, 70, 74],
                        None => [12, 12, 14],
                    },
                };
                let (x, y) = px(i, j);
                for a in 0..SC {
                    for b in 0..SC {
                        img.put_pixel(x + a, y + b, image::Rgb(c));
                    }
                }
            }
        }
        // A faint line every 10 yards.
        for j in j0..=j1 {
            for i in i0..=i1 {
                let [x, y] = g.cell_xy(i, j);
                if (x.round() as i32).rem_euclid(10) == 0 || (y.round() as i32).rem_euclid(10) == 0 {
                    let (cx, cy) = px(i, j);
                    let p = img.get_pixel_mut(cx + SC / 2, cy + SC / 2);
                    p.0 = [p.0[0].saturating_add(30), p.0[1].saturating_add(30), p.0[2].saturating_add(30)];
                }
            }
        }
        let to_px = |at: [f32; 3]| -> Option<(f32, f32)> {
            let (i, j) = g.cell_at(at[0], at[1])?;
            if i < i0 || i > i1 || j < j0 || j > j1 {
                return None;
            }
            let (x, y) = px(i, j);
            Some((x as f32 + SC as f32 / 2.0, y as f32 + SC as f32 / 2.0))
        };
        let mut rows = Vec::new();
        // Every walk, point by point (for laying out patrols and ambushes on walked ground):
        // the way to each marker, and to each pack.
        let mut paths: Vec<String> = Vec::new();
        for (k, p) in d.packs.iter().enumerate() {
            let z = p.z.unwrap_or_else(|| land_height(&land, p.at));
            if let Some(w) = land.walk(start, [p.at[0], p.at[1], z]) {
                let pts: Vec<String> = w.iter().map(|q| format!("{:.1},{:.1},{:.1}", q[0], q[1], q[2])).collect();
                paths.push(format!("pack{k}\t{}\t{}", p.mob, pts.join(" ")));
            }
        }
        for (n, (kind, name, at)) in marks.iter().enumerate() {
            let walked = if kind == "lamp" && n == 0 {
                "start".to_string()
            } else {
                match land.walk(start, *at) {
                    Some(p) => {
                        let end = p.last().copied().unwrap_or(start);
                        let short = dist2d(end, *at);
                        paths.push(format!("mark{}\t{kind}: {name}\t{}", n + 1, p.iter().map(|q| format!("{:.1},{:.1},{:.1}", q[0], q[1], q[2])).collect::<Vec<_>>().join(" ")));
                        // The route, yellow.
                        let mut prev = to_px(start);
                        for q in &p {
                            let cur = to_px(*q);
                            if let (Some(a), Some(b)) = (prev, cur) {
                                let steps = ((b.0 - a.0).abs().max((b.1 - a.1).abs()) as usize).max(1);
                                for s in 0..=steps {
                                    let t = s as f32 / steps as f32;
                                    let (x, y) = ((a.0 + (b.0 - a.0) * t) as u32, (a.1 + (b.1 - a.1) * t) as u32);
                                    if x < iw && y < ih {
                                        img.put_pixel(x, y, image::Rgb([255, 215, 0]));
                                    }
                                }
                            }
                            prev = cur;
                        }
                        if short > 3.0 {
                            format!("stops {short:.0} yd short")
                        } else {
                            let len: f32 = std::iter::once(start).chain(p.iter().copied()).collect::<Vec<_>>().windows(2).map(|w| dist2d(w[0], w[1])).sum();
                            format!("ok, {len:.0} yd walk")
                        }
                    }
                    None => "NO WAY".to_string(),
                }
            };
            let (x, y) = to_px(*at).unwrap_or((-1.0, -1.0));
            rows.push(format!("{}\t{kind}\t{name}\t{:.1}\t{:.1}\t{:.1}\t{x:.0}\t{y:.0}\t{walked}", n + 1, at[0], at[1], at[2]));
        }
        img.save(out.join(format!("{}.png", d.key)))?;
        std::fs::write(out.join(format!("{}-paths.tsv", d.key)), paths.join("\n") + "\n")?;
        std::fs::write(out.join(format!("{}.tsv", d.key)), format!("# {}\n{}\n", d.name, rows.join("\n")))?;
        println!("{}: {} markers, {}x{} px", d.name, rows.len(), iw, ih);
    }
    Ok(())
}

fn land_height(land: &DeadminesLand, at: [f32; 2]) -> f32 {
    use htl_survival::Land;
    land.height(at[0], at[1])
}

/// The floor something at `at` stands on: in its cell, the one nearest its height (from a yard
/// above it to two below).
fn standing(g: &htl_map::layers::Layers, at: [f32; 3]) -> Option<usize> {
    let (i, j) = g.cell_at(at[0], at[1])?;
    g.floor_range(i, j)
        .filter(|f| {
            let z = g.floor(*f).z;
            z <= at[2] + 1.0 && z >= at[2] - 2.0
        })
        .min_by(|a, b| (g.floor(*a).z - at[2]).abs().total_cmp(&(g.floor(*b).z - at[2]).abs()))
}

/// The nearest spot to `at` walkable comfortably, with room around it (every cell next to it
/// walkable too, at about its height), within 25 yards.
fn nearest_ok(g: &htl_map::layers::Layers, reach: &[bool], at: [f32; 3]) -> Option<[f32; 3]> {
    let (ci, cj) = g.cell_at(at[0], at[1])?;
    let (w, h) = g.size();
    let r = 25i32;
    let mut best: Option<(f32, [f32; 3])> = None;
    for dj in -r..=r {
        for di in -r..=r {
            let (i, j) = (ci as i32 + di, cj as i32 + dj);
            if i < 1 || j < 1 || i as usize >= w - 1 || j as usize >= h - 1 {
                continue;
            }
            let (i, j) = (i as usize, j as usize);
            for f in g.floor_range(i, j) {
                if !reach[f] || g.floor(f).blocked {
                    continue;
                }
                let z = g.floor(f).z;
                let roomy = (-1i32..=1).all(|a| {
                    (-1i32..=1).all(|b| {
                        g.floor_range((i as i32 + a) as usize, (j as i32 + b) as usize)
                            .any(|k| reach[k] && !g.floor(k).blocked && (g.floor(k).z - z).abs() < 1.0)
                    })
                });
                if !roomy {
                    continue;
                }
                let [x, y] = g.cell_xy(i, j);
                let score = dist2d([x, y, 0.0], [at[0], at[1], 0.0]) + 0.3 * (z - at[2]).abs();
                if best.is_none_or(|(s, _)| score < s) {
                    best = Some((score, [x, y, z]));
                }
            }
        }
    }
    best.map(|(_, p)| p)
}

/// Every spot heroes go to, checked at its height.
fn audit(
    c: &Content,
    d: &htl_survival::content::DungeonDef,
    g: &htl_map::layers::Layers,
    land: &DeadminesLand,
    reach: &[bool],
    reach_hard: &[bool],
    out: &std::path::Path,
) -> anyhow::Result<()> {
    let mut spots: Vec<(String, String, [f32; 3])> = Vec::new();
    for l in &d.lamps {
        spots.push(("lamp".into(), l.key.clone(), l.at));
    }
    for x in &d.doors {
        if x.secret() || x.kind == "shortcut" || x.kind == "lift" {
            spots.push((format!("door:{}", x.kind), x.key.clone(), x.at));
            if let Some(f) = x.from {
                spots.push((format!("door-from:{}", x.kind), x.key.clone(), f));
            }
        }
    }
    for b in &d.boxes {
        spots.push(("box".into(), b.key.clone(), b.at));
    }
    for n in &d.notes {
        spots.push(("note".into(), n.key.clone(), n.at));
    }
    spots.push(("exit".into(), "exit".into(), d.exit));
    for p in &d.packs {
        if c.mob(&p.mob).is_none() {
            continue;
        }
        // As the server sets it down: at its height, or (none given) on the highest floor there.
        let z = p.z.unwrap_or_else(|| land_height(land, p.at));
        spots.push(("pack".into(), format!("{}@{},{}", p.mob, p.at[0], p.at[1]), [p.at[0], p.at[1], z]));
    }
    let mut rows = Vec::new();
    let (mut bad, mut steep) = (0, 0);
    for (kind, name, at) in &spots {
        let f = standing(g, *at);
        let cliff = if kind == "pack" { cliff_layers(g, reach, *at) } else { None };
        let status = match f {
            None => "NO FLOOR",
            Some(f) if !reach[f] && reach_hard[f] => "steep",
            Some(f) if !reach[f] => "UNREACHABLE",
            _ if cliff.is_some_and(|c| c >= CLIFF) => "CLIFF",
            _ => "ok",
        };
        let fix = match status {
            "ok" => None,
            "CLIFF" => nearest_open_layers(g, reach, *at),
            _ => nearest_ok(g, reach, *at),
        };
        match status {
            "ok" => {}
            "steep" => steep += 1,
            _ => bad += 1,
        }
        let fixed = fix.map_or_else(|| "\t\t".to_string(), |p| format!("{:.1}\t{:.1}\t{:.1}", p[0], p[1], p[2]));
        let cl = cliff.map_or_else(|| "-".to_string(), |c| format!("{:.0}%", c * 100.0));
        rows.push(format!("{kind}\t{name}\t{:.1}\t{:.1}\t{:.1}\t{status}\t{fixed}\t{cl}", at[0], at[1], at[2]));
    }
    std::fs::write(out.join(format!("{}-audit.tsv", d.key)), format!("# {}\n{}\n", d.name, rows.join("\n")))?;
    println!("{}: {} spots, {bad} unreachable, {steep} steep", d.name, spots.len());
    Ok(())
}

/// Elwynn's creatures: each pack's spot (those on the open ground; a cave's have their height
/// given) walkable from the camp, as in a dungeon's audit, into `elwynn-audit.tsv`.
fn elwynn(c: &Content, data: &std::path::Path, out: &std::path::Path) -> anyhow::Result<()> {
    let loaded = htl_map::elwynn::load(data)?;
    let g = &loaded.grid;
    let (w, h) = g.size();
    let hs = g.heights();
    let spawn = c.world.spawn;
    let Some((si, sj)) = g.cell_at(spawn[0], spawn[1]) else { anyhow::bail!("the camp's off the ground") };
    let from = [spawn[0], spawn[1], hs[sj * w + si]];
    let reach = htl_map::reachable(g, from, COMFORT);
    let hard = htl_map::reachable(g, from, CLIENT);
    let mut rows = Vec::new();
    let (mut bad, mut steep) = (0, 0);
    for p in c.packs.iter().filter(|p| p.z.is_none()) {
        let Some((i, j)) = g.cell_at(p.at[0], p.at[1]) else { continue };
        let k = j * w + i;
        let cliff = if reach[k] { cliff_grid(g, &hard, p.at) } else { None };
        let status = if !reach[k] && hard[k] { "steep" } else if !reach[k] { "UNREACHABLE" } else if cliff.is_some_and(|c| c >= CLIFF) { "CLIFF" } else { "ok" };
        let mut fix = None;
        if status != "ok" {
            let r = (25.0 / g.cell()).ceil() as i32;
            let mut best: Option<(f32, [f32; 3])> = None;
            for dj in -r..=r {
                for di in -r..=r {
                    let (a, b) = (i as i32 + di, j as i32 + dj);
                    if a < 1 || b < 1 || a as usize >= w - 1 || b as usize >= h - 1 {
                        continue;
                    }
                    let (a, b) = (a as usize, b as usize);
                    let roomy = (-1i32..=1).all(|x| (-1i32..=1).all(|y| reach[(b as i32 + y) as usize * w + (a as i32 + x) as usize]));
                    if !roomy {
                        continue;
                    }
                    let [x, y] = g.cell_xy(a, b);
                    let d = dist2d([x, y, 0.0], [p.at[0], p.at[1], 0.0]);
                    if best.is_some_and(|(s, _)| d >= s) {
                        continue;
                    }
                    // Up a cliff: only somewhere it isn't.
                    if status == "CLIFF" && (di % 2 != 0 || dj % 2 != 0 || !cliff_grid(g, &hard, [x, y]).is_some_and(|c| c < 0.2)) {
                        continue;
                    }
                    best = Some((d, [x, y, hs[b * w + a]]));
                }
            }
            fix = best.map(|(_, q)| q);
        }
        match status {
            "ok" => {}
            "steep" => steep += 1,
            _ => bad += 1,
        }
        let fixed = fix.map_or_else(|| "\t\t".to_string(), |q| format!("{:.1}\t{:.1}\t{:.1}", q[0], q[1], q[2]));
        let cl = cliff.map_or_else(|| "-".to_string(), |c| format!("{:.0}%", c * 100.0));
        rows.push(format!("pack\t{}@{},{}\t{:.1}\t{:.1}\t{:.1}\t{status}\t{fixed}\t{cl}", p.mob, p.at[0], p.at[1], p.at[0], p.at[1], hs[k]));
    }
    // ELWYNN_LOOK="x,y;...": the heights around each spot (every 2 yards, 24 yards each way, north
    // up, minus its own), and whether each is walkable comfortably (lower case: only steeply).
    if let Ok(look) = std::env::var("ELWYNN_LOOK") {
        for spot in look.split(';').filter(|v| !v.is_empty()) {
            let v: Vec<f32> = spot.split(',').map(|n| n.trim().parse().unwrap_or(0.0)).collect();
            let Some((ci, cj)) = g.cell_at(v[0], v[1]) else { continue };
            let z0 = hs[cj * w + ci];
            println!("around {:.0},{:.0} (height {z0:.1}): rise in yards; '.' walkable, '~' only steeply, '#' not", v[0], v[1]);
            for dx in (-12i32..=12).rev() {
                let mut line = String::new();
                for dy in (-12i32..=12).rev() {
                    let (i, j) = (ci as i32 + dx * 2, cj as i32 + dy * 2);
                    if i < 0 || j < 0 || i as usize >= w || j as usize >= h {
                        line.push_str("    ");
                        continue;
                    }
                    let k = j as usize * w + i as usize;
                    let mark = if reach[k] { '.' } else if hard[k] { '~' } else { '#' };
                    line.push_str(&format!("{:>3.0}{mark}", hs[k] - z0));
                }
                println!("{line}");
            }
        }
    }
    // Each camp's packs: walked to from the camp's middle (its pack nearest the middle of them
    // all), none more than 1.6 times the straight way round and well above or below it ("FAR UP":
    // up a hill or a cliff from the rest of the camp), with the nearest spot that isn't.
    let mut camps: Vec<&str> = c.packs.iter().filter(|p| !p.camp.is_empty() && p.z.is_none()).map(|p| p.camp.as_str()).collect();
    camps.sort_unstable();
    camps.dedup();
    for camp in camps {
        let packs: Vec<_> = c.packs.iter().filter(|p| p.camp == camp && p.z.is_none()).collect();
        let n = packs.len() as f32;
        let mid = [packs.iter().map(|p| p.at[0]).sum::<f32>() / n, packs.iter().map(|p| p.at[1]).sum::<f32>() / n];
        let anchor = packs.iter().min_by(|a, b| dist2d([a.at[0], a.at[1], 0.0], [mid[0], mid[1], 0.0]).total_cmp(&dist2d([b.at[0], b.at[1], 0.0], [mid[0], mid[1], 0.0]))).unwrap().at;
        let steps = grid_steps(g, &hard, anchor, 160.0);
        let za = g.cell_at(anchor[0], anchor[1]).map_or(0.0, |(i, j)| hs[j * w + i]);
        for p in &packs {
            let Some((i, j)) = g.cell_at(p.at[0], p.at[1]) else { continue };
            let straight = dist2d([p.at[0], p.at[1], 0.0], [anchor[0], anchor[1], 0.0]);
            let dz = hs[j * w + i] - za;
            let walked = steps.get(&(i, j)).copied();
            let far = straight > 5.0 && dz.abs() > 8.0 && walked.is_none_or(|d| d > 1.6 * straight);
            if !far {
                continue;
            }
            // The nearest spot to it walked to from the middle no more than 1.6 times the straight
            // way, and no more than 4 yards above or below the middle.
            let mut best: Option<(f32, [f32; 3])> = None;
            for ((a, b), d) in &steps {
                let [x, y] = g.cell_xy(*a, *b);
                let z = hs[b * w + a];
                let st = dist2d([x, y, 0.0], [anchor[0], anchor[1], 0.0]);
                if *d > 1.6 * st.max(3.0) || (z - za).abs() > 4.0 || !reach[b * w + a] {
                    continue;
                }
                let off = dist2d([x, y, 0.0], [p.at[0], p.at[1], 0.0]);
                if best.is_none_or(|(s, _)| off < s) {
                    best = Some((off, [x, y, z]));
                }
            }
            let fixed = best.map_or_else(|| "\t\t".to_string(), |(_, q)| format!("{:.1}\t{:.1}\t{:.1}", q[0], q[1], q[2]));
            let walk = walked.map_or_else(|| "no way".to_string(), |d| format!("{d:.0} yd"));
            rows.push(format!("pack\t{}@{},{}\t{:.1}\t{:.1}\t{:.1}\tFAR UP\t{fixed}\t{camp}: {dz:+.0} yd, {straight:.0} yd off, walked {walk}", p.mob, p.at[0], p.at[1], p.at[0], p.at[1], hs[j * w + i]));
        }
    }
    std::fs::write(out.join("elwynn-audit.tsv"), format!("# Elwynn\n{}\n", rows.join("\n")))?;
    println!("Elwynn: {} packs, {bad} unreachable, {steep} steep", rows.len());
    Ok(())
}

/// How cut off a spot is from the ground around it: of the walkable floors 12 and 18 yards off
/// in 16 directions (each the one nearest the spot's height), the share a walker only reaches by
/// going more than 2.5 times the straight way round (a cliff, a ledge, a drop between). None:
/// too few to tell.
fn cliff_layers(g: &htl_map::layers::Layers, reach: &[bool], at: [f32; 3]) -> Option<f32> {
    let f = standing(g, at)?;
    let z = g.floor(f).z;
    let steps = g.reach_steps([at[0], at[1], z], COMFORT);
    let cell = {
        let a = g.cell_xy(0, 0);
        let b = g.cell_xy(1, 0);
        (b[0] - a[0]).abs()
    };
    let (mut near, mut far) = (0, 0);
    for r in [12.0f32, 18.0] {
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let (x, y) = (at[0] + a.cos() * r, at[1] + a.sin() * r);
            let Some((i, j)) = g.cell_at(x, y) else { continue };
            let Some(best) = g.floor_range(i, j).filter(|q| reach[*q]).min_by(|p, q| (g.floor(*p).z - z).abs().total_cmp(&(g.floor(*q).z - z).abs())) else { continue };
            let walked = steps[best];
            if walked != u32::MAX && walked as f32 * cell <= 2.5 * r {
                near += 1;
            } else if (g.floor(best).z - z).abs() > CLIFF_RISE {
                far += 1;
            }
            // (Level with it but the long way round: a wall between, not a cliff.)
        }
    }
    (near + far >= 8).then(|| far as f32 / (near + far) as f32)
}

/// The same on open ground (one height a spot): `walk` is where a walker can be at all.
fn cliff_grid(g: &htl_map::HeightGrid, walk: &[bool], at: [f32; 2]) -> Option<f32> {
    let (w, h) = g.size();
    let hs = g.heights();
    let cell = g.cell();
    let (ci, cj) = g.cell_at(at[0], at[1])?;
    let r = (40.0 / cell) as i32;
    // Steps from the spot, out to 40 yards, no steeper than comfortable.
    let mut dist: std::collections::HashMap<(usize, usize), u32> = std::collections::HashMap::new();
    let mut queue = std::collections::VecDeque::from([(ci, cj)]);
    dist.insert((ci, cj), 0);
    while let Some((i, j)) = queue.pop_front() {
        let d = dist[&(i, j)];
        for (di, dj, len) in [(1i32, 0i32, 1.0f32), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0), (1, 1, 1.414), (1, -1, 1.414), (-1, 1, 1.414), (-1, -1, 1.414)] {
            let (a, b) = (i as i32 + di, j as i32 + dj);
            if a < 0 || b < 0 || a as usize >= w || b as usize >= h || (a - ci as i32).abs() > r || (b - cj as i32).abs() > r {
                continue;
            }
            let (a, b) = (a as usize, b as usize);
            if dist.contains_key(&(a, b)) || !walk[b * w + a] {
                continue;
            }
            if (hs[b * w + a] - hs[j * w + i]).abs() > COMFORT * len * cell + 0.1 {
                continue;
            }
            dist.insert((a, b), d + 1);
            queue.push_back((a, b));
        }
    }
    let (mut near, mut far) = (0, 0);
    for rr in [12.0f32, 18.0] {
        for k in 0..16 {
            let a = k as f32 * std::f32::consts::TAU / 16.0;
            let Some((i, j)) = g.cell_at(at[0] + a.cos() * rr, at[1] + a.sin() * rr) else { continue };
            if !walk[j * w + i] {
                continue;
            }
            match dist.get(&(i, j)) {
                Some(d) if *d as f32 * cell <= 2.5 * rr => near += 1,
                // Level with it but the long way round: a wall between (a house's), not a cliff.
                _ if (hs[j * w + i] - hs[cj * w + ci]).abs() <= CLIFF_RISE => {}
                _ => far += 1,
            }
        }
    }
    (near + far >= 8).then(|| far as f32 / (near + far) as f32)
}

/// The nearest spot to `at` (within 25 yards) not cut off from the ground around it.
fn nearest_open_layers(g: &htl_map::layers::Layers, reach: &[bool], at: [f32; 3]) -> Option<[f32; 3]> {
    let (ci, cj) = g.cell_at(at[0], at[1])?;
    let (w, h) = g.size();
    let mut cands: Vec<(f32, [f32; 3])> = Vec::new();
    for dj in (-25i32..=25).step_by(2) {
        for di in (-25i32..=25).step_by(2) {
            let (i, j) = (ci as i32 + di, cj as i32 + dj);
            if i < 1 || j < 1 || i as usize >= w - 1 || j as usize >= h - 1 {
                continue;
            }
            let (i, j) = (i as usize, j as usize);
            for f in g.floor_range(i, j).filter(|f| reach[*f] && !g.floor(*f).blocked) {
                let [x, y] = g.cell_xy(i, j);
                let z = g.floor(f).z;
                cands.push((dist2d([x, y, 0.0], at) + 0.3 * (z - at[2]).abs(), [x, y, z]));
            }
        }
    }
    cands.sort_by(|a, b| a.0.total_cmp(&b.0));
    cands.into_iter().map(|(_, p)| p).find(|p| roomy(g, reach, *p) && cliff_layers(g, reach, *p).is_some_and(|c| c < 0.2))
}

/// Room to stand at `p`: every cell next to it walkable too, at about its height.
fn roomy(g: &htl_map::layers::Layers, reach: &[bool], p: [f32; 3]) -> bool {
    let Some((i, j)) = g.cell_at(p[0], p[1]) else { return false };
    let (w, h) = g.size();
    if i < 1 || j < 1 || i >= w - 1 || j >= h - 1 {
        return false;
    }
    (-1i32..=1).all(|a| {
        (-1i32..=1).all(|b| {
            g.floor_range((i as i32 + a) as usize, (j as i32 + b) as usize)
                .any(|k| reach[k] && !g.floor(k).blocked && (g.floor(k).z - p[2]).abs() < 1.0)
        })
    })
}

/// Steps from `from` over open ground, out to `radius` yards, no steeper than comfortable.
fn grid_steps(g: &htl_map::HeightGrid, walk: &[bool], from: [f32; 2], radius: f32) -> std::collections::HashMap<(usize, usize), f32> {
    let (w, h) = g.size();
    let hs = g.heights();
    let cell = g.cell();
    let mut dist: std::collections::HashMap<(usize, usize), f32> = std::collections::HashMap::new();
    let Some((ci, cj)) = g.cell_at(from[0], from[1]) else { return dist };
    let r = (radius / cell) as i32;
    // Dijkstra over the cells (diagonals cost their length).
    let mut heap = std::collections::BinaryHeap::new();
    dist.insert((ci, cj), 0.0);
    heap.push((std::cmp::Reverse(0u32), ci, cj));
    while let Some((std::cmp::Reverse(dq), i, j)) = heap.pop() {
        let d = dq as f32 / 100.0;
        if dist.get(&(i, j)).is_some_and(|x| *x < d - 1e-3) {
            continue;
        }
        for (di, dj, len) in [(1i32, 0i32, 1.0f32), (-1, 0, 1.0), (0, 1, 1.0), (0, -1, 1.0), (1, 1, 1.414), (1, -1, 1.414), (-1, 1, 1.414), (-1, -1, 1.414)] {
            let (a, b) = (i as i32 + di, j as i32 + dj);
            if a < 0 || b < 0 || a as usize >= w || b as usize >= h || (a - ci as i32).abs() > r || (b - cj as i32).abs() > r {
                continue;
            }
            let (a, b) = (a as usize, b as usize);
            if !walk[b * w + a] || (hs[b * w + a] - hs[j * w + i]).abs() > COMFORT * len * cell + 0.1 {
                continue;
            }
            let nd = d + len * cell;
            if dist.get(&(a, b)).is_none_or(|x| nd < *x - 1e-3) {
                dist.insert((a, b), nd);
                heap.push((std::cmp::Reverse((nd * 100.0) as u32), a, b));
            }
        }
    }
    dist
}
