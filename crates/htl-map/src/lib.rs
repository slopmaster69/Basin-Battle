//! The arena, read from the host's own 1.12.1 install: Arathi Basin's terrain, the hub at the
//! Blacksmith, and one lane from each outer base's flag all the way to the hub: enemies pour out of
//! the bases, so pushing out to a base means pushing against the stream. Lanes follow the roads
//! painted on the terrain and cross the river by its bridges: each base sends its enemies down
//! both roads leaving it, so the lanes from different bases meet at crossroads and at the two
//! bridges into the Blacksmith.
//!
//! Nothing read here leaves the host's machine: it becomes a height grid and a few paths in
//! memory.

use std::path::Path;

use anyhow::{bail, Context, Result};
use benilla_formats::{load_area_poi_catalog, load_tiles_around, terrain_height_at, Chain, ChunkMesh};
use htl_sim::{Arena, BaseSite, Ground, Lane, Vec3};

pub mod assets;
pub mod deadmines;
pub mod elwynn;
pub mod grid;
pub mod layers;
pub mod siege;
pub mod world;
pub use grid::{find_path, find_path_with, reachable, HeightGrid};

/// Arathi Basin.
pub const MAP_ID: u32 = 529;
pub const MAP_DIR: &str = "PVPZone04";
/// `AreaTable.dbc`: Arathi Basin.
pub const AREA_ID: u32 = 3358;

/// The hub's base name in `AreaPOI.dbc`.
const HUB_POI: &str = "Blacksmith";
/// The lanes, in the order waves open them: the first two bases are used from wave 1.
const LANE_POIS: [(&str, &str); 4] = [
    ("Farm", "the Farm"),
    ("Stables", "the Stables"),
    ("Gold Mine", "the Gold Mine"),
    ("Lumber Mill", "the Lumber Mill"),
];

/// Arathi Basin's roads are painted with this terrain texture.
const ROAD_TEXTURE: &str = "RubbleBase";
/// A base's second road shuns ground this close to its first route...
const SHUN_RADIUS: f32 = 10.0;
/// ...by this factor...
const SHUN: f32 = 8.0;
/// ...except this close to the hub, where every route ends.
const MERGE_NEAR_HUB: f32 = 30.0;

/// A post stands this many lane points (4 yd each) past a bridge, on the hub's side...
const POST_STEPS_IN: usize = 4;
/// ...on dry, open ground this wide around it (room for its defences).
const POST_CLEARANCE: f32 = 7.5;

/// How far out from the hub a lane starts, in yards walked.
const LANE_LENGTH: f32 = 140.0;
/// Open ground the hub needs around it, in yards.
const HUB_CLEARANCE: f32 = 9.0;
/// The steepest climb a lane takes, as rise over run.
pub const MAX_SLOPE: f32 = 0.9;
/// Grid spacing in yards.
const CELL: f32 = 1.0;
/// Spacing of points along a finished lane.
const LANE_STEP: f32 = 4.0;

pub struct LoadedArena {
    pub arena: Arena,
    pub ground: HeightGrid,
}

/// Build the arena from the `Data` folder of a 1.12.1 install.
/// Icon names for spells (`Spell_Frost_FrostBolt02`, without the `Interface\Icons\` folder).
pub fn spell_icons(data_dir: &Path, spells: &[u32]) -> Result<std::collections::HashMap<u32, String>> {
    let mut chain = Chain::open(data_dir)
        .with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let catalog = benilla_formats::load_spell_catalog(&mut chain).context("reading Spell.dbc")?;
    let mut out = std::collections::HashMap::new();
    for &id in spells {
        if let Some(icon) = catalog.get(id).and_then(|s| s.icon.clone()) {
            let name = icon.rsplit(['\\', '/']).next().unwrap_or(&icon).to_string();
            out.insert(id, name);
        }
    }
    Ok(out)
}

