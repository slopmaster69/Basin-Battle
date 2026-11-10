//! The Deadmines mine's own doodads, by their index in its WMO (the numbers client patch 0091 and
//! `htl_map::deadmines::aftermath::MINE_DOODADS` hide): `dmdoodads <Data> [word...]` lists each
//! doodad the mine places (its set 0 and the placement's own set) whose model holds a word (all,
//! with none): index, where it stands in the world, its scale, how far it leans, its model, `M2
//! light` when the model carries a light of its own, and the nearest of the WMO's own lights
//! (MOLT: patch 0091 puts out those within 1.6 yd of a dark lantern). `DM_NEAR=x,y,r` keeps only
//! those within r yd of (x, y), `DM_FULL` prints whole model paths; `lights` as a word lists the
//! WMO's lights instead. Never commit the output.
use benilla_formats::{load_tile_mesh, m2_file_name, parse_m2_lights, parse_wmo_lights, parse_wmo_root, Chain};
use htl_map::world::{placement_matrix, transform};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let words: Vec<String> = args[2..].iter().map(|w| w.to_lowercase()).collect();
    let near: Option<Vec<f32>> = std::env::var("DM_NEAR").ok().map(|v| v.split(',').map(|s| s.parse().unwrap()).collect());
    let inside = |p: [f32; 3]| near.as_ref().is_none_or(|n| (p[0] - n[0]).hypot(p[1] - n[1]) <= n[2]);
    let mut seen = std::collections::HashSet::new();
    for y in 0..64u32 {
        for x in 0..64u32 {
            let Ok(tile) = load_tile_mesh(&mut chain, htl_map::deadmines::MAP_DIR, x, y) else { continue };
            for w in &tile.wmos {
                if !seen.insert(w.unique_id) {
                    continue;
                }
                let bytes = chain.read_file(&w.model.to_ascii_lowercase())?;
                let root = parse_wmo_root(&bytes)?;
                let m = placement_matrix(w.rotation);
                let lights: Vec<([f32; 3], f32, [f32; 3])> =
                    parse_wmo_lights(&bytes).iter().map(|l| (transform(&m, w.position, l.position), l.intensity, l.color)).collect();
                println!("== {} at {:.1} {:.1} {:.1}, set {}, {} doodads, {} lights", w.model, w.position[0], w.position[1], w.position[2], w.doodad_set, root.doodads().len(), lights.len());
                if words.iter().any(|w| w == "lights") {
                    for (i, (p, intensity, c)) in lights.iter().enumerate().filter(|(_, l)| inside(l.0)) {
                        println!("  light {i:>3} {:>7.1} {:>7.1} {:>6.1}  intensity {intensity:.2} colour {:.2} {:.2} {:.2}", p[0], p[1], p[2], c[0], c[1], c[2]);
                    }
                    continue;
                }
                let sets = root.doodad_sets();
                let mut pick: Vec<usize> = Vec::new();
                for k in [0usize, w.doodad_set as usize] {
                    if let Some(set) = sets.get(k) {
                        pick.extend(set.start as usize..(set.start + set.count) as usize);
                    }
                }
                pick.sort_unstable();
                pick.dedup();
                for k in pick {
                    let Some(d) = root.doodads().get(k) else { continue };
                    let ml = d.model.to_lowercase();
                    if !words.is_empty() && !words.iter().any(|w| ml.contains(w.as_str())) {
                        continue;
                    }
                    let at = transform(&m, w.position, d.position);
                    if !inside(at) {
                        continue;
                    }
                    let q = d.orientation;
                    // The model's up axis (its own z) against the world's: 0 stands straight.
                    let up_z = 1.0 - 2.0 * (q[0] * q[0] + q[1] * q[1]);
                    let lit = m2_file_name(&d.model).and_then(|f| chain.read_file(&f).ok())
                        .map_or(0, |b| parse_m2_lights(&b).len());
                    let nearest = lights.iter().enumerate()
                        .map(|(i, (p, _, _))| (i, (0..3).map(|a| (p[a] - at[a]).powi(2)).sum::<f32>().sqrt()))
                        .min_by(|a, b| a.1.total_cmp(&b.1));
                    let name = if std::env::var_os("DM_FULL").is_some() { d.model.as_str() } else { d.model.rsplit(['\\', '/']).next().unwrap_or(&d.model) };
                    println!(
                        "  {k:>4} {:>7.1} {:>7.1} {:>6.1}  x{:.2} lean {:>3.0}  {name}{}{}",
                        at[0], at[1], at[2], d.scale, up_z.clamp(-1.0, 1.0).acos().to_degrees(),
                        if lit > 0 { format!("  M2 light x{lit}") } else { String::new() },
                        nearest.filter(|(_, dist)| *dist < 6.0).map(|(i, dist)| format!("  light {i} at {dist:.1}")).unwrap_or_default(),
                    );
                }
            }
        }
    }
    Ok(())
}
