//! The survival world: heroes, the creatures of Elwynn, the camp and what's built in it.

use std::collections::{BTreeMap, BTreeSet};

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

pub use htl_sim::geom::{dist2d, facing, Vec3};

use crate::content::{Content, ItemKind};
use crate::land::Land;

pub mod abilities;
pub mod arena;
pub(crate) mod beta;
pub mod cc;
mod combat;
pub mod dungeon;
mod journal;
pub mod meter;
pub mod mobs;
pub(crate) mod moves;
pub mod recap;
pub mod recasts;
pub(crate) mod explore;
pub(crate) mod heroes;
mod raid;
pub mod siege;
pub mod wager;
mod tactics;
mod world;

#[cfg(test)]
pub(crate) use combat::Follow;
pub use combat::{BLOCK_ANIM, CRIT_ANIM, EAT_DONE_SPELL, EAT_SPELL, KIT_BASH, KIT_CLEAVE, KIT_DUST, KIT_HEAVY, KIT_PARRY, KIT_RIPOSTE, KIT_SLAM, KNOCKDOWN_ANIM, PARRY_ANIM, SPRINT_SPELL, STUN_ANIM, WOUND_ANIM,
};
pub use abilities::{look_dir, Cooldown, HitKind, Stance};
pub use cc::Cc;
pub use arena::Arena;
pub use dungeon::{door_open, Below, BelowHero};
pub use raid::RaidPhase;
pub use world::{Placement, BUILD_DISTANCE, INTERACT_RANGE};

pub type Id = u32;

/// How a blow came out, as the client shows it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Hit,
    /// Swung at nothing (or nothing in reach).
    Miss,
    Blocked,
    Dodged,
    /// The guard broke: part of the blow landed and the defender staggers.
    GuardBroken,
    /// A guard raised just in time: nothing lands and the attacker reels.
    Parried,
}

/// What kind of blow a [`Event::Swing`] is, for how it shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blow {
    /// A swing (or a creature's blow).
    Light,
    /// A hero's heavy attack: its own look, a bigger number.
    Heavy,
    /// An arrow, a bolt or a spell from this spell's look: no swing, and the archer keeps the bow
    /// out.
    Arrow(u32),
    /// A burn's tick, a drain's: a number, no swing, no flinch.
    Dot,
}

/// Which blow a swing is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrikeKind {
    /// A blow of the light chain.
    Light,
    Heavy,
    /// An attack while sprinting.
    Running,
    /// An attack out of a dodge, as the roll ends.
    Roll,
    /// An attack out of a Corey roll (a jump taken in a dodge's dash), as the hero lands.
    Leap,
    /// A guard breaker: the shield bash, the kick.
    Bash,
    Kick,
}

/// A swing as committed at its start: its kind and place in the chain, its timing, its blow, its
/// look (the clip and take, for the clients that play clips) and how far the hero lunges into it.
#[derive(Debug, Clone, PartialEq)]
pub struct Strike {
    pub kind: StrikeKind,
    pub step: u8,
    pub windup: f32,
    pub active: f32,
    pub recovery: f32,
    /// The damage share of the weapon's own (a guard breaker: its own roll).
    pub damage: f32,
    pub range: f32,
    pub arc: f32,
    pub stagger: f32,
    pub clip: u16,
    pub take: u32,
    pub lunge: f32,
    /// The stamina it costs.
    pub stamina: f32,
    /// A weapon art's blow: the ability it is.
    pub art: Option<usize>,
    /// Another character model whose take on the clip the clients play (`MoveDef::from`).
    pub from: Option<String>,
}

impl Strike {
    pub fn total(&self) -> f32 {
        self.windup + self.active + self.recovery
    }
    pub fn heavy(&self) -> bool {
        self.kind == StrikeKind::Heavy
    }
    /// A guard breaker.
    pub fn shove(&self) -> bool {
        matches!(self.kind, StrikeKind::Bash | StrikeKind::Kick)
    }
}

/// Where a missile or a visual goes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    Unit(Id),
    Point(Vec3),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    MobSpawned { id: Id },
    MobGone { id: Id },
    /// Walk (or run) along `path`; `face`: turned to that unit the while (circling a foe), at a
    /// walk.
    MobMove { id: Id, path: Vec<Vec3>, speed: f32, face: Option<Id> },
    /// A shield-bearer raises its shield (or lets it down).
    MobGuard { id: Id, up: bool },
    MobStop { id: Id, pos: Vec3, facing: f32 },
    /// A swing: `anim` plays the attacker's swing (with no damage shown); otherwise it's the
    /// result landing (damage, block, dodge) without a second swing.
    Swing { attacker: Id, victim: Option<Id>, damage: u32, outcome: Outcome, anim: bool, style: Blow },
    /// A missile or a spell's look from `caster` to `target`.
    Missile { caster: Id, spell: u32, target: Target },
    /// A missile's look from a spot, not from a unit (a boomerang turning back to its thrower):
    /// `anchor` names an unseen object standing at `from` to fly it, until an
    /// [`Event::ObjGone`] of that id. `caster` is whose it is.
    MissileFrom { anchor: Id, caster: Id, spell: u32, from: Vec3, target: Target },
    /// A cast begins (its look and the caster's cast pose) or is cut short.
    Cast { caster: Id, spell: u32, ms: u32 },
    CastEnd { caster: Id },
    /// A cast went (its missile or its effect follows): its bar and its look (the precast, the
    /// glow in the hands) end with it, no failure shown. `spell`: only if it's that one (0: any).
    CastDone { caster: Id, spell: u32 },
    /// Ground that lingers (holy ground, fire) wears this spell's look (a DynamicObject) until
    /// the mark `id` is gone.
    AreaLook { id: Id, caster: Id, spell: u32, pos: Vec3, radius: f32 },
    Emote { id: Id, emote: u32 },
    Health { id: Id, health: u32, max: u32 },
    Stamina { player: Id },
    /// The hero's run speed and run-back speed.
    Speed { player: Id, speed: f32, back: f32 },
    /// The hero's kit or gear changed: what they hold, and how.
    Gear { player: Id },
    /// A unit takes a blow, for the clients that play clips: the reaction (`anim`, held `hold`
    /// seconds, looping, for a stagger) and how far it's thrown back from where the blow came.
    /// `fit`: the clip's whole length fitted to that many seconds (a knockdown), 0 its own.
    React { id: Id, anim: u16, from: Vec3, lean: f32, hold: f32, fit: f32 },
    /// A creature winds up a blow: the clip and take it swings with, timed to land `windup`
    /// seconds from now, and the recovery after (for the clients that play clips themselves);
    /// `blockable`: a guard stops it (its swing's trail says which); `from`: another character
    /// model's take on the clip.
    MobWindup { id: Id, clip: u16, take: u32, windup: f32, recovery: f32, blockable: bool, from: Option<String> },
    /// A spell visual kit plays on a unit (an impact's flash and sound); `by`: the hero whose blow
    /// it is (whose own client may already have played it).
    Visual { id: Id, kit: u32, by: Option<Id> },
    /// The hero's lock-on: the unit, or none.
    Lock { player: Id, target: Option<Id> },
    /// A hero's lasting effects changed (one began or ran out).
    Effects { player: Id },
    /// The journal moved on: a goal done.
    Goal,
    /// The camp's places found or chests opened changed: the world map's marks again.
    Sites,
    /// The dodge's dash (`roll`: a dodge, which tumbles), or a blow throwing a hero back.
    /// `speed` 0: no throw (the dash runs on the ground, by [`Event::Speed`]); just the tumble.
    /// `style`: which way a dodge goes relative to the facing (`roll` forward, `left`, `right`,
    /// `back`).
    Hop { player: Id, dir: f32, speed: f32, lift: f32, roll: bool, style: &'static str },
    /// The guard went up or down (the hero's stance).
    Guard { player: Id, up: bool },
    /// A word flashed on one hero's screen (a parry).
    Flash { player: Id, text: String },
    /// Too winded for what they tried: the stamina bar says so.
    Winded { player: Id },
    /// Someone fell: to `killer` if a hero or creature struck them last.
    Kill { killer: Option<String>, victim: String },
    Died { id: Id },
    Revived { player: Id },
    Teleport { player: Id, pos: Vec3, facing: f32 },
    ObjSpawned { id: Id },
    ObjGone { id: Id },
    ObjChanged { id: Id },
    /// A danger mark on the ground: where a telegraphed blow will land, until it does; for a
    /// creature's blow, the blow itself (its shape, reach, timing and kind), which a client can draw.
    /// `ground`: a real spell's look laid on the ground for it (a DynamicObject: Heated Ground,
    /// Shadow Fissure...) in place of the drawn circle; 0, the circle.
    Mark { id: Id, pos: Vec3, radius: f32, tele: Option<Telegraph>, ground: u32 },
    MarkGone { id: Id },
    Bags { player: Id },
    Storage,
    Tell { player: Id, text: String },
    Announce { text: String },
    /// A big banner for everyone (or one hero).
    Banner { player: Option<Id>, title: String, sub: String },
    /// A page for one hero to read at leisure (a note, a secret found, the book's record of a
    /// dungeon): stays up till closed. `kind` is note, secret or lore; `text`'s paragraphs are
    /// split by blank lines; `links` are (command, label) to read on.
    /// A wager won (in time or not), for the camp and the Bookmaker's Board.
    WagerDone(wager::WagerRun),
    Letter { player: Id, kind: String, title: String, place: String, text: String, clue: String, links: Vec<(String, String)> },
    /// A blow coming at a hero that no guard stops (dodge it): warned for its wind-up.
    /// A blow no guard stops on its way to `player` for `secs`; `from` the creature winding it up
    /// (its weapon goes red), none for a hazard or a spell's ground.
    Danger { player: Id, secs: f32, from: Option<Id> },
    /// Open a station's work for a hero.
    StationOpen { player: Id, station: Id },
    /// Ask a hero to confirm something (ringing the bell).
    Confirm { player: Id, what: String, text: String,
    },
    /// A hero goes between the camp and the Deadmines: to a lamp below (its key), or (`None`)
    /// home to the camp.
    Travel {
        player: Id,
        to: Option<String>,
    },
    /// Done below and counted at camp too: the journal's counts, and trophies for the chest.
    ToCamp {
        counts: Vec<(String, u32)>,
        items: BTreeMap<usize, u32>,
    },
    /// A unit's lasting looks changed (an effect on it, a stance, a channel).
    Auras { id: Id },
    /// A hero held still (a root, a stun) or let go: their client is told.
    Rooted { player: Id, on: bool },
    /// A hero's own dash along the ground: which way, how fast, how long.
    Dash { player: Id, dir: f32, speed: f32, secs: f32 },
    /// A hero's ability keys, cooldowns or tree changed.
    Abilities { player: Id },
    /// A hero's Valor changed.
    Valor { player: Id },
    /// A clip played on a unit (a throw, a cast), for the clients that play clips.
    Clip { id: Id, clip: u16, secs: f32 },
    /// A unit's clip called off (a feint: the guard raised before the blow came).
    ClipStop { id: Id },
    /// A channel begins on `target` (its beam) or ends (`None`).
    Channel { caster: Id, target: Option<Id>, spell: u32, ms: u32 },
    /// A hero healed by this much (a green number).
    Healed { id: Id, amount: u32 },
    /// A unit says something (`SoundEntries.dbc`): a creature's voice.
    Sound { id: Id, sound: u32 },
    /// How the Proving Ring stands changed for a hero (in it, or just out of it).
    Arena { player: Id },
    /// How Siege stands changed for a hero in its world (the hall, the match).
    Siege { player: Id },
    /// A line for the server's log only.
    Log(String),
    /// Something worth saving changed.
    Dirty,
}

