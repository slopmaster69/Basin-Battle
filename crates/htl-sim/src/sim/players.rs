//! Players: joining and leaving, levels and experience, stats from level, cards and gear,
//! auto-attacks, and what happens when they're hit.

use std::collections::VecDeque;

use rand::Rng;

use super::cards::{Offer, Owned, Pick};
use super::spells::{Buff, BuffKind, Hot, OwnedSpell};
use super::enemies::Via;
use super::gear::ItemInst;
use super::*;
use crate::items::{self, slot};

/// Luck from Renown, set when a player joins.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Luck {
    pub rerolls: u32,
    pub banishes: u32,
    pub skips: u32,
    pub wider: bool,
    pub lock: bool,
    pub head_start: bool,
}

#[derive(Debug)]
pub(super) struct Channel {
    pub what: ChannelKind,
    pub left: f32,
    pub from: Vec3,
    /// Seconds hits have pushed it back so far.
    pub pushed: f32,
}

/// Each hit while channelling sets it back this far, up to `MAX_PUSHBACK` in all: a channel is
/// slowed by a fight, never broken by one.
/// Within this many yards of another hero: Shoulder to Shoulder.
pub(super) const TOGETHER_YARDS: f32 = 12.0;
/// Its share more damage dealt, and less taken.
const TOGETHER_BONUS: f32 = 0.1;
/// The buff it shows as (Blessing of Kings' crown).
pub(super) const TOGETHER_SPELL: u32 = 20217;
const PUSHBACK: f32 = 0.25;
const MAX_PUSHBACK: f32 = 2.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ChannelKind {
    Claim(usize),
    Hearth,
}

#[derive(Debug)]
pub(super) struct Player {
    pub name: String,
    pub class: usize,
    pub attacking: Option<EntityId>,
    pub swing: f32,
    /// When the last swing (or shot) went off: a fight opened again can't swing sooner than a
    /// swing's time after it.
    pub swung_at: f32,
    /// Healing banked between whole-second ticks.
    pub regen: f32,
    pub level: u32,
    /// Experience into the current level.
    pub xp: u32,
    /// Levels from objectives (bosses, bases): they lift the cap a wave's kills can reach.
    pub bonus_levels: u32,
    /// The wave the player was last told their bar is full.
    pub capped_told: u32,
    pub gold: u32,
    pub kills: u32,
    /// This run: damage dealt to enemies (its own and its pet's), and times fallen.
    pub dealt: u64,
    /// The damage meter: this wave's damage, and the player's own count since they last reset it.
    pub meter: Meter,
    /// The Hero tab's wares on offer to this hero now, as item entries (restocked each wave).
    pub stock: Vec<u32>,
    pub falls: u32,
    /// Strikes stepped out of, and ticks of marked ground stood in.
    pub dodges: u32,
    pub struck: u32,
    /// Legendary cards held.
    pub cards: Vec<Owned>,
    /// Powers held (the Escalations' and the bosses'), by index into the content's powers.
    pub powers: Vec<usize>,
    /// A warrior's weapon path, once a path talent is taken: greatsword, shield or dual.
    pub path: Option<String>,
    /// Class spells held: from cards, and from talents.
    pub spells: Vec<OwnedSpell>,
    /// Talent ranks, by the class kit's talent index.
    pub talents: Vec<u8>,
    pub buffs: Vec<Buff>,
    pub hots: Vec<Hot>,
    /// Health share banked from buffs that regenerate.
    pub buff_regen: f32,
    /// Warriors: rage (0-100) and what the client was last told.
    pub rage: f32,
    pub rage_sent: u32,
    /// Fighting until (rage holds while fighting).
    pub fighting_until: f32,
    /// Revenge is open until.
    pub revenge_until: f32,
    /// Hunters: the pet, when it returns after falling, and what it should go for.
    pub pet: Option<EntityId>,
    pub pet_back_at: f32,
    pub pet_target: Option<EntityId>,
    pub stats: Stats,
    pub equip: [Option<ItemInst>; slot::EQUIPMENT as usize],
    pub pack: [Option<ItemInst>; slot::PACK_SLOTS as usize],
    pub offers: VecDeque<Offer>,
    pub menu: Option<Menu>,
    /// What each option of the open menu does.
    pub menu_actions: Vec<cards::MenuAction>,
    pub luck: Luck,
    /// What's left of this run's luck.
    pub rerolls: u32,
    pub banishes: u32,
    pub skips: u32,
    pub gold_rerolls: u32,
    pub banished: Vec<Pick>,
    pub locked: Option<Pick>,
    pub channel: Option<Channel>,
    /// A spell being cast (a cast time running), if any.
    pub casting: Option<super::spells::CastJob>,
    pub hearth_ready: f32,
    // Triggers.
    pub shield_charges: f32,
    pub divine_ready: f32,
    pub divine_until: f32,
    pub undying: bool,
    pub undying_used: bool,
    pub autos: u32,
    pub casts: u32,
    pub chilled_until: f32,
    /// When an ordinary enemy's strike last marked the ground for this player: the next waits
    /// (`STRIKE_GAP`), so a crowd can't keep you dodging without a chance to cast.
    pub struck_at: f32,
    /// What's left of an absorb shield (Power Word: Shield, Ice Barrier...), until when, what
    /// it reflects and its burst when it breaks.
    pub shield_pool: f32,
    pub shield_ready: f32,
    pub shield_until: f32,
    pub shield_reflect: f32,
    pub shield_burst: f32,
    pub shield_caster: EntityId,
    pub shield_spell: u32,
    pub necrotic: u8,
    pub necrotic_until: f32,
    /// Run speed last sent to the client.
    pub sent_speed: f32,
    /// Buff slots as last sent: spell and stacks.
    pub sent_auras: Vec<(u32, u8)>,
    /// Another hero stands close: Shoulder to Shoulder ([`TOGETHER_YARDS`]).
    pub together: bool,
    /// Card windows wait until this time: a client that has just been revived or moved can drop
    /// a dialog that arrives in the same moment.
    pub menu_hold: f32,
    /// An offer is waiting for the hold to pass.
    pub menu_due: bool,
    /// A sprint pick-up's speed, until when.
    pub sprint_until: f32,
    pub sprinting: bool,
    /// Dead: back on their feet at the Ancient at this time.
    pub respawn_at: f32,
    /// Spells are cast by hand (the add-on's bar) rather than firing on their own.
    pub manual: bool,
    /// The unit the player has selected in the client.
    pub selected: Option<EntityId>,
    /// The global cooldown after a cast by hand.
    pub gcd_until: f32,
    /// Spells cast by hand this run (the tracker stops teaching the keys after a few).
    pub hand_casts: u32,
    /// The site this player said they'd hold (a ping).
    pub holds: Option<usize>,
    /// Triggers: their timed buffs, when each can go off again, and how deep a chain of them
    /// runs now.
    pub tbuffs: Vec<super::triggers::TBuff>,
    pub trig_ready: Vec<((u8, u16, u16, u8), f32)>,
    /// Level-up offers since the last Rare-or-better card (the safety net).
    pub since_rare: u8,
    /// When each trigger may next be called out on screen.
    pub shout_ready: Vec<((u8, u16, u16, u8), f32)>,
    pub trig_depth: u8,
}

/// Derived numbers: recomputed whenever level, cards or gear change.
#[derive(Debug, Clone, Default)]
pub(super) struct Stats {
    pub max_health: u32,
    pub hit_lo: f32,
    pub hit_hi: f32,
    pub attack_secs: f32,
    pub crit: f32,
    pub damage: f32,
    pub haste: f32,
    pub regen: f32,
    pub speed: f32,
    pub armor: u32,
    pub reduction: f32,
    pub strength: i32,
    pub agility: i32,
    pub stamina: i32,
    pub intellect: i32,
    pub spirit: i32,
    pub cdr: f32,
    pub radius: f32,
    pub echo: f32,
    pub cleave: f32,
    pub dot: f32,
    pub dot_secs: f32,
    pub dot_spell: u32,
    pub burn: f32,
    pub burn_secs: f32,
    pub split: f32,
    pub heal: f32,
    pub set_pieces: u32,
    pub per_wave: f32,
    pub soul_harvest: f32,
    pub raise_dead: f32,
    pub raise_secs: f32,
    pub bloodthirst: f32,
    pub lightning: f32,
    pub divine_secs: f32,
    pub divine_cooldown: f32,
    pub devotion: f32,
    pub devotion_radius: f32,
    pub shout: f32,
    pub shout_radius: f32,
    pub team_haste: f32,
    pub team_haste_radius: f32,
    pub team_regen: f32,
    pub team_regen_radius: f32,
    pub leech: f32,
    pub execute: f32,
    pub windfury: f32,
    pub thorns: f32,
    pub shield: f32,
    pub shield_cooldown: f32,
    pub dot_school: u8,
    /// Card tags held (plus the weapon enchant's), for resonance.
    pub tag_counts: Vec<(String, u32)>,
    /// The weapon enchant's school (its spells deal 10% more).
    pub enchant_school: Option<u8>,
    /// From talents: less damage taken, more auto-attack damage, shields' strength, damage to
    /// elites and bosses, the heal from a block.
    pub taken: f32,
    pub auto: f32,
    pub absorb: f32,
    pub elite_damage: f32,
    pub block_heal: f32,
    /// Rage: more from swings and from blows taken, a second while fighting, kept between fights.
    pub rage_auto: f32,
    pub rage_hit: f32,
    pub rage_regen: f32,
    pub rage_keep: f32,
    /// Defiance: enemies this close turn on you.
    pub reach: f32,
    /// The hunter's pet.
    pub pet_damage: f32,
    pub pet_health: f32,
    pub pet_crit: f32,
    pub pet_taken: f32,
    pub pet_regen: f32,
    pub respawn: f32,
    /// From worn items: more gold picked up, a share of damage dealt that heals the Ancient,
    /// more damage from defences near you, and getting up sooner after falling.
    pub gold_find: f32,
    pub hub_leech: f32,
    pub defence_aura: f32,
    pub rise: f32,
    /// Named talent effects and their strengths.
    pub procs: Procs,
    /// Team auras from talents (you and allies within 30 yd).
    pub team_health: f32,
}

