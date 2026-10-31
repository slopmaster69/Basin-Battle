//! `fxdest <Data> <spell>...`: what each spell shows at a ground point when it goes off there (its
//! visual's area effect, the one-shot a spell-go to a point plays), and whether the model ships;
//! `fxdest <Data> find <word>...`: the spells whose one-shot there holds a word, and whether they
//! play anything on the caster (a precast or cast kit) besides.
//! Read from the player's own data: never commit the output.
use benilla_formats::{load_spell_catalog, load_spell_visual_catalog, m2_file_name, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let visuals = load_spell_visual_catalog(&mut chain)?;
    let spells = load_spell_catalog(&mut chain)?;
    if args[2] == "find" {
        let words: Vec<String> = args[3..].iter().map(|w| w.to_lowercase()).collect();
        let mut rows: Vec<_> = spells.iter().collect();
        rows.sort_by_key(|r| r.0);
        for (id, s) in rows {
            let Some(st) = visuals.stages(s.visual) else { continue };
            let path = visuals.effect_path(st.area_effect).unwrap_or("");
            let l = path.to_lowercase();
            if st.missile_gate == 0 && !path.is_empty() && words.iter().any(|w| l.contains(w.as_str())) {
                let on_caster = st.precast != 0 || st.cast != 0;
                println!("{id}\t{:?}\t{path}\t{}", s.name, if on_caster { "caster kit" } else { "clean" });
            }
        }
        return Ok(());
    }
    for id in args[2..].iter().filter_map(|a| a.parse::<u32>().ok()) {
        let Some(s) = spells.get(id) else {
            println!("{id}: no spell");
            continue;
        };
        let Some(st) = visuals.stages(s.visual) else {
            println!("{id} {:?}: no visual", s.name);
            continue;
        };
        let path = visuals.effect_path(st.area_effect).unwrap_or("");
        let ok = m2_file_name(path).is_some_and(|f| chain.contains(&f));
        println!("{id} {:?}: gate {} area {} {path:?} {}", s.name, st.missile_gate, st.area_effect, if path.is_empty() { "-" } else if ok { "ok" } else { "MISSING" });
    }
    Ok(())
}
