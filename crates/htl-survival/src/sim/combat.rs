//! A hero's actions: committed swings, the shield, the dodge, the bow, eating; stamina; blows that
//! land on heroes; arrows and fireballs in flight.

use std::collections::BTreeMap;
use std::f32::consts::{FRAC_PI_2, PI};

use super::*;
use crate::land::clear_line;

/// Where a blow lands, re-checked when it resolves: a hero who left it by then (as they saw it,
/// a round trip after the server) is missed.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Area {
    /// In reach and in front of a creature's committed facing.
    Arc { pos: Vec3, facing: f32, range: f32, arc: f32 },
    /// In a blast.
    Circle { at: Vec3, radius: f32 },
    /// In a strip from `pos` along `facing`, `len` long and `half` either side.
    Line { pos: Vec3, facing: f32, len: f32, half: f32 },
}

/// Whether `p` lies in a strip from `pos` along `facing`, `len` long and `half` either side.
pub(crate) fn in_strip(pos: Vec3, facing: f32, len: f32, half: f32, p: Vec3) -> bool {
    let (dx, dy) = (p[0] - pos[0], p[1] - pos[1]);
    let (c, s) = (facing.cos(), facing.sin());
    let along = dx * c + dy * s;
    let side = -dx * s + dy * c;
    (-0.5..=len).contains(&along) && side.abs() <= half && (p[2] - pos[2]).abs() <= 4.0
}

/// What a hero's blow does past the blow itself, once it's resolved.
#[derive(Debug, Clone)]
pub(crate) enum Follow {
    None,
    /// A hero's swing: a stagger if it lands on a wind-up, a throw if heavy.
    Swing { stagger: f32, heavy: bool, winding: bool },
    /// An ability's blow: its effects, the drain, Valor, if it lands.
    Ability { ability: usize, share: f32 },
}

/// A blow on a hero waiting out their round trip, so a dodge or a guard they pressed as they saw it
/// coming still counts.
pub(crate) struct PendingStrike {
    pub attacker: Id,
    pub victim: Id,
    pub damage: u32,
    pub from: Vec3,
    pub blockable: bool,
    pub heavy: bool,
    pub parryable: bool,
    pub style: Blow,
    pub at: f32,
    pub area: Option<Area>,
    pub follow: Follow,
    pub how: HitKind,
}

/// A creature's missile in flight to where its target stood: whoever is in the blast when it
/// lands is struck.
pub(crate) struct Missile {
    pub owner: Id,
    pub from: Vec3,
    pub at: Vec3,
    pub radius: f32,
    pub damage: u32,
    pub blockable: bool,
    pub heavy: bool,
    pub land_at: f32,
    pub mark: Id,
    /// The spell whose ground look plays where it lands (0: none).
    pub look: u32,
}

impl Survival {
    /// What a hero pressed or let go.
    pub fn input(&mut self, id: Id, input: Input) {
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        match input {
            Input::Guard => {
                // A fresh press: what a parry is timed from.
                if !p.guard_held {
                    p.prev_guard_at = p.guard_at;
                    p.guard_at = now;
                }
                p.guard_held = true;
            }
            Input::GuardUp => p.guard_held = false,
            Input::Attack | Input::Heavy => p.attack_held = true,
            Input::AttackUp => p.attack_held = false,
            Input::Sprint | Input::SprintUp => {
                p.sprint_held = input == Input::Sprint;
                return;
            }
            Input::Cycle => {
                self.cycle_quick(id);
                return;
            }
            _ => {}
        }
        if matches!(p.action, Action::Dead { .. }) {
            return;
        }
        // Stunned, frozen, panicked: nothing (a guard let go still counts).
        if self.disabled(id) && !matches!(input, Input::GuardUp | Input::AttackUp) {
            return;
        }
        if !self.try_act(id, input) {
            self.buffer(id, input);
        }
    }

    /// Keep an input for the end of the current commitment if it's nearly over; otherwise it's
    /// dropped (spamming doesn't queue).
    fn buffer(&mut self, id: Id, input: Input) {
        let left = self.commit_left(id);
        let window = self.content.player.buffer;
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        if matches!(input, Input::GuardUp | Input::AttackUp | Input::Cancel | Input::Sprint | Input::SprintUp) {
            return;
        }
        if left.is_some_and(|l| l <= window) {
            p.buffered = Some((input, now));
        }
    }

    /// Seconds left of what the hero is committed to, if they're committed.
    pub fn commit_left(&self, id: Id) -> Option<f32> {
        let p = self.players.get(&id)?;
        let c = &self.content;
        match &p.action {
            Action::Swing { t, strike, .. } => Some(strike.total() - t),
            Action::Dodge { t, .. } => Some(c.dodge.secs - t),
            Action::Cast { t, total, .. } => Some(total - t),
            Action::Dash { t, secs, .. } => Some(secs - t),
            Action::Stagger { until } => Some(until - self.clock),
            _ => None,
        }
    }

