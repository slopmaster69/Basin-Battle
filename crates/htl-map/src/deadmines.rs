//! The Deadmines, read from the host's own 1.12.1 install: the instance's map (Ironclad Cove's
//! terrain and water, the mine's four dungeon models) as ground with floors above floors. Any
//! other instance map reads the same way ([`load_instance`]): its tiles' terrain and buildings if
//! it has tiles, its one building if it's a WMO-only map (the Stockade, Shadowfang Keep...).
//! Nothing read here leaves the machine.

use std::path::Path;

use anyhow::{bail, Context, Result};
use benilla_formats::{
    load_m2_collision_hull, load_tile_mesh, load_wmo_collision_tris, parse_wmo_root, terrain_height_at,
    wmo_group_liquid_mesh, Chain, ChunkMesh, TileMesh, WdtReader, WmoInstance, WowVersion,
};

use crate::layers::{Layers, Sources};
use crate::world::{placement_matrix, transform};

/// `v` turned by the quaternion `q` (x, y, z, w).
fn quat_rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let n = (x * x + y * y + z * z + w * w).sqrt().max(1e-9);
    let (x, y, z, w) = (x / n, y / n, z / n, w / n);
    // t = 2 (q.xyz × v); v' = v + w t + q.xyz × t
    let t = [2.0 * (y * v[2] - z * v[1]), 2.0 * (z * v[0] - x * v[2]), 2.0 * (x * v[1] - y * v[0])];
    [
        v[0] + w * t[0] + (y * t[2] - z * t[1]),
        v[1] + w * t[1] + (z * t[0] - x * t[2]),
        v[2] + w * t[2] + (x * t[1] - y * t[0]),
    ]
}

/// The Deadmines' instance map.
pub const MAP_ID: u32 = 36;
pub const MAP_DIR: &str = "DeadminesInstance";
/// `AreaTable.dbc`: The Deadmines.
pub const AREA_ID: u32 = 1581;
const CELL: f32 = 1.0;
/// Ground this far round the dungeon's models is kept.
const MARGIN: f32 = 20.0;

pub struct LoadedDeadmines {
    pub ground: Layers,
    /// Each dungeon model placed: its file and where (for the probe).
    pub models: Vec<(String, [f32; 3], [f32; 3], [f32; 3])>,
    /// Every prop placed in it (a model's doodads, and what stands on the terrain): its file,
    /// where, and whether it has a hull to walk into (for the probe).
    pub props: Vec<(String, [f32; 3], bool)>,
    pub min: [f32; 2],
    pub max: [f32; 2],
}

/// Every tile an instance's map has (none, for a WMO-only map), and its one building if it's a
/// WMO-only map, as a tile of its own.
pub fn tiles_of(chain: &mut Chain, map_dir: &str) -> Result<Vec<TileMesh>> {
    let mut out = Vec::new();
    for y in 0..64u32 {
        for x in 0..64u32 {
            if let Ok(t) = load_tile_mesh(chain, map_dir, x, y) {
                out.push(t);
            }
        }
    }
    let wdt = chain
        .read_file(&format!("World\\Maps\\{map_dir}\\{map_dir}.wdt"))
        .ok()
        .and_then(|b| WdtReader::new(std::io::Cursor::new(b), WowVersion::Classic).read().ok());
    if let Some(g) = wdt.as_ref().and_then(|w| w.global_wmo()) {
        out.push(TileMesh {
            chunks: Vec::new(),
            doodads: Vec::new(),
            wmos: vec![WmoInstance {
                model: g.model.clone(),
                position: g.position,
                rotation: g.rotation,
                unique_id: u32::MAX,
                doodad_set: g.doodad_set,
                name_set: g.name_set,
            }],
        });
    }
    if out.is_empty() {
        bail!("no {map_dir} found in the game data");
    }
    Ok(out)
}

/// Every tile the Deadmines' map has.
pub fn tiles(chain: &mut Chain) -> Result<Vec<TileMesh>> {
    tiles_of(chain, MAP_DIR)
}

pub fn load(data_dir: &Path) -> Result<LoadedDeadmines> {
    load_with(data_dir, CELL, None)
}

