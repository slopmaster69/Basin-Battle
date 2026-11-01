//! The abilities (`[[ability]]` in `survival.toml`): every aimed, charged, placed or timed thing a
//! hero can do past the sword and shield, their EX versions (held a moment, or Ctrl, for Valor),
//! the crowd control rules (`[cc]`) and Valor itself (`[valor]`), and the board they're chosen on:
//! five rows (one per key), four cards in each.

use std::collections::BTreeMap;

use serde::Deserialize;

/// A hero's five ability keys, each a row of the board: reaching the foe (Q), in the brawl (E),
/// moving and control (1), staying alive (2), and the ultimate (3).
pub const SLOTS: [&str; 5] = ["ranged", "close", "utility", "defense", "ultimate"];

/// The row an ability's slot is (one row, one key).
pub fn slot_role(slot: &str) -> &str {
    slot
}

/// The board's columns: the nine classes.
pub const SCHOOLS: [&str; 9] = ["warrior", "paladin", "hunter", "rogue", "priest", "shaman", "mage", "warlock", "druid"];
/// Each class's `ChrClasses.dbc` id, in the order of [`SCHOOLS`] (the creation screen's class).
pub const CLASS_IDS: [u8; 9] = [1, 2, 3, 4, 5, 7, 8, 9, 11];

/// The class (a school's key) for a `ChrClasses.dbc` id.
pub fn school_of_class(id: u8) -> Option<&'static str> {
    CLASS_IDS.iter().position(|c| *c == id).map(|k| SCHOOLS[k])
}

/// The `ChrClasses.dbc` id of a class (a Warrior's for anything else).
pub fn class_id_of(school: &str) -> u8 {
    SCHOOLS.iter().position(|s| *s == school).map_or(1, |k| CLASS_IDS[k])
}

/// How an ability is used.
pub const KINDS: [&str; 17] = [
    // A leap through the air to the reticle's point (or, `back`, away from the facing): where it
    // lands is drawn for everyone, and what's there when it does is struck.
    "leap",
    // A straight strip along the reticle from the caster: `range` long, `radius` either side.
    "line",
    // Set down at the reticle (or the feet): it arms after `delay` and goes off on the first foe
    // to step within `radius`.
    "trap",
    // A blow with the weapon in hand: its own wind-up, reach and share of the weapon's damage.
    "art",
    // Something on the caster for a while (the effects are boons on them).
    "buff",
    // A projectile along the reticle: it flies, and strikes the first thing it touches.
    "projectile",
    // The ground at the reticle: marked, then it lands there (and may burn a while).
    "ground",
    // Something set down at the reticle: a totem.
    "place",
    // A cone in front, along the reticle.
    "cone",
    // All round the caster.
    "nova",
    // A tether to the first foe along the reticle, held while it lasts.
    "channel",
    // A dash along the reticle.
    "dash",
    // A step through space to the reticle's point.
    "blink",
    // A counter's stance: it fires when a blow or a projectile reaches you.
    "stance",
    // The bow: a projectile whose numbers are the hero's bow's own.
    "shot",
    // Help on a friend, at once: the first friendly hero along the reticle within `range` (traced
    // `radius` wide, 1 yd if none), else the one nearest the reticle within 15° of it, else the
    // caster. It gets `ally_heal`, `cleanse` and the boons among `effects` (nothing else of them).
    "ally",
    // A sweeping channel: a strip from the caster along their aim, `range` long and `radius`
    // either side, held `secs` and moving as they turn; every `tick` it deals `damage` a second
    // (times the tick) to each foe in it, with `effects`, and heals each friend in it `ally_heal`
    // a second.
    "beam",
];

