//! Elwynn out past the camp: places found and marked, chests and caches (some hidden until a hero
//! comes upon them, some guarded by an ambush that wakes as they're opened, some shut until the
//! camp has beaten what guards them), shrines and relics whose cards the whole camp learns, and
//! rare creatures whose fall is told to everyone.

use super::*;

/// How often the land is looked over for chests filling again.
const REFILL_EVERY: f32 = 5.0;
/// A chest that is once a world: never full again.
pub const NEVER: f64 = 1.0e18;

/// A trial is under way while a hero stays this near its shrine or relic...
const TRIAL_STAY: f32 = 60.0;
/// ...and given up when none has been for this long.
const TRIAL_GIVE_UP: f32 = 12.0;
/// An escort's walker walks on only while a hero is this near him.
pub const ESCORT_NEAR: f32 = 20.0;
/// He has reached a point of his route this near it.
const ESCORT_REACHED: f32 = 1.5;

impl Survival {
    /// Set out the chests: those not opened (or full again), the secret ones kept back until found;
    /// a kept shrine that's spent stands spent.
    pub(crate) fn place_chests(&mut self) {
        if self.is_below() {
            return;
        }
        let wall = self.wall();
        for def in 0..self.content.chests.len() {
            let c = self.content.chests[def].clone();
            let opened = self.camp.opened.get(&c.key).is_some_and(|back| *back > wall);
            // A shrine or relic stands for every hero to learn from (a secret one, once found).
            let lasting = !c.teaches.is_empty();
            if opened && !c.keep && !lasting {
                continue;
            }
            if !opened && c.secret > 0.0 {
                self.unfound.insert(def);
                continue;
            }
            self.set_chest(def, opened && !lasting);
        }
    }

    /// Set out the land's props (fires, braziers, cages): only to be seen.
    pub(crate) fn place_props(&mut self) {
        if self.is_below() {
            return;
        }
        for k in 0..self.content.props.len() {
            if self.content.props[k].after.is_none() && !self.prop_gone(k) {
                self.place_piece(k);
            }
        }
        // The camp's own, as far as it has come.
        self.grow_camp(false);
    }

    /// Whether pack `k`'s condition holds (its `after`; none: always).
    pub(crate) fn pack_due(&self, k: usize) -> bool {
        self.content.packs[k].after.as_deref().is_none_or(|a| self.camp_has(a))
    }

