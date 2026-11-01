//! The death recap: the blows that hurt a hero, kept a little while and read back when they fall,
//! so a fall says what felled you and how it caught you.

use super::{Action, Blow, Id, Outcome, Survival};

/// How long a blow is remembered for the recap (seconds).
pub const RECAP_SECS: f32 = 10.0;
/// The most blows a recap names: the one that felled them and the two before it.
pub const RECAP_BLOWS: usize = 3;
/// The most blows kept on a hero at once.
const KEPT: usize = 8;

/// A blow that hurt a hero.
#[derive(Debug, Clone, PartialEq)]
pub struct Wound {
    pub at: f32,
    /// Who struck ("Targorr the Dread"); empty for the world's own hurts (a fall, burning ground).
    pub by: String,
    /// What it was: "heavy blow", "blow", "heavy attack", "shot", "burn", "fall", "burning ground".
    pub what: String,
    pub damage: u32,
    /// How it caught them: "through your guard", "unguarded", "mid-swing"; empty when it doesn't
    /// matter (a fall).
    pub caught: &'static str,
}

/// What the next hurt on a hero is: set by the blow (or the ground) just before it lands.
#[derive(Debug, Clone, PartialEq)]
pub struct Cause {
    /// Who struck; 0 for the world's own.
    pub by: Id,
    pub what: String,
    pub caught: &'static str,
}

impl Cause {
    /// The world's own hurt: a fall, burning ground.
    pub fn world(what: &str) -> Self {
        Cause { by: 0, what: what.to_string(), caught: "" }
    }

    /// A blow from `by`: what it was and how it caught them.
    pub fn blow(by: Id, what: &str, caught: &'static str) -> Self {
        Cause { by, what: what.to_string(), caught }
    }
}

/// A recap: the blows, the one that felled them first, and a word on how to answer it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Recap {
    pub lines: Vec<String>,
    pub tip: String,
}

/// What kind of blow it was.
pub fn blow_word(style: Blow, heavy: bool) -> &'static str {
    match style {
        Blow::Dot => "burn",
        Blow::Arrow(_) => "shot",
        Blow::Heavy => "heavy attack",
        Blow::Light if heavy => "heavy blow",
        Blow::Light => "blow",
    }
}

/// How a blow caught a hero doing this (before the blow changed it), and how it came out.
pub fn caught_word(action: &Action, outcome: Outcome, blockable: bool) -> &'static str {
    if outcome == Outcome::GuardBroken {
        return "through your guard";
    }
    match action {
        Action::Guard if !blockable => "past your guard (it can't be blocked)",
        Action::Guard => "around your guard",
        Action::Dodge { .. } => "outside your dodge",
        Action::Swing { .. } => "mid-swing",
        Action::Aim { .. } | Action::Charge { .. } | Action::Cast { .. } | Action::Channel { .. } => "mid-cast",
        Action::Stance { .. } => "in your stance",
        Action::Dash { .. } => "mid-dash",
        Action::Eat { .. } => "while eating",
        Action::Gather { .. } | Action::Work { .. } => "while busy",
        Action::Stagger { .. } => "while reeling",
        Action::Ready => "unguarded",
        Action::Dead { .. } => "",
    }
}

/// A word on how to answer what felled them.
fn tip_for(w: &Wound) -> &'static str {
    match (w.what.as_str(), w.caught) {
        ("fall", _) => "Mind the drop.",
        ("burning ground", _) => "Step off a red mark before it burns.",
        (_, "through your guard") => "A guard breaks when your stamina runs out: dodge the heavy ones.",
        (_, "past your guard (it can't be blocked)") => "That one can't be blocked: dodge it.",
        (_, "around your guard") => "A guard only covers your front: turn to face them.",
        (_, "outside your dodge") => "A dodge is safe only in its middle: roll as the blow comes down.",
        (_, "mid-swing") => "A swing commits you: watch for the wind-up before you swing.",
        (_, "mid-cast") => "Casting leaves you open: cast when the foe is busy.",
        (_, "while reeling") => "Once you reel, the next blows come free: don't let the guard break.",
        (_, "unguarded") => "Raise your guard or dodge when a blow winds up.",
        _ => "",
    }
}