/// One ability, or an ability's EX version (`ex_of` names the ability it's the EX of).
#[derive(Debug, Clone, Deserialize)]
pub struct AbilityDef {
    pub key: String,
    pub name: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub icon: String,
    /// Its column of the board (`core`: none, an EX version or one fired by something else), and
    /// the row (the key) it goes on.
    #[serde(default = "core")]
    pub school: String,
    /// No longer used (the ring tree's border and ring).
    #[serde(default)]
    pub border: String,
    #[serde(default)]
    pub ring: u32,
    pub slot: String,
    pub kind: String,
    /// This is the EX version of that ability (it shares its cooldown, and costs `valor`).
    #[serde(default)]
    pub ex_of: String,
    /// What it costs in Valor (an EX version 25 or 50, an ultimate the whole bar).
    #[serde(default)]
    pub valor: u32,
    /// The charge: the least that lets it go and the full charge (seconds); `[0, 0]` goes on the
    /// click.
    #[serde(default)]
    pub charge: [f32; 2],
    /// A full charge can be held this long past full before it goes by itself (0: until let go).
    #[serde(default)]
    pub hold: f32,
    /// Damage, range and radius at the least and the full charge (one value: both).
    #[serde(default)]
    pub damage: [f32; 2],
    #[serde(default)]
    pub range: [f32; 2],
    #[serde(default)]
    pub radius: [f32; 2],
    /// A projectile's speed (the visual's own, so what's seen is where it is).
    #[serde(default)]
    pub speed: f32,
    /// How many projectiles, spread this many degrees apart.
    #[serde(default = "one")]
    pub count: u32,
    #[serde(default)]
    pub spread: f32,
    /// The cone's width (degrees).
    #[serde(default)]
    pub arc: f32,
    /// Ground: seconds between the mark and the landing; then burning ground this long, at this
    /// much a second.
    #[serde(default)]
    pub delay: f32,
    #[serde(default)]
    pub linger: f32,
    #[serde(default)]
    pub linger_damage: f32,
    /// A channel's (a beam's) or a stance's length, a totem's life; a channel or a beam ticks (a
    /// totem pulses) every `tick`.
    #[serde(default)]
    pub secs: f32,
    #[serde(default)]
    pub tick: f32,
    /// The caster heals this share of the damage it does (a drain).
    #[serde(default)]
    pub drain: f32,
    /// The caster heals this share of their health when it lands (Death Coil).
    #[serde(default)]
    pub heal: f32,
    /// It kills outright what's below this share of its health.
    #[serde(default)]
    pub execute: f32,
    /// A kill with it gives back the Valor it cost.
    #[serde(default)]
    pub refund_kill: bool,
    /// A heavy blow on the caster while charging breaks the charge.
    #[serde(default)]
    pub fragile: bool,
    #[serde(default)]
    pub cooldown: f32,
    /// Charges held at once (each comes back on its own cooldown); 1 for most.
    #[serde(default = "one")]
    pub charges: u32,
    /// What it does to what it strikes.
    #[serde(default)]
    pub effects: Vec<AbilityEffect>,
    /// A dash's or a blink's: what it does to the caster's own effects (`cleanse` roots and
    /// snares), and whether it's untouchable meanwhile.
    #[serde(default)]
    pub cleanse: bool,
    #[serde(default)]
    pub invulnerable: bool,
    /// An EX blink leaves this ability (a Frost Nova) where it started.
    #[serde(default)]
    pub leaves: String,
    /// A placed totem: its look, health and the radius it works in.
    #[serde(default)]
    pub totem: Option<TotemDef>,
    /// A counter's stance: what it does when something reaches the hero.
    #[serde(default)]
    pub counter: Option<CounterDef>,
    /// Its looks: the 1.12 spell flown or cast (`spell`), the charge's look (`charge_spell`, its
    /// start shown to everyone), an impact kit on what it strikes, a thrown item's look (`ammo`),
    /// and an aura shown on the caster while it lasts (a stance, a channel).
    #[serde(default)]
    pub spell: u32,
    #[serde(default)]
    pub charge_spell: u32,
    /// At full charge: a faster missile (its own spell, flying at its own speed, so the look keeps
    /// pace) and a flare on the caster as the charge tops out (0: the school's).
    #[serde(default)]
    pub full_spell: u32,
    #[serde(default)]
    pub full_speed: f32,
    #[serde(default)]
    pub full_kit: u32,
    #[serde(default)]
    pub impact_kit: u32,
    /// A trap's look while it's set (`GameObjectDisplayInfo`: 3072 an Explosive Trap, 3071 a
    /// Freezing Trap), and the spell shown at its spot as it goes off (its ground one-shot).
    #[serde(default)]
    pub object: u32,
    #[serde(default)]
    pub burst_spell: u32,
    /// The look its lingering ground wears (a DynamicObject of this spell for as long as it
    /// lingers: Consecration's holy ground, Flamestrike's fire); 0: its own `spell`.
    #[serde(default)]
    pub linger_spell: u32,
    #[serde(default)]
    pub ammo: u32,
    #[serde(default)]
    pub aura: u32,
    /// The cast's clip (`AnimationData`), played on the caster as it goes.
    #[serde(default)]
    pub clip: u16,
    /// Valor for each foe it strikes (0: `[valor].ability`).
    #[serde(default)]
    pub valor_hit: f32,
    /// Only ever fired by something else (a counter's reflected blast): no node, no key.
    #[serde(default)]
    pub inner: bool,
    /// Off the board (kept for tests and for later); a dev build can still put it on a key.
    #[serde(default)]
    pub bench: bool,
    /// Every hero has it from the start (the rest are unlocked later; all in a dev build).
    #[serde(default)]
    pub free: bool,
    /// A weapon art's blow: its wind-up (seconds), its share of the weapon's damage, yards past the
    /// weapon's reach, its arc (0: the weapon's), its stamina; a guard that blocks it loses
    /// `crush` times the stamina; no guard stops it or parry turns it (`unblockable`); light
    /// blows don't knock it out of its wind-up (`poise`); from behind it does `behind` times as
    /// much; `ambush`: first a step behind the foe under the reticle, this far off at most.
    #[serde(default)]
    pub windup: f32,
    #[serde(default)]
    pub weapon: f32,
    #[serde(default)]
    pub reach: f32,
    #[serde(default)]
    pub stamina: f32,
    #[serde(default)]
    pub crush: f32,
    #[serde(default)]
    pub unblockable: bool,
    #[serde(default)]
    pub poise: bool,
    #[serde(default)]
    pub behind: f32,
    #[serde(default)]
    pub ambush: f32,
    /// A Mark it uses up makes it a critical hit, this many times the damage.
    #[serde(default)]
    pub mark_crit: f32,
    /// On a frozen foe it shatters the ice for half again its damage.
    #[serde(default)]
    pub shatter: bool,
    /// A projectile that touches a friend heals them this share of their health (and, with
    /// `cleanse`, ends what's on them); so does an `ally` on whoever it picks. A `beam` heals each
    /// friend in it this share a second.
    #[serde(default)]
    pub ally_heal: f32,
    /// Ground laid around the caster at once, not at the reticle.
    #[serde(default)]
    pub at_self: bool,
    /// Ground that heals friends in it this share of their health a second, and gives them this
    /// boon (for a moment, again each second they stand in it) at this strength.
    #[serde(default)]
    pub linger_heal: f32,
    #[serde(default)]
    pub boon: String,
    #[serde(default)]
    pub boon_amount: f32,
    /// A projectile goes on through this many bodies past the first.
    #[serde(default)]
    pub pierce: u32,
    /// A projectile's hit leaps on to the nearest other foe within `chain_range`, this many times,
    /// each doing `chain_falloff` of the last.
    #[serde(default)]
    pub chain: u32,
    #[serde(default)]
    pub chain_range: f32,
    #[serde(default = "falloff")]
    pub chain_falloff: f32,
    /// A projectile bursts this wide where it hits (a body or the ground): everyone else in the
    /// burst is struck too.
    #[serde(default)]
    pub splash: f32,
    /// A leap's arc (its upward speed, yards a second: 6 is a bound); `back`: it goes the range
    /// away from the facing, not to the reticle (a disengage).
    #[serde(default)]
    pub lift: f32,
    #[serde(default)]
    pub back: bool,
    /// A blink with an anchor: the first press sets it at the feet (and costs nothing); pressed
    /// again within this many seconds, the hero steps back to it.
    #[serde(default)]
    pub anchor: f32,
    /// Another ability fired as well, at once, from the same aim (a Disengage's Net).
    #[serde(default)]
    pub also: String,
    /// Effects on the caster as it goes (a leap's haste, a cast that shields you).
    #[serde(default)]
    pub self_effects: Vec<AbilityEffect>,
    /// Lingering ground's effects on each foe inside, every tick (a vortex's pull, a storm's snare).
    #[serde(default)]
    pub tick_effects: Vec<AbilityEffect>,
    /// A cone's, a nova's or a line's wind-up (seconds): drawn on the ground for everyone, then it
    /// goes off; a stun or a stagger meanwhile loses it.
    #[serde(default)]
    pub cast: f32,
    /// A totem that swallows the first foe's projectile flying within its radius.
    #[serde(default)]
    pub grounding: bool,
    /// A projectile that meets friends too: they get what helps or moves of its effects (a pull to
    /// safety), not its harm.
    #[serde(default)]
    pub friendly: bool,
    /// Friends in its area (the caster too) heal this share of their health as it goes off.
    #[serde(default)]
    pub area_heal: f32,
    /// Lingering ground's look each tick (a spell shown at its middle: arrows, lightning); a
    /// beam's, from the caster to the first foe in it or its far end.
    #[serde(default)]
    pub tick_spell: u32,
    /// A second activation (Battlerite's recast): once this goes, its key is armed for
    /// `recast_secs`, and pressed again meanwhile it fires this `inner` ability from the hero's
    /// aim, for no Valor and no cooldown (the cooldown was spent by the first cast). An EX
    /// version can have its own. Unused, the window just closes.
    #[serde(default)]
    pub recast: String,
    #[serde(default)]
    pub recast_secs: f32,
    /// A blink or a leap fired as a recast that goes back to where the cast that armed it began
    /// (a Shadowstep there and back), not to the reticle.
    #[serde(default)]
    pub recast_to_start: bool,
    /// A dash that goes through bodies: it strikes every foe it passes once, as it passes, and
    /// stops only at a wall, a step or its range.
    #[serde(default)]
    pub through: bool,
    /// A combo: once this goes, for `secs` the next press of that base card's key (the same hero's,
    /// on whatever key) fires its EX version for no Valor; the proc is spent then.
    #[serde(default)]
    pub empower: Option<EmpowerDef>,
    /// A projectile that comes back (a boomerang): out to its full range (or a wall), then back to
    /// where the caster is now, striking every foe it passes once each way. It ends at the caster.
    /// The way back strikes at `return_damage` of the way out (1: the same).
    #[serde(default)]
    pub returns: bool,
    #[serde(default = "full_share")]
    pub return_damage: f32,
    /// A projectile lobbed in an arc to the reticle's point on the ground (within its range): it
    /// flies over bodies, its landing ring shown to everyone meanwhile, and lands after its flight
    /// (distance / `speed`) to burst `splash` wide there (then `linger`). Needs `splash`.
    #[serde(default)]
    pub lob: bool,
    /// A sticky projectile: what it strikes, it sticks to (or the ground or wall it meets, or where
    /// its range ends), and it goes off this many seconds later, a `splash` burst round the unit
    /// where it is then (where it fell, if it died). Needs `splash`. While it's stuck the unit
    /// shows `fuse_aura` (a spell's state look; 0: none), and for the last second a ring follows
    /// them for everyone.
    #[serde(default)]
    pub fuse: f32,
    #[serde(default)]
    pub fuse_aura: u32,
    /// A projectile (or a shot) that pulls its caster after it (a grappling hook): what it strikes
    /// (a foe, or a wall or the ground it meets) yanks them there in a fast dash along the ground,
    /// stopped short by a wall, ending beside a foe (not in them). Gone its full range without
    /// meeting anything, it does nothing.
    #[serde(default)]
    pub yank: bool,
    /// A projectile that swaps its caster and what it strikes (a foe, a creature, a friend): each
    /// is set down where the other stood. A champion, and anything steadfast or untouchable, holds
    /// its ground (it's only struck).
    #[serde(default)]
    pub swap: bool,
    /// A projectile that binds the foe it strikes to the caster for `tether_secs`, a link drawn
    /// between them (`tick_spell`, a channelled spell's look): once the foe is farther than
    /// `tether_range` from them, it's pulled back to the caster and given `tether_effects` (a
    /// stun) as it lands, and the link ends. With `chain`, the foes within `chain_range` of the
    /// first are bound too (as many as `chain`).
    #[serde(default)]
    pub tether_secs: f32,
    #[serde(default)]
    pub tether_range: f32,
    #[serde(default)]
    pub tether_effects: Vec<AbilityEffect>,
    /// A buff that leaves an echo where the caster stands, holding their health then (its look:
    /// `burst_spell` there once, `linger_spell` on the ground while it lasts): its recast (with
    /// `recast`, whose ability then goes off from the echo), or else `rewind_secs` after the cast,
    /// snaps them back to it, with the health it holds if that's more than they have now (never
    /// less). Unused, a recast's echo fades with its window.
    #[serde(default)]
    pub rewind: bool,
    #[serde(default)]
    pub rewind_secs: f32,
    /// A trap that launches instead of springing (a jump pad): the caster or a friend who steps on
    /// it is thrown this many yards the way they face, as a leap (`lift`), landing with `damage`
    /// and `effects` within its `radius`. Foes don't set it off; it stays its whole life, each
    /// hero launched at most once a second.
    #[serde(default)]
    pub launch: f32,
    /// Lingering ground that bursts as it ends (a black hole's collapse): this ability (a `nova`:
    /// its damage, effects and look) goes off over it, as wide as the ground (or the nova's own
    /// `radius`, if it has one).
    #[serde(default)]
    pub end_burst: String,
}

