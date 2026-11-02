//! What the camp does with what it has built: moving a station, tearing one down for half its
//! cost, upgrading one into what it becomes, and the buildings' jobs (a barracks that gets the
//! fallen up sooner, an infirmary to rest at, a mill that fills the chest).

use super::world::{Preview, CRAFT_SPELL, INTERACT_RANGE};
use super::*;

/// Moving a station takes this share of building it.
const MOVE_WORK: f32 = 0.5;
/// Torn down, a station gives back this share of what it cost.
const REFUND: f32 = 0.5;

/// A station's own window: what can be done with it besides its work (`key~label~text~can`), and
/// its health.
#[derive(Debug, Clone, PartialEq)]
pub struct StationActions {
    pub actions: Vec<(String, String, String, bool)>,
    pub health: u32,
    pub max: u32,
}

impl Survival {
    /// Whether station `def` does the work of the stations of kind `key` (it is one, or an
    /// upgrade of one).
    pub fn station_does(&self, def: usize, key: &str) -> bool {
        let s = &self.content.stations[def];
        s.key == key || s.makes_as.iter().any(|k| k == key)
    }

    /// How many stations of kind `key` (or upgrades of it) stand.
    pub fn standing(&self, key: &str) -> usize {
        self.objs.values().filter(|o| matches!(o.kind, ObjKind::Station { def, .. } if self.station_does(def, key))).count()
    }

    fn station_at(&self, id: Id, station: Id) -> Result<(usize, Vec3), String> {
        let Some(Obj { kind: ObjKind::Station { def, .. }, pos, .. }) = self.objs.get(&station) else { return Err("That's not one of the camp's stations.".into()) };
        let p = self.players.get(&id).ok_or("No such hero.")?;
        if p.health == 0 {
            return Err("You're down.".into());
        }
        if dist2d(p.pos, *pos) > INTERACT_RANGE + 2.0 {
            return Err("Go to it first.".into());
        }
        if self.is_siege() || self.is_below() {
            return Err("Only at the camp.".into());
        }
        if self.raid.active() {
            return Err("Not during a raid.".into());
        }
        Ok((*def, *pos))
    }

    /// What a station's window offers besides its work: upgrading it, moving it, tearing it down.
    pub fn station_actions(&self, id: Id, station: Id) -> Option<StationActions> {
        let Some(Obj { kind: ObjKind::Station { def, health, .. }, .. }) = self.objs.get(&station) else { return None };
        let s = &self.content.stations[*def];
        let mut actions = Vec::new();
        if self.is_siege() || self.is_below() {
            return Some(StationActions { actions, health: *health, max: s.health });
        }
        if let Some(u) = &s.upgrade {
            if let Some(to) = self.content.station(&u.to) {
                let t = &self.content.stations[to];
                let cost = self.upgrade_cost(*def);
                let can = cost.iter().all(|(k, n)| self.content.item(k).is_some_and(|i| self.available(id, i) >= *n));
                actions.push(("upgrade".to_string(), format!("Upgrade to {}", t.name), format!("{} ({})", t.text, self.cost_text(&cost)), can));
            }
        }
        actions.push(("move".to_string(), "Move".to_string(), "Place it somewhere else in the camp. Nothing is lost.".to_string(), true));
        let back = self.refund(*def);
        let back = if back.is_empty() { "nothing".to_string() } else { self.cost_text(&back) };
        actions.push(("demolish".to_string(), "Tear Down".to_string(), format!("Returns 50% of its cost to storage: {back}."), true));
        Some(StationActions { actions, health: *health, max: s.health })
    }

    /// Half of what station `def` cost (its own cost: an upgrade's is what it all came to).
    fn refund(&self, def: usize) -> BTreeMap<String, u32> {
        self.content.stations[def]
            .cost
            .iter()
            .map(|(k, n)| (k.clone(), (*n as f32 * REFUND).floor() as u32))
            .filter(|(_, n)| *n > 0)
            .collect()
    }