/// What of these spells' and kits' looks the game data can't show: a spell missing from
/// `Spell.dbc`, a missile or kit effect with no `SpellVisualEffectName` row (the client draws its
/// error cube there) or whose model isn't in the data. Each line names the id and what's wrong.
pub fn spell_audit(data_dir: &Path, spells: &[u32], kits: &[u32]) -> Result<Vec<String>> {
    let mut chain = Chain::open(data_dir)
        .with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let catalog = benilla_formats::load_spell_catalog(&mut chain).context("reading Spell.dbc")?;
    let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).context("reading SpellVisual.dbc")?;
    let mut out = Vec::new();
    let mut check_effect = |chain: &Chain, what: &str, effect: u32, out: &mut Vec<String>| match visuals.effect_path(effect) {
        None => out.push(format!("{what}: effect {effect} has no SpellVisualEffectName row (the error cube)")),
        Some(path) => {
            let mdx = path.to_string();
            let m2 = if mdx.to_ascii_lowercase().ends_with(".mdx") { format!("{}.m2", &mdx[..mdx.len() - 4]) } else { mdx.clone() };
            if !chain.contains(&mdx) && !chain.contains(&m2) {
                out.push(format!("{what}: effect {effect}'s model {mdx} isn't in the data"));
            }
        }
    };
    let kit_check = |chain: &Chain, what: &str, kit: u32, out: &mut Vec<String>, check_effect: &mut dyn FnMut(&Chain, &str, u32, &mut Vec<String>)| {
        if kit == 0 {
            return;
        }
        match visuals.kit(kit) {
            None => out.push(format!("{what}: kit {kit} isn't in SpellVisualKit.dbc")),
            Some(k) => {
                for (_, e) in k.effects() {
                    check_effect(chain, &format!("{what} kit {kit}"), e, out);
                }
            }
        }
    };
    for &id in spells {
        let Some(sp) = catalog.get(id) else {
            out.push(format!("spell {id}: not in Spell.dbc"));
            continue;
        };
        let what = format!("spell {id} ({})", sp.name);
        if sp.visual == 0 {
            continue;
        }
        let Some(st) = visuals.stages(sp.visual) else {
            out.push(format!("{what}: visual {} isn't in SpellVisual.dbc", sp.visual));
            continue;
        };
        if st.missile_model != 0 {
            check_effect(&chain, &format!("{what} missile"), st.missile_model, &mut out);
        }
        for (stage, kit) in [("precast", st.precast), ("cast", st.cast), ("impact", st.impact), ("state", st.state), ("channel", st.channel)] {
            kit_check(&chain, &format!("{what} {stage}"), kit, &mut out, &mut check_effect);
        }
    }
    for &kit in kits {
        kit_check(&chain, "kit", kit, &mut out, &mut check_effect);
    }
    Ok(out)
}

