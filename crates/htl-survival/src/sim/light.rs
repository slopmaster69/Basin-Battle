//! The Light Priest's own (claude/priest-kit-draft.md): Penance's flick and Holy Fire's Kindle
//! (`flick.rs`), Holy Nova and Leap of Faith as counters for others (`aid.rs`), and Holy Word:
//! Salvation, the group check (`sigil.rs`). What they keep from one tick to the next is here.

use super::*;

#[derive(Default)]
pub(crate) struct Light {
    /// Friends a Holy Nova or a Leap of Faith reached a moment ago: a blow landing on them now is
    /// caught.
    pub covers: Vec<aid::Cover>,
    /// Salvation's circles, closing on their beats.
    pub sigils: Vec<sigil::Sigil>,
    /// Penance's bolts on their way, counted as they land.
    pub flicks: Vec<flick::Flick>,
    /// Salvation's floor marks (its circle and the rim closing): a client that draws the circle
    /// itself from the `SV` line (Corey's art, on the ground) isn't sent these.
    pub drawn_marks: std::collections::BTreeSet<Id>,
}

impl Survival {
    /// A floor mark a client with ground rings draws itself (Salvation's circle and rim, `SV`).
    pub fn drawn_by_addon(&self, mark: Id) -> bool {
        self.light.drawn_marks.contains(&mark)
    }

    /// The Light Priest's every tick: old flicks and covers forgotten, circles closing.
    pub(crate) fn step_light(&mut self) {
        self.step_flicks();
        self.step_covers();
        self.step_sigils();
    }
}
