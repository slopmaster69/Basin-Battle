//! The outer bases: Blight Gates with guardians, relic caches, banners to claim, the boons they
//! give, and the Escalation once all are taken. Also the Ancient's forge.

use rand::seq::SliceRandom;
use rand::Rng;

use super::cards::{MenuAction, OfferSource};
use super::players::ChannelKind;
use super::shop::capitalize;
use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BaseState {
    /// Not open yet (the first waves).
    Locked,
    /// Guardians stand at the gate.
    Guarded,
    /// The guardians are dead: the cache is open and the banner can be claimed.
    Cleared,
    Claimed,
}

#[derive(Debug)]
pub(super) struct Base {
    pub site: usize,
    pub state: BaseState,
    pub gate: Option<EntityId>,
    pub banner: Option<EntityId>,
    pub cache: Option<EntityId>,
    pub guardians: Vec<EntityId>,
    /// Players who opened this cycle's cache.
    pub opened: Vec<EntityId>,
    /// The pack that bursts from the gate at the first claim has come, this cycle.
    pub burst: bool,
    /// The wave's health and damage multipliers the guardians were last scaled to.
    pub scaled: (f32, f32),
}

impl Base {
    pub fn new(site: usize) -> Self {
        Self { site, state: BaseState::Locked, gate: None, banner: None, cache: None, guardians: Vec::new(), opened: Vec::new(), burst: false, scaled: (0.0, 0.0) }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForgeAction {
    Repair,
    Thornbark,
    /// The Ancient lashes out: everything near it rooted and torn (once a wave).
    Wrath,
    Enchant(u32),
    /// A rank of a soldier upgrade for everyone's defenders (into `SOLDIERS`).
    Soldier(u8),
    /// Build at a site (0 the Ancient, 1.. a post).
    Build(super::defenses::Build, u8),
}

const BANNER_SPELL: u32 = 23333; // a capture channel's look (Warsong flag pickup)
/// Wrath of the Ancient: how far it reaches, how long it roots, how it looks.
pub(super) const WRATH_REACH: f32 = 35.0;
const WRATH_ROOT_SECS: f32 = 5.0;
const WRATH_ROOT: u32 = 339;
const WRATH_LOOK: u32 = 16914;

impl Sim {
    /// Whether the base called `name` is claimed (its boon is on).
    pub(super) fn boon(&self, name: &str) -> bool {
        self.bases
            .iter()
            .any(|b| b.state == BaseState::Claimed && self.arena.bases[b.site].name == name)
    }

    /// A base's guardians still standing: (id, where, alive, its kind's name).
    pub fn base_guardians_for_test(&self, base: usize) -> Vec<(EntityId, Vec3, bool, String)> {
        self.bases
            .get(base)
            .map(|b| {
                b.guardians
                    .iter()
                    .filter_map(|g| self.entities.get(g).map(|e| (*g, e.pos, e.alive(), e.enemy().map(|en| self.enemy_def(en.kind).name.clone()).unwrap_or_default())))
                    .collect()
            })
            .unwrap_or_default()
    }

    pub fn base_state(&self, base: usize) -> Option<BaseState> {
        self.bases.get(base).map(|b| b.state)
    }

    /// Open every base: gates, banners, guardians.
    pub(super) fn open_bases(&mut self) {
        if self.bases.is_empty() {
            return;
        }
        for b in 0..self.bases.len() {
            self.populate_base(b);
        }
        let names: Vec<String> = self.arena.bases.iter().map(|b| b.name.clone()).collect();
        let names: Vec<&str> = names.iter().map(|s| s.as_str()).collect();
        self.announce(
            format!(
                "Blight Gates have opened at {}! Slay their guardians for relics and gear, then claim their banners.",
                join_names(&names)
            ),
            true,
        );
        self.splash(Splash::Bases, "The Blight Gates Open", "Raid the outer bases for relics and gear");
    }

    fn populate_base(&mut self, b: usize) {
        let site = self.arena.bases[self.bases[b].site].clone();
        let toward_hub = facing(site.pos, self.arena.hub);
        if self.bases[b].gate.is_none() {
            let id = self.alloc();
            let pos = site.pos;
            self.entities.insert(id, Entity { pos, facing: toward_hub, health: 1, max_health: 1, level: 63, body: Body::Npc { npc: NpcKind::Gate, base: b } });
            self.events.push(Event::SpawnNpc { id, npc: NpcKind::Gate, base: b, pos, facing: toward_hub });
            self.bases[b].gate = Some(id);
        }
        let banner_pos = self.ring(site.pos, 7.0, toward_hub + 1.2);
        match self.bases[b].banner {
            None => {
                let id = self.alloc();
                let npc = NpcKind::Banner { claimed: false };
                self.entities.insert(id, Entity { pos: banner_pos, facing: toward_hub, health: 1, max_health: 1, level: 1, body: Body::Npc { npc, base: b } });
                self.events.push(Event::SpawnNpc { id, npc, base: b, pos: banner_pos, facing: toward_hub });
                self.bases[b].banner = Some(id);
            }
            Some(id) => self.set_banner(id, false),
        }
        if let Some(cache) = self.bases[b].cache.take() {
            self.entities.remove(&cache);
            self.events.push(Event::Despawn { id: cache });
        }
        // Guardians: one elite and some adds, tougher for each base already claimed this cycle.
        let claimed = self.bases.iter().filter(|x| x.state == BaseState::Claimed).count() as f32;
        let t = &self.content.bases;
        let tough = t.guardian_health * (1.0 + t.guardian_per_claim * claimed) * (1.0 + 0.25 * (self.player_count().max(1) - 1) as f32);
        let elite_kind = EnemyKind { boss: false, index: self.content.enemy_index(&t.guardian_elite).unwrap_or(0) };
        let adds = t.guardian_adds;
        let health = self.health_mult();
        let damage = self.damage_mult();
        let mut guardians = Vec::new();
        let id = self.alloc();
        let pos = self.ring(site.pos, 3.0, toward_hub);
        // (Their swings already land on heroes `hero_damage` times harder.)
        self.spawn_enemy(id, elite_kind, true, Some(b), pos, toward_hub, health * tough, damage);
        guardians.push(id);
        let unlocked: Vec<usize> = self
            .content
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.first_wave <= self.run.wave.max(1) && e.rank == 0)
            .map(|(i, _)| i)
            .collect();
        for k in 0..adds {
            let index = *unlocked.choose(&mut self.rng).unwrap_or(&0);
            let id = self.alloc();
            let a = toward_hub + (k as f32 + 1.0) * std::f32::consts::TAU / (adds as f32 + 1.0);
            let pos = self.ring(site.pos, 6.0, a);
            self.spawn_enemy(id, EnemyKind { boss: false, index }, false, Some(b), pos, toward_hub, health * tough * 0.18, damage * 0.8);
            guardians.push(id);
        }
        let base = &mut self.bases[b];
        base.guardians = guardians;
        base.scaled = (health, damage);
        base.state = BaseState::Guarded;
        base.opened.clear();
        base.burst = false;
    }

    /// Guardians keep pace with the waves: a base left standing at wave 3 is no pushover at
    /// wave 6. At each wave's start the living ones grow by what the wave's multipliers grew
    /// (keeping their share of health) and take the wave's level.
    pub(super) fn rescale_guardians(&mut self) {
        let (health, damage, level) = (self.health_mult(), self.damage_mult(), self.enemy_level());
        for b in 0..self.bases.len() {
            let (h0, d0) = self.bases[b].scaled;
            if self.bases[b].state != BaseState::Guarded || h0 <= 0.0 || d0 <= 0.0 {
                continue;
            }
            self.bases[b].scaled = (health, damage);
            let (hr, dr) = (health / h0, damage / d0);
            for id in self.bases[b].guardians.clone() {
                let Some(e) = self.entities.get_mut(&id) else { continue };
                if !e.alive() {
                    continue;
                }
                let share = e.health as f32 / e.max_health.max(1) as f32;
                e.max_health = (e.max_health as f32 * hr).round().clamp(1.0, 4.0e9) as u32;
                e.health = ((e.max_health as f32 * share).round() as u32).clamp(1, e.max_health);
                let elite = e.enemy().is_some_and(|en| en.elite);
                e.level = level + u32::from(elite);
                if let Some(en) = e.enemy_mut() {
                    en.damage_mult *= dr;
                }
                let (health, max_health, level) = (e.health, e.max_health, e.level);
                self.events.push(Event::Health { id, health, max_health });
                self.events.push(Event::UnitLevel { id, level });
            }
        }
    }

    fn set_banner(&mut self, id: EntityId, claimed: bool) {
        let npc = NpcKind::Banner { claimed };
        if let Some(Entity { body: Body::Npc { npc: n, .. }, .. }) = self.entities.get_mut(&id) {
            if *n != npc {
                *n = npc;
                self.events.push(Event::NpcChanged { id, npc });
            }
        }
    }

    pub(super) fn guardian_died(&mut self, b: usize, id: EntityId) {
        let Some(base) = self.bases.get_mut(b) else { return };
        base.guardians.retain(|&g| g != id);
        if !base.guardians.is_empty() || base.state != BaseState::Guarded {
            return;
        }
        base.state = BaseState::Cleared;
        let site = self.arena.bases[base.site].clone();
        let toward_hub = facing(site.pos, self.arena.hub);
        let pos = self.ring(site.pos, 7.0, toward_hub - 1.2);
        let cid = self.alloc();
        self.entities.insert(cid, Entity { pos, facing: toward_hub, health: 1, max_health: 1, level: 1, body: Body::Npc { npc: NpcKind::Cache, base: b } });
        self.events.push(Event::SpawnNpc { id: cid, npc: NpcKind::Cache, base: b, pos, facing: toward_hub });
        self.bases[b].cache = Some(cid);
        self.announce(
            format!("The guardians of {} have fallen! Open the relic cache and claim the banner.", site.name),
            true,
        );
    }

    /// The player right-clicked a unit: the Ancient's forge, a banner or a cache.
    pub fn interact(&mut self, id: EntityId, target: EntityId) {
        let Some(t) = self.entities.get(&target) else { return };
        let Some(me) = self.entities.get(&id) else { return };
        if !me.alive() {
            return;
        }
        if dist2d(t.pos, me.pos) > 12.0 {
            self.tell(id, "Too far away.".into());
            return;
        }
        match t.body {
            Body::Hub | Body::Npc { npc: NpcKind::Vendor, .. } => self.show_forge(id, 0),
            Body::Npc { npc: NpcKind::Post(k), .. } => {
                // Bought at home: the post only says what it is.
                let name = self.site_name(k as usize + 1);
                let vendor = self.vendor_name();
                self.tell(id, format!("Defences for {name} are bought from {vendor}: pick {name} on the shop's defences row."));
            }
            Body::Npc { npc: NpcKind::Banner { claimed }, base } => {
                let name = self.arena.bases[self.bases[base].site].name.clone();
                let state = self.bases[base].state;
                let (title, options, actions) = if claimed {
                    (format!("{name} is yours."), vec![MenuOption { icon: 0, text: "Leave it be.".into() }], vec![MenuAction::Close])
                } else if state == BaseState::Cleared {
                    let secs = self.content.bases.channel_secs as u32;
                    (
                        format!("Claim {name}"),
                        vec![
                            MenuOption { icon: 9, text: format!("Claim {name} ({secs} second channel; damage interrupts)") },
                            MenuOption { icon: 0, text: "Not yet.".into() },
                        ],
                        vec![MenuAction::Claim(base), MenuAction::Close],
                    )
                } else {
                    (
                        format!("{name}'s guardians still stand."),
                        vec![MenuOption { icon: 0, text: "The guardians must fall first.".into() }],
                        vec![MenuAction::Close],
                    )
                };
                self.open_menu(id, Menu { kind: MenuKind::Banner, source: target, title, options, offer: None }, actions);
            }
            Body::Npc { npc: NpcKind::Cache, base } => {
                if self.bases[base].opened.contains(&id) {
                    self.tell(id, "You've already taken your relic from this cache.".into());
                    return;
                }
                let name = self.arena.bases[self.bases[base].site].name.clone();
                self.open_menu(
                    id,
                    Menu {
                        kind: MenuKind::Cache,
                        source: target,
                        title: format!("Relic cache of {name}"),
                        options: vec![MenuOption { icon: 1, text: "Take your relic.".into() }],
                        offer: None,
                    },
                    vec![MenuAction::OpenCache(base)],
                );
            }
            _ => {}
        }
    }

    pub(super) fn open_cache(&mut self, id: EntityId, base: usize) {
        if self.bases[base].opened.contains(&id) {
            return;
        }
        self.bases[base].opened.push(id);
        self.offer(id, OfferSource::Relic);
        let tier = self.drop_tier();
        self.offer(id, OfferSource::SetPiece { tier });
    }

    pub(super) fn begin_claim(&mut self, id: EntityId, base: usize) {
        if self.bases[base].state != BaseState::Cleared {
            return;
        }
        // Already claiming it: another click doesn't start the channel over.
        if self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.channel.as_ref().is_some_and(|c| c.what == ChannelKind::Claim(base))) {
            return;
        }
        let secs = self.content.bases.channel_secs;
        self.start_channel(id, ChannelKind::Claim(base), secs, BANNER_SPELL);
        let name = self.arena.bases[self.bases[base].site].name.clone();
        let who = self.entities[&id].player().map(|p| p.name.clone()).unwrap_or_default();
        self.announce(format!("{who} is claiming {name}!"), false);
        // Claiming draws attention: a pack bursts from the gate (once: a broken claim tried
        // again doesn't call another).
        if std::mem::replace(&mut self.bases[base].burst, true) {
            return;
        }
        let site = self.arena.bases[self.bases[base].site].pos;
        let health = self.health_mult();
        let damage = self.damage_mult();
        for _ in 0..3 {
            let id = self.alloc();
            let pos = self.scatter(site, 5.0);
            let f = self.rng.gen::<f32>() * std::f32::consts::TAU;
            self.spawn_enemy(id, EnemyKind { boss: false, index: 0 }, false, Some(base), pos, f, health, damage);
        }
    }

