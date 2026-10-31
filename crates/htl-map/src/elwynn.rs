//! Elwynn Survival's world, read from the host's own 1.12.1 install: the whole of Elwynn Forest,
//! from Northshire to the Westfall bridge and the Redridge road, with its buildings as floors and
//! walls, water, the roads painted on the ground, and the trees as solid trunks. Nothing read here
//! leaves the machine.

use std::path::Path;

use anyhow::{Context, Result};
use benilla_formats::{area_id_at, load_area_table_catalog, load_tile_mesh, terrain_height_at, Chain, ChunkMesh, TileMesh};

use crate::grid::HeightGrid;
use crate::world;

/// Eastern Kingdoms.
pub const MAP_ID: u32 = 0;
pub const MAP_DIR: &str = "Azeroth";
/// Elwynn Forest, and Goldshire within it.
pub const ZONE_ID: u32 = 12;
pub const AREA_ID: u32 = 87;
/// The world the game is played in: the zone, Northshire's head to Forest's Edge, the Westfall
/// bridge to the Redridge road (Stormwind's walls stand at its north-west corner).
pub const MIN: [f32; 2] = [-10160.0, -1700.0];
pub const MAX: [f32; 2] = [-8580.0, 960.0];
/// Buildings not read: the city (a wall's worth of it at the corner is not worth its weight).
const SKIP: [&str; 1] = ["stormwind\\stormwind.wmo"];
const CELL: f32 = 1.0;
/// A floor counts this far above the terrain at most: a house's own floor (a step or two up),
/// so its inside is walked, not its roof (a bridge's deck over the water whatever its height).
const MAX_FLOOR: f32 = 3.5;
/// Elwynn's roads, as its ground texture names them.
pub const ROAD_TEXTURE: &str = "ElwynnCobbleStone";

/// A stretch of a map read as a world: which map, its box, which zone is its own, the buildings
/// left out, its roads' texture, how high a floor counts.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    pub map_dir: &'static str,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub zone_id: u32,
    pub skip: &'static [&'static str],
    pub road_texture: &'static str,
    pub max_floor: f32,
}

/// Elwynn Forest.
pub const ELWYNN: Region = Region { map_dir: MAP_DIR, min: MIN, max: MAX, zone_id: ZONE_ID, skip: &SKIP, road_texture: ROAD_TEXTURE, max_floor: MAX_FLOOR };

/// A tree trunk: where, and how wide.
#[derive(Debug, Clone, Copy)]
pub struct Trunk {
    pub pos: [f32; 3],
    pub radius: f32,
}

pub struct LoadedElwynn {
    /// The box read.
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub grid: HeightGrid,
    pub trunks: Vec<Trunk>,
    /// Which ground is Elwynn's.
    pub zone: ZoneMask,
    /// The caves and mines (under the hills, where the ground has floors below floors).
    pub caves: Vec<Cave>,
}

/// A cave or a mine: its own ground with floors below floors (its tunnels under the hill's
/// slope), over its box (the model and a margin of the land round its mouth).
pub struct Cave {
    pub model: String,
    pub min: [f32; 2],
    pub max: [f32; 2],
    pub ground: crate::layers::Layers,
}

/// The land kept round a cave's model in its own ground.
const CAVE_MARGIN: f32 = 12.0;

const TILE: f32 = 1600.0 / 3.0;

/// A map chunk's side: the terrain names each chunk's area.
pub const ZONE_CELL: f32 = TILE / 16.0;

/// Whether each map chunk ([`ZONE_CELL`] yards a side, from [`MIN`], `x` rows then `y`) is
/// Elwynn's own (not Stormwind's, Westfall's, Duskwood's nor Redridge's ground).
#[derive(Debug, Clone, Default)]
pub struct ZoneMask {
    cells: Vec<bool>,
    dims: (usize, usize),
    origin: [f32; 2],
}

impl ZoneMask {
    /// Whether the ground there is the zone's.
    pub fn contains(&self, x: f32, y: f32) -> bool {
        let (i, j) = (((x - self.origin[0]) / ZONE_CELL).floor(), ((y - self.origin[1]) / ZONE_CELL).floor());
        if i < 0.0 || j < 0.0 || i as usize >= self.dims.0 || j as usize >= self.dims.1 {
            return false;
        }
        self.cells[i as usize * self.dims.1 + j as usize]
    }

    /// How many chunks are Elwynn's.
    pub fn count(&self) -> usize {
        self.cells.iter().filter(|c| **c).count()
    }
}

/// The tiles that cover the play area, each with its place on the map's grid.
fn tiles_at(chain: &mut Chain, r: &Region) -> Result<Vec<((u32, u32), TileMesh)>> {
    let off = 32.0 * TILE;
    let (tx0, tx1) = (((off - r.max[1]) / TILE).floor() as u32, ((off - r.min[1]) / TILE).floor() as u32);
    let (ty0, ty1) = (((off - r.max[0]) / TILE).floor() as u32, ((off - r.min[0]) / TILE).floor() as u32);
    let mut out = Vec::new();
    for tx in tx0..=tx1 {
        for ty in ty0..=ty1 {
            if let Ok(t) = load_tile_mesh(chain, r.map_dir, tx, ty) {
                out.push(((tx, ty), t));
            }
        }
    }
    anyhow::ensure!(!out.is_empty(), "reading {}'s terrain: no tiles", r.map_dir);
    Ok(out)
}

