//! Triggers: "when this happens (and this holds), do that". Upgrades carry them for their own
//! spell, talents for any spell (or the spells they name), or for swings, blocks, blows and
//! waves. They are the upgrades and talents that change how a build plays, not only its numbers.
//!
//! Each trigger is a [`TriggerDef`] (see `kit.rs`); this module finds the ones that apply when
//! something happens and does what they say. The timed effects they give are [`TBuff`]s:
//! stacking buffs with a key, worth damage, haste, less damage taken, speed, crit or leech per
//! stack, read by the same functions that read the game's own buffs.

use rand::Rng;

use super::enemies::Via;
use super::spells::Vuln;
use super::*;
use crate::kit::{BuffDef, Cond, On, TriggerDef};

/// A trigger's timed effect on a player.
#[derive(Debug, Clone)]
pub(super) struct TBuff {
    pub key: String,
    pub until: f32,
    pub stacks: u32,
    pub def: BuffDef,
}

/// How far a "blast around you" counts as near: allies for team heals and team buffs.
const TEAM_YARDS: f32 = 15.0;
/// At a bridge post: within this many yards of one.
const POST_YARDS: f32 = 15.0;
/// Whether a trigger is a big enough moment to call out by default.
fn big_moment(t: &TriggerDef) -> bool {
    t.reset
        || t.resets.is_some()
        || t.echo
        || t.cast.is_some()
        || (t.damage > 0.0 && t.radius > 0.0)
        || (t.stun > 0.0 && t.radius > 0.0)
        || (t.taunt > 0.0 && t.radius > 0.0)
        || t.shield >= 0.1
        || t.heal >= 0.08
        || t.heal_team > 0.0
        || t.heal_ancient >= 0.01
        || t.buff.as_ref().is_some_and(|b| !b.next.is_empty() || b.team || (b.stacks <= 1 && b.secs >= 6.0))
}

/// Triggers set off by what a trigger did (its blast killing something) go one level deep.
const MAX_DEPTH: u8 = 2;

/// One trigger a player holds, ready to check: where it came from and how strong.
struct Held {
    def: TriggerDef,
    /// Times over: an upgrade taken twice, a talent's rank.
    scale: f32,
    /// The spell (kit index) an upgrade's trigger belongs to.
    own: Option<usize>,
    /// For its internal cooldown: (0 upgrade / 1 talent, spell or talent, perk, trigger).
    src: (u8, u16, u16, u8),
    /// Its card's name and icon, to call it out.
    name: String,
    icon: u32,
}

