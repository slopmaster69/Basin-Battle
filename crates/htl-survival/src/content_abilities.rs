//! The abilities (`[[ability]]` in `survival.toml`): every aimed, charged, placed or timed thing a
//! hero can do past the sword and shield, their EX versions (held a moment, or Ctrl, for Valor),
//! the crowd control rules (`[cc]`) and Valor itself (`[valor]`), and the board they're chosen on:
//! five rows (one per key), a card of each class in each, and a second page of beta cards (up to
//! four a row) that any class can use.

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
/// The board's second page: generic cards any class can own and slot (each on its own row's key),
/// free to take from the board, there to be tried out. Not a class's column: a card's `school`.
pub const BETA: &str = "beta";
/// At most this many beta cards to a row (the page is five rows of four).
pub const BETA_ROW: usize = 4;
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
pub const KINDS: [&str; 21] = [
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
    // A jet `arc` wide sweeping `sweep` degrees across the front over `secs`, striking each foe
    // it passes once; the key again stops it where it is (`stop_secs`, `stop_effects`), a foe
    // within `perfect_arc` of its centre a perfect (the stop check).
    "sweep",
    // An orb rolling out along the reticle, hurting what it passes; the key again stops it where
    // it is and it bursts, a foe under it a perfect (the stop check).
    "orb",
    // Beats: the first few by themselves, then one more for every press on the beat, up to its
    // most; a press out of time or a beat let pass ends it (the rhythm check).
    "rhythm",
    // A bolt that plants a bomb on what it hits; it flares, then bursts; its key again on a flare
    // sets it off early and harder (the release check on a foe).
    "bomb",
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
    /// A release check: the charge flares as it tops out, and a release inside `[checks]`'s
    /// window of the top is a perfect cast (stronger, Valor, a burst everyone sees).
    #[serde(default)]
    pub perfect: bool,
    /// What a missed check costs: "" (safe: it still goes, weaker) or "all" (all or nothing:
    /// anything but a perfect fizzles, the Valor spent and half the cooldown with it).
    #[serde(default)]
    pub stakes: String,
    /// A critical (the very middle of a check's window): these effects on what it strikes, on
    /// top of the perfect. Each spell its own: a longer burn, a freeze, a silence.
    #[serde(default)]
    pub crit_effects: Vec<AbilityEffect>,
    /// This ability's own burst for a perfect (a spell visual kit), in place of `[checks]`'s:
    /// the spell's school bursting from the hands, not a shield on the caster.
    #[serde(default)]
    pub perfect_kit: u32,
    /// A channel's look on the caster while it lasts (a sweep's: Cone of Cold's spray from the
    /// hands), replayed every `CHANNEL_EVERY` sec.
    #[serde(default)]
    pub channel_kit: u32,
    /// An orb's own rolling model, drawn by the clients that can (`PlayGroundOrb`), and its size.
    #[serde(default)]
    pub orb_model: String,
    #[serde(default)]
    pub orb_scale: f32,
    /// A rhythm's casts, beat by beat (cycled): the body's clip for each missile.
    #[serde(default)]
    pub beat_clips: Vec<u16>,
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
    /// Its shape flashed on the ground as it lands (a cone's wedge, a nova's ring), on the clients
    /// that draw one (patch 0026's `PlayGroundBurst`): an effect, not a floor marker.
    #[serde(default)]
    pub burst: Option<BurstDef>,
    /// Ground: seconds between the mark and the landing; then burning ground this long, at this
    /// much a second.
    #[serde(default)]
    pub delay: f32,
    /// Ground: how long its look takes to come down (a meteor's fall is in the effect's own
    /// model, from the moment it's shown): shown that much before it lands, so the blow and the
    /// look land together.
    #[serde(default)]
    pub look_lead: f32,
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
    /// One of the Hunter's bow abilities: the bow comes into the hands as it's aimed (the hero's
    /// own, or the camp's first bow if they carry none), it's drawn with the base game's draw
    /// while it charges (or, with nothing to charge, while it's aimed), and it looses with the
    /// bow's own release before the melee weapon comes back. A leap with the bow (Skyfall) holds
    /// it through the leap.
    #[serde(default)]
    pub bow: bool,
    /// A bow ability's damage as a share of the hero's bow's at the same draw (Aimed Shot's 1.2:
    /// 120% of what the bow would do), so it grows with the bow as a weapon art grows with the
    /// weapon. 0: its own `damage`. (Serg's play-test, #118.)
    #[serde(default)]
    pub bow_damage: f32,
    /// A fan (`count` > 1) whose arrows pass this far apart (yd) where the reticle points, so a
    /// pack there is covered near or far; `spread` is then the widest it opens (close in). 0: a
    /// fixed `spread`. (Multi-Shot: 10° apart left the arrows passing between a pack past 15 yd;
    /// Serg's play-test, #118.)
    #[serde(default)]
    pub spread_yd: f32,
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
    /// A wind-up on the caster themself while a charge is held (Kill Shot's draw): this spell's
    /// aura look in their aura slots from the press to the release or the let-go (0: none).
    #[serde(default)]
    pub charge_look: u32,
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
    /// A piercing shot's weight on each foe it goes through (Kill Shot): this kit and this
    /// `SoundEntries` id on each, besides `impact_kit`, and the heavy blow's reel (0: none).
    #[serde(default)]
    pub through_kit: u32,
    #[serde(default)]
    pub through_sound: u32,
    /// A kill with it: this `SoundEntries` id from the foe's spot, for everyone near (0: none).
    #[serde(default)]
    pub kill_sound: u32,
    /// The caster's own screen as it goes, and as it kills: the add-on's beats of these names
    /// (`BT` lines; empty: none).
    #[serde(default)]
    pub beat: String,
    #[serde(default)]
    pub kill_beat: String,
    /// A trap's look while it's set (`GameObjectDisplayInfo`: 3072 an Explosive Trap, 3071 a
    /// Freezing Trap), and the spell shown at its spot as it goes off (its ground one-shot, cast
    /// from the spot by an unseen stand-in, so no caster-side look plays on the hunter; what it
    /// does to each one it catches is the `impact_kit`).
    #[serde(default)]
    pub object: u32,
    #[serde(default)]
    pub burst_spell: u32,
    /// A ricochet's flash where it skips off the ground, apart from the wall's `burst_spell`
    /// (Bank Shot: not fire on the floor; Corey's review). 0: none, the arrow flies on from there.
    #[serde(default)]
    pub skip_spell: u32,
    /// A trap's throw: the spell whose missile flies from the hand to the spot before the trap
    /// is set there (a bomb's), so the player sees it go out (Corey's review: "the trap should
    /// fly out of the hand and land on the spot"). 0: set at once.
    #[serde(default)]
    pub lob_spell: u32,
    /// The look its lingering ground wears (a DynamicObject of this spell for as long as it
    /// lingers: Consecration's holy ground, Flamestrike's fire); 0: its own `spell`.
    #[serde(default)]
    pub linger_spell: u32,
    #[serde(default)]
    pub ammo: u32,
    #[serde(default)]
    pub aura: u32,
    /// The cast's clip (`AnimationData`), played on the caster as it goes (a weapon art's: its
    /// blow).
    #[serde(default)]
    pub clip: u16,
    /// Which of the model's versions of `clip` (0 the first).
    #[serde(default)]
    pub take: u32,
    /// Another character model whose take on `clip` every hero plays (`"TaurenMale"`; as a
    /// move's `from`), so the ability looks the same on every race; without one, the hero's own.
    #[serde(default)]
    pub from: Option<String>,
    /// A leap's own look, the same on every hero whatever their race or how they were moving: a
    /// clip over the whole body for the flight (`leap_clip`, its `leap_take`), borrowed from
    /// another character model (`leap_from`, as `"ScourgeFemale"`; empty: their own), and a
    /// tumble head over heels along the way it flies (`leap_flip`), on the clients that play
    /// clips. In place of the spell's own cast pose.
    #[serde(default)]
    pub leap_clip: u16,
    #[serde(default)]
    pub leap_take: u32,
    #[serde(default)]
    pub leap_from: String,
    #[serde(default)]
    pub leap_flip: bool,
    /// A pull's look on whoever casts it, for the whole flight of what it hauls (Reverse Grapple:
    /// a pull, not the bow's release pose): a clip (`pull_clip`), borrowed from another character
    /// model (`pull_from`, as `"ScourgeFemale"`; empty: their own), on the clients that play
    /// clips. Only on an ability with a `pull` effect.
    #[serde(default)]
    pub pull_clip: u16,
    /// Or one of these clips, picked at random each use (Reverse Grapple's five pulls, so the
    /// haul doesn't look the same every time); when set, it takes the place of `pull_clip`.
    #[serde(default)]
    pub pull_clips: Vec<u16>,
    #[serde(default)]
    pub pull_from: String,
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
    /// With a two-handed weapon (no shield to throw), its key fires this ability instead.
    #[serde(default)]
    pub two_hand: String,
    /// Only with a one-handed weapon and its shield (Shield Wall): with a two-hander it won't go.
    #[serde(default)]
    pub needs_shield: bool,
    /// An ally spell that strikes a foe instead (its `damage`) when one is on the reticle's line
    /// (Holy Shock).
    #[serde(default)]
    pub foe_damage: bool,
    /// Once given, the same hero can't have it again for this long (Lay on Hands).
    #[serde(default)]
    pub lockout: f32,
    /// Its leaps go to a foe that's casting first (Avenger's Shield), else the nearest.
    #[serde(default)]
    pub chain_casters: bool,
    /// A returning projectile: its key again while it flies out calls it back from where it is.
    #[serde(default)]
    pub recall: bool,
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
    /// A bomb's burst, yards (`kind = "bomb"`); its bolt flies with `radius` like any projectile.
    #[serde(default)]
    pub bomb_radius: f32,
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
    /// Let go at full charge, it goes on through this many instead (Aimed Shot's full draw).
    #[serde(default)]
    pub full_pierce: u32,
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
    /// A leap: no ring on the ground where it comes down, for anyone (a rival under it still
    /// gets the warning over them).
    #[serde(default)]
    pub unmarked: bool,
    /// A dive (Stoop): a leap only from the air (landing first closes its recast), down onto the
    /// foe thrown up with the hero, fast however high it starts, its damage from `damage[0]` near
    /// the ground to `damage[1]` from high up.
    #[serde(default)]
    pub dive: bool,
    /// A blink with an anchor: the first press sets it at the feet (and costs nothing); pressed
    /// again within this many seconds, the hero steps back to it.
    #[serde(default)]
    pub anchor: f32,
    /// Alter Time (the rhythm check, on an anchor): the mark pulses every this many seconds
    /// (`tick_spell` at it, `beat_kit` on the hero); the step back within `[checks] window` of a
    /// pulse is a perfect: foes within `radius` get `stop_effects` (the throw back), and the
    /// perfect's Valor and look.
    #[serde(default)]
    pub pulse: f32,
    /// The step back to the anchor gives back this share of the health lost since it was set.
    #[serde(default)]
    pub rewind_health: f32,
    /// A blink that slips a blow (the counter check, Blink): a creature's or a hero's blow that
    /// would have caught the hero where they blinked from, landing within `[checks] window` of
    /// the blink (their round trip allowed), misses. The first one pays: the cooldown halved,
    /// a counter's Valor and the perfect's burst.
    #[serde(default)]
    pub slip: bool,
    /// Another ability fired as well, at once, from the same aim (a Disengage's Net).
    #[serde(default)]
    pub also: String,
    /// Effects on the caster as it goes (a leap's haste, a cast that shields you).
    #[serde(default)]
    pub self_effects: Vec<AbilityEffect>,
    /// Lingering ground's effects on each foe inside, every tick (a vortex's pull, a storm's snare);
    /// the boons among them go to each friend inside instead, held as its `boon` is.
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
    /// A sweep (`kind = "sweep"`, the stop check): a jet `arc` wide and `range` long that
    /// sweeps `sweep` degrees across the hero's front over `secs`, striking each foe it passes
    /// once (`damage`, `effects`). The key again stops it: the jet holds for `stop_secs` and the
    /// foes in it take `stop_effects` (a freeze); a foe within `perfect_arc` of the jet's centre
    /// makes the stop a perfect (`[checks]`). Not stopped, it finishes its sweep.
    #[serde(default)]
    pub sweep: f32,
    #[serde(default)]
    pub stop_secs: f32,
    #[serde(default)]
    pub stop_effects: Vec<AbilityEffect>,
    #[serde(default)]
    pub perfect_arc: f32,
    /// A rhythm (`kind = "rhythm"`, the rhythm check): it goes off on beats `beat_secs` seconds
    /// apart, the first `free_beats` by themselves, then each further one only if its key is
    /// pressed within its window of the beat (`beat_window`: the first's, tightening to the
    /// last's), up to `most_beats`. A press out of time, or a beat let pass, ends it where it is.
    /// Each beat is `beat_kind`: a `projectile` along the reticle or a `nova` round the caster
    /// (its `radius` growing from the low end to the high over the beats), for `damage` plus
    /// `beat_step` a beat; the last beat is the finisher, `finisher_damage` times as hard and
    /// with `finisher_effects`. A full chain is a perfect.
    #[serde(default)]
    pub beat_secs: f32,
    #[serde(default)]
    pub free_beats: u32,
    #[serde(default)]
    pub most_beats: u32,
    #[serde(default)]
    pub beat_window: [f32; 2],
    #[serde(default)]
    pub beat_step: f32,
    #[serde(default)]
    pub beat_kind: String,
    #[serde(default)]
    pub finisher_damage: f32,
    #[serde(default)]
    pub finisher_effects: Vec<AbilityEffect>,
    /// The pulse on the caster on every beat to come (a spell visual kit): the beat to press on.
    #[serde(default)]
    pub beat_kit: u32,
    /// A bomb (`kind = "bomb"`, the release check on a foe): a bolt that plants it on what it
    /// hits; it flares `bomb_flares` times over `bomb_secs`, then bursts for `damage`'s low end in
    /// `radius` with `bomb_effects`. The key again on a flare (within `[checks] window`) sets it
    /// off early: the high end, `stop_effects` too, a perfect; off a flare, it goes at once,
    /// plain.
    #[serde(default)]
    pub bomb_secs: f32,
    #[serde(default)]
    pub bomb_flares: u32,
    #[serde(default)]
    pub bomb_effects: Vec<AbilityEffect>,
    /// The pulse on the bombed foe at each flare (a spell visual kit).
    #[serde(default)]
    pub bomb_kit: u32,
    /// Combustion: what burns on the foe this reaches catches on every foe within this many yards.
    #[serde(default)]
    pub spread_burns: f32,
    /// An orb (`kind = "orb"`, the stop check): it rolls out along the reticle `range` yards
    /// over `linger` seconds, `linger_damage` a second to every foe within `radius` (with the
    /// ability's `effects`), every `tick`. The key again stops it where it is and it bursts:
    /// `damage` (the low end; the high end on a perfect) in `stop_radius`; a foe within
    /// `perfect_yd` of it makes the stop a perfect and the burst carries `stop_effects` (the
    /// freeze). Stopped within `backfire_yd` of the caster, `backfire_effects` fall on them.
    /// Rolled the whole way, it fades with no burst.
    #[serde(default)]
    pub stop_radius: f32,
    #[serde(default)]
    pub perfect_yd: f32,
    #[serde(default)]
    pub backfire_yd: f32,
    #[serde(default)]
    pub backfire_effects: Vec<AbilityEffect>,
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
    /// The colour (`"RRGGBB"`) its missile's ribbon takes as it flies and after each ricochet, one
    /// a bounce, the last kept past them (Bank Shot's arrow hotter each time it bounces: Corey's
    /// review). Drawn on a client with patch 0030, over the spell's own `[[missile_look]]`.
    #[serde(default)]
    pub bounce_trail: Vec<String>,
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
    /// Which of its class's two choices in its row it is (1 the first, 2 the second): the board
    /// shows the second under the first, and a hero takes one of the two onto the key.
    #[serde(default = "first_choice")]
    pub choice: u32,
    /// The Hunter's arrows: one that ends its flight sticks in the ground where it stops (at its
    /// range's end, at the wall it meets, a yard past the foe that stopped it), for Arrow Recall.
    #[serde(default)]
    pub plants: bool,
    /// A Hunter ability's own mechanic, beyond its kind's: `call` and `double_call` (Call of the
    /// Wild's beasts), `tripwire`, `recall` (Arrow Recall), `haul` (a pull that hauls its caster
    /// to what won't move), `scatter` (arrows scattered where a leap began), `volley` (Skyfall's),
    /// `float` (a leap that hangs at its top, aiming its `recast`, then floats down).
    #[serde(default)]
    pub hunt: String,
    /// A projectile that ricochets off walls, trees and rocks this many times, each bounce adding
    /// `bounce_gain` of its damage (Bank Shot).
    #[serde(default)]
    pub bounces: u32,
    #[serde(default)]
    pub bounce_gain: f32,
    /// A projectile that bursts where its flight ends (the first foe, a wall, the ground or its
    /// range's end) into this `inner` projectile's arrows, fanned out from there (Split Arrow);
    /// its `burst_spell` shows at the burst.
    #[serde(default)]
    pub splits: String,
    /// It goes on the key press, along the facing (no aim and click): Raptor Strike's hits,
    /// Skyfall's leap.
    #[serde(default)]
    pub at_once: bool,
    /// The Warlord's (the Warrior's second choices).
    /// A dash that carries the first body it meets to the end of its rush and Crashes it into
    /// what's there (Tackle): a wall, another body, or nothing (dropped for its `damage`).
    #[serde(default)]
    pub crash: Option<CrashDef>,
    /// An `ally` that rushes to the friend it picks (up to its `range`) and takes the first blow
    /// meant for them within this many seconds (Intervene).
    #[serde(default)]
    pub intervene: f32,
    /// A nova's friends in it (the caster too): this much stamina back, these boons, and this
    /// spell's look on each of them (Rallying Cry).
    #[serde(default)]
    pub ally_stamina: f32,
    #[serde(default)]
    pub ally_effects: Vec<AbilityEffect>,
    #[serde(default)]
    pub ally_spell: u32,
    /// A beam all round the caster, `radius` out, not a strip along the aim (Bladestorm's spin);
    /// it walks at [`AROUND_PACE`] of a run.
    #[serde(default)]
    pub around: bool,
    /// A beam whose every strike after the first deals this much more, up to `ramp_cap` a strike
    /// (Bladestorm's ramp).
    #[serde(default)]
    pub ramp: f32,
    #[serde(default)]
    pub ramp_cap: f32,
    /// A beam the clients that play clips show as a storm of cuts in turning directions
    /// (Bladestorm's flurry), over the spell's own spin.
    #[serde(default)]
    pub flurry: Option<FlurryDef>,
    /// A channel the guard ends at once, raising the guard on that press (a dodge ends any
    /// channel and comes out on its press already): Bladestorm.
    #[serde(default)]
    pub bail: bool,
    /// Against an opened enemy (staggered, knocked down, stunned, Exposed, or a boss
    /// stance-broken) it deals this many times its damage instead (Cleave's 250%: `weapon = 1.0`,
    /// `opened = 2.5`).
    #[serde(default)]
    pub opened: f32,
    /// A `place` that plants a banner at the reticle (War Banner): a thing on the ground (its
    /// `object`, a `GameObjectDisplayInfo`) standing `secs`, with health, that helps the heroes
    /// near it. Its recast moves it.
    #[serde(default)]
    pub banner: Option<BannerDef>,
    /// A buff that goes even while its caster is staggered or knocked down, and ends that, a root
    /// and a slow (Berserker Rage).
    #[serde(default)]
    pub breaks_free: bool,
    /// A buff that costs this share of the caster's health now as it goes (never the last point
    /// of it: Blood Fury).
    #[serde(default)]
    pub health_cost: f32,
    /// A passive the hero has while this ability is on one of their keys (one of [`PASSIVES`]):
    /// `momentum`, the Juggernaut's.
    #[serde(default)]
    pub passive: String,
}