/// A creature's blow as it winds up, for the ground under it: from the creature (following it as
/// it turns or lunges), `reach` yards out, over `arc` degrees of its facing (300 and up: all
/// round), landing `secs` from now; a guard stops it (and so can parry it) or not; `foe` the hero
/// it's meant for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Telegraph {
    pub mob: Id,
    pub reach: f32,
    pub arc: f32,
    pub secs: f32,
    pub blockable: bool,
    pub foe: Option<Id>,
    /// A strip `width` wide (a line), not a wedge or a ring.
    pub width: f32,
    /// Laid on the ground there, facing that way (not under a unit): a sky-fall, a strip.
    pub at: Option<(Vec3, f32)>,
    /// A hero's own (their friends see it as a friend's, their foes as a warning).
    pub hero: bool,
}

impl Telegraph {
    /// A hero's ability on the ground: a ring (`arc` 360), a wedge or a strip.
    pub fn hero_shape(owner: Id, reach: f32, arc: f32, width: f32, secs: f32, at: Option<(Vec3, f32)>) -> Self {
        Telegraph { mob: owner, reach, arc, secs, blockable: false, foe: None, width, at, hero: true }
    }
}

/// A hero in the air: since when, and whether the jump was a Corey roll's (taken in a dodge's
/// dash, so it carries the dash's speed).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Air {
    pub since: f32,
    pub corey: bool,
}

/// A jump this late after a dodge's dash still counts as taken in it (a frame or two of the
/// client's word on its way).
pub(crate) const COREY_GRACE: f32 = 0.06;
/// A hero whose landing never came (a teleport, a lost packet) is back on the ground after this.
pub(crate) const AIR_MOST: f32 = 3.0;

/// A hero this far under the last firm ground they stood on has fallen through the world.
const FELL_THROUGH: f32 = 60.0;
/// Just arrived somewhere far (a lamp, the world below, put back): for this long a drop of
/// `LANDING_FALL` already counts, as the floor may not have been there yet when the client landed.
const LANDING_SECS: f32 = 8.0;
const LANDING_FALL: f32 = 10.0;
/// And for this long they're held where they landed (a root on their client: no falling), while
/// the floor there streams in on a client that's just been carried across the map.
pub(crate) const ARRIVE_HOLD: f32 = 1.5;
/// A hero set down somewhere far lands from this high over the floor, never level with it.
pub(crate) const ARRIVE_LIFT: f32 = 0.5;

/// How long the server carries a hero on past their client's last report.
const RECKON_FOR: f32 = 0.6;
/// The most a reckoned hero's height is lifted onto the ground the land knows (0.6 s at a sprint
/// up a steep slope).
const RECKON_RAISE: f32 = 4.0;

/// Which weapons a hero has out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kit {
    Melee,
    Bow,
}

/// What a hero is doing: one thing at a time.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    Ready,
    /// A melee swing, `t` seconds in; who it has already struck; the blow it is.
    Swing { t: f32, struck: Vec<Id>, weapon: usize, strike: Strike },
    Guard,
    /// `dash`: running the dash along the ground (a direction was held), not hopping back.
    Dodge { t: f32, dir: f32, dash: bool },
    /// An ability's reticle up (the shoulder camera), waiting for the click.
    Aim { ability: usize },
    /// Charging an ability since then (left click held).
    Charge { ability: usize, since: f32 },
    /// The beat after an ability goes.
    Cast { t: f32, total: f32, ability: usize },
    /// A tether held on `target` until then, its next tick at `next`.
    Channel { ability: usize, target: Id, until: f32, next: f32, share: f32 },
    /// A counter's stance until then.
    Stance { ability: usize, until: f32 },
    /// A dash along the ground, `t` of `secs` in, into whoever's at its end.
    Dash { t: f32, secs: f32, ability: usize, target: Option<Id>, share: f32 },
    /// Eating, `t` seconds in.
    Eat { t: f32, item: usize },
    /// Gathering from a node, `t` seconds in.
    Gather { t: f32, node: Id, at: Vec3 },
    /// Work at the camp, `t` of `total` seconds in, standing at `at`.
    Work { t: f32, total: f32, job: Job, at: Vec3 },
    /// A broken guard: nothing until then.
    Stagger { until: f32 },
    Dead { until: f32 },
}

impl Action {
    pub fn name(&self) -> &'static str {
        match self {
            Action::Ready => "ready",
            Action::Swing { strike, .. } => match strike.kind {
                StrikeKind::Heavy => "heavy",
                StrikeKind::Bash => "bash",
                StrikeKind::Kick => "kick",
                StrikeKind::Light | StrikeKind::Running | StrikeKind::Roll | StrikeKind::Leap => "swing",
            },
            Action::Guard => "guard",
            Action::Dodge { .. } => "dodge",
            Action::Aim { .. } => "aim",
            Action::Charge { .. } => "charge",
            Action::Cast { .. } => "cast",
            Action::Channel { .. } => "channel",
            Action::Stance { .. } => "stance",
            Action::Dash { .. } => "dash",
            Action::Eat { .. } => "eat",
            Action::Gather { .. } => "gather",
            Action::Work { job: Job::Craft { .. }, .. } => "craft",
            Action::Work { job: Job::Build, .. } => "build",
            Action::Work { job: Job::Repair { .. }, .. } => "repair",
            Action::Stagger { .. } => "stagger",
            Action::Dead { .. } => "dead",
        }
    }
}

/// What a hero's work at the camp makes: a recipe at a station, the station they're placing
/// (where its ghost stands), a wrecked station mended.
#[derive(Debug, Clone, PartialEq)]
pub enum Job {
    Craft { station: Id, recipe: String },
    Build,
    Repair { station: Id },
}

/// An input a hero sent: the mouse buttons and keys of the action layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Input {
    /// Left mouse down: swing, or charge (or cast) the ability being aimed.
    Attack,
    /// Left mouse up: an ability's charge goes.
    AttackUp,
    /// A heavy attack (held left, or Shift with it).
    Heavy,
    /// Raise the guard (right mouse) and let it down; it calls off an aim.
    Guard,
    GuardUp,
    /// An ability key (its slot, 0 ranged to 4 ultimate), with its EX version (held, or Ctrl).
    Ability(u8, bool),
    /// Call off the aim or charge in hand.
    Cancel,
    /// Put the weapons away, or draw them.
    Sheathe,
    /// Hold to sprint (Shift).
    Sprint,
    SprintUp,
    Dodge,
    /// Eat (key 1).
    Use,
    /// An attack with the guard up: the shield bash (a kick with no shield).
    Bash,
    /// A heavy attack with the guard up: the kick.
    Kick,
    /// Lock on to the foe nearest the facing; locked already, switch to another foe in view.
    Lock,
    /// Let go of the lock (Tab held).
    Unlock,
    /// The next usable item in the bags becomes the one R uses (G).
    Cycle,
}

/// Which movement keys a hero holds, from their client's movement flags.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Moving {
    pub forward: bool,
    pub back: bool,
    pub left: bool,
    pub right: bool,
    /// The walk (the client's own toggle).
    pub walk: bool,
}

