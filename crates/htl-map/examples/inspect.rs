//! Checks against the host's own game data, printing only verdicts:
//!   inspect <Data> files <path>...   which files the data has
//!   inspect <Data> icons             gear displays without an inventory icon
//!   inspect <Data> spells <id>...    a spell's name, icon and whether it draws an area on the ground
//!   inspect <Data> find <name>...    spells with exactly that name: id, rank, icon, passive
//!   inspect <Data> creatures <word>... creature displays whose model path holds the word
use benilla_formats::{load_creature_catalog, load_item_display_catalog, load_spell_catalog, load_spell_visual_catalog, Chain};
use htl_sim::items::{template, GearKey, ARCHETYPES, MAX_TIER, WEAPON_PIECE};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    match args[2].as_str() {
        "files" => {
            for f in &args[3..] {
                println!("{}\t{f}", if chain.contains(f) { "yes" } else { "NO" });
            }
        }
        "icons" => {
            let cat = load_item_display_catalog(&mut chain)?;
            let content = htl_sim::Content::builtin();
            let mut missing = std::collections::BTreeMap::new();
            let mut total = std::collections::BTreeSet::new();
            for class in 0..content.classes.len() {
                for tier in 0..=MAX_TIER {
                    for piece in 0..=WEAPON_PIECE {
                        for quality in 1..=5u8 {
                            for archetype in 0..ARCHETYPES {
                                let key = GearKey { class, tier, quality, piece, archetype };
                                let Some(t) = template(&content, key.entry()) else { continue };
                                total.insert(t.display_id);
                                if cat.get(t.display_id).and_then(|d| d.icon.clone()).is_none() {
                                    missing.entry(t.display_id).or_insert(t.name.clone());
                                }
                            }
                        }
                    }
                }
            }
            for (d, n) in &missing {
                println!("{d}\t{n}");
            }
            println!("{} displays, {} without an icon", total.len(), missing.len());
        }
        "gearcheck" => {
            // Every gear display, checked for the files the client draws it from.
            let cat = load_item_display_catalog(&mut chain)?;
            const DIRS: [&str; 8] = ["ArmUpperTexture", "ArmLowerTexture", "HandTexture", "TorsoUpperTexture", "TorsoLowerTexture", "LegUpperTexture", "LegLowerTexture", "FootTexture"];
            const RACES: [&str; 8] = ["Hu", "Or", "Dw", "Ni", "Sc", "Ta", "Gn", "Tr"];
            let content = htl_sim::Content::builtin();
            let mut seen = std::collections::BTreeMap::new();
            for class in 0..content.classes.len() {
                for tier in 0..=MAX_TIER {
                    for piece in 0..=WEAPON_PIECE {
                        for quality in 1..=5u8 {
                            for archetype in 0..ARCHETYPES {
                                let key = GearKey { class, tier, quality, piece, archetype };
                                let Some(t) = template(&content, key.entry()) else { continue };
                                seen.entry(t.display_id).or_insert((t.name.clone(), t.inventory_type));
                            }
                        }
                    }
                }
            }
            let stem = |m: &str| m.strip_suffix(".m2").unwrap_or(m).to_string();
            let mut bad = 0;
            for (d, (name, inv)) in &seen {
                let Some(row) = cat.get(*d) else {
                    println!("{d}\t{name}\tNO SUCH DISPLAY");
                    bad += 1;
                    continue;
                };
                let mut faults = Vec::new();
                for (i, t) in row.region_textures.iter().enumerate() {
                    let Some(t) = t else { continue };
                    let has = |sx: &str| chain.contains(&format!("Item\\TextureComponents\\{}\\{t}_{sx}.blp", DIRS[i]));
                    if !has("U") && !(has("M") && has("F")) {
                        faults.push(format!("{} {t}", DIRS[i]));
                    }
                }
                let dir = match inv {
                    1 => Some("Head"),
                    3 => Some("Shoulder"),
                    13 | 14 | 15 | 17 | 21 | 22 | 23 | 25 | 26 => Some(if *inv == 14 { "Shield" } else { "Weapon" }),
                    _ => None,
                };
                if let Some(dir) = dir {
                    for col in 0..2 {
                        let Some(m) = &row.model[col] else { continue };
                        if dir == "Head" {
                            let missing: Vec<String> = RACES
                                .iter()
                                .flat_map(|r| ["M", "F"].map(|sx| format!("{r}{sx}")))
                                .filter(|rs| !chain.contains(&format!("Item\\ObjectComponents\\Head\\{}_{rs}.m2", stem(m))))
                                .collect();
                            if !missing.is_empty() {
                                faults.push(format!("helm {m} missing for {missing:?}"));
                            }
                        } else if !chain.contains(&format!("Item\\ObjectComponents\\{dir}\\{m}")) {
                            faults.push(format!("model {dir}\\{m}"));
                        }
                        if let Some(tex) = &row.model_texture[col] {
                            if !chain.contains(&format!("Item\\ObjectComponents\\{dir}\\{tex}.blp")) {
                                faults.push(format!("texture {dir}\\{tex}"));
                            }
                        }
                    }
                    if row.model[0].is_none() && row.model[1].is_none() {
                        faults.push("no model".into());
                    }
                }
                // The display has to be the kind of piece the slot is.
                let has_region = |i: usize| row.region_textures[i].is_some();
                let fits = match inv {
                    1 => row.model[0].is_some(),
                    3 => row.model[0].is_some() && row.model[1].is_some(),
                    5 | 20 => has_region(3),
                    6 => row.region_textures[5].as_ref().is_some_and(|t| t.to_lowercase().contains("belt")),
                    7 => has_region(5) || has_region(6),
                    8 => has_region(7),
                    9 => has_region(1),
                    10 => has_region(2),
                    _ => true,
                };
                if !fits {
                    faults.push(format!("not a piece for inventory type {inv}"));
                }
                if !faults.is_empty() {
                    bad += 1;
                    println!("{d}\t{name}\t{}", faults.join("; "));
                }
            }
            println!("{} displays, {bad} with faults", seen.len());
        }
        "wands" => {
            // Wand displays: model, its shot's spell visual, and whether the model is there.
            let cat = load_item_display_catalog(&mut chain)?;
            let mut rows: Vec<_> = cat.iter().filter(|(_, d)| d.model[0].as_ref().is_some_and(|m| m.to_lowercase().starts_with("wand"))).collect();
            rows.sort_by_key(|(id, _)| *id);
            for (id, d) in rows {
                let m = d.model[0].clone().unwrap();
                let ok = chain.contains(&format!("Item\\ObjectComponents\\Weapon\\{m}"));
                println!("{id}\t{m}\tvisual {}\ticon {:?}\tmodel {}", d.spell_visual, d.icon, if ok { "ok" } else { "MISSING" });
            }
        }
        "finddisp" => {
            // finddisp <head|shoulder> <word>: displays whose model names hold the word, with
            // every file the client needs present.
            let cat = load_item_display_catalog(&mut chain)?;
            const RACES: [&str; 8] = ["Hu", "Or", "Dw", "Ni", "Sc", "Ta", "Gn", "Tr"];
            let head = args[3] == "head";
            let word = args[4].to_lowercase();
            let mut rows: Vec<_> = cat.iter().filter(|(_, d)| d.model[0].as_ref().is_some_and(|m| m.to_lowercase().contains(&word))).collect();
            rows.sort_by_key(|(id, _)| *id);
            for (id, d) in rows {
                let m = d.model[0].clone().unwrap();
                let stem = m.strip_suffix(".m2").unwrap_or(&m).to_string();
                let ok = if head {
                    RACES.iter().all(|r| ["M", "F"].iter().all(|sx| chain.contains(&format!("Item\\ObjectComponents\\Head\\{stem}_{r}{sx}.m2"))))
                        && d.model_texture[0].as_ref().is_some_and(|t| chain.contains(&format!("Item\\ObjectComponents\\Head\\{t}.blp")))
                } else {
                    (0..2).all(|c| {
                        d.model[c].as_ref().is_some_and(|m| chain.contains(&format!("Item\\ObjectComponents\\Shoulder\\{m}")))
                            && d.model_texture[c].as_ref().is_some_and(|t| chain.contains(&format!("Item\\ObjectComponents\\Shoulder\\{t}.blp")))
                    })
                };
                if ok {
                    println!("{id}\t{m}\t{:?}\ticon {:?}", d.model_texture[0], d.icon);
                }
            }
        }
        "displays" => {
            let cat = load_item_display_catalog(&mut chain)?;
            const DIRS: [&str; 8] = ["ArmUpperTexture", "ArmLowerTexture", "HandTexture", "TorsoUpperTexture", "TorsoLowerTexture", "LegUpperTexture", "LegLowerTexture", "FootTexture"];
            for id in &args[3..] {
                let id: u32 = id.parse()?;
                let Some(d) = cat.get(id) else {
                    println!("{id}\tNO SUCH DISPLAY");
                    continue;
                };
                let mut out = format!("{id}\tmodel {:?} tex {:?} geo {:?} vis {:?}", d.model, d.model_texture, d.geoset_groups, d.helmet_vis);
                for (i, t) in d.region_textures.iter().enumerate() {
                    let Some(t) = t else { continue };
                    let have: Vec<&str> = ["U", "M", "F"]
                        .into_iter()
                        .filter(|sx| chain.contains(&format!("Item\\TextureComponents\\{}\\{t}_{sx}.blp", DIRS[i])))
                        .collect();
                    out += &format!(" | {}:{t} {:?}", &DIRS[i][..DIRS[i].len() - 7], have);
                }
                println!("{out}");
            }
        }
        "spells" => {
            let spells = load_spell_catalog(&mut chain)?;
            let visuals = load_spell_visual_catalog(&mut chain)?;
            for id in &args[3..] {
                let id: u32 = id.parse()?;
                match spells.get(id) {
                    Some(s) => {
                        let v = visuals.stages(s.visual);
                        println!(
                            "{id}\t{}\ticon {:?}\tarea {}",
                            s.name,
                            s.icon,
                            v.map_or("none".to_string(), |v| format!("gate {} effect {}", v.area_gate, v.area_effect))
                        );
                    }
                    None => println!("{id}\tNO SUCH SPELL"),
                }
            }
        }
        "find" => {
            let spells = load_spell_catalog(&mut chain)?;
            let want: Vec<String> = args[3..].iter().map(|s| s.to_lowercase()).collect();
            let mut rows: Vec<(u32, String)> = spells
                .iter()
                .filter(|(_, s)| want.iter().any(|w| s.name.to_lowercase() == *w))
                .map(|(id, s)| {
                    (id, format!("{id}\t{}\t{:?}\ticon {:?}\tpassive {}\tvisual {}", s.name, s.rank, s.icon, s.passive, s.visual))
                })
                .collect();
            rows.sort();
            for (_, r) in rows {
                println!("{r}");
            }
        }
        "creatures" => {
            let cat = load_creature_catalog(&mut chain)?;
            let want: Vec<String> = args[3..].iter().map(|s| s.to_lowercase()).collect();
            for id in 1..20000u32 {
                if let Some(m) = cat.model(id) {
                    let path = m.model_path.to_lowercase();
                    if want.iter().any(|w| path.contains(w.as_str())) {
                        println!("{id}\t{}\t{:?}\tscale {}", m.model_path, m.textures, m.scale);
                    }
                }
            }
        }
        "gearcheck" => {
            // Every gear display, checked for the files the client draws it from.
            let cat = load_item_display_catalog(&mut chain)?;
            const DIRS: [&str; 8] = ["ArmUpperTexture", "ArmLowerTexture", "HandTexture", "TorsoUpperTexture", "TorsoLowerTexture", "LegUpperTexture", "LegLowerTexture", "FootTexture"];
            const RACES: [&str; 8] = ["Hu", "Or", "Dw", "Ni", "Sc", "Ta", "Gn", "Tr"];
            let content = htl_sim::Content::builtin();
            let mut seen = std::collections::BTreeMap::new();
            for class in 0..content.classes.len() {
                for tier in 0..=MAX_TIER {
                    for piece in 0..=WEAPON_PIECE {
                        for quality in 1..=5u8 {
                            for archetype in 0..ARCHETYPES {
                                let key = GearKey { class, tier, quality, piece, archetype };
                                let Some(t) = template(&content, key.entry()) else { continue };
                                seen.entry(t.display_id).or_insert((t.name.clone(), t.inventory_type));
                            }
                        }
                    }
                }
            }
            let stem = |m: &str| m.strip_suffix(".m2").unwrap_or(m).to_string();
            let mut bad = 0;
            for (d, (name, inv)) in &seen {
                let Some(row) = cat.get(*d) else {
                    println!("{d}\t{name}\tNO SUCH DISPLAY");
                    bad += 1;
                    continue;
                };
                let mut faults = Vec::new();
                for (i, t) in row.region_textures.iter().enumerate() {
                    let Some(t) = t else { continue };
                    let has = |sx: &str| chain.contains(&format!("Item\\TextureComponents\\{}\\{t}_{sx}.blp", DIRS[i]));
                    if !has("U") && !(has("M") && has("F")) {
                        faults.push(format!("{} {t}", DIRS[i]));
                    }
                }
                let dir = match inv {
                    1 => Some("Head"),
                    3 => Some("Shoulder"),
                    13 | 14 | 15 | 17 | 21 | 22 | 23 | 25 | 26 => Some(if *inv == 14 { "Shield" } else { "Weapon" }),
                    _ => None,
                };
                if let Some(dir) = dir {
                    for col in 0..2 {
                        let Some(m) = &row.model[col] else { continue };
                        if dir == "Head" {
                            let missing: Vec<String> = RACES
                                .iter()
                                .flat_map(|r| ["M", "F"].map(|sx| format!("{r}{sx}")))
                                .filter(|rs| !chain.contains(&format!("Item\\ObjectComponents\\Head\\{}_{rs}.m2", stem(m))))
                                .collect();
                            if !missing.is_empty() {
                                faults.push(format!("helm {m} missing for {missing:?}"));
                            }
                        } else if !chain.contains(&format!("Item\\ObjectComponents\\{dir}\\{m}")) {
                            faults.push(format!("model {dir}\\{m}"));
                        }
                        if let Some(tex) = &row.model_texture[col] {
                            if !chain.contains(&format!("Item\\ObjectComponents\\{dir}\\{tex}.blp")) {
                                faults.push(format!("texture {dir}\\{tex}"));
                            }
                        }
                    }
                    if row.model[0].is_none() && row.model[1].is_none() {
                        faults.push("no model".into());
                    }
                }
                if !faults.is_empty() {
                    bad += 1;
                    println!("{d}\t{name}\t{}", faults.join("; "));
                }
            }
            println!("{} displays, {bad} with faults", seen.len());
        }
        "wands" => {
            // Wand displays: model, its shot's spell visual, and whether the model is there.
            let cat = load_item_display_catalog(&mut chain)?;
            let mut rows: Vec<_> = cat.iter().filter(|(_, d)| d.model[0].as_ref().is_some_and(|m| m.to_lowercase().starts_with("wand"))).collect();
            rows.sort_by_key(|(id, _)| *id);
            for (id, d) in rows {
                let m = d.model[0].clone().unwrap();
                let ok = chain.contains(&format!("Item\\ObjectComponents\\Weapon\\{m}"));
                println!("{id}\t{m}\tvisual {}\ticon {:?}\tmodel {}", d.spell_visual, d.icon, if ok { "ok" } else { "MISSING" });
            }
        }
        "finddisp" => {
            // finddisp <head|shoulder> <word>: displays whose model names hold the word, with
            // every file the client needs present.
            let cat = load_item_display_catalog(&mut chain)?;
            const RACES: [&str; 8] = ["Hu", "Or", "Dw", "Ni", "Sc", "Ta", "Gn", "Tr"];
            let head = args[3] == "head";
            let word = args[4].to_lowercase();
            let mut rows: Vec<_> = cat.iter().filter(|(_, d)| d.model[0].as_ref().is_some_and(|m| m.to_lowercase().contains(&word))).collect();
            rows.sort_by_key(|(id, _)| *id);
            for (id, d) in rows {
                let m = d.model[0].clone().unwrap();
                let stem = m.strip_suffix(".m2").unwrap_or(&m).to_string();
                let ok = if head {
                    RACES.iter().all(|r| ["M", "F"].iter().all(|sx| chain.contains(&format!("Item\\ObjectComponents\\Head\\{stem}_{r}{sx}.m2"))))
                        && d.model_texture[0].as_ref().is_some_and(|t| chain.contains(&format!("Item\\ObjectComponents\\Head\\{t}.blp")))
                } else {
                    (0..2).all(|c| {
                        d.model[c].as_ref().is_some_and(|m| chain.contains(&format!("Item\\ObjectComponents\\Shoulder\\{m}")))
                            && d.model_texture[c].as_ref().is_some_and(|t| chain.contains(&format!("Item\\ObjectComponents\\Shoulder\\{t}.blp")))
                    })
                };
                if ok {
                    println!("{id}\t{m}\t{:?}\ticon {:?}", d.model_texture[0], d.icon);
                }
            }
        }
        "displays" => {
            // displays <from> <to>: item displays in a range with an icon and their leg textures.
            let cat = load_item_display_catalog(&mut chain)?;
            let (from, to): (u32, u32) = (args[3].parse()?, args[4].parse()?);
            for id in from..=to {
                if let Some(d) = cat.get(id) {
                    println!("{id}\ticon {:?}\tlegs {:?}", d.icon, &d.region_textures[5..7]);
                }
            }
        }
        other => anyhow::bail!("unknown mode {other}"),
    }
    Ok(())
}