    /// Act on an input now if the hero is free to; false if it must wait.
    fn try_act(&mut self, id: Id, input: Input) -> bool {
        let c = self.content.clone();
        let Some(p) = self.players.get(&id) else { return true };
        // In Ice Block nothing but its own key (an ability key) does anything.
        if self.immune(id) && !matches!(input, Input::Ability(..) | Input::GuardUp | Input::AttackUp) {
            return true;
        }
        let aiming = matches!(p.action, Action::Aim { .. });
        let charging = matches!(p.action, Action::Charge { .. });
        let free = matches!(p.action, Action::Ready | Action::Guard);
        let busy_soft = matches!(p.action, Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. });
        let fresh = p.stamina >= c.player.exhausted_below;
        match input {
            // While aiming, the click charges the ability (or casts it); its release lets it go.
            Input::Attack | Input::Heavy if aiming => {
                self.ability_press(id);
                true
            }
            Input::Attack | Input::Heavy if charging => true,
            Input::AttackUp if charging => {
                self.ability_release(id);
                true
            }
            Input::Attack | Input::Heavy => {
                if matches!(p.action, Action::Charge { .. } | Action::Channel { .. } | Action::Stance { .. } | Action::Dash { .. }) {
                    return true;
                }
                self.unsheathe(id);
                let p = &self.players[&id];
                // With the guard up, an attack shoves: the shield bash, or a kick.
                if matches!(p.action, Action::Guard) {
                    let kick = input == Input::Heavy || self.weapon_of(p).two_handed;
                    return self.try_shove(id, kick);
                }
                // Late in a light blow's recovery, the next of the chain comes at once.
                if let Action::Swing { t, strike, .. } = &p.action {
                    let chain = input == Input::Attack
                        && matches!(strike.kind, StrikeKind::Light | StrikeKind::Running | StrikeKind::Roll | StrikeKind::Leap)
                        && *t >= strike.windup + strike.active + strike.recovery * c.combat.chain_after;
                    if !chain {
                        return false;
                    }
                    if !fresh {
                        self.winded(id);
                        return true;
                    }
                    self.start_swing(id, StrikeKind::Light);
                    return true;
                }
                // Pressed during a dodge: the attack out of it, as the roll ends (or, if it ends in
                // the air, as the hero lands).
                if input == Input::Attack && matches!(p.action, Action::Dodge { .. }) {
                    self.players.get_mut(&id).unwrap().dodge_attack = true;
                    return true;
                }
                // In the air out of a Corey roll: it strikes as the hero lands, with the speed
                // the leap carried.
                if input == Input::Attack && (free || busy_soft) && self.in_corey(id) {
                    self.players.get_mut(&id).unwrap().landing_attack = Some(StrikeKind::Leap);
                    return true;
                }
                if !(free || busy_soft) {
                    return false;
                }
                if !fresh {
                    self.winded(id);
                    return true;
                }
                let kind = if input == Input::Heavy {
                    StrikeKind::Heavy
                } else if p.sprinting {
                    StrikeKind::Running
                } else {
                    StrikeKind::Light
                };
                self.start_swing(id, kind);
                true
            }
            Input::AttackUp | Input::Sprint | Input::SprintUp => true,
            Input::Guard => {
                // Right click calls off an aim or a charge.
                if aiming || charging {
                    self.ability_cancel(id);
                }
                let p = &self.players[&id];
                if matches!(p.action, Action::Channel { .. } | Action::Stance { .. } | Action::Dash { .. }) {
                    return true;
                }
                // A feint: the guard raised while a blow still winds up calls it off, its stamina back.
                if let Action::Swing { t, strike, .. } = &p.action {
                    if *t < strike.windup {
                        let refund = strike.stamina;
                        let most = c.player.stamina;
                        if let Some(p) = self.players.get_mut(&id) {
                            p.stamina = (p.stamina + refund).min(most);
                            p.combo_next = 0;
                            p.combo_until = self.clock;
                        }
                        self.events.push(Event::Stamina { player: id });
                        self.events.push(Event::ClipStop { id });
                        self.unsheathe(id);
                        self.set_action(id, Action::Guard);
                        return true;
                    }
                }
                if matches!(p.action, Action::Ready) || busy_soft || self.cancelable(p) || matches!(p.action, Action::Cast { .. }) {
                    self.unsheathe(id);
                    self.set_action(id, Action::Guard);
                    return true;
                }
                // Held through a swing: the guard goes up when the swing is done.
                !matches!(p.action, Action::Stagger { .. })
            }
            Input::GuardUp => {
                if matches!(p.action, Action::Guard) {
                    self.set_action(id, Action::Ready);
                }
                true
            }
            Input::Ability(slot, ex) => self.ability_key(id, slot as usize, ex),
            Input::Cancel => {
                self.ability_cancel(id);
                if matches!(self.players[&id].action, Action::Channel { .. }) {
                    self.end_ability_action(id);
                    self.set_action(id, Action::Ready);
                }
                true
            }
            Input::Sheathe => {
                if !(free || matches!(p.action, Action::Aim { .. })) {
                    return true;
                }
                self.ability_cancel(id);
                let p = self.players.get_mut(&id).unwrap();
                p.sheathed = !p.sheathed;
                let text = if p.sheathed { "You put your weapons away." } else { "Weapons out." };
                self.events.push(Event::Gear { player: id });
                self.tell(id, text);
                true
            }
            Input::Dodge => {
                // A roll calls off an aim or a charge; a channel too.
                let p = &self.players[&id];
                if aiming || charging || matches!(p.action, Action::Channel { .. }) {
                    self.end_ability_action(id);
                    self.set_action(id, Action::Ready);
                }
                let p = &self.players[&id];
                if self.held_still(id) || matches!(p.action, Action::Stance { .. } | Action::Dash { .. }) {
                    return true;
                }
                let free = matches!(p.action, Action::Ready | Action::Guard);
                if !(free || busy_soft || self.cancelable(p) || matches!(p.action, Action::Cast { .. })) {
                    return false;
                }
                if !fresh || self.players[&id].stamina < self.content.dodge.needs() {
                    self.winded(id);
                    return true;
                }
                self.start_dodge(id);
                true
            }
            Input::Bash | Input::Kick => {
                if !(free || busy_soft) {
                    return false;
                }
                self.unsheathe(id);
                let kick = input == Input::Kick || self.weapon_of(&self.players[&id]).two_handed;
                self.try_shove(id, kick)
            }
            Input::Lock => {
                self.toggle_lock(id);
                true
            }
            Input::Unlock => {
                self.unlock(id);
                true
            }
            Input::Cycle => true,
            Input::Use => {
                if !free {
                    return busy_soft;
                }
                match self.quick_item(p) {
                    Some(item) => {
                        self.set_action(id, Action::Eat { t: 0.0, item });
                        self.events.push(Event::Cast { caster: id, spell: EAT_SPELL, ms: (c.items[item].use_secs * 1000.0) as u32 });
                        self.punish_healing(id);
                    }
                    None => self.tell(id, "Nothing to use: cook raw meat at a cooking fire, or brew at a cauldron."),
                }
                true
            }
        }
    }

    /// The beat after a cast is over: back to the guard if it's held, then anything buffered.
    pub(crate) fn finish_cast(&mut self, id: Id) {
        self.finish_commitment(id);
    }

    pub(crate) fn set_action(&mut self, id: Id, action: Action) {
        // A swing brings them out of a veil.
        if matches!(action, Action::Swing { .. }) {
            self.end_veil(id);
        }
        // Steadfast (Onslaught, Sanctuary), or in an unstoppable art's wind-up: no stagger takes.
        if matches!(action, Action::Stagger { .. }) && self.unstaggerable(id) {
            return;
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        let was_eating = matches!(p.action, Action::Eat { .. });
        let was_gathering = matches!(p.action, Action::Gather { .. } | Action::Work { .. });
        let was_guard = matches!(p.action, Action::Guard);
        p.action = action;
        let eating = matches!(p.action, Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. });
        let guard = matches!(p.action, Action::Guard);
        if !matches!(p.action, Action::Charge { .. }) {
            p.draw_shown = false;
        }
        if (was_eating || was_gathering) && !eating {
            self.events.push(Event::CastEnd { caster: id });
        }
        if guard != was_guard {
            self.events.push(Event::Guard { player: id, up: guard });
        }
        self.update_speed(id);
    }

    pub(crate) fn update_speed(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let speed = self.speed_for(id);
        let back = self.back_speed_for(p) * self.speed_share(id);
        if (speed - p.sent_speed).abs() > 0.01 || (back - p.sent_back).abs() > 0.01 {
            let p = self.players.get_mut(&id).unwrap();
            p.sent_speed = speed;
            p.sent_back = back;
            self.events.push(Event::Speed { player: id, speed, back });
        }
    }

    pub(crate) fn spend(&mut self, id: Id, amount: f32) {
        let now = self.clock;
        // Onslaught: blows and the guard cost nothing.
        let amount = if self.has_cc(id, "onslaught") { 0.0 } else { amount };
        if let Some(p) = self.players.get_mut(&id) {
            p.stamina = (p.stamina - amount).max(0.0);
            p.spent_at = now;
            self.events.push(Event::Stamina { player: id });
        }
    }

    /// Late enough in a blow's recovery to cut the rest short (a dodge, the guard).
    fn cancelable(&self, p: &Player) -> bool {
        let c = &self.content.combat;
        match &p.action {
            Action::Swing { t, strike, .. } => {
                let share = if strike.heavy() { c.heavy_cancel_after } else { c.cancel_after };
                *t >= strike.windup + strike.active + strike.recovery * share
            }
            _ => false,
        }
    }

    /// The blow a hero's swing of this kind is: its place in the chain, its timing and look.
    pub(crate) fn strike_for(&self, id: Id, kind: StrikeKind) -> Strike {
        let p = &self.players[&id];
        let step = if kind == StrikeKind::Light && self.clock <= p.combo_until { p.combo_next as usize } else { 0 };
        self.strike_at(id, kind, step)
    }

    /// The `step`th blow of a hero's set of this kind (the light chain's; 0 for the others).
    pub fn strike_at(&self, id: Id, kind: StrikeKind, step: usize) -> Strike {
        let c = &self.content;
        let p = &self.players[&id];
        let w = self.weapon_of(p);
        let set = c.moveset_of(w);
        let h = &c.heavy;
        let step = step % set.light.len();
        let (m, windup, recovery) = match kind {
            StrikeKind::Heavy => (&set.heavy, w.windup * h.windup, w.recovery * h.recovery),
            StrikeKind::Running => (&set.running, w.windup, w.recovery),
            StrikeKind::Roll => (set.roll_move(), w.windup, w.recovery),
            StrikeKind::Leap => (set.leap_move(), w.windup, w.recovery),
            _ => (&set.light[step], w.windup, w.recovery),
        };
        let heavy = kind == StrikeKind::Heavy;
        Strike {
            kind,
            step: step as u8,
            windup: windup * m.windup,
            active: w.active,
            recovery: recovery * m.recovery,
            damage: m.damage * if heavy { h.damage } else { 1.0 },
            range: w.range * m.range,
            arc: w.arc,
            stagger: if heavy { w.stagger.max(h.stagger) } else { w.stagger },
            clip: m.clip,
            take: m.take,
            lunge: m.lunge,
            stamina: w.stamina * m.stamina * if heavy { h.stamina } else { 1.0 },
            art: None,
            from: m.from.clone(),
        }
    }

    fn start_swing(&mut self, id: Id, kind: StrikeKind) {
        let weapon = self.players[&id].weapon;
        let wdef = self.content.weapon(&self.content.items[weapon].key).cloned().expect("a melee weapon");
        let windex = self.content.weapons.iter().position(|w| w.key == wdef.key).unwrap();
        let strike = self.strike_for(id, kind);
        let set_len = self.content.moveset_of(&wdef).light.len().max(1) as u8;
        self.spend(id, strike.stamina);
        {
            let p = self.players.get_mut(&id).unwrap();
            p.combo_next = if kind == StrikeKind::Light { (strike.step + 1) % set_len } else { 0 };
            p.combo_until = f32::INFINITY;
        }
        self.set_action(id, Action::Swing { t: 0.0, struck: Vec::new(), weapon: windex, strike });
        self.players.get_mut(&id).unwrap().combat_at = self.clock;
        // The swing plays now; what it strikes shows as it connects.
        let style = if kind == StrikeKind::Heavy { Blow::Heavy } else { Blow::Light };
        self.events.push(Event::Swing { attacker: id, victim: None, damage: 0, outcome: Outcome::Miss, anim: true, style });
        if kind == StrikeKind::Heavy {
            self.evade_heavy(id);
        }
    }

    /// The shield bash or the kick, if there's stamina for it.
    fn try_shove(&mut self, id: Id, kick: bool) -> bool {
        let d = if kick { self.content.kick.clone() } else { self.content.bash.clone() };
        if self.players[&id].stamina < self.content.player.exhausted_below.max(1.0) {
            self.winded(id);
            return true;
        }
        let weapon = self.players[&id].weapon;
        let windex = self.content.weapons.iter().position(|w| w.key == self.content.items[weapon].key).unwrap_or(0);
        self.spend(id, d.stamina);
        let strike = Strike {
            kind: if kick { StrikeKind::Kick } else { StrikeKind::Bash },
            step: 0,
            windup: d.windup,
            active: d.active,
            recovery: d.recovery,
            damage: 1.0,
            range: d.range,
            arc: d.arc,
            stagger: d.stagger,
            clip: d.clip,
            take: 0,
            lunge: d.lunge,
            stamina: d.stamina,
            art: None,
            from: None,
        };
        if let Some(p) = self.players.get_mut(&id) {
            p.combo_next = 0;
            p.combo_until = -100.0;
        }
        self.set_action(id, Action::Swing { t: 0.0, struck: Vec::new(), weapon: windex, strike });
        self.players.get_mut(&id).unwrap().combat_at = self.clock;
        self.events.push(Event::Swing { attacker: id, victim: None, damage: 0, outcome: Outcome::Miss, anim: true, style: Blow::Light });
        true
    }

    /// Lock on to the foe nearest the facing (within reach of sight), or let go of the one held.
    /// Tab: lock on to the foe nearest the facing; locked already, switch to the best other foe
    /// in view (ones not yet switched through first, so taps go round them all). With no other
    /// foe in view the lock stays where it is.
    pub(crate) fn toggle_lock(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let (pos, f, current) = (p.pos, p.facing, p.lock);
        // Whom they're fighting: the one they last traded blows with (a duel's foe), lately.
        let dueling = p.foe.filter(|(_, at)| self.clock - at <= LOCK_FOE_MEMORY).map(|(t, _)| t);
        // Switching looks wider: the hero faces the one they're locked on to.
        let arc = if current.is_some() { SWITCH_ARC } else { 75.0 };
        let mut found: Vec<(f32, Id)> = Vec::new();
        // Lower is likelier: the foe they're fighting first, then a hero to fight, then creatures
        // fighting someone, and creatures minding their own business (a wolf wandering by) last.
        let lean = |tid: Id, hero: bool, engaged: bool| -> f32 {
            if Some(tid) == dueling {
                -1.0
            } else if hero {
                -0.4
            } else if engaged {
                0.0
            } else {
                0.6
            }
        };
        let mut consider = |tid: Id, at: Vec3, bias: f32| {
            if Some(tid) == current {
                return;
            }
            let d = dist2d(pos, at);
            if !(0.3..=LOCK_RANGE).contains(&d) {
                return;
            }
            let off = angle_between(f, facing(pos, at));
            if off > arc || !clear_line(self.land.as_ref(), pos, at) {
                return;
            }
            found.push((d / LOCK_RANGE + off / arc + bias, tid));
        };
        for (mid, m) in &self.mobs {
            if m.health > 0 && !m.hidden {
                consider(*mid, m.pos, lean(*mid, false, m.state.foe().is_some()));
            }
        }
        if self.content.pvp.friendly_fire {
            for (hid, h) in &self.players {
                if *hid != id && h.health > 0 && self.hostile(id, *hid) {
                    consider(*hid, h.pos, lean(*hid, true, true));
                }
            }
        }
        let p = self.players.get_mut(&id).unwrap();
        let best = |list: &mut dyn Iterator<Item = &(f32, Id)>| {
            list.fold(None::<(f32, Id)>, |b, &(s, t)| if b.is_none_or(|(bs, _)| s < bs) { Some((s, t)) } else { b })
        };
        let fresh = best(&mut found.iter().filter(|(_, t)| !p.lock_seen.contains(t)));
        let target = match (current, fresh.or_else(|| best(&mut found.iter()))) {
            (_, Some((_, t))) => t,
            // Nothing else in view: keep the one held.
            (Some(_), None) => return,
            (None, None) => {
                self.events.push(Event::Lock { player: id, target: None });
                return;
            }
        };
        if current.is_none() || fresh.is_none() {
            p.lock_seen.clear();
        }
        if let Some(c) = current {
            if !p.lock_seen.contains(&c) {
                p.lock_seen.push(c);
            }
        }
        p.lock = Some(target);
        self.events.push(Event::Lock { player: id, target: Some(target) });
    }

    /// Tab held: let go of the lock.
    pub(crate) fn unlock(&mut self, id: Id) {
        let Some(p) = self.players.get_mut(&id) else { return };
        p.lock_seen.clear();
        if p.lock.take().is_some() {
            self.events.push(Event::Lock { player: id, target: None });
        }
    }

    /// Too winded for what they tried.
    pub(crate) fn winded(&mut self, id: Id) {
        self.events.push(Event::Winded { player: id });
    }

    /// Two who just traded blows: each is the other's foe (the bar on their screen).
    pub(crate) fn trade_blows(&mut self, a: Id, b: Id) {
        let now = self.clock;
        if a == b {
            return;
        }
        if let Some(p) = self.players.get_mut(&a) {
            p.foe = Some((b, now));
        }
        if let Some(p) = self.players.get_mut(&b) {
            p.foe = Some((a, now));
        }
    }

    /// Where a swing goes: toward the foe the hero is locked on to when it's in reach, else the
    /// facing, or toward a foe just outside its arc (the soft lock), so a blow aimed a little wide
    /// still finds who it was meant for. Foes are where the striker saw them (`seen_at`).
    fn swing_aim_at(&self, id: Id, w: WeaponReach, seen_at: f32) -> f32 {
        let p = &self.players[&id];
        let (pos, f) = (p.pos, p.facing);
        let where_ = |tid: Id, now: Vec3| self.pos_at(tid, seen_at).unwrap_or(now);
        if let Some(t) = p.lock {
            let at = self.mobs.get(&t).filter(|m| m.health > 0).map(|m| (where_(t, m.pos), 90.0)).or_else(|| self.players.get(&t).filter(|h| h.health > 0).map(|h| (where_(t, h.pos), LOCK_SNAP_HERO)));
            if let Some((at, snap)) = at {
                if dist2d(pos, at) <= w.range + 1.0 && angle_between(f, facing(pos, at)) <= snap {
                    return facing(pos, at);
                }
            }
        }
        let half = w.arc / 2.0;
        let cone = half + self.content.lock.melee;
        let mut best: Option<(f32, f32)> = None;
        let mut inside = false;
        let mut consider = |at: Vec3, size: f32| {
            let d = dist2d(pos, at);
            if d > w.range + size + 0.3 {
                return;
            }
            let toward = facing(pos, at);
            let off = angle_between(f, toward);
            if off > cone || !clear_line(self.land.as_ref(), pos, at) {
                return;
            }
            if off <= half {
                inside = true;
            }
            // The nearest to the line of the blow, then the nearest.
            let score = off / cone + 0.25 * d / w.range.max(0.1);
            if best.is_none_or(|(s, _)| score < s) {
                best = Some((score, toward));
            }
        };
        for (mid, m) in self.mobs.iter().filter(|(_, m)| m.health > 0) {
            consider(where_(*mid, m.pos), 0.5 * self.content.mobs[m.def].scale);
        }
        if self.content.pvp.friendly_fire {
            for (hid, h) in &self.players {
                if *hid != id && h.health > 0 && self.hostile(id, *hid) {
                    consider(where_(*hid, h.pos), HERO_SIZE);
                }
            }
        }
        match best {
            Some((_, toward)) if !inside => toward,
            _ => f,
        }
    }

    fn start_dodge(&mut self, id: Id) {
        if let Some(p) = self.players.get_mut(&id) {
            p.dodge_attack = false;
        }
        let c = self.content.dodge.clone();
        let p = &self.players[&id];
        let m = p.moving;
        // The direction the movement keys point, or a step back.
        let (mut fx, mut fy) = (0.0f32, 0.0f32);
        let mut add = |a: f32| {
            fx += a.cos();
            fy += a.sin();
        };
        if m.forward {
            add(0.0);
        }
        if m.back {
            add(PI);
        }
        if m.left {
            add(FRAC_PI_2);
        }
        if m.right {
            add(-FRAC_PI_2);
        }
        let dash = fx.abs() + fy.abs() >= 0.01;
        let rel = if dash { fy.atan2(fx) } else { PI };
        let dir = (p.facing + rel).rem_euclid(std::f32::consts::TAU);
        self.spend(id, c.stamina);
        // With a direction held, the dash is the hero's own run, sped up (set_action sends the
        // speed); with none, a hop back.
        self.set_action(id, Action::Dodge { t: 0.0, dir, dash });
        // A client that runs its own dodge gets no throw; others hop back with no keys held.
        let local = self.players[&id].local_moves;
        let (speed, lift) = if dash || local { (0.0, 0.0) } else { (c.back_step[0], c.back_step[1]) };
        // Forward (or forward and to a side) it tumbles; to a side it sways aside; back, a step.
        let style = if !dash {
            "back"
        } else if rel.abs() <= 1.0 {
            "roll"
        } else if rel.abs() >= 2.2 {
            "back"
        } else if rel > 0.0 {
            "left"
        } else {
            "right"
        };
        self.events.push(Event::Hop { player: id, dir, speed, lift, roll: true, style });
        if c.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: c.spell, target: Target::Unit(id) });
        }
    }

    /// Advance every hero one tick.
    pub(crate) fn step_players(&mut self, dt: f32) {
        for id in self.player_ids() {
            self.check_lock(id);
            self.step_player(id, dt);
        }
    }

    /// A lock lets go once its target is no foe to fight any more: dead or gone, gone home
    /// (leashed), or well out of reach; and a fallen hero holds none. (A lock held on leaves the
    /// body turned to it, walking sideways and back.)
    fn check_lock(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let Some(t) = p.lock else { return };
        let pos = p.pos;
        let fallen = p.health == 0;
        let keep = !fallen
            && match (self.mobs.get(&t), self.players.get(&t)) {
                (Some(m), _) => m.health > 0 && !matches!(m.state, mobs::MobState::Return | mobs::MobState::Dead { .. }) && dist2d(m.pos, pos) <= LOCK_RANGE * LOCK_HOLD,
                (None, Some(h)) => h.health > 0 && dist2d(h.pos, pos) <= LOCK_RANGE * LOCK_HOLD,
                _ => false,
            };
        if !keep {
            self.players.get_mut(&id).unwrap().lock = None;
            self.events.push(Event::Lock { player: id, target: None });
        }
    }

    fn step_player(&mut self, id: Id, dt: f32) {
        let now = self.clock;
        let c = self.content.clone();
        // Getting up.
        if let Some(Action::Dead { until }) = self.players.get(&id).map(|p| p.action.clone()) {
            if now >= until {
                self.revive(id);
            }
            return;
        }
        // Sprinting: Shift held, moving (and not walking), nothing else in hand, stamina to spend.
        // No look or sound of its own: the pace and the run animation say it (as with the dodge).
        {
            let p = &self.players[&id];
            let moving = p.moving.forward || p.moving.back || p.moving.left || p.moving.right;
            // Run dry, it stops; it starts again only with some stamina back.
            let enough = if p.sprinting { p.stamina > c.player.exhausted_below } else { p.stamina >= c.player.sprint_restart.max(c.player.exhausted_below) };
            let want = p.sprint_held && moving && !p.moving.walk && matches!(p.action, Action::Ready) && enough;
            if want != p.sprinting {
                let p = self.players.get_mut(&id).unwrap();
                p.sprinting = want;
                self.update_speed(id);
            }
            if want {
                let p = self.players.get_mut(&id).unwrap();
                let before = p.stamina.floor();
                p.stamina = (p.stamina - c.player.sprint_drain * dt).max(0.0);
                p.spent_at = now;
                if p.stamina.floor() != before {
                    self.events.push(Event::Stamina { player: id });
                }
            }
        }
        // Stamina comes back after a pause, not while the shield is up; armor slows it, a tonic
        // quickens it.
        {
            let rate = {
                let p = &self.players[&id];
                c.player.stamina_regen * (1.0 - self.armor_weight(p)) * (1.0 + self.effect_of(p, "stamina"))
            };
            let p = self.players.get_mut(&id).unwrap();
            let before = p.stamina.floor();
            let guarding = matches!(p.action, Action::Guard);
            // Nothing comes back mid-blow or mid-dodge: the pause starts as it ends.
            if c.combat.stamina_waits && matches!(p.action, Action::Swing { .. } | Action::Dodge { .. }) {
                p.spent_at = now;
            }
            if !guarding && now - p.spent_at >= c.player.stamina_delay && p.stamina < c.player.stamina {
                p.stamina = (p.stamina + rate * dt).min(c.player.stamina);
            }
            if p.stamina.floor() != before {
                self.events.push(Event::Stamina { player: id });
            }
        }
        // Lasting effects: a salve's healing, and running out.
        {
            let regen = self.effect_of(&self.players[&id], "regen");
            let p = self.players.get_mut(&id).unwrap();
            if regen > 0.0 && p.health > 0 && p.health < p.max_health {
                p.heal_bank += p.max_health as f32 * regen * dt;
                if p.heal_bank >= 1.0 {
                    let n = p.heal_bank.floor();
                    p.heal_bank -= n;
                    let was = p.health;
                    p.health = (p.health + n as u32).min(p.max_health);
                    let (h, m) = (p.health, p.max_health);
                    self.events.push(Event::Health { id, health: h, max: m });
                    // Shown as it comes in (green numbers), so a salve is seen working.
                    self.events.push(Event::Healed { id, amount: h - was });
                }
            }
            let p = self.players.get_mut(&id).unwrap();
            let before = p.effects.len();
            p.effects.retain(|e| e.until > now);
            if p.effects.len() != before {
                self.events.push(Event::Effects { player: id });
            }
        }
        // Drawing the bow on the move, the legs walk (the drawn pose's cast is withdrawn); stood
        // still, it's shown again.
        if let Action::Charge { ability, .. } = self.players[&id].action {
            if self.content.abilities[ability].kind == "shot" {
                let walking = self.walking(&self.players[&id]);
                let shown = self.players[&id].draw_shown;
                let spell = self.ranged_of(&self.players[&id]).spell;
                if walking && shown {
                    self.players.get_mut(&id).unwrap().draw_shown = false;
                    self.events.push(Event::CastEnd { caster: id });
                } else if !walking && !shown && spell != 0 {
                    self.players.get_mut(&id).unwrap().draw_shown = true;
                    self.events.push(Event::Cast { caster: id, spell, ms: super::abilities::DRAW_HOLD_MS });
                }
            }
        }
        // The action in hand.
        let action = self.players[&id].action.clone();
        match action {
            Action::Swing { t, struck, weapon, strike } => {
                let w = c.weapons[weapon].clone();
                let t2 = t + dt;
                let mut struck = struck;
                if t2 >= strike.windup && t < strike.windup + strike.active {
                    self.connect(id, &w, &strike, &mut struck);
                }
                // Parried mid-swing: the stagger stands.
                if !matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Swing { .. })) {
                } else if t2 >= strike.total() {
                    self.finish_commitment(id);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Swing { t: t2, struck, weapon, strike };
                }
            }
            Action::Dodge { t, dir, dash } => {
                if t + dt >= c.dodge.secs {
                    self.finish_commitment(id);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Dodge { t: t + dt, dir, dash };
                    // The dash spent: back to the usual speeds for the rest of the roll.
                    if t < c.dodge.dash_secs && t + dt >= c.dodge.dash_secs {
                        self.update_speed(id);
                    }
                }
            }
            Action::Charge { .. } | Action::Cast { .. } | Action::Channel { .. } | Action::Dash { .. } | Action::Stance { .. } => {
                self.step_ability_action(id, dt);
            }
            Action::Stagger { until } => {
                if now >= until {
                    self.finish_commitment(id);
                }
            }
            Action::Eat { t, item } => {
                let d = &c.items[item];
                if t + dt >= d.use_secs {
                    // Eaten: the bite is taken from the bag only now.
                    let p = self.players.get_mut(&id).unwrap();
                    if let Some(n) = p.bag.get_mut(&item) {
                        *n -= 1;
                        if *n == 0 {
                            p.bag.remove(&item);
                        }
                        let heal = (p.max_health as f32 * d.heal).round() as u32;
                        p.health = (p.health + heal).min(p.max_health);
                        let (h, m) = (p.health, p.max_health);
                        // What it does for a while: a fresh one replaces the last of its kind.
                        if let Some(e) = &d.effect {
                            p.effects.retain(|x| x.kind != e.kind);
                            let name = if e.name.is_empty() { d.name.clone() } else { e.name.clone() };
                            p.effects.push(Effect { kind: e.kind.clone(), amount: e.amount, until: now + e.secs, name: name.clone() });
                            self.events.push(Event::Effects { player: id });
                            // Said plainly, so a slow effect isn't taken for nothing.
                            let what = match e.kind.as_str() {
                                "regen" => format!("{name}: you heal {:.0}% of your health over {:.0} s.", e.amount * e.secs * 100.0, e.secs),
                                "stamina" => format!("{name}: your stamina comes back {:.0}% faster for {:.0} s.", e.amount * 100.0, e.secs),
                                "might" => format!("{name}: your blows land {:.0}% harder for {:.0} s.", e.amount * 100.0, e.secs),
                                "ward" => format!("{name}: blows on you lose {:.0}% more for {:.0} s.", e.amount * 100.0, e.secs),
                                "focus" => format!("{name}: your abilities come back {:.0}% sooner for {:.0} s.", e.amount * 100.0, e.secs),
                                _ => format!("{name} for {:.0} s.", e.secs),
                            };
                            self.events.push(Event::Tell { player: id, text: what });
                        }
                        self.events.push(Event::Health { id, health: h, max: m });
                        self.events.push(Event::Bags { player: id });
                        self.events.push(Event::Dirty);
                        self.events.push(Event::Missile { caster: id, spell: EAT_DONE_SPELL, target: Target::Unit(id) });
                    }
                    self.events.push(Event::CastEnd { caster: id });
                    self.players.get_mut(&id).unwrap().action = Action::Ready;
                    self.finish_commitment(id);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Eat { t: t + dt, item };
                }
            }
            Action::Gather { t, node, at } => {
                let moved = dist2d(self.players[&id].pos, at) > 1.5;
                let gone = !matches!(self.objs.get(&node).map(|o| &o.kind), Some(ObjKind::Node { ready: true, .. }) | Some(ObjKind::Chest { spent: false, .. }));
                if moved || gone {
                    self.set_action(id, Action::Ready);
                    if moved {
                        self.tell(id, "You stopped gathering.");
                    }
                } else {
                    let (secs, chest) = match &self.objs[&node].kind {
                        ObjKind::Node { def, .. } => (c.nodes[*def].gather_secs, None),
                        ObjKind::Chest { def, .. } => (c.chests[*def].secs, Some(*def)),
                        _ => (1.0, None),
                    };
                    // A chop each `CHOP_SECS`, while there's another to come.
                    let clip = match &self.objs[&node].kind {
                        ObjKind::Node { def, .. } => c.nodes[*def].gather_clip,
                        _ => 0,
                    };
                    let chop = super::world::CHOP_SECS;
                    if clip != 0 && (t / chop).floor() != ((t + dt) / chop).floor() && t + dt + chop * 0.5 < secs {
                        self.events.push(Event::Clip { id, clip, secs: chop });
                    }
                    if t + dt >= secs {
                        self.events.push(Event::CastEnd { caster: id });
                        self.players.get_mut(&id).unwrap().action = Action::Ready;
                        match chest {
                            Some(def) => self.open_chest(id, def, node),
                            None => self.gathered(id, node),
                        }
                        self.update_speed(id);
                    } else if let Some(p) = self.players.get_mut(&id) {
                        p.action = Action::Gather { t: t + dt, node, at };
                    }
                }
            }
            Action::Work { t, total, job, at } => {
                if dist2d(self.players[&id].pos, at) > 1.5 {
                    self.set_action(id, Action::Ready);
                    self.tell(id, "You stopped working.");
                } else if t + dt >= total {
                    self.events.push(Event::CastEnd { caster: id });
                    self.players.get_mut(&id).unwrap().action = Action::Ready;
                    self.update_speed(id);
                    self.work_done(id, job);
                } else if let Some(p) = self.players.get_mut(&id) {
                    p.action = Action::Work { t: t + dt, total, job, at };
                }
            }
            Action::Ready | Action::Guard | Action::Aim { .. } | Action::Dead { .. } => {}
        }
        // Health back: resting at the camp, or a trickle in the field after a while.
        let p = &self.players[&id];
        if p.health > 0 && p.health < p.max_health {
            let heart = c.world.heart;
            let home = dist2d(p.pos, [heart[0], heart[1], 0.0]) <= c.world.rest_radius && !self.raid.active();
            let calm = now - p.combat_at >= c.player.field_regen_after;
            let rate = if home && now - p.combat_at >= 2.0 { c.player.rest_regen } else if calm { c.player.field_regen } else { 0.0 };
            if rate > 0.0 {
                let p = self.players.get_mut(&id).unwrap();
                p.heal_bank += p.max_health as f32 * rate * dt;
                if p.heal_bank >= 1.0 {
                    let n = p.heal_bank.floor();
                    p.heal_bank -= n;
                    p.health = (p.health + n as u32).min(p.max_health);
                    let (h, m) = (p.health, p.max_health);
                    self.events.push(Event::Health { id, health: h, max: m });
                }
            }
        }
        // Firm ground underfoot: remember it.
        let p = &self.players[&id];
        let (x, y) = (p.pos[0], p.pos[1]);
        let z = p.pos[2];
        let ground = self.land.height_near(x, y, z);
        if !self.land.blocked_near(x, y, z) && !self.land.water_near(x, y, z) && (z - ground).abs() < 2.0 {
            // The floor itself, not where the feet were over it.
            self.players.get_mut(&id).unwrap().last_safe = [x, y, ground];
        }
    }

    /// The end of a commitment: back to ready (or the held guard or aim), then any buffered input.
    fn finish_commitment(&mut self, id: Id) {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return };
        let next = if p.guard_held { Action::Guard } else { Action::Ready };
        let buffered = p.buffered;
        // A dodge with an attack pressed in it: the attack out of it, now, or as the hero lands.
        if matches!(p.action, Action::Dodge { .. }) && p.dodge_attack {
            let air = self.airborne(id);
            self.set_action(id, next);
            let p = self.players.get_mut(&id).unwrap();
            p.dodge_attack = false;
            p.buffered = None;
            match air {
                Some(a) => p.landing_attack = Some(if a.corey { StrikeKind::Leap } else { StrikeKind::Roll }),
                None => self.attack_out_of(id, StrikeKind::Roll),
            }
            return;
        }
        // The light chain waits a moment for its next blow.
        let grace = self.content.combat.combo_grace;
        self.set_action(id, next);
        if let Some(p) = self.players.get_mut(&id) {
            p.buffered = None;
            if p.combo_until.is_infinite() {
                p.combo_until = now + grace;
            }
        }
        if let Some((input, at)) = buffered {
            if self.clock - at <= self.content.player.buffer + 0.1 {
                let _ = self.try_act(id, input);
            }
        }
    }

    /// The swing's active moment: everything in the arc, in reach, and not behind something
    /// solid is struck, once, judged where the striker saw them (their round trip ago).
    fn connect(&mut self, id: Id, w: &crate::content::WeaponDef, strike: &Strike, struck: &mut Vec<Id>) {
        if let Some(ability) = strike.art {
            self.connect_art(id, w, strike, ability, struck);
            return;
        }
        let pos = self.players[&id].pos;
        let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
        let seen_at = self.clock - rewind;
        let reach = WeaponReach { range: strike.range, arc: strike.arc };
        let f = self.swing_aim_at(id, reach, seen_at);
        let style = if strike.heavy() { Blow::Heavy } else { Blow::Light };
        let mut hits: Vec<(f32, Id)> = Vec::new();
        for (&mid, m) in &self.mobs {
            if m.health == 0 || struck.contains(&mid) {
                continue;
            }
            let at = self.pos_at(mid, seen_at).unwrap_or(m.pos);
            let size = 0.5 * self.content.mobs[m.def].scale;
            let d = dist2d(pos, at);
            if d > strike.range + size {
                continue;
            }
            let off = angle_between(f, facing(pos, at));
            // Right on top of you still has to be in front of you.
            if off > strike.arc / 2.0 && !(d < size + 0.4 && off < 90.0) {
                continue;
            }
            if !clear_line(self.land.as_ref(), pos, at) {
                continue;
            }
            hits.push((d, mid));
        }
        // Other heroes, with friendly fire on.
        if self.content.pvp.friendly_fire {
            for (&hid, h) in &self.players {
                if hid == id || h.health == 0 || struck.contains(&hid) || !self.hostile(id, hid) {
                    continue;
                }
                let at = self.pos_at(hid, seen_at).unwrap_or(h.pos);
                let d = dist2d(pos, at);
                if d > strike.range + HERO_SIZE {
                    continue;
                }
                let off = angle_between(f, facing(pos, at));
                if off > strike.arc / 2.0 && !(d < HERO_SIZE + 0.4 && off < 90.0) {
                    continue;
                }
                if !clear_line(self.land.as_ref(), pos, at) {
                    continue;
                }
                hits.push((d, hid));
            }
        }
        // The nearest first, as many as the weapon strikes in one swing (a shove, one).
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        let most = if strike.shove() { 1 } else { w.targets.max(1) };
        let room = most.saturating_sub(struck.len());
        let lean = if strike.heavy() { self.content.combat.lean_heavy } else { self.content.combat.lean_light };
        for (_, mid) in hits.into_iter().take(room) {
            struck.push(mid);
            if self.players.contains_key(&mid) {
                if strike.shove() {
                    self.shove_hero(id, mid, strike);
                } else {
                    self.strike_hero(id, mid, w, strike);
                }
                continue;
            }
            // A training dummy's counter, roll or guard.
            if let Some(out) = self.counter_catch(mid, id, HitKind::Melee, None) {
                if out != Outcome::Hit {
                    self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage: 0, outcome: out, anim: false, style });
                    self.trade_blows(id, mid);
                    continue;
                }
            }
            if self.dummy_rolls(mid) {
                self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage: 0, outcome: Outcome::Dodged, anim: false, style });
                continue;
            }
            if self.dummy_guards(mid, pos) && !strike.heavy() && !strike.shove() {
                self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage: 0, outcome: Outcome::Blocked, anim: false, style });
                self.events.push(Event::React { id: mid, anim: BLOCK_ANIM, from: pos, lean: 0.1, hold: 0.0, fit: 0.0 });
                self.trade_blows(id, mid);
                continue;
            }
            // Gone home past its leash: the blow finds nothing.
            if self.evading(mid, id) {
                self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage: 0, outcome: Outcome::Miss, anim: false, style });
                self.events.push(Event::Flash { player: id, text: "Evade".into() });
                continue;
            }
            // A shield-bearer between blows, facing the blow, may take a light one on its shield.
            let shielded = self.mobs.get(&mid).is_some_and(|m| {
                let d = &self.content.mobs[m.def];
                d.block > 0.0
                    && d.shield != 0
                    && m.stagger_until <= self.clock
                    && matches!(m.state, mobs::MobState::Chase { .. } | mobs::MobState::Recover { .. })
                    && angle_between(m.facing, facing(m.pos, pos)) <= 70.0
            });
            if shielded && !strike.heavy() && !strike.shove() {
                let chance = self.mobs.get(&mid).map_or(0.0, |m| self.content.mobs[m.def].block);
                if self.rng.gen::<f32>() < chance {
                    self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage: 0, outcome: Outcome::Blocked, anim: false, style: Blow::Light });
                    self.events.push(Event::React { id: mid, anim: BLOCK_ANIM, from: pos, lean: 0.1, hold: 0.0, fit: 0.0 });
                    self.trade_blows(id, mid);
                    if self.mobs.get(&mid).is_some_and(|m| matches!(m.state, mobs::MobState::Idle { .. } | mobs::MobState::Return)) {
                        self.engage(mid, mobs::Foe::Hero(id));
                    }
                    continue;
                }
            }
            let winding = self.mobs.get(&mid).is_some_and(|m| matches!(m.state, mobs::MobState::Windup { .. }));
            let casting = winding && self.attack_now(mid).ranged;
            let boss = self.content.mobs[self.mobs[&mid].def].boss;
            let rules = &self.content.combat;
            // A poised wind-up (a heavy creature's) shrugs off a light blow. What knocks one aside
            // is a weapon made for it (or any heavy blow, by the old rules); not a boss's, not one
            // knocked aside a moment ago.
            let poised = winding && self.attack_now(mid).poised() && !strike.heavy();
            let knocker = w.stagger > 0.0 || (strike.heavy() && rules.heavy_knocks_aside);
            let wary = self.mobs.get(&mid).is_some_and(|m| m.wary_until > self.clock);
            let winding = winding && !poised && knocker && !wary && (!boss || rules.boss_knock_aside);
            // Just after its own blow (its recovery, not reeling already), a heavy blow staggers
            // it: the window to strike in. Not a boss.
            let recovering = rules.recovery_stagger
                && strike.heavy()
                && !boss
                && self.mobs.get(&mid).is_some_and(|m| matches!(m.state, mobs::MobState::Recover { .. }) && m.stagger_until <= self.clock);
            // A bash or a kick knocks anything reeling, but a boss only out of a cast.
            let shoved = strike.shove() && (rules.shove_bosses || !boss || casting);
            let reeling = self.mobs.get(&mid).is_some_and(|m| m.riposte && m.stagger_until > self.clock);
            let rolled = if strike.shove() {
                let d = if strike.kind == StrikeKind::Kick { &self.content.kick } else { &self.content.bash };
                let (lo, hi) = (d.damage[0], d.damage[1]);
                if hi == 0 { 0 } else { self.roll([lo, hi]) }
            } else {
                let might = 1.0 + self.effect_of(&self.players[&id], "might");
                (self.roll(w.damage) as f32 * strike.damage * might).round() as u32
            };
            // From behind (a backstab): harder, unless it's a riposte already.
            let behind = !reeling && self.mobs.get(&mid).is_some_and(|m| angle_between(m.facing, facing(m.pos, pos)) >= BACKSTAB_ARC);
            let stab = if w.backstab > 0.0 { w.backstab } else { self.content.combat.backstab };
            // Stabbed in the back again and again, it's ready for it: each backstab lately (more
            // so with more heroes on it) takes from the next one's bonus.
            let stab = if behind { self.backstab_bonus(mid, stab) } else { stab };
            let damage = if behind { (rolled as f32 * stab).round() as u32 } else { self.riposted(mid, rolled) };
            let damage = (damage as f32 * self.taken_share(mid)).round() as u32;
            if behind {
                self.backstabbed(mid, id);
            }
            if behind && damage > 0 {
                self.events.push(Event::Flash { player: id, text: "Backstab!".into() });
            }
            self.events.push(Event::Swing { attacker: id, victim: Some(mid), damage, outcome: Outcome::Hit, anim: false, style });
            self.trade_blows(id, mid);
            // What the blow itself set off (a second wind, a broken stance) stands: the blow's own
            // stagger doesn't undo it.
            let before = self.mobs.get(&mid).map(|m| (m.phase, m.stagger_until));
            if damage > 0 {
                self.hurt_mob(mid, id, damage);
            }
            if damage > 0 && !self.content.mobs[self.mobs[&mid].def].totem {
                let v = if strike.heavy() { self.content.valor.heavy } else { self.content.valor.light };
                self.gain_valor(id, v);
            }
            let changed = self.mobs.get(&mid).map(|m| (m.phase, m.stagger_until)) != before;
            let standing = self.mobs.get(&mid).is_some_and(|m| m.health > 0);
            // A shove knocks a creature reeling; a blow, one winding up (when blows do) or a heavy
            // one just after its own blow.
            if standing && !changed && strike.stagger > 0.0 && (winding || shoved || recovering) {
                let riposte = self.content.combat.riposte_any_stagger;
                self.stagger_mob(mid, strike.stagger, riposte);
                if winding && !shoved {
                    let wary = self.clock + self.content.combat.knock_aside_wary;
                    self.mobs.get_mut(&mid).unwrap().wary_until = wary;
                }
            } else if standing {
                self.events.push(Event::React { id: mid, anim: WOUND_ANIM, from: pos, lean: lean * 0.7, hold: 0.0, fit: 0.0 });
            }
            // The chain's last blow and a running attack cleave.
            let last = self.content.moveset_of(w).light.len().saturating_sub(1);
            let finisher = matches!(strike.kind, StrikeKind::Running | StrikeKind::Leap) || (strike.kind == StrikeKind::Light && strike.step as usize == last && last > 0);
            if strike.shove() {
                self.events.push(Event::Visual { id: mid, kit: KIT_BASH, by: Some(id) });
            } else if reeling || behind {
                self.events.push(Event::Visual { id: mid, kit: KIT_RIPOSTE, by: Some(id) });
            } else if strike.heavy() {
                let kit = self.heavy_kit();
                self.events.push(Event::Visual { id: mid, kit, by: Some(id) });
            } else if finisher {
                self.events.push(Event::Visual { id: mid, kit: KIT_CLEAVE, by: Some(id) });
            }
        }
    }

    /// Hero `id`'s swing reaches hero `victim`: a blow like any other, guarded, parried or dodged
    /// by the same rules (once their round trip has passed); a heavy weapon knocks a wind-up aside.
    fn strike_hero(&mut self, id: Id, victim: Id, w: &crate::content::WeaponDef, strike: &Strike) {
        let from = self.players[&id].pos;
        let might = 1.0 + self.effect_of(&self.players[&id], "might");
        let rolled = (self.roll(w.damage) as f32 * strike.damage * might).round() as u32;
        let damage = self.hero_riposted(victim, rolled);
        // A heavy attack under way is poised: a light blow doesn't knock it aside.
        let poised = (self.content.combat.heavy_poise
            && !strike.heavy()
            && matches!(&self.players[&victim].action, Action::Swing { t, strike, .. } if strike.kind == StrikeKind::Heavy && *t < strike.windup + strike.active))
            || self.unstaggerable(victim);
        let winding = !poised && matches!(&self.players[&victim].action, Action::Swing { t, strike, .. } if *t < strike.windup);
        if self.players[&victim].riposte_until > self.clock {
            self.events.push(Event::Visual { id: victim, kit: KIT_RIPOSTE, by: None });
        }
        let heavy = w.two_handed || w.stagger > 0.0 || strike.heavy();
        let style = if strike.heavy() { Blow::Heavy } else { Blow::Light };
        let follow = Follow::Swing { stagger: strike.stagger, heavy: strike.heavy(), winding };
        // In an Onslaught, a heavy attack goes through any guard.
        let guardable = !(strike.heavy() && self.has_cc(id, "onslaught"));
        self.strike_fair(id, victim, damage, from, guardable, heavy, guardable, style, None, follow);
    }

    /// A guard breaker reaches a hero: through a raised guard (tearing stamina from it, and
    /// breaking it if that's more than there is), a stagger and a shove; dodged in a dodge's
    /// window like anything else.
    fn shove_hero(&mut self, id: Id, victim: Id, strike: &Strike) {
        let now = self.clock;
        let d = if strike.kind == StrikeKind::Kick { self.content.kick.clone() } else { self.content.bash.clone() };
        let from = self.players[&id].pos;
        let Some(v) = self.players.get(&victim) else { return };
        if v.health == 0 {
            return;
        }
        let dodged = matches!(&v.action, Action::Dodge { t, .. } if *t >= self.content.dodge.evade_from && *t <= self.content.dodge.evade_to);
        let guarding = matches!(v.action, Action::Guard);
        let broke = guarding && v.stamina <= d.guard_break;
        if dodged {
            self.events.push(Event::Swing { attacker: id, victim: Some(victim), damage: 0, outcome: Outcome::Dodged, anim: false, style: Blow::Light });
            return;
        }
        let (lo, hi) = (d.damage[0], d.damage[1]);
        let damage = if hi == 0 { 0 } else { self.roll([lo, hi]) };
        let outcome = if broke { Outcome::GuardBroken } else if guarding { Outcome::Blocked } else { Outcome::Hit };
        if guarding {
            self.spend(victim, d.guard_break);
        }
        let landed = if guarding && !broke { 0 } else { damage };
        self.events.push(Event::Swing { attacker: id, victim: Some(victim), damage: landed, outcome, anim: false, style: Blow::Light });
        self.trade_blows(id, victim);
        if landed > 0 {
            self.hurt_player(victim, landed);
        }
        if !self.players.get(&victim).is_some_and(|p| p.health > 0) {
            return;
        }
        // Reeling, unless the guard held; shoved back either way.
        if !guarding || broke {
            // A kick through a guard puts them on the ground.
            let down = broke && strike.kind == StrikeKind::Kick;
            let secs = if down {
                self.content.combat.knockdown
            } else if broke {
                self.guard_of(&self.players[&victim]).break_secs.max(d.stagger)
            } else {
                d.stagger
            };
            self.set_action(victim, Action::Stagger { until: now + secs });
            if self.content.combat.riposte_any_stagger {
                self.players.get_mut(&victim).unwrap().riposte_until = now + secs;
            }
            self.events.push(Event::Emote { id: victim, emote: mobs::STUN_EMOTE });
            if down {
                self.events.push(Event::React { id: victim, anim: KNOCKDOWN_ANIM, from, lean: 0.0, hold: 0.0, fit: secs });
                self.events.push(Event::Visual { id: victim, kit: KIT_DUST, by: None });
            } else {
                self.events.push(Event::React { id: victim, anim: STUN_ANIM, from, lean: 0.35, hold: secs, fit: 0.0 });
            }
            if broke {
                self.tell(victim, if down { "Kicked down!" } else { "Your guard breaks!" });
            }
        }
        self.events.push(Event::Visual { id: victim, kit: KIT_BASH, by: Some(id) });
        let dir = facing(from, self.players[&victim].pos);
        self.push_hero(victim, dir, d.push[0], d.push[1]);
    }

    /// Throw a hero back along `dir`: a hop (their client flies it).
    fn push_hero(&mut self, id: Id, dir: f32, speed: f32, lift: f32) {
        if speed > 0.0 {
            self.events.push(Event::Hop { player: id, dir, speed, lift, roll: false, style: "" });
        }
    }

    /// A blow on a hero, waiting out their round trip when they have one: what they did on their
    /// own screen as they saw it coming (a dodge, a guard, stepping out of it) counts.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn strike_fair(
        &mut self,
        attacker: Id,
        victim: Id,
        damage: u32,
        from: Vec3,
        blockable: bool,
        heavy: bool,
        parryable: bool,
        style: Blow,
        area: Option<Area>,
        follow: Follow,
    ) {
        // A creature's blow is melee, its missile marked on the ground an area; a hero's swing
        // melee.
        let how = if area.is_some_and(|a| matches!(a, Area::Circle { .. } | Area::Line { .. })) { HitKind::Area } else { HitKind::Melee };
        self.strike_fair_as(attacker, victim, damage, from, blockable, heavy, parryable, style, area, follow, how);
    }

    /// [`Self::strike_fair`], saying how the blow reaches them (counters fire on melee and
    /// projectiles, not on ground and beams).
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn strike_fair_as(
        &mut self,
        attacker: Id,
        victim: Id,
        damage: u32,
        from: Vec3,
        blockable: bool,
        heavy: bool,
        parryable: bool,
        style: Blow,
        area: Option<Area>,
        follow: Follow,
        how: HitKind,
    ) {
        // A creature's blow lands as hard as its fight makes it.
        let damage = if self.mobs.contains_key(&attacker) { (damage as f32 * self.foe_blow_share(attacker)).round() as u32 } else { damage };
        let grace = self.players.get(&victim).map_or(0.0, |p| p.rtt.min(self.content.combat.max_grace));
        let pending = PendingStrike { attacker, victim, damage, from, blockable, heavy, parryable, style, at: self.clock + grace, area, follow, how };
        if grace <= 0.0 {
            self.resolve(pending);
        } else {
            self.pending.push(pending);
        }
    }

    /// Blows whose round trip is up.
    pub(crate) fn step_pending(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<PendingStrike>, Vec<PendingStrike>) = std::mem::take(&mut self.pending).into_iter().partition(|p| p.at <= now);
        self.pending = keep;
        for p in due {
            self.resolve(p);
        }
    }

    fn resolve(&mut self, s: PendingStrike) {
        let now = self.clock;
        let h = self.content.heavy.clone();
        let Some(v) = self.players.get(&s.victim) else { return };
        if v.health == 0 {
            return;
        }
        // Out of it by now: a whiff.
        let inside = match s.area {
            None => true,
            Some(Area::Arc { pos, facing: f, range, arc }) => {
                let d = dist2d(pos, v.pos);
                d <= range + 0.4 && (angle_between(f, facing(pos, v.pos)) <= arc / 2.0 || d < 0.8)
            }
            Some(Area::Circle { at, radius }) => dist2d(v.pos, at) <= radius,
            Some(Area::Line { pos, facing: f, len, half }) => in_strip(pos, f, len, half + 0.4, v.pos),
        };
        if !inside {
            self.events.push(Event::Swing { attacker: s.attacker, victim: Some(s.victim), damage: 0, outcome: Outcome::Miss, anim: false, style: s.style });
            return;
        }
        // A counter's stance meets it first.
        if let Some(out) = self.counter_catch(s.victim, s.attacker, s.how, match s.follow {
            Follow::Ability { ability, share } => Some((ability, share)),
            _ => None,
        }) {
            if out != Outcome::Hit {
                self.events.push(Event::Swing { attacker: s.attacker, victim: Some(s.victim), damage: 0, outcome: out, anim: false, style: s.style });
                self.trade_blows(s.attacker, s.victim);
                return;
            }
        }
        let out = self.strike_player(s.attacker, s.victim, s.damage, s.from, s.blockable, s.heavy, s.parryable, s.style);
        // A guard that blocked a crushing blow (Sundering Blow) pays for it many times over.
        if let (Outcome::Blocked, Follow::Ability { ability, .. }) = (out, &s.follow) {
            let ability = *ability;
            let crush = self.content.abilities[ability].crush;
            if crush > 1.0 {
                self.crush_guard(s.victim, s.damage, crush);
            }
        }
        // An ability that landed: its effects, its drain, Valor.
        if let Follow::Ability { ability, share } = s.follow {
            if matches!(out, Outcome::Hit | Outcome::GuardBroken) {
                let killed = !self.players.get(&s.victim).is_some_and(|p| p.health > 0);
                let was = self.fx_from.replace(s.from);
                self.ability_after(s.attacker, s.victim, ability, share, s.damage, killed);
                self.fx_from = was;
            }
            return;
        }
        // A hero's own blow that landed on another: Valor.
        if matches!(out, Outcome::Hit | Outcome::GuardBroken) && self.players.contains_key(&s.attacker) {
            let v = if s.style == Blow::Heavy { self.content.valor.heavy } else { self.content.valor.light };
            self.gain_valor(s.attacker, v);
        }
        let Follow::Swing { stagger, heavy, winding } = s.follow else { return };
        let standing = self.players.get(&s.victim).is_some_and(|p| p.health > 0);
        if out == Outcome::Hit && winding && stagger > 0.0 && standing {
            self.set_action(s.victim, Action::Stagger { until: now + stagger });
            if self.content.combat.riposte_any_stagger {
                self.players.get_mut(&s.victim).unwrap().riposte_until = now + stagger;
            }
            self.events.push(Event::Emote { id: s.victim, emote: mobs::STUN_EMOTE });
            self.events.push(Event::React { id: s.victim, anim: STUN_ANIM, from: s.from, lean: 0.3, hold: stagger, fit: 0.0 });
        }
        // A heavy attack that lands throws them back.
        if heavy && standing && matches!(out, Outcome::Hit | Outcome::GuardBroken) {
            let dir = facing(s.from, self.players[&s.victim].pos);
            self.push_hero(s.victim, dir, h.knockback[0], h.knockback[1]);
        }
    }

    /// Where a unit was at `t` (its trail, between the samples either side), if it has one.
    pub(crate) fn pos_at(&self, id: Id, t: f32) -> Option<Vec3> {
        let trail = self.trails.get(&id)?;
        type Sample<'a> = Option<&'a (f32, Vec3)>;
        let (mut before, mut after): (Sample, Sample) = (None, None);
        for s in trail.iter() {
            if s.0 <= t {
                before = Some(s);
            } else {
                after = Some(s);
                break;
            }
        }
        match (before, after) {
            (Some(a), Some(b)) => {
                let k = ((t - a.0) / (b.0 - a.0).max(1.0e-4)).clamp(0.0, 1.0);
                Some([a.1[0] + (b.1[0] - a.1[0]) * k, a.1[1] + (b.1[1] - a.1[1]) * k, a.1[2] + (b.1[2] - a.1[2]) * k])
            }
            (Some(a), None) => Some(a.1),
            (None, Some(b)) => Some(b.1),
            (None, None) => None,
        }
    }

    /// Everyone's whereabouts this tick, kept for a little while.
    pub(crate) fn record_trails(&mut self) {
        let now = self.clock;
        let keep = self.content.combat.max_rewind + 0.3;
        let here: Vec<(Id, Vec3)> = self
            .players
            .iter()
            .map(|(id, p)| (*id, p.pos))
            .chain(self.mobs.iter().map(|(id, m)| (*id, m.pos)))
            .collect();
        for (id, pos) in &here {
            let t = self.trails.entry(*id).or_default();
            t.push_back((now, *pos));
            while t.front().is_some_and(|s| now - s.0 > keep) {
                t.pop_front();
            }
        }
        self.trails.retain(|id, _| self.players.contains_key(id) || self.mobs.contains_key(id));
    }

    /// A blow on a hero reeling from a parry lands harder.
    pub(crate) fn hero_riposted(&self, id: Id, damage: u32) -> u32 {
        match self.players.get(&id) {
            Some(p) if p.riposte_until > self.clock => (damage as f32 * self.content.parry.riposte).round() as u32,
            _ => damage,
        }
    }

    /// A blow on a creature reeling from a parry lands harder.
    pub(crate) fn riposted(&self, mid: Id, damage: u32) -> u32 {
        match self.mobs.get(&mid) {
            Some(m) if m.riposte && m.stagger_until > self.clock => (damage as f32 * self.content.parry.riposte).round() as u32,
            _ => damage,
        }
    }

    /// A blow from creature `attacker` reaches hero `victim` from `from`: dodged in the dodge's
    /// window, parried by a guard raised just in time, blocked by a guard raised towards it, or it
    /// lands.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn strike_player(&mut self, attacker: Id, victim: Id, damage: u32, from: Vec3, blockable: bool, heavy: bool, parryable: bool, style: Blow) -> Outcome {
        let now = self.clock;
        let c = self.content.clone();
        let Some(p) = self.players.get(&victim) else { return Outcome::Miss };
        if p.health == 0 {
            return Outcome::Miss;
        }
        let guard = self.guard_of(p).clone();
        let cost = (guard.cost + damage as f32 * guard.cost_per_damage) * if heavy { 2.0 } else { 1.0 };
        // A blow heavy enough to put a hero down when it breaks their guard: a heavy attack, or a
        // heavy creature's.
        let heavy_blow = style == Blow::Heavy || (heavy && self.mobs.contains_key(&attacker));
        // Untouchable: in Ice Block, or mid-dash on an ability that's untouchable while it runs.
        let dashing = matches!(&p.action, Action::Dash { ability, .. } if self.content.abilities[*ability].invulnerable);
        let outcome = match &p.action {
            _ if self.immune(victim) => Outcome::Blocked,
            _ if dashing => Outcome::Dodged,
            Action::Dodge { t, .. } if *t >= c.dodge.evade_from && *t <= c.dodge.evade_to => Outcome::Dodged,
            Action::Guard if blockable => {
                let toward = facing(p.pos, from);
                if angle_between(p.facing, toward) > guard.arc / 2.0 {
                    Outcome::Hit
                } else if parryable && now - p.guard_at <= c.parry.window + self.weapon_of(p).parry && p.guard_at - p.prev_guard_at >= c.parry.cooldown {
                    Outcome::Parried
                } else if p.stamina >= cost {
                    Outcome::Blocked
                } else {
                    Outcome::GuardBroken
                }
            }
            _ => Outcome::Hit,
        };
        // Armor takes its share off whatever gets through.
        let worn = 1.0 - self.armor_reduce(p);
        let worn = worn * self.taken_share(victim);
        let landed = match outcome {
            Outcome::Hit => (damage as f32 * worn).round() as u32,
            Outcome::GuardBroken => (damage as f32 * guard.break_share * worn).round() as u32,
            _ => 0,
        };
        match outcome {
            Outcome::Blocked => self.spend(victim, cost),
            Outcome::GuardBroken => {
                self.spend(victim, 1000.0);
                let secs = if heavy_blow { guard.break_secs.max(c.combat.knockdown) } else { guard.break_secs };
                self.set_action(victim, Action::Stagger { until: now + secs });
                if c.combat.riposte_any_stagger {
                    self.players.get_mut(&victim).unwrap().riposte_until = now + secs;
                }
                self.tell(victim, if heavy_blow { "Your guard breaks and you go down!" } else { "Your guard breaks!" });
            }
            Outcome::Parried => {
                self.events.push(Event::Flash { player: victim, text: "Parry!".into() });
                if self.mobs.contains_key(&attacker) {
                    self.stagger_mob(attacker, c.parry.stagger, true);
                } else if self.players.get(&attacker).is_some_and(|a| a.health > 0) {
                    // A hero parried: they reel, open to a riposte.
                    let until = now + c.parry.hero_stagger;
                    self.set_action(attacker, Action::Stagger { until });
                    self.players.get_mut(&attacker).unwrap().riposte_until = until;
                    self.events.push(Event::Emote { id: attacker, emote: mobs::STUN_EMOTE });
                    let at = self.players[&victim].pos;
                    self.events.push(Event::React { id: attacker, anim: STUN_ANIM, from: at, lean: 0.3, hold: c.parry.hero_stagger, fit: 0.0 });
                    self.events.push(Event::Flash { player: attacker, text: "Parried!".into() });
                }
            }
            _ => {}
        }
        if let Some(p) = self.players.get_mut(&victim) {
            p.last_hit_by = Some(attacker);
        }
        match outcome {
            Outcome::Parried => self.gain_valor(victim, c.valor.parry),
            Outcome::Dodged if !dashing && damage > 0 => self.gain_valor(victim, c.valor.roll),
            _ => {}
        }
        // A heavy blow that gets through breaks a fragile charge (Pyroblast's).
        let fragile = matches!(&self.players[&victim].action, Action::Charge { ability, .. } if self.content.abilities[*ability].fragile);
        if fragile && heavy_blow && matches!(outcome, Outcome::Hit | Outcome::GuardBroken) {
            self.tell(victim, "The heavy blow breaks your charge!");
            self.end_ability_action(victim);
            self.set_action(victim, Action::Ready);
        }
        // How it shows on the bodies (on the clients that play clips).
        let lean = match style {
            Blow::Heavy => c.combat.lean_heavy,
            Blow::Arrow(_) => c.combat.lean_light * 0.6,
            Blow::Light if heavy => c.combat.lean_heavy * 0.8,
            Blow::Light => c.combat.lean_light,
            Blow::Dot => 0.0,
        };
        // A heavy creature's blow that lands puts a hero on the ground.
        let felled = outcome == Outcome::Hit && heavy && self.mobs.contains_key(&attacker) && landed < p_health(self, victim);
        match outcome {
            Outcome::Hit if felled => {
                let secs = c.combat.knockdown;
                self.set_action(victim, Action::Stagger { until: now + secs });
                if c.combat.riposte_any_stagger {
                    self.players.get_mut(&victim).unwrap().riposte_until = now + secs;
                }
                self.events.push(Event::React { id: victim, anim: KNOCKDOWN_ANIM, from, lean: 0.0, hold: 0.0, fit: secs });
                self.events.push(Event::Visual { id: victim, kit: KIT_DUST, by: None });
            }
            Outcome::Hit => {
                let anim = if style == Blow::Heavy { CRIT_ANIM } else { WOUND_ANIM };
                self.events.push(Event::React { id: victim, anim, from, lean, hold: 0.0, fit: 0.0 });
                if style == Blow::Heavy {
                    let kit = self.heavy_kit();
                    self.events.push(Event::Visual { id: victim, kit, by: Some(attacker) });
                }
            }
            Outcome::GuardBroken if heavy_blow => {
                let secs = guard.break_secs.max(c.combat.knockdown);
                self.events.push(Event::React { id: victim, anim: KNOCKDOWN_ANIM, from, lean: 0.0, hold: 0.0, fit: secs });
                self.events.push(Event::Visual { id: victim, kit: KIT_DUST, by: None });
            }
            Outcome::GuardBroken => {
                self.events.push(Event::React { id: victim, anim: STUN_ANIM, from, lean: lean.max(0.35), hold: guard.break_secs, fit: 0.0 });
                self.events.push(Event::Visual { id: victim, kit: KIT_BASH, by: None });
            }
            Outcome::Blocked => {
                self.events.push(Event::React { id: victim, anim: BLOCK_ANIM, from, lean: lean * 0.4, hold: 0.0, fit: 0.0 });
            }
            Outcome::Parried => {
                self.events.push(Event::Visual { id: victim, kit: KIT_PARRY, by: None });
                self.events.push(Event::React { id: victim, anim: PARRY_ANIM, from, lean: 0.06, hold: 0.0, fit: 0.0 });
            }
            _ => {}
        }
        self.events.push(Event::Swing { attacker, victim: Some(victim), damage: landed, outcome, anim: false, style });
        self.trade_blows(attacker, victim);
        if let Some(p) = self.players.get_mut(&victim) {
            p.combat_at = now;
        }
        if landed > 0 {
            self.hurt_player(victim, landed);
        }
        outcome
    }

    /// Their client jumped: it costs stamina (the add-on won't jump without enough). Taken in a
    /// dodge's dash, it's a Corey roll: the client carries the dash's speed through the air, and
    /// an attack in it strikes as the hero lands.
    pub fn jumped(&mut self, id: Id) {
        let cost = self.content.player.jump_stamina;
        let now = self.clock;
        let dash = self.content.dodge.dash_secs + COREY_GRACE;
        let Some(p) = self.players.get_mut(&id) else { return };
        if p.health == 0 {
            return;
        }
        let corey = matches!(p.action, Action::Dodge { t, dash: true, .. } if t < dash);
        p.air = Some(Air { since: now, corey });
        p.landing_attack = None;
        if cost > 0.0 {
            self.spend(id, cost);
        }
    }

    /// Their client landed: an attack pressed in the air (out of a Corey roll, or out of a dodge
    /// that ended in it) strikes now.
    pub fn landed(&mut self, id: Id) {
        // Only the landing of the leap it was pressed in: one whose landing went unheard (a lost
        // packet, a teleport) is over after [`AIR_MOST`], and its attack with it.
        let current = self.airborne(id).is_some();
        let Some(p) = self.players.get_mut(&id) else { return };
        p.air = None;
        if let Some(kind) = p.landing_attack.take().filter(|_| current) {
            self.attack_out_of(id, kind);
        }
    }

    /// In the air, and how (a landing that never came counts after [`AIR_MOST`]).
    pub fn airborne(&self, id: Id) -> Option<Air> {
        self.players.get(&id)?.air.filter(|a| self.clock - a.since < AIR_MOST)
    }

    /// In the air out of a Corey roll.
    pub fn in_corey(&self, id: Id) -> bool {
        self.airborne(id).is_some_and(|a| a.corey)
    }

    /// An attack out of a dodge or a Corey roll, if the hero is free for it and has the breath.
    fn attack_out_of(&mut self, id: Id, kind: StrikeKind) {
        let Some(p) = self.players.get(&id) else { return };
        if p.health == 0 || !matches!(p.action, Action::Ready | Action::Guard) || self.disabled(id) {
            return;
        }
        if p.stamina < self.content.player.exhausted_below {
            self.winded(id);
            return;
        }
        self.unsheathe(id);
        self.start_swing(id, kind);
    }

    pub fn fall_damage(&mut self, id: Id, damage: u32) {
        if self.players.get(&id).is_some_and(|p| p.health > 0) {
            self.hurt_player(id, damage);
        }
    }

    pub(crate) fn hurt_player(&mut self, id: Id, damage: u32) {
        // A frozen hero's ice takes it first; a panic or an incapacitate may end.
        let damage = self.cc_on_damage(id, damage);
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        p.health = p.health.saturating_sub(damage);
        // Hurt is in the fight: no healing in the field for a while.
        if damage > 0 {
            p.combat_at = now;
        }
        let (h, m) = (p.health, p.max_health);
        // A hit spoils a meal or a gather.
        let interrupted = matches!(p.action, Action::Eat { .. } | Action::Gather { .. } | Action::Work { .. });
        self.events.push(Event::Health { id, health: h, max: m });
        if h == 0 {
            self.die(id);
        } else if interrupted {
            self.set_action(id, Action::Ready);
            self.tell(id, "Interrupted!");
        }
    }

    /// Fireballs and the like land where they were aimed.
    pub(crate) fn step_missiles(&mut self) {
        let now = self.clock;
        let (due, keep): (Vec<Missile>, Vec<Missile>) = std::mem::take(&mut self.missiles).into_iter().partition(|m| m.land_at <= now);
        self.missiles = keep;
        for m in due {
            self.unmark(m.mark);
            // A grounding totem near where it lands takes it.
            if self.grounded_at(m.at) {
                continue;
            }
            // Its splash or burst where it comes down.
            if m.look != 0 && m.owner != 0 {
                self.events.push(Event::Missile { caster: m.owner, spell: m.look, target: Target::Point(m.at) });
            }
            let caught: Vec<Id> = self
                .players
                .iter()
                .filter(|(_, p)| p.health > 0 && dist2d(p.pos, m.at) <= m.radius)
                .map(|(id, _)| *id)
                .collect();
            for v in caught {
                let area = Some(Area::Circle { at: m.at, radius: m.radius });
                self.strike_fair(m.owner, v, m.damage, m.from, m.blockable, m.heavy, false, Blow::Light, area, Follow::None);
            }
        }
    }

    // --- falling and getting up -----------------------------------------------------------------

    fn die(&mut self, id: Id) {
        let now = self.clock;
        let pvp = self.content.pvp.clone();
        let secs = self.siege_respawn_secs(id).unwrap_or_else(|| pvp.respawn_secs.unwrap_or(self.content.player.respawn_secs));
        self.cancel_preview(id);
        self.end_ability_action(id);
        self.stances.remove(&id);
        self.death_mark_reset(id);
        self.clear_cc(id);
        // In the ring, a fall ends the run and costs nothing.
        let arena = self.in_arena(id);
        if arena {
            self.leave_arena(id, super::arena::Exit::Fell);
        }
        let totems: Vec<Id> = self.totems.iter().filter(|t| t.owner == id).map(|t| t.mob).collect();
        for t in totems {
            self.remove_totem(t);
        }
        let Some(p) = self.players.get_mut(&id) else { return };
        p.action = Action::Dead { until: now + secs };
        // A fall puts an end to a leap and the attack waiting for its landing.
        p.air = None;
        p.landing_attack = None;
        p.dodge_attack = false;
        p.buffered = None;
        // What they carried for the camp stays where they fell; food and gear go with them. Below,
        // what they carried that's of the mine (plunder, gear found) waits in one sack.
        let below = self.below.is_some();
        let dropped: BTreeMap<usize, u32> = if below || arena || self.siege.is_some() {
            BTreeMap::new()
        } else if pvp.keep_supplies {
            BTreeMap::new()
        } else {
            p.bag.iter().filter(|(i, n)| **n > 0 && self.content.items[**i].kind == ItemKind::Resource).map(|(i, n)| (*i, *n)).collect()
        };
        for i in dropped.keys() {
            p.bag.remove(i);
        }
        if below {
            self.wager_fall();
        }
        let dropped = if below { self.below_drops(id) } else { dropped };
        let Some(p) = self.players.get(&id) else {
            return;
        };
        let (pos, safe, account, name) = (p.pos, p.last_safe, p.account.clone(), p.name.clone());
        self.events.push(Event::Died { id });
        self.events.push(Event::Bags { player: id });
        self.events.push(Event::CastEnd { caster: id });
        if !dropped.is_empty() {
            // Somewhere it can be reached: where they fell, or the last firm ground they stood on.
            let ok = |x: f32, y: f32, z: f32| {
                !self.land.blocked_near(x, y, z) && !self.land.water_near(x, y, z)
            };
            let at = if ok(pos[0], pos[1], pos[2]) && (pos[2] - self.land.height_near(pos[0], pos[1], pos[2])).abs() < 3.0 { pos } else { safe };
            let at = self.ground_near(at[0], at[1], at[2]);
            let uid = self.uid();
            let cid = self.alloc();
            let what: Vec<String> = dropped.iter().map(|(i, n)| format!("{n} {}", self.content.items[*i].name)).collect();
            self.objs.insert(cid, Obj { kind: ObjKind::Cache { uid, owner: account, items: dropped }, pos: at, facing: 0.0 });
            self.events.push(Event::ObjSpawned { id: cid });
            if below {
                self.tell(id, format!("You fall. Your sack ({}) waits where you fell: win it back before you fall again, or it's lost.", what.join(", ")));
            } else {
                self.tell(id, format!("You fall. What you carried ({}) waits where you fell: go back for it.", what.join(", ")));
            }
        }
        // Who felled them, for everyone's kill feed: a hero, or a creature.
        let recent = self.players.get(&id).is_some_and(|p| now - p.combat_at <= self.content.player.combat_secs);
        let last = self.players.get(&id).and_then(|p| p.last_hit_by).filter(|_| recent);
        let hero = last.and_then(|k| self.players.get(&k)).map(|k| k.name.clone());
        let creature = last.and_then(|k| self.mobs.get(&k)).map(|m| self.content.mobs[m.def].name.clone());
        // Felled by a rival in a match: they take a share of the gold.
        if self.is_siege() {
            self.siege_fell(id, last.filter(|k| self.players.contains_key(k)));
        }
        // A creature's word over the hero it felled.
        if let Some((k, sound)) = last.and_then(|k| self.mobs.get(&k).map(|m| (k, self.content.mobs[m.def].slay_sound))) {
            if sound != 0 {
                self.events.push(Event::Sound { id: k, sound });
            }
        }
        match &hero {
            Some(k) => self.events.push(Event::Log(format!("{k} fells {name}."))),
            None => self.events.push(Event::Log(format!("{name} fell"))),
        }
        self.events.push(Event::Kill { killer: hero.or(creature), victim: name });
        self.events.push(Event::Dirty);
        // Whatever was fighting them looks elsewhere (a raider, for its next foe).
        let mut next = Vec::new();
        for (mid, m) in self.mobs.iter_mut() {
            if m.state.target() == Some(id) {
                m.state = mobs::MobState::Return;
                if m.raider {
                    next.push(*mid);
                }
            }
        }
        for m in next {
            self.raider_next(m);
        }
    }

    fn revive(&mut self, id: Id) {
        let c = self.content.clone();
        let (spawn, spawn_facing) = match self.below_revive(id).or_else(|| self.siege_revive_at(id)) {
            Some(at) => at,
            None => (
                self.ground(c.world.spawn[0], c.world.spawn[1]),
                c.world.spawn_facing,
            ),
        };
        let Some(p) = self.players.get_mut(&id) else { return };
        // Below, a hero wakes at the lamp whole.
        let share = if self.below.is_some() || self.siege.is_some() {
            1.0
        } else {
            c.pvp.respawn_health.unwrap_or(c.player.respawn_health)
        };
        p.health = ((p.max_health as f32 * share).round() as u32).max(1);
        p.last_hit_by = None;
        p.foe = None;
        p.riposte_until = -100.0;
        p.stamina = c.player.stamina;
        p.pos = spawn;
        p.facing = spawn_facing;
        p.action = Action::Ready;
        p.air = None;
        p.landing_attack = None;
        p.dodge_attack = false;
        p.kit = Kit::Melee;
        p.combat_at = self.clock;
        p.valor = c.valor.start;
        p.rooted = false;
        let (h, m) = (p.health, p.max_health);
        self.events.push(Event::Revived { player: id });
        self.events.push(Event::Teleport { player: id, pos: spawn, facing: spawn_facing });
        self.events.push(Event::Health { id, health: h, max: m });
        self.events.push(Event::Stamina { player: id });
        self.events.push(Event::Gear { player: id });
        self.events.push(Event::Valor { player: id });
        self.events.push(Event::Auras { id });
        self.update_speed(id);
        self.events.push(Event::Dirty);
    }
}


