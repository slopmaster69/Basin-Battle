//! Elwynn's subzones as the terrain names them: `areas <Data>` prints, for every area id under
//! a chunk in the zone's map rect, its name, chunk count, middle and bounds, and the height and
//! water there; then the world map's overlays for Elwynn and the buildings (WMOs) in it.
use benilla_formats::{
    load_area_table_catalog, load_tile_mesh, load_world_map_area_catalog, load_world_map_overlay_catalog, Chain,
};

const TILE: f32 = 1600.0 / 3.0;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let areas = load_area_table_catalog(&mut chain)?;
    let wma = load_world_map_area_catalog(&mut chain)?;
    let wmo = load_world_map_overlay_catalog(&mut chain)?;
    let (wid, z) = wma.iter().find(|(_, a)| a.area_id == 12).map(|(i, a)| (i, a.clone())).unwrap();
    println!("WMA {wid} {} left(y)={} right(y)={} top(x)={} bottom(x)={}", z.name, z.loc_left, z.loc_right, z.loc_top, z.loc_bottom);
    for o in wmo.for_area(wid) {
        let names: Vec<String> = o.area_id.iter().filter(|a| **a != 0).map(|a| format!("{a}:{}:flag{}", areas.name(*a).unwrap_or("?"), areas.get(*a).map_or(0, |r| r.explore_flag))).collect();
        println!("overlay {} {:?} at {},{} size {}x{}", o.texture_name, names, o.offset_x, o.offset_y, o.texture_width, o.texture_height);
    }
    // Tiles covering the rect.
    let off = 32.0 * TILE;
    let tx0 = ((off - z.loc_left) / TILE).floor() as i32;
    let tx1 = ((off - z.loc_right) / TILE).floor() as i32;
    let ty0 = ((off - z.loc_top) / TILE).floor() as i32;
    let ty1 = ((off - z.loc_bottom) / TILE).floor() as i32;
    #[derive(Default)]
    struct Acc {
        n: usize,
        sx: f32,
        sy: f32,
        sz: f32,
        min: [f32; 2],
        max: [f32; 2],
    }
    let mut acc: std::collections::BTreeMap<u32, Acc> = Default::default();
    let mut wmos: Vec<(String, [f32; 3])> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for tx in tx0..=tx1 {
        for ty in ty0..=ty1 {
            let Ok(t) = load_tile_mesh(&mut chain, "Azeroth", tx as u32, ty as u32) else { continue };
            for c in &t.chunks {
                let p = c.positions[0];
                let mid = c.positions[72];
                let a = acc.entry(c.area_id).or_insert_with(|| Acc { min: [f32::MAX; 2], max: [f32::MIN; 2], ..Default::default() });
                a.n += 1;
                a.sx += mid[0];
                a.sy += mid[1];
                a.sz += mid[2];
                a.min = [a.min[0].min(p[0] - 33.3), a.min[1].min(p[1] - 33.3)];
                a.max = [a.max[0].max(p[0]), a.max[1].max(p[1])];
            }
            for w in &t.wmos {
                if seen.insert(w.unique_id) {
                    wmos.push((w.model.clone(), w.position));
                }
            }
        }
    }
    println!("area\tname\tparent\tchunks\tmid_x\tmid_y\tz\tmin_x\tmin_y\tmax_x\tmax_y");
    for (id, a) in &acc {
        let row = areas.get(*id);
        let n = a.n as f32;
        println!(
            "{id}\t{}\t{}\t{}\t{:.0}\t{:.0}\t{:.0}\t{:.0}\t{:.0}\t{:.0}\t{:.0}",
            areas.name(*id).unwrap_or("?"),
            row.map_or(0, |r| r.zone_id),
            a.n,
            a.sx / n,
            a.sy / n,
            a.sz / n,
            a.min[0],
            a.min[1],
            a.max[0],
            a.max[1]
        );
    }
    wmos.sort_by(|a, b| a.0.cmp(&b.0));
    for (m, p) in wmos {
        if p[0] < z.loc_top && p[0] > z.loc_bottom && p[1] < z.loc_left && p[1] > z.loc_right {
            println!("wmo\t{m}\t{:.0}\t{:.0}\t{:.0}", p[0], p[1], p[2]);
        }
    }
    Ok(())
}
