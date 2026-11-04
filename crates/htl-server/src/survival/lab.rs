//! The spell lab (`.es lab`, dev builds only): cast an ability the same way every time and see
//! exactly what it looked like.
//!
//! `.es lab <ability> [scene] [creature] [do <steps>] [shots <n>@<ms>] [cam behind|close|none] [stage here] [look <spell>[/<aura>]] [as <race>-<m|f>] [allies <n>]`
//!
//! - `as tauren-f` makes the hero that race and gender (plain looks) for this run and the ones after;
//!   `allies 2` stands that many stand-in ally heroes beside them (gone with the next run).
//! - `look` tries another of the game's looks on the ability for that run only (its spell, and
//!   its lasting aura's, the same unless given), put back after.
//! - Each run starts on the lab's stage, the nearest open level ground to the camp's spawn (no
//!   tree, wall or water within 22 yd), facing the spawn's way; `stage here` keeps the hero
//!   where they stand.
//! - It sets out the scene in front of the hero (`one`, `pack`, `close`, `ring`, `far`; see
//!   `htl_survival::sim::lab`), of training dummies or of a creature (`brave`), puts the ability
//!   on its key with cooldowns off, full Valor and full health.
//! - It tells the add-on to take a burst of screenshots (`LB\t<n>\t<ms>\t<label>`; the add-on's
//!   `Lab.lua`), and a moment later plays the steps: by default the ability's key (with Ctrl for
//!   an EX) and a click if it's aimed. Steps, comma-separated: `q e 1 2 3` (a key), `^q`... (with
//!   Ctrl), `click`, `attack`, `heavy`, `guard`, `guardup`, `dodge`, `bash`, `blow` (the nearest
//!   creature of the scene swings at the hero), `wait:<secs>`.
//! - Everything the rules sent that touches the hero or the scene, until the burst is over, goes
//!   to `logs/lab/<label>.txt`, each line with its time from the first step. `tools/spell-lab`
//!   lays the screenshots and that trace out side by side (docs/spell-lab.md).
//!
//! `.es lab clear` takes the scene away.

use std::collections::VecDeque;
use std::io::Write as _;

use htl_survival::{Event, Id, Input};

use super::*;

/// The burst unless the command says otherwise: 2.4 sec of shots, 12 a second.
const SHOTS: u32 = 30;
const EVERY_MS: u32 = 80;
/// With a video of the run, the stills for its sheet: 8, every 300 ms (the same 2.4 sec).
const VIDEO_STILLS: u32 = 8;
const VIDEO_EVERY_MS: u32 = 300;
/// The shots start this long before the first step, so the frame before it is in the burst.
const LEAD: f32 = 0.25;
/// The scene stands this long before the burst: a body just set out fades in on the client over
/// about a second (seen in the lab: the brave's wind-up was invisible), and the lock-on camera
/// settles.
const SETTLE: f32 = 1.5;
/// And this much more when the run first sets the hero down on the lab's stage (far from where
/// they stood, the client shows its loading screen first).
const MOVED_SETTLE: f32 = 5.0;

pub(crate) struct Lab {
    pub hero: Id,
    pub conn: ConnId,
    pub label: String,
    /// When (game clock) each step goes.
    pub steps: VecDeque<(f32, String)>,
    pub start: f32,
    pub until: f32,
    /// The burst's line to the add-on and the video's length, sent once the scene has settled.
    pub burst: Option<(f32, String, f32)>,
    pub mobs: Vec<Id>,
    pub trace: Vec<String>,
    /// The ability's key on the bar (0-4) and whether its aim is the ground's.
    pub slot: u8,
    pub ground: bool,
    /// Debug was on (its marks are kept out of the shots while the run lasts).
    pub debug_was: bool,
    /// When (wall clock) the burst was asked for, to find its screenshots after.
    pub asked: std::time::SystemTime,
    /// A look tried for this run only (`look <spell>[/<aura>]`): the ability and the spell and
    /// aura it had, put back when the run is over.
    pub look_was: Option<(usize, u32, u32)>,
}

