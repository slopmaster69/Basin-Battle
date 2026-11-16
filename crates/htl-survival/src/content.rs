//! `survival.toml`: everything Elwynn Survival plays by.

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use serde::Deserialize;

pub use crate::content_abilities::*;

pub const DEFAULT: &str = include_str!("../../../content/survival.toml");
/// The Deadmines: the dungeon's own world (`[dungeon]`) and its enemies and gear, read beside
/// `survival.toml` (its lists join the camp's: the gear found below is good above too).
pub const DEFAULT_DUNGEON: &str = include_str!("../../../content/deadmines.toml");
/// The dungeons beside the Deadmines, each its own file: (file name, built-in text).
pub const MORE_DUNGEONS: &[(&str, &str)] = &[
    ("stockade.toml", include_str!("../../../content/stockade.toml")),
    ("ragefire.toml", include_str!("../../../content/ragefire.toml")),
    ("wailing.toml", include_str!("../../../content/wailing.toml")),
    ("shadowfang.toml", include_str!("../../../content/shadowfang.toml")),
    ("razorfen.toml", include_str!("../../../content/razorfen.toml")),
    ("graveyard.toml", include_str!("../../../content/graveyard.toml")),
    ("cathedral.toml", include_str!("../../../content/cathedral.toml")),
    ("library.toml", include_str!("../../../content/library.toml")),
];
/// Siege: matches in Arathi Basin (`[siege]`), read beside `survival.toml` as the Deadmines are.
pub const DEFAULT_SIEGE: &str = include_str!("../../../content/siege.toml");

#[derive(Debug, Clone, Deserialize)]
pub struct Content {
    pub world: WorldDef,
    pub player: PlayerDef,
    pub dodge: DodgeDef,
    pub parry: ParryDef,
    /// A caster's guard: the ward a staff raises, with a bar of its own (`[ward]`).
    #[serde(default)]
    pub ward: WardDef,
    /// The skill checks built into spells: the window a perfect must land in, and what a perfect
    /// pays (`[checks]`).
    #[serde(default)]
    pub checks: ChecksDef,
    /// What a casting staff's bolts build and its heavy bolt spends (`[staffs]`).
    #[serde(default)]
    pub staffs: StaffsDef,
    #[serde(default)]
    pub heavy: HeavyDef,
    #[serde(default)]
    pub lock: LockDef,
    /// How creatures fight a band of heroes: whom they go for, when they press and when they
    /// give ground (`[ai]`).
    #[serde(default)]
    pub ai: AiDef,
    /// How each kind of creature fights (`[[temper]]`): a creature's `temper` names one.
    #[serde(rename = "temper", default)]
    pub tempers: Vec<TemperDef>,
    /// How a fight of two or more goes (`[[plan]]`): one is picked as it begins, from those that
    /// fit who's in it.
    #[serde(rename = "plan", default)]
    pub plans: Vec<PlanDef>,
    #[serde(default)]
    pub pvp: PvpDef,
    /// Two heroes' duel: a challenge, a countdown, and a fight until one would fall.
    #[serde(default)]
    pub duel: DuelDef,
    /// How strikes chain, cancel and lunge, and how far the server makes allowances for lag.
    #[serde(default)]
    pub combat: CombatDef,
    /// Each grip's blows: the light chain, the heavy attack, the running attack.
    #[serde(rename = "moveset", default = "default_movesets")]
    pub movesets: Vec<MovesetDef>,
    /// The shield bash (an attack with the guard up) and the kick (a heavy attack with it up, or
    /// any with no shield): guard breakers.
    #[serde(default = "default_bash")]
    pub bash: ShoveDef,
    #[serde(default = "default_kick")]
    pub kick: ShoveDef,
    #[serde(rename = "ranged")]
    pub ranged: Vec<RangedDef>,
    /// How chosen spells' missiles look on a client that draws them (patch 0030): a glowing
    /// ribbon behind them, a size, a flat spin.
    #[serde(rename = "missile_look", default)]
    pub missile_looks: Vec<MissileLookDef>,
    #[serde(rename = "weapon")]
    pub weapons: Vec<WeaponDef>,
    #[serde(rename = "shield")]
    pub shields: Vec<ShieldDef>,
    #[serde(rename = "item")]
    pub items: Vec<ItemDef>,
    pub start: StartDef,
    #[serde(rename = "station")]
    pub stations: Vec<StationDef>,
    #[serde(rename = "recipe")]
    pub recipes: Vec<RecipeDef>,
    pub nodes: Vec<NodeDef>,
    #[serde(rename = "mob")]
    pub mobs: Vec<MobDef>,
    #[serde(rename = "pack")]
    pub packs: Vec<PackDef>,
    /// Groups spread over the whole land at a world's start (wolves in the woods, bandits off the
    /// roads), each becoming a pack where it stands.
    #[serde(rename = "roam", default)]
    pub roams: Vec<RoamDef>,
    /// The camp's journal: goals in order, each rewarded into the chest.
    #[serde(rename = "goal", default)]
    pub goals: Vec<GoalDef>,
    /// The Horde's camps, which heroes can take for a while by killing everyone in them.
    #[serde(rename = "camp", default)]
    pub camps: Vec<CampDef>,
    /// Sets of gear (`[[set]]`): pieces worn together give more.
    #[serde(rename = "set", default)]
    pub sets: Vec<SetDef>,
    /// Elwynn's places: found by walking there, marked on the world map.
    #[serde(rename = "site", default)]
    pub sites: Vec<SiteDef>,
    /// Chests, caches, relics and shrines out in the land: some in plain sight, some hidden.
    #[serde(rename = "chest", default)]
    pub chests: Vec<ChestDef>,
    /// Things only to be seen, that make a camp look lived in: fires, braziers, cages, carts.
    #[serde(rename = "prop", default)]
    pub props: Vec<PropDef>,
    pub raid: RaidDef,
    /// The Deadmines, a world of its own reached by a book at camp (below, the world's own dungeon).
    #[serde(default)]
    pub dungeon: Option<DungeonDef>,
    /// The other dungeons, each from a file of its own beside `deadmines.toml` (its `[dungeon]`),
    /// each a world of its own and a page of the same book.
    #[serde(rename = "more_dungeon", default)]
    pub more_dungeons: Vec<DungeonDef>,
    /// In a dungeon's own world, the book's other dungeons: not this world's, but on its ladder
    /// (what the run's page says is next).
    #[serde(skip)]
    pub elsewhere: Vec<DungeonDef>,
    /// The Proving Ring: endless waves, a page of the same book.
    #[serde(default)]
    pub arena: Option<ArenaDef>,
    /// Siege: camps against camps in Arathi Basin, a page of the same book.
    #[serde(default)]
    pub siege: Option<std::sync::Arc<SiegeDef>>,
    /// The abilities on the ring tree, their EX versions and the counters.
    #[serde(rename = "ability", default)]
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub valor: ValorDef,
    #[serde(default)]
    pub cc: CcDef,
    /// The Juggernaut's Momentum: its numbers.
    #[serde(default)]
    pub momentum: MomentumDef,
    /// What a hit in a creature's back looks like, whoever lands it.
    #[serde(default)]
    pub backstab: BackstabDef,
    /// A hero created as a Rogue: two blades and no guard, and what a perfect dodge pays
    /// (docs/rogue-roles.md, "The core").
    #[serde(default)]
    pub rogue: RogueDef,
    /// How the ice shatters off a frozen foe: the same for every class.
    #[serde(default)]
    pub shatter: ShatterDef,
    #[serde(default)]
    pub dev: DevDef,
    /// How long the work at the camp takes: making something at a station, building one, mending.
    #[serde(default)]
    pub work: WorkDef,
    /// A fresh hero's ability keys: slot to ability.
    #[serde(default)]
    pub loadout: LoadoutDef,
    /// The Priest's staffs' marks and stacks, and the burst of a Priest's perfect (`[priest]`).
    #[serde(default)]
    pub priest: PriestDef,
    /// Elwynn's weather above ground (`[weather]`): how often each kind comes and how it eases.
    #[serde(default)]
    pub weather: crate::weather::WeatherDef,
    /// The text it was read from, for tuning in play ([`crate::tune`]), and the file, if any.
    #[serde(skip)]
    pub source: String,
    /// `deadmines.toml`'s text, read beside it.
    #[serde(skip)]
    pub dungeon_source: String,
    /// The other dungeons' files (name and text), read beside it.
    #[serde(skip)]
    pub more_sources: Vec<(String, String)>,
    #[serde(skip)]
    pub path: Option<std::path::PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WorldDef {
    pub spawn: [f32; 2],
    pub spawn_facing: f32,
    pub heart: [f32; 2],
    pub rest_radius: f32,
    pub alarm: [f32; 2],
    pub build_center: [f32; 2],
    pub build_radius: f32,
    /// What rings the building ground, to see it by (`GameObjectDisplayInfo`; 0: nothing),
    /// one about every `build_marker_every` yards round its edge.
    #[serde(default)]
    pub build_marker: u32,
    #[serde(default = "fourteen")]
    pub build_marker_every: f32,
    pub keep_clear: f32,
    /// The weapon rack: every weapon to take up and try, free while `armory` is on.
    pub rack: [f32; 2],
    #[serde(default)]
    pub armory: bool,
    /// The game clock pinned at this hour and minute (the sky, the light and the fog follow it):
    /// set again every `clock_every` seconds, so it never runs on to another part of the day.
    /// None: the clock runs on from the time it is now.
    #[serde(default)]
    pub clock: Option<[u32; 2]>,
    #[serde(default = "clock_every")]
    pub clock_every: f32,
    /// How fast the game clock runs: 1, a minute a minute, the pin holding the hour; 120, a whole
    /// day in 12 minutes, for watching every hour of the look in one sitting (`.es tune
    /// world.clock_speed 120`). Over 1 the clock starts at `clock` (or the time it is now) and
    /// everyone is set to where it has got to every `clock_every` seconds, so a late hero sees the
    /// same hour; a change of speed or of `clock` starts it again. The client holds a rate from 1
    /// through 3600. Real time below and in Siege.
    #[serde(default = "one_w")]
    pub clock_speed: f32,
    /// The camp's own things' looks (`GameObjectDisplayInfo`) and sizes: the chest, the bell
    /// (raised `alarm_lift` over the ground), the weapon rack, and what the Logbook lies on (its
    /// top `book_stand_height` over the ground).
    #[serde(default = "storage_display")]
    pub storage_display: u32,
    #[serde(default = "one_w")]
    pub storage_scale: f32,
    #[serde(default = "alarm_display")]
    pub alarm_display: u32,
    #[serde(default = "alarm_scale")]
    pub alarm_scale: f32,
    #[serde(default)]
    pub alarm_lift: f32,
    #[serde(default = "rack_display")]
    pub rack_display: u32,
    #[serde(default = "one_w")]
    pub rack_scale: f32,
    #[serde(default = "book_stand_display")]
    pub book_stand_display: u32,
    #[serde(default = "one_w")]
    pub book_stand_scale: f32,
    #[serde(default = "book_stand_height")]
    pub book_stand_height: f32,
    /// The night's fog over the land (the add-on asks the client for it; a stock client keeps its
    /// own): its colour (0 to 1), how far it ends (yards), where it starts (a share of that), how
    /// much of it is laid over Elwynn's own (0 to 1), and how much darker the light (0 to 1).
    #[serde(default)]
    pub fog: Option<FogDef>,
    /// Thicker while a raid is called and under way.
    #[serde(default)]
    pub raid_fog: Option<FogDef>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq)]
pub struct FogDef {
    pub color: [f32; 3],
    pub end: f32,
    pub start: f32,
    #[serde(default = "one_w")]
    pub weight: f32,
    #[serde(default)]
    pub darken: f32,
    /// Seconds to ease into it.
    #[serde(default = "fog_secs")]
    pub secs: f32,
}

fn fog_secs() -> f32 {
    6.0
}

fn one_w() -> f32 {
    1.0
}

fn storage_display() -> u32 {
    10
}

fn alarm_display() -> u32 {
    4571
}

fn alarm_scale() -> f32 {
    0.6
}

fn rack_display() -> u32 {
    130
}

fn book_stand_display() -> u32 {
    18
}

fn book_stand_height() -> f32 {
    0.5
}

fn clock_every() -> f32 {
    300.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct PlayerDef {
    pub health: u32,
    pub stamina: f32,
    pub stamina_regen: f32,
    pub stamina_delay: f32,
    pub exhausted_below: f32,
    pub run_speed: f32,
    /// Sprinting (Shift held): this fast, draining this much stamina a second.
    pub sprint_speed: f32,
    pub sprint_drain: f32,
    #[serde(default)]
    pub sprint_restart: f32,
    /// What a jump costs (and the least it takes to make one).
    #[serde(default)]
    pub jump_stamina: f32,
    /// How fast the hero turns (radians a second) with the turn keys.
    #[serde(default = "turn_rate")]
    pub turn_rate: f32,
    pub attack_speed: f32,
    pub guard_speed: f32,
    pub aim_speed: f32,
    pub use_speed: f32,
    /// The clips a hero plays using an item marked `anim = "eat"` or `"drink"` (0: the Food
    /// spell's cast instead).
    #[serde(default)]
    pub eat_clip: u16,
    #[serde(default)]
    pub drink_clip: u16,
    pub field_regen: f32,
    pub field_regen_after: f32,
    pub rest_regen: f32,
    pub respawn_secs: f32,
    pub respawn_health: f32,
    pub buffer: f32,
    pub combat_secs: f32,
    pub bag_slots: usize,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DodgeDef {
    pub stamina: f32,
    /// The least stamina a roll starts with.
    #[serde(default)]
    pub min_stamina: f32,
    pub secs: f32,
    pub evade_from: f32,
    pub evade_to: f32,
    /// The dash with a direction held: the hero's run (and run-back) speed for `dash_secs`, so the
    /// client walks it along the ground like any step, slopes and all. (It used to be a
    /// knockback: a throw that fast is a flight, and a flight into a tree's roots can come to rest
    /// up in them.)
    pub speed: f32,
    #[serde(default = "dash_secs")]
    pub dash_secs: f32,
    /// The step back with no direction held: a short hop (speed, lift), a jump's worth.
    #[serde(default = "back_step")]
    pub back_step: [f32; 2],
    /// A spell whose cast look (a whoosh and a dust trail, no animation) marks the dash; 0 for none.
    #[serde(default)]
    pub spell: u32,
    /// A roll through a blow gives back this much stamina.
    #[serde(default)]
    pub refund: f32,
    /// A Rogue's perfect dodge: the blow lands from `evade_from` to this long after the dodge
    /// starts (0: no perfect dodge). It gives back `perfect_refund` stamina in place of `refund`,
    /// and fills `perfect_posture` of a boss's or champion's posture (docs/rogue-roles.md).
    #[serde(default)]
    pub perfect_to: f32,
    #[serde(default)]
    pub perfect_refund: f32,
    #[serde(default)]
    pub perfect_posture: f32,
}

impl DodgeDef {
    /// What a roll needs in hand to start.
    pub fn needs(&self) -> f32 {
        self.stamina.max(self.min_stamina)
    }
}

fn dash_secs() -> f32 {
    0.25
}

fn back_step() -> [f32; 2] {
    [8.0, 3.0]
}

/// A guard raised just before a blow lands turns it aside.
#[derive(Debug, Clone, Deserialize)]
pub struct ParryDef {
    /// How soon before the blow the guard must have gone up.
    pub window: f32,
    /// The guard must have been down this long before (no parrying by twitching it).
    pub cooldown: f32,
    /// The attacker reels this long...
    pub stagger: f32,
    /// ...taking this much more from every blow meanwhile.
    pub riposte: f32,
    /// A hero whose blow is parried reels this long.
    pub hero_stagger: f32,
    /// A parry gives back this much stamina...
    #[serde(default)]
    pub stamina: f32,
    /// ...and opens this long a window in which everything costs half the stamina.
    #[serde(default)]
    pub opening: f32,
    /// A champion with a posture isn't staggered by a parry: the parry fills this share of its
    /// posture instead (so parries break a boss's stance, they don't lock it).
    #[serde(default)]
    pub posture: f32,
}

/// The ice shattering off a frozen foe (a shattering ability on it): one look for every class,
/// whoever's ability it was (Corey: "SHATTER is a global effect not a class specific effect").
/// Besides the frost burst (kit 450), the sound and the beat, a short pale radial burst on the
/// foe, action lines round the shatter (Corey: "small bursty action lines on the target").
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ShatterDef {
    /// The action lines' look (`SpellVisualKit`), on the shattered foe with the frost burst; 0
    /// for none. Picks by model name, none seen on video yet (`.es tune shatter.burst_kit <id>`
    /// to try another): 121 `Holy_ImpactDD_Low_Chest` (Vindication's impact, a small white-gold
    /// flash with radial spikes at the chest); 487 `Exorcism_Impact_Chest` (a bigger holy burst,
    /// yellow-white rays); 3381 `starfire_area` (Starfire's white-blue starburst, the biggest,
    /// drawn at the foe's base); 283 `LightningBolt_Impact_Chest` (a white-blue electric flash).
    pub burst_kit: u32,
}

impl Default for ShatterDef {
    fn default() -> Self {
        Self { burst_kit: 121 }
    }
}

/// The ward: what a staff guards with. An arcane barrier with a bar of its own in place of
/// stamina. Every blow it stops drains the bar (the guard's `cost`s); a guard raised in the parry
/// window is a perfect ward, which costs nothing and reels the attacker as a parry does; a blow
/// too heavy for what's left breaks it, and a broken ward can't go up again until the bar is full.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WardDef {
    /// The bar.
    pub bar: f32,
    /// How fast it refills, a second, while the ward is down...
    pub refill: f32,
    /// ...starting this long after it came down (or broke).
    pub refill_delay: f32,
    /// A perfect ward gives this much of the bar back.
    pub perfect_refill: f32,
    /// A broken ward's stagger.
    pub break_secs: f32,
    /// The look on a hero holding the ward up (a spell whose aura the clients show).
    pub aura: u32,
    /// The crackle on the ward as it stops a blow, the burst of a perfect ward, and the fizzle of
    /// one that breaks (spell visual kits).
    pub block_kit: u32,
    pub perfect_kit: u32,
    pub break_kit: u32,
    /// While up, `block_kit` crackles on the hero every `crackle_full` sec with the bar whole,
    /// down to every `crackle_empty` sec as it runs dry (0: never).
    pub crackle_full: f32,
    pub crackle_empty: f32,
    /// A spell whose precast plays in the hands for as long as the ward is up (a held glow in the
    /// raised hand, where the aura wrapped the chest and hid the arm); 0 for none.
    pub hold_spell: u32,
    /// The arm raised while it's up: this clip on the upper body (the legs free), held at its
    /// peak until the ward comes down (`clip_take` of it, from `clip_from`'s model if given; 0:
    /// the guard's own pose alone).
    pub clip: u16,
    pub clip_take: u32,
    pub clip_from: String,
    /// How long each play of it holds (renewed a second before it ends), and on which body:
    /// `upper` (the legs free) or `full`.
    pub clip_hold: f32,
    pub clip_body: String,
    /// While broken (until the bar is whole again), this spell's aura on the warder: a debuff's
    /// look, so a broken ward reads as a weakness, not a cleanse (0: none).
    pub broken_aura: u32,
}

impl Default for WardDef {
    fn default() -> Self {
        Self { bar: 100.0, refill: 35.0, refill_delay: 0.4, perfect_refill: 20.0, break_secs: 1.0, aura: 1463, block_kit: 0, perfect_kit: 555, break_kit: 7, crackle_full: 0.0, crackle_empty: 0.0, hold_spell: 0, clip: 0, clip_take: 0, clip_from: String::new(), clip_hold: 8.0, clip_body: String::new(), broken_aura: 0 }
    }
}

/// What a casting staff's plain bolts build and its heavy bolt spends: the spells keep the skill
/// checks, the staffs build and spend (Corey's playtest, Nov 5: staff attacks should feel apart
/// from their school's spells). An Ember staff heats the foe it hits and the last heat ignites; a
/// Rime staff chills it, slower with each, and the last chill freezes it; an Arcane staff gathers
/// charges at the caster's staff. The heavy bolt spends them, harder for each.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct StaffsDef {
    /// The most of any kind: the bolt that reaches it ignites or freezes (heat, chill).
    pub most: u32,
    /// How long heat or chill stays on a foe after the last bolt, and charges on the caster.
    pub keep: f32,
    /// A heavy bolt is this much harder for each stack it spends.
    pub spend_share: f32,
    /// Heat's ignite: this much damage round the foe, within `ignite_radius`, with the bolt's
    /// burn, and this spell's look there (Blast Wave's burst).
    pub ignite_damage: f32,
    pub ignite_radius: f32,
    pub ignite_spell: u32,
    /// Chill: each stack slows the foe this much more; the last freezes it this long.
    pub chill_slow: f32,
    pub freeze_secs: f32,
    /// Charges: the flash at the caster's hands as the last gathers (a spell visual kit), and a
    /// heavy bolt spending them all throws the foe back this far.
    pub charged_kit: u32,
    pub charged_knockback: f32,
    /// An Arcane staff's heavy orb carries the charges stored in the staff (Corey, Nov 5): its
    /// strength, a share of its full damage by how many (none: nothing goes, a fizzle on the
    /// caster), and its look by how many (bigger with each); with them all, `charge_full_kit`
    /// bursts on what it strikes.
    pub charge_scale: [f32; 4],
    pub charge_spells: [u32; 4],
    pub charge_full_kit: u32,
    /// The burst on a foe the last chill freezes, and on one each chill takes (visual kits; 0:
    /// none), so the ice reads (Corey: hard to see).
    pub freeze_kit: u32,
    pub chill_kit: u32,
}

impl Default for StaffsDef {
    fn default() -> Self {
        Self { most: 3, keep: 5.0, spend_share: 0.4, ignite_damage: 18.0, ignite_radius: 2.5, ignite_spell: 11113, chill_slow: 0.15, freeze_secs: 1.5, charged_kit: 3394, charged_knockback: 3.0, charge_scale: [0.0, 0.3, 0.5, 1.0], charge_spells: [0; 4], charge_full_kit: 0, freeze_kit: 0, chill_kit: 0 }
    }
}

/// The skill checks built into spells. The first kind, the release: a charged spell with
/// `perfect = true` flares as its charge tops out, and a release within `window` of the top is a
/// perfect cast: full strength and `perfect_damage` times more, `perfect_valor` to the caster,
/// and a burst on them everyone sees. Let go earlier it goes at its charge, as ever; held past
/// the window it's full and no more. A missed check is still a working spell.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ChecksDef {
    /// Half the window, seconds either side of the moment.
    pub window: f32,
    /// A perfect's damage, times the full charge's.
    pub perfect_damage: f32,
    /// The Valor a perfect gives (a parry's).
    pub perfect_valor: f32,
    /// The burst on the caster at a perfect release (a spell visual kit).
    pub perfect_kit: u32,
    /// A full rhythm chain's Valor (a whole segment).
    pub chain_valor: f32,
    /// The flare (the release check's moment) comes, at random, anywhere from this share of a
    /// ranged charge's top to the top; a fixed charge's up to `flare_late` seconds
    /// after it. No two casts keep quite the same time.
    pub flare_from: f32,
    pub flare_late: f32,
    /// The critical: the middle this share of any check's window.
    pub crit_share: f32,
    /// The look of an all-or-nothing cast that fizzles (a spell visual kit on the caster).
    pub fizzle_kit: u32,
}

impl Default for ChecksDef {
    fn default() -> Self {
        Self { window: 0.1, perfect_damage: 1.2, perfect_valor: 10.0, perfect_kit: 450, chain_valor: 25.0, flare_from: 0.72, flare_late: 0.35, crit_share: 0.3, fizzle_kit: 0 }
    }
}

/// The heavy attack (Shift with the left mouse): slower to come, harder to take.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct HeavyDef {
    /// The weapon's wind-up and recovery are this much longer...
    pub windup: f32,
    pub recovery: f32,
    /// ...its damage and stamina this much more.
    pub damage: f32,
    pub stamina: f32,
    /// It knocks any wind-up aside for at least this long (a heavy weapon's own, if longer).
    pub stagger: f32,
    /// A hero it lands on is thrown back: this fast, this high.
    pub knockback: [f32; 2],
}

impl Default for HeavyDef {
    fn default() -> Self {
        Self { windup: 1.55, recovery: 1.2, damage: 1.7, stamina: 1.6, stagger: 0.6, knockback: [9.0, 2.5] }
    }
}

/// How strikes flow into each other, and the server's allowances for lag.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CombatDef {
    /// An attack pressed this far into a light blow's recovery (a share of it) chains into the
    /// next blow of the set at once.
    pub chain_after: f32,
    /// A dodge or the guard this far into a blow's recovery cuts the rest of it short (a heavy
    /// attack's, the second).
    pub cancel_after: f32,
    pub heavy_cancel_after: f32,
    /// The chain carries on if the next attack comes within this long of a blow's end.
    pub combo_grace: f32,
    /// A lunge runs over this long, ending as the blow lands.
    pub lunge_secs: f32,
    /// A swing judges where the others were when its striker saw them: up to this far back.
    pub max_rewind: f32,
    /// A blow on a hero waits up to this long (their round trip) for a dodge or a guard they
    /// had already pressed when they saw it coming.
    pub max_grace: f32,
    /// Leaning back from a blow (radians): a light one's, a heavy one's, a creature's.
    pub lean_light: f32,
    pub lean_heavy: f32,
    /// A heavy attack's poise: from its start until it lands, a light blow neither knocks it
    /// aside nor throws the attacker back (a heavy blow, a bash or a kick still do).
    pub heavy_poise: bool,
    /// Knocked down (a guard broken by a heavy blow or a kick, a heavy creature's blow landing):
    /// on the ground this long, open to a riposte.
    pub knockdown: f32,
    /// A riposte (a blow on a body reeling from a stagger of yours or lying knocked down) does
    /// the parry's riposte damage too, not only after a parry.
    pub riposte_any_stagger: bool,
    /// Stamina doesn't come back during a blow or a dodge: its pause starts as they end.
    pub stamina_waits: bool,
    /// A blow on a creature from behind (more than 120 degrees off its facing) does this many
    /// times its damage, with Backstab's flash.
    pub backstab: f32,
    /// Every foe's health is this many times its own `health` (critters, dummies and totems
    /// as they are): the whole bestiary's toughness tuned at once.
    pub foe_health: f32,
    /// Every foe's blows (and burns) on a hero do this many times their own damage.
    pub foe_damage: f32,
    /// For each hero in a foe's fight past the first (near it, or on it from farther off), it
    /// has this share more health and its blows this share more damage (below, every hero in
    /// the mines counts; the arena sends more foes instead).
    pub hero_health: f32,
    pub hero_damage: f32,
    /// A heavy blow knocks a creature's wind-up aside, whatever the weapon. Off, only a weapon
    /// with its own `stagger` (a mace, a maul, an axe) does it, light or heavy; any other blow,
    /// a wind-up goes through.
    pub heavy_knocks_aside: bool,
    /// A creature whose wind-up was knocked aside isn't again for this long (its blows come
    /// through the next ones).
    pub knock_aside_wary: f32,
    /// Blows knock a boss's wind-up aside too.
    pub boss_knock_aside: bool,
    /// A heavy blow on a creature just after its own blow (in its recovery, not reeling already)
    /// staggers it for the heavy's stagger: the window to strike in. Not a boss's.
    pub recovery_stagger: bool,
    /// Blows in a creature's recovery count this many times towards breaking its posture.
    pub recovery_posture: f32,
    /// Every boss is drawn (and struck) this much larger than its own scale: bigger than life,
    /// to loom, and so its blows read from farther off.
    pub boss_scale: f32,
    /// A bash or a kick knocks a boss's blow aside, not only its cast.
    pub shove_bosses: bool,
}

impl Default for CombatDef {
    fn default() -> Self {
        Self {
            chain_after: 0.25,
            cancel_after: 0.45,
            heavy_cancel_after: 0.65,
            combo_grace: 0.6,
            lunge_secs: 0.18,
            max_rewind: 0.25,
            max_grace: 0.2,
            lean_light: 0.22,
            lean_heavy: 0.45,
            heavy_poise: true,
            knockdown: 1.5,
            riposte_any_stagger: true,
            stamina_waits: true,
            backstab: 1.5,
            foe_health: 1.0,
            foe_damage: 1.0,
            hero_health: 0.0,
            hero_damage: 0.0,
            heavy_knocks_aside: true,
            knock_aside_wary: 0.0,
            boss_knock_aside: true,
            recovery_stagger: false,
            recovery_posture: 1.0,
            boss_scale: 1.0,
            shove_bosses: true,
        }
    }
}

