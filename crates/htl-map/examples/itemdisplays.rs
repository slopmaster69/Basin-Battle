//! Item displays whose model or icon matches a word: `itemdisplays <Data> arrow wand`. One
//! display per icon, up to 80 a word, with the display's first model.
use benilla_formats::{load_item_display_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_item_display_catalog(&mut chain)?;
    for word in &args[2..] {
        let w = word.to_lowercase();
        println!("== {word}");
        let mut n = 0;
        let mut icons = std::collections::BTreeSet::new();
        for id in 1..40000u32 {
            if let Some(d) = cat.get(id) {
                let s = format!("{:?} {:?}", d.model, d.icon).to_lowercase();
                if s.contains(&w) && icons.insert(d.icon.clone()) {
                    println!("{id}\t{}\t{}", d.icon.clone().unwrap_or_default(), d.model[0].clone().unwrap_or_default());
                    n += 1;
                    if n >= 80 {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}