/// Talent effects the game knows by name (0 when not taken).
#[derive(Debug, Clone, Default)]
pub(super) struct Procs {
    pub clearcasting: f32,
    pub ignite: f32,
    pub deep_wounds: f32,
    pub master_of_elements: f32,
    pub enrage: f32,
    pub flurry: f32,
    pub blood_craze: f32,
    pub spirit_tap: f32,
    pub quick_shots: f32,
    pub extra_attack: f32,
    pub block: f32,
    pub martyrdom: f32,
    pub inspiration: f32,
    pub vampiric_embrace: f32,
    pub spirit_of_redemption: f32,
    pub frenzy: f32,
}

impl Procs {
    fn add(&mut self, name: &str, v: f32) {
        let f = match name {
            "clearcasting" => &mut self.clearcasting,
            "ignite" => &mut self.ignite,
            "deep_wounds" => &mut self.deep_wounds,
            "master_of_elements" => &mut self.master_of_elements,
            "enrage" => &mut self.enrage,
            "flurry" => &mut self.flurry,
            "blood_craze" => &mut self.blood_craze,
            "spirit_tap" => &mut self.spirit_tap,
            "quick_shots" => &mut self.quick_shots,
            "extra_attack" => &mut self.extra_attack,
            "block" => &mut self.block,
            "martyrdom" => &mut self.martyrdom,
            "inspiration" => &mut self.inspiration,
            "vampiric_embrace" => &mut self.vampiric_embrace,
            "spirit_of_redemption" => &mut self.spirit_of_redemption,
            "frenzy" => &mut self.frenzy,
            _ => return,
        };
        *f += v;
    }
}

/// One player as the server needs to show them.
#[derive(Debug, Clone)]
pub struct PlayerView {
    pub name: String,
    pub class: usize,
    pub level: u32,
    pub xp: u32,
    pub next_xp: u32,
    pub gold: u32,
    pub stats: StatsView,
    pub equip: Vec<(u8, u32, u32, u32)>,
    pub pack: Vec<(u8, u32, u32, u32)>,
    pub cards: Vec<(String, u8)>,
}

#[derive(Debug, Clone, Default)]
pub struct StatsView {
    pub max_health: u32,
    pub damage: (f32, f32),
    pub attack_ms: u32,
    pub armor: u32,
    /// Strength, agility, stamina, intellect, spirit.
    pub attributes: [i32; 5],
    pub speed: f32,
    pub crit_pct: f32,
    /// How big the hero stands (1.0 as made; Avatar and its like grow them).
    pub scale: f32,
}

/// A hero's damage meter.
#[derive(Debug, Clone, Default)]
pub(super) struct Meter {
    /// This wave's damage (shown until the next wave begins).
    pub wave: u64,
    /// Since the last reset: the total, when it began (the sim's clock), and by source.
    pub total: u64,
    pub since: f32,
    pub by: Vec<(String, u64)>,
    /// This wave's beating, for the balance log: damage taken, hits, the biggest and who dealt
    /// it, and times fallen.
    pub taken: u64,
    pub hits: u32,
    pub biggest: (u32, String),
    pub fell: u32,
}

/// A hero's own damage since they last reset the meter.
#[derive(Debug, Clone, PartialEq)]
pub struct MeterView {
    pub secs: f32,
    pub total: u64,
    /// By source (a spell, Auto Attack, the pet), most first.
    pub rows: Vec<(String, u64)>,
}

/// One hero's line on the party's meter for the wave.
#[derive(Debug, Clone, PartialEq)]
pub struct PartyMeterRow {
    pub name: String,
    pub class: String,
    pub damage: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PlayerResult {
    pub name: String,
    pub class: String,
    pub level: u32,
    pub kills: u32,
    pub dealt: u64,
    pub falls: u32,
    pub dodges: u32,
}

impl Sim {
    /// A player enters the arena as `class` (a content key). Returns its id.
    pub fn add_player(&mut self, name: &str, class: &str, luck: Luck) -> EntityId {
        let class_idx = self.content.class_index(class).unwrap_or(0);
        if let Some(id) = self.rejoin(name, class_idx) {
            return id;
        }
        let id = self.alloc();
        let (pos, facing) = (self.arena.player_spawn, self.arena.player_facing);
        let player = Player {
            name: name.to_string(),
            class: class_idx,
            attacking: None,
            swing: 0.0,
            swung_at: f32::NEG_INFINITY,
            regen: 0.0,
            level: 1,
            xp: 0,
            capped_told: 0,
            bonus_levels: 0,
            gold: 0,
            kills: 0,
            dealt: 0,
            meter: Meter::default(),
            stock: Vec::new(),
            falls: 0,
            dodges: 0,
            struck: 0,
            cards: Vec::new(),
            powers: Vec::new(),
            path: None,
            spells: Vec::new(),
            talents: Vec::new(),
            buffs: Vec::new(),
            hots: Vec::new(),
            buff_regen: 0.0,
            rage: 0.0,
            rage_sent: 0,
            fighting_until: 0.0,
            revenge_until: 0.0,
            pet: None,
            pet_back_at: 0.0,
            pet_target: None,
            stats: Stats::default(),
            equip: Default::default(),
            pack: Default::default(),
            offers: VecDeque::new(),
            menu: None,
            menu_actions: Vec::new(),
            luck,
            rerolls: luck.rerolls,
            banishes: luck.banishes,
            skips: luck.skips,
            gold_rerolls: 0,
            banished: Vec::new(),
            locked: None,
            channel: None,
            casting: None,
            hearth_ready: 0.0,
            shield_charges: 3.0,
            divine_ready: 0.0,
            divine_until: 0.0,
            undying: false,
            undying_used: false,
            autos: 0,
            casts: 0,
            chilled_until: 0.0,
            struck_at: f32::NEG_INFINITY,
            together: false,
            shield_pool: 0.0,
            shield_ready: 0.0,
            shield_until: 0.0,
            shield_reflect: 0.0,
            shield_burst: 0.0,
            shield_caster: 0,
            shield_spell: 0,
            necrotic: 0,
            necrotic_until: 0.0,
            sent_speed: 7.0,
            sent_auras: Vec::new(),
            menu_hold: 0.0,
            menu_due: false,
            sprint_until: 0.0,
            sprinting: false,
            respawn_at: f32::INFINITY,
            manual: false,
            selected: None,
            gcd_until: 0.0,
            hand_casts: 0,
            holds: None,
            tbuffs: Vec::new(),
            trig_ready: Vec::new(),
            shout_ready: Vec::new(),
            since_rare: 0,
            trig_depth: 0,
        };
        self.entities.insert(
            id,
            Entity { pos, facing, health: 1, max_health: 1, level: 1, body: Body::Player(Box::new(player)) },
        );
        self.order.push_back(id);
        self.starting_kit(id);
        self.restock(id);
        self.recompute(id);
        let max = self.entities[&id].max_health;
        self.entities.get_mut(&id).unwrap().health = max;
        if matches!(self.phase, Phase::Waiting) {
            self.reset();
            self.phase = Phase::Countdown { left: self.content.game.countdown_secs };
            self.announce(
                format!(
                    "Defend the {}! The first wave arrives in {} seconds. Type .start to begin now.",
                    self.content.hub.name, self.content.game.countdown_secs as u32
                ),
                true,
            );
            self.start_of_run_offers();
        } else {
            self.announce(format!("{name} joins the defence."), false);
            // A late arrival catches up to the team's level.
            let team = self.player_ids().into_iter().filter(|&o| o != id).filter_map(|o| self.entities.get(&o).and_then(|e| e.player()).map(|p| (p.level, p.bonus_levels))).max();
            if let Some((level, bonus)) = team {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.bonus_levels = bonus;
                }
                let target = level.saturating_sub(1).max(1);
                if target > 1 {
                    self.set_level(id, target);
                }
            }
            self.starting_offers(id);
        }
        id
    }

    /// A hero who left this run comes back as they were: at the Ancient, healed, with their
    /// spells, talents, cards, level, gear, gold and any picks still waiting.
    fn rejoin(&mut self, name: &str, class: usize) -> Option<EntityId> {
        let i = self.departed.iter().position(|e| e.player().is_some_and(|p| p.name == name && p.class == class))?;
        let mut e = self.departed.remove(i);
        let id = self.alloc();
        let now = self.clock;
        e.pos = self.arena.player_spawn;
        e.facing = self.arena.player_facing;
        if let Some(p) = e.player_mut() {
            p.attacking = None;
            p.selected = None;
            p.channel = None;
            p.casting = None;
            p.menu = None;
            p.menu_due = !p.offers.is_empty();
            p.pet = None;
            p.pet_target = None;
            p.pet_back_at = now + 1.0;
            p.respawn_at = f32::INFINITY;
            p.shield_caster = 0;
            p.sent_auras.clear();
        }
        self.entities.insert(id, e);
        self.order.push_back(id);
        self.recompute(id);
        let e = self.entities.get_mut(&id).unwrap();
        e.health = e.max_health;
        self.announce(format!("{name} is back, just as they left."), false);
        for ev in [Event::Bar { player: id }, Event::Talents { player: id }, Event::Stats { player: id }] {
            self.events.push(ev);
        }
        let gold = self.entities[&id].player().map_or(0, |p| p.gold);
        self.events.push(Event::Gold { player: id, gold });
        self.refresh_auras(id);
        Some(id)
    }

