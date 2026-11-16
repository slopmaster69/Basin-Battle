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

pub mod aftermath;

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
/// Pieces Elwynn Survival adds to the Deadmines' collision, triangles in WoW coordinates. The
/// client spawns the same list (benilla patch, `terrain_stream.rs`, `DEADMINES_BRIDGES`): change
/// both together, so the server's floors and the player's feet agree.
/// 1. The gangway off the Juggernaut's gun deck onto the plank down to Mr. Smite's dock. The data
///    leaves a 5 yd hole between the deck's edge (z 30-31, along (-32.6, -798.3)–(-37, -802.6))
///    and the plank's top (z 30.5-31.6, along (-30.3, -803.2)–(-34.9, -807.4)), over a trough at
///    z 27.7; players had to jump it (Corey, 7 Nov 2026). The first slab joined only the middle
///    4 yd of the two edges, and stepping off at either end still fell in (Corey, 8 Nov): this one
///    joins the whole of both, a strip of six triangles between points measured on each edge.
pub const DEADMINES_BRIDGES: [[[f32; 3]; 3]; 6] = [
    [[-32.6, -798.3, 30.4], [-33.6, -800.1, 30.1], [-31.6, -804.2, 31.6]],
    [[-32.6, -798.3, 30.4], [-31.6, -804.2, 31.6], [-30.3, -803.2, 31.4]],
    [[-33.6, -800.1, 30.1], [-35.0, -802.3, 30.2], [-33.3, -805.6, 31.3]],
    [[-33.6, -800.1, 30.1], [-33.3, -805.6, 31.3], [-31.6, -804.2, 31.6]],
    [[-35.0, -802.3, 30.2], [-37.0, -802.6, 31.2], [-34.9, -807.4, 30.5]],
    [[-35.0, -802.3, 30.2], [-34.9, -807.4, 30.5], [-33.3, -805.6, 31.3]],
];

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

/// What a straight look up and down from a spot meets on an instance map: its terrain, and the
/// floors and ceilings of its buildings' collision. For checking where heroes are set down: a
/// spot on the terrain under a building's own floor (Shadowfang Keep's first lamp, Nov 2026,
/// 4.5 yards under the gatehouse's floor) puts a hero under the floor, which the land's floors
/// can't tell, as that terrain is a floor of its own there.
pub struct Columns {
    chunks: Vec<ChunkMesh>,
    tris: Vec<[[f32; 3]; 3]>,
    /// The triangles over each `BUCKET`-yard square (by its corner's index), so a look up and
    /// down reads only those near it.
    near: std::collections::HashMap<(i32, i32), Vec<u32>>,
}

/// The side of [`Columns`]' squares of triangles, in yards.
const BUCKET: f32 = 4.0;

/// How far over a spot a building's floor seen from under it still seals the spot in (Baron
/// Silverlaine stood 15 yards under Shadowfang Keep's floor, Nov 2026).
const SEALED_REACH: f32 = 20.0;

impl Columns {
    /// An instance map's terrain and buildings (no props: what's checked is the floor itself).
    pub fn load(data_dir: &Path, map_dir: &str) -> Result<Self> {
        let mut chain = Chain::open(data_dir)
            .with_context(|| format!("opening the game data in {}", data_dir.display()))?;
        let tiles = tiles_of(&mut chain, map_dir)?;
        let mut seen = std::collections::HashSet::new();
        let mut tris = Vec::new();
        for w in tiles.iter().flat_map(|t| &t.wmos) {
            if !seen.insert(w.unique_id) {
                continue;
            }
            let Ok(mesh) = load_wmo_collision_tris(&mut chain, &w.model) else { continue };
            let m = placement_matrix(w.rotation);
            let p: Vec<[f32; 3]> = mesh.positions.iter().map(|&v| transform(&m, w.position, v)).collect();
            tris.extend(mesh.indices.chunks_exact(3).map(|c| [p[c[0] as usize], p[c[1] as usize], p[c[2] as usize]]));
        }
        let chunks = tiles.into_iter().flat_map(|t| t.chunks).collect();
        Ok(Self::from_parts(chunks, tris))
    }

