//! The Ancient's shop as cards: every defence, Ancient upgrade and weapon enchantment, with its
//! price, for the add-on's shop window. Buying is done at home, by the shopkeeper (or the
//! Ancient): heroes go back between waves to spend.

use super::bases::ForgeAction;
use super::defenses::Build;
use super::*;

/// One card in the shop.
#[derive(Debug, Clone, PartialEq)]
pub struct ShopItem {
    /// What `.bb buy` names: `protector`, `spring`, `defender`, `fortify`, `strengthen`,
    /// `thornbark`, `enchant:fire`...
    pub key: String,
    /// `Defence`, `The Ancient` or `Your weapon`.
    pub group: &'static str,
    pub name: String,
    pub icon_spell: u32,
    pub text: String,
    /// "2 of 4 planted", "Rank 3"...
    pub count: String,
    /// The price, or `None` when it can't be bought now.
    pub cost: Option<u32>,
    /// Why it can't be bought, when it can't.
    pub note: String,
    /// 0 common to 4 legendary, for the card's colour.
    pub rarity: u8,
    /// An icon by name (an item's), instead of `icon_spell`'s.
    pub icon_name: String,
}

/// A place defences stand, for the shop's tabs.
#[derive(Debug, Clone, PartialEq)]
pub struct SiteView {
    /// "the Ancient", "the north bridge".
    pub name: String,
    pub protectors: usize,
    pub springs: usize,
    pub defenders: usize,
}

/// The enchantment on a weapon for each spell school.
const ENCHANTS: [(&str, u32, u32); 7] = [
    ("physical", 1897, 78),
    ("holy", 1900, 635),
    ("fire", 803, 133),
    ("nature", 2564, 403),
    ("frost", 1894, 116),
    ("shadow", 1898, 589),
    ("arcane", 2504, 1449),
];

/// A rank-by-rank upgrade in the shop: training for one hero, or one for everyone's soldiers.
pub(super) struct Upgrade {
    pub key: &'static str,
    pub name: &'static str,
    pub icon: u32,
    pub max: u8,
    /// What a rank adds (a share, or for Muster a count).
    pub per: f32,
    /// The first rank's price, and how much more each next one costs.
    pub cost: u32,
    pub growth: f32,
}

/// The Soldiers tab: what the party's hired defenders and treants become.
pub(super) const SOLDIERS: [Upgrade; 5] = [
    Upgrade { key: "drill", name: "Drill the Defenders", icon: 18499, max: 5, per: 0.15, cost: 100, growth: 1.5 },
    Upgrade { key: "muster", name: "Muster", icon: 6673, max: 3, per: 2.0, cost: 140, growth: 1.6 },
    Upgrade { key: "shieldwall", name: "Shield Wall", icon: 871, max: 3, per: 0.15, cost: 120, growth: 1.6 },
    Upgrade { key: "roots", name: "Deep Roots", icon: 1126, max: 5, per: 0.25, cost: 100, growth: 1.5 },
    // Each rank: a new name and look (Footman, Knight, Stormwind Guard), 20% harder and tougher,
    // and one more enemy held.
    Upgrade { key: "promote", name: "Promote the Defenders", icon: 20217, max: 3, per: 0.20, cost: 260, growth: 1.8 },
];

/// Every spell whose icon the shop shows, so the server can look them up.
pub const SHOP_ICONS: [u32; 22] = [5176, 740, 2565, 465, 22812, 467, 78, 635, 133, 403, 116, 589, 1449, 339, 18499, 6673, 871, 1126, 20217, 3018, 2061, 2457];

/// How many wares each hero is shown at once.
pub(super) const STOCK: usize = 5;
/// The Restock card's icon: a satchel.
const RESTOCK_ICON: &str = "INV_Misc_Bag_10";

pub(super) fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn pct(v: f32) -> String {
    format!("{}%", (v * 100.0).round() as i32)
}