    pub(super) fn claim_base(&mut self, id: EntityId, base: usize) {
        if self.bases[base].state != BaseState::Cleared {
            return;
        }
        self.bases[base].state = BaseState::Claimed;
        if let Some(b) = self.bases[base].banner {
            self.set_banner(b, true);
        }
        let site = self.arena.bases[self.bases[base].site].name.clone();
        let who = self.entities[&id].player().map(|p| p.name.clone()).unwrap_or_default();
        let boon = match site.as_str() {
            "the Farm" => "Everyone regenerates 1% health a second more.",
            "the Stables" => "Everyone runs 15% faster.",
            "the Gold Mine" => "Kills give 50% more gold.",
            "the Lumber Mill" => "Barricades slow the waves by 15%.",
            _ => "Its lane sends fewer enemies.",
        };
        self.announce(format!("{who} claimed {site}! {boon}"), true);
        let mut title = format!("{site} claimed");
        if let Some(first) = title.get(..1) {
            title = first.to_uppercase() + &title[1..];
        }
        self.splash(Splash::Claimed, title, boon.to_string());
        for p in self.player_ids() {
            self.recompute(p);
        }
        if !self.run.claimed_once.contains(&base) {
            self.run.claimed_once.push(base);
            let site = site.clone();
            self.bonus_level(self.content.levels.base_bonus, &format!("{} taken", super::shop::capitalize(&site)));
        }
        let all = self.bases.iter().all(|b| b.state == BaseState::Claimed);
        if all {
            let secs = self.content.game.escalation_warning_secs;
            self.run.escalation = Some(secs);
            self.announce(format!("Every base is yours. The Scourge escalates in {} seconds!", secs as u32), true);
        }
    }

