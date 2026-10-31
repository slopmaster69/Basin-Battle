//! Which spells and kits the content shows can't be drawn from the game data:
//! `spell_probe <Data> <content dir>`. Every `...spell... = N` and `...kit... = N` in the content's
//! files is checked (with the kits and spells the rules name in code), and what's missing printed:
//! a spell not in Spell.dbc, an effect with no row (the client's error cube), a model not there.
use std::collections::BTreeSet;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let data = std::path::Path::new(&args[1]);
    let dir = std::path::Path::new(&args[2]);
    let (mut spells, mut kits) = (BTreeSet::new(), BTreeSet::new());
    for f in ["survival.toml", "deadmines.toml", "siege.toml"] {
        let Ok(text) = std::fs::read_to_string(dir.join(f)) else { continue };
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') {
                continue;
            }
            // `key = 123` and `{ ..., spell = 123, ... }` alike.
            for part in line.split([',', '{', '}']) {
                let Some((k, v)) = part.split_once('=') else { continue };
                let (k, v) = (k.trim().to_ascii_lowercase(), v.trim());
                let Ok(n) = v.parse::<u32>() else { continue };
                if n == 0 {
                    continue;
                }
                if k.contains("spell") {
                    spells.insert(n);
                } else if k.contains("kit") {
                    kits.insert(n);
                }
            }
        }
    }
    let spells: Vec<u32> = spells.into_iter().collect();
    let kits: Vec<u32> = kits.into_iter().collect();
    println!("{} spells, {} kits", spells.len(), kits.len());
    for line in htl_map::spell_audit(data, &spells, &kits)? {
        println!("{line}");
    }
    Ok(())
}