pub fn load(data_dir: &Path) -> Result<LoadedArena> {
    let mut chain = Chain::open(data_dir)
        .with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let pois = load_area_poi_catalog(&mut chain).context("reading AreaPOI.dbc")?;
    let poi = |name: &str| -> Result<Vec3> {
        pois.rows()
            .find(|(_, p)| p.continent_id == MAP_ID && p.name == name)
            .map(|(_, p)| p.pos)
            .with_context(|| format!("no '{name}' point in Arathi Basin"))
    };
    let hub_poi = poi(HUB_POI)?;
    let bases: Vec<(Vec3, &str)> = LANE_POIS
        .iter()
        .map(|(poi_name, label)| poi(poi_name).map(|p| (p, *label)))
        .collect::<Result<_>>()?;

    // Terrain for everything within reach of the bases.
    let tiles = load_tiles_around(&mut chain, MAP_DIR, hub_poi[0], hub_poi[1], 1)
        .context("reading Arathi Basin terrain")?;
    if tiles.is_empty() {
        bail!("no Arathi Basin terrain found in the game data");
    }
    let tiles: Vec<benilla_formats::TileMesh> = tiles.into_iter().map(|(_, t)| t).collect();
    let chunks: Vec<ChunkMesh> = tiles.iter().flat_map(|t| t.chunks.iter().cloned()).collect();
    let (min, max) = bounds(&bases.iter().map(|b| b.0).chain([hub_poi]).collect::<Vec<_>>(), 110.0);
    let mut ground = HeightGrid::sample(min, max, CELL, |x, y| {
        terrain_height_at(&chunks, [x, y, 0.0])
    });
    let buildings = world::load_buildings(&mut chain, &tiles, min, max);
    world::apply(&mut ground, &buildings, &chunks);
    world::mark_roads(&mut ground, &chunks, ROAD_TEXTURE);

    // The hub stands in the open by the Blacksmith, not inside it.
    let hub = ground
        .open_spot([hub_poi[0], hub_poi[1]], HUB_CLEARANCE, 40.0)
        .context("no open ground near the Blacksmith for the hub")?;
    let mut lanes = Vec::new();
    let mut sites: Vec<(String, Vec3, Vec3)> = Vec::new();
    let mut missing = Vec::new();
    let reach = reachable(&ground, hub, MAX_SLOPE);
    for (base, label) in &bases {
        // Start from open ground by the base that connects to the hub, not inside a building.
        let start = ground
            .open_spot_where([base[0], base[1]], 2.0, 60.0, |k| reach[k])
            .unwrap_or(*base);
        let Some(first) = find_path(&ground, start, hub, MAX_SLOPE) else {
            missing.push(*label);
            continue;
        };
        // The second road out: the best way that keeps off the first.
        let shun = shun_route(&ground, &first, hub);
        let second = find_path_with(&ground, start, hub, MAX_SLOPE, Some(&shun)).filter(|p| apart(p, &first, hub));
        let mut routes = vec![resample(&ground, &first)];
        if let Some(second) = second {
            routes.push(resample(&ground, &second));
        }
        sites.push((label.to_string(), start, routes[0][0]));
        lanes.push(Lane::split(label.to_string(), routes));
    }
    let mut merged = lanes;
    // Early waves use the first two lanes: make them the two that come from opposite sides.
    if merged.len() > 2 {
        let first = merged[0].points[0];
        let far = (1..merged.len())
            .max_by(|&a, &b| {
                let da = htl_sim::geom::dist2d(merged[a].points[0], first);
                let db = htl_sim::geom::dist2d(merged[b].points[0], first);
                da.total_cmp(&db)
            })
            .unwrap();
        let lane = merged.remove(far);
        merged.insert(1, lane);
    }
    let lanes = merged;
    // Each base feeds the lane that starts where its own route does.
    let bases: Vec<BaseSite> = sites
        .into_iter()
        .filter_map(|(name, pos, lane_start)| {
            let lane = lanes
                .iter()
                .enumerate()
                .min_by(|a, b| {
                    let da = htl_sim::geom::dist2d(a.1.points[0], lane_start);
                    let db = htl_sim::geom::dist2d(b.1.points[0], lane_start);
                    da.total_cmp(&db)
                })
                .map(|(i, _)| i)?;
            Some(BaseSite { name, pos, lane })
        })
        .collect();
    if lanes.len() < 2 {
        bail!("only {} distinct walkable lanes to the hub", lanes.len());
    }
    for m in &missing {
        eprintln!("[arena] no walkable way from {m} to the hub; that lane is skipped");
    }
    // Players appear just behind the hub as seen from the first lane, facing it: the camera opens
    // on the Ancient with that lane beyond it. Open ground is preferred.
    let toward_first = htl_sim::geom::facing(hub, lanes[0].points[0]);
    let back = toward_first + std::f32::consts::PI;
    let (sx, sy) = (hub[0] + 12.0 * back.cos(), hub[1] + 12.0 * back.sin());
    let player_spawn = ground
        .open_spot_where([sx, sy], 1.5, 8.0, |k| reach[k])
        .unwrap_or([sx, sy, ground.height(sx, sy)]);
    Ok(LoadedArena {
        arena: Arena {
            map_id: MAP_ID,
            area_id: AREA_ID,
            hub,
            hub_facing: toward_first,
            player_spawn,
            player_facing: toward_first,
            posts: find_posts(&ground, &lanes, hub, &reach),
            map_rect: benilla_formats::load_world_map_area_catalog(&mut chain)
                .ok()
                .and_then(|c| c.iter().find(|(_, a)| a.map_id == MAP_ID && a.area_id != 0 || a.map_id == MAP_ID).map(|(_, a)| [a.loc_left, a.loc_right, a.loc_top, a.loc_bottom])),
            lanes,
            bases,
        },
        ground,
    })
}

/// The last bridge each route crosses before the hub, one post per bridge: open ground on the
/// hub's side of it, facing the way enemies come.
fn find_posts(ground: &HeightGrid, lanes: &[Lane], hub: Vec3, reach: &[bool]) -> Vec<htl_sim::Post> {
    use htl_sim::geom::{dist2d, facing};
    // (where the deck ends on the hub's side, a point on the deck, how many routes)
    let mut found: Vec<(Vec3, Vec3, u32)> = Vec::new();
    for route in lanes.iter().flat_map(|l| l.routes.iter()) {
        // Walk out from the hub to the first deck.
        let Some(k) = (0..route.len()).rev().find(|&k| ground.is_deck(route[k][0], route[k][1])) else { continue };
        let end = route[k];
        // A few steps further toward the hub: the bank on the hub's side.
        let bank = route[(k + POST_STEPS_IN).min(route.len() - 1)];
        match found.iter_mut().find(|f| dist2d(f.1, end) < 30.0) {
            Some(f) => f.2 += 1,
            None => found.push((bank, end, 1)),
        }
    }
    let mut posts = Vec::new();
    if std::env::var_os("HTL_DEBUG_POSTS").is_some() {
        eprintln!("[posts] bridges {:?}", found);
    }
    for (bank, deck, _) in found {
        // As much room as the bank gives, down to a little.
        let spot = [POST_CLEARANCE, 5.5, 4.0, 2.5]
            .into_iter()
            .find_map(|clear| ground.open_spot_where([bank[0], bank[1]], clear, 18.0, |k| reach[k]));
        let Some(pos) = spot else {
            eprintln!("[arena] no open ground for a post by the bridge at {deck:?}");
            continue;
        };
        if dist2d(pos, hub) < 25.0 {
            continue;
        }
        // WoW +x is north, +y west.
        let (north, west) = (deck[0] - hub[0], deck[1] - hub[1]);
        let names = ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"];
        let bearing = (-west).atan2(north).to_degrees().rem_euclid(360.0);
        let dir = names[((bearing + 22.5) / 45.0) as usize % 8];
        posts.push(htl_sim::Post { name: format!("the {dir} bridge"), pos, facing: facing(pos, deck) });
    }
    // Two bridges the same way: tell them apart by number.
    let names: Vec<String> = posts.iter().map(|p| p.name.clone()).collect();
    for (i, p) in posts.iter_mut().enumerate() {
        if names.iter().filter(|n| **n == names[i]).count() > 1 {
            p.name = format!("{} {}", p.name, i + 1);
        }
    }
    posts
}