    pub fn remove_player(&mut self, id: EntityId) {
        let Some(e) = self.entities.remove(&id) else {
            return;
        };
        self.order.retain(|&p| p != id);
        if let Body::Player(p) = &e.body {
            self.announce(format!("{} has left.", p.name), false);
        }
        // Mid-run, the hero is kept for their return.
        if e.is_player() && !matches!(self.phase, Phase::Waiting | Phase::Over { .. }) {
            let name = e.player().map(|p| p.name.clone()).unwrap_or_default();
            self.departed.retain(|d| d.player().is_some_and(|p| p.name != name));
            self.departed.push(e);
        }
        // Anyone fighting them looks for someone else.
        let ids: Vec<EntityId> = self.entities.keys().copied().collect();
        for eid in ids {
            let other = self.entities.get_mut(&eid).unwrap();
            if let Body::Enemy(en) = &mut other.body {
                if en.goal == Goal::Player(id) {
                    en.goal = if en.guard.is_some() { Goal::Guard } else { Goal::Hub };
                    en.mover = None;
                    if en.engaged == Some(id) {
                        en.engaged = None;
                        self.events.push(Event::AttackStop { attacker: eid, victim: id });
                    }
                }
            }
        }
        let allies: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(&e.body, Body::Ally(a) if a.owner == id))
            .map(|(&a, _)| a)
            .collect();
        for a in allies {
            self.entities.remove(&a);
            self.events.push(Event::Despawn { id: a });
        }
        self.run.drought.remove(&id);
        if self.player_count() == 0 {
            self.reset();
            self.phase = Phase::Waiting;
        }
    }

    /// The player's client reports where it is.
    pub fn move_player(&mut self, id: EntityId, pos: Vec3, facing: f32) {
        if let Some(e) = self.entities.get_mut(&id) {
            if e.is_player() {
                e.pos = pos;
                e.facing = facing;
            }
        }
    }

    /// The player starts auto-attacking `target`.
    pub fn attack(&mut self, id: EntityId, target: EntityId) {
        let target_ok = self.entities.get(&target).is_some_and(|t| t.alive() && t.is_enemy());
        let now = self.clock;
        let Some(e) = self.entities.get_mut(&id) else {
            return;
        };
        if !e.alive() || !target_ok {
            return;
        }
        let Some(p) = e.player_mut() else {
            return;
        };
        let fresh = p.attacking.is_none();
        if let Some(old) = p.attacking.replace(target) {
            if old != target {
                self.events.push(Event::AttackStop { attacker: id, victim: old });
            }
        }
        // A swing is ready at once when a fight opens, unless one went off less than a swing's
        // time ago. Switching targets keeps the timer running: clicking between two enemies (or
        // stopping and starting) used to make every click a fresh swing.
        if fresh {
            p.swing = p.swing.max((now - p.swung_at).min(p.stats.attack_secs));
        }
        self.events.push(Event::AttackStart { attacker: id, victim: target });
    }

    pub fn stop_attack(&mut self, id: EntityId) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            if let Some(victim) = p.attacking.take() {
                self.events.push(Event::AttackStop { attacker: id, victim });
            }
        }
    }

    /// Everything the server shows about a player.
    pub fn player_view(&self, id: EntityId) -> Option<PlayerView> {
        let e = self.entities.get(&id)?;
        let p = e.player()?;
        let s = &p.stats;
        let inst = |(i, it): (usize, &Option<ItemInst>), base: u8| {
            it.as_ref().map(|it| (base + i as u8, it.id, it.entry, it.enchant))
        };
        Some(PlayerView {
            name: p.name.clone(),
            class: p.class,
            level: p.level,
            xp: p.xp,
            next_xp: self.content.levels.xp_to_next(p.level),
            gold: p.gold,
            stats: StatsView {
                max_health: e.max_health,
                damage: (s.hit_lo * s.damage, s.hit_hi * s.damage),
                attack_ms: (s.attack_secs * 1000.0) as u32,
                armor: s.armor,
                attributes: [s.strength, s.agility, s.stamina, s.intellect, s.spirit],
                speed: s.speed,
                crit_pct: s.crit * 100.0,
                scale: 1.0
                    + p.cards.iter().map(|o| self.content.cards[o.card].grow).sum::<f32>()
                    + p.powers.iter().filter_map(|&k| self.content.powers.get(k)).map(|pw| pw.grow).sum::<f32>(),
            },
            equip: p.equip.iter().enumerate().filter_map(|x| inst(x, 0)).collect(),
            pack: p.pack.iter().enumerate().filter_map(|x| inst(x, slot::PACK_FIRST)).collect(),
            cards: {
                let kit = self.content.kit(p.class);
                let mut v: Vec<(String, u8)> =
                    self.bar_order(id).into_iter().map(|i| (kit.spells[p.spells[i].spell].name.clone(), p.spells[i].perks.len() as u8 + 1)).collect();
                v.extend(p.cards.iter().map(|o| (self.content.cards[o.card].name.clone(), o.rank)));
                v.extend(p.powers.iter().filter_map(|&k| self.content.powers.get(k)).map(|pw| (pw.name.clone(), 1)));
                v
            },
        })
    }

    pub(super) fn player_results(&self) -> Vec<PlayerResult> {
        self.player_ids()
            .iter()
            .filter_map(|id| self.entities[id].player())
            .map(|p| PlayerResult {
                name: p.name.clone(),
                class: self.content.classes[p.class].key.clone(),
                level: p.level,
                kills: p.kills,
                dealt: p.dealt,
                falls: p.falls,
                dodges: p.dodges,
            })
            .collect()
    }

    /// Back to the start of a run: level 1, no cards, the starting kit.
    pub(super) fn reset_player(&mut self, id: EntityId) {
        let (old_items, luck) = {
            let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
            let had = p.equip.iter().chain(p.pack.iter()).any(|i| i.is_some());
            p.level = 1;
            p.xp = 0;
            p.bonus_levels = 0;
            p.gold = 0;
            p.kills = 0;
            p.dealt = 0;
            p.meter = Meter::default();
            p.stock.clear();
            p.falls = 0;
            p.dodges = 0;
            p.struck = 0;
            p.cards.clear();
            p.powers.clear();
            p.path = None;
            p.spells.clear();
            p.talents.clear();
            p.buffs.clear();
            p.hots.clear();
            p.rage = 0.0;
            p.shield_pool = 0.0;
            p.offers.clear();
            p.menu = None;
            p.menu_actions.clear();
            p.rerolls = p.luck.rerolls;
            p.banishes = p.luck.banishes;
            p.skips = p.luck.skips;
            p.gold_rerolls = 0;
            p.banished.clear();
            p.locked = None;
            p.channel = None;
            p.undying = false;
            p.undying_used = false;
            p.equip = Default::default();
            p.pack = Default::default();
            p.gcd_until = 0.0;
            p.hand_casts = 0;
            p.respawn_at = f32::INFINITY;
            (had, p.luck)
        };
        let _ = luck;
        self.events.push(Event::Menu { player: id, menu: None });
        self.events.push(Event::Bar { player: id });
        self.events.push(Event::Talents { player: id });
        if let Some(pet) = self.entities.get(&id).and_then(|e| e.player()).and_then(|p| p.pet) {
            self.halt_enemy(pet);
            self.entities.remove(&pet);
            self.events.push(Event::Despawn { id: pet });
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.pet = None;
                p.pet_back_at = 0.0;
            }
        }
        if old_items {
            self.events.push(Event::Inventory { player: id });
        }
        self.starting_kit(id);
        self.restock(id);
        self.recompute(id);
        let e = self.entities.get_mut(&id).unwrap();
        e.level = 1;
        let was_dead = !e.alive();
        e.health = e.max_health;
        e.pos = self.arena.player_spawn;
        let (health, max_health) = (e.health, e.max_health);
        self.events.push(Event::Level { player: id, level: 1, gained_health: 0 });
        self.events.push(Event::Gold { player: id, gold: 0 });
        if was_dead {
            self.events.push(Event::Revived { id, health, max_health });
        } else {
            self.events.push(Event::Health { id, health, max_health });
        }
        let (pos, facing) = (self.arena.player_spawn, self.arena.player_facing);
        self.events.push(Event::Teleport { player: id, pos, facing });
        self.hold_menus(id);
    }

    /// Hold back card windows for a moment (after a revive or a teleport).
    pub(super) fn hold_menus(&mut self, id: EntityId) {
        let until = self.clock + MENU_HOLD_SECS;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.menu_hold = p.menu_hold.max(until);
        }
    }

    /// How long a fallen player waits at this wave.
    pub(super) fn respawn_secs(&self) -> f32 {
        let g = &self.content.game;
        (g.respawn_secs + g.respawn_per_wave * self.run.wave.saturating_sub(1) as f32).min(g.respawn_cap)
    }

    /// Seconds until a fallen player is back up (`None` when standing).
    pub fn respawn_in(&self, id: EntityId) -> Option<f32> {
        let e = self.entities.get(&id)?;
        let p = e.player()?;
        (!e.alive() && p.respawn_at.is_finite()).then(|| (p.respawn_at - self.clock).max(0.0))
    }

    /// The fallen whose wait is over get up at the Ancient.
    pub(super) fn step_respawns(&mut self) {
        let now = self.clock;
        let due: Vec<EntityId> = self
            .player_ids()
            .into_iter()
            .filter(|id| {
                let e = &self.entities[id];
                !e.alive() && e.player().is_some_and(|p| p.respawn_at <= now)
            })
            .collect();
        for id in due {
            self.revive_player(id);
            let name = self.entities[&id].player().map(|p| p.name.clone()).unwrap_or_default();
            self.announce(format!("{name} is back in the fight!"), false);
        }
    }

    pub(super) fn revive_and_heal_players(&mut self) {
        for id in self.player_ids() {
            self.revive_player(id);
        }
    }

    fn revive_player(&mut self, id: EntityId) {
        let e = self.entities.get_mut(&id).unwrap();
        let was_dead = !e.alive();
        e.health = e.max_health;
        let (health, max_health) = (e.health, e.max_health);
        if let Some(p) = e.player_mut() {
            p.shield_charges = 3.0;
            p.necrotic = 0;
            p.respawn_at = f32::INFINITY;
        }
        self.events.push(if was_dead {
            Event::Revived { id, health, max_health }
        } else {
            Event::Health { id, health, max_health }
        });
        if was_dead {
            // The fallen get up at the hub.
            let (pos, facing) = (self.arena.player_spawn, self.arena.player_facing);
            self.entities.get_mut(&id).unwrap().pos = pos;
            self.events.push(Event::Teleport { player: id, pos, facing });
            self.bring_pet(id, pos, facing);
            self.hold_menus(id);
        }
    }

    /// A hunter's pet goes where its master was sent (home by Hearthstone, up at the hub after
    /// falling): at their heel, its fight dropped.
    pub(super) fn bring_pet(&mut self, id: EntityId, pos: Vec3, facing_: f32) {
        let Some(pet) = self.pet_of(id) else { return };
        let at = self.ring(pos, 2.5, facing_ + 2.4);
        let Some(e) = self.entities.get_mut(&pet) else { return };
        e.pos = at;
        e.facing = facing_;
        if let Body::Ally(a) = &mut e.body {
            a.mover = None;
            a.engaged = None;
        }
        self.events.push(Event::Stop { id: pet, pos: at, facing: facing_ });
    }

    // --- levels, experience, gold ---------------------------------------------------------------

    /// The player's own meter since they last reset it.
    pub fn meter(&self, id: EntityId) -> Option<MeterView> {
        let p = self.entities.get(&id)?.player()?;
        let mut rows = p.meter.by.clone();
        rows.sort_by(|a, b| b.1.cmp(&a.1));
        Some(MeterView { secs: (self.clock - p.meter.since).max(0.0), total: p.meter.total, rows })
    }

    /// Start the player's own meter over.
    pub fn reset_meter(&mut self, id: EntityId) {
        let now = self.clock;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.meter.total = 0;
            p.meter.by.clear();
            p.meter.since = now;
        }
    }

    /// Every hero's damage this wave (the last wave's, between waves), and the base's and the
    /// soldiers' once they've landed a hit, most first.
    pub fn party_meter(&self) -> Vec<PartyMeterRow> {
        let mut rows: Vec<PartyMeterRow> = self
            .order
            .iter()
            .filter_map(|id| self.entities.get(id).and_then(|e| e.player()))
            .map(|p| PartyMeterRow { name: p.name.clone(), class: self.content.classes[p.class].key.clone(), damage: p.meter.wave })
            .collect();
        for (i, (name, class)) in [("Base defences", "base"), ("Soldiers", "soldiers")].into_iter().enumerate() {
            if self.run.defence_meter[i] > 0 {
                rows.push(PartyMeterRow { name: name.into(), class: class.into(), damage: self.run.defence_meter[i] });
            }
        }
        rows.sort_by(|a, b| b.damage.cmp(&a.damage));
        rows
    }

    /// What a hit counts as on its hero's meter: the spell, Auto Attack, or the pet.
    pub(super) fn meter_label(&self, attacker: EntityId, owner: EntityId, via: super::enemies::Via) -> String {
        if attacker != owner {
            return match self.entities.get(&attacker).map(|e| &e.body) {
                Some(Body::Ally(a)) if a.pet => self.content.pet.name.clone(),
                _ => "Allies".into(),
            };
        }
        let Some(p) = self.entities.get(&owner).and_then(|e| e.player()) else { return "Other".into() };
        match via {
            super::enemies::Via::Melee => "Auto Attack".into(),
            // The ranged auto-attacks: a hunter's bow, a caster's wand.
            super::enemies::Via::Spell { spell: 75, .. } => "Auto Shot".into(),
            super::enemies::Via::Spell { spell: 5019, .. } => "Shoot".into(),
            super::enemies::Via::Spell { spell, periodic, .. } => {
                let kit = self.content.kit(p.class);
                if let Some(o) = p.spells.iter().find(|o| kit.spells[o.spell].icon == spell) {
                    return kit.spells[o.spell].name.clone();
                }
                if let Some(o) = p.cards.iter().find(|o| self.content.cards[o.card].spell == spell) {
                    return self.content.cards[o.card].name.clone();
                }
                if periodic { "Damage over time".into() } else { "Other effects".into() }
            }
        }
    }

    /// The highest level a hero's kills can take them to now: a level for each wave begun
    /// (`levels_per_wave`), plus the bonus levels their objectives earned.
    pub(super) fn level_cap(&self, id: EntityId) -> u32 {
        let l = &self.content.levels;
        let bonus = self.entities.get(&id).and_then(|e| e.player()).map_or(0, |p| p.bonus_levels);
        (1 + self.run.wave * l.levels_per_wave + bonus).min(l.max_level)
    }

    pub(super) fn give_xp(&mut self, id: EntityId, amount: f32, victim: Option<EntityId>) {
        let cap = self.level_cap(id);
        let levels = self.content.levels.clone();
        let Some(e) = self.entities.get_mut(&id) else { return };
        let Some(p) = e.player_mut() else { return };
        // At the cap the bar fills to the brim and waits for the next wave.
        let room = if p.level >= cap { 0 } else { levels.xp_total(cap) - levels.xp_total(p.level) - p.xp as u64 };
        if room == 0 {
            let wave = self.run.wave;
            if p.capped_told != wave {
                p.capped_told = wave;
                self.tell(id, "You've learned all this wave can teach you: your next level comes with the next wave (bosses and new bases give bonus levels).".into());
            }
            return;
        }
        // Two or more levels behind (joined late, or died a lot): catch up faster.
        let boost = if p.level + 2 < cap { 1.5 } else { 1.0 };
        let amount = ((amount * boost).round().max(1.0) as u64).min(room) as u32;
        p.xp += amount;
        self.events.push(Event::Xp { player: id, amount, victim });
        let mut level = p.level;
        let mut xp = p.xp;
        while level < cap && xp >= levels.xp_to_next(level) {
            xp -= levels.xp_to_next(level);
            level += 1;
        }
        if level != p.level {
            p.xp = xp;
            self.set_level(id, level);
        }
    }

    /// An objective done: every hero gets a level now, past the wave's cap.
    pub(super) fn bonus_level(&mut self, levels: u32, why: &str) {
        if levels == 0 {
            return;
        }
        let max = self.content.levels.max_level;
        for id in self.player_ids() {
            let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { continue };
            p.bonus_levels += levels;
            let level = (p.level + levels).min(max);
            if level != p.level {
                self.set_level(id, level);
            }
        }
        self.announce(format!("{why}: a bonus level for everyone!"), false);
    }

    pub(super) fn set_level(&mut self, id: EntityId, level: u32) {
        let before = self.entities[&id].max_health;
        let old_level = self.entities[&id].level;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.level = level;
            if level >= self.content.levels.max_level {
                p.xp = 0;
            }
        }
        self.entities.get_mut(&id).unwrap().level = level;
        self.recompute(id);
        let e = self.entities.get_mut(&id).unwrap();
        let gained = e.max_health as i32 - before as i32;
        if e.alive() && gained > 0 {
            e.health = (e.health + gained as u32).min(e.max_health);
        }
        let (health, max_health) = (e.health, e.max_health);
        self.events.push(Event::Level { player: id, level, gained_health: gained });
        self.events.push(Event::Health { id, health, max_health });
        self.wear_from_bags(id);
        self.events.push(Event::Talents { player: id });
        // Every level brings one pick: a talent, a spell or an upgrade.
        for l in old_level + 1..=level {
            if level_pick(l) {
                self.offer(id, cards::OfferSource::Level);
            }
        }
    }

    pub(super) fn give_gold(&mut self, id: EntityId, amount: u32) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.gold = p.gold.saturating_add(amount);
            let gold = p.gold;
            self.events.push(Event::Gold { player: id, gold });
        }
    }

    /// Gold a hero earns (kills, loot, bounties): a Miser's Band adds to it.
    pub(super) fn earn_gold(&mut self, id: EntityId, amount: u32) {
        let find = self.entities.get(&id).and_then(|e| e.player()).map_or(0.0, |p| p.stats.gold_find);
        self.give_gold(id, (amount as f32 * (1.0 + find.max(0.0))).round() as u32);
    }

    pub(super) fn take_gold(&mut self, id: EntityId, amount: u32) -> bool {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return false };
        if p.gold < amount {
            return false;
        }
        p.gold -= amount;
        let gold = p.gold;
        self.events.push(Event::Gold { player: id, gold });
        true
    }

    // --- stats ------------------------------------------------------------------------------------

    /// The effects of the wares a player wears, and of the powers they hold.
    pub(super) fn worn_ware_effects<'a>(&'a self, p: &Player) -> Vec<&'a crate::kit::TalentEffect> {
        p.equip
            .iter()
            .flatten()
            .filter_map(|it| items::decode_ware(it.entry))
            .filter_map(|(w, _, _)| self.content.wares.get(w))
            .flat_map(|w| w.effects_for(p.class).iter())
            .chain(p.powers.iter().filter_map(|&k| self.content.powers.get(k)).flat_map(|pw| pw.effects_for(p.class).iter()))
            .collect()
    }

    /// Work out a player's numbers from level, gear and cards.
    pub(super) fn recompute(&mut self, id: EntityId) {
        let Some(e) = self.entities.get(&id) else { return };
        let Some(p) = e.player() else { return };
        let class = &self.content.classes[p.class];
        let levels = &self.content.levels;
        let budget = &self.content.gear.budget;
        let mut s = Stats {
            damage: 1.0,
            haste: 0.0,
            crit: 0.05,
            speed: 7.0,
            heal: 1.0,
            radius: 1.0,
            ..Stats::default()
        };
        // Gear.
        let weapon_slot = class.weapon_slot as usize;
        let mut weapon = None;
        let mut tags: Vec<(String, u32)> = Vec::new();
        for (i, it) in p.equip.iter().enumerate() {
            let Some(it) = it else { continue };
            let Some(t) = items::template(&self.content, it.entry) else { continue };
            s.armor += t.armor;
            s.strength += t.stat(items::stat::STRENGTH);
            s.agility += t.stat(items::stat::AGILITY);
            s.stamina += t.stat(items::stat::STAMINA);
            s.intellect += t.stat(items::stat::INTELLECT);
            s.spirit += t.stat(items::stat::SPIRIT);
            if t.set_piece {
                s.set_pieces += 1;
            }
            if i == weapon_slot && t.damage.is_some() {
                weapon = Some(t.clone());
                if let Some(tag) = enchant_tag(it.enchant) {
                    tags.push((tag.to_string(), 1));
                    s.enchant_school = crate::kit::school_index(tag);
                }
            }
        }
        let level_damage = levels.mult(levels.damage_per_level, p.level);
        let level_health = levels.mult(levels.health_per_level, p.level);
        let (lo, hi, secs) = match &weapon {
            Some(w) => {
                let (a, b) = w.damage.unwrap();
                (a, b, w.delay_ms as f32 / 1000.0)
            }
            None => (class.damage[0] as f32 * 0.4, class.damage[1] as f32 * 0.4, class.attack_secs),
        };
        s.hit_lo = lo * level_damage;
        s.hit_hi = hi * level_damage;
        s.attack_secs = secs;
        let main = match class.main_stat.as_str() {
            "strength" => s.strength,
            "agility" => s.agility,
            "spirit" => s.spirit,
            _ => s.intellect,
        };
        s.damage *= 1.0 + main as f32 * budget.main_stat_damage;
        s.crit += s.agility as f32 * 0.0005;
        let mut health_mult = 1.0 + s.stamina as f32 * budget.stamina_health;
        s.regen = self.content.game.player_regen_per_sec + s.spirit as f32 * budget.spirit_regen;
        s.reduction = self.armor_reduction(s.armor as f32);
        s.taken += class.taken;
        // Cards.
        let mut damage_add = 0.0;
        let mut haste_add = 0.0;
        let mut undying = false;
        for o in &p.cards {
            let c = &self.content.cards[o.card];
            for t in &c.tags {
                match tags.iter_mut().find(|(k, _)| k == t) {
                    Some((_, n)) => *n += 1,
                    None => tags.push((t.clone(), 1)),
                }
            }
            let a = c.amount * c.rank_mult(o.rank);
            match c.kind {
                crate::content::CardKind::Stat => match c.stat.as_str() {
                    "damage" => damage_add += a,
                    "haste" => haste_add += a,
                    "crit" => s.crit += a,
                    "health" => health_mult += a,
                    "regen" => s.regen += a,
                    "speed" => s.speed *= 1.0 + a,
                    "per_wave" => s.per_wave += a,
                    "leech" => s.leech += a,
                    "execute" => s.execute += a,
                    "avatar" => {
                        damage_add += a;
                        health_mult += a;
                    }
                    _ => {}
                },
                crate::content::CardKind::Modifier => match c.modifier.as_str() {
                    "cdr" => s.cdr += a,
                    "radius" => s.radius += a,
                    "echo" => s.echo += a,
                    "cleave" => s.cleave += a,
                    "dot" => {
                        s.dot += a;
                        s.dot_secs = c.secs;
                        s.dot_spell = c.spell;
                        s.dot_school = c.school;
                    }
                    "windfury" => s.windfury += a,
                    "burn" => {
                        s.burn += a;
                        s.burn_secs = c.secs;
                    }
                    "split" => s.split += a,
                    _ => {}
                },
                crate::content::CardKind::Trigger => match c.trigger.as_str() {
                    "lightning_shield" => s.lightning += a,
                    "bloodthirst" => s.bloodthirst += a,
                    "divine_shield" => {
                        s.divine_secs = a;
                        s.divine_cooldown = c.cooldown;
                    }
                    "raise_dead" => {
                        s.raise_dead += a;
                        s.raise_secs = c.secs;
                    }
                    "soul_harvest" => s.soul_harvest += a,
                    "undying" => undying = true,
                    "thorns" => s.thorns += a,
                    "shield" => {
                        s.shield += a;
                        s.shield_cooldown = c.cooldown.max(3.0);
                    }
                    _ => {}
                },
                crate::content::CardKind::Team => match c.team.as_str() {
                    "damage_taken" => {
                        s.devotion += a;
                        s.devotion_radius = c.radius;
                    }
                    "damage" => {
                        s.shout += a;
                        s.shout_radius = c.radius;
                    }
                    "haste" => {
                        s.team_haste += a;
                        s.team_haste_radius = c.radius;
                    }
                    "regen" => {
                        s.team_regen += a;
                        s.team_regen_radius = c.radius;
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        // Talents, and the wares worn.
        let kit = self.content.kit(p.class);
        let mut acc = Adds { damage: damage_add, haste: haste_add, health: health_mult, undying };
        for (ti, t) in kit.talents.iter().enumerate() {
            let rank = p.talents.get(ti).copied().unwrap_or(0) as f32;
            if rank == 0.0 {
                continue;
            }
            for e in &t.effects {
                apply_effect(&mut s, &mut acc, e, rank);
            }
        }
        for e in self.worn_ware_effects(p) {
            apply_effect(&mut s, &mut acc, e, 1.0);
        }
        let Adds { damage: mut damage_add, haste: haste_add, health: mut health_mult, undying } = acc;
        // Fortitude from anyone in the team.
        let team_health = self
            .order
            .iter()
            .filter_map(|o| self.entities.get(o).and_then(|e| e.player()))
            .map(|op| team_health_of(&self.content, op))
            .fold(0.0f32, f32::max);
        health_mult += team_health;
        let now = self.clock;
        for b in p.buffs.iter().filter(|b| b.until > now) {
            if b.kind == BuffKind::LastStand {
                health_mult *= 1.0 + b.value;
            }
        }
        // Set bonuses (2 pieces: the class's first bonus).
        let class_key = class.key.as_str();
        if s.set_pieces >= 2 {
            match class_key {
                "warrior" => damage_add += 0.10,
                "priest" => s.heal += 0.20,
                _ => {}
            }
        }
        if class_key == "priest" && s.set_pieces >= 6 {
            if let Some((_, n)) = tags.iter_mut().find(|(k, _)| k == "holy") {
                *n *= 2;
            }
        }
        s.damage *= 1.0 + damage_add;
        s.haste = haste_add;
        s.attack_secs /= 1.0 + s.haste;
        s.tag_counts = tags;
        if self.boon("the Stables") {
            s.speed *= 1.15;
        }
        if self.boon("the Farm") {
            s.regen += 0.01;
        }
        s.max_health = (class.health as f32 * level_health * health_mult).round().max(1.0) as u32;
        let max = s.max_health;
        let speed = s.speed;
        let e = self.entities.get_mut(&id).unwrap();
        e.max_health = max;
        if e.health > max {
            e.health = max;
        }
        let p = e.player_mut().unwrap();
        p.stats = s;
        p.undying = undying;
        let speed_changed = (speed - p.sent_speed).abs() > 0.01;
        self.recompute_spells(id);
        self.events.push(Event::Stats { player: id });
        if speed_changed {
            self.update_speed(id);
        }
        self.refresh_auras(id);
    }

    /// Re-send the player's run speed if slows or buffs changed it.
    pub(super) fn update_speed(&mut self, id: EntityId) {
        let now = self.clock;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        let mut speed = p.stats.speed;
        if p.chilled_until > now {
            speed *= 0.8;
        }
        if p.sprinting && p.sprint_until > now {
            speed *= 1.0 + self.content.sprint.speed;
        }
        if let Some(b) = p.buffs.iter().find(|b| b.kind == BuffKind::Accelerated && b.until > now) {
            speed *= 1.0 + b.value;
        }
        let tspeed: f32 = p.tbuffs.iter().filter(|b| b.until > now).map(|b| b.def.speed * b.stacks as f32).sum();
        speed *= (1.0 + tspeed).max(0.3);
        if (speed - p.sent_speed).abs() > 0.01 {
            p.sent_speed = speed;
            self.events.push(Event::Stats { player: id });
        }
    }

    /// The player's current run speed.
    pub fn player_speed(&self, id: EntityId) -> f32 {
        self.entities.get(&id).and_then(|e| e.player()).map_or(7.0, |p| p.sent_speed)
    }

    pub(super) fn chill_player(&mut self, id: EntityId) {
        let now = self.clock;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.chilled_until = now + 2.0;
        }
        self.update_speed(id);
    }

    /// Whether the player's build is resonant in `school` (3+ cards with the tag).
    pub(super) fn resonant(&self, id: EntityId, school: &str) -> bool {
        self.entities
            .get(&id)
            .and_then(|e| e.player())
            .is_some_and(|p| p.stats.tag_counts.iter().any(|(k, n)| k == school && *n >= 3))
    }

    /// Like [`Sim::resonant`], for an attacker that may be an ally (its owner's build counts).
    pub(super) fn resonant_any(&self, attacker: EntityId, school: &str) -> bool {
        match self.entities.get(&attacker).map(|e| &e.body) {
            Some(Body::Player(_)) => self.resonant(attacker, school),
            Some(Body::Ally(a)) => self.resonant(a.owner, school),
            _ => false,
        }
    }

    /// The player's "hit": weapon damage per second times two, with every damage bonus. Card
    /// damage is a multiple of this.
    pub(super) fn player_hit(&self, id: EntityId) -> f32 {
        let Some(e) = self.entities.get(&id) else { return 0.0 };
        let Some(p) = e.player() else { return 0.0 };
        let s = &p.stats;
        let base = (s.hit_lo + s.hit_hi) / 2.0 / (s.attack_secs * (1.0 + s.haste)).max(0.5) * 2.0;
        base * s.damage * self.dynamic_damage(id)
    }

    /// Damage bonuses that change during a run: team auras, Soul Harvest, Mark of the Ancient.
    pub(super) fn dynamic_damage(&self, id: EntityId) -> f32 {
        let Some(e) = self.entities.get(&id) else { return 1.0 };
        let Some(p) = e.player() else { return 1.0 };
        let mut m = 1.0;
        m *= 1.0 + p.stats.per_wave * self.run.wave as f32;
        m *= 1.0 + p.stats.soul_harvest * (p.kills / 10).min(SOUL_HARVEST_STACKS) as f32;
        let pos = e.pos;
        let shout = self
            .order
            .iter()
            .filter_map(|o| self.entities.get(o))
            .filter_map(|o| o.player().map(|op| (o.pos, op.stats.shout, op.stats.shout_radius)))
            .filter(|(op, a, r)| *a > 0.0 && dist2d(*op, pos) <= *r)
            .map(|(_, a, _)| a)
            .fold(0.0f32, f32::max);
        m *= 1.0 + shout;
        if p.together {
            m *= 1.0 + TOGETHER_BONUS;
        }
        m
    }

    /// The best team aura of one kind reaching `id`: (kind getter) over every player in range.
    pub(super) fn team_aura(&self, id: EntityId, pick: fn(&Stats) -> (f32, f32)) -> f32 {
        let Some(e) = self.entities.get(&id) else { return 0.0 };
        let pos = e.pos;
        self.order
            .iter()
            .filter_map(|o| self.entities.get(o))
            .filter_map(|o| o.player().map(|op| (o.pos, pick(&op.stats))))
            .filter(|(op, (a, r))| *a > 0.0 && dist2d(*op, pos) <= *r)
            .map(|(_, (a, _))| a)
            .fold(0.0f32, f32::max)
    }

    // --- the fight --------------------------------------------------------------------------------

    pub(super) fn step_players(&mut self, dt: f32) {
        let fighting = self.combat_on();
        let ids = self.player_ids();
        // Shoulder to Shoulder: heroes who hold a spot together hit harder and take less.
        for &id in &ids {
            let near = self.entities.get(&id).is_some_and(|e| {
                e.alive()
                    && ids.iter().any(|&o| o != id && self.entities.get(&o).is_some_and(|x| x.alive() && dist2d(x.pos, e.pos) <= TOGETHER_YARDS))
            });
            let was = self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.together);
            if near != was {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.together = near;
                }
                self.refresh_auras(id);
            }
        }
        for &id in &ids {
            self.player_regen(id, dt);
            self.step_channel(id, dt);
            // Lightning Shield recharges a charge every 3 seconds.
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.shield_charges = (p.shield_charges + dt / 3.0).min(3.0);
                if p.necrotic_until < self.clock {
                    p.necrotic = 0;
                }
            }
            let chilled = self.entities[&id].player().is_some_and(|p| p.chilled_until > 0.0 && p.chilled_until <= self.clock);
            if chilled {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.chilled_until = 0.0;
                }
                self.update_speed(id);
            }
        }
        for id in ids {
            let (pos, alive, target, class) = {
                let e = &self.entities[&id];
                let Some(p) = e.player() else { continue };
                (e.pos, e.alive(), p.attacking, p.class)
            };
            let Some(target) = target else { continue };
            let target_alive = self.entities.get(&target).is_some_and(|t| t.alive());
            if !alive || !target_alive || !fighting {
                self.stop_attack(id);
                continue;
            }
            let def = self.content.classes[class].clone();
            let tpos = self.entities[&target].pos;
            let reach = if def.ranged() { def.range } else { MELEE_RANGE + UNIT_RADIUS };
            let in_reach = dist2d(pos, tpos) <= reach;
            let haste = (1.0 + self.team_aura(id, |s| (s.team_haste, s.team_haste_radius))) * self.attack_haste(id);
            let (swing_ready, attack_secs) = {
                let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                p.swing += dt * haste;
                (p.swing >= p.stats.attack_secs, p.stats.attack_secs)
            };
            if !swing_ready {
                continue;
            }
            if !in_reach {
                // Hold the swing ready and tell the player, once a swing's worth.
                self.events.push(Event::NotInRange { player: id });
                let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                p.swing = (attack_secs - 1.0).max(0.0);
                continue;
            }
            let (lo, hi, crit_chance, damage_mult) = {
                let p = self.entities.get_mut(&id).unwrap().player_mut().unwrap();
                p.swing -= attack_secs;
                p.swung_at = self.clock;
                p.autos += 1;
                (p.stats.hit_lo, p.stats.hit_hi, p.stats.crit, p.stats.damage)
            };
            let auto = self.entities[&id].player().unwrap().stats.auto;
            let mut damage = self.rng.gen_range(lo..=hi.max(lo)) * damage_mult * self.dynamic_damage(id) * (1.0 + auto) * self.buff_damage(id, None);
            let crit = self.rng.gen::<f32>() < crit_chance + self.tbuff(id, |b| b.crit);
            if crit {
                damage *= 2.0;
            }
            // Flurry spends a charge a swing.
            if let Some(b) = self.buff(id, BuffKind::Flurry) {
                if b.stacks <= 1 {
                    self.drop_buff(id, BuffKind::Flurry);
                } else if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    if let Some(f) = p.buffs.iter_mut().find(|x| x.kind == BuffKind::Flurry) {
                        f.stacks -= 1;
                    }
                }
            }
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.fighting_until = self.clock + 6.0;
            }
            if def.ranged() {
                let spell = def.attack_spell;
                self.events.push(Event::SpellGo { caster: id, spell, target: SpellTarget::Unit(target), hits: vec![target] });
                // Every 10th Auto Shot with the hunter's full set is an Aimed Shot.
                let autos = self.entities[&id].player().unwrap().autos;
                let pieces = self.entities[&id].player().unwrap().stats.set_pieces;
                let (spell, damage) = if def.key == "hunter" && pieces >= 6 && autos % 10 == 0 {
                    self.events.push(Event::SpellGo { caster: id, spell: 19434, target: SpellTarget::Unit(target), hits: vec![target] });
                    (19434, damage * 4.0)
                } else {
                    (spell, damage)
                };
                let delay = (dist2d(pos, tpos) / 30.0).min(1.0);
                self.queue_hit(id, target, spell, damage, crit, 0, delay, cards::HitFx::auto());
                // Quick Shots.
                let quick = self.entities[&id].player().unwrap().stats.procs.quick_shots;
                if quick > 0.0 && self.rng.gen::<f32>() < quick {
                    self.give_buff(id, BuffKind::QuickShots, 8.0, 0.3, 1);
                }
            } else {
                let (m, _) = self.against(id, target, 0, &crate::kit::Nums::default(), false);
                let damage = damage * m;
                self.deal(id, target, damage.round() as u32, crit, Via::Melee);
                self.rage_from_swing(id, attack_secs);
                self.after_auto_hit(id, target, damage, crit);
                self.fire(id, crate::kit::On::Swing, None, Some(target), 0.0);
                if crit {
                    self.fire(id, crate::kit::On::SwingCrit, None, Some(target), 0.0);
                }
                // Sweeping Strikes: the swing also hits one more.
                if let Some(b) = self.buff(id, BuffKind::SweepingStrikes) {
                    let near: Option<EntityId> = self.enemies_near(tpos, 8.0).into_iter().map(|(o, _)| o).find(|o| *o != target);
                    if let Some(o) = near {
                        self.deal(id, o, damage.round() as u32, false, Via::Melee);
                        if b.stacks <= 1 {
                            self.drop_buff(id, BuffKind::SweepingStrikes);
                        } else if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                            if let Some(x) = p.buffs.iter_mut().find(|x| x.kind == BuffKind::SweepingStrikes) {
                                x.stacks -= 1;
                            }
                        }
                    }
                }
                // Sword Specialization: an extra swing.
                let extra = self.entities[&id].player().unwrap().stats.procs.extra_attack;
                if extra > 0.0 && self.rng.gen::<f32>() < extra && self.entities.get(&target).is_some_and(|t| t.alive()) {
                    self.events.push(Event::SpellGo { caster: id, spell: 12281, target: SpellTarget::Unit(target), hits: vec![target] });
                    self.deal(id, target, damage.round() as u32, false, Via::Melee);
                    self.rage_from_swing(id, attack_secs * 0.5);
                }
                // Every 4th swing with the warrior's full set is a free Whirlwind.
                let autos = self.entities[&id].player().unwrap().autos;
                let pieces = self.entities[&id].player().unwrap().stats.set_pieces;
                if def.key == "warrior" && pieces >= 6 && autos % 4 == 0 {
                    self.free_cast(id, "whirlwind");
                }
            }
            // Being hit pulls an enemy off the hub and onto you.
            if let Some(en) = self.entities.get_mut(&target).and_then(|e| e.enemy_mut()) {
                if !matches!(en.goal, Goal::Player(_)) {
                    en.goal = Goal::Player(id);
                    en.think = 0.0;
                }
            }
        }
    }

    /// What an auto-attack hit brings with it: cleave, damage over time, Bloodthirst.
    pub(super) fn after_auto_hit(&mut self, id: EntityId, target: EntityId, damage: f32, crit: bool) {
        if crit {
            self.on_crit(id, target, damage, 0, false);
        }
        let s = self.entities[&id].player().unwrap().stats.clone();
        let tpos = match self.entities.get(&target) {
            Some(t) => t.pos,
            None => return,
        };
        let mut cleave = s.cleave;
        if self.resonant(id, "physical") {
            cleave += 0.3;
        }
        if cleave > 0.0 {
            if let Some((other, _)) = self.enemies_near(tpos, 6.0).into_iter().find(|(o, _)| *o != target) {
                self.deal(id, other, (damage * cleave).round().max(1.0) as u32, false, Via::Melee);
            }
        }
        if s.dot > 0.0 && self.entities.get(&target).is_some_and(|t| t.alive()) {
            let hit = self.player_hit(id);
            self.add_dot(id, target, s.dot_spell, s.dot_school, hit * s.dot, s.dot_secs, true);
        }
        if crit && s.bloodthirst > 0.0 {
            let heal = (self.entities[&id].max_health as f32 * s.bloodthirst) as u32;
            self.heal(id, id, 23881, heal, false);
        }
        // Windfury: a free extra attack (never chaining into another).
        let leech = self.tbuff(id, |b| b.leech);
        if leech > 0.0 {
            self.heal(id, id, 0, (damage * leech).round() as u32, false);
        }
        if s.windfury > 0.0 && self.rng.gen::<f32>() < s.windfury.min(0.9) && self.entities.get(&target).is_some_and(|t| t.alive()) {
            self.events.push(Event::SpellGo { caster: id, spell: 8516, target: SpellTarget::Unit(target), hits: vec![target] });
            self.deal(id, target, damage.round().max(1.0) as u32, false, Via::Spell { spell: 8516, school: 0, periodic: false });
        }
    }

    fn player_regen(&mut self, id: EntityId, dt: f32) {
        let team_regen = self.team_aura(id, |s| (s.team_regen, s.team_regen_radius));
        self.shield_tick(id);
        let e = self.entities.get_mut(&id).unwrap();
        let (alive, full, max) = (e.alive(), e.health >= e.max_health, e.max_health);
        let p = e.player_mut().unwrap();
        if !alive || full {
            p.regen = 0.0;
            return;
        }
        p.regen += dt;
        if p.regen >= 1.0 {
            p.regen -= 1.0;
            let amount = (max as f32 * (p.stats.regen + team_regen) * (1.0 - 0.05 * p.necrotic as f32)).round() as u32;
            e.health = (e.health + amount.max(1)).min(max);
            let health = e.health;
            self.events.push(Event::Health { id, health, max_health: max });
        }
    }

    /// Heal `target`; returns how much it took.
    pub(super) fn heal(&mut self, healer: EntityId, target: EntityId, spell: u32, amount: u32, crit: bool) -> u32 {
        let mult = self.entities.get(&healer).and_then(|e| e.player()).map_or(1.0, |p| p.stats.heal);
        let necrotic = self.entities.get(&target).and_then(|e| e.player()).map_or(0, |p| p.necrotic);
        let amount = (amount as f32 * mult * (1.0 - 0.05 * necrotic as f32)).round() as u32;
        let Some(e) = self.entities.get_mut(&target) else { return 0 };
        if !e.alive() || amount == 0 {
            return 0;
        }
        let before = e.health;
        e.health = (e.health + amount).min(e.max_health);
        let (health, max_health) = (e.health, e.max_health);
        self.events.push(Event::Heal { healer, target, spell, amount, crit });
        if health != before {
            self.events.push(Event::Health { id: target, health, max_health });
        }
        // A priest's healing spills onto the hired defenders standing by, and blesses them.
        if self.entities.get(&target).is_some_and(|e| e.is_player()) {
            self.bless_defenders(healer, target, spell, amount);
        }
        let inspiration = self.entities.get(&healer).and_then(|e| e.player()).map_or(0.0, |p| p.stats.procs.inspiration);
        if inspiration > 0.0 && healer != target && self.entities.get(&target).is_some_and(|e| e.is_player()) {
            self.give_buff(target, BuffKind::Inspiration, 10.0, inspiration, 1);
        }
        health - before
    }

    /// Damage reaching a player: armour, auras, talents and buffs, blocks, shields. Returns
    /// what's left of it.
    /// Armour against blows from the wave's enemies, as in 1.12: armour / (armour + k), where k
    /// grows with the enemies' level, so the armour on new gear keeps pace rather than running
    /// away with it. Three quarters at most.
    pub(super) fn armor_reduction(&self, armor: f32) -> f32 {
        let budget = &self.content.gear.budget;
        let k = budget.armor_k + budget.armor_k_per_level * self.enemy_level() as f32;
        (armor / (armor + k.max(1.0))).min(0.75)
    }

    pub(super) fn player_takes(&mut self, id: EntityId, attacker: EntityId, damage: u32) -> u32 {
        let now = self.clock;
        let reduction = self.entities.get(&id).and_then(|e| e.player()).map_or(0.0, |p| self.armor_reduction(p.stats.armor as f32));
        let pos = self.entities[&id].pos;
        // The strongest Devotion Aura in range.
        let devotion = self
            .order
            .iter()
            .filter_map(|o| self.entities.get(o))
            .filter_map(|o| o.player().map(|op| (o.pos, op.stats.devotion, op.stats.devotion_radius)))
            .filter(|(op, a, r)| *a > 0.0 && dist2d(*op, pos) <= *r)
            .map(|(_, a, _)| a)
            .fold(0.0f32, f32::max);
        let buffs = self.buff_taken(id);
        let block = self.entities[&id].player().map_or(0.0, |p| p.stats.procs.block);
        let blocked = block > 0.0 && self.rng.gen::<f32>() < block;
        let e = self.entities.get_mut(&id).unwrap();
        let health = e.health;
        let max = e.max_health;
        let p = e.player_mut().unwrap();
        if p.divine_until > now {
            return 0;
        }
        p.stats.reduction = reduction;
        let mut d = damage as f32 * (1.0 - reduction) * (1.0 - devotion.min(0.6)) * (1.0 - p.stats.taken.min(0.6)) * buffs;
        if p.together {
            d *= 1.0 - TOGETHER_BONUS;
        }
        if blocked {
            d *= 0.5;
        }
        // Divine Shield when health drops low.
        let after = health as f32 - d;
        if p.stats.divine_secs > 0.0 && p.divine_ready <= now && after < max as f32 * 0.35 {
            p.divine_until = now + p.stats.divine_secs;
            p.divine_ready = now + p.stats.divine_cooldown.max(10.0);
            let secs = p.stats.divine_secs;
            self.events.push(Event::SpellGo { caster: id, spell: 642, target: SpellTarget::Unit(id), hits: vec![id] });
            self.events.push(Event::Aura { unit: id, slot: 31, spell: 1050, stacks: 1, secs });
            d = d.min((health as f32 - max as f32 * 0.35).max(0.0));
        }
        // A shield soaks it up first.
        let mut reflected = 0.0;
        let mut burst = None;
        if p.shield_pool > 0.0 && d > 0.0 && (p.shield_until > now || p.stats.shield > 0.0) {
            let soak = p.shield_pool.min(d);
            p.shield_pool -= soak;
            d -= soak;
            reflected = soak * p.shield_reflect;
            if p.shield_pool <= 0.0 {
                self.events.push(Event::Aura { unit: id, slot: 30, spell: 0, stacks: 0, secs: 0.0 });
                if p.shield_burst > 0.0 {
                    burst = Some((p.shield_caster, p.shield_burst));
                }
            }
        }
        // Hits push a channel back a little.
        if let Some(c) = p.channel.as_mut() {
            if c.pushed < MAX_PUSHBACK && d > 0.0 {
                let push = PUSHBACK.min(MAX_PUSHBACK - c.pushed);
                c.pushed += push;
                c.left += push;
                let ms = (c.left * 1000.0) as u32;
                self.events.push(Event::ChannelLeft { player: id, ms });
            }
        }
        let martyrdom = p.stats.procs.martyrdom;
        let block_heal = p.stats.block_heal;
        let left = d.round().max(if d > 0.0 { 1.0 } else { 0.0 }) as u32;
        if blocked {
            self.gain_rage(id, 5.0);
            self.events.push(Event::SpellGo { caster: id, spell: 2565, target: SpellTarget::Unit(id), hits: vec![id] });
            if block_heal > 0.0 {
                self.heal(id, id, 2565, (max as f32 * block_heal).round() as u32, false);
            }
            let by = self.entities.get(&attacker).filter(|a| a.is_enemy()).map(|_| attacker);
            self.fire(id, crate::kit::On::Block, None, by, 0.0);
        }
        if left > 0 {
            let by = self.entities.get(&attacker).filter(|a| a.is_enemy()).map(|_| attacker);
            self.fire(id, crate::kit::On::Hurt, None, by, left as f32 / max.max(1) as f32);
            self.rage_from_hit(id, left);
            if martyrdom > 0.0 && left as f32 > max as f32 * 0.08 && self.rng.gen::<f32>() < martyrdom {
                self.give_buff(id, BuffKind::FocusedCasting, 6.0, 0.3, 1);
            }
        }
        if reflected >= 1.0 && self.entities.get(&attacker).is_some_and(|a| a.alive() && a.is_enemy()) {
            self.deal(id, attacker, reflected.round() as u32, false, Via::Spell { spell: 33201, school: 1, periodic: false });
        }
        if let Some((caster, share)) = burst {
            let dmg = self.player_hit(caster) * share;
            let hits: Vec<EntityId> = self.enemies_near(pos, 8.0).into_iter().map(|(v, _)| v).collect();
            self.events.push(Event::SpellGo { caster: id, spell: 15237, target: SpellTarget::Unit(id), hits: hits.clone() });
            for v in hits {
                self.deal(caster, v, dmg.round().max(1.0) as u32, false, Via::Spell { spell: 15237, school: 1, periodic: false });
            }
        }
        left
    }

    pub(super) fn lightning_shield(&mut self, id: EntityId, attacker: EntityId) {
        let thorns = self.entities.get(&id).and_then(|e| e.player()).map_or(0.0, |p| p.stats.thorns);
        if thorns > 0.0 && self.entities.get(&attacker).is_some_and(|a| a.alive()) {
            let dmg = (self.player_hit(id) * thorns).round().max(1.0) as u32;
            self.deal(id, attacker, dmg, false, Via::Spell { spell: 467, school: 3, periodic: false });
        }
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
        if p.stats.lightning <= 0.0 || p.shield_charges < 1.0 {
            return;
        }
        let amount = p.stats.lightning;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.shield_charges -= 1.0;
        }
        let dmg = (self.player_hit(id) * amount).round().max(1.0) as u32;
        self.events.push(Event::SpellGo { caster: id, spell: 26364, target: SpellTarget::Unit(attacker), hits: vec![attacker] });
        self.deal(id, attacker, dmg, false, Via::Spell { spell: 26364, school: 3, periodic: false });
    }

    /// A player's kill: Raise Dead, resonance effects.
    pub(super) fn on_kill_triggers(&mut self, id: EntityId, victim: EntityId, pos: Vec3) {
        let s = match self.entities.get(&id).and_then(|e| e.player()) {
            Some(p) => p.stats.clone(),
            None => return,
        };
        let mut raise = s.raise_dead;
        if self.resonant(id, "shadow") {
            raise += 0.10;
        }
        if raise > 0.0 && self.rng.gen::<f32>() < raise {
            let secs = if s.raise_secs > 0.0 { s.raise_secs } else { 15.0 };
            self.events.push(Event::SpellGo { caster: id, spell: 17473, target: SpellTarget::Point(pos), hits: vec![] });
            self.spawn_ally(id, pos, secs);
        }
        // Fire resonance: a burning enemy's death spreads the burn.
        let burning = self
            .entities
            .get(&victim)
            .and_then(|e| e.enemy())
            .is_some_and(|en| en.dots.iter().any(|d| d.school == 2));
        let mage_four = self.entities[&id].player().is_some_and(|p| self.content.classes[p.class].key == "mage" && p.stats.set_pieces >= 4);
        if burning || (self.resonant(id, "fire") || mage_four) && s.burn > 0.0 {
            if self.resonant(id, "fire") || mage_four {
                if let Some((n, _)) = self.enemies_near(pos, 8.0).into_iter().find(|(n, _)| *n != victim) {
                    let hit = self.player_hit(id);
                    let total = hit * s.burn.max(0.25);
                    self.add_dot(id, n, 348, 2, total, s.burn_secs.max(4.0), false);
                }
            }
        }
    }

    // --- channels -----------------------------------------------------------------------------------

    pub(super) fn start_channel(&mut self, id: EntityId, what: ChannelKind, secs: f32, spell: u32) {
        let Some(e) = self.entities.get_mut(&id) else { return };
        let from = e.pos;
        let Some(p) = e.player_mut() else { return };
        p.channel = Some(Channel { what, left: secs, from, pushed: 0.0 });
        self.events.push(Event::Channel { player: id, spell, ms: (secs * 1000.0) as u32 });
    }

    fn step_channel(&mut self, id: EntityId, dt: f32) {
        let pos = self.entities[&id].pos;
        let done = {
            let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
            let Some(c) = p.channel.as_mut() else { return };
            if dist2d(c.from, pos) > 1.5 {
                let hearth = c.what == ChannelKind::Hearth;
                p.channel = None;
                self.events.push(Event::ChannelEnd { player: id });
                // A broken Hearthstone costs nothing: it's ready to try again at once.
                if hearth {
                    self.events.push(Event::CooldownCleared { player: id, spell: HEARTH_SPELL });
                }
                self.tell(id, "You moved: the channel was interrupted.".into());
                return;
            }
            c.left -= dt;
            if c.left > 0.0 {
                return;
            }
            let what = c.what;
            p.channel = None;
            what
        };
        self.events.push(Event::ChannelEnd { player: id });
        match done {
            ChannelKind::Claim(base) => self.claim_base(id, base),
            ChannelKind::Hearth => {
                // The cooldown starts when it gets you home, not when you start it.
                let ready = self.clock + self.content.game.hearth_cooldown;
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.hearth_ready = ready;
                }
                let (pos, facing) = (self.arena.player_spawn, self.arena.player_facing);
                self.entities.get_mut(&id).unwrap().pos = pos;
                self.events.push(Event::Teleport { player: id, pos, facing });
                self.bring_pet(id, pos, facing);
                // The Hearthstone's own effect, at home.
                self.events.push(Event::SpellGo { caster: id, spell: HEARTH_SPELL, target: SpellTarget::Unit(id), hits: vec![] });
                self.hold_menus(id);
            }
        }
    }

    /// A hero landed from a drop of `height` yards. The 1.12 rule: nothing under about 28 yards,
    /// then 1.8% of their health a yard, all of it by about 83. It can kill.
    pub fn fall(&mut self, id: EntityId, height: f32) -> u32 {
        let Some(e) = self.entities.get_mut(&id) else { return 0 };
        if !e.alive() || !e.is_player() {
            return 0;
        }
        let share = 0.018 * (height - 14.57) - 0.2426;
        if share <= 0.0 {
            return 0;
        }
        let damage = ((share * e.max_health as f32).round() as u32).max(1);
        e.health = e.health.saturating_sub(damage);
        let (health, max_health) = (e.health, e.max_health);
        self.events.push(Event::Environmental { id, kind: Hazard::Fall, damage });
        self.events.push(Event::Health { id, health, max_health });
        if health == 0 {
            self.kill(id, id);
        }
        damage
    }

    /// `.hearth`: channel back to the Ancient.
    pub fn hearth(&mut self, id: EntityId) {
        let now = self.clock;
        let Some(e) = self.entities.get(&id) else { return };
        if !e.alive() {
            return;
        }
        let Some(p) = e.player() else { return };
        if p.hearth_ready > now {
            let left = (p.hearth_ready - now).ceil() as u32;
            self.tell(id, format!("Your Hearthstone is ready in {left} seconds."));
            return;
        }
        if dist2d(e.pos, self.arena.hub) < 12.0 {
            self.tell(id, "You're already at the Ancient.".into());
            return;
        }
        if p.channel.is_some() {
            return;
        }
        let secs = self.content.game.hearth_secs;
        self.start_channel(id, ChannelKind::Hearth, secs, HEARTH_SPELL);
    }
}

