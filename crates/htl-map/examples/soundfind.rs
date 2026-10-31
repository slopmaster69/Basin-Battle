//! `soundfind <Data> <word>...`: the sound kits (`SoundEntries.dbc`) whose name or files hold a
//! word, for a creature's voice. Read from the player's own data: never commit the output.
use benilla_formats::{load_sound_kit_catalog, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let kits = load_sound_kit_catalog(&mut chain)?;
    let words: Vec<String> = args[2..].iter().map(|w| w.to_lowercase()).collect();
    for id in 1..40_000u32 {
        let Some(k) = kits.get(id) else { continue };
        let hay = format!("{} {}", k.name, k.files.iter().map(|f| f.0.as_str()).collect::<Vec<_>>().join(" ")).to_lowercase();
        if words.iter().all(|w| hay.contains(w.as_str())) {
            let files: Vec<&str> = k.files.iter().map(|f| f.0.as_str()).collect();
            println!("{id}\t{}\t{}", k.name, files.join(" "));
        }
    }
    Ok(())
}
