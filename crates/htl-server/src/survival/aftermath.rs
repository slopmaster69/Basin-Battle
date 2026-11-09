//! The finished Deadmines, inspected independently of a boss fight. This is a temporary
//! presentation of the world: the rules, their clock, hero positions and saves stay intact.
//! Every visible ruin is a native, solid game object. The display-less marker asks client
//! patch 0087 to remove the ship's render groups AND colliders for this map only.

use std::collections::BTreeMap;

use htl_survival::Land;

use super::*;
use crate::packets::{Create, Placement};

#[path = "aftermath_layout.rs"]
mod layout;
#[cfg(test)]
#[path = "aftermath_tests.rs"]
mod tests;

pub(super) const MARKER_ENTRY: u32 = 500_100;
pub(super) const MARKER_GUID: u64 = 0xF110_07A184_FFFFFF;
const FIRST_ENTRY: u32 = MARKER_ENTRY + 1;
const INERT: u32 = 0x30;
const GENERIC: u32 = 5;
const BATCH: usize = 16;

pub(crate) struct Aftermath {
    props: Vec<Prop>,
    /// Inspectors move only here, never in the saved simulation.
    positions: BTreeMap<ConnId, ([f32; 3], f32)>,
    missing: Vec<&'static str>,
}

pub(super) struct Region {
    key: &'static str,
    name: &'static str,
    view: [f32; 3],
    facing: f32,
}

#[derive(Clone, Copy)]
pub(super) enum Dressing { Tunnel, Timber, Minecart, Machine, Cannon, Cargo, Shore }

pub(super) struct Site {
    region: usize,
    name: &'static str,
    at: [f32; 3],
    yaw: f32,
    scale: f32,
    dressing: Dressing,
}

#[derive(Debug, Clone, PartialEq)]
struct Prop {
    region: usize,
    site: &'static str,
    model: usize,
    pos: [f32; 3],
    scale: f32,
    rotation: [f32; 4],
    facing: f32,
}

/// Bounds measured from the installed native meshes. For tipped machinery, the transformed
/// eight corners determine the bottom; putting its original pivot on the floor would bury it.
struct Model {
    display: u32,
    name: &'static str,
    min: [f32; 3],
    max: [f32; 3],
}

const BOULDER: usize = 0;
const LOW_ROCK: usize = 1;
const DARK_ROCK: usize = 2;
const RUBBLE: usize = 3;
const LUMBER: usize = 4;
const WOODPILE: usize = 5;
const PLANK: usize = 6;
const CART: usize = 7;
const BARREL: usize = 8;
const CRATE: usize = 9;
const MACHINE: usize = 10;
const CANNON: usize = 11;
const BOAT: usize = 12;
const SUPPORT: usize = 13;
const CAVE_IN: usize = 14;

const MODELS: &[Model] = &[
    // Native check: the Westfall sea boulders read as luminous pale mounds indoors.
    // These rough dark meshes share the mine rubble's ashrock04 texture instead.
    Model { display: 5737, name: "Fallen Boulder", min: [-4.04,-4.73,-0.10], max: [4.08,4.31,9.42] },
    Model { display: 5741, name: "Broken Rock", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 5741, name: "Cave Rubble", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 5741, name: "Stone Fragments", min: [-2.93,-3.64,0.41], max: [2.62,4.05,3.61] },
    Model { display: 1109, name: "Broken Timbers", min: [-1.68,-0.91,0.0], max: [1.70,0.86,1.67] },
    Model { display: 6193, name: "Splintered Timber", min: [-3.0,-2.12,-0.02], max: [1.76,2.39,1.19] },
    Model { display: 6481, name: "Broken Plank", min: [-1.69,-0.32,-0.01], max: [0.87,0.80,0.22] },
    Model { display: 1887, name: "Wrecked Mine Cart", min: [-0.82,-1.06,-0.10], max: [1.25,1.05,1.60] },
    Model { display: 9, name: "Broken Barrel", min: [-1.23,-0.58,-0.04], max: [0.43,0.82,1.02] },
    Model { display: 3791, name: "Broken Crate", min: [-0.89,-1.05,-0.02], max: [1.25,0.85,1.31] },
    Model { display: 2029, name: "Toppled Machinery", min: [-1.30,-1.77,-0.02], max: [1.30,1.77,2.11] },
    Model { display: 245, name: "Overturned Cannon", min: [-4.72,-2.24,-0.01], max: [5.48,2.23,4.13] },
    Model { display: 223, name: "Shattered Boat", min: [-6.71,-1.77,-1.96], max: [2.79,2.24,2.32] },
    Model { display: 4651, name: "Fallen Support", min: [-0.28,-5.57,0.0], max: [0.22,5.82,2.29] },
    Model { display: 2230, name: "Cave-in", min: [-5.73,-8.31,-5.47], max: [5.77,8.25,7.35] },
];

