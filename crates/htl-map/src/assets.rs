//! What the game data has for a spell, a kit, an icon or an item display: for the asset audit
//! (`examples/asset_probe.rs`), which says what of the content's looks can't be shown.

use std::path::Path;

use anyhow::{Context, Result};
use benilla_formats::{Chain, CreatureCatalog, GameObjectCatalog, ItemDisplayCatalog, SpellCatalog, SpellVisualCatalog};

pub struct Assets {
    chain: Chain,
    spells: SpellCatalog,
    visuals: SpellVisualCatalog,
    items: ItemDisplayCatalog,
    creatures: CreatureCatalog,
    objects: GameObjectCatalog,
}

impl Assets {
    pub fn open(data_dir: &Path) -> Result<Self> {
        let mut chain = Chain::open(data_dir).with_context(|| format!("opening the game data in {}", data_dir.display()))?;
        let spells = benilla_formats::load_spell_catalog(&mut chain).context("reading Spell.dbc")?;
        let visuals = benilla_formats::load_spell_visual_catalog(&mut chain).context("reading SpellVisual.dbc")?;
        let items = benilla_formats::load_item_display_catalog(&mut chain).context("reading ItemDisplayInfo.dbc")?;
        let creatures = benilla_formats::load_creature_catalog(&mut chain).context("reading CreatureDisplayInfo.dbc")?;
        let objects = benilla_formats::load_gameobject_catalog(&mut chain).context("reading GameObjectDisplayInfo.dbc")?;
        Ok(Self { chain, spells, visuals, items, creatures, objects })
    }

    /// A creature display: its model (and whether it's there), and for a character-model NPC its
    /// race, sex and what it wears (each piece's model or texture name), to tell one from another.
    pub fn creature(&self, id: u32) -> String {
        let Some(m) = self.creatures.model(id) else { return format!("creature display {id}: NO ROW") };
        let file = m.model_path.rsplit(['\\', '/']).next().unwrap_or(&m.model_path).to_string();
        let missing = if self.has(&m.model_path) { "" } else { " (MODEL MISSING)" };
        let mut line = format!("creature display {id}: {file}{missing} scale {:.2}", m.scale);
        if let Some(a) = &m.npc_appearance {
            let worn: Vec<String> = a.equipment.iter().filter(|d| **d != 0).map(|d| self.items.get(*d).map_or(format!("{d}?"), |x| x.region_textures.iter().flatten().chain(x.model.iter().flatten()).next().cloned().unwrap_or_default())).collect();
            line.push_str(&format!(" race {} sex {} wears [{}]", a.race, a.sex, worn.join(", ")));
        }
        line
    }

    /// Creature displays whose model's file name holds `part` (cap 60).
    pub fn creatures_like(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        let mut out: Vec<(u32, String)> = self.creatures.display_models().filter_map(|(d, _)| self.creatures.model(d).filter(|m| m.model_path.to_ascii_lowercase().contains(&want)).map(|_| (d, self.creature(d)))).collect();
        out.sort();
        out.into_iter().take(60).map(|(_, l)| l).collect()
    }

    /// An object display (`GameObjectDisplayInfo`): its model, and whether it's there.
    pub fn object(&self, id: u32) -> String {
        match self.objects.model_path(id) {
            None => format!("object display {id}: NO ROW"),
            Some(p) => format!("object display {id}: {p}{}", if self.has(p) { "" } else { " (MODEL MISSING)" }),
        }
    }

    /// Object displays whose model path holds `part` (cap 60).
    pub fn objects_like(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        let mut out: Vec<(u32, String)> = self.objects.iter().filter(|(_, p)| p.to_ascii_lowercase().contains(&want)).map(|(d, p)| (d, format!("object display {d}: {p}"))).collect();
        out.sort();
        out.into_iter().take(60).map(|(_, l)| l).collect()
    }

