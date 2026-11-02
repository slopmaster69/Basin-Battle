//! Where each dungeon's foes stand, against the ground they could stand on: `dungeon_survey <Data>
//! <out dir> [key]` loads each dungeon's ground and writes, for laying packs out by hand:
//!
//! - `<key>-placement.png`: the ground walkable from the first lamp from above (north up, 3 px a
//!   yard, lighter is higher), each pack a dot (red a pack, orange an elite, magenta a boss, its
//!   size its count), each lamp yellow, and the ground farther than `GAP` yards from any foe
//!   tinted cyan (the empty stretches).
//! - `<key>-packs.tsv`: each pack: its place in the list, mob, count, where, the floor under it,
//!   how far it is walked from the first lamp (yards), and the nearest other pack.
//! - `<key>-gaps.tsv`: each empty stretch (cells farther than `GAP` from any foe, joined): its
//!   size (square yards), a walkable spot at its heart (x, y, z), how far that is walked from the
//!   first lamp, and whether it lies on the way to a boss (the walk from the first lamp to some
//!   boss passes within `ROUTE` yards of it).
//! - `<key>-seeds.tsv`: spots for new packs, spread evenly: again and again, the walkable spot
//!   farthest from every foe and every spot already picked (never within `LAMP_CLEAR` of a lamp,
//!   always within `NEAR_WAY` of a walk to a boss),
//!   until none is farther than `GAP`: where it is, how far it's walked, how far it was from
//!   everything, whether it's on the way to a boss, and the nearest pack's mob (the place's own
//!   kind of foe).
//!
//! It reads the rules as `Content::builtin()` (or `SURVIVAL_TOML`), the same as the server.
use std::collections::VecDeque;

use htl_server::survival::land::DeadminesLand;
use htl_survival::{dist2d, Content, Land};

