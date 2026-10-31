//! What's around Goldshire, for laying out Elwynn Survival:
//! `elwynn <Data> pois` lists Eastern Kingdoms points of interest near Goldshire;
//! `elwynn <Data> near <x> <y> <radius> [word...]` lists buildings and doodads there (model
//! names holding a word, or all), with positions and terrain heights.
use benilla_formats::{load_area_poi_catalog, load_tiles_around, terrain_height_at, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    if args[2] == "pois" {
        let pois = load_area_poi_catalog(&mut chain)?;
        for (id, p) in pois.rows() {
            if p.continent_id == 0 && (p.pos[0] - -9465.0).abs() < 1500.0 && (p.pos[1] - 64.0).abs() < 1500.0 {
                println!("{id}\t{}\t{:.1} {:.1} {:.1}", p.name, p.pos[0], p.pos[1], p.pos[2]);
            }
        }
        return Ok(());
    }
    let x: f32 = args[3].parse()?;
    let y: f32 = args[4].parse()?;
    let r: f32 = args[5].parse()?;
    let words: Vec<String> = args[6..].iter().map(|w| w.to_lowercase()).collect();
    let tiles = load_tiles_around(&mut chain, "Azeroth", x, y, 1)?;
    let chunks: Vec<_> = tiles.iter().flat_map(|(_, t)| t.chunks.iter().cloned()).collect();
    let h = |px: f32, py: f32| terrain_height_at(&chunks, [px, py, 0.0]).unwrap_or(f32::NAN);
    println!("terrain at centre: {:.1}", h(x, y));
    let near = |p: [f32; 3]| ((p[0] - x).powi(2) + (p[1] - y).powi(2)).sqrt() <= r;
    let want = |m: &str| words.is_empty() || words.iter().any(|w| m.to_lowercase().contains(w));
    let mut seen = std::collections::BTreeSet::new();
    for (_, t) in &tiles {
        for w in &t.wmos {
            if near(w.position) && want(&w.model) && seen.insert(w.unique_id) {
                println!("WMO\t{}\t{:.1} {:.1} {:.1}\trot {:.0}", w.model, w.position[0], w.position[1], w.position[2], w.rotation[1]);
            }
        }
    }
    let mut counts: std::collections::BTreeMap<String, Vec<[f32; 3]>> = Default::default();
    for (_, t) in &tiles {
        for d in &t.doodads {
            if near(d.position) && want(&d.model) && seen.insert(d.unique_id) {
                counts.entry(d.model.clone()).or_default().push(d.position);
            }
        }
    }
    for (m, ps) in counts {
        let list: Vec<String> = ps.iter().take(8).map(|p| format!("({:.0},{:.0},{:.0})", p[0], p[1], p[2])).collect();
        println!("M2\t{}\tx{}\t{}", m, ps.len(), list.join(" "));
    }
    Ok(())
}
