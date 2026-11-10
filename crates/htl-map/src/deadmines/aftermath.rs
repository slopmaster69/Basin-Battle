//! The ship is part of the mine's WMO, separated by authored group indices. Keep the dock,
//! approach and every mine room; never remove an overlapping world-space box of floors.

use std::collections::HashMap;

use anyhow::{Context, Result};
use benilla_formats::{
    accumulate_wmo_group_collision, load_gameobject_catalog, load_object_model, parse_wmo_root, Chain, CollisionMesh,
};

use crate::solids::{Solids, Tri};
use crate::world::transform;

// Keep in step with the client's benilla-assets aftermath module (Basin Battle patch 0087).
pub(super) const SHIP_GROUPS: &[u32] = &[17, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 40, 43];
// MODR: shiphull (30) and hull_platforms (43) exclusively own indices 0 through 63.
pub(super) const SHIP_DOODADS: usize = 64;
// Independently placed MDDF fixtures attached to the hull, platforms and ramps.
// Native attachment pivots verified against the removed group triangles; preserve shore lights.
pub(super) const SHIP_ADT_DOODADS: &[u32] = &[
    293853, 293855, 293857, 293858, 293862, 293863, 293865, 293866,
    293867, 293868, 293869, 293871, 293872, 293873, 293874, 293875,
    293876, 293877, 293878, 293879, 293880, 293881, 294176, 294178,
    294179, 294180, 294182, 294183, 294184, 333883, 333884, 333885,
    333886, 333887, 333888, 333889, 333890, 333896, 333897, 333898,
    333899, 333900, 333901, 333902, 333903, 333904, 333905, 333906,
    333907, 333908, 333909, 333910, 333911, 333912, 333913, 333914,
    333915, 333916, 333917, 333918, 333919,
];
const SHIP_ROOT: &str = "world/wmo/dungeon/az_deadmines/az_deadmines_b.wmo";

// Keep in step with the client's benilla-assets aftermath module (Basin Battle patch 0091) and the
// server's plan for each (`aftermath_doodads.rs`, whose tests hold them to the same).
/// The mine's own doodads hidden while the aftermath preview's marker stands, by their place in
/// its WMO (MODD): the neat cargo stacks, carts and barrows the collapse knocked over, and the
/// lanterns it put out or left hanging askew. The server sets its own copies where they stood.
pub const MINE_DOODADS: &[u32] = &[
    67, 88, 89, 92, 93, 98, 101, 103, 104, 105, 106, 108, 120, 141, 162, 184, 185, 186, 220, 221,
    251, 252, 253, 254, 255, 256, 257, 259, 264, 283, 331, 332, 333, 342, 381, 386, 387, 502, 503,
    513,
];
/// Of those, the lanterns whose light goes out with them: the client puts out every light of
/// the mine's own (MOLT) within [`LANTERN_REACH`] of each.
pub const DARK_LANTERNS: &[u32] = &[
    98, 101, 106, 108, 120, 162, 184, 185, 220, 221, 283, 331, 332, 333, 381, 386, 387, 502, 503,
    513,
];
/// How far from a put-out lantern its own light hangs (yd): the mine's lanterns carry theirs 0.4
/// to 1 yd from their pivot, and no two lanterns' lights are this close to each other.
pub const LANTERN_REACH: f32 = 1.6;

/// What [`super::load_aftermath_survey`] keeps while it loads.
#[derive(Default)]
pub(super) struct Kept {
    pub(super) solid: Vec<Tri>,
    pub(super) standing: Vec<Standing>,
    pub(super) hidden: Vec<Standing>,
    pub(super) box_: ([f32; 2], [f32; 2]),
    bounds: HashMap<String, Option<([f32; 3], [f32; 3])>>,
}

impl Kept {
    /// A doodad model's mesh box in its own space (once per model).
    pub(super) fn bounds(&mut self, chain: &mut Chain, model: &str) -> Option<([f32; 3], [f32; 3])> {
        if let Some(b) = self.bounds.get(model) {
            return *b;
        }
        let b = model_triangles(chain, model).ok().and_then(|tris| bounds_of(&tris));
        self.bounds.insert(model.to_string(), b);
        b
    }
}