const SC: u32 = 3;
/// The climb walked comfortably (rise per yard, about 40 degrees), as `secret_map`.
const COMFORT: f32 = 0.84;
/// Ground this far (yards) from every foe is an empty stretch.
const GAP: f32 = 22.0;
/// A stretch this near a walk to a boss is on the way.
const ROUTE: f32 = 8.0;
/// Stretches smaller than this (square yards) aren't worth a line.
const SMALLEST: usize = 60;
/// No new pack this near a lamp (resting there is safe).
const LAMP_CLEAR: f32 = 20.0;
/// Seeds only this near the walks to the bosses (the rooms and halls a run goes through, not a
/// roof or a cliff top the ground happens to reach).
const NEAR_WAY: f32 = 25.0;

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
    for d in c.dungeons().cloned().collect::<Vec<_>>() {
        if only.as_ref().is_some_and(|k| *k != d.key) {
            continue;
        }
        let window = d.window.map(|w| (w[0], w[1]));
        let loaded = htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?;
        let land = DeadminesLand::new(htl_map::deadmines::load_instance_with(data, &d.map_dir, 1.0, window)?);
        let g = &loaded.ground;
        let cell = g.cell();
        let start = d.lamps[0].at;
        let steps = g.reach_steps(start, COMFORT);
        let (w, h) = g.size();
        // Each pack where it stands: (index, mob, count, kind, at with its floor's height).
        let mut packs = Vec::new();
        for (k, p) in d.packs.iter().enumerate() {
            let Some(def) = c.mob(&p.mob) else { continue };
            let m = &c.mobs[def];
            if m.critter || m.inert() {
                continue;
            }
            let z = p.z.unwrap_or_else(|| land.height(p.at[0], p.at[1]));
            let z = land.height_near(p.at[0], p.at[1], z);
            let kind = if m.boss { "boss" } else if p.elite { "elite" } else if p.rare { "rare" } else { "pack" };
            packs.push((k, p.mob.clone(), p.count, kind, [p.at[0], p.at[1], z], p.hidden > 0.0 || !p.wake.is_empty()));
        }
        // The walked distance (yards) to a spot: its floor's steps from the first lamp.
        let walked = |at: [f32; 3]| -> Option<f32> {
            let (i, j) = g.cell_at(at[0], at[1])?;
            let f = g.floor_range(i, j).min_by(|a, b| (g.floor(*a).z - at[2]).abs().total_cmp(&(g.floor(*b).z - at[2]).abs()))?;
            (steps[f] != u32::MAX).then(|| steps[f] as f32 * cell)
        };
        // Every floor walkable from the first lamp: its cell, height, and nearest foe.
        let mut floors: Vec<(usize, usize, usize, f32, f32)> = Vec::new();
        for j in 0..h {
            for i in 0..w {
                for f in g.floor_range(i, j) {
                    let fl = g.floor(f);
                    if steps[f] == u32::MAX || fl.blocked || fl.water || fl.wet {
                        continue;
                    }
                    let [x, y] = g.cell_xy(i, j);
                    let near = packs.iter().filter(|p| (p.4[2] - fl.z).abs() < 8.0).map(|p| dist2d(p.4, [x, y, fl.z])).fold(f32::MAX, f32::min);
                    floors.push((i, j, f, fl.z, near));
                }
            }
        }
        // The walks to each boss (for whether a stretch is on the way).
        let mut routes: Vec<[f32; 3]> = Vec::new();
        for p in packs.iter().filter(|p| p.3 == "boss") {
            if let Some(path) = land.walk(start, p.4) {
                let mut prev = start;
                for q in path {
                    let n = (dist2d(prev, q) / 2.0).ceil().max(1.0) as usize;
                    for s in 0..=n {
                        let t = s as f32 / n as f32;
                        routes.push([prev[0] + (q[0] - prev[0]) * t, prev[1] + (q[1] - prev[1]) * t, prev[2] + (q[2] - prev[2]) * t]);
                    }
                    prev = q;
                }
            }
        }
        // The empty stretches: far floors joined cell to cell (and floor to near floor).
        let index: std::collections::HashMap<usize, usize> = floors.iter().enumerate().filter(|(_, f)| f.4 > GAP).map(|(n, f)| (f.2, n)).collect();
        let mut seen = vec![false; floors.len()];
        let mut gaps = Vec::new();
        for (n, f) in floors.iter().enumerate() {
            if f.4 <= GAP || seen[n] {
                continue;
            }
            let mut members = Vec::new();
            let mut queue = VecDeque::from([n]);
            seen[n] = true;
            while let Some(m) = queue.pop_front() {
                members.push(m);
                let (i, j, _, z, _) = floors[m];
                for (di, dj) in [(-1i32, 0i32), (1, 0), (0, -1), (0, 1)] {
                    let (ni, nj) = (i as i32 + di, j as i32 + dj);
                    if ni < 0 || nj < 0 || ni as usize >= w || nj as usize >= h {
                        continue;
                    }
                    for g2 in g.floor_range(ni as usize, nj as usize) {
                        if let Some(&o) = index.get(&g2) {
                            if !seen[o] && (floors[o].3 - z).abs() < 1.6 {
                                seen[o] = true;
                                queue.push_back(o);
                            }
                        }
                    }
                }
            }
            let area = (members.len() as f32 * cell * cell) as usize;
            if area < SMALLEST {
                continue;
            }
            // Its heart: the member farthest from any foe (the middle of the emptiness).
            let heart = *members.iter().max_by(|a, b| floors[**a].4.total_cmp(&floors[**b].4)).unwrap();
            let (i, j, f, z, far) = floors[heart];
            let [x, y] = g.cell_xy(i, j);
            let on_way = routes.iter().any(|r| (r[2] - z).abs() < 8.0 && dist2d(*r, [x, y, z]) <= ROUTE)
                || members.iter().any(|m| {
                    let (i, j, _, z, _) = floors[*m];
                    let [x, y] = g.cell_xy(i, j);
                    routes.iter().any(|r| (r[2] - z).abs() < 6.0 && dist2d(*r, [x, y, z]) <= 2.0)
                });
            gaps.push((area, [x, y, z], steps[f] as f32 * cell, far, on_way, members));
        }
        gaps.sort_by(|a, b| b.0.cmp(&a.0));
        // Seeds: farthest-point picks over every walkable floor (one in four cells, for speed).
        let mut near: Vec<f32> = floors.iter().map(|f| f.4).collect();
        let open: Vec<bool> = floors
            .iter()
            .map(|f| {
                let [x, y] = g.cell_xy(f.0, f.1);
                f.0 % 2 == 0
                    && f.1 % 2 == 0
                    && d.lamps.iter().all(|l| (l.at[2] - f.3).abs() > 10.0 || dist2d(l.at, [x, y, f.3]) > LAMP_CLEAR)
                    && routes.iter().any(|r| (r[2] - f.3).abs() < 6.0 && dist2d(*r, [x, y, f.3]) <= NEAR_WAY)
            })
            .collect();
        let mut seeds = Vec::new();
        loop {
            let Some((n, far)) = near.iter().enumerate().filter(|(n, _)| open[*n]).map(|(n, v)| (n, *v)).max_by(|a, b| a.1.total_cmp(&b.1)) else { break };
            if far <= GAP || seeds.len() >= 60 {
                break;
            }
            let (i, j, f, z, _) = floors[n];
            let [x, y] = g.cell_xy(i, j);
            let at = [x, y, z];
            let on_way = routes.iter().any(|r| (r[2] - z).abs() < 8.0 && dist2d(*r, at) <= ROUTE);
            let kind = packs.iter().filter(|p| p.3 == "pack" && (p.4[2] - z).abs() < 12.0).min_by(|a, b| dist2d(a.4, at).total_cmp(&dist2d(b.4, at))).map(|p| p.1.clone()).unwrap_or_default();
            seeds.push((at, steps[f] as f32 * cell, far.min(999.0), on_way, kind));
            for (m, fl) in floors.iter().enumerate() {
                if (fl.3 - z).abs() < 8.0 {
                    let [x2, y2] = g.cell_xy(fl.0, fl.1);
                    let dd = dist2d(at, [x2, y2, fl.3]);
                    if dd < near[m] {
                        near[m] = dd;
                    }
                }
            }
        }
        let mut rows = vec!["seed\tx\ty\tz\twalked\tfrom_any_foe\ton_the_way\tnearest_kind".to_string()];
        for (n, (at, walk, far, on_way, kind)) in seeds.iter().enumerate() {
            rows.push(format!("{}\t{:.1}\t{:.1}\t{:.1}\t{walk:.0}\t{far:.0}\t{}\t{kind}", n + 1, at[0], at[1], at[2], if *on_way { "yes" } else { "" }));
        }
        std::fs::write(out.join(format!("{}-seeds.tsv", d.key)), rows.join("\n") + "\n")?;
        // The tables.
        let mut rows = vec!["pack\tmob\tcount\tkind\tx\ty\tz\twalked\tnearest_pack\tnearest_mob\tlater".to_string()];
        for p in &packs {
            let (other, dist) = packs
                .iter()
                .filter(|o| o.0 != p.0 && (o.4[2] - p.4[2]).abs() < 8.0)
                .map(|o| (o.1.clone(), dist2d(o.4, p.4)))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .unwrap_or_default();
            let walk = walked(p.4).map_or("UNREACHABLE".to_string(), |y| format!("{y:.0}"));
            rows.push(format!("{}\t{}\t{}\t{}\t{:.1}\t{:.1}\t{:.1}\t{walk}\t{dist:.1}\t{other}\t{}", p.0, p.1, p.2, p.3, p.4[0], p.4[1], p.4[2], if p.5 { "yes" } else { "" }));
        }
        std::fs::write(out.join(format!("{}-packs.tsv", d.key)), rows.join("\n") + "\n")?;
        let mut rows = vec!["gap\tsquare_yards\tx\ty\tz\twalked\tfrom_any_foe\ton_the_way".to_string()];
        for (n, (area, at, walk, far, on_way, _)) in gaps.iter().enumerate() {
            rows.push(format!("{}\t{area}\t{:.1}\t{:.1}\t{:.1}\t{walk:.0}\t{far:.0}\t{}", n + 1, at[0], at[1], at[2], if *on_way { "yes" } else { "" }));
        }
        std::fs::write(out.join(format!("{}-gaps.tsv", d.key)), rows.join("\n") + "\n")?;
        // The picture: the walkable ground's box with a margin.
        let (mut i0, mut i1, mut j0, mut j1) = (w, 0usize, h, 0usize);
        for f in &floors {
            i0 = i0.min(f.0);
            i1 = i1.max(f.0);
            j0 = j0.min(f.1);
            j1 = j1.max(f.1);
        }
        if floors.is_empty() {
            eprintln!("{}: nothing walkable from its first lamp", d.key);
            continue;
        }
        let pad = 8;
        let (i0, j0) = (i0.saturating_sub(pad), j0.saturating_sub(pad));
        let (i1, j1) = ((i1 + pad).min(w - 1), (j1 + pad).min(h - 1));
        let (iw, ih) = ((j1 - j0 + 1) as u32 * SC, (i1 - i0 + 1) as u32 * SC);
        let px = |i: usize, j: usize| ((j1 - j) as u32 * SC, (i1 - i) as u32 * SC);
        let (zlo, zhi) = floors.iter().fold((f32::MAX, f32::MIN), |(a, b), f| (a.min(f.3), b.max(f.3)));
        let mut img = image::RgbImage::from_pixel(iw, ih, image::Rgb([12, 12, 14]));
        // The highest walkable floor of each cell, tinted cyan where it's empty of foes.
        let mut top: std::collections::HashMap<(usize, usize), (f32, f32)> = Default::default();
        for f in &floors {
            let e = top.entry((f.0, f.1)).or_insert((f.3, f.4));
            if f.3 > e.0 {
                *e = (f.3, f.4);
            }
        }
        for ((i, j), (z, near)) in &top {
            let t = (((z - zlo) / (zhi - zlo).max(1.0)) * 140.0 + 60.0) as u8;
            let c = if *near > GAP { [t / 5, t, t] } else { [t / 4, t, t / 3] };
            let (x, y) = px(*i, *j);
            for a in 0..SC {
                for b in 0..SC {
                    img.put_pixel(x + a, y + b, image::Rgb(c));
                }
            }
        }
        let mut dot = |at: [f32; 3], r: i32, c: [u8; 3]| {
            let Some((i, j)) = g.cell_at(at[0], at[1]) else { return };
            if i < i0 || i > i1 || j < j0 || j > j1 {
                return;
            }
            let (x, y) = px(i, j);
            let (cx, cy) = (x as i32 + SC as i32 / 2, y as i32 + SC as i32 / 2);
            for dx in -r..=r {
                for dy in -r..=r {
                    let (qx, qy) = (cx + dx, cy + dy);
                    if dx * dx + dy * dy <= r * r && qx >= 0 && qy >= 0 && (qx as u32) < iw && (qy as u32) < ih {
                        img.put_pixel(qx as u32, qy as u32, image::Rgb(c));
                    }
                }
            }
        };
        for r in &routes {
            dot(*r, 0, [120, 110, 40]);
        }
        for l in &d.lamps {
            dot(l.at, 5, [255, 220, 0]);
        }
        for p in &packs {
            let (r, c) = match p.3 {
                "boss" => (7, [230, 40, 230]),
                "elite" => (5, [255, 140, 0]),
                "rare" => (4, [255, 255, 255]),
                _ => (2 + p.2.min(5) as i32, if p.5 { [150, 30, 30] } else { [230, 40, 40] }),
            };
            dot(p.4, r, c);
        }
        for s in &seeds {
            dot(s.0, 4, if s.3 { [255, 255, 255] } else { [140, 140, 140] });
        }
        img.save(out.join(format!("{}-placement.png", d.key)))?;
        println!("{}: {} packs, {} walkable floors, {} empty stretches ({} on the way to a boss), {} seeds", d.key, packs.len(), floors.len(), gaps.len(), gaps.iter().filter(|g| g.4).count(), seeds.len());
    }
    Ok(())
}