    /// Packs that stand once the camp has come this far (`after`, not woken by a chest nor lying in
    /// wait): those whose condition holds now come into sight.
    fn packs_come(&mut self) {
        let due: Vec<Id> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.hidden && m.health > 0)
            .filter(|(_, m)| m.pack.is_some_and(|k| {
                let p = &self.content.packs[k];
                p.after.is_some() && p.wake.is_empty() && p.hidden <= 0.0 && self.pack_due(k)
            }))
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            self.mobs.get_mut(&id).unwrap().hidden = false;
            self.events.push(Event::MobSpawned { id });
            if self.content.mobs[self.mobs[&id].def].aura != 0 {
                self.events.push(Event::Auras { id });
            }
        }
    }

    /// Whether prop `k` has gone: its `until` holds.
    fn prop_gone(&self, k: usize) -> bool {
        self.content.props[k].until.as_deref().is_some_and(|u| self.camp_has(u))
    }

    /// Set out `[[prop]]` `k`.
    fn place_piece(&mut self, k: usize) -> Id {
        let d = &self.content.props[k];
        let (at, lift, facing) = (d.at, d.lift, d.facing);
        let mut pos = match d.z {
            Some(z) => self.ground_near(at[0], at[1], z),
            None => self.ground(at[0], at[1]),
        };
        pos[2] += lift;
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Piece { def: k }, pos, facing });
        id
    }

    /// Whether the camp has come as far as `after` says (`raids:N`, `built:<station>`,
    /// `goal:N`, `found:<site>`, `kill:<mob>` once the camp has killed one, `open:<chest>` once
    /// one has been opened); an unknown condition never holds.
    pub fn camp_has(&self, after: &str) -> bool {
        let (what, arg) = after.split_once(':').unwrap_or((after, ""));
        match what.trim() {
            "start" => true,
            "raids" => arg.trim().parse::<u32>().is_ok_and(|n| self.camp.raids_won >= n),
            "goal" => arg.trim().parse::<usize>().is_ok_and(|n| self.camp.goal >= n),
            "built" => {
                let key = arg.trim();
                self.has_station(key) || self.camp.counts.get(&format!("build:{key}")).is_some_and(|n| *n > 0)
            }
            "found" => self.camp.found.contains(arg.trim()),
            // `chose:<group>` once any option was chosen, `chose:<group>:<option>` for that one.
            "kill" | "open" | "chose" => self.ever(what.trim(), arg.trim()),
            _ => false,
        }
    }

    /// How near a hero must stand to a person to put a choice to them.
    const CHOOSE_RANGE: f32 = 6.0;

    /// A hero makes a choice a person is offering (`.es choose <group>:<option>`, a link in their
    /// talk line): only within 6 yd of a person whose current line has that link, and only once
    /// for a group. It is the camp's: it counts `chose:<group>` and `chose:<group>:<option>`, and
    /// the camp's people stand or go at once.
    pub fn choose(&mut self, id: Id, spec: &str) -> Result<(), String> {
        let spec = spec.trim();
        let Some((group, option)) = spec.split_once(':').filter(|(g, o)| !g.is_empty() && !o.is_empty() && !o.contains(':')) else {
            return Err("Try .es choose <group>:<option>.".into());
        };
        if self.ever("chose", group) {
            return Err("That is already decided.".into());
        }
        let Some(pos) = self.players.get(&id).filter(|p| p.health > 0).map(|p| p.pos) else { return Err("Not now.".into()) };
        let link = format!(".es choose {group}:{option}");
        let offered = self.objs.values().any(|o| match o.kind {
            ObjKind::Piece { def } => {
                dist2d(o.pos, pos) <= Self::CHOOSE_RANGE
                    && self.talk_line(def).is_some_and(|t| t.links.iter().any(|l| l.split('~').next().is_some_and(|c| c.trim() == link)))
            }
            _ => false,
        });
        if !offered {
            return Err("Nobody here is waiting on that. Talk to them first.".into());
        }
        self.count("chose", group, 1);
        self.count("chose", &format!("{group}:{option}"), 1);
        // Who stands and who goes follows the choice now, not at the next look.
        self.grow_camp(true);
        self.events.push(Event::Dirty);
        Ok(())
    }

    /// What the camp's person at prop `def` says now: the last of their lines whose `after`
    /// holds (none if they have no lines, or none holds yet).
    pub fn talk_line(&self, def: usize) -> Option<&crate::content::TalkDef> {
        let d = self.content.props.get(def)?;
        if !d.npc {
            return None;
        }
        d.talk.iter().rev().find(|t| t.after.as_deref().is_none_or(|a| self.camp_has(a)))
    }

    /// A hero uses one of the camp's people: their words, as a page (talk, no choices).
    pub(crate) fn talk_to(&mut self, id: Id, def: usize) {
        let Some(t) = self.talk_line(def).cloned() else { return };
        let d = &self.content.props[def];
        let links: Vec<(String, String)> = t.links.iter().filter_map(|l| l.split_once('~')).map(|(c, n)| (c.trim().to_string(), n.trim().to_string())).collect();
        self.events.push(Event::Letter { player: id, kind: "talk".into(), title: d.name.clone(), place: d.subname.clone(), text: t.text, clue: String::new(), links });
    }

    /// Set out every camp prop the camp has earned and doesn't have yet; with `tell`, the camp
    /// hears how it grows (each new piece's `say`).
    pub(crate) fn grow_camp(&mut self, tell: bool) {
        if self.is_below() || self.is_siege() {
            return;
        }
        // Packs whose time has come stand.
        self.packs_come();
        // What the camp has outgrown goes (a person taken away: `until`), and never stands again.
        let gone: Vec<Id> = self
            .objs
            .iter()
            .filter(|(_, o)| matches!(o.kind, ObjKind::Piece { def } if self.prop_gone(def)))
            .map(|(id, _)| *id)
            .collect();
        for id in gone {
            self.objs.remove(&id);
            self.events.push(Event::ObjGone { id });
        }
        let standing: BTreeSet<usize> = self.objs.values().filter_map(|o| match o.kind { ObjKind::Piece { def } => Some(def), _ => None }).collect();
        let due: Vec<usize> = (0..self.content.props.len())
            .filter(|k| !standing.contains(k))
            .filter(|k| self.content.props[*k].after.as_deref().is_some_and(|a| self.camp_has(a)))
            .filter(|k| !self.prop_gone(*k))
            .collect();
        for k in due {
            let id = self.place_piece(k);
            if tell {
                self.events.push(Event::ObjSpawned { id });
                let say = self.content.props[k].say.clone();
                if !say.is_empty() {
                    self.announce(say);
                }
            }
        }
    }

    /// Lay out the chests afresh (after a save is read: what it opened stays open).
    pub(crate) fn replace_chests(&mut self) {
        let old: Vec<Id> = self.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Chest { .. })).map(|(id, _)| *id).collect();
        for id in old {
            self.objs.remove(&id);
        }
        self.unfound.clear();
        self.place_chests();
    }

    fn set_chest(&mut self, def: usize, spent: bool) -> Id {
        let c = &self.content.chests[def];
        let at = match c.z {
            Some(z) => self.ground_near(c.at[0], c.at[1], z),
            None => self.ground(c.at[0], c.at[1]),
        };
        let facing = c.facing;
        let id = self.alloc();
        self.objs.insert(id, Obj { kind: ObjKind::Chest { def, spent }, pos: at, facing });
        self.events.push(Event::ObjSpawned { id });
        id
    }

    /// Whether a chest's `after` holds: a creature's key (the camp has killed one), or, with a colon
    /// in it, a camp condition (`open:<chest>`, `found:<site>`, `kill:<mob>`) read as `camp_has`.
    pub(crate) fn chest_unlocked(&self, after: &str) -> bool {
        if after.contains(':') {
            self.camp_has(after)
        } else {
            self.camp.counts.get(&format!("kill:{after}")).copied().unwrap_or(0) > 0
        }
    }

    /// Use a chest: shut until its guardian falls (or its condition holds), else the work of
    /// opening it begins.
    pub(crate) fn use_chest(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        if !c.after.is_empty() && !self.chest_unlocked(&c.after) {
            let text = if !c.locked.is_empty() {
                c.locked.clone()
            } else if c.after.contains(':') {
                format!("{} is shut.", c.name)
            } else {
                let who = self.content.mob(&c.after).map_or(c.after.clone(), |m| self.content.mobs[m].name.clone());
                format!("{} won't open while {who} lives.", c.name)
            };
            self.tell(id, text);
            return;
        }
        let pos = self.players[&id].pos;
        self.set_action(id, Action::Gather { t: 0.0, node: obj, at: pos });
        self.events.push(Event::Cast { caster: id, spell: CHEST_SPELL, ms: (c.secs * 1000.0) as u32 });
    }

    /// A chest opened: what's in it to the opener's bags (what won't fit, to the camp's chest), its
    /// ambush awake. A shrine or relic teaches the hero using it its cards, and stays for the next.
    pub(crate) fn open_chest(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        if !matches!(self.objs.get(&obj).map(|o| &o.kind), Some(ObjKind::Chest { spent: false, .. })) {
            return;
        }
        // One with a trial: its guardians first (a hold's waves too, which teach nothing).
        if !c.trial.is_empty() {
            self.start_trial(id, def, obj);
            return;
        }
        // A shrine or relic stays for every hero to learn from: what's in it, the first time only.
        if !c.teaches.is_empty() {
            let first = !self.camp.opened.contains_key(&c.key);
            self.camp.opened.insert(c.key.clone(), NEVER);
            if first {
                self.give_chest(id, &c);
                self.count("open", &c.key, 1);
                self.count("open", "any", 1);
            } else if !c.text.is_empty() {
                self.tell(id, c.text.clone());
            }
            if self.teach(&[id], &c.teaches, &format!("at {}", c.name)) == 0 {
                self.tell(id, format!("You know all {} has to teach.", c.name));
            }
            if first {
                for k in &c.ambush {
                    self.wake_ambush(k, id);
                }
            }
            self.events.push(Event::Sites);
            self.events.push(Event::Dirty);
            return;
        }
        self.finish_open(id, def, obj, true);
    }

    /// An ordinary chest opened by `id`: spent (full again in its time), what's in it theirs, and
    /// (with `wake`) its ambush awake. A hold's chest is opened this way once it is held, its
    /// guardians having come already.
    fn finish_open(&mut self, id: Id, def: usize, obj: Id, wake: bool) {
        let c = self.content.chests[def].clone();
        let back = if c.respawn > 0.0 { self.wall() + c.respawn as f64 } else { NEVER };
        self.camp.opened.insert(c.key.clone(), back);
        if c.keep {
            if let Some(o) = self.objs.get_mut(&obj) {
                o.kind = ObjKind::Chest { def, spent: true };
            }
            // Drawn again, spent (not to be used).
            self.events.push(Event::ObjGone { id: obj });
            self.events.push(Event::ObjSpawned { id: obj });
        } else {
            self.objs.remove(&obj);
            self.events.push(Event::ObjGone { id: obj });
        }
        self.give_chest(id, &c);
        self.count("open", &c.key, 1);
        self.count("open", "any", 1);
        if wake {
            for k in &c.ambush {
                self.wake_ambush(k, id);
            }
        }
        self.events.push(Event::Sites);
        self.events.push(Event::Dirty);
    }

    /// What's in a chest to the opener's bags (what won't fit, to the camp's chest), and its story.
    fn give_chest(&mut self, id: Id, c: &crate::content::ChestDef) {
        let mut items: BTreeMap<usize, u32> = BTreeMap::new();
        for (k, n) in &c.loot {
            if let Some(i) = self.content.item(k) {
                *items.entry(i).or_default() += n;
            }
        }
        for (k, r) in &c.roll {
            let n = self.roll(*r);
            if let (Some(i), true) = (self.content.item(k), n > 0) {
                *items.entry(i).or_default() += n;
            }
        }
        let mut words = Vec::new();
        let mut banked = Vec::new();
        for (i, n) in items {
            let left = self.give(id, i, n);
            if n > left {
                words.push(format!("{} {}", n - left, self.content.items[i].name));
            }
            if left > 0 {
                *self.storage.entry(i).or_default() += left;
                banked.push(format!("{left} {}", self.content.items[i].name));
            }
        }
        if !c.text.is_empty() {
            self.tell(id, c.text.clone());
        }
        if !words.is_empty() {
            self.tell(id, format!("{} held {}.", c.name, words.join(", ")));
        }
        if !banked.is_empty() {
            self.tell(id, format!("Your bags were full. {} went to storage.", banked.join(", ")));
            self.events.push(Event::Storage);
        }
        self.events.push(Event::Bags { player: id });
    }

    /// These heroes learn cards (each hero's own, kept in their save): those they didn't know,
    /// with a banner each and the news for everyone. How many were new, in all.
    pub(crate) fn teach(&mut self, heroes: &[Id], keys: &[String], how: &str) -> usize {
        let mut all = 0;
        for &h in heroes {
            let Some(p) = self.players.get_mut(&h) else { continue };
            let new: Vec<String> = keys.iter().filter(|k| p.owned.insert((*k).clone())).cloned().collect();
            if new.is_empty() {
                continue;
            }
            all += new.len();
            let name = p.name.clone();
            let names: Vec<String> = new.iter().filter_map(|k| self.content.ability(k).map(|a| self.content.abilities[a].name.clone())).collect();
            let list = names.join(" and ");
            self.count("learn", "any", new.len() as u32);
            self.events.push(Event::Abilities { player: h });
            self.events.push(Event::Banner { player: Some(h), title: format!("Learned: {list}"), sub: "Open the board (N) to put it on a key".into() });
            self.announce(format!("{name} learned {list} {how}."));
        }
        if all > 0 {
            self.events.push(Event::Dirty);
        }
        all
    }

    /// The packs of each wave of a chest's trial: an entry names one pack, or several joined with `+`.
    fn trial_waves(&self, c: &crate::content::ChestDef) -> Vec<Vec<usize>> {
        c.trial.iter().map(|w| w.split('+').filter_map(|key| self.content.packs.iter().position(|p| p.key == key.trim())).collect()).collect()
    }

    /// A hero calls up a trial: a shrine's or a relic's guardians (those fallen since come again at
    /// once), on them. A hold calls its first wave, and the rest as its time runs (`step_trials`).
    /// Nothing to learn, or one already under way: only words.
    fn start_trial(&mut self, id: Id, def: usize, obj: Id) {
        let c = self.content.chests[def].clone();
        let knows = !c.teaches.is_empty() && self.players.get(&id).is_some_and(|p| c.teaches.iter().all(|k| p.owned.contains(k)));
        if knows {
            self.tell(id, format!("You know all {} has to teach.", c.name));
            return;
        }
        if self.trials.contains_key(&def) {
            if let Some(name) = self.trials[&def].walker.and_then(|w| self.mobs.get(&w)).map(|m| self.content.mobs[m.def].name.clone()) {
                self.tell(id, format!("{name} is already on the road. Stay near him."));
            } else if c.hold > 0.0 {
                self.tell(id, format!("{} is under way. Stay near it.", c.name));
            } else {
                self.tell(id, format!("The trial is under way. Defeat {}'s guardians.", c.name));
            }
            return;
        }
        // An escort: its walker wakes where he waits, and the first wave comes for him.
        if !c.escort.is_empty() {
            self.wake_ambush(&c.escort, id);
            let Some(w) = self.escort_walker(def) else { return };
            for k in &c.ambush {
                self.wake_ambush(k, id);
            }
            let now = self.clock;
            self.trials.insert(def, Trial { obj, by: id, near_at: now, started: now, called: 1, walker: Some(w), leg: 0 });
            self.call_wave(def, 0, id);
            let name = self.content.mobs[self.mobs[&w].def].name.clone();
            let title = if c.hold_title.is_empty() { format!("Escort {name}") } else { c.hold_title.clone() };
            self.events.push(Event::Banner { player: Some(id), title, sub: format!("Stay near {name}") });
            return;
        }
        let waves = self.trial_waves(&c);
        let called = if c.hold > 0.0 { 1.min(waves.len()) } else { waves.len() };
        for i in 0..called {
            self.call_wave(def, i, id);
        }
        // A hold's helpers (a friendly shooter in an `ambush` pack) come with it.
        if c.hold > 0.0 {
            for k in &c.ambush {
                self.wake_ambush(k, id);
            }
        }
        let now = self.clock;
        self.trials.insert(def, Trial { obj, by: id, near_at: now, started: now, called, walker: None, leg: 0 });
        if c.hold > 0.0 {
            let title = if c.hold_title.is_empty() { format!("Hold {}", c.name) } else { c.hold_title.clone() };
            self.events.push(Event::Banner { player: Some(id), title, sub: format!("Stay near it for {} sec", c.hold.round() as u32) });
        } else {
            self.events.push(Event::Banner { player: Some(id), title: format!("The Trial of {}", c.name), sub: "Defeat its guardians to learn what it teaches".into() });
            self.tell(id, format!("{} wakes and its guardians attack. Defeat them and everyone nearby learns its abilities.", c.name));
        }
    }

    /// Wave `i` of a trial comes: its packs' guardians (those fallen since stand again now, not
    /// later) awake and on `by`, and its words said to the camp.
    fn call_wave(&mut self, def: usize, i: usize, by: Id) {
        let c = self.content.chests[def].clone();
        let Some(wave) = self.trial_waves(&c).get(i).cloned() else { return };
        for k in wave {
            let pack = self.content.packs[k].clone();
            let Some(mdef) = self.content.mob(&pack.mob) else { continue };
            // Those fallen in an earlier trial stand again now, not later.
            self.respawns.retain(|r| r.2 != Some(k));
            let have = self.mobs.values().filter(|m| m.pack == Some(k) && m.health > 0).count();
            for n in have..pack.count {
                let a = n as f32 / pack.count.max(1) as f32 * std::f32::consts::TAU;
                let r = if pack.count > 1 { pack.radius.max(1.0) } else { 0.0 };
                let mut at = self.ground_near(pack.at[0] + a.cos() * r, pack.at[1] + a.sin() * r, pack.z.unwrap_or(1.0e4));
                // One down in a cave stays on its floor; none in a wall.
                if pack.z.is_some_and(|z| (at[2] - z).abs() > 3.0) || self.land.blocked_near(at[0], at[1], at[2]) {
                    at = self.ground_near(pack.at[0], pack.at[1], pack.z.unwrap_or(1.0e4));
                }
                self.spawn_mob(mdef, at, Some(k), false);
            }
            let key = pack.key.clone();
            // An escort's waves come for the one walked, not for the heroes.
            let foe = self.trials.get(&def).and_then(|t| t.walker).map_or(mobs::Foe::Hero(by), mobs::Foe::Mob);
            self.wake_ambush_on(&key, foe);
        }
        if let Some(text) = c.wave_text.get(i) {
            self.announce(text.clone());
        }
    }

    /// A hold's helpers (friendly creatures of its `ambush` packs) stand down: unseen again.
    fn stand_down_allies(&mut self, def: usize) {
        let c = &self.content.chests[def];
        let packs: Vec<usize> = c.ambush.iter().chain(std::iter::once(&c.escort)).filter_map(|key| self.content.packs.iter().position(|p| !key.is_empty() && p.key == *key)).collect();
        let ids: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.friendly && !m.hidden && m.pack.is_some_and(|k| packs.contains(&k))).map(|(id, _)| *id).collect();
        for id in ids {
            // A walker goes back to where he waits, whole.
            let start = self.mobs[&id].pack.filter(|k| self.content.packs[*k].key == self.content.chests[def].escort).map(|k| self.content.packs[k].at);
            let start = start.map(|a| self.ground(a[0], a[1]));
            if let Some(x) = self.mobs.get_mut(&id) {
                x.hidden = true;
                if let Some(at) = start {
                    x.home = at;
                    x.health = x.max_health;
                }
                x.pos = x.home;
                x.path.clear();
                x.state = mobs::MobState::Idle { wander_at: f32::MAX };
            }
            self.ally_at.remove(&id);
            self.events.push(Event::MobGone { id });
        }
    }

    /// Trials under way: a hold calls its next wave when its time comes; one is passed when the last
    /// guardian falls (and, for a hold, its time has run: everyone near learns what a shrine
    /// teaches, the first finder gets what's in it, a hold's opener gets it and the chest is spent),
    /// and given up when no hero has been near a while (the guardians go back to waiting, whole).
    pub(crate) fn step_trials(&mut self) {
        let now = self.clock;
        for def in self.trials.keys().copied().collect::<Vec<_>>() {
            let c = self.content.chests[def].clone();
            let Some(t) = self.trials.get(&def).cloned() else { continue };
            let Some(at) = self.objs.get(&t.obj).map(|o| o.pos) else {
                self.trials.remove(&def);
                continue;
            };
            let waves = self.trial_waves(&c);
            // An escort: near him is what counts, and he walks; struck down, it is lost.
            let mut at = at;
            let mut walked = true;
            if let Some(w) = t.walker {
                let Some(wpos) = self.mobs.get(&w).filter(|m| m.health > 0 && !m.hidden).map(|m| m.pos) else {
                    self.lose_trial(def);
                    continue;
                };
                at = wpos;
                let leg = self.escort_walks(def, w, t.leg);
                // Wave `i` of `n` comes as he sets off from point `i * len / n` of his route.
                let mut called = t.called;
                let by = if self.players.contains_key(&t.by) { t.by } else { self.players.keys().next().copied().unwrap_or(t.by) };
                while called < waves.len() && leg * waves.len() >= called * c.route.len() {
                    if let Some(x) = self.trials.get_mut(&def) {
                        x.called = called + 1;
                    }
                    self.call_wave(def, called, by);
                    called += 1;
                }
                if let Some(x) = self.trials.get_mut(&def) {
                    x.leg = leg;
                }
                walked = leg >= c.route.len();
            }
            let Some(t) = self.trials.get(&def).cloned() else { continue };
            // A hold calls its waves over its time: wave `i` of `n` at `i * hold / n` sec.
            let mut called = t.called;
            if c.hold > 0.0 {
                let on = if self.players.contains_key(&t.by) { Some(t.by) } else { self.players.iter().find(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= TRIAL_STAY).map(|(id, _)| *id) };
                while let Some(h) = on.filter(|_| called < waves.len() && now - t.started >= called as f32 * c.hold / waves.len() as f32) {
                    self.call_wave(def, called, h);
                    called += 1;
                }
                if let Some(x) = self.trials.get_mut(&def) {
                    x.called = called;
                }
            }
            let packs: Vec<usize> = waves.iter().flatten().copied().collect();
            let up: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| packs.contains(&k)) && m.health > 0 && !m.hidden).map(|(id, _)| *id).collect();
            let called = called.max(t.called);
            let held = called >= waves.len() && now - t.started >= c.hold && walked;
            if up.is_empty() && held {
                self.trials.remove(&def);
                let mut heroes: Vec<Id> = self.players.iter().filter(|(_, p)| p.health > 0 && dist2d(p.pos, at) <= mobs::TEACH_RANGE).map(|(id, _)| *id).collect();
                if self.players.contains_key(&t.by) && !heroes.contains(&t.by) {
                    heroes.push(t.by);
                }
                for h in &heroes {
                    let title = if c.held_title.is_empty() { "Trial Passed".to_string() } else { c.held_title.clone() };
                    self.events.push(Event::Banner { player: Some(*h), title, sub: c.name.clone() });
                }
                if c.teaches.is_empty() {
                    // A hold: whoever opened it is paid, and it is a chest like any other from here.
                    self.stand_down_allies(def);
                    let by = if self.players.contains_key(&t.by) { Some(t.by) } else { heroes.first().copied() };
                    if let Some(by) = by {
                        self.finish_open(by, def, t.obj, false);
                    }
                    continue;
                }
                if !self.camp.opened.contains_key(&c.key) && self.players.contains_key(&t.by) {
                    self.camp.opened.insert(c.key.clone(), NEVER);
                    self.give_chest(t.by, &c);
                    self.count("open", &c.key, 1);
                    self.count("open", "any", 1);
                }
                self.teach(&heroes, &c.teaches, &format!("in the trial of {}", c.name));
                self.events.push(Event::Sites);
                self.events.push(Event::Dirty);
                continue;
            }
            let near = self.players.values().any(|p| p.health > 0 && dist2d(p.pos, at) <= TRIAL_STAY);
            if near {
                if let Some(t) = self.trials.get_mut(&def) {
                    t.near_at = now;
                }
            } else if now - t.near_at > TRIAL_GIVE_UP {
                self.lose_trial(def);
            }
        }
    }

    /// A trial given up (no hero near a while) or an escort lost (its walker struck down): its
    /// guardians go back to waiting, whole and unseen; a hold's or an escort's helpers stand down,
    /// and the camp hears it is lost.
    fn lose_trial(&mut self, def: usize) {
        let c = self.content.chests[def].clone();
        self.trials.remove(&def);
        let packs: Vec<usize> = self.trial_waves(&c).into_iter().flatten().collect();
        let up: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.pack.is_some_and(|k| packs.contains(&k)) && m.health > 0 && !m.hidden).map(|(id, _)| *id).collect();
        for m in up {
            if let Some(x) = self.mobs.get_mut(&m) {
                x.hidden = true;
                x.health = x.max_health;
                x.pos = x.home;
                x.path.clear();
                x.ward = None;
                x.drawn_off = None;
                x.pattern = None;
                x.state = mobs::MobState::Idle { wander_at: f32::MAX };
            }
            self.events.push(Event::MobGone { id: m });
        }
        if c.hold > 0.0 || !c.escort.is_empty() {
            self.stand_down_allies(def);
            let text = if c.lost_text.is_empty() { format!("{} is lost. Use it again to try again.", c.name) } else { c.lost_text.clone() };
            self.announce(text);
        }
    }

    /// The walker of escort `def` (its pack's one creature, standing).
    fn escort_walker(&self, def: usize) -> Option<Id> {
        let key = &self.content.chests[def].escort;
        let k = self.content.packs.iter().position(|p| !key.is_empty() && p.key == *key)?;
        self.mobs.iter().find(|(_, m)| m.pack == Some(k) && m.health > 0).map(|(id, _)| *id)
    }

    /// An escort's walker `w` on his way: the point he makes for, after this moment. He walks to it
    /// at his own pace while a hero is near him and nothing is fighting him, and waits otherwise.
    fn escort_walks(&mut self, def: usize, w: Id, leg: usize) -> usize {
        let route = self.content.chests[def].route.clone();
        let Some(m) = self.mobs.get(&w) else { return leg };
        let (pos, speed) = (m.pos, self.content.mobs[m.def].walk_speed.max(1.0));
        let mut leg = leg;
        while leg < route.len() && dist2d(pos, [route[leg][0], route[leg][1], 0.0]) <= ESCORT_REACHED {
            leg += 1;
        }
        let beset = self.mobs.values().any(|o| o.health > 0 && !o.hidden && o.state.foe() == Some(mobs::Foe::Mob(w)));
        let near = self.players.values().any(|p| p.health > 0 && dist2d(p.pos, pos) <= ESCORT_NEAR);
        if leg < route.len() && near && !beset {
            if self.mobs[&w].path.is_empty() {
                let to = self.ground_near(route[leg][0], route[leg][1], pos[2]);
                self.mobs.get_mut(&w).unwrap().home = to;
                self.go(w, to, speed);
            }
        } else if !self.mobs[&w].path.is_empty() {
            // He stops where he is, and waits.
            let m = self.mobs.get_mut(&w).unwrap();
            m.path.clear();
            m.home = pos;
            self.events.push(Event::MobStop { id: w, pos, facing: self.mobs[&w].facing });
        }
        leg
    }

    /// A blow on the friendly creature the heroes guard (an escort's walker): it takes it (never a
    /// kill for the camp), and struck down, its escort is lost.
    pub(crate) fn hurt_ward(&mut self, w: Id, damage: u32, from: Vec3) {
        let Some(m) = self.mobs.get_mut(&w) else { return };
        if m.health == 0 || m.hidden {
            return;
        }
        m.health = m.health.saturating_sub(damage);
        let (h, max) = (m.health, m.max_health);
        self.events.push(Event::Health { id: w, health: h, max });
        self.events.push(Event::React { id: w, anim: combat::WOUND_ANIM, from, lean: 0.15, hold: 0.0, fit: 0.0 });
        if h == 0 {
            if let Some(def) = self.trials.iter().find(|(_, t)| t.walker == Some(w)).map(|(d, _)| *d) {
                self.lose_trial(def);
            }
        }
    }

    /// Each friendly shooter (`shoots` on its `[[mob]]`) shoots the nearest foe in reach that is
    /// fighting a hero (or one the heroes guard), or a raider on its way to or at a station, every
    /// so often, if it can see it: a hero's helper, as a tower is, and a foe it finishes counts for
    /// the camp. It never shoots an idle pack, nor a hero.
    pub(crate) fn allies_shoot(&mut self) {
        let now = self.clock;
        let shooters: Vec<(Id, Vec3, crate::content::ShootDef)> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.friendly && m.health > 0 && !m.hidden)
            .filter_map(|(id, m)| self.content.mobs[m.def].shoots.clone().map(|sh| (*id, m.pos, sh)))
            .collect();
        self.ally_at.retain(|id, _| shooters.iter().any(|(s, _, _)| s == id));
        let spell = self.content.ranged.first().map_or(0, |r| r.spell);
        for (a, at, sh) in shooters {
            if self.ally_at.get(&a).is_some_and(|n| now < *n) {
                continue;
            }
            let eye = [at[0], at[1], at[2] + 1.4];
            let target = self
                .mobs
                .iter()
                .filter(|(id, m)| **id != a && m.health > 0 && !m.hidden && !m.friendly && !self.content.mobs[m.def].inert())
                .filter(|(_, m)| {
                    matches!(m.state, mobs::MobState::Chase { foe: mobs::Foe::Hero(_) | mobs::Foe::Mob(_) } | mobs::MobState::Windup { foe: mobs::Foe::Hero(_) | mobs::Foe::Mob(_), .. })
                        || (m.raider && !matches!(m.state, mobs::MobState::Dead { .. }))
                })
                .filter(|(_, m)| dist2d(m.pos, at) <= sh.range && crate::land::clear_line(self.land.as_ref(), eye, [m.pos[0], m.pos[1], m.pos[2] + 1.0]))
                .min_by(|x, y| dist2d(x.1.pos, at).total_cmp(&dist2d(y.1.pos, at)))
                .map(|(id, _)| *id);
            let Some(r) = target else { continue };
            self.ally_at.insert(a, now + sh.every);
            self.events.push(Event::Missile { caster: a, spell: if sh.spell != 0 { sh.spell } else { spell }, target: Target::Unit(r) });
            self.events.push(Event::React { id: r, anim: combat::WOUND_ANIM, from: at, lean: 0.15, hold: 0.0, fit: 0.0 });
            self.hurt_mob(r, 0, sh.damage);
        }
    }

    /// Wake an ambush lying in wait (a pack by key) on a hero.
    pub(crate) fn wake_ambush(&mut self, key: &str, on: Id) {
        self.wake_ambush_on(key, mobs::Foe::Hero(on));
    }

    /// Wake an ambush lying in wait (a pack by key) on a foe: a hero, or the one the heroes guard.
    pub(crate) fn wake_ambush_on(&mut self, key: &str, on: mobs::Foe) {
        let Some(k) = self.content.packs.iter().position(|p| p.key == key) else { return };
        // A pack with a condition that doesn't hold stays asleep (a helper of one side of a choice).
        if self.content.packs[k].after.as_deref().is_some_and(|a| !self.camp_has(a)) {
            return;
        }
        let ids: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.pack == Some(k) && m.hidden && m.health > 0).map(|(id, _)| *id).collect();
        for m in &ids {
            self.mobs.get_mut(m).unwrap().hidden = false;
            self.events.push(Event::MobSpawned { id: *m });
        }
        for m in ids {
            self.engage(m, on);
        }
    }

    /// Each tick: places found, secrets come upon, ambushers who see a hero, chests full again.
    pub(crate) fn step_explore(&mut self) {
        self.step_trials();
        // The camp grows as it gets further (looked at now and then, heroes there or not).
        if self.clock - self.grown_at >= REFILL_EVERY {
            self.grown_at = self.clock;
            self.grow_camp(true);
        }
        let heroes: Vec<(Id, Vec3)> = self.players.iter().filter(|(_, p)| p.health > 0).map(|(id, p)| (*id, p.pos)).collect();
        if heroes.is_empty() {
            return;
        }
        // A friendly shooter helps the heroes in their fights.
        self.allies_shoot();
        // Places found.
        for k in 0..self.content.sites.len() {
            let site = &self.content.sites[k];
            if self.camp.found.contains(&site.key) {
                continue;
            }
            let at = [site.at[0], site.at[1], 0.0];
            let Some((by, _)) = heroes.iter().find(|(_, p)| dist2d(*p, at) <= site.radius) else { continue };
            self.found(*by, k);
        }
        // Secrets come upon.
        let unfound: Vec<usize> = self.unfound.iter().copied().collect();
        for def in unfound {
            let c = &self.content.chests[def];
            let at = [c.at[0], c.at[1], 0.0];
            let Some((by, _)) = heroes.iter().find(|(_, p)| dist2d(*p, at) <= c.secret) else { continue };
            let (by, name) = (*by, c.name.clone());
            self.unfound.remove(&def);
            self.set_chest(def, false);
            self.count("secret", "any", 1);
            // Quietly: a line to the finder (no banner and horn, which are for what's coming).
            self.tell(by, format!("You found something hidden: {name}."));
            self.events.push(Event::Dirty);
        }
        // Ambushers lying in wait step out as a hero comes near.
        let waiting: Vec<(Id, Vec3, f32)> = self
            .mobs
            .iter()
            .filter(|(_, m)| m.hidden && m.health > 0)
            .filter_map(|(id, m)| {
                let p = &self.content.packs[m.pack?];
                (p.wake.is_empty() && p.hidden > 0.0 && self.pack_due(m.pack?)).then_some((*id, m.pos, p.hidden))
            })
            .collect();
        for (m, pos, r) in waiting {
            if let Some((h, _)) = heroes.iter().find(|(_, p)| dist2d(*p, pos) <= r) {
                if let Some(x) = self.mobs.get_mut(&m) {
                    x.hidden = false;
                }
                self.events.push(Event::MobSpawned { id: m });
                self.engage(m, mobs::Foe::Hero(*h));
            }
        }
        // Chests full again.
        if self.clock - self.chests_looked >= REFILL_EVERY {
            self.chests_looked = self.clock;
            let wall = self.wall();
            let due: Vec<usize> = (0..self.content.chests.len())
                .filter(|d| self.camp.opened.get(&self.content.chests[*d].key).is_some_and(|b| *b <= wall))
                .collect();
            for def in due {
                let c = self.content.chests[def].clone();
                // Not while a hero stands over it.
                let at = [c.at[0], c.at[1], 0.0];
                if heroes.iter().any(|(_, p)| dist2d(*p, at) < 20.0) {
                    continue;
                }
                self.camp.opened.remove(&c.key);
                // What helped the heroes at it stands down.
                self.stand_down_allies(def);
                let old: Vec<Id> = self.objs.iter().filter(|(_, o)| matches!(o.kind, ObjKind::Chest { def: d, .. } if d == def)).map(|(id, _)| *id).collect();
                for o in old {
                    self.objs.remove(&o);
                    self.events.push(Event::ObjGone { id: o });
                }
                if c.secret > 0.0 {
                    self.unfound.insert(def);
                } else {
                    self.set_chest(def, false);
                }
                self.events.push(Event::Sites);
            }
        }
    }

    /// A place found: the camp's for good, a banner, and its worth into the chest.
    fn found(&mut self, by: Id, k: usize) {
        let site = self.content.sites[k].clone();
        self.camp.found.insert(site.key.clone());
        self.count("explore", "any", 1);
        self.count("explore", &site.kind, 1);
        if site.key != site.kind {
            self.count("explore", &site.key, 1);
        }
        for (i, n) in &site.reward {
            if let Some(i) = self.content.item(i) {
                *self.storage.entry(i).or_default() += n;
            }
        }
        let name = self.players.get(&by).map_or(String::new(), |p| p.name.clone());
        let n = self.camp.found.len();
        let of = self.content.sites.len();
        let reward = if site.reward.is_empty() { String::new() } else { format!(" Sent to storage: {}.", self.cost_text(&site.reward)) };
        // The add-on tells this one quietly by its "Discovered: " start (ES.Banner): keep the words.
        self.events.push(Event::Banner { player: None, title: format!("Discovered: {}", site.name), sub: format!("{n} of {of} places in Elwynn") });
        self.announce(format!("{name} found {}.{reward}", site.name));
        if !site.reward.is_empty() {
            self.events.push(Event::Storage);
        }
        self.events.push(Event::Sites);
        self.events.push(Event::Dirty);
    }

    /// Testing (debug): set a hero down at one of Elwynn's places, or at `x y [z] [facing]`
    /// anywhere, a dungeon's floors too (give z there: the ground has several, and without it the
    /// lowest found is taken). The visual check films a spot of a dungeon without walking there.
    pub fn goto_site(&mut self, id: Id, key: &str) -> Result<String, String> {
        let words: Vec<&str> = key.split_whitespace().collect();
        let nums: Vec<f32> = words.iter().filter_map(|w| w.parse().ok()).collect();
        if words.len() >= 2 && nums.len() == words.len() {
            if self.in_arena(id) {
                return Err("Not from here.".into());
            }
            let at = match nums.get(2) {
                Some(&z) => [nums[0], nums[1], z],
                None => self.ground(nums[0], nums[1]),
            };
            let facing = nums.get(3).copied().unwrap_or(0.0);
            self.put_at(id, [at[0], at[1], at[2] + ARRIVE_LIFT], facing);
            self.land_from_far(id);
            return Ok(format!("{:.1}, {:.1}, {:.1}", at[0], at[1], at[2]));
        }
        let Some(site) = self.content.sites.iter().find(|s| s.key == key).cloned() else {
            let keys: Vec<&str> = self.content.sites.iter().map(|s| s.key.as_str()).collect();
            return Err(format!("No place called {key}. Try {}.", keys.join(", ")));
        };
        if self.is_below() || self.in_arena(id) {
            return Err("Not from here.".into());
        }
        let at = self.ground(site.at[0], site.at[1]);
        self.put_at(id, [at[0], at[1], at[2] + ARRIVE_LIFT], 0.0);
        self.land_from_far(id);
        Ok(site.name)
    }

    /// Every place with whether it's found, for the world map: key, name, where, kind, its words.
    pub fn sites_view(&self) -> Vec<(&crate::content::SiteDef, bool)> {
        self.content.sites.iter().map(|s| (s, self.camp.found.contains(&s.key))).collect()
    }

    /// The name of the place a spot is in or near (none past three of its radii).
    pub fn place_near(&self, at: [f32; 2]) -> Option<String> {
        self.content
            .sites
            .iter()
            .filter(|s| s.kind != "camp")
            .map(|s| (s, ((s.at[0] - at[0]).powi(2) + (s.at[1] - at[1]).powi(2)).sqrt() / s.radius.max(1.0)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .filter(|(_, d)| *d < 3.0)
            .map(|(s, _)| s.name.clone())
    }

    /// Where in Elwynn the camp learns a card, in words: the shrine, relic or champion and the place
    /// it's at ("" for one that's free, or that nothing teaches).
    pub fn learned_where(&self, key: &str) -> String {
        if self.is_siege() {
            return self.card_price(key).map_or(String::new(), |g| format!("{g} gold"));
        }
        let c = &self.content;
        let place = |at: [f32; 2]| self.place_near(at);
        if let Some(ch) = c.chests.iter().find(|ch| ch.teaches.iter().any(|k| k == key)) {
            let at = place(ch.at).map_or(String::new(), |p| format!(", {p}"));
            return if ch.secret > 0.0 { format!("A hidden relic somewhere near{}", at.trim_start_matches(',')) } else { format!("{}{at}", ch.name) };
        }
        if let Some(m) = c.mobs.iter().position(|m| m.teaches.iter().any(|k| k == key)) {
            let at = c.packs.iter().find(|p| p.mob == c.mobs[m].key).and_then(|p| place(p.at)).map_or(String::new(), |p| format!(" near {p}"));
            return format!("When {} falls{at}", c.mobs[m].name);
        }
        String::new()
    }

    /// The chests and secrets the camp has opened, of all there are (secrets, of all secrets).
    pub fn chests_tally(&self) -> (usize, usize, usize, usize) {
        let c = &self.content.chests;
        let opened = |d: &crate::content::ChestDef| self.camp.opened.contains_key(&d.key) || self.camp.counts.contains_key(&format!("open:{}", d.key));
        let all = c.iter().filter(|d| d.secret <= 0.0).count();
        let open = c.iter().filter(|d| d.secret <= 0.0 && opened(d)).count();
        let secrets = c.iter().filter(|d| d.secret > 0.0).count();
        let found = c.iter().filter(|d| d.secret > 0.0 && opened(d)).count();
        (open, all, found, secrets)
    }
}

/// The look of a chest's lock being worked (Opening's cast).
pub const CHEST_SPELL: u32 = 21651;