fn rotation(roll: f32, pitch: f32, yaw: f32) -> [f32; 4] {
    let (sr, cr) = (roll * 0.5).sin_cos();
    let (sp, cp) = (pitch * 0.5).sin_cos();
    let (sy, cy) = (yaw * 0.5).sin_cos();
    [sr*cp*cy-cr*sp*sy, cr*sp*cy+sr*cp*sy, cr*cp*sy-sr*sp*cy, cr*cp*cy+sr*sp*sy]
}

fn bottom(model: usize, q: [f32; 4]) -> f32 {
    let m = &MODELS[model];
    let [x, y, z, w] = q;
    let row = [2.0*(x*z-y*w), 2.0*(y*z+x*w), 1.0-2.0*(x*x+y*y)];
    (0..8).map(|i| (0..3).map(|a| row[a] * if i & (1 << a) == 0 { m.min[a] } else { m.max[a] }).sum::<f32>())
        .fold(f32::INFINITY, f32::min)
}

fn floor(land: &dyn Land, at: [f32; 3], wet: bool) -> Option<f32> {
    let z = land.height_near(at[0], at[1], at[2]);
    // Never jump to the outdoor roof (~300 yd) or another Foundry floor. Blocked cells are
    // permitted for a cave-in's edge to enter a wall, but water is reserved for the cove.
    (z.is_finite() && (z-at[2]).abs() <= 3.0 && (wet || !land.water_near(at[0], at[1], z))).then_some(z)
}

fn make_prop(site: &Site, model: usize, pos: [f32; 3], scale: f32, roll: f32, pitch: f32, yaw: f32) -> Prop {
    let rotation = rotation(roll, pitch, yaw);
    // Terrain rocks have open ground-facing skirts. Keep those buried instead of exposing
    // them by seating only their single lowest rotated vertex on the floor.
    let embed = if model <= RUBBLE { 0.4 } else { 0.08 };
    // The cave-in's Closed pose is its bind pose, with chunks reaching 5.47 yd below its
    // origin. Lifting that extremity to the floor left a suspended wall in the native view.
    // Keep the composite's origin on the floor and let its lower chunks enter the ground.
    let z = if model == CAVE_IN { pos[2]-0.08*scale }
        else { pos[2] - bottom(model, rotation)*scale - embed*scale };
    Prop { region: site.region, site: site.name, model,
        pos: [pos[0], pos[1], z],
        scale, rotation, facing: yaw }
}

/// Test a 0.9 yd standing capsule against the actual oriented model box. The first native
/// check showed that a model's enclosing sphere cleared enormous clean disks around views,
/// even for a low plank or a tall rock whose solid body never reached the inspector.
fn covers_view(p: &Prop) -> bool {
    let m = &MODELS[p.model];
    let [qx,qy,qz,w] = p.rotation;
    let [x,y,z] = [-qx,-qy,-qz];
    layout::REGIONS.iter().any(|r| {
        [0.15,0.95,1.75].iter().any(|height| {
            let v = [(r.view[0]-p.pos[0])/p.scale, (r.view[1]-p.pos[1])/p.scale,
                (r.view[2]+height-p.pos[2])/p.scale];
            let t = [2.0*(y*v[2]-z*v[1]), 2.0*(z*v[0]-x*v[2]), 2.0*(x*v[1]-y*v[0])];
            let local = [v[0]+w*t[0]+y*t[2]-z*t[1], v[1]+w*t[1]+z*t[0]-x*t[2], v[2]+w*t[2]+x*t[1]-y*t[0]];
            let d2 = (0..3).map(|a| (local[a]-local[a].clamp(m.min[a],m.max[a])).powi(2)).sum::<f32>();
            d2 < (0.9/p.scale).powi(2)
        })
    })
}

