//! Siege's field, read from the host's own 1.12.1 install: Arathi Basin, its four outer bases
//! (the Farm, the Stables, the Gold Mine, the Lumber Mill) the camps and the Blacksmith between
//! them. It's read as Elwynn is (heights, buildings as floors and walls, roads, trunks); nothing
//! read here leaves the machine.

use std::path::Path;

use anyhow::{Context, Result};
use benilla_formats::{load_area_poi_catalog, Chain};

use crate::elwynn::{load_region, LoadedElwynn, Region};

/// Arathi Basin.
pub const MAP_ID: u32 = crate::MAP_ID;
pub const AREA_ID: u32 = crate::AREA_ID;
/// The basin and the hills round it.
pub const REGION: Region = Region {
    map_dir: crate::MAP_DIR,
    min: [560.0, 560.0],
    max: [1440.0, 1440.0],
    zone_id: AREA_ID,
    skip: &[],
    road_texture: "RubbleBase",
    max_floor: 3.5,
};

pub fn load(data_dir: &Path) -> Result<LoadedElwynn> {
    load_region(data_dir, &REGION)
}

/// The basin's named points (`AreaPOI.dbc`): the bases and the halls.
pub fn pois(data_dir: &Path) -> Result<Vec<(String, [f32; 3])>> {
    let mut chain = Chain::open(data_dir).with_context(|| format!("opening the game data in {}", data_dir.display()))?;
    let pois = load_area_poi_catalog(&mut chain).context("reading AreaPOI.dbc")?;
    Ok(pois.rows().filter(|(_, p)| p.continent_id == MAP_ID).map(|(_, p)| (p.name.clone(), p.pos)).collect())
}