impl Sim {
    /// Renew Power Word: Shield when it's due.
    fn shield_tick(&mut self, id: EntityId) {
        let now = self.clock;
        let max = self.entities[&id].max_health as f32;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if p.stats.shield <= 0.0 || p.shield_ready > now {
            return;
        }
        p.shield_ready = now + p.stats.shield_cooldown;
        p.shield_pool = max * p.stats.shield;
        self.events.push(Event::Aura { unit: id, slot: 30, spell: 1277, stacks: 1, secs: 0.0 });
    }
}

/// Whether reaching level `l` brings a card pick.
/// Soul Harvest counts kills up to this many tens.
pub(crate) const SOUL_HARVEST_STACKS: u32 = 200;

pub(super) fn level_pick(l: u32) -> bool {
    l >= 2
}

/// The card tag a weapon enchant adds.
pub(super) fn enchant_tag(enchant: u32) -> Option<&'static str> {
    match enchant {
        1897 => Some("physical"),
        803 => Some("fire"),
        1894 => Some("frost"),
        1900 => Some("holy"),
        1898 | 1899 => Some("shadow"),
        2504 => Some("arcane"),
        2564 => Some("nature"),
        _ => None,
    }
}

impl Sim {
    /// Renown changed: the new luck applies from the next run.
    pub fn set_luck(&mut self, id: EntityId, luck: Luck) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.luck = luck;
        }
    }

    /// The affixes this run has picked up, by name.
    pub fn affix_names(&self) -> Vec<String> {
        self.run.affixes.iter().map(|&i| self.content.affixes[i].name.clone()).collect()
    }
}

