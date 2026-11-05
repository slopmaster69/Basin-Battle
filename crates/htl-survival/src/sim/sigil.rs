//! The group check: Holy Word: Salvation. A great circle of light where the Priest stands, seen
//! by everyone, friend and foe: the one floor mark the Priest makes, and the point of it. Its rim
//! closes on the centre on each of its `beats` (2 sec, then 1.5, then 1), drawn as a friend's
//! ring shrinking step by step, and flashes as it touches. Every hero inside who acts (attacks,
//! dodges, guards, casts: any action key) within `[checks] window` of a beat lands it: their own
//! perfect, and a stack of Holy Light (more damage, their blows glowing gold). A hero who misses
//! one just has a smaller gift. The last beat calls a pillar of light at the centre: its damage
//! grows with every hero who landed it, and it heals each of them.
//!
//! The Priest gets a perfect for each beat any friend lands (Grace is fed from it).
//!
//! Lag: each hero's press is judged on their own screen (its arrival less their round trip, as
//! far back as `combat.max_rewind`), and where they stood then; a beat waits that long before
//! it's settled (the pillar comes then). Presses that land are paid as they arrive. A press just
//! before a beat's window spends that beat, so mashing keys doesn't land it.

use std::collections::BTreeSet;

use super::*;

/// Steps the rim takes as it closes over a beat.
const RIM_STEPS: u32 = 6;
/// A press this long before a beat's window opens is too early: it spends the beat.
const TOO_EARLY: f32 = 0.3;

/// A circle on the ground, closing on its beats.
#[derive(Debug, Clone)]
pub(crate) struct Sigil {
    pub owner: Id,
    pub ability: usize,
    pub at: Vec3,
    pub since: f32,
    /// When each beat falls.
    pub beats: Vec<f32>,
    /// Who landed each beat, and who spent it pressing too early.
    pub landed: Vec<BTreeSet<Id>>,
    pub early: Vec<BTreeSet<Id>>,
    /// Beats the Priest has been paid for (a friend landed it).
    pub paid: Vec<bool>,
    /// Beats settled so far, and flashed.
    pub settled: usize,
    pub flashed: usize,
    /// The circle (a friend's ring, for everyone), its look, and the rim closing now.
    pub ring: Id,
    pub look: Id,
    pub rim: Option<(Id, u32, usize)>,
}

/// An action key: what lands a beat.
fn acts(input: Input) -> bool {
    matches!(input, Input::Attack | Input::Heavy | Input::Guard | Input::Ability(..) | Input::Dodge | Input::Bash | Input::Kick)
}

impl Survival {
    /// Salvation goes: the circle where the caster stands, its first beat to come.
    pub(crate) fn start_sigil(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let d = self.content.abilities[ability].clone();
        let p = self.players[&id].pos;
        let at = self.ground_near(p[0], p[1], p[2] + 1.0);
        let radius = d.radius[1];
        let mut t = now;
        let beats: Vec<f32> = d
            .beats
            .iter()
            .map(|b| {
                t += b;
                t
            })
            .collect();
        let n = beats.len();
        let life = t - now + self.settle_wait() + 0.2;
        // The circle, for everyone: a friend's ring (the deliberate floor mark), and its light on
        // the ground.
        let ring = self.mark_kind(at, radius, life, true);
        let look = self.mark_unseen(life);
        let spell = if d.linger_spell != 0 { d.linger_spell } else { d.spell };
        if spell != 0 {
            self.events.push(Event::AreaLook { id: look, caster: id, spell, pos: at, radius });
        }
        if d.spell != 0 {
            self.events.push(Event::Missile { caster: id, spell: d.spell, target: Target::Point(at) });
        }
        self.light.sigils.push(Sigil {
            owner: id,
            ability,
            at,
            since: now,
            beats,
            landed: vec![BTreeSet::new(); n],
            early: vec![BTreeSet::new(); n],
            paid: vec![false; n],
            settled: 0,
            flashed: 0,
            ring,
            look,
            rim: None,
        });
    }