    /// Made of these pieces (for tests).
    pub fn from_parts(chunks: Vec<ChunkMesh>, tris: Vec<[[f32; 3]; 3]>) -> Self {
        let mut near: std::collections::HashMap<(i32, i32), Vec<u32>> = Default::default();
        for (k, t) in tris.iter().enumerate() {
            let lo = |a: usize| (t.iter().map(|p| p[a]).fold(f32::MAX, f32::min) / BUCKET).floor() as i32;
            let hi = |a: usize| (t.iter().map(|p| p[a]).fold(f32::MIN, f32::max) / BUCKET).floor() as i32;
            for i in lo(0)..=hi(0) {
                for j in lo(1)..=hi(1) {
                    near.entry((i, j)).or_default().push(k as u32);
                }
            }
        }
        Self { chunks, tris, near }
    }

    /// The terrain's height at (x, y), if there is terrain there (not a hole).
    pub fn terrain(&self, x: f32, y: f32) -> Option<f32> {
        terrain_height_at(&self.chunks, [x, y, 0.0])
    }

    /// The buildings' faces over (x, y), lowest first: each one's height and whether it faces up
    /// (a floor) or down (a ceiling). Walls are left out.
    pub fn faces(&self, x: f32, y: f32) -> Vec<(f32, bool)> {
        let mut out = Vec::new();
        let key = ((x / BUCKET).floor() as i32, (y / BUCKET).floor() as i32);
        for &k in self.near.get(&key).map_or(&[][..], |v| v.as_slice()) {
            let t = &self.tris[k as usize];
            let (a, b, c) = (t[0], t[1], t[2]);
            let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
            if det.abs() < 1e-9 {
                continue;
            }
            let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / det;
            let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / det;
            let l3 = 1.0 - l1 - l2;
            if l1 < 0.0 || l2 < 0.0 || l3 < 0.0 {
                continue;
            }
            let (u, v) = ([b[0] - a[0], b[1] - a[1], b[2] - a[2]], [c[0] - a[0], c[1] - a[1], c[2] - a[2]]);
            let n = [u[1] * v[2] - u[2] * v[1], u[2] * v[0] - u[0] * v[2], u[0] * v[1] - u[1] * v[0]];
            let nz = n[2] / (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
            if nz.abs() >= 0.6 {
                out.push((l1 * a[2] + l2 * b[2] + l3 * c[2], nz > 0.0));
            }
        }
        out.sort_by(|p, q| p.0.total_cmp(&q.0));
        out
    }

    /// What a hero set down at `at` stands on: the terrain or a building's floor, at most 1.5
    /// yards under their feet (or a hair over them).
    pub fn stands_on(&self, at: [f32; 3]) -> Option<f32> {
        let near = |z: f32| z <= at[2] + 0.6 && z >= at[2] - 1.5;
        let floor = self.faces(at[0], at[1]).into_iter().filter(|&(z, up)| up && near(z)).map(|f| f.0).reduce(f32::max);
        let ground = self.terrain(at[0], at[1]).filter(|&z| near(z));
        match (floor, ground) {
            (Some(a), Some(b)) => Some(a.max(b)),
            (a, b) => a.or(b),
        }
    }

    /// A building's floor seen from under it, just over `at`, with no ceiling between: the spot
    /// is under that floor (in the hollow between it and the terrain), not on it.
    pub fn sealed_over(&self, at: [f32; 3]) -> Option<f32> {
        let over = self.faces(at[0], at[1]).into_iter().find(|&(z, _)| z > at[2] + 0.6 && z < at[2] + SEALED_REACH)?;
        over.1.then_some(over.0)
    }

    /// Whether a floor at `at` is the terrain itself, not a building's floor on or just over it.
    pub fn on_terrain(&self, at: [f32; 3]) -> bool {
        self.terrain(at[0], at[1]).is_some_and(|t| (t - at[2]).abs() < 0.05)
            && !self.faces(at[0], at[1]).iter().any(|&(z, up)| up && (z - at[2]).abs() < 0.4)
    }

    /// Whether a floor at `at` is the terrain in the hollow under a building's floor
    /// ([`Self::sealed_over`]): no hero gets there in the game, as the floor over it is all they
    /// walk on.
    pub fn in_hollow(&self, at: [f32; 3]) -> bool {
        self.on_terrain(at) && self.sealed_over(at).is_some()
    }

    /// Whether a floor at `at` is the terrain out from under every building: nothing of theirs
    /// over it or under it (a hillside outside a keep's walls).
    pub fn outdoors(&self, at: [f32; 3]) -> bool {
        self.on_terrain(at) && self.faces(at[0], at[1]).is_empty()
    }

    /// Each of `ground`'s floors (by floor index) that a walk inside the dungeon's buildings keeps
    /// off: the terrain in a hollow under their floors ([`Self::in_hollow`]), which the server's
    /// ground has as a floor of its own, and with `indoors` the terrain outside them too
    /// ([`Self::outdoors`]). Shadowfang Keep's walks to Fenrus and Wolf Master Nandos ran under the
    /// courtyard's floor, or out of the kitchen and round the hillside outside the walls, until
    /// Nov 2026.
    pub fn off_limits(&self, ground: &Layers, indoors: bool) -> Vec<bool> {
        let mut out = vec![false; ground.floor_count()];
        let (w, h) = ground.size();
        for j in 0..h {
            for i in 0..w {
                let r = ground.floor_range(i, j);
                if r.is_empty() {
                    continue;
                }
                let [x, y] = ground.cell_xy(i, j);
                let Some(t) = self.terrain(x, y) else { continue };
                for f in r {
                    let at = [x, y, ground.floor(f).z];
                    out[f] = (t - at[2]).abs() < 0.05 && (self.in_hollow(at) || (indoors && self.outdoors(at)));
                }
            }
        }
        out
    }
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
    load_variant(data_dir, map_dir, cell, window, false, false, None)
}

/// The finished Deadmines: the Juggernaut, its cargo and its added gangway are absent.
/// The client applies the same group mask in its aftermath WMO asset (patch 0087).
pub fn load_aftermath(data_dir: &Path) -> Result<LoadedDeadmines> {
    load_variant(data_dir, MAP_DIR, CELL, None, true, false, None)
}

/// [`load_aftermath`] with the shapes themselves kept: every solid triangle (walls, ceilings,
/// floors, the hulls of what stands in the mine) and where every native doodad still standing
/// is, for seating the ruins against the real rock and keeping them out of what's already there.
pub fn load_aftermath_survey(data_dir: &Path) -> Result<aftermath::Survey> {
    let mut kept = aftermath::Kept::default();
    let loaded = load_variant(data_dir, MAP_DIR, CELL, None, true, false, Some(&mut kept))?;
    Ok(aftermath::Survey::new(loaded, kept))
}

/// [`load_aftermath_survey`] as a client with patch 0091 has it: the mine's own doodads in
/// [`aftermath::MINE_DOODADS`] are gone too (their bodies, their hulls and the floors they gave),
/// and [`aftermath::Survey::hidden`] says where each stood, for what the server sets there instead.
pub fn load_aftermath_open_survey(data_dir: &Path) -> Result<aftermath::Survey> {
    let mut kept = aftermath::Kept::default();
    let loaded = load_variant(data_dir, MAP_DIR, CELL, None, true, true, Some(&mut kept))?;
    Ok(aftermath::Survey::new(loaded, kept))
}

fn load_variant(
    data_dir: &Path,
    map_dir: &str,
    cell: f32,
    window: Option<([f32; 2], [f32; 2])>,
    aftermath: bool,
    // The aftermath's mine doodads gone as well (patch 0091).
    open: bool,
    mut kept: Option<&mut aftermath::Kept>,
) -> Result<LoadedDeadmines> {
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
            let ship_removed = aftermath && aftermath::is_ship_root(&w.model);
            let mesh = if ship_removed {
                aftermath::collision(&mut chain, &w.model)?
            } else {
                let Ok(mesh) = load_wmo_collision_tris(&mut chain, &w.model) else { continue };
                mesh
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
                        if ship_removed && k < aftermath::SHIP_DOODADS {
                            continue;
                        }
                        let Some(d) = root.doodads().get(k) else { continue };
                        if d.model.is_empty() {
                            continue;
                        }
                        // The mine's own doodads are numbered by their place in its WMO.
                        let index = aftermath::is_ship_root(&w.model).then_some(k as u32);
                        if open && index.is_some_and(|i| aftermath::MINE_DOODADS.contains(&i)) {
                            if let Some(k) = kept.as_deref_mut() {
                                let bounds = k.bounds(&mut chain, &d.model);
                                k.hidden.push(aftermath::Standing::in_wmo(&d.model, bounds, &m, w.position, d.orientation, d.position, d.scale, index));
                            }
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
                        if let Some(k) = kept.as_deref_mut() {
                            let bounds = k.bounds(&mut chain, &d.model);
                            k.standing.push(aftermath::Standing::in_wmo(&d.model, bounds, &m, w.position, d.orientation, d.position, d.scale, index));
                        }
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
                if ship_removed && aftermath::SHIP_GROUPS.contains(&gi) {
                    continue;
                }
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
                if d.model.is_empty() || !placed.insert(d.unique_id)
                    || (aftermath && aftermath::SHIP_ADT_DOODADS.contains(&d.unique_id))
                {
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
                let m = placement_matrix(d.rotation);
                if let Some(k) = kept.as_deref_mut() {
                    let bounds = k.bounds(&mut chain, &d.model);
                    k.standing.push(aftermath::Standing::on_terrain(&d.model, bounds, &m, d.position, d.scale));
                }
                if hull.is_empty() {
                    continue;
                }
                let pts: Vec<[f32; 3]> = hull
                    .positions
                    .iter()
                    .map(|&v| transform(&m, d.position, [v[0] * d.scale, v[1] * d.scale, v[2] * d.scale]))
                    .collect();
                solid.extend(hull.indices.chunks_exact(3).map(|c| [pts[c[0] as usize], pts[c[1] as usize], pts[c[2] as usize]]));
            }
        }
    }
    // Our own pieces, where the data leaves a gap (`DEADMINES_BRIDGES`).
    if map_dir == MAP_DIR && !aftermath {
        solid.extend(DEADMINES_BRIDGES);
    }
    let (min, max) = window.unwrap_or(([lo[0] - MARGIN, lo[1] - MARGIN], [hi[0] + MARGIN, hi[1] + MARGIN]));
    if let Some(k) = kept.as_deref_mut() {
        k.solid = solid.clone();
        k.box_ = (min, max);
    }
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

#[cfg(test)]
mod tests {
    use super::Columns;

    /// A square of two triangles at height z over 0..10, facing up (or down).
    fn quad(z: f32, up: bool) -> Vec<[[f32; 3]; 3]> {
        let (a, b, c, d) = ([0.0, 0.0, z], [10.0, 0.0, z], [10.0, 10.0, z], [0.0, 10.0, z]);
        if up {
            vec![[a, b, c], [a, c, d]]
        } else {
            vec![[a, c, b], [a, d, c]]
        }
    }

    #[test]
    fn a_spot_under_a_buildings_floor_is_sealed_in_and_one_on_it_is_not() {
        // A raised floor at 5 with no underside (a building's floor seen from the hollow under
        // it, as Shadowfang Keep's gatehouse over the terrain), and a room's ceiling at 9.
        let mut tris = quad(5.0, true);
        tris.extend(quad(9.0, false));
        let cols = Columns::from_parts(Vec::new(), tris);
        assert_eq!(cols.sealed_over([5.0, 5.0, 0.5]), Some(5.0), "under the floor");
        assert_eq!(cols.sealed_over([5.0, 5.0, 5.1]), None, "on it: the ceiling is over it");
        assert_eq!(cols.stands_on([5.0, 5.0, 5.4]), Some(5.0));
        assert_eq!(cols.stands_on([5.0, 5.0, 0.5]), None, "nothing to stand on down there");
        // Off the building's footprint: nothing over or under.
        assert_eq!(cols.sealed_over([20.0, 5.0, 0.5]), None);
        // A deck with its own underside is a ceiling over what's under it, not a seal.
        let mut deck = quad(5.0, true);
        deck.extend(quad(4.7, false));
        let cols = Columns::from_parts(Vec::new(), deck);
        assert_eq!(cols.sealed_over([5.0, 5.0, 0.5]), None);
    }
}
