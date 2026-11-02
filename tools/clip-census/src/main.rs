//! The clip census: every character model's attack clips, measured from your own WoW 1.12.1 data.
//!
//!   clip-census <Data folder> [--json]
//!
//! For each race and gender, every clip with an impact keyframe (the game's attacks, specials and
//! spell strikes), each take: how long it runs and when its blow lands, whether the body leaps,
//! flips, spins or lunges, how the weapon hand strikes (overhead, chop, sweep, thrust, upswing), and
//! what kind of blow that fits (a flurry, a cut, a heavy, a wide swing, a gap closer, a boss's
//! special). The table is the menu a move's or a foe's `from` picks from.

mod motion;

use std::path::PathBuf;

use anyhow::{Context, Result};
use benilla_formats::{parse_m2_animations, parse_m2_attachments, parse_m2_skeleton, Chain};
use glam::{Quat, Vec3};

use motion::{describe, key_bone, measure, Bone, Tracks};

const RACES: [&str; 9] = ["Human", "Orc", "Dwarf", "NightElf", "Scourge", "Tauren", "Gnome", "Troll", "Goblin"];
const IMPACTS: [&[u8; 4]; 5] = [b"$AH0", b"$AH1", b"$AH2", b"$AH3", b"$CAH"];

fn name_of(id: u16) -> &'static str {
    match id {
        16 => "AttackUnarmed",
        17 => "Attack1H",
        18 => "Attack2H",
        19 => "Attack2HL",
        30 => "Dodge",
        46 => "AttackBow",
        49 => "AttackRifle",
        55 => "BattleRoar",
        57 => "Special1H",
        58 => "Special2H",
        59 => "ShieldBash",
        85 => "Pierce",
        118 => "SpecialUnarmed",
        _ => "",
    }
}

struct Row {
    model: String,
    anim: u16,
    take: usize,
    m: motion::Motion,
    tags: Vec<&'static str>,
    fits: Vec<&'static str>,
}

fn census(chain: &mut Chain, model: &str) -> Result<Vec<Row>> {
    let (race, gender) = model.split_at(model.len() - if model.ends_with("Female") { 6 } else { 4 });
    let path = format!("Character\\{race}\\{gender}\\{model}.m2");
    let bytes = chain.read_file(&path).with_context(|| format!("reading {path}"))?;
    let skel = parse_m2_skeleton(&bytes)?;
    let bones: Vec<Bone> = skel.bones.iter().map(|b| Bone { parent: b.parent, pivot: Vec3::from(b.pivot), key: b.key_bone }).collect();
    let attach = parse_m2_attachments(&bytes)?;
    let hand_of = |id: u16| attach.iter().find(|a| a.id == id).map(|a| a.bone as usize);
    let body = key_bone(&bones, 5).or_else(|| key_bone(&bones, 4)).context("no waist or spine key bone")?;
    let head = key_bone(&bones, 6).context("no head key bone")?;
    let hand = hand_of(1).context("no right-hand attachment")?;
    let off = hand_of(2);
    let mut takes: std::collections::HashMap<u16, usize> = Default::default();
    let mut rows = Vec::new();
    for seq in parse_m2_animations(&bytes) {
        let take = {
            let t = takes.entry(seq.anim_id).or_insert(0);
            *t += 1;
            *t - 1
        };
        let Some(impact) = seq.events.iter().find(|e| IMPACTS.iter().any(|i| **i == e.ident)).map(|e| e.time) else {
            continue;
        };
        let mut tracks = vec![Tracks::default(); bones.len()];
        for b in &seq.bones {
            let Some(t) = tracks.get_mut(b.bone as usize) else { continue };
            t.translation = b.translation.iter().map(|(s, v)| (*s, Vec3::from(*v))).collect();
            t.rotation = b.rotation.iter().map(|(s, q)| (*s, Quat::from_array(*q))).collect();
        }
        let m = measure(&bones, &tracks, seq.duration, impact, body, head, hand, off);
        let (tags, fits) = describe(&m);
        rows.push(Row { model: model.to_string(), anim: seq.anim_id, take, m, tags, fits });
    }
    Ok(rows)
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let data = PathBuf::from(args.first().context("usage: clip-census <WoW 1.12.1 Data folder> [--json]")?);
    let json = args.iter().any(|a| a == "--json");
    let mut chain = Chain::open(&data).with_context(|| format!("opening {}", data.display()))?;
    let mut rows = Vec::new();
    for race in RACES {
        for gender in ["Male", "Female"] {
            let model = format!("{race}{gender}");
            match census(&mut chain, &model) {
                Ok(r) => rows.extend(r),
                Err(e) => eprintln!("{model}: {e:#}"),
            }
        }
    }
    rows.sort_by(|a, b| (a.anim, &a.model, a.take).cmp(&(b.anim, &b.model, b.take)));
    if json {
        for r in &rows {
            let m = &r.m;
            println!(
                "{{\"model\":\"{}\",\"anim\":{},\"name\":\"{}\",\"take\":{},\"duration\":{:.3},\"impact\":{:.3},\"rise\":{:.2},\"tilt\":{:.0},\"spin\":{:.0},\"travel_fwd\":{:.2},\"travel_side\":{:.2},\"stroke_up\":{:.2},\"stroke_side\":{:.2},\"stroke_fwd\":{:.2},\"raised\":{:.2},\"speed\":{:.1},\"off_hand\":{},\"tags\":{:?},\"fits\":{:?}}}",
                r.model, r.anim, name_of(r.anim), r.take, m.duration, m.impact, m.rise, m.tilt, m.spin, m.travel_fwd, m.travel_side,
                m.stroke_up, m.stroke_side, m.stroke_fwd, m.raised, m.speed, m.off_hand_leads, r.tags, r.fits
            );
        }
        return Ok(());
    }
    println!("| Clip | Model | Take | Length | Lands | Rise | Tilt | Spin | Travel | Stroke (up / side / fwd) | Raised | What it is | Fits |");
    println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
    for r in &rows {
        let m = &r.m;
        println!(
            "| {} {} | {} | {} | {:.2} s | {:.2} s | {:.2} | {:.0}° | {:.0}° | {:.1} yd | {:+.2} / {:+.2} / {:+.2} | {:+.2} | {} | {} |",
            r.anim, name_of(r.anim), r.model, r.take, m.duration, m.impact, m.rise, m.tilt, m.spin,
            m.travel_fwd.hypot(m.travel_side), m.stroke_up, m.stroke_side, m.stroke_fwd, m.raised,
            r.tags.join(", "), r.fits.join(", ")
        );
    }
    eprintln!("{} clips measured", rows.len());
    Ok(())
}