    /// How long a beat waits on the heroes' presses before it's settled.
    fn settle_wait(&self) -> f32 {
        self.content.checks.window + self.content.combat.max_rewind + 1.0e-3
    }

    /// The circles: their rims closing, each beat's flash as it falls, beats settled, the last
    /// one's pillar.
    pub(crate) fn step_sigils(&mut self) {
        let now = self.clock;
        let wait = self.settle_wait();
        let mut k = 0;
        while k < self.light.sigils.len() {
            let g = self.light.sigils[k].clone();
            let d = self.content.abilities[g.ability].clone();
            let n = g.beats.len();
            // The rim closing on the next beat to fall: a smaller ring each step of the way.
            let next = g.beats.iter().position(|b| *b > now);
            if let Some(b) = next {
                let from = if b == 0 { g.since } else { g.beats[b - 1] };
                let span = (g.beats[b] - from).max(0.01);
                let step = (((now - from) / span) * RIM_STEPS as f32).floor().clamp(0.0, (RIM_STEPS - 1) as f32) as u32;
                if g.rim.is_none_or(|(_, s, beat)| s != step || beat != b) {
                    if let Some((m, ..)) = g.rim {
                        self.unmark(m);
                    }
                    let r = d.radius[1] * (RIM_STEPS - step) as f32 / (RIM_STEPS + 1) as f32;
                    let m = self.mark_kind(g.at, r, span / RIM_STEPS as f32 + 0.05, true);
                    self.light.sigils[k].rim = Some((m, step, b));
                }
            } else if let Some((m, ..)) = g.rim {
                self.unmark(m);
                self.light.sigils[k].rim = None;
            }
            // Each beat's flash as the rim touches the centre: the light, and a ring running out.
            let fallen = g.beats.iter().filter(|b| **b <= now + 1.0e-4).count();
            if fallen > g.flashed {
                self.light.sigils[k].flashed = fallen;
                if d.tick_spell != 0 {
                    self.events.push(Event::Missile { caster: g.owner, spell: d.tick_spell, target: Target::Point(g.at) });
                }
                if let Some(b) = &d.burst {
                    self.events.push(Event::GroundBurst { at: g.at, yaw: 0.0, shape: "ring", reach: d.radius[1], arc: 360.0, width: 0.0, secs: b.secs, colour: b.colour.clone() });
                }
            }
            // Beats whose presses are all in: settled; the last, the pillar.
            if g.settled < n && now >= g.beats[g.settled] + wait {
                self.light.sigils[k].settled += 1;
                if g.settled + 1 == n {
                    self.pillar(&g);
                }
                continue;
            }
            if g.settled >= n {
                let g = self.light.sigils.remove(k);
                self.unmark(g.ring);
                self.unmark(g.look);
                if let Some((m, ..)) = g.rim {
                    self.unmark(m);
                }
                continue;
            }
            k += 1;
        }
    }

    /// A hero pressed something: on a beat inside a circle (on their own screen, where they
    /// stood then), they land it.
    pub(crate) fn sigil_act(&mut self, id: Id, input: Input) {
        if !acts(input) || self.light.sigils.is_empty() {
            return;
        }
        let Some(p) = self.players.get(&id).filter(|p| p.health > 0) else { return };
        let rewind = p.rtt.min(self.content.combat.max_rewind);
        let t = self.clock - rewind;
        let pos = self.pos_at(id, t).unwrap_or(p.pos);
        let w = self.content.checks.window;
        for k in 0..self.light.sigils.len() {
            let g = &self.light.sigils[k];
            let radius = self.content.abilities[g.ability].radius[1];
            if dist2d(pos, g.at) > radius + abilities::HERO_SIZE || t < g.since {
                continue;
            }
            for b in g.settled..g.beats.len() {
                let at = g.beats[b];
                if (t - at).abs() <= w + 1.0e-4 {
                    if !g.early[b].contains(&id) && !g.landed[b].contains(&id) {
                        self.land_beat(k, b, id);
                    }
                    break;
                }
                if t < at - w && t >= at - w - TOO_EARLY {
                    self.light.sigils[k].early[b].insert(id);
                    break;
                }
            }
        }
    }

