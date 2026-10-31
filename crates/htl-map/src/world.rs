//! What a walker stands on: terrain, plus building and bridge floors, with walls and deep water
//! marked. Built once at startup from the arena's terrain, buildings (WMOs) and liquids.

use benilla_formats::{load_wmo_collision_tris, Chain, ChunkMesh, TileMesh, ALPHA_MAP_SIZE};

use crate::grid::HeightGrid;

/// A floor this far above the terrain still counts (a bridge deck over a river bed); anything
/// higher is a roof or an upper storey and is ignored.
const MAX_FLOOR_ABOVE_TERRAIN: f32 = 12.0;
/// A bridge deck over water stands at most this far above the water.
const MAX_DECK_ABOVE_WATER: f32 = 10.0;
/// A wall between these heights above a floor blocks walking.
const WALL_FROM: f32 = 0.5;
const WALL_TO: f32 = 2.2;
/// Water deeper than this is "deep": walkable, but paths avoid it.
const DEEP_WATER: f32 = 1.2;

/// A placement rotation as a 3x3 matrix in WoW space: wowdev's `createPlacementMatrix`,
/// `Rx(90°)·Ry(ry−180°)·Rz(−rx)·Rx(rz−90°)`, the same benilla's renderer uses.
pub fn placement_matrix(rotation_deg: [f32; 3]) -> [[f32; 3]; 3] {
    use std::f32::consts::{FRAC_PI_2, PI};
    let (rx, ry, rz) = (
        rotation_deg[0].to_radians(),
        rotation_deg[1].to_radians(),
        rotation_deg[2].to_radians(),
    );
    let rot_x = |a: f32| {
        let (s, c) = a.sin_cos();
        [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
    };
    let rot_y = |a: f32| {
        let (s, c) = a.sin_cos();
        [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
    };
    let rot_z = |a: f32| {
        let (s, c) = a.sin_cos();
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    };
    let m = mul(rot_x(FRAC_PI_2), rot_y(ry - PI));
    let m = mul(m, rot_z(-rx));
    mul(m, rot_x(rz - FRAC_PI_2))
}

fn mul(a: [[f32; 3]; 3], b: [[f32; 3]; 3]) -> [[f32; 3]; 3] {
    let mut r = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            r[i][j] = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    r
}

pub fn transform(m: &[[f32; 3]; 3], pos: [f32; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2] + pos[0],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2] + pos[1],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2] + pos[2],
    ]
}

/// One building's walking-collision triangles, in world space.
pub struct Building {
    pub model: String,
    pub tris: Vec<[[f32; 3]; 3]>,
}

/// Load every building placed on `tiles` within the box, deduplicated.
pub fn load_buildings(chain: &mut Chain, tiles: &[TileMesh], min: [f32; 2], max: [f32; 2]) -> Vec<Building> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for t in tiles {
        for w in &t.wmos {
            if !seen.insert(w.unique_id) {
                continue;
            }
            let Ok(mesh) = load_wmo_collision_tris(chain, &w.model) else {
                continue;
            };
            let m = placement_matrix(w.rotation);
            let world: Vec<[f32; 3]> = mesh
                .positions
                .iter()
                .map(|&v| transform(&m, w.position, v))
                .collect();
            let tris: Vec<[[f32; 3]; 3]> = mesh
                .indices
                .chunks_exact(3)
                .map(|c| [world[c[0] as usize], world[c[1] as usize], world[c[2] as usize]])
                .filter(|t| {
                    t.iter().any(|p| p[0] >= min[0] && p[0] <= max[0] && p[1] >= min[1] && p[1] <= max[1])
                })
                .collect();
            if !tris.is_empty() {
                out.push(Building {
                    model: w.model.clone(),
                    tris,
                });
            }
        }
    }
    out
}

fn normal_z(t: &[[f32; 3]; 3]) -> f32 {
    let (a, b, c) = (t[0], t[1], t[2]);
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    let n = [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ];
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
    n[2] / len
}