/// What an ability empowers as it goes: the base card (`ability`, one with an EX version) whose
/// next press is its EX version for free, within `secs`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct EmpowerDef {
    pub ability: String,
    pub secs: f32,
}

fn falloff() -> f32 {
    0.75
}

fn full_share() -> f32 {
    1.0
}

fn core() -> String {
    "core".into()
}

fn one() -> u32 {
    1
}

impl AbilityDef {
    /// A value at a share of the charge (0 the least, 1 full).
    pub fn at(range: [f32; 2], share: f32) -> f32 {
        range[0] + (range[1] - range[0]) * share.clamp(0.0, 1.0)
    }
    /// It needs a charge (held, then let go).
    pub fn charged(&self) -> bool {
        self.charge[1] > 0.0
    }
    /// It's aimed with the reticle (the shoulder camera comes up for it).
    pub fn aimed(&self) -> bool {
        !matches!(self.kind.as_str(), "nova" | "stance" | "art" | "buff")
            && !self.at_self
            && !(self.kind == "leap" && self.back)
            && self.anchor <= 0.0
            && !(self.recast_to_start && matches!(self.kind.as_str(), "blink" | "leap"))
    }
    /// On the board (a card to choose), not off it or only fired by something else.
    pub fn on_board(&self) -> bool {
        !self.is_ex() && !self.inner && !self.bench
    }
    /// A projectile that meets friends too (to heal them, to pull them to safety, to swap places).
    pub fn friends(&self) -> bool {
        self.ally_heal > 0.0 || self.friendly || self.swap
    }
    /// It mends friends (a heal on whoever it reaches, a nova's, holy ground's mending or boon):
    /// a hero it can help is never harmed by it, friendly fire or not.
    pub fn heals(&self) -> bool {
        self.ally_heal > 0.0 || self.area_heal > 0.0 || self.linger_heal > 0.0 || !self.boon.is_empty()
    }
    pub fn is_ex(&self) -> bool {
        !self.ex_of.is_empty()
    }
    /// Full charge reached (the share a release goes at).
    pub fn full(share: f32) -> bool {
        share >= FULL_SHARE
    }
}