impl SurvivalGame {
    pub(crate) fn lab_command(&mut self, conn: ConnId, me: Id, words: &[&str]) -> Result<(), String> {
        let first = words.first().copied().unwrap_or("");
        if first.is_empty() {
            return Err(format!("Try .es lab <ability> [scene] [creature] [do <steps>] [shots <n>@<ms>]. Scenes: {}.", htl_survival::sim::lab::LAB_SCENES));
        }
        // What the last run set out goes first.
        if let Some(old) = self.lab.take() {
            self.sim.lab_clear(&old.mobs);
            self.sim.debug = old.debug_was;
            self.lab_look_back(old.look_was);
        }
        let old = std::mem::take(&mut self.lab_mobs);
        self.sim.lab_clear(&old);
        if first == "clear" {
            self.lab_set_allies(me, 0);
            self.tell(conn, "The lab is cleared.");
            return Ok(());
        }
        // A night of runs once filled the host's disk: past its budget, the lab stops and says so.
        let used = folder_bytes(&lab_dir());
        if used > LAB_BUDGET {
            return Err(format!(
                "The lab's runs take {:.1} GB, over its {:.0} GB budget. Delete old run folders in {}, then run it again.",
                used as f64 / 1e9,
                LAB_BUDGET as f64 / 1e9,
                lab_dir().display()
            ));
        }
        let key = first.to_lowercase();
        let ability = self.sim.content.ability(&key).ok_or_else(|| format!("No ability called {key}."))?;
        let d = self.sim.content.abilities[ability].clone();
        let base = self.sim.content.base_of(ability);
        let ex = base != ability;
        let row = self.sim.content.abilities[base].slot.clone();
        let slot = htl_survival::content::SLOTS.iter().position(|s| *s == row).ok_or("That ability has no key.")? as u8;
        // The words after the ability: a scene, a creature, then `do` and `shots`.
        let mut scene = if d.kind == "stance" || d.kind == "art" { "close" } else { "pack" }.to_string();
        let mut creature = if d.kind == "stance" { "brave" } else { "dummy" }.to_string();
        let mut steps: Option<String> = None;
        let mut cam: Option<String> = None;
        let mut stage = true;
        let mut look: Option<(u32, Option<u32>)> = None;
        let mut body: Option<(u8, u8)> = None;
        let mut allies = 0usize;
        // With video on this computer, the stills are only the sheet's: a few, spread over the run.
        let video = cfg!(windows) && ffmpeg().is_some();
        let (mut shots, mut every) = if video { (VIDEO_STILLS, VIDEO_EVERY_MS) } else { (SHOTS, EVERY_MS) };
        let mut k = 1;
        let mut free = 0;
        while k < words.len() {
            match words[k] {
                "do" => {
                    steps = words.get(k + 1).map(|s| s.to_string());
                    k += 1;
                }
                // A look to try for this run only: `look 3631` (its spell and aura both),
                // `look 12294/3631` (the spell's, then the aura's).
                "look" => {
                    let spec = words.get(k + 1).copied().unwrap_or("");
                    let mut it = spec.split('/');
                    let sp = it.next().and_then(|n| n.parse().ok()).ok_or("Try look <spell>[/<aura>].")?;
                    look = Some((sp, it.next().and_then(|n| n.parse().ok())));
                    k += 1;
                }
                // The hero's race and gender for this run and the ones after (`as tauren-f`): every
                // race's take on an ability, recorded one after another.
                "as" => {
                    let spec = words.get(k + 1).copied().unwrap_or("");
                    body = Some(race_gender(spec).ok_or_else(|| format!("No race called {spec}. Try as <{}>-<m|f>.", RACE_NAMES.join("|")))?);
                    k += 1;
                }
                // Stand-in ally heroes beside the hero (`allies 2`), for what reaches allies.
                "allies" => {
                    allies = words.get(k + 1).and_then(|n| n.parse().ok()).unwrap_or(2).min(4);
                    k += 1;
                }
                "stage" => {
                    stage = words.get(k + 1).is_none_or(|w| *w != "here");
                    k += 1;
                }
                "cam" => {
                    cam = words.get(k + 1).map(|w| w.to_lowercase());
                    k += 1;
                }
                "shots" => {
                    let spec = words.get(k + 1).copied().unwrap_or("");
                    let mut it = spec.split('@');
                    shots = it.next().and_then(|n| n.parse().ok()).unwrap_or(SHOTS).clamp(1, 120);
                    every = it.next().and_then(|n| n.parse().ok()).unwrap_or(EVERY_MS).clamp(16, 1000);
                    k += 1;
                }
                w if free == 0 => {
                    scene = w.to_lowercase();
                    free += 1;
                }
                w => {
                    creature = w.to_lowercase();
                    free += 1;
                }
            }
            k += 1;
        }
        let keyname = ["q", "e", "1", "2", "3"].get(slot as usize).copied().unwrap_or("q");
        let steps = steps.unwrap_or_else(|| format!("{}{keyname},click?", if ex { "^" } else { "" }));
        // The hero ready: the ability on its key, nothing cooling down, full Valor and health.
        let base_key = self.sim.content.abilities[base].key.clone();
        if self.sim.slot_ability(me, &row, &base_key).is_err() {
            // Not learned yet: the lab's hero gets the whole board.
            self.sim.set_tree(me, true);
            self.sim.slot_ability(me, &row, &base_key)?;
        }
        self.sim.lab_reset(me);
        self.sim.set_cooldowns(me, true);
        self.sim.set_valor(me, 100.0);
        self.sim.heal_full(me);
        let reshaped = body.is_some_and(|(race, gender)| self.lab_reshape(me, race, gender));
        // Every run from the same open ground, facing the same way (`stage here` keeps the hero
        // where they stand): a run that started wherever the last left the hero was once framed
        // through a tree.
        let moved = if stage {
            if self.lab_stage.is_none() {
                let spawn = self.sim.content.world.spawn;
                self.lab_stage = self.sim.lab_stage(spawn, &[]);
                if let Some((at, f)) = self.lab_stage {
                    eprintln!("[lab] the lab's stage: {at:?} facing {f:.2}");
                }
            }
            self.lab_stage.is_some_and(|st| self.sim.lab_to_stage(me, st))
        } else {
            false
        };
        let look_was = look.map(|(sp, au)| {
            let a = &mut self.sim.content.abilities[ability];
            let was = (ability, a.spell, a.aura);
            a.spell = sp;
            a.aura = au.unwrap_or(sp);
            was
        });
        let d = self.sim.content.abilities[ability].clone();
        let mobs = self.sim.lab_scene(me, &scene, &creature)?;
        self.lab_mobs = mobs.clone();
        self.lab_set_allies(me, allies);
        // The steps, timed from a moment after the burst starts (and after the client has set the
        // hero down, if the stage moved them).
        let now = self.sim.clock + if moved || reshaped { MOVED_SETTLE } else { 0.0 };
        let start = now + SETTLE + LEAD;
        let mut at = start;
        let mut queue = VecDeque::new();
        for s in steps.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(w) = s.strip_prefix("wait:") {
                at += w.parse::<f32>().unwrap_or(0.0).clamp(0.0, 10.0);
            } else {
                queue.push_back((at, s.to_string()));
                at += 0.05;
            }
        }
        let secs = shots as f32 * every as f32 / 1000.0;
        let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_secs()).unwrap_or(0);
        let label = format!("{key}-{scene}-{stamp}");
        // The camera the add-on puts the run in: nearer for a scene at arm's length, else behind
        // (the hero is locked on to the scene, so either way it's in front of them).
        let cam = cam.unwrap_or_else(|| if matches!(scene.as_str(), "close" | "ring") { "close" } else { "behind" }.to_string());
        let mut trace = vec![
            format!("# {label}: {} ({}), scene {scene} of {creature}, steps {steps}, {shots} shots every {every} ms from -{LEAD:.2} s", d.name, d.key),
            format!("# spell {} aura {} icon {} clip {} kind {}", d.spell, d.aura, d.icon, d.clip, d.kind),
            format!("# hero {me} at {:?}, scene {:?}", self.sim.players.get(&me).map(|p| p.pos), mobs),
        ];
        trace.push(format!("{:+.3} burst starts", -LEAD));
        self.lab = Some(Lab {
            hero: me,
            conn,
            label: label.clone(),
            steps: queue,
            start,
            until: start + secs - LEAD + 0.3,
            // With video, no stills (the video holds every frame; a night's stills once filled the
            // lab's budget): the shots' count and spacing only time the run, and the add-on keeps
            // the interface hidden for the video's length.
            burst: Some((now + SETTLE, format!("LB\t{}\t{every}\t{label}\t{cam}\t{:.2}", if video { 0 } else { shots }, secs + 0.6), secs + 0.6)),
            mobs,
            trace,
            slot,
            // Aimed at the feet: what lands on a spot (a leap's landing too).
            ground: matches!(d.kind.as_str(), "ground" | "trap" | "place" | "leap"),
            debug_was: self.sim.debug,
            asked: std::time::SystemTime::now(),
            look_was,
        });
        // Debug's rings and marks would be in every shot: off while the run lasts.
        self.sim.debug = false;
        // Nothing said on screen while it runs: it would be in every frame.
        Ok(())
    }

    /// A lab run asked for in a file instead of in chat (`logs/lab/queue.txt`, one command a line,
    /// as typed after `.es lab`; `@Name` first picks the hero): so a run can be set going on the
    /// computer that hosts, with no one at the keyboard. Only that computer can write the file.
    pub(crate) fn lab_queue(&mut self) {
        if self.lab.is_some() || self.lab_polled.elapsed() < std::time::Duration::from_millis(500) {
            return;
        }
        self.lab_polled = std::time::Instant::now();
        let path = std::path::Path::new("logs").join("lab").join("queue.txt");
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let lines: Vec<&str> = text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')).collect();
        if lines.is_empty() {
            return;
        }
        let line = lines[0].to_string();
        let mut words: Vec<&str> = line.split_whitespace().collect();
        let named = words.first().and_then(|w| w.strip_prefix('@')).map(str::to_lowercase);
        if named.is_some() {
            words.remove(0);
        }
        // `find [missile] <word>... [spell <id>...]`: the game's own spell looks that show a model
        // by that name, written to logs/lab/find-<words>.txt, to pick a look from instead of
        // guessing. Any world takes it; it reads the game data off the rules' thread.
        if words.first() == Some(&"find") {
            let rest = lines[1..].join("\n");
            let _ = std::fs::write(&path, if rest.is_empty() { String::new() } else { rest + "\n" });
            let args: Vec<String> = words[1..].iter().map(|w| w.to_string()).collect();
            std::thread::spawn(move || lab_find(&args));
            return;
        }
        // Every world the server runs (Elwynn, each dungeon below) reads this file: the line is
        // left for the one the hero is in.
        let who = self.in_world().find(|(_, _, w)| named.as_ref().is_none_or(|n| w.name.to_lowercase() == *n)).map(|(c, _, w)| (*c, w.id));
        let Some((conn, me)) = who else { return };
        let rest = lines[1..].join("\n");
        let _ = std::fs::write(&path, if rest.is_empty() { String::new() } else { rest + "\n" });
        if let Err(e) = self.lab_command(conn, me, &words) {
            eprintln!("[lab] queued run failed: {line}: {e}");
            self.tell(conn, &format!("Lab: {e}"));
        }
    }

    /// Before the rules step: the lab's steps that are due.
    pub(crate) fn lab_steps(&mut self) {
        let now = self.sim.clock;
        if !self.lab_mobs.is_empty() {
            let mobs = self.lab_mobs.clone();
            self.sim.lab_hold(&mobs);
        }
        let Some(lab) = self.lab.as_mut() else { return };
        if lab.burst.as_ref().is_some_and(|(at, _, _)| *at <= now) {
            let (_, line, secs) = lab.burst.take().unwrap();
            let (conn, label, hero, scene) = (lab.conn, lab.label.clone(), lab.hero, lab.mobs.clone());
            self.sim.lab_lock(hero, &scene);
            let Some(lab) = self.lab.as_mut() else { return };
            lab.asked = std::time::SystemTime::now();
            self.ui_line(conn, &line);
            record_video(&label, secs);
        }
        let Some(lab) = self.lab.as_mut() else { return };
        let mut due = Vec::new();
        while lab.steps.front().is_some_and(|(at, _)| *at <= now) {
            due.push(lab.steps.pop_front().unwrap().1);
        }
        let (me, start, slot, ground) = (lab.hero, lab.start, lab.slot, lab.ground);
        let target = lab.mobs.get(lab.mobs.len() / 2).copied();
        let nearest = lab.mobs.first().copied();
        for step in due {
            // Aimed at the scene's middle body: its chest, or its feet for the ground's.
            if let Some(t) = target.and_then(|t| self.sim.mobs.get(&t)).map(|m| m.pos) {
                self.sim.lab_aim_at(me, if ground { t } else { [t[0], t[1], t[2] + 1.0] });
            }
            let input = match step.as_str() {
                "click" | "attack" => Some(Input::Attack),
                "click?" if self.sim.lab_aiming(me) => Some(Input::Attack),
                "click?" => None,
                "heavy" => Some(Input::Heavy),
                "guard" => Some(Input::Guard),
                "guardup" => Some(Input::GuardUp),
                "dodge" => Some(Input::Dodge),
                "bash" => Some(Input::Bash),
                "blow" => {
                    if let Some(m) = nearest {
                        self.sim.lab_blow(m, me);
                    }
                    None
                }
                k => {
                    let (ex, k) = match k.strip_prefix('^') {
                        Some(rest) => (true, rest),
                        None => (false, k),
                    };
                    ["q", "e", "1", "2", "3"].iter().position(|x| *x == k).map(|s| Input::Ability(s as u8, ex)).or(if k == "key" { Some(Input::Ability(slot, false)) } else { None })
                }
            };
            if let Some(i) = input {
                self.sim.input(me, i);
                // A click is let go at once, as a tap.
                if i == Input::Attack {
                    self.sim.input(me, Input::AttackUp);
                }
            }
            let what = self.sim.lab_action(me);
            if let Some(lab) = self.lab.as_mut() {
                lab.trace.push(format!("{:+.3} STEP {step} -> {what}", now - start));
            }
        }
    }

    /// After the rules step: what they sent that touches the hero or the scene, until the burst
    /// is over; then the trace is written.
    pub(crate) fn lab_trace(&mut self, events: &[Event]) {
        let now = self.sim.clock;
        let Some(lab) = self.lab.as_mut() else { return };
        let mut ids: Vec<Id> = lab.mobs.clone();
        ids.push(lab.hero);
        for e in events {
            let text = format!("{e:?}");
            if mentions(&text, &ids) {
                lab.trace.push(format!("{:+.3} {text}", now - lab.start));
            }
        }
        if now < lab.until {
            return;
        }
        let lab = self.lab.take().unwrap();
        self.sim.debug = lab.debug_was;
        self.lab_look_back(lab.look_was);
        let dir = lab_dir();
        let path = dir.join(format!("{}.txt", lab.label));
        let written = std::fs::create_dir_all(&dir).and_then(|_| std::fs::File::create(&path)).and_then(|mut f| f.write_all((lab.trace.join("\n") + "\n").as_bytes()));
        // The run's screenshots, copied beside its trace once the client has written them all.
        // Only this run's: a queued run that starts at once takes its own after it.
        let since = lab.asked;
        let upto = std::time::SystemTime::now() + std::time::Duration::from_millis(400);
        let folder = dir.join(&lab.label);
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_secs(3));
            copy_shots(since, upto, &folder);
        });
        match written {
            // In chat, not on screen: a queued run starting at once would have it in its frames.
            Ok(()) => self.say_to(lab.conn, &format!("Lab done: {}", dir.join(format!("{}.txt", lab.label)).display())),
            Err(e) => self.say_to(lab.conn, &format!("Lab done, but its trace couldn't be written: {e}")),
        }
    }
}

