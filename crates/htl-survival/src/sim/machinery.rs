//! Sneed's Shredder and Sneed (docs/shredder-encounter.md). The machine works a log at its post,
//! turns and walks in, then cycles: saw sweep, stamp and rake, then the grind (a plant and a rev in
//! place, a straight drive) and a slumped stall. Broken, it collapses and throws Sneed out the far
//! side. On foot he keeps his distance with a wrench, flurries when cornered, and gets one repair
//! at the edge of the wreck farthest from his foe; finished, the wreck heaves up at a third of its
//! health. Every move shows itself by its own animation, sound and effects, not a tint.

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
const SEEK_SPEED: f32 = 5.0;
const WORK_ARRIVAL: f32 = 0.2;
const WORK_DRIFT: f32 = 0.35;
// Existing smoke puff (Vanish), used only as a visual. No concealment or damage.
const SMOKE: u32 = 1856;
// Mining's working animation, already used by gatherers.
const WORK: u32 = 2575;
/// GoblinShredder's own clips: Run (churning in place for the rev, then the drive), StandWound
/// (the slump of a stall) and JumpEnd (heaving up off the ground, rebuilt).
const RUN_CLIP: u16 = 5;
const SLUMP_CLIP: u16 = 8;
const HEAVE_CLIP: u16 = 39;
/// GoblinMale's StandWound, held: Sneed open after his flurry.
const OPEN_CLIP: u16 = 8;
/// Sounds: the saw (GoblinShredderAttack), the engine (GoblinShredder's loop), a stall's wound,
/// a vent's hiss (SteamGeyserVar1).
const SAW_SOUND: u32 = 1977;
const ENGINE_SOUND: u32 = 3700;
const STALL_SOUND: u32 = 1979;
const HISS_SOUND: u32 = 4858;
/// The pull: it turns from its log, then walks in at its Walk clip's own pace.
const TURN: f32 = 0.8;
const WALK_IN: f32 = 2.5;
/// Between cuts at its post.
const LOG_EVERY: f32 = 3.2;
/// The break: GoblinShredder's Death lands at 2.5 s; Sneed flies out over 0.6 s, speaks a second
/// after he lands, and strikes no sooner than 2 s after he lands.
const COLLAPSE: f32 = 2.5;
const THROWN: f32 = 0.6;
const THROWN_LIFT: f32 = 2.5;
const TALK_AFTER: f32 = 1.0;
const FIRST_BLOW: f32 = 2.0;
/// On foot: a wrench every 6 s; he backs off to 8 yd, and up close (cornered) he flurries.
const WRENCH_EVERY: f32 = 6.0;
const KEEP: f32 = 8.0;
const CORNERED: f32 = 3.4;
const SCRAMBLE: f32 = 4.0;
/// His one repair comes no sooner than this after he lands (a wrench or two first).
const REPAIR_AFTER: f32 = 6.0;
/// Rebuilt, the machine stands a moment before its first blow.
const REBUILT_WAIT: f32 = 2.0;
/// No two of the encounter's lines closer than this.
const LINE_GAP: f32 = 2.0;

#[derive(Default)]
pub(crate) struct Machinery {
    pressure: BTreeMap<Id, u8>,
    steam_at: BTreeMap<Id, f32>,
    grinds: BTreeMap<Id, Grind>,
    stalls: BTreeMap<Id, Stall>,
    revs: BTreeMap<Id, Rev>,
    wrenches: BTreeMap<Id, Vec3>,
    drivers: BTreeMap<Id, Driver>,
    /// A repaired machine carries its driver's remaining health, never a fresh driver.
    rebuilt: BTreeMap<Id, u32>,
    /// At its post: the next cut at the log, and whether its sparks are still to fly.
    logs: BTreeMap<Id, (f32, Option<f32>)>,
    /// Turned on a foe and walking in: when it sets off, and when it next looks again.
    approach: BTreeMap<Id, (f32, f32)>,
    /// In the fight: walked in already (or rebuilt where it stands).
    approached: BTreeSet<Id>,
    /// Lines waiting their turn, and when the last one was said.
    lines: Vec<(f32, Id, String)>,
    last_line: f32,
}

#[derive(Clone)]
struct Grind {
    last: Vec3,
    facing: f32,
    until: f32,
    damage: u32,
    hit: BTreeSet<Id>,
    fx_at: f32,
    fx: u8,
}

#[derive(Clone, Copy)]
struct Stall {
    until: f32,
    fx_at: f32,
    fx: u8,
}

