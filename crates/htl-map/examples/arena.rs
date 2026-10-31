//! Print the arena built from a 1.12.1 install and draw it to a PNG:
//! `cargo run -p htl-map --example arena -- <Data dir> [out.png]`.
//! Shading is height, red is walls, blue is deep water, yellow the lanes, green the hub.
use htl_sim::Ground;

fn main() -> anyhow::Result<()> {
    let data = std::env::args().nth(1).expect("pass the Data folder");
    let out = std::env::args().nth(2);
    let t = std::time::Instant::now();
    let a = htl_map::load(std::path::Path::new(&data))?;
    println!("loaded in {:?}", t.elapsed());
    println!("hub {:?} facing {:.2}", a.arena.hub, a.arena.hub_facing);
    println!("spawn {:?}", a.arena.player_spawn);
    for l in &a.arena.lanes {
        let len = htl_sim::geom::path_len(&l.points);
        let wet = l.points.iter().filter(|p| a.ground.is_water(p[0], p[1])).count();
        let zs: Vec<f32> = l.points.iter().map(|p| p[2]).collect();
        let zmin = zs.iter().cloned().fold(f32::MAX, f32::min);
        let zmax = zs.iter().cloned().fold(f32::MIN, f32::max);
        println!("{}: {} points, {:.0} yd, start {:?}, z {:.1}..{:.1}, {} in deep water", l.name, l.points.len(), len, l.points[0], zmin, zmax, wet);
        for (k, r) in l.routes.iter().enumerate() {
            let wet = r.iter().filter(|p| a.ground.is_wet(p[0], p[1])).count();
            let road = r.iter().filter(|p| a.ground.road(p[0], p[1]) > 0.5).count();
            println!("  route {k}: {:.0} yd, {} of {} points on road, {} wet", htl_sim::geom::path_len(r), road, r.len(), wet);
        }
    }
    if let Some(path) = out {
        let g = &a.ground;
        let (w, h) = g.size();
        let zs = g.heights();
        let (lo, hi) = zs.iter().fold((f32::MAX, f32::MIN), |(a, b), &z| (a.min(z), b.max(z)));
        let mut img = image::RgbImage::new(w as u32, h as u32);
        // WoW +x is north and +y west: draw north up, west left.
        let to_px = |i: usize, j: usize| ((w.max(h) - 1 - j) as i32, (w - 1 - i) as i32);
        let mut img2 = image::RgbImage::new(h as u32, w as u32);
        let reach = htl_map::reachable(g, a.arena.hub, htl_map::MAX_SLOPE);
        for j in 0..h {
            for i in 0..w {
                let [x, y] = g.cell_xy(i, j);
                let v = ((g.height(x, y) - lo) / (hi - lo) * 200.0 + 30.0) as u8;
                let tint = reach[j * w + i];
                let px = if g.is_blocked(x, y) { [220, 40, 40] } else if g.is_water(x, y) { [40, 80, if tint { 255 } else { 150 }] } else if tint { [v / 2, v, v / 2] } else { [v, v, v] };
                img2.put_pixel((h - 1 - j) as u32, (w - 1 - i) as u32, image::Rgb(px));
            }
        }
        let _ = (&mut img, to_px);
        let mut dot = |p: [f32; 3], c: [u8; 3], r: i32| {
            if let Some((i, j)) = g.cell_at(p[0], p[1]) {
                for dy in -r..=r {
                    for dx in -r..=r {
                        let (px, py) = ((h - 1 - j) as i32 + dx, (w - 1 - i) as i32 + dy);
                        if px >= 0 && py >= 0 && px < h as i32 && py < w as i32 {
                            img2.put_pixel(px as u32, py as u32, image::Rgb(c));
                        }
                    }
                }
            }
        };
        for l in &a.arena.lanes {
            for p in &l.points {
                dot(*p, [250, 220, 0], 1);
            }
        }
        dot(a.arena.hub, [0, 230, 0], 4);
        dot(a.arena.player_spawn, [0, 200, 255], 2);
        img2.save(&path)?;
        println!("drew {path}");
    }
    Ok(())
}