    /// Whether a file is in the data (a model asked for as .mdx is there as .m2).
    pub fn has(&self, path: &str) -> bool {
        if self.chain.contains(path) {
            return true;
        }
        let low = path.to_ascii_lowercase();
        for (from, to) in [(".mdx", ".m2"), (".mdl", ".m2")] {
            if low.ends_with(from) {
                return self.chain.contains(&format!("{}{to}", &path[..path.len() - from.len()]));
            }
        }
        false
    }

    /// Listed files whose name holds `part` (any case), with whether `has` finds each.
    pub fn find(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        self.chain.list().unwrap_or_default().into_iter().filter(|e| e.name.to_ascii_lowercase().contains(&want)).take(20).map(|e| format!("{} has={}", e.name, self.has(&e.name))).collect()
    }

    /// Whether an icon is there, by its name ("INV_Axe_01") or its path ("Interface\\Icons\\...").
    pub fn icon_exists(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let low = name.to_ascii_lowercase();
        let path = if low.starts_with("interface") { name.to_string() } else { format!("Interface\\Icons\\{name}") };
        let path = if path.to_ascii_lowercase().ends_with(".blp") { path } else { format!("{path}.blp") };
        self.has(&path)
    }

    /// An effect's model, and whether it's there.
    fn effect(&self, id: u32) -> String {
        match self.visuals.effect_path(id) {
            None => format!("effect {id}: NO ROW (error cube)"),
            Some(p) if self.has(p) => p.rsplit(['\\', '/']).next().unwrap_or(p).to_string(),
            Some(p) => format!("{p}: MISSING"),
        }
    }

    /// A kit's models, in words ("-" none; "kit N missing").
    pub fn kit(&self, kit: u32) -> String {
        if kit == 0 {
            return "-".into();
        }
        match self.visuals.kit(kit) {
            None => format!("kit {kit} MISSING"),
            Some(k) => {
                let e: Vec<String> = k.effects().map(|(_, e)| self.effect(e)).collect();
                if e.is_empty() {
                    format!("kit {kit} (anim/sound only)")
                } else {
                    format!("kit {kit} [{}]", e.join(", "))
                }
            }
        }
    }

    /// A spell in words: its 1.12 name, its icon, and each stage of its look.
    pub fn spell(&self, id: u32) -> String {
        let Some(sp) = self.spells.get(id) else { return format!("spell {id}: NOT IN Spell.dbc") };
        let icon = sp.icon.as_deref().map_or("", |i| i.rsplit(['\\', '/']).next().unwrap_or(i));
        let mut out = format!("spell {id} \"{}\" icon {icon} visual {}", sp.name, sp.visual);
        if sp.visual == 0 {
            out.push_str(" (NO LOOK)");
            return out;
        }
        let Some(st) = self.visuals.stages(sp.visual) else {
            out.push_str(" (VISUAL ROW MISSING)");
            return out;
        };
        for (name, k) in [("precast", st.precast), ("cast", st.cast), ("impact", st.impact), ("state", st.state), ("channel", st.channel), ("area_kit", st.area_kit)] {
            if k != 0 {
                out.push_str(&format!("; {name} {}", self.kit(k)));
            }
        }
        if st.missile_model != 0 {
            out.push_str(&format!("; missile {}", self.effect(st.missile_model)));
        }
        if st.area_effect != 0 {
            out.push_str(&format!("; area {} (gate {})", self.effect(st.area_effect), st.area_gate));
        }
        out
    }

    /// An item display in words: its icon (and whether it's there) and its model.
    pub fn item_display(&self, id: u32) -> String {
        let Some(d) = self.items.get(id) else { return format!("display {id}: NOT IN ItemDisplayInfo") };
        let icon = d.icon.clone().unwrap_or_default();
        let ok = self.icon_exists(&icon);
        let model = d.model[0].clone().unwrap_or_default();
        format!("display {id} icon {icon}{} model {model}", if ok { "" } else { " (ICON MISSING)" })
    }

