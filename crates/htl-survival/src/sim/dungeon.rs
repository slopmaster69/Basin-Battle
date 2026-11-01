//! The Deadmines: a world of its own, reached by the Smuggler's Logbook at camp. Its rules are a
//! souls-like's: lamps to light and rest at (resting heals, refills the Draughts and brings every
//! foe but the bosses back), Draughts the only heal, a fall that leaves one sack to win back (die
//! again first and it's gone), plunder from every foe, gear that lies where its owner fell,
//! strongboxes off the path, shortcuts opened from their far side, fog doors that bar a boss's
//! room while it fights, and a front door that opens when the last boss falls.

use std::collections::{BTreeMap, BTreeSet};

use super::*;
use crate::content::{DoorDef, DungeonDef, LampDef, NoteDef};

/// The world below: what the group has done there, and each hero's lamp and Draughts.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Below {
    pub lit: BTreeSet<String>,
    /// Shortcuts and gates opened for good.
    pub open: BTreeSet<String>,
    /// Packs beaten for good (bosses, elites), by key.
    pub slain: BTreeSet<String>,
    /// Strongboxes emptied.
    pub looted: BTreeSet<String>,
    pub heroes: BTreeMap<String, BelowHero>,
    /// Fog doors barred now, while their boss fights (not kept).
    pub fog: BTreeSet<String>,
    /// Hidden spots a hero has been nudged toward this run (not kept).
    pub noticed: BTreeSet<String>,
    /// The Bookmaker's Wager under way, if any (not kept).
    pub wager: Option<super::wager::Wager>,
    /// Foes down since the last rest: (kind, home, pack), back at the next.
    pub fallen: Vec<(usize, Vec3, Option<usize>)>,
    /// The hazards' next beat, by their place in the list.
    pub beats: Vec<f32>,
    /// How many heroes the foes' health was last scaled for.
    pub heroes_seen: usize,
    /// When this run's first boss fell or first strongbox was emptied (unix seconds): the run
    /// starts over `reset_minutes` after, once no hero is below.
    pub run_at: Option<f64>,
}

/// A hero below: the lamp they wake at, their Draughts, and the flasks they've found.
#[derive(Debug, Clone, PartialEq)]
pub struct BelowHero {
    pub lamp: String,
    pub draughts: u32,
    pub flasks: u32,
}

/// One dungeon's part of the book at camp.
#[derive(Debug, Clone, PartialEq)]
pub struct BookDungeon {
    pub key: String,
    pub name: String,
    pub icon: String,
    pub blurb: String,
    pub first_page: String,
    /// The camp has passed its gate (raids beaten).
    pub open: bool,
    pub gate: u32,
    /// How many lamps it has, and its pages: (lamp key, lamp name), its first lamp's first.
    pub lamps: usize,
    pub pages: Vec<(String, String)>,
    /// Its secrets found of all, and its notes read of all (the camp's, for good).
    pub secrets: (usize, usize),
    pub notes: (usize, usize),
    /// The highest wager the camp may take on it, and its time to beat (seconds).
    pub wager: (u32, f32),
    /// How hard it is next to the others (1 to 3).
    pub tier: u32,
}

/// A door's state as it's drawn: open, or shut.
pub fn door_open(below: &Below, d: &DoorDef) -> bool {
    match d.kind.as_str() {
        "fog" => !below.fog.contains(&d.key),
        _ => below.open.contains(&d.key),
    }
}

/// Whether segment a-b crosses segment c-d (in the plane).
fn crosses(a: Vec3, b: Vec3, c: [f32; 2], d: [f32; 2]) -> bool {
    let o = |p: [f32; 2], q: [f32; 2], r: [f32; 2]| {
        (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0])
    };
    let (a, b) = ([a[0], a[1]], [b[0], b[1]]);
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

/// A hazard's flames burn this long where it went off.
const HAZARD_FLAME_SECS: f32 = 1.4;
/// The veil's columns stand this far apart along what it bars.
const VEIL_SPACING: f32 = 2.5;
/// How near a lamp a hero must be to rest at it or leave by it.
const LAMP_RANGE: f32 = 6.0;

impl Survival {
    /// The Deadmines' world: its lamps, doors, strongboxes and foes.
    pub fn new_dungeon(content: Content, land: Box<dyn Land>, seed: u64, wall_now: f64) -> Self {
        let mut s = Self::bare(content.for_dungeon(), land, seed, wall_now);
        let hazards = s.dungeon_def().map_or(0, |d| d.hazards.len());
        s.below = Some(Below {
            beats: vec![0.0; hazards],
            ..Below::default()
        });
        s.place_below();
        s.place_packs();
        s.events.clear();
        s
    }

    /// This is the world below.
    pub fn is_below(&self) -> bool {
        self.below.is_some()
    }

    pub(crate) fn dungeon_def(&self) -> Option<&DungeonDef> {
        self.content.dungeon.as_ref()
    }

    fn lamp_def(&self, key: &str) -> Option<&LampDef> {
        self.dungeon_def()?.lamps.iter().find(|l| l.key == key)
    }

    /// The lamp a hero wakes at: their own, else the first.
    pub fn lamp_of(&self, account: &str) -> Option<&LampDef> {
        let d = self.dungeon_def()?;
        let own = self
            .below
            .as_ref()
            .and_then(|b| b.heroes.get(account))
            .map(|h| h.lamp.as_str());
        own.and_then(|k| self.lamp_def(k))
            .or_else(|| d.lamps.first())
    }

