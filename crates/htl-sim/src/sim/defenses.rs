//! The Ancient's defences, bought with gold: Treant Protectors that hurl bolts at enemies in
//! reach, Healing Springs that mend whoever stands by them, and Basin Defenders who fight beside
//! them. They stand in the world, at the Ancient or at a post on the way in (the bridges), so the
//! base visibly grows over a run and players choose where to hold.

use super::cards::HitFx;
use super::enemies::Ally;
use super::*;

/// What the defences shop sells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Build {
    Protector,
    Spring,
    Defender,
    /// A dwarf with a rifle: shoots from behind the line.
    Rifleman,
    /// Heals the heroes and soldiers near them.
    Cleric,
    /// Every defence hits harder.
    Upgrade,
}

/// What kind of soldier an ally of the Ancient is, for its look.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoldierKind {
    /// A defender, at a rank (0 the Basin Defender, then each Promotion's).
    Defender(u8),
    Rifleman,
    Cleric,
}

/// The defences standing this run.
#[derive(Debug, Default)]
pub(super) struct Defenses {
    /// Treant Protectors, when each can throw again, and the spot each stands on.
    pub protectors: Vec<(EntityId, f32, usize)>,
    /// Springs and Defenders, with their spots.
    pub springs: Vec<(EntityId, usize)>,
    pub defenders: Vec<(EntityId, usize)>,
    pub riflemen: Vec<(EntityId, usize)>,
    pub clerics: Vec<(EntityId, usize)>,
    /// When each soldier can shoot or heal again.
    pub ready: BTreeMap<EntityId, f32>,
    /// How many have been torn down this run (for the shop's wording).
    pub fallen: u32,
    pub rank: u32,
    pub spring_tick: f32,
    pub spring_show: f32,
    /// Where each defence stands: 0 the Ancient, 1.. the arena's posts.
    pub site: BTreeMap<EntityId, usize>,
    /// Soldier upgrades bought (the shop's Soldiers tab), by rank: drill, muster, shield wall,
    /// deep roots, promotion.
    pub soldiers: [u8; 5],
}

impl Sim {
    /// A normal enemy's health this wave: what defence damage is measured against.
    pub(super) fn reference_health(&self) -> f32 {
        self.content.defenses.reference_health * self.health_mult()
    }

    fn defense_mult(&self) -> f32 {
        1.0 + self.content.defenses.upgrade_bonus * self.defenses.rank as f32
    }

    /// The price of the next of `what`, or `None` when no more can be built.
    pub(super) fn build_cost(&self, what: Build) -> Option<u32> {
        let d = &self.content.defenses;
        let n = match what {
            Build::Protector => return d.protector_costs.get(self.defenses.protectors.len()).map(|c| self.shared_price(*c)),
            Build::Spring => return d.spring_costs.get(self.defenses.springs.len()).map(|c| self.shared_price(*c)),
            Build::Defender => return self.defender_price(self.defenses.defenders.len()).map(|c| self.shared_price(c)),
            Build::Rifleman => return d.rifleman_costs.get(self.defenses.riflemen.len()).map(|c| self.shared_price(*c)),
            Build::Cleric => return d.cleric_costs.get(self.defenses.clerics.len()).map(|c| self.shared_price(*c)),
            Build::Upgrade => self.defenses.rank,
        };
        let built = !self.defenses.protectors.is_empty() || !self.defenses.springs.is_empty() || !self.soldier_ids().is_empty();
        built.then(|| self.shared_price((d.upgrade_cost as f32 * d.upgrade_growth.powi(n as i32)).round() as u32))
    }

    /// What the whole party shares (defences, the Ancient's upgrades) costs more for a bigger
    /// party, whose heroes each earn gold from every kill.
    pub(super) fn shared_price(&self, base: u32) -> u32 {
        let extra = self.player_count().saturating_sub(1) as f32;
        (base as f32 * (1.0 + self.content.defenses.party_price * extra)).round() as u32
    }

    /// Places defences can stand: the Ancient, then each post.
    pub fn site_count(&self) -> usize {
        1 + self.arena.posts.len()
    }

    /// "the Ancient", "the north bridge".
    pub fn site_name(&self, site: usize) -> String {
        match site {
            0 => format!("the {}", self.content.hub.name),
            k => self.arena.posts.get(k - 1).map_or_else(|| "the Ancient".to_string(), |p| p.name.clone()),
        }
    }

