//! `kitanims <Data> <anim>...`: the spells whose precast or cast kit plays one of these
//! `AnimationData` ids on the caster, with their visual, cast kits and speed.
use benilla_formats::{load_anim_data_catalog, load_spell_catalog, load_spell_visual_catalog, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let anims = load_anim_data_catalog(&mut chain)?;
    let visuals = load_spell_visual_catalog(&mut chain)?;
    let spells = load_spell_catalog(&mut chain)?;
    let want: Vec<u16> = args[2..].iter().take_while(|a| *a != "spell").filter_map(|a| a.parse().ok()).collect();
    let only: Vec<u32> = args.iter().skip_while(|a| *a != "spell").skip(1).filter_map(|a| a.parse().ok()).collect();
    let anim_of = |kit: u32| visuals.kit(kit).and_then(|k| k.anim_id);
    let mut found = 0;
    let mut rows: Vec<_> = spells.iter().collect();
    rows.sort_by_key(|r| r.0);
    for (id, s) in rows {
        let Some(st) = visuals.stages(s.visual) else { continue };
        let (pre, cast, imp) = (anim_of(st.precast), anim_of(st.cast), anim_of(st.impact));
        let hit = only.contains(&id) || [pre, cast, imp].iter().flatten().any(|a| want.contains(a));
        if hit && found < 60 {
            found += 1;
            let n = |a: Option<u16>| a.map(|a| format!("{a} {}", anims.name(a).unwrap_or("?"))).unwrap_or("-".into());
            let snd = |k: u32| visuals.kit(k).and_then(|k| k.sound).unwrap_or(0);
            let fx = |k: u32| visuals.kit(k).map_or(0, |k| k.effects().count());
            println!("spell {id} {:?} visual {} speed {} | precast {} | cast kit {} {} (snd {} fx {}) | impact kit {} {} (snd {} fx {}) | state {} (fx {} anim {}) | missile {}", s.name, s.visual, s.speed, n(pre), st.cast, n(cast), snd(st.cast), fx(st.cast), st.impact, n(imp), snd(st.impact), fx(st.impact), st.state, fx(st.state), n(anim_of(st.state)), st.missile_model);
        }
    }
    println!("{found} shown");
    Ok(())
}
