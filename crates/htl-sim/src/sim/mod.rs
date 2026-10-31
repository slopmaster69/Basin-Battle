//! The game: a hub at the centre of an arena, lanes leading to it, endless waves of enemies
//! walking the lanes, and four outer bases to raid for power. Advance it with [`Sim::step`] and
//! read what happened from the returned [`Event`]s; ask for the whole picture with
//! [`Sim::entities`] and [`Sim::player_view`].

use std::collections::{BTreeMap, VecDeque};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::content::{ClassDef, Content, EnemyDef};
use crate::geom::{dist2d, facing, Ground, Vec3};

mod bases;
mod cards;
mod enemies;
pub use enemies::{MODIFIERS, RIFLE_SHOT};
mod defenses;
pub use defenses::SoldierKind;
mod guide;
pub use guide::Objective;
mod gear;
mod hazards;
pub mod gear_api {
    pub use super::gear::{EquipError, ItemInst, LootView};
}
mod players;
mod shop;
mod fronts;
mod pause;
pub use fronts::{FrontView, Ping};
pub use shop::{ShopItem, SiteView, SHOP_ICONS};
mod spells;
mod synergy;
mod waves;
pub use spells::talent_points_at;

pub use bases::BaseState;
pub use players::{Luck, MeterView, PartyMeterRow, PlayerResult, PlayerView, StatsView};

pub type EntityId = u32;

/// The playing field, from the map data (or a test).
#[derive(Debug, Clone)]
pub struct Arena {
    pub map_id: u32,
    pub area_id: u32,
    pub hub: Vec3,
    pub hub_facing: f32,
    pub player_spawn: Vec3,
    pub player_facing: f32,
    pub lanes: Vec<Lane>,
    /// The outer bases players raid; may be empty.
    pub bases: Vec<BaseSite>,
    /// Chokepoints on the way in (the bridges to the Blacksmith), where defences can be built
    /// too; may be empty.
    pub posts: Vec<Post>,
    /// The world rect the client's map of the arena covers: left and right are world Y, top and
    /// bottom world X (`WorldMapArea.dbc`); `None` without one.
    pub map_rect: Option<[f32; 4]>,
}

impl Arena {
    /// Where a world spot falls on the client's map, 0 to 1 across and down.
    pub fn map_xy(&self, p: Vec3) -> Option<(f32, f32)> {
        let [left, right, top, bottom] = self.map_rect?;
        let x = (left - p[1]) / (left - right);
        let y = (top - p[0]) / (top - bottom);
        ((0.0..=1.0).contains(&x) && (0.0..=1.0).contains(&y)).then_some((x, y))
    }
}

/// A chokepoint lanes pass through on their way to the hub.
#[derive(Debug, Clone)]
pub struct Post {
    /// "the north bridge".
    pub name: String,
    /// Open ground on the hub's side of it.
    pub pos: Vec3,
    /// The way enemies come from.
    pub facing: f32,
}

#[derive(Debug, Clone)]
pub struct Lane {
    /// What players are told: "enemies from the north".
    pub name: String,
    /// From the spawn point to the edge of the hub, on the ground: the first route.
    pub points: Vec<Vec3>,
    /// Every way this lane's enemies walk, the first route included: a base splits its enemies
    /// between the roads leaving it.
    pub routes: Vec<Vec<Vec3>>,
}

impl Lane {
    /// A lane along one route.
    pub fn new(name: impl Into<String>, points: Vec<Vec3>) -> Self {
        Self { name: name.into(), routes: vec![points.clone()], points }
    }
    /// A lane split over several routes (the first is the main one).
    pub fn split(name: impl Into<String>, routes: Vec<Vec<Vec3>>) -> Self {
        let points = routes.first().cloned().unwrap_or_default();
        Self { name: name.into(), points, routes }
    }
}

/// An outer base: where its gate stands and which lane it feeds.
#[derive(Debug, Clone)]
pub struct BaseSite {
    /// "the Farm".
    pub name: String,
    pub pos: Vec3,
    pub lane: usize,
}

/// Which content entry an enemy is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnemyKind {
    pub boss: bool,
    pub index: usize,
}

/// The non-fighting units at a base.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NpcKind {
    /// The Blight Gate: the base's portal.
    Gate,
    /// The relic cache, once the guardians are dead.
    Cache,
    /// The banner players channel to claim the base.
    Banner { claimed: bool },
    /// A sprint pick-up on a lane.
    Sprint,
    /// One of the Ancient's Treant Protectors.
    Protector,
    /// One of the Ancient's Healing Springs.
    Spring,
    /// The standard at a post (a bridge): right-click it to build defences there.
    Post(u8),
    /// The shopkeeper by the Ancient.
    Vendor,
}