// --- weapon arts -----------------------------------------------------------------------------------

impl Survival {
    /// A weapon art's blow for this hero: their heavy attack's timing and look, its own wind-up,
    /// reach, arc, clip and stamina.
    pub(crate) fn art_strike(&self, id: Id, ability: usize) -> Strike {
        let d = &self.content.abilities[ability];
        let mut s = self.strike_at(id, StrikeKind::Heavy, 0);
        if d.windup > 0.0 {
            s.windup = d.windup;
        }
        s.range += d.reach;
        if d.arc > 0.0 {
            s.arc = d.arc;
        }
        if d.clip != 0 {
            s.clip = d.clip;
            s.take = 0;
        }
        s.lunge = s.lunge.min(0.6);
        s.stamina = d.stamina;
        s.damage = d.weapon;
        s.stagger = 0.0;
        s.art = Some(ability);
        s
    }

    /// Start a weapon art (its stamina, its swing); an Ambush first steps behind its foe.
    pub(crate) fn start_art(&mut self, id: Id, ability: usize) {
        let d = self.content.abilities[ability].clone();
        let weapon = self.players[&id].weapon;
        let Some(windex) = self.content.weapons.iter().position(|w| w.key == self.content.items[weapon].key) else { return };
        let strike = self.art_strike(id, ability);
        self.spend(id, strike.stamina);
        if let Some(p) = self.players.get_mut(&id) {
            p.combo_next = 0;
            p.combo_until = -100.0;
            p.combat_at = self.clock;
        }
        let windup = strike.windup;
        let (range, arc) = (strike.range, strike.arc);
        self.set_action(id, Action::Swing { t: 0.0, struck: Vec::new(), weapon: windex, strike });
        self.events.push(Event::Swing { attacker: id, victim: None, damage: 0, outcome: Outcome::Miss, anim: true, style: Blow::Heavy });
        // No guard stops it: whoever it's coming for sees the weapon go red.
        if d.unblockable && self.content.pvp.friendly_fire {
            let p = &self.players[&id];
            let (pos, f) = (p.pos, p.facing);
            let near: Vec<Id> = self
                .players
                .iter()
                .filter(|(h, v)| **h != id && v.health > 0 && dist2d(v.pos, pos) <= range + 3.0 && angle_between(f, facing(pos, v.pos)) <= arc / 2.0 + 30.0)
                .map(|(h, _)| *h)
                .collect();
            for h in near {
                self.events.push(Event::Danger { player: h, secs: windup, from: Some(id) });
            }
        }
    }