impl Sim {
    /// Every trigger a player holds.
    fn held_triggers(&self, id: EntityId) -> Vec<Held> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let kit = self.content.kit(p.class);
        let mut out = Vec::new();
        for o in &p.spells {
            let def = &kit.spells[o.spell];
            let mut seen: Vec<usize> = Vec::new();
            for &pk in &o.perks {
                if seen.contains(&pk) {
                    continue;
                }
                seen.push(pk);
                let count = o.perks.iter().filter(|&&x| x == pk).count() as f32;
                let Some(perk) = def.perks.get(pk) else { continue };
                for (ti, t) in perk.triggers.iter().enumerate() {
                    out.push(Held {
                        def: t.clone(),
                        scale: count,
                        own: Some(o.spell),
                        src: (0, o.spell as u16, pk as u16, ti as u8),
                        name: perk.name.clone(),
                        icon: def.icon,
                    });
                }
            }
        }
        for (ti, t) in kit.talents.iter().enumerate() {
            let rank = p.talents.get(ti).copied().unwrap_or(0);
            if rank == 0 {
                continue;
            }
            for (k, tr) in t.triggers.iter().enumerate() {
                out.push(Held { def: tr.clone(), scale: rank as f32, own: None, src: (1, ti as u16, 0, k as u8), name: t.name.clone(), icon: t.icon });
            }
        }
        out
    }

    /// Something happened to player `id`: set off what applies. `spell` is the kit index of
    /// the spell involved (if any), `target` the enemy involved (if any), `blow` the share of
    /// max health a blow took (for `Hurt`).
    pub(super) fn fire(&mut self, id: EntityId, on: On, spell: Option<usize>, target: Option<EntityId>, blow: f32) {
        let depth = match self.entities.get(&id).and_then(|e| e.player()) {
            Some(p) => p.trig_depth,
            None => return,
        };
        if depth >= MAX_DEPTH || !self.entities[&id].alive() {
            return;
        }
        let held = self.held_triggers(id);
        if held.is_empty() {
            return;
        }
        let class = self.entities[&id].player().unwrap().class;
        for h in held {
            let t = &h.def;
            if t.on != on {
                continue;
            }
            // Whose spell it is about.
            if on.of_spell() {
                match (h.own, &t.spells) {
                    (Some(own), _) => {
                        if spell != Some(own) {
                            continue;
                        }
                    }
                    (None, Some(f)) => {
                        let Some(s) = spell else { continue };
                        if !f.matches(&self.content.kit(class).spells[s]) {
                            continue;
                        }
                    }
                    (None, None) => {
                        if spell.is_none() && on != On::Kill {
                            continue;
                        }
                    }
                }
            }
            let now = self.clock;
            let ready = self.entities[&id].player().unwrap().trig_ready.iter().find(|(k, _)| *k == h.src).map_or(0.0, |(_, at)| *at);
            if ready > now {
                continue;
            }
            if !self.trigger_holds(id, t, target, blow) {
                continue;
            }
            if t.chance < 1.0 && self.rng.gen::<f32>() >= t.chance * h.scale.min(3.0) {
                continue;
            }
            if t.icd > 0.0 {
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    match p.trig_ready.iter_mut().find(|(k, _)| *k == h.src) {
                        Some(x) => x.1 = now + t.icd,
                        None => p.trig_ready.push((h.src, now + t.icd)),
                    }
                }
            }
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.trig_depth += 1;
            }
            self.do_trigger(id, t, h.scale, h.own.or(spell), target);
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.trig_depth = p.trig_depth.saturating_sub(1);
            }
            if t.shout.unwrap_or_else(|| big_moment(t)) {
                self.shout(id, h.src, &h.name, h.icon);
            }
            if !self.entities.get(&id).is_some_and(|e| e.alive()) {
                return;
            }
        }
    }

    /// Call a trigger out on screen, at most once every 1.5 sec each.
    fn shout(&mut self, id: EntityId, src: (u8, u16, u16, u8), name: &str, icon: u32) {
        let now = self.clock;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        match p.shout_ready.iter_mut().find(|(k, _)| *k == src) {
            Some(x) if x.1 > now => return,
            Some(x) => x.1 = now + 1.5,
            None => p.shout_ready.push((src, now + 1.5)),
        }
        self.events.push(Event::Proc { player: id, name: name.to_string(), icon });
    }

    /// Whether a trigger's conditions hold now.
    fn trigger_holds(&self, id: EntityId, t: &TriggerDef, target: Option<EntityId>, blow: f32) -> bool {
        let me = &self.entities[&id];
        let mine = me.health as f32 / me.max_health.max(1) as f32;
        if t.self_below > 0.0 && mine >= t.self_below {
            return false;
        }
        if t.self_above > 0.0 && mine < t.self_above {
            return false;
        }
        if t.blow_above > 0.0 && blow < t.blow_above {
            return false;
        }
        if t.rage_at_least > 0.0 && me.player().map_or(0.0, |p| p.rage) < t.rage_at_least {
            return false;
        }
        if t.enemies > 0 && (self.enemies_near(me.pos, 8.0).len() as u32) < t.enemies {
            return false;
        }
        if t.near_ancient > 0.0 && !self.near_ancient(id, t.near_ancient) {
            return false;
        }
        if t.at_post && !self.at_post(id) {
            return false;
        }
        let needs_target = t.target_below > 0.0 || t.target_above > 0.0 || !t.target.is_empty();
        if needs_target {
            let Some(v) = target.and_then(|v| self.entities.get(&v)) else { return false };
            let share = v.health as f32 / v.max_health.max(1) as f32;
            if t.target_below > 0.0 && share >= t.target_below {
                return false;
            }
            if t.target_above > 0.0 && share < t.target_above {
                return false;
            }
            for c in &t.target {
                if !self.enemy_is(target.unwrap(), *c) {
                    return false;
                }
            }
        }
        true
    }

    /// Whether an enemy is in a state.
    pub(super) fn enemy_is(&self, v: EntityId, c: Cond) -> bool {
        let Some(e) = self.entities.get(&v) else { return false };
        let Some(en) = e.enemy() else { return false };
        let now = self.clock;
        match c {
            Cond::Bleeding => en.dots.iter().any(|d| d.school == 0),
            Cond::Stunned => en.aff.stun_until > now,
            Cond::Slowed => en.slow_until > now,
            Cond::Sundered => en.aff.vulns[Vuln::Sunder as usize].until > now && en.aff.vulns[Vuln::Sunder as usize].n > 0,
            Cond::Frozen => en.aff.frozen_until > now,
            Cond::Elite => en.elite || en.kind.boss,
            Cond::Full => e.health as f32 >= e.max_health as f32 * 0.8,
            Cond::Burning => en.dots.iter().any(|d| d.school == 2 || d.school == 1),
            Cond::Afflicted => en.dots.iter().any(|d| d.school == 5),
            Cond::Poisoned => en.dots.iter().any(|d| d.school == 3),
        }
    }

    /// Within `yards` of the Ancient.
    pub(super) fn near_ancient(&self, id: EntityId, yards: f32) -> bool {
        let (Some(me), Some(hub)) = (self.entities.get(&id), self.entities.get(&self.hub)) else { return false };
        dist2d(me.pos, hub.pos) <= yards
    }

    /// Standing at one of the bridge posts.
    pub(super) fn at_post(&self, id: EntityId) -> bool {
        let Some(me) = self.entities.get(&id) else { return false };
        self.arena.posts.iter().any(|p| dist2d(p.pos, me.pos) <= POST_YARDS)
    }

    /// Do what a trigger says.
    fn do_trigger(&mut self, id: EntityId, t: &TriggerDef, scale: f32, spell: Option<usize>, target: Option<EntityId>) {
        let pos = self.entities[&id].pos;
        let max = self.entities[&id].max_health as f32;
        if t.rage > 0.0 {
            self.gain_rage(id, t.rage * scale);
        }
        if t.heal > 0.0 {
            self.heal(id, id, if t.visual != 0 { t.visual } else { 441 }, (max * t.heal * scale).round().max(1.0) as u32, false);
        }
        if t.shield > 0.0 {
            self.give_shield(id, id, max * t.shield * scale, 10.0, 0.0, 0.0, 17);
        }
        if t.cdr > 0.0 {
            self.cut_others(id, spell, t.cdr * scale);
        }
        if t.reset {
            self.reset_spell(id, t, spell);
        }
        if let Some(f) = &t.resets {
            self.reset_matching(id, f);
        }
        // Blasts and statuses.
        let hits_statuses = t.stun > 0.0
            || t.slow > 0.0
            || t.bleed > 0.0
            || t.sunder > 0.0
            || t.taunt > 0.0
            || t.freeze > 0.0
            || t.burn > 0.0
            || t.vuln_fire > 0.0
            || t.vuln_frost > 0.0
            || t.vuln_shadow > 0.0;
        if t.damage > 0.0 || hits_statuses {
            // (A kill's blast goes off where the enemy fell.)
            let fell_at = target.and_then(|v| self.entities.get(&v)).map(|e| e.pos);
            let target = target.filter(|v| self.entities.get(v).is_some_and(|e| e.alive() && e.is_enemy()));
            let center = if t.at_self { pos } else { target.map(|v| self.entities[&v].pos).or(fell_at).unwrap_or(pos) };
            let victims: Vec<EntityId> = if t.radius > 0.0 {
                self.enemies_near(center, t.radius).into_iter().map(|(v, _)| v).collect()
            } else {
                target.into_iter().collect()
            };
            if t.visual != 0 {
                let go_at = if t.radius > 0.0 { SpellTarget::Point(center) } else { target.map_or(SpellTarget::Unit(id), SpellTarget::Unit) };
                self.events.push(Event::SpellGo { caster: id, spell: t.visual, target: go_at, hits: victims.clone() });
            }
            let dmg = self.player_hit(id) * t.damage * scale * self.buff_damage(id, Some(t.school));
            let hit = self.player_hit(id);
            for v in victims {
                if !self.entities.get(&v).is_some_and(|e| e.alive()) {
                    continue;
                }
                if dmg >= 0.5 {
                    let visual = if t.visual != 0 { t.visual } else { 845 };
                    self.deal(id, v, dmg.round().max(1.0) as u32, false, Via::Spell { spell: visual, school: t.school, periodic: false });
                }
                if !self.entities.get(&v).is_some_and(|e| e.alive()) {
                    continue;
                }
                if t.stun > 0.0 {
                    self.stun_enemy(v, t.stun);
                }
                if t.slow > 0.0 {
                    self.slow_enemy(v, t.slow.min(0.9), if t.slow_secs > 0.0 { t.slow_secs } else { 4.0 });
                }
                if t.bleed > 0.0 {
                    self.add_dot(id, v, 12721, 0, hit * t.bleed * scale, 6.0, true);
                }
                if t.sunder > 0.0 {
                    self.add_vuln(v, Vuln::Sunder, (t.sunder * scale).round().max(1.0) as u8);
                }
                if t.taunt > 0.0 {
                    self.taunt_enemy(v, id, t.taunt);
                }
                if t.freeze > 0.0 {
                    self.freeze_enemy(v, t.freeze);
                }
                if t.burn > 0.0 {
                    self.add_dot(id, v, 2120, 2, hit * t.burn * scale, 6.0, true);
                }
                for (n, kind) in [(t.vuln_fire, Vuln::Fire), (t.vuln_frost, Vuln::Frost), (t.vuln_shadow, Vuln::Shadow)] {
                    if n > 0.0 {
                        self.add_vuln(v, kind, (n * scale).round().max(1.0) as u8);
                    }
                }
                self.dirty_debuffs(v);
            }
        }
        if t.heal_ancient > 0.0 {
            let hub = self.hub;
            if let Some(h) = self.entities.get(&hub) {
                let amount = (h.max_health as f32 * t.heal_ancient * scale).round().max(1.0) as u32;
                self.heal(id, hub, 25300, amount, false);
            }
        }
        if t.heal_team > 0.0 {
            for a in self.allies_near(pos, TEAM_YARDS) {
                if a == id || !self.entities[&a].is_player() {
                    continue;
                }
                let amount = (self.entities[&a].max_health as f32 * t.heal_team * scale).round().max(1.0) as u32;
                self.heal(id, a, 25300, amount, false);
            }
        }
        if let Some(b) = &t.buff {
            let mut b = b.clone();
            for v in [&mut b.damage, &mut b.haste, &mut b.taken, &mut b.speed, &mut b.crit, &mut b.leech] {
                *v *= scale;
            }
            let team = b.team;
            self.give_tbuff(id, &b);
            if team {
                for a in self.allies_near(pos, TEAM_YARDS) {
                    if a != id && self.entities[&a].is_player() {
                        self.give_tbuff(a, &b);
                    }
                }
            }
        }
        if t.echo {
            if let Some(s) = spell {
                self.echo_spell(id, s);
            }
        }
        if let Some(key) = &t.cast {
            self.free_cast(id, key);
        }
    }

    /// Finish the cooldowns of the held spells a filter matches.
    fn reset_matching(&mut self, id: EntityId, f: &crate::kit::Filter) {
        let Some(class) = self.entities.get(&id).and_then(|e| e.player()).map(|p| p.class) else { return };
        let kit_spells = self.content.kit(class).spells.clone();
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        for o in p.spells.iter_mut() {
            if f.matches(&kit_spells[o.spell]) {
                o.cooldown = 0.0;
            }
        }
        self.events.push(Event::Bar { player: id });
    }

    /// Seconds off every held spell's cooldown but `except`'s.
    fn cut_others(&mut self, id: EntityId, except: Option<usize>, secs: f32) {
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        for o in p.spells.iter_mut() {
            if Some(o.spell) == except {
                continue;
            }
            o.cooldown = (o.cooldown - secs).max(0.0);
        }
        self.events.push(Event::Bar { player: id });
    }

    /// Finish a cooldown: an upgrade's own spell, a talent's named spells, else the spell that
    /// set it off.
    fn reset_spell(&mut self, id: EntityId, t: &TriggerDef, spell: Option<usize>) {
        let class = match self.entities.get(&id).and_then(|e| e.player()) {
            Some(p) => p.class,
            None => return,
        };
        let filter = t.spells.clone();
        let kit_spells = self.content.kit(class).spells.clone();
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        for o in p.spells.iter_mut() {
            let hit = match (&filter, spell) {
                (Some(f), _) => f.matches(&kit_spells[o.spell]),
                (None, Some(s)) => o.spell == s,
                (None, None) => false,
            };
            if hit {
                o.cooldown = 0.0;
            }
        }
        self.events.push(Event::Bar { player: id });
    }

    /// Cast a held spell again for free (an upgrade's echo).
    fn echo_spell(&mut self, id: EntityId, spell: usize) {
        let found = {
            let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return };
            let kit = self.content.kit(p.class);
            p.spells.iter().position(|o| o.spell == spell).map(|i| (i, kit.spells[spell].clone(), p.spells[i].nums.clone()))
        };
        if let Some((i, def, nums)) = found {
            let _ = self.cast_again(id, &def, &nums, i);
        }
    }

    // --- trigger buffs ---------------------------------------------------------------------------

    /// Give (or refresh and stack) a trigger's buff.
    pub(super) fn give_tbuff(&mut self, id: EntityId, b: &BuffDef) {
        let until = self.clock + b.secs;
        let speed = b.speed != 0.0;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        match p.tbuffs.iter_mut().find(|x| x.key == b.key) {
            Some(x) => {
                x.until = until;
                x.stacks = (x.stacks + 1).min(b.stacks.max(1));
                x.def = b.clone();
            }
            None => p.tbuffs.push(TBuff { key: b.key.clone(), until, stacks: 1, def: b.clone() }),
        }
        if speed {
            self.update_speed(id);
        }
        if b.icon != 0 {
            self.refresh_auras(id);
        }
        // An empowered cast is ready at once: its key lights up (a cue only lights it).
        if !b.next.is_empty() && b.hold {
            self.events.push(Event::Bar { player: id });
        } else if !b.next.is_empty() {
            let class = self.entities[&id].player().map(|p| p.class);
            if let Some(class) = class {
                let kit_spells = self.content.kit(class).spells.clone();
                if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                    for o in p.spells.iter_mut() {
                        if b.next.contains(&kit_spells[o.spell].key) {
                            o.cooldown = 0.0;
                        }
                    }
                }
            }
            self.events.push(Event::Bar { player: id });
        }
    }

    /// The empowerment waiting for spell `key`, if any: its buff (one stack's worth).
    pub(super) fn empower_for(&self, id: EntityId, key: &str) -> Option<BuffDef> {
        let p = self.entities.get(&id)?.player()?;
        let now = self.clock;
        p.tbuffs.iter().find(|b| b.until > now && b.def.next.iter().any(|k| k == key)).map(|b| b.def.clone())
    }

    /// Use up one stack of the empowerment waiting for spell `key`.
    pub(super) fn spend_empower(&mut self, id: EntityId, key: &str) {
        let now = self.clock;
        let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) else { return };
        if let Some(i) = p.tbuffs.iter().position(|b| b.until > now && b.def.next.iter().any(|k| k == key)) {
            if p.tbuffs[i].stacks > 1 {
                p.tbuffs[i].stacks -= 1;
            } else {
                p.tbuffs.remove(i);
            }
        }
        self.refresh_auras(id);
        self.events.push(Event::Bar { player: id });
    }

    /// Whether spell `key`'s next cast is empowered (its key lights up).
    pub(super) fn is_lit(&self, id: EntityId, key: &str) -> bool {
        self.empower_for(id, key).is_some()
    }

    /// The sum over a player's trigger buffs of one number (per stack).
    pub(super) fn tbuff(&self, id: EntityId, pick: fn(&BuffDef) -> f32) -> f32 {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return 0.0 };
        let now = self.clock;
        p.tbuffs.iter().filter(|b| b.until > now).map(|b| pick(&b.def) * b.stacks as f32).sum()
    }

    /// Drop trigger buffs that have run out.
    pub(super) fn step_tbuffs(&mut self) {
        let now = self.clock;
        for id in self.player_ids() {
            let (expired, speed, icon, lit) = {
                let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { continue };
                let gone: Vec<&TBuff> = p.tbuffs.iter().filter(|b| b.until <= now).collect();
                (
                    !gone.is_empty(),
                    gone.iter().any(|b| b.def.speed != 0.0),
                    gone.iter().any(|b| b.def.icon != 0),
                    gone.iter().any(|b| !b.def.next.is_empty()),
                )
            };
            if !expired {
                continue;
            }
            if let Some(p) = self.entities.get_mut(&id).and_then(|e| e.player_mut()) {
                p.tbuffs.retain(|b| b.until > now);
            }
            if speed {
                self.update_speed(id);
            }
            if icon {
                self.refresh_auras(id);
            }
            if lit {
                self.events.push(Event::Bar { player: id });
            }
        }
    }

    /// The trigger buffs to show on the buff bar: (icon, stacks).
    pub(super) fn tbuff_auras(&self, id: EntityId) -> Vec<(u32, u8)> {
        let Some(p) = self.entities.get(&id).and_then(|e| e.player()) else { return Vec::new() };
        let now = self.clock;
        p.tbuffs.iter().filter(|b| b.until > now && b.def.icon != 0).map(|b| (b.def.icon, b.stacks.min(255) as u8)).collect()
    }

    /// The spell (kit index) behind a hit the player landed, from what the client was shown.
    pub(super) fn spell_of_visual(&self, id: EntityId, visual: u32) -> Option<usize> {
        let p = self.entities.get(&id)?.player()?;
        let kit = self.content.kit(p.class);
        p.spells.iter().find(|o| kit.spells[o.spell].visual() == visual).map(|o| o.spell)
    }
}