fn fallen_rock_pose(index: usize) -> (f32, f32) {
    // Native inspection showed that steep rotations expose these models' open undersides.
    // Small leans fit within the buried skirt; broader pieces vary the pile's silhouette.
    match index % 5 {
        0 => (0.0, 0.0),
        1 => (0.0, 0.04),
        2 => (0.04, 0.0),
        3 => (0.0, -0.04),
        _ => (-0.03, 0.03),
    }
}

/// A site has a tall main fall, a lower shoulder, an apron, and room-specific wreckage.
/// The changing yaw, handedness and size are authored, never seeded randomly at load time.
fn compose(land: &dyn Land) -> (Vec<Prop>, Vec<&'static str>) {
    let mut props = Vec::new();
    let mut missing = Vec::new();
    for (index, site) in layout::SITES.iter().enumerate() {
        let Some(z) = floor(land, site.at, matches!(site.dressing, Dressing::Shore)) else {
            missing.push(site.name);
            continue;
        };
        let hand = if index % 2 == 0 { 1.0 } else { -1.0 };
        let (s, c) = site.yaw.sin_cos();
        let mut place = |model, offset: [f32; 2], scale, roll, pitch, yaw| {
            let at = [site.at[0] + (c*offset[0]-s*offset[1])*site.scale,
                site.at[1] + (s*offset[0]+c*offset[1])*site.scale, z];
            if let Some(z) = floor(land, at, matches!(site.dressing, Dressing::Shore)) {
                props.push(make_prop(site, model, [at[0], at[1], z], scale*site.scale, roll, pitch, site.yaw+yaw));
            }
        };
        let (roll, pitch) = fallen_rock_pose(index);
        if site.name == "Foreman's chamber" {
            // One bounded native appearance test before using this animated cave-in widely.
            place(CAVE_IN, [1.5, 1.3*hand], 0.9, 0.0, 0.0, 0.2);
        } else if index % 3 == 0 {
            place(BOULDER, [1.5, 1.3*hand], 0.82, roll, pitch, 0.2);
        } else {
            place(LOW_ROCK, [1.5, 1.3*hand], 1.5+0.2*(index%3) as f32, roll, pitch, 0.2);
        }
        place(LOW_ROCK, [-2.5, 0.9*hand], 0.95, 0.0, -0.03, 1.7);
        place(DARK_ROCK, [0.0, -2.5*hand], 0.65, 0.0, 0.03, 2.8);
        place(RUBBLE, [2.6, -2.1*hand], 0.3, 0.0, 0.0, 0.6);
        match site.dressing {
            Dressing::Tunnel | Dressing::Timber => {
                place(SUPPORT, [-0.8, -0.8*hand], 0.85, 0.12, 1.38, 0.5);
                place(LUMBER, [2.2, -2.8*hand], 1.25, 0.0, 0.15, 1.9);
                place(WOODPILE, [-2.8, -2.5*hand], 1.1, 0.0, 0.0, 0.0);
                place(PLANK, [0.5, -4.0*hand], 1.8, 0.09, 0.16, 2.5);
            }
            Dressing::Minecart => {
                place(CART, [-1.2, -2.0*hand], 1.5, 0.18, 0.12, 1.6);
                place(LUMBER, [2.2, -2.5*hand], 1.1, 0.0, 0.2, 2.4);
                place(WOODPILE, [-3.5, -1.0*hand], 1.0, 0.0, 0.0, 1.2);
                place(CRATE, [0.8, -4.0*hand], 1.4, 0.0, 0.0, 0.6);
            }
            Dressing::Machine => {
                place(MACHINE, [-2.2, -3.0*hand], 1.05, 1.32, 0.2, 0.8);
                place(CART, [2.5, -2.5*hand], 1.3, 0.2, 0.0, 2.0);
                place(SUPPORT, [-2.5, -1.5*hand], 0.9, 0.2, 1.5, 2.5);
                place(WOODPILE, [0.6, -4.0*hand], 1.3, 0.0, 0.0, 1.4);
            }
            Dressing::Cannon => {
                if site.name == "Smite's ruined landing" {
                    // The first native dock view was inside the large cannon's turned base.
                    // Keep a grounded wreck beside the approach at a readable size and lean.
                    place(CANNON, [3.5, -2.5*hand], 0.45, 0.55, 0.08, 0.9);
                } else {
                    place(CANNON, [-0.4, -1.8*hand], 0.7, 1.1, 0.15, 0.9);
                }
                place(BARREL, [2.8, -3.3*hand], 1.5, 0.2, 0.0, 2.4);
                place(CRATE, [-2.4, -3.0*hand], 1.4, 0.0, 0.0, 1.0);
                place(LUMBER, [0.0, -4.4*hand], 1.2, 0.0, 0.0, 2.8);
            }
            Dressing::Cargo => {
                place(CRATE, [-1.2, -2.0*hand], 1.8, 0.0, 0.1, 0.8);
                place(CRATE, [2.1, -2.7*hand], 1.3, 0.16, 0.0, 2.1);
                place(BARREL, [0.0, -4.0*hand], 1.6, 0.2, 0.0, 1.5);
                place(WOODPILE, [-3.4, -1.5*hand], 1.25, 0.0, 0.0, 2.8);
            }
            Dressing::Shore => {
                place(BOAT, [-1.0, -3.0*hand], 0.6, 0.2, 0.12, 1.1);
                place(SUPPORT, [2.0, -2.0*hand], 1.0, 0.12, 1.5, 2.4);
                place(WOODPILE, [-3.0, -2.0*hand], 1.8, 0.0, 0.0, 0.5);
                place(CRATE, [0.5, -4.5*hand], 1.5, 0.0, 0.2, 2.8);
            }
        }
    }
    // The damage runs into the foreground: shallow aprons between the original support
    // bases, with only the inspector's exact footing kept clear. No clean circular stage.
    for (region, r) in layout::REGIONS.iter().enumerate() {
        let apron = Site { region, name: "Rubble apron", at: r.view, yaw: r.facing, scale: 1.0, dressing: Dressing::Timber };
        let (s,c) = r.facing.sin_cos();
        for &(model, [dx,dy], scale, yaw) in layout::APRONS[region] {
            let at = [r.view[0]+c*dx-s*dy, r.view[1]+s*dx+c*dy, r.view[2]];
            if let Some(z) = floor(land, at, false) {
                props.push(make_prop(&apron, model, [at[0],at[1],z], scale, 0.0, 0.0, r.facing+yaw));
            }
        }
    }
    // The ship's old footprint is water over a seabed at -16 yd. These masses are seated
    // on that bottom and rise 17–25 yd above it; they are not floating on vanished decks.
    let crater = Site { region: 6, name: "Juggernaut collapse basin", at: [0.0; 3], yaw: 0.0, scale: 1.0, dressing: Dressing::Shore };
    for (at, scale, yaw) in [([-80.0,-820.0,-16.0], 8.8, 0.5), ([-60.0,-850.0,-16.0], 7.5, 2.3),
        ([-100.0,-850.0,-16.0], 7.1, 4.1), ([-60.0,-800.0,-16.0], 6.3, 1.2),
        ([-105.0,-821.0,-16.0], 8.1, 3.8)] {
        if let Some(z) = floor(land, at, true) {
            props.push(make_prop(&crater, LOW_ROCK, [at[0], at[1], z], scale, 0.0, 0.0, yaw));
            props.push(make_prop(&crater, LOW_ROCK, [at[0]+5.0, at[1]+3.0, z], scale*0.55, 0.0, 0.0, yaw+1.5));
        }
    }
    // Native DeadminesInstance_33_32 MCLQ: all 42 water chunks have surface -0.023 yd.
    // A sparse trail occupies the vanished decks and stores. These are floating fragments,
    // deliberately partly submerged, not a replacement walkable ship or a hidden old hull.
    const WATER: f32 = -0.023;
    for (name, x, y, yaw) in [
        ("East scaffold drift", -24.0, -832.0, 1.4),
        ("East stores drift", -34.0, -846.0, 3.2),
        ("Broken bow timbers", -51.0, -806.0, 0.8),
        ("Stern wreck trail", -88.0, -861.0, 4.1),
        ("Quartermaster's scattered cargo", -103.0, -828.0, 2.4),
    ] {
        if !land.water_near(x, y, -16.0) { continue; }
        let drift = Site { region: 6, name, at: [x,y,WATER], yaw, scale: 1.0, dressing: Dressing::Shore };
        let mut boat = make_prop(&drift, BOAT, [x,y,WATER-1.25], 1.1, 0.25, 0.12, yaw);
        boat.pos[2] -= 0.15;
        props.push(boat);
        props.push(make_prop(&drift, WOODPILE, [x+4.0,y-2.0,WATER-0.6], 1.4, 0.05, 0.0, yaw+1.2));
        props.push(make_prop(&drift, PLANK, [x-4.0,y+2.0,WATER-0.18], 2.3, 0.08, 0.0, yaw-0.4));
        props.push(make_prop(&drift, CRATE, [x+2.0,y+3.0,WATER-0.7], 1.1, 0.4, 0.15, yaw+2.0));
    }
    props.retain(|p| !covers_view(p));
    (props, missing)
}

