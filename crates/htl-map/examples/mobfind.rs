//! Creature displays by model word, compact: `mobfind <Data> kobold murloc ...` prints each
//! display id, its model's file name, scale and texture variations.
use benilla_formats::{load_creature_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_creature_catalog(&mut chain)?;
    let mut ids: Vec<u32> = cat.display_models().map(|(d, _)| d).collect();
    ids.sort();
    for word in &args[2..] {
        let w = word.to_lowercase();
        println!("== {word}");
        let mut n = 0;
        for d in &ids {
            let Some(m) = cat.model(*d) else { continue };
            if !m.model_path.to_lowercase().contains(&w) || m.npc_appearance.is_some() {
                continue;
            }
            let file = m.model_path.rsplit('\\').next().unwrap_or("").to_string();
            let tex: Vec<String> = m.textures.iter().flatten().cloned().collect();
            println!("{d}\t{file}\t{:.2}\t{}", m.scale, tex.join(","));
            n += 1;
            if n >= 80 {
                break;
            }
        }
    }
    Ok(())
}
