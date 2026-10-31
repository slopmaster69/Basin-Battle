//! Elwynn Survival's play area: `elwynnmap <Data> list` prints the ground textures by coverage;
//! `elwynnmap <Data> draw <out.png> [x,y ...]` draws it (north up, 1 px a yard): height, walls
//! (red), water (blue), road (orange), tree trunks (green), a tick every 50 yards, and any marks.
use benilla_formats::{Chain, ChunkMesh};
use htl_map::elwynn;
use htl_sim::Ground;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    if args[2] == "list" {
        let mut chain = Chain::open(data)?;
        let tiles = elwynn::tiles(&mut chain)?;
        let chunks: Vec<ChunkMesh> = tiles.iter().flat_map(|t| t.chunks.iter().cloned()).collect();
        let mut cover: std::collections::BTreeMap<String, usize> = Default::default();
        for c in &chunks {
            let [x, y, _] = c.positions[0];
            if x < elwynn::MIN[0] || x > elwynn::MAX[0] + 34.0 || y < elwynn::MIN[1] || y > elwynn::MAX[1] + 34.0 {
                continue;
            }
            for t in &c.layer_textures {
                *cover.entry(t.clone()).or_default() += 1;
            }
        }
        let mut v: Vec<_> = cover.into_iter().collect();
        v.sort_by(|a, b| b.1.cmp(&a.1));
        for (t, n) in v {
            println!("{n:>5} chunks  {t}");
        }
        return Ok(());
    }
    let e = elwynn::load(data)?;
    let g = &e.grid;
    let (w, h) = g.size();
    let zs = g.heights();
    let (lo, hi) = zs.iter().fold((f32::MAX, f32::MIN), |(a, b), &z| (a.min(z), b.max(z)));
    let mut img = image::RgbImage::new(h as u32, w as u32);
    let put = |img: &mut image::RgbImage, i: usize, j: usize, px: [u8; 3]| {
        img.put_pixel((h - 1 - j) as u32, (w - 1 - i) as u32, image::Rgb(px));
    };
    for j in 0..h {
        for i in 0..w {
            let [x, y] = g.cell_xy(i, j);
            let v = ((g.height(x, y) - lo) / (hi - lo) * 200.0 + 30.0) as u8;
            let mut px = [v, v, v];
            let r = g.road(x, y);
            if r > 0.0 {
                px = [(v as f32 * (1.0 - r) + 255.0 * r) as u8, (v as f32 * (1.0 - r) + 140.0 * r) as u8, (v as f32 * (1.0 - r)) as u8];
            }
            if g.is_water(x, y) {
                px = [20, 40, 160];
            } else if g.is_wet(x, y) {
                px = [90, 160, 255];
            }
            if g.is_blocked(x, y) {
                px = [220, 40, 40];
            }
            if (x.round() as i32).rem_euclid(50) == 0 || (y.round() as i32).rem_euclid(50) == 0 {
                px = [px[0] / 2, px[1] / 2, px[2] / 2 + 60];
            }
            put(&mut img, i, j, px);
        }
    }
    let mut dot = |p: [f32; 2], c: [u8; 3], r: i32| {
        if let Some((i, j)) = g.cell_at(p[0], p[1]) {
            for dy in -r..=r {
                for dx in -r..=r {
                    let (a, b) = (i as i32 + dx, j as i32 + dy);
                    if a >= 0 && b >= 0 && (a as usize) < w && (b as usize) < h && dx * dx + dy * dy <= r * r {
                        put(&mut img, a as usize, b as usize, c);
                    }
                }
            }
        }
    };
    for t in &e.trunks {
        dot([t.pos[0], t.pos[1]], [20, 200, 40], t.radius.ceil() as i32);
    }
    for m in &args[4..] {
        let v: Vec<f32> = m.split(',').filter_map(|s| s.parse().ok()).collect();
        if v.len() >= 2 {
            dot([v[0], v[1]], [255, 0, 255], 3);
        }
    }
    img.save(&args[3])?;
    println!("{}x{} yards, heights {lo:.0}..{hi:.0}, {} trunks", w, h, e.trunks.len());
    Ok(())
}
