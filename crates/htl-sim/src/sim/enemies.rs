//! Enemies and allies: spawning, walking, choosing fights, swinging; debuffs; and the shared
//! damage and death rules.

use rand::Rng;

use super::synergy::RALLY_REACH;
use super::*;
use crate::geom::{facing, ground_path};

const LEASH: f32 = 40.0;
const THINK_SECS: f32 = 0.3;
/// A herald's war cry: how far it reaches, what it adds, and how it looks.
pub(super) const HERALD_REACH: f32 = 15.0;
const HERALD_DAMAGE: f32 = 0.25;
const HERALD_SPEED: f32 = 0.20;
const HERALD_CRY: u32 = 6673;
/// A sapper's blast at the Ancient.
const SAPPER_BLAST: u32 = 1949;
/// Bolstering stacks an enemy can take.
const BOLSTER_STACKS: u8 = 3;
/// How far a siege enemy sees defences to tear down.
const SIEGE_SIGHT: f32 = 40.0;
/// How far a healer reaches to mend another enemy.
const HEAL_REACH: f32 = 22.0;
/// How close a fight must be for a summoner to raise the dead.
const SUMMON_SIGHT: f32 = 30.0;
/// No hero this close to the Ancient: a hit on it is called out, at most every HUB_ALERT_EVERY
/// seconds.
const HUB_GUARD: f32 = 30.0;
const HUB_ALERT_EVERY: f32 = 30.0;
/// What a thorned elite's payback looks like.
const THORNS_SPELL: u32 = 467;
/// A boss calling its dead, and raging.
const SUMMON_SPELL: u32 = 8853;
const ENRAGE_SPELL: u32 = 8599;
/// How much harder an enraged boss hits.
const ENRAGE: f32 = 1.35;
/// What a splitter falling apart looks like.
const SPLIT_SPELL: u32 = 6343;

#[derive(Debug)]
pub(super) struct Dot {
    pub caster: EntityId,
    pub spell: u32,
    pub school: u8,
    pub per_tick: f32,
    pub ticks_left: u32,
    pub next: f32,
    /// Stacks (for the target frame), share of each tick that heals the caster, and how many
    /// enemies it jumps to when the victim dies.
    pub stacks: u8,
    pub leech: f32,
    pub spread: f32,
    /// Share of each tick that mends the most hurt ally near the caster.
    pub mend: f32,
}

#[derive(Debug)]
pub(super) struct Enemy {
    pub kind: EnemyKind,
    pub elite: bool,
    /// The base it guards, for guardians.
    pub guard: Option<usize>,
    pub home: Vec3,
    pub goal: Goal,
    pub mover: Option<Mover>,
    /// The victim it is currently swinging at, so AttackStart/Stop pair up.
    pub engaged: Option<EntityId>,
    pub swing: f32,
    pub think: f32,
    pub damage_mult: f32,
    /// Bolstering stacks taken (at most `BOLSTER_STACKS`).
    pub bolster: u8,
    /// Seconds left as a corpse.
    pub corpse: Option<f32>,
    /// A slot around its target, so a crowd spreads out instead of stacking.
    pub slot_angle: f32,
    pub base_speed: f32,
    /// Speed lost to slows (0.4 = 40% slower) and until when.
    pub slow: f32,
    pub slow_until: f32,
    pub root_until: f32,
    pub dots: Vec<Dot>,
    /// The speed its current path was sent with, to notice when a slow changes it.
    pub sent_speed: f32,
    /// Who hit it last: the kill's owner.
    pub last_hit_by: Option<EntityId>,
    /// The spell (as the client is shown it) that last hurt it; 0 for a swing.
    pub last_spell: u32,
    /// Abilities it casts (indices into the content's abilities), and when each is ready.
    pub abilities: Vec<usize>,
    pub ready_at: Vec<f32>,
    /// The ability it is casting, and when it lands.
    pub casting: Option<(usize, f32)>,
    /// Part of the endless stream from a base, not of the wave: the wave ends without it.
    pub stream: bool,
    /// A sapper that reached the Ancient and blew itself up: no reward for its death.
    pub detonated: bool,
    /// Extra march speed for a long road, so every road's enemies reach the bridges together.
    pub road_haste: f32,
    /// The site its road leads through: 0 straight to the Ancient, 1.. a post.
    pub front: usize,
    /// When its role acts next (a healer's mending).
    pub role_at: f32,
    /// Frozen, stunned, feared, vulnerable, marked...
    pub aff: super::spells::Afflictions,
    /// It won't pick a fight with this player again until (Feign Death).
    pub ignore: Option<(EntityId, f32)>,
    /// Its debuffs need showing again, and when to look next.
    pub debuffs_dirty: bool,
    pub debuff_check: f32,
    /// Raised or split off by this enemy.
    pub parent: Option<EntityId>,
    /// A boss's phase: 0 fresh, 1 called its dead (half health), 2 enraged (a quarter).
    pub phase: u8,
    /// An elite's twist ([`MODIFIERS`]): 0 for none.
    pub modifier: u8,
}

/// The twists an elite may carry: its name's prefix and the line under it. Index + 1 is the
/// modifier number.
pub const MODIFIERS: [(&str, &str); 4] = [
    ("Vampiric", "Heals as it hits"),
    ("Frenzied", "Attacks fast"),
    ("Swift", "Runs fast"),
    ("Thorned", "Hurts what hits it"),
];
const VAMPIRIC: u8 = 1;
/// A rifleman's shot (Shoot: the load and fire) and a cleric's heal (Flash Heal), as they look.
pub const RIFLE_SHOT: u32 = 3018;
const CLERIC_HEAL: u32 = 2061;
const FRENZIED: u8 = 2;
const SWIFT: u8 = 3;
const THORNED: u8 = 4;

#[derive(Debug)]
pub(super) struct Ally {
    pub owner: EntityId,
    pub until: f32,
    pub mover: Option<Mover>,
    pub engaged: Option<EntityId>,
    pub swing: f32,
    pub think: f32,
    pub hit: f32,
    /// A hunter's pet: it follows its master and comes back after it falls.
    pub pet: bool,
    /// Blessed by a priest's healing until then: it hits harder.
    pub blessed_until: f32,
}

/// How damage arrives, for the combat log.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Via {
    Melee,
    Spell { spell: u32, school: u8, periodic: bool },
}

impl Sim {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn spawn_enemy(
        &mut self,
        id: EntityId,
        kind: EnemyKind,
        elite: bool,
        guard: Option<usize>,
        pos: Vec3,
        facing_: f32,
        health_mult: f32,
        damage_mult: f32,
    ) {
        let def = self.enemy_def(kind).clone();
        let health = ((def.health as f32) * health_mult).round().clamp(1.0, 4.0e9) as u32;
        if guard.is_none() {
            self.run.spawned += 1;
            self.run.spawned_health += u64::from(health);
            self.run.spawned_hit += (def.damage[0] + def.damage[1]) as f32 / 2.0 * damage_mult;
        }
        let mut level = self.enemy_level();
        if elite {
            level += 1;
        }
        if kind.boss {
            level += 3;
        }
        let mut abilities: Vec<usize> = def.abilities.iter().filter_map(|k| self.content.ability_index(k)).collect();
        // Elites add the elite slam to whatever their kind already does.
        if elite {
            for a in self.content.scaling.elite_abilities.iter().filter_map(|k| self.content.ability_index(k)) {
                if !abilities.contains(&a) {
                    abilities.push(a);
                }
            }
        }
        // Abilities wake up a few seconds after it first meets a player.
        let ready_at = vec![f32::INFINITY; abilities.len()];
        let mut speed = def.speed;
        if self.has_affix("swift") {
            speed *= 1.25;
        }
        // Elites of the wave (not guardians or bosses) carry a twist.
        let modifier = if elite && !kind.boss && guard.is_none() && self.content.scaling.elite_modifiers {
            self.rng.gen_range(1..=MODIFIERS.len() as u8)
        } else {
            0
        };
        if modifier == SWIFT {
            speed *= 1.35;
        }
        self.entities.insert(
            id,
            Entity {
                pos,
                facing: facing_,
                health,
                max_health: health,
                level,
                body: Body::Enemy(Enemy {
                    kind,
                    elite,
                    guard,
                    home: pos,
                    goal: if guard.is_some() { Goal::Guard } else { Goal::Lane },
                    mover: None,
                    engaged: None,
                    swing: 0.0,
                    think: 0.0,
                    damage_mult,
                    bolster: 0,
                    corpse: None,
                    slot_angle: slot_angle(id),
                    base_speed: speed,
                    slow: 0.0,
                    slow_until: 0.0,
                    root_until: 0.0,
                    dots: Vec::new(),
                    sent_speed: speed,
                    last_hit_by: None,
                    last_spell: 0,
                    abilities,
                    ready_at,
                    casting: None,
                    stream: false,
                    detonated: false,
                    road_haste: 1.0,
                    front: 0,
                    role_at: 0.0,
                    aff: Default::default(),
                    ignore: None,
                    debuffs_dirty: false,
                    debuff_check: f32::INFINITY,
                    parent: None,
                    phase: 0,
                    modifier,
                }),
            },
        );
        self.events.push(Event::SpawnEnemy {
            id,
            kind,
            elite,
            level,
            pos,
            facing: facing_,
            health,
            max_health: health,
        });
    }

    pub(super) fn has_affix(&self, key: &str) -> bool {
        self.run.affixes.iter().any(|&i| self.content.affixes[i].key == key)
    }