/// What marks a ground effect for everyone to see, besides its spell's own look: a danger circle
/// the size of an enemy ability's reach, or a hunter's trap.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroundMark {
    None,
    /// An enemy's ability will land (or is burning) here: get out.
    Danger,
    FreezingTrap,
    ExplosiveTrap,
    FrostTrap,
}

/// What the world itself hurt someone with, numbered as the client's environmental damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hazard {
    Fall = 2,
}

/// Where a spell lands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SpellTarget {
    Unit(EntityId),
    Point(Vec3),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    /// A new enemy, standing still at `pos`. A `MoveTo` usually follows in the same step.
    SpawnEnemy {
        id: EntityId,
        kind: EnemyKind,
        elite: bool,
        level: u32,
        pos: Vec3,
        facing: f32,
        health: u32,
        max_health: u32,
    },
    SpawnNpc {
        id: EntityId,
        npc: NpcKind,
        base: usize,
        pos: Vec3,
        facing: f32,
    },
    /// A risen skeleton fighting for `owner`.
    SpawnAlly {
        id: EntityId,
        owner: EntityId,
        level: u32,
        pos: Vec3,
        facing: f32,
        health: u32,
        /// A hunter's pet.
        pet: bool,
    },
    /// A warrior's rage changed (the client's power field: rage times ten).
    Power {
        player: EntityId,
        power: u32,
    },
    /// A player's talents changed (or their points did): read them with `Sim::talent_ranks`.
    Talents {
        player: EntityId,
    },
    /// A banner changed hands.
    NpcChanged {
        id: EntityId,
        npc: NpcKind,
    },
    Despawn {
        id: EntityId,
    },
    /// Walk `path` (its first point is where the unit is now) at `speed` yards a second.
    MoveTo {
        id: EntityId,
        path: Vec<Vec3>,
        speed: f32,
    },
    Stop {
        id: EntityId,
        pos: Vec3,
        facing: f32,
    },
    AttackStart {
        attacker: EntityId,
        victim: EntityId,
    },
    AttackStop {
        attacker: EntityId,
        victim: EntityId,
    },
    /// A melee swing landed.
    Hit {
        attacker: EntityId,
        victim: EntityId,
        damage: u32,
        crit: bool,
    },
    /// A spell goes off: play its visual from `caster` to the target (and on each of `hits`).
    SpellGo {
        caster: EntityId,
        spell: u32,
        target: SpellTarget,
        hits: Vec<EntityId>,
    },
    /// Spell damage, for the floating numbers and the combat log.
    SpellHit {
        caster: EntityId,
        victim: EntityId,
        spell: u32,
        damage: u32,
        crit: bool,
        school: u8,
        periodic: bool,
    },
    Heal {
        healer: EntityId,
        target: EntityId,
        spell: u32,
        amount: u32,
        crit: bool,
    },
    Health {
        id: EntityId,
        health: u32,
        max_health: u32,
    },
    Died {
        id: EntityId,
    },
    Revived {
        id: EntityId,
        health: u32,
        max_health: u32,
    },
    /// The player swung at a target out of reach.
    NotInRange {
        player: EntityId,
    },
    Announce {
        text: String,
        /// Shown big in the middle of the screen as well as in chat.
        big: bool,
    },
    /// The world paused (or resumed, with no one left) while these players choose a card.
    Paused {
        choosing: Vec<String>,
    },
    /// A player starts casting a spell with a cast time (`spell` names it and its animation).
    CastStart {
        caster: EntityId,
        spell: u32,
        ms: u32,
        target: Option<EntityId>,
    },
    /// A cast that didn't go off after all, and why (for the player's screen).
    CastFailed {
        player: EntityId,
        error: CastError,
    },
    /// A cast was broken (the caster moved, or there was nothing left to hit).
    CastStop {
        caster: EntityId,
        spell: u32,
    },
    /// A player pinged: holding a front, or calling for help.
    Ping {
        player: EntityId,
        text: String,
        help: bool,
    },
    /// A moment worth a big banner across the screen: a wave begins or ends, a boss, Escalation.
    Splash {
        style: Splash,
        title: String,
        subtitle: String,
    },
    /// An enemy starts casting an ability players should dodge; it lands in `ms`.
    /// A player stepped out of a strike that was aimed at them.
    Dodged {
        player: EntityId,
        name: String,
    },
    EnemyCast {
        caster: EntityId,
        name: String,
        spell: u32,
        ms: u32,
        /// The warning for the players standing in it.
        warn: String,
        endangered: Vec<EntityId>,
        /// A strike everyone should see coming (a share-of-health cleave): a cast bar like a boss's.
        big: bool,
    },
    /// A player's spell bar changed (a spell was cast, learned or lost, or casting switched
    /// between by hand and automatic): read it again with `Sim::spell_bar`.
    Bar {
        player: EntityId,
    },
    /// A player took a card (or ranked one up).
    CardTaken {
        player: EntityId,
        card: String,
        rank: u8,
    },
    /// A message for one player only.
    Tell {
        player: EntityId,
        text: String,
    },
    /// A cooldown the client started is lifted (a Hearthstone cast that was broken).
    CooldownCleared {
        player: EntityId,
        spell: u32,
    },
    /// The wave counter changed or the phase did; for the HUD.
    Status(Status),
    /// A line for the server's log only: the balance report at the end of each wave.
    Log(String),
    /// Harm from the world itself, not a foe (a long fall).
    Environmental {
        id: EntityId,
        kind: Hazard,
        damage: u32,
    },
    /// A unit that isn't a player changed level (a guardian keeping pace with the waves).
    UnitLevel {
        id: EntityId,
        level: u32,
    },
    /// A player's level changed (or their experience bar needs redrawing from scratch).
    Level {
        player: EntityId,
        level: u32,
        gained_health: i32,
    },
    /// Experience gained; `victim` is what died for it, if anything.
    Xp {
        player: EntityId,
        amount: u32,
        victim: Option<EntityId>,
    },
    Gold {
        player: EntityId,
        gold: u32,
    },
    /// What the player's stats are has changed: health, damage, speed, armour.
    Stats {
        player: EntityId,
    },
    /// One of the player's buff slots shows `spell` with `stacks` (0 clears the slot).
    Aura {
        unit: EntityId,
        slot: u8,
        spell: u32,
        stacks: u8,
        /// Seconds left, for timed auras; 0 for permanent.
        secs: f32,
    },
    /// Open (or, with `None`, close) a menu on a player's screen.
    Menu {
        player: EntityId,
        menu: Option<Menu>,
    },
    /// The player's bags or equipment changed.
    Inventory {
        player: EntityId,
    },
    /// An item arrived in the player's bags.
    ItemReceived {
        player: EntityId,
        entry: u32,
        slot: u8,
    },
    /// What others see the player wearing changed.
    Visible {
        player: EntityId,
    },
    /// Whether `player` may loot `corpse`.
    Lootable {
        corpse: EntityId,
        player: EntityId,
        lootable: bool,
    },
    Channel {
        player: EntityId,
        spell: u32,
        ms: u32,
    },
    ChannelEnd {
        player: EntityId,
    },
    /// A hit pushed the channel back: `ms` are left.
    ChannelLeft {
        player: EntityId,
        ms: u32,
    },
    Teleport {
        player: EntityId,
        pos: Vec3,
        facing: f32,
    },
    /// A hero rushes from `from` to `to` in `secs` (a Charge): the client runs them there.
    Rush {
        player: EntityId,
        from: Vec3,
        to: Vec3,
        secs: f32,
    },
    /// A lasting ground effect (Blizzard, Consecration): show its area.
    ZoneStart {
        id: EntityId,
        caster: EntityId,
        spell: u32,
        pos: Vec3,
        radius: f32,
        /// What marks it on the ground besides its spell's own look.
        mark: GroundMark,
    },
    ZoneEnd {
        id: EntityId,
    },
    /// The run is over: the team's depth and who played.
    RunOver(RunResult),
    /// The victory wave is cleared: the run is won (and goes on, endless).
    Victory(RunResult),
}