impl Sim {
    /// Gold for a hero in a soak run on the real map (tools only).
    pub fn give_gold_for_soak(&mut self, id: EntityId, gold: u32) {
        self.give_gold(id, gold);
    }
}

#[cfg(test)]
impl Sim {
    /// What an enemy blow of `damage` comes to on player `id`, after armour and toughness.
    pub fn damage_taken_for_test(&mut self, id: EntityId, damage: u32) -> u32 {
        let hub = self.hub;
        self.player_takes(id, hub, damage)
    }

    /// Put on ware `key` (tier 0), as if bought.
    pub fn give_ware_for_test(&mut self, id: EntityId, key: &str) {
        let w = self.content.wares.iter().position(|w| w.key == key).expect("a ware");
        let class = self.entities[&id].player().unwrap().class;
        self.take_ware(id, items::ware_entry(w, class, 0)).unwrap();
    }

    /// Jump the run to Escalation `cycle` (the gear tier follows it).
    pub fn set_cycle_for_test(&mut self, cycle: u32) {
        self.run.cycle = cycle;
    }

    pub fn give_gold_for_test(&mut self, id: EntityId, gold: u32) {
        self.give_gold(id, gold);
    }
    pub fn set_level_for_test(&mut self, id: EntityId, level: u32) {
        self.set_level(id, level);
    }
    pub fn close_all_menus_for_test(&mut self, id: EntityId) {
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.offers.clear();
            p.menu = None;
            p.menu_actions.clear();
        }
    }
    pub fn kill_for_test(&mut self, id: EntityId) {
        let hub = self.hub;
        if let Some(e) = self.entities.get_mut(&id) {
            e.health = 0;
        }
        self.kill(id, hub);
    }
    pub fn results_for_test(&self) -> Vec<PlayerResult> {
        self.player_results()
    }
    /// A hit of `share` of its max health, as a fight would deal it.
    pub fn strike_for_test(&mut self, id: EntityId, share: f32) {
        let d = (self.entities[&id].max_health as f32 * share) as u32;
        let hub = self.hub;
        self.deal(hub, id, d, false, super::enemies::Via::Melee);
    }
    pub fn hurt_for_test(&mut self, id: EntityId, share: f32) {
        let e = self.entities.get_mut(&id).unwrap();
        let d = (e.max_health as f32 * share) as u32;
        e.health = e.health.saturating_sub(d).max(1);
    }
}

