//! The Rogue's core (docs/rogue-roles.md, "The core: footwork and two blades"): a hero created as a
//! Rogue fights with a blade in each hand and has no guard, no block and no parry.
//!
//! **Two hands.** The right click swings the off hand (dual wield's `off_press`), whatever is in
//! it: an empty off hand jabs for half the main hand's blow (`[rogue] jab`). A Rogue can't hold a
//! shield, a two-hander, a staff or an axe (`Content::rogue_can_hold`); an old save that held one
//! loads with it in the bags ([`rogue_hands`]). A card that blocks or parries refuses.
//!
//! **The perfect dodge.** A blow that lands 0.04 to `[dodge] perfect_to` sec into a Rogue's dodge
//! (pressed in its last moment, judged a round trip late as every dodge is) pays: more stamina
//! back, a parry's Valor, a share of a boss's or champion's posture, and the **Shadow Cut**: for
//! `[rogue] cut_secs`, either attack steps behind the one who swung and strikes its back. It reads
//! with no floor marker: the body flickers (the Stealth look), Vanish's puff and sound where the
//! blow fell, and a short stop on the Rogue's own screen (the `perfect_dodge` beat).
//!
//! **Footwork in the run.** A dodge pressed in the Rogue's own wind-up calls the blow off, its
//! stamina back (the dodge-feint); a dodge cuts an off-hand blow short at once; a dodge between
//! blows keeps the two-blade run, and the Shadow Cut counts as its next hand.
//!
//! None of it is any other class's: a Warrior with two blades has the off hand and the run, not
//! the footwork.
//!
//! **Shadow** (docs/rogue-roles.md, "Shadow: get behind, and be gone"): the first of the three
//! trees, the Rogue column's first choice on every key. Its trait, **Shrouded Step**, is on while
//! 3 of a Rogue's 5 keys hold its cards: a step that lands behind an enemy (Shadowstep, Blinding
//! Step, Ambush, the Shadow Cut, Cloak and Dagger) hides the Rogue for a moment from every creature
//! but a boss. **Distract** turns the enemies where its pouch lands to face it (a boss only
//! glances), **Sap** drops one from behind before its fight, and **Cloak and Dagger** steps behind
//! an enemy before every blow. None of it resets a boss (#75): a boss sees through the shroud,
//! glances at a Distract and is never Sapped.
//!
//! **Venom** (docs/rogue-roles.md, "Venom: build it on, cash it in"): the second tree, the
//! column's second choice. Venom is a coat on a creature: stacks up to `[rogue] venom_most`, each
//! ticking `venom_dps` a second, for `venom_secs` from the latest. Its trait, **Coated Blade**,
//! lays it with every off-hand hit (the two-blade finisher too) and every Fan of Knives knife.
//! **Eviscerate** spends it from any side, **Rupture** lets every hero's blow lay it, **Garrote**
//! breaks a cast from behind and silences, **Feint** hands the enemies on the Rogue to a friend,
//! **Leeching Venom** heals the Rogue by its ticks and **Choking Cloud** slows wind-ups and lays it.
//! It reads with no floor marker: the poison's look on the body (a deeper one once ripe), a drip
//! and its number at every tick. It is apart from a Bazil's Shiv's own `poison`: a dagger's
//! poison and the Rogue's Venom stack side by side, and Eviscerate spends only Venom.

use super::*;
use crate::land::clear_line;

/// What a Rogue's footwork holds between ticks.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Rogue {
    /// The Shadow Cut is open until then...
    pub cut_until: f32,
    /// ...behind this one (who swung).
    pub cut_on: Option<Id>,
    /// The perfect dodge's flicker shows on the body until then (0: none showing).
    pub flicker_until: f32,
    /// Shrouded Step: hidden from creatures (not bosses) until then (0: not shrouded).
    pub shroud_until: f32,
    /// Cloak and Dagger: the steps it has left, and the enemies struck from them so far.
    pub cloak_steps: u32,
    pub cloak_struck: Vec<Id>,
}