/// The finished Deadmines as shapes: its walking ground, every solid triangle bucketed for lines
/// and boxes to test, and the native doodads still standing in it.
pub struct Survey {
    pub loaded: super::LoadedDeadmines,
    pub solids: Solids,
    pub standing: Vec<Standing>,
    /// The mine's own doodads hidden in this survey ([`MINE_DOODADS`], an open survey's only),
    /// where they stood.
    pub hidden: Vec<Standing>,
}

impl Survey {
    pub(super) fn new(loaded: super::LoadedDeadmines, kept: Kept) -> Self {
        let (min, max) = kept.box_;
        Self { loaded, solids: Solids::build(min, max, 4.0, kept.solid), standing: kept.standing, hidden: kept.hidden }
    }
}

/// A native doodad of the mine (a lantern, a cart, a stack of cargo): its model, and where its
/// own space lands in the world, `world = origin + axes · v` (its scale included), with the
/// model's mesh box when the model loaded.
#[derive(Debug, Clone)]
pub struct Standing {
    pub model: String,
    pub origin: [f32; 3],
    pub axes: [[f32; 3]; 3],
    pub bounds: Option<([f32; 3], [f32; 3])>,
    /// Its place among the mine's own doodads (its WMO's MODD), when it is one of them.
    pub index: Option<u32>,
}

impl Standing {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn in_wmo(model: &str, bounds: Option<([f32; 3], [f32; 3])>, m: &[[f32; 3]; 3], wmo_at: [f32; 3], q: [f32; 4], at: [f32; 3], scale: f32, index: Option<u32>) -> Self {
        let origin = transform(m, wmo_at, at);
        let column = |c: usize| {
            let mut e = [0.0; 3];
            e[c] = scale;
            let turned = super::quat_rotate(q, e);
            transform(m, [0.0; 3], turned)
        };
        Self { index, ..Self::from_columns(model, bounds, origin, [column(0), column(1), column(2)]) }
    }

    pub(super) fn on_terrain(model: &str, bounds: Option<([f32; 3], [f32; 3])>, m: &[[f32; 3]; 3], at: [f32; 3], scale: f32) -> Self {
        let column = |c: usize| {
            let mut e = [0.0; 3];
            e[c] = scale;
            transform(m, [0.0; 3], e)
        };
        Self::from_columns(model, bounds, at, [column(0), column(1), column(2)])
    }

    fn from_columns(model: &str, bounds: Option<([f32; 3], [f32; 3])>, origin: [f32; 3], cols: [[f32; 3]; 3]) -> Self {
        let axes = [[cols[0][0], cols[1][0], cols[2][0]], [cols[0][1], cols[1][1], cols[2][1]], [cols[0][2], cols[1][2], cols[2][2]]];
        Self { model: model.to_string(), origin, axes, bounds, index: None }
    }

    /// A point of the model's own space, in the world.
    pub fn world(&self, v: [f32; 3]) -> [f32; 3] {
        transform(&self.axes, self.origin, v)
    }
}

/// A model's triangles in its own space (every submesh), from the player's data.
pub fn model_triangles(chain: &mut Chain, path: &str) -> Result<Vec<Tri>> {
    let meshes = load_object_model(chain, path)?;
    let mut out = Vec::new();
    for m in &meshes {
        for c in m.indices.chunks_exact(3) {
            let p = |i: u32| m.positions.get(i as usize).copied();
            if let (Some(a), Some(b), Some(d)) = (p(c[0]), p(c[1]), p(c[2])) {
                out.push([a, b, d]);
            }
        }
    }
    Ok(out)
}

/// A game object display's model triangles in its own space.
pub fn display_triangles(chain: &mut Chain, display: u32) -> Result<Vec<Tri>> {
    let catalog = load_gameobject_catalog(chain)?;
    let path = catalog.model_path(display).with_context(|| format!("no game object display {display}"))?.to_string();
    model_triangles(chain, &path)
}

/// Several displays' model triangles, read from the player's data folder.
pub fn display_shapes(data_dir: &std::path::Path, displays: &[u32]) -> Result<Vec<Vec<Tri>>> {
    display_shapes_with(data_dir, displays, &[])
}

/// [`display_shapes`], with displays the data has no row for (a client patch's own, placing a
/// doodad's model as an object) read from their model's path.
pub fn display_shapes_with(data_dir: &std::path::Path, displays: &[u32], extra: &[(u32, &str)]) -> Result<Vec<Vec<Tri>>> {
    let mut chain = Chain::open(data_dir).with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    displays.iter().map(|d| match extra.iter().find(|(id, _)| id == d) {
        Some((_, path)) => model_triangles(&mut chain, path),
        None => display_triangles(&mut chain, *d),
    }).collect()
}

