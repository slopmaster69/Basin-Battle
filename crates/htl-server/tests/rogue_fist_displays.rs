//! The fist weapons' looks (docs/rogue-roles.md, "Weapon paths"): every fist weapon's item display
//! is in the client's `ItemDisplayInfo`, wears a fist model (`hand_1h_*`), and its icon is the
//! one the item names; its off hand's look is its own left-hand model where 1.12 has one.
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

/// A fist weapon fills both of a Rogue's hands, so the off hand shows it too: where 1.12 keeps a
/// left-hand model of the fist (`<model>left`, as the claw has), the item's `off_display` is one
/// with that model and the same skin; where it keeps none (the knuckles), the item names none and
/// the off hand shows the same model.
#[test]
#[ignore]
fn every_fist_weapons_off_hand_look_is_its_left_hand_model_where_the_data_has_one() {
    let Some(data) = data_dir() else {
        eprintln!("No WoW data (WOW_DATA): skipped.");
        return;
    };
    let mut chain = Chain::open(&data).expect("the game data");
    let cat = load_item_display_catalog(&mut chain).expect("ItemDisplayInfo.dbc");
    let content = htl_survival::Content::builtin();
    let lower = |o: &Option<String>| o.clone().unwrap_or_default().to_ascii_lowercase();
    for w in content.weapons.iter().filter(|w| w.subclass == 13 && !w.key.contains("_honed")) {
        let item = &content.items[content.item(&w.key).unwrap()];
        let main = cat.get(item.display).unwrap();
        let left = lower(&main.model[0]).replace(".m2", "left.m2");
        let skin = lower(&main.model_texture[0]);
        let lefts: Vec<u32> = cat.iter().filter(|(_, d)| lower(&d.model[0]) == left && lower(&d.model_texture[0]) == skin).map(|(i, _)| i).collect();
        println!("{}: display {} off_display {} (left-hand displays with its skin: {lefts:?})", w.key, item.display, item.off_display);
        if lefts.is_empty() {
            assert_eq!(item.off_display, 0, "{}: 1.12 has no {left}, so the off hand shows the same model", w.key);
        } else {
            assert!(lefts.contains(&item.off_display), "{}: off_display {} is one of {lefts:?} ({left}, {skin})", w.key, item.off_display);
        }
    }
}
