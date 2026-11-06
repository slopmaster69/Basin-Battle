//! The Priest's rules (docs/priest-draft3.md): the staffs' marks and stacks (`[priest]`), and the
//! shapes of the spells that are the Priest's alone, each a table on its [[ability]].

use serde::Deserialize;

/// The Priest's staffs (`[priest]`).
///
/// **Searing Light** (the holy staff): a Smite marks the foe it strikes for `sear_secs`. Any other
/// Priest spell's blow on a marked foe, or a perfect Smite's, tears the mark out: `sear_damage`
/// more, and `sear_refund` seconds off the recharge of `gust` (Healing Gust).
///
/// **Shadow Weaving** (the shadow staff): each Shadow Bolt that strikes weaves a stack into the
/// foe, up to `weave_most`, the last held `weave_secs`; each makes the caster's staff bolts
/// `weave_step` stronger on it, and Mind Blast's `shadowed` bonus counts it. `weave_after`
/// seconds with no bolt and the next is **Empowered**: `empower_speed` times as fast,
/// `empower_range` yards farther, `empower_damage` times as hard, `empower_valor` on a hit; its
/// look `empower_aura` on the Priest while it waits.
///
/// Every Priest perfect shows `perfect_kit` (holy; never the Mage's Arcane Explosion).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct PriestDef {
    pub sear_secs: f32,
    pub sear_damage: f32,
    pub sear_refund: f32,
    pub gust: String,
    pub sear_kit: u32,
    pub weave_most: u32,
    pub weave_secs: f32,
    pub weave_step: f32,
    pub weave_after: f32,
    pub empower_speed: f32,
    pub empower_range: f32,
    pub empower_damage: f32,
    pub empower_valor: f32,
    pub empower_aura: u32,
    pub perfect_kit: u32,
}

impl Default for PriestDef {
    fn default() -> Self {
        Self {
            sear_secs: 5.0,
            sear_damage: 8.0,
            sear_refund: 3.0,
            gust: "healing_gust".into(),
            sear_kit: 0,
            weave_most: 5,
            weave_secs: 6.0,
            weave_step: 0.08,
            weave_after: 2.0,
            empower_speed: 1.5,
            empower_range: 10.0,
            empower_damage: 1.5,
            empower_valor: 4.0,
            empower_aura: 0,
            perfect_kit: 0,
        }
    }
}

/// Penance (`kind = "penance"`): up to `targets` bodies in front of the caster (within the
/// ability's `range` and `cone` degrees of the reticle, in sight) are picked, hurt friends and
/// foes taking turns; a ring closes on each in turn, the first `first` seconds after the press and
/// the rest `gap` apart, `beats` in all (round the bodies again if there are fewer). The ability's
/// key pressed within `window` of a ring closing sends a bolt of light to that body: `damage` to a
/// foe, `mend` to a friend; within the middle `[checks] crit_share` of it, `crit` instead. A ring
/// let pass is that bolt lost and the rest still come. Every ring landed: the perfect.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct PenanceDef {
    pub beats: u32,
    pub targets: u32,
    pub cone: f32,
    pub first: f32,
    pub gap: f32,
    pub window: f32,
    pub crit: f32,
}

/// Angelic Feather (`kind = "feather"`): a feather laid at the reticle (within `range`), or at the
/// caster's feet. The first hero to touch it (within `reach`, the caster or a friend) gets the
/// ability's `effects` and is freed of slows and roots. It lies `secs`, and a caster has `most`
/// down at once (the oldest goes).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct FeatherDef {
    pub reach: f32,
    pub secs: f32,
    pub most: u32,
}

/// Shadowguard (`kind = "shell"`): a shell for `secs` at most. The caster takes `cut` less and
/// moves at `pace` of their speed, and can't attack; what it stops is stored, `cap` at most. The
/// key again (or its time up) lets it go: the stored shadow flies at the first foe along the
/// reticle (the `shard`, an ability). Let go within `window` of a blow landing on them, `perfect`
/// times as much (`perfect_cap` at most).
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct ShellDef {
    pub shard: String,
    pub secs: f32,
    pub cut: f32,
    pub pace: f32,
    pub cap: f32,
    pub window: f32,
    pub perfect: f32,
    pub perfect_cap: f32,
}

/// Void Rift (an `orb` that doesn't roll): its reach grows from the ability's `radius` to
/// `grow_to` over `grow_secs`; every second it drags foes `pull` yards to its heart (a hero
/// `hero_most` yards in all). The key again implodes it: the ability's `damage` low end, `per_foe`
/// more for each foe it has caught, its high end at most; `perfect_foes` caught or more is the
/// perfect. Left `linger` seconds, it collapses for `collapse`.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct RiftDef {
    pub grow_to: f32,
    pub grow_secs: f32,
    pub pull: f32,
    pub hero_most: f32,
    pub per_foe: f32,
    pub perfect_foes: u32,
    pub collapse: f32,
}

/// The Hollow Star (a `place`d orb): it has `health` (up to `most`) and lasts the ability's
/// `secs`. Every `tick` it deals `damage` to each foe and heals each friend `heal` within its reach
/// (`reach[0]` empty, `reach[1]` full); the damage it deals fills it, the healing it gives drains
/// it. A hero standing in it gets `boon`'s effects. Foes can break it.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct HollowDef {
    pub mob: String,
    pub health: f32,
    pub most: f32,
    pub reach: [f32; 2],
    pub damage: f32,
    pub heal: f32,
    /// Its size by its health: `scale[0]` empty, `scale[1]` full.
    pub scale: [f32; 2],
}

/// The Void Tether (a held `channel` from the left click): its `damage` a second on a foe rises from
/// the low end to the high over `ramp` seconds; on a friend it heals `heal` a second, rising the
/// same; on the caster's own Hollow Star it fills it `fill` a second and draws it `drag` yards a
/// second toward them.
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct TetherDef {
    pub ramp: f32,
    pub heal: [f32; 2],
    pub fill: f32,
    pub drag: f32,
    /// A second's Valor while it holds a foe.
    #[serde(default)]
    pub valor: f32,
}
