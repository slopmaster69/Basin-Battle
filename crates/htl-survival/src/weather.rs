//! Elwynn's weather: the same for every hero, a fixed function of the server's clock (seconds
//! since the epoch) and `[weather]` in `survival.toml`, never a local random. Mostly clear, with
//! spells of overcast, rain showers and now and then a thunderstorm, each easing in and out over
//! minutes. The server sends it as 1.12's own `SMSG_WEATHER` (rain at a grade, [`wire`]), which any
//! 1.12 client shows: 1.12 clouds the sky over the grade's first quarter and rains past it, so a
//! spell easing in clouds over before the first drops fall. A client with the new look (patch
//! 0071) dresses it: grey skies, rain lit by the hour, wet ground, lightning in a storm.
//!
//! The kinds' grade bands are the client's (`benilla_world::look::weather::tune`); keep in sync.

use serde::Deserialize;

/// A weather, and how strong (0 to 1).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weather {
    pub sky: Sky,
    pub intensity: f32,
}

impl Weather {
    pub const CLEAR: Self = Self { sky: Sky::Clear, intensity: 0.0 };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sky {
    Clear,
    Overcast,
    Rain,
    Storm,
}

impl Sky {
    pub fn parse(w: &str) -> Option<Self> {
        Some(match w.to_ascii_lowercase().as_str() {
            "clear" | "fine" => Sky::Clear,
            "overcast" | "cloudy" => Sky::Overcast,
            "rain" => Sky::Rain,
            "storm" => Sky::Storm,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Sky::Clear => "clear",
            Sky::Overcast => "overcast",
            Sky::Rain => "rain",
            Sky::Storm => "storm",
        }
    }

    /// The grade of this kind at full strength of `intensity` (0 to 1): overcast up to 0.25 (no
    /// drops), rain from 0.3 to 0.8, a storm from 0.85 to 1. Clear is 0.
    pub fn grade(self, intensity: f32) -> f32 {
        let i = intensity.clamp(0.0, 1.0);
        match self {
            Sky::Clear => 0.0,
            Sky::Overcast => OVERCAST_TOP * i,
            Sky::Rain => RAIN_FROM + (RAIN_TO - RAIN_FROM) * i,
            Sky::Storm => STORM_FROM + (1.0 - STORM_FROM) * i,
        }
    }
}

const OVERCAST_TOP: f32 = 0.25;
const RAIN_FROM: f32 = 0.3;
const RAIN_TO: f32 = 0.8;
const STORM_FROM: f32 = 0.85;

/// The sound loops 1.12 plays for rain (SoundEntries; vmangos' `Weather::GetSound`).
const SOUND_LIGHT: u32 = 8533;
const SOUND_MEDIUM: u32 = 8534;
const SOUND_HEAVY: u32 = 8535;

/// 1.12's `SMSG_WEATHER` for a weather eased to `ease` (0 none, 1 full): type (0 clear, 1 rain),
/// grade and sound loop. Easing scales the grade, so rain clouds over before it falls.
pub fn wire(w: Weather, ease: f32) -> (u32, f32, u32) {
    let grade = w.sky.grade(w.intensity) * ease.clamp(0.0, 1.0);
    if grade <= 0.0 {
        return (0, 0.0, 0);
    }
    (1, grade, wire_sound(1, grade))
}

/// The rain's sound loop for a wire weather (type, grade): none while it only clouds over.
pub fn wire_sound(kind: u32, grade: f32) -> u32 {
    if kind != 1 || grade <= OVERCAST_TOP {
        0
    } else if grade < 0.4 {
        SOUND_LIGHT
    } else if grade < 0.7 {
        SOUND_MEDIUM
    } else {
        SOUND_HEAVY
    }
}

/// Whether a grade is a storm's (the client's lightning comes past it).
pub fn stormy(grade: f32) -> bool {
    grade > RAIN_TO
}

/// `[weather]`: how often each weather comes, how long it lasts and how it eases. The odds are
/// shares of the spells (they need not sum to 1).
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct WeatherDef {
    /// Off: always clear.
    pub on: bool,
    /// The day is cut into spells this many minutes long; each rolls its own weather.
    pub spell_minutes: f32,
    pub clear: f32,
    pub overcast: f32,
    pub rain: f32,
    pub storm: f32,
    /// A spell's weather lasts this share of it, at least to at most (the rest is clear).
    pub last_min: f32,
    pub last_max: f32,
    /// Minutes to ease in, and to ease out.
    pub ease_minutes: f32,
    /// A weather's strength rolls between these.
    pub intensity_min: f32,
    pub intensity_max: f32,
    /// A grade change this big goes out to everyone at once.
    pub step: f32,
    /// In a storm, the weather goes out again this often, a hair apart each time: the clients'
    /// lightning runs from the latest, so the strikes never repeat and a hero who came in
    /// mid-storm falls into step.
    pub cue_seconds: f32,
    /// Mixed into every roll, for a different run of weather.
    pub seed: u64,
}

impl Default for WeatherDef {
    fn default() -> Self {
        Self {
            on: true,
            spell_minutes: 40.0,
            clear: 0.55,
            overcast: 0.22,
            rain: 0.16,
            storm: 0.07,
            last_min: 0.45,
            last_max: 0.9,
            ease_minutes: 6.0,
            intensity_min: 0.4,
            intensity_max: 1.0,
            step: 0.02,
            cue_seconds: 40.0,
            seed: 0,
        }
    }
}

fn mix64(x: u64) -> u64 {
    let mut z = x.wrapping_add(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

fn unit(h: u64) -> f32 {
    (h >> 40) as f32 / (1u64 << 24) as f32
}

fn smoothstep(x: f32) -> f32 {
    let t = x.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// One spell: its weather and the window (seconds into the spell) it holds, easing in and out.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Spell {
    pub weather: Weather,
    pub from: f64,
    pub to: f64,
}

impl WeatherDef {
    fn spell_secs(&self) -> f64 {
        f64::from(self.spell_minutes.max(1.0)) * 60.0
    }

    /// Spell `n` of the clock's: the day it falls in and its place in it seed the dice.
    pub fn spell(&self, n: i64) -> Spell {
        let len = self.spell_secs();
        let day = ((n as f64 * len) / 86_400.0).floor() as i64;
        let h = mix64(mix64(self.seed ^ 0x7765_6174_6865_7221) ^ (day as u64).rotate_left(29) ^ n as u64);
        let roll = |k: u64| unit(mix64(h ^ k));
        let odds = [
            (Sky::Clear, self.clear),
            (Sky::Overcast, self.overcast),
            (Sky::Rain, self.rain),
            (Sky::Storm, self.storm),
        ];
        let total: f32 = odds.iter().map(|(_, w)| w.max(0.0)).sum();
        let mut pick = roll(1) * total;
        let mut sky = Sky::Clear;
        for (s, w) in odds {
            if pick < w.max(0.0) {
                sky = s;
                break;
            }
            pick -= w.max(0.0);
        }
        let intensity = self.intensity_min + (self.intensity_max - self.intensity_min) * roll(2);
        let share = self.last_min + (self.last_max - self.last_min) * roll(3);
        let last = len * f64::from(share.clamp(0.05, 1.0));
        let from = (len - last) * f64::from(roll(4));
        Spell { weather: Weather { sky, intensity }, from, to: from + last }
    }

    /// The weather at `secs` (seconds since the epoch) and how far it has eased in (0 to 1).
    pub fn at(&self, secs: f64) -> (Weather, f32) {
        if !self.on {
            return (Weather::CLEAR, 0.0);
        }
        let len = self.spell_secs();
        let n = (secs / len).floor() as i64;
        let s = self.spell(n);
        let t = secs - n as f64 * len;
        if s.weather.sky == Sky::Clear || t < s.from || t > s.to {
            return (Weather::CLEAR, 0.0);
        }
        let ease = f64::from(self.ease_minutes.max(0.1)) * 60.0;
        let ease = ease.min((s.to - s.from) / 2.0);
        let e = smoothstep(((t - s.from) / ease) as f32).min(smoothstep(((s.to - t) / ease) as f32));
        (s.weather, e)
    }
}

/// What the server last sent and when the next storm cue is due, so it sends only a change.
#[derive(Debug, Clone, Default)]
pub struct WeatherSend {
    pub sent: Option<(u32, f32)>,
    /// Seconds (the caller's clock) of the last packet.
    pub sent_at: f64,
    pub cues: u64,
}

impl WeatherSend {
    /// The packet to send everyone at `now` (the caller's seconds) for this wire weather, if any:
    /// a new type, a grade moved by `step`, or in a storm a cue every `cue_seconds` (its grade
    /// nudged a hair, under any look, so the clients' lightning starts a fresh run).
    pub fn next(&mut self, def: &WeatherDef, wire: (u32, f32, u32), now: f64) -> Option<(u32, f32, u32)> {
        let (kind, grade, sound) = wire;
        let due = match self.sent {
            None => true,
            Some((k, g)) => {
                k != kind
                    || (g - grade).abs() >= def.step
                    || (stormy(grade) && now - self.sent_at >= f64::from(def.cue_seconds.max(5.0)))
            }
        };
        if !due {
            return None;
        }
        let mut grade = grade;
        if stormy(grade) {
            self.cues += 1;
            grade -= (mix64(self.cues) % 64) as f32 * 0.0002;
        }
        self.sent = Some((kind, grade));
        self.sent_at = now;
        Some((kind, grade, sound))
    }

    /// The last weather sent, for a hero coming in (sent at once).
    pub fn current(&self) -> (u32, f32) {
        self.sent.unwrap_or((0, 0.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every kind lands in its own band of 1.12's wire, and easing clouds over before it rains.
    #[test]
    fn the_kinds_and_their_wire() {
        assert_eq!(wire(Weather::CLEAR, 1.0), (0, 0.0, 0));
        let overcast = Weather { sky: Sky::Overcast, intensity: 1.0 };
        assert_eq!(wire(overcast, 1.0), (1, 0.25, 0), "clouds, no drops, no sound");
        let storm = Weather { sky: Sky::Storm, intensity: 1.0 };
        assert_eq!(wire(storm, 1.0), (1, 1.0, SOUND_HEAVY));
        assert!(stormy(wire(storm, 1.0).1));
        let (_, g, s) = wire(storm, 0.2);
        assert!(g <= 0.25 && s == 0, "a storm easing in clouds over first");
        assert_eq!(wire(storm, 0.0), (0, 0.0, 0));
        let rain = Weather { sky: Sky::Rain, intensity: 1.0 };
        assert!(!stormy(wire(rain, 1.0).1), "rain is no storm");
        assert_eq!(Sky::parse("Storm"), Some(Sky::Storm));
        assert_eq!(Sky::parse("snow"), None);
    }

    /// The same weather for the same clock, mostly clear, every kind coming up, and each spell
    /// easing in and out over minutes rather than popping.
    #[test]
    fn the_weather_is_a_function_of_the_clock() {
        let def = WeatherDef::default();
        let day = 20_400.0 * 86_400.0;
        assert_eq!(def.at(day + 1234.0), def.at(day + 1234.0));
        let mut counts = [0usize; 4];
        let spells = 2000;
        for n in 0..spells {
            let s = def.spell((day / def.spell_secs()) as i64 + n);
            counts[s.weather.sky as usize] += 1;
        }
        assert!(counts[0] > spells as usize * 2 / 5, "mostly clear: {counts:?}");
        assert!(counts.iter().all(|c| *c > 0), "{counts:?}");
        // Over a week, the eased grade never jumps more than a step's worth in ten seconds.
        let mut last = 0.0f32;
        let mut changed = 0;
        let mut t = day;
        while t < day + 7.0 * 86_400.0 {
            let (w, e) = def.at(t);
            let g = wire(w, e).1;
            assert!((g - last).abs() < 0.05, "a jump at {t}: {last} to {g}");
            if (g - last).abs() > 0.0 {
                changed += 1;
            }
            last = g;
            t += 10.0;
        }
        assert!(changed > 100, "it changes: {changed}");
        let off = WeatherDef { on: false, ..WeatherDef::default() };
        assert_eq!(off.at(day + 99.0), (Weather::CLEAR, 0.0));
    }

    /// A change goes out once, a small one not at all, and a storm cues again now and then, each
    /// time a hair apart.
    #[test]
    fn only_a_change_or_a_storm_cue_goes_out() {
        let def = WeatherDef::default();
        let mut send = WeatherSend::default();
        assert_eq!(send.next(&def, (0, 0.0, 0), 0.0), Some((0, 0.0, 0)));
        assert_eq!(send.next(&def, (0, 0.0, 0), 5.0), None);
        assert!(send.next(&def, (1, 0.1, 0), 6.0).is_some());
        assert_eq!(send.next(&def, (1, 0.11, 0), 7.0), None, "under a step");
        assert!(send.next(&def, (0, 0.0, 0), 8.0).is_some(), "the type changed");
        let a = send.next(&def, (1, 1.0, SOUND_HEAVY), 10.0).expect("the storm");
        assert_eq!(send.next(&def, (1, 1.0, SOUND_HEAVY), 20.0), None);
        let b = send.next(&def, (1, 1.0, SOUND_HEAVY), 10.0 + f64::from(def.cue_seconds)).expect("a cue");
        assert!(a.1 <= 1.0 && b.1 <= 1.0 && a.1 > 0.98 && b.1 > 0.98);
        let mut grades = vec![a.1, b.1];
        for k in 2..10 {
            let c = send.next(&def, (1, 1.0, SOUND_HEAVY), 10.0 + k as f64 * f64::from(def.cue_seconds)).unwrap();
            grades.push(c.1);
        }
        grades.dedup();
        assert!(grades.len() >= 6, "cues a hair apart: {grades:?}");
    }
}
