//! The second half of Sneed's Shredder (docs/shredder-encounter.md). Broken, the machine careers
//! back to its post and crashes into the log pile it was cutting, so its wreck always lies in the
//! same place: under a log rigged over it from the pull. One rope holds the log, tied off at a
//! stake on the floor a few yards outside its drop; a blow on the stake cuts it, the log creaks
//! and comes down, and whoever is under it is stunned: a hero, Sneed at his repair, the machine.

use super::*;

/// The wreck lies this far ahead of the post, toward the log it was cutting: the fallen machine's
/// front (3.6 yd of it ahead of its middle) then reaches the Mast Room's pile, 7.4 yd ahead.
const CRASH_AHEAD: f32 = 3.5;
/// Broken, it churns where it stands this long (its line, smoke, sparks) before it runs: time
/// to see that it broke and wasn't killed.
const SPUTTER: f32 = 0.7;
/// The run back takes about this long wherever it starts: its pace is set from the way's length,
/// between its own run (6 yd/s) and the fastest that still reads as a run. (The longest way
/// back in the Mast Room is measured in `shredder_rig.rs`.)
pub(super) const RUN_BACK: f32 = 2.0;
const RUN_SLOWEST: f32 = 6.0;
const RUN_FASTEST: f32 = 18.0;
/// A run still going after this is set down at the pile under its smoke, and logged: the wreck
/// lies under the log every time. (Never needed on the Mast Room's floor: `shredder_rig.rs`.)
const RUN_LIMIT: f32 = 6.0;
/// Smoke and sparks pour off it as it runs, this often.
const RUN_FX_EVERY: f32 = 0.2;
/// The log: the dig site's barrier pole (ExcavationBarrierPole, 2.4 yd thick and 29.6 long along
/// its own Z, 5.77 of that behind its origin), laid on its side at a third of its size: 0.8 yd
/// thick, 9.5 long. It hangs from the wreck's middle out past where Sneed works.
const LOG: u32 = 1508;
const LOG_SCALE: f32 = 0.32;
const LOG_MODEL_BACK: f32 = 5.77;
pub(super) const LOG_LEN: f32 = 9.5;
/// One pole reads as a stick from the floor (its shaft is a third of a yard; the 0.8 is its end
/// fitting): three are slung together, two side by side and one on them (side, up, in yards).
pub(super) const LOG_BUNDLE: [(f32, f32); 3] = [(-0.19, 0.0), (0.19, 0.0), (0.0, 0.31)];
/// How high it hangs over the post's floor: over the machine's head (5 yd), under the mast.
const LOG_UP: f32 = 6.0;
/// Where its rope is made fast overhead: this far over the floor, a little behind the log.
const ROPE_UP: f32 = 7.6;
const ROPE_BACK: f32 = 0.7;
/// The drop on the floor: the log's length, this far to each side of its middle line (the log
/// is 0.4 yd to a side; the rest is how far it rolls and what a body adds).
pub(super) const ZONE_HALF: f32 = 1.6;
/// The tie-off: a stake (ExcavationStake, 0.55 yd tall, at 3 times its size: waist high) this
/// far past the drop's middle along the log and this far ahead of it. 4.8 yd is 3.2 yd outside
/// the drop: whoever cuts it stands clear of the log.
const STAKE: u32 = 2270;
const STAKE_SCALE: f32 = 3.0;
const TIE_ALONG: f32 = 2.0;
const TIE_AHEAD: f32 = 4.8;
/// Cut, the log creaks this long before it comes down: a hero in the drop's middle who moves at
/// the creak is out either side (1.6 yd at 7 yd/s, 0.25 s, after a 0.3 s start).
pub(super) const CREAK: f32 = 0.7;
/// Its fall: 6 yd, seen coming down.
pub(super) const FALL: f32 = 0.3;
/// Whoever is under it is stunned this long: Sneed 4 s (as Gilnid's reel at his valve), a hero
/// half of it (the heroes' share of any stun), time the machine uses for one blow at most. The
/// machine runs its own stall.
pub(super) const LOG_STUN: f32 = 4.0;
/// The crash's and the fallen log's lumber (DeadMineLumberPileSmall, loose planks), and their
/// sounds: wood struck (a pole's crack), a heavy thud, the creak (the mine's door).
const LUMBER: u32 = 1108;
const PLANK: u32 = 6481;
const WOOD_SOUND: u32 = 3720;
const THUD_SOUND: u32 = 921;
const CREAK_SOUND: u32 = 3360;
/// A spell's dust laid at a spot (the falling mine's).
const DUST: u32 = 22792;
/// The drop's mark: quiet from the pull, bright from the cut until the log lands (Gilnid's
/// channel's cadence and colours).
const QUIET: &str = "8FB4D8";
const SOON: &str = "FFF2C8";
const BRIGHT: &str = "FFB23A";
const QUIET_EVERY: f32 = 0.5;
const QUIET_SECS: f32 = 1.0;
const BRIGHT_EVERY: f32 = 0.3;
const BRIGHT_SECS: f32 = 0.6;
/// The ropes are an event, not an object: sent again this often for a hero who came in late.
const ROPE_EVERY: f32 = 6.0;
const LINE_BREAK: &str = "No, no, not now!";
const LINE_LOG: &str = "Gah! Who cut that loose?!";

