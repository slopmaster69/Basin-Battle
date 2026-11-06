//! How an outcome shows without a word (Corey's rule: no text explains a spell or a combat
//! outcome; its look and its sound do), and the hero's own voice on a refusal.
//!
//! An outcome is a spell visual kit on the unit it happened to and a sound from there (everyone
//! near sees and hears both), and a beat for the hero whose outcome it is (`CU` to their add-on:
//! a hit-stop, a shake, a kick of the camera).
//!
//! A refusal (out of range, not ready, too winded) keeps its words and is said too, in the hero's
//! race and sex voice, as 1.12 says its own ("Out of range.", "That ability isn't ready yet."):
//! that voice's `SoundEntries.dbc` kit, played to that hero alone.
//!
//! The sound ids are 1.12.1's `SoundEntries.dbc` (build 5875), each checked by its name in a
//! dump of that table (the `sound_entries` rows of a 1.12 MaNGOS world database). The kits are
//! the ones the repo already plays, named where they're defined. Nobody has seen or heard any of
//! these in the game yet: each says what it should be, and the ones marked "check on video" are
//! picks by name whose look or sound on screen nobody here knows for certain.

use super::{Event, Id, Survival};

// --- the looks (`SpellVisualKit.dbc`) -----------------------------------------------------------

/// Frostbolt's flare as its charge tops out (`full_kit` in content): a burst of frost, on a
/// frozen foe as its ice shatters. A caster's flare used as an impact: check on video.
pub const KIT_FROST_BURST: u32 = 450;

// --- the sounds (`SoundEntries.dbc`, by id: its name) ---------------------------------------------

/// `HeavyWeaponCritical`: a critical's heavy, wet hit (a marked foe struck for the crit).
pub const SOUND_CRIT: u32 = 238;
/// `FrostNovaStateEnd`: Frost Nova's ice breaking off a foe, as the ice shatters. Check on video.
pub const SOUND_SHATTER: u32 = 3182;
/// `ExecuteTarget`: the Warrior's Execute landing on its target.
pub const SOUND_EXECUTE: u32 = 3133;
/// `Dagger1H_ArmorFleshCritical`: a dagger's critical into flesh, for a blow in the back.
pub const SOUND_BACKSTAB: u32 = 1152;
/// `Mace2H_ArmorMetalShieldCritical`: a two-handed mace's critical on a metal shield, for a
/// champion's stance breaking. Check on video.
pub const SOUND_BREAK: u32 = 958;
/// `Shield Metal Impact`: a guard taking a blow too heavy to turn aside (a giant's: no parry, only
/// a block). Check on video.
pub const SOUND_TOO_HEAVY: u32 = 3263;
/// `BoulderImpactSound`: a body thrown into a wall. Check on video.
pub const SOUND_WALL: u32 = 7674;
/// `SoulstoneRessurection_Base`: a Soulstone's rising, as a killing blow is cheated.
pub const SOUND_CHEAT_DEATH: u32 = 7354;
/// `Chains of Ice Impact`: a chain's yank, a tether dragging a foe back. Check on video.
pub const SOUND_TETHER: u32 = 1502;
/// `Clearcasting_Impact_Chest`: the Clearcasting proc's chime, the Valor of a killing ability
/// given back.
pub const SOUND_VALOR_BACK: u32 = 4874;
/// `ColdBlood`: Cold Blood's sting, as a Death Mark's kill readies every cooldown. Check on video.
pub const SOUND_READIED: u32 = 6774;
/// `(DONOTRENAME)AbsorbGetHit`: a shield swallowing a blow, a grounding totem taking a missile.
/// Check on video.
pub const SOUND_ABSORB: u32 = 3334;
/// `LOOTWINDOWCOINSOUND`: coins taken from loot, gold earned.
pub const SOUND_COINS: u32 = 120;

// --- the beats (the hero's own camera, by the add-on: `CU\t<beat>`) ----------------------------------