/// A wind-up's own look while it builds: which move, its next effect, when it lands.
#[derive(Clone, Copy)]
struct Rev {
    kind: Move,
    fx_at: f32,
    lands_at: f32,
    landed: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum Move {
    Saw,
    Stamp,
    Rake,
    Grind,
}

#[derive(Clone)]
struct Driver {
    wreck: Option<Id>,
    machine: usize,
    max_health: u32,
    stage: RepairStage,
    emerge: Option<Emerge>,
    wrench_at: f32,
    smoke_at: f32,
    /// The wreck gone quiet after a stopped repair: a body for a while, gone at a wipe.
    quiet: Option<Id>,
}

/// Thrown out of the collapsing machine: hidden in it until `out_at`, in the air to `land_at`,
/// his line at `talk_at`.
#[derive(Clone, Copy)]
struct Emerge {
    out_at: f32,
    land_at: Option<f32>,
    talk_at: Option<f32>,
}

#[derive(Clone, Copy)]
enum RepairStage {
    Fighting { after: f32 },
    Seeking { at: Vec3, until: f32, repath_at: f32 },
    Working { at: Vec3, until: f32, spark_at: f32 },
    Spent,
}

impl Survival {
    pub(crate) fn is_machine(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == MACHINE)
    }

    fn is_driver(&self, id: Id) -> bool {
        self.mobs.get(&id).is_some_and(|m| self.content.mobs[m.def].key == DRIVER)
    }

    /// The machine or its driver: this module picks their blows.
    pub(crate) fn is_machinery(&self, id: Id) -> bool {
        self.is_machine(id) || self.is_driver(id)
    }

    /// The machine shows its own tells (its arm out, a stamp, a rev): no tint over its body.
    pub fn tells_by_body(&self, id: Id) -> bool {
        self.is_machine(id)
    }

    pub(crate) fn machine_grind(&self, id: Id) -> bool {
        self.is_machine(id) && self.pattern_name(id) == Some("grind")
    }

    fn pattern_name(&self, id: Id) -> Option<&str> {
        let m = self.mobs.get(&id)?;
        m.pattern.map(|(k, _)| self.content.mobs[m.def].patterns[k].name.as_str())
    }

    /// The machine's first two completed strings build heat; the third spends it, at any
    /// health. Sneed flurries when cornered, else throws his wrench when it's due.
    pub(crate) fn machinery_pattern(&self, id: Id, dist: f32, size: f32) -> Option<usize> {
        let m = self.mobs.get(&id)?;
        let d = &self.content.mobs[m.def];
        let name = if self.is_machine(id) {
            match self.machinery.pressure.get(&id).copied().unwrap_or(0) { 0 => "saws", 1 => "stamp and rake", _ => "grind" }
        } else if dist <= CORNERED {
            "flurry"
        } else if self.machinery.drivers.get(&id).is_none_or(|d| self.clock >= d.wrench_at) {
            "wrench"
        } else {
            return None;
        };
        d.patterns.iter().position(|p| p.name == name && dist <= if p.max_range > 0.0 { p.max_range } else { d.attack.range + size - 0.3 })
    }

    /// Debug damage never counts toward either form's posture, nor stops a repair.
    pub(crate) fn machinery_debug(&self, id: Id, label: &str) -> bool {
        label == "debug" && self.is_machinery(id)
    }

    /// Sneed inside the falling machine or in the air out of it: nothing reaches him yet.
    pub(crate) fn machinery_shielded(&self, id: Id) -> bool {
        self.machinery.drivers.get(&id).and_then(|d| d.emerge).is_some_and(|e| e.land_at.is_none_or(|t| self.clock < t))
    }

    /// One of the encounter's lines, never within 2 s of the last.
    fn machinery_say(&mut self, id: Id, text: &str) {
        let at = self.clock.max(self.machinery.last_line + LINE_GAP);
        self.machinery.last_line = at;
        if at <= self.clock {
            self.yell(id, text);
        } else {
            self.machinery.lines.push((at, id, text.to_string()));
        }
    }