pub struct Player {
    /// The hero's key: who they are across reconnects and restarts (a character's own key; an
    /// account's for its first footman).
    pub account: String,
    pub name: String,
    /// The login it belongs to (empty: the key is the account's own), its class (a school's key:
    /// its column's Q and E cards from the start) and how it looks.
    pub owner: String,
    pub class: String,
    pub look: HeroLook,
    pub pos: Vec3,
    pub facing: f32,
    pub moving: Moving,
    pub health: u32,
    pub max_health: u32,
    pub stamina: f32,
    /// The last time stamina was spent (it comes back a moment after).
    pub spent_at: f32,
    pub action: Action,
    pub guard_held: bool,
    pub attack_held: bool,
    /// When the guard last went up, and the time before that (a parry needs a fresh guard).
    pub guard_at: f32,
    pub prev_guard_at: f32,
    pub sprint_held: bool,
    /// Sprinting now (held, moving, not walking, not committed to anything, stamina left).
    pub sprinting: bool,
    /// In the air since a jump (their client's word) until it lands.
    pub air: Option<Air>,
    /// An attack pressed during a dodge, for as it ends (in the air: as the hero lands).
    pub dodge_attack: bool,
    /// An attack waiting for the hero to land: the leap out of a Corey roll, or the attack out of
    /// a dodge that ended in the air.
    pub landing_attack: Option<StrikeKind>,
    /// Reeling from a parried blow until then: every blow on them lands harder.
    pub riposte_until: f32,
    /// Who struck them last (a fall to another hero is announced).
    pub last_hit_by: Option<Id>,
    /// What the next hurt on them is (set by the blow about to land), and the hurts of the last
    /// few seconds: read back as the recap when they fall.
    pub cause: Option<recap::Cause>,
    pub wounds: Vec<recap::Wound>,
    /// One input waiting for the end of a commitment, and when it was pressed.
    pub buffered: Option<(Input, f32)>,
    /// The bow's drawn pose is shown (the draw's cast is out): not while they walk, when the legs
    /// must walk (the action-combat client holds the draw on the upper body meanwhile).
    pub draw_shown: bool,
    /// Who they last traded blows with, and when: the foe's bar on their screen.
    pub foe: Option<(Id, f32)>,
    /// Their client aims over the shoulder (the action-combat client): aimed shots fly along
    /// that line, beside them.
    pub shoulder_cam: bool,
    /// The reticle's line as their client last told it with a shot: a facing, an elevation
    /// (radians, up positive) and when.
    pub aim_line: Option<(f32, f32, f32)>,
    pub kit: Kit,
    pub weapon: usize,
    pub shield: usize,
    pub bow: usize,
    /// What's carried, by item.
    pub bag: BTreeMap<usize, u32>,
    /// When this hero last struck or was struck.
    pub combat_at: f32,
    /// Fractions of health healed, banked until whole.
    pub heal_bank: f32,
    /// The last spot they stood on firm, open ground: where their supplies drop if they fall
    /// somewhere unreachable.
    pub last_safe: Vec3,
    /// Until when they're still landing from a far arrival (see [`LANDING_SECS`]).
    pub landing_until: f32,
    /// Until when they're held still where they landed (see [`ARRIVE_HOLD`]); 0 when not.
    pub hold_until: f32,
    pub preview: Option<world::Preview>,
    /// The run speed as last sent.
    pub sent_speed: f32,
    pub sent_back: f32,
    /// The light chain: the next blow's place, if an attack comes by then.
    pub combo_next: u8,
    pub combo_until: f32,
    /// Their round trip to the server (seconds), for the allowances for lag.
    pub rtt: f32,
    /// Their client runs its own dashes and lunges (the action-combat client): the server sends it
    /// no speed bursts or hops for them.
    pub local_moves: bool,
    /// Who they're locked on to.
    pub lock: Option<Id>,
    /// The foes Tab has already switched through since the lock was taken (so taps go round).
    pub lock_seen: Vec<Id>,
    /// Out of a veil (Vanish) until then: their blow counts as from behind.
    pub veil_strike_until: f32,
    /// When their client last told us where they are: between its reports the server carries
    /// them on along the keys they hold (a heartbeat comes only twice a second).
    pub moved_at: f32,
    /// What they wear, by slot (`chest`, `head`), and the pack on their back.
    pub armor: BTreeMap<String, usize>,
    pub pack: Option<usize>,
    /// What a used item is still doing for them.
    pub effects: Vec<Effect>,
    /// The item the quick-item key uses: the next-item key cycles it.
    pub quick: Option<usize>,
    /// Valor: earned by fighting well, spent on EX versions and the ultimate.
    pub valor: f32,
    /// What's on the ability keys (slot to ability key), and the tree's nodes they own.
    pub loadout: BTreeMap<String, String>,
    pub owned: BTreeSet<String>,
    /// Each ability's cooldown (by the ability it runs on), and the dev switch that skips them.
    pub cooldowns: BTreeMap<usize, Cooldown>,
    pub cd_off: bool,
    /// A charge let go before its least: it goes as soon as it's reached.
    pub release_at_least: bool,
    /// The last ability cast and when.
    pub last_cast: Option<(usize, f32)>,
    /// The weapons put away (R).
    pub sheathed: bool,
    /// Held still by an effect (their client told so).
    pub rooted: bool,
    /// A panic's last stride.
    pub panic_stride: f32,
    /// Keys armed for a recast, cards empowered, a dash going through bodies.
    pub combos: recasts::Combos,
}

/// How a hero looks, as the creation screen made them: race, sex, skin, face, hair, its colour,
/// and the beard or earrings (`CharSections` choices). The footman of Goldshire by default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct HeroLook {
    pub race: u8,
    pub gender: u8,
    pub skin: u8,
    pub face: u8,
    pub hair: u8,
    pub hair_color: u8,
    pub facial: u8,
}

impl Default for HeroLook {
    fn default() -> Self {
        Self { race: 1, gender: 0, skin: 0, face: 1, hair: 2, hair_color: 3, facial: 0 }
    }
}

impl HeroLook {
    /// The five appearance bytes as the character list and the player's fields carry them.
    pub fn bytes(&self) -> [u8; 5] {
        [self.skin, self.face, self.hair, self.hair_color, self.facial]
    }
}

/// How far the camp has come, kept in the save: raids beaten, the journal's goal and what's been
/// done towards goals (`bank:wood`, `craft:roast`, `kill:kobold`, `build:smithy`...).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Camp {
    pub raids_won: u32,
    pub goal: usize,
    pub counts: BTreeMap<String, u32>,
    /// Places found (`[[site]]` keys) and chests opened (each with when it's full again, unix
    /// seconds; never for one that's once a world).
    pub found: BTreeSet<String>,
    pub opened: BTreeMap<String, f64>,
}

/// A used item's lasting effect on a hero, until then.
#[derive(Debug, Clone, PartialEq)]
pub struct Effect {
    pub kind: String,
    pub amount: f32,
    pub until: f32,
    pub name: String,
}

