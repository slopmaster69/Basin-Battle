//! Heroes as characters: a new one's first save (its class's first cards on its keys, its look),
//! and, for testing, a hero or the whole camp started over.

use super::*;

impl Survival {
    /// A new character's first save: at the camp, as a fresh hero of its class (its column's Q and
    /// E cards on those keys), with the look the creation screen gave it.
    pub fn fresh_save(&self, owner: &str, name: &str, class: &str, look: HeroLook) -> crate::save::PlayerSave {
        let c = &self.content;
        let spawn = self.ground(c.world.spawn[0], c.world.spawn[1]);
        let (loadout, owned) = self.fresh_abilities(class);
        let item = |k: &str| c.item(k).map(|i| c.items[i].key.clone()).unwrap_or_default();
        crate::save::PlayerSave {
            pos: spawn,
            facing: c.world.spawn_facing,
            health: c.player.health,
            stamina: c.player.stamina,
            weapon: item(&c.start.weapon),
            shield: item(&c.start.shield),
            bow: item(&c.start.bow),
            bag: c.start.items.clone(),
            dead: false,
            armor: BTreeMap::new(),
            pack: None,
            quick: None,
            loadout,
            owned: owned.into_iter().collect(),
            valor: Some(c.valor.start),
            owner: owner.to_string(),
            name: name.to_string(),
            class: class.to_string(),
            look: Some(look),
        }
    }

