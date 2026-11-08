//! Sneed's machine builds pressure through completed strings, grinds in a committed lane,
//! then stalls. Its driver gets one repair attempt; damage to him is never healed by it.

use super::mobs::{Foe, MobState};
use super::*;

const MACHINE: &str = "dm_shredder";
const DRIVER: &str = "dm_sneed";
const GRIND: f32 = 0.9;
const DISTANCE: f32 = 6.0;
const HALF_WIDTH: f32 = 1.5;
const STALL: f32 = 3.5;
const REPAIR: f32 = 4.0;
const SEEK_LIMIT: f32 = 6.0;
// Existing smoke puff (Vanish), used only as a visual. No concealment or damage.
const SMOKE: u32 = 1856;
// Mining's working animation, already used by gatherers.
const WORK: u32 = 2575;

#[derive(Default)]
pub(crate) struct Machinery {
    pressure: BTreeMap<Id, u8>,
    steam_at: BTreeMap<Id, f32>,
    grinds: BTreeMap<Id, Grind>,
    stalls: BTreeMap<Id, f32>,
    wrenches: BTreeMap<Id, Vec3>,
    drivers: BTreeMap<Id, Driver>,
    /// A repaired machine carries its driver's remaining health, never a fresh driver.
    rebuilt: BTreeMap<Id, u32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::land::{FlatLand, Land};

    const DT: f32 = 0.05;

    fn fight(heroes: usize, seed: u64) -> (Survival, Vec<Id>, Id) {
        let mut c = Content::dev();
        c.pvp = Default::default();
        let d = c.dungeon.as_mut().unwrap();
        d.packs.retain(|p| p.key == "sneed");
        d.packs[0].at = [0.0, 0.0];
        d.packs[0].z = Some(0.0);
        d.lamps[0].at = [0.0, -20.0, 0.0];
        let mut s = Survival::new_dungeon(c, Box::new(FlatLand::default()), seed, 1_000_000.0);
        let machine = live(&s, MACHINE).unwrap();
        let mut band = Vec::new();
        for n in 0..heroes {
            let h = s.join(&format!("hero{n}"), &format!("Hero{n}"), None);
            s.move_player(h, [2.5, n as f32 * 2.0, 0.0], std::f32::consts::PI, Moving::default());
            let p = s.players.get_mut(&h).unwrap();
            p.max_health = 100_000;
            p.health = 100_000;
            band.push(h);
        }
        s.engage(machine, Foe::Hero(band[0]));
        (s, band, machine)
    }

    fn live(s: &Survival, key: &str) -> Option<Id> {
        s.mobs.iter().find(|(_, m)| m.health > 0 && s.content.mobs[m.def].key == key).map(|(id, _)| *id)
    }

    fn run(s: &mut Survival, secs: f32) -> Vec<Event> {
        (0..(secs / DT).round() as usize).flat_map(|_| s.step(DT)).collect()
    }