/// The races a lab run can take (`as <race>-<m|f>`), in the game's order (`ChrRaces`, 1 to 8).
const RACE_NAMES: [&str; 8] = ["human", "orc", "dwarf", "nightelf", "undead", "tauren", "gnome", "troll"];

/// `tauren-f` → (6, 1).
fn race_gender(spec: &str) -> Option<(u8, u8)> {
    let (race, gender) = spec.to_lowercase().split_once('-').map(|(r, g)| (r.to_string(), g.to_string()))?;
    let race = RACE_NAMES.iter().position(|r| *r == race.replace(['_', ' '], ""))? as u8 + 1;
    let gender = match gender.as_str() {
        "m" | "male" => 0,
        "f" | "female" => 1,
        _ => return None,
    };
    Some((race, gender))
}

impl SurvivalGame {
    /// The hero takes another race and gender (plain looks), redrawn in place on every client:
    /// true if they changed (their client builds the new body, so the run waits a moment).
    pub(crate) fn lab_reshape(&mut self, id: Id, race: u8, gender: u8) -> bool {
        let Some(p) = self.sim.players.get_mut(&id) else { return false };
        if p.look.race == race && p.look.gender == gender {
            return false;
        }
        p.look = htl_survival::HeroLook { race, gender, skin: 0, face: 0, hair: 0, hair_color: 0, facial: 0 };
        let class = htl_survival::content::class_id_of(&p.class);
        self.lab_redraw(id, race, gender, class);
        true
    }