pub struct Mob {
    pub def: usize,
    /// A wager's Rallying: packmates fallen, each a tenth harder.
    pub rally: u32,
    pub pos: Vec3,
    pub facing: f32,
    pub home: Vec3,
    pub health: u32,
    pub max_health: u32,
    pub state: mobs::MobState,
    /// Next attack allowed at.
    pub ready_at: f32,
    /// Which pack it belongs to, or a raid.
    pub pack: Option<usize>,
    pub raider: bool,
    /// Reeling (parried, or struck mid wind-up) until then: it does nothing, and takes more.
    pub stagger_until: f32,
    /// The stagger came from a parry (the riposte bonus applies).
    pub riposte: bool,
    /// The blow it's winding up is its special one.
    pub special: bool,
    /// The attack pattern it's in (which, and which step of it), and whether this step's lunge
    /// has begun.
    pub pattern: Option<(usize, usize)>,
    pub lunged: bool,
    /// Its shield is up (shown).
    pub guarding: bool,
    /// Which way it circles a foe while it waits its turn (1 or -1), and the foe it keeps facing
    /// while it steps (circling, stepping back).
    pub circle: f32,
    pub sidle: Option<Id>,
    /// Its second wind has come (`[mob.enrage]`, or the first of its phases).
    pub enraged: bool,
    /// How many of its phases (`[[mob.phase]]`) have begun.
    pub phase: usize,
    /// A secret form lying in wait where the last one fell, unseen and untouchable until then
    /// (its `then_after`): when, the hero it rises on, and the form it steps out of.
    pub rise_at: Option<(f32, Id, usize)>,
    /// A lobbed throw being wound up: where it will come down, and the ring marked there from the
    /// wind-up's start until it lands.
    pub lob: Option<(Vec3, Id)>,
    /// Damage towards breaking its posture, and since when it's been piling up.
    pub posture: u32,
    pub posture_from: f32,
    /// Its walk: the path and how far along.
    pub path: Vec<Vec3>,
    pub speed: f32,
    pub think: f32,
    /// Lying in wait, unseen, until a hero comes near (an ambush).
    pub hidden: bool,
    /// Since when it has found no way to its foe (it gives up after a while).
    pub lost_at: Option<f32>,
    /// Where the way it's on was searched to (its last leg's end, on a long walk), so a goal that
    /// has hardly moved keeps that way instead of searching again.
    pub plan_to: Option<Vec3>,
    /// A second form (`then`): the creature it stepped out of, back in its place at a rest.
    pub form_of: Option<usize>,
    /// A panic's last stride.
    pub panic_stride: f32,
    /// It has called out on turning to fight (not again until it's home).
    pub voiced: bool,
    /// When it was last stabbed from behind (each one lately takes some of the next one's bonus).
    pub backstabs: Vec<f32>,
    /// The hero at its back it means to turn on as soon as it's free to.
    pub turn_on: Option<Id>,
    /// Heroes in its fight, the most there have been since it was last at rest at home: its
    /// health and its blows grow with them (`combat.hero_health`, `hero_damage`).
    pub fighters: u32,
    /// Its wind-up was knocked aside lately: not again until then.
    pub wary_until: f32,
    /// Who has hurt it, how much, and when last (it fades): whom it goes for.
    pub threat: Vec<(Id, f32, f32)>,
    /// When it next looks round for whom to fight.
    pub retarget_at: f32,
    /// A raider at a station struck by this hero: it ends the string it's in and turns on them.
    pub drawn_off: Option<Id>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ObjKind {
    /// The camp's shared storage chest.
    Storage,
    /// The alarm bell that calls the settlement attack.
    Alarm,
    Node { def: usize, key: String, uses: u32, ready: bool, back_at: f64 },
    Station { def: usize, uid: u64, health: u32, damaged: bool },
    /// What a fallen hero carried, waiting for them.
    Cache { uid: u64, owner: String, items: BTreeMap<usize, u32> },
    /// A station a hero is placing: where it would go, and whether it can.
    Ghost { player: Id, def: usize, ok: bool },
    /// The fire on a damaged station.
    Fire { on: Id },
    /// The weapon rack: every weapon to try.
    Rack,
    /// The Smuggler's Logbook at camp: its pages go down to the Deadmines.
    Book,
    /// A miner's lamp below (lit or not).
    Lamp { key: String },
    /// A door below: a shortcut, a fog door, a gate, a lift's first end.
    Door { key: String },
    /// A wall of shadow across a fog door or a gate, while it's shut (one of a row).
    Veil { key: String },
    /// A lift's other end (where it's set going from).
    LiftEnd { key: String },
    /// A strongbox below, not yet opened.
    Strongbox { key: String },
    /// A note left below, to be read.
    Note { key: String },
    /// A chest, cache, relic or shrine out in the land (`[[chest]]`); a kept one stands on spent.
    Chest { def: usize, spent: bool },
    /// Gear (or anything) lying where a foe fell, for the taking.
    Loot { item: usize, n: u32 },
    /// Something only to be seen: a crate the book lies on.
    Prop { display: u32, scale: f32 },
    /// The Proving Ring's banner: the way home, and the call for the next wave.
    ArenaBanner,
    /// Siege's hall: the banner where heroes join a way to play (and the way home).
    SiegeBanner,
    /// A Siege camp's Mercenary Ledger: creatures to send at rivals.
    SiegeLedger { camp: usize },
    /// A Siege camp's Battle Trainer: the class's other cards, for gold.
    SiegeTrainer { camp: usize },
}

pub struct Obj {
    pub kind: ObjKind,
    pub pos: Vec3,
    pub facing: f32,
}

/// One of the world's objects as the server draws it.
#[derive(Debug, Clone, PartialEq)]
pub struct ObjView {
    pub id: Id,
    pub kind: ObjKind,
    pub pos: Vec3,
    pub facing: f32,
    pub display: u32,
    pub scale: f32,
    pub name: String,
    /// Right-clicking it does something.
    pub usable: bool,
}

pub struct Survival {
    pub content: Content,
    pub(crate) land: Box<dyn Land>,
    pub(crate) rng: SmallRng,
    pub clock: f32,
    /// Real time (unix seconds) at clock 0: what nodes come back by.
    pub wall_start: f64,
    next_id: Id,
    pub next_uid: u64,
    pub players: BTreeMap<Id, Player>,
    pub mobs: BTreeMap<Id, Mob>,
    pub objs: BTreeMap<Id, Obj>,
    /// The camp's storage chest, by item.
    pub storage: BTreeMap<usize, u32>,
    pub events: Vec<Event>,
    pub(crate) missiles: Vec<combat::Missile>,
    /// Heroes' projectiles in flight, ground marked to land, ground burning, totems standing.
    pub(crate) projectiles: Vec<abilities::Projectile>,
    pub(crate) grounds: Vec<abilities::GroundCast>,
    pub(crate) burnings: Vec<abilities::Burning>,
    pub(crate) totems: Vec<abilities::Totem>,
    /// Leaps in the air, traps set, wind-ups and strips about to go off, anchors to step back to.
    pub(crate) landings: Vec<moves::Landing>,
    pub(crate) traps: Vec<moves::Trap>,
    pub(crate) windups: Vec<moves::Windup>,
    pub(crate) anchors: BTreeMap<Id, moves::Anchor>,
    /// Where the blow whose effects are being given came from (a knockback's centre).
    pub(crate) fx_from: Option<Vec3>,
    /// Counters' stances (a hero's, a dummy's), by unit.
    pub stances: BTreeMap<Id, Stance>,
    /// Training dummies' loops (`guard`, `roll`, `counter <ability>`).
    pub dummies: BTreeMap<Id, String>,
    /// Crowd control on units, the heroes' grace after a hard one, and each hero's log of it.
    pub cc: BTreeMap<Id, Vec<Cc>>,
    pub grace: BTreeMap<Id, f32>,
    pub(crate) cc_log: BTreeMap<Id, std::collections::VecDeque<String>>,
    /// Each hero's damage meter (see [`meter`]).
    pub(crate) meters: BTreeMap<Id, meter::Meter>,
    /// Blows on heroes waiting out their round trip (a dodge or guard already on its way).
    pub(crate) pending: Vec<combat::PendingStrike>,
    /// Where everyone has been lately, for judging a swing as its striker saw it.
    pub(crate) trails: BTreeMap<Id, std::collections::VecDeque<(f32, Vec3)>>,
    /// Danger marks still on the ground: until when; the friends' among them.
    marks: Vec<(Id, f32)>,
    friendly_marks: BTreeSet<Id>,
    pub raid: raid::Raid,
    /// The Proving Ring's run, if one's on.
    pub arena: arena::Arena,
    /// How far the camp has come: raids beaten, the journal, what's been done.
    pub camp: Camp,
    /// Packs: who's in each, and when each dead member comes back.
    pub(crate) respawns: Vec<(usize, Vec3, Option<usize>, f32)>,
    /// Patrolling packs: the point of the route each is bound for (0 its start), and when it
    /// moves on.
    pub(crate) patrols: BTreeMap<usize, (usize, f32)>,
    pub debug: bool,
    /// The world below (the Deadmines), if this is it.
    pub below: Option<Below>,
    /// Siege's world (Arathi Basin), if this is it.
    pub siege: Option<siege::Siege>,
    /// At camp: the lamps lit below (the book's pages), as the world below says.
    pub pages: Vec<String>,
    /// Hazards' looks waiting for their moment: when, the spell, where.
    pub(crate) hazard_looks: Vec<(f32, u32, Vec3, Option<Id>)>,
    /// Secret chests not yet come upon (by `[[chest]]`), and when the land was last looked over
    /// for chests filling again.
    pub(crate) unfound: BTreeSet<usize>,
    pub(crate) chests_looked: f32,
    /// Shrines' and relics' trials under way, by chest: where it stands, who called it, when a
    /// hero was last near.
    pub(crate) trials: BTreeMap<usize, Trial>,
    /// The beta page's mechanics in play: yanks, echoes, tethers, shields that burst, pads.
    pub(crate) beta: beta::Beta,
}

/// A trial at a shrine or a relic: its guardians are up until they fall (it teaches) or no hero
/// stays near (they go back to waiting).
#[derive(Debug, Clone)]
pub(crate) struct Trial {
    pub obj: Id,
    pub by: Id,
    pub near_at: f32,
}

impl Survival {
    /// A new world: the camp, the land's resources and its creatures.
    pub fn new(content: Content, land: Box<dyn Land>, seed: u64, wall_now: f64) -> Self {
        let mut s = Self::bare(content, land, seed, wall_now);
        s.place_camp();
        s.place_arena();
        s.place_nodes(None);
        s.place_chests();
        s.place_props();
        s.lay_roamers();
        s.place_packs();
        s.events.clear();
        s
    }

    /// A world with nothing in it yet.
    pub(crate) fn bare(content: Content, land: Box<dyn Land>, seed: u64, wall_now: f64) -> Self {
        Self {
            content,
            land,
            rng: SmallRng::seed_from_u64(seed),
            clock: 0.0,
            wall_start: wall_now,
            next_id: 1,
            next_uid: 1,
            players: BTreeMap::new(),
            mobs: BTreeMap::new(),
            objs: BTreeMap::new(),
            storage: BTreeMap::new(),
            events: Vec::new(),
            missiles: Vec::new(),
            projectiles: Vec::new(),
            landings: Vec::new(),
            traps: Vec::new(),
            windups: Vec::new(),
            anchors: BTreeMap::new(),
            fx_from: None,
            grounds: Vec::new(),
            burnings: Vec::new(),
            totems: Vec::new(),
            stances: BTreeMap::new(),
            dummies: BTreeMap::new(),
            cc: BTreeMap::new(),
            grace: BTreeMap::new(),
            cc_log: BTreeMap::new(),
            meters: BTreeMap::new(),
            pending: Vec::new(),
            trails: BTreeMap::new(),
            marks: Vec::new(),
            friendly_marks: BTreeSet::new(),
            raid: raid::Raid::default(),
            arena: arena::Arena::default(),
            camp: Camp::default(),
            respawns: Vec::new(),
            patrols: BTreeMap::new(),
            debug: false,
        below: None,
            siege: None,
            pages: Vec::new(),
            hazard_looks: Vec::new(),
            unfound: BTreeSet::new(),
            chests_looked: 0.0,
            trials: BTreeMap::new(),
            beta: beta::Beta::default(),
        }
    }

    pub(crate) fn alloc(&mut self) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub(crate) fn uid(&mut self) -> u64 {
        let u = self.next_uid;
        self.next_uid += 1;
        u
    }

    pub fn wall(&self) -> f64 {
        self.wall_start + self.clock as f64
    }

    pub fn land(&self) -> &dyn Land {
        self.land.as_ref()
    }

    pub(crate) fn ground(&self, x: f32, y: f32) -> Vec3 {
        [x, y, self.land.height(x, y)]
    }

