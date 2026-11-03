//! `spellfx <Data> <word>...`: spells whose visual kits spawn a model whose path holds a word (the
//! impact kit's, the cast kit's or the state's), with the cast animation each plays, to find a
//! look to borrow; `spellfx <Data> missile <word>...` spells whose missile's model holds a word;
//! `spell <id>...` names those spells whatever they show. Read from the player's own data: never
//! commit the output.
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let only: Vec<u32> = args.iter().skip_while(|a| *a != "spell").skip(1).filter_map(|a| a.parse().ok()).collect();
    let missiles = args.get(2).is_some_and(|a| a == "missile");
    let words: Vec<String> = args[2..].iter().filter(|a| *a != "missile").take_while(|a| *a != "spell").cloned().collect();
    for line in htl_map::spell_find(std::path::Path::new(&args[1]), &words, missiles, &only, 400)? {
        println!("{line}");
    }
    Ok(())
}