/// The kinds of big banner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Splash {
    NewRun,
    Wave,
    Boss,
    Cleared,
    Bases,
    Claimed,
    Escalation,
    Defeat,
    Victory,
}

impl Splash {
    pub fn key(self) -> &'static str {
        match self {
            Splash::NewRun => "run",
            Splash::Wave => "wave",
            Splash::Boss => "boss",
            Splash::Cleared => "cleared",
            Splash::Bases => "bases",
            Splash::Claimed => "claimed",
            Splash::Escalation => "escalation",
            Splash::Defeat => "defeat",
            Splash::Victory => "victory",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunResult {
    /// The deepest wave cleared.
    pub depth: u32,
    /// The run reached its victory wave.
    pub won: bool,
    pub cycles: u32,
    pub secs: u32,
    pub players: Vec<PlayerResult>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Status {
    pub wave: u32,
    pub cycle: u32,
    pub phase: PhaseName,
    /// Seconds until the next wave, during a countdown or intermission.
    pub seconds_left: u32,
    pub hub_health: u32,
    pub hub_max_health: u32,
    pub enemies_left: u32,
    pub bases_claimed: u32,
    pub bases: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhaseName {
    Waiting,
    Countdown,
    Wave,
    Intermission,
    Defeat,
}

/// A menu of choices for one player. The server shows it as an NPC dialog.
#[derive(Debug, Clone, PartialEq)]
pub struct Menu {
    pub kind: MenuKind,
    /// What the menu belongs to: the player themselves, or a unit they clicked.
    pub source: EntityId,
    /// A line for chat, so text-only clients can follow.
    pub title: String,
    pub options: Vec<MenuOption>,
    /// For a card pick, everything the card picker draws.
    pub offer: Option<OfferView>,
}

/// A card pick as the card picker draws it. Choosing card `i` is choosing option `i`.
#[derive(Debug, Clone, PartialEq)]
pub struct OfferView {
    pub kind: OfferKind,
    pub cards: Vec<CardView>,
    /// The reroll's option index and price.
    pub reroll: Option<(usize, RerollCost)>,
    /// The skip's option index, the gold it pays and the skips left.
    pub skip: Option<(usize, u32, u32)>,
    pub banishes: u32,
    pub can_lock: bool,
    /// Picks queued behind this one.
    pub more: usize,
    pub level: u32,
    pub wave: u32,
    pub gold: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfferKind {
    Start,
    HeadStart,
    Level,
    Wave,
    Boss,
    Relic,
    Legendary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RerollCost {
    /// Free rerolls left.
    Free(u32),
    Gold(u32),
}

/// One card in an offer.
#[derive(Debug, Clone, PartialEq)]
pub struct CardView {
    pub key: String,
    pub name: String,
    /// The spell whose icon the card shows.
    pub icon_spell: u32,
    pub rarity: crate::content::Rarity,
    /// The rank it will be once taken: 1 for a new card.
    pub rank: u8,
    pub max_rank: u8,
    pub text: crate::describe::CardText,
    pub tags: Vec<String>,
    /// "Takes a spell slot · 3/6" or "Ranks up a spell you hold".
    pub slot: String,
}

/// One button of a player's spell bar.
#[derive(Debug, Clone, PartialEq)]
pub struct BarSlot {
    pub key: String,
    pub name: String,
    pub icon_spell: u32,
    pub rarity: crate::content::Rarity,
    pub rank: u8,
    /// Its full cooldown and what's left of it, in seconds at the player's current haste.
    pub cooldown: f32,
    pub remaining: f32,
    /// Rage it costs (warriors).
    pub cost: u32,
    /// Learned from a talent (keys 7 and up).
    pub talent: bool,
    /// A toggle that is on.
    pub on: bool,
}

impl BarSlot {
    /// A key with nothing on it.
    pub fn empty() -> Self {
        Self {
            key: String::new(),
            name: String::new(),
            icon_spell: 0,
            rarity: crate::content::Rarity::Common,
            rank: 0,
            cooldown: 0.0,
            remaining: 0.0,
            cost: 0,
            talent: false,
            on: false,
        }
    }
}

/// The spell bar: what's on each of the six keys (an empty key between spells shows blank).
#[derive(Debug, Clone, PartialEq)]
pub struct SpellBar {
    /// The player casts them (true) or they fire on their own.
    pub manual: bool,
    /// What's left of the global cooldown.
    pub gcd: f32,
    pub slots: Vec<BarSlot>,
    /// A warrior's rage.
    pub rage: Option<u32>,
}

/// Why a spell didn't go off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CastError {
    NoSpell,
    Dead,
    Busy,
    NotReady,
    Gcd,
    NoTarget,
    NotNow,
    NoRage,
    Shadowform,
    TooClose,
    NoPet,
    Moved,
}

impl CastError {
    pub fn text(self) -> &'static str {
        match self {
            CastError::NoSpell => "No spell on that key yet: pick one from a card.",
            CastError::Dead => "You can't cast while dead.",
            CastError::Busy => "You're busy casting.",
            CastError::Moved => "Interrupted: stand still to cast that.",
            CastError::NotReady => "That spell isn't ready yet.",
            CastError::Gcd => "Not ready yet.",
            CastError::NoTarget => "No enemy in range.",
            CastError::NotNow => "You can't do that now.",
            CastError::NoRage => "Not enough rage.",
            CastError::Shadowform => "You can't cast holy spells or heals in Shadowform.",
            CastError::TooClose => "Too close.",
            CastError::NoPet => "Your pet isn't here.",
        }
    }
}

/// A card the player holds.
#[derive(Debug, Clone, PartialEq)]
pub struct HeldCard {
    pub key: String,
    pub name: String,
    pub icon_spell: u32,
    /// The spell its buff shows as.
    pub aura_spell: u32,
    pub rarity: crate::content::Rarity,
    pub rank: u8,
    pub max_rank: u8,
    /// What it does now (colour on).
    pub text: crate::describe::CardText,
    /// What the next rank adds; empty at the top rank.
    pub next: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MenuKind {
    Card,
    Relic,
    Legendary,
    Banish,
    Lock,
    SetPiece,
    Forge,
    Banner,
    Cache,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MenuOption {
    /// The dialog's option icon.
    pub icon: u8,
    pub text: String,
}

/// One entity as it stands now, for drawing a world from scratch (a player joining late).
#[derive(Debug, Clone)]
pub struct EntityView {
    pub id: EntityId,
    pub what: What,
    pub pos: Vec3,
    pub facing: f32,
    pub health: u32,
    pub max_health: u32,
    pub level: u32,
    pub alive: bool,
    /// The rest of the path it is walking, from where it is now.
    pub moving: Option<(Vec<Vec3>, f32)>,
    pub attacking: Option<EntityId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum What {
    Hub,
    Enemy { kind: EnemyKind, elite: bool },
    Player { name: String, class: String },
    Npc { npc: NpcKind, base: usize },
    Ally { owner: EntityId, pet: bool },
    Zone { caster: EntityId, spell: u32, radius: f32, mark: GroundMark },
}

#[derive(Debug)]
enum Phase {
    Waiting,
    Countdown { left: f32 },
    Wave { pending: Vec<waves::PendingSpawn> },
    Intermission { left: f32 },
    Over { left: f32 },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Goal {
    /// Walking the lane towards the hub.
    Lane,
    /// At (or heading straight for) the hub.
    Hub,
    Player(EntityId),
    /// Tearing down one of the Ancient's defences (or a summoned ally) it passed.
    Attack(EntityId),
    /// Guarding a base.
    Guard,
}

#[derive(Debug)]
struct Mover {
    path: Vec<Vec3>,
    seg: usize,
    along: f32,
    speed: f32,
}

impl Mover {
    fn new(path: Vec<Vec3>, speed: f32) -> Self {
        Self { path, seg: 0, along: 0.0, speed }
    }
    fn done(&self) -> bool {
        self.seg + 1 >= self.path.len()
    }
    /// Advance by `dt` seconds; returns the new position and facing.
    fn advance(&mut self, dt: f32) -> (Vec3, f32) {
        let mut budget = self.speed * dt;
        while budget > 0.0 && !self.done() {
            let (a, b) = (self.path[self.seg], self.path[self.seg + 1]);
            let len = dist2d(a, b);
            let left = len - self.along;
            if budget < left {
                self.along += budget;
                budget = 0.0;
            } else {
                budget -= left;
                self.seg += 1;
                self.along = 0.0;
            }
        }
        self.here()
    }
    fn here(&self) -> (Vec3, f32) {
        if self.done() {
            let n = self.path.len();
            let last = self.path[n - 1];
            let f = if n >= 2 { facing(self.path[n - 2], last) } else { 0.0 };
            return (last, f);
        }
        let (a, b) = (self.path[self.seg], self.path[self.seg + 1]);
        let len = dist2d(a, b).max(1e-4);
        let t = self.along / len;
        (
            [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t, a[2] + (b[2] - a[2]) * t],
            facing(a, b),
        )
    }
    fn rest(&self) -> Vec<Vec3> {
        let (here, _) = self.here();
        let mut out = vec![here];
        out.extend_from_slice(&self.path[(self.seg + 1).min(self.path.len())..]);
        out
    }
    fn end(&self) -> Vec3 {
        *self.path.last().expect("a path has points")
    }
}

#[derive(Debug)]
enum Body {
    Hub,
    Enemy(enemies::Enemy),
    Player(Box<players::Player>),
    Npc { npc: NpcKind, base: usize },
    Ally(enemies::Ally),
    Zone(cards::Zone),
    Trap(spells::Trap),
}

#[derive(Debug)]
struct Entity {
    pos: Vec3,
    facing: f32,
    health: u32,
    max_health: u32,
    level: u32,
    body: Body,
}

impl Entity {
    fn alive(&self) -> bool {
        self.health > 0
    }
    fn is_enemy(&self) -> bool {
        matches!(self.body, Body::Enemy(_))
    }
    fn is_player(&self) -> bool {
        matches!(self.body, Body::Player(_))
    }
    fn player(&self) -> Option<&players::Player> {
        match &self.body {
            Body::Player(p) => Some(p),
            _ => None,
        }
    }
    fn player_mut(&mut self) -> Option<&mut players::Player> {
        match &mut self.body {
            Body::Player(p) => Some(p),
            _ => None,
        }
    }
    fn enemy(&self) -> Option<&enemies::Enemy> {
        match &self.body {
            Body::Enemy(e) => Some(e),
            _ => None,
        }
    }
    fn enemy_mut(&mut self) -> Option<&mut enemies::Enemy> {
        match &mut self.body {
            Body::Enemy(e) => Some(e),
            _ => None,
        }
    }
}

/// The run as a whole.
#[derive(Debug, Default)]
struct Run {
    /// The wave in progress, or the last one cleared.
    wave: u32,
    /// The wave in progress comes with a twist (the Swarm, the Vanguard).
    event: Option<waves::WaveEvent>,
    cycle: u32,
    /// The deepest wave cleared.
    depth: u32,
    /// The victory wave has been cleared.
    won: bool,
    started: f32,
    /// Indices into the content's affixes, in the order they joined.
    affixes: Vec<usize>,
    kills: u32,
    /// Seconds until the Escalation, once the last base is claimed.
    escalation: Option<f32>,
    /// Experience per point of enemy xp weight, this wave.
    xp_per_weight: f32,
    gold_per_weight: f32,
    /// Hub upgrades bought this run.
    thornbark: u32,
    repairs: u32,
    /// The wave the Ancient's wrath was last called in.
    wrath_wave: u32,
    /// Waves since each player last got a drop, by player.
    drought: BTreeMap<EntityId, u32>,
    /// Bases claimed at least once this run (the first claim is worth a bonus level).
    claimed_once: Vec<usize>,
    /// The wave the first sapper was called out in.
    sapper_warned: u32,
    /// The defences' damage this wave, on the party meter: the base's (treants, the Ancient)
    /// and the soldiers' (the Basin Defenders).
    defence_meter: [u64; 2],
    /// The wave's enemies as they came, for the balance log: how many, their health and level,
    /// and the Ancient's lowest share of health.
    spawned: u32,
    /// Healing for the Ancient banked from heroes' Heartwood Lockets, until it's a whole point.
    hub_mend: f32,
    spawned_health: u64,
    spawned_hit: f32,
    hub_low: f32,
}

pub struct Sim {
    content: Content,
    arena: Arena,
    ground: Box<dyn Ground>,
    rng: SmallRng,
    entities: BTreeMap<EntityId, Entity>,
    next_id: EntityId,
    next_item: u32,
    hub: EntityId,
    phase: Phase,
    run: Run,
    clock: f32,
    events: Vec<Event>,
    hub_warned: u8,
    /// When the next "the Ancient is under attack" may be called (a lone hero away at a bridge
    /// hears it at once).
    hub_alert_at: f32,
    last_status: Option<Status>,
    bases: Vec<bases::Base>,
    hits: Vec<cards::PendingHit>,
    /// Visuals shown later: a channel's missiles leave one by one (when, caster, spell, event).
    later: Vec<(f32, EntityId, u32, Event)>,
    /// Corpses holding loot, by corpse.
    loot: BTreeMap<EntityId, Vec<gear::CorpseLoot>>,
    /// Players' chosen arrival order, for stable iteration.
    order: VecDeque<EntityId>,
    sprints: Vec<hazards::SprintSpot>,
    landings: Vec<hazards::Landing>,
    /// Warriors on their way in a Charge: (who, the target, when they arrive, the spell, its numbers).
    charges: Vec<(EntityId, EntityId, f32, crate::kit::SpellDef, crate::kit::Nums)>,
    /// When each lane's stream sends its next enemy.
    stream_next: Vec<f32>,
    /// Which of each lane's routes its next enemy takes.
    route_next: Vec<usize>,
    /// The standards at the posts.
    post_ids: Vec<EntityId>,
    /// The shopkeeper by the Ancient.
    vendor: Option<EntityId>,
    /// The next wave, decided ahead so it can be shown.
    next_plan: Option<waves::WavePlan>,
    /// Who the world is paused for (choosing a card), and for how long now.
    paused_for: Vec<EntityId>,
    /// Heroes who left this run, kept whole: the same name coming back as the same class picks
    /// up where they left off (spells, talents, cards, level, gear, gold).
    departed: Vec<Entity>,
    pause_secs: f32,
    defenses: defenses::Defenses,
}

pub(crate) const MELEE_RANGE: f32 = 5.5;
pub(crate) const UNIT_RADIUS: f32 = 0.6;
const GOLDEN_ANGLE: f32 = 2.399_963;
pub(crate) const PATH_STEP: f32 = 4.0;
/// The hearthstone's channel and the spell it shows.
pub(crate) const HEARTH_SPELL: u32 = 8690;
pub const HEARTH_SPELL_ID: u32 = HEARTH_SPELL;
/// How long card windows wait after a revive or a teleport.
pub(crate) const MENU_HOLD_SECS: f32 = 1.5;

impl Sim {
    pub fn new(content: Content, arena: Arena, ground: Box<dyn Ground>, seed: u64) -> Self {
        let mut entities = BTreeMap::new();
        let hub = 1;
        entities.insert(
            hub,
            Entity {
                pos: arena.hub,
                facing: arena.hub_facing,
                health: content.hub.health,
                max_health: content.hub.health,
                level: content.hub.level,
                body: Body::Hub,
            },
        );
        let mut sim = Self {
            content,
            arena,
            ground,
            rng: SmallRng::seed_from_u64(seed),
            entities,
            next_id: 2,
            next_item: 1,
            hub,
            phase: Phase::Waiting,
            run: Run::default(),
            clock: 0.0,
            events: Vec::new(),
            hub_warned: 0,
            hub_alert_at: 0.0,
            last_status: None,
            bases: Vec::new(),
            hits: Vec::new(),
            later: Vec::new(),
            loot: BTreeMap::new(),
            order: VecDeque::new(),
            sprints: Vec::new(),
            landings: Vec::new(),
            charges: Vec::new(),
            vendor: None,
            stream_next: Vec::new(),
            route_next: Vec::new(),
            defenses: defenses::Defenses::default(),
            post_ids: Vec::new(),
            next_plan: None,
            paused_for: Vec::new(),
            departed: Vec::new(),
            pause_secs: 0.0,
        };
        sim.spawn_posts();
        // There from the start, before anyone is told of anything.
        sim.spawn_vendor(false);
        sim.sprints = Sim::sprint_spots(&sim.arena, &sim.content.sprint.along);
        sim.bases = (0..sim.arena.bases.len()).map(bases::Base::new).collect();
        sim
    }

    pub fn content(&self) -> &Content {
        &self.content
    }
    pub fn arena(&self) -> &Arena {
        &self.arena
    }
    pub fn hub_id(&self) -> EntityId {
        self.hub
    }
    pub fn enemy_def(&self, kind: EnemyKind) -> &EnemyDef {
        if kind.boss {
            &self.content.bosses[kind.index]
        } else {
            &self.content.enemies[kind.index]
        }
    }
    pub fn class_def(&self, id: EntityId) -> Option<&ClassDef> {
        self.entities.get(&id)?.player().map(|p| &self.content.classes[p.class])
    }
    pub fn ground_height(&self, x: f32, y: f32) -> f32 {
        self.ground.height(x, y)
    }
    pub fn wave(&self) -> u32 {
        self.run.wave
    }
    pub fn cycle(&self) -> u32 {
        self.run.cycle
    }
    pub fn depth(&self) -> u32 {
        self.run.depth
    }

    /// Everything in the world as it stands.
    pub fn entities(&self) -> Vec<EntityView> {
        self.entities
            .iter()
            .map(|(&id, e)| EntityView {
                id,
                what: match &e.body {
                    Body::Hub => What::Hub,
                    Body::Enemy(en) => What::Enemy { kind: en.kind, elite: en.elite },
                    Body::Player(p) => What::Player {
                        name: p.name.clone(),
                        class: self.content.classes[p.class].key.clone(),
                    },
                    Body::Npc { npc, base } => What::Npc { npc: *npc, base: *base },
                    Body::Ally(a) => What::Ally { owner: a.owner, pet: a.pet },
                    Body::Zone(z) => What::Zone { caster: z.caster, spell: z.spell, radius: z.radius, mark: z.mark() },
                    Body::Trap(t) => What::Zone { caster: t.owner, spell: spells::TRAP_LOOK, radius: 2.0, mark: t.mark },
                },
                pos: e.pos,
                facing: e.facing,
                health: e.health,
                max_health: e.max_health,
                level: e.level,
                alive: e.alive(),
                moving: match &e.body {
                    Body::Enemy(en) => en.mover.as_ref().filter(|m| !m.done()).map(|m| (m.rest(), m.speed)),
                    Body::Ally(a) => a.mover.as_ref().filter(|m| !m.done()).map(|m| (m.rest(), m.speed)),
                    _ => None,
                },
                attacking: match &e.body {
                    Body::Enemy(en) => en.engaged,
                    Body::Player(p) => p.attacking,
                    Body::Ally(a) => a.engaged,
                    _ => None,
                },
            })
            .collect()
    }

    pub fn status(&self) -> Status {
        let hub = &self.entities[&self.hub];
        let (phase, left) = match &self.phase {
            Phase::Waiting => (PhaseName::Waiting, 0.0),
            Phase::Countdown { left } => (PhaseName::Countdown, *left),
            Phase::Wave { .. } => (PhaseName::Wave, 0.0),
            Phase::Intermission { left } => (PhaseName::Intermission, *left),
            Phase::Over { left } => (PhaseName::Defeat, *left),
        };
        let enemies_left = self
            .entities
            .values()
            .filter(|e| e.enemy().is_some_and(|en| en.guard.is_none()) && e.alive())
            .count() as u32
            + match &self.phase {
                Phase::Wave { pending } => pending.len() as u32,
                _ => 0,
            };
        Status {
            wave: self.run.wave,
            cycle: self.run.cycle,
            phase,
            seconds_left: left.ceil() as u32,
            hub_health: hub.health,
            hub_max_health: hub.max_health,
            enemies_left,
            bases_claimed: self.bases.iter().filter(|b| b.state == BaseState::Claimed).count() as u32,
            bases: self.bases.len() as u32,
        }
    }

    fn player_ids(&self) -> Vec<EntityId> {
        self.order.iter().copied().filter(|id| self.entities.contains_key(id)).collect()
    }

    fn player_count(&self) -> usize {
        self.order.len()
    }

    /// Whether the fighting runs: enemies think and abilities fire.
    fn combat_on(&self) -> bool {
        !matches!(self.phase, Phase::Waiting | Phase::Over { .. })
    }

    fn alloc(&mut self) -> EntityId {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn alloc_item(&mut self) -> u32 {
        let id = self.next_item;
        self.next_item += 1;
        id
    }

    // --- the loop -----------------------------------------------------------------------------

    pub fn step(&mut self, dt: f32) -> Vec<Event> {
        // Someone is choosing a card: the world holds still until every pick is made.
        if self.step_pause(dt) {
            return std::mem::take(&mut self.events);
        }
        self.clock += dt;
        self.step_phase(dt);
        self.step_enemies(dt);
        self.step_allies(dt);
        self.step_players(dt);
        self.step_casts();
        self.step_spells(dt);
        self.step_buffs(dt);
        self.step_rage(dt);
        self.step_pets();
        self.step_traps();
        self.step_menus();
        self.step_hits();
        if !self.later.is_empty() {
            let now = self.clock;
            let (due, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.later).into_iter().partition(|l| l.0 <= now);
            self.later = keep;
            self.events.extend(due.into_iter().map(|l| l.3));
        }
        self.step_zones(dt);
        self.step_landings();
        self.step_charges();
        self.step_sprints();
        self.step_respawns();
        self.step_defenses(dt);
        self.step_blessings();
        self.step_bases(dt);
        self.step_outcome();
        self.step_debuffs();
        let status = self.status();
        let changed = self.last_status.map_or(true, |s| s != status);
        if changed {
            self.events.push(Event::Status(status));
            self.last_status = Some(status);
        }
        std::mem::take(&mut self.events)
    }

    // --- shared -------------------------------------------------------------------------------

    fn announce(&mut self, text: String, big: bool) {
        self.events.push(Event::Announce { text, big });
    }

    fn splash(&mut self, style: Splash, title: impl Into<String>, subtitle: impl Into<String>) {
        self.events.push(Event::Splash { style, title: title.into(), subtitle: subtitle.into() });
    }

    pub(super) fn tell(&mut self, player: EntityId, text: String) {
        self.events.push(Event::Tell { player, text });
    }

    /// A random point within `r` yards of `p`, on the ground.
    fn scatter(&mut self, p: Vec3, r: f32) -> Vec3 {
        let a = self.rng.gen::<f32>() * std::f32::consts::TAU;
        let d = r * self.rng.gen::<f32>().sqrt();
        let (x, y) = (p[0] + d * a.cos(), p[1] + d * a.sin());
        [x, y, self.ground.height(x, y)]
    }

    /// A spot `r` yards from `center`, at angle `a`.
    fn ring(&self, center: Vec3, r: f32, a: f32) -> Vec3 {
        let (x, y) = (center[0] + r * a.cos(), center[1] + r * a.sin());
        [x, y, self.ground.height(x, y)]
    }

    fn set_health(&mut self, id: EntityId, health: u32) {
        if let Some(e) = self.entities.get_mut(&id) {
            let h = health.min(e.max_health);
            if h != e.health {
                e.health = h;
                let max_health = e.max_health;
                self.events.push(Event::Health { id, health: h, max_health });
            }
        }
    }

    /// Living enemies within `r` of `p`, nearest first.
    fn enemies_near(&self, p: Vec3, r: f32) -> Vec<(EntityId, f32)> {
        let mut v: Vec<(EntityId, f32)> = self
            .entities
            .iter()
            .filter(|(_, e)| e.is_enemy() && e.alive())
            .map(|(&id, e)| (id, dist2d(e.pos, p)))
            .filter(|(_, d)| *d <= r)
            .collect();
        v.sort_by(|a, b| a.1.total_cmp(&b.1));
        v
    }

    /// Restart the whole run: no enemies, a healthy hub, wave 1 next, everyone back at level 1.
    pub fn restart(&mut self) {
        self.reset();
        self.phase = if self.player_count() > 0 {
            Phase::Countdown { left: self.content.game.countdown_secs }
        } else {
            Phase::Waiting
        };
        self.announce("A new run begins.".into(), true);
        self.splash(Splash::NewRun, "A New Run", "Choose your starting ability");
        self.start_of_run_offers();
    }

    /// Skip the countdown or intermission.
    pub fn start_now(&mut self) {
        if matches!(self.phase, Phase::Countdown { .. } | Phase::Intermission { .. }) {
            self.begin_wave();
        }
    }
}

fn join_names(names: &[&str]) -> String {
    match names {
        [] => String::new(),
        [one] => one.to_string(),
        [rest @ .., last] => format!("{} and {last}", rest.join(", ")),
    }
}

/// Golden-angle spread, so a crowd fans out instead of stacking.
fn slot_angle(id: EntityId) -> f32 {
    (id as f32 * GOLDEN_ANGLE) % std::f32::consts::TAU
}
