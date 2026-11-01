//! Phases and waves: countdown, endless waves with steep scaling, intermissions, defeat.

use rand::Rng;

use super::*;
use crate::geom::{facing, path_len};

#[derive(Debug, Clone)]
pub(super) struct PendingSpawn {
    pub at: f32,
    pub kind: EnemyKind,
    pub elite: bool,
    pub lane: usize,
    /// The road it takes; `None`: its lane's roads in turn.
    pub route: Option<usize>,
}

/// A wave decided ahead of time (spawn times from the wave's start).
#[derive(Debug, Clone)]
pub(super) struct WavePlan {
    pub wave: u32,
    pub players: usize,
    pub spawns: Vec<PendingSpawn>,
    /// The boss leading it, if any.
    pub boss: Option<usize>,
    /// A wave with a twist of its own.
    pub event: Option<WaveEvent>,
}

/// Now and then a wave comes differently.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum WaveEvent {
    /// Twice as many, half as tough, no elites.
    Swarm,
    /// Few, and every one an elite.
    Vanguard,
}

impl WaveEvent {
    /// Waves 7, 17, 27...: the Swarm; 12, 22, 32...: the Vanguard (boss waves stay as they are).
    pub(super) fn of(n: u32) -> Option<WaveEvent> {
        match n % 10 {
            7 => Some(WaveEvent::Swarm),
            2 if n > 10 => Some(WaveEvent::Vanguard),
            _ => None,
        }
    }
    pub(super) fn name(self) -> &'static str {
        match self {
            WaveEvent::Swarm => "The Swarm",
            WaveEvent::Vanguard => "The Vanguard",
        }
    }
    pub(super) fn text(self) -> &'static str {
        match self {
            WaveEvent::Swarm => "twice as many, half as tough",
            WaveEvent::Vanguard => "few, and every one an elite",
        }
    }
    /// Enemy count, health, and spawn spacing.
    fn count(self) -> f32 {
        match self {
            WaveEvent::Swarm => 2.0,
            WaveEvent::Vanguard => 0.35,
        }
    }
    pub(super) fn health(self) -> f32 {
        match self {
            WaveEvent::Swarm => 0.5,
            WaveEvent::Vanguard => 1.0,
        }
    }
    fn spacing(self) -> f32 {
        match self {
            WaveEvent::Swarm => 0.6,
            WaveEvent::Vanguard => 2.0,
        }
    }
}

fn defenses_reset() -> super::defenses::Defenses {
    super::defenses::Defenses::default()
}

impl Sim {
    pub(super) fn step_phase(&mut self, dt: f32) {
        self.step_streams();
        // Between waves the next one is decided, and everyone told what's coming, and where.
        let between = matches!(self.phase, Phase::Countdown { .. } | Phase::Intermission { .. });
        if between && self.player_count() > 0 && self.ensure_plan() {
            if let Some(text) = self.preview_text() {
                self.announce(text, false);
            }
        }
        match &mut self.phase {
            Phase::Waiting => {}
            Phase::Countdown { left } | Phase::Intermission { left } => {
                let before = left.ceil() as u32;
                *left -= dt;
                let after = left.ceil().max(0.0) as u32;
                if *left <= 0.0 {
                    self.begin_wave();
                } else if before != after && [10, 5].contains(&after) {
                    let n = self.run.wave + 1;
                    self.announce(format!("Wave {n} in {after} seconds."), false);
                }
            }
            Phase::Wave { pending } => {
                let now = self.clock;
                let due: Vec<PendingSpawn> = pending.iter().filter(|p| p.at <= now).cloned().collect();
                pending.retain(|p| p.at > now);
                for p in due {
                    self.spawn_wave_enemy(p.kind, p.elite, p.lane, p.route);
                }
                let none_pending = matches!(&self.phase, Phase::Wave { pending } if pending.is_empty());
                let none_alive = !self
                    .entities
                    .values()
                    .any(|e| e.enemy().is_some_and(|en| en.guard.is_none() && !en.stream) && e.alive());
                if none_pending && none_alive {
                    self.end_wave();
                }
            }
            Phase::Over { left } => {
                *left -= dt;
                if *left <= 0.0 {
                    self.restart();
                }
            }
        }
    }

