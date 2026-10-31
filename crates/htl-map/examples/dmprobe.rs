//! The Deadmines' ground: `dmprobe <Data> <out-prefix> [x y z]...` loads the instance, lists its
//! dungeon models, and draws the ground (2 px a yard, north up): `-top.png` the highest floor at
//! each spot, `-low.png` the lowest, `-count.png` how many floors; heights as grey (dark low),
//! walls red, deep water blue, wading water cyan. With points given, `-reach.png` shows every
//! floor walkable from the first, by height, and each point's floors are listed.
use htl_map::layers::Layers;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let out = &args[2];
    let t0 = std::time::Instant::now();
    // DM_FINE="x0,x1,y0,y1,cell": only that box, at that cell size. DM_MAP: another instance
    // map's folder (StormwindJail, Shadowfang...), not the Deadmines'.
    let map = std::env::var("DM_MAP").unwrap_or_else(|_| htl_map::deadmines::MAP_DIR.to_string());
    let d = match std::env::var("DM_FINE") {
        Ok(v) => {
            let n: Vec<f32> = v.split(',').map(|s| s.parse().unwrap()).collect();
            htl_map::deadmines::load_instance_with(data, &map, n[4], Some(([n[0], n[2]], [n[1], n[3]])))?
        }
        Err(_) => htl_map::deadmines::load_instance(data, &map)?,
    };
    println!(
        "loaded in {:.1}s: box {:?}..{:?}, {} floors",
        t0.elapsed().as_secs_f32(),
        d.min,
        d.max,
        d.ground.floor_count()
    );
    for (m, p, lo, hi) in &d.models {
        println!(
            "  model {m} at {:.1} {:.1} {:.1}  spans x {:.0}..{:.0} y {:.0}..{:.0} z {:.0}..{:.0}",
            p[0], p[1], p[2], lo[0], hi[0], lo[1], hi[1], lo[2], hi[2]
        );
    }
    // DM_PROPS="tent,crate": the props whose file has any of those words, inside the box.
    if let Ok(words) = std::env::var("DM_PROPS") {
        let words: Vec<String> = words.split(',').map(|w| w.to_lowercase()).collect();
        for (m, p, solid) in &d.props {
            let ml = m.to_lowercase();
            let inside = p[0] >= d.min[0] && p[0] <= d.max[0] && p[1] >= d.min[1] && p[1] <= d.max[1];
            if inside && words.iter().any(|w| w == "*" || ml.contains(w.as_str())) {
                println!("  prop {:>7.1} {:>7.1} {:>6.1} {} {m}", p[0], p[1], p[2], if *solid { "solid" } else { "no hull" });
            }
        }
    }
    let g = &d.ground;
    // DM_SLOPE: the steepest climb walked (rise per yard; 1.2 by default, as walkers climb).
    let slope: f32 = std::env::var("DM_SLOPE").ok().and_then(|v| v.parse().ok()).unwrap_or(1.2);
    let (w, h) = g.size();
    let mut zlo = f32::MAX;
    let mut zhi = f32::MIN;
    let mut hist = std::collections::BTreeMap::<i32, usize>::new();
    for j in 0..h {
        for i in 0..w {
            for f in g.floors_at(i, j) {
                zlo = zlo.min(f.z);
                zhi = zhi.max(f.z);
                *hist.entry((f.z / 10.0).floor() as i32 * 10).or_default() += 1;
            }
        }
    }
    println!("floor heights {zlo:.0}..{zhi:.0}; by tens: {hist:?}");
    let pts: Vec<[f32; 3]> = args[3..]
        .chunks_exact(3)
        .map(|c| {
            [
                c[0].parse().unwrap(),
                c[1].parse().unwrap(),
                c[2].parse().unwrap(),
            ]
        })
        .collect();
    let sc = 2u32;
    // Image column = west to east (WoW -y), row = north to south (WoW -x).
    let draw = |name: &str, pick: &dyn Fn(usize, usize) -> Option<[u8; 3]>| -> anyhow::Result<()> {
        let mut img = image::RgbImage::new(h as u32 * sc, w as u32 * sc);
        for j in 0..h {
            for i in 0..w {
                let c = pick(i, j).unwrap_or([0, 0, 0]);
                let (col, row) = ((h - 1 - j) as u32, (w - 1 - i) as u32);
                for a in 0..sc {
                    for b in 0..sc {
                        img.put_pixel(col * sc + a, row * sc + b, image::Rgb(c));
                    }
                }
            }
        }
        // A tick every 50 yards.
        for j in 0..h {
            for i in 0..w {
                let [x, y] = g.cell_xy(i, j);
                if (x.round() as i32).rem_euclid(50) == 0 && (y.round() as i32).rem_euclid(50) == 0
                {
                    let (col, row) = ((h - 1 - j) as u32, (w - 1 - i) as u32);
                    img.put_pixel(col * sc, row * sc, image::Rgb([255, 255, 0]));
                }
            }
        }
        img.save(format!("{out}-{name}.png"))?;
        Ok(())
    };
    let grey = |z: f32| ((z - zlo) / (zhi - zlo).max(1.0) * 210.0 + 40.0) as u8;
    let paint = |f: &htl_map::layers::Floor| -> [u8; 3] {
        if f.blocked {
            [200, 40, 40]
        } else if f.water {
            [30, 60, 220]
        } else if f.wet {
            [40, 200, 220]
        } else {
            let v = grey(f.z);
            [v, v, v]
        }
    };
    draw("top", &|i, j| g.floors_at(i, j).last().map(paint))?;
    draw("low", &|i, j| g.floors_at(i, j).first().map(paint))?;
    draw("count", &|i, j| {
        let n = g.floors_at(i, j).len() as u8;
        (n > 0).then(|| [n.saturating_mul(50), n.saturating_mul(50), 60])
    })?;
    for p in &pts {
        match g.cell_at(p[0], p[1]) {
            Some((i, j)) => {
                let fl: Vec<String> = g
                    .floors_at(i, j)
                    .iter()
                    .map(|f| {
                        format!(
                            "{:.1}{}{}",
                            f.z,
                            if f.blocked { "#" } else { "" },
                            if f.water { "~" } else { "" }
                        )
                    })
                    .collect();
                println!(
                    "point {p:?}: floors {fl:?}, stands at {:.1}",
                    g.height_near(p[0], p[1], p[2])
                );
            }
            None => println!("point {p:?}: off the ground"),
        }
    }
    if let Some(first) = pts.first() {
        let reach = g.reachable(*first, slope);
        let n = reach.iter().filter(|r| **r).count();
        println!("{n} floors walkable from {first:?}");
        draw("reach", &|i, j| {
            let r = g.floor_range(i, j);
            r.clone().rev().find(|&f| reach[f]).map(|f| {
                let fl = g.floor(f);
                let v = grey(fl.z);
                if fl.wet {
                    [v / 3, v / 2, v]
                } else {
                    [v / 2, v, v / 2]
                }
            })
        })?;
        for p in &pts[1..] {
            match g.find_path(*first, *p, slope, None) {
                Some(path) => {
                    println!(
                        "walk to {p:?}: {} steps, ends at {:?}",
                        path.len(),
                        path.last()
                    );
                    if std::env::var_os("DM_PATH").is_some() {
                        for (n, q) in path.iter().enumerate().step_by(12) {
                            println!("    {n:>4}: {:>7.1} {:>7.1} {:>6.1}", q[0], q[1], q[2]);
                        }
                    }
                }
                None => println!("walk to {p:?}: no way"),
            }
        }
    }
    // DM_SPOTS=<yards>: every reachable spot from the first point on a grid that size: each
    // square's floors grouped by height (5-yard bands), with the floor's height, the fewest steps
    // there from the first point, and how much of the square is floor (open ground: a room).
    if let (Ok(v), Some(first)) = (std::env::var("DM_SPOTS"), pts.first()) {
        let size: f32 = v.parse().unwrap_or(10.0);
        let dist = g.reach_steps(*first, slope);
        let mut sq: std::collections::BTreeMap<(i32, i32, i32), (u32, f32, f32, f32, u32)> = Default::default();
        let (w, h) = g.size();
        for j in 0..h {
            for i in 0..w {
                for f in g.floor_range(i, j) {
                    if dist[f] == u32::MAX {
                        continue;
                    }
                    let fl = g.floor(f);
                    if fl.blocked {
                        continue;
                    }
                    let [x, y] = g.cell_xy(i, j);
                    let key = ((x / size).floor() as i32, (y / size).floor() as i32, (fl.z / 5.0).floor() as i32);
                    let e = sq.entry(key).or_insert((u32::MAX, 0.0, 0.0, 0.0, 0));
                    e.0 = e.0.min(dist[f]);
                    e.1 += x;
                    e.2 += y;
                    e.3 += fl.z;
                    e.4 += 1;
                }
            }
        }
        let full = (size / g.cell()).powi(2);
        let mut rows: Vec<_> = sq.into_iter().filter(|(_, e)| e.4 as f32 >= full * 0.15).collect();
        rows.sort_by_key(|(_, e)| e.0);
        println!("spots (x y z steps open%):");
        for (_, e) in rows {
            let n = e.4 as f32;
            println!("  {:>7.1} {:>7.1} {:>6.1} {:>5} {:>3}", e.1 / n, e.2 / n, e.3 / n, e.0, (n / full * 100.0).min(100.0) as u32);
        }
    }
    // Close looks: DM_CROP="name,x0,x1,y0,y1,zlo,zhi;..." at 4 px a yard, the floors in the band
    // (the highest in it), heights grey over the band, walls red, deep water blue, wading cyan; a
    // faint line every 5 yards, brighter every 10, yellow every 50.
    // With points given, what the first reaches shows green in the crops.
    let crop_reach = pts.first().map(|p| g.reachable(*p, slope));
    if let Ok(crops) = std::env::var("DM_CROP") {
        for c in crops.split(';').filter(|c| !c.is_empty()) {
            let v: Vec<&str> = c.split(',').collect();
            let name = v[0];
            let n: Vec<f32> = v[1..].iter().map(|s| s.parse().unwrap()).collect();
            let (x0, x1, y0, y1, zlo, zhi) = (n[0], n[1], n[2], n[3], n[4], n[5]);
            let sc = 4.0;
            let (iw, ih) = (((y1 - y0) * sc) as u32, ((x1 - x0) * sc) as u32);
            let mut img = image::RgbImage::new(iw, ih);
            for row in 0..ih {
                for col in 0..iw {
                    let (x, y) = (x1 - row as f32 / sc, y1 - col as f32 / sc);
                    let mut px = [10u8, 10, 10];
                    if let Some((i, j)) = g.cell_at(x, y) {
                        if let Some(fi) = g.floor_range(i, j).rev().find(|&k| {
                            let f = g.floor(k);
                            f.z >= zlo && f.z <= zhi
                        }) {
                            let f = g.floor(fi);
                            let t = ((f.z - zlo) / (zhi - zlo).max(1.0) * 200.0 + 50.0) as u8;
                            px = if f.blocked {
                                [210, 40, 40]
                            } else if f.water {
                                [30, 60, 220]
                            } else if f.wet {
                                [40, 190, 220]
                            } else if crop_reach.as_ref().is_some_and(|r| r[fi]) {
                                [t / 3, t, t / 3]
                            } else {
                                [t, t, t]
                            };
                        }
                    }
                    let on = |step: f32| {
                        (x / step).fract().abs() < 0.5 / sc / step * step.min(1.0)
                            || (y / step).fract().abs() < 0.5 / sc / step * step.min(1.0)
                    };
                    let line = |v: f32, step: f32| (v.rem_euclid(step)) < 1.0 / sc;
                    let _ = on;
                    if line(x, 50.0) || line(y, 50.0) {
                        px = [230, 210, 0];
                    } else if line(x, 10.0) || line(y, 10.0) {
                        px = [
                            px[0].saturating_add(60),
                            px[1].saturating_add(60),
                            px[2].saturating_add(20),
                        ];
                    } else if line(x, 5.0) || line(y, 5.0) {
                        px = [
                            px[0].saturating_add(25),
                            px[1].saturating_add(25),
                            px[2].saturating_add(25),
                        ];
                    }
                    img.put_pixel(col, row, image::Rgb(px));
                }
            }
            img.save(format!("{out}-{name}.png"))?;
            println!("crop {name}: x {x0}..{x1} (top is {x1}), y {y0}..{y1} (left is {y1}), z {zlo}..{zhi}; {iw}x{ih} px");
        }
    }
    let _ = Layers::build;
    Ok(())
}