    /// Where an Ambush steps: just behind the foe under the reticle, facing its back (`None`:
    /// nothing there to step behind).
    pub(crate) fn ambush_spot(&self, id: Id, range: f32) -> Option<(Vec3, f32)> {
        let (o, dir) = self.reticle(id);
        let t = self.trace(o, dir, range + 3.0, 0.9, id);
        let p = &self.players[&id];
        let (pos, lock, aim) = (p.pos, p.lock, self.aim_of(id).0);
        let foe_at = |u: Id| match (self.mobs.get(&u), self.players.get(&u)) {
            (Some(m), _) if m.health > 0 && !self.content.mobs[m.def].inert() => Some((m.pos, m.facing)),
            (_, Some(h)) if u != id && h.health > 0 && self.hostile(id, u) => Some((h.pos, h.facing)),
            _ => None,
        };
        let near = |at: Vec3| dist2d(at, pos) <= range + 1.0 && clear_line(self.land.as_ref(), pos, at);
        // Under the reticle; else the one locked on to; else whoever is nearest the reticle's
        // line within a little of it (a moving foe is hard to keep the reticle on).
        let mut target = t.unit.and_then(foe_at).filter(|(at, _)| near(*at));
        if target.is_none() {
            target = lock.and_then(foe_at).filter(|(at, _)| near(*at));
        }
        if target.is_none() {
            let mut best: Option<(f32, (Vec3, f32))> = None;
            let ids: Vec<Id> = self.mobs.keys().chain(self.players.keys()).copied().collect();
            for u in ids {
                let Some((at, f)) = foe_at(u) else { continue };
                let off = angle_between(aim, facing(pos, at));
                if off <= AMBUSH_CONE && near(at) && best.is_none_or(|(b, _)| off < b) {
                    best = Some((off, (at, f)));
                }
            }
            target = best.map(|(_, t)| t);
        }
        let (at, f) = target?;
        // Behind it; where that's a wall or a drop, round its side.
        for turn in [0.0f32, 0.6, -0.6, 1.2, -1.2] {
            let a = f + std::f32::consts::PI + turn;
            let spot = self.ground_near(at[0] + a.cos() * 1.3, at[1] + a.sin() * 1.3, at[2] + 1.0);
            if !self.land.blocked_near(spot[0], spot[1], spot[2]) && (spot[2] - at[2]).abs() <= 2.0 {
                return Some((spot, facing(spot, at)));
            }
        }
        None
    }