    /// How fast the enemy moves now, slows included; 0 while rooted.
    pub(super) fn enemy_speed(&self, id: EntityId) -> f32 {
        let Some(en) = self.entities.get(&id).and_then(|e| e.enemy()) else {
            return 0.0;
        };
        if en.root_until > self.clock || en.aff.frozen_until > self.clock || en.aff.stun_until > self.clock {
            return 0.0;
        }
        let mut s = en.base_speed;
        if en.slow_until > self.clock {
            s *= 1.0 - en.slow.min(0.9);
        }
        if en.guard.is_none() && self.boon("the Lumber Mill") {
            s *= 0.85;
        }
        // Hurrying down the lane; at the Ancient's doorstep it slows to its own pace to fight.
        let far = self.entities.get(&id).is_some_and(|e| dist2d(e.pos, self.arena.hub) > self.content.scaling.march_until);
        let marching = en.guard.is_none() && far && matches!(en.goal, Goal::Lane | Goal::Hub);
        if marching && !en.stream {
            let sc = &self.content.scaling;
            let w = self.run.wave.saturating_sub(1) as f32;
            s *= (sc.march_speed + sc.march_speed_per_wave * w).min(sc.march_speed_cap.max(sc.march_speed));
        }
        if marching {
            s *= en.road_haste;
        }
        if let Some(pos) = self.entities.get(&id).map(|e| e.pos) {
            if self.heralded(pos) {
                s *= 1.0 + HERALD_SPEED;
            }
        }
        s.max(0.5)
    }

    pub(super) fn set_path(&mut self, id: EntityId, mut path: Vec<Vec3>, speed: f32) {
        let Some(e) = self.entities.get_mut(&id) else {
            return;
        };
        if speed <= 0.0 {
            return;
        }
        path[0] = e.pos;
        path.dedup_by(|a, b| dist2d(*a, *b) < 0.05);
        if path.len() < 2 {
            return;
        }
        match &mut e.body {
            Body::Enemy(en) => {
                en.mover = Some(Mover::new(path.clone(), speed));
                en.sent_speed = speed;
            }
            Body::Ally(a) => a.mover = Some(Mover::new(path.clone(), speed)),
            _ => return,
        }
        self.events.push(Event::MoveTo { id, path, speed });
    }

    pub(super) fn halt_enemy(&mut self, id: EntityId) {
        let Some(e) = self.entities.get_mut(&id) else {
            return;
        };
        let (mover, engaged) = match &mut e.body {
            Body::Enemy(en) => (&mut en.mover, &mut en.engaged),
            Body::Ally(a) => (&mut a.mover, &mut a.engaged),
            _ => return,
        };
        if mover.take().is_some_and(|m| !m.done()) {
            self.events.push(Event::Stop { id, pos: e.pos, facing: e.facing });
        }
        if let Some(victim) = engaged.take() {
            self.events.push(Event::AttackStop { attacker: id, victim });
        }
    }