/// From this share of the charge a release counts as full (the faster missile, the flare).
pub const FULL_SHARE: f32 = 0.99;

/// The flare on the caster as a charge tops out, by class (`SpellVisualKit` ids: Sweeping
/// Strikes', Righteous Fury's, Shadow Trance's, Combustion's impacts, and the like).
pub fn full_kit_of(school: &str) -> u32 {
    match school {
        "warrior" | "hunter" | "rogue" => 353,
        "paladin" | "priest" => 457,
        "warlock" => 4569,
        "mage" => 448,
        "shaman" => 62,
        "druid" => 56,
        _ => 0,
    }
}

/// What an ability does to what it strikes: `kind` one of [`CC_KINDS`] or `burn` (damage a second),
/// `mark` (the next hit on it does `amount` more), `heal` (the caster), `cleanse_panic`; for
/// `secs` (at the least and the full charge).
#[derive(Debug, Clone, Default, Deserialize, PartialEq)]
pub struct AbilityEffect {
    pub kind: String,
    #[serde(default)]
    pub secs: [f32; 2],
    #[serde(default)]
    pub amount: f32,
    /// A root breaks on damage once it's held this long.
    #[serde(default)]
    pub break_after: f32,
    /// A snare on a foe already snared freezes it instead (Cone of Cold).
    #[serde(default)]
    pub freeze_if_snared: bool,
    /// A knockback's or a pull's arc (its upward speed; 0: the rules' own).
    #[serde(default)]
    pub lift: f32,
    /// A knockback away from (a pull toward) the caster, not where the blow came from (a burst's
    /// centre, a projectile's flight).
    #[serde(default)]
    pub from_caster: bool,
    /// Its look on whoever has it, in place of the kind's own (a spell whose state look plays).
    #[serde(default)]
    pub aura: u32,
    /// A knockback that throws them into a wall (or up against a steep step) stuns them this long
    /// as they hit it (a creature's duration: a hero's is shortened, and kept from them by the
    /// grace, as any stun's).
    #[serde(default)]
    pub wall_stun: f32,
    /// A shield that bursts as it ends (broken or run out): this ability (a `nova`) goes off round
    /// whoever held it, as theirs.
    #[serde(default)]
    pub pop: String,
}