    pub(super) fn step_bases(&mut self, dt: f32) {
        let Some(left) = self.run.escalation.as_mut() else { return };
        *left -= dt;
        if *left <= 0.0 {
            self.run.escalation = None;
            self.escalate();
        }
    }

    /// Everything resets and the enemies jump in strength.
    fn escalate(&mut self) {
        self.run.cycle += 1;
        let cycle = self.run.cycle;
        let fresh: Vec<usize> = (0..self.content.affixes.len()).filter(|i| !self.run.affixes.contains(i)).collect();
        let mut text = format!("ESCALATION {cycle}! The Scourge grows stronger.");
        if let Some(&a) = fresh.choose(&mut self.rng) {
            self.run.affixes.push(a);
            let def = &self.content.affixes[a];
            text.push_str(&format!(" New affix: {}. {}", def.name, def.text));
        }
        self.announce(text, true);
        let affix = self.run.affixes.last().map(|&a| format!("New affix: {}", self.content.affixes[a].name)).unwrap_or_default();
        self.splash(Splash::Escalation, format!("Escalation {cycle}"), affix);
        for b in &mut self.bases {
            b.state = BaseState::Guarded;
        }
        for b in 0..self.bases.len() {
            self.populate_base(b);
        }
        for p in self.player_ids() {
            self.recompute(p);
            self.offer(p, OfferSource::Legendary);
            self.escalation_gear(p);
        }
    }

