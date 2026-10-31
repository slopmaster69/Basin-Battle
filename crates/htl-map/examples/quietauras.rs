//! For each card's spell, find a spell with the same icon whose aura plays no visual on the
//! wearer: `quietauras <Data> <cards.toml>` prints `key spell aura`.
use benilla_formats::{load_spell_catalog, load_spell_visual_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let spells = load_spell_catalog(&mut chain)?;
    let visuals = load_spell_visual_catalog(&mut chain)?;
    let text = std::fs::read_to_string(&args[2])?;
    let doc: toml::Value = toml::from_str(&text)?;
    let quiet = |id: u32| -> bool {
        let Some(s) = spells.get(id) else { return false };
        s.visual == 0 || visuals.stages(s.visual).map_or(true, |v| v.state == 0)
    };
    let mut extra: Vec<(String, u32)> = vec![];
    for (k, id) in [("res_fire", 11129u32), ("res_frost", 11958), ("res_arcane", 12042), ("res_holy", 20216), ("res_shadow", 15473), ("res_nature", 16166), ("res_physical", 12292), ("shield", 17), ("divine", 642), ("thornbark", 467)] {
        extra.push((k.to_string(), id));
    }
    let cards = doc["card"].as_array().unwrap();
    for (key, id) in cards
        .iter()
        .map(|c| (c["key"].as_str().unwrap().to_string(), c["spell"].as_integer().unwrap() as u32))
        .chain(extra)
    {
        let icon = spells.get(id).and_then(|s| s.icon.clone());
        let pick = if quiet(id) {
            Some(id)
        } else {
            icon.as_ref().and_then(|icon| {
                spells
                    .iter()
                    .filter(|(sid, s)| s.icon.as_ref() == Some(icon) && quiet(*sid))
                    .map(|(sid, _)| sid)
                    .min()
            })
        };
        println!("{key} {id} {} {}", pick.map_or("NONE".to_string(), |p| p.to_string()), icon.unwrap_or_default());
    }
    Ok(())
}
