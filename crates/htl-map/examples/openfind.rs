//! Find open level ground in Elwynn Survival's play area: `openfind <Data> <radius> [x,y ...]`
//! scores every spot on a 4-yard lattice by how level the ground within `radius` is, and lists
//! the best that have no wall, water, road or tree trunk inside it and are at least 40 yards from
//! every `x,y` given (packs, the camp). Prints positions and heights only.
use htl_map::elwynn;
use htl_sim::Ground;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let e = elwynn::load(std::path::Path::new(&args[1]))?;
    let radius: f32 = args[2].parse()?;
    let avoid: Vec<[f32; 2]> = args[3..]
        .iter()
        .filter_map(|m| {
            let v: Vec<f32> = m.split(',').filter_map(|s| s.parse().ok()).collect();
            (v.len() >= 2).then(|| [v[0], v[1]])
        })
        .collect();
    let g = &e.grid;
    let mut found: Vec<(f32, [f32; 2], f32, usize)> = Vec::new();
    let mut x = elwynn::MIN[0] + radius + 2.0;
    while x < elwynn::MAX[0] - radius - 2.0 {
        let mut y = elwynn::MIN[1] + radius + 2.0;
        while y < elwynn::MAX[1] - radius - 2.0 {
            if avoid.iter().any(|a| (a[0] - x).hypot(a[1] - y) < 40.0 + radius) {
                y += 4.0;
                continue;
            }
            let (mut lo, mut hi, mut bad, mut trunks) = (f32::MAX, f32::MIN, false, 0);
            let steps = radius.ceil() as i32;
            'cells: for dj in -steps..=steps {
                for di in -steps..=steps {
                    let (dx, dy) = (di as f32, dj as f32);
                    if dx.hypot(dy) > radius {
                        continue;
                    }
                    let (px, py) = (x + dx, y + dy);
                    if g.is_blocked(px, py) || g.is_wet(px, py) || g.road(px, py) > 0.2 {
                        bad = true;
                        break 'cells;
                    }
                    let z = g.height(px, py);
                    lo = lo.min(z);
                    hi = hi.max(z);
                }
            }
            if !bad {
                for t in &e.trunks {
                    if (t.pos[0] - x).hypot(t.pos[1] - y) < radius + t.radius {
                        trunks += 1;
                    }
                }
                found.push((hi - lo, [x, y], g.height(x, y), trunks));
            }
            y += 4.0;
        }
        x += 4.0;
    }
    found.sort_by(|a, b| (a.0 + a.3 as f32 * 1.5).total_cmp(&(b.0 + b.3 as f32 * 1.5)));
    let mut shown: Vec<[f32; 2]> = Vec::new();
    for (rise, at, z, trunks) in found {
        if shown.iter().any(|s| (s[0] - at[0]).hypot(s[1] - at[1]) < radius * 2.0) {
            continue;
        }
        shown.push(at);
        println!("[{:.0}, {:.0}] z {:.1}  rise {:.1} yd  {} trunks", at[0], at[1], z, rise, trunks);
        if shown.len() >= 15 {
            break;
        }
    }
    Ok(())
}