    // --- the forge -------------------------------------------------------------------------------

    pub(super) fn forge_cost(&self, action: ForgeAction) -> u32 {
        let wave = self.run.wave.max(1);
        match action {
            ForgeAction::Repair => self.shared_price((60.0 * 1.5f32.powi(self.run.repairs as i32)) as u32 + 4 * wave),
            ForgeAction::Thornbark => self.shared_price((120.0 * 2.0f32.powi(self.run.thornbark as i32)) as u32),
            ForgeAction::Enchant(_) => 80 + 8 * wave,
            ForgeAction::Wrath => self.shared_price(60 + 8 * wave),
            ForgeAction::Build(b, _) => self.build_cost(b).unwrap_or(u32::MAX),
            ForgeAction::Soldier(k) => {
                let Some(u) = super::shop::SOLDIERS.get(k as usize) else { return u32::MAX };
                self.shared_price((u.cost as f32 * u.growth.powi(self.defenses.soldiers[k as usize] as i32)).round() as u32)
            }
        }
    }

    fn show_forge(&mut self, id: EntityId, site: usize) {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        let gold = p.gold;
        let mut options = Vec::new();
        let mut actions = Vec::new();
        for (text, b) in self.build_options() {
            options.push(MenuOption { icon: 6, text });
            actions.push(MenuAction::Forge(ForgeAction::Build(b, site as u8)));
        }
        let repair = self.forge_cost(ForgeAction::Repair);
        options.push(MenuOption { icon: 6, text: format!("Strengthen the Ancient: +15% health ({repair} gold)") });
        actions.push(MenuAction::Forge(ForgeAction::Repair));
        let thorns = self.forge_cost(ForgeAction::Thornbark);
        options.push(MenuOption {
            icon: 6,
            text: format!("Thornbark rank {}: the Ancient hurts what strikes it ({thorns} gold)", self.run.thornbark + 1),
        });
        actions.push(MenuAction::Forge(ForgeAction::Thornbark));
        if self.wrath_blocked().is_none() {
            let wrath = self.forge_cost(ForgeAction::Wrath);
            options.push(MenuOption {
                icon: 6,
                text: format!("Wrath of the Ancient: root and tear everything within 35 yards, once a wave ({wrath} gold)"),
            });
            actions.push(MenuAction::Forge(ForgeAction::Wrath));
        }
        let enchant_cost = self.forge_cost(ForgeAction::Enchant(0));
        for (e, tag) in self.enchant_choices(id) {
            options.push(MenuOption {
                icon: 3,
                text: format!("Enchant your weapon: {} (+10% power to your {tag} spells) ({enchant_cost} gold)", capitalize(tag)),
            });
            actions.push(MenuAction::Forge(ForgeAction::Enchant(e)));
        }
        options.push(MenuOption { icon: 0, text: format!("Leave. (You have {gold} gold.)") });
        actions.push(MenuAction::Close);
        self.open_menu(
            id,
            Menu {
                kind: MenuKind::Forge,
                source: if site == 0 { self.hub } else { self.post_ids.get(site - 1).copied().unwrap_or(self.hub) },
                title: if site == 0 { format!("{}: upgrades", self.content.hub.name) } else { format!("Defences at {}", self.site_name(site)) },
                options,
                offer: None,
            },
            actions,
        );
    }