/// The crowd control there is (Battlerite's rules, WoW's names): `stun` (nothing, damage doesn't
/// break it), `incapacitate` (nothing, damage breaks it), `petrify` (nothing, shielded: a frozen
/// foe), `panic` (runs, out of control, ended by enough damage), `root` (no moving), `snare`
/// (slower by `amount`), `silence` (no abilities), `weaken` (deals and heals `amount` less),
/// `stagger` (the attack or charge in hand is lost).
pub const CC_KINDS: [&str; 9] = ["stun", "incapacitate", "petrify", "panic", "root", "snare", "silence", "weaken", "stagger"];
/// The other effects an ability can carry.
/// `knockback` throws them `amount` yards away from where it came from (`pull`: up to `amount`
/// yards toward it); `shield` soaks `amount` damage; `regen` heals `amount` of their health a
/// second; `haste` makes them `amount` faster; `veil` hides them (creatures lose them, locks let
/// go; their next blow counts as from behind) until they strike or cast; `cheat_death` keeps a hero
/// from falling: a blow that would fell them leaves them at 1 health instead, ends it, and mends
/// `amount` of their health over the next 3 s.
pub const OTHER_EFFECTS: [&str; 15] =
    ["burn", "mark", "heal", "cleanse_panic", "deathmark", "onslaught", "hallowed", "steadfast", "knockback", "pull", "shield", "regen", "haste", "veil", "cheat_death"];
