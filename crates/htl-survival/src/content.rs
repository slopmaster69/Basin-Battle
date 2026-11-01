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
    #[serde(default)]
    pub heavy: HeavyDef,
    #[serde(default)]
    pub lock: LockDef,
    /// How creatures fight a band of heroes: whom they go for, when they press and when they
    /// give ground (`[ai]`).
    #[serde(default)]
    pub ai: AiDef,
    #[serde(default)]
    pub pvp: PvpDef,
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
    /// The Proving Ring: endless waves, a page of the same book.
    #[serde(default)]
    pub arena: Option<ArenaDef>,
    /// Siege: camps against camps in Arathi Basin, a page of the same book.
    #[serde(default)]
    pub siege: Option<SiegeDef>,
    /// The abilities on the ring tree, their EX versions and the counters.
    #[serde(rename = "ability", default)]
    pub abilities: Vec<AbilityDef>,
    #[serde(default)]
    pub valor: ValorDef,
    #[serde(default)]
    pub cc: CcDef,
    #[serde(default)]
    pub dev: DevDef,
    /// How long the work at the camp takes: making something at a station, building one, mending.
    #[serde(default)]
    pub work: WorkDef,
    /// A fresh hero's ability keys: slot to ability.
    #[serde(default)]
    pub loadout: LoadoutDef,
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
}

fn one_share() -> f32 {
    1.0
}

impl MoveDef {
    fn of(clip: u16, take: u32, windup: f32, recovery: f32, damage: f32, lunge: f32) -> Self {
        Self { clip, take, windup, recovery, damage, stamina: 1.0, lunge, range: 1.0 }
    }
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
    /// 10 yards off and `crowd` for every other creature already on them past the first; its
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
}