    /// Testing: a hero back to their first day (bags, gear, cards, Valor, at the camp), still who
    /// they are (name, class, look). Not below, nor in the ring.
    pub fn reset_hero(&mut self, id: Id) -> Result<(), String> {
        let p = self.players.get(&id).ok_or("No hero.")?;
        if self.is_below() {
            return Err("Not below: go home first.".into());
        }
        if self.in_arena(id) {
            return Err("Leave the ring first.".into());
        }
        let (account, owner, name, class, look) = (p.account.clone(), p.owner.clone(), p.name.clone(), p.class.clone(), p.look);
        let save = self.fresh_save(&owner, &name, &class, look);
        self.cancel_preview(id);
        // A fresh hero made the usual way, then put in this one's place (their id stays).
        let fresh = self.join(&account, &name, Some(&save));
        let mut p = self.players.remove(&fresh).expect("just joined");
        let old = self.players.get(&id).map(|o| (o.rtt, o.local_moves, o.shoulder_cam));
        if let Some((rtt, local, cam)) = old {
            p.rtt = rtt;
            p.local_moves = local;
            p.shoulder_cam = cam;
        }
        let (pos, facing) = (p.pos, p.facing);
        self.players.insert(id, p);
        self.cc.remove(&id);
        self.stances.remove(&id);
        self.anchors.remove(&id);
        self.grace.remove(&id);
        for m in self.mobs.values_mut() {
            if m.state.target() == Some(id) {
                m.state = mobs::MobState::Return;
            }
        }
        self.put_at(id, [pos[0], pos[1], pos[2] + ARRIVE_LIFT], facing);
        self.land_from_far(id);
        let health = self.players[&id].health;
        let max = self.players[&id].max_health;
        self.events.push(Event::Health { id, health, max });
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::Abilities { player: id });
        self.events.push(Event::Effects { player: id });
        self.events.push(Event::Gear { player: id });
        self.events.push(Event::Dirty);
        self.tell(id, "You start over: the camp's first day, your class's first two cards (Q and E), an axe, a buckler, a bow and a roast.");
        Ok(())
    }

    /// Testing: the camp's world started over, as on a new world's first day: the chest emptied,
    /// the stations gone, the journal at its start, no raid beaten, no place found nor chest
    /// opened, every creature and resource back where it begins. The heroes stay who they are
    /// (each can start over with `reset_hero`), set down at the camp. (The server starts the
    /// Deadmines over with it: [`Survival::reset_below`].)
    pub fn reset_camp(&mut self) -> Result<(), String> {
        if self.is_below() {
            return Err("Not from below.".into());
        }
        let in_ring: Vec<Id> = self.players.keys().copied().filter(|h| self.in_arena(*h)).collect();
        for h in in_ring {
            self.leave_arena(h, arena::Exit::Home);
        }
        self.raid_reset();
        for p in self.players.keys().copied().collect::<Vec<_>>() {
            self.cancel_preview(p);
        }
        self.clear_world();
        self.unfound.clear();
        self.storage.clear();
        self.camp = Camp::default();
        self.arena = arena::Arena::default();
        let before: BTreeSet<Id> = self.objs.keys().copied().collect();
        self.place_camp();
        self.place_arena();
        self.place_nodes(None);
        self.place_chests();
        self.place_props();
        self.place_packs();
        self.draw_new(&before);
        let w = self.content.world.clone();
        let spawn = self.ground(w.spawn[0], w.spawn[1]);
        for p in self.players.keys().copied().collect::<Vec<_>>() {
            self.put_at(p, [spawn[0], spawn[1], spawn[2] + ARRIVE_LIFT], w.spawn_facing);
            self.land_from_far(p);
        }
        self.events.push(Event::Storage);
        self.events.push(Event::Goal);
        self.events.push(Event::Sites);
        self.events.push(Event::Dirty);
        self.announce("The camp starts over: a new world's first day.");
        Ok(())
    }

    /// Everything laid out in the world gone (drawn gone for everyone): its objects and its
    /// creatures, and all in flight or on the ground.
    fn clear_world(&mut self) {
        let objs: Vec<Id> = self.objs.keys().copied().collect();
        for o in objs {
            self.objs.remove(&o);
            self.events.push(Event::ObjGone { id: o });
        }
        let mobs: Vec<Id> = self.mobs.keys().copied().collect();
        for m in mobs {
            self.mobs.remove(&m);
            self.events.push(Event::MobGone { id: m });
        }
        self.respawns.clear();
        self.patrols.clear();
        self.drop_projectiles();
        self.grounds.clear();
        self.burnings.clear();
        self.totems.clear();
        self.landings.clear();
        self.traps.clear();
        self.windups.clear();
        self.missiles.clear();
        self.pending.clear();
    }

    /// What's new since `before`, drawn for everyone (what says so itself already has).
    fn draw_new(&mut self, before: &BTreeSet<Id>) {
        let drawn: BTreeSet<Id> = self.events.iter().filter_map(|e| if let Event::ObjSpawned { id } = e { Some(*id) } else { None }).collect();
        let new: Vec<Id> = self.objs.keys().copied().filter(|o| !before.contains(o) && !drawn.contains(o)).collect();
        for o in new {
            self.events.push(Event::ObjSpawned { id: o });
        }
    }

    /// The Deadmines start over. `full` (the camp started over): as on the first day, no lamp
    /// lit, no door opened, the heroes below set down at the first lamp. Else (a run's reset,
    /// for its loot): its bosses and their crews stand again and its strongboxes fill, the lamps,
    /// gates and shortcuts as they were.
    pub fn reset_below(&mut self, full: bool) {
        let Some(d) = self.dungeon_def().cloned() else { return };
        self.clear_world();
        let first = d.lamps.first().map(|l| l.key.clone()).unwrap_or_default();
        let online: Vec<String> = self.players.values().map(|p| p.account.clone()).collect();
        if let Some(b) = self.below.as_mut() {
            b.slain.clear();
            b.looted.clear();
            b.fallen.clear();
            b.fog.clear();
            b.run_at = None;
            b.heroes_seen = 0;
            if full {
                b.lit.clear();
                b.open.clear();
                b.heroes.retain(|k, _| online.contains(k));
                for h in b.heroes.values_mut() {
                    h.lamp = first.clone();
                    h.flasks = 0;
                }
            }
        }
        let before: BTreeSet<Id> = self.objs.keys().copied().collect();
        self.place_below();
        self.place_packs();
        self.draw_new(&before);
        if full {
            for p in self.players.keys().copied().collect::<Vec<_>>() {
                self.arrive_at(p, &first);
            }
            let place = self.dungeon_def().map_or_else(|| "The dungeon".to_string(), |d| d.name.clone());
            self.announce(format!("{place} starts over with the camp: no lamp lit, no door open."));
        } else {
            let place = self.dungeon_def().map_or_else(|| "The dungeon".to_string(), |d| d.name.clone());
            self.announce(format!("{place}: its bosses stand again, and its strongboxes are full."));
        }
        self.events.push(Event::Dirty);
    }
}
