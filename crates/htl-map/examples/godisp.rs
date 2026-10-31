//! Game object displays: `godisp <Data> find <word>...` lists displays whose model path holds a
//! word; `godisp <Data> <display>...` measures each one's mesh (its footprint on the ground).
use benilla_formats::{load_gameobject_catalog, load_object_model, m2_file_name, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_gameobject_catalog(&mut chain)?;
    if args[2] == "find" {
        for word in &args[3..] {
            let w = word.to_lowercase();
            let mut rows: Vec<(u32, String)> = cat.iter().filter(|(_, p)| p.to_lowercase().contains(&w)).map(|(d, p)| (d, p.to_string())).collect();
            rows.sort();
            println!("== {word}");
            for (d, p) in rows.iter().take(60) {
                let ok = m2_file_name(p).is_some_and(|f| chain.contains(&f));
                println!("{d}\t{p}\t{}", if ok { "ok" } else { "MISSING" });
            }
        }
        return Ok(());
    }
    for id in &args[2..] {
        let d: u32 = id.parse()?;
        let Some(path) = cat.model_path(d).map(str::to_string) else {
            println!("{d}\tno such display");
            continue;
        };
        match load_object_model(&mut chain, &path) {
            Ok(meshes) => {
                let mut lo = [f32::MAX; 3];
                let mut hi = [f32::MIN; 3];
                let mut n = 0;
                for m in &meshes {
                    for p in &m.positions {
                        n += 1;
                        for k in 0..3 {
                            lo[k] = lo[k].min(p[k]);
                            hi[k] = hi[k].max(p[k]);
                        }
                    }
                }
                let tex: Vec<String> = meshes.iter().filter_map(|m| m.texture.clone()).collect();
                println!("{d}\t{path}\t{} meshes {n} verts x {:.2}..{:.2} y {:.2}..{:.2} z {:.2}..{:.2} {:?}", meshes.len(), lo[0], hi[0], lo[1], hi[1], lo[2], hi[2], tex);
            }
            Err(e) => println!("{d}\t{path}\terror {e}"),
        }
    }
    Ok(())
}
