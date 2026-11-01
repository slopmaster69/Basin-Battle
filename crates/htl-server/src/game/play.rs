//! Playing the sim's events out as packets.

use htl_sim::{EntityView, Event, MenuKind, SpellTarget, What};

use super::*;
use crate::packets::Aim;

/// The greeting text each kind of dialog opens with.
pub(crate) fn menu_text_id(kind: MenuKind) -> u32 {
    match kind {
        MenuKind::Card => 90001,
        MenuKind::Relic => 90002,
        MenuKind::Legendary => 90003,
        MenuKind::Banish | MenuKind::Lock => 90004,
        MenuKind::SetPiece => 90005,
        MenuKind::Forge => 90006,
        MenuKind::Banner => 90007,
        MenuKind::Cache => 90008,
    }
}


/// Hunter shots cast from the ranged slot (Auto, Multi-, Arcane and Aimed Shot).
const RANGED_SLOT_SHOTS: [u32; 4] = [75, 2643, 3044, 19434];

impl Game {
    pub(crate) fn play(&mut self, events: Vec<Event>) {
        if self.in_world().next().is_none() {
            // Nobody is watching; still keep the guid book straight.
            for e in &events {
                match e {
                    Event::SpawnEnemy { id, kind, elite, .. } => {
                        let twist = self.sim.enemy_modifier(*id);
                        self.bind(*id, creature_guid(entry_of(*kind, *elite, twist), *id))
                    }
                    Event::SpawnNpc { id, npc, .. } => self.bind(*id, creature_guid(npc_entry(*npc), *id)),
                    Event::SpawnAlly { id, owner, pet, .. } => {
                        let entry = if *pet { PET_ENTRY } else if *owner == self.sim.hub_id() { DEFENDER_ENTRY } else { ALLY_ENTRY };
                        self.bind(*id, creature_guid(entry, *id))
                    }
                    Event::NpcChanged { id, npc } => {
                        self.unbind(*id);
                        self.bind(*id, creature_guid(npc_entry(*npc), *id));
                    }
                    Event::Despawn { id } => self.unbind(*id),
                    Event::RunOver(r) => self.run_over(r.clone()),
                    Event::Victory(r) => self.victory(r.clone()),
                    _ => {}
                }
            }
            return;
        }
        for e in events {
            self.event(e);
        }
    }

    fn guid(&self, id: EntityId) -> Option<u64> {
        self.guids.get(&id).copied()
    }

    fn to_player(&self, player: EntityId, p: Packet) {
        if let Some(c) = self.conn_of(player) {
            self.send(c, p);
        }
    }

    fn create_unit(&mut self, view: EntityView, guid: u64) {
        self.bind(view.id, guid);
        if let Some(p) = self.unit_create(&view, guid) {
            self.broadcast(&p);
        }
    }