    /// The endless stream out of the bases that haven't been claimed.
    fn step_streams(&mut self) {
        let every = match self.phase {
            Phase::Wave { .. } => self.content.scaling.stream_secs,
            Phase::Intermission { .. } => self.content.scaling.stream_rest_secs,
            _ => 0.0,
        };
        if every <= 0.0 || self.run.wave == 0 || self.run.wave < self.content.scaling.stream_from_wave {
            return;
        }
        // While a boss leads a wave, the bases hold their breath: the fight is the boss's.
        let boss_wave = self.content.scaling.boss_every > 0 && self.run.wave % self.content.scaling.boss_every == 0;
        if boss_wave && matches!(self.phase, Phase::Wave { .. }) {
            return;
        }
        let lanes = self.content.lanes_for_wave(self.run.wave).min(self.arena.lanes.len()).max(1);
        if self.stream_next.len() < lanes {
            self.stream_next.resize(lanes, 0.0);
        }
        let alive = self
            .entities
            .values()
            .filter(|e| e.alive() && e.enemy().is_some_and(|en| en.stream))
            .count() as u32;
        let cap = (self.content.scaling.stream_cap as f32 * self.solo(self.content.scaling.solo_stream_cap)).round() as u32;
        if alive >= cap {
            return;
        }
        let now = self.clock;
        // More heroes, a thicker stream; one alone, a thinner one.
        let every = (2.0 / (1.0 + self.player_count().max(1) as f32)).sqrt() * every * self.solo(self.content.scaling.solo_stream_slower);
        // It thickens over the first waves: a trickle while heroes are green.
        let ramp = self.content.scaling.stream_ramp_waves as f32;
        let every = every * (1.0 + (ramp - self.run.wave as f32).max(0.0) / 3.0);
        for lane in 0..lanes {
            // A claimed base's stream stops.
            let claimed = self
                .bases
                .iter()
                .any(|b| b.state == BaseState::Claimed && self.arena.bases[b.site].lane == lane);
            if claimed {
                continue;
            }
            if self.stream_next[lane] == 0.0 {
                // Lanes don't all fire at once.
                self.stream_next[lane] = now + every * (0.5 + 0.5 * lane as f32 / lanes as f32);
                continue;
            }
            if now < self.stream_next[lane] {
                continue;
            }
            self.stream_next[lane] = now + every;
            let roster = self.lane_roster(lane, self.run.wave.max(1), true);
            let Some(pick) = self.roll_roster(&roster) else { continue };
            if let Some(id) = self.spawn_wave_enemy(EnemyKind { boss: false, index: pick }, false, lane, None) {
                if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
                    en.stream = true;
                }
            }
        }
    }

    /// The enemies a lane sends by wave `n`: its base's own kinds (and those from anywhere).
    /// `plain` leaves out the tougher ranks, for the stream.
    fn lane_roster(&self, lane: usize, n: u32, plain: bool) -> Vec<(usize, f32)> {
        let base = self.arena.bases.iter().find(|b| b.lane == lane).map(|b| b.name.as_str());
        let pick = |own_only: bool| -> Vec<(usize, f32)> {
            self.content
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.first_wave <= n && (!plain || e.rank == 0))
                .filter(|(_, e)| {
                    if e.bases.is_empty() {
                        !own_only
                    } else {
                        base.is_some_and(|b| e.bases.iter().any(|x| x == b))
                    }
                })
                .map(|(i, e)| (i, e.weight))
                .collect()
        };
        let mut roster = pick(false);
        if !roster.iter().any(|(i, _)| !self.content.enemies[*i].bases.is_empty()) {
            // No base of its own (or none of its kinds yet): anything unlocked.
            roster = self
                .content
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.first_wave <= n && (!plain || e.rank == 0))
                .map(|(i, e)| (i, e.weight))
                .collect();
        }
        roster
    }

    fn roll_roster(&mut self, roster: &[(usize, f32)]) -> Option<usize> {
        let total: f32 = roster.iter().map(|(_, w)| w).sum();
        if roster.is_empty() || total <= 0.0 {
            return None;
        }
        let mut roll = self.rng.gen::<f32>() * total;
        for (i, w) in roster {
            if roll < *w {
                return Some(*i);
            }
            roll -= w;
        }
        roster.last().map(|(i, _)| *i)
    }

    /// Enemy health multiplier for this wave and cycle.
    pub(super) fn health_mult(&self) -> f32 {
        self.content.scaling.health(self.run.wave.max(1), self.run.cycle) * self.solo(self.content.scaling.solo_health) * self.party(self.content.scaling.party_health)
    }

    pub(super) fn damage_mult(&self) -> f32 {
        self.content.scaling.damage(self.run.wave.max(1), self.run.cycle) * self.solo(self.content.scaling.solo_damage) * self.party(self.content.scaling.party_damage)
    }

    /// 1 + `per` for each hero past the first, growing in over the first eight waves: early on a
    /// party is as green as a lone hero, and only later does it outgrow the waves together.
    fn party(&self, per: f32) -> f32 {
        let ramp = (self.run.wave.saturating_sub(1) as f32 / 7.0).min(1.0);
        1.0 + per * ramp * self.player_count().saturating_sub(1) as f32
    }

    /// `v` for a hero playing alone, else 1.
    fn solo(&self, v: f32) -> f32 {
        if self.player_count() <= 1 {
            v
        } else {
            1.0
        }
    }

    /// The level enemies show this wave.
    pub(super) fn enemy_level(&self) -> u32 {
        let t = self.content.levels.target_level(self.run.wave.saturating_sub(1));
        (t + self.run.cycle * 2).clamp(1, 63)
    }

    /// What wave `n` will send, decided ahead so players can see it coming: each enemy with its
    /// delay from the wave's start, its lane and the road it takes.
    pub(super) fn plan_wave(&mut self, n: u32) -> WavePlan {
        let players = self.player_count().max(1);
        let s = self.content.scaling.clone();
        let event = if self.content.game.wave_events { WaveEvent::of(n) } else { None };
        let raw = s.raw_count(n, players) * event.map_or(1.0, |e| e.count());
        let count = (raw.round() as u32).clamp(1, s.count_cap) as usize;
        let lanes = self.content.lanes_for_wave(n).min(self.arena.lanes.len()).max(1);
        // Claimed bases thin out their lane.
        let mut lane_weight = vec![1.0f32; lanes];
        for b in &self.bases {
            if b.state == BaseState::Claimed {
                let lane = self.arena.bases[b.site].lane;
                if lane < lanes {
                    lane_weight[lane] *= 1.0 - self.content.bases.lane_cut;
                }
            }
        }
        let rosters: Vec<Vec<(usize, f32)>> = (0..lanes).map(|l| self.lane_roster(l, n, false)).collect();
        let elite_share = match event {
            Some(WaveEvent::Swarm) => 0.0,
            Some(WaveEvent::Vanguard) => 1.0,
            None => s.elite_share(n),
        };
        let mut spawns = Vec::with_capacity(count + 1);
        let mut per_lane = vec![0usize; lanes];
        // A party's waves come closer together, not just longer.
        let mut s = s;
        s.spawn_interval /= self.party(s.party_spawn);
        s.spawn_interval *= event.map_or(1.0, |e| e.spacing());
        let mut next_route = vec![0usize; lanes];
        let lane_total: f32 = lane_weight.iter().sum();
        for _ in 0..count {
            // A lane, weighted.
            let mut roll = self.rng.gen::<f32>() * lane_total;
            let mut lane = 0;
            for (i, w) in lane_weight.iter().enumerate() {
                if roll < *w {
                    lane = i;
                    break;
                }
                roll -= w;
            }
            let Some(pick) = self.roll_roster(&rosters[lane]) else { continue };
            let elite = self.rng.gen::<f32>() < elite_share;
            let kind = EnemyKind { boss: false, index: pick };
            // Its base's roads in turn; a flanker takes the long way round.
            let routes = self.arena.lanes[lane].routes.len().max(1);
            let route = if self.content.enemies[pick].role == "flanker" {
                self.longest_route(lane)
            } else {
                let r = next_route[lane] % routes;
                next_route[lane] += 1;
                r
            };
            spawns.push(PendingSpawn { at: 1.0 + per_lane[lane] as f32 * s.spawn_interval, kind, elite, lane, route: Some(route) });
            per_lane[lane] += 1;
        }
        let mut boss = None;
        if s.boss_every > 0 && n % s.boss_every == 0 && !self.content.bosses.is_empty() {
            let index = ((n / s.boss_every) as usize - 1) % self.content.bosses.len();
            let lane = self.rng.gen_range(0..lanes);
            let route = self.rng.gen_range(0..self.arena.lanes[lane].routes.len().max(1));
            spawns.push(PendingSpawn {
                at: 1.0 + per_lane[lane] as f32 * s.spawn_interval * 0.5,
                kind: EnemyKind { boss: true, index },
                elite: false,
                lane,
                route: Some(route),
            });
            boss = Some(index);
        }
        WavePlan { wave: n, players, spawns, boss, event }
    }

    /// The plan for the next wave, made now if there isn't a fresh one (true when it was).
    pub(super) fn ensure_plan(&mut self) -> bool {
        let n = self.run.wave + 1;
        let players = self.player_count().max(1);
        let fresh = self.next_plan.as_ref().is_some_and(|p| p.wave == n && p.players == players);
        if !fresh {
            let plan = self.plan_wave(n);
            self.next_plan = Some(plan);
        }
        !fresh
    }

    /// The road of `lane` that's longest.
    fn longest_route(&self, lane: usize) -> usize {
        let routes = &self.arena.lanes[lane].routes;
        (0..routes.len())
            .max_by(|&a, &b| path_len(&routes[a]).total_cmp(&path_len(&routes[b])))
            .unwrap_or(0)
    }

    pub(super) fn begin_wave(&mut self) {
        self.ensure_plan();
        let plan = self.next_plan.take().expect("a plan");
        self.run.wave += 1;
        let n = self.run.wave;
        let now = self.clock;
        let pending: Vec<PendingSpawn> = plan.spawns.iter().map(|p| PendingSpawn { at: now + p.at, ..p.clone() }).collect();
        let boss_name = plan.boss.map(|i| self.content.bosses[i].name.clone());
        self.run.event = plan.event;
        let lanes = self.content.lanes_for_wave(n).min(self.arena.lanes.len()).max(1);
        // Experience and gold for the wave, shared out by each enemy's weight.
        let weight: f32 = pending.iter().map(|p| self.xp_weight(p.kind, p.elite)).sum::<f32>().max(1.0);
        // Clearing the wave is worth a level to a hero on the curve (kills beyond that wait at the
        // cap; the stream and the raised dead add to the same bar).
        let l = &self.content.levels;
        // (A tenth spare, for the stragglers a hero doesn't see die.)
        let wave_xp = (l.xp_to_next(l.target_level(n - 1)) * l.levels_per_wave) as f32 * 1.1;
        self.run.xp_per_weight = wave_xp / weight;
        self.run.gold_per_weight = self.content.game.gold_per_kill
            * (1.0 + self.content.game.gold_per_kill_per_wave * (n - 1) as f32 * 0.2);
        self.phase = Phase::Wave { pending };
        for pid in self.player_ids() {
            self.fire(pid, crate::kit::On::Wave, None, None, 0.0);
        }
        let from: Vec<&str> = self.arena.lanes[..lanes].iter().map(|l| l.name.as_str()).collect();
        let mut text = format!("Wave {n}: enemies from {}!", join_names(&from));
        if let Some(e) = plan.event {
            text = format!("Wave {n}, {}: {}, from {}!", e.name(), e.text(), join_names(&from));
        }
        if self.run.cycle > 0 {
            text = format!("Wave {n} (Escalation {}): enemies from {}!", self.run.cycle, join_names(&from));
        }
        if let Some(boss) = &boss_name {
            text.push_str(&format!(" {boss} leads them."));
        }
        self.announce(text, true);
        let from_text = format!("Enemies from {}", join_names(&self.arena.lanes[..lanes].iter().map(|l| l.name.as_str()).collect::<Vec<_>>()));
        let last = !self.run.won && n == self.content.game.victory_wave;
        let title = if let (false, Some(e)) = (last, plan.event) {
            format!("Wave {n} · {}", e.name())
        } else if last {
            format!("Final Wave {n}")
        } else if self.run.cycle > 0 {
            format!("Wave {n} · Escalation {}", self.run.cycle)
        } else {
            format!("Wave {n}")
        };
        match boss_name {
            Some(boss) => self.splash(Splash::Boss, title, format!("{boss} leads them!")),
            None => self.splash(Splash::Wave, title, from_text),
        }
        // The hub toughens up with the waves.
        let grow = 1.0 + self.content.hub.health_per_wave;
        let hub_id = self.hub;
        let hub = self.entities.get_mut(&hub_id).expect("hub");
        let ratio = hub.health as f32 / hub.max_health.max(1) as f32;
        hub.max_health = (hub.max_health as f32 * grow).round() as u32;
        hub.health = (hub.max_health as f32 * ratio).round() as u32;
        let (health, max_health) = (hub.health, hub.max_health);
        self.events.push(Event::Health { id: hub_id, health, max_health });
        self.refresh_defenses(true);
        self.rescale_guardians();
        if n == self.content.bases.open_from_wave && self.run.cycle == 0 {
            self.open_bases();
        }
        self.run.defence_meter = [0; 2];
        self.run.spawned = 0;
        self.run.spawned_health = 0;
        self.run.spawned_hit = 0.0;
        self.run.hub_low = 1.0;
        for id in self.player_ids() {
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.undying_used = false;
                // The party's meter counts this wave afresh.
                p.meter.wave = 0;
                p.meter.taken = 0;
                p.meter.hits = 0;
                p.meter.biggest = (0, String::new());
                p.meter.fell = 0;
            }
        }
    }

    pub(super) fn xp_weight(&self, kind: EnemyKind, elite: bool) -> f32 {
        let def = self.enemy_def(kind);
        def.xp * if elite { 3.0 } else { 1.0 }
    }

    fn end_wave(&mut self) {
        self.run.event = None;
        let n = self.run.wave;
        self.run.depth = self.run.depth.max(n);
        let boss_wave = self.content.scaling.boss_every > 0 && n % self.content.scaling.boss_every == 0;
        let rest = if boss_wave {
            self.content.game.boss_intermission_secs
        } else {
            self.content.game.intermission_secs
        };
        self.phase = Phase::Intermission { left: rest };
        // Everyone gets back up and is healed; the hub is patched up.
        self.revive_and_heal_players();
        let repair = self.content.game.hub_repair_per_wave;
        let hub_id = self.hub;
        let (h, m) = {
            let hub = &self.entities[&hub_id];
            (hub.health, hub.max_health)
        };
        self.set_health(hub_id, h + (m as f32 * repair) as u32);
        let hub = &self.entities[&hub_id];
        if hub.health * 100 / hub.max_health.max(1) > 50 {
            self.hub_warned = 0;
        }
        self.announce(
            format!("Wave {n} held! Wave {} in {} seconds.", n + 1, rest as u32),
            true,
        );
        self.balance_report(n);
        // The Hero tab's wares turn over.
        for id in self.player_ids() {
            self.restock(id);
        }
        // The victory wave gets its own banner instead.
        if self.run.won || n < self.content.game.victory_wave {
            self.splash(
                Splash::Cleared,
                format!("Wave {n} Cleared"),
                format!("Wave {} in {} seconds", n + 1, rest as u32),
            );
        }
        self.show_waiting_offers();
        self.wave_rewards(boss_wave);
        if !self.run.won && n >= self.content.game.victory_wave {
            self.win();
        }
        let bonus = (self.content.game.wave_gold + self.content.game.wave_gold_per_wave * n as f32).round() as u32;
        if bonus > 0 {
            for id in self.player_ids() {
                self.earn_gold(id, bonus);
            }
            self.announce(format!("Wave {n} bounty: {bonus} gold each."), false);
        }
    }

    /// The victory wave is cleared: the run is won. The waves go on for those who want more.
    fn win(&mut self) {
        self.run.won = true;
        let n = self.run.wave;
        let mins = ((self.clock - self.run.started).max(0.0) / 60.0) as u32;
        self.splash(Splash::Victory, "Victory!", format!("The Basin holds after {n} waves. The endless waves begin: how far can you go?"));
        self.announce(
            format!("VICTORY! The {} stands after {n} waves ({mins} minutes). The waves go on: every one from here adds to your record.", self.content.hub.name),
            true,
        );
        let hub = self.hub;
        self.events.push(Event::SpellGo { caster: hub, spell: 11541, target: SpellTarget::Unit(hub), hits: vec![hub] });
        let result = RunResult {
            depth: self.run.depth,
            won: true,
            cycles: self.run.cycle,
            secs: (self.clock - self.run.started).max(0.0) as u32,
            players: self.player_results(),
        };
        self.events.push(Event::Victory(result));
    }

    /// The run is won (the victory wave cleared).
    pub fn won(&self) -> bool {
        self.run.won
    }

    pub(super) fn step_outcome(&mut self) {
        if !matches!(self.phase, Phase::Wave { .. } | Phase::Intermission { .. }) {
            return;
        }
        // Only the Ancient falling ends a run: fallen heroes come back after a short wait.
        let hub_down = !self.entities[&self.hub].alive();
        if hub_down {
            self.phase = Phase::Over { left: 20.0 };
            let why = format!("The {} has fallen.", self.content.hub.name);
            let depth = self.run.depth;
            let cycles = self.run.cycle;
            self.announce(
                format!(
                    "Defeat on wave {}. {why} Depth reached: wave {depth}{}. A new run starts shortly.",
                    self.run.wave,
                    if cycles > 0 { format!(", Escalation {cycles}") } else { String::new() }
                ),
                true,
            );
            let result = RunResult {
                depth,
                won: self.run.won,
                cycles,
                secs: (self.clock - self.run.started).max(0.0) as u32,
                players: self.player_results(),
            };
            self.splash(
                Splash::Defeat,
                "Defeat",
                format!(
                    "Depth reached: wave {depth}{}",
                    if cycles > 0 { format!(", Escalation {cycles}") } else { String::new() }
                ),
            );
            self.events.push(Event::RunOver(result));
            // Enemies stop where they are.
            let ids: Vec<EntityId> = self.entities.keys().copied().collect();
            for id in ids {
                self.halt_enemy(id);
            }
        }
    }

    pub(super) fn reset(&mut self) {
        // A new run: whoever left the last one starts fresh.
        self.departed.clear();
        let doomed: Vec<EntityId> = self
            .entities
            .iter()
            // (The shopkeeper stays.)
            .filter(|(_, e)| matches!(e.body, Body::Enemy(_) | Body::Npc { .. } | Body::Ally(_)) && !matches!(e.body, Body::Npc { npc: NpcKind::Vendor, .. }))
            .map(|(&id, _)| id)
            .collect();
        for id in doomed {
            self.entities.remove(&id);
            self.events.push(Event::Despawn { id });
        }
        let zones: Vec<EntityId> = self
            .entities
            .iter()
            .filter(|(_, e)| matches!(e.body, Body::Zone(_)))
            .map(|(&id, _)| id)
            .collect();
        for id in zones {
            self.entities.remove(&id);
            self.events.push(Event::ZoneEnd { id });
        }
        self.loot.clear();
        self.hits.clear();
        self.landings.clear();
        self.stream_next.clear();
        self.defenses = defenses_reset();
        self.reset_sprints();
        self.spawn_posts();
        self.spawn_vendor(true);
        self.next_plan = None;
        let hub_id = self.hub;
        let base_health = self.content.hub.health;
        let hub = self.entities.get_mut(&hub_id).expect("hub");
        let was_dead = !hub.alive();
        hub.max_health = base_health;
        hub.health = base_health;
        self.events.push(if was_dead {
            Event::Revived { id: hub_id, health: base_health, max_health: base_health }
        } else {
            Event::Health { id: hub_id, health: base_health, max_health: base_health }
        });
        self.run = Run { started: self.clock, ..Run::default() };
        self.hub_warned = 0;
        self.bases = (0..self.arena.bases.len()).map(bases::Base::new).collect();
        for id in self.player_ids() {
            self.stop_attack(id);
            self.reset_player(id);
        }
    }

    /// A wave enemy appears at the start of its lane.
    fn spawn_wave_enemy(&mut self, kind: EnemyKind, elite: bool, lane: usize, route: Option<usize>) -> Option<EntityId> {
        // Saboteurs are news: the first of a wave is called out, from the lane it comes down.
        if !kind.boss && self.content.enemies[kind.index].role == "sapper" && self.run.sapper_warned != self.run.wave {
            self.run.sapper_warned = self.run.wave;
            let name = self.content.enemies[kind.index].name.clone();
            let from = self.arena.lanes[lane].name.clone();
            self.announce(format!("{name}s are running for the {} from {from}: stop them before they reach it!", self.content.hub.name), true);
        }
        // A base's enemies take its roads in turn, so a wave splits between them.
        let routes = self.arena.lanes[lane].routes.len().max(1);
        if self.route_next.len() <= lane {
            self.route_next.resize(lane + 1, 0);
        }
        let route = match route {
            Some(r) => r % routes,
            None => {
                let r = self.route_next[lane] % routes;
                self.route_next[lane] = self.route_next[lane].wrapping_add(1);
                r
            }
        };
        let lane_points = match self.arena.lanes[lane].routes.get(route) {
            Some(r) if r.len() >= 2 => r.clone(),
            _ => self.arena.lanes[lane].points.clone(),
        };
        // Spread spawns a little so a column doesn't walk in single file.
        let jitter = [self.rng.gen_range(-2.5..2.5), self.rng.gen_range(-2.5..2.5)];
        let mut path: Vec<Vec3> = lane_points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i + 1 == lane_points.len() {
                    *p
                } else {
                    let (x, y) = (p[0] + jitter[0], p[1] + jitter[1]);
                    [x, y, self.ground.height(x, y)]
                }
            })
            .collect();
        let id = self.alloc();
        // The lane ends at the hub's edge; each enemy takes its own spot along that edge.
        if let Some(last) = path.last_mut() {
            *last = self.hub_slot(*last, slot_angle(id));
        }
        let start = path[0];
        let start_facing = if path.len() > 1 { facing(path[0], path[1]) } else { 0.0 };
        let players = self.player_count().max(1);
        let s = &self.content.scaling;
        let raw = s.raw_count(self.run.wave, players);
        let overflow = (raw / s.count_cap as f32).max(1.0);
        let mut health_mult = self.health_mult() * overflow;
        if !kind.boss {
            health_mult *= self.run.event.map_or(1.0, |e| e.health());
        }
        let mut damage_mult = self.damage_mult();
        if elite {
            health_mult *= s.elite_health;
            damage_mult *= s.elite_damage;
        }
        if kind.boss {
            health_mult *= s.boss_health * (1.0 + s.boss_health_per_extra_player * (players - 1) as f32);
            // Each round of bosses comes back tougher.
            let round = (self.run.wave / s.boss_every.max(1)).saturating_sub(1) as usize / self.content.bosses.len().max(1);
            health_mult *= 1.0 + round as f32 * 0.5;
        }
        self.spawn_enemy(id, kind, elite, None, start, start_facing, health_mult, damage_mult);
        // The long way round is walked faster, up to twice as fast.
        let shortest = self
            .arena
            .lanes
            .iter()
            .flat_map(|l| l.routes.iter())
            .map(|r| path_len(r))
            .fold(f32::MAX, f32::min);
        let haste = (path_len(&lane_points) / shortest.max(1.0)).clamp(1.0, 2.0);
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.road_haste = haste;
        }
        let front = self.route_front(lane, route);
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.front = front;
        }
        let speed = self.enemy_speed(id);
        self.set_path(id, path, speed);
        Some(id)
    }

    /// A spot at the hub's edge near `near`, nudged round by the enemy's own angle.
    pub(super) fn hub_slot(&self, near: Vec3, slot_angle: f32) -> Vec3 {
        let hub = self.arena.hub;
        let r = (self.content.hub.reach - 1.5).max(1.0);
        let base = facing(hub, near);
        let a = base + (slot_angle - std::f32::consts::PI) * 0.35;
        self.ring(hub, r, a)
    }
}