    /// The ground at (x, y) for a walker coming from height `z` (its own floor, where there are
    /// floors above floors).
    pub(crate) fn ground_near(&self, x: f32, y: f32, z: f32) -> Vec3 {
        [x, y, self.land.height_near(x, y, z)]
    }

    /// Advance the world `dt` seconds and return what happened.
    pub fn step(&mut self, dt: f32) -> Vec<Event> {
        self.clock += dt;
        let online = !self.players.is_empty();
        self.reckon_players(dt);
        self.step_players(dt);
        self.step_projectiles(dt);
        self.step_moves();
        self.step_grounds();
        self.step_totems();
        self.step_stances();
        self.step_combos();
        self.step_beta();
        self.step_cc();
        self.step_holds();
        self.step_dummies();
        self.step_missiles();
        self.step_pending();
        if online {
            self.step_mobs(dt);
        }
        self.step_respawns();
        if online {
            self.step_patrols();
        }
        self.step_nodes();
        self.step_marks();
        self.step_previews();
        if self.is_below() {
            self.step_below();
        } else if self.is_siege() {
            self.step_siege(dt);
        } else {
            self.step_raid(dt);
            self.step_arena(dt);
            self.step_explore();
            self.step_goals();
        }
        self.record_trails();
        std::mem::take(&mut self.events)
    }

    pub(crate) fn tell(&mut self, player: Id, text: impl Into<String>) {
        self.events.push(Event::Tell { player, text: text.into() });
    }

    pub(crate) fn announce(&mut self, text: impl Into<String>) {
        let text = text.into();
        self.events.push(Event::Log(text.clone()));
        self.events.push(Event::Announce { text });
    }

    // --- heroes ---------------------------------------------------------------------------------

    /// A hero arrives: from their save if they have one, else fresh at the camp.
    pub fn join(&mut self, account: &str, name: &str, saved: Option<&crate::save::PlayerSave>) -> Id {
        let id = self.alloc();
        let c = &self.content;
        let spawn = self.ground(c.world.spawn[0], c.world.spawn[1]);
        let class = saved.map_or("warrior".to_string(), |s| s.class.clone());
        let (loadout, owned) = self.fresh_abilities(&class);
        let c = &self.content;
        let item = |k: &str| c.item(k).unwrap_or(0);
        let mut p = Player {
            account: account.to_string(),
            name: name.to_string(),
            owner: saved.map_or(String::new(), |s| s.owner.clone()),
            class,
            look: saved.and_then(|s| s.look).unwrap_or_default(),
            pos: spawn,
            facing: c.world.spawn_facing,
            moving: Moving::default(),
            health: c.player.health,
            max_health: c.player.health,
            stamina: c.player.stamina,
            spent_at: -100.0,
            action: Action::Ready,
            guard_held: false,
            attack_held: false,
            guard_at: -100.0,
            prev_guard_at: -100.0,
            sprint_held: false,
            sprinting: false,
            air: None,
            dodge_attack: false,
            landing_attack: None,
            riposte_until: -100.0,
            last_hit_by: None,
            cause: None,
            wounds: Vec::new(),
            buffered: None,
            draw_shown: false,
            foe: None,
            shoulder_cam: false,
            aim_line: None,
            kit: Kit::Melee,
            weapon: item(&c.start.weapon),
            shield: item(&c.start.shield),
            bow: item(&c.start.bow),
            bag: c.start.items.iter().filter_map(|(k, n)| Some((c.item(k)?, *n))).collect(),
            combat_at: -100.0,
            heal_bank: 0.0,
            last_safe: spawn,
            landing_until: 0.0,
            hold_until: 0.0,
            preview: None,
            sent_speed: c.player.run_speed,
            sent_back: BACK_SPEED,
            combo_next: 0,
            combo_until: -100.0,
            rtt: 0.0,
            local_moves: false,
            lock: None,
            lock_seen: Vec::new(),
            veil_strike_until: 0.0,
            moved_at: 0.0,
            armor: BTreeMap::new(),
            pack: None,
            effects: Vec::new(),
            quick: None,
            valor: c.valor.start,
            loadout,
            owned,
            cooldowns: BTreeMap::new(),
            cd_off: false,
            release_at_least: false,
            last_cast: None,
            sheathed: false,
            rooted: false,
            panic_stride: 0.0,
            combos: recasts::Combos::default(),
        };
        if let Some(sv) = saved {
            let near = sv.pos[2];
            crate::save::restore_player(&self.content, &mut p, sv, |x, y| self.land.height_near(x, y, near));
        }
        // Saved in the ring (a restart mid-run): back at the camp, the ring empty.
        if self.within_arena(p.pos) {
            p.pos = spawn;
            p.facing = c.world.spawn_facing;
            p.last_safe = spawn;
        }
        let dead = p.health == 0;
        self.players.insert(id, p);
        if self.is_below() {
            self.below_join(id);
        }
        if dead {
            // Still down from last time: they get up at the camp as if they'd just fallen.
            let until = self.clock + self.content.player.respawn_secs;
            self.players.get_mut(&id).unwrap().action = Action::Dead { until };
        }
        self.events.push(Event::Log(format!("{name} joined")));
        self.events.push(Event::Dirty);
        id
    }

    /// A hero leaves: what they were in the middle of is resolved as an interruption (a meal not
    /// eaten, a gather not finished); where they stand and what they carry is kept.
    pub fn leave(&mut self, id: Id) -> Option<crate::save::PlayerSave> {
        self.cancel_preview(id);
        let p = self.players.get_mut(&id)?;
        if !matches!(p.action, Action::Dead { .. } | Action::Stagger { .. }) {
            p.action = Action::Ready;
        }
        p.buffered = None;
        p.guard_held = false;
        p.attack_held = false;
        if self.is_below() {
            self.below_leave(id);
        }
        if self.is_siege() {
            self.siege_depart(id);
        }
        // Out of the ring, and kept at the camp (the ring is empty when they're back).
        if self.in_arena(id) || self.players.get(&id).is_some_and(|p| self.within_arena(p.pos)) {
            self.leave_arena(id, arena::Exit::Gone);
            let w = self.content.world.clone();
            let spawn = self.ground(w.spawn[0], w.spawn[1]);
            let p = self.players.get_mut(&id)?;
            p.pos = spawn;
            p.facing = w.spawn_facing;
        }
        let p = self.players.get_mut(&id)?;
        let save = crate::save::save_player(&self.content, p);
        self.players.remove(&id);
        self.meters.remove(&id);
        // Creatures fighting them go home.
        for m in self.mobs.values_mut() {
            if m.state.target() == Some(id) {
                m.state = mobs::MobState::Return;
            }
        }
        self.events.push(Event::Dirty);
        Some(save)
    }

