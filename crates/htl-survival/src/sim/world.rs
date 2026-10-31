//! The camp and the land: the storage chest, resource nodes, gathering, stations and their work,
//! placing new ones, mending, and the supplies a fallen hero left behind.

use std::collections::BTreeMap;

use super::*;

/// How close a hero must be to use something.
pub const INTERACT_RANGE: f32 = 4.5;
/// How far in front of a hero a station being placed stands.
pub const BUILD_DISTANCE: f32 = 4.0;
/// Seconds of cast for gathering (the bar the hero sees), by spell look.
const GATHER_SPELL: u32 = 2575;

/// A station a hero is placing.
#[derive(Debug, Clone, PartialEq)]
pub struct Preview {
    pub ghost: Id,
    pub def: usize,
    /// Turned this far from the hero's facing.
    pub turn: f32,
    pub pos: Vec3,
    pub facing: f32,
    /// Why it can't go here, if it can't.
    pub problem: Option<String>,
}

/// Where a station would go and whether it can, for the hero's screen.
#[derive(Debug, Clone, PartialEq)]
pub struct Placement {
    pub station: String,
    pub name: String,
    pub cost: String,
    pub problem: Option<String>,
    /// Where it would stand (its ghost), and how much ground it takes (yards round).
    pub at: Option<Vec3>,
    pub size: f32,
}

/// A chop's length: a gather that chops swings this often.
pub(crate) const CHOP_SECS: f32 = 0.8;

/// How far from a tree's middle what grows by it stands: clear of the widest trunk.
const BY_TRUNK: f32 = 3.4;
/// The room round a node's spot that must be clear of trunks and walls.
const CLEAR_ROUND: f32 = 1.3;