/// The effects that help whoever has them (a hero's own, or a friend's from holy ground): never
/// shortened on heroes, never stopped by the grace, never ended by a cleanse.
pub const BOONS: [&str; 8] = ["onslaught", "hallowed", "steadfast", "shield", "regen", "haste", "veil", "cheat_death"];
/// The effects that move a body (applied at once, not kept).
pub const DISPLACE: [&str; 2] = ["knockback", "pull"];
/// What a cleanse on a friend ends (Purifying Shock).
pub const HARMFUL: [&str; 11] = ["stun", "incapacitate", "petrify", "panic", "root", "snare", "silence", "weaken", "burn", "mark", "deathmark"];

/// A totem set down: the creature it is (a `[[mob]]` with `totem = true`: its look, its health)
/// and how far it works; every `tick` it gives the ability's `effects` to foes in that radius
/// (`cleanse_panic` to its owner).
#[derive(Debug, Clone, Deserialize)]
pub struct TotemDef {
    pub mob: String,
    pub radius: f32,
}

/// A counter: what it does when a melee blow or a projectile reaches the hero in its stance
/// (ground spells and channels pass it by).
#[derive(Debug, Clone, Deserialize, Default)]
pub struct CounterDef {
    /// The blow is blocked (no damage, no stamina).
    #[serde(default)]
    pub block: bool,
    /// The blow is absorbed and heals this share of the hero's health.
    #[serde(default)]
    pub absorb_heal: f32,
    /// Nothing reaches them (Ice Block); they can't act either, and the counter's key ends it.
    #[serde(default)]
    pub immune: bool,
    /// Projectiles are absorbed and fired back as this ability (Fire Ward's Fire Blast).
    #[serde(default)]
    pub reflect: String,
    /// What it does to a melee attacker (and, `any_attacker`, a shooter too): an effect each.
    #[serde(default)]
    pub attacker: Vec<AbilityEffect>,
    #[serde(default)]
    pub any_attacker: bool,
    /// Only melee blows set it off (Retaliation, Nature's Grasp); else projectiles too.
    #[serde(default)]
    pub melee_only: bool,
    /// Only projectiles set it off (Deterrence); melee blows go through.
    #[serde(default)]
    pub ranged_only: bool,
    /// It ends on the first thing it catches.
    #[serde(default)]
    pub once: bool,
    /// Every melee blow and projectile is dodged (Evasion).
    #[serde(default)]
    pub dodge: bool,
    /// A mirror: a foe's projectile that would strike them (a hero's, or a creature's bolt) flies
    /// back at whoever threw it, as hard as it was and with what it does; melee blows and ground go
    /// through as ever. Nothing else (no block, dodge, ward or `reflect` with it).
    #[serde(default)]
    pub mirror: bool,
    /// A counter that catches something comes back this much sooner (a share of its cooldown).
    #[serde(default = "half")]
    pub caught_cooldown: f32,
}