/// What a Rogue's Shadow keeps on a creature.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OnMob {
    /// Where a Distract turned it to (while its `distract` effect lasts).
    pub distract_at: Option<Vec3>,
    /// A boss: no glance at a Distract before then.
    pub glance_ready: f32,
    /// Garroted: it starts no cast until then (its silence on a creature).
    pub silenced_until: f32,
    /// Feinted while winding up a blow at a Rogue: once the blow is done, it loses interest in
    /// that Rogue for this long (or turns to another hero).
    pub feint_from: Option<(Id, f32)>,
}

/// The Rogue's class key.
pub const ROGUE: &str = "rogue";

/// A Rogue's hands put right, as a save is loaded: what a Rogue can't hold goes into the bags. A
/// main hand that can't be a Rogue's takes the Rogue's start blade; an off hand that held a shield
/// or a blade a Rogue can't use is left empty (it jabs). Once put right, it stays: the save then
/// holds the blade and an empty off hand.
pub(crate) fn rogue_hands(c: &crate::content::Content, p: &mut Player) {
    if p.class != ROGUE {
        return;
    }
    if c.rogue_can_hold(p.weapon).is_err() {
        if let Some(knife) = c.item(&c.rogue.weapon) {
            *p.bag.entry(p.weapon).or_default() += 1;
            p.weapon = knife;
        }
    }
    if let Some(o) = p.off_weapon.filter(|o| c.rogue_can_hold(*o).is_err()) {
        *p.bag.entry(o).or_default() += 1;
        p.off_weapon = None;
        p.off_empty = true;
    }
    // A shield held (no blade in the off hand, and not left empty): into the bags.
    if p.off_weapon.is_none() && !p.off_empty {
        *p.bag.entry(p.shield).or_default() += 1;
        p.off_empty = true;
    }
}

impl Survival {
    /// A hero created as a Rogue.
    pub fn is_rogue(&self, p: &Player) -> bool {
        p.class == ROGUE
    }

    /// Fights with two hands: a blade in the off hand (dual wield), or a Rogue's (an empty off
    /// hand jabs). The right click is then the off hand, never a guard.
    pub fn two_hands(&self, p: &Player) -> bool {
        self.off_weapon_of(p).is_some() || self.is_rogue(p)
    }

    /// Whether this hero may take up this item, and if not, the words why (a Rogue's hands).
    pub(crate) fn can_hold(&self, id: Id, item: usize) -> Result<(), String> {
        match self.players.get(&id) {
            Some(p) if self.is_rogue(p) => self.content.rogue_can_hold(item).map_err(String::from),
            _ => Ok(()),
        }
    }

    /// A card this hero can't use for their hands: one that blocks or parries, on a Rogue; one that
    /// needs two blades, with one.
    pub(crate) fn hands_refuse_card(&self, id: Id, d: &crate::content::AbilityDef) -> Option<String> {
        let p = self.players.get(&id)?;
        if self.is_rogue(p) {
            if let Some(k) = d.counter.as_ref().filter(|k| k.block) {
                // A brace held toward the front blocks (Retaliation, Shield Wall); one from
                // anywhere turns the blow aside, a parry (Riposte).
                return Some(if k.front > 0.0 { "Rogues can't block." } else { "Rogues can't parry." }.to_string());
            }
        }
        if d.needs_two_blades && self.off_weapon_of(p).is_none() {
            return Some(format!("{} requires a weapon in each hand", d.name));
        }
        None
    }

    /// A Rogue's dodge took this blow in its perfect moment.
    pub(crate) fn dodged_perfectly(&self, victim: Id, doing: &Action) -> bool {
        let d = &self.content.dodge;
        d.perfect_to > 0.0 && self.players.get(&victim).is_some_and(|p| self.is_rogue(p)) && matches!(doing, Action::Dodge { t, .. } if *t >= d.evade_from && *t <= d.perfect_to)
    }

