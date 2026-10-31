//! Which gear display ids have no inventory icon: `itemicons <Data>`.
use benilla_formats::{load_item_display_catalog, Chain};
use htl_sim::items::{template, tier_quality, GearKey, MAX_TIER, WEAPON_PIECE};
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_item_display_catalog(&mut chain)?;
    let content = htl_sim::Content::builtin();
    let mut seen = std::collections::BTreeMap::new();
    for class in 0..content.classes.len() {
        for tier in 0..=MAX_TIER {
            for piece in 0..=WEAPON_PIECE {
                for q in [1u8, 2, 3, 4, 5] {
                    let key = GearKey { class, tier, quality: q, piece, archetype: 0 };
                    if let Some(t) = template(&content, key.entry()) {
                        let icon = cat.get(t.display_id).and_then(|d| d.icon.clone());
                        seen.entry(t.display_id).or_insert((t.name.clone(), icon));
                    }
                }
            }
        }
    }
    let _ = tier_quality(0);
    for (d, (name, icon)) in &seen {
        if icon.is_none() {
            println!("{d}\t{name}\tNO ICON (or not in this data)");
        }
    }
    println!("{} displays, {} without an icon", seen.len(), seen.values().filter(|v| v.1.is_none()).count());
    Ok(())
}