    /// Where the client says the hero is (it moves itself; the server trusts it).
    pub fn move_player(&mut self, id: Id, pos: Vec3, facing: f32, moving: Moving) {
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let was = p.pos;
        let (safe, f, up) = (p.last_safe, p.facing, p.health > 0);
        let drop = if self.clock < p.landing_until { LANDING_FALL } else { FELL_THROUGH };
        if self.is_below() && pos.iter().all(|v| v.is_finite()) && self.hold_at_doors(id, was, pos)
        {
            return;
        }
        // Fallen through the world (the floor not there on arrival): back to the last firm ground.
        if pos.iter().all(|v| v.is_finite()) && pos[2] < safe[2] - drop && up {
            self.put_at(id, [safe[0], safe[1], safe[2] + ARRIVE_LIFT], f);
            self.land_from_far(id);
            if let Some(p) = self.players.get_mut(&id) {
                p.last_safe = safe;
            }
            self.tell(id, "You fell through the world: back to the last firm ground.");
            return;
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        if pos.iter().all(|v| v.is_finite()) {
            p.pos = pos;
        }
        if facing.is_finite() {
            p.facing = facing;
        }
        p.moving = moving;
        p.moved_at = self.clock;
    }

    /// Heroes just set down somewhere far: held still a moment, then let go.
    fn step_holds(&mut self) {
        let now = self.clock;
        let ids: Vec<Id> = self.players.iter().filter(|(_, p)| p.hold_until > 0.0).map(|(id, _)| *id).collect();
        for id in ids {
            if let Some(p) = self.players.get_mut(&id) {
                if now >= p.hold_until {
                    p.hold_until = 0.0;
                }
            }
            self.update_rooted(id);
        }
    }

    /// A far arrival: the next few seconds, a short drop is a fall through the world.
    pub(crate) fn land_from_far(&mut self, id: Id) {
        let until = self.clock + LANDING_SECS;
        let hold = self.clock + ARRIVE_HOLD;
        if let Some(p) = self.players.get_mut(&id) {
            p.landing_until = until;
            // The root goes out on the next tick, after the move that set them down.
            p.hold_until = hold;
        }
    }

    /// Put a hero somewhere at once (their client is told).
    /// The ground a hero standing at `at` (as the server has them) stands on. Between their
    /// client's reports the server carries a moving hero on along x and y at the height they
    /// were, so on a slope they were climbing `at` can be under the ground: the ground the land
    /// knows lifts it (never by more than `RECKON_RAISE`, as the land's own floors can be off under
    /// a roof or up a stair; and never down: up a stair, or in the air, it stays).
    pub(crate) fn standing_spot(&self, at: Vec3) -> Vec3 {
        let g = self.land.height_near(at[0], at[1], at[2] + 1.0);
        let z = if g > at[2] && g - at[2] <= RECKON_RAISE { g } else { at[2] };
        [at[0], at[1], z]
    }

    /// Set a hero down at `at` (on the ground there) by a teleport, a little above it, so their
    /// client settles onto the ground instead of dropping through it.
    pub(crate) fn set_down(&mut self, id: Id, at: Vec3, facing: f32) {
        self.put_at(id, [at[0], at[1], at[2] + ARRIVE_LIFT], facing);
    }

    pub(crate) fn put_at(&mut self, id: Id, pos: Vec3, facing: f32) {
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        p.pos = pos;
        p.facing = facing;
        p.moved_at = now;
        p.last_safe = pos;
        self.events.push(Event::Teleport { player: id, pos, facing });
    }

    /// Stuck or falling: to the lamp they'd wake at below, the camp's spawn above.
    pub fn unstuck(&mut self, id: Id) -> Result<(), String> {
        let p = self.players.get(&id).ok_or("No hero.")?;
        if p.health == 0 {
            return Err("You're down: you'll get up shortly.".into());
        }
        let account = p.account.clone();
        let (pos, f) = match self.lamp_of(&account).filter(|_| self.is_below()) {
            Some(l) => (l.at, l.facing),
            None => {
                let c = &self.content.world;
                (self.ground(c.spawn[0], c.spawn[1]), c.spawn_facing)
            }
        };
        self.put_at(id, [pos[0], pos[1], pos[2] + ARRIVE_LIFT], f);
        self.land_from_far(id);
        self.tell(id, if self.is_below() { "Back at your lamp." } else { "Back at the camp." });
        Ok(())
    }

    /// Carry each moving hero on between their client's reports, along the keys they hold at the
    /// speed the server gave them, for a moment (the next report puts them where they really are).
    /// Whether a hero is walking or running now (keys held, reported lately).
    pub fn walking(&self, p: &Player) -> bool {
        let m = p.moving;
        p.health > 0 && self.clock - p.moved_at <= RECKON_FOR && (m.forward || m.back || m.left || m.right)
    }

    fn reckon_players(&mut self, dt: f32) {
        let now = self.clock;
        let ids: Vec<Id> = self.players.keys().copied().collect();
        for id in ids {
            let p = &self.players[&id];
            let m = p.moving;
            if p.health == 0 || now - p.moved_at > RECKON_FOR || !(m.forward || m.back || m.left || m.right) {
                continue;
            }
            let (mut fx, mut fy) = (0.0f32, 0.0f32);
            let mut add = |a: f32| {
                fx += a.cos();
                fy += a.sin();
            };
            if m.forward {
                add(0.0);
            }
            if m.back {
                add(std::f32::consts::PI);
            }
            if m.left {
                add(std::f32::consts::FRAC_PI_2);
            }
            if m.right {
                add(-std::f32::consts::FRAC_PI_2);
            }
            let len = (fx * fx + fy * fy).sqrt();
            if len < 0.01 {
                continue;
            }
            // Walking (X) is the client's own pace: no slower than whatever else holds them back.
            // A roll's burst or a Charge keeps its own pace, walking or not: the client runs those
            // itself at full speed, and the server's hero mustn't trail it while hits are judged.
            let burst = match p.action {
                Action::Dodge { t, dash: true, .. } => t < self.content.dodge.dash_secs,
                Action::Dash { .. } => true,
                _ => false,
            };
            let speed = if m.back && !m.forward {
                self.back_speed_for(p)
            } else if m.walk && !burst {
                self.speed_for(id).min(WALK_SPEED)
            } else {
                self.speed_for(id)
            };
            let rel = fy.atan2(fx);
            let dir = p.facing + rel;
            let step = speed * dt;
            let (x, y) = (p.pos[0] + dir.cos() * step, p.pos[1] + dir.sin() * step);
            if self.land.blocked_near(x, y, p.pos[2]) {
                continue;
            }
            let p = self.players.get_mut(&id).unwrap();
            p.pos[0] = x;
            p.pos[1] = y;
        }
    }

    pub fn player_ids(&self) -> Vec<Id> {
        self.players.keys().copied().collect()
    }

    pub fn alive(&self, id: Id) -> bool {
        self.players.get(&id).is_some_and(|p| p.health > 0) || self.mobs.get(&id).is_some_and(|m| m.health > 0)
    }

    /// How many stacks a hero's bags hold.
    pub fn bag_capacity(&self, _p: &Player) -> usize {
        self.content.player.bag_slots
    }

    /// How many of `item` make a stack in bags with `pack` on (a pack stacks resources higher).
    pub fn stack_for(&self, item: usize, pack: Option<usize>) -> u32 {
        let d = &self.content.items[item];
        let carry = match (d.kind, pack) {
            (ItemKind::Resource, Some(k)) => self.content.items[k].carry,
            _ => 0.0,
        };
        ((d.stack.max(1) as f32) * (1.0 + carry)).round().max(1.0) as u32
    }

    /// The share every blow that lands on a hero loses to their armor (at most 60%) and a poultice.
    pub fn armor_reduce(&self, p: &Player) -> f32 {
        // (A poultice's stoneskin on top, past the armor's own limit.)
        p.armor.values().map(|i| self.content.items[*i].reduce).sum::<f32>().clamp(0.0, 0.6) + self.effect_of(p, "ward").clamp(0.0, 0.2)
    }

    /// How much their armor slows the stamina's return (a share, at most half).
    pub fn armor_weight(&self, p: &Player) -> f32 {
        p.armor.values().map(|i| self.content.items[*i].weight).sum::<f32>().clamp(0.0, 0.5)
    }

    /// The strength of a lasting effect on a hero now (0: none): what they drank or were given,
    /// and what their gear gives (its perks, and its sets').
    pub fn effect_of(&self, p: &Player, kind: &str) -> f32 {
        p.effects.iter().filter(|e| e.kind == kind && e.until > self.clock).map(|e| e.amount).sum::<f32>() + self.gear_effect(p, kind)
    }

    /// What a hero's gear gives of one kind: each piece's perks (armor worn, the weapon or bow of
    /// the kit in hand) and each set's bonuses for as many of its pieces as they have on.
    pub fn gear_effect(&self, p: &Player, kind: &str) -> f32 {
        let gear = self.gear_on(p);
        let mut total: f32 = gear.iter().flat_map(|i| self.content.items[*i].perks.iter()).filter(|k| k.kind == kind).map(|k| k.amount).sum();
        for s in &self.content.sets {
            let n = gear.iter().filter(|i| self.content.items[**i].set == s.key).count();
            total += s.bonus.iter().filter(|b| b.kind == kind && n >= b.pieces).map(|b| b.amount).sum::<f32>();
        }
        total
    }

    /// An item's perks and set in words (empty: none), for its tooltip after its own text.
    pub fn gear_words(&self, item: usize) -> String {
        let c = &self.content;
        let d = &c.items[item];
        let mut out = String::new();
        if !d.perks.is_empty() {
            let words: Vec<String> = d.perks.iter().map(|p| crate::content::perk_words(&p.kind, p.amount)).collect();
            out.push_str(&format!("{}: {}.", if d.kind == ItemKind::Armor { "Worn" } else { "In hand" }, words.join(", ")));
        }
        if let Some(s) = c.sets.iter().find(|s| s.key == d.set) {
            // (Its pieces, not their honed copies.)
            let n = c.items.iter().filter(|i| i.set == s.key && i.hone == 0).count();
            let bonus: Vec<String> = s.bonus.iter().map(|b| format!("{} pieces: {}", b.pieces, crate::content::perk_words(&b.kind, b.amount))).collect();
            if !out.is_empty() {
                out.push(' ');
            }
            out.push_str(&format!("{} ({n} pieces). {}.", s.name, bonus.join("; ")));
        }
        out
    }

    /// The gear a hero has on: their armor, and the weapon of the kit in hand.
    pub fn gear_on(&self, p: &Player) -> Vec<usize> {
        let mut g: Vec<usize> = p.armor.values().copied().collect();
        g.push(if p.kit == Kit::Bow { p.bow } else { p.weapon });
        g.sort_unstable();
        g.dedup();
        g
    }

    /// The items a hero can use from the bags, in bag order.
    pub fn usable_items(&self, p: &Player) -> Vec<usize> {
        // Below, only what's made for it (the Draughts): no food.
        let below = self.is_below();
        p.bag.iter().filter(|(i, n)| {
                **n > 0 && self.content.items[**i].kind == ItemKind::Food
                    && (!below || self.content.items[**i].below)
            }).map(|(i, _)| *i).collect()
    }

    /// The item R uses for a hero: their pick while they have it, else the first they carry.
    pub fn quick_item(&self, p: &Player) -> Option<usize> {
        let usable = self.usable_items(p);
        p.quick.filter(|q| usable.contains(q)).or_else(|| usable.first().copied())
    }

    /// The bag's free slots: each item takes a slot per full stack.
    /// The slots a hero's bags take, with their pack.
    pub fn bag_slots_used_by(&self, p: &Player) -> usize {
        self.bag_slots_with(&p.bag, p.pack)
    }

    pub fn bag_slots_with(&self, bag: &BTreeMap<usize, u32>, pack: Option<usize>) -> usize {
        bag.iter().filter(|(_, n)| **n > 0).map(|(i, n)| n.div_ceil(self.stack_for(*i, pack)) as usize).sum()
    }

    pub fn bag_slots_used(&self, bag: &BTreeMap<usize, u32>) -> usize {
        bag.iter()
            .filter(|(_, n)| **n > 0)
            .map(|(i, n)| n.div_ceil(self.content.items[*i].stack.max(1)) as usize)
            .sum()
    }

    /// Whether `add` fits in hero `id`'s bags on top of what's there.
    pub fn bag_fits(&self, id: Id, add: &BTreeMap<usize, u32>) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        let mut b = p.bag.clone();
        for (i, n) in add {
            *b.entry(*i).or_default() += n;
        }
        self.bag_slots_with(&b, p.pack) <= self.bag_capacity(p)
    }

