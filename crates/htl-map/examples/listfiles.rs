//! The game files whose path holds every given word: `listfiles <Data> <word> [word...]`. Names
//! only, read from the player's own data; never commit the output.
use benilla_formats::Chain;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let chain = Chain::open(std::path::Path::new(&args[1]))?;
    let words: Vec<String> = args[2..].iter().map(|w| w.to_lowercase()).collect();
    for e in chain.list()? {
        let l = e.name.to_lowercase();
        if words.iter().all(|w| l.contains(w.as_str())) {
            println!("{}", e.name);
        }
    }
    Ok(())
}