    /// What an NPC creature display wears, by body slot (head, shoulder, shirt, chest, belt,
    /// pants, boots, wrist, gloves, tabard): item display ids, 0 empty. None: not a dressed NPC.
    pub fn npc_gear(&self, creature_display: u32) -> Option<[u32; 10]> {
        self.creatures.model(creature_display)?.npc_appearance.as_ref().map(|a| a.equipment)
    }

    /// Whether a helm display hides the hair under it for both men and women (its
    /// `HelmetGeosetVisData` pair, read only when it has a model): else hair pokes through.
    pub fn helm_hides_hair(&self, display: u32) -> bool {
        self.items.get(display).and_then(|d| d.worn_helm_vis()).is_some_and(|v| v[0] != 0 && v[1] != 0)
    }

    /// A display's look in a few words: its model (shoulders, helms) or its textures (chest, legs).
    pub fn display_look(&self, display: u32) -> String {
        let Some(d) = self.items.get(display) else { return format!("{display}: NO ROW") };
        let mut parts: Vec<String> = d.model.iter().flatten().take(1).map(|m| m.rsplit(['\\', '/']).next().unwrap_or(m).to_string()).collect();
        parts.extend(d.region_textures.iter().flatten().take(2).cloned());
        format!("{display} [{}]", parts.join(", "))
    }

    /// Helm displays built on the same model as `display` that hide the hair (both sexes), the
    /// nearest id first: what to wear instead of an NPC's own helm, which may let hair through.
    pub fn helms_like(&self, display: u32) -> Vec<u32> {
        let Some(model) = self.items.get(display).and_then(|d| d.model[0].clone()) else { return Vec::new() };
        let model = model.to_ascii_lowercase();
        let mut v: Vec<u32> = self
            .items
            .iter()
            .filter(|(id, d)| d.model[0].as_ref().is_some_and(|m| m.to_ascii_lowercase() == model) && self.helm_hides_hair(*id) && d.icon.as_ref().is_some_and(|i| self.icon_exists(i)))
            .map(|(id, _)| id)
            .collect();
        v.sort_by_key(|id| (*id as i64 - display as i64).abs());
        v
    }

    /// Item displays for a body slot whose model or textures hold `part` (cap 40): each with its
    /// look, its icon, and (a helm) whether it hides the hair.
    pub fn displays_wearing(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        let mut out: Vec<(u32, String)> = self
            .items
            .iter()
            .filter(|(_, d)| d.model.iter().flatten().chain(d.region_textures.iter().flatten()).any(|t| t.to_ascii_lowercase().contains(&want)))
            .map(|(id, d)| {
                let icon = d.icon.clone().unwrap_or_default();
                let ok = self.icon_exists(&icon);
                (id, format!("{} icon {}{}{}", self.display_look(id), icon.rsplit(['\\', '/']).next().unwrap_or(&icon), if ok { "" } else { " (ICON MISSING)" }, if d.model[0].as_ref().is_some_and(|m| m.to_ascii_lowercase().contains("helm")) { if self.helm_hides_hair(id) { " hides hair" } else { " HAIR SHOWS" } } else { "" }))
            })
            .collect();
        out.sort();
        out.into_iter().take(40).map(|(_, l)| l).collect()
    }

    /// Displays that look the same as `display` (the same model, or the same body textures) and
    /// have an icon that's there: the one to give an item, so the bags show a picture.
    pub fn twins(&self, display: u32) -> Vec<u32> {
        let Some(d) = self.items.get(display) else { return Vec::new() };
        let key = |x: &benilla_formats::ItemDisplay| -> (Option<String>, Vec<String>) {
            (x.model[0].as_ref().map(|m| m.to_ascii_lowercase()), x.region_textures.iter().flatten().map(|t| t.to_ascii_lowercase()).collect())
        };
        let want = key(d);
        let mut v: Vec<u32> = self
            .items
            .iter()
            .filter(|(id, x)| *id != display && key(x) == want && x.icon.as_ref().is_some_and(|i| self.icon_exists(i)) && (d.model[0].is_none() || !d.model[0].as_ref().unwrap().to_ascii_lowercase().contains("helm") || self.helm_hides_hair(*id)))
            .map(|(id, _)| id)
            .collect();
        v.sort_by_key(|id| (*id as i64 - display as i64).abs());
        v
    }