    /// What a perfect dodge pays besides the breath and Valor (`strike_player`): a share of the
    /// striker's posture, the Shadow Cut opened behind it, and how it shows.
    pub(crate) fn perfect_dodge(&mut self, victim: Id, attacker: Id) {
        let now = self.clock;
        let r = self.content.rogue.clone();
        let share = self.content.dodge.perfect_posture;
        // A boss's or champion's posture fills; nothing reels from one dodge.
        if let Some(m) = self.mobs.get(&attacker).filter(|m| m.health > 0) {
            if share > 0.0 && self.content.mobs[m.def].posture > 0 {
                let fill = (self.posture_limit(attacker) as f32 * share).round().max(1.0) as u32;
                self.build_posture(attacker, victim, fill);
            }
        }
        let alive = self.mobs.get(&attacker).is_some_and(|m| m.health > 0) || self.players.get(&attacker).is_some_and(|h| h.health > 0 && attacker != victim);
        let Some(p) = self.players.get_mut(&victim) else { return };
        let at = p.pos;
        if alive {
            p.rogue.cut_until = now + r.cut_secs;
            p.rogue.cut_on = Some(attacker);
        }
        // No word: the body flickers, a puff of smoke where the blow fell, Vanish's sound, and a
        // short stop on their own screen.
        if r.flicker_aura != 0 && r.flicker_secs > 0.0 {
            p.rogue.flicker_until = now + r.flicker_secs;
            self.events.push(Event::Auras { id: victim });
        }
        if r.puff != 0 {
            self.events.push(Event::Missile { caster: victim, spell: r.puff, target: Target::Point(at) });
        }
        self.cue(Some(victim), victim, 0, cues::SOUND_PERFECT_DODGE, cues::BEAT_PERFECT_DODGE);
    }

    /// The perfect dodge's flicker on the body, and Shrouded Step's veil, while they last (for
    /// `auras_of`).
    pub(crate) fn rogue_auras(&self, p: &Player) -> Vec<u32> {
        let r = &self.content.rogue;
        let mut out = Vec::new();
        if r.flicker_aura != 0 && p.rogue.flicker_until > self.clock {
            out.push(r.flicker_aura);
        }
        if r.shroud_aura != 0 && p.rogue.shroud_until > self.clock {
            out.push(r.shroud_aura);
        }
        out
    }

    /// Each tick: the flicker and the shroud gone from the body once their time is up.
    pub(crate) fn step_rogue(&mut self, id: Id) {
        let now = self.clock;
        let Some(p) = self.players.get_mut(&id) else { return };
        let mut gone = false;
        if p.rogue.flicker_until > 0.0 && now >= p.rogue.flicker_until {
            p.rogue.flicker_until = 0.0;
            gone = true;
        }
        if p.rogue.shroud_until > 0.0 && now >= p.rogue.shroud_until {
            p.rogue.shroud_until = 0.0;
            gone = true;
        }
        if gone {
            self.events.push(Event::Auras { id });
        }
    }