    /// Put items in a hero's bags, as many as fit; returns what didn't.
    pub(crate) fn give(&mut self, id: Id, item: usize, n: u32) -> u32 {
        let mut left = n;
        while left > 0 {
            let one = BTreeMap::from([(item, 1)]);
            if !self.bag_fits(id, &one) {
                break;
            }
            if let Some(p) = self.players.get_mut(&id) {
                *p.bag.entry(item).or_default() += 1;
            }
            left -= 1;
        }
        if left < n {
            self.events.push(Event::Bags { player: id });
            self.events.push(Event::Dirty);
        }
        left
    }

    /// How many of `item` a hero can draw on: what they carry and the camp's chest.
    pub fn available(&self, id: Id, item: usize) -> u32 {
        // Siege's gold is a purse, not something carried.
        if self.is_siege() && self.content.items[item].key == "gold" {
            return self.gold_of(id);
        }
        let carried = self.players.get(&id).and_then(|p| p.bag.get(&item)).copied().unwrap_or(0);
        carried + self.storage.get(&item).copied().unwrap_or(0)
    }

    /// Pay `cost` from the bags first, then the chest: all or nothing.
    /// Whether the hero (with the chest) has what it costs; why not, if not.
    pub(crate) fn affords(&self, id: Id, cost: &BTreeMap<String, u32>) -> Result<Vec<(usize, u32)>, String> {
        let mut need = Vec::new();
        for (k, n) in cost {
            let i = self.content.item(k).ok_or("Unknown material.")?;
            let have = self.available(id, i);
            if have < *n {
                return Err(format!("Needs {} {} (you have {have} with the chest).", n, self.content.items[i].name));
            }
            need.push((i, *n));
        }
        Ok(need)
    }

