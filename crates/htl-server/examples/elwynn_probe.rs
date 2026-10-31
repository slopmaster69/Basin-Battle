//! The whole of Elwynn for the rules: `elwynn_probe <Data> <survival.toml> <out.png>` loads the
//! zone (time and memory), lays the world out from that content, reports every pack, site, chest
//! and node that stands somewhere it shouldn't (in a wall, in deep water, off the land, on a
//! cliff), walks from the camp to every site and times it, ticks the world with a hero at each
//! site, and draws the land at a pixel for every 2 yards (north up): walls red, water blue, road
//! orange, trunks green, creatures magenta, objects cyan, sites yellow rings.
use std::time::Instant;

use htl_server::survival::land::ElwynnLand;
use htl_survival::{dist2d, Content, Moving, Survival};

fn peak_kb() -> u64 {
    std::fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| s.lines().find(|l| l.starts_with("VmHWM")).and_then(|l| l.split_whitespace().nth(1)?.parse().ok()))
        .unwrap_or(0)
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let t0 = Instant::now();
    let loaded = htl_map::elwynn::load(data)?;
    println!("loaded Elwynn in {:.1}s: {} trunks, peak {} MB", t0.elapsed().as_secs_f32(), loaded.trunks.len(), peak_kb() / 1024);
    // `caves`: each cave and mine under the hills: its model, its box, how much of it is walked
    // underground, and spots spread through it (x y z, with room round them) for what lives there.
    if args.get(4).map(String::as_str) == Some("caves") {
        for c in &loaded.caves {
            let g = &c.ground;
            let mut under = Vec::new();
            let (w, h) = g.size();
            for j in (0..h).step_by(2) {
                for i in (0..w).step_by(2) {
                    let [x, y] = g.cell_xy(i, j);
                    let top = htl_sim::Ground::height(&loaded.grid, x, y);
                    for f in g.floors_at(i, j) {
                        if f.blocked || f.water || f.z > top - 2.5 {
                            continue;
                        }
                        let room = [(1.5f32, 0.0f32), (-1.5, 0.0), (0.0, 1.5), (0.0, -1.5), (1.1, 1.1), (-1.1, -1.1), (1.1, -1.1), (-1.1, 1.1)]
                            .iter()
                            .all(|(dx, dy)| g.floor_near(x + dx, y + dy, f.z + 0.5).is_some_and(|n| !n.blocked && !n.water && (n.z - f.z).abs() < 1.2));
                        if room {
                            under.push([x, y, f.z]);
                        }
                    }
                }
            }
            let mut spots: Vec<[f32; 3]> = Vec::new();
            if let Some(first) = under.iter().min_by(|a, b| a[2].total_cmp(&b[2])) {
                spots.push(*first);
            }
            while spots.len() < 8 && !under.is_empty() {
                let next = under.iter().max_by(|a, b| {
                    let da = spots.iter().map(|s| dist2d(*s, **a) + (s[2] - a[2]).abs()).fold(f32::MAX, f32::min);
                    let db = spots.iter().map(|s| dist2d(*s, **b) + (s[2] - b[2]).abs()).fold(f32::MAX, f32::min);
                    da.total_cmp(&db)
                });
                match next {
                    Some(n) if spots.iter().all(|s| dist2d(*s, *n) > 6.0) => spots.push(*n),
                    _ => break,
                }
            }
            let name = c.model.rsplit('\\').next().unwrap_or(&c.model);
            println!("cave {name} box {:.0},{:.0} to {:.0},{:.0}: {} cells underground (2 yd)", c.min[0], c.min[1], c.max[0], c.max[1], under.len());
            // Given spots too (`caves x y z...`) in this cave's box.
            for t in args[5..].chunks(3) {
                if let (Some(x), Some(y), Some(z)) = (t.first().and_then(|v| v.parse::<f32>().ok()), t.get(1).and_then(|v| v.parse::<f32>().ok()), t.get(2).and_then(|v| v.parse::<f32>().ok())) {
                    if x >= c.min[0] && x <= c.max[0] && y >= c.min[1] && y <= c.max[1] {
                        spots.push([x, y, z]);
                    }
                }
            }
            for sp in &spots {
                // Whether a walker there can walk out to the hill's slope (the land outside).
                let seen = g.reachable(*sp, htl_map::layers::PIECE_SLOPE);
                let mut out = 0;
                for j in 0..h {
                    for i in 0..w {
                        let [x, y] = g.cell_xy(i, j);
                        let top = htl_sim::Ground::height(&loaded.grid, x, y);
                        for f in g.floor_range(i, j) {
                            let fl = g.floor(f);
                            if seen[f] && !fl.blocked && (fl.z - top).abs() < 1.0 {
                                out += 1;
                            }
                        }
                    }
                }
                let floor = g.floor_near(sp[0], sp[1], sp[2] + 0.5).map_or(f32::NAN, |f| f.z);
                println!("  spot {:.0} {:.0} z {:.1} (floor {floor:.1}, land {:.1}): {} cells of the land outside reached", sp[0], sp[1], sp[2], htl_sim::Ground::height(&loaded.grid, sp[0], sp[1]), out);
            }
        }
        return Ok(());
    }
    let land = ElwynnLand::new(loaded);
    // `near x y...`: a little map round each point (2 yd a cell, north up): # wall, ~ deep water,
    // ^ steep, . open, @ the point.
    if args.get(4).map(String::as_str) == Some("near") {
        use htl_survival::Land;
        for pair in args[5..].chunks(2) {
            let (x, y): (f32, f32) = (pair[0].parse()?, pair[1].parse()?);
            println!("near {x} {y} (z {:.1})", land.height(x, y));
            for j in (-10..=10).rev() {
                let mut line = String::new();
                for i in (-10..=10).rev() {
                    let (px, py) = (x + j as f32 * 2.0, y + i as f32 * 2.0);
                    let z = land.height(px, py);
                    let steep = [(1.5f32, 0.0f32), (0.0, 1.5)].iter().any(|(dx, dy)| (land.height(px + dx, py + dy) - z).abs() > 2.4);
                    line.push(if i == 0 && j == 0 { '@' } else if land.blocked(px, py) { '#' } else if land.water(px, py) { '~' } else if steep { '^' } else { '.' });
                }
                println!("{line}");
            }
        }
        return Ok(());
    }
    // `inside x y...`: the rooms of the building near each point (open cells walled in nearly
    // every way within 12 yards): the middle of the biggest (the spot farthest from any wall),
    // how many cells it has, and two more spots in it well apart (for a chest, a second foe).
    if args.get(4).map(String::as_str) == Some("inside") {
        use htl_survival::Land;
        let walled = |x: f32, y: f32| -> bool {
            if land.blocked(x, y) || land.water(x, y) {
                return false;
            }
            let z = land.height(x, y);
            (0..8)
                .filter(|k| {
                    let a = *k as f32 * std::f32::consts::FRAC_PI_4;
                    (1..=24).any(|i| {
                        let d = i as f32 * 0.5;
                        let (px, py) = (x + a.cos() * d, y + a.sin() * d);
                        land.blocked(px, py) || (land.height(px, py) - z).abs() > 2.0
                    })
                })
                .count()
                >= 7
        };
        let clear = |x: f32, y: f32| -> f32 {
            (1..=12).find(|i| {
                let d = *i as f32 * 0.5;
                (0..16).any(|k| {
                    let a = k as f32 / 16.0 * std::f32::consts::TAU;
                    land.blocked(x + a.cos() * d, y + a.sin() * d)
                })
            }).map_or(6.0, |i| i as f32 * 0.5)
        };
        for pair in args[5..].chunks(2) {
            let (x0, y0): (f32, f32) = (pair[0].parse()?, pair[1].parse()?);
            let mut cells = Vec::new();
            let mut gx = -30.0f32;
            while gx <= 30.0 {
                let mut gy = -30.0f32;
                while gy <= 30.0 {
                    let (x, y) = (x0 + gx, y0 + gy);
                    if walled(x, y) {
                        cells.push((x, y, clear(x, y)));
                    }
                    gy += 1.0;
                }
                gx += 1.0;
            }
            if cells.is_empty() {
                println!("inside {x0} {y0}: no room");
                continue;
            }
            cells.sort_by(|a, b| b.2.total_cmp(&a.2));
            let mid = cells[0];
            // Its room: the walled cells joined to the middle's.
            let key = |x: f32, y: f32| ((x - x0).round() as i32, (y - y0).round() as i32);
            let all: std::collections::HashMap<(i32, i32), (f32, f32, f32)> = cells.iter().map(|c| (key(c.0, c.1), *c)).collect();
            let mut room = std::collections::HashSet::new();
            let mut todo = vec![key(mid.0, mid.1)];
            while let Some(k) = todo.pop() {
                if !all.contains_key(&k) || !room.insert(k) {
                    continue;
                }
                for (di, dj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    todo.push((k.0 + di, k.1 + dj));
                }
            }
            let cells: Vec<(f32, f32, f32)> = room.iter().map(|k| all[k]).collect();
            let far = |from: &[(f32, f32)]| cells.iter().filter(|c| c.2 >= 1.5).max_by(|a, b| {
                let da = from.iter().map(|f| (a.0 - f.0).hypot(a.1 - f.1)).fold(f32::MAX, f32::min);
                let db = from.iter().map(|f| (b.0 - f.0).hypot(b.1 - f.1)).fold(f32::MAX, f32::min);
                da.total_cmp(&db)
            }).copied();
            let second = far(&[(mid.0, mid.1)]);
            let third = second.and_then(|s2| far(&[(mid.0, mid.1), (s2.0, s2.1)]));
            println!(
                "inside {x0} {y0}: {} cells; middle {:.0},{:.0} z {:.1} ({:.1} yd clear); spots {:?} {:?}",
                cells.len(), mid.0, mid.1, land.height(mid.0, mid.1), mid.2,
                second.map(|c| (c.0.round(), c.1.round(), c.2)), third.map(|c| (c.0.round(), c.1.round(), c.2))
            );
        }
        return Ok(());
    }
    let c = Content::load(std::path::Path::new(&args[2]))?;
    let camp_at = c.world.heart;
    // `build [x y r]...`: the building ground there (default the rules' own) at a cell every 3
    // yards: . a cooking fire goes there, r road, ^ steep, # in the way, ~ wet, o too close to
    // something, x outside; and how many cells take one.
    if args.get(4).map(String::as_str) == Some("build") {
        let mut tries: Vec<(f32, f32, f32)> = args[5..].chunks(3).filter_map(|t| Some((t.first()?.parse().ok()?, t.get(1)?.parse().ok()?, t.get(2)?.parse().ok()?))).collect();
        if tries.is_empty() {
            tries.push((c.world.build_center[0], c.world.build_center[1], c.world.build_radius));
        }
        for (x, y, r) in tries {
            let mut cc = c.clone();
            cc.world.build_center = [x, y];
            cc.world.build_radius = r;
            let s = Survival::new(cc.clone(), Box::new(ElwynnLand::new(htl_map::elwynn::load(data)?)), 7, 0.0);
            let fire = cc.station("cooking").unwrap();
            let (mut ok, mut all) = (0, 0);
            let n = (r / 3.0).ceil() as i32;
            println!("build ground at {x} {y} r {r}:");
            for j in (-n..=n).rev() {
                let mut line = String::new();
                for i in (-n..=n).rev() {
                    let (px, py) = (x + j as f32 * 3.0, y + i as f32 * 3.0);
                    let pos = [px, py, s.land().height(px, py)];
                    let ch = match s.site_problem(fire, pos, None) {
                        None => '.',
                        Some(p) if p.contains("Outside") => ' ',
                        Some(p) if p.contains("road") => 'r',
                        Some(p) if p.contains("steep") => '^',
                        Some(p) if p.contains("way") => '#',
                        Some(p) if p.contains("wet") => '~',
                        Some(_) => 'o',
                    };
                    if ch != ' ' {
                        all += 1;
                    }
                    if ch == '.' {
                        ok += 1;
                    }
                    line.push(ch);
                }
                println!("{line}");
            }
            println!("{ok} of {all} cells take a cooking fire");
        }
        return Ok(());
    }
    let t1 = Instant::now();
    let mut s = Survival::new(c.clone(), Box::new(land), 7, 0.0);
    println!("world laid out in {:.2}s: {} creatures, {} objects, peak {} MB", t1.elapsed().as_secs_f32(), s.mobs.len(), s.objs.len(), peak_kb() / 1024);
    let bad = |s: &Survival, x: f32, y: f32| -> Option<&'static str> {
        let l = s.land();
        if l.blocked(x, y) {
            return Some("blocked");
        }
        if l.water(x, y) {
            return Some("deep water");
        }
        let z = l.height(x, y);
        let steep = [(1.5f32, 0.0f32), (-1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().any(|(dx, dy)| (l.height(x + dx, y + dy) - z).abs() > 2.4);
        if steep {
            return Some("steep");
        }
        // Up on something (a roof): a sheer drop of more than 3 yards within 15 every way round.
        let drops = (0..8)
            .filter(|k| {
                let a = *k as f32 * std::f32::consts::FRAC_PI_4;
                let mut last = z;
                (1..=10).any(|i| {
                    let d = i as f32 * 1.5;
                    let h = l.height(x + a.cos() * d, y + a.sin() * d);
                    let sheer = last - h > 2.4;
                    last = h;
                    sheer && z - h > 3.0
                })
            })
            .count();
        (drops >= 6).then_some("up on a roof")
    };
    for (k, p) in c.packs.iter().enumerate() {
        if p.z.is_some() {
            continue;
        }
        if let Some(why) = bad(&s, p.at[0], p.at[1]) {
            println!("PACK {k} {} at {:?}: {why}", p.mob, p.at);
        }
        for (i, r) in p.spots.iter().enumerate() {
            if let Some(why) = bad(&s, r[0], r[1]) {
                println!("PACK {k} {} spot {i} {:?}: {why}", p.mob, r);
            }
        }
        for (i, r) in p.route.iter().enumerate() {
            if let Some(why) = bad(&s, r[0], r[1]) {
                println!("PACK {k} {} route {i} {:?}: {why}", p.mob, r);
            }
        }
    }
    for (id, m) in &s.mobs {
        // One set down in a cave (its pack's height given): judged on its own floor there.
        let pack_z = m.pack.and_then(|k| s.content.packs.get(k)).and_then(|p| p.z);
        if let Some(z) = pack_z {
            let l = s.land();
            if l.blocked_near(m.pos[0], m.pos[1], m.pos[2]) || (m.pos[2] - z).abs() > 3.0 {
                println!("CAVE MOB {id} {} at {:.0},{:.0},{:.1}: wanted z {z:.1}{}", c.mobs[m.def].key, m.pos[0], m.pos[1], m.pos[2], if l.blocked_near(m.pos[0], m.pos[1], m.pos[2]) { ", blocked" } else { "" });
            }
            continue;
        }
        if let Some(why) = bad(&s, m.pos[0], m.pos[1]) {
            println!("MOB {id} {} at {:.0},{:.0}: {why}", c.mobs[m.def].key, m.pos[0], m.pos[1]);
        }
    }
    let roamers = s.content.packs.len() - c.packs.len();
    println!("roaming groups laid: {roamers}; creatures in all: {}", s.mobs.len());
    for v in s.objects() {
        if let Some(why) = bad(&s, v.pos[0], v.pos[1]) {
            println!("OBJ {} at {:.0},{:.0},{:.0}: {why}", v.name, v.pos[0], v.pos[1], v.pos[2]);
        }
    }
    // How many of each thing stands in the land.
    let mut counts: std::collections::BTreeMap<String, usize> = Default::default();
    for v in s.objects() {
        *counts.entry(if v.name.is_empty() { format!("prop {}", v.display) } else { v.name.clone() }).or_default() += 1;
    }
    println!("objects: {counts:?}");
    // Each kind of node: how many stand of how many wanted, and how many of Elwynn's 300-yard
    // squares have one (of how many squares are Elwynn's at all).
    let sq = 300.0f32;
    let key = |x: f32, y: f32| (((x - htl_map::elwynn::MIN[0]) / sq) as i32, ((y - htl_map::elwynn::MIN[1]) / sq) as i32);
    let mut zone_sq = std::collections::BTreeSet::new();
    let mut x = htl_map::elwynn::MIN[0];
    while x < htl_map::elwynn::MAX[0] {
        let mut y = htl_map::elwynn::MIN[1];
        while y < htl_map::elwynn::MAX[1] {
            if s.land().in_zone(x, y) {
                zone_sq.insert(key(x, y));
            }
            y += 20.0;
        }
        x += 20.0;
    }
    // Nodes with no room round them (in or against a trunk or a wall).
    let tight: Vec<String> = s
        .objs
        .values()
        .filter(|o| matches!(o.kind, htl_survival::ObjKind::Node { .. }))
        .filter(|o| (0..8).any(|k| {
            let a = k as f32 * std::f32::consts::FRAC_PI_4;
            s.land().blocked(o.pos[0] + a.cos() * 1.2, o.pos[1] + a.sin() * 1.2)
        }) || s.land().blocked(o.pos[0], o.pos[1]))
        .map(|o| format!("{:.0},{:.0}", o.pos[0], o.pos[1]))
        .collect();
    println!("nodes against a trunk or wall: {} {:?}", tight.len(), tight.iter().take(12).collect::<Vec<_>>());
    for (def, n) in c.nodes.iter().enumerate() {
        let at: Vec<[f32; 3]> = s.objs.values().filter(|o| matches!(o.kind, htl_survival::ObjKind::Node { def: d, .. } if d == def)).map(|o| o.pos).collect();
        let squares: std::collections::BTreeSet<(i32, i32)> = at.iter().map(|p| key(p[0], p[1])).collect();
        let near_camp = at.iter().filter(|p| dist2d(**p, [camp_at[0], camp_at[1], 0.0]) < 300.0).count();
        println!("node {:<18} {:>3}/{:<3} in {:>2} of {} squares, {near_camp} within 300 yd of the camp", n.id(), at.len(), n.count, squares.len(), zone_sq.len());
    }
    // Every chest where it would stand (the secret ones too).
    for ch in &c.chests {
        if let Some(why) = bad(&s, ch.at[0], ch.at[1]) {
            println!("CHEST {} at {:?}: {why}", ch.key, ch.at);
        }
    }
    // Walks from the camp to every site, timed.
    let camp = c.world.heart;
    let from = [camp[0], camp[1], s.land().height(camp[0], camp[1])];
    for site in &c.sites {
        let to = [site.at[0], site.at[1], s.land().height(site.at[0], site.at[1])];
        if let Some(why) = bad(&s, site.at[0], site.at[1]) {
            println!("SITE {} at {:?}: {why}", site.key, site.at);
        }
        let t = Instant::now();
        let path = s.land().path(from, to);
        println!(
            "site {:<22} {:>6.0} yd  z {:>5.1}  walk {} in {:.1} ms",
            site.key,
            dist2d(from, to),
            to[2],
            path.as_ref().map_or("none".into(), |p| format!("{} legs", p.len())),
            t.elapsed().as_secs_f64() * 1000.0
        );
    }
    // A hero at each site in turn, the world ticking.
    let id = s.join("probe", "Probe", None);
    for site in &c.sites {
        let at = [site.at[0], site.at[1], s.land().height(site.at[0], site.at[1])];
        s.move_player(id, at, 0.0, Moving::default());
        let t = Instant::now();
        let mut worst = 0.0f64;
        // Creatures walking where none should: into a trunk or a wall, or up a sudden rise.
        let mut was: std::collections::BTreeMap<u32, [f32; 3]> = s.mobs.iter().map(|(id, m)| (*id, m.pos)).collect();
        let (mut stuck, mut climbs) = (std::collections::BTreeSet::new(), std::collections::BTreeSet::new());
        for _ in 0..200 {
            let t = Instant::now();
            let _ = s.step(0.05);
            worst = worst.max(t.elapsed().as_secs_f64());
            for (id, m) in &s.mobs {
                if m.health == 0 {
                    continue;
                }
                if let Some(p) = was.get(id) {
                    let (run, rise) = (dist2d(*p, m.pos), (m.pos[2] - p[2]).abs());
                    if run > 0.05 && rise > 0.5 && rise > 1.3 * run {
                        climbs.insert(*id);
                    }
                    if run > 0.05 && s.land().blocked(m.pos[0], m.pos[1]) && stuck.insert(*id) {
                        println!(
                            "  {} at {:.1},{:.1},{:.1} from {:.1},{:.1}: home {:.1},{:.1} (blocked {}), {:?}, path {:?}",
                            c.mobs[m.def].key, m.pos[0], m.pos[1], m.pos[2], p[0], p[1], m.home[0], m.home[1], s.land().blocked(m.home[0], m.home[1]), m.state, m.path
                        );
                    }
                }
                was.insert(*id, m.pos);
            }
        }
        if !stuck.is_empty() || !climbs.is_empty() {
            let name = |id: &u32| s.mobs.get(id).map_or("?".to_string(), |m| c.mobs[m.def].key.clone());
            println!("WALK at {}: into trunks/walls {:?}, up cliffs {:?}", site.key, stuck.iter().map(name).collect::<Vec<_>>(), climbs.iter().map(name).collect::<Vec<_>>());
        }
        if let Some((ns, f, t, found)) = ElwynnLand::slowest_walk() {
            if ns > 20_000_000 {
                println!("SLOW WALK at {}: {:.0} ms from {:.0},{:.0},{:.0} to {:.0},{:.0},{:.0} ({} yd), {}", site.key, ns as f64 / 1e6, f[0], f[1], f[2], t[0], t[1], t[2], dist2d(f, t).round(), if found { "found" } else { "none" });
            }
        }
        let near = s.mobs.values().filter(|m| m.health > 0 && dist2d(m.pos, at) < 60.0).count();
        println!("tick at {:<22} {:.3} ms mean, {:.2} ms worst, {near} creatures within 60 yd", site.key, t.elapsed().as_secs_f64() * 1000.0 / 200.0, worst * 1000.0);
    }
    // The map.
    let (x0, x1, y0, y1) = (htl_map::elwynn::MIN[0], htl_map::elwynn::MAX[0], htl_map::elwynn::MIN[1], htl_map::elwynn::MAX[1]);
    let sc = 0.5;
    let (w, h) = (((y1 - y0) * sc) as u32, ((x1 - x0) * sc) as u32);
    let mut img = image::RgbImage::new(w, h);
    let l = s.land();
    for j in 0..h {
        for i in 0..w {
            let (x, y) = (x1 - j as f32 / sc, y1 - i as f32 / sc);
            let z = l.height(x, y);
            let v = ((z + 10.0) / 160.0 * 200.0 + 30.0).clamp(0.0, 255.0) as u8;
            let mut col = [v / 2, v, v / 2];
            if l.road(x, y) > 0.4 {
                col = [230, 150, 40];
            }
            if l.water(x, y) {
                col = [40, 80, 220];
            }
            if l.blocked(x, y) {
                col = [220, 40, 40];
            }
            img.put_pixel(i, j, image::Rgb(col));
        }
    }
    let mut dot = |x: f32, y: f32, r: i32, col: [u8; 3]| {
        let (i, j) = (((y1 - y) * sc) as i32, ((x1 - x) * sc) as i32);
        for d in -r..=r {
            for e in -r..=r {
                if d * d + e * e <= r * r && i + d >= 0 && j + e >= 0 && ((i + d) as u32) < w && ((j + e) as u32) < h {
                    img.put_pixel((i + d) as u32, (j + e) as u32, image::Rgb(col));
                }
            }
        }
    };
    for v in s.objects() {
        dot(v.pos[0], v.pos[1], 1, [40, 230, 230]);
    }
    for m in s.mobs.values() {
        dot(m.pos[0], m.pos[1], 1, [230, 40, 230]);
    }
    for site in &c.sites {
        let r = (site.radius * sc) as i32;
        for k in 0..72 {
            let a = k as f32 / 72.0 * std::f32::consts::TAU;
            dot(site.at[0] + a.cos() * r as f32 / sc, site.at[1] + a.sin() * r as f32 / sc, 0, [255, 255, 0]);
        }
    }
    img.save(&args[3])?;
    println!("drawn {}x{}; peak {} MB", w, h, peak_kb() / 1024);
    Ok(())
}