pub const BEAT_CRIT: &str = "crit";
pub const BEAT_SHATTER: &str = "shatter";
pub const BEAT_EXECUTE: &str = "execute";
pub const BEAT_BACKSTAB: &str = "backstab";
pub const BEAT_BREAK: &str = "break";
pub const BEAT_TOO_HEAVY: &str = "too_heavy";
pub const BEAT_WALL: &str = "wall";
pub const BEAT_CHEAT_DEATH: &str = "cheat_death";
pub const BEAT_READIED: &str = "readied";
pub const BEAT_VALOR_BACK: &str = "valor";
/// Thrown up by a blow's leap (Raptor Strike's Uppercut): the camera in and tipped down on the
/// fight, to aim the dive. A camera beat, no stop and no shake.
pub const BEAT_RISE: &str = "rise";
/// Raised on Salvation's wings: the camera up with the Priest, looking down on the field.
pub const BEAT_SALVATION: &str = "salvation";

// --- the hero's voice on a refusal --------------------------------------------------------------------

/// Which of 1.12's error lines a refusal is said with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Vocal {
    /// `CantCastOutofRange`: "Out of range."
    OutOfRange,
    /// `CantCastGenericNoTarget`: "I have no target."
    NoTarget,
    /// `InvalidTarget`: "Invalid target."
    InvalidTarget,
    /// `CantUseAbilityCooling`: "That ability isn't ready yet."
    NotReady,
    /// `CantUseGeneric`: "I can't do that."
    CantDo,
    /// `NoEnergy`: "Not enough energy.", for stamina.
    NoStamina,
    /// `NoAmmo`: "I have no ammo.", for arrows.
    NoArrows,
}

/// Each race's lines (`ChrRaces` 1..=8: Human, Orc, Dwarf, Night Elf, Undead, Tauren, Gnome,
/// Troll), a man's then a woman's, in [`Vocal`]'s order. Each id is that voice's own kit, by name
/// (`<Race><Sex>_<line>`, as `VocalUISounds.dbc` points the client's own refusals at them).
const VOCALS: [[[u32; 7]; 2]; 8] = [
    [[1913, 1912, 1882, 1917, 1918, 2627, 2618], [2037, 2036, 2006, 2041, 2042, 2564, 2581]],
    [[2324, 2323, 2291, 2328, 2329, 2954, 2945], [2379, 2378, 2348, 2383, 2384, 2971, 2962]],
    [[1633, 1626, 1589, 1607, 1653, 2573, 2601], [1691, 1690, 1661, 1695, 1696, 2899, 2889]],
    [[2156, 2155, 2125, 2160, 2161, 2649, 2640], [2267, 2266, 2236, 2271, 2272, 2666, 2657]],
    [[2101, 2100, 2062, 2105, 2106, 2638, 2629], [2212, 2211, 2180, 2216, 2217, 2570, 2593]],
    [[2472, 2471, 2411, 2480, 2483, 2567, 2588], [2473, 2470, 2410, 2481, 2482, 2923, 2914]],
    [[1748, 1747, 1716, 1752, 1753, 2887, 2877], [1803, 1802, 1771, 1807, 1808, 2912, 2903]],
    [[1858, 1857, 1827, 1862, 1863, 2577, 2609], [1968, 1967, 1937, 1986, 1987, 2982, 2973]],
];

/// The kit a hero of `race` and `gender` (0 a man, 1 a woman) says `line` with; none for a race
/// past the eight.
pub fn vocal_sound(race: u8, gender: u8, line: Vocal) -> Option<u32> {
    let r = VOCALS.get(usize::from(race).checked_sub(1)?)?;
    Some(r[usize::from(gender.min(1))][line as usize])
}

/// A refusal's voice no oftener than this (seconds), so a key mashed on its cooldown says it
/// once, not over itself.
const VOCAL_EVERY: f32 = 0.9;

impl Survival {
    /// An outcome on `on`: its look and its sound there, and the beat on `hero`'s screen (whose
    /// outcome it is). 0 or "" for what it hasn't.
    pub(crate) fn cue(&mut self, hero: Option<Id>, on: Id, kit: u32, sound: u32, beat: &'static str) {
        if kit != 0 {
            self.events.push(Event::Visual { id: on, kit, by: None });
        }
        if sound != 0 {
            self.events.push(Event::Sound { id: on, sound });
        }
        if let Some(h) = hero.filter(|h| !beat.is_empty() && self.players.contains_key(h)) {
            self.events.push(Event::Beat { player: h, beat: beat.to_string(), on: Some(on) });
        }
    }