    /// The defences standing at `site`: (protectors, springs, defenders).
    pub fn site_defenses(&self, site: usize) -> (usize, usize, usize) {
        let at = |id: &EntityId| self.defenses.site.get(id).copied().unwrap_or(0) == site;
        (
            self.defenses.protectors.iter().filter(|p| at(&p.0)).count(),
            self.defenses.springs.iter().filter(|p| at(&p.0)).count(),
            self.soldier_ids().iter().filter(|p| at(p)).count(),
        )
    }

    /// Every soldier of the Ancient's: defenders, riflemen and clerics.
    pub(super) fn soldier_ids(&self) -> Vec<EntityId> {
        let d = &self.defenses;
        d.defenders.iter().chain(&d.riflemen).chain(&d.clerics).map(|x| x.0).collect()
    }

    /// What kind of soldier `id` is (for its look), if it's one.
    pub fn soldier_kind(&self, id: EntityId) -> Option<SoldierKind> {
        let d = &self.defenses;
        if d.defenders.iter().any(|x| x.0 == id) {
            Some(SoldierKind::Defender(self.defender_rank()))
        } else if d.riflemen.iter().any(|x| x.0 == id) {
            Some(SoldierKind::Rifleman)
        } else if d.clerics.iter().any(|x| x.0 == id) {
            Some(SoldierKind::Cleric)
        } else {
            None
        }
    }

    /// The defenders' rank: how many Promotions (0-3).
    pub fn defender_rank(&self) -> u8 {
        self.defenses.soldiers[4].min(self.content.defenses.ranks.len().saturating_sub(1) as u8)
    }

    /// How many enemies defence `id` holds at once: a promoted defender one more a rank.
    pub(super) fn holds(&self, id: EntityId) -> u32 {
        let hold = self.content.defenses.hold;
        if hold > 0 && self.defenses.defenders.iter().any(|x| x.0 == id) {
            hold + self.defender_rank() as u32
        } else {
            hold
        }
    }

    pub fn holds_for_test(&self, id: EntityId) -> u32 {
        self.holds(id)
    }

    pub fn site_of_soldier_for_test(&self, id: EntityId) -> usize {
        self.defenses.site.get(&id).copied().unwrap_or(0)
    }

