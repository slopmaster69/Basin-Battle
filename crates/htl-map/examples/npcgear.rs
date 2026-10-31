//! Who wears what: `npcgear <Data> armor <word>...` lists item displays whose model, textures or
//! icon hold a word; `npcgear <Data> wear <id>...` lists the character-model creature displays
//! wearing any of those item displays (race, sex, what they wear); `npcgear <Data> models
//! <word>...` lists creature displays whose model path holds a word, with their skin.
use benilla_formats::{load_creature_catalog, load_item_display_catalog, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    match args[2].as_str() {
        "armor" => {
            let cat = load_item_display_catalog(&mut chain)?;
            for word in &args[3..] {
                let w = word.to_lowercase();
                println!("== {word}");
                let mut rows: Vec<(u32, String)> = cat
                    .iter()
                    .filter(|(_, d)| {
                        let mut all: Vec<String> = d.model.iter().chain(d.model_texture.iter()).chain(d.region_textures.iter()).flatten().cloned().collect();
                        all.extend(d.icon.clone());
                        all.iter().any(|s| s.to_lowercase().contains(&w))
                    })
                    .map(|(id, d)| {
                        let regions: Vec<&String> = d.region_textures.iter().flatten().collect();
                        (id, format!("{:?} {:?} {:?} {:?}", d.icon.as_deref().map(|i| i.rsplit('\\').next().unwrap_or(i)), d.model[0], d.model_texture[0], regions))
                    })
                    .collect();
                rows.sort();
                for (id, s) in rows.iter().take(60) {
                    println!("{id}\t{s}");
                }
            }
        }
        "wear" => {
            let ids: Vec<u32> = args[3..].iter().filter_map(|s| s.parse().ok()).collect();
            let cat = load_creature_catalog(&mut chain)?;
            let mut rows: Vec<(u32, String)> = cat
                .display_models()
                .filter_map(|(d, _)| {
                    let m = cat.model(d)?;
                    let a = m.npc_appearance?;
                    a.equipment.iter().any(|e| ids.contains(e)).then(|| (d, format!("race {} sex {} gear {:?}", a.race, a.sex, a.equipment)))
                })
                .collect();
            rows.sort();
            for (d, s) in rows.iter().take(80) {
                println!("{d}\t{s}");
            }
        }
        // `show <display>...`: each creature display's model, skin and (a character model's) gear.
        "show" => {
            let cat = load_creature_catalog(&mut chain)?;
            for d in args[3..].iter().filter_map(|s| s.parse::<u32>().ok()) {
                match cat.model(d) {
                    Some(m) => println!(
                        "{d}\t{} {:?} scale {:.2} {}",
                        m.model_path,
                        m.textures,
                        m.scale,
                        m.npc_appearance.as_ref().map_or(String::new(), |a| format!("race {} sex {} gear {:?}", a.race, a.sex, a.equipment))
                    ),
                    None => println!("{d}\tno such display"),
                }
            }
        }
        _ => {
            let cat = load_creature_catalog(&mut chain)?;
            for word in &args[3..] {
                let w = word.to_lowercase();
                println!("== {word}");
                let mut rows: Vec<(u32, String)> = cat
                    .display_models()
                    .filter_map(|(d, _)| cat.model(d).map(|m| (d, m)))
                    .filter(|(_, m)| m.model_path.to_lowercase().contains(&w))
                    .map(|(d, m)| (d, format!("{} {:?} scale {:.2}", m.model_path, m.textures, m.scale)))
                    .collect();
                rows.sort();
                for (d, s) in rows.iter().take(40) {
                    println!("{d}\t{s}");
                }
            }
        }
    }
    Ok(())
}