    pub(crate) fn machinery_windup(&mut self, id: Id) {
        let now = self.clock;
        if self.is_machine(id) {
            let step = self.mobs[&id].pattern.map_or(0, |(_, s)| s);
            let kind = match (self.pattern_name(id), step) {
                (Some("grind"), _) => Move::Grind,
                (Some("stamp and rake"), 0) => Move::Stamp,
                (Some("stamp and rake"), _) => Move::Rake,
                _ => Move::Saw,
            };
            let windup = self.attack_now(id).windup;
            self.machinery.revs.insert(id, Rev { kind, fx_at: now, lands_at: now + windup, landed: false });
            if kind == Move::Grind {
                self.machinery_say(id, "That'll cut it!");
                // It plants and churns its legs in place: its Run, held for the rev, not the
                // rake's swing.
                self.events.retain(|e| !matches!(e, Event::MobWindup { id: who, .. } if *who == id));
                let pos = self.mobs[&id].pos;
                self.events.push(Event::React { id, anim: RUN_CLIP, from: pos, lean: 0.0, hold: windup, fit: 0.0 });
                self.events.push(Event::Sound { id, sound: ENGINE_SOUND });
            }
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
        let now = self.clock;
        let m = &self.mobs[&id];
        let name = self.content.mobs[m.def].patterns[k].name.clone();
        if self.is_machine(id) {
            if name == "grind" {
                let (pos, dir) = (m.pos, m.facing);
                let to = self.slide_end(pos, dir, DISTANCE);
                self.machinery.pressure.insert(id, 0);
                self.machinery.steam_at.remove(&id);
                self.machinery.grinds.insert(id, Grind { last: pos, facing: dir, until: now + GRIND, damage, hit: BTreeSet::new(), fx_at: now, fx: 0 });
                // Straight only: a grind never follows a navigation detour around its victim.
                self.flee_to(id, to, dist2d(pos, to) / GRIND);
            } else {
                let heat = self.machinery.pressure.entry(id).or_default();
                *heat = (*heat + 1).min(2);
                let heat = *heat;
                // One puff from the vents after the saw; at full heat they pour, hissing.
                self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                if heat >= 2 {
                    self.events.push(Event::Sound { id, sound: HISS_SOUND });
                    self.machinery.steam_at.insert(id, now + 0.5);
                }
            }
        } else if self.content.mobs[m.def].key == DRIVER && name == "wrench" {
            if let Some(d) = self.machinery.drivers.get_mut(&id) {
                d.wrench_at = now + WRENCH_EVERY;
            }
            // The throw covers a short, navigable retreat. When cornered he stays and flurries.
            if let Some(foe) = m.state.foe().and_then(|f| self.foe_pos(f)) {
                let (pos, home) = (m.pos, m.home);
                let away = facing(foe, pos);
                let to = self.slide_end(pos, away, 4.0);
                if dist2d(pos, to) > 0.8 && dist2d(home, to) < 22.0 && self.land.walk_clear(pos, to) {
                    self.go(id, to, 4.0);
                }
            }
        } else if self.content.mobs[m.def].key == DRIVER && name == "flurry" {
            // Spent after three blows: hunched, open for his recovery.
            let secs = self.content.mobs[m.def].patterns[k].steps.last().and_then(|st| st.recovery).unwrap_or(1.4);
            self.events.push(Event::HoldPose { id, clip: OPEN_CLIP, secs, from: None });
        }
    }

    /// The normal `then` handoff has spawned the driver, with his pack and original home. He
    /// stays inside the machine until it has collapsed, then is thrown out the far side.
    pub(crate) fn machinery_handoff(&mut self, machine: Id, driver: Id) {
        if !self.is_machine(machine) || self.content.mobs[self.mobs[&driver].def].key != DRIVER {
            return;
        }
        let now = self.clock;
        let old = &self.mobs[&machine];
        let (def, max_health) = (old.def, old.max_health);
        let rebuilt = self.machinery.rebuilt.remove(&machine);
        let stage = if rebuilt.is_some() { RepairStage::Spent } else { RepairStage::Fighting { after: f32::INFINITY } };
        if let Some(hp) = rebuilt {
            let m = self.mobs.get_mut(&driver).unwrap();
            m.health = hp.min(m.max_health).max(1);
            self.events.push(Event::Health { id: driver, health: m.health, max: m.max_health });
        } else {
            // The body is the wreck, kept only while this driver's one attempt is available.
            self.mobs.get_mut(&machine).unwrap().state = MobState::Dead { gone_at: f32::INFINITY };
        }
        // Not seen yet: the machine is still coming down.
        self.events.retain(|e| match e {
            Event::MobSpawned { id } | Event::Yell { id, .. } | Event::Visual { id, .. } | Event::Sound { id, .. } | Event::Emote { id, .. } => *id != driver,
            Event::Missile { caster, .. } => *caster != driver,
            _ => true,
        });
        let m = self.mobs.get_mut(&driver).unwrap();
        m.hidden = true;
        m.ready_at = f32::INFINITY;
        self.machinery.drivers.insert(driver, Driver {
            wreck: rebuilt.is_none().then_some(machine),
            machine: def,
            max_health,
            stage,
            emerge: Some(Emerge { out_at: now + COLLAPSE, land_at: None, talk_at: None }),
            wrench_at: 0.0,
            smoke_at: now + COLLAPSE,
            quiet: None,
        });
    }

    pub(crate) fn machinery_hurt(&mut self, id: Id, damage: u32, label: &str) {
        if damage > 0 && label != "debug" && self.machinery.drivers.get(&id).is_some_and(|d| matches!(d.stage, RepairStage::Working { .. })) {
            self.abandon_repair(id);
        }
    }

    pub(crate) fn machinery_died(&mut self, id: Id) {
        let dying = self.mobs.get(&id).is_some_and(|m| m.health == 0);
        if dying && self.is_machinery(id) {
            // Whatever pose it held (a slump, a rev, a hunch) stops as it falls.
            self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
        }
        if dying && self.is_machine(id) {
            self.machinery_say(id, "No, no, not now!");
        }
        self.machinery.pressure.remove(&id);
        self.machinery.steam_at.remove(&id);
        self.machinery.grinds.remove(&id);
        self.machinery.stalls.remove(&id);
        self.machinery.revs.remove(&id);
        self.machinery.wrenches.remove(&id);
        self.machinery.logs.remove(&id);
        self.machinery.approach.remove(&id);
        self.machinery.approached.remove(&id);
        // Keep rebuilt until the normal second-form handoff consumes it.
        if let Some(d) = self.machinery.drivers.remove(&id) {
            self.events.push(Event::CastEnd { caster: id });
            if let Some(wreck) = d.wreck {
                self.dismiss(wreck);
            }
        }
    }

    /// A hit (or a stagger) stops the repair for good. The wreck goes quiet and, like any body,
    /// is gone a while later.
    fn abandon_repair(&mut self, id: Id) {
        let now = self.clock;
        let wreck = self.machinery.drivers.get_mut(&id).and_then(|d| {
            d.stage = RepairStage::Spent;
            d.quiet = d.wreck.take();
            d.quiet
        });
        self.events.push(Event::CastEnd { caster: id });
        self.halt(id);
        if let Some(m) = self.mobs.get_mut(&id) {
            if let Some(foe) = m.state.foe() {
                m.state = MobState::Recover { foe, until: now + 1.2 };
                m.ready_at = now + 1.2;
                m.think = 0.0;
            }
        }
        if let Some(w) = wreck.and_then(|w| self.mobs.get_mut(&w)) {
            w.state = MobState::Dead { gone_at: now + 6.0 };
        }
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
        let now = self.clock;
        let stale: Vec<Id> = self.machinery.drivers.keys().copied().filter(|id| !self.mobs.get(id).is_some_and(|m| m.health > 0)).collect();
        for id in stale { self.machinery_died(id); }
        let alive = |s: &Survival, id: &Id| s.mobs.get(id).is_some_and(|m| m.health > 0);
        let fighting = |s: &Survival, id: &Id| s.mobs.get(id).is_some_and(|m| m.health > 0 && m.state.foe().is_some());
        let keep: Vec<Id> = self.machinery.pressure.keys().copied().filter(|id| fighting(self, id)).collect();
        self.machinery.pressure.retain(|id, _| keep.contains(id));
        self.machinery.steam_at.retain(|id, _| keep.contains(id));
        let keep: Vec<Id> = self.mobs.keys().copied().filter(|id| fighting(self, id)).collect();
        self.machinery.rebuilt.retain(|id, _| keep.contains(id));
        self.machinery.stalls.retain(|id, _| keep.contains(id));
        self.machinery.approach.retain(|id, _| keep.contains(id));
        self.machinery.approached.retain(|id| keep.contains(id));
        let keep: Vec<Id> = self.mobs.keys().copied().filter(|id| alive(self, id)).collect();
        self.machinery.grinds.retain(|id, _| keep.contains(id));
        self.machinery.logs.retain(|id, _| keep.contains(id));
        let winding: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.health > 0 && matches!(m.state, MobState::Windup { .. })).map(|(id, _)| *id).collect();
        self.machinery.wrenches.retain(|id, _| winding.contains(id));
        self.machinery.revs.retain(|id, _| winding.contains(id));
        // Lines whose turn has come; a speaker gone takes his with him (a dead machine's last
        // words were already said).
        let (due, later): (Vec<_>, Vec<_>) = std::mem::take(&mut self.machinery.lines).into_iter().partition(|(at, _, _)| *at <= now);
        self.machinery.lines = later;
        for (_, id, text) in due {
            if self.mobs.contains_key(&id) {
                self.yell(id, text);
            }
        }
        if self.machinery.drivers.is_empty() && !self.mobs.values().any(|m| m.health > 0 && m.state.foe().is_some() && self.content.mobs[m.def].key == MACHINE) {
            self.machinery.lines.clear();
        }
    }