    /// A weapon art's active moment: what's in its reach takes its blow (the weapon's roll times
    /// its share, more from behind), judged as a swing is.
    fn connect_art(&mut self, id: Id, w: &crate::content::WeaponDef, strike: &Strike, ability: usize, struck: &mut Vec<Id>) {
        let d = self.content.abilities[ability].clone();
        let pos = self.players[&id].pos;
        let rewind = self.players[&id].rtt.min(self.content.combat.max_rewind);
        let seen_at = self.clock - rewind;
        let reach = WeaponReach { range: strike.range, arc: strike.arc };
        let f = self.swing_aim_at(id, reach, seen_at);
        let mut hits: Vec<(f32, Id, Vec3, f32)> = Vec::new();
        for (&mid, m) in &self.mobs {
            if m.health == 0 || m.hidden || struck.contains(&mid) || self.content.mobs[m.def].inert() && !self.content.mobs[m.def].dummy {
                continue;
            }
            let at = self.pos_at(mid, seen_at).unwrap_or(m.pos);
            let size = 0.5 * self.content.mobs[m.def].scale;
            let dd = dist2d(pos, at);
            let off = angle_between(f, facing(pos, at));
            if dd > strike.range + size || (off > strike.arc / 2.0 && !(dd < size + 0.4 && off < 90.0)) || !clear_line(self.land.as_ref(), pos, at) {
                continue;
            }
            hits.push((dd, mid, at, m.facing));
        }
        if self.content.pvp.friendly_fire {
            for (&hid, h) in &self.players {
                if hid == id || h.health == 0 || struck.contains(&hid) || !self.hostile(id, hid) {
                    continue;
                }
                let at = self.pos_at(hid, seen_at).unwrap_or(h.pos);
                let dd = dist2d(pos, at);
                let off = angle_between(f, facing(pos, at));
                if dd > strike.range + HERO_SIZE || (off > strike.arc / 2.0 && !(dd < HERO_SIZE + 0.4 && off < 90.0)) || !clear_line(self.land.as_ref(), pos, at) {
                    continue;
                }
                hits.push((dd, hid, at, h.facing));
            }
        }
        hits.sort_by(|a, b| a.0.total_cmp(&b.0));
        // A wide art cleaves (three at least); the rest strike as the weapon does.
        let most = if d.arc > 0.0 { w.targets.max(3) } else { w.targets.max(1) };
        let room = most.saturating_sub(struck.len());
        let might = 1.0 + self.effect_of(&self.players[&id], "might");
        for (_, v, at, vf) in hits.into_iter().take(room) {
            struck.push(v);
            let from_veil = self.players[&id].veil_strike_until > self.clock;
            let behind = d.ambush > 0.0 || from_veil || angle_between(vf, facing(at, pos)) >= BACKSTAB_ARC;
            let mut base = self.roll(w.damage) as f32 * d.weapon * might;
            if behind && d.behind > 0.0 {
                base *= d.behind;
                self.events.push(Event::Flash { player: id, text: "Backstab!".into() });
            }
            if self.players.contains_key(&v) {
                let dmg = self.ability_damage(id, v, ability, base);
                self.strike_fair_as(id, v, dmg, pos, !d.unblockable, true, !d.unblockable, Blow::Heavy, None, Follow::Ability { ability, share: 1.0 }, HitKind::Melee);
            } else {
                self.ability_on_mob(id, v, ability, 1.0, base, pos, HitKind::Melee, d.spell);
            }
            let kit = if d.impact_kit != 0 { d.impact_kit } else if behind && d.behind > 0.0 { KIT_RIPOSTE } else { KIT_SLAM };
            if self.alive(v) {
                self.events.push(Event::Visual { id: v, kit, by: Some(id) });
            }
        }
    }