/// One blow of a set: the clip and which take of it (`AnimationData` id; 0 the first), its wind-up
/// and recovery as shares of the weapon's own, its damage and stamina shares, how far it lunges
/// (yards) and how far its reach goes (a share).
#[derive(Debug, Clone, Deserialize)]
pub struct MoveDef {
    pub clip: u16,
    #[serde(default)]
    pub take: u32,
    #[serde(default = "one_share")]
    pub windup: f32,
    #[serde(default = "one_share")]
    pub recovery: f32,
    #[serde(default = "one_share")]
    pub damage: f32,
    #[serde(default = "one_share")]
    pub stamina: f32,
    #[serde(default)]
    pub lunge: f32,
    #[serde(default = "one_share")]
    pub range: f32,
    /// Another character model's take on the clip (`"ScourgeFemale"`, `"OrcMale"`…), played on
    /// the hero's own body by a client that can (`ac10`); without one, the hero's own.
    #[serde(default)]
    pub from: Option<String>,
}

fn one_share() -> f32 {
    1.0
}

impl MoveDef {
    fn of(clip: u16, take: u32, windup: f32, recovery: f32, damage: f32, lunge: f32) -> Self {
        Self { clip, take, windup, recovery, damage, stamina: 1.0, lunge, range: 1.0, from: None }
    }
}

/// The character models a clip can be borrowed from, as `<Race><Gender>`.
pub const CLIP_RACES: [&str; 9] = ["Human", "Orc", "Dwarf", "NightElf", "Scourge", "Tauren", "Gnome", "Troll", "Goblin"];

/// Whether `name` is a character model a clip can come from (`"ScourgeFemale"`), or a model's path.
pub fn clip_source_ok(name: &str) -> bool {
    if name.contains('\\') || name.to_ascii_lowercase().ends_with(".m2") {
        return true;
    }
    let lower = name.to_ascii_lowercase();
    let race = lower.strip_suffix("female").or_else(|| lower.strip_suffix("male"));
    race.is_some_and(|r| CLIP_RACES.iter().any(|x| x.eq_ignore_ascii_case(r)))
}

/// A grip's blows (`one_hand`, `dagger`, `two_hand`, `polearm`).
#[derive(Debug, Clone, Deserialize)]
pub struct MovesetDef {
    pub key: String,
    /// The light chain, in order; it starts over after the last.
    pub light: Vec<MoveDef>,
    pub heavy: MoveDef,
    /// An attack while sprinting.
    pub running: MoveDef,
    /// An attack out of a dodge: pressed during the roll, it comes as the roll ends (without one,
    /// the running attack).
    #[serde(default)]
    pub roll: Option<MoveDef>,
    /// An attack out of a Corey roll (a jump taken in the dodge's dash, which carries the dash's
    /// speed through the air): pressed in the air, it strikes as the hero lands (without one, the
    /// attack out of a dodge).
    #[serde(default)]
    pub leap: Option<MoveDef>,
}

impl MovesetDef {
    /// The attack out of a dodge.
    pub fn roll_move(&self) -> &MoveDef {
        self.roll.as_ref().unwrap_or(&self.running)
    }

    /// The attack out of a Corey roll, on landing.
    pub fn leap_move(&self) -> &MoveDef {
        self.leap.as_ref().unwrap_or_else(|| self.roll_move())
    }
}

fn default_movesets() -> Vec<MovesetDef> {
    let m = MoveDef::of;
    vec![
        MovesetDef {
            key: "one_hand".into(),
            light: vec![m(17, 0, 1.0, 1.0, 1.0, 0.7), m(17, 1, 1.0, 1.05, 1.0, 0.7), m(57, 0, 1.2, 1.25, 1.35, 1.0)],
            heavy: m(57, 0, 1.0, 1.0, 1.0, 1.0),
            running: m(17, 1, 1.0, 1.1, 1.2, 2.4),
            roll: None,
            leap: None,
        },
        MovesetDef {
            key: "dagger".into(),
            light: vec![m(85, 0, 1.0, 1.0, 1.0, 0.6), m(88, 0, 1.0, 1.0, 1.0, 0.6), m(85, 0, 0.9, 1.0, 1.0, 0.6), m(57, 0, 1.2, 1.2, 1.5, 1.0)],
            heavy: m(57, 0, 1.0, 1.0, 1.0, 1.0),
            running: m(85, 0, 1.0, 1.1, 1.3, 2.4),
            roll: None,
            leap: None,
        },
        MovesetDef {
            key: "two_hand".into(),
            light: vec![m(18, 0, 1.0, 1.0, 1.0, 0.8), m(18, 1, 1.0, 1.05, 1.05, 0.8)],
            heavy: m(58, 0, 1.0, 1.0, 1.0, 1.2),
            running: m(18, 1, 1.0, 1.1, 1.2, 2.6),
            roll: None,
            leap: None,
        },
        MovesetDef {
            key: "polearm".into(),
            light: vec![m(19, 0, 1.0, 1.0, 1.0, 0.8), m(19, 1, 1.0, 1.05, 1.05, 0.8), m(19, 2, 1.1, 1.1, 1.15, 0.9)],
            heavy: m(58, 0, 1.0, 1.0, 1.0, 1.2),
            running: m(19, 0, 1.0, 1.1, 1.2, 2.8),
            roll: None,
            leap: None,
        },
    ]
}

/// A guard breaker: its clip, timing, cost, reach, what it does to whoever it reaches (damage,
/// a stagger, stamina torn from a raised guard) and the shove on a hero (speed, lift).
#[derive(Debug, Clone, Deserialize)]
pub struct ShoveDef {
    pub clip: u16,
    pub windup: f32,
    pub active: f32,
    pub recovery: f32,
    pub stamina: f32,
    pub damage: [u32; 2],
    pub range: f32,
    pub arc: f32,
    pub stagger: f32,
    pub guard_break: f32,
    pub push: [f32; 2],
    #[serde(default)]
    pub lunge: f32,
}

fn default_bash() -> ShoveDef {
    ShoveDef {
        clip: 59,
        windup: 0.2,
        active: 0.1,
        recovery: 0.4,
        stamina: 16.0,
        damage: [4, 7],
        range: 2.5,
        arc: 100.0,
        stagger: 0.7,
        guard_break: 45.0,
        push: [6.0, 1.5],
        lunge: 0.6,
    }
}

fn default_kick() -> ShoveDef {
    ShoveDef {
        clip: 95,
        windup: 0.22,
        active: 0.1,
        recovery: 0.45,
        stamina: 12.0,
        damage: [0, 0],
        range: 2.4,
        arc: 80.0,
        stagger: 0.45,
        guard_break: 60.0,
        push: [8.0, 2.0],
        lunge: 0.4,
    }
}

/// A temperament: how a kind of creature fights, over `[ai]` (Corey, Nov 7: "not all enemies
/// need to behave the exact same way"). What it leaves out is `[ai]`'s. The fairness rules hold
/// for every one: nothing at a reeling hero, no blade with a bolt in the air, no two wind-ups at
/// once.
#[derive(Debug, Clone, Deserialize, Default)]
pub struct TemperDef {
    pub key: String,
    /// It comes at an opening (a swing's follow-through, a cast, a meal), its turn or not.
    #[serde(default)]
    pub punish: Option<bool>,
    /// It takes turns with the others on its hero (and their breath after a string).
    #[serde(default)]
    pub waits_turn: Option<bool>,
    /// Waiting, it works round toward its hero's back.
    #[serde(default)]
    pub flank: Option<bool>,
    /// It strikes only from behind its hero (or at an opening); otherwise it circles round.
    #[serde(default)]
    pub backstab: bool,
    /// After a string, the chance it gives ground, and how far (a boss too, when set here).
    #[serde(default)]
    pub retreat: Option<f32>,
    #[serde(default)]
    pub retreat_yards: Option<f32>,
    /// A light blow doesn't knock its wind-up aside (a heavy one or a shove still does).
    #[serde(default)]
    pub poise: bool,
    /// Below this share of its health it runs from its hero, once, before it comes back.
    #[serde(default)]
    pub flee_below: f32,
    /// It holds back, circling at a distance, while others of its fight are on its hero.
    #[serde(default)]
    pub hold_back: bool,
    /// Its cooldown between strings, times this (under 1: it presses harder).
    #[serde(default = "one_f")]
    pub pace: f32,
    /// It answers every opening at once: a swing's follow-through (or a cast, a meal) draws its
    /// blow whatever its cooldown still has to run, within `PRESS_RANGE`. (Without it, only when the
    /// cooldown has nearly run out.)
    #[serde(default)]
    pub answer: bool,
}

/// A fight's plan (`[[plan]]`): picked as a fight of two creatures or more begins, so the same
/// pack doesn't fight the same way twice. `key` is one the rules know: `all_in` (every fighter
/// presses), `pincer` (one holds the hero's front, another works round to their back), `screen`
/// (a fighter stands between the hero and the fight's caster or shooter), `bait` (one fights,
/// another holds back for an opening or its fellow's fall), `waves` (half the fighters wait their
/// turn out of reach).
#[derive(Debug, Clone, Deserialize)]
pub struct PlanDef {
    pub key: String,
    /// How likely, against the others that fit.
    #[serde(default = "one_f")]
    pub weight: f32,
    /// Fighters up close it needs (not casters or shooters).
    #[serde(default)]
    pub min_melee: usize,
    /// It needs a caster or a shooter in the fight.
    #[serde(default)]
    pub ranged: bool,
}

/// A creature's tactics, as Yharnam's hunters learned them the hard way: it goes for whoever
/// hurts it, the one alone and the one with their guard down; it presses an opening (a heal, a
/// swing's follow-through, a spell drawn back, a stagger) at once, past the turn it would wait;
/// against a raised guard it reaches for the blow no guard stops; between strings it now and
/// then gives ground and makes you come to it; waiting its turn it works round to your back; and
/// its blows don't come on a metronome.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct AiDef {
    /// Seconds between a creature's looks round for whom to fight (mid-string it keeps its foe).
    pub retarget_secs: f32,
    /// Whom it goes for: the damage a hero has done it lately (as a share of its health) times
    /// `threat`, `opening` more for a hero with their guard down (healing, working, mid-swing,
    /// drawing a spell, reeling), `wounded` times how hurt they are; less `distance` for every
    /// 10 yards off and `crowd` for every other creature already on them; its
    /// foe now counts `stick` more, so it doesn't turn every second.
    pub threat: f32,
    pub opening: f32,
    pub wounded: f32,
    pub distance: f32,
    pub crowd: f32,
    pub stick: f32,
    /// Its memory of who hurt it halves this often (seconds).
    pub threat_half: f32,
    /// An opening it's near: it comes this many seconds sooner than its cooldown says, past
    /// the two-at-a-time turn.
    pub press_early: f32,
    /// Against a raised guard, a string with a blow no guard stops (or a heavy one) is this many
    /// times as likely.
    pub guard_break: f32,
    /// The chance it gives ground after a string (not a boss, nor one that shoots), and how far.
    pub retreat: f32,
    pub retreat_yards: f32,
    /// Waiting its turn, it works round toward the hero's back.
    pub flank: bool,
    /// Its cooldown between strings, give or take this share.
    pub jitter: f32,
    /// Every creature's leash (how far from home it chases) times this: one knob for how far
    /// foes follow, in the open and below. A wager's own farther chase isn't stacked on it
    /// (the farther of the two).
    pub leash_scale: f32,
    /// The string a boss just began is this many times as likely to come again next: it seldom
    /// does the same thing twice running.
    pub repeat: f32,
    /// The first blow of a string (or a lone blow) up close never winds up quicker than this,
    /// whatever its own wind-up or its phase's pace: long enough to answer from inside a light
    /// swing. A string's later blows keep their own pace. 0: no floor.
    #[serde(default)]
    pub first_blow: f32,
    /// One string at a time on a hero: while another creature is in a string at them up close
    /// (winding up or recovering), and for this long after it's done, a creature waits its turn
    /// (0: no turns).
    #[serde(default)]
    pub turn_beat: f32,
    /// A creature that has just walked up settles this long before a string up close (not a lunge
    /// or a charge from afar, nor a lone creature's own reach). 0: none.
    #[serde(default)]
    pub arrive_beat: f32,
    /// One blow of a boss's string up close is now and then held a beat (this chance a string,
    /// up to `hold_secs` longer in its wind-up): the delayed swing, so a parry timed by rote
    /// comes early.
    pub hold: f32,
    pub hold_secs: f32,
    /// A boss's string up close of three blows or more stops one short, this often; of two or
    /// more, runs one longer (its last blow again), `extend` as often.
    pub cut: f32,
    pub extend: f32,
    /// A boss's string up close done, this often it goes straight into another instead of
    /// resting (not past three close strings in a row).
    pub chain: f32,
    /// A boss's string up close done, the chance it backs off (facing its foe) to where one of
    /// its blows for farther off (a charge, a lunge, a throw) can come from, and uses it:
    /// `space`, and `space_streak` more for each close string in a row before, up to
    /// `space_most`; times `space_band` with two heroes or more in its fight.
    pub space: f32,
    pub space_streak: f32,
    pub space_most: f32,
    pub space_band: f32,
    /// Each creature its own within its kind: as it appears, its pace between strings and its
    /// readiness to give ground are each its kind's give or take this share, and one of its
    /// strings is its favourite, `favourite` times as likely as its kind's own weight says.
    #[serde(default)]
    pub quirk: f32,
    #[serde(default = "one_f")]
    pub favourite: f32,
}

impl Default for AiDef {
    fn default() -> Self {
        Self {
            retarget_secs: 1.2,
            threat: 3.0,
            opening: 1.0,
            wounded: 0.8,
            distance: 1.0,
            crowd: 1.0,
            stick: 0.8,
            threat_half: 8.0,
            press_early: 1.2,
            guard_break: 3.0,
            retreat: 0.3,
            retreat_yards: 4.0,
            flank: true,
            jitter: 0.25,
            leash_scale: 1.0,
            repeat: 0.3,
            first_blow: 0.0,
            turn_beat: 0.0,
            arrive_beat: 0.0,
            hold: 0.25,
            hold_secs: 0.45,
            cut: 0.2,
            extend: 0.2,
            chain: 0.25,
            space: 0.3,
            space_streak: 0.2,
            space_most: 0.8,
            space_band: 0.5,
            quirk: 0.0,
            favourite: 1.0,
        }
    }
}

/// The swing's soft lock: a blow turns toward a foe a little off to the side.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct LockDef {
    /// How far past the weapon's own arc (degrees, each side) a foe can be and still draw the
    /// blow to it.
    pub melee: f32,
    /// How far off the facing (degrees) an unaimed shot finds a foe.
    pub shot: f32,
}

impl Default for LockDef {
    fn default() -> Self {
        Self { melee: 35.0, shot: 10.0 }
    }
}

/// Heroes fighting each other.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct PvpDef {
    /// Heroes' blows and arrows strike other heroes.
    #[serde(default)]
    pub friendly_fire: bool,
    /// Getting up after a fall: how soon, and with how much health.
    pub respawn_secs: Option<f32>,
    pub respawn_health: Option<f32>,
    /// A fall leaves no sack: what a hero carries stays with them.
    #[serde(default)]
    pub keep_supplies: bool,
}

/// A duel between two heroes (WoW's own: the challenge's popup, its countdown, the flag between
/// them). Outside Siege only.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct DuelDef {
    /// How near the one challenged must be.
    pub reach: f32,
    /// How long a challenge waits for an answer.
    pub ask_secs: f32,
    /// The countdown once it's taken.
    pub countdown_secs: f32,
    /// How far from the flag a duellist can go; out past it, they must be back within
    /// `return_secs` or forfeit. Back inside `back_in` counts as back.
    pub bounds: f32,
    pub back_in: f32,
    pub return_secs: f32,
    /// The flag planted between them (`GameObjectDisplayInfo`: 787 is the 1.12 dueling flag).
    pub flag_display: u32,
    pub flag_scale: f32,
}

impl Default for DuelDef {
    fn default() -> Self {
        Self { reach: 30.0, ask_secs: 60.0, countdown_secs: 3.0, bounds: 50.0, back_in: 45.0, return_secs: 10.0, flag_display: 787, flag_scale: 1.0 }
    }
}

/// A bow or crossbow.
/// A spell's missiles as a client with patch 0030 draws them (`SetMissileLook`): a ribbon of
/// `trail` (`"RRGGBB"`; empty, none) `width` yards wide, each stretch fading over `secs`, the model
/// `scale` times its size and spinning `spin` turns a second, flat. Keyed by the spell, so every
/// ability flying it looks so.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MissileLookDef {
    pub spell: u32,
    #[serde(default)]
    pub trail: String,
    #[serde(default = "look_width")]
    pub width: f32,
    #[serde(default = "look_secs")]
    pub secs: f32,
    #[serde(default = "full_size")]
    pub scale: f32,
    #[serde(default)]
    pub spin: f32,
    /// The spin end over end, about the axis across the flight (a thrown axe's tumble), instead
    /// of flat about its up (the sixteenth client; older ones spin it flat).
    #[serde(default)]
    pub tumble: bool,
    /// How fast it flies, yards a second, on a client with patch 0040 (0: the spell's own speed,
    /// which the clients cap at 100): a bolt quicker than anything in the data (Kill Shot). The
    /// rules' own speed is the ability's; this keeps the look in step with it.
    #[serde(default)]
    pub speed: f32,
}

fn look_width() -> f32 {
    0.25
}

fn look_secs() -> f32 {
    0.35
}