    /// The Shadow Cut: an attack while it's open steps the Rogue behind the one who swung (a puff
    /// at both ends) and strikes as that hand's next blow, from behind. False (and the press goes
    /// on as any other) when it isn't open, the Rogue isn't free to strike, or there's no spot
    /// behind within reach (a wall, a drop): no step, and the cut is spent.
    pub(crate) fn shadow_cut(&mut self, id: Id, kind: StrikeKind) -> bool {
        let now = self.clock;
        let c = &self.content;
        let Some(p) = self.players.get(&id) else { return false };
        if !self.is_rogue(p) || p.rogue.cut_until < now {
            return false;
        }
        let Some(on) = p.rogue.cut_on else { return false };
        // Out of the dodge, standing, or late in a blow's recovery (as a chain's next blow).
        let free = match &p.action {
            Action::Ready | Action::Dodge { .. } => true,
            Action::Swing { t, strike, .. } => strike.art.is_none() && *t >= strike.windup + strike.active + strike.recovery * c.combat.chain_after,
            _ => false,
        };
        if !free || (p.stamina < c.player.exhausted_below && !self.tireless(id)) {
            return false;
        }
        let (from, reach) = (p.pos, c.rogue.cut_reach);
        let puff = c.rogue.puff;
        let foe = match (self.mobs.get(&on), self.players.get(&on)) {
            (Some(m), _) if m.health > 0 => Some((m.pos, m.facing)),
            (_, Some(h)) if on != id && h.health > 0 => Some((h.pos, h.facing)),
            _ => None,
        };
        // One press: open or not, it's spent.
        let p = self.players.get_mut(&id).unwrap();
        p.rogue.cut_until = -100.0;
        p.rogue.cut_on = None;
        let Some((at, f)) = foe else { return false };
        // Squarely behind, or a little to a side (still in its back): never round its flank.
        let Some((spot, face)) = self.spot_behind(at, f, &[0.0, 0.5, -0.5]) else { return false };
        if dist2d(from, spot) > reach || !clear_line(self.land.as_ref(), from, spot) {
            return false;
        }
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(from) });
        }
        self.set_down(id, spot, face);
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(spot) });
        }
        self.shrouded_step(id);
        // It counts as the run's next hand, whichever button it was.
        let p = self.players.get_mut(&id).unwrap();
        if p.dual_run > 0 {
            p.dual_last_off = kind != StrikeKind::Off;
        }
        p.dodge_attack = false;
        self.set_action(id, Action::Ready);
        self.unsheathe(id);
        self.start_swing(id, kind);
        true
    }

    /// A Rogue's dodge keeps the two-blade run going: its window counts from the dodge's end, as
    /// from a blow's.
    pub(crate) fn run_through_dodge(&mut self, id: Id) {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return };
        if !self.is_rogue(p) {
            return;
        }
        let p = self.players.get_mut(&id).unwrap();
        if p.dual_run > 0 && now <= p.combo_until + DUAL_WINDOW {
            p.combo_until = f32::INFINITY;
        }
    }

    /// A Rogue's dodge-feint: the dodge pressed while their own blow (not a card's) still winds up
    /// calls it off, its stamina back, as the guard's feint does; the run it was part of ends.
    /// True if one was called off (the dodge then goes).
    pub(crate) fn dodge_feint(&mut self, id: Id) -> bool {
        let Some(p) = self.players.get(&id) else { return false };
        if !self.is_rogue(p) {
            return false;
        }
        let Action::Swing { t, strike, .. } = &p.action else { return false };
        if *t >= strike.windup || strike.art.is_some() {
            return false;
        }
        let (refund, most, now) = (strike.stamina, self.content.player.stamina, self.clock);
        let p = self.players.get_mut(&id).unwrap();
        p.stamina = (p.stamina + refund).min(most);
        p.combo_next = 0;
        p.off_next = 0;
        p.combo_until = now;
        p.dual_run = 0;
        self.events.push(Event::Stamina { player: id });
        self.events.push(Event::ClipStop { id });
        self.set_action(id, Action::Ready);
        true
    }
}

// --- Shadow -----------------------------------------------------------------------------------------

/// The Rogue column's tree of a card: its `choice` (1 Shadow, 2 Venom).
pub const SHADOW: u32 = 1;

impl Survival {
    /// A Rogue tree's trait is on: a hero created as a Rogue with `[rogue] trait_keys` of their 5
    /// keys on that tree's cards (a Rogue card of that `choice`). Never another class's, whatever
    /// its keys hold.
    pub fn rogue_trait(&self, p: &Player, tree: u32) -> bool {
        if !self.is_rogue(p) {
            return false;
        }
        let on = p.loadout.values().filter_map(|k| self.content.ability(k)).map(|a| &self.content.abilities[a]).filter(|d| d.school == ROGUE && d.choice == tree).count();
        on >= self.content.rogue.trait_keys
    }

    /// Shrouded Step (Shadow's trait): a step that just landed a Rogue behind an enemy hides them
    /// for `[rogue] shroud_secs` from every creature but a boss, and the body shows it.
    pub(crate) fn shrouded_step(&mut self, id: Id) {
        let Some(p) = self.players.get(&id) else { return };
        if !self.rogue_trait(p, SHADOW) {
            return;
        }
        let until = self.clock + self.content.rogue.shroud_secs;
        self.players.get_mut(&id).unwrap().rogue.shroud_until = until;
        self.events.push(Event::Auras { id });
    }

    /// Hidden by Shrouded Step now.
    pub fn shrouded(&self, id: Id) -> bool {
        self.players.get(&id).is_some_and(|p| p.rogue.shroud_until > self.clock)
    }

