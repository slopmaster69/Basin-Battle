//! Survey Arathi Basin's ground textures and water, and draw them:
//! `roads <Data> list` prints every texture's coverage; `roads <Data> draw <out.png> <word>...`
//! draws height, water (light blue shallow, dark deep), walls (red), textures whose name holds a
//! word (orange), the current lanes (yellow) and the hub (green).
use benilla_formats::{load_area_poi_catalog, load_tiles_around, Chain, ChunkMesh, ALPHA_MAP_SIZE};
use htl_sim::Ground;

const CHUNK: f32 = 100.0 / 3.0;

/// The weight of each layer at (x, y) in a chunk, layer 0 first.
fn weights(c: &ChunkMesh, x: f32, y: f32) -> Option<Vec<f32>> {
    let [wx, wy, _] = c.positions[0];
    let (v, u) = ((wx - x) / CHUNK, (wy - y) / CHUNK);
    if !(0.0..1.0).contains(&u) || !(0.0..1.0).contains(&v) {
        return None;
    }
    let n = c.layer_textures.len();
    let mut a = vec![0.0f32; n];
    if let Some(map) = &c.alpha_map {
        let s = ALPHA_MAP_SIZE as usize;
        let (col, row) = ((u * s as f32) as usize, (v * s as f32) as usize);
        let t = (row * s + col) * 4;
        for l in 1..n.min(4) {
            a[l] = map[t + l - 1] as f32 / 255.0;
        }
    }
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

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let a = htl_map::load(data)?;
    let mut chain = Chain::open(data)?;
    let pois = load_area_poi_catalog(&mut chain)?;
    let hub = pois.rows().find(|(_, p)| p.continent_id == 529 && p.name == "Blacksmith").unwrap().1.pos;
    for (_, p) in pois.rows().filter(|(_, p)| p.continent_id == 529) {
        println!("poi {} {:?}", p.name, p.pos);
    }
    let tiles = load_tiles_around(&mut chain, "PVPZone04", hub[0], hub[1], 1)?;
    let chunks: Vec<ChunkMesh> = tiles.iter().flat_map(|(_, t)| t.chunks.iter().cloned()).collect();
    let g = &a.ground;
    let (w, h) = g.size();
    let chunk_at = |x: f32, y: f32| chunks.iter().find(|c| {
        let [wx, wy, _] = c.positions[0];
        x <= wx && x > wx - CHUNK && y <= wy && y > wy - CHUNK
    });
    match args[2].as_str() {
        "list" => {
            let mut cover: std::collections::BTreeMap<String, f32> = Default::default();
            for j in (0..h).step_by(2) {
                for i in (0..w).step_by(2) {
                    let [x, y] = g.cell_xy(i, j);
                    if let Some(c) = chunk_at(x, y) {
                        if let Some(ws) = weights(c, x, y) {
                            for (l, t) in c.layer_textures.iter().enumerate() {
                                *cover.entry(t.clone()).or_default() += ws[l] * 4.0;
                            }
                        }
                    }
                }
            }
            let mut v: Vec<_> = cover.into_iter().collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            for (t, c) in v {
                println!("{c:>9.0} yd2  {t}");
            }
            let mut shallow = 0;
            for c in &chunks {
                shallow += c.liquids.len();
            }
            println!("{} liquid surfaces", shallow);
        }
        "draw" => {
            let words: Vec<String> = args[4..].iter().map(|s| s.to_lowercase()).collect();
            let zs = g.heights();
            let (lo, hi) = zs.iter().fold((f32::MAX, f32::MIN), |(a, b), &z| (a.min(z), b.max(z)));
            let mut img = image::RgbImage::new(h as u32, w as u32);
            for j in 0..h {
                for i in 0..w {
                    let [x, y] = g.cell_xy(i, j);
                    let v = ((g.height(x, y) - lo) / (hi - lo) * 200.0 + 30.0) as u8;
                    let mut px = [v, v, v];
                    if let Some(c) = chunk_at(x, y) {
                        if let Some(ws) = weights(c, x, y) {
                            let road: f32 = c
                                .layer_textures
                                .iter()
                                .enumerate()
                                .filter(|(_, t)| words.iter().any(|wd| t.to_lowercase().contains(wd.as_str())))
                                .map(|(l, _)| ws[l])
                                .sum();
                            if road > 0.0 {
                                let r = road.min(1.0);
                                px = [
                                    (px[0] as f32 * (1.0 - r) + 255.0 * r) as u8,
                                    (px[1] as f32 * (1.0 - r) + 140.0 * r) as u8,
                                    (px[2] as f32 * (1.0 - r) + 0.0 * r) as u8,
                                ];
                            }
                        }
                    }
                    if g.is_water(x, y) {
                        px = [20, 40, 160];
                    } else if g.is_wet(x, y) {
                        px = [90, 160, 255];
                    }
                    if g.is_blocked(x, y) {
                        px = [220, 40, 40];
                    }
                    img.put_pixel((h - 1 - j) as u32, (w - 1 - i) as u32, image::Rgb(px));
                }
            }
            let mut dot = |p: [f32; 3], c: [u8; 3], r: i32| {
                if let Some((i, j)) = g.cell_at(p[0], p[1]) {
                    for dy in -r..=r {
                        for dx in -r..=r {
                            let (px, py) = ((h - 1 - j) as i32 + dx, (w - 1 - i) as i32 + dy);
                            if px >= 0 && py >= 0 && px < h as i32 && py < w as i32 {
                                img.put_pixel(px as u32, py as u32, image::Rgb(c));
                            }
                        }
                    }
                }
            };
            let colors = [[250, 220, 0], [255, 0, 255], [0, 255, 255], [255, 255, 255], [120, 255, 0], [255, 120, 120], [160, 120, 255], [255, 200, 120]];
            for (k, l) in a.arena.lanes.iter().enumerate() {
                for (r, route) in l.routes.iter().enumerate() {
                    for p in route.iter().skip(r) .step_by(1 + r) {
                        dot(*p, colors[k % colors.len()], 1);
                    }
                }
                println!("{}: {}", l.name, l.routes.iter().map(|r| format!("{:.0} yd", htl_sim::geom::path_len(r))).collect::<Vec<_>>().join(", "));
            }
            for b in &a.arena.bases {
                dot(b.pos, [255, 255, 255], 3);
            }
            for p in &a.arena.posts {
                dot(p.pos, [255, 40, 40], 5);
                dot(p.pos, [255, 255, 255], 2);
                let ahead = [p.pos[0] + 8.0 * p.facing.cos(), p.pos[1] + 8.0 * p.facing.sin(), 0.0];
                dot(ahead, [255, 40, 40], 1);
                println!("post {} at {:?} facing {:.2}", p.name, p.pos, p.facing);
            }
            dot(a.arena.hub, [0, 230, 0], 4);
            img.save(&args[3])?;
            println!("drew {} ({}x{})", args[3], h, w);
        }
        "fronts" => {
            println!("map rect {:?}; hub on the map at {:?}; posts at {:?}", a.arena.map_rect, a.arena.map_xy(a.arena.hub), a.arena.posts.iter().map(|p| a.arena.map_xy(p.pos)).collect::<Vec<_>>());
            // Plan a few waves on the real map: where does each go?
            let mut sim = htl_sim::Sim::new(htl_sim::Content::builtin(), a.arena.clone(), Box::new(a.ground), 3);
            let p = sim.add_player("Probe", "warrior", htl_sim::Luck::default());
            let _ = p;
            for _ in 0..3 {
                for e in sim.step(0.05) {
                    if let htl_sim::Event::Announce { text, .. } = e {
                        println!("{text}");
                    }
                }
            }
            for f in sim.fronts() {
                println!("{}: planned {} ({} elite) {:?} boss {:?}", f.name, f.planned, f.planned_elites, f.notes, f.boss);
            }
            return Ok(());
        }
        "soak" => {
            // Run the game on the real map for many waves (a tough Ancient, two heroes whose
            // spells fire on their own): nothing should panic, and every wave should end.
            let mut content = htl_sim::Content::builtin();
            content.hub.health *= 200;
            content.game.pause_for_picks = false;
            let waves: u32 = args.get(3).and_then(|w| w.parse().ok()).unwrap_or(21);
            let mut sim = htl_sim::Sim::new(content, a.arena.clone(), Box::new(a.ground), 5);
            let heroes = [sim.add_player("Soak", "mage", htl_sim::Luck::default()), sim.add_player("Tank", "warrior", htl_sim::Luck::default())];
            sim.start_now();
            let (mut t, mut zones, mut spawns, mut last) = (0.0f32, 0u32, 0u32, 0u32);
            while t < 3.0 * 3600.0 {
                for &h in &heroes {
                    // Take whatever is offered.
                    if sim.menu(h).is_some() {
                        sim.menu_select(h, 0);
                    }
                }
                for e in sim.step(0.05) {
                    match e {
                        htl_sim::Event::ZoneStart { .. } => zones += 1,
                        htl_sim::Event::SpawnEnemy { .. } => spawns += 1,
                        htl_sim::Event::Announce { text, big: true } if text.contains("calls its dead") || text.contains("enraged") => println!("  {text}"),
                        _ => {}
                    }
                }
                t += 0.05;
                let st = sim.status();
                // Every five minutes into a wave: where are the ones left?
                if (t * 20.0).round() as u32 % 6000 == 0 && st.phase == htl_sim::PhaseName::Wave {
                    let hub = a.arena.hub;
                    for v in sim.entities().into_iter().filter(|v| v.alive && matches!(v.what, htl_sim::What::Enemy { .. })) {
                        let d = ((v.pos[0] - hub[0]).powi(2) + (v.pos[1] - hub[1]).powi(2)).sqrt();
                        println!("    left: {:?} at {:.0},{:.0} ({d:.0} from hub) moving {} attacking {:?}", v.what, v.pos[0], v.pos[1], v.moving.is_some(), v.attacking);
                    }
                }
                if st.wave != last {
                    println!("{:6.0}s wave {:2} {:?} enemies {:3} hub {:5.1}% spawns {spawns} zones {zones}", t, st.wave, st.phase, st.enemies_left, st.hub_health as f32 * 100.0 / st.hub_max_health.max(1) as f32);
                    last = st.wave;
                }
                if st.phase == htl_sim::PhaseName::Intermission && st.seconds_left > 3 {
                    // Spend at home on soldiers (all kinds, promoted), and march some to the posts.
                    let vendor = sim.entities().into_iter().find(|v| matches!(v.what, htl_sim::What::Npc { npc: htl_sim::NpcKind::Vendor, .. })).map(|v| v.pos);
                    for (k, &h) in heroes.iter().enumerate() {
                        // Where they stand now, unless they're too far from the shopkeeper to buy.
                        let back = sim.entities().into_iter().find(|v| v.id == h).map(|v| v.pos);
                        if !sim.at_shop(h) {
                            if let Some(v) = vendor {
                                sim.move_player(h, v, 0.0);
                            }
                        }
                        for key in ["defender", "rifleman", "cleric", "promote", "protector", "spring"] {
                            let _ = sim.buy(h, key);
                        }
                        let _ = sim.buy(h, &format!("march@{}", 1 + (st.wave as usize + k) % sim.site_count().max(2).saturating_sub(1).max(1)));
                        sim.give_gold_for_soak(h, 400);
                        if let Some(b) = back {
                            sim.move_player(h, b, 0.0);
                        }
                    }
                    sim.start_now();
                }
                if st.phase == htl_sim::PhaseName::Defeat || st.wave > waves {
                    println!("end: {st:?} after {t:.0}s");
                    break;
                }
            }
            return Ok(());
        }
        "wmos" => {
            // Every building, where it is on the drawing, and what it does to the walking grid.
            let (min, max) = {
                let [x0, y0] = g.cell_xy(0, 0);
                let [x1, y1] = g.cell_xy(w - 1, h - 1);
                ([x0, y0], [x1, y1])
            };
            let tiles_only: Vec<benilla_formats::TileMesh> = tiles.iter().map(|(_, t)| t.clone()).collect();
            let b = htl_map::world::load_buildings(&mut chain, &tiles_only, min, max);
            for bl in &b {
                let (mut lo, mut hi) = ([i32::MAX; 2], [i32::MIN; 2]);
                let (mut zlo, mut zhi) = (f32::MAX, f32::MIN);
                for t in &bl.tris {
                    for p in t {
                        if let Some((i, j)) = g.cell_at(p[0], p[1]) {
                            let (px, py) = ((h - 1 - j) as i32, (w - 1 - i) as i32);
                            lo = [lo[0].min(px), lo[1].min(py)];
                            hi = [hi[0].max(px), hi[1].max(py)];
                        }
                        zlo = zlo.min(p[2]);
                        zhi = zhi.max(p[2]);
                    }
                }
                println!("{:<60} px {:?}..{:?} z {:.1}..{:.1} tris {}", bl.model, lo, hi, zlo, zhi, bl.tris.len());
            }
        }
        "probe" => {
            // Print the grid along a pixel row/column window: x0 y0 x1 y1 in drawing pixels.
            let n: Vec<i32> = args[3..7].iter().map(|v| v.parse().unwrap()).collect();
            let ground: Vec<(usize, usize)> = Vec::new();
            let _ = ground;
            for py in n[1]..=n[3] {
                let mut line = String::new();
                for px in n[0]..=n[2] {
                    let j = h as i32 - 1 - px;
                    let i = w as i32 - 1 - py;
                    if i < 0 || j < 0 {
                        continue;
                    }
                    let [x, y] = g.cell_xy(i as usize, j as usize);
                    let terrain = chunk_at(x, y).and_then(|_| benilla_formats::terrain_height_at(&chunks, [x, y, 0.0])).unwrap_or(0.0);
                    let floor = g.height(x, y);
                    let c = if g.is_blocked(x, y) {
                        '#'
                    } else if g.is_water(x, y) {
                        '~'
                    } else if g.is_wet(x, y) {
                        ','
                    } else if floor > terrain + 0.5 {
                        '='
                    } else if g.road(x, y) > 0.5 {
                        'r'
                    } else {
                        '.'
                    };
                    line.push(c);
                }
                println!("{py:>4} {line}");
            }
        }
        _ => anyhow::bail!("list or draw"),
    }
    Ok(())
}