fn prop_guid(index: usize, model: usize) -> u64 {
    (0xF110_u64 << 48) | (u64::from(FIRST_ENTRY + model as u32) << 24) | (0x80_0000 + index as u64)
}

fn go_kind(display: u32) -> u32 {
    // G_CaveIn has no Stand sequence. GOOBER's state1 holds Closed147 at creation; generic5
    // only plays a loader idle. Both types keep their solid collider and cannot be used here.
    if display == 2230 { 10 } else { GENERIC }
}

fn go_fields(guid: u64, entry: u32, display: u32, pos: [f32; 3], scale: f32, q: [f32; 4], facing: f32) -> Fields {
    let mut f = Fields::new();
    f.set_u64(idx::GUID, guid).set(idx::TYPE, fields::TYPEMASK_GAMEOBJECT).set(idx::ENTRY, entry)
        .set_f32(idx::SCALE_X, scale).set(idx::GAMEOBJECT_DISPLAYID, display)
        .set(idx::GAMEOBJECT_FLAGS, INERT).set(idx::GAMEOBJECT_TYPE_ID, go_kind(display))
        .set(idx::GAMEOBJECT_STATE, 1).set(idx::GAMEOBJECT_FACTION, 35).set(idx::GAMEOBJECT_ANIMPROGRESS, 100)
        .set_f32(idx::GAMEOBJECT_FACING, facing);
    for (i, value) in pos.iter().enumerate() { f.set_f32(idx::GAMEOBJECT_POS_X + i as u16, *value); }
    for (i, value) in q.iter().enumerate() { f.set_f32(idx::GAMEOBJECT_ROTATION + i as u16, *value); }
    f
}

