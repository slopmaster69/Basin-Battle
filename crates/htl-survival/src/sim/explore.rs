//! Elwynn out past the camp: places found and marked, chests and caches (some hidden until a hero
//! comes upon them, some guarded by an ambush that wakes as they're opened, some shut until the
//! camp has beaten what guards them), shrines and relics whose cards the whole camp learns, and
//! rare creatures whose fall is told to everyone.

use super::*;

/// How often the land is looked over for chests filling again.
const REFILL_EVERY: f32 = 5.0;
/// A chest that is once a world: never full again.
pub const NEVER: f64 = 1.0e18;

/// A trial is under way while a hero stays this near its shrine or relic...
const TRIAL_STAY: f32 = 60.0;
/// ...and given up when none has been for this long.
const TRIAL_GIVE_UP: f32 = 12.0;

impl Survival {
    /// Set out the chests: those not opened (or full again), the secret ones kept back until found;
    /// a kept shrine that's spent stands spent.
    pub(crate) fn place_chests(&mut self) {
        if self.is_below() {
            return;
        }
        let wall = self.wall();
        for def in 0..self.content.chests.len() {
            let c = self.content.chests[def].clone();
            let opened = self.camp.opened.get(&c.key).is_some_and(|back| *back > wall);
            // A shrine or relic stands for every hero to learn from (a secret one, once found).
            let lasting = !c.teaches.is_empty();
            if opened && !c.keep && !lasting {
                continue;
            }
            if !opened && c.secret > 0.0 {
                self.unfound.insert(def);
                continue;
            }
            self.set_chest(def, opened && !lasting);
        }
    }