    fn working(s: &mut Survival, driver: Id) -> f32 {
        for _ in 0..600 {
            let ev = s.step(DT);
            if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: WORK, ms: 4000 } if *caster == driver)) {
                return s.clock;
            }
        }
        panic!("Sneed did not reach his wreck and start working");
    }

    #[test]
    fn shredder_executes_pressure_cycle_at_full_and_low_health_then_grinds_a_fixed_safe_lane() {
        for (heroes, seed, low) in [(1, 2, false), (2, 7, false), (2, 19, true)] {
            let (mut s, band, id) = fight(heroes, seed);
            if low { s.mobs.get_mut(&id).unwrap().health = s.mobs[&id].max_health / 4; }
            let mut moves = Vec::new();
            let mut origin = None;
            for _ in 0..800 {
                let ev = s.step(DT);
                if ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)) {
                    let m = &s.mobs[&id];
                    let (k, step) = m.pattern.unwrap();
                    let name = s.content.mobs[m.def].patterns[k].name.clone();
                    if step == 0 { moves.push(name.clone()); }
                    if name == "grind" {
                        assert_eq!(s.machinery.pressure[&id], 2);
                        origin = Some((m.pos, m.facing));
                        break;
                    }
                }
            }
            assert_eq!(moves, ["saws", "stamp and rake", "grind"]);
            let (at, dir) = origin.expect("the actual AI reached its grind");
            let point = |ahead: f32, side: f32| [at[0] + dir.cos() * ahead - dir.sin() * side, at[1] + dir.sin() * ahead + dir.cos() * side, at[2]];
            // The hero changes sides as the rev begins; the aim must not follow them.
            s.move_player(band[0], point(3.0, 4.0), 0.0, Moving::default());
            if heroes == 2 { s.move_player(band[1], point(4.0, 0.0), 0.0, Moving::default()); }
            let before = s.players[&band[0]].health;
            let ev = run(&mut s, 2.4);
            assert_eq!(s.players[&band[0]].health, before, "a basic sidestep is a safe answer");
            assert!(dist2d(s.mobs[&id].pos, point(DISTANCE, 0.0)) < 0.2, "a straight six-yard path");
            if heroes == 2 {
                assert_eq!(ev.iter().filter(|e| matches!(e, Event::Swing { attacker, victim: Some(v), damage, anim: false, .. } if *attacker == id && *v == band[1] && *damage > 0)).count(), 1, "one hit as the saw passes, no repeat tick");
            }
            assert_eq!(s.machinery.pressure[&id], 0);
            assert!(ev.iter().any(|e| matches!(e, Event::Yell { id: who, text } if *who == id && text.contains("heap of junk"))));
            assert!(matches!(s.mobs[&id].state, MobState::Recover { until, .. } if until - s.clock > 3.2));
            let ev = run(&mut s, 3.0);
            assert!(!ev.iter().any(|e| matches!(e, Event::MobWindup { id: who, .. } if *who == id)), "stall is a real punish window");
            assert!(!s.mobs[&id].enraged && s.mobs[&id].phase == 0);
        }
    }

    #[test]
    fn shredder_interrupted_string_does_not_build_pressure() {
        let (mut s, _, id) = fight(1, 4);
        for _ in 0..100 {
            s.step(DT);
            if matches!(s.mobs[&id].state, MobState::Windup { .. }) { break; }
        }
        assert!(matches!(s.mobs[&id].state, MobState::Windup { .. }));
        s.stagger_mob(id, 2.0, false);
        run(&mut s, 1.0);
        assert_eq!(s.machinery.pressure.get(&id).copied().unwrap_or(0), 0);
    }

    #[test]
    fn shredder_repair_takes_four_seconds_preserves_driver_health_and_cannot_repeat() {
        for heroes in [1, 2] {
            let (mut s, band, machine) = fight(heroes, 13);
            s.hurt_mob(machine, band[0], u32::MAX);
            let driver = live(&s, DRIVER).unwrap();
            assert!(!s.slain("sneed"));
            s.hurt_mob(driver, band[0], 20);
            let driver_hp = s.mobs[&driver].health;
            let began = working(&mut s, driver);
            assert!(s.mobs.contains_key(&machine), "wreck stays for the repair");
            run(&mut s, 3.85);
            assert!(live(&s, MACHINE).is_none(), "no early rebuild");
            assert!(!s.slain("sneed"));
            run(&mut s, 0.25);
            let back = live(&s, MACHINE).expect("repair succeeds");
            assert!(s.clock - began >= REPAIR);
            assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health / 3);
            assert!(!s.mobs.contains_key(&driver) && !s.mobs.contains_key(&machine));
            assert!(!s.slain("sneed"));
            s.hurt_mob(back, band[0], u32::MAX);
            let final_driver = live(&s, DRIVER).unwrap();
            assert_eq!(s.mobs[&final_driver].health, driver_hp);
            let ev = run(&mut s, 20.0);
            assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
            assert!(live(&s, MACHINE).is_none());
            s.hurt_mob(final_driver, band[0], u32::MAX);
            assert!(s.slain("sneed"));
            assert_eq!(s.camp.counts.get("kill:dm_sneed"), Some(&1));
            run(&mut s, 5.0);
            assert!(!s.mobs.values().any(|m| matches!(s.content.mobs[m.def].key.as_str(), MACHINE | DRIVER)), "no ghost body");
            assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty());
        }
    }

    #[test]
    fn shredder_repair_is_interrupted_by_an_ordinary_melee_swing_without_a_loadout() {
        let (mut s, band, machine) = fight(1, 17);
        let h = band[0];
        s.players.get_mut(&h).unwrap().loadout.clear();
        s.hurt_mob(machine, h, u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        working(&mut s, driver);
        let pos = s.mobs[&driver].pos;
        s.move_player(h, [pos[0] + 1.2, pos[1], pos[2]], std::f32::consts::PI, Moving::default());
        let hp = s.mobs[&driver].health;
        s.input(h, Input::Attack);
        run(&mut s, 1.4);
        assert!(s.mobs[&driver].health < hp, "the ordinary swing landed");
        assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
        assert!(!s.mobs.contains_key(&machine));
        let ev = run(&mut s, 18.0);
        assert!(!ev.iter().any(|e| matches!(e, Event::Cast { spell: WORK, .. })));
        assert!(live(&s, MACHINE).is_none() && !s.slain("sneed"));
    }

    #[test]
    fn shredder_repair_stagger_aborts_and_wipe_restores_exactly_one_whole_machine() {
        let (mut s, band, machine) = fight(1, 6);
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        working(&mut s, driver);
        s.stagger_mob(driver, 1.0, false);
        s.step(DT);
        assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
        s.hurt_player_by(band[0], u32::MAX, driver, "test wipe");
        s.step(DT);
        assert!(live(&s, DRIVER).is_none());
        let back = live(&s, MACHINE).unwrap();
        assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health);
        assert_eq!(s.mobs.values().filter(|m| s.content.mobs[m.def].key == MACHINE).count(), 1);
        assert!(s.machinery.drivers.is_empty() && s.machinery.rebuilt.is_empty());
        assert!(!s.slain("sneed"));
    }

    struct NoRoute;
    impl Land for NoRoute {
        fn height(&self, _: f32, _: f32) -> f32 { 0.0 }
        fn plans(&self) -> bool { true }
        fn walk_clear(&self, _: Vec3, _: Vec3) -> bool { false }
    }

    #[test]
    fn shredder_unreachable_wreck_has_a_bounded_attempt_and_no_teleport() {
        let (mut s, band, machine) = fight(1, 8);
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        let away = [8.0, 0.0, 0.0];
        s.mobs.get_mut(&driver).unwrap().pos = away;
        s.mobs.get_mut(&driver).unwrap().path.clear();
        s.machinery.drivers.get_mut(&driver).unwrap().stage = RepairStage::Seeking { until: s.clock + SEEK_LIMIT, repath_at: s.clock };
        s.land = Box::new(NoRoute);
        run(&mut s, SEEK_LIMIT + 0.1);
        assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Spent));
        assert!(dist2d(s.mobs[&driver].pos, away) < 0.1);
        assert!(!s.mobs.contains_key(&machine));
        assert!(live(&s, MACHINE).is_none());
    }

    #[test]
    fn shredder_stalls_at_a_wall_and_a_posture_break_cannot_shorten_the_opening() {
        let (mut s, band, id) = fight(1, 22);
        s.land = Box::new(FlatLand { walls: vec![([4.0, 0.0], 0.7)], ..Default::default() });
        let k = s.content.mobs[s.mobs[&id].def].patterns.iter().position(|p| p.name == "grind").unwrap();
        s.begin_pattern(id, k, false);
        s.windup(id, Foe::Hero(band[0]), [8.0, 0.0, 0.0]);
        s.move_player(band[0], [4.8, 0.0, 0.0], 0.0, Moving::default());
        let hp = s.players[&band[0]].health;
        run(&mut s, 2.4);
        assert!(s.mobs[&id].pos[0] <= 3.3, "saw stops before the wall");
        assert_eq!(s.players[&band[0]].health, hp, "no hit through the wall");
        let until = s.machinery.stalls[&id];
        s.stagger_mob(id, 0.6, false);
        run(&mut s, 0.8);
        assert!(matches!(s.mobs[&id].state, MobState::Recover { until: end, .. } if end >= until));
    }

    #[test]
    fn shredder_zero_damage_does_not_cancel_work_but_a_departed_party_resets_it() {
        let (mut s, band, machine) = fight(1, 21);
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        working(&mut s, driver);
        s.hurt_mob(driver, band[0], 0);
        assert!(matches!(s.machinery.drivers[&driver].stage, RepairStage::Working { .. }));
        s.move_player(band[0], [500.0, 0.0, 0.0], 0.0, Moving::default());
        run(&mut s, 0.1);
        assert!(!s.mobs.contains_key(&driver) && !s.mobs.contains_key(&machine));
        let back = live(&s, MACHINE).unwrap();
        assert_eq!(s.mobs[&back].health, s.mobs[&back].max_health);
        assert!(s.machinery.drivers.is_empty() && !s.slain("sneed"));
    }

    #[test]
    fn shredder_driver_commits_his_wrench_before_retreating() {
        let (mut s, band, machine) = fight(1, 31);
        s.hurt_mob(machine, band[0], u32::MAX);
        let driver = live(&s, DRIVER).unwrap();
        s.move_player(band[0], [8.0, 0.0, 0.0], 0.0, Moving::default());
        let mut wound = false;
        for _ in 0..100 {
            let ev = s.step(DT);
            if ev.iter().any(|e| matches!(e, Event::Cast { caster, spell: 13398, .. } if *caster == driver)) {
                wound = true;
                break;
            }
        }
        assert!(wound, "the AI chose its throw at range");
        let at = s.mobs[&driver].pos;
        s.move_player(band[0], [8.0, 3.0, 0.0], 0.0, Moving::default());
        let hp = s.players[&band[0]].health;
        let ev = run(&mut s, 2.0);
        assert_eq!(s.players[&band[0]].health, hp, "a sidestep during the tell evades the wrench");
        assert!(ev.iter().any(|e| matches!(e, Event::Missile { caster, spell: 13398, target: Target::Point(p) } if *caster == driver && dist2d(*p, [8.0, 0.0, 0.0]) < 0.1)));
        assert!(dist2d(s.mobs[&driver].pos, at) > 3.0, "the throw covers his retreat");
    }
}

