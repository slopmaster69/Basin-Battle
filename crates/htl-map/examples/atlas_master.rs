//! The launcher atlas's geographic master, from the host's own 1.12.1 data: one continuous
//! surface for the regions it's asked for, so coastlines, roads, rivers and passes meet across
//! zone borders by construction. The painted atlas is drawn over this; nothing here ships.
//!
//! `atlas_master <Data> <out dir> [zones=Elwynn,Westfall] [ppy=1.0] [margin=300]`
//!
//! The frame is yards with x east (−world Y) and y south (−world X), the same as the client's
//! WorldMapArea rects, so a zone's map rect drops straight in. Writes, all `ppy` pixels a yard:
//! - `master.json`: every Eastern Kingdoms map rect (Alterac Mountains included), the covered
//!   rect, and the legends for the rasters below;
//! - `height.png` (16-bit; the range is in `master.json`) and `water.png` (0 dry, 1 still,
//!   2 ocean, 3 magma, 4 slime);
//! - `zones.png` (16-bit top-level `AreaTable` id per pixel: the true borders between zones) and
//!   `areas.png` (16-bit subzone id: Goldshire, Westbrook, the Jangolode Mine);
//! - `ground.png` (16-bit index of the strongest ground texture, legend in `master.json`), which
//!   finds the roads, paths and fields;
//! - `minimap.png`: the client's own minimap tiles, as a reference for the painters only;
//! - `features.json`: buildings (WMOs) and the doodads that mark crossings and defences.
use benilla_formats::{load_area_table_catalog, load_minimap_translate, load_tile_mesh, load_world_map_area_catalog, blp_to_rgba, Chain, TILE_SIZE};
use benilla_formats::LiquidKind;
use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;

const OFFSET: f32 = 32.0 * TILE_SIZE;
/// Doodads worth knowing about at the tactical distance.
const FEATURE_WORDS: [&str; 10] = ["bridge", "tower", "wall", "gate", "fence", "dock", "windmill", "watchtower", "palisade", "barricade"];

struct Raster {
    w: usize,
    h: usize,
    x0: f32,
    y0: f32,
    ppy: f32,
}

impl Raster {
    fn px(&self, ax: f32, ay: f32) -> (f32, f32) {
        ((ax - self.x0) * self.ppy, (ay - self.y0) * self.ppy)
    }
    /// Every pixel centre inside the triangle, with its barycentric weights.
    fn triangle(&self, p: [(f32, f32); 3], mut f: impl FnMut(usize, [f32; 3])) {
        let (minx, maxx) = (p.iter().map(|q| q.0).fold(f32::MAX, f32::min), p.iter().map(|q| q.0).fold(f32::MIN, f32::max));
        let (miny, maxy) = (p.iter().map(|q| q.1).fold(f32::MAX, f32::min), p.iter().map(|q| q.1).fold(f32::MIN, f32::max));
        let area = (p[1].0 - p[0].0) * (p[2].1 - p[0].1) - (p[2].0 - p[0].0) * (p[1].1 - p[0].1);
        if area.abs() < 1e-9 {
            return;
        }
        for j in (miny.floor().max(0.0) as usize)..=(maxy.ceil().min(self.h as f32 - 1.0).max(0.0) as usize) {
            for i in (minx.floor().max(0.0) as usize)..=(maxx.ceil().min(self.w as f32 - 1.0).max(0.0) as usize) {
                let (x, y) = (i as f32 + 0.5, j as f32 + 0.5);
                let w0 = ((p[1].0 - x) * (p[2].1 - y) - (p[2].0 - x) * (p[1].1 - y)) / area;
                let w1 = ((p[2].0 - x) * (p[0].1 - y) - (p[0].0 - x) * (p[2].1 - y)) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 >= -1e-4 && w1 >= -1e-4 && w2 >= -1e-4 {
                    f(j * self.w + i, [w0, w1, w2]);
                }
            }
        }
    }
}

