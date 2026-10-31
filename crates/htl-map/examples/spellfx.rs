//! `spellfx <Data> <word>...`: spells whose visual kits spawn a model whose path holds a word (the
//! impact kit's, the cast kit's or the state's), with the cast animation each plays, to find a
//! look to borrow; `spellfx <Data> missile <word>...` spells whose missile's model holds a word;
//! `spell <id>...` names those spells whatever they show. Read from the player's own data: never
//! commit the output.
use benilla_formats::{load_spell_catalog, load_spell_visual_catalog, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let visuals = load_spell_visual_catalog(&mut chain)?;
    let spells = load_spell_catalog(&mut chain)?;
    let only: Vec<u32> = args.iter().skip_while(|a| *a != "spell").skip(1).filter_map(|a| a.parse().ok()).collect();
    let missiles = args.get(2).is_some_and(|a| a == "missile");
    let words: Vec<String> = args[2..].iter().filter(|a| *a != "missile").take_while(|a| *a != "spell").map(|w| w.to_lowercase()).collect();
    let mut rows: Vec<_> = spells.iter().collect();
    rows.sort_by_key(|r| r.0);
    let mut shown = 0;
    for (id, s) in rows {
        let Some(st) = visuals.stages(s.visual) else {
            if only.contains(&id) {
                println!("spell {id} {:?} speed {} (no visual {})", s.name, s.speed, s.visual);
            }
            continue;
        };
        let mut hits = Vec::new();
        let missile = visuals.effect_path(st.missile_model).unwrap_or("").to_string();
        if missiles {
            let l = missile.to_lowercase();
            if !missile.is_empty() && words.iter().any(|w| l.contains(w.as_str())) {
                hits.push(format!("missile:{missile}"));
            }
        }
        for (stage, kit) in [("precast", st.precast), ("cast", st.cast), ("impact", st.impact), ("state", st.state)] {
            let Some(k) = visuals.kit(kit) else { continue };
            if missiles {
                continue;
            }
            for (_, e) in k.effects() {
                if let Some(p) = visuals.effect_path(e) {
                    let l = p.to_lowercase();
                    if words.is_empty() || words.iter().any(|w| l.contains(w.as_str())) {
                        hits.push(format!("{stage}[kit {kit}]:{p}"));
                    }
                }
            }
        }
        if (hits.is_empty() && !only.contains(&id)) || (!only.is_empty() && !only.contains(&id)) || shown >= 400 {
            continue;
        }
        shown += 1;
        let anim = |kit: u32| visuals.kit(kit).and_then(|k| k.anim_id).map_or("-".to_string(), |a| a.to_string());
        let m = if missile.is_empty() { String::new() } else { format!(" missile {missile}") };
        println!(
            "spell {id} {:?} speed {} precast anim {} cast anim {} impact anim {} state anim {}{m} | {}",
            s.name,
            s.speed,
            anim(st.precast),
            anim(st.cast),
            anim(st.impact),
            anim(st.state),
            hits.join(" ")
        );
    }
    Ok(())
}