    /// A Distract's pouch landed (`fx_from`, where): this enemy turns to face it for `secs`. A boss
    /// only glances (`[rogue] glance_secs`), at most once every `glance_every`, and never in the
    /// middle of a string. A hero's facing is their own: no effect. A friendly creature, a totem:
    /// none either.
    pub(crate) fn distract(&mut self, owner: Id, victim: Id, secs: f32, source: &str) {
        let now = self.clock;
        let at = self.fx_from.or_else(|| self.unit_pos(owner));
        let (Some(at), Some(m)) = (at, self.mobs.get(&victim)) else { return };
        let d = &self.content.mobs[m.def];
        if m.health == 0 || m.friendly || d.totem || d.inert() {
            return;
        }
        let boss = d.boss;
        let secs = if boss {
            let mid_string = m.pattern.is_some() || matches!(m.state, mobs::MobState::Windup { .. });
            if now < m.rogue.glance_ready || mid_string {
                return;
            }
            self.content.rogue.glance_secs
        } else {
            secs
        };
        if !self.apply_cc(victim, owner, "distract", secs, 0.0, 0.0, source) {
            return;
        }
        let every = self.content.rogue.glance_every;
        let m = self.mobs.get_mut(&victim).unwrap();
        m.rogue.distract_at = Some(at);
        if boss {
            m.rogue.glance_ready = now + every;
        }
        // The turn is the tell: at once, not on its next thought (one in a wind-up turns once
        // its blow is done).
        if !matches!(m.state, mobs::MobState::Windup { .. }) {
            self.face_point(victim, at);
        }
    }

    /// A creature turned to `at` where it stands, the clients told at once.
    fn face_point(&mut self, id: Id, at: Vec3) {
        let now = self.clock;
        let Some(m) = self.mobs.get_mut(&id) else { return };
        let face = facing(m.pos, at);
        let turned = angle_between(face, m.facing) > 1.0;
        m.facing = face;
        if !m.path.is_empty() {
            self.halt(id);
        } else if turned {
            let pos = m.pos;
            m.face_sent = Some((face, now));
            self.events.push(Event::MobStop { id, pos, facing: face });
        }
    }

    /// A creature that doesn't think this tick for a Rogue's Shadow (true): distracted, it stands
    /// facing where the pouch landed; its foe shrouded (and it no boss), it stands where it is.
    /// Either way it keeps its foe, and a blow it is winding up still lands first.
    pub(crate) fn step_shadowed(&mut self, id: Id) -> bool {
        let Some(m) = self.mobs.get(&id) else { return false };
        if matches!(m.state, mobs::MobState::Windup { .. }) {
            return false;
        }
        // Feinted mid-blow: the blow done, it looks away (Venom's Feint).
        if m.rogue.feint_from.is_some() {
            self.feint_after_blow(id);
        }
        let Some(m) = self.mobs.get(&id) else { return false };
        if self.has_cc(id, "distract") {
            if let Some(at) = m.rogue.distract_at {
                self.face_point(id, at);
                return true;
            }
        }
        let boss = self.content.mobs[m.def].boss;
        if !boss && matches!(m.state.foe(), Some(mobs::Foe::Hero(h)) if self.shrouded(h)) {
            if !m.path.is_empty() {
                self.halt(id);
            }
            return true;
        }
        false
    }

    /// A distracted creature struck from its front (by anyone, anything) looks back: the
    /// distraction ends. A blow in its back leaves it be.
    pub(crate) fn distract_struck(&mut self, mid: Id, by: Id) {
        if !self.has_cc(mid, "distract") {
            return;
        }
        let Some(from) = self.unit_pos(by) else { return };
        if by == mid || self.from_behind(mid, from) {
            return;
        }
        self.cleanse(mid, &["distract"]);
        if let Some(m) = self.mobs.get_mut(&mid) {
            m.rogue.distract_at = None;
        }
    }

    /// Cloak and Dagger begins: its steps counted afresh.
    pub(crate) fn cloak_begins(&mut self, id: Id, steps: f32) {
        if let Some(p) = self.players.get_mut(&id) {
            p.rogue.cloak_steps = steps.round().max(0.0) as u32;
            p.rogue.cloak_struck.clear();
        }
    }