fn arg<'a>(args: &'a [String], key: &str) -> Option<&'a str> {
    args.iter().find_map(|a| a.strip_prefix(&format!("{key}=")))
}

fn save16(path: &std::path::Path, w: usize, h: usize, data: &[u16]) -> anyhow::Result<()> {
    image::ImageBuffer::<image::Luma<u16>, Vec<u16>>::from_raw(w as u32, h as u32, data.to_vec())
        .ok_or_else(|| anyhow::anyhow!("raster size"))?
        .save(path)?;
    Ok(())
}

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    anyhow::ensure!(args.len() >= 3, "usage: atlas_master <Data> <out dir> [zones=Elwynn,Westfall] [ppy=1.0] [margin=300]");
    let out = std::path::PathBuf::from(&args[2]);
    std::fs::create_dir_all(&out)?;
    let wanted: Vec<String> = arg(&args, "zones").unwrap_or("Elwynn,Westfall").split(',').map(str::to_owned).collect();
    let ppy: f32 = arg(&args, "ppy").map_or(Ok(1.0), str::parse)?;
    let margin: f32 = arg(&args, "margin").map_or(Ok(300.0), str::parse)?;

    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let areas = load_area_table_catalog(&mut chain)?;
    let wma = load_world_map_area_catalog(&mut chain)?;
    // Every Eastern Kingdoms map rect, in the atlas frame: [x0, y0, x1, y1].
    let mut rects: BTreeMap<String, [f32; 4]> = BTreeMap::new();
    for (_, a) in wma.iter().filter(|(_, a)| a.map_id == 0 && a.area_id != 0) {
        rects.insert(a.name.clone(), [-a.loc_left, -a.loc_top, -a.loc_right, -a.loc_bottom]);
    }
    let pick: Vec<[f32; 4]> = wanted.iter().map(|z| rects.get(z).copied().ok_or_else(|| anyhow::anyhow!("no map rect called {z}; known: {:?}", rects.keys().collect::<Vec<_>>()))).collect::<anyhow::Result<_>>()?;
    let snap = |v: f32, up: bool| if up { (v / 100.0).ceil() * 100.0 } else { (v / 100.0).floor() * 100.0 };
    let x0 = snap(pick.iter().map(|r| r[0]).fold(f32::MAX, f32::min) - margin, false);
    let y0 = snap(pick.iter().map(|r| r[1]).fold(f32::MAX, f32::min) - margin, false);
    let x1 = snap(pick.iter().map(|r| r[2]).fold(f32::MIN, f32::max) + margin, true);
    let y1 = snap(pick.iter().map(|r| r[3]).fold(f32::MIN, f32::max) + margin, true);
    let r = Raster { w: ((x1 - x0) * ppy) as usize, h: ((y1 - y0) * ppy) as usize, x0, y0, ppy };
    eprintln!("[atlas] {:?} at {ppy} px/yd: {}x{} px over x {x0}..{x1}, y {y0}..{y1}", wanted, r.w, r.h);

    let n = r.w * r.h;
    let mut height = vec![f32::NAN; n];
    let mut water = vec![0u8; n];
    let mut zone = vec![0u16; n];
    let mut sub = vec![0u16; n];
    let mut ground = vec![0u16; n];
    let mut legend: Vec<String> = vec!["(none)".into()];
    let mut legend_ix: HashMap<String, u16> = HashMap::new();
    let mut features = String::from("[\n");
    let mut seen = std::collections::BTreeSet::new();

    // ADT tiles covering the rect: tile x runs with atlas x, tile y with atlas y.
    let (tx0, tx1) = (((OFFSET + x0) / TILE_SIZE).floor() as i32, ((OFFSET + x1) / TILE_SIZE).floor() as i32);
    let (ty0, ty1) = (((OFFSET + y0) / TILE_SIZE).floor() as i32, ((OFFSET + y1) / TILE_SIZE).floor() as i32);
    for tx in tx0.max(0)..=tx1.min(63) {
        for ty in ty0.max(0)..=ty1.min(63) {
            let Ok(t) = load_tile_mesh(&mut chain, "Azeroth", tx as u32, ty as u32) else { continue };
            for c in &t.chunks {
                let top = areas.top_zone(c.area_id).unwrap_or(c.area_id) as u16;
                let layer_ix: Vec<u16> = c.layer_textures.iter().map(|name| {
                    *legend_ix.entry(name.clone()).or_insert_with(|| { legend.push(name.clone()); (legend.len() - 1) as u16 })
                }).collect();
                let alpha = c.alpha_map.as_deref();
                for tri in c.indices.chunks_exact(3) {
                    let v = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                    let p = v.map(|k| r.px(-c.positions[k][1], -c.positions[k][0]));
                    r.triangle(p, |k, w| {
                        height[k] = w[0] * c.positions[v[0]][2] + w[1] * c.positions[v[1]][2] + w[2] * c.positions[v[2]][2];
                        zone[k] = top;
                        sub[k] = c.area_id as u16;
                        // The strongest ground layer here, from the chunk's alpha map.
                        let (u, vv) = (w[0] * c.uvs[v[0]][0] + w[1] * c.uvs[v[1]][0] + w[2] * c.uvs[v[2]][0], w[0] * c.uvs[v[0]][1] + w[1] * c.uvs[v[1]][1] + w[2] * c.uvs[v[2]][1]);
                        let mut best = (0usize, 0u8);
                        if let Some(a) = alpha {
                            let (ai, aj) = (((u * 64.0) as usize).min(63), ((vv * 64.0) as usize).min(63));
                            let texel = &a[(aj * 64 + ai) * 4..(aj * 64 + ai) * 4 + 4];
                            // Each later layer paints over the ones below at its own alpha.
                            let mut cover = [255u8, 0, 0, 0];
                            for l in 1..layer_ix.len().min(4) {
                                cover[l] = texel[l - 1];
                            }
                            for l in (0..layer_ix.len().min(4)).rev() {
                                if cover[l] > 127 || l == 0 {
                                    best = (l, cover[l]);
                                    break;
                                }
                            }
                        }
                        ground[k] = layer_ix.get(best.0).copied().unwrap_or(0);
                    });
                }
                for liq in &c.liquids {
                    let kind = match liq.kind {
                        LiquidKind::Ocean => 2,
                        LiquidKind::Magma => 3,
                        LiquidKind::Slime => 4,
                        _ => 1,
                    };
                    for tri in liq.indices.chunks_exact(3) {
                        let p = [0, 1, 2].map(|q| { let s = liq.positions[tri[q] as usize]; r.px(-s[1], -s[0]) });
                        r.triangle(p, |k, _| water[k] = kind);
                    }
                }
            }
            for w in &t.wmos {
                if seen.insert(("w", w.unique_id)) {
                    let _ = writeln!(features, "  {{\"kind\":\"building\",\"model\":{:?},\"at\":[{},{}],\"z\":{},\"heading\":{}}},", w.model, -w.position[1], -w.position[0], w.position[2], w.rotation[1]);
                }
            }
            for d in &t.doodads {
                let low = d.model.to_lowercase();
                if FEATURE_WORDS.iter().any(|f| low.contains(f)) {
                    let _ = writeln!(features, "  {{\"kind\":\"doodad\",\"model\":{:?},\"at\":[{},{}],\"z\":{},\"heading\":{},\"scale\":{}}},", d.model, -d.position[1], -d.position[0], d.position[2], d.rotation[1], d.scale);
                }
            }
        }
    }
    if features.ends_with(",\n") {
        features.truncate(features.len() - 2);
        features.push('\n');
    }
    features.push_str("]\n");
    std::fs::write(out.join("features.json"), features)?;

    // Heights to 16 bits over the range found; holes (cave mouths) stay at 0.
    let (hmin, hmax) = height.iter().filter(|h| h.is_finite()).fold((f32::MAX, f32::MIN), |(a, b), &h| (a.min(h), b.max(h)));
    let h16: Vec<u16> = height.iter().map(|&h| if h.is_finite() { (1.0 + (h - hmin) / (hmax - hmin).max(1.0) * 65534.0) as u16 } else { 0 }).collect();
    save16(&out.join("height.png"), r.w, r.h, &h16)?;
    image::GrayImage::from_raw(r.w as u32, r.h as u32, water).ok_or_else(|| anyhow::anyhow!("raster size"))?.save(out.join("water.png"))?;
    save16(&out.join("zones.png"), r.w, r.h, &zone)?;
    save16(&out.join("areas.png"), r.w, r.h, &sub)?;
    save16(&out.join("ground.png"), r.w, r.h, &ground)?;

    // The client's minimap tiles, 256 px a tile, scaled to the master's resolution.
    if let Ok(trs) = load_minimap_translate(&mut chain) {
        let side = (TILE_SIZE * ppy).round() as u32;
        let mut mm = image::RgbImage::new(r.w as u32, r.h as u32);
        for tx in tx0.max(0)..=tx1.min(63) {
            for ty in ty0.max(0)..=ty1.min(63) {
                let Some(file) = trs.tile("Azeroth", tx as u32, ty as u32) else { continue };
                let Ok(bytes) = chain.read_file(&format!("textures\\Minimap\\{file}")) else { continue };
                let Ok((w, h, rgba)) = blp_to_rgba(&bytes) else { continue };
                let Some(img) = image::RgbaImage::from_raw(w, h, rgba) else { continue };
                let img = image::imageops::resize(&img, side, side, image::imageops::FilterType::Triangle);
                let (ox, oy) = r.px(tx as f32 * TILE_SIZE - OFFSET, ty as f32 * TILE_SIZE - OFFSET);
                for (i, j, p) in img.enumerate_pixels() {
                    let (x, y) = (ox as i64 + i as i64, oy as i64 + j as i64);
                    if x >= 0 && y >= 0 && (x as usize) < r.w && (y as usize) < r.h {
                        mm.put_pixel(x as u32, y as u32, image::Rgb([p[0], p[1], p[2]]));
                    }
                }
            }
        }
        mm.save(out.join("minimap.png"))?;
    }

    // The frame, the rects and the legends.
    let mut json = String::new();
    let _ = writeln!(json, "{{\n \"frame\": \"yards; x east (-world Y), y south (-world X)\",\n \"zones_asked\": {:?},", wanted);
    let _ = writeln!(json, " \"rect\": [{x0}, {y0}, {x1}, {y1}],\n \"px_per_yd\": {ppy},\n \"size\": [{}, {}],\n \"height_range\": [{hmin}, {hmax}],", r.w, r.h);
    json.push_str(" \"map_rects\": {\n");
    let rows: Vec<String> = rects.iter().map(|(k, v)| format!("  {k:?}: [{}, {}, {}, {}]", v[0], v[1], v[2], v[3])).collect();
    json.push_str(&rows.join(",\n"));
    json.push_str("\n },\n \"zone_names\": {\n");
    let mut ids: Vec<u16> = zone.iter().chain(sub.iter()).copied().collect::<std::collections::BTreeSet<_>>().into_iter().collect();
    ids.retain(|&i| i != 0);
    let rows: Vec<String> = ids.iter().map(|&i| format!("  \"{i}\": {:?}", areas.name(i as u32).unwrap_or("?"))).collect();
    json.push_str(&rows.join(",\n"));
    json.push_str("\n },\n \"ground_legend\": [\n");
    let rows: Vec<String> = legend.iter().map(|s| format!("  {s:?}")).collect();
    json.push_str(&rows.join(",\n"));
    json.push_str("\n ]\n}\n");
    std::fs::write(out.join("master.json"), json)?;
    eprintln!("[atlas] wrote {}", out.display());
    Ok(())
}