    pub(super) fn step_enemies(&mut self, dt: f32) {
        let fighting = self.combat_on();
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| e.is_enemy())
            .map(|(&id, _)| id)
            .collect();
        for id in ids {
            // Corpses wait, then go.
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                if let Some(c) = en.corpse.as_mut() {
                    *c -= dt;
                    if *c <= 0.0 {
                        self.remove_corpse(id);
                    }
                    continue;
                }
            }
            // Walk, at the speed slows allow.
            let speed = self.enemy_speed(id);
            let mut repath = None;
            if let Some(e) = self.entities.get_mut(&id) {
                if let Body::Enemy(en) = &mut e.body {
                    if speed <= 0.0 {
                        if en.mover.as_ref().is_some_and(|m| !m.done()) {
                            en.mover = None;
                            self.events.push(Event::Stop { id, pos: e.pos, facing: e.facing });
                        }
                    } else if let Some(m) = en.mover.as_mut() {
                        if (speed - en.sent_speed).abs() > 0.05 && !m.done() {
                            m.speed = speed;
                            en.sent_speed = speed;
                            repath = Some(m.rest());
                        }
                        let (pos, f) = m.advance(dt);
                        e.pos = pos;
                        e.facing = f;
                    }
                }
            }
            if let Some(path) = repath {
                if path.len() >= 2 {
                    self.events.push(Event::MoveTo { id, path, speed });
                }
            }
            self.tick_dots(id);
            if fighting {
                self.think_enemy(id, dt);
            }
        }
    }

    fn tick_dots(&mut self, id: EntityId) {
        let now = self.clock;
        let due: Vec<(EntityId, u32, u8, u32, f32, f32)> = {
            let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) else {
                return;
            };
            let mut due = Vec::new();
            let before = en.dots.len();
            for d in &mut en.dots {
                if d.ticks_left > 0 && d.next <= now {
                    d.ticks_left -= 1;
                    d.next += 1.0;
                    due.push((d.caster, d.spell, d.school, d.per_tick.round().max(1.0) as u32, d.leech, d.mend));
                }
            }
            en.dots.retain(|d| d.ticks_left > 0);
            if en.dots.len() != before {
                en.debuffs_dirty = true;
            }
            due
        };
        for (caster, spell, school, dmg, leech, mend) in due {
            if !self.entities.get(&id).is_some_and(|e| e.alive()) {
                break;
            }
            // Vulnerabilities count for damage over time too.
            let (m, _) = self.against(caster, id, school, &crate::kit::Nums::default(), false);
            let dmg = (dmg as f32 * m).round().max(1.0) as u32;
            self.deal(caster, id, dmg, false, Via::Spell { spell, school, periodic: true });
            let alive_caster = self.entities.get(&caster).is_some_and(|c| c.alive() && c.is_player());
            if alive_caster {
                if leech > 0.0 {
                    self.heal(caster, caster, spell, (dmg as f32 * leech).round() as u32, false);
                }
                if mend > 0.0 {
                    let pos = self.entities[&caster].pos;
                    if let Some(a) = self.most_hurt(caster, pos, super::spells::ALLY_RANGE, 1.0) {
                        self.heal(caster, a, 15290, (dmg as f32 * mend).round().max(1.0) as u32, false);
                    }
                }
                let ve = self.entities[&caster].player().map_or(0.0, |p| p.stats.procs.vampiric_embrace);
                if school == 5 && ve > 0.0 {
                    let pos = self.entities[&caster].pos;
                    let amount = (dmg as f32 * ve).round() as u32;
                    if amount > 0 {
                        for a in self.allies_near(pos, 30.0) {
                            if self.entities[&a].health < self.entities[&a].max_health {
                                self.heal(caster, a, 15290, amount, false);
                            }
                        }
                    }
                }
            }
        }
    }

    /// Put a damage-over-time effect on an enemy; the same spell from the same caster refreshes
    /// (or, with `stack`, adds up to three times over).
    pub(super) fn add_dot(&mut self, caster: EntityId, victim: EntityId, spell: u32, school: u8, total: f32, secs: f32, stack: bool) {
        self.add_dot_full(caster, victim, spell, school, total, secs, stack, 0.0, 0.0, 0.0);
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn add_dot_full(
        &mut self,
        caster: EntityId,
        victim: EntityId,
        spell: u32,
        school: u8,
        total: f32,
        secs: f32,
        stack: bool,
        leech: f32,
        spread: f32,
        mend: f32,
    ) {
        let now = self.clock;
        let Some(en) = self.entities.get_mut(&victim).and_then(|e| e.enemy_mut()) else {
            return;
        };
        if total <= 0.0 {
            return;
        }
        let ticks = secs.round().max(1.0) as u32;
        let per_tick = total / ticks as f32;
        if let Some(d) = en.dots.iter_mut().find(|d| d.spell == spell && d.caster == caster) {
            if stack {
                let one = per_tick;
                d.stacks = (d.stacks + 1).min(3);
                d.per_tick = (d.per_tick + per_tick).min(one * 3.0).max(per_tick);
            } else {
                d.per_tick = per_tick.max(d.per_tick);
            }
            d.ticks_left = ticks;
            d.leech = d.leech.max(leech);
            d.spread = d.spread.max(spread);
            d.mend = d.mend.max(mend);
        } else {
            en.dots.push(Dot { caster, spell, school, per_tick, ticks_left: ticks, next: now + 1.0, stacks: 1, leech, spread, mend });
        }
        en.debuffs_dirty = true;
    }

    pub(super) fn slow_enemy(&mut self, id: EntityId, slow: f32, secs: f32) {
        let now = self.clock;
        let frost_freeze = false;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            if en.slow_until < now || slow >= en.slow {
                en.slow = slow;
            }
            en.slow_until = en.slow_until.max(now + secs);
        }
        let _ = frost_freeze;
    }

    pub(super) fn root_enemy(&mut self, id: EntityId, secs: f32) {
        let now = self.clock;
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            // Bosses shrug roots off quickly.
            let secs = if en.kind.boss { secs * 0.25 } else { secs };
            en.root_until = en.root_until.max(now + secs);
        }
    }

    pub(super) fn frozen(&self, id: EntityId) -> bool {
        self.enemy_frozen(id)
    }

    fn think_enemy(&mut self, id: EntityId, dt: f32) {
        let Some(e) = self.entities.get(&id) else {
            return;
        };
        if e.enemy().is_none() || !e.alive() {
            return;
        }
        // Frozen, stunned or asleep: nothing at all. Feared: it runs.
        if self.enemy_frozen(id) {
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                en.swing = (en.swing - dt).max(0.0);
            }
            return;
        }
        if self.enemy_feared(id) {
            self.flee(id);
            return;
        }
        if self.enemy_ability(id) {
            return;
        }
        self.enemy_role(id);
        let Some(e) = self.entities.get(&id) else {
            return;
        };
        let Some(en) = e.enemy() else {
            return;
        };
        let def = self.enemy_def(en.kind).clone();
        let pos = e.pos;
        let mut goal = en.goal;
        let guard = en.guard;
        let home = en.home;
        let leash = if guard.is_some() { self.content.bases.leash } else { LEASH };

        // Lose interest in a dead, missing or far-off player; a guardian also gives up when led
        // too far from home.
        if let Goal::Player(pid) | Goal::Attack(pid) = goal {
            let keep = self.entities.get(&pid).is_some_and(|p| p.alive() && dist2d(p.pos, pos) <= leash)
                && (guard.is_none() || dist2d(pos, home) <= leash);
            if !keep {
                goal = if guard.is_some() { Goal::Guard } else { Goal::Hub };
            }
        }
        // Pick a fight with the nearest player in reach of its senses.
        let ignore = en.ignore.filter(|(_, until)| *until > self.clock).map(|(who, _)| who);
        if !matches!(goal, Goal::Player(_)) {
            let aggro = if guard.is_some() { def.aggro_radius.max(18.0) } else { def.aggro_radius };
            if let Some(pid) = self.nearest_player(pos, aggro).filter(|p| Some(*p) != ignore) {
                goal = Goal::Player(pid);
            }
        }
        // Taunted: it has to fight the one who taunted it. Defiance: a protector close by pulls
        // it off the Ancient and its allies.
        let taunt = en.aff.taunt.filter(|(who, until)| *until > self.clock && self.entities.get(who).is_some_and(|w| w.alive()));
        if let Some((who, _)) = taunt {
            goal = Goal::Player(who);
        } else if guard.is_none() {
            if let Some(w) = self.defiant_near(pos) {
                if goal != Goal::Player(w) {
                    goal = Goal::Player(w);
                }
            }
        }
        // Siege goes for the defences it can see, whoever is in the way.
        if guard.is_none() && taunt.is_none() && def.role == "siege" {
            if let Some(did) = self.nearest_defense(pos, SIEGE_SIGHT) {
                goal = Goal::Attack(did);
            }
        }
        // On the march, it turns on a defence (or an ally) it passes close by; a flanker runs on.
        // A defence only holds so many at once: the rest march past it.
        if guard.is_none() && def.role != "flanker" && def.role != "sapper" && matches!(goal, Goal::Lane | Goal::Hub) {
            if let Some(did) = self.open_defense(id, pos, self.content.defenses.aggro) {
                goal = Goal::Attack(did);
            }
        }

        let chasing = matches!(goal, Goal::Player(_) | Goal::Attack(_));
        let (victim, victim_pos, victim_radius) = match goal {
            Goal::Player(pid) | Goal::Attack(pid) => (Some(pid), self.entities[&pid].pos, UNIT_RADIUS + if matches!(goal, Goal::Attack(_)) { 1.0 } else { 0.0 }),
            Goal::Hub | Goal::Lane => (
                Some(self.hub),
                self.arena.hub,
                self.content.hub.reach - MELEE_RANGE + UNIT_RADIUS,
            ),
            Goal::Guard => (None, home, 0.0),
        };
        // Once swinging at someone, a step back doesn't break it.
        let engaged_now = self.entities[&id].enemy().is_some_and(|en| en.engaged.is_some() && en.engaged == victim);
        let reach = if def.range > 0.0 {
            def.range + victim_radius
        } else {
            MELEE_RANGE + victim_radius + if engaged_now { 2.0 } else { 0.0 }
        };
        let in_reach = victim.is_some() && dist2d(pos, victim_pos) <= reach;
        // The next swing gets ready on the way, so it lands as soon as it's in reach.
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            if !in_reach {
                en.swing = (en.swing + dt).min(def.attack_secs);
            }
        }
        let at_hub_edge = matches!(goal, Goal::Lane | Goal::Hub) && in_reach;

        let think_now = {
            let en = self.entities.get_mut(&id).unwrap().enemy_mut().unwrap();
            en.goal = goal;
            en.think -= dt;
            if en.think <= 0.0 {
                en.think = THINK_SECS;
                true
            } else {
                false
            }
        };

        if at_hub_edge && def.role == "sapper" {
            self.detonate(id);
            return;
        }
        if (chasing && in_reach) || at_hub_edge {
            // Close enough: stand and swing.
            let face = facing(pos, victim_pos);
            let moving = self.entities[&id].enemy().unwrap().mover.as_ref().is_some_and(|m| !m.done());
            if moving && (chasing || dist2d(pos, victim_pos) <= reach - 1.0) {
                let e = self.entities.get_mut(&id).unwrap();
                if let Body::Enemy(en) = &mut e.body {
                    en.mover = None;
                }
                e.facing = face;
                self.events.push(Event::Stop { id, pos, facing: face });
            }
            if def.range > 0.0 {
                // An archer shoots from where it stands: no melee swing to show.
                if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                    en.engaged = victim;
                }
                if let Some(e) = self.entities.get_mut(&id) {
                    e.facing = face;
                }
            } else {
                self.engage(id, victim.expect("a victim"));
            }
            self.swing_enemy(id, dt, &def);
            return;
        }

        // Not in reach: keep (or start) walking.
        self.disengage(id);
        if !think_now {
            return;
        }
        let speed = self.enemy_speed(id);
        if speed <= 0.0 {
            return;
        }
        let (current_end, idle) = {
            let en = self.entities[&id].enemy().unwrap();
            match &en.mover {
                Some(m) if !m.done() => (Some(m.end()), false),
                _ => (None, true),
            }
        };
        match goal {
            Goal::Player(_) | Goal::Attack(_) => {
                let slot = self.slot_near(id, victim_pos, 2.5);
                let stale = current_end.map_or(true, |end| dist2d(end, slot) > 2.0);
                if stale {
                    let path = ground_path(&*self.ground, pos, slot, PATH_STEP);
                    self.set_path(id, path, speed);
                }
            }
            Goal::Lane => {
                if idle {
                    // Pushed off the lane end (or never had a path): head for the hub.
                    self.entities.get_mut(&id).unwrap().enemy_mut().unwrap().goal = Goal::Hub;
                }
            }
            Goal::Hub => {
                let a = self.entities[&id].enemy().unwrap().slot_angle;
                let target = self.hub_slot(pos, a);
                let stale = current_end.map_or(true, |end| dist2d(end, target) > 2.0);
                if stale {
                    let path = ground_path(&*self.ground, pos, target, PATH_STEP);
                    self.set_path(id, path, speed);
                }
            }
            Goal::Guard => {
                // Walk home and heal up there.
                if dist2d(pos, home) > 3.0 && current_end.map_or(true, |end| dist2d(end, home) > 2.0) {
                    let path = ground_path(&*self.ground, pos, home, PATH_STEP);
                    self.set_path(id, path, speed);
                } else if dist2d(pos, home) <= 3.0 {
                    let e = &self.entities[&id];
                    if e.health < e.max_health {
                        let m = e.max_health;
                        self.set_health(id, m);
                    }
                }
            }
        }
    }

    /// A spot `r` yards from `center`, at this enemy's own angle.
    fn slot_near(&self, id: EntityId, center: Vec3, r: f32) -> Vec3 {
        let a = self.entities[&id].enemy().map_or(0.0, |en| en.slot_angle);
        self.ring(center, r, a)
    }

    fn engage(&mut self, id: EntityId, victim: EntityId) {
        let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) else {
            return;
        };
        if en.engaged == Some(victim) {
            return;
        }
        if let Some(old) = en.engaged.replace(victim) {
            self.events.push(Event::AttackStop { attacker: id, victim: old });
        }
        // The swing timer keeps running while it chases: stepping out of reach and back in
        // doesn't buy time.
        self.events.push(Event::AttackStart { attacker: id, victim });
    }

    fn disengage(&mut self, id: EntityId) {
        let ranged = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| self.enemy_def(en.kind).range > 0.0);
        let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) else {
            return;
        };
        if let Some(victim) = en.engaged.take() {
            if !ranged {
                self.events.push(Event::AttackStop { attacker: id, victim });
            }
        }
    }

    /// A summoner raises the dead while it fights.
    fn enemy_summon(&mut self, id: EntityId) {
        let Some((kind, pos, at)) = self.entities.get(&id).and_then(|e| e.enemy().map(|en| (en.kind, e.pos, en.role_at))) else { return };
        let d = self.enemy_def(kind).clone();
        if self.clock < at {
            return;
        }
        let Some(index) = self.content.enemies.iter().position(|e| e.key == d.summon) else { return };
        // Only in a fight: a player or a defence close by.
        let fighting = self.nearest_player(pos, SUMMON_SIGHT).is_some() || self.nearest_defense(pos, SUMMON_SIGHT).is_some();
        let mine = self.entities.values().filter(|e| e.alive() && e.enemy().is_some_and(|en| en.parent == Some(id))).count() as u32;
        let next = if fighting && mine < d.summon_cap.max(1) {
            let n = d.summon_count.max(1).min(d.summon_cap.max(1) - mine);
            self.events.push(Event::SpellGo { caster: id, spell: d.summon_spell, target: SpellTarget::Point(pos), hits: vec![] });
            for k in 0..n {
                let a = k as f32 / n as f32 * std::f32::consts::TAU + self.rng.gen_range(0.0..1.0);
                let at = self.ring(pos, 3.0, a);
                self.spawn_add(id, index, at);
            }
            self.clock + d.summon_secs.max(2.0)
        } else {
            self.clock + 1.0
        };
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.role_at = next;
        }
    }

    /// One of `parent`'s: raised or split off beside it, at a share of a wave enemy's health,
    /// heading for the Ancient.
    pub(super) fn spawn_add(&mut self, parent: EntityId, index: usize, pos: Vec3) -> Option<EntityId> {
        let (front, stream, damage_mult, share) = {
            let e = self.entities.get(&parent)?;
            let en = e.enemy()?;
            (en.front, en.stream, en.damage_mult, self.enemy_def(en.kind).add_health)
        };
        let id = self.alloc();
        let pos = [pos[0], pos[1], self.ground.height(pos[0], pos[1])];
        let health_mult = self.health_mult() * share;
        self.spawn_enemy(id, EnemyKind { boss: false, index }, false, None, pos, 0.0, health_mult, damage_mult);
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.parent = Some(parent);
            en.front = front;
            en.stream = stream;
            en.goal = Goal::Hub;
        }
        Some(id)
    }

    /// A boss turns the fight at half health (it calls its dead) and at a quarter (it rages).
    fn boss_phase(&mut self, id: EntityId, health: u32, max: u32) {
        let Some((kind, phase, pos)) = self.entities.get(&id).and_then(|e| e.enemy().map(|en| (en.kind, en.phase, e.pos))) else { return };
        if !kind.boss {
            return;
        }
        let pct = health as f32 / max.max(1) as f32;
        let def = self.enemy_def(kind).clone();
        if phase == 0 && pct <= 0.5 {
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                en.phase = 1;
            }
            if let Some(index) = self.content.enemies.iter().position(|e| e.key == def.phase_adds) {
                // Fewer for fewer heroes: half the call for one, all of it for four.
                let players = self.player_count().max(1) as f32;
                let n = ((def.phase_add_count.max(1) as f32 * (players + 1.0) / 5.0).ceil() as u32).max(1);
                let what = &self.content.enemies[index].name;
                self.announce(format!("{} calls its dead: {n} {what}{} join the fight!", def.name, if n == 1 { "" } else { "s" }), true);
                self.events.push(Event::SpellGo { caster: id, spell: SUMMON_SPELL, target: SpellTarget::Point(pos), hits: vec![] });
                for k in 0..n {
                    let a = k as f32 / n as f32 * std::f32::consts::TAU;
                    let at = self.ring(pos, 6.0, a);
                    self.spawn_add(id, index, at);
                }
            }
        } else if phase == 1 && pct <= 0.25 {
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                en.phase = 2;
                en.damage_mult *= ENRAGE;
            }
            self.announce(format!("{} is enraged: it hits much harder now!", def.name), true);
            self.events.push(Event::SpellGo { caster: id, spell: ENRAGE_SPELL, target: SpellTarget::Unit(id), hits: vec![id] });
        }
    }

    /// Whether a living herald stands within its cry's reach of `pos`: enemies there hit harder
    /// and move faster.
    pub(super) fn heralded(&self, pos: Vec3) -> bool {
        self.entities.values().any(|e| {
            e.alive() && e.enemy().is_some_and(|en| self.enemy_def(en.kind).role == "herald") && dist2d(e.pos, pos) <= HERALD_REACH
        })
    }

    /// A sapper at the Ancient: it blows itself up, tearing a share off the Ancient's health.
    fn detonate(&mut self, id: EntityId) {
        let Some((kind, pos)) = self.entities.get(&id).and_then(|e| e.enemy().map(|en| (en.kind, e.pos))) else { return };
        let def = self.enemy_def(kind).clone();
        let hub = self.hub;
        let blast = (self.entities[&hub].max_health as f32 * def.blast).round() as u32;
        self.events.push(Event::SpellGo { caster: id, spell: SAPPER_BLAST, target: SpellTarget::Point(pos), hits: vec![hub] });
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.detonated = true;
        }
        self.announce(format!("A {} blew up at the {}!", def.name, self.content.hub.name), false);
        self.deal(id, hub, blast, true, Via::Spell { spell: SAPPER_BLAST, school: 2, periodic: false });
        // Heroes in the blast are burnt too.
        let near: Vec<EntityId> = self.entities.iter().filter(|(_, e)| e.is_player() && e.alive() && dist2d(e.pos, pos) <= 6.0).map(|(&p, _)| p).collect();
        for p in near {
            let dmg = self.entities[&p].max_health / 6;
            self.deal(id, p, dmg, false, Via::Spell { spell: SAPPER_BLAST, school: 2, periodic: false });
        }
        if let Some(e) = self.entities.get_mut(&id) {
            e.health = 0;
            let (health, max_health) = (0, e.max_health);
            self.events.push(Event::Health { id, health, max_health });
        }
        self.kill(id, id);
    }

    /// A healer mends the most hurt enemy near it now and then; a herald cries out.
    fn enemy_role(&mut self, id: EntityId) {
        let Some((kind, pos, at)) = self.entities.get(&id).and_then(|e| e.enemy().map(|en| (en.kind, e.pos, en.role_at))) else { return };
        if !self.enemy_def(kind).summon.is_empty() {
            self.enemy_summon(id);
            return;
        }
        if self.enemy_def(kind).role == "herald" {
            if self.clock >= at {
                self.events.push(Event::SpellGo { caster: id, spell: HERALD_CRY, target: SpellTarget::Unit(id), hits: vec![] });
                if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                    en.role_at = self.clock + 6.0;
                }
            }
            return;
        }
        // A haunt pulses death around itself every second: don't let it catch you.
        let (pulse, pulse_radius, pulse_spell, school, dmg) = {
            let d = self.enemy_def(kind);
            (d.pulse, d.pulse_radius, d.pulse_spell, 5u8, (d.damage[0] + d.damage[1]) as f32 / 2.0)
        };
        if pulse > 0.0 {
            if self.clock >= at {
                let mult = self.entities.get(&id).and_then(|e| e.enemy()).map_or(1.0, |en| en.damage_mult);
                self.events.push(Event::SpellGo { caster: id, spell: pulse_spell, target: SpellTarget::Point(pos), hits: vec![] });
                self.hazard_tick(id, pos, pulse_radius, pulse_spell, dmg * mult * pulse, school, 0.0, &[], 0.0);
                if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                    en.role_at = self.clock + 1.0;
                }
            }
            return;
        }
        let (heal, secs, spell) = {
            let d = self.enemy_def(kind);
            (d.heal, d.heal_secs, d.heal_spell)
        };
        if heal <= 0.0 || self.clock < at {
            return;
        }
        let target = self
            .entities
            .iter()
            .filter(|(_, e)| e.alive() && e.enemy().is_some() && e.health * 100 < e.max_health * 85 && dist2d(e.pos, pos) <= HEAL_REACH)
            // Mortal Wound: it can't be healed.
            .filter(|(_, e)| e.enemy().is_some_and(|en| en.aff.wound_until <= self.clock))
            .min_by(|a, b| {
                let ra = a.1.health as f32 / a.1.max_health.max(1) as f32;
                let rb = b.1.health as f32 / b.1.max_health.max(1) as f32;
                ra.total_cmp(&rb)
            })
            .map(|(&t, e)| (t, e.health, e.max_health));
        let next = match target {
            Some((t, h, m)) => {
                self.events.push(Event::SpellGo { caster: id, spell, target: SpellTarget::Unit(t), hits: vec![t] });
                self.set_health(t, h + (m as f32 * heal).round() as u32);
                self.clock + secs.max(1.0)
            }
            None => self.clock + 0.5,
        };
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.role_at = next;
        }
    }

    fn swing_enemy(&mut self, id: EntityId, dt: f32, def: &EnemyDef) {
        let (victim, ready, mut mult, modifier) = {
            let en = self.entities.get_mut(&id).unwrap().enemy_mut().unwrap();
            // Thunder Clapped: its attacks come slower.
            let weak = if en.aff.weaken_until > self.clock { en.aff.weaken } else { 0.0 };
            en.swing += dt * (1.0 - weak);
            let every = if en.modifier == FRENZIED { def.attack_secs * 0.6 } else { def.attack_secs };
            let ready = en.swing >= every;
            if ready {
                en.swing = (en.swing - every).min(every);
            }
            (en.engaged, ready, en.damage_mult, en.modifier)
        };
        let (Some(victim), true) = (victim, ready) else {
            return;
        };
        if self.has_affix("raging") {
            let e = &self.entities[&id];
            if e.health * 10 < e.max_health * 3 {
                mult *= 1.5;
            }
        }
        if def.role != "herald" && self.heralded(self.entities[&id].pos) {
            mult *= 1.0 + HERALD_DAMAGE;
        }
        let mut damage = self.rng.gen_range(def.damage[0]..=def.damage[1]) as f32 * mult;
        if let Some(en) = self.entities.get(&id).and_then(|e| e.enemy()) {
            if en.aff.demoralize_until > self.clock {
                damage *= 1.0 - en.aff.demoralize.min(0.8);
            }
        }
        if victim == self.hub {
            damage *= def.hub_damage;
        }
        if self.entities.get(&victim).is_some_and(|v| v.is_player()) {
            // Bosses already hit hard, and their real threat is what they cast: half the boost.
            let boss = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.kind.boss);
            let sc = &self.content.scaling;
            let more = sc.hero_damage - 1.0 + sc.hero_damage_per_wave * self.run.wave.saturating_sub(1) as f32;
            damage *= 1.0 + if boss { more * 0.5 } else { more };
        }
        // Defences wear down under attack (their health keeps pace with the enemies', so their
        // blows must too); siege tears through them.
        if self.is_defense(victim) {
            // (Not a boss's: one already flattens a defence it reaches.)
            let boss = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| en.kind.boss);
            if !boss {
                damage *= self.content.scaling.defence_damage;
            }
            if def.siege > 0.0 {
                damage *= def.siege;
            }
        }
        if def.range > 0.0 && def.shot_spell > 0 {
            self.events.push(Event::SpellGo { caster: id, spell: def.shot_spell, target: SpellTarget::Unit(victim), hits: vec![victim] });
        }
        let crit = victim != self.hub && self.rng.gen::<f32>() < 0.05;
        if crit {
            damage *= 1.5;
        }
        let before = self.entities.get(&victim).map_or(0, |e| e.health);
        self.deal(id, victim, damage.round() as u32, crit, Via::Melee);
        // A vampire drinks half of what it took.
        if modifier == VAMPIRIC {
            let taken = before.saturating_sub(self.entities.get(&victim).map_or(0, |e| e.health));
            let (h, m) = self.entities.get(&id).map_or((0, 0), |e| (e.health, e.max_health));
            if taken > 0 && h > 0 && h < m {
                self.set_health(id, (h + taken / 2).min(m));
            }
        }
    }

    /// An elite's twist, 0 for none (see [`MODIFIERS`]).
    pub fn enemy_modifier(&self, id: EntityId) -> u8 {
        self.entities.get(&id).and_then(|e| e.enemy()).map_or(0, |en| en.modifier)
    }

    /// A warrior with Defiance close enough to pull this enemy over.
    fn defiant_near(&self, pos: Vec3) -> Option<EntityId> {
        self.order
            .iter()
            .filter_map(|id| self.entities.get(id).map(|e| (*id, e)))
            .filter(|(_, e)| e.alive())
            .filter_map(|(id, e)| e.player().map(|p| (id, e.pos, p.stats.reach)))
            .filter(|(_, p, r)| *r > 0.0 && dist2d(*p, pos) <= *r)
            .min_by(|a, b| dist2d(a.1, pos).total_cmp(&dist2d(b.1, pos)))
            .map(|(id, _, _)| id)
    }

    /// A feared enemy runs from what scared it.
    fn flee(&mut self, id: EntityId) {
        let (pos, from, think) = {
            let e = &self.entities[&id];
            let en = e.enemy().unwrap();
            (e.pos, en.aff.fear_from, en.think)
        };
        self.disengage(id);
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.think = think - 0.05;
            if en.think > 0.0 && en.mover.as_ref().is_some_and(|m| !m.done()) {
                return;
            }
            en.think = 1.0;
        }
        let d = dist2d(pos, from).max(0.5);
        let jitter = self.rng.gen_range(-0.6..0.6f32);
        let a = (pos[1] - from[1]).atan2(pos[0] - from[0]) + jitter;
        let _ = d;
        let to = self.ring(pos, 10.0, a);
        let path = ground_path(&*self.ground, pos, to, PATH_STEP);
        let speed = self.enemy_def(self.entities[&id].enemy().unwrap().kind).speed;
        self.set_path(id, path, speed);
    }

    /// The nearest standing defence or ally within `radius`.
    /// A defence: a treant, a spring, or an ally fighting for the Ancient.
    /// Which of the party meter's defence lines a hit counts on: 0 for the base (its treants and
    /// the Ancient itself), 1 for the soldiers.
    pub(super) fn defence_side(&self, id: EntityId) -> Option<usize> {
        if id == self.hub {
            return Some(0);
        }
        match self.entities.get(&id).map(|e| &e.body) {
            Some(Body::Npc { npc: NpcKind::Protector, .. }) => Some(0),
            Some(Body::Ally(a)) if a.owner == self.hub => Some(1),
            _ => None,
        }
    }

    pub(super) fn is_defense(&self, id: EntityId) -> bool {
        self.entities.get(&id).is_some_and(|e| match &e.body {
            Body::Npc { npc: NpcKind::Protector | NpcKind::Spring, .. } => true,
            Body::Ally(a) => a.owner == self.hub,
            _ => false,
        })
    }

    fn nearest_defense(&self, pos: Vec3, radius: f32) -> Option<EntityId> {
        self.entities
            .iter()
            .filter(|(_, e)| e.alive() && matches!(e.body, Body::Ally(_) | Body::Npc { npc: NpcKind::Protector | NpcKind::Spring, .. }))
            .map(|(&id, e)| (id, dist2d(e.pos, pos)))
            .filter(|(_, d)| *d <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    /// The nearest defence (or ally) within `radius` that isn't already holding as many enemies
    /// as it can (`defenses.hold`), for enemy `me` on the march.
    fn open_defense(&self, me: EntityId, pos: Vec3, radius: f32) -> Option<EntityId> {
        let mut near: Vec<(EntityId, f32)> = self
            .entities
            .iter()
            .filter(|(_, e)| e.alive() && matches!(e.body, Body::Ally(_) | Body::Npc { npc: NpcKind::Protector | NpcKind::Spring, .. }))
            .map(|(&id, e)| (id, dist2d(e.pos, pos)))
            .filter(|(_, d)| *d <= radius)
            .collect();
        if near.is_empty() {
            return None;
        }
        near.sort_by(|a, b| a.1.total_cmp(&b.1));
        near.into_iter()
            .find(|(did, _)| {
                let hold = self.holds(*did);
                if hold == 0 {
                    return true;
                }
                let held = self
                    .entities
                    .iter()
                    .filter(|(&eid, e)| eid != me && e.alive() && e.enemy().is_some_and(|en| en.goal == Goal::Attack(*did)))
                    .count();
                held < hold as usize
            })
            .map(|n| n.0)
    }

    pub(super) fn nearest_player(&self, pos: Vec3, radius: f32) -> Option<EntityId> {
        self.entities
            .iter()
            .filter(|(_, e)| e.is_player() && e.alive())
            .map(|(&id, e)| (id, dist2d(e.pos, pos)))
            .filter(|(_, d)| *d <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    // --- allies -------------------------------------------------------------------------------

    pub(super) fn spawn_ally(&mut self, owner: EntityId, at: Vec3, secs: f32) {
        let Some(o) = self.entities.get(&owner) else { return };
        let level = o.level;
        let hit = self.player_hit(owner) * 0.4;
        let health = (o.max_health / 2).max(1);
        let alive_allies = self
            .entities
            .values()
            .filter(|e| matches!(&e.body, Body::Ally(a) if a.owner == owner))
            .count();
        if alive_allies >= 6 {
            return;
        }
        let id = self.alloc();
        let f = self.rng.gen::<f32>() * std::f32::consts::TAU;
        self.entities.insert(
            id,
            Entity {
                pos: at,
                facing: f,
                health,
                max_health: health,
                level,
                body: Body::Ally(Ally { owner, until: self.clock + secs, mover: None, engaged: None, swing: 0.0, think: 0.0, hit, pet: false, blessed_until: 0.0 }),
            },
        );
        self.events.push(Event::SpawnAlly { id, owner, level, pos: at, facing: f, health, pet: false });
    }

    pub(super) fn step_allies(&mut self, dt: f32) {
        let ids: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.body, Body::Ally(_)))
            .map(|(&id, _)| id)
            .collect();
        let fighting = self.combat_on();
        for id in ids {
            let (until, owner, pet) = match &self.entities[&id].body {
                Body::Ally(a) => (a.until, a.owner, a.pet),
                _ => continue,
            };
            if pet {
                if !self.entities.contains_key(&owner) {
                    self.halt_enemy(id);
                    self.entities.remove(&id);
                    self.events.push(Event::Despawn { id });
                } else {
                    self.step_pet(id, owner, dt, fighting);
                }
                continue;
            }
            if until <= self.clock || !self.entities.contains_key(&owner) || !fighting {
                self.halt_enemy(id);
                self.entities.remove(&id);
                self.events.push(Event::Despawn { id });
                continue;
            }
            // Walk.
            if let Some(e) = self.entities.get_mut(&id) {
                if let Body::Ally(a) = &mut e.body {
                    if let Some(m) = a.mover.as_mut() {
                        let (pos, f) = m.advance(dt);
                        e.pos = pos;
                        e.facing = f;
                    }
                }
            }
            let pos = self.entities[&id].pos;
            // A defender at a post guards the post; the rest stay by their owner.
            let owner_pos = self.defense_home(id).unwrap_or(self.entities[&owner].pos);
            // Riflemen shoot and clerics heal from where they stand.
            if owner == self.hub && self.step_backline(id, pos, owner_pos, dt) {
                continue;
            }
            // The nearest enemy near its master (a defender turns on a marked or rallied-to one
            // first).
            let focus = if owner == self.hub { self.defence_focus(pos, RALLY_REACH) } else { None };
            let target = focus
                .filter(|(t, _)| dist2d(self.entities[t].pos, owner_pos) < 50.0)
                .or_else(|| self.enemies_near(pos, 25.0).into_iter().find(|(t, _)| dist2d(self.entities[t].pos, owner_pos) < 40.0))
                .map(|(t, _)| t);
            let (think, engaged, hit) = {
                let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body else { continue };
                a.think -= dt;
                let think = a.think <= 0.0;
                if think {
                    a.think = THINK_SECS;
                }
                (think, a.engaged, a.hit)
            };
            match target {
                Some(t) if dist2d(self.entities[&t].pos, pos) <= MELEE_RANGE + UNIT_RADIUS => {
                    if engaged != Some(t) {
                        if let Some(old) = engaged {
                            self.events.push(Event::AttackStop { attacker: id, victim: old });
                        }
                        self.events.push(Event::AttackStart { attacker: id, victim: t });
                        let tpos = self.entities[&t].pos;
                        let e = self.entities.get_mut(&id).unwrap();
                        if let Body::Ally(a) = &mut e.body {
                            a.engaged = Some(t);
                            a.mover = None;
                            a.swing = 0.0;
                        }
                        e.facing = facing(pos, tpos);
                        let face = e.facing;
                        self.events.push(Event::Stop { id, pos, facing: face });
                    }
                    let ready = {
                        let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body else { continue };
                        a.swing += dt;
                        if a.swing >= 2.0 {
                            a.swing -= 2.0;
                            true
                        } else {
                            false
                        }
                    };
                    if ready {
                        let dmg = (hit * self.rng.gen_range(0.85..1.15)).round().max(1.0) as u32;
                        self.deal(id, t, dmg, false, Via::Melee);
                    }
                }
                other => {
                    if let Some(old) = engaged {
                        self.events.push(Event::AttackStop { attacker: id, victim: old });
                        if let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body {
                            a.engaged = None;
                        }
                    }
                    if think {
                        let goal = other.map_or(owner_pos, |t| self.entities[&t].pos);
                        let goal = self.ring(goal, 2.0, slot_angle(id));
                        let stale = match &self.entities[&id].body {
                            Body::Ally(a) => a.mover.as_ref().map_or(true, |m| dist2d(m.end(), goal) > 2.0),
                            _ => false,
                        };
                        if stale && dist2d(pos, goal) > 2.5 {
                            let path = ground_path(&*self.ground, pos, goal, PATH_STEP);
                            self.set_path(id, path, 7.0);
                        }
                    }
                }
            }
        }
    }

    /// A rifleman or a cleric of the Ancient's: shoots the nearest enemy in reach (or heals the
    /// most hurt hero or soldier near them) and otherwise keeps to their place. False for any
    /// other ally.
    fn step_backline(&mut self, id: EntityId, pos: Vec3, home: Vec3, dt: f32) -> bool {
        let kind = self.soldier_kind(id);
        let rifle = kind == Some(super::defenses::SoldierKind::Rifleman);
        let cleric = kind == Some(super::defenses::SoldierKind::Cleric);
        if !rifle && !cleric {
            return false;
        }
        let d = self.content.defenses.clone();
        let now = self.clock;
        let ready = self.defenses.ready.get(&id).copied().unwrap_or(0.0) <= now;
        // Walk home (after a march, or pushed off).
        let think = {
            let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body else { return true };
            a.think -= dt;
            let t = a.think <= 0.0;
            if t {
                a.think = THINK_SECS;
            }
            t
        };
        let goal = self.ring(home, 2.0, slot_angle(id));
        let far = dist2d(pos, goal) > 3.0;
        if think && far {
            let stale = match &self.entities[&id].body {
                Body::Ally(a) => a.mover.as_ref().map_or(true, |m| dist2d(m.end(), goal) > 2.0),
                _ => false,
            };
            if stale {
                let path = ground_path(&*self.ground, pos, goal, PATH_STEP);
                self.set_path(id, path, 7.0);
            }
        }
        // On the march (more than a few steps from their place): no shooting, no healing.
        if dist2d(pos, home) > 12.0 || !ready {
            return true;
        }
        if rifle {
            let target = self.defence_focus(pos, d.rifleman_range).or_else(|| self.enemies_near(pos, d.rifleman_range).into_iter().next());
            if let Some((t, dist)) = target {
                let hit = match &self.entities[&id].body {
                    Body::Ally(a) => a.hit,
                    _ => 0.0,
                };
                let tpos = self.entities[&t].pos;
                if let Some(e) = self.entities.get_mut(&id) {
                    e.facing = facing(pos, tpos);
                }
                self.defenses.ready.insert(id, now + 2.5);
                self.events.push(Event::SpellGo { caster: id, spell: RIFLE_SHOT, target: SpellTarget::Unit(t), hits: vec![t] });
                let roll = self.rng.gen_range(0.9..1.1);
                self.queue_hit(id, t, RIFLE_SHOT, hit * roll, false, 0, dist / 40.0 + 0.05, super::cards::HitFx::default());
            }
        } else {
            // The most hurt (by share) hero, soldier or defence in reach.
            let mut hurt: Vec<(EntityId, f32)> = self
                .entities
                .iter()
                .filter(|(_, e)| e.alive() && e.health < e.max_health && dist2d(e.pos, pos) <= d.cleric_range)
                .filter(|(_, e)| e.is_player() || matches!(&e.body, Body::Ally(a) if a.owner == self.hub || !a.pet) || matches!(e.body, Body::Npc { npc: NpcKind::Protector | NpcKind::Spring, .. }))
                .map(|(&eid, e)| (eid, e.health as f32 / e.max_health.max(1) as f32))
                .collect();
            hurt.sort_by(|a, b| a.1.total_cmp(&b.1));
            if let Some(&(t, _)) = hurt.first() {
                let mult = 1.0 + self.content.defenses.upgrade_bonus * self.defenses.rank as f32;
                let amount = (self.entities[&t].max_health as f32 * d.cleric_heal * mult).round().max(1.0) as u32;
                self.defenses.ready.insert(id, now + 3.0);
                self.heal(id, t, CLERIC_HEAL, amount, false);
            }
        }
        true
    }

    /// A hunter's pet: it goes for its master's target (or the nearest enemy near its master),
    /// and otherwise stays at heel.
    fn step_pet(&mut self, id: EntityId, owner: EntityId, dt: f32, fighting: bool) {
        // Walk.
        if let Some(e) = self.entities.get_mut(&id) {
            if let Body::Ally(a) = &mut e.body {
                if let Some(m) = a.mover.as_mut() {
                    let (pos, f) = m.advance(dt);
                    e.pos = pos;
                    e.facing = f;
                }
            }
        }
        let pos = self.entities[&id].pos;
        let (owner_pos, owner_alive, wish) = {
            let o = &self.entities[&owner];
            let p = o.player();
            let wish = p.and_then(|p| p.pet_target.or(p.attacking).or(p.selected));
            (o.pos, o.alive(), wish)
        };
        let enemy_ok = |t: &EntityId| self.entities.get(t).is_some_and(|e| e.is_enemy() && e.alive());
        let target = if !fighting || !owner_alive {
            None
        } else {
            wish.filter(|t| enemy_ok(t) && dist2d(self.entities[t].pos, owner_pos) < 40.0).or_else(|| {
                self.enemies_near(pos, 20.0)
                    .into_iter()
                    .find(|(t, _)| dist2d(self.entities[t].pos, owner_pos) < 30.0)
                    .map(|(t, _)| t)
            })
        };
        let (think, engaged, hit) = {
            let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body else { return };
            a.think -= dt;
            let think = a.think <= 0.0;
            if think {
                a.think = THINK_SECS;
            }
            (think, a.engaged, a.hit)
        };
        match target {
            Some(t) if dist2d(self.entities[&t].pos, pos) <= MELEE_RANGE + UNIT_RADIUS => {
                if engaged != Some(t) {
                    if let Some(old) = engaged {
                        self.events.push(Event::AttackStop { attacker: id, victim: old });
                    }
                    self.events.push(Event::AttackStart { attacker: id, victim: t });
                    let tpos = self.entities[&t].pos;
                    let e = self.entities.get_mut(&id).unwrap();
                    if let Body::Ally(a) = &mut e.body {
                        a.engaged = Some(t);
                        a.mover = None;
                    }
                    e.facing = facing(pos, tpos);
                    let face = e.facing;
                    self.events.push(Event::Stop { id, pos, facing: face });
                }
                let haste = 1.0 + self.buff_value(owner, super::spells::BuffKind::PetFrenzy);
                let ready = {
                    let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body else { return };
                    a.swing += dt * haste;
                    if a.swing >= 2.0 {
                        a.swing -= 2.0;
                        true
                    } else {
                        false
                    }
                };
                if ready {
                    let (m, crit_bonus) = self.against(owner, t, 0, &crate::kit::Nums::default(), true);
                    let crit = self.rng.gen::<f32>() < 0.05 + self.pet_crit(owner) + crit_bonus;
                    let mut dmg = hit * self.rng.gen_range(0.85..1.15) * m;
                    if crit {
                        dmg *= 2.0;
                    }
                    self.deal(id, t, dmg.round().max(1.0) as u32, crit, Via::Melee);
                    if crit {
                        self.on_crit(owner, t, dmg, 0, true);
                    }
                    // Its bite pulls the enemy onto it.
                    if let Some(en) = self.entities.get_mut(&t).and_then(|e| e.enemy_mut()) {
                        if matches!(en.goal, Goal::Lane | Goal::Hub) {
                            en.goal = Goal::Attack(id);
                        }
                    }
                }
            }
            other => {
                if let Some(old) = engaged {
                    self.events.push(Event::AttackStop { attacker: id, victim: old });
                    if let Body::Ally(a) = &mut self.entities.get_mut(&id).unwrap().body {
                        a.engaged = None;
                    }
                }
                if think {
                    let (goal, near) = match other {
                        Some(t) => (self.entities[&t].pos, 1.5),
                        None => (self.ring(owner_pos, 3.0, slot_angle(id)), 4.0),
                    };
                    let stale = match &self.entities[&id].body {
                        Body::Ally(a) => a.mover.as_ref().map_or(true, |m| dist2d(m.end(), goal) > 2.0),
                        _ => false,
                    };
                    if stale && dist2d(pos, goal) > near {
                        let path = ground_path(&*self.ground, pos, goal, PATH_STEP);
                        let speed = if other.is_some() { 9.0 } else { self.content.pet.speed };
                        self.set_path(id, path, speed);
                    }
                }
            }
        }
    }

    // --- damage and death -----------------------------------------------------------------------

    /// `attacker` does `damage` to `victim`. Handles shields, reflections and death.
    pub(super) fn deal(&mut self, attacker: EntityId, victim: EntityId, damage: u32, crit: bool, via: Via) {
        let Some(v) = self.entities.get(&victim) else {
            return;
        };
        if !v.alive() {
            return;
        }
        let mut damage = damage;
        let victim_is_player = v.is_player();
        if let Body::Ally(a) = &v.body {
            if a.pet {
                let owner = a.owner;
                let taken = self.entities.get(&owner).and_then(|e| e.player()).map_or(0.0, |p| p.stats.pet_taken);
                let wrath = self.buff_value(owner, super::spells::BuffKind::BestialWrath);
                let ward = self.buff_value(owner, super::spells::BuffKind::PetWard);
                damage = (damage as f32 * (1.0 - taken.min(0.6)) * (1.0 - wrath.min(0.8)) * (1.0 - ward.min(0.8))).round() as u32;
            }
        }
        let attacker_is_enemy = self.entities.get(&attacker).is_some_and(|a| a.is_enemy());
        // Shield Wall: the party's drilled defenders shrug some of it off.
        if attacker_is_enemy && matches!(&self.entities[&victim].body, Body::Ally(a) if a.owner == self.hub) {
            let wall = super::shop::SOLDIERS[2].per * self.defenses.soldiers[2] as f32;
            damage = (damage as f32 * (1.0 - wall.min(0.6))).round() as u32;
        }
        if victim_is_player {
            damage = self.player_takes(victim, attacker, damage);
            if damage == 0 {
                return;
            }
        }
        // Execute: more against the nearly dead.
        let (exec, leech) = self
            .entities
            .get(&attacker)
            .and_then(|a| a.player())
            .map_or((0.0, 0.0), |p| (p.stats.execute, p.stats.leech));
        if exec > 0.0 && self.entities[&victim].is_enemy() {
            let v = &self.entities[&victim];
            if v.health * 4 < v.max_health {
                damage = (damage as f32 * (1.0 + exec)) as u32;
            }
        }
        // The Ancient's defences hit marked, frozen and rallied-to enemies harder, and blessed
        // defenders hit harder.
        if self.entities[&victim].is_enemy() && self.is_defense(attacker) {
            damage = (damage as f32 * self.defence_bonus(attacker, victim)).round() as u32;
        }
        // Hamstrung (crippled) enemies take more from everyone.
        if let Some(en) = self.entities.get(&victim).and_then(|e| e.enemy()) {
            if en.aff.exposed_until > self.clock && en.aff.exposed > 0.0 {
                damage = (damage as f32 * (1.0 + en.aff.exposed)).round() as u32;
            }
        }
        // Armor: elites and bosses shrug off part of every physical blow from heroes and their
        // pets, until Sunder Armor strips it (each stack takes ARMOR_PER_SUNDER off).
        let physical = match via {
            Via::Melee => true,
            Via::Spell { school, .. } => school == 0,
        };
        if physical && !self.is_defense(attacker) {
            let hero = matches!(self.entities.get(&attacker).map(|a| &a.body), Some(Body::Player(_)) | Some(Body::Ally(_)));
            if hero {
                if let Some(en) = self.entities.get(&victim).and_then(|e| e.enemy()) {
                    let sc = &self.content.scaling;
                    let base = if en.kind.boss { sc.boss_armor } else if en.elite { sc.elite_armor } else { 0.0 };
                    if base > 0.0 {
                        let s = en.aff.vulns[super::spells::Vuln::Sunder as usize];
                        let stacks = if s.until > self.clock { s.n as f32 } else { 0.0 };
                        let armor = (base - stacks * sc.armor_per_sunder).max(0.0).min(0.8);
                        damage = (damage as f32 * (1.0 - armor)).round().max(1.0) as u32;
                    }
                }
            }
        }
        // Frozen enemies take more from a frost-resonant build.
        if self.entities[&victim].is_enemy() && self.frozen(victim) && self.resonant_any(attacker, "frost") {
            damage = (damage as f32 * 1.3) as u32;
        }
        // Who gets the credit if this kills an enemy: a player, or an ally's owner.
        let owner = match self.entities.get(&attacker).map(|a| &a.body) {
            Some(Body::Player(_)) => Some(attacker),
            Some(Body::Ally(a)) => Some(a.owner),
            _ => None,
        };
        let v = self.entities.get_mut(&victim).unwrap();
        let mut lethal = damage >= v.health;
        // Undying Will: once a wave, a killing blow leaves the player standing.
        if lethal && victim_is_player {
            let (health, max) = (v.health, v.max_health);
            if let Some(p) = v.player_mut() {
                if p.undying && !p.undying_used {
                    p.undying_used = true;
                    damage = health.saturating_sub(max * 3 / 10).max(1).min(health.saturating_sub(1));
                    lethal = false;
                }
            }
        }
        // The meter's line for it, worked out while nothing is borrowed.
        let side = if self.entities[&victim].is_enemy() { self.defence_side(attacker) } else { None };
        let label = match owner {
            Some(o) if self.entities[&victim].is_enemy() && self.entities.get(&o).is_some_and(|e| e.is_player()) => Some(self.meter_label(attacker, o, via)),
            _ => None,
        };
        let v = self.entities.get_mut(&victim).unwrap();
        let landed = damage.min(v.health);
        let on_enemy = v.is_enemy();
        v.health = v.health.saturating_sub(damage);
        let (health, max_health) = (v.health, v.max_health);
        if let (Some(en), Some(owner)) = (v.enemy_mut(), owner) {
            en.last_hit_by = Some(owner);
            en.last_spell = match via {
                Via::Spell { spell, .. } => spell,
                Via::Melee => 0,
            };
        }
        if let (true, Some(owner)) = (on_enemy, owner) {
            if let Some(p) = self.entities.get_mut(&owner).and_then(|e| e.player_mut()) {
                p.dealt += u64::from(landed);
                p.meter.wave += u64::from(landed);
                p.meter.total += u64::from(landed);
                if let Some(label) = label {
                    match p.meter.by.iter_mut().find(|(l, _)| *l == label) {
                        Some((_, n)) => *n += u64::from(landed),
                        None => p.meter.by.push((label, u64::from(landed))),
                    }
                }
            }
        }
        if let Some(i) = side {
            self.run.defence_meter[i] += u64::from(landed);
        }
        // A Heartwood Locket: a share of the hero's damage mends the Ancient.
        if on_enemy {
            let leech = owner.and_then(|o| self.entities.get(&o)).and_then(|o| o.player()).map_or(0.0, |p| p.stats.hub_leech);
            if leech > 0.0 && landed > 0 {
                self.run.hub_mend += landed as f32 * leech;
                if self.run.hub_mend >= 1.0 {
                    let mend = self.run.hub_mend.floor();
                    self.run.hub_mend -= mend;
                    let hub = self.hub;
                    let h = self.entities.get(&hub).map_or(0, |e| e.health);
                    if h > 0 {
                        self.set_health(hub, h + mend as u32);
                    }
                }
            }
        }
        if victim_is_player && landed > 0 {
            let from = match self.entities.get(&attacker).map(|a| &a.body) {
                Some(Body::Enemy(en)) => self.enemy_def(en.kind).name.clone(),
                _ => String::from("something"),
            };
            if let Some(p) = self.entities.get_mut(&victim).and_then(|e| e.player_mut()) {
                p.meter.taken += u64::from(landed);
                p.meter.hits += 1;
                if landed > p.meter.biggest.0 {
                    p.meter.biggest = (landed, from);
                }
            }
        }
        if victim == self.hub {
            self.run.hub_low = self.run.hub_low.min(health as f32 / max_health.max(1) as f32);
        }
        match via {
            Via::Melee => self.events.push(Event::Hit { attacker, victim, damage, crit }),
            Via::Spell { spell, school, periodic } => self.events.push(Event::SpellHit {
                caster: attacker,
                victim,
                spell,
                damage,
                crit,
                school,
                periodic,
            }),
        }
        self.events.push(Event::Health { id: victim, health, max_health });
        if victim == self.hub && attacker_is_enemy && self.clock >= self.hub_alert_at {
            // Nobody at the Ancient: say where it's being hit from.
            let hub_pos = self.arena.hub;
            let guarded = self.entities.values().any(|e| e.is_player() && e.alive() && dist2d(e.pos, hub_pos) <= HUB_GUARD);
            if !guarded {
                let from = self.entities.get(&attacker).map_or(hub_pos, |a| a.pos);
                let (north, west) = (from[0] - hub_pos[0], from[1] - hub_pos[1]);
                let names = ["north", "north-east", "east", "south-east", "south", "south-west", "west", "north-west"];
                let bearing = (-west).atan2(north).to_degrees().rem_euclid(360.0);
                let dir = names[((bearing + 22.5) / 45.0) as usize % 8];
                let name = self.content.hub.name.clone();
                self.announce(format!("The {name} is under attack from the {dir}!"), true);
            }
            self.hub_alert_at = self.clock + HUB_ALERT_EVERY;
        }
        if victim == self.hub {
            let pct = health * 100 / max_health.max(1);
            for (level, mark) in [(1u8, 50u32), (2, 25), (3, 10)] {
                if pct <= mark && self.hub_warned < level {
                    self.hub_warned = level;
                    let name = self.content.hub.name.clone();
                    self.announce(format!("The {name} is at {mark}% health!"), true);
                }
            }
            // Thornbark bites back.
            if self.run.thornbark > 0 && attacker_is_enemy && via == Via::Melee {
                let thorns = (self.run.thornbark as f32 * (5.0 + 1.0 * self.run.wave as f32) * self.damage_mult().sqrt()) as u32;
                self.deal(self.hub, attacker, thorns, false, Via::Spell { spell: 467, school: 3, periodic: false });
            }
        }
        // Lightning Shield answers a melee hit.
        if victim_is_player && attacker_is_enemy && via == Via::Melee && !lethal {
            self.lightning_shield(victim, attacker);
        }
        // A thorned elite pays back a share of every hit taken up close.
        if via == Via::Melee
            && self.entities.get(&victim).and_then(|e| e.enemy()).is_some_and(|en| en.modifier == THORNED)
            && self.entities.get(&attacker).is_some_and(|a| a.is_player() && a.alive())
        {
            let back = (damage as f32 * 0.3).round() as u32;
            if back > 0 {
                self.deal(victim, attacker, back, false, Via::Spell { spell: THORNS_SPELL, school: 3, periodic: false });
            }
        }
        if leech > 0.0 && !victim_is_player {
            let amount = (damage as f32 * leech).round() as u32;
            if amount > 0 {
                let e = &self.entities[&attacker];
                if e.alive() && e.health < e.max_health {
                    self.heal(attacker, attacker, 15286, amount, false);
                }
            }
        }
        if health > 0 {
            self.boss_phase(victim, health, max_health);
        }
        if health == 0 {
            self.kill(victim, attacker);
        } else if victim_is_player && via == Via::Melee && attacker_is_enemy {
            if self.has_affix("frostbound") {
                // A chill: shown as a short Frostbolt slow. Players' speed is the client's, so
                // the slow is felt through the speed change event.
                self.chill_player(victim);
            }
            if self.has_affix("necrotic") {
                if let Some(p) = self.entities.get_mut(&victim).and_then(|e| e.player_mut()) {
                    p.necrotic = (p.necrotic + 1).min(10);
                    p.necrotic_until = self.clock + 6.0;
                }
            }
        }
    }

    pub(super) fn kill(&mut self, id: EntityId, killer: EntityId) {
        self.events.push(Event::Died { id });
        // Whoever was fighting it stops.
        let attackers: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| match &e.body {
                Body::Player(p) => p.attacking == Some(id),
                Body::Enemy(en) => en.engaged == Some(id),
                Body::Ally(a) => a.engaged == Some(id),
                _ => false,
            })
            .map(|(&a, _)| a)
            .collect();
        for a in attackers {
            match &mut self.entities.get_mut(&a).unwrap().body {
                Body::Player(p) => p.attacking = None,
                Body::Enemy(en) => {
                    en.engaged = None;
                    if en.goal == Goal::Player(id) || en.goal == Goal::Attack(id) {
                        en.goal = if en.guard.is_some() { Goal::Guard } else { Goal::Hub };
                        en.think = 0.0;
                    }
                }
                Body::Ally(al) => al.engaged = None,
                _ => {}
            }
            self.events.push(Event::AttackStop { attacker: a, victim: id });
        }
        let corpse_secs = self.content.game.corpse_secs;
        if self.entities.get(&id).is_some_and(|e| e.is_enemy()) {
            self.death_spreads(id);
        }
        let e = self.entities.get_mut(&id).unwrap();
        let pos = e.pos;
        match &mut e.body {
            Body::Enemy(en) => {
                en.corpse = Some(corpse_secs);
                en.dots.clear();
                en.aff.mark = None;
                en.debuffs_dirty = true;
                let was_moving = en.mover.take().is_some_and(|m| !m.done());
                if let Some(victim) = en.engaged.take() {
                    self.events.push(Event::AttackStop { attacker: id, victim });
                }
                if was_moving {
                    let (pos, facing) = (e.pos, e.facing);
                    self.events.push(Event::Stop { id, pos, facing });
                }
                self.enemy_died(id, killer, pos);
            }
            Body::Player(p) => {
                let name = p.name.clone();
                if let Some(victim) = p.attacking.take() {
                    self.events.push(Event::AttackStop { attacker: id, victim });
                }
                p.channel = None;
                self.events.push(Event::ChannelEnd { player: id });
                let rise = self.entities[&id].player().map_or(0.0, |p| p.stats.rise);
                let mut wait = self.respawn_secs() * (1.0 - rise.clamp(0.0, 0.8));
                let sor = self.entities[&id].player().map_or(0.0, |p| p.stats.procs.spirit_of_redemption);
                if sor > 0.0 {
                    wait = (wait - 4.0).max(2.0);
                    self.spirit_of_redemption(id, pos, sor);
                }
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.falls += 1;
                    p.meter.fell += 1;
                    p.respawn_at = self.clock + wait;
                    p.buffs.clear();
                    p.hots.clear();
                    p.shield_pool = 0.0;
                }
                self.refresh_auras(id);
                self.announce(format!("{name} has fallen! Back at the Ancient in {} seconds.", wait.round() as u32), false);
            }
            Body::Ally(a) => {
                let (pet, owner) = (a.pet, a.owner);
                self.halt_enemy(id);
                self.entities.remove(&id);
                self.events.push(Event::Despawn { id });
                if pet {
                    self.pet_fell(id, owner);
                } else {
                    self.defense_fell(id);
                }
            }
            Body::Npc { npc: NpcKind::Protector | NpcKind::Spring, .. } => {
                self.entities.remove(&id);
                self.events.push(Event::Despawn { id });
                self.defense_fell(id);
            }
            _ => {}
        }
    }

    /// What an enemy's death sets loose: damage over time that jumps on, a mark that moves,
    /// cooldowns a mark resets.
    fn death_spreads(&mut self, id: EntityId) {
        let Some(e) = self.entities.get(&id) else { return };
        let Some(en) = e.enemy() else { return };
        let pos = e.pos;
        let spreads: Vec<(EntityId, u32, u8, f32, u32, f32)> = en
            .dots
            .iter()
            .filter(|d| d.spread > 0.0)
            .map(|d| (d.caster, d.spell, d.school, d.per_tick * d.ticks_left.max(4) as f32, d.ticks_left.max(4), d.spread))
            .collect();
        let mark = en.aff.mark;
        for (caster, spell, school, total, ticks, n) in spreads {
            let others: Vec<EntityId> = self.enemies_near(pos, 12.0).into_iter().map(|(o, _)| o).filter(|o| *o != id).take(n as usize).collect();
            for o in others {
                self.add_dot_full(caster, o, spell, school, total, ticks as f32, false, 0.0, n, 0.0);
                self.events.push(Event::SpellGo { caster, spell, target: SpellTarget::Unit(o), hits: vec![o] });
            }
        }
        if let Some(m) = mark {
            if m.jump {
                if let Some((o, _)) = self.enemies_near(pos, 30.0).into_iter().find(|(o, _)| *o != id) {
                    if let Some(en) = self.entities.get_mut(&o).and_then(|e| e.enemy_mut()) {
                        en.aff.mark = Some(m);
                        en.debuffs_dirty = true;
                    }
                }
            }
            if m.reset {
                if let Some(p) = self.entities.get_mut(&m.owner).and_then(|e| e.player_mut()) {
                    for o in p.spells.iter_mut() {
                        if o.nums.mark <= 0.0 {
                            o.cooldown = 0.0;
                        }
                    }
                }
                self.events.push(Event::Bar { player: m.owner });
            }
        }
    }

    /// The priest's spirit lingers, healing everyone near.
    fn spirit_of_redemption(&mut self, id: EntityId, pos: Vec3, share: f32) {
        let zid = self.alloc();
        self.entities.insert(
            zid,
            Entity {
                pos,
                facing: 0.0,
                health: 1,
                max_health: 1,
                level: 1,
                body: Body::Zone(super::cards::Zone {
                    caster: id,
                    spell: 27827,
                    radius: 30.0,
                    until: self.clock + 8.0,
                    next_tick: self.clock + 1.0,
                    per_tick: 0.0,
                    school: 1,
                    slow: 0.0,
                    hostile: false,
                    heal: share,
                    share: 0.0,
                    info: None,
                    cast: 0,
                }),
            },
        );
        self.events.push(Event::SpellGo { caster: id, spell: 27827, target: SpellTarget::Point(pos), hits: vec![] });
        self.events.push(Event::ZoneStart { id: zid, caster: id, spell: 27827, pos, radius: 30.0, mark: GroundMark::None });
    }

    /// An enemy just died: experience, gold, loot, affixes and triggers.
    fn enemy_died(&mut self, id: EntityId, killer: EntityId, pos: Vec3) {
        let (kind, elite, guard, owner, stream, add, detonated) = {
            let en = self.entities[&id].enemy().unwrap();
            (en.kind, en.elite, en.guard, en.last_hit_by, en.stream, en.parent.is_some(), en.detonated)
        };
        // A sapper that got through: its blast was its last act, and nobody earned anything.
        if detonated {
            if let Some(b) = guard {
                self.guardian_died(b, id);
            }
            return;
        }
        self.run.kills += 1;
        // The raised and the split-off are worth little: no farming a necromancer.
        let weight = self.xp_weight(kind, elite)
            * if guard.is_some() { 2.0 } else { 1.0 }
            * if stream { 0.5 } else { 1.0 }
            * if add { 0.2 } else { 1.0 };
        let xp = (self.run.xp_per_weight * weight).max(1.0);
        // Gold for the stream is a trickle and for the raised dead nothing: they never stop
        // coming, and a wave's worth of gold shouldn't grow with how long it lasts.
        let mine = if self.boon("the Gold Mine") { 1.5 } else { 1.0 };
        // A base's guardians pay as ordinary enemies: the base itself is the reward (its cache,
        // its boon, a bonus level), and bases are guarded again and again.
        let gold = if add {
            0
        } else if stream {
            ((1.0 + 0.1 * self.run.wave as f32) * mine).round() as u32
        } else if guard.is_some() {
            (self.run.gold_per_weight * self.enemy_def(kind).xp * mine).round() as u32
        } else {
            (self.run.gold_per_weight * weight * mine).round() as u32
        };
        for pid in self.player_ids() {
            self.give_xp(pid, xp, Some(id));
            if gold > 0 {
                self.earn_gold(pid, gold);
            }
        }
        let credit = owner.or(Some(killer)).filter(|k| self.entities.get(k).is_some_and(|e| e.is_player()));
        if let Some(k) = credit {
            if let Some(p) = self.entities.get_mut(&k).and_then(|e| e.player_mut()) {
                p.kills += 1;
            }
            let last = self.entities.get(&id).and_then(|e| e.enemy()).map_or(0, |en| en.last_spell);
            let spell = if last != 0 { self.spell_of_visual(k, last) } else { None };
            self.fire(k, crate::kit::On::Kill, spell, Some(id), 0.0);
            self.on_kill_triggers(k, id, pos);
        }
        // Affixes.
        if self.has_affix("bursting") {
            let burst = (self.entities[&id].max_health as f32 * 0.10) as u32;
            let near: Vec<EntityId> = self
                .entities
                .iter()
                .filter(|(_, e)| e.is_player() && e.alive() && dist2d(e.pos, pos) <= 5.0)
                .map(|(&pid, _)| pid)
                .collect();
            if !near.is_empty() {
                self.events.push(Event::SpellGo { caster: id, spell: 1949, target: SpellTarget::Point(pos), hits: near.clone() });
            }
            for pid in near {
                let dmg = burst.min(self.entities[&pid].max_health / 5);
                self.deal(id, pid, dmg, false, Via::Spell { spell: 1949, school: 2, periodic: false });
            }
        }
        if self.has_affix("bolstering") {
            let near: Vec<EntityId> = self.enemies_near(pos, 10.0).into_iter().map(|(e, _)| e).filter(|&e| e != id).collect();
            for e in near {
                // A few stacks at most: a dying swarm mustn't grow the last of it into a boss.
                let fresh = self.entities.get_mut(&e).and_then(|x| x.enemy_mut()).is_some_and(|en| {
                    en.bolster += 1;
                    en.bolster <= BOLSTER_STACKS
                });
                if !fresh {
                    continue;
                }
                if let Some(ent) = self.entities.get_mut(&e) {
                    ent.max_health = (ent.max_health as f32 * 1.15) as u32;
                    ent.health = (ent.health as f32 * 1.15) as u32;
                    let (health, max_health) = (ent.health, ent.max_health);
                    if let Some(en) = ent.enemy_mut() {
                        en.damage_mult *= 1.15;
                    }
                    self.events.push(Event::Health { id: e, health, max_health });
                }
            }
        }
        // A splitter falls apart into smaller dead.
        let (split, count) = {
            let d = self.enemy_def(kind);
            (d.split_into.clone(), d.split_count)
        };
        if count > 0 {
            if let Some(index) = self.content.enemies.iter().position(|e| e.key == split) {
                self.events.push(Event::SpellGo { caster: id, spell: SPLIT_SPELL, target: SpellTarget::Point(pos), hits: vec![] });
                for k in 0..count {
                    let a = k as f32 / count as f32 * std::f32::consts::TAU;
                    let at = self.ring(pos, 2.5, a);
                    self.spawn_add(id, index, at);
                }
            }
        }
        if kind.boss && !add {
            let name = self.enemy_def(kind).name.clone();
            self.bonus_level(self.content.levels.boss_bonus, &format!("{name} is slain"));
        }
        // Loot and base bookkeeping.
        self.roll_corpse_loot(id, kind, elite, guard);
        if let Some(b) = guard {
            self.guardian_died(b, id);
        }
    }

    fn remove_corpse(&mut self, id: EntityId) {
        self.entities.remove(&id);
        if let Some(loots) = self.loot.remove(&id) {
            for l in loots {
                self.events.push(Event::Lootable { corpse: id, player: l.player, lootable: false });
            }
        }
        self.events.push(Event::Despawn { id });
    }
}