/// The passives an ability on a key can give its hero.
pub const PASSIVES: [&str; 1] = ["momentum"];

/// A War Banner as it stands: its `health` (every blow that reaches it, a foe's or a hero's,
/// takes from it), and what heroes within its ability's `radius` of it get: stamina back `regen`
/// times as fast, heavy attacks that stagger `stagger` longer, and `aura`'s look over them while
/// they're near. Drawn at `scale`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BannerDef {
    pub health: f32,
    pub regen: f32,
    pub stagger: f32,
    #[serde(default)]
    pub aura: u32,
    #[serde(default = "full_share")]
    pub scale: f32,
}

/// The Juggernaut's Momentum (`[momentum]`): each of the hero's own blows that lands (a light, a
/// heavy, a weapon art) adds a stack, `damage` more a stack on their blows, `most` stacks at
/// most; all lost when they raise their guard, or `secs` after their last hit.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MomentumDef {
    pub damage: f32,
    pub most: u32,
    pub secs: f32,
}

impl Default for MomentumDef {
    fn default() -> Self {
        Self { damage: 0.05, most: 5, secs: 4.0 }
    }
}

/// An around beam (Bladestorm) walks at this share of a run.
pub const AROUND_PACE: f32 = 0.7;

/// Tackle's Crash: what the carried body takes where the rush ends. Into a wall, a tree or a
/// rock: `wall_damage` and stunned `wall_stun` (a hero's shortened as any stun's). Into another
/// enemy: both take `foe_damage` and are knocked down `knockdown`. A boss (anything that won't
/// move) in the way: the rush stops against it for `posture`; a body carried into one gives it
/// `boss_posture`. A hero is carried at most `hero_carry` yd.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct CrashDef {
    pub wall_damage: f32,
    pub wall_stun: f32,
    pub foe_damage: f32,
    pub knockdown: f32,
    pub posture: u32,
    pub boss_posture: u32,
    pub hero_carry: f32,
}