    /// Cloak and Dagger: an attack while it lasts first steps the hero behind the nearest enemy
    /// within `[rogue] cloak_reach` (one not yet struck in it before any other), a puff at both
    /// ends, then strikes as that hand's blow. False (the press goes on as any other) when it isn't
    /// on, no steps are left, the hero isn't free to strike, or no back is in reach.
    pub(crate) fn cloak_step(&mut self, id: Id, kind: StrikeKind) -> bool {
        let c = &self.content;
        let Some(p) = self.players.get(&id) else { return false };
        if p.rogue.cloak_steps == 0 || !self.has_cc(id, "cloak") {
            return false;
        }
        let free = match &p.action {
            Action::Ready | Action::Dodge { .. } => true,
            Action::Swing { t, strike, .. } => strike.art.is_none() && *t >= strike.windup + strike.active + strike.recovery * c.combat.chain_after,
            _ => false,
        };
        if !free || (p.stamina < c.player.exhausted_below && !self.tireless(id)) {
            return false;
        }
        let (from, reach, puff) = (p.pos, c.rogue.cloak_reach, c.rogue.puff);
        let struck = p.rogue.cloak_struck.clone();
        let mut foes: Vec<(bool, f32, Id, Vec3, f32)> = Vec::new();
        for (&mid, m) in &self.mobs {
            let d = &c.mobs[m.def];
            if m.health == 0 || m.hidden || m.friendly || d.totem || (d.inert() && !d.dummy) || self.own_totem(mid, id) {
                continue;
            }
            foes.push((struck.contains(&mid), dist2d(from, m.pos), mid, m.pos, m.facing));
        }
        if self.heroes_clash() {
            for (&h, x) in &self.players {
                if h != id && x.health > 0 && self.hostile(id, h) {
                    foes.push((struck.contains(&h), dist2d(from, x.pos), h, x.pos, x.facing));
                }
            }
        }
        foes.retain(|f| f.1 <= reach && clear_line(self.land.as_ref(), from, f.3));
        // One not yet struck in it first, then the nearest.
        foes.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.total_cmp(&b.1)));
        let Some((on, spot, face)) = foes.iter().find_map(|(_, _, u, at, f)| self.spot_behind(*at, *f, &[0.0, 0.5, -0.5]).filter(|(spot, _)| clear_line(self.land.as_ref(), from, *spot)).map(|(spot, face)| (*u, spot, face))) else {
            return false;
        };
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(from) });
        }
        self.set_down(id, spot, face);
        if puff != 0 {
            self.events.push(Event::Missile { caster: id, spell: puff, target: Target::Point(spot) });
        }
        let p = self.players.get_mut(&id).unwrap();
        p.rogue.cloak_steps -= 1;
        if !p.rogue.cloak_struck.contains(&on) {
            p.rogue.cloak_struck.push(on);
        }
        // (Out of a dodge it's this hand's blow, not the roll's attack.)
        p.dodge_attack = false;
        self.shrouded_step(id);
        self.set_action(id, Action::Ready);
        self.unsheathe(id);
        self.start_swing(id, kind);
        true
    }
}

// --- Venom ------------------------------------------------------------------------------------------

/// The Rogue column's second choice: Venom's cards.
pub const VENOM: u32 = 2;

impl Survival {
    /// The Venom on a unit now (0: none).
    pub fn venom_on(&self, id: Id) -> u32 {
        let now = self.clock;
        self.cc_of(id).iter().find(|c| c.kind == "venom" && c.until > now).map_or(0, |c| c.amount.round() as u32)
    }

    /// Lay `n` Venom on a creature for hero `owner` (up to the most, its time from now). Ripe, at
    /// the most: the deeper look, and a burst of it as it turns. Heroes take none (a duel's blows
    /// land a round trip late, and an 8 sec coat on a hero is outside the PvP budget).
    pub(crate) fn lay_venom(&mut self, owner: Id, mid: Id, n: u32) {
        let r = &self.content.rogue;
        let (most, secs, aura, ripe_aura, drip) = (r.venom_most, r.venom_secs, r.venom_aura, r.ripe_aura, r.venom_drip);
        let Some(m) = self.mobs.get(&mid) else { return };
        let d = &self.content.mobs[m.def];
        if n == 0 || most == 0 || m.health == 0 || d.totem || (d.inert() && !d.dummy) {
            return;
        }
        let have = self.venom_on(mid);
        let now_n = (have + n).min(most);
        let ripe = now_n >= most;
        let look = if ripe { ripe_aura } else { aura };
        // (A refresh keeps the higher count: `apply_cc` keeps an effect's larger amount.)
        if self.apply_cc_look(mid, owner, "venom", secs, now_n as f32, 0.0, "Venom", look) && ripe && have < most && drip != 0 {
            self.events.push(Event::Visual { id: mid, kit: drip, by: Some(owner) });
        }
    }

