//! The clip census: every character model's attack clips, measured from your own WoW 1.12.1 data.
//!
//!   clip-census <Data folder> [--json]
//!
//! For each race and gender, every clip with an impact keyframe (the game's attacks, specials and
//! spell strikes), each take: how long it runs and when its blow lands, whether the body leaps,
//! flips, spins or lunges, how the weapon hand strikes (overhead, chop, sweep, thrust, upswing), and
//! what kind of blow that fits (a flurry, a cut, a heavy, a wide swing, a gap closer, a boss's
//! special). The table is the menu a move's or a foe's `from` picks from.
//!
//!   clip-census <Data folder> --displays <file> [--json]
//!
//! The creature census: for each creature display id in the file (numbers, one a line or
//! separated by spaces or commas; `#` starts a comment), its model and scale, then every attack-like
//! clip (one with an impact key, or an attack, special, spell or roar clip without one), each take:
//! its length and impact, how far the head moves into the blow, whether the body lunges, leaps,
//! spins or rears, and how big a wind-up a player sees before it lands (see [`creature`]).
//!
//!   clip-census <Data folder> --find <text>
//!
//! Every creature display whose model path holds the text (any case): its id, path and scale.

mod creature;
mod motion;

use std::path::PathBuf;

use anyhow::{Context, Result};
use benilla_formats::{
    load_anim_data_catalog, load_creature_catalog, m2_file_name, parse_m2_animations, parse_m2_attachments, parse_m2_bounds, parse_m2_skeleton,
    AnimDataCatalog, Chain, CreatureCatalog,
};
use glam::{Quat, Vec3};

use creature::{body_bone, describe_creature, head_bone, measure_creature, model_stem, parse_display_list, skeleton_extent, CreatureMotion};
use motion::{describe, key_bone, measure, Bone, Tracks};

const RACES: [&str; 9] = ["Human", "Orc", "Dwarf", "NightElf", "Scourge", "Tauren", "Gnome", "Troll", "Goblin"];
const IMPACTS: [&[u8; 4]; 5] = [b"$AH0", b"$AH1", b"$AH2", b"$AH3", b"$CAH"];

fn name_of(id: u16) -> &'static str {
    match id {
        0 => "Stand",
        1 => "Death",
        2 => "Spell",
        3 => "Stop",
        4 => "Walk",
        5 => "Run",
        6 => "Dead",
        7 => "Rise",
        8 => "StandWound",
        9 => "CombatWound",
        10 => "CombatCritical",
        11 => "ShuffleLeft",
        12 => "ShuffleRight",
        13 => "Walkbackwards",
        14 => "Stun",
        15 => "HandsClosed",
        16 => "AttackUnarmed",
        17 => "Attack1H",
        18 => "Attack2H",
        19 => "Attack2HL",
        20 => "ParryUnarmed",
        21 => "Parry1H",
        22 => "Parry2H",
        23 => "Parry2HL",
        24 => "ShieldBlock",
        25 => "ReadyUnarmed",
        26 => "Ready1H",
        27 => "Ready2H",
        28 => "Ready2HL",
        29 => "ReadyBow",
        30 => "Dodge",
        31 => "SpellPrecast",
        32 => "SpellCast",
        33 => "SpellCastArea",
        34 => "NPCWelcome",
        35 => "NPCGoodbye",
        36 => "Block",
        37 => "JumpStart",
        38 => "Jump",
        39 => "JumpEnd",
        40 => "Fall",
        41 => "SwimIdle",
        42 => "Swim",
        43 => "SwimLeft",
        44 => "SwimRight",
        45 => "SwimBackwards",
        46 => "AttackBow",
        47 => "FireBow",
        48 => "ReadyRifle",
        49 => "AttackRifle",
        50 => "Loot",
        51 => "ReadySpellDirected",
        52 => "ReadySpellOmni",
        53 => "SpellCastDirected",
        54 => "SpellCastOmni",
        55 => "BattleRoar",
        56 => "ReadyAbility",
        57 => "Special1H",
        58 => "Special2H",
        59 => "ShieldBash",
        74 => "EmoteRoar",
        85 => "Pierce",
        118 => "SpecialUnarmed",
        _ => "",
    }
}

/// The clips the creature census measures even without an impact key: the attacks, the specials,
/// the spell readies and casts, the roar and the ability ready.
const CREATURE_ATTACKS: [u16; 15] = [16, 17, 18, 19, 57, 58, 59, 85, 118, 51, 52, 53, 54, 55, 56];

/// A creature clip's name: the census's own, else `AnimationData.dbc`'s, else none.
fn creature_name(id: u16, anims: Option<&AnimDataCatalog>) -> String {
    match name_of(id) {
        "" => anims.and_then(|a| a.name(id)).unwrap_or("").to_string(),
        n => n.to_string(),
    }
}