fn first_choice() -> u32 {
    1
}

/// A shape flashed on the ground as an ability lands: a shock front runs out to the edge over most
/// of `secs` and fades, in `colour` (`RRGGBB`).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct BurstDef {
    pub secs: f32,
    pub colour: String,
}

/// Bladestorm's flurry, as the client plays it (patch 0025's `PlayUnitFlurry`): `cuts` the clips
/// and takes it cycles through, from the character model `from` names (the same on every race),
/// each `cut` seconds long, the first wound up over `windup` (the anticipation: the draw-back,
/// compact); the body snaps round `turn` degrees a step to face a new way for each; each trails
/// the weapons in `trail` (`AARRGGBB`) for `trail_ms`. When the spin ends, one wider sweep
/// (`finish`, over `finish_secs`, its trail hanging longer) as the body comes back to face ahead.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FlurryDef {
    pub from: Option<String>,
    pub cuts: Vec<[u32; 2]>,
    pub cut: f32,
    pub windup: f32,
    pub finish: [u32; 2],
    pub finish_secs: f32,
    pub turn: f32,
    #[serde(default)]
    pub trail: String,
    #[serde(default)]
    pub trail_ms: u32,
}

impl FlurryDef {
    /// Clips and takes as the client's call reads them: `"88/0,87/0"`.
    pub fn cuts_text(&self) -> String {
        self.cuts.iter().map(|[c, t]| format!("{c}/{t}")).collect::<Vec<_>>().join(",")
    }
}

