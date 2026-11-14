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
/// The repair is a race to the log's tie-off, sized on the Mast Room's floor (`shredder_rig.rs`):
/// his hands are on the wreck 0.6 s after his line; a warrior on him is 4 yd from the stake and
/// the log lands 2.0 s after they go for it (0.55 s of run, 0.45 s of swing, the creak and the
/// fall). At 4.2 s whoever goes within 2.5 s of his line wins and whoever waits 3 s loses. No
/// longer than that: the stake is a few yards from him, so a longer repair is won at a stroll.
const REPAIR: f32 = 4.2;
/// His stance from his line until the repair is done or dropped: his hands are on the wreck, not
/// up (as Cookie's at her scoop and Gilnid's at his valve, 120). With the log spent a broken
/// stance is the one stop short of killing him, and at his own 160 no warrior breaks it in 4.2 s
/// from nothing (light blows reach 97, heavy 67: `shredder_rig.rs`). At 100 light blows without
/// a gap from the moment his hands are on it break it 3.35 s in, heavy ones 3.95 s in: possible,
/// and hard. At 120 it is 3.95 s for light blows and never for heavy.
pub(crate) const MEND_POSTURE: u32 = 100;
const SEEK_LIMIT: f32 = 6.0;
const SEEK_SPEED: f32 = 5.0;
const WORK_ARRIVAL: f32 = 0.2;
const WORK_DRIFT: f32 = 0.35;
// Existing smoke puff (Vanish), used only as a visual. No concealment or damage.
const SMOKE: u32 = 1856;
// Mining's working animation, already used by gatherers.
const WORK: u32 = 2575;
/// GoblinShredder's own clips: Run (churning in place for the rev, then the drive) and JumpEnd
/// (heaving up off the ground, rebuilt). The stall's slump is JumpEnd too: its landing crouch,
/// caught `SLUMP_AT` in with the legs buckled (the body at half its height, tipped about 30
/// degrees) and held still until the engine catches `SLUMP_RISE` before the stall's end and it
/// heaves itself up. (StandWound, the slump before, kept the body at full height: the Nov 9
/// filming read the stall as idling.)
const RUN_CLIP: u16 = 5;
const SLUMP_CLIP: u16 = 39;
const SLUMP_AT: f32 = 0.25;
const SLUMP_RISE: f32 = 0.45;
const HEAVE_CLIP: u16 = 39;
/// GoblinMale's StandWound, held: Sneed open after his flurry.
const OPEN_CLIP: u16 = 8;
/// Sounds: the saw (GoblinShredderAttack), the engine (GoblinShredder's loop), a stall's wound,
/// a vent's hiss (SteamGeyserVar1).
const SAW_SOUND: u32 = 1977;
const ENGINE_SOUND: u32 = 3700;
const STALL_SOUND: u32 = 1979;
const HISS_SOUND: u32 = 4858;
/// The stalled engine's sputter: the machine's own clanks (GoblinShredderWound), at uneven gaps.
const SPUTTER_SOUND: u32 = 1978;
const SPUTTER_GAPS: [f32; 5] = [0.55, 0.3, 0.75, 0.35, 0.5];
/// Through a stall, a smoke puff this often (it pours) and sparks this often.
const STALL_SMOKE_EVERY: f32 = 0.25;
const STALL_SPARK_EVERY: f32 = 0.4;
/// A wreck left on the floor (a stopped repair's, or the one under Sneed) goes with his body.
const BODY_SECS: f32 = 4.0;
/// The pull: it turns from its log, then walks in at its Walk clip's own pace.
const TURN: f32 = 0.8;
const WALK_IN: f32 = 2.5;
/// Between cuts at its post.
const LOG_EVERY: f32 = 3.2;
/// The break: GoblinShredder's Death lands at 2.5 s. Then scrap bursts off the wreck and Sneed
/// lies in it 1 s (GoblinMale's Sleep), gets up over 2 s (SleepUp, its own length), says his
/// line, and strikes no sooner than 1 s after it: 4 s in view before he can act.
const COLLAPSE: f32 = 2.5;
const LIE_CLIP: u16 = 100;
const LIE: f32 = 1.0;
const RISE_CLIP: u16 = 101;
const RISE: f32 = 2.0;
const FIRST_BLOW: f32 = 1.0;
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
/// His repair's lines: half way, stopped by a broken stance, and with no way to the wreck. (The
/// log's is its own: `machinery_rig.rs`.) He never gives it up without a word.
const LINE_HALF: &str = "She's turning over!";
const LINE_STOPPED: &str = "Bah! Forget the heap. I'll do this by hand!";
const LINE_NO_WAY: &str = "Bah! Can't get at her. By hand, then!";

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
    /// Each post's rigged log, a broken machine on its run back, and the props drawn with no
    /// hull at a tilt (the log, its stake, the crash's lumber).
    rigs: BTreeMap<usize, rig::Rig>,
    runaways: BTreeMap<Id, rig::Runaway>,
    tilts: BTreeMap<Id, f32>,
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
    smoke_at: f32,
    spark_at: f32,
    sputter_at: f32,
    sputters: usize,
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

