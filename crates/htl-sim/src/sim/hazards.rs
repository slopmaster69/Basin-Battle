//! What players dodge and what speeds them up: the abilities bosses and elites cast (a marked
//! patch of ground, a cast, then the hit) and the sprint pick-ups along the lanes.

use rand::seq::SliceRandom;

use super::cards::Zone;
use super::enemies::Via;
use super::*;

/// How far a caster looks for players to aim a pool at.
const POOL_RANGE: f32 = 35.0;
/// How close a player must be for an enemy's abilities to wake up.
const WAKE_RANGE: f32 = 25.0;
/// Seconds between ordinary enemies' strikes on any one player. Bosses and elites aren't held
/// back; a crowd of ordinary dead takes turns.
const STRIKE_GAP: f32 = 3.5;
/// The buff slot the sprint shows in.
pub(super) const SPRINT_SLOT: u8 = 29;
pub(super) const SPRINT_SPELL: u32 = 2983;

/// A sprint pick-up's spot, and what stands there now.
#[derive(Debug, Clone)]
pub(super) struct SprintSpot {
    pub pos: Vec3,
    pub entity: Option<EntityId>,
    pub back_at: f32,
}

impl Sim {
    // --- enemy abilities ----------------------------------------------------------------------

    /// Run an enemy's abilities: true while it is busy casting (it does nothing else).
    pub(super) fn enemy_ability(&mut self, id: EntityId) -> bool {
        let now = self.clock;
        let (pos, casting, abilities, ready_at) = {
            let Some(e) = self.entities.get(&id) else { return false };
            let Some(en) = e.enemy() else { return false };
            if en.abilities.is_empty() {
                return false;
            }
            (e.pos, en.casting, en.abilities.clone(), en.ready_at.clone())
        };
        if let Some((_, lands)) = casting {
            if now >= lands {
                if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                    en.casting = None;
                }
                return false;
            }
            return true;
        }
        let near = self.nearest_player(pos, WAKE_RANGE);
        // A healer wakes for its hurt allies too, players or not.
        let mends = abilities.iter().any(|&a| self.content.abilities[a].kind == "heal") && self.hurt_enemy_near(id, pos, 15.0);
        if near.is_none() && !mends {
            return false;
        }
        // First contact: the abilities wake up after a moment.
        if ready_at.iter().any(|t| t.is_infinite()) {
            if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                for (k, &a) in abilities.iter().enumerate() {
                    if en.ready_at[k].is_infinite() {
                        en.ready_at[k] = now + self.content.abilities[a].first_after;
                    }
                }
            }
            return false;
        }
        let ordinary = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| !en.kind.boss && !en.elite);
        // The first ready ability that has someone to hit.
        for (k, &a) in abilities.iter().enumerate() {
            if ready_at[k] > now {
                continue;
            }
            let def = &self.content.abilities[a];
            let reach = match def.kind.as_str() {
                "slam" => def.radius + 3.0,
                "ring" => def.ring + def.radius + 3.0,
                "line" => def.length,
                _ => POOL_RANGE,
            };
            let in_reach = if def.kind == "heal" {
                self.hurt_enemy_near(id, pos, def.radius)
            } else if def.kind == "bolt" {
                self.nearest_player(pos, reach).is_some()
            } else if ordinary {
                self.strikable(pos, reach).is_some()
            } else {
                self.nearest_player(pos, reach).is_some()
            };
            if in_reach {
                self.cast_ability(id, k, a);
                return true;
            }
        }
        false
    }

    /// Whether an enemy within `radius` of `pos` (the caster too) is hurt enough to mend.
    fn hurt_enemy_near(&self, _caster: EntityId, pos: Vec3, radius: f32) -> bool {
        let now = self.clock;
        self.entities.values().any(|e| {
            e.alive() && e.enemy().is_some_and(|en| en.aff.wound_until <= now) && e.health * 100 < e.max_health * 85 && dist2d(e.pos, pos) <= radius
        })
    }

    /// The nearest player within `radius` that an ordinary strike may aim at now.
    fn strikable(&self, pos: Vec3, radius: f32) -> Option<EntityId> {
        let now = self.clock;
        self.entities
            .iter()
            .filter(|(_, e)| e.alive() && e.player().is_some_and(|p| now - p.struck_at >= STRIKE_GAP))
            .map(|(&id, e)| (id, dist2d(e.pos, pos)))
            .filter(|(_, d)| *d <= radius)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    fn cast_ability(&mut self, id: EntityId, k: usize, a: usize) {
        let def = self.content.abilities[a].clone();
        let now = self.clock;
        let lands = now + def.cast_secs;
        let pos = self.entities[&id].pos;
        // One hit is worth the caster's average swing.
        let hit = {
            let en = self.entities[&id].enemy().unwrap();
            let d = self.enemy_def(en.kind);
            (d.damage[0] + d.damage[1]) as f32 / 2.0 * en.damage_mult
        };
        self.halt_enemy(id);
        if matches!(def.kind.as_str(), "bolt" | "heal") {
            self.cast_spell_ability(id, k, def, hit);
            return;
        }
        let centers: Vec<Vec3> = match def.kind.as_str() {
            "slam" => vec![pos],
            "line" => {
                // Towards a player in reach, marks every 1.5 radii out to the line's length.
                let target = self.nearest_player(pos, def.length).and_then(|p| self.entities.get(&p)).map(|e| e.pos);
                let toward = target.unwrap_or([pos[0] + 1.0, pos[1], pos[2]]);
                let (dx, dy) = (toward[0] - pos[0], toward[1] - pos[1]);
                let len = (dx * dx + dy * dy).sqrt().max(0.01);
                let (ux, uy) = (dx / len, dy / len);
                let step = def.radius * 1.5;
                let n = ((def.length - def.radius) / step).floor().max(0.0) as usize + 1;
                (0..n)
                    .map(|i| {
                        let d = def.radius + step * i as f32;
                        let (x, y) = (pos[0] + ux * d, pos[1] + uy * d);
                        [x, y, self.ground.height(x, y)]
                    })
                    .collect()
            }
            "ring" => {
                // A ring of marks at `ring` yards: the caster's feet are safe.
                let around = 2.0 * std::f32::consts::PI * def.ring;
                let n = ((around / (def.radius * 1.6)).ceil() as usize).max(6);
                (0..n)
                    .map(|i| {
                        let a = i as f32 / n as f32 * 2.0 * std::f32::consts::PI;
                        let (x, y) = (pos[0] + a.cos() * def.ring, pos[1] + a.sin() * def.ring);
                        [x, y, self.ground.height(x, y)]
                    })
                    .collect()
            }
            _ => {
                let ordinary = self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| !en.kind.boss && !en.elite);
                let now = self.clock;
                let mut players: Vec<(EntityId, Vec3)> = self
                    .order
                    .iter()
                    .filter_map(|p| self.entities.get(p).map(|e| (*p, e)))
                    .filter(|(_, e)| e.alive() && dist2d(e.pos, pos) <= POOL_RANGE)
                    .filter(|(_, e)| !ordinary || e.player().is_some_and(|p| now - p.struck_at >= STRIKE_GAP))
                    .map(|(p, e)| (p, e.pos))
                    .collect();
                players.shuffle(&mut self.rng);
                players.into_iter().take(def.targets.max(1) as usize).map(|(_, p)| p).collect()
            }
        };
        if let Some(c) = centers.first() {
            if let Some(e) = self.entities.get_mut(&id) {
                if def.kind != "slam" && def.kind != "ring" {
                    e.facing = facing(pos, *c);
                }
            }
        }
        let (until, per_tick) = match def.kind.as_str() {
            "slam" | "line" | "ring" if def.duration <= 0.0 => (lands + 0.5, hit * def.damage),
            _ => (lands + def.duration.max(0.5), hit * def.damage),
        };
        let cast = if centers.len() > 1 && matches!(def.kind.as_str(), "line" | "ring") {
            self.alloc()
        } else {
            0
        };
        for &c in &centers {
            let zid = self.alloc();
            self.entities.insert(
                zid,
                Entity {
                    pos: c,
                    facing: 0.0,
                    health: 1,
                    max_health: 1,
                    level: 1,
                    body: Body::Zone(Zone {
                        caster: id,
                        spell: def.telegraph,
                        radius: def.radius,
                        until,
                        next_tick: lands,
                        per_tick,
                        school: def.school,
                        slow: def.slow,
                        hostile: true,
                        heal: 0.0,
                        share: def.health_share,
                        info: None,
                        cast,
                    }),
                },
            );
            self.events.push(Event::ZoneStart { id: zid, caster: id, spell: def.telegraph, pos: c, radius: def.radius, mark: GroundMark::Danger });
        }
        // Who stands in it now, for their warnings.
        let endangered: Vec<EntityId> = self
            .order
            .iter()
            .copied()
            .filter(|p| {
                self.entities
                    .get(p)
                    .is_some_and(|e| e.alive() && centers.iter().any(|c| dist2d(e.pos, *c) <= def.radius + 1.0))
            })
            .collect();
        // An ordinary strike makes the others wait their turn on whoever it threatens.
        if self.entities.get(&id).and_then(|e| e.enemy()).is_some_and(|en| !en.kind.boss && !en.elite) {
            let marked: Vec<EntityId> = self
                .order
                .iter()
                .copied()
                .filter(|p| self.entities.get(p).is_some_and(|e| e.alive() && centers.iter().any(|c| dist2d(e.pos, *c) <= def.radius + 3.0)))
                .collect();
            for p in marked {
                if let Some(pl) = self.entities.get_mut(&p).and_then(|e| e.player_mut()) {
                    pl.struck_at = now;
                }
            }
        }
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.casting = Some((a, lands));
            en.ready_at[k] = lands + def.cooldown;
        }
        self.events.push(Event::EnemyCast {
            caster: id,
            name: def.name.clone(),
            spell: def.spell,
            ms: (def.cast_secs * 1000.0) as u32,
            warn: def.warn.clone(),
            endangered: endangered.clone(),
            big: def.health_share > 0.0,
            interruptible: def.interruptible,
        });
        if def.interruptible {
            self.cue_interrupts(id, def.cast_secs);
        }
        // When it lands: the caster's cast ends, the impact shows.
        self.landings.push(Landing {
            at: lands,
            caster: id,
            spell: def.spell,
            impact: def.impact,
            centers,
            suicide: def.suicide,
            endangered,
            radius: def.radius,
            name: def.name.clone(),
            effect: LandEffect::Zones,
            bound: Vec::new(),
        });
    }

    /// A spell an enemy casts at someone (a bolt) or for its own (a mass heal): no marks on the
    /// ground to dodge, a cast bar to break. It lands unless the caster is interrupted or killed.
    fn cast_spell_ability(&mut self, id: EntityId, k: usize, def: crate::content::AbilityDef, hit: f32) {
        let now = self.clock;
        let lands = now + def.cast_secs;
        let pos = self.entities[&id].pos;
        let (effect, endangered) = match def.kind.as_str() {
            "bolt" => {
                let Some(target) = self.nearest_player(pos, POOL_RANGE) else { return };
                if let Some(t) = self.entities.get(&target).map(|e| e.pos) {
                    if let Some(e) = self.entities.get_mut(&id) {
                        e.facing = facing(pos, t);
                    }
                }
                (LandEffect::Bolt { target, damage: hit * def.damage, share: def.health_share, school: def.school }, vec![target])
            }
            _ => {
                // Whoever is in reach as the cast begins: the spell is bound to them.
                let now = self.clock;
                let bound: Vec<EntityId> = self
                    .entities
                    .iter()
                    .filter(|(_, e)| e.alive() && e.enemy().is_some_and(|en| en.aff.wound_until <= now) && dist2d(e.pos, pos) <= def.radius)
                    .map(|(&i, _)| i)
                    .collect();
                (LandEffect::Heal { share: def.heal }, bound)
            }
        };
        let (endangered, bound) = if def.kind == "heal" { (Vec::new(), endangered) } else { (endangered, Vec::new()) };
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.casting = Some((k, lands));
            en.ready_at[k] = lands + def.cooldown;
        }
        self.events.push(Event::EnemyCast {
            caster: id,
            name: def.name.clone(),
            spell: def.spell,
            ms: (def.cast_secs * 1000.0) as u32,
            warn: def.warn.clone(),
            endangered: endangered.clone(),
            big: true,
            interruptible: def.interruptible,
        });
        if def.interruptible {
            self.cue_interrupts(id, def.cast_secs);
        }
        self.landings.push(Landing {
            at: lands,
            caster: id,
            spell: def.spell,
            impact: def.impact,
            centers: vec![pos],
            suicide: false,
            endangered: Vec::new(),
            radius: def.radius,
            name: def.name.clone(),
            effect,
            bound,
        });
    }

    /// An interruptible cast began: every hero who holds an interrupt and can reach the caster
    /// sees it light up for as long as the cast lasts.
    fn cue_interrupts(&mut self, caster: EntityId, secs: f32) {
        let Some(at) = self.entities.get(&caster).map(|e| e.pos) else { return };
        for p in self.player_ids() {
            let found = {
                let Some(e) = self.entities.get(&p) else { continue };
                let Some(pl) = e.player() else { continue };
                if !e.alive() {
                    continue;
                }
                let kit = self.content.kit(pl.class);
                pl.spells.iter().find(|o| o.nums.interrupt > 0.0 && dist2d(e.pos, at) <= o.nums.interrupt).map(|o| kit.spells[o.spell].key.clone())
            };
            if let Some(key) = found {
                let cue = crate::kit::BuffDef { key: "interrupt_cue".into(), secs, stacks: 1, next: vec![key], hold: true, ..Default::default() };
                self.give_tbuff(p, &cue);
            }
        }
    }

    /// Break an enemy's cast, if it is casting something that can be broken: its marks vanish,
    /// it never lands, the caster reels (stunned 2 sec). True if a cast was broken.
    pub(super) fn interrupt_enemy(&mut self, by: EntityId, victim: EntityId) -> bool {
        let Some((a, lands)) = self.entities.get(&victim).and_then(|e| e.enemy()).and_then(|en| en.casting) else { return false };
        let Some(&ability) = self.entities.get(&victim).and_then(|e| e.enemy()).and_then(|en| en.abilities.get(a)) else { return false };
        let def = self.content.abilities[ability].clone();
        if !def.interruptible || lands <= self.clock {
            return false;
        }
        if let Some(en) = self.entities.get_mut(&victim).and_then(|e| e.enemy_mut()) {
            en.casting = None;
        }
        self.landings.retain(|l| l.caster != victim);
        let zones: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(&e.body, Body::Zone(z) if z.caster == victim && z.hostile))
            .map(|(&z, _)| z)
            .collect();
        for z in zones {
            self.entities.remove(&z);
            self.events.push(Event::ZoneEnd { id: z });
        }
        self.events.push(Event::CastInterrupted { caster: victim, spell: def.spell });
        self.stun_enemy(victim, 2.0);
        self.events.push(Event::Proc { player: by, name: format!("Interrupted: {}", def.name), icon: 72 });
        true
    }

    /// Whether a player holds an interrupt that reaches `caster`.
    pub fn can_interrupt(&self, player: EntityId, caster: EntityId) -> bool {
        let (Some(e), Some(c)) = (self.entities.get(&player), self.entities.get(&caster)) else { return false };
        e.alive() && e.player().is_some_and(|p| p.spells.iter().any(|o| o.nums.interrupt > 0.0 && dist2d(e.pos, c.pos) <= o.nums.interrupt))
    }

    /// Whether an enemy is casting something that can be interrupted.
    pub(super) fn casting_breakable(&self, id: EntityId) -> bool {
        let Some(en) = self.entities.get(&id).and_then(|e| e.enemy()) else { return false };
        let Some((a, lands)) = en.casting else { return false };
        lands > self.clock && en.abilities.get(a).is_some_and(|&ab| self.content.abilities[ab].interruptible)
    }

    /// Show casts landing.
    pub(super) fn step_landings(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<Landing>, Vec<Landing>) = std::mem::take(&mut self.landings).into_iter().partition(|l| l.at <= now);
        self.landings = keep;
        for l in due {
            if !self.entities.get(&l.caster).is_some_and(|e| e.alive()) {
                continue;
            }
            let first = l.centers.first().copied().unwrap_or(self.entities[&l.caster].pos);
            // Whoever stood in it when it was cast and is clear of it now: dodged!
            for &p in &l.endangered {
                let clear = self
                    .entities
                    .get(&p)
                    .is_some_and(|e| e.alive() && l.centers.iter().all(|c| dist2d(e.pos, *c) > l.radius));
                if clear {
                    if let Some(pl) = self.entities.get_mut(&p).and_then(|e| e.player_mut()) {
                        pl.dodges += 1;
                    }
                    self.events.push(Event::Dodged { player: p, name: l.name.clone() });
                }
            }
            match l.effect {
                LandEffect::Zones => {}
                LandEffect::Bolt { target, damage, share, school } => {
                    if self.entities.get(&target).is_some_and(|e| e.alive()) {
                        self.events.push(Event::SpellGo { caster: l.caster, spell: l.spell, target: SpellTarget::Unit(target), hits: vec![target] });
                        let dmg = if share > 0.0 { self.entities[&target].max_health as f32 * share } else { damage };
                        self.deal(l.caster, target, dmg.round().max(1.0) as u32, false, Via::Spell { spell: l.spell, school, periodic: false });
                    }
                    continue;
                }
                LandEffect::Heal { share } => {
                    let now = self.clock;
                    let healed: Vec<(EntityId, u32, u32)> = l
                        .bound
                        .iter()
                        .filter_map(|i| self.entities.get(i).map(|e| (*i, e)))
                        .filter(|(_, e)| e.alive() && e.enemy().is_some_and(|en| en.aff.wound_until <= now))
                        .map(|(i, e)| (i, e.health, e.max_health))
                        .collect();
                    let hits: Vec<EntityId> = healed.iter().map(|h| h.0).collect();
                    self.events.push(Event::SpellGo { caster: l.caster, spell: l.spell, target: SpellTarget::Unit(l.caster), hits });
                    for (i, h, m) in healed {
                        self.set_health(i, (h + (m as f32 * share).round() as u32).min(m));
                    }
                    continue;
                }
            }
            self.events.push(Event::SpellGo { caster: l.caster, spell: l.spell, target: SpellTarget::Point(first), hits: vec![] });
            if l.impact != 0 {
                for c in &l.centers {
                    self.events.push(Event::SpellGo { caster: l.caster, spell: l.impact, target: SpellTarget::Point(*c), hits: vec![] });
                }
            }
            if l.suicide {
                // Its blast still lands (the zone ticks this step); the body is spent.
                let hub = self.hub;
                if let Some(e) = self.entities.get_mut(&l.caster) {
                    e.health = 0;
                }
                self.kill(l.caster, hub);
            }
        }
    }

    /// A danger zone's tick: everyone standing in it is hit.
    #[allow(clippy::too_many_arguments)]
    /// Returns who was hit; `skip` are spared (already hit by another mark of the same cast).
    pub(super) fn hazard_tick(
        &mut self,
        caster: EntityId,
        pos: Vec3,
        radius: f32,
        spell: u32,
        per_tick: f32,
        school: u8,
        slow: f32,
        skip: &[EntityId],
        share: f32,
    ) -> Vec<EntityId> {
        let victims: Vec<EntityId> = self
            .order
            .iter()
            .copied()
            .filter(|p| !skip.contains(p) && self.entities.get(p).is_some_and(|e| e.alive() && dist2d(e.pos, pos) <= radius))
            .collect();
        for v in victims.iter().copied() {
            if let Some(p) = self.entities.get_mut(&v).and_then(|e| e.player_mut()) {
                p.struck += 1;
            }
            // A share-of-health strike takes that share of a hero's max health.
            let dmg = if share > 0.0 && self.entities.get(&v).is_some_and(|e| e.is_player()) {
                (self.entities[&v].max_health as f32 * share).round().max(1.0) as u32
            } else {
                per_tick.round().max(1.0) as u32
            };
            self.deal(caster, v, dmg, false, Via::Spell { spell, school, periodic: true });
            if slow > 0.0 {
                let until = self.clock + 1.5;
                if let Some(p) = self.entities.get_mut(&v).and_then(|e| e.player_mut()) {
                    p.chilled_until = p.chilled_until.max(until);
                }
                self.update_speed(v);
            }
        }
        victims
    }

    // --- sprint pick-ups --------------------------------------------------------------------------

    /// Where the pick-ups stand: fractions along each lane.
    pub(super) fn sprint_spots(arena: &Arena, along: &[f32]) -> Vec<SprintSpot> {
        let mut out = Vec::new();
        for pts in arena.lanes.iter().flat_map(|l| l.routes.iter()) {
            if pts.len() < 2 {
                continue;
            }
            let total: f32 = pts.windows(2).map(|w| dist2d(w[0], w[1])).sum();
            for &f in along {
                let mut left = total * f.clamp(0.0, 1.0);
                for w in pts.windows(2) {
                    let d = dist2d(w[0], w[1]);
                    if left <= d {
                        let t = if d > 0.0 { left / d } else { 0.0 };
                        let p = [w[0][0] + (w[1][0] - w[0][0]) * t, w[0][1] + (w[1][1] - w[0][1]) * t, w[0][2] + (w[1][2] - w[0][2]) * t];
                        // Routes that share a road share its pick-ups.
                        if !out.iter().any(|s: &SprintSpot| dist2d(s.pos, p) < 15.0) {
                            out.push(SprintSpot { pos: p, entity: None, back_at: 0.0 });
                        }
                        break;
                    }
                    left -= d;
                }
            }
        }
        out
    }

    pub(super) fn step_sprints(&mut self) {
        let now = self.clock;
        let tuning = self.content.sprint.clone();
        for i in 0..self.sprints.len() {
            let spot = self.sprints[i].clone();
            match spot.entity {
                None if now >= spot.back_at => {
                    let id = self.alloc();
                    let pos = [spot.pos[0], spot.pos[1], self.ground.height(spot.pos[0], spot.pos[1])];
                    self.entities.insert(
                        id,
                        Entity { pos, facing: 0.0, health: 1, max_health: 1, level: 1, body: Body::Npc { npc: NpcKind::Sprint, base: 0 } },
                    );
                    self.events.push(Event::SpawnNpc { id, npc: NpcKind::Sprint, base: 0, pos, facing: 0.0 });
                    self.sprints[i].entity = Some(id);
                }
                Some(id) => {
                    let Some(pos) = self.entities.get(&id).map(|e| e.pos) else {
                        self.sprints[i].entity = None;
                        continue;
                    };
                    let taker = self
                        .order
                        .iter()
                        .copied()
                        .find(|p| self.entities.get(p).is_some_and(|e| e.alive() && dist2d(e.pos, pos) <= tuning.reach));
                    if let Some(p) = taker {
                        self.entities.remove(&id);
                        self.events.push(Event::Despawn { id });
                        self.sprints[i].entity = None;
                        self.sprints[i].back_at = now + tuning.respawn;
                        self.give_sprint(p, tuning.secs);
                    }
                }
                None => {}
            }
        }
        // Sprints running out.
        for id in self.player_ids() {
            let ended = self.entities.get(&id).and_then(|e| e.player()).is_some_and(|p| p.sprinting && p.sprint_until <= now);
            if ended {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    p.sprinting = false;
                }
                self.events.push(Event::Aura { unit: id, slot: SPRINT_SLOT, spell: 0, stacks: 0, secs: 0.0 });
                self.update_speed(id);
            }
        }
    }

    pub(super) fn give_sprint(&mut self, id: EntityId, secs: f32) {
        let until = self.clock + secs;
        if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
            p.sprint_until = until;
            p.sprinting = true;
        }
        self.events.push(Event::SpellGo { caster: id, spell: SPRINT_SPELL, target: SpellTarget::Unit(id), hits: vec![id] });
        self.events.push(Event::Aura { unit: id, slot: SPRINT_SLOT, spell: SPRINT_SPELL, stacks: 1, secs });
        self.update_speed(id);
    }

    /// Take away every pick-up and put them all back (a new run).
    pub(super) fn reset_sprints(&mut self) {
        for i in 0..self.sprints.len() {
            if let Some(id) = self.sprints[i].entity.take() {
                if self.entities.remove(&id).is_some() {
                    self.events.push(Event::Despawn { id });
                }
            }
            self.sprints[i].back_at = 0.0;
        }
    }
}

/// A cast about to land.
#[derive(Debug, Clone)]
pub(super) struct Landing {
    pub at: f32,
    pub caster: EntityId,
    pub spell: u32,
    pub impact: u32,
    pub centers: Vec<Vec3>,
    /// The caster dies as it lands.
    pub suicide: bool,
    /// The players standing in it when it was cast, its marks' radius, and its name: those
    /// clear of it when it lands dodged it.
    pub endangered: Vec<EntityId>,
    pub radius: f32,
    pub name: String,
    /// What happens as it lands, besides its marks.
    pub effect: LandEffect,
    /// A heal's targets, bound as the cast began.
    pub bound: Vec<EntityId>,
}

/// A cast's effect as it lands.
#[derive(Debug, Clone, Copy)]
pub(super) enum LandEffect {
    /// Its marks on the ground do the work.
    Zones,
    /// A bolt at one hero: `damage`, or `share` of their max health.
    Bolt { target: EntityId, damage: f32, share: f32, school: u8 },
    /// Every enemy within its radius of the caster heals `share` of its max health (not the
    /// Mortally Wounded).
    Heal { share: f32 },
}