    /// A hero's body sent again as fields that changed: their race, gender and looks, and the
    /// display that makes the client build the new model.
    fn lab_redraw(&mut self, id: Id, race: u8, gender: u8, class: u8) {
        let Some(g) = self.guid_of(id) else { return };
        let mut f = Fields::new();
        f.set(idx::UNIT_BYTES_0, race as u32 | (class as u32) << 8 | (gender as u32) << 16 | 3 << 24)
            .set(idx::PLAYER_BYTES, 0)
            .set(idx::PLAYER_BYTES_2, 0x02 << 24)
            .set(idx::PLAYER_BYTES_3, gender as u32)
            .set(idx::UNIT_DISPLAYID, super::draw::native_display(race, gender))
            .set(idx::UNIT_NATIVEDISPLAYID, super::draw::native_display(race, gender));
        self.broadcast(&packets::update_object(&[], &[(g, &f)]));
    }

    /// `n` stand-in ally heroes beside hero `id` (the last run's taken away first): to their left,
    /// right and behind, each a different race, still unless something moves them.
    pub(crate) fn lab_set_allies(&mut self, id: Id, n: usize) {
        for a in std::mem::take(&mut self.lab_allies) {
            if let Some(g) = self.guid_of(a) {
                self.broadcast(&packets::destroy_object(g));
            }
            self.guids.remove(&a);
            let _ = self.sim.leave(a);
        }
        let Some((pos, f)) = self.sim.players.get(&id).map(|p| (p.pos, p.facing)) else { return };
        let looks: [(u8, u8); 4] = [(2, 1), (3, 0), (4, 1), (6, 0)];
        let spots: [(f32, f32); 4] = [(3.0, 90.0), (3.0, -90.0), (3.5, 150.0), (3.5, -150.0)];
        for k in 0..n.min(4) {
            let account = format!("{}{k}", htl_survival::sim::lab::LAB_ALLY);
            let a = self.sim.join(&account, &format!("Ally {}", k + 1), None);
            let (d, off) = spots[k];
            let ang = f + off.to_radians();
            let (x, y) = (pos[0] + ang.cos() * d, pos[1] + ang.sin() * d);
            let at = [x, y, self.sim.land().height(x, y)];
            if let Some(p) = self.sim.players.get_mut(&a) {
                let (race, gender) = looks[k];
                p.look = htl_survival::HeroLook { race, gender, skin: 0, face: 0, hair: 0, hair_color: 0, facial: 0 };
            }
            self.sim.move_player(a, at, f, htl_survival::Moving::default());
            self.bind(a, Self::player_guid(&account));
            let p = self.player_create(a, false);
            self.broadcast(&p);
            self.lab_allies.push(a);
        }
    }

