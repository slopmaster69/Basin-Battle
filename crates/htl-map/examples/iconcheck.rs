//! `iconcheck <Data> <icon>...`: whether each Interface\Icons texture ships.
use benilla_formats::Chain;
fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let chain = Chain::open(std::path::Path::new(&args[1]))?;
    for name in &args[2..] {
        let ok = chain.contains(&format!("Interface\\Icons\\{name}.blp"));
        println!("{name}\t{}", if ok { "ok" } else { "MISSING" });
    }
    Ok(())
}
