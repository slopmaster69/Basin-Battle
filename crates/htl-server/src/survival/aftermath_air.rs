//! The air of the finished Deadmines (change E of the 9 Nov review): dust still settling off the
//! piles and grit trickling down them, and the timber creaking. All of it native: the dust is the
//! low cloud a charge raises on landing (`DustCloud_Land`, spell 22792's ground look), cast now
//! and then at a pile's foot by an unseen stand-in standing there, as Rhahk'Zor's falling mine
//! raises its dust (#327); the grit is the ore-on-stone clatter of putting down rocks (sound kit
//! 1211) with some of those puffs, from the gravel there; the creak is the Deadmines' own wooden
//! door (3078) and the haunted windmill's strained timber (1650), from the snapped beams and
//! posts. Nothing changes the world: these are packets to whoever is inspecting.
//!
//! (The first take played the dust as a spell visual kit on the gravel itself: nothing showed on
//! film. The client draws a kit on a game object no more than it flies a missile from one.)

use std::time::Instant;

use super::*;
use crate::packets::Aim;

/// `Plant Thornling`'s ground look, `DustCloud_Land.mdx` and nothing else: a low cloud that
/// spreads and settles in a couple of seconds.
pub(super) const SPELL_DUST: u32 = 22792;
/// `PutDownRocks_Ore`: stones landing on stones.
pub(super) const SOUND_GRIT: u32 = 1211;
/// `DeadmineDoor01Open` and `WindMillHauntedStandVar1`: timber taking strain.
pub(super) const SOUND_CREAK: [u32; 2] = [3078, 1650];
/// Each pile raises its dust this often, give or take a third (seconds).
const DUST_EVERY: f32 = 7.0;
/// A beam creaks this often, give or take a half.
const CREAK_EVERY: f32 = 16.0;
/// No more than this many dust spots (each an unseen creature while the preview lasts).
const MAX_DUST: usize = 90;
/// The dust's stand-ins' own ids, clear of the rules' (which number up from 1).
const FIRST_STAND_IN: u32 = 0x00E0_0000;
/// The first puffs wait this long, for the stand-ins to be placed.
const SETTLE: f32 = 1.0;

/// One place the dust rises: the gravel it rises from (and the grit sounds from), the spot on
/// the floor, and the stand-in that casts it there.
pub(super) struct Spot {
    pub(super) prop: usize,
    pub(super) at: [f32; 3],
    pub(super) guid: u64,
    next: f32,
}

/// Where the air comes from, and when each next does something.
pub(crate) struct Air {
    pub(super) started: Instant,
    pub(super) dust: Vec<Spot>,
    /// (prop index, next creak), on beams.
    creaks: Vec<(usize, f32)>,
    count: u32,
}

/// A fixed spread in [0, 1): the same every preview, different for each source and turn.
fn spread(a: usize, b: u32) -> f32 {
    let mut h = (a as u32).wrapping_mul(0x9E37_79B1) ^ b.wrapping_mul(0x85EB_CA77) ^ 0x2545_F491;
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    (h % 10_000) as f32 / 10_000.0
}

impl Air {
    /// The sources of a composed ruin: up to three spots a pile, on its gravel (or its smallest
    /// stones) a couple of yards apart, on the floor there; and every pile's beams. The views'
    /// foreground and the drowned cove raise none.
    pub(super) fn new(props: &[Prop], g: &Ground, started: Instant) -> Self {
        let mut dust: Vec<Spot> = Vec::new();
        let mut per_site: BTreeMap<&str, Vec<[f32; 3]>> = BTreeMap::new();
        for (i, p) in props.iter().enumerate() {
            if p.site == "Rubble apron" || p.site == "Juggernaut collapse basin" { continue; }
            let gravel = p.model == SKIRT || p.model == SCREE || (p.model == ROCK && p.scale < 0.3);
            if !gravel || dust.len() >= MAX_DUST { continue; }
            let spots = per_site.entry(p.site).or_default();
            let apart = spots.iter().all(|s| (s[0] - p.pos[0]).hypot(s[1] - p.pos[1]) > 2.0);
            if spots.len() >= 3 || !apart { continue; }
            let floor = floor_under(&*g.land, [p.pos[0], p.pos[1], p.pos[2] + 0.6]);
            let z = if floor.is_finite() && (floor - p.pos[2]).abs() < 2.0 { floor } else { p.pos[2] };
            let at = [p.pos[0], p.pos[1], z + 0.1];
            spots.push(at);
            let guid = SurvivalGame::stand_in_guid(FIRST_STAND_IN + dust.len() as u32);
            dust.push(Spot { prop: i, at, guid, next: SETTLE + DUST_EVERY * spread(i, 0) });
        }
        let creaks = props.iter().enumerate().filter(|(_, p)| p.model == BEAM && p.site != "Rubble apron")
            .map(|(i, _)| (i, SETTLE + CREAK_EVERY * spread(i, 1))).collect();
        Self { started, dust, creaks, count: 0 }
    }

    /// The unseen stand-ins, for an inspector arriving.
    pub(super) fn stand_ins(&self) -> Vec<Packet> {
        self.dust.iter().map(|s| SurvivalGame::stand_in_packet(s.guid, s.at)).collect()
    }

    /// What is due by now: each source's packet, and its next turn.
    pub(super) fn due(&mut self, props: &[Prop], now: f32) -> Vec<Packet> {
        let mut out = Vec::new();
        for s in self.dust.iter_mut() {
            if s.next > now { continue; }
            self.count += 1;
            out.push(packets::spell_go(s.guid, SPELL_DUST, Aim::Point(s.at), &[], None));
            // About a third of the puffs come with the sound of grit landing.
            if spread(s.prop, self.count) < 0.35 {
                out.push(packets::play_object_sound(prop_guid(s.prop, props[s.prop].model), SOUND_GRIT));
            }
            s.next += DUST_EVERY * (0.67 + 0.67 * spread(s.prop, self.count + 7));
        }
        for (i, at) in self.creaks.iter_mut() {
            if *at > now { continue; }
            self.count += 1;
            let sound = SOUND_CREAK[(spread(*i, self.count) * 2.0) as usize % 2];
            out.push(packets::play_object_sound(prop_guid(*i, props[*i].model), sound));
            *at += CREAK_EVERY * (0.5 + spread(*i, self.count + 3));
        }
        out
    }
}

impl SurvivalGame {
    /// The preview's air, once a tick: what's due goes to everyone in the world.
    pub(crate) fn aftermath_air(&mut self) {
        let Some(a) = self.aftermath.as_mut() else { return };
        let now = a.air.started.elapsed().as_secs_f32();
        let due = a.air.due(&a.props, now);
        for p in due { self.broadcast(&p); }
    }
}