    /// Puts back the look a run tried (`look`), if it tried one.
    fn lab_look_back(&mut self, was: Option<(usize, u32, u32)>) {
        if let Some((i, sp, au)) = was {
            if let Some(a) = self.sim.content.abilities.get_mut(i) {
                a.spell = sp;
                a.aura = au;
            }
        }
    }
}

/// ffmpeg on this computer, if it's there: `LAB_FFMPEG`, the lab helper's copy beside this checkout
/// (`Documents\\Basin-Battle-Lab\\ffmpeg`), or one on the path.
fn ffmpeg() -> Option<std::path::PathBuf> {
    let found = [std::env::var("LAB_FFMPEG").ok().map(std::path::PathBuf::from), Some(std::path::PathBuf::from("../Basin-Battle-Lab/ffmpeg/bin/ffmpeg.exe"))];
    found.into_iter().flatten().find(|p| p.is_file()).or_else(|| {
        let ok = std::process::Command::new("ffmpeg").arg("-version").stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()).status().is_ok_and(|s| s.success());
        ok.then(|| std::path::PathBuf::from("ffmpeg"))
    })
}

/// The game's window as video for the run, 60 frames a second, to `logs/lab/<label>.mp4`: a few
/// MB a run where the screenshot burst is tens. By Windows Graphics Capture (ffmpeg's
/// `gfxcapture`), the game's own window whatever stands over it (a window on top was in the
/// desktop's frames); where that can't start, by Desktop Duplication (`ddagrab`) of the primary
/// monitor. GDI's grab (`gdigrab`) saw only a few stale frames of benilla's Vulkan window (#119).
/// Nothing happens without ffmpeg or on another system.
fn record_video(label: &str, secs: f32) {
    if !cfg!(windows) {
        return;
    }
    let Some(ff) = ffmpeg() else { return };
    let label = label.to_string();
    std::thread::spawn(move || {
        let out = lab_dir().join(format!("{label}.mp4"));
        let _ = std::fs::create_dir_all(lab_dir());
        let began = std::time::Instant::now();
        let window = "gfxcapture=window_exe=(?i)benilla:capture_cursor=0:max_framerate=60";
        let screen = "ddagrab=output_idx=0:framerate=60:draw_mouse=0:dup_frames=1";
        for (k, source) in [window, screen].into_iter().enumerate() {
            let left = secs - began.elapsed().as_secs_f32();
            if left < 1.0 {
                break;
            }
            // What ffmpeg says (frames dropped, a grab that failed) beside the video.
            let said = std::fs::OpenOptions::new().create(true).append(k > 0).write(true).truncate(k == 0).open(lab_dir().join(format!("{label}.ffmpeg.txt")));
            // Every frame at 60, scaled to 1920 wide, encoded with x264's fastest preset so the
            // encoder keeps up with a wide screen.
            let ran = std::process::Command::new(&ff)
                .args(["-y", "-hide_banner", "-loglevel", "warning", "-f", "lavfi", "-i", source])
                .args(["-t", &format!("{left:.2}"), "-vf", "hwdownload,format=bgra,scale=1920:-2:flags=bilinear,format=yuv420p"])
                .args(["-fps_mode", "cfr", "-r", "60", "-c:v", "libx264", "-preset", "ultrafast", "-tune", "zerolatency", "-crf", "18"])
                .arg(&out)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(said.map(std::process::Stdio::from).unwrap_or_else(|_| std::process::Stdio::null()))
                .status();
            match ran {
                Ok(st) if st.success() => break,
                Ok(st) => eprintln!("[lab] {source} ended with {st}; trying the next way"),
                Err(e) => {
                    eprintln!("[lab] couldn't record {}: {e}", out.display());
                    break;
                }
            }
        }
    });
}

