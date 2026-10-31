//! `emotes <Data> <id>... [spell <id>...]`: which animation each `Emotes.dbc` id plays (what
//! `SMSG_EMOTE` shows), and after `spell` each spell's name, visual and speed.
use benilla_formats::{load_anim_data_catalog, load_emote_sound_catalog, Chain};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut chain = Chain::open(std::path::Path::new(&args[1]))?;
    let emotes = load_emote_sound_catalog(&mut chain)?;
    let anims = load_anim_data_catalog(&mut chain)?;
    let split = args.iter().position(|a| a == "spell").unwrap_or(args.len());
    if split < args.len() {
        let spells = benilla_formats::load_spell_catalog(&mut chain)?;
        for a in &args[split + 1..] {
            let id: u32 = a.parse()?;
            match spells.get(id) {
                Some(s) => println!("spell {id}: {} visual {} speed {}", s.name, s.visual, s.speed),
                None => println!("spell {id}: none"),
            }
        }
    }
    if args.get(2).map(String::as_str) == Some("all") {
        for id in 0..500u32 {
            if let Some(anim) = emotes.anim(id) {
                println!("emote {id}: anim {anim} {:?} flags {:?} state {:?}", anims.name(anim as u16), emotes.emote_flags(id), emotes.state_anim(id));
            }
        }
        for id in 0..200u16 {
            if let Some(n) = anims.name(id) {
                println!("anim {id} {n}");
            }
        }
        return Ok(());
    }
    for a in &args[2..split] {
        let id: u32 = a.parse()?;
        let anim = emotes.anim(id);
        println!("emote {id}: anim {anim:?} {:?}", anim.and_then(|x| anims.name(x as u16)));
    }
    Ok(())
}