#[cfg(test)]
impl Sim {
    /// Spawn content enemy `key` at `pos`, marching on the hub.
    pub fn spawn_for_test(&mut self, key: &str, pos: Vec3) -> EntityId {
        let kind = match self.content.enemies.iter().position(|e| e.key == key) {
            Some(index) => EnemyKind { boss: false, index },
            None => EnemyKind { boss: true, index: self.content.bosses.iter().position(|e| e.key == key).expect("an enemy with that key") },
        };
        let id = self.alloc();
        self.spawn_enemy(id, kind, false, None, pos, 0.0, 1.0, 1.0);
        let path = crate::geom::ground_path(&*self.ground, pos, self.arena.hub, PATH_STEP);
        let speed = self.enemy_speed(id);
        self.set_path(id, path, speed);
        id
    }
    /// Give an enemy an elite's twist.
    pub fn twist_for_test(&mut self, id: EntityId, modifier: u8) {
        if let Some(en) = self.entities.get_mut(&id).and_then(|e| e.enemy_mut()) {
            en.elite = true;
            en.modifier = modifier;
        }
    }
    /// Every pending spawn dropped and every enemy killed: the wave ends on the next step.
    pub fn clear_wave_for_test(&mut self) {
        if let Phase::Wave { pending } = &mut self.phase {
            pending.clear();
        }
        let ids: Vec<EntityId> = self.entities.iter().filter(|(_, e)| e.enemy().is_some() && e.alive()).map(|(&id, _)| id).collect();
        let hub = self.hub;
        for id in ids {
            if let Some(e) = self.entities.get_mut(&id) {
                e.health = 0;
            }
            self.kill(id, hub);
        }
    }
    pub fn health_for_test(&self, id: EntityId) -> (u32, u32) {
        self.entities.get(&id).map_or((0, 0), |e| (e.health, e.max_health))
    }
    pub fn set_health_for_test(&mut self, id: EntityId, health: u32) {
        if let Some(e) = self.entities.get_mut(&id) {
            e.health = health;
        }
    }
    /// (boss, elite) for each spawn of wave `n`'s plan.
    pub fn plan_kinds_for_test(&mut self, n: u32) -> Vec<(bool, bool)> {
        self.plan_wave(n).spawns.iter().map(|s| (s.kind.boss, s.elite)).collect()
    }
    pub fn plan_for_test(&mut self, n: u32) -> Vec<(String, usize, usize)> {
        self.plan_wave(n)
            .spawns
            .iter()
            .map(|s| (self.enemy_def(s.kind).key.clone(), s.lane, s.route.unwrap_or(0)))
            .collect()
    }
}