/// Where the client on this computer saves its screenshots: `LAB_SHOTS`, or the Windows
/// release's folder beside this checkout (`Documents\\Basin Battle for Windows`).
fn shots_dir() -> Option<std::path::PathBuf> {
    let candidates = [std::env::var("LAB_SHOTS").ok().map(std::path::PathBuf::from), Some(std::path::PathBuf::from("../Basin Battle for Windows/benilla-config/Screenshots"))];
    candidates.into_iter().flatten().find(|d| d.is_dir())
}

/// The game's Data folder, as the server was started with it (`--data`, or `WOW_DATA`).
fn data_dir() -> Option<std::path::PathBuf> {
    let args: Vec<String> = std::env::args().collect();
    args.iter().position(|a| a == "--data").and_then(|i| args.get(i + 1)).map(std::path::PathBuf::from).or_else(|| std::env::var_os("WOW_DATA").map(std::path::PathBuf::from))
}

/// A lab `find`: the spells whose looks show a model by one of these words (`missile` first: the
/// missile's; `area` first: what it plays at a ground point), or the `spell <id>...` named, into logs/lab/find-<words>.txt.
fn lab_find(args: &[String]) {
    let what = args.first().filter(|a| *a == "missile" || *a == "area").cloned().unwrap_or_default();
    let words: Vec<String> = args.iter().filter(|a| *a != "missile" && *a != "area").take_while(|a| *a != "spell").cloned().collect();
    let only: Vec<u32> = args.iter().skip_while(|a| *a != "spell").skip(1).filter_map(|a| a.parse().ok()).collect();
    let name: String = args.join("-").chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).take(60).collect();
    let out = lab_dir().join(format!("find-{name}.txt"));
    let text = match data_dir().ok_or_else(|| anyhow::anyhow!("no --data")).and_then(|d| htl_map::spell_find(&d, &words, &what, &only, 600)) {
        Ok(lines) => lines.join("\n") + "\n",
        Err(e) => format!("find failed: {e:#}\n"),
    };
    let _ = std::fs::create_dir_all(out.parent().unwrap()).and_then(|_| std::fs::write(&out, text));
}

