//! Crowd control, by one exact rule each (Battlerite's, with WoW's names): what a stun, a root, a
//! panic does to a hero or a creature, how long, and the two rules that keep it fair: heroes get
//! half the duration creatures do, and after a stun, incapacitate or panic a hero can't be given
//! another of those three for a while (the grace).

use std::collections::VecDeque;

use super::*;

/// An effect on a unit: what, until when, from whom, how strong (a snare's or a weaken's share, a
/// burn's damage a second, a mark's bonus), and what's left of a frozen foe's ice.
#[derive(Debug, Clone, PartialEq)]
pub struct Cc {
    pub kind: String,
    pub until: f32,
    pub since: f32,
    pub from: Id,
    pub amount: f32,
    /// A root that damage breaks once it's been held this long (0: damage doesn't).
    pub break_after: f32,
    /// A petrify's ice, a panic's damage still to take before it ends.
    pub left: f32,
    /// What did it (for the log and the screen).
    pub source: String,
    /// The next tick of a burn (a regen's).
    pub next_tick: f32,
    /// Its look, in place of the kind's (0: the kind's).
    pub aura: u32,
}

/// The three that bring the grace after them.
const HARD: [&str; 3] = ["stun", "incapacitate", "panic"];
/// The ones that keep a unit from acting at all.
const DISABLED: [&str; 4] = ["stun", "incapacitate", "petrify", "panic"];
/// A panicked hero is driven a stride away this often.
const PANIC_STRIDE: f32 = 0.5;
/// How many lines of the crowd-control log a hero keeps.
const LOG_KEEP: usize = 40;

impl Survival {
    /// The effects on a unit now.
    pub fn cc_of(&self, id: Id) -> &[Cc] {
        self.cc.get(&id).map_or(&[], |v| v.as_slice())
    }

    pub fn has_cc(&self, id: Id, kind: &str) -> bool {
        let now = self.clock;
        self.cc_of(id).iter().any(|c| c.kind == kind && c.until > now)
    }

    /// Stunned, incapacitated, frozen or panicked: no acting at all.
    pub fn disabled(&self, id: Id) -> bool {
        let now = self.clock;
        self.cc_of(id).iter().any(|c| c.until > now && DISABLED.contains(&c.kind.as_str()))
    }

    /// Kept from moving (a root, or anything that disables, but a panic, which runs).
    pub fn held_still(&self, id: Id) -> bool {
        let now = self.clock;
        self.cc_of(id).iter().any(|c| c.until > now && matches!(c.kind.as_str(), "stun" | "incapacitate" | "petrify" | "root"))
    }

    /// No abilities: disabled, or silenced.
    pub fn silenced(&self, id: Id) -> bool {
        self.disabled(id) || self.has_cc(id, "silence")
    }

    /// The share of its speed a unit keeps under its snares (the strongest of them).
    pub fn speed_share(&self, id: Id) -> f32 {
        let now = self.clock;
        let worst = self.cc_of(id).iter().filter(|c| c.kind == "snare" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max);
        let haste = self.cc_of(id).iter().filter(|c| c.kind == "haste" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max);
        ((1.0 - worst) * (1.0 + haste)).clamp(0.05, 2.0)
    }

    /// The share a unit's blows and heals keep under a weaken.
    pub fn weakened(&self, id: Id) -> f32 {
        let now = self.clock;
        let worst = self.cc_of(id).iter().filter(|c| c.kind == "weaken" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max);
        (1.0 - worst).clamp(0.0, 1.0)
    }

    /// A line in a hero's crowd-control log (`.es cc log`).
    pub(crate) fn cc_note(&mut self, hero: Id, line: String) {
        if !self.players.contains_key(&hero) {
            return;
        }
        let at = self.clock;
        let log = self.cc_log.entry(hero).or_insert_with(VecDeque::new);
        log.push_back(format!("[{at:.1}] {line}"));
        while log.len() > LOG_KEEP {
            log.pop_front();
        }
    }

    pub fn cc_log_of(&self, hero: Id) -> Vec<String> {
        self.cc_log.get(&hero).map_or(Vec::new(), |l| l.iter().cloned().collect())
    }

    pub(crate) fn unit_name(&self, id: Id) -> String {
        if let Some(p) = self.players.get(&id) {
            return p.name.clone();
        }
        self.mobs.get(&id).map_or_else(|| "something".into(), |m| self.content.mobs[m.def].name.clone())
    }