    /// Runs after walking but before generic AI/CC checks, so a stun cannot finish a repair or
    /// leave a machine grinding while disabled. Return true while this beat owns its thinking.
    pub(crate) fn step_machinery(&mut self, id: Id) -> bool {
        if self.is_machine(id) {
            return self.step_machine(id);
        }
        if self.machinery.drivers.contains_key(&id) {
            return self.step_driver(id);
        }
        false
    }

    fn step_machine(&mut self, id: Id) -> bool {
        let now = self.clock;
        if matches!(self.mobs[&id].state, MobState::Idle { .. }) {
            self.work_log(id);
            return false;
        }
        self.machinery.logs.remove(&id);
        if let Some(mut st) = self.machinery.stalls.get(&id).copied() {
            if now < st.until {
                let m = self.mobs.get_mut(&id).unwrap();
                if let Some(foe) = m.state.foe() {
                    // A posture break during the opening may extend it, never shorten it.
                    m.state = MobState::Recover { foe, until: st.until.max(m.stagger_until) };
                    m.pattern = None;
                    // Slumped, smoking and sparking.
                    if now >= st.fx_at {
                        if st.fx % 2 == 0 {
                            self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                        } else {
                            self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                        }
                        st.fx += 1;
                        st.fx_at = now + 0.6;
                        self.machinery.stalls.insert(id, st);
                    }
                    return true;
                }
            }
            self.machinery.stalls.remove(&id);
        }
        let heat = self.machinery.pressure.get(&id).copied().unwrap_or(0);
        if heat >= 2 && self.machinery.steam_at.get(&id).is_some_and(|t| now >= *t) {
            self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
            self.machinery.steam_at.insert(id, now + 0.5);
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
                // Chips and sparks off the floor as it drives.
                if now >= g.fx_at {
                    let kit = if g.fx % 2 == 0 { combat::KIT_DUST } else { combat::KIT_PARRY };
                    self.events.push(Event::Visual { id, kit, by: None });
                    g.fx += 1;
                    g.fx_at = now + 0.2;
                }
            }
            if stopped || now >= g.until {
                self.halt(id);
                if let Some(foe) = self.mobs[&id].state.foe() {
                    let m = self.mobs.get_mut(&id).unwrap();
                    m.state = MobState::Recover { foe, until: now + STALL };
                    m.ready_at = now + STALL;
                    self.machinery.stalls.insert(id, Stall { until: now + STALL, fx_at: now + 0.3, fx: 1 });
                    self.machinery_say(id, "C'mon, you heap of junk!");
                    self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                    // Slumped where it stopped, facing the way it drove (no stun clip on this model).
                    self.events.push(Event::React { id, anim: SLUMP_CLIP, from: g.last, lean: 0.0, hold: STALL, fit: 0.0 });
                    self.events.push(Event::Sound { id, sound: STALL_SOUND });
                }
            } else {
                self.machinery.grinds.insert(id, g);
            }
            return true;
        }
        if let Some(mut r) = self.machinery.revs.get(&id).copied() {
            self.machine_rev(id, &mut r);
            self.machinery.revs.insert(id, r);
            return false;
        }
        self.machine_footwork(id)
    }

    /// At its post between fights, it cuts at the log in front of it: a swing every few seconds
    /// and sparks off the blade. It still notices a hero as ever (`think`).
    fn work_log(&mut self, id: Id) {
        let now = self.clock;
        let m = self.mobs.get_mut(&id).unwrap();
        // It stays at its work rather than wandering.
        m.state = MobState::Idle { wander_at: f32::INFINITY };
        let (pos, home, moving) = (m.pos, m.home, !m.path.is_empty());
        if moving || dist2d(pos, home) > 1.0 {
            return;
        }
        let Some((next, sparks)) = self.machinery.logs.get(&id).copied() else {
            self.machinery.logs.insert(id, (now + 0.5, None));
            return;
        };
        if sparks.is_some_and(|t| now >= t) {
            self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
            self.machinery.logs.insert(id, (next, None));
        }
        if now < next {
            return;
        }
        let face = self.log_facing(home);
        let m = self.mobs.get_mut(&id).unwrap();
        if (m.facing - face).abs() > 0.05 {
            m.facing = face;
            self.events.push(Event::MobStop { id, pos, facing: face });
        }
        let take = (now / LOG_EVERY) as u32 % 2;
        self.events.push(Event::Clip { id, clip: 16, take, from: None, secs: 1.5, at: 0.9, body: ClipBody::Full });
        self.events.push(Event::Sound { id, sound: SAW_SOUND });
        self.machinery.logs.insert(id, (now + LOG_EVERY, Some(now + 0.9)));
    }

    /// The way it faces at its work: away from the Mast Room's door, so a hero coming in finds
    /// its back.
    fn log_facing(&self, home: Vec3) -> f32 {
        self.dungeon_def()
            .and_then(|d| d.doors.iter().find(|dr| dr.boss == "sneed" && dr.kind == "fog"))
            .map_or(0.0, |dr| facing(dr.at, home))
    }

    /// A wind-up's look as it builds. The saw revs, sparking at the blade; the stamp lands in
    /// dust; the grind's engine races in place, smoking.
    fn machine_rev(&mut self, id: Id, r: &mut Rev) {
        let now = self.clock;
        match r.kind {
            Move::Saw if now >= r.fx_at => {
                self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                r.fx_at = now + 0.35;
            }
            Move::Grind if now >= r.fx_at => {
                self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                r.fx_at = now + 0.35;
            }
            _ => {}
        }
        if r.kind == Move::Stamp && !r.landed && now + 0.05 >= r.lands_at {
            r.landed = true;
            self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        }
    }

    /// Its approach and its waits. Pulled, it turns from its work, then walks in until in reach;
    /// between blows it stands squared up rather than circling inside its own legs.
    fn machine_footwork(&mut self, id: Id) -> bool {
        let now = self.clock;
        let m = &self.mobs[&id];
        let MobState::Chase { foe } = m.state else { return false };
        let Some(to) = self.foe_pos(foe) else { return false };
        if self.disabled(id) || m.stagger_until > now {
            return false;
        }
        let pos = m.pos;
        let dist = dist2d(pos, to);
        let a = self.attack_now(id);
        let stop = a.range - 0.3;
        if !self.machinery.approached.contains(&id) {
            let Some((walk_from, repath_at)) = self.machinery.approach.get(&id).copied() else {
                // Just turned on its foe (its pull line said as it did).
                self.machinery.last_line = self.machinery.last_line.max(now);
                if dist <= stop + 0.5 {
                    self.machinery.approached.insert(id);
                    return false;
                }
                self.machinery.approach.insert(id, (now + TURN, now + TURN));
                self.halt(id);
                let face = facing(pos, to);
                self.mobs.get_mut(&id).unwrap().facing = face;
                self.events.push(Event::MobStop { id, pos, facing: face });
                return true;
            };
            if now < walk_from {
                return true;
            }
            if dist <= stop + 0.5 {
                self.machinery.approach.remove(&id);
                self.machinery.approached.insert(id);
                self.halt(id);
                return false;
            }
            if self.mobs[&id].path.is_empty() || now >= repath_at {
                self.machinery.approach.insert(id, (walk_from, now + 0.5));
                let back = facing(to, pos);
                let goal = self.ground_near(to[0] + back.cos() * stop, to[1] + back.sin() * stop, to[2]);
                self.go(id, goal, WALK_IN);
            }
            return true;
        }
        // Waiting for its next blow, in reach: it turns to face its foe where it stands.
        if now < self.mobs[&id].ready_at && dist <= stop + 1.5 {
            let face = facing(pos, to);
            if !self.mobs[&id].path.is_empty() {
                self.halt(id);
            }
            let m = self.mobs.get_mut(&id).unwrap();
            if angle_between(m.facing, face) > 10.0 {
                m.facing = face;
                self.events.push(Event::MobStop { id, pos, facing: face });
            }
            return true;
        }
        false
    }

    fn step_driver(&mut self, id: Id) -> bool {
        let now = self.clock;
        let d = self.machinery.drivers[&id].clone();
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
            if let Some(w) = d.quiet { self.dismiss(w); }
            self.machinery.lines.clear();
            self.spawn_mob(d.machine, home, pack, raider);
            return true;
        }
        if let Some(e) = d.emerge {
            self.emerge(id, e);
            return true;
        }
        let wreck = d.wreck.and_then(|w| self.mobs.get(&w).map(|m| m.pos));
        // The wreck smokes while he can still mend it.
        if let (Some(w), Some(_)) = (d.wreck, wreck) {
            if now >= d.smoke_at {
                self.events.push(Event::Missile { caster: w, spell: SMOKE, target: Target::Unit(w) });
                self.machinery.drivers.get_mut(&id).unwrap().smoke_at = now + 1.6;
            }
        }
        if let Some(wreck) = wreck {
            if let Some(owned) = self.step_repair(id, &d, wreck) {
                return owned;
            }
        }
        self.driver_footwork(id)
    }

    /// Hidden in the collapsing machine, then thrown out the side away from his foe, landing in
    /// dust; a second later, his line; his first blow no sooner than 2 s after he lands.
    fn emerge(&mut self, id: Id, mut e: Emerge) {
        let now = self.clock;
        let foe = self.mobs[&id].state.foe().or_else(|| self.next_in_fight(id).map(Foe::Hero));
        if e.land_at.is_none() && now >= e.out_at {
            let to = self.bail_point(id, foe);
            let m = self.mobs.get_mut(&id).unwrap();
            m.hidden = false;
            self.events.push(Event::MobSpawned { id });
            self.throw_mob(id, to, THROWN, None, THROWN_LIFT);
            e.land_at = Some(now + THROWN);
        } else if let Some(land) = e.land_at.filter(|t| e.talk_at.is_none() && now >= *t) {
            let _ = land;
            self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
            let pos = self.mobs[&id].pos;
            if let Some(at) = foe.and_then(|f| self.foe_pos(f)) {
                let face = facing(pos, at);
                self.mobs.get_mut(&id).unwrap().facing = face;
                self.events.push(Event::MobStop { id, pos, facing: face });
            }
            e.talk_at = Some(now + TALK_AFTER);
            let d = self.machinery.drivers.get_mut(&id).unwrap();
            d.wrench_at = now + FIRST_BLOW;
            if matches!(d.stage, RepairStage::Fighting { .. }) {
                d.stage = RepairStage::Fighting { after: now + REPAIR_AFTER };
            }
            let m = self.mobs.get_mut(&id).unwrap();
            m.ready_at = now + FIRST_BLOW;
        } else if e.talk_at.is_some_and(|t| now >= t) {
            self.machinery_say(id, "That was company property!");
            self.machinery.drivers.get_mut(&id).unwrap().emerge = None;
            let m = self.mobs.get_mut(&id).unwrap();
            if let Some(f) = foe {
                m.state = MobState::Chase { foe: f };
            }
            m.think = 0.0;
            return;
        }
        self.machinery.drivers.get_mut(&id).unwrap().emerge = Some(e);
    }

    /// Where Sneed lands, thrown out: past the wreck's far edge from his foe, on dry footing.
    fn bail_point(&self, id: Id, foe: Option<Foe>) -> Vec3 {
        let wreck = self.machinery.drivers[&id].wreck.and_then(|w| self.mobs.get(&w));
        let (origin, face, scale) = match wreck {
            Some(w) => (w.pos, w.facing, self.content.mobs[w.def].scale),
            None => {
                let m = &self.mobs[&id];
                (m.pos, m.facing, self.content.mobs[self.machinery.drivers[&id].machine].scale)
            }
        };
        let hero = foe.and_then(|f| self.foe_pos(f)).unwrap_or(origin);
        let mut candidates = self.wreck_edges(origin, face, scale, 1.5);
        candidates.sort_by(|a, b| dist2d(*b, hero).total_cmp(&dist2d(*a, hero)));
        candidates
            .into_iter()
            .find_map(|at| self.land.footing(at).filter(|p| (p[2] - origin[2]).abs() <= 1.5))
            .unwrap_or_else(|| self.slide_end(origin, facing(hero, origin), 3.0))
    }

    /// Four points just outside GoblinShredder's Death pose (X [-4.29, 3.26], Y [-4.01, 4.80]
    /// before scale), `out` yards past it.
    fn wreck_edges(&self, origin: Vec3, face: f32, scale: f32, out: f32) -> [Vec3; 4] {
        [
            [3.26 * scale + out, 0.0], [-4.29 * scale - out, 0.0],
            [0.0, 4.80 * scale + out], [0.0, -4.01 * scale - out],
        ].map(|p| [origin[0] + face.cos() * p[0] - face.sin() * p[1],
            origin[1] + face.sin() * p[0] + face.cos() * p[1], origin[2]])
    }

    /// His one repair: run to the wreck's far edge from his foe, 4 s of hammering, any hit
    /// stops it. `Some(owned)` while this beat decides what he does.
    fn step_repair(&mut self, id: Id, d: &Driver, wreck: Vec3) -> Option<bool> {
        let now = self.clock;
        let m = &self.mobs[&id];
        let (pos, foe) = (m.pos, m.state.foe());
        let free = !self.disabled(id) && m.stagger_until <= now && m.thrown_until <= now;
        match d.stage {
            RepairStage::Fighting { after } => {
                let boundary = matches!(m.state, MobState::Chase { .. }) || matches!(m.state, MobState::Recover { until, .. } if now >= until && m.pattern.is_none());
                if now < after || !boundary || !free { return None; }
                let Some(at) = self.repair_work_point(id, d.wreck.unwrap(), foe.unwrap()) else {
                    self.abandon_repair(id);
                    return Some(true);
                };
                self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Seeking { at, until: now + SEEK_LIMIT, repath_at: now + 0.5 };
                self.machinery_say(id, "Just... one... bolt...");
                let m = self.mobs.get_mut(&id).unwrap();
                m.pattern = None;
                m.state = MobState::Chase { foe: foe.unwrap() };
                self.go(id, at, SEEK_SPEED);
                Some(true)
            }
            RepairStage::Seeking { at, until, repath_at } => {
                if now >= until {
                    self.abandon_repair(id);
                } else if free && dist2d(pos, at) <= WORK_ARRIVAL && (pos[2] - at[2]).abs() < 0.5 && self.land.footing(pos).is_some() {
                    self.halt(id);
                    let face = facing(pos, wreck);
                    self.mobs.get_mut(&id).unwrap().facing = face;
                    self.events.push(Event::MobStop { id, pos, facing: face });
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { at, until: now + REPAIR, spark_at: now };
                    self.events.push(Event::Cast { caster: id, spell: WORK, ms: (REPAIR * 1000.0) as u32 });
                } else if free && !self.held_still(id) && self.mobs[&id].path.is_empty() && now >= repath_at {
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Seeking { at, until, repath_at: now + 0.5 };
                    self.go(id, at, SEEK_SPEED);
                }
                Some(true)
            }
            RepairStage::Working { at, until, spark_at } => {
                if !free || dist2d(pos, at) > WORK_DRIFT || (pos[2] - at[2]).abs() >= 0.5 || self.land.footing(pos).is_none() {
                    self.abandon_repair(id);
                } else if now >= until {
                    self.finish_repair(id, d.clone(), wreck, foe.unwrap());
                } else if now >= spark_at {
                    self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { at, until, spark_at: now + 0.75 };
                }
                Some(true)
            }
            RepairStage::Spent => None,
        }
    }

    /// On foot he keeps his distance, backing off facing his foe; a wrench when it's due, a
    /// flurry once he's caught. The generic AI throws and flurries (`machinery_pattern`).
    fn driver_footwork(&mut self, id: Id) -> bool {
        let now = self.clock;
        let m = &self.mobs[&id];
        let MobState::Chase { foe } = m.state else { return false };
        let Foe::Hero(h) = foe else { return false };
        let Some(to) = self.foe_pos(foe) else { return false };
        if self.disabled(id) || m.stagger_until > now || m.thrown_until > now {
            return false;
        }
        let (pos, home) = (m.pos, m.home);
        let dist = dist2d(pos, to);
        if dist <= CORNERED {
            return false;
        }
        let wrench = self.machinery.drivers[&id].wrench_at;
        if now >= m.ready_at && now >= wrench && dist <= 16.0 && crate::land::clear_line(self.land.as_ref(), pos, to) {
            return false;
        }
        if dist > 16.0 {
            return false;
        }
        // Still backing off: he finishes that.
        if !m.path.is_empty() {
            return true;
        }
        if dist < KEEP {
            let goal = self.slide_end(pos, facing(to, pos), 3.0);
            if dist2d(goal, pos) > 1.0 && dist2d(home, goal) < 22.0 && self.land.walk_clear(pos, goal) {
                self.sidle_to(id, goal, SCRAMBLE, h);
                if !self.mobs[&id].path.is_empty() {
                    return true;
                }
            }
        }
        // Nowhere to back off to, or far enough: he stands and faces his foe.
        let face = facing(pos, to);
        let m = self.mobs.get_mut(&id).unwrap();
        if angle_between(m.facing, face) > 15.0 {
            m.facing = face;
            self.events.push(Event::MobStop { id, pos, facing: face });
        }
        true
    }

    /// Work beside the retained Death pose, not at the machine's origin inside it. Leave room
    /// for the driver's body even at arrival/drift tolerance. Try the edge farthest from his foe
    /// first (he runs from you to it); a blocked edge never becomes a center fallback.
    fn repair_work_point(&self, driver: Id, wreck: Id, foe: Foe) -> Option<Vec3> {
        let m = &self.mobs[&wreck];
        let (origin, face, scale) = (m.pos, m.facing, self.content.mobs[m.def].scale);
        let from = self.mobs[&driver].pos;
        let hero = self.foe_pos(foe)?;
        let clearance = 0.5 * self.content.mobs[self.mobs[&driver].def].scale + 0.5;
        let mut candidates = self.wreck_edges(origin, face, scale, clearance);
        candidates.sort_by(|a, b| dist2d(*b, hero).total_cmp(&dist2d(*a, hero)));
        candidates.into_iter().find_map(|at| {
            let at = self.land.footing(at)?;
            // The driver and an approaching hero must stay on the wreck's own floor.
            if (at[2] - origin[2]).abs() > 1.5 { return None; }
            // A dry work point pressed against a wall is still unreadable to melee.
            // Keep one ordinary swing's standing room outside this same edge.
            let outward = facing(origin, at);
            let approach = self.land.footing([at[0] + outward.cos() * 1.2,
                at[1] + outward.sin() * 1.2, at[2]])?;
            if (approach[2] - at[2]).abs() > 0.5 || !self.land.walk_clear(approach, at) { return None; }
            let route = crate::land::walking_route(self.land.as_ref(), &[from, at])?;
            let length: f32 = route.windows(2).map(|p| dist2d(p[0], p[1])).sum();
            (length <= SEEK_LIMIT * SEEK_SPEED).then_some(at)
        })
    }

    /// The same wreck heaves back up in smoke, at a third of its health, where it lay and facing
    /// the way it fell; it stands a moment before its first blow.
    fn finish_repair(&mut self, driver: Id, d: Driver, at: Vec3, foe: Foe) {
        let now = self.clock;
        let m = &self.mobs[&driver];
        let (home, pack, raider, hp) = (m.home, m.pack, m.raider, m.health);
        let face = d.wreck.and_then(|w| self.mobs.get(&w)).map_or(0.0, |w| w.facing);
        self.machinery_died(driver);
        self.dismiss(driver);
        let id = self.spawn_mob(d.machine, at, pack, raider);
        let m = self.mobs.get_mut(&id).unwrap();
        m.facing = face;
        // Not a fresh machine: no pull line, and no walk in.
        m.voiced = true;
        self.machinery.approached.insert(id);
        self.engage(id, foe);
        let m = self.mobs.get_mut(&id).unwrap();
        m.home = home;
        m.max_health = d.max_health;
        m.health = (d.max_health / 3).max(1);
        m.state = MobState::Recover { foe, until: now + REBUILT_WAIT };
        m.ready_at = now + REBUILT_WAIT;
        self.events.push(Event::Health { id, health: m.health, max: m.max_health });
        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::React { id, anim: HEAVE_CLIP, from: at, lean: 0.0, hold: 0.0, fit: 1.2 });
        self.events.push(Event::Sound { id, sound: ENGINE_SOUND });
        self.machinery.rebuilt.insert(id, hp);
    }
}

#[cfg(test)]
#[path = "machinery_tests.rs"]
mod tests;