/// Where the lab's runs go: `LAB_DIR` (the lab helper points it at another drive when the host
/// has one), else `logs/lab`. The queue stays in `logs/lab` either way.
pub(crate) fn lab_dir() -> std::path::PathBuf {
    std::env::var_os("LAB_DIR").map(std::path::PathBuf::from).unwrap_or_else(|| std::path::Path::new("logs").join("lab"))
}

/// The most the lab's runs may take on the disk (`lab_dir`): about 300 runs of video alone.
const LAB_BUDGET: u64 = 3_000_000_000;

/// Everything under `dir`, in bytes (0 if it isn't there).
fn folder_bytes(dir: &std::path::Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(dir) else { return 0 };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => folder_bytes(&e.path()),
            _ => e.metadata().map(|m| m.len()).unwrap_or(0),
        })
        .sum()
}

/// The screenshots saved from `since` (a moment before) to `upto`, moved into `to` at half size.
/// Moved, not copied, and halved: at full size, a night of runs kept twice over filled the host's
/// disk (a 3440 wide shot is about 6 MB). Half size still makes a 1280 wide clip.
fn copy_shots(since: std::time::SystemTime, upto: std::time::SystemTime, to: &std::path::Path) {
    let Some(dir) = shots_dir() else { return };
    let from = since - std::time::Duration::from_millis(500);
    let Ok(entries) = std::fs::read_dir(&dir) else { return };
    let mut made = false;
    for e in entries.flatten() {
        let fresh = e.metadata().and_then(|m| m.modified()).is_ok_and(|t| t >= from && t <= upto);
        let name = e.file_name();
        if !fresh || !name.to_string_lossy().starts_with("WoWScrnShot_") {
            continue;
        }
        if !made {
            if std::fs::create_dir_all(to).is_err() {
                return;
            }
            made = true;
        }
        let out = to.join(&name);
        let halved = image::open(e.path()).ok().map(|im| im.resize(im.width() / 2, im.height() / 2, image::imageops::FilterType::Triangle)).and_then(|im| im.save(&out).ok());
        // The original goes only once its half-size copy is written; if that failed, it moves whole.
        if halved.is_some() {
            let _ = std::fs::remove_file(e.path());
        } else if std::fs::rename(e.path(), &out).is_err() {
            let _ = std::fs::copy(e.path(), &out);
        }
    }
}