    /// Send one soldier (from wherever has the most) to `site`: it walks there. Err when there
    /// are none to spare.
    pub(super) fn march(&mut self, id: EntityId, to: usize) -> Result<String, String> {
        if to >= self.site_count() {
            return Err("No such place.".into());
        }
        let mut by_site: BTreeMap<usize, Vec<EntityId>> = BTreeMap::new();
        for s in self.soldier_ids() {
            let at = self.defenses.site.get(&s).copied().unwrap_or(0);
            if at != to {
                by_site.entry(at).or_default().push(s);
            }
        }
        let Some((from, ids)) = by_site.into_iter().max_by_key(|(_, v)| v.len()) else {
            return Err(format!("No soldiers elsewhere to send to {}.", self.site_name(to)));
        };
        // A defender first, then a rifleman, then a cleric.
        let order = |s: &EntityId| match self.soldier_kind(*s) {
            Some(SoldierKind::Defender(_)) => 0,
            Some(SoldierKind::Rifleman) => 1,
            _ => 2,
        };
        let mut ids = ids;
        ids.sort_by_key(order);
        let sid = ids[0];
        self.defenses.site.insert(sid, to);
        let who = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.name.clone()).unwrap_or_default();
        let (from_name, to_name) = (self.site_name(from), self.site_name(to));
        self.announce(format!("{who} sends a soldier from {from_name} to {to_name}: they march there now."), false);
        Ok(format!("A soldier marches to {to_name}"))
    }

    /// Promote every defender a rank: a new name and look, harder and tougher.
    pub(super) fn promote_defenders(&mut self) {
        let olds: Vec<(EntityId, usize)> = self.defenses.defenders.clone();
        for (old, k) in olds {
            let Some(e) = self.entities.get(&old) else { continue };
            let (pos, share) = (e.pos, e.health as f32 / e.max_health.max(1) as f32);
            let site = self.defenses.site.remove(&old).unwrap_or(0);
            self.halt_enemy(old);
            self.entities.remove(&old);
            self.events.push(Event::Despawn { id: old });
            let new = self.spawn_defender(pos);
            if let Some(e) = self.entities.get_mut(&new) {
                e.health = ((e.max_health as f32 * share).round() as u32).clamp(1, e.max_health);
                let (health, max_health) = (e.health, e.max_health);
                self.events.push(Event::Health { id: new, health, max_health });
            }
            if let Some(d) = self.defenses.defenders.iter_mut().find(|d| d.0 == old) {
                *d = (new, k);
            }
            self.defenses.site.insert(new, site);
        }
    }

    /// The site a hub or post standard stands for.
    pub fn site_of(&self, entity: EntityId) -> usize {
        self.post_ids.iter().position(|&p| p == entity).map_or(0, |k| k + 1)
    }

    /// Where a defender stands guard: its post, or the Ancient.
    pub(super) fn defense_home(&self, id: EntityId) -> Option<Vec3> {
        let site = *self.defenses.site.get(&id)?;
        (site > 0).then(|| self.arena.posts.get(site - 1).map(|p| p.pos)).flatten()
    }

    /// The shopkeeper by the Ancient (once; they stay through every run).
    pub(super) fn spawn_vendor(&mut self, announce: bool) {
        if self.vendor.is_some_and(|v| self.entities.contains_key(&v)) {
            return;
        }
        let v = self.content.hub.vendor.clone();
        let id = self.alloc();
        let a = self.arena.hub_facing + v.angle.to_radians();
        let at = self.ring(self.arena.hub, v.distance, a);
        let f = facing(at, self.arena.hub) + std::f32::consts::PI;
        self.entities.insert(id, Entity { pos: at, facing: f, health: 1, max_health: 1, level: 30, body: Body::Npc { npc: NpcKind::Vendor, base: 0 } });
        if announce {
            self.events.push(Event::SpawnNpc { id, npc: NpcKind::Vendor, base: 0, pos: at, facing: f });
        }
        self.vendor = Some(id);
    }

    /// Whether hero `id` is at home, by the shopkeeper or the Ancient, where the shop sells.
    pub fn at_shop(&self, id: EntityId) -> bool {
        let Some(me) = self.entities.get(&id) else { return false };
        let reach = self.content.hub.vendor.reach;
        let near = |e: Option<&Entity>| e.is_some_and(|e| dist2d(e.pos, me.pos) <= reach);
        near(self.vendor.and_then(|v| self.entities.get(&v))) || near(self.entities.get(&self.hub))
    }

    /// "Bruni Ironhammer by the Ancient"
    pub fn vendor_name(&self) -> String {
        format!("{} by the {}", self.content.hub.vendor.name, self.content.hub.name)
    }

    /// The standards at the posts (again, after a reset).
    pub(super) fn spawn_posts(&mut self) {
        self.post_ids.clear();
        for (k, post) in self.arena.posts.clone().into_iter().enumerate() {
            let id = self.alloc();
            let npc = NpcKind::Post(k as u8);
            let pos = [post.pos[0], post.pos[1], self.ground.height(post.pos[0], post.pos[1])];
            self.entities.insert(id, Entity { pos, facing: post.facing, health: 1, max_health: 1, level: 1, body: Body::Npc { npc, base: k } });
            self.events.push(Event::SpawnNpc { id, npc, base: k, pos, facing: post.facing });
            self.post_ids.push(id);
        }
    }

    /// Where defence number `k` of a kind stands at `site`.
    fn site_spot(&self, site: usize, what: Build, k: usize) -> Vec3 {
        let d = &self.content.defenses;
        if site == 0 {
            return match what {
                Build::Protector => self.defense_spot(k as f32 * 90.0 + 45.0, d.protector_ring),
                Build::Spring => self.defense_spot(180.0 + if k % 2 == 0 { -35.0 } else { 35.0 }, d.spring_ring),
                Build::Rifleman => self.defense_spot(k as f32 * 90.0 + 15.0, 8.5),
                Build::Cleric => self.defense_spot(k as f32 * 180.0 + 90.0, 4.5),
                _ => self.defense_spot(k as f32 * 60.0 + 30.0, 6.0),
            };
        }
        let post = &self.arena.posts[site - 1];
        let (angle, r) = match what {
            // Treants either side of the road a step back from the water, springs behind,
            // defenders across the bridgehead.
            Build::Protector => ([-90.0f32, 90.0, -140.0, 140.0][k % 4], 6.0),
            Build::Spring => ([180.0f32, 160.0][k % 2], 8.0),
            // Riflemen a few steps behind the defenders, clerics behind them.
            Build::Rifleman => ([170.0f32, 190.0, 150.0, 210.0][k % 4], 5.0),
            Build::Cleric => ([180.0f32, 200.0][k % 2], 7.0),
            _ => ([0.0f32, 25.0, -25.0, 50.0, -50.0, 75.0][k % 6], 3.5),
        };
        self.ring(post.pos, r, post.facing + angle.to_radians())
    }

    /// The shop's lines, for the Ancient's menu: (text, what).
    pub(super) fn build_options(&self) -> Vec<(String, Build)> {
        let d = &self.content.defenses;
        let mut out = Vec::new();
        if let Some(c) = self.build_cost(Build::Protector) {
            out.push((
                format!(
                    "{} a Treant Protector ({}/{}): hurls bolts at enemies near the Ancient ({c} gold)",
                    if self.defenses.fallen > 0 { "Replant" } else { "Plant" },
                    self.defenses.protectors.len() + 1,
                    d.protector_costs.len()
                ),
                Build::Protector,
            ));
        }
        if let Some(c) = self.build_cost(Build::Spring) {
            out.push((
                format!(
                    "Raise a Healing Spring ({}/{}): heals everyone standing near it ({c} gold)",
                    self.defenses.springs.len() + 1,
                    d.spring_costs.len()
                ),
                Build::Spring,
            ));
        }
        if let Some(c) = self.build_cost(Build::Defender) {
            out.push((
                format!(
                    "Hire a Basin Defender ({}/{}): fights at the Ancient's feet ({c} gold)",
                    self.defenses.defenders.len() + 1,
                    self.defender_cap()
                ),
                Build::Defender,
            ));
        }
        if let Some(c) = self.build_cost(Build::Rifleman) {
            out.push((
                format!("Hire a Basin Rifleman ({}/{}): shoots from behind the line ({c} gold)", self.defenses.riflemen.len() + 1, d.rifleman_costs.len()),
                Build::Rifleman,
            ));
        }
        if let Some(c) = self.build_cost(Build::Cleric) {
            out.push((
                format!("Hire a Basin Cleric ({}/{}): heals heroes and soldiers near them ({c} gold)", self.defenses.clerics.len() + 1, d.cleric_costs.len()),
                Build::Cleric,
            ));
        }
        if let Some(c) = self.build_cost(Build::Upgrade) {
            out.push((
                format!(
                    "Fortify the defences to rank {}: +{}% damage and healing ({c} gold)",
                    self.defenses.rank + 2,
                    (d.upgrade_bonus * 100.0).round() as u32
                ),
                Build::Upgrade,
            ));
        }
        out
    }

    /// Buy `what` for player `id` at `site` (the gold is already paid).
    pub(super) fn build(&mut self, id: EntityId, what: Build, site: usize) {
        let who = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.name.clone()).unwrap_or_default();
        let d = self.content.defenses.clone();
        let site = if site < self.site_count() { site } else { 0 };
        let place = if site == 0 { "by the Ancient".to_string() } else { format!("at {}", self.site_name(site)) };
        let here = |list: &mut dyn Iterator<Item = (EntityId, usize)>, sites: &BTreeMap<EntityId, usize>| -> Vec<usize> {
            list.filter(|(e, _)| sites.get(e).copied().unwrap_or(0) == site).map(|(_, k)| k).collect()
        };
        match what {
            Build::Protector => {
                let taken = here(&mut self.defenses.protectors.iter().map(|p| (p.0, p.2)), &self.defenses.site);
                let k = free_spot(taken.into_iter());
                let pos = self.site_spot(site, what, k);
                let pid = self.spawn_defense(NpcKind::Protector, k, pos, d.protector_health);
                self.defenses.protectors.push((pid, self.clock + 1.0, k));
                self.defenses.site.insert(pid, site);
                self.announce(format!("{who} plants a Treant Protector {place}."), false);
            }
            Build::Spring => {
                let taken = here(&mut self.defenses.springs.iter().copied(), &self.defenses.site);
                let k = free_spot(taken.into_iter());
                let pos = self.site_spot(site, what, k);
                let sid = self.spawn_defense(NpcKind::Spring, k, pos, d.spring_health);
                self.defenses.springs.push((sid, k));
                self.defenses.site.insert(sid, site);
                self.announce(format!("{who} raises a Healing Spring {place}."), false);
            }
            Build::Defender => {
                let taken = here(&mut self.defenses.defenders.iter().copied(), &self.defenses.site);
                let k = free_spot(taken.into_iter());
                let pos = self.site_spot(site, what, k);
                let did = self.spawn_defender(pos);
                self.defenses.defenders.push((did, k));
                self.defenses.site.insert(did, site);
                let name = self.content.defenses.ranks.get(self.defender_rank() as usize).map_or("Basin Defender".to_string(), |r| r.name.clone());
                self.announce(format!("{who} hires a {name} {place}."), false);
            }
            Build::Rifleman | Build::Cleric => {
                let list = if what == Build::Rifleman { &self.defenses.riflemen } else { &self.defenses.clerics };
                let taken = here(&mut list.iter().copied(), &self.defenses.site);
                let k = free_spot(taken.into_iter());
                let pos = self.site_spot(site, what, k);
                let toughness = if what == Build::Rifleman { d.rifleman_health } else { d.cleric_health };
                let sid = self.spawn_soldier(pos, toughness, if what == Build::Rifleman { self.rifleman_hit() } else { 0.0 });
                if what == Build::Rifleman {
                    self.defenses.riflemen.push((sid, k));
                } else {
                    self.defenses.clerics.push((sid, k));
                }
                self.defenses.site.insert(sid, site);
                let name = if what == Build::Rifleman { "Basin Rifleman" } else { "Basin Cleric" };
                self.announce(format!("{who} hires a {name} {place}."), false);
            }
            Build::Upgrade => {
                self.defenses.rank += 1;
                self.refresh_defenses(false);
                let hub = self.hub;
                self.events.push(Event::SpellGo { caster: hub, spell: 635, target: SpellTarget::Unit(hub), hits: vec![hub] });
                self.announce(format!("{who} fortifies the Ancient's defences (rank {}).", self.defenses.rank + 1), false);
            }
        }
    }

    /// A defence was torn down: free its spot so it can be rebuilt.
    pub(super) fn defense_fell(&mut self, id: EntityId) {
        let what = if let Some(i) = self.defenses.protectors.iter().position(|p| p.0 == id) {
            self.defenses.protectors.remove(i);
            "A Treant Protector"
        } else if let Some(i) = self.defenses.springs.iter().position(|p| p.0 == id) {
            self.defenses.springs.remove(i);
            "A Healing Spring"
        } else if let Some(i) = self.defenses.defenders.iter().position(|p| p.0 == id) {
            self.defenses.defenders.remove(i);
            "A Basin Defender"
        } else if let Some(i) = self.defenses.riflemen.iter().position(|p| p.0 == id) {
            self.defenses.riflemen.remove(i);
            "A Basin Rifleman"
        } else if let Some(i) = self.defenses.clerics.iter().position(|p| p.0 == id) {
            self.defenses.clerics.remove(i);
            "A Basin Cleric"
        } else {
            return;
        };
        self.defenses.fallen += 1;
        let site = self.defenses.site.remove(&id).unwrap_or(0);
        let place = if site == 0 { String::new() } else { format!(" at {}", self.site_name(site)) };
        self.announce(format!("{what}{place} has been torn down! Rebuild it from the Shop."), true);
    }

    /// A point `r` yards from the hub at `angle_deg` from the way it faces.
    fn defense_spot(&self, angle_deg: f32, r: f32) -> Vec3 {
        let a = self.arena.hub_facing + angle_deg.to_radians();
        self.ring(self.arena.hub, r, a)
    }

    fn spawn_defense(&mut self, npc: NpcKind, k: usize, pos: Vec3, toughness: f32) -> EntityId {
        let id = self.alloc();
        let facing_ = facing(self.arena.hub, pos);
        let health = self.defense_health(toughness);
        self.entities.insert(id, Entity { pos, facing: facing_, health, max_health: health, level: 60, body: Body::Npc { npc, base: k } });
        self.events.push(Event::SpawnNpc { id, npc, base: k, pos, facing: facing_ });
        self.events.push(Event::Health { id, health, max_health: health });
        self.events.push(Event::SpellGo { caster: id, spell: 1126, target: SpellTarget::Unit(id), hits: vec![id] });
        id
    }

    fn defender_hit(&self) -> f32 {
        self.reference_health() * self.content.defenses.defender_damage * self.defense_mult() * self.drill_mult()
    }

    /// Drilled (and promoted) defenders hit harder and take more.
    pub(super) fn drill_mult(&self) -> f32 {
        1.0 + super::shop::SOLDIERS[0].per * self.defenses.soldiers[0] as f32 + super::shop::SOLDIERS[4].per * self.defender_rank() as f32
    }

    /// A rifleman's shot.
    pub(super) fn rifleman_hit(&self) -> f32 {
        self.reference_health() * self.content.defenses.rifleman_damage * self.defense_mult()
    }

    /// How many defenders can be hired: the price list, and two more for each Muster.
    pub(super) fn defender_cap(&self) -> usize {
        self.content.defenses.defender_costs.len() + (super::shop::SOLDIERS[1].per as usize) * self.defenses.soldiers[1] as usize
    }

    /// The price of defender `n` (0-based), past the list a little more each.
    fn defender_price(&self, n: usize) -> Option<u32> {
        let costs = &self.content.defenses.defender_costs;
        if n >= self.defender_cap() {
            return None;
        }
        costs.get(n).copied().or_else(|| costs.last().map(|c| (*c as f32 * 1.15f32.powi((n + 1 - costs.len()) as i32)).round() as u32))
    }

    /// A rifleman or a cleric: `toughness` normal enemies' healths, a hit of `hit`.
    fn spawn_soldier(&mut self, pos: Vec3, toughness: f32, hit: f32) -> EntityId {
        let id = self.alloc();
        let health = self.defense_health(toughness);
        let f = facing(self.arena.hub, pos);
        let owner = self.hub;
        let level = self.enemy_level().max(10);
        self.entities.insert(
            id,
            Entity {
                pos,
                facing: f,
                health,
                max_health: health,
                level,
                body: Body::Ally(Ally { owner, until: f32::INFINITY, mover: None, engaged: None, swing: 0.0, think: 0.0, hit, pet: false, blessed_until: 0.0 }),
            },
        );
        self.events.push(Event::SpawnAlly { id, owner, level, pos, facing: f, health, pet: false });
        id
    }

    fn spawn_defender(&mut self, pos: Vec3) -> EntityId {
        let id = self.alloc();
        let hit = self.defender_hit();
        let health = (self.defense_health(self.content.defenses.defender_health) as f32 * self.drill_mult()).round() as u32;
        let f = facing(self.arena.hub, pos);
        let owner = self.hub;
        self.entities.insert(
            id,
            Entity {
                pos,
                facing: f,
                health,
                max_health: health,
                level: self.enemy_level().max(10),
                body: Body::Ally(Ally { owner, until: f32::INFINITY, mover: None, engaged: None, swing: 0.0, think: 0.0, hit, pet: false, blessed_until: 0.0 }),
            },
        );
        self.events.push(Event::SpawnAlly { id, owner, level: self.enemy_level().max(10), pos, facing: f, health, pet: false });
        id
    }

    /// A defence's health: `toughness` normal enemies' healths at this wave.
    fn defense_health(&self, toughness: f32) -> u32 {
        (self.reference_health() * toughness * self.defense_mult().sqrt()).round().max(1.0) as u32
    }

    /// Defences keep pace with the waves and the upgrades; between waves (`mend`) they're
    /// patched up to full.
    pub(super) fn refresh_defenses(&mut self, mend: bool) {
        let d = self.content.defenses.clone();
        let hit = self.defender_hit();
        let mut all: Vec<(EntityId, f32)> = Vec::new();
        all.extend(self.defenses.protectors.iter().map(|p| (p.0, d.protector_health)));
        all.extend(self.defenses.springs.iter().map(|p| (p.0, d.spring_health)));
        let drill = self.drill_mult();
        all.extend(self.defenses.defenders.iter().map(|p| (p.0, d.defender_health * drill)));
        all.extend(self.defenses.riflemen.iter().map(|p| (p.0, d.rifleman_health)));
        all.extend(self.defenses.clerics.iter().map(|p| (p.0, d.cleric_health)));
        let rifle = self.rifleman_hit();
        let riflemen: Vec<EntityId> = self.defenses.riflemen.iter().map(|p| p.0).collect();
        for (id, toughness) in all {
            let max = self.defense_health(toughness);
            let Some(e) = self.entities.get_mut(&id) else { continue };
            if !e.alive() {
                continue;
            }
            let share = e.health as f32 / e.max_health.max(1) as f32;
            e.max_health = max;
            e.health = if mend { max } else { ((max as f32 * share).round() as u32).clamp(1, max) };
            if let Body::Ally(a) = &mut e.body {
                a.hit = if riflemen.contains(&id) { rifle } else { hit };
            }
            let (health, max_health) = (e.health, e.max_health);
            self.events.push(Event::Health { id, health, max_health });
        }
    }

    pub(super) fn step_defenses(&mut self, dt: f32) {
        if !self.combat_on() {
            return;
        }
        let now = self.clock;
        let d = self.content.defenses.clone();
        // Protectors.
        let roots = 1.0 + super::shop::SOLDIERS[3].per * self.defenses.soldiers[3] as f32;
        let bolt = self.reference_health() * d.protector_damage * self.defense_mult() * roots;
        for k in 0..self.defenses.protectors.len() {
            let (pid, ready, _) = self.defenses.protectors[k];
            if now < ready {
                continue;
            }
            let Some(pos) = self.entities.get(&pid).map(|e| e.pos) else { continue };
            // A marked or rallied-to enemy first.
            let pick = self.defence_focus(pos, d.protector_range).or_else(|| self.enemies_near(pos, d.protector_range).into_iter().next());
            let Some((target, dist)) = pick else {
                self.defenses.protectors[k].1 = now + 0.3;
                continue;
            };
            self.defenses.protectors[k].1 = now + d.protector_secs;
            self.events.push(Event::SpellGo { caster: pid, spell: d.protector_spell, target: SpellTarget::Unit(target), hits: vec![target] });
            self.queue_hit(pid, target, d.protector_spell, bolt, false, 3, dist / 24.0 + 0.1, HitFx::default());
        }
        // Springs: a heal every second, a visible pulse every few.
        if self.defenses.springs.is_empty() {
            return;
        }
        self.defenses.spring_tick += dt;
        if self.defenses.spring_tick < 1.0 {
            return;
        }
        self.defenses.spring_tick -= 1.0;
        let show = now >= self.defenses.spring_show;
        if show {
            self.defenses.spring_show = now + 3.0;
        }
        let mult = self.defense_mult();
        let mut mends: Vec<EntityId> = self.defenses.protectors.iter().map(|p| p.0).collect();
        mends.extend(self.defenses.springs.iter().map(|p| p.0));
        mends.extend(self.soldier_ids());
        for (sid, _) in self.defenses.springs.clone() {
            let Some(pos) = self.entities.get(&sid).map(|e| e.pos) else { continue };
            if show {
                self.events.push(Event::SpellGo { caster: sid, spell: d.spring_spell, target: SpellTarget::Unit(sid), hits: vec![sid] });
            }
            let hurt: Vec<(EntityId, u32)> = self
                .order
                .iter()
                .filter_map(|p| self.entities.get(p).map(|e| (*p, e)))
                .filter(|(_, e)| e.alive() && e.health < e.max_health && dist2d(e.pos, pos) <= d.spring_radius)
                .map(|(p, e)| (p, (e.max_health as f32 * d.spring_heal * mult).round().max(1.0) as u32))
                .collect();
            for (p, amount) in hurt {
                self.heal(sid, p, d.spring_spell, amount, false);
            }
            // Springs mend the other defences too, slowly.
            for &m in &mends {
                let Some(e) = self.entities.get(&m) else { continue };
                if e.alive() && e.health < e.max_health && dist2d(e.pos, pos) <= d.spring_radius * 1.5 {
                    let amount = (e.max_health as f32 * d.spring_heal * 0.4 * mult).round().max(1.0) as u32;
                    self.heal(sid, m, d.spring_spell, amount, false);
                }
            }
            // Only a spring by the Ancient reaches it.
            let hub = self.hub;
            let h = &self.entities[&hub];
            if self.defenses.site.get(&sid).copied().unwrap_or(0) == 0 && h.alive() && h.health < h.max_health {
                let amount = (h.max_health as f32 * d.spring_hub_heal * mult).round().max(1.0) as u32;
                self.heal(sid, hub, d.spring_spell, amount, false);
            }
        }
    }
}

/// The lowest spot number not taken.
fn free_spot(taken: impl Iterator<Item = usize>) -> usize {
    let taken: Vec<usize> = taken.collect();
    (0..).find(|k| !taken.contains(k)).unwrap()
}