    /// Set out the land's props (fires, braziers, cages): only to be seen.
    pub(crate) fn place_props(&mut self) {
        if self.is_below() {
            return;
        }
        for k in 0..self.content.props.len() {
            let d = self.content.props[k].clone();
            let at = match d.z {
                Some(z) => self.ground_near(d.at[0], d.at[1], z),
                None => self.ground(d.at[0], d.at[1]),
            };
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Prop { display: d.display, scale: d.scale }, pos: at, facing: d.facing });
        }
    }

    /// Lay out the chests afresh (after a save is read: what it opened stays open).
    pub(crate) fn replace_chests(&mut self) {
        let old: Vec<Id> = self.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Chest { .. })).map(|(id, _)| *id).collect();
        for id in old {
            self.objs.remove(&id);
        }
        self.unfound.clear();
        self.place_chests();
    }

    fn set_chest(&mut self, def: usize, spent: bool) -> Id {
        let c = &self.content.chests[def];
        let at = match c.z {
            Some(z) => self.ground_near(c.at[0], c.at[1], z),
            None => self.ground(c.at[0], c.at[1]),
        };
        let facing = c.facing;
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Chest { def, spent }, pos: at, facing });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    /// Use a chest: shut until its guardian falls, else the work of opening it begins.
    pub(crate) fn use_chest(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        if !c.after.is_empty() && self.camp.counts.get(&format!("kill:{}", c.after)).copied().unwrap_or(0) == 0 {
            let who = self.content.mob(&c.after).map_or(c.after.clone(), |m| self.content.mobs[m].name.clone());
            let text = if c.locked.is_empty() { format!("{} won't open while {who} lives.", c.name) } else { c.locked.clone() };
            self.tell(id, text);
            return;
        }
        let pos = self.players[&id].pos;
        self.set_action(id, Action::Gather { t: 0.0, node: obj, at: pos });
        self.events.push(Event::Cast { caster: id, spell: CHEST_SPELL, ms: (c.secs * 1000.0) as u32 });
    }

    /// A chest opened: what's in it to the opener's bags (what won't fit, to the camp's chest), its
    /// ambush awake. A shrine or relic teaches the hero using it its cards, and stays for the next.
    pub(crate) fn open_chest(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        if !matches!(self.objs.get(&obj).map(|o| &o.kind), Some(ObjKind::Chest { spent: false, .. })) {
            return;
        }
        // One with a trial: its guardians first.
        if !c.teaches.is_empty() && !c.trial.is_empty() {
            self.start_trial(id, def, obj);
            return;
        }
        // A shrine or relic stays for every hero to learn from: what's in it, the first time only.
        if !c.teaches.is_empty() {
            let first = !self.camp.opened.contains_key(&c.key);
            self.camp.opened.insert(c.key.clone(), NEVER);
            if first {
                self.give_chest(id, &c);
                self.count("open", &c.key, 1);
                self.count("open", "any", 1);
            } else if !c.text.is_empty() {
                self.tell(id, c.text.clone());
            }
            let name = self.players[&id].name.clone();
            if self.teach(&[id], &c.teaches, &format!("{name} at {}", c.name)) == 0 {
                self.tell(id, format!("You know all {} has to teach.", c.name));
            }
            if first {
                for k in &c.ambush {
                    self.wake_ambush(k, id);
                }
            }
            self.events.push(Event::Sites);
            self.events.push(Event::Dirty);
            return;
        }
        let back = if c.respawn > 0.0 { self.wall() + c.respawn as f64 } else { NEVER };
        self.camp.opened.insert(c.key.clone(), back);
        if c.keep {
            if let Some(o) = self.objs.get_mut(&obj) {
                o.kind = ObjKind::Chest { def, spent: true };
            }
            // Drawn again, spent (not to be used).
            self.events.push(Event::ObjGone { id: obj });
            self.events.push(Event::ObjSpawned { id: obj });
        } else {
            self.objs.remove(&obj);
            self.events.push(Event::ObjGone { id: obj });
        }
        self.give_chest(id, &c);
        self.count("open", &c.key, 1);
        self.count("open", "any", 1);
        for k in &c.ambush {
            self.wake_ambush(k, id);
        }
        self.events.push(Event::Sites);
        self.events.push(Event::Dirty);
    }

    /// What's in a chest to the opener's bags (what won't fit, to the camp's chest), and its story.
    fn give_chest(&mut self, id: Id, c: &crate::content::ChestDef) {
        let mut items: BTreeMap<usize, u32> = BTreeMap::new();
        for (k, n) in &c.loot {
            if let Some(i) = self.content.item(k) {
                *items.entry(i).or_default() += n;
            }
        }
        for (k, r) in &c.roll {
            let n = self.roll(*r);
            if let (Some(i), true) = (self.content.item(k), n > 0) {
                *items.entry(i).or_default() += n;
            }
        }
        let mut words = Vec::new();
        let mut banked = Vec::new();
        for (i, n) in items {
            let left = self.give(id, i, n);
            if n > left {
                words.push(format!("{} {}", n - left, self.content.items[i].name));
            }
            if left > 0 {
                *self.storage.entry(i).or_default() += left;
                banked.push(format!("{left} {}", self.content.items[i].name));
            }
        }
        if !c.text.is_empty() {
            self.tell(id, c.text.clone());
        }
        if !words.is_empty() {
            self.tell(id, format!("In {}: {}.", c.name, words.join(", ")));
        }
        if !banked.is_empty() {
            self.tell(id, format!("Your bags were full: {} went to the camp's chest.", banked.join(", ")));
            self.events.push(Event::Storage);
        }
        self.events.push(Event::Bags { player: id });
    }

    /// These heroes learn cards (each hero's own, kept in their save): those they didn't know,
    /// with a banner each and the news for everyone. How many were new, in all.
    pub(crate) fn teach(&mut self, heroes: &[Id], keys: &[String], how: &str) -> usize {
        let mut all = 0;
        for &h in heroes {
            let Some(p) = self.players.get_mut(&h) else { continue };
            let new: Vec<String> = keys.iter().filter(|k| p.owned.insert((*k).clone())).cloned().collect();
            if new.is_empty() {
                continue;
            }
            all += new.len();
            let name = p.name.clone();
            let names: Vec<String> = new.iter().filter_map(|k| self.content.ability(k).map(|a| self.content.abilities[a].name.clone())).collect();
            let list = names.join(" and ");
            self.count("learn", "any", new.len() as u32);
            self.events.push(Event::Abilities { player: h });
            self.events.push(Event::Banner { player: Some(h), title: format!("Learned: {list}"), sub: "Open the board (N) to put it on a key".into() });
            self.announce(format!("{how}: {name} has learned {list}."));
        }
        if all > 0 {
            self.events.push(Event::Dirty);
        }
        all
    }

    /// A hero calls up a shrine's or a relic's trial: its guardians (those fallen since come again
    /// at once), on them. Nothing to learn, or one already under way: only words.
    fn start_trial(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        let knows = self.players.get(&id).is_some_and(|p| c.teaches.iter().all(|k| p.owned.contains(k)));
        if knows {
            self.tell(id, format!("You know all {} has to teach.", c.name));
            return;
        }
        if self.trials.contains_key(&def) {
            self.tell(id, format!("The trial is under way: defeat {}'s guardians.", c.name));
            return;
        }
        for key in &c.trial {
            let Some(k) = self.content.packs.iter().position(|p| p.key == *key) else { continue };
            let pack = self.content.packs[k].clone();
            let Some(mdef) = self.content.mob(&pack.mob) else { continue };
            // Those fallen in an earlier trial stand again now, not later.
            self.respawns.retain(|r| r.2 != Some(k));
            let have = self.mobs.values().filter(|m| m.pack == Some(k) && m.health > 0).count();
            for n in have..pack.count {
                let a = n as f32 / pack.count.max(1) as f32 * std::f32::consts::TAU;
                let r = if pack.count > 1 { pack.radius.max(1.0) } else { 0.0 };
                let mut at = self.ground_near(pack.at[0] + a.cos() * r, pack.at[1] + a.sin() * r, pack.z.unwrap_or(1.0e4));
                // One down in a cave stays on its floor; none in a wall.
                if pack.z.is_some_and(|z| (at[2] - z).abs() > 3.0) || self.land.blocked_near(at[0], at[1], at[2]) {
                    at = self.ground_near(pack.at[0], pack.at[1], pack.z.unwrap_or(1.0e4));
                }
                self.spawn_mob(mdef, at, Some(k), false);
            }
            self.wake_ambush(key, id);
        }
        let now = self.clock;
        self.trials.insert(def, Trial { obj, by: id, near_at: now });
        self.events.push(Event::Banner { player: Some(id), title: format!("The Trial of {}", c.name), sub: "Defeat its guardians to learn what it teaches".into() });
        self.tell(id, format!("{} wakes: its guardians come. Defeat them, and everyone near learns its cards.", c.name));
    }

    /// Trials under way: passed when the last guardian falls (everyone near learns, the first
    /// finder gets what's in it), given up when no hero has been near a while (the guardians go
    /// back to waiting, whole).
    pub(crate) fn step_trials(&mut self) {
        let now = self.clock;
        for def in self.trials.keys().copied().collect::<Vec<_>>() {
            let c = self.content.chests[def].clone();
            let Some(t) = self.trials.get(&def).cloned() else { continue };
            let Some(at) = self.objs.get(&t.obj).map(|o| o.pos) else {
                self.trials.remove(&def);
                continue;
            };
            let packs: Vec<usize> = c.trial.iter().filter_map(|key| self.content.packs.iter().position(|p| p.key == *key)).collect();
            let up: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| packs.contains(&k)) && m.health > 0 && !m.hidden).map(|(id, _)| *id).collect();
            if up.is_empty() {
                self.trials.remove(&def);
                let mut heroes: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= mobs::TEACH_RANGE).map(|(id, _)| *id).collect();
                if self.players.contains_key(&t.by) && !heroes.contains(&t.by) {
                    heroes.push(t.by);
                }
                for h in &heroes {
                    self.events.push(Event::Banner { player: Some(*h), title: "Trial passed".into(), sub: c.name.clone() });
                }
                if !self.camp.opened.contains_key(&c.key) && self.players.contains_key(&t.by) {
                    self.camp.opened.insert(c.key.clone(), NEVER);
                    self.give_chest(t.by, &c);
                    self.count("open", &c.key, 1);
                    self.count("open", "any", 1);
                }
                self.teach(&heroes, &c.teaches, &format!("The trial of {}", c.name));
                self.events.push(Event::Sites);
                self.events.push(Event::Dirty);
                continue;
            }
            let near = self.players.values().any(|p| p.health > 0 && dist2d(p.pos, at) <= TRIAL_STAY);
            if near {
                if let Some(t) = self.trials.get_mut(&def) {
                    t.near_at = now;
                }
            } else if now - t.near_at > TRIAL_GIVE_UP {
                self.trials.remove(&def);
                for m in up {
                    if let Some(x) = self.mobs.get_mut(&m) {
                        x.hidden = true;
                        x.health = x.max_health;
                        x.pos = x.home;
                        x.path.clear();
                        x.state = mobs::MobState::Idle { wander_at: f32::MAX };
                    }
                    self.events.push(Event::MobGone { id: m });
                }
            }
        }
    }

    /// Wake an ambush lying in wait (a pack by key) on a hero.
    pub(crate) fn wake_ambush(&mut self, key: &str, on: Id) {
        let Some(k) = self.content.packs.iter().position(|p| p.key == key) else { return };
        let ids: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.pack == Some(k) && m.hidden && m.health > 0).map(|(id, _)| *id).collect();
        for m in &ids {
            self.mobs.get_mut(m).unwrap().hidden = false;
            self.events.push(Event::MobSpawned { id: *m });
        }
        for m in ids {
            self.engage(m, mobs::Foe::Hero(on));
        }
    }

    /// Each tick: places found, secrets come upon, ambushers who see a hero, chests full again.
    pub(crate) fn step_explore(&mut self) {
        self.step_trials();
        let heroes: Vec<(Id, Vec3)> = self.players.iter().filter(|(_, p)| p.health > 0).map(|(id, p)| (*id, p.pos)).collect();
        if heroes.is_empty() {
            return;
        }
        // Places found.
        for k in 0..self.content.sites.len() {
            let site = &self.content.sites[k];
            if self.camp.found.contains(&site.key) {
                continue;
            }
            let at = [site.at[0], site.at[1], 0.0];
            let Some((by, _)) = heroes.iter().find(|(_, p)| dist2d(*p, at) <= site.radius) else { continue };
            self.found(*by, k);
        }
        // Secrets come upon.
        let unfound: Vec<usize> = self.unfound.iter().copied().collect();
        for def in unfound {
            let c = &self.content.chests[def];
            let at = [c.at[0], c.at[1], 0.0];
            let Some((by, _)) = heroes.iter().find(|(_, p)| dist2d(*p, at) <= c.secret) else { continue };
            let (by, name) = (*by, c.name.clone());
            self.unfound.remove(&def);
            self.set_chest(def, false);
            self.count("secret", "any", 1);
            self.events.push(Event::Banner { player: Some(by), title: "A secret!".into(), sub: name.clone() });
            self.tell(by, format!("You've come upon something hidden: {name}."));
            self.events.push(Event::Dirty);
        }
        // Ambushers lying in wait step out as a hero comes near.
        let waiting: Vec<(Id, Vec3, f32)> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.hidden && m.health > 0)
            .filter_map(|(id, m)| {
                let p = &self.content.packs[m.pack?];
                (p.wake.is_empty() && p.hidden > 0.0).then_some((*id, m.pos, p.hidden))
            })
            .collect();
        for (m, pos, r) in waiting {
            if let Some((h, _)) = heroes.iter().find(|(_, p)| dist2d(*p, pos) <= r) {
                if let Some(x) = self.mobs.get_mut(&m) {
                    x.hidden = false;
                }
                self.events.push(Event::MobSpawned { id: m });
                self.engage(m, mobs::Foe::Hero(*h));
            }
        }
        // Chests full again.
        if self.clock - self.chests_looked >= REFILL_EVERY {
            self.chests_looked = self.clock;
            let wall = self.wall();
            let due: Vec<usize> = (0..self.content.chests.len())
                .filter(|d| self.camp.opened.get(&self.content.chests[*d].key).is_some_and(|b| *b <= wall))
                .collect();
            for def in due {
                let c = self.content.chests[def].clone();
                // Not while a hero stands over it.
                let at = [c.at[0], c.at[1], 0.0];
                if heroes.iter().any(|(_, p)| dist2d(*p, at) < 20.0) {
                    continue;
                }
                self.camp.opened.remove(&c.key);
                let old: Vec<Id> = self.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, .. } if d == def)).map(|(id, _)| *id).collect();
                for o in old {
                    self.objs.remove(&o);
                    self.events.push(Event::ObjGone { id: o });
                }
                if c.secret > 0.0 {
                    self.unfound.insert(def);
                } else {
                    self.set_chest(def, false);
                }
                self.events.push(Event::Sites);
            }
        }
    }

    /// A place found: the camp's for good, a banner, and its worth into the chest.
    fn found(&mut self, by: Id, k: usize) {
        let site = self.content.sites[k].clone();
        self.camp.found.insert(site.key.clone());
        self.count("explore", "any", 1);
        self.count("explore", &site.kind, 1);
        if site.key != site.kind {
            self.count("explore", &site.key, 1);
        }
        for (i, n) in &site.reward {
            if let Some(i) = self.content.item(i) {
                *self.storage.entry(i).or_default() += n;
            }
        }
        let name = self.players.get(&by).map_or(String::new(), |p| p.name.clone());
        let n = self.camp.found.len();
        let of = self.content.sites.len();
        let reward = if site.reward.is_empty() { String::new() } else { format!(" Into the chest: {}.", self.cost_text(&site.reward)) };
        self.events.push(Event::Banner { player: None, title: format!("Discovered: {}", site.name), sub: format!("{n} of {of} places in Elwynn") });
        self.announce(format!("{name} has found {}.{reward}", site.name));
        if !site.reward.is_empty() {
            self.events.push(Event::Storage);
        }
        self.events.push(Event::Sites);
        self.events.push(Event::Dirty);
    }

    /// Testing (debug): set a hero down at one of Elwynn's places.
    pub fn goto_site(&mut self, id: Id, key: &str) -> Result<String, String> {
        let Some(site) = self.content.sites.iter().find(|s| s.key == key).cloned() else {
            let keys: Vec<&str> = self.content.sites.iter().map(|s| s.key.as_str()).collect();
            return Err(format!("No place {key}: try {}.", keys.join(", ")));
        };
        if self.is_below() || self.in_arena(id) {
            return Err("Not from here.".into());
        }
        let at = self.ground(site.at[0], site.at[1]);
        self.put_at(id, [at[0], at[1], at[2] + ARRIVE_LIFT], 0.0);
        self.land_from_far(id);
        Ok(site.name)
    }

    /// Every place with whether it's found, for the world map: key, name, where, kind, its words.
    pub fn sites_view(&self) -> Vec<(&crate::content::SiteDef, bool)> {
        self.content.sites.iter().map(|s| (s, self.camp.found.contains(&s.key))).collect()
    }

    /// The name of the place a spot is in or near (none past three of its radii).
    pub fn place_near(&self, at: [f32; 2]) -> Option<String> {
        self.content
            .sites
            .iter()
            .filter(|s| s.kind != "camp")
            .map(|s| (s, ((s.at[0] - at[0]).powi(2) + (s.at[1] - at[1]).powi(2)).sqrt() / s.radius.max(1.0)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, d)| *d < 3.0)
            .map(|(s, _)| s.name.clone())
    }

    /// Where in Elwynn the camp learns a card, in words: the shrine, relic or champion and the place
    /// it's at ("" for one that's free, or that nothing teaches).
    pub fn learned_where(&self, key: &str) -> String {
        if self.is_siege() {
            return self.card_price(key).map_or(String::new(), |g| format!("{g} gold"));
        }
        let c = &self.content;
        let place = |at: [f32; 2]| self.place_near(at);
        if let Some(ch) = c.chests.iter().find(|ch| ch.teaches.iter().any(|k| k == key)) {
            let at = place(ch.at).map_or(String::new(), |p| format!(", {p}"));
            return if ch.secret > 0.0 { format!("A hidden relic somewhere near{}", at.trim_start_matches(',')) } else { format!("{}{at}", ch.name) };
        }
        if let Some(m) = c.mobs.iter().position(|m| m.teaches.iter().any(|k| k == key)) {
            let at = c.packs.iter().find(|p| p.mob == c.mobs[m].key).and_then(|p| place(p.at)).map_or(String::new(), |p| format!(" ({p})"));
            return format!("When {} falls{at}", c.mobs[m].name);
        }
        String::new()
    }

    /// The chests and secrets the camp has opened, of all there are (secrets, of all secrets).
    pub fn chests_tally(&self) -> (usize, usize, usize, usize) {
        let c = &self.content.chests;
        let opened = |d: &crate::content::ChestDef| self.camp.opened.contains_key(&d.key) || self.camp.counts.contains_key(&format!("open:{}", d.key));
        let all = c.iter().filter(|d| d.secret <= 0.0).count();
        let open = c.iter().filter(|d| d.secret <= 0.0 && opened(d)).count();
        let secrets = c.iter().filter(|d| d.secret > 0.0).count();
        let found = c.iter().filter(|d| d.secret > 0.0 && opened(d)).count();
        (open, all, found, secrets)
    }
}

/// The look of a chest's lock being worked (Opening's cast).
pub const CHEST_SPELL: u32 = 21651;