/// [`load`] at another cell size, over part of the map only (for a close look).
pub fn load_with(data_dir: &Path, cell: f32, window: Option<([f32; 2], [f32; 2])>) -> Result<LoadedDeadmines> {
    load_instance_with(data_dir, MAP_DIR, cell, window)
}

/// Any instance map (its `World\Maps` folder's name), as ground with floors above floors.
pub fn load_instance(data_dir: &Path, map_dir: &str) -> Result<LoadedDeadmines> {
    load_instance_with(data_dir, map_dir, CELL, None)
}

/// [`load_instance`] at another cell size, over part of the map only.
pub fn load_instance_with(data_dir: &Path, map_dir: &str, cell: f32, window: Option<([f32; 2], [f32; 2])>) -> Result<LoadedDeadmines> {
    let mut chain = Chain::open(data_dir)
        .with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let tiles = tiles_of(&mut chain, map_dir)?;
    let chunks: Vec<ChunkMesh> = tiles
        .iter()
        .flat_map(|t| t.chunks.iter().cloned())
        .collect();
    let mut seen = std::collections::HashSet::new();
    let mut solid: Vec<[[f32; 3]; 3]> = Vec::new();
    let mut liquid: Vec<[[f32; 3]; 3]> = Vec::new();
    let mut models = Vec::new();
    let mut props: Vec<(String, [f32; 3], bool)> = Vec::new();
    let mut hulls: std::collections::HashMap<String, benilla_formats::CollisionMesh> = std::collections::HashMap::new();
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for t in &tiles {
        for w in &t.wmos {
            if !seen.insert(w.unique_id) {
                continue;
            }
            let Ok(mesh) = load_wmo_collision_tris(&mut chain, &w.model) else {
                continue;
            };
            let m = placement_matrix(w.rotation);
            let world: Vec<[f32; 3]> = mesh
                .positions
                .iter()
                .map(|&v| transform(&m, w.position, v))
                .collect();
            // Its doodads that collide (stairs, crates, planks): the always-on set and its own
            // (`DM_NO_DOODADS` set: none, to see what stands on the model alone).
            let doodads = std::env::var_os("DM_NO_DOODADS").is_none();
            if let (true, Ok(bytes)) = (doodads, chain.read_file(&w.model.to_ascii_lowercase())) {
                if let Ok(root) = parse_wmo_root(&bytes) {
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
                        if d.model.is_empty() {
                            continue;
                        }
                        let hull = match hulls.get(&d.model) {
                            Some(h) => h.clone(),
                            None => {
                                let h = load_m2_collision_hull(&mut chain, &d.model).unwrap_or_default();
                                hulls.insert(d.model.clone(), h.clone());
                                h
                            }
                        };
                        props.push((d.model.clone(), transform(&m, w.position, d.position), !hull.is_empty()));
                        if hull.is_empty() {
                            continue;
                        }
                        let pts: Vec<[f32; 3]> = hull
                            .positions
                            .iter()
                            .map(|&v| {
                                let local = quat_rotate(d.orientation, [v[0] * d.scale, v[1] * d.scale, v[2] * d.scale]);
                                let at = [local[0] + d.position[0], local[1] + d.position[1], local[2] + d.position[2]];
                                transform(&m, w.position, at)
                            })
                            .collect();
                        solid.extend(hull.indices.chunks_exact(3).map(|c| [pts[c[0] as usize], pts[c[1] as usize], pts[c[2] as usize]]));
                    }
                }
            }
            let (mut mlo, mut mhi) = ([f32::MAX; 3], [f32::MIN; 3]);
            for p in &world {
                for a in 0..3 {
                    mlo[a] = mlo[a].min(p[a]);
                    mhi[a] = mhi[a].max(p[a]);
                }
            }
            for a in 0..2 {
                lo[a] = lo[a].min(mlo[a]);
                hi[a] = hi[a].max(mhi[a]);
            }
            models.push((w.model.clone(), w.position, mlo, mhi));
            solid.extend(mesh.indices.chunks_exact(3).map(|c| {
                [
                    world[c[0] as usize],
                    world[c[1] as usize],
                    world[c[2] as usize],
                ]
            }));
            // Its water: each group's surface, placed as the model is.
            let root_path = w.model.to_ascii_lowercase();
            let groups = chain
                .read_file(&root_path)
                .ok()
                .and_then(|b| parse_wmo_root(&b).ok())
                .map_or(0, |r| r.group_count());
            let stem = root_path
                .strip_suffix(".wmo")
                .unwrap_or(&root_path)
                .to_string();
            for gi in 0..groups {
                let Ok(gb) = chain.read_file(&format!("{stem}_{gi:03}.wmo")) else {
                    continue;
                };
                let Some(lm) = wmo_group_liquid_mesh(&gb) else {
                    continue;
                };
                let wp: Vec<[f32; 3]> = lm
                    .positions
                    .iter()
                    .map(|&v| transform(&m, w.position, v))
                    .collect();
                liquid.extend(
                    lm.indices
                        .chunks_exact(3)
                        .map(|c| [wp[c[0] as usize], wp[c[1] as usize], wp[c[2] as usize]]),
                );
            }
        }
    }
    if models.is_empty() {
        bail!("no {map_dir} dungeon models found in the game data");
    }
    // What stands on the terrain itself (the cove's rocks, its tents and crates): each placed
    // model's collision hull, where the map has terrain (`DM_NO_DOODADS` set: none).
    if std::env::var_os("DM_NO_DOODADS").is_none() {
        let mut placed = std::collections::HashSet::new();
        for t in &tiles {
            for d in &t.doodads {
                if d.model.is_empty() || !placed.insert(d.unique_id) {
                    continue;
                }
                let hull = match hulls.get(&d.model) {
                    Some(h) => h.clone(),
                    None => {
                        let h = load_m2_collision_hull(&mut chain, &d.model).unwrap_or_default();
                        hulls.insert(d.model.clone(), h.clone());
                        h
                    }
                };
                props.push((d.model.clone(), d.position, !hull.is_empty()));
                if hull.is_empty() {
                    continue;
                }
                let m = placement_matrix(d.rotation);
                let pts: Vec<[f32; 3]> = hull
                    .positions
                    .iter()
                    .map(|&v| transform(&m, d.position, [v[0] * d.scale, v[1] * d.scale, v[2] * d.scale]))
                    .collect();
                solid.extend(hull.indices.chunks_exact(3).map(|c| [pts[c[0] as usize], pts[c[1] as usize], pts[c[2] as usize]]));
            }
        }
    }
    let (min, max) = window.unwrap_or(([lo[0] - MARGIN, lo[1] - MARGIN], [hi[0] + MARGIN, hi[1] + MARGIN]));
    for c in &chunks {
        for l in &c.liquids {
            liquid.extend(l.indices.chunks_exact(3).map(|i| {
                [
                    l.positions[i[0] as usize],
                    l.positions[i[1] as usize],
                    l.positions[i[2] as usize],
                ]
            }));
        }
    }
    // Only the chunks over the box are searched for the terrain's height.
    let near: Vec<ChunkMesh> = chunks
        .into_iter()
        .filter(|c| {
            c.positions.first().is_some_and(|p| {
                p[0] >= min[0] - 40.0
                    && p[0] <= max[0] + 40.0
                    && p[1] >= min[1] - 40.0
                    && p[1] <= max[1] + 40.0
            })
        })
        .collect();
    let terrain = |x: f32, y: f32| terrain_height_at(&near, [x, y, 0.0]);
    let in_box = |t: &[[f32; 3]; 3]| {
        t.iter()
            .any(|p| p[0] >= min[0] && p[0] <= max[0] && p[1] >= min[1] && p[1] <= max[1])
    };
    liquid.retain(|t| in_box(t));
    let ground = Layers::build(
        min,
        max,
        cell,
        &Sources {
            terrain: &terrain,
            solid: &solid,
            liquid: &liquid,
        },
    );
    Ok(LoadedDeadmines {
        ground,
        models,
        props,
        min,
        max,
    })
}
