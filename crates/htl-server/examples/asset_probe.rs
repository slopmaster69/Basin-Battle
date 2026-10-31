//! What of the content's looks the game data can't show: `asset_probe <Data> <survival.toml>`.
//! Every ability's spells and kits, every creature's blows, hazards, ranged weapons, and every
//! item's icon and display (with the display's own icon, a fallback), one line each, the
//! problems marked in capitals.
use htl_map::assets::Assets;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let a = Assets::open(std::path::Path::new(&args[1]))?;
    let lines = match args[2].as_str() {
        "--areas" => Some(a.area_models()),
        "--missiles" => Some(a.missile_models()),
        "--search" => Some(a.search(&args[3])),
        "--icons" => Some(a.displays_with_icon(&args[3])),
        "--creatures" => Some(a.creatures_like(&args[3])),
        "--objects" => Some(a.objects_like(&args[3])),
        // `--displays 2300 2370` (a range) or `--displays 1 5 9` (each): creature displays.
        "--displays" => {
            let n: Vec<u32> = args[3..].iter().filter_map(|x| x.parse().ok()).collect();
            let ids: Vec<u32> = if n.len() == 2 && n[1] > n[0] + 1 { (n[0]..=n[1]).collect() } else { n };
            Some(ids.into_iter().map(|d| a.creature(d)).collect())
        }
        "--gos" => Some(args[3..].iter().filter_map(|x| x.parse().ok()).map(|d| a.object(d)).collect()),
        _ => None,
    };
    if let Some(lines) = lines {
        for l in lines {
            println!("{l}");
        }
        return Ok(());
    }
    // `--outfit <creature display>...`: what each dressed NPC wears, slot by slot, with icons.
    if args[2] == "--outfit" {
        const SLOTS: [&str; 10] = ["head", "shoulders", "shirt", "chest", "belt", "legs", "boots", "wrist", "gloves", "tabard"];
        for d in args[3..].iter().filter_map(|x| x.parse::<u32>().ok()) {
            match a.npc_gear(d) {
                None => println!("{d}: not dressed"),
                Some(g) => {
                    println!("{d}:");
                    for (k, id) in g.iter().enumerate().filter(|(_, id)| **id != 0) {
                        let hair = if k == 0 { if a.helm_hides_hair(*id) { " hides hair".to_string() } else { format!(" HAIR SHOWS (like it that hide: {:?})", a.helms_like(*id).into_iter().take(3).collect::<Vec<_>>()) } } else { String::new() };
                        println!("    {:<9} {}{hair}", SLOTS[k], a.item_display(*id));
                    }
                }
            }
        }
        return Ok(());
    }
    // `--items <display>...`: item displays, each with its icon and look (helms: hair).
    if args[2] == "--items" {
        for d in args[3..].iter().filter_map(|x| x.parse::<u32>().ok()) {
            println!("{} | {}{}", a.item_display(d), a.display_look(d), if a.helm_hides_hair(d) { " | hides hair" } else { "" });
        }
        return Ok(());
    }
    // `--twins <display>...`: the same look with an icon.
    if args[2] == "--twins" {
        for d in args[3..].iter().filter_map(|x| x.parse::<u32>().ok()) {
            let t = a.twins(d);
            println!("{d}: {}", t.iter().take(4).map(|x| a.item_display(*x)).collect::<Vec<_>>().join(" || "));
        }
        return Ok(());
    }
    // `--wearing <part>`: item displays whose model or textures hold it (helms: whether hair shows).
    if args[2] == "--wearing" {
        for l in a.displays_wearing(&args[3]) {
            println!("{l}");
        }
        return Ok(());
    }
    // `--gear <survival.toml>`: every piece of armor a creature leaves, beside what that creature
    // wears in the slot (its NPC display's own item), and every helm checked for hair showing
    // through; a line ends "USE <display>" where the creature's own look (or a helm like it that
    // hides the hair) should be the piece's.
    if args[2] == "--gear" {
        let c = htl_survival::Content::load(std::path::Path::new(&args[3]))?;
        let slot_of = |s: &str| match s {
            "head" => Some(0usize),
            "shoulders" => Some(1),
            "chest" => Some(3),
            "legs" => Some(5),
            _ => None,
        };
        for m in c.mobs.iter().filter(|m| !m.gear.is_empty()) {
            let worn = a.npc_gear(m.display);
            println!("{} ({}), display {}: {}", m.key, m.name, m.display, if worn.is_some() { "dressed" } else { "not a dressed NPC" });
            for g in &m.gear {
                let Some(i) = c.item(g) else { continue };
                let it = &c.items[i];
                if it.kind != htl_survival::content::ItemKind::Armor {
                    continue;
                }
                let k = slot_of(&it.slot);
                let own = k.and_then(|k| worn.map(|w| w[k])).filter(|d| *d != 0);
                let hair = if it.slot == "head" { if a.helm_hides_hair(it.display) { "" } else { " HAIR SHOWS" } } else { "" };
                let mut line = format!("    {} ({}) is {}{hair}", it.key, it.slot, a.display_look(it.display));
                if let Some(d) = own {
                    line.push_str(&format!("; it wears {}", a.display_look(d)));
                    let pick = if it.slot == "head" && !a.helm_hides_hair(d) { a.helms_like(d).first().copied() } else { Some(d) };
                    match pick {
                        Some(p) if p != it.display => line.push_str(&format!(" -> USE {p}")),
                        Some(_) => line.push_str(" (the same)"),
                        None => line.push_str(" (its helm lets hair through; no like one that doesn't)"),
                    }
                }
                println!("{line}");
            }
        }
        // Every helm in the rules, worn by anyone.
        for it in c.items.iter().filter(|i| i.slot == "head") {
            if !a.helm_hides_hair(it.display) {
                println!("helm {} ({}) display {}: HAIR SHOWS; like it that don't: {:?}", it.key, it.name, it.display, a.helms_like(it.display).into_iter().take(5).collect::<Vec<_>>());
            }
        }
        return Ok(());
    }
    if args[2] == "--find" {
        for l in a.find(&args[3]) {
            println!("{l}");
        }
        return Ok(());
    }
    let c = htl_survival::Content::load(std::path::Path::new(&args[2]))?;
    let siege = c.siege.clone();
    println!("== abilities");
    for d in &c.abilities {
        let mut line = format!("{} [{}] {} icon {}{}", d.key, d.kind, d.name, d.icon, if a.icon_exists(&d.icon) { "" } else { " (ICON MISSING)" });
        if d.linger > 0.0 {
            line.push_str(&format!(" linger {}s", d.linger));
        }
        if d.delay > 0.0 {
            line.push_str(&format!(" delay {}s", d.delay));
        }
        println!("{line}");
        for (f, id) in [("spell", d.spell), ("charge_spell", d.charge_spell), ("full_spell", d.full_spell), ("burst_spell", d.burst_spell)] {
            if id != 0 {
                println!("    {f}: {}", a.spell(id));
            }
        }
        for (f, k) in [("full_kit", d.full_kit), ("impact_kit", d.impact_kit)] {
            if k != 0 {
                println!("    {f}: {}", a.kit(k));
            }
        }
    }
    println!("== creatures");
    for m in &c.mobs {
        let mut parts = Vec::new();
        if m.attack.spell != 0 {
            parts.push(format!("attack {}", a.spell(m.attack.spell)));
        }
        if let Some(s) = &m.special {
            if let Some(sp) = s.spell.filter(|x| *x != 0) {
                parts.push(format!("special {}", a.spell(sp)));
            }
            if s.kit != 0 {
                parts.push(format!("special {}", a.kit(s.kit)));
            }
        }
        for p in &m.patterns {
            for (i, s) in p.steps.iter().enumerate() {
                if let Some(sp) = s.spell.filter(|x| *x != 0) {
                    parts.push(format!("{} #{i} {}", p.name, a.spell(sp)));
                }
                if s.kit != 0 {
                    parts.push(format!("{} #{i} {}", p.name, a.kit(s.kit)));
                }
            }
        }
        for (f, d) in [("weapon", m.weapon), ("shield", m.shield), ("off_hand", m.off_hand), ("bow", m.bow)] {
            if d != 0 {
                parts.push(format!("{f} {}", a.item_display(d)));
            }
        }
        if !parts.is_empty() {
            println!("{} ({}):", m.key, m.name);
            for p in parts {
                println!("    {p}");
            }
        }
    }
    println!("== hazards");
    if let Some(dg) = &c.dungeon {
        for h in &dg.hazards {
            if h.spell != 0 {
                println!("{}: {}", h.name, a.spell(h.spell));
            }
        }
    }
    println!("== ranged");
    for r in &c.ranged {
        println!("{}: {}", r.key, a.spell(r.spell));
    }
    println!("== items");
    for it in &c.items {
        let icon_ok = a.icon_exists(&it.icon);
        let disp = if it.display != 0 { a.item_display(it.display) } else { "no display".into() };
        let mark = if !icon_ok { " (ICON MISSING)" } else { "" };
        let fallback = if !icon_ok { a.display_icon(it.display).map_or(String::new(), |i| format!(" -> display's icon {i}")) } else { String::new() };
        println!("{} \"{}\" icon {}{mark}{fallback}; {disp}", it.key, it.name, it.icon, );
    }
    println!("== stations");
    for s in &c.stations {
        if !a.icon_exists(&s.icon) {
            println!("{} icon {} (ICON MISSING)", s.key, s.icon);
        }
    }
    let _ = siege;
    Ok(())
}
