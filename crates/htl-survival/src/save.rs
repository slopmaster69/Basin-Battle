//! The survival world on disk: one versioned file holding the camp (chest, stations), the land
//! (each node's state), what fallen heroes left, a raid in progress, and every hero by account.
//! Things are named by key, never by the in-memory ids, so a file outlives a restart.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::content::Content;
use crate::sim::*;

pub const VERSION: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct WorldSave {
    pub version: u32,
    pub next_uid: u64,
    /// The chest, by item key.
    pub storage: BTreeMap<String, u32>,
    pub stations: Vec<StationSave>,
    pub nodes: Vec<NodeSave>,
    pub caches: Vec<CacheSave>,
    pub raid: Option<RaidSave>,
    pub players: BTreeMap<String, PlayerSave>,
    /// How far the camp has come: raids beaten, the journal's goal, what's been done.
    #[serde(default)]
    pub raids_won: u32,
    #[serde(default)]
    pub goal: usize,
    /// The goal by its title (so goals added to the journal later don't move it).
    #[serde(default)]
    pub goal_title: Option<String>,
    #[serde(default)]
    pub counts: BTreeMap<String, u32>,
    /// The world below: what the group has done there.
    #[serde(default)]
    pub below: Option<BelowSave>,
    /// Elwynn's places the camp has found and its chests opened (with when each is full again,
    /// unix seconds). (Cards are each hero's own, in their save.)
    #[serde(default)]
    pub found: Vec<String>,
    #[serde(default)]
    pub opened: BTreeMap<String, f64>,
}

/// The Deadmines as the group left them: lamps lit, doors opened, bosses beaten, strongboxes
/// emptied, and each hero's lamp and Draughts.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct BelowSave {
    #[serde(default)]
    pub lit: Vec<String>,
    #[serde(default)]
    pub open: Vec<String>,
    #[serde(default)]
    pub slain: Vec<String>,
    #[serde(default)]
    pub looted: Vec<String>,
    #[serde(default)]
    pub heroes: BTreeMap<String, BelowHeroSave>,
    /// When this run's first boss fell (unix seconds): the bosses stand again a while after.
    #[serde(default)]
    pub run_at: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BelowHeroSave {
    pub lamp: String,
    pub draughts: u32,
    #[serde(default)]
    pub flasks: u32,
}