    /// Give `target` an effect from `from` for `secs` (a creature's duration: a hero gets the
    /// share the rules say). False when it doesn't take (the grace, a dead or inert target).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_cc(&mut self, target: Id, from: Id, kind: &str, secs: f32, amount: f32, break_after: f32, source: &str) -> bool {
        self.apply_cc_look(target, from, kind, secs, amount, break_after, source, 0)
    }

    /// [`Self::apply_cc`] with its own look (`aura`: a spell whose state look shows; 0 the kind's).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn apply_cc_look(&mut self, target: Id, from: Id, kind: &str, secs: f32, amount: f32, break_after: f32, source: &str, aura: u32) -> bool {
        let now = self.clock;
        let hero = self.players.contains_key(&target);
        let alive = self.players.get(&target).is_some_and(|p| p.health > 0) || self.mobs.get(&target).is_some_and(|m| m.health > 0 && !self.content.mobs[m.def].inert());
        if !alive || secs <= 0.0 {
            return false;
        }
        // A boon (Onslaught, holy ground's) helps: the rules that soften what harms don't touch it.
        let boon = crate::content::BOONS.contains(&kind);
        // Nothing reaches someone in Ice Block.
        if self.immune(target) && !boon {
            return false;
        }
        let secs = if hero && !boon { secs * self.content.cc.player_share } else { secs };
        let (by, to) = (self.unit_name(from), self.unit_name(target));
        if kind == "stagger" && self.unstaggerable(target) {
            return false;
        }
        if hero && HARD.contains(&kind) && self.grace.get(&target).is_some_and(|t| *t > now) {
            let line = format!("{source}: {kind} {secs:.1}s from {by} on {to}, stopped by the grace");
            self.cc_note(target, line.clone());
            self.cc_note(from, line);
            return false;
        }
        if kind == "stagger" {
            self.stagger_unit(target, from, secs);
            let line = format!("{source}: stagger {secs:.1}s from {by} on {to}");
            self.cc_note(target, line.clone());
            self.cc_note(from, line);
            return true;
        }
        let left = match kind {
            "petrify" => self.content.cc.petrify_shield,
            "shield" => amount,
            "panic" => self.max_health_of(target) as f32 * self.content.cc.panic_break,
            _ => 0.0,
        };
        let list = self.cc.entry(target).or_default();
        // One of each kind: the longer stands (a stronger snare replaces a weaker).
        if let Some(c) = list.iter_mut().find(|c| c.kind == kind && c.until > now) {
            c.until = c.until.max(now + secs);
            c.amount = c.amount.max(amount);
            c.left = c.left.max(left);
            c.from = from;
            if aura != 0 {
                c.aura = aura;
            }
        } else {
            list.retain(|c| c.kind != kind);
            list.push(Cc { kind: kind.into(), until: now + secs, since: now, from, amount, break_after, left, source: source.into(), next_tick: now + 1.0, aura });
        }
        let renewed = boon && self.cc_of(target).iter().any(|c| c.kind == kind && c.since < now);
        if renewed {
            return true;
        }
        let line = format!("{source}: {kind} {secs:.1}s from {by} on {to}");
        self.cc_note(target, line.clone());
        self.cc_note(from, line);
        self.events.push(Event::Auras { id: target });
        self.cc_began(target, kind, secs, from);
        if kind == "veil" {
            self.veiled(target);
        }
        true
    }

    fn max_health_of(&self, id: Id) -> u32 {
        self.players.get(&id).map(|p| p.max_health).or_else(|| self.mobs.get(&id).map(|m| m.max_health)).unwrap_or(1)
    }

    /// What an effect does as it starts: a disable cuts short what the unit was doing.
    fn cc_began(&mut self, id: Id, kind: &str, secs: f32, from: Id) {
        let disables = DISABLED.contains(&kind);
        if self.players.contains_key(&id) {
            if disables {
                self.interrupt_hero(id);
                if kind != "panic" {
                    let anim = if kind == "petrify" { 0 } else { combat::STUN_ANIM };
                    if anim != 0 {
                        let at = self.unit_pos(from).unwrap_or(self.players[&id].pos);
                        self.events.push(Event::React { id, anim, from: at, lean: 0.2, hold: secs, fit: 0.0 });
                    }
                }
            }
            if kind == "panic" {
                self.players.get_mut(&id).unwrap().panic_stride = self.clock;
            }
            self.update_rooted(id);
            self.update_speed(id);
            self.events.push(Event::Effects { player: id });
        } else if self.mobs.contains_key(&id) {
            if matches!(kind, "stun" | "incapacitate" | "petrify") {
                self.stagger_mob(id, secs, false);
            }
            if kind == "panic" {
                self.stagger_mob(id, 0.05, false);
                if let Some(m) = self.mobs.get_mut(&id) {
                    m.panic_stride = 0.0;
                }
            }
            // Held, or slowed mid-stride: it stops, and walks on at its new pace.
            if kind == "root" || kind == "snare" {
                self.halt_mob(id);
            }
        }
    }

    /// Where a unit is.
    pub fn unit_pos(&self, id: Id) -> Option<Vec3> {
        self.players.get(&id).map(|p| p.pos).or_else(|| self.mobs.get(&id).map(|m| m.pos))
    }

    /// A hero's hands are emptied: an aim, a charge, a channel, a swing in hand is lost.
    pub(crate) fn interrupt_hero(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        if matches!(p.action, Action::Dead { .. }) {
            return;
        }
        let busy = !matches!(p.action, Action::Ready | Action::Stagger { .. });
        if busy {
            self.end_ability_action(id);
            self.set_action(id, Action::Ready);
        }
        if let Some(p) = self.players.get_mut(&id) {
            p.buffered = None;
        }
    }

    /// A stagger: whatever's in hand is lost for a moment.
    fn stagger_unit(&mut self, id: Id, from: Id, secs: f32) {
        let now = self.clock;
        if self.players.contains_key(&id) {
            self.end_ability_action(id);
            self.set_action(id, Action::Stagger { until: now + secs });
            self.events.push(Event::Emote { id, emote: mobs::STUN_EMOTE });
            let at = self.unit_pos(from).unwrap_or(self.players[&id].pos);
            self.events.push(Event::React { id, anim: combat::STUN_ANIM, from: at, lean: 0.3, hold: secs, fit: 0.0 });
        } else if self.mobs.contains_key(&id) {
            self.stagger_mob(id, secs, false);
        }
    }

    /// A hero held still or let go: their client is told (a root).
    pub(crate) fn update_rooted(&mut self, id: Id) {
        let now = self.clock;
        let still = self.held_still(id)
            || self.players.get(&id).is_some_and(|p| p.hold_until > now && p.health > 0)
            || self.players.get(&id).is_some_and(|p| matches!(&p.action, Action::Stance { ability, .. } if self.content.abilities[*ability].counter.as_ref().is_some_and(|c| c.immune)));
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.rooted != still {
            p.rooted = still;
            self.events.push(Event::Rooted { player: id, on: still });
        }
    }

    /// Damage reaching a unit: what its ice takes first (the rest comes through), and what it does
    /// to the effects on it (an incapacitate ends, a root held long enough breaks, a panic ends
    /// after enough).
    pub(crate) fn cc_on_damage(&mut self, id: Id, damage: u32) -> u32 {
        let now = self.clock;
        if self.immune(id) {
            return 0;
        }
        let Some(list) = self.cc.get_mut(&id) else { return damage };
        let mut through = damage as f32;
        let mut ended = Vec::new();
        // A shield soaks it first.
        for c in list.iter_mut().filter(|c| c.until > now && c.kind == "shield") {
            let took = through.min(c.left);
            c.left -= took;
            through -= took;
            if c.left <= 0.0 {
                c.until = now;
                ended.push(c.kind.clone());
            }
        }
        for c in list.iter_mut().filter(|c| c.until > now) {
            match c.kind.as_str() {
                "petrify" => {
                    let took = through.min(c.left);
                    c.left -= took;
                    through -= took;
                    if c.left <= 0.0 {
                        c.until = now;
                        ended.push(c.kind.clone());
                    }
                }
                "incapacitate" if damage > 0 => {
                    c.until = now;
                    ended.push(c.kind.clone());
                }
                "root" if damage > 0 && c.break_after > 0.0 && now - c.since >= c.break_after => {
                    c.until = now;
                    ended.push(c.kind.clone());
                }
                "panic" if damage > 0 => {
                    c.left -= damage as f32;
                    if c.left <= 0.0 {
                        c.until = now;
                        ended.push(c.kind.clone());
                    }
                }
                _ => {}
            }
        }
        if !ended.is_empty() {
            for k in &ended {
                let line = format!("{k} on {} broken by damage", self.unit_name(id));
                self.cc_note(id, line);
            }
            self.cc_ended(id, &ended);
        }
        through.round() as u32
    }

    /// Effects run out, burns tick, the panicked run.
    pub(crate) fn step_cc(&mut self) {
        let now = self.clock;
        let ids: Vec<Id> = self.cc.keys().copied().collect();
        for id in ids {
            // Burning: a tick a second (it never lands the killing blow on a hero).
            let ticks: Vec<(Id, f32, String)> = self
                .cc
                .get_mut(&id)
                .map(|l| {
                    l.iter_mut()
                        .filter(|c| c.kind == "burn" && c.next_tick <= now && c.since < c.until)
                        .map(|c| {
                            c.next_tick += 1.0;
                            (c.from, c.amount, c.source.clone())
                        })
                        .collect()
                })
                .unwrap_or_default();
            for (from, amount, _) in ticks {
                self.dot_tick(id, from, amount);
            }
            // Mending: its share of their health a second.
            let mends: Vec<f32> = self
                .cc
                .get_mut(&id)
                .map(|l| {
                    l.iter_mut()
                        .filter(|c| c.kind == "regen" && c.next_tick <= now && c.since < c.until)
                        .map(|c| {
                            c.next_tick += 1.0;
                            c.amount
                        })
                        .collect()
                })
                .unwrap_or_default();
            for share in mends {
                let amount = self.players.get(&id).map_or(0, |p| (p.max_health as f32 * share).round() as u32);
                self.heal_hero(id, amount);
            }
            let Some(list) = self.cc.get_mut(&id) else { continue };
            let gone: Vec<String> = list.iter().filter(|c| c.until <= now).map(|c| c.kind.clone()).collect();
            list.retain(|c| c.until > now);
            if list.is_empty() {
                self.cc.remove(&id);
            }
            if !gone.is_empty() {
                self.cc_ended(id, &gone);
            }
            self.panic_run(id);
        }
        self.grace.retain(|_, t| *t > now);
    }

    fn cc_ended(&mut self, id: Id, kinds: &[String]) {
        let now = self.clock;
        if let Some(list) = self.cc.get_mut(&id) {
            list.retain(|c| c.until > now);
        }
        self.events.push(Event::Auras { id });
        if self.players.contains_key(&id) {
            // The grace, once the last of the three is over.
            if kinds.iter().any(|k| HARD.contains(&k.as_str())) && !self.cc_of(id).iter().any(|c| HARD.contains(&c.kind.as_str())) {
                let until = now + self.content.cc.grace;
                self.grace.insert(id, until);
                self.cc_note(id, format!("grace for {:.1}s", self.content.cc.grace));
            }
            self.update_rooted(id);
            self.update_speed(id);
            self.events.push(Event::Effects { player: id });
        } else if let Some(m) = self.mobs.get_mut(&id) {
            // Free again: a creature stunned takes up its fight at once.
            if kinds.iter().any(|k| matches!(k.as_str(), "stun" | "incapacitate" | "petrify" | "panic")) && m.stagger_until > now {
                m.stagger_until = now;
                m.ready_at = m.ready_at.min(now + 0.4);
            }
        }
    }

    /// A burn's tick on a unit.
    fn dot_tick(&mut self, id: Id, from: Id, amount: f32) {
        // A creature's burn on a hero, as hard as its blows.
        let amount = if self.mobs.contains_key(&from) && self.players.contains_key(&id) { amount * self.foe_blow_share(from) } else { amount };
        let damage = amount.round().max(1.0) as u32;
        if let Some(p) = self.players.get(&id) {
            if p.health == 0 {
                return;
            }
            let damage = damage.min(p.health.saturating_sub(1));
            if damage > 0 {
                self.events.push(Event::Swing { attacker: from, victim: Some(id), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
                self.hurt_player(id, damage);
            }
        } else if self.mobs.get(&id).is_some_and(|m| m.health > 0) {
            self.events.push(Event::Swing { attacker: from, victim: Some(id), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
            self.hurt_mob(id, from, damage);
        }
    }

    /// A panicked unit runs from what scared it, out of control.
    fn panic_run(&mut self, id: Id) {
        let now = self.clock;
        let Some(c) = self.cc_of(id).iter().find(|c| c.kind == "panic" && c.until > now).cloned() else { return };
        let from = self.unit_pos(c.from).unwrap_or([0.0; 3]);
        if let Some(p) = self.players.get(&id) {
            if now - p.panic_stride < PANIC_STRIDE {
                return;
            }
            let dir = facing(from, p.pos) + (self.rng.gen::<f32>() - 0.5) * 0.8;
            self.players.get_mut(&id).unwrap().panic_stride = now;
            let speed = self.content.cc.panic_speed * 2.2;
            self.events.push(Event::Hop { player: id, dir, speed, lift: 1.2, roll: false, style: "" });
        } else if let Some(m) = self.mobs.get(&id) {
            if now - m.panic_stride < PANIC_STRIDE * 2.0 {
                return;
            }
            let pos = m.pos;
            let def = m.def;
            let dir = facing(from, pos) + (self.rng.gen::<f32>() - 0.5) * 0.8;
            let to = self.ground_near(pos[0] + dir.cos() * 4.0, pos[1] + dir.sin() * 4.0, pos[2]);
            let speed = self.content.mobs[def].run_speed * 0.8;
            self.mobs.get_mut(&id).unwrap().panic_stride = now;
            if !self.land.blocked_near(to[0], to[1], to[2]) && self.land.walk_clear(pos, to) {
                self.flee_to(id, to, speed);
            }
        }
    }

    /// Take a unit's effects off (a blink out of roots, Tremor Totem's pulse on a panic).
    pub(crate) fn cleanse(&mut self, id: Id, kinds: &[&str]) {
        let now = self.clock;
        let Some(list) = self.cc.get_mut(&id) else { return };
        let mut gone = Vec::new();
        for c in list.iter_mut() {
            if kinds.contains(&c.kind.as_str()) && c.until > now {
                c.until = now;
                gone.push(c.kind.clone());
            }
        }
        if !gone.is_empty() {
            self.cc_ended(id, &gone);
        }
    }

    /// A mark's bonus on a unit, used up by the hit that finds it.
    pub(crate) fn take_mark(&mut self, id: Id) -> f32 {
        let now = self.clock;
        let Some(list) = self.cc.get_mut(&id) else { return 0.0 };
        let Some(k) = list.iter().position(|c| c.kind == "mark" && c.until > now) else { return 0.0 };
        let c = list.remove(k);
        self.events.push(Event::Auras { id });
        c.amount
    }

    /// A unit nothing reaches (in Ice Block).
    pub fn immune(&self, id: Id) -> bool {
        self.stances.get(&id).is_some_and(|s| s.until > self.clock && self.content.abilities[s.ability].counter.as_ref().is_some_and(|c| c.immune))
    }

    /// Every effect gone (a fall, a rest, `.es heal`).
    pub(crate) fn clear_cc(&mut self, id: Id) {
        if self.cc.remove(&id).is_some() {
            self.events.push(Event::Auras { id });
        }
        self.grace.remove(&id);
        if self.players.contains_key(&id) {
            self.update_rooted(id);
            self.update_speed(id);
        }
    }

    /// What shows on a unit for its effects and stance (spell ids whose state looks the clients
    /// play), for the server's aura slots.
    pub fn auras_of(&self, id: Id) -> Vec<u32> {
        let now = self.clock;
        let mut out: Vec<u32> = self
            .cc_of(id)
            .iter()
            .filter(|c| c.until > now)
            .filter_map(|c| if c.aura != 0 { Some(c.aura) } else { self.content.cc.auras.get(&c.kind).copied() })
            .collect();
        if let Some(s) = self.stances.get(&id).filter(|s| s.until > now) {
            let a = self.content.abilities[s.ability].aura;
            if a != 0 {
                out.push(a);
            }
        }
        if let Some(p) = self.players.get(&id) {
            if let Action::Channel { ability, .. } = &p.action {
                let a = self.content.abilities[*ability].aura;
                if a != 0 {
                    out.push(a);
                }
            }
        }
        out.sort_unstable();
        out.dedup();
        out
    }
}