    /// A guard that took a crushing blow: `crush` times the stamina a heavy blow costs it, and
    /// broken if that's more than there is.
    fn crush_guard(&mut self, victim: Id, damage: u32, crush: f32) {
        let now = self.clock;
        let Some(p) = self.players.get(&victim) else { return };
        let guard = self.guard_of(p).clone();
        let extra = (guard.cost + damage as f32 * guard.cost_per_damage) * 2.0 * (crush - 1.0);
        let broke = p.stamina <= extra;
        self.spend(victim, extra);
        if broke && !self.unstaggerable(victim) {
            self.set_action(victim, Action::Stagger { until: now + guard.break_secs });
            if self.content.combat.riposte_any_stagger {
                self.players.get_mut(&victim).unwrap().riposte_until = now + guard.break_secs;
            }
            self.events.push(Event::Emote { id: victim, emote: mobs::STUN_EMOTE });
            self.tell(victim, "Your guard is crushed!");
        }
    }

    /// Nothing staggers them now: Onslaught or Sanctuary on them, or in the wind-up of an art that
    /// won't be stopped.
    pub(crate) fn unstaggerable(&self, id: Id) -> bool {
        if self.has_cc(id, "onslaught") || self.has_cc(id, "steadfast") {
            return true;
        }
        matches!(self.players.get(&id).map(|p| &p.action), Some(Action::Swing { t, strike, .. })
            if *t < strike.windup + strike.active && strike.art.is_some_and(|a| self.content.abilities[a].poise))
    }

