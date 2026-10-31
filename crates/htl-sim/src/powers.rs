//! Powers (`powers.toml`): the run-changing picks. Legendaries offered at each Escalation, and
//! the powers a boss leaves behind, generic or for one class, each with what it does (as a
//! talent's or item's effects) and how it shows on the hero (an aura's look, a bigger frame).

use anyhow::{bail, Context, Result};

use crate::content::{ClassDef, Rarity};
use crate::kit::{parse_effects, Kit, TalentEffect, ITEM_STATS};

/// Where a power comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PowerSource {
    /// An Escalation's legendary pick.
    Escalation,
    /// A boss's reward.
    Boss,
}

#[derive(Debug, Clone)]
pub struct PowerDef {
    pub key: String,
    pub name: String,
    pub source: PowerSource,
    /// Only offered to this class.
    pub class: Option<usize>,
    /// Only this boss leaves it (a boss's key), or any boss.
    pub boss: Option<String>,
    /// The spell whose icon it shows on the card and the buff bar.
    pub icon: u32,
    /// A spell whose lasting look the hero wears while they hold it (0: none).
    pub aura: u32,
    /// How much bigger it makes the hero.
    pub grow: f32,
    pub rarity: Rarity,
    pub text: String,
    /// What it does, by class (its spell patches read against that class's spells).
    pub effects: Vec<Vec<TalentEffect>>,
}

impl PowerDef {
    pub fn effects_for(&self, class: usize) -> &[TalentEffect] {
        self.effects.get(class).map_or(&[], |v| v.as_slice())
    }
    /// The spell on the buff bar: its look if it has one, else its icon.
    pub fn buff_spell(&self) -> u32 {
        if self.aura != 0 {
            self.aura
        } else {
            self.icon
        }
    }
}

/// Read `powers.toml` against the classes and their kits.
pub fn parse(text: &str, classes: &[ClassDef], kits: &[Kit]) -> Result<Vec<PowerDef>> {
    let root: toml::Table = toml::from_str(text).context("reading powers.toml")?;
    let mut out: Vec<PowerDef> = Vec::new();
    let Some(list) = root.get("power") else { return Ok(out) };
    for t in list.as_array().context("powers.toml: [[power]]s")? {
        let t = t.as_table().context("a [[power]]")?;
        let s = |k: &str| -> Result<String> { t.get(k).and_then(|v| v.as_str()).map(str::to_string).with_context(|| format!("power needs {k}")) };
        let key = s("key")?;
        let what = format!("power {key}");
        let source = match s("source")?.as_str() {
            "escalation" => PowerSource::Escalation,
            "boss" => PowerSource::Boss,
            other => bail!("{what}: no source called {other}"),
        };
        let int = |k: &str| t.get(k).and_then(|v| v.as_integer()).map(|v| v as u32);
        let class = match t.get("class").and_then(|v| v.as_str()) {
            None => None,
            Some(c) => Some(classes.iter().position(|x| x.key == c).with_context(|| format!("{what}: no class {c}"))?),
        };
        let rarity = match t.get("rarity").and_then(|v| v.as_str()) {
            None if source == PowerSource::Escalation => Rarity::Legendary,
            None => Rarity::Epic,
            Some("epic") => Rarity::Epic,
            Some("legendary") => Rarity::Legendary,
            Some(other) => bail!("{what}: rarity {other} (epic or legendary)"),
        };
        let grow = t.get("grow").and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64))).unwrap_or(0.0) as f32;
        let mut effects = Vec::new();
        for (ci, kit) in kits.iter().enumerate() {
            let mine = class.is_none_or(|c| c == ci);
            effects.push(match (t.get("effect"), mine) {
                (Some(list), true) => parse_effects(kit, list, &what, ITEM_STATS).with_context(|| format!("{what} for {}", classes[ci].key))?,
                _ => Vec::new(),
            });
        }
        if out.iter().any(|p| p.key == key) {
            bail!("{what}: two powers with that key");
        }
        out.push(PowerDef {
            name: s("name")?,
            source,
            class,
            boss: t.get("boss").and_then(|v| v.as_str()).map(str::to_string),
            icon: int("icon").with_context(|| format!("{what}: icon"))?,
            aura: int("aura").unwrap_or(0),
            grow,
            rarity,
            text: s("text")?,
            effects,
            key,
        });
    }
    Ok(out)
}