    /// The display's icon, if it has one that's there.
    pub fn display_icon(&self, id: u32) -> Option<String> {
        self.items.get(id).and_then(|d| d.icon.clone()).filter(|i| self.icon_exists(i))
    }

    /// Every model a spell's visual can lay on the ground at a point (its area effect, drawn by a
    /// GO at a point or a DynamicObject), once each, with up to three spells that carry it.
    pub fn area_models(&self) -> Vec<String> {
        let mut by: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        let mut ids: Vec<(u32, &benilla_formats::SpellDisplay)> = self.spells.iter().collect();
        ids.sort_by_key(|(id, _)| *id);
        for (id, sp) in ids {
            let Some(st) = self.visuals.stages(sp.visual) else { continue };
            if st.area_effect == 0 {
                continue;
            }
            let Some(p) = self.visuals.effect_path(st.area_effect) else { continue };
            if !self.has(p) {
                continue;
            }
            let model = p.rsplit(['\\', '/']).next().unwrap_or(p).to_string();
            let e = by.entry(model).or_default();
            if e.len() < 3 {
                e.push(format!("{id} \"{}\" gate {} missile {}", sp.name, st.area_gate, st.missile_model));
            }
        }
        by.into_iter().map(|(m, v)| format!("{m}: {}", v.join(" | "))).collect()
    }

    /// Spells whose look has a model whose path holds `part` (any case), one line each.
    pub fn search(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        let mut out = Vec::new();
        let mut ids: Vec<u32> = self.spells.iter().map(|(id, _)| id).collect();
        ids.sort_unstable();
        for id in ids {
            let line = self.spell(id);
            if line.to_ascii_lowercase().contains(&want) {
                out.push(line);
            }
            if out.len() >= 40 {
                break;
            }
        }
        out
    }

    /// Item displays whose icon holds `part`, with their models and textures.
    pub fn displays_with_icon(&self, part: &str) -> Vec<String> {
        let want = part.to_ascii_lowercase();
        let mut out: Vec<(u32, String)> = self
            .items
            .iter()
            .filter(|(_, d)| {
                let mut words: Vec<String> = d.region_textures.iter().flatten().cloned().collect();
                words.extend(d.icon.clone());
                words.extend(d.model[0].clone());
                words.iter().any(|w| w.to_ascii_lowercase().contains(&want))
            })
            .map(|(id, d)| {
                let tex: Vec<String> = d.region_textures.iter().flatten().cloned().collect();
                (id, format!("{id} icon {} model {} tex {}", d.icon.clone().unwrap_or_default(), d.model[0].clone().unwrap_or_default(), tex.join(",")))
            })
            .collect();
        out.sort_by_key(|(id, _)| *id);
        out.into_iter().take(600).map(|(_, s)| s).collect()
    }

    /// Every missile model a spell flies, once each, with up to two spells that fly it.
    pub fn missile_models(&self) -> Vec<String> {
        let mut by: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        let mut ids: Vec<(u32, &benilla_formats::SpellDisplay)> = self.spells.iter().collect();
        ids.sort_by_key(|(id, _)| *id);
        for (id, sp) in ids {
            let Some(st) = self.visuals.stages(sp.visual) else { continue };
            if st.missile_model == 0 {
                continue;
            }
            let Some(p) = self.visuals.effect_path(st.missile_model) else { continue };
            if !self.has(p) {
                continue;
            }
            let model = p.rsplit(['\\', '/']).next().unwrap_or(p).to_string();
            let e = by.entry(model).or_default();
            if e.len() < 2 {
                let imp = if st.impact != 0 { self.kit(st.impact) } else { "-".into() };
                e.push(format!("{id} \"{}\" impact {imp}", sp.name));
            }
        }
        by.into_iter().map(|(m, v)| format!("{m}: {}", v.join(" | "))).collect()
    }
}