impl Sim {
    /// The weapon enchantments a player is offered: the schools their class's spells use most,
    /// at most three, as (enchantment, school).
    pub(super) fn enchant_choices(&self, id: EntityId) -> Vec<(u32, &'static str)> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let kit = self.content.kit(p.class);
        let mut counts = [0u32; 7];
        for s in &kit.spells {
            if (s.school as usize) < counts.len() {
                counts[s.school as usize] += if s.core { 3 } else { 1 };
            }
        }
        let mut order: Vec<usize> = (0..counts.len()).filter(|&i| counts[i] > 0).collect();
        order.sort_by(|a, b| counts[*b].cmp(&counts[*a]).then(a.cmp(b)));
        order
            .into_iter()
            .take(3)
            .filter_map(|i| ENCHANTS.iter().find(|(tag, _, _)| *tag == crate::kit::SCHOOLS[i]).map(|(tag, e, _)| (*e, *tag)))
            .collect()
    }

    /// Where defences can stand, and what stands there now.
    pub fn shop_sites(&self) -> Vec<SiteView> {
        (0..self.site_count())
            .map(|s| {
                let (protectors, springs, defenders) = self.site_defenses(s);
                SiteView { name: self.site_name(s), protectors, springs, defenders }
            })
            .collect()
    }

    /// The shop's cards for player `id`.
    pub fn shop(&self, id: EntityId) -> Vec<ShopItem> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let d = &self.content.defenses;
        let mut out = Vec::new();
        let built_any = !self.defenses.protectors.is_empty() || !self.defenses.springs.is_empty() || !self.defenses.defenders.is_empty();
        let mult = 1.0 + d.upgrade_bonus * self.defenses.rank as f32;