    /// All the Venom on a creature, used up (Eviscerate): how much there was. Its look goes out.
    pub(crate) fn take_venom(&mut self, mid: Id) -> u32 {
        let n = self.venom_on(mid);
        if n > 0 {
            self.cleanse(mid, &["venom"]);
        }
        n
    }

    /// A Venom tick on a creature: its damage (the stacks' share, its fractions carried over by
    /// `step_cc`), a drip of poison on the body and its number; with Leeching Venom on its Rogue,
    /// as much healed.
    pub(crate) fn venom_tick(&mut self, mid: Id, from: Id, damage: u32) {
        if damage == 0 || !self.mobs.get(&mid).is_some_and(|m| m.health > 0) {
            return;
        }
        let drip = self.content.rogue.venom_drip;
        self.events.push(Event::Swing { attacker: from, victim: Some(mid), damage, outcome: Outcome::Hit, anim: false, style: Blow::Dot });
        if drip != 0 {
            self.events.push(Event::Visual { id: mid, kit: drip, by: None });
        }
        self.hurt_mob_as(mid, from, damage, "Venom");
        if self.players.get(&from).is_some_and(|p| p.health > 0) && self.has_cc(from, "leech") {
            self.heal_hero_by(from, damage, from, "Leeching Venom");
        }
    }

    /// A hero's blow landed on a creature: Venom for it. Under Rupture, any hero's blow lays 1 for
    /// the Rupture's Rogue; otherwise a Rogue with Coated Blade lays 1 with the off hand (and the
    /// two-blade finisher). One a blow, at most.
    pub(crate) fn blow_coats(&mut self, hero: Id, mid: Id, off_hand: bool) {
        let now = self.clock;
        if !self.mobs.get(&mid).is_some_and(|m| m.health > 0) {
            return;
        }
        if let Some(owner) = self.cc_of(mid).iter().find(|c| c.kind == "rupture" && c.until > now).map(|c| c.from) {
            self.lay_venom(owner, mid, 1);
            return;
        }
        if off_hand && self.players.get(&hero).is_some_and(|p| self.rogue_trait(p, VENOM)) {
            self.lay_venom(hero, mid, 1);
        }
    }

    /// An ability's hit that lays Venom with the trait (Fan of Knives' knives).
    pub(crate) fn ability_coats(&mut self, owner: Id, victim: Id, ability: usize) {
        let n = self.content.abilities[ability].coats.round() as u32;
        if n > 0 && self.players.get(&owner).is_some_and(|p| self.rogue_trait(p, VENOM)) {
            self.lay_venom(owner, victim, n);
        }
    }

    /// Garrote's silence: a hero can't use abilities; a creature starts no cast for its time (a
    /// boss shrugs it off, as it does any silence). The silence's look shows either way.
    pub(crate) fn garrote(&mut self, owner: Id, victim: Id, secs: f32, source: &str) {
        if !self.apply_cc(victim, owner, "silence", secs, 0.0, 0.0, source) {
            return;
        }
        let until = self.clock + secs;
        if let Some(m) = self.mobs.get_mut(&victim) {
            m.rogue.silenced_until = m.rogue.silenced_until.max(until);
        }
    }

    /// A creature Garrote silenced: it starts no cast (`caster_waits`); a creature that fights up
    /// close fights on.
    pub(crate) fn garroted(&self, mid: Id) -> bool {
        self.mobs.get(&mid).is_some_and(|m| m.rogue.silenced_until > self.clock && m.health > 0)
    }

