//! The ship is part of the mine's WMO, separated by authored group indices. Keep the dock,
//! approach and every mine room; never remove an overlapping world-space box of floors.

use anyhow::Result;
use benilla_formats::{accumulate_wmo_group_collision, parse_wmo_root, Chain, CollisionMesh};

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
