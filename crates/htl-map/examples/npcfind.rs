//! `npcfind <Data> <race> <sex> <word>...`: dressed NPC displays (CreatureDisplayInfoExtra) of a
//! race (1 human, 3 dwarf, 7 gnome...) and sex whose worn items' body textures name a word
//! ("apron", "smith"), and item displays whose model or icon names a word with `items <word>`.
use benilla_formats::{load_creature_catalog, load_item_display_catalog, Chain};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let items = load_item_display_catalog(&mut chain)?;
    if args[2] == "items" {
        let w = args[3].to_lowercase();
        let mut rows: Vec<_> = items
            .iter()
            .filter(|(_, d)| {
                d.model[0].as_ref().is_some_and(|m| m.to_lowercase().contains(&w))
                    || d.model_texture[0].as_ref().is_some_and(|m| m.to_lowercase().contains(&w))
                    || d.icon.as_ref().is_some_and(|m| m.to_lowercase().contains(&w))
            })
            .map(|(id, d)| (id, format!("{:?} {:?} {:?}", d.model[0], d.model_texture[0], d.icon)))
            .collect();
        rows.sort();
        let limit: usize = args.get(4).and_then(|n| n.parse().ok()).unwrap_or(40);
        for r in rows.iter().take(limit) {
            println!("{}\t{}", r.0, r.1);
        }
        return Ok(());
    }
    let race: u8 = args[2].parse()?;
    let sex: u8 = args[3].parse()?;
    let words: Vec<String> = args[4..].iter().map(|w| w.to_lowercase()).collect();
    let cat = load_creature_catalog(&mut chain)?;
    let mut found = 0;
    let mut rows: Vec<_> = cat.display_models().map(|(d, _)| d).collect();
    rows.sort();
    for d in rows {
        let Some(m) = cat.model(d) else { continue };
        let Some(a) = m.npc_appearance.as_ref() else { continue };
        if a.race != race || a.sex != sex {
            continue;
        }
        let mut names = Vec::new();
        for e in a.equipment.iter().filter(|e| **e != 0) {
            if let Some(it) = items.get(*e) {
                for t in it.region_textures.iter().flatten() {
                    names.push(t.clone());
                }
            }
        }
        let all = names.join(" ").to_lowercase();
        if words.iter().any(|w| all.contains(w)) {
            println!("{d}\tequipment {:?}\t{}", a.equipment, names.join(" "));
            found += 1;
            if found > 25 {
                break;
            }
        }
    }
    Ok(())
}