impl Default for AiDef {
    fn default() -> Self {
        Self {
            retarget_secs: 1.2,
            threat: 3.0,
            opening: 1.0,
            wounded: 0.8,
            distance: 1.0,
            crowd: 0.6,
            stick: 0.8,
            threat_half: 8.0,
            press_early: 1.2,
            guard_break: 3.0,
            retreat: 0.3,
            retreat_yards: 4.0,
            flank: true,
            jitter: 0.25,
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

/// A bow or crossbow.
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
    /// The Miner's Draught (an item key): the only heal below, charges refilled at a lamp.
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
    /// Where the run ends: past the front door, and how near counts.
    pub exit: [f32; 3],
    #[serde(default = "exit_radius")]
    pub exit_radius: f32,
    /// The boss (a pack's key) that must be down for the front door to open.
    #[serde(default)]
    pub exit_boss: String,
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
    /// In the build menu (a Siege camp's Hearth is not).
    #[serde(default = "yes")]
    pub menu: bool,
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

/// A tower's shot (`shoots` on a `[[station]]`).
#[derive(Debug, Clone, Deserialize)]
pub struct ShootDef {
    pub damage: u32,
    pub range: f32,
    /// Seconds between shots.
    pub every: f32,
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
    /// The clip it swings with (`AnimationData`; the creature's usual attack when not given).
    #[serde(default)]
    pub clip: Option<u16>,
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
    pub display: u32,
    #[serde(default = "one_f")]
    pub scale: f32,
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
    /// Its second wind: below a share of its health, a pattern at once and quicker blows after.
    #[serde(default)]
    pub enrage: Option<EnrageDef>,
    /// Its turns as it weakens (`[[mob.phase]]`, the enrage among them), highest share first:
    /// each a pattern at once, its pace and might from then, help called, a line said. New blows
    /// come with a pattern's `below`. Read in order once loaded (see [`Content::settle`]).
    #[serde(rename = "phase", default)]
    pub phases: Vec<EnrageDef>,
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
    /// A totem a hero sets down, or a training dummy: no mind of its own, it only stands there and
    /// is struck.
    #[serde(default)]
    pub totem: bool,
    #[serde(default)]
    pub dummy: bool,
    /// The cards the camp learns the first time it falls (a class's champion).
    #[serde(default)]
    pub teaches: Vec<String>,
    /// Harmless: it never fights back, and runs from a blow (a rabbit, a sheep).
    #[serde(default)]
    pub critter: bool,
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
}

impl MobDef {
    /// It stands and is struck, nothing more (a totem, a dummy).
    pub fn inert(&self) -> bool {
        self.totem || self.dummy
    }
}

/// A creature's second wind (`[mob.enrage]`).
#[derive(Debug, Clone, Deserialize)]
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
    pub const KINDS: [&'static str; 13] = ["bank", "build", "craft", "kill", "gather", "raid", "wear", "clear", "delve", "explore", "open", "secret", "learn"];
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
/// the camp has killed that creature; `ambush` wakes those packs (by key) as it's opened; `keep`
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
    /// A shrine's or a relic's trial: its guardians (packs by key that wake on it), called up
    /// each time a hero who has something to learn uses it; it teaches everyone near (and its
    /// first finder what's in it) only once they've all fallen.
    #[serde(default)]
    pub trial: Vec<String>,
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

/// A thing only to be seen (`[[prop]]`): a camp's fire, a cult's candles, a cage.
#[derive(Debug, Clone, Deserialize)]
pub struct PropDef {
    pub display: u32,
    pub at: [f32; 2],
    #[serde(default)]
    pub z: Option<f32>,
    #[serde(default = "one_f")]
    pub scale: f32,
    #[serde(default)]
    pub facing: f32,
}

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
        let mut c = Self::builtin();
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

    /// The dungeon (by key) a lamp is in.
    pub fn dungeon_of_lamp(&self, lamp: &str) -> Option<&DungeonDef> {
        self.dungeons().find(|d| d.lamps.iter().any(|l| l.key == lamp))
    }

    /// Put what's read in the order it's played: each creature's phases, its enrage among them,
    /// highest share of health first.
    fn settle(&mut self) {
        for m in &mut self.mobs {
            if let Some(e) = m.enrage.clone() {
                if !m.phases.iter().any(|p| p.at == e.at) {
                    m.phases.push(e);
                }
            }
            m.phases.sort_by(|a, b| b.at.total_cmp(&a.at));
        }
    }

    fn validate(&self) -> Result<()> {
        // Gear's perks and sets.
        for i in &self.items {
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
            if !c.after.is_empty() && self.mob(&c.after).is_none() {
                bail!("chest {}: after {}, no such mob", c.key, c.after);
            }
            for k in c.ambush.iter().chain(&c.trial) {
                if !self.packs.iter().any(|p| &p.key == k && p.wake == c.key) {
                    bail!("chest {}: ambush {k} needs a [[pack]] of that key that wakes on it", c.key);
                }
            }
        }
        for m in &self.mobs {
            for k in &m.teaches {
                card(k, &format!("mob {}", m.key))?;
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
        }
        for w in &self.weapons {
            if !w.moveset.is_empty() && !self.movesets.iter().any(|m| m.key == w.moveset) {
                bail!("weapon {}: no [[moveset]] {}", w.key, w.moveset);
            }
            if w.two_handed && self.shield(&w.guard).is_none() {
                bail!("weapon {}: two-handed, and its guard {:?} isn't a [[shield]]", w.key, w.guard);
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
                // Below: the book read, a lamp lit (by its key), or out the front door.
                "delve" => g.what == "book" || g.what == "end" || self.dungeon.as_ref().is_some_and(|d| d.lamps.iter().any(|l| l.key == g.what)),
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
        let mut keys = std::collections::BTreeSet::new();
        let effect_ok = |e: &AbilityEffect, what: &str| -> Result<()> {
            if !CC_KINDS.contains(&e.kind.as_str()) && !OTHER_EFFECTS.contains(&e.kind.as_str()) {
                bail!("{what}: no effect {:?} (one of {CC_KINDS:?} or {OTHER_EFFECTS:?})", e.kind);
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
            if a.school != "core" && !SCHOOLS.contains(&a.school.as_str()) {
                bail!("{what}: school {:?} (core or one of {SCHOOLS:?})", a.school);
            }
            if !a.is_ex() && !a.inner && !SCHOOLS.contains(&a.school.as_str()) {
                bail!("{what}: a card needs its column, `school` (one of {SCHOOLS:?})");
            }
            if a.kind == "art" && a.weapon <= 0.0 {
                bail!("{what}: a weapon art needs its share of the weapon's damage (`weapon`)");
            }
            if a.at_self && !matches!(a.kind.as_str(), "ground" | "trap") {
                bail!("{what}: only ground and traps are laid `at_self`");
            }
            if a.kind == "trap" && (a.radius[1] <= 0.0 || a.secs <= 0.0) {
                bail!("{what}: a trap needs its reach (`radius`) and its life (`secs`)");
            }
            if a.kind == "line" && (a.range[1] <= 0.0 || a.radius[1] <= 0.0) {
                bail!("{what}: a strip needs its length (`range`) and half-width (`radius`)");
            }
            if a.kind == "ally" && (a.damage[0] > 0.0 || a.damage[1] > 0.0) {
                bail!("{what}: an ally ability helps whoever it lands on: no `damage`");
            }
            if a.kind == "beam" && (a.range[1] <= 0.0 || a.radius[1] <= 0.0 || a.secs <= 0.0) {
                bail!("{what}: a beam needs its length (`range`), half-width (`radius`) and how long it's held (`secs`)");
            }
            if a.cast > 0.0 && !matches!(a.kind.as_str(), "cone" | "nova" | "line") {
                bail!("{what}: only a cone, a nova or a strip winds up (`cast`)");
            }
            if (a.pierce > 0 || a.full_pierce > 0 || a.chain > 0 || a.splash > 0.0) && a.kind != "projectile" && a.kind != "trap" {
                bail!("{what}: pierce, chain and splash are a projectile's");
            }
            if !a.boon.is_empty() && !BOONS.contains(&a.boon.as_str()) {
                bail!("{what}: boon {:?} (one of {BOONS:?})", a.boon);
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
            if a.kind == "place" && a.totem.is_none() {
                bail!("{what}: a placed ability needs a [ability.totem]");
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
            if a.empower.as_ref().is_some_and(|e| e.secs <= 0.0) {
                bail!("{what}: an empower needs its `secs`");
            }
            for e in a.effects.iter().chain(&a.self_effects).chain(&a.tick_effects) {
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
                if !r.recast.is_empty() {
                    bail!("{what}: its recast {} can't have a recast of its own", a.recast);
                }
            }
            // An empower names a base card with an EX version (the one it makes free).
            if let Some(e) = &a.empower {
                let Some(b) = self.ability(&e.ability) else { bail!("{what}: empowers {}, no such ability", e.ability) };
                if self.abilities[b].is_ex() || self.abilities[b].inner || self.ex_of(b).is_none() {
                    bail!("{what}: empowers {}, which must be a base card with an EX version", e.ability);
                }
            }
        }
        // The board: four cards a row at most, one a column.
        for row in SLOTS {
            let cards: Vec<&AbilityDef> = self.abilities.iter().filter(|a| a.on_board() && a.slot == row).collect();
            if cards.len() > SCHOOLS.len() {
                bail!("the board's {row} row: {} cards, at most {}", cards.len(), SCHOOLS.len());
            }
            for col in SCHOOLS {
                if cards.iter().filter(|a| a.school == col).count() > 1 {
                    bail!("the board's {row} row: two {col} cards");
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
}

#[cfg(test)]
mod tests {
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