/// One creature display resolved: what the census prints above its clips.
struct Display {
    id: u32,
    path: String,
    /// `CreatureModelData.modelScale × CreatureDisplayInfo.creatureModelScale`, and the display's
    /// own factor.
    scale: f32,
    display_scale: f32,
    /// The model's largest dimension (its bounding box's, else its skeleton's), unscaled yards: the
    /// measure every share in its rows is of.
    size: f32,
    skeleton: f32,
    collision_height: f32,
    head: &'static str,
    body: &'static str,
}

struct CreatureRow {
    anim: u16,
    name: String,
    take: usize,
    m: CreatureMotion,
    tags: Vec<&'static str>,
}

fn creature_census(chain: &mut Chain, catalog: &CreatureCatalog, anims: Option<&AnimDataCatalog>, id: u32) -> Result<(Display, Vec<CreatureRow>)> {
    let model = catalog.model(id).context("no such creature display")?;
    let file = m2_file_name(&model.model_path).with_context(|| format!("{} is no model file", model.model_path))?;
    let bytes = chain.read_file(&file).with_context(|| format!("reading {file}"))?;
    let skel = parse_m2_skeleton(&bytes)?;
    let bones: Vec<Bone> = skel.bones.iter().map(|b| Bone { parent: b.parent, pivot: Vec3::from(b.pivot), key: b.key_bone }).collect();
    let (head, head_how) = head_bone(&bones).context("the model has no bones")?;
    let body = body_bone(&bones);
    let skeleton = skeleton_extent(&bones);
    let boxed = parse_m2_bounds(&bytes).map(|b| (Vec3::from(b.bbox_max) - Vec3::from(b.bbox_min)).max_element()).unwrap_or(0.0);
    let collision_height = model.collision_height;
    let size = [boxed, skeleton, collision_height].into_iter().find(|s| s.is_finite() && *s > 0.1).unwrap_or(0.1);
    let display = Display {
        id,
        path: model.model_path.clone(),
        scale: model.scale,
        display_scale: catalog.display_scale(id).unwrap_or(1.0),
        size,
        skeleton,
        collision_height,
        head: head_how,
        body: match body {
            Some(b) if bones[b].key == 5 => "waist key bone",
            Some(_) => "low spine key bone",
            None => "centre of all bones",
        },
    };
    let mut takes: std::collections::HashMap<u16, usize> = Default::default();
    let mut rows = Vec::new();
    for seq in parse_m2_animations(&bytes) {
        let take = {
            let t = takes.entry(seq.anim_id).or_insert(0);
            *t += 1;
            *t - 1
        };
        let impact = seq.events.iter().filter(|e| IMPACTS.iter().any(|i| **i == e.ident)).map(|e| e.time).reduce(f32::min);
        if impact.is_none() && !CREATURE_ATTACKS.contains(&seq.anim_id) {
            continue;
        }
        let mut tracks = vec![Tracks::default(); bones.len()];
        for b in &seq.bones {
            let Some(t) = tracks.get_mut(b.bone as usize) else { continue };
            t.translation = b.translation.iter().map(|(s, v)| (*s, Vec3::from(*v))).collect();
            t.rotation = b.rotation.iter().map(|(s, q)| (*s, Quat::from_array(*q))).collect();
        }
        let m = measure_creature(&bones, &tracks, seq.duration, impact, head, body, size);
        let tags = describe_creature(&m);
        rows.push(CreatureRow { anim: seq.anim_id, name: creature_name(seq.anim_id, anims), take, m, tags });
    }
    rows.sort_by_key(|r| (r.anim, r.take));
    Ok((display, rows))
}

