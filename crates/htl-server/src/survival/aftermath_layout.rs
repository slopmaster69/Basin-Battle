//! Authored ruin sites, in WoW world coordinates. These cover the side galleries and both
//! Foundry floors as well as the route through the instance. No placement follows a boss.
//! Where each pile leans, how tall it stands and what roof it fell from are read from the map
//! (`aftermath_piles.rs`); a site says only where, how big and how the rock came down.

use super::{Dressing, Fall, Region, Site, BEAM, BOARD, ROCK, SCREE, SKIRT};
use Dressing::*;
use Fall::*;

pub(super) const REGIONS: &[Region] = &[
    Region { key: "mouth", name: "Mine Entrance", view: [-40.0, -385.0, 55.8], facing: 3.1 },
    Region { key: "galleries", name: "Mining Galleries", view: [-112.5, -405.0, 59.9], facing: 3.4 },
    Region { key: "mast", name: "Mast Room", view: [-244.0, -482.0, 49.4], facing: 3.5 },
    Region { key: "workshop", name: "Workshop", view: [-292.0, -555.0, 49.4], facing: 4.4 },
    Region { key: "foundry", name: "Foundry", view: [-250.0, -578.0, 51.1], facing: 0.0 },
    Region { key: "cannon", name: "Cannon Gallery", view: [-133.0, -591.0, 18.7], facing: 5.0 },
    Region { key: "cove", name: "Ironclad Cove", view: [-2.0, -768.0, 9.6], facing: 3.9 },
    Region { key: "landing", name: "Smugglers' Landing", view: [-80.0, -936.0, 40.0], facing: 1.8 },
];

/// The review's other cameras (Codex's branch and cove views, filmed again on 10 Nov): where each
/// stands and the way it looks (map degrees). The ruin keeps their footing and the yards in front
/// clear, as it does the regions' views (Johnson's camera stood inside the debris).
pub(super) const CAMERAS: &[([f32; 3], f32)] = &[
    ([-59.0, -363.0, 54.6], 105.0), ([-49.0, -372.0, 55.4], 122.0), ([-120.0, -438.0, 54.8], 305.0),
    ([-127.0, -442.0, 55.7], 346.0), ([-160.0, -526.0, 52.5], 338.0), ([-158.0, -512.0, 53.0], 294.0),
    ([-300.0, -595.0, 48.2], 258.0), ([-200.0, -585.0, 21.0], 234.0), ([-208.0, -580.0, 27.9], 284.0),
    ([-114.0, -614.0, 13.7], 348.0), ([-146.0, -906.0, 8.1], 186.0), ([-103.0, -993.0, 35.0], 235.0),
    ([8.0, -766.0, 9.6], 220.0), ([-6.0, -758.0, 9.0], 222.0), ([-86.0, -936.0, 39.2], 85.0),
    ([-76.0, -925.0, 36.0], 89.0), ([-127.0, -790.0, 17.2], 328.0),
];