    pub(crate) fn place_below(&mut self) {
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        let looted = self
            .below
            .as_ref()
            .map(|b| b.looted.clone())
            .unwrap_or_default();
        for l in &d.lamps {
            let id = self.alloc();
            self.objs.insert(
                id,
                Obj {
                    kind: ObjKind::Lamp { key: l.key.clone() },
                    pos: l.at,
                    facing: l.facing,
                },
            );
        }
        for dr in &d.doors {
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Door { key: dr.key.clone() }, pos: dr.at, facing: dr.facing });
            // A lift's other end.
            if let (Some(from), "lift") = (dr.from, dr.kind.as_str()) {
                let id = self.alloc();
                self.objs.insert(id, Obj { kind: ObjKind::LiftEnd { key: dr.key.clone() }, pos: from, facing: dr.facing });
            }
            // The veil along what a fog or a gate bars, a column every couple of yards.
            if let (Some([a, b]), true) = (dr.bars, matches!(dr.kind.as_str(), "fog" | "gate")) {
                let len = ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2)).sqrt();
                let n = ((len / VEIL_SPACING).ceil() as usize).max(1);
                for k in 0..=n {
                    let t = k as f32 / n as f32;
                    let (x, y) = (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t);
                    let pos = self.ground_near(x, y, dr.at[2]);
                    let id = self.alloc();
                    self.objs.insert(id, Obj { kind: ObjKind::Veil { key: dr.key.clone() }, pos, facing: dr.facing });
                }
            }
        }
        // A note a secret keeps (its finder's to read) waits for it to be found.
        let notes: Vec<_> = d.notes.iter().filter(|n| !self.kept_secret(&n.key)).cloned().collect();
        for n in &notes {
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Note { key: n.key.clone() }, pos: n.at, facing: n.facing });
        }
        let kept: Vec<String> = d.boxes.iter().filter(|b| self.kept_secret(&b.key)).map(|b| b.key.clone()).collect();
        for b in d.boxes.iter().filter(|b| !looted.contains(&b.key) && !kept.contains(&b.key)) {
            let id = self.alloc();
            self.objs.insert(
                id,
                Obj {
                    kind: ObjKind::Strongbox { key: b.key.clone() },
                    pos: b.at,
                    facing: b.facing,
                },
            );
        }
    }

    /// A pack that's gone for good once beaten (a boss's, an elite's).
    pub(crate) fn lasting_pack(&self, k: usize) -> bool {
        let p = &self.content.packs[k];
        p.elite
            || self
                .content
                .mob(&p.mob)
                .is_some_and(|m| self.content.mobs[m].boss)
    }

    fn pack_slain(&self, k: usize) -> bool {
        let key = &self.content.packs[k].key;
        !key.is_empty() && self.below.as_ref().is_some_and(|b| b.slain.contains(key))
    }

    /// Whether a pack (by key) is beaten for good.
    pub fn slain(&self, key: &str) -> bool {
        self.below.as_ref().is_some_and(|b| b.slain.contains(key))
    }

    /// Whether a pack (by its place) should be laid out: not one beaten for good, nor one a
    /// secret still keeps.
    pub(crate) fn pack_wanted(&self, k: usize) -> bool {
        !(self.is_below() && (self.pack_slain(k) || self.kept_secret(&self.content.packs[k].key)))
    }

    /// A pack, strongbox or note (by key) that a secret not yet found keeps back.
    pub(crate) fn kept_secret(&self, key: &str) -> bool {
        if key.is_empty() {
            return false;
        }
        let (Some(d), Some(b)) = (self.dungeon_def(), self.below.as_ref()) else { return false };
        d.doors.iter().any(|dr| dr.secret() && dr.reveals.iter().any(|r| r == key) && !b.open.contains(&dr.key))
    }

    /// A secret found (a false wall given way, a hidden spot come upon): told to all, counted, and
    /// what it kept laid out.
    pub(crate) fn find_secret(&mut self, by: Id, key: &str) {
        let Some(d) = self.door_def(key).cloned() else { return };
        if self.below.as_ref().is_none_or(|b| b.open.contains(key)) {
            return;
        }
        // What it keeps, before it's open (after, nothing is kept).
        let reveals: Vec<String> = d.reveals.clone();
        self.below.as_mut().unwrap().open.insert(key.to_string());
        let who = self.players.get(&by).map_or(String::from("Someone"), |p| p.name.clone());
        // No banner and horn for everyone: the finder's page opens below, the rest read a line.
        let text = if d.text.is_empty() { format!("{who} has found something hidden: {}.", d.name) } else { format!("{who}: {}", d.text) };
        self.announce(text);
        self.forward_count("secret", key, 1);
        // The finder's to read at leisure: what it was.
        if let Some(dg) = self.dungeon_def() {
            // No tally: a secret is found, not ticked off.
            let place = format!("A secret of {}", dg.name);
            // Its page, if it has one (the chat line stays the short `text`). Content writes its
            // breaks as `{n}`; a letter's are real newlines (`letter` turns them back for the add-on,
            // and makes them paragraphs in chat without it).
            let body = if !d.page.is_empty() {
                d.page.replace("{n}", "\n")
            } else if d.text.is_empty() {
                format!("You have found something hidden: {}.", d.name)
            } else {
                sentence(&d.text)
            };
            let kept: Vec<String> = dg.notes.iter().filter(|n| reveals.contains(&n.key)).map(|n| n.name.clone()).collect();
            let clue = if kept.is_empty() { String::new() } else { format!("Something was left here to read: {}.", kept.join(", ")) };
            self.events.push(Event::Letter { player: by, kind: "secret".into(), title: d.name.clone(), place, text: body, clue, links: Vec::new() });
        }
        self.door_changed(key);
        let dd = self.dungeon_def().cloned().unwrap();
        let looted = self.below.as_ref().map(|b| b.looted.clone()).unwrap_or_default();
        for b in dd.boxes.iter().filter(|b| reveals.contains(&b.key) && !looted.contains(&b.key)) {
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Strongbox { key: b.key.clone() }, pos: b.at, facing: b.facing });
            self.events.push(Event::ObjSpawned { id });
        }
        for n in dd.notes.iter().filter(|n| reveals.contains(&n.key)) {
            let id = self.alloc();
            self.objs.insert(id, Obj { kind: ObjKind::Note { key: n.key.clone() }, pos: n.at, facing: n.facing });
            self.events.push(Event::ObjSpawned { id });
        }
        let packs: Vec<usize> = (0..self.content.packs.len()).filter(|k| reveals.contains(&self.content.packs[*k].key) && self.pack_wanted(*k)).collect();
        for k in packs {
            self.place_pack(k);
        }
        self.events.push(Event::Dirty);
    }

    /// A hero arrives below: at their lamp, with their Draughts in hand.
    pub(crate) fn below_join(&mut self, id: Id) {
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        let Some(account) = self.players.get(&id).map(|p| p.account.clone()) else {
            return;
        };
        let b = self.below.as_mut().unwrap();
        let first = d.lamps.first().map(|l| l.key.clone()).unwrap_or_default();
        let hero = b
            .heroes
            .entry(account.clone())
            .or_insert(BelowHero {
                lamp: first,
                draughts: d.draughts,
                flasks: 0,
            })
            .clone();
        if let (Some(lamp), Some(draught)) = (
            self.lamp_of(&account).cloned(),
            self.content.item(&d.draught),
        ) {
            let landing = self.landing_spot(lamp.at);
            self.land_from_far(id);
            let p = self.players.get_mut(&id).unwrap();
            p.pos = landing;
            p.facing = lamp.facing;
            p.last_safe = lamp.at;
            p.bag.remove(&draught);
            if hero.draughts > 0 {
                p.bag.insert(draught, hero.draughts);
            }
            p.quick = Some(draught);
        }        if self.wager().is_some() {
            self.wager_join(id);
        }
    }

    /// A hero leaves the world below: their Draughts stay down here, counted.
    pub(crate) fn below_leave(&mut self, id: Id) {
        let Some(draught) = self
            .dungeon_def()
            .and_then(|d| self.content.item(&d.draught))
        else {
            return;
        };
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        let n = p.bag.remove(&draught).unwrap_or(0);
        let account = p.account.clone();
        if let Some(h) = self.below.as_mut().and_then(|b| b.heroes.get_mut(&account)) {
            h.draughts = n;
        }
    }

    /// Where a hero sets down at a lamp: over its floor by [`super::ARRIVE_LIFT`], so the client
    /// drops onto it rather than starting level with it (and slipping under).
    pub(crate) fn landing_spot(&self, at: Vec3) -> Vec3 {
        let floor = self.land.height_near(at[0], at[1], at[2] + 1.0);
        let z = if (floor - at[2]).abs() < 3.0 { floor.max(at[2]) } else { at[2] };
        [at[0], at[1], z + super::ARRIVE_LIFT]
    }

    /// Put a hero at a lamp (lit), as if they'd just come there.
    pub fn arrive_at(&mut self, id: Id, lamp: &str) {
        let Some(l) = self.lamp_def(lamp).cloned() else {
            return;
        };
        let landing = self.landing_spot(l.at);
        self.land_from_far(id);
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        p.pos = landing;
        p.facing = l.facing;
        p.last_safe = l.at;
        let account = p.account.clone();
        if let Some(h) = self.below.as_mut().and_then(|b| b.heroes.get_mut(&account)) {
            h.lamp = l.key.clone();
        }
    }

    /// The lamps lit, in the route's order: (key, name).
    pub fn lit_lamps(&self) -> Vec<(String, String)> {
        let Some(d) = self.dungeon_def() else {
            return Vec::new();
        };
        let lit = self.below.as_ref().map(|b| &b.lit);
        d.lamps
            .iter()
            .filter(|l| lit.is_some_and(|s| s.contains(&l.key)))
            .map(|l| (l.key.clone(), l.name.clone()))
            .collect()
    }

    /// The book's pages at camp, every dungeon's: each one's first lamp always, and every lamp lit
    /// below (in the dungeons whose gate the camp has passed).
    pub fn book_pages(&self) -> Vec<(String, String)> {
        self.book_dungeons().into_iter().filter(|b| b.open).flat_map(|b| b.pages).collect()
    }

    /// The book's dungeons at camp, the easiest first: each one's name, look, whether the camp has
    /// earned it, and its pages (its first lamp, then those lit).
    pub fn book_dungeons(&self) -> Vec<BookDungeon> {
        let mut list: Vec<&DungeonDef> = self.content.dungeons().collect();
        list.sort_by_key(|d| d.tier);
        list.into_iter()
            .map(|d| BookDungeon {
                key: d.key.clone(),
                name: d.name.clone(),
                icon: d.icon.clone(),
                blurb: d.blurb.clone(),
                first_page: d.first_page.clone(),
                open: self.debug || self.camp.raids_won >= d.gate_raids,
                gate: d.gate_raids,
                lamps: d.lamps.len(),
                pages: d.lamps.iter().enumerate().filter(|(k, l)| *k == 0 || self.pages.contains(&l.key)).map(|(_, l)| (l.key.clone(), l.name.clone())).collect(),
                secrets: (d.doors.iter().filter(|dr| dr.secret() && self.ever("secret", &dr.key)).count(), d.doors.iter().filter(|dr| dr.secret()).count()),
                notes: (d.notes.iter().filter(|n| self.ever("read", &n.key)).count(), d.notes.len()),
                wager: (self.wager_unlocked(&d.key), super::wager::wager_limit(d, &self.content)),
                tier: d.tier,
            })
            .collect()
    }

    /// The lamps lit below, as the world below last said (for the book at camp).
    pub fn set_pages(&mut self, keys: Vec<String>) {
        self.pages = keys;
    }

    /// Whether the camp has earned the book (any of its dungeons).
    pub fn book_open(&self) -> bool {
        self.debug || self.content.dungeons().any(|d| self.camp.raids_won >= d.gate_raids)
    }

    pub fn book_id(&self) -> Id {
        self.objs
            .iter()
            .find(|(_, o)| o.kind == ObjKind::Book)
            .map_or(0, |(id, _)| *id)
    }

    /// Read the book at camp: a page takes the hero to that lamp below.
    pub fn travel(&mut self, id: Id, to: &str) -> Result<(), String> {
        let p = self.players.get(&id).ok_or("No hero.")?;
        if p.health == 0 {
            return Err("You're down.".into());
        }
        if self.is_below() {
            return self.travel_below(id, to);
        }
        // Siege's world: home from the hall (out of the match, if in one).
        if self.is_siege() {
            if to != "camp" {
                return Err("The muster banner in the hall is the way home: .es travel camp.".into());
            }
            if self.camp_of(id).is_some() && self.siege.as_ref().is_some_and(|s| s.phase == super::siege::SiegePhase::Running) {
                self.tell(id, "You leave the match: your camp holds without you.");
            }
            self.siege_depart(id);
            self.events.push(Event::Travel { player: id, to: None });
            return Ok(());
        }
        // In the ring: home by its banner.
        if self.in_arena(id) {
            if to != "camp" {
                return Err("Leave the ring first: the banner takes you home.".into());
            }
            self.leave_arena(id, super::arena::Exit::Home);
            return Ok(());
        }
        let book = self
            .objs
            .get(&self.book_id())
            .map(|o| o.pos)
            .ok_or("There's no book here.")?;
        if dist2d(p.pos, book) > INTERACT_RANGE + 2.0 {
            return Err("Go to the Smuggler's Logbook by the camp's chest.".into());
        }
        // The Proving Ring's page is open from the start, and Siege's.
        if to == "arena" && self.arena_def().is_some() {
            self.cancel_preview(id);
            return self.enter_arena(id);
        }
        if to == super::siege::SIEGE_PAGE && self.siege_def().is_some() {
            self.cancel_preview(id);
            self.count("siege", "book", 1);
            self.events.push(Event::Travel { player: id, to: Some(to.to_string()) });
            return Ok(());
        }
        if !self.book_open() {
            return Err(
                "The logbook's pages mean nothing yet: hold the camp against its first raid."
                    .into(),
            );
        }
        if !self.book_pages().iter().any(|(k, _)| k == to) {
            return match self.content.dungeon_of_lamp(to) {
                Some(d) if !(self.debug || self.camp.raids_won >= d.gate_raids) => Err(format!("{}'s pages are still in cipher: hold the camp against {} raid(s) first.", d.name, d.gate_raids)),
                _ => Err("That page is blank: light its lamp below first.".into()),
            };
        }
        self.cancel_preview(id);
        self.count("delve", "book", 1);
        self.events.push(Event::Travel {
            player: id,
            to: Some(to.to_string()),
        });
        Ok(())
    }

    /// Read the book at camp for a wager: down to a dungeon's first lamp at `level`, fresh (the
    /// world below decides whether it can start one now).
    pub fn travel_wager(&mut self, id: Id, dungeon: &str, level: u32) -> Result<(), String> {
        let d = self.content.dungeons().find(|d| d.key == dungeon).cloned().ok_or("No such dungeon in the book.")?;
        let first = d.lamps.first().map(|l| l.key.clone()).ok_or("That dungeon has no way in.")?;
        let most = self.wager_unlocked(&d.key);
        if level == 0 || level > most {
            return Err(format!("The Bookmaker won't take +{level} on {} yet: up to +{most} (win one in time to raise it).", d.name));
        }
        if !self.book_pages().iter().any(|(k, _)| *k == first) {
            return Err(format!("{}'s pages are still in cipher.", d.name));
        }
        let p = self.players.get(&id).ok_or("No hero.")?;
        let book = self.objs.get(&self.book_id()).map(|o| o.pos).ok_or("There's no book here.")?;
        if dist2d(p.pos, book) > INTERACT_RANGE + 2.0 {
            return Err("Go to the Smuggler's Logbook by the camp's chest.".into());
        }
        self.cancel_preview(id);
        self.events.push(Event::Travel { player: id, to: Some(format!("{first}@{level}")) });
        Ok(())
    }

    /// Below, at a lamp: on to another lit lamp, or (`camp`) home.
    fn travel_below(&mut self, id: Id, to: &str) -> Result<(), String> {
        let p = &self.players[&id];
        let lamp = self.lamp_near(p.pos).ok_or("Go to a lit lamp first.")?;
        if !self.lit(&lamp) {
            return Err("Light the lamp first.".into());
        }
        if self.in_fight(id) {
            return Err("Not with foes on you!".into());
        }
        if to != "camp" && self.wager_running() {
            return Err("Not during a wager: the lamps only take you home (and the wager goes on without you).".into());
        }
        if to == "camp" {
            self.events.push(Event::Travel {
                player: id,
                to: None,
            });
            return Ok(());
        }
        let l = self.lamp_def(to).cloned().ok_or("No such lamp.")?;
        if !self.lit(&l.key) {
            return Err("That lamp isn't lit.".into());
        }
        self.arrive_at(id, &l.key);
        let pos = self.players[&id].pos;
        self.events.push(Event::Teleport {
            player: id,
            pos,
            facing: l.facing,
        });
        self.tell(id, format!("You make your way to {}.", l.name));
        self.events.push(Event::Dirty);
        Ok(())
    }

    fn lit(&self, key: &str) -> bool {
        self.below.as_ref().is_some_and(|b| b.lit.contains(key))
    }

    /// The lamp (by key) within reach of a spot, if any.
    fn lamp_near(&self, pos: Vec3) -> Option<String> {
        self.dungeon_def()?
            .lamps
            .iter()
            .filter(|l| dist2d(l.at, pos) <= LAMP_RANGE && (l.at[2] - pos[2]).abs() < 4.0)
            .map(|l| l.key.clone())
            .next()
    }

    /// A foe is fighting this hero.
    fn in_fight(&self, id: Id) -> bool {
        self.mobs
            .values()
            .any(|m| m.health > 0 && m.state.target() == Some(id))
    }

    /// A lamp, used: lit the first time, then its window (rest, travel, trade, home).
    pub(crate) fn use_lamp(&mut self, id: Id, key: &str, lamp: Id) {
        if !self.lit(key) {
            let name = self
                .lamp_def(key)
                .map(|l| l.name.clone())
                .unwrap_or_default();
            self.below.as_mut().unwrap().lit.insert(key.to_string());
            let hero = self.players[&id].name.clone();
            self.events.push(Event::Banner {
                player: None,
                title: "Lamp Lit".into(),
                sub: name.clone(),
            });
            self.announce(format!("{hero} lights the lamp at {name}. It's a page in the book now, and a place to wake."));
            self.events.push(Event::ObjChanged { id: lamp });
            self.forward_count("delve", key, 1);
            self.events.push(Event::Dirty);
        }
        let account = self.players[&id].account.clone();
        if let Some(h) = self.below.as_mut().and_then(|b| b.heroes.get_mut(&account)) {
            h.lamp = key.to_string();
        }
        self.events.push(Event::StationOpen {
            player: id,
            station: lamp,
        });
    }

    /// Rest at the lamp in reach: whole again, the Draughts full, and the mine stirs (every foe
    /// but the bosses back at their posts).
    pub fn rest(&mut self, id: Id) -> Result<(), String> {
        let p = self.players.get(&id).ok_or("No hero.")?;
        let lamp = self.lamp_near(p.pos).ok_or("Rest at a lamp.")?;
        if !self.lit(&lamp) {
            return Err("Light the lamp first.".into());
        }
        if self.in_fight(id) {
            return Err("Not with foes on you!".into());
        }
        let account = p.account.clone();
        if let Some(h) = self.below.as_mut().and_then(|b| b.heroes.get_mut(&account)) {
            h.lamp = lamp.clone();
        }
        self.refresh_hero(id);
        self.stir();
        let name = self.players[&id].name.clone();
        let place = self.dungeon_def().map_or_else(|| "the dungeon".to_string(), |d| d.name.clone());
        self.announce(format!("{name} rests at the lamp. {place} stirs: its foes are back where they stood."));
        self.events.push(Event::Missile {
            caster: id,
            spell: REST_SPELL,
            target: Target::Unit(id),
        });
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// Whole again, Draughts refilled.
    fn refresh_hero(&mut self, id: Id) {
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        let Some(p) = self.players.get_mut(&id) else {
            return;
        };
        p.health = p.max_health;
        p.stamina = self.content.player.stamina;
        p.effects.clear();
        let (h, m) = (p.health, p.max_health);
        let account = p.account.clone();
        let flasks = self
            .below
            .as_ref()
            .and_then(|b| b.heroes.get(&account))
            .map_or(0, |h| h.flasks);
        let full = (d.draughts + flasks).min(d.draughts_max);
        if let Some(i) = self.content.item(&d.draught) {
            self.players.get_mut(&id).unwrap().bag.insert(i, full);
        }
        self.events.push(Event::Health {
            id,
            health: h,
            max: m,
        });
        self.events.push(Event::Stamina { player: id });
        self.events.push(Event::Effects { player: id });
        self.events.push(Event::Bags { player: id });
    }

    /// Every foe that isn't gone for good back at its post, whole.
    fn stir(&mut self) {
        let fallen = std::mem::take(&mut self.below.as_mut().unwrap().fallen);
        for (def, home, pack) in fallen {
            if pack.is_some_and(|k| self.pack_slain(k)) {
                continue;
            }
            self.spawn_mob(def, home, pack, false);
        }
        // A second form still standing (Sneed out of his shredder): the machine is back.
        let forms: Vec<(Id, usize)> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.health > 0)
            .filter_map(|(id, m)| m.form_of.map(|d| (*id, d)))
            .collect();
        for (id, def) in forms {
            let m = self.mobs.remove(&id).unwrap();
            self.events.push(Event::MobGone { id });
            // A boss's first form; a common one's comes back with the fallen.
            if m.pack.is_some_and(|k| self.lasting_pack(k)) {
                self.spawn_mob(def, m.home, m.pack, false);
            }
        }
        let ids: Vec<Id> = self.mobs.keys().copied().collect();
        for id in ids {
            let m = &self.mobs[&id];
            if m.health == 0 {
                continue;
            }
            let (home, max) = (m.home, m.max_health);
            let m = self.mobs.get_mut(&id).unwrap();
            m.health = max;
            if m.pos != home {
                m.pos = home;
                m.path.clear();
                m.state = mobs::MobState::Idle {
                    wander_at: self.clock + 5.0,
                };
                self.events.push(Event::MobStop {
                    id,
                    pos: home,
                    facing: m.facing,
                });
            }
            self.events.push(Event::Health {
                id,
                health: max,
                max,
            });
        }
        self.scale_foes(true);
    }

    /// The doors' look and what bars the way.
    pub fn door_def(&self, key: &str) -> Option<&DoorDef> {
        self.dungeon_def()?.doors.iter().find(|d| d.key == key)
    }

    pub fn door_is_open(&self, key: &str) -> bool {
        match (self.below.as_ref(), self.door_def(key)) {
            (Some(b), Some(d)) => door_open(b, d),
            _ => true,
        }
    }

    /// A walk from a to b goes through a shut door.
    pub(crate) fn through_shut_door(&self, a: Vec3, b: Vec3) -> bool {
        let (Some(below), Some(d)) = (self.below.as_ref(), self.dungeon_def()) else {
            return false;
        };
        d.doors.iter().any(|dr| dr.kind != "lift" && dr.bars.is_some_and(|[c, e]| !door_open(below, dr) && (a[2] - dr.at[2]).abs() < 6.0 && crosses(a, b, c, e)))
    }

    /// A door's look changed: the door and its veil, drawn again.
    pub(crate) fn door_changed(&mut self, key: &str) {
        let ids: Vec<Id> = self
            .objs
            .iter()
            .filter(|(_, o)| matches!(&o.kind, ObjKind::Door { key: k } | ObjKind::Veil { key: k } | ObjKind::LiftEnd { key: k } if k == key))
            .map(|(id, _)| *id)
            .collect();
        for id in ids {
            self.events.push(Event::ObjChanged { id });
        }
    }

    /// A lift (a rope, a cart): set going from its far end, then a ride between its ends.
    pub(crate) fn ride(&mut self, id: Id, key: &str, from_near: bool) {
        let Some(d) = self.door_def(key).cloned() else { return };
        let Some(far) = d.from else { return };
        let open = self.door_is_open(key);
        if !open && from_near {
            self.tell(id, format!("The {} is at its other end: set it going from there.", d.name));
            return;
        }
        if self.in_fight(id) {
            self.tell(id, "Not with foes on you!");
            return;
        }
        if !open {
            self.below.as_mut().unwrap().open.insert(key.to_string());
            let text = if d.text.is_empty() { format!("The {} works again: a way back.", d.name) } else { d.text.clone() };
            self.events.push(Event::Banner { player: None, title: "Shortcut Opened".into(), sub: d.name.clone() });
            self.announce(text);
            self.door_changed(key);
            self.events.push(Event::Dirty);
            return;
        }
        // To the other end, a step off it.
        let (to, toward) = if from_near { (far, d.at) } else { (d.at, far) };
        let away = facing(toward, to);
        let spot = self.ground_near(to[0] + away.cos() * 2.0, to[1] + away.sin() * 2.0, to[2]);
        if let Some(p) = self.players.get_mut(&id) {
            p.pos = spot;
            p.last_safe = spot;
        }
        self.events.push(Event::Teleport { player: id, pos: spot, facing: away });
        self.tell(id, format!("You ride the {}.", d.name));
    }

    /// A door, used: a shortcut opens from its far side; the others say why they're shut.
    pub(crate) fn use_door(&mut self, id: Id, key: &str, door: Id) {
        let Some(d) = self.door_def(key).cloned() else {
            return;
        };
        if d.kind == "lift" {
            self.ride(id, key, true);
            return;
        }
        if self.door_is_open(key) {
            self.tell(id, format!("The {} stands open.", d.name));
            return;
        }
        match d.kind.as_str() {
            "shortcut" => {
                let from = d.from.unwrap_or(d.at);
                let p = &self.players[&id];
                if dist2d(p.pos, from) > INTERACT_RANGE + 2.5 || (p.pos[2] - from[2]).abs() > 4.0 {
                    self.tell(id, format!("The {} is barred from the other side.", d.name));
                    return;
                }
                self.below.as_mut().unwrap().open.insert(key.to_string());
                let text = if d.text.is_empty() {
                    format!("The {} opens: a way back.", d.name)
                } else {
                    d.text.clone()
                };
                self.events.push(Event::Banner {
                    player: None,
                    title: "Shortcut Opened".into(),
                    sub: d.name.clone(),
                });
                self.announce(text);
                let _ = door;
                self.door_changed(key);
                self.events.push(Event::Dirty);
            }
            "illusory" | "hidden" => self.find_secret(id, key),
            "fog" => self.tell(id, "The way is barred while the fight goes on."),
            _ => self.tell(
                id,
                format!("The {} won't open while its keeper lives.", d.name),
            ),
        }
    }

    /// A note, read.
    pub(crate) fn read_note(&mut self, id: Id, key: &str) {
        if self.dungeon_def().is_none_or(|d| !d.notes.iter().any(|n| n.key == key)) {
            return;
        }
        self.forward_count("read", key, 1);
        self.note_letter(id, key, false);
    }

    /// Whether the camp has ever done `what` to `key` (read a note, found a secret).
    pub fn ever(&self, what: &str, key: &str) -> bool {
        self.camp.counts.get(&format!("{what}:{key}")).is_some_and(|n| *n > 0)
    }

    /// A note's page for a hero: its text, and its clue while what it points to is still hidden.
    fn note_letter(&mut self, id: Id, key: &str, again: bool) {
        let Some((d, n)) = self.content.dungeons().find_map(|d| d.notes.iter().find(|n| n.key == key).map(|n| (d.clone(), n.clone()))) else { return };
        let hidden = n.hints.iter().any(|h| !self.ever("secret", h));
        let clue = if hidden && !n.clue.is_empty() { n.clue.clone() } else { String::new() };
        let place = if again { format!("From the book: read in {}", d.name) } else { format!("Found in {}", d.name) };
        let links = vec![(format!(".es lore {}", d.key), format!("All found in {}", short_name(&d.name)))];
        self.events.push(Event::Letter { player: id, kind: "note".into(), title: n.name.clone(), place, text: n.text.clone(), clue, links });
    }

    /// A note read before, read again (from the book).
    pub fn reread(&mut self, id: Id, key: &str) -> Result<(), String> {
        if !self.content.dungeons().any(|d| d.notes.iter().any(|n| n.key == key)) || !self.ever("read", key) {
            return Err("You haven't read that one yet.".into());
        }
        self.note_letter(id, key, true);
        Ok(())
    }

    /// The book's record of a dungeon: the secrets found, the rumours of the rest (the clues of
    /// the notes read), and every note read, to read again.
    pub fn lore_page(&mut self, id: Id, key: &str) -> Result<(), String> {
        let d = self.content.dungeons().find(|d| d.key == key || (key.is_empty() && self.content.dungeon.as_ref().is_some_and(|x| x.key == d.key))).cloned().ok_or("No such dungeon in the book.")?;
        let read: Vec<&NoteDef> = d.notes.iter().filter(|n| self.ever("read", &n.key)).collect();
        // Secrets aren't listed or counted here: they're found down there, not ticked off a page.
        // What the notes read say of those still hidden is kept, as rumours.
        let mut paras = Vec::new();
        // A rumour for each secret still hidden that a note read speaks of.
        let mut rumours = Vec::new();
        for n in &read {
            if n.hints.iter().any(|h| !self.ever("secret", h)) && !n.clue.is_empty() {
                rumours.push(format!("- {} ({})", n.clue, n.name));
            }
        }
        if !rumours.is_empty() {
            paras.push(format!("Rumours:\n{}", rumours.join("\n")));
        }
        if read.is_empty() {
            paras.push("No notes read yet: they lie where they were dropped, waiting.".to_string());
        }
        let place = format!("{} of {} notes read", read.len(), d.notes.len());
        let links = read.iter().map(|n| (format!(".es note {}", n.key), n.name.clone())).collect();
        self.events.push(Event::Letter { player: id, kind: "lore".into(), title: d.name.clone(), place, text: paras.join("\n\n"), clue: String::new(), links });
        Ok(())
    }

    /// A strongbox: what it holds into the bags, once.
    pub(crate) fn open_box(&mut self, id: Id, key: &str, obj: Id) {
        let Some(b) = self
            .dungeon_def()
            .and_then(|d| d.boxes.iter().find(|b| b.key == key))
            .cloned()
        else {
            return;
        };
        let mut items: BTreeMap<usize, u32> = BTreeMap::new();
        for (k, n) in &b.loot {
            if let Some(i) = self.content.item(k) {
                items.insert(i, *n);
            }
        }
        // A flask isn't carried: it's a Draught more from now on.
        let flask = self.dungeon_def().and_then(|d| self.content.item(&d.flask));
        let flasks = flask.and_then(|f| items.remove(&f)).unwrap_or(0);
        if !self.bag_fits(id, &items) {
            self.tell(id, "Your bags can't hold what's in it.");
            return;
        }
        self.objs.remove(&obj);
        self.events.push(Event::ObjGone { id: obj });
        let wall = self.wall();
        let b = self.below.as_mut().unwrap();
        b.looted.insert(key.to_string());
        b.run_at.get_or_insert(wall);
        let mut words: Vec<String> = Vec::new();
        for (i, n) in &items {
            *self
                .players
                .get_mut(&id)
                .unwrap()
                .bag
                .entry(*i)
                .or_default() += n;
            words.push(format!("{n} {}", self.content.items[*i].name));
        }
        if flasks > 0 {
            self.add_flasks(id, flasks);
            words.push("a Draught Flask (a Draught more at every rest)".into());
        }
        self.tell(id, format!("In the strongbox: {}.", words.join(", ")));
        self.events.push(Event::Bags { player: id });
        // Its guards, waiting unseen for whoever opens it: on them at once.
        let guards: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.hidden && m.health > 0 && m.pack.is_some_and(|k| self.content.packs[k].wake == key))
            .map(|(mid, _)| *mid)
            .collect();
        if !guards.is_empty() {
            self.events.push(Event::Banner { player: Some(id), title: "Ambush!".into(), sub: String::new() });
        }
        for mid in guards {
            self.mobs.get_mut(&mid).unwrap().hidden = false;
            self.events.push(Event::MobSpawned { id: mid });
            self.events.push(Event::Visual { id: mid, kit: combat::KIT_DUST, by: None });
            self.engage(mid, mobs::Foe::Hero(id));
        }
        self.events.push(Event::Dirty);
    }

    fn add_flasks(&mut self, id: Id, n: u32) {
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        let Some(account) = self.players.get(&id).map(|p| p.account.clone()) else {
            return;
        };
        let Some(h) = self.below.as_mut().and_then(|b| b.heroes.get_mut(&account)) else {
            return;
        };
        let room = d.draughts_max.saturating_sub(d.draughts + h.flasks);
        let n = n.min(room);
        h.flasks += n;
        if let Some(i) = self.content.item(&d.draught) {
            *self.players.get_mut(&id).unwrap().bag.entry(i).or_default() += n;
        }
    }

    /// Gear lying where its owner fell: into the bags.
    pub(crate) fn take_loot(&mut self, id: Id, item: usize, n: u32, obj: Id) {
        let one = BTreeMap::from([(item, n)]);
        if !self.bag_fits(id, &one) {
            self.tell(id, "Your bags are full.");
            return;
        }
        self.objs.remove(&obj);
        self.events.push(Event::ObjGone { id: obj });
        *self
            .players
            .get_mut(&id)
            .unwrap()
            .bag
            .entry(item)
            .or_default() += n;
        let d = &self.content.items[item];
        let how = if matches!(
            d.kind,
            ItemKind::Weapon | ItemKind::Shield | ItemKind::Bow | ItemKind::Armor
        ) {
            " Right-click it in your bags to wear it."
        } else {
            ""
        };
        self.tell(id, format!("You take the {}.{how}", d.name));
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Dirty);
    }

    /// The lamp's quartermaster: what plunder buys, (key, name, price, have).
    pub fn trades(&self, id: Id) -> Vec<(String, String, u32, bool)> {
        let Some(d) = self.dungeon_def() else {
            return Vec::new();
        };
        let have = self
            .content
            .item(&d.plunder)
            .and_then(|i| self.players.get(&id).and_then(|p| p.bag.get(&i)))
            .copied()
            .unwrap_or(0);
        d.trade
            .iter()
            .filter_map(|(k, n)| {
                Some((
                    k.clone(),
                    self.content.items[self.content.item(k)?].name.clone(),
                    *n,
                    have >= *n,
                ))
            })
            .collect()
    }

    pub fn buy(&mut self, id: Id, key: &str) -> Result<String, String> {
        let d = self.dungeon_def().cloned().ok_or("Nobody trades here.")?;
        let price = *d
            .trade
            .get(key)
            .ok_or("The quartermaster doesn't have that.")?;
        let p = self.players.get(&id).ok_or("No hero.")?;
        if self.lamp_near(p.pos).is_none() {
            return Err("The quartermaster keeps to the lamps.".into());
        }
        let item = self.content.item(key).ok_or("No such thing.")?;
        let plunder = self.content.item(&d.plunder).ok_or("No plunder.")?;
        if p.bag.get(&plunder).copied().unwrap_or(0) < price {
            return Err(format!("That's {price} Defias Plunder."));
        }
        if !self.bag_fits(id, &BTreeMap::from([(item, 1)])) {
            return Err("Your bags are full.".into());
        }
        let p = self.players.get_mut(&id).unwrap();
        let e = p.bag.get_mut(&plunder).unwrap();
        *e -= price;
        if *e == 0 {
            p.bag.remove(&plunder);
        }
        *p.bag.entry(item).or_default() += 1;
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Dirty);
        Ok(self.content.items[item].name.clone())
    }

    /// Something done below that counts at camp too.
    pub(crate) fn forward_count(&mut self, what: &str, key: &str, n: u32) {
        self.count(what, key, n);
        if self.is_below() {
            self.events.push(Event::ToCamp {
                counts: vec![(format!("{what}:{key}"), n)],
                items: BTreeMap::new(),
            });
        }
    }

    /// A foe falls below: plunder into the killer's bags (as its drops), the gear it held lying
    /// where it fell (sometimes), a boss's trophy to the camp's chest, and its pack gone for good
    /// if it was a boss's or an elite's.
    pub(crate) fn below_kill(
        &mut self,
        def: usize,
        home: Vec3,
        at: Vec3,
        pack: Option<usize>,
        raider: bool,
        form: bool,
    ) {
        if raider || pack.is_none() {
            return;
        }
        self.wager_kill(def, at, pack, form);
        let k = pack.unwrap();
        let lasting = self.lasting_pack(k);
        // (A second form, a druid's serpent or a split ooze, comes back as its first: that one is
        // counted already.)
        if !lasting && !form {
            self.below.as_mut().unwrap().fallen.push((def, home, pack));
        } else {
            let standing = self
                .mobs
                .values()
                .any(|m| m.health > 0 && m.pack == Some(k));
            let key = self.content.packs[k].key.clone();
            if !standing && !key.is_empty() {
                let wall = self.wall();
                let b = self.below.as_mut().unwrap();
                b.slain.insert(key.clone());
                b.run_at.get_or_insert(wall);
                self.events.push(Event::Dirty);
                self.boss_beaten(&key);
            }
        }
        // What it held and wore, on the ground for whoever takes it.
        let d = self.content.mobs[def].clone();
        let (wc, ac, extra) = self
            .dungeon_def()
            .map_or((0.0, 0.0, 0.0), |d| (d.weapon_chance, d.armor_chance, d.boss_extra_chance));
        // A boss: one of its pieces for certain (its weapon or its armour), the rest by chance.
        let sure = if lasting && !d.gear.is_empty() { Some(self.rng.gen_range(0..d.gear.len())) } else { None };
        for (n, key) in d.gear.iter().enumerate() {
            let Some(item) = self.content.item(key) else {
                continue;
            };
            let armor = self.content.items[item].kind == ItemKind::Armor;
            let chance = if sure == Some(n) {
                1.0
            } else if lasting {
                extra
            } else if armor {
                ac
            } else {
                wc
            };
            if self.rng.gen::<f32>() < chance {
                let a = self.rng.gen::<f32>() * std::f32::consts::TAU;
                let pos = self.ground_near(at[0] + a.cos() * 0.8, at[1] + a.sin() * 0.8, at[2]);
                let id = self.alloc();
                self.objs.insert(
                    id,
                    Obj {
                        kind: ObjKind::Loot { item, n: 1 },
                        pos,
                        facing: a,
                    },
                );
                self.events.push(Event::ObjSpawned { id });
            }
        }
    }

    /// A boss (or an elite's pack) beaten for good: the doors it kept open.
    fn boss_beaten(&mut self, key: &str) {
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        for dr in d.doors.iter().filter(|dr| dr.boss == key) {
            let b = self.below.as_mut().unwrap();
            b.fog.remove(&dr.key);
            if dr.kind != "shortcut" {
                b.open.insert(dr.key.clone());
            }
            self.door_changed(&dr.key);
            if dr.kind == "gate" {
                let text = if dr.text.is_empty() {
                    format!("The {} opens.", dr.name)
                } else {
                    dr.text.clone()
                };
                self.announce(text);
            }
        }
        if key == d.exit_boss {
            self.events.push(Event::Banner {
                player: None,
                title: "The Way Out".into(),
                sub: "The front door stands open: walk out into Moonbrook".into(),
            });
        }
    }

    /// At camp: what was done below (the journal's counts), and trophies into the chest.
    pub fn from_below(&mut self, counts: &[(String, u32)], items: &BTreeMap<usize, u32>) {
        for (k, n) in counts {
            *self.camp.counts.entry(k.clone()).or_default() += n;
        }
        if !items.is_empty() {
            let mut names = Vec::new();
            for (i, n) in items {
                *self.storage.entry(*i).or_default() += n;
                names.push(self.content.items[*i].name.clone());
            }
            self.events.push(Event::Storage);
            let place = self.dungeon_def().map_or_else(|| "below".to_string(), |d| d.name.clone());
            self.announce(format!("From {place}, by the book: into the camp's chest, {}.", names.join(", ")));
        }
        self.events.push(Event::Dirty);
    }

    /// The trophy of a boss below goes to the camp's chest, through the book.
    pub(crate) fn trophy_to_camp(&mut self, items: BTreeMap<usize, u32>) {
        self.events.push(Event::ToCamp {
            counts: Vec::new(),
            items,
        });
    }

    /// A hero falls below: what they carried that's of the dungeon (plunder, gear in the bags)
    /// waits in one sack where they fell; an older sack of theirs is lost.
    pub(crate) fn below_drops(&mut self, id: Id) -> BTreeMap<usize, u32> {
        let Some(d) = self.dungeon_def().cloned() else {
            return BTreeMap::new();
        };
        let draught = self.content.item(&d.draught);
        let Some(p) = self.players.get_mut(&id) else {
            return BTreeMap::new();
        };
        let account = p.account.clone();
        let dropped: BTreeMap<usize, u32> = p
            .bag
            .iter()
            .filter(|(i, n)| {
                **n > 0
                    && Some(**i) != draught
                    && !matches!(self.content.items[**i].kind, ItemKind::Food)
            })
            .map(|(i, n)| (*i, *n))
            .collect();
        for i in dropped.keys() {
            p.bag.remove(i);
        }
        let old: Vec<Id> = self
            .objs
            .iter()
            .filter(|(_, o)| matches!(&o.kind, ObjKind::Cache { owner, .. } if *owner == account))
            .map(|(id, _)| *id)
            .collect();
        if !old.is_empty() && !dropped.is_empty() {
            self.tell(id, "Your last sack is lost down here.");
        }
        for o in old {
            self.objs.remove(&o);
            self.events.push(Event::ObjGone { id: o });
        }
        dropped
    }

    /// Where a fallen hero gets up below: their lamp, and if nobody else still stands, the mine
    /// stirs as if they'd rested.
    pub(crate) fn below_revive(&mut self, id: Id) -> Option<(Vec3, f32)> {
        if !self.is_below() {
            return None;
        }
        let account = self.players.get(&id)?.account.clone();
        let lamp = self.lamp_of(&account)?.clone();
        let others = self.players.iter().any(|(h, p)| *h != id && p.health > 0);
        self.refresh_hero(id);
        if !others {
            self.stir();
        }
        Some((lamp.at, lamp.facing))
    }

    /// A shut door in a hero's way: back to where they were.
    pub(crate) fn hold_at_doors(&mut self, id: Id, from: Vec3, to: Vec3) -> bool {
        if dist2d(from, to) > 12.0 || !self.through_shut_door(from, to) {
            return false;
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.pos = from;
            let f = p.facing;
            self.events.push(Event::Teleport {
                player: id,
                pos: from,
                facing: f,
            });
        }
        true
    }

    /// Each tick below: fog doors, ambushers stepping out, hazards, foes' health for the heroes
    /// here, and the front door.
    pub(crate) fn step_below(&mut self) {
        if !self.is_below() {
            return;
        }
        let Some(d) = self.dungeon_def().cloned() else {
            return;
        };
        // The Bookmaker's Wager, if one's on.
        if self.wager().is_some() {
            self.step_wager();
            if self.wager().is_none() {
                return;
            }
        }
        // A run beaten a while ago, and nobody below: the bosses stand again for the next.
        if let Some(at) = self.below.as_ref().and_then(|b| b.run_at) {
            if d.reset_minutes > 0.0 && self.players.is_empty() && self.wall() >= at + d.reset_minutes as f64 * 60.0 {
                self.reset_below(false);
                return;
            }
        }
        let now = self.clock;
        // A fog door bars its boss's room while the boss fights.
        for dr in d.doors.iter().filter(|dr| dr.kind == "fog") {
            let k = self.content.packs.iter().position(|p| p.key == dr.boss);
            let fighting = k.is_some_and(|k| {
                self.mobs
                    .values()
                    .any(|m| m.pack == Some(k) && m.health > 0 && (m.state.foe().is_some() || m.rise_at.is_some()))
            });
            let b = self.below.as_mut().unwrap();
            let was = b.fog.contains(&dr.key);
            if fighting != was {
                if fighting {
                    b.fog.insert(dr.key.clone());
                } else {
                    b.fog.remove(&dr.key);
                }
                self.door_changed(&dr.key);
            }
        }
        // Hidden spots, come upon.
        for dr in d.doors.iter().filter(|dr| dr.kind == "hidden") {
            if self.below.as_ref().is_some_and(|b| b.open.contains(&dr.key)) {
                continue;
            }
            // A hero coming near its marker is nudged once a run; searching it finds it.
            if self.below.as_ref().is_some_and(|b| b.noticed.contains(&dr.key)) {
                continue;
            }
            let by = self.players.iter().find(|(_, p)| p.health > 0 && dist2d(p.pos, dr.at) <= dr.find && (p.pos[2] - dr.at[2]).abs() < 5.0).map(|(h, _)| *h);
            if let Some(h) = by {
                self.below.as_mut().unwrap().noticed.insert(dr.key.clone());
                let look = dr.look.to_lowercase();
                let look = look.strip_prefix("a ").or_else(|| look.strip_prefix("an ")).or_else(|| look.strip_prefix("the ")).unwrap_or(&look).to_string();
                self.tell(h, format!("Something about the {look} isn't right. Search it."));
            }
        }
        // Ambushers step out of the dark when a hero comes near.
        let hidden: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.hidden && m.health > 0 && m.rise_at.is_none())
            .map(|(id, _)| *id)
            .collect();
        for mid in hidden {
            let m = &self.mobs[&mid];
            // (One its strongbox wakes waits for that; see `open_box`.)
            let wake = m.pack.filter(|k| self.content.packs[*k].wake.is_empty()).map_or(0.0, |k| self.content.packs[k].hidden);
            let near = self
                .players
                .iter()
                .filter(|(_, p)| {
                    p.health > 0
                        && dist2d(p.pos, m.pos) <= wake
                        && (p.pos[2] - m.pos[2]).abs() < 5.0
                })
                .map(|(h, p)| (*h, dist2d(p.pos, m.pos)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            if let Some((h, _)) = near {
                self.mobs.get_mut(&mid).unwrap().hidden = false;
                self.events.push(Event::MobSpawned { id: mid });
                self.events.push(Event::Visual {
                    id: mid,
                    kit: combat::KIT_DUST,
                    by: None,
                });
                self.engage(mid, mobs::Foe::Hero(h));
            }
        }
        // Hazards on their beat while a hero is near (and their boss fights, if they have one).
        for (k, h) in d.hazards.iter().enumerate() {
            let due = self
                .below
                .as_ref()
                .unwrap()
                .beats
                .get(k)
                .copied()
                .unwrap_or(0.0);
            if now < due {
                continue;
            }
            let near = self.players.values().any(|p| {
                p.health > 0 && dist2d(p.pos, h.at) <= h.wake && (p.pos[2] - h.at[2]).abs() < 12.0
            });
            let boss_on = h.boss.is_empty()
                || self
                    .content
                    .packs
                    .iter()
                    .position(|p| p.key == h.boss)
                    .is_some_and(|pk| {
                        self.mobs
                            .values()
                            .any(|m| m.pack == Some(pk) && m.health > 0 && m.state.foe().is_some())
                    });
            // The first beat waits for its phase.
            let next = if due == 0.0 {
                now + h.phase
            } else {
                now + h.every
            };
            if let Some(b) = self.below.as_mut().and_then(|b| b.beats.get_mut(k)) {
                *b = next.max(now + 0.05);
            }
            if due == 0.0 || !near || !boss_on {
                continue;
            }
            // Warned by its own spell's ground (Heated Ground before the flames...).
            let mark = self.mark_ground(h.at, h.radius, h.warn, mobs::ground_look(h.spell, 0));
            self.missiles.push(combat::Missile {
                look: 0,
                owner: 0,
                from: h.at,
                at: h.at,
                radius: h.radius,
                damage: h.damage,
                blockable: false,
                heavy: h.knockdown,
                land_at: now + h.warn,
                mark,
            });
            // As it goes off: flames where it lands, a moment (and the spell's look, if it has one).
            self.hazard_looks.push((now + h.warn, h.spell, h.at, None));
            let warned: Vec<Id> = self
                .players
                .iter()
                .filter(|(_, p)| p.health > 0 && dist2d(p.pos, h.at) <= h.radius + 3.0)
                .map(|(id, _)| *id)
                .collect();
            for v in warned {
                self.events.push(Event::Danger {
                    player: v,
                    secs: h.warn,
                    from: None,
                });
            }
        }
        let looks: Vec<(f32, u32, Vec3, Option<Id>)> = self.hazard_looks.iter().filter(|l| l.0 <= now).copied().collect();
        self.hazard_looks.retain(|l| l.0 > now);
        for (_, spell, at, fire) in looks {
            match fire {
                // The flames out.
                Some(f) => {
                    self.objs.remove(&f);
                    self.events.push(Event::ObjGone { id: f });
                }
                None => {
                    if spell != 0 {
                        self.events.push(Event::Missile { caster: 0, spell, target: Target::Point(at) });
                    }
                    let f = self.alloc();
                    self.objs.insert(f, Obj { kind: ObjKind::Fire { on: 0 }, pos: at, facing: 0.0 });
                    self.events.push(Event::ObjSpawned { id: f });
                    self.hazard_looks.push((now + HAZARD_FLAME_SECS, 0, at, Some(f)));
                }
            }
        }
        self.scale_foes(false);
        // Out the front door, once its keeper is down.
        let open = d.exit_boss.is_empty() || self.slain(&d.exit_boss);
        if open {
            let out: Vec<Id> = self
                .players
                .iter()
                .filter(|(_, p)| {
                    p.health > 0
                        && dist2d(p.pos, d.exit) <= d.exit_radius
                        && (p.pos[2] - d.exit[2]).abs() < 6.0
                })
                .map(|(id, _)| *id)
                .collect();
            for id in out {
                let name = self.players[&id].name.clone();
                self.forward_count("delve", "end", 1);
                self.events.push(Event::Banner {
                    player: Some(id),
                    title: format!("{} Cleared", d.name),
                    sub: if d.key == "deadmines" { "Out into Moonbrook, and home by the book".into() } else { "Out, and home by the book".into() },
                });
                if d.key == "deadmines" {
                    self.announce(format!("{name} walks out of the Deadmines' front door into Moonbrook."));
                } else {
                    self.announce(format!("{name} walks out of {} and home.", d.name));
                }
                self.events.push(Event::Travel {
                    player: id,
                    to: None,
                });
            }
        }
    }

    /// Foes take more health for each hero below: a foe unhurt is set to it.
    /// A foe's health for the heroes below now (`hero_health` more for each past the first).
    pub(crate) fn foe_health(&self, def: usize) -> u32 {
        let base = self.base_health(def);
        let Some(share) = self.dungeon_def().map(|d| d.hero_health) else {
            return base;
        };
        let n = self.players.len().max(1);
        (base as f32 * (1.0 + share * (n - 1) as f32)).round() as u32
    }

    fn scale_foes(&mut self, force: bool) {
        let Some(share) = self.dungeon_def().map(|d| d.hero_health) else {
            return;
        };
        let n = self.players.len().max(1);
        let b = self.below.as_mut().unwrap();
        if !force && b.heroes_seen == n {
            return;
        }
        b.heroes_seen = n;
        let ids: Vec<Id> = self.mobs.keys().copied().collect();
        for id in ids {
            let m = &self.mobs[&id];
            if m.health == 0 || m.health < m.max_health {
                continue;
            }
            let base = self.base_health(m.def) as f32;
            let max = (base * (1.0 + share * (n - 1) as f32)).round() as u32;
            let m = self.mobs.get_mut(&id).unwrap();
            m.max_health = max;
            m.health = max;
            self.events.push(Event::Health {
                id,
                health: max,
                max,
            });
        }
    }
}

/// The look of resting at a lamp (a soft glow).
pub const REST_SPELL: u32 = 2061;

/// A secret's text as a page's sentence (they're written to follow "Name: ").
fn sentence(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A dungeon's name without its region ("Scarlet Monastery: Library" is the Library).
fn short_name(name: &str) -> String {
    let n = name.rsplit(": ").next().unwrap_or(name);
    format!("the {}", n.strip_prefix("The ").unwrap_or(n))
}