    /// A refusal: its words on the hero's screen, and said in their own voice.
    pub(crate) fn refuse(&mut self, id: Id, text: impl Into<String>, line: Vocal) {
        self.events.push(Event::Flash { player: id, text: text.into() });
        self.say_vocal(id, line);
    }

    /// Whether a hero hears their own voice on a refusal: the game's Error Speech setting, as
    /// their add-on reports it (on until it says otherwise).
    pub fn set_error_speech(&mut self, id: Id, on: bool) {
        if on {
            self.no_error_speech.remove(&id);
        } else {
            self.no_error_speech.insert(id);
        }
    }

    /// The hero says `line` (to themselves: no one else hears it), unless they just said one.
    pub(crate) fn say_vocal(&mut self, id: Id, line: Vocal) {
        let Some(look) = self.players.get(&id).map(|p| p.look) else { return };
        // The game's own Error Speech switch is theirs to turn off (Corey: never forced on
        // anyone), and turns this off with it.
        if self.no_error_speech.contains(&id) {
            return;
        }
        let now = self.clock;
        if self.told_vocal.get(&id).is_some_and(|t| now < *t) {
            return;
        }
        let Some(sound) = vocal_sound(look.race, look.gender, line) else { return };
        self.told_vocal.insert(id, now + VOCAL_EVERY);
        self.events.push(Event::Hear { player: id, sound });
    }
}

/// For the tests: what an outcome showed.
#[cfg(test)]
pub(crate) mod seen {
    use super::super::{Event, Id};

    /// `hero` felt this beat.
    pub fn beat(ev: &[Event], hero: Id, beat: &str) -> bool {
        ev.iter().any(|e| matches!(e, Event::Beat { player, beat: b, .. } if *player == hero && *b == beat))
    }

    /// This sound came from `on`.
    pub fn sounded(ev: &[Event], on: Id, sound: u32) -> bool {
        ev.iter().any(|e| matches!(e, Event::Sound { id, sound: s } if *id == on && *s == sound))
    }

    /// This look played on `on`.
    pub fn looked(ev: &[Event], on: Id, kit: u32) -> bool {
        ev.iter().any(|e| matches!(e, Event::Visual { id, kit: k, .. } if *id == on && *k == kit))
    }

    /// No word on `hero`'s screen.
    pub fn no_words(ev: &[Event], hero: Id) -> bool {
        !ev.iter().any(|e| matches!(e, Event::Flash { player, .. } if *player == hero))
    }

    /// `hero` heard this (their own voice, coins).
    pub fn heard(ev: &[Event], hero: Id, sound: u32) -> bool {
        ev.iter().any(|e| matches!(e, Event::Hear { player, sound: s } if *player == hero && *s == sound))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn each_voice_has_every_line_and_a_race_past_eight_has_none() {
        // Human man, out of range: `HumanMale_CantCastOutofRange`; an Orc woman not ready:
        // `OrcFemale_CantUseAbilityCooling`; a Troll man with no arrows: `TrollMale_noammo`.
        assert_eq!(vocal_sound(1, 0, Vocal::OutOfRange), Some(1913));
        assert_eq!(vocal_sound(2, 1, Vocal::NotReady), Some(2383));
        assert_eq!(vocal_sound(8, 0, Vocal::NoArrows), Some(2609));
        assert_eq!(vocal_sound(0, 0, Vocal::CantDo), None);
        assert_eq!(vocal_sound(9, 0, Vocal::CantDo), None);
        // Every one distinct: no voice speaks another's line.
        let mut all: Vec<u32> = VOCALS.iter().flatten().flatten().copied().collect();
        let n = all.len();
        all.sort_unstable();
        all.dedup();
        assert_eq!(all.len(), n, "one kit per voice and line");
    }
}