fn full_size() -> f32 {
    1.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct RangedDef {
    pub key: String,
    /// 0: no draw, it shoots at once at full power (a crossbow), then reloads (`recovery`).
    pub draw_secs: f32,
    pub min_draw: f32,
    pub stamina: f32,
    pub damage: [u32; 2],
    pub range: f32,
    pub hit_radius: f32,
    pub arrow_speed: f32,
    pub recovery: f32,
    pub spell: u32,
    /// The client's item subclass (2 bow, 18 crossbow).
    pub subclass: u32,
    /// A click without aiming: a quick shot, loosed this long into its draw.
    #[serde(default = "quick_draw")]
    pub quick_draw: f32,
    /// Aimed shots fly along the aiming camera's line, this far to the hero's right.
    #[serde(default = "shoulder")]
    pub shoulder: f32,
}

fn quick_draw() -> f32 {
    0.35
}

fn shoulder() -> f32 {
    0.75
}

#[derive(Debug, Clone, Deserialize)]
pub struct WeaponDef {
    pub key: String,
    pub windup: f32,
    pub active: f32,
    pub recovery: f32,
    pub stamina: f32,
    pub damage: [u32; 2],
    pub range: f32,
    pub arc: f32,
    /// At most this many struck by one swing.
    #[serde(default = "two")]
    pub targets: usize,
    /// A blow that lands on a creature winding up interrupts it and staggers it this long.
    #[serde(default)]
    pub stagger: f32,
    /// Held in both hands: no shield; it guards with its own `guard` (a [[shield]] key).
    #[serde(default)]
    pub two_handed: bool,
    #[serde(default)]
    pub guard: String,
    /// The swing's look (`Emotes.dbc`: 36 one-handed, 37 two-handed, 38 a polearm's).
    #[serde(default = "one_handed_swing")]
    pub emote: u32,
    /// The heavy attack's look; 0 picks one for the grip (a one-handed special attack, or the
    /// other two-handed swing).
    #[serde(default)]
    pub heavy_emote: u32,
    /// The client's item subclass (0 axe, 4 mace, 7 sword, 15 dagger; 1, 5, 8 their two-handed
    /// kinds, 6 polearm).
    #[serde(default)]
    pub subclass: u32,
    /// Its blows ([[moveset]]); empty picks one for the grip.
    #[serde(default)]
    pub moveset: String,
    /// The timed guard (a parry) has this much longer with it in hand.
    #[serde(default)]
    pub parry: f32,
    /// A blow from behind does this many times its damage (0: the rules' own backstab).
    #[serde(default)]
    pub backstab: f32,
    /// Its blows count this many times over towards breaking a foe's stance (0: once).
    #[serde(default)]
    pub posture: f32,
    /// A blow on a foe reeling from a parry does this many times its damage (0: `[parry]
    /// riposte`): the sword's.
    #[serde(default)]
    pub riposte: f32,
    /// Each hit bleeds the foe this much a second for `BLEED_SECS` (the axe's).
    #[serde(default)]
    pub bleed: f32,
    /// Its blows go through a creature's raised shield (the spear's thrust).
    #[serde(default)]
    pub through_guard: bool,
    /// Swinging it, a heavy blow doesn't put the hero down: they take it and finish the swing
    /// (the greatsword's).
    #[serde(default)]
    pub swing_poise: bool,
    /// On a foe under a third of its health, its blows do this many times their damage (0: no
    /// more): the executioner's axe.
    #[serde(default)]
    pub execute: f32,
    /// A staff that casts instead of swinging: the left click casts this ability (a benched
    /// projectile, the staff's bolt), and the heavy attack this one (empty: the heavy swings).
    #[serde(default)]
    pub bolt: String,
    #[serde(default)]
    pub heavy_bolt: String,
    /// The right click as an ability instead of a guard (a holy staff's Flash Heal, a shadow
    /// staff's Drain): a benched ability, cast as the right mouse goes down. A `channel` is held
    /// while the button is (let go, it ends); anything else goes on the press, as often as its
    /// `cooldown` lets it (the fire rate). A weapon with one has no guard at all: no block, no
    /// parry, no ward.
    #[serde(default)]
    pub alt: String,
    // --- the weapon identity pass: one mechanic each (docs/weapon-identities.md) -------------
    /// Foes of these kinds (`[[mob]] tags`: undead, worgen, stone, mechanical...) take
    /// `bane_damage` more from its blows.
    #[serde(default)]
    pub bane: Vec<String>,
    /// The extra a bane foe takes (0.5: half as much again).
    #[serde(default)]
    pub bane_damage: f32,
    /// A bane foe it strikes is silvered for `SILVER_SECS`: it doesn't mend on its way home and
    /// stays down once slain (no respawn, no return at a rest).
    #[serde(default)]
    pub silver: bool,
    /// Its heavy blow hooks the foe and pulls it this many yards toward the hero.
    #[serde(default)]
    pub hook: f32,
    /// Its heavy blow is a leap: the hero crosses this many yards into it.
    #[serde(default)]
    pub boarding: f32,
    /// Each light blow strikes every foe in a line this long in front, not an arc.
    #[serde(default)]
    pub line: f32,
    /// A blow from behind is nothing special (a blade that doesn't stab).
    #[serde(default)]
    pub no_backstab: bool,
    /// The last blow of its light chain carries the hero behind the foe it strikes.
    #[serde(default)]
    pub blink_behind: bool,
    /// Its heavy blow hooks a shield-bearer's guard aside: it can't block for this many seconds.
    #[serde(default)]
    pub guard_hook: f32,
    /// Each kill speeds the hero's swings by this much, stacking up to `BLOODLUST_STACKS` for
    /// `BLOODLUST_SECS` after the latest.
    #[serde(default)]
    pub bloodlust: f32,
    /// Each foe a blow strikes heals the hero this share of their health.
    #[serde(default)]
    pub reap_heal: f32,
    /// A blow slows the foe by this share for `SNARE_SECS`.
    #[serde(default)]
    pub snare: f32,
    /// A blow cows the foe (it deals this much less and winds up slower) for `COW_SECS`.
    #[serde(default)]
    pub cow: f32,
    /// Its heavy blow knocks the foe back this many yards.
    #[serde(default)]
    pub knock: f32,
    /// A hit tenderizes the foe for `TENDER_SECS`: the next hit on it from another ally (not the
    /// wielder) does this much more.
    #[serde(default)]
    pub tender: f32,
    /// Its heavy blow stuns a mechanical foe (a machine that isn't a champion) this many seconds.
    #[serde(default)]
    pub machine_stun: f32,
    /// Each hit heals the most wounded hero within `MEND_RANGE` (the wielder too) this share of
    /// their health.
    #[serde(default)]
    pub mend_ally: f32,
    /// A hit curses the foe for `CURSE_SECS`; a cursed foe that dies bursts in shadow for this
    /// many times the weapon's average blow on foes within `CURSE_RADIUS`.
    #[serde(default)]
    pub curse: f32,
    /// Each hit puts a shield of this much on the hero for `IRON_SKIN_SECS`, stacking to
    /// `IRON_SKIN_CAP`.
    #[serde(default)]
    pub iron_skin: f32,
    /// A hit makes a foe lose interest in the hero (it drops them, and takes no notice for
    /// `LULL_SECS`). Not a champion.
    #[serde(default)]
    pub lull: bool,
    /// A blow from behind is a shatter trigger: it breaks a frozen foe's ice for `SHATTER` times
    /// the damage (the one shatter, `sim::abilities`).
    #[serde(default)]
    pub shatter: bool,
    /// A blow from behind bursts in fire on the foes round the target for this share of it,
    /// within `BURST_RADIUS`.
    #[serde(default)]
    pub burst: f32,
    /// A hit poisons the foe this much a second, a stack each, up to `POISON_STACKS`, for
    /// `POISON_SECS` from the latest.
    #[serde(default)]
    pub poison: f32,
    /// A hit on a foe casting interrupts it and staggers it this many seconds.
    #[serde(default)]
    pub interrupt: f32,
    /// A blow from behind puts the foe to sleep this many seconds (any damage wakes it).
    #[serde(default)]
    pub sleep: f32,
    /// Its heavy blow roots every foe it strikes this many seconds.
    #[serde(default)]
    pub root: f32,
    /// Its heavy blow sends a holy shockwave out to `SHOCK_RADIUS`: this many times the weapon's
    /// average blow, on every foe in it.
    #[serde(default)]
    pub shockwave: f32,
    /// Hits in a row on the same foe (within `BUILD_WINDOW` of one another) build: the
    /// `BUILD_HITS`th staggers it this many seconds.
    #[serde(default)]
    pub build: f32,
    /// Its heavy blow brings rocks down on the foe's spot after `ROCK_DELAY`: this many times the
    /// weapon's average blow on every foe within `ROCK_RADIUS`.
    #[serde(default)]
    pub rockfall: f32,
    /// Its blows do up to this much more as the hero's health falls (at none left).
    #[serde(default)]
    pub rage: f32,
    /// Its heavy blow on a helpless foe (stunned, asleep, rooted, frozen or staggered) does this
    /// many times the damage.
    #[serde(default)]
    pub guillotine: f32,
    /// Its heavy blow rallies the heroes within `RALLY_RADIUS` (the wielder too): they take this
    /// much less damage for `RALLY_SECS`.
    #[serde(default)]
    pub rally: f32,
    /// Its blows do this much more to a creature in its recovery, between its own blows (the
    /// punisher's dagger: let it strike first).
    #[serde(default)]
    pub punish: f32,
    /// Each hit gives the hero this much stamina back (a fist's quick jabs keep the feet light).
    #[serde(default)]
    pub wind: f32,
    /// Its hits within `SLIP_SECS` of a dodge that slipped a blow do this much more (the claw's
    /// counter-punch: let it swing, slip it, strike).
    #[serde(default)]
    pub slip: f32,
}

/// How long after a dodge slips a blow a slipping weapon's hits do more.
pub const SLIP_SECS: f32 = 1.0;

/// The shockwave's reach, the build's hits and window, the rockfall's delay and radius, the
/// rally's reach and length.
pub const SHOCK_RADIUS: f32 = 6.0;
pub const BUILD_HITS: u32 = 3;
pub const BUILD_WINDOW: f32 = 3.0;
pub const ROCK_DELAY: f32 = 1.0;
pub const ROCK_RADIUS: f32 = 2.5;
pub const RALLY_RADIUS: f32 = 8.0;
pub const RALLY_SECS: f32 = 5.0;

/// How long a lulled foe takes no notice, the fire burst's radius, the poison's stacks and length.
pub const LULL_SECS: f32 = 4.0;
pub const BURST_RADIUS: f32 = 4.0;
pub const POISON_STACKS: u32 = 5;
pub const POISON_SECS: f32 = 6.0;
/// How long after a sleep a foe can't be put to sleep again (after the sleep itself).
pub const SLEEP_IMMUNE_SECS: f32 = 6.0;

/// The tenderized foe's window, the mending hand's reach, the curse's length and burst radius, the
/// iron skin's length and cap.
pub const TENDER_SECS: f32 = 6.0;
pub const MEND_RANGE: f32 = 15.0;
pub const CURSE_SECS: f32 = 12.0;
pub const CURSE_RADIUS: f32 = 5.0;
pub const IRON_SKIN_SECS: f32 = 8.0;
pub const IRON_SKIN_CAP: f32 = 30.0;

/// How long a bloodlust lasts after the latest kill, and how many kills it stacks.
pub const BLOODLUST_SECS: f32 = 6.0;
pub const BLOODLUST_STACKS: u32 = 4;
/// How long a blow's slow and its cow last.
pub const SNARE_SECS: f32 = 2.5;
pub const COW_SECS: f32 = 4.0;

/// "undead and worgen", "stone, mechanical and undead": a list of kinds as words.
fn join_kinds(kinds: &[String]) -> String {
    match kinds {
        [] => String::new(),
        [one] => one.clone(),
        [init @ .., last] => format!("{} and {last}", init.join(", ")),
    }
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
}

/// A weapon this long (yards) has the longest reach on the rack.
pub const LONGEST_REACH: f32 = 5.4;

/// How long a silvered foe stays silvered (a bane foe struck by a silver weapon).
pub const SILVER_SECS: f32 = 10.0;
/// The width, either side, of a line blow (a machete clearing brush).
pub const LINE_HALF_WIDTH: f32 = 0.9;

impl WeaponDef {
    /// The mechanics a weapon has of its own from the identity pass (docs/weapon-identities.md):
    /// what it does that no number says. A weapon on the rack with none is a number copy.
    pub fn unique_mechanics(&self) -> Vec<String> {
        let on = [
            (!self.bane.is_empty() && self.bane_damage > 0.0, "bane"),
            (self.silver, "silver"),
            (self.hook > 0.0, "hook"),
            (self.boarding > 0.0, "boarding"),
            (self.line > 0.0, "line"),
            (self.no_backstab, "no_backstab"),
            (self.blink_behind, "blink_behind"),
            (self.guard_hook > 0.0, "guard_hook"),
            (self.bloodlust > 0.0, "bloodlust"),
            (self.reap_heal > 0.0, "reap_heal"),
            (self.snare > 0.0, "snare"),
            (self.cow > 0.0, "cow"),
            (self.knock > 0.0, "knock"),
            (self.tender > 0.0, "tender"),
            (self.machine_stun > 0.0, "machine_stun"),
            (self.mend_ally > 0.0, "mend_ally"),
            (self.curse > 0.0, "curse"),
            (self.iron_skin > 0.0, "iron_skin"),
            (self.lull, "lull"),
            (self.shatter, "shatter"),
            (self.burst > 0.0, "burst"),
            (self.poison > 0.0, "poison"),
            (self.interrupt > 0.0, "interrupt"),
            (self.sleep > 0.0, "sleep"),
            (self.root > 0.0, "root"),
            (self.shockwave > 0.0, "shockwave"),
            (self.build > 0.0, "build"),
            (self.rockfall > 0.0, "rockfall"),
            (self.rage > 0.0, "rage"),
            (self.guillotine > 0.0, "guillotine"),
            (self.rally > 0.0, "rally"),
            (self.punish > 0.0, "punish"),
            (self.wind > 0.0, "wind"),
            (self.slip > 0.0, "slip"),
        ];
        on.into_iter().filter(|(on, _)| *on).map(|(_, n)| n.to_string()).collect()
    }

    /// Everything that sets a weapon apart besides its numbers, the kinds' own traits (a sword's
    /// parry, an axe's bleed, a staff's bolt) and the pass's mechanics together, as names: two
    /// weapons with the same moveset and the same names are the same weapon in other numbers.
    /// A bare stagger or posture counts here (they differ between weapons that differ otherwise)
    /// but is no mechanic (`unique_mechanics` and the kinds' traits are).
    pub fn trait_names(&self) -> Vec<String> {
        let mut s: Vec<String> = Vec::new();
        let kinds = [
            (self.parry > 0.0, "parry"),
            (self.riposte > 0.0, "riposte"),
            (self.backstab > 0.0, "backstab"),
            (self.bleed > 0.0, "bleed"),
            (self.through_guard, "through_guard"),
            (self.swing_poise, "swing_poise"),
            (self.execute > 0.0, "execute"),
            (self.posture > 1.0, "posture"),
            (self.stagger > 0.0, "stagger"),
            // (The longest reach on the rack is an identity of its own: the spear's.)
            (self.range >= LONGEST_REACH, "longest_reach"),
        ];
        s.extend(kinds.into_iter().filter(|(on, _)| *on).map(|(_, n)| n.to_string()));
        s.extend(self.unique_mechanics());
        if !self.bolt.is_empty() {
            s.push(format!("bolt:{}", self.bolt));
        }
        if !self.heavy_bolt.is_empty() {
            s.push(format!("heavy_bolt:{}", self.heavy_bolt));
        }
        if !self.alt.is_empty() {
            s.push(format!("alt:{}", self.alt));
        }
        s.sort();
        s
    }

    /// The sentences of its own mechanic (the pass of docs/weapon-identities.md), built from its
    /// numbers so the tooltip never drifts from the rules. An item's `text` has `{fx}` where they
    /// go. Empty for a weapon with none of its own.
    pub fn effect_text(&self) -> String {
        let mut s: Vec<String> = Vec::new();
        if self.bane_damage > 0.0 && !self.bane.is_empty() {
            s.push(format!("Deals {}% more damage to {}.", (self.bane_damage * 100.0).round(), join_kinds(&self.bane)));
        }
        if self.silver {
            s.push(format!("{} it harms are silvered for {} sec. They stop mending and stay down once slain.", capitalize(&join_kinds(&self.bane)), SILVER_SECS));
        }
        if self.hook > 0.0 {
            s.push(format!("Heavy attacks hook the enemy and pull it up to {} yd toward you.", self.hook));
        }
        if self.boarding > 0.0 {
            s.push(format!("Heavy attacks leap {} yd to board the enemy.", self.boarding));
        }
        if self.line > 0.0 {
            s.push(format!("Light attacks strike every enemy in a line {} yd long.", self.line));
        }
        if self.no_backstab {
            s.push("Does no extra damage from behind.".into());
        }
        if self.posture > 1.0 && !self.bane.is_empty() {
            s.push(format!("Breaks stances {}% faster.", ((self.posture - 1.0) * 100.0).round()));
        }
        if self.guard_hook > 0.0 {
            s.push(format!("Heavy attacks hook a shield aside: the enemy can't block for {} sec.", self.guard_hook));
        }
        if self.bloodlust > 0.0 {
            s.push(format!("Each kill speeds your swings by {}% for {} sec, up to {} times.", (self.bloodlust * 100.0).round(), BLOODLUST_SECS, BLOODLUST_STACKS));
        }
        if self.reap_heal > 0.0 {
            s.push(format!("Restores {}% of your health for each enemy it strikes.", (self.reap_heal * 100.0).round()));
        }
        if self.snare > 0.0 {
            s.push(format!("Slows the enemy by {}% for {} sec.", (self.snare * 100.0).round(), SNARE_SECS));
        }
        if self.cow > 0.0 {
            s.push(format!("Cows the enemy for {} sec: it deals {}% less damage and winds up slower.", COW_SECS, (self.cow * 100.0).round()));
        }
        if self.knock > 0.0 {
            s.push(format!("Heavy attacks knock the enemy back {} yd.", self.knock));
        }
        if self.tender > 0.0 {
            s.push(format!("Hits tenderize the enemy for {} sec. The next hit on it from another ally deals {}% more.", TENDER_SECS, (self.tender * 100.0).round()));
        }
        if self.machine_stun > 0.0 {
            s.push(format!("Heavy attacks stun mechanical enemies and machines for {} sec.", self.machine_stun));
        }
        if self.mend_ally > 0.0 {
            s.push(format!("Each hit heals the most wounded ally within {} yd, you included, for {}% of their health.", MEND_RANGE, (self.mend_ally * 100.0).round()));
        }
        if self.curse > 0.0 {
            let burst = ((self.damage[0] + self.damage[1]) as f32 / 2.0 * self.curse).round();
            s.push(format!("Hits curse the enemy for {} sec. A cursed enemy that dies bursts in shadow for {} damage to enemies within {} yd.", CURSE_SECS, burst, CURSE_RADIUS));
        }
        if self.iron_skin > 0.0 {
            s.push(format!("Each hit gives you a shield that absorbs {} damage for {} sec, stacking up to {}.", self.iron_skin, IRON_SKIN_SECS, IRON_SKIN_CAP));
        }
        if self.lull {
            s.push(format!("Hits make the enemy lose interest in you for {} sec.", LULL_SECS));
        }
        if self.shatter {
            s.push("Blows from behind shatter a frozen enemy's ice for extra damage.".into());
        }
        if self.burst > 0.0 {
            s.push(format!("Blows from behind burst in fire for {}% of their damage on enemies within {} yd.", (self.burst * 100.0).round(), BURST_RADIUS));
        }
        if self.poison > 0.0 {
            s.push(format!("Hits poison the enemy for {} damage a sec, stacking up to {} times, for {} sec.", self.poison, POISON_STACKS, POISON_SECS));
        }
        if self.interrupt > 0.0 {
            s.push(format!("Hits interrupt an enemy that is casting and stagger it for {} sec.", self.interrupt));
        }
        if self.sleep > 0.0 {
            s.push(format!("Blows from behind put the enemy to sleep for {} sec. Damage wakes it, and it can't be put to sleep again for {} sec.", self.sleep, SLEEP_IMMUNE_SECS));
        }
        if self.root > 0.0 {
            s.push(format!("Heavy attacks root the enemies they hit for {} sec.", self.root));
        }
        if self.shockwave > 0.0 {
            let n = ((self.damage[0] + self.damage[1]) as f32 / 2.0 * self.shockwave).round();
            s.push(format!("Heavy attacks send a holy shockwave out to {} yd that deals {} damage to every enemy in it.", SHOCK_RADIUS, n));
        }
        if self.build > 0.0 {
            s.push(format!("The {}rd hit in a row on the same enemy within {} sec staggers it for {} sec.", BUILD_HITS, BUILD_WINDOW, self.build));
        }
        if self.rockfall > 0.0 {
            let n = ((self.damage[0] + self.damage[1]) as f32 / 2.0 * self.rockfall).round();
            s.push(format!("Heavy attacks call rocks down on the enemy's spot after {} sec, dealing {} damage to enemies within {} yd.", ROCK_DELAY, n, ROCK_RADIUS));
        }
        if self.rage > 0.0 {
            s.push(format!("Deals up to {}% more damage as your health falls.", (self.rage * 100.0).round()));
        }
        if self.guillotine > 0.0 {
            s.push(format!("Heavy attacks on a stunned, asleep, rooted, frozen or staggered enemy deal {}% damage.", (self.guillotine * 100.0).round()));
        }
        if self.rally > 0.0 {
            s.push(format!("Heavy attacks rally allies within {} yd, you included: they take {}% less damage for {} sec.", RALLY_RADIUS, (self.rally * 100.0).round(), RALLY_SECS));
        }
        if self.punish > 0.0 {
            s.push(format!("Deals {}% more damage to an enemy recovering from its blow.", (self.punish * 100.0).round()));
        }
        if self.blink_behind {
            s.push("The last blow of a light chain carries you behind the enemy.".into());
        }
        if self.wind > 0.0 {
            s.push(format!("Each hit restores {} stamina.", self.wind));
        }
        if self.slip > 0.0 {
            s.push(format!("Hits within {} sec of a dodge that slips a blow deal {}% more damage.", SLIP_SECS, (self.slip * 100.0).round()));
        }
        s.join(" ")
    }

    /// The weapon casts its light attack.
    pub fn casts(&self) -> bool {
        !self.bolt.is_empty()
    }
    /// Its right click is an ability, not a guard.
    pub fn has_alt(&self) -> bool {
        !self.alt.is_empty()
    }
}

fn turn_rate() -> f32 {
    std::f32::consts::PI
}

fn two() -> usize {
    2
}

fn one_handed_swing() -> u32 {
    36
}

/// `Emotes.dbc`: a one-handed special attack (an overhead blow), the two two-handed swings.
const EMOTE_SPECIAL_1H: u32 = 54;
const EMOTE_ATTACK_2H: u32 = 37;
const EMOTE_ATTACK_2H_LOOSE: u32 = 38;

impl WeaponDef {
    /// (wind-up, active, recovery) of a swing, light or heavy.
    pub fn times(&self, heavy: Option<&HeavyDef>) -> (f32, f32, f32) {
        match heavy {
            Some(h) => (self.windup * h.windup, self.active, self.recovery * h.recovery),
            None => (self.windup, self.active, self.recovery),
        }
    }

    pub fn swing_secs(&self, heavy: Option<&HeavyDef>) -> f32 {
        let (w, a, r) = self.times(heavy);
        w + a + r
    }

    /// The swing's clip (`AnimationData`) for a client that plays clips directly: Attack1H (17),
    /// a dagger's Attack1HPierce (85), Attack2H (18), a polearm's Attack2HL (19); a heavy attack's
    /// Special1H (57) or Special2H (58).
    pub fn swing_clip(&self, heavy: bool) -> u32 {
        match (heavy, self.two_handed) {
            (true, false) => 57,
            (true, true) => 58,
            (false, true) if self.emote == EMOTE_ATTACK_2H_LOOSE => 19,
            (false, true) => 18,
            (false, false) if self.subclass == 15 => 85,
            (false, false) => 17,
        }
    }

    /// Which set of blows it swings: its own, or the grip's.
    pub fn moveset_key(&self) -> &str {
        if !self.moveset.is_empty() {
            &self.moveset
        } else if self.two_handed && self.subclass == 6 {
            "polearm"
        } else if self.two_handed {
            "two_hand"
        } else if self.subclass == 15 {
            "dagger"
        } else {
            "one_hand"
        }
    }

    /// The heavy attack's look.
    pub fn heavy_look(&self) -> u32 {
        match self.heavy_emote {
            0 if !self.two_handed => EMOTE_SPECIAL_1H,
            0 if self.emote == EMOTE_ATTACK_2H => EMOTE_ATTACK_2H_LOOSE,
            0 => EMOTE_ATTACK_2H,
            e => e,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ShieldDef {
    pub key: String,
    pub arc: f32,
    pub cost: f32,
    pub cost_per_damage: f32,
    pub break_secs: f32,
    pub break_share: f32,
    /// A ward (a staff's guard): its costs come off the ward bar, not stamina (`[ward]`).
    #[serde(default)]
    pub ward: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ItemKind {
    Resource,
    /// Anything used from the bags: a meal, a salve, a tonic, a sharpening stone.
    Food,
    Weapon,
    Shield,
    Bow,
    /// Worn: on the chest or the head. It takes a share off every blow that lands, and a heavy
    /// one slows the stamina's return.
    Armor,
    /// A bigger pack: resources stack higher in the bags.
    Pack,
}

/// What a used item does for `secs` after: `regen` heals `amount` of max health a second (a
/// share); `stamina` brings stamina back `amount` faster (a share); `might` adds `amount` to every
/// blow's damage (a share).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EffectDef {
    pub kind: String,
    pub amount: f32,
    pub secs: f32,
    /// What the hero's screen calls it.
    #[serde(default)]
    pub name: String,
}

impl ItemDef {
    /// What a used item's `anim` may be: nothing, eating or drinking.
    pub const ANIMS: [&'static str; 3] = ["", "eat", "drink"];

    /// The clip the body plays using it (0: none, the Food spell's cast instead).
    pub fn use_clip(&self, player: &PlayerDef) -> u16 {
        match self.anim.as_str() {
            "eat" => player.eat_clip,
            "drink" => player.drink_clip,
            _ => 0,
        }
    }
}

impl EffectDef {
    pub const KINDS: [&'static str; 5] = ["regen", "stamina", "might", "ward", "focus"];
}

#[derive(Debug, Clone, Deserialize)]
pub struct ItemDef {
    pub key: String,
    pub name: String,
    pub display: u32,
    /// Its icon (`Interface\Icons\` name), for the add-on's windows.
    #[serde(default)]
    pub icon: String,
    pub kind: ItemKind,
    #[serde(default = "one")]
    pub stack: u32,
    #[serde(default)]
    pub use_secs: f32,
    #[serde(default)]
    pub heal: f32,
    /// The `heal` comes over this many seconds once it's used, instead of at once (a bite of
    /// food: the healing comes after it, as a regeneration).
    #[serde(default)]
    pub heal_secs: f32,
    /// A hit doesn't stop it: neither the use nor the healing after it (food, a fight's
    /// resource). Otherwise a hit interrupts it and nothing is used.
    #[serde(default)]
    pub steady: bool,
    /// What the body does using it: `eat`, `drink` (the clips `[player]` names), or nothing (the
    /// Food spell's cast).
    #[serde(default)]
    pub anim: String,
    #[serde(default)]
    pub text: String,
    /// A used item's lasting effect.
    #[serde(default)]
    pub effect: Option<EffectDef>,
    /// Armor: where it's worn (`chest`, `head`), the share it takes off a blow, and how much it
    /// slows the stamina's return (a share).
    #[serde(default)]
    pub slot: String,
    #[serde(default)]
    pub reduce: f32,
    #[serde(default)]
    pub weight: f32,
    /// A pack: resources stack this share higher in the bags (0.5: half again).
    #[serde(default)]
    pub carry: f32,
    /// Usable in the Deadmines, where food isn't (a draught, a whetstone).
    #[serde(default)]
    pub below: bool,
    /// What it gives while worn (armor) or in hand (a weapon or bow, in its kit): lasting
    /// effects of the kinds a potion gives (`might`, `stamina`, `regen`, `focus`, `ward`).
    #[serde(default)]
    pub perks: Vec<PerkDef>,
    /// The set it's a piece of (`[[set]]`, by key), if any.
    #[serde(default)]
    pub set: String,
    /// A honed piece (made by the rules, not written in a file): its tier, 1 to [`HONE_TIERS`]'s
    /// length, and the piece it was honed from (by key). 0: an ordinary piece.
    #[serde(default)]
    pub hone: u8,
    #[serde(default)]
    pub honed_from: String,
}

/// Gear a won wager pays out from +4 up comes honed, better the higher the wager: (the least
/// wager, its name, how much better: weapons' damage, armour's share off a blow, perks).
/// Every piece the dungeons' foes carry has a honed copy of each tier.
pub const HONE_TIERS: [(u32, &str, f32); 3] = [(4, "Honed", 1.12), (8, "Keen", 1.25), (12, "Peerless", 1.4)];
/// What a honed tier adds besides its share (from Keen on, each worn piece of that tier or finer
/// counts): a parry gives back this much breath for each Keen piece (up to the most), and a
/// dodge through a creature's blow throws it off balance this long (open to a riposte) with any
/// Peerless piece on. First guesses, to try in play.
pub const KEEN_BREATH: f32 = 8.0;
pub const KEEN_BREATH_MOST: f32 = 24.0;
pub const PEERLESS_STAGGER: f32 = 0.8;
/// The trick a tier adds, said on the piece (none for the first).
pub const HONE_TRICKS: [&str; 3] = ["", " Keen: a parry gives back breath.", " Peerless: a parry gives back breath, and a dodge through a blow throws the foe off balance."];

/// A dungeon tier's foes past the first: this much more health, and their blows this much harder,
/// a tier. (Ragefire's trash and the Cathedral's were all but alike; a first pass to try in play.)
pub const TIER_HEALTH: f32 = 0.25;
pub const TIER_DAMAGE: f32 = 0.15;
/// And a won wager's plunder there: this much more a tier (a harder dungeon pays more).
pub const TIER_PLUNDER: f32 = 0.25;
/// The tiers' names, for the book.
pub const TIER_NAMES: [&str; 3] = ["a first dungeon", "a harder one", "among the hardest"];

fn first_tier() -> u32 {
    1
}

/// The honing tier a wager at `level` pays out (0: plain).
pub fn hone_tier(level: u32) -> u8 {
    HONE_TIERS.iter().filter(|(at, ..)| level >= *at).count() as u8
}

/// A lasting effect gear gives: its kind (as [`EffectDef::KINDS`]) and how much.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PerkDef {
    pub kind: String,
    pub amount: f32,
}

/// A perk in words, for a tooltip: "+5% damage", "+1% health a second"...
pub fn perk_words(kind: &str, amount: f32) -> String {
    let pc = |v: f32| {
        let p = v * 100.0;
        if (p - p.round()).abs() < 0.05 { format!("{:.0}", p) } else { format!("{p:.1}") }
    };
    match kind {
        "might" => format!("+{}% damage", pc(amount)),
        "stamina" => format!("+{}% stamina recovery", pc(amount)),
        "regen" => format!("heals {}% of your health a second", pc(amount)),
        "focus" => format!("{}% shorter cooldowns", pc(amount)),
        "ward" => format!("+{}% toughness (past armor's limit)", pc(amount)),
        _ => format!("{kind} {amount}"),
    }
}

/// A set of gear (`[[set]]`): with this many of its pieces worn or in hand, more perks.
#[derive(Debug, Clone, Deserialize)]
pub struct SetDef {
    pub key: String,
    pub name: String,
    pub bonus: Vec<SetBonusDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetBonusDef {
    pub pieces: usize,
    pub kind: String,
    pub amount: f32,
}

/// Where armor is worn.
/// The longest a secret's `page` may be, in characters: the whole page goes to the add-on in one
/// line. Not a client limit we know of, a guard against a runaway page.
const PAGE_MAX: usize = 1000;

pub const ARMOR_SLOTS: [&str; 4] = ["chest", "head", "shoulders", "legs"];

/// The Deadmines (`[dungeon]` in `deadmines.toml`): a souls-like run from Ironclad Cove to the
/// mine's front door, its own world on the instance's map.
#[derive(Debug, Clone, Deserialize)]
pub struct DungeonDef {
    /// Known by (its save and its page in the book); the Deadmines' when not given.
    #[serde(default = "dm_key")]
    pub key: String,
    pub name: String,
    /// The instance map it's on: its `Map.dbc` id, its `World\Maps` folder, and its area
    /// (`AreaTable.dbc`), the Deadmines' when not given.
    #[serde(default = "dm_map")]
    pub map: u32,
    #[serde(default = "dm_map_dir")]
    pub map_dir: String,
    #[serde(default = "dm_area")]
    pub area: u32,
    /// Only this much of its map is read (its lowest x and y, then its highest), for a map
    /// whose dungeon sits in wide country (Shadowfang Keep in Silverpine's hills). None: all of it.
    #[serde(default)]
    pub window: Option<[[f32; 2]; 2]>,
    /// Its page in the book at camp: an icon, a line under its name, and what the first page says.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub blurb: String,
    #[serde(default)]
    pub first_page: String,
    /// Said to a hero as they come down.
    #[serde(default)]
    pub welcome: String,
    /// The Smuggler's Logbook at camp: where it lies (Elwynn), and which way it faces (the
    /// Deadmines' says; the others' pages are in the same book).
    #[serde(default)]
    pub book: [f32; 2],
    #[serde(default)]
    pub book_facing: f32,
    /// Raids the camp must have beaten before the book opens.
    #[serde(default)]
    pub gate_raids: u32,
    /// The Miner's Draught (an item key): the heal in a fight below, charges refilled at a lamp.
    pub draught: String,
    pub draughts: u32,
    pub draughts_max: u32,
    /// A Draught Flask (an item key) found below: one charge more, up to the most.
    #[serde(default)]
    pub flask: String,
    /// Defias Plunder (an item key): what every foe drops, lost with the sack.
    pub plunder: String,
    /// Each hero past the first gives every foe this share more health.
    #[serde(default)]
    pub hero_health: f32,
    /// Each hero past the first brings this share more foes into every pack of two or more
    /// (rounded; a boss's, an elite's or a lone foe's stays as it is). The pack's whole health
    /// is what `hero_health` would have made it, shared over its bodies (each at least its own).
    #[serde(default)]
    pub hero_foes: f32,
    /// Where the run ends: past the front door, and how near counts.
    pub exit: [f32; 3],
    #[serde(default = "exit_radius")]
    pub exit_radius: f32,
    /// The boss (a pack's key) that must be down for the front door to open.
    #[serde(default)]
    pub exit_boss: String,
    /// Other ways out, open on the same terms (the smugglers' sea tunnel, once the mine has come
    /// down on the front door), and what the banner says when they open (empty: the front door's).
    #[serde(default)]
    pub more_exits: Vec<[f32; 3]>,
    #[serde(default)]
    pub way_out: String,
    /// The lamp's quartermaster: what plunder buys (item key to its price).
    #[serde(default)]
    pub trade: BTreeMap<String, u32>,
    /// The chance a foe drops what it holds, and a piece of what it wears.
    #[serde(default)]
    pub weapon_chance: f32,
    #[serde(default)]
    pub armor_chance: f32,
    /// A boss: one of its pieces falls for certain, each other piece this often.
    #[serde(default = "quarter")]
    pub boss_extra_chance: f32,
    /// Minutes after the first boss of a run is beaten (once no hero is below) that the bosses
    /// stand again and the strongboxes fill: the Deadmines run again for their loot. 0: never.
    #[serde(default)]
    pub reset_minutes: f32,
    /// The Bookmaker's Wager: the time to beat (minutes; by the dungeon's size when not given).
    #[serde(default)]
    pub wager_minutes: f32,
    /// How hard it is next to the others: 1 (a first dungeon) to 3 (the hardest). Its foes are
    /// tougher by [`TIER_HEALTH`] and [`TIER_DAMAGE`] a tier past the first; the book lists the
    /// dungeons by it, the easiest first.
    #[serde(default = "first_tier")]
    pub tier: u32,
    /// What the camp's chest gets the first time the camp clears it (every boss down in a run):
    /// item key to count. Empty: a banner, and the book's next page, but nothing in the chest.
    #[serde(default)]
    pub first_clear: BTreeMap<String, u32>,
    #[serde(rename = "lamp", default)]
    pub lamps: Vec<LampDef>,
    #[serde(rename = "door", default)]
    pub doors: Vec<DoorDef>,
    #[serde(rename = "box", default)]
    pub boxes: Vec<BoxDef>,
    #[serde(rename = "pack", default)]
    pub packs: Vec<PackDef>,
    /// Ground marked to burn (a molten channel), cave in or blow up: hazards on a beat.
    #[serde(rename = "hazard", default)]
    pub hazards: Vec<HazardDef>,
    /// Notes left lying about: diaries, orders, recipes; some tell of what's hidden.
    #[serde(rename = "note", default)]
    pub notes: Vec<NoteDef>,
    /// Rails along open edges (a ship's side): nothing crosses one.
    #[serde(rename = "rail", default)]
    pub rails: Vec<RailDef>,
}

/// A rail along an open edge, drawn with a native fence piece every few yards seated on the floor
/// and following the line. Nothing crosses it: a hero's walk, roll or jump over it is put back,
/// and a knockback, a pull or a hop back stops at it, for heroes and creatures alike.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct RailDef {
    pub key: String,
    /// The line it runs along, in order: `[x, y, z]`, z the floor's height there. The deck lies to
    /// the line's left as its points run.
    pub points: Vec<[f32; 3]>,
    /// The piece (`GameObjectDisplayInfo`) and its scale.
    pub display: u32,
    #[serde(default = "rail_scale")]
    pub scale: f32,
    /// Where the piece runs, from and to, at scale 1: along its own x from its origin, or along its
    /// y with `along_y`. Pieces are laid as straight chords of the line that long, end to end.
    pub run: [f32; 2],
    #[serde(default)]
    pub along_y: bool,
    /// Its bottom and top over its origin at scale 1 (its collision hull's).
    pub height: [f32; 2],
    /// How far its origin stands over the deck under it (below zero: sunk into it). Each piece is
    /// tipped to follow the deck's rise along its chord, so this holds all along it.
    #[serde(default)]
    pub lift: f32,
}

fn rail_scale() -> f32 {
    1.0
}

fn exit_radius() -> f32 {
    6.0
}

fn dm_key() -> String {
    "deadmines".into()
}

fn dm_map() -> u32 {
    36
}

fn dm_map_dir() -> String {
    "DeadminesInstance".into()
}

fn dm_area() -> u32 {
    1581
}

/// A miner's lamp: lighting it makes it the hero's to wake at, and a page in the book.
#[derive(Debug, Clone, Deserialize)]
pub struct LampDef {
    pub key: String,
    pub name: String,
    pub at: [f32; 3],
    #[serde(default)]
    pub facing: f32,
}

/// A way through: a fog that bars a boss's room while the boss fights (`fog`, its `boss` a pack's
/// key), a wall of shadow until a boss falls (`gate`), a shortcut opened from its far side
/// (`shortcut`, a door at `at` opened from `from`), or a lift, rope or cart between two ends
/// (`lift`: `at` and `from`, set going from `from`, then a ride either way). A secret: a wall that
/// isn't one (`illusory`: it gives way when used, from either side) or a spot found by walking
/// there (`hidden`, within `find`); either is "A secret!" and lays out what it `reveals`.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct DoorDef {
    pub key: String,
    pub name: String,
    pub kind: String,
    /// The door's look (`GameObjectDisplayInfo`; 0 for a fog or a gate), where, which way, how big.
    #[serde(default)]
    pub display: u32,
    pub at: [f32; 3],
    #[serde(default)]
    pub facing: f32,
    #[serde(default = "one_f")]
    pub scale: f32,
    /// A shortcut: the spot it's opened from (within reach of it).
    #[serde(default)]
    pub from: Option<[f32; 3]>,
    #[serde(default)]
    pub boss: String,
    /// The line it bars while shut, end to end (nobody walks through it).
    #[serde(default)]
    pub bars: Option<[[f32; 2]; 2]>,
    /// Said when it opens (a secret's, to everyone in chat after the finder's name: keep it a line).
    #[serde(default)]
    pub text: String,
    /// A secret's page, the finder's to read (the letter that opens): what they find and read
    /// there, `{n}` between lines. Without one, the page is `text`.
    #[serde(default)]
    pub page: String,
    /// A `hidden` secret: found by a hero this near `at`.
    #[serde(default)]
    pub find: f32,
    /// What a secret lays out once found: packs and strongboxes (by key), kept back till then.
    #[serde(default)]
    pub reveals: Vec<String>,
    /// A `hidden` secret's marker, as it looks before anyone knows what it is (its `display` at
    /// `at`): "A Heap of Sacks". Searched, it gives the secret up.
    #[serde(default)]
    pub look: String,
}

impl DoorDef {
    pub const KINDS: [&'static str; 6] = ["shortcut", "fog", "gate", "lift", "illusory", "hidden"];

    /// A secret: found, not opened.
    pub fn secret(&self) -> bool {
        matches!(self.kind.as_str(), "illusory" | "hidden")
    }
}

/// A note left below (`[[dungeon.note]]`): a page of a diary, an order, a recipe. Read, it says
/// what it says; some tell of what's hidden.
#[derive(Debug, Clone, Deserialize)]
pub struct NoteDef {
    pub key: String,
    pub name: String,
    pub at: [f32; 3],
    #[serde(default)]
    pub facing: f32,
    /// Its look (`GameObjectDisplayInfo`; a scroll on the ground by default).
    #[serde(default)]
    pub display: u32,
    #[serde(default = "one_f")]
    pub scale: f32,
    pub text: String,
    /// The secrets (door keys) it points to, and a line of where, kept in the book's rumours
    /// till they're found.
    #[serde(default)]
    pub hints: Vec<String>,
    #[serde(default)]
    pub clue: String,
}

/// A strongbox off the path: what it holds, once.
#[derive(Debug, Clone, Deserialize)]
pub struct BoxDef {
    pub key: String,
    /// What it's called ("Defias Strongbox" by default).
    #[serde(default)]
    pub name: String,
    pub at: [f32; 3],
    #[serde(default)]
    pub facing: f32,
    pub loot: BTreeMap<String, u32>,
}

/// Ground that hurts on a beat: marked `warn` seconds before, then `damage` to everyone in the
/// circle, every `every` seconds (offset `phase`), while a hero is within `wake` yards.
#[derive(Debug, Clone, Deserialize)]
pub struct HazardDef {
    pub name: String,
    pub at: [f32; 3],
    pub radius: f32,
    pub damage: u32,
    pub every: f32,
    #[serde(default)]
    pub phase: f32,
    #[serde(default = "hazard_warn")]
    pub warn: f32,
    #[serde(default = "hazard_wake")]
    pub wake: f32,
    /// Its look as it goes off (a spell visual), and only while this boss (a pack key) fights.
    #[serde(default)]
    pub spell: u32,
    #[serde(default)]
    pub boss: String,
    /// It throws down whoever it catches.
    #[serde(default)]
    pub knockdown: bool,
}

fn hazard_warn() -> f32 {
    1.2
}

fn hazard_wake() -> f32 {
    40.0
}

fn one() -> u32 {
    1
}

#[derive(Debug, Clone, Deserialize)]
pub struct StartDef {
    pub weapon: String,
    pub shield: String,
    pub bow: String,
    #[serde(default)]
    pub items: BTreeMap<String, u32>,
    /// The rows of their class's column a new hero knows from the start (Q and E: `ranged`,
    /// `close`); the rest are learned out in Elwynn.
    #[serde(default = "first_cards")]
    pub cards: Vec<String>,
}

fn first_cards() -> Vec<String> {
    vec!["ranged".into(), "close".into()]
}

#[derive(Debug, Clone, Deserialize)]
pub struct StationDef {
    pub key: String,
    pub name: String,
    pub display: u32,
    /// Its icon in the build menu and its own window (`Interface\Icons\` name).
    #[serde(default)]
    pub icon: String,
    #[serde(default = "one_f")]
    pub scale: f32,
    pub footprint: f32,
    pub cost: BTreeMap<String, u32>,
    pub health: u32,
    pub repair: BTreeMap<String, u32>,
    #[serde(default)]
    pub text: String,
    /// A defence, not a workplace: raiders go for it first when they pass near it.
    #[serde(default)]
    pub defense: bool,
    /// Spikes: raiders right at it take this much a second.
    #[serde(default)]
    pub spikes: u32,
    /// A tower: it shoots the nearest raider in reach, now and then.
    #[serde(default)]
    pub shoots: Option<ShootDef>,
    /// Its building takes this many times `work.build`.
    #[serde(default = "one_f")]
    pub work: f32,
    /// The spell whose look making something at it wears (its cast on the maker; 0: a bandage's).
    #[serde(default)]
    pub work_spell: u32,
    /// In the build menu (a Siege camp's Hearth is not; nor what a station is upgraded into).
    #[serde(default = "yes")]
    pub menu: bool,
    /// The most the camp may have standing (0: no limit): one of each building.
    #[serde(default)]
    pub most: u32,
    /// Set on a grid this many yards a cell (and turned a quarter at a time), and blocks of it
    /// stand cell to cell: a wall laid out by eye lines up. 0: anywhere, any way round.
    #[serde(default)]
    pub snap: f32,
    /// Heroes walk through it (a gate): drawn with no collision on a patched client (0017);
    /// raiders still have to break it.
    #[serde(default)]
    pub passable: bool,
    /// What it can be upgraded into (another station's key), and what that costs: the station
    /// stays where it stands, keeps its uid, and comes back to full health as the new one.
    #[serde(default)]
    pub upgrade: Option<UpgradeDef>,
    /// The stations whose work it does as well as its own (an upgrade does what it was).
    #[serde(default)]
    pub makes_as: Vec<String>,
    /// Its work goes this many times as fast (an upgraded workplace: 1.5 is a third less time).
    #[serde(default = "one_f")]
    pub work_speed: f32,
    /// This many more of what it makes, for a meal or a material (not gear).
    #[serde(default)]
    pub bonus: u32,
    /// A building's job. Barracks: heroes who fall get up this many times as soon, and with
    /// this share of their health more.
    #[serde(default)]
    pub respawn: Option<RespawnDef>,
    /// Infirmary: resting within `radius` of it heals this many times as fast (as at the chest).
    #[serde(default)]
    pub rest: Option<RestDef>,
    /// A mill: what it adds to the camp's chest, every `every` seconds while it stands (not
    /// damaged, not during a raid).
    #[serde(default)]
    pub yields: Option<YieldDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct UpgradeDef {
    pub to: String,
    pub cost: BTreeMap<String, u32>,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RespawnDef {
    pub faster: f32,
    #[serde(default)]
    pub health: f32,
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct RestDef {
    pub radius: f32,
    pub faster: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct YieldDef {
    pub items: BTreeMap<String, u32>,
    pub every: f32,
}

fn one_f() -> f32 {
    1.0
}

fn send_cap() -> u32 {
    u32::MAX
}

fn supply_most() -> u32 {
    3
}

fn ready_secs() -> f32 {
    5.0
}

fn fourteen() -> f32 {
    14.0
}

fn quarter() -> f32 {
    0.25
}

/// A shot: a tower's (`shoots` on a `[[station]]`), or a friendly creature's (`shoots` on a `[[mob]]`).
#[derive(Debug, Clone, Deserialize)]
pub struct ShootDef {
    pub damage: u32,
    pub range: f32,
    /// Seconds between shots.
    pub every: f32,
    /// The look of the shot in flight (a spell with a missile; 0: the ranged weapon's).
    #[serde(default)]
    pub spell: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RecipeDef {
    pub key: String,
    pub station: String,
    pub inputs: BTreeMap<String, u32>,
    pub output: String,
    #[serde(default = "one")]
    pub count: u32,
    /// Its making takes this many times `work.craft` (gear longer than a meal).
    #[serde(default = "one_f")]
    pub work: f32,
}

/// The camp's work, in seconds: making something at a station, building one (times its own
/// `work`), mending a wrecked one. A hero stands at it the while (moving off, or a blow, stops
/// it); what it costs is paid as it's done. 0: at once.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WorkDef {
    pub craft: f32,
    /// Gear (a weapon, armour, a pack) takes this many times as long as a meal or a material.
    pub gear: f32,
    pub build: f32,
    pub repair: f32,
}

impl Default for WorkDef {
    fn default() -> Self {
        Self { craft: 2.5, gear: 1.6, build: 4.0, repair: 3.0 }
    }
}

impl WorkDef {
    /// All of it at once (tests that aren't about the waiting).
    pub fn instant() -> Self {
        Self { craft: 0.0, gear: 1.0, build: 0.0, repair: 0.0 }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct Circle {
    pub at: [f32; 2],
    pub radius: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NodeDef {
    /// What it's known by (a save names its nodes so); its item's key when not given.
    #[serde(default)]
    pub key: String,
    pub item: String,
    /// More it gives at each gather, each a roll (0 at the low end: maybe none).
    #[serde(default)]
    pub loot: BTreeMap<String, [u32; 2]>,
    pub name: String,
    pub display: u32,
    #[serde(default = "one_f")]
    pub scale: f32,
    pub gather_secs: f32,
    /// The spell whose look the gathering wears (its precast animation on the gatherer): 0 is
    /// Mining's swing (timber, ore, stone); Herb Gathering (2366) kneels to a plant, Opening
    /// (21651) works at a crate's lid.
    #[serde(default)]
    pub gather_spell: u32,
    /// A clip the gatherer swings over and over instead (timber: a chop with what's in hand, not
    /// Mining's pick), every `CHOP_SECS`; 0: the spell's look.
    #[serde(default)]
    pub gather_clip: u16,
    #[serde(rename = "yield")]
    pub yields: [u32; 2],
    pub uses: u32,
    pub respawn: f32,
    #[serde(default)]
    pub areas: Vec<Circle>,
    #[serde(default)]
    pub points: Vec<[f32; 2]>,
    /// Points with their height: down in a cave or a mine, under the hill (taken first).
    #[serde(default)]
    pub deep: Vec<[f32; 3]>,
    pub count: usize,
    pub spacing: f32,
    /// At the water's edge (a fishing school): only dry ground with water a stride or two off.
    #[serde(default)]
    pub shore: bool,
    /// Spread across the whole zone instead (on a land that knows its extent): the land cut into
    /// squares this many yards a side, a spot tried in each, in a random order, up to `count`
    /// (`spacing` apart). Clear of the camp's grounds.
    #[serde(default)]
    pub spread: f32,
    /// Beside a tree's trunk (timber, the herbs of the forest's shade); else open ground.
    #[serde(default)]
    pub by_trees: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AttackDef {
    #[serde(default)]
    pub ranged: bool,
    pub windup: f32,
    pub damage: u32,
    pub range: f32,
    pub arc: f32,
    pub recovery: f32,
    pub cooldown: f32,
    pub turn_rate: f32,
    pub lock: f32,
    #[serde(default = "yes")]
    pub blockable: bool,
    #[serde(default)]
    pub heavy: bool,
    #[serde(default)]
    pub emote: u32,
    /// A ranged attack's missile: speed, the blast's radius, and its visual.
    #[serde(default)]
    pub speed: f32,
    #[serde(default)]
    pub radius: f32,
    #[serde(default)]
    pub spell: u32,
    /// Light blows don't knock its wind-up aside (only a heavy blow, a bash or a kick): on by
    /// default for a heavy attack.
    #[serde(default)]
    pub poise: Option<bool>,
    /// A spell whose ground look plays where the blow lands (a missile's landing spot, or a blow
    /// all round at the creature's feet): a splash, a burst, a shockwave. 0: none.
    #[serde(default)]
    pub look: u32,
    /// Its own sound as its wind-up begins, from the creature, for everyone near: a crossbow
    /// drawn, a spell gathered, where its spell's look plays none and a hero must hear it coming.
    /// 0: none.
    #[serde(default)]
    pub sound: u32,
}

impl AttackDef {
    pub fn poised(&self) -> bool {
        self.poise.unwrap_or(self.heavy)
    }
}

/// A creature's second blow, now and then in place of its usual one: slower to come and
/// telegraphed with a flash, harder to take; what it doesn't say is the usual blow's.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecialDef {
    /// The chance of it each time the creature attacks.
    #[serde(default)]
    pub chance: f32,
    /// The clip it swings with (`AnimationData`; the creature's usual attack when not given). A
    /// cast (`ranged`) plays its spell's own cast unless given one.
    #[serde(default)]
    pub clip: Option<u16>,
    /// Another character model's take on it (as a hero's move's `from`); else the creature's
    /// `clips_from`, else its own.
    #[serde(default)]
    pub from: Option<String>,
    /// How far it steps into the blow as it lands (yards).
    #[serde(default)]
    pub lunge: f32,
    pub windup: Option<f32>,
    pub damage: Option<u32>,
    pub range: Option<f32>,
    pub arc: Option<f32>,
    pub recovery: Option<f32>,
    pub blockable: Option<bool>,
    pub heavy: Option<bool>,
    pub poise: Option<bool>,
    /// A caster's blow up close (`false`), or a fighter's missile (`true`).
    pub ranged: Option<bool>,
    /// A missile's own speed, blast and look (a pot thrown, a bomb lobbed).
    pub speed: Option<f32>,
    pub radius: Option<f32>,
    pub spell: Option<u32>,
    /// The take of the attack clip it plays (the model's second, by default).
    #[serde(default = "one_u32")]
    pub take: u32,
    /// The flash on the creature as it winds up (`SpellVisualKit`; 0 none).
    #[serde(default)]
    pub kit: u32,
    /// Where it lands, the look of this spell (see [`AttackDef::look`]).
    #[serde(default)]
    pub look: Option<u32>,
    /// Its own sound as it winds up (see [`AttackDef::sound`]).
    #[serde(default)]
    pub sound: Option<u32>,
    /// Basin Battle: its weapon sticks where this blow lands, and it hauls at it this long
    /// (seconds; 0: no), open to every blow: the best opening of a fight.
    #[serde(default)]
    pub lodge: f32,
    /// The look on the creature as the blow lands, hit or miss (`SpellVisualKit`; 0 none): a
    /// stomp's dust, a slam's debris. `kit` is the wind-up's tell; this is the impact.
    #[serde(default)]
    pub impact_kit: u32,
    /// Its shape flashed on the ground as it lands (a colour, `RRGGBB`), for the clients that draw
    /// one: what it covered reads with the floor markers off.
    #[serde(default)]
    pub burst: Option<String>,
}

fn one_u32() -> u32 {
    1
}

impl SpecialDef {
    /// The creature's usual blow with this one's changes.
    pub fn over(&self, a: &AttackDef) -> AttackDef {
        let mut b = a.clone();
        b.windup = self.windup.unwrap_or(a.windup);
        b.damage = self.damage.unwrap_or(a.damage);
        b.range = self.range.unwrap_or(a.range);
        b.arc = self.arc.unwrap_or(a.arc);
        b.recovery = self.recovery.unwrap_or(a.recovery);
        b.blockable = self.blockable.unwrap_or(a.blockable);
        b.heavy = self.heavy.unwrap_or(a.heavy);
        b.poise = self.poise.or(a.poise);
        b.ranged = self.ranged.unwrap_or(a.ranged);
        b.speed = self.speed.unwrap_or(a.speed);
        b.radius = self.radius.unwrap_or(a.radius);
        b.spell = self.spell.unwrap_or(a.spell);
        b.look = self.look.unwrap_or(a.look);
        b.sound = self.sound.unwrap_or(a.sound);
        b
    }
}

fn yes() -> bool {
    true
}

#[derive(Debug, Clone, Deserialize)]
pub struct MobDef {
    pub key: String,
    pub name: String,
    /// How it fights (a `[[temper]]`'s key; none: `[ai]` as it is).
    #[serde(default)]
    pub temper: String,
    pub display: u32,
    /// A creature on a character model (an orc, a Defias): every blow of its from this character
    /// model's clips (`"ScourgeFemale"`…) where the step names none, on a client that can.
    #[serde(default)]
    pub clips_from: Option<String>,
    #[serde(default = "one_f")]
    pub scale: f32,
    /// Its body's half-width at scale 1, where a blow reaches it from and where a foe stops
    /// (0: half a yard). A machine on legs stands wider than its middle.
    #[serde(default)]
    pub girth: f32,
    pub health: u32,
    pub run_speed: f32,
    pub walk_speed: f32,
    pub aggro: f32,
    pub leash: f32,
    #[serde(default)]
    pub weapon: u32,
    #[serde(default)]
    pub drops: BTreeMap<String, [u32; 2]>,
    pub respawn: f32,
    #[serde(default)]
    pub keep_away: f32,
    /// The line under its name (its clan or tribe).
    #[serde(default)]
    pub subname: String,
    pub attack: AttackDef,
    #[serde(default)]
    pub special: Option<SpecialDef>,
    /// Its attack patterns (souls-like strings of blows), one picked each time it attacks among
    /// those that suit how far off its foe is. With none, it swings its `attack` (and `special`).
    #[serde(default, rename = "pattern")]
    pub patterns: Vec<PatternDef>,
    /// What it holds: the weapon's grip (`1h`, `2h`, `staff`, `dagger`, `polearm`) and a shield on
    /// its arm (an item display).
    #[serde(default)]
    pub grip: String,
    #[serde(default)]
    pub shield: u32,
    /// A second weapon in the off hand (an item display: a dagger), and a bow (shown in its hands
    /// as it shoots).
    #[serde(default)]
    pub off_hand: u32,
    #[serde(default)]
    pub bow: u32,
    /// With a shield up between blows, the chance it blocks a light blow from in front.
    #[serde(default)]
    pub block: f32,
    /// The chance it steps back out of a heavy attack wound up at it.
    #[serde(default)]
    pub evade: f32,
    /// A champion: its health shown as a bar across the bottom of the screen to everyone near
    /// while it fights, and never more than one of it in a raid.
    #[serde(default)]
    pub boss: bool,
    /// Too big to parry (a giant beast, an elemental, a construct: a paw, a stomp, a slab of stone,
    /// not a blade): a timed guard only blocks its blows. Its blows are still blocked as ever.
    #[serde(default)]
    pub unparryable: bool,
    /// Its second wind: below a share of its health, a pattern at once and quicker blows after.
    #[serde(default)]
    pub enrage: Option<EnrageDef>,
    /// Its turns as it weakens (`[[mob.phase]]`, the enrage among them), highest share first:
    /// each a pattern at once, its pace and might from then, help called, a line said. New blows
    /// come with a pattern's `below`. Read in order once loaded (see [`Content::settle`]).
    #[serde(rename = "phase", default)]
    pub phases: Vec<EnrageDef>,
    /// A finite crew on ledges over the drop, each plank on a support that breaks, and a furnace
    /// valve (`sim::perches`).
    #[serde(default)]
    pub perch_crew: Option<PerchCrewDef>,
    /// Ordered equipment changes at a physical rack, using the health phases as milestones.
    #[serde(default)]
    pub arsenal: Option<ArsenalDef>,
    /// Its posture: this much damage within `posture_secs` breaks its stance (it reels, open to
    /// a riposte). 0: none.
    #[serde(default)]
    pub posture: u32,
    #[serde(default = "posture_secs")]
    pub posture_secs: f32,
    /// What the first one the camp kills leaves in the camp's chest (an item each, by key).
    #[serde(default)]
    pub trophy: BTreeMap<String, u32>,
    /// Below: what it holds and wears (item keys), now and then left where it falls (a boss's,
    /// always).
    #[serde(default)]
    pub gear: Vec<String>,
    /// What steps out of it when it falls (a mob key): a machine's driver, a second form. It
    /// stands in its place, in its pack, on its killer at once; `then_text` is said as it does.
    #[serde(default)]
    pub then: String,
    #[serde(default)]
    pub then_text: String,
    /// It lies dead this long before what steps out of it rises (0: at once): the fight seems won.
    #[serde(default)]
    pub then_after: f32,
    /// What steps out only if this is so (a secret below found, a boss beaten, or the item of this
    /// key in its killer's bags): a secret phase. Empty: always.
    #[serde(default)]
    pub then_needs: String,
    /// The flash on it as it rises (`SpellVisualKit`), and a spell's ground look at its feet.
    #[serde(default)]
    pub then_kit: u32,
    #[serde(default)]
    pub then_look: u32,
    /// A hunter: set on the one the heroes guard (an escort's walker), a hero's blow turns it on
    /// that hero for one string only, and then it goes back for its quarry. Without it, a creature
    /// drawn off is the hero's for good.
    #[serde(default)]
    pub hunts: bool,
    /// What it is, for the weapons that single out a kind: `undead`, `worgen`, `stone`,
    /// `mechanical`.
    #[serde(default)]
    pub tags: Vec<String>,
    /// A totem a hero sets down, or a training dummy: no mind of its own, it only stands there and
    /// is struck.
    #[serde(default)]
    pub totem: bool,
    #[serde(default)]
    pub dummy: bool,
    /// A look it always wears: a spell whose state look plays on it (a spirit beast's ghostly
    /// glow). 0: none.
    #[serde(default)]
    pub aura: u32,
    /// The cards the camp learns the first time it falls (a class's champion).
    #[serde(default)]
    pub teaches: Vec<String>,
    /// Harmless: it never fights back, and runs from a blow (a rabbit, a sheep).
    #[serde(default)]
    pub critter: bool,
    /// An ally of the heroes: it never takes up a fight against one, their blows and shots pass
    /// it by, and a heal aimed at a friend can land on it (its number floats over it). None in the
    /// rules yet; `.es ally` sets one down to try.
    #[serde(default)]
    pub friendly: bool,
    /// A friendly creature that shoots: the nearest foe within `range` that is fighting a hero,
    /// every `every` sec for `damage` (`sim::explore`, as a tower does). Only for a `friendly` one.
    #[serde(default)]
    pub shoots: Option<ShootDef>,
    /// Set down by a hero (a `totem`) yet a heal can reach it: help aimed at a friend lands on
    /// it as on an ally (a Priest's orb). A totem without it never takes a heal.
    #[serde(default)]
    pub healable: bool,
    /// Its voice (`SoundEntries.dbc`): as it turns on a foe, at its second wind, as it falls.
    #[serde(default)]
    pub aggro_sound: u32,
    #[serde(default)]
    pub enrage_sound: u32,
    #[serde(default)]
    pub death_sound: u32,
    /// Said over a hero it has just felled.
    #[serde(default)]
    pub slay_sound: u32,
    /// What it yells (a monster yell from where it stands, with a speech bubble) as it turns on a
    /// foe, over a hero it has just felled, and as it falls. Empty: nothing. A phase's `text` and a
    /// second form's `then_text` are yelled the same way.
    #[serde(default)]
    pub aggro_text: String,
    #[serde(default)]
    pub slay_text: String,
    #[serde(default)]
    pub death_text: String,
    /// Its voice as it begins a string up close (the first blow's wind-up), from where it stands,
    /// so a blow from behind is heard coming: a grunt, a shout. `heavy_sound` for a heavy first
    /// blow (else `windup_sound`). 0: none.
    #[serde(default)]
    pub windup_sound: u32,
    #[serde(default)]
    pub heavy_sound: u32,
}

/// Offsets are from the boss's original home, on that floor. The crew is encounter-owned:
/// it never becomes a respawning pack or leaves a second set behind after a wipe.
#[derive(Debug, Clone, Deserialize)]
pub struct PerchCrewDef {
    pub airborne: String,
    pub grounded: String,
    /// One engineer's ledge each, in turn order (the first is the one the furnace brings down).
    pub ledges: Vec<LedgeDef>,
    /// The furnace's own valve that he works.
    pub valve: [f32; 2],
    /// The furnace's mouth (x, y and height over his floor): it belches and glows.
    pub furnace: [f32; 3],
    /// The way the fire's channel runs across the floor from the valve (x, y; any length). The
    /// same every fight. Left out: straight out from the furnace, through the valve.
    #[serde(default)]
    pub channel: Option<[f32; 2]>,
}

/// A plank laid from a rail out over the drop, a support under it on the floor below.
#[derive(Debug, Clone, Deserialize)]
pub struct LedgeDef {
    /// Where the engineer stands on the plank: x, y, and his height over the boss's floor.
    pub at: [f32; 3],
    /// The way the plank juts out over the drop, from its rail (radians).
    pub out: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArsenalDef {
    /// Offset from the original home, projected onto that floor and checked for a walking path.
    pub rack: [f32; 2],
    pub display: u32,
    pub loadouts: Vec<ArsenalLoadout>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArsenalLoadout {
    pub weapon: u32,
    #[serde(default)]
    pub off_hand: u32,
    pub grip: String,
    pub patterns: Vec<String>,
    /// What he shouts as he lifts this loadout at the rack (empty: nothing).
    #[serde(default)]
    pub lift_text: String,
}

impl MobDef {
    /// It stands and is struck, nothing more (a totem, a dummy).
    pub fn inert(&self) -> bool {
        self.totem || self.dummy
    }

    /// Its body's half-width as drawn (`girth` at its scale; half a yard at its scale by default).
    pub fn body(&self) -> f32 {
        self.scale * if self.girth > 0.0 { self.girth } else { 0.5 }
    }
}

/// A creature's second wind (`[mob.enrage]`).
#[derive(Debug, Clone, Default, Deserialize)]
pub struct EnrageDef {
    /// At or below this share of its health.
    pub at: f32,
    /// Its wind-ups, recoveries and cooldowns are this share of what they were.
    #[serde(default = "one_f")]
    pub speed: f32,
    /// The pattern (by name) it starts at once, whatever it was doing.
    #[serde(default)]
    pub pattern: String,
    /// Said to everyone.
    #[serde(default)]
    pub text: String,
    /// It calls these to its side (creature keys), on its foe at once.
    #[serde(default)]
    pub summon: Vec<String>,
    /// Its blows land this much harder from then (1: as they were).
    #[serde(default = "one_f")]
    pub might: f32,
    /// The flash on it as it turns (`SpellVisualKit`; 0: none), a spell's ground look at its feet
    /// (a burst, a shockwave), and its voice (`SoundEntries`; 0: its `enrage_sound`).
    #[serde(default)]
    pub kit: u32,
    #[serde(default)]
    pub look: u32,
    #[serde(default)]
    pub sound: u32,
    /// It throws down every hero this near as it turns (0: none).
    #[serde(default)]
    pub shove: f32,
    /// Basin Battle's boss beats (`sim::bosses`). The boss breaks off and runs along these points
    /// (by the land's ways between them), heeding nobody, and fights again where the last one is
    /// once a hero has come within `wait_for` yards of it (its home from then: a lost fight
    /// puts it back where it began). Empty: it stays.
    #[serde(default)]
    pub run_to: Vec<[f32; 3]>,
    /// On the run, rocks shaken loose behind it this often (seconds; 0: none): dust first, then
    /// the fall, a light blow.
    #[serde(default)]
    pub run_falls: f32,
    #[serde(default)]
    pub wait_for: f32,
    /// Yelled, and its flash on it, as it takes up the fight again at the run's end.
    #[serde(default)]
    pub arrive_text: String,
    #[serde(default)]
    pub arrive_kit: u32,
    /// From this phase on its heavy blows leave burning ground where they land.
    #[serde(default)]
    pub burn: Option<BurnDef>,
    /// Rocks brought down as it turns: dust over each of these points, then the fall a moment
    /// later (a heavy blow), and a dry way left between them.
    #[serde(default)]
    pub debris: Vec<[f32; 3]>,
    /// Rocks brought down round it, wherever it stands, this far out (0: none): six falls in a
    /// ring, dust first.
    #[serde(default)]
    pub rocks_round: f32,
    /// The place coming down behind the heroes (`sim::bosses`'s collapse): section after section
    /// along a route, each left full of rubble behind a face of it.
    #[serde(default)]
    pub collapse: Option<CollapseDef>,
    /// How the phase begins on the creature's own body (`sim::bosses`'s slam): it runs to a wall
    /// and drives itself into it, the place shakes for everyone in it and rock comes loose.
    #[serde(default)]
    pub slam: Option<SlamDef>,
}

/// A phase begun with the creature's own body (`[mob.phase.slam]`): from the yell it runs to the
/// frame of the collapse's first door if that is near enough, else to the nearest wall, swings
/// into it and strikes it `secs` after the yell. Nothing stops it; it can be hit all the while.
#[derive(Debug, Clone, Deserialize)]
pub struct SlamDef {
    /// Seconds from the yell to the blow on the wall.
    #[serde(default = "slam_secs")]
    pub secs: f32,
    /// The clip it swings with (the model's own), its take, how long into the clip its blow
    /// lands, and how long the clip runs on after that.
    pub clip: u16,
    #[serde(default)]
    pub take: u32,
    pub swing: f32,
    #[serde(default = "slam_recover")]
    pub recover: f32,
    /// How far it will go for a wall (yards), how far from the wall it stands to strike it, and
    /// from how far it makes for the first door's frame instead (the seconds it would take at
    /// its own run).
    #[serde(default = "slam_reach")]
    pub reach: f32,
    #[serde(default = "slam_stand")]
    pub stand: f32,
    #[serde(default = "slam_door_secs")]
    pub door_secs: f32,
    /// The rock that comes loose from the blow, over `volley` seconds: this many for show round
    /// it and at the door (they hurt nobody), and one marked on the floor under each hero within
    /// `near` of it, which strikes whoever stays for `share` of its own blow.
    #[serde(default = "slam_volley")]
    pub volley: f32,
    #[serde(default = "slam_rocks")]
    pub rocks: usize,
    #[serde(default = "slam_door_rocks")]
    pub door_rocks: usize,
    #[serde(default = "slam_near")]
    pub near: f32,
    #[serde(default = "slam_share")]
    pub share: f32,
}

impl Default for SlamDef {
    /// The ogre's overhead blow (`Special2H`, landing 0.8 s in), and every number its own default.
    fn default() -> Self {
        SlamDef {
            secs: slam_secs(),
            clip: 58,
            take: 0,
            swing: 0.8,
            recover: slam_recover(),
            reach: slam_reach(),
            stand: slam_stand(),
            door_secs: slam_door_secs(),
            volley: slam_volley(),
            rocks: slam_rocks(),
            door_rocks: slam_door_rocks(),
            near: slam_near(),
            share: slam_share(),
        }
    }
}

fn slam_secs() -> f32 {
    2.0
}
fn slam_recover() -> f32 {
    0.7
}
fn slam_reach() -> f32 {
    12.0
}
fn slam_stand() -> f32 {
    2.2
}
fn slam_door_secs() -> f32 {
    2.5
}
fn slam_volley() -> f32 {
    1.5
}
fn slam_rocks() -> usize {
    9
}
fn slam_door_rocks() -> usize {
    4
}
fn slam_near() -> f32 {
    30.0
}
fn slam_share() -> f32 {
    0.6
}

/// A collapse (`[mob.phase.collapse]`): the place comes down in `sections` along `route`, which
/// runs from the buried end (behind the boss) the way the heroes are to go. Each section goes in
/// a surge: `warn` seconds of warning (a bright band across the section's front edge, dust from
/// the roof), then a wave of falling rock from its back edge to its front edge at `speed` yards a
/// second, which leaves a face of rubble across that front edge, wall to wall; then the front
/// holds still for `rest` seconds and the next section goes. A hero behind the wave or behind a
/// face, however far to the side (`reach`), is caught: `catch` of their health gone once a surge,
/// thrown onto clear floor `throw` yards past the surge's line, down for `down` seconds. Nobody is
/// left behind the rubble. For `lead` yards ahead of the wave lighter rocks come down. The boss
/// fights on through it and can't be brought below `hold` of its health until the last section is
/// down, where it yells `end_text` and the last of the fight is fought.
#[derive(Debug, Clone, Deserialize)]
pub struct CollapseDef {
    pub route: Vec<[f32; 3]>,
    pub speed: f32,
    #[serde(default = "collapse_lead")]
    pub lead: f32,
    #[serde(default)]
    pub hold: f32,
    #[serde(default)]
    pub end_text: String,
    /// Yelled the first time a blow would have brought it below `hold` before the end.
    #[serde(default)]
    pub held_text: String,
    /// What shows on it while it is held: a spell whose state the clients play, on it from the
    /// first blow refused until the last section is down (0: none), and a flash on it for each
    /// blow refused (`SpellVisualKit`; 0: none).
    #[serde(default)]
    pub held_aura: u32,
    #[serde(default)]
    pub held_kit: u32,
    /// The end of the collapse on it: a flash as the last section lands (`SpellVisualKit`), a
    /// spell whose state it wears from then until it falls, and its size from then (0: as it
    /// was).
    #[serde(default)]
    pub end_kit: u32,
    #[serde(default)]
    pub end_aura: u32,
    #[serde(default)]
    pub end_grow: f32,
    /// The sections, in the order they come down. None: the whole route is one, its face a little
    /// short of the route's end.
    #[serde(rename = "section", default)]
    pub sections: Vec<CollapseSectionDef>,
    #[serde(default = "collapse_warn")]
    pub warn: f32,
    #[serde(default = "collapse_rest")]
    pub rest: f32,
    #[serde(default = "collapse_catch")]
    pub catch: f32,
    #[serde(default = "collapse_throw")]
    pub throw: f32,
    #[serde(default = "collapse_down")]
    pub down: f32,
    /// The collapse wrecks the place as it goes: from its blow to the end of the fight no lamp
    /// can be rested at (they stand dark), each section down lies as wreck (the server lays the
    /// ruin's own pieces there), and other things than rock come down in its wave. Off: the
    /// collapse as it was, rock alone.
    #[serde(default)]
    pub wreck: bool,
    /// How many sections are down when the ship in the dry dock is gone, and with it (on the
    /// clients as they are) the whole mine's wrecked look. 0: with the first blow.
    #[serde(default = "collapse_wreck_ship")]
    pub wreck_ship: usize,
    /// The route's lanterns go out section by section (client patch 0096, a client of this round
    /// only: an older one has its lanterns as ever). Off: no lantern is asked to go out.
    #[serde(default)]
    pub wreck_lamps: bool,
    /// With `wreck_lamps`: by section, the mine's own lanterns (their place in its WMO) that go
    /// out when that section is down. A section past the list puts out none.
    #[serde(default)]
    pub wreck_lamp_sections: Vec<Vec<u32>>,
}

impl Default for CollapseDef {
    fn default() -> Self {
        CollapseDef {
            route: Vec::new(),
            speed: 5.0,
            lead: collapse_lead(),
            hold: 0.0,
            end_text: String::new(),
            held_text: String::new(),
            held_aura: 0,
            held_kit: 0,
            end_kit: 0,
            end_aura: 0,
            end_grow: 0.0,
            sections: Vec::new(),
            warn: collapse_warn(),
            rest: collapse_rest(),
            catch: collapse_catch(),
            throw: collapse_throw(),
            down: collapse_down(),
            wreck: false,
            wreck_ship: collapse_wreck_ship(),
            wreck_lamps: false,
            wreck_lamp_sections: Vec::new(),
        }
    }
}

/// One section of a collapse: `face` is its front edge, the line its rubble is built across, from
/// one wall to the other (the route crosses it). A hero within `reach` yards of the route there
/// (a side cave, a pocket) is in the section. `height` is the roof over the floor at the face: how
/// high the rubble is built.
#[derive(Debug, Clone, Deserialize)]
pub struct CollapseSectionDef {
    pub face: [[f32; 2]; 2],
    #[serde(default = "collapse_reach")]
    pub reach: f32,
    #[serde(default = "collapse_height")]
    pub height: f32,
}

fn collapse_wreck_ship() -> usize {
    1
}

fn collapse_lead() -> f32 {
    12.0
}

fn collapse_warn() -> f32 {
    2.0
}

fn collapse_rest() -> f32 {
    12.0
}

fn collapse_catch() -> f32 {
    0.35
}

fn collapse_throw() -> f32 {
    4.0
}

fn collapse_down() -> f32 {
    1.0
}

pub(crate) fn collapse_reach() -> f32 {
    20.0
}

pub(crate) fn collapse_height() -> f32 {
    8.0
}

/// Burning ground a boss's heavy blow leaves (`[mob.phase.burn]`): how wide, what it does each
/// second to a hero standing in it, how long it burns, and its look (a ground spell, played only
/// for a patch wide enough for one: a smaller one shows by its drawn edge and its flames).
/// `aura`: what the boss wears from then (a spell whose state the clients play; 0: nothing).
#[derive(Debug, Clone, Deserialize)]
pub struct BurnDef {
    pub radius: f32,
    pub damage: u32,
    pub secs: f32,
    #[serde(default)]
    pub look: u32,
    #[serde(default)]
    pub aura: u32,
}

/// One of the journal's goals: done when the camp has `kind` (`bank`, `build`, `craft`, `kill`,
/// `gather`, `raid`, `wear`) of `what` `count` times over (a raid by its place in the list, from 0;
/// `wear`, an armor slot someone wears).
#[derive(Debug, Clone, Deserialize)]
pub struct GoalDef {
    pub title: String,
    pub text: String,
    pub kind: String,
    pub what: String,
    #[serde(default = "one")]
    pub count: u32,
    #[serde(default)]
    pub reward: BTreeMap<String, u32>,
}

impl GoalDef {
    pub const KINDS: [&'static str; 15] = ["bank", "build", "craft", "kill", "gather", "raid", "wear", "clear", "delve", "explore", "open", "secret", "learn", "cleared", "chose"];
}

/// A string of blows a creature commits to: each a step (its clip, wind-up, reach, damage...)
/// over the creature's usual attack, the next coming after the last's recovery.
#[derive(Debug, Clone, Deserialize)]
pub struct PatternDef {
    #[serde(default)]
    pub name: String,
    /// How often it's picked against the others that suit (a weight).
    #[serde(default = "one_f")]
    pub weight: f32,
    /// Only with its foe at least this near and no farther than this (0: the first blow's reach
    /// plus its lunge).
    #[serde(default)]
    pub min_range: f32,
    #[serde(default)]
    pub max_range: f32,
    /// Only once its health is at or below this share (a later phase's blows); a weight of 0
    /// keeps it for the enrage alone.
    #[serde(default = "one_f")]
    pub below: f32,
    /// A parry of any blow of the string ends it, for a boss too (whose parries only fill its
    /// posture: `[parry] posture`). "Parry the first and the rest never come": the creature
    /// doesn't reel (so a group can't parry it into reeling forever), it just loses the string.
    #[serde(default)]
    pub parry_ends: bool,
    pub steps: Vec<SpecialDef>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PackDef {
    pub mob: String,
    pub at: [f32; 2],
    /// The floor it stands on, where there are floors above floors (the top one if not given).
    #[serde(default)]
    pub z: Option<f32>,
    pub radius: f32,
    pub count: usize,
    #[serde(default)]
    pub link: f32,
    /// A patrol: the pack walks these points in turn and back (from `at`), waiting at each.
    #[serde(default)]
    pub route: Vec<[f32; 2]>,
    #[serde(default = "patrol_wait")]
    pub wait: f32,
    /// The Horde camp it holds (`[[camp]]`), if any.
    #[serde(default)]
    pub camp: String,
    /// Known by (a Deadmines pack: a boss's, or one that stays dead once beaten).
    #[serde(default)]
    pub key: String,
    /// Once beaten, it stays dead for good (an elite; a boss always does).
    #[serde(default)]
    pub elite: bool,
    /// Asleep until a hero comes this near (an ambush that steps out of the dark), 0: always awake.
    #[serde(default)]
    pub hidden: f32,
    /// Unseen until this chest (by key) is opened, then on its opener at once (an ambush).
    #[serde(default)]
    pub wake: String,
    /// Each time it comes, at one of these spots (picked at random), not `at`: a wanderer.
    #[serde(default)]
    pub spots: Vec<[f32; 2]>,
    /// A rare creature: its fall is told to everyone.
    #[serde(default)]
    pub rare: bool,
    /// It wakes (on its chest's opener, a `wake` pack) only if this holds (a condition as a prop's
    /// `after`: `chose:remy:arrest`); otherwise it stays asleep. Any other pack stays unseen (and an
    /// ambusher lying in wait never steps out) until it holds, and stands from then on.
    #[serde(default)]
    pub after: Option<String>,
}

/// Groups of creatures spread over the whole land (`[[roam]]`): `groups` of them, each of one of
/// `mobs` (picked in turn), `size` strong, a spot tried every `spread` yards, `spacing` apart,
/// none within `clear_of_camp` of the camp's heart. A world lays its own.
#[derive(Debug, Clone, Deserialize)]
pub struct RoamDef {
    pub mobs: Vec<String>,
    pub size: [usize; 2],
    pub groups: usize,
    #[serde(default = "roam_spread")]
    pub spread: f32,
    pub spacing: f32,
    #[serde(default = "roam_radius")]
    pub radius: f32,
    #[serde(default = "roam_clear")]
    pub clear_of_camp: f32,
    /// By the trees (wolves, prowlers), else out in the open.
    #[serde(default)]
    pub by_trees: bool,
}

fn roam_spread() -> f32 {
    80.0
}

fn roam_radius() -> f32 {
    5.0
}

fn roam_clear() -> f32 {
    110.0
}

/// One of Elwynn's places: found by the first hero to come within `radius` (the camp's, for good),
/// worth `reward` into the chest then, and marked on everyone's world map with what's there.
#[derive(Debug, Clone, Deserialize)]
pub struct SiteDef {
    pub key: String,
    pub name: String,
    pub at: [f32; 2],
    #[serde(default = "site_radius")]
    pub radius: f32,
    /// What it is, for its mark: `camp`, `town`, `horde`, `foes`, `sanctum`, `lair`, `landmark`,
    /// `arena`, `fishing`, `mine`.
    pub kind: String,
    /// What's there, in a line or two (on the map's tooltip).
    #[serde(default)]
    pub text: String,
    /// Its mark's icon (`Interface\Icons\...`); by its kind when not given.
    #[serde(default)]
    pub icon: String,
    #[serde(default)]
    pub reward: BTreeMap<String, u32>,
}

fn site_radius() -> f32 {
    40.0
}

/// A chest out in the land (or a cache, a relic, a shrine): opened with the use key, its `loot`
/// into the opener's bags (and `roll`'s random extras), the cards it `teaches` learned by the
/// whole camp. A `secret` one is unseen until a hero comes that near; `after` keeps it shut until
/// the camp has killed that creature (a key), or meets a condition (`open:<chest>`, `found:<site>`,
/// `kill:<mob>`); `ambush` wakes those packs (by key) as it's opened; `keep`
/// leaves it standing (a shrine), spent; `respawn` fills it again after that many seconds of real
/// time (0: once a world).
#[derive(Debug, Clone, Deserialize)]
pub struct ChestDef {
    pub key: String,
    pub name: String,
    pub at: [f32; 2],
    #[serde(default)]
    pub z: Option<f32>,
    #[serde(default = "chest_display")]
    pub display: u32,
    #[serde(default = "one_f")]
    pub scale: f32,
    #[serde(default)]
    pub facing: f32,
    #[serde(default)]
    pub loot: BTreeMap<String, u32>,
    #[serde(default)]
    pub roll: BTreeMap<String, [u32; 2]>,
    #[serde(default)]
    pub teaches: Vec<String>,
    #[serde(default)]
    pub secret: f32,
    #[serde(default)]
    pub after: String,
    /// Said while it's shut for `after`.
    #[serde(default)]
    pub locked: String,
    #[serde(default)]
    pub ambush: Vec<String>,
    /// A trial: a shrine's or a relic's (or a hold's, below) guardians (packs by key that wake on it), called up
    /// each time a hero who has something to learn uses it; it teaches everyone near (and its
    /// first finder what's in it) only once they've all fallen.
    #[serde(default)]
    pub trial: Vec<String>,
    /// A trial that is a hold: seconds a hero must stay near, its waves (`trial` entries, each a
    /// pack or several joined with `+`) called up one after another over that time, the chest
    /// opened for whoever opened it once the time has run and the last wave has fallen. Left
    /// at 0, every wave is called at once. `wave_text` is said as each wave comes; `hold_title`,
    /// `held_title` and `lost_text` are the banners and the news as it begins, is held, or is
    /// lost (a default from the chest's name when empty).
    #[serde(default)]
    pub hold: f32,
    #[serde(default)]
    pub wave_text: Vec<String>,
    #[serde(default)]
    pub hold_title: String,
    #[serde(default)]
    pub held_title: String,
    #[serde(default)]
    pub lost_text: String,
    /// An escort (a hold that walks): the key of a pack of one friendly creature that wakes on the
    /// chest and walks `route` (from its pack's `at`) while a hero is near it and nothing is fighting
    /// it. Wave `i` of `n` comes as it sets off from point `i * len / n` of the route, and comes for
    /// it, not for the heroes; it is passed once it reaches the end and every wave has fallen, and
    /// lost if it is struck down. Empty: no escort.
    #[serde(default)]
    pub escort: String,
    #[serde(default)]
    pub route: Vec<[f32; 2]>,
    #[serde(default)]
    pub keep: bool,
    #[serde(default)]
    pub respawn: f32,
    /// Seconds of work to open it (a lock picked, a cairn's stones moved).
    #[serde(default = "chest_secs")]
    pub secs: f32,
    /// Said as it's opened (its story).
    #[serde(default)]
    pub text: String,
}

/// A thing only to be seen (`[[prop]]`): a camp's fire, a cult's candles, a cage; or one of
/// the camp's own people (`npc`), standing at their post.
///
/// The camp's own props grow with it: one with `after` stands only once the camp has got that
/// far (`raids:1` one raid beaten, `built:smithy` a smithy built once, `goal:6` the journal at
/// its seventh step, `found:<site>` a place found), and stays from then on.
#[derive(Debug, Clone, Deserialize)]
pub struct PropDef {
    /// Its look: a game object's (`GameObjectDisplayInfo`), or for an `npc` a creature's
    /// (`CreatureDisplayInfo`).
    pub display: u32,
    pub at: [f32; 2],
    #[serde(default)]
    pub z: Option<f32>,
    /// Raised this far over the ground it stands on (a bell on its post, a book on its table).
    #[serde(default)]
    pub lift: f32,
    #[serde(default = "one_f")]
    pub scale: f32,
    #[serde(default)]
    pub facing: f32,
    /// How far the camp must have come for it to stand (see above); none: from the start.
    #[serde(default)]
    pub after: Option<String>,
    /// It goes once the camp has come this far (a condition as `after`: `chose:remy:arrest`): a
    /// person taken away, a thing cleared. It stays gone, and never stands again.
    #[serde(default)]
    pub until: Option<String>,
    /// Ground it keeps: nothing is built within this of it (0: none).
    #[serde(default)]
    pub clear: f32,
    /// One of the camp's people: a creature that stands at its post (friendly, never fought).
    #[serde(default)]
    pub npc: bool,
    #[serde(default)]
    pub name: String,
    /// Under an npc's name (their trade).
    #[serde(default)]
    pub subname: String,
    /// What the camp hears when it first stands (the first of a tier says how the camp grows).
    #[serde(default)]
    pub say: String,
    /// What an `npc` says when used (the use key), as the camp grows: the last line whose `after`
    /// holds is the one read. None: the person only stands there.
    #[serde(default)]
    pub talk: Vec<TalkDef>,
}

/// One thing a camp's person says, once the camp has come as far as its `after`.
#[derive(Debug, Clone, Deserialize)]
pub struct TalkDef {
    /// How far the camp must have come (as a prop's `after`: `raids:N`, `built:<station>`,
    /// `goal:N`, `found:<site>`, `kill:<mob>`, `open:<chest>`); none: always.
    #[serde(default)]
    pub after: Option<String>,
    pub text: String,
    /// Links under the words, each `command~label` (a dungeon's lore page: `.es lore deadmines~The Deadmines`).
    #[serde(default)]
    pub links: Vec<String>,
}

/// The conditions a prop's or a line's `after` may name, each with what follows its colon.
pub const CAMP_CONDITIONS: [&str; 8] = ["start", "raids", "goal", "built", "found", "kill", "open", "chose"];

fn chest_display() -> u32 {
    259
}

fn chest_secs() -> f32 {
    1.5
}

/// A Horde camp: taken when every one of its packs is dead, and held that long before they come
/// back; the first time, a reward into the chest.
#[derive(Debug, Clone, Deserialize)]
pub struct CampDef {
    pub key: String,
    /// In words: "the Fargodeep mine yard".
    pub name: String,
    /// The banner's: "Fargodeep Mine Yard Taken".
    pub title: String,
    pub hold: f32,
    #[serde(default)]
    pub reward: BTreeMap<String, u32>,
}

fn posture_secs() -> f32 {
    6.0
}

fn patrol_wait() -> f32 {
    8.0
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaiderDef {
    pub mob: String,
    pub count: usize,
}

/// The Proving Ring: a ring out in the land, reached by a page of the book at camp, where foes
/// come in waves that never end, each worth more than the last and tougher, a champion every so
/// often. A run ends when its last hero falls, leaves by the banner or walks out.
#[derive(Debug, Clone, Deserialize)]
pub struct ArenaDef {
    /// "the Proving Ring" (in sentences), and the book's page for it.
    pub name: String,
    pub page: String,
    /// Its middle and how far it reaches; heroes arrive on the side `facing` looks away from,
    /// facing in, the banner (the way out) behind them.
    pub at: [f32; 2],
    pub radius: f32,
    pub facing: f32,
    /// Seconds before the first wave, and between a wave beaten and the next.
    pub first_gap: f32,
    pub gap: f32,
    /// The share of their health heroes get back as a wave is beaten.
    pub mend: f32,
    /// What the first wave is worth (in foes' `cost`), how much more each after, and the share
    /// more for each hero past the first.
    pub budget: f32,
    pub growth: f32,
    pub per_hero: f32,
    /// The most foes in one wave (past it, they're only tougher).
    pub most: usize,
    /// The share more health foes have for each wave past the first.
    pub toughen: f32,
    /// A champion every this many waves, in this order (round again after the last), with half
    /// the wave's worth beside him.
    pub boss_every: u32,
    pub bosses: Vec<String>,
    /// Who comes: each from its wave on, worth its cost.
    pub roster: Vec<ArenaFoeDef>,
    /// Into the chest for each champion's wave beaten.
    #[serde(default)]
    pub reward: BTreeMap<String, u32>,
}

/// Siege (`[siege]` in siege.toml): heroes hold camps round Arathi Basin, each round its Hearth,
/// against a base wave everyone faces and the creatures their rivals buy to send; each send
/// raises the sender's income. The last Hearth standing wins (the healthiest after the last wave).
#[derive(Debug, Clone, Deserialize)]
pub struct SiegeDef {
    /// "the Siege of Arathi Basin" (in sentences), the book's page for it, and what it is.
    pub name: String,
    pub page: String,
    #[serde(default)]
    pub text: String,
    /// Where heroes wait for a match, looking this way; the contested middle.
    pub lobby: [f32; 3],
    pub lobby_facing: f32,
    pub middle: [f32; 3],
    /// Seconds: a full match's countdown, before the first wave, each wave, the warning of what's
    /// coming, after the end.
    pub countdown: f32,
    pub first_gap: f32,
    pub wave_secs: f32,
    pub warn_secs: f32,
    pub over_secs: f32,
    pub waves: u32,
    pub start_gold: u32,
    pub start_income: u32,
    /// A foe's own `health` times this: the gold for felling it.
    pub kill_gold: f32,
    /// The share of a hero's gold the rival who fells them takes.
    pub steal: f32,
    pub build_radius: f32,
    pub gate_walk: f32,
    pub respawn_secs: f32,
    /// The Hearth's station.
    pub hearth: String,
    /// A defence's price in gold, by station.
    pub build: BTreeMap<String, u32>,
    /// A board card's price in gold, by its row.
    pub cards: BTreeMap<String, u32>,
    /// The base wave: its first's worth, more each after, the share more for each hero past the
    /// first in the camp, the most at once, the share more health each wave past the first.
    pub budget: f32,
    pub growth: f32,
    pub per_hero: f32,
    pub most: usize,
    pub toughen: f32,
    /// A match's foes have this share of their usual health, and their blows this share of
    /// their usual damage (heroes in a match start with two cards).
    #[serde(default = "one_f")]
    pub foe_health: f32,
    #[serde(default = "one_f")]
    pub foe_damage: f32,
    /// The most creatures a hero can send for one wave: this many, and `send_cap_growth` more
    /// each wave on.
    #[serde(default = "send_cap")]
    pub send_cap: u32,
    #[serde(default)]
    pub send_cap_growth: u32,
    pub vein_gold: u32,
    /// What each hero in a standing camp is given at every wave's start (food and salves: the
    /// only way to them in a match), up to `supply_most` of each in the bags.
    #[serde(default)]
    pub supply: BTreeMap<String, u32>,
    #[serde(default = "supply_most")]
    pub supply_most: u32,
    /// Everyone in the match ready (F at their Hearth, or `.siege ready`): the next wave comes in
    /// this many seconds, if it wasn't coming sooner.
    #[serde(default = "ready_secs")]
    pub ready_secs: f32,
    #[serde(rename = "base")]
    pub bases: Vec<SiegeBaseDef>,
    #[serde(rename = "mode")]
    pub modes: Vec<SiegeModeDef>,
    pub roster: Vec<ArenaFoeDef>,
    #[serde(rename = "send")]
    pub sends: Vec<SendDef>,
    /// The match's nodes (the middle's veins), in place of the camp's.
    #[serde(rename = "node", default)]
    pub nodes: Vec<NodeDef>,
}

/// A camp in a Siege: where its Hearth stands, looking toward the middle.
#[derive(Debug, Clone, Deserialize)]
pub struct SiegeBaseDef {
    pub key: String,
    pub name: String,
    pub at: [f32; 3],
    pub facing: f32,
}

/// A way to play: these camps, this many heroes to each.
#[derive(Debug, Clone, Deserialize)]
pub struct SiegeModeDef {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub text: String,
    pub bases: Vec<String>,
    pub per_base: usize,
}

/// A send on the mercenary board: `count` of `mob` for `cost` gold, `income` more a wave for the
/// rest of the match, from wave `from`; `once`, one per hero per match.
#[derive(Debug, Clone, Deserialize)]
pub struct SendDef {
    pub key: String,
    pub mob: String,
    pub count: u32,
    pub cost: u32,
    pub income: u32,
    pub from: u32,
    #[serde(default)]
    pub once: bool,
    #[serde(default)]
    pub text: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArenaFoeDef {
    pub mob: String,
    pub from: u32,
    pub cost: f32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RaidDef {
    /// The first raid's name for the banner.
    #[serde(default = "first_raid")]
    pub title: String,
    pub needs: Vec<String>,
    pub warning: f32,
    pub from: [f32; 2],
    /// Where they come from, in words.
    #[serde(default = "first_road")]
    pub road: String,
    pub raiders: Vec<RaiderDef>,
    pub wave_size: usize,
    pub wave_gap: f32,
    pub rout_secs: f32,
    pub reward: BTreeMap<String, u32>,
    /// The raids that come after the first is beaten, in order; once the last is beaten it comes
    /// again, a raider more of each kind every time.
    #[serde(default, rename = "tier")]
    pub tiers: Vec<RaidTierDef>,
}

fn first_raid() -> String {
    "Kobold Raid".into()
}

fn first_road() -> String {
    "the south-west road".into()
}

/// A later raid: what it needs built, who comes and from where, and what beating it is worth.
#[derive(Debug, Clone, Deserialize)]
pub struct RaidTierDef {
    pub title: String,
    pub needs: Vec<String>,
    pub from: [f32; 2],
    pub road: String,
    pub raiders: Vec<RaiderDef>,
    #[serde(default)]
    pub wave_size: Option<usize>,
    #[serde(default)]
    pub warning: Option<f32>,
    pub reward: BTreeMap<String, u32>,
}

/// A raid as it comes: one of the list, filled in from the first's defaults.
#[derive(Debug, Clone)]
pub struct RaidPlan {
    pub tier: usize,
    pub title: String,
    pub needs: Vec<String>,
    pub from: [f32; 2],
    pub road: String,
    pub raiders: Vec<(String, usize)>,
    pub wave_size: usize,
    pub warning: f32,
    pub reward: BTreeMap<String, u32>,
}

impl RaidDef {
    /// How many raids there are before the last repeats.
    pub fn count(&self) -> usize {
        1 + self.tiers.len()
    }

    /// The raid that comes after `won` have been beaten.
    pub fn plan(&self, won: u32) -> RaidPlan {
        let last = self.count() - 1;
        let tier = (won as usize).min(last);
        let extra = (won as usize).saturating_sub(last);
        let mut plan = if tier == 0 {
            RaidPlan {
                tier,
                title: self.title.clone(),
                needs: self.needs.clone(),
                from: self.from,
                road: self.road.clone(),
                raiders: self.raiders.iter().map(|r| (r.mob.clone(), r.count)).collect(),
                wave_size: self.wave_size,
                warning: self.warning,
                reward: self.reward.clone(),
            }
        } else {
            let t = &self.tiers[tier - 1];
            RaidPlan {
                tier,
                title: t.title.clone(),
                needs: t.needs.clone(),
                from: t.from,
                road: t.road.clone(),
                raiders: t.raiders.iter().map(|r| (r.mob.clone(), r.count)).collect(),
                wave_size: t.wave_size.unwrap_or(self.wave_size),
                warning: t.warning.unwrap_or(self.warning),
                reward: t.reward.clone(),
            }
        };
        // Beaten already: harder each time it comes back.
        for r in plan.raiders.iter_mut() {
            r.1 += extra;
        }
        if extra > 0 {
            plan.title = format!("{} ({})", plan.title, extra + 1);
        }
        plan
    }
}

impl Content {
    /// The built-in rules as a dev world has them (every card owned), for tests.
    #[cfg(test)]
    pub fn dev() -> Self {
        Self::into_dev(Self::builtin())
    }

    /// Any rules (the solo sim's tuned ones) as a dev world has them.
    #[cfg(test)]
    pub fn into_dev(mut c: Self) -> Self {
        c.dev.full_tree = true;
        c.work = WorkDef::instant();
        c.combat.foe_health = 1.0;
        c.combat.foe_damage = 1.0;
        c.combat.hero_health = 0.0;
        c.combat.hero_damage = 0.0;
        c
    }

    pub fn builtin() -> Self {
        Self::parse_with(DEFAULT, DEFAULT_DUNGEON).expect("the built-in survival.toml and deadmines.toml are valid")
    }

    /// The built-in dungeon files beside `deadmines.toml` (name and text).
    pub fn builtin_dungeons() -> Vec<(String, String)> {
        MORE_DUNGEONS.iter().map(|(n, t)| (n.to_string(), t.to_string())).collect()
    }

    /// `survival.toml` alone (no Deadmines).
    pub fn parse(text: &str) -> Result<Self> {
        Self::parse_with(text, "")
    }

    /// `survival.toml` with `deadmines.toml` beside it: the second's lists join the first's, and
    /// its tables are added (a table in both is a mistake). The other built-in dungeons with them.
    pub fn parse_with(text: &str, dungeon: &str) -> Result<Self> {
        Self::parse_full(text, dungeon, &Self::builtin_dungeons(), DEFAULT_SIEGE)
    }

    /// `survival.toml` with `deadmines.toml` and `siege.toml` beside it (and the built-in others).
    pub fn parse_all(text: &str, dungeon: &str, siege: &str) -> Result<Self> {
        Self::parse_full(text, dungeon, &Self::builtin_dungeons(), siege)
    }

    /// `survival.toml` with `deadmines.toml`, the other dungeons' files (each's `[dungeon]` a
    /// dungeon of its own, its lists joining the rest), and `siege.toml`.
    pub fn parse_full(text: &str, dungeon: &str, more: &[(String, String)], siege: &str) -> Result<Self> {
        let mut base: toml::Table = toml::from_str(text).context("reading survival.toml")?;
        let mut files: Vec<(String, String)> = vec![("deadmines.toml".into(), dungeon.into())];
        for (name, t) in more {
            let mut tab: toml::Table = toml::from_str(t).with_context(|| format!("reading {name}"))?;
            if let Some(d) = tab.remove("dungeon") {
                match base.get_mut("more_dungeon") {
                    Some(toml::Value::Array(a)) => a.push(d),
                    _ => {
                        base.insert("more_dungeon".into(), toml::Value::Array(vec![d]));
                    }
                }
            }
            files.push((name.clone(), toml::to_string(&tab).with_context(|| format!("rewriting {name}"))?));
        }
        files.push(("siege.toml".into(), siege.into()));
        for (name, extra) in &files {
            if extra.trim().is_empty() {
                continue;
            }
            let more: toml::Table = toml::from_str(extra).with_context(|| format!("reading {name}"))?;
            for (k, v) in more {
                match (base.get_mut(&k), v) {
                    (Some(toml::Value::Array(a)), toml::Value::Array(b)) => a.extend(b),
                    (None, v) => {
                        base.insert(k, v);
                    }
                    _ => bail!("{name}: {k} is in survival.toml already"),
                }
            }
        }
        let mut c: Content = toml::Value::Table(base)
            .try_into()
            .context("reading the rules")?;
        c.settle();
        c.hone();
        c.validate()?;
        c.source = text.to_string();
        c.dungeon_source = dungeon.to_string();
        c.more_sources = more.to_vec();
        Ok(c)
    }

    /// A `survival.toml` file, with the `deadmines.toml` beside it (the built-in one if none).
    pub fn load(path: &std::path::Path) -> Result<Self> {
        let text = std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
        let beside = path.with_file_name("deadmines.toml");
        let dungeon =
            std::fs::read_to_string(&beside).unwrap_or_else(|_| DEFAULT_DUNGEON.to_string());
        let siege = std::fs::read_to_string(path.with_file_name("siege.toml")).unwrap_or_else(|_| DEFAULT_SIEGE.to_string());
        // The other dungeons: each file beside it, else the built-in one.
        let more: Vec<(String, String)> = MORE_DUNGEONS
            .iter()
            .map(|(n, t)| (n.to_string(), std::fs::read_to_string(path.with_file_name(n)).unwrap_or_else(|_| t.to_string())))
            .collect();
        let mut c = Self::parse_full(&text, &dungeon, &more, &siege)?;
        c.path = Some(path.to_path_buf());
        Ok(c)
    }

    /// Which file a value's path is in: `deadmines.toml` for its own tables and list entries.
    fn in_dungeon_file(&self, path: &str) -> bool {
        let segs: Vec<&str> = path.split('.').collect();
        if segs.first() == Some(&"dungeon") {
            return true;
        }
        segs.len() >= 2
            && crate::tune::get(&self.dungeon_source, &format!("{}.{}", segs[0], segs[1]))
                .ok()
                .flatten()
                .is_some()
    }

    /// These rules with one value changed (`combat.chain_after`, `moveset.one_hand.light.2.lunge`):
    /// the new rules and what it was before.
    pub fn tuned(&self, path: &str, value: &str) -> Result<(Self, Option<String>)> {
        let (base, dungeon, old) = if self.in_dungeon_file(path) {
            let (t, old) = crate::tune::set(&self.dungeon_source, path, value)?;
            (self.source.clone(), t, old)
        } else {
            let (t, old) = crate::tune::set(&self.source, path, value)?;
            (t, self.dungeon_source.clone(), old)
        };
        let mut c = Self::parse_full(&base, &dungeon, &self.more_sources, DEFAULT_SIEGE)?;
        c.path.clone_from(&self.path);
        Ok((c, old))
    }

    /// What a value is set to in these rules (`None`: its default).
    pub fn tuning(&self, path: &str) -> Result<Option<String>> {
        if self.in_dungeon_file(path) {
            crate::tune::get(&self.dungeon_source, path)
        } else {
            crate::tune::get(&self.source, path)
        }
    }

    /// These rules as the Deadmines' world plays them: its own packs, no camp, nodes, Horde camps
    /// or journal, and heroes' blows pass through each other.
    /// The rules as a Siege world has them: no Elwynn (its packs, nodes, places, chests), the
    /// middle's veins, heroes arriving in the hall, the rack free, heroes able to strike each
    /// other (whose foes they are is the match's to say).
    pub fn for_siege(&self) -> Self {
        let mut c = self.clone();
        c.packs.clear();
        c.nodes = self.siege.as_ref().map(|s| s.nodes.clone()).unwrap_or_default();
        c.roams.clear();
        c.camps.clear();
        c.sites.clear();
        c.chests.clear();
        c.props.clear();
        c.goals.clear();
        c.arena = None;
        c.dungeon = None;
        c.pvp.friendly_fire = true;
        c.world.armory = true;
        if let Some(s) = &self.siege {
            c.world.spawn = [s.lobby[0], s.lobby[1]];
            c.world.spawn_facing = s.lobby_facing;
        }
        c
    }

    pub fn for_dungeon(&self) -> Self {
        let first = self.dungeon.as_ref().map(|d| d.key.clone()).unwrap_or_default();
        self.for_dungeon_of(&first)
    }

    /// These rules as one dungeon's world plays them (by its key): that dungeon is the world's own.
    pub fn for_dungeon_of(&self, key: &str) -> Self {
        let mut c = self.clone();
        c.dungeon = self.dungeons().find(|d| d.key == key).cloned();
        c.elsewhere = self.ladder().filter(|d| d.key != key).cloned().collect();
        c.more_dungeons.clear();
        if let Some(d) = &c.dungeon {
            c.packs = d.packs.clone();
        }
        c.nodes.clear();
        c.roams.clear();
        c.camps.clear();
        c.sites.clear();
        c.chests.clear();
        c.props.clear();
        c.arena = None;
        c.siege = None;
        c.pvp.friendly_fire = false;
        c
    }

    /// Every dungeon in these rules (the Deadmines', and those beside it).
    pub fn dungeons(&self) -> impl Iterator<Item = &DungeonDef> {
        self.dungeon.iter().chain(self.more_dungeons.iter())
    }

    /// Every dungeon on the book's ladder, in a dungeon's own world too (where the others are
    /// `elsewhere`).
    pub fn ladder(&self) -> impl Iterator<Item = &DungeonDef> {
        self.dungeons().chain(self.elsewhere.iter())
    }

    /// The dungeon (by key) a lamp is in.
    pub fn dungeon_of_lamp(&self, lamp: &str) -> Option<&DungeonDef> {
        self.dungeons().find(|d| d.lamps.iter().any(|l| l.key == lamp))
    }

    /// Put what's read in the order it's played: each creature's phases, its enrage among them,
    /// highest share of health first.
    fn settle(&mut self) {
        // A weapon that names nothing of its own takes its kind's (docs/weapon-identities.md): a
        // sword the duelist's, an axe a bleed, a polearm reach, a two-handed sword the sweep.
        for w in &mut self.weapons {
            let own = w.parry > 0.0 || w.riposte > 0.0 || w.bleed > 0.0 || w.posture > 0.0 || w.backstab > 0.0 || w.through_guard || w.swing_poise || w.execute > 0.0 || w.stagger > 0.0;
            if own || !w.bolt.is_empty() || !w.alt.is_empty() {
                continue;
            }
            match w.subclass {
                7 => {
                    w.parry = 0.1;
                    w.riposte = 2.25;
                }
                0 => w.bleed = if w.two_handed { 6.0 } else { 4.0 },
                4 | 5 => w.posture = 2.0,
                15 => w.backstab = 2.5,
                6 => w.through_guard = true,
                8 => w.swing_poise = true,
                1 => w.execute = 2.0,
                _ => {}
            }
        }
        // A weapon's tooltip says its own mechanic, built from its numbers: `{fx}` in the item's
        // text is where it goes (a text without the mark has none to say).
        let texts: Vec<(String, String)> = self.weapons.iter().map(|w| (w.key.clone(), w.effect_text())).collect();
        for (key, fx) in texts {
            if let Some(i) = self.items.iter_mut().find(|i| i.key == key) {
                i.text = i.text.replace("{fx}", &fx).replace("  ", " ");
            }
        }
        let grow = self.combat.boss_scale.max(0.1);
        for m in &mut self.mobs {
            if m.boss {
                m.scale *= grow;
            }
            if let Some(e) = m.enrage.clone() {
                if !m.phases.iter().any(|p| p.at == e.at) {
                    m.phases.push(e);
                }
            }
            m.phases.sort_by(|a, b| b.at.total_cmp(&a.at));
        }
        // An ultimate's control and movement take on bosses too (everything else's don't).
        for a in self.abilities.iter_mut().filter(|a| a.slot == "ultimate") {
            for e in a.effects.iter_mut().chain(a.tick_effects.iter_mut()).chain(a.tether_effects.iter_mut()) {
                e.vs_bosses = true;
            }
        }
    }

    /// Honed copies of every piece the dungeons' foes carry (weapons and armour), a tier each:
    /// the same look, better by the tier's share, its name and words saying so.
    fn hone(&mut self) {
        let mut pool: Vec<String> = Vec::new();
        for d in self.dungeon.iter().chain(self.more_dungeons.iter()) {
            for p in &d.packs {
                if let Some(m) = self.mobs.iter().find(|m| m.key == p.mob) {
                    for g in &m.gear {
                        if !pool.contains(g) {
                            pool.push(g.clone());
                        }
                    }
                }
            }
        }
        for key in pool {
            let Some(item) = self.items.iter().find(|i| i.key == key && i.hone == 0).cloned() else { continue };
            let weapon = self.weapons.iter().find(|w| w.key == key).cloned();
            if !(item.kind == ItemKind::Armor || (item.kind == ItemKind::Weapon && weapon.is_some())) {
                continue;
            }
            for (t, (at, name, by)) in HONE_TIERS.iter().enumerate() {
                let tier = t as u8 + 1;
                let k = format!("{key}_honed{tier}");
                if self.items.iter().any(|i| i.key == k) {
                    continue;
                }
                let mut h = item.clone();
                h.key = k.clone();
                h.name = format!("{}, {name}", item.name);
                h.hone = tier;
                h.honed_from = key.clone();
                // (Capped, but never below the piece it was honed from.)
                h.reduce = (item.reduce * by).min(0.6).max(item.reduce);
                for p in &mut h.perks {
                    p.amount *= by;
                }
                h.text = format!("{} {name} by the Bookmaker (a wager won at +{at} or higher): {}% better.{}", item.text, ((by - 1.0) * 100.0).round(), HONE_TRICKS[t]);
                self.items.push(h);
                if let Some(w) = &weapon {
                    let mut hw = w.clone();
                    hw.key = k;
                    hw.damage = [(w.damage[0] as f32 * by).round() as u32, (w.damage[1] as f32 * by).round() as u32];
                    self.weapons.push(hw);
                }
            }
        }
    }

    /// A piece's honed copy at `tier` (itself at 0, or if it has none).
    pub fn honed(&self, item: usize, tier: u8) -> usize {
        if tier == 0 {
            return item;
        }
        let k = format!("{}_honed{tier}", self.items[item].key);
        self.item(&k).unwrap_or(item)
    }

    /// The choices the camp's people offer: each `(group, option)` of a `.es choose group:option`
    /// link in a talk line.
    pub fn choices(&self) -> Vec<(String, String)> {
        self.props
            .iter()
            .flat_map(|p| p.talk.iter().flat_map(|t| t.links.iter()))
            .filter_map(|l| l.split('~').next()?.trim().strip_prefix(".es choose ").map(str::trim).map(str::to_string))
            .filter_map(|c| c.split_once(':').map(|(g, o)| (g.to_string(), o.to_string())))
            .collect()
    }

    /// A condition on how far the camp has come (`raids:2`, `kill:hogger`) that names something real.
    fn check_camp_condition(&self, cond: &str, what: &str) -> Result<()> {
        let (kind, arg) = cond.split_once(':').unwrap_or((cond, ""));
        let (kind, arg) = (kind.trim(), arg.trim());
        if !CAMP_CONDITIONS.contains(&kind) {
            bail!("{what}: after {cond:?}: no condition {kind:?} (one of {CAMP_CONDITIONS:?})");
        }
        let real = match kind {
            "start" => true,
            "raids" => arg.parse::<u32>().is_ok(),
            "goal" => arg.parse::<usize>().is_ok(),
            "built" => self.station(arg).is_some(),
            "found" => self.sites.iter().any(|s| s.key == arg),
            "kill" => self.mob(arg).is_some(),
            // `chose:<group>` or `chose:<group>:<option>`: one a person's `.es choose` link offers.
            "chose" => {
                let (group, option) = arg.split_once(':').map_or((arg, None), |(g, o)| (g, Some(o)));
                self.choices().iter().any(|(g, o)| g == group && option.is_none_or(|x| x == o))
            }
            _ => self.chests.iter().any(|c| c.key == arg),
        };
        if !real {
            bail!("{what}: after {cond:?}: {arg:?} names nothing");
        }
        Ok(())
    }

    fn validate(&self) -> Result<()> {
        // Gear's perks and sets.
        for i in &self.items {
            if !ItemDef::ANIMS.contains(&i.anim.as_str()) {
                bail!("item {}: anim {:?} (one of {:?})", i.key, i.anim, ItemDef::ANIMS);
            }
            if i.heal_secs < 0.0 {
                bail!("item {}: heal_secs {} below 0", i.key, i.heal_secs);
            }
            for p in &i.perks {
                if !EffectDef::KINDS.contains(&p.kind.as_str()) {
                    bail!("item {}: a perk of kind {:?} (one of {:?})", i.key, p.kind, EffectDef::KINDS);
                }
            }
            if !i.set.is_empty() && !self.sets.iter().any(|s| s.key == i.set) {
                bail!("item {}: no set {:?}", i.key, i.set);
            }
        }
        for s in &self.sets {
            if !self.items.iter().any(|i| i.set == s.key) {
                bail!("set {}: no pieces", s.key);
            }
            for b in &s.bonus {
                if !EffectDef::KINDS.contains(&b.kind.as_str()) || b.pieces < 2 {
                    bail!("set {}: a bonus of kind {:?} at {} pieces", s.key, b.kind, b.pieces);
                }
            }
        }
        if let Some(sg) = &self.siege {
            for f in sg.roster.iter().map(|f| &f.mob).chain(sg.sends.iter().map(|x| &x.mob)) {
                if self.mob(f).is_none() {
                    bail!("siege: no mob {f}");
                }
            }
            for m in &sg.modes {
                for b in &m.bases {
                    if !sg.bases.iter().any(|x| &x.key == b) {
                        bail!("siege mode {}: no base {b}", m.key);
                    }
                }
                if m.bases.len() < 2 || m.per_base == 0 {
                    bail!("siege mode {}: two camps at least, a hero to each", m.key);
                }
            }
            for k in sg.build.keys().chain(std::iter::once(&sg.hearth)) {
                if self.station(k).is_none() {
                    bail!("siege: no station {k}");
                }
            }
            if self.item("gold").is_none() {
                bail!("siege: no item gold");
            }
        }
        let item = |k: &str, what: &str| -> Result<()> {
            if self.item(k).is_none() {
                bail!("{what}: no item {k}");
            }
            Ok(())
        };
        for s in &self.stations {
            for k in s.cost.keys().chain(s.repair.keys()) {
                item(k, &format!("station {}", s.key))?;
            }
        }
        for r in &self.recipes {
            if self.station(&r.station).is_none() {
                bail!("recipe {}: no station {}", r.key, r.station);
            }
            item(&r.output, &format!("recipe {}", r.key))?;
            for k in r.inputs.keys() {
                item(k, &format!("recipe {}", r.key))?;
            }
        }
        let mut node_keys = std::collections::BTreeSet::new();
        for n in &self.nodes {
            item(&n.item, "nodes")?;
            for k in n.loot.keys() {
                item(k, &format!("nodes {}", n.id()))?;
            }
            if !node_keys.insert(n.id().to_string()) {
                bail!("two kinds of nodes called {:?}: give one a `key`", n.id());
            }
            if n.areas.is_empty() && n.points.is_empty() && n.deep.is_empty() && n.spread <= 0.0 {
                bail!("nodes of {}: no areas, points nor spread", n.item);
            }
        }
        for r in &self.roams {
            for m in &r.mobs {
                if self.mob(m).is_none() {
                    bail!("roam: no mob {m}");
                }
            }
            if r.mobs.is_empty() || r.size[0] == 0 || r.size[0] > r.size[1] {
                bail!("roam {:?}: needs mobs and a size [least, most]", r.mobs);
            }
        }
        if let Some(a) = &self.arena {
            for k in a.bosses.iter().chain(a.roster.iter().map(|f| &f.mob)) {
                if self.mob(k).is_none() {
                    bail!("arena: no mob {k}");
                }
            }
            for k in a.reward.keys() {
                item(k, "arena reward")?;
            }
            if a.roster.iter().all(|f| f.from > 1) || a.roster.iter().any(|f| f.cost <= 0.0) {
                bail!("arena: a foe for the first wave, and every cost above nothing");
            }
            if a.boss_every == 0 || a.most == 0 {
                bail!("arena: boss_every and most at least 1");
            }
        }
        for p in &self.packs {
            if !p.camp.is_empty() && !self.camps.iter().any(|c| c.key == p.camp) {
                bail!("pack of {}: no [[camp]] {}", p.mob, p.camp);
            }
        }
        for pl in &self.plans {
            if !["all_in", "pincer", "screen", "bait", "waves"].contains(&pl.key.as_str()) {
                bail!("plan {}: no such plan (all_in, pincer, screen, bait, waves)", pl.key);
            }
        }
        for c in &self.camps {
            if c.hold <= 0.0 || !self.packs.iter().any(|p| p.camp == c.key) {
                bail!("camp {}: it needs a hold above 0 and a pack in it", c.key);
            }
            for k in c.reward.keys() {
                item(k, &format!("camp {}", c.key))?;
            }
        }
        for st in &self.stations {
            if let Some(sh) = &st.shoots {
                if sh.every <= 0.0 || sh.range <= 0.0 {
                    bail!("station {}: a tower shoots every so often (above 0) and some way (above 0)", st.key);
                }
            }
        }
        for m in &self.mobs {
            if let Some(s) = &m.shoots {
                if !m.friendly {
                    bail!("mob {}: only a friendly creature shoots", m.key);
                }
                if s.damage == 0 || s.range <= 0.0 || s.every <= 0.0 {
                    bail!("mob {}: a shot needs damage, range and every", m.key);
                }
            }
            if let Some(a) = &m.arsenal {
                if a.loadouts.len() != m.phases.len() + 1 || a.display == 0 || a.rack.iter().any(|x| !x.is_finite()) {
                    bail!("mob {}: arsenal needs a rack and one loadout per stage", m.key);
                }
                for g in &a.loadouts {
                    if g.weapon == 0 || g.patterns.is_empty() || !["1h", "2h", "sword", "hammer"].contains(&g.grip.as_str())
                        || g.patterns.iter().any(|name| !m.patterns.iter().any(|p| &p.name == name && p.weight > 0.0)) {
                        bail!("mob {}: invalid arsenal loadout", m.key);
                    }
                }
                if m.phases.iter().any(|p| p.speed != 1.0 || p.might != 1.0 || p.shove != 0.0 || !p.pattern.is_empty()) {
                    bail!("mob {}: arsenal phases only request equipment changes", m.key);
                }
            }
            if let Some(c) = &m.perch_crew {
                if !m.boss || !(2..=3).contains(&c.ledges.len()) {
                    bail!("mob {}: a perch crew needs a boss and 2-3 ledges", m.key);
                }
                if c.ledges.iter().any(|l| !(5.0..=12.0).contains(&l.at[2]) || !l.out.is_finite()) {
                    bail!("mob {}: a ledge stands 5 to 12 yd over the floor, its plank jutting some way", m.key);
                }
                for key in [&c.airborne, &c.grounded] {
                    let Some(k) = self.mob(key) else { bail!("mob {}: no crew mob {key}", m.key) };
                    if self.mobs[k].perch_crew.is_some() || self.mobs[k].boss || self.mobs[k].inert() {
                        bail!("mob {}: crew {key} must be an ordinary fighter", m.key);
                    }
                }
                if !self.mobs[self.mob(&c.airborne).unwrap()].attack.ranged || self.mobs[self.mob(&c.grounded).unwrap()].attack.ranged {
                    bail!("mob {}: its perched crew throws, its grounded crew fights up close", m.key);
                }
                if c.ledges.iter().flat_map(|l| l.at).chain(c.valve).chain(c.furnace).any(|n| !n.is_finite() || n.abs() > 30.0) {
                    bail!("mob {}: crew offsets must be finite and within 30 yd", m.key);
                }
            }
            if !m.temper.is_empty() && !self.tempers.iter().any(|t| t.key == m.temper) {
                bail!("mob {}: temper {} is no [[temper]]", m.key, m.temper);
            }
            for k in m.drops.keys().chain(m.trophy.keys()).chain(m.gear.iter()) {
                item(k, &format!("mob {}", m.key))?;
            }
            if !m.then.is_empty() && (self.mob(&m.then).is_none() || m.then == m.key) {
                bail!("mob {}: then {} is no other mob", m.key, m.then);
            }
            if !m.then_needs.is_empty() {
                let below = self.dungeons().any(|d| d.doors.iter().any(|x| x.key == m.then_needs) || d.packs.iter().any(|p| p.key == m.then_needs));
                if self.item(&m.then_needs).is_none() && !below {
                    bail!("mob {}: then_needs {:?} is no item, door, secret or pack below", m.key, m.then_needs);
                }
            }
            for e in &m.phases {
                if !(e.at > 0.0 && e.at < 1.0) || e.speed <= 0.0 || e.might <= 0.0 {
                    bail!("mob {}: a phase's at must be between 0 and 1, its speed and might above 0", m.key);
                }
                if !e.pattern.is_empty() && !m.patterns.iter().any(|p| p.name == e.pattern) {
                    bail!("mob {}: enrage pattern {} isn't one of its patterns", m.key, e.pattern);
                }
                for k in &e.summon {
                    if self.mob(k).is_none() {
                        bail!("mob {}: enrage summons {k}, no such mob", m.key);
                    }
                }
            }
        }
        for p in &self.packs {
            if self.mob(&p.mob).is_none() {
                bail!("pack: no mob {}", p.mob);
            }
            if let Some(cond) = &p.after {
                self.check_camp_condition(cond, &format!("pack of {}", p.mob))?;
            }
            if !p.wake.is_empty() && !self.chests.iter().any(|c| c.key == p.wake) {
                bail!("pack of {}: it wakes on chest {}, and there's none", p.mob, p.wake);
            }
        }
        let card = |k: &str, what: &str| -> Result<()> {
            match self.ability(k) {
                Some(a) if !self.abilities[a].is_ex() && !self.abilities[a].inner => Ok(()),
                _ => bail!("{what}: it teaches {k}, which is no card"),
            }
        };
        let mut seen = std::collections::BTreeSet::new();
        for site in &self.sites {
            if !seen.insert(site.key.clone()) || site.radius <= 0.0 {
                bail!("site {}: a key of its own and a radius above 0", site.key);
            }
            for k in site.reward.keys() {
                item(k, &format!("site {}", site.key))?;
            }
        }
        let mut seen = std::collections::BTreeSet::new();
        for c in &self.chests {
            if !seen.insert(c.key.clone()) {
                bail!("two chests called {}", c.key);
            }
            for k in c.loot.keys().chain(c.roll.keys()) {
                item(k, &format!("chest {}", c.key))?;
            }
            for k in &c.teaches {
                card(k, &format!("chest {}", c.key))?;
            }
            // A bare key is a creature (shut until the camp has killed one); with a colon it is a
            // condition the camp meets (`open:<chest>`, `found:<site>`, `kill:<mob>`, `chose:<group>:<option>`).
            if c.after.contains(':') {
                let what = format!("chest {}", c.key);
                let kind = c.after.split(':').next().unwrap_or("").trim();
                if !["open", "found", "kill", "chose"].contains(&kind) {
                    bail!("{what}: after {:?}: a chest waits on open:, found:, kill: or chose:", c.after);
                }
                if c.after.trim() == format!("open:{}", c.key) {
                    bail!("{what}: after {:?}: it would wait on itself", c.after);
                }
                self.check_camp_condition(&c.after, &what)?;
            } else if !c.after.is_empty() && self.mob(&c.after).is_none() {
                bail!("chest {}: after {}, no such mob", c.key, c.after);
            }
            // A trial's entry is a wave: one pack, or several joined with `+`.
            for k in c.ambush.iter().map(String::as_str).chain(c.trial.iter().flat_map(|w| w.split('+')).map(str::trim)) {
                if !self.packs.iter().any(|p| p.key == k && p.wake == c.key) {
                    bail!("chest {}: ambush {k} needs a [[pack]] of that key that wakes on it", c.key);
                }
            }
            if c.hold < 0.0 || (c.hold > 0.0 && c.trial.is_empty()) {
                bail!("chest {}: a hold of {} sec needs waves (`trial`)", c.key, c.hold);
            }
            if c.wave_text.len() > c.trial.len() {
                bail!("chest {}: {} wave texts for {} waves", c.key, c.wave_text.len(), c.trial.len());
            }
            // An escort: one friendly walker that wakes on it, a way to walk, waves, and no clock.
            if !c.escort.is_empty() {
                let what = format!("chest {}: escort {}", c.key, c.escort);
                let Some(p) = self.packs.iter().find(|p| p.key == c.escort) else { bail!("{what}: no [[pack]] of that key") };
                if p.wake != c.key || p.count != 1 {
                    bail!("{what}: its pack must be one creature that wakes on the chest");
                }
                if !self.mob(&p.mob).is_some_and(|m| self.mobs[m].friendly) {
                    bail!("{what}: the one walked is a friendly creature");
                }
                if c.route.is_empty() || c.trial.is_empty() || c.hold > 0.0 || !c.teaches.is_empty() {
                    bail!("{what}: an escort has a route and waves (`trial`), and no `hold` or `teaches`");
                }
            } else if !c.route.is_empty() {
                bail!("chest {}: a route is for an escort", c.key);
            }
        }
        for m in &self.mobs {
            for k in &m.teaches {
                card(k, &format!("mob {}", m.key))?;
            }
        }
        // What a prop and what a person's words wait for.
        for (n, p) in self.props.iter().enumerate() {
            let what = format!("prop {n} ({})", if p.name.is_empty() { "unnamed" } else { &p.name });
            if let Some(a) = &p.after {
                self.check_camp_condition(a, &what)?;
            }
            if let Some(u) = &p.until {
                self.check_camp_condition(u, &what)?;
            }
            if !p.talk.is_empty() && !p.npc {
                bail!("{what}: talk lines are for an npc");
            }
            for t in &p.talk {
                if let Some(a) = &t.after {
                    self.check_camp_condition(a, &what)?;
                }
                if t.text.trim().is_empty() {
                    bail!("{what}: a talk line with no words");
                }
                for l in &t.links {
                    if l.split_once('~').is_none_or(|(c, n)| c.trim().is_empty() || n.trim().is_empty()) {
                        bail!("{what}: link {l:?} is not `command~label`");
                    }
                    // `.es choose <group>:<option>`: a group and one option, nothing else.
                    let command = l.split('~').next().unwrap_or("").trim();
                    if let Some(c) = command.strip_prefix(".es choose") {
                        if c.trim().split_once(':').is_none_or(|(g, o)| g.is_empty() || o.is_empty() || o.contains(':') || c.trim().contains(' ')) {
                            bail!("{what}: link {l:?} is not `.es choose <group>:<option>`");
                        }
                    }
                }
            }
        }
        // Every choice offered is waited on: some line, person or chest names its option.
        let mut waits: Vec<&str> = Vec::new();
        for p in &self.props {
            waits.extend(p.after.as_deref().into_iter().chain(p.until.as_deref()));
            waits.extend(p.talk.iter().filter_map(|t| t.after.as_deref()));
        }
        waits.extend(self.chests.iter().map(|c| c.after.as_str()));
        for (g, o) in self.choices() {
            let one = format!("chose:{g}:{o}");
            if !waits.iter().any(|w| w.trim() == one) {
                bail!("choice {g}:{o}: nothing waits on it (a line, person or chest `after = \"{one}\"`)");
            }
        }
        for t in 0..self.raid.count() {
            let p = self.raid.plan(t as u32);
            for (m, _) in &p.raiders {
                if self.mob(m).is_none() {
                    bail!("raid {}: no mob {m}", p.title);
                }
            }
            for s in &p.needs {
                if self.station(s).is_none() {
                    bail!("raid {}: no station {s}", p.title);
                }
            }
            for k in p.reward.keys() {
                item(k, &format!("raid {} reward", p.title))?;
            }
        }
        for k in [&self.start.weapon, &self.start.shield, &self.start.bow] {
            item(k, "start")?;
        }
        if self.weapon(&self.start.weapon).is_none() {
            bail!("start: {} isn't a [[weapon]]", self.start.weapon);
        }
        if self.shield(&self.start.shield).is_none() {
            bail!("start: {} isn't a [[shield]]", self.start.shield);
        }
        for k in [&self.rogue.weapon, &self.rogue.off_hand] {
            item(k, "rogue")?;
            if self.weapon(k).is_none() || self.item(k).is_none_or(|i| self.rogue_can_hold(i).is_err()) {
                bail!("rogue: {k} isn't a weapon a Rogue can hold");
            }
        }
        for i in &self.items {
            match i.kind {
                ItemKind::Weapon if self.weapon(&i.key).is_none() => bail!("item {}: no [[weapon]] of that key", i.key),
                ItemKind::Shield if self.shield(&i.key).is_none() => bail!("item {}: no [[shield]] of that key", i.key),
                _ => {}
            }
        }
        if self.movesets.is_empty() {
            bail!("no [[moveset]]");
        }
        for m in &self.movesets {
            if m.light.is_empty() {
                bail!("moveset {}: no light blows", m.key);
            }
            // A borrowed clip names a character model the clients can find.
            let moves = m.light.iter().chain([&m.heavy, &m.running]).chain(m.roll.iter()).chain(m.leap.iter());
            for d in moves {
                if let Some(f) = d.from.as_deref().filter(|f| !clip_source_ok(f)) {
                    bail!("moveset {}: from {f:?} is no character model (<Race><Gender>, as ScourgeFemale)", m.key);
                }
            }
        }
        for m in &self.mobs {
            let steps = m.special.iter().chain(m.patterns.iter().flat_map(|p| p.steps.iter()));
            let named = steps.filter_map(|s| s.from.as_deref()).chain(m.clips_from.as_deref());
            for f in named {
                if !clip_source_ok(f) {
                    bail!("mob {}: clips from {f:?}, which is no character model (<Race><Gender>, as ScourgeFemale)", m.key);
                }
            }
        }
        for w in &self.weapons {
            if !w.moveset.is_empty() && !self.movesets.iter().any(|m| m.key == w.moveset) {
                bail!("weapon {}: no [[moveset]] {}", w.key, w.moveset);
            }
            if w.two_handed && self.shield(&w.guard).is_none() {
                bail!("weapon {}: two-handed, and its guard {:?} isn't a [[shield]]", w.key, w.guard);
            }
            // (The click may be a cord held while the button is: the Hollow Star's Void Tether. A
            // heavy bolt may be help at the reticle instead. The right click's ability is a bolt, a
            // cord held while the button is, or a thing it places: the Hollow Star.)
            for (what, key, kinds) in [("bolt", &w.bolt, &["projectile", "channel"][..]), ("heavy_bolt", &w.heavy_bolt, &["projectile", "ally"][..]), ("alt", &w.alt, &["projectile", "channel", "place"][..])] {
                if key.is_empty() {
                    continue;
                }
                match self.ability(key) {
                    Some(a) if kinds.contains(&self.abilities[a].kind.as_str()) => {}
                    Some(_) => bail!("weapon {}: its {what} {key} must be one of {kinds:?}", w.key),
                    None => bail!("weapon {}: no [[ability]] {key} for its {what}", w.key),
                }
            }
            if !w.heavy_bolt.is_empty() && w.bolt.is_empty() {
                bail!("weapon {}: a heavy_bolt needs a bolt", w.key);
            }
        }
        if self.ranged(&self.start.bow).is_none() {
            bail!("start: {} isn't a [[ranged]]", self.start.bow);
        }
        for i in &self.items {
            if i.kind == ItemKind::Bow && self.ranged(&i.key).is_none() {
                bail!("item {}: no [[ranged]] of that key", i.key);
            }
            if i.kind == ItemKind::Armor && !ARMOR_SLOTS.contains(&i.slot.as_str()) {
                bail!("item {}: armor worn where? slot {:?} (one of {ARMOR_SLOTS:?})", i.key, i.slot);
            }
            if i.kind == ItemKind::Pack && i.carry <= 0.0 {
                bail!("item {}: a pack that carries nothing more", i.key);
            }
            if let Some(e) = &i.effect {
                if !EffectDef::KINDS.contains(&e.kind.as_str()) {
                    bail!("item {}: no effect {:?} (one of {:?})", i.key, e.kind, EffectDef::KINDS);
                }
            }
        }
        for g in &self.goals {
            if !GoalDef::KINDS.contains(&g.kind.as_str()) {
                bail!("goal {}: no kind {:?} (one of {:?})", g.title, g.kind, GoalDef::KINDS);
            }
            for k in g.reward.keys() {
                item(k, &format!("goal {}", g.title))?;
            }
            let known = match g.kind.as_str() {
                "bank" | "gather" => self.item(&g.what).is_some(),
                "build" => self.station(&g.what).is_some(),
                "craft" => self.recipes.iter().any(|r| r.key == g.what),
                "kill" => self.mob(&g.what).is_some(),
                "raid" => g.what.parse::<usize>().is_ok_and(|t| t < self.raid.count()),
                "clear" => self.camps.iter().any(|c| c.key == g.what),
                // Places found (any, of a kind, or one by key), chests opened (any, or one), secrets
                // come upon and cards learned (any).
                "explore" => g.what == "any" || self.sites.iter().any(|x| x.key == g.what || x.kind == g.what),
                "open" => g.what == "any" || self.chests.iter().any(|c| c.key == g.what),
                "secret" | "learn" => g.what == "any",
                // A dungeon cleared (every boss down in one run), by its key.
                "cleared" => self.dungeons().any(|d| d.key == g.what),
                // A choice made (a group a person's `.es choose` link offers).
                "chose" => self.choices().iter().any(|(grp, _)| *grp == g.what),
                // Below: the book read, a lamp lit (by its key), out of any dungeon's way out
                // (`end`) or one's (`end:<dungeon>`).
                "delve" => {
                    g.what == "book"
                        || g.what == "end"
                        || g.what.strip_prefix("end:").is_some_and(|k| self.dungeons().any(|d| d.key == k))
                        || self.dungeons().any(|d| d.lamps.iter().any(|l| l.key == g.what))
                }
                _ => ARMOR_SLOTS.contains(&g.what.as_str()),
            };
            if !known {
                bail!("goal {}: no {} {:?}", g.title, g.kind, g.what);
            }
        }
        let mut lamp_keys = std::collections::BTreeSet::new();
        let mut dungeon_keys = std::collections::BTreeSet::new();
        for d in self.dungeons() {
            if !dungeon_keys.insert(d.key.clone()) {
                bail!("two dungeons called {}", d.key);
            }
            for l in &d.lamps {
                if !lamp_keys.insert(l.key.clone()) {
                    bail!("dungeon {}: lamp {} is another dungeon's already (a lamp is a page of the book)", d.key, l.key);
                }
            }
        }
        for d in self.dungeons() {
            for k in [&d.draught, &d.plunder] {
                item(k, "dungeon")?;
            }
            if !d.flask.is_empty() {
                item(&d.flask, "dungeon flask")?;
            }
            for k in d.trade.keys() {
                item(k, "dungeon trade")?;
            }
            for k in d.first_clear.keys() {
                item(k, "dungeon first_clear")?;
            }
            if d.lamps.is_empty() {
                bail!("dungeon: no [[dungeon.lamp]]");
            }
            let mut keys = std::collections::BTreeSet::new();
            for l in &d.lamps {
                if !keys.insert(l.key.clone()) {
                    bail!("dungeon: two lamps called {}", l.key);
                }
            }
            let pack_keys: Vec<&str> = d
                .packs
                .iter()
                .filter(|p| !p.key.is_empty())
                .map(|p| p.key.as_str())
                .collect();
            for p in &d.packs {
                if self.mob(&p.mob).is_none() {
                    bail!("dungeon pack: no mob {}", p.mob);
                }
                if self.mob(&p.mob).is_some_and(|m| self.mobs[m].boss) && p.key.is_empty() {
                    bail!("dungeon pack of {}: a boss's pack needs a key", p.mob);
                }
                if !p.wake.is_empty() && !d.boxes.iter().any(|b| b.key == p.wake) {
                    bail!("dungeon pack of {}: it wakes on strongbox {}, and there's none", p.mob, p.wake);
                }
            }
            for dr in &d.doors {
                if !DoorDef::KINDS.contains(&dr.kind.as_str()) {
                    bail!(
                        "dungeon door {}: kind {:?} (one of {:?})",
                        dr.key,
                        dr.kind,
                        DoorDef::KINDS
                    );
                }
                if matches!(dr.kind.as_str(), "fog" | "gate" | "shortcut") && dr.bars.is_none() {
                    bail!("dungeon door {}: a {} needs `bars`", dr.key, dr.kind);
                }
                if matches!(dr.kind.as_str(), "shortcut" | "lift") && dr.from.is_none() {
                    bail!("dungeon door {}: a shortcut needs `from`", dr.key);
                }
                if dr.kind == "hidden" && dr.find <= 0.0 {
                    bail!("dungeon door {}: a hidden secret is noticed within `find` yards (above 0)", dr.key);
                }
                // A page travels to the add-on in one tab-separated line: `{n}` for its breaks.
                if !dr.page.is_empty() && !dr.secret() {
                    bail!("dungeon door {}: only a secret has a `page`", dr.key);
                }
                if dr.page.contains(['\t', '\n', '\r']) {
                    bail!("dungeon door {}: its `page` breaks lines with {{n}}, not tabs or newlines", dr.key);
                }
                // `|`, `~` and `^` separate fields in the add-on's lines, so the server blanks them.
                if dr.page.contains(['|', '~', '^']) {
                    bail!("dungeon door {}: its `page` can't use |, ~ or ^ (the add-on's line uses them)", dr.key);
                }
                // One line to the add-on carries the whole page; the longest written is about 830.
                if dr.page.chars().count() > PAGE_MAX {
                    bail!("dungeon door {}: its `page` is {} characters, over {PAGE_MAX}", dr.key, dr.page.chars().count());
                }
                if dr.kind == "hidden" && (dr.display == 0 || dr.look.is_empty()) {
                    bail!("dungeon door {}: a hidden secret needs a marker to search (`display` and `look`)", dr.key);
                }
                for r in &dr.reveals {
                    if !pack_keys.contains(&r.as_str()) && !d.boxes.iter().any(|b| &b.key == r) && !d.notes.iter().any(|n| &n.key == r) {
                        bail!("dungeon door {}: it reveals {r:?}, no pack, strongbox or note of that key", dr.key);
                    }
                }
                if matches!(dr.kind.as_str(), "fog" | "gate") && !pack_keys.contains(&dr.boss.as_str()) {
                    bail!(
                        "dungeon door {}: no pack {:?} for its boss",
                        dr.key,
                        dr.boss
                    );
                }
            }
            for n in &d.notes {
                for h in &n.hints {
                    if !d.doors.iter().any(|dr| &dr.key == h && dr.secret()) {
                        bail!("dungeon note {}: it hints at {h:?}, no secret of that key", n.key);
                    }
                }
                if n.hints.is_empty() != n.clue.is_empty() {
                    bail!("dungeon note {}: `hints` and `clue` go together", n.key);
                }
            }
            if !(1..=TIER_NAMES.len() as u32).contains(&d.tier) {
                bail!("dungeon {}: tier {} (1 to {})", d.key, d.tier, TIER_NAMES.len());
            }
            if !d.exit_boss.is_empty() && !pack_keys.contains(&d.exit_boss.as_str()) {
                bail!(
                    "dungeon: no pack {:?} for the front door's boss",
                    d.exit_boss
                );
            }
            for b in &d.boxes {
                for k in b.loot.keys() {
                    item(k, &format!("dungeon box {}", b.key))?;
                }
            }
            for h in &d.hazards {
                if h.every <= 0.0 || (!h.boss.is_empty() && !pack_keys.contains(&h.boss.as_str())) {
                    bail!(
                        "dungeon hazard {}: every above 0, and a boss that's a pack's key",
                        h.name
                    );
                }
            }
        }
        // (Items are queried by entries from 300 000 up, outfits from 300 900.)
        if self.items.len() > 800 {
            bail!("at most 800 items");
        }
        self.validate_abilities()?;
        Ok(())
    }

    pub(crate) fn validate_abilities(&self) -> Result<()> {
        let rgb = |c: &str| c.len() == 6 && u32::from_str_radix(c, 16).is_ok();
        for (k, l) in self.missile_looks.iter().enumerate() {
            if l.spell == 0 || self.missile_looks[..k].iter().any(|o| o.spell == l.spell) {
                bail!("missile_look {}: a spell, once", l.spell);
            }
            if !(l.trail.is_empty() || rgb(&l.trail)) || l.width < 0.0 || l.secs < 0.0 || l.scale <= 0.0 || l.speed < 0.0 {
                bail!("missile_look {}: its trail is \"RRGGBB\" (or none), and its width, secs, scale and speed aren't below 0", l.spell);
            }
        }
        for a in &self.abilities {
            if let Some(c) = a.bounce_trail.iter().find(|c| !rgb(c)) {
                bail!("{}: bounce_trail colour {c:?} isn't \"RRGGBB\"", a.key);
            }
        }
        let mut keys = std::collections::BTreeSet::new();
        let effect_ok = |e: &AbilityEffect, what: &str| -> Result<()> {
            if !CC_KINDS.contains(&e.kind.as_str()) && !OTHER_EFFECTS.contains(&e.kind.as_str()) {
                bail!("{what}: no effect {:?} (one of {CC_KINDS:?} or {OTHER_EFFECTS:?})", e.kind);
            }
            if e.wall_stun < 0.0 || (e.wall_stun > 0.0 && e.kind != "knockback") {
                bail!("{what}: only a knockback stuns against a wall (`wall_stun`)");
            }
            if !e.pop.is_empty() && e.kind != "shield" {
                bail!("{what}: only a shield bursts as it ends (`pop`)");
            }
            if e.kind == "evasion" && !(self.dodge.evade_from..=self.dodge.evade_to).contains(&e.amount) {
                bail!("{what}: evasion widens the perfect dodge to `amount` sec, inside the dodge's evade window");
            }
            if e.kind == "venom" && e.amount < 1.0 {
                bail!("{what}: venom lays a whole number of stacks (`amount`, 1 or more)");
            }
            if e.kind == "cheat_death" && !(0.0..=1.0).contains(&e.amount) {
                bail!("{what}: cheat_death mends a share of their health (`amount`, 0 to 1)");
            }
            Ok(())
        };
        for a in &self.abilities {
            let what = format!("ability {}", a.key);
            if !keys.insert(a.key.clone()) {
                bail!("two abilities called {}", a.key);
            }
            if !KINDS.contains(&a.kind.as_str()) {
                bail!("{what}: kind {:?} (one of {KINDS:?})", a.kind);
            }
            if !SLOTS.contains(&a.slot.as_str()) {
                bail!("{what}: slot {:?} (one of {SLOTS:?})", a.slot);
            }
            if !BODIES.contains(&a.body.as_str()) {
                bail!("{what}: body {:?} (\"\" for the torso while the legs walk, or \"full\")", a.body);
            }
            if a.school != "core" && !a.is_beta() && !SCHOOLS.contains(&a.school.as_str()) {
                bail!("{what}: school {:?} (core, {BETA} or one of {SCHOOLS:?})", a.school);
            }
            if !a.is_ex() && !a.inner && !a.is_beta() && !SCHOOLS.contains(&a.school.as_str()) {
                bail!("{what}: a card needs its column, `school` (one of {SCHOOLS:?}, or {BETA} for the beta page)");
            }
            if a.bow_damage > 0.0 && (!a.bow || a.kind != "projectile" || a.damage[1] > 0.0) {
                bail!("{what}: a share of the bow's damage (`bow_damage`) is for a bow's projectile, in place of its own `damage`");
            }
            if a.kind == "art" && a.weapon <= 0.0 {
                bail!("{what}: a weapon art needs its share of the weapon's damage (`weapon`)");
            }
            if a.at_self && !matches!(a.kind.as_str(), "ground" | "trap") {
                bail!("{what}: only ground and traps are laid `at_self`");
            }
            if a.kind == "spread" && (a.range[1] <= 0.0 || a.splash <= 0.0) {
                bail!("{what}: a spread needs its reach (`range`) and the radius it spreads over (`splash`)");
            }
            if a.kind == "trap" && (a.radius[1] <= 0.0 || a.secs <= 0.0) {
                bail!("{what}: a trap needs its reach (`radius`) and its life (`secs`)");
            }
            if a.kind == "line" && (a.range[1] <= 0.0 || a.radius[1] <= 0.0) {
                bail!("{what}: a strip needs its length (`range`) and half-width (`radius`)");
            }
            if a.kind == "ally" && !a.foe_damage && (a.damage[0] > 0.0 || a.damage[1] > 0.0) {
                bail!("{what}: an ally ability helps whoever it lands on: no `damage` (unless `foe_damage`: a foe on the line takes it)");
            }
            if a.kind == "beam" && ((a.range[1] <= 0.0 && !a.around) || a.radius[1] <= 0.0 || a.secs <= 0.0) {
                bail!("{what}: a beam needs its length (`range`), half-width (`radius`) and how long it's held (`secs`)");
            }
            if a.cast > 0.0 && !matches!(a.kind.as_str(), "cone" | "nova" | "line") {
                bail!("{what}: only a cone, a nova or a strip winds up (`cast`)");
            }
            if (a.pierce > 0 || a.full_pierce > 0 || a.chain > 0 || a.splash > 0.0) && a.kind != "projectile" && a.kind != "trap" && a.kind != "spread" {
                bail!("{what}: pierce, chain and splash are a projectile's");
            }
            // One boon, or several joined with `+` (Hallowed Ground's `hallowed+steadfast`).
            if !a.boon.is_empty() && !a.boon.split('+').all(|b| BOONS.contains(&b)) {
                bail!("{what}: boon {:?} (one of {BOONS:?}, or several joined with +)", a.boon);
            }
            if a.charge[1] < a.charge[0] {
                bail!("{what}: its full charge comes before its least");
            }
            if matches!(a.kind.as_str(), "projectile") && a.speed <= 0.0 {
                bail!("{what}: a projectile needs a speed");
            }
            if (a.returns || a.lob || a.fuse > 0.0) && a.kind != "projectile" {
                bail!("{what}: only a projectile comes back (`returns`), is lobbed (`lob`) or sticks (`fuse`)");
            }
            if u8::from(a.returns) + u8::from(a.lob) + u8::from(a.fuse > 0.0) > 1 {
                bail!("{what}: a projectile comes back, is lobbed or sticks: one of `returns`, `lob` and `fuse`");
            }
            if (a.lob || a.fuse > 0.0) && a.splash <= 0.0 {
                bail!("{what}: a lobbed or sticky projectile bursts where it goes off: it needs `splash`");
            }
            // (A banner's recast moves the banner its first cast planted: it needs none of its own.)
            let banner = a.banner.is_some() || (a.inner && self.abilities.iter().any(|b| b.banner.is_some() && b.recast == a.key));
            if a.detonate < 0.0 || (a.detonate > 0.0 && (a.kind != "place" || a.totem.is_none() || a.effects.is_empty())) {
                bail!("{what}: only a placed totem with `effects` waits to go off (`detonate`, seconds)");
            }
            if a.kind == "place" && a.hollow.is_none() && a.totem.is_some() == banner {
                bail!("{what}: a placed ability needs a [ability.totem], a `banner` or a `hollow` (one of them)");
            }
            if a.banner.is_some() && (a.kind != "place" || a.object == 0 || a.secs <= 0.0) {
                bail!("{what}: a banner is placed (`kind = \"place\"`), with its look (`object`) and how long it stands (`secs`)");
            }
            if a.banner.as_ref().is_some_and(|b| b.health <= 0.0) || (a.banner.is_some() && a.radius[1] <= 0.0) {
                bail!("{what}: a banner needs its `health`, and the reach it helps in (`radius`)");
            }
            if a.breaks_free && a.kind != "buff" {
                bail!("{what}: only a buff breaks free (`breaks_free`)");
            }
            if !(0.0..1.0).contains(&a.health_cost) || (a.health_cost > 0.0 && a.kind != "buff") {
                bail!("{what}: only a buff costs health, a share of it below 1 (`health_cost`)");
            }
            if !a.passive.is_empty() && (!PASSIVES.contains(&a.passive.as_str()) || a.is_ex() || a.inner) {
                bail!("{what}: passive {:?} (one of {PASSIVES:?}, on a card: not an EX version or an inner one)", a.passive);
            }
            if a.kind == "stance" && a.counter.is_none() {
                bail!("{what}: a stance needs a [ability.counter]");
            }
            if !a.recast.is_empty() && a.recast_secs <= 0.0 {
                bail!("{what}: a recast needs its window (`recast_secs`)");
            }
            if a.recast.is_empty() && a.recast_secs > 0.0 {
                bail!("{what}: `recast_secs` with no `recast`");
            }
            if a.recast_to_start && !matches!(a.kind.as_str(), "blink" | "leap") {
                bail!("{what}: only a blink or a leap goes back to the start (`recast_to_start`)");
            }
            if a.through && a.kind != "dash" {
                bail!("{what}: only a dash goes `through`");
            }
            if !a.hunt.is_empty() && !crate::content_abilities::HUNTS.contains(&a.hunt.as_str()) {
                bail!("{what}: hunt {:?} (one of {:?})", a.hunt, crate::content_abilities::HUNTS);
            }
            // The Rogue's column has a third: its three trees (docs/rogue-roles.md).
            let most = if a.school == "rogue" { 3 } else { 2 };
            if !(1..=most).contains(&a.choice) {
                bail!("{what}: choice {} (1 or 2: a class has two a row; the Rogue three)", a.choice);
            }
            // The Warlord's mechanics.
            if a.crash.is_some() && a.kind != "dash" {
                bail!("{what}: only a dash carries a body to Crash it (`crash`)");
            }
            if a.intervene > 0.0 && a.kind != "ally" {
                bail!("{what}: only an ally ability rushes to a friend to take a blow (`intervene`)");
            }
            if (a.ally_stamina > 0.0 || !a.ally_effects.is_empty()) && a.kind != "nova" {
                bail!("{what}: only a nova gives friends stamina or boons (`ally_stamina`, `ally_effects`)");
            }
            if a.ally_effects.iter().any(|e| !BOONS.contains(&e.kind.as_str())) {
                bail!("{what}: `ally_effects` are boons (one of {BOONS:?})");
            }
            if (a.around || a.ramp > 0.0) && a.kind != "beam" {
                bail!("{what}: only a beam spins round the caster (`around`) or ramps (`ramp`)");
            }
            if a.ramp > 0.0 && a.ramp_cap <= 0.0 {
                bail!("{what}: a ramp needs its most a strike (`ramp_cap`)");
            }
            if a.bail && a.kind != "beam" {
                bail!("{what}: only a beam is ended into a guard (`bail`)");
            }
            if a.bounces > 0 && a.kind != "projectile" {
                bail!("{what}: only a projectile ricochets (`bounces`)");
            }
            if a.empower.as_ref().is_some_and(|e| e.secs <= 0.0) {
                bail!("{what}: an empower needs its `secs`");
            }
            // A leap's own look: on a leap, from a character model the clients can find.
            if (a.leap_clip != 0 || a.leap_flip || !a.leap_from.is_empty()) && a.kind != "leap" {
                bail!("{what}: only a leap has a leap's look (`leap_clip`, `leap_from`, `leap_flip`)");
            }
            if !a.leap_from.is_empty() && (a.leap_clip == 0 || !clip_source_ok(&a.leap_from)) {
                bail!("{what}: leap_from {:?} needs a `leap_clip` and a character model (<Race><Gender>, as ScourgeFemale)", a.leap_from);
            }
            // A hauler's own look: on a pull, from a character model the clients can find.
            if (a.pull_clip != 0 || !a.pull_clips.is_empty() || !a.pull_from.is_empty()) && !a.effects.iter().any(|e| e.kind == "pull") {
                bail!("{what}: only an ability with a `pull` effect has the hauler's look (`pull_clip`, `pull_clips`, `pull_from`)");
            }
            if a.pull_clips.contains(&0) {
                bail!("{what}: pull_clips names clips, not 0");
            }
            if !a.pull_from.is_empty() && ((a.pull_clip == 0 && a.pull_clips.is_empty()) || !clip_source_ok(&a.pull_from)) {
                bail!("{what}: pull_from {:?} needs a `pull_clip` and a character model (<Race><Gender>, as ScourgeFemale)", a.pull_from);
            }
            // The beta page's mechanics.
            let tether = a.tether_secs > 0.0;
            if a.yank && a.kind != "projectile" {
                bail!("{what}: only a projectile pulls its caster after it (`yank`)");
            }
            if (a.swap || tether) && a.kind != "projectile" {
                bail!("{what}: only a projectile swaps places (`swap`) or binds (`tether_secs`)");
            }
            if u8::from(a.yank) + u8::from(a.swap) + u8::from(tether) > 1 {
                bail!("{what}: a projectile yanks, swaps or binds: one of `yank`, `swap` and `tether_secs`");
            }
            if (a.yank || a.swap || tether) && (a.returns || a.lob || a.fuse > 0.0 || a.pierce > 0 || a.count > 1) {
                bail!("{what}: a yank, a swap or a tether is one projectile that stops at what it strikes (no `returns`, `lob`, `fuse`, `pierce` or `count`)");
            }
            if a.swap && a.chain > 0 {
                bail!("{what}: a swap goes with one (no `chain`)");
            }
            if tether != (a.tether_range > 0.0) || a.tether_secs < 0.0 {
                bail!("{what}: a tether needs how long it binds (`tether_secs`) and how far it lets them go (`tether_range`)");
            }
            if !tether && !a.tether_effects.is_empty() {
                bail!("{what}: `tether_effects` with no tether (`tether_secs`)");
            }
            if a.rewind && a.kind != "buff" {
                bail!("{what}: only a buff leaves an echo to come back to (`rewind`)");
            }
            if a.rewind && a.recast.is_empty() == (a.rewind_secs <= 0.0) {
                bail!("{what}: an echo is come back to by its recast (`recast`) or after a while (`rewind_secs`): one of them");
            }
            if !a.rewind && a.rewind_secs > 0.0 {
                bail!("{what}: `rewind_secs` with no echo (`rewind`)");
            }
            if a.launch < 0.0 || (a.launch > 0.0 && a.kind != "trap") {
                bail!("{what}: only a trap launches (`launch`)");
            }
            // The Priest's core.
            if a.friends_only && (a.kind != "projectile" || a.ally_heal <= 0.0) {
                bail!("{what}: only a healing projectile passes foes by (`friends_only` needs `ally_heal`)");
            }
            if a.penance.is_some() != (a.kind == "penance") {
                bail!("{what}: a `penance` kind and its `penance` table go together");
            }
            if a.feather.is_some() != (a.kind == "feather") {
                bail!("{what}: a `feather` kind and its `feather` table go together");
            }
            if a.shell.is_some() != (a.kind == "shell") {
                bail!("{what}: a `shell` kind and its `shell` table go together");
            }
            if let Some(sd) = &a.shell {
                if self.ability(&sd.shard).is_none_or(|x| self.abilities[x].kind != "projectile") {
                    bail!("{what}: its shell's `shard` must be a projectile");
                }
            }
            if a.rift.is_some() && a.kind != "ground" {
                bail!("{what}: only a ground cast opens a rift (`rift`)");
            }
            if let Some(h) = &a.hollow {
                if a.kind != "place" || a.totem.is_some() || a.banner.is_some() {
                    bail!("{what}: only a placed thing is a Hollow Star (`hollow`), and it's no totem or banner");
                }
                if self.mob(&h.mob).is_none_or(|m| !self.mobs[m].totem) {
                    bail!("{what}: its Hollow Star's body {} must be a totem [[mob]]", h.mob);
                }
            }
            if a.tether.is_some() && a.kind != "channel" {
                bail!("{what}: only a channel is a tether (`tether`)");
            }
            if a.fetch && (a.kind != "ally" || !a.effects.iter().any(|e| e.kind == "pull" && e.from_caster)) {
                bail!("{what}: only an `ally` with a `pull` from the caster fetches a friend (`fetch`)");
            }
            if a.parry_for && (a.kind != "ally" || !a.effects.iter().any(|e| e.kind == "shield")) {
                bail!("{what}: only a shield given to a friend (`ally` with a `shield`) parries for them (`parry_for`)");
            }
            if !a.end_burst.is_empty() && a.linger <= 0.0 {
                bail!("{what}: only lingering ground bursts as it ends (`end_burst` needs `linger`)");
            }
            if let Some(c) = a.counter.as_ref().filter(|c| c.mirror) {
                if c.block || c.dodge || c.immune || c.absorb_heal > 0.0 || !c.reflect.is_empty() || c.melee_only {
                    bail!("{what}: a mirror only turns projectiles back (no block, dodge, immune, absorb_heal, reflect or melee_only with it)");
                }
            }
            if a.slip && a.kind != "blink" && !(a.kind == "dash" && a.sidestep) {
                bail!("{what}: only a blink or a sidestep slips blows (`slip`)");
            }
            if a.sidestep && a.kind != "dash" {
                bail!("{what}: only a dash is a sidestep (`sidestep`)");
            }
            // The Shadow Priest's mechanics.
            if (a.slip_radius > 0.0 || !a.slip_effects.is_empty() || !a.slip_self.is_empty()) && !a.slip {
                bail!("{what}: `slip_radius`, `slip_effects` and `slip_self` are a slip's (they need `slip`)");
            }
            if let Some(f) = &a.flay {
                if a.kind != "channel" || f.taut <= 0.0 || f.taut >= a.range[1] || f.taut_secs <= 0.0 || f.snap <= 0.0 || f.snap_cap <= 0.0 {
                    bail!("{what}: a flay is a channel's, taut from `taut` yd short of its range, with `taut_secs`, `snap` and `snap_cap`");
                }
                if (f.spill > 0.0) != (f.spill_radius > 0.0) {
                    bail!("{what}: a flay's `spill` and `spill_radius` go together");
                }
            }
            if a.on_taut < 0.0 || (a.on_taut > 0.0 && a.kind != "projectile") {
                bail!("{what}: only a projectile snaps a taut cord (`on_taut`)");
            }
            if (a.low_health > 0.0) != (a.low_mult > 1.0) || !(0.0..1.0).contains(&a.low_health) {
                bail!("{what}: `low_health` (below 1) and `low_mult` (above 1) go together");
            }
            if a.backlash < 0.0 {
                bail!("{what}: `backlash` is damage back at the caster (0 or more)");
            }
            if a.counter.as_ref().is_some_and(|c| !(0.0..1.0).contains(&c.cut) || c.mend < 0.0) {
                bail!("{what}: a stance's `cut` is a share below 1, its `mend` 0 or more");
            }
            if a.counter.as_ref().is_some_and(|c| !c.perfect_pass.is_empty() && (c.immune || c.mirror || c.block || c.dodge)) {
                bail!("{what}: `perfect_pass` lets a blow through clean (no immune, mirror, block or dodge with it)");
            }
            for e in a.slip_effects.iter().chain(&a.slip_self).chain(a.counter.iter().flat_map(|c| c.perfect_pass.iter())).chain(&a.behind_effects) {
                effect_ok(e, &what)?;
            }
            // The Rogue's Shadow: a blink to a back, an art whose effects want one.
            if a.behind_foe && a.kind != "blink" {
                bail!("{what}: only a blink lands behind a foe (`behind_foe`)");
            }
            if (!a.behind_effects.is_empty() || a.calm_only) && a.kind != "art" {
                bail!("{what}: only a weapon art has effects from behind (`behind_effects`, `calm_only`)");
            }
            // The Rogue's Venom: an art spends it or breaks a cast from behind.
            if (a.per_venom > 0.0 || a.breaks_cast > 0.0) && a.kind != "art" {
                bail!("{what}: only a weapon art spends Venom or breaks a cast (`per_venom`, `breaks_cast`)");
            }
            // The Rogue's Blades.
            if (a.quickstep || a.cuts > 0) && a.kind != "buff" {
                bail!("{what}: only a buff is a Quickstep or a Blade Flurry (`quickstep`, `cuts`)");
            }
            if a.shade > 0.0 && !a.quickstep {
                bail!("{what}: only a Quickstep leaves a shade (`shade`)");
            }
            if a.hand && a.kind != "projectile" {
                bail!("{what}: only a projectile counts as a hand in the two-blade run (`hand`)");
            }
            if a.shade > 0.0 && self.mob(&self.rogue.shade).is_none() {
                bail!("{what}: [rogue] shade {:?} is no creature", self.rogue.shade);
            }
            if a.counter.as_ref().is_some_and(|c| !c.perfect_break.is_empty() && !c.immune) {
                bail!("{what}: `perfect_break` is an immune stance's (Ice Block's) check");
            }
            if let Some(c) = &a.counter {
                for e in &c.perfect_break {
                    effect_ok(e, &what)?;
                }
            }
            if a.counter.as_ref().is_some_and(|c| c.perfect_mirror && !c.mirror) {
                bail!("{what}: `perfect_mirror` is a mirror's check (it needs `mirror`)");
            }
            if (a.pulse < 0.0 || a.rewind_health < 0.0 || a.rewind_health > 1.0) || ((a.pulse > 0.0 || a.rewind_health > 0.0) && a.anchor <= 0.0) {
                bail!("{what}: `pulse` and `rewind_health` (0 to 1) are an anchor's (they need `anchor`)");
            }
            // The Light Priest's.
            if a.kind == "sigil" && (a.beats.is_empty() || a.beats.iter().any(|b| *b <= 0.0) || a.radius[1] <= 0.0 || a.stack.is_none()) {
                bail!("{what}: a sigil needs its `beats` (each after the last), its `radius` and its `stack`");
            }
            if a.kind != "sigil" && (!a.beats.is_empty() || a.stack.is_some() || a.per_lander > 0.0) {
                bail!("{what}: `beats`, `stack` and `per_lander` are a sigil's");
            }
            if a.stack.as_ref().is_some_and(|k| k.most == 0 || k.might < 0.0 || k.secs <= 0.0) {
                bail!("{what}: a stack needs `most`, `might` and `secs`");
            }
            for e in a.effects.iter().chain(&a.self_effects).chain(&a.tick_effects).chain(&a.tether_effects).chain(&a.ally_effects).chain(&a.crit_effects).chain(&a.stop_effects).chain(&a.finisher_effects).chain(&a.bomb_effects).chain(&a.backfire_effects).chain(&a.strike_back) {
                effect_ok(e, &what)?;
            }
            if let Some(c) = &a.counter {
                for e in &c.attacker {
                    effect_ok(e, &what)?;
                }
            }
        }
        for a in &self.abilities {
            let what = format!("ability {}", a.key);
            if a.is_ex() {
                let Some(b) = self.ability(&a.ex_of) else { bail!("{what}: the EX of {}, no such ability", a.ex_of) };
                if self.abilities[b].is_ex() || self.abilities[b].slot != a.slot {
                    bail!("{what}: the EX of {} must take its slot, and not be an EX itself", a.ex_of);
                }
            }
            for k in [&a.leaves, &a.also].into_iter().chain(a.counter.as_ref().map(|c| &c.reflect)) {
                if !k.is_empty() && self.ability(k).is_none() {
                    bail!("{what}: no ability {k}");
                }
            }
            // A recast is an inner ability, fired only by the key again; it arms nothing itself.
            if !a.recast.is_empty() {
                let Some(r) = self.ability(&a.recast) else { bail!("{what}: recast {}, no such ability", a.recast) };
                let r = &self.abilities[r];
                if !r.inner || r.key == a.key {
                    bail!("{what}: its recast {} must be an `inner` ability of its own", a.recast);
                }
                // A chain (Raptor Strike): a recast may arm one of its own, never back round.
                let mut seen = vec![a.key.clone()];
                let mut next = r.recast.clone();
                while !next.is_empty() {
                    if seen.contains(&next) {
                        bail!("{what}: its recasts come back round to {next}");
                    }
                    let Some(k) = self.ability(&next) else { bail!("{what}: recast {next}, no such ability") };
                    if !self.abilities[k].inner {
                        bail!("{what}: its chain's recast {next} must be an `inner` ability of its own");
                    }
                    seen.push(next.clone());
                    next = self.abilities[k].recast.clone();
                }
            }
            // A burst as ground ends, or as a shield does, is a nova (a shield's with its own reach).
            let pops = a.effects.iter().chain(&a.self_effects).filter(|e| !e.pop.is_empty()).map(|e| (&e.pop, true));
            for (k, own) in std::iter::once((&a.end_burst, false)).filter(|(k, _)| !k.is_empty()).chain(pops) {
                let Some(b) = self.ability(k) else { bail!("{what}: bursts as {k}, no such ability") };
                let b = &self.abilities[b];
                if b.kind != "nova" || (own && b.radius[1] <= 0.0) {
                    bail!("{what}: bursts as {k}, which must be a `nova`{}", if own { " with its `radius`" } else { "" });
                }
            }
            // An empower names a base card with an EX version (the one it makes free).
            if let Some(e) = &a.empower {
                let Some(b) = self.ability(&e.ability) else { bail!("{what}: empowers {}, no such ability", e.ability) };
                if self.abilities[b].is_ex() || self.abilities[b].inner || self.ex_of(b).is_none() {
                    bail!("{what}: empowers {}, which must be a base card with an EX version", e.ability);
                }
                // An empowered key fires at once; an ultimate's waits for its hold, so none is.
                if self.abilities[b].slot == "ultimate" {
                    bail!("{what}: empowers {}, an ultimate (its key waits for a hold)", e.ability);
                }
            }
        }
        // The board: a card a column at most in each row; the beta page, four a row.
        for row in SLOTS {
            let (beta, cards): (Vec<&AbilityDef>, Vec<&AbilityDef>) = self.abilities.iter().filter(|a| a.on_board() && a.slot == row).partition(|a| a.is_beta());
            // Two a class, and the Rogue's third.
            if cards.len() > SCHOOLS.len() * 2 + 1 {
                bail!("the board's {row} row: {} cards, at most {}", cards.len(), SCHOOLS.len() * 2 + 1);
            }
            if beta.len() > BETA_ROW {
                bail!("the beta page's {row} row: {} cards, at most {BETA_ROW}", beta.len());
            }
            for col in SCHOOLS {
                for choice in [1, 2, 3] {
                    if cards.iter().filter(|a| a.school == col && a.choice == choice).count() > 1 {
                        bail!("the board's {row} row: two {col} cards as its choice {choice}");
                    }
                }
            }
        }
        for (slot, k) in &self.loadout {
            if !SLOTS.contains(&slot.as_str()) {
                bail!("loadout: no slot {slot} (one of {SLOTS:?})");
            }
            let Some(a) = self.ability(k) else { bail!("loadout: no ability {k}") };
            if self.abilities[a].slot != slot_role(slot) || self.abilities[a].is_ex() {
                bail!("loadout: {k} doesn't go on {slot}");
            }
        }
        Ok(())
    }

    pub fn item(&self, key: &str) -> Option<usize> {
        self.items.iter().position(|i| i.key == key)
    }
    pub fn station(&self, key: &str) -> Option<usize> {
        self.stations.iter().position(|s| s.key == key)
    }
    pub fn mob(&self, key: &str) -> Option<usize> {
        self.mobs.iter().position(|m| m.key == key)
    }
    /// A dungeon's own gear: what a foe of one of its packs carries (`gear`) or yields
    /// (`trophy`). It's earned down there: the rack at camp offers it once the camp has found one.
    pub fn is_dungeon_gear(&self, key: &str) -> bool {
        self.dungeons().any(|d| {
            d.packs.iter().any(|p| self.mob(&p.mob).is_some_and(|m| self.mobs[m].gear.iter().any(|g| g == key) || self.mobs[m].trophy.contains_key(key)))
        })
    }

    /// A piece's own key, a honed copy's that of the piece it was honed from.
    pub fn base_key(&self, item: usize) -> &str {
        let it = &self.items[item];
        if it.honed_from.is_empty() { &it.key } else { &it.honed_from }
    }

    /// Whether an item is some champion's trophy (won, never on the rack).
    pub fn is_trophy(&self, key: &str) -> bool {
        self.mobs.iter().any(|m| m.trophy.contains_key(key)) || self.chests.iter().any(|c| c.loot.contains_key(key) || c.roll.contains_key(key))
    }
    pub fn weapon(&self, key: &str) -> Option<&WeaponDef> {
        self.weapons.iter().find(|w| w.key == key)
    }
    /// A weapon's set of blows (the first set when it names none there is).
    pub fn moveset_of(&self, w: &WeaponDef) -> &MovesetDef {
        let key = w.moveset_key();
        self.movesets.iter().find(|m| m.key == key).unwrap_or(&self.movesets[0])
    }
    pub fn shield(&self, key: &str) -> Option<&ShieldDef> {
        self.shields.iter().find(|s| s.key == key)
    }
    pub fn ranged(&self, key: &str) -> Option<&RangedDef> {
        self.ranged.iter().find(|r| r.key == key)
    }
    pub fn ability(&self, key: &str) -> Option<usize> {
        self.abilities.iter().position(|a| a.key == key)
    }
    /// An ability's EX version, if it has one.
    pub fn ex_of(&self, base: usize) -> Option<usize> {
        let key = &self.abilities[base].key;
        self.abilities.iter().position(|a| &a.ex_of == key)
    }
    /// The ability whose cooldown this one runs on (an EX version, its base's).
    pub fn base_of(&self, a: usize) -> usize {
        let d = &self.abilities[a];
        if d.is_ex() {
            self.ability(&d.ex_of).unwrap_or(a)
        } else {
            a
        }
    }
    /// A War Banner's: it plants a `banner`, or it's the recast of one that does (which moves it).
    pub fn plants_banner(&self, a: usize) -> bool {
        let d = &self.abilities[a];
        d.banner.is_some() || (d.inner && self.abilities.iter().any(|b| b.banner.is_some() && b.recast == d.key))
    }
}

#[cfg(test)]
mod tests {
    /// What the player reads about an ability follows the style guide's word list and has none of
    /// its banned marks: no semicolons, no keys' `~` or `^`, never "cards" (Elwynn Survival has
    /// abilities), no double spaces or stray ends, "sec" and "yd" never spelled out.
    #[test]
    fn ability_text_keeps_to_the_style_guide() {
        let c = super::Content::builtin();
        for a in &c.abilities {
            let t = &a.text;
            assert!(!t.is_empty() || a.inner, "{} has no text", a.key);
            for bad in [";", "~", "^", "  ", "TODO", "TBD", "lorem", " yards", " seconds", " yard "] {
                assert!(!t.contains(bad), "{}: {bad:?} in {t:?}", a.key);
            }
            assert!(!t.to_lowercase().split(|ch: char| !ch.is_alphabetic()).any(|w| w == "card" || w == "cards"), "{}: {t:?}", a.key);
            assert_eq!(t.trim(), t, "{}: stray space", a.key);
        }
    }

    #[test]
    fn the_builtin_content_reads() {
        let c = super::Content::builtin();
        assert!(c.items.len() >= 5);
        assert!(c.mob("grunt").is_some());
    }

    /// The built-in rules with one more ability (TOML, an `[[ability]]` table's fields), checked.
    fn with(extra: &str) -> anyhow::Result<()> {
        let mut c = super::Content::dev();
        let a: super::AbilityDef = toml::from_str(extra).expect("a test ability");
        c.abilities.push(a);
        c.validate_abilities()
    }

    #[test]
    fn recasts_dashes_through_and_empowers_are_checked_as_they_load() {
        let base = "key = \"t_a\"\nname = \"A\"\nschool = \"mage\"\nslot = \"ranged\"\nbench = true\nspell = 1\n";
        // A recast must be an inner ability, with its window.
        assert!(with(&format!("{base}kind = \"blink\"\nrecast = \"fire_blast\"\nrecast_secs = 3.0")).is_ok());
        assert!(with(&format!("{base}kind = \"blink\"\nrecast = \"fire_blast\"")).is_err(), "no window");
        assert!(with(&format!("{base}kind = \"blink\"\nrecast = \"fireball\"\nrecast_secs = 3.0")).is_err(), "a card, not inner");
        assert!(with(&format!("{base}kind = \"blink\"\nrecast = \"nothing\"\nrecast_secs = 3.0")).is_err(), "no such ability");
        assert!(with(&format!("{base}kind = \"nova\"\nrecast_to_start = true")).is_err(), "only a blink or a leap goes back");
        // Only a dash goes through.
        assert!(with(&format!("{base}kind = \"dash\"\nthrough = true")).is_ok());
        assert!(with(&format!("{base}kind = \"leap\"\nthrough = true")).is_err());
        // An empower names a base card with an EX version.
        assert!(with(&format!("{base}kind = \"nova\"\nempower = {{ ability = \"fireball\", secs = 4.0 }}")).is_ok());
        assert!(with(&format!("{base}kind = \"nova\"\nempower = {{ ability = \"pyroblast\", secs = 4.0 }}")).is_err(), "an EX version");
        assert!(with(&format!("{base}kind = \"nova\"\nempower = {{ ability = \"meteor\", secs = 4.0 }}")).is_err(), "no EX version");
        assert!(with(&format!("{base}kind = \"nova\"\nempower = {{ ability = \"fireball\", secs = 0.0 }}")).is_err(), "no time");
    }
}

impl NodeDef {
    /// What the node is known by.
    pub fn id(&self) -> &str {
        if self.key.is_empty() {
            &self.item
        } else {
            &self.key
        }
    }
}

/// A hit in a creature's back, by anyone (a swing, an art, an arrow): the backstab. Besides the
/// riposte's stab (kit 330, Backstab's chest impact: it plays on the chest, the side turned away
/// from whoever stabbed it, so from behind it shows nothing), the sound and the beat, a look of
/// its own on the creature that reads from behind.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct BackstabDef {
    /// The look (`SpellVisualKit`) on the stabbed creature; 0 for none. `.es tune backstab.kit
    /// <id>` to film another: 220 `BloodyExplosion` (Rend's and Rupture's blood burst), 290 (the
    /// same burst as Shred and Maul play it), 815 (as Rip plays it), 3051 (as Hamstring does).
    pub kit: u32,
}

impl Default for BackstabDef {
    fn default() -> Self {
        Self { kit: 220 }
    }
}

/// A hero created as a Rogue (docs/rogue-roles.md, "The core"): what its hands may hold and what
/// they start with, the Shadow Cut a perfect dodge opens, and how a perfect dodge shows.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct RogueDef {
    /// The main hand and the off hand a new Rogue starts with.
    pub weapon: String,
    pub off_hand: String,
    /// The weapon kinds a Rogue may hold, by the client's item subclass (15 dagger, 7 sword, 4
    /// mace, 13 fist), one-handed only. No shield, two-hander, staff or axe.
    pub weapons: Vec<u32>,
    /// The Shadow Cut: this long after a perfect dodge, either attack steps behind the one who
    /// swung, up to this far away, and strikes from there.
    pub cut_secs: f32,
    pub cut_reach: f32,
    /// A dodge cuts an off-hand blow's follow-through short from this share of its recovery
    /// (`[combat] cancel_after` for the main hand).
    pub off_cancel_after: f32,
    /// An empty off hand jabs for this share of the main-hand weapon's blow.
    pub jab: f32,
    /// The perfect dodge's flicker: this spell's state look on the body for this long.
    pub flicker_aura: u32,
    pub flicker_secs: f32,
    /// The puff of smoke where the blow fell, and at both ends of the Shadow Cut's step.
    pub puff: u32,
    /// A tree's trait is on while this many of a Rogue's 5 keys hold that tree's cards (a Rogue
    /// card's tree is its `choice`: 1 Shadow).
    pub trait_keys: usize,
    /// Shadow's trait, Shrouded Step: a step that lands behind an enemy hides the Rogue this long
    /// from every creature but a boss (none starts a blow at them), shown by this look on the body.
    pub shroud_secs: f32,
    pub shroud_aura: u32,
    /// Distract on a boss: it only glances, this long, at most once in this long, never mid-string.
    pub glance_secs: f32,
    pub glance_every: f32,
    /// Cloak and Dagger: each attack steps behind an enemy this near first.
    pub cloak_reach: f32,
    /// Venom (`sim::rogue`): stacks on an enemy up to this many, for this long from the latest,
    /// each dealing this much damage a second. Its look on the body (a deeper one once it's ripe,
    /// at the most), and the drip of its ticks (a spell visual kit).
    pub venom_most: u32,
    pub venom_secs: f32,
    pub venom_dps: f32,
    pub venom_aura: u32,
    pub ripe_aura: u32,
    pub venom_drip: u32,
    /// Feint: a creature on the Rogue turns to another hero this near it, else loses interest.
    pub feint_turn: f32,
    /// Blades' trait, Flourish (`sim::rogue`): each perfect dodge gives one, up to this many, for
    /// this long from the latest. Each makes the Rogue's blows this much harder and this much
    /// quicker; a blow landing on the Rogue takes them all. Its look on the body while it holds.
    pub flourish_most: u32,
    pub flourish_secs: f32,
    pub flourish_damage: f32,
    pub flourish_haste: f32,
    pub flourish_aura: u32,
    /// Quickstep's dash: this fast along the ground for the dodge's `dash_secs`.
    pub quick_speed: f32,
    /// A dodge's stamina under Evasion.
    pub evasion_cost: f32,
    /// Blade Flurry: its cuts this far apart, each lunging this far in.
    pub flurry_gap: f32,
    pub flurry_step: f32,
    /// A Darting Knife that strikes holds the two-blade run this long for the next blow.
    pub knife_window: f32,
    /// Pinning Knife on a boss: it turns at this share of its speed (a creature not at all).
    pub pinned_boss_turn: f32,
    /// Afterimage's shade: the creature (`[[mob]]` key) left where the Rogue stood.
    pub shade: String,
    /// The dual-wield pairs (docs/rogue-roles.md, "Weapon paths"; `sim::weapon_fx`, `Pair`): what a
    /// Rogue's two hands add to the two-blade finisher. Crossed Swords' cross-cut strikes every
    /// enemy in this arc (degrees) in reach.
    pub cross_arc: f32,
    /// Breaker's Pair: its finisher counts this many times toward a champion's posture.
    pub breaker_heft: f32,
    /// Iron Knuckles: a run this many blows long (the others' 4); its finisher wheels a creature
    /// (not a boss) round, its back to the Rogue, for this long (a Distract away from the Rogue),
    /// and counts this many times toward a boss's posture.
    pub knuckle_run: u8,
    pub wheel_secs: f32,
    pub knuckle_boss_heft: f32,
    /// Iron Knuckles' finisher: both fists at once (`AttackUnarmed`), in place of `Attack1H`.
    pub knuckle_clip: u16,
}

impl Default for RogueDef {
    fn default() -> Self {
        Self {
            weapon: "dagger".into(),
            off_hand: "dagger".into(),
            weapons: vec![15, 7, 4, 13],
            cut_secs: 0.6,
            cut_reach: 6.0,
            off_cancel_after: 0.0,
            jab: 0.5,
            flicker_aura: 5215,
            flicker_secs: 0.3,
            puff: 1856,
            trait_keys: 3,
            shroud_secs: 1.0,
            shroud_aura: 5215,
            glance_secs: 0.8,
            glance_every: 15.0,
            cloak_reach: 8.0,
            venom_most: 5,
            venom_secs: 8.0,
            venom_dps: 1.5,
            venom_aura: 12881,
            ripe_aura: 3815,
            venom_drip: 805,
            feint_turn: 10.0,
            flourish_most: 5,
            flourish_secs: 6.0,
            flourish_damage: 0.06,
            flourish_haste: 0.04,
            flourish_aura: 0,
            quick_speed: 20.0,
            evasion_cost: 8.0,
            flurry_gap: 0.3,
            flurry_step: 1.0,
            knife_window: 1.5,
            pinned_boss_turn: 0.5,
            shade: "rogue_shade".into(),
            cross_arc: 150.0,
            breaker_heft: 3.0,
            knuckle_run: 6,
            wheel_secs: 1.0,
            knuckle_boss_heft: 2.0,
            knuckle_clip: 16,
        }
    }
}

impl Content {
    /// Whether a Rogue may hold this item, and if not, the words why.
    pub fn rogue_can_hold(&self, item: usize) -> Result<(), &'static str> {
        let Some(d) = self.items.get(item) else { return Ok(()) };
        match d.kind {
            ItemKind::Shield => Err("Rogues can't use shields."),
            ItemKind::Weapon => match self.weapon(&d.key) {
                Some(w) if w.two_handed => Err("Rogues can't use two-handed weapons."),
                Some(w) if w.subclass == 0 => Err("Rogues can't use axes."),
                Some(w) if !self.rogue.weapons.contains(&w.subclass) => Err("Rogues can't use that weapon."),
                _ => Ok(()),
            },
            _ => Ok(()),
        }
    }
}
