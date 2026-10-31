//! Elwynn Survival on the real land: `survival_probe <Data> <out.png>` loads Elwynn, lays out the
//! world, and reports what the rules made of it (the camp, nodes, packs, building ground, a walk to
//! the mine, how long a tick takes with creatures fighting), and draws it (2 px a yard, north up):
//! walls red, water blue, road orange, trunks green, timber brown, copper cyan, creatures magenta,
//! the camp yellow, good building spots light green, the walk white.
use std::time::Instant;

use htl_server::survival::land::ElwynnLand;
use htl_survival::{dist2d, Content, Input, ObjKind, Survival};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let t0 = Instant::now();
    let loaded = htl_map::elwynn::load(data)?;
    println!("loaded Elwynn in {:.1}s: {} trunks", t0.elapsed().as_secs_f32(), loaded.trunks.len());
    let land = ElwynnLand::new(loaded);
    // A close look at the mine yard: 6 px a yard, walls red, trunks green, steep (slope > 1.2
    // to a neighbour) dark, heights as grey; the ore points cyan.
    {
        let (x0, x1, y0, y1) = (-9895.0f32, -9785.0f32, 150.0f32, 260.0f32);
        let sc = 6.0;
        let (w, h) = (((y1 - y0) * sc) as u32, ((x1 - x0) * sc) as u32);
        let mut img = image::RgbImage::new(w, h);
        for j in 0..h {
            for i in 0..w {
                let (x, y) = (x1 - j as f32 / sc, y1 - i as f32 / sc);
                let z = land.height_at(x, y);
                let v = ((z - 5.0) / 60.0 * 200.0 + 40.0).clamp(0.0, 255.0) as u8;
                let steep = [(1.0f32, 0.0f32), (0.0, 1.0)].iter().any(|(dx, dy)| (land.height_at(x + dx, y + dy) - z).abs() > 1.2);
                let mut c = [v, v, v];
                if steep {
                    c = [v / 3, v / 3, v / 2];
                }
                if land.is_wall(x, y) {
                    c = [220, 40, 40];
                }
                if land.is_trunk(x, y) {
                    c = [30, 200, 40];
                }
                if (x.round() as i32).rem_euclid(10) == 0 && (y.round() as i32).rem_euclid(10) == 0 {
                    c = [255, 255, 0];
                }
                img.put_pixel(i, j, image::Rgb(c));
            }
        }
        for p in &Content::builtin().nodes.iter().find(|n| n.item == "ore").unwrap().points {
            let (i, j) = (((y1 - p[1]) * sc) as i32, ((x1 - p[0]) * sc) as i32);
            for d in -3..=3 {
                for e in -3..=3 {
                    if i + d >= 0 && j + e >= 0 && ((i + d) as u32) < w && ((j + e) as u32) < h {
                        img.put_pixel((i + d) as u32, (j + e) as u32, image::Rgb([40, 230, 230]));
                    }
                }
            }
        }
        img.save(args[2].replace(".png", "-mine.png"))?;
    }
    let c = Content::builtin();
    let t1 = Instant::now();
    let mut s = Survival::new(c.clone(), Box::new(land), 7, 0.0);
    println!("world laid out in {:.2}s", t1.elapsed().as_secs_f32());
    let sp = c.world.spawn;
    let z = s.land().height(sp[0], sp[1]);
    println!("spawn {:?} z {z:.1} blocked {} road {:.2}", sp, s.land().blocked(sp[0], sp[1]), s.land().road(sp[0], sp[1]));
    for v in s.objects() {
        let blocked = s.land().blocked(v.pos[0], v.pos[1]);
        let kind = match &v.kind {
            ObjKind::Node { key, .. } => key.clone(),
            other => format!("{other:?}").chars().take(20).collect(),
        };
        let nearest_tree = s.land().trees().iter().map(|t| dist2d(*t, v.pos)).fold(f32::MAX, f32::min);
        println!("  obj {:<22} at {:>8.1} {:>6.1} {:>6.1}  blocked {blocked}  tree {nearest_tree:.1}", kind, v.pos[0], v.pos[1], v.pos[2]);
    }
    for (id, m) in &s.mobs {
        println!("  mob {id:>3} {:<10} at {:>8.1} {:>6.1} {:>6.1} blocked {}", c.mobs[m.def].key, m.pos[0], m.pos[1], m.pos[2], s.land().blocked(m.pos[0], m.pos[1]));
    }
    // Building ground: which spots take a cooking fire.
    let fire = c.station("cooking").unwrap();
    let bc = c.world.build_center;
    let mut good = Vec::new();
    let mut why: std::collections::BTreeMap<String, usize> = Default::default();
    let r = c.world.build_radius as i32;
    for dy in -r..=r {
        for dx in -r..=r {
            let (x, y) = (bc[0] + dx as f32, bc[1] + dy as f32);
            let pos = [x, y, s.land().height(x, y)];
            match s.site_problem(fire, pos, None) {
                None => good.push(pos),
                Some(p) => *why.entry(p).or_default() += 1,
            }
        }
    }
    println!("building ground: {} good 1-yd spots; refused: {why:?}", good.len());
    // The walk to the mine.
    let mine = [-9870.0, 200.0];
    let from = [sp[0], sp[1], z];
    let to = [mine[0], mine[1], s.land().height(mine[0], mine[1])];
    let t2 = Instant::now();
    let walk = s.land().path(from, to);
    println!("path camp -> mine: {:.1} ms, {} corners", t2.elapsed().as_secs_f32() * 1000.0, walk.as_ref().map_or(0, |p| p.len()));
    if let Some(w) = &walk {
        let pts: Vec<String> = w.iter().map(|q| format!("({:.0},{:.0})", q[0], q[1])).collect();
        println!("  the walk: {}", pts.join(" "));
    }
    // Each patrol's route: every point, and a way between each and the next.
    for (k, pk) in c.packs.iter().enumerate().filter(|(_, p)| !p.route.is_empty()) {
        let mut stops = vec![pk.at];
        stops.extend(pk.route.iter().copied());
        for w in stops.windows(2) {
            let (a, b) = (w[0], w[1]);
            let p = s.land().path([a[0], a[1], 0.0], [b[0], b[1], 0.0]);
            println!(
                "patrol {k} {}: {:?} -> {:?} blocked {} water {} road {:.2}, path {}",
                pk.mob,
                a,
                b,
                s.land().blocked(b[0], b[1]),
                s.land().water(b[0], b[1]),
                s.land().road(b[0], b[1]),
                p.as_ref().map_or("none".to_string(), |p| format!("{} corners", p.len()))
            );
        }
    }
    let raid_from = c.raid.from;
    let t3 = Instant::now();
    let rp = s.land().path([raid_from[0], raid_from[1], 0.0], [c.world.heart[0], c.world.heart[1], 0.0]);
    println!("path raid road -> camp: {:.1} ms, {} corners", t3.elapsed().as_secs_f32() * 1000.0, rp.as_ref().map_or(0, |p| p.len()));
    for t in 0..c.raid.count() {
        let plan = c.raid.plan(t as u32);
        let f = plan.from;
        let p = s.land().path([f[0], f[1], 0.0], [c.world.heart[0], c.world.heart[1], 0.0]);
        println!(
            "raid {} from {:?}: blocked {} water {} path {}",
            plan.title,
            f,
            s.land().blocked(f[0], f[1]),
            s.land().water(f[0], f[1]),
            p.as_ref().map_or("none".to_string(), |p| format!("{} corners", p.len()))
        );
    }
    // A hero at the mine yard among the Warsong: how long do ticks take?
    let id = s.join("probe", "Probe", None);
    s.move_player(id, to, 0.0, Default::default());
    let mut worst = 0.0f32;
    let mut total = 0.0f32;
    let mut hits = 0;
    for k in 0..400 {
        if k % 20 == 0 {
            s.input(id, Input::Guard);
        }
        let t = Instant::now();
        let ev = s.step(0.05);
        let dt = t.elapsed().as_secs_f32() * 1000.0;
        hits += ev.iter().filter(|e| matches!(e, htl_survival::Event::Swing { victim: Some(v), .. } if *v == id)).count();
        worst = worst.max(dt);
        total += dt;
        // Back up if knocked down, to keep the creatures busy.
        if !s.alive(id) {
            s.step(10.0);
            s.move_player(id, to, 0.0, Default::default());
        }
    }
    for (mid, m) in &s.mobs {
        let d = dist2d(m.pos, to);
        if d < 25.0 {
            let clear = htl_survival::land::clear_line(s.land(), m.pos, to);
            // Where the line is stopped.
            let mut stop = String::new();
            let n = (d / 0.5).ceil() as usize;
            for k in 1..n {
                let t = k as f32 / n as f32;
                let (x, y) = (m.pos[0] + (to[0] - m.pos[0]) * t, m.pos[1] + (to[1] - m.pos[1]) * t);
                let zl = m.pos[2] + 1.5 + (to[2] + 1.0 - m.pos[2] - 1.5) * t;
                if s.land().blocked(x, y) {
                    stop = format!("blocked at {x:.1},{y:.1}");
                    break;
                }
                if s.land().height(x, y) > zl {
                    stop = format!("ground {:.1} over the line {zl:.1} at {x:.1},{y:.1}", s.land().height(x, y));
                    break;
                }
            }
            println!("  mob {mid} {} {:?} d {d:.1} clear {clear} {stop}", c.mobs[m.def].key, m.state);
        }
    }
    println!("400 ticks at the mine: mean {:.2} ms, worst {:.2} ms; {hits} blows at the hero", total / 400.0, worst);

    let near: Vec<String> = s.land().trees().iter().filter(|t| dist2d(**t, from) < 160.0).map(|t| format!("({:.0},{:.0})", t[0], t[1])).collect();
    println!("{} trees within 160 yd of the camp: {}", near.len(), near.join(" "));
    // The picture.
    let (x0, x1, y0, y1) = (-9900.0f32, -9440.0f32, 20.0f32, 320.0f32);
    let scale = 2.0;
    let (w, h) = (((y1 - y0) * scale) as u32, ((x1 - x0) * scale) as u32);
    let mut img = image::RgbImage::new(w, h);
    let px = |x: f32, y: f32| -> Option<(u32, u32)> {
        if x < x0 || x >= x1 || y < y0 || y >= y1 {
            return None;
        }
        Some((((y1 - y) * scale) as u32, ((x1 - x) * scale) as u32))
    };
    let (mut lo, mut hi) = (f32::MAX, f32::MIN);
    for j in 0..h {
        for i in 0..w {
            let (x, y) = (x1 - j as f32 / scale, y1 - i as f32 / scale);
            let z = s.land().height(x, y);
            lo = lo.min(z);
            hi = hi.max(z);
        }
    }
    for j in 0..h {
        for i in 0..w {
            let (x, y) = (x1 - j as f32 / scale, y1 - i as f32 / scale);
            let v = ((s.land().height(x, y) - lo) / (hi - lo).max(1.0) * 180.0 + 40.0) as u8;
            let mut c = [v, v, v];
            let r = s.land().road(x, y);
            if r > 0.3 {
                c = [230, 140, 30];
            }
            if s.land().water(x, y) {
                c = [30, 60, 200];
            }
            if s.land().blocked(x, y) {
                c = [210, 40, 40];
            }
            img.put_pixel(i, j, image::Rgb(c));
        }
    }
    let dot = |img: &mut image::RgbImage, p: [f32; 3], col: [u8; 3], r: i32| {
        if let Some((i, j)) = px(p[0], p[1]) {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (a, b) = (i as i32 + dx, j as i32 + dy);
                    if a >= 0 && b >= 0 && (a as u32) < w && (b as u32) < h && dx * dx + dy * dy <= r * r {
                        img.put_pixel(a as u32, b as u32, image::Rgb(col));
                    }
                }
            }
        }
    };
    for t in s.land().trees() {
        dot(&mut img, t, [30, 170, 40], 2);
    }
    for g in &good {
        dot(&mut img, *g, [150, 255, 150], 0);
    }
    if let Some(p) = &walk {
        let mut last = from;
        for q in p {
            for k in 0..40 {
                let t = k as f32 / 40.0;
                dot(&mut img, [last[0] + (q[0] - last[0]) * t, last[1] + (q[1] - last[1]) * t, 0.0], [255, 255, 255], 0);
            }
            last = *q;
        }
    }
    for v in s.objects() {
        let col = match &v.kind {
            ObjKind::Node { key, .. } if key.starts_with("wood") => [140, 90, 40],
            ObjKind::Node { .. } => [40, 230, 230],
            _ => [255, 230, 40],
        };
        dot(&mut img, v.pos, col, 4);
    }
    for m in s.mobs.values() {
        dot(&mut img, m.pos, [240, 40, 240], 3);
    }
    img.save(&args[2])?;
    println!("drew {}x{} px", w, h);
    Ok(())
}
