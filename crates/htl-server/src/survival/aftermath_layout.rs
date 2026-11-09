//! Authored ruin sites, in WoW world coordinates. These cover the side galleries and both
//! Foundry floors as well as the route through the instance. No placement follows a boss.

use super::{Dressing, Region, Site, BARREL, CART, CRATE, DARK_ROCK, PLANK, RUBBLE, WOODPILE};
use Dressing::*;

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

// Foregrounds are deliberately different at each view. Keep the focal path open and lay
// shallow fragments toward the support bases; no repeated central cone or neat log bundle.
// Each row is (model, forward/side offset from the view, scale, additional yaw).
pub(super) const APRONS: &[&[(usize, [f32; 2], f32, f32)]] = &[
    &[
        (WOODPILE,[3.5,-3.4],0.8,1.5), (PLANK,[2.6,2.2],1.25,0.3),
        (PLANK,[4.9,1.5],1.1,2.2), (RUBBLE,[5.4,-1.8],0.26,1.1),
        (DARK_ROCK,[6.8,3.4],0.32,2.8), (PLANK,[-2.6,-2.4],1.1,0.8), (RUBBLE,[-2.0,3.1],0.22,2.7),
    ],
    &[
        (CART,[6.9,-2.6],0.75,2.1), (PLANK,[3.1,2.5],1.3,-1.0),
        (WOODPILE,[5.0,3.5],0.75,0.8), (RUBBLE,[2.8,-2.2],0.25,1.8),
        (PLANK,[-3.2,1.6],1.1,2.5), (DARK_ROCK,[7.2,3.0],0.28,0.5),
    ],
    &[
        (WOODPILE,[5.8,3.1],1.0,2.1), (PLANK,[3.0,-2.7],1.8,0.4),
        (PLANK,[4.5,-3.4],1.4,-0.8), (PLANK,[2.1,3.2],1.2,1.8),
        (DARK_ROCK,[7.0,-4.0],0.3,2.7), (RUBBLE,[-2.0,-2.8],0.23,1.2),
        (PLANK,[-3.4,1.7],1.3,2.8), (WOODPILE,[7.5,1.8],0.75,0.5),
    ],
    &[
        (WOODPILE,[4.2,3.7],0.8,0.2), (PLANK,[3.6,-2.2],1.35,2.0),
        (CRATE,[7.0,-3.5],0.75,1.1), (RUBBLE,[5.2,2.0],0.23,0.4),
        (PLANK,[-2.4,2.8],1.25,-1.0), (PLANK,[-3.2,-1.7],1.1,1.5),
    ],
    &[
        (CART,[7.8,2.3],0.65,0.5), (WOODPILE,[4.8,-3.0],0.75,2.6),
        (PLANK,[2.4,2.8],1.0,1.3), (DARK_ROCK,[7.0,-3.0],0.35,0.8),
        (RUBBLE,[-3.0,2.3],0.25,1.4), (CRATE,[5.5,3.5],0.7,2.9), (PLANK,[-3.0,-2.0],0.9,0.4),
    ],
    &[
        (BARREL,[6.2,-3.1],1.0,0.4), (CRATE,[7.3,2.5],0.8,1.8),
        (PLANK,[3.7,-2.4],1.25,2.6), (RUBBLE,[4.8,3.3],0.27,0.7),
        (PLANK,[-2.8,2.4],1.0,0.2), (WOODPILE,[6.5,3.9],0.65,1.2),
    ],
    &[
        (PLANK,[3.2,2.7],1.6,0.5), (PLANK,[5.8,-2.4],1.3,-0.9),
        (BARREL,[7.0,3.2],0.9,2.3), (CRATE,[5.0,-3.6],0.8,1.6),
        (WOODPILE,[7.2,-2.1],0.9,0.8), (DARK_ROCK,[-3.2,2.6],0.25,2.8),
        (RUBBLE,[4.6,3.7],0.28,1.1), (PLANK,[-2.4,-2.8],1.15,1.8),
    ],
    &[
        (PLANK,[3.4,-2.6],1.3,0.7), (CRATE,[6.4,2.7],0.75,2.1),
        (PLANK,[4.9,-3.4],1.1,-0.4), (DARK_ROCK,[-2.8,2.3],0.25,1.4), (PLANK,[-3.0,-1.9],1.0,2.7),
    ],
];