fn bounds_of(tris: &[Tri]) -> Option<([f32; 3], [f32; 3])> {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in tris.iter().flatten() {
        for a in 0..3 {
            lo[a] = lo[a].min(p[a]);
            hi[a] = hi[a].max(p[a]);
        }
    }
    (lo[0] <= hi[0]).then_some((lo, hi))
}

pub(super) fn is_ship_root(path: &str) -> bool {
    path.replace('\\', "/").eq_ignore_ascii_case(SHIP_ROOT)
}

pub(super) fn collision(chain: &mut Chain, path: &str) -> Result<CollisionMesh> {
    let path = path.to_ascii_lowercase();
    let root = parse_wmo_root(&chain.read_file(&path)?)?;
    let stem = path.strip_suffix(".wmo").unwrap_or(&path);
    let mut mesh = CollisionMesh::default();
    for group in 0..root.group_count() {
        if SHIP_GROUPS.contains(&group) {
            continue;
        }
        // A missing retained group is an error, not a successful but incomplete afterstate.
        let bytes = chain.read_file(&format!("{stem}_{group:03}.wmo"))?;
        accumulate_wmo_group_collision(&bytes, &mut mesh.positions, &mut mesh.indices);
    }
    Ok(mesh)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_juggernaut_root_is_selected_and_the_dock_and_mine_stay() {
        assert!(is_ship_root("World\\WMO\\Dungeon\\AZ_Deadmines\\AZ_Deadmines_B.wmo"));
        assert!(!is_ship_root("world/wmo/dungeon/az_deadmines/az_deadmines_d.wmo"));
        for keep in [0, 16, 18, 21, 34, 35, 39, 41, 42] {
            assert!(!SHIP_GROUPS.contains(&keep));
        }
        assert_eq!(SHIP_GROUPS.len(), 15);
    }

    #[test]
    fn the_hidden_doodads_are_the_mines_own_and_the_dark_lanterns_among_them() {
        assert!(MINE_DOODADS.windows(2).all(|w| w[0] < w[1]), "sorted, each once");
        assert!(MINE_DOODADS.iter().all(|&i| i as usize >= SHIP_DOODADS), "none of the ship's (patch 0087 has those)");
        assert!(DARK_LANTERNS.windows(2).all(|w| w[0] < w[1]));
        assert!(DARK_LANTERNS.iter().all(|i| MINE_DOODADS.contains(i)));
    }

    /// The lanterns patch 0091 puts out: each carries a light of the mine's own within reach and
    /// puts out no other lantern's, and they are about a third of the mine's lit lanterns. The open
    /// survey lacks exactly the hidden doodads, and nothing of the mine's own stands on one of them.
    #[test]
    #[ignore = "requires the player's installed WoW data"]
    fn native_a_third_of_the_lit_lanterns_go_out_each_with_its_own_light() {
        let data = std::env::var("WOW_DATA").expect("set WOW_DATA");
        let mut chain = Chain::open(std::path::Path::new(&data)).unwrap();
        let bytes = chain.read_file(&SHIP_ROOT.replace('/', "\\")).unwrap();
        let root = parse_wmo_root(&bytes).unwrap();
        let lights: Vec<_> = benilla_formats::parse_wmo_lights(&bytes).into_iter().filter(|l| l.is_omni() && l.intensity > 0.0).collect();
        let doodads = root.doodads();
        let near = |a: [f32; 3], b: [f32; 3]| (0..3).map(|i| (a[i] - b[i]).powi(2)).sum::<f32>() < LANTERN_REACH * LANTERN_REACH;
        let lantern = |k: usize| doodads[k].model.to_ascii_lowercase().contains("lantern");
        let lit: Vec<usize> = (SHIP_DOODADS..doodads.len())
            .filter(|&k| lantern(k) && lights.iter().any(|l| near(l.position, doodads[k].position)))
            .collect();
        for &k in DARK_LANTERNS {
            assert!(lit.contains(&(k as usize)), "doodad {k} is a lantern with a light of its own");
        }
        for l in lights.iter().filter(|l| DARK_LANTERNS.iter().any(|&k| near(l.position, doodads[k as usize].position))) {
            let kept: Vec<_> = lit.iter().filter(|&&k| !DARK_LANTERNS.contains(&(k as u32)) && near(l.position, doodads[k].position)).collect();
            assert!(kept.is_empty(), "a put-out lantern's light is also lantern {kept:?}'s");
        }
        let share = DARK_LANTERNS.len() as f32 / lit.len() as f32;
        eprintln!("{} of {} lit lanterns out ({:.0}%)", DARK_LANTERNS.len(), lit.len(), 100.0 * share);
        assert!((0.25..=0.4).contains(&share), "about a third of the lit lanterns go out");

        let open = super::super::load_aftermath_open_survey(std::path::Path::new(&data)).unwrap();
        let mut hidden: Vec<u32> = open.hidden.iter().filter_map(|h| h.index).collect();
        hidden.sort_unstable();
        assert_eq!(hidden, MINE_DOODADS, "every hidden doodad is one the mine places");
        assert!(open.standing.iter().all(|s| s.index.is_none_or(|i| !MINE_DOODADS.contains(&i))));
        let corners = |s: &Standing| -> Vec<[f32; 3]> {
            let Some((lo, hi)) = s.bounds else { return Vec::new() };
            (0..8).map(|c| s.world([if c & 1 == 0 { lo[0] } else { hi[0] }, if c & 2 == 0 { lo[1] } else { hi[1] }, if c & 4 == 0 { lo[2] } else { hi[2] }])).collect()
        };
        for h in &open.hidden {
            let box_ = corners(h);
            let Some(top) = box_.iter().map(|p| p[2]).reduce(f32::max) else { continue };
            let (x0, x1) = box_.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[0]), b.max(p[0])));
            let (y0, y1) = box_.iter().fold((f32::MAX, f32::MIN), |(a, b), p| (a.min(p[1]), b.max(p[1])));
            for s in &open.standing {
                let Some(low) = corners(s).iter().map(|p| p[2]).reduce(f32::min) else { continue };
                let on = (x0..=x1).contains(&s.origin[0]) && (y0..=y1).contains(&s.origin[1]) && (top - 0.4..=top + 0.3).contains(&low);
                assert!(!on, "{} stands on hidden doodad {:?} ({})", s.model, h.index, h.model);
            }
        }
    }

    #[test]
    #[ignore = "requires the player's installed WoW data"]
    fn native_ship_floors_are_gone_and_the_dock_still_stands() {
        let data = std::env::var("WOW_DATA").expect("set WOW_DATA");
        let intact = super::super::load(std::path::Path::new(&data)).unwrap();
        let after = super::super::load_aftermath(std::path::Path::new(&data)).unwrap();
        for [x, y, z] in [[-76.0, -820.0, 40.2], [-30.0, -812.0, 39.4], [-33.0, -803.0, 31.0]] {
            let old = intact.ground.height_near(x, y, z);
            let new = after.ground.height_near(x, y, z);
            eprintln!("ship {x} {y}: intact {old}, aftermath {new}");
            assert!((old - z).abs() < 3.0, "probe must stand on the original ship");
            assert!((new - old).abs() > 5.0, "the deck or added gangway still exists");
        }
        let old = intact.ground.height_near(-2.0, -768.0, 9.63);
        let new = after.ground.height_near(-2.0, -768.0, 9.63);
        assert!((old - new).abs() < 0.1, "Smite's dock must remain");
        assert!(after.props.len() < intact.props.len());
        let mut chain = Chain::open(std::path::Path::new(&data)).unwrap();
        let tile = benilla_formats::load_tile_mesh(&mut chain, "DeadminesInstance", 33, 32).unwrap();
        let fixtures: Vec<_> = tile.doodads.iter()
            .filter(|d| SHIP_ADT_DOODADS.contains(&d.unique_id)).collect();
        assert_eq!(fixtures.len(), 61);
        for fixture in fixtures {
            assert!(intact.props.iter().any(|(_, pos, _)| *pos == fixture.position));
            assert!(!after.props.iter().any(|(_, pos, _)| *pos == fixture.position),
                "orphaned native fixture {} remains", fixture.unique_id);
        }
        let shore = tile.doodads.iter().find(|d| d.unique_id == 333891).unwrap();
        assert!(after.props.iter().any(|(_, pos, _)| *pos == shore.position));

    }
}