// The views' foregrounds: a few small stones, soil and boards between the support bases, the
// inspector's own footing clear. Each row is (model, forward/side yards from the view, size,
// turn); a board's size is its length in yards.
pub(super) const APRONS: &[&[(usize, [f32; 2], f32, f32)]] = &[
    &[(ROCK, [5.4, -1.8], 0.18, 1.1), (BOARD, [2.6, 2.2], 2.6, 0.3), (ROCK, [6.8, 3.4], 0.24, 2.8),
      (SCREE, [4.5, -3.2], 0.9, 1.5), (BOARD, [-2.6, -2.4], 2.2, 0.8)],
    &[(ROCK, [2.8, -2.2], 0.17, 1.8), (BOARD, [3.1, 2.5], 2.8, -1.0), (SCREE, [5.0, 3.5], 0.8, 0.8),
      (ROCK, [7.2, 3.0], 0.22, 0.5), (BEAM, [-3.2, 1.6], 3.0, 2.5)],
    &[(BOARD, [3.0, -2.7], 3.2, 0.4), (ROCK, [7.0, -4.0], 0.24, 2.7), (SKIRT, [5.8, 3.1], 0.8, 2.1),
      (ROCK, [-2.0, -2.8], 0.16, 1.2), (BOARD, [2.1, 3.2], 2.4, 1.8)],
    &[(SCREE, [4.2, 3.7], 0.8, 0.2), (BOARD, [3.6, -2.2], 2.7, 2.0), (ROCK, [5.2, 2.0], 0.18, 0.4),
      (BEAM, [-2.4, 2.8], 2.8, -1.0), (ROCK, [-3.2, -1.7], 0.15, 1.5)],
    &[(SCREE, [4.8, -3.0], 0.8, 2.6), (BOARD, [2.4, 2.8], 2.2, 1.3), (ROCK, [7.0, -3.0], 0.25, 0.8),
      (ROCK, [-3.0, 2.3], 0.17, 1.4), (BOARD, [-3.0, -2.0], 2.0, 0.4)],
    &[(BOARD, [3.7, -2.4], 2.6, 2.6), (ROCK, [4.8, 3.3], 0.19, 0.7), (SCREE, [6.5, 3.9], 0.7, 1.2),
      (BOARD, [-2.8, 2.4], 2.0, 0.2), (ROCK, [6.2, -3.1], 0.2, 0.4)],
    &[(BOARD, [3.2, 2.7], 3.0, 0.5), (BOARD, [5.8, -2.4], 2.6, -0.9), (ROCK, [-3.2, 2.6], 0.18, 2.8),
      (ROCK, [4.6, 3.7], 0.2, 1.1), (SCREE, [7.2, -2.1], 0.8, 0.8)],
    &[(BOARD, [3.4, -2.6], 2.6, 0.7), (ROCK, [-2.8, 2.3], 0.18, 1.4), (BOARD, [-3.0, -1.9], 2.0, 2.7),
      (SCREE, [6.4, 2.7], 0.7, 2.1)],
];

// Scale is the whole pile's size: a tunnel's main mass is 6-10 yd across; the Mast Room and the
// cove take larger ones. Side passages that are sealed fill to their lintels; on the way through
// the rock slumps against a wall and leaves the passage open.
macro_rules! site {
    ($region:literal, $name:literal, $at:expr, $yaw:expr, $scale:expr, $fall:ident, $kind:ident) => {
        Site { region: $region, name: $name, at: $at, yaw: $yaw, scale: $scale, fall: $fall, dressing: $kind }
    };
}