/// Out of the wreck: hidden in it until `out_at`, then seen lying in its scrap until he gets up
/// at `up_at`, on his feet with his line at `talk_at`. Nothing reaches him, and he strikes
/// nobody, until then.
#[derive(Clone, Copy)]
struct Emerge {
    out_at: f32,
    up_at: Option<f32>,
    talk_at: Option<f32>,
}

#[derive(Clone, Copy)]
enum RepairStage {
    Fighting { after: f32 },
    Seeking { at: Vec3, until: f32, repath_at: f32 },
    Working { at: Vec3, until: f32, spark_at: f32, beat: u8 },
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

    /// Sneed inside the wreck or getting up out of it, and the broken machine on its run back:
    /// nothing reaches them.
    pub(crate) fn machinery_shielded(&self, id: Id) -> bool {
        self.machinery.runaways.contains_key(&id) || self.machinery.drivers.get(&id).is_some_and(|d| d.emerge.is_some())
    }

    /// Sneed on his way to the wreck or hammering at it (he steps back from no blow then:
    /// `evade_heavy`).
    pub(crate) fn mending(&self, id: Id) -> bool {
        self.machinery.drivers.get(&id).is_some_and(|d| matches!(d.stage, RepairStage::Seeking { .. } | RepairStage::Working { .. }))
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
            // Held through the hits that punish it.
            self.hold_open_pose(id, OPEN_CLIP, secs);
        }
    }

    /// The normal `then` handoff has spawned the driver, with his pack and original home. He
    /// stays inside the machine until it has collapsed, then crawls out of the wreck.
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
        }
        // The body is the wreck he crawls out of: it lies there until his body goes (a second
        // wreck too, quiet from the start: its 4 s as a body ended while he was getting up).
        self.mobs.get_mut(&machine).unwrap().state = MobState::Dead { gone_at: f32::INFINITY };
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
            emerge: Some(Emerge { out_at: now + COLLAPSE, up_at: None, talk_at: None }),
            wrench_at: 0.0,
            smoke_at: now + COLLAPSE,
            quiet: rebuilt.is_some().then_some(machine),
        });
    }


    pub(crate) fn machinery_died(&mut self, id: Id) {
        let dying = self.mobs.get(&id).is_some_and(|m| m.health == 0);
        if dying && self.is_machinery(id) {
            // Whatever pose it held (a slump, a rev, a hunch) stops as it falls.
            self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
        }
        // Its last words were said as it broke, before its run back (`start_runaway`).
        self.machinery.runaways.remove(&id);
        self.machinery.pressure.remove(&id);
        self.machinery.steam_at.remove(&id);
        self.machinery.grinds.remove(&id);
        // A stall's held slump lets go, so its fall plays.
        if self.machinery.stalls.remove(&id).is_some() {
            self.freeze_open_pose(id, 0.0, 0.0);
        }
        self.machinery.revs.remove(&id);
        self.machinery.wrenches.remove(&id);
        self.machinery.logs.remove(&id);
        self.machinery.approach.remove(&id);
        self.machinery.approached.remove(&id);
        // Keep rebuilt until the normal second-form handoff consumes it.
        if let Some(d) = self.machinery.drivers.remove(&id) {
            self.events.push(Event::CastEnd { caster: id });
            // Sneed fallen, the wreck lies where it fell and goes with his body; otherwise (a
            // repair finished, a wipe) it's gone now.
            for wreck in d.wreck.into_iter().chain(d.quiet) {
                if dying {
                    let gone_at = self.clock + BODY_SECS;
                    if let Some(w) = self.mobs.get_mut(&wreck) { w.state = MobState::Dead { gone_at }; }
                } else {
                    self.dismiss(wreck);
                }
            }
            if dying {
                // The crash's lumber and a fallen log go with his body too.
                self.rig_body_gone(id, self.clock + BODY_SECS);
            }
        }
    }

    /// The repair is stopped for good: by the log, a stagger, or no way to the wreck. Never
    /// without a word: `line` is what he says (the log has its own). The wreck goes quiet where
    /// it lies (no smoke, no sparks) until Sneed's body goes or a wipe clears the floor.
    fn abandon_repair(&mut self, id: Id, line: Option<&str>) {
        let now = self.clock;
        let Some(d) = self.machinery.drivers.get_mut(&id) else { return };
        if matches!(d.stage, RepairStage::Spent) {
            return;
        }
        d.stage = RepairStage::Spent;
        d.quiet = d.wreck.take();
        self.events.push(Event::CastEnd { caster: id });
        self.halt(id);
        if let Some(m) = self.mobs.get_mut(&id) {
            if let Some(foe) = m.state.foe() {
                // Reeling already (a broken stance): that holds him, and longer.
                m.state = MobState::Recover { foe, until: (now + 1.2).max(m.stagger_until) };
                m.ready_at = (now + 1.2).max(m.stagger_until);
                m.think = 0.0;
            }
        }
        if let Some(text) = line {
            self.machinery_say(id, text);
        }
    }

    /// Reset before the lamp replaces second forms. Bodies never survive their encounter.
    pub(crate) fn reset_machinery(&mut self) {
        let old = std::mem::take(&mut self.machinery);
        for (id, d) in old.drivers {
            self.events.push(Event::CastEnd { caster: id });
            for wreck in d.wreck.into_iter().chain(d.quiet) { self.dismiss(wreck); }
        }
        // The log, its ropes and what fell go too; the log is hung again when the machine is back.
        self.rigs_gone(old.rigs);
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
        self.machinery.runaways.retain(|id, _| keep.contains(id));
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
        self.step_rigs();
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
        // Broken: it runs back to its post and does nothing else.
        if self.machinery.runaways.contains_key(&id) {
            self.step_runaway(id);
            return true;
        }
        self.rig_ensure(id);
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
                    // Slumped, smoke pouring and sparks flying the whole stall, the engine
                    // sputtering.
                    if now >= st.smoke_at {
                        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
                        st.smoke_at = now + STALL_SMOKE_EVERY;
                    }
                    if now >= st.spark_at {
                        self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                        st.spark_at = now + STALL_SPARK_EVERY;
                    }
                    if now >= st.sputter_at {
                        self.events.push(Event::Sound { id, sound: SPUTTER_SOUND });
                        st.sputter_at = now + SPUTTER_GAPS[st.sputters % SPUTTER_GAPS.len()];
                        st.sputters += 1;
                    }
                    self.machinery.stalls.insert(id, st);
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
                self.stall_machine(id, g.last);
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

    /// The engine stalls (after a Grind, or under the fallen log): slumped and open for `STALL`,
    /// smoke pouring. `from` is where the blow that stalled it came from (the way it drove).
    fn stall_machine(&mut self, id: Id, from: Vec3) {
        let now = self.clock;
        let Some(foe) = self.mobs[&id].state.foe() else { return };
        let m = self.mobs.get_mut(&id).unwrap();
        m.state = MobState::Recover { foe, until: now + STALL };
        m.ready_at = now + STALL;
        self.machinery.stalls.insert(id, Stall { until: now + STALL, smoke_at: now + STALL_SMOKE_EVERY, spark_at: now + 0.15, sputter_at: now + 0.4, sputters: 0 });
        self.machinery_say(id, "C'mon, you heap of junk!");
        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
        // Slumped where it stopped, facing the way it drove: its legs buckle and it's
        // held there, no flinch standing it up, until it heaves itself up at the end.
        self.events.push(Event::React { id, anim: SLUMP_CLIP, from, lean: 0.0, hold: 0.1, fit: 0.0 });
        self.freeze_open_pose(id, SLUMP_AT, STALL - SLUMP_AT - SLUMP_RISE);
        self.mobs.get_mut(&id).unwrap().posed_until = now + STALL;
        self.events.push(Event::Sound { id, sound: STALL_SOUND });
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
                // Thicker and thicker as his repair comes on.
                let done = match d.stage { RepairStage::Working { until, .. } => (1.0 - (until - now) / REPAIR).clamp(0.0, 1.0), _ => 0.0 };
                self.machinery.drivers.get_mut(&id).unwrap().smoke_at = now + 1.6 - 1.2 * done;
            }
        }
        if let Some(wreck) = wreck {
            if let Some(owned) = self.step_repair(id, &d, wreck) {
                return owned;
            }
        }
        self.driver_footwork(id)
    }

    /// Hidden in the collapsing machine, then seen coming out of it: scrap bursts off the wreck
    /// and he lies in it, gets up, and says his line; his first blow no sooner than 1 s after
    /// that. Nothing reaches him and he strikes nobody until he's up (`machinery_shielded`).
    fn emerge(&mut self, id: Id, mut e: Emerge) {
        let now = self.clock;
        let foe = self.mobs[&id].state.foe().or_else(|| self.next_in_fight(id).map(Foe::Hero));
        if e.up_at.is_none() && now >= e.out_at {
            let (to, lie) = self.crawl_point(id, foe);
            let m = self.mobs.get_mut(&id).unwrap();
            m.pos = to;
            m.facing = lie;
            m.path.clear();
            m.speed = 0.0;
            m.hidden = false;
            self.events.push(Event::MobSpawned { id });
            self.hold_open_pose(id, LIE_CLIP, LIE + 0.3);
            self.scrap_burst(id, to);
            e.up_at = Some(now + LIE);
        } else if e.up_at.is_some_and(|t| e.talk_at.is_none() && now >= t) {
            let pos = self.mobs[&id].pos;
            if let Some(at) = foe.and_then(|f| self.foe_pos(f)) {
                let face = facing(pos, at);
                self.mobs.get_mut(&id).unwrap().facing = face;
                self.events.push(Event::MobStop { id, pos, facing: face });
            }
            self.hold_open_pose(id, 0, 0.0);
            self.events.push(Event::React { id, anim: RISE_CLIP, from: pos, lean: 0.0, hold: 0.0, fit: RISE });
            e.talk_at = Some(now + RISE);
        } else if e.talk_at.is_some_and(|t| now >= t) {
            self.machinery_say(id, "That was company property!");
            let d = self.machinery.drivers.get_mut(&id).unwrap();
            d.emerge = None;
            d.wrench_at = now + FIRST_BLOW;
            if matches!(d.stage, RepairStage::Fighting { .. }) {
                d.stage = RepairStage::Fighting { after: now + REPAIR_AFTER };
            }
            let m = self.mobs.get_mut(&id).unwrap();
            m.ready_at = now + FIRST_BLOW;
            if let Some(f) = foe {
                m.state = MobState::Chase { foe: f };
            }
            m.think = 0.0;
            return;
        }
        self.machinery.drivers.get_mut(&id).unwrap().emerge = Some(e);
    }

    /// Where Sneed comes out, and the way he lies: the post's own place beside the wreck (open
    /// floor, toward where he'll work), lying away from it; a wreck anywhere else (a land with
    /// no way back to the post), past its far edge from his foe.
    fn crawl_point(&self, id: Id, foe: Option<Foe>) -> (Vec3, f32) {
        let pos = self.mobs[&id].pos;
        if let Some(g) = self.rig_of(id).and_then(|k| self.machinery.rigs.get(&k)) {
            if let Some(at) = g.crawl.filter(|_| dist2d(pos, g.crash) <= 1.5) {
                return (at, g.across);
            }
        }
        let at = self.bail_point(id, foe);
        (at, facing(pos, at))
    }

    /// Where Sneed comes out of a wreck away from its post: past its far edge from his foe, on
    /// dry footing.
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

    /// His one repair: a run to his place at the wreck, under the log, and `REPAIR` s of
    /// hammering. A plain hit hurts him and no more: the log on him, a broken stance or his death
    /// stop it, for good. It's seen coming on: his sparks faster and faster, the wreck coughing
    /// and smoking more at each quarter. `Some(owned)` while this beat decides what he does.
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
                    self.abandon_repair(id, Some(LINE_NO_WAY));
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
                if !free {
                    // Staggered on his way to it: that's the repair, too.
                    self.abandon_repair(id, Some(LINE_STOPPED));
                } else if now >= until {
                    self.abandon_repair(id, Some(LINE_NO_WAY));
                } else if dist2d(pos, at) <= WORK_ARRIVAL && (pos[2] - at[2]).abs() < 0.5 && self.land.footing(pos).is_some() {
                    self.halt(id);
                    let face = facing(pos, wreck);
                    self.mobs.get_mut(&id).unwrap().facing = face;
                    self.events.push(Event::MobStop { id, pos, facing: face });
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { at, until: now + REPAIR, spark_at: now, beat: 0 };
                    self.events.push(Event::Cast { caster: id, spell: WORK, ms: (REPAIR * 1000.0) as u32 });
                } else if !self.held_still(id) && self.mobs[&id].path.is_empty() && now >= repath_at {
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Seeking { at, until, repath_at: now + 0.5 };
                    self.go(id, at, SEEK_SPEED);
                }
                Some(true)
            }
            RepairStage::Working { at, until, spark_at, beat } => {
                let done = (1.0 - (until - now) / REPAIR).clamp(0.0, 1.0);
                if !free || dist2d(pos, at) > WORK_DRIFT || (pos[2] - at[2]).abs() >= 0.5 || self.land.footing(pos).is_none() {
                    self.abandon_repair(id, Some(LINE_STOPPED));
                } else if now >= until {
                    self.finish_repair(id, d.clone(), wreck, foe.unwrap());
                } else if done >= 0.25 * (beat as f32 + 1.0) && beat < 3 {
                    // A quarter further on: the wreck coughs, sparks and smokes, louder each time.
                    if let Some(w) = d.wreck {
                        self.events.push(Event::Sound { id: w, sound: SPUTTER_SOUND });
                        self.events.push(Event::Visual { id: w, kit: combat::KIT_PARRY, by: None });
                        for _ in 0..=beat {
                            self.events.push(Event::Missile { caster: w, spell: SMOKE, target: Target::Unit(w) });
                        }
                    }
                    if beat == 1 {
                        self.machinery_say(id, LINE_HALF);
                    }
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { at, until, spark_at, beat: beat + 1 };
                } else if now >= spark_at {
                    // His hammer's sparks, faster as it comes on: one every 0.8 s, then every 0.25.
                    self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
                    self.machinery.drivers.get_mut(&id).unwrap().stage = RepairStage::Working { at, until, spark_at: now + 0.8 - 0.55 * done, beat };
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
        // At its post the wreck lies under the rigged log, and he works at one fixed place there.
        if let Some(g) = self.rig_of(driver).and_then(|k| self.machinery.rigs.get(&k)) {
            if let Some(at) = g.work.filter(|_| dist2d(origin, g.crash) <= 1.5) {
                let route = crate::land::walking_route(self.land.as_ref(), &[from, at])?;
                let length: f32 = route.windows(2).map(|p| dist2d(p[0], p[1])).sum();
                return (length <= SEEK_LIMIT * SEEK_SPEED).then_some(at);
            }
        }
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

#[path = "machinery_rig.rs"]
mod rig;
pub use rig::ShredderRig;

#[cfg(test)]
#[path = "machinery_tests.rs"]
mod tests;