impl SurvivalGame {
    pub(crate) fn aftermath_command(&mut self, conn: ConnId, arg: &str, target: &str) -> Result<(), String> {
        match arg {
            "on" => self.aftermath_start(conn),
            "off" => { self.aftermath_stop(); Ok(()) }
            "visit" => {
                if self.aftermath.is_none() { return Err("Start the preview with .es aftermath on.".into()); }
                let region = layout::REGIONS.iter().find(|r| r.key == target).ok_or("Choose a region with .es aftermath list.")?;
                let land = self.aftermath_land.as_deref().ok_or("The aftermath ground is unavailable.")?;
                let z = floor(land, region.view, false).ok_or("This view has no floor after the collapse.")?;
                self.aftermath_teleport(conn, [region.view[0], region.view[1], z+0.2], region.facing);
                self.say_to(conn, region.name);
                Ok(())
            }
            "point" => {
                if self.aftermath.is_none() { return Err("Start the preview with .es aftermath on.".into()); }
                let values: Vec<f32> = target.split_whitespace().map(str::parse::<f32>).collect::<Result<_, _>>()
                    .map_err(|_| "Use .es aftermath point <x> <y> <z> [facing].")?;
                if !(3..=4).contains(&values.len()) || values.iter().any(|v| !v.is_finite()) {
                    return Err("Use .es aftermath point <x> <y> <z> [facing].".into());
                }
                let at = [values[0], values[1], values[2]];
                let z = floor(self.aftermath_land.as_deref().unwrap(), at, false)
                    .ok_or("That point has no dry floor after the collapse.")?;
                self.aftermath_teleport(conn, [at[0], at[1], z+0.2], values.get(3).copied().unwrap_or(0.0));
                Ok(())
            }
            "list" => {
                self.say_to(conn, &format!("Inspect a region with .es aftermath visit <name>. {}.", layout::REGIONS.iter().map(|r| r.key).collect::<Vec<_>>().join(", ")));
                Ok(())
            }
            "" | "status" => {
                let text = self.aftermath.as_ref().map_or_else(|| "Aftermath preview is off.".into(), |a|
                    format!("Aftermath preview is on. {} solid objects across {} regions. {} sites lack a matching floor. Use .es aftermath off to restore the dungeon.", a.props.len(), layout::REGIONS.len(), a.missing.len()));
                self.say_to(conn, &text);
                Ok(())
            }
            _ => Err("Use .es aftermath on, off, list, status, visit <name> or point <x> <y> <z> [facing].".into()),
        }
    }