/// The event's text names one of these ids: the number as a field's value (`: 12`, `Some(12)`,
/// `Unit(12)`, `[12, 30]`), not part of a longer number or a position.
fn mentions(text: &str, ids: &[Id]) -> bool {
    ids.iter().any(|id| {
        let n = id.to_string();
        text.match_indices(&n).any(|(i, _)| {
            let before = text[..i].chars().last();
            let after = text[i + n.len()..].chars().next();
            matches!(before, Some(' ' | '(' | '[')) && !after.is_some_and(|c| c.is_ascii_digit() || c == '.') && !text[..i].ends_with("pos: [") && !text[..i].trim_end().ends_with(".")
        })
    })
}

#[cfg(test)]
mod tests {
    use super::{mentions, race_gender};

    #[test]
    fn a_run_names_its_race_and_gender_as_race_dash_m_or_f() {
        assert_eq!(race_gender("tauren-f"), Some((6, 1)));
        assert_eq!(race_gender("NightElf-male"), Some((4, 0)));
        assert_eq!(race_gender("night_elf-m"), Some((4, 0)));
        assert_eq!(race_gender("goblin-m"), None);
        assert_eq!(race_gender("orc"), None);
    }

    #[test]
    fn a_trace_keeps_the_events_that_name_the_hero_or_the_scene() {
        assert!(mentions("Swing { attacker: 12, victim: Some(34), damage: 0 }", &[34]));
        assert!(mentions("Missile { caster: 12, spell: 845, target: Unit(12) }", &[12]));
        assert!(!mentions("Missile { caster: 120, spell: 845 }", &[12]), "not part of a longer number");
        assert!(!mentions("Hop { player: 7, dir: 12.5 }", &[12]), "not a number with a point");
        assert!(!mentions("Swing { damage: 34 }", &[3]));
    }
}