    /// From behind, a cast an enemy is winding up breaks: it reels `secs` and its cast ends. A
    /// boss's too (it loses only the cast: Garrote's silence doesn't take on a boss).
    pub(crate) fn break_cast(&mut self, mid: Id, secs: f32) -> bool {
        let casting = self.mobs.get(&mid).is_some_and(|m| m.health > 0 && matches!(m.state, mobs::MobState::Windup { .. })) && self.attack_now(mid).ranged;
        if casting {
            self.stagger_mob(mid, secs, false);
        }
        casting
    }

    /// A creature that is a boss.
    fn boss_of(&self, mid: Id) -> bool {
        self.mobs.get(&mid).is_some_and(|m| self.content.mobs[m.def].boss)
    }

    /// A creature Choking Cloud slows: its wind-ups this much slower (a boss's not at all, as
    /// Cowed's).
    pub(crate) fn choked_slow(&self, mid: Id) -> f32 {
        if self.boss_of(mid) {
            return 0.0;
        }
        let now = self.clock;
        self.cc_of(mid).iter().filter(|c| c.kind == "choked" && c.until > now).map(|c| c.amount).fold(0.0f32, f32::max)
    }

    /// Feint: every creature within `reach` yd on the Rogue loses interest for `secs`, or turns to
    /// another hero near it. One winding up a blow finishes it first. A boss halves the Rogue's
    /// threat instead.
    pub(crate) fn feint(&mut self, id: Id, secs: f32, reach: f32) {
        let Some(at) = self.players.get(&id).map(|p| p.pos) else { return };
        let on: Vec<Id> = self.mobs.iter().filter(|(_, m)| m.health > 0 && !m.friendly && m.state.target() == Some(id) && dist2d(m.pos, at) <= reach).map(|(k, _)| *k).collect();
        for mid in on {
            if self.boss_of(mid) {
                let now = self.clock;
                let half = self.content.ai.threat_half.max(0.1);
                if let Some(t) = self.mobs.get_mut(&mid).and_then(|m| m.threat.iter_mut().find(|t| t.0 == id)) {
                    // (What's left of it now, halved, from now.)
                    t.1 = t.1 * 0.5f32.powf((now - t.2) / half) * 0.5;
                    t.2 = now;
                }
                continue;
            }
            if matches!(self.mobs[&mid].state, mobs::MobState::Windup { .. }) {
                self.mobs.get_mut(&mid).unwrap().rogue.feint_from = Some((id, secs));
            } else {
                self.feint_turn(mid, id, secs);
            }
        }
    }

    /// A feinted creature looks away from the Rogue: to the nearest other hero within
    /// `[rogue] feint_turn` yd it can see, else at nobody, for `secs`. Either way it takes no
    /// notice of the Rogue for that long.
    fn feint_turn(&mut self, mid: Id, rogue: Id, secs: f32) {
        let now = self.clock;
        let Some(pos) = self.mobs.get(&mid).map(|m| m.pos) else { return };
        let turn = self.content.rogue.feint_turn;
        let friend = self
            .players
            .iter()
            .filter(|(h, p)| **h != rogue && p.health > 0 && dist2d(p.pos, pos) <= turn && !self.unseen_by(mid, **h) && clear_line(self.land.as_ref(), pos, p.pos))
            .min_by(|a, b| dist2d(a.1.pos, pos).total_cmp(&dist2d(b.1.pos, pos)))
            .map(|(h, _)| *h);
        if friend.is_none() {
            self.halt_mob(mid);
        }
        let Some(m) = self.mobs.get_mut(&mid) else { return };
        m.pattern = None;
        m.state = match friend {
            Some(h) => mobs::MobState::Chase { foe: mobs::Foe::Hero(h) },
            None => mobs::MobState::Idle { wander_at: now + secs },
        };
        m.fx.lulled = Some((rogue, now + secs));
        m.rogue.feint_from = None;
    }

    /// A creature Feinted mid-blow: once the blow is done, it looks away (`step_shadowed`).
    fn feint_after_blow(&mut self, mid: Id) {
        let Some((rogue, secs)) = self.mobs.get(&mid).and_then(|m| m.rogue.feint_from) else { return };
        if self.mobs[&mid].state.target() == Some(rogue) {
            self.feint_turn(mid, rogue, secs);
        } else if let Some(m) = self.mobs.get_mut(&mid) {
            m.rogue.feint_from = None;
        }
    }
}
