//! Print the card picker's icon names: `icons <Data> <cards.toml>`.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let content = htl_sim::Content::load(std::path::Path::new(&args[2]).with_file_name("game.toml").as_path())?;
    let spells: Vec<u32> = content.cards.iter().map(|c| c.spell).collect();
    let icons = htl_map::spell_icons(std::path::Path::new(&args[1]), &spells)?;
    let missing: Vec<&str> = content.cards.iter().filter(|c| !icons.contains_key(&c.spell)).map(|c| c.key.as_str()).collect();
    println!("{} of {} cards have icons; missing: {:?}", content.cards.len() - missing.len(), content.cards.len(), missing);
    for c in content.cards.iter().take(5) {
        println!("{} -> {:?}", c.key, icons.get(&c.spell));
    }
    Ok(())
}