    fn forge_site(&self, action: ForgeAction) -> usize {
        match action {
            ForgeAction::Build(_, s) => s as usize,
            _ => 0,
        }
    }

    /// A choice in the Ancient's gossip menu (players without the add-on).
    pub(super) fn forge(&mut self, id: EntityId, action: ForgeAction) {
        if !self.at_shop(id) {
            let v = self.vendor_name();
            self.tell(id, format!("Shopping is done at home: visit {v}."));
            self.close_menu(id);
            return;
        }
        match self.buy_forge(id, action) {
            Err(why) => {
                self.tell(id, why);
                self.show_forge(id, self.forge_site(action));
            }
            // Keep shopping after a building.
            Ok(()) if matches!(action, ForgeAction::Build(..)) => self.show_forge(id, self.forge_site(action)),
            Ok(()) => self.close_menu(id),
        }
    }

    /// Why the Ancient's wrath can't be called now, if it can't.
    pub(super) fn wrath_blocked(&self) -> Option<&'static str> {
        if !matches!(self.phase, Phase::Wave { .. }) {
            Some("Only during a wave.")
        } else if self.run.wrath_wave == self.run.wave {
            Some("Its wrath is spent this wave.")
        } else {
            None
        }
    }

    /// Wrath of the Ancient: every enemy within reach of it rooted and a quarter of it torn away
    /// (a boss: a twentieth).
    fn ancient_wrath(&mut self, id: EntityId) {
        self.run.wrath_wave = self.run.wave;
        let hub = self.hub;
        let pos = self.arena.hub;
        self.events.push(Event::SpellGo { caster: hub, spell: WRATH_LOOK, target: SpellTarget::Point(pos), hits: vec![] });
        let caught: Vec<(EntityId, bool, u32)> = self
            .entities
            .iter()
            .filter(|(_, e)| e.alive() && dist2d(e.pos, pos) <= WRATH_REACH)
            .filter_map(|(&eid, e)| e.enemy().map(|en| (eid, en.kind.boss, e.max_health)))
            .collect();
        for &(eid, boss, max) in &caught {
            self.root_enemy(eid, WRATH_ROOT_SECS);
            self.events.push(Event::SpellGo { caster: hub, spell: WRATH_ROOT, target: SpellTarget::Unit(eid), hits: vec![eid] });
            let share = if boss { 0.05 } else { 0.25 };
            self.deal(id, eid, ((max as f32) * share).round().max(1.0) as u32, false, super::enemies::Via::Spell { spell: WRATH_ROOT, school: 3, periodic: false });
        }
        let name = self.entities.get(&id).and_then(|e| e.player()).map_or(String::new(), |p| p.name.clone());
        self.announce(format!("{name} calls the {}'s wrath: {} enemies rooted and torn!", self.content.hub.name, caught.len()), true);
    }

    /// Pay for and carry out a forge purchase (the caller checks the hero is at home).
    /// The next rank of soldier upgrade `k`, or `None` at the top rank.
    pub(super) fn soldier_cost(&self, k: usize) -> Option<u32> {
        let u = super::shop::SOLDIERS.get(k)?;
        (self.defenses.soldiers[k] < u.max).then(|| self.forge_cost(ForgeAction::Soldier(k as u8)))
    }

    pub(super) fn buy_forge(&mut self, id: EntityId, action: ForgeAction) -> Result<(), String> {
        if let ForgeAction::Soldier(k) = action {
            if self.soldier_cost(k as usize).is_none() {
                return Err("That's at its top rank.".into());
            }
        }
        if let ForgeAction::Build(b, _) = action {
            if self.build_cost(b).is_none() {
                return Err("No more of those can be built.".into());
            }
        }
        if let ForgeAction::Enchant(e) = action {
            let Some(p) = self.entities.get(&id).and_then(|x| x.player()) else { return Err(String::new()) };
            let slot = self.content.classes[p.class].weapon_slot as usize;
            match &p.equip[slot] {
                None => return Err("You have no weapon to enchant.".into()),
                Some(it) if it.enchant == e => return Err("Your weapon already has that enchantment.".into()),
                _ => {}
            }
        }
        if action == ForgeAction::Wrath {
            if let Some(why) = self.wrath_blocked() {
                return Err(why.into());
            }
        }
        let cost = self.forge_cost(action);
        if !self.take_gold(id, cost) {
            let gold = self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.gold);
            return Err(format!("That costs {cost} gold; you have {gold}."));
        }
        match action {
            ForgeAction::Build(b, site) => self.build(id, b, site as usize),
            ForgeAction::Repair => {
                self.run.repairs += 1;
                let hub = self.hub;
                let e = self.entities.get_mut(&hub).unwrap();
                let add = (e.max_health as f32 * 0.15) as u32;
                e.max_health += add;
                e.health = (e.health + add).min(e.max_health);
                let (health, max_health) = (e.health, e.max_health);
                self.events.push(Event::Health { id: hub, health, max_health });
                self.events.push(Event::SpellGo { caster: hub, spell: 635, target: SpellTarget::Unit(hub), hits: vec![hub] });
                self.announce(format!("The {} grows stronger.", self.content.hub.name), false);
            }
            ForgeAction::Wrath => self.ancient_wrath(id),
            ForgeAction::Soldier(k) => {
                let u = &super::shop::SOLDIERS[k as usize];
                self.defenses.soldiers[k as usize] += 1;
                let rank = self.defenses.soldiers[k as usize];
                let who = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.name.clone()).unwrap_or_default();
                self.announce(format!("{who} buys {} (rank {rank}) for the soldiers.", u.name), false);
                // Promoted: each defender becomes the new rank (a new name and look).
                if super::shop::SOLDIERS[k as usize].key == "promote" {
                    self.promote_defenders();
                }
                // Drilled defenders: their numbers now.
                self.refresh_defenses(false);
                for d in self.defenses.defenders.iter().map(|d| d.0).collect::<Vec<_>>() {
                    self.events.push(Event::SpellGo { caster: d, spell: u.icon, target: SpellTarget::Unit(d), hits: vec![] });
                }
            }
            ForgeAction::Thornbark => {
                self.run.thornbark += 1;
                let hub = self.hub;
                self.events.push(Event::SpellGo { caster: hub, spell: 467, target: SpellTarget::Unit(hub), hits: vec![hub] });
                self.events.push(Event::Aura { unit: hub, slot: 0, spell: 467, stacks: self.run.thornbark.min(255) as u8, secs: 0.0 });
                self.announce(format!("Thornbark rank {} covers the Ancient.", self.run.thornbark), false);
            }
            ForgeAction::Enchant(e) => {
                let Some(p) = self.entities.get(&id).and_then(|x| x.player()) else { return Ok(()) };
                let slot = self.content.classes[p.class].weapon_slot as usize;
                if let Some(it) = self.entities.get_mut(&id).and_then(|x| x.player_mut()).and_then(|p| p.equip[slot].as_mut()) {
                    it.enchant = e;
                }
                self.events.push(Event::Inventory { player: id });
                self.events.push(Event::Visible { player: id });
                self.recompute(id);
                self.tell(id, "Your weapon glows with new power.".into());
            }
        }
        Ok(())
    }
}