    // --- moving -------------------------------------------------------------------------------

    /// Pick station `station` up: a ghost of it follows in front of the hero, and F sets it down
    /// there (nothing to pay, a little work), keeping its health; H calls it off.
    pub fn move_start(&mut self, id: Id, station: Id) -> Result<(), String> {
        let (def, _) = self.station_at(id, station)?;
        let p = &self.players[&id];
        let (pos, f) = (p.pos, p.facing);
        self.cancel_preview(id);
        let ghost = self.alloc();
        self.objs.insert(ghost, Obj { kind: ObjKind::Ghost { player: id, def, ok: false }, pos, facing: f });
        self.players.get_mut(&id).unwrap().preview = Some(Preview { ghost, def, turn: 0.0, pos, facing: f, problem: Some("…".into()), moving: Some(station) });
        self.place_preview_now(id);
        self.tell(id, format!("Moving the {}. Press (F) to place it, (R) or (T) to turn it, (H) to cancel.", self.content.stations[def].name));
        Ok(())
    }

    /// The move done: the station stands where its ghost did.
    pub(crate) fn move_done(&mut self, id: Id, pv: &Preview, station: Id) -> Result<String, String> {
        if !self.objs.contains_key(&station) {
            self.cancel_preview(id);
            return Err("It's gone.".into());
        }
        if let Some(problem) = self.site_problem_except(pv.def, pv.pos, Some(pv.ghost), Some(station)) {
            return Err(problem);
        }
        self.cancel_preview(id);
        if let Some(o) = self.objs.get_mut(&station) {
            o.pos = pv.pos;
            o.facing = pv.facing;
        }
        for f in self.objs.values_mut() {
            if f.kind == (ObjKind::Fire { on: station }) {
                f.pos = pv.pos;
            }
        }
        let fires: Vec<Id> = self.objs.iter().filter(|(_, o)| o.kind == ObjKind::Fire { on: station }).map(|(k, _)| *k).collect();
        self.events.push(Event::ObjChanged { id: station });
        for f in fires {
            self.events.push(Event::ObjChanged { id: f });
        }
        self.yielded.remove(&station);
        let name = self.content.stations[pv.def].name.clone();
        self.tell(id, format!("You moved the {name}."));
        self.events.push(Event::Dirty);
        Ok(name)
    }

    /// How long moving station `def` takes.
    pub(crate) fn move_secs(&self, def: usize) -> f32 {
        self.content.work.build * self.content.stations[def].work * MOVE_WORK
    }

    // --- tearing down -------------------------------------------------------------------------