/// The Hunter's own mechanics an ability can name (`hunt`).
pub const HUNTS: [&str; 8] = ["call", "double_call", "tripwire", "recall", "haul", "scatter", "volley", "float"];

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
            && !self.at_once
            && !self.at_self
            && !(self.kind == "leap" && self.back)
            && self.anchor <= 0.0
            && !(self.recast_to_start && matches!(self.kind.as_str(), "blink" | "leap"))
    }
    /// On the board (a card to choose), not off it or only fired by something else.
    pub fn on_board(&self) -> bool {
        !self.is_ex() && !self.inner && !self.bench
    }
    /// A card of the beta page ([`BETA`]): any class's, not one class's column.
    pub fn is_beta(&self) -> bool {
        self.school == BETA
    }
    /// The board's page it's on: 1 the classes' columns, 2 the beta page.
    pub fn page(&self) -> u8 {
        if self.is_beta() {
            2
        } else {
            1
        }
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
    /// A one-shot look (a `SpellVisualKit`) replayed on whoever has it every `mark_every` seconds
    /// while it holds, where the data has no lasting look that fits: the Cowed mark over the head,
    /// the Exposed crack. A kit plays no body animation, as a spell's cast would.
    #[serde(default)]
    pub mark: u32,
    #[serde(default)]
    pub mark_every: f32,
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
    /// Takes on a foe that holds its ground (a champion, a totem, a creature winding up a blow no
    /// guard stops): an ultimate's effects do, set as the rules load; nothing else's.
    #[serde(default)]
    pub vs_bosses: bool,
}