fn off(p: Vec3, yaw: f32, d: f32) -> Vec3 {
    [p[0] + yaw.cos() * d, p[1] + yaw.sin() * d, p[2]]
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub(super) enum RigState {
    Hung,
    Cut { creak_until: f32, lands_at: f32 },
    Down,
}

/// A post's fixed places and its rigged log.
#[derive(Clone)]
pub(super) struct Rig {
    pub(super) home: Vec3,
    /// The way the machine faces at its work, and the way the log lies (to its right).
    pub(super) face: f32,
    pub(super) across: f32,
    /// Where the wreck lies, and the drop's start on the floor.
    pub(super) crash: Vec3,
    /// Where Sneed comes out of the wreck, where he mends it, and the tie-off.
    pub(super) crawl: Option<Vec3>,
    pub(super) work: Option<Vec3>,
    pub(super) tie: Option<Vec3>,
    /// The drop's mark, in runs of one floor height: where each starts and how long it is.
    marks: Vec<(Vec3, f32)>,
    pub(super) log: Option<Id>,
    /// The poles slung with it (`LOG_BUNDLE`'s second and third).
    pub(super) bundle: Vec<Id>,
    pub(super) stake: Option<Id>,
    /// The tie rope first, then the log's two slings.
    pub(super) ropes: Vec<Id>,
    pub(super) state: RigState,
    mark_at: f32,
    rope_at: f32,
    /// What the crash and the fallen log left lying.
    pub(super) lumber: Vec<Id>,
    /// The crash's lumber is already down (a rebuilt machine crashes into the same pile).
    crashed: bool,
    /// Sneed fallen: the lumber and the fallen log go with his body.
    clear_at: Option<f32>,
    /// A fight has been on here since it was last rigged: a wipe rigs it again.
    fought: bool,
}

/// The rig as a test or the log sees it.
#[derive(Clone, Debug, PartialEq)]
pub struct ShredderRig {
    pub home: Vec3,
    pub face: f32,
    pub crash: Vec3,
    pub crawl: Option<Vec3>,
    pub work: Option<Vec3>,
    pub tie: Option<Vec3>,
    /// The drop: from `crash` along `across`, `len` long, `half` to each side.
    pub across: f32,
    pub len: f32,
    pub half: f32,
    pub log: Option<Id>,
    pub stake: Option<Id>,
    /// What lies on the floor from the crash and the fallen log.
    pub lumber: Vec<Id>,
    /// `hung`, `cut` or `down`.
    pub state: &'static str,
}

/// A broken machine on its way back to its post.
#[derive(Clone, Copy)]
pub(super) struct Runaway {
    by: Id,
    stage: Dash,
    fx_at: f32,
    fx: u8,
    speed: f32,
    limit: f32,
    again_at: f32,
}

#[derive(Clone, Copy, PartialEq)]
enum Dash {
    Sputter { until: f32 },
    Home,
    Lunge,
}

impl Survival {
    /// The rig of the post `id` (the machine or Sneed) belongs to.
    pub(super) fn rig_of(&self, id: Id) -> Option<usize> {
        let home = self.mobs.get(&id)?.home;
        self.machinery.rigs.iter().find(|(_, g)| dist2d(g.home, home) <= 1.0).map(|(k, _)| *k)
    }

    /// Where the fight's second half stands (for the tests on the real floor): the machine
    /// whole, on its run back, Sneed inside the wreck, crawling out of it, on foot, on his way
    /// to mend it, mending it, or done with it.
    pub fn shredder_stage(&self) -> &'static str {
        if !self.machinery.runaways.is_empty() {
            return "runaway";
        }
        let Some(d) = self.machinery.drivers.values().next() else { return "machine" };
        match (d.emerge, d.stage) {
            (Some(e), _) if self.clock < e.out_at => "inside",
            (Some(_), _) => "crawling",
            (None, RepairStage::Fighting { .. }) => "on foot",
            (None, RepairStage::Seeking { .. }) => "to wreck",
            (None, RepairStage::Working { .. }) => "mending",
            (None, RepairStage::Spent) => "spent",
        }
    }

    /// The first post's rig (there is one machine).
    pub fn shredder_rig(&self) -> Option<ShredderRig> {
        self.machinery.rigs.values().next().map(|g| ShredderRig {
            home: g.home,
            face: g.face,
            crash: g.crash,
            crawl: g.crawl,
            work: g.work,
            tie: g.tie,
            across: g.across,
            len: LOG_LEN,
            half: ZONE_HALF,
            log: g.log,
            stake: g.stake,
            lumber: g.lumber.clone(),
            state: match g.state { RigState::Hung => "hung", RigState::Cut { .. } => "cut", RigState::Down => "down" },
        })
    }

    /// Whether `id` is one of the rig's or the crash's props, and its tilt: drawn at once with no
    /// hull, moved in place as it falls (`Survival::collapse_piece_tilt`).
    pub(crate) fn rig_piece_tilt(&self, id: Id) -> Option<f32> {
        self.machinery.tilts.get(&id).copied()
    }

    /// The broken machine on its run back: nothing reaches it, and it strikes nobody.
    pub fn machinery_runaway(&self, id: Id) -> bool {
        self.machinery.runaways.contains_key(&id)
    }

    fn rig_prop(&mut self, at: Vec3, display: u32, scale: f32, facing: f32, tilt: f32) -> Id {
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Prop { display, scale }, pos: at, facing });
        self.machinery.tilts.insert(id, tilt);
        self.events.push(Event::ObjSpawned { id });
        id
    }

    fn rig_prop_gone(&mut self, id: Id) {
        self.machinery.tilts.remove(&id);
        if self.objs.remove(&id).is_some() {
            self.events.push(Event::ObjGone { id });
        }
    }

    fn rig_dust(&mut self, by: Id, at: Vec3) {
        let anchor = self.alloc();
        self.events.push(Event::MissileFrom { anchor, caster: by, spell: DUST, from: at, target: Target::Point(at) });
        self.events.push(Event::ObjGone { id: anchor });
    }

    /// Dry footing at a spot on the post's own floor (within a step and a half of it).
    fn rig_floor(&self, home: Vec3, p: Vec3) -> Option<Vec3> {
        let g = self.ground_near(p[0], p[1], home[2]);
        self.land.footing(g).filter(|f| (f[2] - home[2]).abs() <= 1.5)
    }

    /// The post's rig, made the first time its machine is seen there (and again after a lamp
    /// rest): the places measured from the post, the log hung.
    pub(super) fn rig_ensure(&mut self, machine: Id) {
        let Some(m) = self.mobs.get(&machine) else { return };
        let (home, def, key) = (m.home, m.def, m.pack.unwrap_or(usize::MAX));
        match self.machinery.rigs.get(&key) {
            Some(g) if dist2d(g.home, home) <= 1.0 => return,
            Some(_) => self.rig_remove(key),
            None => {}
        }
        let scale = self.content.mobs[def].scale;
        let driver = self.content.mobs.iter().find(|d| d.key == DRIVER).map_or(1.0, |d| d.scale);
        let face = self.log_facing(home);
        let across = face - std::f32::consts::FRAC_PI_2;
        let crash = self.rig_floor(home, off(home, face, CRASH_AHEAD)).filter(|c| self.land.walk_clear(home, *c)).unwrap_or(home);
        // Sneed mends it at the wreck's right edge (GoblinShredder's Death pose reaches 4.01 yd
        // that way before scale), with room for his body and for a swing at him from outside.
        let reach = 4.01 * scale + 0.5 * driver + 0.5;
        let work = self.rig_floor(home, off(crash, across, reach)).filter(|w| {
            self.rig_floor(home, off(*w, across, 1.2)).is_some_and(|a| (a[2] - w[2]).abs() <= 0.5 && self.land.walk_clear(a, *w))
        });
        // He comes out of the wreck on that side, between its middle and where he'll work, a
        // little toward the pile: on open floor, never inside the wreck or the pile.
        let crawl = self.rig_floor(home, off(off(crash, across, reach - 0.6), face, 0.8));
        // The tie-off, outside the drop: ahead of it first, then behind.
        let mid = off(crash, across, LOG_LEN / 2.0);
        let tie = work.and_then(|w| {
            [(TIE_ALONG, TIE_AHEAD), (0.0, TIE_AHEAD), (TIE_ALONG, -TIE_AHEAD), (0.0, -TIE_AHEAD)].into_iter().find_map(|(a, f)| {
                let t = self.rig_floor(home, off(off(mid, across, a), face, f))?;
                let route = crate::land::walking_route(self.land.as_ref(), &[w, t])?;
                let len: f32 = route.windows(2).map(|p| dist2d(p[0], p[1])).sum();
                (len <= 12.0).then_some(t)
            })
        });
        // The drop's mark follows the floor: a new run wherever it steps up or down.
        let mut marks: Vec<(Vec3, f32)> = Vec::new();
        let mut k = 0.0;
        while k < LOG_LEN {
            let p = off(crash, across, k);
            let p = self.ground_near(p[0], p[1], home[2]);
            let p = if (p[2] - home[2]).abs() <= 1.5 { p } else { [p[0], p[1], home[2]] };
            match marks.last_mut() {
                Some((at, len)) if (at[2] - p[2]).abs() <= 0.3 => *len += 0.5,
                _ => marks.push((p, 0.5)),
            }
            k += 0.5;
        }
        let rigged = work.is_some() && tie.is_some();
        self.machinery.rigs.insert(key, Rig {
            home, face, across, crash, crawl,
            work: work.filter(|_| rigged),
            tie: tie.filter(|_| rigged),
            marks,
            log: None, bundle: Vec::new(), stake: None, ropes: Vec::new(),
            state: RigState::Down,
            mark_at: 0.0, rope_at: 0.0,
            lumber: Vec::new(), crashed: false, clear_at: None, fought: false,
        });
        if rigged {
            self.rig_hang(key);
        } else {
            self.events.push(Event::Log(format!("shredder rig at ({:.1}, {:.1}): no dry place for the work point or the tie-off, no log hung", home[0], home[1])));
        }
    }

    /// Where the log hangs (its origin: the model's own, 5.77 of its length from one end).
    fn rig_log_at(g: &Rig, z: f32) -> Vec3 {
        let p = off(g.crash, g.across, LOG_MODEL_BACK * LOG_SCALE);
        [p[0], p[1], z]
    }

    /// The log's ropes: from overhead to the stake's top, and to the log either side of its middle.
    fn rig_rope_ends(g: &Rig) -> Option<[(Vec3, Vec3); 3]> {
        let tie = g.tie?;
        let mid = off(g.crash, g.across, LOG_LEN / 2.0);
        let top = off(mid, g.face, -ROPE_BACK);
        let top = [top[0], top[1], g.home[2] + ROPE_UP];
        let on_log = |d: f32| { let p = off(mid, g.across, d); [p[0], p[1], g.home[2] + LOG_UP + 0.3] };
        Some([(top, [tie[0], tie[1], tie[2] + 0.55 * STAKE_SCALE]), (top, on_log(-3.0)), (top, on_log(3.0))])
    }

    /// The log hung, the stake stood and the ropes strung, over a floor cleared of what fell.
    fn rig_hang(&mut self, key: usize) {
        self.rig_clear(key);
        let Some(g) = self.machinery.rigs.get(&key).cloned() else { return };
        let Some(tie) = g.tie else { return };
        let at = Self::rig_log_at(&g, g.home[2] + LOG_UP);
        let logs: Vec<Id> = LOG_BUNDLE.iter().map(|(side, up)| {
            let p = off(at, g.face, *side);
            self.rig_prop([p[0], p[1], at[2] + up], LOG, LOG_SCALE, g.across, std::f32::consts::FRAC_PI_2)
        }).collect();
        let log = logs[0];
        let stake = self.rig_prop(tie, STAKE, STAKE_SCALE, g.face, 0.0);
        let ropes: Vec<Id> = (0..3).map(|_| self.alloc()).collect();
        let g = self.machinery.rigs.get_mut(&key).unwrap();
        g.log = Some(log);
        g.bundle = logs[1..].to_vec();
        g.stake = Some(stake);
        g.ropes = ropes;
        g.state = RigState::Hung;
        g.rope_at = 0.0;
        g.mark_at = 0.0;
        g.clear_at = None;
        g.fought = false;
    }

    /// Everything of the rig off the floor and out of the air: the log, the stake, the ropes,
    /// what fell.
    fn rig_clear(&mut self, key: usize) {
        let Some(g) = self.machinery.rigs.get_mut(&key) else { return };
        let props: Vec<Id> = g.log.take().into_iter().chain(std::mem::take(&mut g.bundle)).chain(g.stake.take()).chain(std::mem::take(&mut g.lumber)).collect();
        let ropes = std::mem::take(&mut g.ropes);
        g.state = RigState::Down;
        g.clear_at = None;
        g.crashed = false;
        for id in props { self.rig_prop_gone(id); }
        for id in ropes { self.events.push(Event::MarkGone { id }); }
    }

    fn rig_remove(&mut self, key: usize) {
        self.rig_clear(key);
        self.machinery.rigs.remove(&key);
    }

    /// A lamp rest: every rig goes with the encounter, and is hung again when its machine is back.
    pub(super) fn rigs_gone(&mut self, rigs: BTreeMap<usize, Rig>) {
        for (_, g) in rigs {
            for id in g.log.into_iter().chain(g.bundle).chain(g.stake).chain(g.lumber) {
                if self.objs.remove(&id).is_some() {
                    self.events.push(Event::ObjGone { id });
                }
            }
            for id in g.ropes { self.events.push(Event::MarkGone { id }); }
        }
    }

    /// Sneed fallen: what lies on the floor goes with his body. A log still hung stays hung.
    pub(super) fn rig_body_gone(&mut self, driver: Id, at: f32) {
        if let Some(g) = self.rig_of(driver).and_then(|k| self.machinery.rigs.get_mut(&k)) {
            g.clear_at = Some(at);
        }
    }

    fn in_drop(g: &Rig, p: Vec3, pad: f32) -> bool {
        combat::in_strip(g.crash, g.across, LOG_LEN, ZONE_HALF + pad, p)
    }

    // --- the break: the run back and the crash ----------------------------------------------------

    /// A blow that would break the machine leaves it a breath: it runs back to its post and
    /// crashes into the log pile, and dies there (`runaway_crash`).
    pub(crate) fn machinery_hold(&mut self, mid: Id, by: Id, damage: u32) -> u32 {
        if !self.is_machine(mid) {
            return damage;
        }
        if self.machinery.runaways.contains_key(&mid) {
            return 0;
        }
        let health = self.mobs[&mid].health;
        if health == 0 || damage < health {
            return damage;
        }
        self.start_runaway(mid, by);
        health - 1
    }

    /// Whatever blow it was winding up is called off, and what it held let go.
    fn machine_let_go(&mut self, id: Id) {
        let mark = match self.mobs[&id].state { MobState::Windup { mark, .. } => mark, _ => None };
        if let Some(mk) = mark {
            self.unmark(mk);
        }
        if let Some((_, mk)) = self.mobs.get_mut(&id).and_then(|m| m.lob.take()) {
            self.unmark(mk);
        }
        self.machinery.grinds.remove(&id);
        self.machinery.revs.remove(&id);
        if self.machinery.stalls.remove(&id).is_some() {
            self.freeze_open_pose(id, 0.0, 0.0);
        }
        self.halt(id);
        let m = self.mobs.get_mut(&id).unwrap();
        m.pattern = None;
        m.special = false;
    }

    fn start_runaway(&mut self, id: Id, by: Id) {
        let now = self.clock;
        self.rig_ensure(id);
        self.machine_let_go(id);
        let hero = self.players.contains_key(&by).then_some(Foe::Hero(by));
        let m = self.mobs.get_mut(&id).unwrap();
        if let Some(foe) = m.state.foe().or(hero) {
            m.state = MobState::Recover { foe, until: f32::INFINITY };
        }
        m.ready_at = f32::INFINITY;
        // Out of control, not reeling: no stagger holds it, and the blow that broke it breaks no
        // stance.
        m.stagger_until = 0.0;
        m.posture_guard = f32::INFINITY;
        m.posed_until = 0.0;
        m.lost_at = None;
        let pos = m.pos;
        self.events.push(Event::HoldPose { id, clip: 0, secs: 0.0, from: None });
        // Said as it breaks: the line is what tells the room the run that follows is no attack.
        // Lines of the machine's still waiting are dropped for it. Like every line it keeps
        // 2 s from the one before it (`yell`), so broken within 2 s of "C'mon, you heap of
        // junk!" it is heard during the run rather than at its start.
        self.machinery.lines.clear();
        self.machinery.last_line = self.clock;
        self.yell(id, LINE_BREAK);
        self.shredder_note(&format!("the machine breaks at ({:.1}, {:.1})", pos[0], pos[1]));
        // Its legs churn where it stands, smoke pouring, the engine failing.
        self.events.push(Event::React { id, anim: RUN_CLIP, from: pos, lean: 0.0, hold: SPUTTER, fit: 0.0 });
        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
        self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
        self.events.push(Event::Sound { id, sound: STALL_SOUND });
        self.machinery.runaways.insert(id, Runaway { by, stage: Dash::Sputter { until: now + SPUTTER }, fx_at: now + RUN_FX_EVERY, fx: 0, speed: RUN_SLOWEST, limit: now + SPUTTER + RUN_LIMIT, again_at: 0.0 });
    }

    /// How long the way back is from `pos` and how fast the run: the way's length over
    /// `RUN_BACK`, between its own run and the fastest that reads as one. (Also what the tests
    /// measure.)
    pub fn runaway_pace(&self, pos: Vec3, home: Vec3, crash: Vec3) -> (f32, f32) {
        let to_home = crate::land::walking_route(self.land.as_ref(), &[pos, home])
            .map_or(dist2d(pos, home), |r| r.windows(2).map(|p| dist2d(p[0], p[1])).sum());
        let len = to_home + dist2d(home, crash);
        (len, (len / RUN_BACK).clamp(RUN_SLOWEST, RUN_FASTEST))
    }

    pub(super) fn step_runaway(&mut self, id: Id) {
        let now = self.clock;
        let Some(mut r) = self.machinery.runaways.get(&id).copied() else { return };
        // A lost fight: the run is off. It goes home as any creature does, and is whole there.
        if matches!(self.mobs[&id].state, MobState::Return | MobState::Idle { .. }) || self.next_in_fight(id).is_none() {
            self.machinery.runaways.remove(&id);
            self.halt(id);
            let m = self.mobs.get_mut(&id).unwrap();
            m.state = MobState::Return;
            m.ready_at = now;
            m.posture_guard = 0.0;
            return;
        }
        let pos = self.mobs[&id].pos;
        let (home, crash) = self.rig_of(id).and_then(|k| self.machinery.rigs.get(&k)).map_or((pos, pos), |g| (g.home, g.crash));
        if now >= r.fx_at {
            self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
            if r.fx % 2 == 0 {
                self.events.push(Event::Visual { id, kit: combat::KIT_PARRY, by: None });
            }
            r.fx = r.fx.wrapping_add(1);
            r.fx_at = now + RUN_FX_EVERY;
        }
        let idle = self.mobs[&id].path.is_empty();
        match r.stage {
            Dash::Sputter { until } if now < until => {}
            Dash::Sputter { .. } => {
                r.speed = self.runaway_pace(pos, home, crash).1;
                // At its post already, or at the pile: straight in.
                if dist2d(pos, home) <= 1.0 || dist2d(pos, crash) <= 1.5 {
                    r.stage = Dash::Lunge;
                    self.flee_to(id, crash, r.speed);
                } else {
                    r.stage = Dash::Home;
                    self.go(id, home, r.speed);
                }
                self.events.push(Event::Sound { id, sound: ENGINE_SOUND });
                r.again_at = now + 0.3;
            }
            Dash::Home => {
                if dist2d(pos, home) <= 0.8 {
                    r.stage = Dash::Lunge;
                    self.flee_to(id, crash, r.speed);
                } else if idle && now >= r.again_at {
                    self.go(id, home, r.speed);
                    r.again_at = now + 0.3;
                }
            }
            Dash::Lunge => {
                if dist2d(pos, crash) <= 0.3 {
                    self.runaway_crash(id);
                    return;
                } else if idle && now >= r.again_at {
                    self.flee_to(id, crash, r.speed);
                    r.again_at = now + 0.3;
                }
            }
        }
        if now >= r.limit {
            // No way back in time: set down at the pile under its smoke, so the wreck still lies
            // under the log.
            self.events.push(Event::Log(format!("shredder {id}: no way back to its post from ({:.1}, {:.1}, {:.1}) in {RUN_LIMIT} s, set down at the pile", pos[0], pos[1], pos[2])));
            let m = self.mobs.get_mut(&id).unwrap();
            m.path.clear();
            m.speed = 0.0;
            m.pos = crash;
            self.events.push(Event::MobStop { id, pos: crash, facing: 0.0 });
            self.runaway_crash(id);
            return;
        }
        self.machinery.runaways.insert(id, r);
    }

    /// Into the pile: a bang, dust, lumber thrown off it, and the machine falls where the log
    /// hangs. It dies here, by whoever broke it.
    fn runaway_crash(&mut self, id: Id) {
        let Some(r) = self.machinery.runaways.remove(&id) else { return };
        let key = self.rig_of(id);
        self.halt(id);
        let pos = self.mobs[&id].pos;
        let turned = self.mobs[&id].facing;
        let rig = key.and_then(|k| self.machinery.rigs.get(&k));
        let (face, across) = rig.map_or((turned, turned - std::f32::consts::FRAC_PI_2), |g| (g.face, g.across));
        let crashed = rig.is_none_or(|g| g.crashed);
        self.mobs.get_mut(&id).unwrap().facing = face;
        self.events.push(Event::MobStop { id, pos, facing: face });
        self.events.push(Event::Sound { id, sound: WOOD_SOUND });
        self.events.push(Event::Sound { id, sound: THUD_SOUND });
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::Missile { caster: id, spell: SMOKE, target: Target::Unit(id) });
        // Lumber off the pile, ahead of the wreck and to its left (never where Sneed works):
        // no hull, gone with the encounter.
        for (ahead, side, display, scale, turn, tilt) in [(4.4, 1.9, LUMBER, 0.7, 0.6, 0.0), (5.0, -1.4, PLANK, 1.2, 2.2, 0.45), (3.9, 0.3, PLANK, 1.0, 1.1, -0.3)] {
            let p = off(off(pos, face, ahead), across, -side);
            let p = self.ground_near(p[0], p[1], pos[2]);
            if (p[2] - pos[2]).abs() > 1.5 { continue; }
            self.rig_dust(id, p);
            if !crashed {
                let l = self.rig_prop([p[0], p[1], p[2] + 0.1], display, scale, face + turn, tilt);
                self.machinery.rigs.get_mut(&key.unwrap()).unwrap().lumber.push(l);
            }
        }
        if let Some(g) = key.and_then(|k| self.machinery.rigs.get_mut(&k)) {
            g.crashed = true;
        }
        let m = self.mobs.get_mut(&id).unwrap();
        m.health = 0;
        let max = m.max_health;
        self.events.push(Event::Health { id, health: 0, max });
        self.mob_died(id, r.by);
    }

    /// Sneed comes out of the wreck: scrap and dust burst off it where he lies, a plank thrown
    /// clear (no hull; gone with the rest of the lumber).
    pub(super) fn scrap_burst(&mut self, id: Id, at: Vec3) {
        self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
        self.events.push(Event::Sound { id, sound: WOOD_SOUND });
        self.rig_dust(id, at);
        let wreck = self.machinery.drivers.get(&id).and_then(|d| d.wreck);
        if let Some(w) = wreck {
            self.events.push(Event::Visual { id: w, kit: combat::KIT_PARRY, by: None });
            self.events.push(Event::Missile { caster: w, spell: SMOKE, target: Target::Unit(w) });
        }
        let Some(key) = self.rig_of(id) else { return };
        let g = &self.machinery.rigs[&key];
        if dist2d(at, g.crash) > 8.0 { return; }
        let (face, across) = (g.face, g.across);
        let p = off(off(at, across, 1.4), face, -1.1);
        let p = self.ground_near(p[0], p[1], at[2]);
        if (p[2] - at[2]).abs() > 1.0 { return; }
        let plank = self.rig_prop([p[0], p[1], p[2] + 0.1], PLANK, 0.8, face + 0.7, 0.2);
        self.machinery.rigs.get_mut(&key).unwrap().lumber.push(plank);
    }

    // --- the rigged log ---------------------------------------------------------------------------

    /// A hero's swing: the tie-off in reach and in its arc is cut.
    pub(crate) fn swing_on_rig(&mut self, hero: Id, face: f32, range: f32, arc: f32, foes: f32) {
        let Some((pos, own)) = self.players.get(&hero).map(|p| (p.pos, p.facing)) else { return };
        let cut: Vec<usize> = self.machinery.rigs.iter().filter(|(_, g)| g.state == RigState::Hung && g.tie.is_some_and(|t| {
            dist2d(pos, t) <= range + 0.6 && (pos[2] - t[2]).abs() < 2.0 && angle_between(face, facing(pos, t)) <= arc / 2.0
                && angle_between(own, facing(pos, t)) <= foes
                && crate::land::clear_line(self.land.as_ref(), pos, t)
        })).map(|(k, _)| *k).collect();
        for key in cut {
            self.rig_cut(key);
        }
    }

    /// The rope parts at the stake: the log creaks, its drop on the floor bright, and comes down.
    pub(super) fn rig_cut(&mut self, key: usize) {
        let now = self.clock;
        let Some(g) = self.machinery.rigs.get_mut(&key) else { return };
        if g.state != RigState::Hung { return; }
        g.state = RigState::Cut { creak_until: now + CREAK, lands_at: now + CREAK + FALL };
        self.events.push(Event::Log(format!("shredder {now:.2} s: the log's rope is cut")));
        g.mark_at = now;
        let (stake, log, tie_rope) = (g.stake, g.log, g.ropes.first().copied());
        if let Some(rope) = tie_rope {
            self.events.push(Event::MarkGone { id: rope });
        }
        // The stake is knocked flat, chips flying.
        if let Some(st) = stake {
            self.machinery.tilts.insert(st, 1.3);
            self.events.push(Event::ObjChanged { id: st });
            self.events.push(Event::Sound { id: st, sound: WOOD_SOUND });
            if let Some(at) = self.objs.get(&st).map(|o| o.pos) {
                self.rig_dust(st, at);
            }
        }
        if let Some(l) = log {
            self.events.push(Event::Sound { id: l, sound: CREAK_SOUND });
        }
    }

    pub(super) fn step_rigs(&mut self) {
        if self.machinery.rigs.is_empty() {
            return;
        }
        let keys: Vec<usize> = self.machinery.rigs.keys().copied().collect();
        for key in keys {
            self.step_rig(key);
        }
    }

    fn step_rig(&mut self, key: usize) {
        let now = self.clock;
        let g = &self.machinery.rigs[&key];
        let home = g.home;
        // Who is here: the machine at rest (a wipe, or nobody yet), Sneed, a fight on.
        let (mut fighting, mut resting, mut driver, mut mending) = (false, false, false, false);
        for (id, m) in &self.mobs {
            if m.health == 0 || dist2d(m.home, home) > 1.0 { continue; }
            let k = self.content.mobs[m.def].key.as_str();
            if k != MACHINE && k != DRIVER { continue; }
            fighting |= m.state.foe().is_some();
            resting |= k == MACHINE && matches!(m.state, MobState::Idle { .. } | MobState::Return);
            driver |= k == DRIVER;
            mending |= self.machinery.drivers.get(id).is_some_and(|d| matches!(d.stage, RepairStage::Seeking { .. } | RepairStage::Working { .. }));
        }
        // A lost fight, at any stage: the machine is back at its post, and the log is hung again
        // over a clean floor at once.
        if g.fought && resting && !driver && !fighting {
            let dirty = g.state != RigState::Hung || !g.lumber.is_empty();
            if dirty && g.tie.is_some() {
                self.rig_hang(key);
            } else if dirty {
                self.rig_clear(key);
            }
            self.machinery.rigs.get_mut(&key).unwrap().fought = false;
            return;
        }
        if fighting {
            self.machinery.rigs.get_mut(&key).unwrap().fought = true;
        }
        let g = self.machinery.rigs[&key].clone();
        if g.clear_at.is_some_and(|t| now >= t) {
            // With Sneed's body: the lumber, and the log if it fell.
            let fallen = if g.state == RigState::Down { g.log } else { None };
            let bundle = if fallen.is_some() { g.bundle.clone() } else { Vec::new() };
            for id in g.lumber.iter().copied().chain(fallen).chain(bundle) { self.rig_prop_gone(id); }
            let g = self.machinery.rigs.get_mut(&key).unwrap();
            g.lumber.clear();
            if fallen.is_some() { g.log = None; g.bundle.clear(); }
            g.clear_at = None;
            return;
        }
        match g.state {
            RigState::Hung => {
                if now >= g.rope_at && self.players.values().any(|p| dist2d(p.pos, home) < 80.0) {
                    if let Some(ends) = Self::rig_rope_ends(&g) {
                        for (id, (a, b)) in g.ropes.iter().zip(ends) {
                            self.events.push(Event::Rope { id: *id, a, b });
                        }
                    }
                    self.machinery.rigs.get_mut(&key).unwrap().rope_at = now + ROPE_EVERY;
                }
                // From the pull: the drop on the floor, quiet, and a ring at the tie-off (lit
                // while Sneed is at his repair: this is what stops it).
                if fighting && now >= g.mark_at {
                    self.rig_mark(&g, QUIET, QUIET_SECS);
                    if let Some(at) = g.tie {
                        let colour = if mending { SOON } else { QUIET };
                        self.events.push(Event::GroundBurst { at, yaw: 0.0, shape: "ring", reach: 1.2, arc: 360.0, width: 0.0, secs: QUIET_SECS, colour: colour.into() });
                    }
                    self.machinery.rigs.get_mut(&key).unwrap().mark_at = now + QUIET_EVERY;
                }
            }
            RigState::Cut { creak_until, lands_at } => {
                if now >= lands_at {
                    self.rig_land(key);
                    return;
                }
                if now >= g.mark_at {
                    self.rig_mark(&g, BRIGHT, BRIGHT_SECS.min(lands_at - now + 0.1));
                    self.machinery.rigs.get_mut(&key).unwrap().mark_at = now + BRIGHT_EVERY;
                }
                if now >= creak_until {
                    // Coming down: its slings let go, the log falls faster as it goes.
                    for rope in g.ropes.iter().skip(1) {
                        self.events.push(Event::MarkGone { id: *rope });
                    }
                    self.machinery.rigs.get_mut(&key).unwrap().ropes.truncate(1);
                    let t = ((now - creak_until) / FALL).clamp(0.0, 1.0);
                    let (top, floor) = (g.home[2] + LOG_UP, self.rig_log_floor(&g));
                    self.rig_log_to(&g, top - (top - floor) * t * t);
                }
            }
            RigState::Down => {}
        }
    }

    /// The log and the poles slung with it, this high.
    fn rig_log_to(&mut self, g: &Rig, z: f32) {
        for (id, (_, up)) in g.log.into_iter().chain(g.bundle.iter().copied()).zip(LOG_BUNDLE) {
            if let Some(o) = self.objs.get_mut(&id) {
                o.pos[2] = z + up;
                self.events.push(Event::ObjChanged { id });
            }
        }
    }

    /// Where the fallen log lies: on the floor where Sneed works (its far end rests on the wreck).
    fn rig_log_floor(&self, g: &Rig) -> f32 {
        g.work.map_or(g.crash[2], |w| w[2]) + 0.3
    }

    fn rig_mark(&mut self, g: &Rig, colour: &str, secs: f32) {
        for (at, len) in &g.marks {
            self.events.push(Event::GroundBurst { at: *at, yaw: g.across, shape: "line", reach: *len, arc: 0.0, width: ZONE_HALF * 2.0, secs, colour: colour.into() });
        }
    }

    /// The log lands. Whoever is under it is stunned: a hero (no damage), the machine (its own
    /// stall), Sneed (4 s open, and his repair stopped for good). The log lies where it fell with
    /// no hull: it walls nobody in.
    fn rig_land(&mut self, key: usize) {
        let now = self.clock;
        let Some(g) = self.machinery.rigs.get(&key).cloned() else { return };
        for rope in &g.ropes {
            self.events.push(Event::MarkGone { id: *rope });
        }
        let floor = self.rig_log_floor(&g);
        self.rig_log_to(&g, floor);
        if let Some(l) = g.log {
            self.events.push(Event::Sound { id: l, sound: THUD_SOUND });
            self.events.push(Event::Sound { id: l, sound: WOOD_SOUND });
            for d in [1.5, LOG_LEN / 2.0, LOG_LEN - 1.5] {
                let p = off(g.crash, g.across, d);
                let p = self.ground_near(p[0], p[1], g.home[2]);
                self.rig_dust(l, p);
            }
        }
        // Splinters off it either side.
        let mut lumber = Vec::new();
        for (d, side, turn) in [(2.6, 1.0, 0.4), (7.0, -0.9, 1.9)] {
            let p = off(off(g.crash, g.across, d), g.face, side);
            let p = self.ground_near(p[0], p[1], g.home[2]);
            if (p[2] - g.home[2]).abs() > 1.5 { continue; }
            lumber.push(self.rig_prop([p[0], p[1], p[2] + 0.1], PLANK, 0.9, g.across + turn, 0.0));
        }
        let r = self.machinery.rigs.get_mut(&key).unwrap();
        r.state = RigState::Down;
        r.ropes.clear();
        r.lumber.extend(lumber);
        // The post's own creatures: who the heroes' stun is laid to, and who may be under it.
        let here: Vec<(Id, bool, Vec3)> = self.mobs.iter()
            .filter(|(_, m)| m.health > 0 && dist2d(m.home, g.home) <= 1.0)
            .filter_map(|(id, m)| {
                let k = self.content.mobs[m.def].key.as_str();
                (k == MACHINE || k == DRIVER).then_some((*id, k == MACHINE, m.pos))
            }).collect();
        if here.is_empty() {
            // Cut loose with the fight already won: it lies a moment and is cleared away.
            self.machinery.rigs.get_mut(&key).unwrap().clear_at = Some(now + BODY_SECS);
        }
        let heroes: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && Self::in_drop(&g, p.pos, 0.0)).map(|(h, _)| *h).collect();
        for h in heroes {
            let owner = here.first().map_or(h, |(id, _, _)| *id);
            self.knock_down(owner, h, LOG_STUN, "Falling log");
        }
        for (id, machine, pos) in here {
            if self.mobs[&id].hidden || self.machinery_shielded(id) || !Self::in_drop(&g, pos, if machine { 1.0 } else { 0.5 }) {
                continue;
            }
            self.events.push(Event::Visual { id, kit: combat::KIT_DUST, by: None });
            if machine {
                // Its engine stalls under it: the stall it has, nothing new.
                if self.mobs[&id].state.foe().is_some() {
                    self.machine_let_go(id);
                    self.stall_machine(id, pos);
                }
            } else {
                // Sneed, flattened: he's open, and a repair in hand is over.
                if self.mending(id) {
                    self.abandon_repair(id, None);
                }
                // His own cry, at once, over whatever he was saying (filmed: behind "She's
                // turning over!" it came 2 s after the log had flattened him).
                self.machinery.lines.clear();
                self.machinery.last_line = now;
                self.say_line(id, LINE_LOG.to_string());
                self.shredder_note("the log lands on Sneed");
                self.events.push(Event::CastEnd { caster: id });
                self.stagger_mob(id, LOG_STUN, true);
                if let Some(m) = self.mobs.get_mut(&id) {
                    m.posture = 0;
                    m.posture_guard = now + LOG_STUN + super::super::mobs::POSTURE_GRACE;
                }
            }
        }
    }
}

#[cfg(test)]
#[path = "machinery_rig_tests.rs"]
mod tests;
