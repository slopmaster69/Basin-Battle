//! A creature display's model: `creaturedisp <Data> <display>...`; or item displays whose weapon
//! model holds a word, with the model present: `creaturedisp <Data> items <word>`.
use benilla_formats::{load_creature_catalog, load_item_display_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    if args[2] == "items" {
        let cat = load_item_display_catalog(&mut chain)?;
        let word = args[3].to_lowercase();
        let mut rows: Vec<_> = cat.iter().filter(|(_, d)| d.model[0].as_ref().is_some_and(|m| m.to_lowercase().contains(&word))).collect();
        rows.sort_by_key(|(id, _)| *id);
        for (id, d) in rows.into_iter().take(40) {
            let m = d.model[0].clone().unwrap();
            let ok = chain.contains(&format!("Item\\ObjectComponents\\Weapon\\{m}"));
            println!("{id}\t{m}\t{:?}\t{}", d.model_texture[0], if ok { "ok" } else { "MISSING" });
        }
        return Ok(());
    }
    let cat = load_creature_catalog(&mut chain)?;
    if args[2] == "find" {
        for word in &args[3..] {
            let w = word.to_lowercase();
            let mut rows: Vec<(u32, String)> = cat
                .display_models()
                .filter_map(|(d, _)| cat.model(d).map(|m| (d, m.model_path.clone(), m.textures[0].clone())))
                .filter(|(_, p, _)| p.to_lowercase().contains(&w))
                .map(|(d, p, t)| (d, format!("{p} {t:?}")))
                .collect();
            rows.sort();
            println!("== {word}");
            for (d, m) in rows.iter().take(14) {
                println!("{d}\t{m}");
            }
        }
        return Ok(());
    }
    for id in &args[2..] {
        let d: u32 = id.parse()?;
        println!("{d}\t{:?}", cat.model(d));
    }
    Ok(())
}