impl Survival {
    pub(crate) fn place_camp(&mut self) {
        let w = self.content.world.clone();
        let heart = self.ground(w.heart[0], w.heart[1]);
        let alarm = self.ground(w.alarm[0], w.alarm[1]);
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Storage, pos: heart, facing: w.spawn_facing });
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Alarm, pos: alarm, facing: w.spawn_facing });
        let rack = self.ground(w.rack[0], w.rack[1]);
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Rack, pos: rack, facing: w.spawn_facing });
        // The building ground ringed (torches), to see where it is: not on the road, nor in a
        // trunk, a wall or the water.
        if w.build_marker != 0 && w.build_radius > 0.0 {
            let r = w.build_radius + 1.0;
            let n = ((std::f32::consts::TAU * r / w.build_marker_every.max(2.0)).round() as usize).max(4);
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU;
                let (x, y) = (w.build_center[0] + a.cos() * r, w.build_center[1] + a.sin() * r);
                if self.land.road(x, y) > 0.2 || self.land.blocked(x, y) || self.land.water(x, y) {
                    continue;
                }
                let at = self.ground(x, y);
                let id = self.alloc();
                self.objs.insert(id, Obj { kind: ObjKind::Prop { display: w.build_marker, scale: 1.0 }, pos: at, facing: a });
            }
        }
        // The Smuggler's Logbook, whose pages go down to the Deadmines.
        if let Some(d) = self.content.dungeon.clone() {
            // Clear of the trees (a trunk hid it): nudged out into the open if one stands close.
            let mut spot = d.book;
            for t in self.land.trees() {
                let (dx, dy) = (spot[0] - t[0], spot[1] - t[1]);
                let dd = (dx * dx + dy * dy).sqrt();
                if dd < BOOK_CLEAR {
                    let k = if dd < 0.01 { 1.0 } else { (BOOK_CLEAR - dd) / dd };
                    spot = [spot[0] + dx * k + if dd < 0.01 { BOOK_CLEAR } else { 0.0 }, spot[1] + dy * k];
                }
            }
            let at = self.ground(spot[0], spot[1]);
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Prop { display: BOOK_CRATE_DISPLAY, scale: 1.0 }, pos: at, facing: d.book_facing });
            let id = self.alloc();
            let on = [at[0], at[1], at[2] + BOOK_CRATE_HEIGHT];
            self.objs.insert(id, Obj { kind: ObjKind::Book, pos: on, facing: d.book_facing });
            // A lit brazier beside it, to find it by.
            let (sx, sy) = ((d.book_facing + std::f32::consts::FRAC_PI_2).cos() * 1.6, (d.book_facing + std::f32::consts::FRAC_PI_2).sin() * 1.6);
            let beacon = self.ground(at[0] + sx, at[1] + sy);
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Prop { display: LAMP_LIT_DISPLAY, scale: 0.6 }, pos: beacon, facing: d.book_facing });
        }
    }

    /// Lay out the land's resource nodes: beside real trees in their areas, or at their points.
    pub(crate) fn place_nodes(&mut self, only: Option<usize>) {
        let trees = self.land.trees();
        for def in 0..self.content.nodes.len() {
            if only.is_some_and(|o| o != def) {
                continue;
            }
            let n = self.content.nodes[def].clone();
            let mut spots: Vec<Vec3> = Vec::new();
            // Beside real trees (and the given points) first; open ground in the areas only to
            // make up the count.
            let mut first: Vec<[f32; 2]> = n.points.clone();
            let mut fill: Vec<[f32; 2]> = Vec::new();
            // Down in a cave first: on its floor there, not the hill over it.
            for d in &n.deep {
                if spots.len() < n.count {
                    spots.push(self.ground_near(d[0], d[1], d[2] + 0.5));
                }
            }
            // Spread over the whole land: as evenly as the count allows.
            let spread = self.spread_spots(&n, &trees);
            for c in self.even_pick(spread, n.count, n.spacing) {
                spots.push(self.ground(c[0], c[1]));
            }
            for a in &n.areas {
                for t in trees.iter().filter(|t| dist2d(**t, [a.at[0], a.at[1], 0.0]) <= a.radius) {
                    // Beside the trunk, not in it (nor in the next one).
                    let ang = self.rng.gen::<f32>() * std::f32::consts::TAU;
                    let c = [t[0] + ang.cos() * BY_TRUNK, t[1] + ang.sin() * BY_TRUNK];
                    if self.clear_round(c) {
                        first.push(c);
                    }
                }
                for _ in 0..n.count * if n.shore { 200 } else { 3 } {
                    let ang = self.rng.gen::<f32>() * std::f32::consts::TAU;
                    let r = a.radius * self.rng.gen::<f32>().sqrt();
                    fill.push([a.at[0] + ang.cos() * r, a.at[1] + ang.sin() * r]);
                }
            }
            // A random order within each, so each world differs, then keep those far enough apart.
            for list in [&mut first, &mut fill] {
                for i in (1..list.len()).rev() {
                    let j = self.rng.gen_range(0..=i);
                    list.swap(i, j);
                }
            }
            let candidates: Vec<[f32; 2]> = first.into_iter().chain(fill).collect();
            for c in candidates {
                if spots.len() >= n.count {
                    break;
                }
                if self.land.blocked(c[0], c[1]) || self.land.water(c[0], c[1]) || self.land.road(c[0], c[1]) > 0.3 {
                    continue;
                }
                // A fishing school wants the water right there.
                if n.shore && !(0..8).any(|k| {
                    let a = k as f32 * std::f32::consts::FRAC_PI_4;
                    self.land.water(c[0] + a.cos() * 3.0, c[1] + a.sin() * 3.0)
                }) {
                    continue;
                }
                let at = self.ground(c[0], c[1]);
                if spots.iter().any(|s| dist2d(*s, at) < n.spacing) {
                    continue;
                }
                spots.push(at);
            }
            for (k, at) in spots.into_iter().enumerate() {
                let id = self.alloc();
                let facing = self.rng.gen::<f32>() * std::f32::consts::TAU;
                self.objs.insert(
                    id,
                    Obj {
                        kind: ObjKind::Node { def, key: format!("{}-{k}", n.id()), uses: n.uses, ready: true, back_at: 0.0 },
                        pos: at,
                        facing,
                    },
                );
                self.events.push(Event::ObjSpawned { id });
            }
        }
    }

    /// Nothing solid at a spot nor a stride round it (a trunk, a wall): room to stand at it.
    fn clear_round(&self, c: [f32; 2]) -> bool {
        !self.land.blocked(c[0], c[1])
            && (0..8).all(|k| {
                let a = k as f32 * std::f32::consts::FRAC_PI_4;
                !self.land.blocked(c[0] + a.cos() * CLEAR_ROUND, c[1] + a.sin() * CLEAR_ROUND)
            })
    }

    /// Up to `count` of the spots, as evenly over them as can be: each pick the one of a dozen
    /// tried that's farthest from those already picked, none closer than `spacing`.
    fn even_pick(&mut self, mut pool: Vec<[f32; 2]>, count: usize, spacing: f32) -> Vec<[f32; 2]> {
        let mut out: Vec<[f32; 2]> = Vec::new();
        let near = |out: &[[f32; 2]], c: [f32; 2]| out.iter().map(|o| (o[0] - c[0]).hypot(o[1] - c[1])).fold(f32::MAX, f32::min);
        while out.len() < count && !pool.is_empty() {
            let mut best: Option<(usize, f32)> = None;
            for _ in 0..12.min(pool.len()) {
                let k = self.rng.gen_range(0..pool.len());
                let d = near(&out, pool[k]);
                if best.is_none_or(|(_, b)| d > b) {
                    best = Some((k, d));
                }
            }
            let (k, d) = best.expect("a try");
            let c = pool.swap_remove(k);
            if d >= spacing {
                out.push(c);
            }
        }
        out
    }

    /// A kind spread across the whole zone: a spot in each square of the land (beside a trunk
    /// in it, for one that grows by trees), in a random order. None on a land with no extent.
    fn spread_spots(&mut self, n: &crate::content::NodeDef, trees: &[Vec3]) -> Vec<[f32; 2]> {
        self.spread_cells(n.spread, n.by_trees, trees)
    }

    /// A spot tried in each square `cell` yards a side over the whole land (beside a trunk in
    /// it, `by_trees`), that passes [`Self::spread_ok`], in a random order.
    pub(crate) fn spread_cells(&mut self, cell: f32, by_trees: bool, trees: &[Vec3]) -> Vec<[f32; 2]> {
        let Some((lo, hi)) = self.land.bounds().filter(|_| cell > 0.0) else { return Vec::new() };
        let (w, h) = (((hi[0] - lo[0]) / cell).ceil() as usize, ((hi[1] - lo[1]) / cell).ceil() as usize);
        // The trunks by square.
        let mut by_cell: BTreeMap<(usize, usize), Vec<Vec3>> = BTreeMap::new();
        if by_trees {
            for t in trees {
                if t[0] >= lo[0] && t[1] >= lo[1] {
                    let k = (((t[0] - lo[0]) / cell) as usize, ((t[1] - lo[1]) / cell) as usize);
                    by_cell.entry(k).or_default().push(*t);
                }
            }
        }
        let mut out = Vec::new();
        for i in 0..w {
            for j in 0..h {
                // A few tries in each square: the first that stands passes on to the checks.
                for _ in 0..3 {
                    let c = if by_trees {
                        let Some(ts) = by_cell.get(&(i, j)) else { break };
                        let t = ts[self.rng.gen_range(0..ts.len())];
                        let ang = self.rng.gen::<f32>() * std::f32::consts::TAU;
                        [t[0] + ang.cos() * BY_TRUNK, t[1] + ang.sin() * BY_TRUNK]
                    } else {
                        [lo[0] + (i as f32 + self.rng.gen::<f32>()) * cell, lo[1] + (j as f32 + self.rng.gen::<f32>()) * cell]
                    };
                    if self.spread_ok(c) {
                        out.push(c);
                        break;
                    }
                }
            }
        }
        for i in (1..out.len()).rev() {
            let j = self.rng.gen_range(0..=i);
            out.swap(i, j);
        }
        out
    }

    /// The `[[roam]]` groups made packs where they stand this world: spread over the land, apart
    /// from each other and from the packs already set down, clear of the camp.
    pub(crate) fn lay_roamers(&mut self) {
        let roams = self.content.roams.clone();
        if roams.is_empty() || self.land.bounds().is_none() {
            return;
        }
        let trees = self.land.trees();
        let heart = self.content.world.heart;
        let mut taken: Vec<[f32; 2]> = self.content.packs.iter().map(|p| p.at).collect();
        for r in roams {
            let pool: Vec<[f32; 2]> = self
                .spread_cells(r.spread, r.by_trees, &trees)
                .into_iter()
                .filter(|c| (c[0] - heart[0]).hypot(c[1] - heart[1]) > r.clear_of_camp)
                .filter(|c| taken.iter().all(|t| (c[0] - t[0]).hypot(c[1] - t[1]) > 25.0))
                .collect();
            for (k, at) in self.even_pick(pool, r.groups, r.spacing).into_iter().enumerate() {
                let mob = r.mobs[k % r.mobs.len()].clone();
                let count = self.rng.gen_range(r.size[0]..=r.size[1]);
                taken.push(at);
                self.content.packs.push(crate::content::PackDef {
                    mob,
                    at,
                    z: None,
                    radius: r.radius,
                    count,
                    link: 10.0,
                    route: Vec::new(),
                    wait: 8.0,
                    camp: String::new(),
                    key: String::new(),
                    elite: false,
                    hidden: 0.0,
                    wake: String::new(),
                    spots: Vec::new(),
                    rare: false,
                });
            }
        }
    }

    /// A spread node's spot: the zone's own ground, level enough to stand on, clear of the
    /// camp's grounds and the ring.
    fn spread_ok(&self, c: [f32; 2]) -> bool {
        let w = &self.content.world;
        if !self.land.in_zone(c[0], c[1]) || !self.clear_round(c) || self.land.water(c[0], c[1]) || self.land.road(c[0], c[1]) > 0.3 {
            return false;
        }
        let at = [c[0], c[1], 0.0];
        if dist2d(at, [w.build_center[0], w.build_center[1], 0.0]) < w.build_radius + 15.0 || dist2d(at, [w.heart[0], w.heart[1], 0.0]) < w.rest_radius + 15.0 {
            return false;
        }
        if self.content.arena.as_ref().is_some_and(|a| dist2d(at, [a.at[0], a.at[1], 0.0]) < a.radius + 10.0) {
            return false;
        }
        !self.steep_at(c[0], c[1], 2.0) && !self.up_on_something(c[0], c[1])
    }

    /// The ground rises or falls more than `rise` within a stride of here, some way.
    pub(crate) fn steep_at(&self, x: f32, y: f32, rise: f32) -> bool {
        let z = self.land.height(x, y);
        [(1.5f32, 0.0f32), (-1.5, 0.0), (0.0, 1.5), (0.0, -1.5)].iter().any(|(dx, dy)| (self.land.height(x + dx, y + dy) - z).abs() > rise)
    }

    /// Up on something no one walks onto (a roof, a rock's top): a sheer drop of more than 3
    /// yards within 15 nearly every way round.
    pub(crate) fn up_on_something(&self, x: f32, y: f32) -> bool {
        let z = self.land.height(x, y);
        let drops = (0..8)
            .filter(|k| {
                let a = *k as f32 * std::f32::consts::FRAC_PI_4;
                let mut last = z;
                (1..=10).any(|i| {
                    let d = i as f32 * 1.5;
                    let h = self.land.height(x + a.cos() * d, y + a.sin() * d);
                    let sheer = last - h > 2.4;
                    last = h;
                    sheer && z - h > 3.0
                })
            })
            .count();
        drops >= 6
    }

    pub(crate) fn step_nodes(&mut self) {
        let wall = self.wall();
        let mut back = Vec::new();
        for (&id, o) in self.objs.iter_mut() {
            if let ObjKind::Node { def, uses, ready, back_at, .. } = &mut o.kind {
                if !*ready && wall >= *back_at {
                    *ready = true;
                    *uses = self.content.nodes[*def].uses;
                    back.push(id);
                }
            }
        }
        for id in back {
            self.events.push(Event::ObjSpawned { id });
            self.events.push(Event::Dirty);
        }
    }

    /// What the use key would use now, and in a word what it does (for the on-screen prompt).
    pub fn use_prompt(&self, id: Id) -> Option<(Id, String, &'static str)> {
        let p = self.players.get(&id)?;
        if p.health == 0 {
            return None;
        }
        let (pos, f) = (p.pos, p.facing);
        let (oid, _, _) = self
            .objs
            .iter()
            .filter(|(oid, _)| self.object(**oid).is_some_and(|v| v.usable))
            .map(|(oid, o)| (*oid, dist2d(o.pos, pos), angle_between(f, facing(pos, o.pos))))
            .filter(|(_, d, a)| *d <= INTERACT_RANGE && (*a <= 100.0 || *d <= 2.5))
            .min_by(|a, b| a.1.total_cmp(&b.1))?;
        let v = self.object(oid)?;
        let verb = match &v.kind {
            ObjKind::Storage => "Bank and open",
            ObjKind::Alarm => "Ring",
            ObjKind::Node { .. } => "Gather",
            ObjKind::Station { damaged: true, .. } => "Mend",
            ObjKind::Station { .. } => "Use",
            ObjKind::Cache { .. } => "Take back",
            ObjKind::Rack => "Choose a weapon",
            ObjKind::Book => "Read",
            ObjKind::ArenaBanner => "Use",
            ObjKind::SiegeBanner => "Muster",
            ObjKind::SiegeLedger { .. } => "Open",
            ObjKind::SiegeTrainer { .. } => "Learn from",
            ObjKind::Lamp { .. } => "Light",
            ObjKind::Door { key } if self.door_def(key).is_some_and(|d| d.secret()) => "Search",
            ObjKind::Door { .. } | ObjKind::LiftEnd { .. } => "Open",
            ObjKind::Strongbox { .. } => "Open",
            ObjKind::Note { .. } => "Read",
            ObjKind::Chest { def, .. } if self.content.chests[*def].keep => "Use",
            ObjKind::Chest { .. } => "Open",
            ObjKind::Loot { .. } => "Pick up",
            _ => "Use",
        };
        Some((oid, v.name, verb))
    }

    /// Right-click or E: use the thing in front of the hero (or the one they clicked).
    pub fn interact(&mut self, id: Id, target: Option<Id>) {
        let Some(p) = self.players.get(&id) else { return };
        if p.health == 0 {
            return;
        }
        let (pos, f) = (p.pos, p.facing);
        let target = target.filter(|t| self.objs.get(t).is_some_and(|o| dist2d(o.pos, pos) <= INTERACT_RANGE + 1.5)).or_else(|| {
            self.objs
                .iter()
                .filter(|(oid, _)| self.object(**oid).is_some_and(|v| v.usable))
                .map(|(oid, o)| (*oid, dist2d(o.pos, pos), angle_between(f, facing(pos, o.pos))))
                .filter(|(_, d, a)| *d <= INTERACT_RANGE && (*a <= 100.0 || *d <= 2.5))
                .min_by(|a, b| a.1.total_cmp(&b.1))
                .map(|(oid, _, _)| oid)
        });
        let Some(t) = target else {
            self.tell(id, "Nothing to use here.");
            return;
        };
        if !matches!(self.players[&id].action, Action::Ready | Action::Guard | Action::Aim { .. }) {
            self.tell(id, "Finish what you're doing first.");
            return;
        }
        // A match's Hearth: ready (or not) for the next wave.
        if self.is_siege_hearth(t) {
            match self.siege_ready(id) {
                Ok(m) | Err(m) => self.tell(id, m),
            }
            return;
        }
        match self.objs[&t].kind.clone() {
            ObjKind::Storage => self.deposit_all(id),
            ObjKind::Alarm => self.ring_bell(id),
            ObjKind::Node { def, ready: true, .. } => {
                let secs = self.content.nodes[def].gather_secs;
                let spell = match self.content.nodes[def].gather_spell {
                    0 => GATHER_SPELL,
                    s => s,
                };
                self.set_action(id, Action::Gather { t: 0.0, node: t, at: pos });
                // Chopped with what's in hand (the first swing now, the rest as it goes), else the
                // spell's own look.
                match self.content.nodes[def].gather_clip {
                    0 => self.events.push(Event::Cast { caster: id, spell, ms: (secs * 1000.0) as u32 }),
                    clip => {
                        self.unsheathe(id);
                        self.events.push(Event::Clip { id, clip, secs: CHOP_SECS });
                    }
                }
            }
            ObjKind::Node { .. } => {}
            ObjKind::Station { damaged: true, .. } => self.repair(id, t),
            ObjKind::Station { def, health, .. } if self.content.stations[def].defense => {
                let d = &self.content.stations[def];
                let spikes = if d.spikes > 0 { " Its spikes bite whoever stands at it." } else { "" };
                self.tell(id, format!("The {}: {health} of {}. Raiders go for it before what it guards.{spikes}", d.name, d.health));
            }
            ObjKind::Station { .. } => self.events.push(Event::StationOpen { player: id, station: t }),
            ObjKind::Cache { .. } => self.claim(id, t),
            ObjKind::Rack | ObjKind::ArenaBanner | ObjKind::SiegeBanner | ObjKind::SiegeLedger { .. } | ObjKind::SiegeTrainer { .. } => self.events.push(Event::StationOpen { player: id, station: t }),
            ObjKind::Book if !self.book_open() && self.arena_def().is_none() => {
                let gate = self.dungeon_def().map_or(1, |d| d.gate_raids);
                self.tell(id, format!("The Smuggler's Logbook: a Defias smuggler's, its pages in a cipher. Hold the camp against {gate} raid(s) and the camp's scouts will have read it."));
            }
            ObjKind::Book => self.events.push(Event::StationOpen {
                player: id,
                station: t,
            }),
            ObjKind::Lamp { key } => self.use_lamp(id, &key, t),
            ObjKind::Door { key } => self.use_door(id, &key, t),
            ObjKind::LiftEnd { key } => self.ride(id, &key, false),
            ObjKind::Veil { .. } => {}
            ObjKind::Strongbox { key } => self.open_box(id, &key, t),
            ObjKind::Note { key } => self.read_note(id, &key),
            ObjKind::Chest { def, spent: false } => self.use_chest(id, def, t),
            ObjKind::Chest { .. } => {}
            ObjKind::Loot { item, n } => self.take_loot(id, item, n, t),
            ObjKind::Ghost { .. } | ObjKind::Fire { .. } | ObjKind::Prop { .. } => {}
        }
    }

    /// A gather finished: the node gives what it gives, and is spent after its uses.
    pub(crate) fn gathered(&mut self, id: Id, node: Id) {
        let Some(ObjKind::Node { def, uses, .. }) = self.objs.get(&node).map(|o| o.kind.clone()) else { return };
        let n = self.content.nodes[def].clone();
        let item = self.content.item(&n.item).unwrap();
        let amount = self.roll(n.yields);
        // A Siege vein pays gold, not ore.
        let vein = self.siege_def().map(|d| d.vein_gold).filter(|_| self.camp_of(id).is_some());
        let left = if vein.is_some() { 0 } else { self.give(id, item, amount) };
        let got = amount - left;
        if got == 0 {
            self.tell(id, "Your bags are full: bank what you carry at the camp's chest.");
            return;
        }
        let wall = self.wall();
        if let Some(Obj { kind: ObjKind::Node { uses: u, ready, back_at, .. }, .. }) = self.objs.get_mut(&node) {
            *u = uses.saturating_sub(1);
            if *u == 0 {
                *ready = false;
                *back_at = wall + n.respawn as f64;
            }
        }
        if uses <= 1 {
            self.events.push(Event::ObjGone { id: node });
        }
        self.count("gather", &n.item.clone(), got);
        if let Some(g) = vein {
            let gold = g * got;
            self.add_gold(id, gold);
            self.tell(id, format!("+{gold} gold from the vein."));
            self.events.push(Event::Flash { player: id, text: format!("+{gold} gold") });
            return;
        }
        let carried = self.players[&id].bag.get(&item).copied().unwrap_or(0);
        let mut words = vec![format!("+{got} {} ({carried} carried)", self.content.items[item].name)];
        // And whatever else it holds.
        for (k, range) in &n.loot {
            let Some(extra) = self.content.item(k) else { continue };
            let roll = self.roll(*range);
            if roll == 0 {
                continue;
            }
            let left = self.give(id, extra, roll);
            if roll > left {
                words.push(format!("+{} {}", roll - left, self.content.items[extra].name));
            }
        }
        self.tell(id, format!("{}.", words.join(", ")));
        self.events.push(Event::Dirty);
    }

    // --- the storage chest -----------------------------------------------------------------------

    /// Bank everything the hero carries for the camp (resources; food stays in the bags).
    pub fn deposit_all(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id) else { return };
        let moving: Vec<(usize, u32)> = p
            .bag
            .iter()
            .filter(|(i, n)| **n > 0 && self.content.items[**i].kind == ItemKind::Resource)
            .map(|(i, n)| (*i, *n))
            .collect();
        if moving.is_empty() {
            // Nothing to bank: just the chest's window.
            self.events.push(Event::StationOpen { player: id, station: self.storage_id() });
            return;
        }
        for (i, _) in &moving {
            p.bag.remove(i);
        }
        let mut words = Vec::new();
        for (i, n) in moving {
            *self.storage.entry(i).or_default() += n;
            words.push(format!("{n} {}", self.content.items[i].name));
            let key = self.content.items[i].key.clone();
            self.count("bank", &key, n);
        }
        self.tell(id, format!("Banked: {}.", words.join(", ")));
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Storage);
        self.events.push(Event::Dirty);
        self.events.push(Event::StationOpen { player: id, station: self.storage_id() });
    }

    pub fn storage_id(&self) -> Id {
        self.objs.iter().find(|(_, o)| o.kind == ObjKind::Storage).map_or(0, |(id, _)| *id)
    }

    /// Take `n` of item `key` from the chest into the bags (at the chest).
    pub fn withdraw(&mut self, id: Id, key: &str, n: u32) -> Result<(), String> {
        let item = self.content.item(key).ok_or("No such thing.")?;
        let chest = self.objs.get(&self.storage_id()).map(|o| o.pos).ok_or("No chest.")?;
        let p = self.players.get(&id).ok_or("No such hero.")?;
        if dist2d(p.pos, chest) > INTERACT_RANGE + 2.0 {
            return Err("Go to the camp's chest to take things out.".into());
        }
        let have = self.storage.get(&item).copied().unwrap_or(0);
        let n = n.min(have);
        if n == 0 {
            return Err("The chest has none.".into());
        }
        if !self.bag_fits(id, &BTreeMap::from([(item, n)])) {
            return Err("Your bags are full.".into());
        }
        let s = self.storage.get_mut(&item).unwrap();
        *s -= n;
        if *s == 0 {
            self.storage.remove(&item);
        }
        *self.players.get_mut(&id).unwrap().bag.entry(item).or_default() += n;
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Storage);
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// The chest's contents: (item key, name, count).
    pub fn storage_view(&self) -> Vec<(String, String, u32)> {
        self.storage.iter().map(|(i, n)| (self.content.items[*i].key.clone(), self.content.items[*i].name.clone(), *n)).collect()
    }

    // --- stations ----------------------------------------------------------------------------------

    /// What station `station` can make for hero `id`: (recipe key, output name, inputs as words,
    /// whether they have it all).
    pub fn recipes(&self, id: Id, station: Id) -> Vec<(String, String, String, bool)> {
        let Some(ObjKind::Station { def, .. }) = self.objs.get(&station).map(|o| &o.kind) else { return Vec::new() };
        let key = &self.content.stations[*def].key;
        self.content
            .recipes
            .iter()
            .filter(|r| &r.station == key)
            .map(|r| {
                let can = r.inputs.iter().all(|(k, n)| self.content.item(k).is_some_and(|i| self.available(id, i) >= *n));
                let out = &self.content.items[self.content.item(&r.output).unwrap()];
                let name = if r.count > 1 { format!("{} x{}", out.name, r.count) } else { out.name.clone() };
                (r.key.clone(), name, self.cost_text(&r.inputs), can)
            })
            .collect()
    }

    /// Make recipe `key` at station `station`: the work starts (its name back), and what it costs
    /// is paid and what it makes given as it's done (at once, with no `work.craft`).
    pub fn craft(&mut self, id: Id, station: Id, key: &str) -> Result<String, String> {
        let (name, secs, def) = self.craft_check(id, station, key)?;
        if secs <= 0.0 {
            self.craft_done(id, station, key)?;
            return Ok(name);
        }
        let pos = self.players[&id].pos;
        let spell = match self.content.stations[def].work_spell {
            0 => CRAFT_SPELL,
            s => s,
        };
        self.cancel_preview(id);
        self.set_action(id, Action::Work { t: 0.0, total: secs, job: Job::Craft { station, recipe: key.to_string() }, at: pos });
        self.events.push(Event::Cast { caster: id, spell, ms: (secs * 1000.0) as u32 });
        Ok(name)
    }

    /// Whether recipe `key` can be made at `station` now: its name, how long it takes, the
    /// station's kind.
    fn craft_check(&self, id: Id, station: Id, key: &str) -> Result<(String, f32, usize), String> {
        let c = &self.content;
        let recipe = c.recipes.iter().find(|r| r.key == key).ok_or("No such recipe.")?;
        let (def, damaged, at) = match self.objs.get(&station) {
            Some(Obj { kind: ObjKind::Station { def, damaged, .. }, pos, .. }) => (*def, *damaged, *pos),
            _ => return Err("That's not a station.".into()),
        };
        if c.stations[def].key != recipe.station {
            return Err(format!("That's made at a {}.", c.stations[c.station(&recipe.station).unwrap()].name));
        }
        if damaged {
            return Err("It's damaged: mend it first.".into());
        }
        let p = self.players.get(&id).ok_or("No such hero.")?;
        if p.health == 0 {
            return Err("You're down.".into());
        }
        if dist2d(p.pos, at) > INTERACT_RANGE + 2.0 {
            return Err("Go to the station to work it.".into());
        }
        let out = c.item(&recipe.output).unwrap();
        let kind = c.items[out].kind;
        let stackable = matches!(kind, ItemKind::Resource | ItemKind::Food);
        if stackable && !self.bag_fits(id, &BTreeMap::from([(out, recipe.count)])) {
            return Err("Your bags are full.".into());
        }
        self.affords(id, &recipe.inputs)?;
        let gear = if stackable { 1.0 } else { c.work.gear };
        Ok((c.items[out].name.clone(), c.work.craft * recipe.work * gear, def))
    }

    /// The making done: paid for, and what it makes in the bags (gear put on).
    fn craft_done(&mut self, id: Id, station: Id, key: &str) -> Result<(), String> {
        self.craft_check(id, station, key)?;
        let c = self.content.clone();
        let recipe = c.recipes.iter().find(|r| r.key == key).ok_or("No such recipe.")?;
        let out = c.item(&recipe.output).unwrap();
        let kind = c.items[out].kind;
        self.pay(id, &recipe.inputs)?;
        self.count("craft", &recipe.key, recipe.count);
        let name = c.items[out].name.clone();
        match kind {
            ItemKind::Resource | ItemKind::Food => {
                self.give(id, out, recipe.count);
                self.tell(id, format!("You make {} {name}.", recipe.count));
            }
            ItemKind::Weapon | ItemKind::Shield | ItemKind::Bow | ItemKind::Armor | ItemKind::Pack => {
                let old = self.put_on(id, out);
                self.events.push(Event::Gear { player: id });
                let what = c.items[out].text.clone();
                let verb = if matches!(kind, ItemKind::Armor | ItemKind::Pack) { "make the" } else { "forge the" };
                match old {
                    Some(old) => {
                        self.tell(id, format!("You {verb} {name} and put it on: {what} (Your {} goes in the chest.)", c.items[old].name));
                        *self.storage.entry(old).or_default() += 1;
                        self.events.push(Event::Storage);
                    }
                    None => self.tell(id, format!("You {verb} {name} and put it on: {what}")),
                }
                self.events.push(Event::Bags { player: id });
            }
        }
        self.events.push(Event::Missile { caster: id, spell: CRAFT_SPELL, target: Target::Unit(id) });
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// A hero's work at the camp done (its time stood out).
    pub(crate) fn work_done(&mut self, id: Id, job: Job) {
        let result = match job {
            Job::Craft { station, recipe } => self.craft_done(id, station, &recipe),
            Job::Build => self.build_done(id).map(|_| ()),
            Job::Repair { station } => self.repair_done(id, station),
        };
        if let Err(e) = result {
            self.tell(id, e);
        }
    }

    /// Wear or wield `item`: what it replaces, if anything.
    pub(crate) fn put_on(&mut self, id: Id, item: usize) -> Option<usize> {
        let d = self.content.items[item].clone();
        let p = self.players.get_mut(&id)?;
        match d.kind {
            ItemKind::Weapon => Some(std::mem::replace(&mut p.weapon, item)),
            ItemKind::Shield => Some(std::mem::replace(&mut p.shield, item)),
            ItemKind::Bow => Some(std::mem::replace(&mut p.bow, item)),
            ItemKind::Armor => p.armor.insert(d.slot.clone(), item),
            ItemKind::Pack => p.pack.replace(item),
            ItemKind::Resource | ItemKind::Food => None,
        }
    }

    /// Swap to gear kept in the chest (at the chest).
    pub fn equip_from_chest(&mut self, id: Id, key: &str) -> Result<(), String> {
        let item = self.content.item(key).ok_or("No such thing.")?;
        let kind = self.content.items[item].kind;
        if !matches!(kind, ItemKind::Weapon | ItemKind::Shield | ItemKind::Bow | ItemKind::Armor | ItemKind::Pack) {
            return Err("That isn't gear.".into());
        }
        if self.storage.get(&item).copied().unwrap_or(0) == 0 {
            return Err("The chest has none.".into());
        }
        let chest = self.objs.get(&self.storage_id()).map(|o| o.pos).ok_or("No chest.")?;
        if dist2d(self.players.get(&id).ok_or("No hero.")?.pos, chest) > INTERACT_RANGE + 2.0 {
            return Err("Go to the camp's chest.".into());
        }
        // A smaller pack can't hold what's carried.
        if kind == ItemKind::Pack {
            let p = &self.players[&id];
            if self.bag_slots_with(&p.bag, Some(item)) > self.bag_capacity(p) {
                return Err("Your bags hold more than that pack can: bank something first.".into());
            }
        }
        let old = self.put_on(id, item);
        let s = self.storage.get_mut(&item).unwrap();
        *s -= 1;
        if *s == 0 {
            self.storage.remove(&item);
        }
        if let Some(old) = old {
            *self.storage.entry(old).or_default() += 1;
        }
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Gear { player: id });
        self.events.push(Event::Storage);
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// Wear or wield gear carried in the bags (anywhere): what it replaces goes in the bags.
    pub fn wear_from_bag(&mut self, id: Id, item: usize) -> Result<String, String> {
        let d = self.content.items.get(item).ok_or("No such thing.")?.clone();
        if !matches!(d.kind, ItemKind::Weapon | ItemKind::Shield | ItemKind::Bow | ItemKind::Armor | ItemKind::Pack) {
            return Err("That isn't gear.".into());
        }
        let p = self.players.get(&id).ok_or("No hero.")?;
        if p.health == 0 {
            return Err("Not while down.".into());
        }
        if p.bag.get(&item).copied().unwrap_or(0) == 0 {
            return Err(format!("You carry no {}.", d.name));
        }
        let old = match d.kind {
            ItemKind::Weapon => Some(p.weapon),
            ItemKind::Shield => Some(p.shield),
            ItemKind::Bow => Some(p.bow),
            ItemKind::Armor => p.armor.get(&d.slot).copied(),
            _ => p.pack,
        };
        // The bags as they'd be: this one out, what it replaces in.
        let mut bag = p.bag.clone();
        if let Some(n) = bag.get_mut(&item) {
            *n -= 1;
        }
        if let Some(o) = old {
            *bag.entry(o).or_default() += 1;
        }
        let pack = if d.kind == ItemKind::Pack { Some(item) } else { p.pack };
        if self.bag_slots_with(&bag, pack) > self.bag_capacity(p) {
            return Err("Your bags are too full to swap that: bank something first.".into());
        }
        bag.retain(|_, n| *n > 0);
        self.put_on(id, item);
        if let Some(p) = self.players.get_mut(&id) {
            p.bag = bag;
        }
        let said = match old {
            Some(o) => format!("You put on the {}. (Your {} goes in your bags.)", d.name, self.content.items[o].name),
            None => format!("You put on the {}.", d.name),
        };
        self.tell(id, said.clone());
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Gear { player: id });
        self.events.push(Event::Dirty);
        Ok(said)
    }

    /// The weapon rack's id.
    pub fn rack_id(&self) -> Id {
        self.objs.iter().find(|(_, o)| o.kind == ObjKind::Rack).map_or(0, |(id, _)| *id)
    }

    /// What the rack offers: (item key, name, kind word, held now, what it's like).
    pub fn rack_view(&self, id: Id) -> Vec<(String, String, &'static str, bool, String)> {
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        self.content
            .items
            .iter()
            .enumerate()
            // A champion's trophy is won, not taken from the rack.
            .filter(|(_, d)| !self.content.is_trophy(&d.key))
            .filter_map(|(i, d)| {
                let kind = match d.kind {
                    ItemKind::Weapon if self.content.weapon(&d.key).is_some_and(|w| w.two_handed) => "two-handed",
                    ItemKind::Weapon => "one-handed",
                    ItemKind::Shield => "shield",
                    ItemKind::Bow => "ranged",
                    _ => return None,
                };
                let held = i == p.weapon || i == p.shield || i == p.bow;
                let text = match self.weapon_stats(&d.key) {
                    Some(stats) => format!("{} {stats}", d.text),
                    None => d.text.clone(),
                };
                Some((d.key.clone(), d.name.clone(), kind, held, text))
            })
            .collect()
    }

    /// A weapon's numbers in a line, so two can be told apart at a glance: its damage, how often
    /// a light blow comes, what that does a second, the heavy blow, its reach, and what's its own.
    pub fn weapon_stats(&self, key: &str) -> Option<String> {
        let c = &self.content;
        if let Some(w) = c.weapon(key) {
            let light = w.swing_secs(None);
            let heavy = w.swing_secs(Some(&c.heavy));
            let avg = (w.damage[0] + w.damage[1]) as f32 / 2.0;
            let mut perks = Vec::new();
            if w.stagger > 0.0 {
                perks.push(format!("knocks a wind-up aside for {:.1} s (not a boss's)", w.stagger));
            }
            if w.posture > 1.0 {
                perks.push(format!("breaks a stance {:.0}% faster", (w.posture - 1.0) * 100.0));
            }
            if w.parry > 0.0 {
                perks.push(format!("a parry {:.0} ms easier", w.parry * 1000.0));
            }
            if w.backstab > 0.0 {
                perks.push(format!("{:.0}x from behind", w.backstab));
            }
            if w.targets > 1 {
                perks.push(format!("strikes up to {}", w.targets));
            }
            let own = if perks.is_empty() { String::new() } else { format!(" Its own: {}.", perks.join(", ")) };
            return Some(format!(
                "{}-{} damage, a light blow every {light:.2} s ({:.0} a second), a heavy one {:.0}-{:.0} in {heavy:.2} s; reach {:.1} yd.{own}",
                w.damage[0],
                w.damage[1],
                avg / light.max(0.1),
                w.damage[0] as f32 * c.heavy.damage,
                w.damage[1] as f32 * c.heavy.damage,
                w.range
            ));
        }
        let r = c.ranged(key)?;
        Some(if r.draw_secs <= 0.0 {
            format!("Aimed Shot: {}-{} damage at once, {:.1} s to reload; {:.0} yd.", r.damage[0], r.damage[1], r.recovery, r.range)
        } else {
            format!("Aimed Shot: {} damage drawn briefly, up to {} at a full {:.1} s draw; {:.0} yd.", r.damage[0], r.damage[1], r.draw_secs, r.range)
        })
    }

    /// Take up a weapon, shield or bow from the rack (free while the armory is open).
    pub fn arm(&mut self, id: Id, key: &str) -> Result<String, String> {
        if !self.content.world.armory {
            return Err("The rack is empty.".into());
        }
        let item = self.content.item(key).ok_or("No such weapon.")?;
        if self.content.is_trophy(key) {
            return Err("That one's won from its owner, not taken from the rack.".into());
        }
        let kind = self.content.items[item].kind;
        let p = self.players.get(&id).ok_or("No such hero.")?;
        // The nearest rack (a Siege camp has its own).
        let rack = self.objs.values().filter(|o| o.kind == ObjKind::Rack).map(|o| o.pos).min_by(|a, b| dist2d(*a, p.pos).total_cmp(&dist2d(*b, p.pos))).ok_or("No rack.")?;
        if dist2d(p.pos, rack) > INTERACT_RANGE + 2.0 {
            return Err("Go to the weapon rack.".into());
        }
        if !matches!(p.action, Action::Ready | Action::Guard | Action::Aim { .. }) {
            return Err("Finish what you're doing first.".into());
        }
        let p = self.players.get_mut(&id).unwrap();
        match kind {
            ItemKind::Weapon => p.weapon = item,
            ItemKind::Shield => p.shield = item,
            ItemKind::Bow => p.bow = item,
            _ => return Err("That isn't a weapon.".into()),
        }
        // Whatever is up comes down with the swap.
        p.action = Action::Ready;
        p.guard_held = false;
        let name = self.content.items[item].name.clone();
        self.events.push(Event::Guard { player: id, up: false });
        self.events.push(Event::Gear { player: id });
        self.update_speed(id);
        self.events.push(Event::Dirty);
        Ok(name)
    }

    /// Mend a wrecked station: the work starts (it's paid for and mended as it's done).
    fn repair(&mut self, id: Id, station: Id) {
        let Some(ObjKind::Station { def, .. }) = self.objs.get(&station).map(|o| o.kind.clone()) else { return };
        let s = self.content.stations[def].clone();
        if let Err(e) = self.repair_check(id, station) {
            self.tell(id, e);
            return;
        }
        let secs = self.content.work.repair;
        if secs <= 0.0 {
            if let Err(e) = self.repair_done(id, station) {
                self.tell(id, e);
            }
            return;
        }
        let pos = self.players[&id].pos;
        self.set_action(id, Action::Work { t: 0.0, total: secs, job: Job::Repair { station }, at: pos });
        self.events.push(Event::Cast { caster: id, spell: GATHER_SPELL, ms: (secs * 1000.0) as u32 });
        self.tell(id, format!("You set to mending the {}.", s.name));
    }

    fn repair_check(&self, id: Id, station: Id) -> Result<(), String> {
        let Some(ObjKind::Station { def, damaged: true, .. }) = self.objs.get(&station).map(|o| &o.kind) else { return Err("There's nothing to mend.".into()) };
        let s = &self.content.stations[*def];
        if self.is_siege() && !self.buildable(*def) {
            return Err(format!("The {} can't be mended.", s.name));
        }
        if self.raid.active() && self.raid.raiders_near(self, station) {
            return Err("Not with raiders at it!".into());
        }
        let cost = self.station_repair(*def);
        self.affords(id, &cost).map(|_| ()).map_err(|e| format!("The {} is damaged. To mend it: {} ({e})", s.name, self.cost_text(&cost)))
    }

    fn repair_done(&mut self, id: Id, station: Id) -> Result<(), String> {
        self.repair_check(id, station)?;
        let Some(ObjKind::Station { def, .. }) = self.objs.get(&station).map(|o| o.kind.clone()) else { return Err("It's gone.".into()) };
        let s = self.content.stations[def].clone();
        self.pay(id, &self.station_repair(def))?;
        if let Some(Obj { kind: ObjKind::Station { health, damaged, .. }, .. }) = self.objs.get_mut(&station) {
            *health = s.health;
            *damaged = false;
        }
        self.put_out(station);
        self.events.push(Event::ObjChanged { id: station });
        self.tell(id, format!("You mend the {}.", s.name));
        self.events.push(Event::Dirty);
        Ok(())
    }

    pub(crate) fn hurt_station(&mut self, station: Id, damage: u32) {
        let Some(Obj { kind: ObjKind::Station { def, health, damaged, .. }, pos, .. }) = self.objs.get_mut(&station) else { return };
        if *damaged {
            return;
        }
        *health = health.saturating_sub(damage);
        if *health == 0 {
            *damaged = true;
            let (def, pos) = (*def, *pos);
            let name = self.content.stations[def].name.clone();
            let mend = self.cost_text(&self.content.stations[def].repair);
            let fire = self.alloc();
            self.objs.insert(fire, Obj { kind: ObjKind::Fire { on: station }, pos, facing: 0.0 });
            self.events.push(Event::ObjSpawned { id: fire });
            self.events.push(Event::ObjChanged { id: station });
            self.announce(format!("The {name} is wrecked! Mend it at the station with {mend}."));
            self.events.push(Event::Dirty);
        }
    }

    fn put_out(&mut self, station: Id) {
        let fires: Vec<Id> = self.objs.iter().filter(|(_, o)| o.kind == ObjKind::Fire { on: station }).map(|(id, _)| *id).collect();
        for f in fires {
            self.objs.remove(&f);
            self.events.push(Event::ObjGone { id: f });
        }
    }

    // --- placing stations --------------------------------------------------------------------------

    /// The stations a hero can build, for the build menu.
    pub fn build_menu(&self, id: Id) -> Vec<(String, String, String, bool, usize, String)> {
        self.content
            .stations
            .iter()
            .enumerate()
            .filter(|(k, _)| self.buildable(*k))
            .map(|(k, s)| {
                let cost = self.station_cost(k);
                let can = cost.iter().all(|(k, n)| self.content.item(k).is_some_and(|i| self.available(id, i) >= *n));
                let built = self.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { def, .. } if self.content.stations[def].key == s.key)).count();
                (s.key.clone(), s.name.clone(), self.cost_text(&cost), can, built, s.text.clone())
            })
            .collect()
    }

    /// Start placing station `key`: a ghost of it follows in front of the hero.
    pub fn build_start(&mut self, id: Id, key: &str) -> Result<(), String> {
        let def = self.content.station(key).filter(|d| self.buildable(*d)).ok_or("No such station.")?;
        let p = self.players.get(&id).ok_or("No such hero.")?;
        if p.health == 0 {
            return Err("You're down.".into());
        }
        let (pos, f) = (p.pos, p.facing);
        self.cancel_preview(id);
        let ghost = self.alloc();
        self.objs.insert(ghost, Obj { kind: ObjKind::Ghost { player: id, def, ok: false }, pos, facing: f });
        self.players.get_mut(&id).unwrap().preview = Some(Preview { ghost, def, turn: 0.0, pos, facing: f, problem: Some("…".into()) });
        self.place_preview(id, true);
        Ok(())
    }

    /// Turn the station being placed by `steps` eighths of a turn.
    pub fn build_rotate(&mut self, id: Id, steps: i32) {
        if let Some(pv) = self.players.get_mut(&id).and_then(|p| p.preview.as_mut()) {
            pv.turn = (pv.turn + steps as f32 * std::f32::consts::FRAC_PI_4).rem_euclid(std::f32::consts::TAU);
        }
        self.place_preview(id, true);
    }

    pub fn cancel_preview(&mut self, id: Id) {
        // Building it: that stops too.
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Work { job: Job::Build, .. })) {
            self.set_action(id, Action::Ready);
        }
        let Some(pv) = self.players.get_mut(&id).and_then(|p| p.preview.take()) else { return };
        self.objs.remove(&pv.ghost);
        self.events.push(Event::ObjGone { id: pv.ghost });
    }

    /// Place the station where the ghost stands, if it can go there and the hero can pay: the
    /// building starts there (the ghost stays put), and it stands, paid for, as it's done.
    pub fn build_confirm(&mut self, id: Id) -> Result<String, String> {
        if matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Work { job: Job::Build, .. })) {
            return Err("You're building it.".into());
        }
        self.place_preview(id, false);
        let pv = self.players.get(&id).and_then(|p| p.preview.clone()).ok_or("You aren't placing anything.")?;
        if let Some(problem) = &pv.problem {
            return Err(problem.clone());
        }
        let s = self.content.stations[pv.def].clone();
        self.affords(id, &self.station_cost(pv.def))?;
        let secs = self.content.work.build * s.work;
        if secs <= 0.0 {
            return self.build_done(id);
        }
        let pos = self.players[&id].pos;
        self.set_action(id, Action::Work { t: 0.0, total: secs, job: Job::Build, at: pos });
        self.events.push(Event::Cast { caster: id, spell: GATHER_SPELL, ms: (secs * 1000.0) as u32 });
        Ok(s.name)
    }

    /// The building done: the station stands where its ghost did, paid for.
    fn build_done(&mut self, id: Id) -> Result<String, String> {
        let pv = self.players.get(&id).and_then(|p| p.preview.clone()).ok_or("You aren't placing anything.")?;
        // Judged again where it stands (someone may have built there meanwhile).
        if let Some(problem) = self.site_problem(pv.def, pv.pos, Some(pv.ghost)) {
            return Err(problem);
        }
        let s = self.content.stations[pv.def].clone();
        self.pay(id, &self.station_cost(pv.def))?;
        self.cancel_preview(id);
        let uid = self.uid();
        let sid = self.alloc();
        self.objs.insert(sid, Obj { kind: ObjKind::Station { def: pv.def, uid, health: s.health, damaged: false }, pos: pv.pos, facing: pv.facing });
        // A match's station is its camp's.
        if let Some(camp) = self.camp_of(id) {
            if let Some(sg) = self.siege.as_mut() {
                sg.owner.insert(sid, camp);
            }
        }
        self.events.push(Event::ObjSpawned { id: sid });
        self.events.push(Event::Missile { caster: id, spell: CRAFT_SPELL, target: Target::Point(pv.pos) });
        let name = self.players[&id].name.clone();
        self.announce(format!("{name} builds a {}.", s.name));
        self.events.push(Event::Dirty);
        self.count("build", &s.key, 1);
        if self.raid.phase == RaidPhase::Idle && self.camp_ready() && self.next_raid().needs.contains(&s.key) {
            let title = self.next_raid().title;
            self.announce(format!("The camp is ready for the {title}. Ring the alarm bell when you're ready to defend it."));
        }
        Ok(s.name)
    }

    /// Where the station being placed would stand, and why it can't (if it can't).
    pub fn placement(&self, id: Id) -> Option<Placement> {
        let pv = self.players.get(&id)?.preview.as_ref()?;
        let s = &self.content.stations[pv.def];
        let at = self.objs.get(&pv.ghost).map(|o| o.pos);
        Some(Placement { station: s.key.clone(), name: s.name.clone(), cost: self.cost_text(&self.station_cost(pv.def)), problem: pv.problem.clone(), at, size: s.footprint.max(1.0) })
    }

    pub(crate) fn step_previews(&mut self) {
        // (Not while it's being built: the ghost stands where it's going up.)
        let ids: Vec<Id> = self.players.iter().filter(|(_, p)| p.preview.is_some() && !matches!(p.action, Action::Work { job: Job::Build, .. })).map(|(id, _)| *id).collect();
        for id in ids {
            self.place_preview(id, false);
        }
    }

    /// Move the ghost in front of the hero and judge the spot.
    fn place_preview(&mut self, id: Id, force: bool) {
        let Some(p) = self.players.get(&id) else { return };
        let Some(pv) = p.preview.clone() else { return };
        let f = p.facing;
        let at = [p.pos[0] + f.cos() * BUILD_DISTANCE, p.pos[1] + f.sin() * BUILD_DISTANCE];
        let pos = self.ground(at[0], at[1]);
        let facing_ = (f + std::f32::consts::PI + pv.turn).rem_euclid(std::f32::consts::TAU);
        let mut problem = self.site_problem(pv.def, pos, Some(pv.ghost));
        if problem.is_none() {
            let cost = self.station_cost(pv.def);
            let short: Vec<String> = cost
                .iter()
                .filter_map(|(k, n)| {
                    let i = self.content.item(k)?;
                    let have = self.available(id, i);
                    (have < *n).then(|| format!("{} more {}", n - have, self.content.items[i].name))
                })
                .collect();
            if !short.is_empty() {
                problem = Some(format!("Needs {}.", short.join(" and ")));
            }
        }
        let moved = dist2d(pos, pv.pos) > 0.4 || angle_between(facing_, pv.facing) > 1.0;
        let ok = problem.is_none();
        let changed = moved || ok != pv.problem.is_none() || force;
        if let Some(pp) = self.players.get_mut(&id).and_then(|p| p.preview.as_mut()) {
            pp.problem = problem;
            if changed {
                pp.pos = pos;
                pp.facing = facing_;
            }
        }
        if changed {
            if let Some(o) = self.objs.get_mut(&pv.ghost) {
                o.pos = pos;
                o.facing = facing_;
                o.kind = ObjKind::Ghost { player: id, def: pv.def, ok };
            }
            self.events.push(Event::ObjChanged { id: pv.ghost });
        }
    }

    /// Why station `def` can't stand at `pos`, if it can't: outside the camp, on the road, on
    /// steep or wet ground, too near another station or the camp's own spots.
    pub fn site_problem(&self, def: usize, pos: Vec3, ghost: Option<Id>) -> Option<String> {
        let w = &self.content.world;
        let s = &self.content.stations[def];
        let r = s.footprint;
        if self.is_siege() {
            let builder = ghost.and_then(|g| match self.objs.get(&g).map(|o| &o.kind) {
                Some(ObjKind::Ghost { player, .. }) => Some(*player),
                _ => None,
            });
            if let Some(why) = self.siege_site_problem(builder, pos) {
                return Some(why);
            }
        } else {
            let center = [w.build_center[0], w.build_center[1], 0.0];
            let off = dist2d(pos, center) + r - w.build_radius;
            if off > 0.0 {
                return Some(format!("Outside the camp's building ground (the meadow by the camp, ringed by torches): {:.0} yd {} of here.", off.ceil(), compass(facing(pos, center))));
            }
        }
        // Sample the footprint.
        let mut lo = f32::MAX;
        let mut hi = f32::MIN;
        for k in 0..13 {
            let (dx, dy) = if k == 0 {
                (0.0, 0.0)
            } else {
                let a = k as f32 / 12.0 * std::f32::consts::TAU;
                (a.cos() * r, a.sin() * r)
            };
            let (x, y) = (pos[0] + dx, pos[1] + dy);
            // (Arathi's camps stand in their own rubble: Siege builds on it.)
            if self.land.road(x, y) > 0.3 && !self.is_siege() {
                return Some("That's the road: keep it clear.".into());
            }
            if self.land.blocked(x, y) {
                return Some("Something's in the way.".into());
            }
            if self.land.water(x, y) {
                return Some("Too wet.".into());
            }
            let z = self.land.height(x, y);
            lo = lo.min(z);
            hi = hi.max(z);
        }
        if hi - lo > 1.6 {
            return Some("The ground's too steep.".into());
        }
        for (oid, o) in &self.objs {
            if Some(*oid) == ghost {
                continue;
            }
            let other = match &o.kind {
                ObjKind::Station { def, .. } => self.content.stations[*def].footprint,
                ObjKind::Storage | ObjKind::Alarm | ObjKind::Rack | ObjKind::Book | ObjKind::SiegeLedger { .. } | ObjKind::SiegeTrainer { .. } | ObjKind::SiegeBanner => w.keep_clear,
                ObjKind::Node { ready: true, .. } => 1.0,
                _ => continue,
            };
            if dist2d(o.pos, pos) < r + other + 0.5 {
                return Some("Too close to something else.".into());
            }
        }
        let spawn = [w.spawn[0], w.spawn[1], 0.0];
        if dist2d(spawn, pos) < r + w.keep_clear {
            return Some("That's where heroes arrive: keep it clear.".into());
        }
        None
    }

    /// The camp has what the next raid needs (standing).
    pub fn camp_ready(&self) -> bool {
        self.next_raid().needs.iter().all(|k| self.has_station(k))
    }

    /// A standing station of kind `key` is up.
    pub fn has_station(&self, key: &str) -> bool {
        self.objs.values().any(|o| matches!(o.kind, ObjKind::Station { def, damaged: false, .. } if self.content.stations[def].key == key))
    }

    /// One more towards a goal's count (`bank:wood`, `craft:roast`, `kill:kobold`...).
    pub(crate) fn count(&mut self, what: &str, key: &str, n: u32) {
        *self.camp.counts.entry(format!("{what}:{key}")).or_default() += n;
    }

    // --- what the fallen left ----------------------------------------------------------------------

    /// Take back what you dropped: all of it at once, or nothing if it won't all fit.
    fn claim(&mut self, id: Id, cache: Id) {
        let Some(ObjKind::Cache { owner, items, .. }) = self.objs.get(&cache).map(|o| o.kind.clone()) else { return };
        let me = self.players[&id].account.clone();
        if owner != me {
            self.tell(id, format!("Those are {owner}'s supplies."));
            return;
        }
        if !self.bag_fits(id, &items) {
            self.tell(id, "Your bags can't hold it all: bank or drop something first.");
            return;
        }
        self.objs.remove(&cache);
        self.events.push(Event::ObjGone { id: cache });
        let p = self.players.get_mut(&id).unwrap();
        for (i, n) in &items {
            *p.bag.entry(*i).or_default() += n;
        }
        let words: Vec<String> = items.iter().map(|(i, n)| format!("{n} {}", self.content.items[*i].name)).collect();
        self.tell(id, format!("You recover your supplies: {}.", words.join(", ")));
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Dirty);
    }

    /// The caches a hero has waiting, nearest first: where, and what.
    pub fn caches_of(&self, account: &str) -> Vec<(Id, Vec3)> {
        self.objs
            .iter()
            .filter_map(|(id, o)| match &o.kind {
                ObjKind::Cache { owner, .. } if owner == account => Some((*id, o.pos)),
                _ => None,
            })
            .collect()
    }
}

/// The look of work done (a station built, a thing made).
pub const CRAFT_SPELL: u32 = 3275;

/// A heading as a way on the compass (WoW's: north along +x, west along +y).
pub fn compass(heading: f32) -> &'static str {
    const WAYS: [&str; 8] = ["north", "north-west", "west", "south-west", "south", "south-east", "east", "north-east"];
    let k = (heading.rem_euclid(std::f32::consts::TAU) / std::f32::consts::FRAC_PI_4).round() as usize % 8;
    WAYS[k]
}