fn half() -> f32 {
    0.5
}

/// Valor: the bar and what fills it.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ValorDef {
    pub max: f32,
    /// Each life starts with this much.
    pub start: f32,
    pub light: f32,
    pub heavy: f32,
    /// An ability's hit (an ability's own `valor_hit` over this).
    pub ability: f32,
    pub parry: f32,
    /// Rolling through an attack.
    pub roll: f32,
    pub counter: f32,
    pub kill: f32,
    /// One segment of the bar (EX versions cost one or two).
    pub segment: f32,
}

impl Default for ValorDef {
    fn default() -> Self {
        Self { max: 100.0, start: 25.0, light: 2.0, heavy: 6.0, ability: 6.0, parry: 10.0, roll: 8.0, counter: 15.0, kill: 3.0, segment: 25.0 }
    }
}

/// The crowd control rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct CcDef {
    /// Heroes get this share of the duration creatures do.
    pub player_share: f32,
    /// After a stun, incapacitate or panic ends, a hero can't be given another of those for this
    /// long.
    pub grace: f32,
    /// A panic ends early once it has taken this share of the panicked one's health.
    pub panic_break: f32,
    /// A frozen (petrified) foe's ice takes this much damage before it breaks.
    pub petrify_shield: f32,
    /// A panicked hero is driven away this fast (yards a second), a stride at a time.
    pub panic_speed: f32,
    /// How each effect shows on a unit: a 1.12 spell whose state look the clients play.
    pub auras: BTreeMap<String, u32>,
}

impl Default for CcDef {
    fn default() -> Self {
        let auras = [
            ("stun", 853),
            ("incapacitate", 853),
            ("petrify", 122),
            ("panic", 5782),
            ("root", 339),
            ("snare", 116),
            ("silence", 15487),
            ("burn", 2120),
            ("deathmark", 1130),
            ("onslaught", 12328),
            ("hallowed", 465),
            ("steadfast", 10060),
            ("shield", 17),
            ("regen", 774),
            ("haste", 2983),
            ("veil", 1784),
        ]
            .into_iter()
            .map(|(k, v)| (k.to_string(), v))
            .collect();
        Self { player_share: 0.5, grace: 3.0, panic_break: 0.15, petrify_shield: 40.0, panic_speed: 5.0, auras }
    }
}

/// Dev builds: every node of the tree owned from the start.
#[derive(Debug, Clone, Deserialize, Default)]
#[serde(default)]
pub struct DevDef {
    pub full_tree: bool,
}

/// A hero's starting loadout: slot to ability key.
pub type LoadoutDef = BTreeMap<String, String>;
