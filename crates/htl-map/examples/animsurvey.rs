//! Every animation a model has, for choosing move sets: `animsurvey <Data> <m2 path> [...]`.
//! Prints the AnimationData id and name, the variation, length, move speed, blend time, whether it
//! loops, and its event keys (impact `$AH*`/`$CAH`, sounds, footsteps) with their times. The
//! output is read from the game's data: never commit it.
use benilla_formats::{load_anim_data_catalog, parse_m2_animations, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let cat = load_anim_data_catalog(&mut chain)?;
    for path in &args[2..] {
        let bytes = chain.read_file(path)?;
        let anims = parse_m2_animations(&bytes);
        println!("{path}: {} sequences", anims.len());
        let mut seen = std::collections::HashMap::<u16, u32>::new();
        for a in &anims {
            let var = seen.entry(a.anim_id).or_insert(0);
            let name = cat.name(a.anim_id).unwrap_or("?");
            let events: Vec<String> = a
                .events
                .iter()
                .map(|e| format!("{}@{:.2}", String::from_utf8_lossy(&e.ident), e.time))
                .collect();
            println!(
                "{:4} {:24} v{} {:5.2}s move {:4.1} blend {:.2} {} freq {} {}",
                a.anim_id,
                name,
                var,
                a.duration,
                a.move_speed,
                a.blend_time,
                if a.looping { "loop" } else { "once" },
                a.frequency,
                events.join(" ")
            );
            *var += 1;
        }
    }
    Ok(())
}