    /// Beat `b` of circle `k` landed by `id`: their perfect, a stack of its gift; the Priest's
    /// perfect for the first friend to land it.
    fn land_beat(&mut self, k: usize, b: usize, id: Id) {
        let g = &mut self.light.sigils[k];
        g.landed[b].insert(id);
        let (owner, ability) = (g.owner, g.ability);
        let pay_owner = id != owner && !g.paid[b];
        if pay_owner {
            g.paid[b] = true;
        }
        let c = self.content.checks.clone();
        self.gain_valor(id, c.perfect_valor);
        if c.perfect_kit != 0 {
            self.events.push(Event::Visual { id, kit: c.perfect_kit, by: None });
        }
        self.events.push(Event::Perfect { player: id, ability });
        self.stack_up(id, ability);
        if pay_owner && self.players.contains_key(&owner) {
            self.events.push(Event::Perfect { player: owner, ability });
        }
    }

    /// One more stack of a sigil's gift on a hero (at most its most), all held anew.
    fn stack_up(&mut self, id: Id, ability: usize) {
        let now = self.clock;
        let Some(st) = self.content.abilities[ability].stack.clone() else { return };
        let Some(p) = self.players.get_mut(&id) else { return };
        let had = p.effects.iter().find(|e| e.name == st.name && e.kind == "might" && e.until > now).map_or(0, |e| (e.amount / st.might.max(1.0e-4)).round() as u32);
        let stacks = (had + 1).min(st.most);
        p.effects.retain(|e| e.name != st.name);
        p.effects.push(Effect { kind: "might".into(), amount: st.might * stacks as f32, until: now + st.secs, name: st.name.clone() });
        self.events.push(Event::Effects { player: id });
        self.events.push(Event::Auras { id });
    }

    /// How many stacks of a sigil's gift a hero has now.
    pub fn holy_stacks(&self, id: Id) -> u32 {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return 0 };
        self.content
            .abilities
            .iter()
            .filter_map(|a| a.stack.as_ref())
            .filter_map(|st| p.effects.iter().find(|e| e.name == st.name && e.kind == "might" && e.until > now).map(|e| (e.amount / st.might.max(1.0e-4)).round() as u32))
            .max()
            .unwrap_or(0)
    }

    /// The gift's look on a hero who has it.
    pub(crate) fn sigil_auras(&self, id: Id) -> Vec<u32> {
        let now = self.clock;
        let Some(p) = self.players.get(&id) else { return Vec::new() };
        self.content
            .abilities
            .iter()
            .filter_map(|a| a.stack.as_ref())
            .filter(|st| st.aura != 0 && p.effects.iter().any(|e| e.name == st.name && e.until > now))
            .map(|st| st.aura)
            .collect()
    }

    /// The last beat settled: a pillar of light at the centre, harder for every hero who landed
    /// it, healing each of them.
    fn pillar(&mut self, g: &Sigil) {
        let d = self.content.abilities[g.ability].clone();
        let landers: Vec<Id> = g.landed.last().map(|l| l.iter().copied().filter(|h| self.players.get(h).is_some_and(|p| p.health > 0)).collect()).unwrap_or_default();
        if !self.players.contains_key(&g.owner) {
            return;
        }
        let damage = (d.damage[0] + d.per_lander * landers.len() as f32).min(d.damage[1]);
        let look = if d.full_spell != 0 { d.full_spell } else { d.spell };
        if look != 0 {
            self.events.push(Event::Missile { caster: g.owner, spell: look, target: Target::Point(g.at) });
        }
        self.area(g.owner, g.ability, g.at, d.radius[1], 1.0, damage, None);
        if d.mend > 0.0 {
            for h in landers {
                self.heal_hero_by(h, d.mend.round() as u32, g.owner, &d.name);
            }
        }
    }
}
