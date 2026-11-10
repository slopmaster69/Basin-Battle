//! The mine's own doodads the collapse moved. Client patch 0091 hides them while the preview's
//! marker stands ([`htl_map::deadmines::aftermath::MINE_DOODADS`]: the neat cargo, carts and
//! barrows of views 3, 9 and 15, and a third of the mine's lit lanterns), and puts out a lantern's
//! own light when its fate here is dark ([`htl_map::deadmines::aftermath::DARK_LANTERNS`]); the
//! server sets its own copies where they stood (`aftermath_piles.rs`'s `knock_over`), fitted to
//! the mine without them, among the piles.
//!
//! Only a client of round [`LEVEL`] hides them, so only one that says so in its hello gets the
//! copies: an older client keeps the mine's own and is sent nothing more.
//!
//! The lanterns: the walk from the entrance to the cove stays lit, and so do the mine's own
//! lanterns on the way in from the portal; a third of the rest go out. Those right over a pile
//! came down with the roof (the entrance's, view 1, and the buried front door's, view 15), and
//! those in the dead ends (Johnson's gallery, the eastern side tunnel, the passages off the
//! workshop, the corner by the entrance, the Shredder's loft and the lower Foundry's far bays) are
//! gone. A put-out lantern leaves no copy: every lantern model in the data has the same glowing
//! yellow glass, and the one first laid on the front door's rock read lit on film (10 Nov). Some
//! on the walk hang askew and still burn, the eastern niche's among them (view 9: the review found
//! it too dark to read already).

use super::*;

/// The client round that hides the mine's doodads (patch 0091): `.es hello <version> ac30`.
pub(crate) const LEVEL: u8 = 30;

/// What became of one of the mine's own doodads.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) enum Fate {
    /// Knocked over where it stood, as `model`, rolled `roll` about its length and pitched `pitch`
    /// (a stack on its side, a cart on its side, a barrow upside down, a keg rolled).
    Over(usize, f32, f32),
    /// Hung askew from its own hook, as `model`, its light still burning.
    Askew(usize),
    /// A lantern gone: buried in the fall or smashed, its light out. (No dark copy: the data's
    /// lanterns all have glowing glass, and a fallen one read lit on film.)
    Gone,
    /// Gone under the fall (a sack that leaned on a lumber pile that spilled).
    Buried,
}
use Fate::*;

use std::f32::consts::{FRAC_PI_2, PI};

/// Each hidden doodad (by its place in the mine's WMO) and its fate, in order. The client's lists
/// are these indices and the dark ones' (`aftermath_tests.rs` holds patch 0091 to them).
pub(super) const PLAN: &[(u32, Fate)] = &[
    // The Southern gallery's lantern, on the walk.
    (67, Askew(LANTERN)),
    // The eastern niche (view 9): its stacked lumber spilled under the fall, the bucket that stood
    // on it kicked over, the sack that leaned on it gone under the rock; its lantern askew.
    (88, Buried),
    (89, Askew(LANTERN)),
    (92, Over(LUMBER, 1.15, 0.0)),
    (93, Over(BUCKET, FRAC_PI_2, 0.2)),
    // The eastern side tunnel, a dead end.
    (98, Gone),
    (101, Gone),
    // The buried front door (view 15): the barrow upside down, the stacked lumber spilled, its
    // bucket kicked over, and the lantern over the fall gone with the roof.
    (103, Over(BARROW, PI, 0.15)),
    (104, Over(LUMBER, 1.15, 0.0)),
    (105, Over(BUCKET, FRAC_PI_2, -0.2)),
    (106, Gone),
    // Johnson's gallery, a dead end.
    (108, Gone),
    (120, Gone),
    // The west gallery's throat, on the walk.
    (141, Askew(LANTERN)),
    // The corner south of the entrance, a dead end.
    (162, Gone),
    // The entrance (view 1): the lantern over the Foreman's pile gone with the roof, the next one
    // on askew; the floor lantern of the eastern approach's dead end gone.
    (184, Gone),
    (185, Gone),
    (186, Askew(LANTERN)),
    // The north gallery's high lanterns, off the walk.
    (220, Gone),
    (221, Gone),
    // The Mast Room (view 3): its neat cargo, kegs and barrels toppled and rolled, the netted cargo
    // on its side, the large lumber pile spilled and the ore cart tipped.
    (251, Over(KEG, FRAC_PI_2, 0.0)),
    (252, Over(KEG, FRAC_PI_2, 0.3)),
    (253, Over(CASK, FRAC_PI_2, 0.0)),
    (254, Over(CASK, FRAC_PI_2, -0.2)),
    (255, Over(CARGO, 1.45, 0.0)),
    (256, Over(LUMBER_PILE, 1.15, 0.0)),
    (257, Over(NET_CARGO, 1.4, 0.0)),
    (259, Over(MINE_CART, 1.5, 0.1)),
    // The Cannon Gallery's top, on the walk.
    (264, Askew(LANTERN)),
    // The cannon approach's side recess.
    (283, Gone),
    // The workshop's side passage and rear cell, and its bench's floor lantern.
    (331, Gone),
    (332, Gone),
    (333, Gone),
    (342, Askew(LANTERN)),
    // The Foundry's far bays, upper and lower.
    (381, Gone),
    (386, Gone),
    (387, Gone),
    // The Shredder's loft over the Mast Room.
    (502, Gone),
    (503, Gone),
    (513, Gone),
];

/// Whether a fate puts the doodad's light out.
#[cfg(test)]
pub(super) fn dark(f: Fate) -> bool { matches!(f, Gone) }

pub(super) fn fate(index: u32) -> Option<Fate> {
    PLAN.iter().find(|(i, _)| *i == index).map(|(_, f)| *f)
}

/// What a copy is called among the ruins (the contact check names a piece by it).
pub(super) fn site(f: Fate) -> &'static str {
    match f {
        Over(..) => "The mine's own, knocked over",
        Askew(_) => "The mine's own lantern, askew",
        Gone | Buried => "",
    }
}

/// The displays the client's patch 0091 adds for the mine's own models, which the data has no
/// game-object row for (benilla-formats' `EXTRA_DISPLAYS`, which must change with this).
pub(super) const EXTRA: &[(u32, &str)] = &[
    (60005, "World\\Generic\\Human\\Passive Doodads\\Lanterns\\GeneralLantern03.mdx"),
    (60006, "World\\Generic\\PassiveDoodads\\Misc\\MineCars\\CaveMineCar01.mdx"),
    (60007, "World\\Generic\\Human\\Passive Doodads\\CargoNetBoxes\\DeadMineCargoNetBoxes.mdx"),
];