/// One blow, as the recap says it: "Targorr the Dread's heavy blow, through your guard (212)".
fn line_of(w: &Wound) -> String {
    let what = if w.by.is_empty() {
        let mut c = w.what.chars();
        c.next().map_or(String::new(), |f| f.to_uppercase().collect::<String>() + c.as_str())
    } else {
        format!("{}'s {}", w.by, w.what)
    };
    let line = if w.caught.is_empty() { what } else { format!("{what}, {}", w.caught) };
    // The add-on splits on tabs and carets: never let a name carry one.
    format!("{line} ({})", w.damage).replace(['\t', '^'], " ")
}

impl Survival {
    /// What the next hurt on this hero is (the blow about to land, the ground under them).
    pub(crate) fn set_cause(&mut self, id: Id, cause: Cause) {
        if let Some(p) = self.players.get_mut(&id) {
            p.cause = Some(cause);
        }
    }

    /// Keep a hurt that took `lost` health for the recap, by its cause (set just before; none: a
    /// hurt of unknown cause).
    pub(crate) fn keep_wound(&mut self, id: Id, lost: u32) {
        let now = self.clock;
        let Some(cause) = self.players.get_mut(&id).map(|p| p.cause.take()) else { return };
        if lost == 0 {
            return;
        }
        let (by, what, caught) = match cause {
            Some(c) if c.by == 0 => (String::new(), c.what, c.caught),
            Some(c) => (self.unit_name(c.by), c.what, c.caught),
            None => (String::new(), "wound".to_string(), ""),
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        p.wounds.retain(|w| now - w.at <= RECAP_SECS);
        p.wounds.push(Wound { at: now, by, what, damage: lost, caught });
        let extra = p.wounds.len().saturating_sub(KEPT);
        p.wounds.drain(..extra);
    }

    /// The recap of a hero's fall: the blow that felled them, then up to two before it, all from
    /// the last [`RECAP_SECS`]; and a word on answering the last.
    pub fn death_recap(&self, id: Id) -> Recap {
        let Some(p) = self.players.get(&id) else { return Recap::default() };
        let last = p.wounds.last().map_or(0.0, |w| w.at);
        let recent: Vec<&Wound> = p.wounds.iter().rev().filter(|w| last - w.at <= RECAP_SECS).take(RECAP_BLOWS).collect();
        Recap { lines: recent.iter().map(|w| line_of(w)).collect(), tip: recent.first().map_or("", |w| tip_for(w)).to_string() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn wound(by: &str, what: &str, caught: &'static str, damage: u32) -> Wound {
        Wound { at: 0.0, by: by.into(), what: what.into(), damage, caught }
    }

    #[test]
    fn a_blow_reads_as_who_what_how_and_how_much() {
        assert_eq!(line_of(&wound("Targorr the Dread", "heavy blow", "through your guard", 212)), "Targorr the Dread's heavy blow, through your guard (212)");
        assert_eq!(line_of(&wound("", "burning ground", "", 30)), "Burning ground (30)");
        assert_eq!(line_of(&wound("Odd\tName^", "blow", "unguarded", 5)), "Odd Name 's blow, unguarded (5)");
    }

    #[test]
    fn every_way_of_being_caught_has_a_tip() {
        for caught in [
            "through your guard",
            "past your guard (it can't be blocked)",
            "around your guard",
            "outside your dodge",
            "mid-swing",
            "mid-cast",
            "while reeling",
            "unguarded",
        ] {
            assert!(!tip_for(&wound("Kobold", "blow", caught, 1)).is_empty(), "{caught}");
        }
    }

    #[test]
    fn a_broken_guard_wins_over_what_they_were_doing() {
        assert_eq!(caught_word(&Action::Guard, Outcome::GuardBroken, true), "through your guard");
        assert_eq!(caught_word(&Action::Guard, Outcome::Hit, false), "past your guard (it can't be blocked)");
        assert_eq!(caught_word(&Action::Ready, Outcome::Hit, true), "unguarded");
    }
}