fn creature_main(chain: &mut Chain, list: &str, json: bool) -> Result<()> {
    let text = std::fs::read_to_string(list).with_context(|| format!("reading {list}"))?;
    let ids = parse_display_list(&text);
    anyhow::ensure!(!ids.is_empty(), "{list} names no display ids");
    let catalog = load_creature_catalog(chain).context("loading the creature tables")?;
    let anims = load_anim_data_catalog(chain).map_err(|e| eprintln!("no clip names from AnimationData.dbc: {e:#}")).ok();
    if !json {
        println!("# Creature clip census\n");
        println!("Every attack-like clip of each display: one with an impact key, or an attack, special, spell or roar clip without one (its Lands is blank and it is measured to the moment the head reaches farthest). Head, Body and Tell are shares of the model's size (its largest dimension, unscaled): Head is how far the head moves forward and up from the clip's first frame to the impact; Body is how far the body carries forward and its highest rise; Tell is how far the head moves from rest during the wind-up, up to the Coil (the moment it is farthest from where it lands, after which it is the blow). Spin is the whole body's turn. `small` is a tell under {:.2}.\n", creature::SMALL_TELL);
    }
    let mut measured = 0;
    for id in ids {
        let (d, rows) = match creature_census(chain, &catalog, anims.as_ref(), id) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("display {id}: {e:#}");
                if !json {
                    println!("## Display {id}\n\nNot measured: {e:#}\n");
                }
                continue;
            }
        };
        measured += rows.len();
        let stem = model_stem(&d.path);
        if json {
            for r in &rows {
                let m = &r.m;
                let impact = m.impact.map_or("null".to_string(), |t| format!("{t:.3}"));
                println!(
                    "{{\"display\":{},\"model\":{:?},\"path\":{:?},\"scale\":{:.3},\"display_scale\":{:.3},\"size\":{:.3},\"anim\":{},\"name\":{:?},\"take\":{},\"duration\":{:.3},\"impact\":{},\"strike\":{:.3},\"coil\":{:.3},\"head_fwd\":{:.3},\"head_up\":{:.3},\"body_fwd\":{:.3},\"body_rise\":{:.3},\"spin\":{:.0},\"tell\":{:.3},\"rear\":{:.3},\"head_bone\":{:?},\"body_bone\":{:?},\"tags\":{:?}}}",
                    d.id, stem, d.path, d.scale, d.display_scale, d.size, r.anim, r.name, r.take, m.duration, impact, m.strike, m.coil,
                    m.head_fwd, m.head_up, m.body_fwd, m.body_rise, m.spin, m.tell, m.rear, d.head, d.body, r.tags
                );
            }
            continue;
        }
        println!("## Display {}: {}\n", d.id, d.path);
        println!(
            "Scale {:.2} (display {:.2}); size {:.2} yd unscaled (skeleton {:.2}, collision height {:.2}); head: {}; body: {}.\n",
            d.scale, d.display_scale, d.size, d.skeleton, d.collision_height, d.head, d.body
        );
        if rows.is_empty() {
            println!("No attack-like clips.\n");
            continue;
        }
        println!("| Display | Model | Clip | Take | Length | Lands | Coil | Head (fwd / up) | Body (fwd / rise) | Spin | Tell | What it is |");
        println!("| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |");
        for r in &rows {
            let m = &r.m;
            let lands = m.impact.map_or(String::new(), |t| format!("{t:.2} s"));
            println!(
                "| {} | {} | {} {} | {} | {:.2} s | {} | {:.2} s | {:+.2} / {:+.2} | {:+.2} / {:.2} | {:.0}° | {:.2} | {} |",
                d.id, stem, r.anim, r.name, r.take, m.duration, lands, m.coil, m.head_fwd, m.head_up, m.body_fwd, m.body_rise, m.spin, m.tell,
                r.tags.join(", ")
            );
        }
        println!();
    }
    eprintln!("{measured} creature clips measured");
    Ok(())
}

fn find_main(chain: &mut Chain, text: &str) -> Result<()> {
    let catalog = load_creature_catalog(chain).context("loading the creature tables")?;
    let needle = text.to_ascii_lowercase();
    let mut found: Vec<(u32, String, f32, f32)> = catalog
        .display_models()
        .filter_map(|(id, _)| catalog.model(id).map(|m| (id, m)))
        .filter(|(_, m)| m.model_path.to_ascii_lowercase().contains(&needle))
        .map(|(id, m)| (id, m.model_path, m.scale, catalog.display_scale(id).unwrap_or(1.0)))
        .collect();
    found.sort_by_key(|f| f.0);
    for (id, path, scale, display_scale) in &found {
        println!("{id:>6}  {path}  scale {scale:.2} (display {display_scale:.2})");
    }
    eprintln!("{} displays with \"{text}\" in their model path", found.len());
    Ok(())
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
    let usage = "usage: clip-census <WoW 1.12.1 Data folder> [--displays <file> | --find <text>] [--json]";
    let data = PathBuf::from(args.first().context(usage)?);
    let json = args.iter().any(|a| a == "--json");
    let value_of = |flag: &str| -> Result<Option<&String>> {
        match args.iter().position(|a| a == flag) {
            Some(i) => args.get(i + 1).filter(|v| !v.starts_with("--")).map(Some).with_context(|| format!("{flag} needs a value; {usage}")),
            None => Ok(None),
        }
    };
    let (displays, find) = (value_of("--displays")?, value_of("--find")?);
    let mut chain = Chain::open(&data).with_context(|| format!("opening {}", data.display()))?;
    if let Some(list) = displays {
        return creature_main(&mut chain, list, json);
    }
    if let Some(text) = find {
        return find_main(&mut chain, text);
    }
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
