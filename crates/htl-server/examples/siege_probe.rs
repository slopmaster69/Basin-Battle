//! Arathi Basin for Siege: `siege_probe <Data> <out.png> [survival.toml]` loads the basin (time
//! and memory), lists its named points, finds open ground at each base, walks between the bases
//! and to the Blacksmith (the land's own path), and draws the basin a pixel a yard (north up):
//! walls red, water blue, road orange, trunks green, named points white, paths yellow. With a
//! content file, it lays a Siege world out from it and reports where its pieces stand.
use std::time::Instant;

use htl_server::survival::land::ElwynnLand;
use htl_survival::{dist2d, Land};

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
    let pois = htl_map::siege::pois(data)?;
    for (n, p) in &pois {
        println!("POI {n:<28} {:.1} {:.1} {:.1}", p[0], p[1], p[2]);
    }
    let loaded = htl_map::siege::load(data)?;
    println!("loaded Arathi Basin in {:.1}s: {} trunks, {} caves, peak {} MB", t0.elapsed().as_secs_f32(), loaded.trunks.len(), loaded.caves.len(), peak_kb() / 1024);
    let (lo, hi) = (loaded.min, loaded.max);
    if args.get(3).map(String::as_str) == Some("match") {
        return a_match(ElwynnLand::new(loaded));
    }
    let land = ElwynnLand::new(loaded);
    let l: &dyn Land = &land;
    // Open ground at each named point: the nearest spot 6 yd round that's flat, dry and clear.
    let open = |x: f32, y: f32, r: f32| -> Option<[f32; 3]> {
        let mut best: Option<([f32; 3], f32)> = None;
        for ring in 0..40 {
            let rad = ring as f32 * 1.5;
            let n = (ring * 8).max(1);
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let (px, py) = (x + a.cos() * rad, y + a.sin() * rad);
                let z = l.height(px, py);
                let ok = (0..16).all(|q| {
                    let b = q as f32 / 16.0 * std::f32::consts::TAU;
                    let (qx, qy) = (px + b.cos() * r, py + b.sin() * r);
                    !l.blocked(qx, qy) && !l.water(qx, qy) && (l.height(qx, qy) - z).abs() < 1.6
                });
                if ok {
                    let d = rad;
                    if best.is_none_or(|b| d < b.1) {
                        best = Some(([px, py, z], d));
                    }
                }
            }
            if best.is_some() {
                break;
            }
        }
        best.map(|b| b.0)
    };
    let mut marks: Vec<[f32; 3]> = Vec::new();
    for (n, p) in &pois {
        match open(p[0], p[1], 6.0) {
            Some(o) => println!("OPEN {n:<27} {:.1} {:.1} {:.1} ({:.0} yd off)", o[0], o[1], o[2], dist2d(o, *p)),
            None => println!("OPEN {n:<27} none"),
        }
        marks.push(*p);
    }
    // Walks: every base to every other and to the Blacksmith.
    let named = |k: &str| pois.iter().find(|(n, _)| n == k).map(|(_, p)| *p);
    let bases = ["Farm", "Stables", "Gold Mine", "Lumber Mill", "Blacksmith"];
    let mut paths = Vec::new();
    for (i, a) in bases.iter().enumerate() {
        for b in bases.iter().skip(i + 1) {
            let (Some(pa), Some(pb)) = (named(a), named(b)) else { continue };
            let (Some(oa), Some(ob)) = (open(pa[0], pa[1], 3.0), open(pb[0], pb[1], 3.0)) else { continue };
            let t = Instant::now();
            let p = l.path(oa, ob);
            let len = p.as_ref().map_or(0.0, |p| p.windows(2).map(|w| dist2d(w[0], w[1])).sum::<f32>());
            println!("WALK {a} -> {b}: {} in {:.0} ms, {:.0} yd walked ({:.0} straight)", if p.is_some() { "found" } else { "NONE" }, t.elapsed().as_secs_f64() * 1000.0, len, dist2d(oa, ob));
            if let Some(p) = p {
                paths.push(p);
            }
        }
    }
    // Each base's way in: points along the walk from it to the Blacksmith; and how much of the
    // ground round it a station could stand on.
    let smith = named("Blacksmith").and_then(|p| open(p[0], p[1], 3.0));
    for b in &bases[..4] {
        let (Some(p), Some(sm)) = (named(b), smith) else { continue };
        let Some(o) = open(p[0], p[1], 6.0) else { continue };
        if let Some(path) = l.path(o, sm) {
            let mut walked = 0.0;
            let mut out = Vec::new();
            let mut want = [40.0f32, 60.0, 80.0, 100.0].into_iter().peekable();
            for w in path.windows(2) {
                walked += dist2d(w[0], w[1]);
                if let Some(&next) = want.peek() {
                    if walked >= next {
                        out.push(format!("{next:.0}yd ({:.1},{:.1},{:.1})", w[1][0], w[1][1], w[1][2]));
                        want.next();
                    }
                }
            }
            println!("GATE {b}: {}", out.join("; "));
        }
        for r in [25.0f32, 35.0, 45.0] {
            let (mut all, mut ok) = (0, 0);
            let mut x = o[0] - r;
            while x <= o[0] + r {
                let mut y = o[1] - r;
                while y <= o[1] + r {
                    if dist2d([x, y, 0.0], o) <= r {
                        all += 1;
                        let z = l.height(x, y);
                        let flat = [(2.0f32, 0.0f32), (-2.0, 0.0), (0.0, 2.0), (0.0, -2.0)].iter().all(|(dx, dy)| !l.blocked(x + dx, y + dy) && !l.water(x + dx, y + dy) && (l.height(x + dx, y + dy) - z).abs() < 1.0);
                        if flat {
                            ok += 1;
                        }
                    }
                    y += 2.0;
                }
                x += 2.0;
            }
            println!("BUILD {b} within {r:.0} yd: {ok} of {all} spots flat and clear");
        }
    }
    for (n, x, y) in [("Trollbane Hall", 1300.0f32, 1300.0f32), ("Defiler's Den", 700.0, 700.0)] {
        match open(x, y, 6.0) {
            Some(o) => println!("OPEN {n:<27} {:.1} {:.1} {:.1}", o[0], o[1], o[2]),
            None => println!("OPEN {n:<27} none"),
        }
    }
    // Each base's roads: road cells within 70 yd, by bearing (where waves could come in from).
    for b in &bases[..4] {
        let Some(p) = named(b) else { continue };
        let mut by: Vec<(i32, f32, [f32; 3])> = Vec::new();
        for k in 0..36 {
            let a = k as f32 * 10.0;
            let r = a.to_radians();
            for d in (30..=70).step_by(4) {
                let (x, y) = (p[0] + r.cos() * d as f32, p[1] + r.sin() * d as f32);
                if l.road(x, y) > 0.4 && !l.blocked(x, y) && !l.water(x, y) {
                    by.push((a as i32, d as f32, [x, y, l.height(x, y)]));
                    break;
                }
            }
        }
        let list: Vec<String> = by.iter().map(|(a, d, q)| format!("{a}deg {d:.0}yd ({:.0},{:.0},{:.0})", q[0], q[1], q[2])).collect();
        println!("ROADS {b}: {}", list.join("; "));
    }
    // The map.
    let sc = 1.0;
    let (x0, x1, y0, y1) = (lo[0], hi[0], lo[1], hi[1]);
    let (w, h) = (((y1 - y0) * sc) as u32, ((x1 - x0) * sc) as u32);
    let mut img = image::RgbImage::new(w, h);
    for j in 0..h {
        for i in 0..w {
            let (x, y) = (x1 - j as f32 / sc, y1 - i as f32 / sc);
            let z = l.height(x, y);
            let v = ((z + 20.0) / 120.0 * 200.0 + 30.0).clamp(0.0, 255.0) as u8;
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
    for t in l.trees() {
        dot(t[0], t[1], 1, [20, 120, 20]);
    }
    for p in &paths {
        for q in p {
            dot(q[0], q[1], 1, [250, 240, 60]);
        }
    }
    for m in &marks {
        dot(m[0], m[1], 4, [255, 255, 255]);
    }
    img.save(&args[2])?;
    println!("map written to {} ({}x{}), peak {} MB", args[2], w, h, peak_kb() / 1024);
    Ok(())
}

/// A four-camp match on the real basin: where each camp's pieces and gate stand, whether its waves
/// reach its Hearth (and how long they take), and what a tick costs.
fn a_match(land: ElwynnLand) -> anyhow::Result<()> {
    use htl_survival::{Content, Moving, Survival};
    let mut c = Content::builtin();
    c.dev.full_tree = false;
    let d = c.siege.clone().unwrap();
    let mut s = Survival::new_siege(c, Box::new(land), 7, 1_000_000.0);
    let names = ["Ann", "Bob", "Cat", "Dan"];
    let ids: Vec<u32> = names.iter().map(|n| {
        let id = s.join(&n.to_lowercase(), n, None);
        s.siege_arrive(id);
        id
    }).collect();
    let veins = s.objects().into_iter().filter(|o| matches!(o.kind, htl_survival::ObjKind::Node { .. })).count();
    println!("HALL heroes at {:?}; {veins} veins round the Blacksmith", s.players[&ids[0]].pos.map(|v| v.round()));
    for id in &ids {
        s.siege_join(*id, "1v1v1v1").map_err(anyhow::Error::msg)?;
    }
    for _ in 0..((d.countdown + 0.5) / 0.05) as usize {
        s.step(0.05);
    }
    let sg = s.siege.clone().unwrap();
    for (k, cmp) in sg.camps.iter().enumerate() {
        let b = &d.bases[cmp.def];
        let pos = |id: u32| s.objs.get(&id).map(|o| o.pos).unwrap_or([0.0; 3]);
        let (h, l, r) = (pos(cmp.hearth), pos(cmp.ledger), pos(cmp.rack));
        let hero = s.players[&cmp.heroes[0]].pos;
        let walk = s.land().path(cmp.gate, h);
        let len = walk.as_ref().map_or(0.0, |p| { let mut f = cmp.gate; p.iter().map(|q| { let d = dist2d(f, *q); f = *q; d }).sum::<f32>() });
        println!("CAMP {k} {}: hearth {:?} ledger {:.0} yd off, rack {:.0} yd off, hero {:.0} yd off (z {:.1} vs {:.1}); gate {:?} {:.0} yd straight, walk {} {:.0} yd", b.name, h.map(|v| v.round()), dist2d(l, h), dist2d(r, h), dist2d(hero, h), hero[2], h[2], cmp.gate.map(|v| v.round()), dist2d(cmp.gate, h), if walk.is_some() { "found" } else { "NONE" }, len);
        if walk.is_none() {
            let back = s.land().path(h, cmp.gate).is_some();
            let near = [h[0] + (cmp.gate[0] - h[0]) * 0.1, h[1] + (cmp.gate[1] - h[1]) * 0.1, h[2]];
            let to_near = s.land().path(cmp.gate, near).is_some();
            println!("  hearth blocked {} water {}; gate blocked {} water {}; hearth->gate {back}; gate->7 yd out {to_near}", s.land().blocked(h[0], h[1]), s.land().water(h[0], h[1]), s.land().blocked(cmp.gate[0], cmp.gate[1]), s.land().water(cmp.gate[0], cmp.gate[1]));
            for r in [2.0f32, 4.0, 6.0, 8.0] {
                let ring: Vec<String> = (0..8).map(|q| {
                    let a = q as f32 * std::f32::consts::FRAC_PI_4;
                    let (x, y) = (h[0] + a.cos() * r, h[1] + a.sin() * r);
                    format!("{}{}", if s.land().blocked(x, y) { "B" } else { "." }, (s.land().height(x, y) - h[2]).round())
                }).collect();
                println!("  ring {r}: {}", ring.join(" "));
            }
        }
        for (what, at) in [("ledger", l), ("rack", r), ("gate", cmp.gate)] {
            if s.land().blocked(at[0], at[1]) || s.land().water(at[0], at[1]) {
                println!("  {what} stands in a wall or water");
            }
        }
    }
    // Everyone stands in the hall: the waves walk to the Hearths unopposed.
    for id in &ids {
        let lb = d.lobby;
        s.move_player(*id, [lb[0], lb[1], lb[2]], 0.0, Moving::default());
    }
    let t = Instant::now();
    let mut worst = 0.0f64;
    let mut arrived: std::collections::BTreeMap<u32, f32> = Default::default();
    let start = s.clock;
    for _ in 0..((d.first_gap + 60.0) / 0.05) as usize {
        let t1 = Instant::now();
        s.step(0.05);
        let took = t1.elapsed().as_secs_f64();
        worst = worst.max(took);
        if took > 0.04 {
            let moving = s.mobs.values().filter(|m| !m.path.is_empty()).count();
            println!("SLOW TICK at {:.2} s: {:.0} ms ({} creatures, {moving} walking)", s.clock - start, took * 1000.0, s.mobs.len());
            for m in s.mobs.values() {
                println!("   {} {:?} at {:.0},{:.0},{:.0} pattern {:?} special {} stagger {:.1}", s.content.mobs[m.def].key, m.state, m.pos[0], m.pos[1], m.pos[2], m.pattern, m.special, m.stagger_until - s.clock);
            }
        }
        let sg = s.siege.as_ref().unwrap();
        for (m, k) in &sg.foes {
            if arrived.contains_key(m) {
                continue;
            }
            let h = s.objs.get(&sg.camps[*k].hearth).map(|o| o.pos).unwrap_or([0.0; 3]);
            if s.mobs.get(m).is_some_and(|x| dist2d(x.pos, h) < 6.0) {
                arrived.insert(*m, s.clock - start - d.first_gap);
            }
        }
    }
    let sg = s.siege.as_ref().unwrap();
    for (k, cmp) in sg.camps.iter().enumerate() {
        let foes: Vec<u32> = sg.foes.iter().filter(|(_, c)| **c == k).map(|(m, _)| *m).collect();
        let at: Vec<f32> = foes.iter().filter_map(|m| arrived.get(m).copied()).collect();
        let hp = match s.objs.get(&cmp.hearth).map(|o| &o.kind) { Some(htl_survival::ObjKind::Station { health, .. }) => *health, _ => 0 };
        let stuck: Vec<String> = foes.iter().filter(|m| !arrived.contains_key(m)).filter_map(|m| s.mobs.get(m).map(|x| format!("{}@{:.0},{:.0},{:.0}", s.content.mobs[x.def].key, x.pos[0], x.pos[1], x.pos[2]))).collect();
        println!("WAVE at {}: {} foes, {} at the Hearth (first {:.0} s, last {:.0} s), hearth {hp}; not there: {:?}", d.bases[cmp.def].name, foes.len(), at.len(), at.iter().cloned().fold(f32::MAX, f32::min), at.iter().cloned().fold(0.0, f32::max), stuck);
    }
    if let Some((ns, f, t, found)) = ElwynnLand::slowest_walk() {
        println!("SLOWEST WALK: {:.0} ms from {:.0},{:.0},{:.0} to {:.0},{:.0},{:.0} ({:.0} yd), {}", ns as f64 / 1e6, f[0], f[1], f[2], t[0], t[1], t[2], dist2d(f, t), if found { "found" } else { "none" });
    }
    println!("ticks: {:.3} ms mean, {:.2} ms worst", t.elapsed().as_secs_f64() * 1000.0 / ((d.first_gap + 60.0) / 0.05) as f64, worst * 1000.0);
    Ok(())
}