#[cfg(test)]
impl Sim {
    /// A player's hit (what spell damage multiplies) and the health of a normal enemy now.
    pub fn power_for_test(&self, id: EntityId) -> (f32, f32) {
        let reference = self.content.defenses.reference_health * self.health_mult();
        (self.player_hit(id), reference)
    }
}

impl Sim {
    /// The wave's balance in the server log: how hard the enemies came and how hard they hit each
    /// hero, against that hero's health and armour.
    fn balance_report(&mut self, n: u32) {
        let r = &self.run;
        let level = self.enemy_level();
        let count = r.spawned.max(1);
        let mut lines = vec![format!(
            "Wave {n} (escalation {}): {} enemies at level {level}, average health {}, average swing {:.0} before armour (x{:.2} on heroes); the Ancient fell to {:.0}%",
            r.cycle,
            r.spawned,
            r.spawned_health / u64::from(count),
            r.spawned_hit / count as f32,
            {
                let sc = &self.content.scaling;
                sc.hero_damage + sc.hero_damage_per_wave * n.saturating_sub(1) as f32
            },
            r.hub_low * 100.0,
        )];
        let defences = r.defence_meter;
        for id in self.player_ids() {
            let Some(e) = self.entities.get(&id) else { continue };
            let Some(p) = e.player() else { continue };
            let m = &p.meter;
            let avg = if m.hits > 0 { m.taken / u64::from(m.hits) } else { 0 };
            let share = |d: u64| d as f32 * 100.0 / e.max_health.max(1) as f32;
            lines.push(format!(
                "  {} ({} {}, {} health, {} armour = {:.0}% off): took {} in {} hits, {} a hit ({:.1}% of health), biggest {} ({:.0}%) from {}; fell {}; dealt {}",
                p.name,
                self.content.classes[p.class].key,
                p.level,
                e.max_health,
                p.stats.armor,
                self.armor_reduction(p.stats.armor as f32) * 100.0,
                m.taken,
                m.hits,
                avg,
                share(avg),
                m.biggest.0,
                share(u64::from(m.biggest.0)),
                if m.biggest.1.is_empty() { "nothing" } else { &m.biggest.1 },
                m.fell,
                m.wave,
            ));
        }
        lines.push(format!("  Base defences dealt {}, soldiers {}", defences[0], defences[1]));
        for l in lines {
            self.events.push(Event::Log(l));
        }
    }
}

