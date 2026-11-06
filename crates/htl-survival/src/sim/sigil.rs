//! The group check: Holy Word: Salvation. A great circle of light where the Priest stands, seen
//! by everyone, friend and foe: the one floor mark the Priest makes, and the point of it. On each
//! of its `beats` (2 sec, then 1.5, then 1) a ring runs out from the Priest to the circle's edge
//! (Corey: the edge is the line everyone can see), drawn as a friend's ring growing step by step,
//! and flashes as it reaches it. Every hero inside who acts (attacks, dodges, guards, casts: any
//! action key) within `[checks] window` of a beat lands it: their own perfect, and a stack of
//! Holy Light (more damage, their blows glowing gold). A hero who misses one just has a smaller
//! gift. The last beat calls a pillar of light at the centre: its damage grows with every hero
//! who landed it, and it heals each of them.
//!
//! The Priest rises on wings of light (`float`) while it lasts: held up there, nothing reaching
//! them, still able to fight (and so to land the beats themselves, alone). They come down slowly
//! as it ends, as Skyfall's hunter does.
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
        // Up on wings of light till the last beat is settled.
        if d.float > 0.0 {
            self.float_up(id, ability, t + self.settle_wait());
        }
        // The circle, for everyone: a friend's ring (the deliberate floor mark), and its light on
        // the ground.
        let ring = self.mark_kind(at, radius, life, true);
        self.light.drawn_marks.insert(ring);
        let look = self.mark_unseen(life);
        self.events.push(Event::SigilLaid { look, at, radius, beats: beats.iter().map(|b| b - now).collect() });
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
                        self.light.drawn_marks.remove(&m);
                    }
                    let r = d.radius[1] * (step + 1) as f32 / RIM_STEPS as f32;
                    let m = self.mark_kind(g.at, r, span / RIM_STEPS as f32 + 0.05, true);
                    self.light.drawn_marks.insert(m);
                    self.light.sigils[k].rim = Some((m, step, b));
                }
            } else if let Some((m, ..)) = g.rim {
                self.unmark(m);
                self.light.sigils[k].rim = None;
            }
            // Each beat's flash as the ring reaches the edge: the light there, and the edge's ring.
            let fallen = g.beats.iter().filter(|b| **b <= now + 1.0e-4).count();
            if fallen > g.flashed {
                self.light.sigils[k].flashed = fallen;
                if d.tick_spell != 0 {
                    let r = d.radius[1];
                    for k in 0..6 {
                        let a = k as f32 * std::f32::consts::TAU / 6.0;
                        let at = self.ground_near(g.at[0] + a.cos() * r, g.at[1] + a.sin() * r, g.at[2] + 1.0);
                        self.events.push(Event::Missile { caster: g.owner, spell: d.tick_spell, target: Target::Point(at) });
                    }
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
                self.light.drawn_marks.remove(&g.ring);
                if let Some((m, ..)) = g.rim {
                    self.unmark(m);
                    self.light.drawn_marks.remove(&m);
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
            // The Priest's and their friends' beats: a foe in it (a duel's) lands nothing.
            if id != g.owner && !self.helpable(g.owner, id) {
                continue;
            }
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
        let ability = g.ability;
        g.paid[b] = true;
        let c = self.content.checks.clone();
        self.gain_valor(id, c.perfect_valor);
        let kit = self.perfect_kit_of(ability);
        if kit != 0 {
            self.events.push(Event::Visual { id, kit, by: None });
        }
        self.events.push(Event::Perfect { player: id, ability });
        self.stack_up(id, ability);
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
        self.events.push(Event::PriestLook { player: id, kind: "light", n: stacks, on: None, a: st.secs, b: 0.0 });
        self.events.push(Event::Auras { id });
    }

    /// Salvation lifts its caster: up `float` yards, then held there (rooted in the air, nothing
    /// reaching them, wings on) until `until`.
    fn float_up(&mut self, id: Id, ability: usize, until: f32) {
        let now = self.clock;
        let height = self.content.abilities[ability].float;
        // The lift that tops out `height` up (as Skyfall reckons its leap: half its air time up).
        let g = 2.0 / super::moves::airtime(1.0).max(0.01);
        let lift = (2.0 * g * height).sqrt();
        let rise = super::moves::airtime(lift) / 2.0;
        let Some(f) = self.players.get(&id).map(|p| p.facing) else { return };
        self.events.push(Event::Hop { player: id, dir: f, speed: 0.0, lift, roll: false, style: "" });
        self.light.aloft.retain(|a| a.0 != id);
        self.light.aloft.push((id, ability, now + rise, until, false));
        if let Some(p) = self.players.get_mut(&id) {
            p.untouchable_until = p.untouchable_until.max(until + 0.2);
        }
        self.events.push(Event::Auras { id });
    }

    /// Up on Salvation's wings: rising or held.
    pub(crate) fn salvation_aloft(&self, id: Id) -> bool {
        self.light.aloft.iter().any(|a| a.0 == id)
    }

    /// Held still at the top of Salvation's rise.
    pub(crate) fn salvation_held(&self, id: Id) -> bool {
        self.light.aloft.iter().any(|a| a.0 == id && a.4)
    }

    /// Salvation's hang: held at the top as they reach it (the hover's pose on the legs, falling
    /// slowly if anything lets go); at its end, or as they fall, let go and down slowly.
    pub(crate) fn step_aloft(&mut self) {
        let now = self.clock;
        for (id, _, top, until, held) in self.light.aloft.clone() {
            let alive = self.players.get(&id).is_some_and(|p| p.health > 0);
            if !alive || now >= until {
                self.light.aloft.retain(|a| a.0 != id);
                self.update_rooted(id);
                if alive {
                    // Down slowly, as Skyfall's hunter comes down (`skyfall_landed` ends it).
                    self.hunt.floating.retain(|(h, _)| *h != id);
                    self.hunt.floating.push((id, now + super::hunter::DESCENT_SECS));
                    self.events.push(Event::FeatherFall { player: id, on: true });
                    self.events.push(Event::ClipStop { id });
                }
                self.events.push(Event::Auras { id });
                continue;
            }
            if !held && now >= top {
                if let Some(a) = self.light.aloft.iter_mut().find(|a| a.0 == id) {
                    a.4 = true;
                }
                self.update_rooted(id);
                self.events.push(Event::FeatherFall { player: id, on: true });
                self.events.push(Event::Clip { id, clip: super::hunter::HOVER_CLIP, take: 0, from: None, secs: (until - now) + 0.5, at: 0.0, body: ClipBody::Lower });
            }
        }
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
        // Capped after the caster's own might (Holy Light's stacks): 110 is the most it lands for.
        let might = self.players.get(&g.owner).map_or(1.0, |p| 1.0 + self.effect_of(p, "might")).max(1.0);
        let damage = (d.damage[0] + d.per_lander * landers.len() as f32).min(d.damage[1] / might);
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
