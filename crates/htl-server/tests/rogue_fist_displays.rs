//! The fist weapons' looks (docs/rogue-roles.md, "Weapon paths"): every fist weapon's item display
//! is in the client's `ItemDisplayInfo`, wears a fist model (`hand_1h_*`), and its icon is the
//! one the item names.
//!
//! Needs the game data: `WOW_DATA`, or the first line of `settings-windows.txt`.
//! `cargo test --release -p htl-server --test rogue_fist_displays -- --ignored --nocapture`.

use std::path::PathBuf;

use benilla_formats::{load_item_display_catalog, Chain};

fn data_dir() -> Option<PathBuf> {
    if let Ok(d) = std::env::var("WOW_DATA") {
        return Some(PathBuf::from(d));
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for f in ["settings-windows.txt", "../Basin-Battle/settings-windows.txt"] {
        if let Ok(t) = std::fs::read_to_string(root.join(f)) {
            if let Some(l) = t.lines().next() {
                return Some(PathBuf::from(l.trim()));
            }
        }
    }
    None
}

#[test]
#[ignore]
fn every_fist_weapons_display_is_a_fist_in_the_clients_data() {
    let Some(data) = data_dir() else {
        eprintln!("No WoW data (WOW_DATA): skipped.");
        return;
    };
    let mut chain = Chain::open(&data).expect("the game data");
    let cat = load_item_display_catalog(&mut chain).expect("ItemDisplayInfo.dbc");
    let content = htl_survival::Content::builtin();
    let fists: Vec<_> = content.weapons.iter().filter(|w| w.subclass == 13 && !w.key.contains("_honed")).collect();
    assert!(fists.len() >= 2, "the fist weapons");
    for w in fists {
        let item = &content.items[content.item(&w.key).unwrap()];
        let d = cat.get(item.display).unwrap_or_else(|| panic!("{}: display {} is not in ItemDisplayInfo", w.key, item.display));
        let model = d.model[0].clone().unwrap_or_default().to_ascii_lowercase();
        let icon = d.icon.clone().unwrap_or_default().to_ascii_lowercase();
        println!("{}: display {} model {model} icon {icon}", w.key, item.display);
        assert!(model.starts_with("hand_1h_"), "{}: display {} is a fist model ({model})", w.key, item.display);
        assert!(icon.ends_with(&item.icon.to_ascii_lowercase()), "{}: the item's icon {} is its display's ({icon})", w.key, item.icon);
    }
}