/// Crowd control and forced movement a foe that holds its ground shrugs off (an ultimate's
/// aside): what takes away its control of itself or its place. A snare, a weakness, a burn and a
/// mark still take.
pub const CONTROL: [&str; 7] = ["stun", "incapacitate", "petrify", "panic", "root", "silence", "stagger"];

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
///
/// The Warrior's: `guarded` (the first blow meant for them strikes the one who gave it instead:
/// Intervene), `rallied` (any stagger or knockdown ends as it lands; nothing staggers them, and
/// their attacks come `amount` faster: Rallying Cry), `fury` (their attacks cost no stamina and
/// deal `amount` more: Blood Fury, Rallying Cry), `grown` (drawn `amount` larger), `cowed` (they
/// deal `amount` less damage and wind their attacks up slower; a creature low on health flees: a
/// boss only deals a little less: Intimidating Roar), `exposed` (no block or parry, and every
/// hero's hit on them deals `amount` more and staggers longer: Rising Cut), `momentum` (the
/// Juggernaut's stacks, `amount` of them), `war_banner` (near a War Banner: kept on for a moment
/// at a time while they stand by it).
pub const OTHER_EFFECTS: [&str; 27] = [
    "knockdown", "burn", "mark", "heal", "cleanse_panic", "deathmark", "sunder", "freedom", "steed", "wrath", "hallowed", "steadfast", "knockback", "pull", "shield", "regen", "haste", "veil", "cheat_death", "guarded", "rallied", "fury", "grown", "cowed", "exposed", "momentum", "war_banner",
];
/// The effects that help whoever has them (a hero's own, or a friend's from holy ground): never
/// shortened on heroes, never stopped by the grace, never ended by a cleanse.
pub const BOONS: [&str; 16] = ["freedom", "steed", "wrath", "hallowed", "steadfast", "shield", "regen", "haste", "veil", "cheat_death", "guarded", "rallied", "fury", "grown", "momentum", "war_banner"];
/// The effects that move a body (applied at once, not kept).
pub const DISPLACE: [&str; 2] = ["knockback", "pull"];
/// What a cleanse on a friend ends (Purifying Shock).
pub const HARMFUL: [&str; 14] = ["stun", "incapacitate", "petrify", "panic", "root", "snare", "silence", "weaken", "burn", "mark", "deathmark", "sunder", "cowed", "exposed"];

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
    /// The counter check on an immune stance (Ice Block): broken out of within `[checks] window`
    /// of a blow landing on it (the hero's round trip allowed), it bursts: these effects on the
    /// one who struck it, and the perfect's burst.
    #[serde(default)]
    pub perfect_break: Vec<AbilityEffect>,
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
    /// The counter check on a mirror (Spell Mirror): only what reaches the hero within
    /// `[checks] window` of raising it (plus their round trip) flies back, a perfect; later in the
    /// stance it's only swallowed.
    #[serde(default)]
    pub perfect_mirror: bool,
    /// A counter that catches something comes back this much sooner (a share of its cooldown).
    #[serde(default = "half")]
    pub caught_cooldown: f32,
    /// And one that catches nothing (Retaliation: a misread costs little).
    #[serde(default = "whole")]
    pub missed_cooldown: f32,
    /// The hero's own attack, guard, dodge, jump or another ability ends it at once.
    #[serde(default)]
    pub ends_on_action: bool,
    /// What it strikes back with when it catches a melee blow: this ability (a cone) from the
    /// hero, along their facing (Retaliation's sweep).
    #[serde(default)]
    pub answer: String,
    /// Only blows and projectiles from in front, within this arc (degrees; 0: from anywhere).
    #[serde(default)]
    pub front: f32,
    /// The guard's pose held while it lasts (Shield Wall: the shield up).
    #[serde(default)]
    pub guard_pose: bool,
    /// What goes off when it runs out having caught nothing (Trapper's Instinct: the plain
    /// Ensnaring Retreat), so a read that wasn't needed still does something.
    #[serde(default)]
    pub expire_answer: String,
    /// A melee blow it catches is parried (as a timed guard would: the attacker reels, the hero
    /// gets a parry's stamina and opening) and the hero answers with their own heavy attack at
    /// once, free, untouchable until it has struck (Retaliation).
    #[serde(default)]
    pub parry_heavy: bool,
    /// While it lasts, a left click with the guard held (or the bash key) shoves with the shield,
    /// harder, and the stance holds on after it (Shield Wall's Shield Bash).
    #[serde(default)]
    pub bash: bool,
}

fn whole() -> f32 {
    1.0
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
            // Death Wish's red: Blood Fury, and the Warrior's own Rallying Cry.
            ("fury", 12328),
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