    /// The share of a blow's damage a unit takes: more under a Death Mark, less on hallowed ground.
    pub(crate) fn taken_share(&self, id: Id) -> f32 {
        let now = self.clock;
        let mut k = 1.0;
        for c in self.cc_of(id).iter().filter(|c| c.until > now) {
            match c.kind.as_str() {
                "deathmark" => k *= 1.0 + c.amount,
                "hallowed" => k *= (1.0 - c.amount).max(0.0),
                _ => {}
            }
        }
        k
    }

    /// A unit dies under a Death Mark: its marker's cooldowns are all ready again.
    pub(crate) fn death_mark_reset(&mut self, victim: Id) {
        let now = self.clock;
        let owners: Vec<Id> = self.cc_of(victim).iter().filter(|c| c.kind == "deathmark" && c.until > now).map(|c| c.from).collect();
        for o in owners {
            if let Some(p) = self.players.get_mut(&o) {
                p.cooldowns.clear();
                self.events.push(Event::Abilities { player: o });
                self.events.push(Event::Flash { player: o, text: "Death Mark: every cooldown ready".into() });
            }
        }
    }
}

/// How long after the last blow traded the one traded with is still the foe a lock-on prefers.
const LOCK_FOE_MEMORY: f32 = 10.0;
/// A locked swing at a hero turns toward them only this far (degrees): a hero rolling past is
/// out of it, as with a creature's blow. Creatures, slower to slip round, are found up to 90.
const LOCK_SNAP_HERO: f32 = 50.0;
/// How far off the reticle's line a foe may be for a step behind it to find it (degrees).
const AMBUSH_CONE: f32 = 20.0;
/// How wide a hero is, for blows and arrows.
const HERO_SIZE: f32 = 0.4;