pub(super) const SITES: &[Site] = &[
    site!(0, "Buried front door", [-62.5, -350.0, 55.9], 0.6, 1.1, Slump, Tunnel),
    site!(0, "Entrance supports", [-65.0, -360.0, 54.1], 1.8, 0.7, Slump, Timber),
    site!(0, "Foreman's chamber", [-70.0, -380.0, 54.2], 2.6, 1.25, Slump, Tunnel),
    site!(1, "Lamp gallery", [-123.0, -409.5, 58.6], 2.9, 0.9, Slump, Tunnel),
    site!(1, "Eastern approach", [-100.0, -400.0, 58.5], 0.6, 0.85, Spill, Cargo),
    site!(1, "Upper gallery bend", [-140.0, -400.0, 58.2], 1.5, 1.1, Slump, Timber),
    site!(1, "West gallery throat", [-165.0, -405.0, 57.9], 0.3, 0.9, Spill, Tunnel),
    site!(1, "North mine collapse", [-190.0, -442.5, 53.6], 4.5, 1.25, Slump, Tunnel),
    site!(1, "Johnson's side gallery", [-115.0, -445.0, 55.4], 2.0, 1.1, Seal, Cargo),
    site!(1, "Mine crossing", [-190.0, -480.0, 54.0], 1.1, 1.1, Slump, Timber),
    site!(1, "East crossing", [-180.0, -480.0, 54.6], 0.1, 0.8, Spill, Tunnel),
    site!(1, "Southern gallery", [-190.0, -505.0, 53.2], 3.8, 1.1, Slump, Tunnel),
    site!(1, "Eastern side tunnel", [-170.0, -510.0, 53.6], 0.7, 0.9, Spill, Cargo),
    site!(1, "Sealed eastern niche", [-150.0, -530.0, 50.2], 2.4, 1.25, Seal, Timber),
    site!(1, "Gallery cart wreck", [-220.0, -492.5, 48.3], 4.3, 1.0, Spill, Minecart),
    site!(1, "Mast approach", [-235.0, -490.0, 49.9], 2.1, 0.85, Spill, Tunnel),
    site!(2, "Mast room collapse", [-253.5, -485.5, 49.4], 0.3, 1.1, Slump, Timber),
    site!(2, "Broken timber yard", [-260.0, -490.0, 49.4], 1.4, 1.2, Spill, Timber),
    site!(2, "Shredder wreck", [-288.0, -505.0, 50.0], 3.0, 1.1, Slump, Machine),
    site!(2, "Mast south wall", [-295.5, -533.5, 49.4], 0.8, 1.0, Slump, Timber),
    site!(3, "Workshop throat", [-285.0, -545.0, 49.4], 4.4, 0.8, Spill, Minecart),
    site!(3, "Workshop north", [-295.0, -564.5, 49.0], 1.2, 1.0, Slump, Machine),
    site!(3, "Workshop bench wreck", [-300.0, -580.0, 47.2], 4.5, 1.25, Slump, Machine),
    site!(3, "Workshop side passage", [-295.0, -605.0, 47.6], 2.7, 1.0, Spill, Minecart),
    site!(3, "Workshop rear cell", [-303.0, -609.0, 47.6], 0.4, 0.9, Seal, Cargo),
    site!(3, "Foundry supply passage", [-260.0, -580.0, 50.8], 3.3, 0.85, Spill, Cargo),
    site!(4, "Foundry back door", [-241.0, -579.0, 51.2], 0.0, 1.0, Lintel, Timber),
    site!(4, "Upper west rail", [-233.0, -584.0, 51.2], 0.4, 0.8, Spill, Timber),
    site!(4, "Upper loading bay", [-230.0, -570.0, 51.2], 3.1, 1.1, Slump, Machine),
    site!(4, "Upper north machinery", [-215.0, -555.0, 51.2], 1.4, 1.2, Slump, Machine),
    site!(4, "Upper east landing", [-192.0, -563.0, 51.2], 2.6, 1.05, Spill, Minecart),
    site!(4, "Broken high ramp", [-184.0, -575.0, 47.9], 3.8, 0.7, Spill, Timber),
    site!(4, "Middle ramp debris", [-185.0, -587.0, 41.9], 4.6, 0.65, Spill, Cargo),
    site!(4, "Ramp machinery", [-195.0, -598.0, 34.4], 0.9, 0.7, Spill, Machine),
    site!(4, "Low ramp foot", [-207.0, -600.0, 28.2], 2.1, 0.7, Spill, Timber),
    site!(4, "Gilnid's broken foundry", [-205.0, -592.0, 21.0], 0.2, 1.4, Slump, Machine),
    site!(4, "Lower west bay", [-219.0, -595.0, 21.8], 2.5, 1.05, Spill, Minecart),
    site!(4, "Lower east bay", [-182.0, -585.0, 19.3], 3.6, 1.0, Spill, Machine),
    site!(4, "Lower gate", [-175.0, -570.0, 19.3], 4.7, 0.9, Spill, Tunnel),
    site!(5, "Cannon gallery entrance", [-155.0, -585.0, 19.3], 2.0, 1.0, Spill, Cannon),
    site!(5, "Gallery upper corner", [-130.4, -600.0, 16.1], 3.7, 0.85, Slump, Timber),
    site!(5, "Marisa's side room", [-105.0, -616.0, 14.2], 1.2, 1.0, Seal, Cargo),
    site!(5, "Upper cannon approach", [-125.0, -610.0, 14.9], 2.9, 0.75, Slump, Tunnel),
    site!(5, "Middle cannon approach", [-126.0, -615.0, 13.7], 4.5, 0.7, Spill, Cannon),
    site!(5, "Lower cannon approach", [-119.0, -639.0, 11.4], 2.4, 0.85, Spill, Minecart),
    site!(5, "Cannon battery", [-110.0, -650.0, 7.2], 0.7, 1.15, Slump, Cannon),
    site!(5, "Battery east wall", [-100.0, -660.0, 7.4], 1.9, 0.9, Slump, Tunnel),
    site!(5, "Cove passage west", [-109.0, -691.0, 4.6], 4.4, 0.9, Spill, Timber),
    site!(5, "Cove passage east", [-100.0, -692.0, 8.2], 3.2, 0.8, Spill, Cargo),
    site!(5, "Cove side quarry", [-120.4, -702.1, 8.6], 0.4, 1.1, Spill, Tunnel),
    site!(5, "Cove passage bend", [-93.0, -716.0, 8.6], 1.6, 0.8, Spill, Tunnel),
    site!(6, "North cove wall", [-80.0, -731.0, 8.9], 3.1, 1.2, Spill, Shore),
    site!(6, "North cove cargo", [-57.0, -728.0, 8.9], 4.3, 1.05, Spill, Cargo),
    site!(6, "North east cave-in", [-38.4, -720.1, 8.4], 0.8, 1.3, Slump, Shore),
    site!(6, "Cove east rim", [-21.0, -742.0, 9.1], 2.0, 1.0, Spill, Shore),
    site!(6, "Smite's ruined landing", [-2.0, -768.0, 9.6], 3.5, 1.4, Spill, Cannon),
    site!(6, "North west waterline", [-92.4, -746.1, 2.8], 4.7, 1.3, Spill, Shore),
    site!(6, "West cove shorefall", [-132.4, -776.1, 0.16], 0.5, 1.25, Spill, Shore),
    site!(6, "West cove south shelf", [-160.0, -870.0, 9.8], 2.1, 1.3, Spill, Shore),
    site!(6, "West waterline", [-140.0, -870.0, 0.9], 4.0, 1.2, Spill, Shore),
    // The cove's stores tent (Codex's "pantry"): the fall spills beside it, not on it.
    site!(6, "Pantry tent stores", [-152.5, -905.0, 8.0], 0.8, 0.9, Spill, Cargo),
    site!(6, "South west beach", [-140.0, -900.0, 6.8], 2.9, 1.2, Spill, Shore),
    site!(6, "South cove shelf", [-100.0, -900.0, 22.8], 4.3, 1.05, Spill, Shore),
    site!(7, "Cook's destroyed stores", [-63.0, -900.0, 16.0], 2.0, 1.2, Spill, Cargo),
    site!(7, "Beach slope", [-67.4, -914.1, 17.7], 3.4, 0.9, Spill, Shore),
    site!(7, "Landing ascent", [-74.0, -922.0, 32.0], 4.7, 0.75, Spill, Timber),
    site!(7, "Smugglers' lamp", [-80.0, -936.0, 40.0], 0.8, 0.9, Spill, Cargo),
    site!(7, "Sea tunnel mouth", [-85.0, -958.0, 42.5], 2.3, 1.0, Slump, Tunnel),
    site!(7, "Sea tunnel bend", [-94.0, -988.0, 36.0], 4.0, 0.8, Slump, Timber),
    site!(7, "Smugglers' cache", [-107.0, -998.5, 34.0], 1.1, 1.0, Seal, Cargo),
];