/// The z of triangle `t` above (x, y), if (x, y) is inside its footprint.
fn z_at(t: &[[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let (a, b, c) = (t[0], t[1], t[2]);
    let det = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if det.abs() < 1e-9 {
        return None;
    }
    let l1 = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / det;
    let l2 = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / det;
    let l3 = 1.0 - l1 - l2;
    let eps = -1e-4;
    (l1 >= eps && l2 >= eps && l3 >= eps).then(|| l1 * a[2] + l2 * b[2] + l3 * c[2])
}

/// Layers on top of the terrain grid: floors raised by buildings and bridges, walls, deep water.
pub fn apply(grid: &mut HeightGrid, buildings: &[Building], chunks: &[ChunkMesh]) {
    apply_with(grid, buildings, chunks, MAX_FLOOR_ABOVE_TERRAIN);
}

/// [`apply`], a floor counting only up to `max_floor` above the terrain (a bridge's deck over
/// water whatever it is): low, and it's a building's ground floor, not its roof.
pub fn apply_with(grid: &mut HeightGrid, buildings: &[Building], chunks: &[ChunkMesh], max_floor: f32) {
    let (w, h) = grid.size();
    let terrain: Vec<f32> = grid.heights().to_vec();
    let mut floor = terrain.clone();
    let mut walls: Vec<Vec<f32>> = vec![Vec::new(); w * h];
    let cell = grid.cell();

    // Water surfaces from the terrain's liquid layers (NaN where there is none).
    let mut surface = vec![f32::NAN; w * h];
    for c in chunks {
        for l in &c.liquids {
            for tri in l.indices.chunks_exact(3) {
                let t = [
                    l.positions[tri[0] as usize],
                    l.positions[tri[1] as usize],
                    l.positions[tri[2] as usize],
                ];
                let (lo, hi) = grid.cell_range(&t);
                for j in lo.1..=hi.1 {
                    for i in lo.0..=hi.0 {
                        let [x, y] = grid.cell_xy(i, j);
                        if let Some(z) = z_at(&t, x, y) {
                            let k = j * w + i;
                            if surface[k].is_nan() || z > surface[k] {
                                surface[k] = z;
                            }
                        }
                    }
                }
            }
        }
    }

    for b in buildings {
        for t in &b.tris {
            let nz = normal_z(t);
            let (lo, hi) = grid.cell_range(t);
            if nz > 0.6 {
                // A floor: raise cells to it, if it is low enough to be walked onto.
                for j in lo.1..=hi.1 {
                    for i in lo.0..=hi.0 {
                        let [x, y] = grid.cell_xy(i, j);
                        if let Some(z) = z_at(t, x, y) {
                            let k = j * w + i;
                            // A bridge deck over a deep river stands high above the river bed,
                            // but not far above the water.
                            let over_water = surface[k].is_finite()
                                && z >= surface[k] - 0.5
                                && z <= surface[k] + MAX_DECK_ABOVE_WATER;
                            if (z <= terrain[k] + max_floor || over_water) && z > floor[k] {
                                floor[k] = z;
                            }
                        }
                    }
                }
            } else if nz.abs() <= 0.6 {
                // A wall: sample its surface and mark the cells it stands in.
                let n = (edge_max(t) / (cell * 0.5)).ceil().max(1.0) as usize;
                for a in 0..=n {
                    for bb in 0..=(n - a) {
                        let (fa, fb) = (a as f32 / n as f32, bb as f32 / n as f32);
                        let fc = 1.0 - fa - fb;
                        let p = [
                            t[0][0] * fa + t[1][0] * fb + t[2][0] * fc,
                            t[0][1] * fa + t[1][1] * fb + t[2][1] * fc,
                            t[0][2] * fa + t[1][2] * fb + t[2][2] * fc,
                        ];
                        if let Some((i, j)) = grid.cell_at(p[0], p[1]) {
                            walls[j * w + i].push(p[2]);
                        }
                    }
                }
            }
        }
    }
    let blocked: Vec<bool> = (0..w * h)
        .map(|k| walls[k].iter().any(|&z| z > floor[k] + WALL_FROM && z < floor[k] + WALL_TO))
        .collect();

    // Deep water, and any water at all, over the floor; and bridge decks above it.
    let mut water = vec![false; w * h];
    let mut wet = vec![false; w * h];
    for k in 0..w * h {
        if surface[k].is_finite() {
            water[k] = surface[k] > floor[k] + DEEP_WATER;
            wet[k] = surface[k] > floor[k] + 0.05;
        }
    }
    // A bridge deck: a floor well above the ground, with water close by (the river's surface
    // can have a hole right under the bridge).
    let near = 6i32;
    let deck: Vec<bool> = (0..w * h)
        .map(|k| {
            if floor[k] <= terrain[k] + 2.0 {
                return false;
            }
            let (i, j) = ((k % w) as i32, (k / w) as i32);
            (-near..=near).any(|dj| {
                (-near..=near).any(|di| {
                    let (a, b) = (i + di, j + dj);
                    a >= 0 && b >= 0 && a < w as i32 && b < h as i32 && surface[b as usize * w + a as usize].is_finite()
                })
            })
        })
        .collect();
    grid.set_layers(floor, blocked, water, wet);
    grid.set_deck(deck);
}

fn edge_max(t: &[[f32; 3]; 3]) -> f32 {
    let d = |a: [f32; 3], b: [f32; 3]| ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
    d(t[0], t[1]).max(d(t[1], t[2])).max(d(t[2], t[0]))
}

/// A terrain chunk's side, in yards.
const CHUNK: f32 = 100.0 / 3.0;

/// How much each texture layer shows at (x, y) in a chunk, layer 0 first; `None` off the chunk.
pub fn layer_weights(c: &ChunkMesh, x: f32, y: f32) -> Option<Vec<f32>> {
    let [wx, wy, _] = *c.positions.first()?;
    // Rows step south (WoW -x), columns east (WoW -y).
    let (v, u) = ((wx - x) / CHUNK, (wy - y) / CHUNK);
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return None;
    }
    let n = c.layer_textures.len();
    let mut a = vec![0.0f32; n];
    if let Some(map) = &c.alpha_map {
        let s = ALPHA_MAP_SIZE as usize;
        let (col, row) = (((u * s as f32) as usize).min(s - 1), ((v * s as f32) as usize).min(s - 1));
        let t = (row * s + col) * 4;
        for l in 1..n.min(4) {
            a[l] = map[t + l - 1] as f32 / 255.0;
        }
    }
    // Each layer paints over the ones below it.
    let mut w = vec![0.0f32; n];
    let mut rest = 1.0;
    for l in (1..n).rev() {
        w[l] = a[l] * rest;
        rest *= 1.0 - a[l];
    }
    if n > 0 {
        w[0] = rest;
    }
    Some(w)
}

/// The paved roads: how much of each cell shows a texture whose name holds `road` (Arathi's
/// roads are its rubble texture), widened a little so a lane keeps to the middle of the road.
pub fn mark_roads(grid: &mut HeightGrid, chunks: &[ChunkMesh], road: &str) {
    let (w, h) = grid.size();
    let road = road.to_lowercase();
    let mut out = vec![0.0f32; w * h];
    let mut last: Option<usize> = None;
    // Chunks by the corner they start at (a whole zone's are too many to walk for each cell).
    let key = |x: f32, y: f32| ((x / CHUNK).round() as i32, (y / CHUNK).round() as i32);
    let mut by_corner: std::collections::HashMap<(i32, i32), usize> = std::collections::HashMap::new();
    for (k, c) in chunks.iter().enumerate() {
        if let Some(p) = c.positions.first() {
            by_corner.entry(key(p[0], p[1])).or_insert(k);
        }
    }
    for j in 0..h {
        for i in 0..w {
            let [x, y] = grid.cell_xy(i, j);
            let inside = |c: &ChunkMesh| {
                let [wx, wy, _] = c.positions[0];
                x <= wx && x > wx - CHUNK && y <= wy && y > wy - CHUNK
            };
            let k = match last.filter(|&k| inside(&chunks[k])) {
                Some(k) => Some(k),
                None => {
                    // The chunk whose corner is the next lattice point north-west of the cell.
                    let (cx, cy) = ((x / CHUNK).ceil() * CHUNK, (y / CHUNK).ceil() * CHUNK);
                    let near = [(0.0, 0.0), (CHUNK, 0.0), (0.0, CHUNK), (CHUNK, CHUNK), (-CHUNK, 0.0), (0.0, -CHUNK)];
                    near.iter()
                        .filter_map(|(dx, dy)| by_corner.get(&key(cx + dx, cy + dy)).copied())
                        .find(|&k| !chunks[k].positions.is_empty() && inside(&chunks[k]))
                        .or_else(|| chunks.iter().position(|c| !c.positions.is_empty() && inside(c)))
                }
            };
            last = k;
            let Some(c) = k.map(|k| &chunks[k]) else { continue };
            let Some(ws) = layer_weights(c, x, y) else { continue };
            out[j * w + i] = c
                .layer_textures
                .iter()
                .zip(&ws)
                .filter(|(t, _)| t.to_lowercase().contains(&road))
                .map(|(_, v)| v)
                .sum::<f32>()
                .min(1.0);
        }
    }
    // Patchy texture paint: a cell counts as road as much as the best cell within 2 yards.
    let r = (2.0 / grid.cell()).ceil() as i32;
    let mut wide = out.clone();
    for j in 0..h as i32 {
        for i in 0..w as i32 {
            let mut best = 0.0f32;
            for dj in -r..=r {
                for di in -r..=r {
                    let (a, b) = (i + di, j + dj);
                    if a >= 0 && b >= 0 && a < w as i32 && b < h as i32 && di * di + dj * dj <= r * r {
                        best = best.max(out[b as usize * w + a as usize]);
                    }
                }
            }
            wide[j as usize * w + i as usize] = (best * 1.6).min(1.0);
        }
    }
    grid.set_road(wide);
}