/// A swing's reach: how far, how wide (degrees).
#[derive(Clone, Copy)]
pub(crate) struct WeaponReach {
    pub range: f32,
    pub arc: f32,
}

/// The reactions (`AnimationData`): a flinch, a heavy blow's, a stagger's reel, the shield taking
/// a blow.
pub const WOUND_ANIM: u16 = 9;
pub const CRIT_ANIM: u16 = 10;
pub const STUN_ANIM: u16 = 14;
pub const BLOCK_ANIM: u16 = 24;
/// The impacts' looks (`SpellVisualKit`): Heroic Strike's flash for a heavy blow, Shield Bash's for
/// a shove or a broken guard, Disarm's clash for a parry, Backstab's for a riposte.
pub const KIT_HEAVY: u32 = 437;
/// Slam's impact, the other heavy flash; Cleave's, for a chain's last blow; a dust nova for a
/// body hitting the ground.
pub const KIT_SLAM: u32 = 1666;
pub const KIT_CLEAVE: u32 = 507;
pub const KIT_DUST: u32 = 5649;
/// Knocked to the ground and up again, and the parry's own sweep.
pub const KNOCKDOWN_ANIM: u16 = 121;
/// A blow this far off a creature's facing (degrees) is from behind.
const BACKSTAB_ARC: f32 = 120.0;
/// A creature remembers the backstabs it took this long.
const BACKSTAB_MEMORY: f32 = 8.0;
pub const PARRY_ANIM: u16 = 21;
pub const KIT_BASH: u32 = 83;
pub const KIT_PARRY: u32 = 555;
pub const KIT_RIPOSTE: u32 = 330;
/// How far a lock-on reaches.
const LOCK_RANGE: f32 = 30.0;
/// How far off the facing Tab looks for another foe to switch to (degrees).
const SWITCH_ARC: f32 = 110.0;
/// A lock holds until its target is this many times the lock's reach away.
const LOCK_HOLD: f32 = 1.3;

/// The Rogue's Sprint (its whoosh and trail), which a sprint no longer casts: kept so the tests
/// can say it doesn't.
pub const SPRINT_SPELL: u32 = 2983;

/// The meal's cast look (eating), and the burst when it's done.
pub const EAT_SPELL: u32 = 433;
pub const EAT_DONE_SPELL: u32 = 2050;

/// A hero's health now (0 when they're gone).
fn p_health(s: &Survival, id: Id) -> u32 {
    s.players.get(&id).map_or(0, |p| p.health)
}

impl Survival {
    /// How many heroes are fighting a creature now (struck it or were struck by it lately).
    pub(crate) fn heroes_on(&self, mid: Id) -> usize {
        let now = self.clock;
        self.players.values().filter(|p| p.health > 0 && p.foe.is_some_and(|(f, at)| f == mid && now - at <= 6.0)).count()
    }

    #[cfg(test)]
    pub(crate) fn backstab_bonus_for_test(&mut self, mid: Id) -> f32 {
        self.backstab_bonus(mid, 2.0)
    }

    /// A backstab's bonus on a creature, less for each one it took lately.
    fn backstab_bonus(&mut self, mid: Id, stab: f32) -> f32 {
        let now = self.clock;
        let heroes = self.heroes_on(mid).max(1);
        let Some(m) = self.mobs.get_mut(&mid) else { return stab };
        m.backstabs.retain(|t| now - *t <= BACKSTAB_MEMORY);
        let n = m.backstabs.len() as i32;
        // Each lately takes a quarter of the bonus, a third with three heroes or more.
        let keep = if heroes >= 3 { 0.66f32 } else { 0.75 };
        1.0 + (stab - 1.0).max(0.0) * keep.powi(n)
    }

    /// Stabbed from behind: remembered; and a champion with more than one hero on it may turn on
    /// the one at its back (the more heroes, the likelier), so no one strikes it from behind for
    /// long while another holds it.
    fn backstabbed(&mut self, mid: Id, by: Id) {
        let now = self.clock;
        let heroes = self.heroes_on(mid);
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        m.backstabs.push(now);
        let boss = self.content.mobs[m.def].boss || self.content.mobs[m.def].posture > 0;
        let on_other = m.state.target().is_some_and(|t| t != by);
        if !boss || !on_other || heroes < 2 || m.health == 0 {
            return;
        }
        let chance = (0.3 * (heroes - 1) as f32 + 0.1 * m.backstabs.len() as f32).min(0.9);
        if m.turn_on.is_none() && self.rng.gen::<f32>() < chance {
            // It wheels round on them as soon as its blow in hand is done.
            self.mobs.get_mut(&mid).unwrap().turn_on = Some(by);
            self.events.push(Event::Flash { player: by, text: "It turns on you!".into() });
        }
    }

    /// A heavy blow's flash, one of two so they don't all look alike.
    pub(crate) fn heavy_kit(&mut self) -> u32 {
        if self.rng.gen::<bool>() {
            KIT_HEAVY
        } else {
            KIT_SLAM
        }
    }
}

impl Survival {
    /// The next usable item in the bags becomes the one R uses.
    pub(crate) fn cycle_quick(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        let usable = self.usable_items(p);
        if usable.is_empty() {
            self.tell(id, "Nothing to use in your bags.");
            return;
        }
        let now = self.quick_item(p);
        let next = match now.and_then(|q| usable.iter().position(|i| *i == q)) {
            Some(k) => usable[(k + 1) % usable.len()],
            None => usable[0],
        };
        self.players.get_mut(&id).unwrap().quick = Some(next);
        self.events.push(Event::Bags { player: id });
    }

    /// Use a particular item from the bags (a right-click on it): it becomes the quick item.
    pub fn use_item(&mut self, id: Id, item: usize) {
        if let Some(p) = self.players.get_mut(&id) {
            if p.bag.get(&item).copied().unwrap_or(0) > 0 && self.content.items[item].kind == ItemKind::Food {
                p.quick = Some(item);
            }
        }
        self.input(id, Input::Use);
    }
}
