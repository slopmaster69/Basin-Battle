//! The Hero tab's wares (`items.toml`): rings, necklaces, trinkets and cloaks that change how a
//! hero plays, for the slots the class sets leave empty.

use anyhow::{bail, Context, Result};

use crate::content::ClassDef;
use crate::items::{slot, QUALITY_EPIC, QUALITY_LEGENDARY, QUALITY_RARE, QUALITY_UNCOMMON};
use crate::kit::{parse_effects, Kit, TalentEffect, ITEM_STATS};

/// Where a ware is worn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WareSlot {
    Neck,
    Finger,
    Trinket,
    Back,
    /// A helm in place of the set's: a look and an effect for a set piece's bonus.
    Head,
    Tabard,
}

impl WareSlot {
    /// The equipment slots it fits, as the client numbers them.
    pub fn slots(self) -> &'static [u8] {
        match self {
            WareSlot::Neck => &[slot::NECK],
            WareSlot::Finger => &[slot::FINGER1, slot::FINGER2],
            WareSlot::Trinket => &[slot::TRINKET1, slot::TRINKET2],
            WareSlot::Back => &[slot::BACK],
            WareSlot::Head => &[slot::HEAD],
            WareSlot::Tabard => &[slot::TABARD],
        }
    }

    /// The client's inventory type.
    pub fn inventory_type(self) -> u32 {
        match self {
            WareSlot::Neck => 2,
            WareSlot::Finger => 11,
            WareSlot::Trinket => 12,
            WareSlot::Back => 16,
            WareSlot::Head => 1,
            WareSlot::Tabard => 19,
        }
    }

    pub fn noun(self) -> &'static str {
        match self {
            WareSlot::Neck => "Neck",
            WareSlot::Finger => "Ring",
            WareSlot::Trinket => "Trinket",
            WareSlot::Back => "Cloak",
            WareSlot::Head => "Helm",
            WareSlot::Tabard => "Tabard",
        }
    }

    /// How much of a chest's stat points it carries.
    pub fn weight(self) -> f32 {
        match self {
            WareSlot::Back => 0.8,
            WareSlot::Head => 1.0,
            WareSlot::Tabard => 0.5,
            _ => 0.9,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WareDef {
    pub key: String,
    pub name: String,
    pub slot: WareSlot,
    /// 2 uncommon to 5 legendary.
    pub quality: u8,
    pub display: u32,
    /// The icon's name, for the shop card.
    pub icon: String,
    pub class: Option<usize>,
    pub from_wave: u32,
    /// Shares of the slot's stat points: `stamina`, `strength`, `agility`, `intellect`, `spirit`
    /// or `main`.
    pub stats: Vec<(String, f32)>,
    pub text: String,
    /// What it does while worn, by class (its spell patches read against that class's spells).
    pub effects: Vec<Vec<TalentEffect>>,
}

impl WareDef {
    pub fn effects_for(&self, class: usize) -> &[TalentEffect] {
        self.effects.get(class).map_or(&[], |v| v.as_slice())
    }
}

/// Read `items.toml` against the classes and their kits.
pub fn parse(text: &str, classes: &[ClassDef], kits: &[Kit]) -> Result<Vec<WareDef>> {
    let root: toml::Table = toml::from_str(text).context("reading items.toml")?;
    let mut out = Vec::new();
    let Some(list) = root.get("item") else { return Ok(out) };
    for t in list.as_array().context("items.toml: [[item]]s")? {
        let t = t.as_table().context("an [[item]]")?;
        let s = |k: &str| -> Result<String> { t.get(k).and_then(|v| v.as_str()).map(str::to_string).with_context(|| format!("item needs {k}")) };
        let key = s("key")?;
        let what = format!("item {key}");
        let slot = match s("slot")?.as_str() {
            "neck" => WareSlot::Neck,
            "finger" => WareSlot::Finger,
            "trinket" => WareSlot::Trinket,
            "back" => WareSlot::Back,
            "head" => WareSlot::Head,
            "tabard" => WareSlot::Tabard,
            other => bail!("{what}: no slot called {other}"),
        };
        let quality = match s("quality")?.as_str() {
            "uncommon" => QUALITY_UNCOMMON,
            "rare" => QUALITY_RARE,
            "epic" => QUALITY_EPIC,
            "legendary" => QUALITY_LEGENDARY,
            other => bail!("{what}: no quality called {other}"),
        };
        let int = |k: &str| t.get(k).and_then(|v| v.as_integer()).map(|v| v as u32);
        let class = match t.get("class").and_then(|v| v.as_str()) {
            None => None,
            Some(c) => Some(classes.iter().position(|x| x.key == c).with_context(|| format!("{what}: no class {c}"))?),
        };
        let mut stats = Vec::new();
        if let Some(tab) = t.get("stats").and_then(|v| v.as_table()) {
            for (k, v) in tab {
                if !matches!(k.as_str(), "stamina" | "strength" | "agility" | "intellect" | "spirit" | "main") {
                    bail!("{what}: stats.{k} isn't a stat");
                }
                let v = v.as_float().or_else(|| v.as_integer().map(|i| i as f64)).with_context(|| format!("{what}: stats.{k}"))?;
                stats.push((k.clone(), v as f32));
            }
        }
        let mut effects = Vec::new();
        for (ci, kit) in kits.iter().enumerate() {
            let mine = class.is_none_or(|c| c == ci);
            effects.push(match (t.get("effect"), mine) {
                (Some(list), true) => parse_effects(kit, list, &what, ITEM_STATS).with_context(|| format!("{what} for {}", classes[ci].key))?,
                _ => Vec::new(),
            });
        }
        if out.iter().any(|w: &WareDef| w.key == key) {
            bail!("{what}: two items with that key");
        }
        out.push(WareDef {
            name: s("name")?,
            slot,
            quality,
            display: int("display").with_context(|| format!("{what}: display"))?,
            icon: s("icon")?,
            class,
            from_wave: int("from_wave").unwrap_or(1),
            stats,
            text: s("text")?,
            effects,
            key,
        });
    }
    if out.len() > 99 {
        bail!("items.toml: 99 items at most");
    }
    Ok(out)
}