        let protectors = self.defenses.protectors.len();
        out.push(ShopItem {
            key: "protector".into(),
            group: "Defence",
            name: "Treant Protector".into(),
            icon_spell: 5176,
            text: format!(
                "Plants a treant by the Ancient that hurls bolts at enemies within |cffffffff{}|r yd, every |cffffffff{}|r sec. Enemies passing it stop to tear it down.",
                d.protector_range.round(),
                d.protector_secs
            ),
            count: format!("{protectors} of {} planted", d.protector_costs.len()),
            cost: self.build_cost(Build::Protector),
            note: if protectors >= d.protector_costs.len() { "All planted".into() } else { String::new() },
            rarity: 2,
            icon_name: String::new(),
        });
        let springs = self.defenses.springs.len();
        out.push(ShopItem {
            key: "spring".into(),
            group: "Defence",
            name: "Healing Spring".into(),
            icon_spell: 740,
            text: format!(
                "Heals everyone within |cffffffff{}|r yd of it |cffffffff{}|r of their max health every second, and mends the Ancient a little.",
                d.spring_radius.round(),
                pct(d.spring_heal * mult)
            ),
            count: format!("{springs} of {} raised", d.spring_costs.len()),
            cost: self.build_cost(Build::Spring),
            note: if springs >= d.spring_costs.len() { "All raised".into() } else { String::new() },
            rarity: 2,
            icon_name: String::new(),
        });
        let defenders = self.defenses.defenders.len();
        out.push(ShopItem {
            key: "defender".into(),
            group: "Defence",
            name: "Basin Defender".into(),
            icon_spell: 2565,
            text: "Hires a soldier who fights at the Ancient's feet and holds enemies off it. Cheap, and quick to replace.".into(),
            count: format!("{defenders} of {} hired", self.defender_cap()),
            cost: self.build_cost(Build::Defender),
            note: if defenders >= self.defender_cap() { "All hired (Muster brings more)".into() } else { String::new() },
            rarity: 1,
            icon_name: String::new(),
        });
        let riflemen = self.defenses.riflemen.len();
        out.push(ShopItem {
            key: "rifleman".into(),
            group: "Defence",
            name: "Basin Rifleman".into(),
            icon_spell: 3018,
            text: format!(
                "Hires a dwarf rifleman who shoots enemies within |cffffffff{}|r yd from behind the line. Fragile: keep defenders in front.",
                d.rifleman_range.round()
            ),
            count: format!("{riflemen} of {} hired", d.rifleman_costs.len()),
            cost: self.build_cost(Build::Rifleman),
            note: if riflemen >= d.rifleman_costs.len() { "All hired".into() } else { String::new() },
            rarity: 2,
            icon_name: String::new(),
        });
        let clerics = self.defenses.clerics.len();
        out.push(ShopItem {
            key: "cleric".into(),
            group: "Defence",
            name: "Basin Cleric".into(),
            icon_spell: 2061,
            text: format!(
                "Hires a cleric who heals the most hurt hero or soldier within |cffffffff{}|r yd for |cffffffff{}|r of their health every 3 sec.",
                d.cleric_range.round(),
                pct(d.cleric_heal * mult)
            ),
            count: format!("{clerics} of {} hired", d.cleric_costs.len()),
            cost: self.build_cost(Build::Cleric),
            note: if clerics >= d.cleric_costs.len() { "All hired".into() } else { String::new() },
            rarity: 2,
            icon_name: String::new(),
        });
        out.push(ShopItem {
            key: "fortify".into(),
            group: "Defence",
            name: "Fortify".into(),
            icon_spell: 465,
            text: format!(
                "Every defence deals and heals |cffffffff{}|r more, and they are all mended at once.",
                pct(d.upgrade_bonus)
            ),
            count: format!("Rank {} > {}", self.defenses.rank + 1, self.defenses.rank + 2),
            cost: self.build_cost(Build::Upgrade),
            note: if built_any { String::new() } else { "Build a defence first".into() },
            rarity: 3,
            icon_name: String::new(),
        });
        let (hp, max) = self.entities.get(&self.hub).map_or((0, 1), |h| (h.health, h.max_health.max(1)));
        out.push(ShopItem {
            key: "strengthen".into(),
            group: "The Ancient",
            name: "Strengthen the Ancient".into(),
            icon_spell: 22812,
            text: format!(
                "The Ancient gains |cffffffff15%|r max health and is healed for it at once (|cffffffff{}|r more). It stands at {}%.",
                (max as f32 * 0.15) as u32,
                hp * 100 / max
            ),
            count: format!("Bought {} time{}", self.run.repairs, if self.run.repairs == 1 { "" } else { "s" }),
            cost: Some(self.forge_cost(ForgeAction::Repair)),
            note: String::new(),
            rarity: 2,
            icon_name: String::new(),
        });
        out.push(ShopItem {
            key: "thornbark".into(),
            group: "The Ancient",
            name: "Thornbark".into(),
            icon_spell: 467,
            text: "Thorns cover the Ancient: whatever strikes it is hurt in return, more with every rank.".into(),
            count: format!("Rank {} > {}", self.run.thornbark, self.run.thornbark + 1),
            cost: Some(self.forge_cost(ForgeAction::Thornbark)),
            note: String::new(),
            rarity: 3,
            icon_name: String::new(),
        });
        let blocked = self.wrath_blocked();
        out.push(ShopItem {
            key: "wrath".into(),
            group: "The Ancient",
            name: "Wrath of the Ancient".into(),
            icon_spell: 339,
            text: format!(
                "Every enemy within |cffffffff{}|r yd of the Ancient is rooted for |cffffffff5|r sec and loses a quarter of its health (a boss: a twentieth). Once a wave: for when the wave is at the door.",
                super::bases::WRATH_REACH.round()
            ),
            count: match blocked {
                None => "Ready".into(),
                Some(w) if w.starts_with("Only") => "Waves only".into(),
                Some(_) => "Spent this wave".into(),
            },
            cost: blocked.is_none().then(|| self.forge_cost(ForgeAction::Wrath)),
            note: blocked.unwrap_or("").to_string(),
            rarity: 4,
            icon_name: String::new(),
        });
        // Soldiers: the party's defenders and treants.
        for (k, u) in SOLDIERS.iter().enumerate() {
            let rank = self.defenses.soldiers[k];
            let text = match u.key {
                "drill" => format!(
                    "Every Basin Defender, hired or to come, hits |cffffffff{}|r harder and has that much more health ({} now).",
                    pct(u.per),
                    pct(u.per * rank as f32)
                ),
                "muster" => format!(
                    "|cffffffff{}|r more Basin Defenders can be hired (|cffffffff{}|r in all now).",
                    u.per as u32,
                    self.defender_cap()
                ),
                "shieldwall" => format!("Basin Defenders take |cffffffff{}|r less damage ({} now).", pct(u.per), pct(u.per * rank as f32)),
                "promote" => {
                    let ranks = &self.content.defenses.ranks;
                    let next = ranks.get(rank as usize + 1).map_or("their best".to_string(), |r| r.name.clone());
                    format!(
                        "Every defender becomes a |cffffffff{next}|r: a new look, sword and shield, |cffffffff{}|r harder and tougher, and holding one more enemy at once.",
                        pct(u.per)
                    )
                }
                _ => format!("Treant Protectors' bolts hit |cffffffff{}|r harder ({} now).", pct(u.per), pct(u.per * rank as f32)),
            };
            let top = rank >= u.max;
            out.push(ShopItem {
                key: u.key.into(),
                group: "Your soldiers",
                name: u.name.into(),
                icon_spell: u.icon,
                text,
                count: format!("Rank {rank} of {}", u.max),
                cost: self.soldier_cost(k),
                note: if top { "Top rank".into() } else { String::new() },
                rarity: 2 + (k == 1) as u8 + (k == 4) as u8,
                icon_name: String::new(),
            });
        }
        // Orders: a soldier marches from wherever has the most to the place the tabs pick.
        let soldiers = self.soldier_ids().len();
        out.push(ShopItem {
            key: "march".into(),
            group: "Your soldiers",
            name: "March Orders".into(),
            icon_spell: 2457,
            text: "Sends one soldier from wherever most stand to the place picked on the tabs above: they walk there, so send them before the wave, not into it.".into(),
            count: format!("{soldiers} soldiers"),
            cost: (soldiers > 0).then_some(0),
            note: if soldiers == 0 { "Hire soldiers first".into() } else { String::new() },
            rarity: 1,
            icon_name: String::new(),
        });
        // Wares: rings, necklaces, trinkets and cloaks for you alone, restocked every wave.
        for (k, &entry) in p.stock.iter().enumerate() {
            let Some((w, _, tier)) = crate::items::decode_ware(entry) else { continue };
            let (Some(ware), Some(t)) = (self.content.wares.get(w), self.item_template(entry)) else { continue };
            let stats: Vec<String> = t
                .stats
                .iter()
                .map(|(sid, v)| format!("+{v} {}", stat_name(*sid)))
                .chain((t.armor > 0).then(|| format!("+{} Armor", t.armor)))
                .collect();
            let class_note = ware.class.map_or(String::new(), |c| format!(" {} only.", capitalize(&self.content.classes[c].key)));
            // What it would go beside or replace: what's worn in its slots now.
            let worn: Vec<String> = ware
                .slot
                .slots()
                .iter()
                .map(|&s| match p.equip[s as usize].and_then(|w| self.item_template(w.entry)) {
                    Some(w) => format!("|c{}{}|r", gear_color(w.quality), w.name),
                    None => "nothing".to_string(),
                })
                .collect();
            let now = match worn.as_slice() {
                [one] => format!("Your {}: {one}", ware.slot.noun().to_lowercase()),
                [a, b] => format!("Your {}s: {a} and {b}", ware.slot.noun().to_lowercase()),
                _ => String::new(),
            };
            out.push(ShopItem {
                key: format!("item:{k}"),
                group: "Items",
                name: ware.name.clone(),
                icon_spell: 0,
                icon_name: ware.icon.clone(),
                text: format!("{}|n|cff9d9d9d{}.{class_note}|r|n|cffffd060{now}.|r", ware.text, stats.join(", ")),
                count: format!("{}  ·  Tier {}", ware.slot.noun(), tier + 1),
                cost: Some(self.ware_price(ware.quality, tier)),
                note: String::new(),
                rarity: ware.quality.saturating_sub(1),
            });
        }
        // Reforging: a worn ware from an earlier tier brought up to this one, effect and all.
        let now_tier = self.drop_tier();
        let mut reforges = Vec::new();
        for (s, it) in p.equip.iter().enumerate() {
            let Some(it) = it else { continue };
            let Some((w, _, tier)) = crate::items::decode_ware(it.entry) else { continue };
            if tier >= now_tier {
                continue;
            }
            let Some(ware) = self.content.wares.get(w) else { continue };
            reforges.push(ShopItem {
                key: format!("reforge:{s}"),
                group: "Reforge",
                name: format!("Reforge {}", ware.name),
                icon_spell: 0,
                icon_name: ware.icon.clone(),
                text: format!(
                    "{} ({}) from Tier {} to Tier {}: its stats grow to match the gear dropping now; what it does stays yours.",
                    ware.name,
                    ware.slot.noun(),
                    tier + 1,
                    now_tier + 1
                ),
                count: format!("Worn  ·  Tier {}", tier + 1),
                cost: Some(self.reforge_price(ware.quality, now_tier)),
                note: String::new(),
                rarity: ware.quality.saturating_sub(1),
            });
        }
        // The four finest first (the window has room for four).
        reforges.sort_by_key(|r| std::cmp::Reverse(r.rarity));
        reforges.truncate(4);
        out.extend(reforges);
        out.push(ShopItem {
            key: "restock".into(),
            group: "Items",
            name: "Restock".into(),
            icon_spell: 0,
            icon_name: RESTOCK_ICON.into(),
            text: format!("Five fresh wares in place of these. The stock turns over by itself after every wave."),
            count: format!("{} on offer", p.stock.len()),
            cost: Some(self.restock_price()),
            note: String::new(),
            rarity: 0,
        });
        let slot = self.content.classes[p.class].weapon_slot as usize;
        let weapon = p.equip[slot].as_ref();
        for (e, tag) in self.enchant_choices(id) {
            let icon = ENCHANTS.iter().find(|x| x.1 == e).map_or(133, |x| x.2);
            let has = weapon.is_some_and(|w| w.enchant == e);
            out.push(ShopItem {
                key: format!("enchant:{tag}"),
                group: "Your weapon",
                name: format!("{} Enchantment", capitalize(tag)),
                icon_spell: icon,
                text: format!(
                    "Your weapon glows: your {tag} spells deal |cffffffff10%|r more. One enchantment at a time: a new one replaces the old."
                ),
                count: if has { "On your weapon".into() } else { "Enchantment".into() },
                cost: Some(self.forge_cost(ForgeAction::Enchant(e))),
                note: if weapon.is_none() {
                    "No weapon".into()
                } else if has {
                    "Already yours".into()
                } else {
                    String::new()
                },
                rarity: 1,
                icon_name: String::new(),
            });
        }
        for it in &mut out {
            if !it.note.is_empty() {
                it.cost = None;
            }
        }
        out
    }

    /// Buy shop card `key` for player `id`, at home by the shopkeeper. `Ok` says what happened.
    /// A defence goes where `@site` says (`protector@1`: at the first post), or by the Ancient.
    pub fn buy(&mut self, id: EntityId, key: &str) -> Result<String, String> {
        if !self.at_shop(id) {
            return Err(format!("Shopping is done at home: visit {}.", self.vendor_name()));
        }
        let (key, site) = match key.split_once('@') {
            Some((k, s)) => (k, s.parse::<usize>().map_err(|_| "No such place.".to_string())?),
            None => (key, 0),
        };
        if site >= self.site_count() {
            return Err("No such place.".into());
        }
        let s = site as u8;
        let action = match key {
            "protector" => ForgeAction::Build(Build::Protector, s),
            "spring" => ForgeAction::Build(Build::Spring, s),
            "defender" => ForgeAction::Build(Build::Defender, s),
            "rifleman" => ForgeAction::Build(Build::Rifleman, s),
            "cleric" => ForgeAction::Build(Build::Cleric, s),
            "march" => return self.march(id, site),
            "fortify" => ForgeAction::Build(Build::Upgrade, 0),
            "strengthen" => ForgeAction::Repair,
            "thornbark" => ForgeAction::Thornbark,
            "wrath" => ForgeAction::Wrath,
            _ if SOLDIERS.iter().any(|u| u.key == key) => ForgeAction::Soldier(SOLDIERS.iter().position(|u| u.key == key).unwrap() as u8),
            "restock" => {
                let cost = self.restock_price();
                self.pay(id, cost)?;
                self.restock(id);
                return Ok("Restock".into());
            }
            _ if key.starts_with("reforge:") => {
                let s: u8 = key["reforge:".len()..].parse().map_err(|_| "Nothing like that to reforge.".to_string())?;
                return self.reforge(id, s);
            }
            _ if key.starts_with("item:") => {
                let k: usize = key["item:".len()..].parse().map_err(|_| "Nothing like that for sale.".to_string())?;
                return self.buy_ware(id, k);
            }
            _ => {
                let tag = key.strip_prefix("enchant:").ok_or_else(|| "Nothing like that for sale.".to_string())?;
                let (e, _) = self
                    .enchant_choices(id)
                    .into_iter()
                    .find(|(_, t)| *t == tag)
                    .ok_or_else(|| "That enchantment isn't offered to you.".to_string())?;
                ForgeAction::Enchant(e)
            }
        };
        if matches!(action, ForgeAction::Build(Build::Upgrade, _)) && self.build_cost(Build::Upgrade).is_none() {
            return Err("Build a defence first.".into());
        }
        let mut name = self.shop(id).into_iter().find(|i| i.key == key).map(|i| i.name).unwrap_or_default();
        self.buy_forge(id, action)?;
        if site > 0 && matches!(key, "protector" | "spring" | "defender") {
            name = format!("{name} at {}", self.site_name(site));
        }
        Ok(name)
    }
}