#[derive(Clone)]
struct Grind {
    last: Vec3,
    facing: f32,
    until: f32,
    damage: u32,
    hit: BTreeSet<Id>,
}

#[derive(Clone)]
struct Driver {
    wreck: Option<Id>,
    machine: usize,
    max_health: u32,
    stage: RepairStage,
}

#[derive(Clone, Copy)]
enum RepairStage {
    Fighting { after: f32 },
    Seeking { until: f32, repath_at: f32 },
    Working { until: f32, spark_at: f32 },
    Spent,
}

impl Survival {
    pub(crate) fn is_machine(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == MACHINE)
    }

    pub(crate) fn machine_grind(&self, id: Id) -> bool {
        self.is_machine(id) && self.mobs[&id].pattern.is_some_and(|(k, _)| self.content.mobs[self.mobs[&id].def].patterns[k].name == "grind")
    }

    /// The first two completed strings build heat. The third spends it, at any health.
    pub(crate) fn machine_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let heat = self.machinery.pressure.get(&id).copied().unwrap_or(0);
        let name = match heat { 0 => "saws", 1 => "stamp and rake", _ => "grind" };
        d.patterns.iter().position(|p| p.name == name && dist <= if p.max_range > 0.0 { p.max_range } else { d.attack.range + size - 0.3 })
    }

    pub(crate) fn machinery_windup(&mut self, id: Id) {
        if self.machine_grind(id) {
            self.yell(id, "That'll cut it!");
        }
        let m = &self.mobs[&id];
        if self.content.mobs[m.def].key == DRIVER {
            self.machinery.wrenches.remove(&id);
            if m.pattern.is_some_and(|(k, _)| self.content.mobs[m.def].patterns[k].name == "wrench") {
                if let Some(at) = m.state.foe().and_then(|f| self.foe_pos(f)) {
                    self.machinery.wrenches.insert(id, at);
                }
            }
        }
    }

    pub(crate) fn machinery_shot(&mut self, id: Id) -> Option<Vec3> {
        self.machinery.wrenches.remove(&id)
    }

    /// Called only after the last blow actually executes, before its recovery. Interrupted
    /// wind-ups never build pressure, and generic string variation cannot repeat a grind.
    pub(crate) fn machinery_string_landed(&mut self, id: Id, pattern: Option<usize>, damage: u32) {
        let Some(k) = pattern else { return };
        let m = &self.mobs[&id];
        let name = self.content.mobs[m.def].patterns[k].name.as_str();
        if self.is_machine(id) {
            if name == "grind" {
                let (pos, dir) = (m.pos, m.facing);
                let to = self.slide_end(pos, dir, DISTANCE);
                self.machinery.pressure.insert(id, 0);
                self.machinery.grinds.insert(id, Grind { last: pos, facing: dir, until: self.clock + GRIND, damage, hit: BTreeSet::new() });
                // Straight only: a grind never follows a navigation detour around its victim.
                self.flee_to(id, to, dist2d(pos, to) / GRIND);
            } else {
                let heat = self.machinery.pressure.entry(id).or_default();
                *heat = (*heat + 1).min(2);
                self.machinery.steam_at.insert(id, self.clock);
            }
        } else if self.content.mobs[m.def].key == DRIVER && name == "wrench" {
            // The throw covers a short, navigable retreat. When cornered he stays and flurries.
            if let Some(foe) = m.state.foe().and_then(|f| self.foe_pos(f)) {
                let (pos, home) = (m.pos, m.home);
                let away = facing(foe, pos);
                let to = self.slide_end(pos, away, 4.0);
                if dist2d(pos, to) > 0.8 && dist2d(home, to) < 22.0 && self.land.walk_clear(pos, to) {
                    self.go(id, to, 4.0);
                }
            }
        }
    }

    /// The normal `then` handoff has spawned the driver, with his pack and original home.
    pub(crate) fn machinery_handoff(&mut self, machine: Id, driver: Id) {
        if !self.is_machine(machine) || self.content.mobs[self.mobs[&driver].def].key != DRIVER {
            return;
        }
        let old = &self.mobs[&machine];
        let (def, max_health) = (old.def, old.max_health);
        let rebuilt = self.machinery.rebuilt.remove(&machine);
        let stage = if rebuilt.is_some() { RepairStage::Spent } else { RepairStage::Fighting { after: self.clock + 5.0 } };
        if let Some(hp) = rebuilt {
            let m = self.mobs.get_mut(&driver).unwrap();
            m.health = hp.min(m.max_health).max(1);
            self.events.push(Event::Health { id: driver, health: m.health, max: m.max_health });
        } else {
            // The body is the wreck, kept only while this driver's one attempt is available.
            self.mobs.get_mut(&machine).unwrap().state = MobState::Dead { gone_at: f32::INFINITY };
        }
        self.machinery.drivers.insert(driver, Driver { wreck: rebuilt.is_none().then_some(machine), machine: def, max_health, stage });
    }

    pub(crate) fn machinery_hurt(&mut self, id: Id, damage: u32) {
        if damage > 0 && self.machinery.drivers.get(&id).is_some_and(|d| matches!(d.stage, RepairStage::Working { .. })) {
            self.abandon_repair(id);
        }
    }

    pub(crate) fn machinery_died(&mut self, id: Id) {
        self.machinery.pressure.remove(&id);
        self.machinery.steam_at.remove(&id);
        self.machinery.grinds.remove(&id);
        self.machinery.stalls.remove(&id);
        self.machinery.wrenches.remove(&id);
        // Keep rebuilt until the normal second-form handoff consumes it.
        if let Some(d) = self.machinery.drivers.remove(&id) {
            self.events.push(Event::CastEnd { caster: id });
            if let Some(wreck) = d.wreck {
                self.dismiss(wreck);
            }
        }
    }

    fn abandon_repair(&mut self, id: Id) {
        let wreck = self.machinery.drivers.get_mut(&id).and_then(|d| {
            d.stage = RepairStage::Spent;
            d.wreck.take()
        });
        self.events.push(Event::CastEnd { caster: id });
        self.halt(id);
        if let Some(m) = self.mobs.get_mut(&id) {
            if let Some(foe) = m.state.foe() {
                m.state = MobState::Recover { foe, until: self.clock + 1.2 };
                m.ready_at = self.clock + 1.2;
                m.think = 0.0;
            }
        }
        if let Some(wreck) = wreck { self.dismiss(wreck); }
    }

    /// Reset before the lamp replaces second forms. Bodies never survive their encounter.
    pub(crate) fn reset_machinery(&mut self) {
        let old = std::mem::take(&mut self.machinery);
        for (id, d) in old.drivers {
            self.events.push(Event::CastEnd { caster: id });
            if let Some(wreck) = d.wreck { self.dismiss(wreck); }
        }
    }

    pub(crate) fn clean_machinery(&mut self) {
        let stale: Vec<Id> = self.machinery.drivers.keys().copied().filter(|id| !self.mobs.get(id).is_some_and(|m| m.health > 0)).collect();
        for id in stale { self.machinery_died(id); }
        self.machinery.pressure.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0 && m.state.foe().is_some()));
        self.machinery.steam_at.retain(|id, _| self.machinery.pressure.contains_key(id));
        self.machinery.rebuilt.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0 && m.state.foe().is_some()));
        self.machinery.grinds.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0));
        self.machinery.stalls.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0 && m.state.foe().is_some()));
        self.machinery.wrenches.retain(|id, _| self.mobs.get(id).is_some_and(|m| m.health > 0 && matches!(m.state, MobState::Windup { .. })));
    }

    /// Runs after walking but before generic AI/CC checks, so a stun cannot finish a repair or
    /// leave a machine grinding while disabled. Return true while this beat owns its thinking.
    pub(crate) fn step_machinery(&mut self, id: Id) -> bool {
        let now = self.clock;
        if let Some(until) = self.machinery.stalls.get(&id).copied() {
            if now < until {
                let m = self.mobs.get_mut(&id).unwrap();
                if let Some(foe) = m.state.foe() {
                    // A posture break during the opening may extend it, never shorten it.
                    m.state = MobState::Recover { foe, until: until.max(m.stagger_until) };
                    m.pattern = None;
                    return true;
                }
            }
            self.machinery.stalls.remove(&id);
        }
        let heat = self.machinery.pressure.get(&id).copied().unwrap_or(0);
        if heat > 0 && now >= self.machinery.steam_at.get(&id).copied().unwrap_or(0.0) {
            self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
            self.machinery.steam_at.insert(id, now + 2.0 / heat as f32);
        }
        if let Some(mut g) = self.machinery.grinds.remove(&id) {
            let m = &self.mobs[&id];
            let stopped = m.state.foe().is_none() || self.disabled(id) || m.stagger_until > now || self.held_still(id);
            if !stopped {
                let at = m.pos;
                let len = dist2d(g.last, at) + 1.5;
                let victims: Vec<Id> = self.players.iter().filter(|(h, p)| p.health > 0 && !g.hit.contains(h) && combat::in_strip(g.last, g.facing, len, HALF_WIDTH, p.pos) && crate::land::clear_line(self.land.as_ref(), g.last, p.pos)).map(|(h, _)| *h).collect();
                for h in victims {
                    g.hit.insert(h);
                    self.strike_fair(id, h, g.damage, g.last, false, false, false, Blow::Light,
                        Some(combat::Area::Line { pos: g.last, facing: g.facing, len, half: HALF_WIDTH }), combat::Follow::None);
                }
                g.last = at;
            }
            if stopped || now >= g.until {
                self.halt(id);
                if let Some(foe) = self.mobs[&id].state.foe() {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.state = MobState::Recover { foe, until: now + STALL };
                    m.ready_at = now + STALL;
                    self.machinery.stalls.insert(id, now + STALL);
                    self.yell(id, "C'mon, you heap of junk!");
                    self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                    // This model has StandWound (8), but no stun clip.
                    self.events.push(Event::React { id, anim: 8, from: g.last, lean: 0.0, hold: STALL, fit: 0.0 });
                    self.events.push(Event::Sound { id, sound: 1979 });
                }
            } else {
                self.machinery.grinds.insert(id, g);
            }
            return true;
        }
        let Some(d) = self.machinery.drivers.get(&id).cloned() else { return false };
        // This beat temporarily owns AI, including its leash. Heroes leaving during the work
        // must not leave Sneed hammering forever or rebuilding an abandoned encounter.
        if self.next_in_fight(id).is_none() {
            self.mobs.get_mut(&id).unwrap().state = MobState::Return;
        }
        let m = &self.mobs[&id];
        if matches!(m.state, MobState::Return | MobState::Idle { .. }) {
            // A wipe during either form restores one whole machine, at its original post.
            let (home, pack, raider) = (m.home, m.pack, m.raider);
            self.machinery_died(id);
            self.dismiss(id);
            self.spawn_mob(d.machine, home, pack, raider);
            return true;
        }
        let Some(wreck) = d.wreck.and_then(|w| self.mobs.get(&w).map(|m| m.pos)) else { return false };
        let (pos, foe) = (m.pos, m.state.foe());
        let free = !self.disabled(id) && m.stagger_until <= now && m.thrown_until <= now;
        match d.stage {
            RepairStage::Fighting { after } => {
                let boundary = matches!(m.state, MobState::Chase { .. }) || matches!(m.state, MobState::Recover { until, .. } if now >= until && m.pattern.is_none());
                if now < after || !boundary || !free { return false; }
                self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Seeking { until: now + SEEK_LIMIT, repath_at: now + 0.5 };
                let m = self.mobs.get_mut(&id).unwrap();
                m.pattern = None;
                m.state = MobState::Chase { foe: foe.unwrap() };
                self.go(id, wreck, 5.0);
                true
            }
            RepairStage::Seeking { until, repath_at } => {
                if now >= until {
                    self.abandon_repair(id);
                } else if free && dist2d(pos, wreck) <= 1.6 && (pos[2] - wreck[2]).abs() < 2.0 {
                    self.halt(id);
                    self.mobs.get_mut(&id).unwrap().facing = facing(pos, wreck);
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { until: now + REPAIR, spark_at: now };
                    self.yell(id, "Just... one... bolt...");
                    self.events.push(Event::Cast { caster: id, spell: WORK, ms: (REPAIR * 1000.0) as u32 });
                } else if free && !self.held_still(id) && self.mobs[&id].path.is_empty() && now >= repath_at {
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Seeking { until, repath_at: now + 0.5 };
                    self.go(id, wreck, 5.0);
                }
                true
            }
            RepairStage::Working { until, spark_at } => {
                if !free || dist2d(pos, wreck) > 1.8 || (pos[2] - wreck[2]).abs() >= 2.0 {
                    self.abandon_repair(id);
                } else if now >= until {
                    self.finish_repair(id, d, wreck, foe.unwrap());
                } else if now >= spark_at {
                    self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { until, spark_at: now + 0.75 };
                }
                true
            }
            RepairStage::Spent => false,
        }
    }

    fn finish_repair(&mut self, driver: Id, d: Driver, at: Vec3, foe: Foe) {
        let m = &self.mobs[&driver];
        let (home, pack, raider, hp) = (m.home, m.pack, m.raider, m.health);
        self.machinery_died(driver);
        self.dismiss(driver);
        let id = self.spawn_mob(d.machine, at, pack, raider);
        self.engage(id, foe);
        let m = self.mobs.get_mut(&id).unwrap();
        m.home = home;
        m.max_health = d.max_health;
        m.health = (d.max_health / 3).max(1);
        m.state = MobState::Recover { foe, until: self.clock + 1.5 };
        m.ready_at = self.clock + 1.5;
        self.events.push(Event::Health { id, health: m.health, max: m.max_health });
        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
        self.machinery.rebuilt.insert(id, hp);
    }
}