    /// Tear station `station` down: asked first (`yes` false), then done (half its cost back to
    /// the chest).
    pub fn demolish(&mut self, id: Id, station: Option<Id>, yes: bool) -> Result<(), String> {
        let station = match (station, yes) {
            (_, true) => self.players.get(&id).and_then(|p| p.demolish).ok_or("Use the station you want to tear down first.")?,
            (Some(s), false) => s,
            (None, false) => return Err("Use the station you want to tear down first.".into()),
        };
        let (def, _) = self.station_at(id, station)?;
        let s = self.content.stations[def].clone();
        let back = self.refund(def);
        if !yes {
            self.players.get_mut(&id).unwrap().demolish = Some(station);
            let words = if back.is_empty() { "nothing".to_string() } else { self.cost_text(&back) };
            self.events.push(Event::Confirm {
                player: id,
                what: "demolish".into(),
                text: format!("Tear down the {}? 50% of its cost returns to storage: {words}.", s.name),
                yes: "Tear Down".into(),
                no: "Keep It".into(),
            });
            return Ok(());
        }
        self.players.get_mut(&id).unwrap().demolish = None;
        self.put_out(station);
        self.objs.remove(&station);
        self.yielded.remove(&station);
        self.events.push(Event::ObjGone { id: station });
        for (k, n) in &back {
            if let Some(i) = self.content.item(k) {
                *self.storage.entry(i).or_default() += n;
            }
        }
        self.events.push(Event::Storage);
        self.events.push(Event::Missile { caster: id, spell: CRAFT_SPELL, target: Target::Unit(id) });
        let name = self.players[&id].name.clone();
        self.announce(format!("{name} tore down the {}.", s.name));
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// The question about tearing down answered no.
    pub fn demolish_no(&mut self, id: Id) {
        if let Some(p) = self.players.get_mut(&id) {
            p.demolish = None;
        }
    }

    // --- upgrading ----------------------------------------------------------------------------

    /// Upgrade station `station` into what it becomes: the work starts, paid for as it's done.
    pub fn upgrade(&mut self, id: Id, station: Id) -> Result<String, String> {
        let (to, secs) = self.upgrade_check(id, station)?;
        let name = self.content.stations[to].name.clone();
        if secs <= 0.0 {
            self.upgrade_done(id, station)?;
            return Ok(name);
        }
        let pos = self.players[&id].pos;
        self.set_action(id, Action::Work { t: 0.0, total: secs, job: Job::Upgrade { station }, at: pos });
        self.events.push(Event::Cast { caster: id, spell: 2575, ms: (secs * 1000.0) as u32 });
        self.tell(id, format!("Upgrading it to a {name}."));
        Ok(name)
    }

    fn upgrade_check(&self, id: Id, station: Id) -> Result<(usize, f32), String> {
        let (def, _) = self.station_at(id, station)?;
        let s = &self.content.stations[def];
        let u = s.upgrade.as_ref().ok_or(format!("The {} is fully upgraded.", s.name))?;
        let to = self.content.station(&u.to).ok_or("No such station.")?;
        if matches!(self.objs.get(&station).map(|o| &o.kind), Some(ObjKind::Station { damaged: true, .. })) {
            return Err("It's damaged. Repair it first.".into());
        }
        let t = &self.content.stations[to];
        if t.most > 0 && self.standing(&t.key) as u32 >= t.most {
            return Err(format!("The camp already has a {}.", t.name));
        }
        // Its own ground grown: nothing in the way of the bigger one.
        let pos = self.objs[&station].pos;
        if let Some(problem) = self.site_problem_except(to, pos, None, Some(station)) {
            return Err(format!("No room for a {} here. {problem}", t.name));
        }
        let cost = self.upgrade_cost(def);
        self.affords(id, &cost).map_err(|e| format!("A {} costs {}. {e}", t.name, self.cost_text(&cost)))?;
        Ok((to, self.content.work.build * t.work))
    }

    pub(crate) fn upgrade_done(&mut self, id: Id, station: Id) -> Result<(), String> {
        let (to, _) = self.upgrade_check(id, station)?;
        let def = match self.objs.get(&station).map(|o| &o.kind) {
            Some(ObjKind::Station { def, .. }) => *def,
            _ => return Err("It's gone.".into()),
        };
        let cost = self.upgrade_cost(def);
        self.pay(id, &cost)?;
        let t = self.content.stations[to].clone();
        if let Some(Obj { kind: ObjKind::Station { def, health, damaged, .. }, .. }) = self.objs.get_mut(&station) {
            *def = to;
            *health = t.health;
            *damaged = false;
        }
        self.events.push(Event::ObjChanged { id: station });
        self.events.push(Event::Missile { caster: id, spell: CRAFT_SPELL, target: Target::Unit(id) });
        let name = self.players[&id].name.clone();
        self.announce(format!("{name} upgraded a station to a {}.", t.name));
        self.count("build", &t.key, 1);
        self.count("upgrade", &t.key, 1);
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// What upgrading station `def` costs (nothing with `free_build`).
    fn upgrade_cost(&self, def: usize) -> BTreeMap<String, u32> {
        if self.free_build {
            return BTreeMap::new();
        }
        self.content.stations[def].upgrade.as_ref().map(|u| u.cost.clone()).unwrap_or_default()
    }

    /// Testing: `n` more of everything any station, upgrade or mending asks for, into the
    /// camp's chest. What went in, by name.
    pub fn stock_for_building(&mut self, n: u32) -> Vec<String> {
        let mut keys: std::collections::BTreeSet<String> = Default::default();
        for s in &self.content.stations {
            keys.extend(s.cost.keys().cloned());
            keys.extend(s.repair.keys().cloned());
            if let Some(u) = &s.upgrade {
                keys.extend(u.cost.keys().cloned());
            }
        }
        let mut names = Vec::new();
        for k in keys {
            if let Some(i) = self.content.item(&k) {
                *self.storage.entry(i).or_default() += n;
                names.push(self.content.items[i].name.clone());
            }
        }
        self.events.push(Event::Storage);
        self.events.push(Event::Dirty);
        names
    }

    // --- the buildings' jobs --------------------------------------------------------------------

    /// Standing, whole buildings of the camp (not below, not in Siege), with their kind.
    fn buildings(&self) -> impl Iterator<Item = (Id, &crate::content::StationDef, Vec3)> {
        let here = !self.is_below() && !self.is_siege();
        self.objs.iter().filter(move |_| here).filter_map(|(id, o)| match o.kind {
            ObjKind::Station { def, damaged: false, .. } => Some((*id, &self.content.stations[def], o.pos)),
            _ => None,
        })
    }

    /// A fall's wait and the health a hero gets up with, as a barracks has them: (wait share,
    /// health more).
    pub(crate) fn barracks(&self) -> (f32, f32) {
        self.buildings().filter_map(|(_, s, _)| s.respawn).fold((1.0f32, 0.0f32), |(w, h), r| (w.min(1.0 / r.faster.max(0.1)), h.max(r.health)))
    }

    /// How much faster a hero at `pos` rests (at an infirmary), if they're at one.
    pub(crate) fn infirmary_at(&self, pos: Vec3) -> Option<f32> {
        self.buildings().filter_map(|(_, s, at)| s.rest.filter(|r| dist2d(at, pos) <= r.radius).map(|r| r.faster)).reduce(f32::max)
    }

    /// The mills put what they make into the chest (not during a raid).
    pub(crate) fn step_mills(&mut self) {
        if self.raid.active() || self.is_below() || self.is_siege() {
            return;
        }
        let mills: Vec<(Id, crate::content::YieldDef)> = self.buildings().filter_map(|(id, s, _)| s.yields.clone().map(|y| (id, y))).collect();
        let now = self.clock;
        let mut gave = false;
        for (id, y) in mills {
            let last = *self.yielded.entry(id).or_insert(now);
            if now - last < y.every.max(1.0) {
                continue;
            }
            self.yielded.insert(id, now);
            for (k, n) in &y.items {
                if let Some(i) = self.content.item(k) {
                    *self.storage.entry(i).or_default() += n;
                    gave = true;
                }
            }
        }
        if gave {
            self.events.push(Event::Storage);
            self.events.push(Event::Dirty);
        }
        self.yielded.retain(|id, _| matches!(self.objs.get(id).map(|o| &o.kind), Some(ObjKind::Station { .. })));
    }

    /// Where station `def` would stand placed near `at`: on its grid (and turned a quarter at a
    /// time), if it has one.
    pub(crate) fn snapped(&self, def: usize, at: [f32; 2], facing: f32) -> ([f32; 2], f32) {
        let snap = self.content.stations[def].snap;
        if snap <= 0.0 {
            return (at, facing);
        }
        let o = if self.is_siege() { [0.0, 0.0] } else { self.content.world.build_center };
        let g = |v: f32, o: f32| o + ((v - o) / snap).round() * snap;
        let q = std::f32::consts::FRAC_PI_2;
        ([g(at[0], o[0]), g(at[1], o[1])], ((facing / q).round() * q).rem_euclid(std::f32::consts::TAU))
    }
}