    pub(crate) fn pay(&mut self, id: Id, cost: &BTreeMap<String, u32>) -> Result<(), String> {
        let need = self.affords(id, cost)?;
        for (i, n) in need {
            if self.is_siege() && self.content.items[i].key == "gold" {
                self.spend_gold(id, n)?;
                continue;
            }
            let p = self.players.get_mut(&id).ok_or("No such hero.")?;
            let carried = p.bag.get(&i).copied().unwrap_or(0);
            let from_bag = carried.min(n);
            if from_bag > 0 {
                let e = p.bag.get_mut(&i).unwrap();
                *e -= from_bag;
                if *e == 0 {
                    p.bag.remove(&i);
                }
            }
            let rest = n - from_bag;
            if rest > 0 {
                let s = self.storage.get_mut(&i).ok_or("The chest ran short.")?;
                *s -= rest;
                if *s == 0 {
                    self.storage.remove(&i);
                }
                self.events.push(Event::Storage);
            }
        }
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// A cost as words: "6 Elwynn Timber, 4 Fargodeep Copper".
    pub fn cost_text(&self, cost: &BTreeMap<String, u32>) -> String {
        cost.iter()
            .map(|(k, n)| format!("{n} {}", self.content.item(k).map_or(k.as_str(), |i| self.content.items[i].name.as_str())))
            .collect::<Vec<_>>()
            .join(", ")
    }

    /// The hero's run speed, as the client is told it. Walking (X) isn't in it: the client walks
    /// at its own walk speed, and a run speed of a walk's pace would stay after X back to running.
    pub(crate) fn speed_for(&self, id: Id) -> f32 {
        let Some(p) = self.players.get(&id) else { return 0.0 };
        let c = &self.content.player;
        let base = match p.action {
            Action::Dodge { t, dash: true, .. } if t < self.content.dodge.dash_secs => return self.content.dodge.speed,
            Action::Dash { .. } => return DASH_SPEED,
            Action::Swing { .. } => c.attack_speed,
            Action::Guard => c.guard_speed,
            Action::Aim { .. } | Action::Charge { .. } | Action::Cast { .. } | Action::Channel { .. } => c.aim_speed,
            Action::Stance { .. } => c.guard_speed,
            Action::Eat { .. } => c.use_speed,
            Action::Gather { .. } | Action::Work { .. } | Action::Stagger { .. } => c.use_speed,
            _ if p.sprinting => c.sprint_speed,
            _ => c.run_speed,
        };
        base * self.speed_share(id)
    }

    /// Where a hero's reticle looks as they shoot (their client's word): the facing and the
    /// elevation of the view's centre.
    pub fn set_aim_line(&mut self, id: Id, facing: f32, elevation: f32) {
        let now = self.clock;
        if let Some(p) = self.players.get_mut(&id) {
            if facing.is_finite() && elevation.is_finite() {
                p.aim_line = Some((facing.rem_euclid(std::f32::consts::TAU), elevation.clamp(-1.4, 1.4), now));
            }
        }
    }

    /// A hero's round trip to the server, as their connection measures it.
    pub fn set_rtt(&mut self, id: Id, rtt: f32) {
        if let Some(p) = self.players.get_mut(&id) {
            p.rtt = rtt.clamp(0.0, 1.0);
        }
    }

    /// A hero's client runs its own dashes and lunges.
    pub fn set_local_moves(&mut self, id: Id, on: bool) {
        if let Some(p) = self.players.get_mut(&id) {
            p.local_moves = on;
        }
    }

    /// The run-back speed: 1.12's own, but the dodge's while it dashes backwards.
    pub(crate) fn back_speed_for(&self, p: &Player) -> f32 {
        match p.action {
            Action::Dodge { t, dash: true, .. } if t < self.content.dodge.dash_secs => self.content.dodge.speed,
            _ => BACK_SPEED,
        }
    }

    // --- danger marks ---------------------------------------------------------------------------

    pub(crate) fn mark(&mut self, pos: Vec3, radius: f32, secs: f32) -> Id {
        self.mark_as(pos, radius, secs, None)
    }

    /// A mark on the ground, a friend's (holy ground: drawn green) or a warning.
    pub(crate) fn mark_kind(&mut self, pos: Vec3, radius: f32, secs: f32, friendly: bool) -> Id {
        let id = self.alloc();
        if friendly {
            self.friendly_marks.insert(id);
        }
        self.marks.push((id, self.clock + secs));
        self.events.push(Event::Mark { id, pos, radius, tele: None, ground: 0 });
        id
    }

    /// A warning laid on the ground as a real spell's look (`ground`, see [`mobs::ground_look`]):
    /// a creature's splash coming down, a hazard about to go off. 0: the drawn circle. A spell's
    /// ground look keeps its own size whatever the radius (Heated Ground covers a whole room), so
    /// below [`GROUND_LOOK_MIN`] the circle is drawn instead: it shows where the blow lands.
    pub(crate) fn mark_ground(&mut self, pos: Vec3, radius: f32, secs: f32, ground: u32) -> Id {
        let ground = if radius < GROUND_LOOK_MIN { 0 } else { ground };
        let id = self.alloc();
        self.marks.push((id, self.clock + secs));
        self.events.push(Event::Mark { id, pos, radius, tele: None, ground });
        id
    }

    /// A mark that's a friend's, not a warning.
    pub fn mark_friendly(&self, id: Id) -> bool {
        self.friendly_marks.contains(&id)
    }

    /// A mark that is a creature's blow too (see [`Telegraph`]).
    pub(crate) fn mark_as(&mut self, pos: Vec3, radius: f32, secs: f32, tele: Option<Telegraph>) -> Id {
        let id = self.alloc();
        self.marks.push((id, self.clock + secs));
        self.events.push(Event::Mark { id, pos, radius, tele, ground: 0 });
        id
    }

    /// Seconds a danger mark has left on the ground (0: gone).
    pub fn mark_left(&self, id: Id) -> f32 {
        self.marks.iter().find(|(m, _)| *m == id).map_or(0.0, |(_, t)| (t - self.clock).max(0.0))
    }

    pub(crate) fn unmark(&mut self, id: Id) {
        if let Some(k) = self.marks.iter().position(|(m, _)| *m == id) {
            self.marks.remove(k);
            self.friendly_marks.remove(&id);
            self.events.push(Event::MarkGone { id });
        }
    }

    fn step_marks(&mut self) {
        let now = self.clock;
        let gone: Vec<Id> = self.marks.iter().filter(|(_, t)| *t <= now).map(|(id, _)| *id).collect();
        for id in gone {
            self.unmark(id);
        }
    }

    // --- views ----------------------------------------------------------------------------------

    pub fn objects(&self) -> Vec<ObjView> {
        self.objs.keys().filter_map(|&id| self.object(id)).collect()
    }

    pub fn object(&self, id: Id) -> Option<ObjView> {
        let o = self.objs.get(&id)?;
        let c = &self.content;
        let (display, scale, name, usable) = match &o.kind {
            ObjKind::Storage => (10, 1.0, "Camp Storage".to_string(), true),
            ObjKind::Alarm => (4571, 0.6, "Alarm Bell".to_string(), true),
            ObjKind::Node { def, ready, .. } => {
                let n = &c.nodes[*def];
                if !ready {
                    return None;
                }
                (n.display, n.scale, n.name.clone(), true)
            }
            ObjKind::Station { def, damaged, .. } => {
                let s = &c.stations[*def];
                let name = if *damaged { format!("{} (damaged)", s.name) } else { s.name.clone() };
                (s.display, s.scale, name, true)
            }
            ObjKind::Cache { owner, .. } => (2890, 1.0, format!("{owner}'s Supplies"), true),
            ObjKind::Ghost { def, .. } => {
                let s = &c.stations[*def];
                (s.display, s.scale, format!("{} (placing)", s.name), false)
            }
            ObjKind::Fire { .. } => (397, 0.8, "Fire".to_string(), false),
            ObjKind::Rack => (130, 1.0, "Weapon Rack".to_string(), true),
            ObjKind::Book => (BOOK_DISPLAY, BOOK_SCALE, "The Smuggler's Logbook".to_string(), true),
            ObjKind::Prop { display, scale } => (*display, *scale, String::new(), false),
            ObjKind::ArenaBanner => (
                arena::ARENA_BANNER_DISPLAY,
                1.0,
                c.arena.as_ref().map_or_else(String::new, |a| format!("{}'s Banner", a.page)),
                true,
            ),
            ObjKind::SiegeBanner => (siege::SIEGE_BANNER_DISPLAY, 1.3, "The Siege Muster".to_string(), true),
            ObjKind::SiegeLedger { .. } => (BOOK_DISPLAY, BOOK_SCALE, "Mercenary Ledger".to_string(), true),
            ObjKind::SiegeTrainer { .. } => (siege::SIEGE_BANNER_DISPLAY, 1.0, "Battle Trainer".to_string(), true),
            ObjKind::Lamp { key } => {
                let name = c
                    .dungeon
                    .as_ref()
                    .and_then(|d| d.lamps.iter().find(|l| &l.key == key))
                    .map_or("Miner's Lamp".to_string(), |l| l.name.clone());
                let lit = self.below.as_ref().is_some_and(|b| b.lit.contains(key));
                (
                    if lit {
                        LAMP_LIT_DISPLAY
                    } else {
                        LAMP_DARK_DISPLAY
                    },
                    1.0,
                    if lit {
                        format!("Lamp: {name}")
                    } else {
                        "Unlit Miner's Lamp".to_string()
                    },
                    true,
                )
            }
            ObjKind::Note { key } => {
                let n = c.dungeon.as_ref().and_then(|d| d.notes.iter().find(|x| &x.key == key))?;
                (if n.display != 0 { n.display } else { NOTE_DISPLAY }, n.scale, n.name.clone(), true)
            }
            ObjKind::Door { key } => {
                let d = c.dungeon.as_ref().and_then(|d| d.doors.iter().find(|x| &x.key == key))?;
                match d.kind.as_str() {
                    // A fog or a gate is only its veil.
                    "fog" | "gate" => return None,
                    // A hidden spot is its marker, searched till it gives its secret up (then cleared
                    // aside for what it kept, as a false wall gives way).
                    "hidden" | "illusory" if self.door_is_open(key) => return None,
                    // A secret looks like what it hides behind.
                    "hidden" | "illusory" if !d.look.is_empty() => (d.display, d.scale, d.look.clone(), true),
                    "lift" => (if d.display != 0 { d.display } else { LEVER_DISPLAY }, d.scale, d.name.clone(), true),
                    _ => (d.display, d.scale, d.name.clone(), true),
                }
            }
            ObjKind::Veil { key } => {
                if self.door_is_open(key) {
                    return None;
                }
                let name = c.dungeon.as_ref().and_then(|d| d.doors.iter().find(|x| &x.key == key)).map_or(String::new(), |d| d.name.clone());
                (VEIL_DISPLAY, 1.0, name, false)
            }
            ObjKind::LiftEnd { key } => {
                let d = c.dungeon.as_ref().and_then(|d| d.doors.iter().find(|x| &x.key == key))?;
                (if d.display != 0 { d.display } else { LEVER_DISPLAY }, d.scale, d.name.clone(), true)
            }
            ObjKind::Strongbox { key } => {
                let name = c.dungeon.as_ref().and_then(|d| d.boxes.iter().find(|b| &b.key == key)).map_or(String::new(), |b| b.name.clone());
                (STRONGBOX_DISPLAY, 1.0, if name.is_empty() { "Defias Strongbox".to_string() } else { name }, true)
            }
            ObjKind::Chest { def, spent } => {
                let d = &c.chests[*def];
                (d.display, d.scale, d.name.clone(), !spent)
            }
            ObjKind::Loot { item, .. } => (LOOT_DISPLAY, 0.6, c.items[*item].name.clone(), true),
        };
        Some(ObjView { id, kind: o.kind.clone(), pos: o.pos, facing: o.facing, display, scale, name, usable })
    }

    /// The melee weapon a hero holds.
    pub fn weapon_of(&self, p: &Player) -> &crate::content::WeaponDef {
        let c = &self.content;
        c.weapon(&c.items[p.weapon].key).or_else(|| c.weapons.first()).expect("a weapon")
    }

    /// What a hero guards with: the shield, or a two-handed weapon's own guard.
    pub fn guard_of(&self, p: &Player) -> &crate::content::ShieldDef {
        let c = &self.content;
        let w = self.weapon_of(p);
        let key = if w.two_handed { w.guard.as_str() } else { c.items[p.shield].key.as_str() };
        c.shield(key).or_else(|| c.shields.first()).expect("a shield")
    }

    /// The bow or crossbow a hero carries.
    pub fn ranged_of(&self, p: &Player) -> &crate::content::RangedDef {
        let c = &self.content;
        c.ranged(&c.items[p.bow].key).or_else(|| c.ranged.first()).expect("a bow")
    }

    /// How many creatures a hero's swing would reach right now (in reach, in front, in sight).
    pub fn foes_in_reach(&self, id: Id) -> usize {
        let Some(p) = self.players.get(&id) else { return 0 };
        let w = self.weapon_of(p);
        let reaches = |at: Vec3, size: f32| {
            dist2d(p.pos, at) <= w.range + size
                && angle_between(p.facing, facing(p.pos, at)) <= w.arc / 2.0
                && crate::land::clear_line(self.land.as_ref(), p.pos, at)
        };
        let mobs = self.mobs.values().filter(|m| m.health > 0 && reaches(m.pos, 0.5 * self.content.mobs[m.def].scale)).count();
        let heroes = if self.content.pvp.friendly_fire {
            self.players.iter().filter(|(hid, h)| **hid != id && h.health > 0 && self.hostile(id, **hid) && reaches(h.pos, 0.4)).count()
        } else {
            0
        };
        mobs + heroes
    }

    /// The foe a hero last traded blows with, while it's fresh: (name, health, max health).
    pub fn foe_view(&self, id: Id) -> Option<(String, u32, u32)> {
        let (foe, at) = self.players.get(&id)?.foe?;
        if self.clock - at > FOE_SHOWN_SECS {
            return None;
        }
        if let Some(m) = self.mobs.get(&foe) {
            // A foe killed: its empty bar a moment, then gone.
            if m.health == 0 && self.clock - at > FOE_DEAD_SECS {
                return None;
            }
            return Some((self.content.mobs[m.def].name.clone(), m.health, m.max_health));
        }
        let h = self.players.get(&foe)?;
        Some((h.name.clone(), h.health, h.max_health))
    }

    /// How many meals a hero carries.
    pub fn food_count(&self, id: Id) -> u32 {
        let Some(p) = self.players.get(&id) else { return 0 };
        p.bag.iter().filter(|(i, _)| self.content.items[**i].kind == ItemKind::Food).map(|(_, n)| *n).sum()
    }

    pub fn item_kind(&self, item: usize) -> ItemKind {
        self.content.items[item].kind
    }

    pub(crate) fn roll(&mut self, range: [u32; 2]) -> u32 {
        if range[1] <= range[0] {
            range[0]
        } else {
            self.rng.gen_range(range[0]..=range[1])
        }
    }
}

/// The Smuggler's Logbook (an open book), a lamp below (a brazier) unlit and lit, a strongbox, and
/// a small crate for gear lying where a foe fell (`GameObjectDisplayInfo`).
pub const BOOK_DISPLAY: u32 = 560;
/// The book's drawn big enough to see in the grass, open on a smuggler's crate (this high).
pub const BOOK_SCALE: f32 = 2.6;
pub const BOOK_CRATE_DISPLAY: u32 = 18;
pub const BOOK_CRATE_HEIGHT: f32 = 0.5;
/// The book keeps this far from any tree's trunk (one hid it).
pub const BOOK_CLEAR: f32 = 3.0;
pub const LAMP_DARK_DISPLAY: u32 = 759;
pub const LAMP_LIT_DISPLAY: u32 = 654;
pub const STRONGBOX_DISPLAY: u32 = 259;
pub const LOOT_DISPLAY: u32 = 335;
/// A tall column of shadow (a fog across a boss's door), and a lever (a lift's ends).
pub const VEIL_DISPLAY: u32 = 2234;
pub const LEVER_DISPLAY: u32 = 295;
/// A note lying below (a scroll), unless it says otherwise.
pub const NOTE_DISPLAY: u32 = 163;

/// How long after the last blow traded a foe's bar stays up.
/// A killed foe's bar stays this long, empty.
const FOE_DEAD_SECS: f32 = 1.0;
const FOE_SHOWN_SECS: f32 = 8.0;
/// 1.12's run-back speed (yards a second), its walk, and an ability's dash.
pub const BACK_SPEED: f32 = 4.5;
pub const WALK_SPEED: f32 = 2.5;

/// The smallest marked area (yards) shown as a spell's look on the ground rather than the drawn
/// circle. The looks are as big as they are (play-test: the Deadmines' dynamite, 2.6 yards, laid
/// fire over the whole floor), so a splash smaller than this gets its true size drawn.
pub const GROUND_LOOK_MIN: f32 = 6.0;
pub const DASH_SPEED: f32 = 30.0;

/// The smallest angle between two facings, in degrees.
pub fn angle_between(a: f32, b: f32) -> f32 {
    let d = (a - b).rem_euclid(std::f32::consts::TAU);
    d.min(std::f32::consts::TAU - d).to_degrees()
}