    fn aftermath_start(&mut self, conn: ConnId) -> Result<(), String> {
        if !self.sim.debug { return Err("Turn debug on first with .es debug quiet.".into()); }
        if self.realm != Realm::BELOW { return Err("Enter the Deadmines before starting its aftermath preview.".into()); }
        if self.aftermath.is_some() { return self.aftermath_command(conn, "status", ""); }
        if self.lab.is_some() || !self.mark_at.is_empty() || !self.casting.is_empty() || !self.area_looks.is_empty() || !self.stand_ins.is_empty() {
            return Err("Finish the current fight or lab scene before starting the aftermath preview.".into());
        }
        if self.aftermath_land.is_none() {
            let data = super::lab::data_dir().ok_or("Start the server with --data or WOW_DATA to inspect the aftermath.")?;
            let loaded = htl_map::deadmines::load_aftermath(&data).map_err(|e| format!("Couldn't load the collapsed Deadmines. {e}"))?;
            self.aftermath_land = Some(Box::new(super::land::DeadminesLand::new(loaded)));
        }
        let (props, missing) = compose(self.aftermath_land.as_deref().unwrap());
        if layout::REGIONS.iter().enumerate().any(|(r, _)| !props.iter().any(|p| p.region == r)) {
            return Err("The aftermath has an empty region. Check the installed map data before previewing it.".into());
        }
        println!("[aftermath] {} solids; {} sites unavailable: {}", props.len(), missing.len(), missing.join(", "));
        self.aftermath = Some(Aftermath { props, positions: BTreeMap::new(), missing });
        let hidden: Vec<u64> = self.sim.mobs.keys().chain(self.sim.objs.keys()).filter_map(|id| self.guid_of(*id)).collect();
        for guid in hidden { self.broadcast(&packets::destroy_object(guid)); }
        let viewers: Vec<ConnId> = self.in_world().map(|(c, _, _)| *c).collect();
        for c in viewers {
            if let Some(w) = self.world_mut(c) { w.aftermath_return = None; }
            self.send_aftermath_to(c);
            self.aftermath_arrival(c);
        }
        self.aftermath_command(conn, "status", "")
    }

    pub(crate) fn send_aftermath_to(&self, conn: ConnId) {
        let Some(scene) = self.aftermath.as_ref() else { return };
        self.send(conn, packets::gameobject_query_response(MARKER_ENTRY, GENERIC, 0, ""));
        let f = go_fields(MARKER_GUID, MARKER_ENTRY, 0, [0.0; 3], 1.0, [0.0, 0.0, 0.0, 1.0], 0.0);
        self.send(conn, packets::update_object(&[Create { guid: MARKER_GUID, object_type: packets::TYPE_GAMEOBJECT,
            is_self: false, placement: Placement::At { pos: [0.0; 3], facing: 0.0 }, fields: &f }], &[]));
        for (i, m) in MODELS.iter().enumerate() {
            self.send(conn, packets::gameobject_query_response(FIRST_ENTRY+i as u32, go_kind(m.display), m.display, m.name));
        }
        for (chunk_index, chunk) in scene.props.chunks(BATCH).enumerate() {
            let fields: Vec<Fields> = chunk.iter().enumerate().map(|(i, p)| go_fields(prop_guid(chunk_index*BATCH+i, p.model),
                FIRST_ENTRY+p.model as u32, MODELS[p.model].display, p.pos, p.scale, p.rotation, p.facing)).collect();
            let creates: Vec<Create> = chunk.iter().enumerate().map(|(i, p)| Create {
                guid: prop_guid(chunk_index*BATCH+i, p.model), object_type: packets::TYPE_GAMEOBJECT, is_self: false,
                placement: Placement::At { pos: p.pos, facing: p.facing }, fields: &fields[i],
            }).collect();
            self.send(conn, packets::update_object(&creates, &[]));
        }
    }