/// The journal as it was when saves held only the goal's place in it: an older save's place is
/// read through this.
const FIRST_JOURNAL: [&str; 17] = [
    "Timber for the camp",
    "A fire to cook on",
    "Provisions",
    "Copper from Fargodeep",
    "An anvil",
    "Thin the Warsong",
    "Hold the camp",
    "A rack for hides",
    "Armor of hide",
    "A herbalist's cauldron",
    "Salves for the road",
    "Drive off the Darkspear",
    "The Overseer",
    "Bronze",
    "A wall",
    "A workbench",
    "Break the warband",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StationSave {
    pub uid: u64,
    pub key: String,
    pub pos: [f32; 3],
    pub facing: f32,
    pub health: u32,
    pub damaged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct NodeSave {
    pub key: String,
    pub item: String,
    pub pos: [f32; 3],
    pub facing: f32,
    pub uses: u32,
    pub ready: bool,
    /// Unix seconds it comes back at.
    pub back_at: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CacheSave {
    pub uid: u64,
    pub owner: String,
    pub pos: [f32; 3],
    pub items: BTreeMap<String, u32>,
}

/// A raid in progress, kept waiting across a restart: its phase, its clock, and the raiders still
/// to come (those that had arrived come again).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RaidSave {
    pub warning: bool,
    pub timer: f32,
    pub queue: Vec<String>,
    /// Which raid (how many had been beaten when it was called).
    #[serde(default)]
    pub won_before: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PlayerSave {
    pub pos: [f32; 3],
    pub facing: f32,
    pub health: u32,
    pub stamina: f32,
    pub weapon: String,
    pub shield: String,
    pub bow: String,
    pub bag: BTreeMap<String, u32>,
    /// Down when they left: they get up at the camp on their return.
    pub dead: bool,
    /// What they wear by slot, the pack on their back, and the item R uses.
    #[serde(default)]
    pub armor: BTreeMap<String, String>,
    #[serde(default)]
    pub pack: Option<String>,
    #[serde(default)]
    pub quick: Option<String>,
    /// The ability keys (slot to ability), the tree's nodes owned, and Valor.
    #[serde(default)]
    pub loadout: BTreeMap<String, String>,
    #[serde(default)]
    pub owned: Vec<String>,
    #[serde(default)]
    pub valor: Option<f32>,
    /// A character's own: the login it belongs to (empty: the key is the account's own first
    /// footman), its name, class and look (the footman's when not given).
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub name: String,
    #[serde(default = "warrior")]
    pub class: String,
    #[serde(default)]
    pub look: Option<HeroLook>,
}

fn warrior() -> String {
    "warrior".into()
}

pub fn save_player(c: &Content, p: &Player) -> PlayerSave {
    PlayerSave {
        pos: p.pos,
        facing: p.facing,
        health: p.health,
        stamina: p.stamina,
        weapon: c.items[p.weapon].key.clone(),
        shield: c.items[p.shield].key.clone(),
        bow: c.items[p.bow].key.clone(),
        bag: p.bag.iter().filter(|(_, n)| **n > 0).map(|(i, n)| (c.items[*i].key.clone(), *n)).collect(),
        dead: p.health == 0 || matches!(p.action, Action::Dead { .. }),
        armor: p.armor.iter().map(|(s, i)| (s.clone(), c.items[*i].key.clone())).collect(),
        pack: p.pack.map(|i| c.items[i].key.clone()),
        quick: p.quick.map(|i| c.items[i].key.clone()),
        loadout: p.loadout.clone(),
        owned: p.owned.iter().cloned().collect(),
        valor: Some(p.valor),
        owner: p.owner.clone(),
        name: p.name.clone(),
        class: p.class.clone(),
        look: Some(p.look),
    }
}

/// Cards renamed or replaced since a save was written, old key to new (the class spell redesign,
/// #40): a hero who had the old card owns, and keeps on its key, the card that took its place.
/// One block per class, so changes to different classes don't collide.
const RENAMED: &[(&str, &str)] = &[
    // Warrior
    // Paladin (Holy Shock and Hammer of Justice live on, as Avenger's Shield's EX and Divine
    // Steed's recast: never a card of their own on a key or in a hero's tree)
    ("holy_shock", "avengers_shield"),
    ("hammer_of_justice", "divine_steed"),
    ("divine_protection", "blessing_of_protection"),
    // Hunter
    ("arcane_shot", "aimed_arrow"),
    ("deterrence", "flare"),
    ("volley", "kill_shot"),
    // Rogue
    // Priest
    // Shaman
    // Mage
    // Warlock
    // Druid
];

/// A saved card's key as it is now.
pub fn renamed(k: &str) -> &str {
    RENAMED.iter().find(|(old, _)| *old == k).map_or(k, |(_, new)| new)
}

/// A returning hero is where they left, as hurt as they were: no heal and no trip home for
/// logging out (only a fall sends them to the camp).
pub fn restore_player(c: &Content, p: &mut Player, s: &PlayerSave, height: impl Fn(f32, f32) -> f32) {
    let item = |k: &str, fallback: usize| c.item(k).unwrap_or(fallback);
    p.pos = [s.pos[0], s.pos[1], height(s.pos[0], s.pos[1]).max(s.pos[2])];
    p.facing = s.facing;
    p.health = if s.dead { 0 } else { s.health.min(p.max_health) };
    p.stamina = s.stamina.clamp(0.0, c.player.stamina);
    p.weapon = item(&s.weapon, p.weapon);
    p.shield = item(&s.shield, p.shield);
    p.bow = item(&s.bow, p.bow);
    p.bag = s.bag.iter().filter_map(|(k, n)| Some((c.item(k)?, *n))).collect();
    p.armor = s.armor.iter().filter_map(|(slot, k)| Some((slot.clone(), c.item(k).filter(|i| c.items[*i].kind == crate::ItemKind::Armor)?))).collect();
    p.pack = s.pack.as_deref().and_then(|k| c.item(k)).filter(|i| c.items[*i].kind == crate::ItemKind::Pack);
    p.quick = s.quick.as_deref().and_then(|k| c.item(k));
    p.last_safe = p.pos;
    // The tree as they left it (a dev build owns it all whatever was saved), and their keys where
    // the abilities still exist and are theirs.
    // A card since renamed or replaced (the class redesign) comes back as its successor.
    let saved: std::collections::BTreeSet<String> = s.owned.iter().map(|k| renamed(k).to_string()).filter(|k| c.ability(k).is_some()).collect();
    if !c.dev.full_tree && !s.owned.is_empty() {
        p.owned = saved;
    }
    // Their class's first cards always.
    p.owned.extend(crate::sim::abilities::first_cards(c, &p.class));
    if !s.loadout.is_empty() {
        // Over the fresh keys: what was on each key, where it still goes there; a save from before
        // the board (its keys named otherwise), or with a card since moved to another row (Ice
        // Block to defense), puts each card it had on that card's own row.
        let fits = |k: &str| c.ability(k).filter(|a| !c.abilities[*a].is_ex() && p.owned.contains(k));
        let mut keys = p.loadout.clone();
        let mut set = std::collections::BTreeSet::new();
        let loadout: BTreeMap<String, String> = s.loadout.iter().map(|(slot, k)| (slot.clone(), renamed(k).to_string())).collect();
        for (slot, k) in &loadout {
            if crate::content::SLOTS.contains(&slot.as_str()) && fits(k).is_some_and(|a| c.abilities[a].slot == *slot) {
                keys.insert(slot.clone(), k.clone());
                set.insert(slot.clone());
            }
        }
        for (slot, k) in &loadout {
            if crate::content::SLOTS.contains(&slot.as_str()) && fits(k).is_some_and(|a| c.abilities[a].slot == *slot) {
                continue;
            }
            if let Some(a) = fits(k).filter(|a| c.abilities[*a].on_board()) {
                let row = c.abilities[a].slot.clone();
                if set.insert(row.clone()) {
                    keys.insert(row, k.clone());
                }
            }
        }
        p.loadout = keys;
    }
    if let Some(v) = s.valor {
        p.valor = v.clamp(0.0, c.valor.max);
    }
}

impl Survival {
    /// Everything worth keeping, with `offline` heroes as last saved.
    pub fn save(&self, offline: &BTreeMap<String, PlayerSave>) -> WorldSave {
        let c = &self.content;
        let key = |i: &usize| c.items[*i].key.clone();
        let mut players = offline.clone();
        for p in self.players.values() {
            players.insert(p.account.clone(), save_player(c, p));
        }
        let mut out = WorldSave {
            version: VERSION,
            next_uid: self.next_uid,
            storage: self.storage.iter().map(|(i, n)| (key(i), *n)).collect(),
            players,
            raids_won: self.camp.raids_won,
            goal: self.camp.goal,
            goal_title: self.content.goals.get(self.camp.goal).map(|g| g.title.clone()),
            counts: self.camp.counts.clone(),
            found: self.camp.found.iter().cloned().collect(),
            opened: self.camp.opened.clone(),
            ..WorldSave::default()
        };
        for o in self.objs.values() {
            match &o.kind {
                ObjKind::Station { def, uid, health, damaged } => out.stations.push(StationSave {
                    uid: *uid,
                    key: c.stations[*def].key.clone(),
                    pos: o.pos,
                    facing: o.facing,
                    health: *health,
                    damaged: *damaged,
                }),
                ObjKind::Node { def, key: k, uses, ready, back_at } => out.nodes.push(NodeSave {
                    key: k.clone(),
                    item: c.nodes[*def].id().to_string(),
                    pos: o.pos,
                    facing: o.facing,
                    uses: *uses,
                    ready: *ready,
                    back_at: *back_at,
                }),
                ObjKind::Cache { uid, owner, items } => out.caches.push(CacheSave {
                    uid: *uid,
                    owner: owner.clone(),
                    pos: o.pos,
                    items: items.iter().map(|(i, n)| (key(i), *n)).collect(),
                }),
                _ => {}
            }
        }
        if let Some(b) = &self.below {
            // A hero below has their Draughts in hand: those are what they have.
            let mut heroes = b.heroes.clone();
            if let Some(draught) = self.dungeon_def().and_then(|d| c.item(&d.draught)) {
                for p in self.players.values() {
                    if let Some(h) = heroes.get_mut(&p.account) {
                        h.draughts = p.bag.get(&draught).copied().unwrap_or(0);
                    }
                }
                // ...and that's not carried back up in their save.
                for p in out.players.values_mut() {
                    p.bag.remove(&c.items[draught].key);
                }
            }
            out.below = Some(BelowSave {
                lit: b.lit.iter().cloned().collect(),
                open: b.open.iter().cloned().collect(),
                slain: b.slain.iter().cloned().collect(),
                looted: b.looted.iter().cloned().collect(),
                run_at: b.run_at,
                heroes: heroes
                    .into_iter()
                    .map(|(k, h)| {
                        (
                            k,
                            BelowHeroSave {
                                lamp: h.lamp,
                                draughts: h.draughts,
                                flasks: h.flasks,
                            },
                        )
                    })
                    .collect(),
            });
        }
        if self.raid.active() {
            let mut queue: Vec<String> = self.raid.queue.iter().map(|d| c.mobs[*d].key.clone()).collect();
            queue.extend(self.raid.raiders.iter().filter_map(|r| self.mobs.get(r).filter(|m| m.health > 0).map(|m| c.mobs[m.def].key.clone())));
            out.raid = Some(RaidSave { warning: self.raid.phase == RaidPhase::Warning, timer: self.raid.timer, queue, won_before: self.raid.won_before });
        }
        out
    }

    /// A world from its save: the camp, stations, nodes and caches as they were. Nodes whose kind
    /// has none saved are laid out afresh; creatures start at their packs.
    pub fn load(content: Content, land: Box<dyn crate::Land>, seed: u64, wall_now: f64, save: &WorldSave) -> Self {
        let mut s = Self::new(content, land, seed, wall_now);
        // Forget the fresh nodes where the save has its own.
        let saved_kinds: Vec<String> = save.nodes.iter().map(|n| n.item.clone()).collect();
        // (A node's `item` in the save is its kind's key: its item's, unless it has its own.)
        let fresh: Vec<Id> = s
            .objs
            .iter()
            .filter(|(_, o)| matches!(&o.kind, ObjKind::Node { def, .. } if saved_kinds.iter().any(|k| k == s.content.nodes[*def].id())))
            .map(|(id, _)| *id)
            .collect();
        for id in fresh {
            s.objs.remove(&id);
        }
        for n in &save.nodes {
            let Some(def) = s.content.nodes.iter().position(|d| d.id() == n.item) else { continue };
            let id = s.alloc();
            s.objs.insert(
                id,
                Obj { kind: ObjKind::Node { def, key: n.key.clone(), uses: n.uses, ready: n.ready, back_at: n.back_at }, pos: n.pos, facing: n.facing },
            );
        }
        for st in &save.stations {
            let Some(def) = s.content.station(&st.key) else { continue };
            let id = s.alloc();
            s.objs.insert(id, Obj { kind: ObjKind::Station { def, uid: st.uid, health: st.health, damaged: st.damaged }, pos: st.pos, facing: st.facing });
            if st.damaged {
                let fire = s.alloc();
                s.objs.insert(fire, Obj { kind: ObjKind::Fire { on: id }, pos: st.pos, facing: 0.0 });
            }
        }
        for c in &save.caches {
            let items: BTreeMap<usize, u32> = c.items.iter().filter_map(|(k, n)| Some((s.content.item(k)?, *n))).collect();
            let id = s.alloc();
            s.objs.insert(id, Obj { kind: ObjKind::Cache { uid: c.uid, owner: c.owner.clone(), items }, pos: c.pos, facing: 0.0 });
        }
        s.storage = save.storage.iter().filter_map(|(k, n)| Some((s.content.item(k)?, *n))).collect();
        s.next_uid = s.next_uid.max(save.next_uid).max(
            save.stations.iter().map(|x| x.uid + 1).chain(save.caches.iter().map(|x| x.uid + 1)).max().unwrap_or(1),
        );
        if let Some(r) = &save.raid {
            s.raid.phase = if r.warning { RaidPhase::Warning } else { RaidPhase::Active };
            s.raid.timer = r.timer;
            s.raid.queue = r.queue.iter().filter_map(|k| s.content.mob(k)).collect();
            let content = &s.content;
            s.raid.queue.sort_by_key(|d| content.mobs[*d].boss);
            s.raid.suspended = true;
            s.raid.won_before = r.won_before;
        }
        // The goal it was on, by title (an older save: by its place in the first journal).
        let title = save.goal_title.clone().or_else(|| FIRST_JOURNAL.get(save.goal).map(|t| t.to_string()));
        let goal = match title {
            Some(t) => s.content.goals.iter().position(|g| g.title == t).unwrap_or(save.goal),
            // Past the end of the first journal: done with it, so at the end of this one.
            None if save.goal >= FIRST_JOURNAL.len() => s.content.goals.len(),
            None => save.goal,
        };
        s.camp = Camp {
            raids_won: save.raids_won,
            goal,
            counts: save.counts.clone(),
            found: save.found.iter().cloned().collect(),
            opened: save.opened.clone(),
        };
        // What it opened stays open; what it found, found.
        s.replace_chests();
        s.events.clear();
        s
    }
}

impl Survival {
    /// The Deadmines from their save: lamps, doors, bosses and strongboxes as the group left them,
    /// and the sacks waiting.
    pub fn load_dungeon(
        content: Content,
        land: Box<dyn crate::Land>,
        seed: u64,
        wall_now: f64,
        save: &WorldSave,
    ) -> Self {
        let mut s = Self::bare(content.for_dungeon(), land, seed, wall_now);
        let hazards = s.dungeon_def().map_or(0, |d| d.hazards.len());
        let mut below = Below {
            beats: vec![0.0; hazards],
            ..Below::default()
        };
        if let Some(b) = &save.below {
            below.lit = b.lit.iter().cloned().collect();
            below.open = b.open.iter().cloned().collect();
            below.slain = b.slain.iter().cloned().collect();
            below.looted = b.looted.iter().cloned().collect();
            below.run_at = b.run_at;
            below.heroes = b
                .heroes
                .iter()
                .map(|(k, h)| {
                    (
                        k.clone(),
                        BelowHero {
                            lamp: h.lamp.clone(),
                            draughts: h.draughts,
                            flasks: h.flasks,
                        },
                    )
                })
                .collect();
        }
        s.below = Some(below);
        s.place_below();
        s.place_packs();
        for c in &save.caches {
            let items: BTreeMap<usize, u32> = c
                .items
                .iter()
                .filter_map(|(k, n)| Some((s.content.item(k)?, *n)))
                .collect();
            let id = s.alloc();
            s.objs.insert(
                id,
                Obj {
                    kind: ObjKind::Cache {
                        uid: c.uid,
                        owner: c.owner.clone(),
                        items,
                    },
                    pos: c.pos,
                    facing: 0.0,
                },
            );
        }
        s.next_uid = s
            .next_uid
            .max(save.next_uid)
            .max(save.caches.iter().map(|x| x.uid + 1).max().unwrap_or(1));
        s.camp = Camp {
            raids_won: 0,
            goal: 0,
            counts: save.counts.clone(),
            ..Camp::default()
        };
        s.events.clear();
        s
    }
}

pub fn to_toml(save: &WorldSave) -> anyhow::Result<String> {
    Ok(toml::to_string(save)?)
}

pub fn from_toml(text: &str) -> anyhow::Result<WorldSave> {
    let s: WorldSave = toml::from_str(text)?;
    if s.version > VERSION {
        anyhow::bail!("this survival save is from a newer version ({})", s.version);
    }
    Ok(s)
}