impl Sim {
    /// A ware's price: by quality, and more at each gear tier.
    pub(super) fn ware_price(&self, quality: u8, tier: u8) -> u32 {
        let base = match quality {
            0..=2 => 70.0,
            3 => 130.0,
            4 => 220.0,
            _ => 360.0,
        };
        (base * (1.0 + 0.3 * tier as f32)).round() as u32
    }

    /// Bringing a worn ware up to `tier`: a little over half the price of a new one.
    pub(super) fn reforge_price(&self, quality: u8, tier: u8) -> u32 {
        (self.ware_price(quality, tier) as f32 * 0.6).round() as u32
    }

    /// Reforge the ware worn in slot `s` up to the tier dropping now.
    fn reforge(&mut self, id: EntityId, s: u8) -> Result<String, String> {
        let now_tier = self.drop_tier();
        let it = self
            .entities
            .get(&id)
            .and_then(|e| e.player())
            .and_then(|p| p.equip.get(s as usize).copied().flatten())
            .ok_or("Nothing worn there.")?;
        let (w, class, tier) = crate::items::decode_ware(it.entry).ok_or("Only the shop's items can be reforged.")?;
        if tier >= now_tier {
            return Err("Already as strong as the gear dropping now.".into());
        }
        let (quality, name) = {
            let ware = self.content.wares.get(w).ok_or("Nothing like that to reforge.")?;
            (ware.quality, ware.name.clone())
        };
        self.pay(id, self.reforge_price(quality, now_tier))?;
        let entry = crate::items::ware_entry(w, class, now_tier);
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.equip[s as usize] = None;
        }
        // A new item (the client keeps an item's entry for good).
        let fresh = gear::ItemInst { id: self.alloc_item(), entry, enchant: it.enchant };
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.equip[s as usize] = Some(fresh);
        }
        self.events.push(Event::ItemReceived { player: id, entry, slot: s });
        self.events.push(Event::Inventory { player: id });
        self.events.push(Event::Visible { player: id });
        self.recompute(id);
        self.tell(id, format!("{name} is reforged to Tier {}.", now_tier + 1));
        Ok(format!("Reforged {name}"))
    }

    pub(super) fn restock_price(&self) -> u32 {
        15 + 5 * self.run.wave
    }

    fn pay(&mut self, id: EntityId, cost: u32) -> Result<(), String> {
        if self.take_gold(id, cost) {
            return Ok(());
        }
        let gold = self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.gold);
        Err(format!("That costs {cost} gold; you have {gold}."))
    }

    /// Buy the `k`th ware on offer: it goes on at once if a slot it fits is free, else into the
    /// bags.
    fn buy_ware(&mut self, id: EntityId, k: usize) -> Result<String, String> {
        let entry = self.entities.get(&id).and_then(|e| e.player()).and_then(|p| p.stock.get(k).copied()).ok_or("That's sold.")?;
        let (w, _, tier) = crate::items::decode_ware(entry).ok_or("Nothing like that for sale.")?;
        let (quality, name, slot) = {
            let ware = self.content.wares.get(w).ok_or("Nothing like that for sale.")?;
            (ware.quality, ware.name.clone(), ware.slot)
        };
        let room = self.free_pair_slot(id, slot.slots()[0]).is_some()
            || self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.pack.iter().any(|s| s.is_none()));
        if !room {
            return Err("Your bags are full: make room first.".into());
        }
        self.pay(id, self.ware_price(quality, tier))?;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.stock.remove(k);
        }
        self.take_ware(id, entry)?;
        Ok(name)
    }

    /// A fresh stock of wares for hero `id`: four, none they already own, rarer ones as the run
    /// goes on, at the wave's gear tier.
    pub(super) fn restock(&mut self, id: EntityId) {
        let wave = self.run.wave.max(1);
        let tier = self.drop_tier();
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let class = p.class;
        let owned: Vec<usize> = p
            .equip
            .iter()
            .chain(p.pack.iter())
            .flatten()
            .filter_map(|it| crate::items::decode_ware(it.entry).map(|(w, _, _)| w))
            .collect();
        let mut pool: Vec<(usize, f32)> = self
            .content
            .wares
            .iter()
            .enumerate()
            .filter(|(i, w)| w.class.is_none_or(|c| c == class) && w.from_wave <= wave && !owned.contains(i))
            .map(|(i, w)| {
                let weight = match w.quality {
                    0..=2 => 10.0,
                    3 => 7.0,
                    4 => 3.0 + 0.3 * wave as f32,
                    _ => 1.0 + 0.1 * wave as f32,
                } * if w.class.is_some() { 1.5 } else { 1.0 };
                (i, weight)
            })
            .collect();
        let mut stock = Vec::new();
        while stock.len() < STOCK && !pool.is_empty() {
            let total: f32 = pool.iter().map(|(_, w)| w).sum();
            let mut roll = self.rng.gen::<f32>() * total;
            let mut pick = pool.len() - 1;
            for (k, (_, w)) in pool.iter().enumerate() {
                if roll < *w {
                    pick = k;
                    break;
                }
                roll -= w;
            }
            let (w, _) = pool.remove(pick);
            stock.push(crate::items::ware_entry(w, class, tier));
        }
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.stock = stock;
        }
    }
}

/// A stat's name as the tooltip says it.
fn stat_name(id: u32) -> &'static str {
    use crate::items::stat;
    match id {
        stat::AGILITY => "Agility",
        stat::STRENGTH => "Strength",
        stat::INTELLECT => "Intellect",
        stat::SPIRIT => "Spirit",
        _ => "Stamina",
    }
}

/// An item quality's colour, for names in text.
fn gear_color(quality: u8) -> &'static str {
    match quality {
        0 => "ff9d9d9d",
        1 => "ffffffff",
        2 => "ff1eff00",
        3 => "ff0070dd",
        4 => "ffa335ee",
        _ => "ffff8000",
    }
}