    pub(crate) fn aftermath_arrival(&mut self, conn: ConnId) {
        // A returning inspector might have logged out on the old ship. Start every inspection
        // at the retained smugglers' landing; the saved position is still used on exit.
        let _ = self.aftermath_command(conn, "visit", "landing");
        if let Some(a) = self.aftermath.as_ref() {
            for (other, (pos, facing)) in &a.positions {
                if *other != conn {
                    if let Some(w) = self.world_of(*other) { self.send(conn, packets::heartbeat(w.guid, *pos, *facing)); }
                }
            }
        }
    }

    pub(crate) fn aftermath_stop(&mut self) {
        let Some(scene) = self.aftermath.take() else { return };
        for (i, p) in scene.props.iter().enumerate() { self.broadcast(&packets::destroy_object(prop_guid(i, p.model))); }
        self.broadcast(&packets::destroy_object(MARKER_GUID));
        let viewers: Vec<(ConnId, Id)> = self.in_world().map(|(c, _, w)| (*c, w.id)).collect();
        for (conn, id) in viewers {
            if let Some(p) = self.sim.players.get(&id) { self.aftermath_teleport(conn, p.pos, p.facing); }
            self.send_world_to(conn, id);
            self.say_to(conn, "Restored the dungeon and your original position.");
        }
    }

    fn aftermath_teleport(&mut self, conn: ConnId, pos: [f32; 3], facing: f32) {
        let returning = self.aftermath.is_none();
        let Some(w) = self.world_mut(conn) else { return };
        w.counter = w.counter.wrapping_add(1);
        w.aftermath_return = returning.then_some(w.counter);
        w.fall_top = None;
        w.ground_z = pos[2];
        let (guid, counter) = (w.guid, w.counter);
        if let Some(a) = self.aftermath.as_mut() { a.positions.insert(conn, (pos, facing)); }
        self.send(conn, packets::teleport(guid, counter, pos, facing));
        self.broadcast_except(conn, &packets::heartbeat(guid, pos, facing));
    }

    pub(crate) fn aftermath_return_ack(&mut self, conn: ConnId, r: &mut Reader) -> std::io::Result<()> {
        if self.world_of(conn).and_then(|w| w.aftermath_return).is_none() { return Ok(()); }
        // The native client's reply is unpacked GUID, counter, client time (16 bytes).
        let guid = r.u64()?;
        let counter = r.u32()?;
        let _time = r.u32()?;
        if let Some(w) = self.world_mut(conn) {
            if w.guid == guid && w.aftermath_return == Some(counter) { w.aftermath_return = None; }
        }
        Ok(())
    }

    pub(crate) fn aftermath_movement(&mut self, conn: ConnId, pos: [f32; 3], facing: f32) -> bool {
        let Some(a) = self.aftermath.as_mut() else { return false };
        if pos.iter().all(|v| v.is_finite()) && facing.is_finite() { a.positions.insert(conn, (pos, facing)); }
        true
    }

    pub(crate) fn aftermath_where(&self, conn: ConnId) -> Option<([f32; 3], f32)> {
        self.aftermath.as_ref()?.positions.get(&conn).copied()
    }

    pub(crate) fn aftermath_query(&self, conn: ConnId, entry: u32) -> bool {
        if entry == MARKER_ENTRY {
            self.send(conn, packets::gameobject_query_response(entry, GENERIC, 0, ""));
            return true;
        }
        if let Some(m) = entry.checked_sub(FIRST_ENTRY).and_then(|i| MODELS.get(i as usize)) {
            self.send(conn, packets::gameobject_query_response(entry, go_kind(m.display), m.display, m.name));
            return true;
        }
        false
    }
}
