//! List creature display ids whose model path matches a word: `displays <Data> chest banner`.
use benilla_formats::{load_creature_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_creature_catalog(&mut chain)?;
    let mut rows: Vec<(u32, String)> = cat
        .display_models()
        .filter_map(|(d, _)| cat.model(d).map(|m| (d, format!("{:?}", m))))
        .collect();
    rows.sort();
    for word in &args[2..] {
        let w = word.to_lowercase();
        println!("== {word}");
        for (d, m) in rows.iter().filter(|(_, m)| m.to_lowercase().contains(&w)).take(25) {
            println!("{d}\t{}", &m[..m.len().min(140)]);
        }
    }
    Ok(())
}
