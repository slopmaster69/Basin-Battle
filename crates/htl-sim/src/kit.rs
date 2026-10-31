//! Class kits: each class's spells (with the upgrades a card can bring them), its three talent
//! trees, and the numbers both are made of. Loaded from `content/classes/<class>.toml`.
//!
//! A spell is a kind (bolt, nova, zone...) and a set of numbers ([`Nums`]). An upgrade (a "perk",
//! offered as a card) and a talent change those numbers with a [`Patch`]: `set` replaces a
//! value, `add` adds to it, `pct` grows it by a share (0.2 = 20% more), and `more` adds only to
//! a value that is already there. Talents scale their patch by their rank. Effects that numbers
//! can't say (Ignite, Clearcasting, Flurry...) are named procs the game knows.

use std::collections::HashMap;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

use crate::content::Rarity;

macro_rules! nums {
    ($( $(#[$doc:meta])* $name:ident ),* $(,)?) => {
        /// A spell's numbers. Damage and healing are multiples of the caster's "hit" (weapon
        /// damage per second times two) or shares of max health; times are seconds; ranges and
        /// radii yards; chances 0-1.
        #[derive(Debug, Clone, Default, PartialEq)]
        pub struct Nums {
            $( $(#[$doc])* pub $name: f32, )*
        }

        impl Nums {
            pub const NAMES: &'static [&'static str] = &[$(stringify!($name)),*];

            pub fn get_mut(&mut self, name: &str) -> Option<&mut f32> {
                match name {
                    $( stringify!($name) => Some(&mut self.$name), )*
                    _ => None,
                }
            }

            pub fn get(&self, name: &str) -> Option<f32> {
                match name {
                    $( stringify!($name) => Some(self.$name), )*
                    _ => None,
                }
            }
        }
    };
}

nums! {
    /// Seconds between casts.
    cooldown,
    /// Cast time: the caster stands still this long before it goes off (moving breaks it).
    cast,
    /// Channel: its missiles fly over this long while the caster stands still; moving stops
    /// the rest.
    channel,
    /// Rage it costs (warriors).
    cost,
    range,
    radius,
    /// A cone's width in degrees.
    angle,
    /// How long a zone, buff or trap lasts.
    duration,
    /// A zone's damage (or healing share) each second, per enemy (ally).
    tick,
    /// The hit's damage.
    damage,
    /// Extra damage and healing: 0.2 = 20% more.
    power,
    /// Enemies a bolt strikes besides the first.
    targets,
    /// Missiles per target.
    projectiles,
    /// Jumps of a chain.
    chain,
    falloff,
    /// Missile speed, yards a second (0: it lands at once).
    speed,
    /// Extra crit chance, and extra crit damage (crits do 150% plus this).
    crit,
    crit_mult,
    crit_vs_frozen,
    /// Damage over time it leaves: the total, over `dot_secs`; `dot_stack` 1 lets it stack to 3.
    dot,
    dot_secs,
    dot_stack,
    slow,
    slow_secs,
    /// Frozen: can't move or attack. Stunned: the same. Rooted: can't move. Feared: runs away.
    freeze_chance,
    freeze_secs,
    root_secs,
    /// The chance to root (0: always, when `root_secs` is set).
    root_chance,
    stun_chance,
    stun_secs,
    fear_secs,
    /// Stacks of Fire Vulnerability, Winter's Chill, Shadow Vulnerability or Sunder Armor each
    /// hit applies (fractions are chances).
    vuln_fire,
    vuln_frost,
    vuln_shadow,
    vuln_sunder,
    /// A share of the hit splashes to enemies within `splash_radius` of the target.
    splash,
    splash_radius,
    /// A share of the damage heals you (`leech`) or the most hurt ally near you (`mend`).
    leech,
    mend,
    /// Heals: a share of max health at once, over time, or as an absorb shield.
    heal,
    hot,
    hot_secs,
    absorb,
    /// Shields: a share of what they absorb hits back; a blast when they break.
    reflect,
    burst,
    /// More damage against frozen, slowed, burning, bleeding, shadow-afflicted, poisoned,
    /// marked and stunned enemies, and per Sunder Armor stack.
    vs_frozen,
    vs_chilled,
    vs_burning,
    vs_bleeding,
    vs_afflicted,
    vs_stung,
    vs_marked,
    vs_stunned,
    vs_sunder,
    /// More damage below `execute_at` health; only castable on targets below `only_below`.
    execute,
    execute_at,
    only_below,
    /// Only castable for a few seconds after you're hit (Revenge).
    after_hit,
    /// Hunter's Mark: the target takes this much more from you and your pet.
    mark,
    taunt_secs,
    /// Enemies hit deal this much less damage for `demoralize_secs`.
    demoralize,
    demoralize_secs,
    /// Rage gained on cast, refunded on a crit, gained on a kill; `reset_on_kill` 1 finishes the
    /// cooldown when it kills.
    rage,
    refund,
    rage_on_kill,
    reset_on_kill,
    /// Each enemy hit takes this much off your other spells' cooldowns, up to `cdr_cap`.
    cdr_per_hit,
    cdr_cap,
    /// Chance to cast it again for free.
    echo,
    /// Spreads the target's burns to this many enemies nearby.
    spread,
    /// When an afflicted enemy dies, its damage over time jumps to this many nearby.
    spread_on_death,
    /// Sets off the target's poison: this share of what's left of it lands at once.
    detonate,
    /// The mark jumps to the nearest enemy when its target dies.
    jump_on_death,
    /// Heals you this share of max health per enemy hit, or at once.
    heal_per_hit,
    heal_self,
    /// An absorb shield on you, a share of max health per enemy hit (Frost Warding).
    ward,
    /// Buffs: strength and second number.
    buff_value,
    buff_value2,
    /// A burst of run speed after casting.
    speed_buff,
    speed_secs,
    /// Strikes a charge buff lasts (Sweeping Strikes).
    charges,
    /// Traps: how close an enemy must come.
    trigger_radius,
    /// 1: casting it gives you Clearcasting. 1: casting it finishes your other spells' cooldowns.
    clearcast,
    reset_others,
    /// Blink and Charge: the shortest range.
    min_range,
}

/// What a spell does when cast.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SpellKind {
    /// Missiles (or an instant strike, at `speed` 0) at the target.
    Bolt,
    /// A burst around the caster.
    Nova,
    /// A burst in front of the caster.
    Cone,
    /// A lasting area at the target (or the caster).
    Zone,
    /// Heals the most hurt ally in range (or the caster).
    Heal,
    /// Heals over time.
    Hot,
    /// An absorb shield.
    Shield,
    /// A timed effect on the caster (or an ally).
    Buff,
    /// Placed at the caster's feet; goes off when an enemy comes close.
    Trap,
    /// A jump forward.
    Blink,
    /// A rush to the target.
    Charge,
    /// The hunter's pet strikes.
    Pet,
    /// Switched on and off (Shadowform).
    Toggle,
    /// Something only the game knows (Cold Snap, Lightwell, Feign Death...).
    Special,
}

pub const SCHOOLS: [&str; 7] = ["physical", "holy", "fire", "nature", "frost", "shadow", "arcane"];

pub fn school_index(name: &str) -> Option<u8> {
    SCHOOLS.iter().position(|s| *s == name).map(|i| i as u8)
}

/// Changes to a spell's numbers.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Patch {
    pub set: Vec<(String, f32)>,
    pub add: Vec<(String, f32)>,
    pub pct: Vec<(String, f32)>,
    pub more: Vec<(String, f32)>,
}

impl Patch {
    pub fn is_empty(&self) -> bool {
        self.set.is_empty() && self.add.is_empty() && self.pct.is_empty() && self.more.is_empty()
    }

    /// Apply to `n`, `times` over (a talent's rank).
    pub fn apply(&self, n: &mut Nums, times: f32) {
        for (k, v) in &self.set {
            if let Some(f) = n.get_mut(k) {
                *f = *v;
            }
        }
        for (k, v) in &self.add {
            if let Some(f) = n.get_mut(k) {
                *f += v * times;
            }
        }
        for (k, v) in &self.more {
            if let Some(f) = n.get_mut(k) {
                if *f != 0.0 {
                    *f += v * times;
                }
            }
        }
        for (k, v) in &self.pct {
            if let Some(f) = n.get_mut(k) {
                *f *= 1.0 + v * times;
            }
        }
    }

    fn parse(t: &toml::Table, what: &str) -> Result<Self> {
        let mut p = Patch::default();
        for (op, list) in [("set", &mut p.set), ("add", &mut p.add), ("pct", &mut p.pct), ("more", &mut p.more)] {
            let Some(v) = t.get(op) else { continue };
            let tab = v.as_table().with_context(|| format!("{what}: {op} must be a table"))?;
            for (k, v) in tab {
                if !Nums::NAMES.contains(&k.as_str()) {
                    bail!("{what}: {op}.{k} isn't a spell number");
                }
                list.push((k.clone(), num(v).with_context(|| format!("{what}: {op}.{k}"))?));
            }
        }
        Ok(p)
    }
}

fn num(v: &toml::Value) -> Result<f32> {
    match v {
        toml::Value::Float(f) => Ok(*f as f32),
        toml::Value::Integer(i) => Ok(*i as f32),
        toml::Value::Boolean(b) => Ok(if *b { 1.0 } else { 0.0 }),
        _ => bail!("a number expected"),
    }
}

/// An upgrade a card brings to a spell you hold.
#[derive(Debug, Clone)]
pub struct PerkDef {
    pub key: String,
    pub name: String,
    pub rarity: Rarity,
    /// What it does, as the card says it.
    pub text: String,
    pub patch: Patch,
}

#[derive(Debug, Clone)]
pub struct SpellDef {
    pub key: String,
    pub name: String,
    /// The talent tree it belongs with (0-2).
    pub tree: usize,
    pub rarity: Rarity,
    /// Can be a run's first spell.
    pub core: bool,
    /// Learned from a talent, never offered as a card.
    pub talent: bool,
    pub kind: SpellKind,
    pub school: u8,
    /// The icon, and the spell whose effect plays (`icon` unless set).
    pub icon: u32,
    pub visual: u32,
    /// For buffs and toggles: which effect; for specials: what it does.
    pub effect: String,
    /// Bolts, zones, heals and buffs: `enemy`, `self`, `ally` or `cluster`.
    pub target: String,
    /// A line added to the generated description (synergies, rules).
    pub note: String,
    pub nums: Nums,
    pub perks: Vec<PerkDef>,
}

impl SpellDef {
    pub fn visual(&self) -> u32 {
        if self.visual != 0 {
            self.visual
        } else {
            self.icon
        }
    }

    pub fn is_heal(&self) -> bool {
        matches!(self.kind, SpellKind::Heal | SpellKind::Hot | SpellKind::Shield)
    }
}

/// Which spells a talent changes.
#[derive(Debug, Clone, PartialEq)]
pub enum Filter {
    All,
    Keys(Vec<String>),
    School(u8),
    Tree(usize),
    Kind(SpellKind),
    Any(Vec<Filter>),
}

impl Filter {
    pub fn matches(&self, s: &SpellDef) -> bool {
        match self {
            Filter::All => true,
            Filter::Keys(k) => k.iter().any(|k| *k == s.key),
            Filter::School(x) => s.school == *x,
            Filter::Tree(t) => s.tree == *t,
            Filter::Kind(k) => s.kind == *k,
            Filter::Any(list) => list.iter().any(|f| f.matches(s)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct TalentEffect {
    pub filter: Option<Filter>,
    pub patch: Patch,
    /// Player numbers it adds to, per rank (see `players::Stats`).
    pub stats: Vec<(String, f32)>,
    /// A team aura: you and allies within 30 yards gain this, per rank.
    pub team: Vec<(String, f32)>,
    /// A named effect the game knows, and its strength per rank.
    pub proc_: Option<(String, f32)>,
}

pub const STATS: &[&str] = &[
    "crit", "haste", "damage", "health", "regen", "speed", "taken", "heal", "auto", "cdr", "absorb", "elite_damage", "block_heal",
    "rage_auto", "rage_hit", "rage_regen", "rage_keep", "reach", "pet_damage", "pet_health", "pet_crit", "pet_taken",
    "pet_regen", "respawn",
];
/// What an item's `stats` may add to, besides the talents' `STATS`: the cards' passives, and the
/// items' own.
pub const ITEM_STATS: &[&str] = &[
    "crit", "haste", "damage", "health", "regen", "speed", "taken", "heal", "auto", "cdr", "absorb", "elite_damage", "block_heal",
    "rage_auto", "rage_hit", "rage_regen", "rage_keep", "reach", "pet_damage", "pet_health", "pet_crit", "pet_taken",
    "pet_regen", "respawn", "leech", "execute", "per_wave", "thorns", "lightning", "bloodthirst", "raise_dead", "soul_harvest",
    "cleave", "windfury", "echo", "split", "radius", "shield", "undying", "divine", "gold_find", "hub_leech", "defence_aura",
    "rise",
];
pub const TEAM: &[&str] = &["damage", "health", "haste", "regen", "taken"];
pub const PROCS: &[&str] = &[
    "clearcasting",
    "ignite",
    "deep_wounds",
    "master_of_elements",
    "enrage",
    "flurry",
    "blood_craze",
    "spirit_tap",
    "quick_shots",
    "extra_attack",
    "block",
    "martyrdom",
    "inspiration",
    "vampiric_embrace",
    "spirit_of_redemption",
    "frenzy",
];

#[derive(Debug, Clone)]
pub struct TalentDef {
    pub key: String,
    pub name: String,
    pub tree: usize,
    /// 1-5: tier n opens once 3*(n-1) points are spent in the tree.
    pub tier: u8,
    /// 1-4 across the tree.
    pub col: u8,
    pub ranks: u8,
    pub icon: u32,
    /// With `{0%}`, `{1}`, `{0s}` standing for `values[i]` times the rank.
    pub text: String,
    pub values: Vec<f32>,
    pub effects: Vec<TalentEffect>,
    /// The spell it teaches.
    pub grants: Option<usize>,
    /// Spells that must all be held for it to be offered (a combination talent).
    pub requires: Vec<usize>,
    /// A warrior's weapon path it commits to (`greatsword`, `shield`, `dual`): only one per run.
    pub path: Option<String>,
}

impl TalentDef {
    /// Whether it only reshapes spells (no stats, auras or procs of its own): then it's worth
    /// offering only to a hero holding a spell it reshapes.
    pub fn only_patches_spells(&self) -> bool {
        !self.effects.is_empty() && self.effects.iter().all(|e| e.filter.is_some() && e.stats.is_empty() && e.team.is_empty() && e.proc_.is_none())
    }
}

impl TalentDef {
    /// The talent's text at `rank` (1 or more).
    pub fn text_at(&self, rank: u8) -> String {
        let mut out = String::new();
        let mut rest = self.text.as_str();
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else { break };
            let spec = &rest[open + 1..open + close];
            let digits: String = spec.chars().take_while(|c| c.is_ascii_digit()).collect();
            let unit = &spec[digits.len()..];
            let v = digits.parse::<usize>().ok().and_then(|i| self.values.get(i)).copied().unwrap_or(0.0) * rank as f32;
            out.push_str(&match unit {
                "%" => format!("{}%", trim(v * 100.0)),
                "s" => format!("{} sec", trim(v)),
                "yd" => format!("{} yd", trim(v)),
                _ => trim(v),
            });
            rest = &rest[open + close + 1..];
        }
        out.push_str(rest);
        out
    }
}

pub fn trim(v: f32) -> String {
    let r = (v * 10.0).round() / 10.0;
    if (r - r.round()).abs() < 0.01 {
        format!("{}", r.round() as i64)
    } else {
        format!("{r:.1}")
    }
}

#[derive(Debug, Clone)]
pub struct TreeDef {
    pub key: String,
    pub name: String,
    /// One line on what the tree is about.
    pub text: String,
    pub icon: u32,
    /// The talent-frame background (`Interface\TalentFrame\<name>-TopLeft`...).
    pub background: String,
}

/// One class's spells and talents.
#[derive(Debug, Clone)]
pub struct Kit {
    pub class: String,
    pub trees: Vec<TreeDef>,
    pub spells: Vec<SpellDef>,
    pub talents: Vec<TalentDef>,
}

/// Points spent in a tree before tier `tier` opens.
pub fn tier_points(tier: u8) -> u32 {
    3 * (tier.max(1) as u32 - 1)
}

impl Kit {
    pub fn spell_index(&self, key: &str) -> Option<usize> {
        self.spells.iter().position(|s| s.key == key)
    }

    pub fn tree_index(&self, key: &str) -> Option<usize> {
        self.trees.iter().position(|t| t.key == key)
    }

    pub fn parse(class: &str, text: &str) -> Result<Kit> {
        let root: toml::Table = toml::from_str(text).with_context(|| format!("reading {class}.toml"))?;
        let mut kit = Kit { class: class.to_string(), trees: Vec::new(), spells: Vec::new(), talents: Vec::new() };
        for t in arr(&root, "tree")? {
            kit.trees.push(TreeDef {
                key: s(t, "key")?,
                name: s(t, "name")?,
                text: s_or(t, "text"),
                icon: u(t, "icon")?,
                background: s_or(t, "background"),
            });
        }
        if kit.trees.len() != 3 {
            bail!("{class}: three talent trees expected");
        }
        for t in arr(&root, "spell")? {
            let key = s(t, "key")?;
            let what = format!("{class} spell {key}");
            let tree = kit.tree_index(&s(t, "tree")?).with_context(|| format!("{what}: no such tree"))?;
            let kind: SpellKind = t
                .get("kind")
                .context("kind")?
                .clone()
                .try_into()
                .with_context(|| format!("{what}: bad kind"))?;
            let school = school_index(&s(t, "school")?).with_context(|| format!("{what}: bad school"))?;
            let rarity: Rarity = t
                .get("rarity")
                .map(|v| v.clone().try_into())
                .transpose()
                .with_context(|| format!("{what}: bad rarity"))?
                .unwrap_or(Rarity::Common);
            let mut nums = Nums::default();
            const IDENT: &[&str] =
                &["key", "name", "tree", "rarity", "core", "talent", "kind", "school", "icon", "visual", "effect", "target", "note", "perk"];
            for (k, v) in t {
                if IDENT.contains(&k.as_str()) {
                    continue;
                }
                let f = nums.get_mut(k).with_context(|| format!("{what}: {k} isn't a spell number"))?;
                *f = num(v).with_context(|| format!("{what}: {k}"))?;
            }
            let mut perks = Vec::new();
            if let Some(list) = t.get("perk") {
                let list = list.as_array().with_context(|| format!("{what}: perk must be [[spell.perk]]"))?;
                for p in list {
                    let p = p.as_table().context("perk")?;
                    let pk = s(p, "key")?;
                    let pw = format!("{what} perk {pk}");
                    for k in p.keys() {
                        if !matches!(k.as_str(), "key" | "name" | "rarity" | "text" | "set" | "add" | "pct" | "more") {
                            bail!("{pw}: unknown field {k}");
                        }
                    }
                    perks.push(PerkDef {
                        key: pk,
                        name: s(p, "name")?,
                        rarity: p
                            .get("rarity")
                            .map(|v| v.clone().try_into())
                            .transpose()
                            .with_context(|| format!("{pw}: bad rarity"))?
                            .unwrap_or(Rarity::Common),
                        text: s(p, "text")?,
                        patch: Patch::parse(p, &pw)?,
                    });
                }
            }
            kit.spells.push(SpellDef {
                key,
                name: s(t, "name")?,
                tree,
                rarity,
                core: b(t, "core"),
                talent: b(t, "talent"),
                kind,
                school,
                icon: u(t, "icon")?,
                visual: t.get("visual").map(|v| num(v).map(|f| f as u32)).transpose()?.unwrap_or(0),
                effect: s_or(t, "effect"),
                target: s_or(t, "target"),
                note: s_or(t, "note"),
                nums,
                perks,
            });
        }
        let mut grants: Vec<(usize, String)> = Vec::new();
        for t in arr(&root, "talent")? {
            let key = s(t, "key")?;
            let what = format!("{class} talent {key}");
            let tree = kit.tree_index(&s(t, "tree")?).with_context(|| format!("{what}: no such tree"))?;
            let effects = match t.get("effect") {
                Some(list) => parse_effects(&kit, list, &what, STATS)?,
                None => Vec::new(),
            };
            let values = match t.get("values") {
                None => Vec::new(),
                Some(v) => v
                    .as_array()
                    .context("values must be a list")?
                    .iter()
                    .map(num)
                    .collect::<Result<Vec<f32>>>()?,
            };
            let tier = u(t, "tier")? as u8;
            let col = u(t, "col")? as u8;
            if !(1..=5).contains(&tier) || !(1..=4).contains(&col) {
                bail!("{what}: tier 1-5 and col 1-4");
            }
            if let Some(g) = t.get("grants").and_then(|v| v.as_str()) {
                grants.push((kit.talents.len(), g.to_string()));
            }
            let mut requires = Vec::new();
            if let Some(list) = t.get("requires") {
                for r in list.as_array().with_context(|| format!("{what}: requires is a list of spells"))? {
                    let r = r.as_str().with_context(|| format!("{what}: requires names spells"))?;
                    requires.push(kit.spell_index(r).with_context(|| format!("{what}: no spell {r}"))?);
                }
            }
            let path = t.get("path").and_then(|v| v.as_str()).map(str::to_string);
            if let Some(p) = &path {
                if !matches!(p.as_str(), "greatsword" | "shield" | "dual") {
                    bail!("{what}: path is greatsword, shield or dual");
                }
            }
            kit.talents.push(TalentDef {
                key,
                name: s(t, "name")?,
                tree,
                tier,
                col,
                ranks: u(t, "ranks").unwrap_or(1).clamp(1, 5) as u8,
                icon: u(t, "icon")?,
                text: s(t, "text")?,
                values,
                effects,
                grants: None,
                requires,
                path,
            });
        }
        for (ti, g) in grants {
            let si = kit.spell_index(&g).with_context(|| format!("{class} talent {}: no spell {g}", kit.talents[ti].key))?;
            if !kit.spells[si].talent {
                bail!("{class} spell {g} is granted by a talent: mark it talent = true");
            }
            kit.talents[ti].grants = Some(si);
        }
        kit.validate()?;
        Ok(kit)
    }

    fn validate(&self) -> Result<()> {
        let class = &self.class;
        let mut keys = std::collections::HashSet::new();
        for sp in &self.spells {
            if !keys.insert(sp.key.clone()) {
                bail!("{class}: two spells called {}", sp.key);
            }
            let n = &sp.nums;
            let needs_cd = !matches!(sp.kind, SpellKind::Toggle);
            if needs_cd && n.cooldown <= 0.0 {
                bail!("{class} spell {}: needs a cooldown", sp.key);
            }
            if matches!(sp.kind, SpellKind::Nova | SpellKind::Cone | SpellKind::Zone) && n.radius <= 0.0 {
                bail!("{class} spell {}: needs a radius", sp.key);
            }
            if matches!(sp.kind, SpellKind::Buff | SpellKind::Toggle) && sp.effect.is_empty() {
                bail!("{class} spell {}: which effect?", sp.key);
            }
            let mut pk = std::collections::HashSet::new();
            for p in &sp.perks {
                if !pk.insert(&p.key) {
                    bail!("{class} spell {}: two perks called {}", sp.key, p.key);
                }
                if p.text.contains(['~', '^', ';']) {
                    bail!("{class} perk {}: no ~ ^ ; in text", p.key);
                }
            }
            if sp.note.contains(['~', '^', ';']) || sp.name.contains(['~', '^', ';']) {
                bail!("{class} spell {}: no ~ ^ ; in its name or note", sp.key);
            }
            if !sp.talent && sp.perks.len() < 4 {
                bail!("{class} spell {}: at least 4 perks", sp.key);
            }
        }
        if self.spells.iter().filter(|s| s.core && !s.talent).count() < 3 {
            bail!("{class}: at least 3 core spells");
        }
        let mut places = std::collections::HashSet::new();
        for t in &self.talents {
            if !keys.insert(format!("talent:{}", t.key)) {
                bail!("{class}: two talents called {}", t.key);
            }
            if !places.insert((t.tree, t.tier, t.col)) {
                bail!("{class} talent {}: its spot is taken", t.key);
            }
            if t.text.contains(['~', '^', ';', '|']) || t.name.contains(['~', '^', ';', '|']) {
                bail!("{class} talent {}: no ~ ^ ; | in text", t.key);
            }
        }
        Ok(())
    }
}

/// A list of `[[*.effect]]` tables: spell patches (with `spells`), `stats` from `allowed`, a team
/// aura, a named proc. Talents and the shop's items both use them.
pub(crate) fn parse_effects(kit: &Kit, list: &toml::Value, what: &str, allowed: &[&str]) -> Result<Vec<TalentEffect>> {
    let mut effects = Vec::new();
    for e in list.as_array().with_context(|| format!("{what}: effect must be a list of tables"))? {
        let e = e.as_table().context("effect")?;
        let filter = match e.get("spells").and_then(|v| v.as_str()) {
            None => None,
            Some(f) => Some(parse_filter(kit, f).with_context(|| format!("{what}: spells = {f}"))?),
        };
        let mut eff = TalentEffect { filter, patch: Patch::parse(e, what)?, ..TalentEffect::default() };
        if !eff.patch.is_empty() && eff.filter.is_none() {
            bail!("{what}: a spell patch needs `spells`");
        }
        for (field, list, allowed) in [("stats", &mut eff.stats, allowed), ("team", &mut eff.team, TEAM)] {
            if let Some(tab) = e.get(field) {
                for (k, v) in tab.as_table().with_context(|| format!("{what}: {field} must be a table"))? {
                    if !allowed.contains(&k.as_str()) {
                        bail!("{what}: {field}.{k} isn't known");
                    }
                    list.push((k.clone(), num(v)?));
                }
            }
        }
        if let Some(p) = e.get("proc") {
            let name = p.as_str().context("proc must be a name")?.to_string();
            if !PROCS.contains(&name.as_str()) {
                bail!("{what}: no proc called {name}");
            }
            let amount = e.get("amount").map(num).transpose()?.unwrap_or(0.0);
            eff.proc_ = Some((name, amount));
        }
        for k in e.keys() {
            if !matches!(k.as_str(), "spells" | "set" | "add" | "pct" | "more" | "stats" | "team" | "proc" | "amount") {
                bail!("{what}: unknown effect field {k}");
            }
        }
        effects.push(eff);
    }
    Ok(effects)
}

fn parse_filter(kit: &Kit, f: &str) -> Result<Filter> {
    let parts: Vec<&str> = f.split(',').map(|p| p.trim()).collect();
    let mut keys = Vec::new();
    let mut others = Vec::new();
    for p in parts {
        if p == "all" {
            others.push(Filter::All);
        } else if let Some(x) = p.strip_prefix("school:") {
            others.push(Filter::School(school_index(x).context("no such school")?));
        } else if let Some(x) = p.strip_prefix("tree:") {
            others.push(Filter::Tree(kit.tree_index(x).context("no such tree")?));
        } else if let Some(x) = p.strip_prefix("kind:") {
            let k: SpellKind = toml::Value::String(x.to_string()).try_into().context("no such kind")?;
            others.push(Filter::Kind(k));
        } else {
            if kit.spell_index(p).is_none() {
                bail!("no spell {p}");
            }
            keys.push(p.to_string());
        }
    }
    if !keys.is_empty() {
        others.push(Filter::Keys(keys));
    }
    Ok(if others.len() == 1 { others.pop().unwrap() } else { Filter::Any(others) })
}

fn arr<'a>(t: &'a toml::Table, k: &str) -> Result<Vec<&'a toml::Table>> {
    match t.get(k) {
        None => Ok(Vec::new()),
        Some(v) => v
            .as_array()
            .with_context(|| format!("{k} must be [[{k}]]"))?
            .iter()
            .map(|x| x.as_table().with_context(|| format!("{k}: a table expected")))
            .collect(),
    }
}

fn s(t: &toml::Table, k: &str) -> Result<String> {
    t.get(k).and_then(|v| v.as_str()).map(|s| s.to_string()).with_context(|| format!("missing {k}"))
}

fn s_or(t: &toml::Table, k: &str) -> String {
    t.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

fn u(t: &toml::Table, k: &str) -> Result<u32> {
    t.get(k).map(num).transpose()?.map(|f| f as u32).with_context(|| format!("missing {k}"))
}

fn b(t: &toml::Table, k: &str) -> bool {
    t.get(k).and_then(|v| v.as_bool()).unwrap_or(false)
}

/// Spell numbers by name, for quick lookups in tests and tools.
pub fn nums_map(n: &Nums) -> HashMap<&'static str, f32> {
    Nums::NAMES.iter().map(|k| (*k, n.get(k).unwrap_or(0.0))).collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn every_kit_loads_with_three_trees_of_talents() {
        let c = crate::Content::builtin();
        assert_eq!(c.kits.len(), 4);
        for kit in &c.kits {
            for tree in 0..3 {
                let points: u32 = kit.talents.iter().filter(|t| t.tree == tree).map(|t| t.ranks as u32).sum();
                assert!((18..=30).contains(&points), "{} {}: {points} points", kit.class, kit.trees[tree].key);
                assert!(kit.talents.iter().any(|t| t.tree == tree && t.tier == 5), "{} {} has a capstone", kit.class, kit.trees[tree].key);
            }
            let cards = kit.spells.iter().filter(|s| !s.talent).count();
            assert!(cards >= 10, "{}: {cards} spells", kit.class);
            for t in &kit.talents {
                let text = t.text_at(t.ranks);
                assert!(!text.contains('{'), "{}: {text}", t.key);
            }
        }
        let mage = &c.kits[c.class_index("mage").unwrap()];
        let shards = mage.talents.iter().find(|t| t.key == "ice_shards").unwrap();
        assert_eq!(shards.text_at(2), "Your frost critical strikes deal 30% more damage.");
    }
}
