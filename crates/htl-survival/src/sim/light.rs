//! The Priest's own state (docs/priest-draft3.md): Penance's rings (`penance.rs`), Angelic
//! Feather (`feather.rs`), Holy Word: Salvation, the group check (`sigil.rs`), Void Rift
//! (`rift.rs`) and the Hollow Star (`hollow.rs`). What they keep from one tick to the next is
//! here.

use super::*;

#[derive(Default)]
pub(crate) struct Light {
    /// Salvation's circles, their rings running out on their beats.
    pub sigils: Vec<sigil::Sigil>,
    /// Priests hanging over their Salvation: who, its ability, when they top out (held there),
    /// until when, and whether they're held.
    pub aloft: Vec<(Id, usize, f32, f32, bool)>,
    /// Penance's rings, closing on the bodies picked.
    pub penances: Vec<penance::Penance>,
    /// Feathers lying on the ground.
    pub feathers: Vec<feather::Feather>,
    /// Void Rifts open.
    pub rifts: Vec<rift::Rift>,
    /// Hollow Stars out.
    pub hollows: Vec<hollow::Hollow>,
    /// A Shadow Word: Death just cast, free or not (a kill with the free one gives no other).
    pub deaths: std::collections::BTreeMap<Id, bool>,
    /// A Mind Blast landing now: its heart (where, how wide, its damage, the blast's own), and
    /// whether a foe was in it.
    pub heart: Option<(Vec3, f32, f32, f32)>,
    pub heart_struck: bool,
    /// A Holy Nova going off now: how much stronger it is for the heroes inside (0: as ever).
    pub boost: f32,
    /// When a Priest last let a Greater Smite throw them back (once a jump).
    pub recoiled: std::collections::BTreeMap<Id, f32>,
    /// When each hero last fired a shadow staff's bolt, and who wears its Empowered look now.
    pub last_bolt: std::collections::BTreeMap<Id, f32>,
    pub empower_shown: std::collections::BTreeSet<Id>,
    /// Salvation's floor marks (its circle and the ring): a client that draws the circle itself
    /// from the `SV` line (Corey's art, on the ground) isn't sent these.
    pub drawn_marks: std::collections::BTreeSet<Id>,
}

impl Survival {
    /// A floor mark a client with ground rings draws itself (Salvation's circle and rim, `SV`).
    pub fn drawn_by_addon(&self, mark: Id) -> bool {
        self.light.drawn_marks.contains(&mark)
    }

    /// The Priest's every tick: rings closing, feathers taken, circles beating, rifts dragging,
    /// orbs pulsing.
    pub(crate) fn step_light(&mut self) {
        self.step_penances();
        self.step_feathers();
        self.step_sigils();
        self.step_rifts();
        self.step_hollows();
        self.step_empower();
        self.step_aloft();
    }
}