/// The max-health share a player's Fortitude talent gives the team.
fn team_health_of(content: &Content, p: &Player) -> f32 {
    let kit = content.kit(p.class);
    let mut v = 0.0;
    for (ti, t) in kit.talents.iter().enumerate() {
        let rank = p.talents.get(ti).copied().unwrap_or(0) as f32;
        for e in &t.effects {
            for (k, x) in &e.team {
                if k == "health" {
                    v += x * rank;
                }
            }
        }
    }
    v
}

/// What a recompute adds up before it's applied: damage, haste and health shares, and Undying.
struct Adds {
    damage: f32,
    haste: f32,
    health: f32,
    undying: bool,
}

/// One talent's (or worn item's) effect on a hero's numbers, `rank` times over. Spell patches
/// are `recompute_spells`' business.
fn apply_effect(s: &mut Stats, acc: &mut Adds, e: &crate::kit::TalentEffect, rank: f32) {
    for (k, v) in &e.stats {
        let v = v * rank;
        match k.as_str() {
            "crit" => s.crit += v,
            "haste" => acc.haste += v,
            "damage" => acc.damage += v,
            "health" => acc.health += v,
            "regen" => s.regen += v,
            "speed" => s.speed *= 1.0 + v,
            "taken" => s.taken += v,
            "heal" => s.heal += v,
            "auto" => s.auto += v,
            "cdr" => s.cdr += v,
            "absorb" => s.absorb += v,
            "elite_damage" => s.elite_damage += v,
            "block_heal" => s.block_heal += v,
            "rage_auto" => s.rage_auto += v,
            "rage_hit" => s.rage_hit += v,
            "rage_regen" => s.rage_regen += v,
            "rage_keep" => s.rage_keep += v,
            "reach" => s.reach += v,
            "pet_damage" => s.pet_damage += v,
            "pet_health" => s.pet_health += v,
            "pet_crit" => s.pet_crit += v,
            "pet_taken" => s.pet_taken += v,
            "pet_regen" => s.pet_regen += v,
            "respawn" => s.respawn += v,
            // The cards' passives, as items carry them.
            "leech" => s.leech += v,
            "execute" => s.execute += v,
            "per_wave" => s.per_wave += v,
            "thorns" => s.thorns += v,
            "lightning" => s.lightning += v,
            "bloodthirst" => s.bloodthirst += v,
            "raise_dead" => {
                s.raise_dead += v;
                s.raise_secs = s.raise_secs.max(20.0);
            }
            "soul_harvest" => s.soul_harvest += v,
            "cleave" => s.cleave += v,
            "windfury" => s.windfury += v,
            "echo" => s.echo += v,
            "split" => s.split += v,
            "radius" => s.radius += v,
            "shield" => {
                s.shield += v;
                s.shield_cooldown = if s.shield_cooldown > 0.0 { s.shield_cooldown } else { 10.0 };
            }
            "undying" => acc.undying |= v > 0.0,
            "divine" => {
                s.divine_secs = s.divine_secs.max(v);
                s.divine_cooldown = if s.divine_cooldown > 0.0 { s.divine_cooldown } else { 60.0 };
            }
            // The items' own.
            "gold_find" => s.gold_find += v,
            "hub_leech" => s.hub_leech += v,
            "defence_aura" => s.defence_aura += v,
            "rise" => s.rise += v,
            _ => {}
        }
    }
    for (k, v) in &e.team {
        let v = v * rank;
        match k.as_str() {
            "damage" => {
                s.shout += v;
                s.shout_radius = 30.0;
            }
            "haste" => {
                s.team_haste += v;
                s.team_haste_radius = 30.0;
            }
            "regen" => {
                s.team_regen += v;
                s.team_regen_radius = 30.0;
            }
            "taken" => {
                s.devotion += v;
                s.devotion_radius = 30.0;
            }
            "health" => s.team_health += v,
            _ => {}
        }
    }
    if let Some((name, v)) = &e.proc_ {
        s.procs.add(name, v * rank);
    }
}