    fn event(&mut self, e: Event) {
        match e {
            Event::SpawnEnemy { id, kind, elite, level, pos, facing, health, max_health } => {
                let view = EntityView {
                    id,
                    what: What::Enemy { kind, elite },
                    pos,
                    facing,
                    health,
                    max_health,
                    level,
                    alive: true,
                    moving: None,
                    attacking: None,
                };
                let twist = self.sim.enemy_modifier(id);
                self.create_unit(view, creature_guid(entry_of(kind, elite, twist), id));
            }
            Event::SpawnNpc { id, npc, base, pos, facing } => {
                let view = EntityView {
                    id,
                    what: What::Npc { npc, base },
                    pos,
                    facing,
                    health: 1,
                    max_health: 1,
                    level: 1,
                    alive: true,
                    moving: None,
                    attacking: None,
                };
                self.create_unit(view, creature_guid(npc_entry(npc), id));
            }
            Event::SpawnAlly { id, owner, level, pos, facing, health, pet } => {
                let view = EntityView {
                    id,
                    what: What::Ally { owner, pet },
                    pos,
                    facing,
                    health,
                    max_health: health,
                    level,
                    alive: true,
                    moving: None,
                    attacking: None,
                };
                let entry = if pet { PET_ENTRY } else if owner == self.sim.hub_id() { DEFENDER_ENTRY } else { ALLY_ENTRY };
                self.create_unit(view, creature_guid(entry, id));
            }
            Event::NpcChanged { id, npc } => {
                // A new look and name: the client caches names by entry, so it's a new creature.
                if let Some(old) = self.guid(id) {
                    self.broadcast(&packets::destroy_object(old));
                }
                self.unbind(id);
                if let Some(view) = self.sim.entities().into_iter().find(|v| v.id == id) {
                    let _ = npc;
                    let guid = creature_guid(npc_entry(npc), id);
                    self.create_unit(view, guid);
                }
            }
            Event::Despawn { id } => {
                if let Some(guid) = self.guid(id) {
                    self.broadcast(&packets::destroy_object(guid));
                }
                self.unbind(id);
            }
            Event::MoveTo { id, path, speed } => {
                if let Some(guid) = self.guid(id) {
                    if let Some(p) = self.move_packet(guid, &path, speed) {
                        self.broadcast(&p);
                    }
                }
            }
            Event::Stop { id, pos, facing } => {
                if let Some(guid) = self.guid(id) {
                    self.spline_id = self.spline_id.wrapping_add(1);
                    let p = packets::monster_face(guid, self.spline_id, pos, facing);
                    self.broadcast(&p);
                }
            }
            Event::AttackStart { attacker, victim } => {
                if let (Some(a), Some(v)) = (self.guid(attacker), self.guid(victim)) {
                    self.broadcast(&packets::attack_start(a, v));
                    let mut f = Fields::new();
                    f.set_u64(idx::UNIT_TARGET, v);
                    self.broadcast(&packets::update_object(&[], &[(a, &f)]));
                }
            }
            Event::AttackStop { attacker, victim } => {
                if let (Some(a), Some(v)) = (self.guid(attacker), self.guid(victim)) {
                    self.broadcast(&packets::attack_stop(a, v));
                    let mut f = Fields::new();
                    f.set_u64(idx::UNIT_TARGET, 0);
                    self.broadcast(&packets::update_object(&[], &[(a, &f)]));
                }
            }
            Event::Hit { attacker, victim, damage, crit } => {
                if let (Some(a), Some(v)) = (self.guid(attacker), self.guid(victim)) {
                    self.broadcast(&packets::attacker_state_update(a, v, damage, crit));
                }
            }
            Event::SpellGo { caster, spell, target, hits } => {
                let Some(c) = self.guid(caster) else { return };
                let aim = match target {
                    SpellTarget::Unit(u) if u == caster => Aim::Caster,
                    SpellTarget::Unit(u) => match self.guid(u) {
                        Some(g) => Aim::Unit(g),
                        None => return,
                    },
                    SpellTarget::Point(p) => Aim::Point(p),
                };
                let hits: Vec<u64> = hits.iter().filter_map(|h| self.guid(*h)).collect();
                // Shots fly with an arrow.
                // (A wand's Shoot flies the wand's own bolt.)
                let ammo = if spell == htl_sim::RIFLE_SHOT {
                    Some(packets::BULLET_DISPLAY)
                } else {
                    matches!(spell, 75 | 2643 | 3044 | 19434).then_some(packets::ARROW_DISPLAY)
                };
                self.broadcast(&packets::spell_go(c, spell, aim, &hits, ammo));
            }
            Event::SpellHit { caster, victim, spell, damage, crit, school, periodic } => {
                if let (Some(a), Some(v)) = (self.guid(caster), self.guid(victim)) {
                    self.broadcast(&packets::spell_damage(v, a, spell, damage, school, crit, periodic));
                }
            }
            Event::Heal { healer, target, spell, amount, crit } => {
                if let (Some(h), Some(t)) = (self.guid(healer), self.guid(target)) {
                    self.broadcast(&packets::spell_heal(t, h, spell, amount, crit));
                }
            }
            Event::Health { id, health, max_health } => {
                if let Some(guid) = self.guid(id) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, health).set(idx::UNIT_MAXHEALTH, max_health);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::Died { id } => {
                if let Some(guid) = self.guid(id) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, 0).set_u64(idx::UNIT_TARGET, 0);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::Revived { id, health, max_health } => {
                if let Some(guid) = self.guid(id) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_HEALTH, health).set(idx::UNIT_MAXHEALTH, max_health).set(idx::UNIT_DYNAMIC_FLAGS, 0);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::NotInRange { player } => self.to_player(player, Packet::empty(op::SMSG_ATTACKSWING_NOTINRANGE)),
            Event::Announce { text, big } => {
                println!("[game] {text}");
                if big {
                    // Add-on players see it in their own banner, and in chat as a plain line.
                    let rest = self.ui_announce(&text);
                    for (c, _, w) in self.in_world() {
                        let kind = if !w.addon && rest.contains(c) { packets::CHAT_RAID_WARNING } else { packets::CHAT_SYSTEM };
                        self.send(*c, packets::chat(kind, packets::LANG_UNIVERSAL, 0, &text));
                    }
                } else {
                    self.broadcast(&packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text));
                }
            }
            Event::Splash { style, title, subtitle } => self.ui_splash(style.key(), &title, &subtitle),
            Event::Victory(r) => self.victory(r),
            Event::Dodged { player, name } => {
                if let Some(c) = self.conn_of(player).filter(|c| self.has_addon(*c)) {
                    self.ui_line(c, &format!("DG~{}", name.replace('~', " ")));
                }
            }
            Event::CastStart { caster, spell, ms, target } => {
                let Some(guid) = self.guid(caster) else { return };
                let aim = target.and_then(|t| self.guid(t)).map_or(Aim::Caster, Aim::Unit);
                self.broadcast(&packets::spell_start(guid, spell, ms, aim));
                // The client opens no cast bar of its own for a shot from the ranged slot (Aimed
                // Shot) that the server started: the add-on draws one.
                if RANGED_SLOT_SHOTS.contains(&spell) && ms > 0 {
                    if let Some(c) = self.conn_of(caster).filter(|c| self.has_addon(*c)) {
                        let name = self.sim.spell_bar(caster).and_then(|b| b.slots.iter().find(|s| s.icon_spell == spell).map(|s| s.name.clone())).unwrap_or_default();
                        self.ui_line(c, &format!("MC~{}~{ms}", name.replace('~', " ")));
                    }
                }
            }
            Event::CastStop { caster, spell } => {
                if RANGED_SLOT_SHOTS.contains(&spell) {
                    if let Some(c) = self.conn_of(caster).filter(|c| self.has_addon(*c)) {
                        self.ui_line(c, "MC~");
                    }
                }
                let Some(guid) = self.guid(caster) else { return };
                self.to_player(caster, packets::cast_result(spell, Some(packets::SPELL_FAILED_INTERRUPTED)));
                let others = packets::spell_failed_other(guid, spell);
                for (c, _, w) in self.in_world() {
                    if w.entity != caster {
                        self.send(*c, others.clone());
                    }
                }
            }
            Event::CastFailed { player, error } => {
                if let Some(c) = self.conn_of(player) {
                    if self.has_addon(c) {
                        self.ui_line(c, &format!("E~{}", error.text()));
                    }
                }
            }
            Event::Paused { choosing } => {
                let line = format!("PZ~{}", choosing.join(", ").replace('~', " "));
                for (c, _, w) in self.in_world() {
                    if w.addon && w.addon_version >= 4 {
                        self.ui_line(*c, &line);
                    }
                }
                // Nobody walks into position while the world waits: everyone holds still.
                let root = !choosing.is_empty();
                let conns: Vec<(u64, EntityId)> = self.in_world().map(|(c, _, w)| (*c, w.entity)).collect();
                for (c, entity) in conns {
                    let Some(guid) = self.guid(entity) else { continue };
                    let send = self.world_mut(c).and_then(|w| {
                        if w.rooted == root {
                            return None;
                        }
                        w.rooted = root;
                        w.counter += 1;
                        Some(w.counter)
                    });
                    if let Some(counter) = send {
                        self.send(c, packets::force_move_root(guid, counter, root));
                    }
                }
            }
            Event::Ping { text, help, player } => {
                println!("[game] {text}");
                if help {
                    if let Some(pos) = self.sim.entities().into_iter().find(|e| e.id == player).map(|e| e.pos) {
                        self.pings.retain(|(at, _)| at.elapsed().as_secs() < 10);
                        self.pings.push((std::time::Instant::now(), pos));
                    }
                }
                for (c, _, w) in self.in_world() {
                    if w.addon && w.addon_version >= 4 {
                        self.ui_line(*c, &format!("PG~{}~{}", u8::from(help), text.replace('~', " ")));
                    } else {
                        let kind = if help { packets::CHAT_RAID_WARNING } else { packets::CHAT_SYSTEM };
                        self.send(*c, packets::chat(kind, packets::LANG_UNIVERSAL, 0, &text));
                    }
                }
            }
            Event::CardTaken { player, card, rank } => self.ui_card_taken(player, &card, rank),
            Event::CooldownCleared { player, spell } => {
                if let Some(guid) = self.guid(player) {
                    self.to_player(player, packets::clear_cooldown(spell, guid));
                    // A broken Hearthstone cast fails like any cast, on every screen.
                    if spell == htl_sim::sim::HEARTH_SPELL_ID {
                        self.to_player(player, packets::cast_result(spell, Some(packets::SPELL_FAILED_INTERRUPTED)));
                        let others = packets::spell_failed_other(guid, spell);
                        for (c, _, w) in self.in_world() {
                            if w.entity != player {
                                self.send(*c, others.clone());
                            }
                        }
                    }
                }
            }
            Event::Power { player, power } => {
                if let Some(guid) = self.guid(player) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_POWER1 + 1, power);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::Talents { player } => {
                if let Some(conn) = self.conn_of(player) {
                    if self.has_addon(conn) {
                        self.ui_talents(conn, player);
                    }
                }
            }
            Event::Proc { player, name, icon } => {
                if let Some(conn) = self.conn_of(player) {
                    if self.has_addon(conn) {
                        let line = format!("PR~{}~{}", name.replace(['~', '\n'], " "), self.spell_icon(icon));
                        self.ui_line(conn, &line);
                    }
                }
            }
            Event::Bar { player } => {
                if let Some(conn) = self.conn_of(player) {
                    if self.has_addon(conn) {
                        self.ui_bar(conn, player);
                    }
                }
            }
            Event::EnemyCast { caster, name, spell, ms, warn, endangered, big, interruptible } => {
                if let Some(guid) = self.guid(caster) {
                    self.broadcast(&packets::spell_start(guid, spell, ms, Aim::Caster));
                }
                self.ui_enemy_cast(caster, &name, ms, &warn, &endangered, big, interruptible);
            }
            Event::CastInterrupted { caster, spell } => {
                if let Some(guid) = self.guid(caster) {
                    self.broadcast(&packets::spell_failed_other(guid, spell));
                }
                for c in self.in_world().filter(|(_, _, w)| w.addon).map(|(c, _, _)| *c).collect::<Vec<_>>() {
                    self.ui_line(c, "XI~");
                }
            }
            Event::Tell { player, text } => {
                self.to_player(player, packets::chat(packets::CHAT_SYSTEM, packets::LANG_UNIVERSAL, 0, &text));
            }
            Event::Status(s) => self.ui_status(s),
            Event::Log(text) => println!("[balance] {text}"),
            Event::Environmental { id, kind, damage } => {
                if let Some(guid) = self.guid(id) {
                    self.broadcast(&packets::environmental_damage(guid, kind as u8, damage));
                }
            }
            Event::UnitLevel { id, level } => {
                let Some(guid) = self.guid(id) else { return };
                let mut f = Fields::new();
                f.set(idx::UNIT_LEVEL, level);
                self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
            }
            Event::Level { player, level, gained_health } => {
                let Some(guid) = self.guid(player) else { return };
                let next = self.sim.player_view(player).map_or(0, |v| v.next_xp);
                let xp = self.sim.player_view(player).map_or(0, |v| v.xp);
                if gained_health > 0 {
                    self.to_player(player, packets::level_up(level, gained_health));
                }
                let mut public = Fields::new();
                public.set(idx::UNIT_LEVEL, level);
                self.broadcast(&packets::update_object(&[], &[(guid, &public)]));
                let mut private = Fields::new();
                private.set(idx::PLAYER_XP, xp).set(idx::PLAYER_NEXT_LEVEL_XP, next);
                self.to_player(player, packets::update_object(&[], &[(guid, &private)]));
            }
            Event::Xp { player, amount, victim } => {
                let Some(guid) = self.guid(player) else { return };
                // Only the bar moves: the client floats "XP: n" over the hero for every kill
                // otherwise, which buried the fight in purple numbers.
                let _ = (victim, amount);
                let xp = self.sim.player_view(player).map_or(0, |v| v.xp);
                let mut f = Fields::new();
                f.set(idx::PLAYER_XP, xp);
                self.to_player(player, packets::update_object(&[], &[(guid, &f)]));
            }
            Event::Gold { player, gold } => {
                let Some(guid) = self.guid(player) else { return };
                let mut f = Fields::new();
                f.set(idx::PLAYER_COINAGE, gold.saturating_mul(10_000));
                self.to_player(player, packets::update_object(&[], &[(guid, &f)]));
            }
            Event::Stats { player } => {
                let Some(guid) = self.guid(player) else { return };
                if let Some(f) = self.stat_fields(player) {
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
                // Speed is the client's to apply: tell it when it changes.
                let speed = self.sim.player_speed(player);
                if let Some(c) = self.conn_of(player) {
                    let send = self.world_mut(c).and_then(|w| {
                        if (w.speed - speed).abs() > 0.01 {
                            w.speed = speed;
                            w.counter += 1;
                            Some(w.counter)
                        } else {
                            None
                        }
                    });
                    if let Some(counter) = send {
                        self.send(c, packets::force_run_speed(guid, counter, speed));
                    }
                }
            }
            Event::Aura { unit, slot, spell, stacks, secs } => {
                let Some(guid) = self.guid(unit) else { return };
                let bar = self.auras.entry(unit).or_insert([(0, 0); 48]);
                let slot = (slot as usize).min(47);
                bar[slot] = if spell == 0 { (0, 0) } else { (spell, stacks.max(1)) };
                let bar = *bar;
                let mut f = Fields::new();
                fields::aura_words(&mut f, &bar, slot);
                self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                if secs > 0.0 {
                    self.to_player(unit, packets::aura_duration(slot as u8, (secs * 1000.0) as i32));
                }
            }
            Event::Menu { player, menu } => {
                let Some(c) = self.conn_of(player) else { return };
                if self.ui_menu(c, player, menu.as_ref()) {
                    return;
                }
                match menu {
                    Some(m) => {
                        let source = self.guid(m.source).unwrap_or(0);
                        let options: Vec<(u8, &str)> = m.options.iter().map(|o| (o.icon, o.text.as_str())).collect();
                        if let Some(w) = self.world_mut(c) {
                            w.renown_menu = None;
                        }
                        self.send(c, packets::gossip_message(source, menu_text_id(m.kind), &options));
                    }
                    None => self.send(c, packets::gossip_complete()),
                }
            }
            Event::Inventory { player } => {
                if let Some(c) = self.conn_of(player) {
                    self.sync_items(c);
                }
            }
            Event::ItemReceived { player, entry, slot } => {
                if let Some(guid) = self.guid(player) {
                    self.to_player(player, packets::item_push(guid, slot, entry, false));
                }
            }
            Event::Visible { player } => {
                let Some(guid) = self.guid(player) else { return };
                let Some(view) = self.sim.player_view(player) else { return };
                let mut f = Fields::new();
                self.visible_fields(&mut f, &view);
                self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
            }
            Event::Lootable { corpse, player, lootable } => {
                if let Some(guid) = self.guid(corpse) {
                    let mut f = Fields::new();
                    // Only the lootable bit: the client loots a dead creature only when it isn't also
                    // marked "dead" (that bit means feigning death).
                    let flags = if lootable { fields::UNIT_DYNFLAG_LOOTABLE } else { 0 };
                    f.set(idx::UNIT_DYNAMIC_FLAGS, flags);
                    self.to_player(player, packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::Channel { player, spell, ms } if spell == htl_sim::sim::HEARTH_SPELL_ID => {
                // The Hearthstone shows as a real cast does (the swirl and the bar, for everyone),
                // and the hero says they're heading home.
                let Some(guid) = self.guid(player) else { return };
                self.broadcast(&packets::spell_start(guid, spell, ms, Aim::Caster));
                const LINES: [&str; 5] = [
                    "Heading back to the Ancient!",
                    "Hold on, I'm coming home!",
                    "Back to the Ancient, quick!",
                    "Hearthing home: cover me!",
                    "The Ancient calls me back!",
                ];
                let line = LINES[(self.hearths as usize) % LINES.len()];
                self.hearths += 1;
                self.broadcast(&packets::chat(packets::CHAT_SAY, packets::LANG_UNIVERSAL, guid, line));
            }
            Event::Channel { player, spell, ms } => {
                self.to_player(player, packets::channel_start(spell, ms));
                if let Some(guid) = self.guid(player) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_CHANNEL_SPELL, spell);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::ChannelLeft { player, ms } => self.to_player(player, packets::channel_update(ms)),
            Event::ChannelEnd { player } => {
                self.to_player(player, packets::channel_update(0));
                if let Some(guid) = self.guid(player) {
                    let mut f = Fields::new();
                    f.set(idx::UNIT_CHANNEL_SPELL, 0);
                    self.broadcast(&packets::update_object(&[], &[(guid, &f)]));
                }
            }
            Event::Teleport { player, pos, facing } => {
                let Some(guid) = self.guid(player) else { return };
                let Some(c) = self.conn_of(player) else { return };
                let counter = self.world_mut(c).map_or(0, |w| {
                    w.counter += 1;
                    w.counter
                });
                self.send(c, packets::teleport(guid, counter, pos, facing));
                self.broadcast_except(c, &packets::heartbeat(guid, pos, facing));
            }
            Event::Rush { player, from, to, secs } => {
                // The client runs its own hero along a server spline (as 1.12's Charge does) and
                // everyone else sees the same run.
                let Some(guid) = self.guid(player) else { return };
                self.spline_id = self.spline_id.wrapping_add(1);
                let ms = (secs * 1000.0).round().max(1.0) as u32;
                self.broadcast(&packets::monster_move(guid, self.spline_id, &[from, to], ms));
            }
            Event::ZoneStart { id, caster, spell, pos, radius, mark } => {
                let p = self.zone_create(id, caster, spell, pos, radius);
                self.broadcast(&p);
                if let Some(p) = self.mark_create(id, mark, pos, radius) {
                    self.marks.insert(id);
                    self.broadcast(&p);
                }
            }
            Event::ZoneEnd { id } => {
                self.broadcast(&packets::destroy_object(zone_guid(id)));
                if self.marks.remove(&id) {
                    self.broadcast(&packets::destroy_object(mark_guid(id)));
                }
            }
            Event::RunOver(r) => self.run_over(r),
        }
    }
}
