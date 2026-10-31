//! Print item displays by id: `itemdisplay <Data> 1817 1818`.
use benilla_formats::{load_item_display_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_item_display_catalog(&mut chain)?;
    for id in &args[2..] {
        let id: u32 = id.parse()?;
        println!("{id}: {:?}", cat.get(id));
    }
    Ok(())
}