/// Extra cost for walking near `route`, away from the hub: what makes a second route take the
/// other road.
fn shun_route(ground: &HeightGrid, route: &[Vec3], hub: Vec3) -> Vec<f32> {
    let (w, h) = ground.size();
    let mut out = vec![1.0f32; w * h];
    let r = (SHUN_RADIUS / ground.cell()).ceil() as i32;
    for p in route.iter().step_by(2) {
        if htl_sim::geom::dist2d(*p, hub) < MERGE_NEAR_HUB {
            continue;
        }
        let k = ground.index_of(p[0], p[1]);
        let (ci, cj) = ((k % w) as i32, (k / w) as i32);
        for dj in -r..=r {
            for di in -r..=r {
                let (i, j) = (ci + di, cj + dj);
                if i >= 0 && j >= 0 && i < w as i32 && j < h as i32 && di * di + dj * dj <= r * r {
                    out[j as usize * w + i as usize] = SHUN;
                }
            }
        }
    }
    out
}

/// Whether `b` mostly keeps away from `a` (away from the hub): a real second road.
fn apart(b: &[Vec3], a: &[Vec3], hub: Vec3) -> bool {
    let far: Vec<&Vec3> = b.iter().step_by(3).filter(|p| htl_sim::geom::dist2d(**p, hub) >= MERGE_NEAR_HUB).collect();
    if far.is_empty() {
        return false;
    }
    let shared = far
        .iter()
        .filter(|p| a.iter().step_by(2).any(|q| htl_sim::geom::dist2d(***p, *q) < 6.0))
        .count();
    (shared as f32) < far.len() as f32 * 0.6
}

fn bounds(points: &[Vec3], margin: f32) -> ([f32; 2], [f32; 2]) {
    let mut min = [f32::MAX; 2];
    let mut max = [f32::MIN; 2];
    for p in points {
        for i in 0..2 {
            min[i] = min[i].min(p[i]);
            max[i] = max[i].max(p[i]);
        }
    }
    (
        [min[0] - margin, min[1] - margin],
        [max[0] + margin, max[1] + margin],
    )
}

/// A whole base-to-hub path, resampled every [`LANE_STEP`] yards on the ground.
pub fn resample(ground: &dyn Ground, path: &[Vec3]) -> Vec<Vec3> {
    resample_from(ground, path)
}

/// Keep the last [`LANE_LENGTH`] yards of a base-to-hub path, resampled every [`LANE_STEP`]
/// yards on the ground.
pub fn finish_lane(ground: &dyn Ground, path: &[Vec3]) -> Vec<Vec3> {
    // Walk backwards from the hub to find where the lane starts.
    let mut acc = 0.0;
    let mut start = 0;
    for i in (1..path.len()).rev() {
        acc += htl_sim::geom::dist2d(path[i], path[i - 1]);
        if acc >= LANE_LENGTH {
            start = i - 1;
            break;
        }
    }
    resample_from(ground, &path[start..])
}

fn resample_from(ground: &dyn Ground, trimmed: &[Vec3]) -> Vec<Vec3> {
    // Resample at a steady spacing.
    let mut out = vec![trimmed[0]];
    let mut carry = 0.0;
    for w in trimmed.windows(2) {
        let (a, b) = (w[0], w[1]);
        let seg = htl_sim::geom::dist2d(a, b);
        let mut t = LANE_STEP - carry;
        while t < seg {
            let f = t / seg;
            let (x, y) = (a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f);
            out.push([x, y, ground.height(x, y)]);
            t += LANE_STEP;
        }
        carry = seg - (t - LANE_STEP);
    }
    let last = *trimmed.last().unwrap();
    if htl_sim::geom::dist2d(*out.last().unwrap(), last) > 0.5 {
        out.push(last);
    }
    out
}
