//! Print every card's text as the picker shows it: `cargo run -p htl-sim --example cardtext [hit]`.
fn main() {
    let hit: f32 = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(50.0);
    let content = htl_sim::Content::builtin();
    for def in &content.cards {
        let t = htl_sim::describe::card_text(def, 1, hit, 1.0, false);
        let up = htl_sim::describe::card_text(def, 2, hit, 1.0, false).upgrade;
        println!("{} [{:?} {} {} {}s] {}  || r2: {}", def.name, def.rarity, t.kind, t.school, t.cooldown, t.text, up);
    }
}
