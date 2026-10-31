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
