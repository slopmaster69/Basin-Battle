//! Game content and tuning, loaded from `content/game.toml`, `content/cards.toml` and
//! `content/gear.toml`.

use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

/// The content shipped with the game, built in so the server runs without content files.
pub const DEFAULT_GAME: &str = include_str!("../../../content/game.toml");
pub const DEFAULT_CARDS: &str = include_str!("../../../content/cards.toml");
pub const DEFAULT_GEAR: &str = include_str!("../../../content/gear.toml");
pub const DEFAULT_ITEMS: &str = include_str!("../../../content/items.toml");
pub const DEFAULT_POWERS: &str = include_str!("../../../content/powers.toml");
/// Each class's spells and talents, by class key.
pub const DEFAULT_KITS: [(&str, &str); 4] = [
    ("warrior", include_str!("../../../content/classes/warrior.toml")),
    ("priest", include_str!("../../../content/classes/priest.toml")),
    ("mage", include_str!("../../../content/classes/mage.toml")),
    ("hunter", include_str!("../../../content/classes/hunter.toml")),
];

#[derive(Debug, Clone, Deserialize)]
pub struct Content {
    pub game: GameTuning,
    pub hub: HubDef,
    pub levels: LevelTuning,
    pub scaling: Scaling,
    pub bases: BaseTuning,
    #[serde(rename = "class")]
    pub classes: Vec<ClassDef>,
    #[serde(rename = "enemy")]
    pub enemies: Vec<EnemyDef>,
    #[serde(rename = "boss")]
    pub bosses: Vec<EnemyDef>,
    #[serde(rename = "affix", default)]
    pub affixes: Vec<AffixDef>,
    #[serde(rename = "renown", default)]
    pub renown: Vec<RenownDef>,
    /// What bosses and elites cast that players dodge.
    #[serde(rename = "ability", default)]
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub sprint: SprintTuning,
    #[serde(default)]
    pub defenses: DefenseTuning,
    #[serde(default)]
    pub pet: PetTuning,
    /// From `cards.toml`.
    #[serde(skip)]
    pub cards: Vec<CardDef>,
    /// From `gear.toml`.
    #[serde(skip)]
    pub gear: GearDefs,
    /// From `classes/<class>.toml`, in the order of `classes`.
    #[serde(skip)]
    pub kits: Vec<crate::kit::Kit>,
    /// From `items.toml`: the Hero tab's wares.
    #[serde(skip)]
    pub wares: Vec<crate::wares::WareDef>,
    /// From `powers.toml`: the Escalations' legendaries and the bosses' powers.
    #[serde(skip)]
    pub powers: Vec<crate::powers::PowerDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GameTuning {
    /// How many different spells (cards that fire on their own) and passives a build can hold.
    #[serde(default = "six")]
    pub spell_slots: usize,
    #[serde(default = "six")]
    pub passive_slots: usize,
    /// The Hearthstone's channel and cooldown, in seconds.
    #[serde(default = "hearth_secs")]
    pub hearth_secs: f32,
    #[serde(default = "hearth_cooldown")]
    pub hearth_cooldown: f32,
    /// A fallen hero gets up at the Ancient after this many seconds (plus a little each wave, up
    /// to the cap). Only the Ancient falling loses the run.
    /// The pause after a spell cast by hand before the next (shortened by haste).
    #[serde(default = "gcd_secs")]
    pub gcd_secs: f32,
    /// Extra spell damage for casting by hand.
    #[serde(default = "hand_cast_bonus")]
    pub hand_cast_bonus: f32,
    #[serde(default = "respawn_secs")]
    pub respawn_secs: f32,
    #[serde(default = "respawn_per_wave")]
    pub respawn_per_wave: f32,
    #[serde(default = "respawn_cap")]
    pub respawn_cap: f32,
    /// Gold every player gets for clearing a wave: base plus this much per wave.
    #[serde(default)]
    pub wave_gold: f32,
    #[serde(default)]
    pub wave_gold_per_wave: f32,
    pub countdown_secs: f32,
    pub intermission_secs: f32,
    pub boss_intermission_secs: f32,
    /// Clearing this wave (its boss leads it) wins the run; the waves go on, endless, after.
    #[serde(default = "victory_wave")]
    pub victory_wave: u32,
    /// The world stops while anyone chooses a card (and every pick must be made).
    #[serde(default = "yes")]
    pub pause_for_picks: bool,
    /// Now and then a wave comes differently: the Swarm (waves 7, 17...), the Vanguard (12, 22...).
    #[serde(default = "yes")]
    pub wave_events: bool,
    /// Loot goes straight to the players: no corpse to click.
    #[serde(default = "yes")]
    pub auto_loot: bool,
    pub corpse_secs: f32,
    pub loot_corpse_secs: f32,
    pub player_regen_per_sec: f32,
    pub hub_repair_per_wave: f32,
    pub escalation_warning_secs: f32,
    pub gold_per_kill: f32,
    pub gold_per_kill_per_wave: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HubDef {
    pub name: String,
    pub subname: String,
    pub display_id: u32,
    pub level: u32,
    pub health: u32,
    /// The hub's health grows by this share, compounding, each wave.
    pub health_per_wave: f32,
    pub reach: f32,
    pub bounding_radius: f32,
    /// The shopkeeper by the Ancient: everything the shop sells is bought from them, in person.
    #[serde(default)]
    pub vendor: VendorDef,
}

/// The shopkeeper at home (the Blacksmith): where heroes go between waves to buy.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct VendorDef {
    pub name: String,
    pub subname: String,
    pub display_id: u32,
    /// The hammer in their hand (an item display).
    pub weapon: u32,
    /// Where they stand: degrees round from the way the Ancient faces, and yards from it.
    pub angle: f32,
    pub distance: f32,
    /// How close a hero must be (to them or the Ancient) to buy.
    pub reach: f32,
}

impl Default for VendorDef {
    fn default() -> Self {
        Self {
            name: "Bruni Ironhammer".into(),
            subname: "Blacksmith and supplies".into(),
            display_id: 1362,
            weapon: 1597,
            angle: 150.0,
            distance: 9.0,
            reach: 16.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct LevelTuning {
    pub max_level: u32,
    pub health_per_level: f32,
    pub damage_per_level: f32,
    pub xp_base: u32,
    pub xp_per_level: u32,
    /// Kills carry a hero at most this many levels past the waves begun (1 + waves + bonuses).
    #[serde(default = "one_u32")]
    pub levels_per_wave: u32,
    /// Bonus levels for objectives: killing a boss, claiming a base for the first time.
    #[serde(default = "one_u32")]
    pub boss_bonus: u32,
    #[serde(default = "one_u32")]
    pub base_bonus: u32,
}

impl LevelTuning {
    /// Experience from `level` to the next.
    pub fn xp_to_next(&self, level: u32) -> u32 {
        self.xp_base + self.xp_per_level * level
    }
    /// Experience from level 1 to the start of `level`.
    pub fn xp_total(&self, level: u32) -> u64 {
        (1..level.max(1)).map(|l| self.xp_to_next(l) as u64).sum()
    }
    /// The level a team that cleared every wave up to `wave` should be at: a level a wave, one
    /// for each boss (every fifth wave) and one for each of the four bases (taken from wave 3).
    pub fn target_level(&self, wave: u32) -> u32 {
        let bosses = wave / 5 * self.boss_bonus;
        let bases = wave.saturating_sub(2).min(4) * self.base_bonus;
        (1 + wave * self.levels_per_wave + bosses + bases).min(self.max_level)
    }
    /// Damage and health multiplier at `level`.
    pub fn mult(&self, per_level: f32, level: u32) -> f32 {
        1.0 + per_level * (level.max(1) - 1) as f32
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Scaling {
    /// What elites cast, when the enemy has no abilities of its own.
    #[serde(default)]
    pub elite_abilities: Vec<String>,
    /// Wave elites carry a twist: vampiric, frenzied, swift or thorned.
    #[serde(default = "yes")]
    pub elite_modifiers: bool,
    pub health_early: f32,
    pub health_late: f32,
    pub late_from: u32,
    pub damage_wave: f32,
    /// Enemy swings and shots at the Ancient's defences land this much harder.
    #[serde(default = "one_f32")]
    pub defence_damage: f32,
    /// The stream comes 1 + (stream_ramp_waves - wave) / 3 times slower before that wave.
    #[serde(default)]
    pub stream_ramp_waves: u32,
    /// Enemy swings and shots at heroes (not at defences or the Ancient) hit this much harder,
    /// plus `hero_damage_per_wave` more each wave after the first: gentle early, hard late.
    #[serde(default = "one_f32")]
    pub hero_damage: f32,
    #[serde(default)]
    pub hero_damage_per_wave: f32,
    pub health_cycle: f32,
    pub damage_cycle: f32,
    pub count_base: f32,
    pub count_per_wave: f32,
    pub count_player_base: f32,
    pub count_per_player: f32,
    pub count_cap: u32,
    pub elite_share_base: f32,
    pub elite_share_per_wave: f32,
    pub elite_share_cap: f32,
    pub elite_health: f32,
    pub elite_damage: f32,
    pub elite_scale: f32,
    pub boss_every: u32,
    pub boss_health: f32,
    pub boss_health_per_extra_player: f32,
    pub spawn_interval: f32,
    /// The endless stream: every open lane whose base isn't claimed sends one more enemy every
    /// this many seconds during a wave, and every `stream_rest_secs` between waves (0: never).
    #[serde(default)]
    pub stream_secs: f32,
    #[serde(default = "two")]
    pub stream_from_wave: u32,
    #[serde(default)]
    pub stream_rest_secs: f32,
    /// Most stream enemies alive at once.
    #[serde(default = "stream_cap")]
    pub stream_cap: u32,
    /// Enemies marching on the Ancient move this much faster than their own speed (plus a
    /// little more each wave, up to the cap); they drop back to their own pace to fight.
    #[serde(default = "one")]
    pub march_speed: f32,
    #[serde(default)]
    pub march_speed_per_wave: f32,
    #[serde(default = "one")]
    pub march_speed_cap: f32,
    /// Within this many yards of the Ancient they slow back down.
    #[serde(default)]
    pub march_until: f32,
    /// `[first wave, lanes]` pairs, ascending.
    pub lanes_by_wave: Vec<[u32; 2]>,
    /// Every enemy's health is this much of its listed health...
    #[serde(default = "one")]
    pub health_base: f32,
    /// ...and grows by this share more each wave, keeping pace with what upgrades and talents
    /// add to a hero (on top of `health_early`/`health_late`, which follow the levels).
    #[serde(default)]
    pub health_per_wave: f32,
    /// A party: each hero past the first adds this share of enemy health and damage (four
    /// heroes cover each other, heal each other and stand together).
    #[serde(default)]
    pub party_health: f32,
    #[serde(default)]
    pub party_damage: f32,
    /// ...and this share faster spawning.
    #[serde(default)]
    pub party_spawn: f32,
    /// A hero alone: enemy health and damage are this much of normal, the stream comes this
    /// many times slower, and at most this share of `stream_cap` of it is alive at once.
    #[serde(default = "one")]
    pub solo_health: f32,
    #[serde(default = "one")]
    pub solo_damage: f32,
    #[serde(default = "one")]
    pub solo_stream_slower: f32,
    #[serde(default = "one")]
    pub solo_stream_cap: f32,
}

impl Scaling {
    /// Health multiplier for wave `w` (1-based) in cycle `c`.
    pub fn health(&self, w: u32, c: u32) -> f32 {
        let w = w.max(1);
        let early = w.min(self.late_from) - 1;
        let late = w.saturating_sub(self.late_from);
        self.health_base
            * (1.0 + self.health_per_wave * (w - 1) as f32)
            * self.health_early.powi(early as i32)
            * self.health_late.powi(late as i32)
            * self.health_cycle.powi(c as i32)
    }
    pub fn damage(&self, w: u32, c: u32) -> f32 {
        self.damage_wave.powi(w.max(1) as i32 - 1) * self.damage_cycle.powi(c as i32)
    }
    /// Enemies in the wave before the cap, as a fraction.
    pub fn raw_count(&self, w: u32, players: usize) -> f32 {
        (self.count_base + self.count_per_wave * (w.max(1) - 1) as f32)
            * (self.count_player_base + self.count_per_player * players.max(1) as f32)
    }
    pub fn elite_share(&self, w: u32) -> f32 {
        // The first wave is a warm-up: no elites yet.
        if w <= 1 {
            return 0.0;
        }
        (self.elite_share_base + self.elite_share_per_wave * w as f32).min(self.elite_share_cap)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct BaseTuning {
    pub open_from_wave: u32,
    pub guardian_elite: String,
    pub guardian_adds: u32,
    pub guardian_health: f32,
    pub guardian_per_claim: f32,
    pub leash: f32,
    pub channel_secs: f32,
    pub lane_cut: f32,
    pub gate_display: u32,
    pub cache_display: u32,
    pub banner_display: u32,
    pub claimed_display: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClassDef {
    pub key: String,
    pub name: String,
    pub class_id: u8,
    pub race: u8,
    pub gender: u8,
    pub display_id: u32,
    pub faction: u32,
    pub power: u8,
    pub max_power: u32,
    pub health: u32,
    /// The class's own toughness: this share less damage taken from everything (a warrior's).
    #[serde(default)]
    pub taken: f32,
    pub damage: [u32; 2],
    pub attack_secs: f32,
    pub range: f32,
    /// The ranged attack's spell (its missile); 0 for melee.
    pub attack_spell: u32,
    pub main_stat: String,
    pub armor: String,
    /// The equipment slot the class's weapon goes in: 15 main hand, 17 ranged.
    pub weapon_slot: u8,
    /// skin, face, hair style, hair colour, facial hair.
    pub appearance: [u8; 5],
}

impl ClassDef {
    pub fn ranged(&self) -> bool {
        self.attack_spell != 0
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct EnemyDef {
    pub key: String,
    pub name: String,
    pub display_id: u32,
    pub health: u32,
    pub damage: [u32; 2],
    pub attack_secs: f32,
    pub speed: f32,
    pub scale: f32,
    pub creature_type: u32,
    pub rank: u32,
    pub aggro_radius: f32,
    pub hub_damage: f32,
    pub first_wave: u32,
    #[serde(default = "one")]
    pub weight: f32,
    #[serde(default = "one")]
    pub xp: f32,
    /// Keys of the abilities it casts (see `[[ability]]`).
    #[serde(default)]
    pub abilities: Vec<String>,
    /// The bases whose lanes send it ("the Farm"); empty for every lane.
    #[serde(default)]
    pub bases: Vec<String>,
    /// What makes it more than a body in the crowd: "siege" (goes for your defences), "archer"
    /// (shoots from range), "healer" (mends the others), "flanker" (takes the long road and
    /// runs past everyone for the Ancient). Empty for none.
    #[serde(default)]
    pub role: String,
    /// A sapper's blast at the Ancient: this share of its max health.
    #[serde(default)]
    pub blast: f32,
    /// The line under its name: "Siege".
    #[serde(default)]
    pub title: String,
    /// Archers: how far it shoots, and the spell its shots look like.
    #[serde(default)]
    pub range: f32,
    #[serde(default)]
    pub shot_spell: u32,
    /// Healers: the share of max health it mends, how often, and how it looks.
    #[serde(default)]
    pub heal: f32,
    #[serde(default)]
    pub heal_secs: f32,
    #[serde(default)]
    pub heal_spell: u32,
    /// Siege: how much harder it hits defences (and how far it looks for them).
    #[serde(default)]
    pub siege: f32,
    /// Summoners: the enemy it raises (a key), how many at a time, how often, how many of its
    /// own may stand at once, and how the raising looks.
    #[serde(default)]
    pub summon: String,
    #[serde(default)]
    pub summon_count: u32,
    #[serde(default)]
    pub summon_secs: f32,
    #[serde(default)]
    pub summon_cap: u32,
    #[serde(default)]
    pub summon_spell: u32,
    /// Splitters: what it falls apart into when it dies (a key), and how many.
    #[serde(default)]
    pub split_into: String,
    #[serde(default)]
    pub split_count: u32,
    /// Bosses: at half health they call this enemy (a key) to their side, this many of them;
    /// at a quarter they're enraged.
    #[serde(default)]
    pub phase_adds: String,
    #[serde(default)]
    pub phase_add_count: u32,
    /// Haunts: every second, `pulse` times its average hit to every hero within `pulse_radius`
    /// yards, shown as `pulse_spell`.
    #[serde(default)]
    pub pulse: f32,
    #[serde(default)]
    pub pulse_radius: f32,
    #[serde(default)]
    pub pulse_spell: u32,
    /// Its share of a wave enemy's health, for one raised or split off.
    #[serde(default = "add_health")]
    pub add_health: f32,
}

fn add_health() -> f32 {
    0.5
}

/// An enemy ability with a warning: a marker on the ground, a cast, then the hit.
#[derive(Debug, Clone, Deserialize)]
pub struct AbilityDef {
    pub key: String,
    pub name: String,
    /// "slam": everything around the caster; "pool": a lasting patch under some of the players;
    /// "line": a row of marks from the caster towards a player (a charge, a breath); "ring": a ring
    /// of marks around the caster, safe inside (get close!).
    pub kind: String,
    /// Seconds of warning before it lands.
    pub cast_secs: f32,
    pub radius: f32,
    /// Damage as a multiple of one of the caster's average melee hits; a pool deals it every second.
    pub damage: f32,
    pub cooldown: f32,
    /// Seconds after the caster first meets a player before its first cast.
    #[serde(default = "first_after")]
    pub first_after: f32,
    /// The spell the caster is seen casting (its name and animation).
    pub spell: u32,
    /// The spell whose ground effect marks the danger.
    pub telegraph: u32,
    /// The spell visual when it lands, if any.
    #[serde(default)]
    pub impact: u32,
    #[serde(default)]
    pub school: u8,
    /// Pools: how many players it targets, and how long a pool lasts once it lands.
    #[serde(default = "one_u32")]
    pub targets: u32,
    #[serde(default)]
    pub duration: f32,
    /// Slows players standing in it.
    #[serde(default)]
    pub slow: f32,
    /// The warning players see: "Bone Storm: get away from the Bonelord!".
    pub warn: String,
    /// The caster dies when it lands (a bloat bursting).
    #[serde(default)]
    pub suicide: bool,
    /// "line": how far the line of marks runs from the caster towards its target.
    #[serde(default = "line_length")]
    pub length: f32,
    /// "ring": how far from the caster the ring of marks stands; inside it is safe.
    #[serde(default = "ring_distance")]
    pub ring: f32,
    /// A rare kind of strike: it takes this share of each hero's max health, whatever their
    /// level (`damage` still sets what it does to anything else).
    #[serde(default)]
    pub health_share: f32,
}

fn line_length() -> f32 {
    30.0
}

fn ring_distance() -> f32 {
    10.0
}

fn two() -> u32 {
    2
}

fn stream_cap() -> u32 {
    24
}

fn six() -> usize {
    6
}

fn hearth_secs() -> f32 {
    3.0
}

fn hand_cast_bonus() -> f32 {
    0.15
}

fn gcd_secs() -> f32 {
    1.0
}

fn respawn_secs() -> f32 {
    8.0
}

fn respawn_per_wave() -> f32 {
    1.0
}

fn respawn_cap() -> f32 {
    20.0
}

fn hearth_cooldown() -> f32 {
    20.0
}

fn first_after() -> f32 {
    4.0
}

fn one_u32() -> u32 {
    1
}

/// The hunter's pet.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PetTuning {
    /// A bite's share of its master's hit (every 2 seconds), its health as a share of its
    /// master's, seconds to come back after falling, its run speed and its look.
    pub damage: f32,
    pub health: f32,
    pub respawn: f32,
    pub speed: f32,
    pub display: u32,
    pub scale: f32,
    pub name: String,
}

impl Default for PetTuning {
    fn default() -> Self {
        Self { damage: 0.45, health: 0.7, respawn: 12.0, speed: 8.0, display: 780, scale: 1.0, name: "Wolf".into() }
    }
}

/// The Ancient's defences, bought with gold at the Ancient.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DefenseTuning {
    /// A normal enemy's health before scaling: defence damage is a share of it, so it keeps pace.
    pub reference_health: f32,
    /// The price of each Protector, Spring and Defender in turn (their count is the list's length).
    pub protector_costs: Vec<u32>,
    pub spring_costs: Vec<u32>,
    pub defender_costs: Vec<u32>,
    /// How far from the Ancient they stand.
    pub protector_ring: f32,
    pub spring_ring: f32,
    /// Protectors: reach, seconds between bolts, a bolt's share of a normal enemy's health, the
    /// bolt's spell, and the look.
    pub protector_range: f32,
    pub protector_secs: f32,
    pub protector_damage: f32,
    pub protector_spell: u32,
    pub protector_display: u32,
    pub protector_scale: f32,
    /// Springs: reach, share of max health healed a second (players, the Ancient), spell, look.
    pub spring_radius: f32,
    pub spring_heal: f32,
    pub spring_hub_heal: f32,
    pub spring_spell: u32,
    pub spring_display: u32,
    pub spring_scale: f32,
    /// Defenders: a blow's share of a normal enemy's health (every 2 seconds), and the look.
    pub defender_damage: f32,
    pub defender_display: u32,
    /// The sword a defender holds (an item display id): the guard model has no unarmed swing.
    #[serde(default = "default_defender_weapon")]
    pub defender_weapon: u32,
    /// Fortifying: the first price, its growth per rank, and the bonus per rank.
    pub upgrade_cost: u32,
    pub upgrade_growth: f32,
    pub upgrade_bonus: f32,
    /// Shared buys cost this share more per hero past the first (each hero earns from every kill).
    #[serde(default)]
    pub party_price: f32,
    /// Toughness, as a multiple of a normal enemy's health this wave: enemies marching past
    /// stop to tear them down, and a fallen one can be rebuilt at the Ancient.
    pub protector_health: f32,
    pub spring_health: f32,
    pub defender_health: f32,
    /// How close an enemy on its way to the Ancient has to pass to turn on a defence.
    pub aggro: f32,
    /// How many enemies a defence (a soldier, a treant, a spring, a pet) holds at once; the rest
    /// march past it to the next one, a hero, or the Ancient. 0: no limit.
    pub hold: u32,
    /// Riflemen: the price of each, a shot's share of a normal enemy's health (every 2.5 sec,
    /// out to `rifleman_range`), toughness, and the look (a dwarf and his rifle).
    pub rifleman_costs: Vec<u32>,
    pub rifleman_damage: f32,
    pub rifleman_range: f32,
    pub rifleman_health: f32,
    pub rifleman_display: u32,
    pub rifleman_weapon: u32,
    /// Clerics: prices, the share of max health each heal mends (every 3 sec, within
    /// `cleric_range`), toughness, and the look.
    pub cleric_costs: Vec<u32>,
    pub cleric_heal: f32,
    pub cleric_range: f32,
    pub cleric_health: f32,
    pub cleric_display: u32,
    pub cleric_weapon: u32,
    /// The defenders' ranks, as Promotion raises them: rank 0 is the Basin Defender.
    pub ranks: Vec<SoldierRank>,
}

/// One rank of the Ancient's defenders: its name, look, sword and shield (item displays, 0 for
/// none).
#[derive(Debug, Clone, Deserialize)]
pub struct SoldierRank {
    pub name: String,
    pub display: u32,
    pub weapon: u32,
    #[serde(default)]
    pub shield: u32,
}

impl Default for DefenseTuning {
    fn default() -> Self {
        Self {
            reference_health: 300.0,
            protector_costs: vec![50, 100, 170, 260],
            spring_costs: vec![80, 180],
            defender_costs: vec![30, 50, 75, 105, 140, 180],
            protector_ring: 11.0,
            spring_ring: 9.0,
            protector_range: 32.0,
            protector_secs: 1.6,
            protector_damage: 0.08,
            protector_spell: 5176,
            protector_display: 1921,
            protector_scale: 0.6,
            spring_radius: 14.0,
            spring_heal: 0.025,
            spring_hub_heal: 0.004,
            spring_spell: 5394,
            spring_display: 2030,
            spring_scale: 1.6,
            protector_health: 9.0,
            spring_health: 12.0,
            defender_health: 6.0,
            aggro: 9.0,
            hold: 4,
            rifleman_costs: vec![60, 90, 130, 180],
            rifleman_damage: 0.07,
            rifleman_range: 26.0,
            rifleman_health: 3.5,
            rifleman_display: 1406,
            rifleman_weapon: 2409,
            cleric_costs: vec![90, 160],
            cleric_heal: 0.05,
            cleric_range: 25.0,
            cleric_health: 4.0,
            cleric_display: 1616,
            cleric_weapon: 1161,
            ranks: vec![
                SoldierRank { name: "Basin Defender".into(), display: 164, weapon: 1928, shield: 0 },
                SoldierRank { name: "Basin Footman".into(), display: 1769, weapon: 1928, shield: 2331 },
                SoldierRank { name: "Knight of the Basin".into(), display: 2042, weapon: 2806, shield: 18533 },
                SoldierRank { name: "Stormwind Guard".into(), display: 1984, weapon: 25053, shield: 31733 },
            ],
            defender_damage: 0.12,
            defender_display: 164,
            defender_weapon: default_defender_weapon(),
            upgrade_cost: 120,
            upgrade_growth: 1.5,
            upgrade_bonus: 0.35,
            party_price: 0.0,
        }
    }
}

/// Sprint pick-ups along the lanes.
#[derive(Debug, Clone, Deserialize)]
pub struct SprintTuning {
    /// Where along each lane, from its spawn end (0) to the hub (1).
    pub along: Vec<f32>,
    pub display_id: u32,
    pub scale: f32,
    /// Speed bonus and how long it lasts.
    pub speed: f32,
    pub secs: f32,
    /// Seconds until a taken pick-up comes back.
    pub respawn: f32,
    /// How close a player must walk.
    pub reach: f32,
}

impl Default for SprintTuning {
    fn default() -> Self {
        Self { along: Vec::new(), display_id: 4590, scale: 1.0, speed: 0.7, secs: 8.0, respawn: 25.0, reach: 2.5 }
    }
}

fn yes() -> bool {
    true
}

fn victory_wave() -> u32 {
    20
}

fn default_defender_weapon() -> u32 {
    1928
}

fn one_f32() -> f32 {
    1.0
}

fn one() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct AffixDef {
    pub key: String,
    pub name: String,
    pub text: String,
    pub spell: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenownDef {
    pub key: String,
    pub name: String,
    pub text: String,
    /// The cost of each level, in order.
    pub costs: Vec<u32>,
}

// --- cards -----------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl Rarity {
    pub fn name(self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
        }
    }
    /// The item-quality colour code for chat and menus.
    pub fn color(self) -> &'static str {
        match self {
            Rarity::Common => "ffffffff",
            Rarity::Uncommon => "ff1eff00",
            Rarity::Rare => "ff0070dd",
            Rarity::Epic => "ffa335ee",
            Rarity::Legendary => "ffff8000",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CardKind {
    Bolt,
    Nova,
    Zone,
    Modifier,
    Stat,
    Trigger,
    Team,
    /// Heals the most hurt ally in range.
    Heal,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CardDef {
    pub key: String,
    pub name: String,
    pub rarity: Rarity,
    #[serde(default)]
    pub classes: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    /// Icon on the buff bar, and the visual unless `visual` is set.
    pub spell: u32,
    #[serde(default)]
    pub visual: u32,
    /// The buff-bar spell, when `spell`'s own aura would paint an effect on the player.
    #[serde(default)]
    pub aura: u32,
    pub kind: CardKind,
    #[serde(default)]
    pub cooldown: f32,
    #[serde(default)]
    pub damage: f32,
    #[serde(default)]
    pub range: f32,
    #[serde(default)]
    pub radius: f32,
    #[serde(default)]
    pub duration: f32,
    #[serde(default)]
    pub school: u8,
    #[serde(default)]
    pub slow: f32,
    #[serde(default)]
    pub slow_secs: f32,
    #[serde(default)]
    pub root_secs: f32,
    #[serde(default)]
    pub heal: f32,
    #[serde(default)]
    pub projectiles: u32,
    #[serde(default)]
    pub targets: u32,
    #[serde(default)]
    pub chain: u32,
    #[serde(default)]
    pub falloff: f32,
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub modifier: String,
    #[serde(default)]
    pub stat: String,
    #[serde(default)]
    pub trigger: String,
    #[serde(default)]
    pub team: String,
    #[serde(default)]
    pub amount: f32,
    #[serde(default)]
    pub secs: f32,
    /// Damage over time each hit leaves, as a multiple of your hit over `dot_secs`.
    #[serde(default)]
    pub dot: f32,
    #[serde(default)]
    pub dot_secs: f32,
    /// Share of the damage done that heals you.
    #[serde(default)]
    pub leech: f32,
    /// Extra damage (1.0 = double) against enemies below 25% health.
    #[serde(default)]
    pub execute: f32,
    /// How much bigger it makes the hero who holds it (0.15: 15% bigger), so a choice shows.
    #[serde(default)]
    pub grow: f32,
}

impl CardDef {
    /// The spell whose visual plays when it fires.
    pub fn visual(&self) -> u32 {
        if self.visual != 0 {
            self.visual
        } else {
            self.spell
        }
    }
    /// The spell shown on the buff bar.
    pub fn aura(&self) -> u32 {
        if self.aura != 0 {
            self.aura
        } else {
            self.spell
        }
    }
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
    /// Strength multiplier at `rank` (1-5): stats grow by their whole amount per rank, everything
    /// else by half.
    pub fn rank_mult(&self, rank: u8) -> f32 {
        let r = rank.max(1) as f32;
        if self.kind == CardKind::Stat {
            r
        } else {
            1.0 + 0.5 * (r - 1.0)
        }
    }
    /// How far it ranks up: Undying Will has nothing to gain past rank 1.
    pub fn max_rank(&self) -> u8 {
        if self.kind == CardKind::Trigger && self.trigger == "undying" {
            1
        } else {
            5
        }
    }
    pub fn is_ability(&self) -> bool {
        matches!(self.kind, CardKind::Bolt | CardKind::Nova | CardKind::Zone | CardKind::Heal)
    }

    /// Extra effects common to abilities, as a phrase.
    fn extras(&self, m: f32) -> String {
        let pct = |v: f32| format!("{}%", (v * 100.0).round() as i32);
        let mut s = String::new();
        if self.dot > 0.0 {
            s.push_str(&format!(", plus {} over {}s", pct(self.dot * m), trim(self.dot_secs)));
        }
        if self.root_secs > 0.0 {
            s.push_str(&format!(", holds them {}s", trim(self.root_secs)));
        }
        if self.slow > 0.0 {
            s.push_str(&format!(", slows {}", pct(self.slow)));
        }
        if self.leech > 0.0 {
            s.push_str(&format!(", heals you for {} of it", pct(self.leech)));
        }
        if self.execute > 0.0 {
            s.push_str(&format!(", {} more below 25% health", pct(self.execute)));
        }
        s
    }

    /// What the card does at `rank`, in a line.
    pub fn describe(&self, rank: u8) -> String {
        let m = self.rank_mult(rank);
        let pct = |v: f32| format!("{}%", (v * 100.0).round() as i32);
        match self.kind {
            CardKind::Bolt => {
                let what = if self.damage > 0.0 { pct(self.damage * m) } else { "a hex".to_string() };
                let mut s = format!("Every {}s: {} of your hit", trim(self.cooldown), what);
                if self.damage <= 0.0 {
                    s = format!("Every {}s: a curse", trim(self.cooldown));
                }
                if self.projectiles > 1 {
                    s = format!("Every {}s: {} missiles of {}", trim(self.cooldown), self.projectiles, pct(self.damage * m));
                }
                if self.targets > 1 {
                    s.push_str(&format!(" at {} targets", self.targets));
                }
                if self.chain > 0 {
                    s.push_str(&format!(", jumping {} times", self.chain));
                }
                s.push_str(&self.extras(m));
                s
            }
            CardKind::Nova => {
                let mut s = format!(
                    "Every {}s: {} to all within {} yd",
                    trim(self.cooldown),
                    pct(self.damage * m),
                    trim(self.radius)
                );
                s.push_str(&self.extras(m));
                if self.heal > 0.0 {
                    s.push_str(&format!(", heals allies {}", pct(self.heal * m)));
                }
                s
            }
            CardKind::Heal => {
                let mut s = format!("Every {}s: heals the most hurt ally for {} of their health", trim(self.cooldown), pct(self.heal * m));
                if self.targets > 1 {
                    s.push_str(&format!(", then {} more", self.targets - 1));
                }
                s
            }
            CardKind::Zone => format!(
                "Every {}s: {} per second for {}s in {} yd{}",
                trim(self.cooldown),
                pct(self.damage * m),
                trim(self.duration),
                trim(self.radius),
                if self.at == "self" { " around you" } else { " on the biggest pack" }
            ),
            CardKind::Modifier => match self.modifier.as_str() {
                "dot" => format!("Your attacks add {} over {}s", pct(self.amount * m), trim(self.secs)),
                "burn" => format!("Fire hits burn for {} over {}s", pct(self.amount * m), trim(self.secs)),
                "cleave" => format!("Attacks hit 1 more enemy for {}", pct(self.amount * m)),
                "radius" => format!("Areas {} larger", pct(self.amount * m)),
                "cdr" => format!("Cards recharge {} faster", pct(1.0 - 1.0 / (1.0 + self.amount * m))),
                "echo" => format!("{} chance for abilities to fire twice", pct((self.amount * m).min(1.0))),
                "split" => format!("Missiles split into 2 for {}", pct(self.amount * m)),
                "windfury" => format!("{} chance for an extra attack", pct(self.amount * m)),
                other => other.to_string(),
            },
            CardKind::Stat => match self.stat.as_str() {
                "damage" => format!("+{} damage", pct(self.amount * m)),
                "haste" => format!("+{} attack and cast speed", pct(self.amount * m)),
                "crit" => format!("+{} critical strike chance", pct(self.amount * m)),
                "health" => format!("+{} health", pct(self.amount * m)),
                "regen" => format!("Regenerate {} health a second", pct(self.amount * m)),
                "speed" => format!("+{} movement speed", pct(self.amount * m)),
                "per_wave" => format!("+{} damage for every wave reached", pct(self.amount * m)),
                "avatar" => format!("+{} damage and health", pct(self.amount * m)),
                "leech" => format!("All your damage heals you for {} of it", pct(self.amount * m)),
                "execute" => format!("+{} damage to enemies below 25% health", pct(self.amount * m)),
                other => other.to_string(),
            },
            CardKind::Trigger => match self.trigger.as_str() {
                "lightning_shield" => format!("Attackers take {} (3 charges)", pct(self.amount * m)),
                "bloodthirst" => format!("Critical hits heal {} of your health", pct(self.amount * m)),
                "divine_shield" => format!(
                    "Below 35% health: immune for {}s (every {}s)",
                    trim(self.amount * m),
                    trim(self.cooldown)
                ),
                "raise_dead" => format!("{} of your kills rise to fight for you", pct(self.amount * m)),
                "soul_harvest" => format!("+{} damage for every 10 kills this run", trim_pct(self.amount * m)),
                "undying" => "Once a wave, a killing blow leaves you at 30% health instead".to_string(),
                "shield" => format!("A shield absorbs {} of your health, renewed every {}s", pct(self.amount * m), trim(self.cooldown)),
                "thorns" => format!("Melee attackers take {} of your hit", pct(self.amount * m)),
                other => other.to_string(),
            },
            CardKind::Team => match self.team.as_str() {
                "damage_taken" => format!("Allies within {} yd take {} less damage", trim(self.radius), pct(self.amount * m)),
                "damage" => format!("Allies within {} yd deal {} more damage", trim(self.radius), pct(self.amount * m)),
                "haste" => format!("Allies within {} yd attack and cast {} faster", trim(self.radius), pct(self.amount * m)),
                "regen" => format!("Allies within {} yd regenerate {} health a second", trim(self.radius), pct(self.amount * m)),
                other => other.to_string(),
            },
        }
    }
}

fn trim(v: f32) -> String {
    let s = format!("{v:.1}");
    s.trim_end_matches(".0").to_string()
}

fn trim_pct(v: f32) -> String {
    let s = format!("{:.2}", v * 100.0);
    format!("{}%", s.trim_end_matches('0').trim_end_matches('.'))
}

#[derive(Debug, Clone, Deserialize)]
struct CardsFile {
    card: Vec<CardDef>,
}

// --- gear -------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GearDefs {
    pub budget: GearBudget,
    #[serde(rename = "set")]
    pub sets: Vec<SetDef>,
    #[serde(rename = "bonus")]
    pub bonuses: Vec<BonusDef>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GearBudget {
    pub base: f32,
    pub tier_mult: f32,
    pub quality: Vec<f32>,
    pub slot_weight: Vec<f32>,
    pub required_level: Vec<u32>,
    pub item_level: Vec<u32>,
    pub stamina_health: f32,
    pub main_stat_damage: f32,
    pub spirit_regen: f32,
    pub armor: HashMap<String, f32>,
    pub armor_k: f32,
    /// More armour needed per level of the enemies striking: the 1.12 rule (400 + 85 a level).
    #[serde(default)]
    pub armor_k_per_level: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetDef {
    pub class: String,
    pub names: Vec<String>,
    pub slots: Vec<String>,
    pub displays: Vec<Vec<u32>>,
    pub weapon_names: Vec<String>,
    pub weapons: Vec<u32>,
    pub weapon_glow: Vec<u32>,
    pub legendary_weapon: u32,
    pub legendary_name: String,
    /// A warrior's weapon paths: the one-handed sword (shield and dual paths), the shield, and
    /// the off-hand weapon, by tier, each with a name, a look and the legendary's.
    #[serde(default)]
    pub path_gear: Option<PathGear>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PathGear {
    pub one_hand_names: Vec<String>,
    pub one_hands: Vec<u32>,
    pub shield_names: Vec<String>,
    pub shields: Vec<u32>,
    pub off_hand_names: Vec<String>,
    pub off_hands: Vec<u32>,
    /// (one-hand, shield, off-hand) looks and names for a legendary.
    pub legendary: [u32; 3],
    pub legendary_names: [String; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct BonusDef {
    pub class: String,
    pub two: String,
    pub four: String,
    pub six: String,
}

impl Content {
    /// Content from a `game.toml`, with `cards.toml` and `gear.toml` read from the same folder
    /// when they're there and the built-in ones otherwise.
    pub fn load(game_toml: &Path) -> Result<Self> {
        let read = |p: &Path| std::fs::read_to_string(p).with_context(|| format!("reading {}", p.display()));
        let game = read(game_toml)?;
        let dir = game_toml.parent().unwrap_or(Path::new("."));
        let cards = dir.join("cards.toml");
        let gear = dir.join("gear.toml");
        let cards = if cards.exists() { read(&cards)? } else { DEFAULT_CARDS.to_string() };
        let gear = if gear.exists() { read(&gear)? } else { DEFAULT_GEAR.to_string() };
        let mut kits = Vec::new();
        for (key, builtin) in DEFAULT_KITS {
            let path = dir.join("classes").join(format!("{key}.toml"));
            kits.push((key.to_string(), if path.exists() { read(&path)? } else { builtin.to_string() }));
        }
        let mut c = Self::from_all(&game, &cards, &gear, &kits)?;
        let items = dir.join("items.toml");
        if items.exists() {
            c.wares = crate::wares::parse(&read(&items)?, &c.classes, &c.kits)?;
        }
        let powers = dir.join("powers.toml");
        if powers.exists() {
            c.powers = crate::powers::parse(&read(&powers)?, &c.classes, &c.kits)?;
        }
        Ok(c)
    }

    pub fn from_parts(game: &str, cards: &str, gear: &str) -> Result<Self> {
        let kits: Vec<(String, String)> = DEFAULT_KITS.iter().map(|(k, t)| (k.to_string(), t.to_string())).collect();
        Self::from_all(game, cards, gear, &kits)
    }

    /// Everything: game.toml, cards.toml, gear.toml and the class kits (class key, text).
    pub fn from_all(game: &str, cards: &str, gear: &str, kits: &[(String, String)]) -> Result<Self> {
        let mut c: Content = toml::from_str(game).context("reading game.toml")?;
        c.cards = toml::from_str::<CardsFile>(cards).context("reading cards.toml")?.card;
        c.gear = toml::from_str(gear).context("reading gear.toml")?;
        for class in &c.classes {
            let (_, text) = kits
                .iter()
                .find(|(k, _)| *k == class.key)
                .with_context(|| format!("no classes/{}.toml", class.key))?;
            c.kits.push(crate::kit::Kit::parse(&class.key, text)?);
        }
        c.wares = crate::wares::parse(DEFAULT_ITEMS, &c.classes, &c.kits)?;
        c.powers = crate::powers::parse(DEFAULT_POWERS, &c.classes, &c.kits)?;
        c.validate()?;
        Ok(c)
    }

    /// Just a game.toml, with the built-in cards and gear.
    pub fn from_toml(game: &str) -> Result<Self> {
        Self::from_parts(game, DEFAULT_CARDS, DEFAULT_GEAR)
    }

    pub fn builtin() -> Self {
        Self::from_parts(DEFAULT_GAME, DEFAULT_CARDS, DEFAULT_GEAR).expect("the built-in content is valid")
    }

    /// A class's kit, by class index.
    pub fn kit(&self, class: usize) -> &crate::kit::Kit {
        &self.kits[class]
    }

    pub fn ability_index(&self, key: &str) -> Option<usize> {
        self.abilities.iter().position(|a| a.key == key)
    }

    fn validate(&self) -> Result<()> {
        if self.classes.is_empty() {
            bail!("content has no classes");
        }
        if self.classes.len() > 8 {
            bail!("at most 8 classes");
        }
        if self.enemies.is_empty() {
            bail!("content has no enemies");
        }
        if self.enemies.len() > 400 || self.bosses.len() > 400 {
            bail!("too many enemies");
        }
        for e in self.enemies.iter().chain(&self.bosses) {
            if e.damage[0] > e.damage[1] || e.speed <= 0.0 || e.attack_secs <= 0.0 {
                bail!("enemy {} has bad numbers", e.key);
            }
            for a in &e.abilities {
                if self.ability_index(a).is_none() {
                    bail!("enemy {}: no ability called {a}", e.key);
                }
            }
        }
        for a in &self.scaling.elite_abilities {
            if self.ability_index(a).is_none() {
                bail!("elite_abilities: no ability called {a}");
            }
        }
        for e in self.enemies.iter().chain(self.bosses.iter()) {
            for k in [&e.summon, &e.split_into, &e.phase_adds] {
                if !k.is_empty() && !self.enemies.iter().any(|x| &x.key == k) {
                    bail!("enemy {}: no enemy called {k} to raise or split into", e.key);
                }
            }
        }
        for a in &self.abilities {
            if !matches!(a.kind.as_str(), "slam" | "pool" | "line" | "ring") || a.cast_secs <= 0.0 || a.cooldown <= 0.0 {
                bail!("ability {} has a bad kind or numbers", a.key);
            }
            if a.warn.contains('~') {
                bail!("ability {}: no ~ in the warning", a.key);
            }
        }
        for c in &self.classes {
            if c.damage[0] > c.damage[1] || c.attack_secs <= 0.0 {
                bail!("class {} has bad numbers", c.key);
            }
            if !matches!(c.main_stat.as_str(), "strength" | "agility" | "intellect" | "spirit") {
                bail!("class {}: main_stat must be strength, agility, intellect or spirit", c.key);
            }
            if !self.gear.budget.armor.contains_key(&c.armor) {
                bail!("class {}: no armour value for {}", c.key, c.armor);
            }
            if !self.gear.sets.iter().any(|s| s.class == c.key) {
                bail!("class {} has no gear set", c.key);
            }
        }
        if self.enemy_index(&self.bases.guardian_elite).is_none() {
            bail!("guardian_elite {} isn't an enemy", self.bases.guardian_elite);
        }
        for s in &self.gear.sets {
            if s.slots.len() != 8 || s.displays.iter().any(|d| d.len() != 8) {
                bail!("gear set {}: 8 slots expected", s.class);
            }
            if s.displays.is_empty() || s.weapons.is_empty() || s.weapons.len() != s.weapon_names.len() {
                bail!("gear set {}: displays and weapons needed", s.class);
            }
        }
        let b = &self.gear.budget;
        if b.quality.len() < 5 || b.slot_weight.len() != 8 || b.required_level.is_empty() {
            bail!("gear budget is incomplete");
        }
        let mut keys = std::collections::HashSet::new();
        for card in &self.cards {
            if !keys.insert(card.key.as_str()) {
                bail!("two cards are called {}", card.key);
            }
            if card.is_ability() && card.cooldown <= 0.0 {
                bail!("card {} needs a cooldown", card.key);
            }
        }
        if self.cards.len() > 200 {
            bail!("at most 200 cards");
        }
        Ok(())
    }

    pub fn class(&self, key: &str) -> Option<&ClassDef> {
        self.classes.iter().find(|c| c.key == key)
    }

    pub fn class_index(&self, key: &str) -> Option<usize> {
        self.classes.iter().position(|c| c.key == key)
    }

    pub fn enemy_index(&self, key: &str) -> Option<usize> {
        self.enemies.iter().position(|e| e.key == key)
    }

    pub fn card_index(&self, key: &str) -> Option<usize> {
        self.cards.iter().position(|c| c.key == key)
    }

    pub fn set_for(&self, class: &str) -> &SetDef {
        self.gear.sets.iter().find(|s| s.class == class).expect("validated: every class has a set")
    }

    pub fn bonus_for(&self, class: &str) -> Option<&BonusDef> {
        self.gear.bonuses.iter().find(|b| b.class == class)
    }

    /// How many lanes wave `n` (1-based) uses.
    pub fn lanes_for_wave(&self, n: u32) -> usize {
        self.scaling
            .lanes_by_wave
            .iter()
            .filter(|[first, _]| *first <= n)
            .map(|[_, lanes]| *lanes as usize)
            .last()
            .unwrap_or(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_content_loads() {
        let c = Content::builtin();
        assert_eq!(c.classes.len(), 4);
        assert!(c.class("warrior").is_some());
        assert_eq!(c.lanes_for_wave(1), 2);
        assert_eq!(c.lanes_for_wave(5), 3);
        assert_eq!(c.lanes_for_wave(20), 4);
        assert!(c.cards.len() >= 32);
        for card in &c.cards {
            assert!(!card.describe(1).is_empty());
            assert!(!card.describe(5).is_empty());
        }
    }

    #[test]
    fn levels_come_a_wave_at_a_time() {
        let l = Content::builtin().levels;
        assert_eq!(l.target_level(0), 1);
        assert_eq!(l.target_level(1), 2);
        assert_eq!(l.target_level(20), 29);
        assert!(l.target_level(60) <= l.max_level);
        assert_eq!(l.xp_total(1), 0);
        assert_eq!(l.xp_total(2), l.xp_to_next(1) as u64);
    }

    #[test]
    fn scaling_grows() {
        let s = Content::builtin().scaling;
        assert!((s.health(1, 0) - s.health_base).abs() < 1e-6);
        assert!(s.health(9, 0) > s.health(8, 0) * 1.06);
        assert!(s.health(5, 1) > s.health(5, 0) * 1.3);
        assert!(s.raw_count(1, 4) > s.raw_count(1, 1));
    }
}