// Scale describes the whole collapse composition, not a dust radius. A tunnel's primary
// dark boulder is 6–10 yd across; the Mast Room and cove use larger broken silhouettes.
macro_rules! site {
    ($region:literal, $name:literal, $at:expr, $yaw:expr, $scale:expr, $kind:ident) => {
        Site { region: $region, name: $name, at: $at, yaw: $yaw, scale: $scale, dressing: $kind }
    };
}

pub(super) const SITES: &[Site] = &[
    site!(0, "Buried front door", [-62.5, -350.0, 55.9], 0.6, 1.4, Tunnel),
    site!(0, "Entrance supports", [-65.0, -360.0, 54.1], 1.8, 0.9, Timber),
    site!(0, "Foreman's chamber", [-70.0, -380.0, 54.2], 2.6, 1.25, Tunnel),
    site!(0, "Entrance east wall", [-40.0, -385.0, 55.8], 0.0, 0.9, Timber),
    site!(1, "Lamp gallery", [-112.5, -405.0, 59.9], 2.9, 0.9, Tunnel),
    site!(1, "Eastern approach", [-100.0, -400.0, 58.5], 0.6, 0.85, Cargo),
    site!(1, "Upper gallery bend", [-140.0, -400.0, 58.2], 1.5, 1.1, Timber),
    site!(1, "West gallery throat", [-165.0, -405.0, 57.9], 0.3, 0.9, Tunnel),
    site!(1, "North mine collapse", [-190.0, -442.5, 53.6], 4.5, 1.25, Tunnel),
    site!(1, "Johnson's side gallery", [-115.0, -445.0, 55.4], 2.0, 1.1, Cargo),
    site!(1, "Mine crossing", [-190.0, -480.0, 54.0], 1.1, 1.1, Timber),
    site!(1, "East crossing", [-180.0, -480.0, 54.6], 0.1, 0.8, Tunnel),
    site!(1, "Southern gallery", [-190.0, -505.0, 53.2], 3.8, 1.1, Tunnel),
    site!(1, "Eastern side tunnel", [-170.0, -510.0, 53.6], 0.7, 0.9, Cargo),
    site!(1, "Sealed eastern niche", [-150.0, -530.0, 50.2], 2.4, 1.25, Timber),
    site!(1, "Gallery cart wreck", [-220.0, -492.5, 48.3], 4.3, 1.0, Minecart),
    site!(1, "Mast approach", [-235.0, -490.0, 49.9], 2.1, 0.85, Tunnel),
    site!(2, "Mast room east door", [-244.0, -482.0, 49.4], 0.3, 1.05, Timber),
    site!(2, "Broken timber yard", [-260.0, -490.0, 49.4], 1.4, 1.2, Timber),
    site!(2, "Shredder wreck", [-288.0, -505.0, 50.0], 3.0, 1.65, Machine),
    site!(2, "Mast south wall", [-290.3, -537.0, 49.4], 0.8, 1.15, Timber),
    site!(3, "Workshop throat", [-285.0, -545.0, 49.4], 4.4, 0.8, Minecart),
    site!(3, "Workshop north", [-292.0, -555.0, 49.4], 1.2, 1.0, Machine),
    site!(3, "Workshop bench wreck", [-300.0, -580.0, 47.2], 4.5, 1.35, Machine),
    site!(3, "Workshop side passage", [-295.0, -605.0, 47.6], 2.7, 1.0, Minecart),
    site!(3, "Workshop rear cell", [-303.0, -609.0, 47.6], 0.4, 0.9, Cargo),
    site!(3, "Foundry supply passage", [-260.0, -580.0, 50.8], 3.3, 0.85, Cargo),
    site!(4, "Foundry back door", [-250.0, -578.0, 51.1], 1.8, 0.85, Machine),
    site!(4, "Upper west rail", [-240.0, -579.0, 51.2], 0.4, 0.8, Timber),
    site!(4, "Upper loading bay", [-230.0, -570.0, 51.2], 3.1, 1.1, Machine),
    site!(4, "Upper north machinery", [-215.0, -555.0, 51.2], 1.4, 1.2, Machine),
    site!(4, "Upper east landing", [-192.0, -563.0, 51.2], 2.6, 1.05, Minecart),
    site!(4, "Broken high ramp", [-184.0, -575.0, 47.9], 3.8, 0.7, Timber),
    site!(4, "Middle ramp debris", [-185.0, -587.0, 41.9], 4.6, 0.65, Cargo),
    site!(4, "Ramp machinery", [-195.0, -598.0, 34.4], 0.9, 0.7, Machine),
    site!(4, "Low ramp foot", [-207.0, -600.0, 28.2], 2.1, 0.7, Timber),
    site!(4, "Gilnid's broken foundry", [-205.0, -592.0, 21.0], 0.2, 1.55, Machine),
    site!(4, "Lower west bay", [-219.0, -595.0, 21.8], 2.5, 1.05, Minecart),
    site!(4, "Lower east bay", [-181.0, -583.0, 19.3], 3.6, 1.05, Machine),
    site!(4, "Lower gate", [-175.0, -570.0, 19.3], 4.7, 0.9, Tunnel),
    site!(5, "Cannon gallery entrance", [-155.0, -585.0, 19.3], 2.0, 1.0, Cannon),
    site!(5, "Gallery upper corner", [-133.0, -591.0, 18.7], 3.7, 0.85, Timber),
    site!(5, "Marisa's side room", [-105.0, -616.0, 14.2], 1.2, 1.0, Cargo),
    site!(5, "Upper cannon approach", [-125.0, -610.0, 14.9], 2.9, 0.75, Tunnel),
    site!(5, "Middle cannon approach", [-126.0, -615.0, 13.7], 4.5, 0.7, Cannon),
    site!(5, "Lower cannon approach", [-119.0, -639.0, 11.4], 2.4, 0.85, Minecart),
    site!(5, "Cannon battery", [-110.0, -650.0, 7.2], 0.7, 1.15, Cannon),
    site!(5, "Battery east wall", [-100.0, -660.0, 7.4], 1.9, 0.9, Tunnel),
    site!(5, "Cove passage west", [-109.0, -691.0, 4.6], 4.4, 0.9, Timber),
    site!(5, "Cove passage east", [-100.0, -692.0, 8.2], 3.2, 0.8, Cargo),
    site!(5, "Cove side quarry", [-120.4, -702.1, 8.6], 0.4, 1.1, Tunnel),
    site!(5, "Cove passage bend", [-93.0, -716.0, 8.6], 1.6, 0.8, Tunnel),
    site!(6, "North cove wall", [-81.0, -726.0, 8.9], 3.1, 1.35, Shore),
    site!(6, "North cove cargo", [-57.0, -728.0, 8.9], 4.3, 1.05, Cargo),
    site!(6, "North east cave-in", [-38.4, -720.1, 8.4], 0.8, 1.3, Shore),
    site!(6, "Cove east rim", [-21.0, -742.0, 9.1], 2.0, 1.15, Shore),
    site!(6, "Smite's ruined landing", [-2.0, -768.0, 9.6], 3.5, 1.4, Cannon),
    site!(6, "North west waterline", [-92.4, -746.1, 2.8], 4.7, 1.3, Shore),
    site!(6, "West cove shorefall", [-132.4, -776.1, 0.16], 0.5, 1.25, Shore),
    site!(6, "West cove south shelf", [-160.0, -870.0, 9.8], 2.1, 1.3, Shore),
    site!(6, "West waterline", [-140.0, -870.0, 0.9], 4.0, 1.2, Shore),
    site!(6, "Pantry branch", [-158.6, -907.4, 8.8], 0.8, 1.0, Cargo),
    site!(6, "South west beach", [-140.0, -900.0, 6.8], 2.9, 1.2, Shore),
    site!(6, "South cove shelf", [-100.0, -900.0, 22.8], 4.3, 1.05, Shore),
    site!(7, "Cook's destroyed stores", [-63.0, -900.0, 16.0], 2.0, 1.2, Cargo),
    site!(7, "Beach slope", [-67.4, -914.1, 17.7], 3.4, 0.9, Shore),
    site!(7, "Landing ascent", [-74.0, -922.0, 32.0], 4.7, 0.75, Timber),
    site!(7, "Smugglers' lamp", [-80.0, -936.0, 40.0], 0.8, 0.9, Cargo),
    site!(7, "Sea tunnel mouth", [-85.0, -958.0, 42.5], 2.3, 1.0, Tunnel),
    site!(7, "Sea tunnel bend", [-94.0, -988.0, 36.0], 4.0, 1.1, Timber),
    site!(7, "Smugglers' cache", [-110.0, -1003.0, 34.0], 1.1, 1.1, Cargo),
];