/// The tiles that cover the play area.
pub fn tiles(chain: &mut Chain) -> Result<Vec<TileMesh>> {
    Ok(tiles_at(chain, &ELWYNN)?.into_iter().map(|(_, t)| t).collect())
}

pub fn load(data_dir: &Path) -> Result<LoadedElwynn> {
    load_region(data_dir, &ELWYNN)
}

/// Any stretch of a map, read as Elwynn is: heights, buildings as floors and walls, caves, roads
/// and trunks.
pub fn load_region(data_dir: &Path, r: &Region) -> Result<LoadedElwynn> {
    let (min_, max_, zone_id, skip, max_floor, road_texture) = (r.min, r.max, r.zone_id, r.skip, r.max_floor, r.road_texture);
    let mut chain = Chain::open(data_dir).with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let placed = tiles_at(&mut chain, r)?;
    // Each tile's own chunks, by its place: a height is looked up in its tile, not the whole land.
    let by_tile: std::collections::HashMap<(u32, u32), usize> = placed.iter().enumerate().map(|(k, (at, _))| (*at, k)).collect();
    let off = 32.0 * TILE;
    let height = |x: f32, y: f32| {
        let at = (((off - y) / TILE).floor() as u32, ((off - x) / TILE).floor() as u32);
        let t = by_tile.get(&at)?;
        terrain_height_at(&placed[*t].1.chunks, [x, y, 0.0])
    };
    let mut grid = HeightGrid::sample(min_, max_, CELL, height);
    // Which chunks are Elwynn's: its own area or one of its subzones.
    let table = load_area_table_catalog(&mut chain)?;
    let zone_dims = (((max_[0] - min_[0]) / ZONE_CELL).ceil() as usize, ((max_[1] - min_[1]) / ZONE_CELL).ceil() as usize);
    let mut zone = vec![false; zone_dims.0 * zone_dims.1];
    for i in 0..zone_dims.0 {
        for j in 0..zone_dims.1 {
            let (x, y) = (min_[0] + (i as f32 + 0.5) * ZONE_CELL, min_[1] + (j as f32 + 0.5) * ZONE_CELL);
            let at = (((off - y) / TILE).floor() as u32, ((off - x) / TILE).floor() as u32);
            let area = by_tile.get(&at).and_then(|t| area_id_at(&placed[*t].1.chunks, [x, y, 0.0]));
            zone[i * zone_dims.1 + j] = area.is_some_and(|a| a == zone_id || table.get(a).is_some_and(|r| r.zone_id == zone_id));
        }
    }
    let mut tiles: Vec<TileMesh> = placed.into_iter().map(|(_, t)| t).collect();
    for t in &mut tiles {
        t.wmos.retain(|w| !skip.iter().any(|s| w.model.to_lowercase().ends_with(s)));
    }
    let chunks: Vec<ChunkMesh> = tiles.iter().flat_map(|t| t.chunks.iter().cloned()).collect();
    let buildings = world::load_buildings(&mut chain, &tiles, min_, max_);
    world::apply_with(&mut grid, &buildings, &chunks, max_floor);
    // The caves: each its own ground, floors under the hill as well as the hill's slope.
    let mut caves = Vec::new();
    for b in buildings.iter().filter(|b| b.model.to_lowercase().contains("\\dungeon\\")) {
        let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
        for t in &b.tris {
            for p in t {
                for a in 0..2 {
                    lo[a] = lo[a].min(p[a]);
                    hi[a] = hi[a].max(p[a]);
                }
            }
        }
        let (min, max) = ([(lo[0] - CAVE_MARGIN).max(min_[0]), (lo[1] - CAVE_MARGIN).max(min_[1])], [(hi[0] + CAVE_MARGIN).min(max_[0]), (hi[1] + CAVE_MARGIN).min(max_[1])]);
        if min[0] >= max[0] || min[1] >= max[1] {
            continue;
        }
        let near: Vec<ChunkMesh> = chunks
            .iter()
            .filter(|c| c.positions.first().is_some_and(|p| p[0] >= min[0] - 40.0 && p[0] <= max[0] + 40.0 && p[1] >= min[1] - 40.0 && p[1] <= max[1] + 40.0))
            .cloned()
            .collect();
        let terrain = |x: f32, y: f32| terrain_height_at(&near, [x, y, 0.0]);
        let ground = crate::layers::Layers::build(min, max, CELL, &crate::layers::Sources { terrain: &terrain, solid: &b.tris, liquid: &[] });
        caves.push(Cave { model: b.model.clone(), min, max, ground });
    }
    world::mark_roads(&mut grid, &chunks, road_texture);
    let mut seen = std::collections::HashSet::new();
    let mut trunks = Vec::new();
    for t in &tiles {
        for d in &t.doodads {
            let m = d.model.to_lowercase();
            let inside = d.position[0] >= min_[0] && d.position[0] <= max_[0] && d.position[1] >= min_[1] && d.position[1] <= max_[1];
            if !inside || !seen.insert(d.unique_id) {
                continue;
            }
            // Trees and stumps stand in the way; bushes, grass and flowers don't.
            if m.contains("\\trees\\") || m.contains("tree") && !m.contains("bush") {
                let radius = if m.contains("stump") { 0.7 } else if m.contains("canopy") { 1.4 } else { 1.0 };
                trunks.push(Trunk { pos: d.position, radius: radius * d.scale.clamp(0.5, 2.0) });
            }
        }
    }
    Ok(LoadedElwynn { min: min_, max: max_, grid, trunks, zone: ZoneMask { cells: zone, dims: zone_dims, origin: min_ }, caves })
}
